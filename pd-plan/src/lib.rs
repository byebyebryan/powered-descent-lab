//! Deterministic bounded V1 waypoint planner.
//!
//! This crate deliberately depends only on `pd-core`.  It searches a finite
//! candidate graph over the immutable heightfield, validates every selected
//! leg with the exact core corridor query, and returns a typed policy
//! rejection instead of reaching into controller or evaluator configuration.

use std::cmp::Ordering;

use pd_core::{
    CorridorClearance, CorridorEnvelope, NormalizedRouteGeometry, PlanningRejection,
    PlanningRejectionCode, RoutePlan, RoutePlanDiagnostics, RoutePlanningPolicy,
    RoutePlanningRequest, RouteTopology, SafetyProfile, TransferRouteSpec, TransferWaypointSpec,
    Vec2, build_endpoint_profile, compute_waypoint_authority, endpoint_shaped_centerline,
    normalized_geometry, validate_route,
};
use serde::{Deserialize, Serialize};

// Superseded V1 fixed-gate experiment; retained only as historical tests.
#[cfg(test)]
mod conservative_ballistic;

/// Versioned V2 ballistic-first analytical direct certificate. This remains
/// isolated from production planner behavior and the historical V1 model.
#[cfg(any(test, feature = "conservative-ballistic-report"))]
pub mod conservative_ballistic_bridge;

pub const ALGORITHM_ID: &str = pd_core::HEIGHTFIELD_VISIBILITY_ALGORITHM_ID;

/// Versioned, input-only planner candidate exposure consumed by research
/// diagnostics.  This API deliberately sits beside (rather than inside)
/// [`plan`], so ordinary planner results and rejection strings remain
/// unchanged.
pub const CANDIDATE_EXPOSURE_SCHEMA_ID: &str = "planner_candidate_exposure_v1";
pub const CANDIDATE_EXPOSURE_SCHEMA_VERSION: u32 = 1;
pub const CANDIDATE_EXPOSURE_MAX_EXAMINED_PATHS: usize = 65_536;
pub const CANDIDATE_EXPOSURE_MAX_RETAINED_CANDIDATES: usize = 256;

/// Quantization used only for candidate ranking. Exact authority caps emitted
/// into route plans and validated by pd-core remain unquantized.
const AUTHORITY_RANK_QUANTUM_MPS: f64 = 1.0e-9;

/// Plan an owned request snapshot through the deterministic V1 search.
pub fn plan(request: &RoutePlanningRequest) -> Result<RoutePlan, PlanningRejection> {
    Planner.plan(request)
}

/// A bounded, input-only view of the exact candidate paths considered by the
/// production planner.  The selected production plan is always rank zero,
/// including when the diagnostic path budget prevents the search from
/// reaching that path in traversal order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlannerCandidateExposureV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub request_digest: String,
    pub selected_plan_digest: String,
    pub limits: PlannerCandidateExposureLimitsV1,
    pub examined_path_count: usize,
    /// Number of valid candidates found by bounded traversal. An injected
    /// selected plan is deliberately excluded from this count.
    pub accepted_candidate_count: usize,
    pub retained_candidate_count: usize,
    pub complete: bool,
    pub path_truncated: bool,
    pub retention_truncated: bool,
    pub truncation_reasons: Vec<String>,
    pub selected_plan_injected: bool,
    pub candidates: Vec<PlannerCandidateV1>,
    pub exposure_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannerCandidateExposureLimitsV1 {
    pub max_examined_paths: usize,
    pub max_retained_candidates: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlannerCandidateV1 {
    pub rank: usize,
    pub plan_digest: String,
    pub plan: RoutePlan,
    pub node_ids: Vec<String>,
    pub minimum_handoff_speed_cap_mps: f64,
    pub route_length_m: f64,
    pub peak_extra_loft_m: f64,
}

impl PlannerCandidateExposureV1 {
    /// Validate structural, selected-plan, ranking, and digest invariants.
    /// Candidate order is established by and rechecked against the exact
    /// production comparator; duplicated metrics are canonicalized to the
    /// emitted plan's diagnostics and validated independently.
    /// Callers should run this before persisting or consuming an exposure.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_id != CANDIDATE_EXPOSURE_SCHEMA_ID {
            return Err(format!(
                "candidate exposure schema_id must equal {CANDIDATE_EXPOSURE_SCHEMA_ID}"
            ));
        }
        if self.schema_version != CANDIDATE_EXPOSURE_SCHEMA_VERSION {
            return Err(format!(
                "candidate exposure schema_version must equal {CANDIDATE_EXPOSURE_SCHEMA_VERSION}"
            ));
        }
        let expected_limits = PlannerCandidateExposureLimitsV1 {
            max_examined_paths: CANDIDATE_EXPOSURE_MAX_EXAMINED_PATHS,
            max_retained_candidates: CANDIDATE_EXPOSURE_MAX_RETAINED_CANDIDATES,
        };
        if self.limits != expected_limits {
            return Err("candidate exposure limits do not match schema v1".to_owned());
        }
        if self.examined_path_count > self.limits.max_examined_paths {
            return Err("candidate exposure examined path count exceeds its limit".to_owned());
        }
        if self.path_truncated && self.examined_path_count != self.limits.max_examined_paths {
            return Err("path-truncated exposure did not exhaust its path limit".to_owned());
        }
        let direct_exposure_counts = self
            .candidates
            .first()
            .is_some_and(|candidate| candidate.plan.topology == RouteTopology::Direct);
        if self.accepted_candidate_count > self.examined_path_count && !direct_exposure_counts {
            return Err("candidate exposure accepted count exceeds examined paths".to_owned());
        }
        if self.retained_candidate_count != self.candidates.len()
            || self.retained_candidate_count > self.limits.max_retained_candidates
        {
            return Err("candidate exposure retained count is inconsistent".to_owned());
        }
        if self.complete == self.path_truncated {
            return Err(
                "candidate exposure complete/path truncation flags are inconsistent".to_owned(),
            );
        }
        let mut expected_reasons = Vec::new();
        if self.path_truncated {
            expected_reasons.push("path_budget_exhausted".to_owned());
        }
        if self.retention_truncated {
            expected_reasons.push("retention_cap_exceeded".to_owned());
        }
        if self.truncation_reasons != expected_reasons {
            return Err("candidate exposure truncation reasons are inconsistent".to_owned());
        }
        if self.candidates.is_empty() {
            return Err("candidate exposure has no selected candidate".to_owned());
        }
        if self.candidates[0].plan.topology == RouteTopology::Direct {
            if self.selected_plan_injected
                || self.accepted_candidate_count != 1
                || self.examined_path_count != 0
                || self.path_truncated
                || self.retention_truncated
            {
                return Err("direct exposure has impossible selection counts".to_owned());
            }
        } else if !self.selected_plan_injected && self.accepted_candidate_count == 0 {
            return Err("waypoint exposure without injection has no selected candidate".to_owned());
        }
        let known_valid_count =
            self.accepted_candidate_count + usize::from(self.selected_plan_injected);
        if known_valid_count == 0 {
            return Err("candidate exposure has no known valid candidate".to_owned());
        }
        if self.retention_truncated != (known_valid_count > self.limits.max_retained_candidates)
            || self.retained_candidate_count
                != known_valid_count.min(self.limits.max_retained_candidates)
        {
            return Err("candidate exposure retention counts are inconsistent".to_owned());
        }
        if self.candidates[0].rank != 0
            || self.candidates[0].plan_digest != self.selected_plan_digest
            || self.candidates[0].plan.plan_digest != self.selected_plan_digest
            || self.candidates[0].plan.request_digest != self.request_digest
        {
            return Err("candidate exposure rank zero is not the selected plan".to_owned());
        }
        let mut digests = std::collections::BTreeSet::new();
        let expected_policy = &self.candidates[0].plan.policy;
        for (rank, candidate) in self.candidates.iter().enumerate() {
            if candidate.rank != rank {
                return Err("candidate exposure ranks are not contiguous".to_owned());
            }
            if candidate.plan_digest != candidate.plan.plan_digest {
                return Err(format!("candidate rank {rank} has a plan digest mismatch"));
            }
            let recomputed_plan_digest = digest(&PlanDigestInput {
                algorithm_id: &candidate.plan.algorithm_id,
                policy: &candidate.plan.policy,
                request_digest: &candidate.plan.request_digest,
                topology: candidate.plan.topology,
                route: &candidate.plan.route,
                diagnostics: &candidate.plan.diagnostics,
            })
            .map_err(|error| format!("candidate rank {rank} digest input is invalid: {error}"))?;
            if candidate.plan_digest != recomputed_plan_digest {
                return Err(format!(
                    "candidate rank {rank} plan digest is not canonical"
                ));
            }
            if candidate.plan.request_digest != self.request_digest {
                return Err(format!(
                    "candidate rank {rank} has a different request digest"
                ));
            }
            if candidate.plan.algorithm_id != ALGORITHM_ID {
                return Err(format!(
                    "candidate rank {rank} uses an unexpected planner algorithm"
                ));
            }
            if candidate.plan.policy != *expected_policy {
                return Err(format!(
                    "candidate rank {rank} uses a different planning policy"
                ));
            }
            if candidate.node_ids != candidate.plan.diagnostics.selected_node_ids {
                return Err(format!(
                    "candidate rank {rank} node identity does not match its plan"
                ));
            }
            if [
                candidate.minimum_handoff_speed_cap_mps,
                candidate.route_length_m,
                candidate.peak_extra_loft_m,
            ]
            .iter()
            .any(|value| !value.is_finite())
            {
                return Err(format!(
                    "candidate rank {rank} ranking metrics are not finite"
                ));
            }
            let expected_minimum_handoff_speed_cap_mps = candidate
                .plan
                .diagnostics
                .waypoint_authority
                .iter()
                .map(|authority| authority.handoff_speed_cap_mps)
                .min_by(f64::total_cmp)
                .unwrap_or(0.0);
            if candidate.minimum_handoff_speed_cap_mps != expected_minimum_handoff_speed_cap_mps
                || candidate.route_length_m != candidate.plan.diagnostics.route_length_m
                || candidate.peak_extra_loft_m != candidate.plan.diagnostics.peak_extra_loft_m
            {
                return Err(format!(
                    "candidate rank {rank} ranking metrics are not canonical"
                ));
            }
            if !digests.insert(candidate.plan_digest.clone()) {
                return Err(format!("candidate rank {rank} duplicates a plan digest"));
            }
            candidate
                .plan
                .route
                .validate()
                .map_err(|error| format!("candidate rank {rank} route is invalid: {error}"))?;
            if let Some(previous) = self.candidates.get(rank.saturating_sub(1))
                && rank > 0
            {
                let previous_path = candidate_path_from_plan(previous).map_err(|error| {
                    format!("candidate rank {} ranking data: {error}", rank - 1)
                })?;
                let candidate_path = candidate_path_from_plan(candidate)
                    .map_err(|error| format!("candidate rank {rank} ranking data: {error}"))?;
                if !candidate_precedes(&previous_path, &candidate_path) {
                    return Err(format!(
                        "candidate rank {rank} is not ordered by planner ranking"
                    ));
                }
            }
        }
        if self.exposure_digest != candidate_exposure_digest(self) {
            return Err("candidate exposure digest mismatch".to_owned());
        }
        Ok(())
    }
}

/// Compute the stable exposure digest, excluding the digest field itself.
pub fn candidate_exposure_digest(exposure: &PlannerCandidateExposureV1) -> String {
    let mut material = exposure.clone();
    material.exposure_digest.clear();
    digest(&material).unwrap_or_else(|_| "fnv1a64:0000000000000000".to_owned())
}

/// Expose the planner's bounded, statically valid waypoint candidates for a
/// research-only diagnostic.  This function shares the production candidate
/// generator, exact route constructor/validator, comparator, and plan digest
/// machinery.  It has no executor or outcome inputs.
pub fn expose_candidates(
    request: &RoutePlanningRequest,
) -> Result<PlannerCandidateExposureV1, PlanningRejection> {
    let selected = plan(request)?;
    let request_digest = selected.request_digest.clone();
    let limits = PlannerCandidateExposureLimitsV1 {
        max_examined_paths: CANDIDATE_EXPOSURE_MAX_EXAMINED_PATHS,
        max_retained_candidates: CANDIDATE_EXPOSURE_MAX_RETAINED_CANDIDATES,
    };
    if selected.topology == RouteTopology::Direct {
        let selected_route_length_m = selected.diagnostics.route_length_m;
        let selected_peak_extra_loft_m = selected.diagnostics.peak_extra_loft_m;
        let mut exposure = PlannerCandidateExposureV1 {
            schema_id: CANDIDATE_EXPOSURE_SCHEMA_ID.to_owned(),
            schema_version: CANDIDATE_EXPOSURE_SCHEMA_VERSION,
            request_digest,
            selected_plan_digest: selected.plan_digest.clone(),
            limits,
            examined_path_count: 0,
            accepted_candidate_count: 1,
            retained_candidate_count: 1,
            complete: true,
            path_truncated: false,
            retention_truncated: false,
            truncation_reasons: Vec::new(),
            selected_plan_injected: false,
            candidates: vec![PlannerCandidateV1 {
                rank: 0,
                plan_digest: selected.plan_digest.clone(),
                plan: selected,
                node_ids: Vec::new(),
                minimum_handoff_speed_cap_mps: 0.0,
                route_length_m: selected_route_length_m,
                peak_extra_loft_m: selected_peak_extra_loft_m,
            }],
            exposure_digest: String::new(),
        };
        exposure.exposure_digest = candidate_exposure_digest(&exposure);
        exposure.validate().map_err(exposure_rejection)?;
        return Ok(exposure);
    }

    request.validate().map_err(|error| {
        PlanningRejection::new(PlanningRejectionCode::InvalidRequest, error.to_string())
    })?;
    let geometry = normalized_geometry(request).map_err(rejection_from_validation)?;
    let (profile, free_span) = build_endpoint_profile(request, geometry.direct_horizontal_span_m)
        .map_err(rejection_from_validation)?;
    if free_span <= 0.0 {
        return Err(PlanningRejection::new(
            PlanningRejectionCode::UnsupportedGeometry,
            "source and target pad footprints overlap",
        ));
    }
    let terrain = normalized_terrain(request, &geometry)?;
    let profile_events =
        safety_profile_events(&profile, &terrain, geometry.direct_horizontal_span_m);
    let safe_profile_points =
        safe_profile_points_world(request, &geometry, &profile, &terrain, &profile_events)?;
    let direct_points = endpoint_shaped_centerline(request, &geometry, &profile, &[])
        .map_err(rejection_from_validation)?;
    let direct_clearance = exact_path_clearance(request, &geometry, &profile, &direct_points)?;
    let candidates = build_candidates(request, &geometry, &profile, &terrain, &profile_events)?;
    let capped_nodes = candidates
        .iter()
        .filter(|node| node.point.y <= node.loft_cap_y + 1.0e-9)
        .cloned()
        .collect::<Vec<_>>();
    let mut search = ExposureSearch::default();
    let mut candidate_rejections = CandidateRejections::default();
    let source = source_point(request, &geometry);
    let target = target_point(request, &geometry);
    if request.policy.max_waypoints > 0 {
        search_exposure_paths(
            request,
            &geometry,
            &profile,
            &capped_nodes,
            source,
            target,
            usize::from(request.policy.max_waypoints),
            0,
            &mut Vec::new(),
            &mut search,
            &mut candidate_rejections,
        )?;
    }

    let selected_identity = selected
        .diagnostics
        .selected_node_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("/");
    let selected_path_in_search = search
        .paths
        .iter()
        .find(|path| candidate_identity(path) == selected_identity)
        .cloned();
    let selected_in_search = selected_path_in_search.is_some();
    let selected_path = selected_path_in_search
        .unwrap_or_else(|| selected_candidate_path(request, &geometry, &selected));
    search.paths.sort_by(candidate_ordering);
    let retained_limit_without_selected = CANDIDATE_EXPOSURE_MAX_RETAINED_CANDIDATES - 1;
    search
        .paths
        .retain(|path| candidate_identity(path) != selected_identity);
    search.paths.truncate(retained_limit_without_selected);

    let mut candidate_plans = Vec::with_capacity(search.paths.len() + 1);
    candidate_plans.push((selected_path, selected.clone()));
    for path in search.paths {
        let plan = build_success_plan(
            request,
            request_digest.clone(),
            geometry.clone(),
            profile.clone(),
            path.clone(),
            direct_clearance.clone(),
            safe_profile_points.clone(),
        )?;
        candidate_plans.push((path, plan));
    }
    candidate_plans[1..].sort_by(|left, right| candidate_ordering(&left.0, &right.0));
    if candidate_plans[1..]
        .iter()
        .any(|candidate| !candidate_precedes(&candidate_plans[0].0, &candidate.0))
    {
        return Err(PlanningRejection::new(
            PlanningRejectionCode::UnsupportedGeometry,
            "ordinary selected planner candidate is not rank zero",
        ));
    }
    let candidates = candidate_plans
        .into_iter()
        .enumerate()
        .map(|(rank, (path, plan))| {
            if rank == 0 && plan.plan_digest != selected.plan_digest {
                return Err(PlanningRejection::new(
                    PlanningRejectionCode::UnsupportedGeometry,
                    "selected planner candidate digest changed during exposure",
                ));
            }
            let _ = path;
            let minimum_handoff_speed_cap_mps = plan
                .diagnostics
                .waypoint_authority
                .iter()
                .map(|authority| authority.handoff_speed_cap_mps)
                .min_by(f64::total_cmp)
                .unwrap_or(0.0);
            let route_length_m = plan.diagnostics.route_length_m;
            let peak_extra_loft_m = plan.diagnostics.peak_extra_loft_m;
            Ok(PlannerCandidateV1 {
                rank,
                plan_digest: plan.plan_digest.clone(),
                plan,
                node_ids: path.nodes.iter().map(|node| node.id.clone()).collect(),
                minimum_handoff_speed_cap_mps,
                route_length_m,
                peak_extra_loft_m,
            })
        })
        .collect::<Result<Vec<_>, PlanningRejection>>()?;
    let retention_truncated = search.accepted_candidate_count + usize::from(!selected_in_search)
        > CANDIDATE_EXPOSURE_MAX_RETAINED_CANDIDATES;
    let mut exposure = PlannerCandidateExposureV1 {
        schema_id: CANDIDATE_EXPOSURE_SCHEMA_ID.to_owned(),
        schema_version: CANDIDATE_EXPOSURE_SCHEMA_VERSION,
        request_digest,
        selected_plan_digest: selected.plan_digest.clone(),
        limits,
        examined_path_count: search.examined_path_count,
        accepted_candidate_count: search.accepted_candidate_count,
        retained_candidate_count: candidates.len(),
        complete: !search.path_truncated,
        path_truncated: search.path_truncated,
        retention_truncated,
        truncation_reasons: Vec::new(),
        selected_plan_injected: !selected_in_search,
        candidates,
        exposure_digest: String::new(),
    };
    if exposure.path_truncated {
        exposure
            .truncation_reasons
            .push("path_budget_exhausted".to_owned());
    }
    if exposure.retention_truncated {
        exposure
            .truncation_reasons
            .push("retention_cap_exceeded".to_owned());
    }
    exposure.exposure_digest = candidate_exposure_digest(&exposure);
    exposure.validate().map_err(exposure_rejection)?;
    Ok(exposure)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Planner;

impl Planner {
    pub fn plan(&self, request: &RoutePlanningRequest) -> Result<RoutePlan, PlanningRejection> {
        let request_digest = digest(request)?;
        request.validate().map_err(|error| {
            PlanningRejection::new(PlanningRejectionCode::InvalidRequest, error.to_string())
        })?;
        let geometry = normalized_geometry(request).map_err(rejection_from_validation)?;
        let (profile, free_span) =
            build_endpoint_profile(request, geometry.direct_horizontal_span_m)
                .map_err(rejection_from_validation)?;
        if free_span <= 0.0 {
            return Err(PlanningRejection::new(
                PlanningRejectionCode::UnsupportedGeometry,
                "source and target pad footprints overlap",
            ));
        }

        let terrain = normalized_terrain(request, &geometry)?;
        let profile_events =
            safety_profile_events(&profile, &terrain, geometry.direct_horizontal_span_m);
        let safe_profile_points =
            safe_profile_points_world(request, &geometry, &profile, &terrain, &profile_events)?;
        let direct_points = endpoint_shaped_centerline(request, &geometry, &profile, &[])
            .map_err(rejection_from_validation)?;
        let direct_clearance = exact_path_clearance(request, &geometry, &profile, &direct_points)?;
        if direct_clearance.clear {
            let direct_route_points = [
                source_point(request, &geometry),
                target_point(request, &geometry),
            ];
            let direct_route = route_from_points(request, &geometry, &direct_route_points, &[])?;
            let direct_validation = validate_route(request, &direct_route).map_err(|error| {
                PlanningRejection::new(error.rejection_code(), error.to_string())
            })?;
            return finalize_plan(
                request,
                request_digest,
                geometry,
                RouteTopology::Direct,
                direct_route,
                vec![],
                direct_clearance,
                direct_validation,
                safe_profile_points,
            );
        }

        let candidates = build_candidates(request, &geometry, &profile, &terrain, &profile_events)?;
        let capped_nodes: Vec<Node> = candidates
            .iter()
            .filter(|node| node.point.y <= node.loft_cap_y + 1.0e-9)
            .cloned()
            .collect();
        let mut candidate_rejections = CandidateRejections::default();
        let capped_path = find_path(
            request,
            &geometry,
            &profile,
            &capped_nodes,
            usize::from(request.policy.max_waypoints),
            &mut candidate_rejections,
        )?;

        let capped_extended = find_path(
            request,
            &geometry,
            &profile,
            &capped_nodes,
            usize::from(request.policy.max_waypoints).saturating_add(1),
            &mut candidate_rejections,
        )?;
        if capped_path.is_none()
            && (capped_extended
                .as_ref()
                .is_some_and(|path| path.nodes.len() > usize::from(request.policy.max_waypoints))
                || candidate_rejections.complexity)
        {
            return Err(PlanningRejection::new(
                PlanningRejectionCode::RouteComplexityExceeded,
                "a clear candidate path exceeds the waypoint-count policy",
            ));
        }

        if let Some(path) = capped_path {
            return build_success_plan(
                request,
                request_digest,
                geometry,
                profile,
                path,
                direct_clearance,
                safe_profile_points,
            );
        }

        // A path that is clear only above the loft ceiling receives the more
        // specific loft rejection.  Search is still bounded by the V1
        // waypoint count so this check cannot silently become an optimizer.
        let uncapped_nodes: Vec<Node> = candidates
            .iter()
            .filter(|node| node.point.y.is_finite())
            .cloned()
            .collect();
        if find_path(
            request,
            &geometry,
            &profile,
            &uncapped_nodes,
            usize::from(request.policy.max_waypoints),
            &mut candidate_rejections,
        )?
        .is_some()
        {
            return Err(PlanningRejection::new(
                PlanningRejectionCode::LoftLimitExceeded,
                "a clear bounded route exceeds the extra-loft policy",
            ));
        }
        if find_path(
            request,
            &geometry,
            &profile,
            &uncapped_nodes,
            usize::from(request.policy.max_waypoints).saturating_add(1),
            &mut candidate_rejections,
        )?
        .is_some()
            || candidate_rejections.complexity
        {
            return Err(PlanningRejection::new(
                PlanningRejectionCode::RouteComplexityExceeded,
                "no route within the waypoint-count and loft policy",
            ));
        }
        if candidate_rejections.loft {
            Err(PlanningRejection::new(
                PlanningRejectionCode::LoftLimitExceeded,
                "candidate routes exceed the extra-loft policy",
            ))
        } else if candidate_rejections.authority {
            Err(PlanningRejection::new(
                PlanningRejectionCode::InsufficientAuthority,
                "candidate paths cannot construct a valid authority envelope",
            ))
        } else {
            Err(PlanningRejection::new(
                PlanningRejectionCode::NoRouteWithinPolicy,
                "candidate graph contains no route within policy",
            ))
        }
    }
}

#[derive(Clone, Debug)]
struct NormalizedTerrain {
    points: Vec<Vec2>,
    domain_min_x_m: f64,
    domain_max_x_m: f64,
}

#[derive(Clone, Debug)]
struct Node {
    id: String,
    point: Vec2,
    loft_cap_y: f64,
}

#[derive(Clone, Debug)]
struct CandidatePath {
    nodes: Vec<Node>,
    minimum_handoff_speed_cap_mps: f64,
    route_length_m: f64,
    peak_extra_loft_m: f64,
}

#[derive(Default)]
struct ExposureSearch {
    paths: Vec<CandidatePath>,
    examined_path_count: usize,
    accepted_candidate_count: usize,
    path_truncated: bool,
}

#[derive(Default)]
struct CandidateRejections {
    authority: bool,
    loft: bool,
    complexity: bool,
}

fn normalized_terrain(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
) -> Result<NormalizedTerrain, PlanningRejection> {
    let points = request.world.terrain.points();
    let mut normalized: Vec<Vec2> = points
        .iter()
        .map(|point| {
            Vec2::new(
                f64::from(geometry.horizontal_sign)
                    * (point.x - request.source_pad().expect("validated source").center_x_m),
                point.y,
            )
        })
        .collect();
    if geometry.horizontal_sign < 0 {
        normalized.reverse();
    }
    if normalized
        .windows(2)
        .any(|pair| pair[1].x <= pair[0].x || !pair[1].x.is_finite())
    {
        return Err(PlanningRejection::new(
            PlanningRejectionCode::UnsupportedGeometry,
            "normalized heightfield must increase in horizontal progress",
        ));
    }
    let domain_min_x_m = normalized[0].x;
    let domain_max_x_m = normalized[normalized.len() - 1].x;
    if domain_min_x_m > 0.0 || domain_max_x_m < geometry.direct_horizontal_span_m {
        // The strict corridor query will provide the exact overrun location;
        // fail early here only when the pad centers themselves are absent.
        return Err(PlanningRejection::new(
            PlanningRejectionCode::UnsupportedGeometry,
            "pad centers lie outside the terrain domain",
        ));
    }
    Ok(NormalizedTerrain {
        points: normalized,
        domain_min_x_m,
        domain_max_x_m,
    })
}

fn build_candidates(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    profile: &SafetyProfile,
    terrain: &NormalizedTerrain,
    profile_events: &[f64],
) -> Result<Vec<Node>, PlanningRejection> {
    let d = geometry.direct_horizontal_span_m;
    let x_values = profile_events
        .iter()
        .copied()
        .filter(|x| {
            *x > profile.source_transition_end_m + 1.0e-9
                && *x < profile.target_transition_start_m - 1.0e-9
        })
        .collect::<Vec<_>>();

    let source_y = source_point(request, geometry).y;
    let target_y = target_point(request, geometry).y;
    let loft_limit = request.policy.max_extra_loft_ratio * geometry.direct_distance_m;
    let mut nodes = Vec::new();
    for (index, x) in x_values.into_iter().enumerate() {
        let envelope = profile.envelope_at(x);
        let required = terrain_required_height(terrain, x, envelope)?;
        let direct_y = lerp(source_y, target_y, x / d);
        let cap = direct_y + loft_limit;
        let source_support =
            source_visible_height(source_y, target_y, x, d, terrain, profile, profile_events)?;
        let target_support =
            target_visible_height(source_y, target_y, x, d, terrain, profile, profile_events)?;
        // Profile, support-ray, and policy-cap heights are all canonical
        // bounded candidate levels.  Support rays find the lowest point that
        // remains visible from each endpoint; the cap is retained as the
        // bounded fallback for routes requiring a steeper loft.
        let support_height = source_support.max(target_support);
        let mut heights = vec![
            required.max(direct_y),
            source_support.max(direct_y),
            target_support.max(direct_y),
            support_height.max(direct_y),
        ];
        if cap > required.max(direct_y) + 1.0e-9 {
            heights.push(cap);
        }
        heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        heights.dedup_by(|a, b| (*a - *b).abs() <= 1.0e-9);
        for (height_index, y) in heights.into_iter().enumerate() {
            nodes.push(Node {
                id: format!("node-{index:04}-{height_index}"),
                point: Vec2::new(x, y),
                loft_cap_y: cap,
            });
        }
    }
    nodes.sort_by(|left, right| {
        left.point
            .x
            .total_cmp(&right.point.x)
            .then_with(|| left.point.y.total_cmp(&right.point.y))
            .then_with(|| left.id.cmp(&right.id))
    });
    if nodes.is_empty() {
        return Err(PlanningRejection::new(
            PlanningRejectionCode::NoRouteWithinPolicy,
            "heightfield produced no interior candidate nodes",
        ));
    }
    Ok(nodes)
}

fn source_visible_height(
    source_y: f64,
    target_y: f64,
    candidate_x: f64,
    horizontal_span: f64,
    terrain: &NormalizedTerrain,
    profile: &SafetyProfile,
    profile_events: &[f64],
) -> Result<f64, PlanningRejection> {
    let mut minimum = f64::NEG_INFINITY;
    let source_start_x = profile.source_transition_end_m;
    let source_start_y = lerp(source_y, target_y, source_start_x / horizontal_span);
    for &x in profile_events {
        if x <= source_start_x + 1.0e-9 || x > candidate_x + 1.0e-9 {
            continue;
        }
        let required = terrain_required_height(terrain, x, profile.envelope_at(x))?;
        let required_at_candidate = source_start_y
            + ((required - source_start_y) * (candidate_x - source_start_x) / (x - source_start_x));
        minimum = minimum.max(required_at_candidate);
    }
    Ok(minimum)
}

fn target_visible_height(
    source_y: f64,
    target_y: f64,
    candidate_x: f64,
    horizontal_span: f64,
    terrain: &NormalizedTerrain,
    profile: &SafetyProfile,
    profile_events: &[f64],
) -> Result<f64, PlanningRejection> {
    let mut minimum = f64::NEG_INFINITY;
    let target_start_x = profile.target_transition_start_m;
    let target_start_y = lerp(source_y, target_y, target_start_x / horizontal_span);
    for &x in profile_events {
        if x < candidate_x - 1.0e-9 || x >= target_start_x - 1.0e-9 {
            continue;
        }
        let required = terrain_required_height(terrain, x, profile.envelope_at(x))?;
        let required_at_candidate = target_start_y
            + ((required - target_start_y) * (target_start_x - candidate_x) / (target_start_x - x));
        minimum = minimum.max(required_at_candidate);
    }
    Ok(minimum)
}

fn safety_profile_events(
    profile: &SafetyProfile,
    terrain: &NormalizedTerrain,
    horizontal_span_m: f64,
) -> Vec<f64> {
    let mut events = profile
        .breakpoints()
        .into_iter()
        .filter(|x| *x >= 0.0 && *x <= horizontal_span_m)
        .collect::<Vec<_>>();
    let breakpoints = profile.breakpoints();
    for region in breakpoints.windows(2) {
        let start = region[0];
        let end = region[1];
        if end - start <= 1.0e-12 {
            continue;
        }
        let mut arrangement = vec![start, end];
        let start_extent = profile.envelope_at(start).horizontal_extent_m;
        let end_extent = profile.envelope_at(end).horizontal_extent_m;
        for vertex in &terrain.points {
            for (start_value, end_value) in [
                (start - start_extent - vertex.x, end - end_extent - vertex.x),
                (start + start_extent - vertex.x, end + end_extent - vertex.x),
            ] {
                if start_value.abs() <= 1.0e-12 {
                    events.push(start);
                }
                if end_value.abs() <= 1.0e-12 {
                    events.push(end);
                }
                let denominator = end_value - start_value;
                if denominator.abs() > 1.0e-12 {
                    let t = -start_value / denominator;
                    if t > 0.0 && t < 1.0 {
                        let crossing = start + ((end - start) * t);
                        events.push(crossing);
                        arrangement.push(crossing);
                    }
                }
            }
        }

        arrangement.sort_by(f64::total_cmp);
        arrangement.dedup_by(|left, right| (*left - *right).abs() <= 1.0e-9);

        // Within a profile/terrain arrangement interval, each active
        // terrain contributor (the two moving lateral-boundary samples and
        // every included terrain vertex) is affine in centerline x.  The
        // maximum can switch owners away from a boundary-crossing event, so
        // retain every pairwise affine intersection as an exact safety
        // profile breakpoint as well.
        for interval in arrangement.windows(2) {
            let features = profile_requirement_features(profile, terrain, interval[0], interval[1]);
            for (left, right) in features.iter().enumerate() {
                for other in features.iter().skip(left + 1) {
                    let denominator = right.1 - other.1;
                    if denominator.abs() <= 1.0e-12 {
                        continue;
                    }
                    let crossing = (other.0 - right.0) / denominator;
                    if crossing > interval[0] + 1.0e-10 && crossing < interval[1] - 1.0e-10 {
                        events.push(crossing);
                    }
                }
            }
        }
    }
    for point in &terrain.points {
        if point.x >= 0.0 && point.x <= horizontal_span_m {
            events.push(point.x);
        }
    }
    events.sort_by(f64::total_cmp);
    events.dedup_by(|left, right| (*left - *right).abs() <= 1.0e-9);
    events
}

/// Return affine `(intercept, slope)` requirements for all terrain features
/// active over a single profile arrangement interval.  The profile's
/// vertical envelope is included in each affine requirement.
fn profile_requirement_features(
    profile: &SafetyProfile,
    terrain: &NormalizedTerrain,
    start: f64,
    end: f64,
) -> Vec<(f64, f64)> {
    let span = end - start;
    if span <= 1.0e-12 {
        return Vec::new();
    }
    let start_envelope = profile.envelope_at(start);
    let end_envelope = profile.envelope_at(end);
    let horizontal_slope =
        (end_envelope.horizontal_extent_m - start_envelope.horizontal_extent_m) / span;
    let horizontal_intercept = start_envelope.horizontal_extent_m - (horizontal_slope * start);
    let vertical_slope = (end_envelope.vertical_extent_m - start_envelope.vertical_extent_m) / span;
    let vertical_intercept = start_envelope.vertical_extent_m - (vertical_slope * start);
    let midpoint = (start + end) * 0.5;
    let midpoint_envelope = profile.envelope_at(midpoint);
    let midpoint_left = midpoint - midpoint_envelope.horizontal_extent_m;
    let midpoint_right = midpoint + midpoint_envelope.horizontal_extent_m;
    let mut features = Vec::with_capacity(terrain.points.len() + 2);

    for boundary in [false, true] {
        let boundary_slope = if boundary {
            1.0 + horizontal_slope
        } else {
            1.0 - horizontal_slope
        };
        let boundary_intercept = if boundary {
            horizontal_intercept
        } else {
            -horizontal_intercept
        };
        let boundary_x = boundary_slope.mul_add(midpoint, boundary_intercept);
        if boundary_x < terrain.domain_min_x_m || boundary_x > terrain.domain_max_x_m {
            continue;
        }
        let segment = terrain_segment_index(terrain, boundary_x);
        let p0 = terrain.points[segment];
        let p1 = terrain.points[segment + 1];
        let terrain_slope = (p1.y - p0.y) / (p1.x - p0.x);
        let terrain_intercept = p0.y - (terrain_slope * p0.x);
        features.push((
            terrain_slope.mul_add(boundary_intercept, terrain_intercept + vertical_intercept),
            terrain_slope.mul_add(boundary_slope, vertical_slope),
        ));
    }

    for vertex in &terrain.points {
        if vertex.x >= midpoint_left && vertex.x <= midpoint_right {
            features.push((vertex.y + vertical_intercept, vertical_slope));
        }
    }
    features
}

fn terrain_segment_index(terrain: &NormalizedTerrain, x: f64) -> usize {
    if x <= terrain.points[0].x {
        return 0;
    }
    if x >= terrain.points[terrain.points.len() - 1].x {
        return terrain.points.len() - 2;
    }
    terrain
        .points
        .binary_search_by(|point| point.x.total_cmp(&x))
        .unwrap_or_else(|index| index)
        .saturating_sub(1)
        .min(terrain.points.len() - 2)
}

fn safe_profile_points_world(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    profile: &SafetyProfile,
    terrain: &NormalizedTerrain,
    events: &[f64],
) -> Result<Vec<Vec2>, PlanningRejection> {
    events
        .iter()
        .map(|x| {
            let envelope = profile.envelope_at(*x);
            let y = terrain_required_height(terrain, *x, envelope)?;
            Ok(denormalize(Vec2::new(*x, y), request, geometry))
        })
        .collect()
}

fn terrain_required_height(
    terrain: &NormalizedTerrain,
    center_x_m: f64,
    envelope: CorridorEnvelope,
) -> Result<f64, PlanningRejection> {
    let left = center_x_m - envelope.horizontal_extent_m;
    let right = center_x_m + envelope.horizontal_extent_m;
    if left < terrain.domain_min_x_m || right > terrain.domain_max_x_m {
        return Err(PlanningRejection::new(
            PlanningRejectionCode::UnsupportedGeometry,
            format!(
                "candidate envelope [{left}, {right}] exceeds terrain domain [{}, {}]",
                terrain.domain_min_x_m, terrain.domain_max_x_m
            ),
        ));
    }
    let mut maximum = normalized_sample(terrain, left);
    maximum = maximum.max(normalized_sample(terrain, right));
    for vertex in &terrain.points {
        if vertex.x >= left && vertex.x <= right {
            maximum = maximum.max(vertex.y);
        }
    }
    Ok(maximum + envelope.vertical_extent_m)
}

fn normalized_sample(terrain: &NormalizedTerrain, x_m: f64) -> f64 {
    if x_m <= terrain.points[0].x {
        return terrain.points[0].y;
    }
    if x_m >= terrain.points[terrain.points.len() - 1].x {
        return terrain.points[terrain.points.len() - 1].y;
    }
    let index = terrain
        .points
        .binary_search_by(|point| point.x.total_cmp(&x_m))
        .unwrap_or_else(|index| index)
        .saturating_sub(1);
    let p0 = terrain.points[index];
    let p1 = terrain.points[index + 1];
    lerp(p0.y, p1.y, (x_m - p0.x) / (p1.x - p0.x))
}

fn find_path(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    profile: &SafetyProfile,
    nodes: &[Node],
    max_waypoints: usize,
    candidate_rejections: &mut CandidateRejections,
) -> Result<Option<CandidatePath>, PlanningRejection> {
    if max_waypoints == 0 {
        return Ok(None);
    }
    let source = source_point(request, geometry);
    let target = target_point(request, geometry);
    let mut best: Option<CandidatePath> = None;
    let mut current = Vec::new();
    search_paths(
        request,
        geometry,
        profile,
        nodes,
        source,
        target,
        max_waypoints,
        0,
        &mut current,
        &mut best,
        candidate_rejections,
    )?;
    Ok(best)
}

#[allow(clippy::too_many_arguments)]
fn search_paths(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    profile: &SafetyProfile,
    nodes: &[Node],
    source: Vec2,
    target: Vec2,
    remaining_waypoints: usize,
    next_index: usize,
    current: &mut Vec<Node>,
    best: &mut Option<CandidatePath>,
    candidate_rejections: &mut CandidateRejections,
) -> Result<(), PlanningRejection> {
    let previous = current.last().map_or(source, |node| node.point);
    for index in next_index..nodes.len() {
        let node = &nodes[index];
        if !edge_clear(request, geometry, profile, previous, node.point)? {
            continue;
        }
        current.push(node.clone());
        if edge_clear(request, geometry, profile, node.point, target)? {
            consider_path(
                best,
                current.clone(),
                request,
                geometry,
                source,
                target,
                candidate_rejections,
            )?;
        }
        if remaining_waypoints > 1 {
            search_paths(
                request,
                geometry,
                profile,
                nodes,
                source,
                target,
                remaining_waypoints - 1,
                index + 1,
                current,
                best,
                candidate_rejections,
            )?;
        }
        current.pop();
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn search_exposure_paths(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    profile: &SafetyProfile,
    nodes: &[Node],
    source: Vec2,
    target: Vec2,
    remaining_waypoints: usize,
    next_index: usize,
    current: &mut Vec<Node>,
    search: &mut ExposureSearch,
    candidate_rejections: &mut CandidateRejections,
) -> Result<(), PlanningRejection> {
    if search.path_truncated {
        return Ok(());
    }
    let previous = current.last().map_or(source, |node| node.point);
    for index in next_index..nodes.len() {
        if search.path_truncated {
            break;
        }
        let node = &nodes[index];
        if !edge_clear(request, geometry, profile, previous, node.point)? {
            continue;
        }
        current.push(node.clone());
        if edge_clear(request, geometry, profile, node.point, target)?
            && candidate_path_capture_allowed(source, target, current)
        {
            if search.examined_path_count >= CANDIDATE_EXPOSURE_MAX_EXAMINED_PATHS {
                search.path_truncated = true;
                current.pop();
                break;
            }
            search.examined_path_count += 1;
            if let Some(candidate) = evaluate_candidate_path(
                current.clone(),
                request,
                geometry,
                source,
                target,
                candidate_rejections,
            )? {
                search.accepted_candidate_count += 1;
                search.paths.push(candidate);
            }
        }
        if remaining_waypoints > 1 && !search.path_truncated {
            search_exposure_paths(
                request,
                geometry,
                profile,
                nodes,
                source,
                target,
                remaining_waypoints - 1,
                index + 1,
                current,
                search,
                candidate_rejections,
            )?;
        }
        current.pop();
    }
    Ok(())
}

fn consider_path(
    best: &mut Option<CandidatePath>,
    nodes: Vec<Node>,
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    source: Vec2,
    target: Vec2,
    candidate_rejections: &mut CandidateRejections,
) -> Result<(), PlanningRejection> {
    let Some(candidate) = evaluate_candidate_path(
        nodes,
        request,
        geometry,
        source,
        target,
        candidate_rejections,
    )?
    else {
        return Ok(());
    };
    let replace = best
        .as_ref()
        .is_none_or(|current| candidate_precedes(&candidate, current));
    if replace {
        *best = Some(candidate);
    }
    Ok(())
}

fn evaluate_candidate_path(
    nodes: Vec<Node>,
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    source: Vec2,
    target: Vec2,
    candidate_rejections: &mut CandidateRejections,
) -> Result<Option<CandidatePath>, PlanningRejection> {
    if !candidate_path_capture_allowed(source, target, &nodes) {
        return Ok(None);
    }
    let normalized_points = std::iter::once(source)
        .chain(nodes.iter().map(|node| node.point))
        .chain(std::iter::once(target))
        .collect::<Vec<_>>();
    let node_ids = nodes.iter().map(|node| node.id.clone()).collect::<Vec<_>>();
    let route = match route_from_points(request, geometry, &normalized_points, &node_ids) {
        Ok(route) => route,
        Err(rejection) => {
            return if reject_local_candidate(&rejection, candidate_rejections) {
                Ok(None)
            } else {
                Err(rejection)
            };
        }
    };
    let validation = match validate_route(request, &route) {
        Ok(validation) => validation,
        Err(error) => {
            let rejection = PlanningRejection::new(error.rejection_code(), error.to_string());
            return if reject_local_candidate(&rejection, candidate_rejections) {
                Ok(None)
            } else {
                Err(rejection)
            };
        }
    };
    let authority_caps_mps = validation
        .waypoint_authority
        .iter()
        .map(|authority| authority.handoff_speed_cap_mps)
        .collect::<Vec<_>>();
    let minimum_handoff_speed_cap_mps = authority_caps_mps
        .iter()
        .copied()
        .min_by(f64::total_cmp)
        .ok_or_else(|| {
        PlanningRejection::new(
            PlanningRejectionCode::UnsupportedGeometry,
            "waypoint candidate produced no authority envelope",
        )
    })?;
    let mut previous = source;
    let mut route_length_m = 0.0;
    let mut peak_extra_loft_m: f64 = 0.0;
    for node in &nodes {
        route_length_m += (node.point - previous).length();
        peak_extra_loft_m =
            peak_extra_loft_m.max(extra_loft(node.point, geometry, source.y, target.y));
        previous = node.point;
    }
    route_length_m += (target - previous).length();
    let candidate = CandidatePath {
        nodes,
        minimum_handoff_speed_cap_mps,
        route_length_m,
        peak_extra_loft_m,
    };
    Ok(Some(candidate))
}

fn candidate_path_capture_allowed(source: Vec2, target: Vec2, nodes: &[Node]) -> bool {
    let capture_radius = ((target - source).length() * 0.08).clamp(35.0, 95.0);
    !nodes
        .windows(2)
        .any(|pair| (pair[1].point - pair[0].point).length() <= capture_radius * 2.0)
}

fn reject_local_candidate(
    rejection: &PlanningRejection,
    candidate_rejections: &mut CandidateRejections,
) -> bool {
    match rejection.code {
        PlanningRejectionCode::InsufficientAuthority => {
            candidate_rejections.authority = true;
            true
        }
        PlanningRejectionCode::LoftLimitExceeded => {
            candidate_rejections.loft = true;
            true
        }
        PlanningRejectionCode::NoRouteWithinPolicy => true,
        PlanningRejectionCode::RouteComplexityExceeded => {
            candidate_rejections.complexity = true;
            true
        }
        PlanningRejectionCode::InvalidRequest | PlanningRejectionCode::UnsupportedGeometry => false,
    }
}

fn candidate_precedes(candidate: &CandidatePath, current: &CandidatePath) -> bool {
    candidate
        .nodes
        .len()
        .cmp(&current.nodes.len())
        .then_with(|| {
            authority_rank_key(current.minimum_handoff_speed_cap_mps)
                .cmp(&authority_rank_key(candidate.minimum_handoff_speed_cap_mps))
        })
        .then_with(|| candidate.route_length_m.total_cmp(&current.route_length_m))
        .then_with(|| {
            candidate
                .peak_extra_loft_m
                .total_cmp(&current.peak_extra_loft_m)
        })
        .then_with(|| candidate_identity(candidate).cmp(&candidate_identity(current)))
        == Ordering::Less
}

fn authority_rank_key(cap_mps: f64) -> i64 {
    (cap_mps / AUTHORITY_RANK_QUANTUM_MPS).round() as i64
}

fn candidate_identity(path: &CandidatePath) -> String {
    path.nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn candidate_ordering(left: &CandidatePath, right: &CandidatePath) -> Ordering {
    if candidate_precedes(left, right) {
        Ordering::Less
    } else if candidate_precedes(right, left) {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

/// Reconstruct only the comparator's immutable path metadata from an exposed
/// plan. The source/target endpoints are present in the canonical shaped
/// centerline, so this does not need to recreate a planning request.
fn candidate_path_from_plan(candidate: &PlannerCandidateV1) -> Result<CandidatePath, String> {
    if candidate.plan.route.waypoints.len() != candidate.node_ids.len() {
        return Err("waypoint count does not match node identity".to_owned());
    }
    let Some(source_world) = candidate
        .plan
        .diagnostics
        .selected_centerline_m
        .first()
        .copied()
    else {
        return Err("selected centerline has no source endpoint".to_owned());
    };
    let Some(target_world) = candidate
        .plan
        .diagnostics
        .selected_centerline_m
        .last()
        .copied()
    else {
        return Err("selected centerline has no target endpoint".to_owned());
    };
    let geometry = &candidate.plan.normalized_geometry;
    let sign = f64::from(geometry.horizontal_sign);
    let source = Vec2::new(0.0, source_world.y);
    let target = Vec2::new(geometry.direct_horizontal_span_m, target_world.y);
    let nodes = candidate
        .plan
        .route
        .waypoints
        .iter()
        .zip(&candidate.node_ids)
        .map(|(waypoint, node_id)| Node {
            id: node_id.clone(),
            point: Vec2::new(
                sign * (waypoint.position_m.x - source_world.x),
                waypoint.position_m.y,
            ),
            loft_cap_y: 0.0,
        })
        .collect::<Vec<_>>();
    let minimum_handoff_speed_cap_mps = candidate
        .plan
        .diagnostics
        .waypoint_authority
        .iter()
        .map(|authority| authority.handoff_speed_cap_mps)
        .min_by(f64::total_cmp)
        .unwrap_or(0.0);
    let mut previous = source;
    let mut route_length_m = 0.0;
    let mut peak_extra_loft_m: f64 = 0.0;
    for node in &nodes {
        route_length_m += (node.point - previous).length();
        peak_extra_loft_m =
            peak_extra_loft_m.max(extra_loft(node.point, geometry, source.y, target.y));
        previous = node.point;
    }
    route_length_m += (target - previous).length();
    Ok(CandidatePath {
        nodes,
        minimum_handoff_speed_cap_mps,
        route_length_m,
        peak_extra_loft_m,
    })
}

fn selected_candidate_path(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    plan: &RoutePlan,
) -> CandidatePath {
    let source_x = request.source_pad().expect("validated source").center_x_m;
    let sign = f64::from(geometry.horizontal_sign);
    let loft_limit = request.policy.max_extra_loft_ratio * geometry.direct_distance_m;
    let source = source_point(request, geometry);
    let target = target_point(request, geometry);
    let nodes = plan
        .route
        .waypoints
        .iter()
        .map(|waypoint| {
            let point = Vec2::new(
                sign * (waypoint.position_m.x - source_x),
                waypoint.position_m.y,
            );
            let direct_y = lerp(
                source.y,
                target.y,
                point.x / geometry.direct_horizontal_span_m,
            );
            Node {
                id: waypoint.id.clone(),
                point,
                loft_cap_y: direct_y + loft_limit,
            }
        })
        .collect::<Vec<_>>();
    let minimum_handoff_speed_cap_mps = plan
        .diagnostics
        .waypoint_authority
        .iter()
        .map(|authority| authority.handoff_speed_cap_mps)
        .min_by(f64::total_cmp)
        .unwrap_or(0.0);
    let mut previous = source;
    let mut route_length_m = 0.0;
    let mut peak_extra_loft_m: f64 = 0.0;
    for node in &nodes {
        route_length_m += (node.point - previous).length();
        peak_extra_loft_m =
            peak_extra_loft_m.max(extra_loft(node.point, geometry, source.y, target.y));
        previous = node.point;
    }
    route_length_m += (target - previous).length();
    CandidatePath {
        nodes,
        minimum_handoff_speed_cap_mps,
        route_length_m,
        peak_extra_loft_m,
    }
}

fn exposure_rejection(message: String) -> PlanningRejection {
    PlanningRejection::new(PlanningRejectionCode::UnsupportedGeometry, message)
}

fn edge_clear(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    profile: &SafetyProfile,
    start: Vec2,
    end: Vec2,
) -> Result<bool, PlanningRejection> {
    if end.x <= start.x {
        return Ok(false);
    }
    if end.y < start.y - 1.0e-9 {
        // Descents are valid; this branch is intentionally empty.  Keeping
        // the explicit comparison documents that no monotone vertical policy
        // is being smuggled into V1.
    }
    let edge_points = shaped_edge_points(request, geometry, start, end)?;
    for edge in edge_points.windows(2) {
        let segment_start = edge[0];
        let segment_end = edge[1];
        if segment_end.x <= segment_start.x {
            return Ok(false);
        }
        for (left, right) in profile.breakpoints_between(segment_start.x, segment_end.x) {
            let t0 = (left - segment_start.x) / (segment_end.x - segment_start.x);
            let t1 = (right - segment_start.x) / (segment_end.x - segment_start.x);
            let interval_start = interpolate(segment_start, segment_end, t0);
            let interval_end = interpolate(segment_start, segment_end, t1);
            let start_world = denormalize(interval_start, request, geometry);
            let end_world = denormalize(interval_end, request, geometry);
            let clearance = request
                .world
                .terrain
                .exact_corridor_clearance(
                    start_world,
                    end_world,
                    profile.envelope_at(left),
                    profile.envelope_at(right),
                )
                .map_err(rejection_from_query)?;
            if !clearance.clear {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn shaped_edge_points(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    start: Vec2,
    end: Vec2,
) -> Result<Vec<Vec2>, PlanningRejection> {
    let profile = build_endpoint_profile(request, geometry.direct_horizontal_span_m)
        .map_err(rejection_from_validation)?
        .0;
    let baseline = endpoint_shaped_centerline(request, geometry, &profile, &[])
        .map_err(rejection_from_validation)?;
    let mut points = vec![start];
    points.extend(
        baseline
            .into_iter()
            .filter(|point| point.x > start.x + 1.0e-9 && point.x < end.x - 1.0e-9),
    );
    points.push(end);
    points.sort_by(|lhs, rhs| lhs.x.total_cmp(&rhs.x));
    points.dedup_by(|lhs, rhs| (lhs.x - rhs.x).abs() <= 1.0e-9 && (lhs.y - rhs.y).abs() <= 1.0e-9);
    Ok(points)
}

fn exact_path_clearance(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    profile: &SafetyProfile,
    points: &[Vec2],
) -> Result<CorridorClearance, PlanningRejection> {
    let mut worst: Option<CorridorClearance> = None;
    for pair in points.windows(2) {
        for (left, right) in profile.breakpoints_between(pair[0].x, pair[1].x) {
            let t0 = (left - pair[0].x) / (pair[1].x - pair[0].x);
            let t1 = (right - pair[0].x) / (pair[1].x - pair[0].x);
            let start = interpolate(pair[0], pair[1], t0);
            let end = interpolate(pair[0], pair[1], t1);
            let clearance = request
                .world
                .terrain
                .exact_corridor_clearance(
                    denormalize(start, request, geometry),
                    denormalize(end, request, geometry),
                    profile.envelope_at(left),
                    profile.envelope_at(right),
                )
                .map_err(rejection_from_query)?;
            if worst.as_ref().is_none_or(|current| {
                clearance.worst_residual.residual_m > current.worst_residual.residual_m
            }) {
                worst = Some(clearance);
            }
        }
    }
    worst.ok_or_else(|| {
        PlanningRejection::new(
            PlanningRejectionCode::NoRouteWithinPolicy,
            "path produced no profile intervals",
        )
    })
}

fn build_success_plan(
    request: &RoutePlanningRequest,
    request_digest: String,
    geometry: NormalizedRouteGeometry,
    profile: SafetyProfile,
    path: CandidatePath,
    direct_clearance: CorridorClearance,
    safe_profile_points: Vec<Vec2>,
) -> Result<RoutePlan, PlanningRejection> {
    let normalized_points = std::iter::once(source_point(request, &geometry))
        .chain(path.nodes.iter().map(|node| node.point))
        .chain(std::iter::once(target_point(request, &geometry)))
        .collect::<Vec<_>>();
    let node_ids = path
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    let route = route_from_points(request, &geometry, &normalized_points, &node_ids)?;
    let validation = validate_route(request, &route)
        .map_err(|error| PlanningRejection::new(error.rejection_code(), error.to_string()))?;
    // Keep profile in this function's signature to make it impossible to
    // accidentally construct a route without having built endpoint policy;
    // exact validation above remains authoritative.
    let _ = profile;
    finalize_plan(
        request,
        request_digest,
        geometry,
        RouteTopology::Waypoint,
        route,
        node_ids,
        direct_clearance,
        validation,
        safe_profile_points,
    )
}

fn route_from_points(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    normalized_points: &[Vec2],
    node_ids: &[String],
) -> Result<TransferRouteSpec, PlanningRejection> {
    let source = request.source_pad().expect("validated source");
    let target = request.target_pad().expect("validated target");
    let shaped_data = shaped_route_data(request, geometry, normalized_points)?;
    let mut waypoints = Vec::with_capacity(normalized_points.len().saturating_sub(2));
    for (index, point) in normalized_points
        .iter()
        .enumerate()
        .skip(1)
        .take(normalized_points.len().saturating_sub(2))
    {
        let shape_index = shaped_data.waypoint_shape_indices[index - 1];
        let previous = shaped_data.shaped_points[shape_index - 1];
        let next = shaped_data.shaped_points[shape_index + 1];
        let inbound = (Vec2::new(point.x - previous.x, point.y - previous.y)).normalized();
        let outbound = (Vec2::new(next.x - point.x, next.y - point.y)).normalized();
        let tangent = (inbound + outbound).normalized_or(outbound);
        let authority = &shaped_data.waypoint_authority[index - 1];
        let max_speed = authority.handoff_speed_cap_mps;
        let id = node_ids
            .get(index - 1)
            .cloned()
            .unwrap_or_else(|| format!("waypoint-{index}"));
        waypoints.push(TransferWaypointSpec {
            id,
            position_m: denormalize(*point, request, geometry),
            handoff_tangent_unit: Some(denormalize_vector(tangent, geometry)),
            capture_radius_m: shaped_data.capture_radius_m,
            max_cross_track_m: shaped_data.capture_radius_m,
            max_outbound_heading_error_rad: request.policy.max_outbound_heading_error_rad,
            min_outbound_progress_mps: request.policy.min_outbound_progress_mps,
            max_outbound_cross_speed_mps: Some(request.policy.max_outbound_cross_speed_mps),
            min_speed_mps: request.policy.min_handoff_speed_mps,
            max_speed_mps: max_speed,
            min_vertical_speed_mps: None,
            max_vertical_speed_mps: None,
        });
    }
    // Explicitly derive these values from pad centers; do not accept route
    // metadata supplied by a caller or candidate node.
    Ok(TransferRouteSpec {
        source_pad_id: source.id.clone(),
        target_pad_id: target.id.clone(),
        route_angle_deg: geometry.route_angle_deg,
        route_radius_m: geometry.direct_distance_m,
        waypoints,
    })
}

struct ShapedRouteData {
    shaped_points: Vec<Vec2>,
    waypoint_shape_indices: Vec<usize>,
    waypoint_authority: Vec<pd_core::WaypointAuthorityDiagnostics>,
    capture_radius_m: f64,
}

/// Construct exactly the endpoint-shaped points and waypoint authority data
/// emitted by `route_from_points`. Candidate screening uses this helper so a
/// route cannot pass search with an envelope that construction later rejects.
fn shaped_route_data(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    normalized_points: &[Vec2],
) -> Result<ShapedRouteData, PlanningRejection> {
    if normalized_points.len() < 2 {
        return Err(PlanningRejection::new(
            PlanningRejectionCode::InvalidRequest,
            "route needs source and target points",
        ));
    }
    let profile = build_endpoint_profile(request, geometry.direct_horizontal_span_m)
        .map_err(rejection_from_validation)?
        .0;
    let shaped_points = endpoint_shaped_centerline(
        request,
        geometry,
        &profile,
        &normalized_points[1..normalized_points.len().saturating_sub(1)],
    )
    .map_err(rejection_from_validation)?;
    let capture_radius_m = (geometry.direct_distance_m * 0.08).clamp(35.0, 95.0);
    let mut waypoint_shape_indices = Vec::with_capacity(normalized_points.len().saturating_sub(2));
    let mut waypoint_authority = Vec::with_capacity(normalized_points.len().saturating_sub(2));
    for point in normalized_points
        .iter()
        .skip(1)
        .take(normalized_points.len().saturating_sub(2))
    {
        let shape_index = shaped_points
            .iter()
            .position(|candidate| {
                (candidate.x - point.x).abs() <= 1.0e-9 && (candidate.y - point.y).abs() <= 1.0e-9
            })
            .ok_or_else(|| {
                PlanningRejection::new(
                    PlanningRejectionCode::UnsupportedGeometry,
                    "waypoint was not retained in endpoint-shaped centerline",
                )
            })?;
        if shape_index == 0 || shape_index + 1 >= shaped_points.len() {
            return Err(PlanningRejection::new(
                PlanningRejectionCode::UnsupportedGeometry,
                "waypoint has no adjacent shaped centerline points",
            ));
        }
        let previous = denormalize(shaped_points[shape_index - 1], request, geometry);
        let position = denormalize(shaped_points[shape_index], request, geometry);
        let next = denormalize(shaped_points[shape_index + 1], request, geometry);
        let authority =
            compute_waypoint_authority(request, previous, position, next, capture_radius_m)
                .map_err(|error| {
                    PlanningRejection::new(error.rejection_code(), error.to_string())
                })?;
        waypoint_shape_indices.push(shape_index);
        waypoint_authority.push(authority);
    }
    Ok(ShapedRouteData {
        shaped_points,
        waypoint_shape_indices,
        waypoint_authority,
        capture_radius_m,
    })
}

#[allow(clippy::too_many_arguments)]
fn finalize_plan(
    request: &RoutePlanningRequest,
    request_digest: String,
    geometry: NormalizedRouteGeometry,
    topology: RouteTopology,
    route: TransferRouteSpec,
    selected_node_ids: Vec<String>,
    direct_clearance: CorridorClearance,
    validation: pd_core::RouteValidation,
    safe_profile_points: Vec<Vec2>,
) -> Result<RoutePlan, PlanningRejection> {
    let diagnostics = RoutePlanDiagnostics {
        direct_path_clear: direct_clearance.clear,
        direct_path_clearance: Some(direct_clearance),
        route_length_m: validation.route_length_m,
        direct_distance_m: geometry.direct_distance_m,
        excess_length_m: (validation.route_length_m - geometry.direct_distance_m).max(0.0),
        peak_extra_loft_m: validation.peak_extra_loft_m,
        minimum_planned_clearance_m: validation.minimum_planned_clearance_m,
        leg_diagnostics: validation.leg_diagnostics,
        selected_node_ids: selected_node_ids.clone(),
        safe_profile_points_m: safe_profile_points,
        selected_centerline_m: validation.selected_centerline_m,
        waypoint_authority: validation.waypoint_authority,
    };
    let mut plan = RoutePlan {
        algorithm_id: ALGORITHM_ID.to_owned(),
        policy: request.policy.clone(),
        request_digest,
        plan_digest: String::new(),
        topology,
        route,
        normalized_geometry: geometry,
        diagnostics,
    };
    plan.plan_digest = digest(&PlanDigestInput {
        algorithm_id: &plan.algorithm_id,
        policy: &plan.policy,
        request_digest: &plan.request_digest,
        topology: plan.topology,
        route: &plan.route,
        diagnostics: &plan.diagnostics,
    })?;
    Ok(plan)
}

#[derive(Serialize)]
struct PlanDigestInput<'a> {
    algorithm_id: &'a str,
    policy: &'a RoutePlanningPolicy,
    request_digest: &'a str,
    topology: RouteTopology,
    route: &'a TransferRouteSpec,
    diagnostics: &'a RoutePlanDiagnostics,
}

fn digest<T: Serialize>(value: &T) -> Result<String, PlanningRejection> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        PlanningRejection::new(
            PlanningRejectionCode::InvalidRequest,
            format!("cannot serialize planning digest input: {error}"),
        )
    })?;
    // FNV-1a is small, dependency-free, and deterministic.  The digest is a
    // cache/provenance identity, not a cryptographic signature.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("fnv1a64:{hash:016x}"))
}

fn rejection_from_validation(error: pd_core::RouteValidationError) -> PlanningRejection {
    PlanningRejection::new(error.rejection_code(), error.to_string())
}

fn rejection_from_query(error: pd_core::TerrainQueryError) -> PlanningRejection {
    let code = match error {
        pd_core::TerrainQueryError::DomainOverrun { .. } => {
            PlanningRejectionCode::UnsupportedGeometry
        }
        pd_core::TerrainQueryError::InvalidTerrain { .. }
        | pd_core::TerrainQueryError::InvalidCorridor { .. } => {
            PlanningRejectionCode::InvalidRequest
        }
    };
    PlanningRejection::new(code, error.to_string())
}

fn source_point(request: &RoutePlanningRequest, _geometry: &NormalizedRouteGeometry) -> Vec2 {
    let pad = request.source_pad().expect("validated source");
    Vec2::new(
        0.0,
        pad.surface_y_m + request.vehicle.geometry.touchdown_base_offset_m,
    )
}

fn target_point(request: &RoutePlanningRequest, geometry: &NormalizedRouteGeometry) -> Vec2 {
    let pad = request.target_pad().expect("validated target");
    Vec2::new(
        geometry.direct_horizontal_span_m,
        pad.surface_y_m + request.vehicle.geometry.touchdown_base_offset_m,
    )
}

fn denormalize(
    point: Vec2,
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
) -> Vec2 {
    let source_x = request.source_pad().expect("validated source").center_x_m;
    Vec2::new(
        source_x + point.x * f64::from(geometry.horizontal_sign),
        point.y,
    )
}

fn denormalize_vector(vector: Vec2, geometry: &NormalizedRouteGeometry) -> Vec2 {
    Vec2::new(vector.x * f64::from(geometry.horizontal_sign), vector.y)
}

fn extra_loft(
    point: Vec2,
    geometry: &NormalizedRouteGeometry,
    source_y: f64,
    target_y: f64,
) -> f64 {
    // Caller only needs a stable candidate tie-break.  Route validation
    // recomputes the exact global profile; this local conservative metric is
    // relative to the source/target pad surface chord.
    let direct_y = lerp(
        source_y,
        target_y,
        point.x / geometry.direct_horizontal_span_m,
    );
    (point.y - direct_y).max(0.0)
}

fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + ((end - start) * t)
}

fn interpolate(start: Vec2, end: Vec2, t: f64) -> Vec2 {
    Vec2::new(lerp(start.x, end.x, t), lerp(start.y, end.y, t))
}

trait Vec2Ext {
    fn normalized(self) -> Vec2;
    fn normalized_or(self, fallback: Vec2) -> Vec2;
}

impl Vec2Ext for Vec2 {
    fn normalized(self) -> Vec2 {
        let length = self.length();
        if length <= 1.0e-12 {
            Vec2::new(0.0, 0.0)
        } else {
            Vec2::new(self.x / length, self.y / length)
        }
    }

    fn normalized_or(self, fallback: Vec2) -> Vec2 {
        if self.length() <= 1.0e-12 {
            fallback.normalized()
        } else {
            self.normalized()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::{
        LandingPadSpec, TerrainDefinition, VehicleGeometry, VehicleInitialState, VehicleSpec,
        WorldSpec,
    };

    fn request(terrain_points: Vec<Vec2>) -> RoutePlanningRequest {
        RoutePlanningRequest {
            world: WorldSpec {
                gravity_mps2: 9.8,
                terrain: TerrainDefinition::Heightfield {
                    points_m: terrain_points
                        .into_iter()
                        .map(|point| {
                            Vec2::new(point.x, if point.y == 0.0 { 100.0 } else { point.y })
                        })
                        .collect(),
                },
                landing_pads: vec![
                    LandingPadSpec {
                        id: "source".to_owned(),
                        center_x_m: 0.0,
                        surface_y_m: 100.0,
                        width_m: 20.0,
                    },
                    LandingPadSpec {
                        id: "target".to_owned(),
                        center_x_m: 200.0,
                        surface_y_m: 100.0,
                        width_m: 20.0,
                    },
                ],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 10.0,
                    hull_height_m: 10.0,
                    touchdown_half_span_m: 5.0,
                    touchdown_base_offset_m: 5.0,
                },
                dry_mass_kg: 1.0,
                initial_fuel_kg: 1.0,
                max_fuel_kg: 2.0,
                max_thrust_n: 100.0,
                max_fuel_burn_kgps: 1.0,
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.0,
                safe_touchdown_normal_speed_mps: 1.0,
                safe_touchdown_tangential_speed_mps: 1.0,
                safe_touchdown_attitude_error_rad: 1.0,
                safe_touchdown_angular_rate_radps: 1.0,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(0.0, 105.0),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            source_pad_id: "source".to_owned(),
            target_pad_id: "target".to_owned(),
            policy: RoutePlanningPolicy::default(),
        }
    }

    #[test]
    fn level_carved_corridor_accepts_clear_direct_route() {
        let mut request = request(vec![
            Vec2::new(-200.0, 100.0),
            Vec2::new(20.0, 100.0),
            Vec2::new(50.0, 0.0),
            Vec2::new(150.0, 0.0),
            Vec2::new(180.0, 100.0),
            Vec2::new(400.0, 100.0),
        ]);
        request.world.landing_pads[0].width_m = 40.0;
        request.world.landing_pads[1].width_m = 40.0;
        request.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-200.0, 100.0),
                Vec2::new(20.0, 100.0),
                Vec2::new(50.0, 0.0),
                Vec2::new(150.0, 0.0),
                Vec2::new(180.0, 100.0),
                Vec2::new(400.0, 100.0),
            ],
        };
        let plan = plan(&request).unwrap();
        assert_eq!(plan.topology, RouteTopology::Direct);
        assert!(plan.route.waypoints.is_empty());
        assert!(plan.diagnostics.direct_path_clear);
    }

    #[test]
    fn direct_attempt_and_candidate_search_are_deterministic() {
        let mut request = request(vec![
            Vec2::new(-100.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, -100.0),
            Vec2::new(180.0, -100.0),
            Vec2::new(190.0, 0.0),
            Vec2::new(300.0, 0.0),
        ]);
        request.world.landing_pads[0].width_m = 2.0;
        request.world.landing_pads[1].surface_y_m = 100.0;
        request.world.landing_pads[1].width_m = 2.0;
        request.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-100.0, 100.0),
                Vec2::new(1.0, 100.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(80.0, 0.0),
                Vec2::new(100.0, 130.0),
                Vec2::new(120.0, 0.0),
                Vec2::new(198.0, 0.0),
                Vec2::new(199.0, 100.0),
                Vec2::new(300.0, 100.0),
            ],
        };
        let first = plan(&request).unwrap();
        let second = plan(&request).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.topology, RouteTopology::Waypoint);
        assert!(!first.diagnostics.direct_path_clear);
        assert_eq!(first.topology, second.topology);
        assert_eq!(first.request_digest, second.request_digest);
        assert_eq!(first.plan_digest, second.plan_digest);
        assert!(!first.diagnostics.safe_profile_points_m.is_empty());
        assert!(!first.diagnostics.selected_centerline_m.is_empty());
    }

    #[test]
    fn candidate_exposure_direct_route_is_complete_and_selected() {
        let mut request = request(vec![
            Vec2::new(-200.0, 100.0),
            Vec2::new(20.0, 100.0),
            Vec2::new(50.0, 0.0),
            Vec2::new(150.0, 0.0),
            Vec2::new(180.0, 100.0),
            Vec2::new(400.0, 100.0),
        ]);
        request.world.landing_pads[0].width_m = 40.0;
        request.world.landing_pads[1].width_m = 40.0;
        request.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-200.0, 100.0),
                Vec2::new(20.0, 100.0),
                Vec2::new(50.0, 0.0),
                Vec2::new(150.0, 0.0),
                Vec2::new(180.0, 100.0),
                Vec2::new(400.0, 100.0),
            ],
        };
        let exposure = expose_candidates(&request).unwrap();
        assert!(exposure.complete);
        assert_eq!(exposure.examined_path_count, 0);
        assert_eq!(exposure.candidates.len(), 1);
        assert!(!exposure.selected_plan_injected);
        exposure.validate().unwrap();
    }

    #[test]
    fn candidate_exposure_preserves_selected_rank_and_detects_plan_tamper() {
        let mut request = request(vec![
            Vec2::new(-100.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, -100.0),
            Vec2::new(180.0, -100.0),
            Vec2::new(190.0, 0.0),
            Vec2::new(300.0, 0.0),
        ]);
        request.world.landing_pads[0].width_m = 2.0;
        request.world.landing_pads[1].surface_y_m = 100.0;
        request.world.landing_pads[1].width_m = 2.0;
        request.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-100.0, 100.0),
                Vec2::new(1.0, 100.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(80.0, 0.0),
                Vec2::new(100.0, 130.0),
                Vec2::new(120.0, 0.0),
                Vec2::new(198.0, 0.0),
                Vec2::new(199.0, 100.0),
                Vec2::new(300.0, 100.0),
            ],
        };
        let selected = plan(&request).unwrap();
        let exposure = expose_candidates(&request).unwrap();
        assert_eq!(exposure.candidates[0].plan, selected);
        assert_eq!(exposure.candidates[0].rank, 0);
        exposure.validate().unwrap();

        let selected_bytes = serde_json::to_vec(&selected).unwrap();
        let selected_round_trip: RoutePlan = serde_json::from_slice(&selected_bytes).unwrap();
        assert_eq!(selected_round_trip, selected);
        assert_eq!(selected_round_trip.plan_digest, selected.plan_digest);
        assert_eq!(
            serde_json::to_vec(&selected_round_trip).unwrap(),
            selected_bytes
        );

        let exposure_bytes = serde_json::to_vec(&exposure).unwrap();
        let exposure_round_trip: PlannerCandidateExposureV1 =
            serde_json::from_slice(&exposure_bytes).unwrap();
        assert_eq!(exposure_round_trip, exposure);
        exposure_round_trip.validate().unwrap();
        assert_eq!(
            candidate_exposure_digest(&exposure_round_trip),
            exposure.exposure_digest
        );
        assert_eq!(
            serde_json::to_vec(&exposure_round_trip).unwrap(),
            exposure_bytes
        );

        let mut tampered = exposure.clone();
        tampered.candidates[0].plan.route.route_angle_deg += 1.0;
        assert!(tampered.validate().is_err());
    }

    #[test]
    fn candidate_exposure_rejects_ranking_metric_and_count_tamper() {
        let mut request = request(vec![
            Vec2::new(-100.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, -100.0),
            Vec2::new(180.0, -100.0),
            Vec2::new(190.0, 0.0),
            Vec2::new(300.0, 0.0),
        ]);
        request.world.landing_pads[0].width_m = 2.0;
        request.world.landing_pads[1].surface_y_m = 100.0;
        request.world.landing_pads[1].width_m = 2.0;
        request.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-100.0, 100.0),
                Vec2::new(1.0, 100.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(80.0, 0.0),
                Vec2::new(100.0, 130.0),
                Vec2::new(120.0, 0.0),
                Vec2::new(198.0, 0.0),
                Vec2::new(199.0, 100.0),
                Vec2::new(300.0, 100.0),
            ],
        };
        let exposure = expose_candidates(&request).unwrap();

        let mut metric_tampered = exposure.clone();
        metric_tampered.candidates[0].route_length_m += 1.0;
        metric_tampered.exposure_digest = candidate_exposure_digest(&metric_tampered);
        assert!(metric_tampered.validate().is_err());

        let mut count_tampered = exposure.clone();
        count_tampered.accepted_candidate_count =
            count_tampered.accepted_candidate_count.saturating_add(1);
        count_tampered.exposure_digest = candidate_exposure_digest(&count_tampered);
        assert!(count_tampered.validate().is_err());
    }

    #[test]
    fn candidate_exposure_accepts_and_checks_synthetic_path_truncation() {
        let mut request = request(vec![
            Vec2::new(-100.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, -100.0),
            Vec2::new(180.0, -100.0),
            Vec2::new(190.0, 0.0),
            Vec2::new(300.0, 0.0),
        ]);
        request.world.landing_pads[0].width_m = 2.0;
        request.world.landing_pads[1].surface_y_m = 100.0;
        request.world.landing_pads[1].width_m = 2.0;
        request.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-100.0, 100.0),
                Vec2::new(1.0, 100.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(80.0, 0.0),
                Vec2::new(100.0, 130.0),
                Vec2::new(120.0, 0.0),
                Vec2::new(198.0, 0.0),
                Vec2::new(199.0, 100.0),
                Vec2::new(300.0, 100.0),
            ],
        };
        let exposure = expose_candidates(&request).unwrap();
        let mut truncated = exposure.clone();
        truncated.complete = false;
        truncated.path_truncated = true;
        truncated.truncation_reasons = vec!["path_budget_exhausted".to_owned()];
        truncated.examined_path_count = truncated.limits.max_examined_paths;
        truncated.exposure_digest = candidate_exposure_digest(&truncated);
        truncated.validate().unwrap();

        truncated.examined_path_count -= 1;
        truncated.exposure_digest = candidate_exposure_digest(&truncated);
        assert!(truncated.validate().is_err());
    }

    #[test]
    fn exact_validation_skips_higher_invalid_single_ridge_candidate() {
        let mut request = request(vec![
            Vec2::new(-100.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, -20.0),
            Vec2::new(390.0, -20.0),
            Vec2::new(400.0, 130.0),
            Vec2::new(410.0, -20.0),
            Vec2::new(780.0, -20.0),
            Vec2::new(790.0, 0.0),
            Vec2::new(900.0, 0.0),
        ]);
        request.world.landing_pads[1].center_x_m = 800.0;
        let plan = plan(&request).unwrap();
        assert_eq!(plan.topology, RouteTopology::Waypoint);
        assert_eq!(plan.route.waypoints.len(), 1);
        let waypoint = &plan.route.waypoints[0];
        assert!(waypoint.position_m.x > 0.0 && waypoint.position_m.x < 800.0);
        assert!(plan.diagnostics.minimum_planned_clearance_m >= 0.0);
        let loft_cap =
            request.policy.max_extra_loft_ratio * plan.normalized_geometry.direct_distance_m;
        // The graph also contains a bounded loft-cap candidate.  Exact route
        // validation must discard it when its shaped final leg exceeds the
        // policy, allowing the lower valid support candidate to win.
        assert!(plan.diagnostics.peak_extra_loft_m < loft_cap - 1.0e-6);
    }

    #[test]
    fn separated_ridges_fit_two_monotone_waypoints() {
        let mut request = request(vec![
            Vec2::new(-100.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, -20.0),
            Vec2::new(190.0, -20.0),
            Vec2::new(200.0, 130.0),
            Vec2::new(210.0, -20.0),
            Vec2::new(590.0, -20.0),
            Vec2::new(600.0, 130.0),
            Vec2::new(610.0, -20.0),
            Vec2::new(780.0, -20.0),
            Vec2::new(790.0, 0.0),
            Vec2::new(900.0, 0.0),
        ]);
        request.world.landing_pads[1].center_x_m = 800.0;
        let plan = plan(&request).unwrap();
        assert_eq!(plan.route.waypoints.len(), 2);
        assert!(
            plan.route
                .waypoints
                .windows(2)
                .all(|pair| pair[1].position_m.x > pair[0].position_m.x)
        );
        let exposure = expose_candidates(&request).unwrap();
        assert_eq!(exposure.candidates[0].plan, plan);
        assert!(exposure.candidates.len() >= 3);
        exposure.validate().unwrap();

        let mut reordered = exposure.clone();
        reordered.candidates.swap(1, 2);
        reordered.candidates[1].rank = 1;
        reordered.candidates[2].rank = 2;
        reordered.exposure_digest = candidate_exposure_digest(&reordered);
        assert!(reordered.validate().is_err());
    }

    #[test]
    fn candidate_ranking_prefers_maximin_authority_before_length() {
        let low_authority = CandidatePath {
            nodes: vec![Node {
                id: "low".to_owned(),
                point: Vec2::new(100.0, 200.0),
                loft_cap_y: 500.0,
            }],
            minimum_handoff_speed_cap_mps: 20.0,
            route_length_m: 100.0,
            peak_extra_loft_m: 10.0,
        };
        let high_authority = CandidatePath {
            nodes: vec![Node {
                id: "high".to_owned(),
                point: Vec2::new(110.0, 210.0),
                loft_cap_y: 500.0,
            }],
            minimum_handoff_speed_cap_mps: 21.0,
            route_length_m: 200.0,
            peak_extra_loft_m: 20.0,
        };
        assert!(candidate_precedes(&high_authority, &low_authority));
        assert!(!candidate_precedes(&low_authority, &high_authority));
    }

    #[test]
    fn candidate_ranking_ties_subnanometer_authority_before_length() {
        let shorter = CandidatePath {
            nodes: vec![Node {
                id: "shorter".to_owned(),
                point: Vec2::new(100.0, 200.0),
                loft_cap_y: 500.0,
            }],
            minimum_handoff_speed_cap_mps: 20.0,
            route_length_m: 100.0,
            peak_extra_loft_m: 10.0,
        };
        let nearly_equal_authority = CandidatePath {
            nodes: vec![Node {
                id: "nearly-equal".to_owned(),
                point: Vec2::new(110.0, 210.0),
                loft_cap_y: 500.0,
            }],
            minimum_handoff_speed_cap_mps: 20.0 + 4.0e-15,
            route_length_m: 200.0,
            peak_extra_loft_m: 20.0,
        };
        assert!(candidate_precedes(&shorter, &nearly_equal_authority));
        assert!(!candidate_precedes(&nearly_equal_authority, &shorter));
    }

    #[test]
    fn profile_includes_interior_affine_owner_switch() {
        let request = request(vec![
            Vec2::new(-200.0, 100.0),
            Vec2::new(10.0, 100.0),
            Vec2::new(50.0, 130.0),
            Vec2::new(100.0, 100.0),
            Vec2::new(130.0, 200.0),
            Vec2::new(150.0, 130.0),
            Vec2::new(180.0, 100.0),
            Vec2::new(400.0, 100.0),
        ]);
        let geometry = normalized_geometry(&request).unwrap();
        let (profile, _) =
            build_endpoint_profile(&request, geometry.direct_horizontal_span_m).unwrap();
        let terrain = normalized_terrain(&request, &geometry).unwrap();
        let events = safety_profile_events(&profile, &terrain, geometry.direct_horizontal_span_m);
        assert!(events.iter().any(|event| *event > 140.0 && *event < 140.2));
    }

    #[test]
    fn three_ridge_waypoint_bound_maps_to_complexity_rejection() {
        let mut request = request(vec![
            Vec2::new(-100.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, -20.0),
            Vec2::new(140.0, -20.0),
            Vec2::new(150.0, 130.0),
            Vec2::new(160.0, -20.0),
            Vec2::new(390.0, -20.0),
            Vec2::new(400.0, 130.0),
            Vec2::new(410.0, -20.0),
            Vec2::new(640.0, -20.0),
            Vec2::new(650.0, 130.0),
            Vec2::new(660.0, -20.0),
            Vec2::new(780.0, -20.0),
            Vec2::new(790.0, 0.0),
            Vec2::new(900.0, 0.0),
        ]);
        request.world.landing_pads[1].center_x_m = 800.0;
        request.policy.max_waypoints = 1;
        let rejection = plan(&request).unwrap_err();
        assert_eq!(
            rejection.code,
            PlanningRejectionCode::RouteComplexityExceeded
        );
    }

    #[test]
    fn all_exact_candidates_above_loft_cap_preserve_loft_rejection() {
        let mut request = request(vec![
            Vec2::new(-100.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, -20.0),
            Vec2::new(390.0, -20.0),
            Vec2::new(400.0, 180.0),
            Vec2::new(410.0, -20.0),
            Vec2::new(780.0, -20.0),
            Vec2::new(790.0, 0.0),
            Vec2::new(900.0, 0.0),
        ]);
        request.world.landing_pads[1].center_x_m = 800.0;
        request.policy.max_extra_loft_ratio = 0.05;
        let rejection = plan(&request).unwrap_err();
        assert_eq!(rejection.code, PlanningRejectionCode::LoftLimitExceeded);
    }

    #[test]
    fn all_exact_candidates_without_authority_preserve_authority_rejection() {
        let mut request = request(vec![
            Vec2::new(-100.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, -20.0),
            Vec2::new(390.0, -20.0),
            Vec2::new(400.0, 130.0),
            Vec2::new(410.0, -20.0),
            Vec2::new(780.0, -20.0),
            Vec2::new(790.0, 0.0),
            Vec2::new(900.0, 0.0),
        ]);
        request.world.landing_pads[1].center_x_m = 800.0;
        request.vehicle.max_thrust_n = 19.7;
        let rejection = plan(&request).unwrap_err();
        assert_eq!(rejection.code, PlanningRejectionCode::InsufficientAuthority);
    }

    #[test]
    fn terrain_domain_overrun_is_unsupported_geometry() {
        let mut request = request(vec![Vec2::new(-100.0, 0.0), Vec2::new(210.0, 0.0)]);
        request.vehicle.geometry.touchdown_half_span_m = 12.0;
        let rejection = plan(&request).unwrap_err();
        assert_eq!(rejection.code, PlanningRejectionCode::UnsupportedGeometry);
    }

    #[test]
    fn malformed_request_maps_to_stable_rejection() {
        let mut request = request(vec![Vec2::new(-100.0, 0.0), Vec2::new(300.0, 0.0)]);
        request.source_pad_id = "missing".to_owned();
        let rejection = plan(&request).unwrap_err();
        assert_eq!(rejection.code.as_str(), "invalid_request");
    }

    #[test]
    fn policy_json_is_complete_and_versioned() {
        let policy = serde_json::to_value(RoutePlanningPolicy::default()).unwrap();
        let mut missing = policy.clone();
        missing
            .as_object_mut()
            .expect("policy serializes as object")
            .remove("max_waypoints");
        assert!(serde_json::from_value::<RoutePlanningPolicy>(missing).is_err());

        let mut unknown_request =
            serde_json::to_value(request(vec![Vec2::new(-100.0, 0.0), Vec2::new(300.0, 0.0)]))
                .unwrap();
        unknown_request["policy"]["policy_version"] = serde_json::json!("unknown");
        let decoded: RoutePlanningRequest = serde_json::from_value(unknown_request).unwrap();
        let rejection = plan(&decoded).unwrap_err();
        assert_eq!(rejection.code, PlanningRejectionCode::InvalidRequest);
    }

    #[test]
    fn zero_endpoint_transition_is_rejected() {
        let mut request = request(vec![Vec2::new(-100.0, 0.0), Vec2::new(300.0, 0.0)]);
        request.policy.endpoint_transition_m = 0.0;
        let rejection = plan(&request).unwrap_err();
        assert_eq!(rejection.code, PlanningRejectionCode::InvalidRequest);
    }

    #[test]
    fn rejection_codes_serialize_with_stable_snake_case_labels() {
        let cases = [
            (PlanningRejectionCode::InvalidRequest, "invalid_request"),
            (
                PlanningRejectionCode::UnsupportedGeometry,
                "unsupported_geometry",
            ),
            (
                PlanningRejectionCode::LoftLimitExceeded,
                "loft_limit_exceeded",
            ),
            (
                PlanningRejectionCode::RouteComplexityExceeded,
                "route_complexity_exceeded",
            ),
            (
                PlanningRejectionCode::InsufficientAuthority,
                "insufficient_authority",
            ),
            (
                PlanningRejectionCode::NoRouteWithinPolicy,
                "no_route_within_policy",
            ),
        ];
        for (code, expected) in cases {
            assert_eq!(
                serde_json::to_string(&code).unwrap(),
                format!("\"{expected}\"")
            );
        }
    }

    #[test]
    fn reversed_pad_direction_preserves_normalized_progress() {
        let mut request = request(vec![
            Vec2::new(-100.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(50.0, -20.0),
            Vec2::new(750.0, -20.0),
            Vec2::new(780.0, 0.0),
            Vec2::new(900.0, 0.0),
        ]);
        request.world.landing_pads[0].center_x_m = 800.0;
        request.world.landing_pads[1].center_x_m = 0.0;
        request.world.landing_pads[0].width_m = 40.0;
        request.world.landing_pads[1].width_m = 40.0;
        request.initial_state.position_m.x = 800.0;
        let plan = plan(&request).unwrap();
        assert_eq!(plan.normalized_geometry.horizontal_sign, -1);
        assert_eq!(
            plan.diagnostics
                .selected_centerline_m
                .first()
                .expect("source centerline point")
                .x,
            800.0
        );
        assert_eq!(
            plan.diagnostics
                .selected_centerline_m
                .last()
                .expect("target centerline point")
                .x,
            0.0
        );
        for pair in plan
            .route
            .waypoints
            .windows(2)
            .map(|pair| (pair[0].position_m.x, pair[1].position_m.x))
        {
            assert!(pair.0 > pair.1);
        }
    }

    #[test]
    fn reversed_pad_direction_accepts_a_waypoint_and_world_tangent() {
        let mut request = request(vec![
            Vec2::new(-100.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, -20.0),
            Vec2::new(390.0, -20.0),
            Vec2::new(400.0, 130.0),
            Vec2::new(410.0, -20.0),
            Vec2::new(780.0, -20.0),
            Vec2::new(790.0, 0.0),
            Vec2::new(900.0, 0.0),
        ]);
        request.world.landing_pads[0].center_x_m = 800.0;
        request.world.landing_pads[1].center_x_m = 0.0;
        request.initial_state.position_m.x = 800.0;
        let plan = plan(&request).unwrap();
        assert_eq!(plan.topology, RouteTopology::Waypoint);
        assert_eq!(plan.route.waypoints.len(), 1);
        assert!(plan.route.waypoints[0].position_m.x < 800.0);
        assert!(plan.route.waypoints[0].handoff_tangent_unit.unwrap().x < 0.0);
        validate_route(&request, &plan.route).unwrap();
    }
}
