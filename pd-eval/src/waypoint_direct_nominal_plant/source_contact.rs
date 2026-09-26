//! Additive source-contact shadow and counterfactual continuation diagnostic.
//!
//! This evaluator lane reuses the sealed nominal-plant preparation and command
//! conversion. Its free-body continuation deliberately calls the neutral
//! physics/contact seam without mission terminalization; every result is
//! counterfactual and is not evidence of route or landing safety.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::{
    Command, ContactClassification, CorridorEnvelope, EventKind, EventRecord, RunContext,
    SimulationState, TerrainDefinition, Vec2,
};
use pd_plan::conservative_ballistic_bridge::{DirectBridgePolicyV2, MarginV2, VehicleInputV2};
use serde::{Deserialize, Serialize};

use super::{
    CommandSaturationEvidence, ContactEvidence, FirstDivergenceEvidence, NominalPhase,
    PhaseHandoffErrorEvidence, PreparedCase, PreparedInputs, PreparedProfileCandidate,
    RolloutCadence, STATE_PARITY_POSITION_TOLERANCE_M, STATE_PARITY_VELOCITY_TOLERANCE_MPS,
    SelectionRoleEvidence, ThrottleSaturation, WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_ID,
    WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_VERSION, WaypointDirectNominalPlantArtifact,
    artifact_identity as nominal_plant_identity, build_input_manifest, enum_label,
    plant_applied_throttle, prepare_inputs, read_summary, resolve_output_dir, shortest_angle_delta,
    stable_digest, throttle_request,
};

pub const WAYPOINT_DIRECT_SOURCE_CONTACT_ID: &str = "waypoint-direct-source-contact";
pub const WAYPOINT_DIRECT_SOURCE_CONTACT_SCHEMA_ID: &str = "waypoint_direct_source_contact_v1";
pub const WAYPOINT_DIRECT_SOURCE_CONTACT_SCHEMA_VERSION: u32 = 1;

const EXPECTED_NOMINAL_INPUT_MANIFEST_IDENTITY: &str = "fnv1a64:9e32aa0c1a52b28b";
const EXPECTED_NOMINAL_PLANT_IDENTITY: &str = "fnv1a64:9e8cbc902ca11fbc";
const EXPECTED_CASES: [&str; 6] = [
    "continuous_flat_r00",
    "continuous_uphill_r+30",
    "continuous_downhill_r-30",
    "center_050_width_025_height_020",
    "center_050_width_025_height_030",
    "center_050_width_025_height_040",
];
const UPRIGHT_CONTROL_TICKS: u64 = 60;
const CONTACT_V2_ENDPOINT_TOLERANCE_M: f64 = 1.0e-8;

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectSourceContactPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectSourceContactRun {
    pub artifact: WaypointDirectSourceContactArtifact,
    pub paths: WaypointDirectSourceContactPaths,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceContactInputGateEvidence {
    pub schema_id: String,
    pub schema_version: u32,
    pub baseline_identity: String,
    pub sweep_identity: String,
    pub nominal_plant_identity: String,
    pub nominal_input_manifest_identity: String,
    pub case_count: u32,
    pub unique_profile_count: u32,
    pub continuation_row_count: u32,
    pub cadence_modes: Vec<String>,
    pub cases: Vec<SourceContactInputGateCaseEvidence>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceContactInputGateCaseEvidence {
    pub case_id: String,
    pub source: String,
    pub probe_identity: String,
    pub result_identity: String,
    pub scenario_identity: String,
    pub native_v2_candidate_identity: String,
    pub research_shortest_certified_candidate_identity: String,
    pub candidate_profiles: Vec<SourceContactProfileBindingEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceContactProfileBindingEvidence {
    pub candidate_identity: String,
    pub selected_roles: Vec<String>,
    pub phase_accounting: super::ProfilePhaseAccountingEvidence,
    pub nominal_profile_tick_count: u64,
    pub nominal_profile_tick_identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectSourceContactArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub baseline_identity: String,
    pub sweep_identity: String,
    pub nominal_plant_identity: String,
    pub nominal_input_manifest_identity: String,
    pub input_gate_identity: String,
    pub protocol: SourceContactProtocolEvidence,
    pub cases: Vec<SourceContactCaseEvidence>,
    pub upright_controls: Vec<UprightControlEvidence>,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceContactProtocolEvidence {
    pub input_gate: String,
    pub clearance_mirror: String,
    pub source_disagreement_rule: String,
    pub counterfactual_continuation: String,
    pub command_rule: String,
    pub cadence_rule: String,
    pub upright_control: String,
    pub static_footprint_rule: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceContactCaseEvidence {
    pub input: super::PreparedCaseEvidence,
    pub selection_roles: Vec<SelectionRoleEvidence>,
    pub candidate_profiles: Vec<SourceContactCandidateEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceContactCandidateEvidence {
    pub candidate_identity: String,
    pub selected_roles: Vec<String>,
    pub phase_accounting: super::ProfilePhaseAccountingEvidence,
    pub continuations: Vec<FreeBodyContinuationEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FreeBodyContinuationEvidence {
    pub cadence: String,
    pub mode: String,
    pub counterfactual: bool,
    pub nominal_profile_tick_count: u64,
    pub physics_steps_advanced: u64,
    pub commanded_update_count: u64,
    pub held_command_tick_count: u64,
    pub source_contact_shadow: SourceContactShadowEvidence,
    pub first_divergence: Option<FirstDivergenceEvidence>,
    pub max_position_error_m: f64,
    pub max_velocity_error_mps: f64,
    pub phase_handoff_errors: Vec<PhaseHandoffErrorEvidence>,
    pub contact_classification_counts: ContactClassificationCounts,
    /// Every non-none contact is retained; `contact_classification_counts`
    /// accounts for `none` classifications on every other physics tick.
    pub contacts: Vec<ContactEvidence>,
    pub profile_end: SourceContactStateEvidence,
    pub saturation: CommandSaturationEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceContactShadowEvidence {
    pub evaluated_source_bridge_tick_count: u64,
    pub source_pad_center_height_tick_count: u64,
    pub rotated_hull_clearance_tick_count: u64,
    pub v2_pass_tick_count: u64,
    pub v2_fail_tick_count: u64,
    pub source_pad_margin_extrema: Option<MarginExtremaEvidence>,
    pub rotated_hull_margin_extrema: Option<MarginExtremaEvidence>,
    pub first_source_tick: Option<SourceContactTickEvidence>,
    pub first_disagreement: Option<SourceContactDisagreementEvidence>,
    pub contact_classification_counts: ContactClassificationCounts,
    pub minimum_actual_touchdown_clearance_m: f64,
    pub maximum_actual_touchdown_clearance_m: f64,
    pub minimum_actual_hull_clearance_m: f64,
    pub maximum_actual_hull_clearance_m: f64,
    pub minimum_actual_attitude_rad: f64,
    pub maximum_actual_attitude_rad: f64,
    pub maximum_absolute_actual_attitude_rad: f64,
    pub minimum_actual_angular_rate_radps: f64,
    pub maximum_actual_angular_rate_radps: f64,
    pub maximum_absolute_actual_angular_rate_radps: f64,
    pub minimum_actual_fuel_kg: f64,
    pub maximum_actual_fuel_kg: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarginExtremaEvidence {
    pub minimum_raw_margin: f64,
    pub maximum_raw_margin: f64,
    pub minimum_normalized_margin: f64,
    pub maximum_normalized_margin: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct V2ClearanceMarginEvidence {
    pub raw_margin: f64,
    pub normalized_margin: f64,
    pub passes_declared_screen: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceContactDisagreementEvidence {
    pub tick: SourceContactTickEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceContactTickEvidence {
    pub profile_tick_index: u64,
    pub physics_step: u64,
    pub clearance_mode: String,
    pub v2_margin: V2ClearanceMarginEvidence,
    pub actual_contact_classification: String,
    pub actual_contact_present: bool,
    pub actual_touchdown_clearance_m: f64,
    pub actual_hull_clearance_m: f64,
    pub actual_attitude_rad: f64,
    pub actual_angular_rate_radps: f64,
    pub actual_fuel_kg: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactClassificationCounts {
    pub total_physics_ticks: u64,
    pub no_contact: u64,
    pub stable_touchdown_on_target: u64,
    pub stable_touchdown_off_target: u64,
    pub crash: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceContactStateEvidence {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
    pub fuel_used_kg: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UprightControlEvidence {
    pub case_id: String,
    pub scenario_identity: String,
    pub mode: String,
    pub requested_physics_ticks: u64,
    pub physics_ticks_completed: u64,
    pub fixed_command: String,
    pub contact_classification_counts: ContactClassificationCounts,
    pub contacts: Vec<ContactEvidence>,
    pub samples: Vec<UprightControlSampleEvidence>,
    pub first_clear_by_frozen_attitude: Vec<FrozenAttitudeClearanceEvidence>,
    pub final_state: UprightControlFinalStateEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UprightControlSampleEvidence {
    pub physics_step: u64,
    pub contact_classification: String,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub touchdown_clearance_m: f64,
    pub hull_clearance_m: f64,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
    pub static_hull_clearances: Vec<StaticHullClearanceSampleEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticHullClearanceSampleEvidence {
    pub candidate_identity: String,
    pub first_powered_attitude_rad: f64,
    pub clearance_m: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenAttitudeClearanceEvidence {
    pub candidate_identity: String,
    pub selected_roles: Vec<String>,
    pub first_powered_attitude_rad: f64,
    pub first_strictly_clear_physics_step: Option<u64>,
    pub clearance_at_first_clear_m: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UprightControlFinalStateEvidence {
    pub state: SourceContactStateEvidence,
    pub touchdown_clearance_m: f64,
    pub hull_clearance_m: f64,
    pub minimum_touchdown_clearance_m: f64,
    pub minimum_hull_clearance_m: f64,
    pub physical_outcome: String,
    pub end_reason: String,
    pub terminated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum V2ClearanceMode {
    SourcePadCenterHeight,
    RotatedHullClearance,
}

impl V2ClearanceMode {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::SourcePadCenterHeight => "source_pad_center_height",
            Self::RotatedHullClearance => "rotated_hull_clearance",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct V2ClearanceResult {
    pub(super) mode: V2ClearanceMode,
    pub(super) margin: MarginV2,
    pub(super) passes_declared_screen: bool,
}

#[derive(Clone, Debug)]
struct MarginExtrema {
    minimum_raw: f64,
    maximum_raw: f64,
    minimum_normalized: f64,
    maximum_normalized: f64,
}

impl MarginExtrema {
    fn new(margin: MarginV2) -> Self {
        Self {
            minimum_raw: margin.raw,
            maximum_raw: margin.raw,
            minimum_normalized: margin.normalized,
            maximum_normalized: margin.normalized,
        }
    }

    fn include(&mut self, margin: MarginV2) {
        self.minimum_raw = self.minimum_raw.min(margin.raw);
        self.maximum_raw = self.maximum_raw.max(margin.raw);
        self.minimum_normalized = self.minimum_normalized.min(margin.normalized);
        self.maximum_normalized = self.maximum_normalized.max(margin.normalized);
    }

    fn evidence(&self) -> MarginExtremaEvidence {
        MarginExtremaEvidence {
            minimum_raw_margin: self.minimum_raw,
            maximum_raw_margin: self.maximum_raw,
            minimum_normalized_margin: self.minimum_normalized,
            maximum_normalized_margin: self.maximum_normalized,
        }
    }
}

/// Run the additive source-contact diagnostic after revalidating the exact
/// sealed baseline, sweep, nominal input manifest, and prior nominal artifact.
pub fn run_waypoint_direct_source_contact(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    nominal_summary_path: &Path,
    output_dir: &Path,
) -> Result<WaypointDirectSourceContactRun> {
    let (prepared, nominal) = prepare_source_contact_inputs(
        repo_root,
        baseline_summary_path,
        sweep_summary_path,
        nominal_summary_path,
    )?;
    let input_gate = source_contact_input_gate_evidence(&prepared, &nominal)?;
    let mut cases = Vec::with_capacity(prepared.cases.len());
    for (prepared_case, nominal_case) in prepared.cases.iter().zip(&nominal.cases) {
        let mut candidate_profiles = Vec::with_capacity(prepared_case.profile_candidates.len());
        for (profile_candidate, prior_profile) in prepared_case
            .profile_candidates
            .iter()
            .zip(&nominal_case.candidate_rollouts)
        {
            let direct = run_free_body_continuation(
                prepared_case,
                profile_candidate,
                &prepared.baseline.evaluated_policy,
                &prepared.baseline.vehicle,
                RolloutCadence::DirectPerTick,
            )?;
            let held = run_free_body_continuation(
                prepared_case,
                profile_candidate,
                &prepared.baseline.evaluated_policy,
                &prepared.baseline.vehicle,
                RolloutCadence::ControllerCadence,
            )?;
            if prior_profile.candidate_identity != profile_candidate.candidate_identity
                || prior_profile.selected_roles != profile_candidate.selected_roles
            {
                bail!("candidate role binding changed after source-contact input gate");
            }
            candidate_profiles.push(SourceContactCandidateEvidence {
                candidate_identity: profile_candidate.candidate_identity.clone(),
                selected_roles: profile_candidate.selected_roles.clone(),
                phase_accounting: profile_candidate.profile.accounting.clone(),
                continuations: vec![direct, held],
            });
        }
        cases.push(SourceContactCaseEvidence {
            input: prepared_case.evidence.clone(),
            selection_roles: nominal_case.selection_roles.clone(),
            candidate_profiles,
        });
    }

    let upright_controls = prepared
        .cases
        .iter()
        .map(run_upright_control)
        .collect::<Result<Vec<_>>>()?;
    let mut artifact = WaypointDirectSourceContactArtifact {
        schema_id: WAYPOINT_DIRECT_SOURCE_CONTACT_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_SOURCE_CONTACT_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_SOURCE_CONTACT_ID.to_owned(),
        baseline_identity: prepared.baseline.identity.clone(),
        sweep_identity: prepared.sweep.identity.clone(),
        nominal_plant_identity: nominal.identity,
        nominal_input_manifest_identity: nominal.input_manifest_identity,
        input_gate_identity: input_gate.identity,
        protocol: SourceContactProtocolEvidence {
            input_gate: "the sealed baseline and topology sweep were rebuilt and reevaluated; all six case, role, candidate, and materialized-profile bindings were compared with the frozen nominal input manifest and prior nominal artifact before any SimulationState was constructed".to_owned(),
            clearance_mirror: "evaluator-local mirror of the declared V2 per-tick clearance screen, applied to each nominal source-bridge post-step state with that same tick's thrust direction; this mirror is diagnostic evidence, not a new certificate".to_owned(),
            source_disagreement_rule: "a disagreement is recorded when the mirrored V2 clearance margin pass result differs from the exact post-step simulator contact classification being None; source-pad contact is retained as contact even when the V2 support-pad center-height screen passes".to_owned(),
            counterfactual_continuation: "from the unchanged scenario initial state, advance exactly the nominal profile tick count with step_physics_and_classify_contact; record every tick classification but do not apply mission contact, progress, or timeout terminalization and do not add a post-profile idle tail".to_owned(),
            command_rule: "reuse the nominal-plant command inversion and angle conversion: f=a*m/(F+a*B*dt), current pre-burn mass, plant minimum-throttle mapping, zero thrust off, sub-minimum and exact-minimum on commands use f64::MIN_POSITIVE, above-maximum clamps to one, powered target atan2(thrust_x, thrust_y), coast holds the first terminal powered direction".to_owned(),
            cadence_rule: "direct mode updates every 120 Hz physics tick; held mode updates at the scenario's 60 Hz controller ticks from the first desired tick in each interval and holds each command for two physics ticks".to_owned(),
            upright_control: "separate unchanged-simulator positive control: fixed upright zero-attitude full throttle for at most 60 physics ticks, stopping if the simulator terminates".to_owned(),
            static_footprint_rule: "after each upright-control physics step, measure exact terrain clearance of the bounding envelope of the frozen first-powered attitude's rotated hull; first clear means strictly positive clearance, with no pad exception or added clearance reserve".to_owned(),
        },
        cases,
        upright_controls,
        scope_non_claims: vec![
            "The free-body continuation is counterfactual after any contact because mission terminalization is omitted; it is not a simulator mission rollout.".to_owned(),
            "No continuation or upright-control row claims a safe launch, route, clearance certificate, landing, or mission success.".to_owned(),
            "The V2 clearance calculation is an evaluator-side diagnostic mirror and does not create or modify planner authority.".to_owned(),
            "The upright control is a separate fixed positive control and is not grafted onto any selected candidate profile.".to_owned(),
        ],
        identity: String::new(),
    };
    artifact.identity = source_contact_artifact_identity(&artifact)?;

    let output_dir = resolve_output_dir(repo_root, output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create waypoint direct source-contact output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&summary_path, &summary_bytes).with_context(|| {
        format!(
            "failed to write waypoint direct source-contact summary {}",
            summary_path.display()
        )
    })?;
    let reloaded: WaypointDirectSourceContactArtifact = serde_json::from_slice(&summary_bytes)
        .context("failed to reload waypoint direct source-contact summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes {
        bail!("waypoint direct source-contact summary is not byte-stable after reload");
    }
    if source_contact_artifact_identity(&reloaded)? != reloaded.identity {
        bail!("waypoint direct source-contact semantic identity failed round-trip check");
    }

    Ok(WaypointDirectSourceContactRun {
        artifact: reloaded,
        paths: WaypointDirectSourceContactPaths {
            output_dir,
            summary_path,
        },
    })
}

/// Validate and report the exact source/profile bindings without constructing
/// simulation state. This is the CLI's preflight-only path.
pub fn validate_waypoint_direct_source_contact_inputs(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    nominal_summary_path: &Path,
) -> Result<SourceContactInputGateEvidence> {
    let (prepared, nominal) = prepare_source_contact_inputs(
        repo_root,
        baseline_summary_path,
        sweep_summary_path,
        nominal_summary_path,
    )?;
    let mut gate = source_contact_input_gate_evidence(&prepared, &nominal)?;
    gate.identity = source_contact_input_gate_identity(&gate)?;
    let bytes = serde_json::to_vec(&gate)?;
    let reloaded: SourceContactInputGateEvidence = serde_json::from_slice(&bytes)?;
    if reloaded != gate || source_contact_input_gate_identity(&reloaded)? != reloaded.identity {
        bail!("source-contact input gate evidence failed semantic identity round-trip");
    }
    Ok(reloaded)
}

fn source_contact_input_gate_evidence(
    prepared: &PreparedInputs,
    nominal: &WaypointDirectNominalPlantArtifact,
) -> Result<SourceContactInputGateEvidence> {
    let cases = prepared
        .cases
        .iter()
        .map(|case| {
            let candidate_profiles = case
                .profile_candidates
                .iter()
                .map(|selected| {
                    Ok(SourceContactProfileBindingEvidence {
                        candidate_identity: selected.candidate_identity.clone(),
                        selected_roles: selected.selected_roles.clone(),
                        phase_accounting: selected.profile.accounting.clone(),
                        nominal_profile_tick_count: selected.profile.ticks.len() as u64,
                        nominal_profile_tick_identity: profile_tick_identity(&selected.profile)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(SourceContactInputGateCaseEvidence {
                case_id: case.evidence.id.clone(),
                source: case.evidence.source.clone(),
                probe_identity: case.evidence.probe_identity.clone(),
                result_identity: case.evidence.result_identity.clone(),
                scenario_identity: case.evidence.scenario_identity.clone(),
                native_v2_candidate_identity: case.evidence.native_v2_candidate_identity.clone(),
                research_shortest_certified_candidate_identity: case
                    .evidence
                    .research_shortest_certified_candidate_identity
                    .clone(),
                candidate_profiles,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let unique_profile_count = cases
        .iter()
        .map(|case| case.candidate_profiles.len() as u32)
        .sum::<u32>();
    let mut gate = SourceContactInputGateEvidence {
        schema_id: "waypoint_direct_source_contact_input_gate_v1".to_owned(),
        schema_version: 1,
        baseline_identity: prepared.baseline.identity.clone(),
        sweep_identity: prepared.sweep.identity.clone(),
        nominal_plant_identity: nominal.identity.clone(),
        nominal_input_manifest_identity: nominal.input_manifest_identity.clone(),
        case_count: cases.len() as u32,
        unique_profile_count,
        continuation_row_count: unique_profile_count * 2,
        cadence_modes: vec![
            "direct_per_tick_120_hz".to_owned(),
            "held_controller_60_hz".to_owned(),
        ],
        cases,
        identity: String::new(),
    };
    gate.identity = source_contact_input_gate_identity(&gate)?;
    Ok(gate)
}

#[derive(Serialize)]
struct ProfileTickIdentityInput {
    phase: &'static str,
    expected_position_m: Vec2,
    expected_velocity_mps: Vec2,
    thrust_acceleration_mps2: Vec2,
    target_attitude_rad: f64,
}

fn profile_tick_identity(profile: &super::NominalProfile) -> Result<String> {
    let input = profile
        .ticks
        .iter()
        .map(|tick| ProfileTickIdentityInput {
            phase: tick.phase.as_str(),
            expected_position_m: tick.expected_state.position_m,
            expected_velocity_mps: tick.expected_state.velocity_mps,
            thrust_acceleration_mps2: tick.thrust_acceleration_mps2,
            target_attitude_rad: tick.target_attitude_rad,
        })
        .collect::<Vec<_>>();
    stable_digest(&input)
}

fn source_contact_input_gate_identity(gate: &SourceContactInputGateEvidence) -> Result<String> {
    let mut identity_input = gate.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

pub(super) fn prepare_source_contact_inputs(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    nominal_summary_path: &Path,
) -> Result<(PreparedInputs, WaypointDirectNominalPlantArtifact)> {
    let prepared = prepare_inputs(repo_root, baseline_summary_path, sweep_summary_path)?;
    let manifest = build_input_manifest(&prepared)?;
    if manifest.identity != EXPECTED_NOMINAL_INPUT_MANIFEST_IDENTITY {
        bail!("rebuilt nominal input manifest identity is not the frozen source-contact input");
    }
    let nominal = read_summary::<WaypointDirectNominalPlantArtifact>(nominal_summary_path)?;
    if nominal_plant_identity(&nominal)? != nominal.identity
        || nominal.identity != EXPECTED_NOMINAL_PLANT_IDENTITY
    {
        bail!("prior nominal plant semantic identity is not the frozen source-contact input");
    }
    if nominal.schema_id != WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_ID
        || nominal.schema_version != WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_VERSION
        || nominal.baseline_identity != prepared.baseline.identity
        || nominal.sweep_identity != prepared.sweep.identity
        || nominal.input_manifest_identity != manifest.identity
    {
        bail!("prior nominal plant source identities or input manifest binding changed");
    }
    validate_prior_candidate_bindings(&prepared, &nominal)?;
    Ok((prepared, nominal))
}

fn validate_prior_candidate_bindings(
    prepared: &PreparedInputs,
    nominal: &WaypointDirectNominalPlantArtifact,
) -> Result<()> {
    if prepared.cases.len() != EXPECTED_CASES.len()
        || nominal.cases.len() != EXPECTED_CASES.len()
        || prepared
            .cases
            .iter()
            .map(|case| case.evidence.id.as_str())
            .ne(EXPECTED_CASES)
    {
        bail!("source-contact input gate requires the exact six frozen case rows");
    }
    let mut total_profiles = 0_usize;
    for (prepared_case, nominal_case) in prepared.cases.iter().zip(&nominal.cases) {
        if prepared_case.evidence != nominal_case.input {
            bail!(
                "prior nominal case {} no longer matches the rebuilt source/candidate identities",
                prepared_case.evidence.id
            );
        }
        let expected_roles = expected_selection_roles(prepared_case);
        if nominal_case.selection_roles != expected_roles {
            bail!(
                "prior nominal case {} role-to-candidate bindings changed",
                prepared_case.evidence.id
            );
        }
        if nominal_case.candidate_rollouts.len() != prepared_case.profile_candidates.len() {
            bail!(
                "prior nominal case {} candidate profile count changed",
                prepared_case.evidence.id
            );
        }
        for (selected, previous) in prepared_case
            .profile_candidates
            .iter()
            .zip(&nominal_case.candidate_rollouts)
        {
            if previous.candidate_identity != selected.candidate_identity
                || previous.selected_roles != selected.selected_roles
                || previous.phase_accounting != selected.profile.accounting
                || selected.profile.ticks.len() as u64
                    != selected.profile.accounting.nominal_profile_tick_count
            {
                bail!(
                    "prior nominal case {} candidate/profile materialization changed",
                    prepared_case.evidence.id
                );
            }
            if selected
                .profile
                .ticks
                .iter()
                .all(|tick| tick.thrust_acceleration_mps2.length() <= 1.0e-12)
            {
                bail!(
                    "source-contact profile {} has no frozen powered attitude",
                    selected.candidate_identity
                );
            }
            total_profiles += 1;
        }
    }
    if total_profiles != 9 {
        bail!("source-contact input gate requires nine unique selected profiles");
    }
    Ok(())
}

fn expected_selection_roles(case: &PreparedCase) -> Vec<SelectionRoleEvidence> {
    vec![
        SelectionRoleEvidence {
            role: "v2_native_selected".to_owned(),
            selection_rule:
                "retain the V2 evaluator's native diagnostic selected_candidate_identity".to_owned(),
            candidate_identity: case.evidence.native_v2_candidate_identity.clone(),
            shared_rollout_candidate_identity: case.evidence.native_v2_candidate_identity.clone(),
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
    ]
}

fn run_free_body_continuation(
    case: &PreparedCase,
    selected: &PreparedProfileCandidate,
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    cadence: RolloutCadence,
) -> Result<FreeBodyContinuationEvidence> {
    let context = RunContext::from_scenario(&case.scenario).map_err(anyhow::Error::msg)?;
    if context.sim.physics_hz != selected.profile.accounting.physics_hz
        || context.sim.controller_hz != 60
        || context.sim.control_interval_steps() != 2
    {
        bail!("scenario cadence no longer matches the frozen 120 Hz / 60 Hz protocol");
    }
    let source_pad = case.probe.source.clone();
    let source_count = selected.profile.accounting.source_bridge_sample_count;
    let profile_tick_count = selected.profile.ticks.len() as u64;
    let mut state = SimulationState::new(&context)?;
    let control_interval_steps = context.sim.control_interval_steps();
    let dt_s = context.sim.physics_dt_s();
    let mut saturation = CommandSaturationEvidence {
        commanded_update_count: 0,
        below_minimum_saturation_count: 0,
        above_maximum_saturation_count: 0,
        on_at_exact_minimum_count: 0,
        fuel_burn_capped_tick_count: 0,
        fuel_exhausted_tick_count: 0,
    };
    let mut shadow = SourceContactShadowAccumulator::new();
    let mut first_divergence = None;
    let mut max_position_error_m = 0.0_f64;
    let mut max_velocity_error_mps = 0.0_f64;
    let mut phase_handoff_errors = Vec::with_capacity(selected.profile.handoff_ticks.len());
    let mut contact_classification_counts = ContactClassificationCounts::default();
    let mut contacts = Vec::new();
    let mut held_command_tick_count = 0_u64;

    for profile_tick_index in 0..profile_tick_count {
        let physics_step = state.physics_step;
        let tick = &selected.profile.ticks[profile_tick_index as usize];
        let desired = throttle_request(
            tick.thrust_acceleration_mps2.length(),
            state.mass_kg(&context),
            context.vehicle.max_thrust_n,
            context.vehicle.max_fuel_burn_kgps,
            dt_s,
            context.vehicle.min_throttle_frac,
        )?;
        let normal_update = cadence.update_due(physics_step, control_interval_steps);
        if normal_update {
            state.set_command(Command {
                throttle_frac: desired.command_fraction,
                target_attitude_rad: tick.target_attitude_rad,
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
            held_command_tick_count += 1;
        }
        let actual_command = state.held_command;
        let held_command_mismatch = (actual_command.throttle_frac - desired.command_fraction).abs()
            > 1.0e-12
            || shortest_angle_delta(actual_command.target_attitude_rad, tick.target_attitude_rad)
                .abs()
                > 1.0e-12;
        let pre_step_fuel_kg = state.fuel_kg;
        let applied_throttle_frac = plant_applied_throttle(
            actual_command,
            context.vehicle.min_throttle_frac,
            pre_step_fuel_kg,
        );
        let fuel_burn_budget_kg = context.vehicle.max_fuel_burn_kgps * applied_throttle_frac * dt_s;
        let fuel_burn_capped =
            pre_step_fuel_kg > 0.0 && fuel_burn_budget_kg > pre_step_fuel_kg + f64::EPSILON;
        if desired.applied_fraction > 0.0 {
            if pre_step_fuel_kg <= 0.0 {
                saturation.fuel_exhausted_tick_count += 1;
            } else if fuel_burn_capped {
                saturation.fuel_burn_capped_tick_count += 1;
            }
        }
        let actual_attitude_before_step_rad = state.attitude_rad;
        let contact = state.step_physics_and_classify_contact(&context);
        contact_classification_counts.record(&contact);
        if !matches!(contact, ContactClassification::None) {
            contacts.push(ContactEvidence {
                physics_step: state.physics_step,
                kind: contact_classification_label(&contact).to_owned(),
            });
        }
        let actual_observation = state.build_observation(&context);

        if profile_tick_index < source_count {
            let mirror = mirror_v2_clearance(
                tick.expected_state,
                tick.thrust_acceleration_mps2,
                &source_pad,
                vehicle,
                policy,
                &case.scenario.world.terrain,
            );
            shadow.record(
                profile_tick_index,
                state.physics_step,
                mirror,
                &contact,
                actual_observation.touchdown_clearance_m,
                actual_observation.min_hull_clearance_m,
                state.attitude_rad,
                state.angular_rate_radps,
                state.fuel_kg,
            );
        }

        let expected = tick.expected_state;
        let position_error_m = (state.position_m - expected.position_m).length();
        let velocity_error_mps = (state.velocity_mps - expected.velocity_mps).length();
        max_position_error_m = max_position_error_m.max(position_error_m);
        max_velocity_error_mps = max_velocity_error_mps.max(velocity_error_mps);
        let diverged = position_error_m > STATE_PARITY_POSITION_TOLERANCE_M
            || velocity_error_mps > STATE_PARITY_VELOCITY_TOLERANCE_MPS;
        if diverged && first_divergence.is_none() {
            let attitude_slew_mismatch =
                shortest_angle_delta(state.attitude_rad, tick.target_attitude_rad).abs()
                    > STATE_PARITY_POSITION_TOLERANCE_M;
            let throttle_saturation_mismatch =
                (applied_throttle_frac - desired.applied_fraction).abs() > 1.0e-6;
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
                profile_tick_index,
                physics_step: state.physics_step,
                phase: tick.phase.as_str().to_owned(),
                cause,
                position_error_m,
                velocity_error_mps,
                desired_target_attitude_rad: tick.target_attitude_rad,
                actual_attitude_before_step_rad,
                actual_attitude_after_step_rad: state.attitude_rad,
                desired_applied_throttle_frac: desired.applied_fraction,
                commanded_throttle_frac: actual_command.throttle_frac,
                applied_throttle_frac,
                held_command_mismatch,
                fuel_burn_capped,
            });
        }
        for (boundary, boundary_step, expected_state) in &selected.profile.handoff_ticks {
            if state.physics_step == *boundary_step {
                phase_handoff_errors.push(PhaseHandoffErrorEvidence {
                    boundary: boundary.clone(),
                    physics_step: state.physics_step,
                    position_error_m: (state.position_m - expected_state.position_m).length(),
                    velocity_error_mps: (state.velocity_mps - expected_state.velocity_mps).length(),
                });
            }
        }
    }
    if state.physics_step != profile_tick_count
        || contact_classification_counts.total_physics_ticks != profile_tick_count
        || phase_handoff_errors.len() != selected.profile.handoff_ticks.len()
    {
        bail!("counterfactual continuation did not account for the complete nominal profile");
    }
    let (expected_command_updates, expected_held_ticks) =
        cadence_update_accounting(cadence, profile_tick_count, control_interval_steps);
    if saturation.commanded_update_count != expected_command_updates
        || held_command_tick_count != expected_held_ticks
    {
        bail!("counterfactual continuation command cadence accounting changed");
    }
    let cadence_label = cadence_name(cadence);
    Ok(FreeBodyContinuationEvidence {
        cadence: cadence_label.to_owned(),
        mode: match cadence {
            RolloutCadence::DirectPerTick => {
                "counterfactual_free_body_120_hz_direct_per_tick".to_owned()
            }
            RolloutCadence::ControllerCadence => {
                "counterfactual_free_body_60_hz_two_tick_hold".to_owned()
            }
        },
        counterfactual: true,
        nominal_profile_tick_count: profile_tick_count,
        physics_steps_advanced: state.physics_step,
        commanded_update_count: saturation.commanded_update_count,
        held_command_tick_count,
        source_contact_shadow: shadow.finish(source_count)?,
        first_divergence,
        max_position_error_m,
        max_velocity_error_mps,
        phase_handoff_errors,
        contact_classification_counts,
        contacts,
        profile_end: source_contact_state(&state, context.vehicle.initial_fuel_kg),
        saturation,
    })
}

fn run_upright_control(case: &PreparedCase) -> Result<UprightControlEvidence> {
    let context = RunContext::from_scenario(&case.scenario).map_err(anyhow::Error::msg)?;
    let mut state = SimulationState::new(&context)?;
    state.set_command(Command {
        throttle_frac: 1.0,
        target_attitude_rad: 0.0,
    });
    let frozen_attitudes = frozen_first_powered_attitudes(case)?;
    let mut first_clear = frozen_attitudes
        .iter()
        .map(|attitude| FrozenAttitudeClearanceEvidence {
            candidate_identity: attitude.candidate_identity.clone(),
            selected_roles: attitude.selected_roles.clone(),
            first_powered_attitude_rad: attitude.attitude_rad,
            first_strictly_clear_physics_step: None,
            clearance_at_first_clear_m: None,
        })
        .collect::<Vec<_>>();
    let mut counts = ContactClassificationCounts::default();
    let mut contacts = Vec::new();
    let mut samples = Vec::with_capacity(UPRIGHT_CONTROL_TICKS as usize);

    for _ in 0..UPRIGHT_CONTROL_TICKS {
        if state.is_terminal() {
            break;
        }
        let events = state.step(&context);
        let classification = contact_classification_from_events(&events);
        counts.record_label(&classification);
        if classification != "none" {
            contacts.push(ContactEvidence {
                physics_step: state.physics_step,
                kind: classification.clone(),
            });
        }
        let observation = state.build_observation(&context);
        let static_hull_clearances = frozen_attitudes
            .iter()
            .map(|attitude| {
                static_hull_clearance_m(
                    &case.scenario.world.terrain,
                    state.position_m,
                    attitude.attitude_rad,
                    &context,
                )
                .map(|clearance_m| StaticHullClearanceSampleEvidence {
                    candidate_identity: attitude.candidate_identity.clone(),
                    first_powered_attitude_rad: attitude.attitude_rad,
                    clearance_m: Some(clearance_m),
                })
                .unwrap_or_else(|| StaticHullClearanceSampleEvidence {
                    candidate_identity: attitude.candidate_identity.clone(),
                    first_powered_attitude_rad: attitude.attitude_rad,
                    clearance_m: None,
                })
            })
            .collect::<Vec<_>>();
        for (first, sample) in first_clear.iter_mut().zip(&static_hull_clearances) {
            if first.first_strictly_clear_physics_step.is_none()
                && sample.clearance_m.is_some_and(|clearance| clearance > 0.0)
            {
                first.first_strictly_clear_physics_step = Some(state.physics_step);
                first.clearance_at_first_clear_m = sample.clearance_m;
            }
        }
        samples.push(UprightControlSampleEvidence {
            physics_step: state.physics_step,
            contact_classification: classification,
            position_m: state.position_m,
            velocity_mps: state.velocity_mps,
            touchdown_clearance_m: observation.touchdown_clearance_m,
            hull_clearance_m: observation.min_hull_clearance_m,
            attitude_rad: state.attitude_rad,
            angular_rate_radps: state.angular_rate_radps,
            fuel_kg: state.fuel_kg,
            static_hull_clearances,
        });
    }
    let final_observation = state.build_observation(&context);
    Ok(UprightControlEvidence {
        case_id: case.evidence.id.clone(),
        scenario_identity: case.evidence.scenario_identity.clone(),
        mode: "unchanged_simulator_fixed_upright_full_throttle_positive_control".to_owned(),
        requested_physics_ticks: UPRIGHT_CONTROL_TICKS,
        physics_ticks_completed: state.physics_step,
        fixed_command: "throttle=1.0,target_attitude_rad=0.0; command held for the full lane"
            .to_owned(),
        contact_classification_counts: counts,
        contacts,
        samples,
        first_clear_by_frozen_attitude: first_clear,
        final_state: UprightControlFinalStateEvidence {
            state: source_contact_state(&state, context.vehicle.initial_fuel_kg),
            touchdown_clearance_m: final_observation.touchdown_clearance_m,
            hull_clearance_m: final_observation.min_hull_clearance_m,
            minimum_touchdown_clearance_m: state.min_touchdown_clearance_m,
            minimum_hull_clearance_m: state.min_hull_clearance_m,
            physical_outcome: enum_label(&state.physical_outcome),
            end_reason: enum_label(&state.end_reason),
            terminated: state.is_terminal(),
        },
    })
}

#[derive(Clone, Debug)]
struct FrozenFirstPoweredAttitude {
    candidate_identity: String,
    selected_roles: Vec<String>,
    attitude_rad: f64,
}

fn frozen_first_powered_attitudes(case: &PreparedCase) -> Result<Vec<FrozenFirstPoweredAttitude>> {
    case.profile_candidates
        .iter()
        .map(|selected| {
            selected
                .profile
                .ticks
                .iter()
                .find(|tick| {
                    tick.phase == NominalPhase::SourceBridge
                        && tick.thrust_acceleration_mps2.length() > 1.0e-12
                })
                .map(|tick| FrozenFirstPoweredAttitude {
                    candidate_identity: selected.candidate_identity.clone(),
                    selected_roles: selected.selected_roles.clone(),
                    // This is the frozen nominal-plant command angle for this
                    // exact first powered source sample.
                    attitude_rad: tick.target_attitude_rad,
                })
                .ok_or_else(|| {
                    anyhow!(
                        "selected source profile {} has no frozen first-powered attitude",
                        selected.candidate_identity
                    )
                })
        })
        .collect()
}

pub(super) fn mirror_v2_clearance(
    state: pd_plan::conservative_ballistic_bridge::KinematicStateV2,
    thrust_acceleration_mps2: Vec2,
    source_pad: &pd_plan::conservative_ballistic_bridge::PadInputV2,
    vehicle: &VehicleInputV2,
    policy: &DirectBridgePolicyV2,
    terrain: &TerrainDefinition,
) -> V2ClearanceResult {
    let thrust_magnitude = thrust_acceleration_mps2.length();
    let direction = (thrust_magnitude > 1.0e-12).then(|| {
        Vec2::new(
            thrust_acceleration_mps2.x / thrust_magnitude,
            thrust_acceleration_mps2.y / thrust_magnitude,
        )
    });
    if direction.is_some_and(|direction| {
        complete_touchdown_footprint_on_source_pad(state.position_m, direction, source_pad, vehicle)
    }) {
        let raw = state.position_m.y
            - (source_pad.surface_y_m + vehicle.geometry.touchdown_base_offset_m)
            + CONTACT_V2_ENDPOINT_TOLERANCE_M;
        let margin = if raw.is_finite() {
            MarginV2 {
                raw,
                normalized: raw / CONTACT_V2_ENDPOINT_TOLERANCE_M,
            }
        } else {
            failed_v2_margin()
        };
        return V2ClearanceResult {
            mode: V2ClearanceMode::SourcePadCenterHeight,
            passes_declared_screen: v2_margin_passes(margin, policy),
            margin,
        };
    }

    let envelope = rotated_hull_envelope(direction, vehicle, policy.minimum_clearance_m);
    let margin = match terrain.exact_point_clearance(state.position_m, envelope) {
        Ok(clearance) => {
            let raw = clearance.minimum_clearance_m;
            MarginV2 {
                raw,
                normalized: raw / policy.minimum_clearance_m.abs().max(f64::EPSILON),
            }
        }
        Err(_) => failed_v2_margin(),
    };
    V2ClearanceResult {
        mode: V2ClearanceMode::RotatedHullClearance,
        passes_declared_screen: v2_margin_passes(margin, policy),
        margin,
    }
}

fn failed_v2_margin() -> MarginV2 {
    MarginV2 {
        raw: -f64::MAX,
        normalized: -f64::MAX,
    }
}

fn v2_margin_passes(margin: MarginV2, policy: &DirectBridgePolicyV2) -> bool {
    margin.raw >= 0.0 && margin.normalized + 1.0e-12 >= policy.declared_robustness_margin
}

fn complete_touchdown_footprint_on_source_pad(
    center_m: Vec2,
    direction: Vec2,
    pad: &pd_plan::conservative_ballistic_bridge::PadInputV2,
    vehicle: &VehicleInputV2,
) -> bool {
    // This sign follows the V2 source screen's `attitude_rad(direction)`;
    // nominal command attitude is separately kept from the profile tick.
    let attitude = (-direction.x).atan2(direction.y);
    let left = center_m
        + Vec2::new(
            -vehicle.geometry.touchdown_half_span_m,
            -vehicle.geometry.touchdown_base_offset_m,
        )
        .rotated(attitude);
    let right = center_m
        + Vec2::new(
            vehicle.geometry.touchdown_half_span_m,
            -vehicle.geometry.touchdown_base_offset_m,
        )
        .rotated(attitude);
    let pad_left = pad.center_x_m - pad.width_m * 0.5;
    let pad_right = pad.center_x_m + pad.width_m * 0.5;
    left.x >= pad_left - CONTACT_V2_ENDPOINT_TOLERANCE_M
        && left.x <= pad_right + CONTACT_V2_ENDPOINT_TOLERANCE_M
        && right.x >= pad_left - CONTACT_V2_ENDPOINT_TOLERANCE_M
        && right.x <= pad_right + CONTACT_V2_ENDPOINT_TOLERANCE_M
}

fn rotated_hull_envelope(
    direction: Option<Vec2>,
    vehicle: &VehicleInputV2,
    extra_vertical_clearance_m: f64,
) -> CorridorEnvelope {
    match direction {
        Some(direction) => rotated_hull_envelope_for_attitude(
            (-direction.x).atan2(direction.y),
            vehicle.geometry.hull_width_m,
            vehicle.geometry.hull_height_m,
            extra_vertical_clearance_m,
        ),
        None => {
            let half_width = vehicle.geometry.hull_width_m * 0.5;
            let half_height = vehicle.geometry.hull_height_m * 0.5;
            let radius = half_width.hypot(half_height);
            CorridorEnvelope::new(radius, radius + extra_vertical_clearance_m)
        }
    }
}

fn rotated_hull_envelope_for_attitude(
    attitude_rad: f64,
    hull_width_m: f64,
    hull_height_m: f64,
    extra_vertical_clearance_m: f64,
) -> CorridorEnvelope {
    let half_width = hull_width_m * 0.5;
    let half_height = hull_height_m * 0.5;
    let (sin, cos) = attitude_rad.sin_cos();
    let horizontal = cos.abs() * half_width + sin.abs() * half_height;
    let vertical = sin.abs() * half_width + cos.abs() * half_height;
    CorridorEnvelope::new(horizontal, vertical + extra_vertical_clearance_m)
}

fn static_hull_clearance_m(
    terrain: &TerrainDefinition,
    position_m: Vec2,
    attitude_rad: f64,
    context: &RunContext,
) -> Option<f64> {
    let envelope = rotated_hull_envelope_for_attitude(
        attitude_rad,
        context.vehicle.geometry.hull_width_m,
        context.vehicle.geometry.hull_height_m,
        0.0,
    );
    terrain
        .exact_point_clearance(position_m, envelope)
        .ok()
        .map(|clearance| clearance.minimum_clearance_m)
}

struct SourceContactShadowAccumulator {
    source_pad_center_height_tick_count: u64,
    rotated_hull_clearance_tick_count: u64,
    v2_pass_tick_count: u64,
    v2_fail_tick_count: u64,
    pad_margin_extrema: Option<MarginExtrema>,
    hull_margin_extrema: Option<MarginExtrema>,
    first_source_tick: Option<SourceContactTickEvidence>,
    first_disagreement: Option<SourceContactDisagreementEvidence>,
    counts: ContactClassificationCounts,
    minimum_actual_touchdown_clearance_m: f64,
    maximum_actual_touchdown_clearance_m: f64,
    minimum_actual_hull_clearance_m: f64,
    maximum_actual_hull_clearance_m: f64,
    minimum_actual_attitude_rad: f64,
    maximum_actual_attitude_rad: f64,
    maximum_absolute_actual_attitude_rad: f64,
    minimum_actual_angular_rate_radps: f64,
    maximum_actual_angular_rate_radps: f64,
    maximum_absolute_actual_angular_rate_radps: f64,
    minimum_actual_fuel_kg: f64,
    maximum_actual_fuel_kg: f64,
    recorded_tick_count: u64,
}

impl SourceContactShadowAccumulator {
    fn new() -> Self {
        Self {
            source_pad_center_height_tick_count: 0,
            rotated_hull_clearance_tick_count: 0,
            v2_pass_tick_count: 0,
            v2_fail_tick_count: 0,
            pad_margin_extrema: None,
            hull_margin_extrema: None,
            first_source_tick: None,
            first_disagreement: None,
            counts: ContactClassificationCounts::default(),
            minimum_actual_touchdown_clearance_m: f64::INFINITY,
            maximum_actual_touchdown_clearance_m: f64::NEG_INFINITY,
            minimum_actual_hull_clearance_m: f64::INFINITY,
            maximum_actual_hull_clearance_m: f64::NEG_INFINITY,
            minimum_actual_attitude_rad: f64::INFINITY,
            maximum_actual_attitude_rad: f64::NEG_INFINITY,
            maximum_absolute_actual_attitude_rad: 0.0,
            minimum_actual_angular_rate_radps: f64::INFINITY,
            maximum_actual_angular_rate_radps: f64::NEG_INFINITY,
            maximum_absolute_actual_angular_rate_radps: 0.0,
            minimum_actual_fuel_kg: f64::INFINITY,
            maximum_actual_fuel_kg: f64::NEG_INFINITY,
            recorded_tick_count: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn record(
        &mut self,
        profile_tick_index: u64,
        physics_step: u64,
        clearance: V2ClearanceResult,
        contact: &ContactClassification,
        touchdown_clearance_m: f64,
        hull_clearance_m: f64,
        attitude_rad: f64,
        angular_rate_radps: f64,
        fuel_kg: f64,
    ) {
        match clearance.mode {
            V2ClearanceMode::SourcePadCenterHeight => {
                self.source_pad_center_height_tick_count += 1;
                include_margin(&mut self.pad_margin_extrema, clearance.margin);
            }
            V2ClearanceMode::RotatedHullClearance => {
                self.rotated_hull_clearance_tick_count += 1;
                include_margin(&mut self.hull_margin_extrema, clearance.margin);
            }
        }
        let passes = clearance.passes_declared_screen;
        if passes {
            self.v2_pass_tick_count += 1;
        } else {
            self.v2_fail_tick_count += 1;
        }
        self.counts.record(contact);
        self.minimum_actual_touchdown_clearance_m = self
            .minimum_actual_touchdown_clearance_m
            .min(touchdown_clearance_m);
        self.maximum_actual_touchdown_clearance_m = self
            .maximum_actual_touchdown_clearance_m
            .max(touchdown_clearance_m);
        self.minimum_actual_hull_clearance_m =
            self.minimum_actual_hull_clearance_m.min(hull_clearance_m);
        self.maximum_actual_hull_clearance_m =
            self.maximum_actual_hull_clearance_m.max(hull_clearance_m);
        self.minimum_actual_attitude_rad = self.minimum_actual_attitude_rad.min(attitude_rad);
        self.maximum_actual_attitude_rad = self.maximum_actual_attitude_rad.max(attitude_rad);
        self.maximum_absolute_actual_attitude_rad = self
            .maximum_absolute_actual_attitude_rad
            .max(attitude_rad.abs());
        self.minimum_actual_angular_rate_radps = self
            .minimum_actual_angular_rate_radps
            .min(angular_rate_radps);
        self.maximum_actual_angular_rate_radps = self
            .maximum_actual_angular_rate_radps
            .max(angular_rate_radps);
        self.maximum_absolute_actual_angular_rate_radps = self
            .maximum_absolute_actual_angular_rate_radps
            .max(angular_rate_radps.abs());
        self.minimum_actual_fuel_kg = self.minimum_actual_fuel_kg.min(fuel_kg);
        self.maximum_actual_fuel_kg = self.maximum_actual_fuel_kg.max(fuel_kg);
        let tick_evidence = SourceContactTickEvidence {
            profile_tick_index,
            physics_step,
            clearance_mode: clearance.mode.as_str().to_owned(),
            v2_margin: V2ClearanceMarginEvidence {
                raw_margin: clearance.margin.raw,
                normalized_margin: clearance.margin.normalized,
                passes_declared_screen: passes,
            },
            actual_contact_classification: contact_classification_label(contact).to_owned(),
            actual_contact_present: !matches!(contact, ContactClassification::None),
            actual_touchdown_clearance_m: touchdown_clearance_m,
            actual_hull_clearance_m: hull_clearance_m,
            actual_attitude_rad: attitude_rad,
            actual_angular_rate_radps: angular_rate_radps,
            actual_fuel_kg: fuel_kg,
        };
        if self.first_source_tick.is_none() {
            self.first_source_tick = Some(tick_evidence.clone());
        }
        if self.first_disagreement.is_none()
            && passes != matches!(contact, ContactClassification::None)
        {
            self.first_disagreement = Some(SourceContactDisagreementEvidence {
                tick: tick_evidence,
            });
        }
        self.recorded_tick_count += 1;
    }

    fn finish(self, expected_source_ticks: u64) -> Result<SourceContactShadowEvidence> {
        if self.recorded_tick_count != expected_source_ticks
            || self.counts.total_physics_ticks != expected_source_ticks
            || self.source_pad_center_height_tick_count + self.rotated_hull_clearance_tick_count
                != expected_source_ticks
            || self.first_source_tick.is_none()
        {
            bail!("source-contact shadow did not account for every source bridge tick");
        }
        ensure_finite_extrema(&[
            self.minimum_actual_touchdown_clearance_m,
            self.maximum_actual_touchdown_clearance_m,
            self.minimum_actual_hull_clearance_m,
            self.maximum_actual_hull_clearance_m,
            self.minimum_actual_attitude_rad,
            self.maximum_actual_attitude_rad,
            self.maximum_absolute_actual_attitude_rad,
            self.minimum_actual_angular_rate_radps,
            self.maximum_actual_angular_rate_radps,
            self.maximum_absolute_actual_angular_rate_radps,
            self.minimum_actual_fuel_kg,
            self.maximum_actual_fuel_kg,
        ])?;
        Ok(SourceContactShadowEvidence {
            evaluated_source_bridge_tick_count: self.recorded_tick_count,
            source_pad_center_height_tick_count: self.source_pad_center_height_tick_count,
            rotated_hull_clearance_tick_count: self.rotated_hull_clearance_tick_count,
            v2_pass_tick_count: self.v2_pass_tick_count,
            v2_fail_tick_count: self.v2_fail_tick_count,
            source_pad_margin_extrema: self.pad_margin_extrema.map(|extrema| extrema.evidence()),
            rotated_hull_margin_extrema: self.hull_margin_extrema.map(|extrema| extrema.evidence()),
            first_source_tick: self.first_source_tick,
            first_disagreement: self.first_disagreement,
            contact_classification_counts: self.counts,
            minimum_actual_touchdown_clearance_m: self.minimum_actual_touchdown_clearance_m,
            maximum_actual_touchdown_clearance_m: self.maximum_actual_touchdown_clearance_m,
            minimum_actual_hull_clearance_m: self.minimum_actual_hull_clearance_m,
            maximum_actual_hull_clearance_m: self.maximum_actual_hull_clearance_m,
            minimum_actual_attitude_rad: self.minimum_actual_attitude_rad,
            maximum_actual_attitude_rad: self.maximum_actual_attitude_rad,
            maximum_absolute_actual_attitude_rad: self.maximum_absolute_actual_attitude_rad,
            minimum_actual_angular_rate_radps: self.minimum_actual_angular_rate_radps,
            maximum_actual_angular_rate_radps: self.maximum_actual_angular_rate_radps,
            maximum_absolute_actual_angular_rate_radps: self
                .maximum_absolute_actual_angular_rate_radps,
            minimum_actual_fuel_kg: self.minimum_actual_fuel_kg,
            maximum_actual_fuel_kg: self.maximum_actual_fuel_kg,
        })
    }
}

fn include_margin(target: &mut Option<MarginExtrema>, margin: MarginV2) {
    if let Some(target) = target {
        target.include(margin);
    } else {
        *target = Some(MarginExtrema::new(margin));
    }
}

impl ContactClassificationCounts {
    fn record(&mut self, classification: &ContactClassification) {
        match classification {
            ContactClassification::None => self.no_contact += 1,
            ContactClassification::StableTouchdown { on_target: true } => {
                self.stable_touchdown_on_target += 1;
            }
            ContactClassification::StableTouchdown { on_target: false } => {
                self.stable_touchdown_off_target += 1;
            }
            ContactClassification::Crash => self.crash += 1,
        }
        self.total_physics_ticks += 1;
    }

    fn record_label(&mut self, label: &str) {
        match label {
            "none" => self.no_contact += 1,
            "stable_touchdown_on_target" => self.stable_touchdown_on_target += 1,
            "stable_touchdown_off_target" => self.stable_touchdown_off_target += 1,
            "crash" => self.crash += 1,
            _ => {}
        }
        self.total_physics_ticks += 1;
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

fn contact_classification_from_events(events: &[EventRecord]) -> String {
    events
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::TouchdownOnTarget => Some("stable_touchdown_on_target"),
            EventKind::TouchdownOffTarget => Some("stable_touchdown_off_target"),
            EventKind::Crash => Some("crash"),
            _ => None,
        })
        .unwrap_or("none")
        .to_owned()
}

fn source_contact_state(
    state: &SimulationState,
    initial_fuel_kg: f64,
) -> SourceContactStateEvidence {
    SourceContactStateEvidence {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        position_m: state.position_m,
        velocity_mps: state.velocity_mps,
        attitude_rad: state.attitude_rad,
        angular_rate_radps: state.angular_rate_radps,
        fuel_kg: state.fuel_kg,
        fuel_used_kg: (initial_fuel_kg - state.fuel_kg).max(0.0),
    }
}

fn cadence_name(cadence: RolloutCadence) -> &'static str {
    match cadence {
        RolloutCadence::DirectPerTick => "direct_per_tick_120_hz",
        RolloutCadence::ControllerCadence => "held_controller_60_hz",
    }
}

fn cadence_update_accounting(
    cadence: RolloutCadence,
    profile_tick_count: u64,
    control_interval_steps: u64,
) -> (u64, u64) {
    let commanded_updates = (0..profile_tick_count)
        .filter(|physics_step| cadence.update_due(*physics_step, control_interval_steps))
        .count() as u64;
    (commanded_updates, profile_tick_count - commanded_updates)
}

fn source_contact_artifact_identity(
    artifact: &WaypointDirectSourceContactArtifact,
) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

fn ensure_finite_extrema(values: &[f64]) -> Result<()> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        bail!("source-contact measurement contains a non-finite physical value")
    }
}

#[cfg(test)]
mod tests {
    use super::super::{CandidateRolloutEvidence, PlantStateEvidence};
    use super::*;
    use crate::waypoint_direct_primitive_analytical::build_artifact as build_baseline;
    use crate::waypoint_direct_topology_sweep::build_artifact as build_sweep;
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
            "pd-eval-source-contact-{}-{nonce}",
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

    fn nominal_stub(prepared: &PreparedInputs) -> WaypointDirectNominalPlantArtifact {
        let cases = prepared
            .cases
            .iter()
            .map(|case| super::super::NominalPlantCaseEvidence {
                input: case.evidence.clone(),
                selection_roles: expected_selection_roles(case),
                candidate_rollouts: case
                    .profile_candidates
                    .iter()
                    .map(|selected| CandidateRolloutEvidence {
                        candidate_identity: selected.candidate_identity.clone(),
                        selected_roles: selected.selected_roles.clone(),
                        phase_accounting: selected.profile.accounting.clone(),
                        direct_per_tick_120_hz: dummy_plant_rollout(
                            "direct_per_tick_120_hz_nonstandard_diagnostic",
                        ),
                        held_controller_60_hz: dummy_plant_rollout(
                            "controller_60_hz_two_tick_hold",
                        ),
                    })
                    .collect(),
            })
            .collect();
        WaypointDirectNominalPlantArtifact {
            schema_id: WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_ID.to_owned(),
            schema_version: WAYPOINT_DIRECT_NOMINAL_PLANT_SCHEMA_VERSION,
            characterization_id: super::super::WAYPOINT_DIRECT_NOMINAL_PLANT_ID.to_owned(),
            baseline_identity: prepared.baseline.identity.clone(),
            sweep_identity: prepared.sweep.identity.clone(),
            input_manifest_identity: build_input_manifest(prepared)
                .expect("manifest builds")
                .identity,
            protocol: super::super::NominalPlantProtocolEvidence {
                input_gate: "test".to_owned(),
                command_rule: "test".to_owned(),
                direct_per_tick_mode: "test".to_owned(),
                held_controller_cadence_mode: "test".to_owned(),
                post_profile_rule: "test".to_owned(),
                state_parity_position_tolerance_m: 1.0e-6,
                state_parity_velocity_tolerance_mps: 1.0e-6,
                en_route_clearance_rule: "test".to_owned(),
            },
            cases,
            scope_non_claims: Vec::new(),
            identity: String::new(),
        }
    }

    fn dummy_plant_rollout(mode: &str) -> super::super::PlantRolloutEvidence {
        super::super::PlantRolloutEvidence {
            mode: mode.to_owned(),
            status: "test".to_owned(),
            first_divergence: None,
            max_position_error_m: 0.0,
            max_velocity_error_mps: 0.0,
            phase_handoff_errors: Vec::new(),
            minimum_en_route_hull_clearance_m: None,
            global_minimum_hull_clearance_m: None,
            contacts: Vec::new(),
            profile_end: None,
            idle_fallback_start_physics_step: None,
            post_profile_held_command_tick_count: 0,
            termination: PlantStateEvidence {
                sim_time_s: 0.0,
                physics_step: 0,
                position_m: Vec2::default(),
                velocity_mps: Vec2::default(),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
                fuel_kg: 0.0,
                fuel_used_kg: 0.0,
                physical_outcome: "flying".to_owned(),
                mission_outcome: "in_progress".to_owned(),
                end_reason: "running".to_owned(),
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

    fn prepared_current_sources() -> (PathBuf, PreparedInputs) {
        let directory = source_directory();
        let prepared = prepare_inputs(
            &repo_root(),
            &directory.join("baseline.json"),
            &directory.join("sweep.json"),
        )
        .expect("sealed six-case input set prepares without physics");
        (directory, prepared)
    }

    #[test]
    fn tampered_prior_source_aborts_before_physics_or_output_creation() {
        let directory = source_directory();
        let root = repo_root();
        let prepared = prepare_inputs(
            &root,
            &directory.join("baseline.json"),
            &directory.join("sweep.json"),
        )
        .expect("the sealed analytical sources prepare before the prior artifact check");
        let mut tampered = nominal_stub(&prepared);
        tampered.identity = "fnv1a64:tampered".to_owned();
        let prior_path = directory.join("tampered-nominal.json");
        fs::write(
            &prior_path,
            serde_json::to_vec_pretty(&tampered).expect("tampered nominal source serializes"),
        )
        .expect("tampered nominal source is written");
        let output_dir = directory.join("must-not-be-created");
        let error = run_waypoint_direct_source_contact(
            &root,
            &directory.join("baseline.json"),
            &directory.join("sweep.json"),
            &prior_path,
            &output_dir,
        )
        .expect_err("tampered nominal identity is rejected at the input gate");
        assert!(error.to_string().contains("semantic identity"));
        assert!(!output_dir.exists(), "gate failure must not create output");
        fs::remove_dir_all(directory).expect("only the unique test directory is removed");
    }

    #[test]
    fn six_case_gate_binds_nine_profiles_and_eighteen_cadence_rows() {
        let (directory, prepared) = prepared_current_sources();
        let nominal = nominal_stub(&prepared);
        validate_prior_candidate_bindings(&prepared, &nominal)
            .expect("all prior case, role, candidate, and phase bindings match");
        let gate = source_contact_input_gate_evidence(&prepared, &nominal)
            .expect("gate materializes every profile binding");
        assert_eq!(gate.case_count, 6);
        assert_eq!(gate.unique_profile_count, 9);
        assert_eq!(gate.continuation_row_count, 18);
        assert_eq!(gate.cadence_modes.len(), 2);
        assert_eq!(gate.cases.len(), 6);
        assert_eq!(
            gate.cases
                .iter()
                .map(|case| case.candidate_profiles.len())
                .sum::<usize>(),
            9
        );
        for (prepared_case, gated_case) in prepared.cases.iter().zip(&gate.cases) {
            assert_eq!(gated_case.case_id, prepared_case.evidence.id);
            assert_eq!(
                gated_case.native_v2_candidate_identity,
                prepared_case.evidence.native_v2_candidate_identity
            );
            assert_eq!(
                gated_case.research_shortest_certified_candidate_identity,
                prepared_case
                    .evidence
                    .research_shortest_certified_candidate_identity
            );
            assert_eq!(
                gated_case.candidate_profiles.len(),
                prepared_case.profile_candidates.len()
            );
            for (prepared_profile, gated_profile) in prepared_case
                .profile_candidates
                .iter()
                .zip(&gated_case.candidate_profiles)
            {
                assert_eq!(
                    gated_profile.candidate_identity,
                    prepared_profile.candidate_identity
                );
                assert_eq!(
                    gated_profile.selected_roles,
                    prepared_profile.selected_roles
                );
                assert_eq!(
                    gated_profile.phase_accounting,
                    prepared_profile.profile.accounting
                );
                assert_eq!(
                    gated_profile.nominal_profile_tick_count,
                    prepared_profile.profile.ticks.len() as u64
                );
                assert!(
                    gated_profile
                        .nominal_profile_tick_identity
                        .starts_with("fnv1a64:")
                );
            }
        }
        let mut bad_roles = nominal.clone();
        bad_roles.cases[0].selection_roles[0].candidate_identity =
            "fnv1a64:wrong-role-candidate".to_owned();
        assert!(validate_prior_candidate_bindings(&prepared, &bad_roles).is_err());
        fs::remove_dir_all(directory).expect("only the unique test directory is removed");
    }

    #[test]
    fn cadence_accounting_distinguishes_per_tick_and_two_tick_hold() {
        assert_eq!(
            cadence_update_accounting(RolloutCadence::DirectPerTick, 6, 2),
            (6, 0)
        );
        assert_eq!(
            cadence_update_accounting(RolloutCadence::ControllerCadence, 6, 2),
            (3, 3)
        );
        assert_eq!(
            cadence_update_accounting(RolloutCadence::ControllerCadence, 5, 2),
            (3, 2)
        );
    }

    #[test]
    fn flat_native_first_tick_records_mirror_and_upright_control_runs_sixty_ticks() {
        let (directory, prepared) = prepared_current_sources();
        let case = prepared
            .cases
            .iter()
            .find(|case| case.evidence.id == "continuous_flat_r00")
            .expect("flat case is present");
        let native = case
            .profile_candidates
            .iter()
            .find(|profile| profile.selected_roles == ["v2_native_selected"])
            .expect("flat native profile is present");
        let context = RunContext::from_scenario(&case.scenario).expect("scenario context builds");
        let mut state = SimulationState::new(&context).expect("first-tick state builds");
        let tick = &native.profile.ticks[0];
        let desired = throttle_request(
            tick.thrust_acceleration_mps2.length(),
            state.mass_kg(&context),
            context.vehicle.max_thrust_n,
            context.vehicle.max_fuel_burn_kgps,
            context.sim.physics_dt_s(),
            context.vehicle.min_throttle_frac,
        )
        .expect("first native command conversion is valid");
        state.set_command(Command {
            throttle_frac: desired.command_fraction,
            target_attitude_rad: tick.target_attitude_rad,
        });
        let contact = state.step_physics_and_classify_contact(&context);
        assert_eq!(state.physics_step, 1);
        let mirror = mirror_v2_clearance(
            tick.expected_state,
            tick.thrust_acceleration_mps2,
            &case.probe.source,
            &prepared.baseline.vehicle,
            &prepared.baseline.evaluated_policy,
            &case.scenario.world.terrain,
        );
        assert_eq!(mirror.mode, V2ClearanceMode::SourcePadCenterHeight);
        assert!(mirror.passes_declared_screen);
        let observation = state.build_observation(&context);
        let first_tick = SourceContactTickEvidence {
            profile_tick_index: 0,
            physics_step: state.physics_step,
            clearance_mode: mirror.mode.as_str().to_owned(),
            v2_margin: V2ClearanceMarginEvidence {
                raw_margin: mirror.margin.raw,
                normalized_margin: mirror.margin.normalized,
                passes_declared_screen: mirror.passes_declared_screen,
            },
            actual_contact_classification: contact_classification_label(&contact).to_owned(),
            actual_contact_present: !matches!(contact, ContactClassification::None),
            actual_touchdown_clearance_m: observation.touchdown_clearance_m,
            actual_hull_clearance_m: observation.min_hull_clearance_m,
            actual_attitude_rad: state.attitude_rad,
            actual_angular_rate_radps: state.angular_rate_radps,
            actual_fuel_kg: state.fuel_kg,
        };
        assert_eq!(first_tick.profile_tick_index, 0);
        assert_eq!(first_tick.physics_step, 1);
        assert_eq!(first_tick.actual_contact_classification, "crash");
        assert_eq!(
            first_tick.actual_contact_present,
            first_tick.actual_contact_classification != "none"
        );
        assert!(first_tick.actual_hull_clearance_m < 0.0);
        assert!(first_tick.v2_margin.raw_margin > 0.0);
        let expected_first_step_angle =
            context.vehicle.max_rotation_rate_radps * context.sim.physics_dt_s();
        assert!(tick.target_attitude_rad < 0.0);
        assert!(tick.target_attitude_rad.abs() < expected_first_step_angle);
        assert!(state.attitude_rad < 0.0);
        assert!((state.attitude_rad - tick.target_attitude_rad).abs() < 1.0e-12);
        assert!(state.angular_rate_radps < 0.0);
        assert!(
            (state.angular_rate_radps - tick.target_attitude_rad / context.sim.physics_dt_s())
                .abs()
                < 1.0e-12
        );

        let downhill = prepared
            .cases
            .iter()
            .find(|case| case.evidence.id == "continuous_downhill_r-30")
            .expect("downhill case is present");
        let downhill_research = downhill
            .profile_candidates
            .iter()
            .find(|profile| profile.selected_roles == ["research_shortest_certified"])
            .expect("downhill research profile is present");
        let downhill_context =
            RunContext::from_scenario(&downhill.scenario).expect("downhill context builds");
        let mut downhill_state =
            SimulationState::new(&downhill_context).expect("downhill first-tick state builds");
        let downhill_tick = &downhill_research.profile.ticks[0];
        let downhill_desired = throttle_request(
            downhill_tick.thrust_acceleration_mps2.length(),
            downhill_state.mass_kg(&downhill_context),
            downhill_context.vehicle.max_thrust_n,
            downhill_context.vehicle.max_fuel_burn_kgps,
            downhill_context.sim.physics_dt_s(),
            downhill_context.vehicle.min_throttle_frac,
        )
        .expect("downhill first command conversion is valid");
        downhill_state.set_command(Command {
            throttle_frac: downhill_desired.command_fraction,
            target_attitude_rad: downhill_tick.target_attitude_rad,
        });
        let _ = downhill_state.step_physics_and_classify_contact(&downhill_context);
        let downhill_slew_limit =
            downhill_context.vehicle.max_rotation_rate_radps * downhill_context.sim.physics_dt_s();
        assert!(downhill_tick.target_attitude_rad.abs() > downhill_slew_limit);
        let expected_downhill_slew =
            downhill_tick.target_attitude_rad.signum() * downhill_slew_limit;
        assert!((downhill_state.attitude_rad - expected_downhill_slew).abs() < 1.0e-12);
        assert_eq!(
            downhill_state.angular_rate_radps.signum(),
            downhill_tick.target_attitude_rad.signum()
        );
        assert!(
            (downhill_state.angular_rate_radps
                - downhill_tick.target_attitude_rad.signum()
                    * downhill_context.vehicle.max_rotation_rate_radps)
                .abs()
                < 1.0e-12
        );

        let control = run_upright_control(case).expect("upright positive control completes");
        assert_eq!(control.requested_physics_ticks, UPRIGHT_CONTROL_TICKS);
        assert_eq!(control.physics_ticks_completed, UPRIGHT_CONTROL_TICKS);
        assert_eq!(control.samples.len() as u64, UPRIGHT_CONTROL_TICKS);
        assert_eq!(
            control.contact_classification_counts.total_physics_ticks,
            UPRIGHT_CONTROL_TICKS
        );
        assert_eq!(
            control.contact_classification_counts.no_contact,
            UPRIGHT_CONTROL_TICKS
        );
        assert!(control.contacts.is_empty());
        assert!(!control.final_state.terminated);
        assert!(control.final_state.state.fuel_used_kg > 0.0);
        let source_height_m = case.scenario.initial_state.position_m.y;
        assert!(
            (control.final_state.state.position_m.y - source_height_m - 1.013_999_77).abs()
                < 2.0e-6
        );
        assert!((control.final_state.state.velocity_mps.y - 3.992_183_06).abs() < 2.0e-6);
        assert!((control.final_state.state.fuel_kg - 6275.25).abs() < 1.0e-9);
        assert_eq!(control.first_clear_by_frozen_attitude.len(), 2);
        assert!(control.samples.iter().all(|sample| {
            sample.static_hull_clearances.len() == control.first_clear_by_frozen_attitude.len()
        }));
        fs::remove_dir_all(directory).expect("only the unique test directory is removed");
    }

    #[test]
    fn source_contact_identity_roundtrips_through_pretty_json_bytes() {
        let mut artifact = WaypointDirectSourceContactArtifact {
            schema_id: WAYPOINT_DIRECT_SOURCE_CONTACT_SCHEMA_ID.to_owned(),
            schema_version: WAYPOINT_DIRECT_SOURCE_CONTACT_SCHEMA_VERSION,
            characterization_id: WAYPOINT_DIRECT_SOURCE_CONTACT_ID.to_owned(),
            baseline_identity: "baseline".to_owned(),
            sweep_identity: "sweep".to_owned(),
            nominal_plant_identity: "nominal".to_owned(),
            nominal_input_manifest_identity: "manifest".to_owned(),
            input_gate_identity: "gate".to_owned(),
            protocol: SourceContactProtocolEvidence {
                input_gate: "gate".to_owned(),
                clearance_mirror: "mirror".to_owned(),
                source_disagreement_rule: "compare".to_owned(),
                counterfactual_continuation: "counterfactual".to_owned(),
                command_rule: "same".to_owned(),
                cadence_rule: "two modes".to_owned(),
                upright_control: "separate".to_owned(),
                static_footprint_rule: "strictly positive".to_owned(),
            },
            cases: Vec::new(),
            upright_controls: Vec::new(),
            scope_non_claims: Vec::new(),
            identity: String::new(),
        };
        artifact.identity =
            source_contact_artifact_identity(&artifact).expect("semantic identity computes");
        let bytes = serde_json::to_vec_pretty(&artifact).expect("artifact serializes");
        let reloaded: WaypointDirectSourceContactArtifact =
            serde_json::from_slice(&bytes).expect("artifact reloads");
        assert_eq!(serde_json::to_vec_pretty(&reloaded).unwrap(), bytes);
        assert_eq!(
            source_contact_artifact_identity(&reloaded).unwrap(),
            reloaded.identity
        );
    }
}
