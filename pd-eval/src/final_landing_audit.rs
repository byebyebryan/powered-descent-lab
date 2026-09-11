//! CB0 ordinary final-landing audit over the exact D0 development corpus.
//!
//! This is deliberately an evaluator-owned overlay.  It resolves the
//! existing input-only development rows, changes only the mission evaluation
//! goal, and runs the existing controller/simulator path.  No planner result,
//! controller specification, or physical execution input is regenerated or
//! modified.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result, anyhow, bail};
use pd_control::{
    ControlledRunArtifacts, ControllerSpec, ControllerUpdateRecord, run_controller_spec,
};
use pd_core::{
    EndReason, EvaluationGoal, MissionOutcome, PhysicalOutcome, RoutePlan, RunArtifacts,
    RunContext, RunManifest, ScenarioSpec,
};
use serde::{Deserialize, Serialize};

use crate::{
    SourceTransitionDevelopmentInputs, SourceTransitionDiagnosticInputCase, canonical_digest,
    read_json, resolve_source_transition_development_inputs, source_transition_raw_bundle_digest,
    source_transition_resolved_input_digest, write_artifact_bundle, write_json,
};

pub const FINAL_LANDING_AUDIT_SCHEMA_ID: &str = "final_landing_audit_v1";
pub const FINAL_LANDING_AUDIT_SCHEMA_VERSION: u32 = 1;
pub const FINAL_LANDING_AUDIT_FUEL_EPSILON_KG: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalLandingAuditCorpus {
    Baseline,
    Diagnostic,
}

impl FinalLandingAuditCorpus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Diagnostic => "diagnostic",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalLandingTerminalClass {
    TargetTouchdown,
    OffTargetTouchdown,
    Crash,
    MaxTime,
}

impl FinalLandingTerminalClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TargetTouchdown => "target_touchdown",
            Self::OffTargetTouchdown => "off_target_touchdown",
            Self::Crash => "crash",
            Self::MaxTime => "max_time",
        }
    }
}

/// A compact, deterministic index for one persisted CB0 case bundle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FinalLandingAuditCaseIndex {
    pub corpus: FinalLandingAuditCorpus,
    pub row_id: String,
    pub terminal_class: FinalLandingTerminalClass,
    pub fuel_depleted: bool,
    pub case_digest: String,
}

/// The result identity and authoritative terminal result for one audit row.
///
/// Controller performance timing and the full raw bundle are intentionally not
/// included in the digest-bearing result; its stable raw-bundle digest binds
/// the behavior-bearing records.  The ordinary run bundle is still persisted
/// beside this JSON for inspection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FinalLandingAuditCase {
    pub schema_id: String,
    pub schema_version: u32,
    pub corpus: FinalLandingAuditCorpus,
    pub row_id: String,
    pub source_d0_input_digest: String,
    pub base_resolved_input_digest: String,
    pub audit_resolved_input_digest: String,
    pub execution_identity_digest: String,
    pub audit_scenario_digest: String,
    pub route_plan_digest: String,
    pub controller_id: String,
    pub controller_digest: String,
    pub original_goal: EvaluationGoal,
    pub audit_goal: EvaluationGoal,
    pub terminal_class: FinalLandingTerminalClass,
    pub fuel_remaining_kg: f64,
    pub fuel_used_kg: f64,
    pub fuel_depleted: bool,
    pub manifest: RunManifest,
    pub manifest_digest: String,
    pub raw_bundle_digest: String,
    pub case_digest: String,
}

impl FinalLandingAuditCase {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_id != FINAL_LANDING_AUDIT_SCHEMA_ID
            || self.schema_version != FINAL_LANDING_AUDIT_SCHEMA_VERSION
        {
            return Err("final-landing audit case schema mismatch".to_owned());
        }
        for (name, value) in [
            ("row_id", self.row_id.as_str()),
            (
                "source_d0_input_digest",
                self.source_d0_input_digest.as_str(),
            ),
            (
                "base_resolved_input_digest",
                self.base_resolved_input_digest.as_str(),
            ),
            (
                "audit_resolved_input_digest",
                self.audit_resolved_input_digest.as_str(),
            ),
            (
                "execution_identity_digest",
                self.execution_identity_digest.as_str(),
            ),
            ("audit_scenario_digest", self.audit_scenario_digest.as_str()),
            ("route_plan_digest", self.route_plan_digest.as_str()),
            ("controller_id", self.controller_id.as_str()),
            ("controller_digest", self.controller_digest.as_str()),
            ("manifest_digest", self.manifest_digest.as_str()),
            ("raw_bundle_digest", self.raw_bundle_digest.as_str()),
            ("case_digest", self.case_digest.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("final-landing audit case {name} must not be empty"));
            }
        }
        if !matches!(self.audit_goal, EvaluationGoal::LandingOnPad { .. }) {
            return Err("final-landing audit goal must be landing_on_pad".to_owned());
        }
        if self.original_goal.target_pad_id() != self.audit_goal.target_pad_id() {
            return Err("final-landing audit changed the target pad".to_owned());
        }
        if self.manifest.scenario_id != self.row_id {
            return Err("final-landing manifest scenario ID does not match row ID".to_owned());
        }
        if self.manifest.controller_id != self.controller_id {
            return Err("final-landing manifest controller ID does not match case".to_owned());
        }
        if !self.fuel_remaining_kg.is_finite()
            || self.fuel_remaining_kg < 0.0
            || !self.fuel_used_kg.is_finite()
            || self.fuel_used_kg < 0.0
        {
            return Err(
                "final-landing fuel diagnostics must be finite and non-negative".to_owned(),
            );
        }
        if self.fuel_depleted != fuel_is_depleted(self.fuel_remaining_kg) {
            return Err("final-landing fuel depletion flag is not canonical".to_owned());
        }
        if self.manifest.summary.fuel_remaining_kg != self.fuel_remaining_kg
            || self.manifest.summary.fuel_used_kg != self.fuel_used_kg
        {
            return Err("final-landing fuel diagnostics do not match manifest".to_owned());
        }
        if classify_final_landing_terminal(&self.manifest)? != self.terminal_class {
            return Err("final-landing terminal class does not match manifest".to_owned());
        }
        if self.manifest_digest != canonical_digest(&self.manifest)? {
            return Err("final-landing manifest digest mismatch".to_owned());
        }
        let mut material = self.clone();
        material.case_digest.clear();
        if self.case_digest != canonical_digest(&material)? {
            return Err("final-landing case digest mismatch".to_owned());
        }
        Ok(())
    }

    fn seal(&mut self) -> Result<(), String> {
        self.case_digest.clear();
        self.case_digest = canonical_digest(self)?;
        self.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FinalLandingAuditSummary {
    pub schema_id: String,
    pub schema_version: u32,
    pub manifest_id: String,
    pub source_d0_input_digest: String,
    pub baseline_resolved_input_digest: String,
    pub diagnostic_resolved_input_digest: String,
    pub baseline_case_count: usize,
    pub diagnostic_case_count: usize,
    pub terminal_counts: BTreeMap<String, usize>,
    pub fuel_depleted_count: usize,
    pub cases: Vec<FinalLandingAuditCaseIndex>,
    pub summary_digest: String,
}

impl FinalLandingAuditSummary {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_id != FINAL_LANDING_AUDIT_SCHEMA_ID
            || self.schema_version != FINAL_LANDING_AUDIT_SCHEMA_VERSION
        {
            return Err("final-landing audit summary schema mismatch".to_owned());
        }
        for (name, value) in [
            ("manifest_id", self.manifest_id.as_str()),
            (
                "source_d0_input_digest",
                self.source_d0_input_digest.as_str(),
            ),
            (
                "baseline_resolved_input_digest",
                self.baseline_resolved_input_digest.as_str(),
            ),
            (
                "diagnostic_resolved_input_digest",
                self.diagnostic_resolved_input_digest.as_str(),
            ),
            ("summary_digest", self.summary_digest.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!(
                    "final-landing audit summary {name} must not be empty"
                ));
            }
        }
        if self.baseline_case_count + self.diagnostic_case_count != self.cases.len() {
            return Err("final-landing audit summary counts do not match cases".to_owned());
        }
        let expected_total = self.cases.len();
        let mut seen_ids = BTreeSet::new();
        let mut observed_counts = BTreeMap::<String, usize>::new();
        let mut observed_fuel_depleted = 0;
        for (index, case) in self.cases.iter().enumerate() {
            if case.row_id.trim().is_empty()
                || case.case_digest.trim().is_empty()
                || !seen_ids.insert(case.row_id.clone())
            {
                return Err(format!(
                    "final-landing audit summary has duplicate or empty case ID at index {index}"
                ));
            }
            *observed_counts
                .entry(case.terminal_class.as_str().to_owned())
                .or_insert(0) += 1;
            if case.fuel_depleted {
                observed_fuel_depleted += 1;
            }
        }
        if expected_total == 0 {
            return Err("final-landing audit summary must contain cases".to_owned());
        }
        if observed_counts != self.terminal_counts {
            return Err("final-landing terminal counts do not match cases".to_owned());
        }
        if observed_fuel_depleted != self.fuel_depleted_count {
            return Err("final-landing fuel count does not match cases".to_owned());
        }
        let mut material = self.clone();
        material.summary_digest.clear();
        if self.summary_digest != canonical_digest(&material)? {
            return Err("final-landing summary digest mismatch".to_owned());
        }
        Ok(())
    }

    fn seal(&mut self) -> Result<(), String> {
        self.summary_digest.clear();
        self.summary_digest = canonical_digest(self)?;
        self.validate()
    }
}

#[derive(Clone)]
struct FinalLandingAuditInput {
    corpus: FinalLandingAuditCorpus,
    row_id: String,
    manifest_id: String,
    source_d0_input_digest: String,
    baseline_resolved_input_digest: String,
    diagnostic_resolved_input_digest: String,
    scenario: ScenarioSpec,
    route_plan: RoutePlan,
    controller: ControllerSpec,
    base_resolved_input_digest: String,
}

struct PersistedRawBundle {
    artifacts: RunArtifacts,
    controller_updates: Vec<ControllerUpdateRecord>,
}

impl PersistedRawBundle {
    fn digest(&self) -> String {
        source_transition_raw_bundle_digest(&self.artifacts, &self.controller_updates)
    }
}

fn load_persisted_raw_bundle(case_dir: &Path) -> Result<PersistedRawBundle> {
    Ok(PersistedRawBundle {
        artifacts: RunArtifacts {
            manifest: read_json(&case_dir.join("manifest.json"))?,
            actions: read_json(&case_dir.join("actions.json"))?,
            events: read_json(&case_dir.join("events.json"))?,
            samples: read_json(&case_dir.join("samples.json"))?,
        },
        controller_updates: read_json(&case_dir.join("controller_updates.json"))?,
    })
}

/// Run the CB0 ordinary final-landing audit over the complete D0 corpus.
pub fn run_final_landing_audit(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
) -> Result<FinalLandingAuditSummary> {
    require_empty_output_dir(output_dir)?;
    let inputs = collect_final_landing_inputs(manifest_path, repo_root)?;
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create final-landing audit output directory {}",
            output_dir.display()
        )
    })?;

    let mut cases = Vec::with_capacity(inputs.len());
    for input in &inputs {
        let case_dir = output_dir.join("case").join(&input.row_id);
        fs::create_dir_all(&case_dir).with_context(|| {
            format!(
                "failed to create final-landing audit case directory {}",
                case_dir.display()
            )
        })?;
        let case = run_final_landing_case(input, &case_dir)?;
        write_json(&case_dir.join("case.json"), &case)?;
        cases.push(case);
    }

    let mut summary = build_final_landing_summary(&inputs, &cases)?;
    summary.seal().map_err(anyhow::Error::msg)?;
    write_json(&output_dir.join("summary.json"), &summary)?;

    let reloaded = load_final_landing_audit(manifest_path, repo_root, output_dir)?;
    if reloaded != summary {
        bail!("final-landing audit changed during JSON reload");
    }
    Ok(reloaded)
}

/// Reload and fully validate a previously written CB0 audit root.
pub fn load_final_landing_audit(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
) -> Result<FinalLandingAuditSummary> {
    let inputs = collect_final_landing_inputs(manifest_path, repo_root)?;
    let summary: FinalLandingAuditSummary = read_json(&output_dir.join("summary.json"))?;
    summary.validate().map_err(anyhow::Error::msg)?;
    let first_input = inputs
        .first()
        .ok_or_else(|| anyhow!("CB0 requires at least one input"))?;
    if summary.manifest_id != first_input.manifest_id
        || summary.source_d0_input_digest != inputs_source_digest(&inputs)
        || summary.baseline_resolved_input_digest != first_input.baseline_resolved_input_digest
        || summary.diagnostic_resolved_input_digest != first_input.diagnostic_resolved_input_digest
    {
        bail!("final-landing audit summary is bound to different D0 inputs");
    }
    if summary.baseline_case_count != inputs_baseline_count(&inputs)
        || summary.diagnostic_case_count != inputs_diagnostic_count(&inputs)
    {
        bail!("final-landing audit summary corpus counts drifted");
    }

    let mut loaded_cases = Vec::with_capacity(inputs.len());
    for (index, input) in inputs.iter().enumerate() {
        let case_dir = output_dir.join("case").join(&input.row_id);
        let case: FinalLandingAuditCase = read_json(&case_dir.join("case.json"))?;
        case.validate().map_err(anyhow::Error::msg)?;
        validate_case_against_input(&case, input)?;
        validate_bundle_against_case(&case, input, &case_dir)?;
        let expected_index = summary
            .cases
            .get(index)
            .ok_or_else(|| anyhow!("final-landing audit summary is missing case index {index}"))?;
        if expected_index.row_id != case.row_id
            || expected_index.corpus != case.corpus
            || expected_index.terminal_class != case.terminal_class
            || expected_index.fuel_depleted != case.fuel_depleted
            || expected_index.case_digest != case.case_digest
        {
            bail!(
                "final-landing audit summary index mismatch for {}",
                case.row_id
            );
        }
        loaded_cases.push(case);
    }
    if loaded_cases.len() != summary.cases.len() {
        bail!("final-landing audit summary has an unexpected case count");
    }
    Ok(summary)
}

fn collect_final_landing_inputs(
    manifest_path: &Path,
    repo_root: &Path,
) -> Result<Vec<FinalLandingAuditInput>> {
    let inputs = resolve_source_transition_development_inputs(manifest_path, repo_root)?;
    if inputs.manifest.baseline_expected_case_count != 36
        || inputs.manifest.diagnostic_expected_case_count != 24
        || inputs.baseline_runs.len() != 36
        || inputs.diagnostic_cases.len() != 24
    {
        bail!(
            "CB0 requires exactly 36 baseline and 24 diagnostic inputs, got {} and {}",
            inputs.baseline_runs.len(),
            inputs.diagnostic_cases.len()
        );
    }

    let mut resolved = Vec::with_capacity(60);
    for run in &inputs.baseline_runs {
        let route_plan = run.descriptor.route_plan.clone().ok_or_else(|| {
            anyhow!(
                "baseline case '{}' has no route plan",
                run.descriptor.run_id
            )
        })?;
        let controller = run.descriptor.controller_spec.clone();
        let base_resolved_input_digest = source_transition_resolved_input_digest(
            &run.descriptor.run_id,
            &run.scenario,
            &route_plan,
            &controller,
        );
        resolved.push(FinalLandingAuditInput {
            corpus: FinalLandingAuditCorpus::Baseline,
            row_id: run.descriptor.run_id.clone(),
            manifest_id: inputs.manifest.manifest_id.clone(),
            source_d0_input_digest: inputs.manifest.input_digest.clone(),
            baseline_resolved_input_digest: inputs.manifest.baseline_resolved_input_digest.clone(),
            diagnostic_resolved_input_digest: inputs
                .manifest
                .diagnostic_resolved_input_digest
                .clone(),
            scenario: run.scenario.clone(),
            route_plan,
            controller,
            base_resolved_input_digest,
        });
    }
    for case in &inputs.diagnostic_cases {
        resolved.push(final_landing_input_from_diagnostic(&inputs, case));
    }
    Ok(resolved)
}

fn final_landing_input_from_diagnostic(
    inputs: &SourceTransitionDevelopmentInputs,
    case: &SourceTransitionDiagnosticInputCase,
) -> FinalLandingAuditInput {
    let base_resolved_input_digest = source_transition_resolved_input_digest(
        &case.run_id,
        &case.scenario,
        &case.route_plan,
        &case.controller,
    );
    FinalLandingAuditInput {
        corpus: FinalLandingAuditCorpus::Diagnostic,
        row_id: case.run_id.clone(),
        manifest_id: inputs.manifest.manifest_id.clone(),
        source_d0_input_digest: inputs.manifest.input_digest.clone(),
        baseline_resolved_input_digest: inputs.manifest.baseline_resolved_input_digest.clone(),
        diagnostic_resolved_input_digest: inputs.manifest.diagnostic_resolved_input_digest.clone(),
        scenario: case.scenario.clone(),
        route_plan: case.route_plan.clone(),
        controller: case.controller.clone(),
        base_resolved_input_digest,
    }
}

fn run_final_landing_case(
    input: &FinalLandingAuditInput,
    case_dir: &Path,
) -> Result<FinalLandingAuditCase> {
    let audit_scenario = landing_goal_overlay(&input.scenario, &input.route_plan)?;
    validate_case_input_identity(input, &audit_scenario)?;
    let context = RunContext::from_scenario(&audit_scenario)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("failed to build CB0 context for {}", input.row_id))?;
    let artifacts = run_controller_spec(&context, &input.controller)
        .with_context(|| format!("failed to run CB0 controller for {}", input.row_id))?;
    let terminal_class = classify_final_landing_terminal(&artifacts.run.manifest)
        .map_err(|error| anyhow!("CB0 case '{}' has invalid terminal: {error}", input.row_id))?;

    // The existing bundle writer includes performance/compute observations,
    // but the digest-bearing CB0 case below deliberately excludes them.
    write_artifact_bundle(
        case_dir,
        &audit_scenario,
        &input.controller,
        &artifacts,
        Some(&input.route_plan),
        None,
    )?;
    let persisted_bundle = load_persisted_raw_bundle(case_dir)?;
    build_final_landing_case(
        input,
        &audit_scenario,
        &artifacts,
        terminal_class,
        persisted_bundle.digest(),
    )
}

fn build_final_landing_case(
    input: &FinalLandingAuditInput,
    audit_scenario: &ScenarioSpec,
    artifacts: &ControlledRunArtifacts,
    terminal_class: FinalLandingTerminalClass,
    raw_bundle_digest: String,
) -> Result<FinalLandingAuditCase> {
    let manifest = artifacts.run.manifest.clone();
    let summary = &manifest.summary;
    let mut case = FinalLandingAuditCase {
        schema_id: FINAL_LANDING_AUDIT_SCHEMA_ID.to_owned(),
        schema_version: FINAL_LANDING_AUDIT_SCHEMA_VERSION,
        corpus: input.corpus,
        row_id: input.row_id.clone(),
        source_d0_input_digest: input.source_d0_input_digest.clone(),
        base_resolved_input_digest: input.base_resolved_input_digest.clone(),
        audit_resolved_input_digest: source_transition_resolved_input_digest(
            &input.row_id,
            audit_scenario,
            &input.route_plan,
            &input.controller,
        ),
        execution_identity_digest: execution_identity_digest(
            audit_scenario,
            &input.route_plan,
            &input.controller,
        )?,
        audit_scenario_digest: canonical_digest(audit_scenario).map_err(anyhow::Error::msg)?,
        route_plan_digest: input.route_plan.plan_digest.clone(),
        controller_id: input.controller.id().to_owned(),
        controller_digest: canonical_digest(&input.controller).map_err(anyhow::Error::msg)?,
        original_goal: input.scenario.mission.goal.clone(),
        audit_goal: audit_scenario.mission.goal.clone(),
        terminal_class,
        fuel_remaining_kg: summary.fuel_remaining_kg,
        fuel_used_kg: summary.fuel_used_kg,
        fuel_depleted: fuel_is_depleted(summary.fuel_remaining_kg),
        manifest,
        manifest_digest: String::new(),
        raw_bundle_digest,
        case_digest: String::new(),
    };
    case.manifest_digest = canonical_digest(&case.manifest).map_err(anyhow::Error::msg)?;
    case.seal().map_err(anyhow::Error::msg)?;
    validate_case_against_input(&case, input)?;
    Ok(case)
}

fn build_final_landing_summary(
    inputs: &[FinalLandingAuditInput],
    cases: &[FinalLandingAuditCase],
) -> Result<FinalLandingAuditSummary> {
    if inputs.len() != cases.len() {
        bail!("CB0 inputs and cases have different lengths");
    }
    let mut terminal_counts = BTreeMap::new();
    let mut fuel_depleted_count = 0;
    let mut indexes = Vec::with_capacity(cases.len());
    for (input, case) in inputs.iter().zip(cases) {
        case.validate().map_err(anyhow::Error::msg)?;
        validate_case_against_input(case, input)?;
        *terminal_counts
            .entry(case.terminal_class.as_str().to_owned())
            .or_insert(0) += 1;
        if case.fuel_depleted {
            fuel_depleted_count += 1;
        }
        indexes.push(FinalLandingAuditCaseIndex {
            corpus: case.corpus,
            row_id: case.row_id.clone(),
            terminal_class: case.terminal_class,
            fuel_depleted: case.fuel_depleted,
            case_digest: case.case_digest.clone(),
        });
    }
    let baseline_case_count = indexes
        .iter()
        .filter(|case| case.corpus == FinalLandingAuditCorpus::Baseline)
        .count();
    let diagnostic_case_count = indexes
        .iter()
        .filter(|case| case.corpus == FinalLandingAuditCorpus::Diagnostic)
        .count();
    let first = inputs
        .first()
        .ok_or_else(|| anyhow!("CB0 requires at least one input"))?;
    // The resolver validates the manifest before this function is called.  All
    // summary input identities are copied from that resolved corpus.
    let mut summary = FinalLandingAuditSummary {
        schema_id: FINAL_LANDING_AUDIT_SCHEMA_ID.to_owned(),
        schema_version: FINAL_LANDING_AUDIT_SCHEMA_VERSION,
        manifest_id: first.manifest_id.clone(),
        source_d0_input_digest: first.source_d0_input_digest.clone(),
        baseline_resolved_input_digest: first.baseline_resolved_input_digest.clone(),
        diagnostic_resolved_input_digest: first.diagnostic_resolved_input_digest.clone(),
        baseline_case_count,
        diagnostic_case_count,
        terminal_counts,
        fuel_depleted_count,
        cases: indexes,
        summary_digest: String::new(),
    };
    summary.seal().map_err(anyhow::Error::msg)?;
    Ok(summary)
}

fn validate_case_input_identity(
    input: &FinalLandingAuditInput,
    audit_scenario: &ScenarioSpec,
) -> Result<()> {
    let expected = landing_goal_overlay(&input.scenario, &input.route_plan)?;
    if *audit_scenario != expected {
        bail!("CB0 overlay changed fields other than the evaluation goal");
    }
    if input.scenario.mission.transfer_route != Some(input.route_plan.route.clone())
        || audit_scenario.mission.transfer_route != Some(input.route_plan.route.clone())
    {
        bail!("CB0 route plan and scenario transfer route do not match");
    }
    if input.route_plan.route.target_pad_id != audit_scenario.mission.goal.target_pad_id() {
        bail!("CB0 overlay target pad differs from route plan target");
    }
    if input.controller.id().is_empty() {
        bail!("CB0 controller identity is empty");
    }
    Ok(())
}

fn validate_case_against_input(
    case: &FinalLandingAuditCase,
    input: &FinalLandingAuditInput,
) -> Result<()> {
    case.validate().map_err(anyhow::Error::msg)?;
    if case.corpus != input.corpus
        || case.row_id != input.row_id
        || case.source_d0_input_digest != input.source_d0_input_digest
        || case.base_resolved_input_digest != input.base_resolved_input_digest
        || case.route_plan_digest != input.route_plan.plan_digest
        || case.controller_id != input.controller.id()
        || case.original_goal != input.scenario.mission.goal
    {
        bail!("CB0 case input identity mismatch for {}", input.row_id);
    }
    let audit_scenario = landing_goal_overlay(&input.scenario, &input.route_plan)?;
    if case.audit_goal != audit_scenario.mission.goal
        || case.audit_resolved_input_digest
            != source_transition_resolved_input_digest(
                &input.row_id,
                &audit_scenario,
                &input.route_plan,
                &input.controller,
            )
        || case.audit_scenario_digest
            != canonical_digest(&audit_scenario).map_err(anyhow::Error::msg)?
        || case.execution_identity_digest
            != execution_identity_digest(&audit_scenario, &input.route_plan, &input.controller)?
        || case.controller_digest
            != canonical_digest(&input.controller).map_err(anyhow::Error::msg)?
    {
        bail!("CB0 case derived identity mismatch for {}", input.row_id);
    }
    Ok(())
}

fn validate_raw_bundle_digest(
    case: &FinalLandingAuditCase,
    bundle: &PersistedRawBundle,
) -> Result<()> {
    if case.raw_bundle_digest != bundle.digest() {
        bail!("CB0 case '{}' raw bundle digest mismatch", case.row_id);
    }
    Ok(())
}

fn validate_bundle_against_case(
    case: &FinalLandingAuditCase,
    input: &FinalLandingAuditInput,
    case_dir: &Path,
) -> Result<()> {
    for file in [
        "scenario.json",
        "controller.json",
        "controller_updates.json",
        "performance.json",
        "manifest.json",
        "actions.json",
        "events.json",
        "samples.json",
        "route_plan.json",
    ] {
        if !case_dir.join(file).is_file() {
            bail!("CB0 case '{}' is missing bundle file '{file}'", case.row_id);
        }
    }
    let audit_scenario: ScenarioSpec = read_json(&case_dir.join("scenario.json"))?;
    let controller: ControllerSpec = read_json(&case_dir.join("controller.json"))?;
    let route_plan: RoutePlan = read_json(&case_dir.join("route_plan.json"))?;
    let bundle = load_persisted_raw_bundle(case_dir)?;
    let manifest = &bundle.artifacts.manifest;
    let expected_scenario = landing_goal_overlay(&input.scenario, &input.route_plan)?;
    if audit_scenario != expected_scenario
        || controller != input.controller
        || route_plan != input.route_plan
        || manifest != &case.manifest
    {
        bail!(
            "CB0 case '{}' bundle differs from its sealed result",
            case.row_id
        );
    }
    validate_raw_bundle_digest(case, &bundle)?;
    if manifest.scenario_id != expected_scenario.id
        || manifest.scenario_name != expected_scenario.name
        || manifest.scenario_seed != expected_scenario.seed
        || manifest.scenario_tags != expected_scenario.tags
        || manifest.controller_id != controller.id()
        || manifest.physics_hz != expected_scenario.sim.physics_hz
        || manifest.controller_hz != expected_scenario.sim.controller_hz
    {
        bail!(
            "CB0 case '{}' manifest identity differs from its audit scenario",
            case.row_id
        );
    }
    Ok(())
}

fn landing_goal_overlay(scenario: &ScenarioSpec, route_plan: &RoutePlan) -> Result<ScenarioSpec> {
    let target_pad_id = scenario.mission.goal.target_pad_id().to_owned();
    if route_plan.route.target_pad_id != target_pad_id {
        bail!("CB0 source scenario and route plan target pads differ");
    }
    let mut overlay = scenario.clone();
    overlay.mission.goal = EvaluationGoal::LandingOnPad { target_pad_id };
    overlay.validate().map_err(anyhow::Error::msg)?;
    Ok(overlay)
}

fn execution_identity_digest(
    scenario: &ScenarioSpec,
    route_plan: &RoutePlan,
    controller: &ControllerSpec,
) -> Result<String> {
    let mut scenario_value = serde_json::to_value(scenario)?;
    let mission = scenario_value
        .get_mut("mission")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("scenario mission did not serialize as an object"))?;
    mission.remove("goal");
    canonical_digest(&(scenario_value, route_plan, controller)).map_err(anyhow::Error::msg)
}

pub fn classify_final_landing_terminal(
    manifest: &RunManifest,
) -> Result<FinalLandingTerminalClass, String> {
    match (
        &manifest.physical_outcome,
        &manifest.mission_outcome,
        &manifest.end_reason,
    ) {
        (
            PhysicalOutcome::LandedOnTarget,
            MissionOutcome::Success,
            EndReason::TouchdownOnTarget,
        ) => Ok(FinalLandingTerminalClass::TargetTouchdown),
        (
            PhysicalOutcome::LandedOffTarget,
            MissionOutcome::FailedOffTarget,
            EndReason::TouchdownOffTarget,
        ) => Ok(FinalLandingTerminalClass::OffTargetTouchdown),
        (PhysicalOutcome::Crashed, MissionOutcome::FailedCrash, EndReason::Crash) => {
            Ok(FinalLandingTerminalClass::Crash)
        }
        (PhysicalOutcome::TimedOut, MissionOutcome::FailedTimeout, EndReason::MaxTimeReached) => {
            Ok(FinalLandingTerminalClass::MaxTime)
        }
        _ => Err(format!(
            "unexpected terminal tuple: physical={:?}, mission={:?}, end={:?}",
            manifest.physical_outcome, manifest.mission_outcome, manifest.end_reason
        )),
    }
}

fn fuel_is_depleted(fuel_remaining_kg: f64) -> bool {
    fuel_remaining_kg <= FINAL_LANDING_AUDIT_FUEL_EPSILON_KG
}

fn require_empty_output_dir(output_dir: &Path) -> Result<()> {
    match fs::read_dir(output_dir) {
        Ok(mut entries) => {
            if entries.next().transpose()?.is_some() {
                bail!(
                    "final-landing audit output directory must be empty: {}",
                    output_dir.display()
                );
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to inspect final-landing audit output directory {}",
                    output_dir.display()
                )
            });
        }
    }
    Ok(())
}

fn inputs_source_digest(inputs: &[FinalLandingAuditInput]) -> String {
    inputs
        .first()
        .map(|input| input.source_d0_input_digest.clone())
        .unwrap_or_default()
}

fn inputs_baseline_count(inputs: &[FinalLandingAuditInput]) -> usize {
    inputs
        .iter()
        .filter(|input| input.corpus == FinalLandingAuditCorpus::Baseline)
        .count()
}

fn inputs_diagnostic_count(inputs: &[FinalLandingAuditInput]) -> usize {
    inputs
        .iter()
        .filter(|input| input.corpus == FinalLandingAuditCorpus::Diagnostic)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::{ActionLogEntry, Command, RunSummary};
    use serde::de::DeserializeOwned;
    use std::path::PathBuf;

    fn fixtures_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval is a workspace member")
            .join("fixtures")
    }

    fn development_manifest_path() -> PathBuf {
        fixtures_root()
            .join("manifests")
            .join("source_transition_d0a_development.json")
    }

    fn test_manifest(
        physical_outcome: PhysicalOutcome,
        mission_outcome: MissionOutcome,
        end_reason: EndReason,
        fuel_remaining_kg: f64,
    ) -> RunManifest {
        RunManifest {
            schema_version: 1,
            scenario_id: "test_case".to_owned(),
            scenario_name: "test case".to_owned(),
            scenario_seed: 0,
            scenario_tags: Vec::new(),
            controller_id: "test_controller".to_owned(),
            physics_hz: 120,
            controller_hz: 60,
            sim_time_s: 1.0,
            physics_steps: 120,
            controller_updates: 60,
            physical_outcome,
            mission_outcome,
            end_reason,
            summary: RunSummary {
                fuel_remaining_kg,
                fuel_used_kg: 100.0 - fuel_remaining_kg,
                ..RunSummary::default()
            },
        }
    }

    fn raw_digest_case(manifest: RunManifest, raw_bundle_digest: String) -> FinalLandingAuditCase {
        FinalLandingAuditCase {
            schema_id: FINAL_LANDING_AUDIT_SCHEMA_ID.to_owned(),
            schema_version: FINAL_LANDING_AUDIT_SCHEMA_VERSION,
            corpus: FinalLandingAuditCorpus::Baseline,
            row_id: "test_case".to_owned(),
            source_d0_input_digest: "fnv1a64:d0".to_owned(),
            base_resolved_input_digest: "fnv1a64:base".to_owned(),
            audit_resolved_input_digest: "fnv1a64:audit".to_owned(),
            execution_identity_digest: "identity".to_owned(),
            audit_scenario_digest: "scenario".to_owned(),
            route_plan_digest: "plan".to_owned(),
            controller_id: "test_controller".to_owned(),
            controller_digest: "controller".to_owned(),
            original_goal: EvaluationGoal::LandingOnPad {
                target_pad_id: "target".to_owned(),
            },
            audit_goal: EvaluationGoal::LandingOnPad {
                target_pad_id: "target".to_owned(),
            },
            terminal_class: FinalLandingTerminalClass::TargetTouchdown,
            fuel_remaining_kg: manifest.summary.fuel_remaining_kg,
            fuel_used_kg: manifest.summary.fuel_used_kg,
            fuel_depleted: fuel_is_depleted(manifest.summary.fuel_remaining_kg),
            manifest_digest: canonical_digest(&manifest).expect("manifest digest"),
            manifest,
            raw_bundle_digest,
            case_digest: "case".to_owned(),
        }
    }

    fn json_round_trip<T>(value: &T) -> T
    where
        T: DeserializeOwned + Serialize,
    {
        serde_json::from_str(&serde_json::to_string(value).expect("serialize round-trip value"))
            .expect("parse round-trip value")
    }

    #[test]
    fn cb0_raw_bundle_digest_rejects_behavior_tampering_but_ignores_compute_time() {
        let artifacts = RunArtifacts {
            manifest: test_manifest(
                PhysicalOutcome::LandedOnTarget,
                MissionOutcome::Success,
                EndReason::TouchdownOnTarget,
                10.0,
            ),
            actions: vec![ActionLogEntry {
                sim_time_s: 0.0,
                physics_step: 0,
                controller_update_index: 0,
                command: Command::idle(),
            }],
            events: Vec::new(),
            samples: Vec::new(),
        };
        let updates = vec![ControllerUpdateRecord {
            sim_time_s: 0.0,
            physics_step: 0,
            controller_update_index: 0,
            compute_time_us: Some(1),
            frame: pd_control::ControllerFrame::command_only(Command::idle()),
        }];
        let bundle = PersistedRawBundle {
            artifacts: artifacts.clone(),
            controller_updates: updates.clone(),
        };
        let case = raw_digest_case(artifacts.manifest.clone(), bundle.digest());
        assert!(validate_raw_bundle_digest(&case, &bundle).is_ok());

        let mut tampered_actions = artifacts.actions.clone();
        tampered_actions[0].command.throttle_frac = 0.5;
        let tampered_bundle = PersistedRawBundle {
            artifacts: RunArtifacts {
                manifest: artifacts.manifest.clone(),
                actions: tampered_actions,
                events: artifacts.events.clone(),
                samples: artifacts.samples.clone(),
            },
            controller_updates: updates.clone(),
        };
        assert!(validate_raw_bundle_digest(&case, &tampered_bundle).is_err());

        let mut timing_only_updates = updates;
        timing_only_updates[0].compute_time_us = Some(999_999);
        let timing_only_bundle = PersistedRawBundle {
            artifacts,
            controller_updates: timing_only_updates,
        };
        assert!(validate_raw_bundle_digest(&case, &timing_only_bundle).is_ok());
    }

    #[test]
    fn cb0_raw_bundle_digest_survives_json_round_trip() {
        let artifacts = RunArtifacts {
            manifest: test_manifest(
                PhysicalOutcome::LandedOnTarget,
                MissionOutcome::Success,
                EndReason::TouchdownOnTarget,
                10.0,
            ),
            actions: vec![ActionLogEntry {
                sim_time_s: 0.0,
                physics_step: 0,
                controller_update_index: 0,
                command: Command::idle(),
            }],
            events: Vec::new(),
            samples: Vec::new(),
        };
        let mut frame = pd_control::ControllerFrame::command_only(Command::idle());
        frame.metrics.insert(
            "integer_metric".to_owned(),
            pd_control::TelemetryValue::Integer(1),
        );
        let updates = vec![ControllerUpdateRecord {
            sim_time_s: 0.0,
            physics_step: 0,
            controller_update_index: 0,
            compute_time_us: Some(1),
            frame,
        }];
        let expected = source_transition_raw_bundle_digest(&artifacts, &updates);
        let persisted_bundle = PersistedRawBundle {
            artifacts: RunArtifacts {
                manifest: json_round_trip(&artifacts.manifest),
                actions: json_round_trip(&artifacts.actions),
                events: json_round_trip(&artifacts.events),
                samples: json_round_trip(&artifacts.samples),
            },
            controller_updates: json_round_trip(&updates),
        };
        let persisted = persisted_bundle.digest();
        assert_ne!(expected, persisted);
        let case = raw_digest_case(persisted_bundle.artifacts.manifest.clone(), persisted);
        assert!(validate_raw_bundle_digest(&case, &persisted_bundle).is_ok());
    }

    #[test]
    fn cb0_resolves_exact_development_order() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval is a workspace member")
            .to_path_buf();
        let inputs = collect_final_landing_inputs(&development_manifest_path(), &repo_root)
            .expect("CB0 inputs should resolve");
        assert_eq!(inputs.len(), 60);
        assert_eq!(inputs_baseline_count(&inputs), 36);
        assert_eq!(inputs_diagnostic_count(&inputs), 24);
        assert!(
            inputs[..36]
                .iter()
                .all(|input| input.corpus == FinalLandingAuditCorpus::Baseline)
        );
        assert!(
            inputs[36..]
                .iter()
                .all(|input| input.corpus == FinalLandingAuditCorpus::Diagnostic)
        );
        assert_eq!(
            inputs[0].row_id,
            "planner_contract_single_empty_single_mid_ridge_empty_rneg30_seed_00_waypoint_handoff"
        );
        assert_eq!(
            inputs[36].row_id,
            "planner_angle_expansion_contract_single_empty_single_mid_ridge_empty_rneg60_seed_00_waypoint_handoff"
        );
    }

    #[test]
    fn cb0_overlay_changes_only_goal_and_preserves_identity() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval is a workspace member")
            .to_path_buf();
        let inputs = collect_final_landing_inputs(&development_manifest_path(), &repo_root)
            .expect("CB0 inputs should resolve");
        let input = &inputs[0];
        let overlay = landing_goal_overlay(&input.scenario, &input.route_plan)
            .expect("overlay should validate");
        let mut expected = input.scenario.clone();
        expected.mission.goal = overlay.mission.goal.clone();
        assert_eq!(overlay, expected);
        assert_eq!(overlay.world, input.scenario.world);
        assert_eq!(overlay.vehicle, input.scenario.vehicle);
        assert_eq!(overlay.initial_state, input.scenario.initial_state);
        assert_eq!(overlay.sim, input.scenario.sim);
        assert_eq!(
            overlay.mission.transfer_route,
            input.scenario.mission.transfer_route
        );
        assert_eq!(input.controller.id(), "transfer_waypoint_pdg_v1");
    }

    #[test]
    fn cb0_overlay_rejects_target_mismatch() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval is a workspace member")
            .to_path_buf();
        let inputs = collect_final_landing_inputs(&development_manifest_path(), &repo_root)
            .expect("CB0 inputs should resolve");
        let input = &inputs[0];
        let mut tampered_route = input.route_plan.clone();
        tampered_route.route.target_pad_id = "tampered_pad".to_owned();
        assert!(landing_goal_overlay(&input.scenario, &tampered_route).is_err());
    }

    #[test]
    fn cb0_classifies_only_authoritative_terminal_tuples() {
        let cases = [
            (
                PhysicalOutcome::LandedOnTarget,
                MissionOutcome::Success,
                EndReason::TouchdownOnTarget,
                FinalLandingTerminalClass::TargetTouchdown,
            ),
            (
                PhysicalOutcome::LandedOffTarget,
                MissionOutcome::FailedOffTarget,
                EndReason::TouchdownOffTarget,
                FinalLandingTerminalClass::OffTargetTouchdown,
            ),
            (
                PhysicalOutcome::Crashed,
                MissionOutcome::FailedCrash,
                EndReason::Crash,
                FinalLandingTerminalClass::Crash,
            ),
            (
                PhysicalOutcome::TimedOut,
                MissionOutcome::FailedTimeout,
                EndReason::MaxTimeReached,
                FinalLandingTerminalClass::MaxTime,
            ),
        ];
        for (physical, mission, end, expected) in cases {
            let manifest = test_manifest(physical, mission, end, 10.0);
            assert_eq!(classify_final_landing_terminal(&manifest), Ok(expected));
        }

        let unexpected = test_manifest(
            PhysicalOutcome::Flying,
            MissionOutcome::InProgress,
            EndReason::Running,
            10.0,
        );
        assert!(classify_final_landing_terminal(&unexpected).is_err());
    }

    #[test]
    fn cb0_fuel_flag_is_secondary_and_canonical() {
        let mut manifest = test_manifest(
            PhysicalOutcome::TimedOut,
            MissionOutcome::FailedTimeout,
            EndReason::MaxTimeReached,
            0.0,
        );
        let mut case = FinalLandingAuditCase {
            schema_id: FINAL_LANDING_AUDIT_SCHEMA_ID.to_owned(),
            schema_version: FINAL_LANDING_AUDIT_SCHEMA_VERSION,
            corpus: FinalLandingAuditCorpus::Baseline,
            row_id: "test_case".to_owned(),
            source_d0_input_digest: "fnv1a64:d0".to_owned(),
            base_resolved_input_digest: "fnv1a64:base".to_owned(),
            audit_resolved_input_digest: "fnv1a64:audit".to_owned(),
            execution_identity_digest: "identity".to_owned(),
            audit_scenario_digest: "scenario".to_owned(),
            route_plan_digest: "plan".to_owned(),
            controller_id: "test_controller".to_owned(),
            controller_digest: "controller".to_owned(),
            original_goal: EvaluationGoal::LandingOnPad {
                target_pad_id: "target".to_owned(),
            },
            audit_goal: EvaluationGoal::LandingOnPad {
                target_pad_id: "target".to_owned(),
            },
            terminal_class: FinalLandingTerminalClass::MaxTime,
            fuel_remaining_kg: 0.0,
            fuel_used_kg: 100.0,
            fuel_depleted: true,
            manifest: manifest.clone(),
            manifest_digest: String::new(),
            raw_bundle_digest: "raw_bundle".to_owned(),
            case_digest: String::new(),
        };
        case.manifest_digest = canonical_digest(&manifest).expect("manifest digest");
        case.seal().expect("case should seal");
        assert!(case.validate().is_ok());
        let first_digest = case.case_digest.clone();
        case.seal()
            .expect("repeated sealing should be deterministic");
        assert_eq!(case.case_digest, first_digest);
        case.fuel_depleted = false;
        assert!(case.validate().is_err());
        manifest.summary.fuel_remaining_kg = 1.0;
        assert!(!fuel_is_depleted(manifest.summary.fuel_remaining_kg));
    }

    #[test]
    fn cb0_rejects_nonempty_output_root() {
        let output = std::env::temp_dir().join(format!(
            "powered_descent_cb0_nonempty_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&output);
        fs::create_dir_all(&output).expect("temp output");
        fs::write(output.join("owned"), b"owned").expect("marker");
        let error = require_empty_output_dir(&output).expect_err("nonempty root must reject");
        assert!(error.to_string().contains("must be empty"));
        fs::remove_dir_all(output).expect("cleanup");
    }
}
