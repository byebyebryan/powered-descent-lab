//! Evaluator-only full-plant/controller shadow for the V2 ridge canary.
//!
//! This module is intentionally downstream of the analytical certificate.  It
//! materializes the frozen V2 setup into the existing simulator and built-in
//! controllers, but does not alter either the production planner or control
//! implementation.  The analytical powered bridge is not replayed: the one
//! waypoint is adapted to the existing runtime `TransferRouteSpec` contract.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_control::{
    ControlledRunArtifacts, ControllerSpec, TelemetryValue, built_in_controller_spec, marker,
    metric, run_controller_spec,
};
use pd_core::{
    EndReason, EvaluationGoal, LandingPadSpec, MissionSpec, RoutePlanningPolicy,
    RoutePlanningRequest, RouteValidation, ScenarioSpec, SimConfig, TerrainDefinition,
    TransferRouteSpec, TransferWaypointSpec, Vec2, VehicleGeometry, VehicleInitialState,
    VehicleSpec, WorldSpec, build_endpoint_profile, compute_waypoint_authority,
    endpoint_shaped_centerline, normalized_geometry, validate_route,
};
use pd_plan::conservative_ballistic_bridge::{
    AnalyticalBridgeV2, DirectBridgeCandidateV2, DirectBridgeFixtureV2, DirectBridgeProbeV2,
    DirectBridgeReportV2, VirtualBallisticArcV2, WaypointCandidateV2, build_report_artifact_v2,
};
use pd_report::site::ReportSite;
use serde::{Deserialize, Serialize};

pub const CONTROLLER_SHADOW_SCHEMA_ID: &str = "conservative-ballistic-controller-shadow-v1";
pub const CONTROLLER_SHADOW_SCHEMA_VERSION: u32 = 1;
pub const CONTROLLER_SHADOW_SETUP_ID: &str = "conservative-ballistic-controller-shadow-v1";
pub const CONTROLLER_SHADOW_MAX_TIME_S: f64 = 180.0;
pub const CONTROLLER_SHADOW_PHYSICS_HZ: u32 = 120;
pub const CONTROLLER_SHADOW_CONTROLLER_HZ: u32 = 60;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowLaneClass {
    TargetLanding,
    TerrainCrash,
    Inconclusive,
    RouteMappingInvalid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainContactEvidence {
    pub event_physics_step: Option<u64>,
    pub event_time_s: Option<f64>,
    /// Vehicle center at the simulator crash sample.  The attributed contact
    /// point below is a reconstructed hull/touchdown point, not this center.
    pub vehicle_center_m: Option<Vec2>,
    pub center_terrain_height_m: Option<f64>,
    pub touchdown_clearance_m: Option<f64>,
    pub hull_clearance_m: Option<f64>,
    pub contact_point_m: Option<Vec2>,
    pub contact_kind: Option<TerrainContactKind>,
    /// Clearance residual at the worst reconstructed contact point.  Negative
    /// values indicate terrain penetration, matching simulator clearance
    /// semantics (point y minus sampled terrain height).
    pub contact_residual_m: Option<f64>,
    pub contact_terrain_height_m: Option<f64>,
    pub reconstructed_touchdown_clearance_m: Option<f64>,
    pub reconstructed_hull_clearance_m: Option<f64>,
    pub touchdown_clearance_error_m: Option<f64>,
    pub hull_clearance_error_m: Option<f64>,
    pub reconstruction_valid: Option<bool>,
    pub within_derived_mesa_span: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainContactKind {
    HullVertex,
    TouchdownPoint,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointContractEvidence {
    pub marker_count: usize,
    pub contract_pass: bool,
    pub capture_time_s: Option<f64>,
    pub capture_position_m: Option<Vec2>,
    pub closest_distance_m: Option<f64>,
    pub cross_track_m: Option<f64>,
    pub outbound_progress_mps: Option<f64>,
    pub outbound_cross_speed_mps: Option<f64>,
    pub speed_mps: Option<f64>,
    pub resolution_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShadowLaneSummary {
    pub id: String,
    pub terrain_kind: String,
    pub controller_id: String,
    pub scenario_id: String,
    pub class: ShadowLaneClass,
    pub causal_reason: String,
    pub end_reason: Option<EndReason>,
    pub physical_outcome: Option<pd_core::PhysicalOutcome>,
    pub mission_outcome: Option<pd_core::MissionOutcome>,
    pub sim_time_s: Option<f64>,
    pub physics_steps: Option<u64>,
    pub fuel_remaining_kg: Option<f64>,
    pub min_touchdown_clearance_m: Option<f64>,
    pub min_hull_clearance_m: Option<f64>,
    pub terrain_contact: TerrainContactEvidence,
    pub waypoint_contract: Option<WaypointContractEvidence>,
    pub preflight: ControllerPreflightEvidence,
    pub scenario: Option<ScenarioSpec>,
    pub controller: Option<ControllerSpec>,
    pub run: Option<ControlledRunArtifacts>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControllerPreflightEvidence {
    pub route: Option<TransferRouteSpec>,
    pub first_phase: Option<String>,
    pub first_target_attitude_rad: Option<f64>,
    pub first_controller_update_physics_step: Option<u64>,
    pub survived_source_launch: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectRouteEvidence {
    pub route: TransferRouteSpec,
    pub normalized_direct_distance_m: f64,
    pub normalized_route_angle_deg: f64,
    pub structural_validation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteAdapterEvidence {
    pub source: String,
    pub analytical_waypoint_position_m: Vec2,
    pub capture_radius_m: f64,
    pub max_cross_track_m: f64,
    pub route: Option<TransferRouteSpec>,
    pub validation: Option<RouteValidation>,
    pub error: Option<String>,
    pub capture_radius_derivation: String,
    pub authority_derivation: String,
    pub bridge_mapping_note: String,
}

/// Compact, controller-neutral analytical paths copied from the selected V2
/// certificate.  These are visual context only: they are not replayed as
/// controller commands or treated as simulator observations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalyticalOverlayPath {
    pub id: String,
    pub points_m: Vec<Vec2>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalyticalShadowOverlay {
    pub nominal_direct: AnalyticalOverlayPath,
    pub waypoint_source_bridge: AnalyticalOverlayPath,
    pub waypoint_source_leg: AnalyticalOverlayPath,
    pub waypoint_intermediate_bridge: AnalyticalOverlayPath,
    pub waypoint_target_leg: AnalyticalOverlayPath,
    pub waypoint_terminal_bridge: AnalyticalOverlayPath,
    pub waypoint_position_m: Vec2,
    pub nominal_apex_position_m: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControllerShadowRun {
    pub schema_id: String,
    pub schema_version: u32,
    pub setup_id: String,
    pub analytical_report_identity: String,
    pub analytical_canary_identity: String,
    pub mesa_identity: String,
    pub waypoint_candidate_identity: String,
    pub direct_route: DirectRouteEvidence,
    pub route_adapter: RouteAdapterEvidence,
    pub analytical_overlay: AnalyticalShadowOverlay,
    pub lanes: Vec<ShadowLaneSummary>,
    pub deterministic_repeat: bool,
    pub semantic_identity: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ControllerShadowPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
    pub report_path: PathBuf,
    pub preview_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ControllerShadowReportRun {
    pub report: ControllerShadowRun,
    pub paths: ControllerShadowPaths,
}

/// Build and execute the frozen three-lane shadow once, followed by a
/// semantic deterministic repeat.  This is an evaluator/report seam only.
pub fn run_controller_shadow(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
) -> Result<ControllerShadowReportRun> {
    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create controller shadow output directory {}",
            output_dir.display()
        )
    })?;

    let report = build_report_artifact_v2();
    let first = execute_shadow(&report)?;
    let repeat = execute_shadow(&report)?;
    let deterministic_repeat = semantic_digest(&first)? == semantic_digest(&repeat)?;
    let mut run = first;
    run.deterministic_repeat = deterministic_repeat;
    if !deterministic_repeat {
        bail!("controller shadow semantic repeat diverged");
    }
    run.semantic_identity = semantic_digest(&run)?;

    write_shadow_artifacts(&output_dir, &run)?;
    let repo_outputs = repo_root.join("outputs");
    let (report_path, update_site) = if is_repo_outputs_path(&repo_outputs, &output_dir) {
        let site = ReportSite::new(repo_root);
        let report_path = site
            .default_output_for_bundle(&output_dir)
            .ok_or_else(|| anyhow!("controller shadow output must be under repository outputs"))?;
        (report_path, Some(site))
    } else {
        (output_dir.join("report/index.html"), None)
    };
    let preview_path = report_path
        .parent()
        .expect("controller shadow report path has a parent")
        .join("preview.svg");
    let report_value = serde_json::to_value(&run)?;
    pd_report::controller_shadow::write_controller_shadow_report(&report_path, &report_value)?;
    pd_report::controller_shadow::write_controller_shadow_preview_svg(
        &preview_path,
        &report_value,
    )?;
    if let Some(site) = update_site {
        site.update_indexes_for_file(&report_path)?;
    }

    Ok(ControllerShadowReportRun {
        report: run,
        paths: ControllerShadowPaths {
            summary_path: output_dir.join("summary.json"),
            output_dir,
            report_path,
            preview_path,
        },
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

/// Reload a persisted shadow summary and verify its semantic identity. Wall
/// and CPU timing are deliberately excluded from the identity, so a reload
/// remains valid across machines and repeated runs.
pub fn load_controller_shadow(path: &Path) -> Result<ControllerShadowRun> {
    let report: ControllerShadowRun = serde_json::from_slice(
        &fs::read(path)
            .with_context(|| format!("failed to read controller shadow {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse controller shadow {}", path.display()))?;
    if report.schema_id != CONTROLLER_SHADOW_SCHEMA_ID
        || report.schema_version != CONTROLLER_SHADOW_SCHEMA_VERSION
    {
        bail!("unexpected controller shadow schema");
    }
    if report.semantic_identity != semantic_digest(&report)? {
        bail!("controller shadow semantic identity does not match contents");
    }
    Ok(report)
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
                .join("outputs/eval")
                .join(CONTROLLER_SHADOW_SETUP_ID)
        })
}

fn write_shadow_artifacts(output_dir: &Path, run: &ControllerShadowRun) -> Result<()> {
    let summary_path = output_dir.join("summary.json");
    let bytes = serde_json::to_vec_pretty(run)?;
    fs::write(&summary_path, &bytes).with_context(|| {
        format!(
            "failed to write controller shadow summary {}",
            summary_path.display()
        )
    })?;
    load_controller_shadow(&summary_path)?;
    for lane in &run.lanes {
        let Some(run_artifacts) = lane.run.as_ref() else {
            continue;
        };
        let Some(scenario) = lane.scenario.as_ref() else {
            continue;
        };
        let Some(controller) = lane.controller.as_ref() else {
            continue;
        };
        let lane_dir = output_dir.join("runs").join(&lane.id);
        fs::create_dir_all(&lane_dir)?;
        fs::write(
            lane_dir.join("scenario.json"),
            serde_json::to_vec_pretty(scenario)?,
        )?;
        fs::write(
            lane_dir.join("controller.json"),
            serde_json::to_vec_pretty(controller)?,
        )?;
        fs::write(
            lane_dir.join("run.json"),
            serde_json::to_vec_pretty(run_artifacts)?,
        )?;
    }
    Ok(())
}

fn execute_shadow(report: &DirectBridgeReportV2) -> Result<ControllerShadowRun> {
    let case = report
        .fixture
        .cases
        .iter()
        .find(|case| case.id == "ridge_probe")
        .ok_or_else(|| anyhow!("embedded report has no ridge_probe"))?;
    let canary = &report.evaluation.ridge_canary;
    let selected = canary
        .waypoint_search
        .selected_candidate
        .as_ref()
        .ok_or_else(|| anyhow!("analytical canary has no selected waypoint witness"))?;
    let analytical_overlay = build_analytical_overlay(canary, selected)?;
    let mut direct_route = derive_direct_route_evidence(case, flat_terrain(case), &report.fixture)?;
    let direct_route_spec = direct_route.route.clone();
    let flat_scenario = build_scenario(
        case,
        flat_terrain(case),
        Some(direct_route_spec.clone()),
        "flat",
    )?;
    let mesa_scenario =
        build_scenario(case, mesa_terrain(canary), Some(direct_route_spec), "mesa")?;
    direct_route.structural_validation =
        "TransferRouteSpec::validate and ScenarioSpec::validate passed for both direct lanes"
            .to_owned();
    let route_adapter = adapt_waypoint_route(case, canary, selected)?;

    let mut lanes = Vec::new();
    let direct = built_in_controller_spec("transfer_pdg").expect("built-in direct controller");
    lanes.push(run_lane(
        "flat-direct",
        "flat",
        flat_scenario,
        direct.clone(),
        &canary.mesa,
        None,
    )?);
    lanes.push(run_lane(
        "mesa-direct",
        "mesa",
        mesa_scenario.clone(),
        direct,
        &canary.mesa,
        None,
    )?);

    if let Some(route) = route_adapter.route.clone() {
        let mut waypoint_scenario = mesa_scenario;
        waypoint_scenario.id = "ridge-canary-mesa-waypoint".to_owned();
        waypoint_scenario.name = "Ridge canary mesa waypoint shadow".to_owned();
        waypoint_scenario.description = "Full-controller shadow using one adapted analytical anchor; the analytical bridge is not replayed.".to_owned();
        waypoint_scenario.mission.transfer_route = Some(route);
        let waypoint = built_in_controller_spec("transfer_waypoint_pdg")
            .expect("built-in waypoint controller");
        lanes.push(run_lane(
            "mesa-waypoint",
            "mesa",
            waypoint_scenario,
            waypoint,
            &canary.mesa,
            Some(selected.waypoint_position_m),
        )?);
    } else {
        lanes.push(invalid_route_lane(&mesa_scenario));
    }

    Ok(ControllerShadowRun {
        schema_id: CONTROLLER_SHADOW_SCHEMA_ID.to_owned(),
        schema_version: CONTROLLER_SHADOW_SCHEMA_VERSION,
        setup_id: CONTROLLER_SHADOW_SETUP_ID.to_owned(),
        analytical_report_identity: report.identity.clone(),
        analytical_canary_identity: canary.identity.clone(),
        mesa_identity: canary.mesa.identity.clone(),
        waypoint_candidate_identity: canary
            .waypoint_search
            .selected_candidate_identity
            .clone()
            .expect("selected candidate identity"),
        direct_route,
        route_adapter,
        analytical_overlay,
        lanes,
        deterministic_repeat: false,
        semantic_identity: String::new(),
    })
}

fn flat_terrain(case: &DirectBridgeProbeV2) -> TerrainDefinition {
    TerrainDefinition::Heightfield {
        points_m: vec![
            case.terrain_points_m[0],
            *case.terrain_points_m.last().expect("terrain endpoint"),
        ],
    }
}

fn mesa_terrain(
    canary: &pd_plan::conservative_ballistic_bridge::RidgeCanaryEvidenceV2,
) -> TerrainDefinition {
    TerrainDefinition::Heightfield {
        points_m: canary.mesa.terrain_points_m.clone(),
    }
}

fn derive_direct_route_evidence(
    case: &DirectBridgeProbeV2,
    terrain: TerrainDefinition,
    fixture: &DirectBridgeFixtureV2,
) -> Result<DirectRouteEvidence> {
    let vehicle = vehicle_from_fixture(&fixture.vehicle);
    let source = LandingPadSpec {
        id: "source".to_owned(),
        center_x_m: case.source.center_x_m,
        surface_y_m: case.source.surface_y_m,
        width_m: case.source.width_m,
    };
    let target = LandingPadSpec {
        id: "target".to_owned(),
        center_x_m: case.target.center_x_m,
        surface_y_m: case.target.surface_y_m,
        width_m: case.target.width_m,
    };
    let request = RoutePlanningRequest {
        world: WorldSpec {
            gravity_mps2: fixture.policy.gravity_mps2,
            terrain,
            landing_pads: vec![source, target],
        },
        vehicle,
        initial_state: VehicleInitialState {
            position_m: case.initial_position_m,
            velocity_mps: case.initial_velocity_mps,
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        },
        source_pad_id: "source".to_owned(),
        target_pad_id: "target".to_owned(),
        policy: RoutePlanningPolicy::v1(),
    };
    let geometry = normalized_geometry(&request).map_err(|error| anyhow!(error.to_string()))?;
    let route = TransferRouteSpec {
        source_pad_id: request.source_pad_id,
        target_pad_id: request.target_pad_id,
        route_angle_deg: geometry.route_angle_deg,
        route_radius_m: geometry.direct_distance_m,
        waypoints: Vec::new(),
    };
    route.validate().map_err(|error| anyhow!(error))?;
    Ok(DirectRouteEvidence {
        route,
        normalized_direct_distance_m: geometry.direct_distance_m,
        normalized_route_angle_deg: geometry.route_angle_deg,
        structural_validation: "TransferRouteSpec::validate passed; ScenarioSpec validation follows for both direct lanes".to_owned(),
    })
}

const ANALYTICAL_PATH_POINT_LIMIT: usize = 96;

fn build_analytical_overlay(
    canary: &pd_plan::conservative_ballistic_bridge::RidgeCanaryEvidenceV2,
    candidate: &WaypointCandidateV2,
) -> Result<AnalyticalShadowOverlay> {
    let nominal = &canary.flat_control.nominal_candidate;
    if nominal.identity != canary.nominal_candidate_identity {
        bail!(
            "flat twin nominal candidate {} does not match canary nominal candidate {}",
            nominal.identity,
            canary.nominal_candidate_identity
        );
    }
    Ok(AnalyticalShadowOverlay {
        nominal_direct: AnalyticalOverlayPath {
            id: "nominal-direct".to_owned(),
            points_m: direct_candidate_points(nominal),
        },
        waypoint_source_bridge: AnalyticalOverlayPath {
            id: "waypoint-source-bridge".to_owned(),
            points_m: candidate
                .source_bridge
                .as_ref()
                .map(bridge_points)
                .unwrap_or_default(),
        },
        waypoint_source_leg: AnalyticalOverlayPath {
            id: "waypoint-source-leg".to_owned(),
            points_m: ballistic_arc_points(&candidate.source_leg),
        },
        waypoint_intermediate_bridge: AnalyticalOverlayPath {
            id: "waypoint-intermediate-bridge".to_owned(),
            points_m: candidate
                .intermediate_bridge
                .as_ref()
                .map(bridge_points)
                .unwrap_or_default(),
        },
        waypoint_target_leg: AnalyticalOverlayPath {
            id: "waypoint-target-leg".to_owned(),
            points_m: ballistic_arc_points(&candidate.target_leg),
        },
        waypoint_terminal_bridge: AnalyticalOverlayPath {
            id: "waypoint-terminal-bridge".to_owned(),
            points_m: candidate
                .terminal_bridge
                .as_ref()
                .map(bridge_points)
                .unwrap_or_default(),
        },
        waypoint_position_m: candidate.waypoint_position_m,
        nominal_apex_position_m: nominal.virtual_arc.apex_position_m,
    })
}

fn direct_candidate_points(candidate: &DirectBridgeCandidateV2) -> Vec<Vec2> {
    let mut points = candidate
        .source_bridge
        .as_ref()
        .map(bridge_points)
        .unwrap_or_else(|| vec![candidate.virtual_arc.start_m]);
    append_without_duplicate(&mut points, ballistic_arc_points(&candidate.virtual_arc));
    if let Some(bridge) = candidate.terminal_bridge.as_ref() {
        append_without_duplicate(&mut points, bridge_points(bridge));
    }
    points
}

fn bridge_points(bridge: &AnalyticalBridgeV2) -> Vec<Vec2> {
    sampled_steps(bridge.steps)
        .into_iter()
        .map(|step| bridge.state_at(step).position_m)
        .collect()
}

fn ballistic_arc_points(arc: &VirtualBallisticArcV2) -> Vec<Vec2> {
    sampled_steps(arc.steps)
        .into_iter()
        .map(|step| arc.state_at(step).position_m)
        .collect()
}

fn sampled_steps(steps: u64) -> Vec<u64> {
    if steps == 0 {
        return vec![0];
    }
    let stride = (steps as usize).div_ceil(ANALYTICAL_PATH_POINT_LIMIT);
    let mut sampled = (0..=steps).step_by(stride.max(1)).collect::<Vec<_>>();
    if sampled.last().copied() != Some(steps) {
        sampled.push(steps);
    }
    sampled
}

fn append_without_duplicate(target: &mut Vec<Vec2>, mut points: Vec<Vec2>) {
    if let (Some(last), Some(first)) = (target.last(), points.first())
        && *last == *first
    {
        points.remove(0);
    }
    target.extend(points);
}

fn build_scenario(
    case: &DirectBridgeProbeV2,
    terrain: TerrainDefinition,
    route: Option<TransferRouteSpec>,
    suffix: &str,
) -> Result<ScenarioSpec> {
    let fixture = build_report_artifact_v2().fixture;
    let vehicle = vehicle_from_fixture(&fixture.vehicle);
    let source = LandingPadSpec {
        id: "source".to_owned(),
        center_x_m: case.source.center_x_m,
        surface_y_m: case.source.surface_y_m,
        width_m: case.source.width_m,
    };
    let target = LandingPadSpec {
        id: "target".to_owned(),
        center_x_m: case.target.center_x_m,
        surface_y_m: case.target.surface_y_m,
        width_m: case.target.width_m,
    };
    let scenario = ScenarioSpec {
        id: format!("ridge-canary-{suffix}"),
        name: format!("Ridge canary {suffix}"),
        description: "Frozen V2 ridge canary controller shadow".to_owned(),
        seed: 0,
        tags: vec![
            "research".to_owned(),
            "controller-shadow".to_owned(),
            "ridge-canary".to_owned(),
        ],
        metadata: BTreeMap::from([
            ("source_case_id".to_owned(), case.id.clone()),
            (
                "shadow_schema".to_owned(),
                CONTROLLER_SHADOW_SCHEMA_ID.to_owned(),
            ),
        ]),
        sim: SimConfig {
            physics_hz: CONTROLLER_SHADOW_PHYSICS_HZ,
            controller_hz: CONTROLLER_SHADOW_CONTROLLER_HZ,
            max_time_s: CONTROLLER_SHADOW_MAX_TIME_S,
            sample_hz: Some(CONTROLLER_SHADOW_PHYSICS_HZ),
        },
        world: WorldSpec {
            gravity_mps2: fixture.policy.gravity_mps2,
            terrain,
            landing_pads: vec![source, target],
        },
        vehicle,
        initial_state: VehicleInitialState {
            position_m: case.initial_position_m,
            velocity_mps: case.initial_velocity_mps,
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        },
        mission: MissionSpec {
            transfer_route: route,
            goal: EvaluationGoal::LandingOnPad {
                target_pad_id: "target".to_owned(),
            },
        },
    };
    scenario.validate().map_err(|error| anyhow!(error))?;
    Ok(scenario)
}

fn vehicle_from_fixture(
    vehicle: &pd_plan::conservative_ballistic_bridge::VehicleInputV2,
) -> VehicleSpec {
    VehicleSpec {
        geometry: VehicleGeometry {
            hull_width_m: vehicle.geometry.hull_width_m,
            hull_height_m: vehicle.geometry.hull_height_m,
            touchdown_half_span_m: vehicle.geometry.touchdown_half_span_m,
            touchdown_base_offset_m: vehicle.geometry.touchdown_base_offset_m,
        },
        dry_mass_kg: vehicle.dry_mass_kg,
        initial_fuel_kg: vehicle.initial_fuel_kg,
        max_fuel_kg: vehicle.max_fuel_kg,
        max_thrust_n: vehicle.max_thrust_n,
        max_fuel_burn_kgps: vehicle.max_fuel_burn_kgps,
        min_throttle_frac: vehicle.min_throttle_frac,
        max_rotation_rate_radps: vehicle.max_rotation_rate_radps,
        safe_touchdown_normal_speed_mps: vehicle.safe_touchdown_normal_speed_mps,
        safe_touchdown_tangential_speed_mps: vehicle.safe_touchdown_tangential_speed_mps,
        safe_touchdown_attitude_error_rad: vehicle.safe_touchdown_attitude_error_rad,
        safe_touchdown_angular_rate_radps: vehicle.safe_touchdown_angular_rate_radps,
    }
}

fn adapt_waypoint_route(
    case: &DirectBridgeProbeV2,
    canary: &pd_plan::conservative_ballistic_bridge::RidgeCanaryEvidenceV2,
    candidate: &WaypointCandidateV2,
) -> Result<RouteAdapterEvidence> {
    let fixture = build_report_artifact_v2().fixture;
    let vehicle = vehicle_from_fixture(&fixture.vehicle);
    let source = LandingPadSpec {
        id: "source".to_owned(),
        center_x_m: case.source.center_x_m,
        surface_y_m: case.source.surface_y_m,
        width_m: case.source.width_m,
    };
    let target = LandingPadSpec {
        id: "target".to_owned(),
        center_x_m: case.target.center_x_m,
        surface_y_m: case.target.surface_y_m,
        width_m: case.target.width_m,
    };
    let request = RoutePlanningRequest {
        world: WorldSpec {
            gravity_mps2: fixture.policy.gravity_mps2,
            terrain: mesa_terrain(canary),
            landing_pads: vec![source, target],
        },
        vehicle,
        initial_state: VehicleInitialState {
            position_m: case.initial_position_m,
            velocity_mps: case.initial_velocity_mps,
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        },
        source_pad_id: "source".to_owned(),
        target_pad_id: "target".to_owned(),
        policy: RoutePlanningPolicy::v1(),
    };
    let geometry = normalized_geometry(&request).map_err(|error| anyhow!(error.to_string()))?;
    let capture_radius_m = (geometry.direct_distance_m * 0.08).clamp(35.0, 95.0);
    let normalized_waypoint = Vec2::new(
        f64::from(geometry.horizontal_sign)
            * (candidate.waypoint_position_m.x - request.source_pad().expect("source").center_x_m),
        candidate.waypoint_position_m.y,
    );
    let (profile, _) = build_endpoint_profile(&request, geometry.direct_horizontal_span_m)
        .map_err(|error| anyhow!(error.to_string()))?;
    let shaped = endpoint_shaped_centerline(&request, &geometry, &profile, &[normalized_waypoint])
        .map_err(|error| anyhow!(error.to_string()))?;
    let waypoint_index = shaped
        .iter()
        .position(|point| *point == normalized_waypoint)
        .ok_or_else(|| anyhow!("adapter waypoint was not retained by endpoint centerline"))?;
    let unit = |vector: Vec2| {
        let length = vector.length();
        if length <= f64::EPSILON {
            None
        } else {
            Some(vector * (1.0 / length))
        }
    };
    let inbound = unit(shaped[waypoint_index] - shaped[waypoint_index - 1])
        .ok_or_else(|| anyhow!("adapter inbound tangent is degenerate"))?;
    let outbound = unit(shaped[waypoint_index + 1] - shaped[waypoint_index])
        .ok_or_else(|| anyhow!("adapter outbound tangent is degenerate"))?;
    let normalized_tangent = unit(inbound + outbound)
        .ok_or_else(|| anyhow!("adapter route legs have opposing tangent"))?;
    let tangent = Vec2::new(
        f64::from(geometry.horizontal_sign) * normalized_tangent.x,
        normalized_tangent.y,
    );
    let preliminary = TransferWaypointSpec {
        id: "ridge-anchor-0".to_owned(),
        position_m: candidate.waypoint_position_m,
        handoff_tangent_unit: Some(tangent),
        capture_radius_m,
        max_cross_track_m: capture_radius_m,
        max_outbound_heading_error_rad: request.policy.max_outbound_heading_error_rad,
        min_outbound_progress_mps: request.policy.min_outbound_progress_mps,
        max_outbound_cross_speed_mps: Some(request.policy.max_outbound_cross_speed_mps),
        min_speed_mps: request.policy.min_handoff_speed_mps,
        max_speed_mps: request.policy.max_handoff_speed_mps,
        min_vertical_speed_mps: None,
        max_vertical_speed_mps: None,
    };
    let authority = compute_waypoint_authority(
        &request,
        shaped[waypoint_index - 1],
        shaped[waypoint_index],
        shaped[waypoint_index + 1],
        capture_radius_m,
    )
    .map_err(|error| anyhow!(error.to_string()))?;
    let max_speed_mps = authority
        .handoff_speed_cap_mps
        .min(request.policy.max_handoff_speed_mps);
    if max_speed_mps + 1.0e-9 < request.policy.min_handoff_speed_mps {
        return Ok(RouteAdapterEvidence {
            source: "analytical_waypoint_candidate_v2".to_owned(),
            analytical_waypoint_position_m: candidate.waypoint_position_m,
            capture_radius_m,
            max_cross_track_m: preliminary.max_cross_track_m,
            route: None,
            validation: None,
            error: Some(format!("derived waypoint authority cap {max_speed_mps}m/s is below policy minimum")),
            capture_radius_derivation: "existing transfer policy: clamp(0.08 * direct route radius, 35m, 95m)".to_owned(),
            authority_derivation: "pd_core::compute_waypoint_authority plus RoutePlanningPolicy::v1 speed bounds".to_owned(),
            bridge_mapping_note: "The analytical powered bridge is not a controller command trace; this is an attempted runtime-anchor adapter, rejected before execution.".to_owned(),
        });
    }
    let waypoint = TransferWaypointSpec {
        max_speed_mps,
        ..preliminary
    };
    let max_cross_track_m = waypoint.max_cross_track_m;
    let route = TransferRouteSpec {
        source_pad_id: request.source_pad_id.clone(),
        target_pad_id: request.target_pad_id.clone(),
        route_angle_deg: geometry.route_angle_deg,
        route_radius_m: geometry.direct_distance_m,
        waypoints: vec![waypoint],
    };
    match validate_route(&request, &route) {
        Ok(validation) => Ok(RouteAdapterEvidence {
            source: "analytical_waypoint_candidate_v2".to_owned(),
            analytical_waypoint_position_m: candidate.waypoint_position_m,
            capture_radius_m,
            max_cross_track_m,
            route: Some(route),
            validation: Some(validation),
            error: None,
            capture_radius_derivation: "existing transfer policy: clamp(0.08 * direct route radius, 35m, 95m)".to_owned(),
            authority_derivation: "pd_core::compute_waypoint_authority plus RoutePlanningPolicy::v1 speed bounds".to_owned(),
            bridge_mapping_note: "The analytical powered bridge is not a controller command trace; this route is an empirical runtime-anchor adapter.".to_owned(),
        }),
        Err(error) => Ok(RouteAdapterEvidence {
            source: "analytical_waypoint_candidate_v2".to_owned(),
            analytical_waypoint_position_m: candidate.waypoint_position_m,
            capture_radius_m,
            max_cross_track_m,
            route: None,
            validation: None,
            error: Some(format!(
                "{error}; waypoint=({:.6},{:.6}), tangent=({:.6},{:.6}), capture_radius={capture_radius_m:.6}, max_cross_track={:.6}, authority_cap={max_speed_mps:.6}",
                candidate.waypoint_position_m.x,
                candidate.waypoint_position_m.y,
                tangent.x,
                tangent.y,
                max_cross_track_m,
            )),
            capture_radius_derivation: "existing transfer policy: clamp(0.08 * direct route radius, 35m, 95m)".to_owned(),
            authority_derivation: "pd_core::compute_waypoint_authority plus RoutePlanningPolicy::v1 speed bounds".to_owned(),
            bridge_mapping_note: "The analytical powered bridge is not a controller command trace; this is an attempted runtime-anchor adapter, rejected before execution.".to_owned(),
        }),
    }
}

fn run_lane(
    id: &str,
    terrain_kind: &str,
    scenario: ScenarioSpec,
    controller: ControllerSpec,
    mesa: &pd_plan::conservative_ballistic_bridge::MesaGeometryV2,
    waypoint_position: Option<Vec2>,
) -> Result<ShadowLaneSummary> {
    let ctx = pd_core::RunContext::from_scenario(&scenario).map_err(|error| anyhow!(error))?;
    let run = run_controller_spec(&ctx, &controller).map_err(|error| anyhow!(error))?;
    let terrain_contact = terrain_contact(&run, &scenario, mesa);
    let waypoint_contract = waypoint_position.map(|_| waypoint_contract(&run));
    let preflight = controller_preflight(&scenario, &run, &terrain_contact);
    let end_reason = run.run.manifest.end_reason.clone();
    let class = if terrain_kind == "flat" && end_reason == EndReason::TouchdownOnTarget {
        ShadowLaneClass::TargetLanding
    } else if id == "mesa-direct"
        && end_reason == EndReason::Crash
        && terrain_contact.within_derived_mesa_span
    {
        ShadowLaneClass::TerrainCrash
    } else if id == "mesa-waypoint"
        && end_reason == EndReason::TouchdownOnTarget
        && waypoint_contract.as_ref().is_some_and(|e| e.contract_pass)
    {
        ShadowLaneClass::TargetLanding
    } else {
        ShadowLaneClass::Inconclusive
    };
    let causal_reason = match class {
        ShadowLaneClass::TargetLanding if id == "flat-direct" => {
            "flat twin target landing".to_owned()
        }
        ShadowLaneClass::TargetLanding if id == "mesa-waypoint" => {
            "runtime waypoint contract passed and target landing completed".to_owned()
        }
        ShadowLaneClass::TerrainCrash => {
            let contact = terrain_contact.contact_point_m.map_or_else(
                || "unknown point".to_owned(),
                |point| format!("({:.6},{:.6})", point.x, point.y),
            );
            let kind = terrain_contact.contact_kind.map_or_else(
                || "unknown contact".to_owned(),
                |kind| match kind {
                    TerrainContactKind::HullVertex => "hull vertex".to_owned(),
                    TerrainContactKind::TouchdownPoint => "touchdown point".to_owned(),
                },
            );
            let residual = terrain_contact.contact_residual_m.map_or_else(
                || "unknown residual".to_owned(),
                |value| format!("{value:.6}m"),
            );
            format!(
                "Crash event occurred at reconstructed {kind} {contact} on the derived mesa span (residual {residual}; reconstruction validated)"
            )
        }
        ShadowLaneClass::Inconclusive if end_reason == EndReason::Crash => {
            let center = terrain_contact.vehicle_center_m.map_or_else(
                || "unknown center".to_owned(),
                |point| format!("({:.6},{:.6})", point.x, point.y),
            );
            let contact = terrain_contact.contact_point_m.map_or_else(
                || "unknown contact point".to_owned(),
                |point| format!("({:.6},{:.6})", point.x, point.y),
            );
            let detail = match terrain_contact.reconstruction_valid {
                Some(false) => "contact reconstruction validation failed",
                Some(true) => "reconstructed contact point outside the derived mesa span",
                None => "contact reconstruction unavailable",
            };
            format!(
                "Crash at vehicle center {center}; contact {contact}; {detail}; acceptance gate not satisfied"
            )
        }
        _ => format!(
            "lane ended as {:?}; acceptance gate not satisfied",
            end_reason
        ),
    };
    Ok(ShadowLaneSummary {
        id: id.to_owned(),
        terrain_kind: terrain_kind.to_owned(),
        controller_id: controller.id().to_owned(),
        scenario_id: scenario.id.clone(),
        class,
        causal_reason,
        end_reason: Some(end_reason),
        physical_outcome: Some(run.run.manifest.physical_outcome.clone()),
        mission_outcome: Some(run.run.manifest.mission_outcome.clone()),
        sim_time_s: Some(run.run.manifest.sim_time_s),
        physics_steps: Some(run.run.manifest.physics_steps),
        fuel_remaining_kg: Some(run.run.manifest.summary.fuel_remaining_kg),
        min_touchdown_clearance_m: Some(run.run.manifest.summary.min_touchdown_clearance_m),
        min_hull_clearance_m: Some(run.run.manifest.summary.min_hull_clearance_m),
        terrain_contact,
        waypoint_contract,
        preflight,
        scenario: Some(scenario),
        controller: Some(controller),
        run: Some(run),
    })
}

fn invalid_route_lane(scenario: &ScenarioSpec) -> ShadowLaneSummary {
    ShadowLaneSummary {
        id: "mesa-waypoint".to_owned(),
        terrain_kind: "mesa".to_owned(),
        controller_id: "transfer_waypoint_pdg_v1".to_owned(),
        scenario_id: scenario.id.clone(),
        class: ShadowLaneClass::RouteMappingInvalid,
        causal_reason: "Analytical witness could not be mapped to a valid TransferRouteSpec under the frozen shared route contract; no controller run was executed.".to_owned(),
        end_reason: None,
        physical_outcome: None,
        mission_outcome: None,
        sim_time_s: None,
        physics_steps: None,
        fuel_remaining_kg: None,
        min_touchdown_clearance_m: None,
        min_hull_clearance_m: None,
        terrain_contact: TerrainContactEvidence {
            event_physics_step: None,
            event_time_s: None,
            vehicle_center_m: None,
            center_terrain_height_m: None,
            touchdown_clearance_m: None,
            hull_clearance_m: None,
            contact_point_m: None,
            contact_kind: None,
            contact_residual_m: None,
            contact_terrain_height_m: None,
            reconstructed_touchdown_clearance_m: None,
            reconstructed_hull_clearance_m: None,
            touchdown_clearance_error_m: None,
            hull_clearance_error_m: None,
            reconstruction_valid: None,
            within_derived_mesa_span: false,
        },
        waypoint_contract: None,
        preflight: ControllerPreflightEvidence {
            route: None,
            first_phase: None,
            first_target_attitude_rad: None,
            first_controller_update_physics_step: None,
            survived_source_launch: None,
        },
        scenario: None,
        controller: None,
        run: None,
    }
}

fn controller_preflight(
    scenario: &ScenarioSpec,
    run: &ControlledRunArtifacts,
    terrain_contact: &TerrainContactEvidence,
) -> ControllerPreflightEvidence {
    let first_update = run.controller_updates.first();
    ControllerPreflightEvidence {
        route: scenario.mission.transfer_route.clone(),
        first_phase: first_update.and_then(|update| update.frame.phase.clone()),
        first_target_attitude_rad: first_update
            .map(|update| update.frame.command.target_attitude_rad),
        first_controller_update_physics_step: first_update.map(|update| update.physics_step),
        survived_source_launch: Some(
            run.run.manifest.physics_steps > 1
                && !terrain_contact
                    .event_physics_step
                    .is_some_and(|physics_step| physics_step <= 1),
        ),
    }
}

const CONTACT_RECONSTRUCTION_TOLERANCE_M: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug)]
struct ReconstructedContact {
    point_m: Vec2,
    kind: TerrainContactKind,
    clearance_m: f64,
}

fn hull_vertices_world(center_m: Vec2, attitude_rad: f64, geometry: &VehicleGeometry) -> [Vec2; 4] {
    let half_w = geometry.hull_width_m * 0.5;
    let half_h = geometry.hull_height_m * 0.5;
    [
        Vec2::new(-half_w, -half_h),
        Vec2::new(half_w, -half_h),
        Vec2::new(half_w, half_h),
        Vec2::new(-half_w, half_h),
    ]
    .map(|point| center_m + point.rotated(attitude_rad))
}

fn touchdown_points_world(
    center_m: Vec2,
    attitude_rad: f64,
    geometry: &VehicleGeometry,
) -> [Vec2; 2] {
    let left_local = Vec2::new(
        -geometry.touchdown_half_span_m,
        -geometry.touchdown_base_offset_m,
    );
    let right_local = Vec2::new(
        geometry.touchdown_half_span_m,
        -geometry.touchdown_base_offset_m,
    );
    [
        center_m + left_local.rotated(attitude_rad),
        center_m + right_local.rotated(attitude_rad),
    ]
}

fn worst_reconstructed_contact(
    points: impl IntoIterator<Item = (Vec2, TerrainContactKind)>,
    terrain: &TerrainDefinition,
) -> ReconstructedContact {
    points
        .into_iter()
        .map(|(point_m, kind)| ReconstructedContact {
            point_m,
            kind,
            clearance_m: point_m.y - terrain.sample_height(point_m.x),
        })
        .min_by(|left, right| left.clearance_m.total_cmp(&right.clearance_m))
        .expect("vehicle geometry has contact points")
}

fn reconstruct_contact(
    sample: &pd_core::SampleRecord,
    scenario: &ScenarioSpec,
) -> (ReconstructedContact, f64, f64, f64, f64, bool) {
    let center_m = sample.observation.position_m;
    let attitude_rad = sample.observation.attitude_rad;
    let geometry = &scenario.vehicle.geometry;
    let hull = worst_reconstructed_contact(
        hull_vertices_world(center_m, attitude_rad, geometry)
            .into_iter()
            .map(|point| (point, TerrainContactKind::HullVertex)),
        &scenario.world.terrain,
    );
    let touchdown = worst_reconstructed_contact(
        touchdown_points_world(center_m, attitude_rad, geometry)
            .into_iter()
            .map(|point| (point, TerrainContactKind::TouchdownPoint)),
        &scenario.world.terrain,
    );
    let worst = [hull, touchdown]
        .into_iter()
        .min_by(|left, right| left.clearance_m.total_cmp(&right.clearance_m))
        .expect("hull and touchdown contacts are non-empty");
    let touchdown_error = (touchdown.clearance_m - sample.observation.touchdown_clearance_m).abs();
    let hull_error = (hull.clearance_m - sample.observation.min_hull_clearance_m).abs();
    let valid = touchdown_error <= CONTACT_RECONSTRUCTION_TOLERANCE_M
        && hull_error <= CONTACT_RECONSTRUCTION_TOLERANCE_M
        && touchdown_error.is_finite()
        && hull_error.is_finite();
    (
        worst,
        touchdown.clearance_m,
        hull.clearance_m,
        touchdown_error,
        hull_error,
        valid,
    )
}

fn terrain_contact(
    run: &ControlledRunArtifacts,
    scenario: &ScenarioSpec,
    mesa: &pd_plan::conservative_ballistic_bridge::MesaGeometryV2,
) -> TerrainContactEvidence {
    let event = run
        .run
        .events
        .iter()
        .find(|event| event.kind == pd_core::EventKind::Crash);
    let sample = event.and_then(|event| {
        run.run
            .samples
            .iter()
            .find(|sample| sample.physics_step == event.physics_step)
    });
    let vehicle_center_m = sample.map(|sample| sample.observation.position_m);
    let center_terrain_height_m =
        vehicle_center_m.map(|position| scenario.world.terrain.sample_height(position.x));
    let reconstruction = sample.map(|sample| reconstruct_contact(sample, scenario));
    let contact_point_m = reconstruction.map(|contact| contact.0.point_m);
    let contact_kind = reconstruction.map(|contact| contact.0.kind);
    let contact_residual_m = reconstruction.map(|contact| contact.0.clearance_m);
    let contact_terrain_height_m =
        reconstruction.map(|contact| scenario.world.terrain.sample_height(contact.0.point_m.x));
    let reconstructed_touchdown_clearance_m = reconstruction.map(|contact| contact.1);
    let reconstructed_hull_clearance_m = reconstruction.map(|contact| contact.2);
    let touchdown_clearance_error_m = reconstruction.map(|contact| contact.3);
    let hull_clearance_error_m = reconstruction.map(|contact| contact.4);
    let reconstruction_valid = reconstruction.map(|contact| contact.5);
    let within_derived_mesa_span = reconstruction.is_some_and(|contact| {
        contact.5
            && contact.0.point_m.x >= mesa.base_left_x_m
            && contact.0.point_m.x <= mesa.base_right_x_m
    });
    TerrainContactEvidence {
        event_physics_step: event.map(|event| event.physics_step),
        event_time_s: event.map(|event| event.sim_time_s),
        vehicle_center_m,
        center_terrain_height_m,
        touchdown_clearance_m: sample.map(|sample| sample.observation.touchdown_clearance_m),
        hull_clearance_m: sample.map(|sample| sample.observation.min_hull_clearance_m),
        contact_point_m,
        contact_kind,
        contact_residual_m,
        contact_terrain_height_m,
        reconstructed_touchdown_clearance_m,
        reconstructed_hull_clearance_m,
        touchdown_clearance_error_m,
        hull_clearance_error_m,
        reconstruction_valid,
        within_derived_mesa_span,
    }
}

fn waypoint_contract(run: &ControlledRunArtifacts) -> WaypointContractEvidence {
    let markers = run
        .controller_updates
        .iter()
        .flat_map(|update| update.frame.markers.iter())
        .filter(|marker| marker.id == marker::WAYPOINT_HANDOFF)
        .collect::<Vec<_>>();
    let marker = markers.last().copied();
    let metadata = marker.map(|marker| &marker.metadata);
    let get_f64 = |key: &str| metadata.and_then(|values| values.get(key)).and_then(as_f64);
    let get_text = |key: &str| {
        metadata
            .and_then(|values| values.get(key))
            .and_then(as_text)
    };
    let contract_pass = marker.is_some_and(|_| {
        get_text(metric::WAYPOINT_HANDOFF_RESOLUTION_REASON).as_deref() == Some("contract_pass")
            && get_text(metric::WAYPOINT_CAPTURE_STATUS).as_deref() == Some("captured")
    });
    WaypointContractEvidence {
        marker_count: markers.len(),
        contract_pass,
        capture_time_s: get_f64(metric::WAYPOINT_CAPTURE_TIME_S),
        capture_position_m: marker
            .and_then(|marker| marker.x_m.zip(marker.y_m).map(|(x, y)| Vec2::new(x, y))),
        closest_distance_m: get_f64(metric::WAYPOINT_CLOSEST_DISTANCE_M),
        cross_track_m: get_f64(metric::WAYPOINT_CROSS_TRACK_M),
        outbound_progress_mps: get_f64(metric::WAYPOINT_OUTBOUND_PROGRESS_MPS),
        outbound_cross_speed_mps: get_f64(metric::WAYPOINT_OUTBOUND_CROSS_SPEED_MPS),
        speed_mps: get_f64(metric::WAYPOINT_SPEED_MPS),
        resolution_reason: get_text(metric::WAYPOINT_HANDOFF_RESOLUTION_REASON),
    }
}

fn as_f64(value: &TelemetryValue) -> Option<f64> {
    match value {
        TelemetryValue::Float(value) => Some(*value),
        TelemetryValue::Integer(value) => Some(*value as f64),
        _ => None,
    }
}

fn as_text(value: &TelemetryValue) -> Option<String> {
    match value {
        TelemetryValue::Text(value) => Some(value.clone()),
        _ => None,
    }
}

fn semantic_lane(lane: &ShadowLaneSummary) -> serde_json::Value {
    let mut value = canonicalize_json(
        serde_json::to_value(lane).expect("shadow lane summary serializes for comparison"),
    );
    if let Some(run_artifacts) = lane.run.as_ref() {
        value
            .as_object_mut()
            .expect("serialized lane summary is an object")
            .insert("run".to_owned(), semantic_run(run_artifacts));
    }
    value
}

fn semantic_run(run: &ControlledRunArtifacts) -> serde_json::Value {
    let mut clone = run.clone();
    clone.performance.wall_time_us = 0;
    clone.performance.thread_cpu_time_us = None;
    for update in &mut clone.controller_updates {
        update.compute_time_us = None;
    }
    canonicalize_json(serde_json::to_value(clone).expect("controlled run artifacts serialize"))
}

fn canonicalize_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(canonicalize_json).collect())
        }
        serde_json::Value::Object(values) => serde_json::Value::Object({
            let mut entries = values
                .into_iter()
                .map(|(key, value)| (key, canonicalize_json(value)))
                .collect::<Vec<_>>();
            entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            entries.into_iter().collect()
        }),
        serde_json::Value::Number(number) if number.as_f64().is_some_and(|value| value == 0.0) => {
            serde_json::Value::from(0.0)
        }
        other => other,
    }
}

fn semantic_digest(run: &ControllerShadowRun) -> Result<String> {
    let lanes = run.lanes.iter().map(semantic_lane).collect::<Vec<_>>();
    let material = serde_json::json!({
        "schema_id": run.schema_id,
        "schema_version": run.schema_version,
        "setup_id": run.setup_id,
        "analytical_report_identity": run.analytical_report_identity,
        "analytical_canary_identity": run.analytical_canary_identity,
        "mesa_identity": run.mesa_identity,
        "waypoint_candidate_identity": run.waypoint_candidate_identity,
        "direct_route": serde_json::to_value(&run.direct_route)?,
        "route_adapter": serde_json::to_value(&run.route_adapter)?,
        "analytical_overlay": serde_json::to_value(&run.analytical_overlay)?,
        "lanes": lanes,
        "deterministic_repeat": run.deterministic_repeat,
    });
    Ok(crate::source_transition_canonical_digest(
        &canonicalize_json(material),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    fn shadow() -> &'static ControllerShadowRun {
        static SHADOW: OnceLock<ControllerShadowRun> = OnceLock::new();
        SHADOW.get_or_init(|| execute_shadow(&build_report_artifact_v2()).expect("shadow run"))
    }

    #[test]
    fn shadow_runs_frozen_lanes_and_records_causal_outcomes() {
        let shadow = shadow();
        assert_eq!(shadow.lanes.len(), 3);
        assert_eq!(shadow.lanes[0].id, "flat-direct");
        assert_eq!(shadow.lanes[1].id, "mesa-direct");
        assert_eq!(shadow.lanes[2].id, "mesa-waypoint");
        assert!(
            shadow.route_adapter.error.is_some(),
            "adapter unexpectedly passed"
        );
        assert_eq!(
            shadow.route_adapter.capture_radius_m,
            shadow.route_adapter.max_cross_track_m
        );
        assert_eq!(shadow.direct_route.route.waypoints.len(), 0);
        assert_eq!(
            shadow.direct_route.route.route_angle_deg,
            shadow.direct_route.normalized_route_angle_deg
        );
        assert_eq!(
            shadow.direct_route.route.route_radius_m,
            shadow.direct_route.normalized_direct_distance_m
        );
        let flat_route = shadow.lanes[0]
            .preflight
            .route
            .clone()
            .expect("flat direct route");
        let mesa_route = shadow.lanes[1]
            .preflight
            .route
            .clone()
            .expect("mesa direct route");
        assert_eq!(flat_route, mesa_route);
        assert_eq!(flat_route, shadow.direct_route.route);
        assert_eq!(shadow.lanes[0].class, ShadowLaneClass::TargetLanding);
        for lane in &shadow.lanes[..2] {
            assert_eq!(lane.preflight.first_phase.as_deref(), Some("takeoff"));
            assert!(
                lane.preflight
                    .first_target_attitude_rad
                    .is_some_and(|attitude| attitude.abs() <= 1.0e-12)
            );
            assert_eq!(lane.preflight.first_controller_update_physics_step, Some(0));
            assert_eq!(lane.preflight.survived_source_launch, Some(true));
            assert!(!lane.causal_reason.contains("source crash at"));
            assert!(lane.end_reason.is_some());
            if lane.class == ShadowLaneClass::TerrainCrash {
                assert!(lane.terrain_contact.within_derived_mesa_span);
            }
        }
        let flat_contact = &shadow.lanes[0].terrain_contact;
        assert!(flat_contact.vehicle_center_m.is_none());
        assert!(flat_contact.contact_point_m.is_none());
        assert!(flat_contact.reconstruction_valid.is_none());
        let report = build_report_artifact_v2();
        let mesa = &report.evaluation.ridge_canary.mesa;
        let mesa_lane = &shadow.lanes[1];
        assert_eq!(mesa_lane.class, ShadowLaneClass::TerrainCrash);
        let mesa_contact = &mesa_lane.terrain_contact;
        assert_eq!(
            mesa_contact.contact_kind,
            Some(TerrainContactKind::HullVertex)
        );
        assert_eq!(mesa_contact.reconstruction_valid, Some(true));
        let center = mesa_contact.vehicle_center_m.expect("mesa vehicle center");
        let contact = mesa_contact.contact_point_m.expect("mesa contact point");
        assert!((contact.x - center.x).abs() > 1.0);
        assert!(contact.x >= mesa.base_left_x_m && contact.x <= mesa.base_right_x_m);
        assert!(contact.x < mesa.top_left_x_m);
        assert!(mesa_contact.within_derived_mesa_span);
        assert!(
            mesa_contact
                .contact_residual_m
                .is_some_and(|residual| residual < 0.0)
        );
        assert!(mesa_contact.contact_terrain_height_m.is_some_and(|height| {
            let residual = mesa_contact.contact_residual_m.expect("contact residual");
            (contact.y - height - residual).abs() <= CONTACT_RECONSTRUCTION_TOLERANCE_M
        }));
        assert!(
            mesa_contact
                .hull_clearance_error_m
                .is_some_and(|error| error <= CONTACT_RECONSTRUCTION_TOLERANCE_M)
        );
        assert!(
            mesa_contact
                .touchdown_clearance_error_m
                .is_some_and(|error| error <= CONTACT_RECONSTRUCTION_TOLERANCE_M)
        );
        assert_eq!(shadow.lanes[2].class, ShadowLaneClass::RouteMappingInvalid);
        assert!(shadow.lanes[2].end_reason.is_none());
        assert!(shadow.lanes[2].physical_outcome.is_none());
        assert!(shadow.lanes[2].mission_outcome.is_none());
        assert!(shadow.lanes[2].sim_time_s.is_none());
        assert!(shadow.lanes[2].physics_steps.is_none());
        assert!(shadow.lanes[2].min_hull_clearance_m.is_none());
        assert!(shadow.lanes[2].run.is_none());
        assert!(shadow.lanes[2].terrain_contact.contact_point_m.is_none());
        assert!(
            shadow.lanes[2]
                .terrain_contact
                .reconstruction_valid
                .is_none()
        );
        let candidate = report
            .evaluation
            .ridge_canary
            .waypoint_search
            .selected_candidate
            .expect("selected waypoint candidate");
        assert!(shadow.analytical_overlay.nominal_direct.points_m.len() > 2);
        assert!(
            shadow
                .analytical_overlay
                .waypoint_source_bridge
                .points_m
                .len()
                > 2
        );
        assert!(shadow.analytical_overlay.waypoint_source_leg.points_m.len() > 2);
        assert!(
            shadow
                .analytical_overlay
                .waypoint_intermediate_bridge
                .points_m
                .len()
                > 2
        );
        assert!(shadow.analytical_overlay.waypoint_target_leg.points_m.len() > 2);
        assert!(
            shadow
                .analytical_overlay
                .waypoint_terminal_bridge
                .points_m
                .len()
                > 2
        );
        assert_eq!(
            shadow
                .analytical_overlay
                .waypoint_source_leg
                .points_m
                .first(),
            Some(&candidate.source_leg.state_at(0).position_m)
        );
        assert_eq!(
            shadow
                .analytical_overlay
                .waypoint_source_leg
                .points_m
                .last(),
            Some(
                &candidate
                    .source_leg
                    .state_at(candidate.source_leg.steps)
                    .position_m
            )
        );
        assert_eq!(
            shadow
                .analytical_overlay
                .waypoint_target_leg
                .points_m
                .first(),
            Some(&candidate.target_leg.state_at(0).position_m)
        );
        assert_eq!(
            shadow
                .analytical_overlay
                .waypoint_target_leg
                .points_m
                .last(),
            Some(
                &candidate
                    .target_leg
                    .state_at(candidate.target_leg.steps)
                    .position_m
            )
        );
    }

    #[test]
    fn shadow_repeat_excludes_compute_timing_from_semantics() {
        let report = build_report_artifact_v2();
        let first = execute_shadow(&report).expect("first shadow");
        let second = execute_shadow(&report).expect("second shadow");
        assert_eq!(
            semantic_digest(&first).unwrap(),
            semantic_digest(&second).unwrap()
        );
        let mut changed_lane = second.clone();
        changed_lane.lanes[2].causal_reason.push_str(" changed");
        assert_ne!(
            semantic_digest(&first).unwrap(),
            semantic_digest(&changed_lane).unwrap()
        );
        let mut changed_adapter = second;
        changed_adapter
            .route_adapter
            .bridge_mapping_note
            .push_str(" changed");
        assert_ne!(
            semantic_digest(&first).unwrap(),
            semantic_digest(&changed_adapter).unwrap()
        );
    }

    #[test]
    fn persisted_shadow_reload_rejects_semantic_tampering() {
        let root = std::env::temp_dir().join(format!(
            "pd-eval-controller-shadow-test-{}",
            std::process::id()
        ));
        let summary = root.join("summary.json");
        let mut report = shadow().clone();
        report.semantic_identity = semantic_digest(&report).unwrap();
        fs::create_dir_all(&root).unwrap();
        write_shadow_artifacts(&root, &report).unwrap();
        let loaded = load_controller_shadow(&summary).unwrap();
        assert_eq!(loaded.semantic_identity, report.semantic_identity);
        assert_eq!(semantic_digest(&loaded).unwrap(), report.semantic_identity);
        let mut tampered = report;
        tampered.lanes[0].class = ShadowLaneClass::Inconclusive;
        fs::write(&summary, serde_json::to_vec_pretty(&tampered).unwrap()).unwrap();
        assert!(load_controller_shadow(&summary).is_err());
    }
}
