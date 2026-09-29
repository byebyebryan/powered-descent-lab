//! Exact-parity regression controls for the opt-in nominal direct flight path.
//!
//! Historical artifacts are comparison references only. Inputs are regenerated
//! from the three checked-in physical manifests before any case is executed.

use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

use crate::{
    BodyAwareTerminalCaseArtifactV1, BodyAwareTerminalPolicyV1, NominalDirectFlightArtifactV1,
    NominalDirectFlightComputeV1, NominalDirectFlightDecisionV1,
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    body_aware_terminal_case_identity, load_waypoint_direct_generation_fresh_manifest,
    load_waypoint_direct_obstacle_discrimination_fresh_manifest, nominal_direct_flight_identity,
    preflight_nominal_direct_flight, run_nominal_direct_flight,
    waypoint_direct_body_aware_terminal::sha256_bytes,
};

pub const DEFAULT_NOMINAL_DIRECT_FLIGHT_ARCHIVE_ROOT: &str =
    "outputs/research/waypoint_direct_body_aware_terminal_20260928";

const BODY_AWARE_FRESH_MANIFEST: &str =
    "fixtures/research/waypoint_direct_body_aware_terminal_fresh_inputs_v1.json";
const INTEGRATION_PROTOCOL: &str = "docs/nominal_direct_flight_integration_protocol.md";
const BODY_AWARE_FRESH_MANIFEST_SHA256: &str =
    "f7b982724cfc8e4df7fd312519a11cfcb1cea25348c7e9451269c8a3e175ddb7";
const DEVELOPMENT_ROOT_SUMMARY: &str = "development_final_a/summary.json";
const DEVELOPMENT_ROOT_SHA256: &str =
    "f828673081ab4bd770eca3646279c719aab7f0cc550a3460e607809ef0022799";
const FRESH_ROOT_SUMMARY: &str = "fresh_run_a/summary.json";
const FRESH_ROOT_SHA256: &str = "60fe4e60737b780dbba61bce583e401df732e9b355b2a93c1891b4ddeeab805d";
const PACKAGE_NAMES: [&str; 6] = [
    "pd-core",
    "pd-plan",
    "pd-control",
    "pd-eval",
    "pd-report",
    "pd-cli",
];
const INPUT_FINGERPRINT_PATHS: [&str; 5] = [
    "fixtures/research/waypoint_direct_generation_fresh_inputs_v1.json",
    "fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json",
    BODY_AWARE_FRESH_MANIFEST,
    "pd-plan/fixtures/conservative_ballistic_direct_bridge_probes_v2.json",
    "fixtures/scenarios/flat_terminal_descent.json",
];
const FRESH_BODY_AWARE_CASE_IDS: [&str; 10] = [
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightSourceFileV1 {
    pub relative_path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightSourceBindingV1 {
    pub files: Vec<NominalDirectFlightSourceFileV1>,
    pub identity_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightArchiveFileCheckV1 {
    pub relative_path: String,
    pub expected_sha256: String,
    pub before_sha256: String,
    pub after_sha256: Option<String>,
    pub unchanged: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightInputFingerprintV1 {
    pub relative_path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightRegressionPreflightV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub ready: bool,
    pub simulation_created: bool,
    pub generation_created: bool,
    pub case_count: usize,
    pub case_ids: Vec<String>,
    pub input_fingerprints: Vec<NominalDirectFlightInputFingerprintV1>,
    pub source_binding: NominalDirectFlightSourceBindingV1,
    pub archive_files: Vec<NominalDirectFlightArchiveFileCheckV1>,
    pub archive_unchanged: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightRegressionSourceCheckV1 {
    pub case_id: String,
    pub before_identity_sha256: Option<String>,
    pub after_identity_sha256: Option<String>,
    pub unchanged_before_case: bool,
    pub unchanged_during_case: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightRegressionChecksV1 {
    pub archive_case_identity_matches: bool,
    pub archive_case_payload_matches: bool,
    pub archive_case_canonical_bytes_match: bool,
    pub selected_row_and_acceptance_count_match: bool,
    pub command_program_matches_selected_witness: bool,
    pub expected_contact_matches_witness: bool,
    pub exact_command_and_clock_parity: bool,
    pub exact_contact_tick_and_fuel_parity: bool,
    pub safe_target_landing: bool,
    pub ordinary_action_replay_parity: bool,
    pub no_authored_waypoints: bool,
    pub archive_unchanged_before_case: bool,
    pub source_unchanged_before_case: bool,
    pub source_unchanged_during_case: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightRegressionCaseV1 {
    pub case_id: String,
    pub relative_bundle_path: String,
    pub status: String,
    pub passed: bool,
    pub archive_case_identity: String,
    pub archive_case_sha256: Option<String>,
    pub regenerated_case_identity: Option<String>,
    pub request_identity: String,
    pub generation_identity: Option<String>,
    pub selected_row_index: Option<usize>,
    pub program_identity: Option<String>,
    pub witness_identity: Option<String>,
    pub ordinary_run_identity: Option<String>,
    pub expected_contact_physics_step: Option<u64>,
    pub observed_contact_physics_step: Option<u64>,
    pub checks: NominalDirectFlightRegressionChecksV1,
    pub compute: Option<NominalDirectFlightComputeV1>,
    pub case_wall_time_us: u64,
    pub failure_detail: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightRegressionComputeV1 {
    pub preflight_wall_time_us: u64,
    pub measured_gate_wall_time_us: u64,
    pub case_wall_time_us: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightRegressionArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub passed: bool,
    pub verdict: String,
    pub identity: String,
    pub source_unchanged: bool,
    pub archive_unchanged: bool,
    pub source_binding_before: NominalDirectFlightSourceBindingV1,
    pub source_binding_after: Option<NominalDirectFlightSourceBindingV1>,
    pub source_checks: Vec<NominalDirectFlightRegressionSourceCheckV1>,
    pub input_fingerprints: Vec<NominalDirectFlightInputFingerprintV1>,
    pub archive_files: Vec<NominalDirectFlightArchiveFileCheckV1>,
    pub cases: Vec<NominalDirectFlightRegressionCaseV1>,
    pub compute: NominalDirectFlightRegressionComputeV1,
}

#[derive(Clone, Debug)]
struct RegressionInputCase {
    case_id: String,
    archive_group: &'static str,
    request: WaypointDirectNominalDirectGenerationRequest,
}

#[derive(Clone, Debug)]
struct ArchiveCasePin {
    case_id: String,
    relative_path: String,
    sha256: String,
    case_identity: String,
    selected_row_index: usize,
    accepted_witness_count: usize,
    request: WaypointDirectNominalDirectGenerationRequest,
}

#[derive(Clone, Debug)]
struct PreparedRegression {
    input_cases: Vec<RegressionInputCase>,
    archive_pins: Vec<ArchiveCasePin>,
    archive_files_before: Vec<NominalDirectFlightArchiveFileCheckV1>,
    input_fingerprints: Vec<NominalDirectFlightInputFingerprintV1>,
    source_binding: NominalDirectFlightSourceBindingV1,
    preflight: NominalDirectFlightRegressionPreflightV1,
}

#[derive(Clone, Debug, Deserialize)]
struct BodyAwareFreshManifestV1 {
    schema_id: String,
    schema_version: u32,
    sealed_before_implementation: bool,
    generation_policy: WaypointDirectNominalDirectGenerationPolicyV1,
    terminal_policy: BodyAwareTerminalPolicyV1,
    fresh_case_order: String,
    cases: Vec<BodyAwareFreshCaseV1>,
}

#[derive(Clone, Debug, Deserialize)]
struct BodyAwareFreshCaseV1 {
    case_id: String,
    source_pad_id: String,
    target_pad_id: String,
    probe_id: String,
    scenario: pd_core::ScenarioSpec,
}

#[derive(Clone, Debug, Deserialize)]
struct ArchiveRootSummaryV1 {
    schema_id: String,
    schema_version: u32,
    phase: String,
    passed: bool,
    all_cases_recorded: bool,
    all_cases_direct: bool,
    rows: Vec<ArchiveRootRowV1>,
}

#[derive(Clone, Debug, Deserialize)]
struct ArchiveRootRowV1 {
    case_id: String,
    case_summary_sha256: String,
    relative_path: String,
    case_identity: String,
    selected_row_index: Option<usize>,
    accepted_witness_count: Option<usize>,
    status: String,
}

/// Validate all current inputs and immutable comparison controls without
/// generating candidates, constructing simulation state, or writing files.
pub fn preflight_nominal_direct_flight_regression(
    repo_root: &Path,
    archive_root: &Path,
) -> Result<NominalDirectFlightRegressionPreflightV1> {
    Ok(prepare_regression(repo_root, archive_root)?.preflight)
}

/// Run all 24 exposed parity controls into a create-only result directory.
/// Each declared case produces a result row, including when generation or
/// execution fails.
pub fn run_nominal_direct_flight_regression(
    repo_root: &Path,
    archive_root: &Path,
    output_dir: &Path,
) -> Result<NominalDirectFlightRegressionArtifactV1> {
    let run_started = Instant::now();
    let preflight_started = Instant::now();
    let prepared = prepare_regression(repo_root, archive_root)?;
    let preflight_wall_time_us = elapsed_us(preflight_started);

    crate::nominal_direct_flight::reserve_output_root(output_dir)?;
    fs::create_dir(output_dir.join("cases"))
        .with_context(|| format!("reserving {}", output_dir.join("cases").display()))?;
    crate::nominal_direct_flight::write_create_only(
        &output_dir.join("preflight.json"),
        &prepared.preflight,
    )?;

    let mut cases = Vec::with_capacity(prepared.input_cases.len());
    let mut source_checks = Vec::with_capacity(prepared.input_cases.len());
    for (index, (input_case, archive_pin)) in prepared
        .input_cases
        .iter()
        .zip(&prepared.archive_pins)
        .enumerate()
    {
        let case_started = Instant::now();
        let relative_bundle_path = format!("cases/{:02}_{}", index + 1, input_case.case_id);
        let case_output = output_dir.join(&relative_bundle_path);
        let before_binding = source_binding(repo_root).ok();
        let unchanged_before_case = before_binding
            .as_ref()
            .is_some_and(|binding| binding == &prepared.source_binding);
        let archive_unchanged_before_case = archive_root_summaries_match(archive_root)
            && archive_case_matches_pin(archive_root, archive_pin);

        let mut case = if unchanged_before_case && archive_unchanged_before_case {
            evaluate_regression_case(
                archive_root,
                input_case,
                archive_pin,
                &case_output,
                relative_bundle_path,
            )
        } else {
            let reason = if !unchanged_before_case {
                "workspace source binding changed before this case"
            } else {
                "archive case changed after preflight"
            };
            let _ = write_skipped_case(&case_output, reason);
            failed_case(
                input_case,
                archive_pin,
                relative_bundle_path,
                reason.to_owned(),
                archive_unchanged_before_case,
            )
        };

        let after_binding = source_binding(repo_root).ok();
        let unchanged_during_case = before_binding.is_some()
            && before_binding == after_binding
            && after_binding.as_ref() == Some(&prepared.source_binding);
        case.checks.source_unchanged_before_case = unchanged_before_case;
        case.checks.source_unchanged_during_case = unchanged_during_case;
        if !unchanged_during_case {
            case.passed = false;
            case.failure_detail.get_or_insert_with(|| {
                "workspace source binding changed during this case".to_owned()
            });
        }
        source_checks.push(NominalDirectFlightRegressionSourceCheckV1 {
            case_id: input_case.case_id.clone(),
            before_identity_sha256: before_binding.map(|binding| binding.identity_sha256),
            after_identity_sha256: after_binding.map(|binding| binding.identity_sha256),
            unchanged_before_case,
            unchanged_during_case,
        });
        case.case_wall_time_us = elapsed_us(case_started);
        eprintln!(
            "nominal-direct-flight-regression {}/{} {}: status={} passed={} elapsed_ms={}",
            index + 1,
            prepared.input_cases.len(),
            case.case_id,
            case.status,
            case.passed,
            case.case_wall_time_us / 1_000
        );
        cases.push(case);
    }

    let archive_files = verify_archive_unchanged(archive_root, &prepared.archive_files_before);
    let archive_unchanged = archive_files.iter().all(|file| file.unchanged);
    let source_binding_after = source_binding(repo_root).ok();
    let source_unchanged = source_binding_after.as_ref() == Some(&prepared.source_binding)
        && source_checks
            .iter()
            .all(|check| check.unchanged_before_case && check.unchanged_during_case);
    let passed = cases.len() == 24
        && cases.iter().all(|case| case.passed)
        && archive_unchanged
        && source_unchanged;
    let mut artifact = NominalDirectFlightRegressionArtifactV1 {
        schema_id: "nominal_direct_flight_regression_v1".to_owned(),
        schema_version: 1,
        passed,
        verdict: if passed {
            "all_24_exposed_controls_exact_parity_passed".to_owned()
        } else {
            "one_or_more_exposed_controls_or_integrity_checks_failed".to_owned()
        },
        identity: String::new(),
        source_unchanged,
        archive_unchanged,
        source_binding_before: prepared.source_binding,
        source_binding_after,
        source_checks,
        input_fingerprints: prepared.input_fingerprints,
        archive_files,
        cases,
        compute: NominalDirectFlightRegressionComputeV1 {
            preflight_wall_time_us,
            measured_gate_wall_time_us: elapsed_us(run_started),
            case_wall_time_us: 0,
        },
    };
    artifact.compute.case_wall_time_us = artifact
        .cases
        .iter()
        .map(|case| case.case_wall_time_us)
        .sum();
    artifact.identity = regression_identity(&artifact)?;
    crate::nominal_direct_flight::write_create_only(&output_dir.join("summary.json"), &artifact)?;
    Ok(artifact)
}

fn prepare_regression(repo_root: &Path, archive_root: &Path) -> Result<PreparedRegression> {
    let source_binding = source_binding(repo_root)?;
    let input_cases = load_regression_inputs(repo_root)?;
    if input_cases.len() != 24 {
        bail!("nominal direct flight regression inputs must contain exactly 24 cases");
    }
    let input_fingerprints = input_fingerprint_hashes(repo_root)?;
    let (archive_pins, archive_files_before) = load_archive_pins(archive_root, &input_cases)?;
    let policy = BodyAwareTerminalPolicyV1::default();
    for case in &input_cases {
        let preflight = preflight_nominal_direct_flight(&case.request, &policy);
        if !preflight.supported || preflight.rejection.is_some() || preflight.simulation_created {
            bail!(
                "regression input {} did not pass typed no-simulation preflight: {:?}",
                case.case_id,
                preflight.rejection
            );
        }
    }
    let preflight = NominalDirectFlightRegressionPreflightV1 {
        schema_id: "nominal_direct_flight_regression_preflight_v1".to_owned(),
        schema_version: 1,
        ready: true,
        simulation_created: false,
        generation_created: false,
        case_count: input_cases.len(),
        case_ids: input_cases
            .iter()
            .map(|case| case.case_id.clone())
            .collect(),
        input_fingerprints: input_fingerprints.clone(),
        source_binding: source_binding.clone(),
        archive_files: archive_files_before.clone(),
        archive_unchanged: archive_files_before.iter().all(|file| file.unchanged),
    };
    Ok(PreparedRegression {
        input_cases,
        archive_pins,
        archive_files_before,
        input_fingerprints,
        source_binding,
        preflight,
    })
}

fn load_regression_inputs(repo_root: &Path) -> Result<Vec<RegressionInputCase>> {
    let generation = load_waypoint_direct_generation_fresh_manifest(repo_root)?;
    let obstacle = load_waypoint_direct_obstacle_discrimination_fresh_manifest(repo_root)?;
    let fresh_path = repo_root.join(BODY_AWARE_FRESH_MANIFEST);
    let fresh_bytes = fs::read(&fresh_path)
        .with_context(|| format!("reading body-aware inputs {}", fresh_path.display()))?;
    if sha256_bytes(&fresh_bytes)? != BODY_AWARE_FRESH_MANIFEST_SHA256 {
        bail!("sealed body-aware input manifest SHA-256 mismatch");
    }
    let fresh: BodyAwareFreshManifestV1 = serde_json::from_slice(&fresh_bytes)?;
    validate_body_aware_fresh_manifest(&fresh)?;

    let mut cases = Vec::with_capacity(24);
    for case in &generation.cases {
        cases.push(RegressionInputCase {
            case_id: case.case_id.clone(),
            archive_group: "development_final_a",
            request: generation.request(&case.case_id)?,
        });
    }
    for case in &obstacle.cases {
        cases.push(RegressionInputCase {
            case_id: case.case_id.clone(),
            archive_group: "development_final_a",
            request: WaypointDirectNominalDirectGenerationRequest {
                scenario: case.scenario.clone(),
                source_pad_id: case.source_pad_id.clone(),
                target_pad_id: case.target_pad_id.clone(),
                probe_id: case.probe_id.clone(),
                policy: obstacle.generation_policy.clone(),
            },
        });
    }
    for case in &fresh.cases {
        cases.push(RegressionInputCase {
            case_id: case.case_id.clone(),
            archive_group: "fresh_run_a",
            request: WaypointDirectNominalDirectGenerationRequest {
                scenario: case.scenario.clone(),
                source_pad_id: case.source_pad_id.clone(),
                target_pad_id: case.target_pad_id.clone(),
                probe_id: case.probe_id.clone(),
                policy: fresh.generation_policy.clone(),
            },
        });
    }
    Ok(cases)
}

fn validate_body_aware_fresh_manifest(manifest: &BodyAwareFreshManifestV1) -> Result<()> {
    if manifest.schema_id != "waypoint_direct_body_aware_terminal_fresh_inputs_v1"
        || manifest.schema_version != 1
        || !manifest.sealed_before_implementation
        || manifest.generation_policy != WaypointDirectNominalDirectGenerationPolicyV1::default()
        || manifest.terminal_policy != BodyAwareTerminalPolicyV1::default()
        || manifest.fresh_case_order
            != "750 then 950 m; flat, uphill +100 m, downhill -100 m, high obstacle, late broad; all Direct required for a passing fresh capability gate"
        || manifest.cases.len() != FRESH_BODY_AWARE_CASE_IDS.len()
    {
        bail!("unsupported or unsealed body-aware fresh input manifest");
    }
    for (case, expected_case_id) in manifest.cases.iter().zip(FRESH_BODY_AWARE_CASE_IDS) {
        if case.case_id != expected_case_id
            || case.probe_id != expected_case_id
            || case.source_pad_id != "pad_source"
            || case.target_pad_id != "pad_main"
        {
            bail!("body-aware fresh input order or pad binding changed");
        }
        case.scenario
            .validate()
            .map_err(|error| anyhow!("invalid fresh input {}: {error}", case.case_id))?;
        if case.scenario.mission.transfer_route.is_some()
            || case.scenario.mission.goal.target_pad_id() != case.target_pad_id
        {
            bail!(
                "body-aware fresh input {} is not route-free landing input",
                case.case_id
            );
        }
    }
    Ok(())
}

fn load_archive_pins(
    archive_root: &Path,
    input_cases: &[RegressionInputCase],
) -> Result<(
    Vec<ArchiveCasePin>,
    Vec<NominalDirectFlightArchiveFileCheckV1>,
)> {
    let mut pins = Vec::with_capacity(24);
    let mut checks = Vec::with_capacity(26);
    let root_specs = [
        (
            "development_final_a",
            DEVELOPMENT_ROOT_SUMMARY,
            DEVELOPMENT_ROOT_SHA256,
            "waypoint_direct_body_aware_terminal_development_v1",
            "development",
            14_usize,
        ),
        (
            "fresh_run_a",
            FRESH_ROOT_SUMMARY,
            FRESH_ROOT_SHA256,
            "waypoint_direct_body_aware_terminal_fresh_gate_v1",
            "fresh",
            10_usize,
        ),
    ];
    for (
        group,
        summary_relative,
        expected_root_sha,
        expected_schema,
        expected_phase,
        expected_count,
    ) in root_specs
    {
        let summary_path = safe_archive_file(archive_root, Path::new(summary_relative))?;
        let summary_bytes = fs::read(&summary_path)
            .with_context(|| format!("reading archived root summary {}", summary_path.display()))?;
        let summary_sha = sha256_bytes(&summary_bytes)?;
        if summary_sha != expected_root_sha {
            bail!("archived root summary digest mismatch: {summary_relative}");
        }
        let summary: ArchiveRootSummaryV1 = serde_json::from_slice(&summary_bytes)?;
        if summary.schema_id != expected_schema
            || summary.schema_version != 1
            || summary.phase != expected_phase
            || !summary.passed
            || !summary.all_cases_recorded
            || !summary.all_cases_direct
            || summary.rows.len() != expected_count
        {
            bail!(
                "archived root summary is not a complete passing Direct control: {summary_relative}"
            );
        }
        checks.push(NominalDirectFlightArchiveFileCheckV1 {
            relative_path: summary_relative.to_owned(),
            expected_sha256: expected_root_sha.to_owned(),
            before_sha256: summary_sha,
            after_sha256: None,
            unchanged: true,
        });

        let expected_cases = input_cases
            .iter()
            .filter(|case| case.archive_group == group)
            .collect::<Vec<_>>();
        if expected_cases.len() != summary.rows.len() {
            bail!("archive root case count differs from its checked-in input group");
        }
        for (input_case, row) in expected_cases.into_iter().zip(summary.rows) {
            if row.case_id != input_case.case_id
                || row.status != "direct"
                || row.selected_row_index.is_none()
                || row.accepted_witness_count.is_none()
                || !is_sha256(&row.case_summary_sha256)
            {
                bail!(
                    "archived case ledger row mismatch for {}",
                    input_case.case_id
                );
            }
            let group_relative =
                Path::new(group).join(validated_relative_path(&row.relative_path)?);
            let case_path = safe_archive_file(archive_root, &group_relative)?;
            let case_bytes = fs::read(&case_path)
                .with_context(|| format!("reading archived case {}", case_path.display()))?;
            let case_sha = sha256_bytes(&case_bytes)?;
            if case_sha != row.case_summary_sha256 {
                bail!("archived case summary digest mismatch for {}", row.case_id);
            }
            let archived: BodyAwareTerminalCaseArtifactV1 = serde_json::from_slice(&case_bytes)
                .with_context(|| format!("parsing archived case {}", row.case_id))?;
            let selected_row_index = row.selected_row_index.expect("validated above");
            let accepted_witness_count = row.accepted_witness_count.expect("validated above");
            if archived.request != input_case.request
                || archived.terminal_policy != BodyAwareTerminalPolicyV1::default()
                || archived.identity != row.case_identity
                || body_aware_terminal_case_identity(&archived)? != archived.identity
                || archived.selected_row_index != Some(selected_row_index)
                || archived.accepted_witness_count != accepted_witness_count
                || !archived.passed
                || archived
                    .rows
                    .get(selected_row_index)
                    .is_none_or(|selected| !selected.accepted || selected.witness.is_none())
            {
                bail!(
                    "archived case payload does not match input/root pins for {}",
                    row.case_id
                );
            }
            checks.push(NominalDirectFlightArchiveFileCheckV1 {
                relative_path: group_relative.to_string_lossy().replace('\\', "/"),
                expected_sha256: row.case_summary_sha256.clone(),
                before_sha256: case_sha,
                after_sha256: None,
                unchanged: true,
            });
            pins.push(ArchiveCasePin {
                case_id: row.case_id,
                relative_path: group_relative.to_string_lossy().replace('\\', "/"),
                sha256: row.case_summary_sha256,
                case_identity: row.case_identity,
                selected_row_index,
                accepted_witness_count,
                request: input_case.request.clone(),
            });
        }
    }
    if pins.len() != 24 {
        bail!("archive pins must contain all 24 input cases");
    }
    Ok((pins, checks))
}

fn evaluate_regression_case(
    archive_root: &Path,
    input_case: &RegressionInputCase,
    archive_pin: &ArchiveCasePin,
    output_dir: &Path,
    relative_bundle_path: String,
) -> NominalDirectFlightRegressionCaseV1 {
    let mut result = failed_case(
        input_case,
        archive_pin,
        relative_bundle_path,
        "case did not complete".to_owned(),
        true,
    );
    if archive_pin.case_id != input_case.case_id {
        result.failure_detail = Some("archive pin is joined to a different input case".to_owned());
        return result;
    }
    let archive_path = match safe_archive_file(archive_root, Path::new(&archive_pin.relative_path))
    {
        Ok(path) => path,
        Err(error) => {
            result.failure_detail = Some(format!("archive path validation failed: {error}"));
            return result;
        }
    };
    let archive_bytes = match fs::read(&archive_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            result.failure_detail = Some(format!("archived case read failed: {error}"));
            return result;
        }
    };
    let archived: BodyAwareTerminalCaseArtifactV1 = match serde_json::from_slice(&archive_bytes) {
        Ok(value) => value,
        Err(error) => {
            result.failure_detail = Some(format!("archived case parse failed: {error}"));
            return result;
        }
    };
    if archived.request != archive_pin.request {
        result.failure_detail = Some("archived request changed after preflight".to_owned());
        return result;
    }
    let archived_sha_matches =
        sha256_bytes(&archive_bytes).is_ok_and(|sha| sha == archive_pin.sha256);
    result.checks.archive_unchanged_before_case = archived_sha_matches;
    if !archived_sha_matches {
        result.failure_detail = Some("archive case changed after preflight".to_owned());
        return result;
    }

    let run_started = Instant::now();
    let run_result = run_nominal_direct_flight(
        &input_case.request,
        &BodyAwareTerminalPolicyV1::default(),
        output_dir,
    );
    result.case_wall_time_us = elapsed_us(run_started);
    let run_artifact = match run_result {
        Ok(artifact) => Some(artifact),
        Err(error) => {
            result.failure_detail = Some(format!("nominal runner failed: {error}"));
            None
        }
    };

    let generation_path = output_dir.join("generation.json");
    let generation_bytes = fs::read(&generation_path).ok();
    let generation = generation_bytes
        .as_deref()
        .and_then(|bytes| serde_json::from_slice::<BodyAwareTerminalCaseArtifactV1>(bytes).ok());
    if let Some(generation) = &generation {
        result.regenerated_case_identity = Some(generation.identity.clone());
        result.generation_identity = Some(generation.baseline_generation_identity.clone());
        result.checks.archive_case_identity_matches =
            generation.identity == archive_pin.case_identity;
        result.checks.archive_case_payload_matches = generation == &archived;
        result.checks.selected_row_and_acceptance_count_match = generation.selected_row_index
            == Some(archive_pin.selected_row_index)
            && generation.accepted_witness_count == archive_pin.accepted_witness_count;
        result.checks.archive_case_canonical_bytes_match =
            generation_bytes.as_deref().is_some_and(|bytes| {
                let generated_pretty = serde_json::to_vec_pretty(generation).ok();
                let archived_pretty = serde_json::to_vec_pretty(&archived).ok();
                generated_pretty == archived_pretty
                    && generated_pretty.as_deref() == Some(strip_one_final_newline(&archive_bytes))
                    && strip_one_final_newline(bytes) == strip_one_final_newline(&archive_bytes)
            });
    } else {
        result
            .failure_detail
            .get_or_insert_with(|| "runner bundle has no parseable generation.json".to_owned());
    }

    let expected_selected = archived
        .rows
        .get(archive_pin.selected_row_index)
        .and_then(|row| row.witness.as_ref());
    if let (Some(program), Some(witness)) = (
        run_artifact
            .as_ref()
            .and_then(|artifact| match &artifact.decision {
                NominalDirectFlightDecisionV1::Direct { program, .. } => Some(program.as_ref()),
                _ => None,
            }),
        expected_selected,
    ) {
        result.status = run_artifact
            .as_ref()
            .map(|artifact| artifact.decision.status().to_owned())
            .unwrap_or_else(|| "error".to_owned());
        if let Some(artifact) = &run_artifact {
            result.request_identity = artifact.request_identity.clone();
            result.program_identity = Some(match &artifact.decision {
                NominalDirectFlightDecisionV1::Direct {
                    program_identity, ..
                } => program_identity.clone(),
                _ => String::new(),
            });
            result.identity_fields_from_artifact(artifact);
        }
        result.witness_identity = Some(witness.identity.clone());
        result.expected_contact_physics_step = Some(witness.verification.physics_ticks_advanced);
        result.checks.command_program_matches_selected_witness = program.updates.len()
            == witness.commands.len()
            && program
                .updates
                .iter()
                .zip(&witness.commands)
                .all(|(update, command)| {
                    update.physics_step.checked_add(1) == Some(command.physics_step)
                        && update.phase == command.phase
                        && update.command == command.command
                });
        result.checks.expected_contact_matches_witness =
            program.expected_contact_physics_step == witness.verification.physics_ticks_advanced;
    } else if let Some(artifact) = &run_artifact {
        result.status = artifact.decision.status().to_owned();
        result.request_identity = artifact.request_identity.clone();
        result.identity_fields_from_artifact(artifact);
        result.compute = Some(artifact.compute.clone());
        result.failure_detail.get_or_insert_with(|| {
            "runner did not produce a Direct decision with its selected witness".to_owned()
        });
    }
    result.checks.no_authored_waypoints =
        input_case.request.scenario.mission.transfer_route.is_none();

    if let Some((artifact, execution)) = run_artifact.as_ref().and_then(|artifact| {
        artifact
            .execution
            .as_ref()
            .map(|execution| (artifact, execution))
    }) {
        result.program_identity = Some(execution.program_identity.clone());
        result.witness_identity = Some(execution.witness_identity.clone());
        result.ordinary_run_identity = Some(execution.ordinary_run_identity.clone());
        result.observed_contact_physics_step = Some(execution.manifest.physics_steps);
        result.checks.exact_command_and_clock_parity = execution.exact_command_and_clock_parity;
        result.checks.exact_contact_tick_and_fuel_parity = execution
            .exact_contact_tick_and_fuel_parity
            && result.expected_contact_physics_step == result.observed_contact_physics_step;
        result.checks.safe_target_landing = execution.safe_target_landing;
        result.checks.ordinary_action_replay_parity = execution.ordinary_action_replay_parity;
        result.compute = Some(artifact.compute.clone());
    }

    let checks = &result.checks;
    result.passed = result.status == "direct"
        && run_artifact
            .as_ref()
            .and_then(|artifact| artifact.execution.as_ref())
            .is_some_and(|execution| execution.passed)
        && checks.archive_case_identity_matches
        && checks.archive_case_payload_matches
        && checks.archive_case_canonical_bytes_match
        && checks.selected_row_and_acceptance_count_match
        && checks.command_program_matches_selected_witness
        && checks.expected_contact_matches_witness
        && checks.exact_command_and_clock_parity
        && checks.exact_contact_tick_and_fuel_parity
        && checks.safe_target_landing
        && checks.ordinary_action_replay_parity
        && checks.no_authored_waypoints
        && checks.archive_unchanged_before_case;
    if !result.passed && result.failure_detail.is_none() {
        result.failure_detail = Some("one or more exact-parity checks failed".to_owned());
    }
    result
}

trait RegressionCaseArtifactFields {
    fn identity_fields_from_artifact(&mut self, artifact: &NominalDirectFlightArtifactV1);
}

impl RegressionCaseArtifactFields for NominalDirectFlightRegressionCaseV1 {
    fn identity_fields_from_artifact(&mut self, artifact: &NominalDirectFlightArtifactV1) {
        if let NominalDirectFlightDecisionV1::Direct {
            generation_identity,
            selected_row_index,
            program_identity,
            ..
        } = &artifact.decision
        {
            self.generation_identity = Some(generation_identity.clone());
            self.selected_row_index = Some(*selected_row_index);
            self.program_identity = Some(program_identity.clone());
        }
        if let Some(execution) = &artifact.execution {
            self.program_identity = Some(execution.program_identity.clone());
            self.witness_identity = Some(execution.witness_identity.clone());
            self.ordinary_run_identity = Some(execution.ordinary_run_identity.clone());
        }
    }
}

fn failed_case(
    input_case: &RegressionInputCase,
    archive_pin: &ArchiveCasePin,
    relative_bundle_path: String,
    detail: String,
    archive_unchanged_before_case: bool,
) -> NominalDirectFlightRegressionCaseV1 {
    NominalDirectFlightRegressionCaseV1 {
        case_id: input_case.case_id.clone(),
        relative_bundle_path,
        status: "error".to_owned(),
        passed: false,
        archive_case_identity: archive_pin.case_identity.clone(),
        archive_case_sha256: Some(archive_pin.sha256.clone()),
        regenerated_case_identity: None,
        request_identity: nominal_direct_flight_identity(&input_case.request).unwrap_or_default(),
        generation_identity: None,
        selected_row_index: Some(archive_pin.selected_row_index),
        program_identity: None,
        witness_identity: None,
        ordinary_run_identity: None,
        expected_contact_physics_step: None,
        observed_contact_physics_step: None,
        checks: NominalDirectFlightRegressionChecksV1 {
            archive_unchanged_before_case,
            ..NominalDirectFlightRegressionChecksV1::default()
        },
        compute: None,
        case_wall_time_us: 0,
        failure_detail: Some(detail),
    }
}

fn write_skipped_case(output_dir: &Path, reason: &str) -> Result<()> {
    fs::create_dir_all(output_dir)?;
    crate::nominal_direct_flight::write_create_only(
        &output_dir.join("regression_error.json"),
        &reason,
    )
}

fn archive_case_matches_pin(archive_root: &Path, pin: &ArchiveCasePin) -> bool {
    safe_archive_file(archive_root, Path::new(&pin.relative_path))
        .and_then(|path| fs::read(path).map_err(Into::into))
        .and_then(|bytes| sha256_bytes(&bytes))
        .is_ok_and(|sha| sha == pin.sha256)
}

fn archive_root_summaries_match(archive_root: &Path) -> bool {
    [
        (DEVELOPMENT_ROOT_SUMMARY, DEVELOPMENT_ROOT_SHA256),
        (FRESH_ROOT_SUMMARY, FRESH_ROOT_SHA256),
    ]
    .into_iter()
    .all(|(relative, expected_sha)| {
        safe_archive_file(archive_root, Path::new(relative))
            .and_then(|path| fs::read(path).map_err(Into::into))
            .and_then(|bytes| sha256_bytes(&bytes))
            .is_ok_and(|sha| sha == expected_sha)
    })
}

fn verify_archive_unchanged(
    archive_root: &Path,
    before: &[NominalDirectFlightArchiveFileCheckV1],
) -> Vec<NominalDirectFlightArchiveFileCheckV1> {
    before
        .iter()
        .map(|file| {
            let after_sha256 = safe_archive_file(archive_root, Path::new(&file.relative_path))
                .and_then(|path| fs::read(path).map_err(Into::into))
                .and_then(|bytes| sha256_bytes(&bytes))
                .ok();
            NominalDirectFlightArchiveFileCheckV1 {
                relative_path: file.relative_path.clone(),
                expected_sha256: file.expected_sha256.clone(),
                before_sha256: file.before_sha256.clone(),
                unchanged: after_sha256.as_deref() == Some(file.expected_sha256.as_str())
                    && file.before_sha256 == file.expected_sha256,
                after_sha256,
            }
        })
        .collect()
}

fn validated_relative_path(path: &str) -> Result<PathBuf> {
    let relative = Path::new(path);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("archive relative path is unsafe: {path}");
    }
    Ok(relative.to_path_buf())
}

fn safe_archive_file(archive_root: &Path, relative: &Path) -> Result<PathBuf> {
    let relative = validated_relative_path(&relative.to_string_lossy())?;
    let root = fs::canonicalize(archive_root)
        .with_context(|| format!("resolving archive root {}", archive_root.display()))?;
    let path = fs::canonicalize(root.join(relative))?;
    if !path.starts_with(&root) || !path.is_file() {
        bail!("archive path resolves outside archive root or is not a file");
    }
    Ok(path)
}

pub(crate) fn source_binding(repo_root: &Path) -> Result<NominalDirectFlightSourceBindingV1> {
    let mut relative_paths = BTreeSet::new();
    for package in PACKAGE_NAMES {
        collect_rust_files(
            repo_root,
            Path::new(package).join("src"),
            &mut relative_paths,
        )?;
        relative_paths.insert(format!("{package}/Cargo.toml"));
    }
    relative_paths.insert("Cargo.toml".to_owned());
    relative_paths.insert("Cargo.lock".to_owned());
    relative_paths.insert(INTEGRATION_PROTOCOL.to_owned());
    for path in INPUT_FINGERPRINT_PATHS {
        relative_paths.insert(path.to_owned());
    }
    let mut files = Vec::with_capacity(relative_paths.len());
    for relative_path in relative_paths {
        let bytes = fs::read(repo_root.join(&relative_path))
            .with_context(|| format!("reading source binding file {relative_path}"))?;
        files.push(NominalDirectFlightSourceFileV1 {
            relative_path,
            sha256: sha256_bytes(&bytes)?,
        });
    }
    let identity_sha256 = sha256_bytes(&serde_json::to_vec(&files)?)?;
    Ok(NominalDirectFlightSourceBindingV1 {
        files,
        identity_sha256,
    })
}

fn collect_rust_files(
    repo_root: &Path,
    relative_directory: PathBuf,
    files: &mut BTreeSet<String>,
) -> Result<()> {
    let directory = repo_root.join(&relative_directory);
    for entry in fs::read_dir(&directory)
        .with_context(|| format!("listing source directory {}", directory.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let relative_path = relative_directory.join(entry.file_name());
        if file_type.is_dir() {
            collect_rust_files(repo_root, relative_path, files)?;
        } else if file_type.is_file()
            && relative_path
                .extension()
                .is_some_and(|extension| extension == "rs")
        {
            files.insert(relative_path.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

fn input_fingerprint_hashes(
    repo_root: &Path,
) -> Result<Vec<NominalDirectFlightInputFingerprintV1>> {
    INPUT_FINGERPRINT_PATHS
        .iter()
        .map(|relative_path| {
            let bytes = fs::read(repo_root.join(relative_path))
                .with_context(|| format!("reading input manifest {relative_path}"))?;
            Ok(NominalDirectFlightInputFingerprintV1 {
                relative_path: (*relative_path).to_owned(),
                sha256: sha256_bytes(&bytes)?,
            })
        })
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn strip_one_final_newline(bytes: &[u8]) -> &[u8] {
    bytes.strip_suffix(b"\n").unwrap_or(bytes)
}

fn regression_identity(artifact: &NominalDirectFlightRegressionArtifactV1) -> Result<String> {
    let mut canonical = artifact.clone();
    canonical.identity.clear();
    canonical.compute.preflight_wall_time_us = 0;
    canonical.compute.measured_gate_wall_time_us = 0;
    canonical.compute.case_wall_time_us = 0;
    canonical.archive_files.clear();
    for case in &mut canonical.cases {
        case.archive_case_sha256 = None;
        case.case_wall_time_us = 0;
        case.failure_detail = None;
        if let Some(compute) = &mut case.compute {
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
    use std::path::Path;

    use super::*;

    #[test]
    fn input_family_uses_exact_24_case_order_and_default_policies() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let cases = load_regression_inputs(repo_root).unwrap();
        let expected = [
            "fresh_flat_span_600_delta_000",
            "fresh_uphill_span_600_delta_p120",
            "fresh_downhill_span_600_delta_m120",
            "fresh_flat_span_1000_delta_000",
            "fresh_uphill_span_1000_delta_p120",
            "fresh_downhill_span_1000_delta_m120",
            "fresh_flat_control_span_700",
            "fresh_low_obstacle_span_700",
            "fresh_high_obstacle_span_700",
            "fresh_late_broad_span_700",
            "fresh_flat_control_span_900",
            "fresh_low_obstacle_span_900",
            "fresh_high_obstacle_span_900",
            "fresh_late_broad_span_900",
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
        assert_eq!(cases.len(), expected.len());
        assert_eq!(
            cases
                .iter()
                .map(|case| case.case_id.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(cases.iter().all(|case| {
            case.request.policy == WaypointDirectNominalDirectGenerationPolicyV1::default()
                && case.request.scenario.mission.transfer_route.is_none()
        }));
    }

    #[test]
    fn archive_paths_reject_absolute_and_parent_components() {
        assert!(validated_relative_path("/tmp/archive/case.json").is_err());
        assert!(validated_relative_path("../outside/case.json").is_err());
        assert!(validated_relative_path("cases/../../outside.json").is_err());
        assert!(validated_relative_path("cases/01_case/summary.json").is_ok());
    }

    #[test]
    fn regression_identity_excludes_observational_times_and_raw_archive_digests() {
        let mut artifact = NominalDirectFlightRegressionArtifactV1 {
            schema_id: "nominal_direct_flight_regression_v1".to_owned(),
            schema_version: 1,
            passed: false,
            verdict: "test".to_owned(),
            identity: String::new(),
            source_unchanged: true,
            archive_unchanged: true,
            source_binding_before: NominalDirectFlightSourceBindingV1 {
                files: Vec::new(),
                identity_sha256: "source".to_owned(),
            },
            source_binding_after: None,
            source_checks: Vec::new(),
            input_fingerprints: Vec::new(),
            archive_files: vec![NominalDirectFlightArchiveFileCheckV1 {
                relative_path: "fresh_run_a/summary.json".to_owned(),
                expected_sha256: "pin-a".to_owned(),
                before_sha256: "pin-a".to_owned(),
                after_sha256: Some("pin-a".to_owned()),
                unchanged: true,
            }],
            cases: Vec::new(),
            compute: NominalDirectFlightRegressionComputeV1::default(),
        };
        artifact.identity = regression_identity(&artifact).unwrap();
        let baseline_identity = artifact.identity.clone();
        artifact.compute.measured_gate_wall_time_us = 1234;
        artifact.archive_files[0].before_sha256 = "different-raw-hash".to_owned();
        artifact.archive_files[0].after_sha256 = Some("different-raw-hash".to_owned());
        assert_eq!(regression_identity(&artifact).unwrap(), baseline_identity);
    }

    #[test]
    fn fresh_manifest_contract_is_independently_checked_after_its_hash_pin() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let bytes = std::fs::read(repo_root.join(BODY_AWARE_FRESH_MANIFEST)).unwrap();
        assert_eq!(
            sha256_bytes(&bytes).unwrap(),
            BODY_AWARE_FRESH_MANIFEST_SHA256
        );
        let manifest: BodyAwareFreshManifestV1 = serde_json::from_slice(&bytes).unwrap();
        validate_body_aware_fresh_manifest(&manifest).unwrap();
    }
}
