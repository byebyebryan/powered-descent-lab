//! Identity-bound evaluator-only closure canary for the third certified flat profile.
//!
//! The sealed nominal, launch-feasibility, and first-contact artifacts remain
//! immutable inputs. This lane materializes the extra profile only after all
//! inputs have passed the no-physics gate, then applies a live 120 Hz
//! source-handoff stop gate before either complete replay is permitted.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::RunContext;
use pd_plan::conservative_ballistic_bridge::CertificationV2;
use serde::{Deserialize, Serialize};

use super::{
    DirectBridgeCandidateV2, FirstContactEvidence, LaunchContactContractCadenceEvidence,
    LaunchContactContractCandidateEvidence, LaunchContactContractCaseEvidence,
    LaunchContactContractInputGateEvidence, LaunchEvidence, LaunchFeasibilityCadenceRunEvidence,
    LaunchFeasibilityInputGateEvidence, PlantStateEvidence, PreparedCase, PreparedInputs,
    PreparedProfileCandidate, ReplayTraceParityEvidence, ReseededBridgeEvidence,
    TrajectorySignGeometryScanEvidence, WaypointDirectLaunchContactContractArtifact,
    WaypointDirectLaunchFeasibilityArtifact, WaypointDirectNominalPlantArtifact,
    build_input_manifest, resolve_output_dir, stable_digest,
};

pub const WAYPOINT_DIRECT_FLAT_CANDIDATE_CLOSURE_ID: &str =
    "waypoint-direct-flat-candidate-closure";
pub const WAYPOINT_DIRECT_FLAT_CANDIDATE_CLOSURE_SCHEMA_ID: &str =
    "waypoint_direct_flat_candidate_closure_v1";
pub const WAYPOINT_DIRECT_FLAT_CANDIDATE_CLOSURE_SCHEMA_VERSION: u32 = 1;

const EXPECTED_BASELINE_IDENTITY: &str = "fnv1a64:d3fa6b24336f7c05";
const EXPECTED_SWEEP_IDENTITY: &str = "fnv1a64:1bcd5a3bd6c6da01";
const EXPECTED_NOMINAL_IDENTITY: &str = "fnv1a64:9e8cbc902ca11fbc";
const EXPECTED_LAUNCH_IDENTITY: &str = "fnv1a64:2c7b965ffdc809d6";
const EXPECTED_CONTACT_AUDIT_IDENTITY: &str = "fnv1a64:f5a6600e99cd283b";
const EXPECTED_FLAT_CASE_ID: &str = "continuous_flat_r00";
const EXPECTED_NATIVE_CANDIDATE_IDENTITY: &str = "fnv1a64:dee613017622ca16";
const EXPECTED_SHORTEST_CANDIDATE_IDENTITY: &str = "fnv1a64:4e6c0b23f9eb1b8f";
const THIRD_CANDIDATE_IDENTITY: &str = "fnv1a64:a18a98ad6e334014";
const STATE_POSITION_TOLERANCE_M: f64 = 1.0e-6;
const STATE_VELOCITY_TOLERANCE_MPS: f64 = 1.0e-6;

const EXPECTED_COUPLED_THRUST_FLAT_CANARY_IDENTITY: &str = "fnv1a64:f3fba9290c9073be";

#[derive(Clone, Debug)]
pub(super) struct CoupledThrustAuditCandidateInput {
    pub(super) candidate: DirectBridgeCandidateV2,
    pub(super) selected_profile: PreparedProfileCandidate,
    pub(super) launch_cadences: Vec<LaunchFeasibilityCadenceRunEvidence>,
    pub(super) contact_cadences: Vec<LaunchContactContractCadenceEvidence>,
    pub(super) third_canary_cadences: Vec<FlatCandidateCadenceEvidence>,
}

#[derive(Clone, Debug)]
pub(super) struct CoupledThrustAuditValidatedInputs {
    pub(super) input_gate: FlatCandidateClosureInputGateEvidence,
    pub(super) coupled_input_gate: Option<super::WaypointDirectCoupledThrustAuditInputGateEvidence>,
    pub(super) flat_canary_identity: String,
    pub(super) flat_canary_input_gate_identity: String,
    pub(super) policy: super::DirectBridgePolicyV2,
    pub(super) vehicle: super::VehicleInputV2,
    pub(super) flat_case: PreparedCase,
    pub(super) candidates: Vec<CoupledThrustAuditCandidateInput>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectFlatCandidateClosurePaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectFlatCandidateClosureRun {
    pub artifact: WaypointDirectFlatCandidateClosureArtifact,
    pub paths: WaypointDirectFlatCandidateClosurePaths,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatCandidateClosureInputGateEvidence {
    pub schema_id: String,
    pub schema_version: u32,
    pub baseline_identity: String,
    pub sweep_identity: String,
    pub nominal_plant_identity: String,
    pub nominal_input_manifest_identity: String,
    pub launch_feasibility_identity: String,
    pub launch_feasibility_input_gate_identity: String,
    pub contact_audit_identity: String,
    pub contact_audit_input_gate_identity: String,
    pub flat_case_id: String,
    pub flat_probe_identity: String,
    pub flat_result_identity: String,
    pub original_native_candidate_identity: String,
    pub original_shortest_candidate_identity: String,
    pub third_candidate: FlatCandidateBindingEvidence,
    pub frozen_case_count: u32,
    pub frozen_selected_profile_count: u32,
    pub frozen_selection_role_count: u32,
    pub frozen_cadence_row_count: u32,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatCandidateBindingEvidence {
    pub candidate_identity: String,
    pub classification: String,
    pub selection_scope: String,
    pub source_handoff_arc_step: u64,
    pub source_bridge_tick_count: u64,
    pub coast_tick_count: u64,
    pub terminal_bridge_tick_count: u64,
    pub nominal_profile_tick_count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectFlatCandidateClosureArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub input_gate: FlatCandidateClosureInputGateEvidence,
    pub protocol: FlatCandidateClosureProtocolEvidence,
    pub execution_status: String,
    pub source_handoff_gate: FlatCandidateSourceHandoffGateEvidence,
    pub frozen_trace_sign_scans: Vec<FrozenTraceSignScanEvidence>,
    pub third_candidate_runs: Vec<FlatCandidateCadenceEvidence>,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatCandidateClosureProtocolEvidence {
    pub candidate_materialization_rule: String,
    pub launch_rule: String,
    pub source_handoff_gate_rule: String,
    pub complete_cadence_rule: String,
    pub source_gate_prefix_parity_rule: String,
    pub trajectory_sign_scan_rule: String,
    pub contact_rule: String,
    pub state_parity_position_tolerance_m: f64,
    pub state_parity_velocity_tolerance_mps: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatCandidateSourceHandoffGateEvidence {
    pub cadence: String,
    pub passed: bool,
    pub stop_reason: Option<String>,
    pub launch: LaunchEvidence,
    pub reseeded_bridge: Option<ReseededBridgeEvidence>,
    pub source_handoff_reached: bool,
    pub source_handoff_contact_free: bool,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub state_at_stop: PlantStateEvidence,
    pub physics_ticks_after_launch: u64,
    pub replay_trace_parity: ReplayTraceParityEvidence,
    pub sign_geometry_scan: TrajectorySignGeometryScanEvidence,
    pub completed_direct_prefix_matched: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatCandidateCadenceEvidence {
    pub cadence: String,
    pub status: String,
    pub launch: LaunchEvidence,
    pub reseeded_bridge: Option<ReseededBridgeEvidence>,
    pub source_handoff_reached: bool,
    pub source_handoff_contact_free: bool,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub strict_source_handoff_within_tolerance: bool,
    pub first_contact: Option<FirstContactEvidence>,
    pub replay_trace_parity: ReplayTraceParityEvidence,
    pub profile_end: Option<PlantStateEvidence>,
    pub post_terminal_state: Option<PlantStateEvidence>,
    pub authoritative_outcome: PlantStateEvidence,
    pub sign_geometry_scan: TrajectorySignGeometryScanEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenTraceSignScanEvidence {
    pub case_id: String,
    pub candidate_identity: String,
    pub cadence: String,
    pub replay_trace_parity_passed: bool,
    pub first_contact_step: Option<u64>,
    pub sign_geometry_scan: TrajectorySignGeometryScanEvidence,
}

/// Validate and identity-bind every frozen input without constructing a
/// `SimulationState` or advancing physics.
pub fn validate_waypoint_direct_flat_candidate_closure_inputs(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    nominal_summary_path: &Path,
    launch_summary_path: &Path,
    contact_audit_summary_path: &Path,
) -> Result<FlatCandidateClosureInputGateEvidence> {
    let (prepared, nominal, launch, launch_gate, contact_audit, contact_gate, selected) =
        prepare_canary_inputs(
            repo_root,
            baseline_summary_path,
            sweep_summary_path,
            nominal_summary_path,
            launch_summary_path,
            contact_audit_summary_path,
        )?;
    build_input_gate(
        &prepared,
        &nominal,
        &launch,
        &launch_gate,
        &contact_audit,
        &contact_gate,
        &selected,
    )
}

/// Run the additive third-candidate canary into a new identity-bound artifact.
pub fn run_waypoint_direct_flat_candidate_closure(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    nominal_summary_path: &Path,
    launch_summary_path: &Path,
    contact_audit_summary_path: &Path,
    output_dir: &Path,
) -> Result<WaypointDirectFlatCandidateClosureRun> {
    let (prepared, nominal, launch, launch_gate, contact_audit, contact_gate, selected) =
        prepare_canary_inputs(
            repo_root,
            baseline_summary_path,
            sweep_summary_path,
            nominal_summary_path,
            launch_summary_path,
            contact_audit_summary_path,
        )?;
    let input_gate = build_input_gate(
        &prepared,
        &nominal,
        &launch,
        &launch_gate,
        &contact_audit,
        &contact_gate,
        &selected,
    )?;

    // All sealed identities, manifests, gate bindings, and the isolated third
    // candidate profile are validated before the first replay creates state.
    let flat_case = flat_case(&prepared)?;
    let execution = super::launch_feasibility::run_flat_third_candidate_with_source_gate(
        flat_case,
        &selected,
        &prepared.baseline.evaluated_policy,
        &prepared.baseline.vehicle,
    )?;
    let source_gate_context =
        RunContext::from_scenario(&flat_case.scenario).map_err(anyhow::Error::msg)?;
    let source_gate_replay = super::launch_contact_contract::replay_logged_cadence(
        &source_gate_context,
        &execution.source_gate_run,
    )?;
    if !source_gate_replay.trace.passed {
        bail!("third-candidate 120 Hz source-gate replay failed paired contact parity");
    }
    let source_handoff_gate = source_handoff_gate_evidence(
        &execution,
        source_gate_replay.trace,
        source_gate_replay.sign_geometry_scan,
    );

    let mut third_candidate_runs = Vec::new();
    if let Some(candidate) = &execution.candidate {
        for cadence_run in &candidate.cadence_runs {
            let replay = super::launch_contact_contract::replay_logged_cadence(
                &source_gate_context,
                cadence_run,
            )?;
            if !replay.trace.passed || !replay.sign_geometry_scan.reached_original_terminal {
                bail!(
                    "third-candidate {} replay failed parity or did not reach its original terminal",
                    cadence_run.cadence
                );
            }
            third_candidate_runs.push(candidate_cadence_evidence(cadence_run, replay));
        }
        if third_candidate_runs.len() != 2
            || third_candidate_runs[0].cadence != "direct_per_tick_120_hz"
            || third_candidate_runs[1].cadence != "held_controller_60_hz"
        {
            bail!("third-candidate execution did not produce the fixed 120/60 Hz cadence pair");
        }
    }
    let frozen_trace_sign_scans = scan_frozen_launch_traces(&prepared, &launch, &contact_audit)?;

    let mut artifact = WaypointDirectFlatCandidateClosureArtifact {
        schema_id: WAYPOINT_DIRECT_FLAT_CANDIDATE_CLOSURE_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_FLAT_CANDIDATE_CLOSURE_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_FLAT_CANDIDATE_CLOSURE_ID.to_owned(),
        input_gate,
        protocol: FlatCandidateClosureProtocolEvidence {
            candidate_materialization_rule: "reevaluate the sealed flat probe and materialize only fnv1a64:a18a98ad6e334014 into an isolated additive profile; preserve the two existing selected-role profiles and all sealed artifacts".to_owned(),
            launch_rule: "start from the same frozen source-pad-rest state; apply 60 full-throttle upright physics ticks and 12 full-throttle ticks targeting the third candidate's first powered source attitude".to_owned(),
            source_handoff_gate_rule: "run only the third candidate's 120 Hz launch and reseeded source bridge through the original source handoff; require no contact and Euclidean position and velocity errors at or below 1e-6 before advancing its post-handoff mission or starting 60 Hz".to_owned(),
            complete_cadence_rule: "after the source gate passes, complete the 120 Hz per-tick lane and then independently replay the 60 Hz controller lane with commands held for two physics ticks; both keep the original source handoff, source tick count, coast, terminal bridge, and scenario".to_owned(),
            source_gate_prefix_parity_rule: "the gate and completed 120 Hz lane share the same launch and source-reseed execution; compare launch evidence, reseed evidence, every source tick command/state record, and source-handoff errors before starting 60 Hz".to_owned(),
            trajectory_sign_scan_rule: "replay logged commands through the original state sequence and compare the core footprint geometry with the opposite V2 footprint sign at every post-physics preterminal sample through the original terminal; report only geometry, pad, and predicate differences, with no alternative post-terminal outcome".to_owned(),
            contact_rule: "replay each logged cadence in paired ordinary and neutral contact paths and report the first preterminal contact, local predicates, event parity, mission outcome, and strict source-handoff errors; do not change core dynamics or classifier".to_owned(),
            state_parity_position_tolerance_m: STATE_POSITION_TOLERANCE_M,
            state_parity_velocity_tolerance_mps: STATE_VELOCITY_TOLERANCE_MPS,
        },
        execution_status: if execution.source_gate_passed {
            "completed_both_cadences".to_owned()
        } else {
            "stopped_at_120_hz_source_handoff_gate".to_owned()
        },
        source_handoff_gate,
        frozen_trace_sign_scans,
        third_candidate_runs,
        scope_non_claims: vec![
            "The third profile remains an evaluator-only replay and is not added to the sealed nominal or launch artifacts.".to_owned(),
            "A failed source-handoff gate leaves the third candidate's post-handoff 120 Hz mission and 60 Hz lane unrun.".to_owned(),
            "Reseeded analytical eligibility and its reserves are separate from plant handoff, contact, and landing outcomes.".to_owned(),
            "The opposite-sign scan reuses each original command/state trajectory only through its original terminal; it does not establish an alternate-sign flight or landing outcome.".to_owned(),
            "No V2 certificate, candidate ranking, planner/controller behavior, F6 behavior, contact threshold, or default is changed.".to_owned(),
        ],
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;

    let output_dir = resolve_output_dir(repo_root, output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create flat candidate closure output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    if summary_path.exists() {
        bail!(
            "flat candidate closure refuses to overwrite existing summary {}",
            summary_path.display()
        );
    }
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&summary_path, &summary_bytes).with_context(|| {
        format!(
            "failed to write flat candidate closure summary {}",
            summary_path.display()
        )
    })?;
    let reloaded: WaypointDirectFlatCandidateClosureArtifact =
        serde_json::from_slice(&summary_bytes)
            .context("failed to reload flat candidate closure summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes {
        bail!("flat candidate closure summary is not byte-stable after reload");
    }
    if artifact_identity(&reloaded)? != reloaded.identity {
        bail!("flat candidate closure semantic identity failed round-trip check");
    }

    Ok(WaypointDirectFlatCandidateClosureRun {
        artifact: reloaded,
        paths: WaypointDirectFlatCandidateClosurePaths {
            output_dir,
            summary_path,
        },
    })
}

fn prepare_canary_inputs(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    nominal_summary_path: &Path,
    launch_summary_path: &Path,
    contact_audit_summary_path: &Path,
) -> Result<(
    PreparedInputs,
    WaypointDirectNominalPlantArtifact,
    WaypointDirectLaunchFeasibilityArtifact,
    LaunchFeasibilityInputGateEvidence,
    WaypointDirectLaunchContactContractArtifact,
    LaunchContactContractInputGateEvidence,
    PreparedProfileCandidate,
)> {
    let (prepared, nominal, launch, launch_gate) =
        super::launch_contact_contract::load_and_validate_inputs(
            repo_root,
            baseline_summary_path,
            sweep_summary_path,
            nominal_summary_path,
            launch_summary_path,
        )?;
    if prepared.baseline.identity != EXPECTED_BASELINE_IDENTITY
        || prepared.sweep.identity != EXPECTED_SWEEP_IDENTITY
        || nominal.identity != EXPECTED_NOMINAL_IDENTITY
        || launch.identity != EXPECTED_LAUNCH_IDENTITY
    {
        bail!("flat candidate closure source identities differ from the frozen protocol");
    }
    let manifest = build_input_manifest(&prepared)?;
    if manifest.identity != nominal.input_manifest_identity {
        bail!("rebuilt nominal input manifest differs from the frozen nominal artifact");
    }
    let contact_gate = super::launch_contact_contract::build_contact_contract_input_gate(
        &prepared,
        &nominal,
        &launch,
        &launch_gate,
    )?;
    let contact_audit: WaypointDirectLaunchContactContractArtifact =
        super::read_summary(contact_audit_summary_path)?;
    validate_contact_audit(
        &contact_audit,
        &contact_gate,
        &prepared,
        &nominal,
        &launch,
        &launch_gate,
    )?;
    let selected = materialize_third_candidate(&prepared)?;
    Ok((
        prepared,
        nominal,
        launch,
        launch_gate,
        contact_audit,
        contact_gate,
        selected,
    ))
}

/// Reuse the complete no-physics flat-candidate input gate and project only
/// the three frozen candidates plus their already stored launch/contact rows.
pub(super) fn load_coupled_thrust_audit_inputs(
    repo_root: &Path,
    input_paths: &super::WaypointDirectCoupledThrustAuditInputPaths,
) -> Result<CoupledThrustAuditValidatedInputs> {
    let (prepared, nominal, launch, launch_gate, contact_audit, contact_gate, selected) =
        prepare_canary_inputs(
            repo_root,
            &input_paths.baseline_summary,
            &input_paths.sweep_summary,
            &input_paths.nominal_summary,
            &input_paths.launch_summary,
            &input_paths.contact_audit_summary,
        )?;
    let input_gate = build_input_gate(
        &prepared,
        &nominal,
        &launch,
        &launch_gate,
        &contact_audit,
        &contact_gate,
        &selected,
    )?;
    let flat_canary: WaypointDirectFlatCandidateClosureArtifact =
        super::read_summary(&input_paths.flat_canary_summary)?;
    let computed_canary_identity = artifact_identity(&flat_canary)?;
    if computed_canary_identity != flat_canary.identity
        || flat_canary.identity != EXPECTED_COUPLED_THRUST_FLAT_CANARY_IDENTITY
        || flat_canary.schema_id != WAYPOINT_DIRECT_FLAT_CANDIDATE_CLOSURE_SCHEMA_ID
        || flat_canary.schema_version != WAYPOINT_DIRECT_FLAT_CANDIDATE_CLOSURE_SCHEMA_VERSION
        || flat_canary.characterization_id != WAYPOINT_DIRECT_FLAT_CANDIDATE_CLOSURE_ID
        || flat_canary.input_gate != input_gate
        || flat_canary.execution_status != "completed_both_cadences"
        || !flat_canary.source_handoff_gate.passed
    {
        bail!("flat candidate canary identity or rebuilt input-gate binding changed");
    }

    let case = flat_case(&prepared)?;
    let certified = case
        .result
        .candidates
        .iter()
        .filter(|candidate| candidate.classification == CertificationV2::Certified)
        .collect::<Vec<_>>();
    if case.result.identity != input_gate.flat_result_identity
        || certified.len() != 3
        || certified
            .iter()
            .map(|candidate| candidate.identity.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            != [
                EXPECTED_NATIVE_CANDIDATE_IDENTITY,
                EXPECTED_SHORTEST_CANDIDATE_IDENTITY,
                THIRD_CANDIDATE_IDENTITY,
            ]
            .into_iter()
            .collect()
    {
        bail!(
            "reevaluated flat probe does not contain exactly the three pinned Certified candidates"
        );
    }

    let launch_case = launch
        .cases
        .iter()
        .find(|row| row.input.id == EXPECTED_FLAT_CASE_ID)
        .ok_or_else(|| anyhow!("frozen launch artifact is missing the flat case"))?;
    let contact_case = contact_audit_case(&contact_audit, EXPECTED_FLAT_CASE_ID)?;
    let mut candidates = Vec::with_capacity(3);
    for candidate_identity in [
        EXPECTED_NATIVE_CANDIDATE_IDENTITY,
        EXPECTED_SHORTEST_CANDIDATE_IDENTITY,
        THIRD_CANDIDATE_IDENTITY,
    ] {
        let candidate = certified
            .iter()
            .find(|candidate| candidate.identity == candidate_identity)
            .ok_or_else(|| anyhow!("pinned flat candidate {candidate_identity} is missing"))?;
        if candidate.source_bridge.is_none() || candidate.source_handoff.is_none() {
            bail!("pinned flat candidate {candidate_identity} has no source bridge or handoff");
        }
        if candidate_identity == THIRD_CANDIDATE_IDENTITY {
            let third_canary_cadences = flat_canary.third_candidate_runs.clone();
            if third_canary_cadences.len() != 2
                || third_canary_cadences[0].cadence != "direct_per_tick_120_hz"
                || third_canary_cadences[1].cadence != "held_controller_60_hz"
                || !third_canary_cadences[0].source_handoff_reached
                || !third_canary_cadences[0].source_handoff_contact_free
                || !third_canary_cadences[0].strict_source_handoff_within_tolerance
                || third_canary_cadences
                    .iter()
                    .any(|row| row.reseeded_bridge.is_none() || !row.replay_trace_parity.passed)
            {
                bail!("pinned third-candidate canary is missing its two validated cadence rows");
            }
            candidates.push(CoupledThrustAuditCandidateInput {
                candidate: (**candidate).clone(),
                selected_profile: selected.clone(),
                launch_cadences: Vec::new(),
                contact_cadences: Vec::new(),
                third_canary_cadences,
            });
            continue;
        }

        let launch_candidate = launch_case
            .candidate_profiles
            .iter()
            .find(|row| row.basis_candidate_identity == candidate_identity)
            .ok_or_else(|| {
                anyhow!("frozen launch artifact is missing candidate {candidate_identity}")
            })?;
        if launch_candidate.cadence_runs.len() != 2
            || launch_candidate.cadence_runs[0].cadence != "direct_per_tick_120_hz"
            || launch_candidate.cadence_runs[1].cadence != "held_controller_60_hz"
        {
            bail!(
                "frozen launch artifact is missing the fixed cadence pair for {candidate_identity}"
            );
        }
        let contact_candidate = contact_audit_candidate(contact_case, candidate_identity)?;
        if !contact_candidate.launch_candidate_available
            || contact_candidate.launch_candidate_identity.as_deref()
                != Some(launch_candidate.identity.as_str())
            || contact_candidate.cadence_runs.len() != 2
        {
            bail!("frozen contact audit does not bind both launch rows for {candidate_identity}");
        }
        let mut contact_cadences = Vec::with_capacity(2);
        for (launch_row, cadence) in launch_candidate
            .cadence_runs
            .iter()
            .zip(["direct_per_tick_120_hz", "held_controller_60_hz"])
        {
            let contact_row = contact_audit_cadence(contact_candidate, cadence)?;
            if contact_row.trace.as_ref().is_none_or(|trace| !trace.passed)
                || contact_row.first_contact.is_none()
                || contact_row.source_handoff_position_error_m
                    != launch_row.rollout.source_handoff_position_error_m
                || contact_row.source_handoff_velocity_error_mps
                    != launch_row.rollout.source_handoff_velocity_error_mps
            {
                bail!(
                    "frozen contact audit does not match launch row {candidate_identity} / {cadence}"
                );
            }
            contact_cadences.push(contact_row.clone());
        }
        candidates.push(CoupledThrustAuditCandidateInput {
            candidate: (**candidate).clone(),
            selected_profile: case
                .profile_candidates
                .iter()
                .find(|profile| profile.candidate_identity == candidate_identity)
                .cloned()
                .ok_or_else(|| anyhow!("frozen flat profile {candidate_identity} is missing"))?,
            launch_cadences: launch_candidate.cadence_runs.clone(),
            contact_cadences,
            third_canary_cadences: Vec::new(),
        });
    }

    let mut validated_inputs = CoupledThrustAuditValidatedInputs {
        input_gate,
        coupled_input_gate: None,
        flat_canary_identity: flat_canary.identity,
        flat_canary_input_gate_identity: flat_canary.input_gate.identity,
        policy: prepared.baseline.evaluated_policy.clone(),
        vehicle: prepared.baseline.vehicle.clone(),
        flat_case: case.clone(),
        candidates,
    };
    validated_inputs.coupled_input_gate = Some(super::coupled_thrust_audit::build_input_gate(
        &validated_inputs,
    )?);
    Ok(validated_inputs)
}

fn validate_contact_audit(
    audit: &WaypointDirectLaunchContactContractArtifact,
    contact_gate: &LaunchContactContractInputGateEvidence,
    prepared: &PreparedInputs,
    nominal: &WaypointDirectNominalPlantArtifact,
    launch: &WaypointDirectLaunchFeasibilityArtifact,
    launch_gate: &LaunchFeasibilityInputGateEvidence,
) -> Result<()> {
    let computed_identity = super::launch_contact_contract::artifact_identity(audit)?;
    if computed_identity != audit.identity || audit.identity != EXPECTED_CONTACT_AUDIT_IDENTITY {
        bail!("first-contact audit semantic identity is not the frozen closure input");
    }
    if audit.schema_id != super::WAYPOINT_DIRECT_LAUNCH_CONTACT_CONTRACT_SCHEMA_ID
        || audit.schema_version != super::WAYPOINT_DIRECT_LAUNCH_CONTACT_CONTRACT_SCHEMA_VERSION
        || audit.characterization_id != super::WAYPOINT_DIRECT_LAUNCH_CONTACT_CONTRACT_ID
        || audit.baseline_identity != prepared.baseline.identity
        || audit.sweep_identity != prepared.sweep.identity
        || audit.nominal_plant_identity != nominal.identity
        || audit.nominal_input_manifest_identity != nominal.input_manifest_identity
        || audit.launch_feasibility_identity != launch.identity
        || audit.launch_feasibility_input_gate_identity != launch_gate.identity
        || audit.input_gate_identity != contact_gate.identity
        || contact_gate.launch_input_gate != *launch_gate
    {
        bail!("first-contact audit bindings differ from the rebuilt frozen inputs");
    }
    let case_count = prepared.cases.len();
    let profile_count: usize = launch
        .cases
        .iter()
        .map(|case| case.candidate_profiles.len())
        .sum();
    let cadence_count: usize = launch
        .cases
        .iter()
        .flat_map(|case| &case.candidate_profiles)
        .map(|candidate| candidate.cadence_runs.len())
        .sum();
    if case_count != 6
        || launch.cases.len() != 6
        || audit.cases.len() != 6
        || profile_count != 9
        || cadence_count != 18
        || contact_gate.existing_cadence_row_count != 18
    {
        bail!(
            "flat candidate closure requires six cases, nine profiles, and eighteen frozen traces"
        );
    }
    Ok(())
}

fn materialize_third_candidate(prepared: &PreparedInputs) -> Result<PreparedProfileCandidate> {
    let case = flat_case(prepared)?;
    if case.evidence.native_v2_candidate_identity != EXPECTED_NATIVE_CANDIDATE_IDENTITY
        || case.evidence.research_shortest_certified_candidate_identity
            != EXPECTED_SHORTEST_CANDIDATE_IDENTITY
    {
        bail!("the two previously flown flat profile identities changed");
    }
    if case.profile_candidates.len() != 2
        || case
            .profile_candidates
            .iter()
            .any(|selected| selected.candidate_identity == THIRD_CANDIDATE_IDENTITY)
    {
        bail!("third candidate must remain outside the sealed selected-profile set");
    }
    let certified_candidates = case
        .result
        .candidates
        .iter()
        .filter(|candidate| candidate.classification == CertificationV2::Certified)
        .collect::<Vec<_>>();
    let matches = certified_candidates
        .iter()
        .filter(|candidate| candidate.identity == THIRD_CANDIDATE_IDENTITY)
        .collect::<Vec<_>>();
    if certified_candidates.len() != 3 || matches.len() != 1 {
        bail!(
            "reevaluated flat result does not contain exactly the pinned third certified candidate"
        );
    }
    let candidate = matches[0];
    let profile = super::materialize_profile(
        &case.result,
        candidate,
        &case.probe,
        &prepared.baseline.evaluated_policy,
        &prepared.baseline.vehicle,
    )?;
    Ok(PreparedProfileCandidate {
        candidate_identity: candidate.identity.clone(),
        selected_roles: vec!["flat_third_certified_diagnostic".to_owned()],
        profile,
    })
}

fn build_input_gate(
    prepared: &PreparedInputs,
    nominal: &WaypointDirectNominalPlantArtifact,
    launch: &WaypointDirectLaunchFeasibilityArtifact,
    launch_gate: &LaunchFeasibilityInputGateEvidence,
    contact_audit: &WaypointDirectLaunchContactContractArtifact,
    contact_gate: &LaunchContactContractInputGateEvidence,
    selected: &PreparedProfileCandidate,
) -> Result<FlatCandidateClosureInputGateEvidence> {
    let case = flat_case(prepared)?;
    let candidate = case
        .result
        .candidates
        .iter()
        .find(|candidate| candidate.identity == selected.candidate_identity)
        .ok_or_else(|| anyhow!("third candidate is missing from reevaluated flat result"))?;
    let source_handoff = candidate
        .source_handoff
        .ok_or_else(|| anyhow!("third candidate has no source handoff"))?;
    let source_bridge = candidate
        .source_bridge
        .as_ref()
        .ok_or_else(|| anyhow!("third candidate has no source bridge"))?;
    let coast = candidate
        .selected_coast
        .as_ref()
        .ok_or_else(|| anyhow!("third candidate has no selected coast"))?;
    let terminal = candidate
        .terminal_bridge
        .as_ref()
        .ok_or_else(|| anyhow!("third candidate has no terminal bridge"))?;
    let third_candidate = FlatCandidateBindingEvidence {
        candidate_identity: candidate.identity.clone(),
        classification: "Certified".to_owned(),
        selection_scope: "one additive diagnostic profile; excluded from sealed selected roles"
            .to_owned(),
        source_handoff_arc_step: source_handoff.arc_step,
        source_bridge_tick_count: source_bridge.steps,
        coast_tick_count: (coast.duration_s
            * f64::from(prepared.baseline.evaluated_policy.physics_hz)
            + 0.5) as u64,
        terminal_bridge_tick_count: terminal.steps,
        nominal_profile_tick_count: selected.profile.accounting.nominal_profile_tick_count,
    };
    let mut gate = FlatCandidateClosureInputGateEvidence {
        schema_id: "waypoint_direct_flat_candidate_closure_input_gate_v1".to_owned(),
        schema_version: 1,
        baseline_identity: prepared.baseline.identity.clone(),
        sweep_identity: prepared.sweep.identity.clone(),
        nominal_plant_identity: nominal.identity.clone(),
        nominal_input_manifest_identity: nominal.input_manifest_identity.clone(),
        launch_feasibility_identity: launch.identity.clone(),
        launch_feasibility_input_gate_identity: launch_gate.identity.clone(),
        contact_audit_identity: contact_audit.identity.clone(),
        contact_audit_input_gate_identity: contact_gate.identity.clone(),
        flat_case_id: case.evidence.id.clone(),
        flat_probe_identity: case.evidence.probe_identity.clone(),
        flat_result_identity: case.result.identity.clone(),
        original_native_candidate_identity: case.evidence.native_v2_candidate_identity.clone(),
        original_shortest_candidate_identity: case
            .evidence
            .research_shortest_certified_candidate_identity
            .clone(),
        third_candidate,
        frozen_case_count: 6,
        frozen_selected_profile_count: 9,
        frozen_selection_role_count: 12,
        frozen_cadence_row_count: 18,
        identity: String::new(),
    };
    gate.identity = input_gate_identity(&gate)?;
    let bytes = serde_json::to_vec(&gate)?;
    let reloaded: FlatCandidateClosureInputGateEvidence = serde_json::from_slice(&bytes)?;
    if reloaded != gate || input_gate_identity(&reloaded)? != reloaded.identity {
        bail!("flat candidate closure input gate failed identity round-trip");
    }
    Ok(reloaded)
}

fn scan_frozen_launch_traces(
    prepared: &PreparedInputs,
    launch: &WaypointDirectLaunchFeasibilityArtifact,
    contact_audit: &WaypointDirectLaunchContactContractArtifact,
) -> Result<Vec<FrozenTraceSignScanEvidence>> {
    let mut scans = Vec::with_capacity(18);
    for prepared_case in &prepared.cases {
        let launch_case = launch
            .cases
            .iter()
            .find(|case| case.input.id == prepared_case.evidence.id)
            .ok_or_else(|| {
                anyhow!(
                    "launch artifact is missing case {}",
                    prepared_case.evidence.id
                )
            })?;
        let audit_case = contact_audit_case(contact_audit, &prepared_case.evidence.id)?;
        let context =
            RunContext::from_scenario(&prepared_case.scenario).map_err(anyhow::Error::msg)?;
        for launch_candidate in &launch_case.candidate_profiles {
            let audit_candidate =
                contact_audit_candidate(audit_case, &launch_candidate.basis_candidate_identity)?;
            for run in &launch_candidate.cadence_runs {
                let audit_row = contact_audit_cadence(audit_candidate, &run.cadence)?;
                let replay = super::launch_contact_contract::replay_logged_cadence(&context, run)?;
                if audit_row.trace.as_ref() != Some(&replay.trace)
                    || audit_row.first_contact != replay.first_contact
                    || !replay.trace.passed
                    || !replay.sign_geometry_scan.reached_original_terminal
                {
                    bail!(
                        "replayed frozen trace {} / {} differs from its pinned first-contact audit",
                        launch_candidate.basis_candidate_identity,
                        run.cadence
                    );
                }
                scans.push(FrozenTraceSignScanEvidence {
                    case_id: prepared_case.evidence.id.clone(),
                    candidate_identity: launch_candidate.basis_candidate_identity.clone(),
                    cadence: run.cadence.clone(),
                    replay_trace_parity_passed: replay.trace.passed,
                    first_contact_step: replay.first_contact.map(|contact| contact.physics_step),
                    sign_geometry_scan: replay.sign_geometry_scan,
                });
            }
        }
    }
    if scans.len() != 18 {
        bail!("trajectory sign scan did not cover all eighteen frozen cadence traces");
    }
    Ok(scans)
}

fn source_handoff_gate_evidence(
    execution: &super::launch_feasibility::FlatThirdCandidateExecution,
    replay_trace_parity: ReplayTraceParityEvidence,
    sign_geometry_scan: TrajectorySignGeometryScanEvidence,
) -> FlatCandidateSourceHandoffGateEvidence {
    let run = &execution.source_gate_run;
    let stop_reason = if execution.source_gate_passed {
        None
    } else if !run.launch.completed || !run.launch.contact_free {
        Some("launch_contact_or_termination_before_source_handoff".to_owned())
    } else if !run.rollout.contacts.is_empty() {
        Some("contact_observed_on_or_before_source_handoff".to_owned())
    } else if run
        .rollout
        .source_handoff_position_error_m
        .is_some_and(|error| error > STATE_POSITION_TOLERANCE_M)
    {
        Some("source_handoff_position_error_exceeds_1e-6_m".to_owned())
    } else if run
        .rollout
        .source_handoff_velocity_error_mps
        .is_some_and(|error| error > STATE_VELOCITY_TOLERANCE_MPS)
    {
        Some("source_handoff_velocity_error_exceeds_1e-6_mps".to_owned())
    } else if run.rollout.source_handoff_position_error_m.is_none()
        || run.rollout.source_handoff_velocity_error_mps.is_none()
    {
        Some("source_handoff_not_reached".to_owned())
    } else if !run.rollout.source_handoff_contact_free {
        Some("source_handoff_was_not_contact_free".to_owned())
    } else if !run.rollout.source_handoff_reached {
        Some("source_handoff_did_not_meet_declared_state_tolerance".to_owned())
    } else {
        Some("source_handoff_gate_failed_for_unclassified_reason".to_owned())
    };
    FlatCandidateSourceHandoffGateEvidence {
        cadence: run.cadence.clone(),
        passed: execution.source_gate_passed,
        stop_reason,
        launch: run.launch.clone(),
        reseeded_bridge: run.reseeded_bridge.clone(),
        source_handoff_reached: run.rollout.source_handoff_reached,
        source_handoff_contact_free: run.rollout.source_handoff_contact_free,
        source_handoff_position_error_m: run.rollout.source_handoff_position_error_m,
        source_handoff_velocity_error_mps: run.rollout.source_handoff_velocity_error_mps,
        state_at_stop: run.rollout.termination.clone(),
        physics_ticks_after_launch: run
            .rollout
            .physics_steps_advanced
            .saturating_sub(run.launch.physics_ticks_completed),
        replay_trace_parity,
        sign_geometry_scan,
        completed_direct_prefix_matched: execution.completed_direct_prefix_matched,
    }
}

fn candidate_cadence_evidence(
    run: &LaunchFeasibilityCadenceRunEvidence,
    replay: super::launch_contact_contract::ReplayTraceResult,
) -> FlatCandidateCadenceEvidence {
    FlatCandidateCadenceEvidence {
        cadence: run.cadence.clone(),
        status: run.rollout.status.clone(),
        launch: run.launch.clone(),
        reseeded_bridge: run.reseeded_bridge.clone(),
        source_handoff_reached: run.rollout.source_handoff_reached,
        source_handoff_contact_free: run.rollout.source_handoff_contact_free,
        source_handoff_position_error_m: run.rollout.source_handoff_position_error_m,
        source_handoff_velocity_error_mps: run.rollout.source_handoff_velocity_error_mps,
        strict_source_handoff_within_tolerance: run.rollout.source_handoff_contact_free
            && run
                .rollout
                .source_handoff_position_error_m
                .is_some_and(|error| error <= STATE_POSITION_TOLERANCE_M)
            && run
                .rollout
                .source_handoff_velocity_error_mps
                .is_some_and(|error| error <= STATE_VELOCITY_TOLERANCE_MPS),
        first_contact: replay.first_contact,
        replay_trace_parity: replay.trace,
        profile_end: replay.profile_end_observed,
        post_terminal_state: replay.post_terminal_state,
        authoritative_outcome: run.rollout.termination.clone(),
        sign_geometry_scan: replay.sign_geometry_scan,
    }
}

fn contact_audit_case<'a>(
    audit: &'a WaypointDirectLaunchContactContractArtifact,
    case_id: &str,
) -> Result<&'a LaunchContactContractCaseEvidence> {
    audit
        .cases
        .iter()
        .find(|case| case.input.id == case_id)
        .ok_or_else(|| anyhow!("first-contact audit is missing case {case_id}"))
}

fn contact_audit_candidate<'a>(
    case: &'a LaunchContactContractCaseEvidence,
    candidate_identity: &str,
) -> Result<&'a LaunchContactContractCandidateEvidence> {
    case.candidate_profiles
        .iter()
        .find(|candidate| candidate.candidate_identity == candidate_identity)
        .ok_or_else(|| anyhow!("first-contact audit is missing candidate {candidate_identity}"))
}

fn contact_audit_cadence<'a>(
    candidate: &'a LaunchContactContractCandidateEvidence,
    cadence: &str,
) -> Result<&'a LaunchContactContractCadenceEvidence> {
    candidate
        .cadence_runs
        .iter()
        .find(|run| run.cadence == cadence)
        .ok_or_else(|| anyhow!("first-contact audit is missing cadence {cadence}"))
}

fn flat_case(prepared: &PreparedInputs) -> Result<&PreparedCase> {
    prepared
        .cases
        .iter()
        .find(|case| case.evidence.id == EXPECTED_FLAT_CASE_ID)
        .ok_or_else(|| anyhow!("reevaluated flat case is missing"))
}

fn input_gate_identity(gate: &FlatCandidateClosureInputGateEvidence) -> Result<String> {
    let mut input = gate.clone();
    input.identity.clear();
    stable_digest(&input)
}

fn artifact_identity(artifact: &WaypointDirectFlatCandidateClosureArtifact) -> Result<String> {
    let mut input = artifact.clone();
    input.identity.clear();
    stable_digest(&input)
}
