//! Identity-bound, evaluator-only launch-feasibility canary.
//!
//! This pass keeps the sealed V2 candidate as a basis identity, adds one
//! frozen source-pad launch, and re-solves only that candidate's source bridge
//! at its original handoff and tick count.  The resulting wrapper is not a V2
//! candidate and never claims `CertificationV2::Certified`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::{Command, EventKind, RunContext, ScenarioSpec, SimulationState, Vec2};
use pd_plan::conservative_ballistic_bridge::{
    AnalyticalBridgeV2, BridgeKindV2, CertificationV2, ComponentMarginsV2, DirectBridgeCandidateV2,
    DirectBridgePolicyV2, DirectBridgeProbeV2, DirectBridgeReasonV2, KinematicStateV2, MarginV2,
    exact_discrete_bridge_v2,
};
use serde::{Deserialize, Serialize};

use super::source_contact::{mirror_v2_clearance, prepare_source_contact_inputs};
use super::{
    CommandSaturationEvidence, ContactEvidence, FirstDivergenceEvidence, PhaseHandoffErrorEvidence,
    PlantStateEvidence, PreparedCase, PreparedInputs, PreparedProfileCandidate, RolloutCadence,
    SelectionRoleEvidence, ThrottleSaturation, WaypointDirectNominalPlantArtifact, direction_angle,
    enum_label, plant_applied_throttle, resolve_output_dir, shortest_angle_delta, stable_digest,
    throttle_request,
};

pub const WAYPOINT_DIRECT_LAUNCH_FEASIBILITY_ID: &str = "waypoint-direct-launch-feasibility";
pub const WAYPOINT_DIRECT_LAUNCH_FEASIBILITY_SCHEMA_ID: &str =
    "waypoint_direct_launch_feasibility_v1";
pub const WAYPOINT_DIRECT_LAUNCH_FEASIBILITY_SCHEMA_VERSION: u32 = 1;

const UPRIGHT_TICKS: u64 = 60;
const TILT_TICKS: u64 = 12;
const LAUNCH_TICKS: u64 = UPRIGHT_TICKS + TILT_TICKS;
const FLAT_CASE_ID: &str = "continuous_flat_r00";
const UNCUT_CASE_IDS: [&str; 2] = ["continuous_uphill_r+30", "continuous_downhill_r-30"];
const STATE_POSITION_TOLERANCE_M: f64 = 1.0e-6;
const STATE_VELOCITY_TOLERANCE_MPS: f64 = 1.0e-6;

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectLaunchFeasibilityPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectLaunchFeasibilityRun {
    pub artifact: WaypointDirectLaunchFeasibilityArtifact,
    pub paths: WaypointDirectLaunchFeasibilityPaths,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchFeasibilityProtocolEvidence {
    pub physics_hz: u32,
    pub controller_hz: u32,
    pub upright_full_throttle_ticks: u64,
    pub tilt_full_throttle_ticks: u64,
    pub launch_boundary_physics_step: u64,
    pub tilt_angle_rule: String,
    pub reseed_rule: String,
    pub cadence_rule: String,
    pub contact_rule: String,
    pub analytic_eligibility_rule: String,
    pub stop_gate_rule: String,
    pub state_parity_position_tolerance_m: f64,
    pub state_parity_velocity_tolerance_mps: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchFeasibilityInputGateEvidence {
    pub schema_id: String,
    pub schema_version: u32,
    pub baseline_identity: String,
    pub sweep_identity: String,
    pub nominal_plant_identity: String,
    pub nominal_input_manifest_identity: String,
    pub protocol: LaunchFeasibilityProtocolEvidence,
    pub cases: Vec<LaunchFeasibilityInputCaseEvidence>,
    pub case_count: u32,
    pub unique_profile_count: u32,
    pub role_count: u32,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchFeasibilityInputCaseEvidence {
    pub input: super::PreparedCaseEvidence,
    pub selection_roles: Vec<SelectionRoleEvidence>,
    pub profiles: Vec<LaunchFeasibilityProfileBindingEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchFeasibilityProfileBindingEvidence {
    pub basis_candidate_identity: String,
    pub selected_roles: Vec<String>,
    pub original_source_handoff_arc_step: u64,
    pub original_source_bridge_tick_count: u64,
    pub coast_tick_count: u64,
    pub terminal_bridge_tick_count: u64,
    pub frozen_first_powered_attitude_rad: f64,
    pub nominal_profile_tick_identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectLaunchFeasibilityArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub baseline_identity: String,
    pub sweep_identity: String,
    pub nominal_plant_identity: String,
    pub nominal_input_manifest_identity: String,
    pub input_gate_identity: String,
    pub protocol: LaunchFeasibilityProtocolEvidence,
    pub execution_stage: String,
    pub flat_gate: FlatLaunchGateEvidence,
    pub uncut_gate: UncutLaunchGateEvidence,
    pub cases: Vec<LaunchFeasibilityCaseEvidence>,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatLaunchGateEvidence {
    pub case_id: String,
    pub selected_profile_count: usize,
    pub source_handoff_contact_free_profile_count_120_hz: usize,
    pub passed: bool,
    pub consequence: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UncutLaunchGateEvidence {
    pub case_ids: Vec<String>,
    pub source_handoff_contact_free_profile_count_120_hz: usize,
    pub evaluated: bool,
    pub passed: bool,
    pub consequence: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchFeasibilityCaseEvidence {
    pub input: super::PreparedCaseEvidence,
    pub selection_roles: Vec<SelectionRoleEvidence>,
    pub status: String,
    pub skip_reason: Option<String>,
    pub candidate_profiles: Vec<LaunchFeasibilityCandidateEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchFeasibilityCandidateEvidence {
    /// New wrapper identity. The frozen V2 identity is retained only as a basis.
    pub identity: String,
    pub basis_candidate_identity: String,
    pub selected_roles: Vec<String>,
    pub original_source_handoff_arc_step: u64,
    pub original_source_bridge_tick_count: u64,
    pub coast_tick_count: u64,
    pub terminal_bridge_tick_count: u64,
    pub frozen_first_powered_attitude_rad: f64,
    pub cadence_runs: Vec<LaunchFeasibilityCadenceRunEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchFeasibilityCadenceRunEvidence {
    pub cadence: String,
    pub launch: LaunchEvidence,
    pub reseeded_bridge: Option<ReseededBridgeEvidence>,
    pub rollout: LaunchRolloutEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchEvidence {
    pub requested_physics_ticks: u64,
    pub physics_ticks_completed: u64,
    pub completed: bool,
    pub contact_free: bool,
    pub commanded_tilt_attitude_rad: f64,
    pub end_state: Option<PlantStateEvidence>,
    pub samples: Vec<LaunchTickEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchTickEvidence {
    pub physics_step: u64,
    pub phase: String,
    pub contact_classification: String,
    pub commanded_throttle_frac: f64,
    pub commanded_target_attitude_rad: f64,
    pub held_target_attitude_rad: f64,
    pub attitude_before_step_rad: f64,
    pub attitude_after_step_rad: f64,
    pub angular_rate_radps: f64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub touchdown_clearance_m: f64,
    pub hull_clearance_m: f64,
    pub fuel_kg: f64,
    pub sim_time_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReseededBridgeEvidence {
    pub solve_status: String,
    pub solve_error: Option<String>,
    pub identity: Option<String>,
    pub classification: Option<CertificationV2>,
    pub reasons: Vec<DirectBridgeReasonV2>,
    pub start_state: Option<KinematicStateV2>,
    pub end_state: Option<KinematicStateV2>,
    pub tick_count: u64,
    pub endpoint_position_error_m: Option<f64>,
    pub endpoint_velocity_error_mps: Option<f64>,
    pub fuel_burn_kg: Option<f64>,
    pub margins: Option<ComponentMarginsV2>,
    pub source_clearance: Option<SourceClearanceScreenEvidence>,
    pub source_attitude_margin: Option<MarginEvidence>,
    pub launch_boundary: Option<LaunchBoundaryEvidence>,
    pub aggregate_fuel_margin: Option<MarginEvidence>,
    pub aggregate_time_margin: Option<MarginEvidence>,
    pub analytically_eligible: bool,
    pub diagnostic_replay_allowed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceClearanceScreenEvidence {
    pub evaluated_tick_count: u64,
    pub source_pad_center_height_tick_count: u64,
    pub rotated_hull_clearance_tick_count: u64,
    pub minimum_margin: MarginEvidence,
    pub all_samples_passed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarginEvidence {
    pub raw_margin: f64,
    pub normalized_margin: f64,
    pub passes_declared_screen: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchBoundaryEvidence {
    pub launch_end_attitude_rad: f64,
    pub launch_end_angular_rate_radps: f64,
    pub first_reseeded_powered_attitude_rad: Option<f64>,
    pub attitude_delta_rad: Option<f64>,
    pub required_one_tick_slew_rate_radps: Option<f64>,
    pub maximum_rotation_rate_radps: f64,
    pub slew_margin: Option<MarginEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchRolloutEvidence {
    pub status: String,
    pub source_handoff_reached: bool,
    pub source_handoff_contact_free: bool,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub profile_tick_count: u64,
    pub physics_steps_advanced: u64,
    pub first_source_tick: Option<FirstSourceTickEvidence>,
    pub first_divergence: Option<FirstDivergenceEvidence>,
    pub max_position_error_m: f64,
    pub max_velocity_error_mps: f64,
    pub contact_transition_state_errors: Vec<ContactTransitionStateErrorEvidence>,
    pub phase_handoff_errors: Vec<PhaseHandoffErrorEvidence>,
    pub minimum_touchdown_clearance_m: Option<f64>,
    pub minimum_hull_clearance_m: Option<f64>,
    pub contacts: Vec<ContactEvidence>,
    pub per_step: Vec<LaunchRolloutTickEvidence>,
    pub profile_end: Option<PlantStateEvidence>,
    pub termination: PlantStateEvidence,
    pub saturation: CommandSaturationEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FirstSourceTickEvidence {
    pub physics_step: u64,
    pub desired_target_attitude_rad: f64,
    pub held_target_attitude_rad: f64,
    pub attitude_before_step_rad: f64,
    pub attitude_after_step_rad: f64,
    pub angular_rate_radps: f64,
    pub commanded_throttle_frac: f64,
    pub applied_throttle_frac: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchRolloutTickEvidence {
    pub physics_step: u64,
    pub phase: String,
    pub contact_classification: String,
    pub desired_target_attitude_rad: f64,
    pub held_target_attitude_rad: f64,
    pub attitude_before_step_rad: f64,
    pub attitude_after_step_rad: f64,
    pub angular_rate_radps: f64,
    pub commanded_throttle_frac: f64,
    pub applied_throttle_frac: f64,
    pub state_position_error_m: Option<f64>,
    pub state_velocity_error_mps: Option<f64>,
    pub touchdown_clearance_m: f64,
    pub hull_clearance_m: f64,
    pub fuel_kg: f64,
    pub sim_time_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactTransitionStateErrorEvidence {
    pub physics_step: u64,
    pub phase: String,
    pub contact_classification: String,
    pub expected_position_error_m: f64,
    pub expected_velocity_error_mps: f64,
}

#[derive(Clone, Copy)]
struct CandidateProfileRunContext<'a> {
    scenario: &'a ScenarioSpec,
    probe: &'a DirectBridgeProbeV2,
    selected: &'a PreparedProfileCandidate,
    basis: &'a DirectBridgeCandidateV2,
    policy: &'a DirectBridgePolicyV2,
    vehicle: &'a super::VehicleInputV2,
    frozen_tilt_attitude_rad: f64,
    coast_tick_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SourceDurationReplayStage {
    AnalyzeOnly,
    SourceHandoff,
    FullFlight,
}

pub(super) struct SourceDurationRunRequest<'a> {
    pub(super) scenario: &'a ScenarioSpec,
    pub(super) probe: &'a DirectBridgeProbeV2,
    pub(super) selected: &'a PreparedProfileCandidate,
    pub(super) basis: &'a DirectBridgeCandidateV2,
    pub(super) policy: &'a DirectBridgePolicyV2,
    pub(super) vehicle: &'a super::VehicleInputV2,
    pub(super) source_bridge_steps: u64,
    pub(super) launch_tilt_attitude_rad: f64,
    pub(super) cadence: RolloutCadence,
    pub(super) stage: SourceDurationReplayStage,
}

/// One desired thrust-vector sample held for a complete controller interval.
/// The paired-command diagnostic supplies exactly one of these for each two
/// source physics ticks; launch and the frozen tail remain on their existing
/// schedules.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct HeldSourceCommand {
    pub(super) thrust_acceleration_mps2: Vec2,
    pub(super) target_attitude_rad: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(super) enum SourceDurationHoldMode {
    Together,
    ThrottleHeldAttitudePerTick,
    AttitudeHeldThrottlePerTick,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct SourceDurationStateSample {
    pub physics_step: u64,
    pub phase: String,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct SourceDurationCommandSample {
    /// Post-step physics tick for the command that was applied on this step.
    pub physics_step: u64,
    pub phase: String,
    pub desired_command_throttle_frac: f64,
    pub desired_applied_throttle_frac: f64,
    pub held_command_throttle_frac: f64,
    pub plant_applied_throttle_frac: f64,
    pub desired_target_attitude_rad: f64,
    pub held_target_attitude_rad: f64,
    pub throttle_update_due: bool,
    pub attitude_update_due: bool,
}

#[derive(Clone, Debug)]
pub(super) struct SourceDurationDiagnosticCadenceRun {
    pub run: LaunchFeasibilityCadenceRunEvidence,
    pub state_samples: Vec<SourceDurationStateSample>,
    pub command_samples: Vec<SourceDurationCommandSample>,
}

struct MaterializedRolloutPolicy {
    cadence: RolloutCadence,
    stop_at_source_handoff: bool,
    hold_mode: SourceDurationHoldMode,
}

#[derive(Clone, Copy)]
struct MaterializedCadenceRequest<'a> {
    cadence: RolloutCadence,
    source_bridge_steps: u64,
    launch_tilt_attitude_rad: f64,
    stage: SourceDurationReplayStage,
    hold_mode: SourceDurationHoldMode,
    capture_state_samples: bool,
    held_source_schedule: Option<&'a [HeldSourceCommand]>,
}

struct MaterializedRolloutRequest<'a, 'candidate> {
    candidate: &'a CandidateProfileRunContext<'candidate>,
    bridge: &'a AnalyticalBridgeV2,
    state_after_launch: &'a SimulationState,
    context: &'a RunContext,
    saturation: &'a mut CommandSaturationEvidence,
    rollout_policy: MaterializedRolloutPolicy,
    held_source_schedule: Option<&'a [HeldSourceCommand]>,
    diagnostic_capture: SourceDurationDiagnosticCapture<'a>,
}

struct SourceDurationDiagnosticCapture<'a> {
    state_samples: Option<&'a mut Vec<SourceDurationStateSample>>,
    command_samples: Option<&'a mut Vec<SourceDurationCommandSample>>,
}

#[derive(Clone, Debug)]
pub(super) struct FlatThirdCandidateExecution {
    pub(super) source_gate_run: LaunchFeasibilityCadenceRunEvidence,
    pub(super) source_gate_passed: bool,
    pub(super) completed_direct_prefix_matched: Option<bool>,
    pub(super) candidate: Option<LaunchFeasibilityCandidateEvidence>,
}

/// Build the deterministic identity gate without constructing a
/// `SimulationState`. This is the preflight CLI path before any physics.
pub fn validate_waypoint_direct_launch_feasibility_inputs(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    nominal_summary_path: &Path,
) -> Result<LaunchFeasibilityInputGateEvidence> {
    let (prepared, nominal) = prepare_source_contact_inputs(
        repo_root,
        baseline_summary_path,
        sweep_summary_path,
        nominal_summary_path,
    )?;
    build_input_gate(&prepared, &nominal)
}

/// Run the fixed 60+12 launch rule and frozen-handoff source reseed after the
/// complete no-physics gate has accepted every sealed input.
pub fn run_waypoint_direct_launch_feasibility(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    nominal_summary_path: &Path,
    output_dir: &Path,
    flat_gate_only: bool,
) -> Result<WaypointDirectLaunchFeasibilityRun> {
    let (prepared, nominal) = prepare_source_contact_inputs(
        repo_root,
        baseline_summary_path,
        sweep_summary_path,
        nominal_summary_path,
    )?;
    let gate = build_input_gate(&prepared, &nominal)?;
    let protocol = launch_protocol();

    let flat_case = prepared
        .cases
        .iter()
        .find(|case| case.evidence.id == FLAT_CASE_ID)
        .ok_or_else(|| anyhow!("frozen flat case is missing"))?;
    let mut case_rows = prepared
        .cases
        .iter()
        .map(|case| {
            Ok(LaunchFeasibilityCaseEvidence {
                input: case.evidence.clone(),
                selection_roles: selection_roles_for(case, &nominal)?,
                status: "not_run".to_owned(),
                skip_reason: None,
                candidate_profiles: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let flat_index = prepared
        .cases
        .iter()
        .position(|case| case.evidence.id == FLAT_CASE_ID)
        .ok_or_else(|| anyhow!("frozen flat case index is missing"))?;
    let flat_results = run_case_profiles(
        flat_case,
        &prepared.baseline.evaluated_policy,
        &prepared.baseline.vehicle,
    )?;
    let flat_profile_count = flat_results.len();
    let flat_pass_count = flat_results
        .iter()
        .filter(|candidate| {
            candidate.cadence_runs.iter().any(|run| {
                run.cadence == "direct_per_tick_120_hz" && run.rollout.source_handoff_contact_free
            })
        })
        .count();
    case_rows[flat_index].status = "ran_flat_gate".to_owned();
    case_rows[flat_index].candidate_profiles = flat_results;
    let flat_gate_passed = flat_pass_count > 0;
    let flat_gate = FlatLaunchGateEvidence {
        case_id: FLAT_CASE_ID.to_owned(),
        selected_profile_count: flat_profile_count,
        source_handoff_contact_free_profile_count_120_hz: flat_pass_count,
        passed: flat_gate_passed,
        consequence: if flat_gate_passed && flat_gate_only {
            "flat gate passed; stop here pending review before running uncut slopes".to_owned()
        } else if flat_gate_passed {
            "finish both uncut slopes; run obstacles only if an uncut case has a contact-free 120 Hz source handoff".to_owned()
        } else {
            "stop after both frozen flat roles and both cadences; leave all remaining cases explicitly not run".to_owned()
        },
    };

    let mut uncut_pass_count = 0_usize;
    let uncut_gate_evaluated = flat_gate_passed && !flat_gate_only;
    if uncut_gate_evaluated {
        for case_id in UNCUT_CASE_IDS {
            let case_index = prepared
                .cases
                .iter()
                .position(|case| case.evidence.id == case_id)
                .ok_or_else(|| anyhow!("frozen uncut case {case_id} is missing"))?;
            let results = run_case_profiles(
                &prepared.cases[case_index],
                &prepared.baseline.evaluated_policy,
                &prepared.baseline.vehicle,
            )?;
            uncut_pass_count += results
                .iter()
                .filter(|candidate| {
                    candidate.cadence_runs.iter().any(|run| {
                        run.cadence == "direct_per_tick_120_hz"
                            && run.rollout.source_handoff_contact_free
                    })
                })
                .count();
            case_rows[case_index].status = "ran_uncut_gate".to_owned();
            case_rows[case_index].candidate_profiles = results;
        }
    } else {
        for case in &prepared.cases {
            if case.evidence.id != FLAT_CASE_ID {
                let row = case_rows
                    .iter_mut()
                    .find(|row| row.input.id == case.evidence.id)
                    .expect("case rows match prepared cases");
                row.skip_reason = Some(if flat_gate_passed {
                    "awaiting_flat_gate_review".to_owned()
                } else {
                    "flat_gate_failed".to_owned()
                });
            }
        }
    }
    let uncut_gate_passed = uncut_gate_evaluated && uncut_pass_count > 0;
    let uncut_gate = UncutLaunchGateEvidence {
        case_ids: UNCUT_CASE_IDS.iter().map(|id| (*id).to_owned()).collect(),
        source_handoff_contact_free_profile_count_120_hz: uncut_pass_count,
        evaluated: uncut_gate_evaluated,
        passed: uncut_gate_passed,
        consequence: if uncut_gate_passed {
            "run all three frozen obstacle profiles and both cadences".to_owned()
        } else if flat_gate_passed && flat_gate_only {
            "uncut gate is not evaluated in the flat-only artifact; slopes and obstacles remain deferred".to_owned()
        } else if flat_gate_passed {
            "leave all obstacle cases explicitly not run".to_owned()
        } else {
            "uncut cases were not reached because the flat gate failed".to_owned()
        },
    };

    if uncut_gate_passed {
        for case in &prepared.cases {
            if case.evidence.id == FLAT_CASE_ID
                || UNCUT_CASE_IDS.contains(&case.evidence.id.as_str())
            {
                continue;
            }
            let index = prepared
                .cases
                .iter()
                .position(|prepared_case| prepared_case.evidence.id == case.evidence.id)
                .expect("case is in prepared input");
            case_rows[index].candidate_profiles = run_case_profiles(
                case,
                &prepared.baseline.evaluated_policy,
                &prepared.baseline.vehicle,
            )?;
            case_rows[index].status = "ran_conditional_obstacle".to_owned();
        }
    } else {
        for case in &prepared.cases {
            if case.evidence.id == FLAT_CASE_ID
                || UNCUT_CASE_IDS.contains(&case.evidence.id.as_str())
            {
                continue;
            }
            let row = case_rows
                .iter_mut()
                .find(|row| row.input.id == case.evidence.id)
                .expect("case rows match prepared cases");
            row.skip_reason = Some(if flat_gate_passed && flat_gate_only {
                "awaiting_flat_gate_review".to_owned()
            } else if flat_gate_passed {
                "uncut_gate_failed".to_owned()
            } else {
                "flat_gate_failed".to_owned()
            });
        }
    }

    let mut artifact = WaypointDirectLaunchFeasibilityArtifact {
        schema_id: WAYPOINT_DIRECT_LAUNCH_FEASIBILITY_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_LAUNCH_FEASIBILITY_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_LAUNCH_FEASIBILITY_ID.to_owned(),
        baseline_identity: prepared.baseline.identity.clone(),
        sweep_identity: prepared.sweep.identity.clone(),
        nominal_plant_identity: nominal.identity.clone(),
        nominal_input_manifest_identity: nominal.input_manifest_identity.clone(),
        input_gate_identity: gate.identity,
        protocol,
        execution_stage: if flat_gate_only {
            "flat_gate_only".to_owned()
        } else {
            "conditional_full".to_owned()
        },
        flat_gate,
        uncut_gate,
        cases: case_rows,
        scope_non_claims: vec![
            "This evaluator wrapper is not a V2 candidate and never carries V2 Certified status.".to_owned(),
            "A fixed-handoff result says nothing about unsearched handoffs, bridge durations, or launch schedules.".to_owned(),
            "A diagnostic replay does not alter simulator contact rules, production planning, controller behavior, F6, or defaults.".to_owned(),
            "120 Hz per-tick execution does not establish 60 Hz controller-cadence success.".to_owned(),
            "No failed or conditionally skipped case is evidence of physical impossibility.".to_owned(),
        ],
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;

    let output_dir = resolve_output_dir(repo_root, output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create waypoint direct launch-feasibility output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&summary_path, &summary_bytes).with_context(|| {
        format!(
            "failed to write waypoint direct launch-feasibility summary {}",
            summary_path.display()
        )
    })?;
    let reloaded: WaypointDirectLaunchFeasibilityArtifact = serde_json::from_slice(&summary_bytes)
        .context("failed to reload waypoint direct launch-feasibility summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes {
        bail!("launch-feasibility summary is not byte-stable after reload");
    }
    if artifact_identity(&reloaded)? != reloaded.identity {
        bail!("launch-feasibility semantic identity failed round-trip check");
    }

    Ok(WaypointDirectLaunchFeasibilityRun {
        artifact: reloaded,
        paths: WaypointDirectLaunchFeasibilityPaths {
            output_dir,
            summary_path,
        },
    })
}

pub(super) fn build_input_gate(
    prepared: &PreparedInputs,
    nominal: &WaypointDirectNominalPlantArtifact,
) -> Result<LaunchFeasibilityInputGateEvidence> {
    let protocol = launch_protocol();
    let mut cases = Vec::with_capacity(prepared.cases.len());
    let mut unique_profile_count = 0_u32;
    let mut role_count = 0_u32;
    for (case, nominal_case) in prepared.cases.iter().zip(&nominal.cases) {
        let selection_roles = nominal_case.selection_roles.clone();
        role_count += selection_roles.len() as u32;
        let mut profiles = Vec::with_capacity(case.profile_candidates.len());
        for selected in &case.profile_candidates {
            let basis = candidate_for(case, selected)?;
            let source_handoff = basis.source_handoff.ok_or_else(|| {
                anyhow!("candidate {} has no frozen source handoff", basis.identity)
            })?;
            let source_bridge = basis.source_bridge.as_ref().ok_or_else(|| {
                anyhow!("candidate {} has no frozen source bridge", basis.identity)
            })?;
            let terminal_bridge = basis.terminal_bridge.as_ref().ok_or_else(|| {
                anyhow!("candidate {} has no frozen terminal bridge", basis.identity)
            })?;
            let frozen_first_powered_attitude_rad = frozen_first_powered_attitude(selected)?;
            profiles.push(LaunchFeasibilityProfileBindingEvidence {
                basis_candidate_identity: basis.identity.clone(),
                selected_roles: selected.selected_roles.clone(),
                original_source_handoff_arc_step: source_handoff.arc_step,
                original_source_bridge_tick_count: source_bridge.steps,
                coast_tick_count: basis
                    .selected_coast
                    .as_ref()
                    .ok_or_else(|| anyhow!("candidate {} has no selected coast", basis.identity))?
                    .duration_s
                    .mul_add(
                        f64::from(prepared.baseline.evaluated_policy.physics_hz),
                        0.5,
                    ) as u64,
                terminal_bridge_tick_count: terminal_bridge.steps,
                frozen_first_powered_attitude_rad,
                nominal_profile_tick_identity: profile_tick_identity(selected)?,
            });
            unique_profile_count += 1;
        }
        cases.push(LaunchFeasibilityInputCaseEvidence {
            input: case.evidence.clone(),
            selection_roles,
            profiles,
        });
    }
    if prepared.cases.len() != 6 || unique_profile_count != 9 || role_count != 12 {
        bail!("launch-feasibility input gate requires six cases, nine profiles, and twelve roles");
    }
    let mut gate = LaunchFeasibilityInputGateEvidence {
        schema_id: "waypoint_direct_launch_feasibility_input_gate_v1".to_owned(),
        schema_version: 1,
        baseline_identity: prepared.baseline.identity.clone(),
        sweep_identity: prepared.sweep.identity.clone(),
        nominal_plant_identity: nominal.identity.clone(),
        nominal_input_manifest_identity: nominal.input_manifest_identity.clone(),
        protocol,
        cases,
        case_count: prepared.cases.len() as u32,
        unique_profile_count,
        role_count,
        identity: String::new(),
    };
    gate.identity = input_gate_identity(&gate)?;
    let bytes = serde_json::to_vec(&gate)?;
    let reloaded: LaunchFeasibilityInputGateEvidence = serde_json::from_slice(&bytes)?;
    if reloaded != gate || input_gate_identity(&reloaded)? != reloaded.identity {
        bail!("launch-feasibility input gate failed semantic identity round-trip");
    }
    Ok(reloaded)
}

fn launch_protocol() -> LaunchFeasibilityProtocolEvidence {
    LaunchFeasibilityProtocolEvidence {
        physics_hz: 120,
        controller_hz: 60,
        upright_full_throttle_ticks: UPRIGHT_TICKS,
        tilt_full_throttle_ticks: TILT_TICKS,
        launch_boundary_physics_step: LAUNCH_TICKS,
        tilt_angle_rule: "the original frozen profile's first nonzero source-bridge thrust sample; target attitude is atan2(thrust_acceleration.x, thrust_acceleration.y)".to_owned(),
        reseed_rule: "from the actual post-launch position and velocity, call exact_discrete_bridge_v2 with BridgeKindV2::Source, the original frozen source handoff state, and the original source bridge tick count; preserve original arc handoff, coast, terminal bridge, and role".to_owned(),
        cadence_rule: "run each profile independently from the same supported source-rest state; direct mode updates each 120 Hz physics tick, held mode updates only on unchanged 60 Hz controller ticks and holds for two physics steps; 72 launch ticks end on a controller boundary".to_owned(),
        contact_rule: "advance the ordinary SimulationState::step path and retain every post-step classification, actual touchdown/hull clearance, and mission outcome; stop a lane immediately when the simulator terminates".to_owned(),
        analytic_eligibility_rule: "report the re-solved bridge classification and source clearance/source-attitude/launch-boundary/fuel/time screens separately; diagnostic replay is still run for every finite bridge even when any analytical screen fails; the wrapper is never a V2 Certified candidate".to_owned(),
        stop_gate_rule: "run both flat roles and both cadences first; if no 120 Hz flat profile is contact-free and within handoff tolerance, stop; otherwise run both uncut slopes; run all obstacles only if an uncut 120 Hz profile is contact-free and within handoff tolerance".to_owned(),
        state_parity_position_tolerance_m: STATE_POSITION_TOLERANCE_M,
        state_parity_velocity_tolerance_mps: STATE_VELOCITY_TOLERANCE_MPS,
    }
}

fn run_case_profiles(
    case: &PreparedCase,
    policy: &DirectBridgePolicyV2,
    vehicle: &super::VehicleInputV2,
) -> Result<Vec<LaunchFeasibilityCandidateEvidence>> {
    case.profile_candidates
        .iter()
        .map(|selected| run_candidate_profile(case, selected, policy, vehicle))
        .collect()
}

fn run_candidate_profile(
    case: &PreparedCase,
    selected: &PreparedProfileCandidate,
    policy: &DirectBridgePolicyV2,
    vehicle: &super::VehicleInputV2,
) -> Result<LaunchFeasibilityCandidateEvidence> {
    let basis = candidate_for(case, selected)?;
    let source_handoff = basis
        .source_handoff
        .ok_or_else(|| anyhow!("candidate {} has no source handoff", basis.identity))?;
    let original_source_bridge = basis
        .source_bridge
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} has no source bridge", basis.identity))?;
    let original_terminal_bridge = basis
        .terminal_bridge
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} has no terminal bridge", basis.identity))?;
    let coast = basis
        .selected_coast
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} has no selected coast", basis.identity))?;
    let frozen_first_powered_attitude_rad = frozen_first_powered_attitude(selected)?;
    let coast_tick_count = (coast.duration_s * f64::from(policy.physics_hz)).round() as u64;
    let candidate_context = CandidateProfileRunContext {
        scenario: &case.scenario,
        probe: &case.probe,
        selected,
        basis,
        policy,
        vehicle,
        frozen_tilt_attitude_rad: frozen_first_powered_attitude_rad,
        coast_tick_count,
    };
    let mut cadence_runs = Vec::with_capacity(2);
    for cadence in [
        RolloutCadence::DirectPerTick,
        RolloutCadence::ControllerCadence,
    ] {
        cadence_runs.push(run_cadence(&candidate_context, cadence, false)?);
    }
    let mut candidate = LaunchFeasibilityCandidateEvidence {
        identity: String::new(),
        basis_candidate_identity: basis.identity.clone(),
        selected_roles: selected.selected_roles.clone(),
        original_source_handoff_arc_step: source_handoff.arc_step,
        original_source_bridge_tick_count: original_source_bridge.steps,
        coast_tick_count,
        terminal_bridge_tick_count: original_terminal_bridge.steps,
        frozen_first_powered_attitude_rad,
        cadence_runs,
    };
    candidate.identity = candidate_identity(&candidate)?;
    Ok(candidate)
}

/// Execute the additive flat candidate with a real 120 Hz source-handoff stop gate.
/// The gate and completed direct lane share the same launch/reseed implementation;
/// their entire overlapping tick log is compared before the 60 Hz lane starts.
pub(super) fn run_flat_third_candidate_with_source_gate(
    case: &PreparedCase,
    selected: &PreparedProfileCandidate,
    policy: &DirectBridgePolicyV2,
    vehicle: &super::VehicleInputV2,
) -> Result<FlatThirdCandidateExecution> {
    let basis = candidate_for(case, selected)?;
    let source_handoff = basis
        .source_handoff
        .ok_or_else(|| anyhow!("candidate {} has no frozen source handoff", basis.identity))?;
    let original_source_bridge = basis
        .source_bridge
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} has no frozen source bridge", basis.identity))?;
    let original_terminal_bridge = basis
        .terminal_bridge
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} has no frozen terminal bridge", basis.identity))?;
    let coast = basis
        .selected_coast
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} has no selected coast", basis.identity))?;
    let frozen_first_powered_attitude_rad = frozen_first_powered_attitude(selected)?;
    let coast_tick_count = (coast.duration_s * f64::from(policy.physics_hz)).round() as u64;
    let candidate_context = CandidateProfileRunContext {
        scenario: &case.scenario,
        probe: &case.probe,
        selected,
        basis,
        policy,
        vehicle,
        frozen_tilt_attitude_rad: frozen_first_powered_attitude_rad,
        coast_tick_count,
    };

    let source_gate_run = run_cadence(&candidate_context, RolloutCadence::DirectPerTick, true)?;
    let source_gate_passed = source_gate_run.launch.completed
        && source_gate_run.launch.contact_free
        && source_gate_run.rollout.source_handoff_contact_free
        && source_gate_run
            .rollout
            .source_handoff_position_error_m
            .is_some_and(|error| error <= STATE_POSITION_TOLERANCE_M)
        && source_gate_run
            .rollout
            .source_handoff_velocity_error_mps
            .is_some_and(|error| error <= STATE_VELOCITY_TOLERANCE_MPS);
    if !source_gate_passed {
        return Ok(FlatThirdCandidateExecution {
            source_gate_run,
            source_gate_passed,
            completed_direct_prefix_matched: None,
            candidate: None,
        });
    }

    let direct = run_cadence(&candidate_context, RolloutCadence::DirectPerTick, false)?;
    let completed_direct_prefix_matched = source_gate_prefix_matches(&source_gate_run, &direct);
    if !completed_direct_prefix_matched {
        bail!("completed third-candidate 120 Hz lane differs from its gated source prefix");
    }

    let held = run_cadence(&candidate_context, RolloutCadence::ControllerCadence, false)?;
    let mut candidate = LaunchFeasibilityCandidateEvidence {
        identity: String::new(),
        basis_candidate_identity: basis.identity.clone(),
        selected_roles: selected.selected_roles.clone(),
        original_source_handoff_arc_step: source_handoff.arc_step,
        original_source_bridge_tick_count: original_source_bridge.steps,
        coast_tick_count,
        terminal_bridge_tick_count: original_terminal_bridge.steps,
        frozen_first_powered_attitude_rad,
        cadence_runs: vec![direct, held],
    };
    candidate.identity = candidate_identity(&candidate)?;
    Ok(FlatThirdCandidateExecution {
        source_gate_run,
        source_gate_passed,
        completed_direct_prefix_matched: Some(completed_direct_prefix_matched),
        candidate: Some(candidate),
    })
}

fn source_gate_prefix_matches(
    gate: &LaunchFeasibilityCadenceRunEvidence,
    completed: &LaunchFeasibilityCadenceRunEvidence,
) -> bool {
    let prefix_len = gate.rollout.per_step.len();
    gate.cadence == completed.cadence
        && gate.launch == completed.launch
        && gate.reseeded_bridge == completed.reseeded_bridge
        && prefix_len > 0
        && completed.rollout.per_step.get(..prefix_len) == Some(&gate.rollout.per_step)
        && gate.rollout.source_handoff_reached == completed.rollout.source_handoff_reached
        && gate.rollout.source_handoff_contact_free == completed.rollout.source_handoff_contact_free
        && gate.rollout.source_handoff_position_error_m
            == completed.rollout.source_handoff_position_error_m
        && gate.rollout.source_handoff_velocity_error_mps
            == completed.rollout.source_handoff_velocity_error_mps
}

fn run_cadence(
    candidate: &CandidateProfileRunContext<'_>,
    cadence: RolloutCadence,
    stop_at_source_handoff: bool,
) -> Result<LaunchFeasibilityCadenceRunEvidence> {
    let basis = candidate.basis;
    let source_steps = basis
        .source_bridge
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} has no source bridge", basis.identity))?
        .steps;
    run_cadence_with_variant(
        candidate,
        cadence,
        source_steps,
        candidate.frozen_tilt_attitude_rad,
        if stop_at_source_handoff {
            SourceDurationReplayStage::SourceHandoff
        } else {
            SourceDurationReplayStage::FullFlight
        },
    )
}

/// Replay one frozen flat basis with an explicitly supplied source tick count
/// and launch target. The selected profile still supplies its unchanged
/// coast/terminal tail; only the source duration and launch tilt vary.
pub(super) fn run_source_duration_variant(
    request: SourceDurationRunRequest<'_>,
) -> Result<LaunchFeasibilityCadenceRunEvidence> {
    Ok(run_source_duration_variant_with_hold_mode(
        request,
        SourceDurationHoldMode::Together,
        false,
    )?
    .run)
}

pub(super) fn run_source_duration_variant_diagnostic(
    request: SourceDurationRunRequest<'_>,
    hold_mode: SourceDurationHoldMode,
) -> Result<SourceDurationDiagnosticCadenceRun> {
    if hold_mode != SourceDurationHoldMode::Together
        && request.cadence != RolloutCadence::ControllerCadence
    {
        bail!("mixed source-duration hold modes require the held 60 Hz cadence");
    }
    run_source_duration_variant_with_hold_mode(request, hold_mode, true)
}

pub(super) fn run_source_duration_variant_with_held_schedule(
    request: SourceDurationRunRequest<'_>,
    schedule: &[HeldSourceCommand],
) -> Result<SourceDurationDiagnosticCadenceRun> {
    if request.cadence != RolloutCadence::ControllerCadence {
        bail!("paired source command schedules require held 60 Hz cadence");
    }
    run_source_duration_variant_with_hold_mode_and_schedule(
        request,
        SourceDurationHoldMode::Together,
        true,
        Some(schedule),
    )
}

fn run_source_duration_variant_with_hold_mode(
    request: SourceDurationRunRequest<'_>,
    hold_mode: SourceDurationHoldMode,
    capture_state_samples: bool,
) -> Result<SourceDurationDiagnosticCadenceRun> {
    run_source_duration_variant_with_hold_mode_and_schedule(
        request,
        hold_mode,
        capture_state_samples,
        None,
    )
}

fn run_source_duration_variant_with_hold_mode_and_schedule(
    request: SourceDurationRunRequest<'_>,
    hold_mode: SourceDurationHoldMode,
    capture_state_samples: bool,
    held_source_schedule: Option<&[HeldSourceCommand]>,
) -> Result<SourceDurationDiagnosticCadenceRun> {
    let SourceDurationRunRequest {
        scenario,
        probe,
        selected,
        basis,
        policy,
        vehicle,
        source_bridge_steps,
        launch_tilt_attitude_rad,
        cadence,
        stage,
    } = request;
    if selected.candidate_identity != basis.identity {
        bail!(
            "source-duration profile does not bind basis candidate {}",
            basis.identity
        );
    }
    let coast = basis
        .selected_coast
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} has no selected coast", basis.identity))?;
    let coast_tick_count = (coast.duration_s * f64::from(policy.physics_hz)).round() as u64;
    let context = CandidateProfileRunContext {
        scenario,
        probe,
        selected,
        basis,
        policy,
        vehicle,
        frozen_tilt_attitude_rad: launch_tilt_attitude_rad,
        coast_tick_count,
    };
    run_cadence_with_variant_and_hold_mode_and_schedule(
        &context,
        MaterializedCadenceRequest {
            cadence,
            source_bridge_steps,
            launch_tilt_attitude_rad,
            stage,
            hold_mode,
            capture_state_samples,
            held_source_schedule,
        },
    )
}

fn run_cadence_with_variant(
    candidate: &CandidateProfileRunContext<'_>,
    cadence: RolloutCadence,
    source_bridge_steps: u64,
    launch_tilt_attitude_rad: f64,
    stage: SourceDurationReplayStage,
) -> Result<LaunchFeasibilityCadenceRunEvidence> {
    Ok(run_cadence_with_variant_and_hold_mode(
        candidate,
        cadence,
        source_bridge_steps,
        launch_tilt_attitude_rad,
        stage,
        SourceDurationHoldMode::Together,
        false,
    )?
    .run)
}

fn run_cadence_with_variant_and_hold_mode(
    candidate: &CandidateProfileRunContext<'_>,
    cadence: RolloutCadence,
    source_bridge_steps: u64,
    launch_tilt_attitude_rad: f64,
    stage: SourceDurationReplayStage,
    hold_mode: SourceDurationHoldMode,
    capture_state_samples: bool,
) -> Result<SourceDurationDiagnosticCadenceRun> {
    run_cadence_with_variant_and_hold_mode_and_schedule(
        candidate,
        MaterializedCadenceRequest {
            cadence,
            source_bridge_steps,
            launch_tilt_attitude_rad,
            stage,
            hold_mode,
            capture_state_samples,
            held_source_schedule: None,
        },
    )
}

fn run_cadence_with_variant_and_hold_mode_and_schedule(
    candidate: &CandidateProfileRunContext<'_>,
    request: MaterializedCadenceRequest<'_>,
) -> Result<SourceDurationDiagnosticCadenceRun> {
    let MaterializedCadenceRequest {
        cadence,
        source_bridge_steps,
        launch_tilt_attitude_rad,
        stage,
        hold_mode,
        capture_state_samples,
        held_source_schedule,
    } = request;
    let CandidateProfileRunContext {
        scenario,
        probe,
        basis,
        policy,
        vehicle,
        ..
    } = *candidate;
    let context = RunContext::from_scenario(scenario).map_err(anyhow::Error::msg)?;
    if context.sim.physics_hz != policy.physics_hz
        || context.sim.controller_hz != 60
        || context.sim.control_interval_steps() != 2
    {
        bail!("launch-feasibility scenario no longer matches frozen 120 Hz / 60 Hz cadence");
    }
    let interval_steps = context.sim.control_interval_steps();
    if let Some(schedule) = held_source_schedule {
        if cadence != RolloutCadence::ControllerCadence
            || !source_bridge_steps.is_multiple_of(interval_steps)
            || schedule.len() != (source_bridge_steps / interval_steps) as usize
        {
            bail!("paired source schedule length or cadence does not match the frozen bridge");
        }
        if schedule.iter().any(|command| {
            !command.thrust_acceleration_mps2.x.is_finite()
                || !command.thrust_acceleration_mps2.y.is_finite()
                || !command.target_attitude_rad.is_finite()
        }) {
            bail!("paired source schedule contains a non-finite command");
        }
    }
    let mut state = SimulationState::new(&context)?;
    let dt_s = context.sim.physics_dt_s();
    let mut saturation = empty_saturation();
    let mut launch_samples = Vec::with_capacity(LAUNCH_TICKS as usize);
    let mut state_samples = Vec::new();
    let mut command_samples = Vec::new();
    let mut launch_contact_free = true;
    for local_tick in 0..LAUNCH_TICKS {
        if state.is_terminal() {
            break;
        }
        let desired_attitude = if local_tick < UPRIGHT_TICKS {
            0.0
        } else {
            launch_tilt_attitude_rad
        };
        let (throttle_update_due, attitude_update_due) =
            component_update_schedule(hold_mode, cadence, state.physics_step, interval_steps);
        if throttle_update_due || attitude_update_due {
            let held = state.held_command;
            state.set_command(Command {
                throttle_frac: if throttle_update_due {
                    1.0
                } else {
                    held.throttle_frac
                },
                target_attitude_rad: if attitude_update_due {
                    desired_attitude
                } else {
                    held.target_attitude_rad
                },
            });
            saturation.commanded_update_count += 1;
        }
        let attitude_before = state.attitude_rad;
        let pre_step_fuel = state.fuel_kg;
        let applied_throttle = plant_applied_throttle(
            state.held_command,
            context.vehicle.min_throttle_frac,
            pre_step_fuel,
        );
        update_fuel_accounting(
            &mut saturation,
            pre_step_fuel,
            applied_throttle,
            dt_s,
            context.vehicle.max_fuel_burn_kgps,
            1.0,
        );
        let events = state.step(&context);
        let contact = contact_label(&events);
        if contact != "none" {
            launch_contact_free = false;
        }
        let observation = state.build_observation(&context);
        launch_samples.push(LaunchTickEvidence {
            physics_step: state.physics_step,
            phase: if local_tick < UPRIGHT_TICKS {
                "upright".to_owned()
            } else {
                "tilt".to_owned()
            },
            contact_classification: contact.to_owned(),
            commanded_throttle_frac: 1.0,
            commanded_target_attitude_rad: desired_attitude,
            held_target_attitude_rad: state.held_command.target_attitude_rad,
            attitude_before_step_rad: attitude_before,
            attitude_after_step_rad: state.attitude_rad,
            angular_rate_radps: state.angular_rate_radps,
            position_m: state.position_m,
            velocity_mps: state.velocity_mps,
            touchdown_clearance_m: observation.touchdown_clearance_m,
            hull_clearance_m: observation.min_hull_clearance_m,
            fuel_kg: state.fuel_kg,
            sim_time_s: state.sim_time_s,
        });
        if !launch_contact_free {
            break;
        }
    }
    let launch_completed =
        state.physics_step == LAUNCH_TICKS && !state.is_terminal() && launch_contact_free;
    let launch = LaunchEvidence {
        requested_physics_ticks: LAUNCH_TICKS,
        physics_ticks_completed: state.physics_step,
        completed: launch_completed,
        contact_free: launch_contact_free,
        commanded_tilt_attitude_rad: launch_tilt_attitude_rad,
        end_state: launch_completed.then(|| plant_state(&state, context.vehicle.initial_fuel_kg)),
        samples: launch_samples,
    };

    let mut bridge_evidence = None;
    let rollout = if launch_completed {
        let source_start = KinematicStateV2 {
            position_m: state.position_m,
            velocity_mps: state.velocity_mps,
        };
        let handoff = basis
            .source_handoff
            .ok_or_else(|| anyhow!("candidate {} has no source handoff", basis.identity))?;
        let steps = source_bridge_steps;
        let source_bridge = match exact_discrete_bridge_v2(
            policy,
            vehicle,
            BridgeKindV2::Source,
            source_start,
            handoff.state,
            steps,
        ) {
            Ok(bridge) => bridge,
            Err(error) => {
                let failed = failed_bridge_evidence(error, steps);
                bridge_evidence = Some(failed);
                let mut placeholder = empty_rollout(
                    "reseeded_bridge_not_materialized",
                    &state,
                    &context,
                    &saturation,
                );
                append_launch_contacts(&mut placeholder, &launch);
                return Ok(SourceDurationDiagnosticCadenceRun {
                    run: LaunchFeasibilityCadenceRunEvidence {
                        cadence: cadence_name(cadence).to_owned(),
                        launch,
                        reseeded_bridge: bridge_evidence,
                        rollout: placeholder,
                    },
                    state_samples,
                    command_samples,
                });
            }
        };
        let bridge_is_finite = bridge_is_finite(&source_bridge);
        let source_clearance = source_clearance_screen(
            &source_bridge,
            &probe.source,
            vehicle,
            policy,
            &scenario.world.terrain,
        );
        let source_attitude_margin = source_attitude_margin(&source_bridge, vehicle, policy);
        let launch_boundary = launch_boundary(&state, &source_bridge, vehicle, policy);
        let launch_fuel_burn = context.vehicle.initial_fuel_kg - state.fuel_kg;
        let terminal_fuel_burn = basis
            .terminal_bridge
            .as_ref()
            .map(|terminal| terminal.fuel_burn_kg)
            .unwrap_or(f64::MAX);
        let conservative_total_fuel_burn =
            launch_fuel_burn + source_bridge.fuel_burn_kg + terminal_fuel_burn;
        let fuel_margin = upper_margin(
            context.vehicle.initial_fuel_kg,
            conservative_total_fuel_burn,
        );
        let frozen_coast_duration = basis
            .selected_coast
            .as_ref()
            .map_or(0.0, |coast| coast.duration_s);
        let total_time_s = LAUNCH_TICKS as f64 / f64::from(policy.physics_hz)
            + source_bridge.duration_s
            + frozen_coast_duration
            + basis
                .terminal_bridge
                .as_ref()
                .map_or(0.0, |terminal| terminal.duration_s);
        let time_margin = upper_margin(policy.mission_budget_s(), total_time_s);
        let margins_pass = source_bridge.classification == CertificationV2::Certified
            && source_clearance.all_samples_passed
            && source_attitude_margin.is_some_and(|margin| margin.passes_declared_screen)
            && launch_boundary
                .slew_margin
                .is_some_and(|margin| margin.passes_declared_screen)
            && margin_passes(fuel_margin, policy)
            && margin_passes(time_margin, policy);
        let analytic_eligible =
            bridge_is_finite && margins_pass && basis.classification == CertificationV2::Certified;
        bridge_evidence = Some(ReseededBridgeEvidence {
            solve_status: if bridge_is_finite {
                "materialized".to_owned()
            } else {
                "non_finite_diagnostic_only".to_owned()
            },
            solve_error: None,
            identity: Some(source_bridge.identity.clone()),
            classification: Some(source_bridge.classification),
            reasons: source_bridge.reasons.clone(),
            start_state: Some(source_bridge.start_state),
            end_state: Some(source_bridge.end_state),
            tick_count: source_bridge.steps,
            endpoint_position_error_m: Some(source_bridge.endpoint_position_error_m),
            endpoint_velocity_error_mps: Some(source_bridge.endpoint_velocity_error_mps),
            fuel_burn_kg: Some(source_bridge.fuel_burn_kg),
            margins: Some(source_bridge.margins),
            source_clearance: Some(source_clearance),
            source_attitude_margin,
            launch_boundary: Some(launch_boundary),
            aggregate_fuel_margin: Some(margin_evidence(fuel_margin, policy)),
            aggregate_time_margin: Some(margin_evidence(time_margin, policy)),
            analytically_eligible: analytic_eligible,
            diagnostic_replay_allowed: bridge_is_finite,
        });
        if bridge_is_finite && stage == SourceDurationReplayStage::AnalyzeOnly {
            let mut rollout =
                empty_rollout("analytical_screen_complete", &state, &context, &saturation);
            append_launch_contacts(&mut rollout, &launch);
            rollout
        } else if bridge_is_finite {
            run_materialized_rollout(MaterializedRolloutRequest {
                candidate,
                bridge: &source_bridge,
                state_after_launch: &state,
                context: &context,
                saturation: &mut saturation,
                rollout_policy: MaterializedRolloutPolicy {
                    cadence,
                    stop_at_source_handoff: stage == SourceDurationReplayStage::SourceHandoff,
                    hold_mode,
                },
                held_source_schedule,
                diagnostic_capture: SourceDurationDiagnosticCapture {
                    state_samples: capture_state_samples.then_some(&mut state_samples),
                    command_samples: capture_state_samples.then_some(&mut command_samples),
                },
            })?
        } else {
            let mut rollout =
                empty_rollout("reseeded_bridge_non_finite", &state, &context, &saturation);
            append_launch_contacts(&mut rollout, &launch);
            rollout
        }
    } else {
        let status = if launch_contact_free {
            "simulator_terminated_during_launch"
        } else {
            "launch_contact_observed"
        };
        let mut rollout = empty_rollout(status, &state, &context, &saturation);
        append_launch_contacts(&mut rollout, &launch);
        rollout
    };
    Ok(SourceDurationDiagnosticCadenceRun {
        run: LaunchFeasibilityCadenceRunEvidence {
            cadence: cadence_name(cadence).to_owned(),
            launch,
            reseeded_bridge: bridge_evidence,
            rollout,
        },
        state_samples,
        command_samples,
    })
}

pub(super) fn component_update_schedule(
    hold_mode: SourceDurationHoldMode,
    cadence: RolloutCadence,
    physics_step: u64,
    control_interval_steps: u64,
) -> (bool, bool) {
    let cadence_update_due = cadence.update_due(physics_step, control_interval_steps);
    match hold_mode {
        SourceDurationHoldMode::Together => (cadence_update_due, cadence_update_due),
        SourceDurationHoldMode::ThrottleHeldAttitudePerTick => (cadence_update_due, true),
        SourceDurationHoldMode::AttitudeHeldThrottlePerTick => (true, cadence_update_due),
    }
}

fn source_duration_state_sample(state: &SimulationState, phase: &str) -> SourceDurationStateSample {
    SourceDurationStateSample {
        physics_step: state.physics_step,
        phase: phase.to_owned(),
        position_m: state.position_m,
        velocity_mps: state.velocity_mps,
        attitude_rad: state.attitude_rad,
        angular_rate_radps: state.angular_rate_radps,
        fuel_kg: state.fuel_kg,
    }
}

fn run_materialized_rollout(
    request: MaterializedRolloutRequest<'_, '_>,
) -> Result<LaunchRolloutEvidence> {
    let MaterializedRolloutRequest {
        candidate,
        bridge,
        state_after_launch,
        context,
        saturation,
        rollout_policy,
        held_source_schedule,
        diagnostic_capture,
    } = request;
    let MaterializedRolloutPolicy {
        cadence,
        stop_at_source_handoff,
        hold_mode,
    } = rollout_policy;
    let SourceDurationDiagnosticCapture {
        mut state_samples,
        mut command_samples,
    } = diagnostic_capture;
    let CandidateProfileRunContext {
        scenario: _,
        probe: _,
        selected,
        basis,
        coast_tick_count,
        ..
    } = *candidate;
    let mut state = state_after_launch.clone();
    if state.physics_step != LAUNCH_TICKS {
        bail!(
            "candidate {} materialized replay expected launch boundary at tick {LAUNCH_TICKS}, got {}",
            basis.identity,
            state.physics_step
        );
    }
    let interval_steps = context.sim.control_interval_steps();
    let dt_s = context.sim.physics_dt_s();
    let source_steps = bridge.steps;
    let tail_start = selected.profile.accounting.source_bridge_sample_count as usize;
    let planned_flight_ticks =
        source_steps + coast_tick_count + selected.profile.accounting.terminal_bridge_sample_count;
    let profile_tick_count = LAUNCH_TICKS + planned_flight_ticks;
    let source_end_step = LAUNCH_TICKS + source_steps;
    let coast_end_step = source_end_step + coast_tick_count;
    let nominal_end_step = profile_tick_count;
    let source_handoff = basis
        .source_handoff
        .ok_or_else(|| anyhow!("candidate {} lost source handoff", basis.identity))?;
    let coast = basis
        .selected_coast
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} lost selected coast", basis.identity))?;
    let terminal_handoff = coast.terminal_handoff;
    let source_angles = bridge_command_angles(bridge);
    let mut first_divergence = None;
    let mut max_position_error_m = 0.0_f64;
    let mut max_velocity_error_mps = 0.0_f64;
    let mut contact_transition_state_errors = Vec::new();
    let mut source_handoff_reached = false;
    let mut source_handoff_contact_free_so_far = true;
    let mut source_handoff_position_error_m = None;
    let mut source_handoff_velocity_error_mps = None;
    let mut first_source_tick = None;
    let mut phase_handoff_errors = Vec::with_capacity(3);
    let mut contacts = Vec::new();
    let mut per_step = Vec::new();
    let mut profile_end = None;
    let mut min_touchdown_clearance = f64::INFINITY;
    let mut min_hull_clearance = f64::INFINITY;

    while !state.is_terminal() {
        let physics_step = state.physics_step;
        if physics_step >= profile_tick_count {
            let (throttle_update_due, attitude_update_due) =
                component_update_schedule(hold_mode, cadence, physics_step, interval_steps);
            if throttle_update_due || attitude_update_due {
                let held = state.held_command;
                state.set_command(Command {
                    throttle_frac: if throttle_update_due {
                        0.0
                    } else {
                        held.throttle_frac
                    },
                    target_attitude_rad: if attitude_update_due {
                        selected.profile.final_target_attitude_rad
                    } else {
                        held.target_attitude_rad
                    },
                });
            }
            let attitude_before = state.attitude_rad;
            let actual_command = state.held_command;
            let applied_throttle = plant_applied_throttle(
                actual_command,
                context.vehicle.min_throttle_frac,
                state.fuel_kg,
            );
            let events = state.step(context);
            let contact = contact_label(&events);
            if contact != "none" {
                contacts.push(ContactEvidence {
                    physics_step: state.physics_step,
                    kind: contact.to_owned(),
                });
            }
            let observation = state.build_observation(context);
            min_touchdown_clearance =
                min_touchdown_clearance.min(observation.touchdown_clearance_m);
            min_hull_clearance = min_hull_clearance.min(observation.min_hull_clearance_m);
            per_step.push(LaunchRolloutTickEvidence {
                physics_step: state.physics_step,
                phase: "post_profile_idle".to_owned(),
                contact_classification: contact.to_owned(),
                desired_target_attitude_rad: selected.profile.final_target_attitude_rad,
                held_target_attitude_rad: state.held_command.target_attitude_rad,
                attitude_before_step_rad: attitude_before,
                attitude_after_step_rad: state.attitude_rad,
                angular_rate_radps: state.angular_rate_radps,
                commanded_throttle_frac: actual_command.throttle_frac,
                applied_throttle_frac: applied_throttle,
                state_position_error_m: None,
                state_velocity_error_mps: None,
                touchdown_clearance_m: observation.touchdown_clearance_m,
                hull_clearance_m: observation.min_hull_clearance_m,
                fuel_kg: state.fuel_kg,
                sim_time_s: state.sim_time_s,
            });
            if let Some(samples) = state_samples.as_deref_mut()
                && !state.is_terminal()
            {
                samples.push(source_duration_state_sample(&state, "post_profile_idle"));
            }
            if let Some(samples) = command_samples.as_deref_mut() {
                samples.push(SourceDurationCommandSample {
                    physics_step: state.physics_step,
                    phase: "post_profile_idle".to_owned(),
                    desired_command_throttle_frac: 0.0,
                    desired_applied_throttle_frac: 0.0,
                    held_command_throttle_frac: actual_command.throttle_frac,
                    plant_applied_throttle_frac: applied_throttle,
                    desired_target_attitude_rad: selected.profile.final_target_attitude_rad,
                    held_target_attitude_rad: actual_command.target_attitude_rad,
                    throttle_update_due,
                    attitude_update_due,
                });
            }
            continue;
        }

        let flight_index = (physics_step - LAUNCH_TICKS) as usize;
        let (phase, expected_state, thrust_acceleration, target_attitude) =
            if flight_index < source_steps as usize {
                let sample = bridge.samples[flight_index];
                let (thrust_acceleration, target_attitude) = held_source_schedule
                    .and_then(|schedule| schedule.get(flight_index / interval_steps as usize))
                    .map_or(
                        (sample.thrust_acceleration_mps2, source_angles[flight_index]),
                        |command| {
                            (
                                command.thrust_acceleration_mps2,
                                command.target_attitude_rad,
                            )
                        },
                    );
                (
                    "source_bridge",
                    Some(sample.state_m),
                    thrust_acceleration,
                    target_attitude,
                )
            } else {
                let tail_index = tail_start + flight_index - source_steps as usize;
                let tick = selected.profile.ticks.get(tail_index).ok_or_else(|| {
                    anyhow!(
                        "candidate {} frozen coast/terminal tail is truncated",
                        basis.identity
                    )
                })?;
                (
                    tick.phase.as_str(),
                    Some(tick.expected_state),
                    tick.thrust_acceleration_mps2,
                    tick.target_attitude_rad,
                )
            };
        let desired = throttle_request(
            thrust_acceleration.length(),
            state.mass_kg(context),
            context.vehicle.max_thrust_n,
            context.vehicle.max_fuel_burn_kgps,
            dt_s,
            context.vehicle.min_throttle_frac,
        )?;
        let (throttle_update_due, attitude_update_due) =
            component_update_schedule(hold_mode, cadence, physics_step, interval_steps);
        if throttle_update_due || attitude_update_due {
            let held = state.held_command;
            state.set_command(Command {
                throttle_frac: if throttle_update_due {
                    desired.command_fraction
                } else {
                    held.throttle_frac
                },
                target_attitude_rad: if attitude_update_due {
                    target_attitude
                } else {
                    held.target_attitude_rad
                },
            });
            saturation.commanded_update_count += 1;
            if throttle_update_due {
                record_throttle_saturation(saturation, desired.saturation);
            }
        }
        let actual_command = state.held_command;
        let actual_applied = plant_applied_throttle(
            actual_command,
            context.vehicle.min_throttle_frac,
            state.fuel_kg,
        );
        let pre_step_fuel = state.fuel_kg;
        update_fuel_accounting(
            saturation,
            pre_step_fuel,
            actual_applied,
            dt_s,
            context.vehicle.max_fuel_burn_kgps,
            if hold_mode == SourceDurationHoldMode::Together {
                desired.applied_fraction
            } else {
                actual_applied
            },
        );
        let held_command_mismatch = (actual_command.throttle_frac - desired.command_fraction).abs()
            > 1.0e-12
            || shortest_angle_delta(actual_command.target_attitude_rad, target_attitude).abs()
                > 1.0e-12;
        let attitude_before = state.attitude_rad;
        let events = state.step(context);
        let contact = contact_label(&events);
        if contact != "none" {
            contacts.push(ContactEvidence {
                physics_step: state.physics_step,
                kind: contact.to_owned(),
            });
            if state.physics_step <= source_end_step {
                source_handoff_contact_free_so_far = false;
            }
        }
        let observation = state.build_observation(context);
        min_touchdown_clearance = min_touchdown_clearance.min(observation.touchdown_clearance_m);
        min_hull_clearance = min_hull_clearance.min(observation.min_hull_clearance_m);
        let (position_error, velocity_error) = if let Some(expected) = expected_state {
            let position_error = (state.position_m - expected.position_m).length();
            let velocity_error = (state.velocity_mps - expected.velocity_mps).length();
            if contact == "none" {
                max_position_error_m = max_position_error_m.max(position_error);
                max_velocity_error_mps = max_velocity_error_mps.max(velocity_error);
                if (position_error > STATE_POSITION_TOLERANCE_M
                    || velocity_error > STATE_VELOCITY_TOLERANCE_MPS)
                    && first_divergence.is_none()
                {
                    let throttle_saturation_mismatch =
                        (actual_applied - desired.applied_fraction).abs() > 1.0e-6;
                    let attitude_slew_mismatch =
                        shortest_angle_delta(state.attitude_rad, target_attitude).abs() > 1.0e-6;
                    let cause_count = usize::from(held_command_mismatch)
                        + usize::from(throttle_saturation_mismatch)
                        + usize::from(attitude_slew_mismatch);
                    let cause = if cause_count != 1 {
                        "unclassified".to_owned()
                    } else if held_command_mismatch {
                        "held_command_mismatch".to_owned()
                    } else if throttle_saturation_mismatch {
                        "throttle_saturation".to_owned()
                    } else {
                        "attitude_slew".to_owned()
                    };
                    first_divergence = Some(FirstDivergenceEvidence {
                        profile_tick_index: physics_step - LAUNCH_TICKS,
                        physics_step: state.physics_step,
                        phase: phase.to_owned(),
                        cause,
                        position_error_m: position_error,
                        velocity_error_mps: velocity_error,
                        desired_target_attitude_rad: target_attitude,
                        actual_attitude_before_step_rad: attitude_before,
                        actual_attitude_after_step_rad: state.attitude_rad,
                        desired_applied_throttle_frac: desired.applied_fraction,
                        commanded_throttle_frac: actual_command.throttle_frac,
                        applied_throttle_frac: actual_applied,
                        held_command_mismatch,
                        fuel_burn_capped: pre_step_fuel > 0.0
                            && context.vehicle.max_fuel_burn_kgps * actual_applied * dt_s
                                > pre_step_fuel + f64::EPSILON,
                    });
                }
            } else {
                contact_transition_state_errors.push(ContactTransitionStateErrorEvidence {
                    physics_step: state.physics_step,
                    phase: phase.to_owned(),
                    contact_classification: contact.to_owned(),
                    expected_position_error_m: position_error,
                    expected_velocity_error_mps: velocity_error,
                });
            }
            (Some(position_error), Some(velocity_error))
        } else {
            (None, None)
        };
        if physics_step == LAUNCH_TICKS {
            first_source_tick = Some(FirstSourceTickEvidence {
                physics_step: state.physics_step,
                desired_target_attitude_rad: target_attitude,
                held_target_attitude_rad: actual_command.target_attitude_rad,
                attitude_before_step_rad: attitude_before,
                attitude_after_step_rad: state.attitude_rad,
                angular_rate_radps: state.angular_rate_radps,
                commanded_throttle_frac: actual_command.throttle_frac,
                applied_throttle_frac: actual_applied,
            });
        }
        per_step.push(LaunchRolloutTickEvidence {
            physics_step: state.physics_step,
            phase: phase.to_owned(),
            contact_classification: contact.to_owned(),
            desired_target_attitude_rad: target_attitude,
            held_target_attitude_rad: actual_command.target_attitude_rad,
            attitude_before_step_rad: attitude_before,
            attitude_after_step_rad: state.attitude_rad,
            angular_rate_radps: state.angular_rate_radps,
            commanded_throttle_frac: actual_command.throttle_frac,
            applied_throttle_frac: actual_applied,
            state_position_error_m: position_error,
            state_velocity_error_mps: velocity_error,
            touchdown_clearance_m: observation.touchdown_clearance_m,
            hull_clearance_m: observation.min_hull_clearance_m,
            fuel_kg: state.fuel_kg,
            sim_time_s: state.sim_time_s,
        });
        if let Some(samples) = state_samples.as_deref_mut()
            && !state.is_terminal()
        {
            samples.push(source_duration_state_sample(&state, phase));
        }
        if let Some(samples) = command_samples.as_deref_mut() {
            samples.push(SourceDurationCommandSample {
                physics_step: state.physics_step,
                phase: phase.to_owned(),
                desired_command_throttle_frac: desired.command_fraction,
                desired_applied_throttle_frac: desired.applied_fraction,
                held_command_throttle_frac: actual_command.throttle_frac,
                plant_applied_throttle_frac: actual_applied,
                desired_target_attitude_rad: target_attitude,
                held_target_attitude_rad: actual_command.target_attitude_rad,
                throttle_update_due,
                attitude_update_due,
            });
        }

        for (boundary, boundary_step, expected) in [
            ("source_handoff", source_end_step, source_handoff.state),
            ("coast_handoff", coast_end_step, terminal_handoff.state),
            (
                "nominal_profile_end",
                nominal_end_step,
                basis
                    .terminal_bridge
                    .as_ref()
                    .expect("frozen terminal bridge")
                    .end_state,
            ),
        ] {
            if state.physics_step == boundary_step {
                let position_error_m = (state.position_m - expected.position_m).length();
                let velocity_error_mps = (state.velocity_mps - expected.velocity_mps).length();
                phase_handoff_errors.push(PhaseHandoffErrorEvidence {
                    boundary: boundary.to_owned(),
                    physics_step: state.physics_step,
                    position_error_m,
                    velocity_error_mps,
                });
                if boundary == "source_handoff" {
                    source_handoff_reached = position_error_m <= STATE_POSITION_TOLERANCE_M
                        && velocity_error_mps <= STATE_VELOCITY_TOLERANCE_MPS;
                    source_handoff_position_error_m = Some(position_error_m);
                    source_handoff_velocity_error_mps = Some(velocity_error_mps);
                }
            }
        }
        if state.physics_step == nominal_end_step {
            profile_end = Some(plant_state(&state, context.vehicle.initial_fuel_kg));
        }
        if stop_at_source_handoff && state.physics_step == source_end_step {
            return Ok(LaunchRolloutEvidence {
                status: "source_handoff_gate_complete".to_owned(),
                source_handoff_reached,
                source_handoff_contact_free: source_handoff_reached
                    && source_handoff_contact_free_so_far,
                source_handoff_position_error_m,
                source_handoff_velocity_error_mps,
                profile_tick_count,
                physics_steps_advanced: state.physics_step,
                first_source_tick,
                first_divergence,
                max_position_error_m,
                max_velocity_error_mps,
                contact_transition_state_errors,
                phase_handoff_errors,
                minimum_touchdown_clearance_m: finite_option(min_touchdown_clearance),
                minimum_hull_clearance_m: finite_option(min_hull_clearance),
                contacts,
                per_step,
                profile_end,
                termination: plant_state(&state, context.vehicle.initial_fuel_kg),
                saturation: saturation.clone(),
            });
        }
    }

    if !source_handoff_reached {
        source_handoff_contact_free_so_far = false;
    }
    let status = if profile_end.is_none() {
        "simulator_terminated_before_profile_end"
    } else if state.physics_step == nominal_end_step {
        "simulator_terminated_at_profile_end"
    } else {
        "simulator_terminated_during_post_profile_idle"
    };
    Ok(LaunchRolloutEvidence {
        status: status.to_owned(),
        source_handoff_reached,
        source_handoff_contact_free: source_handoff_reached && source_handoff_contact_free_so_far,
        source_handoff_position_error_m,
        source_handoff_velocity_error_mps,
        profile_tick_count,
        physics_steps_advanced: state.physics_step,
        first_source_tick,
        first_divergence,
        max_position_error_m,
        max_velocity_error_mps,
        contact_transition_state_errors,
        phase_handoff_errors,
        minimum_touchdown_clearance_m: finite_option(min_touchdown_clearance),
        minimum_hull_clearance_m: finite_option(min_hull_clearance),
        contacts,
        per_step,
        profile_end,
        termination: plant_state(&state, context.vehicle.initial_fuel_kg),
        saturation: saturation.clone(),
    })
}

fn empty_rollout(
    status: &str,
    state: &SimulationState,
    context: &RunContext,
    saturation: &CommandSaturationEvidence,
) -> LaunchRolloutEvidence {
    LaunchRolloutEvidence {
        status: status.to_owned(),
        source_handoff_reached: false,
        source_handoff_contact_free: false,
        source_handoff_position_error_m: None,
        source_handoff_velocity_error_mps: None,
        profile_tick_count: 0,
        physics_steps_advanced: state.physics_step,
        first_source_tick: None,
        first_divergence: None,
        max_position_error_m: 0.0,
        max_velocity_error_mps: 0.0,
        contact_transition_state_errors: Vec::new(),
        phase_handoff_errors: Vec::new(),
        minimum_touchdown_clearance_m: finite_option(state.min_touchdown_clearance_m),
        minimum_hull_clearance_m: finite_option(state.min_hull_clearance_m),
        contacts: Vec::new(),
        per_step: Vec::new(),
        profile_end: None,
        termination: plant_state(state, context.vehicle.initial_fuel_kg),
        saturation: saturation.clone(),
    }
}

fn append_launch_contacts(rollout: &mut LaunchRolloutEvidence, launch: &LaunchEvidence) {
    rollout.contacts.extend(
        launch
            .samples
            .iter()
            .filter(|sample| sample.contact_classification != "none")
            .map(|sample| ContactEvidence {
                physics_step: sample.physics_step,
                kind: sample.contact_classification.clone(),
            }),
    );
}

fn failed_bridge_evidence(error: String, steps: u64) -> ReseededBridgeEvidence {
    ReseededBridgeEvidence {
        solve_status: "rejected".to_owned(),
        solve_error: Some(error),
        identity: None,
        classification: None,
        reasons: Vec::new(),
        start_state: None,
        end_state: None,
        tick_count: steps,
        endpoint_position_error_m: None,
        endpoint_velocity_error_mps: None,
        fuel_burn_kg: None,
        margins: None,
        source_clearance: None,
        source_attitude_margin: None,
        launch_boundary: None,
        aggregate_fuel_margin: None,
        aggregate_time_margin: None,
        analytically_eligible: false,
        diagnostic_replay_allowed: false,
    }
}

fn source_clearance_screen(
    bridge: &AnalyticalBridgeV2,
    source_pad: &pd_plan::conservative_ballistic_bridge::PadInputV2,
    vehicle: &super::VehicleInputV2,
    policy: &DirectBridgePolicyV2,
    terrain: &pd_core::TerrainDefinition,
) -> SourceClearanceScreenEvidence {
    let screens = bridge
        .samples
        .iter()
        .map(|sample| {
            mirror_v2_clearance(
                sample.state_m,
                sample.thrust_acceleration_mps2,
                source_pad,
                vehicle,
                policy,
                terrain,
            )
        })
        .collect::<Vec<_>>();
    let minimum = screens
        .iter()
        .map(|screen| screen.margin)
        .reduce(min_margin)
        .unwrap_or_else(failed_margin);
    SourceClearanceScreenEvidence {
        evaluated_tick_count: screens.len() as u64,
        source_pad_center_height_tick_count: screens
            .iter()
            .filter(|screen| screen.mode.as_str() == "source_pad_center_height")
            .count() as u64,
        rotated_hull_clearance_tick_count: screens
            .iter()
            .filter(|screen| screen.mode.as_str() == "rotated_hull_clearance")
            .count() as u64,
        minimum_margin: margin_evidence(minimum, policy),
        all_samples_passed: !screens.is_empty()
            && screens.iter().all(|screen| screen.passes_declared_screen),
    }
}

fn source_attitude_margin(
    bridge: &AnalyticalBridgeV2,
    vehicle: &super::VehicleInputV2,
    policy: &DirectBridgePolicyV2,
) -> Option<MarginEvidence> {
    bridge
        .samples
        .iter()
        .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
        .map(|sample| {
            let attitude = direction_angle(sample.thrust_acceleration_mps2);
            margin_evidence(
                upper_margin(vehicle.safe_touchdown_attitude_error_rad, attitude.abs()),
                policy,
            )
        })
}

fn launch_boundary(
    state: &SimulationState,
    bridge: &AnalyticalBridgeV2,
    vehicle: &super::VehicleInputV2,
    policy: &DirectBridgePolicyV2,
) -> LaunchBoundaryEvidence {
    let first_angle = bridge
        .samples
        .iter()
        .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
        .map(|sample| direction_angle(sample.thrust_acceleration_mps2));
    let attitude_delta_rad =
        first_angle.map(|angle| shortest_angle_delta(state.attitude_rad, angle).abs());
    let required_rate = attitude_delta_rad.map(|delta| delta * f64::from(policy.physics_hz));
    let slew_margin = required_rate
        .map(|rate| margin_evidence(upper_margin(vehicle.max_rotation_rate_radps, rate), policy));
    LaunchBoundaryEvidence {
        launch_end_attitude_rad: state.attitude_rad,
        launch_end_angular_rate_radps: state.angular_rate_radps,
        first_reseeded_powered_attitude_rad: first_angle,
        attitude_delta_rad,
        required_one_tick_slew_rate_radps: required_rate,
        maximum_rotation_rate_radps: vehicle.max_rotation_rate_radps,
        slew_margin,
    }
}

fn bridge_command_angles(bridge: &AnalyticalBridgeV2) -> Vec<f64> {
    let mut next = None;
    let mut targets = vec![None; bridge.samples.len()];
    for index in (0..bridge.samples.len()).rev() {
        let sample = bridge.samples[index];
        if sample.thrust_acceleration_mps2.length() > 1.0e-12 {
            next = Some(direction_angle(sample.thrust_acceleration_mps2));
        }
        targets[index] = next;
    }
    let mut previous = 0.0;
    targets
        .into_iter()
        .map(|target| {
            if let Some(target) = target {
                previous = target;
            }
            previous
        })
        .collect()
}

fn frozen_first_powered_attitude(selected: &PreparedProfileCandidate) -> Result<f64> {
    selected
        .profile
        .ticks
        .iter()
        .find(|tick| {
            tick.phase == super::NominalPhase::SourceBridge
                && tick.thrust_acceleration_mps2.length() > 1.0e-12
        })
        .map(|tick| direction_angle(tick.thrust_acceleration_mps2))
        .ok_or_else(|| {
            anyhow!(
                "selected source profile {} has no frozen nonzero source thrust sample",
                selected.candidate_identity
            )
        })
}

fn candidate_for<'a>(
    case: &'a PreparedCase,
    selected: &PreparedProfileCandidate,
) -> Result<&'a DirectBridgeCandidateV2> {
    case.result
        .candidates
        .iter()
        .find(|candidate| candidate.identity == selected.candidate_identity)
        .ok_or_else(|| {
            anyhow!(
                "frozen basis candidate {} is missing",
                selected.candidate_identity
            )
        })
}

fn selection_roles_for(
    case: &PreparedCase,
    nominal: &WaypointDirectNominalPlantArtifact,
) -> Result<Vec<SelectionRoleEvidence>> {
    nominal
        .cases
        .iter()
        .find(|nominal_case| nominal_case.input.id == case.evidence.id)
        .map(|nominal_case| nominal_case.selection_roles.clone())
        .ok_or_else(|| anyhow!("nominal input is missing case {}", case.evidence.id))
}

#[derive(Serialize)]
struct ProfileTickIdentity<'a> {
    phase: &'a str,
    position_m: Vec2,
    velocity_mps: Vec2,
    thrust_acceleration_mps2: Vec2,
    target_attitude_rad: f64,
}

fn profile_tick_identity(selected: &PreparedProfileCandidate) -> Result<String> {
    let values = selected
        .profile
        .ticks
        .iter()
        .map(|tick| ProfileTickIdentity {
            phase: tick.phase.as_str(),
            position_m: tick.expected_state.position_m,
            velocity_mps: tick.expected_state.velocity_mps,
            thrust_acceleration_mps2: tick.thrust_acceleration_mps2,
            target_attitude_rad: tick.target_attitude_rad,
        })
        .collect::<Vec<_>>();
    stable_digest(&values)
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

fn record_throttle_saturation(
    saturation: &mut CommandSaturationEvidence,
    class: ThrottleSaturation,
) {
    match class {
        ThrottleSaturation::BelowMinimum => saturation.below_minimum_saturation_count += 1,
        ThrottleSaturation::AboveMaximum => saturation.above_maximum_saturation_count += 1,
        ThrottleSaturation::ExactMinimumOnCommand => saturation.on_at_exact_minimum_count += 1,
        ThrottleSaturation::None => {}
    }
}

fn update_fuel_accounting(
    saturation: &mut CommandSaturationEvidence,
    pre_step_fuel_kg: f64,
    applied_throttle_frac: f64,
    dt_s: f64,
    max_fuel_burn_kgps: f64,
    requested_applied_fraction: f64,
) {
    if requested_applied_fraction <= 0.0 {
        return;
    }
    let burn_budget = max_fuel_burn_kgps * applied_throttle_frac * dt_s;
    if pre_step_fuel_kg <= 0.0 {
        saturation.fuel_exhausted_tick_count += 1;
    } else if burn_budget > pre_step_fuel_kg + f64::EPSILON {
        saturation.fuel_burn_capped_tick_count += 1;
    }
}

fn contact_label(events: &[pd_core::EventRecord]) -> &'static str {
    for event in events {
        match &event.kind {
            EventKind::TouchdownOnTarget => return "stable_touchdown_on_target",
            EventKind::TouchdownOffTarget => return "stable_touchdown_off_target",
            EventKind::Crash => return "crash",
            _ => {}
        }
    }
    "none"
}

fn plant_state(state: &SimulationState, initial_fuel_kg: f64) -> PlantStateEvidence {
    PlantStateEvidence {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        position_m: state.position_m,
        velocity_mps: state.velocity_mps,
        attitude_rad: state.attitude_rad,
        angular_rate_radps: state.angular_rate_radps,
        fuel_kg: state.fuel_kg,
        fuel_used_kg: initial_fuel_kg - state.fuel_kg,
        physical_outcome: enum_label(&state.physical_outcome),
        mission_outcome: enum_label(&state.mission_outcome),
        end_reason: enum_label(&state.end_reason),
    }
}

fn margin_evidence(margin: MarginV2, policy: &DirectBridgePolicyV2) -> MarginEvidence {
    MarginEvidence {
        raw_margin: margin.raw,
        normalized_margin: margin.normalized,
        passes_declared_screen: margin_passes(margin, policy),
    }
}

fn margin_passes(margin: MarginV2, policy: &DirectBridgePolicyV2) -> bool {
    margin.raw >= 0.0 && margin.normalized + 1.0e-12 >= policy.declared_robustness_margin
}

fn upper_margin(limit: f64, value: f64) -> MarginV2 {
    let raw = limit - value;
    MarginV2 {
        raw,
        normalized: raw / limit.abs().max(f64::EPSILON),
    }
}

fn min_margin(left: MarginV2, right: MarginV2) -> MarginV2 {
    if left.normalized.total_cmp(&right.normalized).is_le() {
        left
    } else {
        right
    }
}

fn failed_margin() -> MarginV2 {
    MarginV2 {
        raw: -f64::MAX,
        normalized: -f64::MAX,
    }
}

fn bridge_is_finite(bridge: &AnalyticalBridgeV2) -> bool {
    let finite_vec = |vector: Vec2| vector.x.is_finite() && vector.y.is_finite();
    finite_vec(bridge.start_state.position_m)
        && finite_vec(bridge.start_state.velocity_mps)
        && finite_vec(bridge.end_state.position_m)
        && finite_vec(bridge.end_state.velocity_mps)
        && bridge.duration_s.is_finite()
        && bridge.fuel_burn_kg.is_finite()
        && bridge.samples.iter().all(|sample| {
            finite_vec(sample.state_m.position_m)
                && finite_vec(sample.state_m.velocity_mps)
                && finite_vec(sample.thrust_acceleration_mps2)
                && sample.throttle_fraction.is_finite()
        })
}

fn finite_option(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn cadence_name(cadence: RolloutCadence) -> &'static str {
    match cadence {
        RolloutCadence::DirectPerTick => "direct_per_tick_120_hz",
        RolloutCadence::ControllerCadence => "held_controller_60_hz",
    }
}

pub(super) fn artifact_identity(
    artifact: &WaypointDirectLaunchFeasibilityArtifact,
) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

fn input_gate_identity(gate: &LaunchFeasibilityInputGateEvidence) -> Result<String> {
    let mut identity_input = gate.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

fn candidate_identity(candidate: &LaunchFeasibilityCandidateEvidence) -> Result<String> {
    let mut identity_input = candidate.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_hold_modes_update_only_the_declared_command_component() {
        let cadence = RolloutCadence::ControllerCadence;
        assert_eq!(
            component_update_schedule(SourceDurationHoldMode::Together, cadence, 0, 2),
            (true, true)
        );
        assert_eq!(
            component_update_schedule(SourceDurationHoldMode::Together, cadence, 1, 2),
            (false, false)
        );
        assert_eq!(
            component_update_schedule(
                SourceDurationHoldMode::ThrottleHeldAttitudePerTick,
                cadence,
                1,
                2
            ),
            (false, true)
        );
        assert_eq!(
            component_update_schedule(
                SourceDurationHoldMode::AttitudeHeldThrottlePerTick,
                cadence,
                1,
                2
            ),
            (true, false)
        );
    }

    #[test]
    fn throttle_request_depends_on_the_lane_current_mass() {
        let request = |mass| {
            throttle_request(12.5, mass, 5_000_000.0, 120.0, 1.0 / 120.0, 0.25)
                .expect("finite mass-dependent throttle request")
        };
        let lower_mass = request(110_000.0);
        let higher_mass = request(115_000.0);
        assert!(lower_mass.applied_fraction > 0.25 && lower_mass.applied_fraction < 1.0);
        assert!(higher_mass.applied_fraction > 0.25 && higher_mass.applied_fraction < 1.0);
        assert_ne!(lower_mass.command_fraction, higher_mass.command_fraction);
        assert_ne!(lower_mass.applied_fraction, higher_mass.applied_fraction);
    }

    #[test]
    fn frozen_launch_protocol_binds_even_controller_boundary_and_angle_rule() {
        let protocol = launch_protocol();
        assert_eq!(protocol.upright_full_throttle_ticks, 60);
        assert_eq!(protocol.tilt_full_throttle_ticks, 12);
        assert_eq!(protocol.launch_boundary_physics_step, 72);
        assert_eq!(protocol.launch_boundary_physics_step % 2, 0);
        assert!(
            protocol
                .tilt_angle_rule
                .contains("atan2(thrust_acceleration.x, thrust_acceleration.y)")
        );
        assert!(
            protocol
                .analytic_eligibility_rule
                .contains("never a V2 Certified")
        );
    }

    #[test]
    fn composite_candidate_identity_is_separate_from_its_v2_basis_identity() {
        let mut candidate = LaunchFeasibilityCandidateEvidence {
            identity: String::new(),
            basis_candidate_identity: "fnv1a64:frozen-v2".to_owned(),
            selected_roles: vec!["v2_native_selected".to_owned()],
            original_source_handoff_arc_step: 11,
            original_source_bridge_tick_count: 20,
            coast_tick_count: 8,
            terminal_bridge_tick_count: 18,
            frozen_first_powered_attitude_rad: 0.02,
            cadence_runs: Vec::new(),
        };
        candidate.identity = candidate_identity(&candidate).expect("candidate identity");
        assert_ne!(candidate.identity, candidate.basis_candidate_identity);
        let first_identity = candidate.identity.clone();
        candidate.frozen_first_powered_attitude_rad += 1.0e-3;
        assert_ne!(candidate_identity(&candidate).unwrap(), first_identity);
    }
}
