//! Identity-bound preparation for the frozen direct-leg nominal plant pass.
//!
//! The input gate is deliberately completed before this module constructs any
//! simulation state. The six cases are rebuilt from the sealed V2 baseline
//! and topology sweep, and their stored direct probes are reevaluated before
//! later rollout work is permitted.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::{
    Command, EventKind, LandingPadSpec, RoutePlanningPolicy, RoutePlanningRequest, RunContext,
    ScenarioSpec, SimulationState, TerrainDefinition, Vec2, VehicleSpec, build_endpoint_profile,
    normalized_geometry,
};
use pd_plan::conservative_ballistic_bridge::{
    AnalyticalBridgeV2, BridgeKindV2, CertificationV2, DirectBridgeCandidateV2,
    DirectBridgePolicyV2, DirectBridgeProbeResultV2, DirectBridgeProbeV2, KinematicStateV2,
    VehicleGeometryInputV2, VehicleInputV2, evaluate_direct_bridge_case_v2,
};
use serde::{Deserialize, Serialize};

mod source_contact;
pub use source_contact::*;

mod launch_feasibility;
pub use launch_feasibility::*;

mod launch_contact_contract;
pub use launch_contact_contract::*;

mod flat_candidate_closure;
pub use flat_candidate_closure::*;

mod coupled_thrust_audit;
pub use coupled_thrust_audit::*;

mod source_duration_canary;
pub use source_duration_canary::*;

use crate::{
    waypoint_direct_characterization::{
        continuous_flat_controller_scenario, controller_scenario_for_case,
    },
    waypoint_direct_primitive_analytical::{
        CandidateEvidence, WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_ID,
        WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_SCHEMA_ID,
        WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_SCHEMA_VERSION,
        WaypointDirectPrimitiveAnalyticalArtifact, build_artifact as build_current_baseline,
    },
    waypoint_direct_topology_sweep::{
        WAYPOINT_DIRECT_TOPOLOGY_SWEEP_ID, WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_ID,
        WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_VERSION, WaypointDirectTopologySweepArtifact,
        build_artifact as build_current_sweep,
    },
};

pub const WAYPOINT_DIRECT_NOMINAL_PLANT_ID: &str = "waypoint-direct-nominal-plant";
pub const WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_ID: &str = "waypoint_direct_nominal_plant_v1";
pub const WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_VERSION: u32 = 1;

const EXPECTED_BASELINE_IDENTITY: &str = "fnv1a64:d3fa6b24336f7c05";
const EXPECTED_SWEEP_IDENTITY: &str = "fnv1a64:1bcd5a3bd6c6da01";
const EXPECTED_CASES: [&str; 6] = [
    "continuous_flat_r00",
    "continuous_uphill_r+30",
    "continuous_downhill_r-30",
    "center_050_width_025_height_020",
    "center_050_width_025_height_030",
    "center_050_width_025_height_040",
];
const STATE_PARITY_POSITION_TOLERANCE_M: f64 = 1.0e-6;
const STATE_PARITY_VELOCITY_TOLERANCE_MPS: f64 = 1.0e-6;
const PHASE_BOUNDARY_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectNominalPlantPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectNominalPlantRun {
    pub artifact: WaypointDirectNominalPlantArtifact,
    pub paths: WaypointDirectNominalPlantPaths,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectNominalPlantArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub baseline_identity: String,
    pub sweep_identity: String,
    pub input_manifest_identity: String,
    pub protocol: NominalPlantProtocolEvidence,
    pub cases: Vec<NominalPlantCaseEvidence>,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalPlantProtocolEvidence {
    pub input_gate: String,
    pub command_rule: String,
    pub direct_per_tick_mode: String,
    pub held_controller_cadence_mode: String,
    pub post_profile_rule: String,
    pub state_parity_position_tolerance_m: f64,
    pub state_parity_velocity_tolerance_mps: f64,
    pub en_route_clearance_rule: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreparedCaseEvidence {
    pub id: String,
    pub source: String,
    pub probe_identity: String,
    pub result_identity: String,
    pub scenario_identity: String,
    pub native_v2_candidate_identity: String,
    pub research_shortest_certified_candidate_identity: String,
    pub candidate_identities: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct InputManifestEvidence {
    schema_id: String,
    schema_version: u32,
    characterization_id: String,
    baseline_identity: String,
    sweep_identity: String,
    input_gate: String,
    cases: Vec<PreparedCaseEvidence>,
    identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalPlantCaseEvidence {
    pub input: PreparedCaseEvidence,
    pub selection_roles: Vec<SelectionRoleEvidence>,
    pub candidate_rollouts: Vec<CandidateRolloutEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelectionRoleEvidence {
    pub role: String,
    pub selection_rule: String,
    pub candidate_identity: String,
    pub shared_rollout_candidate_identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateRolloutEvidence {
    pub candidate_identity: String,
    pub selected_roles: Vec<String>,
    pub phase_accounting: ProfilePhaseAccountingEvidence,
    pub direct_per_tick_120_hz: PlantRolloutEvidence,
    pub held_controller_60_hz: PlantRolloutEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProfilePhaseAccountingEvidence {
    pub physics_hz: u32,
    pub source_bridge_sample_count: u64,
    pub source_bridge_tick_start: u64,
    pub source_bridge_tick_end_exclusive: u64,
    pub coast_tick_count: u64,
    pub coast_tick_start: u64,
    pub coast_tick_end_exclusive: u64,
    pub coast_arc_tick_start: u64,
    pub coast_arc_tick_end_exclusive: u64,
    pub terminal_bridge_sample_count: u64,
    pub terminal_bridge_tick_start: u64,
    pub terminal_bridge_tick_end_exclusive: u64,
    pub nominal_profile_tick_count: u64,
    pub nominal_profile_duration_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlantRolloutEvidence {
    pub mode: String,
    pub status: String,
    pub first_divergence: Option<FirstDivergenceEvidence>,
    pub max_position_error_m: f64,
    pub max_velocity_error_mps: f64,
    pub phase_handoff_errors: Vec<PhaseHandoffErrorEvidence>,
    pub minimum_en_route_hull_clearance_m: Option<f64>,
    pub global_minimum_hull_clearance_m: Option<f64>,
    pub contacts: Vec<ContactEvidence>,
    pub profile_end: Option<PlantStateEvidence>,
    pub idle_fallback_start_physics_step: Option<u64>,
    pub post_profile_held_command_tick_count: u64,
    pub termination: PlantStateEvidence,
    pub saturation: CommandSaturationEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FirstDivergenceEvidence {
    pub profile_tick_index: u64,
    pub physics_step: u64,
    pub phase: String,
    pub cause: String,
    pub position_error_m: f64,
    pub velocity_error_mps: f64,
    pub desired_target_attitude_rad: f64,
    pub actual_attitude_before_step_rad: f64,
    pub actual_attitude_after_step_rad: f64,
    pub desired_applied_throttle_frac: f64,
    pub commanded_throttle_frac: f64,
    pub applied_throttle_frac: f64,
    pub held_command_mismatch: bool,
    pub fuel_burn_capped: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhaseHandoffErrorEvidence {
    pub boundary: String,
    pub physics_step: u64,
    pub position_error_m: f64,
    pub velocity_error_mps: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactEvidence {
    pub physics_step: u64,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlantStateEvidence {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
    pub fuel_used_kg: f64,
    pub physical_outcome: String,
    pub mission_outcome: String,
    pub end_reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandSaturationEvidence {
    pub commanded_update_count: u64,
    pub below_minimum_saturation_count: u64,
    pub above_maximum_saturation_count: u64,
    pub on_at_exact_minimum_count: u64,
    pub fuel_burn_capped_tick_count: u64,
    pub fuel_exhausted_tick_count: u64,
}

#[derive(Clone, Debug)]
struct PreparedCase {
    evidence: PreparedCaseEvidence,
    probe: DirectBridgeProbeV2,
    scenario: ScenarioSpec,
    result: DirectBridgeProbeResultV2,
    native_candidate: DirectBridgeCandidateV2,
    research_candidate: DirectBridgeCandidateV2,
    profile_candidates: Vec<PreparedProfileCandidate>,
}

#[derive(Clone, Debug)]
struct PreparedProfileCandidate {
    candidate_identity: String,
    selected_roles: Vec<String>,
    profile: NominalProfile,
}

#[derive(Clone, Debug)]
struct NominalProfile {
    ticks: Vec<NominalProfileTick>,
    accounting: ProfilePhaseAccountingEvidence,
    handoff_ticks: Vec<(String, u64, KinematicStateV2)>,
    final_target_attitude_rad: f64,
}

#[derive(Clone, Debug)]
struct NominalProfileTick {
    phase: NominalPhase,
    expected_state: KinematicStateV2,
    thrust_acceleration_mps2: Vec2,
    target_attitude_rad: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NominalPhase {
    SourceBridge,
    Coast,
    TerminalBridge,
}

impl NominalPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::SourceBridge => "source_bridge",
            Self::Coast => "coast",
            Self::TerminalBridge => "terminal_bridge",
        }
    }
}

struct PreparedInputs {
    baseline: WaypointDirectPrimitiveAnalyticalArtifact,
    sweep: WaypointDirectTopologySweepArtifact,
    cases: Vec<PreparedCase>,
}

/// Validate both sealed analytical sources before materializing every selected
/// candidate profile and running both predeclared plant cadences.
pub fn run_waypoint_direct_nominal_plant(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    output_dir: &Path,
) -> Result<WaypointDirectNominalPlantRun> {
    let prepared = prepare_inputs(repo_root, baseline_summary_path, sweep_summary_path)?;
    let input_manifest = build_input_manifest(&prepared)?;

    let mut cases = Vec::with_capacity(prepared.cases.len());
    for case in &prepared.cases {
        let selection_roles = vec![
            SelectionRoleEvidence {
                role: "v2_native_selected".to_owned(),
                selection_rule:
                    "retain the V2 evaluator's native diagnostic selected_candidate_identity"
                        .to_owned(),
                candidate_identity: case.evidence.native_v2_candidate_identity.clone(),
                shared_rollout_candidate_identity: case
                    .evidence
                    .native_v2_candidate_identity
                    .clone(),
            },
            SelectionRoleEvidence {
                role: "research_shortest_certified".to_owned(),
                selection_rule:
                    "shortest certified direct candidate by total_time_s, then candidate identity"
                        .to_owned(),
                candidate_identity: case
                    .evidence
                    .research_shortest_certified_candidate_identity
                    .clone(),
                shared_rollout_candidate_identity: case
                    .evidence
                    .research_shortest_certified_candidate_identity
                    .clone(),
            },
        ];
        let mut candidate_rollouts = Vec::with_capacity(case.profile_candidates.len());
        for selected in &case.profile_candidates {
            let direct_per_tick_120_hz = run_profile(
                &case.scenario,
                &selected.profile,
                RolloutCadence::DirectPerTick,
            )?;
            let held_controller_60_hz = run_profile(
                &case.scenario,
                &selected.profile,
                RolloutCadence::ControllerCadence,
            )?;
            candidate_rollouts.push(CandidateRolloutEvidence {
                candidate_identity: selected.candidate_identity.clone(),
                selected_roles: selected.selected_roles.clone(),
                phase_accounting: selected.profile.accounting.clone(),
                direct_per_tick_120_hz,
                held_controller_60_hz,
            });
        }
        cases.push(NominalPlantCaseEvidence {
            input: case.evidence.clone(),
            selection_roles,
            candidate_rollouts,
        });
    }

    let mut artifact = WaypointDirectNominalPlantArtifact {
        schema_id: WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_NOMINAL_PLANT_ID.to_owned(),
        baseline_identity: prepared.baseline.identity.clone(),
        sweep_identity: prepared.sweep.identity.clone(),
        input_manifest_identity: input_manifest.identity,
        protocol: NominalPlantProtocolEvidence {
            input_gate: "both sealed sources rebuilt and validated; all six stored probes and candidate identities re-evaluated before the first SimulationState was created".to_owned(),
            command_rule: "At each command update use f=a*m/(F+a*B*dt), where a is requested thrust acceleration magnitude and m is current pre-burn mass; convert applied fraction through plant minimum-throttle mapping. Zero thrust is off; 0<f<min uses f64::MIN_POSITIVE; f==min also uses f64::MIN_POSITIVE; f>1 clamps to command 1. Powered target is atan2(thrust_x, thrust_y); coast points at first terminal powered direction; when no future powered direction exists retain the last target.".to_owned(),
            direct_per_tick_mode: "diagnostic_nonstandard: issue commands every 120 Hz physics tick directly to SimulationState; not the scenario's controller interface".to_owned(),
            held_controller_cadence_mode: "standard cadence diagnostic: issue only at scenario controller update ticks (60 Hz), using the first desired physics tick in each interval, and hold the command for two physics steps".to_owned(),
            post_profile_rule: "after the final nominal profile tick command zero throttle and hold the final nominal target attitude at the next normal command update; if the 60 Hz profile ends between updates, the prior two-tick command remains held for the residual physics tick. profile_end records the state immediately after exactly N nominal ticks, before any such held tail or idle fallback.".to_owned(),
            state_parity_position_tolerance_m: STATE_PARITY_POSITION_TOLERANCE_M,
            state_parity_velocity_tolerance_mps: STATE_PARITY_VELOCITY_TOLERANCE_MPS,
            en_route_clearance_rule: "minimum per-physics-tick hull clearance, sampled only while normalized route progress is within the shared endpoint-profile transition window; excludes pad corridors. This is a denser per-tick equivalent to the prior sampled en-route metric; global minimum is reported separately.".to_owned(),
        },
        cases,
        scope_non_claims: vec![
            "A failed feedforward profile is not proof of physical impossibility, direct-route infeasibility, or waypoint demand.".to_owned(),
            "No tracker or per-case terminal rescue was tuned.".to_owned(),
            "The 120 Hz direct-per-tick mode is a nonstandard diagnostic oracle, not the production controller cadence.".to_owned(),
            "A safe feedforward run is not a robust feedback-controller certificate.".to_owned(),
        ],
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;

    let output_dir = resolve_output_dir(repo_root, output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create waypoint direct nominal plant output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&summary_path, &summary_bytes).with_context(|| {
        format!(
            "failed to write waypoint direct nominal plant summary {}",
            summary_path.display()
        )
    })?;
    let reloaded: WaypointDirectNominalPlantArtifact = serde_json::from_slice(&summary_bytes)
        .context("failed to reload waypoint direct nominal plant summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes {
        bail!("waypoint direct nominal plant summary is not byte-stable after reload");
    }
    if artifact_identity(&reloaded)? != reloaded.identity {
        bail!("waypoint direct nominal plant semantic identity failed round-trip check");
    }

    Ok(WaypointDirectNominalPlantRun {
        artifact: reloaded,
        paths: WaypointDirectNominalPlantPaths {
            output_dir,
            summary_path,
        },
    })
}

fn prepare_inputs(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
) -> Result<PreparedInputs> {
    let baseline =
        read_summary::<WaypointDirectPrimitiveAnalyticalArtifact>(baseline_summary_path)?;
    let sweep = read_summary::<WaypointDirectTopologySweepArtifact>(sweep_summary_path)?;

    let current_baseline = build_current_baseline(repo_root)?;
    validate_baseline(&baseline, &current_baseline)?;
    let current_sweep = build_current_sweep(repo_root)?;
    validate_sweep(&sweep, &current_sweep, &baseline)?;

    let flat_scenario = continuous_flat_controller_scenario(repo_root)?;
    let mut cases = Vec::with_capacity(EXPECTED_CASES.len());
    for case_id in EXPECTED_CASES {
        let (source, probe, native_identity, research_identity, stored_candidates, stored_result) =
            if let Some(index) = baseline.probes.iter().position(|probe| probe.id == case_id) {
                let probe = &baseline.probes[index];
                let evidence = baseline
                    .cases
                    .iter()
                    .find(|case| case.id == case_id)
                    .ok_or_else(|| {
                        anyhow!("sealed primitive baseline is missing case {case_id}")
                    })?;
                (
                    "primitive_baseline",
                    probe,
                    evidence.v2_selected_candidate_identity.as_str(),
                    evidence
                        .research_selection
                        .as_ref()
                        .ok_or_else(|| {
                            anyhow!("case {case_id} has no certified research selection")
                        })?
                        .identity
                        .as_str(),
                    evidence.candidates.as_slice(),
                    Some(evidence.result_identity.as_str()),
                )
            } else {
                let cell = sweep
                    .cases
                    .iter()
                    .find(|cell| cell.id == case_id)
                    .ok_or_else(|| anyhow!("sealed topology sweep is missing case {case_id}"))?;
                let research = cell.direct_research_selection.as_ref().ok_or_else(|| {
                    anyhow!("topology case {case_id} has no certified direct research selection")
                })?;
                (
                    "topology_sweep",
                    &cell.probe,
                    "",
                    research.identity.as_str(),
                    cell.direct_candidates.as_slice(),
                    cell.direct_result_identity.as_deref(),
                )
            };

        let result =
            evaluate_direct_bridge_case_v2(&baseline.evaluated_policy, &baseline.vehicle, probe)
                .map_err(|error| {
                    anyhow!("stored probe {case_id} failed V2 reevaluation: {error}")
                })?;
        if let Some(expected_result) = stored_result
            && result.identity != expected_result
        {
            bail!("stored probe {case_id} V2 result identity changed");
        }
        let native_candidate = if source == "primitive_baseline" {
            let selected = result
                .candidates
                .iter()
                .find(|candidate| candidate.identity == native_identity)
                .ok_or_else(|| anyhow!("case {case_id} native candidate identity is absent"))?;
            if result.selected_candidate_identity != native_identity {
                bail!("case {case_id} native selected candidate identity changed");
            }
            selected.clone()
        } else {
            result
                .candidates
                .iter()
                .find(|candidate| candidate.identity == result.selected_candidate_identity)
                .cloned()
                .ok_or_else(|| anyhow!("case {case_id} V2 selected candidate is absent"))?
        };
        if native_candidate.classification != CertificationV2::Certified {
            bail!("case {case_id} native V2 selected candidate is not certified");
        }
        let research_candidate = result
            .candidates
            .iter()
            .filter(|candidate| candidate.classification == CertificationV2::Certified)
            .min_by(|left, right| {
                left.total_time_s
                    .unwrap_or(f64::INFINITY)
                    .total_cmp(&right.total_time_s.unwrap_or(f64::INFINITY))
                    .then_with(|| left.identity.cmp(&right.identity))
            })
            .cloned()
            .ok_or_else(|| anyhow!("case {case_id} has no certified direct candidate"))?;
        if research_candidate.identity != research_identity {
            bail!("case {case_id} shortest certified candidate identity changed");
        }
        validate_candidate_ids(case_id, stored_candidates, &result)?;

        let scenario = if source == "primitive_baseline" {
            controller_scenario_for_case(repo_root, case_id)?
        } else {
            let mut scenario = flat_scenario.clone();
            scenario.id = format!("{WAYPOINT_DIRECT_NOMINAL_PLANT_ID}_{case_id}");
            scenario.world.terrain = TerrainDefinition::Heightfield {
                points_m: probe.terrain_points_m.clone(),
            };
            scenario
        };
        validate_scenario_probe(&scenario, probe, &baseline.vehicle)?;

        cases.push(PreparedCase {
            evidence: PreparedCaseEvidence {
                id: case_id.to_owned(),
                source: source.to_owned(),
                probe_identity: stable_digest(probe)?,
                result_identity: result.identity.clone(),
                scenario_identity: stable_digest(&scenario)?,
                native_v2_candidate_identity: native_candidate.identity.clone(),
                research_shortest_certified_candidate_identity: research_candidate.identity.clone(),
                candidate_identities: result
                    .candidates
                    .iter()
                    .map(|candidate| candidate.identity.clone())
                    .collect(),
            },
            probe: probe.clone(),
            scenario,
            result,
            native_candidate,
            research_candidate,
            profile_candidates: Vec::new(),
        });
    }

    let case_ids = cases
        .iter()
        .map(|case| case.evidence.id.as_str())
        .collect::<Vec<_>>();
    if case_ids != EXPECTED_CASES {
        bail!("nominal plant case order does not exactly match the six frozen cases");
    }
    for case in &mut cases {
        case.profile_candidates = prepare_profile_candidates(
            &case.result,
            &case.native_candidate,
            &case.research_candidate,
            &case.probe,
            &baseline.evaluated_policy,
            &baseline.vehicle,
        )?;
    }
    Ok(PreparedInputs {
        baseline,
        sweep,
        cases,
    })
}

fn prepare_profile_candidates(
    result: &DirectBridgeProbeResultV2,
    native: &DirectBridgeCandidateV2,
    research: &DirectBridgeCandidateV2,
    probe: &DirectBridgeProbeV2,
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
) -> Result<Vec<PreparedProfileCandidate>> {
    let mut prepared: Vec<PreparedProfileCandidate> = Vec::with_capacity(2);
    for (role, candidate) in [
        ("v2_native_selected", native),
        ("research_shortest_certified", research),
    ] {
        if let Some(existing) = prepared
            .iter_mut()
            .find(|profile| profile.candidate_identity == candidate.identity)
        {
            existing.selected_roles.push(role.to_owned());
            continue;
        }
        let profile = materialize_profile(result, candidate, probe, policy, vehicle)?;
        prepared.push(PreparedProfileCandidate {
            candidate_identity: candidate.identity.clone(),
            selected_roles: vec![role.to_owned()],
            profile,
        });
    }
    Ok(prepared)
}

fn materialize_profile(
    result: &DirectBridgeProbeResultV2,
    candidate: &DirectBridgeCandidateV2,
    probe: &DirectBridgeProbeV2,
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
) -> Result<NominalProfile> {
    if candidate.classification != CertificationV2::Certified {
        bail!("candidate {} is not certified", candidate.identity);
    }
    let source_bridge = candidate
        .source_bridge
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} has no source bridge", candidate.identity))?;
    let terminal_bridge = candidate
        .terminal_bridge
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} has no terminal bridge", candidate.identity))?;
    let source_handoff = candidate
        .source_handoff
        .ok_or_else(|| anyhow!("candidate {} has no source handoff", candidate.identity))?;
    let terminal_handoff = candidate
        .terminal_handoff
        .ok_or_else(|| anyhow!("candidate {} has no terminal handoff", candidate.identity))?;
    let coast = candidate
        .selected_coast
        .as_ref()
        .ok_or_else(|| anyhow!("candidate {} has no selected coast", candidate.identity))?;
    if source_bridge.kind != BridgeKindV2::Source
        || terminal_bridge.kind != BridgeKindV2::Terminal
        || coast.source_handoff != source_handoff
        || coast.terminal_handoff != terminal_handoff
    {
        bail!(
            "candidate {} phase types or handoffs do not match",
            candidate.identity
        );
    }
    let source_steps = source_bridge.steps;
    let terminal_steps = terminal_bridge.steps;
    let coast_steps = terminal_handoff
        .arc_step
        .checked_sub(source_handoff.arc_step)
        .ok_or_else(|| {
            anyhow!(
                "candidate {} coast handoff order is reversed",
                candidate.identity
            )
        })?;
    if coast_steps == 0 {
        bail!(
            "candidate {} selected coast contains no physics ticks",
            candidate.identity
        );
    }
    ensure_bridge_samples(candidate, source_bridge, policy.physics_hz)?;
    ensure_bridge_samples(candidate, terminal_bridge, policy.physics_hz)?;
    let coast_duration_ticks = coast.duration_s * f64::from(policy.physics_hz);
    if !coast_duration_ticks.is_finite()
        || (coast_duration_ticks - coast_duration_ticks.round()).abs() > PHASE_BOUNDARY_TOLERANCE
        || coast_duration_ticks.round() as u64 != coast_steps
    {
        bail!(
            "candidate {} selected coast duration does not match exact arc ticks",
            candidate.identity
        );
    }

    let probe_start = KinematicStateV2 {
        position_m: probe.initial_position_m,
        velocity_mps: probe.initial_velocity_mps,
    };
    if !state_within(source_bridge.start_state, probe_start, 1.0e-8)
        || !state_within(source_bridge.end_state, source_handoff.state, 1.0e-8)
        || !state_within(terminal_bridge.start_state, terminal_handoff.state, 1.0e-8)
    {
        bail!(
            "candidate {} bridge endpoints do not match its sealed probe/handoffs",
            candidate.identity
        );
    }
    let expected_target = KinematicStateV2 {
        position_m: result.touchdown_reference_m,
        velocity_mps: Vec2::new(
            0.0,
            -policy.terminal_target_downward_speed_fraction
                * vehicle.safe_touchdown_normal_speed_mps,
        ),
    };
    if !state_within(terminal_bridge.end_state, expected_target, 1.0e-8) {
        bail!(
            "candidate {} terminal bridge does not end at its certified target state",
            candidate.identity
        );
    }

    let mut ticks = Vec::with_capacity((source_steps + coast_steps + terminal_steps) as usize);
    for sample in &source_bridge.samples {
        ticks.push(NominalProfileTick {
            phase: NominalPhase::SourceBridge,
            expected_state: sample.state_m,
            thrust_acceleration_mps2: sample.thrust_acceleration_mps2,
            target_attitude_rad: 0.0,
        });
    }

    let coast_start = ticks.len() as u64;
    let mut coast_state = coast.source_handoff.state;
    let dt_s = 1.0 / f64::from(policy.physics_hz);
    for _ in 0..coast_steps {
        coast_state.velocity_mps.y -= policy.gravity_mps2 * dt_s;
        coast_state.position_m += coast_state.velocity_mps * dt_s;
        ticks.push(NominalProfileTick {
            phase: NominalPhase::Coast,
            expected_state: coast_state,
            thrust_acceleration_mps2: Vec2::default(),
            target_attitude_rad: 0.0,
        });
    }
    if !state_within(coast_state, coast.terminal_handoff.state, 1.0e-8) {
        bail!(
            "candidate {} materialized coast does not reach its terminal handoff",
            candidate.identity
        );
    }

    let terminal_start = ticks.len() as u64;
    for sample in &terminal_bridge.samples {
        ticks.push(NominalProfileTick {
            phase: NominalPhase::TerminalBridge,
            expected_state: sample.state_m,
            thrust_acceleration_mps2: sample.thrust_acceleration_mps2,
            target_attitude_rad: 0.0,
        });
    }
    let total_steps = source_steps + coast_steps + terminal_steps;
    if ticks.len() as u64 != total_steps {
        bail!(
            "candidate {} phase sample count does not match exact tick accounting",
            candidate.identity
        );
    }

    let terminal_first_powered_direction = terminal_bridge
        .samples
        .iter()
        .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
        .map(|sample| direction_angle(sample.thrust_acceleration_mps2));
    let mut future_powered_direction = vec![None; ticks.len()];
    let mut next_direction = None;
    for index in (0..ticks.len()).rev() {
        let acceleration = ticks[index].thrust_acceleration_mps2;
        if acceleration.length() > 1.0e-12 {
            next_direction = Some(direction_angle(acceleration));
        }
        future_powered_direction[index] = next_direction;
    }
    let mut last_target = 0.0_f64;
    for index in 0..ticks.len() {
        let tick = &mut ticks[index];
        let acceleration = tick.thrust_acceleration_mps2;
        let target = if acceleration.length() > 1.0e-12 {
            direction_angle(acceleration)
        } else if tick.phase == NominalPhase::Coast {
            terminal_first_powered_direction.unwrap_or(last_target)
        } else {
            future_powered_direction[index + 1..]
                .iter()
                .flatten()
                .next()
                .copied()
                .unwrap_or(last_target)
        };
        tick.target_attitude_rad = target;
        last_target = target;
    }

    let nominal_profile_duration_s = total_steps as f64 / f64::from(policy.physics_hz);
    let accounting = ProfilePhaseAccountingEvidence {
        physics_hz: policy.physics_hz,
        source_bridge_sample_count: source_steps,
        source_bridge_tick_start: 0,
        source_bridge_tick_end_exclusive: source_steps,
        coast_tick_count: coast_steps,
        coast_tick_start: coast_start,
        coast_tick_end_exclusive: coast_start + coast_steps,
        coast_arc_tick_start: source_handoff.arc_step,
        coast_arc_tick_end_exclusive: terminal_handoff.arc_step,
        terminal_bridge_sample_count: terminal_steps,
        terminal_bridge_tick_start: terminal_start,
        terminal_bridge_tick_end_exclusive: total_steps,
        nominal_profile_tick_count: total_steps,
        nominal_profile_duration_s,
    };
    let handoff_ticks = vec![
        (
            "source_bridge_to_coast".to_owned(),
            source_steps,
            source_bridge.end_state,
        ),
        (
            "coast_to_terminal_bridge".to_owned(),
            source_steps + coast_steps,
            coast.terminal_handoff.state,
        ),
        (
            "nominal_profile_end".to_owned(),
            total_steps,
            terminal_bridge.end_state,
        ),
    ];
    let final_target_attitude_rad = ticks.last().map_or(0.0, |tick| tick.target_attitude_rad);
    Ok(NominalProfile {
        ticks,
        accounting,
        handoff_ticks,
        final_target_attitude_rad,
    })
}

fn ensure_bridge_samples(
    candidate: &DirectBridgeCandidateV2,
    bridge: &AnalyticalBridgeV2,
    physics_hz: u32,
) -> Result<()> {
    if bridge.steps == 0 || bridge.samples.len() as u64 != bridge.steps {
        bail!(
            "candidate {} bridge samples are not materialized for every tick",
            candidate.identity
        );
    }
    if (bridge.duration_s * f64::from(physics_hz) - bridge.steps as f64).abs()
        > PHASE_BOUNDARY_TOLERANCE
    {
        bail!(
            "candidate {} bridge duration differs from its exact tick count",
            candidate.identity
        );
    }
    for (index, sample) in bridge.samples.iter().enumerate() {
        if sample.tick != index as u64
            || !state_within(bridge.state_at(sample.tick + 1), sample.state_m, 1.0e-10)
        {
            bail!(
                "candidate {} bridge sample indexing or state differs at tick {index}",
                candidate.identity
            );
        }
    }
    Ok(())
}

fn state_within(left: KinematicStateV2, right: KinematicStateV2, tolerance: f64) -> bool {
    (left.position_m.x - right.position_m.x).abs() <= tolerance
        && (left.position_m.y - right.position_m.y).abs() <= tolerance
        && (left.velocity_mps.x - right.velocity_mps.x).abs() <= tolerance
        && (left.velocity_mps.y - right.velocity_mps.y).abs() <= tolerance
}

fn direction_angle(thrust_acceleration_mps2: Vec2) -> f64 {
    thrust_acceleration_mps2.x.atan2(thrust_acceleration_mps2.y)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RolloutCadence {
    DirectPerTick,
    ControllerCadence,
}

impl RolloutCadence {
    fn update_due(self, physics_step: u64, control_interval_steps: u64) -> bool {
        match self {
            Self::DirectPerTick => true,
            Self::ControllerCadence => physics_step.is_multiple_of(control_interval_steps),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ThrottleSaturation {
    None,
    BelowMinimum,
    AboveMaximum,
    ExactMinimumOnCommand,
}

#[derive(Clone, Copy, Debug)]
struct ThrottleRequest {
    applied_fraction: f64,
    command_fraction: f64,
    saturation: ThrottleSaturation,
}

#[derive(Clone, Copy, Debug)]
struct EnRouteWindow {
    source_pad_x_m: f64,
    horizontal_sign: f64,
    start_progress_m: f64,
    end_progress_m: f64,
}

fn run_profile(
    scenario: &ScenarioSpec,
    profile: &NominalProfile,
    cadence: RolloutCadence,
) -> Result<PlantRolloutEvidence> {
    let context = RunContext::from_scenario(scenario).map_err(anyhow::Error::msg)?;
    if context.sim.physics_hz != profile.accounting.physics_hz
        || context.sim.controller_hz != 60
        || context.sim.control_interval_steps() != 2
    {
        bail!("scenario cadence no longer matches the frozen 120 Hz / 60 Hz protocol");
    }
    let en_route_window = en_route_window(scenario)?;
    let mut state = SimulationState::new(&context)?;
    let control_interval_steps = context.sim.control_interval_steps();
    let dt_s = context.sim.physics_dt_s();
    let profile_tick_count = profile.ticks.len() as u64;
    let mut saturation = CommandSaturationEvidence {
        commanded_update_count: 0,
        below_minimum_saturation_count: 0,
        above_maximum_saturation_count: 0,
        on_at_exact_minimum_count: 0,
        fuel_burn_capped_tick_count: 0,
        fuel_exhausted_tick_count: 0,
    };
    let mut first_divergence = None;
    let mut max_position_error_m = 0.0_f64;
    let mut max_velocity_error_mps = 0.0_f64;
    let mut phase_handoff_errors = Vec::with_capacity(profile.handoff_ticks.len());
    let mut minimum_en_route_hull_clearance_m = None;
    let mut contacts = Vec::new();
    let mut profile_end = None;
    let mut idle_fallback_start_physics_step = None;
    let mut post_profile_held_command_tick_count = 0_u64;

    observe_en_route_clearance(
        &state,
        &context,
        en_route_window,
        &mut minimum_en_route_hull_clearance_m,
    );

    while !state.is_terminal() {
        let physics_step = state.physics_step;
        let active_profile = physics_step < profile_tick_count;
        let normal_update = cadence.update_due(physics_step, control_interval_steps);
        let desired = if active_profile {
            let tick = &profile.ticks[physics_step as usize];
            Some(throttle_request(
                tick.thrust_acceleration_mps2.length(),
                state.mass_kg(&context),
                context.vehicle.max_thrust_n,
                context.vehicle.max_fuel_burn_kgps,
                dt_s,
                context.vehicle.min_throttle_frac,
            )?)
        } else {
            None
        };
        let desired_target_attitude_rad = if active_profile {
            profile.ticks[physics_step as usize].target_attitude_rad
        } else {
            profile.final_target_attitude_rad
        };

        if !active_profile {
            if normal_update {
                idle_fallback_start_physics_step.get_or_insert(physics_step);
            } else if idle_fallback_start_physics_step.is_none() {
                post_profile_held_command_tick_count += 1;
            }
        }

        if normal_update {
            if let Some(desired) = desired {
                state.set_command(Command {
                    throttle_frac: desired.command_fraction,
                    target_attitude_rad: desired_target_attitude_rad,
                });
                saturation.commanded_update_count += 1;
                match desired.saturation {
                    ThrottleSaturation::BelowMinimum => {
                        saturation.below_minimum_saturation_count += 1;
                    }
                    ThrottleSaturation::AboveMaximum => {
                        saturation.above_maximum_saturation_count += 1;
                    }
                    ThrottleSaturation::ExactMinimumOnCommand => {
                        saturation.on_at_exact_minimum_count += 1;
                    }
                    ThrottleSaturation::None => {}
                }
            } else {
                state.set_command(Command {
                    throttle_frac: 0.0,
                    target_attitude_rad: profile.final_target_attitude_rad,
                });
            }
        }

        let actual_command = state.held_command;
        let held_command_mismatch = desired.is_some_and(|desired| {
            (actual_command.throttle_frac - desired.command_fraction).abs() > 1.0e-12
                || shortest_angle_delta(
                    actual_command.target_attitude_rad,
                    desired_target_attitude_rad,
                )
                .abs()
                    > 1.0e-12
        });
        let pre_step_fuel_kg = state.fuel_kg;
        let applied_throttle_frac = plant_applied_throttle(
            actual_command,
            context.vehicle.min_throttle_frac,
            pre_step_fuel_kg,
        );
        let fuel_burn_budget_kg = context.vehicle.max_fuel_burn_kgps * applied_throttle_frac * dt_s;
        let fuel_burn_capped =
            pre_step_fuel_kg > 0.0 && fuel_burn_budget_kg > pre_step_fuel_kg + f64::EPSILON;
        if active_profile && desired.is_some_and(|desired| desired.applied_fraction > 0.0) {
            if pre_step_fuel_kg <= 0.0 {
                saturation.fuel_exhausted_tick_count += 1;
            } else if fuel_burn_capped {
                saturation.fuel_burn_capped_tick_count += 1;
            }
        }

        let actual_attitude_before_step_rad = state.attitude_rad;
        let events = state.step(&context);
        for event in events {
            let kind = enum_label(&event.kind);
            if matches!(
                &event.kind,
                EventKind::TouchdownOnTarget | EventKind::TouchdownOffTarget | EventKind::Crash
            ) {
                contacts.push(ContactEvidence {
                    physics_step: event.physics_step,
                    kind,
                });
            }
        }
        observe_en_route_clearance(
            &state,
            &context,
            en_route_window,
            &mut minimum_en_route_hull_clearance_m,
        );

        if active_profile {
            let tick = &profile.ticks[physics_step as usize];
            let expected = tick.expected_state;
            let position_error_m = (state.position_m - expected.position_m).length();
            let velocity_error_mps = (state.velocity_mps - expected.velocity_mps).length();
            max_position_error_m = max_position_error_m.max(position_error_m);
            max_velocity_error_mps = max_velocity_error_mps.max(velocity_error_mps);
            let diverged = position_error_m > STATE_PARITY_POSITION_TOLERANCE_M
                || velocity_error_mps > STATE_PARITY_VELOCITY_TOLERANCE_MPS;
            if diverged && first_divergence.is_none() {
                let attitude_slew_mismatch =
                    shortest_angle_delta(state.attitude_rad, desired_target_attitude_rad).abs()
                        > STATE_PARITY_POSITION_TOLERANCE_M;
                let throttle_saturation_mismatch = desired.is_some_and(|desired| {
                    (applied_throttle_frac - desired.applied_fraction).abs() > 1.0e-6
                });
                let cause_count = usize::from(held_command_mismatch)
                    + usize::from(fuel_burn_capped)
                    + usize::from(attitude_slew_mismatch)
                    + usize::from(throttle_saturation_mismatch);
                let cause = if cause_count != 1 {
                    "unclassified".to_owned()
                } else if held_command_mismatch {
                    "held_command_mismatch".to_owned()
                } else if fuel_burn_capped {
                    "fuel_burn_capped".to_owned()
                } else if attitude_slew_mismatch {
                    "attitude_slew".to_owned()
                } else {
                    "throttle_saturation".to_owned()
                };
                first_divergence = Some(FirstDivergenceEvidence {
                    profile_tick_index: physics_step,
                    physics_step: state.physics_step,
                    phase: tick.phase.as_str().to_owned(),
                    cause,
                    position_error_m,
                    velocity_error_mps,
                    desired_target_attitude_rad,
                    actual_attitude_before_step_rad,
                    actual_attitude_after_step_rad: state.attitude_rad,
                    desired_applied_throttle_frac: desired
                        .map_or(0.0, |desired| desired.applied_fraction),
                    commanded_throttle_frac: actual_command.throttle_frac,
                    applied_throttle_frac,
                    held_command_mismatch,
                    fuel_burn_capped,
                });
            }
        }

        for (boundary, boundary_step, expected) in &profile.handoff_ticks {
            if state.physics_step == *boundary_step {
                phase_handoff_errors.push(PhaseHandoffErrorEvidence {
                    boundary: boundary.clone(),
                    physics_step: state.physics_step,
                    position_error_m: (state.position_m - expected.position_m).length(),
                    velocity_error_mps: (state.velocity_mps - expected.velocity_mps).length(),
                });
            }
        }
        if state.physics_step == profile_tick_count {
            profile_end = Some(plant_state_evidence(
                &state,
                context.vehicle.initial_fuel_kg,
            ));
        }
    }

    let status = if profile_end.is_none() {
        "simulator_terminated_before_profile_end"
    } else if state.physics_step == profile_tick_count {
        "simulator_terminated_at_nominal_profile_end"
    } else if idle_fallback_start_physics_step.is_some() {
        "idle_fallback_ran_until_simulator_termination"
    } else {
        "held_command_tail_terminated_before_idle_fallback"
    };

    Ok(PlantRolloutEvidence {
        mode: match cadence {
            RolloutCadence::DirectPerTick => {
                "direct_per_tick_120_hz_nonstandard_diagnostic".to_owned()
            }
            RolloutCadence::ControllerCadence => "controller_60_hz_two_tick_hold".to_owned(),
        },
        status: status.to_owned(),
        first_divergence,
        max_position_error_m,
        max_velocity_error_mps,
        phase_handoff_errors,
        minimum_en_route_hull_clearance_m,
        global_minimum_hull_clearance_m: finite_option(state.min_hull_clearance_m),
        contacts,
        profile_end,
        idle_fallback_start_physics_step,
        post_profile_held_command_tick_count,
        termination: plant_state_evidence(&state, context.vehicle.initial_fuel_kg),
        saturation,
    })
}

fn throttle_request(
    thrust_acceleration_mps2: f64,
    pre_burn_mass_kg: f64,
    max_thrust_n: f64,
    max_fuel_burn_kgps: f64,
    dt_s: f64,
    min_throttle_frac: f64,
) -> Result<ThrottleRequest> {
    if thrust_acceleration_mps2 <= 0.0 {
        return Ok(ThrottleRequest {
            applied_fraction: 0.0,
            command_fraction: 0.0,
            saturation: ThrottleSaturation::None,
        });
    }
    let denominator = max_thrust_n + thrust_acceleration_mps2 * max_fuel_burn_kgps * dt_s;
    let requested = thrust_acceleration_mps2 * pre_burn_mass_kg / denominator;
    if !requested.is_finite() || !pre_burn_mass_kg.is_finite() || pre_burn_mass_kg <= 0.0 {
        bail!("throttle inversion received non-finite or non-positive plant state");
    }
    let (command_fraction, saturation) = if requested == min_throttle_frac {
        (f64::MIN_POSITIVE, ThrottleSaturation::ExactMinimumOnCommand)
    } else if requested < min_throttle_frac {
        (f64::MIN_POSITIVE, ThrottleSaturation::BelowMinimum)
    } else if requested > 1.0 {
        (1.0, ThrottleSaturation::AboveMaximum)
    } else if min_throttle_frac >= 1.0 {
        (f64::MIN_POSITIVE, ThrottleSaturation::ExactMinimumOnCommand)
    } else {
        (
            (requested - min_throttle_frac) / (1.0 - min_throttle_frac),
            ThrottleSaturation::None,
        )
    };
    Ok(ThrottleRequest {
        applied_fraction: requested,
        command_fraction,
        saturation,
    })
}

fn plant_applied_throttle(command: Command, min_throttle_frac: f64, fuel_kg: f64) -> f64 {
    if fuel_kg <= 0.0 || command.throttle_frac <= 0.0 {
        0.0
    } else {
        min_throttle_frac + command.throttle_frac.clamp(0.0, 1.0) * (1.0 - min_throttle_frac)
    }
}

fn shortest_angle_delta(current_rad: f64, target_rad: f64) -> f64 {
    let turn = std::f64::consts::TAU;
    (target_rad - current_rad + std::f64::consts::PI).rem_euclid(turn) - std::f64::consts::PI
}

fn en_route_window(scenario: &ScenarioSpec) -> Result<EnRouteWindow> {
    let route = scenario
        .mission
        .transfer_route
        .as_ref()
        .ok_or_else(|| anyhow!("scenario {} has no transfer route", scenario.id))?;
    let request = RoutePlanningRequest {
        world: scenario.world.clone(),
        vehicle: scenario.vehicle.clone(),
        initial_state: scenario.initial_state.clone(),
        source_pad_id: route.source_pad_id.clone(),
        target_pad_id: route.target_pad_id.clone(),
        policy: RoutePlanningPolicy::default(),
    };
    let geometry = normalized_geometry(&request).map_err(|error| anyhow!(error.to_string()))?;
    let (profile, _) = build_endpoint_profile(&request, geometry.direct_horizontal_span_m)
        .map_err(|error| anyhow!(error.to_string()))?;
    let source = request
        .source_pad()
        .ok_or_else(|| anyhow!("scenario {} route source pad is missing", scenario.id))?;
    Ok(EnRouteWindow {
        source_pad_x_m: source.center_x_m,
        horizontal_sign: f64::from(geometry.horizontal_sign),
        start_progress_m: profile.source_transition_end_m,
        end_progress_m: profile.target_transition_start_m,
    })
}

fn observe_en_route_clearance(
    state: &SimulationState,
    context: &RunContext,
    window: EnRouteWindow,
    minimum: &mut Option<f64>,
) {
    let progress = (state.position_m.x - window.source_pad_x_m) * window.horizontal_sign;
    if progress >= window.start_progress_m - 1.0e-9 && progress <= window.end_progress_m + 1.0e-9 {
        let clearance = state.build_observation(context).min_hull_clearance_m;
        if clearance.is_finite() {
            *minimum = Some(minimum.map_or(clearance, |current| current.min(clearance)));
        }
    }
}

fn plant_state_evidence(state: &SimulationState, initial_fuel_kg: f64) -> PlantStateEvidence {
    PlantStateEvidence {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        position_m: state.position_m,
        velocity_mps: state.velocity_mps,
        attitude_rad: state.attitude_rad,
        angular_rate_radps: state.angular_rate_radps,
        fuel_kg: state.fuel_kg,
        fuel_used_kg: (initial_fuel_kg - state.fuel_kg).max(0.0),
        physical_outcome: enum_label(&state.physical_outcome),
        mission_outcome: enum_label(&state.mission_outcome),
        end_reason: enum_label(&state.end_reason),
    }
}

fn enum_label<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn finite_option(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn validate_baseline(
    source: &WaypointDirectPrimitiveAnalyticalArtifact,
    current: &WaypointDirectPrimitiveAnalyticalArtifact,
) -> Result<()> {
    if source.schema_id != WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_SCHEMA_ID
        || source.schema_version != WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_SCHEMA_VERSION
        || source.characterization_id != WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_ID
    {
        bail!("sealed primitive baseline schema or identity is unsupported");
    }
    if analytical_artifact_identity(source)? != source.identity {
        bail!("sealed primitive baseline semantic identity does not match its contents");
    }
    if source.identity != EXPECTED_BASELINE_IDENTITY {
        bail!("sealed primitive baseline identity is not the frozen expected identity");
    }
    if source != current || current.identity != EXPECTED_BASELINE_IDENTITY {
        bail!("sealed primitive baseline does not match the current frozen rebuild");
    }
    Ok(())
}

fn build_input_manifest(prepared: &PreparedInputs) -> Result<InputManifestEvidence> {
    let mut manifest = InputManifestEvidence {
        schema_id: WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_NOMINAL_PLANT_ID.to_owned(),
        baseline_identity: prepared.baseline.identity.clone(),
        sweep_identity: prepared.sweep.identity.clone(),
        input_gate: "passed_before_simulation".to_owned(),
        cases: prepared
            .cases
            .iter()
            .map(|case| case.evidence.clone())
            .collect(),
        identity: String::new(),
    };
    let mut identity_input = manifest.clone();
    identity_input.identity.clear();
    manifest.identity = stable_digest(&identity_input)?;
    Ok(manifest)
}

fn validate_sweep(
    source: &WaypointDirectTopologySweepArtifact,
    current: &WaypointDirectTopologySweepArtifact,
    baseline: &WaypointDirectPrimitiveAnalyticalArtifact,
) -> Result<()> {
    if source.schema_id != WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_ID
        || source.schema_version != WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_VERSION
        || source.sweep_id != WAYPOINT_DIRECT_TOPOLOGY_SWEEP_ID
    {
        bail!("sealed topology sweep schema or identity is unsupported");
    }
    if analytical_artifact_identity(source)? != source.identity {
        bail!("sealed topology sweep semantic identity does not match its contents");
    }
    if source.identity != EXPECTED_SWEEP_IDENTITY
        || source.baseline_identity != baseline.identity
        || source.baseline_input_identity != baseline.input.identity
    {
        bail!("sealed topology sweep is not bound to the frozen primitive baseline");
    }
    if source != current || current.identity != EXPECTED_SWEEP_IDENTITY {
        bail!("sealed topology sweep does not match the current frozen rebuild");
    }
    let expected_flat = baseline
        .probes
        .iter()
        .find(|probe| probe.id == "continuous_flat_r00")
        .ok_or_else(|| anyhow!("primitive baseline is missing the continuous flat probe"))?;
    if source.source_flat_probe != *expected_flat {
        bail!("topology sweep source flat probe differs from the sealed baseline");
    }
    Ok(())
}

fn validate_candidate_ids(
    case_id: &str,
    stored_candidates: &[CandidateEvidence],
    result: &DirectBridgeProbeResultV2,
) -> Result<()> {
    let reevaluated = result
        .candidates
        .iter()
        .map(candidate_evidence)
        .collect::<Vec<_>>();
    if reevaluated != stored_candidates {
        bail!("case {case_id} stored candidate identities or evidence changed on reevaluation");
    }
    Ok(())
}

fn candidate_evidence(candidate: &DirectBridgeCandidateV2) -> CandidateEvidence {
    CandidateEvidence {
        identity: candidate.identity.clone(),
        classification: candidate.classification,
        reasons: candidate.reasons.clone(),
        minimum_normalized_margin: candidate.margins.minimum_normalized(),
        total_time_s: candidate.total_time_s,
        total_fuel_burn_kg: candidate.total_fuel_burn_kg,
    }
}

fn validate_scenario_probe(
    scenario: &ScenarioSpec,
    probe: &DirectBridgeProbeV2,
    vehicle: &VehicleInputV2,
) -> Result<()> {
    scenario.validate().map_err(anyhow::Error::msg)?;
    if scenario.sim.physics_hz != 120
        || scenario.sim.controller_hz != 60
        || scenario.world.gravity_mps2 != 9.81
        || scenario.sim.max_time_s != 90.0
        || scenario.initial_state.attitude_rad != 0.0
        || scenario.initial_state.angular_rate_radps != 0.0
        || vehicle_input_v2(&scenario.vehicle) != *vehicle
        || scenario.initial_state.position_m != probe.initial_position_m
        || scenario.initial_state.velocity_mps != probe.initial_velocity_mps
    {
        bail!(
            "scenario {} does not match stored probe {} physical setup",
            scenario.id,
            probe.id
        );
    }
    let route = scenario
        .mission
        .transfer_route
        .as_ref()
        .ok_or_else(|| anyhow!("scenario {} has no direct transfer route", scenario.id))?;
    if !route.waypoints.is_empty() || route.target_pad_id != scenario.mission.goal.target_pad_id() {
        bail!(
            "scenario {} is not a direct route to its mission target",
            scenario.id
        );
    }
    let source_pad = scenario
        .world
        .landing_pad(&route.source_pad_id)
        .ok_or_else(|| anyhow!("scenario {} source pad is missing", scenario.id))?;
    let target_pad = scenario
        .world
        .landing_pad(&route.target_pad_id)
        .ok_or_else(|| anyhow!("scenario {} target pad is missing", scenario.id))?;
    if !pad_matches(source_pad, &probe.source)
        || !pad_matches(target_pad, &probe.target)
        || scenario.world.terrain.points() != probe.terrain_points_m.as_slice()
    {
        bail!(
            "scenario {} pads or terrain do not match stored probe {}",
            scenario.id,
            probe.id
        );
    }
    Ok(())
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

fn pad_matches(
    pad: &LandingPadSpec,
    input: &pd_plan::conservative_ballistic_bridge::PadInputV2,
) -> bool {
    pad.center_x_m == input.center_x_m
        && pad.surface_y_m == input.surface_y_m
        && pad.width_m == input.width_m
}

fn read_summary<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw = fs::read(path).with_context(|| {
        format!(
            "failed to read sealed analytical summary {}",
            path.display()
        )
    })?;
    serde_json::from_slice(&raw).with_context(|| {
        format!(
            "failed to parse sealed analytical summary {}",
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

fn artifact_identity(artifact: &WaypointDirectNominalPlantArtifact) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

fn analytical_artifact_identity<T: Serialize + Clone + IdentityField>(
    artifact: &T,
) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.clear_identity();
    stable_digest(&identity_input)
}

trait IdentityField {
    fn clear_identity(&mut self);
}

impl IdentityField for WaypointDirectPrimitiveAnalyticalArtifact {
    fn clear_identity(&mut self) {
        self.identity.clear();
    }
}

impl IdentityField for WaypointDirectTopologySweepArtifact {
    fn clear_identity(&mut self) {
        self.identity.clear();
    }
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
    use crate::{
        waypoint_direct_primitive_analytical::build_artifact as build_baseline,
        waypoint_direct_topology_sweep::build_artifact as build_sweep,
    };
    use pd_core::{Command, RunContext, run_simulation};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval crate is under repository root")
            .to_path_buf()
    }

    fn write_current_sources() -> PathBuf {
        let root = repo_root();
        let baseline = build_baseline(&root).expect("current baseline builds");
        let sweep = build_sweep(&root).expect("current sweep builds");
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after the epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "pd-eval-nominal-plant-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("unique test input directory is created");
        fs::write(
            directory.join("baseline.json"),
            serde_json::to_vec_pretty(&baseline).expect("baseline serializes"),
        )
        .expect("baseline test summary is written");
        fs::write(
            directory.join("sweep.json"),
            serde_json::to_vec_pretty(&sweep).expect("sweep serializes"),
        )
        .expect("sweep test summary is written");
        directory
    }

    #[test]
    fn sealed_guard_rejects_tampered_source_identities() {
        let root = repo_root();
        let baseline = build_baseline(&root).expect("current baseline builds");
        let sweep = build_sweep(&root).expect("current sweep builds");
        validate_baseline(&baseline, &baseline).expect("current baseline seal validates");
        validate_sweep(&sweep, &sweep, &baseline).expect("current sweep seal validates");

        let mut bad_baseline = baseline.clone();
        bad_baseline.identity.push_str("-tampered");
        assert!(validate_baseline(&bad_baseline, &baseline).is_err());

        let mut bad_sweep = sweep.clone();
        bad_sweep.identity.push_str("-tampered");
        assert!(validate_sweep(&bad_sweep, &sweep, &baseline).is_err());
    }

    #[test]
    fn six_case_materializer_has_both_roles_and_exact_phase_tick_accounting() {
        let directory = write_current_sources();
        let prepared = prepare_inputs(
            &repo_root(),
            &directory.join("baseline.json"),
            &directory.join("sweep.json"),
        )
        .expect("six frozen profiles materialize before simulation");
        assert_eq!(
            prepared
                .cases
                .iter()
                .map(|case| case.evidence.id.as_str())
                .collect::<Vec<_>>(),
            EXPECTED_CASES
        );
        for case in &prepared.cases {
            assert_eq!(case.evidence.candidate_identities.len(), 4);
            assert_eq!(
                case.profile_candidates.len(),
                if case.evidence.native_v2_candidate_identity
                    == case.evidence.research_shortest_certified_candidate_identity
                {
                    1
                } else {
                    2
                }
            );
            for selected in &case.profile_candidates {
                assert!(!selected.selected_roles.is_empty());
                let accounting = &selected.profile.accounting;
                assert_eq!(
                    accounting.source_bridge_tick_end_exclusive,
                    accounting.source_bridge_sample_count
                );
                assert_eq!(
                    accounting.coast_tick_start,
                    accounting.source_bridge_tick_end_exclusive
                );
                assert_eq!(
                    accounting.coast_tick_end_exclusive,
                    accounting.terminal_bridge_tick_start
                );
                assert_eq!(
                    accounting.terminal_bridge_tick_end_exclusive,
                    accounting.nominal_profile_tick_count
                );
                assert_eq!(
                    accounting.nominal_profile_tick_count,
                    accounting.source_bridge_sample_count
                        + accounting.coast_tick_count
                        + accounting.terminal_bridge_sample_count
                );
                assert_eq!(
                    selected.profile.ticks.len() as u64,
                    accounting.nominal_profile_tick_count
                );
                assert_eq!(selected.profile.handoff_ticks.len(), 3);
            }
        }
        fs::remove_dir_all(directory).expect("only the unique test input directory is removed");
    }

    #[test]
    fn throttle_inversion_covers_zero_minimum_and_saturation_edges() {
        let zero = throttle_request(0.0, 100.0, 200.0, 5.0, 0.1, 0.5)
            .expect("zero thrust request is valid");
        assert_eq!(zero.command_fraction, 0.0);
        assert_eq!(zero.applied_fraction, 0.0);

        let below = throttle_request(0.5, 100.0, 200.0, 0.0, 0.1, 0.5)
            .expect("below minimum request is valid");
        assert_eq!(below.command_fraction, f64::MIN_POSITIVE);
        assert_eq!(below.saturation, ThrottleSaturation::BelowMinimum);
        assert_eq!(
            plant_applied_throttle(
                Command {
                    throttle_frac: below.command_fraction,
                    target_attitude_rad: 0.0
                },
                0.5,
                100.0
            ),
            0.5
        );

        let exact_minimum = throttle_request(1.0, 100.0, 200.0, 0.0, 0.1, 0.5)
            .expect("exact minimum request is valid");
        assert_eq!(exact_minimum.applied_fraction, 0.5);
        assert_eq!(exact_minimum.command_fraction, f64::MIN_POSITIVE);
        assert_eq!(
            exact_minimum.saturation,
            ThrottleSaturation::ExactMinimumOnCommand
        );

        let realizable =
            throttle_request(1.5, 100.0, 200.0, 0.0, 0.1, 0.5).expect("interior request is valid");
        assert!((realizable.command_fraction - 0.5).abs() < 1.0e-12);
        assert!(
            (plant_applied_throttle(
                Command {
                    throttle_frac: realizable.command_fraction,
                    target_attitude_rad: 0.0
                },
                0.5,
                100.0
            ) - 0.75)
                .abs()
                < 1.0e-12
        );

        let above = throttle_request(4.0, 100.0, 200.0, 0.0, 0.1, 0.5)
            .expect("above maximum request is valid");
        assert_eq!(above.command_fraction, 1.0);
        assert_eq!(above.saturation, ThrottleSaturation::AboveMaximum);
    }

    #[test]
    fn held_mode_update_and_two_tick_command_hold_match_simulation_runner() {
        let mut scenario = continuous_flat_controller_scenario(&repo_root())
            .expect("continuous flat scenario builds");
        scenario.initial_state.position_m.y += 1000.0;
        scenario.sim.max_time_s = 4.0 / 120.0;
        scenario.sim.sample_hz = Some(120);
        let context =
            RunContext::from_scenario(&scenario).expect("short cadence scenario validates");
        let mut runner_updates = Vec::new();
        let runner = run_simulation(&context, "cadence-test", |_context, observation| {
            runner_updates.push(observation.physics_step);
            Command {
                throttle_frac: if observation.physics_step == 0 {
                    0.5
                } else {
                    0.25
                },
                target_attitude_rad: 0.0,
            }
        })
        .expect("authoritative simulation runner completes four steps");
        assert_eq!(runner.manifest.physics_steps, 4);
        assert_eq!(runner_updates, vec![0, 2]);

        let mut state = SimulationState::new(&context).expect("custom state initializes");
        let mut custom_updates = Vec::new();
        let mut custom_holds = Vec::new();
        while !state.is_terminal() {
            if RolloutCadence::ControllerCadence
                .update_due(state.physics_step, context.sim.control_interval_steps())
            {
                custom_updates.push(state.physics_step);
                state.set_command(Command {
                    throttle_frac: if state.physics_step == 0 { 0.5 } else { 0.25 },
                    target_attitude_rad: 0.0,
                });
            }
            custom_holds.push((state.physics_step, state.held_command.throttle_frac));
            state.step(&context);
        }
        let mut runner_holds = Vec::new();
        let mut runner_command = Command::idle();
        let mut next_action = 0;
        for physics_step in 0..4 {
            while runner
                .actions
                .get(next_action)
                .is_some_and(|action| action.physics_step == physics_step)
            {
                runner_command = runner.actions[next_action].command;
                next_action += 1;
            }
            runner_holds.push((physics_step, runner_command.throttle_frac));
        }
        assert_eq!(custom_updates, runner_updates);
        assert_eq!(custom_holds, runner_holds);
        assert_eq!(custom_holds, vec![(0, 0.5), (1, 0.5), (2, 0.25), (3, 0.25)]);
    }

    #[test]
    fn semantic_artifact_roundtrip_is_byte_stable() {
        let mut artifact = WaypointDirectNominalPlantArtifact {
            schema_id: WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_ID.to_owned(),
            schema_version: WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_VERSION,
            characterization_id: WAYPOINT_DIRECT_NOMINAL_PLANT_ID.to_owned(),
            baseline_identity: EXPECTED_BASELINE_IDENTITY.to_owned(),
            sweep_identity: EXPECTED_SWEEP_IDENTITY.to_owned(),
            input_manifest_identity: "fnv1a64:9e32aa0c1a52b28b".to_owned(),
            protocol: NominalPlantProtocolEvidence {
                input_gate: "passed".to_owned(),
                command_rule: "frozen".to_owned(),
                direct_per_tick_mode: "diagnostic".to_owned(),
                held_controller_cadence_mode: "60hz".to_owned(),
                post_profile_rule: "idle hold".to_owned(),
                state_parity_position_tolerance_m: STATE_PARITY_POSITION_TOLERANCE_M,
                state_parity_velocity_tolerance_mps: STATE_PARITY_VELOCITY_TOLERANCE_MPS,
                en_route_clearance_rule: "profile window".to_owned(),
            },
            cases: Vec::new(),
            scope_non_claims: Vec::new(),
            identity: String::new(),
        };
        artifact.identity = artifact_identity(&artifact).expect("artifact identity computes");
        let bytes = serde_json::to_vec_pretty(&artifact).expect("artifact serializes");
        let reloaded: WaypointDirectNominalPlantArtifact =
            serde_json::from_slice(&bytes).expect("artifact reloads");
        assert_eq!(serde_json::to_vec_pretty(&reloaded).unwrap(), bytes);
        assert_eq!(artifact_identity(&reloaded).unwrap(), reloaded.identity);
    }
}
