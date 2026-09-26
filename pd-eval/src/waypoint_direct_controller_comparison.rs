//! Opt-in comparison of the sealed analytical direct-topology sweep against
//! the unchanged direct transfer controller.
//!
//! The supplied sweep is validated and compared with the current frozen
//! analytical build before any controller invocation. Each run then changes
//! only the characterization scenario id and terrain points from one stored
//! probe. No planner is called, and this module does not change controller or
//! simulator behavior.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_control::{ControllerSpec, built_in_controller_spec};
use pd_core::{LandingPadSpec, ScenarioSpec, TerrainDefinition, VehicleSpec};
use pd_plan::conservative_ballistic_bridge::{PadInputV2, VehicleGeometryInputV2, VehicleInputV2};
use serde::{Deserialize, Serialize};

use crate::{
    waypoint_direct_characterization::{
        ControllerEvidence, continuous_flat_controller_scenario, run_direct_controller,
    },
    waypoint_direct_topology_sweep::{
        TopologyCellEvidence, TopologyClassification, WAYPOINT_DIRECT_TOPOLOGY_SWEEP_ID,
        WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_ID, WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_VERSION,
        WaypointDirectTopologySweepArtifact, build_artifact as build_current_sweep,
    },
};

pub const WAYPOINT_DIRECT_CONTROLLER_COMPARISON_ID: &str = "waypoint-direct-controller-comparison";
pub const WAYPOINT_DIRECT_CONTROLLER_COMPARISON_SCHEMA_ID: &str =
    "waypoint_direct_controller_comparison_v1";
pub const WAYPOINT_DIRECT_CONTROLLER_COMPARISON_SCHEMA_VERSION: u32 = 1;

const EXPECTED_SWEEP_IDENTITY: &str = "fnv1a64:1bcd5a3bd6c6da01";
const EXPECTED_CELL_COUNT: usize = 24;
const EXPECTED_PHYSICS_HZ: u32 = 120;
const EXPECTED_GRAVITY_MPS2: f64 = 9.81;
const EXPECTED_MAX_TIME_S: f64 = 90.0;
const CONTROLLER_SPEC_NAME: &str = "transfer_pdg";

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectControllerComparisonPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectControllerComparisonRun {
    pub artifact: WaypointDirectControllerComparisonArtifact,
    pub paths: WaypointDirectControllerComparisonPaths,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectControllerComparisonArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub comparison_id: String,
    pub analytical_source: AnalyticalSourceBinding,
    pub protocol: ComparisonProtocolEvidence,
    pub waypoint_controller_execution: WaypointControllerExecutionEvidence,
    pub scope: ComparisonScopeEvidence,
    pub cases: Vec<DirectControllerCaseEvidence>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalyticalSourceBinding {
    pub sweep_id: String,
    pub schema_id: String,
    pub schema_version: u32,
    pub identity: String,
    pub semantic_identity_rule: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComparisonProtocolEvidence {
    pub controller_spec_name: String,
    pub controller_spec_id: String,
    pub controller_runner: String,
    pub row_rule: String,
    pub planner_called: bool,
    pub simulation: String,
    pub semantic_identity_excludes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointControllerExecutionEvidence {
    pub run_count: usize,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComparisonScopeEvidence {
    pub claims: Vec<String>,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectControllerCaseEvidence {
    pub cell_id: String,
    pub cell_identity: String,
    pub classification: TopologyClassification,
    pub probe_identity: String,
    pub direct_result_identity: Option<String>,
    pub scenario_id: String,
    pub scenario_identity: String,
    pub controller_id: String,
    pub status: String,
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
    /// Retains the exact run_direct_controller error for unsupported/error rows.
    pub error: Option<String>,
}

struct PreparedComparison {
    source: WaypointDirectTopologySweepArtifact,
    scenarios: Vec<ScenarioSpec>,
}

/// Validate the sealed analytical input, adapt every stored probe to the
/// existing flat controller scenario, execute the unchanged direct
/// controller, and write a single deterministic `summary.json` artifact.
pub fn run_waypoint_direct_controller_comparison(
    repo_root: &Path,
    sweep_summary_path: &Path,
    output_dir: &Path,
) -> Result<WaypointDirectControllerComparisonRun> {
    let prepared = prepare_comparison(repo_root, sweep_summary_path)?;
    let controller_spec = built_in_controller_spec(CONTROLLER_SPEC_NAME)
        .ok_or_else(|| anyhow!("built-in controller {CONTROLLER_SPEC_NAME} is unavailable"))?;
    let mut cases = Vec::with_capacity(prepared.source.cases.len());
    for (cell, scenario) in prepared.source.cases.iter().zip(&prepared.scenarios) {
        let evidence = match run_direct_controller(scenario, &controller_spec) {
            Ok(evidence) => evidence,
            Err(error) => error_controller_evidence(&controller_spec, error.to_string()),
        };
        cases.push(project_case(cell, scenario, evidence)?);
    }

    let mut artifact = WaypointDirectControllerComparisonArtifact {
        schema_id: WAYPOINT_DIRECT_CONTROLLER_COMPARISON_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_CONTROLLER_COMPARISON_SCHEMA_VERSION,
        comparison_id: WAYPOINT_DIRECT_CONTROLLER_COMPARISON_ID.to_owned(),
        analytical_source: AnalyticalSourceBinding {
            sweep_id: prepared.source.sweep_id.clone(),
            schema_id: prepared.source.schema_id.clone(),
            schema_version: prepared.source.schema_version,
            identity: prepared.source.identity.clone(),
            semantic_identity_rule:
                "source identity is the exact semantic identity of the validated sealed topology sweep".to_owned(),
        },
        protocol: ComparisonProtocolEvidence {
            controller_spec_name: CONTROLLER_SPEC_NAME.to_owned(),
            controller_spec_id: controller_spec.id().to_owned(),
            controller_runner: "pd_eval::waypoint_direct_characterization::run_direct_controller".to_owned(),
            row_rule: "one row per stored analytical topology cell, in source artifact order; every row is attempted".to_owned(),
            planner_called: false,
            simulation: "existing direct-controller runner and unchanged simulator; sampled observations only".to_owned(),
            semantic_identity_excludes: vec![
                "input and output filesystem paths".to_owned(),
                "wall-clock and controller performance timing".to_owned(),
            ],
        },
        waypoint_controller_execution: WaypointControllerExecutionEvidence {
            run_count: 0,
            reason: "The sealed analytical artifact contains zero analytical_one_waypoint cells; no waypoint controller runs were eligible.".to_owned(),
        },
        scope: ComparisonScopeEvidence {
            claims: vec![
                "The unchanged transfer_pdg direct controller was attempted once for each stored probe terrain in the sealed 24-cell analytical sweep; per-row status records whether execution completed or was unsupported/error.".to_owned(),
                "Each scenario retained the continuous-flat characterization pads, route, vehicle, initial state, and simulator setup.".to_owned(),
                "Controller outcomes and sampled clearance observations are reported separately from analytical topology classifications.".to_owned(),
            ],
            non_claims: vec![
                "A controller failure on a cell does not establish physical impossibility.".to_owned(),
                "These 24 probes do not establish general terrain-family or waypoint-controller performance.".to_owned(),
                "This comparison does not change or promote planner selection, route topology, controller behavior, or defaults.".to_owned(),
            ],
        },
        cases,
        identity: String::new(),
    };
    artifact.identity = comparison_artifact_identity(&artifact)?;

    let output_dir = resolve_output_dir(repo_root, output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create waypoint direct controller comparison output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&summary_path, &summary_bytes).with_context(|| {
        format!(
            "failed to write waypoint direct controller comparison summary {}",
            summary_path.display()
        )
    })?;
    let reloaded: WaypointDirectControllerComparisonArtifact =
        serde_json::from_slice(&summary_bytes)
            .context("failed to reload waypoint direct controller comparison summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes {
        bail!("waypoint direct controller comparison summary is not byte-stable after reload");
    }
    if comparison_artifact_identity(&reloaded)? != reloaded.identity {
        bail!("waypoint direct controller comparison semantic identity failed round-trip check");
    }

    Ok(WaypointDirectControllerComparisonRun {
        artifact: reloaded,
        paths: WaypointDirectControllerComparisonPaths {
            output_dir,
            summary_path,
        },
    })
}

fn prepare_comparison(repo_root: &Path, sweep_summary_path: &Path) -> Result<PreparedComparison> {
    let raw = fs::read(sweep_summary_path).with_context(|| {
        format!(
            "failed to read sealed topology sweep summary {}",
            sweep_summary_path.display()
        )
    })?;
    let source: WaypointDirectTopologySweepArtifact =
        serde_json::from_slice(&raw).with_context(|| {
            format!(
                "failed to parse sealed topology sweep summary {}",
                sweep_summary_path.display()
            )
        })?;
    let current = build_current_sweep(repo_root)?;
    validate_source_sweep(&source, &current)?;

    let flat_scenario = continuous_flat_controller_scenario(repo_root)?;
    validate_flat_scenario(&flat_scenario, &source)?;
    let scenarios = adapt_probe_scenarios(&flat_scenario, &source)?;
    Ok(PreparedComparison { source, scenarios })
}

fn validate_source_sweep(
    source: &WaypointDirectTopologySweepArtifact,
    current: &WaypointDirectTopologySweepArtifact,
) -> Result<()> {
    if source.schema_id != WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_ID
        || source.schema_version != WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_VERSION
        || source.sweep_id != WAYPOINT_DIRECT_TOPOLOGY_SWEEP_ID
    {
        bail!("sealed topology sweep schema or identity is unsupported");
    }
    if sweep_artifact_identity(source)? != source.identity {
        bail!("sealed topology sweep semantic identity does not match its contents");
    }
    if source.source_flat_probe_identity != stable_digest(&source.source_flat_probe)? {
        bail!("sealed topology sweep source flat probe identity does not match its probe");
    }
    if source.policy_identity != stable_digest(&source.policy)? {
        bail!("sealed topology sweep policy identity does not match its policy");
    }
    if source.vehicle_identity != stable_digest(&source.vehicle)? {
        bail!("sealed topology sweep vehicle identity does not match its vehicle");
    }
    if source.cases.len() != EXPECTED_CELL_COUNT {
        bail!(
            "sealed topology sweep must contain {EXPECTED_CELL_COUNT} cells, found {}",
            source.cases.len()
        );
    }
    for cell in &source.cases {
        if cell.probe_identity != stable_digest(&cell.probe)? {
            bail!(
                "sealed topology cell {} probe identity does not match its stored probe",
                cell.id
            );
        }
        if cell.id != cell.probe.id {
            bail!(
                "sealed topology cell {} does not match its probe id",
                cell.id
            );
        }
    }
    if source.identity != EXPECTED_SWEEP_IDENTITY {
        bail!(
            "sealed topology sweep identity must be {EXPECTED_SWEEP_IDENTITY}, got {}",
            source.identity
        );
    }
    if source.classification_counts.analytical_direct != EXPECTED_CELL_COUNT
        || source.classification_counts.analytical_one_waypoint != 0
        || source.classification_counts.unknown != 0
        || source.classification_counts.invalid_setup != 0
        || source
            .cases
            .iter()
            .any(|cell| cell.classification != TopologyClassification::AnalyticalDirect)
    {
        bail!("sealed topology sweep classifications are outside the direct-only comparison scope");
    }
    if source.protocol.controller_called
        || source.protocol.planner_called
        || source.protocol.simulation_called
    {
        bail!(
            "sealed topology sweep protocol unexpectedly called a planner, controller, or simulator"
        );
    }
    if source != current {
        bail!("sealed topology sweep does not exactly match the current frozen analytical build");
    }
    if current.identity != EXPECTED_SWEEP_IDENTITY {
        bail!(
            "current frozen topology sweep identity changed: expected {EXPECTED_SWEEP_IDENTITY}, got {}",
            current.identity
        );
    }
    Ok(())
}

fn validate_flat_scenario(
    scenario: &ScenarioSpec,
    source: &WaypointDirectTopologySweepArtifact,
) -> Result<()> {
    scenario.validate().map_err(anyhow::Error::msg)?;
    if scenario.sim.physics_hz != EXPECTED_PHYSICS_HZ
        || scenario.sim.physics_hz != source.policy.physics_hz
        || scenario.world.gravity_mps2 != EXPECTED_GRAVITY_MPS2
        || scenario.world.gravity_mps2 != source.policy.gravity_mps2
        || scenario.sim.max_time_s != EXPECTED_MAX_TIME_S
        || scenario.sim.max_time_s != source.policy.maximum_mission_time_s
    {
        bail!(
            "continuous-flat scenario cadence, gravity, or time horizon differs from the sweep policy"
        );
    }
    if vehicle_input_v2(&scenario.vehicle) != source.vehicle {
        bail!("continuous-flat scenario vehicle differs from the sweep vehicle");
    }
    if scenario.initial_state.position_m != source.source_flat_probe.initial_position_m
        || scenario.initial_state.velocity_mps != source.source_flat_probe.initial_velocity_mps
    {
        bail!("continuous-flat scenario initial position or velocity differs from the sweep probe");
    }
    let route = scenario
        .mission
        .transfer_route
        .as_ref()
        .ok_or_else(|| anyhow!("continuous-flat scenario has no transfer route"))?;
    if !route.waypoints.is_empty() {
        bail!("continuous-flat scenario must have a direct route with zero waypoints");
    }
    if route.target_pad_id != scenario.mission.goal.target_pad_id() {
        bail!("continuous-flat scenario route target differs from its mission goal");
    }
    let source_pad = scenario
        .world
        .landing_pad(&route.source_pad_id)
        .ok_or_else(|| anyhow!("continuous-flat scenario route source pad is missing"))?;
    let target_pad = scenario
        .world
        .landing_pad(&route.target_pad_id)
        .ok_or_else(|| anyhow!("continuous-flat scenario route target pad is missing"))?;
    if !pad_matches(source_pad, &source.source_flat_probe.source)
        || !pad_matches(target_pad, &source.source_flat_probe.target)
    {
        bail!("continuous-flat scenario source or target pad differs from the sweep probe");
    }
    if scenario.world.landing_pads.len() != 2 {
        bail!("continuous-flat scenario must contain exactly its source and target pads");
    }
    if scenario.world.terrain.points() != source.source_flat_probe.terrain_points_m.as_slice() {
        bail!("continuous-flat scenario terrain differs from the sweep base probe");
    }
    Ok(())
}

fn adapt_probe_scenarios(
    flat_scenario: &ScenarioSpec,
    source: &WaypointDirectTopologySweepArtifact,
) -> Result<Vec<ScenarioSpec>> {
    let mut scenarios = Vec::with_capacity(source.cases.len());
    for cell in &source.cases {
        let mut scenario = flat_scenario.clone();
        scenario.id = scenario_id_for_cell(cell);
        scenario.world.terrain = TerrainDefinition::Heightfield {
            points_m: cell.probe.terrain_points_m.clone(),
        };
        scenario.validate().map_err(|error| {
            anyhow!("adapted scenario for cell {} is invalid: {error}", cell.id)
        })?;
        scenarios.push(scenario);
    }
    Ok(scenarios)
}

fn scenario_id_for_cell(cell: &TopologyCellEvidence) -> String {
    format!("{WAYPOINT_DIRECT_CONTROLLER_COMPARISON_ID}_{}", cell.id)
}

fn project_case(
    cell: &TopologyCellEvidence,
    scenario: &ScenarioSpec,
    controller: ControllerEvidence,
) -> Result<DirectControllerCaseEvidence> {
    Ok(DirectControllerCaseEvidence {
        cell_id: cell.id.clone(),
        cell_identity: stable_digest(cell)?,
        classification: cell.classification,
        probe_identity: cell.probe_identity.clone(),
        direct_result_identity: cell.direct_result_identity.clone(),
        scenario_id: scenario.id.clone(),
        scenario_identity: stable_digest(scenario)?,
        controller_id: controller.controller_id,
        status: controller.status,
        physical_outcome: controller.physical_outcome,
        mission_outcome: controller.mission_outcome,
        end_reason: controller.end_reason,
        sim_time_s: controller.sim_time_s,
        physics_steps: controller.physics_steps,
        observed_global_min_hull_clearance_m: controller.observed_global_min_hull_clearance_m,
        observed_en_route_min_hull_clearance_m: controller.observed_en_route_min_hull_clearance_m,
        observed_min_touchdown_clearance_m: controller.observed_min_touchdown_clearance_m,
        observed_apex_y_m: controller.observed_apex_y_m,
        observed_apex_above_target_m: controller.observed_apex_above_target_m,
        error: controller.error,
    })
}

fn error_controller_evidence(
    controller_spec: &ControllerSpec,
    error: String,
) -> ControllerEvidence {
    ControllerEvidence {
        status: "error".to_owned(),
        controller_id: controller_spec.id().to_owned(),
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
    }
}

fn pad_matches(pad: &LandingPadSpec, probe: &PadInputV2) -> bool {
    pad.center_x_m == probe.center_x_m
        && pad.surface_y_m == probe.surface_y_m
        && pad.width_m == probe.width_m
}

fn vehicle_input_v2(vehicle: &VehicleSpec) -> VehicleInputV2 {
    VehicleInputV2 {
        geometry: VehicleGeometryInputV2 {
            hull_width_m: vehicle.geometry.hull_width_m,
            hull_height_m: vehicle.geometry.hull_height_m,
            touchdown_half_span_m: vehicle.geometry.touchdown_half_span_m,
            touchdown_base_offset_m: vehicle.geometry.touchdown_base_offset_m,
        },
        dry_mass_kg: vehicle.dry_mass_kg,
        initial_fuel_kg: vehicle.initial_fuel_kg,
        max_fuel_kg: vehicle.max_fuel_kg,
        max_fuel_burn_kgps: vehicle.max_fuel_burn_kgps,
        max_thrust_n: vehicle.max_thrust_n,
        min_throttle_frac: vehicle.min_throttle_frac,
        max_rotation_rate_radps: vehicle.max_rotation_rate_radps,
        safe_touchdown_normal_speed_mps: vehicle.safe_touchdown_normal_speed_mps,
        safe_touchdown_tangential_speed_mps: vehicle.safe_touchdown_tangential_speed_mps,
        safe_touchdown_attitude_error_rad: vehicle.safe_touchdown_attitude_error_rad,
        safe_touchdown_angular_rate_radps: vehicle.safe_touchdown_angular_rate_radps,
    }
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

fn sweep_artifact_identity(artifact: &WaypointDirectTopologySweepArtifact) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

fn comparison_artifact_identity(
    artifact: &WaypointDirectControllerComparisonArtifact,
) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

fn resolve_output_dir(repo_root: &Path, requested: &Path) -> PathBuf {
    if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        repo_root.join(requested)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waypoint_direct_topology_sweep::build_artifact as build_sweep;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval crate is under repository root")
            .to_path_buf()
    }

    fn sweep() -> WaypointDirectTopologySweepArtifact {
        build_sweep(&repo_root()).expect("frozen topology sweep builds")
    }

    fn flat_scenario() -> ScenarioSpec {
        continuous_flat_controller_scenario(&repo_root())
            .expect("continuous-flat characterization scenario builds")
    }

    #[test]
    fn source_validation_rejects_tampered_probe_before_any_controller_run() {
        let current = sweep();
        validate_source_sweep(&current, &current).expect("current sealed sweep validates");

        let mut tampered = current.clone();
        tampered.cases[0].probe.terrain_points_m[4].y += 1.0;
        tampered.identity = sweep_artifact_identity(&tampered).expect("tampered source reseals");
        let error = validate_source_sweep(&tampered, &current)
            .expect_err("a probe changed without its own digest must be rejected")
            .to_string();
        assert!(
            error.contains("probe identity"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn all_24_adaptations_change_only_id_and_terrain_and_validate() {
        let source = sweep();
        let flat = flat_scenario();
        validate_flat_scenario(&flat, &source).expect("flat scenario matches sweep base");
        let scenarios = adapt_probe_scenarios(&flat, &source).expect("all probes adapt");
        assert_eq!(scenarios.len(), EXPECTED_CELL_COUNT);

        for (cell, scenario) in source.cases.iter().zip(&scenarios) {
            let mut expected = flat.clone();
            expected.id = scenario_id_for_cell(cell);
            expected.world.terrain = TerrainDefinition::Heightfield {
                points_m: cell.probe.terrain_points_m.clone(),
            };
            assert_eq!(
                scenario, &expected,
                "cell {} changed an extra field",
                cell.id
            );
            assert_eq!(scenario.world.terrain.points(), cell.probe.terrain_points_m);
            scenario.validate().expect("adapted scenario remains valid");
        }
    }

    #[test]
    fn adaptations_preserve_vehicle_route_pads_initial_state_and_simulation_setup() {
        let source = sweep();
        let flat = flat_scenario();
        let scenarios = adapt_probe_scenarios(&flat, &source).expect("all probes adapt");
        for scenario in &scenarios {
            assert_eq!(scenario.vehicle, flat.vehicle);
            assert_eq!(scenario.initial_state, flat.initial_state);
            assert_eq!(scenario.sim, flat.sim);
            assert_eq!(scenario.world.gravity_mps2, flat.world.gravity_mps2);
            assert_eq!(scenario.world.landing_pads, flat.world.landing_pads);
            assert_eq!(scenario.mission, flat.mission);
            assert_eq!(
                scenario
                    .mission
                    .transfer_route
                    .as_ref()
                    .expect("transfer route")
                    .waypoints
                    .len(),
                0
            );
        }
    }

    #[test]
    fn compact_semantic_projection_is_deterministic() {
        let source = sweep();
        let cell = &source.cases[0];
        let scenario = adapt_probe_scenarios(&flat_scenario(), &source)
            .expect("all probes adapt")
            .remove(0);
        let evidence = ControllerEvidence {
            status: "completed".to_owned(),
            controller_id: "transfer_pdg_v1".to_owned(),
            physical_outcome: Some("crashed".to_owned()),
            mission_outcome: Some("failed_crash".to_owned()),
            end_reason: Some("crash".to_owned()),
            sim_time_s: Some(12.5),
            physics_steps: Some(1500),
            observed_global_min_hull_clearance_m: Some(-2.0),
            observed_en_route_min_hull_clearance_m: Some(-2.0),
            observed_min_touchdown_clearance_m: Some(-2.0),
            observed_apex_y_m: Some(56.0),
            observed_apex_above_target_m: Some(56.0),
            telemetry_apex_over_target_m: None,
            telemetry_boost_apex_target_m: None,
            trajectory: Vec::new(),
            clearance_observation: "sampled_physics_observations".to_owned(),
            error: None,
        };
        let first = project_case(cell, &scenario, evidence.clone()).expect("projection succeeds");
        let second = project_case(cell, &scenario, evidence).expect("projection succeeds");
        assert_eq!(first, second);

        let mut artifact = WaypointDirectControllerComparisonArtifact {
            schema_id: WAYPOINT_DIRECT_CONTROLLER_COMPARISON_SCHEMA_ID.to_owned(),
            schema_version: WAYPOINT_DIRECT_CONTROLLER_COMPARISON_SCHEMA_VERSION,
            comparison_id: WAYPOINT_DIRECT_CONTROLLER_COMPARISON_ID.to_owned(),
            analytical_source: AnalyticalSourceBinding {
                sweep_id: source.sweep_id,
                schema_id: source.schema_id,
                schema_version: source.schema_version,
                identity: source.identity,
                semantic_identity_rule: "test".to_owned(),
            },
            protocol: ComparisonProtocolEvidence {
                controller_spec_name: CONTROLLER_SPEC_NAME.to_owned(),
                controller_spec_id: "transfer_pdg_v1".to_owned(),
                controller_runner: "existing".to_owned(),
                row_rule: "one row".to_owned(),
                planner_called: false,
                simulation: "unchanged".to_owned(),
                semantic_identity_excludes: Vec::new(),
            },
            waypoint_controller_execution: WaypointControllerExecutionEvidence {
                run_count: 0,
                reason: "no eligible rows".to_owned(),
            },
            scope: ComparisonScopeEvidence {
                claims: Vec::new(),
                non_claims: Vec::new(),
            },
            cases: vec![first],
            identity: String::new(),
        };
        let first_identity = comparison_artifact_identity(&artifact).expect("identity computes");
        let second_identity = comparison_artifact_identity(&artifact).expect("identity computes");
        assert_eq!(first_identity, second_identity);
        artifact.identity = first_identity;
        let bytes = serde_json::to_vec_pretty(&artifact).expect("artifact serializes");
        let reloaded: WaypointDirectControllerComparisonArtifact =
            serde_json::from_slice(&bytes).expect("artifact reloads");
        assert_eq!(
            comparison_artifact_identity(&reloaded).unwrap(),
            reloaded.identity
        );
    }
}
