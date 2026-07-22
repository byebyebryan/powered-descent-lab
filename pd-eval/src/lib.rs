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
    RunContext, RunManifest, RunSummary, SampleRecord, ScenarioSpec, TerrainDefinition,
    TransferRouteSpec, TransferWaypointSpec, Vec2, VehicleSpec, WaypointHandoffKinematics,
};
use rayon::{ThreadPoolBuilder, prelude::*};
use serde::{Deserialize, Serialize};

pub mod report;
pub mod report_catalog;

#[cfg(unix)]
use std::os::unix::fs as platform_fs;
#[cfg(windows)]
use std::os::windows::fs as platform_fs;

pub const BATCH_REPORT_SCHEMA_VERSION: u32 = 34;

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
    pd_report::write_run_report_with_context(
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
