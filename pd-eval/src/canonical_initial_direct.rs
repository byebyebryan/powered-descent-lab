//! Opt-in, create-only evaluator harness for the sealed canonical-initial
//! source-rest, terrain-twin, obstacle-discrimination, and live-state gates.

mod old_policy;

use std::{fs, path::Path, time::Instant};

use anyhow::{Context, Result, bail};
use pd_core::{
    EndReason, FlightProgramBindingV1, FlightProgramUpdateV1, FlightProgramV1, MissionOutcome,
    PhysicalOutcome, RunArtifacts, RunContext, SimulationState, SimulationStateSnapshotV1,
    TerrainDefinition, Vec2, replay_simulation,
};
use serde::{Deserialize, Serialize};

use crate::{
    AirborneDirectAuditV1, AirborneDirectProposalV1, AirborneDirectSearchV1,
    BodyAwareTerminalPolicyV1, CanonicalInitialProposalV1, CanonicalInitialSearchV1,
    NominalDirectFlightComputeV1, NominalDirectFlightDecisionV1,
    NominalDirectFlightExecutionEvidenceV1, NominalDirectFlightSourceBindingV1,
    NominalDirectFlightSourceFileV1, WaypointDirectNominalDirectGenerationPolicyV1,
    WaypointDirectNominalDirectGenerationRequest, audit_airborne_direct_proposal,
    audit_canonical_initial_direct, evaluate_airborne_nominal_direct,
    evaluate_canonical_initial_direct, evaluate_nominal_direct_flight,
    execute_nominal_direct_flight_program, load_nominal_direct_operational_fresh_inputs,
    load_waypoint_direct_obstacle_discrimination_fresh_manifest,
    nominal_airborne_direct::{
        NominalAirborneStitchedReplayV1, collect_baseline_captures, construct_terrain_twin,
        replay_stitched_from_source, run_stitched_replay,
    },
    nominal_direct_flight::{reserve_output_root, write_create_only},
    nominal_direct_flight_gate::source_binding as recursive_source_binding,
    nominal_direct_flight_identity,
    waypoint_direct_body_aware_terminal::sha256_bytes,
};

pub const CANONICAL_INITIAL_DIRECT_CANARY_PROTOCOL: &str =
    "docs/canonical_initial_direct_canary_protocol.md";
pub const CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_MANIFEST: &str =
    "fixtures/research/canonical_initial_direct_canary_inputs_v1.json";
pub const CANONICAL_INITIAL_DIRECT_CANARY_SCHEMA_ID: &str = "canonical_initial_direct_canary_v1";
pub const CANONICAL_INITIAL_DIRECT_CANARY_PROTOCOL_SHA256: &str =
    "cf6943eb1ba14df1203777829009f7b3ab67aceab8e7f2c5b6122084e91cc2f8";
pub const CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_SHA256: &str =
    "5d89ccd2025f67509c757b8c6dd36a5f99d47babd6c0ca826ef68ccc7f46139d";
pub const CANONICAL_INITIAL_DIRECT_CANARY_ADDITIONAL_MANIFEST: &str =
    "fixtures/research/nominal_direct_terminal_completion_fresh_inputs_v1.json";
pub const CANONICAL_INITIAL_DIRECT_CANARY_ADDITIONAL_SHA256: &str =
    "57287ac683363b1e76fdfbea0486460a0df5b95f1af88e9ee31ee8ee8fdf716a";
pub const CANONICAL_INITIAL_DIRECT_CANARY_OBSTACLE_MANIFEST: &str =
    "fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json";
pub const CANONICAL_INITIAL_DIRECT_CANARY_OBSTACLE_SHA256: &str =
    "647df7bc94d02a9b48b773c45159e0ffa15bfca232a13b4e21164e2251a8f5b4";
const NOMINAL_DIRECT_OPERATIONAL_FRESH_MANIFEST: &str =
    "fixtures/research/nominal_direct_operational_fresh_inputs_v1.json";
const NOMINAL_DIRECT_OPERATIONAL_FRESH_SHA256: &str =
    "6df19fbd3ebb85d5a7f4a783398aaaa48e50c74a9de49454161b78c8e47658f8";

const SOURCE_CRATE_INPUTS: [&str; 5] = [
    CANONICAL_INITIAL_DIRECT_CANARY_PROTOCOL,
    CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_MANIFEST,
    NOMINAL_DIRECT_OPERATIONAL_FRESH_MANIFEST,
    CANONICAL_INITIAL_DIRECT_CANARY_ADDITIONAL_MANIFEST,
    CANONICAL_INITIAL_DIRECT_CANARY_OBSTACLE_MANIFEST,
];

const GATE_IDS: [&str; 6] = [
    "development_source_rest",
    "development_terrain_twins",
    "900m_late_broad_discriminator",
    "airborne_live_state_compatibility",
    "additional_physical_endpoints",
    "final_source_closure",
];

const DEVELOPMENT_CASE_IDS: [&str; 4] = [
    "operational_flat_span_685",
    "operational_flat_span_845",
    "operational_uphill_span_845",
    "operational_downhill_span_845",
];

const ADDITIONAL_CASE_IDS: [&str; 4] = [
    "completion_flat_span_735",
    "completion_flat_span_915",
    "completion_uphill_span_915",
    "completion_downhill_span_915",
];

const CANONICAL_SOURCE_OFFSETS: [i64; 5] = [-240, -180, -120, -60, 0];
const CANONICAL_TERMINAL_OFFSETS: [u64; 7] = [0, 60, 120, 180, 240, 300, 360];
const CANONICAL_MAXIMUM_OUTER_ATTEMPTS: usize = 140;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryInputHashV1 {
    pub relative_path: String,
    pub expected_sha256: String,
    pub observed_sha256: Option<String>,
    pub matches_seal: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryGateV1 {
    pub gate_id: String,
    /// `passed`, `failed`, or `not_evaluated` for the source-rest baseline only;
    /// downstream terrain/capture gates have independent status records.
    pub status: String,
    pub passed: Option<bool>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryReplayV1 {
    pub final_state: Option<SimulationStateSnapshotV1>,
    pub incoming_contact: Option<pd_core::IncomingContactV1>,
    pub actions: Vec<pd_core::ActionLogEntry>,
    pub events: Vec<pd_core::EventRecord>,
    pub samples: Vec<pd_core::SampleRecord>,
    pub official_replay: Option<RunArtifacts>,
    pub official_actions_match: Option<bool>,
    pub official_events_match: Option<bool>,
    pub official_samples_match: Option<bool>,
    pub final_state_matches_audit: Option<bool>,
    pub full_incoming_contact_matches_audit: Option<bool>,
    pub official_final_clock_matches: Option<bool>,
    pub passed: bool,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanarySourceDepartureV1 {
    pub starts_at_bound_source_rest: bool,
    pub handoff_is_after_initial_state: bool,
    pub handoff_is_forward: bool,
    pub handoff_is_ascending: bool,
    pub selected_commands_reached_audit_endpoint: bool,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryFirstConflictV1 {
    pub physics_step: u64,
    pub phase: String,
    pub position_m: Vec2,
    pub state_snapshot: Option<SimulationStateSnapshotV1>,
    pub incoming_contact: Option<pd_core::IncomingContactV1>,
    pub clearance_m: Option<f64>,
    pub required_clearance_m: Option<f64>,
    pub clearance_reason: Option<String>,
    pub contact_classification: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryTerrainTwinV1 {
    pub case_id: String,
    pub twin_id: String,
    pub kind: String,
    pub status: String,
    pub request: Option<WaypointDirectNominalDirectGenerationRequest>,
    pub source_binding_before_sha256: Option<String>,
    pub source_binding_after_sha256: Option<String>,
    pub source_unchanged_before_and_after: bool,
    pub terrain_identity: Option<String>,
    pub source_pad_unchanged: bool,
    pub target_pad_unchanged: bool,
    pub source_shelf_preserved: bool,
    pub target_shelf_preserved: bool,
    pub nonterrain_context_unchanged: bool,
    pub obstacle_x_min_m: Option<f64>,
    pub obstacle_x_center_m: Option<f64>,
    pub obstacle_x_max_m: Option<f64>,
    pub obstacle_peak_y_m: Option<f64>,
    pub low_change_x_m: Option<f64>,
    pub low_change_delta_m: Option<f64>,
    pub search: Option<CanonicalInitialSearchV1>,
    pub full_search_identical: Option<bool>,
    pub selected_proposal_identical: Option<bool>,
    pub audit: Option<AirborneDirectAuditV1>,
    pub source_replay: Option<CanonicalInitialDirectCanaryReplayV1>,
    pub actual_terrain_conflict: Option<bool>,
    pub first_conflict: Option<CanonicalInitialDirectCanaryFirstConflictV1>,
    pub elapsed_wall_time_us: u64,
    pub failures: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryAirborneCaptureV1 {
    pub case_id: String,
    pub role: String,
    pub status: String,
    pub source_binding_before_sha256: Option<String>,
    pub source_binding_after_sha256: Option<String>,
    pub source_unchanged_before_and_after: bool,
    /// DTO and snapshot are retained as evidence only; regeneration consumes
    /// the actual in-memory SimulationState held by the collector.
    pub live_state: Option<crate::AirborneFlightStateV1>,
    pub full_state_snapshot: Option<SimulationStateSnapshotV1>,
    pub search: Option<AirborneDirectSearchV1>,
    pub selected_proposal: Option<AirborneDirectProposalV1>,
    pub audit: Option<AirborneDirectAuditV1>,
    pub stitched_replay: Option<NominalAirborneStitchedReplayV1>,
    pub search_wall_time_us: u64,
    pub audit_wall_time_us: u64,
    pub stitched_replay_wall_time_us: u64,
    pub failures: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryDiscriminatorCaseV1 {
    pub case_id: String,
    pub status: String,
    pub source_binding_before_sha256: Option<String>,
    pub source_binding_after_sha256: Option<String>,
    pub source_unchanged_before_and_after: bool,
    pub request: Option<WaypointDirectNominalDirectGenerationRequest>,
    pub search: Option<CanonicalInitialSearchV1>,
    pub audit: Option<AirborneDirectAuditV1>,
    pub source_replay: Option<CanonicalInitialDirectCanaryReplayV1>,
    pub actual_terrain_conflict: Option<bool>,
    pub first_conflict: Option<CanonicalInitialDirectCanaryFirstConflictV1>,
    pub elapsed_wall_time_us: u64,
    pub failures: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryPeakReplayV1 {
    pub maximum_com_height_m: f64,
    pub maximum_com_height_physics_step: u64,
    pub final_state: SimulationStateSnapshotV1,
    pub incoming_contact: Option<pd_core::IncomingContactV1>,
    pub ordinary_run_match: bool,
    pub ordinary_action_replay_match: bool,
    pub official_replay: Option<RunArtifacts>,
    pub ordinary_run: Option<RunArtifacts>,
    pub source_actions: Vec<pd_core::ActionLogEntry>,
    pub source_events: Vec<pd_core::EventRecord>,
    pub source_samples: Vec<pd_core::SampleRecord>,
    pub source_actions_match: bool,
    pub source_events_match: bool,
    pub source_samples_match: bool,
    pub final_clock_match: bool,
    pub safe_target_contact: bool,
    pub passed: bool,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryOldPolicyV1 {
    pub status: String,
    pub source_binding_before_sha256: Option<String>,
    pub source_binding_after_sha256: Option<String>,
    pub source_unchanged_before_and_after: bool,
    pub decision: Option<NominalDirectFlightDecisionV1>,
    pub generation: Option<crate::BodyAwareTerminalCaseArtifactV1>,
    pub execution: Option<NominalDirectFlightExecutionEvidenceV1>,
    pub compute: Option<NominalDirectFlightComputeV1>,
    pub peak_replay: Option<CanonicalInitialDirectCanaryPeakReplayV1>,
    pub canonical_peak_com_height_m: f64,
    pub complete_flown_peak_exceeds_canonical: Option<bool>,
    pub elapsed_wall_time_us: u64,
    pub failures: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryDiscriminatorV1 {
    pub flat_control: CanonicalInitialDirectCanaryDiscriminatorCaseV1,
    pub late_broad: CanonicalInitialDirectCanaryDiscriminatorCaseV1,
    pub canonical_search_and_proposal_identical: Option<bool>,
    pub old_terrain_aware_source_only: CanonicalInitialDirectCanaryOldPolicyV1,
    pub passed: bool,
    pub failures: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryCaseV1 {
    pub case_id: String,
    pub cohort: String,
    pub exposure_truth: String,
    pub request: Option<WaypointDirectNominalDirectGenerationRequest>,
    /// `passed`, `failed`, or `not_evaluated` for source-rest only. Downstream
    /// terrain/capture stages have independent status records.
    pub status: String,
    pub search: Option<CanonicalInitialSearchV1>,
    /// Terrain-bound execution wrapper; proposal identity remains terrain-free.
    pub selected_program: Option<FlightProgramV1>,
    pub audit: Option<AirborneDirectAuditV1>,
    pub source_departure: Option<CanonicalInitialDirectCanarySourceDepartureV1>,
    pub source_replay: Option<CanonicalInitialDirectCanaryReplayV1>,
    pub terrain_twins: Vec<CanonicalInitialDirectCanaryTerrainTwinV1>,
    pub airborne_captures: Vec<CanonicalInitialDirectCanaryAirborneCaptureV1>,
    pub source_binding_before_sha256: Option<String>,
    pub source_binding_after_sha256: Option<String>,
    pub source_unchanged_before_and_after: bool,
    pub failures: Vec<String>,
    pub elapsed_wall_time_us: u64,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialDirectCanaryArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub protocol_path: String,
    pub selection_manifest_path: String,
    pub input_hashes: Vec<CanonicalInitialDirectCanaryInputHashV1>,
    pub source_binding: Option<NominalDirectFlightSourceBindingV1>,
    pub development_case_order: Vec<String>,
    pub additional_physical_case_order: Vec<String>,
    pub cases: Vec<CanonicalInitialDirectCanaryCaseV1>,
    pub discriminator: Option<CanonicalInitialDirectCanaryDiscriminatorV1>,
    pub gates: Vec<CanonicalInitialDirectCanaryGateV1>,
    pub setup_failures: Vec<String>,
    pub stop_reason: Option<String>,
    pub passed: bool,
    pub identity: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionManifestV1 {
    schema_id: String,
    schema_version: u32,
    policy_id: String,
    development_manifest: String,
    development_case_ids: Vec<String>,
    additional_endpoint_manifest: String,
    additional_endpoint_case_ids: Vec<String>,
    additional_input_scope: String,
    duration_multipliers: Vec<f64>,
    source_duration_offsets_ticks: Vec<i64>,
    terminal_duration_offsets_ticks: Vec<u64>,
    maximum_outer_attempts_per_case: usize,
    stop_rule: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PhysicalCompletionManifestV1 {
    schema_id: String,
    schema_version: u32,
    sealed_before_implementation: bool,
    #[serde(rename = "execution_policy_identity")]
    _ignored_execution_policy_identity: String,
    case_order: String,
    // Deliberately parsed as opaque values and never used to construct a run.
    #[serde(rename = "completion_policy")]
    _ignored_completion_policy: serde_json::Value,
    #[serde(rename = "intervention_policy")]
    _ignored_intervention_policy: serde_json::Value,
    #[serde(rename = "generation_policy")]
    _ignored_generation_policy: serde_json::Value,
    #[serde(rename = "terminal_policy")]
    _ignored_terminal_policy: serde_json::Value,
    cases: Vec<PhysicalCompletionCaseV1>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PhysicalCompletionCaseV1 {
    case_id: String,
    source_pad_id: String,
    target_pad_id: String,
    #[serde(rename = "profile")]
    _ignored_profile: String,
    #[serde(rename = "horizontal_span_m")]
    _ignored_horizontal_span_m: f64,
    #[serde(rename = "height_change_m")]
    _ignored_height_change_m: f64,
    #[serde(rename = "probe_id")]
    _ignored_probe_id: String,
    scenario: pd_core::ScenarioSpec,
}

#[derive(Serialize)]
struct CanonicalProgramPolicyIdentityV1<'a> {
    policy_id: &'static str,
    analytical_policy: &'a pd_plan::conservative_ballistic_bridge::DirectBridgePolicyV2,
    physics_hz: u32,
    controller_hz: u32,
    launch_upright_ticks: u64,
    launch_tilt_ticks: u64,
    source_solver_version: &'a str,
    source_solver_iterations: usize,
    source_line_search_steps: usize,
    source_finite_difference_step_mps2: f64,
    source_maximum_correction_abs_mps2: f64,
    source_initial_damping: f64,
    source_position_tolerance_m: f64,
    source_velocity_tolerance_mps: f64,
    geometry_convention: &'a str,
    source_duration_offsets_ticks: [i64; 5],
    terminal_duration_offsets_ticks: [u64; 7],
    maximum_outer_attempts: usize,
}

/// Create-only runner for the sealed canonical-initial protocol. Every stage
/// stops at its first decisive failure; unreached cases remain not evaluated.
pub fn run_canonical_initial_direct_canary(
    repo_root: &Path,
    output_root: &Path,
) -> Result<CanonicalInitialDirectCanaryArtifactV1> {
    reserve_output_root(output_root)?;
    let cases_dir = output_root.join("cases");
    let stages_dir = output_root.join("stages");
    fs::create_dir(&cases_dir).context("create-only canonical case directory")?;
    fs::create_dir(&stages_dir).context("create-only canonical stages directory")?;

    let mut artifact = CanonicalInitialDirectCanaryArtifactV1 {
        schema_id: CANONICAL_INITIAL_DIRECT_CANARY_SCHEMA_ID.into(),
        schema_version: 1,
        protocol_path: CANONICAL_INITIAL_DIRECT_CANARY_PROTOCOL.into(),
        selection_manifest_path: CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_MANIFEST.into(),
        input_hashes: input_hashes(repo_root),
        source_binding: None,
        development_case_order: DEVELOPMENT_CASE_IDS.iter().map(|id| (*id).into()).collect(),
        additional_physical_case_order: ADDITIONAL_CASE_IDS.iter().map(|id| (*id).into()).collect(),
        cases: Vec::new(),
        discriminator: None,
        gates: GATE_IDS
            .iter()
            .map(|gate_id| CanonicalInitialDirectCanaryGateV1 {
                gate_id: (*gate_id).into(),
                status: "not_evaluated".into(),
                passed: None,
                reason: Some("not evaluated: previous gate has not passed".into()),
            })
            .collect(),
        setup_failures: Vec::new(),
        stop_reason: None,
        passed: false,
        identity: String::new(),
    };
    artifact
        .cases
        .extend(DEVELOPMENT_CASE_IDS.iter().map(|case_id| {
            empty_case(
                case_id,
                "development",
                "Previously exposed operational source-rest control; not a hidden holdout.",
                None,
            )
        }));
    artifact
        .cases
        .extend(ADDITIONAL_CASE_IDS.iter().map(|case_id| {
        empty_case(
            case_id,
            "additional_physical_endpoint",
            "Previously design-exposed physical endpoint only; no completion-reserve semantics.",
            None,
        )
    }));

    write_create_only(
        &cases_dir.join("case_index.json"),
        &artifact
            .cases
            .iter()
            .map(|case| {
                serde_json::json!({
                    "case_id": case.case_id,
                    "cohort": case.cohort,
                    "exposure_truth": case.exposure_truth,
                    "status": "not_evaluated",
                })
            })
            .collect::<Vec<_>>(),
    )?;

    if let Some(binding) = artifact
        .input_hashes
        .iter()
        .find(|input| !input.matches_seal)
    {
        artifact.setup_failures.push(format!(
            "sealed input hash mismatch or unreadable input: {}",
            binding.relative_path
        ));
    }
    if let Some(error) = validate_selection_manifest(repo_root).err() {
        artifact
            .setup_failures
            .push(format!("selection manifest: {error:#}"));
    }
    let binding_before = match canonical_source_binding(repo_root) {
        Ok(binding) => {
            artifact.source_binding = Some(binding.clone());
            Some(binding)
        }
        Err(error) => {
            artifact
                .setup_failures
                .push(format!("source/input closure: {error:#}"));
            None
        }
    };

    if artifact.setup_failures.is_empty() {
        match load_nominal_direct_operational_fresh_inputs(repo_root) {
            Ok(inputs) if inputs.len() == DEVELOPMENT_CASE_IDS.len() => {
                for (index, (case_id, request)) in inputs.into_iter().enumerate() {
                    let expected = DEVELOPMENT_CASE_IDS[index];
                    if case_id != expected {
                        artifact.setup_failures.push(format!(
                            "development input order mismatch at row {index}: {case_id}"
                        ));
                        break;
                    }
                    artifact.cases[index].request = Some(request);
                }
            }
            Ok(_) => artifact
                .setup_failures
                .push("development input manifest must supply exactly four cases".into()),
            Err(error) => artifact
                .setup_failures
                .push(format!("development input manifest: {error:#}")),
        }
    }

    if artifact.setup_failures.is_empty() {
        let gate_a_passed = run_development_source_rest_gate(
            repo_root,
            binding_before.as_ref(),
            &cases_dir,
            &mut artifact,
        )?;
        if gate_a_passed {
            let terrain_twins_passed = run_development_terrain_twins_gate(
                repo_root,
                binding_before
                    .as_ref()
                    .expect("source closure preflight passed"),
                &cases_dir,
                &mut artifact,
            )?;
            if terrain_twins_passed {
                let discriminator_passed = run_900m_discriminator_gate(
                    repo_root,
                    binding_before
                        .as_ref()
                        .expect("source closure preflight passed"),
                    &stages_dir,
                    &mut artifact,
                )?;
                if discriminator_passed {
                    let airborne_passed = run_live_airborne_compatibility_gate(
                        repo_root,
                        binding_before
                            .as_ref()
                            .expect("source closure preflight passed"),
                        &cases_dir,
                        &mut artifact,
                    )?;
                    if airborne_passed {
                        run_additional_physical_endpoints_gate(
                            repo_root,
                            binding_before
                                .as_ref()
                                .expect("source closure preflight passed"),
                            &cases_dir,
                            &mut artifact,
                        )?;
                    }
                }
            }
        }
        persist_not_evaluated_case_statuses(&cases_dir, &mut artifact.cases)?;
        if artifact.stop_reason.is_some() {
            mark_downstream_not_evaluated(
                &mut artifact.gates,
                artifact
                    .stop_reason
                    .as_deref()
                    .unwrap_or("stopped at first decisive gate"),
            );
        }
    } else {
        artifact.stop_reason =
            Some("sealed input/source preflight failed before any physical case evaluation".into());
        set_gate(
            &mut artifact.gates,
            "development_source_rest",
            "not_evaluated",
            None,
            Some("not evaluated: sealed input or source closure preflight failed".into()),
        );
        mark_downstream_not_evaluated(&mut artifact.gates, "setup/preflight failure");
        for case in &mut artifact.cases {
            case.failures
                .push("not evaluated: sealed input or source closure preflight failed".into());
        }
        persist_not_evaluated_case_statuses(&cases_dir, &mut artifact.cases)?;
    }

    if let Some(expected_binding) = binding_before.as_ref() {
        match canonical_source_binding(repo_root) {
            Ok(after) if &after == expected_binding => set_gate(
                &mut artifact.gates,
                "final_source_closure",
                "passed",
                Some(true),
                None,
            ),
            Ok(_) => {
                artifact
                    .stop_reason
                    .get_or_insert_with(|| "source/input closure changed by end of run".into());
                set_gate(
                    &mut artifact.gates,
                    "final_source_closure",
                    "failed",
                    Some(false),
                    artifact.stop_reason.clone(),
                );
            }
            Err(error) => set_gate(
                &mut artifact.gates,
                "final_source_closure",
                "failed",
                Some(false),
                Some(format!("final source closure check failed: {error:#}")),
            ),
        }
    } else {
        set_gate(
            &mut artifact.gates,
            "final_source_closure",
            "not_evaluated",
            None,
            Some("no valid initial source binding".into()),
        );
    }
    for case in &mut artifact.cases {
        finalize_case(case)?;
        write_create_only(
            &cases_dir.join(format!("{}.final.json", case.case_id)),
            case,
        )?;
    }
    artifact.passed = artifact.setup_failures.is_empty()
        && artifact.gates.iter().all(|gate| gate.passed == Some(true));
    artifact.identity = artifact_identity(&artifact)?;
    write_create_only(&output_root.join("summary.json"), &artifact)?;
    Ok(artifact)
}

fn run_development_source_rest_gate(
    repo_root: &Path,
    expected_binding: Option<&NominalDirectFlightSourceBindingV1>,
    cases_dir: &Path,
    artifact: &mut CanonicalInitialDirectCanaryArtifactV1,
) -> Result<bool> {
    set_gate(
        &mut artifact.gates,
        "development_source_rest",
        "running",
        None,
        None,
    );
    let Some(expected_binding) = expected_binding else {
        artifact.stop_reason = Some("initial source/input closure was unavailable".into());
        set_gate(
            &mut artifact.gates,
            "development_source_rest",
            "failed",
            Some(false),
            artifact.stop_reason.clone(),
        );
        return Ok(false);
    };
    for index in 0..DEVELOPMENT_CASE_IDS.len() {
        let case = &mut artifact.cases[index];
        match canonical_source_binding(repo_root) {
            Ok(binding) => {
                case.source_binding_before_sha256 = Some(binding.identity_sha256.clone());
                if &binding != expected_binding {
                    case.status = "failed".into();
                    case.failures
                        .push("source/input closure changed before source-rest case".into());
                }
            }
            Err(error) => {
                case.status = "failed".into();
                case.failures.push(format!(
                    "source closure check failed before case: {error:#}"
                ));
            }
        }
        if case.status != "failed" {
            let request = case
                .request
                .clone()
                .context("prepared development request missing")?;
            evaluate_development_case(case, &request)?;
        }
        match canonical_source_binding(repo_root) {
            Ok(binding) => {
                case.source_binding_after_sha256 = Some(binding.identity_sha256.clone());
                case.source_unchanged_before_and_after =
                    case.source_binding_before_sha256.as_deref()
                        == Some(binding.identity_sha256.as_str())
                        && &binding == expected_binding;
                if !case.source_unchanged_before_and_after {
                    case.failures
                        .push("source/input closure changed during source-rest case".into());
                    case.status = "failed".into();
                }
            }
            Err(error) => {
                case.failures
                    .push(format!("source closure check failed after case: {error:#}"));
                case.status = "failed".into();
            }
        }
        finalize_case(case)?;
        write_create_only(
            &cases_dir.join(format!("{}.source_rest.json", case.case_id)),
            case,
        )?;
        if case.status != "passed" {
            artifact.stop_reason = Some(format!(
                "first decisive development source-rest failure: {}",
                case.case_id
            ));
            mark_unvisited_development_cases(&mut artifact.cases);
            set_gate(
                &mut artifact.gates,
                "development_source_rest",
                "failed",
                Some(false),
                artifact.stop_reason.clone(),
            );
            return Ok(false);
        }
    }
    set_gate(
        &mut artifact.gates,
        "development_source_rest",
        "passed",
        Some(true),
        None,
    );
    Ok(true)
}

fn persist_not_evaluated_case_statuses(
    cases_dir: &Path,
    cases: &mut [CanonicalInitialDirectCanaryCaseV1],
) -> Result<()> {
    for case in cases
        .iter_mut()
        .filter(|case| case.status == "not_evaluated")
    {
        if case.failures.is_empty() {
            case.failures.push(if case.cohort == "development" {
                "not evaluated: stopped at the first decisive earlier gate".into()
            } else {
                "not evaluated: additional physical endpoint stage was not reached".into()
            });
        }
        finalize_case(case)?;
        write_create_only(
            &cases_dir.join(format!("{}.not_evaluated.json", case.case_id)),
            case,
        )?;
    }
    Ok(())
}

fn run_development_terrain_twins_gate(
    repo_root: &Path,
    expected_binding: &NominalDirectFlightSourceBindingV1,
    cases_dir: &Path,
    artifact: &mut CanonicalInitialDirectCanaryArtifactV1,
) -> Result<bool> {
    set_gate(
        &mut artifact.gates,
        "development_terrain_twins",
        "running",
        None,
        None,
    );
    for case in artifact.cases.iter_mut().take(DEVELOPMENT_CASE_IDS.len()) {
        case.terrain_twins = vec![
            empty_twin_evidence(&case.case_id, "blocking_triangle", "blocking"),
            empty_twin_evidence(&case.case_id, "low_nonblocking", "low"),
        ];
    }
    for index in 0..DEVELOPMENT_CASE_IDS.len() {
        for (twin_index, (kind, twin_id)) in [
            ("blocking", "blocking_triangle"),
            ("low", "low_nonblocking"),
        ]
        .into_iter()
        .enumerate()
        {
            let case = &mut artifact.cases[index];
            let base_request = case
                .request
                .clone()
                .context("accepted source-rest request missing")?;
            let baseline_search = case
                .search
                .clone()
                .context("accepted canonical search missing")?;
            let mut evidence = empty_twin_evidence(&case.case_id, twin_id, kind);
            let before = match canonical_source_binding(repo_root) {
                Ok(binding) => {
                    let matches = &binding == expected_binding;
                    evidence.source_binding_before_sha256 = Some(binding.identity_sha256.clone());
                    (Some(binding.identity_sha256), matches)
                }
                Err(error) => {
                    evidence.failures.push(format!(
                        "source closure failed before terrain twin: {error:#}"
                    ));
                    (None, false)
                }
            };
            if !before.1 && evidence.failures.is_empty() {
                evidence
                    .failures
                    .push("source/input closure changed before terrain twin".into());
            }
            if before.1 {
                evidence = evaluate_terrain_twin(
                    &case.case_id,
                    &base_request,
                    &baseline_search,
                    twin_id,
                    kind,
                );
                evidence.source_binding_before_sha256 = before.0;
            }
            match canonical_source_binding(repo_root) {
                Ok(binding) => {
                    evidence.source_binding_after_sha256 = Some(binding.identity_sha256.clone());
                    evidence.source_unchanged_before_and_after =
                        evidence.source_binding_before_sha256.as_deref()
                            == Some(binding.identity_sha256.as_str())
                            && &binding == expected_binding;
                    if !evidence.source_unchanged_before_and_after {
                        evidence
                            .failures
                            .push("source/input closure changed during terrain twin".into());
                    }
                }
                Err(error) => evidence.failures.push(format!(
                    "source closure failed after terrain twin: {error:#}"
                )),
            }
            if evidence.failures.is_empty() && evidence.status == "passed" {
                // All semantic gates were established by the evaluator below.
            } else {
                evidence.status = "failed".into();
            }
            finalize_twin(&mut evidence)?;
            write_create_only(
                &cases_dir.join(format!("{}.terrain_twin_{}.json", case.case_id, kind)),
                &evidence,
            )?;
            case.terrain_twins[twin_index] = evidence.clone();
            if evidence.status != "passed" {
                artifact.stop_reason = Some(format!(
                    "first decisive development terrain-twin failure: {} / {}",
                    case.case_id, evidence.twin_id
                ));
                finalize_case(case)?;
                set_gate(
                    &mut artifact.gates,
                    "development_terrain_twins",
                    "failed",
                    Some(false),
                    artifact.stop_reason.clone(),
                );
                persist_unvisited_twins(cases_dir, &mut artifact.cases)?;
                return Ok(false);
            }
        }
        finalize_case(&mut artifact.cases[index])?;
    }
    persist_unvisited_twins(cases_dir, &mut artifact.cases)?;
    set_gate(
        &mut artifact.gates,
        "development_terrain_twins",
        "passed",
        Some(true),
        None,
    );
    Ok(true)
}

fn persist_unvisited_twins(
    cases_dir: &Path,
    cases: &mut [CanonicalInitialDirectCanaryCaseV1],
) -> Result<()> {
    for case in cases.iter_mut().take(DEVELOPMENT_CASE_IDS.len()) {
        for twin in &mut case.terrain_twins {
            if twin.status == "not_evaluated" {
                twin.failures.push(
                    "not evaluated: stopped at the first decisive earlier terrain-twin failure"
                        .into(),
                );
                finalize_twin(twin)?;
                write_create_only(
                    &cases_dir.join(format!("{}.terrain_twin_{}.json", twin.case_id, twin.kind)),
                    twin,
                )?;
            }
        }
    }
    Ok(())
}

fn empty_twin_evidence(
    case_id: &str,
    twin_id: &str,
    kind: &str,
) -> CanonicalInitialDirectCanaryTerrainTwinV1 {
    CanonicalInitialDirectCanaryTerrainTwinV1 {
        case_id: case_id.into(),
        twin_id: twin_id.into(),
        kind: kind.into(),
        status: "not_evaluated".into(),
        request: None,
        source_binding_before_sha256: None,
        source_binding_after_sha256: None,
        source_unchanged_before_and_after: false,
        terrain_identity: None,
        source_pad_unchanged: false,
        target_pad_unchanged: false,
        source_shelf_preserved: false,
        target_shelf_preserved: false,
        nonterrain_context_unchanged: false,
        obstacle_x_min_m: None,
        obstacle_x_center_m: None,
        obstacle_x_max_m: None,
        obstacle_peak_y_m: None,
        low_change_x_m: None,
        low_change_delta_m: None,
        search: None,
        full_search_identical: None,
        selected_proposal_identical: None,
        audit: None,
        source_replay: None,
        actual_terrain_conflict: None,
        first_conflict: None,
        elapsed_wall_time_us: 0,
        failures: Vec::new(),
        identity: String::new(),
    }
}

fn evaluate_terrain_twin(
    case_id: &str,
    base_request: &WaypointDirectNominalDirectGenerationRequest,
    baseline_search: &CanonicalInitialSearchV1,
    twin_id: &str,
    kind: &str,
) -> CanonicalInitialDirectCanaryTerrainTwinV1 {
    let started = Instant::now();
    let mut evidence = empty_twin_evidence(case_id, twin_id, kind);
    let Some(proposal) = baseline_search.selected.as_ref() else {
        evidence
            .failures
            .push("baseline search has no selected proposal".into());
        evidence.elapsed_wall_time_us = elapsed_us(started);
        return evidence;
    };
    let base_context = match RunContext::from_scenario(&base_request.scenario) {
        Ok(context) => context,
        Err(error) => {
            evidence
                .failures
                .push(format!("baseline context failed: {error}"));
            evidence.elapsed_wall_time_us = elapsed_us(started);
            return evidence;
        }
    };
    let (twin_request, twin_context) = if kind == "blocking" {
        match construct_terrain_twin(
            &base_context,
            &base_request.scenario,
            &base_request.source_pad_id,
            proposal.initial_state.position_m,
            proposal.peak_com_height_m,
        ) {
            Ok(twin) => {
                evidence.source_pad_unchanged = twin.evidence.source_pad_unchanged;
                evidence.target_pad_unchanged = twin.evidence.target_pad_unchanged;
                evidence.source_shelf_preserved = twin.evidence.source_shelf_preserved;
                evidence.target_shelf_preserved = twin.evidence.target_shelf_preserved;
                evidence.nonterrain_context_unchanged = twin.evidence.nonterrain_context_unchanged;
                evidence.obstacle_x_min_m = twin.evidence.obstacle_x_min_m;
                evidence.obstacle_x_center_m = twin.evidence.obstacle_x_peak_m;
                evidence.obstacle_x_max_m = twin.evidence.obstacle_x_max_m;
                evidence.obstacle_peak_y_m = twin.evidence.obstacle_peak_y_m;
                let mut request = base_request.clone();
                request.scenario.world.terrain = twin.context.world.terrain.clone();
                (request, twin.context)
            }
            Err(error) => {
                evidence
                    .failures
                    .push(format!("blocking triangle construction failed: {error:#}"));
                evidence.elapsed_wall_time_us = elapsed_us(started);
                return evidence;
            }
        }
    } else {
        match construct_low_nonblocking_twin(base_request, &base_context) {
            Ok((request, context, x)) => {
                evidence.source_pad_unchanged = true;
                evidence.target_pad_unchanged = true;
                let Some(source_pad) = base_context.world.landing_pad(&base_request.source_pad_id)
                else {
                    evidence
                        .failures
                        .push("source pad missing for low twin".into());
                    evidence.elapsed_wall_time_us = elapsed_us(started);
                    return evidence;
                };
                let Some(target_pad) = base_context.world.landing_pad(&base_request.target_pad_id)
                else {
                    evidence
                        .failures
                        .push("target pad missing for low twin".into());
                    evidence.elapsed_wall_time_us = elapsed_us(started);
                    return evidence;
                };
                evidence.source_shelf_preserved = shelf_preserved(
                    &base_context.world.terrain,
                    &context.world.terrain,
                    source_pad,
                );
                evidence.target_shelf_preserved = shelf_preserved(
                    &base_context.world.terrain,
                    &context.world.terrain,
                    target_pad,
                );
                evidence.nonterrain_context_unchanged =
                    nonterrain_context_unchanged(&base_context, &context);
                evidence.low_change_x_m = Some(x);
                evidence.low_change_delta_m = Some(-1.0);
                (request, context)
            }
            Err(error) => {
                evidence.failures.push(format!(
                    "low nonblocking terrain construction failed: {error:#}"
                ));
                evidence.elapsed_wall_time_us = elapsed_us(started);
                return evidence;
            }
        }
    };
    evidence.terrain_identity =
        nominal_direct_flight_identity(&twin_request.scenario.world.terrain).ok();
    evidence.request = Some(twin_request.clone());
    if !evidence.source_pad_unchanged
        || !evidence.target_pad_unchanged
        || !evidence.source_shelf_preserved
        || !evidence.target_shelf_preserved
        || !evidence.nonterrain_context_unchanged
    {
        evidence
            .failures
            .push("terrain construction changed a shelf, pad, or nonterrain input".into());
    }
    let twin_search = evaluate_canonical_initial_direct(&twin_request);
    match twin_search {
        Ok(search) => {
            evidence.full_search_identical = Some(&search == baseline_search);
            evidence.selected_proposal_identical =
                Some(search.selected == baseline_search.selected);
            if evidence.full_search_identical != Some(true)
                || evidence.selected_proposal_identical != Some(true)
            {
                evidence.failures.push(
                    "terrain twin changed the complete canonical ledger or selected proposal"
                        .into(),
                );
            }
            evidence.search = Some(search);
        }
        Err(error) => evidence
            .failures
            .push(format!("canonical twin search failed: {error:#}")),
    }
    let source_pad = match source_pad_input(&twin_context, &twin_request.source_pad_id) {
        Ok(pad) => pad,
        Err(error) => {
            evidence
                .failures
                .push(format!("twin source pad missing: {error:#}"));
            evidence.elapsed_wall_time_us = elapsed_us(started);
            return evidence;
        }
    };
    match audit_canonical_initial_direct(
        &twin_context,
        &source_pad,
        proposal,
        twin_request.policy.analytical_policy.minimum_clearance_m,
    ) {
        Ok(audit) => {
            let conflict = has_actual_terrain_conflict(&audit);
            evidence.actual_terrain_conflict = Some(conflict);
            if conflict {
                match first_conflict_evidence(&twin_context, &proposal.updates, &audit) {
                    Ok(first) => evidence.first_conflict = first,
                    Err(error) => evidence
                        .failures
                        .push(format!("first-conflict query failed: {error:#}")),
                }
            }
            evidence.source_replay = Some(source_replay_evidence(
                &twin_context,
                &proposal.updates,
                &audit,
            ));
            evidence.audit = Some(audit);
        }
        Err(error) => evidence.failures.push(format!(
            "fixed-choice terrain audit failed structurally: {error:#}"
        )),
    }
    let search_same = evidence.full_search_identical == Some(true)
        && evidence.selected_proposal_identical == Some(true);
    let audit = evidence.audit.as_ref();
    let replay_passed = evidence
        .source_replay
        .as_ref()
        .is_some_and(|replay| replay.passed);
    let parity = audit.is_some_and(|audit| audit.ordinary_neutral_parity);
    let blocked = evidence.actual_terrain_conflict == Some(true)
        && evidence.first_conflict.is_some()
        && search_same
        && parity
        && replay_passed;
    let low_safe = audit.is_some_and(|audit| {
        audit.passed
            && audit.safe_target_contact
            && audit.ordinary_neutral_parity
            && audit.commands_match
    }) && search_same
        && replay_passed;
    let semantic_passed = if kind == "blocking" {
        blocked
    } else {
        low_safe
    };
    if !semantic_passed && evidence.failures.is_empty() {
        evidence.failures.push(if kind == "blocking" {
            "blocking triangle did not yield a parity-backed actual terrain conflict at the fixed choice".into()
        } else {
            "lowered interior terrain did not preserve the fixed choice's safe target landing".into()
        });
    }
    evidence.status = if evidence.failures.is_empty() && semantic_passed {
        "passed".into()
    } else {
        "failed".into()
    };
    evidence.elapsed_wall_time_us = elapsed_us(started);
    evidence
}

fn construct_low_nonblocking_twin(
    request: &WaypointDirectNominalDirectGenerationRequest,
    context: &RunContext,
) -> Result<(
    WaypointDirectNominalDirectGenerationRequest,
    RunContext,
    f64,
)> {
    let source = context
        .world
        .landing_pad(&request.source_pad_id)
        .context("source pad missing")?;
    let target = context
        .world
        .landing_pad(&request.target_pad_id)
        .context("target pad missing")?;
    let left = source.center_x_m + source.half_width_m() + 1.0;
    let right = target.center_x_m - target.half_width_m() - 1.0;
    let TerrainDefinition::Heightfield { points_m } = &context.world.terrain;
    let (x, _) = points_m
        .windows(2)
        .map(|pair| ((pair[0].x + pair[1].x) * 0.5, pair[1].x - pair[0].x))
        .filter(|(x, width)| *x > left && *x < right && *width > 0.0)
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .context("no deterministic interior heightfield segment for low twin")?;
    let mut twin_points = points_m.clone();
    let changed_height = context.world.terrain.sample_height(x) - 1.0;
    twin_points.push(Vec2::new(x, changed_height));
    twin_points.sort_by(|left, right| left.x.total_cmp(&right.x));
    let terrain = TerrainDefinition::Heightfield {
        points_m: twin_points,
    };
    terrain.validate().map_err(anyhow::Error::msg)?;
    let mut twin_context = context.clone();
    twin_context.world.terrain = terrain.clone();
    let mut twin_request = request.clone();
    twin_request.scenario.world.terrain = terrain;
    if !shelf_preserved(&context.world.terrain, &twin_context.world.terrain, source)
        || !shelf_preserved(&context.world.terrain, &twin_context.world.terrain, target)
        || !nonterrain_context_unchanged(context, &twin_context)
    {
        bail!("low twin did not remain a terrain-only interior change");
    }
    Ok((twin_request, twin_context, x))
}

fn shelf_preserved(
    baseline: &TerrainDefinition,
    twin: &TerrainDefinition,
    pad: &pd_core::LandingPadSpec,
) -> bool {
    (0..=32).all(|sample| {
        let fraction = sample as f64 / 32.0;
        let x = pad.center_x_m - pad.half_width_m() + pad.width_m * fraction;
        baseline.sample_height(x).to_bits() == twin.sample_height(x).to_bits()
    })
}

fn nonterrain_context_unchanged(baseline: &RunContext, twin: &RunContext) -> bool {
    baseline.sim == twin.sim
        && baseline.vehicle == twin.vehicle
        && baseline.initial_state == twin.initial_state
        && baseline.mission == twin.mission
        && baseline.world.gravity_mps2 == twin.world.gravity_mps2
        && baseline.world.landing_pads == twin.world.landing_pads
        && baseline.target_pad == twin.target_pad
        && baseline.scenario_id == twin.scenario_id
        && baseline.scenario_name == twin.scenario_name
        && baseline.scenario_seed == twin.scenario_seed
        && baseline.scenario_tags == twin.scenario_tags
}

pub(crate) fn has_actual_terrain_conflict(audit: &AirborneDirectAuditV1) -> bool {
    audit.clearance_scan.first_violation.is_some()
        || audit
            .first_contact
            .as_ref()
            .is_some_and(contact_is_actual_away_terrain_contact)
}

fn contact_is_actual_away_terrain_contact(contact: &crate::TerminalContactAuditEvidence) -> bool {
    contact_has_away_geometric_contact(
        &contact.classification,
        contact.core_matches_predicate_mirror,
        contact.state.touchdown_pad_contains_both_feet,
        contact.state.minimum_hull_clearance_m,
        [
            contact.state.touchdown_feet[0].signed_clearance_m,
            contact.state.touchdown_feet[1].signed_clearance_m,
        ],
    )
}

fn contact_has_away_geometric_contact(
    classification: &str,
    core_matches_predicate_mirror: bool,
    touchdown_feet_within_target: bool,
    minimum_hull_clearance_m: f64,
    foot_signed_clearance_m: [f64; 2],
) -> bool {
    classification != "stable_touchdown_on_target"
        && core_matches_predicate_mirror
        && !touchdown_feet_within_target
        && (minimum_hull_clearance_m <= 0.0
            || foot_signed_clearance_m
                .iter()
                .any(|clearance| *clearance <= 0.0))
}

pub(crate) fn first_conflict_evidence(
    context: &RunContext,
    updates: &[FlightProgramUpdateV1],
    audit: &AirborneDirectAuditV1,
) -> Result<Option<CanonicalInitialDirectCanaryFirstConflictV1>> {
    let clearance = audit.clearance_scan.first_violation.as_ref();
    let contact = audit
        .first_contact
        .as_ref()
        .filter(|contact| contact.classification != "stable_touchdown_on_target");
    let (tick, phase, clearance_m, required_clearance_m, reason, contact_classification) =
        match (clearance, contact) {
            (Some(violation), Some(contact))
                if violation.physics_step <= contact.state.physics_step =>
            {
                (
                    violation.physics_step,
                    violation.phase.clone(),
                    violation.clearance_m,
                    Some(violation.required_clearance_m),
                    Some(violation.reason.clone()),
                    None,
                )
            }
            (Some(_), Some(contact)) => (
                contact.state.physics_step,
                phase_at_tick(updates, contact.state.physics_step),
                Some(contact.state.minimum_hull_clearance_m),
                None,
                None,
                Some(contact.classification.clone()),
            ),
            (Some(violation), None) => (
                violation.physics_step,
                violation.phase.clone(),
                violation.clearance_m,
                Some(violation.required_clearance_m),
                Some(violation.reason.clone()),
                None,
            ),
            (None, Some(contact)) => (
                contact.state.physics_step,
                phase_at_tick(updates, contact.state.physics_step),
                Some(contact.state.minimum_hull_clearance_m),
                None,
                None,
                Some(contact.classification.clone()),
            ),
            (None, None) => return Ok(None),
        };
    let queried = query_program_state_at(context, updates, tick)?;
    Ok(Some(CanonicalInitialDirectCanaryFirstConflictV1 {
        physics_step: tick,
        phase,
        position_m: queried.position_m,
        state_snapshot: queried.state_snapshot,
        incoming_contact: queried.incoming_contact,
        clearance_m,
        required_clearance_m,
        clearance_reason: reason,
        contact_classification,
    }))
}

struct ConflictStateQueryV1 {
    position_m: Vec2,
    state_snapshot: Option<SimulationStateSnapshotV1>,
    incoming_contact: Option<pd_core::IncomingContactV1>,
}

fn query_program_state_at(
    context: &RunContext,
    updates: &[FlightProgramUpdateV1],
    target_tick: u64,
) -> Result<ConflictStateQueryV1> {
    let interval = context.sim.control_interval_steps();
    let mut by_tick = updates.iter();
    let mut state = SimulationState::new(context)?;
    while state.physics_step < target_tick {
        if state.physics_step.is_multiple_of(interval) {
            let update = by_tick
                .next()
                .context("selected command prefix ended before conflict")?;
            if update.physics_step != state.physics_step {
                bail!("selected prefix clock mismatch at conflict query");
            }
            state.set_command(update.command);
        }
        let transition = state.step_with_contact_report(context);
        if let Some(contact) = transition.incoming_contact {
            if contact.state.physics_step == target_tick {
                return Ok(conflict_state_from_incoming_contact(contact));
            }
            if contact.state.physics_step < target_tick {
                bail!("source prefix contacted terrain before the audited first conflict tick");
            }
        }
        if state.is_terminal() && state.physics_step < target_tick {
            bail!("source prefix terminated before the audited first conflict tick");
        }
    }
    if state.physics_step != target_tick {
        bail!("conflict tick is not reachable on the exact physics clock");
    }
    Ok(ConflictStateQueryV1 {
        position_m: state.position_m,
        state_snapshot: Some(SimulationStateSnapshotV1::from_state(&state)),
        incoming_contact: None,
    })
}

fn conflict_state_from_incoming_contact(
    incoming_contact: pd_core::IncomingContactV1,
) -> ConflictStateQueryV1 {
    ConflictStateQueryV1 {
        position_m: incoming_contact.state.position_m,
        state_snapshot: Some(incoming_contact.state.clone()),
        incoming_contact: Some(incoming_contact),
    }
}

pub(crate) fn phase_at_tick(updates: &[FlightProgramUpdateV1], tick: u64) -> String {
    updates
        .iter()
        .take_while(|update| update.physics_step < tick)
        .last()
        .map(|update| update.phase.clone())
        .unwrap_or_else(|| "source_prefix".into())
}

pub(crate) fn source_pad_input(
    context: &RunContext,
    source_pad_id: &str,
) -> Result<pd_plan::conservative_ballistic_bridge::PadInputV2> {
    let pad = context
        .world
        .landing_pad(source_pad_id)
        .context("source pad missing from actual context")?;
    Ok(pd_plan::conservative_ballistic_bridge::PadInputV2 {
        center_x_m: pad.center_x_m,
        surface_y_m: pad.surface_y_m,
        width_m: pad.width_m,
    })
}

fn finalize_twin(evidence: &mut CanonicalInitialDirectCanaryTerrainTwinV1) -> Result<()> {
    let mut identity = evidence.clone();
    identity.identity.clear();
    identity.elapsed_wall_time_us = 0;
    evidence.identity = nominal_direct_flight_identity(&identity)?;
    Ok(())
}

fn run_900m_discriminator_gate(
    repo_root: &Path,
    expected_binding: &NominalDirectFlightSourceBindingV1,
    stages_dir: &Path,
    artifact: &mut CanonicalInitialDirectCanaryArtifactV1,
) -> Result<bool> {
    set_gate(
        &mut artifact.gates,
        "900m_late_broad_discriminator",
        "running",
        None,
        None,
    );
    let mut evidence = CanonicalInitialDirectCanaryDiscriminatorV1 {
        flat_control: empty_discriminator_case("fresh_flat_control_span_900"),
        late_broad: empty_discriminator_case("fresh_late_broad_span_900"),
        canonical_search_and_proposal_identical: None,
        old_terrain_aware_source_only: empty_old_policy_evidence(),
        passed: false,
        failures: Vec::new(),
    };
    artifact.discriminator = Some(evidence.clone());
    let manifest = match load_waypoint_direct_obstacle_discrimination_fresh_manifest(repo_root) {
        Ok(manifest) => manifest,
        Err(error) => {
            evidence.failures.push(format!(
                "fresh 900 m physical manifest load failed: {error:#}"
            ));
            artifact.discriminator = Some(evidence);
            set_gate(
                &mut artifact.gates,
                "900m_late_broad_discriminator",
                "failed",
                Some(false),
                Some("sealed 900 m discriminator inputs could not be loaded".into()),
            );
            artifact.stop_reason = Some("fresh 900 m discriminator input validation failed".into());
            write_create_only(
                &stages_dir.join("900m_input_failure.json"),
                &artifact.discriminator,
            )?;
            return Ok(false);
        }
    };
    let Some(flat) = manifest
        .cases
        .iter()
        .find(|case| case.case_id == "fresh_flat_control_span_900")
    else {
        evidence
            .failures
            .push("sealed flat 900 m physical case missing".into());
        artifact.discriminator = Some(evidence.clone());
        artifact.stop_reason = Some("fresh 900 m discriminator physical case missing".into());
        set_gate(
            &mut artifact.gates,
            "900m_late_broad_discriminator",
            "failed",
            Some(false),
            artifact.stop_reason.clone(),
        );
        write_create_only(&stages_dir.join("900m_input_failure.json"), &evidence)?;
        return Ok(false);
    };
    let Some(broad) = manifest
        .cases
        .iter()
        .find(|case| case.case_id == "fresh_late_broad_span_900")
    else {
        evidence
            .failures
            .push("sealed late-broad 900 m physical case missing".into());
        artifact.discriminator = Some(evidence.clone());
        artifact.stop_reason = Some("fresh 900 m late-broad physical case missing".into());
        set_gate(
            &mut artifact.gates,
            "900m_late_broad_discriminator",
            "failed",
            Some(false),
            artifact.stop_reason.clone(),
        );
        write_create_only(&stages_dir.join("900m_input_failure.json"), &evidence)?;
        return Ok(false);
    };
    if flat.horizontal_span_m != 900.0
        || flat.profile != "flat_control"
        || broad.horizontal_span_m != 900.0
        || broad.profile != "late_broad"
    {
        evidence.failures.push("sealed discriminator case selection did not resolve exact 900 m flat/late-broad inputs".into());
        artifact.discriminator = Some(evidence.clone());
        artifact.stop_reason = Some("fresh 900 m discriminator case selection mismatch".into());
        set_gate(
            &mut artifact.gates,
            "900m_late_broad_discriminator",
            "failed",
            Some(false),
            artifact.stop_reason.clone(),
        );
        write_create_only(&stages_dir.join("900m_input_failure.json"), &evidence)?;
        return Ok(false);
    }
    let make_request = |case: &crate::WaypointDirectObstacleDiscriminationFreshCaseV1| {
        WaypointDirectNominalDirectGenerationRequest {
            scenario: case.scenario.clone(),
            source_pad_id: case.source_pad_id.clone(),
            target_pad_id: case.target_pad_id.clone(),
            probe_id: case.probe_id.clone(),
            policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
        }
    };
    let flat_request = make_request(flat);
    let broad_request = make_request(broad);
    let mut gate_passed = true;
    for (case_evidence, request, name) in [
        (&mut evidence.flat_control, &flat_request, "flat"),
        (&mut evidence.late_broad, &broad_request, "late_broad"),
    ] {
        case_evidence.status = "running".into();
        case_evidence.request = Some(request.clone());
        match canonical_source_binding(repo_root) {
            Ok(binding) => {
                case_evidence.source_binding_before_sha256 = Some(binding.identity_sha256.clone());
                if &binding != expected_binding {
                    case_evidence.failures.push(
                        "source/input closure changed before canonical discriminator case".into(),
                    );
                }
            }
            Err(error) => case_evidence.failures.push(format!(
                "source closure failed before discriminator case: {error:#}"
            )),
        }
        if case_evidence.failures.is_empty() {
            evaluate_discriminator_case(case_evidence, request);
        }
        match canonical_source_binding(repo_root) {
            Ok(binding) => {
                case_evidence.source_binding_after_sha256 = Some(binding.identity_sha256.clone());
                case_evidence.source_unchanged_before_and_after =
                    case_evidence.source_binding_before_sha256.as_deref()
                        == Some(binding.identity_sha256.as_str())
                        && &binding == expected_binding;
                if !case_evidence.source_unchanged_before_and_after {
                    case_evidence
                        .failures
                        .push("source/input closure changed during discriminator case".into());
                }
            }
            Err(error) => case_evidence.failures.push(format!(
                "source closure failed after discriminator case: {error:#}"
            )),
        }
        let case_passed = if name == "flat" {
            discriminator_flat_passed(case_evidence)
        } else {
            case_evidence.actual_terrain_conflict == Some(true)
                && case_evidence.first_conflict.is_some()
                && case_evidence
                    .audit
                    .as_ref()
                    .is_some_and(|audit| audit.ordinary_neutral_parity)
                && case_evidence
                    .source_replay
                    .as_ref()
                    .is_some_and(|replay| replay.passed)
                && case_evidence.failures.is_empty()
        };
        if !case_passed && case_evidence.failures.is_empty() {
            case_evidence.failures.push(if name == "flat" {
                "canonical 900 m flat control did not retain safe actual target contact".into()
            } else {
                "canonical fixed choice was not blocked by actual late-broad terrain evidence"
                    .into()
            });
        }
        case_evidence.status = if case_passed { "passed" } else { "failed" }.into();
        let path = if name == "flat" {
            "900m_flat_canonical.json"
        } else {
            "900m_late_broad_canonical.json"
        };
        write_create_only(&stages_dir.join(path), case_evidence)?;
        if !case_passed {
            gate_passed = false;
            evidence
                .failures
                .push(format!("900 m {name} canonical case failed"));
            break;
        }
    }
    if gate_passed {
        evidence.canonical_search_and_proposal_identical = Some(
            evidence.flat_control.search == evidence.late_broad.search
                && evidence
                    .flat_control
                    .search
                    .as_ref()
                    .and_then(|search| search.selected.as_ref())
                    == evidence
                        .late_broad
                        .search
                        .as_ref()
                        .and_then(|search| search.selected.as_ref()),
        );
        if evidence.canonical_search_and_proposal_identical != Some(true) {
            gate_passed = false;
            evidence
                .failures
                .push("900 m flat and late-broad canonical search/proposal differed".into());
        }
    }
    if gate_passed {
        let broad_proposal = evidence
            .late_broad
            .search
            .as_ref()
            .and_then(|search| search.selected.as_ref())
            .context("900 m late-broad canonical proposal missing")?;
        let started = Instant::now();
        let before = closure_matches(repo_root, expected_binding);
        let mut old = if before.0 {
            old_policy::run_old_terrain_aware_source_only(
                &broad_request,
                broad_proposal.peak_com_height_m,
            )
        } else {
            let mut failed = empty_old_policy_evidence();
            failed.failures.push(before.1.clone().unwrap_or_else(|| {
                "source/input closure changed before old-policy regeneration".into()
            }));
            failed.canonical_peak_com_height_m = broad_proposal.peak_com_height_m;
            failed
        };
        old.source_binding_before_sha256 = before.2;
        old.elapsed_wall_time_us = old.elapsed_wall_time_us.max(elapsed_us(started));
        match canonical_source_binding(repo_root) {
            Ok(binding) => {
                old.source_binding_after_sha256 = Some(binding.identity_sha256.clone());
                old.source_unchanged_before_and_after = old.source_binding_before_sha256.as_deref()
                    == Some(binding.identity_sha256.as_str())
                    && &binding == expected_binding;
                if !old.source_unchanged_before_and_after {
                    old.failures.push(
                        "source/input closure changed during old-policy regeneration/replay".into(),
                    );
                }
            }
            Err(error) => old.failures.push(format!(
                "source closure failed after old-policy replay: {error:#}"
            )),
        }
        let tall = old.complete_flown_peak_exceeds_canonical == Some(true);
        let execution_passed = old
            .execution
            .as_ref()
            .is_some_and(|execution| execution.passed && execution.safe_target_landing);
        let replay_passed = old
            .peak_replay
            .as_ref()
            .is_some_and(|replay| replay.passed && replay.safe_target_contact);
        if old.failures.is_empty()
            && tall
            && execution_passed
            && replay_passed
            && old.source_unchanged_before_and_after
        {
            old.status = "passed".into();
            evidence.old_terrain_aware_source_only = old;
        } else {
            old.status = "failed".into();
            old.failures.push("source-only old-policy acceptance proof did not satisfy every frozen discriminator requirement".into());
            evidence.failures.extend(old.failures.iter().cloned());
            if !tall {
                evidence.failures.push("fresh old-policy complete flown COM peak did not exceed the fixed canonical whole-flight peak".into());
            }
            if !execution_passed || !replay_passed {
                evidence.failures.push("fresh old-policy direct control failed safe ordinary execution or complete replay".into());
            }
            evidence.old_terrain_aware_source_only = old;
            gate_passed = false;
        }
        write_create_only(
            &stages_dir.join("900m_old_policy_source_only.json"),
            &evidence.old_terrain_aware_source_only,
        )?;
    }
    evidence.passed = gate_passed;
    if !gate_passed && evidence.failures.is_empty() {
        evidence.failures.push(
            "900 m discriminator failed a fixed-choice or fresh old-policy proof gate".into(),
        );
    }
    artifact.discriminator = Some(evidence.clone());
    if gate_passed {
        set_gate(
            &mut artifact.gates,
            "900m_late_broad_discriminator",
            "passed",
            Some(true),
            None,
        );
    } else {
        artifact.stop_reason = Some("first decisive 900 m late-broad discriminator failure".into());
        set_gate(
            &mut artifact.gates,
            "900m_late_broad_discriminator",
            "failed",
            Some(false),
            artifact.stop_reason.clone(),
        );
    }
    write_create_only(&stages_dir.join("900m_discriminator_final.json"), &evidence)?;
    Ok(gate_passed)
}

fn empty_discriminator_case(case_id: &str) -> CanonicalInitialDirectCanaryDiscriminatorCaseV1 {
    CanonicalInitialDirectCanaryDiscriminatorCaseV1 {
        case_id: case_id.into(),
        status: "not_evaluated".into(),
        source_binding_before_sha256: None,
        source_binding_after_sha256: None,
        source_unchanged_before_and_after: false,
        request: None,
        search: None,
        audit: None,
        source_replay: None,
        actual_terrain_conflict: None,
        first_conflict: None,
        elapsed_wall_time_us: 0,
        failures: Vec::new(),
    }
}

fn empty_old_policy_evidence() -> CanonicalInitialDirectCanaryOldPolicyV1 {
    CanonicalInitialDirectCanaryOldPolicyV1 {
        status: "not_evaluated".into(),
        source_binding_before_sha256: None,
        source_binding_after_sha256: None,
        source_unchanged_before_and_after: false,
        decision: None,
        generation: None,
        execution: None,
        compute: None,
        peak_replay: None,
        canonical_peak_com_height_m: 0.0,
        complete_flown_peak_exceeds_canonical: None,
        elapsed_wall_time_us: 0,
        failures: Vec::new(),
    }
}

fn evaluate_discriminator_case(
    case: &mut CanonicalInitialDirectCanaryDiscriminatorCaseV1,
    request: &WaypointDirectNominalDirectGenerationRequest,
) {
    let started = Instant::now();
    case.status = "running".into();
    case.request = Some(request.clone());
    let search = match evaluate_canonical_initial_direct(request) {
        Ok(search) => search,
        Err(error) => {
            case.failures
                .push(format!("canonical search failed: {error:#}"));
            case.elapsed_wall_time_us = elapsed_us(started);
            return;
        }
    };
    let Some(proposal) = search.selected.clone() else {
        case.failures
            .push("canonical finite family selected no complete proposal".into());
        case.search = Some(search);
        case.elapsed_wall_time_us = elapsed_us(started);
        return;
    };
    case.search = Some(search);
    let context = match RunContext::from_scenario(&request.scenario) {
        Ok(context) => context,
        Err(error) => {
            case.failures
                .push(format!("actual 900 m RunContext failed: {error}"));
            case.elapsed_wall_time_us = elapsed_us(started);
            return;
        }
    };
    let source_pad = match source_pad_input(&context, &request.source_pad_id) {
        Ok(pad) => pad,
        Err(error) => {
            case.failures
                .push(format!("source pad lookup failed: {error:#}"));
            case.elapsed_wall_time_us = elapsed_us(started);
            return;
        }
    };
    match audit_canonical_initial_direct(
        &context,
        &source_pad,
        &proposal,
        request.policy.analytical_policy.minimum_clearance_m,
    ) {
        Ok(audit) => {
            case.actual_terrain_conflict = Some(has_actual_terrain_conflict(&audit));
            if case.actual_terrain_conflict == Some(true) {
                match first_conflict_evidence(&context, &proposal.updates, &audit) {
                    Ok(conflict) => case.first_conflict = conflict,
                    Err(error) => case
                        .failures
                        .push(format!("first-conflict query failed: {error:#}")),
                }
            }
            case.source_replay = Some(source_replay_evidence(&context, &proposal.updates, &audit));
            case.audit = Some(audit);
        }
        Err(error) => case
            .failures
            .push(format!("actual terrain audit failed: {error:#}")),
    }
    case.elapsed_wall_time_us = elapsed_us(started);
}

fn discriminator_flat_passed(case: &CanonicalInitialDirectCanaryDiscriminatorCaseV1) -> bool {
    case.failures.is_empty()
        && case.source_unchanged_before_and_after
        && case.audit.as_ref().is_some_and(|audit| {
            audit.passed
                && audit.safe_target_contact
                && audit.ordinary_neutral_parity
                && audit.commands_match
        })
        && case
            .source_replay
            .as_ref()
            .is_some_and(|replay| replay.passed)
}

fn run_live_airborne_compatibility_gate(
    repo_root: &Path,
    expected_binding: &NominalDirectFlightSourceBindingV1,
    cases_dir: &Path,
    artifact: &mut CanonicalInitialDirectCanaryArtifactV1,
) -> Result<bool> {
    const ROLES: [&str; 3] = ["ascending", "near_apex", "descending"];
    set_gate(
        &mut artifact.gates,
        "airborne_live_state_compatibility",
        "running",
        None,
        None,
    );
    for case in artifact.cases.iter_mut().take(DEVELOPMENT_CASE_IDS.len()) {
        case.airborne_captures = ROLES
            .iter()
            .map(|role| empty_airborne_capture(&case.case_id, role))
            .collect();
    }
    let mut gate_passed = true;
    'cases: for case_index in 0..DEVELOPMENT_CASE_IDS.len() {
        let case = &mut artifact.cases[case_index];
        let Some(request) = case.request.clone() else {
            gate_passed = false;
            artifact.stop_reason = Some(format!(
                "{} has no retained source-rest request",
                case.case_id
            ));
            break;
        };
        let Some(program) = case.selected_program.clone() else {
            gate_passed = false;
            artifact.stop_reason = Some(format!(
                "{} has no retained selected source-rest program",
                case.case_id
            ));
            break;
        };
        let Some(baseline_search) = case.search.as_ref() else {
            gate_passed = false;
            artifact.stop_reason =
                Some(format!("{} has no retained canonical search", case.case_id));
            break;
        };
        let context = match RunContext::from_scenario(&request.scenario) {
            Ok(context) => context,
            Err(error) => {
                case.airborne_captures[0].status = "failed".into();
                case.airborne_captures[0]
                    .failures
                    .push(format!("live capture context failed: {error}"));
                finalize_capture(&mut case.airborne_captures[0])?;
                write_create_only(
                    &cases_dir.join(format!("{}.airborne_ascending.json", case.case_id)),
                    &case.airborne_captures[0],
                )?;
                gate_passed = false;
                artifact.stop_reason = Some(format!(
                    "first decisive airborne capture context failure: {}",
                    case.case_id
                ));
                break;
            }
        };
        if !closure_matches(repo_root, expected_binding).0 {
            case.airborne_captures[0].status = "failed".into();
            case.airborne_captures[0]
                .failures
                .push("source/input closure changed before live capture collection".into());
            finalize_capture(&mut case.airborne_captures[0])?;
            write_create_only(
                &cases_dir.join(format!("{}.airborne_ascending.json", case.case_id)),
                &case.airborne_captures[0],
            )?;
            gate_passed = false;
            artifact.stop_reason = Some(format!(
                "source/input closure changed before captures for {}",
                case.case_id
            ));
            break;
        }
        let collection = collect_baseline_captures(&context, &program);
        let capture_states = [collection.first, collection.near_apex, collection.last];
        let source_collection_binding = closure_matches(repo_root, expected_binding);
        if !collection.failures.is_empty() || !source_collection_binding.0 {
            let capture = &mut case.airborne_captures[0];
            capture.status = "failed".into();
            capture.failures.extend(collection.failures.iter().cloned());
            if let Some(error) = source_collection_binding.1 {
                capture.failures.push(error);
            }
            finalize_capture(capture)?;
            write_create_only(
                &cases_dir.join(format!("{}.airborne_ascending.json", case.case_id)),
                capture,
            )?;
            gate_passed = false;
            artifact.stop_reason = Some(format!(
                "first decisive live capture collection failure: {}",
                case.case_id
            ));
            break;
        }
        let Some(first) = capture_states[0].as_ref() else {
            gate_passed = capture_collection_failure(
                case,
                cases_dir,
                "ascending",
                "ascending live capture missing",
            )?;
            artifact.stop_reason = Some(format!(
                "ascending live capture missing for {}",
                case.case_id
            ));
            break;
        };
        let Some(apex) = capture_states[1].as_ref() else {
            gate_passed = capture_collection_failure(
                case,
                cases_dir,
                "near_apex",
                "near-apex live capture missing",
            )?;
            artifact.stop_reason = Some(format!(
                "near-apex live capture missing for {}",
                case.case_id
            ));
            break;
        };
        let Some(last) = capture_states[2].as_ref() else {
            gate_passed = capture_collection_failure(
                case,
                cases_dir,
                "descending",
                "descending live capture missing",
            )?;
            artifact.stop_reason = Some(format!(
                "descending live capture missing for {}",
                case.case_id
            ));
            break;
        };
        let trace_minimum = collection.trace.iter().min_by(|left, right| {
            left.velocity_mps
                .y
                .abs()
                .total_cmp(&right.velocity_mps.y.abs())
        });
        if !(first.state.velocity_mps.y > 0.0
            && last.state.velocity_mps.y < 0.0
            && first.state.physics_step < apex.state.physics_step
            && apex.state.physics_step < last.state.physics_step
            && trace_minimum.is_some_and(|minimum| minimum.physics_step == apex.state.physics_step))
        {
            let capture = &mut case.airborne_captures[0];
            capture.status = "failed".into();
            capture.failures.push("real source-rest coast did not establish distinct ascending, earliest-|vy|-minimum near-apex, and descending live states".into());
            finalize_capture(capture)?;
            write_create_only(
                &cases_dir.join(format!("{}.airborne_ascending.json", case.case_id)),
                capture,
            )?;
            gate_passed = false;
            artifact.stop_reason = Some(format!(
                "real ascent/apex/descent capture condition failed: {}",
                case.case_id
            ));
            break;
        }
        let captures = [first, apex, last];
        for (role_index, role) in ROLES.iter().enumerate() {
            let capture_record = &mut case.airborne_captures[role_index];
            let live = &captures[role_index].state;
            let before = closure_matches(repo_root, expected_binding);
            capture_record.source_binding_before_sha256 = before.2;
            if !before.0 {
                capture_record.status = "failed".into();
                capture_record.failures.push(before.1.unwrap_or_else(|| {
                    "source/input closure changed before airborne regeneration".into()
                }));
            } else {
                evaluate_live_capture(
                    capture_record,
                    &context,
                    live,
                    &program,
                    baseline_search.absolute_deadline_physics_step,
                );
            }
            let after = closure_matches(repo_root, expected_binding);
            capture_record.source_binding_after_sha256 = after.2;
            capture_record.source_unchanged_before_and_after = before.0 && after.0;
            if !after.0 {
                capture_record.status = "failed".into();
                capture_record.failures.push(after.1.unwrap_or_else(|| {
                    "source/input closure changed during airborne regeneration/replay".into()
                }));
            }
            finalize_capture(capture_record)?;
            write_create_only(
                &cases_dir.join(format!("{}.airborne_{}.json", case.case_id, role)),
                capture_record,
            )?;
            if capture_record.status != "passed" {
                gate_passed = false;
                artifact.stop_reason = Some(format!(
                    "first decisive live-state compatibility failure: {} / {}",
                    case.case_id, role
                ));
                break 'cases;
            }
        }
    }
    for case in artifact.cases.iter_mut().take(DEVELOPMENT_CASE_IDS.len()) {
        for capture in &mut case.airborne_captures {
            if capture.status == "not_evaluated" {
                capture.failures.push("not evaluated: stopped at the first decisive earlier live-state capture failure".into());
                finalize_capture(capture)?;
                write_create_only(
                    &cases_dir.join(format!("{}.airborne_{}.json", case.case_id, capture.role)),
                    capture,
                )?;
            }
        }
    }
    if gate_passed {
        set_gate(
            &mut artifact.gates,
            "airborne_live_state_compatibility",
            "passed",
            Some(true),
            None,
        );
    } else {
        set_gate(
            &mut artifact.gates,
            "airborne_live_state_compatibility",
            "failed",
            Some(false),
            artifact.stop_reason.clone(),
        );
    }
    Ok(gate_passed)
}

fn empty_airborne_capture(
    case_id: &str,
    role: &str,
) -> CanonicalInitialDirectCanaryAirborneCaptureV1 {
    CanonicalInitialDirectCanaryAirborneCaptureV1 {
        case_id: case_id.into(),
        role: role.into(),
        status: "not_evaluated".into(),
        source_binding_before_sha256: None,
        source_binding_after_sha256: None,
        source_unchanged_before_and_after: false,
        live_state: None,
        full_state_snapshot: None,
        search: None,
        selected_proposal: None,
        audit: None,
        stitched_replay: None,
        search_wall_time_us: 0,
        audit_wall_time_us: 0,
        stitched_replay_wall_time_us: 0,
        failures: Vec::new(),
        identity: String::new(),
    }
}

fn evaluate_live_capture(
    capture: &mut CanonicalInitialDirectCanaryAirborneCaptureV1,
    context: &RunContext,
    live: &SimulationState,
    program: &FlightProgramV1,
    absolute_deadline: u64,
) {
    capture.status = "failed".into();
    capture.live_state = Some(crate::AirborneFlightStateV1::from_live(live));
    capture.full_state_snapshot = Some(SimulationStateSnapshotV1::from_state(live));
    let started = Instant::now();
    let search = match evaluate_airborne_nominal_direct(context, live, absolute_deadline) {
        Ok(search) => search,
        Err(error) => {
            capture
                .failures
                .push(format!("unchanged airborne generator failed: {error:#}"));
            capture.search_wall_time_us = elapsed_us(started);
            return;
        }
    };
    capture.search_wall_time_us = elapsed_us(started);
    let incoming_state = crate::AirborneFlightStateV1::from_live(live);
    if search.incoming_state != incoming_state {
        capture
            .failures
            .push("unchanged airborne search did not bind the exact captured live state".into());
        capture.search = Some(search);
        return;
    }
    let Some(proposal) = search.selected.clone() else {
        capture.failures.push(
            "unchanged airborne finite family selected no suffix from the captured live state"
                .into(),
        );
        capture.search = Some(search);
        return;
    };
    capture.search = Some(search);
    capture.selected_proposal = Some(proposal.clone());
    if proposal.incoming_state != incoming_state {
        capture.failures.push(
            "selected unchanged airborne suffix did not bind the exact captured live state".into(),
        );
        return;
    }
    let audit_started = Instant::now();
    let audit = match audit_airborne_direct_proposal(
        context,
        live,
        &proposal,
        WaypointDirectNominalDirectGenerationPolicyV1::default()
            .analytical_policy
            .minimum_clearance_m,
    ) {
        Ok(audit) => audit,
        Err(error) => {
            capture.audit_wall_time_us = elapsed_us(audit_started);
            capture
                .failures
                .push(format!("fixed-terrain airborne audit failed: {error:#}"));
            return;
        }
    };
    capture.audit_wall_time_us = elapsed_us(audit_started);
    capture.audit = Some(audit.clone());
    // The helper is called only after a real audit exists. A DTO snapshot is
    // never restored to manufacture the source-prefix replay.
    let replay_started = Instant::now();
    let stitched = run_stitched_replay(context, live, program, &proposal, Some(&audit));
    capture.stitched_replay_wall_time_us = elapsed_us(replay_started);
    let replay_passed = stitched.suffix_terminal_reached_without_fallback
        && stitched.contiguous_absolute_action_clock
        && stitched.replay_actions_match == Some(true)
        && stitched.source_replay_artifacts_match_official_replay == Some(true)
        && stitched.source_replay_final_snapshot_matches_live_audit == Some(true)
        && stitched.source_replay_contact_matches_live_audit == Some(true)
        && stitched.replay_contact_time_tick_fuel_match == Some(true)
        && stitched.replay_target_landing == Some(true)
        && stitched.replay_checkpoint_events_absent == Some(true);
    capture.stitched_replay = Some(stitched);
    if !audit.passed
        || !audit.safe_target_contact
        || !audit.ordinary_neutral_parity
        || !audit.commands_match
    {
        capture.failures.push(
            "fixed-terrain airborne audit did not preserve safe exact command coverage".into(),
        );
    }
    if !replay_passed {
        capture.failures.push("whole-source stitched replay differed in state/contact/actions/events/samples or safe target contact".into());
    }
    if capture.failures.is_empty() {
        capture.status = "passed".into();
    }
}

fn capture_collection_failure(
    case: &mut CanonicalInitialDirectCanaryCaseV1,
    cases_dir: &Path,
    role: &str,
    reason: &str,
) -> Result<bool> {
    let capture = case
        .airborne_captures
        .iter_mut()
        .find(|capture| capture.role == role)
        .context("capture slot missing")?;
    capture.status = "failed".into();
    capture.failures.push(reason.into());
    finalize_capture(capture)?;
    write_create_only(
        &cases_dir.join(format!("{}.airborne_{}.json", case.case_id, role)),
        capture,
    )?;
    Ok(false)
}

fn finalize_capture(evidence: &mut CanonicalInitialDirectCanaryAirborneCaptureV1) -> Result<()> {
    let mut identity = evidence.clone();
    identity.identity.clear();
    identity.search_wall_time_us = 0;
    identity.audit_wall_time_us = 0;
    identity.stitched_replay_wall_time_us = 0;
    evidence.identity = nominal_direct_flight_identity(&identity)?;
    Ok(())
}

fn closure_matches(
    repo_root: &Path,
    expected: &NominalDirectFlightSourceBindingV1,
) -> (bool, Option<String>, Option<String>) {
    match canonical_source_binding(repo_root) {
        Ok(binding) if &binding == expected => (true, None, Some(binding.identity_sha256)),
        Ok(binding) => (
            false,
            Some("source/input closure differs from the sealed preflight binding".into()),
            Some(binding.identity_sha256),
        ),
        Err(error) => (
            false,
            Some(format!("source/input closure check failed: {error:#}")),
            None,
        ),
    }
}

fn run_additional_physical_endpoints_gate(
    repo_root: &Path,
    expected_binding: &NominalDirectFlightSourceBindingV1,
    cases_dir: &Path,
    artifact: &mut CanonicalInitialDirectCanaryArtifactV1,
) -> Result<bool> {
    set_gate(
        &mut artifact.gates,
        "additional_physical_endpoints",
        "running",
        None,
        None,
    );
    let inputs = match load_additional_physical_endpoint_inputs(repo_root) {
        Ok(inputs) => inputs,
        Err(error) => {
            artifact.stop_reason = Some(format!(
                "sealed additional physical-input loader failed: {error:#}"
            ));
            set_gate(
                &mut artifact.gates,
                "additional_physical_endpoints",
                "failed",
                Some(false),
                artifact.stop_reason.clone(),
            );
            write_create_only(
                &cases_dir.join("additional_physical_input_failure.json"),
                &serde_json::json!({
                    "status": "failed",
                    "reason": artifact.stop_reason,
                }),
            )?;
            return Ok(false);
        }
    };
    for (offset, (case_id, request)) in inputs.into_iter().enumerate() {
        let index = DEVELOPMENT_CASE_IDS.len() + offset;
        let case = &mut artifact.cases[index];
        case.request = Some(request.clone());
        let before = closure_matches(repo_root, expected_binding);
        case.source_binding_before_sha256 = before.2;
        if before.0 {
            evaluate_development_case(case, &request)?;
        } else {
            case.status = "failed".into();
            case.failures.push(
                before
                    .1
                    .unwrap_or_else(|| "source closure changed before additional endpoint".into()),
            );
        }
        let after = closure_matches(repo_root, expected_binding);
        case.source_binding_after_sha256 = after.2;
        case.source_unchanged_before_and_after = before.0 && after.0;
        if !after.0 {
            case.status = "failed".into();
            case.failures.push(
                after
                    .1
                    .unwrap_or_else(|| "source closure changed during additional endpoint".into()),
            );
        }
        finalize_case(case)?;
        write_create_only(
            &cases_dir.join(format!("{}.source_rest.json", case.case_id)),
            case,
        )?;
        if case.status != "passed" {
            artifact.stop_reason = Some(format!(
                "first decisive additional physical endpoint failure: {case_id}"
            ));
            set_gate(
                &mut artifact.gates,
                "additional_physical_endpoints",
                "failed",
                Some(false),
                artifact.stop_reason.clone(),
            );
            return Ok(false);
        }
    }
    set_gate(
        &mut artifact.gates,
        "additional_physical_endpoints",
        "passed",
        Some(true),
        None,
    );
    Ok(true)
}

fn load_additional_physical_endpoint_inputs(
    repo_root: &Path,
) -> Result<Vec<(String, WaypointDirectNominalDirectGenerationRequest)>> {
    let bytes = fs::read(repo_root.join(CANONICAL_INITIAL_DIRECT_CANARY_ADDITIONAL_MANIFEST))
        .context("read sealed physical-only additional endpoint manifest")?;
    if sha256_bytes(&bytes)? != CANONICAL_INITIAL_DIRECT_CANARY_ADDITIONAL_SHA256 {
        bail!("additional physical endpoint manifest SHA-256 differs from the pre-code seal");
    }
    let manifest: PhysicalCompletionManifestV1 = serde_json::from_slice(&bytes)?;
    if manifest.schema_id != "nominal_direct_terminal_completion_fresh_inputs_v1"
        || manifest.schema_version != 1
        || !manifest.sealed_before_implementation
        || manifest.case_order != "735-flat,915-flat,915-uphill-75,915-downhill-75"
        || manifest.cases.len() != ADDITIONAL_CASE_IDS.len()
    {
        bail!("unsupported sealed additional physical endpoint selection manifest");
    }
    // The opaque completion/intervention/generation/terminal policy values and
    // execution-policy identity above are deliberately never read here.
    let expected = [
        ("completion_flat_span_735", 735.0, 0.0),
        ("completion_flat_span_915", 915.0, 0.0),
        ("completion_uphill_span_915", 915.0, 75.0),
        ("completion_downhill_span_915", 915.0, -75.0),
    ];
    let mut output = Vec::with_capacity(expected.len());
    for (index, input) in manifest.cases.into_iter().enumerate() {
        let (expected_id, expected_span, expected_height) = expected[index];
        if input.case_id != expected_id
            || input.source_pad_id != "pad_source"
            || input.target_pad_id != "pad_main"
        {
            bail!("additional endpoint physical scenario or pad selection mismatch at row {index}");
        }
        validate_physical_endpoint_scenario(
            &input.scenario,
            &input.source_pad_id,
            &input.target_pad_id,
            expected_span,
            expected_height,
        )?;
        output.push((
            input.case_id,
            WaypointDirectNominalDirectGenerationRequest {
                scenario: input.scenario,
                source_pad_id: input.source_pad_id,
                target_pad_id: input.target_pad_id,
                probe_id: expected_id.into(),
                // Always the normal source-rest family; never reserve policy.
                policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
            },
        ));
    }
    Ok(output)
}

fn validate_physical_endpoint_scenario(
    scenario: &pd_core::ScenarioSpec,
    source_pad_id: &str,
    target_pad_id: &str,
    expected_span: f64,
    expected_height: f64,
) -> Result<()> {
    scenario.validate().map_err(anyhow::Error::msg)?;
    if scenario.sim.physics_hz != 120
        || scenario.sim.controller_hz != 60
        || scenario.sim.max_time_s != 90.0
        || !matches!(
            scenario.mission.goal,
            pd_core::EvaluationGoal::LandingOnPad { .. }
        )
        || scenario.mission.transfer_route.is_some()
    {
        bail!("additional physical scenario is not supported route-free 120/60 source-rest input");
    }
    let context = RunContext::from_scenario(scenario).map_err(anyhow::Error::msg)?;
    let source = context
        .world
        .landing_pad(source_pad_id)
        .context("additional source pad missing")?;
    let target = context
        .world
        .landing_pad(target_pad_id)
        .context("additional target pad missing")?;
    if (target.center_x_m - source.center_x_m - expected_span).abs() > 1.0e-9
        || (target.surface_y_m - source.surface_y_m - expected_height).abs() > 1.0e-9
        || context.mission.goal.target_pad_id() != target_pad_id
        || (scenario.initial_state.position_m.x - source.center_x_m).abs() > 1.0e-9
        || (scenario.initial_state.position_m.y
            - (source.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m))
            .abs()
            > 1.0e-9
        || scenario.initial_state.velocity_mps != Vec2::new(0.0, 0.0)
        || scenario.initial_state.attitude_rad != 0.0
        || scenario.initial_state.angular_rate_radps != 0.0
    {
        bail!("additional endpoint source/target physical geometry or source-rest state mismatch");
    }
    Ok(())
}

fn empty_case(
    case_id: &str,
    cohort: &str,
    exposure_truth: &str,
    request: Option<WaypointDirectNominalDirectGenerationRequest>,
) -> CanonicalInitialDirectCanaryCaseV1 {
    CanonicalInitialDirectCanaryCaseV1 {
        case_id: case_id.into(),
        cohort: cohort.into(),
        exposure_truth: exposure_truth.into(),
        request,
        status: "not_evaluated".into(),
        search: None,
        selected_program: None,
        audit: None,
        source_departure: None,
        source_replay: None,
        terrain_twins: Vec::new(),
        airborne_captures: Vec::new(),
        source_binding_before_sha256: None,
        source_binding_after_sha256: None,
        source_unchanged_before_and_after: false,
        failures: Vec::new(),
        elapsed_wall_time_us: 0,
        identity: String::new(),
    }
}

fn evaluate_development_case(
    case: &mut CanonicalInitialDirectCanaryCaseV1,
    request: &WaypointDirectNominalDirectGenerationRequest,
) -> Result<()> {
    let started = Instant::now();
    case.status = "failed".into();
    let search = match evaluate_canonical_initial_direct(request) {
        Ok(search) => search,
        Err(error) => {
            case.failures
                .push(format!("canonical search failed: {error:#}"));
            case.elapsed_wall_time_us = elapsed_us(started);
            return Ok(());
        }
    };
    let Some(proposal) = search.selected.clone() else {
        case.failures
            .push(if let Some(rejection) = &search.preflight_rejection {
                format!("canonical preflight rejected case: {}", rejection.status())
            } else {
                "canonical finite family produced no complete selected proposal".into()
            });
        case.search = Some(search);
        case.elapsed_wall_time_us = elapsed_us(started);
        return Ok(());
    };
    case.search = Some(search);

    let context = match RunContext::from_scenario(&request.scenario) {
        Ok(context) => context,
        Err(error) => {
            case.failures
                .push(format!("run context construction failed: {error}"));
            case.elapsed_wall_time_us = elapsed_us(started);
            return Ok(());
        }
    };
    let program = match canonical_program(request, &context, &proposal) {
        Ok(program) => program,
        Err(error) => {
            case.failures
                .push(format!("terrain-bound program wrapper failed: {error:#}"));
            case.elapsed_wall_time_us = elapsed_us(started);
            return Ok(());
        }
    };
    case.selected_program = Some(program);
    let source_pad = match context.world.landing_pad(&request.source_pad_id) {
        Some(pad) => pd_plan::conservative_ballistic_bridge::PadInputV2 {
            center_x_m: pad.center_x_m,
            surface_y_m: pad.surface_y_m,
            width_m: pad.width_m,
        },
        None => {
            case.failures
                .push("source pad missing from actual context".into());
            case.elapsed_wall_time_us = elapsed_us(started);
            return Ok(());
        }
    };
    let audit = match audit_canonical_initial_direct(
        &context,
        &source_pad,
        &proposal,
        request.policy.analytical_policy.minimum_clearance_m,
    ) {
        Ok(audit) => audit,
        Err(error) => {
            case.failures
                .push(format!("actual-terrain audit failed: {error:#}"));
            case.elapsed_wall_time_us = elapsed_us(started);
            return Ok(());
        }
    };
    case.source_departure = Some(source_departure_evidence(
        &context,
        &request.source_pad_id,
        &proposal,
        &audit,
    ));
    if !case
        .source_departure
        .as_ref()
        .is_some_and(|evidence| evidence.passed)
    {
        case.failures.push(
            "selected source-rest commands did not establish the required actual forward ascent"
                .into(),
        );
    }
    if !audit.passed {
        case.failures.extend(
            audit
                .rejection_reasons
                .iter()
                .map(|reason| format!("actual-terrain audit: {reason}")),
        );
    }
    case.audit = Some(audit.clone());

    case.source_replay = Some(source_replay_evidence(&context, &proposal.updates, &audit));
    if !case
        .source_replay
        .as_ref()
        .is_some_and(|replay| replay.passed)
    {
        case.failures.push("full source-rest ordinary replay did not match audit and official actions/events/samples".into());
    }
    if case.failures.is_empty() {
        case.status = "passed".into();
    }
    case.elapsed_wall_time_us = elapsed_us(started);
    Ok(())
}

fn canonical_program(
    request: &WaypointDirectNominalDirectGenerationRequest,
    context: &RunContext,
    proposal: &CanonicalInitialProposalV1,
) -> Result<FlightProgramV1> {
    let terminal_entry = proposal
        .source_handoff_physics_step
        .checked_add(proposal.coast_tick_count)
        .context("canonical terminal-entry tick overflow")?;
    let program = FlightProgramV1 {
        schema_version: 1,
        binding: FlightProgramBindingV1::from_context(context),
        source_pad_id: request.source_pad_id.clone(),
        target_pad_id: request.target_pad_id.clone(),
        generation_policy_identity: canonical_generation_policy_identity(&request.policy)?,
        terminal_policy_identity: nominal_direct_flight_identity(
            &crate::BodyAwareTerminalPolicyV1::default(),
        )?,
        witness_identity: proposal.identity.clone(),
        source_handoff_physics_step: proposal.source_handoff_physics_step,
        terminal_entry_physics_step: terminal_entry,
        planned_end_physics_step: proposal.planned_end_physics_step,
        expected_contact_physics_step: proposal.planned_end_physics_step,
        updates: proposal.updates.clone(),
    };
    program
        .validate_against_context(context)
        .map_err(anyhow::Error::msg)?;
    Ok(program)
}

fn source_departure_evidence(
    context: &RunContext,
    source_pad_id: &str,
    proposal: &CanonicalInitialProposalV1,
    audit: &AirborneDirectAuditV1,
) -> CanonicalInitialDirectCanarySourceDepartureV1 {
    let starts_at_bound_source_rest =
        SimulationState::new(context).ok().is_some_and(|initial| {
            crate::AirborneFlightStateV1::from_live(&initial) == proposal.initial_state
        }) && context.world.landing_pad(source_pad_id).is_some_and(|pad| {
            proposal.initial_state.position_m.x == pad.center_x_m
                && proposal.initial_state.position_m.y
                    == pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m
        });
    let handoff = &proposal.source_handoff_state;
    let initial = &proposal.initial_state;
    let evidence = CanonicalInitialDirectCanarySourceDepartureV1 {
        starts_at_bound_source_rest,
        handoff_is_after_initial_state: handoff.physics_step > initial.physics_step
            && handoff.position_m.y > initial.position_m.y,
        handoff_is_forward: handoff.position_m.x > initial.position_m.x
            && handoff.velocity_mps.x > 0.0,
        handoff_is_ascending: handoff.velocity_mps.y > 0.0,
        selected_commands_reached_audit_endpoint: audit.commands_match,
        passed: false,
    };
    CanonicalInitialDirectCanarySourceDepartureV1 {
        passed: evidence.starts_at_bound_source_rest
            && evidence.handoff_is_after_initial_state
            && evidence.handoff_is_forward
            && evidence.handoff_is_ascending
            && evidence.selected_commands_reached_audit_endpoint,
        ..evidence
    }
}

fn canonical_generation_policy_identity(
    policy: &WaypointDirectNominalDirectGenerationPolicyV1,
) -> Result<String> {
    nominal_direct_flight_identity(&CanonicalProgramPolicyIdentityV1 {
        policy_id: crate::CANONICAL_INITIAL_DIRECT_POLICY_ID,
        analytical_policy: &policy.analytical_policy,
        physics_hz: policy.physics_hz,
        controller_hz: policy.controller_hz,
        launch_upright_ticks: policy.launch_upright_ticks,
        launch_tilt_ticks: policy.launch_tilt_ticks,
        source_solver_version: &policy.solver_version,
        source_solver_iterations: policy.maximum_solver_iterations,
        source_line_search_steps: policy.maximum_line_search_steps,
        source_finite_difference_step_mps2: policy.finite_difference_step_mps2,
        source_maximum_correction_abs_mps2: policy.maximum_correction_abs_mps2,
        source_initial_damping: policy.initial_damping,
        source_position_tolerance_m: policy.position_tolerance_m,
        source_velocity_tolerance_mps: policy.velocity_tolerance_mps,
        geometry_convention: &policy.geometry_convention,
        source_duration_offsets_ticks: CANONICAL_SOURCE_OFFSETS,
        terminal_duration_offsets_ticks: CANONICAL_TERMINAL_OFFSETS,
        maximum_outer_attempts: CANONICAL_MAXIMUM_OUTER_ATTEMPTS,
    })
}

pub(crate) fn source_replay_evidence(
    context: &RunContext,
    updates: &[FlightProgramUpdateV1],
    audit: &AirborneDirectAuditV1,
) -> CanonicalInitialDirectCanaryReplayV1 {
    // A real obstruction may terminate before the selected schedule endpoint.
    // Replay only the exact command prefix that the audited simulation could
    // consume; the full proposed command vector remains bound in the search.
    let executed_updates = updates
        .iter()
        .filter(|update| update.physics_step < audit.final_state.physics_step)
        .cloned()
        .collect::<Vec<_>>();
    let source = match replay_stitched_from_source(context, &executed_updates) {
        Ok(source) => source,
        Err(error) => {
            return failed_source_replay(format!("ordinary source replay failed: {error:#}"));
        }
    };
    let official = replay_simulation(
        context,
        "canonical_initial_direct_source_replay",
        &source.actions,
    );
    let (
        official_replay,
        official_actions_match,
        official_events_match,
        official_samples_match,
        official_final_clock_matches,
        official_error,
    ) = match official {
        Ok(replay) => {
            let final_clock = replay.manifest.physics_steps == source.final_state.physics_step
                && replay.manifest.sim_time_s == source.final_state.sim_time_s
                && replay.manifest.summary.fuel_remaining_kg.to_bits()
                    == source.final_state.fuel_kg.to_bits()
                && replay.manifest.end_reason == source.final_state.end_reason
                && replay.manifest.physical_outcome == source.final_state.physical_outcome
                && replay.manifest.mission_outcome == source.final_state.mission_outcome;
            (
                Some(replay.clone()),
                Some(replay.actions == source.actions),
                Some(replay.events == source.events),
                Some(replay.samples == source.samples),
                Some(final_clock),
                None,
            )
        }
        Err(error) => (
            None,
            None,
            None,
            None,
            None,
            Some(format!("official replay failed: {error:#}")),
        ),
    };
    let final_state_matches_audit = source.final_state == audit.final_state;
    let full_incoming_contact_matches_audit = source.incoming_contact == audit.incoming_contact;
    let passed = audit.ordinary_neutral_parity
        && final_state_matches_audit
        && full_incoming_contact_matches_audit
        && official_actions_match == Some(true)
        && official_events_match == Some(true)
        && official_samples_match == Some(true)
        && official_final_clock_matches == Some(true);
    CanonicalInitialDirectCanaryReplayV1 {
        final_state: Some(source.final_state),
        incoming_contact: source.incoming_contact,
        actions: source.actions,
        events: source.events,
        samples: source.samples,
        official_replay,
        official_actions_match,
        official_events_match,
        official_samples_match,
        final_state_matches_audit: Some(final_state_matches_audit),
        full_incoming_contact_matches_audit: Some(full_incoming_contact_matches_audit),
        official_final_clock_matches,
        passed,
        failure: if passed {
            None
        } else {
            official_error
                .or_else(|| Some("source/audit/official replay equality gate failed".into()))
        },
    }
}

fn failed_source_replay(failure: String) -> CanonicalInitialDirectCanaryReplayV1 {
    CanonicalInitialDirectCanaryReplayV1 {
        final_state: None,
        incoming_contact: None,
        actions: Vec::new(),
        events: Vec::new(),
        samples: Vec::new(),
        official_replay: None,
        official_actions_match: None,
        official_events_match: None,
        official_samples_match: None,
        final_state_matches_audit: None,
        full_incoming_contact_matches_audit: None,
        official_final_clock_matches: None,
        passed: false,
        failure: Some(failure),
    }
}

fn validate_selection_manifest(repo_root: &Path) -> Result<()> {
    let bytes = fs::read(repo_root.join(CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_MANIFEST))
        .context("read canonical selection manifest")?;
    if sha256_bytes(&bytes)? != CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_SHA256 {
        bail!("sealed canonical selection manifest SHA-256 mismatch");
    }
    let manifest: SelectionManifestV1 = serde_json::from_slice(&bytes)?;
    let expected_development = DEVELOPMENT_CASE_IDS
        .iter()
        .map(|id| (*id).to_owned())
        .collect::<Vec<_>>();
    let expected_additional = ADDITIONAL_CASE_IDS
        .iter()
        .map(|id| (*id).to_owned())
        .collect::<Vec<_>>();
    if manifest.schema_id != "canonical_initial_direct_canary_inputs_v1"
        || manifest.schema_version != 1
        || manifest.policy_id != "canonical_initial_direct_lowest_peak_v1"
        || manifest.development_manifest != NOMINAL_DIRECT_OPERATIONAL_FRESH_MANIFEST
        || manifest.development_case_ids != expected_development
        || manifest.additional_endpoint_manifest
            != CANONICAL_INITIAL_DIRECT_CANARY_ADDITIONAL_MANIFEST
        || manifest.additional_endpoint_case_ids != expected_additional
        || manifest.additional_input_scope
            != "Previously sealed physical endpoint inputs only; no terminal completion reserve, command extension, or hidden-input claim."
        || manifest.duration_multipliers != [0.75, 1.0, 1.25, 1.5]
        || manifest.source_duration_offsets_ticks != [-240, -180, -120, -60, 0]
        || manifest.terminal_duration_offsets_ticks != [0, 60, 120, 180, 240, 300, 360]
        || manifest.maximum_outer_attempts_per_case != 140
        || manifest.stop_rule
            != "First decisive development compatibility failure stops downstream terrain, composition, and additional-case gates; persist failure and not-evaluated reasons without tuning."
    {
        bail!("unsupported canonical initial selection manifest content");
    }
    Ok(())
}

fn input_hashes(repo_root: &Path) -> Vec<CanonicalInitialDirectCanaryInputHashV1> {
    SOURCE_CRATE_INPUTS
        .iter()
        .map(|relative_path| {
            let expected_sha256 = expected_input_sha(relative_path).to_owned();
            let observed_sha256 = fs::read(repo_root.join(relative_path))
                .ok()
                .and_then(|bytes| sha256_bytes(&bytes).ok());
            let matches_seal = observed_sha256.as_deref() == Some(expected_sha256.as_str());
            CanonicalInitialDirectCanaryInputHashV1 {
                relative_path: (*relative_path).into(),
                expected_sha256,
                observed_sha256,
                matches_seal,
            }
        })
        .collect()
}

fn expected_input_sha(path: &str) -> &'static str {
    match path {
        CANONICAL_INITIAL_DIRECT_CANARY_PROTOCOL => CANONICAL_INITIAL_DIRECT_CANARY_PROTOCOL_SHA256,
        CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_MANIFEST => {
            CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_SHA256
        }
        NOMINAL_DIRECT_OPERATIONAL_FRESH_MANIFEST => NOMINAL_DIRECT_OPERATIONAL_FRESH_SHA256,
        CANONICAL_INITIAL_DIRECT_CANARY_ADDITIONAL_MANIFEST => {
            CANONICAL_INITIAL_DIRECT_CANARY_ADDITIONAL_SHA256
        }
        CANONICAL_INITIAL_DIRECT_CANARY_OBSTACLE_MANIFEST => {
            CANONICAL_INITIAL_DIRECT_CANARY_OBSTACLE_SHA256
        }
        _ => unreachable!("only compile-time sealed input paths are requested"),
    }
}

fn canonical_source_binding(repo_root: &Path) -> Result<NominalDirectFlightSourceBindingV1> {
    let mut binding = recursive_source_binding(repo_root)?;
    for relative_path in SOURCE_CRATE_INPUTS {
        if binding
            .files
            .iter()
            .any(|file| file.relative_path == relative_path)
        {
            continue;
        }
        let bytes = fs::read(repo_root.join(relative_path))
            .with_context(|| format!("read canonical source-binding input {relative_path}"))?;
        binding.files.push(NominalDirectFlightSourceFileV1 {
            relative_path: relative_path.into(),
            sha256: sha256_bytes(&bytes)?,
        });
    }
    binding
        .files
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    binding.identity_sha256 = sha256_bytes(&serde_json::to_vec(&binding.files)?)?;
    Ok(binding)
}

fn set_gate(
    gates: &mut [CanonicalInitialDirectCanaryGateV1],
    gate_id: &str,
    status: &str,
    passed: Option<bool>,
    reason: Option<String>,
) {
    if let Some(gate) = gates.iter_mut().find(|gate| gate.gate_id == gate_id) {
        gate.status = status.into();
        gate.passed = passed;
        gate.reason = reason;
    }
}

fn mark_downstream_not_evaluated(gates: &mut [CanonicalInitialDirectCanaryGateV1], reason: &str) {
    for gate in gates
        .iter_mut()
        .filter(|gate| gate.status == "not_evaluated")
    {
        gate.reason = Some(reason.into());
    }
}

fn mark_unvisited_development_cases(cases: &mut [CanonicalInitialDirectCanaryCaseV1]) {
    for case in cases
        .iter_mut()
        .filter(|case| case.cohort == "development" && case.status == "not_evaluated")
    {
        case.failures
            .push("not evaluated: stopped at first decisive earlier development failure".into());
    }
}

fn finalize_case(case: &mut CanonicalInitialDirectCanaryCaseV1) -> Result<()> {
    for twin in &mut case.terrain_twins {
        finalize_twin(twin)?;
    }
    for capture in &mut case.airborne_captures {
        finalize_capture(capture)?;
    }
    case.identity.clear();
    case.identity = nominal_direct_flight_identity(&identity_case(case)?)?;
    Ok(())
}

fn identity_case(
    case: &CanonicalInitialDirectCanaryCaseV1,
) -> Result<CanonicalInitialDirectCanaryCaseV1> {
    let mut canonical = case.clone();
    canonical.identity.clear();
    canonical.elapsed_wall_time_us = 0;
    for twin in &mut canonical.terrain_twins {
        twin.elapsed_wall_time_us = 0;
    }
    for capture in &mut canonical.airborne_captures {
        capture.search_wall_time_us = 0;
        capture.audit_wall_time_us = 0;
        capture.stitched_replay_wall_time_us = 0;
    }
    Ok(canonical)
}

fn artifact_identity(artifact: &CanonicalInitialDirectCanaryArtifactV1) -> Result<String> {
    let mut canonical = artifact.clone();
    canonical.identity.clear();
    for case in &mut canonical.cases {
        case.elapsed_wall_time_us = 0;
        for twin in &mut case.terrain_twins {
            twin.elapsed_wall_time_us = 0;
        }
        for capture in &mut case.airborne_captures {
            capture.search_wall_time_us = 0;
            capture.audit_wall_time_us = 0;
            capture.stitched_replay_wall_time_us = 0;
        }
    }
    if let Some(discriminator) = &mut canonical.discriminator {
        discriminator.flat_control.elapsed_wall_time_us = 0;
        discriminator.late_broad.elapsed_wall_time_us = 0;
        let old = &mut discriminator.old_terrain_aware_source_only;
        old.elapsed_wall_time_us = 0;
        if let Some(compute) = &mut old.compute {
            compute.generation_wall_time_us = 0;
            compute.selected_verification_wall_time_us = 0;
            compute.ordinary_execution_wall_time_us = 0;
            compute.action_replay_wall_time_us = 0;
            compute.artifact_writing_wall_time_us = 0;
        }
    }
    nominal_direct_flight_identity(&canonical)
}

fn elapsed_us(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_selection_loader_is_strict_and_pins_order_before_evaluation() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        validate_selection_manifest(repo).unwrap();
        let bytes =
            fs::read(repo.join(CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_MANIFEST)).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<SelectionManifestV1>(value).is_err());
    }

    #[test]
    fn source_binding_closure_includes_protocol_and_both_sealed_physical_manifests() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let binding = canonical_source_binding(repo).unwrap();
        for expected in SOURCE_CRATE_INPUTS {
            assert!(
                binding
                    .files
                    .iter()
                    .any(|file| file.relative_path == expected)
            );
        }
        assert!(
            binding
                .files
                .windows(2)
                .all(|pair| pair[0].relative_path < pair[1].relative_path)
        );
    }

    #[test]
    fn additional_endpoint_loader_uses_only_the_sealed_physical_cases() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let loaded = load_additional_physical_endpoint_inputs(repo).unwrap();
        assert_eq!(
            loaded
                .iter()
                .map(|(case_id, _)| case_id.as_str())
                .collect::<Vec<_>>(),
            ADDITIONAL_CASE_IDS
        );
        for (case_id, request) in loaded {
            assert_eq!(request.probe_id, case_id);
            assert_eq!(
                request.policy,
                WaypointDirectNominalDirectGenerationPolicyV1::default()
            );
            assert!(request.scenario.mission.transfer_route.is_none());
            assert_eq!(request.scenario.sim.physics_hz, 120);
            assert_eq!(request.scenario.sim.controller_hz, 60);
            assert_eq!(
                request.scenario.initial_state.velocity_mps,
                Vec2::new(0.0, 0.0)
            );
            assert_eq!(request.scenario.initial_state.attitude_rad, 0.0);
        }
    }

    #[test]
    fn case_identity_excludes_only_observational_wall_time() {
        let mut case = empty_case("synthetic", "development", "test", None);
        case.status = "failed".into();
        case.failures.push("fixed failure cause".into());
        let identity_before =
            nominal_direct_flight_identity(&identity_case(&case).unwrap()).unwrap();
        case.elapsed_wall_time_us = 100;
        let identity_after =
            nominal_direct_flight_identity(&identity_case(&case).unwrap()).unwrap();
        assert_eq!(identity_before, identity_after);
        case.failures.push("new failure cause".into());
        assert_ne!(
            identity_before,
            nominal_direct_flight_identity(&identity_case(&case).unwrap()).unwrap()
        );
    }

    #[test]
    fn artifact_identity_excludes_embedded_case_wall_time_but_keeps_evidence() {
        let mut case = empty_case("synthetic", "development", "test", None);
        case.status = "failed".into();
        case.failures.push("fixed failure cause".into());
        case.airborne_captures
            .push(empty_airborne_capture("synthetic", "ascending"));
        finalize_case(&mut case).unwrap();
        case.elapsed_wall_time_us = 10;
        let mut artifact = CanonicalInitialDirectCanaryArtifactV1 {
            schema_id: CANONICAL_INITIAL_DIRECT_CANARY_SCHEMA_ID.into(),
            schema_version: 1,
            protocol_path: CANONICAL_INITIAL_DIRECT_CANARY_PROTOCOL.into(),
            selection_manifest_path: CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_MANIFEST.into(),
            input_hashes: Vec::new(),
            source_binding: None,
            development_case_order: vec!["synthetic".into()],
            additional_physical_case_order: Vec::new(),
            cases: vec![case],
            discriminator: None,
            gates: Vec::new(),
            setup_failures: Vec::new(),
            stop_reason: None,
            passed: false,
            identity: String::new(),
        };
        let identity_before = artifact_identity(&artifact).unwrap();
        let stale_case_identity = artifact.cases[0].identity.clone();
        artifact.cases[0].airborne_captures[0]
            .failures
            .push("capture failure discovered after source-rest write".into());
        finalize_case(&mut artifact.cases[0]).unwrap();
        let expected_case_identity =
            nominal_direct_flight_identity(&identity_case(&artifact.cases[0]).unwrap()).unwrap();
        assert_eq!(artifact.cases[0].identity, expected_case_identity);
        assert_ne!(artifact.cases[0].identity, stale_case_identity);
        let identity_after_capture = artifact_identity(&artifact).unwrap();
        assert_ne!(identity_before, identity_after_capture);
        artifact.cases[0].elapsed_wall_time_us = 99;
        artifact.cases[0].airborne_captures[0].search_wall_time_us = 123;
        assert_eq!(
            identity_after_capture,
            artifact_identity(&artifact).unwrap()
        );
        artifact.cases[0].failures.push("new evidence".into());
        finalize_case(&mut artifact.cases[0]).unwrap();
        assert_ne!(
            identity_after_capture,
            artifact_identity(&artifact).unwrap()
        );
    }

    #[test]
    fn executable_wrapper_identity_names_canonical_policy_not_old_selection_rule() {
        let base = WaypointDirectNominalDirectGenerationPolicyV1::default();
        let mut selection_only_change = base.clone();
        selection_only_change.selection_rule = "unrelated legacy ranking text".into();
        assert_eq!(
            canonical_generation_policy_identity(&base).unwrap(),
            canonical_generation_policy_identity(&selection_only_change).unwrap()
        );
        let mut numerical_change = base.clone();
        numerical_change.maximum_solver_iterations += 1;
        assert_ne!(
            canonical_generation_policy_identity(&base).unwrap(),
            canonical_generation_policy_identity(&numerical_change).unwrap()
        );
    }

    #[test]
    fn all_sealed_case_ids_are_initialized_even_before_input_loading() {
        let mut cases = DEVELOPMENT_CASE_IDS
            .iter()
            .map(|case_id| empty_case(case_id, "development", "exposed", None))
            .collect::<Vec<_>>();
        cases.extend(ADDITIONAL_CASE_IDS.iter().map(|case_id| {
            empty_case(
                case_id,
                "additional_physical_endpoint",
                "physical only, design exposed",
                None,
            )
        }));
        assert_eq!(
            cases
                .iter()
                .map(|case| case.case_id.as_str())
                .collect::<Vec<_>>(),
            DEVELOPMENT_CASE_IDS
                .into_iter()
                .chain(ADDITIONAL_CASE_IDS)
                .collect::<Vec<_>>()
        );
        assert!(cases.iter().all(|case| case.status == "not_evaluated"));
    }

    #[test]
    fn source_replay_error_never_substitutes_the_separate_audit_snapshot() {
        let evidence = failed_source_replay("synthetic source replay error".into());
        assert!(evidence.final_state.is_none());
        assert!(evidence.incoming_contact.is_none());
        assert!(!evidence.passed);
        assert!(evidence.failure.is_some());
    }

    #[test]
    fn poststep_conflict_uses_the_previous_held_phase_at_an_update_boundary() {
        let updates = [
            FlightProgramUpdateV1 {
                physics_step: 100,
                phase: "source_prefix".into(),
                command: pd_core::Command::idle(),
            },
            FlightProgramUpdateV1 {
                physics_step: 120,
                phase: "terminal".into(),
                command: pd_core::Command::idle(),
            },
        ];
        assert_eq!(phase_at_tick(&updates, 120), "source_prefix");
        assert_eq!(phase_at_tick(&updates, 121), "terminal");
    }

    #[test]
    fn first_conflict_snapshot_is_the_full_incoming_contact_state() {
        let snapshot = SimulationStateSnapshotV1 {
            sim_time_s: 0.5,
            physics_step: 60,
            position_m: Vec2::new(17.25, 91.5),
            velocity_mps: Vec2::new(1.0, -2.0),
            attitude_rad: 0.2,
            angular_rate_radps: 0.1,
            fuel_kg: 42.0,
            held_command: pd_core::Command::idle(),
            physical_outcome: PhysicalOutcome::Crashed,
            mission_outcome: MissionOutcome::FailedCrash,
            end_reason: EndReason::Crash,
            min_touchdown_clearance_m: -0.25,
            min_hull_clearance_m: -0.5,
            max_speed_mps: 4.0,
            max_abs_attitude_rad: 0.3,
            max_abs_angular_rate_radps: 0.2,
            waypoint_sequence_passed: 0,
            waypoint_sequence_first_failure_index: None,
            waypoint_handoff_window_index: None,
        };
        let incoming = pd_core::IncomingContactV1 {
            classification: pd_core::ContactClassification::Crash,
            state: snapshot.clone(),
        };
        let evidence = conflict_state_from_incoming_contact(incoming.clone());
        assert_eq!(evidence.position_m, snapshot.position_m);
        assert_eq!(evidence.state_snapshot, Some(snapshot));
        assert_eq!(evidence.incoming_contact, Some(incoming));
    }

    #[test]
    fn arbitrary_target_entry_contact_does_not_qualify_as_terrain_blockage() {
        assert!(!contact_has_away_geometric_contact(
            "stable_touchdown_on_target",
            true,
            false,
            -1.0,
            [-1.0, -1.0],
        ));
        assert!(!contact_has_away_geometric_contact(
            "off_target_touchdown",
            true,
            true,
            -1.0,
            [-1.0, -1.0],
        ));
        assert!(!contact_has_away_geometric_contact(
            "crash",
            false,
            false,
            -1.0,
            [-1.0, -1.0],
        ));
        assert!(contact_has_away_geometric_contact(
            "crash",
            true,
            false,
            -0.1,
            [1.0, 1.0],
        ));
    }

    #[test]
    fn output_root_creation_refuses_reuse() {
        let base = std::env::temp_dir().join(format!(
            "canonical-initial-create-only-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir(&base);
        reserve_output_root(&base).unwrap();
        assert!(reserve_output_root(&base).is_err());
        fs::remove_dir(&base).unwrap();
    }
}
