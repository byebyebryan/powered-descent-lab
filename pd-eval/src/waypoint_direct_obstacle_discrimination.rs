//! Opt-in, frozen-input discrimination between a fixed flat command program
//! replayed over terrain and a fresh run of the unchanged direct generator.
//!
//! This module is evaluator-only. It does not change planning, controllers,
//! generator policy, or product defaults. A blocked fixed-command terrain twin
//! is paired evidence, not proof that the generator failed only because of
//! terrain; finite unknown results never imply that waypoints are required.

use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use pd_core::{ScenarioSpec, TerrainDefinition, Vec2};
use serde::{Deserialize, Serialize};

use crate::{
    FixedCommandTerrainTwinEvidence, WaypointDirectNominalDirectGenerationArtifact,
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    evaluate_waypoint_direct_fixed_command_terrain_twin,
    run_waypoint_direct_nominal_direct_generation, waypoint_direct_known_flat_generation_request,
};

pub const WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_ID: &str =
    "waypoint-direct-obstacle-discrimination";
pub const WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_SCHEMA_ID: &str =
    "waypoint_direct_obstacle_discrimination_v1";
pub const WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_SCHEMA_VERSION: u32 = 1;
pub const WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST: &str =
    "fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json";
pub const WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST_SHA256: &str =
    "647df7bc94d02a9b48b773c45159e0ffa15bfca232a13b4e21164e2251a8f5b4";
pub const WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY: &str =
    "outputs/research/ballistic_direct_first_decision_20260925/run_a/summary.json";
pub const WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY_SHA256: &str =
    "65ad8538d901998ce6473af6d4248de521ef81901382d20d03bca3993247da5d";
pub const WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_BASELINE_SUMMARY_SHA256: &str =
    "3e479a9e46c1753c0950076f2434c607db7588003830762c45298fc3e3953e35";

const BASELINE_CASE_ID: &str = "continuous_flat_r00";
const DIRECT_STATUS: &str = "nominal_replay_validated_direct";
const UNKNOWN_STATUS: &str = "unknown_no_complete_accepted_witness_within_finite_family";
const SOURCE_DURATION_OFFSETS_TICKS: [i64; 5] = [-240, -180, -120, -60, 0];
const SOURCE_CRATES: [&str; 6] = [
    "pd-core",
    "pd-plan",
    "pd-control",
    "pd-eval",
    "pd-report",
    "pd-cli",
];
const HASHED_INPUT_FILES: [&str; 5] = [
    "Cargo.toml",
    "Cargo.lock",
    "fixtures/scenarios/flat_terminal_descent.json",
    "pd-plan/fixtures/conservative_ballistic_direct_bridge_probes_v2.json",
    "fixtures/research/waypoint_direct_generation_fresh_inputs_v1.json",
];

const SCOPE_NON_CLAIMS: [&str; 5] = [
    "This opt-in evaluator does not change planner, core, controller, generator policy, or defaults.",
    "A fixed-command terrain twin is diagnostic evidence and is never a regenerated accepted witness.",
    "A blocked fixed-command twin plus a separate regenerated Direct result is an association, not proof that terrain alone caused a different generator result.",
    "Finite Unknown means no complete accepted witness was found in the declared family; it is not a physical-impossibility result.",
    "Unknown does not establish that waypoints are needed.",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointDirectObstacleDiscriminationFreshManifestV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub sealed_before_implementation: bool,
    pub generation_policy: WaypointDirectNominalDirectGenerationPolicyV1,
    pub fresh_case_order: String,
    pub cases: Vec<WaypointDirectObstacleDiscriminationFreshCaseV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointDirectObstacleDiscriminationFreshCaseV1 {
    pub case_id: String,
    pub horizontal_span_m: f64,
    pub profile: String,
    pub center_fraction: f64,
    pub width_fraction: f64,
    pub height_fraction: f64,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub probe_id: String,
    pub scenario: ScenarioSpec,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectObstacleDiscriminationArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub phase: String,
    pub fresh_manifest_sha256: String,
    pub historical_summary_sha256: Option<String>,
    pub production_inputs_identity: String,
    pub cases: Vec<WaypointDirectObstacleDiscriminationCaseEvidenceV1>,
    pub gate_checks: Vec<WaypointDirectObstacleDiscriminationGateCheckV1>,
    pub passed: bool,
    pub verdict: String,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectObstacleDiscriminationCodeFreezeV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub development_identity: String,
    pub development_production_inputs_identity: String,
    pub fresh_manifest_sha256: String,
    pub historical_summary_sha256: String,
    pub production_inputs_identity: String,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectObstacleDiscriminationCaseEvidenceV1 {
    pub case_id: String,
    pub probe_id: String,
    pub historical_role: Option<String>,
    pub profile: String,
    pub horizontal_span_m: f64,
    pub maximum_terrain_height_m: f64,
    pub scenario_identity: String,
    pub generator: WaypointDirectObstacleDiscriminationGeneratorLaneV1,
    pub fixed_command_twin: WaypointDirectObstacleDiscriminationTwinLaneV1,
    pub classification: WaypointDirectObstacleDiscriminationClassificationV1,
    pub classification_basis: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectObstacleDiscriminationGeneratorLaneV1 {
    pub attempted: bool,
    pub directness: WaypointDirectObstacleDiscriminationDirectnessV1,
    pub generation_identity: Option<String>,
    pub generated_summary_sha256: Option<String>,
    pub row_count: usize,
    pub scheduled_count: usize,
    pub accepted_count: usize,
    pub ledger: WaypointDirectObstacleDiscriminationLedgerAuditV1,
    pub accepted_complete_existing_gates: bool,
    pub basis_reasons: Vec<WaypointDirectObstacleDiscriminationReasonRecordV1>,
    pub row_reasons: Vec<WaypointDirectObstacleDiscriminationReasonRecordV1>,
    /// Stable error class only; filesystem paths and transient error text are
    /// intentionally excluded from the semantic artifact identity.
    pub failure_kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectObstacleDiscriminationLedgerAuditV1 {
    pub valid: bool,
    pub row_count: usize,
    pub expected_row_count: usize,
    pub scheduled_count_matches: bool,
    pub accepted_count_matches: bool,
    pub all_rows_are_ordered_and_bound: bool,
    pub family_proof_is_coherent: bool,
    pub artifact_identity_matches: bool,
    pub request_matches: bool,
    pub selection_is_coherent: bool,
    pub failures: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectObstacleDiscriminationReasonRecordV1 {
    pub index: usize,
    pub source_field: String,
    pub category: WaypointDirectObstacleDiscriminationReasonCategoryV1,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointDirectObstacleDiscriminationReasonCategoryV1 {
    TerrainGeometry,
    TerminalContact,
    Source,
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectObstacleDiscriminationTwinLaneV1 {
    pub status: WaypointDirectObstacleDiscriminationTwinStatusV1,
    pub baseline_case_id: Option<String>,
    pub baseline_generation_identity: Option<String>,
    pub evidence: Option<FixedCommandTerrainTwinEvidence>,
    pub prefix_parity_valid: bool,
    pub failure_kind: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointDirectObstacleDiscriminationTwinStatusV1 {
    Evaluated,
    NoFlatWitness,
    TwinError,
    NotEvaluatedSourceDrift,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointDirectObstacleDiscriminationDirectnessV1 {
    Direct,
    Unknown,
    Invalid,
    NotEvaluated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointDirectObstacleDiscriminationClassificationV1 {
    SafeSameArc,
    ChosenArcBlockedAlternativeDirect,
    FiniteUnknown,
    InvalidOrUnavailable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectObstacleDiscriminationGateCheckV1 {
    pub gate_id: String,
    pub passed: bool,
    pub observed: String,
    pub required: String,
}

#[derive(Clone, Debug)]
struct RunnerCase {
    case_id: String,
    probe_id: String,
    historical_role: Option<String>,
    profile: String,
    horizontal_span_m: f64,
    request: WaypointDirectNominalDirectGenerationRequest,
}

#[derive(Clone, Debug)]
struct HistoricalTerrainInput {
    role: &'static str,
    probe_id: String,
    points_m: Vec<Vec2>,
}

struct DevelopmentGateInputs<'a> {
    case_count_ok: bool,
    baseline: &'a WaypointDirectObstacleDiscriminationCaseEvidenceV1,
    positive_160m_obstacle: Option<&'a WaypointDirectObstacleDiscriminationCaseEvidenceV1>,
    all_ledger_valid: bool,
    all_twin_prefix_parity: bool,
    at_least_one_obstacle_twin_blocked: bool,
    source_identity_stable: bool,
    baseline_summary_hash_matches: bool,
}

/// Load and validate the sealed eight-case manifest without constructing a
/// plant or evaluating a candidate.
pub fn load_waypoint_direct_obstacle_discrimination_fresh_manifest(
    repo_root: &Path,
) -> Result<WaypointDirectObstacleDiscriminationFreshManifestV1> {
    let path = repo_root.join(WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST);
    verify_file_sha256(
        &path,
        WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST_SHA256,
    )?;
    let raw = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let manifest: WaypointDirectObstacleDiscriminationFreshManifestV1 =
        serde_json::from_slice(&raw)?;
    validate_fresh_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_fresh_manifest(
    manifest: &WaypointDirectObstacleDiscriminationFreshManifestV1,
) -> Result<()> {
    if manifest.schema_id != "waypoint_direct_obstacle_discrimination_fresh_inputs_v1"
        || manifest.schema_version != 1
        || !manifest.sealed_before_implementation
        || manifest.fresh_case_order
            != "span 700 then 900; each flat_control, low_obstacle, high_obstacle, late_broad"
        || manifest.generation_policy != WaypointDirectNominalDirectGenerationPolicyV1::default()
        || manifest.cases.len() != 8
    {
        bail!("unsupported or unsealed obstacle-discrimination manifest");
    }

    let expected = [
        (
            "fresh_flat_control_span_700",
            700.0,
            "flat_control",
            0.0,
            0.0,
            0.0,
        ),
        (
            "fresh_low_obstacle_span_700",
            700.0,
            "low_obstacle",
            0.4,
            0.2,
            0.2,
        ),
        (
            "fresh_high_obstacle_span_700",
            700.0,
            "high_obstacle",
            0.6,
            0.2,
            0.55,
        ),
        (
            "fresh_late_broad_span_700",
            700.0,
            "late_broad",
            0.7,
            0.25,
            0.35,
        ),
        (
            "fresh_flat_control_span_900",
            900.0,
            "flat_control",
            0.0,
            0.0,
            0.0,
        ),
        (
            "fresh_low_obstacle_span_900",
            900.0,
            "low_obstacle",
            0.4,
            0.2,
            0.2,
        ),
        (
            "fresh_high_obstacle_span_900",
            900.0,
            "high_obstacle",
            0.6,
            0.2,
            0.55,
        ),
        (
            "fresh_late_broad_span_900",
            900.0,
            "late_broad",
            0.7,
            0.25,
            0.35,
        ),
    ];
    for (case, (id, span, profile, center, width, height)) in manifest.cases.iter().zip(expected) {
        if case.case_id != id
            || case.horizontal_span_m != span
            || case.profile != profile
            || case.center_fraction != center
            || case.width_fraction != width
            || case.height_fraction != height
            || case.source_pad_id != "pad_source"
            || case.target_pad_id != "pad_main"
            || case.probe_id != id
        {
            bail!("sealed obstacle-discrimination case order or policy changed");
        }
        case.scenario
            .validate()
            .map_err(|error| anyhow::anyhow!("invalid sealed case {}: {error}", case.case_id))?;
        if case.scenario.mission.transfer_route.is_some()
            || case
                .scenario
                .world
                .landing_pad(&case.source_pad_id)
                .is_none()
            || case
                .scenario
                .world
                .landing_pad(&case.target_pad_id)
                .is_none()
        {
            bail!(
                "sealed case {} is not an unmodified direct landing input",
                case.case_id
            );
        }
        let source = case
            .scenario
            .world
            .landing_pad(&case.source_pad_id)
            .expect("source checked above");
        let target = case
            .scenario
            .world
            .landing_pad(&case.target_pad_id)
            .expect("target checked above");
        if source.center_x_m != -span
            || source.surface_y_m != 0.0
            || target.center_x_m != 0.0
            || target.surface_y_m != 0.0
            || case.scenario.initial_state.position_m.x != -span
            || case.scenario.initial_state.position_m.y != 5.0
            || case.scenario.mission.goal.target_pad_id() != case.target_pad_id
        {
            bail!(
                "sealed case {} changed its source/target setup",
                case.case_id
            );
        }
    }

    for span in [700.0, 900.0] {
        let group = manifest
            .cases
            .iter()
            .filter(|case| case.horizontal_span_m == span)
            .collect::<Vec<_>>();
        let flat = group
            .iter()
            .find(|case| case.profile == "flat_control")
            .context("sealed obstacle span lacks its flat control")?;
        for case in group.iter().filter(|case| case.profile != "flat_control") {
            if !same_scenario_except_terrain_and_descriptive_metadata(
                &flat.scenario,
                &case.scenario,
            ) {
                bail!(
                    "sealed case {} changes more than terrain and descriptive metadata",
                    case.case_id
                );
            }
        }
    }
    Ok(())
}

fn same_scenario_except_terrain_and_descriptive_metadata(
    baseline: &ScenarioSpec,
    candidate: &ScenarioSpec,
) -> bool {
    let mut expected = baseline.clone();
    expected.world.terrain = candidate.world.terrain.clone();
    expected.id.clone_from(&candidate.id);
    expected.name.clone_from(&candidate.name);
    expected.description.clone_from(&candidate.description);
    expected.tags.clone_from(&candidate.tags);
    expected.metadata.clone_from(&candidate.metadata);
    expected == *candidate
}

/// Development pass: the exact known-flat factory baseline plus the three
/// historical terrain geometries, each with fixed-command and regenerated
/// lanes. Historical commands and outcomes are never generator inputs.
pub fn run_waypoint_direct_obstacle_discrimination_development(
    repo_root: &Path,
    output_dir: &Path,
) -> Result<WaypointDirectObstacleDiscriminationArtifactV1> {
    let manifest = load_waypoint_direct_obstacle_discrimination_fresh_manifest(repo_root)?;
    let historical = load_historical_terrain_inputs(repo_root)?;
    let baseline_request = waypoint_direct_known_flat_generation_request(repo_root)?;
    let mut cases = vec![RunnerCase {
        case_id: BASELINE_CASE_ID.to_owned(),
        probe_id: baseline_request.probe_id.clone(),
        historical_role: None,
        profile: "flat_control".to_owned(),
        horizontal_span_m: source_target_span_m(&baseline_request.scenario),
        request: baseline_request.clone(),
    }];
    for input in historical {
        let mut request = baseline_request.clone();
        request.probe_id.clone_from(&input.probe_id);
        request.scenario.world.terrain = TerrainDefinition::Heightfield {
            points_m: input.points_m,
        };
        set_development_descriptive_metadata(&mut request.scenario, &input.probe_id, input.role);
        request
            .scenario
            .validate()
            .map_err(|error| anyhow::anyhow!("historical terrain scenario is invalid: {error}"))?;
        cases.push(RunnerCase {
            case_id: input.probe_id.clone(),
            probe_id: input.probe_id,
            historical_role: Some(input.role.to_owned()),
            profile: input.role.to_owned(),
            horizontal_span_m: source_target_span_m(&request.scenario),
            request,
        });
    }
    let fresh_manifest_sha256 = verify_file_sha256(
        &repo_root.join(WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST),
        WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST_SHA256,
    )?;
    let historical_summary_sha256 = verify_file_sha256(
        &repo_root.join(WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY),
        WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY_SHA256,
    )?;
    let source_identity_before = production_inputs_identity(repo_root)?;
    let output_root = create_runner_output_dir(repo_root, output_dir)?;

    let mut evidence = Vec::with_capacity(cases.len());
    let mut flat_artifact: Option<WaypointDirectNominalDirectGenerationArtifact> = None;
    let mut source_identity_stable = true;
    for (index, case) in cases.iter().enumerate() {
        if production_inputs_identity(repo_root)? != source_identity_before {
            source_identity_stable = false;
            evidence.extend(cases[index..].iter().map(not_evaluated_runner_case));
            break;
        }
        let generated = run_generator_case(repo_root, &output_root, case)?;
        let is_baseline = index == 0;
        let generation_artifact = generated.1;
        let baseline = if is_baseline {
            generation_artifact
                .as_ref()
                .filter(|artifact| generated_artifact_is_accepted_direct(artifact, &case.request))
        } else {
            flat_artifact.as_ref()
        };
        let twin = evaluate_twin_lane(baseline, Some(BASELINE_CASE_ID), &case.request.scenario);
        let case_evidence = build_case_evidence(case, generated.0, twin);
        if is_baseline {
            flat_artifact = generation_artifact;
        }
        evidence.push(case_evidence);
        if production_inputs_identity(repo_root)? != source_identity_before {
            source_identity_stable = false;
            evidence.extend(cases[index + 1..].iter().map(not_evaluated_runner_case));
            break;
        }
    }

    let source_identity_after = production_inputs_identity(repo_root)?;
    let baseline_case = evidence.first().context("development baseline missing")?;
    let positive_160m_case = evidence
        .iter()
        .find(|case| case.historical_role.as_deref() == Some("overflight_control"));
    let checks = development_gate_checks(DevelopmentGateInputs {
        case_count_ok: evidence.len() == 4,
        baseline: baseline_case,
        positive_160m_obstacle: positive_160m_case,
        all_ledger_valid: evidence.iter().all(|case| {
            case.generator.ledger.valid
                && matches!(
                    case.generator.directness,
                    WaypointDirectObstacleDiscriminationDirectnessV1::Direct
                        | WaypointDirectObstacleDiscriminationDirectnessV1::Unknown
                )
                && case.generator.accepted_complete_existing_gates
        }),
        all_twin_prefix_parity: evidence.iter().all(|case| {
            case.fixed_command_twin.status
                == WaypointDirectObstacleDiscriminationTwinStatusV1::Evaluated
                && case.fixed_command_twin.prefix_parity_valid
        }),
        at_least_one_obstacle_twin_blocked: evidence.iter().any(|case| {
            case.historical_role.is_some()
                && case
                    .fixed_command_twin
                    .evidence
                    .as_ref()
                    .is_some_and(|twin| twin.terrain_blocked)
        }),
        source_identity_stable: source_identity_stable
            && source_identity_before == source_identity_after,
        baseline_summary_hash_matches: baseline_case.generator.generated_summary_sha256.as_deref()
            == Some(WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_BASELINE_SUMMARY_SHA256),
    });
    let passed = checks.iter().all(|check| check.passed);
    let mut artifact = WaypointDirectObstacleDiscriminationArtifactV1 {
        schema_id: WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_SCHEMA_VERSION,
        phase: "development".to_owned(),
        fresh_manifest_sha256,
        historical_summary_sha256: Some(historical_summary_sha256),
        production_inputs_identity: source_identity_before,
        cases: evidence,
        gate_checks: checks,
        passed,
        verdict: if passed {
            "development_gates_passed_explicit_primary_acceptance_and_freeze_required"
        } else {
            "development_gate_failed_stop_before_fresh_cases_without_tuning"
        }
        .to_owned(),
        scope_non_claims: SCOPE_NON_CLAIMS
            .iter()
            .map(|item| (*item).to_owned())
            .collect(),
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;
    write_create_only_summary(&output_root, &artifact)?;
    // Keep this binding live so the manifest load above remains part of the
    // sealed preflight and its exact policy is checked before any run.
    debug_assert_eq!(
        manifest.generation_policy,
        WaypointDirectNominalDirectGenerationPolicyV1::default()
    );
    Ok(artifact)
}

/// Record an explicit post-development code freeze. Calling this function is
/// the primary's acceptance step; it refuses failed development evidence or
/// any source/input drift since that evidence was produced.
pub fn freeze_waypoint_direct_obstacle_discrimination(
    repo_root: &Path,
    development_summary: &Path,
    output_dir: &Path,
) -> Result<WaypointDirectObstacleDiscriminationCodeFreezeV1> {
    let development = load_development_artifact(development_summary)?;
    require_passed_development(&development)?;
    let manifest_hash = verify_file_sha256(
        &repo_root.join(WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST),
        WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST_SHA256,
    )?;
    let historical_hash = verify_file_sha256(
        &repo_root.join(WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY),
        WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY_SHA256,
    )?;
    let source_identity = production_inputs_identity(repo_root)?;
    if source_identity != development.production_inputs_identity {
        bail!("source inputs changed after development; refusing code freeze");
    }
    let output_root = create_runner_output_dir(repo_root, output_dir)?;
    let mut freeze = WaypointDirectObstacleDiscriminationCodeFreezeV1 {
        schema_id: "waypoint_direct_obstacle_discrimination_code_freeze_v1".to_owned(),
        schema_version: 1,
        development_identity: development.identity,
        development_production_inputs_identity: development.production_inputs_identity,
        fresh_manifest_sha256: manifest_hash,
        historical_summary_sha256: historical_hash,
        production_inputs_identity: source_identity,
        identity: String::new(),
    };
    freeze.identity = code_freeze_identity(&freeze)?;
    write_create_only_summary(&output_root, &freeze)?;
    Ok(freeze)
}

/// Run all eight sealed fresh cases in manifest order. Acceptance failures are
/// recorded after evaluating the full family. A source-integrity failure
/// stops further physical runs and is published as a failed gate.
pub fn run_waypoint_direct_obstacle_discrimination_fresh(
    repo_root: &Path,
    development_summary: &Path,
    freeze_summary: &Path,
    output_dir: &Path,
) -> Result<WaypointDirectObstacleDiscriminationArtifactV1> {
    let development = load_development_artifact(development_summary)?;
    require_passed_development(&development)?;
    let freeze: WaypointDirectObstacleDiscriminationCodeFreezeV1 = serde_json::from_slice(
        &fs::read(freeze_summary)
            .with_context(|| format!("reading code freeze {}", freeze_summary.display()))?,
    )?;
    validate_code_freeze_binding(&development, &freeze)?;
    let manifest = load_waypoint_direct_obstacle_discrimination_fresh_manifest(repo_root)?;
    let source_identity_before = production_inputs_identity(repo_root)?;
    if source_identity_before != freeze.production_inputs_identity {
        bail!("source inputs changed after code freeze; refusing fresh evaluations");
    }
    let historical_hash = verify_file_sha256(
        &repo_root.join(WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY),
        WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY_SHA256,
    )?;
    if historical_hash != freeze.historical_summary_sha256 {
        bail!("historical topology input changed after code freeze");
    }
    let output_root = create_runner_output_dir(repo_root, output_dir)?;

    let mut cases = Vec::with_capacity(manifest.cases.len());
    let mut flat_witnesses: BTreeMap<u64, (String, WaypointDirectNominalDirectGenerationArtifact)> =
        BTreeMap::new();
    let mut integrity_failure = false;
    for (index, sealed_case) in manifest.cases.iter().enumerate() {
        let source_before_case = production_inputs_identity(repo_root)?;
        if source_before_case != freeze.production_inputs_identity {
            integrity_failure = true;
            cases.extend(manifest.cases[index..].iter().map(|remaining| {
                not_evaluated_case(remaining, "source_inputs_changed_before_case")
            }));
            break;
        }
        let case = RunnerCase {
            case_id: sealed_case.case_id.clone(),
            probe_id: sealed_case.probe_id.clone(),
            historical_role: None,
            profile: sealed_case.profile.clone(),
            horizontal_span_m: sealed_case.horizontal_span_m,
            request: WaypointDirectNominalDirectGenerationRequest {
                scenario: sealed_case.scenario.clone(),
                source_pad_id: sealed_case.source_pad_id.clone(),
                target_pad_id: sealed_case.target_pad_id.clone(),
                probe_id: sealed_case.probe_id.clone(),
                policy: manifest.generation_policy.clone(),
            },
        };
        let generated = run_generator_case(repo_root, &output_root, &case)?;
        let generation_artifact = generated.1;
        let span_key = span_key(sealed_case.horizontal_span_m);
        let baseline = if sealed_case.profile == "flat_control" {
            generation_artifact
                .as_ref()
                .filter(|artifact| generated_artifact_is_accepted_direct(artifact, &case.request))
        } else {
            flat_witnesses.get(&span_key).map(|(_, artifact)| artifact)
        };
        let baseline_case_id = if sealed_case.profile == "flat_control" {
            Some(sealed_case.case_id.as_str())
        } else {
            flat_witnesses
                .get(&span_key)
                .map(|(case_id, _)| case_id.as_str())
        };
        let twin = evaluate_twin_lane(baseline, baseline_case_id, &case.request.scenario);
        let case_evidence = build_case_evidence(&case, generated.0, twin);
        if sealed_case.profile == "flat_control"
            && generation_artifact.as_ref().is_some_and(|artifact| {
                generated_artifact_is_accepted_direct(artifact, &case.request)
            })
        {
            flat_witnesses.insert(
                span_key,
                (
                    sealed_case.case_id.clone(),
                    generation_artifact.expect("checked above"),
                ),
            );
        }
        cases.push(case_evidence);
        let source_after_case = production_inputs_identity(repo_root)?;
        if source_after_case != freeze.production_inputs_identity {
            integrity_failure = true;
            if index + 1 < manifest.cases.len() {
                cases.extend(manifest.cases[index + 1..].iter().map(|remaining| {
                    not_evaluated_case(remaining, "source_inputs_changed_after_case")
                }));
            }
            break;
        }
    }

    let source_identity_after = production_inputs_identity(repo_root)?;
    if source_identity_after != freeze.production_inputs_identity {
        integrity_failure = true;
    }
    let checks = fresh_gate_checks(&cases, manifest.cases.len(), !integrity_failure);
    let passed = checks.iter().all(|check| check.passed);
    let mut artifact = WaypointDirectObstacleDiscriminationArtifactV1 {
        schema_id: WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_SCHEMA_VERSION,
        phase: "fresh".to_owned(),
        fresh_manifest_sha256: WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST_SHA256
            .to_owned(),
        historical_summary_sha256: Some(historical_hash),
        production_inputs_identity: source_identity_before,
        cases,
        gate_checks: checks,
        passed,
        verdict: if passed {
            "sealed_fresh_family_passed_opt_in_evaluator_evidence_only"
        } else {
            "sealed_fresh_family_failed_publish_all_evaluated_cases_without_tuning"
        }
        .to_owned(),
        scope_non_claims: SCOPE_NON_CLAIMS
            .iter()
            .map(|item| (*item).to_owned())
            .collect(),
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;
    write_create_only_summary(&output_root, &artifact)?;
    Ok(artifact)
}

fn run_generator_case(
    repo_root: &Path,
    output_root: &Path,
    case: &RunnerCase,
) -> Result<(
    WaypointDirectObstacleDiscriminationGeneratorLaneV1,
    Option<WaypointDirectNominalDirectGenerationArtifact>,
)> {
    let run_dir = output_root
        .join("cases")
        .join(&case.case_id)
        .join("generation");
    let run =
        match run_waypoint_direct_nominal_direct_generation(repo_root, &case.request, &run_dir) {
            Ok(run) => run,
            Err(_) => {
                return Ok((failed_generator_lane("direct_generation_run_error"), None));
            }
        };
    let summary_hash = verify_file_sha256_output(&run.paths.summary_path).ok();
    let lane = audit_generator_artifact(&run.artifact, &case.request, summary_hash);
    Ok((lane, Some(run.artifact)))
}

fn failed_generator_lane(
    failure_kind: &str,
) -> WaypointDirectObstacleDiscriminationGeneratorLaneV1 {
    WaypointDirectObstacleDiscriminationGeneratorLaneV1 {
        attempted: true,
        directness: WaypointDirectObstacleDiscriminationDirectnessV1::Invalid,
        generation_identity: None,
        generated_summary_sha256: None,
        row_count: 0,
        scheduled_count: 0,
        accepted_count: 0,
        ledger: WaypointDirectObstacleDiscriminationLedgerAuditV1 {
            valid: false,
            row_count: 0,
            expected_row_count: 20,
            scheduled_count_matches: false,
            accepted_count_matches: false,
            all_rows_are_ordered_and_bound: false,
            family_proof_is_coherent: false,
            artifact_identity_matches: false,
            request_matches: false,
            selection_is_coherent: false,
            failures: vec![failure_kind.to_owned()],
        },
        accepted_complete_existing_gates: false,
        basis_reasons: Vec::new(),
        row_reasons: Vec::new(),
        failure_kind: Some(failure_kind.to_owned()),
    }
}

fn audit_generator_artifact(
    artifact: &WaypointDirectNominalDirectGenerationArtifact,
    expected_request: &WaypointDirectNominalDirectGenerationRequest,
    summary_sha256: Option<String>,
) -> WaypointDirectObstacleDiscriminationGeneratorLaneV1 {
    let mut failures = Vec::new();
    let identity_matches = artifact.schema_id == "waypoint_direct_nominal_direct_generation_v1"
        && artifact.schema_version == 1
        && artifact.characterization_id == "waypoint-direct-nominal-direct-generation"
        && generated_artifact_identity(artifact)
            .is_ok_and(|identity| identity == artifact.identity);
    let request_matches = artifact.request == *expected_request
        && artifact.policy == expected_request.policy
        && expected_request.policy == WaypointDirectNominalDirectGenerationPolicyV1::default()
        && artifact.probe_id == expected_request.probe_id
        && artifact.source_pad_id == expected_request.source_pad_id
        && artifact.target_pad_id == expected_request.target_pad_id
        && artifact.scenario_identity
            == semantic_digest(&expected_request.scenario).unwrap_or_default()
        && artifact.policy_identity
            == semantic_digest(&expected_request.policy).unwrap_or_default();
    let expected_row_count = expected_request.policy.maximum_variants;
    let expected_basis_count = expected_request.policy.maximum_basis_candidates;
    let expected_offsets = expected_request
        .policy
        .source_duration_offsets_ticks
        .as_slice();
    let ordered_rows = expected_row_count == 20
        && expected_basis_count == 4
        && expected_offsets == SOURCE_DURATION_OFFSETS_TICKS
        && artifact.bases.len() == expected_basis_count
        && artifact
            .bases
            .iter()
            .enumerate()
            .all(|(index, basis)| basis.basis_index == index)
        && artifact.rows.len() == expected_row_count
        && artifact.rows.iter().enumerate().all(|(index, row)| {
            let basis_index = index / SOURCE_DURATION_OFFSETS_TICKS.len();
            let offset_index = index % SOURCE_DURATION_OFFSETS_TICKS.len();
            row.row_index == index
                && row.basis_index == basis_index
                && row.duration_offset_ticks == SOURCE_DURATION_OFFSETS_TICKS[offset_index]
                && artifact.bases.get(basis_index).is_some_and(|basis| {
                    row.basis_candidate_identity == basis.candidate_identity
                        && row.generated_basis_identity == basis.generated_basis_identity
                })
                && row.accepted == row.wrapper.as_ref().is_some_and(|wrapper| wrapper.accepted)
        });
    let scheduled_count = artifact
        .rows
        .iter()
        .filter(|row| row.paired_schedule.is_some())
        .count();
    let accepted_count = artifact.rows.iter().filter(|row| row.accepted).count();
    let completed_acceptance_count = artifact
        .rows
        .iter()
        .filter(|row| row.wrapper.is_some())
        .count();
    let scheduled_count_matches = artifact.family_proof.scheduled_count == scheduled_count;
    let accepted_count_matches = artifact.family_proof.accepted_witness_count == accepted_count;
    let family_proof_is_coherent = artifact.family_proof.basis_count == expected_basis_count
        && artifact.family_proof.expected_basis_count == expected_basis_count
        && artifact.family_proof.row_count == expected_row_count
        && artifact.family_proof.expected_row_count == expected_row_count
        && artifact.family_proof.omitted_row_count == 0
        && artifact.family_proof.all_predeclared_rows_recorded
        && artifact.family_proof.analytical_skip_count
            == artifact
                .rows
                .iter()
                .filter(|row| !row.analytical_survivor)
                .count()
        && artifact.family_proof.completed_acceptance_count == completed_acceptance_count
        && artifact.family_proof.stopping_result == artifact.execution_status;
    let selection_is_coherent = match (&artifact.selection, &artifact.selected_physical_witness) {
        (Some(selection), Some(witness)) => artifact
            .rows
            .get(selection.row_index)
            .and_then(|row| row.wrapper.as_ref().map(|wrapper| (row, wrapper)))
            .is_some_and(|(row, wrapper)| {
                row.accepted
                    && wrapper.accepted
                    && wrapper.wrapper_identity == selection.wrapper_identity
                    && wrapper.planned_total_mission_time_s
                        == selection.planned_total_mission_time_s
                    && witness.row_index == selection.row_index
                    && witness.wrapper_identity == selection.wrapper_identity
            }),
        (None, None) => accepted_count == 0,
        _ => false,
    };
    let accepted_complete_existing_gates = artifact.rows.iter().all(|row| {
        row.wrapper.as_ref().is_none_or(|wrapper| {
            !wrapper.accepted || complete_existing_gates_pass(&wrapper.acceptance)
        })
    });
    if !identity_matches {
        failures.push("artifact_identity_mismatch".to_owned());
    }
    if !request_matches {
        failures.push("request_or_policy_mismatch".to_owned());
    }
    if !ordered_rows {
        failures.push("row_order_or_binding_incomplete".to_owned());
    }
    if !scheduled_count_matches || !accepted_count_matches || !family_proof_is_coherent {
        failures.push("family_proof_count_or_completion_mismatch".to_owned());
    }
    if !selection_is_coherent {
        failures.push("selection_incoherent".to_owned());
    }
    if !accepted_complete_existing_gates {
        failures.push("accepted_wrapper_missing_complete_existing_gates".to_owned());
    }
    let ledger_valid = identity_matches
        && request_matches
        && ordered_rows
        && scheduled_count_matches
        && accepted_count_matches
        && family_proof_is_coherent
        && selection_is_coherent;
    let directness = if !ledger_valid || !accepted_complete_existing_gates {
        WaypointDirectObstacleDiscriminationDirectnessV1::Invalid
    } else if accepted_count > 0
        && artifact.selection.is_some()
        && artifact.execution_status == DIRECT_STATUS
    {
        WaypointDirectObstacleDiscriminationDirectnessV1::Direct
    } else if accepted_count == 0
        && artifact.selection.is_none()
        && artifact.execution_status == UNKNOWN_STATUS
    {
        WaypointDirectObstacleDiscriminationDirectnessV1::Unknown
    } else {
        WaypointDirectObstacleDiscriminationDirectnessV1::Invalid
    };
    let (basis_reasons, row_reasons) = summarize_generator_reasons(artifact);
    WaypointDirectObstacleDiscriminationGeneratorLaneV1 {
        attempted: true,
        directness,
        generation_identity: Some(artifact.identity.clone()),
        generated_summary_sha256: summary_sha256,
        row_count: artifact.rows.len(),
        scheduled_count,
        accepted_count,
        ledger: WaypointDirectObstacleDiscriminationLedgerAuditV1 {
            valid: ledger_valid,
            row_count: artifact.rows.len(),
            expected_row_count,
            scheduled_count_matches,
            accepted_count_matches,
            all_rows_are_ordered_and_bound: ordered_rows,
            family_proof_is_coherent,
            artifact_identity_matches: identity_matches,
            request_matches,
            selection_is_coherent,
            failures,
        },
        accepted_complete_existing_gates,
        basis_reasons,
        row_reasons,
        failure_kind: (directness == WaypointDirectObstacleDiscriminationDirectnessV1::Invalid)
            .then(|| "generated_artifact_failed_runner_audit".to_owned()),
    }
}

fn complete_existing_gates_pass(gates: &crate::CompleteFlatAcceptanceGatesEvidence) -> bool {
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

fn generated_artifact_is_accepted_direct(
    artifact: &WaypointDirectNominalDirectGenerationArtifact,
    request: &WaypointDirectNominalDirectGenerationRequest,
) -> bool {
    audit_generator_artifact(artifact, request, None).directness
        == WaypointDirectObstacleDiscriminationDirectnessV1::Direct
}

fn summarize_generator_reasons(
    artifact: &WaypointDirectNominalDirectGenerationArtifact,
) -> (
    Vec<WaypointDirectObstacleDiscriminationReasonRecordV1>,
    Vec<WaypointDirectObstacleDiscriminationReasonRecordV1>,
) {
    let mut basis_reasons = Vec::new();
    for basis in &artifact.bases {
        for reason in &basis.original_v2_reasons {
            basis_reasons.push(reason_record(
                basis.basis_index,
                "original_v2_reasons",
                reason.clone(),
            ));
        }
    }
    let mut row_reasons = Vec::new();
    for row in &artifact.rows {
        if let Some(reason) = row.skip_reason.as_ref() {
            row_reasons.push(reason_record(row.row_index, "skip_reason", reason.clone()));
        }
        if let Some(source) = row.source_duration.as_ref() {
            if let Some(error) = source.unlaunched_source_bridge.build_error.as_ref() {
                row_reasons.push(reason_record(
                    row.row_index,
                    "unlaunched_source_bridge.build_error",
                    error.clone(),
                ));
            }
            for (field, reason) in [
                ("analysis_error", source.analysis_error.as_ref()),
                ("source_handoff_error", source.source_handoff_error.as_ref()),
                (
                    "source_handoff_skip_reason",
                    source.source_handoff_skip_reason.as_ref(),
                ),
                (
                    "full_flight_skip_reason",
                    source.full_flight_skip_reason.as_ref(),
                ),
            ] {
                if let Some(reason) = reason {
                    row_reasons.push(reason_record(row.row_index, field, reason.clone()));
                }
            }
        }
        if let Some(wrapper) = row.wrapper.as_ref() {
            if let Some(gate) = wrapper.first_failing_gate.as_ref() {
                row_reasons.push(reason_record(
                    row.row_index,
                    "first_failing_gate",
                    gate.clone(),
                ));
            }
            for reason in &wrapper.rejection_reasons {
                row_reasons.push(reason_record(
                    row.row_index,
                    "rejection_reasons",
                    reason.clone(),
                ));
            }
        }
    }
    (basis_reasons, row_reasons)
}

fn reason_record(
    index: usize,
    source_field: &str,
    reason: String,
) -> WaypointDirectObstacleDiscriminationReasonRecordV1 {
    let source_field_lower = source_field.to_ascii_lowercase();
    let reason_lower = reason.to_ascii_lowercase();
    let category = if source_field_lower.contains("source")
        || ["source", "launch_to_source", "no_survivingsource"]
            .iter()
            .any(|token| reason_lower.contains(token))
    {
        WaypointDirectObstacleDiscriminationReasonCategoryV1::Source
    } else if [
        "terrain",
        "clearance",
        "corridor",
        "geometry",
        "obstacle",
        "hull",
    ]
    .iter()
    .any(|token| reason_lower.contains(token))
    {
        WaypointDirectObstacleDiscriminationReasonCategoryV1::TerrainGeometry
    } else if ["contact", "touchdown", "terminal", "landing", "rebound"]
        .iter()
        .any(|token| reason_lower.contains(token))
    {
        WaypointDirectObstacleDiscriminationReasonCategoryV1::TerminalContact
    } else {
        WaypointDirectObstacleDiscriminationReasonCategoryV1::Other
    };
    WaypointDirectObstacleDiscriminationReasonRecordV1 {
        index,
        source_field: source_field.to_owned(),
        category,
        reason,
    }
}

fn evaluate_twin_lane(
    baseline: Option<&WaypointDirectNominalDirectGenerationArtifact>,
    baseline_case_id: Option<&str>,
    terrain_scenario: &ScenarioSpec,
) -> WaypointDirectObstacleDiscriminationTwinLaneV1 {
    let Some(baseline) = baseline else {
        return WaypointDirectObstacleDiscriminationTwinLaneV1 {
            status: WaypointDirectObstacleDiscriminationTwinStatusV1::NoFlatWitness,
            baseline_case_id: None,
            baseline_generation_identity: None,
            evidence: None,
            prefix_parity_valid: false,
            failure_kind: Some("no_flat_witness".to_owned()),
        };
    };
    match evaluate_waypoint_direct_fixed_command_terrain_twin(baseline, terrain_scenario) {
        Ok(evidence) => {
            let identity_valid = twin_identity_matches(&evidence)
                && evidence.terrain_scenario_identity
                    == semantic_digest(terrain_scenario).unwrap_or_default()
                && baseline
                    .selected_physical_witness
                    .as_ref()
                    .is_some_and(|witness| {
                        evidence.baseline_wrapper_identity == witness.wrapper_identity
                    });
            let counts_valid = evidence.logged_tick_count > 0
                && evidence.physics_ticks_replayed > 0
                && evidence.physics_ticks_replayed <= evidence.logged_tick_count
                && evidence
                    .first_terrain_contact_step
                    .is_none_or(|step| step <= evidence.physics_ticks_replayed);
            let prefix_parity_valid = identity_valid
                && counts_valid
                && evidence.commands_and_states_identical_until_terrain_contact
                && evidence.ordinary_neutral_states_match
                && evidence.first_command_mismatch.is_none();
            WaypointDirectObstacleDiscriminationTwinLaneV1 {
                status: WaypointDirectObstacleDiscriminationTwinStatusV1::Evaluated,
                baseline_case_id: baseline_case_id.map(ToOwned::to_owned),
                baseline_generation_identity: Some(baseline.identity.clone()),
                evidence: Some(evidence),
                prefix_parity_valid,
                failure_kind: (!prefix_parity_valid)
                    .then(|| "fixed_command_twin_prefix_or_identity_audit_failed".to_owned()),
            }
        }
        Err(_) => WaypointDirectObstacleDiscriminationTwinLaneV1 {
            status: WaypointDirectObstacleDiscriminationTwinStatusV1::TwinError,
            baseline_case_id: baseline_case_id.map(ToOwned::to_owned),
            baseline_generation_identity: Some(baseline.identity.clone()),
            evidence: None,
            prefix_parity_valid: false,
            failure_kind: Some("fixed_command_twin_evaluation_error".to_owned()),
        },
    }
}

fn twin_identity_matches(evidence: &FixedCommandTerrainTwinEvidence) -> bool {
    let mut input = evidence.clone();
    input.identity.clear();
    semantic_digest(&input).is_ok_and(|identity| identity == evidence.identity)
}

fn build_case_evidence(
    case: &RunnerCase,
    generator: WaypointDirectObstacleDiscriminationGeneratorLaneV1,
    fixed_command_twin: WaypointDirectObstacleDiscriminationTwinLaneV1,
) -> WaypointDirectObstacleDiscriminationCaseEvidenceV1 {
    let (classification, basis) = classify_lanes(&generator, &fixed_command_twin);
    WaypointDirectObstacleDiscriminationCaseEvidenceV1 {
        case_id: case.case_id.clone(),
        probe_id: case.probe_id.clone(),
        historical_role: case.historical_role.clone(),
        profile: case.profile.clone(),
        horizontal_span_m: case.horizontal_span_m,
        maximum_terrain_height_m: maximum_terrain_height_m(&case.request.scenario),
        scenario_identity: semantic_digest(&case.request.scenario).unwrap_or_default(),
        generator,
        fixed_command_twin,
        classification,
        classification_basis: basis.to_owned(),
    }
}

fn classify_lanes(
    generator: &WaypointDirectObstacleDiscriminationGeneratorLaneV1,
    twin: &WaypointDirectObstacleDiscriminationTwinLaneV1,
) -> (
    WaypointDirectObstacleDiscriminationClassificationV1,
    &'static str,
) {
    if matches!(
        generator.directness,
        WaypointDirectObstacleDiscriminationDirectnessV1::Invalid
            | WaypointDirectObstacleDiscriminationDirectnessV1::NotEvaluated
    ) || matches!(
        twin.status,
        WaypointDirectObstacleDiscriminationTwinStatusV1::TwinError
            | WaypointDirectObstacleDiscriminationTwinStatusV1::NotEvaluatedSourceDrift
    ) {
        return (
            WaypointDirectObstacleDiscriminationClassificationV1::InvalidOrUnavailable,
            "one_or_more_evidence_lanes_failed_or_were_not_evaluated",
        );
    }
    match twin.evidence.as_ref() {
        Some(evidence) if twin.prefix_parity_valid && !evidence.terrain_blocked => (
            WaypointDirectObstacleDiscriminationClassificationV1::SafeSameArc,
            "fixed_command_prefix_parity_and_terrain_clearance_passed",
        ),
        Some(evidence)
            if twin.prefix_parity_valid
                && evidence.terrain_blocked
                && generator.directness == WaypointDirectObstacleDiscriminationDirectnessV1::Direct =>
        {
            (
                WaypointDirectObstacleDiscriminationClassificationV1::ChosenArcBlockedAlternativeDirect,
                "fixed_command_twin_blocked_and_separate_regenerated_lane_direct",
            )
        }
        None
            if generator.directness
                == WaypointDirectObstacleDiscriminationDirectnessV1::Unknown
                && twin.status
                    == WaypointDirectObstacleDiscriminationTwinStatusV1::NoFlatWitness =>
        {
            (
                WaypointDirectObstacleDiscriminationClassificationV1::FiniteUnknown,
                "regenerated_finite_family_unknown_and_no_accepted_flat_witness_available",
            )
        }
        Some(_) if !twin.prefix_parity_valid => (
            WaypointDirectObstacleDiscriminationClassificationV1::InvalidOrUnavailable,
            "fixed_command_twin_prefix_or_identity_audit_failed",
        ),
        None if twin.status == WaypointDirectObstacleDiscriminationTwinStatusV1::NoFlatWitness
            && generator.directness == WaypointDirectObstacleDiscriminationDirectnessV1::Direct =>
        {
            (
                WaypointDirectObstacleDiscriminationClassificationV1::InvalidOrUnavailable,
                "direct_regenerated_case_has_no_flat_witness_for_counterfactual",
            )
        }
        _ => (
            WaypointDirectObstacleDiscriminationClassificationV1::FiniteUnknown,
            "insufficient_or_unknown_finite_family_evidence",
        ),
    }
}

fn not_evaluated_case(
    case: &WaypointDirectObstacleDiscriminationFreshCaseV1,
    failure_kind: &str,
) -> WaypointDirectObstacleDiscriminationCaseEvidenceV1 {
    let generator = not_evaluated_generator_lane(failure_kind);
    let twin = WaypointDirectObstacleDiscriminationTwinLaneV1 {
        status: WaypointDirectObstacleDiscriminationTwinStatusV1::NotEvaluatedSourceDrift,
        baseline_case_id: None,
        baseline_generation_identity: None,
        evidence: None,
        prefix_parity_valid: false,
        failure_kind: Some(failure_kind.to_owned()),
    };
    WaypointDirectObstacleDiscriminationCaseEvidenceV1 {
        case_id: case.case_id.clone(),
        probe_id: case.probe_id.clone(),
        historical_role: None,
        profile: case.profile.clone(),
        horizontal_span_m: case.horizontal_span_m,
        maximum_terrain_height_m: maximum_terrain_height_m(&case.scenario),
        scenario_identity: semantic_digest(&case.scenario).unwrap_or_default(),
        generator,
        fixed_command_twin: twin,
        classification: WaypointDirectObstacleDiscriminationClassificationV1::InvalidOrUnavailable,
        classification_basis: "case_not_evaluated_due_to_source_integrity_failure".to_owned(),
    }
}

fn not_evaluated_generator_lane(
    failure_kind: &str,
) -> WaypointDirectObstacleDiscriminationGeneratorLaneV1 {
    WaypointDirectObstacleDiscriminationGeneratorLaneV1 {
        attempted: false,
        directness: WaypointDirectObstacleDiscriminationDirectnessV1::NotEvaluated,
        generation_identity: None,
        generated_summary_sha256: None,
        row_count: 0,
        scheduled_count: 0,
        accepted_count: 0,
        ledger: WaypointDirectObstacleDiscriminationLedgerAuditV1 {
            valid: false,
            row_count: 0,
            expected_row_count: 20,
            scheduled_count_matches: false,
            accepted_count_matches: false,
            all_rows_are_ordered_and_bound: false,
            family_proof_is_coherent: false,
            artifact_identity_matches: false,
            request_matches: false,
            selection_is_coherent: false,
            failures: vec![failure_kind.to_owned()],
        },
        accepted_complete_existing_gates: false,
        basis_reasons: Vec::new(),
        row_reasons: Vec::new(),
        failure_kind: Some(failure_kind.to_owned()),
    }
}

fn not_evaluated_runner_case(
    case: &RunnerCase,
) -> WaypointDirectObstacleDiscriminationCaseEvidenceV1 {
    WaypointDirectObstacleDiscriminationCaseEvidenceV1 {
        case_id: case.case_id.clone(),
        probe_id: case.probe_id.clone(),
        historical_role: case.historical_role.clone(),
        profile: case.profile.clone(),
        horizontal_span_m: case.horizontal_span_m,
        maximum_terrain_height_m: maximum_terrain_height_m(&case.request.scenario),
        scenario_identity: semantic_digest(&case.request.scenario).unwrap_or_default(),
        generator: not_evaluated_generator_lane("source_inputs_changed_during_development"),
        fixed_command_twin: WaypointDirectObstacleDiscriminationTwinLaneV1 {
            status: WaypointDirectObstacleDiscriminationTwinStatusV1::NotEvaluatedSourceDrift,
            baseline_case_id: None,
            baseline_generation_identity: None,
            evidence: None,
            prefix_parity_valid: false,
            failure_kind: Some("source_inputs_changed_during_development".to_owned()),
        },
        classification: WaypointDirectObstacleDiscriminationClassificationV1::InvalidOrUnavailable,
        classification_basis: "case_not_evaluated_due_to_source_integrity_failure".to_owned(),
    }
}

fn development_gate_checks(
    inputs: DevelopmentGateInputs<'_>,
) -> Vec<WaypointDirectObstacleDiscriminationGateCheckV1> {
    vec![
        gate_check(
            "four_development_cases_recorded",
            inputs.case_count_ok,
            inputs.case_count_ok.to_string(),
            "exactly four baseline and historical-terrain cases",
        ),
        gate_check(
            "baseline_summary_sha256",
            inputs.baseline_summary_hash_matches,
            inputs
                .baseline
                .generator
                .generated_summary_sha256
                .clone()
                .unwrap_or_else(|| "missing".to_owned()),
            WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_BASELINE_SUMMARY_SHA256,
        ),
        gate_check(
            "baseline_flat_is_direct",
            inputs.baseline.generator.directness
                == WaypointDirectObstacleDiscriminationDirectnessV1::Direct,
            format!("{:?}", inputs.baseline.generator.directness),
            "Direct",
        ),
        gate_check(
            "positive_160m_obstacle_is_direct",
            inputs.positive_160m_obstacle.is_some_and(|case| {
                case.maximum_terrain_height_m == 160.0
                    && case.generator.directness
                        == WaypointDirectObstacleDiscriminationDirectnessV1::Direct
            }),
            inputs
                .positive_160m_obstacle
                .map(|case| {
                    format!(
                        "{}:max_height={}m:{:?}",
                        case.case_id, case.maximum_terrain_height_m, case.generator.directness
                    )
                })
                .unwrap_or_else(|| "missing_overflight_case".to_owned()),
            "the frozen 160 m overflight obstacle case is Direct",
        ),
        gate_check(
            "all_twenty_row_ledgers_valid",
            inputs.all_ledger_valid,
            inputs.all_ledger_valid.to_string(),
            "every development generator ledger has 20 ordered, bound rows and coherent counts",
        ),
        gate_check(
            "all_fixed_command_prefixes_match",
            inputs.all_twin_prefix_parity,
            inputs.all_twin_prefix_parity.to_string(),
            "all four fixed-command twins preserve command/state prefix and ordinary-neutral parity",
        ),
        gate_check(
            "at_least_one_obstacle_twin_blocked",
            inputs.at_least_one_obstacle_twin_blocked,
            inputs.at_least_one_obstacle_twin_blocked.to_string(),
            "at least one of the three historical obstacle twins is terrain-blocked",
        ),
        gate_check(
            "source_identity_stable",
            inputs.source_identity_stable,
            inputs.source_identity_stable.to_string(),
            "source inputs unchanged during development runs",
        ),
    ]
}

fn fresh_gate_checks(
    cases: &[WaypointDirectObstacleDiscriminationCaseEvidenceV1],
    expected_case_count: usize,
    source_identity_stable: bool,
) -> Vec<WaypointDirectObstacleDiscriminationGateCheckV1> {
    let all_cases_attempted =
        cases.len() == expected_case_count && cases.iter().all(|case| case.generator.attempted);
    let all_ledgers_valid =
        cases.len() == expected_case_count && cases.iter().all(|case| case.generator.ledger.valid);
    let required_directness = cases.iter().all(|case| match case.profile.as_str() {
        "flat_control" | "low_obstacle" => {
            case.generator.directness == WaypointDirectObstacleDiscriminationDirectnessV1::Direct
        }
        "high_obstacle" | "late_broad" => matches!(
            case.generator.directness,
            WaypointDirectObstacleDiscriminationDirectnessV1::Direct
                | WaypointDirectObstacleDiscriminationDirectnessV1::Unknown
        ),
        _ => false,
    });
    let accepted_gates_complete = cases
        .iter()
        .all(|case| case.generator.ledger.valid && case.generator.accepted_complete_existing_gates);
    let all_twins_have_prefix_parity = cases.len() == expected_case_count
        && cases.iter().all(|case| {
            case.fixed_command_twin.status
                == WaypointDirectObstacleDiscriminationTwinStatusV1::Evaluated
                && case.fixed_command_twin.prefix_parity_valid
        });
    let any_fixed_program_blocked = cases.iter().any(|case| {
        case.fixed_command_twin
            .evidence
            .as_ref()
            .is_some_and(|evidence| evidence.terrain_blocked)
    });
    vec![
        gate_check(
            "all_eight_cases_evaluated",
            all_cases_attempted,
            format!(
                "{} attempted of {expected_case_count}",
                cases.iter().filter(|case| case.generator.attempted).count()
            ),
            "all eight sealed cases attempted in manifest order, even if acceptance fails",
        ),
        gate_check(
            "all_twenty_row_ledgers_valid",
            all_ledgers_valid,
            all_ledgers_valid.to_string(),
            "all eight generators recorded complete, ordered 20-row ledgers",
        ),
        gate_check(
            "profile_directness_policy",
            required_directness,
            required_directness.to_string(),
            "flat and low obstacle Direct at both spans; high and late broad Direct or Unknown",
        ),
        gate_check(
            "all_accepted_wrappers_pass_existing_gates",
            accepted_gates_complete,
            accepted_gates_complete.to_string(),
            "every accepted wrapper passes all existing complete-acceptance gates",
        ),
        gate_check(
            "all_fixed_command_twins_match_prefix",
            all_twins_have_prefix_parity,
            all_twins_have_prefix_parity.to_string(),
            "each case has its same-span flat witness and valid fixed-command prefix parity",
        ),
        gate_check(
            "at_least_one_fixed_program_blocked",
            any_fixed_program_blocked,
            any_fixed_program_blocked.to_string(),
            "at least one fixed-command terrain twin is terrain-blocked",
        ),
        gate_check(
            "source_identity_matches_freeze",
            source_identity_stable,
            source_identity_stable.to_string(),
            "source inputs match the accepted code freeze before, between, and after cases",
        ),
    ]
}

fn gate_check(
    gate_id: &str,
    passed: bool,
    observed: impl Into<String>,
    required: impl Into<String>,
) -> WaypointDirectObstacleDiscriminationGateCheckV1 {
    WaypointDirectObstacleDiscriminationGateCheckV1 {
        gate_id: gate_id.to_owned(),
        passed,
        observed: observed.into(),
        required: required.into(),
    }
}

fn load_historical_terrain_inputs(repo_root: &Path) -> Result<Vec<HistoricalTerrainInput>> {
    let path = repo_root.join(WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY);
    verify_file_sha256(
        &path,
        WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY_SHA256,
    )?;
    let document: serde_json::Value = serde_json::from_slice(
        &fs::read(&path).with_context(|| format!("reading {}", path.display()))?,
    )?;
    let overflight = document
        .get("overflight_control")
        .and_then(|group| group.get("probe"))
        .context("historical summary lacks overflight_control.probe")?;
    let boundary = document
        .get("boundary_cells")
        .and_then(serde_json::Value::as_array)
        .and_then(|cells| {
            cells.iter().find(|cell| {
                cell.get("id").and_then(serde_json::Value::as_str)
                    == Some("center_035_width_025_height_055")
            })
        })
        .and_then(|cell| cell.get("probe"))
        .context("historical summary lacks the sealed boundary probe")?;
    let out_of_envelope = document
        .get("out_of_envelope_control")
        .and_then(|group| group.get("probe"))
        .context("historical summary lacks out_of_envelope_control.probe")?;
    let inputs = [
        ("overflight_control", overflight),
        ("boundary_cells", boundary),
        ("out_of_envelope_control", out_of_envelope),
    ];
    inputs
        .into_iter()
        .map(|(role, probe)| {
            let probe_id = probe
                .get("id")
                .and_then(serde_json::Value::as_str)
                .with_context(|| format!("historical {role} probe lacks an id"))?
                .to_owned();
            let points = probe
                .get("terrain_points_m")
                .context("historical probe lacks terrain_points_m")?;
            let points_m: Vec<Vec2> = serde_json::from_value(points.clone())?;
            if points_m.len() < 2
                || points_m.windows(2).any(|pair| pair[0].x >= pair[1].x)
                || points_m
                    .iter()
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
            {
                bail!("historical {role} terrain geometry is malformed");
            }
            Ok(HistoricalTerrainInput {
                role,
                probe_id,
                points_m,
            })
        })
        .collect()
}

fn set_development_descriptive_metadata(scenario: &mut ScenarioSpec, case_id: &str, role: &str) {
    scenario.id = format!("waypoint-direct-obstacle-development-{case_id}");
    scenario.name = format!("Direct obstacle discrimination development: {case_id}");
    scenario.description =
        "Historical terrain topology input used only for bounded evaluator discrimination."
            .to_owned();
    if !scenario
        .tags
        .iter()
        .any(|tag| tag == "direct_obstacle_discrimination_development")
    {
        scenario
            .tags
            .push("direct_obstacle_discrimination_development".to_owned());
    }
    scenario.metadata.insert(
        "research".to_owned(),
        WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_SCHEMA_ID.to_owned(),
    );
    scenario
        .metadata
        .insert("case_id".to_owned(), case_id.to_owned());
    scenario
        .metadata
        .insert("historical_role".to_owned(), role.to_owned());
}

fn source_target_span_m(scenario: &ScenarioSpec) -> f64 {
    let Some(source) = scenario.world.landing_pad("pad_source") else {
        return f64::NAN;
    };
    let Some(target) = scenario.world.landing_pad("pad_main") else {
        return f64::NAN;
    };
    (target.center_x_m - source.center_x_m).abs()
}

fn maximum_terrain_height_m(scenario: &ScenarioSpec) -> f64 {
    scenario
        .world
        .terrain
        .points()
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
}

fn span_key(span_m: f64) -> u64 {
    span_m.round() as u64
}

fn create_runner_output_dir(repo_root: &Path, requested: &Path) -> Result<PathBuf> {
    let output_root = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        repo_root.join(requested)
    };
    if output_root.exists() {
        bail!("create-only obstacle-discrimination output directory already exists");
    }
    if let Some(parent) = output_root.parent()
        && !parent.is_dir()
    {
        bail!("output directory parent must already exist");
    }
    fs::create_dir(&output_root).with_context(|| {
        format!(
            "creating create-only output directory {}",
            output_root.display()
        )
    })?;
    Ok(output_root)
}

fn write_create_only_summary<T: Serialize>(output_root: &Path, value: &T) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_root.join("summary.json"))
        .context("create-only obstacle-discrimination summary write failed")?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn load_development_artifact(
    path: &Path,
) -> Result<WaypointDirectObstacleDiscriminationArtifactV1> {
    let artifact: WaypointDirectObstacleDiscriminationArtifactV1 = serde_json::from_slice(
        &fs::read(path)
            .with_context(|| format!("reading development summary {}", path.display()))?,
    )?;
    if artifact.schema_id != WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_SCHEMA_ID
        || artifact.schema_version != WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_SCHEMA_VERSION
        || artifact.phase != "development"
        || artifact_identity(&artifact)? != artifact.identity
    {
        bail!("development artifact schema or semantic identity mismatch");
    }
    Ok(artifact)
}

fn require_passed_development(
    development: &WaypointDirectObstacleDiscriminationArtifactV1,
) -> Result<()> {
    if !development.passed || development.gate_checks.iter().any(|check| !check.passed) {
        bail!("development gates did not pass; fresh evaluation is prohibited");
    }
    Ok(())
}

fn validate_code_freeze_binding(
    development: &WaypointDirectObstacleDiscriminationArtifactV1,
    freeze: &WaypointDirectObstacleDiscriminationCodeFreezeV1,
) -> Result<()> {
    if freeze.schema_id != "waypoint_direct_obstacle_discrimination_code_freeze_v1"
        || freeze.schema_version != 1
        || freeze.development_identity != development.identity
        || freeze.development_production_inputs_identity != development.production_inputs_identity
        || freeze.fresh_manifest_sha256
            != WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST_SHA256
        || freeze.historical_summary_sha256
            != WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY_SHA256
        || code_freeze_identity(freeze)? != freeze.identity
    {
        bail!("code freeze does not bind the accepted development and sealed inputs");
    }
    Ok(())
}

fn artifact_identity(artifact: &WaypointDirectObstacleDiscriminationArtifactV1) -> Result<String> {
    let mut input = artifact.clone();
    input.identity.clear();
    semantic_digest(&input)
}

fn code_freeze_identity(
    freeze: &WaypointDirectObstacleDiscriminationCodeFreezeV1,
) -> Result<String> {
    let mut input = freeze.clone();
    input.identity.clear();
    semantic_digest(&input)
}

fn generated_artifact_identity(
    artifact: &WaypointDirectNominalDirectGenerationArtifact,
) -> Result<String> {
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

fn production_inputs_identity(repo_root: &Path) -> Result<String> {
    fn collect_rust_sources(
        root: &Path,
        directory: &Path,
        entries: &mut Vec<(String, Vec<u8>)>,
    ) -> Result<()> {
        let mut paths = fs::read_dir(directory)?
            .map(|entry| entry.map(|item| item.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        paths.sort();
        for path in paths {
            if path.is_dir() {
                collect_rust_sources(root, &path, entries)?;
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                entries.push((
                    path.strip_prefix(root)?.to_string_lossy().into_owned(),
                    fs::read(&path)?,
                ));
            }
        }
        Ok(())
    }

    let mut entries = Vec::new();
    for crate_name in SOURCE_CRATES {
        let source = repo_root.join(crate_name).join("src");
        collect_rust_sources(repo_root, &source, &mut entries)?;
        let manifest_path = format!("{crate_name}/Cargo.toml");
        entries.push((
            manifest_path.clone(),
            fs::read(repo_root.join(manifest_path))?,
        ));
    }
    for path in HASHED_INPUT_FILES.into_iter().chain([
        WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST,
        WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY,
    ]) {
        entries.push((path.to_owned(), fs::read(repo_root.join(path))?));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    semantic_digest(&entries)
}

fn verify_file_sha256(path: &Path, expected: &str) -> Result<String> {
    let output = Command::new("sha256sum")
        .arg("--")
        .arg(path)
        .output()
        .with_context(|| format!("checking sealed digest of {}", path.display()))?;
    if !output.status.success() {
        bail!("SHA-256 command failed for {}", path.display());
    }
    let actual = std::str::from_utf8(&output.stdout)?
        .split_whitespace()
        .next()
        .unwrap_or_default();
    if actual != expected {
        bail!("sealed artifact SHA-256 mismatch: {}", path.display());
    }
    Ok(actual.to_owned())
}

fn verify_file_sha256_output(path: &Path) -> Result<String> {
    let output = Command::new("sha256sum")
        .arg("--")
        .arg(path)
        .output()
        .with_context(|| format!("hashing generated summary {}", path.display()))?;
    if !output.status.success() {
        bail!("SHA-256 command failed for {}", path.display());
    }
    Ok(std::str::from_utf8(&output.stdout)?
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_plan::conservative_ballistic_bridge::CertificationV2;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    fn repo_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
    }

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "pd-obstacle-discrimination-{label}-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("unique temporary test directory");
        path
    }

    fn failed_development_artifact() -> WaypointDirectObstacleDiscriminationArtifactV1 {
        let mut development = WaypointDirectObstacleDiscriminationArtifactV1 {
            schema_id: WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_SCHEMA_ID.to_owned(),
            schema_version: WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_SCHEMA_VERSION,
            phase: "development".to_owned(),
            fresh_manifest_sha256: WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST_SHA256
                .to_owned(),
            historical_summary_sha256: Some(
                WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY_SHA256.to_owned(),
            ),
            production_inputs_identity: "source".to_owned(),
            cases: Vec::new(),
            gate_checks: vec![gate_check("deliberate_failure", false, "false", "true")],
            passed: false,
            verdict: "failed".to_owned(),
            scope_non_claims: Vec::new(),
            identity: String::new(),
        };
        development.identity = artifact_identity(&development).unwrap();
        development
    }

    fn synthetic_unknown_generation_artifact(
        request: &WaypointDirectNominalDirectGenerationRequest,
    ) -> WaypointDirectNominalDirectGenerationArtifact {
        let bases = (0..4)
            .map(
                |basis_index| crate::WaypointDirectNominalDirectGenerationBasisEvidence {
                    basis_index,
                    candidate_identity: format!("candidate-{basis_index}"),
                    generated_basis_identity: format!("generated-basis-{basis_index}"),
                    duration_multiplier: request.policy.analytical_policy.duration_multipliers
                        [basis_index],
                    original_v2_classification: CertificationV2::NotCertified,
                    original_v2_reasons: vec!["NoSurvivingSourceHandoff".to_owned()],
                    original_total_time_s: None,
                    original_total_fuel_burn_kg: None,
                    original_source_handoff: None,
                    original_source_bridge_tick_count: None,
                    coast_tick_count: None,
                    terminal_bridge_tick_count: None,
                },
            )
            .collect::<Vec<_>>();
        let rows = (0..20)
            .map(|row_index| {
                let basis_index = row_index / 5;
                crate::WaypointDirectNominalDirectGenerationRowEvidence {
                    row_index,
                    basis_index,
                    basis_candidate_identity: bases[basis_index].candidate_identity.clone(),
                    generated_basis_identity: bases[basis_index].generated_basis_identity.clone(),
                    duration_offset_ticks: SOURCE_DURATION_OFFSETS_TICKS[row_index % 5],
                    source_bridge_tick_count: None,
                    original_v2_classification: CertificationV2::NotCertified,
                    source_handoff: None,
                    analytical_survivor: false,
                    source_handoff_survivor: false,
                    status: "skipped_noncertified_basis".to_owned(),
                    skip_reason: Some("base_v2_not_certified".to_owned()),
                    source_duration: None,
                    paired_schedule: None,
                    wrapper: None,
                    accepted: false,
                }
            })
            .collect::<Vec<_>>();
        let mut artifact = WaypointDirectNominalDirectGenerationArtifact {
            schema_id: "waypoint_direct_nominal_direct_generation_v1".to_owned(),
            schema_version: 1,
            characterization_id: "waypoint-direct-nominal-direct-generation".to_owned(),
            probe_id: request.probe_id.clone(),
            source_pad_id: request.source_pad_id.clone(),
            target_pad_id: request.target_pad_id.clone(),
            scenario_identity: semantic_digest(&request.scenario).unwrap(),
            policy_identity: semantic_digest(&request.policy).unwrap(),
            request: request.clone(),
            policy: request.policy.clone(),
            provenance_convention: "synthetic test only".to_owned(),
            scope_non_claims: Vec::new(),
            bases,
            rows,
            family_proof: crate::WaypointDirectNominalDirectGenerationFamilyProofEvidence {
                basis_count: 4,
                expected_basis_count: 4,
                row_count: 20,
                expected_row_count: 20,
                retained_noncertified_basis_count: 4,
                analytical_skip_count: 20,
                scheduled_count: 0,
                completed_acceptance_count: 0,
                accepted_witness_count: 0,
                omitted_row_count: 0,
                compute_counters: Default::default(),
                all_predeclared_rows_recorded: true,
                completion_gate_passed: false,
                stopping_result: UNKNOWN_STATUS.to_owned(),
            },
            selection: None,
            selected_physical_witness: None,
            execution_status: UNKNOWN_STATUS.to_owned(),
            identity: String::new(),
        };
        artifact.identity = generated_artifact_identity(&artifact).unwrap();
        artifact
    }

    fn synthetic_fresh_case(
        index: usize,
        profile: &str,
        directness: WaypointDirectObstacleDiscriminationDirectnessV1,
        blocked: bool,
    ) -> WaypointDirectObstacleDiscriminationCaseEvidenceV1 {
        let lane = WaypointDirectObstacleDiscriminationGeneratorLaneV1 {
            attempted: true,
            directness,
            generation_identity: Some(format!("generation-{index}")),
            generated_summary_sha256: None,
            row_count: 20,
            scheduled_count: 0,
            accepted_count: 0,
            ledger: WaypointDirectObstacleDiscriminationLedgerAuditV1 {
                valid: true,
                row_count: 20,
                expected_row_count: 20,
                scheduled_count_matches: true,
                accepted_count_matches: true,
                all_rows_are_ordered_and_bound: true,
                family_proof_is_coherent: true,
                artifact_identity_matches: true,
                request_matches: true,
                selection_is_coherent: true,
                failures: Vec::new(),
            },
            accepted_complete_existing_gates: true,
            basis_reasons: Vec::new(),
            row_reasons: Vec::new(),
            failure_kind: None,
        };
        let twin_evidence = FixedCommandTerrainTwinEvidence {
            baseline_wrapper_identity: "wrapper".to_owned(),
            terrain_scenario_identity: "scenario".to_owned(),
            logged_tick_count: 2,
            physics_ticks_replayed: 2,
            commands_and_states_identical_until_terrain_contact: true,
            ordinary_neutral_states_match: true,
            first_terrain_contact_step: None,
            first_contact_classification: None,
            first_contact_state: None,
            clearance_scan: crate::GeometryClearanceScanEvidence {
                poststep_state_count: 2,
                airborne_state_count: 2,
                source_corridor_state_count: 0,
                terminal_corridor_state_count: 0,
                exact_clearance_query_count: 2,
                all_airborne_states_passed: !blocked,
                first_violation: None,
                minimum_airborne: None,
            },
            terrain_blocked: blocked,
            first_command_mismatch: None,
            identity: "twin".to_owned(),
        };
        WaypointDirectObstacleDiscriminationCaseEvidenceV1 {
            case_id: format!("case-{index}"),
            probe_id: format!("case-{index}"),
            historical_role: None,
            profile: profile.to_owned(),
            horizontal_span_m: if index < 4 { 700.0 } else { 900.0 },
            maximum_terrain_height_m: if blocked { 100.0 } else { 0.0 },
            scenario_identity: "scenario".to_owned(),
            generator: lane,
            fixed_command_twin: WaypointDirectObstacleDiscriminationTwinLaneV1 {
                status: WaypointDirectObstacleDiscriminationTwinStatusV1::Evaluated,
                baseline_case_id: Some("flat".to_owned()),
                baseline_generation_identity: Some("baseline".to_owned()),
                evidence: Some(twin_evidence),
                prefix_parity_valid: true,
                failure_kind: None,
            },
            classification: WaypointDirectObstacleDiscriminationClassificationV1::FiniteUnknown,
            classification_basis: "synthetic".to_owned(),
        }
    }

    #[test]
    fn sealed_manifest_has_the_exact_eight_case_order_and_default_policy() {
        let manifest = load_waypoint_direct_obstacle_discrimination_fresh_manifest(repo_root())
            .expect("sealed manifest loads without solving");
        let ids = manifest
            .cases
            .iter()
            .map(|case| case.case_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            [
                "fresh_flat_control_span_700",
                "fresh_low_obstacle_span_700",
                "fresh_high_obstacle_span_700",
                "fresh_late_broad_span_700",
                "fresh_flat_control_span_900",
                "fresh_low_obstacle_span_900",
                "fresh_high_obstacle_span_900",
                "fresh_late_broad_span_900",
            ]
        );
        assert_eq!(
            manifest.generation_policy,
            WaypointDirectNominalDirectGenerationPolicyV1::default()
        );
        assert!(
            manifest
                .cases
                .iter()
                .all(|case| case.scenario.mission.transfer_route.is_none())
        );
    }

    #[test]
    fn unaccepted_development_cannot_be_frozen_or_used_for_fresh() {
        let temp = temp_dir("failed-development");
        let development_path = temp.join("development.json");
        let development = failed_development_artifact();
        fs::write(&development_path, serde_json::to_vec(&development).unwrap()).unwrap();
        let freeze_output = temp.join("freeze");
        let fresh_output = temp.join("fresh");
        assert!(
            freeze_waypoint_direct_obstacle_discrimination(
                repo_root(),
                &development_path,
                &freeze_output,
            )
            .is_err()
        );
        assert!(
            run_waypoint_direct_obstacle_discrimination_fresh(
                repo_root(),
                &development_path,
                &temp.join("missing-freeze.json"),
                &fresh_output,
            )
            .is_err()
        );
        assert!(!freeze_output.exists());
        assert!(!fresh_output.exists());
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn manifest_digest_rejects_tampered_bytes() {
        let temp = temp_dir("manifest-tamper");
        let path = temp.join("manifest.json");
        fs::write(&path, b"{}\n").unwrap();
        assert!(
            verify_file_sha256(
                &path,
                WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST_SHA256
            )
            .is_err()
        );
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn output_directory_is_create_only() {
        let temp = temp_dir("create-only");
        let output = temp.join("run");
        create_runner_output_dir(repo_root(), &output).unwrap();
        assert!(create_runner_output_dir(repo_root(), &output).is_err());
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn fresh_profile_policy_does_not_promote_unknown_to_direct() {
        let mut cases = vec![
            synthetic_fresh_case(
                0,
                "flat_control",
                WaypointDirectObstacleDiscriminationDirectnessV1::Direct,
                false,
            ),
            synthetic_fresh_case(
                1,
                "low_obstacle",
                WaypointDirectObstacleDiscriminationDirectnessV1::Direct,
                false,
            ),
            synthetic_fresh_case(
                2,
                "high_obstacle",
                WaypointDirectObstacleDiscriminationDirectnessV1::Unknown,
                true,
            ),
            synthetic_fresh_case(
                3,
                "late_broad",
                WaypointDirectObstacleDiscriminationDirectnessV1::Unknown,
                false,
            ),
            synthetic_fresh_case(
                4,
                "flat_control",
                WaypointDirectObstacleDiscriminationDirectnessV1::Direct,
                false,
            ),
            synthetic_fresh_case(
                5,
                "low_obstacle",
                WaypointDirectObstacleDiscriminationDirectnessV1::Direct,
                false,
            ),
            synthetic_fresh_case(
                6,
                "high_obstacle",
                WaypointDirectObstacleDiscriminationDirectnessV1::Unknown,
                false,
            ),
            synthetic_fresh_case(
                7,
                "late_broad",
                WaypointDirectObstacleDiscriminationDirectnessV1::Unknown,
                false,
            ),
        ];
        let checks = fresh_gate_checks(&cases, 8, true);
        assert!(checks.iter().all(|check| check.passed), "{checks:#?}");
        assert_eq!(cases.len(), 8, "all sealed rows remain represented");

        cases[1].generator.directness = WaypointDirectObstacleDiscriminationDirectnessV1::Unknown;
        let checks = fresh_gate_checks(&cases, 8, true);
        assert!(
            !checks
                .iter()
                .find(|check| check.gate_id == "profile_directness_policy")
                .unwrap()
                .passed
        );
    }

    #[test]
    fn valid_twenty_row_unknown_artifact_passes_ledger_audit_without_completion_gate() {
        let request = waypoint_direct_known_flat_generation_request(repo_root()).unwrap();
        let artifact = synthetic_unknown_generation_artifact(&request);
        let audit = audit_generator_artifact(&artifact, &request, None);
        assert!(audit.ledger.valid, "{:#?}", audit.ledger);
        assert!(!artifact.family_proof.completion_gate_passed);
        assert_eq!(
            audit.directness,
            WaypointDirectObstacleDiscriminationDirectnessV1::Unknown
        );

        let mut missing = artifact.clone();
        missing.rows.pop();
        missing.identity = generated_artifact_identity(&missing).unwrap();
        let missing_audit = audit_generator_artifact(&missing, &request, None);
        assert!(!missing_audit.ledger.valid);
        assert!(!missing_audit.ledger.all_rows_are_ordered_and_bound);

        let mut reordered = artifact;
        reordered.rows.swap(0, 1);
        reordered.identity = generated_artifact_identity(&reordered).unwrap();
        let reordered_audit = audit_generator_artifact(&reordered, &request, None);
        assert!(!reordered_audit.ledger.valid);
        assert!(!reordered_audit.ledger.all_rows_are_ordered_and_bound);
    }

    #[test]
    fn freeze_requires_matching_development_and_rejects_tampering() {
        let mut development = failed_development_artifact();
        development.passed = true;
        development.gate_checks.clear();
        development.identity = artifact_identity(&development).unwrap();
        let mut freeze = WaypointDirectObstacleDiscriminationCodeFreezeV1 {
            schema_id: "waypoint_direct_obstacle_discrimination_code_freeze_v1".to_owned(),
            schema_version: 1,
            development_identity: development.identity.clone(),
            development_production_inputs_identity: development.production_inputs_identity.clone(),
            fresh_manifest_sha256: WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_FRESH_MANIFEST_SHA256
                .to_owned(),
            historical_summary_sha256:
                WAYPOINT_DIRECT_OBSTACLE_DISCRIMINATION_HISTORICAL_SUMMARY_SHA256.to_owned(),
            production_inputs_identity: development.production_inputs_identity.clone(),
            identity: String::new(),
        };
        freeze.identity = code_freeze_identity(&freeze).unwrap();
        assert!(validate_code_freeze_binding(&development, &freeze).is_ok());

        freeze.identity.push('0');
        assert!(validate_code_freeze_binding(&development, &freeze).is_err());
        freeze.identity.pop();
        freeze.development_identity = "different-development".to_owned();
        freeze.identity = code_freeze_identity(&freeze).unwrap();
        assert!(validate_code_freeze_binding(&development, &freeze).is_err());
    }

    #[test]
    fn reason_categories_keep_source_and_terminal_handoff_distinct() {
        assert_eq!(
            reason_record(
                0,
                "original_v2_reasons",
                "NoSurvivingSourceHandoff".to_owned()
            )
            .category,
            WaypointDirectObstacleDiscriminationReasonCategoryV1::Source
        );
        assert_eq!(
            reason_record(
                0,
                "first_failing_gate",
                "terminal_handoff_on_descending_arc".to_owned()
            )
            .category,
            WaypointDirectObstacleDiscriminationReasonCategoryV1::TerminalContact
        );
        assert_eq!(
            reason_record(
                0,
                "first_failing_gate",
                "pointwise_core_geometry_clearance".to_owned()
            )
            .category,
            WaypointDirectObstacleDiscriminationReasonCategoryV1::TerrainGeometry
        );
    }
}
