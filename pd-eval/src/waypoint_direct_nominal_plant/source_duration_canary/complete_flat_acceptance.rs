//! Evaluator-only complete flat-witness acceptance over the frozen paired
//! command schedules. This lane replays sealed commands; it does not refit
//! schedules or alter any frozen artifact.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::{
    Command, ContactClassification, CorridorEnvelope, EventKind, RunContext, SimulationState, Vec2,
};
use pd_plan::conservative_ballistic_bridge::{KinematicStateV2, PadInputV2};
use serde::{Deserialize, Serialize};

mod terrain_twin;
pub use terrain_twin::*;
mod terminal_admissibility;
pub use terminal_admissibility::*;
mod body_aware_terminal;
pub use body_aware_terminal::*;
mod contact_phase_study;
pub use contact_phase_study::*;

use super::super::{
    CommandSaturationEvidence, NominalProfile, PlantStateEvidence, RolloutCadence,
    WaypointDirectCoupledThrustAuditInputGateEvidence,
    launch_contact_contract::{
        FirstContactEvidence, FirstContactPredicateMarginsEvidence, ReplayTraceParityEvidence,
        ReplayTraceResult, replay_logged_cadence,
    },
    launch_feasibility::{
        LaunchEvidence, LaunchFeasibilityCadenceRunEvidence, LaunchRolloutEvidence,
        LaunchRolloutTickEvidence, SourceDurationHoldMode, component_update_schedule,
    },
    resolve_output_dir, stable_digest,
};
use super::{
    SourceDurationVariantEvidence, WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths,
    held_cadence_diagnostic::{PreparedHeldCadenceInputs, prepare_held_cadence_inputs},
    paired_command_feasibility::{
        PairedCommandRowEvidence, PairedScheduleEvidence, ScheduledFirstContactMarginsEvidence,
        WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_ID,
        WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_SCHEMA_ID,
        WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_SCHEMA_VERSION,
        WaypointDirectSourceDurationPairedCommandFeasibilityArtifact,
    },
};

pub const WAYPOINT_DIRECT_COMPLETE_FLAT_ACCEPTANCE_ID: &str =
    "waypoint-direct-complete-flat-acceptance";
pub const WAYPOINT_DIRECT_COMPLETE_FLAT_ACCEPTANCE_SCHEMA_ID: &str =
    "waypoint_direct_complete_flat_acceptance_v1";
pub const WAYPOINT_DIRECT_COMPLETE_FLAT_ACCEPTANCE_SCHEMA_VERSION: u32 = 1;

const EXPECTED_PAIRED_COMMAND_IDENTITY: &str = "fnv1a64:080e8e9867b02d97";
const EXPECTED_PAIRED_COMMAND_SHA256: &str =
    "6477a947537208872056d46e82c7b6fc0c7aa6801f14ca9ccb8b340f4da86247";
const EXPECTED_SOURCE_DURATION_IDENTITY: &str = "fnv1a64:dfe0f0feaa15fc24";
const EXPECTED_SOURCE_DURATION_SHA256: &str =
    "a93b728374145d4f12b43b21b7d68e383270db19facdf5964ce91bc47e557e9c";
const EXPECTED_ROWS: usize = 15;
const EXPECTED_ANALYTICAL_SKIPS: usize = 6;
const EXPECTED_SCHEDULES: usize = 9;
const EXPECTED_STABLE_ROWS: [usize; 4] = [1, 2, 11, 12];
const EXPECTED_CRASH_ROWS: [usize; 5] = [5, 6, 7, 8, 9];
const LAUNCH_TICKS: u64 = 72;
const COMMAND_HOLD_PHYSICS_TICKS: u64 = 2;
const STRICT_HANDOFF_TOLERANCE_M: f64 = 1.0e-6;
const STRICT_HANDOFF_TOLERANCE_MPS: f64 = 1.0e-6;
const STATE_JOIN_TOLERANCE: f64 = 1.0e-6;
const INITIAL_REST_TOLERANCE: f64 = 1.0e-9;
const HELD_CADENCE: &str = "held_controller_60_hz";
const GEOMETRY_CONVENTION: &str = "core_current_rotated_feet_and_hull";

#[derive(Clone, Debug)]
pub struct WaypointDirectCompleteFlatAcceptanceInputPaths {
    pub source_inputs: WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths,
    pub paired_command_summary: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectCompleteFlatAcceptancePaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectCompleteFlatAcceptanceRun {
    pub artifact: WaypointDirectCompleteFlatAcceptanceArtifact,
    pub paths: WaypointDirectCompleteFlatAcceptancePaths,
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectCompleteFlatAcceptanceValidation {
    pub source_duration_identity: String,
    pub expected_source_duration_sha256: String,
    pub paired_command_identity: String,
    pub expected_paired_command_sha256: String,
    pub input_gate: WaypointDirectCoupledThrustAuditInputGateEvidence,
    pub coupled_audit_identity: String,
    pub ordered_row_count: usize,
    pub analytical_skip_count: usize,
    pub scheduled_survivor_count: usize,
    pub held_command_cadence: String,
    pub physics_hz: u32,
    pub controller_hz: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectCompleteFlatAcceptanceArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub source_duration_identity: String,
    pub expected_source_duration_sha256: String,
    pub paired_command_identity: String,
    pub expected_paired_command_sha256: String,
    pub input_gate: WaypointDirectCoupledThrustAuditInputGateEvidence,
    pub coupled_audit_identity: String,
    pub protocol: CompleteFlatAcceptanceProtocolEvidence,
    pub family_proof: CompleteFlatAcceptanceFamilyProofEvidence,
    pub rows: Vec<CompleteFlatAcceptanceRowEvidence>,
    pub contact_replay_regression: ContactReplayRegressionEvidence,
    pub selection: Option<AcceptedWrapperSelectionEvidence>,
    pub execution_status: String,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompleteFlatAcceptanceProtocolEvidence {
    pub source_input_rule: String,
    pub paired_input_rule: String,
    pub replay_rule: String,
    pub geometry_rule: String,
    pub corridor_rule: String,
    pub contact_rule: String,
    pub selection_rule: String,
    pub evidence_classification: String,
    pub non_claim: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompleteFlatAcceptanceFamilyProofEvidence {
    pub ordered_row_count: usize,
    pub recorded_row_count: usize,
    pub analytical_skip_count: usize,
    pub scheduled_survivor_count: usize,
    pub completed_replay_count: usize,
    pub accepted_wrapper_count: usize,
    pub omitted_row_count: usize,
    pub all_rows_recorded: bool,
    pub contact_replay_regression_passed: bool,
    pub at_least_one_complete_wrapper_accepted: bool,
    pub flat_gate_passed: bool,
    pub stopping_result: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompleteFlatAcceptanceRowEvidence {
    pub row_index: usize,
    pub candidate_identity: String,
    pub role: String,
    pub duration_offset_ticks: i64,
    pub source_bridge_tick_count: u64,
    pub analytical_survivor: bool,
    pub status: String,
    pub skip_reason: Option<String>,
    pub wrapper: Option<CompleteFlatWrapperEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompleteFlatWrapperEvidence {
    pub wrapper_identity: String,
    pub provenance: CompleteFlatWrapperProvenanceEvidence,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub acceptance: CompleteFlatAcceptanceGatesEvidence,
    pub accepted: bool,
    pub first_failing_gate: Option<String>,
    pub first_failing_tick: Option<u64>,
    pub planned_total_mission_time_s: f64,
    pub actual_touchdown_time_s: Option<f64>,
    pub actual_touchdown_fuel_remaining_kg: Option<f64>,
    pub actual_touchdown_fuel_used_kg: Option<f64>,
    pub first_contact: Option<FirstContactEvidence>,
    pub signed_first_contact_margins: Option<ScheduledFirstContactMarginsEvidence>,
    pub minimum_airborne_clearance_m: Option<f64>,
    pub minimum_airborne_clearance: Option<AirborneClearanceMinimumEvidence>,
    pub clearance_scan: GeometryClearanceScanEvidence,
    pub replay: CompleteFlatReplayEvidence,
    pub saturation: Option<CommandSaturationEvidence>,
    pub recomputed_saturation: Option<CommandSaturationEvidence>,
    pub rejection_reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompleteFlatWrapperProvenanceEvidence {
    pub source_row_index: usize,
    pub frozen_basis_identity: String,
    pub source_duration_identity: String,
    pub paired_command_identity: String,
    pub scenario_identity: String,
    pub policy_identity: String,
    pub acceptance_policy_identity: String,
    pub launch_rule: String,
    pub cadence: String,
    pub geometry_convention: String,
    pub launch_schedule_identity: String,
    pub source_command_schedule_identity: String,
    pub tail_command_schedule_identity: String,
    pub frozen_tail_profile_identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompleteFlatAcceptanceGatesEvidence {
    pub supported_source_pad_rest_state: bool,
    pub launch_completed_and_contact_free: bool,
    pub launch_to_source_join_passed: bool,
    pub existing_source_screens_passed: bool,
    pub strict_source_handoff_passed: bool,
    pub scheduled_source_prefix_parity_passed: bool,
    pub ordinary_neutral_and_stored_replay_parity_passed: bool,
    pub pointwise_core_geometry_clearance_passed: bool,
    pub terminal_entry_descending_and_clear: bool,
    pub terminal_handoff_on_descending_arc: bool,
    pub first_contact_stable_safe_on_target: bool,
    pub no_earlier_contact: bool,
    pub fuel_time_and_commandability_budgets_passed: bool,
    pub planned_time_reserve_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompleteFlatReplayEvidence {
    pub stored_replay_trace: Option<ReplayTraceParityEvidence>,
    pub recomputed_replay_trace: Option<ReplayTraceParityEvidence>,
    pub independent_neutral_replay: NeutralReplayEvidence,
    pub stored_first_contact_matches_replay: bool,
    pub stored_first_contact_margins_match_replay: bool,
    pub full_rollout_matches_frozen_launch_and_tail: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeutralReplayEvidence {
    pub physics_ticks_replayed: u64,
    pub contiguous_from_step_one: bool,
    pub logged_command_schedule_matches: bool,
    pub expected_thrust_reference_coverage: bool,
    pub ordinary_and_neutral_states_match: bool,
    pub poststep_contact_labels_match_log: bool,
    pub no_contact_before_first_event: bool,
    pub first_contact_physics_step: Option<u64>,
    pub first_contact_classification: Option<String>,
    pub first_contact_state: Option<PlantStateEvidence>,
    pub first_contact_matches_authoritative_replay: bool,
    pub first_command_mismatch: Option<ReplayValidationMismatchEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayValidationMismatchEvidence {
    pub physics_step: Option<u64>,
    pub field: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometryClearanceScanEvidence {
    pub poststep_state_count: u64,
    pub airborne_state_count: u64,
    pub source_corridor_state_count: u64,
    pub terminal_corridor_state_count: u64,
    pub exact_clearance_query_count: u64,
    pub all_airborne_states_passed: bool,
    pub first_violation: Option<GeometryClearanceViolationEvidence>,
    pub minimum_airborne: Option<AirborneClearanceMinimumEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AirborneClearanceMinimumEvidence {
    pub physics_step: u64,
    pub phase: String,
    pub clearance_m: f64,
    pub required_clearance_m: f64,
    pub corridor: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometryClearanceViolationEvidence {
    pub physics_step: u64,
    pub phase: String,
    pub reason: String,
    pub clearance_m: Option<f64>,
    pub required_clearance_m: f64,
    pub corridor: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactReplayRegressionEvidence {
    pub expected_stable_target_rows: Vec<usize>,
    pub observed_stable_target_rows: Vec<usize>,
    pub expected_crash_rows: Vec<usize>,
    pub observed_crash_rows: Vec<usize>,
    pub replay_parity_row_count: usize,
    pub expected_contact_row_count: usize,
    pub all_expected_contact_classes_reproduced: bool,
    pub all_stored_replays_passed: bool,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcceptedWrapperSelectionEvidence {
    pub row_index: usize,
    pub wrapper_identity: String,
    pub planned_total_mission_time_s: f64,
    pub selection_rule: String,
}

struct PreparedAcceptanceInputs {
    source: PreparedHeldCadenceInputs,
    paired: WaypointDirectSourceDurationPairedCommandFeasibilityArtifact,
}

/// Physical verifier inputs, independent of historical summary adapters.
pub(in crate::waypoint_direct_nominal_plant) struct CompleteWitnessVerifierRequest<'a> {
    pub context: &'a RunContext,
    pub scenario: &'a pd_core::ScenarioSpec,
    pub probe: &'a pd_plan::conservative_ballistic_bridge::DirectBridgeProbeV2,
    pub candidate: &'a pd_plan::conservative_ballistic_bridge::DirectBridgeCandidateV2,
    pub selected: &'a super::super::PreparedProfileCandidate,
    pub profile: &'a NominalProfile,
    pub source_pad: &'a PadInputV2,
    pub target_pad: &'a PadInputV2,
    pub policy: &'a pd_plan::conservative_ballistic_bridge::DirectBridgePolicyV2,
    pub row_index: usize,
    pub basis_candidate_identity: &'a str,
    pub source_bridge_tick_count: u64,
    pub source_handoff: KinematicStateV2,
    pub launch: &'a LaunchEvidence,
    pub reseeded_bridge: &'a Option<super::super::launch_feasibility::ReseededBridgeEvidence>,
    pub schedule: &'a PairedScheduleEvidence,
    pub scenario_identity: &'a str,
    pub baseline_replay_parity: bool,
    pub identity_bindings: CompleteWitnessIdentityBindings<'a>,
}

#[derive(Clone, Copy)]
pub(in crate::waypoint_direct_nominal_plant) enum CompleteWitnessIdentityBindings<'a> {
    Historical {
        source_duration_identity: &'a str,
        paired_command_identity: &'a str,
    },
    Generated {
        request_identity: &'a str,
        generation_policy_identity: &'a str,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct ExpectedRowBinding {
    row_index: usize,
    candidate_identity: String,
    role: String,
    duration_offset_ticks: i64,
    source_bridge_tick_count: u64,
    analytical_survivor: bool,
    source_handoff_survivor: bool,
    source_handoff: KinematicStateV2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BodyAabb {
    horizontal_extent_m: f64,
    vertical_extent_m: f64,
    feet_x_min_m: f64,
    feet_x_max_m: f64,
    hull_x_min_m: f64,
    hull_x_max_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FlatPadBounds {
    left_m: f64,
    right_m: f64,
    surface_y_m: f64,
    flat: bool,
}

#[derive(Clone, Copy)]
struct ClearancePolicy {
    source_pad: FlatPadBounds,
    target_pad: FlatPadBounds,
    minimum_clearance_m: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct NeutralReplayOutcome {
    evidence: NeutralReplayEvidence,
    clearance_scan: GeometryClearanceScanEvidence,
    terminal_entry_state: Option<PlantStateEvidence>,
    first_contact: Option<FirstContactEvidence>,
    first_contact_margins: Option<FirstContactPredicateMarginsEvidence>,
    saturation: CommandSaturationEvidence,
    source_handoff_state: Option<PlantStateEvidence>,
    phase_boundaries_match: bool,
}

#[derive(Clone, Debug, Serialize)]
struct WrapperIdentityInput<'a> {
    source_row_index: usize,
    frozen_basis_identity: &'a str,
    source_duration_identity: &'a str,
    paired_command_identity: &'a str,
    scenario_identity: &'a str,
    policy_identity: &'a str,
    acceptance_policy_identity: &'a str,
    launch_rule: &'a str,
    cadence: &'a str,
    geometry_convention: &'a str,
    launch_schedule_identity: &'a str,
    source_command_schedule_identity: &'a str,
    tail_command_schedule_identity: &'a str,
    frozen_tail_profile_identity: &'a str,
}

#[derive(Serialize)]
struct AcceptancePolicyIdentityInput<'a> {
    frozen_policy: &'a pd_plan::conservative_ballistic_bridge::DirectBridgePolicyV2,
    policy_rule: &'static str,
    geometry_convention: &'static str,
    minimum_clearance_rule: &'static str,
    source_corridor_phases: &'static [&'static str],
    terminal_corridor_phase: &'static str,
    handoff_position_tolerance_m: f64,
    handoff_velocity_tolerance_mps: f64,
    state_join_tolerance: f64,
    initial_rest_tolerance: f64,
    physics_ticks_per_held_command: u64,
}

#[derive(Serialize)]
struct LoggedCommandIdentityTick<'a> {
    physics_step: u64,
    phase: &'a str,
    desired_target_attitude_rad: f64,
    held_target_attitude_rad: f64,
    commanded_throttle_frac: f64,
    applied_throttle_frac: f64,
}

#[derive(Serialize)]
struct TailProfileIdentityTick<'a> {
    phase: &'a str,
    expected_state: KinematicStateV2,
    thrust_acceleration_mps2: Vec2,
    target_attitude_rad: f64,
}

struct ReplayLogTick<'a> {
    physics_step: u64,
    phase: &'a str,
    expected_contact: &'a str,
    desired_target_attitude_rad: f64,
    held_target_attitude_rad: f64,
    commanded_throttle_frac: f64,
    attitude_before_step_rad: f64,
    logged_applied_throttle_frac: Option<f64>,
}

struct NeutralSourceReference<'a> {
    source_commands: &'a [super::paired_command_feasibility::HeldCommandEvidence],
    profile: &'a NominalProfile,
    source_bridge_tick_count: u64,
    verify_generated_reference: bool,
}

/// Rebuild the sealed source family and validate the paired schedule summary.
/// This path does not construct `SimulationState` or create output.
pub fn validate_waypoint_direct_complete_flat_acceptance_inputs(
    repo_root: &Path,
    input_paths: &WaypointDirectCompleteFlatAcceptanceInputPaths,
) -> Result<WaypointDirectCompleteFlatAcceptanceValidation> {
    let prepared = prepare_acceptance_inputs(repo_root, input_paths)?;
    Ok(validation_from_prepared(&prepared))
}

/// Replay the nine sealed held-command schedules and apply the pointwise core
/// geometry and whole-flight acceptance gates.
pub fn run_waypoint_direct_complete_flat_acceptance(
    repo_root: &Path,
    input_paths: &WaypointDirectCompleteFlatAcceptanceInputPaths,
    output_dir: &Path,
) -> Result<WaypointDirectCompleteFlatAcceptanceRun> {
    let prepared = prepare_acceptance_inputs(repo_root, input_paths)?;
    let validation = validation_from_prepared(&prepared);
    let output_dir = resolve_output_dir(repo_root, output_dir);
    let summary_path = output_dir.join("summary.json");
    if summary_path.exists() {
        bail!(
            "complete flat acceptance refuses to overwrite existing summary {}",
            summary_path.display()
        );
    }

    let context = RunContext::from_scenario(&prepared.source.validated.flat_case.scenario)
        .map_err(anyhow::Error::msg)?;
    let mut rows = Vec::with_capacity(EXPECTED_ROWS);
    let mut replayed = 0;
    for expected in expected_row_bindings(&prepared.source)? {
        let paired_row = prepared
            .paired
            .rows
            .get(expected.row_index)
            .ok_or_else(|| anyhow!("validated paired row {} disappeared", expected.row_index))?;
        if !expected.analytical_survivor {
            rows.push(CompleteFlatAcceptanceRowEvidence {
                row_index: expected.row_index,
                candidate_identity: expected.candidate_identity,
                role: expected.role,
                duration_offset_ticks: expected.duration_offset_ticks,
                source_bridge_tick_count: expected.source_bridge_tick_count,
                analytical_survivor: false,
                status: "analytical_screen_skip_unflown".to_owned(),
                skip_reason: paired_row.skip_reason.clone(),
                wrapper: None,
            });
            continue;
        }
        let wrapper = run_wrapper(
            &prepared,
            &context,
            &expected,
            paired_row,
            validation.source_duration_identity.as_str(),
            validation.paired_command_identity.as_str(),
        )?;
        replayed += 1;
        rows.push(CompleteFlatAcceptanceRowEvidence {
            row_index: expected.row_index,
            candidate_identity: expected.candidate_identity,
            role: expected.role,
            duration_offset_ticks: expected.duration_offset_ticks,
            source_bridge_tick_count: expected.source_bridge_tick_count,
            analytical_survivor: true,
            status: if wrapper.accepted {
                "nominal_replay_validated_direct".to_owned()
            } else {
                "complete_witness_rejected_by_declared_gate".to_owned()
            },
            skip_reason: None,
            wrapper: Some(wrapper),
        });
    }

    if rows.len() != EXPECTED_ROWS || rows.iter().enumerate().any(|(i, row)| row.row_index != i) {
        bail!("complete flat acceptance did not retain all fifteen ordered rows");
    }
    let contact_replay_regression = contact_replay_regression(&rows, &prepared.paired)?;
    let accepted_count = rows
        .iter()
        .filter_map(|row| row.wrapper.as_ref())
        .filter(|wrapper| wrapper.accepted)
        .count();
    let all_rows_recorded = rows.len() == EXPECTED_ROWS;
    let at_least_one_complete_wrapper_accepted = accepted_count > 0;
    let flat_gate_passed = all_rows_recorded
        && contact_replay_regression.passed
        && at_least_one_complete_wrapper_accepted;
    let selection = select_accepted_wrapper(&rows, flat_gate_passed);
    let stopping_result = if flat_gate_passed {
        "nominal_replay_validated_direct".to_owned()
    } else if !contact_replay_regression.passed {
        "unknown_contact_replay_regression_failed".to_owned()
    } else {
        "unknown_no_complete_wrapper_accepted".to_owned()
    };
    let artifact = WaypointDirectCompleteFlatAcceptanceArtifact {
        schema_id: WAYPOINT_DIRECT_COMPLETE_FLAT_ACCEPTANCE_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_COMPLETE_FLAT_ACCEPTANCE_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_COMPLETE_FLAT_ACCEPTANCE_ID.to_owned(),
        source_duration_identity: validation.source_duration_identity,
        expected_source_duration_sha256: validation.expected_source_duration_sha256,
        paired_command_identity: validation.paired_command_identity,
        expected_paired_command_sha256: validation.expected_paired_command_sha256,
        input_gate: validation.input_gate,
        coupled_audit_identity: validation.coupled_audit_identity,
        protocol: protocol_evidence(),
        family_proof: CompleteFlatAcceptanceFamilyProofEvidence {
            ordered_row_count: EXPECTED_ROWS,
            recorded_row_count: rows.len(),
            analytical_skip_count: rows.iter().filter(|row| !row.analytical_survivor).count(),
            scheduled_survivor_count: rows.iter().filter(|row| row.analytical_survivor).count(),
            completed_replay_count: replayed,
            accepted_wrapper_count: accepted_count,
            omitted_row_count: EXPECTED_ROWS.saturating_sub(rows.len()),
            all_rows_recorded,
            contact_replay_regression_passed: contact_replay_regression.passed,
            at_least_one_complete_wrapper_accepted,
            flat_gate_passed,
            stopping_result: stopping_result.clone(),
        },
        rows,
        contact_replay_regression,
        selection,
        execution_status: stopping_result,
        scope_non_claims: vec![
            "This is pointwise, discrete nominal simulator replay evidence, not a continuous swept-path or perturbation-robustness guarantee.".to_owned(),
            "The unchanged core-current thrust/body orientation convention is used explicitly; this result is not a V2 Certified classification or a physical-orientation reconciliation.".to_owned(),
            "No planner, waypoint search, controller, held-out family, simulator/contact rule, source schedule, or default selection is changed.".to_owned(),
            "A valid frozen family with no complete accepted wrapper is unknown and does not establish physical impossibility.".to_owned(),
        ],
        identity: String::new(),
    };
    let mut artifact = artifact;
    artifact.identity = acceptance_artifact_identity(&artifact)?;
    write_artifact(&output_dir, &summary_path, &artifact)?;
    Ok(WaypointDirectCompleteFlatAcceptanceRun {
        artifact,
        paths: WaypointDirectCompleteFlatAcceptancePaths {
            output_dir,
            summary_path,
        },
    })
}

fn prepare_acceptance_inputs(
    repo_root: &Path,
    input_paths: &WaypointDirectCompleteFlatAcceptanceInputPaths,
) -> Result<PreparedAcceptanceInputs> {
    let source = prepare_held_cadence_inputs(repo_root, &input_paths.source_inputs)?;
    let paired: WaypointDirectSourceDurationPairedCommandFeasibilityArtifact =
        super::super::read_summary(&input_paths.paired_command_summary)?;
    validate_paired_summary(&source, &paired)?;
    Ok(PreparedAcceptanceInputs { source, paired })
}

fn validate_paired_summary(
    source: &PreparedHeldCadenceInputs,
    paired: &WaypointDirectSourceDurationPairedCommandFeasibilityArtifact,
) -> Result<()> {
    let computed_identity = paired_command_artifact_identity(paired)?;
    if paired.identity != computed_identity || paired.identity != EXPECTED_PAIRED_COMMAND_IDENTITY {
        bail!("paired-command semantic identity changed or is not the pinned run_d/run_e identity");
    }
    if paired.schema_id != WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_SCHEMA_ID
        || paired.schema_version
            != WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_SCHEMA_VERSION
        || paired.characterization_id
            != WAYPOINT_DIRECT_SOURCE_DURATION_PAIRED_COMMAND_FEASIBILITY_ID
        || paired.source_duration_identity != EXPECTED_SOURCE_DURATION_IDENTITY
        || paired.expected_source_duration_sha256 != EXPECTED_SOURCE_DURATION_SHA256
        || paired.input_gate != source.input_gate
        || paired.coupled_audit_identity != source.coupled_audit_identity
    {
        bail!("paired-command schema or upstream input binding changed");
    }
    let proof = &paired.family_proof;
    if proof.ordered_row_count != EXPECTED_ROWS
        || proof.recorded_row_count != EXPECTED_ROWS
        || proof.analytical_skip_count != EXPECTED_ANALYTICAL_SKIPS
        || proof.survivor_count != EXPECTED_SCHEDULES
        || proof.baseline_pair_count != EXPECTED_SCHEDULES
        || proof.representative_row_indices != [1, 6, 11]
        || proof.first_gate_required_pass_count != 3
        || proof.first_gate_observed_pass_count != 3
        || proof.second_phase_survivor_count != 6
        || proof.second_phase_completed_count != 6
        || proof.omitted_row_count != 0
        || !proof.all_rows_recorded
        || !paired.first_gate_passed
        || paired.execution_status != "first_gate_passed_remaining_survivors_completed"
    {
        bail!("paired-command family proof is incomplete or changed");
    }
    let bindings = expected_row_bindings(source)?;
    validate_paired_row_bindings(&paired.rows, &bindings)?;
    for binding in bindings
        .iter()
        .filter(|binding| binding.analytical_survivor)
    {
        let row = &paired.rows[binding.row_index];
        validate_paired_schedule_provenance(source, row, binding)?;
    }
    validate_stored_contact_regression(&paired.rows)?;
    Ok(())
}

fn validate_paired_row_bindings(
    actual: &[PairedCommandRowEvidence],
    expected: &[ExpectedRowBinding],
) -> Result<()> {
    if actual.len() != EXPECTED_ROWS || expected.len() != EXPECTED_ROWS {
        bail!("paired-command row coverage must contain all fifteen rows");
    }
    for (row_index, (row, binding)) in actual.iter().zip(expected).enumerate() {
        if row.row_index != row_index
            || binding.row_index != row_index
            || row.candidate_identity != binding.candidate_identity
            || row.role != binding.role
            || row.duration_offset_ticks != binding.duration_offset_ticks
            || row.source_bridge_tick_count != binding.source_bridge_tick_count
            || row.analytical_survivor != binding.analytical_survivor
            || row.source_handoff_survivor != binding.source_handoff_survivor
            || row.frozen_source_handoff != binding.source_handoff
        {
            bail!("paired-command frozen binding changed at row {row_index}");
        }
        if binding.analytical_survivor {
            if !row.source_handoff_survivor
                || row
                    .baseline_pair
                    .as_ref()
                    .is_none_or(|pair| !pair.both_match_frozen_canary)
                || row.paired_schedule.is_none()
            {
                bail!("paired-command survivor row {row_index} lacks a complete paired schedule");
            }
        } else if row.paired_schedule.is_some() || row.baseline_pair.is_some() {
            bail!("analytical skip row {row_index} unexpectedly contains a flight schedule");
        }
    }
    Ok(())
}

fn validate_paired_schedule_provenance(
    source: &PreparedHeldCadenceInputs,
    row: &PairedCommandRowEvidence,
    binding: &ExpectedRowBinding,
) -> Result<()> {
    let schedule = row
        .paired_schedule
        .as_ref()
        .ok_or_else(|| anyhow!("paired schedule for row {} is missing", row.row_index))?;
    if !schedule.witness
        || !schedule.screens.passed
        || !schedule.screens.source_handoff_reached
        || !schedule.screens.source_contact_free
        || !schedule.screens.coupled_thrust_passed
        || !schedule.screens.minimum_throttle_passed
        || !schedule.screens.powered_slew_passed
        || !schedule.screens.source_attitude_passed
        || !schedule.screens.launch_boundary_passed
        || !schedule.screens.source_clearance.all_samples_passed
        || !schedule.screens.aggregate_fuel_passed
        || !schedule.screens.aggregate_time_passed
        || !schedule.screens.strict_position_passed
        || !schedule.screens.strict_velocity_passed
        || schedule
            .position_error_m
            .is_none_or(|error| !error.is_finite() || error > STRICT_HANDOFF_TOLERANCE_M)
        || schedule
            .velocity_error_mps
            .is_none_or(|error| !error.is_finite() || error > STRICT_HANDOFF_TOLERANCE_MPS)
        || !schedule.scheduled_source_prefix_parity.passed
        || !schedule
            .scheduled_source_prefix_parity
            .rollout_prefix_matches
        || !schedule
            .scheduled_source_prefix_parity
            .state_sample_prefix_matches
        || !schedule
            .scheduled_source_prefix_parity
            .command_sample_prefix_matches
        || !schedule
            .scheduled_source_prefix_parity
            .source_handoff_endpoint_matches
        || !schedule
            .scheduled_source_prefix_parity
            .source_contacts_match
        || !schedule
            .scheduled_source_prefix_parity
            .launch_and_bridge_match
        || schedule.full_flight_status.is_empty()
        || schedule.full_flight_first_contact.is_none()
        || schedule.full_flight_first_contact_margins.is_none()
        || schedule
            .full_flight_replay_trace
            .as_ref()
            .is_none_or(|trace| !trace.passed)
        || schedule.full_flight_rollout.is_none()
    {
        bail!(
            "paired schedule screens, source prefix, or full-flight replay changed at row {}",
            row.row_index
        );
    }
    let analysis = source
        .paired_source_variant(binding.row_index)?
        .launch_and_analytical_screen
        .as_ref()
        .ok_or_else(|| anyhow!("frozen source row {} has no launch analysis", row.row_index))?;
    if !analysis.launch.completed
        || !analysis.launch.contact_free
        || analysis.launch.physics_ticks_completed != LAUNCH_TICKS
        || analysis.launch.end_state.is_none()
        || analysis.reseeded_bridge.is_none()
    {
        bail!(
            "frozen launch or reseeded bridge provenance is incomplete at row {}",
            row.row_index
        );
    }
    let rollout = schedule
        .full_flight_rollout
        .as_ref()
        .ok_or_else(|| anyhow!("scheduled full-flight rollout disappeared"))?;
    if rollout.status != schedule.full_flight_status
        || rollout.source_handoff_reached != schedule.screens.source_handoff_reached
        || !rollout.source_handoff_contact_free
    {
        bail!(
            "scheduled full-flight status or source-handoff provenance changed at row {}",
            row.row_index
        );
    }
    validate_source_schedule_join(row, schedule, rollout, binding)?;
    validate_frozen_tail_join(source, schedule, rollout, binding)?;
    Ok(())
}

fn validate_source_schedule_join(
    row: &PairedCommandRowEvidence,
    schedule: &PairedScheduleEvidence,
    rollout: &LaunchRolloutEvidence,
    binding: &ExpectedRowBinding,
) -> Result<()> {
    if !binding
        .source_bridge_tick_count
        .is_multiple_of(COMMAND_HOLD_PHYSICS_TICKS)
        || schedule.commands.len()
            != (binding.source_bridge_tick_count / COMMAND_HOLD_PHYSICS_TICKS) as usize
    {
        bail!(
            "scheduled source command count changed at row {}",
            row.row_index
        );
    }
    for (pair_index, command) in schedule.commands.iter().enumerate() {
        let first = LAUNCH_TICKS + 1 + pair_index as u64 * COMMAND_HOLD_PHYSICS_TICKS;
        let last = first + COMMAND_HOLD_PHYSICS_TICKS - 1;
        if command.pair_index != pair_index
            || command.first_physics_step != first
            || command.last_physics_step != last
        {
            bail!(
                "scheduled source command labels changed at row {} pair {pair_index}",
                row.row_index
            );
        }
        for (physics_step, expected_applied) in [
            (first, command.applied_throttle_fraction_first_tick),
            (last, command.applied_throttle_fraction_second_tick),
        ] {
            let tick = rollout
                .per_step
                .iter()
                .find(|tick| tick.physics_step == physics_step)
                .ok_or_else(|| {
                    anyhow!(
                        "source command step {physics_step} is missing from row {} rollout",
                        row.row_index
                    )
                })?;
            if tick.phase != "source_bridge"
                || tick.held_target_attitude_rad != command.target_attitude_rad
                || command
                    .commanded_throttle_frac
                    .is_some_and(|throttle| tick.commanded_throttle_frac != throttle)
                || expected_applied.is_some_and(|applied| tick.applied_throttle_frac != applied)
            {
                bail!(
                    "source schedule does not join the logged full-flight command at row {} step {physics_step}",
                    row.row_index
                );
            }
        }
    }
    Ok(())
}

fn validate_frozen_tail_join(
    source: &PreparedHeldCadenceInputs,
    schedule: &PairedScheduleEvidence,
    rollout: &LaunchRolloutEvidence,
    binding: &ExpectedRowBinding,
) -> Result<()> {
    let source_variant = source.paired_source_variant(binding.row_index)?;
    let candidate_input = source
        .variants
        .get(binding.row_index / 5)
        .map(|entry| &entry.0)
        .ok_or_else(|| anyhow!("candidate for row {} disappeared", binding.row_index))?;
    let tail_start = candidate_input
        .selected_profile
        .profile
        .accounting
        .source_bridge_sample_count;
    let source_end_step = LAUNCH_TICKS + binding.source_bridge_tick_count;
    let tail_rows = rollout
        .per_step
        .iter()
        .filter(|tick| tick.physics_step > source_end_step);
    for tick in tail_rows {
        let tail_index = frozen_tail_profile_index(
            tick.physics_step,
            binding.source_bridge_tick_count,
            tail_start,
        )
        .ok_or_else(|| {
            anyhow!(
                "invalid frozen tail index at row {} step {}",
                binding.row_index,
                tick.physics_step
            )
        })?;
        let profile_tick = candidate_input
            .selected_profile
            .profile
            .ticks
            .get(tail_index)
            .ok_or_else(|| {
                anyhow!(
                    "frozen tail profile ended before scheduled row {} step {}",
                    binding.row_index,
                    tick.physics_step
                )
            })?;
        if tick.phase != profile_tick.phase.as_str()
            || tick.desired_target_attitude_rad != profile_tick.target_attitude_rad
        {
            bail!(
                "scheduled coast/terminal tail phase or reference direction changed at row {} step {}",
                binding.row_index,
                tick.physics_step
            );
        }
    }
    if schedule.full_flight_rollout.is_none()
        || source_variant
            .launch_and_analytical_screen
            .as_ref()
            .is_none_or(|analysis| analysis.reseeded_bridge.is_none())
    {
        bail!("scheduled full-flight rollout missing while binding frozen tail");
    }
    Ok(())
}

fn validate_stored_contact_regression(rows: &[PairedCommandRowEvidence]) -> Result<()> {
    for row_index in EXPECTED_STABLE_ROWS {
        let contact = rows
            .get(row_index)
            .and_then(|row| row.paired_schedule.as_ref())
            .and_then(|schedule| schedule.full_flight_first_contact.as_ref())
            .ok_or_else(|| {
                anyhow!("expected stable-contact row {row_index} has no stored contact")
            })?;
        if contact.classification != "stable_touchdown_on_target" {
            bail!("stored contact regression changed at stable row {row_index}");
        }
    }
    for row_index in EXPECTED_CRASH_ROWS {
        let contact = rows
            .get(row_index)
            .and_then(|row| row.paired_schedule.as_ref())
            .and_then(|schedule| schedule.full_flight_first_contact.as_ref())
            .ok_or_else(|| anyhow!("expected crash row {row_index} has no stored contact"))?;
        if contact.classification != "crash" {
            bail!("stored contact regression changed at crash row {row_index}");
        }
    }
    Ok(())
}

fn expected_row_bindings(source: &PreparedHeldCadenceInputs) -> Result<Vec<ExpectedRowBinding>> {
    let mut expected = Vec::with_capacity(EXPECTED_ROWS);
    for (candidate_index, (candidate_input, variants)) in source.variants.iter().enumerate() {
        let basis = source
            .frozen
            .bases
            .get(candidate_index)
            .ok_or_else(|| anyhow!("frozen source basis {candidate_index} is missing"))?;
        for (duration_index, variant) in variants.iter().enumerate() {
            let frozen = basis
                .duration_variants
                .get(duration_index)
                .ok_or_else(|| anyhow!("frozen source variant row is missing"))?;
            let source_handoff = candidate_input
                .candidate
                .source_handoff
                .as_ref()
                .ok_or_else(|| anyhow!("candidate has no frozen source handoff"))?
                .state;
            expected.push(ExpectedRowBinding {
                row_index: variant.row_index,
                candidate_identity: candidate_input.candidate.identity.clone(),
                role: basis.role.clone(),
                duration_offset_ticks: variant.duration_offset_ticks,
                source_bridge_tick_count: variant.source_bridge_tick_count,
                analytical_survivor: frozen.analytical_survivor,
                source_handoff_survivor: frozen.source_handoff_survivor,
                source_handoff,
            });
        }
    }
    if expected.len() != EXPECTED_ROWS
        || expected
            .iter()
            .enumerate()
            .any(|(index, row)| row.row_index != index)
    {
        bail!("rebuilt source family does not contain fifteen ordered row bindings");
    }
    Ok(expected)
}

impl PreparedHeldCadenceInputs {
    fn paired_source_variant(&self, row_index: usize) -> Result<&SourceDurationVariantEvidence> {
        self.frozen
            .bases
            .get(row_index / 5)
            .and_then(|basis| basis.duration_variants.get(row_index % 5))
            .ok_or_else(|| anyhow!("frozen source row {row_index} disappeared"))
    }
}

fn validation_from_prepared(
    prepared: &PreparedAcceptanceInputs,
) -> WaypointDirectCompleteFlatAcceptanceValidation {
    WaypointDirectCompleteFlatAcceptanceValidation {
        source_duration_identity: prepared.source.frozen.identity.clone(),
        expected_source_duration_sha256: EXPECTED_SOURCE_DURATION_SHA256.to_owned(),
        paired_command_identity: prepared.paired.identity.clone(),
        expected_paired_command_sha256: EXPECTED_PAIRED_COMMAND_SHA256.to_owned(),
        input_gate: prepared.source.input_gate.clone(),
        coupled_audit_identity: prepared.source.coupled_audit_identity.clone(),
        ordered_row_count: EXPECTED_ROWS,
        analytical_skip_count: EXPECTED_ANALYTICAL_SKIPS,
        scheduled_survivor_count: EXPECTED_SCHEDULES,
        held_command_cadence: HELD_CADENCE.to_owned(),
        physics_hz: prepared.source.validated.flat_case.scenario.sim.physics_hz,
        controller_hz: prepared
            .source
            .validated
            .flat_case
            .scenario
            .sim
            .controller_hz,
    }
}

fn run_wrapper(
    prepared: &PreparedAcceptanceInputs,
    context: &RunContext,
    binding: &ExpectedRowBinding,
    paired_row: &PairedCommandRowEvidence,
    source_duration_identity: &str,
    paired_command_identity: &str,
) -> Result<CompleteFlatWrapperEvidence> {
    let candidate_input = prepared
        .source
        .variants
        .get(binding.row_index / 5)
        .map(|entry| &entry.0)
        .ok_or_else(|| anyhow!("candidate for row {} disappeared", binding.row_index))?;
    let frozen_variant = prepared.source.paired_source_variant(binding.row_index)?;
    let analysis = frozen_variant
        .launch_and_analytical_screen
        .as_ref()
        .ok_or_else(|| {
            anyhow!(
                "frozen survivor row {} has no launch evidence",
                binding.row_index
            )
        })?;
    let schedule = paired_row.paired_schedule.as_ref().ok_or_else(|| {
        anyhow!(
            "validated schedule for row {} disappeared",
            binding.row_index
        )
    })?;
    let scenario_identity = stable_digest(&prepared.source.validated.flat_case.scenario)?;
    verify_complete_witness(&CompleteWitnessVerifierRequest {
        context,
        scenario: &prepared.source.validated.flat_case.scenario,
        probe: &prepared.source.validated.flat_case.probe,
        candidate: &candidate_input.candidate,
        selected: &candidate_input.selected_profile,
        profile: &candidate_input.selected_profile.profile,
        source_pad: &prepared.source.validated.flat_case.probe.source,
        target_pad: &prepared.source.validated.flat_case.probe.target,
        policy: &prepared.source.validated.policy,
        row_index: binding.row_index,
        basis_candidate_identity: &binding.candidate_identity,
        source_bridge_tick_count: binding.source_bridge_tick_count,
        source_handoff: binding.source_handoff,
        launch: &analysis.launch,
        reseeded_bridge: &analysis.reseeded_bridge,
        schedule,
        scenario_identity: &scenario_identity,
        baseline_replay_parity: paired_row.baseline_pair.as_ref().is_some_and(|pair| {
            pair.both_match_frozen_canary
                && pair.direct_120_hz.replay_trace.passed
                && pair.held_60_hz.replay_trace.passed
        }),
        identity_bindings: CompleteWitnessIdentityBindings::Historical {
            source_duration_identity,
            paired_command_identity,
        },
    })
}

pub(in crate::waypoint_direct_nominal_plant) fn verify_complete_witness(
    request: &CompleteWitnessVerifierRequest<'_>,
) -> Result<CompleteFlatWrapperEvidence> {
    let context = request.context;
    let profile = request.profile;
    let schedule = request.schedule;
    let source_pad = request.source_pad;
    let target_pad = request.target_pad;
    let policy = request.policy;
    let row_index = request.row_index;
    let source_bridge_tick_count = request.source_bridge_tick_count;
    let generated = matches!(
        request.identity_bindings,
        CompleteWitnessIdentityBindings::Generated { .. }
    );
    let rollout = schedule
        .full_flight_rollout
        .as_ref()
        .ok_or_else(|| anyhow!("validated rollout for row {} disappeared", row_index))?;
    let launch = request.launch.clone();
    let reseeded_bridge = request.reseeded_bridge.clone();
    let run = LaunchFeasibilityCadenceRunEvidence {
        cadence: HELD_CADENCE.to_owned(),
        launch: launch.clone(),
        reseeded_bridge,
        rollout: rollout.clone(),
    };
    let stored_trace = schedule
        .full_flight_replay_trace
        .clone()
        .ok_or_else(|| anyhow!("stored replay trace for row {} disappeared", row_index))?;
    let replay = replay_logged_cadence(context, &run)?;
    let source_bounds = flat_pad_bounds(context, source_pad);
    let target_bounds = flat_pad_bounds(context, target_pad);
    let neutral = neutral_replay_and_clearance(
        context,
        &run,
        ClearancePolicy {
            source_pad: source_bounds,
            target_pad: target_bounds,
            minimum_clearance_m: policy.minimum_clearance_m,
        },
        NeutralSourceReference {
            source_commands: &schedule.commands,
            profile,
            source_bridge_tick_count,
            verify_generated_reference: generated,
        },
        &replay,
    )?;
    let first_contact = replay.first_contact.clone();
    let first_contact_margins = replay
        .first_contact_margins
        .as_ref()
        .map(public_contact_margins);
    let schedule_margins_match = schedule
        .full_flight_first_contact_margins
        .as_ref()
        .zip(first_contact_margins.as_ref())
        .is_some_and(|(stored, recomputed)| stored == recomputed);
    let stored_first_contact_matches = schedule.full_flight_first_contact == first_contact;
    let replay_trace_matches =
        stored_trace == replay.trace && stored_trace.passed && replay.trace.passed;
    let frozen_launch_matches =
        neutral.evidence.contiguous_from_step_one && run.launch == *request.launch;
    let source_position_error = if generated {
        neutral
            .source_handoff_state
            .as_ref()
            .map(|state| distance(state.position_m, request.source_handoff.position_m))
    } else {
        schedule.position_error_m
    };
    let source_velocity_error = if generated {
        neutral
            .source_handoff_state
            .as_ref()
            .map(|state| distance(state.velocity_mps, request.source_handoff.velocity_mps))
    } else {
        schedule.velocity_error_mps
    };
    let independently_matched_endpoint = source_endpoint_matches(
        neutral.source_handoff_state.as_ref(),
        schedule.endpoint_state.as_ref(),
    );
    let strict_handoff = schedule.screens.source_handoff_reached
        && schedule.screens.source_contact_free
        && source_position_error
            .is_some_and(|error| error.is_finite() && error <= STRICT_HANDOFF_TOLERANCE_M)
        && source_velocity_error
            .is_some_and(|error| error.is_finite() && error <= STRICT_HANDOFF_TOLERANCE_MPS)
        && (!generated || independently_matched_endpoint && neutral.phase_boundaries_match);
    let launch_to_source_join = launch_join_passed(&launch, &run, request.reseeded_bridge);
    let independent_source = if generated {
        let vehicle = super::super::vehicle_input_v2(&context.vehicle);
        Some(
            super::paired_command_feasibility::replay_source_schedule_screens_from_physical(
                &super::paired_command_feasibility::PhysicalScheduleScreenReplayRequest {
                    scenario: request.scenario,
                    probe: request.probe,
                    policy,
                    vehicle: &vehicle,
                    candidate: request.candidate,
                    selected: request.selected,
                    source_bridge_tick_count,
                    launch_tilt_attitude_rad: request.launch.commanded_tilt_attitude_rad,
                    commands: &schedule.commands,
                },
            )?,
        )
    } else {
        None
    };
    let independently_verified_source_screens = independent_source.as_ref().is_none_or(|source| {
        source.screens == schedule.screens
            && source.screens.passed
            && source.source_handoff_reached
            && source.source_handoff_contact_free
            && source.screens.strict_position_passed
            && source.screens.strict_velocity_passed
            && source.replay_trace.passed
            && source.launch == *request.launch
            && source.reseeded_bridge == *request.reseeded_bridge
    });
    let independent_prefix = independent_source.as_ref().is_none_or(|source| {
        source.source_rollout.per_step.as_slice()
            == rollout
                .per_step
                .get(..source.source_rollout.per_step.len())
                .unwrap_or_default()
            && source.source_rollout.per_step.len() as u64 == source_bridge_tick_count
            && source.source_handoff_position_error_m == source_position_error
            && source.source_handoff_velocity_error_mps == source_velocity_error
    });
    let source_screens = source_screens_passed(schedule) && independently_verified_source_screens;
    let prefix_parity = schedule.scheduled_source_prefix_parity.passed
        && schedule
            .scheduled_source_prefix_parity
            .rollout_prefix_matches
        && schedule
            .scheduled_source_prefix_parity
            .state_sample_prefix_matches
        && schedule
            .scheduled_source_prefix_parity
            .command_sample_prefix_matches
        && schedule
            .scheduled_source_prefix_parity
            .source_handoff_endpoint_matches
        && schedule
            .scheduled_source_prefix_parity
            .source_contacts_match
        && schedule
            .scheduled_source_prefix_parity
            .launch_and_bridge_match
        && (!generated || independently_matched_endpoint && neutral.phase_boundaries_match)
        && independent_prefix;
    let supported_rest = supported_source_pad_rest(context, source_pad, &source_bounds);
    let first_contact_is_stable_target = first_contact
        .as_ref()
        .is_some_and(|contact| contact.classification == "stable_touchdown_on_target")
        && first_contact_margins
            .as_ref()
            .is_some_and(stable_safe_margins_pass);
    let contact_step = neutral.evidence.first_contact_physics_step;
    let no_earlier_contact = neutral.evidence.no_contact_before_first_event
        && neutral.evidence.first_contact_classification.is_some()
        && neutral.evidence.first_contact_classification.as_deref()
            == first_contact
                .as_ref()
                .map(|contact| contact.classification.as_str());
    let terminal_entry_and_clear = terminal_entry_passed(
        context,
        neutral.terminal_entry_state.as_ref(),
        &neutral.clearance_scan,
    );
    let terminal_handoff_descending = frozen_terminal_handoff_descending(profile)
        && neutral
            .terminal_entry_state
            .as_ref()
            .is_some_and(|state| state.velocity_mps.y < 0.0);
    let actual_touchdown = first_contact.as_ref().map(|contact| &contact.state);
    let actual_touchdown_time = actual_touchdown.map(|state| state.sim_time_s);
    let actual_touchdown_fuel_remaining = actual_touchdown.map(|state| state.fuel_kg);
    let actual_touchdown_fuel_used =
        actual_touchdown.map(|state| context.vehicle.initial_fuel_kg - state.fuel_kg);
    let physics_hz = f64::from(context.sim.physics_hz);
    let planned_total_time = (LAUNCH_TICKS
        + source_bridge_tick_count
        + profile.accounting.coast_tick_count
        + profile.accounting.terminal_bridge_sample_count) as f64
        / physics_hz;
    let actual_budget_passed = actual_touchdown_time.is_some_and(|time| {
        time.is_finite()
            && time <= policy.maximum_mission_time_s
            && time <= context.sim.max_time_s
            && time <= policy.mission_budget_s()
    }) && actual_touchdown_fuel_remaining
        .is_some_and(|fuel| fuel.is_finite() && fuel >= 0.0)
        && actual_touchdown_fuel_used
            .is_some_and(|fuel| fuel.is_finite() && fuel <= context.vehicle.initial_fuel_kg)
        && rollout.saturation.below_minimum_saturation_count == 0
        && rollout.saturation.above_maximum_saturation_count == 0
        && rollout.saturation.fuel_burn_capped_tick_count == 0
        && rollout.saturation.fuel_exhausted_tick_count == 0
        && neutral.saturation.below_minimum_saturation_count == 0
        && neutral.saturation.above_maximum_saturation_count == 0
        && neutral.saturation.fuel_burn_capped_tick_count == 0
        && neutral.saturation.fuel_exhausted_tick_count == 0;
    let planned_time_passed =
        planned_total_time.is_finite() && planned_total_time <= policy.mission_budget_s();
    let replay_parity = replay_trace_matches
        && neutral.evidence.logged_command_schedule_matches
        && neutral.evidence.expected_thrust_reference_coverage
        && neutral.evidence.ordinary_and_neutral_states_match
        && neutral.evidence.poststep_contact_labels_match_log
        && neutral.evidence.first_contact_matches_authoritative_replay
        && stored_first_contact_matches
        && schedule_margins_match
        && frozen_launch_matches
        && request.baseline_replay_parity;
    let clearance_passed = neutral.clearance_scan.all_airborne_states_passed;
    let gates = CompleteFlatAcceptanceGatesEvidence {
        supported_source_pad_rest_state: supported_rest,
        launch_completed_and_contact_free: launch.completed
            && launch.contact_free
            && launch.physics_ticks_completed == LAUNCH_TICKS,
        launch_to_source_join_passed: launch_to_source_join,
        existing_source_screens_passed: source_screens,
        strict_source_handoff_passed: strict_handoff,
        scheduled_source_prefix_parity_passed: prefix_parity,
        ordinary_neutral_and_stored_replay_parity_passed: replay_parity,
        pointwise_core_geometry_clearance_passed: clearance_passed,
        terminal_entry_descending_and_clear: terminal_entry_and_clear,
        terminal_handoff_on_descending_arc: terminal_handoff_descending,
        first_contact_stable_safe_on_target: first_contact_is_stable_target,
        no_earlier_contact,
        fuel_time_and_commandability_budgets_passed: actual_budget_passed,
        planned_time_reserve_passed: planned_time_passed,
    };
    let failures = gate_failures(&gates);
    let accepted = failures.is_empty();
    let first_failing_gate = failures.first().map(|(name, _)| (*name).to_owned());
    let first_failing_tick = first_failing_gate.as_deref().and_then(|gate| match gate {
        "pointwise_core_geometry_clearance" => neutral
            .clearance_scan
            .first_violation
            .as_ref()
            .map(|violation| violation.physics_step),
        "ordinary_neutral_and_stored_replay_parity" => neutral
            .evidence
            .first_command_mismatch
            .as_ref()
            .and_then(|mismatch| mismatch.physics_step)
            .or(contact_step),
        "first_contact_stable_safe_on_target" | "no_earlier_contact" => contact_step,
        "terminal_entry_descending_and_clear" | "terminal_handoff_on_descending_arc" => neutral
            .terminal_entry_state
            .as_ref()
            .map(|state| state.physics_step),
        _ => None,
    });
    let scenario_identity = request.scenario_identity.to_owned();
    let old_policy_identity = stable_digest(policy)?;
    let acceptance_policy_identity = stable_digest(&AcceptancePolicyIdentityInput {
        frozen_policy: policy,
        policy_rule: "complete_flat_nominal_acceptance_policy_v1",
        geometry_convention: GEOMETRY_CONVENTION,
        minimum_clearance_rule: "core-rotated-feet-hull-aabb-exact-heightfield; source-or-descending-terminal-flat-pad-corridor-requires-nonpenetration; otherwise-declared-policy-minimum-clearance",
        source_corridor_phases: &["upright", "tilt", "source_bridge"],
        terminal_corridor_phase: "terminal_bridge_descending_only",
        handoff_position_tolerance_m: STRICT_HANDOFF_TOLERANCE_M,
        handoff_velocity_tolerance_mps: STRICT_HANDOFF_TOLERANCE_MPS,
        state_join_tolerance: STATE_JOIN_TOLERANCE,
        initial_rest_tolerance: INITIAL_REST_TOLERANCE,
        physics_ticks_per_held_command: COMMAND_HOLD_PHYSICS_TICKS,
    })?;
    let launch_schedule_identity = stable_digest(&launch)?;
    let source_command_schedule_identity = stable_digest(&schedule.commands)?;
    let tail_command_identity_rows = rollout
        .per_step
        .iter()
        .filter(|tick| tick.physics_step > LAUNCH_TICKS + source_bridge_tick_count)
        .map(logged_command_identity)
        .collect::<Vec<_>>();
    let tail_command_schedule_identity = stable_digest(&tail_command_identity_rows)?;
    let profile_identity = frozen_tail_profile_identity(profile)?;
    let (source_duration_identity, paired_command_identity, launch_rule) =
        match request.identity_bindings {
            CompleteWitnessIdentityBindings::Historical {
                source_duration_identity,
                paired_command_identity,
            } => (
                source_duration_identity.to_owned(),
                paired_command_identity.to_owned(),
                "frozen_72_tick_upright_then_tilt_source_pad_launch_v1",
            ),
            CompleteWitnessIdentityBindings::Generated {
                request_identity, ..
            } => (
                stable_digest(&(
                    "input_driven_source_program_v1",
                    request_identity,
                    request.basis_candidate_identity,
                    source_bridge_tick_count,
                    &launch,
                ))?,
                stable_digest(&(
                    "input_driven_paired_command_schedule_v1",
                    request_identity,
                    schedule,
                ))?,
                "bounded_72_tick_upright_then_tilt_source_pad_launch_v1",
            ),
        };
    let identity_input = WrapperIdentityInput {
        source_row_index: row_index,
        frozen_basis_identity: request.basis_candidate_identity,
        source_duration_identity: &source_duration_identity,
        paired_command_identity: &paired_command_identity,
        scenario_identity: &scenario_identity,
        policy_identity: &old_policy_identity,
        acceptance_policy_identity: &acceptance_policy_identity,
        launch_rule,
        cadence: HELD_CADENCE,
        geometry_convention: GEOMETRY_CONVENTION,
        launch_schedule_identity: &launch_schedule_identity,
        source_command_schedule_identity: &source_command_schedule_identity,
        tail_command_schedule_identity: &tail_command_schedule_identity,
        frozen_tail_profile_identity: &profile_identity,
    };
    let wrapper_identity = match request.identity_bindings {
        CompleteWitnessIdentityBindings::Historical { .. } => stable_digest(&identity_input)?,
        CompleteWitnessIdentityBindings::Generated {
            request_identity,
            generation_policy_identity,
        } => stable_digest(&(
            "input_driven_complete_nominal_direct_witness_v1",
            request_identity,
            generation_policy_identity,
            &identity_input,
        ))?,
    };
    let provenance = CompleteFlatWrapperProvenanceEvidence {
        source_row_index: row_index,
        frozen_basis_identity: request.basis_candidate_identity.to_owned(),
        source_duration_identity: source_duration_identity.to_owned(),
        paired_command_identity: paired_command_identity.to_owned(),
        scenario_identity: scenario_identity.clone(),
        policy_identity: old_policy_identity.clone(),
        acceptance_policy_identity: acceptance_policy_identity.clone(),
        launch_rule: identity_input.launch_rule.to_owned(),
        cadence: HELD_CADENCE.to_owned(),
        geometry_convention: GEOMETRY_CONVENTION.to_owned(),
        launch_schedule_identity: launch_schedule_identity.clone(),
        source_command_schedule_identity: source_command_schedule_identity.clone(),
        tail_command_schedule_identity: tail_command_schedule_identity.clone(),
        frozen_tail_profile_identity: profile_identity.clone(),
    };
    let rejection_reasons = failures
        .iter()
        .map(|(name, detail)| format!("{name}:{detail}"))
        .collect();
    Ok(CompleteFlatWrapperEvidence {
        wrapper_identity,
        provenance,
        source_handoff_position_error_m: source_position_error,
        source_handoff_velocity_error_mps: source_velocity_error,
        acceptance: gates,
        accepted,
        first_failing_gate,
        first_failing_tick,
        planned_total_mission_time_s: planned_total_time,
        actual_touchdown_time_s: actual_touchdown_time,
        actual_touchdown_fuel_remaining_kg: actual_touchdown_fuel_remaining,
        actual_touchdown_fuel_used_kg: actual_touchdown_fuel_used,
        first_contact,
        signed_first_contact_margins: first_contact_margins,
        minimum_airborne_clearance_m: neutral
            .clearance_scan
            .minimum_airborne
            .as_ref()
            .map(|minimum| minimum.clearance_m),
        minimum_airborne_clearance: neutral.clearance_scan.minimum_airborne.clone(),
        clearance_scan: neutral.clearance_scan,
        replay: CompleteFlatReplayEvidence {
            stored_replay_trace: Some(stored_trace),
            recomputed_replay_trace: Some(replay.trace),
            independent_neutral_replay: neutral.evidence,
            stored_first_contact_matches_replay: stored_first_contact_matches,
            stored_first_contact_margins_match_replay: schedule_margins_match,
            full_rollout_matches_frozen_launch_and_tail: frozen_launch_matches,
        },
        saturation: Some(rollout.saturation.clone()),
        recomputed_saturation: Some(neutral.saturation.clone()),
        rejection_reasons,
    })
}

fn source_screens_passed(schedule: &PairedScheduleEvidence) -> bool {
    let screens = &schedule.screens;
    screens.passed
        && screens.source_handoff_reached
        && screens.source_contact_free
        && screens.coupled_thrust_passed
        && screens.minimum_throttle_passed
        && screens.powered_slew_passed
        && screens.source_attitude_passed
        && screens.launch_boundary_passed
        && screens.source_clearance.all_samples_passed
        && screens.aggregate_fuel_passed
        && screens.aggregate_time_passed
        && screens.strict_position_passed
        && screens.strict_velocity_passed
}

fn source_endpoint_matches(
    actual: Option<&PlantStateEvidence>,
    stored: Option<&super::paired_command_feasibility::EndpointStateEvidence>,
) -> bool {
    actual.zip(stored).is_some_and(|(actual, stored)| {
        actual.physics_step == stored.physics_step
            && actual.sim_time_s == stored.sim_time_s
            && actual.position_m == stored.position_m
            && actual.velocity_mps == stored.velocity_mps
            && actual.attitude_rad == stored.attitude_rad
            && actual.angular_rate_radps == stored.angular_rate_radps
            && actual.fuel_kg == stored.fuel_kg
    })
}

fn launch_join_passed(
    launch: &LaunchEvidence,
    run: &LaunchFeasibilityCadenceRunEvidence,
    reseeded_bridge: &Option<super::super::launch_feasibility::ReseededBridgeEvidence>,
) -> bool {
    let Some(launch_end) = launch.end_state.as_ref() else {
        return false;
    };
    let Some(start) = reseeded_bridge
        .as_ref()
        .and_then(|bridge| bridge.start_state)
    else {
        return false;
    };
    let Some(first_source_tick) = run
        .rollout
        .per_step
        .iter()
        .find(|tick| tick.phase == "source_bridge")
    else {
        return false;
    };
    launch_end.physics_step == LAUNCH_TICKS
        && first_source_tick.physics_step == LAUNCH_TICKS + 1
        && distance(launch_end.position_m, start.position_m) <= STATE_JOIN_TOLERANCE
        && distance(launch_end.velocity_mps, start.velocity_mps) <= STATE_JOIN_TOLERANCE
        && first_source_tick.attitude_before_step_rad == launch_end.attitude_rad
}

fn supported_source_pad_rest(
    context: &RunContext,
    source_pad: &PadInputV2,
    bounds: &FlatPadBounds,
) -> bool {
    let initial = &context.initial_state;
    let initial_aabb = body_aabb_from_pose(
        initial.position_m,
        initial.attitude_rad,
        &context.vehicle.geometry,
    );
    bounds.flat
        && initial.attitude_rad.abs() <= INITIAL_REST_TOLERANCE
        && initial.angular_rate_radps.abs() <= INITIAL_REST_TOLERANCE
        && initial.velocity_mps.length() <= INITIAL_REST_TOLERANCE
        && (initial.position_m.x - source_pad.center_x_m).abs() <= INITIAL_REST_TOLERANCE
        && (initial.position_m.y
            - (source_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m))
            .abs()
            <= INITIAL_REST_TOLERANCE
        && body_within_pad(initial_aabb, *bounds)
}

fn flat_pad_bounds(context: &RunContext, pad: &PadInputV2) -> FlatPadBounds {
    let half = pad.width_m * 0.5;
    let left = pad.center_x_m - half;
    let right = pad.center_x_m + half;
    let terrain = &context.world.terrain;
    let flat = match (
        terrain.sample_height_strict(left),
        terrain.sample_height_strict(right),
    ) {
        (Ok(left_y), Ok(right_y)) => {
            (left_y - pad.surface_y_m).abs() <= INITIAL_REST_TOLERANCE
                && (right_y - pad.surface_y_m).abs() <= INITIAL_REST_TOLERANCE
                && terrain.points().iter().all(|point| {
                    point.x < left
                        || point.x > right
                        || (point.y - pad.surface_y_m).abs() <= INITIAL_REST_TOLERANCE
                })
        }
        _ => false,
    };
    FlatPadBounds {
        left_m: left,
        right_m: right,
        surface_y_m: pad.surface_y_m,
        flat,
    }
}

fn body_aabb(state: &SimulationState, geometry: &pd_core::VehicleGeometry) -> BodyAabb {
    body_aabb_from_pose(state.position_m, state.attitude_rad, geometry)
}

fn body_aabb_from_pose(
    position_m: Vec2,
    attitude_rad: f64,
    geometry: &pd_core::VehicleGeometry,
) -> BodyAabb {
    let half_w = geometry.hull_width_m * 0.5;
    let half_h = geometry.hull_height_m * 0.5;
    let feet = [
        position_m
            + Vec2::new(
                -geometry.touchdown_half_span_m,
                -geometry.touchdown_base_offset_m,
            )
            .rotated(attitude_rad),
        position_m
            + Vec2::new(
                geometry.touchdown_half_span_m,
                -geometry.touchdown_base_offset_m,
            )
            .rotated(attitude_rad),
    ];
    let hull = [
        Vec2::new(-half_w, -half_h),
        Vec2::new(half_w, -half_h),
        Vec2::new(half_w, half_h),
        Vec2::new(-half_w, half_h),
    ]
    .map(|point| position_m + point.rotated(attitude_rad));
    let all = feet.into_iter().chain(hull);
    let (mut x_min, mut x_max, mut y_min, mut y_max) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for point in all {
        x_min = x_min.min(point.x);
        x_max = x_max.max(point.x);
        y_min = y_min.min(point.y);
        y_max = y_max.max(point.y);
    }
    BodyAabb {
        horizontal_extent_m: (x_min - position_m.x)
            .abs()
            .max((x_max - position_m.x).abs()),
        vertical_extent_m: (y_min - position_m.y)
            .abs()
            .max((y_max - position_m.y).abs()),
        feet_x_min_m: feet[0].x.min(feet[1].x),
        feet_x_max_m: feet[0].x.max(feet[1].x),
        hull_x_min_m: hull
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min),
        hull_x_max_m: hull
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max),
    }
}

fn body_clearance(context: &RunContext, state: &SimulationState, aabb: BodyAabb) -> Result<f64> {
    let envelope = CorridorEnvelope::new(aabb.horizontal_extent_m, aabb.vertical_extent_m);
    let clearance = context
        .world
        .terrain
        .exact_point_clearance(state.position_m, envelope)
        .map_err(|error| anyhow!("exact body-envelope terrain query failed: {error}"))?;
    if !clearance.minimum_clearance_m.is_finite() {
        bail!("exact body-envelope clearance is non-finite");
    }
    Ok(clearance.minimum_clearance_m)
}

fn body_points_clear(context: &RunContext, state: &SimulationState) -> Result<bool> {
    let geometry = &context.vehicle.geometry;
    let aabb = body_aabb(state, geometry);
    let envelope_clearance = body_clearance(context, state, aabb)?;
    let foot_and_hull_points_clear =
        actual_body_points(state, geometry)
            .into_iter()
            .try_fold(true, |clear, point| {
                let terrain_y = context.world.terrain.sample_height_strict(point.x)?;
                Ok::<_, pd_core::terrain::TerrainQueryError>(clear && point.y - terrain_y > 0.0)
            })?;
    Ok(envelope_clearance > 0.0 && foot_and_hull_points_clear)
}

fn actual_body_points(state: &SimulationState, geometry: &pd_core::VehicleGeometry) -> Vec<Vec2> {
    let feet = [
        Vec2::new(
            -geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        ),
        Vec2::new(
            geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        ),
    ]
    .map(|point| state.position_m + point.rotated(state.attitude_rad));
    let half_w = geometry.hull_width_m * 0.5;
    let half_h = geometry.hull_height_m * 0.5;
    let hull = [
        Vec2::new(-half_w, -half_h),
        Vec2::new(half_w, -half_h),
        Vec2::new(half_w, half_h),
        Vec2::new(-half_w, half_h),
    ]
    .map(|point| state.position_m + point.rotated(state.attitude_rad));
    feet.into_iter().chain(hull).collect()
}

fn body_within_pad(aabb: BodyAabb, pad: FlatPadBounds) -> bool {
    pad.flat
        && aabb.feet_x_min_m >= pad.left_m
        && aabb.feet_x_max_m <= pad.right_m
        && aabb.hull_x_min_m >= pad.left_m
        && aabb.hull_x_max_m <= pad.right_m
}

fn corridor_for_step(
    phase: &str,
    velocity: Vec2,
    aabb: BodyAabb,
    source_pad: FlatPadBounds,
    target_pad: FlatPadBounds,
) -> &'static str {
    if matches!(phase, "upright" | "tilt" | "source_bridge") && body_within_pad(aabb, source_pad) {
        "source_pad_transition"
    } else if phase == "terminal_bridge" && velocity.y < 0.0 && body_within_pad(aabb, target_pad) {
        "descending_terminal_pad_transition"
    } else {
        "none"
    }
}

fn terminal_entry_passed(
    context: &RunContext,
    terminal_entry: Option<&PlantStateEvidence>,
    _clearance_scan: &GeometryClearanceScanEvidence,
) -> bool {
    let Some(entry) = terminal_entry else {
        return false;
    };
    if !entry.velocity_mps.y.is_finite() || entry.velocity_mps.y >= 0.0 {
        return false;
    }
    let mut state = match SimulationState::new(context) {
        Ok(state) => state,
        Err(_) => return false,
    };
    state.position_m = entry.position_m;
    state.velocity_mps = entry.velocity_mps;
    state.attitude_rad = entry.attitude_rad;
    state.angular_rate_radps = entry.angular_rate_radps;
    body_points_clear(context, &state).unwrap_or_default()
}

fn frozen_terminal_handoff_descending(profile: &NominalProfile) -> bool {
    frozen_terminal_handoff(profile).is_some_and(|state| state.velocity_mps.y < 0.0)
}

fn frozen_tail_profile_index(
    completed_physics_step: u64,
    source_bridge_tick_count: u64,
    tail_start: u64,
) -> Option<usize> {
    let profile_step_index = completed_physics_step.checked_sub(LAUNCH_TICKS + 1)?;
    let tail_offset = profile_step_index.checked_sub(source_bridge_tick_count)?;
    usize::try_from(tail_start.checked_add(tail_offset)?).ok()
}

fn frozen_terminal_handoff(profile: &NominalProfile) -> Option<KinematicStateV2> {
    let terminal_index = profile
        .ticks
        .iter()
        .position(|tick| tick.phase.as_str() == "terminal_bridge")?;
    terminal_index
        .checked_sub(1)
        .and_then(|index| profile.ticks.get(index))
        .map(|tick| tick.expected_state)
}

fn frozen_tail_profile_identity(profile: &NominalProfile) -> Result<String> {
    let start = usize::try_from(profile.accounting.source_bridge_sample_count)
        .map_err(|_| anyhow!("frozen tail profile start index exceeds usize"))?;
    let tail = profile
        .ticks
        .iter()
        .skip(start)
        .map(|tick| TailProfileIdentityTick {
            phase: tick.phase.as_str(),
            expected_state: tick.expected_state,
            thrust_acceleration_mps2: tick.thrust_acceleration_mps2,
            target_attitude_rad: tick.target_attitude_rad,
        })
        .collect::<Vec<_>>();
    stable_digest(&tail)
}

fn logged_command_identity(tick: &LaunchRolloutTickEvidence) -> LoggedCommandIdentityTick<'_> {
    LoggedCommandIdentityTick {
        physics_step: tick.physics_step,
        phase: &tick.phase,
        desired_target_attitude_rad: tick.desired_target_attitude_rad,
        held_target_attitude_rad: tick.held_target_attitude_rad,
        commanded_throttle_frac: tick.commanded_throttle_frac,
        applied_throttle_frac: tick.applied_throttle_frac,
    }
}

fn public_contact_margins(
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

fn stable_safe_margins_pass(margins: &ScheduledFirstContactMarginsEvidence) -> bool {
    [
        margins.stable_minimum_clearance_margin_m,
        margins.stable_maximum_clearance_margin_m,
        margins.stable_hull_penetration_margin_m,
        margins.safe_normal_speed_margin_mps,
        margins.safe_tangential_speed_margin_mps,
        margins.safe_attitude_margin_rad,
        margins.safe_angular_rate_margin_radps,
        margins.touchdown_pad_left_margin_m,
        margins.touchdown_pad_right_margin_m,
    ]
    .into_iter()
    .all(|margin| margin.is_finite() && margin >= 0.0)
}

fn gate_failures(gates: &CompleteFlatAcceptanceGatesEvidence) -> Vec<(&'static str, &'static str)> {
    [
        (
            "supported_source_pad_rest_state",
            gates.supported_source_pad_rest_state,
        ),
        (
            "launch_completed_and_contact_free",
            gates.launch_completed_and_contact_free,
        ),
        ("launch_to_source_join", gates.launch_to_source_join_passed),
        (
            "existing_source_screens",
            gates.existing_source_screens_passed,
        ),
        ("strict_source_handoff", gates.strict_source_handoff_passed),
        (
            "scheduled_source_prefix_parity",
            gates.scheduled_source_prefix_parity_passed,
        ),
        (
            "ordinary_neutral_and_stored_replay_parity",
            gates.ordinary_neutral_and_stored_replay_parity_passed,
        ),
        (
            "pointwise_core_geometry_clearance",
            gates.pointwise_core_geometry_clearance_passed,
        ),
        (
            "terminal_entry_descending_and_clear",
            gates.terminal_entry_descending_and_clear,
        ),
        (
            "terminal_handoff_on_descending_arc",
            gates.terminal_handoff_on_descending_arc,
        ),
        (
            "first_contact_stable_safe_on_target",
            gates.first_contact_stable_safe_on_target,
        ),
        ("no_earlier_contact", gates.no_earlier_contact),
        (
            "fuel_time_and_commandability_budgets",
            gates.fuel_time_and_commandability_budgets_passed,
        ),
        ("planned_time_reserve", gates.planned_time_reserve_passed),
    ]
    .into_iter()
    .filter_map(|(gate, passed)| (!passed).then_some((gate, "declared acceptance gate failed")))
    .collect()
}

fn distance(left: Vec2, right: Vec2) -> f64 {
    (left - right).length()
}

fn contact_replay_regression(
    rows: &[CompleteFlatAcceptanceRowEvidence],
    paired: &WaypointDirectSourceDurationPairedCommandFeasibilityArtifact,
) -> Result<ContactReplayRegressionEvidence> {
    let expected_stable_target_rows = EXPECTED_STABLE_ROWS.to_vec();
    let expected_crash_rows = EXPECTED_CRASH_ROWS.to_vec();
    let observed_stable_target_rows = rows
        .iter()
        .filter_map(|row| {
            row.wrapper
                .as_ref()
                .and_then(|wrapper| {
                    wrapper
                        .replay
                        .independent_neutral_replay
                        .first_contact_classification
                        .as_deref()
                })
                .is_some_and(|class| class == "stable_touchdown_on_target")
                .then_some(row.row_index)
        })
        .collect::<Vec<_>>();
    let observed_crash_rows = rows
        .iter()
        .filter_map(|row| {
            row.wrapper
                .as_ref()
                .and_then(|wrapper| {
                    wrapper
                        .replay
                        .independent_neutral_replay
                        .first_contact_classification
                        .as_deref()
                })
                .is_some_and(|class| class == "crash")
                .then_some(row.row_index)
        })
        .collect::<Vec<_>>();
    let wrappers = rows
        .iter()
        .filter_map(|row| row.wrapper.as_ref())
        .collect::<Vec<_>>();
    let replay_parity_row_count = wrappers
        .iter()
        .filter(|wrapper| {
            wrapper
                .replay
                .independent_neutral_replay
                .contiguous_from_step_one
                && wrapper
                    .replay
                    .independent_neutral_replay
                    .logged_command_schedule_matches
                && wrapper
                    .replay
                    .independent_neutral_replay
                    .ordinary_and_neutral_states_match
                && wrapper
                    .replay
                    .independent_neutral_replay
                    .poststep_contact_labels_match_log
                && wrapper
                    .replay
                    .independent_neutral_replay
                    .first_contact_matches_authoritative_replay
                && wrapper
                    .replay
                    .stored_replay_trace
                    .as_ref()
                    .is_some_and(|trace| trace.passed)
                && wrapper
                    .replay
                    .recomputed_replay_trace
                    .as_ref()
                    .is_some_and(|trace| trace.passed)
                && wrapper.replay.stored_first_contact_matches_replay
                && wrapper.replay.stored_first_contact_margins_match_replay
        })
        .count();
    let all_stored_replays_passed = rows.iter().all(|row| {
        if !row.analytical_survivor {
            return row.wrapper.is_none();
        }
        row.wrapper.as_ref().is_some_and(|wrapper| {
            wrapper
                .replay
                .stored_replay_trace
                .as_ref()
                .is_some_and(|trace| trace.passed)
                && wrapper
                    .replay
                    .recomputed_replay_trace
                    .as_ref()
                    .is_some_and(|trace| trace.passed)
        })
    }) && paired
        .rows
        .iter()
        .filter(|row| row.analytical_survivor)
        .all(|row| {
            row.paired_schedule.as_ref().is_some_and(|schedule| {
                schedule
                    .full_flight_replay_trace
                    .as_ref()
                    .is_some_and(|trace| trace.passed)
                    && schedule.full_flight_first_contact.is_some()
            })
        });
    let all_expected_contact_classes_reproduced = observed_stable_target_rows
        == expected_stable_target_rows
        && observed_crash_rows == expected_crash_rows;
    let expected_contact_row_count = expected_stable_target_rows.len() + expected_crash_rows.len();
    let passed = wrappers.len() == EXPECTED_SCHEDULES
        && replay_parity_row_count == EXPECTED_SCHEDULES
        && all_expected_contact_classes_reproduced
        && all_stored_replays_passed;
    Ok(ContactReplayRegressionEvidence {
        expected_stable_target_rows,
        observed_stable_target_rows,
        expected_crash_rows,
        observed_crash_rows,
        replay_parity_row_count,
        expected_contact_row_count,
        all_expected_contact_classes_reproduced,
        all_stored_replays_passed,
        passed,
    })
}

fn select_accepted_wrapper(
    rows: &[CompleteFlatAcceptanceRowEvidence],
    flat_gate_passed: bool,
) -> Option<AcceptedWrapperSelectionEvidence> {
    if !flat_gate_passed {
        return None;
    }
    rows.iter()
        .filter_map(|row| {
            row.wrapper
                .as_ref()
                .filter(|wrapper| wrapper.accepted && wrapper.planned_total_mission_time_s.is_finite())
                .map(|wrapper| (row.row_index, wrapper))
        })
        .min_by(|(_, left), (_, right)| {
            left.planned_total_mission_time_s
                .total_cmp(&right.planned_total_mission_time_s)
                .then_with(|| left.wrapper_identity.cmp(&right.wrapper_identity))
        })
        .map(|(row_index, wrapper)| AcceptedWrapperSelectionEvidence {
            row_index,
            wrapper_identity: wrapper.wrapper_identity.clone(),
            planned_total_mission_time_s: wrapper.planned_total_mission_time_s,
            selection_rule: "accepted wrappers only; planned total mission time ascending; stable wrapper identity ascending for exact ties".to_owned(),
        })
}

fn protocol_evidence() -> CompleteFlatAcceptanceProtocolEvidence {
    CompleteFlatAcceptanceProtocolEvidence {
        source_input_rule: "Rebuild the eight sealed source inputs, including the source-duration summary; with the paired-command summary there are nine checked input artifacts. Retain the exact 3 x 5 basis, six analytical skips, and nine survivor rows.".to_owned(),
        paired_input_rule: "Recompute the paired-command artifact identity with its identity field cleared and require the pinned run_d/run_e identity; verify schema, upstream gates, every row binding, stored replay provenance, source command joins, and unchanged tail phase/reference direction before any SimulationState exists.".to_owned(),
        replay_rule: "Reconstruct each flight from its frozen 72-tick launch, reseeded bridge evidence, and paired full-flight command log; replay at 120 Hz physics with commands held for two physics ticks, checking ordinary and neutral state/contact parity.".to_owned(),
        geometry_rule: "For every airborne post-step before first contact, use the unchanged core-current rotated feet and hull to form a world-axis body AABB and query exact point clearance over the complete envelope; domain overrun is a failure.".to_owned(),
        corridor_rule: "Use zero required clearance only for the upright, tilt, or source-bridge phase when feet and hull horizontal bounds are wholly within the flat source pad, or for descending terminal-bridge states wholly within the flat target pad; every other airborne state requires the frozen policy minimum clearance.".to_owned(),
        contact_rule: "The core ContactClassification enum owns the first-contact result. Require StableTouchdown on target and retain all signed first-contact predicate margins; at that step apply the existing core contact tolerances instead of the airborne reserve.".to_owned(),
        selection_rule: "Retain all fifteen rows. Rank only complete accepted wrappers by planned total mission time, then stable wrapper identity; an early contact time is never a ranking key.".to_owned(),
        evidence_classification: "nominal_replay_validated_direct".to_owned(),
        non_claim: "Discrete pointwise nominal replay is not a continuous swept-path proof, perturbation-robustness guarantee, V2 Certified result, or physical-orientation reconciliation.".to_owned(),
    }
}

fn paired_command_artifact_identity(
    artifact: &WaypointDirectSourceDurationPairedCommandFeasibilityArtifact,
) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

fn acceptance_artifact_identity(
    artifact: &WaypointDirectCompleteFlatAcceptanceArtifact,
) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

fn write_artifact(
    output_dir: &Path,
    summary_path: &Path,
    artifact: &WaypointDirectCompleteFlatAcceptanceArtifact,
) -> Result<()> {
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create complete-flat-acceptance output directory {}",
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
                "complete flat acceptance refuses to overwrite summary {}",
                summary_path.display()
            )
        })?;
    file.write_all(&bytes).with_context(|| {
        format!(
            "failed to write complete flat acceptance summary {}",
            summary_path.display()
        )
    })?;
    drop(file);
    let reloaded: WaypointDirectCompleteFlatAcceptanceArtifact = serde_json::from_slice(&bytes)
        .context("failed to reload complete flat acceptance summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != bytes {
        bail!("complete flat acceptance summary is not byte-stable after reload");
    }
    if acceptance_artifact_identity(&reloaded)? != reloaded.identity {
        bail!("complete flat acceptance semantic identity failed round-trip check");
    }
    Ok(())
}

fn replay_log_ticks(run: &LaunchFeasibilityCadenceRunEvidence) -> Vec<ReplayLogTick<'_>> {
    let mut logs = Vec::with_capacity(run.launch.samples.len() + run.rollout.per_step.len());
    logs.extend(run.launch.samples.iter().map(|tick| ReplayLogTick {
        physics_step: tick.physics_step,
        phase: &tick.phase,
        expected_contact: &tick.contact_classification,
        desired_target_attitude_rad: tick.commanded_target_attitude_rad,
        held_target_attitude_rad: tick.held_target_attitude_rad,
        commanded_throttle_frac: tick.commanded_throttle_frac,
        attitude_before_step_rad: tick.attitude_before_step_rad,
        logged_applied_throttle_frac: None,
    }));
    logs.extend(run.rollout.per_step.iter().map(|tick| ReplayLogTick {
        physics_step: tick.physics_step,
        phase: &tick.phase,
        expected_contact: &tick.contact_classification,
        desired_target_attitude_rad: tick.desired_target_attitude_rad,
        held_target_attitude_rad: tick.held_target_attitude_rad,
        commanded_throttle_frac: tick.commanded_throttle_frac,
        attitude_before_step_rad: tick.attitude_before_step_rad,
        logged_applied_throttle_frac: Some(tick.applied_throttle_frac),
    }));
    logs
}

fn empty_clearance_scan() -> GeometryClearanceScanEvidence {
    GeometryClearanceScanEvidence {
        poststep_state_count: 0,
        airborne_state_count: 0,
        source_corridor_state_count: 0,
        terminal_corridor_state_count: 0,
        exact_clearance_query_count: 0,
        all_airborne_states_passed: true,
        first_violation: None,
        minimum_airborne: None,
    }
}

fn neutral_replay_and_clearance(
    context: &RunContext,
    run: &LaunchFeasibilityCadenceRunEvidence,
    clearance_policy: ClearancePolicy,
    source_reference: NeutralSourceReference<'_>,
    authoritative_replay: &ReplayTraceResult,
) -> Result<NeutralReplayOutcome> {
    let NeutralSourceReference {
        source_commands,
        profile,
        source_bridge_tick_count,
        verify_generated_reference,
    } = source_reference;
    let logs = replay_log_ticks(run);
    let mut ordinary = SimulationState::new(context)?;
    let mut neutral = SimulationState::new(context)?;
    let mut contiguous = true;
    let mut commands_match = true;
    let mut states_match = true;
    let mut contacts_match_log = true;
    let mut first_contact_physics_step = None;
    let mut first_contact_classification = None;
    let mut first_contact_state = None;
    let mut terminal_entry_state = None;
    let mut source_handoff_state = None;
    let mut phase_boundaries_match = true;
    let mut first_mismatch = None;
    let mut expected_thrust_reference_coverage = true;
    let mut recomputed_saturation = empty_saturation();
    let mut scan = empty_clearance_scan();
    let interval = context.sim.control_interval_steps();
    for tick in &logs {
        if ordinary.is_terminal() || neutral.is_terminal() {
            contiguous = false;
            record_replay_mismatch(
                &mut first_mismatch,
                Some(tick.physics_step),
                "trace_after_terminal",
                "no further logged tick",
                "logged tick after ordinary terminal state",
            );
            break;
        }
        if tick.physics_step != ordinary.physics_step + 1
            || tick.physics_step != neutral.physics_step + 1
        {
            contiguous = false;
            record_replay_mismatch(
                &mut first_mismatch,
                Some(tick.physics_step),
                "physics_step",
                (ordinary.physics_step + 1).to_string(),
                tick.physics_step.to_string(),
            );
            break;
        }
        if tick.phase == "terminal_bridge" && terminal_entry_state.is_none() {
            terminal_entry_state = Some(plant_state_evidence(&neutral, context));
        }
        let expected_phase = if tick.physics_step <= 60 {
            "upright"
        } else if tick.physics_step <= LAUNCH_TICKS {
            "tilt"
        } else if tick.physics_step <= LAUNCH_TICKS + source_bridge_tick_count {
            "source_bridge"
        } else {
            frozen_tail_profile_index(
                tick.physics_step,
                source_bridge_tick_count,
                profile.accounting.source_bridge_sample_count,
            )
            .and_then(|index| profile.ticks.get(index))
            .map_or("missing_tail_reference", |tick| tick.phase.as_str())
        };
        if tick.phase != expected_phase {
            phase_boundaries_match = false;
        }
        let cadence = RolloutCadence::ControllerCadence;
        let (throttle_update_due, attitude_update_due) = component_update_schedule(
            SourceDurationHoldMode::Together,
            cadence,
            ordinary.physics_step,
            interval,
        );
        if throttle_update_due || attitude_update_due {
            recomputed_saturation.commanded_update_count += 1;
        }
        if throttle_update_due || attitude_update_due {
            let ordinary_held = ordinary.held_command;
            let neutral_held = neutral.held_command;
            let command = Command {
                throttle_frac: if throttle_update_due {
                    tick.commanded_throttle_frac
                } else {
                    ordinary_held.throttle_frac
                },
                target_attitude_rad: if attitude_update_due {
                    tick.desired_target_attitude_rad
                } else {
                    ordinary_held.target_attitude_rad
                },
            };
            ordinary.set_command(command);
            neutral.set_command(Command {
                throttle_frac: if throttle_update_due {
                    tick.commanded_throttle_frac
                } else {
                    neutral_held.throttle_frac
                },
                target_attitude_rad: if attitude_update_due {
                    tick.desired_target_attitude_rad
                } else {
                    neutral_held.target_attitude_rad
                },
            });
        }
        let pre_step_fuel_kg = neutral.fuel_kg;
        let expected_thrust =
            expected_thrust_acceleration(tick, source_commands, profile, source_bridge_tick_count);
        let expected_thrust = match expected_thrust {
            Ok(acceleration) => acceleration,
            Err(error) => {
                expected_thrust_reference_coverage = false;
                record_replay_mismatch(
                    &mut first_mismatch,
                    Some(tick.physics_step),
                    "expected_thrust_reference",
                    "complete frozen source/tail reference",
                    error.to_string(),
                );
                None
            }
        };
        if let Some(acceleration) = expected_thrust
            && acceleration.length() > 0.0
        {
            let desired = super::super::throttle_request(
                acceleration.length(),
                neutral.mass_kg(context),
                context.vehicle.max_thrust_n,
                context.vehicle.max_fuel_burn_kgps,
                context.sim.physics_dt_s(),
                context.vehicle.min_throttle_frac,
            )?;
            use super::super::ThrottleSaturation;
            match desired.saturation {
                ThrottleSaturation::BelowMinimum => {
                    recomputed_saturation.below_minimum_saturation_count += 1;
                }
                ThrottleSaturation::AboveMaximum => {
                    recomputed_saturation.above_maximum_saturation_count += 1;
                }
                ThrottleSaturation::ExactMinimumOnCommand => {
                    recomputed_saturation.on_at_exact_minimum_count += 1;
                }
                ThrottleSaturation::None => {}
            }
            if verify_generated_reference
                && throttle_update_due
                && neutral.held_command.throttle_frac != desired.command_fraction
            {
                commands_match = false;
                record_replay_mismatch(
                    &mut first_mismatch,
                    Some(tick.physics_step),
                    "generated_throttle_reference",
                    desired.command_fraction.to_string(),
                    neutral.held_command.throttle_frac.to_string(),
                );
            }
        }
        if verify_generated_reference {
            let reference_target = if tick.physics_step <= 60 {
                Some(0.0)
            } else if tick.physics_step <= LAUNCH_TICKS {
                Some(run.launch.commanded_tilt_attitude_rad)
            } else if tick.physics_step <= LAUNCH_TICKS + source_bridge_tick_count {
                let pair_index =
                    (tick.physics_step - LAUNCH_TICKS - 1) / COMMAND_HOLD_PHYSICS_TICKS;
                source_commands
                    .get(pair_index as usize)
                    .filter(|command| {
                        command.pair_index == pair_index as usize
                            && command.first_physics_step
                                == LAUNCH_TICKS + 1 + pair_index * COMMAND_HOLD_PHYSICS_TICKS
                            && command.last_physics_step == command.first_physics_step + 1
                            && command.target_attitude_rad
                                == command
                                    .desired_thrust_acceleration_mps2
                                    .x
                                    .atan2(command.desired_thrust_acceleration_mps2.y)
                    })
                    .map(|command| command.target_attitude_rad)
            } else {
                frozen_tail_profile_index(
                    tick.physics_step,
                    source_bridge_tick_count,
                    profile.accounting.source_bridge_sample_count,
                )
                .and_then(|index| profile.ticks.get(index))
                .map(|tick| tick.target_attitude_rad)
            };
            if reference_target != Some(tick.desired_target_attitude_rad)
                || tick.physics_step <= LAUNCH_TICKS && tick.commanded_throttle_frac != 1.0
                || expected_thrust.is_some_and(|thrust| thrust.length() == 0.0)
                    && throttle_update_due
                    && tick.commanded_throttle_frac != 0.0
            {
                commands_match = false;
                record_replay_mismatch(
                    &mut first_mismatch,
                    Some(tick.physics_step),
                    "generated_phase_command_reference",
                    format!("{reference_target:?}"),
                    tick.desired_target_attitude_rad.to_string(),
                );
            }
        }
        let actual_applied_throttle = super::super::plant_applied_throttle(
            neutral.held_command,
            context.vehicle.min_throttle_frac,
            pre_step_fuel_kg,
        );
        let fuel_burn_budget = context.vehicle.max_fuel_burn_kgps
            * actual_applied_throttle
            * context.sim.physics_dt_s();
        if neutral.held_command.throttle_frac > 0.0 {
            if pre_step_fuel_kg <= 0.0 {
                recomputed_saturation.fuel_exhausted_tick_count += 1;
            } else if fuel_burn_budget > pre_step_fuel_kg + f64::EPSILON {
                recomputed_saturation.fuel_burn_capped_tick_count += 1;
            }
        }
        let command_match = ordinary.held_command.throttle_frac == tick.commanded_throttle_frac
            && neutral.held_command.throttle_frac == tick.commanded_throttle_frac
            && ordinary.held_command.target_attitude_rad == tick.held_target_attitude_rad
            && neutral.held_command.target_attitude_rad == tick.held_target_attitude_rad
            && ordinary.attitude_rad == tick.attitude_before_step_rad
            && neutral.attitude_rad == tick.attitude_before_step_rad
            && tick
                .logged_applied_throttle_frac
                .is_none_or(|logged| logged == actual_applied_throttle);
        if !command_match {
            commands_match = false;
            record_replay_mismatch(
                &mut first_mismatch,
                Some(tick.physics_step),
                "held_command_or_pre_step_attitude",
                format!(
                    "{} @ {} before {}",
                    tick.commanded_throttle_frac,
                    tick.held_target_attitude_rad,
                    tick.attitude_before_step_rad
                ),
                format!(
                    "{} @ {} before {}",
                    ordinary.held_command.throttle_frac,
                    ordinary.held_command.target_attitude_rad,
                    ordinary.attitude_rad
                ),
            );
        }
        let neutral_classification = neutral.step_physics_and_classify_contact(context);
        let neutral_label = contact_classification_label(&neutral_classification);
        let neutral_state = plant_state_evidence(&neutral, context);
        if tick.physics_step == LAUNCH_TICKS + source_bridge_tick_count {
            source_handoff_state = Some(neutral_state.clone());
        }
        scan.poststep_state_count += 1;
        let actual_contact = neutral_label != "none";
        if actual_contact && first_contact_physics_step.is_none() {
            first_contact_physics_step = Some(tick.physics_step);
            first_contact_classification = Some(neutral_label.to_owned());
            first_contact_state = Some(neutral_state.clone());
        }
        if !actual_contact {
            record_airborne_clearance(
                context,
                &neutral,
                tick.physics_step,
                tick.phase,
                clearance_policy,
                &mut scan,
            );
        }
        let events = ordinary.step(context);
        let ordinary_label = event_contact_label(&events);
        if ordinary_label != neutral_label {
            states_match = false;
            record_replay_mismatch(
                &mut first_mismatch,
                Some(tick.physics_step),
                "ordinary_event_vs_neutral_contact",
                neutral_label,
                ordinary_label,
            );
        }
        if neutral_label != tick.expected_contact {
            contacts_match_log = false;
            record_replay_mismatch(
                &mut first_mismatch,
                Some(tick.physics_step),
                "poststep_contact_label",
                tick.expected_contact,
                neutral_label,
            );
        }
        if !same_ordinary_neutral_state(&ordinary, &neutral, &neutral_classification) {
            states_match = false;
            record_replay_mismatch(
                &mut first_mismatch,
                Some(tick.physics_step),
                "ordinary_neutral_poststep_state",
                "matching kinematic state and fuel",
                "state diverged",
            );
        }
        if actual_contact {
            break;
        }
    }
    let no_contact_before_first_event = if let Some(first_step) = first_contact_physics_step {
        logs.iter()
            .filter(|tick| tick.physics_step < first_step)
            .all(|tick| tick.expected_contact == "none")
    } else {
        logs.iter().all(|tick| tick.expected_contact == "none")
    };
    let authoritative_first_contact = authoritative_replay.first_contact.as_ref();
    let first_contact_matches_authoritative = match (
        first_contact_physics_step,
        first_contact_classification.as_deref(),
        first_contact_state.as_ref(),
        authoritative_first_contact,
    ) {
        (None, None, None, None) => true,
        (Some(step), Some(classification), Some(state), Some(authoritative)) => {
            step == authoritative.physics_step
                && classification == authoritative.classification
                && state.physics_step == authoritative.state.physics_step
                && state.position_m == authoritative.state.position_m
                && state.velocity_mps == authoritative.state.velocity_mps
                && state.attitude_rad == authoritative.state.attitude_rad
                && state.angular_rate_radps == authoritative.state.angular_rate_radps
                && state.fuel_kg == authoritative.state.fuel_kg
        }
        _ => false,
    };
    let evidence = NeutralReplayEvidence {
        physics_ticks_replayed: scan.poststep_state_count,
        contiguous_from_step_one: contiguous && scan.poststep_state_count == logs.len() as u64,
        logged_command_schedule_matches: commands_match,
        expected_thrust_reference_coverage,
        ordinary_and_neutral_states_match: states_match,
        poststep_contact_labels_match_log: contacts_match_log,
        no_contact_before_first_event,
        first_contact_physics_step,
        first_contact_classification,
        first_contact_state,
        first_contact_matches_authoritative_replay: first_contact_matches_authoritative,
        first_command_mismatch: first_mismatch,
    };
    let first_contact = authoritative_replay.first_contact.clone();
    Ok(NeutralReplayOutcome {
        evidence,
        clearance_scan: scan,
        terminal_entry_state,
        first_contact,
        first_contact_margins: authoritative_replay.first_contact_margins.clone(),
        saturation: recomputed_saturation,
        source_handoff_state,
        phase_boundaries_match,
    })
}

fn expected_thrust_acceleration(
    tick: &ReplayLogTick<'_>,
    source_commands: &[super::paired_command_feasibility::HeldCommandEvidence],
    profile: &NominalProfile,
    source_bridge_tick_count: u64,
) -> Result<Option<Vec2>> {
    if tick.phase == "upright" || tick.phase == "tilt" {
        return Ok(None);
    }
    if tick.phase == "source_bridge" {
        let first_source_step = LAUNCH_TICKS + 1;
        let pair_index = tick
            .physics_step
            .checked_sub(first_source_step)
            .ok_or_else(|| anyhow!("source command step precedes source bridge"))?
            / COMMAND_HOLD_PHYSICS_TICKS;
        let command = source_commands
            .get(pair_index as usize)
            .ok_or_else(|| anyhow!("source command index {pair_index} is missing"))?;
        return Ok(Some(command.desired_thrust_acceleration_mps2));
    }
    let flight_index = tick
        .physics_step
        .checked_sub(LAUNCH_TICKS + 1)
        .ok_or_else(|| anyhow!("tail step precedes launch completion"))?;
    let source_end_index = source_bridge_tick_count;
    if flight_index < source_end_index {
        bail!(
            "non-source phase {} occurs before source bridge completes",
            tick.phase
        );
    }
    let offset = flight_index - source_end_index;
    let profile_index = profile
        .accounting
        .source_bridge_sample_count
        .checked_add(offset)
        .ok_or_else(|| anyhow!("tail reference index overflowed"))?;
    let profile_index = usize::try_from(profile_index)
        .map_err(|_| anyhow!("tail reference index exceeds usize"))?;
    let profile_tick = profile.ticks.get(profile_index).ok_or_else(|| {
        anyhow!(
            "tail reference ended before logged step {}",
            tick.physics_step
        )
    })?;
    if profile_tick.phase.as_str() != tick.phase {
        bail!(
            "tail phase {} does not match frozen {}",
            tick.phase,
            profile_tick.phase.as_str()
        );
    }
    Ok(Some(profile_tick.thrust_acceleration_mps2))
}

fn empty_saturation() -> CommandSaturationEvidence {
    CommandSaturationEvidence {
        commanded_update_count: 0,
        below_minimum_saturation_count: 0,
        above_maximum_saturation_count: 0,
        on_at_exact_minimum_count: 0,
        fuel_burn_capped_tick_count: 0,
        fuel_exhausted_tick_count: 0,
    }
}

fn record_airborne_clearance(
    context: &RunContext,
    state: &SimulationState,
    physics_step: u64,
    phase: &str,
    policy: ClearancePolicy,
    scan: &mut GeometryClearanceScanEvidence,
) {
    scan.airborne_state_count += 1;
    scan.exact_clearance_query_count += 1;
    let aabb = body_aabb(state, &context.vehicle.geometry);
    let corridor = corridor_for_step(
        phase,
        state.velocity_mps,
        aabb,
        policy.source_pad,
        policy.target_pad,
    );
    let required = if corridor == "none" {
        policy.minimum_clearance_m
    } else {
        match corridor {
            "source_pad_transition" => scan.source_corridor_state_count += 1,
            "descending_terminal_pad_transition" => scan.terminal_corridor_state_count += 1,
            _ => {}
        }
        0.0
    };
    match body_clearance(context, state, aabb) {
        Ok(clearance) => {
            let sample = AirborneClearanceMinimumEvidence {
                physics_step,
                phase: phase.to_owned(),
                clearance_m: clearance,
                required_clearance_m: required,
                corridor: corridor.to_owned(),
            };
            if scan
                .minimum_airborne
                .as_ref()
                .is_none_or(|minimum| clearance < minimum.clearance_m)
            {
                scan.minimum_airborne = Some(sample);
            }
            if clearance < required {
                scan.all_airborne_states_passed = false;
                if scan.first_violation.is_none() {
                    scan.first_violation = Some(GeometryClearanceViolationEvidence {
                        physics_step,
                        phase: phase.to_owned(),
                        reason: "actual core-rotated body envelope is below the declared clearance"
                            .to_owned(),
                        clearance_m: Some(clearance),
                        required_clearance_m: required,
                        corridor: corridor.to_owned(),
                    });
                }
            }
        }
        Err(error) => {
            scan.all_airborne_states_passed = false;
            if scan.first_violation.is_none() {
                scan.first_violation = Some(GeometryClearanceViolationEvidence {
                    physics_step,
                    phase: phase.to_owned(),
                    reason: error.to_string(),
                    clearance_m: None,
                    required_clearance_m: required,
                    corridor: corridor.to_owned(),
                });
            }
        }
    }
}

fn plant_state_evidence(state: &SimulationState, context: &RunContext) -> PlantStateEvidence {
    PlantStateEvidence {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        position_m: state.position_m,
        velocity_mps: state.velocity_mps,
        attitude_rad: state.attitude_rad,
        angular_rate_radps: state.angular_rate_radps,
        fuel_kg: state.fuel_kg,
        fuel_used_kg: context.vehicle.initial_fuel_kg - state.fuel_kg,
        physical_outcome: super::super::enum_label(&state.physical_outcome),
        mission_outcome: super::super::enum_label(&state.mission_outcome),
        end_reason: super::super::enum_label(&state.end_reason),
    }
}

fn same_ordinary_neutral_state(
    ordinary: &SimulationState,
    neutral: &SimulationState,
    classification: &ContactClassification,
) -> bool {
    let common = ordinary.physics_step == neutral.physics_step
        && ordinary.sim_time_s == neutral.sim_time_s
        && ordinary.position_m == neutral.position_m
        && ordinary.attitude_rad == neutral.attitude_rad
        && ordinary.fuel_kg == neutral.fuel_kg
        && ordinary.min_touchdown_clearance_m == neutral.min_touchdown_clearance_m
        && ordinary.min_hull_clearance_m == neutral.min_hull_clearance_m;
    match classification {
        ContactClassification::StableTouchdown { .. } => {
            common
                && ordinary.velocity_mps == Vec2::new(0.0, 0.0)
                && ordinary.angular_rate_radps == 0.0
        }
        ContactClassification::None | ContactClassification::Crash => {
            common
                && ordinary.velocity_mps == neutral.velocity_mps
                && ordinary.angular_rate_radps == neutral.angular_rate_radps
        }
    }
}

fn contact_classification_label(classification: &ContactClassification) -> &'static str {
    match classification {
        ContactClassification::None => "none",
        ContactClassification::StableTouchdown { on_target: true } => "stable_touchdown_on_target",
        ContactClassification::StableTouchdown { on_target: false } => {
            "stable_touchdown_off_target"
        }
        ContactClassification::Crash => "crash",
    }
}

fn event_contact_label(events: &[pd_core::EventRecord]) -> &'static str {
    for event in events {
        match event.kind {
            EventKind::TouchdownOnTarget => return "stable_touchdown_on_target",
            EventKind::TouchdownOffTarget => return "stable_touchdown_off_target",
            EventKind::Crash => return "crash",
            _ => {}
        }
    }
    "none"
}

fn record_replay_mismatch(
    first: &mut Option<ReplayValidationMismatchEvidence>,
    physics_step: Option<u64>,
    field: impl Into<String>,
    expected: impl Into<String>,
    actual: impl Into<String>,
) {
    if first.is_none() {
        *first = Some(ReplayValidationMismatchEvidence {
            physics_step,
            field: field.into(),
            expected: expected.into(),
            actual: actual.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use pd_core::{
        EvaluationGoal, LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig, TerrainDefinition,
        VehicleGeometry, VehicleInitialState, VehicleSpec, WorldSpec,
    };

    pub(super) fn test_context() -> RunContext {
        RunContext::from_scenario(&ScenarioSpec {
            id: "complete-flat-acceptance-test".to_owned(),
            name: "Complete flat acceptance test".to_owned(),
            description: "synthetic unit-test context".to_owned(),
            seed: 1,
            tags: vec!["test".to_owned()],
            metadata: BTreeMap::new(),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 10.0,
                sample_hz: None,
            },
            world: WorldSpec {
                gravity_mps2: 1.62,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-100.0, 0.0), Vec2::new(100.0, 0.0)],
                },
                landing_pads: vec![LandingPadSpec {
                    id: "test-pad".to_owned(),
                    center_x_m: 0.0,
                    surface_y_m: 0.0,
                    width_m: 40.0,
                }],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 4.0,
                    hull_height_m: 6.0,
                    touchdown_half_span_m: 1.5,
                    touchdown_base_offset_m: 5.0,
                },
                dry_mass_kg: 700.0,
                initial_fuel_kg: 200.0,
                max_fuel_kg: 200.0,
                max_thrust_n: 14_000.0,
                max_fuel_burn_kgps: 10.0,
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.0,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.2,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(0.0, 5.0002),
                velocity_mps: Vec2::new(0.0, -0.1),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                transfer_route: None,
                goal: EvaluationGoal::LandingOnPad {
                    target_pad_id: "test-pad".to_owned(),
                },
            },
        })
        .expect("valid synthetic context")
    }

    fn flat_pad(left_m: f64, right_m: f64) -> FlatPadBounds {
        FlatPadBounds {
            left_m,
            right_m,
            surface_y_m: 0.0,
            flat: true,
        }
    }

    #[test]
    fn generated_source_endpoint_requires_replayed_state_not_stored_flags() {
        let context = test_context();
        let state = SimulationState::new(&context).unwrap();
        let actual = plant_state_evidence(&state, &context);
        let mut stored = super::super::paired_command_feasibility::EndpointStateEvidence {
            physics_step: actual.physics_step,
            sim_time_s: actual.sim_time_s,
            position_m: actual.position_m,
            velocity_mps: actual.velocity_mps,
            attitude_rad: actual.attitude_rad,
            angular_rate_radps: actual.angular_rate_radps,
            fuel_kg: actual.fuel_kg,
        };
        assert!(source_endpoint_matches(Some(&actual), Some(&stored)));
        stored.position_m.x += 0.01;
        assert!(!source_endpoint_matches(Some(&actual), Some(&stored)));
        stored.position_m = actual.position_m;
        stored.physics_step += 1;
        assert!(!source_endpoint_matches(Some(&actual), Some(&stored)));
        assert!(!source_endpoint_matches(None, Some(&stored)));
        assert!(!source_endpoint_matches(Some(&actual), None));
    }

    fn selection_row(
        row_index: usize,
        identity: &str,
        planned_time_s: f64,
        touchdown_time_s: f64,
        accepted: bool,
    ) -> CompleteFlatAcceptanceRowEvidence {
        let wrapper = CompleteFlatWrapperEvidence {
            wrapper_identity: identity.to_owned(),
            provenance: CompleteFlatWrapperProvenanceEvidence {
                source_row_index: row_index,
                frozen_basis_identity: String::new(),
                source_duration_identity: String::new(),
                paired_command_identity: String::new(),
                scenario_identity: String::new(),
                policy_identity: String::new(),
                acceptance_policy_identity: String::new(),
                launch_rule: String::new(),
                cadence: String::new(),
                geometry_convention: String::new(),
                launch_schedule_identity: String::new(),
                source_command_schedule_identity: String::new(),
                tail_command_schedule_identity: String::new(),
                frozen_tail_profile_identity: String::new(),
            },
            source_handoff_position_error_m: None,
            source_handoff_velocity_error_mps: None,
            acceptance: CompleteFlatAcceptanceGatesEvidence {
                supported_source_pad_rest_state: true,
                launch_completed_and_contact_free: true,
                launch_to_source_join_passed: true,
                existing_source_screens_passed: true,
                strict_source_handoff_passed: true,
                scheduled_source_prefix_parity_passed: true,
                ordinary_neutral_and_stored_replay_parity_passed: true,
                pointwise_core_geometry_clearance_passed: true,
                terminal_entry_descending_and_clear: true,
                terminal_handoff_on_descending_arc: true,
                first_contact_stable_safe_on_target: true,
                no_earlier_contact: true,
                fuel_time_and_commandability_budgets_passed: true,
                planned_time_reserve_passed: true,
            },
            accepted,
            first_failing_gate: None,
            first_failing_tick: None,
            planned_total_mission_time_s: planned_time_s,
            actual_touchdown_time_s: Some(touchdown_time_s),
            actual_touchdown_fuel_remaining_kg: None,
            actual_touchdown_fuel_used_kg: None,
            first_contact: None,
            signed_first_contact_margins: None,
            minimum_airborne_clearance_m: None,
            minimum_airborne_clearance: None,
            clearance_scan: GeometryClearanceScanEvidence {
                poststep_state_count: 0,
                airborne_state_count: 0,
                source_corridor_state_count: 0,
                terminal_corridor_state_count: 0,
                exact_clearance_query_count: 0,
                all_airborne_states_passed: true,
                first_violation: None,
                minimum_airborne: None,
            },
            replay: CompleteFlatReplayEvidence {
                stored_replay_trace: None,
                recomputed_replay_trace: None,
                independent_neutral_replay: NeutralReplayEvidence {
                    physics_ticks_replayed: 0,
                    contiguous_from_step_one: true,
                    logged_command_schedule_matches: true,
                    expected_thrust_reference_coverage: true,
                    ordinary_and_neutral_states_match: true,
                    poststep_contact_labels_match_log: true,
                    no_contact_before_first_event: true,
                    first_contact_physics_step: None,
                    first_contact_classification: None,
                    first_contact_state: None,
                    first_contact_matches_authoritative_replay: true,
                    first_command_mismatch: None,
                },
                stored_first_contact_matches_replay: true,
                stored_first_contact_margins_match_replay: true,
                full_rollout_matches_frozen_launch_and_tail: true,
            },
            saturation: None,
            recomputed_saturation: None,
            rejection_reasons: Vec::new(),
        };
        CompleteFlatAcceptanceRowEvidence {
            row_index,
            candidate_identity: format!("candidate-{row_index}"),
            role: "test".to_owned(),
            duration_offset_ticks: 0,
            source_bridge_tick_count: 0,
            analytical_survivor: true,
            status: "scheduled_wrapper_replayed".to_owned(),
            skip_reason: None,
            wrapper: Some(wrapper),
        }
    }

    #[test]
    fn frozen_tail_reference_maps_first_last_coast_and_first_terminal_steps() {
        let source_bridge_ticks = 1_560;
        let tail_start = 1_740;
        assert_eq!(
            frozen_tail_profile_index(1_633, source_bridge_ticks, tail_start),
            Some(tail_start as usize)
        );
        assert_eq!(
            frozen_tail_profile_index(2_382, source_bridge_ticks, tail_start),
            Some((tail_start + 749) as usize)
        );
        assert_eq!(
            frozen_tail_profile_index(2_383, source_bridge_ticks, tail_start),
            Some((tail_start + 750) as usize)
        );
        assert_eq!(
            frozen_tail_profile_index(72, source_bridge_ticks, tail_start),
            None
        );
    }

    #[test]
    fn corridors_require_both_actual_feet_and_hull_bounds_and_descending_terminal_entry() {
        let target = flat_pad(0.0, 10.0);
        let contained = BodyAabb {
            horizontal_extent_m: 1.0,
            vertical_extent_m: 2.0,
            feet_x_min_m: 1.0,
            feet_x_max_m: 9.0,
            hull_x_min_m: 1.5,
            hull_x_max_m: 8.5,
        };
        let hull_overhang = BodyAabb {
            hull_x_min_m: -0.1,
            ..contained
        };
        assert!(body_within_pad(contained, target));
        assert!(!body_within_pad(hull_overhang, target));
        assert_eq!(
            corridor_for_step("upright", Vec2::new(0.0, 0.0), contained, target, target),
            "source_pad_transition"
        );
        assert_eq!(
            corridor_for_step(
                "source_bridge",
                Vec2::new(0.0, 0.0),
                hull_overhang,
                target,
                target
            ),
            "none"
        );
        assert_eq!(
            corridor_for_step(
                "terminal_bridge",
                Vec2::new(0.0, -0.1),
                contained,
                target,
                target
            ),
            "descending_terminal_pad_transition"
        );
        assert_eq!(
            corridor_for_step(
                "terminal_bridge",
                Vec2::new(0.0, 0.1),
                contained,
                target,
                target
            ),
            "none"
        );

        let context = test_context();
        let mut state = SimulationState::new(&context).unwrap();
        state.position_m.x = 99.0;
        let aabb = body_aabb(&state, &context.vehicle.geometry);
        assert!(body_clearance(&context, &state, aabb).is_err());
    }

    #[test]
    fn core_positive_attitude_rotates_feet_with_the_unchanged_core_sign() {
        let context = test_context();
        let attitude = 0.2_f64;
        let aabb = body_aabb_from_pose(Vec2::new(0.0, 10.0), attitude, &context.vehicle.geometry);
        let span = context.vehicle.geometry.touchdown_half_span_m;
        let offset = context.vehicle.geometry.touchdown_base_offset_m;
        let sine_shift = offset * attitude.sin();
        let expected_min = -span * attitude.cos() + sine_shift;
        let expected_max = span * attitude.cos() + sine_shift;
        assert!((aabb.feet_x_min_m - expected_min).abs() < 1.0e-12);
        assert!((aabb.feet_x_max_m - expected_max).abs() < 1.0e-12);
    }

    #[test]
    fn ordinary_and_neutral_replay_match_at_real_stable_touchdown_terminal_effect() {
        let context = test_context();
        let mut neutral = SimulationState::new(&context).unwrap();
        let mut ordinary = neutral.clone();
        let classification = neutral.step_physics_and_classify_contact(&context);
        assert_eq!(
            classification,
            ContactClassification::StableTouchdown { on_target: true }
        );
        let events = ordinary.step(&context);
        assert_eq!(event_contact_label(&events), "stable_touchdown_on_target");
        assert!(ordinary.is_terminal());
        assert_eq!(ordinary.velocity_mps, Vec2::new(0.0, 0.0));
        assert!(neutral.velocity_mps.y < 0.0);
        assert!(same_ordinary_neutral_state(
            &ordinary,
            &neutral,
            &classification
        ));
    }

    #[test]
    fn selection_uses_planned_time_then_wrapper_identity_only_after_global_gate() {
        let rows = vec![
            selection_row(0, "z-wrapper", 34.0, 1.0, true),
            selection_row(1, "a-rejected", 1.0, 0.1, false),
            selection_row(2, "b-wrapper", 34.0, 30.0, true),
            selection_row(3, "early-contact", 35.0, 0.05, true),
        ];
        let selection = select_accepted_wrapper(&rows, true).unwrap();
        assert_eq!(selection.row_index, 2);
        assert_eq!(selection.wrapper_identity, "b-wrapper");
        assert_eq!(selection.planned_total_mission_time_s, 34.0);
        assert!(select_accepted_wrapper(&rows, false).is_none());
    }

    #[test]
    fn first_contact_margin_gate_rejects_nonfinite_or_negative_values() {
        let valid = ScheduledFirstContactMarginsEvidence {
            no_contact_minimum_foot_clearance_m: 0.0,
            no_contact_minimum_hull_clearance_m: 0.0,
            stable_minimum_clearance_margin_m: 0.01,
            stable_maximum_clearance_margin_m: 0.1,
            stable_hull_penetration_margin_m: 0.01,
            safe_normal_speed_margin_mps: 1.0,
            safe_tangential_speed_margin_mps: 1.0,
            safe_attitude_margin_rad: 0.1,
            safe_angular_rate_margin_radps: 0.1,
            touchdown_pad_left_margin_m: 1.0,
            touchdown_pad_right_margin_m: 1.0,
        };
        assert!(stable_safe_margins_pass(&valid));
        let mut nonfinite = valid.clone();
        nonfinite.safe_normal_speed_margin_mps = f64::NAN;
        assert!(!stable_safe_margins_pass(&nonfinite));
        let mut negative = valid;
        negative.touchdown_pad_left_margin_m = -f64::EPSILON;
        assert!(!stable_safe_margins_pass(&negative));
    }
}
