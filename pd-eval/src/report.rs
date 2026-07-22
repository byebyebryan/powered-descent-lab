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

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>
    :root {{
      color-scheme: light;
      --bg: #f1ede5;
      --surface: #fffdf8;
      --surface-strong: #f8f2e8;
      --ink: #20211e;
      --muted: #6d665c;
      --line: #d9cdbc;
      --accent: #b95024;
      --accent-soft: #f4ded1;
      --good: #176b5c;
      --bad: #a43a2c;
      --warn: #966515;
      --display: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Palatino, Georgia, serif;
      --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
      --sans: "Avenir Next", "IBM Plex Sans", "Trebuchet MS", sans-serif;
      --shadow: 0 18px 44px rgba(45, 34, 23, 0.08);
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      background:
        linear-gradient(rgba(69, 58, 44, 0.025) 1px, transparent 1px),
        linear-gradient(90deg, rgba(69, 58, 44, 0.025) 1px, transparent 1px),
        radial-gradient(circle at 12% 0%, rgba(185,80,36,0.12), transparent 31rem),
        linear-gradient(180deg, #faf7f0 0%, var(--bg) 100%);
      background-size: 32px 32px, 32px 32px, auto, auto;
      background-attachment: fixed;
      color: var(--ink);
      font-family: var(--sans);
      line-height: 1.45;
    }}
    .page {{
      max-width: 1500px;
      margin: 0 auto;
      padding: 20px 20px 52px;
    }}
    .hero {{
      position: relative;
      overflow: hidden;
      display: flex;
      justify-content: space-between;
      gap: 18px;
      align-items: flex-start;
      margin-bottom: 20px;
      padding: 20px 22px 22px;
      border: 1px solid var(--line);
      border-radius: 22px;
      background:
        radial-gradient(circle at 90% 20%, rgba(185,80,36,0.09), transparent 18rem),
        rgba(255,253,248,0.94);
      box-shadow: var(--shadow);
    }}
    .hero::before {{
      content: "";
      position: absolute;
      inset: 0 0 auto;
      height: 4px;
      background: linear-gradient(90deg, var(--accent) 0 42%, var(--good) 42% 71%, #3568a8 71%);
    }}
    .hero::after {{
      content: "";
      position: absolute;
      width: 250px;
      height: 250px;
      right: -92px;
      top: -154px;
      border: 1px solid rgba(185,80,36,0.2);
      border-radius: 50%;
      pointer-events: none;
    }}
    .hero > * {{
      position: relative;
      z-index: 1;
    }}
    .hero h1 {{
      margin: 0 0 6px;
      max-width: 34ch;
      font-family: var(--display);
      font-size: clamp(1.9rem, 3.2vw, 2.7rem);
      font-weight: 500;
      letter-spacing: -0.035em;
      line-height: 0.98;
      overflow-wrap: anywhere;
    }}
    .subtitle {{
      margin: 0;
      color: var(--muted);
      max-width: 68ch;
      font-size: 0.94rem;
    }}
    .chip-row {{
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-top: 10px;
    }}
    .chip {{
      display: inline-flex;
      gap: 6px;
      align-items: center;
      border-radius: 999px;
      border: 1px solid var(--line);
      background: rgba(255,255,255,0.75);
      padding: 5px 9px;
      font-size: 0.8rem;
      color: var(--muted);
      max-width: 100%;
      overflow-wrap: anywhere;
    }}
    .chip strong {{
      color: var(--ink);
      font-weight: 700;
      flex: none;
      white-space: nowrap;
    }}
    .chip .mono {{
      min-width: 0;
      overflow-wrap: anywhere;
      word-break: break-all;
    }}
    .hero-actions {{
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      justify-content: flex-end;
    }}
    .hero-actions a {{
      text-decoration: none;
      color: var(--ink);
      border: 1px solid var(--line);
      background: var(--surface);
      padding: 7px 12px;
      border-radius: 999px;
      font-size: 0.84rem;
      white-space: nowrap;
      box-shadow: 0 3px 10px rgba(45,34,23,0.04);
    }}
    .hero-actions a:hover {{
      border-color: var(--accent);
      color: var(--accent);
      text-decoration: none;
      transform: translateY(-1px);
    }}
    .header-overview {{
      margin-bottom: 16px;
    }}
    .header-context {{
      margin-bottom: 16px;
      border: 1px solid var(--line);
      border-radius: 14px;
      background: rgba(255,253,248,0.72);
      padding: 10px 12px;
    }}
    .header-context > summary {{
      cursor: pointer;
      list-style: none;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
    }}
    .header-context > summary::-webkit-details-marker {{ display: none; }}
    .header-context > summary::after {{
      content: "+";
      color: var(--accent);
      font-weight: 800;
    }}
    .header-context[open] > summary::after {{ content: "−"; }}
    .header-context[open] > summary {{ margin-bottom: 10px; }}
    .header-context.attention {{
      border-color: rgba(181,93,45,0.42);
      background: rgba(243,214,198,0.24);
    }}
    .section-head {{
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      flex-wrap: wrap;
      gap: 10px 14px;
      margin-bottom: 10px;
    }}
    .header-context h2,
    .header-overview h2,
    .transfer-handoff-section h2,
    .transfer-shape-section h2,
    .review-tree-section h2 {{
      margin: 0;
      font-family: var(--display);
      font-size: 1.28rem;
      font-weight: 600;
      letter-spacing: -0.015em;
      color: var(--ink);
    }}
    .header-context .table-wrap {{
      overflow-x: auto;
    }}
    .context-table {{
      width: 100%;
      min-width: 0;
      table-layout: fixed;
    }}
    .context-table thead th {{
      white-space: normal;
      font-size: 0.74rem;
      letter-spacing: 0.05em;
      text-transform: uppercase;
      color: var(--muted);
      background: rgba(248,243,234,0.92);
      line-height: 1.2;
    }}
    .context-table thead th,
    .context-table td {{
      width: calc(100% / 7);
      vertical-align: top;
      overflow-wrap: anywhere;
      word-break: break-word;
    }}
    .context-value {{
      display: grid;
      gap: 2px;
      font-variant-numeric: tabular-nums;
      min-width: 0;
    }}
    .context-main {{
      color: var(--ink);
      font-weight: 700;
      line-height: 1.25;
      overflow-wrap: anywhere;
      word-break: break-word;
    }}
    .context-sub {{
      color: var(--muted);
      font-size: 0.78rem;
      line-height: 1.25;
      overflow-wrap: anywhere;
      word-break: break-word;
    }}
    .context-main code,
    .context-sub code {{
      white-space: normal;
      word-break: break-all;
    }}
    .status-chip {{
      display: inline-flex;
      align-items: center;
      padding: 2px 8px;
      border-radius: 999px;
      border: 1px solid var(--line);
      font-size: 0.74rem;
      font-weight: 700;
      letter-spacing: 0.05em;
      text-transform: uppercase;
      line-height: 1.1;
    }}
    .status-chip.ok {{
      background: rgba(47, 125, 74, 0.12);
      color: var(--good);
      border-color: rgba(47, 125, 74, 0.22);
    }}
    .status-chip.partial {{
      background: rgba(199, 160, 84, 0.16);
      color: #7a5611;
      border-color: rgba(199, 160, 84, 0.26);
    }}
    .status-chip.warn {{
      background: rgba(181, 93, 45, 0.12);
      color: #8a5126;
      border-color: rgba(181, 93, 45, 0.22);
    }}
    .status-chip.bad {{
      background: rgba(176, 58, 46, 0.12);
      color: #9b2f24;
      border-color: rgba(176, 58, 46, 0.24);
    }}
    .status-chip.muted {{
      background: rgba(102, 92, 79, 0.08);
      color: var(--muted);
      border-color: rgba(102, 92, 79, 0.16);
    }}
    .summary-table {{
      width: 100%;
      min-width: 1040px;
    }}
    .coverage-section {{
      margin-bottom: 18px;
      padding: 16px;
      border: 1px solid var(--line);
      border-radius: 18px;
      background: rgba(255,253,248,0.88);
      box-shadow: 0 12px 32px rgba(39,28,18,0.05);
    }}
    .coverage-section h2 {{
      margin: 0;
      font-family: var(--display);
      font-size: 1.28rem;
      font-weight: 600;
      letter-spacing: -0.015em;
    }}
    .coverage-filters {{ display: flex; flex-wrap: wrap; gap: 9px; }}
    .coverage-filters label {{
      display: grid;
      gap: 3px;
      color: var(--muted);
      font-size: 0.7rem;
      font-weight: 700;
      letter-spacing: 0.05em;
      text-transform: uppercase;
    }}
    .coverage-filters select {{
      border: 1px solid var(--line);
      border-radius: 9px;
      background: var(--surface);
      color: var(--ink);
      padding: 6px 9px;
      font: inherit;
      font-size: 0.82rem;
      text-transform: none;
    }}
    .coverage-table {{ min-width: 680px; table-layout: fixed; }}
    .coverage-table th:first-child {{ width: 112px; }}
    .coverage-cell {{
      cursor: pointer;
      border: 1px solid rgba(215,205,189,0.76);
      border-radius: 10px;
      background: rgba(23,107,92,0.065);
      padding: 7px 8px;
      min-height: 52px;
      box-shadow: inset 3px 0 0 rgba(23,107,92,0.38);
    }}
    .coverage-cell:hover {{
      border-color: var(--accent);
      transform: translateY(-1px);
    }}
    .coverage-target,
    .coverage-target > td {{ animation: coverage-flash 1.4s ease-out; }}
    @keyframes coverage-flash {{
      0%, 35% {{ background: rgba(181,93,45,0.24); }}
      100% {{ background: inherit; }}
    }}
    .coverage-cell.has-failure {{
      background: rgba(164,58,44,0.075);
      box-shadow: inset 3px 0 0 rgba(164,58,44,0.48);
    }}
    .coverage-cell.invalid-only {{
      background: rgba(150,101,21,0.07);
      box-shadow: inset 3px 0 0 rgba(150,101,21,0.42);
    }}
    .coverage-cell strong {{ display: block; font-size: 0.92rem; }}
    .coverage-cell span {{ color: var(--muted); font-size: 0.72rem; }}
    .coverage-cell .coverage-delta {{ color: var(--bad); }}
    .coverage-cell .coverage-delta.improved {{ color: var(--good); }}
    .guidance-diagnostics {{
      margin-bottom: 18px;
      border: 1px solid var(--line);
      border-radius: 16px;
      background: rgba(255,253,248,0.62);
      padding: 11px 13px;
    }}
    .guidance-diagnostics > summary {{ cursor: pointer; list-style: none; }}
    .guidance-diagnostics > summary::-webkit-details-marker {{ display: none; }}
    .guidance-diagnostics > summary h2 {{ margin: 0; font-size: 1rem; }}
    .guidance-diagnostics > summary h2::before {{ content: "[+]"; margin-right: 8px; color: var(--muted); }}
    .guidance-diagnostics[open] > summary h2::before {{ content: "[-]"; }}
    .guidance-diagnostics[open] > summary {{ margin-bottom: 14px; }}
    .summary-table tbody td {{
      vertical-align: top;
    }}
    .current-summary-row,
    .current-summary-row > td {{
      background: rgba(23, 107, 92, 0.045);
    }}
    .baseline-summary-row,
    .baseline-summary-row > td {{
      background: rgba(185, 80, 36, 0.045);
    }}
    .diff-summary-row,
    .diff-summary-row > td {{
      background: rgba(150, 101, 21, 0.045);
    }}
    .summary-table code {{
      font-size: 0.82rem;
    }}
    .overview-stack {{
      display: grid;
      gap: 4px;
    }}
    .overview-main {{
      font-weight: 700;
      color: var(--ink);
      font-variant-numeric: tabular-nums;
    }}
    .overview-sub {{
      color: var(--muted);
      font-size: 0.84rem;
      font-variant-numeric: tabular-nums;
    }}
    .card {{
      background: rgba(255,253,248,0.92);
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 14px 15px;
      box-shadow: 0 12px 32px rgba(39,28,18,0.055);
      min-width: 0;
    }}
    .card h2 {{
      margin: 0 0 10px;
      font-size: 0.9rem;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--muted);
    }}
    .metric {{
      display: flex;
      justify-content: space-between;
      gap: 10px;
      align-items: baseline;
      margin-top: 7px;
      font-size: 0.92rem;
    }}
    .metric strong {{
      font-size: 1.2rem;
      font-variant-numeric: tabular-nums;
    }}
    .metric .good {{ color: var(--good); }}
    .metric .bad {{ color: var(--bad); }}
    .metric .warn {{ color: var(--warn); }}
    .layout {{
      display: grid;
      grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
      gap: 12px;
      margin-bottom: 12px;
    }}
    .panel {{
      background: rgba(255,253,248,0.94);
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 14px 15px;
      box-shadow: 0 12px 32px rgba(39,28,18,0.05);
      min-width: 0;
    }}
    .panel h2 {{
      margin: 0 0 8px;
      font-size: 1rem;
    }}
    .panel p {{
      margin: 0 0 10px;
      color: var(--muted);
      font-size: 0.9rem;
    }}
    .table-wrap {{ overflow-x: auto; }}
    table {{
      width: 100%;
      border-collapse: collapse;
      font-size: 0.88rem;
      font-variant-numeric: tabular-nums;
    }}
    th, td {{
      text-align: left;
      padding: 8px 10px;
      border-bottom: 1px solid rgba(217,205,188,0.72);
      vertical-align: top;
    }}
    th {{
      color: var(--muted);
      font-weight: 700;
      font-size: 0.76rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      white-space: nowrap;
    }}
    td code {{
      font-family: var(--mono);
      font-size: 0.82rem;
      background: rgba(248,243,234,0.9);
      padding: 1px 5px;
      border-radius: 6px;
    }}
    a {{
      color: var(--accent);
      text-decoration: none;
    }}
    a:hover {{ text-decoration: underline; }}
    .rate {{
      display: grid;
      gap: 4px;
      min-width: 150px;
    }}
    .rate-bar {{
      width: 100%;
      height: 7px;
      border-radius: 999px;
      background: #e7ddd0;
      overflow: hidden;
    }}
    .rate-fill {{
      height: 100%;
      border-radius: 999px;
      background: linear-gradient(90deg, var(--good), #4e9c82);
    }}
    .seed-list {{
      color: var(--muted);
      max-width: 18rem;
      word-break: break-word;
    }}
    .muted {{ color: var(--muted); }}
    .mono {{ font-family: var(--mono); }}
    .good {{ color: var(--good); }}
    .bad {{ color: var(--bad); }}
    .warn {{ color: var(--warn); }}
    .delta-pos {{ color: var(--bad); }}
    .delta-neg {{ color: var(--good); }}
    .delta-flat {{ color: var(--muted); }}
    .tree-stack {{
      display: grid;
      gap: 14px;
    }}
    .tree-table-section {{
      border: 1px solid var(--line);
      border-radius: 16px;
      background: rgba(255,253,248,0.86);
      box-shadow: 0 10px 28px rgba(39,28,18,0.045);
      padding: 12px 12px 10px;
      min-width: 0;
      max-width: 100%;
    }}
    .table-heading {{
      display: flex;
      flex-wrap: wrap;
      justify-content: space-between;
      gap: 8px 12px;
      align-items: baseline;
      margin-bottom: 10px;
    }}
    .table-heading h3 {{
      margin: 0;
      font-size: 0.98rem;
    }}
    .table-heading h3 code {{
      font-size: 0.86rem;
    }}
    .table-heading .section-meta {{
      color: var(--muted);
      font-size: 0.84rem;
    }}
    .tree-controls {{
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
    }}
    .tree-controls button {{
      border: 1px solid var(--line);
      background: rgba(255,253,248,0.92);
      color: var(--ink);
      border-radius: 999px;
      padding: 7px 11px;
      font: inherit;
      font-size: 0.84rem;
      cursor: pointer;
    }}
    .tree-controls button:hover {{
      border-color: var(--accent);
      color: var(--accent);
      transform: translateY(-1px);
    }}
    .view-mode-controls {{
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
    }}
    .view-mode-controls button {{
      border: 1px solid var(--line);
      background: rgba(255,253,248,0.92);
      color: var(--ink);
      border-radius: 999px;
      padding: 7px 11px;
      font: inherit;
      font-size: 0.84rem;
      cursor: pointer;
    }}
    .view-mode-controls button:hover {{
      border-color: var(--accent);
      color: var(--accent);
      transform: translateY(-1px);
    }}
    .view-mode-controls button.active {{
      border-color: rgba(23, 107, 92, 0.34);
      background: rgba(23, 107, 92, 0.1);
      color: var(--good);
    }}
    .standalone-toggle-target {{
      display: none;
    }}
    #report-page.current-standalone .compare-toggle-target {{
      display: none !important;
    }}
    #report-page.current-standalone .standalone-toggle-target {{
      display: inline;
    }}
    #report-page.current-standalone .baseline-summary-row,
    #report-page.current-standalone .diff-summary-row,
    #report-page.current-standalone .baseline-row,
    #report-page.current-standalone .lane-controller-baseline,
    #report-page.current-standalone .compare-sections,
    #report-page.current-standalone .compare-only-control {{
      display: none !important;
    }}
    .scenario-table {{
      width: 100%;
      min-width: 980px;
    }}
    .scenario-table thead th {{
      background: rgba(248,243,234,0.96);
      position: sticky;
      top: 0;
      z-index: 1;
    }}
    .scenario-table th:first-child,
    .scenario-table td:first-child {{
      position: sticky;
      left: 0;
      z-index: 2;
      background: #fffdf8;
      box-shadow: 1px 0 0 rgba(217,205,188,0.82);
    }}
    .scenario-table thead th:first-child {{ z-index: 3; background: #f8f3ea; }}
    .scenario-table .current-row td:first-child {{ background: #f0f7f2; }}
    .scenario-table .baseline-row td:first-child {{ background: #faf3ed; }}
    .summary-row {{
      cursor: pointer;
    }}
    .summary-row.current-row {{
      box-shadow: inset 4px 0 0 rgba(14, 107, 96, 0.72);
    }}
    .summary-row.baseline-row {{
      box-shadow: inset 4px 0 0 rgba(163, 101, 54, 0.72);
    }}
    .summary-row.lane-controller-current {{
      box-shadow: inset 4px 0 0 rgba(47, 125, 74, 0.78);
    }}
    .summary-row.lane-controller-baseline {{
      box-shadow: inset 4px 0 0 rgba(163, 101, 54, 0.78);
    }}
    .scenario-row,
    .scenario-row > td {{
      background: rgba(23, 107, 92, 0.045);
    }}
    .scenario-row:hover,
    .scenario-row:hover > td {{
      background: rgba(23, 107, 92, 0.09);
    }}
    .baseline-scenario-row,
    .baseline-scenario-row > td {{
      background: rgba(185, 80, 36, 0.045);
    }}
    .baseline-scenario-row:hover,
    .baseline-scenario-row:hover > td {{
      background: rgba(185, 80, 36, 0.09);
    }}
    .summary-row.lane-controller-current,
    .summary-row.lane-controller-current > td {{
      background: rgba(23, 107, 92, 0.055);
    }}
    .summary-row.lane-controller-current:hover,
    .summary-row.lane-controller-current:hover > td {{
      background: rgba(23, 107, 92, 0.1);
    }}
    .summary-row.lane-controller-baseline,
    .summary-row.lane-controller-baseline > td {{
      background: rgba(185, 80, 36, 0.055);
    }}
    .summary-row.lane-controller-baseline:hover,
    .summary-row.lane-controller-baseline:hover > td {{
      background: rgba(185, 80, 36, 0.1);
    }}
    .summary-row td:first-child {{
      font-weight: 700;
    }}
    .seed-row.current-row {{
      box-shadow: inset 4px 0 0 rgba(14, 107, 96, 0.44);
    }}
    .seed-row.baseline-row {{
      box-shadow: inset 4px 0 0 rgba(163, 101, 54, 0.44);
    }}
    .seed-row.lane-controller-current {{
      box-shadow: inset 4px 0 0 rgba(47, 125, 74, 0.46);
    }}
    .seed-row.lane-controller-baseline {{
      box-shadow: inset 4px 0 0 rgba(163, 101, 54, 0.46);
    }}
    .seed-row,
    .seed-row > td {{
      background: rgba(255, 253, 248, 0.86);
    }}
    .seed-row:hover,
    .seed-row:hover > td {{
      background: rgba(248, 242, 232, 0.92);
    }}
    .baseline-seed-row,
    .baseline-seed-row > td {{
      background: rgba(250, 246, 239, 0.86);
    }}
    .baseline-seed-row:hover,
    .baseline-seed-row:hover > td {{
      background: rgba(246, 237, 226, 0.94);
    }}
    .seed-row.lane-controller-current,
    .seed-row.lane-controller-current > td {{
      background: rgba(248, 252, 249, 0.88);
    }}
    .seed-row.lane-controller-current:hover,
    .seed-row.lane-controller-current:hover > td {{
      background: rgba(238, 247, 243, 0.95);
    }}
    .seed-row.lane-controller-baseline,
    .seed-row.lane-controller-baseline > td {{
      background: rgba(252, 247, 241, 0.9);
    }}
    .seed-row.lane-controller-baseline:hover,
    .seed-row.lane-controller-baseline:hover > td {{
      background: rgba(247, 238, 228, 0.96);
    }}
    .summary-row.current-row.changed {{
      box-shadow: inset 7px 0 0 rgba(181,93,45,0.72);
    }}
    .summary-row.baseline-row.changed {{
      box-shadow: inset 7px 0 0 rgba(181,93,45,0.5);
    }}
    .tree-label {{
      padding-left: calc(10px + var(--depth, 0) * 22px);
      white-space: nowrap;
    }}
    .row-tag {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 4.9rem;
      margin-right: 8px;
      padding: 2px 8px;
      border-radius: 999px;
      font-size: 0.68rem;
      font-weight: 700;
      line-height: 1.15;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      border: 1px solid transparent;
    }}
    .row-tag.current {{
      background: rgba(14, 107, 96, 0.16);
      color: var(--accent);
      border-color: rgba(14, 107, 96, 0.22);
    }}
    .row-tag.baseline {{
      background: rgba(181, 126, 80, 0.14);
      color: #8a5126;
      border-color: rgba(181, 126, 80, 0.24);
    }}
    .row-tag.diff {{
      background: rgba(199, 160, 84, 0.18);
      color: #7a5611;
      border-color: rgba(199, 160, 84, 0.24);
    }}
    .row-tag.lane-current {{
      background: rgba(47, 125, 74, 0.16);
      color: var(--good);
      border-color: rgba(47, 125, 74, 0.24);
    }}
    .row-tag.lane-baseline {{
      background: rgba(181, 93, 45, 0.16);
      color: #8a5126;
      border-color: rgba(181, 93, 45, 0.24);
    }}
    .outcome-bad {{
      color: var(--bad);
      font-weight: 700;
    }}
    .expander {{
      display: inline-block;
      width: 1.15rem;
      color: var(--accent);
      font-weight: 700;
      text-align: center;
      transform-origin: center;
    }}
    .expander.muted {{
      color: var(--muted);
    }}
    .summary-row[aria-expanded="true"] .expander {{
      transform: rotate(45deg);
    }}
    .selector-code {{
      font-family: var(--mono);
      font-size: 0.82rem;
      background: rgba(248,243,234,0.9);
      padding: 1px 5px;
      border-radius: 6px;
    }}
    .selector-inline {{
      font-family: var(--mono);
      font-size: 0.8rem;
      color: var(--muted);
    }}
    .row-note {{
      display: inline-flex;
      align-items: center;
      gap: 6px;
      color: var(--muted);
      font-size: 0.8rem;
      line-height: 1.3;
      flex-wrap: wrap;
    }}
    .row-note .emph {{
      color: var(--ink);
      font-weight: 600;
    }}
    .row-note .bad {{
      color: var(--bad);
      font-weight: 700;
    }}
    .row-note .good {{
      color: var(--good);
      font-weight: 700;
    }}
    .preview-cell {{
      display: grid;
      gap: 6px;
    }}
    .run-preview {{
      display: inline-flex;
      width: 148px;
      max-width: 100%;
      border-radius: 10px;
      border: 1px solid var(--line);
      background: #fbf7ee;
      overflow: hidden;
      box-shadow: 0 3px 10px rgba(39,28,18,0.06);
    }}
    .run-preview img {{
      display: block;
      width: 100%;
      height: auto;
      background: #fbf7ee;
    }}
    .run-preview svg {{
      display: block;
      width: 100%;
      height: auto;
      background: #fbf7ee;
    }}
    .lane-preview {{
      width: 148px;
    }}
    .scenario-table.baseline-hidden .baseline-row {{
      display: none;
    }}
    .review-tree-root.diff-only .scenario-table tr.unchanged {{
      display: none;
    }}
    .compare-metric-grid {{
      display: grid;
      grid-template-columns: minmax(52px, auto) repeat(5, minmax(0, 1fr));
      gap: 8px 10px;
      align-items: center;
      font-size: 0.84rem;
      color: var(--muted);
    }}
    .compare-header {{
      font-size: 0.74rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--muted);
    }}
    .compare-label {{
      font-weight: 700;
      color: var(--ink);
    }}
    .baseline-row {{
      color: var(--muted);
    }}
    .delta-row {{
      color: var(--ink);
    }}
    .review-tree-root.compare-hidden .compare-toggle-target {{
      display: none;
    }}
    .transfer-handoff-section,
    .transfer-shape-section {{
      margin-bottom: 16px;
    }}
    .transfer-handoff-section,
    .transfer-shape-section {{
      border-bottom: 1px solid rgba(215,205,189,0.88);
      padding-bottom: 12px;
    }}
    .transfer-triage-summary {{
      cursor: pointer;
      list-style: none;
    }}
    .transfer-triage-summary::-webkit-details-marker {{
      display: none;
    }}
    .transfer-triage-summary h2::before {{
      content: "[+]";
      display: inline-block;
      margin-right: 8px;
      color: var(--muted);
    }}
    .transfer-handoff-section[open] .transfer-triage-summary h2::before,
    .transfer-shape-section[open] .transfer-triage-summary h2::before {{
      content: "[-]";
    }}
    .transfer-handoff-table {{
      min-width: 1320px;
    }}
    .transfer-shape-table {{
      min-width: 1160px;
    }}
    .transfer-handoff-table tbody td,
    .transfer-shape-table tbody td {{
      vertical-align: top;
    }}
    .transfer-handoff-table .overview-main,
    .transfer-shape-table .overview-main {{
      white-space: nowrap;
    }}
    .triage-risk .overview-main,
    .triage-risk {{
      color: var(--bad);
      font-weight: 700;
    }}
    .triage-warn .overview-main,
    .triage-warn {{
      color: var(--warn);
      font-weight: 700;
    }}
    .review-tree-section {{
      border-top: 1px solid rgba(215,205,189,0.88);
      padding-top: 16px;
      margin-bottom: 16px;
    }}
    .review-tree-section > p {{
      margin: 0 0 10px;
      color: var(--muted);
      font-size: 0.9rem;
    }}
    .section-note {{
      color: var(--muted);
      font-size: 0.86rem;
    }}
    @media (max-width: 1100px) {{
      .layout {{
        grid-template-columns: 1fr;
      }}
      .hero {{
        flex-direction: column;
      }}
      .hero-actions {{
        justify-content: flex-start;
      }}
      .scenario-table {{
        min-width: 860px;
      }}
      .compare-metric-grid {{
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }}
    }}
    @media (max-width: 700px) {{
      .page {{
        padding-inline: 14px;
      }}
      .context-table {{ min-width: 860px; }}
      .coverage-section {{ padding-inline: 10px; }}
      .summary-table {{ min-width: 0; table-layout: fixed; }}
      .summary-table th:nth-child(2),
      .summary-table td:nth-child(2),
      .summary-table th:nth-child(3),
      .summary-table td:nth-child(3),
      .summary-table th:nth-child(5),
      .summary-table td:nth-child(5),
      .summary-table th:nth-child(6),
      .summary-table td:nth-child(6),
      .summary-table th:nth-child(7),
      .summary-table td:nth-child(7) {{ display: none; }}
      .summary-table th:first-child,
      .summary-table td:first-child {{ width: 66%; overflow-wrap: anywhere; }}
      .summary-table th:nth-child(4),
      .summary-table td:nth-child(4) {{ width: 34%; }}
    }}
  </style>
</head>
<body>
  <div id="report-page" class="page">
    <header class="hero">
      <div>
        <h1>{title_html}</h1>
        <p class="subtitle">{subtitle_html}</p>
        <div class="chip-row">
          <span class="chip"><strong>pack</strong> <span class="mono">{pack_id}</span></span>
          <span class="chip"><strong>runs</strong> {total_runs}</span>
          <span class="chip"><strong>workers</strong> {workers_used}/{workers_requested}</span>
          {wall_clock_chip_html}
          <span class="chip"><strong>spec</strong> <span class="mono">{pack_digest}</span></span>
          <span class="chip"><strong>resolved</strong> <span class="mono">{resolved_digest}</span></span>
        </div>
      </div>
      <div class="hero-actions">
        <a href="summary.json">summary.json</a>
        <a href="resolved_runs.json">resolved_runs.json</a>
        {compare_json_link}
        {baseline_report_link}
      </div>
    </header>

    {overview_html}

    {coverage_html}

    {context_html}

    {guidance_diagnostics_html}

    <section class="review-tree-section">
      <div class="section-head">
        <h2>Review Tree</h2>
        {tree_controls}
      </div>
      <div id="review-tree-root" class="review-tree-root">{review_tree}</div>
    </section>

    <div class="compare-sections">{comparison_sections}</div>
  </div>
  <script>
    (() => {{
      const page = document.getElementById("report-page");
      const root = document.getElementById("review-tree-root");
      if (!root || !page) return;
      const tables = () => Array.from(root.querySelectorAll("table[data-tree-table]"));
      const childRows = (table, group) =>
        Array.from(table.querySelectorAll(`tr[data-parent="${{group}}"]`));
      const summaryRows = (table) =>
        Array.from(table.querySelectorAll("tr.summary-row"));
      const allRows = (table) =>
        Array.from(table.querySelectorAll("tr.summary-row, tr.seed-row"));
      const collapseDescendants = (table, group) => {{
        childRows(table, group).forEach((child) => {{
          child.hidden = true;
          if (child.classList.contains("summary-row") && child.dataset.group) {{
            child.setAttribute("aria-expanded", "false");
            collapseDescendants(table, child.dataset.group);
          }}
        }});
      }};
      const showImmediateChildren = (table, row, includeSeeds) => {{
        const group = row.dataset.group;
        if (!group) return;
        childRows(table, group).forEach((child) => {{
          if (child.classList.contains("seed-row")) {{
            child.hidden = !includeSeeds;
          }} else {{
            child.hidden = false;
          }}
        }});
      }};
      const summaryLevel = (kind) => {{
        switch (kind) {{
          case "mission": return 0;
          case "arrival": return 1;
          case "condition": return 2;
          case "arc": return 3;
          case "route": return 3;
          case "band": return 4;
          case "radius": return 4;
          case "vehicle": return 5;
          case "lane": return 6;
          default: return 0;
        }}
      }};
      const laneLeafDepth = summaryLevel("lane");
      const toggleRow = (row) => {{
        if (!row.classList.contains("summary-row")) return;
        const table = row.closest("table");
        if (!table) return;
        const group = row.dataset.group;
        if (!group) return;
        const expanded = row.getAttribute("aria-expanded") === "true";
        if (expanded) {{
          row.setAttribute("aria-expanded", "false");
          collapseDescendants(table, group);
        }} else {{
          row.setAttribute("aria-expanded", "true");
          showImmediateChildren(table, row, row.dataset.kind === "lane");
        }}
      }};
      const collapseGroups = (table) => {{
        allRows(table).forEach((row) => {{
          if (row.dataset.parent) {{
            row.hidden = true;
          }} else {{
            row.hidden = false;
          }}
          if (row.classList.contains("summary-row") && row.dataset.group) {{
            row.setAttribute("aria-expanded", "false");
          }}
        }});
      }};
      const applyExpansionState = (table, targetDepth, showSeeds) => {{
        collapseGroups(table);
        const clampedDepth = Math.max(0, Math.min(laneLeafDepth, targetDepth));
        table.dataset.targetDepth = String(clampedDepth);
        table.dataset.showSeeds = showSeeds ? "true" : "false";
        let changed = true;
        while (changed) {{
          changed = false;
          summaryRows(table).forEach((row) => {{
            if (row.hidden || !row.dataset.group) return;
            const level = summaryLevel(row.dataset.kind || "");
            if (level < clampedDepth) {{
              if (row.getAttribute("aria-expanded") !== "true") {{
                row.setAttribute("aria-expanded", "true");
                showImmediateChildren(table, row, false);
                changed = true;
              }}
            }} else if (level === clampedDepth && row.dataset.kind === "lane" && showSeeds) {{
              if (row.getAttribute("aria-expanded") !== "true") {{
                row.setAttribute("aria-expanded", "true");
                showImmediateChildren(table, row, true);
                changed = true;
              }}
            }}
          }});
        }}
      }};
      const expandGroups = (table) => {{
        applyExpansionState(table, laneLeafDepth, false);
      }};
      const expandSeeds = (table) => {{
        applyExpansionState(table, laneLeafDepth, true);
      }};
      const collapseSeeds = (table) => {{
        applyExpansionState(table, Number(table.dataset.targetDepth || laneLeafDepth), false);
      }};
      const stepDepth = (table, delta) => {{
        const currentDepth = Number(table.dataset.targetDepth || laneLeafDepth);
        const showSeeds = table.dataset.showSeeds === "true";
        const nextDepth = Math.max(0, Math.min(laneLeafDepth, currentDepth + delta));
        const keepSeeds = showSeeds && nextDepth === laneLeafDepth;
        applyExpansionState(table, nextDepth, keepSeeds);
      }};
      const collapseOneLevel = (table) => {{
        stepDepth(table, -1);
      }};
      const expandOneLevel = (table) => {{
        stepDepth(table, 1);
      }};
      const collapseAll = (table) => {{
        applyExpansionState(table, 0, false);
      }};
      const selectorToken = (value) => {{
        let output = "";
        let lastDash = false;
        for (const raw of value.toLowerCase()) {{
          if (/[a-z0-9]/.test(raw)) {{ output += raw; lastDash = false; }}
          else if (raw === "+") {{ output += "plus"; lastDash = false; }}
          else if (raw === "-") {{ output += "minus"; lastDash = false; }}
          else if (!lastDash) {{ output += "-"; lastDash = true; }}
        }}
        return output.replace(/^-+|-+$/g, "") || "x";
      }};
      tables().forEach((table) => {{
        expandGroups(table);
        summaryRows(table)
          .filter((row) => row.dataset.group)
          .forEach((row) => {{
            row.addEventListener("click", (event) => {{
              if (event.target.closest("a, button")) return;
              toggleRow(row);
            }});
            row.addEventListener("keydown", (event) => {{
              if (event.key !== "Enter" && event.key !== " ") return;
              event.preventDefault();
              toggleRow(row);
            }});
          }});
      }});
      document.querySelectorAll("[data-tree-action]").forEach((button) => {{
        button.addEventListener("click", () => {{
          const action = button.getAttribute("data-tree-action");
          if (action === "expand-depth") {{
            tables().forEach(expandOneLevel);
          }} else if (action === "collapse-depth") {{
            tables().forEach(collapseOneLevel);
          }} else if (action === "expand-seeds") {{
            tables().forEach(expandSeeds);
          }} else if (action === "collapse-seeds") {{
            tables().forEach(collapseSeeds);
          }} else if (action === "collapse-all") {{
            tables().forEach(collapseAll);
          }} else if (action === "toggle-baseline") {{
            tables().forEach((table) => table.classList.toggle("baseline-hidden"));
            const anyHidden = tables().some((table) => table.classList.contains("baseline-hidden"));
            button.textContent = anyHidden ? "Show Baseline" : "Hide Baseline";
          }} else if (action === "toggle-diff") {{
            root.classList.toggle("diff-only");
            button.textContent = root.classList.contains("diff-only") ? "Show All Groups" : "Show Changed Only";
          }}
        }});
      }});
      const modeButtons = Array.from(document.querySelectorAll("[data-view-mode]"));
      const setViewMode = (mode) => {{
        page.classList.toggle("current-standalone", mode === "current-only");
        modeButtons.forEach((button) => {{
          button.classList.toggle("active", button.getAttribute("data-view-mode") === mode);
        }});
      }};
      modeButtons.forEach((button) => {{
        button.addEventListener("click", () => {{
          const mode = button.getAttribute("data-view-mode");
          if (!mode) return;
          setViewMode(mode);
        }});
      }});
      setViewMode("compare");
      const coverageFilters = Array.from(document.querySelectorAll("[data-coverage-filter]"));
      const updateCoverage = () => {{
        document.querySelectorAll("[data-coverage-pane]").forEach((pane) => {{
          pane.hidden = coverageFilters.some((filter) =>
            pane.dataset[filter.dataset.coverageFilter] !== filter.value
          );
        }});
      }};
      coverageFilters.forEach((filter) => filter.addEventListener("change", updateCoverage));
      updateCoverage();
      document.querySelectorAll("[data-tree-tokens]").forEach((cell) => {{
        const inspectCoverageCell = () => {{
          const tokens = cell.dataset.treeTokens.split("|").filter(Boolean).map(selectorToken);
          let target = null;
          tables().forEach((table) => {{
            expandGroups(table);
            summaryRows(table).forEach((row) => {{
              const parts = (row.dataset.group || "").split("--");
              if (tokens.every((token) => parts.includes(token)) &&
                  (!target || (row.dataset.group || "").length > (target.dataset.group || "").length)) {{
                target = row;
              }}
            }});
          }});
          if (target) {{
            target.scrollIntoView({{behavior: "smooth", block: "center"}});
            target.classList.add("coverage-target");
            window.setTimeout(() => target.classList.remove("coverage-target"), 1400);
          }}
        }};
        cell.addEventListener("click", inspectCoverageCell);
        cell.addEventListener("keydown", (event) => {{
          if (event.key !== "Enter" && event.key !== " ") return;
          event.preventDefault();
          inspectCoverageCell();
        }});
      }});
    }})();
  </script>
</body>
</html>"#,
        title = escape_html(&title),
        title_html = escape_html(&title),
        subtitle_html = escape_html(&batch_report_subtitle(
            candidate,
            baseline.map(|(_, report)| report),
            comparison,
        )),
        pack_id = escape_html(&candidate.pack_id),
        total_runs = candidate.total_runs,
        workers_used = candidate.workers_used,
        workers_requested = candidate.workers_requested,
        wall_clock_chip_html = render_wall_clock_chip(candidate),
        pack_digest = escape_html(&candidate.identity.pack_spec_digest),
        resolved_digest = escape_html(&candidate.identity.resolved_run_digest),
        compare_json_link = if comparison.is_some() {
            r#"<a href="compare.json">compare.json</a>"#.to_owned()
        } else {
            String::new()
        },
        baseline_report_link = baseline_report_href
            .as_ref()
            .map(|href| format!(r#"<a href="{}">baseline report</a>"#, escape_html(href)))
            .unwrap_or_default(),
        context_html =
            render_context_table(candidate, baseline.map(|(_, report)| report), comparison),
        overview_html = render_overview_table(
            candidate,
            baseline.map(|(_, report)| report),
            comparison,
            render_view_controls(has_compare_view).as_str(),
        ),
        coverage_html =
            render_coverage_matrix(candidate, baseline.map(|(_, report)| report), comparison,),
        guidance_diagnostics_html = render_guidance_diagnostics(
            render_waypoint_sequence_section(candidate),
            render_waypoint_triage_section(candidate, &output_dir, &candidate_record_links),
            render_transfer_handoff_triage_section(candidate, &output_dir, &candidate_record_links,),
            render_transfer_shape_triage_section(
                candidate,
                baseline.map(|(_, report)| report),
                comparison,
                &output_dir,
                &candidate_record_links,
            ),
        ),
        tree_controls = render_tree_controls(comparison.is_some()),
        review_tree = render_review_tree(
            candidate,
            baseline.map(|(_, report)| report),
            comparison,
            &output_dir,
            render_cache,
        ),
        comparison_sections = comparison
            .map(|comparison| {
                render_comparison_sections(
                    &output_dir,
                    comparison,
                    &candidate_record_links,
                    &baseline_record_map,
                )
            })
            .unwrap_or_default(),
    )
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
