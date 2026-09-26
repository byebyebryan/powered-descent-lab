//! No-new-flight coupled-thrust audit for the three frozen flat candidates.
//!
//! This evaluator lane rebuilds exact source bridges from sealed inputs and
//! cross-references stored launch logs. It never constructs simulation state.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::Vec2;
use pd_plan::conservative_ballistic_bridge::{
    AnalyticalBridgeV2, BridgeKindV2, CertificationV2, ComponentMarginsV2, DirectBridgeReasonV2,
    KinematicStateV2, exact_discrete_bridge_v2,
};
use serde::{Deserialize, Serialize};

use super::flat_candidate_closure::{
    CoupledThrustAuditCandidateInput, load_coupled_thrust_audit_inputs,
};
use super::{
    CommandSaturationEvidence, FirstContactEvidence, FlatCandidateClosureInputGateEvidence,
    LaunchEvidence, PlantStateEvidence, ReseededBridgeEvidence, resolve_output_dir, stable_digest,
};

pub const WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_ID: &str = "waypoint-direct-coupled-thrust-audit";
pub const WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_SCHEMA_ID: &str =
    "waypoint_direct_coupled_thrust_audit_v1";
pub const WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_SCHEMA_VERSION: u32 = 1;

const EXPECTED_BASELINE_IDENTITY: &str = "fnv1a64:d3fa6b24336f7c05";
const EXPECTED_SWEEP_IDENTITY: &str = "fnv1a64:1bcd5a3bd6c6da01";
const EXPECTED_NOMINAL_IDENTITY: &str = "fnv1a64:9e8cbc902ca11fbc";
const EXPECTED_NOMINAL_MANIFEST_IDENTITY: &str = "fnv1a64:9e32aa0c1a52b28b";
const EXPECTED_LAUNCH_IDENTITY: &str = "fnv1a64:2c7b965ffdc809d6";
const EXPECTED_LAUNCH_INPUT_GATE_IDENTITY: &str = "fnv1a64:a4654d06c9166b14";
const EXPECTED_CONTACT_AUDIT_IDENTITY: &str = "fnv1a64:f5a6600e99cd283b";
const EXPECTED_CONTACT_AUDIT_INPUT_GATE_IDENTITY: &str = "fnv1a64:86861aa3e4f8b35e";
const EXPECTED_FLAT_CANARY_IDENTITY: &str = "fnv1a64:f3fba9290c9073be";
const EXPECTED_FLAT_CANARY_INPUT_GATE_IDENTITY: &str = "fnv1a64:d7476fd0e0827260";
const EXPECTED_NATIVE_CANDIDATE_IDENTITY: &str = "fnv1a64:dee613017622ca16";
const EXPECTED_SHORTEST_CANDIDATE_IDENTITY: &str = "fnv1a64:4e6c0b23f9eb1b8f";
const EXPECTED_THIRD_CANDIDATE_IDENTITY: &str = "fnv1a64:a18a98ad6e334014";
const EXPECTED_RESEEDED_BRIDGE_IDENTITIES: [&str; 3] = [
    "fnv1a64:f768017d313497b2",
    "fnv1a64:ff662b48b7bfe88d",
    "fnv1a64:d7383dd4be5ad00e",
];
const EXPECTED_PHYSICS_HZ: u32 = 120;
const EXPECTED_ROBUSTNESS_MARGIN: f64 = 0.075;
const LAUNCH_PHYSICS_TICKS: u64 = 72;
const BRIDGE_VECTOR_TOLERANCE: f64 = 2.0e-10;

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectCoupledThrustAuditPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectCoupledThrustAuditRun {
    pub artifact: WaypointDirectCoupledThrustAuditArtifact,
    pub paths: WaypointDirectCoupledThrustAuditPaths,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectCoupledThrustAuditInputPaths {
    pub baseline_summary: PathBuf,
    pub sweep_summary: PathBuf,
    pub nominal_summary: PathBuf,
    pub launch_summary: PathBuf,
    pub contact_audit_summary: PathBuf,
    pub flat_canary_summary: PathBuf,
}

#[derive(Clone, Copy)]
struct ReseedBridgeContext<'a> {
    policy: &'a super::DirectBridgePolicyV2,
    vehicle: &'a super::VehicleInputV2,
    original: &'a AnalyticalBridgeV2,
    handoff: KinematicStateV2,
    candidate_identity: &'a str,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectCoupledThrustAuditInputGateEvidence {
    pub baseline_identity: String,
    pub sweep_identity: String,
    pub nominal_plant_identity: String,
    pub nominal_input_manifest_identity: String,
    pub launch_feasibility_identity: String,
    pub launch_feasibility_input_gate_identity: String,
    pub contact_audit_identity: String,
    pub contact_audit_input_gate_identity: String,
    pub flat_candidate_closure_input_gate: FlatCandidateClosureInputGateEvidence,
    pub flat_canary_identity: String,
    pub flat_canary_input_gate_identity: String,
    pub candidate_identities: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectCoupledThrustAuditProtocolEvidence {
    pub source_bridge_recompute_rule: String,
    pub reseed_rule: String,
    pub peak_rule: String,
    pub source_log_mapping_rule: String,
    pub analytical_attribution_rule: String,
    pub no_new_flight_rule: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectCoupledThrustAuditArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub input_gate: WaypointDirectCoupledThrustAuditInputGateEvidence,
    pub protocol: WaypointDirectCoupledThrustAuditProtocolEvidence,
    pub candidates: Vec<CoupledThrustCandidateEvidence>,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoupledThrustCandidateEvidence {
    pub candidate_identity: String,
    pub role: String,
    pub classification: CertificationV2,
    pub candidate_reasons: Vec<DirectBridgeReasonV2>,
    pub candidate_margins: ComponentMarginsV2,
    pub original_source_start_state: KinematicStateV2,
    pub launch_reseed_start_state: KinematicStateV2,
    pub start_position_offset_m: Vec2,
    pub start_velocity_offset_mps: Vec2,
    pub original_source_bridge: CoupledThrustBridgeEvidence,
    pub reseeded_source_bridge: CoupledThrustBridgeEvidence,
    pub peak_thrust_change_at_reseeded_peak: PeakThrustChangeAttributionEvidence,
    pub cadence_cross_checks: Vec<CoupledThrustCadenceCrossCheckEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoupledThrustBridgeEvidence {
    pub stored_identity: String,
    pub recomputed_identity: String,
    pub identity_matches_stored: bool,
    pub kind: BridgeKindV2,
    pub classification: CertificationV2,
    pub reasons: Vec<DirectBridgeReasonV2>,
    pub margins: ComponentMarginsV2,
    pub start_state: KinematicStateV2,
    pub end_state: KinematicStateV2,
    pub tick_count: u64,
    pub duration_s: f64,
    pub endpoint_position_error_m: f64,
    pub endpoint_velocity_error_mps: f64,
    pub fuel_burn_kg: f64,
    pub initial_net_acceleration_mps2: Vec2,
    pub net_acceleration_step_mps2: Vec2,
    pub first_thrust_acceleration: ThrustVectorEvidence,
    pub last_thrust_acceleration: ThrustVectorEvidence,
    pub peak_thrust_acceleration: ThrustVectorEvidence,
    pub peak_tick_index: u64,
    pub peak_physics_step: Option<u64>,
    pub materialized_sample_count: u64,
    pub materialized_max_thrust_acceleration_mps2: f64,
    pub endpoint_max_thrust_acceleration_mps2: f64,
    pub materialized_max_matches_endpoint_max: bool,
    pub derated_cap_mps2: f64,
    pub robust_cap_mps2: f64,
    pub coupled_thrust_raw_reserve_mps2: f64,
    pub coupled_thrust_normalized_reserve: f64,
    pub signed_robust_cap_margin_mps2: f64,
    pub deficit_to_robust_cap_mps2: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThrustVectorEvidence {
    pub acceleration_mps2: Vec2,
    pub magnitude_mps2: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeakThrustChangeAttributionEvidence {
    pub peak_tick_index: u64,
    pub mapped_physics_step: u64,
    pub original_thrust_acceleration_mps2: Vec2,
    pub reseeded_thrust_acceleration_mps2: Vec2,
    pub full_thrust_change_mps2: Vec2,
    pub position_offset_contribution_mps2: Vec2,
    pub velocity_offset_contribution_mps2: Vec2,
    pub contribution_sum_mps2: Vec2,
    pub decomposition_matches_recomputed_change: bool,
    pub interpretation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoupledThrustCadenceCrossCheckEvidence {
    pub cadence: String,
    pub source_log_status: String,
    pub mapped_peak_physics_step: u64,
    pub logged_peak_phase: Option<String>,
    pub desired_target_attitude_rad: Option<f64>,
    pub held_target_attitude_rad: Option<f64>,
    pub commanded_throttle_frac: Option<f64>,
    pub applied_throttle_frac: Option<f64>,
    pub fuel_kg: Option<f64>,
    pub saturation_counters: Option<CommandSaturationEvidence>,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub replay_trace_parity_passed: bool,
    pub first_contact: Option<FirstContactEvidence>,
    pub authoritative_outcome: PlantStateEvidence,
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectCoupledThrustAuditValidation {
    pub input_gate: WaypointDirectCoupledThrustAuditInputGateEvidence,
    pub candidate_count: usize,
    pub candidate_identities: Vec<String>,
}

/// Validate every sealed identity and input-gate binding without executing
/// physics. This is the only operation used by the preflight CLI mode.
pub fn validate_waypoint_direct_coupled_thrust_audit_inputs(
    repo_root: &Path,
    input_paths: &WaypointDirectCoupledThrustAuditInputPaths,
) -> Result<WaypointDirectCoupledThrustAuditValidation> {
    let inputs = load_coupled_thrust_audit_inputs(repo_root, input_paths)?;
    let input_gate = build_input_gate(&inputs)?;
    Ok(WaypointDirectCoupledThrustAuditValidation {
        candidate_count: inputs.candidates.len(),
        candidate_identities: input_gate.candidate_identities.clone(),
        input_gate,
    })
}

/// Build the compact analytical report from exact source bridge identities and
/// the frozen launch/canary logs. No simulator state or new flight is created.
pub fn run_waypoint_direct_coupled_thrust_audit(
    repo_root: &Path,
    input_paths: &WaypointDirectCoupledThrustAuditInputPaths,
    output_dir: &Path,
) -> Result<WaypointDirectCoupledThrustAuditRun> {
    let inputs = load_coupled_thrust_audit_inputs(repo_root, input_paths)?;
    let input_gate = build_input_gate(&inputs)?;
    let candidates = inputs
        .candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| audit_candidate(candidate, index, &inputs))
        .collect::<Result<Vec<_>>>()?;
    if candidates.len() != 3 {
        bail!("coupled-thrust audit did not produce exactly three flat candidate rows");
    }

    let mut artifact = WaypointDirectCoupledThrustAuditArtifact {
        schema_id: WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_ID.to_owned(),
        input_gate,
        protocol: WaypointDirectCoupledThrustAuditProtocolEvidence {
            source_bridge_recompute_rule: "reevaluate only the exact source bridge already present on each of the three frozen Certified direct candidates; compare its recomputed V2 bridge identity, start/end, tick count, classification, reasons, and every component margin with the reevaluated candidate".to_owned(),
            reseed_rule: "for each candidate, call exact_discrete_bridge_v2 from each stored achieved 72-physics-tick launch end state to that candidate's original source handoff using its original source bridge tick count; require 120/60 launch states and reseed identities to agree".to_owned(),
            peak_rule: "materialize the exact bridge samples, compute the full sample maximum, and verify it equals the maximum of the first and last thrust-vector magnitudes; use that endpoint maximum as the reported convex affine-norm peak".to_owned(),
            source_log_mapping_rule: "map the reseeded peak sample j to physics step 72 + j + 1; join only stored source-phase per-step logs, preserving 120 Hz and 60 Hz cadence labels and retaining aggregate saturation counters separately".to_owned(),
            analytical_attribution_rule: "at the reseeded peak tick, decompose the exact affine coefficient difference into position-offset and velocity-offset vector contributions; these are algebraic contributions, not synthetic trajectories or separate feasibility margins".to_owned(),
            no_new_flight_rule: "perform no SimulationState construction, physics advance, replay, search, sweep, launch change, or candidate ranking; the stored actual-throttle trace is kept distinct from the worst-case-mass certificate".to_owned(),
        },
        candidates,
        scope_non_claims: vec![
            "The reseeded source bridge is an exact analytical diagnostic and does not represent a new flight or prove a causal robustness mechanism.".to_owned(),
            "The component attribution is an algebraic decomposition of bridge coefficients only; it is not a pair of independently feasible bridges or margins.".to_owned(),
            "Logged applied throttle belongs to the actual simulated mass and command history; it is not the worst-case-mass coupled-thrust certificate.".to_owned(),
            "The third flat canary did not retain per-step rollout or saturation counters, so those fields are explicitly unavailable for that candidate.".to_owned(),
            "No candidate ranking, launch policy, threshold, V2 certificate, planner/controller behavior, F6 behavior, or default is changed.".to_owned(),
        ],
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;

    let output_dir = resolve_output_dir(repo_root, output_dir);
    if output_dir.exists() {
        bail!(
            "coupled-thrust audit requires a fresh output directory; refusing to write into {}",
            output_dir.display()
        );
    }
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create coupled-thrust audit output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&summary_path, &summary_bytes).with_context(|| {
        format!(
            "failed to write coupled-thrust audit summary {}",
            summary_path.display()
        )
    })?;
    let reloaded: WaypointDirectCoupledThrustAuditArtifact = serde_json::from_slice(&summary_bytes)
        .context("failed to reload coupled-thrust audit summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes
        || artifact_identity(&reloaded)? != reloaded.identity
    {
        bail!("coupled-thrust audit failed deterministic identity round-trip");
    }
    Ok(WaypointDirectCoupledThrustAuditRun {
        artifact: reloaded,
        paths: WaypointDirectCoupledThrustAuditPaths {
            output_dir,
            summary_path,
        },
    })
}

pub(super) fn build_input_gate(
    inputs: &super::flat_candidate_closure::CoupledThrustAuditValidatedInputs,
) -> Result<WaypointDirectCoupledThrustAuditInputGateEvidence> {
    let prior_gate = &inputs.input_gate;
    if prior_gate.baseline_identity != EXPECTED_BASELINE_IDENTITY
        || prior_gate.sweep_identity != EXPECTED_SWEEP_IDENTITY
        || prior_gate.nominal_plant_identity != EXPECTED_NOMINAL_IDENTITY
        || prior_gate.nominal_input_manifest_identity != EXPECTED_NOMINAL_MANIFEST_IDENTITY
        || prior_gate.launch_feasibility_identity != EXPECTED_LAUNCH_IDENTITY
        || prior_gate.launch_feasibility_input_gate_identity != EXPECTED_LAUNCH_INPUT_GATE_IDENTITY
        || prior_gate.contact_audit_identity != EXPECTED_CONTACT_AUDIT_IDENTITY
        || prior_gate.contact_audit_input_gate_identity
            != EXPECTED_CONTACT_AUDIT_INPUT_GATE_IDENTITY
        || prior_gate.third_candidate.candidate_identity != EXPECTED_THIRD_CANDIDATE_IDENTITY
        || prior_gate.original_native_candidate_identity != EXPECTED_NATIVE_CANDIDATE_IDENTITY
        || prior_gate.original_shortest_candidate_identity != EXPECTED_SHORTEST_CANDIDATE_IDENTITY
        || inputs.flat_canary_identity != EXPECTED_FLAT_CANARY_IDENTITY
        || inputs.flat_canary_input_gate_identity != EXPECTED_FLAT_CANARY_INPUT_GATE_IDENTITY
        || inputs.policy.physics_hz != EXPECTED_PHYSICS_HZ
        || inputs.policy.declared_robustness_margin != EXPECTED_ROBUSTNESS_MARGIN
        || inputs.candidates.len() != 3
    {
        bail!("coupled-thrust input pins or fixed V2 policy values changed");
    }
    let candidate_identities = inputs
        .candidates
        .iter()
        .map(|row| row.candidate.identity.clone())
        .collect::<Vec<_>>();
    if candidate_identities
        != [
            EXPECTED_NATIVE_CANDIDATE_IDENTITY,
            EXPECTED_SHORTEST_CANDIDATE_IDENTITY,
            EXPECTED_THIRD_CANDIDATE_IDENTITY,
        ]
    {
        bail!("coupled-thrust input candidate order or identity changed");
    }
    let mut gate = WaypointDirectCoupledThrustAuditInputGateEvidence {
        baseline_identity: prior_gate.baseline_identity.clone(),
        sweep_identity: prior_gate.sweep_identity.clone(),
        nominal_plant_identity: prior_gate.nominal_plant_identity.clone(),
        nominal_input_manifest_identity: prior_gate.nominal_input_manifest_identity.clone(),
        launch_feasibility_identity: prior_gate.launch_feasibility_identity.clone(),
        launch_feasibility_input_gate_identity: prior_gate
            .launch_feasibility_input_gate_identity
            .clone(),
        contact_audit_identity: prior_gate.contact_audit_identity.clone(),
        contact_audit_input_gate_identity: prior_gate.contact_audit_input_gate_identity.clone(),
        flat_candidate_closure_input_gate: prior_gate.clone(),
        flat_canary_identity: inputs.flat_canary_identity.clone(),
        flat_canary_input_gate_identity: inputs.flat_canary_input_gate_identity.clone(),
        candidate_identities,
        identity: String::new(),
    };
    gate.identity = input_gate_identity(&gate)?;
    let bytes = serde_json::to_vec(&gate)?;
    let reloaded: WaypointDirectCoupledThrustAuditInputGateEvidence =
        serde_json::from_slice(&bytes)?;
    if reloaded != gate || input_gate_identity(&reloaded)? != reloaded.identity {
        bail!("coupled-thrust input gate failed deterministic identity round-trip");
    }
    Ok(reloaded)
}

fn audit_candidate(
    input: &CoupledThrustAuditCandidateInput,
    candidate_index: usize,
    inputs: &super::flat_candidate_closure::CoupledThrustAuditValidatedInputs,
) -> Result<CoupledThrustCandidateEvidence> {
    let candidate = &input.candidate;
    let identity = candidate.identity.as_str();
    let source_bridge = candidate
        .source_bridge
        .as_ref()
        .ok_or_else(|| anyhow!("flat candidate {identity} has no original source bridge"))?;
    let source_handoff = candidate
        .source_handoff
        .ok_or_else(|| anyhow!("flat candidate {identity} has no source handoff"))?;
    if candidate.classification != CertificationV2::Certified {
        bail!("flat candidate {identity} is no longer Certified");
    }
    let original_recomputed = exact_discrete_bridge_v2(
        &inputs.policy,
        &inputs.vehicle,
        BridgeKindV2::Source,
        source_bridge.start_state,
        source_bridge.end_state,
        source_bridge.steps,
    )
    .map_err(|error| anyhow!("failed to recompute original source bridge {identity}: {error}"))?;
    require_bridge_match(
        &format!("original source bridge {identity}"),
        source_bridge,
        &original_recomputed,
    )?;
    if original_recomputed.end_state != source_handoff.state {
        bail!("recomputed original source bridge {identity} does not end at its stored handoff");
    }
    let reseed_context = ReseedBridgeContext {
        policy: &inputs.policy,
        vehicle: &inputs.vehicle,
        original: &original_recomputed,
        handoff: source_handoff.state,
        candidate_identity: identity,
    };
    let original_evidence = bridge_evidence(
        source_bridge.identity.clone(),
        &original_recomputed,
        &inputs.policy,
        &inputs.vehicle,
        None,
    )?;

    let (launch_rows, canary_rows) = if identity == EXPECTED_THIRD_CANDIDATE_IDENTITY {
        (None, Some(input.third_canary_cadences.as_slice()))
    } else {
        (Some(input.launch_cadences.as_slice()), None)
    };
    let mut launch_starts = Vec::with_capacity(2);
    let mut reseeded_bridges = Vec::with_capacity(2);
    let mut cadence_cross_checks = Vec::with_capacity(2);
    if let Some(runs) = launch_rows {
        if runs.len() != 2 || input.contact_cadences.len() != 2 {
            bail!("candidate {identity} does not have the two sealed launch/contact rows");
        }
        for (run, contact) in runs.iter().zip(&input.contact_cadences) {
            let start = launch_start_state(&run.launch, identity, &run.cadence)?;
            let stored = run.reseeded_bridge.as_ref().ok_or_else(|| {
                anyhow!(
                    "candidate {identity} {} row lacks a reseeded bridge",
                    run.cadence
                )
            })?;
            let recomputed =
                recompute_reseeded_bridge(reseed_context, start, stored, &run.cadence)?;
            launch_starts.push(start);
            reseeded_bridges.push(recomputed);
            let peak_step = reseeded_bridges[0].peak_physics_step.unwrap_or(0);
            cadence_cross_checks.push(cadence_cross_check_from_launch(run, contact, peak_step)?);
        }
    } else if let Some(rows) = canary_rows {
        if rows.len() != 2 {
            bail!("third candidate does not have both sealed canary cadence rows");
        }
        for row in rows {
            let start = launch_start_state(&row.launch, identity, &row.cadence)?;
            let stored = row.reseeded_bridge.as_ref().ok_or_else(|| {
                anyhow!(
                    "third candidate {} row lacks a reseeded bridge",
                    row.cadence
                )
            })?;
            let recomputed =
                recompute_reseeded_bridge(reseed_context, start, stored, &row.cadence)?;
            launch_starts.push(start);
            reseeded_bridges.push(recomputed);
            let peak_step = reseeded_bridges[0].peak_physics_step.unwrap_or(0);
            cadence_cross_checks.push(cadence_cross_check_from_canary(row, peak_step));
        }
    } else {
        bail!("candidate {identity} has no sealed launch evidence");
    }
    if launch_starts.len() != 2
        || launch_starts[0] != launch_starts[1]
        || reseeded_bridges.len() != 2
        || reseeded_bridges[0].recomputed_identity != reseeded_bridges[1].recomputed_identity
        || reseeded_bridges[0].recomputed_identity
            != EXPECTED_RESEEDED_BRIDGE_IDENTITIES[candidate_index]
    {
        bail!("candidate {identity} 120/60 launch or reseeded bridge identity diverged");
    }
    let reseeded_bridge = reseeded_bridges.remove(0);
    let launch_reseed_start_state = launch_starts[0];
    let start_position_offset_m =
        launch_reseed_start_state.position_m - original_recomputed.start_state.position_m;
    let start_velocity_offset_mps =
        launch_reseed_start_state.velocity_mps - original_recomputed.start_state.velocity_mps;
    let attribution = peak_thrust_change_attribution(
        &original_recomputed,
        &reseeded_bridge,
        start_position_offset_m,
        start_velocity_offset_mps,
        inputs.policy.physics_hz,
    )?;
    for cross_check in &cadence_cross_checks {
        if cross_check.mapped_peak_physics_step != attribution.mapped_physics_step {
            bail!("candidate {identity} cadence row is mapped to the wrong peak physics step");
        }
    }
    let role = match candidate_index {
        0 => "v2_native_selected",
        1 => "research_shortest_certified",
        2 => "third_certified_flat_canary",
        _ => bail!("unexpected flat candidate position {candidate_index}"),
    };
    Ok(CoupledThrustCandidateEvidence {
        candidate_identity: candidate.identity.clone(),
        role: role.to_owned(),
        classification: candidate.classification,
        candidate_reasons: candidate.reasons.clone(),
        candidate_margins: candidate.margins,
        original_source_start_state: original_recomputed.start_state,
        launch_reseed_start_state,
        start_position_offset_m,
        start_velocity_offset_mps,
        original_source_bridge: original_evidence,
        reseeded_source_bridge: reseeded_bridge,
        peak_thrust_change_at_reseeded_peak: attribution,
        cadence_cross_checks,
    })
}

fn launch_start_state(
    launch: &LaunchEvidence,
    candidate_identity: &str,
    cadence: &str,
) -> Result<KinematicStateV2> {
    if !launch.completed
        || !launch.contact_free
        || launch.physics_ticks_completed != LAUNCH_PHYSICS_TICKS
    {
        bail!(
            "candidate {candidate_identity} {cadence} launch is not the sealed 72-tick contact-free launch"
        );
    }
    let end_state = launch.end_state.as_ref().ok_or_else(|| {
        anyhow!("candidate {candidate_identity} {cadence} launch has no achieved end state")
    })?;
    Ok(KinematicStateV2 {
        position_m: end_state.position_m,
        velocity_mps: end_state.velocity_mps,
    })
}

fn recompute_reseeded_bridge(
    context: ReseedBridgeContext<'_>,
    launch_start: KinematicStateV2,
    stored: &ReseededBridgeEvidence,
    cadence: &str,
) -> Result<CoupledThrustBridgeEvidence> {
    if context.original.end_state != context.handoff {
        bail!(
            "candidate {} original source handoff changed",
            context.candidate_identity
        );
    }
    let bridge = exact_discrete_bridge_v2(
        context.policy,
        context.vehicle,
        BridgeKindV2::Source,
        launch_start,
        context.handoff,
        context.original.steps,
    )
    .map_err(|error| {
        anyhow!(
            "failed to reseed source bridge {}/{cadence}: {error}",
            context.candidate_identity
        )
    })?;
    if stored.identity.as_deref() != Some(bridge.identity.as_str())
        || stored.classification != Some(bridge.classification)
        || stored.reasons != bridge.reasons
        || stored.start_state != Some(bridge.start_state)
        || stored.end_state != Some(bridge.end_state)
        || stored.tick_count != bridge.steps
        || stored.margins != Some(bridge.margins)
        || stored.endpoint_position_error_m != Some(bridge.endpoint_position_error_m)
        || stored.endpoint_velocity_error_mps != Some(bridge.endpoint_velocity_error_mps)
        || stored.fuel_burn_kg != Some(bridge.fuel_burn_kg)
    {
        bail!(
            "recomputed source bridge {}/{cadence} differs from stored launch evidence",
            context.candidate_identity
        );
    }
    bridge_evidence(
        bridge.identity.clone(),
        &bridge,
        context.policy,
        context.vehicle,
        Some(LAUNCH_PHYSICS_TICKS),
    )
}

fn require_bridge_match(
    label: &str,
    stored: &AnalyticalBridgeV2,
    recomputed: &AnalyticalBridgeV2,
) -> Result<()> {
    if stored.identity != recomputed.identity
        || stored.kind != recomputed.kind
        || stored.classification != recomputed.classification
        || stored.reasons != recomputed.reasons
        || stored.margins != recomputed.margins
        || stored.start_state != recomputed.start_state
        || stored.end_state != recomputed.end_state
        || stored.steps != recomputed.steps
        || stored.endpoint_position_error_m != recomputed.endpoint_position_error_m
        || stored.endpoint_velocity_error_mps != recomputed.endpoint_velocity_error_mps
        || stored.fuel_burn_kg != recomputed.fuel_burn_kg
        || stored.initial_net_acceleration_mps2 != recomputed.initial_net_acceleration_mps2
        || stored.net_acceleration_step_mps2 != recomputed.net_acceleration_step_mps2
    {
        bail!("{label} exact_discrete_bridge_v2 identity or certificate fields changed");
    }
    Ok(())
}

fn bridge_evidence(
    stored_identity: String,
    bridge: &AnalyticalBridgeV2,
    policy: &super::DirectBridgePolicyV2,
    vehicle: &super::VehicleInputV2,
    launch_tick_offset: Option<u64>,
) -> Result<CoupledThrustBridgeEvidence> {
    if bridge.steps < 2 || bridge.samples.len() != bridge.steps as usize {
        bail!(
            "source bridge {} has incomplete exact materialized samples",
            bridge.identity
        );
    }
    for (index, sample) in bridge.samples.iter().enumerate() {
        if sample.tick != index as u64 {
            bail!(
                "source bridge {} samples are not indexed consecutively",
                bridge.identity
            );
        }
    }
    let thrust = bridge
        .samples
        .iter()
        .map(|sample| sample.thrust_acceleration_mps2)
        .collect::<Vec<_>>();
    let peak_tick_index = endpoint_peak_tick_index(&thrust)?;
    let first = thrust_vector(thrust[0]);
    let last = thrust_vector(*thrust.last().expect("validated non-empty bridge"));
    let peak_vector = thrust_vector(thrust[peak_tick_index as usize]);
    let materialized_max = thrust
        .iter()
        .map(|vector| vector.length())
        .fold(0.0_f64, f64::max);
    let endpoint_max = first.magnitude_mps2.max(last.magnitude_mps2);
    let max_tolerance = BRIDGE_VECTOR_TOLERANCE * endpoint_max.max(1.0);
    if (materialized_max - endpoint_max).abs() > max_tolerance {
        bail!(
            "source bridge {} materialized peak does not equal its endpoint maximum",
            bridge.identity
        );
    }
    let derated_cap =
        policy.thrust_derate * (vehicle.max_thrust_n / (vehicle.dry_mass_kg + vehicle.max_fuel_kg));
    let robust_cap = derated_cap * (1.0 - policy.declared_robustness_margin);
    let raw_reserve = derated_cap - peak_vector.magnitude_mps2;
    let normalized_reserve = raw_reserve / derated_cap;
    let signed_robust_margin = robust_cap - peak_vector.magnitude_mps2;
    let frozen_margin = bridge.margins.coupled_thrust;
    if (frozen_margin.raw - raw_reserve).abs() > max_tolerance
        || (frozen_margin.normalized - normalized_reserve).abs() > BRIDGE_VECTOR_TOLERANCE
    {
        bail!(
            "source bridge {} recomputed reserve differs from its V2 coupled-thrust margin",
            bridge.identity
        );
    }
    let peak_physics_step = launch_tick_offset.map(|offset| offset + peak_tick_index + 1);
    let identity_matches_stored = bridge.identity == stored_identity;
    Ok(CoupledThrustBridgeEvidence {
        stored_identity,
        recomputed_identity: bridge.identity.clone(),
        identity_matches_stored,
        kind: bridge.kind,
        classification: bridge.classification,
        reasons: bridge.reasons.clone(),
        margins: bridge.margins,
        start_state: bridge.start_state,
        end_state: bridge.end_state,
        tick_count: bridge.steps,
        duration_s: bridge.duration_s,
        endpoint_position_error_m: bridge.endpoint_position_error_m,
        endpoint_velocity_error_mps: bridge.endpoint_velocity_error_mps,
        fuel_burn_kg: bridge.fuel_burn_kg,
        initial_net_acceleration_mps2: bridge.initial_net_acceleration_mps2,
        net_acceleration_step_mps2: bridge.net_acceleration_step_mps2,
        first_thrust_acceleration: first,
        last_thrust_acceleration: last,
        peak_thrust_acceleration: peak_vector,
        peak_tick_index,
        peak_physics_step,
        materialized_sample_count: bridge.samples.len() as u64,
        materialized_max_thrust_acceleration_mps2: materialized_max,
        endpoint_max_thrust_acceleration_mps2: endpoint_max,
        materialized_max_matches_endpoint_max: true,
        derated_cap_mps2: derated_cap,
        robust_cap_mps2: robust_cap,
        coupled_thrust_raw_reserve_mps2: raw_reserve,
        coupled_thrust_normalized_reserve: normalized_reserve,
        signed_robust_cap_margin_mps2: signed_robust_margin,
        deficit_to_robust_cap_mps2: (-signed_robust_margin).max(0.0),
    })
}

fn endpoint_peak_tick_index(thrust: &[Vec2]) -> Result<u64> {
    if thrust.len() < 2 {
        bail!("an affine source bridge needs first and last thrust samples");
    }
    Ok(if thrust[0].length() > thrust[thrust.len() - 1].length() {
        0
    } else {
        (thrust.len() - 1) as u64
    })
}

fn thrust_vector(acceleration_mps2: Vec2) -> ThrustVectorEvidence {
    ThrustVectorEvidence {
        acceleration_mps2,
        magnitude_mps2: acceleration_mps2.length(),
    }
}

fn peak_thrust_change_attribution(
    original: &AnalyticalBridgeV2,
    reseeded: &CoupledThrustBridgeEvidence,
    position_offset_m: Vec2,
    velocity_offset_mps: Vec2,
    physics_hz: u32,
) -> Result<PeakThrustChangeAttributionEvidence> {
    let tick = reseeded.peak_tick_index;
    let original_peak = original.samples[tick as usize].thrust_acceleration_mps2;
    let reseeded_peak = reseeded.peak_thrust_acceleration.acceleration_mps2;
    let (position_initial, position_step) = start_offset_coefficients(
        position_offset_m,
        Vec2::new(0.0, 0.0),
        original.steps,
        physics_hz,
    );
    let (velocity_initial, velocity_step) = start_offset_coefficients(
        Vec2::new(0.0, 0.0),
        velocity_offset_mps,
        original.steps,
        physics_hz,
    );
    let j = tick as f64;
    let position_contribution = position_initial + position_step * j;
    let velocity_contribution = velocity_initial + velocity_step * j;
    let contribution_sum = position_contribution + velocity_contribution;
    let full_change = reseeded_peak - original_peak;
    let matches = vector_approx_eq(contribution_sum, full_change, BRIDGE_VECTOR_TOLERANCE);
    if !matches {
        bail!(
            "position/velocity coefficient attribution does not sum to the recomputed peak thrust change"
        );
    }
    let mapped_physics_step = LAUNCH_PHYSICS_TICKS + tick + 1;
    Ok(PeakThrustChangeAttributionEvidence {
        peak_tick_index: tick,
        mapped_physics_step,
        original_thrust_acceleration_mps2: original_peak,
        reseeded_thrust_acceleration_mps2: reseeded_peak,
        full_thrust_change_mps2: full_change,
        position_offset_contribution_mps2: position_contribution,
        velocity_offset_contribution_mps2: velocity_contribution,
        contribution_sum_mps2: contribution_sum,
        decomposition_matches_recomputed_change: matches,
        interpretation: "exact algebraic contributions from fixed-handoff bridge_coefficients at the reseeded peak tick; not synthetic trajectories or separate feasibility margins".to_owned(),
    })
}

fn start_offset_coefficients(
    delta_start_position: Vec2,
    delta_start_velocity: Vec2,
    steps: u64,
    physics_hz: u32,
) -> (Vec2, Vec2) {
    let dt_s = 1.0 / f64::from(physics_hz);
    let n = steps as f64;
    let d = delta_start_velocity * (-1.0 / dt_s);
    let s = (delta_start_position + delta_start_velocity * (n * dt_s)) * (-1.0 / (dt_s * dt_s));
    let initial = s * (6.0 / (n * (n + 1.0))) - d * (2.0 / n);
    let step = d * (6.0 / (n * (n - 1.0))) - s * (12.0 / (n * (n + 1.0) * (n - 1.0)));
    (initial, step)
}

fn cadence_cross_check_from_launch(
    run: &super::LaunchFeasibilityCadenceRunEvidence,
    contact: &super::LaunchContactContractCadenceEvidence,
    mapped_peak_physics_step: u64,
) -> Result<CoupledThrustCadenceCrossCheckEvidence> {
    let logged = run
        .rollout
        .per_step
        .iter()
        .find(|tick| tick.physics_step == mapped_peak_physics_step)
        .ok_or_else(|| {
            anyhow!(
                "{} log is missing mapped peak physics step {mapped_peak_physics_step}",
                run.cadence
            )
        })?;
    if logged.phase != "source_bridge" {
        bail!(
            "{} mapped peak physics step {mapped_peak_physics_step} is not in source_bridge phase",
            run.cadence
        );
    }
    Ok(CoupledThrustCadenceCrossCheckEvidence {
        cadence: run.cadence.clone(),
        source_log_status: "stored_source_phase_per_step_log".to_owned(),
        mapped_peak_physics_step,
        logged_peak_phase: Some(logged.phase.clone()),
        desired_target_attitude_rad: Some(logged.desired_target_attitude_rad),
        held_target_attitude_rad: Some(logged.held_target_attitude_rad),
        commanded_throttle_frac: Some(logged.commanded_throttle_frac),
        applied_throttle_frac: Some(logged.applied_throttle_frac),
        fuel_kg: Some(logged.fuel_kg),
        saturation_counters: Some(run.rollout.saturation.clone()),
        source_handoff_position_error_m: run.rollout.source_handoff_position_error_m,
        source_handoff_velocity_error_mps: run.rollout.source_handoff_velocity_error_mps,
        replay_trace_parity_passed: contact.trace.as_ref().is_some_and(|trace| trace.passed),
        first_contact: contact.first_contact.clone(),
        authoritative_outcome: run.rollout.termination.clone(),
    })
}

fn cadence_cross_check_from_canary(
    row: &super::FlatCandidateCadenceEvidence,
    mapped_peak_physics_step: u64,
) -> CoupledThrustCadenceCrossCheckEvidence {
    CoupledThrustCadenceCrossCheckEvidence {
        cadence: row.cadence.clone(),
        source_log_status: "not_recorded_in_sealed_canary".to_owned(),
        mapped_peak_physics_step,
        logged_peak_phase: None,
        desired_target_attitude_rad: None,
        held_target_attitude_rad: None,
        commanded_throttle_frac: None,
        applied_throttle_frac: None,
        fuel_kg: None,
        saturation_counters: None,
        source_handoff_position_error_m: row.source_handoff_position_error_m,
        source_handoff_velocity_error_mps: row.source_handoff_velocity_error_mps,
        replay_trace_parity_passed: row.replay_trace_parity.passed,
        first_contact: row.first_contact.clone(),
        authoritative_outcome: row.authoritative_outcome.clone(),
    }
}

fn build_input_gate_identity(
    gate: &WaypointDirectCoupledThrustAuditInputGateEvidence,
) -> Result<String> {
    let mut input = gate.clone();
    input.identity.clear();
    stable_digest(&input)
}

fn input_gate_identity(gate: &WaypointDirectCoupledThrustAuditInputGateEvidence) -> Result<String> {
    build_input_gate_identity(gate)
}

pub(super) fn artifact_identity(
    artifact: &WaypointDirectCoupledThrustAuditArtifact,
) -> Result<String> {
    let mut input = artifact.clone();
    input.identity.clear();
    stable_digest(&input)
}

fn vector_approx_eq(left: Vec2, right: Vec2, tolerance: f64) -> bool {
    (left - right).length() <= tolerance * left.length().max(right.length()).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coupled_thrust_audit_pins_match_the_frozen_flat_inputs() {
        assert_eq!(EXPECTED_BASELINE_IDENTITY, "fnv1a64:d3fa6b24336f7c05");
        assert_eq!(EXPECTED_SWEEP_IDENTITY, "fnv1a64:1bcd5a3bd6c6da01");
        assert_eq!(EXPECTED_NOMINAL_IDENTITY, "fnv1a64:9e8cbc902ca11fbc");
        assert_eq!(
            EXPECTED_NOMINAL_MANIFEST_IDENTITY,
            "fnv1a64:9e32aa0c1a52b28b"
        );
        assert_eq!(EXPECTED_LAUNCH_IDENTITY, "fnv1a64:2c7b965ffdc809d6");
        assert_eq!(
            EXPECTED_LAUNCH_INPUT_GATE_IDENTITY,
            "fnv1a64:a4654d06c9166b14"
        );
        assert_eq!(EXPECTED_CONTACT_AUDIT_IDENTITY, "fnv1a64:f5a6600e99cd283b");
        assert_eq!(
            EXPECTED_CONTACT_AUDIT_INPUT_GATE_IDENTITY,
            "fnv1a64:86861aa3e4f8b35e"
        );
        assert_eq!(EXPECTED_FLAT_CANARY_IDENTITY, "fnv1a64:f3fba9290c9073be");
        assert_eq!(
            EXPECTED_FLAT_CANARY_INPUT_GATE_IDENTITY,
            "fnv1a64:d7476fd0e0827260"
        );
        assert_eq!(
            EXPECTED_RESEEDED_BRIDGE_IDENTITIES,
            [
                "fnv1a64:f768017d313497b2",
                "fnv1a64:ff662b48b7bfe88d",
                "fnv1a64:d7383dd4be5ad00e",
            ]
        );
        assert_eq!(
            [
                EXPECTED_NATIVE_CANDIDATE_IDENTITY,
                EXPECTED_SHORTEST_CANDIDATE_IDENTITY,
                EXPECTED_THIRD_CANDIDATE_IDENTITY,
            ],
            [
                "fnv1a64:dee613017622ca16",
                "fnv1a64:4e6c0b23f9eb1b8f",
                "fnv1a64:a18a98ad6e334014",
            ]
        );
    }

    #[test]
    fn coupled_thrust_reserve_and_robust_deficit_use_worst_case_mass_cap() {
        let policy_derate = 0.9;
        let max_thrust_n = 18_000.0;
        let dry_mass_kg = 500.0;
        let max_fuel_kg = 500.0;
        let peak_mps2 = 12.0;
        let cap = policy_derate * max_thrust_n / (dry_mass_kg + max_fuel_kg);
        let robust_cap = cap * (1.0 - EXPECTED_ROBUSTNESS_MARGIN);
        let raw_reserve = cap - peak_mps2;
        let normalized_reserve = raw_reserve / cap;
        let deficit = (peak_mps2 - robust_cap).max(0.0);
        assert_eq!(cap, 16.2);
        assert_eq!(robust_cap, 14.985);
        assert!((raw_reserve - 4.2).abs() < 1.0e-14);
        assert!((normalized_reserve - (4.2 / 16.2)).abs() < 1.0e-15);
        assert_eq!(deficit, 0.0);

        let over_robust_peak_mps2 = 16.0;
        let signed_robust_margin = robust_cap - over_robust_peak_mps2;
        let over_robust_deficit = (-signed_robust_margin).max(0.0);
        assert!(signed_robust_margin < 0.0);
        assert!((over_robust_deficit - (over_robust_peak_mps2 - robust_cap)).abs() < 1.0e-15);
    }

    #[test]
    fn affine_vector_norm_peak_is_at_one_of_the_endpoints() {
        let samples = [
            Vec2::new(3.0, 4.0),
            Vec2::new(2.0, 1.0),
            Vec2::new(0.0, 2.0),
        ];
        let endpoint = endpoint_peak_tick_index(&samples).expect("valid endpoint samples");
        let materialized_max = samples
            .iter()
            .map(|sample| sample.length())
            .fold(0.0_f64, f64::max);
        let endpoint_max = samples[0].length().max(samples[2].length());
        assert_eq!(endpoint, 0);
        assert_eq!(materialized_max, endpoint_max);
    }

    #[test]
    fn bridge_start_offset_coefficients_split_additively_at_peak_tick() {
        let delta_position = Vec2::new(-0.0006, 1.4566);
        let delta_velocity = Vec2::new(-0.0115, 4.7925);
        let steps = 1920;
        let peak_tick = 1919;
        let (position_initial, position_step) = start_offset_coefficients(
            delta_position,
            Vec2::new(0.0, 0.0),
            steps,
            EXPECTED_PHYSICS_HZ,
        );
        let (velocity_initial, velocity_step) = start_offset_coefficients(
            Vec2::new(0.0, 0.0),
            delta_velocity,
            steps,
            EXPECTED_PHYSICS_HZ,
        );
        let (combined_initial, combined_step) =
            start_offset_coefficients(delta_position, delta_velocity, steps, EXPECTED_PHYSICS_HZ);
        let tick = peak_tick as f64;
        let split =
            (position_initial + position_step * tick) + (velocity_initial + velocity_step * tick);
        let combined = combined_initial + combined_step * tick;
        assert!(vector_approx_eq(split, combined, 1.0e-12));
    }

    #[test]
    fn coupled_thrust_artifact_identity_round_trips_and_binds_payload() {
        let closure_gate = FlatCandidateClosureInputGateEvidence {
            schema_id: "flat_gate".to_owned(),
            schema_version: 1,
            baseline_identity: EXPECTED_BASELINE_IDENTITY.to_owned(),
            sweep_identity: EXPECTED_SWEEP_IDENTITY.to_owned(),
            nominal_plant_identity: EXPECTED_NOMINAL_IDENTITY.to_owned(),
            nominal_input_manifest_identity: EXPECTED_NOMINAL_MANIFEST_IDENTITY.to_owned(),
            launch_feasibility_identity: EXPECTED_LAUNCH_IDENTITY.to_owned(),
            launch_feasibility_input_gate_identity: EXPECTED_LAUNCH_INPUT_GATE_IDENTITY.to_owned(),
            contact_audit_identity: EXPECTED_CONTACT_AUDIT_IDENTITY.to_owned(),
            contact_audit_input_gate_identity: EXPECTED_CONTACT_AUDIT_INPUT_GATE_IDENTITY
                .to_owned(),
            flat_case_id: "continuous_flat_r00".to_owned(),
            flat_probe_identity: "probe".to_owned(),
            flat_result_identity: "result".to_owned(),
            original_native_candidate_identity: EXPECTED_NATIVE_CANDIDATE_IDENTITY.to_owned(),
            original_shortest_candidate_identity: EXPECTED_SHORTEST_CANDIDATE_IDENTITY.to_owned(),
            third_candidate: super::super::FlatCandidateBindingEvidence {
                candidate_identity: EXPECTED_THIRD_CANDIDATE_IDENTITY.to_owned(),
                classification: "Certified".to_owned(),
                selection_scope: "diagnostic".to_owned(),
                source_handoff_arc_step: 1,
                source_bridge_tick_count: 2,
                coast_tick_count: 1,
                terminal_bridge_tick_count: 2,
                nominal_profile_tick_count: 5,
            },
            frozen_case_count: 6,
            frozen_selected_profile_count: 9,
            frozen_selection_role_count: 12,
            frozen_cadence_row_count: 18,
            identity: "closure_gate".to_owned(),
        };
        let mut input_gate = WaypointDirectCoupledThrustAuditInputGateEvidence {
            baseline_identity: EXPECTED_BASELINE_IDENTITY.to_owned(),
            sweep_identity: EXPECTED_SWEEP_IDENTITY.to_owned(),
            nominal_plant_identity: EXPECTED_NOMINAL_IDENTITY.to_owned(),
            nominal_input_manifest_identity: EXPECTED_NOMINAL_MANIFEST_IDENTITY.to_owned(),
            launch_feasibility_identity: EXPECTED_LAUNCH_IDENTITY.to_owned(),
            launch_feasibility_input_gate_identity: EXPECTED_LAUNCH_INPUT_GATE_IDENTITY.to_owned(),
            contact_audit_identity: EXPECTED_CONTACT_AUDIT_IDENTITY.to_owned(),
            contact_audit_input_gate_identity: EXPECTED_CONTACT_AUDIT_INPUT_GATE_IDENTITY
                .to_owned(),
            flat_candidate_closure_input_gate: closure_gate,
            flat_canary_identity: EXPECTED_FLAT_CANARY_IDENTITY.to_owned(),
            flat_canary_input_gate_identity: EXPECTED_FLAT_CANARY_INPUT_GATE_IDENTITY.to_owned(),
            candidate_identities: vec![
                EXPECTED_NATIVE_CANDIDATE_IDENTITY.to_owned(),
                EXPECTED_SHORTEST_CANDIDATE_IDENTITY.to_owned(),
                EXPECTED_THIRD_CANDIDATE_IDENTITY.to_owned(),
            ],
            identity: String::new(),
        };
        input_gate.identity = input_gate_identity(&input_gate).expect("gate identity");
        let mut artifact = WaypointDirectCoupledThrustAuditArtifact {
            schema_id: WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_SCHEMA_ID.to_owned(),
            schema_version: WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_SCHEMA_VERSION,
            characterization_id: WAYPOINT_DIRECT_COUPLED_THRUST_AUDIT_ID.to_owned(),
            input_gate,
            protocol: WaypointDirectCoupledThrustAuditProtocolEvidence {
                source_bridge_recompute_rule: "original".to_owned(),
                reseed_rule: "reseed".to_owned(),
                peak_rule: "endpoint".to_owned(),
                source_log_mapping_rule: "mapped".to_owned(),
                analytical_attribution_rule: "algebraic".to_owned(),
                no_new_flight_rule: "none".to_owned(),
            },
            candidates: Vec::new(),
            scope_non_claims: Vec::new(),
            identity: String::new(),
        };
        artifact.identity = artifact_identity(&artifact).expect("artifact identity");
        let bytes = serde_json::to_vec_pretty(&artifact).expect("serialize artifact");
        let reloaded: WaypointDirectCoupledThrustAuditArtifact =
            serde_json::from_slice(&bytes).expect("reload artifact");
        assert_eq!(
            serde_json::to_vec_pretty(&reloaded).expect("reserialize"),
            bytes
        );
        assert_eq!(
            artifact_identity(&reloaded).expect("semantic identity"),
            reloaded.identity
        );

        let original_identity = reloaded.identity.clone();
        let mut changed = reloaded;
        changed.protocol.peak_rule.push_str(" changed");
        assert_ne!(
            artifact_identity(&changed).expect("changed identity"),
            original_identity
        );
    }
}
