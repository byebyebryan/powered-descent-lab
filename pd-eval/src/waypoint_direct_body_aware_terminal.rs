//! Create-only runner and staged gates for the opt-in body-aware terminal lane.

use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

use crate::{
    BodyAwareTerminalCaseArtifactV1, BodyAwareTerminalPolicyV1,
    WaypointDirectGenerationFreshManifest, WaypointDirectNominalDirectGenerationPolicyV1,
    WaypointDirectNominalDirectGenerationRequest, WaypointDirectObstacleDiscriminationFreshCaseV1,
    WaypointDirectObstacleDiscriminationFreshManifestV1, body_aware_terminal_case_identity,
    evaluate_waypoint_direct_body_aware_terminal, load_waypoint_direct_generation_fresh_manifest,
    load_waypoint_direct_obstacle_discrimination_fresh_manifest,
    validate_body_aware_terminal_policy,
    validate_waypoint_direct_nominal_direct_generation_request, verify_body_aware_terminal_case,
};

pub(crate) use crate::evidence_io::sha256_bytes;

pub const WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_FRESH_MANIFEST: &str =
    "fixtures/research/waypoint_direct_body_aware_terminal_fresh_inputs_v1.json";
pub const WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_FRESH_MANIFEST_SHA256: &str =
    "f7b982724cfc8e4df7fd312519a11cfcb1cea25348c7e9451269c8a3e175ddb7";
pub const WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_PROTOCOL: &str =
    "docs/waypoint_direct_body_aware_terminal_protocol.md";

const DEVELOPMENT_SCHEMA_ID: &str = "waypoint_direct_body_aware_terminal_development_v1";
const FREEZE_SCHEMA_ID: &str = "waypoint_direct_body_aware_terminal_code_freeze_v1";
const FRESH_SCHEMA_ID: &str = "waypoint_direct_body_aware_terminal_fresh_gate_v1";
const PREFLIGHT_SCHEMA_ID: &str = "waypoint_direct_body_aware_terminal_preflight_v1";
const GENERATION_MANIFEST_SHA256: &str =
    "3dffd6bc3f220629b96e817af8e0be7f6cb2e905c75a1df796e97aa15b066d24";
const OBSTACLE_MANIFEST_SHA256: &str =
    "647df7bc94d02a9b48b773c45159e0ffa15bfca232a13b4e21164e2251a8f5b4";
const GENERATION_MANIFEST: &str =
    "fixtures/research/waypoint_direct_generation_fresh_inputs_v1.json";
const OBSTACLE_MANIFEST: &str =
    "fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json";
const SOURCE_PACKAGES: [&str; 6] = [
    "pd-core",
    "pd-plan",
    "pd-control",
    "pd-eval",
    "pd-report",
    "pd-cli",
];
const FINGERPRINT_INPUTS: [&str; 6] = [
    "fixtures/research/waypoint_direct_generation_fresh_inputs_v1.json",
    "fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json",
    WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_FRESH_MANIFEST,
    WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_PROTOCOL,
    "fixtures/scenarios/flat_terminal_descent.json",
    "pd-plan/fixtures/conservative_ballistic_direct_bridge_probes_v2.json",
];
const ROOT_CARGO_FILES: [&str; 2] = ["Cargo.toml", "Cargo.lock"];

const SCOPE_NON_CLAIMS: [&str; 5] = [
    "Evaluator-only nominal evidence; planner, core, controller, source policy, and defaults are unchanged.",
    "Finite Unknown is not physical impossibility and does not establish waypoint necessity.",
    "Reference-pose clearance is conditional geometry; acceptance requires complete ordinary and neutral replay.",
    "Discrete nominal witnesses do not establish swept-path safety or perturbation robustness.",
    "No arbitrary incoming waypoint state, composition, real-time planning, or production authority is claimed.",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BodyAwareTerminalFreshManifestV1 {
    schema_id: String,
    schema_version: u32,
    sealed_before_implementation: bool,
    generation_policy: crate::WaypointDirectNominalDirectGenerationPolicyV1,
    terminal_policy: BodyAwareTerminalPolicyV1,
    fresh_case_order: String,
    cases: Vec<BodyAwareTerminalFreshCaseV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BodyAwareTerminalFreshCaseV1 {
    case_id: String,
    horizontal_span_m: f64,
    profile: String,
    height_change_m: f64,
    source_pad_id: String,
    target_pad_id: String,
    probe_id: String,
    scenario: pd_core::ScenarioSpec,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct HistoricalSummaryPin {
    case_id: &'static str,
    relative_path: &'static str,
    sha256: &'static str,
    was_direct: bool,
}

const HISTORICAL_SUMMARY_PINS: [HistoricalSummaryPin; 14] = [
    HistoricalSummaryPin {
        case_id: "fresh_flat_span_600_delta_000",
        relative_path: "outputs/research/waypoint_direct_generation_20260925/gate_b/run_a/cases/fresh_flat_span_600_delta_000/summary.json",
        sha256: "bf3ac805a45847b86ec4f3f3eca7c80a2263d9354fff225f215a81294fea718d",
        was_direct: true,
    },
    HistoricalSummaryPin {
        case_id: "fresh_uphill_span_600_delta_p120",
        relative_path: "outputs/research/waypoint_direct_generation_20260925/gate_b/run_a/cases/fresh_uphill_span_600_delta_p120/summary.json",
        sha256: "a37e70072a1c1db1952af8e7232d56c4937c6934599e2551a298b587750d4a67",
        was_direct: true,
    },
    HistoricalSummaryPin {
        case_id: "fresh_downhill_span_600_delta_m120",
        relative_path: "outputs/research/waypoint_direct_generation_20260925/gate_b/run_a/cases/fresh_downhill_span_600_delta_m120/summary.json",
        sha256: "2694b851f9d8d9dc1fcaacf6d9000794aeaff2d8b05bea64fb22c60a2e85de50",
        was_direct: true,
    },
    HistoricalSummaryPin {
        case_id: "fresh_flat_span_1000_delta_000",
        relative_path: "outputs/research/waypoint_direct_generation_20260925/gate_b/run_a/cases/fresh_flat_span_1000_delta_000/summary.json",
        sha256: "29971563e0a09071d0eca7a71046f9d9fa43b236273e473c74923168d8b71a42",
        was_direct: true,
    },
    HistoricalSummaryPin {
        case_id: "fresh_uphill_span_1000_delta_p120",
        relative_path: "outputs/research/waypoint_direct_generation_20260925/gate_b/run_a/cases/fresh_uphill_span_1000_delta_p120/summary.json",
        sha256: "1eadc32349276addd2d6d8c5b413c9b21f829593956cc9db2666a70feedfac67",
        was_direct: true,
    },
    HistoricalSummaryPin {
        case_id: "fresh_downhill_span_1000_delta_m120",
        relative_path: "outputs/research/waypoint_direct_generation_20260925/gate_b/run_a/cases/fresh_downhill_span_1000_delta_m120/summary.json",
        sha256: "9046d2069199cab1a0b17603fa3a2e0b1ae1603e904f1ba90c837a53d52a415a",
        was_direct: true,
    },
    HistoricalSummaryPin {
        case_id: "fresh_flat_control_span_700",
        relative_path: "outputs/research/waypoint_direct_obstacle_discrimination_20260925/fresh_run_a/cases/fresh_flat_control_span_700/generation/summary.json",
        sha256: "3f376ef069e94883a099aa25630f937f42f7b74d31ddc97e15c4f90a12f9f5e0",
        was_direct: true,
    },
    HistoricalSummaryPin {
        case_id: "fresh_low_obstacle_span_700",
        relative_path: "outputs/research/waypoint_direct_obstacle_discrimination_20260925/fresh_run_a/cases/fresh_low_obstacle_span_700/generation/summary.json",
        sha256: "0f5f7cb0219ea35afb9c812d1d3542ca53b208fc690a7383778e83988895f019",
        was_direct: true,
    },
    HistoricalSummaryPin {
        case_id: "fresh_high_obstacle_span_700",
        relative_path: "outputs/research/waypoint_direct_obstacle_discrimination_20260925/fresh_run_a/cases/fresh_high_obstacle_span_700/generation/summary.json",
        sha256: "7bdc6d9997484e2116d093d96273989255f01023c6f264c1c5d28035e0794926",
        was_direct: false,
    },
    HistoricalSummaryPin {
        case_id: "fresh_late_broad_span_700",
        relative_path: "outputs/research/waypoint_direct_obstacle_discrimination_20260925/fresh_run_a/cases/fresh_late_broad_span_700/generation/summary.json",
        sha256: "7a9fb06fbd74c84f9e4f9bbc26af0f0024f73cf809070f6bf497580aebc6e1a2",
        was_direct: true,
    },
    HistoricalSummaryPin {
        case_id: "fresh_flat_control_span_900",
        relative_path: "outputs/research/waypoint_direct_obstacle_discrimination_20260925/fresh_run_a/cases/fresh_flat_control_span_900/generation/summary.json",
        sha256: "fda664457c05b937c98d8299aaaaef3166e13db4041116e2d42932933a184018",
        was_direct: true,
    },
    HistoricalSummaryPin {
        case_id: "fresh_low_obstacle_span_900",
        relative_path: "outputs/research/waypoint_direct_obstacle_discrimination_20260925/fresh_run_a/cases/fresh_low_obstacle_span_900/generation/summary.json",
        sha256: "a584ec8457f87ed38ec8c0ad0f1603cc23010d4399e697acdcd6d4de6a9d1abd",
        was_direct: true,
    },
    HistoricalSummaryPin {
        case_id: "fresh_high_obstacle_span_900",
        relative_path: "outputs/research/waypoint_direct_obstacle_discrimination_20260925/fresh_run_a/cases/fresh_high_obstacle_span_900/generation/summary.json",
        sha256: "154040ea0633b4c66276c4722f8e35b50cdfc92254811205e6c82116fab1ec7a",
        was_direct: false,
    },
    HistoricalSummaryPin {
        case_id: "fresh_late_broad_span_900",
        relative_path: "outputs/research/waypoint_direct_obstacle_discrimination_20260925/fresh_run_a/cases/fresh_late_broad_span_900/generation/summary.json",
        sha256: "b2592ea3d58dc6e80cb05a524d6d399a8dc7a8ec0912b72c7ccec0bc1acab62d",
        was_direct: true,
    },
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalFileBindingV1 {
    pub relative_path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalSourceBindingV1 {
    pub identity: String,
    pub protocol_sha256: String,
    pub generation_manifest_sha256: String,
    pub obstacle_manifest_sha256: String,
    pub fresh_manifest_sha256: String,
    pub files: Vec<BodyAwareTerminalFileBindingV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalHistoricalBindingV1 {
    pub case_id: String,
    pub relative_path: String,
    pub summary_sha256: String,
    pub request_identity: String,
    pub previously_direct: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalPreflightV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub phase: String,
    pub source_binding: BodyAwareTerminalSourceBindingV1,
    pub historical_summaries: Vec<BodyAwareTerminalHistoricalBindingV1>,
    pub case_order: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalCaseReferenceV1 {
    pub case_id: String,
    pub previous_direct: Option<bool>,
    pub baseline_summary_sha256: Option<String>,
    pub baseline_generation_identity: Option<String>,
    pub baseline_generation_sha256: Option<String>,
    pub baseline_sha256_matches_history: Option<bool>,
    pub accepted_witness_count: usize,
    pub selected_row_index: Option<usize>,
    pub case_identity: Option<String>,
    pub case_summary_sha256: Option<String>,
    pub relative_path: Option<String>,
    pub status: String,
    pub failure_kind: Option<String>,
    pub failure_detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalSourceCheckV1 {
    pub case_id: String,
    pub before_identity: String,
    pub after_identity: String,
    pub unchanged_before_case: bool,
    pub unchanged_during_case: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalDevelopmentArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub phase: String,
    pub source_binding: BodyAwareTerminalSourceBindingV1,
    pub historical_summaries: Vec<BodyAwareTerminalHistoricalBindingV1>,
    pub rows: Vec<BodyAwareTerminalCaseReferenceV1>,
    pub source_checks: Vec<BodyAwareTerminalSourceCheckV1>,
    pub previously_direct_case_count: usize,
    pub retained_previous_direct_count: usize,
    pub high_obstacle_case_count: usize,
    pub high_obstacle_direct_count: usize,
    pub historical_baselines_match: bool,
    pub all_cases_direct: bool,
    pub all_cases_recorded: bool,
    pub passed: bool,
    pub verdict: String,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalCodeFreezeV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub development_identity: String,
    pub source_binding: BodyAwareTerminalSourceBindingV1,
    pub historical_summaries: Vec<BodyAwareTerminalHistoricalBindingV1>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalFreshGateArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub phase: String,
    pub development_identity: String,
    pub code_freeze_identity: String,
    pub source_binding: BodyAwareTerminalSourceBindingV1,
    pub rows: Vec<BodyAwareTerminalCaseReferenceV1>,
    pub source_checks: Vec<BodyAwareTerminalSourceCheckV1>,
    pub all_cases_recorded: bool,
    pub all_cases_direct: bool,
    pub passed: bool,
    pub verdict: String,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug)]
struct BoundCase {
    case_id: String,
    request: WaypointDirectNominalDirectGenerationRequest,
    historical_pin: Option<HistoricalSummaryPin>,
}

#[derive(Clone, Debug)]
struct PreparedDevelopment {
    preflight: BodyAwareTerminalPreflightV1,
    cases: Vec<BoundCase>,
}

/// Run the input-only development preflight. It validates all sealed manifests
/// and all fourteen pinned old generator summaries without evaluating a case.
pub fn preflight_waypoint_direct_body_aware_terminal_development(
    repo_root: &Path,
) -> Result<BodyAwareTerminalPreflightV1> {
    Ok(prepare_development(repo_root)?.preflight)
}

/// Evaluate all fourteen historical development inputs through the new tail.
/// The output root is reserved before any candidate solve or simulation.
pub fn run_waypoint_direct_body_aware_terminal_development(
    repo_root: &Path,
    output_dir: &Path,
) -> Result<BodyAwareTerminalDevelopmentArtifactV1> {
    let prepared = prepare_development(repo_root)?;
    let output_root = reserve_output_root(repo_root, output_dir)?;
    let run = run_cases(
        repo_root,
        &output_root,
        &prepared.preflight.source_binding,
        &prepared.preflight.historical_summaries,
        &prepared.cases,
        true,
    )?;
    let retained_previous_direct_count = run
        .rows
        .iter()
        .filter(|row| row.previous_direct == Some(true) && row.status == "direct")
        .count();
    let high_obstacle_case_count = run
        .rows
        .iter()
        .filter(|row| row.case_id.contains("high_obstacle"))
        .count();
    let high_obstacle_direct_count = run
        .rows
        .iter()
        .filter(|row| row.case_id.contains("high_obstacle") && row.status == "direct")
        .count();
    let previously_direct_case_count = prepared
        .preflight
        .historical_summaries
        .iter()
        .filter(|summary| summary.previously_direct)
        .count();
    let historical_baselines_match = run.rows.len() == HISTORICAL_SUMMARY_PINS.len()
        && run.rows.iter().all(|row| {
            row.previous_direct.is_some()
                && row.baseline_sha256_matches_history == Some(true)
                && row.baseline_generation_sha256.is_some()
        });
    let all_cases_direct =
        run.rows.len() == prepared.cases.len() && run.rows.iter().all(|row| row.status == "direct");
    let all_cases_recorded = run.rows.len() == prepared.cases.len()
        && run.rows.iter().all(|row| {
            row.case_summary_sha256.is_some() && matches!(row.status.as_str(), "direct" | "unknown")
        });
    let passed = historical_baselines_match
        && retained_previous_direct_count == previously_direct_case_count
        && high_obstacle_case_count == 2
        && high_obstacle_direct_count == 2
        && all_cases_direct
        && all_cases_recorded
        && run
            .source_checks
            .iter()
            .all(|check| check.unchanged_before_case && check.unchanged_during_case);
    let mut artifact = BodyAwareTerminalDevelopmentArtifactV1 {
        schema_id: DEVELOPMENT_SCHEMA_ID.to_owned(),
        schema_version: 1,
        phase: "development".to_owned(),
        source_binding: prepared.preflight.source_binding,
        historical_summaries: prepared.preflight.historical_summaries,
        rows: run.rows,
        source_checks: run.source_checks,
        previously_direct_case_count,
        retained_previous_direct_count,
        high_obstacle_case_count,
        high_obstacle_direct_count,
        historical_baselines_match,
        all_cases_direct,
        all_cases_recorded,
        passed,
        verdict: if passed {
            "development_passed_primary_review_and_source_freeze_required"
        } else {
            "development_failed_stop_before_fresh_physics_without_policy_tuning"
        }
        .to_owned(),
        scope_non_claims: scope_non_claims(),
        identity: String::new(),
    };
    artifact.identity = development_identity(&artifact)?;
    write_create_only(&output_root.join("summary.json"), &artifact)?;
    Ok(artifact)
}

/// Input-only freeze preflight; no output or simulation is created.
pub fn preflight_waypoint_direct_body_aware_terminal_freeze(
    repo_root: &Path,
    development_summary: &Path,
) -> Result<BodyAwareTerminalPreflightV1> {
    prepare_freeze(repo_root, development_summary).map(|(preflight, _)| preflight)
}

/// Bind the currently reviewed passing development artifact to a source seal.
pub fn freeze_waypoint_direct_body_aware_terminal(
    repo_root: &Path,
    development_summary: &Path,
    output_dir: &Path,
) -> Result<BodyAwareTerminalCodeFreezeV1> {
    let (preflight, development) = prepare_freeze(repo_root, development_summary)?;
    let output_root = reserve_output_dir(repo_root, output_dir, false)?;
    let mut freeze = BodyAwareTerminalCodeFreezeV1 {
        schema_id: FREEZE_SCHEMA_ID.to_owned(),
        schema_version: 1,
        development_identity: development.identity,
        source_binding: preflight.source_binding,
        historical_summaries: preflight.historical_summaries,
        identity: String::new(),
    };
    freeze.identity = freeze_identity(&freeze)?;
    write_create_only(&output_root.join("summary.json"), &freeze)?;
    Ok(freeze)
}

/// Input-only fresh-gate preflight. It verifies accepted development, its
/// source freeze, current source identity, and all ten sealed fresh requests.
pub fn preflight_waypoint_direct_body_aware_terminal_fresh_gate(
    repo_root: &Path,
    development_summary: &Path,
    code_freeze_summary: &Path,
) -> Result<BodyAwareTerminalPreflightV1> {
    prepare_fresh_gate(repo_root, development_summary, code_freeze_summary)
        .map(|(preflight, _, _, _)| preflight)
}

/// Evaluate all ten sealed cases under the accepted development source freeze.
pub fn run_waypoint_direct_body_aware_terminal_fresh_gate(
    repo_root: &Path,
    development_summary: &Path,
    code_freeze_summary: &Path,
    output_dir: &Path,
) -> Result<BodyAwareTerminalFreshGateArtifactV1> {
    let (preflight, cases, development_identity, freeze_identity_value) =
        prepare_fresh_gate(repo_root, development_summary, code_freeze_summary)?;
    let output_root = reserve_output_root(repo_root, output_dir)?;
    let run = run_cases(
        repo_root,
        &output_root,
        &preflight.source_binding,
        &preflight.historical_summaries,
        &cases,
        false,
    )?;
    let all_cases_direct =
        run.rows.len() == cases.len() && run.rows.iter().all(|row| row.status == "direct");
    let all_cases_recorded = run.rows.len() == cases.len()
        && run.rows.iter().all(|row| {
            row.case_summary_sha256.is_some() && matches!(row.status.as_str(), "direct" | "unknown")
        });
    let passed = all_cases_direct
        && all_cases_recorded
        && run
            .source_checks
            .iter()
            .all(|check| check.unchanged_before_case && check.unchanged_during_case);
    let mut artifact = BodyAwareTerminalFreshGateArtifactV1 {
        schema_id: FRESH_SCHEMA_ID.to_owned(),
        schema_version: 1,
        phase: "fresh".to_owned(),
        development_identity,
        code_freeze_identity: freeze_identity_value,
        source_binding: preflight.source_binding,
        rows: run.rows,
        source_checks: run.source_checks,
        all_cases_recorded,
        all_cases_direct,
        passed,
        verdict: if passed {
            "all_sealed_fresh_cases_have_new_complete_direct_witnesses"
        } else {
            "fresh_gate_failed_finite_unknown_or_integrity_failure_stop_without_retuning"
        }
        .to_owned(),
        scope_non_claims: scope_non_claims(),
        identity: String::new(),
    };
    artifact.identity = fresh_identity(&artifact)?;
    write_create_only(&output_root.join("summary.json"), &artifact)?;
    Ok(artifact)
}

struct CaseRun {
    rows: Vec<BodyAwareTerminalCaseReferenceV1>,
    source_checks: Vec<BodyAwareTerminalSourceCheckV1>,
}

fn prepare_development(repo_root: &Path) -> Result<PreparedDevelopment> {
    let source_before = source_binding(repo_root)?;
    let cases = development_cases(repo_root)?;
    let fresh = load_body_aware_fresh_manifest(repo_root)?;
    let historical_summaries = validate_historical_summaries(repo_root, &cases)?;
    let source_after = source_binding(repo_root)?;
    if source_before != source_after {
        bail!("source or protocol inputs changed during development preflight");
    }
    let preflight = make_preflight(
        "development_preflight",
        source_after,
        historical_summaries,
        cases.iter().map(|case| case.case_id.clone()).collect(),
    )?;
    // Keep sealed fresh policy and case validation in the input-only preflight.
    validate_fresh_cases(&fresh)?;
    Ok(PreparedDevelopment { preflight, cases })
}

fn prepare_freeze(
    repo_root: &Path,
    development_summary: &Path,
) -> Result<(
    BodyAwareTerminalPreflightV1,
    BodyAwareTerminalDevelopmentArtifactV1,
)> {
    let development = load_development(repo_root, development_summary)?;
    require_passed_development(development.passed)?;
    let prepared = prepare_development(repo_root)?;
    if development.source_binding != prepared.preflight.source_binding
        || development.historical_summaries != prepared.preflight.historical_summaries
    {
        bail!("source, protocol, manifest, or historical inputs changed after development");
    }
    let preflight = make_preflight(
        "freeze_preflight",
        prepared.preflight.source_binding,
        prepared.preflight.historical_summaries,
        prepared.preflight.case_order,
    )?;
    Ok((preflight, development))
}

fn prepare_fresh_gate(
    repo_root: &Path,
    development_summary: &Path,
    code_freeze_summary: &Path,
) -> Result<(BodyAwareTerminalPreflightV1, Vec<BoundCase>, String, String)> {
    let development = load_development(repo_root, development_summary)?;
    require_passed_development(development.passed)?;
    let freeze: BodyAwareTerminalCodeFreezeV1 = read_bound_summary(code_freeze_summary)?;
    validate_freeze_binding(&development, &freeze)?;
    let prepared = prepare_development(repo_root)?;
    if prepared.preflight.source_binding != freeze.source_binding
        || prepared.preflight.historical_summaries != freeze.historical_summaries
    {
        bail!("source, protocol, manifest, or historical inputs changed after code freeze");
    }
    let fresh_manifest = load_body_aware_fresh_manifest(repo_root)?;
    let cases = fresh_cases(&fresh_manifest)?;
    let preflight = make_preflight(
        "fresh_preflight",
        prepared.preflight.source_binding,
        prepared.preflight.historical_summaries,
        cases.iter().map(|case| case.case_id.clone()).collect(),
    )?;
    Ok((preflight, cases, development.identity, freeze.identity))
}

fn development_cases(repo_root: &Path) -> Result<Vec<BoundCase>> {
    let generation: WaypointDirectGenerationFreshManifest =
        load_waypoint_direct_generation_fresh_manifest(repo_root)?;
    let obstacle: WaypointDirectObstacleDiscriminationFreshManifestV1 =
        load_waypoint_direct_obstacle_discrimination_fresh_manifest(repo_root)?;
    let mut cases = Vec::with_capacity(HISTORICAL_SUMMARY_PINS.len());
    for pin in &HISTORICAL_SUMMARY_PINS[..6] {
        cases.push(BoundCase {
            case_id: pin.case_id.to_owned(),
            request: generation.request(pin.case_id)?,
            historical_pin: Some(*pin),
        });
    }
    for pin in &HISTORICAL_SUMMARY_PINS[6..] {
        let case = obstacle
            .cases
            .iter()
            .find(|case| case.case_id == pin.case_id)
            .with_context(|| format!("sealed obstacle case {} missing", pin.case_id))?;
        cases.push(BoundCase {
            case_id: pin.case_id.to_owned(),
            request: obstacle_request(&obstacle, case),
            historical_pin: Some(*pin),
        });
    }
    if cases.len() != 14 {
        bail!("development input family must contain all fourteen exposed cases");
    }
    for case in &cases {
        validate_waypoint_direct_nominal_direct_generation_request(&case.request)?;
    }
    Ok(cases)
}

fn obstacle_request(
    manifest: &WaypointDirectObstacleDiscriminationFreshManifestV1,
    case: &WaypointDirectObstacleDiscriminationFreshCaseV1,
) -> WaypointDirectNominalDirectGenerationRequest {
    WaypointDirectNominalDirectGenerationRequest {
        scenario: case.scenario.clone(),
        source_pad_id: case.source_pad_id.clone(),
        target_pad_id: case.target_pad_id.clone(),
        probe_id: case.probe_id.clone(),
        policy: manifest.generation_policy.clone(),
    }
}

fn load_body_aware_fresh_manifest(repo_root: &Path) -> Result<BodyAwareTerminalFreshManifestV1> {
    let path = repo_root.join(WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_FRESH_MANIFEST);
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let digest = sha256_bytes(&bytes)?;
    if digest != WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_FRESH_MANIFEST_SHA256 {
        bail!("sealed body-aware fresh manifest SHA-256 mismatch");
    }
    let manifest: BodyAwareTerminalFreshManifestV1 = serde_json::from_slice(&bytes)?;
    validate_fresh_cases(&manifest)?;
    Ok(manifest)
}

fn validate_fresh_cases(manifest: &BodyAwareTerminalFreshManifestV1) -> Result<()> {
    const EXPECTED: [&str; 10] = [
        "unseen_flat_span_750",
        "unseen_uphill_span_750",
        "unseen_downhill_span_750",
        "unseen_high_obstacle_span_750",
        "unseen_late_broad_span_750",
        "unseen_flat_span_950",
        "unseen_uphill_span_950",
        "unseen_downhill_span_950",
        "unseen_high_obstacle_span_950",
        "unseen_late_broad_span_950",
    ];
    if manifest.schema_id != "waypoint_direct_body_aware_terminal_fresh_inputs_v1"
        || manifest.schema_version != 1
        || !manifest.sealed_before_implementation
        || manifest.generation_policy != WaypointDirectNominalDirectGenerationPolicyV1::default()
        || manifest.terminal_policy != BodyAwareTerminalPolicyV1::default()
        || manifest.fresh_case_order
            != "750 then 950 m; flat, uphill +100 m, downhill -100 m, high obstacle, late broad; all Direct required for a passing fresh capability gate"
        || manifest.cases.len() != EXPECTED.len()
    {
        bail!("unsupported or unsealed body-aware fresh manifest");
    }
    validate_body_aware_terminal_policy(&manifest.terminal_policy)?;
    for (case, expected_id) in manifest.cases.iter().zip(EXPECTED) {
        if case.case_id != expected_id
            || case.probe_id != expected_id
            || case.source_pad_id != "pad_source"
            || case.target_pad_id != "pad_main"
            || !matches!(case.horizontal_span_m, 750.0 | 950.0)
            || !matches!(
                case.profile.as_str(),
                "flat" | "uphill" | "downhill" | "high_obstacle" | "late_broad"
            )
        {
            bail!("body-aware fresh manifest case order or binding changed");
        }
        let request = WaypointDirectNominalDirectGenerationRequest {
            scenario: case.scenario.clone(),
            source_pad_id: case.source_pad_id.clone(),
            target_pad_id: case.target_pad_id.clone(),
            probe_id: case.probe_id.clone(),
            policy: manifest.generation_policy.clone(),
        };
        validate_waypoint_direct_nominal_direct_generation_request(&request)?;
    }
    Ok(())
}

fn fresh_cases(manifest: &BodyAwareTerminalFreshManifestV1) -> Result<Vec<BoundCase>> {
    validate_fresh_cases(manifest)?;
    Ok(manifest
        .cases
        .iter()
        .map(|case| BoundCase {
            case_id: case.case_id.clone(),
            request: WaypointDirectNominalDirectGenerationRequest {
                scenario: case.scenario.clone(),
                source_pad_id: case.source_pad_id.clone(),
                target_pad_id: case.target_pad_id.clone(),
                probe_id: case.probe_id.clone(),
                policy: manifest.generation_policy.clone(),
            },
            historical_pin: None,
        })
        .collect())
}

fn validate_historical_summaries(
    repo_root: &Path,
    cases: &[BoundCase],
) -> Result<Vec<BodyAwareTerminalHistoricalBindingV1>> {
    if cases.len() != HISTORICAL_SUMMARY_PINS.len() {
        bail!("historical comparison requires all fourteen exact input cases");
    }
    let mut historical = Vec::with_capacity(cases.len());
    for (case, pin) in cases.iter().zip(HISTORICAL_SUMMARY_PINS) {
        if case.case_id != pin.case_id || case.historical_pin != Some(pin) {
            bail!("historical comparison order does not match sealed case order");
        }
        let path = repo_root.join(pin.relative_path);
        let summary_sha256 = sha256_file(&path)?;
        if summary_sha256 != pin.sha256 {
            bail!(
                "pinned historical summary SHA-256 mismatch: {}",
                path.display()
            );
        }
        historical.push(BodyAwareTerminalHistoricalBindingV1 {
            case_id: pin.case_id.to_owned(),
            relative_path: pin.relative_path.to_owned(),
            summary_sha256,
            request_identity: semantic_digest(&case.request)?,
            previously_direct: pin.was_direct,
        });
    }
    Ok(historical)
}

fn source_binding(repo_root: &Path) -> Result<BodyAwareTerminalSourceBindingV1> {
    let mut paths = BTreeSet::new();
    for package in SOURCE_PACKAGES {
        collect_source_files(repo_root, &repo_root.join(package), &mut paths)?;
    }
    for path in ROOT_CARGO_FILES.into_iter().chain(FINGERPRINT_INPUTS) {
        paths.insert(path.to_owned());
    }
    let relative_paths = paths.into_iter().collect::<Vec<_>>();
    let absolute_paths = relative_paths
        .iter()
        .map(|relative| repo_root.join(relative))
        .collect::<Vec<_>>();
    let output = Command::new("sha256sum")
        .arg("--")
        .args(&absolute_paths)
        .output()
        .context("hashing source-bound body-aware inputs")?;
    if !output.status.success() {
        bail!("sha256sum failed while hashing source-bound inputs");
    }
    let stdout = std::str::from_utf8(&output.stdout)?;
    let mut digest_by_path = BTreeSet::new();
    let mut file_bindings = Vec::with_capacity(relative_paths.len());
    for (line, relative_path) in stdout.lines().zip(&relative_paths) {
        let digest = line
            .split_whitespace()
            .next()
            .context("sha256sum source output omitted a digest")?;
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("sha256sum returned a malformed source digest");
        }
        digest_by_path.insert((relative_path.clone(), digest.to_owned()));
        file_bindings.push(BodyAwareTerminalFileBindingV1 {
            relative_path: relative_path.clone(),
            sha256: digest.to_owned(),
        });
    }
    if file_bindings.len() != relative_paths.len() {
        bail!("source fingerprint omitted workspace files");
    }
    let digest_for = |relative_path: &str| -> Result<String> {
        digest_by_path
            .iter()
            .find(|(path, _)| path == relative_path)
            .map(|(_, digest)| digest.clone())
            .with_context(|| format!("source fingerprint omitted {relative_path}"))
    };
    let generation_manifest_sha256 = digest_for(GENERATION_MANIFEST)?;
    let obstacle_manifest_sha256 = digest_for(OBSTACLE_MANIFEST)?;
    let fresh_manifest_sha256 = digest_for(WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_FRESH_MANIFEST)?;
    let protocol_sha256 = digest_for(WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_PROTOCOL)?;
    if generation_manifest_sha256 != GENERATION_MANIFEST_SHA256
        || obstacle_manifest_sha256 != OBSTACLE_MANIFEST_SHA256
        || fresh_manifest_sha256 != WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_FRESH_MANIFEST_SHA256
    {
        bail!("a sealed input manifest changed after its digest was declared");
    }
    let mut binding = BodyAwareTerminalSourceBindingV1 {
        identity: String::new(),
        protocol_sha256,
        generation_manifest_sha256,
        obstacle_manifest_sha256,
        fresh_manifest_sha256,
        files: file_bindings,
    };
    binding.identity = source_binding_identity(&binding)?;
    Ok(binding)
}

fn collect_source_files(root: &Path, directory: &Path, paths: &mut BTreeSet<String>) -> Result<()> {
    if fs::symlink_metadata(directory)?.file_type().is_symlink() {
        bail!(
            "refusing symlinked source package path {}",
            directory.display()
        );
    }
    let mut entries = fs::read_dir(directory)
        .with_context(|| format!("reading source directory {}", directory.display()))?
        .map(|entry| entry.map(|item| item.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    for path in entries {
        let file_type = fs::symlink_metadata(&path)?.file_type();
        if file_type.is_symlink() {
            bail!("refusing symlink under source package {}", path.display());
        }
        if file_type.is_dir() {
            if matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("target" | ".git")
            ) {
                continue;
            }
            collect_source_files(root, &path, paths)?;
        } else if file_type.is_file()
            && (path.extension().is_some_and(|extension| extension == "rs")
                || path.file_name().is_some_and(|name| name == "Cargo.toml"))
        {
            paths.insert(path.strip_prefix(root)?.to_string_lossy().into_owned());
        }
    }
    Ok(())
}

fn make_preflight(
    phase: &str,
    source_binding: BodyAwareTerminalSourceBindingV1,
    historical_summaries: Vec<BodyAwareTerminalHistoricalBindingV1>,
    case_order: Vec<String>,
) -> Result<BodyAwareTerminalPreflightV1> {
    let mut preflight = BodyAwareTerminalPreflightV1 {
        schema_id: PREFLIGHT_SCHEMA_ID.to_owned(),
        schema_version: 1,
        phase: phase.to_owned(),
        source_binding,
        historical_summaries,
        case_order,
        identity: String::new(),
    };
    preflight.identity = preflight_identity(&preflight)?;
    Ok(preflight)
}

fn run_cases(
    repo_root: &Path,
    output_root: &Path,
    bound_source: &BodyAwareTerminalSourceBindingV1,
    historical_summaries: &[BodyAwareTerminalHistoricalBindingV1],
    cases: &[BoundCase],
    compare_historical: bool,
) -> Result<CaseRun> {
    let mut rows = Vec::with_capacity(cases.len());
    let mut source_checks = Vec::with_capacity(cases.len());
    for (case_index, case) in cases.iter().enumerate() {
        let before = source_binding(repo_root)?;
        if before != *bound_source {
            source_checks.push(BodyAwareTerminalSourceCheckV1 {
                case_id: case.case_id.clone(),
                before_identity: before.identity.clone(),
                after_identity: before.identity.clone(),
                unchanged_before_case: false,
                unchanged_during_case: false,
            });
            rows.push(not_evaluated_case(
                case,
                "source_inputs_changed_before_case",
            ));
            for remaining in &cases[case_index + 1..] {
                rows.push(not_evaluated_case(
                    remaining,
                    "source_inputs_changed_before_case",
                ));
                source_checks.push(BodyAwareTerminalSourceCheckV1 {
                    case_id: remaining.case_id.clone(),
                    before_identity: before.identity.clone(),
                    after_identity: before.identity.clone(),
                    unchanged_before_case: false,
                    unchanged_during_case: false,
                });
            }
            break;
        }
        let case_dir =
            output_root
                .join("cases")
                .join(format!("{:02}_{}", case_index + 1, case.case_id));
        fs::create_dir(&case_dir)
            .with_context(|| format!("reserving case output {}", case_dir.display()))?;
        let artifact_result = evaluate_waypoint_direct_body_aware_terminal(
            &case.request,
            &BodyAwareTerminalPolicyV1::default(),
        );
        let mut reference = match artifact_result {
            Ok(artifact) => {
                let artifact_path = case_dir.join("summary.json");
                let artifact_bytes = serde_json::to_vec_pretty(&artifact)?;
                write_create_only_bytes(&artifact_path, &artifact_bytes)?;
                let saved_bytes = fs::read(&artifact_path)
                    .with_context(|| format!("reloading {}", artifact_path.display()))?;
                let saved: BodyAwareTerminalCaseArtifactV1 = serde_json::from_slice(&saved_bytes)?;
                let saved_identity = body_aware_terminal_case_identity(&saved)?;
                let request_matches = saved.request == case.request
                    && saved.terminal_policy == BodyAwareTerminalPolicyV1::default()
                    && saved_identity == saved.identity
                    && serde_json::to_vec_pretty(&saved)? == saved_bytes;
                let verification_result = if request_matches {
                    verify_body_aware_terminal_case(&saved)
                } else {
                    Err(anyhow!(
                        "saved case request, policy, identity, or JSON binding mismatch"
                    ))
                };
                let verification_detail = verification_result
                    .as_ref()
                    .err()
                    .map(|error| format!("{error:#}"));
                let verified = verification_result.is_ok();
                let request_identity = semantic_digest(&case.request)?;
                let pin_binding = if compare_historical {
                    case.historical_pin.and_then(|pin| {
                        historical_summaries
                            .iter()
                            .find(|binding| binding.case_id == pin.case_id)
                            .map(|binding| {
                                saved.baseline_generation_sha256 == pin.sha256
                                    && binding.summary_sha256 == pin.sha256
                                    && binding.request_identity == request_identity
                            })
                    }) == Some(true)
                } else {
                    true
                };
                let direct = verified
                    && pin_binding
                    && saved.passed
                    && saved.accepted_witness_count > 0
                    && saved.selected_row_index.is_some();
                let unknown = verified && pin_binding && !saved.passed;
                let status = if direct {
                    "direct"
                } else if unknown {
                    "unknown"
                } else if !verified {
                    "case_verification_failure"
                } else {
                    "historical_baseline_mismatch"
                };
                let baseline_summary_sha256 = case.historical_pin.map(|pin| pin.sha256.to_owned());
                BodyAwareTerminalCaseReferenceV1 {
                    case_id: case.case_id.clone(),
                    previous_direct: case.historical_pin.map(|pin| pin.was_direct),
                    baseline_summary_sha256,
                    baseline_generation_identity: Some(saved.baseline_generation_identity.clone()),
                    baseline_generation_sha256: Some(saved.baseline_generation_sha256.clone()),
                    baseline_sha256_matches_history: case
                        .historical_pin
                        .map(|pin| saved.baseline_generation_sha256 == pin.sha256),
                    accepted_witness_count: if verified {
                        saved.accepted_witness_count
                    } else {
                        0
                    },
                    selected_row_index: if verified {
                        saved.selected_row_index
                    } else {
                        None
                    },
                    case_identity: Some(saved.identity.clone()),
                    case_summary_sha256: Some(sha256_bytes(&saved_bytes)?),
                    relative_path: Some(
                        artifact_path
                            .strip_prefix(output_root)
                            .unwrap_or(&artifact_path)
                            .to_string_lossy()
                            .into_owned(),
                    ),
                    status: status.to_owned(),
                    failure_kind: if status == "direct" || status == "unknown" {
                        None
                    } else if !verified {
                        Some("independent_case_verification_failed".to_owned())
                    } else {
                        Some("baseline_summary_does_not_match_historical_pin".to_owned())
                    },
                    failure_detail: verification_detail,
                }
            }
            Err(error) => engine_error_case(case, compare_historical, format!("{error:#}")),
        };
        let after = source_binding(repo_root)?;
        let unchanged_during_case = after == *bound_source;
        source_checks.push(BodyAwareTerminalSourceCheckV1 {
            case_id: case.case_id.clone(),
            before_identity: before.identity,
            after_identity: after.identity.clone(),
            unchanged_before_case: true,
            unchanged_during_case,
        });
        if !unchanged_during_case {
            reference.status = "source_drift_during_case".to_owned();
            reference.failure_kind = Some("source_inputs_changed_during_case".to_owned());
            reference.failure_detail =
                Some("source fingerprint changed across the case run".to_owned());
        }
        rows.push(reference);
        if !unchanged_during_case {
            for remaining in &cases[case_index + 1..] {
                rows.push(not_evaluated_case(
                    remaining,
                    "source_inputs_changed_after_case",
                ));
                source_checks.push(BodyAwareTerminalSourceCheckV1 {
                    case_id: remaining.case_id.clone(),
                    before_identity: after.identity.clone(),
                    after_identity: after.identity.clone(),
                    unchanged_before_case: false,
                    unchanged_during_case: false,
                });
            }
            break;
        }
    }
    let final_binding = source_binding(repo_root)?;
    if final_binding != *bound_source
        && let Some(last) = source_checks.last_mut()
    {
        last.after_identity = final_binding.identity;
        last.unchanged_during_case = false;
    }
    Ok(CaseRun {
        rows,
        source_checks,
    })
}

fn engine_error_case(
    case: &BoundCase,
    compare_historical: bool,
    detail: String,
) -> BodyAwareTerminalCaseReferenceV1 {
    BodyAwareTerminalCaseReferenceV1 {
        case_id: case.case_id.clone(),
        previous_direct: case.historical_pin.map(|pin| pin.was_direct),
        baseline_summary_sha256: case.historical_pin.map(|pin| pin.sha256.to_owned()),
        baseline_generation_identity: None,
        baseline_generation_sha256: None,
        baseline_sha256_matches_history: compare_historical.then_some(false),
        accepted_witness_count: 0,
        selected_row_index: None,
        case_identity: None,
        case_summary_sha256: None,
        relative_path: None,
        status: "engine_error".to_owned(),
        failure_kind: Some("case_evaluator_returned_error".to_owned()),
        failure_detail: Some(detail),
    }
}

fn not_evaluated_case(case: &BoundCase, reason: &str) -> BodyAwareTerminalCaseReferenceV1 {
    BodyAwareTerminalCaseReferenceV1 {
        case_id: case.case_id.clone(),
        previous_direct: case.historical_pin.map(|pin| pin.was_direct),
        baseline_summary_sha256: case.historical_pin.map(|pin| pin.sha256.to_owned()),
        baseline_generation_identity: None,
        baseline_generation_sha256: None,
        baseline_sha256_matches_history: None,
        accepted_witness_count: 0,
        selected_row_index: None,
        case_identity: None,
        case_summary_sha256: None,
        relative_path: None,
        status: "not_evaluated".to_owned(),
        failure_kind: Some(reason.to_owned()),
        failure_detail: None,
    }
}

fn load_development(
    repo_root: &Path,
    summary_path: &Path,
) -> Result<BodyAwareTerminalDevelopmentArtifactV1> {
    let artifact: BodyAwareTerminalDevelopmentArtifactV1 = read_bound_summary(summary_path)?;
    if artifact.schema_id != DEVELOPMENT_SCHEMA_ID
        || artifact.schema_version != 1
        || artifact.phase != "development"
        || artifact.rows.len() != HISTORICAL_SUMMARY_PINS.len()
        || development_identity(&artifact)? != artifact.identity
    {
        bail!("invalid body-aware terminal development artifact");
    }
    let current = source_binding(repo_root)?;
    if current != artifact.source_binding {
        bail!("source inputs changed since development summary");
    }
    let cases = development_cases(repo_root)?;
    let historical = validate_historical_summaries(repo_root, &cases)?;
    if historical != artifact.historical_summaries {
        bail!("historical inputs changed since development summary");
    }
    validate_development_ledger(&artifact, summary_path, &cases, &historical)?;
    Ok(artifact)
}

fn require_passed_development(passed: bool) -> Result<()> {
    if !passed {
        bail!("development summary did not pass; refusing the next stage");
    }
    Ok(())
}

fn validate_freeze_binding(
    development: &BodyAwareTerminalDevelopmentArtifactV1,
    freeze: &BodyAwareTerminalCodeFreezeV1,
) -> Result<()> {
    if freeze.schema_id != FREEZE_SCHEMA_ID
        || freeze.schema_version != 1
        || freeze.development_identity != development.identity
        || freeze.source_binding != development.source_binding
        || freeze.historical_summaries != development.historical_summaries
        || freeze_identity(freeze)? != freeze.identity
    {
        bail!("code freeze does not bind the accepted development artifact");
    }
    Ok(())
}

fn validate_development_ledger(
    artifact: &BodyAwareTerminalDevelopmentArtifactV1,
    summary_path: &Path,
    cases: &[BoundCase],
    historical: &[BodyAwareTerminalHistoricalBindingV1],
) -> Result<()> {
    if source_binding_identity(&artifact.source_binding)? != artifact.source_binding.identity
        || artifact.rows.len() != cases.len()
        || artifact.source_checks.len() != cases.len()
    {
        bail!("development source or case ledger is incomplete");
    }
    let mut unique_paths = BTreeSet::new();
    let output_root = summary_path
        .parent()
        .context("development summary has no output root")?;
    let canonical_root = fs::canonicalize(output_root)?;
    for (index, ((row, case), history)) in
        artifact.rows.iter().zip(cases).zip(historical).enumerate()
    {
        let expected_path = format!("cases/{:02}_{}/summary.json", index + 1, case.case_id);
        if row.case_id != case.case_id
            || row.relative_path.as_deref() != Some(expected_path.as_str())
            || !unique_paths.insert(expected_path.clone())
            || row.previous_direct != Some(history.previously_direct)
            || history.request_identity != semantic_digest(&case.request)?
            || row.baseline_summary_sha256.as_deref() != Some(history.summary_sha256.as_str())
            || row.baseline_generation_sha256.as_deref() != Some(history.summary_sha256.as_str())
            || row.baseline_sha256_matches_history != Some(true)
            || row.case_summary_sha256.is_none()
            || row.case_identity.is_none()
        {
            bail!(
                "development row {} is not bound to its pinned case and baseline",
                case.case_id
            );
        }
        let source_check = &artifact.source_checks[index];
        if source_check.case_id != case.case_id
            || source_check.before_identity != artifact.source_binding.identity
            || source_check.after_identity != artifact.source_binding.identity
            || !source_check.unchanged_before_case
            || !source_check.unchanged_during_case
        {
            bail!("development source check failed for {}", case.case_id);
        }
        let case_path = output_root.join(&expected_path);
        if fs::symlink_metadata(&case_path)?.file_type().is_symlink()
            || fs::canonicalize(&case_path)? != canonical_root.join(&expected_path)
        {
            bail!("development case artifact path is not a regular in-root file");
        }
        let bytes = fs::read(&case_path)?;
        if sha256_bytes(&bytes)? != row.case_summary_sha256.as_deref().unwrap_or_default() {
            bail!(
                "development case summary digest mismatch for {}",
                case.case_id
            );
        }
        let saved: BodyAwareTerminalCaseArtifactV1 = serde_json::from_slice(&bytes)?;
        if serde_json::to_vec_pretty(&saved)? != bytes
            || saved.request != case.request
            || saved.terminal_policy != BodyAwareTerminalPolicyV1::default()
            || body_aware_terminal_case_identity(&saved)? != saved.identity
            || saved.identity != row.case_identity.as_deref().unwrap_or_default()
            || saved.baseline_generation_sha256 != history.summary_sha256
            || row.baseline_generation_identity.as_deref()
                != Some(saved.baseline_generation_identity.as_str())
            || saved.accepted_witness_count
                != saved
                    .rows
                    .iter()
                    .filter(|candidate| candidate.accepted)
                    .count()
            || saved.passed != (saved.accepted_witness_count > 0)
            || saved.selected_row_index.is_some() != saved.passed
            || saved.rows.iter().any(|candidate| {
                candidate.accepted != candidate.witness.is_some()
                    || candidate.accepted
                        != candidate
                            .witness
                            .as_ref()
                            .is_some_and(|witness| witness.verification.passed)
            })
            || saved.selected_row_index.is_some_and(|selected| {
                !saved
                    .rows
                    .iter()
                    .any(|candidate| candidate.row_index == selected && candidate.accepted)
            })
        {
            bail!(
                "development case artifact semantic binding mismatch for {}",
                case.case_id
            );
        }
        if (row.status == "direct") != saved.passed
            || (row.status == "unknown") != !saved.passed
            || row.accepted_witness_count != saved.accepted_witness_count
            || row.selected_row_index != saved.selected_row_index
        {
            bail!(
                "development row classification disagrees with case artifact for {}",
                case.case_id
            );
        }
    }
    let previously_direct_case_count = historical
        .iter()
        .filter(|entry| entry.previously_direct)
        .count();
    let retained_previous_direct_count = artifact
        .rows
        .iter()
        .filter(|row| row.previous_direct == Some(true) && row.status == "direct")
        .count();
    let high_obstacle_case_count = cases
        .iter()
        .filter(|case| case.case_id.contains("high_obstacle"))
        .count();
    let high_obstacle_direct_count = artifact
        .rows
        .iter()
        .filter(|row| row.case_id.contains("high_obstacle") && row.status == "direct")
        .count();
    let historical_baselines_match = artifact.rows.iter().all(|row| {
        row.baseline_sha256_matches_history == Some(true)
            && row.baseline_generation_sha256.is_some()
    });
    let all_cases_direct = artifact.rows.iter().all(|row| row.status == "direct");
    let all_cases_recorded = artifact.rows.iter().all(|row| {
        row.case_summary_sha256.is_some() && matches!(row.status.as_str(), "direct" | "unknown")
    });
    let passed = historical_baselines_match
        && retained_previous_direct_count == previously_direct_case_count
        && high_obstacle_case_count == 2
        && high_obstacle_direct_count == 2
        && all_cases_direct
        && all_cases_recorded;
    if artifact.previously_direct_case_count != previously_direct_case_count
        || artifact.retained_previous_direct_count != retained_previous_direct_count
        || artifact.high_obstacle_case_count != high_obstacle_case_count
        || artifact.high_obstacle_direct_count != high_obstacle_direct_count
        || artifact.historical_baselines_match != historical_baselines_match
        || artifact.all_cases_direct != all_cases_direct
        || artifact.all_cases_recorded != all_cases_recorded
        || artifact.passed != passed
        || artifact.verdict
            != if passed {
                "development_passed_primary_review_and_source_freeze_required"
            } else {
                "development_failed_stop_before_fresh_physics_without_policy_tuning"
            }
    {
        bail!("development derived gate fields do not match the case ledger");
    }
    Ok(())
}

fn read_bound_summary<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let bytes = fs::read(path).with_context(|| format!("reading summary {}", path.display()))?;
    let value: T = serde_json::from_slice(&bytes)?;
    if serde_json::to_vec_pretty(&value)? != bytes {
        bail!("summary is not canonical pretty JSON: {}", path.display());
    }
    Ok(value)
}

fn reserve_output_root(repo_root: &Path, output_dir: &Path) -> Result<PathBuf> {
    reserve_output_dir(repo_root, output_dir, true)
}

fn reserve_output_dir(repo_root: &Path, output_dir: &Path, with_cases: bool) -> Result<PathBuf> {
    let path = if output_dir.is_absolute() {
        output_dir.to_path_buf()
    } else {
        repo_root.join(output_dir)
    };
    if path.exists() {
        bail!(
            "refusing existing body-aware terminal output root {}",
            path.display()
        );
    }
    let parent = path
        .parent()
        .context("body-aware terminal output path has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating output parent {}", parent.display()))?;
    fs::create_dir(&path)
        .with_context(|| format!("reserving create-only output {}", path.display()))?;
    if with_cases {
        fs::create_dir(path.join("cases"))
            .with_context(|| format!("creating case-output root under {}", path.display()))?;
    }
    Ok(path)
}

fn write_create_only<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    write_create_only_bytes(path, &serde_json::to_vec_pretty(value)?)
}

fn write_create_only_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| {
            format!(
                "creating output file {} without replacement",
                path.display()
            )
        })?;
    file.write_all(bytes)
        .with_context(|| format!("writing output file {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("syncing output file {}", path.display()))?;
    Ok(())
}

fn source_binding_identity(binding: &BodyAwareTerminalSourceBindingV1) -> Result<String> {
    let mut canonical = binding.clone();
    canonical.identity.clear();
    semantic_digest(&canonical)
}

fn preflight_identity(preflight: &BodyAwareTerminalPreflightV1) -> Result<String> {
    let mut canonical = preflight.clone();
    canonical.identity.clear();
    semantic_digest(&canonical)
}

fn development_identity(artifact: &BodyAwareTerminalDevelopmentArtifactV1) -> Result<String> {
    let mut canonical = artifact.clone();
    canonical.identity.clear();
    semantic_digest(&canonical)
}

fn freeze_identity(freeze: &BodyAwareTerminalCodeFreezeV1) -> Result<String> {
    let mut canonical = freeze.clone();
    canonical.identity.clear();
    semantic_digest(&canonical)
}

fn fresh_identity(artifact: &BodyAwareTerminalFreshGateArtifactV1) -> Result<String> {
    let mut canonical = artifact.clone();
    canonical.identity.clear();
    semantic_digest(&canonical)
}

fn semantic_digest<T: Serialize>(value: &T) -> Result<String> {
    let hash = serde_json::to_vec(value)?
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    Ok(format!("fnv1a64:{hash:016x}"))
}

fn scope_non_claims() -> Vec<String> {
    SCOPE_NON_CLAIMS
        .iter()
        .map(|claim| (*claim).to_owned())
        .collect()
}

fn sha256_file(path: &Path) -> Result<String> {
    let output = Command::new("sha256sum")
        .arg("--")
        .arg(path)
        .output()
        .with_context(|| format!("hashing exact file bytes at {}", path.display()))?;
    if !output.status.success() {
        bail!("sha256sum failed for {}", path.display());
    }
    let digest = std::str::from_utf8(&output.stdout)?
        .split_whitespace()
        .next()
        .unwrap_or_default();
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("sha256sum returned a malformed file digest");
    }
    Ok(digest.to_owned())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "pd-body-aware-terminal-{}-{}",
                std::process::id(),
                NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn sha256_bytes_hashes_exact_bytes() {
        assert_eq!(
            sha256_bytes(b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn manifests_bind_fourteen_development_inputs_and_sealed_ten_without_historical_outputs() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let cases = development_cases(root).unwrap();
        let fresh = load_body_aware_fresh_manifest(root).unwrap();
        let source = source_binding(root).unwrap();
        assert_eq!(cases.len(), HISTORICAL_SUMMARY_PINS.len());
        for (case, pin) in cases.iter().zip(HISTORICAL_SUMMARY_PINS) {
            assert_eq!(case.case_id, pin.case_id);
            assert_eq!(case.historical_pin, Some(pin));
        }
        assert_eq!(fresh_cases(&fresh).unwrap().len(), 10);
        assert_eq!(
            source.protocol_sha256,
            sha256_file(&root.join(WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_PROTOCOL)).unwrap()
        );
        assert_eq!(
            source.fresh_manifest_sha256,
            WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_FRESH_MANIFEST_SHA256
        );
    }

    #[test]
    #[ignore = "integration gate requires fourteen retained git-ignored historical summaries"]
    fn historical_preflight_binds_fourteen_inputs_and_sealed_ten_without_writes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let temp = TestDir::new();
        let output = temp.0.join("not-created-by-preflight");
        let preflight = preflight_waypoint_direct_body_aware_terminal_development(root).unwrap();
        let fresh = load_body_aware_fresh_manifest(root).unwrap();
        assert_eq!(preflight.case_order.len(), 14);
        assert_eq!(preflight.historical_summaries.len(), 14);
        assert_eq!(fresh_cases(&fresh).unwrap().len(), 10);
        assert_eq!(
            preflight.source_binding.protocol_sha256,
            sha256_file(&root.join(WAYPOINT_DIRECT_BODY_AWARE_TERMINAL_PROTOCOL)).unwrap()
        );
        assert!(!output.exists());
    }

    #[test]
    fn create_only_output_root_and_file_refuse_replacement() {
        let temp = TestDir::new();
        let root = &temp.0;
        let output = root.join("gate");
        let reserved = reserve_output_dir(root, &output, true).unwrap();
        assert!(reserved.join("cases").is_dir());
        assert!(reserve_output_dir(root, &output, true).is_err());

        let file = reserved.join("summary.json");
        write_create_only_bytes(&file, b"first").unwrap();
        assert!(write_create_only_bytes(&file, b"replacement").is_err());
        assert_eq!(fs::read(file).unwrap(), b"first");
    }

    #[test]
    fn fresh_manifest_order_cannot_be_tampered() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let mut manifest = load_body_aware_fresh_manifest(root).unwrap();
        manifest.cases.swap(0, 1);
        assert!(validate_fresh_cases(&manifest).is_err());
    }

    #[test]
    fn failed_development_and_mismatched_freeze_refuse_the_next_stage() {
        assert!(require_passed_development(false).is_err());
        let binding = BodyAwareTerminalSourceBindingV1 {
            identity: "source".to_owned(),
            protocol_sha256: "protocol".to_owned(),
            generation_manifest_sha256: "generation".to_owned(),
            obstacle_manifest_sha256: "obstacle".to_owned(),
            fresh_manifest_sha256: "fresh".to_owned(),
            files: Vec::new(),
        };
        let development = BodyAwareTerminalDevelopmentArtifactV1 {
            schema_id: DEVELOPMENT_SCHEMA_ID.to_owned(),
            schema_version: 1,
            phase: "development".to_owned(),
            source_binding: binding.clone(),
            historical_summaries: Vec::new(),
            rows: Vec::new(),
            source_checks: Vec::new(),
            previously_direct_case_count: 0,
            retained_previous_direct_count: 0,
            high_obstacle_case_count: 0,
            high_obstacle_direct_count: 0,
            historical_baselines_match: false,
            all_cases_direct: false,
            all_cases_recorded: false,
            passed: true,
            verdict: String::new(),
            scope_non_claims: Vec::new(),
            identity: "development".to_owned(),
        };
        let mut freeze = BodyAwareTerminalCodeFreezeV1 {
            schema_id: FREEZE_SCHEMA_ID.to_owned(),
            schema_version: 1,
            development_identity: "different-development".to_owned(),
            source_binding: binding,
            historical_summaries: Vec::new(),
            identity: String::new(),
        };
        freeze.identity = freeze_identity(&freeze).unwrap();
        assert!(validate_freeze_binding(&development, &freeze).is_err());
    }

    #[test]
    fn canonical_json_loader_rejects_trailing_bytes() {
        let temp = TestDir::new();
        let freeze = BodyAwareTerminalCodeFreezeV1 {
            schema_id: FREEZE_SCHEMA_ID.to_owned(),
            schema_version: 1,
            development_identity: "dev".to_owned(),
            source_binding: BodyAwareTerminalSourceBindingV1 {
                identity: "source".to_owned(),
                protocol_sha256: "protocol".to_owned(),
                generation_manifest_sha256: "generation".to_owned(),
                obstacle_manifest_sha256: "obstacle".to_owned(),
                fresh_manifest_sha256: "fresh".to_owned(),
                files: Vec::new(),
            },
            historical_summaries: Vec::new(),
            identity: "freeze".to_owned(),
        };
        let canonical = serde_json::to_vec_pretty(&freeze).unwrap();
        let path = temp.0.join("freeze.json");
        fs::write(&path, &canonical).unwrap();
        assert!(read_bound_summary::<BodyAwareTerminalCodeFreezeV1>(&path).is_ok());
        let mut changed = canonical;
        changed.push(b'\n');
        fs::write(&path, changed).unwrap();
        assert!(read_bound_summary::<BodyAwareTerminalCodeFreezeV1>(&path).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn source_package_symlinks_are_refused() {
        use std::os::unix::fs::symlink;

        let temp = TestDir::new();
        let package = temp.0.join("pd-eval");
        fs::create_dir(&package).unwrap();
        fs::write(package.join("real.rs"), "pub fn real() {}\n").unwrap();
        symlink(package.join("real.rs"), package.join("alias.rs")).unwrap();
        let mut paths = BTreeSet::new();
        assert!(collect_source_files(&temp.0, &package, &mut paths).is_err());
    }

    #[test]
    fn source_protocol_digest_is_part_of_the_freeze_identity() {
        let mut binding = BodyAwareTerminalSourceBindingV1 {
            identity: String::new(),
            protocol_sha256: "protocol-a".to_owned(),
            generation_manifest_sha256: "generation".to_owned(),
            obstacle_manifest_sha256: "obstacle".to_owned(),
            fresh_manifest_sha256: "fresh".to_owned(),
            files: Vec::new(),
        };
        binding.identity = source_binding_identity(&binding).unwrap();
        let first = binding.identity.clone();
        binding.protocol_sha256 = "protocol-b".to_owned();
        binding.identity = source_binding_identity(&binding).unwrap();
        assert_ne!(first, binding.identity);
    }
}
