use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::BufReader,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result};
use pd_core::{ScenarioSpec, Vec2};
use pd_report::{AggregatePreviewSeries, build_multi_run_trajectory_preview_svg, site::ReportSite};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    BatchCacheInfo, BatchCacheStatus, BatchCompareResolutionStatus, BatchCompareSource,
    BatchComparison, BatchRegressionPolicyRuleResult, BatchRegressionPolicyStatus, BatchReport,
    BatchRunComparison, BatchRunPointer, compare_batch_reports,
};

const TRANSFER_TERMINAL_REBOUND_RISK_GAIN_M: f64 = 5.0;

mod review_tree;
use review_tree::*;

mod diagnostics;
use diagnostics::*;

mod comparison;
use comparison::*;

mod overview;
use overview::*;

#[derive(Default)]
pub(crate) struct BatchReportRenderCache {
    lane_previews: RefCell<BTreeMap<Vec<PathBuf>, Option<String>>>,
}

pub fn write_batch_report_artifacts(
    output_dir: &Path,
    candidate: &BatchReport,
    baseline: Option<(&Path, &BatchReport)>,
) -> Result<Option<BatchComparison>> {
    write_batch_report_artifacts_with_cache(
        output_dir,
        candidate,
        baseline,
        &BatchReportRenderCache::default(),
    )
}

pub(crate) fn write_batch_report_artifacts_with_cache(
    output_dir: &Path,
    candidate: &BatchReport,
    baseline: Option<(&Path, &BatchReport)>,
    render_cache: &BatchReportRenderCache,
) -> Result<Option<BatchComparison>> {
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create batch report output directory {}",
            output_dir.display()
        )
    })?;

    let comparison = baseline.map(|(_, report)| compare_batch_reports(candidate, report));
    if let Some(comparison) = comparison.as_ref() {
        write_json(&output_dir.join("compare.json"), comparison)?;
    } else {
        let compare_path = output_dir.join("compare.json");
        if compare_path.exists() {
            fs::remove_file(&compare_path).with_context(|| {
                format!(
                    "failed to remove stale compare artifact {}",
                    compare_path.display()
                )
            })?;
        }
    }

    let html = render_batch_report_with_cache(
        output_dir,
        candidate,
        baseline,
        comparison.as_ref(),
        render_cache,
    );
    fs::write(output_dir.join("report.html"), &html).with_context(|| {
        format!(
            "failed to write batch report html {}",
            output_dir.join("report.html").display()
        )
    })?;

    if let Some(site_output) = report_site_output_for_batch(output_dir) {
        if let Some(parent) = site_output.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create batch report site directory {}",
                    parent.display()
                )
            })?;
        }
        let site_dir = site_output
            .parent()
            .expect("report site output should have parent directory");
        let stable_output_dir = resolve_repo_relative(output_dir);
        let base_href = directory_href(site_dir, &stable_output_dir);
        let site_html = html_with_base_href(&html, &base_href);
        fs::write(&site_output, site_html).with_context(|| {
            format!(
                "failed to write batch report site html {}",
                site_output.display()
            )
        })?;
        report_site().update_indexes_for_file(&site_output)?;
        crate::report_catalog::write_report_catalog(&crate::repo_root())?;
    }

    Ok(comparison)
}

fn report_site() -> ReportSite {
    ReportSite::new(crate::repo_root())
        .with_fixture_pack_dir(crate::repo_root().join("fixtures/packs"))
}

#[cfg(test)]
fn render_batch_report(
    output_dir: &Path,
    candidate: &BatchReport,
    baseline: Option<(&Path, &BatchReport)>,
    comparison: Option<&BatchComparison>,
) -> String {
    render_batch_report_with_cache(
        output_dir,
        candidate,
        baseline,
        comparison,
        &BatchReportRenderCache::default(),
    )
}

fn render_batch_report_with_cache(
    output_dir: &Path,
    candidate: &BatchReport,
    baseline: Option<(&Path, &BatchReport)>,
    comparison: Option<&BatchComparison>,
    render_cache: &BatchReportRenderCache,
) -> String {
    let output_dir = resolve_repo_relative(output_dir);
    let baseline_report_href = baseline
        .map(|(dir, _)| resolve_repo_relative(dir))
        .map(|dir| relative_href(&output_dir, &dir.join("report.html")));
    let candidate_record_links = candidate_record_map(candidate);
    let baseline_record_map = baseline
        .map(|(_, report)| candidate_record_map(report))
        .unwrap_or_default();

    let title = if comparison.is_some() {
        format!("{} compare report", candidate.pack_name)
    } else {
        format!("{} batch report", candidate.pack_name)
    };
    let has_compare_view = comparison.is_some();
    let chips_html = format!(
        r#"<span class="chip"><strong>pack</strong> <span class="mono">{}</span></span><span class="chip"><strong>runs</strong> {}</span><span class="chip"><strong>workers</strong> {}/{}</span>{}<span class="chip"><strong>spec</strong> <span class="mono">{}</span></span><span class="chip"><strong>resolved</strong> <span class="mono">{}</span></span>"#,
        escape_html(&candidate.pack_id),
        candidate.total_runs,
        candidate.workers_used,
        candidate.workers_requested,
        render_wall_clock_chip(candidate),
        escape_html(&candidate.identity.pack_spec_digest),
        escape_html(&candidate.identity.resolved_run_digest),
    );
    let actions_html = format!(
        r#"<a href="summary.json">summary.json</a><a href="resolved_runs.json">resolved_runs.json</a>{compare_json_link}{baseline_report_link}"#,
        compare_json_link = if comparison.is_some() {
            r#"<a href="compare.json">compare.json</a>"#.to_owned()
        } else {
            String::new()
        },
        baseline_report_link = baseline_report_href
            .as_ref()
            .map(|href| format!(r#"<a href="{}">baseline report</a>"#, escape_html(href)))
            .unwrap_or_default(),
    );
    let context_html =
        render_context_table(candidate, baseline.map(|(_, report)| report), comparison);
    let overview_html = render_overview_table(
        candidate,
        baseline.map(|(_, report)| report),
        comparison,
        render_view_controls(has_compare_view).as_str(),
    );
    let planner_html = render_planner_diagnostics(candidate, baseline.map(|(_, report)| report));
    let coverage_html =
        render_coverage_matrix(candidate, baseline.map(|(_, report)| report), comparison);
    let diagnostics_html = render_guidance_diagnostics(
        render_waypoint_sequence_section(candidate),
        render_waypoint_triage_section(candidate, &output_dir, &candidate_record_links),
        render_transfer_handoff_triage_section(candidate, &output_dir, &candidate_record_links),
        render_transfer_shape_triage_section(
            candidate,
            baseline.map(|(_, report)| report),
            comparison,
            &output_dir,
            &candidate_record_links,
        ),
    );
    let tree_controls = render_tree_controls(comparison.is_some());
    let tree = render_review_tree(
        candidate,
        baseline.map(|(_, report)| report),
        comparison,
        &output_dir,
        render_cache,
    );
    let review_tree_html = pd_report::batch::render_review_tree_section(&tree_controls, "", &tree);
    let comparison_html = comparison
        .map(|comparison| {
            render_comparison_sections(
                &output_dir,
                comparison,
                &candidate_record_links,
                &baseline_record_map,
            )
        })
        .unwrap_or_default();

    pd_report::batch::render_batch_page(pd_report::batch::BatchPage {
        title: &title,
        subtitle: &batch_report_subtitle(candidate, baseline.map(|(_, report)| report), comparison),
        chips_html: &chips_html,
        actions_html: &actions_html,
        before_hero_html: "",
        after_hero_html: "",
        overview_html: &overview_html,
        planner_html: &planner_html,
        coverage_html: &coverage_html,
        context_html: &context_html,
        diagnostics_html: &diagnostics_html,
        review_tree_html: &review_tree_html,
        comparison_html: &comparison_html,
        appendix_html: "",
        body_class: "",
        tree: pd_report::batch::BatchTreeOptions {
            max_depth: 6,
            depth_by_kind: &[
                ("mission", 0),
                ("arrival", 1),
                ("condition", 2),
                ("arc", 3),
                ("route", 3),
                ("band", 4),
                ("radius", 4),
                ("vehicle", 5),
                ("lane", 6),
            ],
            leaf_selector: "tr.seed-row",
            default_expansion: 6,
        },
        coverage_tree_jump: true,
    })
}

fn render_planner_diagnostics(candidate: &BatchReport, baseline: Option<&BatchReport>) -> String {
    let baseline_by_case = baseline
        .into_iter()
        .flat_map(|report| report.records.iter())
        .filter(|record| record.resolved.route_plan.is_some())
        .map(|record| {
            (
                record
                    .resolved
                    .physical_case_id
                    .as_deref()
                    .unwrap_or(record.resolved.run_id.as_str())
                    .to_owned(),
                record,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut rows = String::new();
    for record in candidate
        .records
        .iter()
        .filter(|record| record.resolved.route_plan.is_some())
    {
        let key = record
            .resolved
            .physical_case_id
            .as_deref()
            .unwrap_or(record.resolved.run_id.as_str());
        let changed = baseline_by_case
            .get(key)
            .and_then(|previous| {
                Some(
                    previous.resolved.route_plan.as_ref()?.plan_digest
                        != record.resolved.route_plan.as_ref()?.plan_digest,
                )
            })
            .unwrap_or(false);
        rows.push_str(&render_planner_row(record, "candidate", changed));
    }
    if rows.is_empty() {
        return String::new();
    }
    format!(
        r#"<section class="planner-diagnostics card" data-planner-diagnostics>
  <div class="section-head"><h2>Planner provenance and diagnostics</h2><span class="section-note">Generated routes retain their immutable plan evidence; authored routes remain outside this section.</span></div>
  <div class="table-wrap"><table class="summary-table planner-table"><thead><tr>
    <th>Source / physical case</th><th>Algorithm / policy</th><th>Plan</th><th>Topology</th><th>Clearance (planned / sampled)</th><th>Route / loft</th><th>Authority</th>
  </tr></thead><tbody>{rows}</tbody></table></div>
</section>"#,
        rows = rows,
    )
}

fn render_planner_row(record: &crate::BatchRunRecord, role: &str, changed: bool) -> String {
    let provenance = record.resolved.route_provenance.as_ref();
    let planner = record.review.planner.as_ref();
    let source = planner
        .and_then(|value| value.route_source.as_deref())
        .or_else(|| provenance.map(|value| value.route_source.as_str()))
        .unwrap_or("planner_matrix");
    let physical_case = planner
        .and_then(|value| value.physical_case_id.as_deref())
        .or(record.resolved.physical_case_id.as_deref())
        .or_else(|| provenance.map(|value| value.physical_case_id.as_str()))
        .unwrap_or(record.resolved.run_id.as_str());
    let algorithm = planner
        .and_then(|value| value.algorithm_id.as_deref())
        .or_else(|| provenance.map(|value| value.algorithm_id.as_str()))
        .unwrap_or("heightfield_visibility_v1");
    let policy_version = planner
        .and_then(|value| value.policy_version.as_deref())
        .or_else(|| provenance.map(|value| value.policy_version.as_str()))
        .unwrap_or("unknown");
    let policy_digest = planner
        .and_then(|value| value.policy_digest.as_deref())
        .or_else(|| provenance.map(|value| value.policy_digest.as_str()))
        .unwrap_or("unknown");
    let request_digest = planner
        .and_then(|value| value.request_digest.as_deref())
        .or_else(|| {
            record
                .resolved
                .route_plan
                .as_ref()
                .map(|value| value.request_digest.as_str())
        })
        .or_else(|| provenance.map(|value| value.request_digest.as_str()))
        .unwrap_or("unknown");
    let plan_digest = planner
        .and_then(|value| value.plan_digest.as_deref())
        .or_else(|| {
            record
                .resolved
                .route_plan
                .as_ref()
                .map(|value| value.plan_digest.as_str())
        })
        .or_else(|| provenance.map(|value| value.plan_digest.as_str()))
        .unwrap_or("unknown");
    let topology = planner
        .and_then(|value| value.topology.clone())
        .or_else(|| {
            record
                .resolved
                .route_plan
                .as_ref()
                .map(|value| format!("{:?}", value.topology))
        })
        .unwrap_or_else(|| "unknown".to_owned());
    let waypoint_count = planner
        .and_then(|value| value.waypoint_count)
        .or_else(|| {
            record
                .resolved
                .route_plan
                .as_ref()
                .map(|value| value.waypoints().len())
        })
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".to_owned());
    let planned_clearance = planner
        .and_then(|value| value.planned_min_clearance_m)
        .map(format_planner_metric)
        .unwrap_or_else(|| "n/a".to_owned());
    let sampled_clearance = planner
        .and_then(|value| value.sampled_en_route_min_hull_clearance_m)
        .map(format_planner_metric)
        .unwrap_or_else(|| "n/a".to_owned());
    let direct_result = match planner.and_then(|value| value.direct_path_clear) {
        Some(true) => "clear".to_owned(),
        Some(false) => planner
            .and_then(|value| value.direct_rejection_residual_m)
            .map(|residual| format!("blocked · residual {residual:.1} m"))
            .unwrap_or_else(|| "blocked".to_owned()),
        None => "n/a".to_owned(),
    };
    let route = planner
        .and_then(|value| value.route_length_m)
        .map(format_planner_metric)
        .unwrap_or_else(|| "n/a".to_owned());
    let direct_distance = planner
        .and_then(|value| value.direct_distance_m)
        .map(format_planner_metric)
        .unwrap_or_else(|| "n/a".to_owned());
    let excess = planner
        .and_then(|value| value.excess_length_m)
        .map(format_planner_metric)
        .unwrap_or_else(|| "n/a".to_owned());
    let loft = planner
        .and_then(|value| value.peak_extra_loft_m)
        .map(format_planner_metric)
        .unwrap_or_else(|| "n/a".to_owned());
    let authority_caps = planner
        .map(|value| format_planner_list(&value.authority_caps_mps))
        .unwrap_or_else(|| "n/a".to_owned());
    let authority_ratios = planner
        .map(|value| format_planner_list(&value.authority_ratios))
        .unwrap_or_else(|| "n/a".to_owned());
    let planner_compute = planner
        .and_then(|value| value.compute_wall_time_us)
        .or_else(|| {
            record
                .resolved
                .planner_compute
                .as_ref()
                .map(|value| value.wall_time_us)
        })
        .map(|value| format!("{:.3} ms monotonic wall", value as f64 / 1000.0))
        .unwrap_or_else(|| "n/a".to_owned());
    let changed_html = if changed {
        r#" <span class="status-chip warn">changed plan</span>"#
    } else {
        ""
    };
    format!(
        r#"<tr data-planner-case="{}"><td><div class="overview-stack"><span class="overview-main">{} · {}</span><span class="overview-sub">{} · run {}</span>{}</div></td><td><div class="overview-stack"><span class="overview-main">{}</span><span class="overview-sub">{} · policy {} · request {}</span></div></td><td><code>{}</code></td><td>{} / {} waypoint{}</td><td><div class="overview-stack"><span class="overview-main">{} / {} m</span><span class="overview-sub">direct {}</span></div></td><td><div class="overview-stack"><span class="overview-main">{} m</span><span class="overview-sub">direct {} · excess {} · loft {} m</span></div></td><td><div class="overview-stack"><span class="overview-main">caps {}</span><span class="overview-sub">ratios {} · compute {}</span></div></td></tr>"#,
        escape_html(physical_case),
        escape_html(role),
        escape_html(source),
        escape_html(physical_case),
        escape_html(&record.resolved.run_id),
        changed_html,
        escape_html(algorithm),
        escape_html(policy_version),
        escape_html(&short_digest(policy_digest)),
        escape_html(&short_digest(request_digest)),
        escape_html(&short_digest(plan_digest)),
        escape_html(&topology),
        escape_html(&waypoint_count),
        if waypoint_count == "1" { "" } else { "s" },
        escape_html(&planned_clearance),
        escape_html(&sampled_clearance),
        escape_html(&direct_result),
        escape_html(&route),
        escape_html(&direct_distance),
        escape_html(&excess),
        escape_html(&loft),
        escape_html(&authority_caps),
        escape_html(&authority_ratios),
        escape_html(&planner_compute),
    )
}

fn format_planner_metric(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.1}")
    } else {
        "n/a".to_owned()
    }
}

fn format_planner_list(values: &[f64]) -> String {
    if values.is_empty() {
        "n/a".to_owned()
    } else {
        values
            .iter()
            .map(|value| format_planner_metric(*value))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn candidate_record_map(candidate: &BatchReport) -> BTreeMap<String, String> {
    candidate
        .records
        .iter()
        .filter_map(|record| {
            record
                .bundle_dir
                .as_ref()
                .map(|bundle_dir| (record.resolved.run_id.clone(), bundle_dir.clone()))
        })
        .collect()
}

fn render_pointer_links(pointer: &BatchRunPointer, output_dir: &Path) -> String {
    let Some(bundle_dir) = pointer.bundle_dir.as_ref() else {
        return r#"<span class="muted">-</span>"#.to_owned();
    };
    render_link_row_for_bundle("bundle", bundle_dir, output_dir)
}

fn render_dual_links(
    run_id: &str,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
    output_dir: &Path,
) -> String {
    let mut links = Vec::new();
    if let Some(bundle_dir) = candidate_record_map.get(run_id) {
        links.push(render_link_row_for_bundle("cur", bundle_dir, output_dir));
    }
    if let Some(bundle_dir) = baseline_record_map.get(run_id) {
        links.push(render_link_row_for_bundle("base", bundle_dir, output_dir));
    }
    if links.is_empty() {
        return format!(r#"<span class="muted mono">{}</span>"#, escape_html(run_id));
    }
    links.join("")
}

fn render_run_preview(record: &crate::BatchRunRecord, output_dir: &Path) -> String {
    let Some(bundle_dir) = record.bundle_dir.as_ref() else {
        return r#"<span class="muted">no bundle</span>"#.to_owned();
    };
    let bundle_dir = resolve_repo_relative(Path::new(bundle_dir));
    let preview_path = bundle_dir.join("preview.svg");
    let detail_href = best_bundle_href(&bundle_dir, output_dir);
    if preview_path.is_file() {
        return format!(
            r#"<a class="run-preview" href="{href}"><img src="{img}" alt="{alt}" loading="lazy" decoding="async" fetchpriority="low"></a>"#,
            href = escape_html(&detail_href),
            img = escape_html(&relative_href(output_dir, &preview_path)),
            alt = escape_html(&record.resolved.run_id),
        );
    }
    render_link_row_for_bundle("run", bundle_dir.to_string_lossy().as_ref(), output_dir)
}

fn render_lane_preview(
    records: &[&crate::BatchRunRecord],
    render_cache: &BatchReportRenderCache,
) -> Option<String> {
    let cache_key = records
        .iter()
        .filter_map(|record| record.bundle_dir.as_deref())
        .map(|bundle_dir| resolve_repo_relative(Path::new(bundle_dir)))
        .map(|bundle_dir| fs::canonicalize(&bundle_dir).unwrap_or(bundle_dir))
        .collect::<Vec<_>>();
    if let Some(preview) = render_cache.lane_previews.borrow().get(&cache_key) {
        return preview.clone();
    }

    let preview = render_lane_preview_uncached(records);
    render_cache
        .lane_previews
        .borrow_mut()
        .insert(cache_key, preview.clone());
    preview
}

fn render_lane_preview_uncached(records: &[&crate::BatchRunRecord]) -> Option<String> {
    let mut loaded = Vec::new();
    for record in records {
        let Some(bundle_dir) = record.bundle_dir.as_deref() else {
            continue;
        };
        let bundle_dir = resolve_repo_relative(Path::new(bundle_dir));
        let Some(scenario) = load_json_file::<ScenarioSpec>(&bundle_dir.join("scenario.json"))
        else {
            continue;
        };
        let Some(trajectory_positions_m) =
            load_preview_trajectory(&bundle_dir.join("samples.json"))
        else {
            continue;
        };
        loaded.push((scenario, trajectory_positions_m, &record.manifest));
    }
    if loaded.is_empty() {
        return None;
    }
    let series = loaded
        .iter()
        .map(
            |(scenario, trajectory_positions_m, manifest)| AggregatePreviewSeries {
                scenario,
                manifest,
                trajectory_positions_m,
            },
        )
        .collect::<Vec<_>>();
    Some(format!(
        r#"<div class="run-preview lane-preview">{}</div>"#,
        build_multi_run_trajectory_preview_svg(&series)
    ))
}

#[derive(Deserialize)]
struct PreviewSample {
    observation: PreviewObservation,
}

#[derive(Deserialize)]
struct PreviewObservation {
    position_m: Vec2,
}

fn load_preview_trajectory(path: &Path) -> Option<Vec<Vec2>> {
    let file = File::open(path).ok()?;
    let samples = serde_json::from_reader::<_, Vec<PreviewSample>>(BufReader::new(file)).ok()?;
    Some(
        samples
            .into_iter()
            .map(|sample| sample.observation.position_m)
            .collect(),
    )
}

fn render_summary_note_with_preview(note_html: &str, preview_html: Option<&str>) -> String {
    let note_html = if note_html.trim() == r#"<span class="row-note muted">-</span>"# {
        ""
    } else {
        note_html
    };
    match preview_html {
        Some(preview_html) => format!(
            r#"{note_html}<div class="preview-cell">{preview_html}</div>"#,
            note_html = note_html,
            preview_html = preview_html,
        ),
        None => note_html.to_owned(),
    }
}

fn render_link_row_for_bundle(label: &str, bundle_dir: &str, output_dir: &Path) -> String {
    let bundle_dir = resolve_repo_relative(Path::new(bundle_dir));
    let href = best_bundle_href(&bundle_dir, output_dir);
    format!(
        r#"<a href="{}">{}:{}</a>"#,
        escape_html(&href),
        escape_html(label),
        escape_html(
            bundle_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("run")
        )
    )
}

fn best_bundle_href(bundle_dir: &Path, output_dir: &Path) -> String {
    let site_report_path = report_site_output_for_batch_run(bundle_dir);
    let report_path = bundle_dir.join("report.html");
    let manifest_path = bundle_dir.join("manifest.json");
    if site_report_path.as_ref().is_some_and(|path| path.is_file()) {
        relative_href(
            output_dir,
            site_report_path.as_ref().expect("checked above"),
        )
    } else if report_path.is_file() {
        relative_href(output_dir, &report_path)
    } else {
        relative_href(output_dir, &manifest_path)
    }
}

fn load_json_file<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    let raw = serde_json::to_string_pretty(value)?;
    fs::write(path, raw)
        .with_context(|| format!("failed to write json file {}", path.display()))?;
    Ok(())
}

fn report_site_output_for_batch(batch_dir: &Path) -> Option<PathBuf> {
    let resolved_batch_dir = resolve_repo_relative(batch_dir);
    let relative = resolved_batch_dir
        .strip_prefix(crate::repo_root().join("outputs"))
        .ok()?;
    let mut components = relative.components();
    if components.next()?.as_os_str() == "eval"
        && components.next().map(|part| part.as_os_str()) == Some("cache".as_ref())
    {
        return None;
    }
    Some(
        crate::repo_root()
            .join("outputs")
            .join("reports")
            .join(relative)
            .join("index.html"),
    )
}

fn directory_href(from_dir: &Path, target_dir: &Path) -> String {
    let href = relative_href(from_dir, target_dir);
    if href.ends_with('/') {
        href
    } else {
        format!("{href}/")
    }
}

fn html_with_base_href(html: &str, base_href: &str) -> String {
    html.replacen(
        "<head>",
        &format!("<head>\n  <base href=\"{}\" />", escape_html(base_href)),
        1,
    )
}

fn report_site_output_for_batch_run(bundle_dir: &Path) -> Option<PathBuf> {
    let resolved_bundle_dir = resolve_repo_relative(bundle_dir);
    let relative = resolved_bundle_dir
        .strip_prefix(crate::repo_root().join("outputs"))
        .ok()?;
    Some(
        crate::repo_root()
            .join("outputs")
            .join("reports")
            .join(relative)
            .join("index.html"),
    )
}

fn resolve_repo_relative(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        crate::repo_root().join(path)
    }
}

fn relative_href(from_dir: &Path, target: &Path) -> String {
    let from_dir = normalize_path(from_dir);
    let target = normalize_path(target);
    if from_dir.as_os_str().is_empty() || target.as_os_str().is_empty() {
        return target.to_string_lossy().into_owned();
    }

    let from_components = from_dir.components().collect::<Vec<_>>();
    let target_components = target.components().collect::<Vec<_>>();
    let mut shared = 0usize;
    while shared < from_components.len()
        && shared < target_components.len()
        && from_components[shared] == target_components[shared]
    {
        shared += 1;
    }

    let mut relative = PathBuf::new();
    for _ in shared..from_components.len() {
        relative.push("..");
    }
    for component in target_components.iter().skip(shared) {
        relative.push(component.as_os_str());
    }

    if relative.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        relative.to_string_lossy().replace('\\', "/")
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn percentage(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        (numerator as f64 / denominator as f64) * 100.0
    }
}

fn format_percent_delta(delta: f64) -> String {
    format!("{:+.1} pp", delta * 100.0)
}

fn format_signed_seconds(value: f64) -> String {
    format!("{:+.2}s", value)
}

fn format_signed_kg(value: f64) -> String {
    format!("{:+.1}kg", value)
}

fn format_signed_i64(value: i64) -> String {
    format!("{value:+}")
}

fn format_margin_ratio(value: f64) -> String {
    format!("{:+.1}%", value * 100.0)
}

fn format_margin_delta(value: f64) -> String {
    format!("{:+.1} pp", value * 100.0)
}

fn delta_class(value: f64) -> &'static str {
    if value > 0.0 {
        "delta-pos"
    } else if value < 0.0 {
        "delta-neg"
    } else {
        "delta-flat"
    }
}

fn margin_class(value: f64) -> &'static str {
    if value > 0.0 {
        "good"
    } else if value < 0.0 {
        "bad"
    } else {
        "warn"
    }
}

fn render_policy_status_chip(status: BatchRegressionPolicyStatus) -> String {
    format!(
        r#"<span class="status-chip {}">{}</span>"#,
        policy_status_class(status),
        escape_html(&enum_label(&status)),
    )
}

fn policy_status_class(status: BatchRegressionPolicyStatus) -> &'static str {
    match status {
        BatchRegressionPolicyStatus::Pass => "ok",
        BatchRegressionPolicyStatus::Warn => "warn",
        BatchRegressionPolicyStatus::Fail => "bad",
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn enum_label<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"unknown\"".to_owned())
        .trim_matches('"')
        .to_owned()
}

#[cfg(test)]
mod report_tests;
