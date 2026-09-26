//! Evaluator-only held-cadence diagnostics for the frozen source-duration canary.
//!
//! The sealed source-duration artifact remains an input. This child lane adds
//! state residuals and two bounded command-hold ablations without changing
//! the source canary's serialized schema or upstream flight policy.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::{RunContext, Vec2};
use serde::{Deserialize, Serialize};

use super::super::{
    CommandSaturationEvidence, PlantStateEvidence, RolloutCadence,
    WaypointDirectCoupledThrustAuditInputGateEvidence, flat_candidate_closure,
    launch_contact_contract::{
        FirstContactEvidence, FirstContactPredicateMarginsEvidence, ReplayTraceParityEvidence,
        ReplayTraceResult, replay_logged_cadence, replay_logged_mixed_cadence,
    },
    launch_feasibility::{
        LaunchFeasibilityCadenceRunEvidence, SourceDurationCommandSample,
        SourceDurationDiagnosticCadenceRun, SourceDurationHoldMode, SourceDurationReplayStage,
        SourceDurationRunRequest, SourceDurationStateSample,
        run_source_duration_variant_diagnostic,
    },
    resolve_output_dir, stable_digest,
};
use super::{
    AnalyticalScreenEvidence, FullFlightEvidence, LaunchAndAnalyticalScreenEvidence,
    PreparedDurationVariant, SourceDurationFamilyProofEvidence, SourceDurationVariantEvidence,
    WaypointDirectSourceDurationCanaryArtifact, WaypointDirectSourceDurationCanaryInputPaths,
    artifact_identity as source_artifact_identity, prepare_duration_variants, read_coupled_audit,
};

pub const WAYPOINT_DIRECT_SOURCE_DURATION_HELD_CADENCE_DIAGNOSTIC_ID: &str =
    "waypoint-direct-source-duration-held-cadence-diagnostic";
pub const WAYPOINT_DIRECT_SOURCE_DURATION_HELD_CADENCE_DIAGNOSTIC_SCHEMA_ID: &str =
    "waypoint_direct_source_duration_held_cadence_diagnostic_v1";
pub const WAYPOINT_DIRECT_SOURCE_DURATION_HELD_CADENCE_DIAGNOSTIC_SCHEMA_VERSION: u32 = 1;

const EXPECTED_SOURCE_DURATION_IDENTITY: &str = "fnv1a64:dfe0f0feaa15fc24";
const EXPECTED_SOURCE_DURATION_SHA256: &str =
    "a93b728374145d4f12b43b21b7d68e383270db19facdf5964ce91bc47e557e9c";
const EXPECTED_CANDIDATE_IDENTITIES: [&str; 3] = [
    "fnv1a64:dee613017622ca16",
    "fnv1a64:4e6c0b23f9eb1b8f",
    "fnv1a64:a18a98ad6e334014",
];
const EXPECTED_CANDIDATE_ROLES: [&str; 3] = ["native", "research_shortest", "third"];
const EXPECTED_BASE_SOURCE_TICKS: [u64; 3] = [1740, 1740, 1920];
const EXPECTED_OFFSETS: [i64; 5] = [-240, -180, -120, -60, 0];
const EXPECTED_ANALYTICAL_SURVIVOR_ROWS: [(usize, i64); 9] = [
    (0, -180),
    (0, -120),
    (1, -240),
    (1, -180),
    (1, -120),
    (1, -60),
    (1, 0),
    (2, -180),
    (2, -120),
];
const SOURCE_DURATION_LAUNCH_TICKS: u64 = 72;

#[derive(Clone, Debug)]
pub struct WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths {
    pub frozen_inputs: WaypointDirectSourceDurationCanaryInputPaths,
    pub source_duration_summary: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectSourceDurationHeldCadenceDiagnosticPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectSourceDurationHeldCadenceDiagnosticRun {
    pub artifact: WaypointDirectSourceDurationHeldCadenceDiagnosticArtifact,
    pub paths: WaypointDirectSourceDurationHeldCadenceDiagnosticPaths,
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectSourceDurationHeldCadenceDiagnosticValidation {
    pub source_duration_identity: String,
    pub expected_source_duration_sha256: String,
    pub input_gate: WaypointDirectCoupledThrustAuditInputGateEvidence,
    pub coupled_audit_identity: String,
    pub predeclared_variant_count: usize,
    pub analytical_survivor_count: usize,
    pub baseline_pair_count: usize,
    pub ablation_variant_count: usize,
    pub representative_candidate_identities: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectSourceDurationHeldCadenceDiagnosticArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub source_duration_identity: String,
    pub expected_source_duration_sha256: String,
    pub input_gate: WaypointDirectCoupledThrustAuditInputGateEvidence,
    pub coupled_audit_identity: String,
    pub protocol: HeldCadenceDiagnosticProtocolEvidence,
    pub family_proof: HeldCadenceDiagnosticFamilyProofEvidence,
    pub rows: Vec<HeldCadenceDiagnosticRowEvidence>,
    pub execution_status: String,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCadenceDiagnosticProtocolEvidence {
    pub frozen_input_rule: String,
    pub command_hold_rule: String,
    pub residual_rule: String,
    pub contact_rule: String,
    pub interaction_rule: String,
    pub non_claim: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCadenceDiagnosticFamilyProofEvidence {
    pub predeclared_row_count: usize,
    pub recorded_row_count: usize,
    pub analytical_screen_skip_count: usize,
    pub baseline_pair_count: usize,
    pub ablation_run_count: usize,
    pub representative_offset_ticks: i64,
    pub candidate_order: Vec<String>,
    pub fixed_offset_order_ticks: Vec<i64>,
    pub row_order_rule: String,
    pub all_rows_recorded: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCadenceDiagnosticRowEvidence {
    pub row_index: usize,
    pub candidate_identity: String,
    pub role: String,
    pub duration_offset_ticks: i64,
    pub source_bridge_tick_count: u64,
    pub analytical_survivor: bool,
    pub source_handoff_survivor: bool,
    pub status: String,
    pub analytical_screen: Option<AnalyticalScreenEvidence>,
    pub skip_reason: Option<String>,
    pub baseline_lanes: Vec<HeldCadenceBaselineLaneEvidence>,
    pub residuals: Vec<SignedStateResidualEvidence>,
    pub unavailable_phase_boundaries: Vec<UnavailablePhaseBoundaryEvidence>,
    pub residual_common_prefix_end_step: Option<u64>,
    pub ablations: Vec<HeldCadenceAblationEvidence>,
    pub interaction: Option<HeldCadenceInteractionEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCadenceBaselineLaneEvidence {
    pub cadence: String,
    pub status: String,
    pub physics_steps_advanced: u64,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub strict_source_handoff_within_tolerance: bool,
    pub termination: PlantStateEvidence,
    pub saturation: CommandSaturationEvidence,
    pub first_contact: Option<FirstContactEvidence>,
    pub first_contact_margins: Option<HeldCadencePredicateMarginsEvidence>,
    pub command_hold_ledger: HeldCadenceCommandHoldLedgerEvidence,
    pub replay_trace: ReplayTraceParityEvidence,
    pub replay_matches_frozen_canary: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignedStateResidualEvidence {
    pub physics_step: u64,
    pub phase: String,
    pub sample_kinds: Vec<String>,
    /// Held-command 60 Hz state minus direct-per-tick 120 Hz state.
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub fuel_kg: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnavailablePhaseBoundaryEvidence {
    pub boundary: String,
    pub physics_step: u64,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCadenceAblationEvidence {
    pub mode: String,
    pub status: String,
    pub physics_steps_advanced: u64,
    pub termination: PlantStateEvidence,
    pub saturation: CommandSaturationEvidence,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub strict_source_handoff_within_tolerance: bool,
    pub source_handoff_residual_vs_direct_120: Option<SignedStateResidualEvidence>,
    pub coast_handoff_residual_vs_direct_120: Option<SignedStateResidualEvidence>,
    pub unavailable_phase_boundaries: Vec<UnavailablePhaseBoundaryEvidence>,
    pub command_hold_ledger: HeldCadenceCommandHoldLedgerEvidence,
    pub first_contact: Option<FirstContactEvidence>,
    pub first_contact_margins: Option<HeldCadencePredicateMarginsEvidence>,
    pub replay_trace: ReplayTraceParityEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCadenceCommandHoldLedgerEvidence {
    /// First shared source-bridge even-tick endpoint; for a held 60 Hz lane it
    /// is the first post-step boundary after a two-physics-step command hold.
    pub first_source_control_boundary: Option<HeldCadenceCommandSampleEvidence>,
    pub phase_summaries: Vec<HeldCadenceCommandPhaseSummaryEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCadenceCommandComparisonEvidence {
    pub maximum_absolute_delta: f64,
    pub sample: HeldCadenceCommandSampleEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCadenceCommandSampleEvidence {
    pub physics_step: u64,
    pub phase: String,
    pub desired_command_throttle_frac: f64,
    pub held_command_throttle_frac: f64,
    pub desired_applied_throttle_frac: f64,
    pub plant_applied_throttle_frac: f64,
    pub desired_command_minus_held_command_frac: f64,
    pub desired_applied_minus_plant_applied_frac: f64,
    pub desired_target_attitude_rad: f64,
    pub held_target_attitude_rad: f64,
    pub desired_minus_held_target_attitude_rad: f64,
    pub throttle_update_due: bool,
    pub attitude_update_due: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCadenceCommandPhaseSummaryEvidence {
    pub phase: String,
    pub physics_step_count: usize,
    pub throttle_update_due_count: usize,
    pub throttle_held_tick_count: usize,
    pub attitude_update_due_count: usize,
    pub attitude_held_tick_count: usize,
    pub maximum_desired_command_vs_held_command: HeldCadenceCommandComparisonEvidence,
    pub maximum_desired_applied_vs_plant_applied: HeldCadenceCommandComparisonEvidence,
    pub maximum_desired_vs_held_target_attitude: HeldCadenceCommandComparisonEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCadencePredicateMarginsEvidence {
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
pub struct HeldCadenceInteractionEvidence {
    pub definition: String,
    pub common_live_control_boundary_count: usize,
    pub last_common_live_control_boundary_step: Option<u64>,
    pub samples: Vec<HeldCadenceInteractionSampleEvidence>,
    pub interpretation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldCadenceInteractionSampleEvidence {
    pub physics_step: u64,
    pub phase: String,
    /// Combined-hold residual minus the sum of the two single-hold residuals.
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub fuel_kg: f64,
}

pub(super) struct PreparedHeldCadenceInputs {
    pub(super) frozen: WaypointDirectSourceDurationCanaryArtifact,
    pub(super) validated: flat_candidate_closure::CoupledThrustAuditValidatedInputs,
    pub(super) input_gate: WaypointDirectCoupledThrustAuditInputGateEvidence,
    pub(super) coupled_audit_identity: String,
    pub(super) variants: Vec<(
        flat_candidate_closure::CoupledThrustAuditCandidateInput,
        Vec<PreparedDurationVariant>,
    )>,
}

#[derive(Clone, Debug)]
struct FrozenRowBinding {
    row_index: usize,
    candidate_index: usize,
    candidate_identity: String,
    role: String,
    duration_offset_ticks: i64,
    source_bridge_tick_count: u64,
    analytical_survivor: bool,
    source_handoff_survivor: bool,
    full_flight_cadences: Vec<String>,
    all_contact_replays_passed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RunWithSamples {
    pub(super) run: LaunchFeasibilityCadenceRunEvidence,
    pub(super) samples: Vec<SourceDurationStateSample>,
    pub(super) command_samples: Vec<SourceDurationCommandSample>,
}

/// Rebuild and verify every sealed input and the frozen 15-row summary without
/// constructing a simulation state.
pub fn validate_waypoint_direct_source_duration_held_cadence_diagnostic_inputs(
    repo_root: &Path,
    input_paths: &WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths,
) -> Result<WaypointDirectSourceDurationHeldCadenceDiagnosticValidation> {
    let prepared = prepare_held_cadence_inputs(repo_root, input_paths)?;
    Ok(validation_from_prepared(&prepared))
}

/// Run the nine frozen baseline pairs and the six predeclared single-component
/// hold ablations after all identities and row bindings have passed preflight.
pub fn run_waypoint_direct_source_duration_held_cadence_diagnostic(
    repo_root: &Path,
    input_paths: &WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths,
    output_dir: &Path,
) -> Result<WaypointDirectSourceDurationHeldCadenceDiagnosticRun> {
    let prepared = prepare_held_cadence_inputs(repo_root, input_paths)?;
    let context = RunContext::from_scenario(&prepared.validated.flat_case.scenario)
        .map_err(anyhow::Error::msg)?;
    let mut rows = Vec::with_capacity(15);
    let mut baseline_pair_count = 0;
    let mut ablation_run_count = 0;

    for (candidate_index, (candidate_input, variants)) in prepared.variants.iter().enumerate() {
        let candidate = &candidate_input.candidate;
        let frozen_basis = prepared
            .frozen
            .bases
            .get(candidate_index)
            .ok_or_else(|| anyhow!("frozen source-duration basis row disappeared"))?;
        for (duration_index, variant) in variants.iter().enumerate() {
            let frozen_variant = frozen_basis
                .duration_variants
                .get(duration_index)
                .ok_or_else(|| anyhow!("frozen source-duration variant row disappeared"))?;
            let role = EXPECTED_CANDIDATE_ROLES[candidate_index].to_owned();
            if !frozen_variant.analytical_survivor || !frozen_variant.source_handoff_survivor {
                rows.push(skipped_row(candidate, &role, variant, frozen_variant));
                continue;
            }

            let launch_target_attitude_rad = variant
                .launch_target_attitude_rad
                .ok_or_else(|| anyhow!("frozen survivor has no launch target"))?;
            let frozen_analysis = frozen_variant
                .launch_and_analytical_screen
                .as_ref()
                .ok_or_else(|| anyhow!("frozen survivor has no launch analysis"))?;
            let run_120 = run_lane(
                candidate_input,
                &prepared.validated,
                variant.source_bridge_tick_count,
                launch_target_attitude_rad,
                RolloutCadence::DirectPerTick,
                SourceDurationHoldMode::Together,
            )?;
            let replay_120 = replay_logged_cadence(&context, &run_120.run)?;
            let frozen_120 = frozen_flight(frozen_variant, "direct_per_tick_120_hz")?;
            let matches_120 =
                baseline_matches_frozen(&run_120.run, &replay_120, frozen_analysis, frozen_120);
            if !matches_120 || !replay_120.trace.passed {
                bail!(
                    "120 Hz baseline for row {} no longer matches the frozen canary and paired replay",
                    variant.row_index
                );
            }

            let run_60 = run_lane(
                candidate_input,
                &prepared.validated,
                variant.source_bridge_tick_count,
                launch_target_attitude_rad,
                RolloutCadence::ControllerCadence,
                SourceDurationHoldMode::Together,
            )?;
            let replay_60 = replay_logged_cadence(&context, &run_60.run)?;
            let frozen_60 = frozen_flight(frozen_variant, "held_controller_60_hz")?;
            let matches_60 =
                baseline_matches_frozen(&run_60.run, &replay_60, frozen_analysis, frozen_60);
            if !matches_60 || !replay_60.trace.passed {
                bail!(
                    "held 60 Hz baseline for row {} no longer matches the frozen canary and paired replay",
                    variant.row_index
                );
            }
            baseline_pair_count += 1;

            let source_handoff_step =
                SOURCE_DURATION_LAUNCH_TICKS + variant.source_bridge_tick_count;
            let coast_handoff_step = source_handoff_step + frozen_basis.frozen_coast_tick_count;
            let residuals =
                build_signed_residuals(&run_120, &run_60, source_handoff_step, coast_handoff_step);
            let (unavailable_phase_boundaries, residual_common_prefix_end_step) =
                residual_comparison_limits(
                    &run_120,
                    &run_60,
                    source_handoff_step,
                    coast_handoff_step,
                );
            let baseline_lanes = vec![
                baseline_lane_evidence(&run_120, &replay_120, true),
                baseline_lane_evidence(&run_60, &replay_60, true),
            ];

            let is_representative = variant.duration_offset_ticks == -180;
            let mut ablations = Vec::new();
            let mut interaction = None;
            if is_representative {
                let throttle_only = run_lane(
                    candidate_input,
                    &prepared.validated,
                    variant.source_bridge_tick_count,
                    launch_target_attitude_rad,
                    RolloutCadence::ControllerCadence,
                    SourceDurationHoldMode::ThrottleHeldAttitudePerTick,
                )?;
                let throttle_replay = replay_logged_mixed_cadence(
                    &context,
                    &throttle_only.run,
                    SourceDurationHoldMode::ThrottleHeldAttitudePerTick,
                )?;
                if !throttle_replay.trace.passed {
                    bail!(
                        "throttle-held/attitude-refreshed ablation for row {} failed paired replay parity",
                        variant.row_index
                    );
                }
                let attitude_only = run_lane(
                    candidate_input,
                    &prepared.validated,
                    variant.source_bridge_tick_count,
                    launch_target_attitude_rad,
                    RolloutCadence::ControllerCadence,
                    SourceDurationHoldMode::AttitudeHeldThrottlePerTick,
                )?;
                let attitude_replay = replay_logged_mixed_cadence(
                    &context,
                    &attitude_only.run,
                    SourceDurationHoldMode::AttitudeHeldThrottlePerTick,
                )?;
                if !attitude_replay.trace.passed {
                    bail!(
                        "attitude-held/throttle-refreshed ablation for row {} failed paired replay parity",
                        variant.row_index
                    );
                }
                ablation_run_count += 2;
                interaction = Some(build_interaction(
                    &run_120,
                    &run_60,
                    &throttle_only,
                    &attitude_only,
                ));
                ablations.push(ablation_evidence(
                    &run_120,
                    &throttle_only,
                    &throttle_replay,
                    SourceDurationHoldMode::ThrottleHeldAttitudePerTick,
                    source_handoff_step,
                    coast_handoff_step,
                ));
                ablations.push(ablation_evidence(
                    &run_120,
                    &attitude_only,
                    &attitude_replay,
                    SourceDurationHoldMode::AttitudeHeldThrottlePerTick,
                    source_handoff_step,
                    coast_handoff_step,
                ));
            }

            rows.push(HeldCadenceDiagnosticRowEvidence {
                row_index: variant.row_index,
                candidate_identity: candidate.identity.clone(),
                role,
                duration_offset_ticks: variant.duration_offset_ticks,
                source_bridge_tick_count: variant.source_bridge_tick_count,
                analytical_survivor: true,
                source_handoff_survivor: true,
                status: "baseline_and_contact_diagnostic_complete".to_owned(),
                analytical_screen: frozen_variant
                    .launch_and_analytical_screen
                    .as_ref()
                    .and_then(|analysis| analysis.analytical_screen.clone()),
                skip_reason: None,
                baseline_lanes,
                residuals,
                unavailable_phase_boundaries,
                residual_common_prefix_end_step,
                ablations,
                interaction,
            });
        }
    }

    if rows.len() != 15 || baseline_pair_count != 9 || ablation_run_count != 6 {
        bail!(
            "held-cadence diagnostic coverage changed: rows={}, baseline_pairs={baseline_pair_count}, ablations={ablation_run_count}",
            rows.len()
        );
    }
    let output_dir = resolve_output_dir(repo_root, output_dir);
    let summary_path = output_dir.join("summary.json");
    if summary_path.exists() {
        bail!(
            "held-cadence diagnostic refuses to overwrite existing summary {}",
            summary_path.display()
        );
    }

    let mut artifact = WaypointDirectSourceDurationHeldCadenceDiagnosticArtifact {
        schema_id: WAYPOINT_DIRECT_SOURCE_DURATION_HELD_CADENCE_DIAGNOSTIC_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_SOURCE_DURATION_HELD_CADENCE_DIAGNOSTIC_SCHEMA_VERSION,
        characterization_id:
            WAYPOINT_DIRECT_SOURCE_DURATION_HELD_CADENCE_DIAGNOSTIC_ID.to_owned(),
        source_duration_identity: prepared.frozen.identity.clone(),
        expected_source_duration_sha256: EXPECTED_SOURCE_DURATION_SHA256.to_owned(),
        input_gate: prepared.input_gate.clone(),
        coupled_audit_identity: prepared.coupled_audit_identity.clone(),
        protocol: protocol_evidence(),
        family_proof: HeldCadenceDiagnosticFamilyProofEvidence {
            predeclared_row_count: 15,
            recorded_row_count: rows.len(),
            analytical_screen_skip_count: rows.iter().filter(|row| !row.analytical_survivor).count(),
            baseline_pair_count,
            ablation_run_count,
            representative_offset_ticks: -180,
            candidate_order: EXPECTED_CANDIDATE_IDENTITIES
                .iter()
                .map(|identity| (*identity).to_owned())
                .collect(),
            fixed_offset_order_ticks: EXPECTED_OFFSETS.to_vec(),
            row_order_rule: "candidate order native, research-shortest, third; within each candidate offsets -240, -180, -120, -60, 0; retain all fifteen rows, including six analytical-screen skips".to_owned(),
            all_rows_recorded: rows.len() == 15,
        },
        rows,
        execution_status: "frozen_baselines_replayed_and_six_bounded_ablations_completed".to_owned(),
        scope_non_claims: vec![
            "This is a known-outcome evaluator diagnostic, not held-out validation or certification.".to_owned(),
            "The source-duration family and its original V2 classifications remain frozen; no new candidate search or duration tuning is performed.".to_owned(),
            "Contact outcomes are the unchanged authoritative core outcomes; predicate margins explain the observed first contact and do not alter it.".to_owned(),
            "Single-hold differences and the interaction term are descriptive; neither the legacy unclassified divergence label nor these ablations establish a single cause.".to_owned(),
            "No planner, controller, simulator/contact rule, V2 policy, F6 path, or default selection is changed.".to_owned(),
        ],
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;
    write_artifact(&output_dir, &summary_path, &artifact)?;
    Ok(WaypointDirectSourceDurationHeldCadenceDiagnosticRun {
        artifact,
        paths: WaypointDirectSourceDurationHeldCadenceDiagnosticPaths {
            output_dir,
            summary_path,
        },
    })
}

pub(super) fn prepare_held_cadence_inputs(
    repo_root: &Path,
    input_paths: &WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths,
) -> Result<PreparedHeldCadenceInputs> {
    let validated = flat_candidate_closure::load_coupled_thrust_audit_inputs(
        repo_root,
        &input_paths.frozen_inputs.frozen_inputs,
    )?;
    let input_gate = validated
        .coupled_input_gate
        .as_ref()
        .ok_or_else(|| anyhow!("coupled-thrust input gate was not rebuilt"))?
        .clone();
    let audit = read_coupled_audit(
        &input_paths.frozen_inputs.coupled_audit_summary,
        &input_gate,
    )?;
    let variants = prepare_duration_variants(&validated)?;
    let frozen: WaypointDirectSourceDurationCanaryArtifact =
        super::super::read_summary(&input_paths.source_duration_summary)?;
    validate_frozen_source_summary(&frozen, &input_gate, &audit.identity, &variants)?;
    Ok(PreparedHeldCadenceInputs {
        frozen,
        validated,
        input_gate,
        coupled_audit_identity: audit.identity,
        variants,
    })
}

pub(super) fn validation_from_prepared(
    prepared: &PreparedHeldCadenceInputs,
) -> WaypointDirectSourceDurationHeldCadenceDiagnosticValidation {
    WaypointDirectSourceDurationHeldCadenceDiagnosticValidation {
        source_duration_identity: prepared.frozen.identity.clone(),
        expected_source_duration_sha256: EXPECTED_SOURCE_DURATION_SHA256.to_owned(),
        input_gate: prepared.input_gate.clone(),
        coupled_audit_identity: prepared.coupled_audit_identity.clone(),
        predeclared_variant_count: 15,
        analytical_survivor_count: 9,
        baseline_pair_count: 9,
        ablation_variant_count: 6,
        representative_candidate_identities: EXPECTED_CANDIDATE_IDENTITIES
            .iter()
            .map(|identity| (*identity).to_owned())
            .collect(),
    }
}

fn validate_frozen_source_summary(
    frozen: &WaypointDirectSourceDurationCanaryArtifact,
    input_gate: &WaypointDirectCoupledThrustAuditInputGateEvidence,
    coupled_audit_identity: &str,
    variants: &[(
        flat_candidate_closure::CoupledThrustAuditCandidateInput,
        Vec<PreparedDurationVariant>,
    )],
) -> Result<()> {
    let computed_identity = source_artifact_identity(frozen)?;
    validate_source_summary_identity(&frozen.identity, &computed_identity)?;
    if frozen.schema_id != super::WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_SCHEMA_ID
        || frozen.schema_version != super::WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_SCHEMA_VERSION
        || frozen.characterization_id != super::WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_ID
        || frozen.identity != EXPECTED_SOURCE_DURATION_IDENTITY
        || &frozen.input_gate != input_gate
        || frozen.coupled_audit_identity != coupled_audit_identity
    {
        bail!("frozen source-duration summary identity or upstream gate binding changed");
    }
    let bindings = frozen_row_bindings(frozen)?;
    validate_frozen_family_contract(&frozen.family_proof, &bindings)?;
    if variants.len() != EXPECTED_CANDIDATE_IDENTITIES.len() {
        bail!("rebuilt source-duration candidates no longer match the frozen family");
    }
    let mut rebuilt_row_index = 0;
    for (candidate_index, (candidate_input, candidate_variants)) in variants.iter().enumerate() {
        if candidate_input.candidate.identity != EXPECTED_CANDIDATE_IDENTITIES[candidate_index]
            || candidate_variants.len() != EXPECTED_OFFSETS.len()
        {
            bail!("rebuilt source-duration row order changed");
        }
        for (duration_index, rebuilt) in candidate_variants.iter().enumerate() {
            let frozen = &bindings[rebuilt_row_index];
            let expected_ticks = i64::try_from(EXPECTED_BASE_SOURCE_TICKS[candidate_index])
                .ok()
                .and_then(|ticks| ticks.checked_add(EXPECTED_OFFSETS[duration_index]))
                .and_then(|ticks| u64::try_from(ticks).ok())
                .ok_or_else(|| anyhow!("invalid frozen source-duration tick count"))?;
            if frozen.row_index != rebuilt_row_index
                || frozen.duration_offset_ticks != EXPECTED_OFFSETS[duration_index]
                || frozen.source_bridge_tick_count != expected_ticks
                || rebuilt.row_index != rebuilt_row_index
                || rebuilt.duration_offset_ticks != frozen.duration_offset_ticks
                || rebuilt.source_bridge_tick_count != frozen.source_bridge_tick_count
            {
                bail!("rebuilt source-duration row binding changed at row {rebuilt_row_index}");
            }
            rebuilt_row_index += 1;
        }
    }
    Ok(())
}

fn validate_source_summary_identity(stored: &str, computed: &str) -> Result<()> {
    if stored != computed || stored != EXPECTED_SOURCE_DURATION_IDENTITY {
        bail!("frozen source-duration semantic identity changed");
    }
    Ok(())
}

fn frozen_row_bindings(
    artifact: &WaypointDirectSourceDurationCanaryArtifact,
) -> Result<Vec<FrozenRowBinding>> {
    if artifact.bases.len() != EXPECTED_CANDIDATE_IDENTITIES.len() {
        bail!("frozen source-duration summary does not contain three bases");
    }
    let mut rows = Vec::with_capacity(15);
    for (candidate_index, basis) in artifact.bases.iter().enumerate() {
        if basis.candidate_identity != EXPECTED_CANDIDATE_IDENTITIES[candidate_index]
            || basis.role != EXPECTED_CANDIDATE_ROLES[candidate_index]
            || basis.duration_variants.len() != EXPECTED_OFFSETS.len()
        {
            bail!("frozen source-duration basis order or row count changed");
        }
        for variant in &basis.duration_variants {
            let full_flight_cadences = variant
                .full_flights
                .iter()
                .map(|flight| flight.cadence.clone())
                .collect::<Vec<_>>();
            let all_contact_replays_passed = variant.full_flights.iter().all(|flight| {
                flight.first_contact.is_some()
                    && flight.contact_replay_error.is_none()
                    && flight
                        .contact_replay_trace
                        .as_ref()
                        .is_some_and(|trace| trace.passed)
            });
            rows.push(FrozenRowBinding {
                row_index: variant.row_index,
                candidate_index,
                candidate_identity: basis.candidate_identity.clone(),
                role: basis.role.clone(),
                duration_offset_ticks: variant.duration_offset_ticks,
                source_bridge_tick_count: variant.source_bridge_tick_count,
                analytical_survivor: variant.analytical_survivor,
                source_handoff_survivor: variant.source_handoff_survivor,
                full_flight_cadences,
                all_contact_replays_passed,
            });
        }
    }
    Ok(rows)
}

fn validate_frozen_family_contract(
    family: &SourceDurationFamilyProofEvidence,
    rows: &[FrozenRowBinding],
) -> Result<()> {
    if family.maximum_predeclared_variant_count != 15
        || family.predeclared_variant_count != 15
        || family.recorded_variant_count != 15
        || family.recorded_failure_row_count != 15
        || family.omitted_failure_row_count != 0
        || family.duration_offsets_ticks != EXPECTED_OFFSETS
        || family.duration_grid_seconds != 0.5
        || family.candidate_order
            != EXPECTED_CANDIDATE_IDENTITIES
                .iter()
                .map(|identity| (*identity).to_owned())
                .collect::<Vec<_>>()
        || family.row_order_rule
            != "candidate order native, research-shortest, third; within each candidate sort fixed offsets -240, -180, -120, -60, 0 physics ticks; retain every predeclared row including bridge, screen, execution, and contact-replay failures"
        || !family.all_rows_recorded
        || rows.len() != 15
    {
        bail!("frozen source-duration family proof changed");
    }
    let mut survivor_count = 0;
    for (row_index, row) in rows.iter().enumerate() {
        let expected_survivor = EXPECTED_ANALYTICAL_SURVIVOR_ROWS
            .contains(&(row.candidate_index, row.duration_offset_ticks));
        if row.row_index != row_index
            || row.candidate_identity != EXPECTED_CANDIDATE_IDENTITIES[row.candidate_index]
            || row.role != EXPECTED_CANDIDATE_ROLES[row.candidate_index]
            || row.duration_offset_ticks != EXPECTED_OFFSETS[row_index % EXPECTED_OFFSETS.len()]
            || row.analytical_survivor != expected_survivor
            || row.source_handoff_survivor != expected_survivor
        {
            bail!("frozen source-duration row coverage or survivor set changed");
        }
        if expected_survivor {
            survivor_count += 1;
            if row.full_flight_cadences != ["direct_per_tick_120_hz", "held_controller_60_hz"]
                || !row.all_contact_replays_passed
            {
                bail!("frozen source-duration survivor is missing a paired contact replay");
            }
        } else if !row.full_flight_cadences.is_empty() {
            bail!("frozen source-duration analytical skip unexpectedly contains a full flight");
        }
    }
    if survivor_count != 9 {
        bail!("frozen source-duration summary no longer has exactly nine survivors");
    }
    Ok(())
}

fn skipped_row(
    candidate: &pd_plan::conservative_ballistic_bridge::DirectBridgeCandidateV2,
    role: &str,
    variant: &PreparedDurationVariant,
    frozen: &SourceDurationVariantEvidence,
) -> HeldCadenceDiagnosticRowEvidence {
    let skip_reason = if let Some(reason) = &frozen.analysis_error {
        format!("analytical_screen_failed:{reason}")
    } else {
        "analytical_screen_failed_before_flight_replay".to_owned()
    };
    HeldCadenceDiagnosticRowEvidence {
        row_index: variant.row_index,
        candidate_identity: candidate.identity.clone(),
        role: role.to_owned(),
        duration_offset_ticks: variant.duration_offset_ticks,
        source_bridge_tick_count: variant.source_bridge_tick_count,
        analytical_survivor: false,
        source_handoff_survivor: false,
        status: "analytical_screen_skip".to_owned(),
        analytical_screen: frozen
            .launch_and_analytical_screen
            .as_ref()
            .and_then(|analysis| analysis.analytical_screen.clone()),
        skip_reason: Some(skip_reason),
        baseline_lanes: Vec::new(),
        residuals: Vec::new(),
        unavailable_phase_boundaries: Vec::new(),
        residual_common_prefix_end_step: None,
        ablations: Vec::new(),
        interaction: None,
    }
}

pub(super) fn run_lane(
    candidate_input: &flat_candidate_closure::CoupledThrustAuditCandidateInput,
    prepared: &flat_candidate_closure::CoupledThrustAuditValidatedInputs,
    source_bridge_steps: u64,
    launch_tilt_attitude_rad: f64,
    cadence: RolloutCadence,
    hold_mode: SourceDurationHoldMode,
) -> Result<RunWithSamples> {
    let diagnostic = run_source_duration_variant_diagnostic(
        SourceDurationRunRequest {
            case: &prepared.flat_case,
            selected: &candidate_input.selected_profile,
            basis: &candidate_input.candidate,
            policy: &prepared.policy,
            vehicle: &prepared.vehicle,
            source_bridge_steps,
            launch_tilt_attitude_rad,
            cadence,
            stage: SourceDurationReplayStage::FullFlight,
        },
        hold_mode,
    )?;
    let samples = complete_state_samples(&diagnostic);
    Ok(RunWithSamples {
        run: diagnostic.run,
        samples,
        command_samples: diagnostic.command_samples,
    })
}

fn complete_state_samples(
    diagnostic: &SourceDurationDiagnosticCadenceRun,
) -> Vec<SourceDurationStateSample> {
    let launch = diagnostic
        .run
        .launch
        .samples
        .iter()
        .map(|sample| SourceDurationStateSample {
            physics_step: sample.physics_step,
            phase: sample.phase.clone(),
            position_m: sample.position_m,
            velocity_mps: sample.velocity_mps,
            attitude_rad: sample.attitude_after_step_rad,
            angular_rate_radps: sample.angular_rate_radps,
            fuel_kg: sample.fuel_kg,
        });
    launch
        .chain(diagnostic.state_samples.iter().cloned())
        .collect()
}

pub(super) fn frozen_flight<'a>(
    variant: &'a SourceDurationVariantEvidence,
    cadence: &str,
) -> Result<&'a FullFlightEvidence> {
    variant
        .full_flights
        .iter()
        .find(|flight| flight.cadence == cadence)
        .ok_or_else(|| anyhow!("frozen survivor has no {cadence} full-flight lane"))
}

pub(super) fn baseline_matches_frozen(
    run: &LaunchFeasibilityCadenceRunEvidence,
    replay: &ReplayTraceResult,
    analysis: &LaunchAndAnalyticalScreenEvidence,
    frozen: &FullFlightEvidence,
) -> bool {
    run.cadence == frozen.cadence
        && run.launch == analysis.launch
        && run.reseeded_bridge == analysis.reseeded_bridge
        && run.rollout == frozen.rollout
        && frozen.launch_matches_analysis
        && frozen.status == run.rollout.status
        && frozen.first_contact == replay.first_contact
        && frozen.contact_replay_trace.as_ref() == Some(&replay.trace)
        && frozen.contact_replay_error.is_none()
}

fn baseline_lane_evidence(
    run_with_samples: &RunWithSamples,
    replay: &ReplayTraceResult,
    replay_matches_frozen_canary: bool,
) -> HeldCadenceBaselineLaneEvidence {
    let run = &run_with_samples.run;
    let source_position_error = run.rollout.source_handoff_position_error_m;
    let source_velocity_error = run.rollout.source_handoff_velocity_error_mps;
    HeldCadenceBaselineLaneEvidence {
        cadence: run.cadence.clone(),
        status: run.rollout.status.clone(),
        physics_steps_advanced: run.rollout.physics_steps_advanced,
        source_handoff_position_error_m: source_position_error,
        source_handoff_velocity_error_mps: source_velocity_error,
        strict_source_handoff_within_tolerance: run.rollout.source_handoff_contact_free
            && source_position_error.is_some_and(|error| error <= 1.0e-6)
            && source_velocity_error.is_some_and(|error| error <= 1.0e-6),
        termination: run.rollout.termination.clone(),
        saturation: run.rollout.saturation.clone(),
        first_contact: replay.first_contact.clone(),
        first_contact_margins: replay
            .first_contact_margins
            .as_ref()
            .map(exposed_contact_margins),
        command_hold_ledger: command_hold_ledger(&run_with_samples.command_samples),
        replay_trace: replay.trace.clone(),
        replay_matches_frozen_canary,
    }
}

fn ablation_evidence(
    direct: &RunWithSamples,
    run_with_samples: &RunWithSamples,
    replay: &ReplayTraceResult,
    mode: SourceDurationHoldMode,
    source_handoff_step: u64,
    coast_handoff_step: u64,
) -> HeldCadenceAblationEvidence {
    let run = &run_with_samples.run;
    let residuals = build_signed_residuals(
        direct,
        run_with_samples,
        source_handoff_step,
        coast_handoff_step,
    );
    let source_handoff_residual_vs_direct_120 = residuals
        .iter()
        .find(|residual| {
            residual
                .sample_kinds
                .iter()
                .any(|kind| kind == "source_handoff")
        })
        .cloned();
    let coast_handoff_residual_vs_direct_120 = residuals
        .iter()
        .find(|residual| {
            residual
                .sample_kinds
                .iter()
                .any(|kind| kind == "coast_handoff")
        })
        .cloned();
    let (unavailable_phase_boundaries, _) = residual_comparison_limits(
        direct,
        run_with_samples,
        source_handoff_step,
        coast_handoff_step,
    );
    let source_position_error = run.rollout.source_handoff_position_error_m;
    let source_velocity_error = run.rollout.source_handoff_velocity_error_mps;
    HeldCadenceAblationEvidence {
        mode: hold_mode_label(mode).to_owned(),
        status: if replay.trace.passed {
            run.rollout.status.clone()
        } else {
            "paired_replay_parity_failed".to_owned()
        },
        physics_steps_advanced: run.rollout.physics_steps_advanced,
        termination: run.rollout.termination.clone(),
        saturation: run.rollout.saturation.clone(),
        source_handoff_position_error_m: source_position_error,
        source_handoff_velocity_error_mps: source_velocity_error,
        strict_source_handoff_within_tolerance: run.rollout.source_handoff_contact_free
            && source_position_error.is_some_and(|error| error <= 1.0e-6)
            && source_velocity_error.is_some_and(|error| error <= 1.0e-6),
        source_handoff_residual_vs_direct_120,
        coast_handoff_residual_vs_direct_120,
        unavailable_phase_boundaries,
        command_hold_ledger: command_hold_ledger(&run_with_samples.command_samples),
        first_contact: replay.first_contact.clone(),
        first_contact_margins: replay
            .first_contact_margins
            .as_ref()
            .map(exposed_contact_margins),
        replay_trace: replay.trace.clone(),
    }
}

fn exposed_contact_margins(
    margins: &FirstContactPredicateMarginsEvidence,
) -> HeldCadencePredicateMarginsEvidence {
    HeldCadencePredicateMarginsEvidence {
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

fn command_hold_ledger(
    samples: &[SourceDurationCommandSample],
) -> HeldCadenceCommandHoldLedgerEvidence {
    let first_source_control_boundary = samples
        .iter()
        .find(|sample| {
            sample.phase == "source_bridge"
                && sample.physics_step > 0
                && sample.physics_step % 2 == 0
        })
        .map(command_comparison_sample);
    let mut phases = BTreeMap::<String, Vec<&SourceDurationCommandSample>>::new();
    for sample in samples {
        phases.entry(sample.phase.clone()).or_default().push(sample);
    }
    let phase_summaries = phases
        .into_iter()
        .map(|(phase, phase_samples)| {
            let comparisons = phase_samples
                .iter()
                .map(|sample| command_comparison_sample(sample))
                .collect::<Vec<_>>();
            HeldCadenceCommandPhaseSummaryEvidence {
                phase,
                physics_step_count: phase_samples.len(),
                throttle_update_due_count: phase_samples
                    .iter()
                    .filter(|sample| sample.throttle_update_due)
                    .count(),
                throttle_held_tick_count: phase_samples.len()
                    - phase_samples
                        .iter()
                        .filter(|sample| sample.throttle_update_due)
                        .count(),
                attitude_update_due_count: phase_samples
                    .iter()
                    .filter(|sample| sample.attitude_update_due)
                    .count(),
                attitude_held_tick_count: phase_samples.len()
                    - phase_samples
                        .iter()
                        .filter(|sample| sample.attitude_update_due)
                        .count(),
                maximum_desired_command_vs_held_command: maximum_command_delta(
                    &comparisons,
                    |sample| sample.desired_command_minus_held_command_frac,
                ),
                maximum_desired_applied_vs_plant_applied: maximum_command_delta(
                    &comparisons,
                    |sample| sample.desired_applied_minus_plant_applied_frac,
                ),
                maximum_desired_vs_held_target_attitude: maximum_command_delta(
                    &comparisons,
                    |sample| sample.desired_minus_held_target_attitude_rad,
                ),
            }
        })
        .collect();
    HeldCadenceCommandHoldLedgerEvidence {
        first_source_control_boundary,
        phase_summaries,
    }
}

fn command_comparison_sample(
    sample: &SourceDurationCommandSample,
) -> HeldCadenceCommandSampleEvidence {
    HeldCadenceCommandSampleEvidence {
        physics_step: sample.physics_step,
        phase: sample.phase.clone(),
        desired_command_throttle_frac: sample.desired_command_throttle_frac,
        held_command_throttle_frac: sample.held_command_throttle_frac,
        desired_applied_throttle_frac: sample.desired_applied_throttle_frac,
        plant_applied_throttle_frac: sample.plant_applied_throttle_frac,
        desired_command_minus_held_command_frac: sample.desired_command_throttle_frac
            - sample.held_command_throttle_frac,
        desired_applied_minus_plant_applied_frac: sample.desired_applied_throttle_frac
            - sample.plant_applied_throttle_frac,
        desired_target_attitude_rad: sample.desired_target_attitude_rad,
        held_target_attitude_rad: sample.held_target_attitude_rad,
        desired_minus_held_target_attitude_rad: super::super::shortest_angle_delta(
            sample.held_target_attitude_rad,
            sample.desired_target_attitude_rad,
        ),
        throttle_update_due: sample.throttle_update_due,
        attitude_update_due: sample.attitude_update_due,
    }
}

fn maximum_command_delta(
    samples: &[HeldCadenceCommandSampleEvidence],
    select_delta: impl Fn(&HeldCadenceCommandSampleEvidence) -> f64,
) -> HeldCadenceCommandComparisonEvidence {
    let sample = samples
        .iter()
        .max_by(|left, right| {
            select_delta(left)
                .abs()
                .total_cmp(&select_delta(right).abs())
        })
        .expect("phase command sample group is nonempty")
        .clone();
    HeldCadenceCommandComparisonEvidence {
        maximum_absolute_delta: select_delta(&sample).abs(),
        sample,
    }
}

fn build_signed_residuals(
    direct: &RunWithSamples,
    held: &RunWithSamples,
    source_handoff_step: u64,
    coast_handoff_step: u64,
) -> Vec<SignedStateResidualEvidence> {
    let direct_samples = sample_map(direct);
    let held_samples = sample_map(held);
    let mut sample_kinds = BTreeMap::<u64, BTreeSet<String>>::new();
    for physics_step in direct_samples
        .keys()
        .filter(|physics_step| held_samples.contains_key(physics_step))
        .filter(|physics_step| **physics_step > 0 && **physics_step % 2 == 0)
    {
        sample_kinds
            .entry(*physics_step)
            .or_default()
            .insert("post_step_control_boundary".to_owned());
    }
    for (name, physics_step) in [
        ("source_handoff", source_handoff_step),
        ("coast_handoff", coast_handoff_step),
    ] {
        if direct_samples.contains_key(&physics_step) && held_samples.contains_key(&physics_step) {
            sample_kinds
                .entry(physics_step)
                .or_default()
                .insert(name.to_owned());
        }
    }
    sample_kinds
        .into_iter()
        .filter_map(|(physics_step, kinds)| {
            let direct = direct_samples.get(&physics_step)?;
            let held = held_samples.get(&physics_step)?;
            Some(SignedStateResidualEvidence {
                physics_step,
                phase: direct.phase.clone(),
                sample_kinds: kinds.into_iter().collect(),
                position_m: held.position_m - direct.position_m,
                velocity_mps: held.velocity_mps - direct.velocity_mps,
                attitude_rad: super::super::shortest_angle_delta(
                    direct.attitude_rad,
                    held.attitude_rad,
                ),
                fuel_kg: held.fuel_kg - direct.fuel_kg,
            })
        })
        .collect()
}

fn residual_comparison_limits(
    direct: &RunWithSamples,
    held: &RunWithSamples,
    source_handoff_step: u64,
    coast_handoff_step: u64,
) -> (Vec<UnavailablePhaseBoundaryEvidence>, Option<u64>) {
    let direct_samples = sample_map(direct);
    let held_samples = sample_map(held);
    let common_end = direct_samples
        .keys()
        .filter(|step| held_samples.contains_key(step))
        .copied()
        .max();
    let mut unavailable = Vec::new();
    for (boundary, step) in [
        ("source_handoff", source_handoff_step),
        ("coast_handoff", coast_handoff_step),
    ] {
        if !(direct_samples.contains_key(&step) && held_samples.contains_key(&step)) {
            unavailable.push(UnavailablePhaseBoundaryEvidence {
                boundary: boundary.to_owned(),
                physics_step: step,
                reason: "at least one cadence had already terminated; no state was extrapolated"
                    .to_owned(),
            });
        }
    }
    (unavailable, common_end)
}

fn sample_map(run: &RunWithSamples) -> BTreeMap<u64, SourceDurationStateSample> {
    run.samples
        .iter()
        .map(|sample| (sample.physics_step, sample.clone()))
        .collect()
}

fn build_interaction(
    direct: &RunWithSamples,
    combined: &RunWithSamples,
    throttle_only: &RunWithSamples,
    attitude_only: &RunWithSamples,
) -> HeldCadenceInteractionEvidence {
    let direct_samples = sample_map(direct);
    let combined_samples = sample_map(combined);
    let throttle_samples = sample_map(throttle_only);
    let attitude_samples = sample_map(attitude_only);
    let mut samples = Vec::new();
    for (step, direct_state) in &direct_samples {
        if *step == 0 || *step % 2 != 0 {
            continue;
        }
        let (Some(combined_state), Some(throttle_state), Some(attitude_state)) = (
            combined_samples.get(step),
            throttle_samples.get(step),
            attitude_samples.get(step),
        ) else {
            continue;
        };
        let position_residual =
            |state: &SourceDurationStateSample| state.position_m - direct_state.position_m;
        let velocity_residual =
            |state: &SourceDurationStateSample| state.velocity_mps - direct_state.velocity_mps;
        let attitude_residual = |state: &SourceDurationStateSample| {
            super::super::shortest_angle_delta(direct_state.attitude_rad, state.attitude_rad)
        };
        let fuel_residual =
            |state: &SourceDurationStateSample| state.fuel_kg - direct_state.fuel_kg;
        let interaction_position = position_residual(combined_state)
            - position_residual(throttle_state)
            - position_residual(attitude_state);
        let interaction_velocity = velocity_residual(combined_state)
            - velocity_residual(throttle_state)
            - velocity_residual(attitude_state);
        let interaction_attitude = super::super::shortest_angle_delta(
            0.0,
            attitude_residual(combined_state)
                - attitude_residual(throttle_state)
                - attitude_residual(attitude_state),
        );
        let interaction_fuel = fuel_residual(combined_state)
            - fuel_residual(throttle_state)
            - fuel_residual(attitude_state);
        samples.push(HeldCadenceInteractionSampleEvidence {
            physics_step: *step,
            phase: direct_state.phase.clone(),
            position_m: interaction_position,
            velocity_mps: interaction_velocity,
            attitude_rad: interaction_attitude,
            fuel_kg: interaction_fuel,
        });
    }
    HeldCadenceInteractionEvidence {
        definition: "(both-held - direct) - (throttle-held-only - direct) - (attitude-held-only - direct); signed state residuals at common live control boundaries".to_owned(),
        common_live_control_boundary_count: samples.len(),
        last_common_live_control_boundary_step: samples.last().map(|sample| sample.physics_step),
        samples,
        interpretation: "Descriptive nonadditivity among these known-outcome lanes; nonlinear command/plant coupling means this is not a single-cause attribution.".to_owned(),
    }
}

fn ablation_mode_label(mode: SourceDurationHoldMode) -> &'static str {
    match mode {
        SourceDurationHoldMode::Together => "both_held_control",
        SourceDurationHoldMode::ThrottleHeldAttitudePerTick => {
            "throttle_held_attitude_refreshed_per_tick"
        }
        SourceDurationHoldMode::AttitudeHeldThrottlePerTick => {
            "attitude_held_throttle_refreshed_per_tick"
        }
    }
}

fn hold_mode_label(mode: SourceDurationHoldMode) -> &'static str {
    ablation_mode_label(mode)
}

fn protocol_evidence() -> HeldCadenceDiagnosticProtocolEvidence {
    HeldCadenceDiagnosticProtocolEvidence {
        frozen_input_rule: "Rebuild the six sealed source inputs and coupled-thrust audit, then verify the frozen source-duration schema, semantic identity fnv1a64:dfe0f0feaa15fc24, input gate, fixed 3 x 5 family, nine survivors, and three -180 representative rows before constructing any SimulationState. The pinned file SHA-256 is independently checked during validation.".to_owned(),
        command_hold_rule: "Run the frozen direct-per-tick 120 Hz and combined held 60 Hz baselines for all nine survivors. For only the native, research-shortest, and third -180 rows, additionally run throttle held for two physics ticks with attitude refreshed every tick, and attitude held for two physics ticks with throttle refreshed every tick. Recompute throttle demand from each lane's current pre-burn mass and fuel.".to_owned(),
        residual_rule: "Record signed held-60 minus direct-120 state residuals at every shared even physics-step control boundary and at shared source/coast handoff boundaries. Samples stop before the first terminal step in either lane; no missing state is interpolated or extrapolated.".to_owned(),
        contact_rule: "Use the paired ordinary/neutral core replay for first-contact step, classification, and preterminal state. Report each cadence's absolute preterminal state, raw foot and hull clearances, and signed predicate margins where positive passes. Do not form an unlabeled same-step contact delta across different contact or terminal steps.".to_owned(),
        interaction_rule: "For each -180 representative, report the signed state interaction (both-held minus direct) minus the two single-hold residuals at common live control boundaries. This is descriptive nonadditivity, not causal attribution.".to_owned(),
        non_claim: "These are known-outcome evaluator diagnostics, not held-out validation, certification, a cadence-aware source solver, or an acceptance-threshold change.".to_owned(),
    }
}

fn artifact_identity(
    artifact: &WaypointDirectSourceDurationHeldCadenceDiagnosticArtifact,
) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

fn write_artifact(
    output_dir: &Path,
    summary_path: &Path,
    artifact: &WaypointDirectSourceDurationHeldCadenceDiagnosticArtifact,
) -> Result<()> {
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create held-cadence diagnostic output directory {}",
            output_dir.display()
        )
    })?;
    let summary_bytes = serde_json::to_vec_pretty(artifact)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(summary_path)
        .with_context(|| {
            format!(
                "held-cadence diagnostic refuses to overwrite summary {}",
                summary_path.display()
            )
        })?;
    file.write_all(&summary_bytes).with_context(|| {
        format!(
            "failed to write held-cadence diagnostic summary {}",
            summary_path.display()
        )
    })?;
    drop(file);
    let reloaded: WaypointDirectSourceDurationHeldCadenceDiagnosticArtifact =
        serde_json::from_slice(&summary_bytes).context("failed to reload held-cadence summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes
        || artifact_identity(&reloaded)? != reloaded.identity
    {
        bail!("held-cadence summary failed its byte-stable identity round trip");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::Command;

    fn family_proof() -> SourceDurationFamilyProofEvidence {
        SourceDurationFamilyProofEvidence {
            maximum_predeclared_variant_count: 15,
            predeclared_variant_count: 15,
            recorded_variant_count: 15,
            recorded_failure_row_count: 15,
            omitted_failure_row_count: 0,
            duration_offsets_ticks: EXPECTED_OFFSETS.to_vec(),
            duration_grid_seconds: 0.5,
            candidate_order: EXPECTED_CANDIDATE_IDENTITIES
                .iter()
                .map(|identity| (*identity).to_owned())
                .collect(),
            row_order_rule: "candidate order native, research-shortest, third; within each candidate sort fixed offsets -240, -180, -120, -60, 0 physics ticks; retain every predeclared row including bridge, screen, execution, and contact-replay failures".to_owned(),
            all_rows_recorded: true,
        }
    }

    fn row_bindings() -> Vec<FrozenRowBinding> {
        let mut rows = Vec::new();
        for candidate_index in 0..3 {
            for (offset_index, offset) in EXPECTED_OFFSETS.iter().enumerate() {
                let row_index = candidate_index * EXPECTED_OFFSETS.len() + offset_index;
                let survivor =
                    EXPECTED_ANALYTICAL_SURVIVOR_ROWS.contains(&(candidate_index, *offset));
                rows.push(FrozenRowBinding {
                    row_index,
                    candidate_index,
                    candidate_identity: EXPECTED_CANDIDATE_IDENTITIES[candidate_index].to_owned(),
                    role: EXPECTED_CANDIDATE_ROLES[candidate_index].to_owned(),
                    duration_offset_ticks: *offset,
                    source_bridge_tick_count: (EXPECTED_BASE_SOURCE_TICKS[candidate_index] as i64
                        + *offset) as u64,
                    analytical_survivor: survivor,
                    source_handoff_survivor: survivor,
                    full_flight_cadences: if survivor {
                        vec![
                            "direct_per_tick_120_hz".to_owned(),
                            "held_controller_60_hz".to_owned(),
                        ]
                    } else {
                        Vec::new()
                    },
                    all_contact_replays_passed: survivor,
                });
            }
        }
        rows
    }

    #[test]
    fn frozen_semantic_identity_pin_rejects_tampering() {
        assert!(
            validate_source_summary_identity(
                EXPECTED_SOURCE_DURATION_IDENTITY,
                EXPECTED_SOURCE_DURATION_IDENTITY
            )
            .is_ok()
        );
        assert!(
            validate_source_summary_identity(
                "fnv1a64:0000000000000000",
                EXPECTED_SOURCE_DURATION_IDENTITY
            )
            .is_err()
        );
    }

    #[test]
    fn frozen_family_gate_rejects_tampered_row_coverage() {
        let proof = family_proof();
        let rows = row_bindings();
        assert!(validate_frozen_family_contract(&proof, &rows).is_ok());
        let mut missing = rows.clone();
        missing.pop();
        assert!(validate_frozen_family_contract(&proof, &missing).is_err());
        let mut changed = rows;
        changed[1].analytical_survivor = false;
        assert!(validate_frozen_family_contract(&proof, &changed).is_err());
        let mut changed_proof = proof;
        changed_proof.recorded_failure_row_count -= 1;
        assert!(validate_frozen_family_contract(&changed_proof, &row_bindings()).is_err());
    }

    #[test]
    fn diagnostic_identity_is_repeatable_and_excludes_its_identity_field() {
        let mut artifact = identity_test_artifact();
        let first = artifact_identity(&artifact).unwrap();
        let second = artifact_identity(&artifact).unwrap();
        assert_eq!(first, second);
        artifact.identity = "different self field".to_owned();
        assert_eq!(artifact_identity(&artifact).unwrap(), first);
    }

    #[test]
    fn command_hold_ledger_reports_first_pair_endpoint_and_phase_maxima() {
        let samples = vec![
            SourceDurationCommandSample {
                physics_step: 73,
                phase: "source_bridge".to_owned(),
                desired_command_throttle_frac: 0.55,
                desired_applied_throttle_frac: 0.55,
                held_command_throttle_frac: 0.55,
                plant_applied_throttle_frac: 0.55,
                desired_target_attitude_rad: 0.1,
                held_target_attitude_rad: 0.1,
                throttle_update_due: true,
                attitude_update_due: true,
            },
            SourceDurationCommandSample {
                physics_step: 74,
                phase: "source_bridge".to_owned(),
                desired_command_throttle_frac: 0.60,
                desired_applied_throttle_frac: 0.52,
                held_command_throttle_frac: 0.50,
                plant_applied_throttle_frac: 0.45,
                desired_target_attitude_rad: 0.3,
                held_target_attitude_rad: 0.1,
                throttle_update_due: false,
                attitude_update_due: false,
            },
        ];
        let ledger = command_hold_ledger(&samples);
        let first_boundary = ledger.first_source_control_boundary.unwrap();
        assert_eq!(first_boundary.physics_step, 74);
        assert!(!first_boundary.throttle_update_due);
        assert!(!first_boundary.attitude_update_due);
        let source = ledger
            .phase_summaries
            .iter()
            .find(|phase| phase.phase == "source_bridge")
            .unwrap();
        assert_eq!(source.throttle_update_due_count, 1);
        assert_eq!(source.throttle_held_tick_count, 1);
        assert_eq!(source.attitude_update_due_count, 1);
        assert_eq!(source.attitude_held_tick_count, 1);
        assert!(
            (source
                .maximum_desired_command_vs_held_command
                .maximum_absolute_delta
                - 0.1)
                .abs()
                < 1e-12
        );
        assert!(
            (source
                .maximum_desired_applied_vs_plant_applied
                .maximum_absolute_delta
                - 0.07)
                .abs()
                < 1e-12
        );
        assert!(
            (source
                .maximum_desired_vs_held_target_attitude
                .maximum_absolute_delta
                - 0.2)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn command_ledger_compares_desired_and_actual_throttle_in_matching_units() {
        let desired =
            super::super::super::throttle_request(10.0, 125.25, 5_000.0, 120.0, 1.0 / 120.0, 0.25)
                .unwrap();
        assert_eq!(desired.applied_fraction, 0.25);
        let command = Command {
            throttle_frac: desired.command_fraction,
            target_attitude_rad: 0.0,
        };
        let plant_applied = super::super::super::plant_applied_throttle(command, 0.25, 100.0);
        assert_eq!(plant_applied, 0.25);
        let comparison = command_comparison_sample(&SourceDurationCommandSample {
            physics_step: 74,
            phase: "source_bridge".to_owned(),
            desired_command_throttle_frac: desired.command_fraction,
            desired_applied_throttle_frac: desired.applied_fraction,
            held_command_throttle_frac: desired.command_fraction,
            plant_applied_throttle_frac: plant_applied,
            desired_target_attitude_rad: 0.0,
            held_target_attitude_rad: 0.0,
            throttle_update_due: false,
            attitude_update_due: true,
        });
        assert_eq!(comparison.desired_command_minus_held_command_frac, 0.0);
        assert_eq!(comparison.desired_applied_minus_plant_applied_frac, 0.0);
        assert!((comparison.desired_command_throttle_frac - plant_applied).abs() > 0.24);
    }

    fn identity_test_artifact() -> WaypointDirectSourceDurationHeldCadenceDiagnosticArtifact {
        use flat_candidate_closure::{
            FlatCandidateBindingEvidence, FlatCandidateClosureInputGateEvidence,
        };

        let flat_gate = FlatCandidateClosureInputGateEvidence {
            schema_id: "flat-schema".to_owned(),
            schema_version: 1,
            baseline_identity: "baseline".to_owned(),
            sweep_identity: "sweep".to_owned(),
            nominal_plant_identity: "nominal".to_owned(),
            nominal_input_manifest_identity: "nominal-manifest".to_owned(),
            launch_feasibility_identity: "launch".to_owned(),
            launch_feasibility_input_gate_identity: "launch-gate".to_owned(),
            contact_audit_identity: "contact".to_owned(),
            contact_audit_input_gate_identity: "contact-gate".to_owned(),
            flat_case_id: "flat".to_owned(),
            flat_probe_identity: "probe".to_owned(),
            flat_result_identity: "result".to_owned(),
            original_native_candidate_identity: "native".to_owned(),
            original_shortest_candidate_identity: "shortest".to_owned(),
            third_candidate: FlatCandidateBindingEvidence {
                candidate_identity: "third".to_owned(),
                classification: "Certified".to_owned(),
                selection_scope: "test".to_owned(),
                source_handoff_arc_step: 0,
                source_bridge_tick_count: 0,
                coast_tick_count: 0,
                terminal_bridge_tick_count: 0,
                nominal_profile_tick_count: 0,
            },
            frozen_case_count: 1,
            frozen_selected_profile_count: 1,
            frozen_selection_role_count: 1,
            frozen_cadence_row_count: 1,
            identity: "flat-gate".to_owned(),
        };
        WaypointDirectSourceDurationHeldCadenceDiagnosticArtifact {
            schema_id: WAYPOINT_DIRECT_SOURCE_DURATION_HELD_CADENCE_DIAGNOSTIC_SCHEMA_ID.to_owned(),
            schema_version: WAYPOINT_DIRECT_SOURCE_DURATION_HELD_CADENCE_DIAGNOSTIC_SCHEMA_VERSION,
            characterization_id: WAYPOINT_DIRECT_SOURCE_DURATION_HELD_CADENCE_DIAGNOSTIC_ID
                .to_owned(),
            source_duration_identity: EXPECTED_SOURCE_DURATION_IDENTITY.to_owned(),
            expected_source_duration_sha256: EXPECTED_SOURCE_DURATION_SHA256.to_owned(),
            input_gate: WaypointDirectCoupledThrustAuditInputGateEvidence {
                baseline_identity: "baseline".to_owned(),
                sweep_identity: "sweep".to_owned(),
                nominal_plant_identity: "nominal".to_owned(),
                nominal_input_manifest_identity: "nominal-manifest".to_owned(),
                launch_feasibility_identity: "launch".to_owned(),
                launch_feasibility_input_gate_identity: "launch-gate".to_owned(),
                contact_audit_identity: "contact".to_owned(),
                contact_audit_input_gate_identity: "contact-gate".to_owned(),
                flat_candidate_closure_input_gate: flat_gate,
                flat_canary_identity: "flat-canary".to_owned(),
                flat_canary_input_gate_identity: "flat-gate".to_owned(),
                candidate_identities: EXPECTED_CANDIDATE_IDENTITIES
                    .iter()
                    .map(|identity| (*identity).to_owned())
                    .collect(),
                identity: "coupled-gate".to_owned(),
            },
            coupled_audit_identity: "coupled-audit".to_owned(),
            protocol: protocol_evidence(),
            family_proof: HeldCadenceDiagnosticFamilyProofEvidence {
                predeclared_row_count: 15,
                recorded_row_count: 15,
                analytical_screen_skip_count: 6,
                baseline_pair_count: 9,
                ablation_run_count: 6,
                representative_offset_ticks: -180,
                candidate_order: EXPECTED_CANDIDATE_IDENTITIES
                    .iter()
                    .map(|identity| (*identity).to_owned())
                    .collect(),
                fixed_offset_order_ticks: EXPECTED_OFFSETS.to_vec(),
                row_order_rule: "fixed test order".to_owned(),
                all_rows_recorded: true,
            },
            rows: Vec::new(),
            execution_status: "test".to_owned(),
            scope_non_claims: Vec::new(),
            identity: "mutable self field".to_owned(),
        }
    }
}
