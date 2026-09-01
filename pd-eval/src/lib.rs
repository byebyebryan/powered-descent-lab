use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_control::{
    ControlledRunArtifacts, ControllerSpec, ControllerUpdateRecord, RunPerformanceStats,
    TelemetryValue, built_in_controller_spec, marker, metric, run_controller_spec,
};
use pd_core::{
    EndReason, EvaluationGoal, EventRecord, LandingPadSpec, MissionOutcome, Observation,
    PhysicalOutcome, PlannerComputeEvidence, RoutePlan, RoutePlanningPolicy, RoutePlanningRequest,
    RouteTopology, RunContext, RunManifest, RunSummary, SampleRecord, ScenarioSpec,
    TerrainDefinition, TransferRouteSpec, TransferWaypointSpec, Vec2, VehicleSpec,
    WaypointHandoffKinematics, build_endpoint_profile, validate_route,
};
use rayon::{ThreadPoolBuilder, prelude::*};
use serde::{Deserialize, Serialize};

pub mod report;
pub mod report_catalog;

#[cfg(unix)]
use std::os::unix::fs as platform_fs;
#[cfg(windows)]
use std::os::windows::fs as platform_fs;

pub const BATCH_REPORT_SCHEMA_VERSION: u32 = 36;

const TRANSFER_TERMINAL_REBOUND_ARM_HEIGHT_M: f64 = 25.0;
const TRANSFER_TERMINAL_REBOUND_NEAR_PAD_HALF_WIDTHS: f64 = 3.0;
const REGRESSION_POLICY_EPSILON: f64 = 1.0e-9;
const REGRESSION_POLICY_MEAN_SIM_TIME_WARN_DELTA_S: f64 = 1.0;

mod model;
pub use model::*;

mod comparison;
pub use comparison::compare_batch_reports;
#[cfg(test)]
pub(crate) use comparison::run_pointer;
pub(crate) use comparison::{metric_summary, success_rate, summarize_records};

mod runtime;
use runtime::*;

mod resolution;
use resolution::*;

mod review;
use review::*;

mod execution;
use execution::*;

mod source_transition;
pub use source_transition::*;

mod route_execution;
pub use route_execution::*;

mod route_capability;
pub use route_capability::*;

mod progress_interval_envelope;
pub use progress_interval_envelope::*;

mod terrain_equivalence_spike;
pub use terrain_equivalence_spike::*;

#[derive(Clone, Debug)]
struct WorkspaceState {
    commit_key: String,
    workspace_key: String,
    dirty: bool,
}

#[derive(Clone, Debug)]
struct ResolvedBatchRun {
    descriptor: ResolvedRunDescriptor,
    scenario: ScenarioSpec,
}

pub fn load_pack(path: &Path) -> Result<ScenarioPackSpec> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario pack file {}", path.display()))?;
    let pack: ScenarioPackSpec = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse scenario pack json {}", path.display()))?;
    validate_pack(&pack)?;
    Ok(pack)
}

/// Inputs accepted by the development-only gate.  The baseline is resolved by
/// the maintained pack resolver; the diagnostic corpus is an immutable archive
/// replay.  Keeping this small adapter separate avoids forging ordinary batch
/// descriptors for archived inputs.
struct SourceTransitionGateRunInput<'a> {
    run_id: &'a str,
    scenario: &'a ScenarioSpec,
    route_plan: &'a RoutePlan,
    controller_spec: &'a ControllerSpec,
}

impl<'a> SourceTransitionGateRunInput<'a> {
    fn from_baseline(run: &'a ResolvedBatchRun) -> Result<Self> {
        Ok(Self {
            run_id: &run.descriptor.run_id,
            scenario: &run.scenario,
            route_plan: run
                .descriptor
                .route_plan
                .as_ref()
                .ok_or_else(|| anyhow!("resolved D0a baseline run has no route plan"))?,
            controller_spec: &run.descriptor.controller_spec,
        })
    }

    fn from_archived(case: &'a SourceTransitionDiagnosticInputCase) -> Self {
        Self {
            run_id: &case.run_id,
            scenario: &case.scenario,
            route_plan: &case.route_plan,
            controller_spec: &case.controller,
        }
    }
}

/// Run the development-only D0a source-transition gate.  This resolves the
/// committed baseline and validates immutable diagnostic input snapshots,
/// captures each case at ordinary and physics-rate retention, and writes only
/// physics-rate raw bundles plus neutral source evidence to `output_dir`.
pub fn run_source_transition_development_gate(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
) -> Result<SourceTransitionDevelopmentGateSummary> {
    run_source_transition_development_gate_filtered(manifest_path, repo_root, output_dir, None)
}

/// Run one explicitly selected D0a case after validating the complete
/// manifest and both input corpora.  This is a non-authoritative inspection
/// probe; the full gate remains the no-filter command.
pub fn run_source_transition_development_case(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
    case_id: &str,
) -> Result<SourceTransitionDevelopmentGateSummary> {
    if case_id.trim().is_empty() {
        bail!("D0a case filter must not be empty");
    }
    run_source_transition_development_gate_filtered(
        manifest_path,
        repo_root,
        output_dir,
        Some(case_id),
    )
}

fn run_source_transition_development_gate_filtered(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
    case_filter: Option<&str>,
) -> Result<SourceTransitionDevelopmentGateSummary> {
    let manifest = load_source_transition_development_manifest(manifest_path)?;
    let baseline_pack_path = resolve_source_transition_path(repo_root, &manifest.baseline_pack);
    let baseline_pack = load_pack(&baseline_pack_path)?;
    let baseline_pack_digest = format!(
        "fnv1a64:{}",
        source_transition_canonical_digest(&baseline_pack)
    );
    if baseline_pack_digest != manifest.baseline_pack_digest {
        bail!(
            "D0a baseline pack digest drift: expected {}, resolved {}",
            manifest.baseline_pack_digest,
            baseline_pack_digest
        );
    }
    let baseline_base_dir = baseline_pack_path
        .parent()
        .ok_or_else(|| anyhow!("baseline pack path has no parent"))?;
    let baseline_runs = resolve_pack_runs(&baseline_pack, baseline_base_dir)?;
    let baseline_expected_ids = manifest
        .baseline_cases
        .iter()
        .flat_map(SourceTransitionDevelopmentCase::resolved_case_keys)
        .collect::<Vec<_>>();
    let baseline_actual_ids = baseline_runs
        .iter()
        .map(|run| run.descriptor.run_id.clone())
        .collect::<Vec<_>>();
    validate_source_transition_resolved_ids(
        "baseline",
        baseline_expected_ids,
        baseline_actual_ids,
        manifest.baseline_expected_case_count,
    )?;
    let baseline_resolved_input_digest =
        source_transition_resolved_input_corpus_digest(&baseline_runs)?;
    if baseline_resolved_input_digest != manifest.baseline_resolved_input_digest {
        bail!(
            "D0a baseline resolved-input digest drift: expected {}, resolved {}",
            manifest.baseline_resolved_input_digest,
            baseline_resolved_input_digest
        );
    }

    let diagnostic_pack_path =
        resolve_source_transition_path(repo_root, &manifest.diagnostic_source_pack);
    let diagnostic_pack = load_source_transition_diagnostic_input_pack(&diagnostic_pack_path)?;
    if diagnostic_pack.source_commit != manifest.diagnostic_source_commit {
        bail!(
            "D0a diagnostic source commit mismatch: manifest {}, pack {}",
            manifest.diagnostic_source_commit,
            diagnostic_pack.source_commit
        );
    }
    let diagnostic_pack_digest = format!(
        "fnv1a64:{}",
        source_transition_canonical_digest(&diagnostic_pack)
    );
    if diagnostic_pack_digest != manifest.diagnostic_pack_digest {
        bail!(
            "D0a diagnostic input-pack digest drift: expected {}, resolved {}",
            manifest.diagnostic_pack_digest,
            diagnostic_pack_digest
        );
    }
    let diagnostic_expected_ids = manifest
        .diagnostic_cases
        .iter()
        .flat_map(SourceTransitionDevelopmentCase::resolved_case_keys)
        .collect::<Vec<_>>();
    let diagnostic_actual_ids = diagnostic_pack
        .cases
        .iter()
        .map(|case| case.run_id.clone())
        .collect::<Vec<_>>();
    validate_source_transition_resolved_ids(
        "diagnostic",
        diagnostic_expected_ids,
        diagnostic_actual_ids,
        manifest.diagnostic_expected_case_count,
    )?;
    let diagnostic_resolved_input_digest =
        source_transition_archived_input_corpus_digest(&diagnostic_pack.cases);
    if diagnostic_resolved_input_digest != manifest.diagnostic_resolved_input_digest {
        bail!(
            "D0a diagnostic resolved-input digest drift: expected {}, resolved {}",
            manifest.diagnostic_resolved_input_digest,
            diagnostic_resolved_input_digest
        );
    }

    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create D0a development output directory {}",
            output_dir.display()
        )
    })?;

    let baseline_selected = baseline_runs
        .iter()
        .filter(|run| case_filter.is_none_or(|case_id| run.descriptor.run_id == case_id))
        .collect::<Vec<_>>();
    let diagnostic_selected = diagnostic_pack
        .cases
        .iter()
        .filter(|case| case_filter.is_none_or(|case_id| case.run_id == case_id))
        .collect::<Vec<_>>();
    if case_filter.is_some() && baseline_selected.len() + diagnostic_selected.len() != 1 {
        bail!(
            "D0a case filter did not select exactly one resolved case: {}",
            case_filter.unwrap_or_default()
        );
    }

    let mut summary = SourceTransitionDevelopmentGateSummary {
        schema_version: SOURCE_TRANSITION_GATE_SCHEMA_VERSION,
        manifest_id: manifest.manifest_id.clone(),
        input_digest: manifest.input_digest.clone(),
        case_filter: case_filter.map(ToOwned::to_owned),
        baseline_total_count: baseline_selected.len(),
        diagnostic_total_count: diagnostic_selected.len(),
        baseline_contract_passes: 0,
        baseline_contract_failures: Vec::new(),
        parity_passes: 0,
        parity_failures: Vec::new(),
        evidence_status_counts: BTreeMap::new(),
        invalidations: Vec::new(),
        deterministic_replay_passes: 0,
        deterministic_replay_failures: Vec::new(),
        overall_passed: false,
        failure_reasons: Vec::new(),
    };

    for run in baseline_selected {
        let input = SourceTransitionGateRunInput::from_baseline(run)?;
        record_source_transition_gate_case(&mut summary, &input, "baseline", output_dir, true);
    }
    for case in diagnostic_selected {
        let input = SourceTransitionGateRunInput::from_archived(case);
        record_source_transition_gate_case(&mut summary, &input, "diagnostic", output_dir, false);
    }

    if !summary.baseline_contract_failures.is_empty() {
        summary
            .failure_reasons
            .push("baseline_contract_failed".to_owned());
    }
    if !summary.parity_failures.is_empty() {
        summary
            .failure_reasons
            .push("cadence_parity_failed".to_owned());
    }
    if !summary.invalidations.is_empty() {
        summary
            .failure_reasons
            .push("source_transition_invalidated".to_owned());
    }
    if !summary.deterministic_replay_failures.is_empty() {
        summary
            .failure_reasons
            .push("deterministic_replay_failed".to_owned());
    }
    summary.overall_passed = summary.failure_reasons.is_empty();
    let summary_path = output_dir.join("summary.json");
    fs::write(&summary_path, serde_json::to_string_pretty(&summary)?).with_context(|| {
        format!(
            "failed to write D0a development summary {}",
            summary_path.display()
        )
    })?;
    Ok(summary)
}

fn resolve_source_transition_path(repo_root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    }
}

fn validate_source_transition_resolved_ids(
    corpus: &str,
    expected_ids: Vec<String>,
    actual_ids: Vec<String>,
    expected_count: usize,
) -> Result<()> {
    if expected_ids.len() != expected_count || actual_ids.len() != expected_count {
        bail!(
            "D0a {corpus} resolved count mismatch: expected {expected_count}, manifest {}, resolved {}",
            expected_ids.len(),
            actual_ids.len()
        );
    }
    if actual_ids != expected_ids {
        bail!(
            "D0a {corpus} resolved run IDs drift from manifest: expected {:?}, resolved {:?}",
            expected_ids,
            actual_ids
        );
    }
    Ok(())
}

fn source_transition_resolved_input_corpus_digest(runs: &[ResolvedBatchRun]) -> Result<String> {
    let identities = runs
        .iter()
        .map(|run| {
            let route_plan = run
                .descriptor
                .route_plan
                .as_ref()
                .ok_or_else(|| anyhow!("resolved D0a run has no route plan"))?;
            Ok(source_transition_resolved_input_digest(
                &run.descriptor.run_id,
                &run.scenario,
                route_plan,
                &run.descriptor.controller_spec,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(format!(
        "fnv1a64:{}",
        source_transition_canonical_digest(&identities)
    ))
}

fn source_transition_archived_input_corpus_digest(
    cases: &[SourceTransitionDiagnosticInputCase],
) -> String {
    let identities = cases
        .iter()
        .map(|case| {
            source_transition_resolved_input_digest(
                &case.run_id,
                &case.scenario,
                &case.route_plan,
                &case.controller,
            )
        })
        .collect::<Vec<_>>();
    format!(
        "fnv1a64:{}",
        source_transition_canonical_digest(&identities)
    )
}

struct SourceTransitionGateCaseResult {
    parity: SourceTransitionCadenceParity,
    evidence: SourceTransitionEvidence,
    replay_stable: bool,
    baseline_contract_pass: bool,
}

fn record_source_transition_gate_case(
    summary: &mut SourceTransitionDevelopmentGateSummary,
    input: &SourceTransitionGateRunInput<'_>,
    corpus: &str,
    output_dir: &Path,
    is_baseline: bool,
) {
    let case_result = run_source_transition_gate_case(input, corpus, output_dir);
    let case_result = match case_result {
        Ok(result) => result,
        Err(error) => {
            summary.invalidations.push(SourceTransitionGateFailure {
                run_id: input.run_id.to_owned(),
                reason: format!("capture_failed:{error}"),
            });
            *summary
                .evidence_status_counts
                .entry("invalid".to_owned())
                .or_default() += 1;
            return;
        }
    };

    if is_baseline {
        if case_result.baseline_contract_pass {
            summary.baseline_contract_passes += 1;
        } else {
            summary
                .baseline_contract_failures
                .push(SourceTransitionGateFailure {
                    run_id: input.run_id.to_owned(),
                    reason: "mission_success_checkpoint_satisfied_while_flying_not_observed"
                        .to_owned(),
                });
        }
    }
    if case_result.parity.passed {
        summary.parity_passes += 1;
    } else {
        summary.parity_failures.push(SourceTransitionGateFailure {
            run_id: input.run_id.to_owned(),
            reason: case_result.parity.mismatch_reasons.join(","),
        });
    }

    let status = serde_json::to_string(&case_result.evidence.status)
        .unwrap_or_else(|_| "\"invalid\"".to_owned())
        .trim_matches('"')
        .to_owned();
    *summary.evidence_status_counts.entry(status).or_default() += 1;
    if let Some(reason) = &case_result.evidence.invalid_reason {
        summary.invalidations.push(SourceTransitionGateFailure {
            run_id: input.run_id.to_owned(),
            reason: reason.to_string(),
        });
    }
    if case_result.replay_stable {
        summary.deterministic_replay_passes += 1;
    } else {
        summary
            .deterministic_replay_failures
            .push(SourceTransitionGateFailure {
                run_id: input.run_id.to_owned(),
                reason: "source_evidence_bytes_or_digest_changed_on_repeat".to_owned(),
            });
    }
}

fn run_source_transition_gate_case(
    input: &SourceTransitionGateRunInput<'_>,
    corpus: &str,
    output_dir: &Path,
) -> Result<SourceTransitionGateCaseResult> {
    let context = RunContext::from_scenario(input.scenario)
        .map_err(|error| anyhow!("failed to build run context: {error}"))?;
    let (ordinary, physics) = run_source_transition_cadence_pair(&context, input.controller_spec)
        .map_err(|error| anyhow!("controller capture failed: {error}"))?;
    let physics_scenario = with_physics_rate_evidence_overlay(input.scenario);
    let parity = compare_source_transition_cadence_parity(
        input.scenario,
        &ordinary,
        &physics_scenario,
        &physics,
    );
    let bundle_dir = output_dir.join(corpus).join(input.run_id);
    write_source_transition_raw_bundle(&bundle_dir, &physics_scenario, input.route_plan, &physics)?;
    let mut provenance = source_transition_provenance_for_route_plan(
        &physics_scenario,
        input.route_plan,
        &physics.run,
        &physics.controller_updates,
        input.run_id.to_owned(),
    )?;
    provenance.resolved_input_digest = source_transition_resolved_input_digest(
        input.run_id,
        input.scenario,
        input.route_plan,
        input.controller_spec,
    );
    let evidence = assemble_source_transition_evidence_from_controlled_artifacts(
        &physics_scenario,
        input.route_plan,
        &physics,
        provenance.clone(),
    );
    validate_source_evidence_round_trip(&evidence)?;
    let evidence_path = bundle_dir.join("source_transition.json");
    write_source_transition_artifacts(&evidence_path, &evidence)?;
    let repeated = assemble_source_transition_evidence_from_controlled_artifacts(
        &physics_scenario,
        input.route_plan,
        &physics,
        provenance,
    );
    let replay_stable = serde_json::to_vec(&evidence)? == serde_json::to_vec(&repeated)?
        && evidence.evidence_digest == repeated.evidence_digest
        && evidence.physical_digest == repeated.physical_digest;
    let baseline_contract_pass = matches!(
        (
            ordinary.run.manifest.mission_outcome,
            ordinary.run.manifest.end_reason,
            ordinary.run.manifest.physical_outcome,
        ),
        (
            MissionOutcome::Success,
            EndReason::CheckpointSatisfied,
            PhysicalOutcome::Flying,
        )
    );
    Ok(SourceTransitionGateCaseResult {
        parity,
        evidence,
        replay_stable,
        baseline_contract_pass,
    })
}

/// Schema version for the sibling D0b route-execution development gate.
pub const ROUTE_EXECUTION_GATE_SCHEMA_VERSION: u32 = 1;

/// Deterministic machine-readable result of the development-only D0b gate.
/// Route evidence remains neutral; baseline contract checks and corpus IDs are
/// gate bookkeeping only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionDevelopmentGateSummary {
    pub schema_version: u32,
    pub manifest_id: String,
    pub input_digest: String,
    pub case_filter: Option<String>,
    pub baseline_total_count: usize,
    pub diagnostic_total_count: usize,
    pub baseline_contract_passes: usize,
    pub baseline_contract_failures: Vec<SourceTransitionGateFailure>,
    pub parity_passes: usize,
    pub parity_failures: Vec<SourceTransitionGateFailure>,
    pub source_status_counts: BTreeMap<String, usize>,
    pub route_status_counts: BTreeMap<String, usize>,
    pub source_invalidations: Vec<SourceTransitionGateFailure>,
    pub route_invalidations: Vec<SourceTransitionGateFailure>,
    pub deterministic_replay_passes: usize,
    pub deterministic_replay_failures: Vec<SourceTransitionGateFailure>,
    pub overall_passed: bool,
    pub failure_reasons: Vec<String>,
    pub summary_digest: String,
}

impl RouteExecutionDevelopmentGateSummary {
    pub fn validate(&self) -> std::result::Result<(), String> {
        let total = self.baseline_total_count + self.diagnostic_total_count;
        if self.schema_version != ROUTE_EXECUTION_GATE_SCHEMA_VERSION
            || self.manifest_id.trim().is_empty()
            || self.input_digest.trim().is_empty()
        {
            return Err("D0b summary schema or identity is invalid".to_owned());
        }
        if self.baseline_contract_passes + self.baseline_contract_failures.len()
            != self.baseline_total_count
            || self.parity_passes + self.parity_failures.len() != total
            || self.deterministic_replay_passes + self.deterministic_replay_failures.len() != total
            || self.source_status_counts.values().sum::<usize>() != total
            || self.route_status_counts.values().sum::<usize>() != total
        {
            return Err("D0b summary counts or pass flag are inconsistent".to_owned());
        }
        let mut expected_failure_reasons = Vec::new();
        if !self.baseline_contract_failures.is_empty() {
            expected_failure_reasons.push("baseline_contract_failed".to_owned());
        }
        if !self.parity_failures.is_empty() {
            expected_failure_reasons.push("cadence_parity_failed".to_owned());
        }
        if !self.source_invalidations.is_empty() {
            expected_failure_reasons.push("source_transition_invalidated".to_owned());
        }
        if !self.route_invalidations.is_empty() {
            expected_failure_reasons.push("route_execution_invalidated".to_owned());
        }
        if !self.deterministic_replay_failures.is_empty() {
            expected_failure_reasons.push("deterministic_replay_failed".to_owned());
        }
        if self.failure_reasons != expected_failure_reasons
            || self.overall_passed != expected_failure_reasons.is_empty()
        {
            return Err("D0b summary failure reasons or pass flag are inconsistent".to_owned());
        }
        let mut material = self.clone();
        material.summary_digest.clear();
        if self.summary_digest != source_transition_canonical_digest(&material) {
            return Err("D0b summary digest mismatch".to_owned());
        }
        Ok(())
    }

    fn seal(&mut self) -> std::result::Result<(), String> {
        self.summary_digest.clear();
        self.summary_digest = source_transition_canonical_digest(self);
        self.validate()
    }
}

/// Run the development-only D0b route-execution gate over both committed
/// corpora.  It captures the same ordinary/physics pair as D0a and writes the
/// physics raw bundle, nested source evidence, and route evidence.
pub fn run_route_execution_development_gate(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
) -> Result<RouteExecutionDevelopmentGateSummary> {
    run_route_execution_development_gate_filtered(manifest_path, repo_root, output_dir, None)
}

/// Run one explicitly selected D0b case after validating both complete input
/// corpora.  This is an inspection probe; the unfiltered invocation is the
/// authoritative gate.
pub fn run_route_execution_development_case(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
    case_id: &str,
) -> Result<RouteExecutionDevelopmentGateSummary> {
    if case_id.trim().is_empty() {
        bail!("D0b case filter must not be empty");
    }
    run_route_execution_development_gate_filtered(
        manifest_path,
        repo_root,
        output_dir,
        Some(case_id),
    )
}

fn run_route_execution_development_gate_filtered(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
    case_filter: Option<&str>,
) -> Result<RouteExecutionDevelopmentGateSummary> {
    let manifest = load_source_transition_development_manifest(manifest_path)?;
    let baseline_pack_path = resolve_source_transition_path(repo_root, &manifest.baseline_pack);
    let baseline_pack = load_pack(&baseline_pack_path)?;
    let baseline_pack_digest = format!(
        "fnv1a64:{}",
        source_transition_canonical_digest(&baseline_pack)
    );
    if baseline_pack_digest != manifest.baseline_pack_digest {
        bail!(
            "D0b baseline pack digest drift: expected {}, resolved {}",
            manifest.baseline_pack_digest,
            baseline_pack_digest
        );
    }
    let baseline_base_dir = baseline_pack_path
        .parent()
        .ok_or_else(|| anyhow!("baseline pack path has no parent"))?;
    let baseline_runs = resolve_pack_runs(&baseline_pack, baseline_base_dir)?;
    validate_source_transition_resolved_ids(
        "baseline",
        manifest
            .baseline_cases
            .iter()
            .flat_map(SourceTransitionDevelopmentCase::resolved_case_keys)
            .collect(),
        baseline_runs
            .iter()
            .map(|run| run.descriptor.run_id.clone())
            .collect(),
        manifest.baseline_expected_case_count,
    )?;
    let baseline_resolved_input_digest =
        source_transition_resolved_input_corpus_digest(&baseline_runs)?;
    if baseline_resolved_input_digest != manifest.baseline_resolved_input_digest {
        bail!(
            "D0b baseline resolved-input digest drift: expected {}, resolved {}",
            manifest.baseline_resolved_input_digest,
            baseline_resolved_input_digest
        );
    }

    let diagnostic_pack_path =
        resolve_source_transition_path(repo_root, &manifest.diagnostic_source_pack);
    let diagnostic_pack = load_source_transition_diagnostic_input_pack(&diagnostic_pack_path)?;
    if diagnostic_pack.source_commit != manifest.diagnostic_source_commit {
        bail!(
            "D0b diagnostic source commit mismatch: manifest {}, pack {}",
            manifest.diagnostic_source_commit,
            diagnostic_pack.source_commit
        );
    }
    let diagnostic_pack_digest = format!(
        "fnv1a64:{}",
        source_transition_canonical_digest(&diagnostic_pack)
    );
    if diagnostic_pack_digest != manifest.diagnostic_pack_digest {
        bail!(
            "D0b diagnostic input-pack digest drift: expected {}, resolved {}",
            manifest.diagnostic_pack_digest,
            diagnostic_pack_digest
        );
    }
    validate_source_transition_resolved_ids(
        "diagnostic",
        manifest
            .diagnostic_cases
            .iter()
            .flat_map(SourceTransitionDevelopmentCase::resolved_case_keys)
            .collect(),
        diagnostic_pack
            .cases
            .iter()
            .map(|case| case.run_id.clone())
            .collect(),
        manifest.diagnostic_expected_case_count,
    )?;
    let diagnostic_resolved_input_digest =
        source_transition_archived_input_corpus_digest(&diagnostic_pack.cases);
    if diagnostic_resolved_input_digest != manifest.diagnostic_resolved_input_digest {
        bail!(
            "D0b diagnostic resolved-input digest drift: expected {}, resolved {}",
            manifest.diagnostic_resolved_input_digest,
            diagnostic_resolved_input_digest
        );
    }

    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create D0b development output directory {}",
            output_dir.display()
        )
    })?;
    let baseline_selected = baseline_runs
        .iter()
        .filter(|run| case_filter.is_none_or(|case_id| run.descriptor.run_id == case_id))
        .collect::<Vec<_>>();
    let diagnostic_selected = diagnostic_pack
        .cases
        .iter()
        .filter(|case| case_filter.is_none_or(|case_id| case.run_id == case_id))
        .collect::<Vec<_>>();
    if case_filter.is_some() && baseline_selected.len() + diagnostic_selected.len() != 1 {
        bail!(
            "D0b case filter did not select exactly one resolved case: {}",
            case_filter.unwrap_or_default()
        );
    }
    let mut summary = RouteExecutionDevelopmentGateSummary {
        schema_version: ROUTE_EXECUTION_GATE_SCHEMA_VERSION,
        manifest_id: manifest.manifest_id,
        input_digest: manifest.input_digest,
        case_filter: case_filter.map(ToOwned::to_owned),
        baseline_total_count: baseline_selected.len(),
        diagnostic_total_count: diagnostic_selected.len(),
        baseline_contract_passes: 0,
        baseline_contract_failures: Vec::new(),
        parity_passes: 0,
        parity_failures: Vec::new(),
        source_status_counts: BTreeMap::new(),
        route_status_counts: BTreeMap::new(),
        source_invalidations: Vec::new(),
        route_invalidations: Vec::new(),
        deterministic_replay_passes: 0,
        deterministic_replay_failures: Vec::new(),
        overall_passed: false,
        failure_reasons: Vec::new(),
        summary_digest: String::new(),
    };
    for run in baseline_selected {
        let input = SourceTransitionGateRunInput::from_baseline(run)?;
        record_route_execution_gate_case(&mut summary, &input, "baseline", output_dir, true);
    }
    for case in diagnostic_selected {
        let input = SourceTransitionGateRunInput::from_archived(case);
        record_route_execution_gate_case(&mut summary, &input, "diagnostic", output_dir, false);
    }
    if !summary.baseline_contract_failures.is_empty() {
        summary
            .failure_reasons
            .push("baseline_contract_failed".to_owned());
    }
    if !summary.parity_failures.is_empty() {
        summary
            .failure_reasons
            .push("cadence_parity_failed".to_owned());
    }
    if !summary.source_invalidations.is_empty() {
        summary
            .failure_reasons
            .push("source_transition_invalidated".to_owned());
    }
    if !summary.route_invalidations.is_empty() {
        summary
            .failure_reasons
            .push("route_execution_invalidated".to_owned());
    }
    if !summary.deterministic_replay_failures.is_empty() {
        summary
            .failure_reasons
            .push("deterministic_replay_failed".to_owned());
    }
    summary.overall_passed = summary.failure_reasons.is_empty();
    summary.seal().map_err(anyhow::Error::msg)?;
    fs::write(
        output_dir.join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;
    Ok(summary)
}

struct RouteExecutionGateCaseResult {
    parity: SourceTransitionCadenceParity,
    source: SourceTransitionEvidence,
    route: RouteExecutionEvidence,
    replay_stable: bool,
    baseline_contract_pass: bool,
}

fn record_route_execution_gate_case(
    summary: &mut RouteExecutionDevelopmentGateSummary,
    input: &SourceTransitionGateRunInput<'_>,
    corpus: &str,
    output_dir: &Path,
    is_baseline: bool,
) {
    let result = run_route_execution_gate_case(input, corpus, output_dir);
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            let failure = SourceTransitionGateFailure {
                run_id: input.run_id.to_owned(),
                reason: format!("capture_failed:{error}"),
            };
            summary.source_invalidations.push(failure.clone());
            summary.route_invalidations.push(failure);
            *summary
                .source_status_counts
                .entry("invalid".to_owned())
                .or_default() += 1;
            *summary
                .route_status_counts
                .entry("invalid".to_owned())
                .or_default() += 1;
            return;
        }
    };
    if is_baseline {
        if result.baseline_contract_pass {
            summary.baseline_contract_passes += 1;
        } else {
            summary
                .baseline_contract_failures
                .push(SourceTransitionGateFailure {
                    run_id: input.run_id.to_owned(),
                    reason: "mission_success_checkpoint_satisfied_while_flying_not_observed"
                        .to_owned(),
                });
        }
    }
    if result.parity.passed {
        summary.parity_passes += 1;
    } else {
        summary.parity_failures.push(SourceTransitionGateFailure {
            run_id: input.run_id.to_owned(),
            reason: result.parity.mismatch_reasons.join(","),
        });
    }
    let source_status = serde_json::to_string(&result.source.status)
        .unwrap_or_else(|_| "\"invalid\"".to_owned())
        .trim_matches('"')
        .to_owned();
    *summary
        .source_status_counts
        .entry(source_status)
        .or_default() += 1;
    if let Some(reason) = &result.source.invalid_reason {
        summary
            .source_invalidations
            .push(SourceTransitionGateFailure {
                run_id: input.run_id.to_owned(),
                reason: reason.to_string(),
            });
    }
    let route_status = result.route.status.code();
    *summary.route_status_counts.entry(route_status).or_default() += 1;
    if let Some(reason) = &result.route.invalid_reason {
        summary
            .route_invalidations
            .push(SourceTransitionGateFailure {
                run_id: input.run_id.to_owned(),
                reason: reason.to_string(),
            });
    }
    if result.replay_stable {
        summary.deterministic_replay_passes += 1;
    } else {
        summary
            .deterministic_replay_failures
            .push(SourceTransitionGateFailure {
                run_id: input.run_id.to_owned(),
                reason: "source_or_route_evidence_changed_on_repeat".to_owned(),
            });
    }
}

fn run_route_execution_gate_case(
    input: &SourceTransitionGateRunInput<'_>,
    corpus: &str,
    output_dir: &Path,
) -> Result<RouteExecutionGateCaseResult> {
    let context = RunContext::from_scenario(input.scenario)
        .map_err(|error| anyhow!("failed to build run context: {error}"))?;
    let (ordinary, physics) = run_source_transition_cadence_pair(&context, input.controller_spec)
        .map_err(|error| anyhow!("controller capture failed: {error}"))?;
    let physics_scenario = with_physics_rate_evidence_overlay(input.scenario);
    let parity = compare_source_transition_cadence_parity(
        input.scenario,
        &ordinary,
        &physics_scenario,
        &physics,
    );
    let bundle_dir = output_dir.join(corpus).join(input.run_id);
    write_source_transition_raw_bundle(&bundle_dir, &physics_scenario, input.route_plan, &physics)?;
    let mut provenance = source_transition_provenance_for_route_plan(
        &physics_scenario,
        input.route_plan,
        &physics.run,
        &physics.controller_updates,
        input.run_id.to_owned(),
    )?;
    provenance.resolved_input_digest = source_transition_resolved_input_digest(
        input.run_id,
        input.scenario,
        input.route_plan,
        input.controller_spec,
    );
    let source = assemble_source_transition_evidence_from_controlled_artifacts(
        &physics_scenario,
        input.route_plan,
        &physics,
        provenance.clone(),
    );
    validate_source_evidence_round_trip(&source)?;
    write_source_transition_artifacts(&bundle_dir.join("source_transition.json"), &source)?;
    let route = assemble_route_execution_evidence_from_controlled_artifacts(
        &physics_scenario,
        input.route_plan,
        &physics,
        provenance.clone(),
    );
    let route_round_trip: RouteExecutionEvidence =
        serde_json::from_slice(&serde_json::to_vec(&route)?)?;
    if route != route_round_trip {
        bail!(
            "route evidence changes on JSON round-trip at {}",
            first_json_difference(
                &serde_json::to_value(&route)?,
                &serde_json::to_value(&route_round_trip)?,
                "$",
            )
            .unwrap_or_else(|| "unknown path".to_owned())
        );
    }
    if route.evidence_digest != route_execution_evidence_digest(&route_round_trip)
        || route.physical_digest != route_execution_physical_digest(&route_round_trip)
        || route.source_transition.evidence_digest
            != source_transition_evidence_digest(&route_round_trip.source_transition)
        || route.source_transition.physical_digest
            != source_transition_physical_digest(&route_round_trip.source_transition)
                .map_err(|error| anyhow!(error))?
        || route.provenance.source_evidence_digest
            != route_round_trip.source_transition.evidence_digest
    {
        bail!("route evidence digests are not stable across JSON round-trip");
    }
    write_route_execution_artifacts(&bundle_dir.join("route_execution.json"), &route)?;
    let source_repeat = assemble_source_transition_evidence_from_controlled_artifacts(
        &physics_scenario,
        input.route_plan,
        &physics,
        provenance.clone(),
    );
    let route_repeat = assemble_route_execution_evidence_from_controlled_artifacts(
        &physics_scenario,
        input.route_plan,
        &physics,
        provenance,
    );
    let replay_stable = serde_json::to_vec(&source)? == serde_json::to_vec(&source_repeat)?
        && source.evidence_digest == source_repeat.evidence_digest
        && source.physical_digest == source_repeat.physical_digest
        && serde_json::to_vec(&route)? == serde_json::to_vec(&route_repeat)?
        && route.evidence_digest == route_repeat.evidence_digest
        && route.physical_digest == route_repeat.physical_digest;
    let baseline_contract_pass = matches!(
        (
            ordinary.run.manifest.mission_outcome,
            ordinary.run.manifest.end_reason,
            ordinary.run.manifest.physical_outcome,
        ),
        (
            MissionOutcome::Success,
            EndReason::CheckpointSatisfied,
            PhysicalOutcome::Flying,
        )
    );
    Ok(RouteExecutionGateCaseResult {
        parity,
        source,
        route,
        replay_stable,
        baseline_contract_pass,
    })
}

fn validate_source_evidence_round_trip(evidence: &SourceTransitionEvidence) -> Result<()> {
    let round_trip: SourceTransitionEvidence =
        serde_json::from_slice(&serde_json::to_vec(evidence)?)?;
    if *evidence != round_trip {
        bail!(
            "source evidence changes on JSON round-trip at {}",
            first_json_difference(
                &serde_json::to_value(evidence)?,
                &serde_json::to_value(&round_trip)?,
                "$",
            )
            .unwrap_or_else(|| "unknown path".to_owned())
        );
    }
    if evidence.evidence_digest != source_transition_evidence_digest(&round_trip)
        || evidence.physical_digest
            != source_transition_physical_digest(&round_trip).map_err(anyhow::Error::msg)?
    {
        bail!("source evidence digests are not stable across JSON round-trip");
    }
    Ok(())
}

fn first_json_difference(
    left: &serde_json::Value,
    right: &serde_json::Value,
    path: &str,
) -> Option<String> {
    match (left, right) {
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
            for key in left.keys().chain(right.keys()).collect::<BTreeSet<_>>() {
                let child_path = format!("{path}.{key}");
                match (left.get(key), right.get(key)) {
                    (Some(left), Some(right)) => {
                        if let Some(difference) = first_json_difference(left, right, &child_path) {
                            return Some(difference);
                        }
                    }
                    _ => return Some(child_path),
                }
            }
            None
        }
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
            if left.len() != right.len() {
                return Some(format!("{path}.length"));
            }
            left.iter()
                .zip(right)
                .enumerate()
                .find_map(|(index, (left, right))| {
                    first_json_difference(left, right, &format!("{path}[{index}]"))
                })
        }
        _ => (left != right).then(|| format!("{path}: {left} != {right}")),
    }
}

pub fn run_pack_file(path: &Path, output_dir: Option<&Path>) -> Result<BatchReport> {
    run_pack_file_with_workers(path, output_dir, 1)
}

pub fn load_batch_report(path: &Path) -> Result<BatchReport> {
    let summary_path = if path.is_dir() {
        path.join("summary.json")
    } else {
        path.to_path_buf()
    };
    let raw = fs::read_to_string(&summary_path)
        .with_context(|| format!("failed to read batch report {}", summary_path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse batch report {}", summary_path.display()))
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ReportRefreshSummary {
    pub requested_packs: usize,
    pub refreshed_batches: usize,
    pub refreshed_runs: usize,
    pub skipped_uncaptured_packs: usize,
}

pub fn refresh_report_outputs(all: bool) -> Result<ReportRefreshSummary> {
    let root = repo_root();
    let pack_ids = report_catalog::refresh_pack_ids(&root, all)?;
    let mut summary = ReportRefreshSummary {
        requested_packs: pack_ids.len(),
        ..ReportRefreshSummary::default()
    };

    for pack_id in pack_ids {
        let output_dir = root.join("outputs/eval").join(&pack_id);
        if !output_dir.join("summary.json").is_file() {
            summary.skipped_uncaptured_packs += 1;
            continue;
        }
        let report = load_batch_report(&output_dir)?;
        let baseline = refresh_baseline(&root, &report);
        let mut render_report = report.clone();
        if report.provenance.compare.status == BatchCompareResolutionStatus::Resolved
            && baseline.is_none()
        {
            render_report.provenance.compare.status = BatchCompareResolutionStatus::Missing;
            render_report.provenance.compare.note = Some(
                "recorded comparison is no longer readable; refreshed as standalone evidence"
                    .to_owned(),
            );
        }
        report::write_batch_report_artifacts(
            &output_dir,
            &render_report,
            baseline
                .as_ref()
                .map(|(baseline_dir, baseline_report)| (baseline_dir.as_path(), baseline_report)),
        )?;
        summary.refreshed_batches += 1;

        summary.refreshed_runs += report
            .records
            .par_iter()
            .map(|record| -> Result<usize> {
                let Some(bundle_dir) = record.bundle_dir.as_deref() else {
                    return Ok(0);
                };
                let bundle_dir = resolve_refresh_path(&root, bundle_dir);
                refresh_run_report(&bundle_dir)?;
                Ok(1)
            })
            .try_reduce(|| 0, |lhs, rhs| Ok(lhs + rhs))?;
    }

    report_catalog::write_report_catalog(&root)?;
    Ok(summary)
}

fn refresh_baseline(repo_root: &Path, report: &BatchReport) -> Option<(PathBuf, BatchReport)> {
    if report.provenance.compare.status != BatchCompareResolutionStatus::Resolved {
        return None;
    }
    let baseline_dir = report.provenance.compare.baseline_dir.as_deref()?;
    let baseline_dir = resolve_refresh_path(repo_root, baseline_dir);
    load_batch_report(&baseline_dir)
        .ok()
        .map(|baseline| (baseline_dir, baseline))
}

fn resolve_refresh_path(repo_root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    }
}

fn refresh_run_report(bundle_dir: &Path) -> Result<()> {
    let scenario = read_json::<ScenarioSpec>(&bundle_dir.join("scenario.json"))?;
    let controller = read_json::<ControllerSpec>(&bundle_dir.join("controller.json"))?;
    let manifest = read_json::<RunManifest>(&bundle_dir.join("manifest.json"))?;
    let events = read_json::<Vec<EventRecord>>(&bundle_dir.join("events.json"))?;
    let samples = read_json::<Vec<SampleRecord>>(&bundle_dir.join("samples.json"))?;
    let controller_updates =
        read_json::<Vec<ControllerUpdateRecord>>(&bundle_dir.join("controller_updates.json"))?;
    let performance = read_json::<RunPerformanceStats>(&bundle_dir.join("performance.json"))?;
    let route_plan = bundle_dir
        .join("route_plan.json")
        .is_file()
        .then(|| read_json::<pd_core::RoutePlan>(&bundle_dir.join("route_plan.json")))
        .transpose()?;
    let planner_compute = bundle_dir
        .join("planner_compute.json")
        .is_file()
        .then(|| {
            read_json::<pd_core::PlannerComputeEvidence>(&bundle_dir.join("planner_compute.json"))
        })
        .transpose()?;
    pd_report::write_run_report_with_plan_context_and_compute(
        &bundle_dir.join("report.html"),
        &scenario,
        Some(&controller),
        &manifest,
        &events,
        &samples,
        &controller_updates,
        Some(&performance),
        Some(&pd_report::RunReportContext {
            parent_report_href: Some("../../report.html".to_owned()),
            parent_report_label: Some("Batch report".to_owned()),
            run_index_href: Some("../".to_owned()),
        }),
        route_plan.as_ref(),
        planner_compute.as_ref(),
    )
}

pub fn run_pack_file_cached(
    path: &Path,
    output_dir: Option<&Path>,
    workers: usize,
    compare_ref: Option<&str>,
    baseline_dir: Option<&Path>,
    missing_compare: MissingComparePolicy,
    reuse_cache: bool,
) -> Result<CachedBatchRunOutcome> {
    let pack = load_pack(path)?;
    let base_dir = path
        .parent()
        .ok_or_else(|| anyhow!("pack path has no parent directory"))?;
    run_pack_cached_with_options(
        &pack,
        base_dir,
        CachedBatchRunOptions {
            output_dir,
            workers,
            compare_ref,
            baseline_dir,
            missing_compare,
            reuse_cache,
        },
    )
}

pub fn resolve_pack_compare_baseline(
    path: &Path,
    compare_ref: Option<&str>,
    baseline_dir: Option<&Path>,
    missing_compare: MissingComparePolicy,
) -> Result<Option<ResolvedBaselineReport>> {
    let pack = load_pack(path)?;
    let base_dir = path
        .parent()
        .ok_or_else(|| anyhow!("pack path has no parent directory"))?;
    validate_pack(&pack)?;
    let resolved_runs = resolve_pack_runs(&pack, base_dir)?;
    let identity = batch_identity_for_pack(&pack, &resolved_runs)?;
    let workspace = current_workspace_state()?;
    let requested_compare =
        resolve_compare_provenance(baseline_dir, compare_ref, missing_compare, &workspace)?;
    let (_, baseline) =
        load_requested_baseline(&pack, &identity, requested_compare, missing_compare)?;
    Ok(baseline)
}

pub fn promote_pack_cache(
    path: &Path,
    source_workspace_key: Option<&str>,
    target_ref: &str,
) -> Result<PathBuf> {
    let pack = load_pack(path)?;
    let base_dir = path
        .parent()
        .ok_or_else(|| anyhow!("pack path has no parent directory"))?;
    validate_pack(&pack)?;
    let resolved_runs = resolve_pack_runs(&pack, base_dir)?;
    let identity = batch_identity_for_pack(&pack, &resolved_runs)?;
    let batch_stem = batch_cache_stem(&pack.id, &identity);
    let workspace = current_workspace_state()?;
    let target_commit_key = git_commit_key_for_ref(target_ref)?;
    let source_key = if let Some(source_workspace_key) = source_workspace_key {
        source_workspace_key.to_owned()
    } else if workspace.dirty
        && cache_dir_for_batch_key(&workspace.workspace_key, &batch_stem).exists()
    {
        workspace.workspace_key.clone()
    } else {
        find_latest_dirty_workspace_key(&target_commit_key, &batch_stem)?.ok_or_else(|| {
            anyhow!(
                "no dirty cache found for commit '{}' and batch '{}'",
                target_commit_key,
                batch_stem
            )
        })?
    };
    let source_dir = cache_dir_for_batch_key(&source_key, &batch_stem);
    let target_dir = cache_dir_for_batch_key(&target_commit_key, &batch_stem);
    if source_dir == target_dir {
        bail!(
            "source cache {} and target cache {} are identical",
            source_dir.display(),
            target_dir.display()
        );
    }

    let mut report = validate_cached_batch_dir(&source_dir, &pack, &identity)?
        .ok_or_else(|| anyhow!("no reusable cache found at {}", source_dir.display()))?;
    let source_cache = report.provenance.cache.clone().ok_or_else(|| {
        anyhow!(
            "cached batch at {} is missing cache provenance",
            source_dir.display()
        )
    })?;

    if target_dir.exists() {
        fs::remove_dir_all(&target_dir).with_context(|| {
            format!(
                "failed to remove existing promoted cache {}",
                target_dir.display()
            )
        })?;
    }
    copy_dir_recursive(&source_dir, &target_dir)?;
    rewrite_report_bundle_dirs(&mut report, &source_dir, &target_dir);
    report.provenance.compare = BatchCompareProvenance::default();
    report.provenance.cache = Some(BatchCacheInfo {
        workspace_key: target_commit_key.clone(),
        commit_key: target_commit_key.clone(),
        batch_stem,
        cache_dir: target_dir.to_string_lossy().into_owned(),
        status: BatchCacheStatus::Promoted,
        created_at_unix_s: current_unix_timestamp(),
        promotion: Some(BatchCachePromotion {
            source_workspace_key: source_key,
            source_cache_dir: source_cache.cache_dir,
            promoted_at_unix_s: current_unix_timestamp(),
        }),
    });
    write_batch_cache_dir(
        &target_dir,
        &pack,
        &report,
        false,
        &report::BatchReportRenderCache::default(),
    )?;
    Ok(target_dir)
}

#[allow(clippy::too_many_arguments)]
pub fn run_pack_cached(
    pack: &ScenarioPackSpec,
    base_dir: &Path,
    output_dir: Option<&Path>,
    workers: usize,
    compare_ref: Option<&str>,
    baseline_dir: Option<&Path>,
    missing_compare: MissingComparePolicy,
    reuse_cache: bool,
) -> Result<CachedBatchRunOutcome> {
    run_pack_cached_with_options(
        pack,
        base_dir,
        CachedBatchRunOptions {
            output_dir,
            workers,
            compare_ref,
            baseline_dir,
            missing_compare,
            reuse_cache,
        },
    )
}

pub fn run_pack_cached_with_options(
    pack: &ScenarioPackSpec,
    base_dir: &Path,
    options: CachedBatchRunOptions<'_>,
) -> Result<CachedBatchRunOutcome> {
    let CachedBatchRunOptions {
        output_dir,
        workers,
        compare_ref,
        baseline_dir,
        missing_compare,
        reuse_cache,
    } = options;
    validate_pack(pack)?;

    let resolved_runs = resolve_pack_runs(pack, base_dir)?;
    let requested_workers = workers.max(1);
    let workers_used = effective_worker_count(requested_workers, resolved_runs.len());
    let identity = batch_identity_for_pack(pack, &resolved_runs)?;
    let workspace = current_workspace_state()?;
    let batch_stem = batch_cache_stem(&pack.id, &identity);
    let cache_dir = cache_dir_for_batch_key(&workspace.workspace_key, &batch_stem);
    let render_cache = report::BatchReportRenderCache::default();

    let base_cache_report = if reuse_cache {
        validate_cached_batch_dir(&cache_dir, pack, &identity)?
    } else {
        None
    };

    let cache_report = if let Some(mut report) = base_cache_report {
        if let Some(cache) = report.provenance.cache.as_mut() {
            cache.status = BatchCacheStatus::Reused;
        }
        report
    } else {
        let started = Instant::now();
        let records = execute_resolved_runs(&resolved_runs, Some(&cache_dir), workers_used)?;
        let report = BatchReport {
            schema_version: BATCH_REPORT_SCHEMA_VERSION,
            pack_id: pack.id.clone(),
            pack_name: pack.name.clone(),
            total_runs: records.len(),
            wall_clock_s: started.elapsed().as_secs_f64(),
            workers_requested: requested_workers,
            workers_used,
            identity: identity.clone(),
            provenance: BatchProvenance {
                cache: Some(BatchCacheInfo {
                    workspace_key: workspace.workspace_key.clone(),
                    commit_key: workspace.commit_key.clone(),
                    batch_stem: batch_stem.clone(),
                    cache_dir: cache_dir.to_string_lossy().into_owned(),
                    status: BatchCacheStatus::Fresh,
                    created_at_unix_s: current_unix_timestamp(),
                    promotion: None,
                }),
                compare: BatchCompareProvenance::default(),
            },
            resolved_runs: resolved_runs
                .iter()
                .map(|run| run.descriptor.clone())
                .collect(),
            summary: summarize_records(&records),
            records,
        };
        write_batch_cache_dir(&cache_dir, pack, &report, true, &render_cache)?;
        report
    };

    let requested_compare =
        resolve_compare_provenance(baseline_dir, compare_ref, missing_compare, &workspace)?;
    let (compare_provenance, baseline) =
        load_requested_baseline(pack, &identity, requested_compare, missing_compare)?;

    let mut output_report = cache_report.clone();
    output_report.provenance.compare = compare_provenance;
    if let Some(cache) = output_report.provenance.cache.as_mut() {
        cache.status = match cache.status {
            BatchCacheStatus::Promoted => BatchCacheStatus::Reused,
            other => other,
        };
    }
    let final_report = if let Some(output_dir) = output_dir {
        let localized_report = localize_report_bundle_dirs(&output_report, output_dir);
        write_batch_output_dir(
            output_dir,
            pack,
            &output_report,
            baseline
                .as_ref()
                .map(|baseline| (baseline.dir.as_path(), &baseline.report)),
            &render_cache,
        )?;
        localized_report
    } else {
        output_report
    };

    Ok(CachedBatchRunOutcome {
        report: final_report,
        baseline,
        cache_dir,
    })
}

pub fn run_pack_file_with_workers(
    path: &Path,
    output_dir: Option<&Path>,
    workers: usize,
) -> Result<BatchReport> {
    let pack = load_pack(path)?;
    let base_dir = path
        .parent()
        .ok_or_else(|| anyhow!("pack path has no parent directory"))?;
    run_pack_with_workers(&pack, base_dir, output_dir, workers)
}

pub fn run_pack(
    pack: &ScenarioPackSpec,
    base_dir: &Path,
    output_dir: Option<&Path>,
) -> Result<BatchReport> {
    run_pack_with_workers(pack, base_dir, output_dir, 1)
}

pub fn run_pack_with_workers(
    pack: &ScenarioPackSpec,
    base_dir: &Path,
    output_dir: Option<&Path>,
    workers: usize,
) -> Result<BatchReport> {
    validate_pack(pack)?;

    let resolved_runs = resolve_pack_runs(pack, base_dir)?;
    let requested_workers = workers.max(1);
    let workers_used = effective_worker_count(requested_workers, resolved_runs.len());
    let identity = batch_identity_for_pack(pack, &resolved_runs)?;

    let started = Instant::now();
    let records = execute_resolved_runs(&resolved_runs, output_dir, workers_used)?;
    let report = BatchReport {
        schema_version: BATCH_REPORT_SCHEMA_VERSION,
        pack_id: pack.id.clone(),
        pack_name: pack.name.clone(),
        total_runs: records.len(),
        wall_clock_s: started.elapsed().as_secs_f64(),
        workers_requested: requested_workers,
        workers_used,
        identity,
        provenance: BatchProvenance::default(),
        resolved_runs: resolved_runs
            .iter()
            .map(|run| run.descriptor.clone())
            .collect(),
        summary: summarize_records(&records),
        records,
    };

    if let Some(output_dir) = output_dir {
        fs::create_dir_all(output_dir).with_context(|| {
            format!(
                "failed to create batch eval output directory {}",
                output_dir.display()
            )
        })?;
        write_json(&output_dir.join("pack.json"), pack)?;
        write_json(
            &output_dir.join("resolved_runs.json"),
            &report.resolved_runs,
        )?;
        write_json(&output_dir.join("summary.json"), &report)?;
        maybe_update_latest_link(output_dir)?;
        if let Some(last_record) = report.records.last()
            && let Some(bundle_dir) = last_record.bundle_dir.as_deref()
        {
            maybe_update_latest_link(Path::new(bundle_dir))?;
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests;
