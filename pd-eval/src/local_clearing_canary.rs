//! Create-only evidence and preservation runner for the one-obstruction local
//! clearing experiment. The physical experiment itself lives in
//! `local_clearing`; this module binds its inputs/source and retains results.

use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    LocalClearingExperimentV1, NominalDirectFlightSourceBindingV1, NominalDirectFlightSourceFileV1,
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    WaypointDirectObstacleDiscriminationFreshCaseV1,
    WaypointDirectObstacleDiscriminationFreshManifestV1,
    canonical_initial_direct::run_canonical_initial_direct_canary,
    nominal_airborne_direct::run_nominal_airborne_direct_canary,
    nominal_direct_flight::{reserve_output_root, write_create_only},
    nominal_direct_flight_gate::{
        DEFAULT_NOMINAL_DIRECT_FLIGHT_ARCHIVE_ROOT, run_nominal_direct_flight_regression,
        source_binding,
    },
    nominal_direct_flight_identity,
    waypoint_direct_body_aware_terminal::sha256_bytes,
    waypoint_direct_obstacle_discrimination::load_waypoint_direct_obstacle_discrimination_fresh_manifest,
};

pub const LOCAL_CLEARING_CANARY_PROTOCOL: &str = "docs/local_clearing_canary_protocol.md";
pub const LOCAL_CLEARING_CANARY_PROTOCOL_SHA256: &str =
    "1948d70544280c60214503d4fd7fb2ce1c4159ce88e7617eaefce0711c4e82e8";
pub const LOCAL_CLEARING_CANARY_INPUTS: &str =
    "fixtures/research/local_clearing_canary_inputs_v1.json";
pub const LOCAL_CLEARING_CANARY_INPUTS_SHA256: &str =
    "178c38890c91d3a6bb611697418668a1639dae9093626d50a95a5a173f9c3702";
pub const LOCAL_CLEARING_CANARY_PHYSICAL_INPUTS: &str =
    "fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json";
pub const LOCAL_CLEARING_CANARY_PHYSICAL_INPUTS_SHA256: &str =
    "647df7bc94d02a9b48b773c45159e0ffa15bfca232a13b4e21164e2251a8f5b4";
pub const LOCAL_CLEARING_CANARY_SCHEMA_ID: &str = "local_clearing_canary_v1";
const REPEAT_COMPARATOR_SOURCE: &str = "pd-eval/examples/compare_local_clearing_canary_repeats.rs";

const FLAT_CASE_ID: &str = "fresh_flat_control_span_900";
const OBSTACLE_CASE_ID: &str = "fresh_late_broad_span_900";
const CANONICAL_REFERENCE_CASES: [&str; 8] = [
    "operational_flat_span_685",
    "operational_flat_span_845",
    "operational_uphill_span_845",
    "operational_downhill_span_845",
    "completion_flat_span_735",
    "completion_flat_span_915",
    "completion_uphill_span_915",
    "completion_downhill_span_915",
];
const AIRBORNE_REFERENCE_CASES: [&str; 4] = [
    "operational_flat_span_685",
    "operational_flat_span_845",
    "operational_uphill_span_845",
    "operational_downhill_span_845",
];
const GATE_IDS: [&str; 5] = [
    "baseline_and_entries",
    "local_generation",
    "local_physical_proof",
    "handoff_compatibility",
    "closure_and_preservation",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingCanarySealV1 {
    pub relative_path: String,
    pub expected_sha256: String,
    pub observed_sha256: Option<String>,
    pub matches_seal: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingCanaryRequestsV1 {
    pub flat_case_id: String,
    pub obstacle_case_id: String,
    pub flat_request: WaypointDirectNominalDirectGenerationRequest,
    pub obstacle_request: WaypointDirectNominalDirectGenerationRequest,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingCanaryPreflightV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub ready: bool,
    pub seals: Vec<LocalClearingCanarySealV1>,
    pub source_binding: Option<NominalDirectFlightSourceBindingV1>,
    pub requests: Option<LocalClearingCanaryRequestsV1>,
    pub failures: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingPayloadComparisonV1 {
    pub comparison_id: String,
    pub reference_relative_path: String,
    pub observed_relative_path: String,
    pub reference_file_sha256: Option<String>,
    pub observed_file_sha256: Option<String>,
    pub normalized_reference_sha256: Option<String>,
    pub normalized_observed_sha256: Option<String>,
    pub exclusion_patterns: Vec<String>,
    pub applied_exclusion_paths: Vec<String>,
    pub passed: bool,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingPreservationStageV1 {
    pub stage_id: String,
    pub status: String,
    pub reference_root: String,
    pub observed_root: String,
    pub source_binding_before_sha256: Option<String>,
    pub source_binding_after_sha256: Option<String>,
    pub comparisons: Vec<LocalClearingPayloadComparisonV1>,
    pub failures: Vec<String>,
    pub passed: bool,
    pub elapsed_wall_time_us: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingPreservationV1 {
    pub canonical_controls: LocalClearingPreservationStageV1,
    pub airborne_continuations: LocalClearingPreservationStageV1,
    pub source_rest_controls: LocalClearingPreservationStageV1,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingCanaryArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub protocol_path: String,
    pub protocol_sha256: Option<String>,
    pub selection_manifest_path: String,
    pub selection_manifest_sha256: Option<String>,
    pub physical_manifest_path: String,
    pub physical_manifest_sha256: Option<String>,
    pub source_binding_before: Option<NominalDirectFlightSourceBindingV1>,
    pub source_binding_after: Option<NominalDirectFlightSourceBindingV1>,
    pub source_unchanged_before_and_after: bool,
    pub requests: Option<LocalClearingCanaryRequestsV1>,
    pub experiment_relative_path: Option<String>,
    pub experiment_identity: Option<String>,
    pub gates: Vec<crate::LocalClearingGateV1>,
    pub preservation_relative_path: Option<String>,
    pub preservation_passed: Option<bool>,
    pub setup_failures: Vec<String>,
    pub passed: bool,
    pub identity: String,
    pub elapsed_wall_time_us: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingRepeatComparisonV1 {
    pub left_root: String,
    pub right_root: String,
    pub source_bindings_match: bool,
    pub preflight_payload_matches: bool,
    pub experiment_payload_matches: bool,
    pub experiment_identities_valid: bool,
    pub preservation_payload_matches: bool,
    pub preservation_ledgers_bind_files: bool,
    pub preservation_files_match: bool,
    pub preservation_file_comparisons: Vec<LocalClearingPayloadComparisonV1>,
    pub summary_identities_valid: bool,
    pub summary_payload_matches: bool,
    pub passed: bool,
    pub failures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionManifestV1 {
    schema_id: String,
    schema_version: u32,
    sealed_before_measurement: bool,
    input_manifest: String,
    input_manifest_sha256: String,
    flat_case_id: String,
    obstacle_case_id: String,
    exposure: String,
    policy_id: String,
    entry_order: Vec<String>,
    attitudes_degrees: Vec<i32>,
    acceleration_factors: Vec<f64>,
    powered_ticks: Vec<u64>,
    maximum_coast_ticks: u64,
    continuation_ticks: u64,
    minimum_clearance_m: f64,
    maximum_powered_rows: usize,
    maximum_handoff_boundary_checks: usize,
    progress: String,
    ranking: String,
    canonical_reference_root: String,
    airborne_reference_root: String,
    source_rest_reference_root: String,
}

struct PreparedInputs {
    preflight: LocalClearingCanaryPreflightV1,
    selection: Option<SelectionManifestV1>,
}

/// Execute the sealed local-clearing experiment and all independent source
/// preservation checks into a new create-only root.
pub fn run_local_clearing_canary(
    repo_root: &Path,
    output_root: &Path,
) -> Result<LocalClearingCanaryArtifactV1> {
    let started = Instant::now();
    reserve_output_root(output_root)?;

    let mut artifact = empty_artifact();
    let source_before = match local_clearing_source_binding(repo_root) {
        Ok(binding) => Some(binding),
        Err(error) => {
            artifact.setup_failures.push(format!(
                "source binding failed before evaluation: {error:#}"
            ));
            None
        }
    };
    artifact.source_binding_before = source_before.clone();

    let prepared = prepare_inputs(repo_root, source_before.clone());
    artifact.protocol_sha256 = prepared
        .preflight
        .seals
        .iter()
        .find(|seal| seal.relative_path == LOCAL_CLEARING_CANARY_PROTOCOL)
        .and_then(|seal| seal.observed_sha256.clone());
    artifact.selection_manifest_sha256 = prepared
        .preflight
        .seals
        .iter()
        .find(|seal| seal.relative_path == LOCAL_CLEARING_CANARY_INPUTS)
        .and_then(|seal| seal.observed_sha256.clone());
    artifact.physical_manifest_sha256 = prepared
        .preflight
        .seals
        .iter()
        .find(|seal| seal.relative_path == LOCAL_CLEARING_CANARY_PHYSICAL_INPUTS)
        .and_then(|seal| seal.observed_sha256.clone());
    artifact.requests = prepared.preflight.requests.clone();
    artifact
        .setup_failures
        .extend(prepared.preflight.failures.clone());

    write_create_only(&output_root.join("preflight.json"), &prepared.preflight)?;

    let mut experiment = None;
    let mut experiment_contract_valid = false;
    if prepared.preflight.ready {
        if let Some(requests) = &prepared.preflight.requests {
            match crate::evaluate_local_clearing_experiment(
                requests.flat_request.clone(),
                requests.obstacle_request.clone(),
            ) {
                Ok(mut completed) => {
                    // Preserve even a contract mismatch as returned bounded
                    // evidence; it cannot accidentally receive closure credit.
                    experiment_contract_valid = match ensure_gate_contract(&mut completed) {
                        Ok(()) => true,
                        Err(error) => {
                            artifact.setup_failures.push(format!(
                                "local experiment gate contract mismatch: {error:#}"
                            ));
                            false
                        }
                    };
                    artifact.experiment_relative_path = Some("experiment.json".into());
                    experiment = Some(completed);
                }
                Err(error) => artifact.setup_failures.push(format!(
                    "local experiment failed before returning bounded evidence: {error:#}"
                )),
            }
        } else {
            artifact
                .setup_failures
                .push("sealed requests were unavailable after successful preflight".into());
        }
    }

    let preservation = if prepared.preflight.ready {
        Some(run_preservation(
            repo_root,
            output_root,
            prepared.selection.as_ref(),
        )?)
    } else {
        None
    };
    if let Some(preservation) = &preservation {
        artifact.preservation_relative_path = Some("preservation.json".into());
        artifact.preservation_passed = Some(preservation.passed);
        write_create_only(&output_root.join("preservation.json"), preservation)?;
    }

    let source_after = match local_clearing_source_binding(repo_root) {
        Ok(binding) => Some(binding),
        Err(error) => {
            artifact
                .setup_failures
                .push(format!("source binding failed after evaluation: {error:#}"));
            None
        }
    };
    artifact.source_binding_after = source_after.clone();
    artifact.source_unchanged_before_and_after =
        source_before.is_some() && source_before == source_after && prepared.preflight.ready;
    if !artifact.source_unchanged_before_and_after && prepared.preflight.ready {
        artifact
            .setup_failures
            .push("recursive source/input/protocol closure changed during canary".into());
    }

    if let Some(experiment) = &mut experiment {
        if experiment_contract_valid {
            let preceding_passed = experiment
                .gates
                .iter()
                .take(4)
                .all(|gate| gate.status == "passed");
            let closure_ok = artifact.source_unchanged_before_and_after
                && preservation.as_ref().is_some_and(|value| value.passed);
            let closure_gate = experiment
                .gates
                .iter_mut()
                .find(|gate| gate.gate_id == "closure_and_preservation")
                .context("experiment omitted closure_and_preservation gate")?;
            if preceding_passed && closure_ok {
                closure_gate.status = "passed".into();
                closure_gate.reason = None;
            } else if preceding_passed {
                closure_gate.status = "failed".into();
                closure_gate.reason = Some(if !artifact.source_unchanged_before_and_after {
                    "recursive source/input/protocol closure changed".into()
                } else {
                    "one or more independent preservation comparisons failed".into()
                });
            } else {
                closure_gate.status = "not_evaluated".into();
                closure_gate.reason = Some(
                    "closure cannot pass because an earlier physical gate did not pass".into(),
                );
            }
        }
        artifact.gates = experiment.gates.clone();
        match nominal_direct_flight_identity(experiment) {
            Ok(identity) => artifact.experiment_identity = Some(identity),
            Err(error) => artifact
                .setup_failures
                .push(format!("local experiment identity failed: {error:#}")),
        }
        write_create_only(&output_root.join("experiment.json"), experiment)?;
    } else {
        artifact.gates = GATE_IDS
            .iter()
            .map(|gate_id| crate::LocalClearingGateV1 {
                gate_id: (*gate_id).into(),
                status: "not_evaluated".into(),
                reason: Some("local experiment evidence unavailable".into()),
            })
            .collect();
    }
    artifact.passed = artifact.setup_failures.is_empty()
        && artifact.gates.len() == GATE_IDS.len()
        && artifact.gates.iter().all(|gate| gate.status == "passed");
    artifact.elapsed_wall_time_us = elapsed_us(started);
    artifact.identity = canary_artifact_identity(&artifact)?;
    write_create_only(&output_root.join("summary.json"), &artifact)?;
    Ok(artifact)
}

fn empty_artifact() -> LocalClearingCanaryArtifactV1 {
    LocalClearingCanaryArtifactV1 {
        schema_id: LOCAL_CLEARING_CANARY_SCHEMA_ID.into(),
        schema_version: 1,
        protocol_path: LOCAL_CLEARING_CANARY_PROTOCOL.into(),
        protocol_sha256: None,
        selection_manifest_path: LOCAL_CLEARING_CANARY_INPUTS.into(),
        selection_manifest_sha256: None,
        physical_manifest_path: LOCAL_CLEARING_CANARY_PHYSICAL_INPUTS.into(),
        physical_manifest_sha256: None,
        source_binding_before: None,
        source_binding_after: None,
        source_unchanged_before_and_after: false,
        requests: None,
        experiment_relative_path: None,
        experiment_identity: None,
        gates: Vec::new(),
        preservation_relative_path: None,
        preservation_passed: None,
        setup_failures: Vec::new(),
        passed: false,
        identity: String::new(),
        elapsed_wall_time_us: 0,
    }
}

fn prepare_inputs(
    repo_root: &Path,
    source_binding: Option<NominalDirectFlightSourceBindingV1>,
) -> PreparedInputs {
    let mut seals = vec![
        check_seal(
            repo_root,
            LOCAL_CLEARING_CANARY_PROTOCOL,
            LOCAL_CLEARING_CANARY_PROTOCOL_SHA256,
        ),
        check_seal(
            repo_root,
            LOCAL_CLEARING_CANARY_INPUTS,
            LOCAL_CLEARING_CANARY_INPUTS_SHA256,
        ),
        check_seal(
            repo_root,
            LOCAL_CLEARING_CANARY_PHYSICAL_INPUTS,
            LOCAL_CLEARING_CANARY_PHYSICAL_INPUTS_SHA256,
        ),
    ];
    let mut failures = Vec::new();
    if seals.iter().any(|seal| !seal.matches_seal) {
        failures.push("one or more sealed protocol/input SHA-256 values differ".into());
    }

    let selection = match fs::read(repo_root.join(LOCAL_CLEARING_CANARY_INPUTS))
        .with_context(|| format!("read selection manifest {LOCAL_CLEARING_CANARY_INPUTS}"))
        .and_then(|bytes| serde_json::from_slice::<SelectionManifestV1>(&bytes).map_err(Into::into))
    {
        Ok(selection) if selection_manifest_matches(&selection) => Some(selection),
        Ok(_) => {
            failures.push("selection manifest fields differ from the sealed policy".into());
            None
        }
        Err(error) => {
            failures.push(format!("selection manifest could not be parsed: {error:#}"));
            None
        }
    };

    let physical_manifest =
        match load_waypoint_direct_obstacle_discrimination_fresh_manifest(repo_root) {
            Ok(manifest) => Some(manifest),
            Err(error) => {
                failures.push(format!(
                    "sealed physical input manifest failed validation: {error:#}"
                ));
                None
            }
        };
    let requests = match (selection.as_ref(), physical_manifest.as_ref()) {
        (Some(selection), Some(manifest)) => match selected_requests(selection, manifest) {
            Ok(requests) => Some(requests),
            Err(error) => {
                failures.push(format!(
                    "selected 900 m physical pair is invalid: {error:#}"
                ));
                None
            }
        },
        _ => None,
    };

    // Preserve observed hashes even when selection parsing failed.
    if seals.len() != 3 {
        failures.push("internal seal inventory did not contain exactly three inputs".into());
    }
    seals.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let ready = failures.is_empty()
        && source_binding.is_some()
        && requests.is_some()
        && seals.iter().all(|seal| seal.matches_seal);
    if source_binding.is_none() {
        failures.push("recursive source closure is unavailable".into());
    }
    PreparedInputs {
        preflight: LocalClearingCanaryPreflightV1 {
            schema_id: "local_clearing_canary_preflight_v1".into(),
            schema_version: 1,
            ready,
            seals,
            source_binding,
            requests,
            failures,
        },
        selection,
    }
}

fn check_seal(
    repo_root: &Path,
    relative_path: &str,
    expected_sha256: &str,
) -> LocalClearingCanarySealV1 {
    let observed_sha256 = fs::read(repo_root.join(relative_path))
        .ok()
        .and_then(|bytes| sha256_bytes(&bytes).ok());
    LocalClearingCanarySealV1 {
        relative_path: relative_path.into(),
        expected_sha256: expected_sha256.into(),
        matches_seal: observed_sha256.as_deref() == Some(expected_sha256),
        observed_sha256,
    }
}

fn selection_manifest_matches(manifest: &SelectionManifestV1) -> bool {
    manifest.schema_id == "local_clearing_canary_inputs_v1"
        && manifest.schema_version == 1
        && manifest.sealed_before_measurement
        && manifest.input_manifest == LOCAL_CLEARING_CANARY_PHYSICAL_INPUTS
        && manifest.input_manifest_sha256 == LOCAL_CLEARING_CANARY_PHYSICAL_INPUTS_SHA256
        && manifest.flat_case_id == FLAT_CASE_ID
        && manifest.obstacle_case_id == OBSTACLE_CASE_ID
        && manifest.exposure
            == "Both physical requests were exposed by the completed canonical initial direct discriminator. No held-out claim."
        && manifest.policy_id == "one_obstruction_powered_coast_clearing_v1"
        && manifest.entry_order
            == [
                "first_idle_hold",
                "source_75_percent",
                "source_50_percent",
                "source_25_percent",
            ]
        && manifest.attitudes_degrees == [-30, 0, 30]
        && manifest.acceleration_factors == [0.75, 1.0]
        && manifest.powered_ticks == [60, 120, 240, 360, 480, 720, 960]
        && manifest.maximum_coast_ticks == 720
        && manifest.continuation_ticks == 240
        && manifest.minimum_clearance_m == 5.0
        && manifest.maximum_powered_rows == 168
        && manifest.maximum_handoff_boundary_checks == 60_480
        && manifest.progress
            == "max(entry_x,first_conflict_x) plus twice maximum COM-to-body-point radius"
        && manifest.ranking
            == "latest entry then earliest handoff then least actual fuel burn then stable row identity"
        && manifest.canonical_reference_root
            == "outputs/research/canonical_initial_direct_canary_20260930/final_b"
        && manifest.airborne_reference_root
            == "outputs/research/canonical_initial_direct_canary_20260930/airborne_preservation"
        && manifest.source_rest_reference_root
            == "outputs/research/canonical_initial_direct_canary_20260930/source_rest_preservation"
}

fn selected_requests(
    selection: &SelectionManifestV1,
    manifest: &WaypointDirectObstacleDiscriminationFreshManifestV1,
) -> Result<LocalClearingCanaryRequestsV1> {
    if manifest.generation_policy != WaypointDirectNominalDirectGenerationPolicyV1::default() {
        bail!("physical manifest generation policy changed");
    }
    let flat = manifest
        .cases
        .iter()
        .find(|case| case.case_id == selection.flat_case_id)
        .context("sealed flat 900 m case is missing")?;
    let obstacle = manifest
        .cases
        .iter()
        .find(|case| case.case_id == selection.obstacle_case_id)
        .context("sealed late/broad 900 m case is missing")?;
    validate_physical_case(flat, FLAT_CASE_ID, "flat_control", 0.0, 0.0, 0.0)?;
    validate_physical_case(obstacle, OBSTACLE_CASE_ID, "late_broad", 0.7, 0.25, 0.35)?;
    let make_request = |case: &WaypointDirectObstacleDiscriminationFreshCaseV1| {
        WaypointDirectNominalDirectGenerationRequest {
            scenario: case.scenario.clone(),
            source_pad_id: case.source_pad_id.clone(),
            target_pad_id: case.target_pad_id.clone(),
            probe_id: case.probe_id.clone(),
            policy: manifest.generation_policy.clone(),
        }
    };
    Ok(LocalClearingCanaryRequestsV1 {
        flat_case_id: flat.case_id.clone(),
        obstacle_case_id: obstacle.case_id.clone(),
        flat_request: make_request(flat),
        obstacle_request: make_request(obstacle),
    })
}

fn validate_physical_case(
    case: &WaypointDirectObstacleDiscriminationFreshCaseV1,
    expected_id: &str,
    expected_profile: &str,
    center_fraction: f64,
    width_fraction: f64,
    height_fraction: f64,
) -> Result<()> {
    if case.case_id != expected_id
        || case.horizontal_span_m != 900.0
        || case.profile != expected_profile
        || case.center_fraction != center_fraction
        || case.width_fraction != width_fraction
        || case.height_fraction != height_fraction
        || case.source_pad_id != "pad_source"
        || case.target_pad_id != "pad_main"
        || case.probe_id != expected_id
        || case.scenario.sim.physics_hz != 120
        || case.scenario.sim.controller_hz != 60
        || case.scenario.sim.max_time_s != 90.0
        || case.scenario.sim.sample_hz != Some(10)
        || case.scenario.mission.transfer_route.is_some()
    {
        bail!("sealed case metadata/clock/route changed for {expected_id}");
    }
    Ok(())
}

fn local_clearing_source_binding(repo_root: &Path) -> Result<NominalDirectFlightSourceBindingV1> {
    let mut binding = source_binding(repo_root)?;
    for relative_path in [
        LOCAL_CLEARING_CANARY_PROTOCOL,
        LOCAL_CLEARING_CANARY_INPUTS,
        REPEAT_COMPARATOR_SOURCE,
        crate::canonical_initial_direct::CANONICAL_INITIAL_DIRECT_CANARY_PROTOCOL,
        crate::canonical_initial_direct::CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_MANIFEST,
        "fixtures/research/nominal_direct_operational_fresh_inputs_v1.json",
        crate::canonical_initial_direct::CANONICAL_INITIAL_DIRECT_CANARY_ADDITIONAL_MANIFEST,
        crate::nominal_airborne_direct::NOMINAL_AIRBORNE_DIRECT_CANARY_PROTOCOL,
    ] {
        if binding
            .files
            .iter()
            .any(|file| file.relative_path == relative_path)
        {
            continue;
        }
        let bytes = fs::read(repo_root.join(relative_path))
            .with_context(|| format!("read local-clearing source input {relative_path}"))?;
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

fn ensure_gate_contract(experiment: &mut LocalClearingExperimentV1) -> Result<()> {
    if experiment.gates.len() != GATE_IDS.len()
        || experiment
            .gates
            .iter()
            .zip(GATE_IDS)
            .any(|(gate, expected)| gate.gate_id != expected)
    {
        bail!("local experiment gate order differs from the sealed contract");
    }
    let closure = experiment
        .gates
        .last()
        .context("local experiment closure gate missing")?;
    if closure.status != "not_evaluated" {
        bail!("local experiment must leave closure_and_preservation not evaluated");
    }
    Ok(())
}

fn canary_artifact_identity(artifact: &LocalClearingCanaryArtifactV1) -> Result<String> {
    let mut canonical = artifact.clone();
    canonical.identity.clear();
    canonical.elapsed_wall_time_us = 0;
    nominal_direct_flight_identity(&canonical)
}

fn run_preservation(
    repo_root: &Path,
    output_root: &Path,
    selection: Option<&SelectionManifestV1>,
) -> Result<LocalClearingPreservationV1> {
    let selection = selection.context("validated selection manifest required for preservation")?;
    let preservation_root = output_root.join("preservation");
    fs::create_dir(&preservation_root).context("create preservation evidence directory")?;
    let canonical = run_canonical_preservation(
        repo_root,
        &preservation_root,
        &selection.canonical_reference_root,
    );
    let airborne = run_airborne_preservation(
        repo_root,
        &preservation_root,
        &selection.airborne_reference_root,
    );
    let source_rest = run_source_rest_preservation(
        repo_root,
        &preservation_root,
        &selection.source_rest_reference_root,
    );
    let passed = canonical.passed && airborne.passed && source_rest.passed;
    Ok(LocalClearingPreservationV1 {
        canonical_controls: canonical,
        airborne_continuations: airborne,
        source_rest_controls: source_rest,
        passed,
    })
}

fn run_canonical_preservation(
    repo_root: &Path,
    preservation_root: &Path,
    reference_root: &str,
) -> LocalClearingPreservationStageV1 {
    let started = Instant::now();
    let stage_id = "canonical_controls";
    let output_relative = "preservation/canonical";
    let output_root = preservation_root.join("canonical");
    let mut evidence = empty_preservation_stage(stage_id, reference_root, output_relative);
    let before = stage_source_binding(repo_root, &mut evidence, "before");
    evidence.source_binding_before_sha256 = before;
    match run_canonical_initial_direct_canary(repo_root, &output_root) {
        Ok(result) => {
            if !result.passed {
                evidence
                    .failures
                    .push("fresh canonical preservation canary failed".into());
            }
        }
        Err(error) => evidence.failures.push(format!(
            "fresh canonical preservation runner failed: {error:#}"
        )),
    }
    let reference_root_path = repo_root.join(reference_root);
    for case_id in CANONICAL_REFERENCE_CASES {
        let relative = format!("cases/{case_id}.final.json");
        evidence.comparisons.push(compare_json_payload(
            &reference_root_path,
            &output_root,
            &relative,
            &canonical_historical_exclusions(),
            format!("canonical:{case_id}"),
        ));
    }
    check_stage_source_binding(repo_root, &mut evidence, "canonical preservation");
    evidence.passed = evidence.failures.is_empty()
        && evidence
            .comparisons
            .iter()
            .all(|comparison| comparison.passed);
    evidence.status = if evidence.passed { "passed" } else { "failed" }.into();
    evidence.elapsed_wall_time_us = elapsed_us(started);
    evidence
}

fn run_airborne_preservation(
    repo_root: &Path,
    preservation_root: &Path,
    reference_root: &str,
) -> LocalClearingPreservationStageV1 {
    let started = Instant::now();
    let stage_id = "airborne_continuations";
    let output_relative = "preservation/airborne";
    let output_root = preservation_root.join("airborne");
    let mut evidence = empty_preservation_stage(stage_id, reference_root, output_relative);
    let before = stage_source_binding(repo_root, &mut evidence, "before");
    evidence.source_binding_before_sha256 = before;
    match run_nominal_airborne_direct_canary(repo_root, &output_root) {
        Ok(result) => {
            if !result.passed {
                evidence
                    .failures
                    .push("fresh airborne preservation canary failed".into());
            }
        }
        Err(error) => evidence.failures.push(format!(
            "fresh airborne preservation runner failed: {error:#}"
        )),
    }
    let reference_root_path = repo_root.join(reference_root);
    for case_id in AIRBORNE_REFERENCE_CASES {
        let relative = format!("cases/{case_id}.json");
        evidence.comparisons.push(compare_json_payload(
            &reference_root_path,
            &output_root,
            &relative,
            &airborne_historical_exclusions(),
            format!("airborne:{case_id}"),
        ));
    }
    check_stage_source_binding(repo_root, &mut evidence, "airborne preservation");
    evidence.passed = evidence.failures.is_empty()
        && evidence
            .comparisons
            .iter()
            .all(|comparison| comparison.passed);
    evidence.status = if evidence.passed { "passed" } else { "failed" }.into();
    evidence.elapsed_wall_time_us = elapsed_us(started);
    evidence
}

fn run_source_rest_preservation(
    repo_root: &Path,
    preservation_root: &Path,
    reference_root: &str,
) -> LocalClearingPreservationStageV1 {
    let started = Instant::now();
    let stage_id = "source_rest_controls";
    let output_relative = "preservation/source_rest";
    let output_root = preservation_root.join("source_rest");
    let mut evidence = empty_preservation_stage(stage_id, reference_root, output_relative);
    let before = stage_source_binding(repo_root, &mut evidence, "before");
    evidence.source_binding_before_sha256 = before;
    let archive_root = repo_root.join(DEFAULT_NOMINAL_DIRECT_FLIGHT_ARCHIVE_ROOT);
    match run_nominal_direct_flight_regression(repo_root, &archive_root, &output_root) {
        Ok(result) => {
            if !result.passed || result.cases.len() != 24 {
                evidence
                    .failures
                    .push("fresh 24-control source-rest regression failed".into());
            }
        }
        Err(error) => evidence.failures.push(format!(
            "fresh 24-control source-rest runner failed: {error:#}"
        )),
    }
    compare_source_rest_tree(
        repo_root,
        Path::new(reference_root),
        &output_root,
        &mut evidence,
    );
    check_stage_source_binding(repo_root, &mut evidence, "source-rest preservation");
    evidence.passed = evidence.failures.is_empty()
        && evidence
            .comparisons
            .iter()
            .all(|comparison| comparison.passed);
    evidence.status = if evidence.passed { "passed" } else { "failed" }.into();
    evidence.elapsed_wall_time_us = elapsed_us(started);
    evidence
}

fn empty_preservation_stage(
    stage_id: &str,
    reference_root: &str,
    observed_root: &str,
) -> LocalClearingPreservationStageV1 {
    LocalClearingPreservationStageV1 {
        stage_id: stage_id.into(),
        status: "running".into(),
        reference_root: reference_root.into(),
        observed_root: observed_root.into(),
        source_binding_before_sha256: None,
        source_binding_after_sha256: None,
        comparisons: Vec::new(),
        failures: Vec::new(),
        passed: false,
        elapsed_wall_time_us: 0,
    }
}

fn stage_source_binding(
    repo_root: &Path,
    evidence: &mut LocalClearingPreservationStageV1,
    timing: &str,
) -> Option<String> {
    match local_clearing_source_binding(repo_root) {
        Ok(binding) => Some(binding.identity_sha256),
        Err(error) => {
            evidence.failures.push(format!(
                "source closure binding {timing} preservation failed: {error:#}"
            ));
            None
        }
    }
}

fn check_stage_source_binding(
    repo_root: &Path,
    evidence: &mut LocalClearingPreservationStageV1,
    stage: &str,
) {
    let after = stage_source_binding(repo_root, evidence, "after");
    evidence.source_binding_after_sha256 = after;
    if evidence.source_binding_before_sha256.is_none()
        || evidence.source_binding_after_sha256.is_none()
    {
        return;
    }
    if evidence.source_binding_before_sha256 != evidence.source_binding_after_sha256 {
        evidence
            .failures
            .push(format!("source closure changed during {stage}"));
    }
}

fn compare_source_rest_tree(
    repo_root: &Path,
    reference_relative_root: &Path,
    observed_root: &Path,
    evidence: &mut LocalClearingPreservationStageV1,
) {
    let reference_root = repo_root.join(reference_relative_root);
    let mut reference_files = match collect_relative_files(&reference_root) {
        Ok(files) => files,
        Err(error) => {
            evidence
                .failures
                .push(format!("source-rest reference inventory failed: {error:#}"));
            return;
        }
    };
    let mut observed_files = match collect_relative_files(observed_root) {
        Ok(files) => files,
        Err(error) => {
            evidence
                .failures
                .push(format!("source-rest observed inventory failed: {error:#}"));
            return;
        }
    };
    reference_files.sort();
    observed_files.sort();
    let reference_set: BTreeSet<_> = reference_files.iter().cloned().collect();
    let observed_set: BTreeSet<_> = observed_files.iter().cloned().collect();
    for missing in reference_set.difference(&observed_set) {
        evidence.failures.push(format!(
            "source-rest output file missing: {}",
            missing.display()
        ));
    }
    for unexpected in observed_set.difference(&reference_set) {
        evidence.failures.push(format!(
            "source-rest output file was not in the reference inventory: {}",
            unexpected.display()
        ));
    }
    for relative in reference_set.intersection(&observed_set) {
        let name = relative
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let comparison = if name == "report.html" {
            compare_text_payload(
                &reference_root,
                observed_root,
                relative,
                format!("source-rest:{}", relative.display()),
            )
        } else {
            let patterns = source_rest_exclusions(relative);
            compare_json_payload(
                &reference_root,
                observed_root,
                &relative.to_string_lossy(),
                &patterns,
                format!("source-rest:{}", relative.display()),
            )
        };
        if !comparison.passed {
            evidence.failures.push(format!(
                "source-rest payload comparison failed at {}: {}",
                relative.display(),
                comparison
                    .failure
                    .as_deref()
                    .unwrap_or("deterministic payload differs")
            ));
        }
        evidence.comparisons.push(comparison);
    }
}

fn compare_preservation_repeat_trees(
    left_root: &Path,
    right_root: &Path,
) -> Result<Vec<LocalClearingPayloadComparisonV1>> {
    let left_preservation = left_root.join("preservation");
    let right_preservation = right_root.join("preservation");
    let left_files = collect_relative_files(&left_preservation)?;
    let right_files = collect_relative_files(&right_preservation)?;
    let left_set: BTreeSet<_> = left_files.iter().cloned().collect();
    let right_set: BTreeSet<_> = right_files.iter().cloned().collect();
    let mut comparisons = Vec::new();
    for missing in left_set.difference(&right_set) {
        let relative = missing.to_string_lossy().into_owned();
        let mut comparison = empty_comparison(
            format!("preservation-repeat:{relative}"),
            &relative,
            &relative,
            &[],
        );
        comparison.failure = Some("right repeat is missing this preservation file".into());
        comparisons.push(comparison);
    }
    for unexpected in right_set.difference(&left_set) {
        let relative = unexpected.to_string_lossy().into_owned();
        let mut comparison = empty_comparison(
            format!("preservation-repeat:{relative}"),
            &relative,
            &relative,
            &[],
        );
        comparison.failure = Some("right repeat has an unpaired preservation file".into());
        comparisons.push(comparison);
    }
    for relative in left_set.intersection(&right_set) {
        let relative_text = relative.to_string_lossy().into_owned();
        let comparison_id = format!("preservation-repeat:{relative_text}");
        let extension = relative
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let comparison = if extension == "html" {
            compare_text_payload(
                &left_preservation,
                &right_preservation,
                relative,
                comparison_id,
            )
        } else if extension == "json" {
            compare_repeat_json_payload(
                &left_preservation,
                &right_preservation,
                relative,
                &repeat_preservation_exclusions(relative),
                comparison_id,
            )
        } else {
            compare_repeat_binary_payload(
                &left_preservation,
                &right_preservation,
                relative,
                comparison_id,
            )
        };
        comparisons.push(comparison);
    }
    comparisons.sort_by(|left, right| left.comparison_id.cmp(&right.comparison_id));
    Ok(comparisons)
}

fn repeat_preservation_exclusions(relative: &Path) -> Vec<String> {
    let stage = relative
        .components()
        .next()
        .and_then(|value| value.as_os_str().to_str());
    let inside = stage
        .and_then(|value| relative.strip_prefix(value).ok())
        .unwrap_or(relative);
    match stage {
        Some("canonical") => canonical_repeat_exclusions(inside),
        Some("airborne") => airborne_repeat_exclusions(inside),
        Some("source_rest") => source_rest_repeat_exclusions(inside),
        _ => Vec::new(),
    }
}

fn canonical_repeat_exclusions(relative: &Path) -> Vec<String> {
    let name = relative
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if relative
        .parent()
        .is_some_and(|parent| parent == Path::new("cases"))
    {
        if name.ends_with(".terrain_twin_blocking.json") || name.ends_with(".terrain_twin_low.json")
        {
            return vec!["/elapsed_wall_time_us".into()];
        }
        if name.contains(".airborne_") {
            return vec![
                "/search_wall_time_us".into(),
                "/audit_wall_time_us".into(),
                "/stitched_replay_wall_time_us".into(),
            ];
        }
        if name.ends_with(".source_rest.json")
            || name.ends_with(".final.json")
            || name.ends_with(".not_evaluated.json")
        {
            return canonical_case_timing_exclusions();
        }
    }
    if relative == Path::new("summary.json") {
        return [
            "/cases/*/elapsed_wall_time_us",
            "/cases/*/terrain_twins/*/elapsed_wall_time_us",
            "/cases/*/airborne_captures/*/search_wall_time_us",
            "/cases/*/airborne_captures/*/audit_wall_time_us",
            "/cases/*/airborne_captures/*/stitched_replay_wall_time_us",
            "/discriminator/flat_control/elapsed_wall_time_us",
            "/discriminator/late_broad/elapsed_wall_time_us",
            "/discriminator/old_terrain_aware_source_only/elapsed_wall_time_us",
            "/discriminator/old_terrain_aware_source_only/compute/generation_wall_time_us",
            "/discriminator/old_terrain_aware_source_only/compute/selected_verification_wall_time_us",
            "/discriminator/old_terrain_aware_source_only/compute/ordinary_execution_wall_time_us",
            "/discriminator/old_terrain_aware_source_only/compute/action_replay_wall_time_us",
            "/discriminator/old_terrain_aware_source_only/compute/artifact_writing_wall_time_us",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
    }
    if relative == Path::new("stages/900m_flat_canonical.json")
        || relative == Path::new("stages/900m_late_broad_canonical.json")
    {
        return vec!["/elapsed_wall_time_us".into()];
    }
    if relative == Path::new("stages/900m_old_policy_source_only.json") {
        return old_policy_timing_exclusions("");
    }
    if relative == Path::new("stages/900m_discriminator_final.json") {
        return [
            "/flat_control/elapsed_wall_time_us",
            "/late_broad/elapsed_wall_time_us",
            "/old_terrain_aware_source_only/elapsed_wall_time_us",
            "/old_terrain_aware_source_only/compute/generation_wall_time_us",
            "/old_terrain_aware_source_only/compute/selected_verification_wall_time_us",
            "/old_terrain_aware_source_only/compute/ordinary_execution_wall_time_us",
            "/old_terrain_aware_source_only/compute/action_replay_wall_time_us",
            "/old_terrain_aware_source_only/compute/artifact_writing_wall_time_us",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
    }
    Vec::new()
}

fn canonical_case_timing_exclusions() -> Vec<String> {
    [
        "/elapsed_wall_time_us",
        "/terrain_twins/*/elapsed_wall_time_us",
        "/airborne_captures/*/search_wall_time_us",
        "/airborne_captures/*/audit_wall_time_us",
        "/airborne_captures/*/stitched_replay_wall_time_us",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn old_policy_timing_exclusions(prefix: &str) -> Vec<String> {
    [
        "elapsed_wall_time_us",
        "compute/generation_wall_time_us",
        "compute/selected_verification_wall_time_us",
        "compute/ordinary_execution_wall_time_us",
        "compute/action_replay_wall_time_us",
        "compute/artifact_writing_wall_time_us",
    ]
    .into_iter()
    .map(|path| format!("/{prefix}{path}"))
    .collect()
}

fn airborne_repeat_exclusions(relative: &Path) -> Vec<String> {
    if relative == Path::new("cases/operational_flat_span_685.json")
        || relative == Path::new("cases/operational_flat_span_845.json")
        || relative == Path::new("cases/operational_uphill_span_845.json")
        || relative == Path::new("cases/operational_downhill_span_845.json")
    {
        return [
            "/baseline_compute/generation_wall_time_us",
            "/baseline_compute/selected_verification_wall_time_us",
            "/baseline_compute/ordinary_execution_wall_time_us",
            "/baseline_compute/action_replay_wall_time_us",
            "/baseline_compute/artifact_writing_wall_time_us",
            "/captures/*/search_wall_time_us",
            "/captures/*/nominal_audit_wall_time_us",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
    }
    Vec::new()
}

fn source_rest_repeat_exclusions(relative: &Path) -> Vec<String> {
    let name = relative
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    match (relative.components().count(), name) {
        (1, "summary.json") => [
            "/compute/preflight_wall_time_us",
            "/compute/measured_gate_wall_time_us",
            "/compute/case_wall_time_us",
            "/cases/*/case_wall_time_us",
            "/cases/*/compute/generation_wall_time_us",
            "/cases/*/compute/selected_verification_wall_time_us",
            "/cases/*/compute/ordinary_execution_wall_time_us",
            "/cases/*/compute/action_replay_wall_time_us",
            "/cases/*/compute/artifact_writing_wall_time_us",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        (_, "summary.json") => [
            "/compute/generation_wall_time_us",
            "/compute/selected_verification_wall_time_us",
            "/compute/ordinary_execution_wall_time_us",
            "/compute/action_replay_wall_time_us",
            "/compute/artifact_writing_wall_time_us",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        (_, "performance.json") => [
            "/generation_wall_time_us",
            "/selected_verification_wall_time_us",
            "/ordinary_execution_wall_time_us",
            "/action_replay_wall_time_us",
            "/artifact_writing_wall_time_us",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        (_, "run_performance.json") => vec!["/wall_time_us".into(), "/thread_cpu_time_us".into()],
        (_, "controller_updates.json") => vec!["/*/compute_time_us".into()],
        _ => Vec::new(),
    }
}

fn compare_repeat_json_payload(
    left_root: &Path,
    right_root: &Path,
    relative_path: &Path,
    exclusions: &[String],
    comparison_id: String,
) -> LocalClearingPayloadComparisonV1 {
    let relative_text = relative_path.to_string_lossy().into_owned();
    let mut declared_exclusions = exclusions.to_vec();
    declared_exclusions.push("<exact output-root path literal>".into());
    let mut comparison = empty_comparison(
        comparison_id,
        &relative_text,
        &relative_text,
        &declared_exclusions,
    );
    let reference_bytes = match fs::read(left_root.join(relative_path)) {
        Ok(bytes) => bytes,
        Err(error) => {
            comparison.failure = Some(format!("left repeat read failed: {error}"));
            return comparison;
        }
    };
    let (normalized_reference, reference_applied) = match normalized_repeat_json_digest(
        reference_bytes,
        exclusions,
        &left_root.to_string_lossy(),
    ) {
        Ok((raw_hash, normalized_hash, applied)) => {
            comparison.reference_file_sha256 = Some(raw_hash);
            (normalized_hash, applied)
        }
        Err(error) => {
            comparison.failure = Some(format!("left repeat normalization failed: {error:#}"));
            return comparison;
        }
    };
    let observed_bytes = match fs::read(right_root.join(relative_path)) {
        Ok(bytes) => bytes,
        Err(error) => {
            comparison.failure = Some(format!("right repeat read failed: {error}"));
            return comparison;
        }
    };
    let (normalized_observed, observed_applied) = match normalized_repeat_json_digest(
        observed_bytes,
        exclusions,
        &right_root.to_string_lossy(),
    ) {
        Ok((raw_hash, normalized_hash, applied)) => {
            comparison.observed_file_sha256 = Some(raw_hash);
            (normalized_hash, applied)
        }
        Err(error) => {
            comparison.failure = Some(format!("right repeat normalization failed: {error:#}"));
            return comparison;
        }
    };
    comparison.applied_exclusion_paths = reference_applied.clone();
    if reference_applied != observed_applied {
        comparison.failure = Some("repeat JSON payloads matched different normalized paths".into());
        return comparison;
    }
    comparison.normalized_reference_sha256 = Some(normalized_reference.clone());
    comparison.normalized_observed_sha256 = Some(normalized_observed.clone());
    comparison.passed = normalized_reference == normalized_observed;
    if !comparison.passed {
        comparison.failure = Some(
            "complete repeat JSON differs after only declared timing/root-path normalization"
                .into(),
        );
    }
    comparison
}

fn normalized_repeat_json_digest(
    bytes: Vec<u8>,
    exclusions: &[String],
    output_root: &str,
) -> Result<(String, String, Vec<String>)> {
    let raw_hash = sha256_bytes(&bytes)?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)?;
    drop(bytes);
    let mut applied = Vec::new();
    remove_explicit_paths(&mut value, exclusions, &mut Vec::new(), &mut applied);
    normalize_json_output_roots(&mut value, output_root, &mut Vec::new(), &mut applied);
    applied.sort();
    let normalized = serde_json::to_vec(&value)?;
    Ok((raw_hash, sha256_bytes(&normalized)?, applied))
}

fn compare_repeat_binary_payload(
    left_root: &Path,
    right_root: &Path,
    relative_path: &Path,
    comparison_id: String,
) -> LocalClearingPayloadComparisonV1 {
    let relative = relative_path.to_string_lossy().into_owned();
    let mut comparison = empty_comparison(comparison_id, &relative, &relative, &[]);
    let left = match fs::read(left_root.join(relative_path)) {
        Ok(bytes) => bytes,
        Err(error) => {
            comparison.failure = Some(format!("left repeat read failed: {error}"));
            return comparison;
        }
    };
    let right = match fs::read(right_root.join(relative_path)) {
        Ok(bytes) => bytes,
        Err(error) => {
            comparison.failure = Some(format!("right repeat read failed: {error}"));
            return comparison;
        }
    };
    comparison.reference_file_sha256 = sha256_bytes(&left).ok();
    comparison.observed_file_sha256 = sha256_bytes(&right).ok();
    comparison.normalized_reference_sha256 = comparison.reference_file_sha256.clone();
    comparison.normalized_observed_sha256 = comparison.observed_file_sha256.clone();
    comparison.passed = left == right;
    if !comparison.passed {
        comparison.failure = Some("complete repeat binary payload differs".into());
    }
    comparison
}

fn normalize_json_output_roots(
    value: &mut serde_json::Value,
    output_root: &str,
    path: &mut Vec<String>,
    applied: &mut Vec<String>,
) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, child) in object.iter_mut() {
                path.push(key.clone());
                normalize_json_output_roots(child, output_root, path, applied);
                path.pop();
            }
        }
        serde_json::Value::Array(array) => {
            for (index, child) in array.iter_mut().enumerate() {
                path.push(index.to_string());
                normalize_json_output_roots(child, output_root, path, applied);
                path.pop();
            }
        }
        serde_json::Value::String(text)
            if !output_root.is_empty() && text.contains(output_root) =>
        {
            *text = text.replace(output_root, "<OUTPUT_ROOT>");
            applied.push(json_pointer(path));
        }
        _ => {}
    }
}

fn collect_relative_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn visit(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(current).with_context(|| format!("list {}", current.display()))? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                visit(root, &path, files)?;
            } else if entry.file_type()?.is_file() {
                files.push(path.strip_prefix(root)?.to_path_buf());
            }
        }
        Ok(())
    }
    if !root.is_dir() {
        bail!("evidence directory does not exist: {}", root.display());
    }
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn source_rest_exclusions(relative: &Path) -> Vec<String> {
    let name = relative
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    match (relative.components().count(), name) {
        (1, "preflight.json") => vec!["/source_binding".into()],
        (1, "summary.json") => vec![
            "/identity".into(),
            "/source_binding_before".into(),
            "/source_binding_after".into(),
            "/source_checks/*/before_identity_sha256".into(),
            "/source_checks/*/after_identity_sha256".into(),
            "/compute/preflight_wall_time_us".into(),
            "/compute/measured_gate_wall_time_us".into(),
            "/compute/case_wall_time_us".into(),
            "/cases/*/case_wall_time_us".into(),
            "/cases/*/compute/generation_wall_time_us".into(),
            "/cases/*/compute/selected_verification_wall_time_us".into(),
            "/cases/*/compute/ordinary_execution_wall_time_us".into(),
            "/cases/*/compute/action_replay_wall_time_us".into(),
            "/cases/*/compute/artifact_writing_wall_time_us".into(),
        ],
        (_, "summary.json") => vec![
            "/identity".into(),
            "/compute/generation_wall_time_us".into(),
            "/compute/selected_verification_wall_time_us".into(),
            "/compute/ordinary_execution_wall_time_us".into(),
            "/compute/action_replay_wall_time_us".into(),
            "/compute/artifact_writing_wall_time_us".into(),
        ],
        (_, "performance.json") => vec![
            "/generation_wall_time_us".into(),
            "/selected_verification_wall_time_us".into(),
            "/ordinary_execution_wall_time_us".into(),
            "/action_replay_wall_time_us".into(),
            "/artifact_writing_wall_time_us".into(),
        ],
        (_, "run_performance.json") => vec!["/wall_time_us".into(), "/thread_cpu_time_us".into()],
        (_, "controller_updates.json") => vec!["/*/compute_time_us".into()],
        _ => Vec::new(),
    }
}

fn canonical_historical_exclusions() -> Vec<String> {
    [
        "/identity",
        "/source_binding_before_sha256",
        "/source_binding_after_sha256",
        "/elapsed_wall_time_us",
        "/terrain_twins/*/identity",
        "/terrain_twins/*/source_binding_before_sha256",
        "/terrain_twins/*/source_binding_after_sha256",
        "/terrain_twins/*/elapsed_wall_time_us",
        "/airborne_captures/*/identity",
        "/airborne_captures/*/source_binding_before_sha256",
        "/airborne_captures/*/source_binding_after_sha256",
        "/airborne_captures/*/search_wall_time_us",
        "/airborne_captures/*/audit_wall_time_us",
        "/airborne_captures/*/stitched_replay_wall_time_us",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn airborne_historical_exclusions() -> Vec<String> {
    [
        "/identity",
        "/baseline_compute/generation_wall_time_us",
        "/baseline_compute/selected_verification_wall_time_us",
        "/baseline_compute/ordinary_execution_wall_time_us",
        "/baseline_compute/action_replay_wall_time_us",
        "/baseline_compute/artifact_writing_wall_time_us",
        "/captures/*/search_wall_time_us",
        "/captures/*/nominal_audit_wall_time_us",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn compare_json_payload(
    reference_root: &Path,
    observed_root: &Path,
    relative_path: &str,
    exclusion_patterns: &[String],
    comparison_id: String,
) -> LocalClearingPayloadComparisonV1 {
    let mut comparison = empty_comparison(
        comparison_id,
        relative_path,
        relative_path,
        exclusion_patterns,
    );
    let relative = match safe_relative_path(relative_path) {
        Ok(relative) => relative,
        Err(error) => {
            comparison.failure = Some(format!("unsafe payload path: {error:#}"));
            return comparison;
        }
    };
    let reference = reference_root.join(&relative);
    let observed = observed_root.join(&relative);
    let reference_bytes = match fs::read(&reference) {
        Ok(bytes) => bytes,
        Err(error) => {
            comparison.failure = Some(format!("reference read failed: {error}"));
            return comparison;
        }
    };
    let observed_bytes = match fs::read(&observed) {
        Ok(bytes) => bytes,
        Err(error) => {
            comparison.failure = Some(format!("observed read failed: {error}"));
            return comparison;
        }
    };
    comparison.reference_file_sha256 = sha256_bytes(&reference_bytes).ok();
    comparison.observed_file_sha256 = sha256_bytes(&observed_bytes).ok();
    let mut expected: serde_json::Value = match serde_json::from_slice(&reference_bytes) {
        Ok(value) => value,
        Err(error) => {
            comparison.failure = Some(format!("reference JSON parse failed: {error}"));
            return comparison;
        }
    };
    let mut actual: serde_json::Value = match serde_json::from_slice(&observed_bytes) {
        Ok(value) => value,
        Err(error) => {
            comparison.failure = Some(format!("observed JSON parse failed: {error}"));
            return comparison;
        }
    };
    let mut expected_applied = Vec::new();
    let mut observed_applied = Vec::new();
    remove_explicit_paths(
        &mut expected,
        exclusion_patterns,
        &mut Vec::new(),
        &mut expected_applied,
    );
    remove_explicit_paths(
        &mut actual,
        exclusion_patterns,
        &mut Vec::new(),
        &mut observed_applied,
    );
    expected_applied.sort();
    observed_applied.sort();
    comparison.applied_exclusion_paths = expected_applied.clone();
    if expected_applied != observed_applied {
        comparison.failure =
            Some("historical and observed schemas matched different exclusion paths".into());
        return comparison;
    }
    let expected_bytes = match serde_json::to_vec(&expected) {
        Ok(bytes) => bytes,
        Err(error) => {
            comparison.failure = Some(format!(
                "normalized reference serialization failed: {error}"
            ));
            return comparison;
        }
    };
    let actual_bytes = match serde_json::to_vec(&actual) {
        Ok(bytes) => bytes,
        Err(error) => {
            comparison.failure = Some(format!("normalized observed serialization failed: {error}"));
            return comparison;
        }
    };
    comparison.normalized_reference_sha256 = sha256_bytes(&expected_bytes).ok();
    comparison.normalized_observed_sha256 = sha256_bytes(&actual_bytes).ok();
    comparison.passed = expected == actual;
    if !comparison.passed {
        comparison.failure = Some(
            "complete deterministic JSON payload differs after only declared exclusions".into(),
        );
    }
    comparison
}

fn compare_text_payload(
    reference_root: &Path,
    observed_root: &Path,
    relative_path: &Path,
    comparison_id: String,
) -> LocalClearingPayloadComparisonV1 {
    let relative = relative_path.to_string_lossy().into_owned();
    let report_exclusions = report_html_exclusions();
    let mut comparison = empty_comparison(comparison_id, &relative, &relative, &report_exclusions);
    let safe_relative = match safe_relative_path(&relative) {
        Ok(relative) => relative,
        Err(error) => {
            comparison.failure = Some(format!("unsafe report path: {error:#}"));
            return comparison;
        }
    };
    let reference_bytes = match fs::read(reference_root.join(&safe_relative)) {
        Ok(bytes) => bytes,
        Err(error) => {
            comparison.failure = Some(format!("reference read failed: {error}"));
            return comparison;
        }
    };
    let observed_bytes = match fs::read(observed_root.join(&safe_relative)) {
        Ok(bytes) => bytes,
        Err(error) => {
            comparison.failure = Some(format!("observed read failed: {error}"));
            return comparison;
        }
    };
    comparison.reference_file_sha256 = sha256_bytes(&reference_bytes).ok();
    comparison.observed_file_sha256 = sha256_bytes(&observed_bytes).ok();
    let reference_text = String::from_utf8_lossy(&reference_bytes);
    let observed_text = String::from_utf8_lossy(&observed_bytes);
    let expected_root = reference_root.to_string_lossy();
    let actual_root = observed_root.to_string_lossy();
    let (normalized_reference, reference_exclusions) =
        match normalize_report_html(&reference_text, expected_root.as_ref()) {
            Ok(value) => value,
            Err(error) => {
                comparison.failure =
                    Some(format!("reference report normalization failed: {error:#}"));
                return comparison;
            }
        };
    let (normalized_observed, observed_exclusions) =
        match normalize_report_html(&observed_text, actual_root.as_ref()) {
            Ok(value) => value,
            Err(error) => {
                comparison.failure =
                    Some(format!("observed report normalization failed: {error:#}"));
                return comparison;
            }
        };
    comparison.applied_exclusion_paths = reference_exclusions.clone();
    if reference_exclusions != observed_exclusions {
        comparison.failure =
            Some("reference and observed report normalized different timing paths".into());
        return comparison;
    }
    comparison.passed = normalized_reference == normalized_observed;
    comparison.normalized_reference_sha256 = sha256_bytes(normalized_reference.as_bytes()).ok();
    comparison.normalized_observed_sha256 = sha256_bytes(normalized_observed.as_bytes()).ok();
    if !comparison.passed {
        comparison.failure = Some(
            "report payload differs after declared timing and exact output-root normalization"
                .into(),
        );
    }
    comparison
}

fn report_html_exclusions() -> Vec<String> {
    [
        "<exact output-root path literal>",
        "/plannerCompute/wallTimeUs",
        "/runPerformance/wallTimeMs",
        "/runPerformance/threadCpuTimeMs",
        "/runPerformance/cpuTimePerTickUs",
        "/runPerformance/simRateX",
        "/runPerformance/physicsStepsPerS",
        "/samples/*/computeTimeMs",
        "/botStats/totalComputeMs",
        "/botStats/meanComputeMs",
        "/botStats/p95ComputeMs",
        "/botStats/maxComputeMs",
        "/botStats/controlDutyCyclePct",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn normalize_report_html(html: &str, output_root: &str) -> Result<(String, Vec<String>)> {
    const REPORT_DATA_PREFIX: &str = "const reportData = ";
    const REPORT_DATA_SUFFIX: &str = ";\n    const paperBg";
    let data_start = html
        .find(REPORT_DATA_PREFIX)
        .context("report data start marker is missing")?
        + REPORT_DATA_PREFIX.len();
    let data_end = data_start
        + html[data_start..]
            .find(REPORT_DATA_SUFFIX)
            .context("report data end marker is missing")?;
    let mut report_data: serde_json::Value = serde_json::from_str(&html[data_start..data_end])
        .context("parse embedded reportData JSON")?;
    let exclusions = report_html_exclusions();
    let mut applied = Vec::new();
    remove_explicit_paths(&mut report_data, &exclusions, &mut Vec::new(), &mut applied);
    applied.sort();
    let normalized_json = serde_json::to_string(&report_data)?
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e");
    let mut normalized = String::with_capacity(html.len());
    normalized.push_str(&html[..data_start]);
    normalized.push_str(&normalized_json);
    normalized.push_str(&html[data_end..]);
    if !output_root.is_empty() && normalized.contains(output_root) {
        normalized = normalized.replace(output_root, "<OUTPUT_ROOT>");
        applied.push("<exact output-root path literal>".into());
    }
    applied.sort();
    Ok((normalized, applied))
}

fn empty_comparison(
    comparison_id: String,
    reference_relative_path: &str,
    observed_relative_path: &str,
    exclusion_patterns: &[String],
) -> LocalClearingPayloadComparisonV1 {
    LocalClearingPayloadComparisonV1 {
        comparison_id,
        reference_relative_path: reference_relative_path.into(),
        observed_relative_path: observed_relative_path.into(),
        reference_file_sha256: None,
        observed_file_sha256: None,
        normalized_reference_sha256: None,
        normalized_observed_sha256: None,
        exclusion_patterns: exclusion_patterns.to_vec(),
        applied_exclusion_paths: Vec::new(),
        passed: false,
        failure: None,
    }
}

fn remove_explicit_paths(
    value: &mut serde_json::Value,
    patterns: &[String],
    path: &mut Vec<String>,
    applied: &mut Vec<String>,
) {
    match value {
        serde_json::Value::Object(object) => {
            let keys: Vec<String> = object.keys().cloned().collect();
            for key in keys {
                path.push(key.clone());
                let pointer = json_pointer(path);
                if patterns
                    .iter()
                    .any(|pattern| path_pattern_matches(pattern, path))
                {
                    object.remove(&key);
                    applied.push(pointer);
                } else if let Some(child) = object.get_mut(&key) {
                    remove_explicit_paths(child, patterns, path, applied);
                }
                path.pop();
            }
        }
        serde_json::Value::Array(array) => {
            for (index, child) in array.iter_mut().enumerate() {
                path.push(index.to_string());
                remove_explicit_paths(child, patterns, path, applied);
                path.pop();
            }
        }
        _ => {}
    }
}

fn path_pattern_matches(pattern: &str, path: &[String]) -> bool {
    let parts: Vec<&str> = pattern.trim_start_matches('/').split('/').collect();
    parts.len() == path.len()
        && parts.iter().zip(path).all(|(expected, observed)| {
            *expected == observed || (*expected == "*" && observed.parse::<usize>().is_ok())
        })
}

fn json_pointer(path: &[String]) -> String {
    if path.is_empty() {
        return String::new();
    }
    format!(
        "/{}",
        path.iter()
            .map(|part| part.replace('~', "~0").replace('/', "~1"))
            .collect::<Vec<_>>()
            .join("/")
    )
}

/// Compare two finished canary roots. Source closure must be byte-identical;
/// comparison removes only named timing/root-path fields and compares the
/// complete experiment plus every retained normalized preservation payload.
fn experiment_identity_valid(
    root: &Path,
    artifact: &LocalClearingCanaryArtifactV1,
) -> Result<bool> {
    match (
        artifact.experiment_relative_path.as_deref(),
        artifact.experiment_identity.as_deref(),
    ) {
        (None, None) => Ok(true),
        (Some("experiment.json"), Some(expected_identity)) => {
            let experiment: LocalClearingExperimentV1 = read_json(&root.join("experiment.json"))?;
            Ok(nominal_direct_flight_identity(&experiment)? == expected_identity)
        }
        _ => Ok(false),
    }
}

pub fn compare_local_clearing_canary_repeats(
    left_root: &Path,
    right_root: &Path,
) -> Result<LocalClearingRepeatComparisonV1> {
    let left: LocalClearingCanaryArtifactV1 = read_json(&left_root.join("summary.json"))?;
    let right: LocalClearingCanaryArtifactV1 = read_json(&right_root.join("summary.json"))?;
    let mut failures = Vec::new();
    let summary_identities_valid = canary_artifact_identity(&left)? == left.identity
        && canary_artifact_identity(&right)? == right.identity;
    if !summary_identities_valid {
        failures.push(
            "one or both canary summary identities do not match their complete envelope".into(),
        );
    }
    let source_bindings_match = left.source_binding_before.is_some()
        && left.source_binding_before == left.source_binding_after
        && right.source_binding_before.is_some()
        && right.source_binding_before == right.source_binding_after
        && left.source_binding_before == right.source_binding_before;
    if !source_bindings_match {
        failures.push("repeat source closures differ or changed during a run".into());
    }

    let preflight_payload_matches = compare_same_json_file(
        &left_root.join("preflight.json"),
        &right_root.join("preflight.json"),
        &[],
    )?;
    if !preflight_payload_matches {
        failures.push("repeat sealed request/preflight payloads differ".into());
    }
    let experiment_payload_matches = match (
        left.experiment_relative_path.as_deref(),
        right.experiment_relative_path.as_deref(),
    ) {
        (Some(left_path), Some(right_path)) if left_path == right_path => compare_same_json_file(
            &left_root.join(left_path),
            &right_root.join(right_path),
            &[],
        )?,
        (None, None) => true,
        _ => false,
    };
    if !experiment_payload_matches {
        failures.push("repeat complete local experiment payloads differ".into());
    }
    let experiment_identities_valid = experiment_identity_valid(left_root, &left)?
        && experiment_identity_valid(right_root, &right)?;
    if !experiment_identities_valid {
        failures.push("one or both complete experiment identity references are invalid".into());
    }
    let preservation_payload_matches = compare_same_json_file(
        &left_root.join("preservation.json"),
        &right_root.join("preservation.json"),
        &[
            "/canonical_controls/elapsed_wall_time_us".into(),
            "/airborne_continuations/elapsed_wall_time_us".into(),
            "/source_rest_controls/elapsed_wall_time_us".into(),
            "/canonical_controls/comparisons/*/observed_file_sha256".into(),
            "/airborne_continuations/comparisons/*/observed_file_sha256".into(),
            "/source_rest_controls/comparisons/*/observed_file_sha256".into(),
        ],
    )?;
    if !preservation_payload_matches {
        failures.push("repeat full normalized preservation comparisons differ".into());
    }
    let preservation_ledgers_bind_files = verify_preservation_ledger_payloads(left_root)?
        && verify_preservation_ledger_payloads(right_root)?;
    if !preservation_ledgers_bind_files {
        failures.push("one or both preservation ledgers do not bind their retained files".into());
    }
    let preservation_file_comparisons = compare_preservation_repeat_trees(left_root, right_root)?;
    let preservation_files_match = preservation_file_comparisons
        .iter()
        .all(|comparison| comparison.passed);
    if !preservation_files_match {
        failures.push(
            "repeat actual preservation output files differ after declared normalization".into(),
        );
    }
    let mut left_summary = serde_json::to_value(&left)?;
    let mut right_summary = serde_json::to_value(&right)?;
    let repeat_exclusions = vec!["/elapsed_wall_time_us".to_owned()];
    let mut left_applied = Vec::new();
    let mut right_applied = Vec::new();
    remove_explicit_paths(
        &mut left_summary,
        &repeat_exclusions,
        &mut Vec::new(),
        &mut left_applied,
    );
    remove_explicit_paths(
        &mut right_summary,
        &repeat_exclusions,
        &mut Vec::new(),
        &mut right_applied,
    );
    let summary_payload_matches =
        summary_identities_valid && left_applied == right_applied && left_summary == right_summary;
    if !summary_payload_matches {
        failures.push("repeat canary envelope differs after declared timing normalization".into());
    }
    Ok(LocalClearingRepeatComparisonV1 {
        left_root: left_root.to_string_lossy().into_owned(),
        right_root: right_root.to_string_lossy().into_owned(),
        source_bindings_match,
        preflight_payload_matches,
        experiment_payload_matches,
        experiment_identities_valid,
        preservation_payload_matches,
        preservation_ledgers_bind_files,
        preservation_files_match,
        preservation_file_comparisons,
        summary_identities_valid,
        summary_payload_matches,
        passed: failures.is_empty(),
        failures,
    })
}

fn verify_preservation_ledger_payloads(root: &Path) -> Result<bool> {
    let ledger: LocalClearingPreservationV1 = read_json(&root.join("preservation.json"))?;
    let stages = [
        (
            "canonical_controls",
            "preservation/canonical",
            &ledger.canonical_controls,
            CANONICAL_REFERENCE_CASES
                .iter()
                .map(|case_id| format!("cases/{case_id}.final.json"))
                .collect::<Vec<_>>(),
        ),
        (
            "airborne_continuations",
            "preservation/airborne",
            &ledger.airborne_continuations,
            AIRBORNE_REFERENCE_CASES
                .iter()
                .map(|case_id| format!("cases/{case_id}.json"))
                .collect::<Vec<_>>(),
        ),
        (
            "source_rest_controls",
            "preservation/source_rest",
            &ledger.source_rest_controls,
            Vec::new(),
        ),
    ];
    let stages_passed = stages.iter().all(|(_, _, stage, _)| stage.passed);
    let mut valid = ledger.passed == stages_passed;
    for (stage_id, observed_root, stage, required_paths) in stages {
        valid &= verify_preservation_stage_payloads(root, stage, stage_id, observed_root)?;
        let recorded_paths: BTreeSet<_> = stage
            .comparisons
            .iter()
            .map(|comparison| comparison.observed_relative_path.as_str())
            .collect();
        valid &= required_paths
            .iter()
            .all(|required| recorded_paths.contains(required.as_str()));
        if stage_id == "source_rest_controls" {
            let stage_root = root.join(safe_relative_path(observed_root)?);
            let actual_paths: BTreeSet<_> = collect_relative_files(&stage_root)?
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            valid &= actual_paths == recorded_paths.into_iter().map(str::to_owned).collect();
        }
    }
    Ok(valid)
}

fn verify_preservation_stage_payloads(
    root: &Path,
    stage: &LocalClearingPreservationStageV1,
    expected_stage_id: &str,
    expected_observed_root: &str,
) -> Result<bool> {
    if stage.stage_id != expected_stage_id || stage.observed_root != expected_observed_root {
        return Ok(false);
    }
    let status_matches = stage.status == if stage.passed { "passed" } else { "failed" };
    let observed_root = root.join(safe_relative_path(&stage.observed_root)?);
    if !observed_root.is_dir() {
        return Ok(false);
    }
    let mut valid = status_matches;
    let mut recorded_paths = BTreeSet::new();
    for comparison in &stage.comparisons {
        let relative = safe_relative_path(&comparison.observed_relative_path)?;
        if !recorded_paths.insert(relative.clone()) {
            valid = false;
            continue;
        }
        let observed_path = observed_root.join(&relative);
        let bytes = match fs::read(&observed_path) {
            Ok(bytes) => bytes,
            Err(_) => {
                valid &= comparison.observed_file_sha256.is_none()
                    && comparison.normalized_observed_sha256.is_none()
                    && !comparison.passed;
                continue;
            }
        };
        let raw_hash = sha256_bytes(&bytes)?;
        if comparison.observed_file_sha256.as_deref() != Some(raw_hash.as_str()) {
            valid = false;
            continue;
        }
        let Some(expected_normalized_hash) = comparison.normalized_observed_sha256.as_deref()
        else {
            valid &= !comparison.passed;
            continue;
        };
        let (normalized_hash, applied_paths) = if relative
            .file_name()
            .is_some_and(|name| name == "report.html")
        {
            let html =
                std::str::from_utf8(&bytes).context("preserved report.html is not valid UTF-8")?;
            let (normalized, applied) =
                normalize_report_html(html, &observed_root.to_string_lossy())?;
            (sha256_bytes(normalized.as_bytes())?, applied)
        } else {
            let mut value: serde_json::Value = serde_json::from_slice(&bytes)
                .with_context(|| format!("parse preserved JSON {}", observed_path.display()))?;
            let mut applied = Vec::new();
            remove_explicit_paths(
                &mut value,
                &comparison.exclusion_patterns,
                &mut Vec::new(),
                &mut applied,
            );
            applied.sort();
            (sha256_bytes(&serde_json::to_vec(&value)?)?, applied)
        };
        if normalized_hash != expected_normalized_hash
            || applied_paths != comparison.applied_exclusion_paths
        {
            valid = false;
        }
    }
    Ok(valid)
}

fn compare_same_json_file(left: &Path, right: &Path, exclusions: &[String]) -> Result<bool> {
    let mut left_value: serde_json::Value = read_json(left)?;
    let mut right_value: serde_json::Value = read_json(right)?;
    let mut left_applied = Vec::new();
    let mut right_applied = Vec::new();
    remove_explicit_paths(
        &mut left_value,
        exclusions,
        &mut Vec::new(),
        &mut left_applied,
    );
    remove_explicit_paths(
        &mut right_value,
        exclusions,
        &mut Vec::new(),
        &mut right_applied,
    );
    Ok(left_applied == right_applied && left_value == right_value)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("read JSON artifact {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parse JSON artifact {}", path.display()))
}

fn elapsed_us(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

fn safe_relative_path(path: &str) -> Result<PathBuf> {
    let relative = Path::new(path);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("reference path is not a safe repository-relative path: {path}");
    }
    Ok(relative.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("local-clearing-canary-{label}-{nonce}"))
    }

    #[test]
    fn sealed_protocol_and_selection_manifest_match_frozen_hashes() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let prepared = prepare_inputs(
            repo_root,
            Some(local_clearing_source_binding(repo_root).unwrap()),
        );
        assert!(
            prepared.preflight.ready,
            "{:?}",
            prepared.preflight.failures
        );
        assert_eq!(
            prepared.preflight.requests.as_ref().unwrap().flat_case_id,
            FLAT_CASE_ID
        );
        assert_eq!(
            prepared
                .preflight
                .requests
                .as_ref()
                .unwrap()
                .obstacle_case_id,
            OBSTACLE_CASE_ID
        );
        let bound_paths: BTreeSet<_> = prepared
            .preflight
            .source_binding
            .as_ref()
            .unwrap()
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect();
        for path in [
            crate::canonical_initial_direct::CANONICAL_INITIAL_DIRECT_CANARY_PROTOCOL,
            crate::canonical_initial_direct::CANONICAL_INITIAL_DIRECT_CANARY_SELECTION_MANIFEST,
            "fixtures/research/nominal_direct_operational_fresh_inputs_v1.json",
            crate::canonical_initial_direct::CANONICAL_INITIAL_DIRECT_CANARY_ADDITIONAL_MANIFEST,
            crate::nominal_airborne_direct::NOMINAL_AIRBORNE_DIRECT_CANARY_PROTOCOL,
        ] {
            assert!(
                bound_paths.contains(path),
                "preservation source is not bound: {path}"
            );
        }
    }

    #[test]
    fn output_root_reservation_rejects_reuse() {
        let root = temp_root("create-only");
        reserve_output_root(&root).unwrap();
        assert!(reserve_output_root(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_comparison_excludes_declared_clock_only_and_detects_state_tamper() {
        let mut expected = serde_json::json!({
            "elapsed_wall_time_us": 12,
            "final_state": {"physics_step": 44, "fuel_kg": 17.0},
            "actions": [{"physics_step": 42, "command": {"throttle_frac": 0.0}}]
        });
        let mut actual = expected.clone();
        actual["elapsed_wall_time_us"] = serde_json::json!(999);
        let exclusions = vec!["/elapsed_wall_time_us".into()];
        let mut expected_applied = Vec::new();
        let mut actual_applied = Vec::new();
        remove_explicit_paths(
            &mut expected,
            &exclusions,
            &mut Vec::new(),
            &mut expected_applied,
        );
        remove_explicit_paths(
            &mut actual,
            &exclusions,
            &mut Vec::new(),
            &mut actual_applied,
        );
        assert_eq!(expected, actual);

        actual["final_state"]["fuel_kg"] = serde_json::json!(18.0);
        assert_ne!(expected, actual);
        assert_eq!(expected_applied, actual_applied);
    }

    #[test]
    fn repeat_tree_rechecks_files_and_preserves_source_and_physical_identities() {
        let left = temp_root("repeat-left");
        let right = temp_root("repeat-right");
        let left_stage = left.join("preservation/source_rest");
        let right_stage = right.join("preservation/source_rest");
        fs::create_dir_all(left_stage.join("cases/01_case")).unwrap();
        fs::create_dir_all(right_stage.join("cases/01_case")).unwrap();

        for (stage, elapsed, controller_us, report_us) in
            [(&left_stage, 11, 2, 3), (&right_stage, 99, 18, 24)]
        {
            let summary = serde_json::json!({
                "identity": "case-identity-stable",
                "source_binding": {"identity_sha256": "source-closure-stable"},
                "physical": {"final_fuel_kg": 17.0, "physics_step": 44},
                "compute": {"generation_wall_time_us": elapsed},
                "output_path": stage.to_string_lossy(),
            });
            fs::write(
                stage.join("cases/01_case/summary.json"),
                serde_json::to_vec(&summary).unwrap(),
            )
            .unwrap();
            let controller_updates = serde_json::json!([{
                "compute_time_us": controller_us,
                "frame": {"physics_step": 42, "command": {"throttle_frac": 0.0}},
            }]);
            fs::write(
                stage.join("cases/01_case/controller_updates.json"),
                serde_json::to_vec(&controller_updates).unwrap(),
            )
            .unwrap();
            let report_data = serde_json::json!({
                "outputPath": stage.to_string_lossy(),
                "runPerformance": {
                    "wallTimeMs": report_us,
                    "threadCpuTimeMs": report_us,
                    "cpuTimePerTickUs": report_us,
                    "simRateX": report_us,
                    "physicsStepsPerS": report_us,
                },
                "samples": [{"physicsStep": 42, "computeTimeMs": report_us, "fuelKg": 17.0}],
                "botStats": {
                    "totalComputeMs": report_us,
                    "meanComputeMs": report_us,
                    "p95ComputeMs": report_us,
                    "maxComputeMs": report_us,
                    "meanControlDtMs": 16.0,
                    "controlDutyCyclePct": report_us,
                },
            });
            let report = format!(
                "<html><script>const reportData = {};\n    const paperBg = \"#fff\";</script></html>",
                serde_json::to_string(&report_data).unwrap()
            );
            fs::write(stage.join("cases/01_case/report.html"), report).unwrap();
        }

        let comparisons = compare_preservation_repeat_trees(&left, &right).unwrap();
        assert_eq!(comparisons.len(), 3);
        assert!(comparisons.iter().all(|comparison| comparison.passed));

        let controller_updates = serde_json::json!([{
            "compute_time_us": 18,
            "frame": {"physics_step": 42, "command": {"throttle_frac": 0.5}},
        }]);
        fs::write(
            right_stage.join("cases/01_case/controller_updates.json"),
            serde_json::to_vec(&controller_updates).unwrap(),
        )
        .unwrap();
        let comparisons = compare_preservation_repeat_trees(&left, &right).unwrap();
        assert!(comparisons.iter().any(|comparison| {
            comparison
                .comparison_id
                .ends_with("controller_updates.json")
                && !comparison.passed
        }));

        let mut changed_summary: serde_json::Value =
            read_json(&right_stage.join("cases/01_case/summary.json")).unwrap();
        changed_summary["source_binding"]["identity_sha256"] =
            serde_json::json!("different-source-closure");
        fs::write(
            right_stage.join("cases/01_case/summary.json"),
            serde_json::to_vec(&changed_summary).unwrap(),
        )
        .unwrap();
        let comparisons = compare_preservation_repeat_trees(&left, &right).unwrap();
        assert!(comparisons.iter().any(|comparison| {
            comparison.comparison_id.ends_with("summary.json") && !comparison.passed
        }));

        fs::remove_dir_all(left).unwrap();
        fs::remove_dir_all(right).unwrap();
    }

    #[test]
    fn preservation_ledger_rehashes_actual_case_and_source_rest_payloads() {
        let root = temp_root("preservation-ledger");
        let reference_root = root.join("reference");
        let mut canonical = empty_preservation_stage(
            "canonical_controls",
            "reference/canonical",
            "preservation/canonical",
        );
        let mut airborne = empty_preservation_stage(
            "airborne_continuations",
            "reference/airborne",
            "preservation/airborne",
        );
        let mut source_rest = empty_preservation_stage(
            "source_rest_controls",
            "reference/source_rest",
            "preservation/source_rest",
        );
        for case_id in CANONICAL_REFERENCE_CASES {
            let relative = format!("cases/{case_id}.final.json");
            canonical.comparisons.push(write_matching_json_payload(
                &reference_root.join("canonical_controls"),
                &root.join(&canonical.observed_root),
                &relative,
                &canonical_historical_exclusions(),
                format!("canonical_controls:{case_id}"),
            ));
        }
        for case_id in AIRBORNE_REFERENCE_CASES {
            let relative = format!("cases/{case_id}.json");
            airborne.comparisons.push(write_matching_json_payload(
                &reference_root.join("airborne_continuations"),
                &root.join(&airborne.observed_root),
                &relative,
                &airborne_historical_exclusions(),
                format!("airborne_continuations:{case_id}"),
            ));
        }
        source_rest.comparisons.push(write_matching_json_payload(
            &reference_root.join("source_rest"),
            &root.join(&source_rest.observed_root),
            "case.json",
            &[],
            "source_rest:case".into(),
        ));
        for stage in [&mut canonical, &mut airborne, &mut source_rest] {
            stage.status = "passed".into();
            stage.passed = true;
        }
        let ledger = LocalClearingPreservationV1 {
            canonical_controls: canonical,
            airborne_continuations: airborne,
            source_rest_controls: source_rest,
            passed: true,
        };
        write_create_only(&root.join("preservation.json"), &ledger).unwrap();
        assert!(verify_preservation_ledger_payloads(&root).unwrap());

        fs::write(
            root.join("preservation/canonical/cases/operational_flat_span_685.final.json"),
            br#"{"physical":{"fuel_kg":18.0,"physics_step":44}}"#,
        )
        .unwrap();
        assert!(!verify_preservation_ledger_payloads(&root).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    fn write_matching_json_payload(
        reference_root: &Path,
        observed_root: &Path,
        relative: &str,
        exclusions: &[String],
        comparison_id: String,
    ) -> LocalClearingPayloadComparisonV1 {
        let reference = reference_root.join(relative);
        let observed = observed_root.join(relative);
        fs::create_dir_all(reference.parent().unwrap()).unwrap();
        fs::create_dir_all(observed.parent().unwrap()).unwrap();
        let payload = br#"{"identity":"stable","physical":{"fuel_kg":17.0,"physics_step":44}}"#;
        fs::write(&reference, payload).unwrap();
        fs::write(&observed, payload).unwrap();
        compare_json_payload(
            reference_root,
            observed_root,
            relative,
            exclusions,
            comparison_id,
        )
    }
}
