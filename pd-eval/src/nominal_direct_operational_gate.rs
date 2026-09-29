//! Fixed 24 exposed + four presealed fresh nominal operational controls. No
//! outcome replacement, stored-program seeding, or disturbance sweep.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, time::Instant};

use crate::{
    BodyAwareTerminalCaseArtifactV1, BodyAwareTerminalPolicyV1, NominalComparisonV1,
    NominalDirectFlightSourceBindingV1, NominalDirectFlightSourceFileV1,
    NominalDirectOperationalOutcomeV1, WaypointDirectNominalDirectGenerationPolicyV1,
    WaypointDirectNominalDirectGenerationRequest,
    nominal_direct_flight::{reserve_output_root, write_create_only},
    nominal_direct_flight_gate::{operational_regression_controls, source_binding},
    nominal_direct_flight_identity,
    nominal_direct_operational::{NOMINAL_DIRECT_OPERATIONAL_PROTOCOL, write_text_create_only},
    preflight_nominal_direct_flight, run_nominal_direct_operational_flight,
    waypoint_direct_body_aware_terminal::sha256_bytes,
};

pub const NOMINAL_DIRECT_OPERATIONAL_FRESH_MANIFEST: &str =
    "fixtures/research/nominal_direct_operational_fresh_inputs_v1.json";
pub const NOMINAL_DIRECT_OPERATIONAL_FRESH_SHA256: &str =
    "6df19fbd3ebb85d5a7f4a783398aaaa48e50c74a9de49454161b78c8e47658f8";
const CONTRACT: &str = "docs/nominal_direct_execution_completion_contract.md";
const PROTOCOL_SHA256: &str = "397da050a60d6a97fe0a0067904846e672f380ae458fd49550401f8667b2d202";
const ARCHIVE_ROOTS: [&str; 3] = [
    "outputs/research/nominal_direct_flight_integration_20260928",
    "outputs/research/waypoint_direct_body_aware_terminal_20260928",
    "outputs/research/nominal_direct_contact_phase_20260928",
];
const FRESH_CASES: [&str; 4] = [
    "operational_flat_span_685",
    "operational_flat_span_845",
    "operational_uphill_span_845",
    "operational_downhill_span_845",
];

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FreshManifest {
    schema_id: String,
    schema_version: u32,
    sealed_before_implementation: bool,
    execution_policy_identity: String,
    case_order: String,
    generation_policy: WaypointDirectNominalDirectGenerationPolicyV1,
    terminal_policy: BodyAwareTerminalPolicyV1,
    cases: Vec<FreshCase>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FreshCase {
    case_id: String,
    horizontal_span_m: f64,
    profile: String,
    height_change_m: f64,
    source_pad_id: String,
    target_pad_id: String,
    probe_id: String,
    scenario: pd_core::ScenarioSpec,
}

pub fn load_nominal_direct_operational_fresh_inputs(
    repo: &Path,
) -> Result<Vec<(String, WaypointDirectNominalDirectGenerationRequest)>> {
    let bytes = fs::read(repo.join(NOMINAL_DIRECT_OPERATIONAL_FRESH_MANIFEST))?;
    if sha256_bytes(&bytes)? != NOMINAL_DIRECT_OPERATIONAL_FRESH_SHA256 {
        bail!("sealed operational physical-input manifest SHA-256 mismatch");
    }
    let manifest: FreshManifest = serde_json::from_slice(&bytes)?;
    if manifest.schema_id != "nominal_direct_operational_fresh_inputs_v1"
        || manifest.schema_version != 1
        || !manifest.sealed_before_implementation
        || manifest.execution_policy_identity != "strict_saved_coverage_v1"
        || manifest.case_order != "685-flat,845-flat,845-uphill-75,845-downhill-75"
        || manifest.generation_policy != WaypointDirectNominalDirectGenerationPolicyV1::default()
        || manifest.terminal_policy != BodyAwareTerminalPolicyV1::default()
        || manifest.cases.len() != 4
    {
        bail!("unsupported operational input seal");
    }
    let mut result = Vec::new();
    for (index, (case, expected)) in manifest.cases.into_iter().zip(FRESH_CASES).enumerate() {
        let expected_profile = ["flat", "flat", "uphill", "downhill"][index];
        if case.case_id != expected
            || case.probe_id != expected
            || case.source_pad_id != "pad_source"
            || case.target_pad_id != "pad_main"
            || case.horizontal_span_m != [685.0, 845.0, 845.0, 845.0][index]
            || case.height_change_m != [0.0, 0.0, 75.0, -75.0][index]
            || case.profile != expected_profile
            || case.scenario.mission.transfer_route.is_some()
        {
            bail!("operational fresh order/physical binding changed");
        }
        let request = WaypointDirectNominalDirectGenerationRequest {
            probe_id: case.probe_id,
            source_pad_id: case.source_pad_id,
            target_pad_id: case.target_pad_id,
            scenario: case.scenario,
            policy: manifest.generation_policy.clone(),
        };
        if !preflight_nominal_direct_flight(&request, &manifest.terminal_policy).supported {
            bail!("sealed operational input is unsupported: {}", case.case_id);
        }
        result.push((case.case_id, request));
    }
    Ok(result)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalDirectOperationalPreflightV1 {
    pub ready: bool,
    pub simulation_created: bool,
    pub generation_created: bool,
    pub case_ids: Vec<String>,
    pub source_binding: NominalDirectFlightSourceBindingV1,
    pub archive_inventory: Vec<NominalDirectFlightSourceFileV1>,
    pub fresh_manifest_sha256: String,
}

struct GateInput {
    case_id: String,
    fresh: bool,
    request: WaypointDirectNominalDirectGenerationRequest,
    comparison: Option<BodyAwareTerminalCaseArtifactV1>,
}

fn prepare(repo: &Path) -> Result<(NominalDirectOperationalPreflightV1, Vec<GateInput>)> {
    if sha256_bytes(&fs::read(repo.join(NOMINAL_DIRECT_OPERATIONAL_PROTOCOL))?)? != PROTOCOL_SHA256
    {
        bail!("operational protocol differs from the pre-implementation seal");
    }
    let mut inputs: Vec<_> = operational_regression_controls(repo, &repo.join(ARCHIVE_ROOTS[1]))?
        .into_iter()
        .map(|(case_id, request, comparison)| GateInput {
            case_id,
            request,
            comparison: Some(comparison),
            fresh: false,
        })
        .collect();
    for (case_id, request) in load_nominal_direct_operational_fresh_inputs(repo)? {
        inputs.push(GateInput {
            case_id,
            request,
            comparison: None,
            fresh: true,
        });
    }
    if inputs.len() != 28
        || inputs.iter().any(|input| {
            !preflight_nominal_direct_flight(&input.request, &BodyAwareTerminalPolicyV1::default())
                .supported
                || input.case_id.is_empty()
                || !input
                    .case_id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
    {
        bail!("operational gate requires all 28 fixed supported inputs");
    }
    Ok((
        NominalDirectOperationalPreflightV1 {
            ready: true,
            simulation_created: false,
            generation_created: false,
            case_ids: inputs.iter().map(|input| input.case_id.clone()).collect(),
            source_binding: operational_source_binding(repo)?,
            archive_inventory: operational_archive_inventory(repo)?,
            fresh_manifest_sha256: NOMINAL_DIRECT_OPERATIONAL_FRESH_SHA256.into(),
        },
        inputs,
    ))
}

pub fn preflight_nominal_direct_operational_gate(
    repo: &Path,
) -> Result<NominalDirectOperationalPreflightV1> {
    Ok(prepare(repo)?.0)
}

pub(crate) fn operational_source_binding(
    repo: &Path,
) -> Result<NominalDirectFlightSourceBindingV1> {
    let mut binding = source_binding(repo)?;
    for relative_path in [
        NOMINAL_DIRECT_OPERATIONAL_FRESH_MANIFEST,
        NOMINAL_DIRECT_OPERATIONAL_PROTOCOL,
        CONTRACT,
    ] {
        let sha256 = sha256_bytes(&fs::read(repo.join(relative_path))?)?;
        binding.files.push(NominalDirectFlightSourceFileV1 {
            relative_path: relative_path.into(),
            sha256,
        });
    }
    binding
        .files
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    binding.identity_sha256 = sha256_bytes(&serde_json::to_vec(&binding.files)?)?;
    Ok(binding)
}

pub(crate) fn operational_archive_inventory(
    repo: &Path,
) -> Result<Vec<NominalDirectFlightSourceFileV1>> {
    fn visit(
        repo: &Path,
        relative: &Path,
        files: &mut Vec<NominalDirectFlightSourceFileV1>,
    ) -> Result<()> {
        for entry in fs::read_dir(repo.join(relative))? {
            let entry = entry?;
            let relative = relative.join(entry.file_name());
            let kind = entry.file_type()?;
            if kind.is_dir() {
                visit(repo, &relative, files)?;
            } else if kind.is_file() {
                files.push(NominalDirectFlightSourceFileV1 {
                    relative_path: relative.to_string_lossy().into_owned(),
                    sha256: sha256_bytes(&fs::read(repo.join(relative))?)?,
                });
            } else {
                bail!("non-regular archived evidence: {}", relative.display());
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    for root in ARCHIVE_ROOTS {
        visit(repo, Path::new(root), &mut files)?;
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalDirectOperationalGateCaseV1 {
    pub case_id: String,
    pub fresh: bool,
    pub status: String,
    pub artifact_identity: Option<String>,
    pub program_identity: Option<String>,
    pub observed_contact_physics_step: Option<u64>,
    pub operational_outcome: Option<NominalDirectOperationalOutcomeV1>,
    pub nominal_comparison: Option<NominalComparisonV1>,
    pub exact_exposed_generation_payload: Option<bool>,
    pub source_unchanged_before_case: bool,
    pub source_unchanged_during_case: bool,
    pub passed: bool,
    pub failure_detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalDirectOperationalGateArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub preflight: NominalDirectOperationalPreflightV1,
    pub source_binding_after: NominalDirectFlightSourceBindingV1,
    pub archive_inventory_after: Vec<NominalDirectFlightSourceFileV1>,
    pub source_unchanged: bool,
    pub archive_unchanged: bool,
    pub all_cases_recorded: bool,
    pub exposed_exact_safe_match_count: usize,
    pub fresh_direct_safe_match_count: usize,
    pub cases: Vec<NominalDirectOperationalGateCaseV1>,
    pub passed: bool,
    pub verdict: String,
    pub measured_wall_time_us: u64,
    pub identity: String,
}

pub fn run_nominal_direct_operational_gate(
    repo: &Path,
    output: &Path,
) -> Result<NominalDirectOperationalGateArtifactV1> {
    let (preflight, inputs) = prepare(repo)?;
    reserve_output_root(output)?;
    write_create_only(&output.join("preflight.json"), &preflight)?;
    let started = Instant::now();
    let mut cases = Vec::new();
    for input in inputs {
        let mut row = NominalDirectOperationalGateCaseV1 {
            case_id: input.case_id.clone(),
            fresh: input.fresh,
            status: "not_run".into(),
            artifact_identity: None,
            program_identity: None,
            observed_contact_physics_step: None,
            operational_outcome: None,
            nominal_comparison: None,
            exact_exposed_generation_payload: None,
            source_unchanged_before_case: operational_source_binding(repo)
                .is_ok_and(|value| value == preflight.source_binding),
            source_unchanged_during_case: false,
            passed: false,
            failure_detail: None,
        };
        let case_path = output.join("cases").join(&input.case_id);
        if row.source_unchanged_before_case {
            match run_nominal_direct_operational_flight(
                &input.request,
                &BodyAwareTerminalPolicyV1::default(),
                &case_path,
            ) {
                Ok(artifact) => {
                    row.status = artifact.decision.status().into();
                    row.artifact_identity = Some(artifact.identity);
                    row.failure_detail = artifact.admission_error;
                    if let Some(execution) = artifact.execution {
                        row.program_identity = Some(execution.program_identity);
                        row.observed_contact_physics_step = execution
                            .validity
                            .first_contact
                            .as_ref()
                            .map(|contact| contact.state.physics_step);
                        row.operational_outcome = Some(execution.operational_outcome);
                        row.nominal_comparison = Some(execution.nominal_comparison);
                    }
                    if let Some(expected) = &input.comparison {
                        let comparison = (|| -> Result<bool> {
                            let bytes = fs::read(case_path.join("generation.json"))?;
                            let actual: BodyAwareTerminalCaseArtifactV1 =
                                serde_json::from_slice(&bytes)?;
                            Ok(actual == *expected
                                && bytes.strip_suffix(b"\n").unwrap_or(&bytes)
                                    == serde_json::to_vec_pretty(expected)?)
                        })();
                        match comparison {
                            Ok(matches) => row.exact_exposed_generation_payload = Some(matches),
                            Err(error) => {
                                row.exact_exposed_generation_payload = Some(false);
                                row.failure_detail = Some(format!(
                                    "exposed generation comparison failed: {error:#}"
                                ));
                            }
                        }
                    }
                }
                Err(error) => {
                    row.status = "error".into();
                    row.failure_detail = Some(format!("{error:#}"));
                }
            }
        } else {
            row.failure_detail = Some("source/protocol changed before case; no execution".into());
        }
        row.source_unchanged_during_case =
            operational_source_binding(repo).is_ok_and(|value| value == preflight.source_binding);
        row.passed = row.status == "direct"
            && row.operational_outcome
                == Some(NominalDirectOperationalOutcomeV1::CompletedSafeTarget)
            && row.nominal_comparison == Some(NominalComparisonV1::Match)
            && row.exact_exposed_generation_payload != Some(false)
            && row.source_unchanged_before_case
            && row.source_unchanged_during_case;
        write_create_only(&output.join(format!("case_{}.json", input.case_id)), &row)?;
        cases.push(row);
    }
    let source_binding_after = operational_source_binding(repo)?;
    let archive_inventory_after = operational_archive_inventory(repo)?;
    let source_unchanged = source_binding_after == preflight.source_binding;
    let archive_unchanged = archive_inventory_after == preflight.archive_inventory;
    let all_cases_recorded = cases.len() == 28;
    let exposed_exact_safe_match_count = cases
        .iter()
        .filter(|case| !case.fresh && case.passed)
        .count();
    let fresh_direct_safe_match_count = cases
        .iter()
        .filter(|case| case.fresh && case.passed)
        .count();
    let passed = all_cases_recorded
        && exposed_exact_safe_match_count == 24
        && fresh_direct_safe_match_count == 4
        && source_unchanged
        && archive_unchanged;
    let mut artifact = NominalDirectOperationalGateArtifactV1 {
        schema_id: "nominal_direct_operational_gate_v1".into(), schema_version: 1, preflight,
        source_binding_after, archive_inventory_after, source_unchanged, archive_unchanged,
        all_cases_recorded, exposed_exact_safe_match_count, fresh_direct_safe_match_count, cases, passed,
        verdict: if passed { "24 exposed exact controls and 4 sealed fresh nominal cases: Direct, completed-safe, Match" }
            else { "bounded operational acceptance/finite coverage gap; inspect every case; no replacement or policy tuning" }.into(),
        measured_wall_time_us: started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
        identity: String::new(),
    };
    let mut canonical = artifact.clone();
    canonical.measured_wall_time_us = 0;
    artifact.identity = nominal_direct_flight_identity(&canonical)?;
    write_create_only(&output.join("summary.json"), &artifact)?;
    let rows = artifact.cases.iter().map(|case| format!(
        "<tr><td><a href='cases/{}/report.html'>{}</a></td><td>{}</td><td>{:?}</td><td>{:?}</td><td>{}</td></tr>",
        case.case_id, case.case_id, case.status, case.operational_outcome, case.nominal_comparison, case.passed,
    )).collect::<String>();
    write_text_create_only(
        &output.join("report.html"),
        &format!(
            "<!doctype html><meta charset='utf-8'><title>Nominal operational gate</title><h1>Strict saved-coverage operational gate V1</h1><p>Exposed exact safe Match: {}/24; sealed fresh: {}/4.</p><p>No continuation, disturbance tolerance, default promotion, or waypoint demand is inferred.</p><table><tr><th>Case</th><th>Decision</th><th>Outcome</th><th>Nominal</th><th>Passed</th></tr>{rows}</table><p><a href='summary.json'>Full gate and provenance</a></p>",
            artifact.exposed_exact_safe_match_count, artifact.fresh_direct_safe_match_count,
        ),
    )?;
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sealed_four_inputs_are_supported_without_generation_or_simulation() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let inputs = load_nominal_direct_operational_fresh_inputs(repo).unwrap();
        assert_eq!(
            inputs
                .iter()
                .map(|input| input.0.as_str())
                .collect::<Vec<_>>(),
            FRESH_CASES
        );
        for (_, request) in inputs {
            let result =
                preflight_nominal_direct_flight(&request, &BodyAwareTerminalPolicyV1::default());
            assert!(result.supported && !result.simulation_created);
        }
    }
    #[test]
    fn source_closure_contains_new_runtime_inputs_and_contract() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let binding = operational_source_binding(repo).unwrap();
        for path in [
            NOMINAL_DIRECT_OPERATIONAL_FRESH_MANIFEST,
            NOMINAL_DIRECT_OPERATIONAL_PROTOCOL,
            CONTRACT,
            "pd-eval/src/nominal_direct_operational.rs",
        ] {
            assert!(binding.files.iter().any(|file| file.relative_path == path));
        }
    }
}
