//! Bounded evaluator-only source-duration canary for the three frozen flat
//! candidates. The five duration offsets and all six prior artifact bindings
//! are fixed before any new simulation state is constructed.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::{RunContext, Vec2};
use pd_plan::conservative_ballistic_bridge::{
    AnalyticalBridgeV2, BridgeKindV2, CertificationV2, ComponentMarginsV2, DirectBridgeCandidateV2,
    DirectBridgeReasonV2, KinematicStateV2, MarginV2, exact_discrete_bridge_v2,
};
use serde::{Deserialize, Serialize};

use super::{
    CommandSaturationEvidence, ContactEvidence, LaunchEvidence, LaunchRolloutEvidence,
    LaunchRolloutTickEvidence, PlantStateEvidence, ReseededBridgeEvidence, RolloutCadence,
    SourceClearanceScreenEvidence, WaypointDirectCoupledThrustAuditArtifact,
    WaypointDirectCoupledThrustAuditInputGateEvidence, WaypointDirectCoupledThrustAuditInputPaths,
    resolve_output_dir, stable_digest,
};
use super::{
    coupled_thrust_audit::{
        WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_ID, WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_SCHEMA_ID,
        WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_SCHEMA_VERSION,
        artifact_identity as coupled_audit_artifact_identity,
    },
    flat_candidate_closure::load_coupled_thrust_audit_inputs,
    launch_contact_contract::{
        FirstContactEvidence, ReplayTraceParityEvidence, replay_logged_cadence,
    },
    launch_feasibility::{
        LaunchFeasibilityCadenceRunEvidence, SourceDurationReplayStage, SourceDurationRunRequest,
        run_source_duration_variant,
    },
};

mod held_cadence_diagnostic;
pub use held_cadence_diagnostic::*;
mod paired_command_feasibility;
pub use paired_command_feasibility::*;
mod complete_flat_acceptance;
pub use complete_flat_acceptance::*;
mod direct_generation;
pub use direct_generation::*;

pub const WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_ID: &str =
    "waypoint-direct-source-duration-canary";
pub const WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_SCHEMA_ID: &str =
    "waypoint_direct_source_duration_canary_v1";
pub const WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_SCHEMA_VERSION: u32 = 1;

const EXPECTED_COUPLED_AUDIT_IDENTITY: &str = "fnv1a64:59cbe1bd2dd9b8d2";
const EXPECTED_CANDIDATE_IDENTITIES: [&str; 3] = [
    "fnv1a64:dee613017622ca16",
    "fnv1a64:4e6c0b23f9eb1b8f",
    "fnv1a64:a18a98ad6e334014",
];
const DURATION_OFFSETS_TICKS: [i64; 5] = [-240, -180, -120, -60, 0];
const MAX_PREDECLARED_VARIANTS: usize = 15;
const LAUNCH_TICKS: u64 = 72;
const STATE_POSITION_TOLERANCE_M: f64 = 1.0e-6;
const STATE_VELOCITY_TOLERANCE_MPS: f64 = 1.0e-6;
const PEAK_EQUALITY_TOLERANCE: f64 = 1.0e-10;

#[derive(Clone, Debug)]
pub struct WaypointDirectSourceDurationCanaryInputPaths {
    pub frozen_inputs: WaypointDirectCoupledThrustAuditInputPaths,
    pub coupled_audit_summary: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectSourceDurationCanaryPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectSourceDurationCanaryRun {
    pub artifact: WaypointDirectSourceDurationCanaryArtifact,
    pub paths: WaypointDirectSourceDurationCanaryPaths,
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectSourceDurationCanaryValidation {
    pub input_gate: WaypointDirectCoupledThrustAuditInputGateEvidence,
    pub coupled_audit_identity: String,
    pub basis_candidate_identities: Vec<String>,
    pub predeclared_duration_offsets_ticks: Vec<i64>,
    pub predeclared_variant_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectSourceDurationCanaryArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub input_gate: WaypointDirectCoupledThrustAuditInputGateEvidence,
    pub coupled_audit_identity: String,
    pub protocol: SourceDurationCanaryProtocolEvidence,
    pub frozen_environment: FrozenSourceEnvironmentEvidence,
    pub family_proof: SourceDurationFamilyProofEvidence,
    pub bases: Vec<SourceDurationBasisEvidence>,
    pub execution_status: String,
    pub analytical_survivor_count: usize,
    pub source_handoff_survivor_count: usize,
    pub completed_full_flight_count: usize,
    pub strict_60_hz_handoff_miss_count: usize,
    pub stable_landing_count: usize,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceDurationCanaryProtocolEvidence {
    pub frozen_basis_rule: String,
    pub finite_family_rule: String,
    pub launch_rule: String,
    pub source_bridge_rule: String,
    pub analytical_screen_rule: String,
    pub stage_gate_rule: String,
    pub full_flight_rule: String,
    pub contact_rule: String,
    pub source_clearance_rule: String,
    pub original_v2_classification_rule: String,
    pub non_claim: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenSourceEnvironmentEvidence {
    pub policy_identity: String,
    pub vehicle_identity: String,
    pub flat_scenario_identity: String,
    pub terrain_identity: String,
    pub source_pad_center_x_m: f64,
    pub source_pad_surface_y_m: f64,
    pub source_pad_width_m: f64,
    pub contact_rule: String,
    pub v2_footprint_sign_note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceDurationFamilyProofEvidence {
    pub maximum_predeclared_variant_count: usize,
    pub predeclared_variant_count: usize,
    pub recorded_variant_count: usize,
    pub recorded_failure_row_count: usize,
    pub omitted_failure_row_count: usize,
    pub duration_offsets_ticks: Vec<i64>,
    pub duration_grid_seconds: f64,
    pub candidate_order: Vec<String>,
    pub row_order_rule: String,
    pub all_rows_recorded: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceDurationBasisEvidence {
    pub candidate_identity: String,
    pub role: String,
    pub original_v2_classification: CertificationV2,
    pub original_v2_reasons: Vec<DirectBridgeReasonV2>,
    pub original_source_bridge_identity: String,
    pub original_source_bridge_tick_count: u64,
    pub original_source_handoff_arc_step: u64,
    pub original_source_handoff_state: KinematicStateV2,
    pub frozen_coast_tick_count: u64,
    pub frozen_coast_duration_s: f64,
    pub frozen_terminal_bridge_identity: String,
    pub frozen_terminal_bridge_tick_count: u64,
    pub frozen_tail_profile_tick_count: u64,
    pub frozen_tail_identity: String,
    pub duration_variants: Vec<SourceDurationVariantEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceDurationVariantEvidence {
    pub row_index: usize,
    pub duration_offset_ticks: i64,
    pub source_bridge_tick_count: u64,
    pub unlaunched_source_bridge: UnlaunchedSourceBridgeEvidence,
    pub launch_target_attitude_rad: Option<f64>,
    pub launch_and_analytical_screen: Option<LaunchAndAnalyticalScreenEvidence>,
    pub analysis_error: Option<String>,
    pub analytical_survivor: bool,
    pub source_handoff_gate: Option<SourceHandoffGateEvidence>,
    pub source_handoff_error: Option<String>,
    pub source_handoff_survivor: bool,
    pub source_handoff_skip_reason: Option<String>,
    pub full_flights: Vec<FullFlightEvidence>,
    pub full_flight_skip_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnlaunchedSourceBridgeEvidence {
    pub build_status: String,
    pub build_error: Option<String>,
    pub identity: Option<String>,
    pub matches_frozen_original_identity: bool,
    pub classification: Option<CertificationV2>,
    pub reasons: Vec<DirectBridgeReasonV2>,
    pub margins: Option<ComponentMarginsV2>,
    pub tick_count: u64,
    pub duration_s: Option<f64>,
    pub endpoint_position_error_m: Option<f64>,
    pub endpoint_velocity_error_mps: Option<f64>,
    pub first_powered_attitude_rad: Option<f64>,
    pub measurements: Option<BridgeMeasurementsEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchAndAnalyticalScreenEvidence {
    pub launch: LaunchEvidence,
    pub reseeded_bridge: Option<ReseededBridgeEvidence>,
    pub analytical_screen: Option<AnalyticalScreenEvidence>,
    pub first_tick_equivalence: Option<FirstTickEquivalenceEvidence>,
    pub frozen_original_duration_parity: Option<FrozenOriginalDurationParityEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalyticalScreenEvidence {
    pub reseeded_bridge_identity_reproduced: bool,
    pub reseeded_bridge_classification: Option<CertificationV2>,
    pub reseeded_bridge_reasons: Vec<DirectBridgeReasonV2>,
    pub reseeded_bridge_margins: Option<ComponentMarginsV2>,
    pub measurements: Option<BridgeMeasurementsEvidence>,
    pub bridge_component_screens: Option<BridgeComponentScreensEvidence>,
    pub source_clearance: Option<SourceClearanceScreenEvidence>,
    pub source_attitude_margin: Option<MarginV2>,
    pub launch_boundary: Option<super::LaunchBoundaryEvidence>,
    pub aggregate_fuel_margin: Option<MarginV2>,
    pub aggregate_time_margin: Option<MarginV2>,
    pub analytical_eligible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BridgeComponentScreensEvidence {
    pub coupled_thrust: MarginScreenEvidence,
    pub minimum_throttle: MarginScreenEvidence,
    pub bridge_endpoint: MarginScreenEvidence,
    pub powered_slew: MarginScreenEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarginScreenEvidence {
    pub raw_margin: f64,
    pub normalized_margin: f64,
    pub passes_declared_screen: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BridgeMeasurementsEvidence {
    pub derated_thrust_cap_mps2: f64,
    pub robust_thrust_cap_mps2: f64,
    pub maximum_requested_thrust_mps2: f64,
    pub peak_tick_index: Option<u64>,
    pub peak_thrust_physics_step: Option<u64>,
    pub materialized_sample_count: usize,
    pub materialized_peak_matches_endpoint_peak: bool,
    pub minimum_nonzero_throttle_frac: Option<f64>,
    pub maximum_powered_slew_radps: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FirstTickEquivalenceEvidence {
    pub launch_target_attitude_rad: f64,
    pub unlaunched_first_powered_attitude_rad: f64,
    pub launch_target_matches_unlaunched_direction: bool,
    pub measured_launch_position_m: Vec2,
    pub measured_launch_velocity_mps: Vec2,
    pub reseeded_bridge_start_state: KinematicStateV2,
    pub measured_launch_matches_reseeded_start: bool,
    pub reseeded_first_powered_attitude_rad: Option<f64>,
    pub first_source_sample_command_attitude_rad: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenOriginalDurationParityEvidence {
    pub frozen_launch_log_available: bool,
    pub frozen_reseeded_bridge_identity: Option<String>,
    pub launch_log_matches_frozen: bool,
    pub unlaunched_bridge_matches_frozen: bool,
    pub reseeded_bridge_matches_frozen: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceHandoffGateEvidence {
    pub cadence: String,
    pub launch_matches_analysis: bool,
    pub reseeded_bridge_matches_analysis: bool,
    pub status: String,
    pub source_handoff_reached: bool,
    pub source_handoff_contact_free: bool,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub first_source_tick: Option<super::FirstSourceTickEvidence>,
    pub contacts: Vec<ContactEvidence>,
    pub launch_end_state: Option<PlantStateEvidence>,
    pub termination: PlantStateEvidence,
    pub saturation: CommandSaturationEvidence,
    pub per_step: Vec<LaunchRolloutTickEvidence>,
    pub first_tick_matches_reseeded_command: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FullFlightEvidence {
    pub cadence: String,
    pub launch_matches_analysis: bool,
    pub source_handoff_prefix_matches_gate: Option<bool>,
    pub status: String,
    pub source_handoff_reached: bool,
    pub source_handoff_contact_free: bool,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub strict_source_handoff_within_tolerance: bool,
    pub rollout: LaunchRolloutEvidence,
    pub first_contact: Option<FirstContactEvidence>,
    pub contact_replay_trace: Option<ReplayTraceParityEvidence>,
    pub contact_replay_error: Option<String>,
}

#[derive(Clone)]
struct PreparedDurationVariant {
    row_index: usize,
    duration_offset_ticks: i64,
    source_bridge_tick_count: u64,
    unlaunched_bridge: Option<AnalyticalBridgeV2>,
    unlaunched_evidence: UnlaunchedSourceBridgeEvidence,
    launch_target_attitude_rad: Option<f64>,
}

/// Validate the six frozen source artifacts and the coupled-thrust audit,
/// materialize all fifteen unlaunched duration bridges, and return without
/// constructing a `SimulationState`.
pub fn validate_waypoint_direct_source_duration_canary_inputs(
    repo_root: &Path,
    input_paths: &WaypointDirectSourceDurationCanaryInputPaths,
) -> Result<WaypointDirectSourceDurationCanaryValidation> {
    let prepared = load_coupled_thrust_audit_inputs(repo_root, &input_paths.frozen_inputs)?;
    let input_gate = prepared
        .coupled_input_gate
        .as_ref()
        .ok_or_else(|| anyhow!("coupled-thrust input gate was not rebuilt"))?;
    let audit = read_coupled_audit(&input_paths.coupled_audit_summary, input_gate)?;
    let variants = prepare_duration_variants(&prepared)?;
    Ok(WaypointDirectSourceDurationCanaryValidation {
        input_gate: input_gate.clone(),
        coupled_audit_identity: audit.identity,
        basis_candidate_identities: variants
            .iter()
            .map(|variant| variant.0.candidate.identity.clone())
            .collect::<Vec<_>>(),
        predeclared_duration_offsets_ticks: DURATION_OFFSETS_TICKS.to_vec(),
        predeclared_variant_count: variants.iter().map(|(_, rows)| rows.len()).sum(),
    })
}

/// Run the predeclared 3 x 5 family. Every input and baseline-duration bridge
/// is identity checked before the first plant state is created.
pub fn run_waypoint_direct_source_duration_canary(
    repo_root: &Path,
    input_paths: &WaypointDirectSourceDurationCanaryInputPaths,
    output_dir: &Path,
) -> Result<WaypointDirectSourceDurationCanaryRun> {
    let prepared = load_coupled_thrust_audit_inputs(repo_root, &input_paths.frozen_inputs)?;
    let input_gate = prepared
        .coupled_input_gate
        .as_ref()
        .ok_or_else(|| anyhow!("coupled-thrust input gate was not rebuilt"))?;
    let audit = read_coupled_audit(&input_paths.coupled_audit_summary, input_gate)?;
    let prepared_variants = prepare_duration_variants(&prepared)?;
    let predeclared_count: usize = prepared_variants.iter().map(|(_, rows)| rows.len()).sum();
    if predeclared_count != MAX_PREDECLARED_VARIANTS {
        bail!("source-duration family does not contain exactly fifteen predeclared rows");
    }

    let output_dir = resolve_output_dir(repo_root, output_dir);
    let summary_path = output_dir.join("summary.json");
    if summary_path.exists() {
        bail!(
            "source-duration canary refuses to overwrite existing summary {}",
            summary_path.display()
        );
    }

    // No simulation can occur above this point. Each row has its exact
    // unlaunched bridge and candidate-specific target before launch execution.
    let mut bases = prepared_variants
        .into_iter()
        .map(|(candidate_input, variants)| {
            execute_basis_variants(&prepared, &candidate_input, variants)
        })
        .collect::<Result<Vec<_>>>()?;

    let source_handoff_survivor_count_before_flights = bases
        .iter()
        .flat_map(|basis| &basis.duration_variants)
        .filter(|variant| variant.source_handoff_survivor)
        .count();
    if source_handoff_survivor_count_before_flights > 0 {
        complete_full_flights(&prepared, &mut bases)?;
    } else {
        let analytical_count = bases
            .iter()
            .flat_map(|basis| &basis.duration_variants)
            .filter(|variant| variant.analytical_survivor)
            .count();
        let reason = full_flight_skip_reason(analytical_count, false)
            .expect("a failed source gate must skip full flight")
            .to_owned();
        for variant in bases
            .iter_mut()
            .flat_map(|basis| &mut basis.duration_variants)
        {
            if variant.full_flight_skip_reason.is_none() {
                variant.full_flight_skip_reason = Some(reason.clone());
            }
        }
    }

    let analytical_survivor_count = bases
        .iter()
        .flat_map(|basis| &basis.duration_variants)
        .filter(|variant| variant.analytical_survivor)
        .count();
    let source_handoff_survivor_count = bases
        .iter()
        .flat_map(|basis| &basis.duration_variants)
        .filter(|variant| variant.source_handoff_survivor)
        .count();
    let completed_full_flight_count = bases
        .iter()
        .flat_map(|basis| &basis.duration_variants)
        .flat_map(|variant| &variant.full_flights)
        .filter(|flight| flight.status != "full_flight_stage_failed")
        .count();
    let strict_60_hz_handoff_miss_count = bases
        .iter()
        .flat_map(|basis| &basis.duration_variants)
        .flat_map(|variant| &variant.full_flights)
        .filter(|flight| {
            flight.cadence == "held_controller_60_hz"
                && !flight.strict_source_handoff_within_tolerance
        })
        .count();
    let stable_landing_count = bases
        .iter()
        .flat_map(|basis| &basis.duration_variants)
        .flat_map(|variant| &variant.full_flights)
        .filter(|flight| {
            flight
                .rollout
                .termination
                .physical_outcome
                .starts_with("landed_")
                || flight.rollout.termination.mission_outcome == "success"
        })
        .count();
    let recorded_count: usize = bases
        .iter()
        .map(|basis| basis.duration_variants.len())
        .sum();
    if recorded_count != predeclared_count {
        bail!("source-duration canary omitted a predeclared variant row");
    }
    let recorded_failure_row_count = bases
        .iter()
        .flat_map(|basis| &basis.duration_variants)
        .filter(|variant| {
            !variant.analytical_survivor
                || !variant.source_handoff_survivor
                || variant.analysis_error.is_some()
                || variant.source_handoff_error.is_some()
                || variant.full_flights.iter().any(|flight| {
                    flight.status == "full_flight_stage_failed"
                        || !flight.strict_source_handoff_within_tolerance
                        || flight.rollout.termination.mission_outcome != "success"
                        || flight.contact_replay_error.is_some()
                        || flight
                            .contact_replay_trace
                            .as_ref()
                            .is_some_and(|trace| !trace.passed)
                })
        })
        .count();

    let mut artifact = WaypointDirectSourceDurationCanaryArtifact {
        schema_id: WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_ID.to_owned(),
        input_gate: input_gate.clone(),
        coupled_audit_identity: audit.identity,
        protocol: protocol_evidence(),
        frozen_environment: frozen_environment(&prepared)?,
        family_proof: SourceDurationFamilyProofEvidence {
            maximum_predeclared_variant_count: MAX_PREDECLARED_VARIANTS,
            predeclared_variant_count: predeclared_count,
            recorded_variant_count: recorded_count,
            recorded_failure_row_count,
            omitted_failure_row_count: 0,
            duration_offsets_ticks: DURATION_OFFSETS_TICKS.to_vec(),
            duration_grid_seconds: 0.5,
            candidate_order: EXPECTED_CANDIDATE_IDENTITIES
                .iter()
                .map(|identity| (*identity).to_owned())
                .collect(),
            row_order_rule: "candidate order native, research-shortest, third; within each candidate sort fixed offsets -240, -180, -120, -60, 0 physics ticks; retain every predeclared row including bridge, screen, execution, and contact-replay failures".to_owned(),
            all_rows_recorded: recorded_count == predeclared_count,
        },
        bases,
        execution_status: if analytical_survivor_count == 0 {
            "stopped_before_source_handoff_no_analytical_survivors".to_owned()
        } else if source_handoff_survivor_count == 0 {
            "stopped_before_full_flights_no_source_handoff_survivors".to_owned()
        } else {
            "full_flights_completed_for_source_handoff_survivors".to_owned()
        },
        analytical_survivor_count,
        source_handoff_survivor_count,
        completed_full_flight_count,
        strict_60_hz_handoff_miss_count,
        stable_landing_count,
        scope_non_claims: vec![
            "The new duration wrapper is evaluator evidence; it is never labeled as a V2 Certified candidate and does not alter the original V2 classification.".to_owned(),
            "A stable landing does not erase a failed robust-thrust screen or a strict 60 Hz source-handoff miss.".to_owned(),
            "A finite bounded family is not a general candidate search and cannot establish physical impossibility.".to_owned(),
            "No planner, simulator/contact rule, controller, F6 route, default selection, or existing artifact is changed.".to_owned(),
        ],
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;

    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create source-duration canary output directory {}",
            output_dir.display()
        )
    })?;
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&summary_path)
        .with_context(|| {
            format!(
                "source-duration canary refuses to overwrite summary {}",
                summary_path.display()
            )
        })?;
    file.write_all(&summary_bytes).with_context(|| {
        format!(
            "failed to write source-duration canary summary {}",
            summary_path.display()
        )
    })?;
    drop(file);
    let reloaded: WaypointDirectSourceDurationCanaryArtifact =
        serde_json::from_slice(&summary_bytes)
            .context("failed to reload source-duration canary summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes {
        bail!("source-duration canary summary is not byte-stable after reload");
    }
    if artifact_identity(&reloaded)? != reloaded.identity {
        bail!("source-duration canary semantic identity failed round-trip check");
    }

    Ok(WaypointDirectSourceDurationCanaryRun {
        artifact: reloaded,
        paths: WaypointDirectSourceDurationCanaryPaths {
            output_dir,
            summary_path,
        },
    })
}

fn read_coupled_audit(
    path: &Path,
    expected_input_gate: &WaypointDirectCoupledThrustAuditInputGateEvidence,
) -> Result<WaypointDirectCoupledThrustAuditArtifact> {
    let audit: WaypointDirectCoupledThrustAuditArtifact = super::read_summary(path)?;
    let computed_identity = coupled_audit_artifact_identity(&audit)?;
    if computed_identity != audit.identity
        || audit.identity != EXPECTED_COUPLED_AUDIT_IDENTITY
        || audit.schema_id != WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_SCHEMA_ID
        || audit.schema_version != WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_SCHEMA_VERSION
        || audit.characterization_id != WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_ID
    {
        bail!("coupled-thrust audit identity or frozen-input binding changed");
    }
    validate_audit_input_gate(&audit.input_gate, expected_input_gate)?;
    if audit.candidates.len() != EXPECTED_CANDIDATE_IDENTITIES.len()
        || audit
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_identity.as_str())
            .collect::<Vec<_>>()
            != EXPECTED_CANDIDATE_IDENTITIES
    {
        bail!("coupled-thrust audit identity or frozen-input binding changed");
    }
    Ok(audit)
}

fn audit_input_gate_matches(
    actual: &WaypointDirectCoupledThrustAuditInputGateEvidence,
    expected: &WaypointDirectCoupledThrustAuditInputGateEvidence,
) -> bool {
    actual == expected
}

fn validate_audit_input_gate(
    actual: &WaypointDirectCoupledThrustAuditInputGateEvidence,
    expected: &WaypointDirectCoupledThrustAuditInputGateEvidence,
) -> Result<()> {
    if !audit_input_gate_matches(actual, expected) {
        bail!("coupled-thrust audit identity or frozen-input binding changed");
    }
    Ok(())
}

fn prepare_duration_variants(
    prepared: &super::flat_candidate_closure::CoupledThrustAuditValidatedInputs,
) -> Result<
    Vec<(
        super::flat_candidate_closure::CoupledThrustAuditCandidateInput,
        Vec<PreparedDurationVariant>,
    )>,
> {
    if prepared.candidates.len() != EXPECTED_CANDIDATE_IDENTITIES.len()
        || prepared
            .candidates
            .iter()
            .map(|candidate| candidate.candidate.identity.as_str())
            .collect::<Vec<_>>()
            != EXPECTED_CANDIDATE_IDENTITIES
    {
        bail!("frozen candidate basis order or identity changed");
    }
    let mut result = Vec::with_capacity(prepared.candidates.len());
    for (candidate_index, candidate_input) in prepared.candidates.iter().enumerate() {
        let candidate = &candidate_input.candidate;
        let original = candidate
            .source_bridge
            .as_ref()
            .ok_or_else(|| anyhow!("basis {} has no original source bridge", candidate.identity))?;
        let handoff = candidate
            .source_handoff
            .ok_or_else(|| anyhow!("basis {} has no source handoff", candidate.identity))?;
        if candidate.classification != CertificationV2::Certified
            || candidate_input.selected_profile.candidate_identity != candidate.identity
            || original.kind != BridgeKindV2::Source
            || candidate.identity != EXPECTED_CANDIDATE_IDENTITIES[candidate_index]
        {
            bail!(
                "frozen basis {} no longer matches its certified source profile",
                candidate.identity
            );
        }
        let mut variants = Vec::with_capacity(DURATION_OFFSETS_TICKS.len());
        for (duration_index, duration_offset_ticks) in DURATION_OFFSETS_TICKS.iter().enumerate() {
            let source_bridge_tick_count = i64::try_from(original.steps)
                .ok()
                .and_then(|steps| steps.checked_add(*duration_offset_ticks))
                .filter(|steps| *steps > 0)
                .and_then(|steps| u64::try_from(steps).ok())
                .ok_or_else(|| {
                    anyhow!(
                        "basis {} duration offset {} produces a nonpositive tick count",
                        candidate.identity,
                        duration_offset_ticks
                    )
                })?;
            let start_state = original.start_state;
            let built = exact_discrete_bridge_v2(
                &prepared.policy,
                &prepared.vehicle,
                BridgeKindV2::Source,
                start_state,
                handoff.state,
                source_bridge_tick_count,
            );
            if *duration_offset_ticks == 0 {
                let rebuilt = built.as_ref().map_err(|error| {
                    anyhow!(
                        "baseline duration bridge for {} failed to rebuild: {error}",
                        candidate.identity
                    )
                })?;
                if rebuilt.identity != original.identity
                    || rebuilt.classification != original.classification
                    || rebuilt.reasons != original.reasons
                    || rebuilt.margins != original.margins
                    || rebuilt.endpoint_position_error_m != original.endpoint_position_error_m
                    || rebuilt.endpoint_velocity_error_mps != original.endpoint_velocity_error_mps
                {
                    bail!(
                        "baseline duration bridge for {} differs from its frozen source bridge",
                        candidate.identity
                    );
                }
            }
            let (unlaunched_bridge, evidence, launch_target_attitude_rad) = match built {
                Ok(bridge) => {
                    let target = bridge
                        .samples
                        .iter()
                        .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
                        .map(|sample| direction_angle(sample.thrust_acceleration_mps2));
                    let measurements =
                        bridge_measurements(&bridge, &prepared.policy, &prepared.vehicle);
                    (
                        Some(bridge.clone()),
                        UnlaunchedSourceBridgeEvidence {
                            build_status: "materialized".to_owned(),
                            build_error: None,
                            identity: Some(bridge.identity.clone()),
                            matches_frozen_original_identity: bridge.identity == original.identity,
                            classification: Some(bridge.classification),
                            reasons: bridge.reasons.clone(),
                            margins: Some(bridge.margins),
                            tick_count: bridge.steps,
                            duration_s: Some(bridge.duration_s),
                            endpoint_position_error_m: Some(bridge.endpoint_position_error_m),
                            endpoint_velocity_error_mps: Some(bridge.endpoint_velocity_error_mps),
                            first_powered_attitude_rad: target,
                            measurements: Some(measurements),
                        },
                        target,
                    )
                }
                Err(error) => (
                    None,
                    UnlaunchedSourceBridgeEvidence {
                        build_status: "bridge_build_failed".to_owned(),
                        build_error: Some(error.to_string()),
                        identity: None,
                        matches_frozen_original_identity: false,
                        classification: None,
                        reasons: Vec::new(),
                        margins: None,
                        tick_count: source_bridge_tick_count,
                        duration_s: None,
                        endpoint_position_error_m: None,
                        endpoint_velocity_error_mps: None,
                        first_powered_attitude_rad: None,
                        measurements: None,
                    },
                    None,
                ),
            };
            variants.push(PreparedDurationVariant {
                row_index: candidate_index * DURATION_OFFSETS_TICKS.len() + duration_index,
                duration_offset_ticks: *duration_offset_ticks,
                source_bridge_tick_count,
                unlaunched_bridge,
                unlaunched_evidence: evidence,
                launch_target_attitude_rad,
            });
        }
        result.push((candidate_input.clone(), variants));
    }
    let count: usize = result.iter().map(|(_, rows)| rows.len()).sum();
    if count != MAX_PREDECLARED_VARIANTS {
        bail!("source-duration canary did not materialize all fifteen predeclared rows");
    }
    Ok(result)
}

fn execute_basis_variants(
    prepared: &super::flat_candidate_closure::CoupledThrustAuditValidatedInputs,
    candidate_input: &super::flat_candidate_closure::CoupledThrustAuditCandidateInput,
    variants: Vec<PreparedDurationVariant>,
) -> Result<SourceDurationBasisEvidence> {
    execute_basis_variants_from_physical(
        &prepared.flat_case.scenario,
        &prepared.flat_case.probe,
        &prepared.policy,
        &prepared.vehicle,
        &candidate_input.candidate,
        &candidate_input.selected_profile,
        Some(candidate_input),
        match candidate_input.candidate.identity.as_str() {
            "fnv1a64:dee613017622ca16" => "native",
            "fnv1a64:4e6c0b23f9eb1b8f" => "research_shortest",
            "fnv1a64:a18a98ad6e334014" => "third",
            _ => "unknown",
        },
        variants,
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_basis_variants_from_physical(
    scenario: &pd_core::ScenarioSpec,
    probe: &pd_plan::conservative_ballistic_bridge::DirectBridgeProbeV2,
    policy: &super::DirectBridgePolicyV2,
    vehicle: &super::VehicleInputV2,
    candidate: &DirectBridgeCandidateV2,
    selected_profile: &super::PreparedProfileCandidate,
    historical_binding: Option<&super::flat_candidate_closure::CoupledThrustAuditCandidateInput>,
    role: &str,
    variants: Vec<PreparedDurationVariant>,
) -> Result<SourceDurationBasisEvidence> {
    let source_bridge = candidate.source_bridge.as_ref().ok_or_else(|| {
        anyhow!(
            "basis {} lost its original source bridge",
            candidate.identity
        )
    })?;
    let handoff = candidate
        .source_handoff
        .ok_or_else(|| anyhow!("basis {} lost its source handoff", candidate.identity))?;
    let coast = candidate
        .selected_coast
        .as_ref()
        .ok_or_else(|| anyhow!("basis {} lost its selected coast", candidate.identity))?;
    let terminal = candidate
        .terminal_bridge
        .as_ref()
        .ok_or_else(|| anyhow!("basis {} lost its terminal bridge", candidate.identity))?;
    let original_tail_start = selected_profile
        .profile
        .accounting
        .source_bridge_sample_count as usize;
    let frozen_tail = selected_profile
        .profile
        .ticks
        .get(original_tail_start..)
        .ok_or_else(|| anyhow!("basis {} frozen tail is truncated", candidate.identity))?;
    let frozen_tail_identity = stable_digest(
        &frozen_tail
            .iter()
            .map(|tick| {
                (
                    tick.phase.as_str(),
                    tick.expected_state,
                    tick.thrust_acceleration_mps2,
                    tick.target_attitude_rad,
                )
            })
            .collect::<Vec<_>>(),
    )?;
    let expected_variant_count = variants.len();
    let mut evidence = Vec::with_capacity(variants.len());
    for variant in variants {
        let mut row = SourceDurationVariantEvidence {
            row_index: variant.row_index,
            duration_offset_ticks: variant.duration_offset_ticks,
            source_bridge_tick_count: variant.source_bridge_tick_count,
            unlaunched_source_bridge: variant.unlaunched_evidence,
            launch_target_attitude_rad: variant.launch_target_attitude_rad,
            launch_and_analytical_screen: None,
            analysis_error: None,
            analytical_survivor: false,
            source_handoff_gate: None,
            source_handoff_error: None,
            source_handoff_survivor: false,
            source_handoff_skip_reason: None,
            full_flights: Vec::new(),
            full_flight_skip_reason: None,
        };
        let Some(unlaunched_bridge) = variant.unlaunched_bridge.as_ref() else {
            row.analysis_error = Some(
                row.unlaunched_source_bridge
                    .build_error
                    .clone()
                    .unwrap_or_else(|| "unlaunched source bridge did not materialize".to_owned()),
            );
            row.source_handoff_skip_reason =
                Some("unlaunched_source_bridge_unavailable".to_owned());
            row.full_flight_skip_reason = Some("unlaunched_source_bridge_unavailable".to_owned());
            evidence.push(row);
            continue;
        };
        let Some(launch_target_attitude_rad) = variant.launch_target_attitude_rad else {
            row.analysis_error =
                Some("unlaunched bridge has no nonzero thrust direction".to_owned());
            row.source_handoff_skip_reason = Some("launch_target_unavailable".to_owned());
            row.full_flight_skip_reason = Some("launch_target_unavailable".to_owned());
            evidence.push(row);
            continue;
        };

        let analysis = run_source_duration_variant(SourceDurationRunRequest {
            scenario,
            probe,
            selected: selected_profile,
            basis: candidate,
            policy,
            vehicle,
            source_bridge_steps: variant.source_bridge_tick_count,
            launch_tilt_attitude_rad: launch_target_attitude_rad,
            cadence: RolloutCadence::DirectPerTick,
            stage: SourceDurationReplayStage::AnalyzeOnly,
        });
        let analysis = match analysis {
            Ok(run) => run,
            Err(error) => {
                row.analysis_error = Some(error.to_string());
                row.source_handoff_skip_reason = Some("analytical_launch_stage_failed".to_owned());
                row.full_flight_skip_reason = Some("analytical_launch_stage_failed".to_owned());
                evidence.push(row);
                continue;
            }
        };
        let reseeded = analysis.reseeded_bridge.clone();
        let analytical_screen =
            build_analytical_screen(reseeded.as_ref(), candidate, policy, vehicle)?;
        let first_tick_equivalence = build_first_tick_equivalence(
            launch_target_attitude_rad,
            unlaunched_bridge,
            reseeded.as_ref(),
            analysis.launch.end_state.as_ref(),
            candidate,
            policy,
            vehicle,
        );
        let frozen_original_duration_parity = if variant.duration_offset_ticks == 0 {
            historical_binding.map(|candidate_input| {
                frozen_original_duration_parity(
                    candidate_input,
                    &analysis,
                    &row.unlaunched_source_bridge,
                )
            })
        } else {
            None
        };
        row.analytical_survivor = analytical_screen
            .as_ref()
            .is_some_and(|screen| screen.analytical_eligible)
            && analysis.launch.completed
            && analysis.launch.contact_free;
        row.launch_and_analytical_screen = Some(LaunchAndAnalyticalScreenEvidence {
            launch: analysis.launch.clone(),
            reseeded_bridge: reseeded.clone(),
            analytical_screen,
            first_tick_equivalence,
            frozen_original_duration_parity,
        });
        if !row.analytical_survivor {
            row.source_handoff_skip_reason = Some(if !analysis.launch.completed {
                "launch_did_not_complete".to_owned()
            } else if !analysis.launch.contact_free {
                "launch_contact_observed".to_owned()
            } else {
                source_handoff_skip_reason(row.analytical_survivor)
                    .unwrap_or("analytical_screen_failed")
                    .to_owned()
            });
            row.full_flight_skip_reason = Some("no_analytical_survivor_for_source_gate".to_owned());
            evidence.push(row);
            continue;
        }

        let source_gate = run_source_duration_variant(SourceDurationRunRequest {
            scenario,
            probe,
            selected: selected_profile,
            basis: candidate,
            policy,
            vehicle,
            source_bridge_steps: variant.source_bridge_tick_count,
            launch_tilt_attitude_rad: launch_target_attitude_rad,
            cadence: RolloutCadence::DirectPerTick,
            stage: SourceDurationReplayStage::SourceHandoff,
        });
        let source_gate = match source_gate {
            Ok(run) => run,
            Err(error) => {
                row.source_handoff_error = Some(error.to_string());
                row.source_handoff_skip_reason = Some("source_handoff_stage_failed".to_owned());
                row.full_flight_skip_reason = Some("source_handoff_stage_failed".to_owned());
                evidence.push(row);
                continue;
            }
        };
        let launch_matches_analysis = source_gate.launch == analysis.launch;
        let reseeded_bridge_matches_analysis =
            source_gate.reseeded_bridge == analysis.reseeded_bridge;
        let source_handoff_reached = source_gate.rollout.source_handoff_reached;
        let source_handoff_contact_free = source_gate.rollout.source_handoff_contact_free;
        let first_tick_matches_reseeded_command = first_tick_matches_reseeded_command(
            source_gate.rollout.first_source_tick.as_ref(),
            source_gate.reseeded_bridge.as_ref(),
        );
        let launch_end_state = source_gate.launch.end_state.clone();
        let source_gate_evidence = SourceHandoffGateEvidence {
            cadence: source_gate.cadence.clone(),
            launch_matches_analysis,
            reseeded_bridge_matches_analysis,
            status: source_gate.rollout.status.clone(),
            source_handoff_reached,
            source_handoff_contact_free,
            source_handoff_position_error_m: source_gate.rollout.source_handoff_position_error_m,
            source_handoff_velocity_error_mps: source_gate
                .rollout
                .source_handoff_velocity_error_mps,
            first_source_tick: source_gate.rollout.first_source_tick.clone(),
            contacts: source_gate.rollout.contacts.clone(),
            launch_end_state,
            termination: source_gate.rollout.termination.clone(),
            saturation: source_gate.rollout.saturation.clone(),
            per_step: source_gate.rollout.per_step.clone(),
            first_tick_matches_reseeded_command,
        };
        row.source_handoff_survivor = source_handoff_reached
            && source_handoff_contact_free
            && launch_matches_analysis
            && reseeded_bridge_matches_analysis
            && first_tick_matches_reseeded_command
            && source_gate
                .rollout
                .source_handoff_position_error_m
                .is_some_and(|error| error <= STATE_POSITION_TOLERANCE_M)
            && source_gate
                .rollout
                .source_handoff_velocity_error_mps
                .is_some_and(|error| error <= STATE_VELOCITY_TOLERANCE_MPS);
        row.source_handoff_gate = Some(source_gate_evidence);
        if !row.source_handoff_survivor {
            row.source_handoff_skip_reason = Some(source_handoff_failure_reason(&source_gate));
            row.full_flight_skip_reason = Some("source_handoff_gate_failed".to_owned());
            evidence.push(row);
            continue;
        }

        evidence.push(row);
    }
    if evidence.len() != expected_variant_count {
        bail!(
            "basis {} omitted one or more duration rows",
            candidate.identity
        );
    }
    let coast_ticks = (coast.duration_s * f64::from(policy.physics_hz)).round() as u64;
    Ok(SourceDurationBasisEvidence {
        candidate_identity: candidate.identity.clone(),
        role: role.to_owned(),
        original_v2_classification: candidate.classification,
        original_v2_reasons: candidate.reasons.clone(),
        original_source_bridge_identity: source_bridge.identity.clone(),
        original_source_bridge_tick_count: source_bridge.steps,
        original_source_handoff_arc_step: handoff.arc_step,
        original_source_handoff_state: handoff.state,
        frozen_coast_tick_count: coast_ticks,
        frozen_coast_duration_s: coast.duration_s,
        frozen_terminal_bridge_identity: terminal.identity.clone(),
        frozen_terminal_bridge_tick_count: terminal.steps,
        frozen_tail_profile_tick_count: frozen_tail.len() as u64,
        frozen_tail_identity,
        duration_variants: evidence,
    })
}

fn complete_full_flights(
    prepared: &super::flat_candidate_closure::CoupledThrustAuditValidatedInputs,
    bases: &mut [SourceDurationBasisEvidence],
) -> Result<()> {
    let context =
        RunContext::from_scenario(&prepared.flat_case.scenario).map_err(anyhow::Error::msg)?;
    for basis in bases {
        let candidate_input = prepared
            .candidates
            .iter()
            .find(|candidate| candidate.candidate.identity == basis.candidate_identity)
            .ok_or_else(|| anyhow!("validated basis {} disappeared", basis.candidate_identity))?;
        let candidate = &candidate_input.candidate;
        for variant in &mut basis.duration_variants {
            if !variant.source_handoff_survivor {
                if variant.full_flight_skip_reason.is_none() {
                    variant.full_flight_skip_reason = Some(if variant.analytical_survivor {
                        "source_handoff_gate_failed".to_owned()
                    } else {
                        "no_analytical_survivor_for_source_gate".to_owned()
                    });
                }
                continue;
            }
            let launch_target_attitude_rad = variant
                .launch_target_attitude_rad
                .ok_or_else(|| anyhow!("surviving variant has no launch target"))?;
            let analysis = variant
                .launch_and_analytical_screen
                .as_ref()
                .ok_or_else(|| anyhow!("surviving variant has no analytical evidence"))?;
            let source_gate = variant
                .source_handoff_gate
                .as_ref()
                .ok_or_else(|| anyhow!("surviving variant has no source handoff gate"))?;
            for cadence in [
                RolloutCadence::DirectPerTick,
                RolloutCadence::ControllerCadence,
            ] {
                let full = run_source_duration_variant(SourceDurationRunRequest {
                    scenario: &prepared.flat_case.scenario,
                    probe: &prepared.flat_case.probe,
                    selected: &candidate_input.selected_profile,
                    basis: candidate,
                    policy: &prepared.policy,
                    vehicle: &prepared.vehicle,
                    source_bridge_steps: variant.source_bridge_tick_count,
                    launch_tilt_attitude_rad: launch_target_attitude_rad,
                    cadence,
                    stage: SourceDurationReplayStage::FullFlight,
                });
                match full {
                    Ok(run) => {
                        let source_handoff_prefix_matches_gate =
                            (cadence == RolloutCadence::DirectPerTick).then(|| {
                                source_gate_prefix_matches_evidence(source_gate, analysis, &run)
                            });
                        let (first_contact, contact_replay_trace, contact_replay_error) =
                            match replay_logged_cadence(&context, &run) {
                                Ok(replay) => (replay.first_contact, Some(replay.trace), None),
                                Err(error) => (None, None, Some(error.to_string())),
                            };
                        let strict_source_handoff_within_tolerance =
                            run.rollout.source_handoff_contact_free
                                && run
                                    .rollout
                                    .source_handoff_position_error_m
                                    .is_some_and(|error| error <= STATE_POSITION_TOLERANCE_M)
                                && run
                                    .rollout
                                    .source_handoff_velocity_error_mps
                                    .is_some_and(|error| error <= STATE_VELOCITY_TOLERANCE_MPS);
                        variant.full_flights.push(FullFlightEvidence {
                            cadence: run.cadence,
                            launch_matches_analysis: run.launch == analysis.launch,
                            source_handoff_prefix_matches_gate,
                            status: run.rollout.status.clone(),
                            source_handoff_reached: run.rollout.source_handoff_reached,
                            source_handoff_contact_free: run.rollout.source_handoff_contact_free,
                            source_handoff_position_error_m: run
                                .rollout
                                .source_handoff_position_error_m,
                            source_handoff_velocity_error_mps: run
                                .rollout
                                .source_handoff_velocity_error_mps,
                            strict_source_handoff_within_tolerance,
                            rollout: run.rollout,
                            first_contact,
                            contact_replay_trace,
                            contact_replay_error,
                        });
                    }
                    Err(error) => variant.full_flights.push(FullFlightEvidence {
                        cadence: cadence_name(cadence),
                        launch_matches_analysis: false,
                        source_handoff_prefix_matches_gate: None,
                        status: "full_flight_stage_failed".to_owned(),
                        source_handoff_reached: false,
                        source_handoff_contact_free: false,
                        source_handoff_position_error_m: None,
                        source_handoff_velocity_error_mps: None,
                        strict_source_handoff_within_tolerance: false,
                        rollout: failed_rollout(&error.to_string()),
                        first_contact: None,
                        contact_replay_trace: None,
                        contact_replay_error: Some(error.to_string()),
                    }),
                }
            }
        }
    }
    Ok(())
}

fn source_gate_prefix_matches_evidence(
    gate: &SourceHandoffGateEvidence,
    analysis: &LaunchAndAnalyticalScreenEvidence,
    completed: &LaunchFeasibilityCadenceRunEvidence,
) -> bool {
    let prefix_len = gate.per_step.len();
    completed.cadence == gate.cadence
        && completed.launch == analysis.launch
        && completed.reseeded_bridge == analysis.reseeded_bridge
        && gate.launch_matches_analysis
        && gate.reseeded_bridge_matches_analysis
        && prefix_len > 0
        && completed.rollout.per_step.get(..prefix_len) == Some(&gate.per_step)
        && gate.source_handoff_reached == completed.rollout.source_handoff_reached
        && gate.source_handoff_contact_free == completed.rollout.source_handoff_contact_free
        && gate.source_handoff_position_error_m == completed.rollout.source_handoff_position_error_m
        && gate.source_handoff_velocity_error_mps
            == completed.rollout.source_handoff_velocity_error_mps
}

fn frozen_environment(
    prepared: &super::flat_candidate_closure::CoupledThrustAuditValidatedInputs,
) -> Result<FrozenSourceEnvironmentEvidence> {
    let case = &prepared.flat_case;
    let terrain_identity = stable_digest(&case.scenario.world.terrain)?;
    let source = &case.probe.source;
    Ok(FrozenSourceEnvironmentEvidence {
        policy_identity: stable_digest(&prepared.policy)?,
        vehicle_identity: stable_digest(&prepared.vehicle)?,
        flat_scenario_identity: case.evidence.scenario_identity.clone(),
        terrain_identity,
        source_pad_center_x_m: source.center_x_m,
        source_pad_surface_y_m: source.surface_y_m,
        source_pad_width_m: source.width_m,
        contact_rule: "advance the unchanged authoritative SimulationState::step path; retain actual contact events, state, and first-contact predicates; terminate each lane when core simulation terminates".to_owned(),
        v2_footprint_sign_note: "the analytical source-clearance mirror retains the V2 convention; actual contact and first-contact predicates come from authoritative core geometry. The known V2 footprint-sign ambiguity is recorded, and neither model is changed".to_owned(),
    })
}

fn protocol_evidence() -> SourceDurationCanaryProtocolEvidence {
    SourceDurationCanaryProtocolEvidence {
        frozen_basis_rule: "rebuild and validate the six sealed source artifacts and coupled-thrust audit identity before creating any SimulationState; use exactly the native, research-shortest, and third flat candidate semantic identities in their frozen order".to_owned(),
        finite_family_rule: "for each original source-bridge tick count n, try exactly n-240, n-180, n-120, n-60, n; these are the only 15 variants on a 0.5 second grid at 120 Hz, with no adaptive tuning, candidate search, or omitted failed rows".to_owned(),
        launch_rule: "materialize each unlaunched exact source bridge from the frozen source-pad-rest state first; set tilt from its first nonzero thrust vector using atan2(thrust.x, thrust.y), even when that bridge is NotCertified; then apply 60 upright and 12 candidate-specific tilt full-throttle physics ticks".to_owned(),
        source_bridge_rule: "after launch, re-solve the source bridge from measured launch position and velocity to the original frozen source handoff using the variant tick count; preserve original handoff, coast, terminal bridge, policy, vehicle, terrain, and contact rule".to_owned(),
        analytical_screen_rule: "report the bridge's 13.32 m/s^2 robust coupled-thrust cap at the frozen policy/vehicle, minimum throttle, powered slew, endpoint, source-environment clearance, source attitude, launch-end-to-first-powered-source-tick slew, aggregate fuel, and mission-time margins; retain the original V2 classification separately".to_owned(),
        stage_gate_rule: "only analytical survivors receive a 120 Hz contact-free source-handoff run; only source-handoff survivors receive full 120 Hz and held 60 Hz flights; if a stage has no survivors, stop before the next stage and record skip reasons for every row".to_owned(),
        full_flight_rule: "retain every source-handoff and full-flight per-step record, saturation counter, termination state, strict handoff position/velocity errors, and source-gate-to-full-120 prefix parity where a row advances".to_owned(),
        contact_rule: "use authoritative core SimulationState contact events and the existing paired replay to record first-contact predicates, event parity, and outcomes; stable landing and strict 60 Hz handoff are separate observations".to_owned(),
        source_clearance_rule: "screen every reseeded source-bridge sample against the frozen source-pad and terrain environment using the existing V2 source-clearance mirror; separately report authoritative flown contact without changing its geometry".to_owned(),
        original_v2_classification_rule: "the basis retains its original V2 classification and reasons; a duration bridge or reseeded wrapper result never rewrites or promotes that candidate".to_owned(),
        non_claim: "the finite duration family cannot prove physical impossibility, and a landed wrapper does not establish robustness or a strict 60 Hz source handoff".to_owned(),
    }
}

fn build_analytical_screen(
    reseeded: Option<&ReseededBridgeEvidence>,
    candidate: &DirectBridgeCandidateV2,
    policy: &super::DirectBridgePolicyV2,
    vehicle: &super::VehicleInputV2,
) -> Result<Option<AnalyticalScreenEvidence>> {
    let Some(reseeded) = reseeded else {
        return Ok(None);
    };
    let Some(start_state) = reseeded.start_state else {
        return Ok(Some(AnalyticalScreenEvidence {
            reseeded_bridge_identity_reproduced: false,
            reseeded_bridge_classification: reseeded.classification,
            reseeded_bridge_reasons: reseeded.reasons.clone(),
            reseeded_bridge_margins: reseeded.margins,
            measurements: None,
            bridge_component_screens: None,
            source_clearance: reseeded.source_clearance.clone(),
            source_attitude_margin: reseeded.source_attitude_margin.map(|margin| MarginV2 {
                raw: margin.raw_margin,
                normalized: margin.normalized_margin,
            }),
            launch_boundary: reseeded.launch_boundary.clone(),
            aggregate_fuel_margin: reseeded.aggregate_fuel_margin.map(|margin| MarginV2 {
                raw: margin.raw_margin,
                normalized: margin.normalized_margin,
            }),
            aggregate_time_margin: reseeded.aggregate_time_margin.map(|margin| MarginV2 {
                raw: margin.raw_margin,
                normalized: margin.normalized_margin,
            }),
            analytical_eligible: false,
        }));
    };
    let handoff = candidate
        .source_handoff
        .ok_or_else(|| anyhow!("candidate {} has no source handoff", candidate.identity))?;
    let bridge = exact_discrete_bridge_v2(
        policy,
        vehicle,
        BridgeKindV2::Source,
        start_state,
        handoff.state,
        reseeded.tick_count,
    );
    let bridge = match bridge {
        Ok(bridge) => bridge,
        Err(_) => {
            return Ok(Some(AnalyticalScreenEvidence {
                reseeded_bridge_identity_reproduced: false,
                reseeded_bridge_classification: reseeded.classification,
                reseeded_bridge_reasons: reseeded.reasons.clone(),
                reseeded_bridge_margins: reseeded.margins,
                measurements: None,
                bridge_component_screens: None,
                source_clearance: reseeded.source_clearance.clone(),
                source_attitude_margin: reseeded.source_attitude_margin.map(|margin| MarginV2 {
                    raw: margin.raw_margin,
                    normalized: margin.normalized_margin,
                }),
                launch_boundary: reseeded.launch_boundary.clone(),
                aggregate_fuel_margin: reseeded.aggregate_fuel_margin.map(|margin| MarginV2 {
                    raw: margin.raw_margin,
                    normalized: margin.normalized_margin,
                }),
                aggregate_time_margin: reseeded.aggregate_time_margin.map(|margin| MarginV2 {
                    raw: margin.raw_margin,
                    normalized: margin.normalized_margin,
                }),
                analytical_eligible: false,
            }));
        }
    };
    let identity_matches = reseeded.identity.as_deref() == Some(bridge.identity.as_str());
    let source_clearance_passes = reseeded
        .source_clearance
        .as_ref()
        .is_some_and(|screen| screen.all_samples_passed);
    let source_attitude_passes = reseeded
        .source_attitude_margin
        .is_some_and(|margin| margin.passes_declared_screen);
    let launch_boundary_passes = reseeded
        .launch_boundary
        .as_ref()
        .and_then(|boundary| boundary.slew_margin)
        .is_some_and(|margin| margin.passes_declared_screen);
    let fuel_passes = reseeded
        .aggregate_fuel_margin
        .is_some_and(|margin| margin.passes_declared_screen);
    let time_passes = reseeded
        .aggregate_time_margin
        .is_some_and(|margin| margin.passes_declared_screen);
    let component_screens = component_screens(bridge.margins, policy);
    let component_passes = component_screens.coupled_thrust.passes_declared_screen
        && component_screens.minimum_throttle.passes_declared_screen
        && component_screens.bridge_endpoint.passes_declared_screen
        && component_screens.powered_slew.passes_declared_screen;
    let measurements = bridge_measurements(&bridge, policy, vehicle);
    let analytical_eligible = identity_matches
        && bridge.classification == CertificationV2::Certified
        && component_passes
        && source_clearance_passes
        && source_attitude_passes
        && launch_boundary_passes
        && fuel_passes
        && time_passes
        && reseeded.analytically_eligible;
    Ok(Some(AnalyticalScreenEvidence {
        reseeded_bridge_identity_reproduced: identity_matches,
        reseeded_bridge_classification: Some(bridge.classification),
        reseeded_bridge_reasons: bridge.reasons,
        reseeded_bridge_margins: Some(bridge.margins),
        measurements: Some(measurements),
        bridge_component_screens: Some(component_screens),
        source_clearance: reseeded.source_clearance.clone(),
        source_attitude_margin: reseeded.source_attitude_margin.map(|margin| MarginV2 {
            raw: margin.raw_margin,
            normalized: margin.normalized_margin,
        }),
        launch_boundary: reseeded.launch_boundary.clone(),
        aggregate_fuel_margin: reseeded.aggregate_fuel_margin.map(|margin| MarginV2 {
            raw: margin.raw_margin,
            normalized: margin.normalized_margin,
        }),
        aggregate_time_margin: reseeded.aggregate_time_margin.map(|margin| MarginV2 {
            raw: margin.raw_margin,
            normalized: margin.normalized_margin,
        }),
        analytical_eligible,
    }))
}

fn component_screens(
    margins: ComponentMarginsV2,
    policy: &super::DirectBridgePolicyV2,
) -> BridgeComponentScreensEvidence {
    BridgeComponentScreensEvidence {
        coupled_thrust: margin_screen(margins.coupled_thrust, policy),
        minimum_throttle: margin_screen(margins.minimum_throttle, policy),
        bridge_endpoint: margin_screen(margins.bridge_endpoint, policy),
        powered_slew: margin_screen(margins.powered_slew, policy),
    }
}

fn margin_screen(margin: MarginV2, policy: &super::DirectBridgePolicyV2) -> MarginScreenEvidence {
    MarginScreenEvidence {
        raw_margin: margin.raw,
        normalized_margin: margin.normalized,
        passes_declared_screen: margin.raw >= 0.0
            && margin.normalized + 1.0e-12 >= policy.declared_robustness_margin,
    }
}

fn build_first_tick_equivalence(
    launch_target_attitude_rad: f64,
    unlaunched_bridge: &AnalyticalBridgeV2,
    reseeded: Option<&ReseededBridgeEvidence>,
    launch_end_state: Option<&PlantStateEvidence>,
    candidate: &DirectBridgeCandidateV2,
    policy: &super::DirectBridgePolicyV2,
    vehicle: &super::VehicleInputV2,
) -> Option<FirstTickEquivalenceEvidence> {
    let target_from_unlaunched = unlaunched_bridge
        .samples
        .iter()
        .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
        .map(|sample| direction_angle(sample.thrust_acceleration_mps2))?;
    let launch_end_state = launch_end_state?;
    let reseeded = reseeded?;
    let reseeded_start = reseeded.start_state?;
    let reseeded_first_angle = reseeded
        .launch_boundary
        .as_ref()
        .and_then(|boundary| boundary.first_reseeded_powered_attitude_rad);
    let handoff = candidate.source_handoff?;
    let source_first_command = exact_discrete_bridge_v2(
        policy,
        vehicle,
        BridgeKindV2::Source,
        reseeded_start,
        handoff.state,
        reseeded.tick_count,
    )
    .ok()
    .and_then(|bridge| bridge_command_angles(&bridge).first().copied());
    Some(FirstTickEquivalenceEvidence {
        launch_target_attitude_rad,
        unlaunched_first_powered_attitude_rad: target_from_unlaunched,
        launch_target_matches_unlaunched_direction: angle_close(
            launch_target_attitude_rad,
            target_from_unlaunched,
        ),
        measured_launch_position_m: launch_end_state.position_m,
        measured_launch_velocity_mps: launch_end_state.velocity_mps,
        reseeded_bridge_start_state: reseeded_start,
        measured_launch_matches_reseeded_start: (launch_end_state.position_m
            - reseeded_start.position_m)
            .length()
            <= STATE_POSITION_TOLERANCE_M
            && (launch_end_state.velocity_mps - reseeded_start.velocity_mps).length()
                <= STATE_VELOCITY_TOLERANCE_MPS,
        reseeded_first_powered_attitude_rad: reseeded_first_angle,
        first_source_sample_command_attitude_rad: source_first_command,
    })
}

fn first_tick_matches_reseeded_command(
    first_tick: Option<&super::FirstSourceTickEvidence>,
    reseeded: Option<&ReseededBridgeEvidence>,
) -> bool {
    let Some(first_tick) = first_tick else {
        return false;
    };
    let Some(reseeded) = reseeded else {
        return false;
    };
    reseeded
        .launch_boundary
        .as_ref()
        .and_then(|boundary| boundary.first_reseeded_powered_attitude_rad)
        .is_some_and(|angle| {
            first_tick.commanded_throttle_frac > 0.0
                && angle_close(first_tick.desired_target_attitude_rad, angle)
        })
}

fn frozen_original_duration_parity(
    candidate_input: &super::flat_candidate_closure::CoupledThrustAuditCandidateInput,
    run: &LaunchFeasibilityCadenceRunEvidence,
    unlaunched: &UnlaunchedSourceBridgeEvidence,
) -> FrozenOriginalDurationParityEvidence {
    let (launch, reseeded_identity) = if let Some(frozen) = candidate_input.launch_cadences.first()
    {
        (
            Some(&frozen.launch),
            frozen
                .reseeded_bridge
                .as_ref()
                .and_then(|bridge| bridge.identity.clone()),
        )
    } else if let Some(frozen) = candidate_input.third_canary_cadences.first() {
        (
            Some(&frozen.launch),
            frozen
                .reseeded_bridge
                .as_ref()
                .and_then(|bridge| bridge.identity.clone()),
        )
    } else {
        (None, None)
    };
    let frozen_launch_log_available = launch.is_some();
    let launch_log_matches_frozen = launch.is_some_and(|launch| *launch == run.launch);
    let current_reseeded_identity = run
        .reseeded_bridge
        .as_ref()
        .and_then(|bridge| bridge.identity.clone());
    let reseeded_bridge_matches_frozen = current_reseeded_identity
        .as_deref()
        .zip(reseeded_identity.as_deref())
        .is_some_and(|(current, frozen)| current == frozen);
    FrozenOriginalDurationParityEvidence {
        frozen_launch_log_available,
        frozen_reseeded_bridge_identity: reseeded_identity,
        launch_log_matches_frozen,
        unlaunched_bridge_matches_frozen: unlaunched.matches_frozen_original_identity,
        reseeded_bridge_matches_frozen,
    }
}

fn bridge_measurements(
    bridge: &AnalyticalBridgeV2,
    policy: &super::DirectBridgePolicyV2,
    vehicle: &super::VehicleInputV2,
) -> BridgeMeasurementsEvidence {
    let derated_thrust_cap_mps2 =
        vehicle.max_thrust_n / (vehicle.dry_mass_kg + vehicle.max_fuel_kg) * policy.thrust_derate;
    let robust_thrust_cap_mps2 =
        derated_thrust_cap_mps2 * (1.0 - policy.declared_robustness_margin);
    let (peak_tick, maximum_requested_thrust_mps2) = bridge
        .samples
        .iter()
        .map(|sample| (sample.tick, sample.thrust_acceleration_mps2.length()))
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .unwrap_or((0, 0.0));
    let first_thrust = bridge
        .samples
        .iter()
        .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12);
    let last_thrust = bridge
        .samples
        .iter()
        .rev()
        .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12);
    let endpoint_peak = first_thrust
        .map(|sample| sample.thrust_acceleration_mps2.length())
        .unwrap_or(0.0)
        .max(
            last_thrust
                .map(|sample| sample.thrust_acceleration_mps2.length())
                .unwrap_or(0.0),
        );
    let materialized_peak_matches_endpoint_peak =
        (maximum_requested_thrust_mps2 - endpoint_peak).abs() <= PEAK_EQUALITY_TOLERANCE;
    let minimum_nonzero_throttle_frac = bridge
        .samples
        .iter()
        .filter(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
        .map(|sample| sample.throttle_fraction)
        .min_by(f64::total_cmp);
    let dt_s = bridge.duration_s / bridge.steps as f64;
    let mut previous = None;
    let mut maximum_powered_slew_radps = 0.0_f64;
    for sample in &bridge.samples {
        let thrust = sample.thrust_acceleration_mps2;
        if thrust.length() <= 1.0e-12 {
            continue;
        }
        if let Some((previous_tick, previous_thrust)) = previous {
            let elapsed = (sample.tick - previous_tick) as f64 * dt_s;
            let angle = angle_between(previous_thrust, thrust);
            maximum_powered_slew_radps = maximum_powered_slew_radps.max(angle / elapsed.max(dt_s));
        }
        previous = Some((sample.tick, thrust));
    }
    BridgeMeasurementsEvidence {
        derated_thrust_cap_mps2,
        robust_thrust_cap_mps2,
        maximum_requested_thrust_mps2,
        peak_tick_index: Some(peak_tick),
        peak_thrust_physics_step: Some(LAUNCH_TICKS + peak_tick + 1),
        materialized_sample_count: bridge.samples.len(),
        materialized_peak_matches_endpoint_peak,
        minimum_nonzero_throttle_frac,
        maximum_powered_slew_radps,
    }
}

fn source_handoff_failure_reason(run: &LaunchFeasibilityCadenceRunEvidence) -> String {
    if !run.launch.completed {
        "launch_did_not_complete".to_owned()
    } else if !run.launch.contact_free {
        "launch_contact_observed".to_owned()
    } else if !run.rollout.source_handoff_contact_free {
        "source_handoff_contact_or_state_miss".to_owned()
    } else if !run.rollout.source_handoff_reached {
        "source_handoff_tolerance_miss".to_owned()
    } else {
        "launch_or_reseed_prefix_mismatch".to_owned()
    }
}

fn bridge_command_angles(bridge: &AnalyticalBridgeV2) -> Vec<f64> {
    let mut next = None;
    let mut targets = vec![None; bridge.samples.len()];
    for index in (0..bridge.samples.len()).rev() {
        let thrust = bridge.samples[index].thrust_acceleration_mps2;
        if thrust.length() > 1.0e-12 {
            next = Some(direction_angle(thrust));
        }
        targets[index] = next;
    }
    let mut last = 0.0;
    targets
        .into_iter()
        .map(|target| {
            if let Some(target) = target {
                last = target;
            }
            last
        })
        .collect()
}

fn direction_angle(thrust: Vec2) -> f64 {
    thrust.x.atan2(thrust.y)
}

fn angle_between(left: Vec2, right: Vec2) -> f64 {
    left.x
        .mul_add(right.y, -left.y * right.x)
        .abs()
        .atan2(left.x.mul_add(right.x, left.y * right.y))
}

fn angle_close(left: f64, right: f64) -> bool {
    let delta = (left - right + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI)
        - std::f64::consts::PI;
    delta.abs() <= 1.0e-12
}

fn source_handoff_skip_reason(analytical_survivor: bool) -> Option<&'static str> {
    (!analytical_survivor).then_some("analytical_screen_failed")
}

fn full_flight_skip_reason(
    analytical_survivor_count: usize,
    source_handoff_survivor: bool,
) -> Option<&'static str> {
    if !source_handoff_survivor {
        Some(if analytical_survivor_count == 0 {
            "no_analytical_survivors_global_stop"
        } else {
            "source_handoff_gate_failed"
        })
    } else {
        None
    }
}

fn cadence_name(cadence: RolloutCadence) -> String {
    match cadence {
        RolloutCadence::DirectPerTick => "direct_per_tick_120_hz".to_owned(),
        RolloutCadence::ControllerCadence => "held_controller_60_hz".to_owned(),
    }
}

fn failed_rollout(error: &str) -> LaunchRolloutEvidence {
    LaunchRolloutEvidence {
        status: format!("full_flight_stage_failed:{error}"),
        source_handoff_reached: false,
        source_handoff_contact_free: false,
        source_handoff_position_error_m: None,
        source_handoff_velocity_error_mps: None,
        profile_tick_count: 0,
        physics_steps_advanced: 0,
        first_source_tick: None,
        first_divergence: None,
        max_position_error_m: 0.0,
        max_velocity_error_mps: 0.0,
        contact_transition_state_errors: Vec::new(),
        phase_handoff_errors: Vec::new(),
        minimum_touchdown_clearance_m: None,
        minimum_hull_clearance_m: None,
        contacts: Vec::new(),
        per_step: Vec::new(),
        profile_end: None,
        termination: PlantStateEvidence {
            sim_time_s: 0.0,
            physics_step: 0,
            position_m: Vec2::default(),
            velocity_mps: Vec2::default(),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
            fuel_kg: 0.0,
            fuel_used_kg: 0.0,
            physical_outcome: "not_run".to_owned(),
            mission_outcome: "not_run".to_owned(),
            end_reason: "full_flight_stage_failed".to_owned(),
        },
        saturation: CommandSaturationEvidence {
            commanded_update_count: 0,
            below_minimum_saturation_count: 0,
            above_maximum_saturation_count: 0,
            on_at_exact_minimum_count: 0,
            fuel_burn_capped_tick_count: 0,
            fuel_exhausted_tick_count: 0,
        },
    }
}

pub fn artifact_identity(artifact: &WaypointDirectSourceDurationCanaryArtifact) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        waypoint_direct_primitive_analytical::build_artifact as build_baseline,
        waypoint_direct_topology_sweep::build_artifact as build_sweep,
    };
    use pd_plan::conservative_ballistic_bridge::CertificationV2;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval crate is under repository root")
            .to_path_buf()
    }

    fn source_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time is after the epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pd-eval-source-duration-canary-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("unique test input directory is created");
        let root = repo_root();
        let baseline = build_baseline(&root).expect("current primitive baseline builds");
        let sweep = build_sweep(&root).expect("current topology sweep builds");
        fs::write(
            path.join("baseline.json"),
            serde_json::to_vec_pretty(&baseline).expect("baseline serializes"),
        )
        .expect("baseline input is written");
        fs::write(
            path.join("sweep.json"),
            serde_json::to_vec_pretty(&sweep).expect("sweep serializes"),
        )
        .expect("sweep input is written");
        path
    }

    #[test]
    fn family_is_finite_ordered_and_has_no_unbounded_search_surface() {
        let rows = EXPECTED_CANDIDATE_IDENTITIES
            .iter()
            .flat_map(|candidate| {
                DURATION_OFFSETS_TICKS
                    .iter()
                    .map(move |offset| (*candidate, *offset))
            })
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), MAX_PREDECLARED_VARIANTS);
        assert_eq!(
            rows.iter().map(|(_, offset)| *offset).collect::<Vec<_>>(),
            [-240, -180, -120, -60, 0]
                .into_iter()
                .chain([-240, -180, -120, -60, 0])
                .chain([-240, -180, -120, -60, 0])
                .collect::<Vec<_>>()
        );
        assert_eq!(DURATION_OFFSETS_TICKS[1] - DURATION_OFFSETS_TICKS[0], 60);
    }

    #[test]
    fn unlaunched_not_certified_bridge_still_defines_candidate_specific_launch_target() {
        let directory = source_directory();
        let root = repo_root();
        let prepared = super::super::prepare_inputs(
            &root,
            &directory.join("baseline.json"),
            &directory.join("sweep.json"),
        )
        .expect("sealed analytical inputs prepare without physics");
        let flat = prepared
            .cases
            .iter()
            .find(|case| case.evidence.id == "continuous_flat_r00")
            .expect("flat case is present");
        let candidate = flat
            .result
            .candidates
            .iter()
            .find(|candidate| candidate.identity == EXPECTED_CANDIDATE_IDENTITIES[1])
            .expect("shortest candidate is present");
        let original = candidate
            .source_bridge
            .as_ref()
            .expect("source bridge exists");
        let bridge = exact_discrete_bridge_v2(
            &prepared.baseline.evaluated_policy,
            &prepared.baseline.vehicle,
            BridgeKindV2::Source,
            original.start_state,
            candidate.source_handoff.expect("handoff exists").state,
            original.steps - 240,
        )
        .expect("predeclared unlaunched bridge materializes");
        let target = bridge
            .samples
            .iter()
            .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
            .map(|sample| direction_angle(sample.thrust_acceleration_mps2))
            .expect("bridge has powered direction");
        let first_powered_thrust = bridge
            .samples
            .iter()
            .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
            .expect("bridge has powered direction")
            .thrust_acceleration_mps2;
        assert_eq!(bridge.classification, CertificationV2::NotCertified);
        assert!(angle_close(
            target,
            first_powered_thrust.x.atan2(first_powered_thrust.y)
        ));
        fs::remove_dir_all(directory).expect("only the unique test directory is removed");
    }

    #[test]
    fn baseline_duration_launch_and_first_source_tick_match_analytical_state() {
        let directory = source_directory();
        let root = repo_root();
        let prepared = super::super::prepare_inputs(
            &root,
            &directory.join("baseline.json"),
            &directory.join("sweep.json"),
        )
        .expect("sealed analytical inputs prepare without physics");
        let flat = prepared
            .cases
            .iter()
            .find(|case| case.evidence.id == "continuous_flat_r00")
            .expect("flat case is present");
        let candidate = flat
            .result
            .candidates
            .iter()
            .find(|candidate| candidate.identity == EXPECTED_CANDIDATE_IDENTITIES[0])
            .expect("native candidate is present");
        let selected = flat
            .profile_candidates
            .iter()
            .find(|profile| profile.candidate_identity == candidate.identity)
            .expect("native frozen profile is present");
        let original = candidate
            .source_bridge
            .as_ref()
            .expect("source bridge exists");
        let unlaunched = exact_discrete_bridge_v2(
            &prepared.baseline.evaluated_policy,
            &prepared.baseline.vehicle,
            BridgeKindV2::Source,
            original.start_state,
            candidate.source_handoff.expect("handoff exists").state,
            original.steps,
        )
        .expect("baseline source bridge rebuilds");
        assert_eq!(unlaunched.identity, original.identity);
        let launch_target = unlaunched
            .samples
            .iter()
            .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
            .map(|sample| direction_angle(sample.thrust_acceleration_mps2))
            .expect("first powered direction exists");
        let analysis = run_source_duration_variant(SourceDurationRunRequest {
            scenario: &flat.scenario,
            probe: &flat.probe,
            selected,
            basis: candidate,
            policy: &prepared.baseline.evaluated_policy,
            vehicle: &prepared.baseline.vehicle,
            source_bridge_steps: original.steps,
            launch_tilt_attitude_rad: launch_target,
            cadence: RolloutCadence::DirectPerTick,
            stage: SourceDurationReplayStage::AnalyzeOnly,
        })
        .expect("analysis launch executes after bridge construction");
        let launch = analysis
            .launch
            .end_state
            .as_ref()
            .expect("launch completes");
        let reseeded_start = analysis
            .reseeded_bridge
            .as_ref()
            .and_then(|bridge| bridge.start_state)
            .expect("reseeded bridge captures measured start");
        assert!(angle_close(
            analysis.launch.commanded_tilt_attitude_rad,
            launch_target
        ));
        assert!(
            (launch.position_m - reseeded_start.position_m).length() <= STATE_POSITION_TOLERANCE_M
        );
        assert!(
            (launch.velocity_mps - reseeded_start.velocity_mps).length()
                <= STATE_VELOCITY_TOLERANCE_MPS
        );

        let source_gate = run_source_duration_variant(SourceDurationRunRequest {
            scenario: &flat.scenario,
            probe: &flat.probe,
            selected,
            basis: candidate,
            policy: &prepared.baseline.evaluated_policy,
            vehicle: &prepared.baseline.vehicle,
            source_bridge_steps: original.steps,
            launch_tilt_attitude_rad: launch_target,
            cadence: RolloutCadence::DirectPerTick,
            stage: SourceDurationReplayStage::SourceHandoff,
        })
        .expect("baseline source handoff executes");
        let first_tick = source_gate
            .rollout
            .first_source_tick
            .as_ref()
            .expect("first powered source step is retained");
        let reseeded = source_gate
            .reseeded_bridge
            .as_ref()
            .expect("bridge is retained");
        assert!(first_tick_matches_reseeded_command(
            Some(first_tick),
            Some(reseeded)
        ));
        fs::remove_dir_all(directory).expect("only the unique test directory is removed");
    }

    #[test]
    fn stage_skip_gates_do_not_advance_failed_rows() {
        assert_eq!(
            source_handoff_skip_reason(false),
            Some("analytical_screen_failed")
        );
        assert_eq!(source_handoff_skip_reason(true), None);
        assert_eq!(
            full_flight_skip_reason(0, false),
            Some("no_analytical_survivors_global_stop")
        );
        assert_eq!(
            full_flight_skip_reason(2, false),
            Some("source_handoff_gate_failed")
        );
        assert_eq!(full_flight_skip_reason(2, true), None);
    }

    #[test]
    fn tampered_coupled_input_gate_is_rejected_by_no_physics_preflight_binding() {
        let expected = sample_input_gate();
        let mut tampered = expected.clone();
        tampered.baseline_identity = "fnv1a64:tampered".to_owned();
        assert!(!audit_input_gate_matches(&tampered, &expected));
        assert!(validate_audit_input_gate(&tampered, &expected).is_err());
    }

    #[test]
    fn semantic_identity_is_path_independent_and_deterministic() {
        let input_gate = sample_input_gate();
        let mut artifact = WaypointDirectSourceDurationCanaryArtifact {
            schema_id: WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_SCHEMA_ID.to_owned(),
            schema_version: WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_SCHEMA_VERSION,
            characterization_id: WAYPOINT_DIRECT_SOURCE_DURATION_CANARY_ID.to_owned(),
            input_gate,
            coupled_audit_identity: EXPECTED_COUPLED_AUDIT_IDENTITY.to_owned(),
            protocol: protocol_evidence(),
            frozen_environment: FrozenSourceEnvironmentEvidence {
                policy_identity: "policy".to_owned(),
                vehicle_identity: "vehicle".to_owned(),
                flat_scenario_identity: "scenario".to_owned(),
                terrain_identity: "terrain".to_owned(),
                source_pad_center_x_m: 0.0,
                source_pad_surface_y_m: 0.0,
                source_pad_width_m: 10.0,
                contact_rule: "core".to_owned(),
                v2_footprint_sign_note: "separate".to_owned(),
            },
            family_proof: SourceDurationFamilyProofEvidence {
                maximum_predeclared_variant_count: MAX_PREDECLARED_VARIANTS,
                predeclared_variant_count: MAX_PREDECLARED_VARIANTS,
                recorded_variant_count: MAX_PREDECLARED_VARIANTS,
                recorded_failure_row_count: 0,
                omitted_failure_row_count: 0,
                duration_offsets_ticks: DURATION_OFFSETS_TICKS.to_vec(),
                duration_grid_seconds: 0.5,
                candidate_order: EXPECTED_CANDIDATE_IDENTITIES
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
                row_order_rule: "ordered".to_owned(),
                all_rows_recorded: true,
            },
            bases: Vec::new(),
            execution_status: "test".to_owned(),
            analytical_survivor_count: 0,
            source_handoff_survivor_count: 0,
            completed_full_flight_count: 0,
            strict_60_hz_handoff_miss_count: 0,
            stable_landing_count: 0,
            scope_non_claims: Vec::new(),
            identity: String::new(),
        };
        let first = artifact_identity(&artifact).expect("identity is deterministic");
        artifact.identity = first.clone();
        assert_eq!(artifact_identity(&artifact).unwrap(), first);
    }

    fn sample_input_gate() -> WaypointDirectCoupledThrustAuditInputGateEvidence {
        WaypointDirectCoupledThrustAuditInputGateEvidence {
            baseline_identity: "baseline".to_owned(),
            sweep_identity: "sweep".to_owned(),
            nominal_plant_identity: "nominal".to_owned(),
            nominal_input_manifest_identity: "manifest".to_owned(),
            launch_feasibility_identity: "launch".to_owned(),
            launch_feasibility_input_gate_identity: "launch_gate".to_owned(),
            contact_audit_identity: "contact".to_owned(),
            contact_audit_input_gate_identity: "contact_gate".to_owned(),
            flat_candidate_closure_input_gate:
                super::super::FlatCandidateClosureInputGateEvidence {
                    schema_id: "closure_gate".to_owned(),
                    schema_version: 1,
                    baseline_identity: "baseline".to_owned(),
                    sweep_identity: "sweep".to_owned(),
                    nominal_plant_identity: "nominal".to_owned(),
                    nominal_input_manifest_identity: "manifest".to_owned(),
                    launch_feasibility_identity: "launch".to_owned(),
                    launch_feasibility_input_gate_identity: "launch_gate".to_owned(),
                    contact_audit_identity: "contact".to_owned(),
                    contact_audit_input_gate_identity: "contact_gate".to_owned(),
                    flat_case_id: "flat".to_owned(),
                    flat_probe_identity: "probe".to_owned(),
                    flat_result_identity: "result".to_owned(),
                    original_native_candidate_identity: "native".to_owned(),
                    original_shortest_candidate_identity: "shortest".to_owned(),
                    third_candidate: super::super::FlatCandidateBindingEvidence {
                        candidate_identity: "third".to_owned(),
                        classification: "Certified".to_owned(),
                        selection_scope: "test".to_owned(),
                        source_handoff_arc_step: 0,
                        source_bridge_tick_count: 1,
                        coast_tick_count: 1,
                        terminal_bridge_tick_count: 1,
                        nominal_profile_tick_count: 3,
                    },
                    frozen_case_count: 6,
                    frozen_selected_profile_count: 9,
                    frozen_selection_role_count: 12,
                    frozen_cadence_row_count: 18,
                    identity: "closure_gate_id".to_owned(),
                },
            flat_canary_identity: "canary".to_owned(),
            flat_canary_input_gate_identity: "canary_gate".to_owned(),
            candidate_identities: EXPECTED_CANDIDATE_IDENTITIES
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            identity: "audit_gate".to_owned(),
        }
    }
}
