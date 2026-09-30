//! Evaluator-only finite shooting experiment for a frozen source handoff.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::{RunContext, ScenarioSpec, Vec2};
use pd_plan::conservative_ballistic_bridge::{
    AnalyticalBridgeV2, BridgeKindV2, DirectBridgeCandidateV2, DirectBridgePolicyV2,
    DirectBridgeProbeV2, KinematicStateV2, VehicleInputV2, exact_discrete_bridge_v2,
};
use serde::{Deserialize, Serialize};

use super::super::{
    ContactEvidence, LaunchFeasibilityCadenceRunEvidence, MarginEvidence, RolloutCadence,
    SourceClearanceScreenEvidence,
    launch_contact_contract::{
        FirstContactEvidence, FirstContactPredicateMarginsEvidence, ReplayTraceParityEvidence,
        replay_logged_cadence,
    },
    launch_feasibility::{
        HeldSourceCommand, SourceDurationCommandSample, SourceDurationDiagnosticCadenceRun,
        SourceDurationReplayStage, SourceDurationRunRequest,
        run_source_duration_variant_with_held_schedule,
    },
    source_contact::mirror_v2_clearance,
    stable_digest,
};
use super::{
    PreparedDurationVariant, SourceDurationVariantEvidence,
    WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths,
    held_cadence_diagnostic::{
        PreparedHeldCadenceInputs, RunWithSamples, baseline_matches_frozen, frozen_flight,
        prepare_held_cadence_inputs, run_lane, validation_from_prepared,
    },
};

pub const WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_ID: &str =
    "waypoint-direct-source-duration-paired-command-feasibility";
pub const WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_SCHEMA_ID: &str =
    "waypoint_direct_source_duration_paired_command_feasibility_v1";
pub const WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_SCHEMA_VERSION: u32 = 1;

const EXPECTED_SOURCE_DURATION_SHA256: &str =
    "a93b728374145d4f12b43b21b7d68e383270db19facdf5964ce91bc47e557e9c";
const EXPECTED_SOURCE_DURATION_IDENTITY: &str = "fnv1a64:dfe0f0feaa15fc24";
const EXPECTED_SOURCE_ROWS: usize = 15;
const EXPECTED_ANALYTICAL_SKIPS: usize = 6;
const EXPECTED_SURVIVORS: usize = 9;
const LAUNCH_TICKS: u64 = 72;
const POSITION_TOLERANCE_M: f64 = 1.0e-6;
const VELOCITY_TOLERANCE_MPS: f64 = 1.0e-6;
const MAX_ITERATIONS: usize = 6;
const MAX_LINE_SEARCH_STEPS: usize = 8;
const FINITE_DIFFERENCE_STEP_MPS2: f64 = 1.0e-4;
const MAX_CORRECTION_MPS2: f64 = 0.25;
const INITIAL_DAMPING: f64 = 1.0e-3;
const SOLVER_VERSION: &str = "paired_mean_vector_damped_finite_difference_shooting_v1";

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectSourceDurationPairedCommandFeasibilityPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectSourceDurationPairedCommandFeasibilityRun {
    pub artifact: WaypointDirectSourceDurationPairedCommandFeasibilityArtifact,
    pub paths: WaypointDirectSourceDurationPairedCommandFeasibilityPaths,
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectSourceDurationPairedCommandFeasibilityValidation {
    pub source_duration_identity: String,
    pub expected_source_duration_sha256: String,
    pub input_gate: super::super::WaypointDirectCoupledThrustAuditInputGateEvidence,
    pub coupled_audit_identity: String,
    pub ordered_row_count: usize,
    pub analytical_skip_count: usize,
    pub analytical_survivor_count: usize,
    pub representative_rows: Vec<usize>,
    pub source_physics_ticks_per_controller_command: u64,
    pub solver_version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectSourceDurationPairedCommandFeasibilityArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub source_duration_identity: String,
    pub expected_source_duration_sha256: String,
    pub input_gate: super::super::WaypointDirectCoupledThrustAuditInputGateEvidence,
    pub coupled_audit_identity: String,
    pub protocol: PairedCommandFeasibilityProtocolEvidence,
    pub family_proof: PairedCommandFamilyProofEvidence,
    pub first_gate_passed: bool,
    pub rows: Vec<PairedCommandRowEvidence>,
    pub execution_status: String,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairedCommandFeasibilityProtocolEvidence {
    pub frozen_input_rule: String,
    pub row_rule: String,
    pub seed_rule: String,
    pub correction_rule: String,
    pub screen_rule: String,
    pub replay_rule: String,
    pub stopping_rule: String,
    pub non_claim: String,
    pub solver_version: String,
    pub maximum_iterations: usize,
    pub maximum_line_search_steps: usize,
    pub finite_difference_step_mps2: f64,
    pub maximum_parameter_abs_mps2: f64,
    pub position_tolerance_m: f64,
    pub velocity_tolerance_mps: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairedCommandFamilyProofEvidence {
    pub ordered_row_count: usize,
    pub recorded_row_count: usize,
    pub analytical_skip_count: usize,
    pub survivor_count: usize,
    pub baseline_pair_count: usize,
    pub representative_row_indices: Vec<usize>,
    pub first_gate_required_pass_count: usize,
    pub first_gate_observed_pass_count: usize,
    pub second_phase_survivor_count: usize,
    pub second_phase_completed_count: usize,
    pub omitted_row_count: usize,
    pub all_rows_recorded: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairedCommandRowEvidence {
    pub row_index: usize,
    pub candidate_identity: String,
    pub role: String,
    pub duration_offset_ticks: i64,
    pub source_bridge_tick_count: u64,
    pub analytical_survivor: bool,
    pub source_handoff_survivor: bool,
    /// Distinguishes the frozen analytical skips, representative first gate,
    /// conditional second phase, and rows deferred by the first-gate stop.
    pub experiment_phase: String,
    pub status: String,
    pub skip_reason: Option<String>,
    pub frozen_source_handoff: KinematicStateV2,
    pub baseline_pair: Option<BaselinePairEvidence>,
    pub paired_schedule: Option<PairedScheduleEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaselinePairEvidence {
    pub direct_120_hz: BaselineLaneEvidence,
    pub held_60_hz: BaselineLaneEvidence,
    pub both_match_frozen_canary: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaselineLaneEvidence {
    pub cadence: String,
    pub matched_frozen_canary: bool,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub source_handoff_contact_free: bool,
    pub first_contact: Option<FirstContactEvidence>,
    pub replay_trace: ReplayTraceParityEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairedScheduleEvidence {
    pub solver_status: String,
    pub witness: bool,
    pub correction: CorrectionEvidence,
    pub iterations: Vec<SolverIterationEvidence>,
    pub commands: Vec<HeldCommandEvidence>,
    pub endpoint_state: Option<EndpointStateEvidence>,
    pub signed_endpoint_residual: Option<SignedEndpointResidualEvidence>,
    pub position_error_m: Option<f64>,
    pub velocity_error_mps: Option<f64>,
    pub screens: ScheduleScreenEvidence,
    pub scheduled_source_prefix_parity: SourcePrefixParityEvidence,
    pub full_flight_status: String,
    pub full_flight_first_contact: Option<FirstContactEvidence>,
    pub full_flight_first_contact_margins: Option<ScheduledFirstContactMarginsEvidence>,
    pub full_flight_replay_trace: Option<ReplayTraceParityEvidence>,
    pub full_flight_rollout: Option<super::super::LaunchRolloutEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScheduledFirstContactMarginsEvidence {
    pub no_contact_minimum_foot_clearance_m: f64,
    pub no_contact_minimum_hull_clearance_m: f64,
    pub stable_minimum_clearance_margin_m: f64,
    pub stable_maximum_clearance_margin_m: f64,
    pub stable_hull_penetration_margin_m: f64,
    pub safe_normal_speed_margin_mps: f64,
    pub safe_tangential_speed_margin_mps: f64,
    pub safe_attitude_margin_rad: f64,
    pub safe_angular_rate_margin_radps: f64,
    pub touchdown_pad_left_margin_m: f64,
    pub touchdown_pad_right_margin_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourcePrefixParityEvidence {
    pub source_physics_tick_count: u64,
    pub source_gate_rollout_tick_count: usize,
    pub full_flight_rollout_tick_count: usize,
    pub rollout_prefix_matches: bool,
    pub state_sample_prefix_matches: bool,
    pub command_sample_prefix_matches: bool,
    pub source_handoff_endpoint_matches: bool,
    pub source_contacts_match: bool,
    pub launch_and_bridge_match: bool,
    pub passed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorrectionEvidence {
    pub constant_x_mps2: f64,
    pub constant_y_mps2: f64,
    pub linear_x_mps2: f64,
    pub linear_y_mps2: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SolverIterationEvidence {
    pub iteration: usize,
    pub correction: CorrectionEvidence,
    pub objective_m: Option<f64>,
    pub position_error_m: Option<f64>,
    pub velocity_error_mps: Option<f64>,
    pub screens_passed: bool,
    pub finite_difference_columns: usize,
    pub accepted_line_search_scale: Option<f64>,
    pub line_search_trials: Vec<LineSearchTrialEvidence>,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineSearchTrialEvidence {
    pub scale: f64,
    pub correction: CorrectionEvidence,
    pub objective_m: Option<f64>,
    pub screens_passed: bool,
    pub accepted: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCommandEvidence {
    pub pair_index: usize,
    pub is_first_powered_pair: bool,
    /// First completed physics step that used this held command (post-step label).
    pub first_physics_step: u64,
    /// Second completed physics step that used this held command (post-step label).
    pub last_physics_step: u64,
    pub desired_thrust_acceleration_mps2: Vec2,
    pub target_attitude_rad: f64,
    pub commanded_throttle_frac: Option<f64>,
    pub applied_throttle_fraction_first_tick: Option<f64>,
    pub applied_throttle_fraction_second_tick: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EndpointStateEvidence {
    pub physics_step: u64,
    pub sim_time_s: f64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignedEndpointResidualEvidence {
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScheduleScreenEvidence {
    pub source_handoff_reached: bool,
    pub source_contact_free: bool,
    pub coupled_thrust_passed: bool,
    pub minimum_throttle_passed: bool,
    pub powered_slew_passed: bool,
    pub source_attitude_passed: bool,
    pub source_attitude: SourceAttitudeScreenEvidence,
    pub launch_boundary_passed: bool,
    pub scheduled_launch_boundary: ScheduledLaunchBoundaryEvidence,
    pub source_clearance: SourceClearanceScreenEvidence,
    pub aggregate_fuel_passed: bool,
    pub aggregate_time_passed: bool,
    pub strict_position_passed: bool,
    pub strict_velocity_passed: bool,
    pub passed: bool,
    pub maximum_requested_thrust_acceleration_mps2: f64,
    pub maximum_powered_slew_radps: f64,
    /// Minimum `desired_applied_throttle_frac` after requested-to-throttle
    /// mapping; distinct from raw command fraction and plant-applied fraction.
    pub minimum_desired_applied_throttle_fraction: Option<f64>,
    pub aggregate_fuel_burn_kg: Option<f64>,
    pub aggregate_mission_time_s: f64,
    pub authoritative_source_contacts: Vec<ContactEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceAttitudeScreenEvidence {
    pub first_powered_scheduled_attitude_rad: Option<f64>,
    pub first_powered_attitude_margin: Option<MarginEvidence>,
    /// Descriptive plant state after the first physics tick using the first
    /// powered held pair; the gate preserves the original first-target margin.
    pub first_powered_first_tick_actual_attitude_rad: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScheduledLaunchBoundaryEvidence {
    pub launch_end_attitude_rad: Option<f64>,
    pub frozen_launch_target_attitude_rad: f64,
    pub reference_first_powered_attitude_rad: Option<f64>,
    pub scheduled_first_powered_attitude_rad: Option<f64>,
    pub first_powered_direction_preserved: bool,
    pub attitude_delta_rad: Option<f64>,
    pub required_one_tick_slew_rate_radps: Option<f64>,
    pub maximum_rotation_rate_radps: f64,
    pub slew_margin: Option<MarginEvidence>,
    pub passed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::waypoint_direct_nominal_plant) struct Correction(
    pub(in crate::waypoint_direct_nominal_plant) [f64; 4],
);

#[derive(Clone, Copy, Debug)]
struct ScheduleResidual {
    signed: SignedEndpointResidualEvidence,
    scaled: [f64; 4],
    objective_m: f64,
    endpoint: EndpointStateEvidence,
}

#[derive(Clone, Debug)]
struct ScheduleEvaluation {
    run: SourceDurationDiagnosticCadenceRun,
    residual: Option<ScheduleResidual>,
    screens: ScheduleScreenEvidence,
}

struct ShootingContext<'a> {
    scenario: &'a ScenarioSpec,
    probe: &'a DirectBridgeProbeV2,
    policy: &'a DirectBridgePolicyV2,
    vehicle: &'a VehicleInputV2,
    candidate: &'a DirectBridgeCandidateV2,
    selected: &'a super::super::PreparedProfileCandidate,
    source_bridge_tick_count: u64,
    handoff: KinematicStateV2,
    launch_target: f64,
    seed: &'a [HeldSourceCommand],
}

pub(in crate::waypoint_direct_nominal_plant) struct PhysicalScheduleScreenReplayRequest<'a> {
    pub scenario: &'a ScenarioSpec,
    pub probe: &'a DirectBridgeProbeV2,
    pub policy: &'a DirectBridgePolicyV2,
    pub vehicle: &'a VehicleInputV2,
    pub candidate: &'a DirectBridgeCandidateV2,
    pub selected: &'a super::super::PreparedProfileCandidate,
    pub source_bridge_tick_count: u64,
    pub launch_tilt_attitude_rad: f64,
    pub commands: &'a [HeldCommandEvidence],
}

#[derive(Clone, Debug)]
pub(in crate::waypoint_direct_nominal_plant) struct SourceScheduleIndependentReplayEvidence {
    pub launch: super::super::LaunchEvidence,
    pub reseeded_bridge: Option<super::super::ReseededBridgeEvidence>,
    pub source_rollout: super::super::LaunchRolloutEvidence,
    pub screens: ScheduleScreenEvidence,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub source_handoff_reached: bool,
    pub source_handoff_contact_free: bool,
    pub replay_trace: ReplayTraceParityEvidence,
}

struct IterationEvidenceInput {
    iteration: usize,
    correction: Correction,
    residual: Option<ScheduleResidual>,
    screens_passed: bool,
    finite_difference_columns: usize,
    accepted_line_search_scale: Option<f64>,
    line_search_trials: Vec<LineSearchTrialEvidence>,
    status: &'static str,
}

/// Rebuild and validate every sealed input and the 15-row family without
/// constructing a simulation state or advancing physics.
pub fn validate_waypoint_direct_source_duration_paired_command_feasibility_inputs(
    repo_root: &Path,
    input_paths: &WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths,
) -> Result<WaypointDirectSourceDurationPairedCommandFeasibilityValidation> {
    let prepared = prepare_held_cadence_inputs(repo_root, input_paths)?;
    Ok(validation_from_prepared_inputs(&prepared))
}

/// Run the predeclared representative gate, followed by the six remaining
/// analytical survivors only if all three representatives pass.
pub fn run_waypoint_direct_source_duration_paired_command_feasibility(
    repo_root: &Path,
    input_paths: &WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths,
    output_dir: &Path,
) -> Result<WaypointDirectSourceDurationPairedCommandFeasibilityRun> {
    let prepared = prepare_held_cadence_inputs(repo_root, input_paths)?;
    let validation = validation_from_prepared_inputs(&prepared);
    let output_dir = super::super::resolve_output_dir(repo_root, output_dir);
    let summary_path = output_dir.join("summary.json");
    if summary_path.exists() {
        bail!(
            "paired-command feasibility refuses to overwrite existing summary {}",
            summary_path.display()
        );
    }

    let mut rows = build_rows(&prepared)?;
    let mut held_baseline_runs = vec![None; EXPECTED_SOURCE_ROWS];
    run_and_bind_baselines(&prepared, &mut rows, &mut held_baseline_runs)?;

    let representative_rows = validation.representative_rows.clone();
    for row_index in &representative_rows {
        let (candidate_input, variant, _frozen_variant) = row_inputs(&prepared, *row_index)?;
        let held_baseline = held_baseline_runs[*row_index]
            .as_ref()
            .ok_or_else(|| anyhow!("representative row {row_index} has no held baseline"))?;
        let schedule = run_schedule_experiment(&prepared, candidate_input, variant, held_baseline)?;
        rows[*row_index].experiment_phase = "first_gate_representative".to_owned();
        rows[*row_index].status = format!("first_gate:{}", schedule.solver_status);
        rows[*row_index].paired_schedule = Some(schedule);
    }

    let first_gate_passed = representative_rows.iter().all(|row_index| {
        rows[*row_index]
            .paired_schedule
            .as_ref()
            .is_some_and(schedule_is_gate_witness)
    });
    if first_gate_passed {
        for row_index in 0..EXPECTED_SOURCE_ROWS {
            if !rows[row_index].analytical_survivor || representative_rows.contains(&row_index) {
                continue;
            }
            let (candidate_input, variant, _frozen_variant) = row_inputs(&prepared, row_index)?;
            let held_baseline = held_baseline_runs[row_index]
                .as_ref()
                .ok_or_else(|| anyhow!("survivor row {row_index} has no held baseline"))?;
            let schedule =
                run_schedule_experiment(&prepared, candidate_input, variant, held_baseline)?;
            rows[row_index].experiment_phase = "second_phase_survivor".to_owned();
            rows[row_index].status = format!("second_phase:{}", schedule.solver_status);
            rows[row_index].paired_schedule = Some(schedule);
        }
    } else {
        for row in &mut rows {
            if row.analytical_survivor && row.paired_schedule.is_none() {
                row.experiment_phase = "deferred_after_first_gate_failure".to_owned();
                row.status = "not_run_after_predeclared_first_gate_failure".to_owned();
                row.skip_reason = Some(
                    "one_or_more_-180_representatives_did_not_pass_strict_handoff_and_screens"
                        .to_owned(),
                );
            }
        }
    }

    if rows.len() != EXPECTED_SOURCE_ROWS
        || rows.iter().enumerate().any(|(i, row)| row.row_index != i)
    {
        bail!("paired-command family did not retain all fifteen ordered source rows");
    }
    let analytical_skip_count = rows.iter().filter(|row| !row.analytical_survivor).count();
    let survivor_count = rows.iter().filter(|row| row.analytical_survivor).count();
    let baseline_pair_count = rows
        .iter()
        .filter(|row| row.baseline_pair.is_some())
        .count();
    let second_phase_survivor_count = survivor_count - representative_rows.len();
    let second_phase_completed_count = rows
        .iter()
        .filter(|row| {
            row.analytical_survivor
                && !representative_rows.contains(&row.row_index)
                && row.paired_schedule.is_some()
        })
        .count();
    let artifact = WaypointDirectSourceDurationPairedCommandFeasibilityArtifact {
        schema_id: WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_ID
            .to_owned(),
        source_duration_identity: prepared.frozen.identity.clone(),
        expected_source_duration_sha256: EXPECTED_SOURCE_DURATION_SHA256.to_owned(),
        input_gate: prepared.input_gate.clone(),
        coupled_audit_identity: prepared.coupled_audit_identity.clone(),
        protocol: protocol_evidence(),
        family_proof: PairedCommandFamilyProofEvidence {
            ordered_row_count: EXPECTED_SOURCE_ROWS,
            recorded_row_count: rows.len(),
            analytical_skip_count,
            survivor_count,
            baseline_pair_count,
            representative_row_indices: representative_rows.clone(),
            first_gate_required_pass_count: representative_rows.len(),
            first_gate_observed_pass_count: representative_rows
                .iter()
                .filter(|row_index| {
                    rows[**row_index]
                        .paired_schedule
                        .as_ref()
                        .is_some_and(schedule_is_gate_witness)
                })
                .count(),
            second_phase_survivor_count,
            second_phase_completed_count,
            omitted_row_count: EXPECTED_SOURCE_ROWS - rows.len(),
            all_rows_recorded: rows.len() == EXPECTED_SOURCE_ROWS,
        },
        first_gate_passed,
        rows,
        execution_status: if first_gate_passed {
            "first_gate_passed_remaining_survivors_completed".to_owned()
        } else {
            "stopped_after_predeclared_first_gate_failure".to_owned()
        },
        scope_non_claims: vec![
            "A solver failure or budget exhaustion is not evidence that the frozen endpoint is unreachable.".to_owned(),
            "A passing source handoff is not held-out validation, V2 recertification, landing certification, or authority to change the planner, controller, simulator, policy, F6 path, or defaults.".to_owned(),
            "The V2 source-clearance mirror is recorded separately; authoritative source contact comes from the core simulation and paired replay.".to_owned(),
        ],
        identity: String::new(),
    };
    let mut artifact = artifact;
    artifact.identity = artifact_identity(&artifact)?;
    write_artifact(&output_dir, &summary_path, &artifact)?;
    Ok(WaypointDirectSourceDurationPairedCommandFeasibilityRun {
        artifact,
        paths: WaypointDirectSourceDurationPairedCommandFeasibilityPaths {
            output_dir,
            summary_path,
        },
    })
}

fn validation_from_prepared_inputs(
    prepared: &PreparedHeldCadenceInputs,
) -> WaypointDirectSourceDurationPairedCommandFeasibilityValidation {
    let held_validation = validation_from_prepared(prepared);
    WaypointDirectSourceDurationPairedCommandFeasibilityValidation {
        source_duration_identity: held_validation.source_duration_identity,
        expected_source_duration_sha256: EXPECTED_SOURCE_DURATION_SHA256.to_owned(),
        input_gate: prepared.input_gate.clone(),
        coupled_audit_identity: prepared.coupled_audit_identity.clone(),
        ordered_row_count: EXPECTED_SOURCE_ROWS,
        analytical_skip_count: EXPECTED_ANALYTICAL_SKIPS,
        analytical_survivor_count: EXPECTED_SURVIVORS,
        representative_rows: representative_row_indices(),
        source_physics_ticks_per_controller_command: 2,
        solver_version: SOLVER_VERSION.to_owned(),
    }
}

fn build_rows(prepared: &PreparedHeldCadenceInputs) -> Result<Vec<PairedCommandRowEvidence>> {
    let mut rows = Vec::with_capacity(EXPECTED_SOURCE_ROWS);
    for (candidate_index, (candidate_input, variants)) in prepared.variants.iter().enumerate() {
        let frozen_basis = prepared
            .frozen
            .bases
            .get(candidate_index)
            .ok_or_else(|| anyhow!("frozen basis {candidate_index} is missing"))?;
        for (duration_index, variant) in variants.iter().enumerate() {
            let frozen_variant = frozen_basis
                .duration_variants
                .get(duration_index)
                .ok_or_else(|| anyhow!("frozen row {duration_index} is missing"))?;
            let analytical_survivor =
                frozen_variant.analytical_survivor && frozen_variant.source_handoff_survivor;
            rows.push(PairedCommandRowEvidence {
                row_index: variant.row_index,
                candidate_identity: candidate_input.candidate.identity.clone(),
                role: frozen_basis.role.clone(),
                duration_offset_ticks: variant.duration_offset_ticks,
                source_bridge_tick_count: variant.source_bridge_tick_count,
                analytical_survivor,
                source_handoff_survivor: frozen_variant.source_handoff_survivor,
                experiment_phase: if analytical_survivor {
                    "pending_baseline_replay".to_owned()
                } else {
                    "analytical_skip".to_owned()
                },
                status: if analytical_survivor {
                    "pending_baseline_replay".to_owned()
                } else {
                    "analytical_screen_skip".to_owned()
                },
                skip_reason: if analytical_survivor {
                    None
                } else {
                    frozen_variant
                        .source_handoff_skip_reason
                        .clone()
                        .or_else(|| frozen_variant.analysis_error.clone())
                        .or_else(|| Some("frozen_analytical_screen_skip".to_owned()))
                },
                frozen_source_handoff: candidate_input
                    .candidate
                    .source_handoff
                    .ok_or_else(|| anyhow!("candidate has no frozen source handoff"))?
                    .state,
                baseline_pair: None,
                paired_schedule: None,
            });
        }
    }
    if rows.len() != EXPECTED_SOURCE_ROWS {
        bail!("reconstructed paired-command row family is not fifteen rows");
    }
    Ok(rows)
}

fn run_and_bind_baselines(
    prepared: &PreparedHeldCadenceInputs,
    rows: &mut [PairedCommandRowEvidence],
    held_baseline_runs: &mut [Option<RunWithSamples>],
) -> Result<()> {
    let context = RunContext::from_scenario(&prepared.validated.flat_case.scenario)
        .map_err(anyhow::Error::msg)?;
    for row_index in 0..EXPECTED_SOURCE_ROWS {
        if !rows[row_index].analytical_survivor {
            continue;
        }
        let (candidate_input, variant, frozen_variant) = row_inputs(prepared, row_index)?;
        let analysis = frozen_variant
            .launch_and_analytical_screen
            .as_ref()
            .ok_or_else(|| anyhow!("survivor row {row_index} lacks frozen launch analysis"))?;
        let mut lanes = Vec::with_capacity(2);
        let mut raw_held = None;
        for (cadence, cadence_label) in [
            (RolloutCadence::DirectPerTick, "direct_per_tick_120_hz"),
            (RolloutCadence::ControllerCadence, "held_controller_60_hz"),
        ] {
            let run = run_lane(
                candidate_input,
                &prepared.validated,
                variant.source_bridge_tick_count,
                variant
                    .launch_target_attitude_rad
                    .ok_or_else(|| anyhow!("survivor row {row_index} lacks launch target"))?,
                cadence,
                super::super::launch_feasibility::SourceDurationHoldMode::Together,
            )?;
            let replay = replay_logged_cadence(&context, &run.run)?;
            let frozen = frozen_flight(frozen_variant, cadence_label)?;
            let matches = baseline_matches_frozen(&run.run, &replay, analysis, frozen);
            if !matches || !replay.trace.passed {
                bail!(
                    "baseline lane {cadence_label} for row {row_index} no longer matches the frozen source-duration canary and paired replay"
                );
            }
            lanes.push(BaselineLaneEvidence {
                cadence: run.run.cadence.clone(),
                matched_frozen_canary: matches,
                source_handoff_position_error_m: run.run.rollout.source_handoff_position_error_m,
                source_handoff_velocity_error_mps: run
                    .run
                    .rollout
                    .source_handoff_velocity_error_mps,
                source_handoff_contact_free: run.run.rollout.source_handoff_contact_free,
                first_contact: replay.first_contact,
                replay_trace: replay.trace,
            });
            if cadence == RolloutCadence::ControllerCadence {
                raw_held = Some(run);
            }
        }
        let direct = lanes.remove(0);
        let held = lanes.remove(0);
        rows[row_index].status = "baseline_pair_replayed_and_matched_frozen_canary".to_owned();
        rows[row_index].baseline_pair = Some(BaselinePairEvidence {
            both_match_frozen_canary: direct.matched_frozen_canary && held.matched_frozen_canary,
            direct_120_hz: direct,
            held_60_hz: held,
        });
        held_baseline_runs[row_index] = raw_held;
    }
    Ok(())
}

fn row_inputs(
    prepared: &PreparedHeldCadenceInputs,
    row_index: usize,
) -> Result<(
    &super::super::flat_candidate_closure::CoupledThrustAuditCandidateInput,
    &PreparedDurationVariant,
    &SourceDurationVariantEvidence,
)> {
    let candidate_index = row_index / 5;
    let duration_index = row_index % 5;
    let (candidate_input, variants) = prepared
        .variants
        .get(candidate_index)
        .ok_or_else(|| anyhow!("candidate index for row {row_index} is missing"))?;
    let variant = variants
        .get(duration_index)
        .ok_or_else(|| anyhow!("duration row {row_index} is missing"))?;
    let frozen_variant = prepared
        .frozen
        .bases
        .get(candidate_index)
        .and_then(|basis| basis.duration_variants.get(duration_index))
        .ok_or_else(|| anyhow!("frozen duration row {row_index} is missing"))?;
    if variant.row_index != row_index || frozen_variant.row_index != row_index {
        bail!("paired-command source family ordering changed at row {row_index}");
    }
    Ok((candidate_input, variant, frozen_variant))
}

fn run_schedule_experiment(
    prepared: &PreparedHeldCadenceInputs,
    candidate_input: &super::super::flat_candidate_closure::CoupledThrustAuditCandidateInput,
    variant: &PreparedDurationVariant,
    held_baseline: &RunWithSamples,
) -> Result<PairedScheduleEvidence> {
    run_schedule_experiment_from_physical(
        &prepared.validated.flat_case.scenario,
        &prepared.validated.flat_case.probe,
        &prepared.validated.policy,
        &prepared.validated.vehicle,
        &candidate_input.candidate,
        &candidate_input.selected_profile,
        variant,
        held_baseline,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_schedule_experiment_from_physical(
    scenario: &ScenarioSpec,
    probe: &DirectBridgeProbeV2,
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    candidate: &DirectBridgeCandidateV2,
    selected: &super::super::PreparedProfileCandidate,
    variant: &PreparedDurationVariant,
    held_baseline: &RunWithSamples,
) -> Result<PairedScheduleEvidence> {
    let launch_target = variant
        .launch_target_attitude_rad
        .ok_or_else(|| anyhow!("row {} has no source launch target", variant.row_index))?;
    let launch_state = held_baseline.run.launch.end_state.as_ref().ok_or_else(|| {
        anyhow!(
            "row {} held baseline has no launch end state",
            variant.row_index
        )
    })?;
    let handoff = candidate
        .source_handoff
        .ok_or_else(|| anyhow!("row {} has no generated source handoff", variant.row_index))?;
    let reference = exact_discrete_bridge_v2(
        policy,
        vehicle,
        BridgeKindV2::Source,
        KinematicStateV2 {
            position_m: launch_state.position_m,
            velocity_mps: launch_state.velocity_mps,
        },
        handoff.state,
        variant.source_bridge_tick_count,
    )
    .map_err(|error| {
        anyhow!(
            "row {} held-launch source reference bridge failed: {error}",
            variant.row_index
        )
    })?;
    if held_baseline
        .run
        .reseeded_bridge
        .as_ref()
        .and_then(|bridge| bridge.identity.as_deref())
        != Some(reference.identity.as_str())
    {
        bail!(
            "row {} source reference bridge differs from its held-60 baseline",
            variant.row_index
        );
    }
    let seed = paired_mean_seed(&reference, launch_target)?;
    let shooting_context = ShootingContext {
        scenario,
        probe,
        policy,
        vehicle,
        candidate,
        selected,
        source_bridge_tick_count: variant.source_bridge_tick_count,
        handoff: handoff.state,
        launch_target,
        seed: &seed,
    };
    let (best_correction, best_evaluation, iterations, status) = fit_schedule(&shooting_context)?;
    let schedule = build_schedule(&reference, launch_target, best_correction)?;
    let context = RunContext::from_scenario(scenario).map_err(anyhow::Error::msg)?;
    let full = run_source_duration_variant_with_held_schedule(
        SourceDurationRunRequest {
            scenario,
            probe,
            selected,
            basis: candidate,
            policy,
            vehicle,
            source_bridge_steps: variant.source_bridge_tick_count,
            launch_tilt_attitude_rad: launch_target,
            cadence: RolloutCadence::ControllerCadence,
            stage: SourceDurationReplayStage::FullFlight,
        },
        &schedule,
    )?;
    if full.run.launch != held_baseline.run.launch
        || full.run.reseeded_bridge != held_baseline.run.reseeded_bridge
    {
        bail!(
            "row {} scheduled full flight changed the frozen held-60 launch or bridge",
            variant.row_index
        );
    }
    let scheduled_source_prefix_parity = compare_scheduled_source_prefix(
        &best_evaluation.run,
        &full,
        variant.source_bridge_tick_count,
    );
    let replay = replay_logged_cadence(&context, &full.run)?;
    let replay_passed = replay.trace.passed;
    let full_rollout = full.run.rollout.clone();
    let first_contact = replay.first_contact;
    let first_contact_margins = replay
        .first_contact_margins
        .as_ref()
        .map(first_contact_margin_evidence);
    let schedule_commands = held_command_evidence(&schedule, &best_evaluation.run.command_samples);
    let mut final_status = status;
    let endpoint_screen_witness = is_schedule_witness(&best_evaluation);
    let witness = endpoint_screen_witness && scheduled_source_prefix_parity.passed && replay_passed;
    if endpoint_screen_witness && !scheduled_source_prefix_parity.passed {
        final_status = "scheduled_source_prefix_mismatch_after_endpoint_witness".to_owned();
    } else if endpoint_screen_witness && !replay_passed {
        final_status = "replay_parity_failed_after_endpoint_witness".to_owned();
    } else if witness {
        final_status = "strict_handoff_witness_with_screens_and_replay".to_owned();
    } else if !replay_passed {
        final_status = "no_strict_handoff_witness_replay_parity_failed".to_owned();
    }
    Ok(PairedScheduleEvidence {
        solver_status: final_status,
        witness,
        correction: correction_evidence(best_correction),
        iterations,
        commands: schedule_commands,
        endpoint_state: best_evaluation.residual.map(|residual| residual.endpoint),
        signed_endpoint_residual: best_evaluation.residual.map(|residual| residual.signed),
        position_error_m: best_evaluation
            .residual
            .map(|residual| residual.signed.position_m.length()),
        velocity_error_mps: best_evaluation
            .residual
            .map(|residual| residual.signed.velocity_mps.length()),
        screens: best_evaluation.screens,
        scheduled_source_prefix_parity,
        full_flight_status: full_rollout.status.clone(),
        full_flight_first_contact: first_contact,
        full_flight_first_contact_margins: first_contact_margins,
        full_flight_replay_trace: Some(replay.trace),
        full_flight_rollout: Some(full_rollout),
    })
}

pub(in crate::waypoint_direct_nominal_plant) fn replay_source_schedule_screens_from_physical(
    request: &PhysicalScheduleScreenReplayRequest<'_>,
) -> Result<SourceScheduleIndependentReplayEvidence> {
    let source_ticks = request.source_bridge_tick_count;
    let interval_steps = request.scenario.sim.control_interval_steps();
    if source_ticks == 0
        || interval_steps != 2
        || !source_ticks.is_multiple_of(interval_steps)
        || request.commands.len() != (source_ticks / interval_steps) as usize
        || !request.launch_tilt_attitude_rad.is_finite()
    {
        bail!("independent source replay received an invalid cadence or schedule length");
    }
    let mut held_commands = Vec::with_capacity(request.commands.len());
    for (pair_index, command) in request.commands.iter().enumerate() {
        let expected_first = LAUNCH_TICKS + 1 + pair_index as u64 * interval_steps;
        if command.pair_index != pair_index
            || command.first_physics_step != expected_first
            || command.last_physics_step != expected_first + interval_steps - 1
            || !command.desired_thrust_acceleration_mps2.x.is_finite()
            || !command.desired_thrust_acceleration_mps2.y.is_finite()
            || !command.target_attitude_rad.is_finite()
        {
            bail!("independent source replay command provenance changed at pair {pair_index}");
        }
        held_commands.push(HeldSourceCommand {
            thrust_acceleration_mps2: command.desired_thrust_acceleration_mps2,
            target_attitude_rad: command.target_attitude_rad,
        });
    }
    let handoff = request
        .candidate
        .source_handoff
        .ok_or_else(|| anyhow!("independent source replay basis has no source handoff"))?;
    let launch_target = request.launch_tilt_attitude_rad;
    let context = ShootingContext {
        scenario: request.scenario,
        probe: request.probe,
        policy: request.policy,
        vehicle: request.vehicle,
        candidate: request.candidate,
        selected: request.selected,
        source_bridge_tick_count: source_ticks,
        handoff: handoff.state,
        launch_target,
        seed: &held_commands,
    };
    let source = run_source_duration_variant_with_held_schedule(
        SourceDurationRunRequest {
            scenario: request.scenario,
            probe: request.probe,
            selected: request.selected,
            basis: request.candidate,
            policy: request.policy,
            vehicle: request.vehicle,
            source_bridge_steps: source_ticks,
            launch_tilt_attitude_rad: launch_target,
            cadence: RolloutCadence::ControllerCadence,
            stage: SourceDurationReplayStage::SourceHandoff,
        },
        &held_commands,
    )?;
    let screens = schedule_screens(&source, &held_commands, &context);
    let replay_context = RunContext::from_scenario(request.scenario).map_err(anyhow::Error::msg)?;
    let replay_trace = replay_logged_cadence(&replay_context, &source.run)?.trace;
    Ok(SourceScheduleIndependentReplayEvidence {
        launch: source.run.launch.clone(),
        reseeded_bridge: source.run.reseeded_bridge.clone(),
        source_rollout: source.run.rollout.clone(),
        screens,
        source_handoff_position_error_m: source.run.rollout.source_handoff_position_error_m,
        source_handoff_velocity_error_mps: source.run.rollout.source_handoff_velocity_error_mps,
        source_handoff_reached: source.run.rollout.source_handoff_reached,
        source_handoff_contact_free: source.run.rollout.source_handoff_contact_free,
        replay_trace,
    })
}

fn compare_scheduled_source_prefix(
    source_gate: &SourceDurationDiagnosticCadenceRun,
    full_flight: &SourceDurationDiagnosticCadenceRun,
    source_steps: u64,
) -> SourcePrefixParityEvidence {
    let source_gate_tick_count = source_gate.run.rollout.per_step.len();
    let full_flight_tick_count = full_flight.run.rollout.per_step.len();
    let rollout_prefix_matches = source_gate_tick_count > 0
        && full_flight
            .run
            .rollout
            .per_step
            .get(..source_gate_tick_count)
            == Some(source_gate.run.rollout.per_step.as_slice());
    let state_sample_prefix_matches = !source_gate.state_samples.is_empty()
        && full_flight
            .state_samples
            .get(..source_gate.state_samples.len())
            == Some(source_gate.state_samples.as_slice());
    let command_sample_prefix_matches = !source_gate.command_samples.is_empty()
        && full_flight
            .command_samples
            .get(..source_gate.command_samples.len())
            == Some(source_gate.command_samples.as_slice());
    let endpoint_step = LAUNCH_TICKS + source_steps;
    let source_endpoint = source_gate
        .state_samples
        .iter()
        .find(|sample| sample.physics_step == endpoint_step && sample.phase == "source_bridge");
    let full_endpoint = full_flight
        .state_samples
        .iter()
        .find(|sample| sample.physics_step == endpoint_step && sample.phase == "source_bridge");
    let source_handoff_endpoint_matches = source_endpoint.is_some()
        && source_endpoint == full_endpoint
        && source_gate.run.rollout.source_handoff_reached
            == full_flight.run.rollout.source_handoff_reached
        && source_gate.run.rollout.source_handoff_position_error_m
            == full_flight.run.rollout.source_handoff_position_error_m
        && source_gate.run.rollout.source_handoff_velocity_error_mps
            == full_flight.run.rollout.source_handoff_velocity_error_mps;
    let source_contacts = |run: &LaunchFeasibilityCadenceRunEvidence| {
        run.rollout
            .contacts
            .iter()
            .filter(|contact| {
                contact.physics_step > LAUNCH_TICKS && contact.physics_step <= endpoint_step
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    let source_contacts_match =
        source_contacts(&source_gate.run) == source_contacts(&full_flight.run);
    let launch_and_bridge_match = source_gate.run.launch == full_flight.run.launch
        && source_gate.run.reseeded_bridge == full_flight.run.reseeded_bridge;
    let passed = rollout_prefix_matches
        && state_sample_prefix_matches
        && command_sample_prefix_matches
        && source_handoff_endpoint_matches
        && source_contacts_match
        && launch_and_bridge_match;
    SourcePrefixParityEvidence {
        source_physics_tick_count: source_steps,
        source_gate_rollout_tick_count: source_gate_tick_count,
        full_flight_rollout_tick_count: full_flight_tick_count,
        rollout_prefix_matches,
        state_sample_prefix_matches,
        command_sample_prefix_matches,
        source_handoff_endpoint_matches,
        source_contacts_match,
        launch_and_bridge_match,
        passed,
    }
}

fn first_contact_margin_evidence(
    margins: &FirstContactPredicateMarginsEvidence,
) -> ScheduledFirstContactMarginsEvidence {
    ScheduledFirstContactMarginsEvidence {
        no_contact_minimum_foot_clearance_m: margins.no_contact_minimum_foot_clearance_m,
        no_contact_minimum_hull_clearance_m: margins.no_contact_minimum_hull_clearance_m,
        stable_minimum_clearance_margin_m: margins.stable_minimum_clearance_margin_m,
        stable_maximum_clearance_margin_m: margins.stable_maximum_clearance_margin_m,
        stable_hull_penetration_margin_m: margins.stable_hull_penetration_margin_m,
        safe_normal_speed_margin_mps: margins.safe_normal_speed_margin_mps,
        safe_tangential_speed_margin_mps: margins.safe_tangential_speed_margin_mps,
        safe_attitude_margin_rad: margins.safe_attitude_margin_rad,
        safe_angular_rate_margin_radps: margins.safe_angular_rate_margin_radps,
        touchdown_pad_left_margin_m: margins.touchdown_pad_left_margin_m,
        touchdown_pad_right_margin_m: margins.touchdown_pad_right_margin_m,
    }
}

fn fit_schedule(
    context: &ShootingContext<'_>,
) -> Result<(
    Correction,
    ScheduleEvaluation,
    Vec<SolverIterationEvidence>,
    String,
)> {
    let mut correction = Correction::default();
    let mut current = evaluate_correction(context, correction)?;
    let mut best_correction = correction;
    let mut best = current.clone();
    let mut damping = INITIAL_DAMPING;
    let mut history = Vec::with_capacity(MAX_ITERATIONS);
    let mut status = "no_strict_handoff_witness_within_iteration_budget".to_owned();

    for iteration in 0..MAX_ITERATIONS {
        if is_schedule_witness(&current) {
            best_correction = correction;
            best = current;
            status = "strict_handoff_witness_found".to_owned();
            break;
        }
        if better_evaluation(&current, &best) {
            best_correction = correction;
            best = current.clone();
        }
        let Some(base_residual) = current.residual else {
            history.push(SolverIterationEvidence {
                iteration,
                correction: correction_evidence(correction),
                objective_m: None,
                position_error_m: current.run.run.rollout.source_handoff_position_error_m,
                velocity_error_mps: current.run.run.rollout.source_handoff_velocity_error_mps,
                screens_passed: current.screens.passed,
                finite_difference_columns: 0,
                accepted_line_search_scale: None,
                line_search_trials: Vec::new(),
                status: "source_handoff_not_reached_for_shooting".to_owned(),
            });
            break;
        };

        let mut jacobian = [[0.0_f64; 4]; 4];
        let mut columns_completed = 0;
        for (parameter, base_value) in correction.0.iter().copied().enumerate() {
            let mut plus = correction;
            plus.0[parameter] = base_value + FINITE_DIFFERENCE_STEP_MPS2;
            let mut minus = correction;
            minus.0[parameter] = base_value - FINITE_DIFFERENCE_STEP_MPS2;
            plus = clamp_correction(plus);
            minus = clamp_correction(minus);
            let plus_eval = evaluate_correction(context, plus)?;
            let minus_eval = evaluate_correction(context, minus)?;
            let (Some(plus_residual), Some(minus_residual)) =
                (plus_eval.residual, minus_eval.residual)
            else {
                continue;
            };
            let denominator = plus.0[parameter] - minus.0[parameter];
            if denominator.abs() <= f64::EPSILON {
                continue;
            }
            for ((row, plus_output), minus_output) in jacobian
                .iter_mut()
                .zip(plus_residual.scaled)
                .zip(minus_residual.scaled)
            {
                row[parameter] = (plus_output - minus_output) / denominator;
            }
            columns_completed += 1;
        }
        if columns_completed != 4 {
            history.push(iteration_evidence(IterationEvidenceInput {
                iteration,
                correction,
                residual: Some(base_residual),
                screens_passed: current.screens.passed,
                finite_difference_columns: columns_completed,
                accepted_line_search_scale: None,
                line_search_trials: Vec::new(),
                status: "finite_difference_jacobian_incomplete",
            }));
            break;
        }
        let Some(delta) = damped_least_squares_step(jacobian, base_residual.scaled, damping) else {
            history.push(iteration_evidence(IterationEvidenceInput {
                iteration,
                correction,
                residual: Some(base_residual),
                screens_passed: current.screens.passed,
                finite_difference_columns: columns_completed,
                accepted_line_search_scale: None,
                line_search_trials: Vec::new(),
                status: "finite_difference_system_singular",
            }));
            break;
        };
        let mut line_trials = Vec::with_capacity(MAX_LINE_SEARCH_STEPS);
        let mut accepted = None;
        for line_search_index in 0..MAX_LINE_SEARCH_STEPS {
            let scale = 0.5_f64.powi(line_search_index as i32);
            let candidate = clamp_correction(Correction(std::array::from_fn(|index| {
                correction.0[index] + scale * delta[index]
            })));
            let evaluation = evaluate_correction(context, candidate)?;
            let objective = evaluation.residual.map(|residual| residual.objective_m);
            let improves = evaluation.screens.passed
                && objective.is_some_and(|objective| {
                    !current.screens.passed
                        || current
                            .residual
                            .is_some_and(|residual| objective < residual.objective_m)
                });
            line_trials.push(LineSearchTrialEvidence {
                scale,
                correction: correction_evidence(candidate),
                objective_m: objective,
                screens_passed: evaluation.screens.passed,
                accepted: improves,
            });
            if better_evaluation(&evaluation, &best) {
                best_correction = candidate;
                best = evaluation.clone();
            }
            if improves {
                accepted = Some((candidate, evaluation, scale));
                break;
            }
        }
        if let Some((next_correction, next, scale)) = accepted {
            history.push(iteration_evidence(IterationEvidenceInput {
                iteration,
                correction,
                residual: Some(base_residual),
                screens_passed: current.screens.passed,
                finite_difference_columns: columns_completed,
                accepted_line_search_scale: Some(scale),
                line_search_trials: line_trials,
                status: "accepted_feasible_descent_step",
            }));
            correction = next_correction;
            current = next;
            damping = (damping * 0.3).max(1.0e-9);
            if is_schedule_witness(&current) {
                best_correction = correction;
                best = current.clone();
                status = "strict_handoff_witness_found".to_owned();
                break;
            }
        } else {
            history.push(iteration_evidence(IterationEvidenceInput {
                iteration,
                correction,
                residual: Some(base_residual),
                screens_passed: current.screens.passed,
                finite_difference_columns: columns_completed,
                accepted_line_search_scale: None,
                line_search_trials: line_trials,
                status: "line_search_exhausted",
            }));
            damping = (damping * 10.0).min(1.0e8);
        }
    }
    if best.screens.passed
        && best.residual.is_some_and(|residual| {
            residual.signed.position_m.length() <= POSITION_TOLERANCE_M
                && residual.signed.velocity_mps.length() <= VELOCITY_TOLERANCE_MPS
        })
    {
        status = "strict_handoff_witness_found".to_owned();
    } else if !best.screens.passed {
        status = "no_safe_schedule_witness_within_iteration_budget".to_owned();
    }
    Ok((best_correction, best, history, status))
}

fn evaluate_correction(
    context: &ShootingContext<'_>,
    correction: Correction,
) -> Result<ScheduleEvaluation> {
    let schedule = apply_correction(context.seed, correction);
    let launch_target = context.launch_target;
    let run = run_source_duration_variant_with_held_schedule(
        SourceDurationRunRequest {
            scenario: context.scenario,
            probe: context.probe,
            selected: context.selected,
            basis: context.candidate,
            policy: context.policy,
            vehicle: context.vehicle,
            source_bridge_steps: context.source_bridge_tick_count,
            launch_tilt_attitude_rad: launch_target,
            cadence: RolloutCadence::ControllerCadence,
            stage: SourceDurationReplayStage::SourceHandoff,
        },
        &schedule,
    )?;
    let residual = endpoint_residual(&run, context.handoff, context.source_bridge_tick_count);
    let screens = schedule_screens(&run, &schedule, context);
    Ok(ScheduleEvaluation {
        run,
        residual,
        screens,
    })
}

fn endpoint_residual(
    run: &SourceDurationDiagnosticCadenceRun,
    target: KinematicStateV2,
    source_steps: u64,
) -> Option<ScheduleResidual> {
    let endpoint_step = LAUNCH_TICKS.checked_add(source_steps)?;
    let sample = run
        .state_samples
        .iter()
        .find(|sample| sample.physics_step == endpoint_step && sample.phase == "source_bridge")?;
    let position = sample.position_m - target.position_m;
    let velocity = sample.velocity_mps - target.velocity_mps;
    let duration_s = source_steps as f64 / 120.0;
    let scaled = [
        position.x,
        position.y,
        velocity.x * duration_s,
        velocity.y * duration_s,
    ];
    let objective_m = scaled.iter().map(|value| value * value).sum::<f64>().sqrt();
    Some(ScheduleResidual {
        signed: SignedEndpointResidualEvidence {
            position_m: position,
            velocity_mps: velocity,
        },
        scaled,
        objective_m,
        endpoint: EndpointStateEvidence {
            physics_step: sample.physics_step,
            sim_time_s: sample.physics_step as f64 / 120.0,
            position_m: sample.position_m,
            velocity_mps: sample.velocity_mps,
            attitude_rad: sample.attitude_rad,
            angular_rate_radps: sample.angular_rate_radps,
            fuel_kg: sample.fuel_kg,
        },
    })
}

fn schedule_screens(
    run: &SourceDurationDiagnosticCadenceRun,
    schedule: &[HeldSourceCommand],
    context: &ShootingContext<'_>,
) -> ScheduleScreenEvidence {
    let policy = context.policy;
    let vehicle = context.vehicle;
    let probe = context.probe;
    let candidate = context.candidate;
    let launch_target = context.launch_target;
    let source_steps = context.source_bridge_tick_count;
    let max_requested = schedule
        .iter()
        .map(|command| command.thrust_acceleration_mps2.length())
        .fold(0.0_f64, f64::max);
    let derated_limit =
        policy.thrust_derate * vehicle.max_thrust_n / (vehicle.dry_mass_kg + vehicle.max_fuel_kg);
    let coupled_limit = derated_limit * (1.0 - policy.declared_robustness_margin);
    let coupled_thrust_passed = max_requested <= coupled_limit + 1.0e-12;
    let minimum_throttle = run
        .command_samples
        .iter()
        .filter(|sample| sample.phase == "source_bridge" && sample.throttle_update_due)
        .filter(|sample| sample.desired_applied_throttle_frac > 0.0)
        .map(|sample| sample.desired_applied_throttle_frac)
        .reduce(f64::min);
    let minimum_throttle_passed = minimum_throttle
        .is_some_and(|minimum| minimum + 1.0e-12 >= vehicle.min_throttle_frac)
        && run.run.rollout.saturation.below_minimum_saturation_count == 0
        && run.run.rollout.saturation.above_maximum_saturation_count == 0;
    let maximum_slew = maximum_powered_slew(schedule);
    let powered_slew_passed = maximum_slew
        <= vehicle.max_rotation_rate_radps * (1.0 - policy.declared_robustness_margin) + 1.0e-12;
    let first_powered_pair_index = schedule
        .iter()
        .position(|command| command.thrust_acceleration_mps2.length() > 1.0e-12);
    let first_powered_scheduled_attitude =
        first_powered_pair_index.map(|index| schedule[index].target_attitude_rad);
    let first_powered_attitude_margin = first_powered_scheduled_attitude.map(|attitude| {
        let raw = vehicle.safe_touchdown_attitude_error_rad - attitude.abs();
        let normalized = raw
            / vehicle
                .safe_touchdown_attitude_error_rad
                .abs()
                .max(f64::EPSILON);
        MarginEvidence {
            raw_margin: raw,
            normalized_margin: normalized,
            passes_declared_screen: raw >= 0.0
                && normalized + 1.0e-12 >= policy.declared_robustness_margin,
        }
    });
    let first_powered_first_tick_actual_attitude = first_powered_pair_index.and_then(|index| {
        let physics_step = LAUNCH_TICKS + 1 + (index as u64 * 2);
        run.state_samples
            .iter()
            .find(|sample| sample.phase == "source_bridge" && sample.physics_step == physics_step)
            .map(|sample| sample.attitude_rad)
    });
    let source_attitude_passed =
        first_powered_attitude_margin.is_some_and(|margin| margin.passes_declared_screen);
    let source_attitude = SourceAttitudeScreenEvidence {
        first_powered_scheduled_attitude_rad: first_powered_scheduled_attitude,
        first_powered_attitude_margin,
        first_powered_first_tick_actual_attitude_rad: first_powered_first_tick_actual_attitude,
    };
    let launch_end_attitude = run
        .run
        .launch
        .end_state
        .as_ref()
        .map(|state| state.attitude_rad);
    let reference_first_powered_attitude = run
        .run
        .reseeded_bridge
        .as_ref()
        .and_then(|bridge| bridge.launch_boundary.as_ref())
        .and_then(|boundary| boundary.first_reseeded_powered_attitude_rad);
    let scheduled_first_powered_attitude = schedule
        .iter()
        .find(|command| command.thrust_acceleration_mps2.length() > 1.0e-12)
        .map(|command| command.target_attitude_rad);
    let first_powered_direction_preserved = scheduled_first_powered_attitude
        .zip(reference_first_powered_attitude)
        .is_some_and(|(scheduled, reference)| {
            shortest_angle_delta(reference, scheduled).abs() <= 1.0e-12
        });
    let attitude_delta = launch_end_attitude
        .zip(scheduled_first_powered_attitude)
        .map(|(launch, first_powered)| shortest_angle_delta(launch, first_powered).abs());
    let required_one_tick_slew_rate =
        attitude_delta.map(|delta| delta * f64::from(policy.physics_hz));
    let launch_slew_margin = required_one_tick_slew_rate.map(|required| {
        let raw = vehicle.max_rotation_rate_radps - required;
        MarginEvidence {
            raw_margin: raw,
            normalized_margin: raw / vehicle.max_rotation_rate_radps.abs().max(f64::EPSILON),
            passes_declared_screen: raw >= 0.0
                && raw / vehicle.max_rotation_rate_radps.abs().max(f64::EPSILON) + 1.0e-12
                    >= policy.declared_robustness_margin,
        }
    });
    let launch_boundary_passed = first_powered_direction_preserved
        && launch_slew_margin.is_some_and(|margin| margin.passes_declared_screen);
    let scheduled_launch_boundary = ScheduledLaunchBoundaryEvidence {
        launch_end_attitude_rad: launch_end_attitude,
        frozen_launch_target_attitude_rad: launch_target,
        reference_first_powered_attitude_rad: reference_first_powered_attitude,
        scheduled_first_powered_attitude_rad: scheduled_first_powered_attitude,
        first_powered_direction_preserved,
        attitude_delta_rad: attitude_delta,
        required_one_tick_slew_rate_radps: required_one_tick_slew_rate,
        maximum_rotation_rate_radps: vehicle.max_rotation_rate_radps,
        slew_margin: launch_slew_margin,
        passed: launch_boundary_passed,
    };

    let source_samples = run
        .state_samples
        .iter()
        .filter(|sample| sample.phase == "source_bridge")
        .collect::<Vec<_>>();
    let clearance_screens = source_samples
        .iter()
        .map(|sample| {
            let direction = Vec2::new(sample.attitude_rad.sin(), sample.attitude_rad.cos());
            mirror_v2_clearance(
                KinematicStateV2 {
                    position_m: sample.position_m,
                    velocity_mps: sample.velocity_mps,
                },
                direction,
                &probe.source,
                vehicle,
                policy,
                &context.scenario.world.terrain,
            )
        })
        .collect::<Vec<_>>();
    let minimum_margin =
        clearance_screens
            .iter()
            .map(|screen| screen.margin)
            .reduce(|left, right| {
                if left.normalized <= right.normalized {
                    left
                } else {
                    right
                }
            });
    let source_clearance = SourceClearanceScreenEvidence {
        evaluated_tick_count: clearance_screens.len() as u64,
        source_pad_center_height_tick_count: clearance_screens
            .iter()
            .filter(|screen| screen.mode.as_str() == "source_pad_center_height")
            .count() as u64,
        rotated_hull_clearance_tick_count: clearance_screens
            .iter()
            .filter(|screen| screen.mode.as_str() == "rotated_hull_clearance")
            .count() as u64,
        minimum_margin: minimum_margin.map_or(
            super::super::MarginEvidence {
                raw_margin: f64::NEG_INFINITY,
                normalized_margin: f64::NEG_INFINITY,
                passes_declared_screen: false,
            },
            |margin| super::super::MarginEvidence {
                raw_margin: margin.raw,
                normalized_margin: margin.normalized,
                passes_declared_screen: margin.normalized + 1.0e-12
                    >= policy.declared_robustness_margin
                    && margin.raw >= 0.0,
            },
        ),
        all_samples_passed: !clearance_screens.is_empty()
            && clearance_screens
                .iter()
                .all(|screen| screen.passes_declared_screen),
    };
    let source_contacts = run
        .run
        .rollout
        .contacts
        .iter()
        .filter(|contact| {
            contact.physics_step > LAUNCH_TICKS
                && contact.physics_step <= LAUNCH_TICKS + source_steps
        })
        .cloned()
        .collect::<Vec<_>>();
    let source_contact_free = run.run.launch.contact_free
        && source_contacts.is_empty()
        && run
            .run
            .rollout
            .per_step
            .iter()
            .filter(|tick| tick.phase == "source_bridge")
            .all(|tick| tick.contact_classification == "none");
    let source_handoff_reached = run.run.rollout.source_handoff_reached;
    let strict_position_passed = run
        .run
        .rollout
        .source_handoff_position_error_m
        .is_some_and(|error| error <= POSITION_TOLERANCE_M);
    let strict_velocity_passed = run
        .run
        .rollout
        .source_handoff_velocity_error_mps
        .is_some_and(|error| error <= VELOCITY_TOLERANCE_MPS);

    let endpoint_fuel = run
        .state_samples
        .iter()
        .find(|sample| {
            sample.physics_step == LAUNCH_TICKS + source_steps && sample.phase == "source_bridge"
        })
        .map(|sample| sample.fuel_kg);
    let terminal_fuel = candidate
        .terminal_bridge
        .as_ref()
        .map(|bridge| bridge.fuel_burn_kg);
    let aggregate_fuel_burn = endpoint_fuel
        .zip(terminal_fuel)
        .map(|(fuel, tail)| vehicle.initial_fuel_kg - fuel + tail);
    let aggregate_fuel_passed =
        aggregate_fuel_burn.is_some_and(|burn| burn <= vehicle.initial_fuel_kg + 1.0e-12);
    let coast_duration = candidate
        .selected_coast
        .as_ref()
        .map_or(0.0, |coast| coast.duration_s);
    let terminal_duration = candidate
        .terminal_bridge
        .as_ref()
        .map_or(0.0, |bridge| bridge.duration_s);
    let aggregate_time = (LAUNCH_TICKS + source_steps) as f64 / f64::from(policy.physics_hz)
        + coast_duration
        + terminal_duration;
    let aggregate_time_passed =
        aggregate_time <= policy.maximum_mission_time_s - policy.mission_time_reserve_s + 1.0e-12;
    // This is physical/control feasibility only. Strict endpoint tolerances
    // remain a separate final witness gate so the shooting method can improve
    // a nonzero residual over successive feasible trials.
    let passed = source_contact_free
        && coupled_thrust_passed
        && minimum_throttle_passed
        && powered_slew_passed
        && source_attitude_passed
        && launch_boundary_passed
        && source_clearance.all_samples_passed
        && aggregate_fuel_passed
        && aggregate_time_passed
        && run.run.launch.completed
        && run.run.launch.contact_free
        && run.run.rollout.saturation.fuel_burn_capped_tick_count == 0
        && run.run.rollout.saturation.fuel_exhausted_tick_count == 0;
    ScheduleScreenEvidence {
        source_handoff_reached,
        source_contact_free,
        coupled_thrust_passed,
        minimum_throttle_passed,
        powered_slew_passed,
        source_attitude_passed,
        source_attitude,
        launch_boundary_passed,
        scheduled_launch_boundary,
        source_clearance,
        aggregate_fuel_passed,
        aggregate_time_passed,
        strict_position_passed,
        strict_velocity_passed,
        passed,
        maximum_requested_thrust_acceleration_mps2: max_requested,
        maximum_powered_slew_radps: maximum_slew,
        minimum_desired_applied_throttle_fraction: minimum_throttle,
        aggregate_fuel_burn_kg: aggregate_fuel_burn,
        aggregate_mission_time_s: aggregate_time,
        authoritative_source_contacts: source_contacts,
    }
}

pub(in crate::waypoint_direct_nominal_plant) fn paired_mean_seed(
    bridge: &AnalyticalBridgeV2,
    launch_target: f64,
) -> Result<Vec<HeldSourceCommand>> {
    if !bridge.steps.is_multiple_of(2) || bridge.samples.len() != bridge.steps as usize {
        bail!("reference source bridge does not split into complete two-tick pairs");
    }
    let pair_count = bridge.samples.len() / 2;
    let first_powered = bridge
        .samples
        .iter()
        .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
        .map(|sample| direction_angle(sample.thrust_acceleration_mps2))
        .ok_or_else(|| anyhow!("reference source bridge has no powered direction"))?;
    let paired_means = bridge
        .samples
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| (pair[0].thrust_acceleration_mps2 + pair[1].thrust_acceleration_mps2) * 0.5)
        .collect::<Vec<_>>();
    let first_powered_pair_index = paired_means
        .iter()
        .position(|mean| mean.length() > 1.0e-12)
        .ok_or_else(|| anyhow!("paired reference bridge has no powered command"))?;
    let mut previous_target = launch_target;
    let mut schedule = Vec::with_capacity(pair_count);
    for (pair_index, mean) in paired_means.into_iter().enumerate() {
        let mut mean = mean;
        if pair_index == first_powered_pair_index {
            mean = Vec2::new(first_powered.sin(), first_powered.cos()) * mean.length();
        }
        let target = if mean.length() > 1.0e-12 {
            direction_angle(mean)
        } else {
            previous_target
        };
        previous_target = target;
        schedule.push(HeldSourceCommand {
            thrust_acceleration_mps2: mean,
            target_attitude_rad: target,
        });
    }
    Ok(schedule)
}

fn build_schedule(
    bridge: &AnalyticalBridgeV2,
    launch_target: f64,
    correction: Correction,
) -> Result<Vec<HeldSourceCommand>> {
    Ok(apply_correction(
        &paired_mean_seed(bridge, launch_target)?,
        correction,
    ))
}

pub(in crate::waypoint_direct_nominal_plant) fn apply_correction(
    seed: &[HeldSourceCommand],
    correction: Correction,
) -> Vec<HeldSourceCommand> {
    let first_powered_index = seed
        .iter()
        .position(|command| command.thrust_acceleration_mps2.length() > 1.0e-12)
        .unwrap_or(0);
    let correction_span = seed.len().saturating_sub(1 + first_powered_index);
    seed.iter()
        .enumerate()
        .map(|(index, command)| {
            if index <= first_powered_index || correction_span == 0 {
                return *command;
            }
            let progress = (index - first_powered_index) as f64 / correction_span as f64;
            let normalized_time = 2.0 * progress - 1.0;
            let ramp = Vec2::new(
                correction.0[0] + correction.0[2] * normalized_time,
                correction.0[1] + correction.0[3] * normalized_time,
            ) * progress;
            let corrected = command.thrust_acceleration_mps2 + ramp;
            let target = if corrected.length() > 1.0e-12 {
                direction_angle(corrected)
            } else {
                command.target_attitude_rad
            };
            HeldSourceCommand {
                thrust_acceleration_mps2: corrected,
                target_attitude_rad: target,
            }
        })
        .collect()
}

fn held_command_evidence(
    schedule: &[HeldSourceCommand],
    command_samples: &[SourceDurationCommandSample],
) -> Vec<HeldCommandEvidence> {
    let first_powered_pair_index = schedule
        .iter()
        .position(|command| command.thrust_acceleration_mps2.length() > 1.0e-12);
    schedule
        .iter()
        .enumerate()
        .map(|(pair_index, command)| {
            let first_tick = LAUNCH_TICKS + 1 + (pair_index as u64 * 2);
            let second_tick = first_tick + 1;
            let first = command_samples.iter().find(|sample| {
                sample.phase == "source_bridge" && sample.physics_step == first_tick
            });
            let second = command_samples.iter().find(|sample| {
                sample.phase == "source_bridge" && sample.physics_step == second_tick
            });
            HeldCommandEvidence {
                pair_index,
                is_first_powered_pair: first_powered_pair_index == Some(pair_index),
                first_physics_step: first_tick,
                last_physics_step: second_tick,
                desired_thrust_acceleration_mps2: command.thrust_acceleration_mps2,
                target_attitude_rad: command.target_attitude_rad,
                commanded_throttle_frac: first.map(|sample| sample.held_command_throttle_frac),
                applied_throttle_fraction_first_tick: first
                    .map(|sample| sample.plant_applied_throttle_frac),
                applied_throttle_fraction_second_tick: second
                    .map(|sample| sample.plant_applied_throttle_frac),
            }
        })
        .collect()
}

pub(in crate::waypoint_direct_nominal_plant) fn maximum_powered_slew(
    schedule: &[HeldSourceCommand],
) -> f64 {
    let mut previous: Option<(usize, f64)> = None;
    let mut maximum = 0.0_f64;
    for (index, command) in schedule.iter().enumerate() {
        if command.thrust_acceleration_mps2.length() <= 1.0e-12 {
            continue;
        }
        if let Some((previous_index, previous_target)) = previous {
            let elapsed_s = (index - previous_index) as f64 * 2.0 / 120.0;
            maximum = maximum.max(
                shortest_angle_delta(previous_target, command.target_attitude_rad).abs()
                    / elapsed_s.max(f64::EPSILON),
            );
        }
        previous = Some((index, command.target_attitude_rad));
    }
    maximum
}

// These small fixed-size loops preserve the original row-major accumulation
// order to keep the bounded experiment's floating-point schedule unchanged.
#[allow(clippy::needless_range_loop)]
pub(in crate::waypoint_direct_nominal_plant) fn damped_least_squares_step(
    jacobian: [[f64; 4]; 4],
    residual: [f64; 4],
    damping: f64,
) -> Option<[f64; 4]> {
    let mut normal = [[0.0_f64; 4]; 4];
    let mut rhs = [0.0_f64; 4];
    for row in 0..4 {
        for column in 0..4 {
            rhs[column] -= jacobian[row][column] * residual[row];
            for other in 0..4 {
                normal[column][other] += jacobian[row][column] * jacobian[row][other];
            }
        }
    }
    for (index, row) in normal.iter_mut().enumerate() {
        row[index] += damping;
    }
    solve_linear_system(normal, rhs)
}

// Pivot/elimination loops intentionally retain their original numerical order.
#[allow(clippy::needless_range_loop)]
fn solve_linear_system(mut matrix: [[f64; 4]; 4], mut rhs: [f64; 4]) -> Option<[f64; 4]> {
    for pivot in 0..4 {
        let best_row = (pivot..4).max_by(|left, right| {
            matrix[*left][pivot]
                .abs()
                .total_cmp(&matrix[*right][pivot].abs())
        })?;
        if matrix[best_row][pivot].abs() <= 1.0e-14 {
            return None;
        }
        matrix.swap(pivot, best_row);
        rhs.swap(pivot, best_row);
        let divisor = matrix[pivot][pivot];
        for column in pivot..4 {
            matrix[pivot][column] /= divisor;
        }
        rhs[pivot] /= divisor;
        for row in 0..4 {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for column in pivot..4 {
                matrix[row][column] -= factor * matrix[pivot][column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs.iter().all(|value| value.is_finite()).then_some(rhs)
}

fn iteration_evidence(input: IterationEvidenceInput) -> SolverIterationEvidence {
    let IterationEvidenceInput {
        iteration,
        correction,
        residual,
        screens_passed,
        finite_difference_columns,
        accepted_line_search_scale,
        line_search_trials,
        status,
    } = input;
    SolverIterationEvidence {
        iteration,
        correction: correction_evidence(correction),
        objective_m: residual.map(|residual| residual.objective_m),
        position_error_m: residual.map(|residual| residual.signed.position_m.length()),
        velocity_error_mps: residual.map(|residual| residual.signed.velocity_mps.length()),
        screens_passed,
        finite_difference_columns,
        accepted_line_search_scale,
        line_search_trials,
        status: status.to_owned(),
    }
}

fn better_evaluation(candidate: &ScheduleEvaluation, current: &ScheduleEvaluation) -> bool {
    match (candidate.residual, current.residual) {
        (Some(candidate_residual), Some(current_residual)) => {
            candidate.screens.passed
                && (!current.screens.passed
                    || candidate_residual.objective_m < current_residual.objective_m)
        }
        (Some(_), None) => true,
        _ => false,
    }
}

fn is_schedule_witness(evaluation: &ScheduleEvaluation) -> bool {
    evaluation.screens.passed
        && evaluation.screens.source_handoff_reached
        && evaluation.screens.strict_position_passed
        && evaluation.screens.strict_velocity_passed
        && evaluation.residual.is_some_and(|residual| {
            residual.signed.position_m.length() <= POSITION_TOLERANCE_M
                && residual.signed.velocity_mps.length() <= VELOCITY_TOLERANCE_MPS
        })
}

fn schedule_is_gate_witness(schedule: &PairedScheduleEvidence) -> bool {
    schedule.witness
        && schedule
            .full_flight_replay_trace
            .as_ref()
            .is_some_and(|trace| trace.passed)
        && schedule.full_flight_first_contact.is_some()
}

pub(in crate::waypoint_direct_nominal_plant) fn clamp_correction(
    correction: Correction,
) -> Correction {
    Correction(std::array::from_fn(|index| {
        correction.0[index].clamp(-MAX_CORRECTION_MPS2, MAX_CORRECTION_MPS2)
    }))
}

fn correction_evidence(correction: Correction) -> CorrectionEvidence {
    CorrectionEvidence {
        constant_x_mps2: correction.0[0],
        constant_y_mps2: correction.0[1],
        linear_x_mps2: correction.0[2],
        linear_y_mps2: correction.0[3],
    }
}

fn shortest_angle_delta(from: f64, to: f64) -> f64 {
    (to - from + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

fn direction_angle(thrust_acceleration: Vec2) -> f64 {
    thrust_acceleration.x.atan2(thrust_acceleration.y)
}

fn representative_row_indices() -> Vec<usize> {
    (0..3)
        .map(|candidate_index| candidate_index * 5 + 1)
        .collect()
}

fn protocol_evidence() -> PairedCommandFeasibilityProtocolEvidence {
    PairedCommandFeasibilityProtocolEvidence {
        frozen_input_rule: format!(
            "Rebuild all six sealed inputs and nested gates; bind frozen source-duration identity {EXPECTED_SOURCE_DURATION_IDENTITY}, expected file SHA-256 {EXPECTED_SOURCE_DURATION_SHA256}, the coupled audit, and all ordered fifteen rows before constructing any SimulationState. Independently check the file SHA with rtk sha256sum."
        ),
        row_rule: "Preserve all fifteen source-duration rows in their frozen order, including six explicit analytical skips. Replay both frozen baseline cadences for all nine survivors and require exact canary plus ordinary/neutral replay parity before schedule trials.".to_owned(),
        seed_rule: "For each predeclared -180 representative, reconstruct the 120 Hz reference source bridge from that row's actual held-60 launch state to the unchanged frozen source handoff at the unchanged duration. Pair adjacent reference thrust vectors by arithmetic mean and hold one resulting throttle/attitude request for two physics ticks. At the first nonzero paired-mean pair, retain mean magnitude and align its direction to the reference bridge's first powered direction; retain frozen launch target for any earlier unpowered pair.".to_owned(),
        correction_rule: "Apply four bounded parameters: constant and normalized linear-in-time x/y acceleration-vector corrections, ramped from zero at the first powered pair so they cannot rotate the preserved launch boundary. Estimate the endpoint Jacobian by central finite differences; use deterministic damped least squares, six iterations maximum, eight descending line-search scales, and a 0.25 m/s^2 absolute parameter bound.".to_owned(),
        screen_rule: "Require the strict frozen handoff position/velocity tolerances, robust coupled thrust, minimum throttle, powered slew, the original robust first-powered-direction source-attitude margin, the scheduled first-powered direction, and robust one-tick launch boundary recomputed from actual launch-end attitude, the unchanged V2 clearance mirror, aggregate fuel/time, and authoritative no-contact core source rollout. Record the mirror separately because its V2 footprint-sign ambiguity is unresolved.".to_owned(),
        replay_rule: "Run the final schedule through the unchanged full-flight 60 Hz core path and replay its logged scheduled commands through ordinary and neutral core states. Require replay parity and record each full-flight first contact, but do not require or claim stable landing.".to_owned(),
        stopping_rule: "Run all three -180 rows first. If any lacks a strict safe source-handoff witness with full-flight replay parity and an authoritative first-contact record, mark all remaining six analytical survivors not-run after the first gate and retain all fifteen rows.".to_owned(),
        non_claim: "Solver failure or fixed-budget exhaustion is not evidence that the frozen endpoint is unreachable. A passing witness is a known-outcome evaluator result, not held-out validation, certification, landing authority, or permission to change production behavior.".to_owned(),
        solver_version: SOLVER_VERSION.to_owned(),
        maximum_iterations: MAX_ITERATIONS,
        maximum_line_search_steps: MAX_LINE_SEARCH_STEPS,
        finite_difference_step_mps2: FINITE_DIFFERENCE_STEP_MPS2,
        maximum_parameter_abs_mps2: MAX_CORRECTION_MPS2,
        position_tolerance_m: POSITION_TOLERANCE_M,
        velocity_tolerance_mps: VELOCITY_TOLERANCE_MPS,
    }
}

fn artifact_identity(
    artifact: &WaypointDirectSourceDurationPairedCommandFeasibilityArtifact,
) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

fn write_artifact(
    output_dir: &Path,
    summary_path: &Path,
    artifact: &WaypointDirectSourceDurationPairedCommandFeasibilityArtifact,
) -> Result<()> {
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create paired-command output directory {}",
            output_dir.display()
        )
    })?;
    let bytes = serde_json::to_vec_pretty(artifact)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(summary_path)
        .with_context(|| {
            format!(
                "paired-command feasibility refuses to overwrite summary {}",
                summary_path.display()
            )
        })?;
    file.write_all(&bytes).with_context(|| {
        format!(
            "failed to write paired-command summary {}",
            summary_path.display()
        )
    })?;
    drop(file);
    let reloaded: WaypointDirectSourceDurationPairedCommandFeasibilityArtifact =
        serde_json::from_slice(&bytes).context("failed to reload paired-command summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != bytes
        || artifact_identity(&reloaded)? != reloaded.identity
    {
        bail!("paired-command summary failed its byte-stable identity round trip");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_plan::conservative_ballistic_bridge::BridgeSampleV2;

    fn reference_bridge(samples: Vec<BridgeSampleV2>) -> AnalyticalBridgeV2 {
        AnalyticalBridgeV2 {
            kind: BridgeKindV2::Source,
            start_state: KinematicStateV2 {
                position_m: Vec2::default(),
                velocity_mps: Vec2::default(),
            },
            end_state: KinematicStateV2 {
                position_m: Vec2::default(),
                velocity_mps: Vec2::default(),
            },
            steps: samples.len() as u64,
            duration_s: samples.len() as f64 / 120.0,
            initial_net_acceleration_mps2: Vec2::default(),
            net_acceleration_step_mps2: Vec2::default(),
            samples,
            endpoint_position_error_m: 0.0,
            endpoint_velocity_error_mps: 0.0,
            fuel_burn_kg: 0.0,
            classification: pd_plan::conservative_ballistic_bridge::CertificationV2::Certified,
            reasons: Vec::new(),
            margins: pd_plan::conservative_ballistic_bridge::ComponentMarginsV2 {
                coupled_thrust: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
                minimum_throttle: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
                clearance: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
                bridge_endpoint: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
                powered_slew: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
                source_attitude: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
                coast_slew: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
                fuel: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
                time: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
                touchdown_speed: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
                touchdown_attitude: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
                touchdown_angular_rate: pd_plan::conservative_ballistic_bridge::MarginV2 {
                    raw: f64::MAX,
                    normalized: f64::MAX,
                },
            },
            identity: String::new(),
        }
    }

    #[test]
    fn seed_uses_pair_means_and_keeps_first_powered_direction() {
        let bridge = reference_bridge(vec![
            BridgeSampleV2 {
                tick: 0,
                state_m: KinematicStateV2 {
                    position_m: Vec2::default(),
                    velocity_mps: Vec2::default(),
                },
                net_acceleration_mps2: Vec2::default(),
                thrust_acceleration_mps2: Vec2::new(1.0, 2.0),
                throttle_fraction: 0.0,
                thrust_direction_unit: None,
            },
            BridgeSampleV2 {
                tick: 1,
                state_m: KinematicStateV2 {
                    position_m: Vec2::default(),
                    velocity_mps: Vec2::default(),
                },
                net_acceleration_mps2: Vec2::default(),
                thrust_acceleration_mps2: Vec2::new(3.0, 4.0),
                throttle_fraction: 0.0,
                thrust_direction_unit: None,
            },
            BridgeSampleV2 {
                tick: 2,
                state_m: KinematicStateV2 {
                    position_m: Vec2::default(),
                    velocity_mps: Vec2::default(),
                },
                net_acceleration_mps2: Vec2::default(),
                thrust_acceleration_mps2: Vec2::new(-1.0, 2.0),
                throttle_fraction: 0.0,
                thrust_direction_unit: None,
            },
            BridgeSampleV2 {
                tick: 3,
                state_m: KinematicStateV2 {
                    position_m: Vec2::default(),
                    velocity_mps: Vec2::default(),
                },
                net_acceleration_mps2: Vec2::default(),
                thrust_acceleration_mps2: Vec2::new(1.0, 2.0),
                throttle_fraction: 0.0,
                thrust_direction_unit: None,
            },
        ]);
        let launch_target = direction_angle(Vec2::new(1.0, 2.0));
        let seed = paired_mean_seed(&bridge, launch_target).expect("paired seed builds");
        assert_eq!(seed.len(), 2);
        assert!((seed[0].thrust_acceleration_mps2.length() - 13.0_f64.sqrt()).abs() <= 1.0e-12);
        assert!(
            shortest_angle_delta(
                launch_target,
                direction_angle(seed[0].thrust_acceleration_mps2)
            )
            .abs()
                <= 1.0e-12
        );
        assert_eq!(seed[0].target_attitude_rad, launch_target);
        assert_eq!(seed[1].thrust_acceleration_mps2, Vec2::new(0.0, 2.0));
    }

    #[test]
    fn initially_unpowered_pairs_keep_launch_target_and_preserve_later_first_powered_pair() {
        let sample = |tick, thrust_acceleration_mps2| BridgeSampleV2 {
            tick,
            state_m: KinematicStateV2 {
                position_m: Vec2::default(),
                velocity_mps: Vec2::default(),
            },
            net_acceleration_mps2: Vec2::default(),
            thrust_acceleration_mps2,
            throttle_fraction: 0.0,
            thrust_direction_unit: None,
        };
        let bridge = reference_bridge(vec![
            sample(0, Vec2::default()),
            sample(1, Vec2::default()),
            sample(2, Vec2::new(0.0, 2.0)),
            sample(3, Vec2::new(0.0, 4.0)),
            sample(4, Vec2::new(1.0, 0.0)),
            sample(5, Vec2::new(1.0, 0.0)),
        ]);
        let launch_target = -0.4;
        let seed = paired_mean_seed(&bridge, launch_target).expect("paired seed builds");
        assert_eq!(seed[0].thrust_acceleration_mps2, Vec2::default());
        assert_eq!(seed[0].target_attitude_rad, launch_target);
        assert_eq!(seed[1].thrust_acceleration_mps2, Vec2::new(0.0, 3.0));
        assert!(seed[1].target_attitude_rad.abs() <= 1.0e-12);

        let corrected = apply_correction(&seed, Correction([0.02, -0.03, 0.01, 0.01]));
        assert_eq!(corrected[1], seed[1]);
        assert!(corrected[2].thrust_acceleration_mps2 != seed[2].thrust_acceleration_mps2);
        assert_eq!(
            corrected
                .iter()
                .position(|command| command.thrust_acceleration_mps2.length() > 1.0e-12),
            Some(1)
        );
    }

    #[test]
    fn correction_is_four_parameter_and_leaves_boundary_pair_unchanged() {
        let seed = vec![
            HeldSourceCommand {
                thrust_acceleration_mps2: Vec2::new(1.0, 2.0),
                target_attitude_rad: 0.1,
            },
            HeldSourceCommand {
                thrust_acceleration_mps2: Vec2::new(1.0, 2.0),
                target_attitude_rad: 0.1,
            },
            HeldSourceCommand {
                thrust_acceleration_mps2: Vec2::new(1.0, 2.0),
                target_attitude_rad: 0.1,
            },
        ];
        let correction = Correction([0.1, -0.05, 0.02, 0.03]);
        let corrected = apply_correction(&seed, correction);
        assert_eq!(corrected[0], seed[0]);
        assert_ne!(
            corrected[1].thrust_acceleration_mps2,
            seed[1].thrust_acceleration_mps2
        );
        assert_ne!(
            corrected[2].thrust_acceleration_mps2,
            seed[2].thrust_acceleration_mps2
        );
        assert!(
            clamp_correction(Correction([1.0, -1.0, 0.3, -0.4]))
                .0
                .iter()
                .all(|value| value.abs() <= MAX_CORRECTION_MPS2)
        );
    }

    #[test]
    fn damped_shooting_linear_system_solves_endpoint_jacobian() {
        let solved = solve_linear_system(
            [
                [2.0, 0.0, 0.0, 0.0],
                [0.0, 3.0, 0.0, 0.0],
                [0.0, 0.0, 4.0, 0.0],
                [0.0, 0.0, 0.0, 5.0],
            ],
            [2.0, 6.0, 12.0, 20.0],
        )
        .expect("full rank system solves");
        assert_eq!(solved, [1.0, 2.0, 3.0, 4.0]);
        assert!(solve_linear_system([[0.0; 4]; 4], [0.0; 4]).is_none());
    }

    #[test]
    fn representative_family_is_fixed_and_ordered() {
        assert_eq!(representative_row_indices(), vec![1, 6, 11]);
        assert_eq!(
            EXPECTED_SOURCE_DURATION_IDENTITY,
            "fnv1a64:dfe0f0feaa15fc24"
        );
    }
}
