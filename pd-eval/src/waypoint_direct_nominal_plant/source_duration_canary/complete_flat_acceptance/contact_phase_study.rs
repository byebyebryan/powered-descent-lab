//! Bounded evaluator-only terminal-entry height sensitivity and command-coverage study.
//!
//! This module replays the 24 frozen nominal programs. It does not generate,
//! refit, or accept a new flight program.

use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result, bail};
use pd_control::{ControllerUpdateRecord, run_flight_program};
use pd_core::{
    ContactClassification, EndReason, FlightProgramV1, RunArtifacts, RunContext, SimulationState,
};
use serde::{Deserialize, Serialize};

use super::{
    ClearancePolicy, FlatPadBounds, GeometryClearanceScanEvidence, PadInputV2, PlantStateEvidence,
    body_aware_terminal_case_identity, contact_classification_label, empty_clearance_scan,
    event_contact_label, flat_pad_bounds, plant_state_evidence, record_airborne_clearance,
    same_ordinary_neutral_state, stable_digest, stable_safe_margins_pass,
    terminal_admissibility::{TerminalContactAuditEvidence, contact_audit},
};
use crate::{
    BodyAwareTerminalCaseArtifactV1, BodyAwareTerminalPolicyV1, BodyAwareTerminalWitnessV1,
    NominalDirectFlightArtifactV1, NominalDirectFlightDecisionV1,
    NominalDirectFlightRegressionArtifactV1, NominalDirectFlightRegressionCaseV1,
    NominalDirectFlightSourceBindingV1, WaypointDirectNominalDirectGenerationRequest,
    nominal_direct_flight_artifact_identity, nominal_direct_flight_identity,
    preflight_nominal_direct_flight, waypoint_direct_body_aware_terminal::sha256_bytes,
};

pub const DEFAULT_NOMINAL_DIRECT_CONTACT_PHASE_INPUT_ROOT: &str =
    "outputs/research/nominal_direct_flight_integration_20260928/gate_a";
pub const NOMINAL_DIRECT_CONTACT_PHASE_PROTOCOL: &str =
    "docs/nominal_direct_contact_phase_protocol.md";

const ROOT_SUMMARY_SHA256: &str =
    "58801ab890cc094c30d6466e3b26ef7c667a10dfdcb272c03de5b0d94167e280";
const ROOT_SUMMARY_IDENTITY: &str = "fnv1a64:3ea6e074f6de460e";
const ROOT_SUMMARY_SCHEMA: &str = "nominal_direct_flight_regression_v1";
const STUDY_SCHEMA: &str = "nominal_direct_contact_phase_study_v1";
const STUDY_SCHEMA_VERSION: u32 = 1;
const OFFSETS_M: [f64; 9] = [
    0.0, -0.001, 0.001, -0.005, 0.005, -0.010, 0.010, -0.020, 0.020,
];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectContactPhaseFileHashV1 {
    pub relative_path: String,
    pub sha256_before: String,
    pub sha256_after: Option<String>,
    pub unchanged: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectContactPhasePreflightV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub ready: bool,
    pub simulation_created: bool,
    pub generation_created: bool,
    pub input_root: String,
    pub input_summary_sha256: String,
    pub input_summary_identity: String,
    pub case_count: usize,
    pub case_ids: Vec<String>,
    pub offsets_m: Vec<f64>,
    pub input_files: Vec<NominalDirectContactPhaseFileHashV1>,
    pub source_binding_identity_sha256: String,
    pub source_files: Vec<NominalDirectContactPhaseFileHashV1>,
    pub protocol_file: NominalDirectContactPhaseFileHashV1,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectContactPhaseComputeV1 {
    pub preflight_wall_time_us: u64,
    pub baseline_wall_time_us: u64,
    pub perturbation_wall_time_us: u64,
    pub total_wall_time_us: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectContactPhaseBaselineCheckV1 {
    pub case_id: String,
    pub terminal_entry_matches_witness: bool,
    pub prefix_contact_free: bool,
    pub prefix_clearance_scan_matches_witness: bool,
    pub baseline_first_contact_matches_witness: bool,
    pub baseline_expected_tick_matches: bool,
    pub baseline_fuel_matches_witness: bool,
    pub saved_command_payload_matches: bool,
    pub ordinary_neutral_parity: bool,
    pub ordinary_flight_artifacts_match: bool,
    pub passed: bool,
    pub failure_detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectContactPhaseLaneV1 {
    pub lane: String,
    pub status: String,
    pub delta_y_m: f64,
    pub entry_state_before_offset: PlantStateEvidence,
    pub entry_state_after_offset: PlantStateEvidence,
    pub entry_held_command_before_offset: pd_core::Command,
    pub entry_held_command_after_offset: pd_core::Command,
    pub saved_command_identity: String,
    pub command_coverage_end_physics_step: u64,
    pub next_missing_update_physics_step: Option<u64>,
    pub saved_command_updates_used: u64,
    pub continuation_update_count: u64,
    pub continuation_command: Option<pd_core::Command>,
    pub post_entry_physics_steps: u64,
    pub first_contact: Option<TerminalContactAuditEvidence>,
    pub expected_contact_physics_step: u64,
    pub contact_tick_shift_physics_steps: Option<i64>,
    pub expected_contact_tick_matches: Option<bool>,
    pub stable_safe_on_target: bool,
    pub physical_stable_safe_target_contact: bool,
    pub path_clearance_passed: bool,
    pub strict_body_domain_valid_at_contact: Option<bool>,
    pub fuel_exhausted_before_contact: bool,
    pub fuel_exhaustion_physics_step: Option<u64>,
    pub terminal_entry_clearance_scan: GeometryClearanceScanEvidence,
    pub airborne_clearance_scan: GeometryClearanceScanEvidence,
    pub ordinary_neutral_parity: bool,
    pub core_predicate_mirror_parity: bool,
    pub termination_state: PlantStateEvidence,
    pub ordinary_termination_state: PlantStateEvidence,
    pub termination_state_basis: String,
    pub stop_reason: String,
    pub failure_physics_step: Option<u64>,
    pub failure_detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectContactPhaseRowV1 {
    pub case_id: String,
    pub delta_y_m: f64,
    pub saved_program_coverage: NominalDirectContactPhaseLaneV1,
    pub diagnostic_continuation: NominalDirectContactPhaseLaneV1,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectContactPhaseFailureV1 {
    pub case_id: String,
    pub delta_y_m: f64,
    pub failed_stage: String,
    pub detail: String,
    pub completed_saved_lane: Option<NominalDirectContactPhaseLaneV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectContactPhaseCountsV1 {
    pub lane: String,
    pub row_count: usize,
    pub stable_safe_target_contact_count: usize,
    pub physical_stable_safe_target_contact_count: usize,
    pub unsafe_contact_count: usize,
    pub coverage_limited_count: usize,
    pub clearance_violation_count: usize,
    pub fuel_exhaustion_before_contact_count: usize,
    pub exact_expected_tick_count: usize,
    pub earlier_contact_count: usize,
    pub later_contact_count: usize,
    pub contact_count: usize,
    pub minimum_hull_penetration_margin_m: Option<f64>,
    pub maximum_hull_penetration_margin_m: Option<f64>,
    pub minimum_dynamic_penetration_allowance_m: Option<f64>,
    pub maximum_dynamic_penetration_allowance_m: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectContactPhaseStudyV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub passed: bool,
    pub verdict: String,
    pub identity: String,
    pub baseline_gate_passed: bool,
    pub baseline_checks: Vec<NominalDirectContactPhaseBaselineCheckV1>,
    pub input_root: String,
    pub input_summary_sha256: String,
    pub input_summary_identity: String,
    pub offsets_m: Vec<f64>,
    pub expected_row_count: usize,
    pub completed_row_count: usize,
    pub rows: Vec<NominalDirectContactPhaseRowV1>,
    pub failure_observation: Option<NominalDirectContactPhaseFailureV1>,
    pub saved_program_counts: NominalDirectContactPhaseCountsV1,
    pub diagnostic_continuation_counts: NominalDirectContactPhaseCountsV1,
    pub deterministic_decision: String,
    pub input_files: Vec<NominalDirectContactPhaseFileHashV1>,
    pub source_binding_identity_before: String,
    pub source_binding_identity_after: Option<String>,
    pub source_files: Vec<NominalDirectContactPhaseFileHashV1>,
    pub protocol_file: NominalDirectContactPhaseFileHashV1,
    pub failure_detail: Option<String>,
    pub compute: NominalDirectContactPhaseComputeV1,
}

#[derive(Clone, Debug)]
pub struct NominalDirectContactPhaseStudyRunV1 {
    pub artifact: NominalDirectContactPhaseStudyV1,
    pub output_dir: PathBuf,
}

#[derive(Clone)]
struct PreparedContactPhaseCase {
    case_id: String,
    request: WaypointDirectNominalDirectGenerationRequest,
    witness: BodyAwareTerminalWitnessV1,
    program: FlightProgramV1,
    archived_run: RunArtifacts,
    archived_controller_updates: Vec<ControllerUpdateRecord>,
}

#[derive(Clone)]
struct PreparedContactPhaseStudy {
    input_root: PathBuf,
    preflight: NominalDirectContactPhasePreflightV1,
    cases: Vec<PreparedContactPhaseCase>,
    input_pins: Vec<NominalDirectContactPhaseFileHashV1>,
    source_binding: NominalDirectFlightSourceBindingV1,
    source_pins: Vec<NominalDirectContactPhaseFileHashV1>,
    protocol_pin: NominalDirectContactPhaseFileHashV1,
}

#[derive(Clone)]
struct ReplayedEntry {
    state: SimulationState,
    prefix_clearance_scan: GeometryClearanceScanEvidence,
    prefix_update_count: usize,
    prefix_command_phase: String,
}

struct BaselineReplay {
    check: NominalDirectContactPhaseBaselineCheckV1,
    entry: Option<ReplayedEntry>,
    lane: Option<NominalDirectContactPhaseLaneV1>,
}

/// Validate all frozen inputs without constructing simulation state or writing
/// files. The input root may move, but its pinned summary and 24 case bindings
/// must remain identical to the accepted gate.
pub fn preflight_nominal_direct_contact_phase_study(
    input_root: &Path,
) -> Result<NominalDirectContactPhasePreflightV1> {
    Ok(prepare_contact_phase_study(input_root)?.preflight)
}

/// Run the fixed 24 x 9 matrix into a new create-only output root.
pub fn run_nominal_direct_contact_phase_study(
    input_root: &Path,
    output_dir: &Path,
) -> Result<NominalDirectContactPhaseStudyRunV1> {
    let total_started = Instant::now();
    let preflight_started = Instant::now();
    let prepared = prepare_contact_phase_study(input_root)?;
    let preflight_wall_time_us = elapsed_us(preflight_started);

    crate::nominal_direct_flight::reserve_output_root(output_dir)?;
    fs::create_dir(output_dir.join("baseline"))?;
    fs::create_dir(output_dir.join("cases"))?;
    crate::nominal_direct_flight::write_create_only(
        &output_dir.join("preflight.json"),
        &prepared.preflight,
    )?;

    let baseline_started = Instant::now();
    let mut baseline_checks = Vec::with_capacity(prepared.cases.len());
    let mut baseline_entries = Vec::with_capacity(prepared.cases.len());
    let mut baseline_lanes = Vec::with_capacity(prepared.cases.len());
    let mut failure_detail = None;

    for (index, case) in prepared.cases.iter().enumerate() {
        let baseline = evaluate_baseline_case(case)?;
        write_case_json(
            &output_dir
                .join("baseline")
                .join(format!("{:02}_{}.json", index + 1, case.case_id)),
            &baseline.check,
        )?;
        baseline_checks.push(baseline.check.clone());
        baseline_entries.push(baseline.entry);
        baseline_lanes.push(baseline.lane.clone());
        if !baseline.check.passed {
            failure_detail = baseline
                .check
                .failure_detail
                .clone()
                .or_else(|| Some(format!("baseline gate failed for {}", case.case_id)));
            break;
        }
    }
    let baseline_wall_time_us = elapsed_us(baseline_started);
    let baseline_gate_passed = baseline_checks.len() == 24
        && baseline_checks.iter().all(|check| check.passed)
        && failure_detail.is_none();

    let mut rows = Vec::with_capacity(24 * OFFSETS_M.len());
    let mut failure_observation = None;
    let mut perturbation_started = Instant::now();
    if baseline_gate_passed {
        for (case_index, case) in prepared.cases.iter().enumerate() {
            let entry = baseline_entries[case_index]
                .as_ref()
                .context("passing baseline has no captured terminal entry")?;
            let zero_lane = baseline_lanes[case_index]
                .as_ref()
                .context("passing baseline has no saved-program lane")?;
            for delta_y_m in OFFSETS_M {
                let mut failed_stage = "row_construction";
                let mut completed_saved_lane = None;
                let attempt = (|| -> Result<NominalDirectContactPhaseRowV1> {
                    if delta_y_m == 0.0 {
                        completed_saved_lane = Some(zero_lane.clone());
                        failed_stage = "baseline_row_identity";
                        row_from_baseline(case, zero_lane.clone())
                    } else {
                        failed_stage = "offset_entry";
                        let offset_entry = apply_vertical_offset(&entry.state, delta_y_m)?;
                        let context = RunContext::from_scenario(&case.request.scenario)
                            .map_err(anyhow::Error::msg)?;
                        let policy = study_clearance_policy(&context, &case.request)?;
                        failed_stage = "saved_program_lane";
                        let saved_program_coverage = replay_lane(
                            &context,
                            &entry.state,
                            &offset_entry,
                            &entry.prefix_clearance_scan,
                            entry.prefix_update_count,
                            &entry.prefix_command_phase,
                            &case.program,
                            delta_y_m,
                            LaneMode::SavedProgram,
                            policy,
                        )?;
                        completed_saved_lane = Some(saved_program_coverage.clone());
                        failed_stage = "diagnostic_continuation_lane";
                        let diagnostic_continuation = replay_lane(
                            &context,
                            &entry.state,
                            &offset_entry,
                            &entry.prefix_clearance_scan,
                            entry.prefix_update_count,
                            &entry.prefix_command_phase,
                            &case.program,
                            delta_y_m,
                            LaneMode::DiagnosticContinuation,
                            policy,
                        )?;
                        failed_stage = "row_identity";
                        build_row(
                            case,
                            delta_y_m,
                            saved_program_coverage,
                            diagnostic_continuation,
                        )
                    }
                })();
                let row = match attempt {
                    Ok(row) => row,
                    Err(error) => {
                        let detail = format!("{error:#}");
                        failure_detail = Some(detail.clone());
                        failure_observation = Some(NominalDirectContactPhaseFailureV1 {
                            case_id: case.case_id.clone(),
                            delta_y_m,
                            failed_stage: failed_stage.into(),
                            detail,
                            completed_saved_lane,
                        });
                        crate::nominal_direct_flight::write_create_only(
                            &output_dir.join("failure.json"),
                            failure_observation.as_ref().expect("just populated"),
                        )?;
                        break;
                    }
                };
                let row_dir = output_dir.join("cases").join(&case.case_id);
                fs::create_dir_all(&row_dir)?;
                write_case_json(&row_dir.join(offset_file_name(delta_y_m)), &row)?;
                let invariant_failure = row_failure(&row);
                if let Some(reason) = &invariant_failure {
                    failure_detail = Some(reason.clone());
                    failure_observation = Some(NominalDirectContactPhaseFailureV1 {
                        case_id: case.case_id.clone(),
                        delta_y_m,
                        failed_stage: "row_invariant".into(),
                        detail: reason.clone(),
                        completed_saved_lane: Some(row.saved_program_coverage.clone()),
                    });
                    crate::nominal_direct_flight::write_create_only(
                        &output_dir.join("failure.json"),
                        failure_observation.as_ref().expect("just populated"),
                    )?;
                }
                rows.push(row);
                if invariant_failure.is_some() {
                    break;
                }
            }
            if failure_detail.is_some() {
                break;
            }
        }
    } else {
        perturbation_started = Instant::now();
    }
    let perturbation_wall_time_us = elapsed_us(perturbation_started);

    let input_files = verify_input_files_after(&prepared)?;
    let (source_binding_identity_after, source_files) = verify_source_binding_after(&prepared)?;
    let protocol_file = verify_protocol_after(&prepared)?;
    if input_files.iter().any(|file| file.unchanged != Some(true)) {
        failure_detail
            .get_or_insert_with(|| "one or more frozen input files changed during study".into());
    }
    if source_binding_identity_after.as_deref()
        != Some(prepared.source_binding.identity_sha256.as_str())
        || source_files.iter().any(|file| file.unchanged != Some(true))
    {
        failure_detail
            .get_or_insert_with(|| "one or more current source files changed during study".into());
    }
    if protocol_file.unchanged != Some(true) {
        failure_detail.get_or_insert_with(|| "study protocol changed during execution".into());
    }

    let saved_program_counts = summarize_lane_counts("saved_program_coverage", &rows, false);
    let diagnostic_continuation_counts =
        summarize_lane_counts("diagnostic_continuation", &rows, true);
    let completed_row_count = rows.len();
    let expected_row_count = 24 * OFFSETS_M.len();
    let passed = baseline_gate_passed
        && completed_row_count == expected_row_count
        && failure_detail.is_none()
        && failure_observation.is_none()
        && rows.iter().all(|row| row_failure(row).is_none())
        && input_files.iter().all(|file| file.unchanged == Some(true))
        && source_binding_identity_after.as_deref()
            == Some(prepared.source_binding.identity_sha256.as_str())
        && source_files.iter().all(|file| file.unchanged == Some(true))
        && protocol_file.unchanged == Some(true);
    let deterministic_decision = study_decision(
        &saved_program_counts,
        &diagnostic_continuation_counts,
        &rows,
        passed,
    );
    let mut artifact = NominalDirectContactPhaseStudyV1 {
        schema_id: STUDY_SCHEMA.into(),
        schema_version: STUDY_SCHEMA_VERSION,
        passed,
        verdict: if passed {
            "complete_reproducible_diagnostic".into()
        } else {
            "incomplete_or_harness_gate_failed".into()
        },
        identity: String::new(),
        baseline_gate_passed,
        baseline_checks,
        input_root: prepared.input_root.display().to_string(),
        input_summary_sha256: prepared.preflight.input_summary_sha256.clone(),
        input_summary_identity: prepared.preflight.input_summary_identity.clone(),
        offsets_m: OFFSETS_M.to_vec(),
        expected_row_count,
        completed_row_count,
        rows,
        failure_observation,
        saved_program_counts,
        diagnostic_continuation_counts,
        deterministic_decision,
        input_files,
        source_binding_identity_before: prepared.source_binding.identity_sha256.clone(),
        source_binding_identity_after,
        source_files,
        protocol_file,
        failure_detail,
        compute: NominalDirectContactPhaseComputeV1 {
            preflight_wall_time_us,
            baseline_wall_time_us,
            perturbation_wall_time_us,
            total_wall_time_us: elapsed_us(total_started),
        },
    };
    artifact.identity = contact_phase_study_identity(&artifact)?;
    crate::nominal_direct_flight::write_create_only(&output_dir.join("summary.json"), &artifact)?;
    Ok(NominalDirectContactPhaseStudyRunV1 {
        artifact,
        output_dir: output_dir.to_path_buf(),
    })
}

fn prepare_contact_phase_study(input_root: &Path) -> Result<PreparedContactPhaseStudy> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("pd-eval manifest has no repository parent")?;
    let canonical_root = input_root
        .canonicalize()
        .with_context(|| format!("resolving input root {}", input_root.display()))?;
    if !canonical_root.is_dir() {
        bail!("input root is not a directory");
    }

    let summary_path = canonical_root.join("summary.json");
    let summary_bytes = read_confined_file(&canonical_root, &summary_path)?;
    let summary_sha256 = sha256_bytes(&summary_bytes)?;
    if summary_sha256 != ROOT_SUMMARY_SHA256 {
        bail!("pinned nominal direct regression summary SHA-256 mismatch");
    }
    let root: NominalDirectFlightRegressionArtifactV1 =
        serde_json::from_slice(&summary_bytes).context("parsing pinned regression summary")?;
    if root.schema_id != ROOT_SUMMARY_SCHEMA
        || root.schema_version != 1
        || !root.passed
        || root.identity != ROOT_SUMMARY_IDENTITY
        || root.cases.len() != 24
        || root
            .cases
            .iter()
            .any(|case| !case.passed || case.status != "direct")
    {
        bail!("pinned nominal direct regression summary has invalid schema, verdict, or cases");
    }

    let mut input_pins = Vec::new();
    add_file_pin(&mut input_pins, "summary.json", &summary_bytes)?;
    let mut seen_ids = BTreeSet::new();
    let mut seen_paths = BTreeSet::new();
    let mut cases = Vec::with_capacity(24);
    for (index, regression_case) in root.cases.iter().enumerate() {
        let relative_bundle = validate_case_path_binding(
            index,
            &regression_case.case_id,
            &regression_case.relative_bundle_path,
            &mut seen_ids,
            &mut seen_paths,
        )?;
        let bundle_path = canonical_root.join(&relative_bundle);
        let canonical_bundle = bundle_path
            .canonicalize()
            .with_context(|| format!("resolving case bundle {}", relative_bundle.display()))?;
        if !canonical_bundle.starts_with(&canonical_root) || !canonical_bundle.is_dir() {
            bail!("case bundle escapes input root or is not a directory");
        }
        let request: WaypointDirectNominalDirectGenerationRequest = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "request.json",
            &mut input_pins,
        )?;
        let scenario: pd_core::ScenarioSpec = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "scenario.json",
            &mut input_pins,
        )?;
        if scenario != request.scenario {
            bail!(
                "case {} scenario differs from its generation request",
                regression_case.case_id
            );
        }
        let terminal_policy: BodyAwareTerminalPolicyV1 = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "terminal_policy.json",
            &mut input_pins,
        )?;
        if terminal_policy != BodyAwareTerminalPolicyV1::default()
            || request.policy != crate::WaypointDirectNominalDirectGenerationPolicyV1::default()
        {
            bail!(
                "case {} uses a changed generation or terminal policy",
                regression_case.case_id
            );
        }
        let generation: BodyAwareTerminalCaseArtifactV1 = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "generation.json",
            &mut input_pins,
        )?;
        let decision: NominalDirectFlightDecisionV1 = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "decision.json",
            &mut input_pins,
        )?;
        let program: FlightProgramV1 = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "program.json",
            &mut input_pins,
        )?;
        let witness: BodyAwareTerminalWitnessV1 = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "witness.json",
            &mut input_pins,
        )?;
        let safety_audit: crate::BodyAwareTerminalVerificationV1 = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "safety_audit.json",
            &mut input_pins,
        )?;
        let flight: NominalDirectFlightArtifactV1 = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "summary.json",
            &mut input_pins,
        )?;
        let manifest: pd_core::RunManifest = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "manifest.json",
            &mut input_pins,
        )?;
        let actions: Vec<pd_core::ActionLogEntry> = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "actions.json",
            &mut input_pins,
        )?;
        let events: Vec<pd_core::EventRecord> = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "events.json",
            &mut input_pins,
        )?;
        let samples: Vec<pd_core::SampleRecord> = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "samples.json",
            &mut input_pins,
        )?;
        let archived_controller_updates: Vec<ControllerUpdateRecord> = read_case_json(
            &canonical_root,
            &canonical_bundle,
            &relative_bundle,
            "controller_updates.json",
            &mut input_pins,
        )?;
        let archived_run = RunArtifacts {
            manifest: manifest.clone(),
            actions,
            events,
            samples,
        };

        validate_case_bindings(
            regression_case,
            &request,
            &terminal_policy,
            &generation,
            &decision,
            &program,
            &witness,
            &safety_audit,
            &flight,
            &archived_run,
            &archived_controller_updates,
        )?;

        cases.push(PreparedContactPhaseCase {
            case_id: regression_case.case_id.clone(),
            request,
            witness,
            program,
            archived_run,
            archived_controller_updates,
        });
    }

    let source_binding = crate::source_binding(repo_root)?;
    let source_pins = source_binding
        .files
        .iter()
        .map(|file| NominalDirectContactPhaseFileHashV1 {
            relative_path: file.relative_path.clone(),
            sha256_before: file.sha256.clone(),
            sha256_after: None,
            unchanged: None,
        })
        .collect::<Vec<_>>();
    let protocol_pin = hash_repo_file(repo_root, NOMINAL_DIRECT_CONTACT_PHASE_PROTOCOL)?;
    let input_files = input_pins.clone();
    let preflight = NominalDirectContactPhasePreflightV1 {
        schema_id: "nominal_direct_contact_phase_preflight_v1".into(),
        schema_version: 1,
        ready: true,
        simulation_created: false,
        generation_created: false,
        input_root: canonical_root.display().to_string(),
        input_summary_sha256: summary_sha256,
        input_summary_identity: root.identity.clone(),
        case_count: cases.len(),
        case_ids: cases.iter().map(|case| case.case_id.clone()).collect(),
        offsets_m: OFFSETS_M.to_vec(),
        input_files,
        source_binding_identity_sha256: source_binding.identity_sha256.clone(),
        source_files: source_pins.clone(),
        protocol_file: protocol_pin.clone(),
    };
    Ok(PreparedContactPhaseStudy {
        input_root: canonical_root,
        preflight,
        cases,
        input_pins,
        source_binding,
        source_pins,
        protocol_pin,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_case_bindings(
    regression_case: &NominalDirectFlightRegressionCaseV1,
    request: &WaypointDirectNominalDirectGenerationRequest,
    terminal_policy: &BodyAwareTerminalPolicyV1,
    generation: &BodyAwareTerminalCaseArtifactV1,
    decision: &NominalDirectFlightDecisionV1,
    program: &FlightProgramV1,
    witness: &BodyAwareTerminalWitnessV1,
    safety_audit: &crate::BodyAwareTerminalVerificationV1,
    flight: &NominalDirectFlightArtifactV1,
    archived_run: &RunArtifacts,
    archived_controller_updates: &[ControllerUpdateRecord],
) -> Result<()> {
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let input_gate = preflight_nominal_direct_flight(request, terminal_policy);
    if !input_gate.supported || input_gate.simulation_created {
        bail!(
            "case {} fails nominal flight no-simulation input preflight",
            regression_case.case_id
        );
    }
    program
        .validate_against_context(&context)
        .map_err(anyhow::Error::msg)
        .context("saved flight program context or clock validation failed")?;
    if generation.schema_id != "waypoint_direct_body_aware_terminal_case_v1"
        || generation.schema_version != 1
        || !generation.passed
        || generation.request != *request
        || generation.terminal_policy != *terminal_policy
        || body_aware_terminal_case_identity(generation)? != generation.identity
    {
        bail!(
            "case {} generation payload or identity is invalid",
            regression_case.case_id
        );
    }
    if regression_case.generation_identity.as_deref() != Some(generation.identity.as_str())
        || regression_case.selected_row_index != generation.selected_row_index
        || regression_case.witness_identity.as_deref() != Some(witness.identity.as_str())
        || generation.selected_row_index != Some(witness.row_index)
    {
        bail!(
            "case {} regression summary does not bind its selected witness",
            regression_case.case_id
        );
    }
    let selected_row = generation
        .rows
        .get(witness.row_index)
        .context("selected witness row is outside generation ledger")?;
    if !selected_row.accepted || selected_row.witness.as_ref() != Some(witness) {
        bail!(
            "case {} generation ledger does not contain its exact selected witness",
            regression_case.case_id
        );
    }
    let mut canonical_witness = witness.clone();
    canonical_witness.identity.clear();
    if stable_digest(&canonical_witness)? != witness.identity
        || witness.request_identity != nominal_direct_flight_identity(request)?
        || witness.terminal_policy_identity != nominal_direct_flight_identity(terminal_policy)?
    {
        bail!(
            "case {} witness identity or policy/request binding is invalid",
            regression_case.case_id
        );
    }

    let NominalDirectFlightDecisionV1::Direct {
        generation_identity,
        selected_row_index,
        program_identity,
        program: decided_program,
    } = decision
    else {
        bail!(
            "case {} archived decision is not Direct",
            regression_case.case_id
        );
    };
    if decision != &flight.decision
        || generation_identity != &generation.identity
        || *selected_row_index != witness.row_index
        || decided_program.as_ref() != program
        || *program_identity != nominal_direct_flight_identity(program)?
        || regression_case.program_identity.as_deref() != Some(program_identity)
    {
        bail!(
            "case {} saved decision and program identities disagree",
            regression_case.case_id
        );
    }
    validate_program_witness_conversion(request, terminal_policy, program, witness)?;

    if flight.schema_id != "nominal_direct_flight_v1"
        || flight.schema_version != 1
        || flight.request_identity != nominal_direct_flight_identity(request)?
        || nominal_direct_flight_artifact_identity(flight)? != flight.identity
    {
        bail!(
            "case {} nominal flight summary identity or request binding is invalid",
            regression_case.case_id
        );
    }
    let execution = flight
        .execution
        .as_ref()
        .context("archived nominal flight has no ordinary execution evidence")?;
    if !execution.passed
        || execution.program_identity != nominal_direct_flight_identity(program)?
        || execution.witness_identity != witness.identity
        || execution.safety_audit != *safety_audit
        || execution.safety_audit != witness.verification
        || execution.manifest != archived_run.manifest
        || nominal_direct_flight_identity(archived_run)? != execution.ordinary_run_identity
        || regression_case.ordinary_run_identity.as_deref()
            != Some(execution.ordinary_run_identity.as_str())
    {
        bail!(
            "case {} archived execution evidence does not bind the selected program and run",
            regression_case.case_id
        );
    }
    if !execution.safe_target_landing
        || !execution.exact_command_and_clock_parity
        || !execution.exact_contact_tick_and_fuel_parity
        || !execution.ordinary_action_replay_parity
        || regression_case.expected_contact_physics_step
            != Some(witness.verification.physics_ticks_advanced)
        || regression_case.observed_contact_physics_step
            != Some(archived_run.manifest.physics_steps)
    {
        bail!(
            "case {} regression summary lacks exact accepted execution checks",
            regression_case.case_id
        );
    }
    let witness_contact = witness
        .verification
        .first_contact
        .as_ref()
        .context("selected witness has no authoritative contact audit")?;
    if witness_contact.state.physics_step != program.expected_contact_physics_step
        || archived_run.manifest.physics_steps != program.expected_contact_physics_step
        || archived_run.manifest.summary.fuel_remaining_kg != witness_contact.state.fuel_kg
    {
        bail!(
            "case {} expected contact tick/fuel binding is inconsistent",
            regression_case.case_id
        );
    }
    if archived_controller_updates.len() != program.updates.len()
        || archived_run.actions.len() != program.updates.len()
        || archived_run
            .actions
            .iter()
            .zip(&program.updates)
            .enumerate()
            .any(|(index, (action, update))| {
                action.physics_step != update.physics_step
                    || action.command != update.command
                    || action.controller_update_index != index as u64
            })
    {
        bail!(
            "case {} archived action payload does not reproduce program updates",
            regression_case.case_id
        );
    }
    Ok(())
}

fn validate_program_witness_conversion(
    request: &WaypointDirectNominalDirectGenerationRequest,
    terminal_policy: &BodyAwareTerminalPolicyV1,
    program: &FlightProgramV1,
    witness: &BodyAwareTerminalWitnessV1,
) -> Result<()> {
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let source_handoff = request
        .policy
        .launch_upright_ticks
        .checked_add(request.policy.launch_tilt_ticks)
        .and_then(|ticks| ticks.checked_add(witness.source_bridge_tick_count))
        .context("source handoff tick overflow")?;
    let terminal_entry = source_handoff
        .checked_add(witness.coast_tick_count)
        .context("terminal entry tick overflow")?;
    let planned_end = terminal_entry
        .checked_add(witness.reference.physics_ticks)
        .and_then(|ticks| ticks.checked_add(terminal_policy.maximum_after_reference_ticks))
        .context("planned end tick overflow")?;
    if program.source_pad_id != request.source_pad_id
        || program.target_pad_id != request.target_pad_id
        || program.generation_policy_identity != nominal_direct_flight_identity(&request.policy)?
        || program.terminal_policy_identity != nominal_direct_flight_identity(terminal_policy)?
        || program.witness_identity != witness.identity
        || program.source_handoff_physics_step != source_handoff
        || program.terminal_entry_physics_step != terminal_entry
        || program.terminal_entry_physics_step != witness.terminal_entry.physics_step
        || program.planned_end_physics_step != planned_end
        || program.planned_end_physics_step != witness.planned_physics_tick_count
        || program.expected_contact_physics_step != witness.verification.physics_ticks_advanced
        || program.updates.len() != witness.commands.len()
    {
        bail!("saved program phase bounds or witness bindings disagree");
    }
    for (update, witness_update) in program.updates.iter().zip(&witness.commands) {
        if update.physics_step.checked_add(1) != Some(witness_update.physics_step)
            || update.phase != witness_update.phase
            || update.command != witness_update.command
        {
            bail!("saved program is not the exact post-step to pre-step witness conversion");
        }
    }
    if program.expected_contact_physics_step > program.planned_end_physics_step
        || program.planned_end_physics_step
            > (context.sim.max_time_s * f64::from(context.sim.physics_hz)).ceil() as u64
    {
        bail!("saved program contact or planned end exceeds its declared horizon");
    }
    Ok(())
}

fn validate_case_id(case_id: &str) -> Result<()> {
    if case_id.is_empty()
        || !case_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        bail!("case ID contains unsupported path characters");
    }
    Ok(())
}

fn validate_case_path_binding(
    index: usize,
    case_id: &str,
    relative_bundle_path: &str,
    seen_ids: &mut BTreeSet<String>,
    seen_paths: &mut BTreeSet<String>,
) -> Result<PathBuf> {
    validate_case_id(case_id)?;
    let expected_bundle = format!("cases/{:02}_{case_id}", index + 1);
    if relative_bundle_path != expected_bundle
        || !seen_ids.insert(case_id.to_owned())
        || !seen_paths.insert(relative_bundle_path.to_owned())
    {
        bail!("duplicate, reordered, or path-invalid regression case binding");
    }
    validate_bundle_relative_path(relative_bundle_path)
}

fn validate_bundle_relative_path(value: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().count() != 2
        || !matches!(path.components().next(), Some(Component::Normal(name)) if name == "cases")
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("case bundle path must be a confined cases/<bundle> relative path");
    }
    Ok(path.to_path_buf())
}

fn read_case_json<T: for<'de> Deserialize<'de>>(
    canonical_root: &Path,
    canonical_bundle: &Path,
    relative_bundle: &Path,
    file_name: &str,
    pins: &mut Vec<NominalDirectContactPhaseFileHashV1>,
) -> Result<T> {
    let relative = relative_bundle.join(file_name);
    let path = canonical_root.join(&relative);
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("resolving case input {}", relative.display()))?;
    if !canonical_path.starts_with(canonical_root)
        || !canonical_path.starts_with(canonical_bundle)
        || !canonical_path.is_file()
    {
        bail!("case input escapes its declared bundle");
    }
    let bytes = fs::read(&canonical_path)
        .with_context(|| format!("reading case input {}", relative.display()))?;
    add_file_pin(pins, &relative.to_string_lossy(), &bytes)?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing case input {}", relative.display()))
}

fn read_confined_file(canonical_root: &Path, path: &Path) -> Result<Vec<u8>> {
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("resolving input file {}", path.display()))?;
    if !canonical_path.starts_with(canonical_root) || !canonical_path.is_file() {
        bail!("input file escapes its declared root or is not a regular file");
    }
    fs::read(&canonical_path).with_context(|| format!("reading input file {}", path.display()))
}

fn add_file_pin(
    pins: &mut Vec<NominalDirectContactPhaseFileHashV1>,
    relative_path: &str,
    bytes: &[u8],
) -> Result<()> {
    if pins.iter().any(|pin| pin.relative_path == relative_path) {
        bail!("duplicate input file binding: {relative_path}");
    }
    pins.push(NominalDirectContactPhaseFileHashV1 {
        relative_path: relative_path.to_owned(),
        sha256_before: sha256_bytes(bytes)?,
        sha256_after: None,
        unchanged: None,
    });
    Ok(())
}

fn hash_repo_file(
    repo_root: &Path,
    relative_path: &str,
) -> Result<NominalDirectContactPhaseFileHashV1> {
    let path = repo_root.join(relative_path);
    let bytes =
        fs::read(&path).with_context(|| format!("reading source binding {}", path.display()))?;
    Ok(NominalDirectContactPhaseFileHashV1 {
        relative_path: relative_path.to_owned(),
        sha256_before: sha256_bytes(&bytes)?,
        sha256_after: None,
        unchanged: None,
    })
}

fn verify_input_files_after(
    prepared: &PreparedContactPhaseStudy,
) -> Result<Vec<NominalDirectContactPhaseFileHashV1>> {
    prepared
        .input_pins
        .iter()
        .map(|pin| verify_pinned_input_file(&prepared.input_root, pin))
        .collect()
}

fn verify_pinned_input_file(
    input_root: &Path,
    pin: &NominalDirectContactPhaseFileHashV1,
) -> Result<NominalDirectContactPhaseFileHashV1> {
    let relative = validate_input_relative_path(&pin.relative_path)?;
    let bytes = read_confined_file(input_root, &input_root.join(relative))?;
    let after = sha256_bytes(&bytes)?;
    Ok(NominalDirectContactPhaseFileHashV1 {
        relative_path: pin.relative_path.clone(),
        sha256_before: pin.sha256_before.clone(),
        unchanged: Some(after == pin.sha256_before),
        sha256_after: Some(after),
    })
}

fn verify_source_binding_after(
    prepared: &PreparedContactPhaseStudy,
) -> Result<(Option<String>, Vec<NominalDirectContactPhaseFileHashV1>)> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("pd-eval manifest has no repository parent")?;
    let current = crate::source_binding(repo_root)?;
    let current_files = current
        .files
        .iter()
        .map(|file| (file.relative_path.as_str(), file.sha256.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let files = prepared
        .source_pins
        .iter()
        .map(|pin| {
            let after = current_files.get(pin.relative_path.as_str()).copied();
            Ok(NominalDirectContactPhaseFileHashV1 {
                relative_path: pin.relative_path.clone(),
                sha256_before: pin.sha256_before.clone(),
                sha256_after: after.map(str::to_owned),
                unchanged: after.map(|sha| sha == pin.sha256_before),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((Some(current.identity_sha256), files))
}

fn verify_protocol_after(
    prepared: &PreparedContactPhaseStudy,
) -> Result<NominalDirectContactPhaseFileHashV1> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("pd-eval manifest has no repository parent")?;
    hash_file_after(repo_root, &prepared.protocol_pin)
}

fn hash_file_after(
    root: &Path,
    pin: &NominalDirectContactPhaseFileHashV1,
) -> Result<NominalDirectContactPhaseFileHashV1> {
    let relative = validate_input_relative_path(&pin.relative_path)?;
    let bytes = fs::read(root.join(relative))
        .with_context(|| format!("rereading source binding {}", pin.relative_path))?;
    let after = sha256_bytes(&bytes)?;
    Ok(NominalDirectContactPhaseFileHashV1 {
        relative_path: pin.relative_path.clone(),
        sha256_before: pin.sha256_before.clone(),
        unchanged: Some(after == pin.sha256_before),
        sha256_after: Some(after),
    })
}

fn validate_input_relative_path(value: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("input binding path is not confined and relative");
    }
    Ok(path.to_path_buf())
}

fn evaluate_baseline_case(case: &PreparedContactPhaseCase) -> Result<BaselineReplay> {
    let mut check = NominalDirectContactPhaseBaselineCheckV1 {
        case_id: case.case_id.clone(),
        terminal_entry_matches_witness: false,
        prefix_contact_free: false,
        prefix_clearance_scan_matches_witness: false,
        baseline_first_contact_matches_witness: false,
        baseline_expected_tick_matches: false,
        baseline_fuel_matches_witness: false,
        saved_command_payload_matches: false,
        ordinary_neutral_parity: false,
        ordinary_flight_artifacts_match: false,
        passed: false,
        failure_detail: None,
    };

    let attempt = (|| -> Result<(ReplayedEntry, NominalDirectContactPhaseLaneV1, bool)> {
        let context =
            RunContext::from_scenario(&case.request.scenario).map_err(anyhow::Error::msg)?;
        let clearance_policy = study_clearance_policy(&context, &case.request)?;
        let entry = reconstruct_terminal_entry(case, &context, clearance_policy)?;
        check.prefix_contact_free = true;
        check.terminal_entry_matches_witness =
            plant_state_evidence(&entry.state, &context) == case.witness.terminal_entry;
        let offset_entry = entry.state.clone();
        let lane = replay_lane(
            &context,
            &entry.state,
            &offset_entry,
            &entry.prefix_clearance_scan,
            entry.prefix_update_count,
            &entry.prefix_command_phase,
            &case.program,
            0.0,
            LaneMode::SavedProgram,
            clearance_policy,
        )?;
        let expected_contact = case
            .witness
            .verification
            .first_contact
            .as_ref()
            .context("baseline witness has no first-contact evidence")?;
        check.baseline_first_contact_matches_witness =
            lane.first_contact.as_ref() == Some(expected_contact);
        check.baseline_expected_tick_matches = lane.expected_contact_tick_matches == Some(true);
        check.baseline_fuel_matches_witness = lane
            .first_contact
            .as_ref()
            .is_some_and(|contact| contact.state.fuel_kg == expected_contact.state.fuel_kg)
            && lane.first_contact.as_ref().is_some_and(|contact| {
                contact.state.physics_step == expected_contact.state.physics_step
            });
        check.prefix_clearance_scan_matches_witness =
            lane.airborne_clearance_scan == case.witness.verification.clearance_scan;
        check.saved_command_payload_matches = lane.saved_command_updates_used
            + u64::try_from(entry.prefix_update_count).unwrap_or(u64::MAX)
            == case.program.updates.len() as u64
            && lane.continuation_update_count == 0
            && lane.stop_reason == "authoritative_first_contact";
        check.ordinary_neutral_parity =
            lane.ordinary_neutral_parity && lane.core_predicate_mirror_parity;

        let ordinary_flight_artifacts_match = compare_archived_ordinary_flight(case, &context)?;
        check.ordinary_flight_artifacts_match = ordinary_flight_artifacts_match;
        check.passed = check.terminal_entry_matches_witness
            && check.prefix_contact_free
            && check.prefix_clearance_scan_matches_witness
            && check.baseline_first_contact_matches_witness
            && check.baseline_expected_tick_matches
            && check.baseline_fuel_matches_witness
            && check.saved_command_payload_matches
            && check.ordinary_neutral_parity
            && check.ordinary_flight_artifacts_match;
        if !check.passed {
            check.failure_detail = Some(first_failed_baseline_check(&check));
        }
        Ok((entry, lane, ordinary_flight_artifacts_match))
    })();

    match attempt {
        Ok((entry, lane, _ordinary_match)) => Ok(BaselineReplay {
            check,
            entry: Some(entry),
            lane: Some(lane),
        }),
        Err(error) => {
            check.passed = false;
            check.failure_detail = Some(format!("{error:#}"));
            Ok(BaselineReplay {
                check,
                entry: None,
                lane: None,
            })
        }
    }
}

fn first_failed_baseline_check(check: &NominalDirectContactPhaseBaselineCheckV1) -> String {
    [
        (
            check.terminal_entry_matches_witness,
            "replayed terminal entry differs from archived witness",
        ),
        (
            check.prefix_contact_free,
            "source/coast prefix is not contact-free",
        ),
        (
            check.prefix_clearance_scan_matches_witness,
            "baseline pointwise clearance scan differs from witness",
        ),
        (
            check.baseline_first_contact_matches_witness,
            "zero-offset first-contact audit differs from witness",
        ),
        (
            check.baseline_expected_tick_matches,
            "zero-offset contact does not match expected physics tick",
        ),
        (
            check.baseline_fuel_matches_witness,
            "zero-offset contact fuel differs from witness",
        ),
        (
            check.saved_command_payload_matches,
            "zero-offset replay did not consume exact saved command coverage",
        ),
        (
            check.ordinary_neutral_parity,
            "zero-offset ordinary/neutral replay parity failed",
        ),
        (
            check.ordinary_flight_artifacts_match,
            "ordinary flight artifacts differ from archived execution",
        ),
    ]
    .into_iter()
    .find_map(|(passed, message)| (!passed).then_some(message))
    .unwrap_or("baseline gate failed")
    .to_owned()
}

fn compare_archived_ordinary_flight(
    case: &PreparedContactPhaseCase,
    context: &RunContext,
) -> Result<bool> {
    let ordinary = run_flight_program(context, &case.program)
        .map_err(anyhow::Error::msg)
        .context("replaying archived ordinary flight program")?;
    let replay = pd_core::replay_simulation(
        context,
        &ordinary.run.manifest.controller_id,
        &ordinary.run.actions,
    )
    .context("replaying archived ordinary action log")?;
    let mut expected_controller = case.archived_controller_updates.clone();
    let mut observed_controller = ordinary.controller_updates.clone();
    for record in &mut expected_controller {
        record.compute_time_us = None;
    }
    for record in &mut observed_controller {
        record.compute_time_us = None;
    }
    Ok(ordinary.run == case.archived_run
        && replay == ordinary.run
        && expected_controller == observed_controller)
}

fn reconstruct_terminal_entry(
    case: &PreparedContactPhaseCase,
    context: &RunContext,
    clearance_policy: ClearancePolicy,
) -> Result<ReplayedEntry> {
    let mut neutral = SimulationState::new(context)?;
    let mut ordinary = neutral.clone();
    let mut scan = empty_clearance_scan();
    let mut update_index = 0_usize;
    let mut phase = String::new();
    let interval = context.sim.control_interval_steps();
    while neutral.physics_step < case.program.terminal_entry_physics_step {
        if neutral.physics_step.is_multiple_of(interval) {
            let update = case
                .program
                .updates
                .get(update_index)
                .context("saved source/coast program ended before terminal entry")?;
            if update.physics_step != neutral.physics_step {
                bail!(
                    "source/coast update clock mismatch at step {}: found {}",
                    neutral.physics_step,
                    update.physics_step
                );
            }
            phase.clone_from(&update.phase);
            neutral.set_command(update.command);
            ordinary.set_command(update.command);
            update_index += 1;
        }
        let prior_step = neutral.physics_step;
        let classification = neutral.step_physics_and_classify_contact(context);
        let events = ordinary.step(context);
        scan.poststep_state_count += 1;
        if classification != ContactClassification::None {
            bail!(
                "source/coast prefix contacted at physics step {}",
                neutral.physics_step
            );
        }
        if !same_ordinary_neutral_state(&ordinary, &neutral, &classification)
            || event_contact_label(&events) != contact_classification_label(&classification)
        {
            bail!(
                "ordinary/neutral source/coast parity failed at physics step {}",
                neutral.physics_step
            );
        }
        validate_live_state(context, &neutral, prior_step)?;
        record_airborne_clearance(
            context,
            &neutral,
            neutral.physics_step,
            &phase,
            clearance_policy,
            &mut scan,
        );
    }
    if neutral.physics_step != case.witness.terminal_entry.physics_step
        || ordinary.is_terminal()
        || neutral.is_terminal()
    {
        bail!("terminal entry clock or outcome state differs from the witness boundary");
    }
    Ok(ReplayedEntry {
        state: neutral,
        prefix_clearance_scan: scan,
        prefix_update_count: update_index,
        prefix_command_phase: phase,
    })
}

fn study_clearance_policy(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
) -> Result<ClearancePolicy> {
    let to_bounds = |pad_id: &str| -> Result<FlatPadBounds> {
        let pad = context
            .world
            .landing_pad(pad_id)
            .with_context(|| format!("study pad {pad_id} is missing"))?;
        Ok(flat_pad_bounds(
            context,
            &PadInputV2 {
                center_x_m: pad.center_x_m,
                surface_y_m: pad.surface_y_m,
                width_m: pad.width_m,
            },
        ))
    };
    let source_pad = to_bounds(&request.source_pad_id)?;
    let target_pad = to_bounds(&request.target_pad_id)?;
    if !source_pad.flat || !target_pad.flat {
        bail!("contact phase study requires the frozen flat source and target pads");
    }
    Ok(ClearancePolicy {
        source_pad,
        target_pad,
        minimum_clearance_m: request.policy.analytical_policy.minimum_clearance_m,
    })
}

fn apply_vertical_offset(state: &SimulationState, delta_y_m: f64) -> Result<SimulationState> {
    if !delta_y_m.is_finite() {
        bail!("terminal-entry vertical offset must be finite");
    }
    let mut shifted = state.clone();
    shifted.position_m.y += delta_y_m;
    if !shifted.position_m.y.is_finite() {
        bail!("terminal-entry vertical offset produced a nonfinite state");
    }
    Ok(shifted)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LaneMode {
    SavedProgram,
    DiagnosticContinuation,
}

impl LaneMode {
    fn label(self) -> &'static str {
        match self {
            Self::SavedProgram => "saved_program_coverage",
            Self::DiagnosticContinuation => "diagnostic_continuation",
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn replay_lane(
    context: &RunContext,
    entry_before_offset: &SimulationState,
    entry_after_offset: &SimulationState,
    prefix_clearance_scan: &GeometryClearanceScanEvidence,
    prefix_update_count: usize,
    prefix_command_phase: &str,
    program: &FlightProgramV1,
    delta_y_m: f64,
    mode: LaneMode,
    clearance_policy: ClearancePolicy,
) -> Result<NominalDirectContactPhaseLaneV1> {
    let interval = context.sim.control_interval_steps();
    if entry_after_offset.physics_step != program.terminal_entry_physics_step
        || entry_after_offset.physics_step != entry_before_offset.physics_step
        || !entry_after_offset.physics_step.is_multiple_of(interval)
    {
        bail!("lane entry is off the saved global control/terminal clock");
    }
    let mut neutral = entry_after_offset.clone();
    let mut ordinary = entry_after_offset.clone();
    let mut scan = prefix_clearance_scan.clone();
    // The offset is applied between the archived prefix scan and this lane.
    // Audit that seam explicitly without stepping; the zero-offset baseline
    // keeps its archived scan counts and equality unchanged.
    let terminal_entry_clearance_scan =
        terminal_entry_clearance_scan(context, entry_after_offset, delta_y_m, clearance_policy);
    let mut update_index = prefix_update_count;
    let mut saved_updates_used = 0_u64;
    let mut continuation_updates = 0_u64;
    let mut continuation_command = None;
    let mut current_phase = prefix_command_phase.to_owned();
    let mut ordinary_neutral_parity = true;
    let mut core_predicate_mirror_parity = true;
    let mut first_contact = None;
    let mut next_missing_update = None;
    let mut failure_detail = None;
    let mut failure_physics_step = None;
    let mut post_entry_steps = 0_u64;
    let mut fuel_exhaustion_physics_step = None;
    let horizon_steps = (context.sim.max_time_s * f64::from(context.sim.physics_hz)).ceil() as u64;
    let command_coverage_end = program
        .updates
        .last()
        .and_then(|update| update.physics_step.checked_add(interval))
        .unwrap_or(program.expected_contact_physics_step);

    let stop_reason = loop {
        if neutral.physics_step >= program.planned_end_physics_step
            || neutral.physics_step >= horizon_steps
        {
            break if neutral.physics_step >= horizon_steps {
                "scenario_horizon_cap"
            } else {
                "planned_end_cap"
            }
            .to_owned();
        }
        if neutral.physics_step.is_multiple_of(interval) {
            if let Some(update) = program.updates.get(update_index) {
                if update.physics_step != neutral.physics_step {
                    failure_physics_step = Some(neutral.physics_step);
                    failure_detail = Some(format!(
                        "saved command clock expected {}, reached {}",
                        update.physics_step, neutral.physics_step
                    ));
                    break "harness_command_clock_failure".to_owned();
                }
                current_phase.clone_from(&update.phase);
                neutral.set_command(update.command);
                ordinary.set_command(update.command);
                update_index += 1;
                saved_updates_used += 1;
            } else {
                next_missing_update.get_or_insert(neutral.physics_step);
                match mode {
                    LaneMode::SavedProgram => {
                        break "next_saved_update_missing".to_owned();
                    }
                    LaneMode::DiagnosticContinuation => {
                        let Some(command) = program.updates.last().map(|update| update.command)
                        else {
                            failure_physics_step = Some(neutral.physics_step);
                            failure_detail = Some("continuation has no final saved command".into());
                            break "harness_missing_final_command".to_owned();
                        };
                        continuation_command = Some(command);
                        neutral.set_command(command);
                        ordinary.set_command(command);
                        continuation_updates += 1;
                    }
                }
            }
        }
        let prior_step = neutral.physics_step;
        let classification = neutral.step_physics_and_classify_contact(context);
        let events = ordinary.step(context);
        post_entry_steps += 1;
        scan.poststep_state_count += 1;
        if !same_ordinary_neutral_state(&ordinary, &neutral, &classification)
            || event_contact_label(&events) != contact_classification_label(&classification)
        {
            ordinary_neutral_parity = false;
            failure_physics_step = Some(neutral.physics_step);
            failure_detail = Some("ordinary/neutral physical state or event parity failed".into());
            break "harness_parity_failure".to_owned();
        }
        if let Err(error) = validate_live_state(context, &neutral, prior_step) {
            failure_physics_step = Some(neutral.physics_step);
            failure_detail = Some(format!("{error:#}"));
            break "harness_state_or_clock_failure".to_owned();
        }
        if classification == ContactClassification::None && neutral.fuel_kg <= 0.0 {
            fuel_exhaustion_physics_step.get_or_insert(neutral.physics_step);
        }
        if classification != ContactClassification::None {
            let audit = contact_audit(context, &neutral, &classification);
            core_predicate_mirror_parity = audit.core_matches_predicate_mirror;
            first_contact = Some(audit);
            if !core_predicate_mirror_parity {
                failure_physics_step = Some(neutral.physics_step);
                failure_detail =
                    Some("core contact classification differs from predicate mirror".into());
                break "harness_contact_mirror_failure".to_owned();
            } else {
                break "authoritative_first_contact".to_owned();
            }
        }
        if ordinary.is_terminal()
            && !(neutral.physics_step >= horizon_steps
                && ordinary.end_reason == EndReason::MaxTimeReached)
        {
            failure_physics_step = Some(neutral.physics_step);
            failure_detail =
                Some("ordinary simulator terminated without authoritative neutral contact".into());
            break "harness_unexpected_ordinary_termination".to_owned();
        }
        record_airborne_clearance(
            context,
            &neutral,
            neutral.physics_step,
            &current_phase,
            clearance_policy,
            &mut scan,
        );
    };
    let entry_before = plant_state_evidence(entry_before_offset, context);
    let entry_after = plant_state_evidence(entry_after_offset, context);
    let contact_tick = first_contact
        .as_ref()
        .map(|contact| contact.state.physics_step);
    let shift = contact_tick.map(|tick| {
        i64::try_from(tick).unwrap_or(i64::MAX)
            - i64::try_from(program.expected_contact_physics_step).unwrap_or(i64::MAX)
    });
    let expected_tick_matches =
        contact_tick.map(|tick| tick == program.expected_contact_physics_step);
    let physical_stable_safe_target_contact = first_contact.as_ref().is_some_and(|contact| {
        contact.classification == "stable_touchdown_on_target"
            && contact.core_matches_predicate_mirror
            && contact.body_within_strict_terrain_domain
            && stable_safe_margins_pass(&contact.margins)
    });
    let path_clearance_passed =
        scan.all_airborne_states_passed && terminal_entry_clearance_scan.all_airborne_states_passed;
    let stable_safe_on_target = physical_stable_safe_target_contact && path_clearance_passed;
    let strict_domain_at_contact = first_contact
        .as_ref()
        .map(|contact| contact.body_within_strict_terrain_domain);
    let status = if failure_detail.is_some() {
        "harness_invalid"
    } else if stable_safe_on_target {
        "stable_safe_on_target"
    } else if physical_stable_safe_target_contact {
        "stable_contact_with_clearance_violation"
    } else if first_contact.is_some() {
        "unsafe_contact"
    } else if !path_clearance_passed {
        "coverage_limited_with_clearance_violation"
    } else {
        "coverage_limited"
    };
    Ok(NominalDirectContactPhaseLaneV1 {
        lane: mode.label().into(),
        status: status.into(),
        delta_y_m,
        entry_state_before_offset: entry_before,
        entry_state_after_offset: entry_after,
        entry_held_command_before_offset: entry_before_offset.held_command,
        entry_held_command_after_offset: entry_after_offset.held_command,
        saved_command_identity: nominal_direct_flight_identity(&program.updates)?,
        command_coverage_end_physics_step: command_coverage_end,
        next_missing_update_physics_step: next_missing_update,
        saved_command_updates_used: saved_updates_used,
        continuation_update_count: continuation_updates,
        continuation_command,
        post_entry_physics_steps: post_entry_steps,
        first_contact,
        expected_contact_physics_step: program.expected_contact_physics_step,
        contact_tick_shift_physics_steps: shift,
        expected_contact_tick_matches: expected_tick_matches,
        stable_safe_on_target,
        physical_stable_safe_target_contact,
        path_clearance_passed,
        strict_body_domain_valid_at_contact: strict_domain_at_contact,
        fuel_exhausted_before_contact: fuel_exhaustion_physics_step.is_some(),
        fuel_exhaustion_physics_step,
        terminal_entry_clearance_scan,
        airborne_clearance_scan: scan,
        ordinary_neutral_parity,
        core_predicate_mirror_parity,
        termination_state: plant_state_evidence(&neutral, context),
        ordinary_termination_state: plant_state_evidence(&ordinary, context),
        termination_state_basis:
            "neutral first-contact physics state before ordinary mission contact effects, or last neutral state at a coverage cap".into(),
        stop_reason,
        failure_physics_step,
        failure_detail,
    })
}

fn terminal_entry_clearance_scan(
    context: &RunContext,
    shifted_entry: &SimulationState,
    delta_y_m: f64,
    clearance_policy: ClearancePolicy,
) -> GeometryClearanceScanEvidence {
    let mut scan = empty_clearance_scan();
    if delta_y_m != 0.0 {
        record_airborne_clearance(
            context,
            shifted_entry,
            shifted_entry.physics_step,
            "terminal_bridge",
            clearance_policy,
            &mut scan,
        );
    }
    scan
}

fn validate_live_state(
    context: &RunContext,
    state: &SimulationState,
    prior_step: u64,
) -> Result<()> {
    if state.physics_step != prior_step + 1
        || state.sim_time_s != state.physics_step as f64 / f64::from(context.sim.physics_hz)
        || !state.position_m.x.is_finite()
        || !state.position_m.y.is_finite()
        || !state.velocity_mps.x.is_finite()
        || !state.velocity_mps.y.is_finite()
        || !state.attitude_rad.is_finite()
        || !state.angular_rate_radps.is_finite()
        || !state.fuel_kg.is_finite()
        || state.fuel_kg < 0.0
        || state.fuel_kg > context.vehicle.initial_fuel_kg
    {
        bail!(
            "nonfinite state, fuel, or global physics clock defect at step {}",
            state.physics_step
        );
    }
    Ok(())
}

fn row_from_baseline(
    case: &PreparedContactPhaseCase,
    saved_program_coverage: NominalDirectContactPhaseLaneV1,
) -> Result<NominalDirectContactPhaseRowV1> {
    let mut diagnostic_continuation = saved_program_coverage.clone();
    diagnostic_continuation.lane = LaneMode::DiagnosticContinuation.label().into();
    build_row(case, 0.0, saved_program_coverage, diagnostic_continuation)
}

fn build_row(
    case: &PreparedContactPhaseCase,
    delta_y_m: f64,
    saved_program_coverage: NominalDirectContactPhaseLaneV1,
    diagnostic_continuation: NominalDirectContactPhaseLaneV1,
) -> Result<NominalDirectContactPhaseRowV1> {
    let mut row = NominalDirectContactPhaseRowV1 {
        case_id: case.case_id.clone(),
        delta_y_m,
        saved_program_coverage,
        diagnostic_continuation,
        identity: String::new(),
    };
    row.identity = contact_phase_row_identity(&row)?;
    Ok(row)
}

fn contact_phase_row_identity(row: &NominalDirectContactPhaseRowV1) -> Result<String> {
    let mut canonical = row.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

fn row_failure(row: &NominalDirectContactPhaseRowV1) -> Option<String> {
    [&row.saved_program_coverage, &row.diagnostic_continuation]
        .into_iter()
        .find(|lane| lane.status == "harness_invalid")
        .and_then(|lane| {
            lane.failure_detail.clone().or_else(|| {
                Some(format!(
                    "{} lane reported a harness invariant failure",
                    lane.lane
                ))
            })
        })
}

fn offset_file_name(delta_y_m: f64) -> String {
    let millimeters = (delta_y_m * 1000.0).round() as i32;
    if millimeters == 0 {
        "offset_000mm.json".into()
    } else if millimeters < 0 {
        format!("offset_m{:03}mm.json", millimeters.unsigned_abs())
    } else {
        format!("offset_p{:03}mm.json", millimeters)
    }
}

fn write_case_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::nominal_direct_flight::write_create_only(path, value)
}

fn summarize_lane_counts(
    lane: &str,
    rows: &[NominalDirectContactPhaseRowV1],
    continuation: bool,
) -> NominalDirectContactPhaseCountsV1 {
    let selected = rows.iter().map(|row| {
        if continuation {
            &row.diagnostic_continuation
        } else {
            &row.saved_program_coverage
        }
    });
    let mut result = NominalDirectContactPhaseCountsV1 {
        lane: lane.to_owned(),
        row_count: rows.len(),
        stable_safe_target_contact_count: 0,
        physical_stable_safe_target_contact_count: 0,
        unsafe_contact_count: 0,
        coverage_limited_count: 0,
        clearance_violation_count: 0,
        fuel_exhaustion_before_contact_count: 0,
        exact_expected_tick_count: 0,
        earlier_contact_count: 0,
        later_contact_count: 0,
        contact_count: 0,
        minimum_hull_penetration_margin_m: None,
        maximum_hull_penetration_margin_m: None,
        minimum_dynamic_penetration_allowance_m: None,
        maximum_dynamic_penetration_allowance_m: None,
    };
    for lane_row in selected {
        result.stable_safe_target_contact_count += usize::from(lane_row.stable_safe_on_target);
        result.physical_stable_safe_target_contact_count +=
            usize::from(lane_row.physical_stable_safe_target_contact);
        result.unsafe_contact_count += usize::from(
            lane_row.first_contact.is_some() && !lane_row.physical_stable_safe_target_contact,
        );
        result.coverage_limited_count +=
            usize::from(lane_row.first_contact.is_none() && lane_row.status != "harness_invalid");
        result.clearance_violation_count += usize::from(
            !lane_row.path_clearance_passed
                || lane_row.strict_body_domain_valid_at_contact == Some(false),
        );
        result.fuel_exhaustion_before_contact_count +=
            usize::from(lane_row.fuel_exhausted_before_contact);
        if let Some(shift) = lane_row.contact_tick_shift_physics_steps {
            result.contact_count += 1;
            if shift == 0 {
                result.exact_expected_tick_count += 1;
            } else if shift < 0 {
                result.earlier_contact_count += 1;
            } else {
                result.later_contact_count += 1;
            }
        }
        if let Some(contact) = &lane_row.first_contact {
            update_range(
                &mut result.minimum_hull_penetration_margin_m,
                &mut result.maximum_hull_penetration_margin_m,
                contact.margins.stable_hull_penetration_margin_m,
            );
            update_range(
                &mut result.minimum_dynamic_penetration_allowance_m,
                &mut result.maximum_dynamic_penetration_allowance_m,
                contact.state.dynamic_hull_penetration_allowance_m,
            );
        }
    }
    result
}

fn update_range(minimum: &mut Option<f64>, maximum: &mut Option<f64>, value: f64) {
    if !value.is_finite() {
        return;
    }
    *minimum = Some(minimum.map_or(value, |current| current.min(value)));
    *maximum = Some(maximum.map_or(value, |current| current.max(value)));
}

fn study_decision(
    saved: &NominalDirectContactPhaseCountsV1,
    continuation: &NominalDirectContactPhaseCountsV1,
    rows: &[NominalDirectContactPhaseRowV1],
    study_valid_and_complete: bool,
) -> String {
    if !study_valid_and_complete {
        return "study_incomplete_or_harness_gate_failed".into();
    }
    if saved.unsafe_contact_count > 0 || continuation.unsafe_contact_count > 0 {
        return "failing_first_contact_predicate_next".into();
    }
    if saved.clearance_violation_count > 0 || continuation.clearance_violation_count > 0 {
        return "failing_clearance_predicate_next".into();
    }
    if continuation.coverage_limited_count > 0 {
        return "incomplete_finite_coverage_evidence".into();
    }
    if rows.iter().any(|row| {
        row.saved_program_coverage.status == "coverage_limited"
            || row
                .saved_program_coverage
                .expected_contact_tick_matches
                .is_some_and(|matches| !matches)
            || row
                .diagnostic_continuation
                .expected_contact_tick_matches
                .is_some_and(|matches| !matches)
    }) {
        return "operational_completion_contract_next".into();
    }
    "no_unsafe_contact_within_fixed_matrix".into()
}

fn contact_phase_study_identity(study: &NominalDirectContactPhaseStudyV1) -> Result<String> {
    let mut canonical = study.clone();
    canonical.identity.clear();
    canonical.compute = NominalDirectContactPhaseComputeV1::default();
    stable_digest(&canonical)
}

fn elapsed_us(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use pd_core::{FlightProgramBindingV1, FlightProgramUpdateV1, LandingPadSpec, Vec2};

    use super::*;

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "pd-contact-phase-{}-{label}-{}",
                std::process::id(),
                NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("unique contact phase test directory");
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn context() -> RunContext {
        super::super::tests::test_context()
    }

    fn test_policy() -> ClearancePolicy {
        let pad = FlatPadBounds {
            left_m: -20.0,
            right_m: 20.0,
            surface_y_m: 0.0,
            flat: true,
        };
        ClearancePolicy {
            source_pad: pad,
            target_pad: pad,
            minimum_clearance_m: 5.0,
        }
    }

    fn make_program(
        context: &RunContext,
        update_steps: &[u64],
        command: pd_core::Command,
        expected_contact: u64,
        planned_end: u64,
    ) -> FlightProgramV1 {
        FlightProgramV1 {
            schema_version: 1,
            binding: FlightProgramBindingV1::from_context(context),
            source_pad_id: "synthetic-source".into(),
            target_pad_id: "test-pad".into(),
            generation_policy_identity: "synthetic-generation-policy".into(),
            terminal_policy_identity: "synthetic-terminal-policy".into(),
            witness_identity: "synthetic-witness".into(),
            source_handoff_physics_step: 1,
            terminal_entry_physics_step: 0,
            planned_end_physics_step: planned_end,
            expected_contact_physics_step: expected_contact,
            updates: update_steps
                .iter()
                .map(|physics_step| FlightProgramUpdateV1 {
                    physics_step: *physics_step,
                    phase: "terminal_bridge".into(),
                    command,
                })
                .collect(),
        }
    }

    fn replay(
        context: &RunContext,
        entry: &SimulationState,
        program: &FlightProgramV1,
        delta_y_m: f64,
        mode: LaneMode,
    ) -> NominalDirectContactPhaseLaneV1 {
        let shifted = apply_vertical_offset(entry, delta_y_m).unwrap();
        replay_lane(
            context,
            entry,
            &shifted,
            &empty_clearance_scan(),
            0,
            "terminal_bridge",
            program,
            delta_y_m,
            mode,
            test_policy(),
        )
        .unwrap()
    }

    fn empty_counts(lane: &str) -> NominalDirectContactPhaseCountsV1 {
        NominalDirectContactPhaseCountsV1 {
            lane: lane.into(),
            row_count: 0,
            stable_safe_target_contact_count: 0,
            physical_stable_safe_target_contact_count: 0,
            unsafe_contact_count: 0,
            coverage_limited_count: 0,
            clearance_violation_count: 0,
            fuel_exhaustion_before_contact_count: 0,
            exact_expected_tick_count: 0,
            earlier_contact_count: 0,
            later_contact_count: 0,
            contact_count: 0,
            minimum_hull_penetration_margin_m: None,
            maximum_hull_penetration_margin_m: None,
            minimum_dynamic_penetration_allowance_m: None,
            maximum_dynamic_penetration_allowance_m: None,
        }
    }

    fn empty_study() -> NominalDirectContactPhaseStudyV1 {
        NominalDirectContactPhaseStudyV1 {
            schema_id: STUDY_SCHEMA.into(),
            schema_version: STUDY_SCHEMA_VERSION,
            passed: true,
            verdict: "complete_reproducible_diagnostic".into(),
            identity: String::new(),
            baseline_gate_passed: true,
            baseline_checks: Vec::new(),
            input_root: "synthetic-input".into(),
            input_summary_sha256: "synthetic-summary-hash".into(),
            input_summary_identity: "synthetic-summary-identity".into(),
            offsets_m: OFFSETS_M.to_vec(),
            expected_row_count: 0,
            completed_row_count: 0,
            rows: Vec::new(),
            failure_observation: None,
            saved_program_counts: empty_counts("saved"),
            diagnostic_continuation_counts: empty_counts("continuation"),
            deterministic_decision: "synthetic".into(),
            input_files: Vec::new(),
            source_binding_identity_before: "synthetic-source".into(),
            source_binding_identity_after: Some("synthetic-source".into()),
            source_files: Vec::new(),
            protocol_file: NominalDirectContactPhaseFileHashV1 {
                relative_path: NOMINAL_DIRECT_CONTACT_PHASE_PROTOCOL.into(),
                sha256_before: "synthetic-protocol".into(),
                sha256_after: Some("synthetic-protocol".into()),
                unchanged: Some(true),
            },
            failure_detail: None,
            compute: NominalDirectContactPhaseComputeV1::default(),
        }
    }

    #[test]
    fn offsets_are_fixed_and_ordered() {
        assert_eq!(
            OFFSETS_M,
            [
                0.0, -0.001, 0.001, -0.005, 0.005, -0.010, 0.010, -0.020, 0.020
            ]
        );
    }

    #[test]
    fn entry_offset_preserves_global_clock_fuel_and_held_command() {
        let context = context();
        let mut entry = SimulationState::new(&context).unwrap();
        entry.physics_step = 6;
        entry.sim_time_s = 6.0 / 120.0;
        entry.position_m = Vec2::new(2.0, 18.0);
        entry.velocity_mps = Vec2::new(0.2, -0.4);
        entry.attitude_rad = 0.03;
        entry.angular_rate_radps = -0.02;
        entry.fuel_kg = 123.0;
        entry.set_command(pd_core::Command {
            throttle_frac: 0.31,
            target_attitude_rad: 0.12,
        });
        let shifted = apply_vertical_offset(&entry, 0.005).unwrap();
        assert_eq!(shifted.position_m.x, entry.position_m.x);
        assert_eq!(shifted.position_m.y, entry.position_m.y + 0.005);
        assert_eq!(shifted.velocity_mps, entry.velocity_mps);
        assert_eq!(shifted.physics_step, entry.physics_step);
        assert_eq!(shifted.sim_time_s, entry.sim_time_s);
        assert_eq!(shifted.fuel_kg, entry.fuel_kg);
        assert_eq!(shifted.held_command, entry.held_command);
        assert_eq!(shifted.attitude_rad, entry.attitude_rad);
        assert_eq!(shifted.angular_rate_radps, entry.angular_rate_radps);
        assert_eq!(shifted.end_reason, entry.end_reason);
        assert_eq!(shifted.physical_outcome, entry.physical_outcome);
        assert_eq!(shifted.mission_outcome, entry.mission_outcome);
        assert_eq!(entry.position_m, Vec2::new(2.0, 18.0));
        assert!(apply_vertical_offset(&entry, f64::NAN).is_err());

        let mut off_clock_entry = entry.clone();
        off_clock_entry.physics_step = 7;
        off_clock_entry.sim_time_s = 7.0 / 120.0;
        let shifted = off_clock_entry.clone();
        let program = make_program(&context, &[0], pd_core::Command::idle(), 8, 10);
        assert!(
            replay_lane(
                &context,
                &off_clock_entry,
                &shifted,
                &empty_clearance_scan(),
                0,
                "terminal_bridge",
                &FlightProgramV1 {
                    terminal_entry_physics_step: 7,
                    ..program
                },
                0.0,
                LaneMode::SavedProgram,
                test_policy(),
            )
            .is_err()
        );
    }

    #[test]
    fn shifted_terminal_entry_clearance_is_scanned_without_stepping() {
        let context = context();
        let mut entry = SimulationState::new(&context).unwrap();
        entry.position_m = Vec2::new(0.0, -1.0);
        let shifted_scan = terminal_entry_clearance_scan(&context, &entry, -6.0, test_policy());
        assert_eq!(shifted_scan.poststep_state_count, 0);
        assert_eq!(shifted_scan.airborne_state_count, 1);
        assert_eq!(shifted_scan.exact_clearance_query_count, 1);
        assert_eq!(
            shifted_scan.minimum_airborne.as_ref().unwrap().physics_step,
            0
        );
        assert_eq!(
            shifted_scan.minimum_airborne.as_ref().unwrap().phase,
            "terminal_bridge"
        );
        assert!(!shifted_scan.all_airborne_states_passed);
        assert!(shifted_scan.first_violation.is_some());

        let zero_scan = terminal_entry_clearance_scan(&context, &entry, 0.0, test_policy());
        assert_eq!(zero_scan.exact_clearance_query_count, 0);
        assert!(zero_scan.all_airborne_states_passed);
    }

    #[test]
    fn physical_shifted_contact_is_not_mislabeled_as_crash_or_exact_replay() {
        let context = context();
        let entry = SimulationState::new(&context).unwrap();
        let command = pd_core::Command::idle();
        let program = make_program(&context, &[0, 2], command, 1, 20);
        let baseline_a = replay(&context, &entry, &program, 0.0, LaneMode::SavedProgram);
        let baseline_b = replay(&context, &entry, &program, 0.0, LaneMode::SavedProgram);
        assert_eq!(baseline_a.first_contact, baseline_b.first_contact);
        assert_eq!(
            baseline_a
                .first_contact
                .as_ref()
                .unwrap()
                .state
                .physics_step,
            1
        );
        assert_eq!(
            baseline_a.first_contact.as_ref().unwrap().state.fuel_kg,
            entry.fuel_kg
        );
        let lane = replay(&context, &entry, &program, 0.001, LaneMode::SavedProgram);
        assert_eq!(
            lane.first_contact.as_ref().unwrap().classification,
            "stable_touchdown_on_target"
        );
        assert!(lane.physical_stable_safe_target_contact);
        assert!(lane.stable_safe_on_target);
        assert_eq!(lane.expected_contact_tick_matches, Some(false));
        assert_eq!(lane.contact_tick_shift_physics_steps, Some(1));
        assert_eq!(lane.status, "stable_safe_on_target");
        assert!(lane.ordinary_neutral_parity);
        assert!(
            lane.termination_state_basis
                .contains("neutral first-contact")
        );
        assert_eq!(lane.termination_state.end_reason, "running");
        assert_ne!(lane.ordinary_termination_state.end_reason, "running");
    }

    #[test]
    fn missing_saved_coverage_censors_then_holds_only_the_exact_last_command_to_cap() {
        let context = context();
        let entry = SimulationState::new(&context).unwrap();
        let final_command = pd_core::Command {
            throttle_frac: 0.05,
            target_attitude_rad: 0.1,
        };
        let program = make_program(&context, &[0, 2], final_command, 15, 20);
        let saved = replay(&context, &entry, &program, 0.5, LaneMode::SavedProgram);
        assert_eq!(saved.first_contact, None);
        assert_eq!(saved.stop_reason, "next_saved_update_missing");
        assert_eq!(saved.status, "coverage_limited");
        assert_eq!(saved.next_missing_update_physics_step, Some(4));
        assert_eq!(saved.saved_command_updates_used, 2);
        assert_eq!(saved.command_coverage_end_physics_step, 4);
        assert_eq!(saved.continuation_update_count, 0);

        let continuation = replay(
            &context,
            &entry,
            &program,
            0.5,
            LaneMode::DiagnosticContinuation,
        );
        assert_eq!(continuation.first_contact, None);
        assert_eq!(continuation.stop_reason, "planned_end_cap");
        assert_eq!(continuation.termination_state.physics_step, 20);
        assert_eq!(continuation.next_missing_update_physics_step, Some(4));
        assert_eq!(continuation.continuation_update_count, 8);
        assert_eq!(continuation.continuation_command, Some(final_command));
        assert!(continuation.termination_state.velocity_mps.y > -0.3);

        let mut fuel_empty_entry = entry.clone();
        fuel_empty_entry.fuel_kg = 0.0;
        let fuel_limited = replay(
            &context,
            &fuel_empty_entry,
            &program,
            0.5,
            LaneMode::DiagnosticContinuation,
        );
        assert!(fuel_limited.fuel_exhausted_before_contact);
        assert_eq!(fuel_limited.fuel_exhaustion_physics_step, Some(1));
        assert_eq!(fuel_limited.first_contact, None);
    }

    #[test]
    fn first_contact_crash_remains_a_physical_failure() {
        let context = context();
        let mut entry = SimulationState::new(&context).unwrap();
        let angle = 0.08_f64;
        entry.position_m = Vec2::new(
            0.0,
            context.vehicle.geometry.touchdown_base_offset_m * angle.cos()
                + context.vehicle.geometry.touchdown_half_span_m * angle.sin()
                - 0.001,
        );
        entry.velocity_mps = Vec2::new(0.0, -1.0);
        entry.attitude_rad = angle;
        let program = make_program(
            &context,
            &[0],
            pd_core::Command {
                throttle_frac: 0.0,
                target_attitude_rad: angle,
            },
            1,
            2,
        );
        let lane = replay(&context, &entry, &program, 0.0, LaneMode::SavedProgram);
        assert_eq!(lane.first_contact.as_ref().unwrap().classification, "crash");
        assert!(!lane.physical_stable_safe_target_contact);
        assert!(!lane.stable_safe_on_target);
        assert_eq!(lane.status, "unsafe_contact");
        assert_eq!(lane.stop_reason, "authoritative_first_contact");
    }

    #[test]
    fn output_identity_round_trips_and_excludes_observational_timing() {
        let mut artifact = empty_study();
        let identity = contact_phase_study_identity(&artifact).unwrap();
        artifact.identity = identity.clone();
        artifact.compute.total_wall_time_us = 321;
        artifact.compute.perturbation_wall_time_us = 123;
        assert_eq!(contact_phase_study_identity(&artifact).unwrap(), identity);
        let bytes = serde_json::to_vec(&artifact).unwrap();
        let round_trip: NominalDirectContactPhaseStudyV1 = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(round_trip, artifact);
    }

    #[test]
    fn censored_rows_count_even_when_their_clearance_also_fails() {
        let context = context();
        let entry = SimulationState::new(&context).unwrap();
        let command = pd_core::Command::idle();
        let program = make_program(&context, &[0, 2], command, 15, 20);
        let mut lane = replay(&context, &entry, &program, 0.5, LaneMode::SavedProgram);
        lane.path_clearance_passed = false;
        lane.status = "coverage_limited_with_clearance_violation".into();
        let row = NominalDirectContactPhaseRowV1 {
            case_id: "synthetic".into(),
            delta_y_m: 0.5,
            saved_program_coverage: lane.clone(),
            diagnostic_continuation: lane,
            identity: String::new(),
        };
        let counts = summarize_lane_counts("saved", std::slice::from_ref(&row), false);
        assert_eq!(counts.coverage_limited_count, 1);
        assert_eq!(counts.clearance_violation_count, 1);
        let mut invalid_row = row;
        invalid_row.saved_program_coverage.status = "harness_invalid".into();
        let invalid_counts = summarize_lane_counts("saved", &[invalid_row], false);
        assert_eq!(invalid_counts.coverage_limited_count, 0);
    }

    #[test]
    fn incomplete_or_harness_invalid_study_never_recommends_a_next_physics_axis() {
        let saved = empty_counts("saved");
        let continuation = empty_counts("continuation");
        assert_eq!(
            study_decision(&saved, &continuation, &[], false),
            "study_incomplete_or_harness_gate_failed"
        );
        assert_eq!(
            study_decision(&saved, &continuation, &[], true),
            "no_unsafe_contact_within_fixed_matrix"
        );
    }

    #[test]
    fn path_case_and_file_bindings_reject_traversal_duplicates_and_tampering() {
        assert!(validate_input_relative_path("../summary.json").is_err());
        assert!(validate_bundle_relative_path("cases/../../outside").is_err());
        assert!(validate_case_id("../case").is_err());
        let mut seen_ids = BTreeSet::new();
        let mut seen_paths = BTreeSet::new();
        assert!(
            validate_case_path_binding(
                0,
                "case_a",
                "cases/01_case_a",
                &mut seen_ids,
                &mut seen_paths
            )
            .is_ok()
        );
        assert!(
            validate_case_path_binding(
                1,
                "case_a",
                "cases/02_case_a",
                &mut seen_ids,
                &mut seen_paths
            )
            .is_err()
        );

        let temp = TestDir::new("tamper");
        fs::create_dir(temp.0.join("cases")).unwrap();
        fs::write(temp.0.join("cases/program.json"), b"saved bytes").unwrap();
        let mut pins = Vec::new();
        add_file_pin(&mut pins, "cases/program.json", b"saved bytes").unwrap();
        assert!(add_file_pin(&mut pins, "cases/program.json", b"duplicate").is_err());
        fs::write(temp.0.join("cases/program.json"), b"tampered bytes").unwrap();
        let changed = verify_pinned_input_file(&temp.0, &pins[0]).unwrap();
        assert_eq!(changed.unchanged, Some(false));
        assert_ne!(
            changed.sha256_after.as_deref(),
            Some(pins[0].sha256_before.as_str())
        );
    }

    #[test]
    fn typed_program_parser_and_global_clock_validation_reject_malformed_inputs() {
        assert!(serde_json::from_slice::<FlightProgramV1>(b"{ malformed").is_err());

        let mut context = context();
        context.world.landing_pads.push(LandingPadSpec {
            id: "synthetic-source".into(),
            center_x_m: -60.0,
            surface_y_m: 0.0,
            width_m: 40.0,
        });
        let command = pd_core::Command::idle();
        let mut program = make_program(&context, &[0, 2, 4], command, 5, 6);
        program.source_handoff_physics_step = 2;
        program.terminal_entry_physics_step = 4;
        assert!(program.validate_against_context(&context).is_ok());

        let mut unsupported = program.clone();
        unsupported.schema_version = 99;
        assert!(unsupported.validate_against_context(&context).is_err());
        let mut off_clock = program.clone();
        off_clock.updates[1].physics_step = 3;
        assert!(off_clock.validate_against_context(&context).is_err());
        program.updates.pop();
        assert!(program.validate_against_context(&context).is_err());

        let mut malformed_payload = make_program(&context, &[0, 2, 4], command, 5, 6);
        malformed_payload.terminal_entry_physics_step = 4;
        malformed_payload.source_handoff_physics_step = 2;
        let mut json = serde_json::to_value(malformed_payload).unwrap();
        json["unexpected_field"] = serde_json::json!(true);
        assert!(serde_json::from_value::<FlightProgramV1>(json).is_err());
    }

    #[test]
    fn preflight_rejects_malformed_archive_without_writing_anything() {
        let temp = TestDir::new("preflight");
        fs::write(temp.0.join("summary.json"), b"{ malformed").unwrap();
        assert!(preflight_nominal_direct_contact_phase_study(&temp.0).is_err());
        let entries = fs::read_dir(&temp.0).unwrap().count();
        assert_eq!(entries, 1);
        assert!(temp.0.join("summary.json").is_file());
    }

    #[test]
    fn existing_output_root_is_refused_without_touching_its_contents() {
        let temp = TestDir::new("output");
        assert!(crate::nominal_direct_flight::reserve_output_root(&temp.0).is_err());
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
    }
}
