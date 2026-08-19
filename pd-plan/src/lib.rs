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
use serde::Serialize;

pub const ALGORITHM_ID: &str = pd_core::HEIGHTFIELD_VISIBILITY_ALGORITHM_ID;

/// Plan an owned request snapshot through the deterministic V1 search.
pub fn plan(request: &RoutePlanningRequest) -> Result<RoutePlan, PlanningRejection> {
    Planner.plan(request)
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
        let capped_path = find_path(
            request,
            &geometry,
            &profile,
            &capped_nodes,
            usize::from(request.policy.max_waypoints),
        )?;

        let capped_extended = find_path(
            request,
            &geometry,
            &profile,
            &capped_nodes,
            usize::from(request.policy.max_waypoints).saturating_add(1),
        )?;
        if capped_path.is_none()
            && capped_extended
                .as_ref()
                .is_some_and(|path| path.nodes.len() > usize::from(request.policy.max_waypoints))
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
        )?
        .is_some()
        {
            return Err(PlanningRejection::new(
                PlanningRejectionCode::RouteComplexityExceeded,
                "no route within the waypoint-count and loft policy",
            ));
        }
        Err(PlanningRejection::new(
            PlanningRejectionCode::NoRouteWithinPolicy,
            "candidate graph contains no route within policy",
        ))
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
    route_length_m: f64,
    peak_extra_loft_m: f64,
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
) -> Result<(), PlanningRejection> {
    let previous = current.last().map_or(source, |node| node.point);
    for index in next_index..nodes.len() {
        let node = &nodes[index];
        if !edge_clear(request, geometry, profile, previous, node.point)? {
            continue;
        }
        current.push(node.clone());
        if edge_clear(request, geometry, profile, node.point, target)? {
            consider_path(best, current.clone(), geometry, source, target);
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
            )?;
        }
        current.pop();
    }
    Ok(())
}

fn consider_path(
    best: &mut Option<CandidatePath>,
    nodes: Vec<Node>,
    geometry: &NormalizedRouteGeometry,
    source: Vec2,
    target: Vec2,
) {
    let capture_radius = ((target - source).length() * 0.08).clamp(35.0, 95.0);
    if nodes
        .windows(2)
        .any(|pair| (pair[1].point - pair[0].point).length() <= capture_radius * 2.0)
    {
        return;
    }
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
        route_length_m,
        peak_extra_loft_m,
    };
    let replace = best.as_ref().is_none_or(|current| {
        candidate
            .nodes
            .len()
            .cmp(&current.nodes.len())
            .then_with(|| candidate.route_length_m.total_cmp(&current.route_length_m))
            .then_with(|| {
                candidate
                    .peak_extra_loft_m
                    .total_cmp(&current.peak_extra_loft_m)
            })
            .then_with(|| candidate_identity(&candidate).cmp(&candidate_identity(current)))
            == Ordering::Less
    });
    if replace {
        *best = Some(candidate);
    }
}

fn candidate_identity(path: &CandidatePath) -> String {
    path.nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<Vec<_>>()
        .join("/")
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
    let mut waypoints = Vec::with_capacity(normalized_points.len().saturating_sub(2));
    for (index, point) in normalized_points
        .iter()
        .enumerate()
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
        let previous = shaped_points[shape_index - 1];
        let next = shaped_points[shape_index + 1];
        let inbound = (Vec2::new(point.x - previous.x, point.y - previous.y)).normalized();
        let outbound = (Vec2::new(next.x - point.x, next.y - point.y)).normalized();
        let tangent = (inbound + outbound).normalized_or(outbound);
        let direct_distance = geometry.direct_distance_m;
        let capture_radius = (direct_distance * 0.08).clamp(35.0, 95.0);
        let world_previous = denormalize(previous, request, geometry);
        let world_position = denormalize(*point, request, geometry);
        let world_next = denormalize(next, request, geometry);
        let authority = compute_waypoint_authority(
            request,
            world_previous,
            world_position,
            world_next,
            capture_radius,
        )
        .map_err(|error| PlanningRejection::new(error.rejection_code(), error.to_string()))?;
        let max_speed = authority.handoff_speed_cap_mps;
        let id = node_ids
            .get(index - 1)
            .cloned()
            .unwrap_or_else(|| format!("waypoint-{index}"));
        waypoints.push(TransferWaypointSpec {
            id,
            position_m: world_position,
            handoff_tangent_unit: Some(denormalize_vector(tangent, geometry)),
            capture_radius_m: capture_radius,
            max_cross_track_m: capture_radius,
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
    fn single_ridge_gets_one_monotone_waypoint() {
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
    fn ridge_above_loft_cap_is_rejected() {
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
    fn gravity_taxed_authority_rejects_short_handoff() {
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
