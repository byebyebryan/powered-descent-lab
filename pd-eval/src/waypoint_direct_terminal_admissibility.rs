//! Frozen-input terminal-reference and command-cadence diagnostics.
//!
//! This runner only reads retained generation artifacts. It never calls the
//! generator or fitter. Every input is pinned and audited before the
//! create-only output directory is reserved or a simulation is constructed.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    TerminalAdmissibilityRowEvidence, WaypointDirectNominalDirectGenerationArtifact,
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    WaypointDirectObstacleDiscriminationArtifactV1,
    WaypointDirectObstacleDiscriminationCaseEvidenceV1,
    WaypointDirectObstacleDiscriminationDirectnessV1,
    WaypointDirectObstacleDiscriminationFreshManifestV1, evaluate_waypoint_direct_terminal_row,
    load_waypoint_direct_obstacle_discrimination_fresh_manifest,
};

pub const WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_SCHEMA_ID: &str =
    "waypoint_direct_terminal_admissibility_v1";
pub const WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_SCHEMA_VERSION: u32 = 1;
pub const WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_PROTOCOL: &str =
    "docs/waypoint_direct_terminal_admissibility_protocol.md";

const FRESH_GATE_RELATIVE_PATH: &str = "summary.json";
const FRESH_GATE_SHA256: &str = "3282a7e533809009a9e5cf377ad8cbbfea598a2b4840df533fbe696015409baa";
const FRESH_GATE_IDENTITY: &str = "fnv1a64:a484e83d323f3e02";
const FRESH_GATE_PRODUCTION_INPUT_IDENTITY: &str = "fnv1a64:06c560a38ef95e02";
const FRESH_MANIFEST_RELATIVE_PATH: &str =
    "fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json";
const FRESH_MANIFEST_SHA256: &str =
    "647df7bc94d02a9b48b773c45159e0ffa15bfca232a13b4e21164e2251a8f5b4";
const DIRECT_STATUS: &str = "nominal_replay_validated_direct";
const UNKNOWN_STATUS: &str = "unknown_no_complete_accepted_witness_within_finite_family";
const SOURCE_CRATES: [&str; 4] = ["pd-core", "pd-plan", "pd-control", "pd-eval"];
const SOURCE_DURATION_OFFSETS_TICKS: [i64; 5] = [-240, -180, -120, -60, 0];
const EXPECTED_LEDGER_ROWS: usize = 160;
const EXPECTED_ANALYTICAL_SKIPS: usize = 84;
const EXPECTED_SCHEDULED_ROWS: usize = 76;
const EXPECTED_ACCEPTED_ROWS: usize = 34;
const EXPECTED_TARGET_CONTACT_REJECTIONS: usize = 42;

#[derive(Clone, Copy, Debug)]
struct ExpectedCase {
    case_id: &'static str,
    span_m: f64,
    profile: &'static str,
    summary_sha256: &'static str,
    scheduled_rows: usize,
    accepted_rows: usize,
    directness: WaypointDirectObstacleDiscriminationDirectnessV1,
}

const EXPECTED_CASES: [ExpectedCase; 8] = [
    ExpectedCase {
        case_id: "fresh_flat_control_span_700",
        span_m: 700.0,
        profile: "flat_control",
        summary_sha256: "3f376ef069e94883a099aa25630f937f42f7b74d31ddc97e15c4f90a12f9f5e0",
        scheduled_rows: 12,
        accepted_rows: 4,
        directness: WaypointDirectObstacleDiscriminationDirectnessV1::Direct,
    },
    ExpectedCase {
        case_id: "fresh_low_obstacle_span_700",
        span_m: 700.0,
        profile: "low_obstacle",
        summary_sha256: "0f5f7cb0219ea35afb9c812d1d3542ca53b208fc690a7383778e83988895f019",
        scheduled_rows: 12,
        accepted_rows: 4,
        directness: WaypointDirectObstacleDiscriminationDirectnessV1::Direct,
    },
    ExpectedCase {
        case_id: "fresh_high_obstacle_span_700",
        span_m: 700.0,
        profile: "high_obstacle",
        summary_sha256: "7bdc6d9997484e2116d093d96273989255f01023c6f264c1c5d28035e0794926",
        scheduled_rows: 4,
        accepted_rows: 0,
        directness: WaypointDirectObstacleDiscriminationDirectnessV1::Unknown,
    },
    ExpectedCase {
        case_id: "fresh_late_broad_span_700",
        span_m: 700.0,
        profile: "late_broad",
        summary_sha256: "7a9fb06fbd74c84f9e4f9bbc26af0f0024f73cf809070f6bf497580aebc6e1a2",
        scheduled_rows: 9,
        accepted_rows: 4,
        directness: WaypointDirectObstacleDiscriminationDirectnessV1::Direct,
    },
    ExpectedCase {
        case_id: "fresh_flat_control_span_900",
        span_m: 900.0,
        profile: "flat_control",
        summary_sha256: "fda664457c05b937c98d8299aaaaef3166e13db4041116e2d42932933a184018",
        scheduled_rows: 14,
        accepted_rows: 9,
        directness: WaypointDirectObstacleDiscriminationDirectnessV1::Direct,
    },
    ExpectedCase {
        case_id: "fresh_low_obstacle_span_900",
        span_m: 900.0,
        profile: "low_obstacle",
        summary_sha256: "a584ec8457f87ed38ec8c0ad0f1603cc23010d4399e697acdcd6d4de6a9d1abd",
        scheduled_rows: 14,
        accepted_rows: 9,
        directness: WaypointDirectObstacleDiscriminationDirectnessV1::Direct,
    },
    ExpectedCase {
        case_id: "fresh_high_obstacle_span_900",
        span_m: 900.0,
        profile: "high_obstacle",
        summary_sha256: "154040ea0633b4c66276c4722f8e35b50cdfc92254811205e6c82116fab1ec7a",
        scheduled_rows: 2,
        accepted_rows: 0,
        directness: WaypointDirectObstacleDiscriminationDirectnessV1::Unknown,
    },
    ExpectedCase {
        case_id: "fresh_late_broad_span_900",
        span_m: 900.0,
        profile: "late_broad",
        summary_sha256: "b2592ea3d58dc6e80cb05a524d6d399a8dc7a8ec0912b72c7ccec0bc1acab62d",
        scheduled_rows: 9,
        accepted_rows: 4,
        directness: WaypointDirectObstacleDiscriminationDirectnessV1::Direct,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalCadenceSelectionClassV1 {
    HighObstacleFailure,
    NearMarginFailure,
    AcceptedControl,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCadenceSelectorV1 {
    pub case_id: String,
    pub row_index: usize,
    pub selection_class: TerminalCadenceSelectionClassV1,
}

const EXPECTED_SELECTORS: [(&str, usize, TerminalCadenceSelectionClassV1); 19] = [
    (
        "fresh_high_obstacle_span_700",
        16,
        TerminalCadenceSelectionClassV1::HighObstacleFailure,
    ),
    (
        "fresh_high_obstacle_span_700",
        17,
        TerminalCadenceSelectionClassV1::HighObstacleFailure,
    ),
    (
        "fresh_high_obstacle_span_700",
        18,
        TerminalCadenceSelectionClassV1::HighObstacleFailure,
    ),
    (
        "fresh_high_obstacle_span_700",
        19,
        TerminalCadenceSelectionClassV1::HighObstacleFailure,
    ),
    (
        "fresh_high_obstacle_span_900",
        16,
        TerminalCadenceSelectionClassV1::HighObstacleFailure,
    ),
    (
        "fresh_high_obstacle_span_900",
        17,
        TerminalCadenceSelectionClassV1::HighObstacleFailure,
    ),
    (
        "fresh_flat_control_span_700",
        10,
        TerminalCadenceSelectionClassV1::NearMarginFailure,
    ),
    (
        "fresh_flat_control_span_700",
        11,
        TerminalCadenceSelectionClassV1::NearMarginFailure,
    ),
    (
        "fresh_flat_control_span_700",
        12,
        TerminalCadenceSelectionClassV1::NearMarginFailure,
    ),
    (
        "fresh_flat_control_span_700",
        13,
        TerminalCadenceSelectionClassV1::NearMarginFailure,
    ),
    (
        "fresh_flat_control_span_700",
        14,
        TerminalCadenceSelectionClassV1::NearMarginFailure,
    ),
    (
        "fresh_low_obstacle_span_700",
        10,
        TerminalCadenceSelectionClassV1::NearMarginFailure,
    ),
    (
        "fresh_low_obstacle_span_700",
        11,
        TerminalCadenceSelectionClassV1::NearMarginFailure,
    ),
    (
        "fresh_low_obstacle_span_700",
        12,
        TerminalCadenceSelectionClassV1::NearMarginFailure,
    ),
    (
        "fresh_low_obstacle_span_700",
        13,
        TerminalCadenceSelectionClassV1::NearMarginFailure,
    ),
    (
        "fresh_low_obstacle_span_700",
        14,
        TerminalCadenceSelectionClassV1::NearMarginFailure,
    ),
    (
        "fresh_flat_control_span_700",
        16,
        TerminalCadenceSelectionClassV1::AcceptedControl,
    ),
    (
        "fresh_flat_control_span_900",
        10,
        TerminalCadenceSelectionClassV1::AcceptedControl,
    ),
    (
        "fresh_late_broad_span_900",
        16,
        TerminalCadenceSelectionClassV1::AcceptedControl,
    ),
];

const SCOPE_NON_CLAIMS: [&str; 5] = [
    "Reference admissibility is an analytical pose path checked with synthetic neutral core contact probes, not a physical flight or complete witness.",
    "Terminal-only cadence outcomes do not create a new accepted witness or change generator, controller, core, planner, or default behavior.",
    "An inadmissible retained reference does not prove another direct arc is impossible and does not establish waypoint demand.",
    "Pointwise body clearance and discrete first-contact predicates are not continuous swept-path or robustness certificates.",
    "The frozen obstacle-gate verdict is retained as diagnostic provenance and does not classify this pass's empirical outcomes.",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalSourceFileBindingV1 {
    pub relative_path: String,
    pub raw_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalSourceBindingV1 {
    /// Semantic FNV identity over sorted `(repository-relative path, bytes)` entries.
    pub identity: String,
    pub protocol_sha256: String,
    pub files: Vec<TerminalSourceFileBindingV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalPreflightCaseV1 {
    pub case_id: String,
    pub profile: String,
    pub horizontal_span_m: f64,
    pub generation_identity: String,
    pub generation_summary_sha256: String,
    pub row_count: usize,
    pub analytical_skip_count: usize,
    pub scheduled_count: usize,
    pub accepted_count: usize,
    pub target_contact_rejection_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalInputLedgerTotalsV1 {
    pub row_count: usize,
    pub analytical_skip_count: usize,
    pub scheduled_count: usize,
    pub fully_wrapped_count: usize,
    pub accepted_count: usize,
    pub target_contact_rejection_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalAdmissibilityPreflightV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub input_gate_raw_sha256: String,
    pub input_gate_semantic_identity: String,
    /// Copied for provenance only. It is not used as the new pass verdict.
    pub input_gate_verdict_diagnostic: String,
    pub input_gate_production_inputs_identity: String,
    pub fresh_manifest_sha256: String,
    pub source_binding: TerminalSourceBindingV1,
    pub cases: Vec<TerminalPreflightCaseV1>,
    pub totals: TerminalInputLedgerTotalsV1,
    pub cadence_selectors: Vec<TerminalCadenceSelectorV1>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalScheduledRowResultV1 {
    pub row_index: usize,
    pub basis_index: usize,
    pub duration_offset_ticks: i64,
    pub original_accepted: bool,
    pub cadence_selection_class: Option<TerminalCadenceSelectionClassV1>,
    pub helper_succeeded: bool,
    /// Stable error class only; transient error strings and paths are excluded.
    pub failure_kind: Option<String>,
    pub evidence: Option<TerminalAdmissibilityRowEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalAdmissibilityCaseArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub input_gate_semantic_identity: String,
    pub input_manifest_sha256: String,
    pub generation_identity: String,
    pub generation_summary_sha256: String,
    pub case_id: String,
    pub profile: String,
    pub horizontal_span_m: f64,
    pub skipped_row_indices: Vec<usize>,
    pub scheduled_rows: Vec<TerminalScheduledRowResultV1>,
    pub scheduled_row_count: usize,
    pub baseline_pass_count: usize,
    pub reference_binding_pass_count: usize,
    pub reference_pose_core_match_pass_count: usize,
    pub core_predicate_mirror_pass_count: usize,
    pub cadence_comparison_count: usize,
    pub helper_error_count: usize,
    pub mandatory_guards_passed: bool,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalCaseArtifactReferenceV1 {
    pub case_id: String,
    pub relative_path: String,
    pub raw_sha256: String,
    pub semantic_identity: String,
    pub scheduled_row_count: usize,
    pub mandatory_guards_passed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TerminalReferenceObservationCountsV1 {
    pub original_witness_accepted: bool,
    pub rows: usize,
    pub admissible_reference_pose_paths: usize,
    pub inadmissible_reference_pose_paths: usize,
    pub rows_with_first_reference_contact: usize,
    pub reference_foot_predicate_passes: usize,
    pub reference_foot_predicate_failures: usize,
    pub reference_normal_speed_predicate_passes: usize,
    pub reference_normal_speed_predicate_failures: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TerminalCadenceOutcomeCountsV1 {
    pub rows: usize,
    pub held_60hz_stable_target_contacts: usize,
    pub terminal_only_120hz_stable_target_contacts: usize,
    pub both_lanes_stable_target_contacts: usize,
    pub held_60hz_only_stable_target_contacts: usize,
    pub terminal_only_120hz_only_stable_target_contacts: usize,
    pub neither_lane_stable_target_contact: usize,
    pub matched_entry_and_global_phase_rows: usize,
    pub held_60hz_ordinary_neutral_parity_rows: usize,
    pub terminal_only_120hz_ordinary_neutral_parity_rows: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalCadenceClassCountsV1 {
    pub selection_class: TerminalCadenceSelectionClassV1,
    pub outcomes: TerminalCadenceOutcomeCountsV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalSourceIntegrityCheckV1 {
    pub case_id: String,
    pub identity_before_case: String,
    pub identity_after_case: String,
    pub unchanged_from_preflight: bool,
    pub unchanged_during_case: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalAdmissibilityArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub phase: String,
    pub preflight: TerminalAdmissibilityPreflightV1,
    pub source_integrity_checks: Vec<TerminalSourceIntegrityCheckV1>,
    pub source_identity_after_all_cases: String,
    pub source_unchanged_throughout_evaluation: bool,
    pub case_artifacts: Vec<TerminalCaseArtifactReferenceV1>,
    pub reference_observations_by_original_acceptance: Vec<TerminalReferenceObservationCountsV1>,
    pub cadence_outcomes_by_selection_class: Vec<TerminalCadenceClassCountsV1>,
    pub audited_scheduled_rows: usize,
    pub baseline_pass_count: usize,
    pub reference_binding_pass_count: usize,
    pub reference_pose_core_match_pass_count: usize,
    pub core_predicate_mirror_pass_count: usize,
    pub cadence_comparison_count: usize,
    pub mandatory_row_guard_failures: usize,
    pub helper_error_count: usize,
    pub passed: bool,
    pub verdict: String,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone)]
struct LoadedCase {
    expected: ExpectedCase,
    generation: WaypointDirectNominalDirectGenerationArtifact,
    generation_summary_sha256: String,
}

struct LoadedInputs {
    preflight: TerminalAdmissibilityPreflightV1,
    cases: Vec<LoadedCase>,
}

/// Verify every sealed input, generation artifact and selector without
/// constructing a plant or evaluating a candidate.
pub fn preflight_waypoint_direct_terminal_admissibility(
    input_root: &Path,
) -> Result<TerminalAdmissibilityPreflightV1> {
    Ok(load_inputs(input_root)?.preflight)
}

/// Replay the 76 retained schedules and the explicitly selected 19 cadence
/// rows. `output_dir` is create-only and is reserved only after preflight.
pub fn run_waypoint_direct_terminal_admissibility(
    input_root: &Path,
    output_dir: &Path,
) -> Result<TerminalAdmissibilityArtifactV1> {
    let repo_root = repo_root();
    ensure_output_target_available(&repo_root, output_dir)?;
    let loaded = load_inputs(input_root)?;
    let output_root = reserve_output_root(&repo_root, output_dir)?;
    let expected_source_identity = loaded.preflight.source_binding.identity.clone();
    let mut case_references = Vec::with_capacity(loaded.cases.len());
    let mut source_checks = Vec::with_capacity(loaded.cases.len());
    let mut row_results = Vec::with_capacity(EXPECTED_SCHEDULED_ROWS);

    let cases_root = output_root.join("cases");
    fs::create_dir(&cases_root).context("creating terminal-admissibility cases directory")?;
    for loaded_case in &loaded.cases {
        let source_before_case = source_identity_only(&repo_root)?;
        let case_artifact = evaluate_if_source_stable(
            &expected_source_identity,
            &source_before_case,
            loaded_case.expected.case_id,
            || evaluate_case(loaded_case, &loaded.preflight),
        )?;
        let case_path = PathBuf::from("cases")
            .join(loaded_case.expected.case_id)
            .join("summary.json");
        let absolute_case_dir = output_root.join("cases").join(loaded_case.expected.case_id);
        fs::create_dir(&absolute_case_dir).with_context(|| {
            format!("creating case output for {}", loaded_case.expected.case_id)
        })?;
        let case_bytes = json_bytes(&case_artifact)?;
        write_create_only(&output_root.join(&case_path), &case_bytes)?;
        let case_raw_sha256 = sha256_bytes(&case_bytes)?;
        row_results.extend(case_artifact.scheduled_rows.iter().cloned());
        case_references.push(TerminalCaseArtifactReferenceV1 {
            case_id: loaded_case.expected.case_id.to_owned(),
            relative_path: case_path.to_string_lossy().into_owned(),
            raw_sha256: case_raw_sha256,
            semantic_identity: case_artifact.identity.clone(),
            scheduled_row_count: case_artifact.scheduled_row_count,
            mandatory_guards_passed: case_artifact.mandatory_guards_passed,
        });
        let source_after_case = source_identity_only(&repo_root)?;
        require_source_identity(
            &expected_source_identity,
            &source_after_case,
            loaded_case.expected.case_id,
            "after case",
        )?;
        if source_before_case != source_after_case {
            bail!(
                "terminal diagnostic source changed during case {}",
                loaded_case.expected.case_id
            );
        }
        source_checks.push(TerminalSourceIntegrityCheckV1 {
            case_id: loaded_case.expected.case_id.to_owned(),
            identity_before_case: source_before_case.clone(),
            identity_after_case: source_after_case.clone(),
            unchanged_from_preflight: source_before_case == expected_source_identity
                && source_after_case == expected_source_identity,
            unchanged_during_case: source_before_case == source_after_case,
        });
    }
    let source_identity_after_all_cases = source_identity_only(&repo_root)?;
    require_source_identity(
        &expected_source_identity,
        &source_identity_after_all_cases,
        "all cases",
        "after all cases",
    )?;
    let source_unchanged_throughout_evaluation = source_identity_after_all_cases
        == expected_source_identity
        && source_checks
            .iter()
            .all(|check| check.unchanged_from_preflight && check.unchanged_during_case);
    let references = reference_observation_counts(&row_results);
    let cadence = cadence_outcome_counts(&row_results);
    let mandatory_row_guard_failures = row_results
        .iter()
        .filter(|row| !row_mandatory_guards_passed(row))
        .count();
    let helper_error_count = row_results
        .iter()
        .filter(|row| !row.helper_succeeded)
        .count();
    let baseline_pass_count = row_results
        .iter()
        .filter(|row| {
            row.evidence
                .as_ref()
                .is_some_and(|e| e.frozen_baseline.passed)
        })
        .count();
    let reference_binding_pass_count = row_results
        .iter()
        .filter(|row| {
            row.evidence
                .as_ref()
                .is_some_and(|e| e.reference.reference_binding_passed)
        })
        .count();
    let reference_pose_core_match_pass_count = row_results
        .iter()
        .filter(|row| {
            row.evidence
                .as_ref()
                .is_some_and(|e| e.reference.core_probes_match_reference_poses)
        })
        .count();
    let core_predicate_mirror_pass_count = row_results
        .iter()
        .filter(|row| {
            row.evidence
                .as_ref()
                .is_some_and(|e| e.reference.core_predicate_mirror_parity)
        })
        .count();
    let cadence_comparison_count = row_results
        .iter()
        .filter(|row| {
            row.evidence
                .as_ref()
                .is_some_and(|e| e.cadence_comparison.is_some())
        })
        .count();
    let passed = source_unchanged_throughout_evaluation
        && row_results.len() == EXPECTED_SCHEDULED_ROWS
        && baseline_pass_count == EXPECTED_SCHEDULED_ROWS
        && reference_binding_pass_count == EXPECTED_SCHEDULED_ROWS
        && reference_pose_core_match_pass_count == EXPECTED_SCHEDULED_ROWS
        && core_predicate_mirror_pass_count == EXPECTED_SCHEDULED_ROWS
        && cadence_comparison_count == EXPECTED_SELECTORS.len()
        && mandatory_row_guard_failures == 0
        && helper_error_count == 0;
    let mut artifact = TerminalAdmissibilityArtifactV1 {
        schema_id: WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_SCHEMA_VERSION,
        phase: "terminal_admissibility_diagnostic".to_owned(),
        preflight: loaded.preflight,
        source_integrity_checks: source_checks,
        source_identity_after_all_cases,
        source_unchanged_throughout_evaluation,
        case_artifacts: case_references,
        reference_observations_by_original_acceptance: references,
        cadence_outcomes_by_selection_class: cadence,
        audited_scheduled_rows: row_results.len(),
        baseline_pass_count,
        reference_binding_pass_count,
        reference_pose_core_match_pass_count,
        core_predicate_mirror_pass_count,
        cadence_comparison_count,
        mandatory_row_guard_failures,
        helper_error_count,
        passed,
        verdict: if passed {
            "frozen_terminal_reference_and_execution_diagnostics_passed_opt_in_only"
        } else {
            "terminal_diagnostic_integrity_gate_failed_publish_observations_without_policy_change"
        }
        .to_owned(),
        scope_non_claims: SCOPE_NON_CLAIMS.iter().map(|s| (*s).to_owned()).collect(),
        identity: String::new(),
    };
    artifact.identity = terminal_artifact_identity(&artifact)?;
    let bytes = json_bytes(&artifact)?;
    write_create_only(&output_root.join("summary.json"), &bytes)?;
    Ok(artifact)
}

fn load_inputs(input_root: &Path) -> Result<LoadedInputs> {
    let repo_root = repo_root();
    let input_root = resolve_from_repo(&repo_root, input_root);
    let source_binding = source_binding(&repo_root)?;

    let manifest_path = repo_root.join(FRESH_MANIFEST_RELATIVE_PATH);
    let manifest_raw = fs::read(&manifest_path)
        .with_context(|| format!("reading frozen manifest {}", manifest_path.display()))?;
    require_raw_sha256(&manifest_raw, FRESH_MANIFEST_SHA256, "fresh manifest")?;
    let manifest_from_bytes: WaypointDirectObstacleDiscriminationFreshManifestV1 =
        serde_json::from_slice(&manifest_raw).context("decoding frozen fresh manifest")?;
    let manifest = load_waypoint_direct_obstacle_discrimination_fresh_manifest(&repo_root)?;
    if manifest != manifest_from_bytes {
        bail!("fresh manifest changed while terminal preflight was reading it");
    }
    validate_manifest_case_order(&manifest)?;

    let gate_path = input_root.join(FRESH_GATE_RELATIVE_PATH);
    let gate_raw = fs::read(&gate_path)
        .with_context(|| format!("reading frozen gate {}", gate_path.display()))?;
    let gate_raw_sha256 = sha256_bytes(&gate_raw)?;
    if gate_raw_sha256 != FRESH_GATE_SHA256 {
        bail!("frozen obstacle gate raw SHA-256 mismatch");
    }
    let gate: WaypointDirectObstacleDiscriminationArtifactV1 =
        serde_json::from_slice(&gate_raw).context("decoding frozen obstacle gate")?;
    validate_gate_artifact(&gate)?;

    let mut cases = Vec::with_capacity(EXPECTED_CASES.len());
    let mut preflight_cases = Vec::with_capacity(EXPECTED_CASES.len());
    let mut totals = TerminalInputLedgerTotalsV1 {
        row_count: 0,
        analytical_skip_count: 0,
        scheduled_count: 0,
        fully_wrapped_count: 0,
        accepted_count: 0,
        target_contact_rejection_count: 0,
    };
    for (index, expected) in EXPECTED_CASES.iter().copied().enumerate() {
        let manifest_case = &manifest.cases[index];
        let gate_case = &gate.cases[index];
        validate_manifest_case(expected, manifest_case)?;
        validate_gate_case(expected, gate_case)?;
        let generation_path = input_root
            .join("cases")
            .join(expected.case_id)
            .join("generation")
            .join("summary.json");
        let generation_raw = fs::read(&generation_path)
            .with_context(|| format!("reading frozen generation {}", generation_path.display()))?;
        let generation_sha256 = sha256_bytes(&generation_raw)?;
        if generation_sha256 != expected.summary_sha256
            || gate_case.generator.generated_summary_sha256.as_deref()
                != Some(generation_sha256.as_str())
        {
            bail!(
                "generation summary raw digest does not bind frozen case {}",
                expected.case_id
            );
        }
        let generation: WaypointDirectNominalDirectGenerationArtifact =
            serde_json::from_slice(&generation_raw).with_context(|| {
                format!("decoding generation artifact for {}", expected.case_id)
            })?;
        validate_generation_artifact(
            expected,
            manifest_case,
            gate_case,
            &generation,
            &generation_sha256,
        )?;
        let case_counts = generation_counts(&generation);
        totals.row_count += case_counts.row_count;
        totals.analytical_skip_count += case_counts.analytical_skip_count;
        totals.scheduled_count += case_counts.scheduled_count;
        totals.fully_wrapped_count += case_counts.fully_wrapped_count;
        totals.accepted_count += case_counts.accepted_count;
        totals.target_contact_rejection_count += case_counts.target_contact_rejection_count;
        preflight_cases.push(TerminalPreflightCaseV1 {
            case_id: expected.case_id.to_owned(),
            profile: expected.profile.to_owned(),
            horizontal_span_m: expected.span_m,
            generation_identity: generation.identity.clone(),
            generation_summary_sha256: generation_sha256.clone(),
            row_count: case_counts.row_count,
            analytical_skip_count: case_counts.analytical_skip_count,
            scheduled_count: case_counts.scheduled_count,
            accepted_count: case_counts.accepted_count,
            target_contact_rejection_count: case_counts.target_contact_rejection_count,
        });
        cases.push(LoadedCase {
            expected,
            generation,
            generation_summary_sha256: generation_sha256,
        });
    }
    validate_ledger_totals(&totals)?;
    let selectors = expected_selectors();
    validate_selector_coverage(&selectors)?;
    validate_selected_rows(&cases, &selectors)?;

    let mut preflight = TerminalAdmissibilityPreflightV1 {
        schema_id: WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_SCHEMA_VERSION,
        input_gate_raw_sha256: gate_raw_sha256,
        input_gate_semantic_identity: gate.identity.clone(),
        input_gate_verdict_diagnostic: gate.verdict.clone(),
        input_gate_production_inputs_identity: gate.production_inputs_identity.clone(),
        fresh_manifest_sha256: FRESH_MANIFEST_SHA256.to_owned(),
        source_binding,
        cases: preflight_cases,
        totals,
        cadence_selectors: selectors,
        identity: String::new(),
    };
    preflight.identity = preflight_identity(&preflight)?;
    Ok(LoadedInputs { preflight, cases })
}

fn validate_manifest_case_order(
    manifest: &WaypointDirectObstacleDiscriminationFreshManifestV1,
) -> Result<()> {
    let ids = manifest
        .cases
        .iter()
        .map(|case| case.case_id.as_str())
        .collect::<Vec<_>>();
    let expected_ids = EXPECTED_CASES
        .iter()
        .map(|case| case.case_id)
        .collect::<Vec<_>>();
    if ids != expected_ids
        || manifest.schema_id != "waypoint_direct_obstacle_discrimination_fresh_inputs_v1"
        || manifest.schema_version != 1
        || !manifest.sealed_before_implementation
        || manifest.generation_policy != WaypointDirectNominalDirectGenerationPolicyV1::default()
        || manifest.cases.len() != EXPECTED_CASES.len()
    {
        bail!("sealed manifest case order or policy changed");
    }
    Ok(())
}

fn validate_manifest_case(
    expected: ExpectedCase,
    manifest_case: &crate::WaypointDirectObstacleDiscriminationFreshCaseV1,
) -> Result<()> {
    if manifest_case.case_id != expected.case_id
        || manifest_case.profile != expected.profile
        || manifest_case.horizontal_span_m != expected.span_m
        || manifest_case.scenario.mission.transfer_route.is_some()
    {
        bail!("manifest case binding changed for {}", expected.case_id);
    }
    Ok(())
}

fn validate_gate_artifact(gate: &WaypointDirectObstacleDiscriminationArtifactV1) -> Result<()> {
    validate_case_order(gate)?;
    if gate.schema_id != "waypoint_direct_obstacle_discrimination_v1"
        || gate.schema_version != 1
        || gate.phase != "fresh"
        || gate.fresh_manifest_sha256 != FRESH_MANIFEST_SHA256
        || gate.production_inputs_identity != FRESH_GATE_PRODUCTION_INPUT_IDENTITY
        || !gate.passed
    {
        bail!("frozen obstacle gate semantic identity or binding mismatch");
    }
    require_semantic_identity(
        &gate.identity,
        &obstacle_gate_identity(gate)?,
        FRESH_GATE_IDENTITY,
        "frozen obstacle gate",
    )?;
    if gate.cases.len() != EXPECTED_CASES.len() {
        bail!("frozen obstacle gate does not contain eight cases");
    }
    for (index, expected) in EXPECTED_CASES.iter().copied().enumerate() {
        validate_gate_case(expected, &gate.cases[index])?;
    }
    Ok(())
}

fn validate_case_order(gate: &WaypointDirectObstacleDiscriminationArtifactV1) -> Result<()> {
    let ids = gate
        .cases
        .iter()
        .map(|case| case.case_id.as_str())
        .collect::<Vec<_>>();
    validate_case_id_order(&ids)
}

fn validate_case_id_order(ids: &[&str]) -> Result<()> {
    let expected_ids = EXPECTED_CASES
        .iter()
        .map(|case| case.case_id)
        .collect::<Vec<_>>();
    if ids != expected_ids.as_slice() {
        bail!("frozen obstacle gate case order changed");
    }
    Ok(())
}

fn validate_gate_case(
    expected: ExpectedCase,
    gate_case: &WaypointDirectObstacleDiscriminationCaseEvidenceV1,
) -> Result<()> {
    if gate_case.case_id != expected.case_id
        || gate_case.profile != expected.profile
        || gate_case.horizontal_span_m != expected.span_m
        || gate_case.generator.directness != expected.directness
        || gate_case.generator.generation_identity.is_none()
        || gate_case.generator.generated_summary_sha256.as_deref() != Some(expected.summary_sha256)
        || !gate_case.generator.ledger.valid
        || !gate_case.generator.accepted_complete_existing_gates
        || gate_case.generator.failure_kind.is_some()
    {
        bail!(
            "frozen gate ledger binding changed for {}",
            expected.case_id
        );
    }
    validate_gate_ledger_counts(
        expected,
        gate_case.generator.row_count,
        gate_case.generator.scheduled_count,
        gate_case.generator.accepted_count,
    )?;
    if gate_case.generator.ledger.row_count != 20
        || gate_case.generator.ledger.expected_row_count != 20
    {
        bail!("frozen gate row count changed for {}", expected.case_id);
    }
    Ok(())
}

fn validate_gate_ledger_counts(
    expected: ExpectedCase,
    row_count: usize,
    scheduled_count: usize,
    accepted_count: usize,
) -> Result<()> {
    if row_count != 20
        || scheduled_count != expected.scheduled_rows
        || accepted_count != expected.accepted_rows
    {
        bail!("frozen gate ledger counts changed for {}", expected.case_id);
    }
    Ok(())
}

fn validate_generation_artifact(
    expected: ExpectedCase,
    manifest_case: &crate::WaypointDirectObstacleDiscriminationFreshCaseV1,
    gate_case: &WaypointDirectObstacleDiscriminationCaseEvidenceV1,
    generation: &WaypointDirectNominalDirectGenerationArtifact,
    raw_sha256: &str,
) -> Result<()> {
    let expected_request = WaypointDirectNominalDirectGenerationRequest {
        scenario: manifest_case.scenario.clone(),
        source_pad_id: manifest_case.source_pad_id.clone(),
        target_pad_id: manifest_case.target_pad_id.clone(),
        probe_id: manifest_case.probe_id.clone(),
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    };
    if raw_sha256 != expected.summary_sha256
        || generation.schema_id != "waypoint_direct_nominal_direct_generation_v1"
        || generation.schema_version != 1
        || generation.characterization_id != "waypoint-direct-nominal-direct-generation"
        || generation.identity
            != gate_case
                .generator
                .generation_identity
                .clone()
                .unwrap_or_default()
        || generation_identity(generation)? != generation.identity
        || generation.request != expected_request
        || generation.policy != expected_request.policy
        || generation.scenario_identity != semantic_digest(&expected_request.scenario)?
        || generation.policy_identity != semantic_digest(&expected_request.policy)?
        || generation.source_pad_id != expected_request.source_pad_id
        || generation.target_pad_id != expected_request.target_pad_id
        || generation.probe_id != expected_request.probe_id
        || generation.bases.len() != 4
        || generation.rows.len() != 20
    {
        bail!(
            "frozen generation identity/request binding changed for {}",
            expected.case_id
        );
    }
    crate::validate_waypoint_direct_nominal_direct_generation_request(&generation.request)
        .context("validating frozen generation request")?;
    let mut paired_count = 0;
    let mut wrapper_count = 0;
    let mut accepted_count = 0;
    let mut analytical_skip_count = 0;
    for (index, row) in generation.rows.iter().enumerate() {
        let basis_index = index / SOURCE_DURATION_OFFSETS_TICKS.len();
        let offset_index = index % SOURCE_DURATION_OFFSETS_TICKS.len();
        let basis = generation.bases.get(basis_index);
        if row.row_index != index
            || row.basis_index != basis_index
            || row.duration_offset_ticks != SOURCE_DURATION_OFFSETS_TICKS[offset_index]
            || basis.is_none_or(|basis| {
                basis.basis_index != basis_index
                    || row.basis_candidate_identity != basis.candidate_identity
                    || row.generated_basis_identity != basis.generated_basis_identity
            })
        {
            bail!(
                "frozen row ordering or basis binding changed for {} row {index}",
                expected.case_id
            );
        }
        let has_schedule = row.paired_schedule.is_some();
        let has_wrapper = row.wrapper.is_some();
        if has_schedule != has_wrapper
            || row.accepted != row.wrapper.as_ref().is_some_and(|w| w.accepted)
        {
            bail!(
                "frozen schedule/wrapper binding changed for {} row {index}",
                expected.case_id
            );
        }
        paired_count += usize::from(has_schedule);
        wrapper_count += usize::from(has_wrapper);
        accepted_count += usize::from(row.accepted);
        analytical_skip_count += usize::from(!row.analytical_survivor);
        if !row.analytical_survivor && (has_schedule || has_wrapper || row.accepted) {
            bail!(
                "analytical skip unexpectedly contains a flown schedule in {} row {index}",
                expected.case_id
            );
        }
        if let Some(wrapper) = &row.wrapper {
            if !wrapper_provenance_matches(
                wrapper.provenance.source_row_index,
                &wrapper.provenance.scenario_identity,
                &wrapper.provenance.policy_identity,
                index,
                &generation.scenario_identity,
                &generation.policy,
            )? {
                bail!(
                    "frozen wrapper provenance is not row-bound in {} row {index}",
                    expected.case_id
                );
            }
            if row.accepted {
                if !complete_acceptance_gates_pass(&wrapper.acceptance)
                    || wrapper.first_failing_gate.is_some()
                {
                    bail!(
                        "accepted frozen wrapper lacks complete gates in {} row {index}",
                        expected.case_id
                    );
                }
            } else if !target_contact_only_rejection(&wrapper.acceptance)
                || wrapper.first_failing_gate.as_deref()
                    != Some("first_contact_stable_safe_on_target")
                || wrapper.first_contact.is_none()
            {
                bail!(
                    "frozen rejected wrapper is not an isolated target-contact rejection in {} row {index}",
                    expected.case_id
                );
            }
        }
    }
    let proof = &generation.family_proof;
    if paired_count != expected.scheduled_rows
        || wrapper_count != expected.scheduled_rows
        || accepted_count != expected.accepted_rows
        || analytical_skip_count != 20 - expected.scheduled_rows
        || proof.basis_count != 4
        || proof.expected_basis_count != 4
        || proof.row_count != 20
        || proof.expected_row_count != 20
        || proof.omitted_row_count != 0
        || !proof.all_predeclared_rows_recorded
        || proof.analytical_skip_count != analytical_skip_count
        || proof.scheduled_count != paired_count
        || proof.completed_acceptance_count != wrapper_count
        || proof.accepted_witness_count != accepted_count
        || proof.stopping_result != generation.execution_status
        || proof.completion_gate_passed != (accepted_count > 0)
        || generation.execution_status
            != if accepted_count > 0 {
                DIRECT_STATUS
            } else {
                UNKNOWN_STATUS
            }
    {
        bail!(
            "frozen generation family counts changed for {}",
            expected.case_id
        );
    }
    validate_selection(generation, accepted_count, expected.case_id)?;
    Ok(())
}

fn validate_selection(
    generation: &WaypointDirectNominalDirectGenerationArtifact,
    accepted_count: usize,
    case_id: &str,
) -> Result<()> {
    match (&generation.selection, &generation.selected_physical_witness) {
        (Some(selection), Some(witness)) if accepted_count > 0 => {
            let row = generation.rows.get(selection.row_index);
            if row.is_none_or(|row| {
                !row.accepted
                    || row.wrapper.as_ref().is_none_or(|wrapper| {
                        !wrapper.accepted
                            || wrapper.wrapper_identity != selection.wrapper_identity
                            || wrapper.planned_total_mission_time_s
                                != selection.planned_total_mission_time_s
                    })
                    || row.duration_offset_ticks != witness.duration_offset_ticks
                    || row.basis_candidate_identity != witness.basis_candidate_identity
                    || row.generated_basis_identity != witness.generated_basis_identity
                    || witness.row_index != selection.row_index
                    || witness.wrapper_identity != selection.wrapper_identity
            }) {
                bail!("frozen selection is not bound to an accepted row in {case_id}");
            }
        }
        (None, None) if accepted_count == 0 => {}
        _ => bail!("frozen selection shape disagrees with accepted count in {case_id}"),
    }
    Ok(())
}

fn complete_acceptance_gates_pass(gates: &crate::CompleteFlatAcceptanceGatesEvidence) -> bool {
    gates.supported_source_pad_rest_state
        && gates.launch_completed_and_contact_free
        && gates.launch_to_source_join_passed
        && gates.existing_source_screens_passed
        && gates.strict_source_handoff_passed
        && gates.scheduled_source_prefix_parity_passed
        && gates.ordinary_neutral_and_stored_replay_parity_passed
        && gates.pointwise_core_geometry_clearance_passed
        && gates.terminal_entry_descending_and_clear
        && gates.terminal_handoff_on_descending_arc
        && gates.first_contact_stable_safe_on_target
        && gates.no_earlier_contact
        && gates.fuel_time_and_commandability_budgets_passed
        && gates.planned_time_reserve_passed
}

fn target_contact_only_rejection(gates: &crate::CompleteFlatAcceptanceGatesEvidence) -> bool {
    gates.supported_source_pad_rest_state
        && gates.launch_completed_and_contact_free
        && gates.launch_to_source_join_passed
        && gates.existing_source_screens_passed
        && gates.strict_source_handoff_passed
        && gates.scheduled_source_prefix_parity_passed
        && gates.ordinary_neutral_and_stored_replay_parity_passed
        && gates.pointwise_core_geometry_clearance_passed
        && gates.terminal_entry_descending_and_clear
        && gates.terminal_handoff_on_descending_arc
        && !gates.first_contact_stable_safe_on_target
        && gates.no_earlier_contact
        && gates.fuel_time_and_commandability_budgets_passed
        && gates.planned_time_reserve_passed
}

fn wrapper_provenance_matches(
    provenance_row_index: usize,
    provenance_scenario_identity: &str,
    provenance_policy_identity: &str,
    row_index: usize,
    generation_scenario_identity: &str,
    generation_policy: &WaypointDirectNominalDirectGenerationPolicyV1,
) -> Result<bool> {
    Ok(provenance_row_index == row_index
        && provenance_scenario_identity == generation_scenario_identity
        && provenance_policy_identity == semantic_digest(&generation_policy.analytical_policy)?)
}

#[derive(Default)]
struct GenerationCounts {
    row_count: usize,
    analytical_skip_count: usize,
    scheduled_count: usize,
    fully_wrapped_count: usize,
    accepted_count: usize,
    target_contact_rejection_count: usize,
}

fn generation_counts(
    generation: &WaypointDirectNominalDirectGenerationArtifact,
) -> GenerationCounts {
    GenerationCounts {
        row_count: generation.rows.len(),
        analytical_skip_count: generation
            .rows
            .iter()
            .filter(|row| !row.analytical_survivor)
            .count(),
        scheduled_count: generation
            .rows
            .iter()
            .filter(|row| row.paired_schedule.is_some())
            .count(),
        fully_wrapped_count: generation
            .rows
            .iter()
            .filter(|row| row.wrapper.is_some())
            .count(),
        accepted_count: generation.rows.iter().filter(|row| row.accepted).count(),
        target_contact_rejection_count: generation
            .rows
            .iter()
            .filter(|row| {
                row.wrapper
                    .as_ref()
                    .is_some_and(|wrapper| target_contact_only_rejection(&wrapper.acceptance))
            })
            .count(),
    }
}

fn validate_ledger_totals(totals: &TerminalInputLedgerTotalsV1) -> Result<()> {
    if totals.row_count != EXPECTED_LEDGER_ROWS
        || totals.analytical_skip_count != EXPECTED_ANALYTICAL_SKIPS
        || totals.scheduled_count != EXPECTED_SCHEDULED_ROWS
        || totals.fully_wrapped_count != EXPECTED_SCHEDULED_ROWS
        || totals.accepted_count != EXPECTED_ACCEPTED_ROWS
        || totals.target_contact_rejection_count != EXPECTED_TARGET_CONTACT_REJECTIONS
    {
        bail!("frozen obstacle ledger totals changed");
    }
    Ok(())
}

fn expected_selectors() -> Vec<TerminalCadenceSelectorV1> {
    EXPECTED_SELECTORS
        .iter()
        .map(
            |(case_id, row_index, selection_class)| TerminalCadenceSelectorV1 {
                case_id: (*case_id).to_owned(),
                row_index: *row_index,
                selection_class: *selection_class,
            },
        )
        .collect()
}

fn validate_selector_coverage(selectors: &[TerminalCadenceSelectorV1]) -> Result<()> {
    let expected = expected_selectors();
    if selectors != expected.as_slice() {
        bail!("terminal cadence selectors differ from the nineteen predeclared rows");
    }
    let unique = selectors
        .iter()
        .map(|selector| (selector.case_id.as_str(), selector.row_index))
        .collect::<std::collections::BTreeSet<_>>();
    if unique.len() != EXPECTED_SELECTORS.len() {
        bail!("terminal cadence selectors contain duplicate case/row keys");
    }
    Ok(())
}

fn validate_selected_rows(
    cases: &[LoadedCase],
    selectors: &[TerminalCadenceSelectorV1],
) -> Result<()> {
    for selector in selectors {
        let case = cases
            .iter()
            .find(|case| case.expected.case_id == selector.case_id)
            .context("selector case missing from frozen generation family")?;
        let row = case
            .generation
            .rows
            .get(selector.row_index)
            .context("selector row outside frozen generation")?;
        let wrapper = row
            .wrapper
            .as_ref()
            .context("selector row lacks a complete wrapper")?;
        if row.paired_schedule.is_none() {
            bail!("selector row lacks frozen paired schedule");
        }
        if !selected_status_is_valid(
            selector.selection_class,
            row.accepted,
            &wrapper.acceptance,
            wrapper.first_failing_gate.as_deref(),
            wrapper.first_contact.is_some(),
        ) {
            bail!("selected row no longer has its frozen acceptance/contact status");
        }
    }
    Ok(())
}

fn selected_status_is_valid(
    selection_class: TerminalCadenceSelectionClassV1,
    accepted: bool,
    gates: &crate::CompleteFlatAcceptanceGatesEvidence,
    first_failing_gate: Option<&str>,
    has_first_contact: bool,
) -> bool {
    match selection_class {
        TerminalCadenceSelectionClassV1::HighObstacleFailure
        | TerminalCadenceSelectionClassV1::NearMarginFailure => {
            !accepted
                && target_contact_only_rejection(gates)
                && first_failing_gate == Some("first_contact_stable_safe_on_target")
                && has_first_contact
        }
        TerminalCadenceSelectionClassV1::AcceptedControl => {
            accepted && complete_acceptance_gates_pass(gates) && first_failing_gate.is_none()
        }
    }
}

fn evaluate_case(
    loaded_case: &LoadedCase,
    preflight: &TerminalAdmissibilityPreflightV1,
) -> Result<TerminalAdmissibilityCaseArtifactV1> {
    let selected = preflight
        .cadence_selectors
        .iter()
        .map(|selector| {
            (
                (selector.case_id.as_str(), selector.row_index),
                selector.selection_class,
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut scheduled_rows = Vec::with_capacity(loaded_case.expected.scheduled_rows);
    for row in &loaded_case.generation.rows {
        if row.paired_schedule.is_none() {
            continue;
        }
        let selection_class = selected
            .get(&(loaded_case.expected.case_id, row.row_index))
            .copied();
        let result = evaluate_waypoint_direct_terminal_row(
            &loaded_case.generation,
            row.row_index,
            selection_class.is_some(),
        );
        let (helper_succeeded, failure_kind, evidence) = match result {
            Ok(evidence) => (true, None, Some(evidence)),
            Err(error) => {
                eprintln!(
                    "terminal admissibility helper failed for {} row {}: {error:#}",
                    loaded_case.expected.case_id, row.row_index
                );
                (false, Some("terminal_row_helper_failed".to_owned()), None)
            }
        };
        scheduled_rows.push(TerminalScheduledRowResultV1 {
            row_index: row.row_index,
            basis_index: row.basis_index,
            duration_offset_ticks: row.duration_offset_ticks,
            original_accepted: row.accepted,
            cadence_selection_class: selection_class,
            helper_succeeded,
            failure_kind,
            evidence,
        });
    }
    let skipped_row_indices = loaded_case
        .generation
        .rows
        .iter()
        .filter(|row| row.paired_schedule.is_none())
        .map(|row| row.row_index)
        .collect::<Vec<_>>();
    let baseline_pass_count = scheduled_rows
        .iter()
        .filter(|row| {
            row.evidence
                .as_ref()
                .is_some_and(|evidence| evidence.frozen_baseline.passed)
        })
        .count();
    let reference_binding_pass_count = scheduled_rows
        .iter()
        .filter(|row| {
            row.evidence
                .as_ref()
                .is_some_and(|evidence| evidence.reference.reference_binding_passed)
        })
        .count();
    let reference_pose_core_match_pass_count = scheduled_rows
        .iter()
        .filter(|row| {
            row.evidence
                .as_ref()
                .is_some_and(|evidence| evidence.reference.core_probes_match_reference_poses)
        })
        .count();
    let core_predicate_mirror_pass_count = scheduled_rows
        .iter()
        .filter(|row| {
            row.evidence
                .as_ref()
                .is_some_and(|evidence| evidence.reference.core_predicate_mirror_parity)
        })
        .count();
    let cadence_comparison_count = scheduled_rows
        .iter()
        .filter(|row| {
            row.evidence
                .as_ref()
                .is_some_and(|evidence| evidence.cadence_comparison.is_some())
        })
        .count();
    let helper_error_count = scheduled_rows
        .iter()
        .filter(|row| !row.helper_succeeded)
        .count();
    let mandatory_guards_passed = scheduled_rows.len() == loaded_case.expected.scheduled_rows
        && baseline_pass_count == scheduled_rows.len()
        && reference_binding_pass_count == scheduled_rows.len()
        && reference_pose_core_match_pass_count == scheduled_rows.len()
        && core_predicate_mirror_pass_count == scheduled_rows.len()
        && helper_error_count == 0
        && scheduled_rows.iter().all(row_mandatory_guards_passed);
    let mut artifact = TerminalAdmissibilityCaseArtifactV1 {
        schema_id: WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_SCHEMA_VERSION,
        input_gate_semantic_identity: preflight.input_gate_semantic_identity.clone(),
        input_manifest_sha256: preflight.fresh_manifest_sha256.clone(),
        generation_identity: loaded_case.generation.identity.clone(),
        generation_summary_sha256: loaded_case.generation_summary_sha256.clone(),
        case_id: loaded_case.expected.case_id.to_owned(),
        profile: loaded_case.expected.profile.to_owned(),
        horizontal_span_m: loaded_case.expected.span_m,
        skipped_row_indices,
        scheduled_row_count: scheduled_rows.len(),
        scheduled_rows,
        baseline_pass_count,
        reference_binding_pass_count,
        reference_pose_core_match_pass_count,
        core_predicate_mirror_pass_count,
        cadence_comparison_count,
        helper_error_count,
        mandatory_guards_passed,
        identity: String::new(),
    };
    artifact.identity = case_artifact_identity(&artifact)?;
    Ok(artifact)
}

fn row_mandatory_guards_passed(row: &TerminalScheduledRowResultV1) -> bool {
    let Some(evidence) = row.evidence.as_ref() else {
        return false;
    };
    if !row.helper_succeeded
        || !helper_row_identity(evidence).is_ok_and(|identity| identity == evidence.identity)
        || !evidence.frozen_baseline.passed
        || !evidence.frozen_baseline.authoritative_replay_passed
        || !evidence
            .frozen_baseline
            .authoritative_contact_matches_frozen
        || !evidence.frozen_baseline.prefix_contact_free
        || !evidence.frozen_baseline.terminal_log_matches_replay
        || !evidence.frozen_baseline.ordinary_neutral_parity
        || !evidence.frozen_baseline.contact_matches_frozen
        || !evidence.reference.reference_binding_passed
        || !evidence.reference.core_probes_match_reference_poses
        || !evidence.reference.core_predicate_mirror_parity
    {
        return false;
    }
    match (&row.cadence_selection_class, &evidence.cadence_comparison) {
        (Some(_), Some(comparison)) => {
            comparison.matched_entry_and_global_phase
                && comparison.held_60hz.ordinary_neutral_parity
                && comparison.terminal_only_120hz.ordinary_neutral_parity
                && comparison.held_60hz.core_predicate_mirror_parity
                && comparison.terminal_only_120hz.core_predicate_mirror_parity
        }
        (None, None) => true,
        _ => false,
    }
}

fn reference_observation_counts(
    rows: &[TerminalScheduledRowResultV1],
) -> Vec<TerminalReferenceObservationCountsV1> {
    [true, false]
        .into_iter()
        .map(|accepted| {
            let mut counts = TerminalReferenceObservationCountsV1 {
                original_witness_accepted: accepted,
                ..TerminalReferenceObservationCountsV1::default()
            };
            for row in rows.iter().filter(|row| row.original_accepted == accepted) {
                counts.rows += 1;
                if let Some(evidence) = row.evidence.as_ref() {
                    if evidence.reference.admissible_reference_pose_path {
                        counts.admissible_reference_pose_paths += 1;
                    } else {
                        counts.inadmissible_reference_pose_paths += 1;
                    }
                    if let Some(contact) = evidence.reference.first_contact.as_ref() {
                        counts.rows_with_first_reference_contact += 1;
                        if contact.predicates.stable_maximum_clearance_predicate {
                            counts.reference_foot_predicate_passes += 1;
                        } else {
                            counts.reference_foot_predicate_failures += 1;
                        }
                        if contact.predicates.safe_normal_speed_predicate {
                            counts.reference_normal_speed_predicate_passes += 1;
                        } else {
                            counts.reference_normal_speed_predicate_failures += 1;
                        }
                    }
                }
            }
            counts
        })
        .collect()
}

fn cadence_outcome_counts(
    rows: &[TerminalScheduledRowResultV1],
) -> Vec<TerminalCadenceClassCountsV1> {
    [
        TerminalCadenceSelectionClassV1::HighObstacleFailure,
        TerminalCadenceSelectionClassV1::NearMarginFailure,
        TerminalCadenceSelectionClassV1::AcceptedControl,
    ]
    .into_iter()
    .map(|class| {
        let mut outcomes = TerminalCadenceOutcomeCountsV1::default();
        for row in rows
            .iter()
            .filter(|row| row.cadence_selection_class == Some(class))
        {
            let Some(comparison) = row
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.cadence_comparison.as_ref())
            else {
                continue;
            };
            outcomes.rows += 1;
            let held = comparison.held_60hz.first_contact_stable_safe_on_target;
            let per_tick = comparison
                .terminal_only_120hz
                .first_contact_stable_safe_on_target;
            match (held, per_tick) {
                (true, true) => {
                    outcomes.held_60hz_stable_target_contacts += 1;
                    outcomes.terminal_only_120hz_stable_target_contacts += 1;
                    outcomes.both_lanes_stable_target_contacts += 1;
                }
                (true, false) => {
                    outcomes.held_60hz_stable_target_contacts += 1;
                    outcomes.held_60hz_only_stable_target_contacts += 1;
                }
                (false, true) => {
                    outcomes.terminal_only_120hz_stable_target_contacts += 1;
                    outcomes.terminal_only_120hz_only_stable_target_contacts += 1;
                }
                (false, false) => outcomes.neither_lane_stable_target_contact += 1,
            }
            outcomes.matched_entry_and_global_phase_rows +=
                usize::from(comparison.matched_entry_and_global_phase);
            outcomes.held_60hz_ordinary_neutral_parity_rows +=
                usize::from(comparison.held_60hz.ordinary_neutral_parity);
            outcomes.terminal_only_120hz_ordinary_neutral_parity_rows +=
                usize::from(comparison.terminal_only_120hz.ordinary_neutral_parity);
        }
        TerminalCadenceClassCountsV1 {
            selection_class: class,
            outcomes,
        }
    })
    .collect()
}

fn reserve_output_root(repo_root: &Path, requested: &Path) -> Result<PathBuf> {
    let output_root = resolve_from_repo(repo_root, requested);
    let parent = output_root
        .parent()
        .context("output directory has no parent")?;
    if !parent.is_dir() {
        bail!("output parent must already exist: {}", parent.display());
    }
    fs::create_dir(&output_root).with_context(|| {
        format!(
            "creating create-only output directory {}",
            output_root.display()
        )
    })?;
    Ok(output_root)
}

fn ensure_output_target_available(repo_root: &Path, requested: &Path) -> Result<()> {
    let output_root = resolve_from_repo(repo_root, requested);
    if output_root.exists() {
        bail!("output directory already exists: {}", output_root.display());
    }
    Ok(())
}

fn require_source_identity(
    expected: &str,
    observed: &str,
    case_id: &str,
    boundary: &str,
) -> Result<()> {
    if expected != observed {
        bail!("terminal diagnostic source binding changed {boundary} {case_id}");
    }
    Ok(())
}

fn evaluate_if_source_stable<T>(
    expected: &str,
    observed: &str,
    case_id: &str,
    evaluate: impl FnOnce() -> Result<T>,
) -> Result<T> {
    require_source_identity(expected, observed, case_id, "before case")?;
    evaluate()
}

fn write_create_only(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("creating output file {} without overwrite", path.display()))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn load_source_files(repo_root: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    fn visit_rust_files(
        repo_root: &Path,
        directory: &Path,
        files: &mut Vec<(String, Vec<u8>)>,
    ) -> Result<()> {
        let mut paths = fs::read_dir(directory)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        paths.sort();
        for path in paths {
            if path.is_dir() {
                visit_rust_files(repo_root, &path, files)?;
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let relative = path.strip_prefix(repo_root)?.to_string_lossy().into_owned();
                files.push((relative, fs::read(&path)?));
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    for crate_name in SOURCE_CRATES {
        visit_rust_files(
            repo_root,
            &repo_root.join(crate_name).join("src"),
            &mut files,
        )?;
        let manifest = format!("{crate_name}/Cargo.toml");
        files.push((manifest.clone(), fs::read(repo_root.join(manifest))?));
    }
    for path in ["Cargo.toml", "Cargo.lock"] {
        files.push((path.to_owned(), fs::read(repo_root.join(path))?));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn source_identity_only(repo_root: &Path) -> Result<String> {
    let mut files = load_source_files(repo_root)?;
    let protocol_path = repo_root.join(WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_PROTOCOL);
    files.push((
        WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_PROTOCOL.to_owned(),
        fs::read(&protocol_path)
            .with_context(|| format!("reading protocol {}", protocol_path.display()))?,
    ));
    files.sort_by(|left, right| left.0.cmp(&right.0));
    semantic_digest(&files)
}

fn source_binding(repo_root: &Path) -> Result<TerminalSourceBindingV1> {
    let files = load_source_files(repo_root)?;
    let protocol_path = repo_root.join(WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_PROTOCOL);
    let protocol_bytes = fs::read(&protocol_path)
        .with_context(|| format!("reading protocol {}", protocol_path.display()))?;
    let protocol_sha256 = sha256_bytes(&protocol_bytes)?;
    let mut identity_files = files.clone();
    identity_files.push((
        WAYPOINT_DIRECT_TERMINAL_ADMISSIBILITY_PROTOCOL.to_owned(),
        protocol_bytes,
    ));
    identity_files.sort_by(|left, right| left.0.cmp(&right.0));
    let identity = semantic_digest(&identity_files)?;
    let file_bindings = files
        .iter()
        .map(|(relative_path, contents)| {
            Ok(TerminalSourceFileBindingV1 {
                relative_path: relative_path.clone(),
                raw_sha256: sha256_bytes(contents)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(TerminalSourceBindingV1 {
        identity,
        protocol_sha256,
        files: file_bindings,
    })
}

fn resolve_from_repo(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pd-eval crate should live under the workspace root")
        .to_path_buf()
}

fn obstacle_gate_identity(gate: &WaypointDirectObstacleDiscriminationArtifactV1) -> Result<String> {
    let mut input = gate.clone();
    input.identity.clear();
    semantic_digest(&input)
}

fn require_semantic_identity(
    claimed: &str,
    computed: &str,
    expected: &str,
    label: &str,
) -> Result<()> {
    if claimed != computed || claimed != expected {
        bail!("{label} semantic identity mismatch");
    }
    Ok(())
}

fn generation_identity(
    generation: &WaypointDirectNominalDirectGenerationArtifact,
) -> Result<String> {
    let mut input = generation.clone();
    input.identity.clear();
    semantic_digest(&input)
}

fn preflight_identity(preflight: &TerminalAdmissibilityPreflightV1) -> Result<String> {
    let mut input = preflight.clone();
    input.identity.clear();
    semantic_digest(&input)
}

fn case_artifact_identity(artifact: &TerminalAdmissibilityCaseArtifactV1) -> Result<String> {
    let mut input = artifact.clone();
    input.identity.clear();
    semantic_digest(&input)
}

fn helper_row_identity(evidence: &TerminalAdmissibilityRowEvidence) -> Result<String> {
    let mut input = evidence.clone();
    input.identity.clear();
    semantic_digest(&input)
}

fn terminal_artifact_identity(artifact: &TerminalAdmissibilityArtifactV1) -> Result<String> {
    let mut input = artifact.clone();
    input.identity.clear();
    semantic_digest(&input)
}

fn semantic_digest<T: Serialize>(value: &T) -> Result<String> {
    let hash = serde_json::to_vec(value)?
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    Ok(format!("fnv1a64:{hash:016x}"))
}

fn require_raw_sha256(bytes: &[u8], expected: &str, label: &str) -> Result<()> {
    let actual = sha256_bytes(bytes)?;
    if actual != expected {
        bail!("{label} raw SHA-256 mismatch");
    }
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> Result<String> {
    let mut child = Command::new("sha256sum")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("starting sha256sum")?;
    child
        .stdin
        .as_mut()
        .context("opening sha256sum stdin")?
        .write_all(bytes)?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!(
            "sha256sum failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let output = String::from_utf8(output.stdout).context("decoding sha256sum output")?;
    let digest = output
        .split_ascii_whitespace()
        .next()
        .context("sha256sum returned no digest")?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("sha256sum returned malformed digest");
    }
    Ok(digest.to_ascii_lowercase())
}

fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "pd-terminal-admissibility-{label}-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("unique temp directory");
        path
    }

    #[test]
    fn raw_input_pin_rejects_mutated_bytes() {
        let manifest_path = repo_root().join(FRESH_MANIFEST_RELATIVE_PATH);
        let manifest_bytes = fs::read(manifest_path).expect("manifest bytes");
        require_raw_sha256(&manifest_bytes, FRESH_MANIFEST_SHA256, "manifest").unwrap();
        let mut mutated_manifest = manifest_bytes;
        mutated_manifest.push(b'\n');
        assert!(require_raw_sha256(&mutated_manifest, FRESH_MANIFEST_SHA256, "manifest").is_err());
    }

    #[derive(Serialize)]
    struct SyntheticSemanticInput {
        case_order: Vec<&'static str>,
        row_count: usize,
    }

    #[test]
    fn semantic_identity_and_case_order_mutants_are_refused() {
        let input = SyntheticSemanticInput {
            case_order: EXPECTED_CASES.iter().map(|case| case.case_id).collect(),
            row_count: EXPECTED_LEDGER_ROWS,
        };
        let claimed = semantic_digest(&input).unwrap();
        require_semantic_identity(
            &claimed,
            &semantic_digest(&input).unwrap(),
            &claimed,
            "synthetic",
        )
        .unwrap();
        let mut mutated = input;
        mutated.row_count += 1;
        assert!(
            require_semantic_identity(
                &claimed,
                &semantic_digest(&mutated).unwrap(),
                &claimed,
                "synthetic"
            )
            .is_err()
        );
        assert!(
            require_semantic_identity(&claimed, &claimed, "fnv1a64:0000000000000000", "synthetic")
                .is_err()
        );

        let expected = EXPECTED_CASES
            .iter()
            .map(|case| case.case_id)
            .collect::<Vec<_>>();
        validate_case_id_order(&expected).unwrap();
        let mut reordered = expected.clone();
        reordered.swap(0, 1);
        assert!(validate_case_id_order(&reordered).is_err());
    }

    #[test]
    fn gate_and_ledger_count_mutants_are_refused() {
        let mut totals = TerminalInputLedgerTotalsV1 {
            row_count: EXPECTED_LEDGER_ROWS,
            analytical_skip_count: EXPECTED_ANALYTICAL_SKIPS,
            scheduled_count: EXPECTED_SCHEDULED_ROWS,
            fully_wrapped_count: EXPECTED_SCHEDULED_ROWS,
            accepted_count: EXPECTED_ACCEPTED_ROWS,
            target_contact_rejection_count: EXPECTED_TARGET_CONTACT_REJECTIONS,
        };
        validate_ledger_totals(&totals).unwrap();
        totals.accepted_count -= 1;
        assert!(validate_ledger_totals(&totals).is_err());

        assert!(validate_gate_ledger_counts(EXPECTED_CASES[0], 20, 13, 4).is_err());
        validate_gate_ledger_counts(EXPECTED_CASES[0], 20, 12, 4).unwrap();
    }

    #[test]
    fn selector_mutants_are_refused_and_expected_keys_are_unique() {
        let mut selectors = expected_selectors();
        validate_selector_coverage(&selectors).unwrap();
        selectors.swap(0, 1);
        assert!(validate_selector_coverage(&selectors).is_err());
        selectors.swap(0, 1);
        selectors.pop();
        assert!(validate_selector_coverage(&selectors).is_err());
        selectors = expected_selectors();
        selectors[1] = selectors[0].clone();
        assert!(validate_selector_coverage(&selectors).is_err());
    }

    fn passing_acceptance_gates() -> crate::CompleteFlatAcceptanceGatesEvidence {
        crate::CompleteFlatAcceptanceGatesEvidence {
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
        }
    }

    #[test]
    fn selected_rows_require_acceptance_or_only_target_contact_failure() {
        let accepted = passing_acceptance_gates();
        assert!(selected_status_is_valid(
            TerminalCadenceSelectionClassV1::AcceptedControl,
            true,
            &accepted,
            None,
            true
        ));
        assert!(!selected_status_is_valid(
            TerminalCadenceSelectionClassV1::AcceptedControl,
            false,
            &accepted,
            None,
            true
        ));

        let mut target_only_failure = passing_acceptance_gates();
        target_only_failure.first_contact_stable_safe_on_target = false;
        for class in [
            TerminalCadenceSelectionClassV1::HighObstacleFailure,
            TerminalCadenceSelectionClassV1::NearMarginFailure,
        ] {
            assert!(selected_status_is_valid(
                class,
                false,
                &target_only_failure,
                Some("first_contact_stable_safe_on_target"),
                true
            ));
            assert!(!selected_status_is_valid(
                class,
                false,
                &target_only_failure,
                Some("launch_to_source_join_passed"),
                true
            ));
            let mut earlier_failure = target_only_failure.clone();
            earlier_failure.terminal_handoff_on_descending_arc = false;
            assert!(!selected_status_is_valid(
                class,
                false,
                &earlier_failure,
                Some("first_contact_stable_safe_on_target"),
                true
            ));
            let mut non_contact_failure = target_only_failure.clone();
            non_contact_failure.first_contact_stable_safe_on_target = true;
            assert!(!selected_status_is_valid(
                class,
                false,
                &non_contact_failure,
                Some("first_contact_stable_safe_on_target"),
                true
            ));
        }

        let earlier_gate_mutations: [fn(&mut crate::CompleteFlatAcceptanceGatesEvidence); 13] = [
            |g| g.supported_source_pad_rest_state = false,
            |g| g.launch_completed_and_contact_free = false,
            |g| g.launch_to_source_join_passed = false,
            |g| g.existing_source_screens_passed = false,
            |g| g.strict_source_handoff_passed = false,
            |g| g.scheduled_source_prefix_parity_passed = false,
            |g| g.ordinary_neutral_and_stored_replay_parity_passed = false,
            |g| g.pointwise_core_geometry_clearance_passed = false,
            |g| g.terminal_entry_descending_and_clear = false,
            |g| g.terminal_handoff_on_descending_arc = false,
            |g| g.no_earlier_contact = false,
            |g| g.fuel_time_and_commandability_budgets_passed = false,
            |g| g.planned_time_reserve_passed = false,
        ];
        for mutate in earlier_gate_mutations {
            let mut gates = target_only_failure.clone();
            mutate(&mut gates);
            assert!(!selected_status_is_valid(
                TerminalCadenceSelectionClassV1::NearMarginFailure,
                false,
                &gates,
                Some("first_contact_stable_safe_on_target"),
                true
            ));
        }
    }

    #[test]
    fn wrapper_policy_provenance_uses_analytical_policy_identity() {
        let generation_policy = WaypointDirectNominalDirectGenerationPolicyV1::default();
        let generation_identity = semantic_digest(&generation_policy).unwrap();
        let analytical_identity = semantic_digest(&generation_policy.analytical_policy).unwrap();
        assert_ne!(generation_identity, analytical_identity);
        assert!(
            wrapper_provenance_matches(
                5,
                "scenario",
                &analytical_identity,
                5,
                "scenario",
                &generation_policy
            )
            .unwrap()
        );
        assert!(
            !wrapper_provenance_matches(
                5,
                "scenario",
                &generation_identity,
                5,
                "scenario",
                &generation_policy
            )
            .unwrap()
        );
    }

    #[test]
    fn observation_strata_label_accepted_and_rejected_even_when_empty() {
        let strata = reference_observation_counts(&[]);
        assert_eq!(strata.len(), 2);
        assert!(strata[0].original_witness_accepted);
        assert!(!strata[1].original_witness_accepted);
        assert_eq!(strata[0].rows, 0);
        assert_eq!(strata[1].rows, 0);
    }

    #[test]
    fn source_drift_refuses_the_case_before_invoking_its_evaluator() {
        use std::cell::Cell;

        let evaluated = Cell::new(false);
        let result = evaluate_if_source_stable("sealed", "changed", "case-x", || {
            evaluated.set(true);
            Ok(())
        });
        assert!(result.is_err());
        assert!(!evaluated.get());
        assert!(require_source_identity("sealed", "changed", "case-x", "after case").is_err());
    }

    #[test]
    fn existing_output_root_is_refused_without_touching_contents() {
        let temp = temp_dir("output-refusal");
        let output = temp.join("existing");
        fs::create_dir(&output).unwrap();
        let sentinel = output.join("sentinel");
        fs::write(&sentinel, b"preserve").unwrap();
        let error =
            run_waypoint_direct_terminal_admissibility(&temp.join("missing-input"), &output)
                .expect_err("existing output is refused before trying to load missing inputs");
        assert!(
            error
                .to_string()
                .contains("output directory already exists")
        );
        assert_eq!(fs::read(&sentinel).unwrap(), b"preserve");
        fs::remove_dir_all(temp).unwrap();
    }
}
