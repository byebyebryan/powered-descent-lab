//! Opt-in characterization of the existing direct waypoint-planning seam.
//!
//! This module deliberately owns no planner, controller, or core behavior. It
//! resolves a small deterministic fixture set, calls the ordinary planner,
//! runs the unchanged direct transfer controller, and writes a display-only
//! artifact that keeps those evidence lanes separate.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_control::{
    ControllerSpec, TelemetryValue, built_in_controller_spec, metric, run_controller_spec,
};
use pd_core::{
    CorridorClearance, EvaluationGoal, LandingPadSpec, MissionSpec, RoutePlan, RoutePlanningPolicy,
    RoutePlanningRequest, RouteTopology, RunContext, ScenarioSpec, SimConfig, TerrainDefinition,
    TransferRouteSpec, Vec2, VehicleInitialState, VehicleSpec, WorldSpec, build_endpoint_profile,
    endpoint_shaped_centerline, normalized_geometry,
};
use pd_plan::plan;
use pd_report::{site::ReportSite, waypoint_direct_characterization::write_report};
use serde::{Deserialize, Serialize};

pub const WAYPOINT_DIRECT_CHARACTERIZATION_ID: &str = "waypoint-direct-characterization";
pub const WAYPOINT_DIRECT_CHARACTERIZATION_SCHEMA_ID: &str = "waypoint_direct_characterization_v1";
pub const WAYPOINT_DIRECT_CHARACTERIZATION_SCHEMA_VERSION: u32 = 1;

const BASE_SCENARIO_PATH: &str = "fixtures/scenarios/flat_terminal_descent.json";
const CONTROLLER_NAME: &str = "transfer_pdg";
const ROUTE_RADIUS_M: f64 = 800.0;

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectCharacterizationPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
    pub report_path: PathBuf,
    pub preview_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectCharacterizationRun {
    pub artifact: WaypointDirectCharacterizationArtifact,
    pub paths: WaypointDirectCharacterizationPaths,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectCharacterizationArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub fixture_identity: String,
    pub planner_algorithm_id: String,
    pub planner_policy: RoutePlanningPolicy,
    pub controller_id: String,
    pub direct_definition: String,
    pub route_definition: String,
    pub analytical: AnalyticalEvidence,
    pub scope: ScopeEvidence,
    pub deterministic_repeat: String,
    pub cases: Vec<WaypointDirectCaseEvidence>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalyticalEvidence {
    pub status: String,
    pub source: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScopeEvidence {
    pub claims: Vec<String>,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectCaseEvidence {
    pub id: String,
    pub label: String,
    pub terrain_kind: String,
    pub route_angle_deg: f64,
    pub route_radius_m: f64,
    pub operational_handoffs: u32,
    pub source_pad: LandingPadSpec,
    pub target_pad: LandingPadSpec,
    pub terrain_points_m: Vec<Vec2>,
    pub direct_chord: ChordEvidence,
    pub planner: PlannerEvidence,
    pub controller: ControllerEvidence,
    pub analytical: AnalyticalEvidence,
    pub semantic_identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChordEvidence {
    pub clear: bool,
    pub minimum_clearance_m: f64,
    pub worst_residual_m: f64,
    pub worst_centerline_position_m: Vec2,
    pub worst_terrain_position_m: Vec2,
    pub endpoint_shaped_centerline_m: Vec<Vec2>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlannerEvidence {
    pub status: String,
    pub request_digest: Option<String>,
    pub topology: Option<RouteTopology>,
    pub plan_digest: Option<String>,
    pub direct_path_clear: Option<bool>,
    pub direct_path_clearance_m: Option<f64>,
    pub selected_centerline_m: Vec<Vec2>,
    pub rejection_code: Option<pd_core::PlanningRejectionCode>,
    pub rejection_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControllerEvidence {
    pub status: String,
    pub controller_id: String,
    pub physical_outcome: Option<String>,
    pub mission_outcome: Option<String>,
    pub end_reason: Option<String>,
    pub sim_time_s: Option<f64>,
    pub physics_steps: Option<u64>,
    pub observed_global_min_hull_clearance_m: Option<f64>,
    pub observed_en_route_min_hull_clearance_m: Option<f64>,
    pub observed_min_touchdown_clearance_m: Option<f64>,
    pub observed_apex_y_m: Option<f64>,
    pub observed_apex_above_target_m: Option<f64>,
    pub telemetry_apex_over_target_m: Option<f64>,
    pub telemetry_boost_apex_target_m: Option<f64>,
    pub trajectory: Vec<TrajectoryPoint>,
    pub clearance_observation: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryPoint {
    pub sim_time_s: f64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub min_hull_clearance_m: f64,
    pub touchdown_clearance_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerrainKind {
    Flat,
    Uphill,
    Downhill,
    NarrowRidge,
    BroadMesa,
}

impl TerrainKind {
    const fn id(self) -> &'static str {
        match self {
            Self::Flat => "continuous_flat_r00",
            Self::Uphill => "continuous_uphill_r+30",
            Self::Downhill => "continuous_downhill_r-30",
            Self::NarrowRidge => "bounded_narrow_ridge",
            Self::BroadMesa => "bounded_broad_mesa",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Flat => "continuous flat r00",
            Self::Uphill => "continuous uphill r+30",
            Self::Downhill => "continuous downhill r-30",
            Self::NarrowRidge => "bounded narrow-ridge control",
            Self::BroadMesa => "bounded broad-mesa control",
        }
    }

    const fn angle_deg(self) -> f64 {
        match self {
            Self::Flat | Self::NarrowRidge | Self::BroadMesa => 0.0,
            Self::Uphill => 30.0,
            Self::Downhill => -30.0,
        }
    }

    const fn terrain_profile(self) -> &'static str {
        match self {
            Self::Flat => "continuous_flat",
            Self::Uphill => "continuous_uphill",
            Self::Downhill => "continuous_downhill",
            Self::NarrowRidge => "bounded_narrow_ridge",
            Self::BroadMesa => "bounded_broad_mesa",
        }
    }
}

#[derive(Clone, Debug)]
struct FixtureCase {
    kind: TerrainKind,
    scenario: ScenarioSpec,
    request: RoutePlanningRequest,
    source_pad: LandingPadSpec,
    target_pad: LandingPadSpec,
    terrain_points_m: Vec<Vec2>,
}

/// Narrow projection of the existing characterization cases for the
/// controller-free direct-bridge research gate. This is derived from the same
/// scenario and case builder as the original artifact; it does not define new
/// fixture geometry.
#[derive(Clone, Debug)]
pub(crate) struct WaypointDirectPrimitiveInputProjection {
    pub(crate) fixture_identity: String,
    pub(crate) cases: Vec<WaypointDirectPrimitiveCaseInput>,
}

#[derive(Clone, Debug)]
pub(crate) struct WaypointDirectPrimitiveCaseInput {
    pub(crate) id: String,
    pub(crate) physics_hz: u32,
    pub(crate) gravity_mps2: f64,
    pub(crate) max_time_s: f64,
    pub(crate) vehicle: VehicleSpec,
    pub(crate) initial_attitude_rad: f64,
    pub(crate) initial_angular_rate_radps: f64,
    pub(crate) source_pad: LandingPadSpec,
    pub(crate) target_pad: LandingPadSpec,
    pub(crate) terrain_points_m: Vec<Vec2>,
    pub(crate) initial_position_m: Vec2,
    pub(crate) initial_velocity_mps: Vec2,
}

/// Rebuild the exact five existing inputs and expose only fields consumed by
/// the analytical-only primitive baseline.
pub(crate) fn project_waypoint_direct_primitive_inputs(
    repo_root: &Path,
) -> Result<WaypointDirectPrimitiveInputProjection> {
    let base = load_base_scenario(repo_root)?;
    let cases = build_fixture_cases(&base)?;
    let fixture_identity = characterization_fixture_identity(&base, &cases)?;
    let projected_cases = cases
        .into_iter()
        .map(|case| WaypointDirectPrimitiveCaseInput {
            id: case.kind.id().to_owned(),
            physics_hz: case.scenario.sim.physics_hz,
            gravity_mps2: case.scenario.world.gravity_mps2,
            max_time_s: case.scenario.sim.max_time_s,
            vehicle: case.scenario.vehicle,
            initial_attitude_rad: case.scenario.initial_state.attitude_rad,
            initial_angular_rate_radps: case.scenario.initial_state.angular_rate_radps,
            source_pad: case.source_pad,
            target_pad: case.target_pad,
            terrain_points_m: case.terrain_points_m,
            initial_position_m: case.scenario.initial_state.position_m,
            initial_velocity_mps: case.scenario.initial_state.velocity_mps,
        })
        .collect();
    Ok(WaypointDirectPrimitiveInputProjection {
        fixture_identity,
        cases: projected_cases,
    })
}

/// Run the opt-in direct-route characterization and write its self-contained
/// summary plus static report. The command's semantic artifact excludes all
/// controller wall/CPU timing fields.
pub fn run_waypoint_direct_characterization(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
) -> Result<WaypointDirectCharacterizationRun> {
    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create waypoint characterization output directory {}",
            output_dir.display()
        )
    })?;

    let base = load_base_scenario(repo_root)?;
    let cases = build_fixture_cases(&base)?;
    let policy = RoutePlanningPolicy::default();
    let controller_spec = built_in_controller_spec(CONTROLLER_NAME)
        .ok_or_else(|| anyhow!("built-in controller '{CONTROLLER_NAME}' is unavailable"))?;
    let controller_id = controller_spec.id().to_owned();
    let fixture_identity = characterization_fixture_identity(&base, &cases)?;
    let analytical = AnalyticalEvidence {
        status: "not_mapped".to_owned(),
        source: "conservative_ballistic_direct_bridge_v2".to_owned(),
        reason: "The frozen direct-bridge result uses a distinct fixture and policy; this checkpoint does not adapt that fixture or promote its analytical family.".to_owned(),
    };
    let scope = ScopeEvidence {
        claims: vec![
            "The five rows preserve ordinary pd_plan::plan() acceptance or typed rejection.".to_owned(),
            "Direct means zero operational handoffs; every route in this artifact has one leg.".to_owned(),
            "Controller values are observations from the unchanged transfer_pdg controller.".to_owned(),
        ],
        non_claims: vec![
            "An observed controller trajectory is not a planner path or a route certificate.".to_owned(),
            "Planner acceptance does not imply controller success, and controller success does not prove planner acceptance.".to_owned(),
            "This checkpoint does not choose or implement a future V2 leg-profile family.".to_owned(),
            "The existing conservative-ballistic analytical result is not mapped onto these terrain fixtures.".to_owned(),
        ],
    };
    let mut evidence_cases = Vec::with_capacity(cases.len());
    for case in &cases {
        evidence_cases.push(evaluate_case(case, &controller_spec, &analytical)?);
    }

    let mut artifact = WaypointDirectCharacterizationArtifact {
        schema_id: WAYPOINT_DIRECT_CHARACTERIZATION_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_CHARACTERIZATION_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_CHARACTERIZATION_ID.to_owned(),
        fixture_identity,
        planner_algorithm_id: pd_core::HEIGHTFIELD_VISIBILITY_ALGORITHM_ID.to_owned(),
        planner_policy: policy,
        controller_id,
        direct_definition: "zero operational handoffs".to_owned(),
        route_definition: "one route composed of one leg; future leg profiles/certificates remain distinct from TransferRouteSpec waypoints".to_owned(),
        analytical,
        scope,
        deterministic_repeat: "semantic_json_excluding_wall_timing".to_owned(),
        cases: evidence_cases,
        identity: String::new(),
    };
    artifact.identity = stable_digest(&artifact)?;

    let summary_path = output_dir.join("summary.json");
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&summary_path, &summary_bytes).with_context(|| {
        format!(
            "failed to write waypoint characterization summary {}",
            summary_path.display()
        )
    })?;
    let reloaded: WaypointDirectCharacterizationArtifact =
        serde_json::from_slice(&summary_bytes)
            .context("failed to reload waypoint characterization summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes {
        bail!("waypoint characterization summary is not byte-stable after reload");
    }

    let site = ReportSite::new(repo_root);
    let repo_outputs = repo_root.join("outputs");
    let (report_path, update_site) = if is_repo_outputs_path(&repo_outputs, &output_dir) {
        let path = site.default_output_for_bundle(&output_dir).ok_or_else(|| {
            anyhow!("waypoint characterization output must be under repository outputs")
        })?;
        (path, true)
    } else {
        (output_dir.join("report/index.html"), false)
    };
    let preview_path = report_path
        .parent()
        .expect("waypoint characterization report has a parent")
        .join("preview.svg");
    let data = serde_json::to_value(&reloaded)?;
    write_report(&report_path, &preview_path, &data)?;
    if update_site {
        site.update_indexes_for_file(&report_path)?;
    }

    Ok(WaypointDirectCharacterizationRun {
        artifact: reloaded,
        paths: WaypointDirectCharacterizationPaths {
            output_dir,
            summary_path,
            report_path,
            preview_path,
        },
    })
}

fn evaluate_case(
    case: &FixtureCase,
    controller_spec: &ControllerSpec,
    analytical: &AnalyticalEvidence,
) -> Result<WaypointDirectCaseEvidence> {
    let direct_chord = direct_chord_clearance(&case.request)?;
    let planner = match plan(&case.request) {
        Ok(route_plan) => planner_success(&route_plan),
        Err(rejection) => PlannerEvidence {
            status: "rejected".to_owned(),
            request_digest: Some(stable_digest(&case.request)?),
            topology: None,
            plan_digest: None,
            direct_path_clear: None,
            direct_path_clearance_m: None,
            selected_centerline_m: Vec::new(),
            rejection_code: Some(rejection.code),
            rejection_message: Some(rejection.message),
        },
    };
    let controller = run_direct_controller(&case.scenario, controller_spec)?;
    let semantic_identity = stable_digest(&(
        case.kind.id(),
        case.route_angle(),
        &case.terrain_points_m,
        &direct_chord,
        &planner,
        &controller,
        analytical,
    ))?;
    Ok(WaypointDirectCaseEvidence {
        id: case.kind.id().to_owned(),
        label: case.kind.label().to_owned(),
        terrain_kind: case.kind.terrain_profile().to_owned(),
        route_angle_deg: case.route_angle(),
        route_radius_m: ROUTE_RADIUS_M,
        operational_handoffs: 0,
        source_pad: case.source_pad.clone(),
        target_pad: case.target_pad.clone(),
        terrain_points_m: case.terrain_points_m.clone(),
        direct_chord,
        planner,
        controller,
        analytical: analytical.clone(),
        semantic_identity,
    })
}

fn planner_success(route_plan: &RoutePlan) -> PlannerEvidence {
    PlannerEvidence {
        status: "accepted".to_owned(),
        request_digest: Some(route_plan.request_digest.clone()),
        topology: Some(route_plan.topology),
        plan_digest: Some(route_plan.plan_digest.clone()),
        direct_path_clear: Some(route_plan.diagnostics.direct_path_clear),
        direct_path_clearance_m: route_plan
            .diagnostics
            .direct_path_clearance
            .as_ref()
            .map(|clearance| clearance.minimum_clearance_m),
        selected_centerline_m: route_plan.diagnostics.selected_centerline_m.clone(),
        rejection_code: None,
        rejection_message: None,
    }
}

pub(crate) fn run_direct_controller(
    scenario: &ScenarioSpec,
    controller_spec: &ControllerSpec,
) -> Result<ControllerEvidence> {
    let controller_id = controller_spec.id().to_owned();
    let context = match RunContext::from_scenario(scenario) {
        Ok(context) => context,
        Err(error) => {
            return Ok(ControllerEvidence {
                status: "unsupported".to_owned(),
                controller_id,
                physical_outcome: None,
                mission_outcome: None,
                end_reason: None,
                sim_time_s: None,
                physics_steps: None,
                observed_global_min_hull_clearance_m: None,
                observed_en_route_min_hull_clearance_m: None,
                observed_min_touchdown_clearance_m: None,
                observed_apex_y_m: None,
                observed_apex_above_target_m: None,
                telemetry_apex_over_target_m: None,
                telemetry_boost_apex_target_m: None,
                trajectory: Vec::new(),
                clearance_observation: "sampled_physics_observations".to_owned(),
                error: Some(error),
            });
        }
    };
    let artifacts = match run_controller_spec(&context, controller_spec) {
        Ok(artifacts) => artifacts,
        Err(error) => {
            return Ok(ControllerEvidence {
                status: "error".to_owned(),
                controller_id,
                physical_outcome: None,
                mission_outcome: None,
                end_reason: None,
                sim_time_s: None,
                physics_steps: None,
                observed_global_min_hull_clearance_m: None,
                observed_en_route_min_hull_clearance_m: None,
                observed_min_touchdown_clearance_m: None,
                observed_apex_y_m: None,
                observed_apex_above_target_m: None,
                telemetry_apex_over_target_m: None,
                telemetry_boost_apex_target_m: None,
                trajectory: Vec::new(),
                clearance_observation: "sampled_physics_observations".to_owned(),
                error: Some(error.to_string()),
            });
        }
    };
    let samples = &artifacts.run.samples;
    let trajectory = samples
        .iter()
        .map(|sample| TrajectoryPoint {
            sim_time_s: sample.sim_time_s,
            position_m: sample.observation.position_m,
            velocity_mps: sample.observation.velocity_mps,
            min_hull_clearance_m: sample.observation.min_hull_clearance_m,
            touchdown_clearance_m: sample.observation.touchdown_clearance_m,
        })
        .collect::<Vec<_>>();
    let observed_apex_y_m = samples
        .iter()
        .map(|sample| sample.observation.position_m.y)
        .max_by(f64::total_cmp);
    let target_surface_y_m = context.target_pad.surface_y_m;
    let observed_apex_above_target_m = observed_apex_y_m.map(|height| height - target_surface_y_m);
    let observed_global_min_hull_clearance_m = samples
        .iter()
        .map(|sample| sample.observation.min_hull_clearance_m)
        .min_by(f64::total_cmp);
    let observed_min_touchdown_clearance_m = samples
        .iter()
        .map(|sample| sample.observation.touchdown_clearance_m)
        .min_by(f64::total_cmp);
    let observed_en_route_min_hull_clearance_m =
        sampled_en_route_min_hull_clearance(scenario, samples);
    Ok(ControllerEvidence {
        status: "completed".to_owned(),
        controller_id,
        physical_outcome: Some(label(&artifacts.run.manifest.physical_outcome)),
        mission_outcome: Some(label(&artifacts.run.manifest.mission_outcome)),
        end_reason: Some(label(&artifacts.run.manifest.end_reason)),
        sim_time_s: Some(artifacts.run.manifest.sim_time_s),
        physics_steps: Some(artifacts.run.manifest.physics_steps),
        observed_global_min_hull_clearance_m,
        observed_en_route_min_hull_clearance_m,
        observed_min_touchdown_clearance_m,
        observed_apex_y_m,
        observed_apex_above_target_m,
        telemetry_apex_over_target_m: max_metric_float(
            &artifacts.controller_updates,
            metric::TRANSFER_APEX_OVER_TARGET_M,
        ),
        telemetry_boost_apex_target_m: max_metric_float(
            &artifacts.controller_updates,
            metric::TRANSFER_BOOST_APEX_TARGET_M,
        ),
        trajectory,
        clearance_observation:
            "sampled_physics_observations; global includes endpoint contact; en_route uses the full-envelope progress window".to_owned(),
        error: None,
    })
}

fn max_metric_float(updates: &[pd_control::ControllerUpdateRecord], key: &str) -> Option<f64> {
    updates
        .iter()
        .filter_map(|update| match update.frame.metrics.get(key) {
            Some(TelemetryValue::Float(value)) if value.is_finite() => Some(*value),
            _ => None,
        })
        .max_by(f64::total_cmp)
}

fn direct_chord_clearance(request: &RoutePlanningRequest) -> Result<ChordEvidence> {
    let geometry = normalized_geometry(request).map_err(|error| anyhow!(error.to_string()))?;
    let (profile, _) = build_endpoint_profile(request, geometry.direct_horizontal_span_m)
        .map_err(|error| anyhow!(error.to_string()))?;
    let points = endpoint_shaped_centerline(request, &geometry, &profile, &[])
        .map_err(|error| anyhow!(error.to_string()))?;
    let source = request
        .source_pad()
        .ok_or_else(|| anyhow!("direct chord source pad is missing"))?;
    let mut worst: Option<CorridorClearance> = None;
    for pair in points.windows(2) {
        for (left, right) in profile.breakpoints_between(pair[0].x, pair[1].x) {
            let t0 = (left - pair[0].x) / (pair[1].x - pair[0].x);
            let t1 = (right - pair[0].x) / (pair[1].x - pair[0].x);
            let start = interpolate(
                *pair.first().expect("pair has start"),
                *pair.last().expect("pair has end"),
                t0,
            );
            let end = interpolate(
                *pair.first().expect("pair has start"),
                *pair.last().expect("pair has end"),
                t1,
            );
            let clearance = request.world.terrain.exact_corridor_clearance(
                denormalize(start, source.center_x_m, geometry.horizontal_sign),
                denormalize(end, source.center_x_m, geometry.horizontal_sign),
                profile.envelope_at(left),
                profile.envelope_at(right),
            )?;
            if worst.as_ref().is_none_or(|current| {
                clearance.worst_residual.residual_m > current.worst_residual.residual_m
            }) {
                worst = Some(clearance);
            }
        }
    }
    let worst = worst.ok_or_else(|| anyhow!("direct chord produced no profile intervals"))?;
    let endpoint_shaped_centerline_m = points
        .iter()
        .copied()
        .map(|point| denormalize(point, source.center_x_m, geometry.horizontal_sign))
        .collect();
    Ok(chord_from_clearance(&worst, endpoint_shaped_centerline_m))
}

fn chord_from_clearance(
    clearance: &CorridorClearance,
    endpoint_shaped_centerline_m: Vec<Vec2>,
) -> ChordEvidence {
    ChordEvidence {
        clear: clearance.clear,
        minimum_clearance_m: clearance.minimum_clearance_m,
        worst_residual_m: clearance.worst_residual.residual_m,
        worst_centerline_position_m: clearance.worst_residual.centerline_position_m,
        worst_terrain_position_m: clearance.worst_residual.terrain_position_m,
        endpoint_shaped_centerline_m,
    }
}

fn sampled_en_route_min_hull_clearance(
    scenario: &ScenarioSpec,
    samples: &[pd_core::SampleRecord],
) -> Option<f64> {
    let route = scenario.mission.transfer_route.as_ref()?;
    let request = RoutePlanningRequest {
        world: scenario.world.clone(),
        vehicle: scenario.vehicle.clone(),
        initial_state: scenario.initial_state.clone(),
        source_pad_id: route.source_pad_id.clone(),
        target_pad_id: route.target_pad_id.clone(),
        policy: RoutePlanningPolicy::default(),
    };
    let geometry = normalized_geometry(&request).ok()?;
    let (profile, _) = build_endpoint_profile(&request, geometry.direct_horizontal_span_m).ok()?;
    let source_pad = request.source_pad()?;
    let sign = f64::from(geometry.horizontal_sign);
    let mut minimum = f64::INFINITY;
    for sample in samples {
        let progress = (sample.observation.position_m.x - source_pad.center_x_m) * sign;
        if progress >= profile.source_transition_end_m - 1.0e-9
            && progress <= profile.target_transition_start_m + 1.0e-9
        {
            minimum = minimum.min(sample.observation.min_hull_clearance_m);
        }
    }
    minimum.is_finite().then_some(minimum)
}

fn build_fixture_cases(base: &ScenarioSpec) -> Result<Vec<FixtureCase>> {
    [
        TerrainKind::Flat,
        TerrainKind::Uphill,
        TerrainKind::Downhill,
        TerrainKind::NarrowRidge,
        TerrainKind::BroadMesa,
    ]
    .into_iter()
    .map(|kind| build_fixture_case(base, kind))
    .collect()
}

/// Return the continuous flat scenario produced by this characterization's
/// existing fixture builder. Downstream evaluator experiments may clone it
/// and change only the scenario id and terrain when that is their contract.
pub(crate) fn continuous_flat_controller_scenario(repo_root: &Path) -> Result<ScenarioSpec> {
    let base = load_base_scenario(repo_root)?;
    build_fixture_cases(&base)?
        .into_iter()
        .find(|case| case.kind == TerrainKind::Flat)
        .map(|case| case.scenario)
        .ok_or_else(|| anyhow!("characterization fixture builder omitted its continuous flat case"))
}

pub(crate) fn controller_scenario_for_case(
    repo_root: &Path,
    case_id: &str,
) -> Result<ScenarioSpec> {
    let base = load_base_scenario(repo_root)?;
    build_fixture_cases(&base)?
        .into_iter()
        .find(|case| case.kind.id() == case_id)
        .map(|case| case.scenario)
        .ok_or_else(|| anyhow!("characterization fixture omitted case {case_id}"))
}

fn build_fixture_case(base: &ScenarioSpec, kind: TerrainKind) -> Result<FixtureCase> {
    let angle_deg = kind.angle_deg();
    let angle_rad = angle_deg.to_radians();
    let target_base = base
        .world
        .landing_pad("pad_main")
        .cloned()
        .ok_or_else(|| anyhow!("base characterization fixture is missing pad_main"))?;
    let target_pad = LandingPadSpec {
        id: target_base.id,
        center_x_m: 0.0,
        surface_y_m: 0.0,
        width_m: target_base.width_m,
    };
    let source_pad = LandingPadSpec {
        id: "pad_source".to_owned(),
        center_x_m: -(ROUTE_RADIUS_M * angle_rad.cos()),
        surface_y_m: -(ROUTE_RADIUS_M * angle_rad.sin()),
        width_m: target_pad.width_m,
    };
    let terrain_points_m = match kind {
        TerrainKind::Flat | TerrainKind::Uphill | TerrainKind::Downhill => {
            super::transfer_route_terrain_points(&source_pad, &target_pad)?
        }
        TerrainKind::NarrowRidge | TerrainKind::BroadMesa => {
            bounded_obstacle_terrain(&source_pad, &target_pad, kind)?
        }
    };
    let route = TransferRouteSpec {
        source_pad_id: source_pad.id.clone(),
        target_pad_id: target_pad.id.clone(),
        route_angle_deg: angle_deg,
        route_radius_m: ROUTE_RADIUS_M,
        waypoints: Vec::new(),
    };
    let mut scenario = base.clone();
    scenario.id = format!("{WAYPOINT_DIRECT_CHARACTERIZATION_ID}_{}", kind.id());
    scenario.name = format!("Waypoint direct characterization: {}", kind.label());
    scenario.description = "Opt-in deterministic direct-route characterization fixture.".to_owned();
    scenario.seed = 7;
    scenario.sim = SimConfig {
        physics_hz: base.sim.physics_hz,
        controller_hz: base.sim.controller_hz,
        max_time_s: 90.0,
        sample_hz: base.sim.sample_hz,
    };
    scenario.world = WorldSpec {
        gravity_mps2: base.world.gravity_mps2,
        terrain: TerrainDefinition::Heightfield {
            points_m: terrain_points_m.clone(),
        },
        landing_pads: vec![source_pad.clone(), target_pad.clone()],
    };
    scenario.initial_state = VehicleInitialState {
        position_m: Vec2::new(
            source_pad.center_x_m,
            source_pad.surface_y_m + scenario.vehicle.geometry.touchdown_base_offset_m,
        ),
        velocity_mps: Vec2::new(0.0, 0.0),
        attitude_rad: 0.0,
        angular_rate_radps: 0.0,
    };
    scenario.mission = MissionSpec {
        transfer_route: Some(route),
        goal: EvaluationGoal::LandingOnPad {
            target_pad_id: target_pad.id.clone(),
        },
    };
    scenario.metadata = BTreeMap::from([
        (
            "characterization".to_owned(),
            WAYPOINT_DIRECT_CHARACTERIZATION_ID.to_owned(),
        ),
        ("terrain_kind".to_owned(), kind.terrain_profile().to_owned()),
        ("route_mode".to_owned(), "direct".to_owned()),
        ("controller".to_owned(), CONTROLLER_NAME.to_owned()),
    ]);
    scenario.tags = vec![
        "waypoint_planning".to_owned(),
        "direct_characterization".to_owned(),
        kind.terrain_profile().to_owned(),
    ];
    scenario.validate().map_err(anyhow::Error::msg)?;
    let request = RoutePlanningRequest {
        world: scenario.world.clone(),
        vehicle: scenario.vehicle.clone(),
        initial_state: scenario.initial_state.clone(),
        source_pad_id: source_pad.id.clone(),
        target_pad_id: target_pad.id.clone(),
        policy: RoutePlanningPolicy::default(),
    };
    request
        .validate()
        .map_err(|error| anyhow!(error.to_string()))?;
    Ok(FixtureCase {
        kind,
        scenario,
        request,
        source_pad,
        target_pad,
        terrain_points_m,
    })
}

fn bounded_obstacle_terrain(
    source: &LandingPadSpec,
    target: &LandingPadSpec,
    kind: TerrainKind,
) -> Result<Vec<Vec2>> {
    let source_left = source.center_x_m - source.half_width_m();
    let source_right = source.center_x_m + source.half_width_m();
    let target_left = target.center_x_m - target.half_width_m();
    let target_right = target.center_x_m + target.half_width_m();
    if source_right >= target_left {
        bail!("characterization pads overlap");
    }
    let baseline = |x: f64| {
        source.surface_y_m
            + (target.surface_y_m - source.surface_y_m)
                * ((x - source.center_x_m) / (target.center_x_m - source.center_x_m))
    };
    let midpoint = (source_right + target_left) * 0.5;
    let (left, peak_left, peak_right, right, offset) = match kind {
        TerrainKind::NarrowRidge => (
            midpoint - 24.0,
            midpoint - 7.0,
            midpoint + 7.0,
            midpoint + 24.0,
            180.0,
        ),
        TerrainKind::BroadMesa => (
            midpoint - 150.0,
            midpoint - 100.0,
            midpoint + 100.0,
            midpoint + 150.0,
            180.0,
        ),
        _ => unreachable!("bounded obstacle helper only accepts obstacle controls"),
    };
    let margin = (target.center_x_m - source.center_x_m).abs() * 0.15;
    let outer_margin = margin.max(160.0);
    Ok(vec![
        Vec2::new(source.center_x_m - outer_margin, source.surface_y_m),
        Vec2::new(source_left, source.surface_y_m),
        Vec2::new(source_right, source.surface_y_m),
        Vec2::new(left, baseline(left)),
        Vec2::new(peak_left, baseline(peak_left) + offset),
        Vec2::new(peak_right, baseline(peak_right) + offset),
        Vec2::new(right, baseline(right)),
        Vec2::new(target_left, target.surface_y_m),
        Vec2::new(target_right, target.surface_y_m),
        Vec2::new(target.center_x_m + outer_margin, target.surface_y_m),
    ])
}

fn characterization_fixture_identity(base: &ScenarioSpec, cases: &[FixtureCase]) -> Result<String> {
    stable_digest(&(
        BASE_SCENARIO_PATH,
        base,
        cases
            .iter()
            .map(|case| {
                (
                    case.kind.id(),
                    request_route_angle(&case.request),
                    case.terrain_points_m.clone(),
                )
            })
            .collect::<Vec<_>>(),
    ))
}

fn load_base_scenario(repo_root: &Path) -> Result<ScenarioSpec> {
    let path = repo_root.join(BASE_SCENARIO_PATH);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read characterization fixture {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| {
        format!(
            "failed to parse characterization fixture {}",
            path.display()
        )
    })
}

fn stable_digest<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("fnv1a64:{hash:016x}"))
}

fn label<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn interpolate(start: Vec2, end: Vec2, t: f64) -> Vec2 {
    Vec2::new(
        start.x + ((end.x - start.x) * t),
        start.y + ((end.y - start.y) * t),
    )
}

fn denormalize(point: Vec2, source_x_m: f64, horizontal_sign: i8) -> Vec2 {
    Vec2::new(source_x_m + (point.x * f64::from(horizontal_sign)), point.y)
}

fn resolve_output_dir(repo_root: &Path, requested: Option<&Path>) -> PathBuf {
    requested
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                repo_root.join(path)
            }
        })
        .unwrap_or_else(|| {
            repo_root
                .join("outputs/setups")
                .join(WAYPOINT_DIRECT_CHARACTERIZATION_ID)
        })
}

fn is_repo_outputs_path(repo_outputs: &Path, output_dir: &Path) -> bool {
    let Ok(repo_outputs) = fs::canonicalize(repo_outputs) else {
        return false;
    };
    let Ok(output_dir) = fs::canonicalize(output_dir) else {
        return false;
    };
    output_dir.starts_with(repo_outputs)
}

impl FixtureCase {
    fn route_angle(&self) -> f64 {
        self.kind.angle_deg()
    }
}

fn request_route_angle(request: &RoutePlanningRequest) -> f64 {
    let source = request.source_pad().expect("fixture source pad exists");
    let target = request.target_pad().expect("fixture target pad exists");
    (target.surface_y_m - source.surface_y_m)
        .atan2((target.center_x_m - source.center_x_m).abs())
        .to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_order_and_ids_are_frozen() {
        let base: ScenarioSpec = serde_json::from_str(include_str!(
            "../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .expect("base fixture parses");
        let cases = build_fixture_cases(&base).expect("fixture cases build");
        assert_eq!(
            cases.iter().map(|case| case.kind.id()).collect::<Vec<_>>(),
            vec![
                "continuous_flat_r00",
                "continuous_uphill_r+30",
                "continuous_downhill_r-30",
                "bounded_narrow_ridge",
                "bounded_broad_mesa",
            ]
        );
        assert!(cases.iter().all(|case| case.request.validate().is_ok()));
    }

    #[test]
    fn continuous_rows_show_typed_planner_rejection_and_controller_evidence() {
        let base: ScenarioSpec = serde_json::from_str(include_str!(
            "../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .expect("base fixture parses");
        let cases = build_fixture_cases(&base).expect("fixture cases build");
        for case in cases.iter().take(3) {
            let chord = direct_chord_clearance(&case.request).expect("chord query");
            assert!(
                !chord.clear,
                "{} direct chord unexpectedly clear",
                case.kind.id()
            );
            let rejection = plan(&case.request).expect_err("continuous row should reject");
            assert_eq!(
                rejection.code,
                pd_core::PlanningRejectionCode::NoRouteWithinPolicy,
                "{} rejection changed",
                case.kind.id()
            );
        }
        let flat = &cases[0];
        let controller = run_direct_controller(
            &flat.scenario,
            &built_in_controller_spec(CONTROLLER_NAME).expect("transfer controller"),
        )
        .expect("flat controller run");
        assert_eq!(controller.mission_outcome.as_deref(), Some("success"));
        assert!(
            controller
                .observed_en_route_min_hull_clearance_m
                .is_some_and(|clearance| clearance > 0.0),
            "flat en-route sampled clearance should remain positive: {:?}",
            controller.observed_en_route_min_hull_clearance_m
        );
        assert!(
            flat.request
                .source_pad()
                .is_some_and(|source| source.center_x_m < 0.0)
        );
    }

    #[test]
    fn direct_chord_evidence_retains_denormalized_endpoint_shaped_centerline() {
        let base: ScenarioSpec = serde_json::from_str(include_str!(
            "../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .expect("base fixture parses");
        let case = build_fixture_case(&base, TerrainKind::Uphill).expect("uphill case");
        let chord = direct_chord_clearance(&case.request).expect("chord query");
        assert!(chord.endpoint_shaped_centerline_m.len() >= 2);
        assert_eq!(
            chord.endpoint_shaped_centerline_m.first().copied(),
            Some(Vec2::new(
                case.source_pad.center_x_m,
                case.source_pad.surface_y_m
                    + case.scenario.vehicle.geometry.touchdown_base_offset_m
            ))
        );
        assert_eq!(
            chord.endpoint_shaped_centerline_m.last().copied(),
            Some(Vec2::new(
                case.target_pad.center_x_m,
                case.target_pad.surface_y_m
                    + case.scenario.vehicle.geometry.touchdown_base_offset_m
            ))
        );
    }

    #[test]
    fn semantic_controller_evidence_excludes_wall_timing() {
        let base: ScenarioSpec = serde_json::from_str(include_str!(
            "../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .expect("base fixture parses");
        let case = build_fixture_case(&base, TerrainKind::Flat).expect("flat case");
        let spec = built_in_controller_spec(CONTROLLER_NAME).expect("transfer controller");
        let first = run_direct_controller(&case.scenario, &spec).expect("controller run");
        let second = run_direct_controller(&case.scenario, &spec).expect("controller run");
        assert_eq!(first, second);
        assert!(
            first
                .clearance_observation
                .starts_with("sampled_physics_observations;")
        );
    }
}
