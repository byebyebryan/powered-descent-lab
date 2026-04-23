use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result, anyhow, bail};
use pd_control::{
    ControlledRunArtifacts, ControllerSpec, built_in_controller_spec, run_controller_spec,
};
use pd_core::{MissionOutcome, RunContext, RunManifest, RunSummary, SampleRecord, ScenarioSpec};
use rayon::{ThreadPoolBuilder, prelude::*};
use serde::{Deserialize, Serialize};

pub mod report;

#[cfg(unix)]
use std::os::unix::fs as platform_fs;
#[cfg(windows)]
use std::os::windows::fs as platform_fs;

pub const BATCH_REPORT_SCHEMA_VERSION: u32 = 7;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchMetricSummary {
    pub mean: f64,
    #[serde(default)]
    pub stddev: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchRunReviewMetrics {
    #[serde(default)]
    pub fuel_used_pct_of_max: Option<f64>,
    #[serde(default)]
    pub landing_offset_abs_m: Option<f64>,
    #[serde(default)]
    pub reference_gap_mean_m: Option<f64>,
    #[serde(default)]
    pub reference_gap_max_m: Option<f64>,
}

fn default_selector_value() -> String {
    "unspecified".to_owned()
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SelectorAxes {
    #[serde(default = "default_selector_value")]
    pub mission: String,
    #[serde(default = "default_selector_value")]
    pub arrival_family: String,
    #[serde(default = "default_selector_value")]
    pub condition_set: String,
    #[serde(default = "default_selector_value")]
    pub vehicle_variant: String,
    #[serde(default = "default_selector_value")]
    pub arc_point: String,
    #[serde(default = "default_selector_value")]
    pub velocity_band: String,
    #[serde(default)]
    pub expectation_tier: Option<String>,
}

impl Default for SelectorAxes {
    fn default() -> Self {
        Self {
            mission: default_selector_value(),
            arrival_family: default_selector_value(),
            condition_set: default_selector_value(),
            vehicle_variant: default_selector_value(),
            arc_point: default_selector_value(),
            velocity_band: default_selector_value(),
            expectation_tier: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioPackSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub entries: Vec<ScenarioPackEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScenarioPackEntry {
    Scenario(ConcreteScenarioPackEntry),
    Family(ScenarioFamilyEntry),
    TerminalMatrix(TerminalMatrixEntry),
}

impl ScenarioPackEntry {
    fn id(&self) -> &str {
        match self {
            Self::Scenario(entry) => &entry.id,
            Self::Family(entry) => &entry.id,
            Self::TerminalMatrix(entry) => &entry.id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConcreteScenarioPackEntry {
    pub id: String,
    pub scenario: String,
    pub controller: String,
    #[serde(default)]
    pub controller_config: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioFamilyEntry {
    pub id: String,
    pub family: String,
    pub base_scenario: String,
    pub controller: String,
    #[serde(default)]
    pub controller_config: Option<String>,
    #[serde(default)]
    pub seeds: Vec<u64>,
    #[serde(default)]
    pub seed_range: Option<SeedRangeSpec>,
    #[serde(default)]
    pub perturbations: Vec<NumericPerturbationSpec>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalMatrixEntry {
    pub id: String,
    pub terminal_matrix: String,
    pub base_scenario: String,
    pub lanes: Vec<TerminalMatrixLaneSpec>,
    pub seed_tier: TerminalSeedTier,
    pub condition_set: String,
    pub vehicle_variant: String,
    pub expectation_tier: String,
    #[serde(default)]
    pub adjustments: Vec<NumericAdjustmentSpec>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalMatrixLaneSpec {
    pub id: String,
    pub controller: String,
    #[serde(default)]
    pub controller_config: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalSeedTier {
    Smoke,
    Full,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NumericAdjustmentSpec {
    pub id: String,
    pub path: String,
    pub mode: NumericPerturbationMode,
    pub value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedRangeSpec {
    pub start: u64,
    pub count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NumericPerturbationSpec {
    pub id: String,
    pub path: String,
    pub mode: NumericPerturbationMode,
    pub min: f64,
    pub max: f64,
    #[serde(default)]
    pub quantize: Option<f64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericPerturbationMode {
    Set,
    Offset,
    Scale,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedRunSourceKind {
    ConcreteScenario,
    FamilySweep,
    TerminalMatrix,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedRunDescriptor {
    pub run_id: String,
    pub entry_id: String,
    pub source_kind: ResolvedRunSourceKind,
    pub scenario_source: String,
    pub resolved_scenario_id: String,
    pub resolved_scenario_name: String,
    pub family_id: Option<String>,
    #[serde(default)]
    pub selector: SelectorAxes,
    #[serde(default)]
    pub lane_id: String,
    pub resolved_seed: u64,
    pub resolved_parameters: BTreeMap<String, f64>,
    pub controller_id: String,
    pub controller_spec: ControllerSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchRunRecord {
    pub resolved: ResolvedRunDescriptor,
    pub manifest: RunManifest,
    #[serde(default)]
    pub review: BatchRunReviewMetrics,
    pub bundle_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchIdentity {
    pub schema_version: u32,
    pub pack_spec_digest: String,
    pub resolved_run_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchGroupSummary {
    pub key: String,
    pub total_runs: usize,
    pub success_runs: usize,
    pub failure_runs: usize,
    pub mean_sim_time_s: f64,
    #[serde(default)]
    pub sim_time_stats: Option<BatchMetricSummary>,
    #[serde(default)]
    pub mean_success_fuel_remaining_kg: Option<f64>,
    #[serde(default)]
    pub fuel_used_pct_of_max: Option<BatchMetricSummary>,
    #[serde(default)]
    pub landing_offset_abs_m: Option<BatchMetricSummary>,
    #[serde(default)]
    pub reference_gap_mean_m: Option<BatchMetricSummary>,
    pub mission_outcomes: BTreeMap<String, usize>,
    pub end_reasons: BTreeMap<String, usize>,
    pub sample_run_ids: Vec<String>,
    pub failed_seeds: Vec<u64>,
    #[serde(default)]
    pub weakest_success_run_id: Option<String>,
    #[serde(default)]
    pub closest_failure_run_id: Option<String>,
    #[serde(default)]
    pub worst_failure_run_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchRunPointer {
    pub run_id: String,
    pub entry_id: String,
    pub family_id: Option<String>,
    #[serde(default)]
    pub selector: SelectorAxes,
    #[serde(default)]
    pub lane_id: String,
    pub scenario_id: String,
    pub scenario_seed: u64,
    pub controller_id: String,
    pub mission_outcome: String,
    pub end_reason: String,
    pub sim_time_s: f64,
    pub bundle_dir: Option<String>,
    #[serde(default)]
    pub margin_ratio: Option<f64>,
    #[serde(default)]
    pub fuel_remaining_kg: f64,
    #[serde(default)]
    pub review: BatchRunReviewMetrics,
    #[serde(default)]
    pub summary: RunSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchSummary {
    pub total_runs: usize,
    pub success_runs: usize,
    pub failure_runs: usize,
    pub mean_sim_time_s: f64,
    pub max_sim_time_s: f64,
    pub mission_outcomes: BTreeMap<String, usize>,
    pub physical_outcomes: BTreeMap<String, usize>,
    pub end_reasons: BTreeMap<String, usize>,
    pub by_entry: Vec<BatchGroupSummary>,
    pub by_family: Vec<BatchGroupSummary>,
    pub failed_runs: Vec<BatchRunPointer>,
    pub slowest_runs: Vec<BatchRunPointer>,
    #[serde(default)]
    pub closest_failures: Vec<BatchRunPointer>,
    #[serde(default)]
    pub worst_failures: Vec<BatchRunPointer>,
    #[serde(default)]
    pub weakest_successes: Vec<BatchRunPointer>,
    #[serde(default)]
    pub lowest_fuel_successes: Vec<BatchRunPointer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchReport {
    pub schema_version: u32,
    pub pack_id: String,
    pub pack_name: String,
    pub total_runs: usize,
    #[serde(default)]
    pub wall_clock_s: f64,
    pub workers_requested: usize,
    pub workers_used: usize,
    pub identity: BatchIdentity,
    pub resolved_runs: Vec<ResolvedRunDescriptor>,
    pub records: Vec<BatchRunRecord>,
    pub summary: BatchSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchCompareBasis {
    pub mode: String,
    pub shared_runs: usize,
    pub candidate_only_runs: usize,
    pub baseline_only_runs: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchSummaryDelta {
    pub candidate_success_rate: f64,
    pub baseline_success_rate: f64,
    pub success_rate_delta: f64,
    pub candidate_success_runs: usize,
    pub baseline_success_runs: usize,
    pub success_runs_delta: i64,
    pub candidate_failure_runs: usize,
    pub baseline_failure_runs: usize,
    pub failure_runs_delta: i64,
    pub candidate_mean_sim_time_s: f64,
    pub baseline_mean_sim_time_s: f64,
    pub mean_sim_time_delta_s: f64,
    pub candidate_max_sim_time_s: f64,
    pub baseline_max_sim_time_s: f64,
    pub max_sim_time_delta_s: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchGroupComparison {
    pub key: String,
    pub candidate_total_runs: Option<usize>,
    pub baseline_total_runs: Option<usize>,
    pub candidate_success_rate: Option<f64>,
    pub baseline_success_rate: Option<f64>,
    pub success_rate_delta: Option<f64>,
    pub candidate_failure_runs: Option<usize>,
    pub baseline_failure_runs: Option<usize>,
    pub failure_runs_delta: Option<i64>,
    pub candidate_mean_sim_time_s: Option<f64>,
    pub baseline_mean_sim_time_s: Option<f64>,
    pub mean_sim_time_delta_s: Option<f64>,
    pub candidate_failed_seeds: Vec<u64>,
    pub baseline_failed_seeds: Vec<u64>,
    pub sample_run_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchRunChangeKind {
    NewFailure,
    Recovered,
    OutcomeChanged,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchRunComparison {
    pub run_id: String,
    pub entry_id: String,
    pub family_id: Option<String>,
    #[serde(default)]
    pub selector: SelectorAxes,
    #[serde(default)]
    pub lane_id: String,
    pub change_kind: BatchRunChangeKind,
    pub candidate_seed: u64,
    pub baseline_seed: u64,
    pub candidate_mission_outcome: String,
    pub baseline_mission_outcome: String,
    pub candidate_end_reason: String,
    pub baseline_end_reason: String,
    pub candidate_sim_time_s: f64,
    pub baseline_sim_time_s: f64,
    pub sim_time_delta_s: f64,
    pub candidate_bundle_dir: Option<String>,
    pub baseline_bundle_dir: Option<String>,
    #[serde(default)]
    pub candidate_margin_ratio: Option<f64>,
    #[serde(default)]
    pub baseline_margin_ratio: Option<f64>,
    #[serde(default)]
    pub margin_ratio_delta: Option<f64>,
    #[serde(default)]
    pub candidate_fuel_remaining_kg: f64,
    #[serde(default)]
    pub baseline_fuel_remaining_kg: f64,
    #[serde(default)]
    pub fuel_remaining_delta_kg: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchComparison {
    pub candidate_pack_id: String,
    pub candidate_pack_name: String,
    pub baseline_pack_id: String,
    pub baseline_pack_name: String,
    pub basis: BatchCompareBasis,
    pub summary: BatchSummaryDelta,
    pub by_entry: Vec<BatchGroupComparison>,
    pub by_family: Vec<BatchGroupComparison>,
    pub regressions: Vec<BatchRunComparison>,
    pub improvements: Vec<BatchRunComparison>,
    pub outcome_changes: Vec<BatchRunComparison>,
    pub candidate_only: Vec<BatchRunPointer>,
    pub baseline_only: Vec<BatchRunPointer>,
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

pub fn compare_batch_reports(candidate: &BatchReport, baseline: &BatchReport) -> BatchComparison {
    let candidate_records = candidate
        .records
        .iter()
        .map(|record| (record.resolved.run_id.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let baseline_records = baseline
        .records
        .iter()
        .map(|record| (record.resolved.run_id.clone(), record))
        .collect::<BTreeMap<_, _>>();

    let mut shared_run_ids = Vec::new();
    let mut candidate_only = Vec::new();
    for (run_id, record) in &candidate_records {
        if baseline_records.contains_key(run_id) {
            shared_run_ids.push(run_id.clone());
        } else {
            candidate_only.push(run_pointer(record));
        }
    }

    let mut baseline_only = Vec::new();
    for (run_id, record) in &baseline_records {
        if !candidate_records.contains_key(run_id) {
            baseline_only.push(run_pointer(record));
        }
    }

    let mut regressions = Vec::new();
    let mut improvements = Vec::new();
    let mut outcome_changes = Vec::new();
    for run_id in shared_run_ids.iter().cloned() {
        let candidate_record = candidate_records
            .get(&run_id)
            .expect("shared run ids should exist in candidate map");
        let baseline_record = baseline_records
            .get(&run_id)
            .expect("shared run ids should exist in baseline map");
        if let Some(comparison) = compare_run_pair(candidate_record, baseline_record) {
            match comparison.change_kind {
                BatchRunChangeKind::NewFailure => regressions.push(comparison),
                BatchRunChangeKind::Recovered => improvements.push(comparison),
                BatchRunChangeKind::OutcomeChanged => outcome_changes.push(comparison),
            }
        }
    }

    regressions.sort_by(run_comparison_sort_key);
    improvements.sort_by(run_comparison_sort_key);
    outcome_changes.sort_by(run_comparison_sort_key);
    candidate_only.sort_by(run_pointer_sort_key);
    baseline_only.sort_by(run_pointer_sort_key);

    BatchComparison {
        candidate_pack_id: candidate.pack_id.clone(),
        candidate_pack_name: candidate.pack_name.clone(),
        baseline_pack_id: baseline.pack_id.clone(),
        baseline_pack_name: baseline.pack_name.clone(),
        basis: BatchCompareBasis {
            mode: "run_id".to_owned(),
            shared_runs: shared_run_ids.len(),
            candidate_only_runs: candidate_only.len(),
            baseline_only_runs: baseline_only.len(),
        },
        summary: compare_summary_delta(&candidate.summary, &baseline.summary),
        by_entry: compare_group_sets(&candidate.summary.by_entry, &baseline.summary.by_entry),
        by_family: compare_group_sets(&candidate.summary.by_family, &baseline.summary.by_family),
        regressions,
        improvements,
        outcome_changes,
        candidate_only,
        baseline_only,
    }
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
    let identity = BatchIdentity {
        schema_version: BATCH_REPORT_SCHEMA_VERSION,
        pack_spec_digest: stable_digest(pack)?,
        resolved_run_digest: stable_digest(
            &resolved_runs
                .iter()
                .map(|run| &run.descriptor)
                .collect::<Vec<_>>(),
        )?,
    };

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

fn validate_pack(pack: &ScenarioPackSpec) -> Result<()> {
    if pack.id.trim().is_empty() {
        bail!("pack id must not be empty");
    }
    if pack.name.trim().is_empty() {
        bail!("pack name must not be empty");
    }
    if pack.entries.is_empty() {
        bail!("pack must contain at least one entry");
    }

    let mut seen_ids = BTreeSet::new();
    for entry in &pack.entries {
        if entry.id().trim().is_empty() {
            bail!("pack entry id must not be empty");
        }
        if !seen_ids.insert(entry.id().to_owned()) {
            bail!("duplicate pack entry id '{}'", entry.id());
        }

        match entry {
            ScenarioPackEntry::Scenario(entry) => {
                if entry.controller.trim().is_empty() {
                    bail!("pack entry controller must not be empty");
                }
                if entry.scenario.trim().is_empty() {
                    bail!("pack entry scenario path must not be empty");
                }
            }
            ScenarioPackEntry::Family(entry) => validate_family_entry(entry)?,
            ScenarioPackEntry::TerminalMatrix(entry) => validate_terminal_matrix_entry(entry)?,
        }
    }

    Ok(())
}

fn validate_family_entry(entry: &ScenarioFamilyEntry) -> Result<()> {
    if entry.controller.trim().is_empty() {
        bail!(
            "family entry '{}' must define a non-empty controller",
            entry.id
        );
    }
    if entry.family.trim().is_empty() {
        bail!(
            "family entry '{}' must define a non-empty family id",
            entry.id
        );
    }
    if entry.base_scenario.trim().is_empty() {
        bail!(
            "family entry '{}' must define a non-empty base_scenario path",
            entry.id
        );
    }
    if entry.tags.iter().any(|tag| tag.trim().is_empty()) {
        bail!(
            "family entry '{}' tags must not contain empty values",
            entry.id
        );
    }
    for (key, value) in &entry.metadata {
        if key.trim().is_empty() || value.trim().is_empty() {
            bail!(
                "family entry '{}' metadata keys and values must not be empty",
                entry.id
            );
        }
    }

    let explicit_seed_count = usize::from(!entry.seeds.is_empty());
    let range_seed_count = usize::from(entry.seed_range.is_some());
    if explicit_seed_count + range_seed_count != 1 {
        bail!(
            "family entry '{}' must define exactly one of 'seeds' or 'seed_range'",
            entry.id
        );
    }
    if let Some(seed_range) = &entry.seed_range
        && seed_range.count == 0
    {
        bail!("family entry '{}' seed_range.count must be > 0", entry.id);
    }
    if !entry.seeds.is_empty() {
        let mut seen = BTreeSet::new();
        for seed in &entry.seeds {
            if !seen.insert(*seed) {
                bail!(
                    "family entry '{}' seeds must not contain duplicates (duplicate seed {})",
                    entry.id,
                    seed
                );
            }
        }
    }

    let mut seen_ids = BTreeSet::new();
    for perturbation in &entry.perturbations {
        validate_numeric_perturbation(entry, perturbation, &mut seen_ids)?;
    }

    Ok(())
}

fn validate_terminal_matrix_entry(entry: &TerminalMatrixEntry) -> Result<()> {
    if entry.terminal_matrix.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty terminal_matrix",
            entry.id
        );
    }
    if entry.base_scenario.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty base_scenario",
            entry.id
        );
    }
    if entry.condition_set.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty condition_set",
            entry.id
        );
    }
    if entry.vehicle_variant.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty vehicle_variant",
            entry.id
        );
    }
    if entry.expectation_tier.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty expectation_tier",
            entry.id
        );
    }
    if entry.lanes.is_empty() {
        bail!(
            "terminal matrix entry '{}' must define at least one lane",
            entry.id
        );
    }
    let mut seen_lane_ids = BTreeSet::new();
    for lane in &entry.lanes {
        if lane.id.trim().is_empty() {
            bail!(
                "terminal matrix entry '{}' has a lane with an empty id",
                entry.id
            );
        }
        if lane.controller.trim().is_empty() {
            bail!(
                "terminal matrix entry '{}' lane '{}' must define a controller",
                entry.id,
                lane.id
            );
        }
        if !seen_lane_ids.insert(lane.id.clone()) {
            bail!(
                "terminal matrix entry '{}' has duplicate lane id '{}'",
                entry.id,
                lane.id
            );
        }
    }
    for (key, value) in &entry.metadata {
        if key.trim().is_empty() || value.trim().is_empty() {
            bail!(
                "terminal matrix entry '{}' metadata keys and values must not be empty",
                entry.id
            );
        }
    }
    let mut seen_adjustment_ids = BTreeSet::new();
    for adjustment in &entry.adjustments {
        validate_numeric_adjustment(entry, adjustment, &mut seen_adjustment_ids)?;
    }
    Ok(())
}

fn validate_numeric_adjustment(
    entry: &TerminalMatrixEntry,
    adjustment: &NumericAdjustmentSpec,
    seen_ids: &mut BTreeSet<String>,
) -> Result<()> {
    if adjustment.id.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' has an adjustment with an empty id",
            entry.id
        );
    }
    if !seen_ids.insert(adjustment.id.clone()) {
        bail!(
            "terminal matrix entry '{}' has duplicate adjustment id '{}'",
            entry.id,
            adjustment.id
        );
    }
    if !is_supported_terminal_adjustment_path(&adjustment.path) {
        bail!(
            "terminal matrix entry '{}' adjustment '{}' uses unsupported path '{}'",
            entry.id,
            adjustment.id,
            adjustment.path
        );
    }
    if !adjustment.value.is_finite() {
        bail!(
            "terminal matrix entry '{}' adjustment '{}' must be finite",
            entry.id,
            adjustment.id
        );
    }
    Ok(())
}

fn validate_numeric_perturbation(
    entry: &ScenarioFamilyEntry,
    perturbation: &NumericPerturbationSpec,
    seen_ids: &mut BTreeSet<String>,
) -> Result<()> {
    if perturbation.id.trim().is_empty() {
        bail!(
            "family entry '{}' has a perturbation with an empty id",
            entry.id
        );
    }
    if !seen_ids.insert(perturbation.id.clone()) {
        bail!(
            "family entry '{}' has duplicate perturbation id '{}'",
            entry.id,
            perturbation.id
        );
    }
    if !is_supported_numeric_path(&perturbation.path) {
        bail!(
            "family entry '{}' perturbation '{}' uses unsupported path '{}'",
            entry.id,
            perturbation.id,
            perturbation.path
        );
    }
    if !perturbation.min.is_finite() || !perturbation.max.is_finite() {
        bail!(
            "family entry '{}' perturbation '{}' bounds must be finite",
            entry.id,
            perturbation.id
        );
    }
    if perturbation.max < perturbation.min {
        bail!(
            "family entry '{}' perturbation '{}' max must be >= min",
            entry.id,
            perturbation.id
        );
    }
    if let Some(step) = perturbation.quantize
        && (!step.is_finite() || step <= 0.0)
    {
        bail!(
            "family entry '{}' perturbation '{}' quantize must be > 0",
            entry.id,
            perturbation.id
        );
    }
    Ok(())
}

fn resolve_pack_runs(pack: &ScenarioPackSpec, base_dir: &Path) -> Result<Vec<ResolvedBatchRun>> {
    let mut resolved = Vec::new();
    for entry in &pack.entries {
        match entry {
            ScenarioPackEntry::Scenario(entry) => {
                let controller_spec = load_controller_spec(
                    base_dir,
                    entry.controller.as_str(),
                    entry.controller_config.as_deref(),
                )?;
                resolved.push(resolve_concrete_run(entry, base_dir, &controller_spec)?)
            }
            ScenarioPackEntry::Family(entry) => {
                let controller_spec = load_controller_spec(
                    base_dir,
                    entry.controller.as_str(),
                    entry.controller_config.as_deref(),
                )?;
                resolved.extend(resolve_family_runs(entry, base_dir, &controller_spec)?)
            }
            ScenarioPackEntry::TerminalMatrix(entry) => {
                resolved.extend(resolve_terminal_matrix_runs(entry, base_dir)?)
            }
        }
    }

    let mut seen_run_ids = BTreeSet::new();
    for run in &resolved {
        if !seen_run_ids.insert(run.descriptor.run_id.clone()) {
            bail!(
                "resolved pack contains duplicate run id '{}'",
                run.descriptor.run_id
            );
        }
    }
    Ok(resolved)
}

fn resolve_concrete_run(
    entry: &ConcreteScenarioPackEntry,
    base_dir: &Path,
    controller_spec: &ControllerSpec,
) -> Result<ResolvedBatchRun> {
    let scenario_path = base_dir.join(&entry.scenario);
    let mut scenario = load_scenario(&scenario_path)?;
    scenario.metadata.extend(entry.metadata.clone());
    let family_id = scenario.metadata.get("family").cloned();
    let selector = selector_axes_from_metadata(&scenario.metadata);
    let descriptor = ResolvedRunDescriptor {
        run_id: sanitize_token(&entry.id),
        entry_id: entry.id.clone(),
        source_kind: ResolvedRunSourceKind::ConcreteScenario,
        scenario_source: entry.scenario.clone(),
        resolved_scenario_id: scenario.id.clone(),
        resolved_scenario_name: scenario.name.clone(),
        family_id,
        selector,
        lane_id: entry.controller.clone(),
        resolved_seed: scenario.seed,
        resolved_parameters: BTreeMap::new(),
        controller_id: controller_spec.id().to_owned(),
        controller_spec: controller_spec.clone(),
    };

    Ok(ResolvedBatchRun {
        descriptor,
        scenario,
    })
}

fn resolve_family_runs(
    entry: &ScenarioFamilyEntry,
    base_dir: &Path,
    controller_spec: &ControllerSpec,
) -> Result<Vec<ResolvedBatchRun>> {
    let base_path = base_dir.join(&entry.base_scenario);
    let base_scenario = load_scenario(&base_path)?;
    let mut runs = Vec::new();

    for seed in family_entry_seeds(entry) {
        let (scenario, resolved_parameters) = resolve_family_scenario(entry, &base_scenario, seed)?;
        let selector = selector_axes_from_metadata(&scenario.metadata);
        let descriptor = ResolvedRunDescriptor {
            run_id: resolved_family_run_id(&entry.id, seed),
            entry_id: entry.id.clone(),
            source_kind: ResolvedRunSourceKind::FamilySweep,
            scenario_source: entry.base_scenario.clone(),
            resolved_scenario_id: scenario.id.clone(),
            resolved_scenario_name: scenario.name.clone(),
            family_id: Some(entry.family.clone()),
            selector,
            lane_id: entry.controller.clone(),
            resolved_seed: seed,
            resolved_parameters,
            controller_id: controller_spec.id().to_owned(),
            controller_spec: controller_spec.clone(),
        };
        runs.push(ResolvedBatchRun {
            descriptor,
            scenario,
        });
    }

    Ok(runs)
}

#[derive(Clone, Copy, Debug)]
struct TerminalArcPointSpec {
    id: &'static str,
    angle_deg: f64,
    nominal_ttg_s: f64,
}

#[derive(Clone, Copy, Debug)]
struct TerminalArrivalFamilySpec {
    arrival_family: &'static str,
    gravity_mps2: f64,
    radius_nominal_m: f64,
    low_multiplier: f64,
    high_multiplier: f64,
    clamp_low_to_descending: bool,
    arc_points: &'static [TerminalArcPointSpec],
}

#[derive(Clone, Copy, Debug)]
struct TerminalBandSpec {
    id: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct TerminalSeedSpec {
    index: u64,
    radial_pct: Option<f64>,
    speed_pct: Option<f64>,
}

const HALF_ARC_TERMINAL_V1_ARC_POINTS: [TerminalArcPointSpec; 7] = [
    TerminalArcPointSpec {
        id: "a00",
        angle_deg: 0.0,
        nominal_ttg_s: 8.50,
    },
    TerminalArcPointSpec {
        id: "a15",
        angle_deg: 15.0,
        nominal_ttg_s: 8.50,
    },
    TerminalArcPointSpec {
        id: "a30",
        angle_deg: 30.0,
        nominal_ttg_s: 8.25,
    },
    TerminalArcPointSpec {
        id: "a45",
        angle_deg: 45.0,
        nominal_ttg_s: 8.00,
    },
    TerminalArcPointSpec {
        id: "a60",
        angle_deg: 60.0,
        nominal_ttg_s: 7.75,
    },
    TerminalArcPointSpec {
        id: "a70",
        angle_deg: 70.0,
        nominal_ttg_s: 7.50,
    },
    TerminalArcPointSpec {
        id: "a80",
        angle_deg: 80.0,
        nominal_ttg_s: 7.00,
    },
];

const HALF_ARC_TERMINAL_V1_SPEC: TerminalArrivalFamilySpec = TerminalArrivalFamilySpec {
    arrival_family: "half_arc_terminal_v1",
    gravity_mps2: 9.81,
    radius_nominal_m: 800.0,
    low_multiplier: 1.25,
    high_multiplier: 0.75,
    clamp_low_to_descending: false,
    arc_points: &HALF_ARC_TERMINAL_V1_ARC_POINTS,
};

const TERMINAL_BANDS: [TerminalBandSpec; 3] = [
    TerminalBandSpec { id: "low" },
    TerminalBandSpec { id: "mid" },
    TerminalBandSpec { id: "high" },
];

const TERMINAL_SMOKE_SEEDS: [TerminalSeedSpec; 3] = [
    TerminalSeedSpec {
        index: 0,
        radial_pct: Some(0.015),
        speed_pct: None,
    },
    TerminalSeedSpec {
        index: 1,
        radial_pct: Some(-0.015),
        speed_pct: None,
    },
    TerminalSeedSpec {
        index: 6,
        radial_pct: None,
        speed_pct: Some(0.010),
    },
];

const TERMINAL_FULL_SEEDS: [TerminalSeedSpec; 12] = [
    TerminalSeedSpec {
        index: 0,
        radial_pct: Some(0.015),
        speed_pct: None,
    },
    TerminalSeedSpec {
        index: 1,
        radial_pct: Some(-0.015),
        speed_pct: None,
    },
    TerminalSeedSpec {
        index: 2,
        radial_pct: Some(0.030),
        speed_pct: None,
    },
    TerminalSeedSpec {
        index: 3,
        radial_pct: Some(-0.030),
        speed_pct: None,
    },
    TerminalSeedSpec {
        index: 4,
        radial_pct: Some(0.045),
        speed_pct: None,
    },
    TerminalSeedSpec {
        index: 5,
        radial_pct: Some(-0.045),
        speed_pct: None,
    },
    TerminalSeedSpec {
        index: 6,
        radial_pct: None,
        speed_pct: Some(0.010),
    },
    TerminalSeedSpec {
        index: 7,
        radial_pct: None,
        speed_pct: Some(-0.010),
    },
    TerminalSeedSpec {
        index: 8,
        radial_pct: None,
        speed_pct: Some(0.020),
    },
    TerminalSeedSpec {
        index: 9,
        radial_pct: None,
        speed_pct: Some(-0.020),
    },
    TerminalSeedSpec {
        index: 10,
        radial_pct: None,
        speed_pct: Some(0.030),
    },
    TerminalSeedSpec {
        index: 11,
        radial_pct: None,
        speed_pct: Some(-0.030),
    },
];

fn resolve_terminal_matrix_runs(
    entry: &TerminalMatrixEntry,
    base_dir: &Path,
) -> Result<Vec<ResolvedBatchRun>> {
    let base_path = base_dir.join(&entry.base_scenario);
    let base_scenario = load_scenario(&base_path)?;
    let family_spec = terminal_arrival_family_spec(&entry.terminal_matrix)?;
    let seed_specs = terminal_seed_specs(entry.seed_tier);
    let mut runs = Vec::new();

    for lane in &entry.lanes {
        let controller_spec = load_controller_spec(
            base_dir,
            lane.controller.as_str(),
            lane.controller_config.as_deref(),
        )?;
        for arc in family_spec.arc_points {
            for band in TERMINAL_BANDS {
                for seed_spec in seed_specs {
                    let run_id = resolved_terminal_matrix_run_id(
                        &entry.id,
                        arc.id,
                        band.id,
                        seed_spec.index,
                        &lane.id,
                    );
                    let (scenario, resolved_parameters, selector) =
                        resolve_terminal_matrix_scenario(
                            entry,
                            &base_scenario,
                            family_spec,
                            arc,
                            band,
                            seed_spec,
                            &lane.id,
                            &run_id,
                        )?;
                    let descriptor = ResolvedRunDescriptor {
                        run_id,
                        entry_id: entry.id.clone(),
                        source_kind: ResolvedRunSourceKind::TerminalMatrix,
                        scenario_source: entry.base_scenario.clone(),
                        resolved_scenario_id: scenario.id.clone(),
                        resolved_scenario_name: scenario.name.clone(),
                        family_id: Some(entry.id.clone()),
                        selector,
                        lane_id: lane.id.clone(),
                        resolved_seed: seed_spec.index,
                        resolved_parameters,
                        controller_id: controller_spec.id().to_owned(),
                        controller_spec: controller_spec.clone(),
                    };
                    runs.push(ResolvedBatchRun {
                        descriptor,
                        scenario,
                    });
                }
            }
        }
    }

    Ok(runs)
}

fn terminal_arrival_family_spec(name: &str) -> Result<&'static TerminalArrivalFamilySpec> {
    match name {
        "half_arc_terminal_v1" => Ok(&HALF_ARC_TERMINAL_V1_SPEC),
        _ => bail!("unsupported terminal matrix '{}'", name),
    }
}

fn terminal_seed_specs(seed_tier: TerminalSeedTier) -> &'static [TerminalSeedSpec] {
    match seed_tier {
        TerminalSeedTier::Smoke => &TERMINAL_SMOKE_SEEDS,
        TerminalSeedTier::Full => &TERMINAL_FULL_SEEDS,
    }
}

fn resolved_terminal_matrix_run_id(
    entry_id: &str,
    arc_point: &str,
    velocity_band: &str,
    seed: u64,
    lane_id: &str,
) -> String {
    sanitize_token(&format!(
        "{entry_id}__{arc_point}__{velocity_band}__seed_{seed:02}__{lane_id}"
    ))
}

fn resolve_terminal_matrix_scenario(
    entry: &TerminalMatrixEntry,
    base_scenario: &ScenarioSpec,
    family_spec: &TerminalArrivalFamilySpec,
    arc: &TerminalArcPointSpec,
    band: TerminalBandSpec,
    seed_spec: &TerminalSeedSpec,
    lane_id: &str,
    run_id: &str,
) -> Result<(ScenarioSpec, BTreeMap<String, f64>, SelectorAxes)> {
    let mut scenario = base_scenario.clone();
    scenario.id = run_id.to_owned();
    scenario.name = format!(
        "{} [{} {} {} {} {} seed {} {}]",
        base_scenario.name,
        family_spec.arrival_family,
        entry.condition_set,
        entry.vehicle_variant,
        arc.id,
        band.id,
        seed_spec.index,
        lane_id
    );
    scenario.description = format!(
        "{} ({} {} {} {} {} {} seed {} lane {})",
        base_scenario.description,
        "terminal_matrix",
        family_spec.arrival_family,
        entry.condition_set,
        entry.vehicle_variant,
        arc.id,
        band.id,
        seed_spec.index,
        lane_id
    );
    scenario.seed = seed_spec.index;
    scenario.tags = merge_unique_tags(&base_scenario.tags, &entry.tags);
    scenario.metadata.extend(entry.metadata.clone());
    scenario
        .metadata
        .insert("family".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("family_entry_id".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("resolved_seed".to_owned(), seed_spec.index.to_string());
    scenario
        .metadata
        .insert("mission".to_owned(), "terminal_guidance".to_owned());
    scenario.metadata.insert(
        "arrival_family".to_owned(),
        family_spec.arrival_family.to_owned(),
    );
    scenario
        .metadata
        .insert("condition_set".to_owned(), entry.condition_set.clone());
    scenario
        .metadata
        .insert("vehicle_variant".to_owned(), entry.vehicle_variant.clone());
    scenario.metadata.insert(
        "expectation_tier".to_owned(),
        entry.expectation_tier.clone(),
    );
    scenario
        .metadata
        .insert("arc_point".to_owned(), arc.id.to_owned());
    scenario
        .metadata
        .insert("velocity_band".to_owned(), band.id.to_owned());
    scenario
        .metadata
        .insert("lane_id".to_owned(), lane_id.to_owned());

    scenario.world.gravity_mps2 = family_spec.gravity_mps2;

    let mut resolved_parameters = BTreeMap::new();
    resolved_parameters.insert("gravity_mps2".to_owned(), family_spec.gravity_mps2);
    resolved_parameters.insert("radius_nominal_m".to_owned(), family_spec.radius_nominal_m);
    resolved_parameters.insert("arc_angle_deg".to_owned(), arc.angle_deg);
    resolved_parameters.insert("mid_ttg_s".to_owned(), arc.nominal_ttg_s);

    for adjustment in &entry.adjustments {
        apply_numeric_adjustment(&mut scenario, adjustment)?;
        resolved_parameters.insert(adjustment.id.clone(), adjustment.value);
        scenario.metadata.insert(
            format!("resolved.{}", adjustment.id),
            format!("{:.6}", adjustment.value),
        );
    }

    let ttg_s = terminal_band_ttg(family_spec, arc, band.id);
    resolved_parameters.insert("ttg_s".to_owned(), ttg_s);

    let (side_label, side_sign) = resolved_side(arc.id, seed_spec.index);
    scenario
        .metadata
        .insert("resolved.side".to_owned(), side_label.to_owned());
    resolved_parameters.insert("side_sign".to_owned(), side_sign);

    let radial_jitter_m = seed_spec
        .radial_pct
        .map(|pct| (family_spec.radius_nominal_m * pct).clamp(-30.0, 30.0))
        .unwrap_or(0.0);
    let resolved_radius_m = family_spec.radius_nominal_m + radial_jitter_m;
    resolved_parameters.insert("radial_jitter_m".to_owned(), radial_jitter_m);
    resolved_parameters.insert("radius_m".to_owned(), resolved_radius_m);
    if let Some(radial_pct) = seed_spec.radial_pct {
        scenario
            .metadata
            .insert("resolved.seed_variation".to_owned(), "radial".to_owned());
        scenario
            .metadata
            .insert("resolved.radial_pct".to_owned(), format!("{radial_pct:.6}"));
    } else if let Some(speed_pct) = seed_spec.speed_pct {
        scenario
            .metadata
            .insert("resolved.seed_variation".to_owned(), "speed".to_owned());
        scenario
            .metadata
            .insert("resolved.speed_pct".to_owned(), format!("{speed_pct:.6}"));
    } else {
        scenario
            .metadata
            .insert("resolved.seed_variation".to_owned(), "none".to_owned());
    }

    let angle_rad = arc.angle_deg.to_radians();
    let x_m = if arc.id == "a00" {
        0.0
    } else {
        side_sign * resolved_radius_m * angle_rad.sin()
    };
    let y_m = resolved_radius_m * angle_rad.cos();
    resolved_parameters.insert("start_x_m".to_owned(), x_m);
    resolved_parameters.insert("start_y_m".to_owned(), y_m);

    let (mut vx_mps, mut vy_mps) =
        solve_ballistic_velocity(x_m, y_m, ttg_s, family_spec.gravity_mps2);
    let speed_scale = 1.0 + seed_spec.speed_pct.unwrap_or(0.0);
    vx_mps *= speed_scale;
    vy_mps *= speed_scale;
    resolved_parameters.insert("speed_scale".to_owned(), speed_scale);
    resolved_parameters.insert("start_vx_mps".to_owned(), vx_mps);
    resolved_parameters.insert("start_vy_mps".to_owned(), vy_mps);
    resolved_parameters.insert(
        "start_speed_mps".to_owned(),
        (vx_mps.powi(2) + vy_mps.powi(2)).sqrt(),
    );

    scenario.initial_state.position_m.x = x_m;
    scenario.initial_state.position_m.y = y_m;
    scenario.initial_state.velocity_mps.x = vx_mps;
    scenario.initial_state.velocity_mps.y = vy_mps;

    scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "resolved terminal matrix scenario '{}' {} {} seed {} failed validation",
                entry.id, arc.id, band.id, seed_spec.index
            )
        })?;

    let selector = SelectorAxes {
        mission: "terminal_guidance".to_owned(),
        arrival_family: family_spec.arrival_family.to_owned(),
        condition_set: entry.condition_set.clone(),
        vehicle_variant: entry.vehicle_variant.clone(),
        arc_point: arc.id.to_owned(),
        velocity_band: band.id.to_owned(),
        expectation_tier: Some(entry.expectation_tier.clone()),
    };

    Ok((scenario, resolved_parameters, selector))
}

fn terminal_band_ttg(
    family_spec: &TerminalArrivalFamilySpec,
    arc: &TerminalArcPointSpec,
    band_id: &str,
) -> f64 {
    match band_id {
        "mid" => arc.nominal_ttg_s,
        "low" => {
            let low = arc.nominal_ttg_s * family_spec.low_multiplier;
            if family_spec.clamp_low_to_descending {
                let y_m = family_spec.radius_nominal_m * arc.angle_deg.to_radians().cos();
                let t_flat_s = ((2.0 * y_m) / family_spec.gravity_mps2).sqrt();
                low.min(t_flat_s * 0.98)
            } else {
                low
            }
        }
        "high" => arc.nominal_ttg_s * family_spec.high_multiplier,
        _ => arc.nominal_ttg_s,
    }
}

fn resolved_side(arc_point: &str, seed: u64) -> (&'static str, f64) {
    if arc_point == "a00" {
        ("center", 0.0)
    } else if seed % 2 == 0 {
        ("left", -1.0)
    } else {
        ("right", 1.0)
    }
}

fn solve_ballistic_velocity(x_m: f64, y_m: f64, ttg_s: f64, gravity_mps2: f64) -> (f64, f64) {
    let vx_mps = -x_m / ttg_s;
    let vy_mps = ((0.5 * gravity_mps2 * ttg_s * ttg_s) - y_m) / ttg_s;
    (vx_mps, vy_mps)
}

fn family_entry_seeds(entry: &ScenarioFamilyEntry) -> Vec<u64> {
    if !entry.seeds.is_empty() {
        return entry.seeds.clone();
    }

    let range = entry
        .seed_range
        .as_ref()
        .expect("validated family entries always define seed source");
    (range.start..range.start.saturating_add(range.count)).collect()
}

fn resolve_family_scenario(
    entry: &ScenarioFamilyEntry,
    base_scenario: &ScenarioSpec,
    seed: u64,
) -> Result<(ScenarioSpec, BTreeMap<String, f64>)> {
    let mut scenario = base_scenario.clone();
    scenario.id = resolved_family_run_id(&entry.id, seed);
    scenario.name = format!("{} [{} seed {}]", base_scenario.name, entry.family, seed);
    scenario.description = format!(
        "{} (family {} seed {})",
        base_scenario.description, entry.family, seed
    );
    scenario.seed = seed;
    scenario.tags = merge_unique_tags(&base_scenario.tags, &entry.tags);
    scenario.metadata.extend(entry.metadata.clone());
    scenario
        .metadata
        .insert("family".to_owned(), entry.family.clone());
    scenario
        .metadata
        .insert("family_entry_id".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("resolved_seed".to_owned(), seed.to_string());

    let mut resolved_parameters = BTreeMap::new();
    for perturbation in &entry.perturbations {
        let sampled_value = sample_perturbation_value(entry, perturbation, seed);
        apply_numeric_perturbation(&mut scenario, perturbation, sampled_value)?;
        resolved_parameters.insert(perturbation.id.clone(), sampled_value);
        scenario.metadata.insert(
            format!("resolved.{}", perturbation.id),
            format!("{sampled_value:.6}"),
        );
    }

    scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "resolved family scenario '{}' seed {} failed validation",
                entry.family, seed
            )
        })?;

    Ok((scenario, resolved_parameters))
}

fn merge_unique_tags(base_tags: &[String], extra_tags: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(base_tags.len() + extra_tags.len());
    let mut seen = BTreeSet::new();
    for tag in base_tags.iter().chain(extra_tags.iter()) {
        if seen.insert(tag.clone()) {
            merged.push(tag.clone());
        }
    }
    merged
}

fn selector_axes_from_metadata(metadata: &BTreeMap<String, String>) -> SelectorAxes {
    SelectorAxes {
        mission: selector_value(metadata.get("mission"), "unspecified"),
        arrival_family: selector_value(metadata.get("arrival_family"), "unspecified"),
        condition_set: selector_value(metadata.get("condition_set"), "unspecified"),
        vehicle_variant: selector_value(metadata.get("vehicle_variant"), "unspecified"),
        arc_point: selector_value(metadata.get("arc_point"), "unspecified"),
        velocity_band: selector_value(metadata.get("velocity_band"), "unspecified"),
        expectation_tier: metadata
            .get("expectation_tier")
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
    }
}

fn selector_value(value: Option<&String>, fallback: &str) -> String {
    value
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_owned()
}

fn is_supported_numeric_path(path: &str) -> bool {
    matches!(
        path,
        "world.gravity_mps2"
            | "vehicle.dry_mass_kg"
            | "vehicle.initial_fuel_kg"
            | "vehicle.max_fuel_kg"
            | "vehicle.max_thrust_n"
            | "vehicle.max_fuel_burn_kgps"
            | "vehicle.max_rotation_rate_radps"
            | "initial_state.position_m.x"
            | "initial_state.position_m.y"
            | "initial_state.velocity_mps.x"
            | "initial_state.velocity_mps.y"
            | "initial_state.attitude_rad"
            | "initial_state.angular_rate_radps"
    )
}

fn is_supported_terminal_adjustment_path(path: &str) -> bool {
    matches!(
        path,
        "vehicle.dry_mass_kg"
            | "vehicle.initial_fuel_kg"
            | "vehicle.max_fuel_kg"
            | "vehicle.max_thrust_n"
            | "vehicle.max_fuel_burn_kgps"
            | "vehicle.max_rotation_rate_radps"
            | "initial_state.attitude_rad"
            | "initial_state.angular_rate_radps"
    )
}

fn apply_numeric_perturbation(
    scenario: &mut ScenarioSpec,
    perturbation: &NumericPerturbationSpec,
    sampled_value: f64,
) -> Result<()> {
    match perturbation.path.as_str() {
        "world.gravity_mps2" => Ok(apply_numeric_mode(
            &mut scenario.world.gravity_mps2,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.dry_mass_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.dry_mass_kg,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.initial_fuel_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.initial_fuel_kg,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.max_fuel_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_fuel_kg,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.max_thrust_n" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_thrust_n,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.max_fuel_burn_kgps" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_fuel_burn_kgps,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.max_rotation_rate_radps" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_rotation_rate_radps,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.position_m.x" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.position_m.x,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.position_m.y" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.position_m.y,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.velocity_mps.x" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.velocity_mps.x,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.velocity_mps.y" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.velocity_mps.y,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.attitude_rad" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.attitude_rad,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.angular_rate_radps" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.angular_rate_radps,
            perturbation.mode,
            sampled_value,
        )),
        _ => bail!(
            "unsupported numeric perturbation path '{}'",
            perturbation.path
        ),
    }
}

fn apply_numeric_adjustment(
    scenario: &mut ScenarioSpec,
    adjustment: &NumericAdjustmentSpec,
) -> Result<()> {
    match adjustment.path.as_str() {
        "vehicle.dry_mass_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.dry_mass_kg,
            adjustment.mode,
            adjustment.value,
        )),
        "vehicle.initial_fuel_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.initial_fuel_kg,
            adjustment.mode,
            adjustment.value,
        )),
        "vehicle.max_fuel_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_fuel_kg,
            adjustment.mode,
            adjustment.value,
        )),
        "vehicle.max_thrust_n" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_thrust_n,
            adjustment.mode,
            adjustment.value,
        )),
        "vehicle.max_fuel_burn_kgps" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_fuel_burn_kgps,
            adjustment.mode,
            adjustment.value,
        )),
        "vehicle.max_rotation_rate_radps" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_rotation_rate_radps,
            adjustment.mode,
            adjustment.value,
        )),
        "initial_state.attitude_rad" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.attitude_rad,
            adjustment.mode,
            adjustment.value,
        )),
        "initial_state.angular_rate_radps" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.angular_rate_radps,
            adjustment.mode,
            adjustment.value,
        )),
        _ => bail!("unsupported numeric adjustment path '{}'", adjustment.path),
    }
}

fn apply_numeric_mode(target: &mut f64, mode: NumericPerturbationMode, value: f64) {
    match mode {
        NumericPerturbationMode::Set => *target = value,
        NumericPerturbationMode::Offset => *target += value,
        NumericPerturbationMode::Scale => *target *= value,
    }
}

fn sample_perturbation_value(
    entry: &ScenarioFamilyEntry,
    perturbation: &NumericPerturbationSpec,
    seed: u64,
) -> f64 {
    let random_value = stable_unit_interval(
        seed,
        &format!("{}::{}::{}", entry.family, entry.id, perturbation.id),
    );
    let sampled = perturbation.min + ((perturbation.max - perturbation.min) * random_value);
    if let Some(step) = perturbation.quantize {
        (sampled / step).round() * step
    } else {
        sampled
    }
}

fn execute_resolved_runs(
    resolved_runs: &[ResolvedBatchRun],
    output_dir: Option<&Path>,
    workers_used: usize,
) -> Result<Vec<BatchRunRecord>> {
    if workers_used <= 1 {
        return resolved_runs
            .iter()
            .map(|run| execute_resolved_run(run, output_dir))
            .collect();
    }

    let pool = ThreadPoolBuilder::new()
        .num_threads(workers_used)
        .build()
        .context("failed to build pd-eval thread pool")?;

    let results = pool.install(|| {
        resolved_runs
            .par_iter()
            .map(|run| execute_resolved_run(run, output_dir))
            .collect::<Vec<_>>()
    });

    results.into_iter().collect()
}

fn execute_resolved_run(
    resolved_run: &ResolvedBatchRun,
    output_dir: Option<&Path>,
) -> Result<BatchRunRecord> {
    let ctx = RunContext::from_scenario(&resolved_run.scenario)
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "failed to build run context for resolved run {}",
                resolved_run.descriptor.run_id
            )
        })?;
    let artifacts = run_controller_spec(&ctx, &resolved_run.descriptor.controller_spec)
        .with_context(|| {
            format!(
                "failed to run controller for resolved run {}",
                resolved_run.descriptor.run_id
            )
        })?;

    let bundle_dir = output_dir.map(|root| root.join("runs").join(&resolved_run.descriptor.run_id));
    if let Some(bundle_dir) = bundle_dir.as_deref() {
        write_artifact_bundle(
            bundle_dir,
            &resolved_run.scenario,
            &resolved_run.descriptor.controller_spec,
            &artifacts,
        )?;
    }

    let review = derive_run_review_metrics(&resolved_run.scenario, &artifacts.run);

    Ok(BatchRunRecord {
        resolved: resolved_run.descriptor.clone(),
        manifest: artifacts.run.manifest,
        review,
        bundle_dir: bundle_dir.map(|path| path.to_string_lossy().into_owned()),
    })
}

fn summarize_records(records: &[BatchRunRecord]) -> BatchSummary {
    let total_runs = records.len();
    let success_runs = records
        .iter()
        .filter(|record| matches!(record.manifest.mission_outcome, MissionOutcome::Success))
        .count();
    let failure_runs = total_runs.saturating_sub(success_runs);
    let mean_sim_time_s = if total_runs == 0 {
        0.0
    } else {
        records
            .iter()
            .map(|record| record.manifest.sim_time_s)
            .sum::<f64>()
            / total_runs as f64
    };
    let max_sim_time_s = records
        .iter()
        .map(|record| record.manifest.sim_time_s)
        .fold(0.0_f64, f64::max);

    let mut mission_outcomes = BTreeMap::new();
    let mut physical_outcomes = BTreeMap::new();
    let mut end_reasons = BTreeMap::new();
    for record in records {
        *mission_outcomes
            .entry(enum_label(&record.manifest.mission_outcome))
            .or_insert(0) += 1;
        *physical_outcomes
            .entry(enum_label(&record.manifest.physical_outcome))
            .or_insert(0) += 1;
        *end_reasons
            .entry(enum_label(&record.manifest.end_reason))
            .or_insert(0) += 1;
    }

    let mut by_entry_groups = BTreeMap::<String, Vec<&BatchRunRecord>>::new();
    let mut by_family_groups = BTreeMap::<String, Vec<&BatchRunRecord>>::new();
    for record in records {
        by_entry_groups
            .entry(record.resolved.entry_id.clone())
            .or_default()
            .push(record);
        if let Some(family_id) = record.resolved.family_id.clone() {
            by_family_groups.entry(family_id).or_default().push(record);
        }
    }

    let by_entry = by_entry_groups
        .into_iter()
        .map(|(key, group)| summarize_group(&key, &group))
        .collect();
    let by_family = by_family_groups
        .into_iter()
        .map(|(key, group)| summarize_group(&key, &group))
        .collect();

    let mut failed_runs = records
        .iter()
        .filter(|record| !matches!(record.manifest.mission_outcome, MissionOutcome::Success))
        .map(run_pointer)
        .collect::<Vec<_>>();
    failed_runs.sort_by(|lhs, rhs| {
        lhs.entry_id
            .cmp(&rhs.entry_id)
            .then(lhs.scenario_seed.cmp(&rhs.scenario_seed))
            .then(lhs.run_id.cmp(&rhs.run_id))
    });

    let mut slowest_runs = records.iter().map(run_pointer).collect::<Vec<_>>();
    slowest_runs.sort_by(|lhs, rhs| {
        rhs.sim_time_s
            .partial_cmp(&lhs.sim_time_s)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(lhs.run_id.cmp(&rhs.run_id))
    });
    slowest_runs.truncate(10);

    let mut closest_failures = records
        .iter()
        .filter(|record| !matches!(record.manifest.mission_outcome, MissionOutcome::Success))
        .map(run_pointer)
        .collect::<Vec<_>>();
    closest_failures.sort_by(closest_failure_sort_key);
    closest_failures.truncate(10);

    let mut worst_failures = records
        .iter()
        .filter(|record| !matches!(record.manifest.mission_outcome, MissionOutcome::Success))
        .map(run_pointer)
        .collect::<Vec<_>>();
    worst_failures.sort_by(worst_failure_sort_key);
    worst_failures.truncate(10);

    let mut weakest_successes = records
        .iter()
        .filter(|record| matches!(record.manifest.mission_outcome, MissionOutcome::Success))
        .map(run_pointer)
        .collect::<Vec<_>>();
    weakest_successes.sort_by(weakest_success_sort_key);
    weakest_successes.truncate(10);

    let mut lowest_fuel_successes = records
        .iter()
        .filter(|record| matches!(record.manifest.mission_outcome, MissionOutcome::Success))
        .map(run_pointer)
        .collect::<Vec<_>>();
    lowest_fuel_successes.sort_by(lowest_fuel_success_sort_key);
    lowest_fuel_successes.truncate(10);

    BatchSummary {
        total_runs,
        success_runs,
        failure_runs,
        mean_sim_time_s,
        max_sim_time_s,
        mission_outcomes,
        physical_outcomes,
        end_reasons,
        by_entry,
        by_family,
        failed_runs,
        slowest_runs,
        closest_failures,
        worst_failures,
        weakest_successes,
        lowest_fuel_successes,
    }
}

fn compare_summary_delta(candidate: &BatchSummary, baseline: &BatchSummary) -> BatchSummaryDelta {
    BatchSummaryDelta {
        candidate_success_rate: success_rate(candidate.success_runs, candidate.total_runs),
        baseline_success_rate: success_rate(baseline.success_runs, baseline.total_runs),
        success_rate_delta: success_rate(candidate.success_runs, candidate.total_runs)
            - success_rate(baseline.success_runs, baseline.total_runs),
        candidate_success_runs: candidate.success_runs,
        baseline_success_runs: baseline.success_runs,
        success_runs_delta: candidate.success_runs as i64 - baseline.success_runs as i64,
        candidate_failure_runs: candidate.failure_runs,
        baseline_failure_runs: baseline.failure_runs,
        failure_runs_delta: candidate.failure_runs as i64 - baseline.failure_runs as i64,
        candidate_mean_sim_time_s: candidate.mean_sim_time_s,
        baseline_mean_sim_time_s: baseline.mean_sim_time_s,
        mean_sim_time_delta_s: candidate.mean_sim_time_s - baseline.mean_sim_time_s,
        candidate_max_sim_time_s: candidate.max_sim_time_s,
        baseline_max_sim_time_s: baseline.max_sim_time_s,
        max_sim_time_delta_s: candidate.max_sim_time_s - baseline.max_sim_time_s,
    }
}

fn compare_group_sets(
    candidate_groups: &[BatchGroupSummary],
    baseline_groups: &[BatchGroupSummary],
) -> Vec<BatchGroupComparison> {
    let candidate_map = candidate_groups
        .iter()
        .map(|group| (group.key.clone(), group))
        .collect::<BTreeMap<_, _>>();
    let baseline_map = baseline_groups
        .iter()
        .map(|group| (group.key.clone(), group))
        .collect::<BTreeMap<_, _>>();
    let keys = candidate_map
        .keys()
        .chain(baseline_map.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    keys.into_iter()
        .map(|key| {
            let candidate = candidate_map.get(&key).copied();
            let baseline = baseline_map.get(&key).copied();
            BatchGroupComparison {
                key,
                candidate_total_runs: candidate.map(|group| group.total_runs),
                baseline_total_runs: baseline.map(|group| group.total_runs),
                candidate_success_rate: candidate
                    .map(|group| success_rate(group.success_runs, group.total_runs)),
                baseline_success_rate: baseline
                    .map(|group| success_rate(group.success_runs, group.total_runs)),
                success_rate_delta: match (candidate, baseline) {
                    (Some(candidate), Some(baseline)) => Some(
                        success_rate(candidate.success_runs, candidate.total_runs)
                            - success_rate(baseline.success_runs, baseline.total_runs),
                    ),
                    _ => None,
                },
                candidate_failure_runs: candidate.map(|group| group.failure_runs),
                baseline_failure_runs: baseline.map(|group| group.failure_runs),
                failure_runs_delta: match (candidate, baseline) {
                    (Some(candidate), Some(baseline)) => {
                        Some(candidate.failure_runs as i64 - baseline.failure_runs as i64)
                    }
                    _ => None,
                },
                candidate_mean_sim_time_s: candidate.map(|group| group.mean_sim_time_s),
                baseline_mean_sim_time_s: baseline.map(|group| group.mean_sim_time_s),
                mean_sim_time_delta_s: match (candidate, baseline) {
                    (Some(candidate), Some(baseline)) => {
                        Some(candidate.mean_sim_time_s - baseline.mean_sim_time_s)
                    }
                    _ => None,
                },
                candidate_failed_seeds: candidate
                    .map(|group| group.failed_seeds.clone())
                    .unwrap_or_default(),
                baseline_failed_seeds: baseline
                    .map(|group| group.failed_seeds.clone())
                    .unwrap_or_default(),
                sample_run_ids: candidate
                    .map(|group| group.sample_run_ids.clone())
                    .or_else(|| baseline.map(|group| group.sample_run_ids.clone()))
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn compare_run_pair(
    candidate_record: &BatchRunRecord,
    baseline_record: &BatchRunRecord,
) -> Option<BatchRunComparison> {
    let candidate_success = matches!(
        candidate_record.manifest.mission_outcome,
        MissionOutcome::Success
    );
    let baseline_success = matches!(
        baseline_record.manifest.mission_outcome,
        MissionOutcome::Success
    );
    let candidate_mission_outcome = enum_label(&candidate_record.manifest.mission_outcome);
    let baseline_mission_outcome = enum_label(&baseline_record.manifest.mission_outcome);
    let candidate_end_reason = enum_label(&candidate_record.manifest.end_reason);
    let baseline_end_reason = enum_label(&baseline_record.manifest.end_reason);
    let sim_time_delta_s =
        candidate_record.manifest.sim_time_s - baseline_record.manifest.sim_time_s;
    let candidate_margin_ratio = summary_margin_ratio(&candidate_record.manifest.summary);
    let baseline_margin_ratio = summary_margin_ratio(&baseline_record.manifest.summary);
    let margin_ratio_delta = match (candidate_margin_ratio, baseline_margin_ratio) {
        (Some(candidate), Some(baseline)) => Some(candidate - baseline),
        _ => None,
    };
    let candidate_fuel_remaining_kg = candidate_record.manifest.summary.fuel_remaining_kg;
    let baseline_fuel_remaining_kg = baseline_record.manifest.summary.fuel_remaining_kg;

    let change_kind = if baseline_success && !candidate_success {
        BatchRunChangeKind::NewFailure
    } else if !baseline_success && candidate_success {
        BatchRunChangeKind::Recovered
    } else if candidate_mission_outcome != baseline_mission_outcome
        || candidate_end_reason != baseline_end_reason
        || sim_time_delta_s.abs() > 1e-9
    {
        BatchRunChangeKind::OutcomeChanged
    } else {
        return None;
    };

    Some(BatchRunComparison {
        run_id: candidate_record.resolved.run_id.clone(),
        entry_id: candidate_record.resolved.entry_id.clone(),
        family_id: candidate_record.resolved.family_id.clone(),
        selector: candidate_record.resolved.selector.clone(),
        lane_id: candidate_record.resolved.lane_id.clone(),
        change_kind,
        candidate_seed: candidate_record.manifest.scenario_seed,
        baseline_seed: baseline_record.manifest.scenario_seed,
        candidate_mission_outcome,
        baseline_mission_outcome,
        candidate_end_reason,
        baseline_end_reason,
        candidate_sim_time_s: candidate_record.manifest.sim_time_s,
        baseline_sim_time_s: baseline_record.manifest.sim_time_s,
        sim_time_delta_s,
        candidate_bundle_dir: candidate_record.bundle_dir.clone(),
        baseline_bundle_dir: baseline_record.bundle_dir.clone(),
        candidate_margin_ratio,
        baseline_margin_ratio,
        margin_ratio_delta,
        candidate_fuel_remaining_kg,
        baseline_fuel_remaining_kg,
        fuel_remaining_delta_kg: candidate_fuel_remaining_kg - baseline_fuel_remaining_kg,
    })
}

fn summarize_group(key: &str, records: &[&BatchRunRecord]) -> BatchGroupSummary {
    let total_runs = records.len();
    let success_runs = records
        .iter()
        .filter(|record| matches!(record.manifest.mission_outcome, MissionOutcome::Success))
        .count();
    let failure_runs = total_runs.saturating_sub(success_runs);
    let mean_sim_time_s = if total_runs == 0 {
        0.0
    } else {
        records
            .iter()
            .map(|record| record.manifest.sim_time_s)
            .sum::<f64>()
            / total_runs as f64
    };

    let mut mission_outcomes = BTreeMap::new();
    let mut end_reasons = BTreeMap::new();
    let mut failed_seeds = BTreeSet::new();
    let mut sample_run_ids = Vec::new();
    let mut success_fuel_remaining = Vec::new();
    let mut success_fuel_used_pct = Vec::new();
    let mut success_landing_offset_abs_m = Vec::new();
    let mut success_reference_gap_mean_m = Vec::new();
    let mut success_sim_time_s = Vec::new();
    let mut success_pointers = Vec::new();
    let mut failure_pointers = Vec::new();

    for record in records {
        *mission_outcomes
            .entry(enum_label(&record.manifest.mission_outcome))
            .or_insert(0) += 1;
        *end_reasons
            .entry(enum_label(&record.manifest.end_reason))
            .or_insert(0) += 1;
        let pointer = run_pointer(record);
        if !matches!(record.manifest.mission_outcome, MissionOutcome::Success) {
            failed_seeds.insert(record.resolved.resolved_seed);
            failure_pointers.push(pointer);
        } else {
            success_fuel_remaining.push(record.manifest.summary.fuel_remaining_kg);
            success_sim_time_s.push(record.manifest.sim_time_s);
            if let Some(value) = record.review.fuel_used_pct_of_max {
                success_fuel_used_pct.push(value);
            }
            if let Some(value) = record.review.landing_offset_abs_m {
                success_landing_offset_abs_m.push(value);
            }
            if let Some(value) = record.review.reference_gap_mean_m {
                success_reference_gap_mean_m.push(value);
            }
            success_pointers.push(pointer);
        }
        if sample_run_ids.len() < 5 {
            sample_run_ids.push(record.resolved.run_id.clone());
        }
    }
    let mean_success_fuel_remaining_kg = if success_fuel_remaining.is_empty() {
        None
    } else {
        Some(success_fuel_remaining.iter().sum::<f64>() / success_fuel_remaining.len() as f64)
    };
    success_pointers.sort_by(weakest_success_sort_key);
    failure_pointers.sort_by(closest_failure_sort_key);
    let closest_failure_run_id = failure_pointers
        .first()
        .map(|pointer| pointer.run_id.clone());
    failure_pointers.sort_by(worst_failure_sort_key);
    let worst_failure_run_id = failure_pointers
        .first()
        .map(|pointer| pointer.run_id.clone());

    BatchGroupSummary {
        key: key.to_owned(),
        total_runs,
        success_runs,
        failure_runs,
        mean_sim_time_s,
        sim_time_stats: metric_summary(&success_sim_time_s),
        mean_success_fuel_remaining_kg,
        fuel_used_pct_of_max: metric_summary(&success_fuel_used_pct),
        landing_offset_abs_m: metric_summary(&success_landing_offset_abs_m),
        reference_gap_mean_m: metric_summary(&success_reference_gap_mean_m),
        mission_outcomes,
        end_reasons,
        sample_run_ids,
        failed_seeds: failed_seeds.into_iter().collect(),
        weakest_success_run_id: success_pointers
            .first()
            .map(|pointer| pointer.run_id.clone()),
        closest_failure_run_id,
        worst_failure_run_id,
    }
}

fn run_pointer(record: &BatchRunRecord) -> BatchRunPointer {
    BatchRunPointer {
        run_id: record.resolved.run_id.clone(),
        entry_id: record.resolved.entry_id.clone(),
        family_id: record.resolved.family_id.clone(),
        selector: record.resolved.selector.clone(),
        lane_id: record.resolved.lane_id.clone(),
        scenario_id: record.manifest.scenario_id.clone(),
        scenario_seed: record.manifest.scenario_seed,
        controller_id: record.manifest.controller_id.clone(),
        mission_outcome: enum_label(&record.manifest.mission_outcome),
        end_reason: enum_label(&record.manifest.end_reason),
        sim_time_s: record.manifest.sim_time_s,
        bundle_dir: record.bundle_dir.clone(),
        margin_ratio: summary_margin_ratio(&record.manifest.summary),
        fuel_remaining_kg: record.manifest.summary.fuel_remaining_kg,
        review: record.review.clone(),
        summary: record.manifest.summary.clone(),
    }
}

fn metric_summary(values: &[f64]) -> Option<BatchMetricSummary> {
    if values.is_empty() {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let delta = *value - mean;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    Some(BatchMetricSummary {
        mean,
        stddev: Some(variance.sqrt()),
    })
}

fn derive_run_review_metrics(
    scenario: &ScenarioSpec,
    artifacts: &pd_core::RunArtifacts,
) -> BatchRunReviewMetrics {
    let fuel_used_pct_of_max = (scenario.vehicle.max_fuel_kg > 1e-9)
        .then(|| (artifacts.manifest.summary.fuel_used_kg / scenario.vehicle.max_fuel_kg) * 100.0);
    let landing_offset_abs_m = artifacts
        .manifest
        .summary
        .landing
        .as_ref()
        .map(|landing| landing.touchdown_center_offset_m.abs());
    let (reference_gap_mean_m, reference_gap_max_m) =
        reference_gap_metrics(scenario, &artifacts.samples)
            .map(|metrics| (Some(metrics.gap_mean_m), Some(metrics.gap_max_m)))
            .unwrap_or((None, None));

    BatchRunReviewMetrics {
        fuel_used_pct_of_max,
        landing_offset_abs_m,
        reference_gap_mean_m,
        reference_gap_max_m,
    }
}

#[derive(Clone, Copy, Debug)]
struct ReferenceGapMetrics {
    gap_mean_m: f64,
    gap_max_m: f64,
}

fn reference_gap_metrics(
    scenario: &ScenarioSpec,
    samples: &[SampleRecord],
) -> Option<ReferenceGapMetrics> {
    let target_pad = scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.id == scenario.mission.goal.target_pad_id())?;
    let actual_points = samples
        .iter()
        .map(|sample| {
            (
                sample.observation.position_m.x,
                sample.observation.position_m.y,
            )
        })
        .collect::<Vec<_>>();
    if actual_points.len() < 2 {
        return None;
    }
    let reference_points = idealized_reference_curve(
        scenario.initial_state.position_m.x,
        scenario.initial_state.position_m.y,
        target_pad.center_x_m,
        target_pad.surface_y_m,
        scenario.world.gravity_mps2,
    )?;
    if reference_points.len() < 2 {
        return None;
    }

    let (actual_cumulative, actual_length) = polyline_lengths(&actual_points)?;
    let mut gaps = Vec::with_capacity(actual_points.len());
    for point in &actual_points {
        let (_projection, distance) = project_point_to_polyline(*point, &reference_points)?;
        gaps.push(distance);
    }
    if gaps.len() < 2 {
        return None;
    }

    let gap_area = (0..(gaps.len() - 1))
        .map(|index| {
            0.5 * (gaps[index] + gaps[index + 1])
                * (actual_cumulative[index + 1] - actual_cumulative[index])
        })
        .sum::<f64>();
    Some(ReferenceGapMetrics {
        gap_mean_m: gap_area / actual_length,
        gap_max_m: gaps.iter().copied().fold(0.0_f64, f64::max),
    })
}

fn idealized_reference_curve(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    gravity_mps2: f64,
) -> Option<Vec<(f64, f64)>> {
    let apex_y = idealized_reference_apex_y(start_x, start_y, target_x, target_y, gravity_mps2);
    let (flight_time, vx_mps, vy_up_mps) =
        idealized_reference_kinematics(start_x, start_y, target_x, target_y, apex_y, gravity_mps2)?;
    let g = gravity_mps2.abs().max(1e-6);
    let point_count = ((18.0 + (flight_time * 10.0)).round() as usize).clamp(24, 84);
    let mut points = Vec::with_capacity(point_count);
    for index in 0..point_count {
        let t = if point_count <= 1 {
            0.0
        } else {
            (flight_time * index as f64) / (point_count as f64 - 1.0)
        };
        points.push((
            start_x + (vx_mps * t),
            start_y + (vy_up_mps * t) - (0.5 * g * t * t),
        ));
    }
    if let Some(last) = points.last_mut() {
        *last = (target_x, target_y);
    }
    Some(points)
}

fn idealized_reference_apex_y(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    gravity_mps2: f64,
) -> f64 {
    let dx = target_x - start_x;
    let dy = target_y - start_y;
    let base_peak = if target_y > start_y {
        start_y.max(target_y + 1.0)
    } else {
        start_y
    };
    if dx.abs() <= 1e-6 {
        return base_peak;
    }
    let meets_angle_floor = |peak_y: f64| {
        idealized_reference_impact_angle_deg(
            start_x,
            start_y,
            target_x,
            target_y,
            peak_y,
            gravity_mps2,
        )
        .is_some_and(|impact_angle| impact_angle >= 45.0)
    };
    if meets_angle_floor(base_peak) {
        return base_peak;
    }

    let mut low_peak = base_peak;
    let mut growth = 16.0_f64.max(0.25 * dx.abs().max(dy.abs()).max(1.0));
    let mut candidate_peak = base_peak;
    let mut high_peak = None;
    for _ in 0..16 {
        candidate_peak += growth;
        if meets_angle_floor(candidate_peak) {
            high_peak = Some(candidate_peak);
            break;
        }
        low_peak = candidate_peak;
        growth *= 2.0;
    }
    let Some(mut high_peak) = high_peak else {
        return candidate_peak;
    };
    for _ in 0..32 {
        let mid_peak = 0.5 * (low_peak + high_peak);
        if meets_angle_floor(mid_peak) {
            high_peak = mid_peak;
        } else {
            low_peak = mid_peak;
        }
    }
    high_peak
}

fn idealized_reference_impact_angle_deg(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    apex_y: f64,
    gravity_mps2: f64,
) -> Option<f64> {
    let (flight_time, vx_mps, vy_up_mps) =
        idealized_reference_kinematics(start_x, start_y, target_x, target_y, apex_y, gravity_mps2)?;
    let g = gravity_mps2.abs().max(1e-6);
    let vy_target = vy_up_mps - (g * flight_time);
    Some(((-vy_target).max(0.0)).atan2(vx_mps.abs()) * (180.0 / std::f64::consts::PI))
}

fn idealized_reference_kinematics(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    apex_y: f64,
    gravity_mps2: f64,
) -> Option<(f64, f64, f64)> {
    let g = gravity_mps2.abs().max(1e-6);
    let peak_y = start_y.max(apex_y);
    let vy_up_mps = (2.0 * g * (peak_y - start_y).max(0.0)).sqrt();
    let flight_time = ballistic_end_time(start_y, target_y, vy_up_mps, gravity_mps2)?;
    if !flight_time.is_finite() || flight_time <= 1e-6 {
        return None;
    }
    Some((flight_time, (target_x - start_x) / flight_time, vy_up_mps))
}

fn ballistic_end_time(start_y: f64, target_y: f64, vy_mps: f64, gravity_mps2: f64) -> Option<f64> {
    let g = gravity_mps2.abs().max(1e-6);
    let a = 0.5 * g;
    let b = -vy_mps;
    let c = target_y - start_y;
    let discriminant = (b * b) - (4.0 * a * c);
    if !discriminant.is_finite() || discriminant < 0.0 {
        return None;
    }
    let sqrt = discriminant.sqrt();
    let mut roots = [(-b - sqrt) / (2.0 * a), (-b + sqrt) / (2.0 * a)]
        .into_iter()
        .filter(|value| value.is_finite() && *value > 1e-6)
        .collect::<Vec<_>>();
    roots.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal));
    roots.pop()
}

fn polyline_lengths(points: &[(f64, f64)]) -> Option<(Vec<f64>, f64)> {
    if points.len() < 2 {
        return None;
    }
    let mut cumulative = Vec::with_capacity(points.len());
    cumulative.push(0.0);
    let mut length = 0.0;
    for index in 1..points.len() {
        let dx = points[index].0 - points[index - 1].0;
        let dy = points[index].1 - points[index - 1].1;
        length += (dx * dx + dy * dy).sqrt();
        cumulative.push(length);
    }
    (length > 1e-9).then_some((cumulative, length))
}

fn project_point_to_polyline(
    point: (f64, f64),
    polyline: &[(f64, f64)],
) -> Option<((f64, f64), f64)> {
    if polyline.is_empty() {
        return None;
    }
    let mut best_projection = polyline[0];
    let mut best_distance = point_distance(point, best_projection);
    for index in 1..polyline.len() {
        let (projection, distance) =
            project_point_to_segment(point, polyline[index - 1], polyline[index]);
        if distance < best_distance {
            best_projection = projection;
            best_distance = distance;
        }
    }
    Some((best_projection, best_distance))
}

fn project_point_to_segment(
    point: (f64, f64),
    start: (f64, f64),
    end: (f64, f64),
) -> ((f64, f64), f64) {
    let seg_dx = end.0 - start.0;
    let seg_dy = end.1 - start.1;
    let seg_len_sq = (seg_dx * seg_dx) + (seg_dy * seg_dy);
    if seg_len_sq <= 1e-12 {
        return (start, point_distance(point, start));
    }
    let mix = (((point.0 - start.0) * seg_dx) + ((point.1 - start.1) * seg_dy)) / seg_len_sq;
    let clamped_mix = mix.clamp(0.0, 1.0);
    let projection = (
        start.0 + (seg_dx * clamped_mix),
        start.1 + (seg_dy * clamped_mix),
    );
    (projection, point_distance(point, projection))
}

fn point_distance(lhs: (f64, f64), rhs: (f64, f64)) -> f64 {
    let dx = lhs.0 - rhs.0;
    let dy = lhs.1 - rhs.1;
    (dx * dx + dy * dy).sqrt()
}

fn summary_margin_ratio(summary: &RunSummary) -> Option<f64> {
    summary.envelope_margin_ratio
}

fn success_rate(success_runs: usize, total_runs: usize) -> f64 {
    if total_runs == 0 {
        0.0
    } else {
        success_runs as f64 / total_runs as f64
    }
}

fn run_pointer_sort_key(lhs: &BatchRunPointer, rhs: &BatchRunPointer) -> std::cmp::Ordering {
    lhs.entry_id
        .cmp(&rhs.entry_id)
        .then(lhs.scenario_seed.cmp(&rhs.scenario_seed))
        .then(lhs.run_id.cmp(&rhs.run_id))
}

fn closest_failure_sort_key(lhs: &BatchRunPointer, rhs: &BatchRunPointer) -> std::cmp::Ordering {
    rhs.margin_ratio
        .unwrap_or(f64::NEG_INFINITY)
        .partial_cmp(&lhs.margin_ratio.unwrap_or(f64::NEG_INFINITY))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(run_pointer_sort_key(lhs, rhs))
}

fn worst_failure_sort_key(lhs: &BatchRunPointer, rhs: &BatchRunPointer) -> std::cmp::Ordering {
    lhs.margin_ratio
        .unwrap_or(f64::INFINITY)
        .partial_cmp(&rhs.margin_ratio.unwrap_or(f64::INFINITY))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(run_pointer_sort_key(lhs, rhs))
}

fn weakest_success_sort_key(lhs: &BatchRunPointer, rhs: &BatchRunPointer) -> std::cmp::Ordering {
    lhs.margin_ratio
        .unwrap_or(f64::INFINITY)
        .partial_cmp(&rhs.margin_ratio.unwrap_or(f64::INFINITY))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(run_pointer_sort_key(lhs, rhs))
}

fn lowest_fuel_success_sort_key(
    lhs: &BatchRunPointer,
    rhs: &BatchRunPointer,
) -> std::cmp::Ordering {
    lhs.fuel_remaining_kg
        .partial_cmp(&rhs.fuel_remaining_kg)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(run_pointer_sort_key(lhs, rhs))
}

fn run_comparison_sort_key(
    lhs: &BatchRunComparison,
    rhs: &BatchRunComparison,
) -> std::cmp::Ordering {
    lhs.candidate_margin_ratio
        .unwrap_or(f64::INFINITY)
        .partial_cmp(&rhs.candidate_margin_ratio.unwrap_or(f64::INFINITY))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(
            lhs.margin_ratio_delta
                .unwrap_or(f64::INFINITY)
                .partial_cmp(&rhs.margin_ratio_delta.unwrap_or(f64::INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal),
        )
        .then(
            lhs.entry_id
                .cmp(&rhs.entry_id)
                .then(lhs.candidate_seed.cmp(&rhs.candidate_seed))
                .then(lhs.run_id.cmp(&rhs.run_id)),
        )
}

fn load_scenario(path: &Path) -> Result<ScenarioSpec> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario file {}", path.display()))?;
    let scenario: ScenarioSpec = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse scenario json {}", path.display()))?;
    scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("scenario '{}' failed validation", path.display()))?;
    Ok(scenario)
}

fn load_controller_spec(
    base_dir: &Path,
    controller_name: &str,
    controller_config_path: Option<&str>,
) -> Result<ControllerSpec> {
    if let Some(path) = controller_config_path {
        let full_path = base_dir.join(path);
        let raw = fs::read_to_string(&full_path).with_context(|| {
            format!(
                "failed to read controller config file {}",
                full_path.display()
            )
        })?;
        return serde_json::from_str(&raw).with_context(|| {
            format!(
                "failed to parse controller config json {}",
                full_path.display()
            )
        });
    }

    built_in_controller_spec(controller_name)
        .ok_or_else(|| anyhow!("unknown controller '{}'", controller_name))
}

fn write_artifact_bundle(
    path: &Path,
    scenario: &ScenarioSpec,
    controller_spec: &ControllerSpec,
    artifacts: &ControlledRunArtifacts,
) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create artifact bundle dir {}", path.display()))?;
    write_json(&path.join("scenario.json"), scenario)?;
    write_json(&path.join("controller.json"), controller_spec)?;
    write_json(
        &path.join("controller_updates.json"),
        &artifacts.controller_updates,
    )?;
    write_json(&path.join("performance.json"), &artifacts.performance)?;
    write_json(&path.join("manifest.json"), &artifacts.run.manifest)?;
    write_json(&path.join("actions.json"), &artifacts.run.actions)?;
    write_json(&path.join("events.json"), &artifacts.run.events)?;
    write_json(&path.join("samples.json"), &artifacts.run.samples)?;
    pd_report::write_run_report(
        &path.join("report.html"),
        scenario,
        Some(controller_spec),
        &artifacts.run.manifest,
        &artifacts.run.events,
        &artifacts.run.samples,
        &artifacts.controller_updates,
        Some(&artifacts.performance),
    )?;
    pd_report::write_run_preview_svg(
        &path.join("preview.svg"),
        scenario,
        &artifacts.run.manifest,
        &artifacts.run.samples,
    )?;
    Ok(())
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    let raw = serde_json::to_string_pretty(value)?;
    fs::write(path, raw)
        .with_context(|| format!("failed to write json file {}", path.display()))?;
    Ok(())
}

fn enum_label<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"unknown\"".to_owned())
        .trim_matches('"')
        .to_owned()
}

fn effective_worker_count(requested_workers: usize, total_runs: usize) -> usize {
    if total_runs == 0 {
        return 1;
    }
    requested_workers.max(1).min(total_runs)
}

fn stable_digest<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    Ok(format!("{:012x}", fnv1a64(&bytes)))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn stable_unit_interval(seed: u64, salt: &str) -> f64 {
    let mixed = splitmix64(seed ^ fnv1a64(salt.as_bytes()));
    let mantissa = mixed >> 11;
    (mantissa as f64) / ((1_u64 << 53) as f64)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = value;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn sanitize_token(token: &str) -> String {
    let mut out = String::with_capacity(token.len());
    let mut last_was_sep = false;
    for ch in token.chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            out.push(normalized);
            last_was_sep = false;
        } else if !last_was_sep {
            out.push('_');
            last_was_sep = true;
        }
    }
    out.trim_matches('_').to_owned()
}

fn resolved_family_run_id(entry_id: &str, seed: u64) -> String {
    sanitize_token(&format!("{entry_id}__seed_{seed:04}"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pd-eval crate should live under repo root")
        .to_path_buf()
}

fn maybe_update_latest_link(target_dir: &Path) -> Result<()> {
    let repo_root = repo_root();
    let outputs_root = repo_root.join("outputs");
    let resolved_target_dir = if target_dir.is_absolute() {
        target_dir.to_path_buf()
    } else {
        repo_root.join(target_dir)
    };

    if !resolved_target_dir.starts_with(&outputs_root) {
        return Ok(());
    }

    let Some(parent_dir) = resolved_target_dir.parent() else {
        return Ok(());
    };
    let Some(target_name) = resolved_target_dir.file_name() else {
        return Ok(());
    };

    let latest_path = parent_dir.join("latest");
    if let Ok(metadata) = fs::symlink_metadata(&latest_path) {
        if metadata.file_type().is_symlink() || metadata.is_file() {
            fs::remove_file(&latest_path).with_context(|| {
                format!(
                    "failed to remove existing latest link {}",
                    latest_path.display()
                )
            })?;
        } else {
            eprintln!(
                "skipping latest link update because '{}' exists and is not a symlink",
                latest_path.display()
            );
            return Ok(());
        }
    }

    platform_fs::symlink(PathBuf::from(target_name), &latest_path).with_context(|| {
        format!(
            "failed to create latest link {} -> {}",
            latest_path.display(),
            target_name.to_string_lossy()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn fixtures_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures")
    }

    #[test]
    fn run_pack_aggregates_nominal_suite() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "unit_pack".to_owned(),
            name: "Unit pack".to_owned(),
            description: "unit pack".to_owned(),
            entries: vec![
                ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                    id: "landing_success".to_owned(),
                    scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    controller: "baseline".to_owned(),
                    controller_config: None,
                    metadata: BTreeMap::new(),
                }),
                ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                    id: "checkpoint_success".to_owned(),
                    scenario: "scenarios/timed_checkpoint_idle.json".to_owned(),
                    controller: "idle".to_owned(),
                    controller_config: None,
                    metadata: BTreeMap::new(),
                }),
            ],
        };

        let report = run_pack(&pack, &base_dir, None).unwrap();

        assert_eq!(report.total_runs, 2);
        assert_eq!(report.summary.success_runs, 2);
        assert_eq!(
            report.summary.mission_outcomes.get("success").copied(),
            Some(2)
        );
        assert_eq!(report.identity.schema_version, BATCH_REPORT_SCHEMA_VERSION);
    }

    #[test]
    fn family_entry_expands_deterministically_across_workers() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "family_pack".to_owned(),
            name: "Family pack".to_owned(),
            description: "family pack".to_owned(),
            entries: vec![
                ScenarioPackEntry::Family(ScenarioFamilyEntry {
                    id: "terminal_sweep".to_owned(),
                    family: "terminal_nominal".to_owned(),
                    base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    controller: "baseline".to_owned(),
                    controller_config: None,
                    seeds: vec![0, 1, 2],
                    seed_range: None,
                    perturbations: vec![
                        NumericPerturbationSpec {
                            id: "spawn_dx".to_owned(),
                            path: "initial_state.position_m.x".to_owned(),
                            mode: NumericPerturbationMode::Offset,
                            min: -12.0,
                            max: 12.0,
                            quantize: Some(0.5),
                        },
                        NumericPerturbationSpec {
                            id: "spawn_vy".to_owned(),
                            path: "initial_state.velocity_mps.y".to_owned(),
                            mode: NumericPerturbationMode::Offset,
                            min: -2.0,
                            max: 2.0,
                            quantize: Some(0.25),
                        },
                    ],
                    tags: vec!["sweep".to_owned()],
                    metadata: BTreeMap::from([
                        ("difficulty".to_owned(), "sweep".to_owned()),
                        ("mission".to_owned(), "terminal_guidance".to_owned()),
                        (
                            "arrival_family".to_owned(),
                            "seeded_terminal_arrival_v0".to_owned(),
                        ),
                        ("condition_set".to_owned(), "clean".to_owned()),
                        ("vehicle_variant".to_owned(), "nominal".to_owned()),
                        ("expectation_tier".to_owned(), "core".to_owned()),
                    ]),
                }),
                ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                    id: "checkpoint".to_owned(),
                    scenario: "scenarios/timed_checkpoint_idle.json".to_owned(),
                    controller: "idle".to_owned(),
                    controller_config: None,
                    metadata: BTreeMap::new(),
                }),
            ],
        };

        let sequential = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
        let parallel = run_pack_with_workers(&pack, &base_dir, None, 2).unwrap();

        assert_eq!(sequential.total_runs, 4);
        assert_eq!(
            sequential.identity.resolved_run_digest,
            parallel.identity.resolved_run_digest
        );
        assert_eq!(
            sequential
                .records
                .iter()
                .map(|record| record.resolved.run_id.clone())
                .collect::<Vec<_>>(),
            parallel
                .records
                .iter()
                .map(|record| record.resolved.run_id.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            sequential
                .summary
                .by_entry
                .iter()
                .find(|group| group.key == "terminal_sweep")
                .map(|group| group.total_runs),
            Some(3)
        );
        assert!(sequential.records.iter().any(|record| {
            record.resolved.source_kind == ResolvedRunSourceKind::FamilySweep
                && !record.resolved.resolved_parameters.is_empty()
        }));
        let family_record = sequential
            .records
            .iter()
            .find(|record| record.resolved.entry_id == "terminal_sweep")
            .expect("family record present");
        assert_eq!(family_record.resolved.selector.mission, "terminal_guidance");
        assert_eq!(
            family_record.resolved.selector.arrival_family,
            "seeded_terminal_arrival_v0"
        );
        assert_eq!(family_record.resolved.selector.condition_set, "clean");
        assert_eq!(family_record.resolved.selector.vehicle_variant, "nominal");
        assert_eq!(
            family_record.resolved.selector.expectation_tier.as_deref(),
            Some("core")
        );
        assert_eq!(family_record.resolved.lane_id, "baseline");
        let pointer = run_pointer(family_record);
        assert_eq!(pointer.selector.vehicle_variant, "nominal");
        assert_eq!(pointer.lane_id, "baseline");
    }

    #[test]
    fn compare_reports_flags_regressions_on_shared_runs() {
        let base_dir = fixtures_root();
        let baseline_pack = ScenarioPackSpec {
            id: "compare_baseline".to_owned(),
            name: "Compare baseline".to_owned(),
            description: "compare baseline".to_owned(),
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "landing_case".to_owned(),
                scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                controller: "baseline".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            })],
        };
        let candidate_pack = ScenarioPackSpec {
            id: "compare_candidate".to_owned(),
            name: "Compare candidate".to_owned(),
            description: "compare candidate".to_owned(),
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "landing_case".to_owned(),
                scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                controller: "idle".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            })],
        };

        let baseline = run_pack(&baseline_pack, &base_dir, None).unwrap();
        let candidate = run_pack(&candidate_pack, &base_dir, None).unwrap();
        let comparison = compare_batch_reports(&candidate, &baseline);

        assert_eq!(comparison.basis.shared_runs, 1);
        assert_eq!(comparison.regressions.len(), 1);
        assert!(comparison.improvements.is_empty());
        assert_eq!(comparison.summary.failure_runs_delta, 1);
        assert_eq!(comparison.regressions[0].run_id, "landing_case");
        assert_eq!(
            comparison.regressions[0].baseline_mission_outcome,
            "success"
        );
        assert_eq!(
            comparison.regressions[0].candidate_mission_outcome,
            "failed_crash"
        );
    }

    #[test]
    fn concrete_entry_metadata_overrides_selector_axes() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "concrete_metadata_override".to_owned(),
            name: "Concrete metadata override".to_owned(),
            description: "selector metadata override".to_owned(),
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "checkpoint".to_owned(),
                scenario: "scenarios/timed_checkpoint_idle.json".to_owned(),
                controller: "idle".to_owned(),
                controller_config: None,
                metadata: BTreeMap::from([
                    ("mission".to_owned(), "terminal_guidance".to_owned()),
                    (
                        "arrival_family".to_owned(),
                        "override_arrival_family".to_owned(),
                    ),
                    ("condition_set".to_owned(), "stress".to_owned()),
                    ("vehicle_variant".to_owned(), "heavy_cargo".to_owned()),
                    ("expectation_tier".to_owned(), "frontier".to_owned()),
                ]),
            })],
        };

        let report = run_pack(&pack, &base_dir, None).unwrap();
        let record = report
            .records
            .iter()
            .find(|record| record.resolved.entry_id == "checkpoint")
            .expect("concrete record present");
        assert_eq!(record.resolved.selector.mission, "terminal_guidance");
        assert_eq!(
            record.resolved.selector.arrival_family,
            "override_arrival_family"
        );
        assert_eq!(record.resolved.selector.condition_set, "stress");
        assert_eq!(record.resolved.selector.vehicle_variant, "heavy_cargo");
        assert_eq!(
            record.resolved.selector.expectation_tier.as_deref(),
            Some("frontier")
        );
    }

    #[test]
    fn terminal_matrix_entry_expands_documented_smoke_axes() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_smoke".to_owned(),
            name: "Terminal matrix smoke".to_owned(),
            description: "terminal matrix smoke".to_owned(),
            entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_clean_nominal".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "staged".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "clean".to_owned(),
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                adjustments: Vec::new(),
                tags: vec!["terminal".to_owned(), "smoke".to_owned()],
                metadata: BTreeMap::from([("difficulty".to_owned(), "nominal".to_owned())]),
            })],
        };

        let report = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
        assert_eq!(report.total_runs, 7 * 3 * 3);
        assert_eq!(report.summary.by_entry[0].total_runs, 7 * 3 * 3);

        let record = report
            .records
            .iter()
            .find(|record| {
                record.resolved.selector.arc_point == "a80"
                    && record.resolved.selector.velocity_band == "high"
                    && record.resolved.resolved_seed == 6
            })
            .expect("matrix record present");

        assert_eq!(
            record.resolved.source_kind,
            ResolvedRunSourceKind::TerminalMatrix
        );
        assert_eq!(record.resolved.selector.mission, "terminal_guidance");
        assert_eq!(
            record.resolved.selector.arrival_family,
            "half_arc_terminal_v1"
        );
        assert_eq!(record.resolved.selector.condition_set, "clean");
        assert_eq!(record.resolved.selector.vehicle_variant, "nominal");
        assert_eq!(record.resolved.selector.arc_point, "a80");
        assert_eq!(record.resolved.selector.velocity_band, "high");
        assert_eq!(record.resolved.lane_id, "current");
        assert!(
            record
                .resolved
                .resolved_scenario_name
                .contains("a80 high seed 6 current")
        );
        assert_eq!(
            record.resolved.selector.expectation_tier.as_deref(),
            Some("core")
        );
        assert!(
            (record.manifest.summary.max_speed_mps > 0.0)
                && (record.manifest.summary.fuel_remaining_kg >= 0.0)
        );
        assert_eq!(record.manifest.scenario_seed, 6);
        assert_eq!(
            record
                .resolved
                .resolved_parameters
                .get("gravity_mps2")
                .copied(),
            Some(9.81)
        );
    }

    #[test]
    fn terminal_matrix_entry_rejects_overwritten_adjustment_paths() {
        for path in ["world.gravity_mps2", "initial_state.position_m.x"] {
            let pack = ScenarioPackSpec {
                id: "terminal_matrix_invalid_adjustment".to_owned(),
                name: "Terminal matrix invalid adjustment".to_owned(),
                description: "terminal matrix invalid adjustment".to_owned(),
                entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                    id: "terminal_guidance_clean_nominal".to_owned(),
                    terminal_matrix: "half_arc_terminal_v1".to_owned(),
                    base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    lanes: vec![TerminalMatrixLaneSpec {
                        id: "current".to_owned(),
                        controller: "staged".to_owned(),
                        controller_config: None,
                    }],
                    seed_tier: TerminalSeedTier::Smoke,
                    condition_set: "clean".to_owned(),
                    vehicle_variant: "nominal".to_owned(),
                    expectation_tier: "core".to_owned(),
                    adjustments: vec![NumericAdjustmentSpec {
                        id: "invalid".to_owned(),
                        path: path.to_owned(),
                        mode: NumericPerturbationMode::Offset,
                        value: 1.0,
                    }],
                    tags: Vec::new(),
                    metadata: BTreeMap::new(),
                })],
            };

            let err = validate_pack(&pack).expect_err("path should be rejected");
            assert!(err.to_string().contains("uses unsupported path"), "{err}");
        }
    }
}
