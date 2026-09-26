//! Input adapters and post-generation gates for the opt-in direct generator.
//! Historical research artifacts are comparison references, never generator inputs.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    WaypointDirectCompleteFlatAcceptanceArtifact, WaypointDirectNominalDirectGenerationArtifact,
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    WaypointDirectSourceDurationPairedCommandFeasibilityArtifact,
};

pub const DIRECT_GENERATION_FRESH_MANIFEST: &str =
    "fixtures/research/waypoint_direct_generation_fresh_inputs_v1.json";
pub const DIRECT_GENERATION_FRESH_MANIFEST_SHA256: &str =
    "3dffd6bc3f220629b96e817af8e0be7f6cb2e905c75a1df796e97aa15b066d24";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointDirectGenerationFreshManifest {
    pub schema_id: String,
    pub schema_version: u32,
    pub sealed_before_implementation: bool,
    pub generation_policy: WaypointDirectNominalDirectGenerationPolicyV1,
    pub fresh_case_order: String,
    pub cases: Vec<WaypointDirectGenerationFreshCase>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointDirectGenerationFreshCase {
    pub case_id: String,
    pub horizontal_span_m: f64,
    pub target_minus_source_height_m: f64,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub probe_id: String,
    pub scenario: pd_core::ScenarioSpec,
}

/// Pure input loading. This neither constructs a plant nor evaluates a candidate.
pub fn load_waypoint_direct_generation_fresh_manifest(
    repo_root: &Path,
) -> Result<WaypointDirectGenerationFreshManifest> {
    let path = repo_root.join(DIRECT_GENERATION_FRESH_MANIFEST);
    verify_file_sha256(&path, DIRECT_GENERATION_FRESH_MANIFEST_SHA256)?;
    let raw = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let manifest: WaypointDirectGenerationFreshManifest = serde_json::from_slice(&raw)?;
    if manifest.schema_id != "waypoint_direct_generation_fresh_inputs_v1"
        || manifest.schema_version != 1
        || !manifest.sealed_before_implementation
        || manifest.cases.len() != 6
        || manifest.generation_policy != WaypointDirectNominalDirectGenerationPolicyV1::default()
    {
        bail!("unsupported sealed direct-generation manifest");
    }
    let expected = [
        ("fresh_flat_span_600_delta_000", 600.0, 0.0),
        ("fresh_uphill_span_600_delta_p120", 600.0, 120.0),
        ("fresh_downhill_span_600_delta_m120", 600.0, -120.0),
        ("fresh_flat_span_1000_delta_000", 1000.0, 0.0),
        ("fresh_uphill_span_1000_delta_p120", 1000.0, 120.0),
        ("fresh_downhill_span_1000_delta_m120", 1000.0, -120.0),
    ];
    for (case, (id, span, height)) in manifest.cases.iter().zip(expected) {
        if case.case_id != id
            || case.probe_id != id
            || case.horizontal_span_m != span
            || case.target_minus_source_height_m != height
            || case.source_pad_id != "pad_source"
            || case.target_pad_id != "pad_main"
            || case.scenario.mission.transfer_route.is_some()
        {
            bail!("sealed direct-generation case order or input contract changed");
        }
    }
    Ok(manifest)
}

fn verify_file_sha256(path: &Path, expected: &str) -> Result<()> {
    let output = Command::new("sha256sum")
        .arg("--")
        .arg(path)
        .output()
        .with_context(|| format!("checking sealed digest of {}", path.display()))?;
    let digest = std::str::from_utf8(&output.stdout)?
        .split_whitespace()
        .next()
        .unwrap_or_default();
    if !output.status.success() || digest != expected {
        bail!("sealed artifact SHA-256 mismatch: {}", path.display());
    }
    Ok(())
}

impl WaypointDirectGenerationFreshManifest {
    pub fn request(&self, case_id: &str) -> Result<WaypointDirectNominalDirectGenerationRequest> {
        let case = self
            .cases
            .iter()
            .find(|case| case.case_id == case_id)
            .with_context(|| format!("unknown sealed direct-generation case {case_id}"))?;
        Ok(WaypointDirectNominalDirectGenerationRequest {
            scenario: case.scenario.clone(),
            source_pad_id: case.source_pad_id.clone(),
            target_pad_id: case.target_pad_id.clone(),
            probe_id: case.probe_id.clone(),
            policy: self.generation_policy.clone(),
        })
    }
}

/// The historical flat factory supplies only a scenario, not research outcomes.
pub fn waypoint_direct_known_flat_generation_request(
    repo_root: &Path,
) -> Result<WaypointDirectNominalDirectGenerationRequest> {
    Ok(WaypointDirectNominalDirectGenerationRequest {
        scenario: crate::waypoint_direct_characterization::controller_scenario_for_case(
            repo_root,
            "continuous_flat_r00",
        )?,
        source_pad_id: "pad_source".to_owned(),
        target_pad_id: "pad_main".to_owned(),
        probe_id: "continuous_flat_r00".to_owned(),
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaypointDirectGenerationParityRow {
    pub basis_candidate_identity: String,
    pub duration_offset_ticks: i64,
    pub analytical_survivor_matches: bool,
    pub complete_paired_schedule_matches: bool,
    pub complete_physical_acceptance_matches: bool,
    pub passed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaypointDirectGenerationGateA {
    pub schema_id: String,
    pub generated_identity: String,
    pub historical_paired_identity: String,
    pub historical_acceptance_identity: String,
    pub rows: Vec<WaypointDirectGenerationParityRow>,
    pub generated_row_count: usize,
    pub scheduled_count: usize,
    pub accepted_count: usize,
    pub extra_rows_are_five_unflown_rejected_basis_skips: bool,
    pub selected_physical_witness_matches: bool,
    pub passed: bool,
    pub identity: String,
}

/// Called only after the generator has produced its independent artifact.
/// This deliberately cannot construct generator inputs from historical evidence.
pub fn compare_waypoint_direct_generation_gate_a(
    generated_summary: &Path,
    paired_summary: &Path,
    acceptance_summary: &Path,
    output_dir: &Path,
) -> Result<WaypointDirectGenerationGateA> {
    // Read the newly generated artifact before opening either reference.
    let generated: WaypointDirectNominalDirectGenerationArtifact =
        serde_json::from_slice(&fs::read(generated_summary)?)?;
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pd-eval belongs to workspace root");
    if generated.request != waypoint_direct_known_flat_generation_request(repo_root)? {
        bail!("Gate A requires the exact pure known-flat input and sealed policy");
    }
    let mut identity_input = generated.clone();
    identity_input.identity.clear();
    if semantic_digest(&identity_input)? != generated.identity {
        bail!("generated artifact identity or serialized round trip changed");
    }
    verify_file_sha256(
        paired_summary,
        "6477a947537208872056d46e82c7b6fc0c7aa6801f14ca9ccb8b340f4da86247",
    )?;
    verify_file_sha256(
        acceptance_summary,
        "85dd1276eb3e1bcbd958cf1d50dd94fc6f82d81d1faecf20c870de61f6c06cd6",
    )?;
    let paired: WaypointDirectSourceDurationPairedCommandFeasibilityArtifact =
        serde_json::from_slice(&fs::read(paired_summary)?)?;
    let acceptance: WaypointDirectCompleteFlatAcceptanceArtifact =
        serde_json::from_slice(&fs::read(acceptance_summary)?)?;
    if paired.identity != "fnv1a64:080e8e9867b02d97"
        || acceptance.identity != "fnv1a64:ba51ace2c0bf1d08"
        || paired.rows.len() != 15
        || acceptance.rows.len() != 15
    {
        bail!("historical Gate A identities or family changed");
    }
    let mut rows = Vec::with_capacity(15);
    for historical in &acceptance.rows {
        let actual = generated.rows.iter().find(|row| {
            row.basis_candidate_identity == historical.candidate_identity
                && row.duration_offset_ticks == historical.duration_offset_ticks
        });
        let old_schedule = paired
            .rows
            .iter()
            .find(|row| {
                row.candidate_identity == historical.candidate_identity
                    && row.duration_offset_ticks == historical.duration_offset_ticks
            })
            .and_then(|row| row.paired_schedule.as_ref());
        let analytical =
            actual.is_some_and(|row| row.analytical_survivor == historical.analytical_survivor);
        let schedule = actual.is_some_and(|row| row.paired_schedule.as_ref() == old_schedule);
        let physical = match (
            actual.and_then(|row| row.wrapper.as_ref()),
            historical.wrapper.as_ref(),
        ) {
            (None, None) => true,
            (Some(actual), Some(old)) => {
                actual.accepted == old.accepted
                    && actual.acceptance == old.acceptance
                    && actual.source_handoff_position_error_m == old.source_handoff_position_error_m
                    && actual.source_handoff_velocity_error_mps
                        == old.source_handoff_velocity_error_mps
                    && actual.first_contact == old.first_contact
                    && actual.signed_first_contact_margins == old.signed_first_contact_margins
                    && actual.planned_total_mission_time_s == old.planned_total_mission_time_s
                    && actual.actual_touchdown_time_s == old.actual_touchdown_time_s
                    && actual.actual_touchdown_fuel_used_kg == old.actual_touchdown_fuel_used_kg
                    && actual.minimum_airborne_clearance == old.minimum_airborne_clearance
                    && actual.clearance_scan == old.clearance_scan
                    && actual.replay == old.replay
                    && actual.recomputed_saturation == old.recomputed_saturation
            }
            _ => false,
        };
        rows.push(WaypointDirectGenerationParityRow {
            basis_candidate_identity: historical.candidate_identity.clone(),
            duration_offset_ticks: historical.duration_offset_ticks,
            analytical_survivor_matches: analytical,
            complete_paired_schedule_matches: schedule,
            complete_physical_acceptance_matches: physical,
            passed: analytical && schedule && physical,
        });
    }
    let extras = generated
        .rows
        .iter()
        .filter(|row| {
            !acceptance.rows.iter().any(|old| {
                old.candidate_identity == row.basis_candidate_identity
                    && old.duration_offset_ticks == row.duration_offset_ticks
            })
        })
        .collect::<Vec<_>>();
    let extra_ok = extras.len() == 5
        && extras.iter().all(|row| {
            !row.analytical_survivor && row.paired_schedule.is_none() && row.wrapper.is_none()
        })
        && extras
            .iter()
            .map(|row| row.duration_offset_ticks)
            .collect::<Vec<_>>()
            == [-240, -180, -120, -60, 0];
    let scheduled_count = generated
        .rows
        .iter()
        .filter(|row| row.paired_schedule.is_some())
        .count();
    let accepted_count = generated
        .rows
        .iter()
        .filter(|row| row.wrapper.as_ref().is_some_and(|wrapper| wrapper.accepted))
        .count();
    let selected_matches = generated.selection.as_ref().is_some_and(|selection| {
        selection.planned_total_mission_time_s == 34.35
            && generated.rows.get(selection.row_index).is_some_and(|row| {
                row.basis_candidate_identity == "fnv1a64:dee613017622ca16"
                    && row.duration_offset_ticks == -180
                    && row.wrapper.as_ref().is_some_and(|wrapper| {
                        wrapper.accepted
                            && wrapper.actual_touchdown_time_s == Some(34.275)
                            && wrapper.wrapper_identity == selection.wrapper_identity
                    })
            })
    });
    let passed = rows.iter().all(|row| row.passed)
        && generated.rows.len() == 20
        && generated.rows.iter().all(|row| {
            row.accepted == row.wrapper.as_ref().is_some_and(|wrapper| wrapper.accepted)
        })
        && extra_ok
        && scheduled_count == 9
        && accepted_count == 4
        && selected_matches;
    let mut gate = WaypointDirectGenerationGateA {
        schema_id: "waypoint_direct_generation_gate_a_v1".to_owned(),
        generated_identity: generated.identity,
        historical_paired_identity: paired.identity,
        historical_acceptance_identity: acceptance.identity,
        rows,
        generated_row_count: generated.rows.len(),
        scheduled_count,
        accepted_count,
        extra_rows_are_five_unflown_rejected_basis_skips: extra_ok,
        selected_physical_witness_matches: selected_matches,
        passed,
        identity: String::new(),
    };
    gate.identity = semantic_digest(&gate)?;
    write_create_only_summary(output_dir, &gate)?;
    Ok(gate)
}

fn semantic_digest<T: Serialize>(value: &T) -> Result<String> {
    let hash = serde_json::to_vec(value)?
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    Ok(format!("fnv1a64:{hash:016x}"))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaypointDirectGenerationCodeFreeze {
    pub schema_id: String,
    pub gate_a_identity: String,
    pub generated_identity: String,
    pub sealed_manifest_sha256: String,
    pub production_inputs_identity: String,
    pub identity: String,
}

/// Seal the production inputs after primary acceptance of Gate A, before any
/// fresh case is solved. Paths and timestamps do not enter the identity.
pub fn seal_waypoint_direct_generation_code(
    repo_root: &Path,
    gate_a_summary: &Path,
    output_dir: &Path,
) -> Result<WaypointDirectGenerationCodeFreeze> {
    let gate = load_passed_gate_a(gate_a_summary)?;
    load_waypoint_direct_generation_fresh_manifest(repo_root)?;
    let mut freeze = WaypointDirectGenerationCodeFreeze {
        schema_id: "waypoint_direct_generation_code_freeze_v1".to_owned(),
        gate_a_identity: gate.identity,
        generated_identity: gate.generated_identity,
        sealed_manifest_sha256: DIRECT_GENERATION_FRESH_MANIFEST_SHA256.to_owned(),
        production_inputs_identity: production_inputs_identity(repo_root)?,
        identity: String::new(),
    };
    freeze.identity = semantic_digest(&freeze)?;
    write_create_only_summary(output_dir, &freeze)?;
    Ok(freeze)
}

fn load_passed_gate_a(path: &Path) -> Result<WaypointDirectGenerationGateA> {
    let gate: WaypointDirectGenerationGateA = serde_json::from_slice(&fs::read(path)?)?;
    let mut input = gate.clone();
    input.identity.clear();
    if gate.schema_id != "waypoint_direct_generation_gate_a_v1"
        || !gate.passed
        || gate.rows.len() != 15
        || !gate.rows.iter().all(|row| row.passed)
        || gate.generated_row_count != 20
        || gate.scheduled_count != 9
        || gate.accepted_count != 4
        || !gate.selected_physical_witness_matches
        || !gate.extra_rows_are_five_unflown_rejected_basis_skips
        || semantic_digest(&input)? != gate.identity
    {
        bail!("Gate A has not passed or its identity changed");
    }
    Ok(gate)
}

fn production_inputs_identity(repo_root: &Path) -> Result<String> {
    fn collect(root: &Path, path: &Path, entries: &mut Vec<(String, Vec<u8>)>) -> Result<()> {
        for item in fs::read_dir(path)? {
            let path = item?.path();
            if path.is_dir() {
                collect(root, &path, entries)?;
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                entries.push((
                    path.strip_prefix(root)?.to_string_lossy().into_owned(),
                    fs::read(&path)?,
                ));
            }
        }
        Ok(())
    }
    let mut entries = Vec::new();
    for name in [
        "pd-core",
        "pd-plan",
        "pd-control",
        "pd-eval",
        "pd-report",
        "pd-cli",
    ] {
        collect(repo_root, &repo_root.join(name).join("src"), &mut entries)?;
        let path = format!("{name}/Cargo.toml");
        entries.push((path.clone(), fs::read(repo_root.join(path))?));
    }
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "fixtures/scenarios/flat_terminal_descent.json",
        "pd-plan/fixtures/conservative_ballistic_direct_bridge_probes_v2.json",
        DIRECT_GENERATION_FRESH_MANIFEST,
    ] {
        entries.push((path.to_owned(), fs::read(repo_root.join(path))?));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    semantic_digest(&entries)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaypointDirectGenerationFreshResult {
    pub case_id: String,
    pub generation_identity: Option<String>,
    pub row_count: usize,
    pub scheduled_count: usize,
    pub accepted_count: usize,
    pub selected_basis_identity: Option<String>,
    pub selected_duration_offset_ticks: Option<i64>,
    pub planned_total_mission_time_s: Option<f64>,
    pub actual_touchdown_time_s: Option<f64>,
    pub actual_fuel_used_kg: Option<f64>,
    pub signed_first_contact_margins: Option<crate::ScheduledFirstContactMarginsEvidence>,
    pub execution_status: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaypointDirectGenerationFreshGate {
    pub schema_id: String,
    pub gate_a_identity: String,
    pub code_freeze_identity: String,
    pub sealed_manifest_sha256: String,
    pub cases: Vec<WaypointDirectGenerationFreshResult>,
    pub passed: bool,
    pub verdict: String,
    pub identity: String,
}

pub fn run_waypoint_direct_generation_fresh_gate(
    repo_root: &Path,
    gate_a_summary: &Path,
    code_freeze_summary: &Path,
    output_dir: &Path,
) -> Result<WaypointDirectGenerationFreshGate> {
    let gate_a = load_passed_gate_a(gate_a_summary)?;
    let freeze: WaypointDirectGenerationCodeFreeze =
        serde_json::from_slice(&fs::read(code_freeze_summary)?)?;
    let mut freeze_input = freeze.clone();
    freeze_input.identity.clear();
    if freeze.schema_id != "waypoint_direct_generation_code_freeze_v1"
        || freeze.gate_a_identity != gate_a.identity
        || freeze.generated_identity != gate_a.generated_identity
        || freeze.sealed_manifest_sha256 != DIRECT_GENERATION_FRESH_MANIFEST_SHA256
        || semantic_digest(&freeze_input)? != freeze.identity
        || production_inputs_identity(repo_root)? != freeze.production_inputs_identity
    {
        bail!("production code is not the accepted frozen Gate A implementation");
    }
    let manifest = load_waypoint_direct_generation_fresh_manifest(repo_root)?;
    if output_dir.exists() {
        bail!("create-only output directory already exists");
    }
    if let Some(parent) = output_dir.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(output_dir)?;
    let mut cases = Vec::with_capacity(6);
    for case in &manifest.cases {
        if production_inputs_identity(repo_root)? != freeze.production_inputs_identity {
            bail!("production input changed during fresh gate; stop without retuning");
        }
        let request = manifest.request(&case.case_id)?;
        let run = crate::run_waypoint_direct_nominal_direct_generation(
            repo_root,
            &request,
            &output_dir.join("cases").join(&case.case_id),
        );
        let result = match run {
            Ok(run) => {
                let artifact = run.artifact;
                let selected = artifact
                    .selection
                    .as_ref()
                    .and_then(|selection| artifact.rows.get(selection.row_index));
                let wrapper = selected.and_then(|row| row.wrapper.as_ref());
                WaypointDirectGenerationFreshResult {
                    case_id: case.case_id.clone(),
                    generation_identity: Some(artifact.identity.clone()),
                    row_count: artifact.rows.len(),
                    scheduled_count: artifact
                        .rows
                        .iter()
                        .filter(|row| row.paired_schedule.is_some())
                        .count(),
                    accepted_count: artifact
                        .rows
                        .iter()
                        .filter(|row| row.wrapper.as_ref().is_some_and(|wrapper| wrapper.accepted))
                        .count(),
                    selected_basis_identity: selected
                        .map(|row| row.basis_candidate_identity.clone()),
                    selected_duration_offset_ticks: selected.map(|row| row.duration_offset_ticks),
                    planned_total_mission_time_s: wrapper
                        .map(|wrapper| wrapper.planned_total_mission_time_s),
                    actual_touchdown_time_s: wrapper
                        .and_then(|wrapper| wrapper.actual_touchdown_time_s),
                    actual_fuel_used_kg: wrapper
                        .and_then(|wrapper| wrapper.actual_touchdown_fuel_used_kg),
                    signed_first_contact_margins: wrapper
                        .and_then(|wrapper| wrapper.signed_first_contact_margins.clone()),
                    execution_status: artifact.execution_status,
                    error: None,
                }
            }
            Err(error) => WaypointDirectGenerationFreshResult {
                case_id: case.case_id.clone(),
                generation_identity: None,
                row_count: 0,
                scheduled_count: 0,
                accepted_count: 0,
                selected_basis_identity: None,
                selected_duration_offset_ticks: None,
                planned_total_mission_time_s: None,
                actual_touchdown_time_s: None,
                actual_fuel_used_kg: None,
                signed_first_contact_margins: None,
                execution_status:
                    "input_or_integrity_or_execution_error_not_physical_impossibility".to_owned(),
                error: Some(format!("{error:#}")),
            },
        };
        cases.push(result);
    }
    if production_inputs_identity(repo_root)? != freeze.production_inputs_identity {
        bail!("production input changed during fresh gate; stop without retuning");
    }
    let passed = cases.iter().all(|case| {
        case.error.is_none()
            && case.row_count == 20
            && case.accepted_count > 0
            && case.selected_basis_identity.is_some()
    });
    let mut gate = WaypointDirectGenerationFreshGate {
        schema_id: "waypoint_direct_generation_fresh_gate_v1".to_owned(),
        gate_a_identity: gate_a.identity,
        code_freeze_identity: freeze.identity,
        sealed_manifest_sha256: DIRECT_GENERATION_FRESH_MANIFEST_SHA256.to_owned(),
        cases,
        passed,
        verdict: if passed {
            "six_predeclared_nominal_direct_cases_passed_not_robust_or_default_authority"
        } else {
            "bounded_fresh_gate_failed_publish_all_six_and_stop_without_retuning"
        }
        .to_owned(),
        identity: String::new(),
    };
    gate.identity = semantic_digest(&gate)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_dir.join("summary.json"))?;
    file.write_all(&serde_json::to_vec_pretty(&gate)?)?;
    file.write_all(b"\n")?;
    Ok(gate)
}

fn write_create_only_summary<T: Serialize>(output_dir: &Path, value: &T) -> Result<PathBuf> {
    if output_dir.exists() {
        bail!("create-only output directory already exists");
    }
    if let Some(parent) = output_dir.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(output_dir)?;
    let path = output_dir.join("summary.json");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.write_all(b"\n")?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
    }

    #[test]
    fn sealed_inputs_load_without_solving_or_physics() {
        let manifest = load_waypoint_direct_generation_fresh_manifest(repo_root()).unwrap();
        for case in &manifest.cases {
            let request = manifest.request(&case.case_id).unwrap();
            crate::validate_waypoint_direct_nominal_direct_generation_request(&request).unwrap();
            assert!(request.scenario.mission.transfer_route.is_none());
        }
        assert!(manifest.request("missing_case").is_err());
    }

    #[test]
    fn known_flat_factory_only_supplies_inputs() {
        let request = waypoint_direct_known_flat_generation_request(repo_root()).unwrap();
        assert_eq!(request.probe_id, "continuous_flat_r00");
        assert_eq!(
            request.scenario.initial_state.position_m,
            pd_core::Vec2::new(-800.0, 5.0)
        );
        crate::validate_waypoint_direct_nominal_direct_generation_request(&request).unwrap();
    }

    fn scratch_dir(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pd-direct-generation-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn summary_writer_is_create_only_and_path_independent() {
        let scratch = scratch_dir("create-only");
        let value = serde_json::json!({"identity": "input-only"});
        let a = write_create_only_summary(&scratch.join("a"), &value).unwrap();
        let b = write_create_only_summary(&scratch.join("b"), &value).unwrap();
        assert_eq!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
        assert!(write_create_only_summary(&scratch.join("a"), &value).is_err());
        fs::remove_file(a).unwrap();
        fs::remove_file(b).unwrap();
        fs::remove_dir(scratch.join("a")).unwrap();
        fs::remove_dir(scratch.join("b")).unwrap();
        fs::remove_dir(scratch).unwrap();
    }

    #[test]
    fn failed_or_tampered_gate_a_cannot_open_fresh_evaluation() {
        let scratch = scratch_dir("gate-integrity");
        let path = scratch.join("gate.json");
        let mut gate = WaypointDirectGenerationGateA {
            schema_id: "waypoint_direct_generation_gate_a_v1".to_owned(),
            generated_identity: "example".to_owned(),
            historical_paired_identity: "fnv1a64:080e8e9867b02d97".to_owned(),
            historical_acceptance_identity: "fnv1a64:ba51ace2c0bf1d08".to_owned(),
            rows: (0..15)
                .map(|row| WaypointDirectGenerationParityRow {
                    basis_candidate_identity: format!("example-{row}"),
                    duration_offset_ticks: 0,
                    analytical_survivor_matches: true,
                    complete_paired_schedule_matches: true,
                    complete_physical_acceptance_matches: true,
                    passed: true,
                })
                .collect(),
            generated_row_count: 20,
            scheduled_count: 9,
            accepted_count: 4,
            extra_rows_are_five_unflown_rejected_basis_skips: true,
            selected_physical_witness_matches: true,
            passed: false,
            identity: String::new(),
        };
        gate.identity = semantic_digest(&gate).unwrap();
        fs::write(&path, serde_json::to_vec(&gate).unwrap()).unwrap();
        assert!(load_passed_gate_a(&path).is_err());
        gate.passed = true; // Deliberately leave the old semantic identity.
        fs::write(&path, serde_json::to_vec(&gate).unwrap()).unwrap();
        assert!(load_passed_gate_a(&path).is_err());
        fs::remove_file(path).unwrap();
        fs::remove_dir(scratch).unwrap();
    }

    #[test]
    fn production_input_freeze_is_deterministic() {
        assert_eq!(
            production_inputs_identity(repo_root()).unwrap(),
            production_inputs_identity(repo_root()).unwrap()
        );
    }
}
