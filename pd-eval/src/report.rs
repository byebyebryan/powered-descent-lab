use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result};
use pd_core::{SampleRecord, ScenarioSpec};
use pd_report::{PreviewSeries, build_multi_run_preview_svg};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    BatchCacheInfo, BatchCacheStatus, BatchCompareResolutionStatus, BatchCompareSource,
    BatchComparison, BatchReport, BatchRunComparison, BatchRunPointer, compare_batch_reports,
};

pub fn write_batch_report_artifacts(
    output_dir: &Path,
    candidate: &BatchReport,
    baseline: Option<(&Path, &BatchReport)>,
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

    let html = render_batch_report(
        output_dir,
        candidate,
        baseline.map(|(dir, report)| (dir, report)),
        comparison.as_ref(),
    );
    fs::write(output_dir.join("report.html"), html).with_context(|| {
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
        let site_html = render_batch_report(
            site_output
                .parent()
                .expect("report site output should have parent directory"),
            candidate,
            baseline.map(|(dir, report)| (dir, report)),
            comparison.as_ref(),
        );
        fs::write(&site_output, site_html).with_context(|| {
            format!(
                "failed to write batch report site html {}",
                site_output.display()
            )
        })?;
        update_report_site_indexes_for_file(&site_output)?;
    }

    Ok(comparison)
}

fn render_batch_report(
    output_dir: &Path,
    candidate: &BatchReport,
    baseline: Option<(&Path, &BatchReport)>,
    comparison: Option<&BatchComparison>,
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
      --bg: #f4f1ea;
      --surface: #fffdf8;
      --surface-strong: #f8f3ea;
      --ink: #1d1a16;
      --muted: #665c4f;
      --line: #d7cdbd;
      --accent: #b55d2d;
      --accent-soft: #f3d6c6;
      --good: #2f7d4a;
      --bad: #b64234;
      --warn: #8f651d;
      --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
      --sans: "IBM Plex Sans", "Segoe UI", sans-serif;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      background:
        radial-gradient(circle at top left, rgba(181,93,45,0.09), transparent 28rem),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
      color: var(--ink);
      font-family: var(--sans);
      line-height: 1.45;
    }}
    .page {{
      max-width: 1500px;
      margin: 0 auto;
      padding: 24px 20px 40px;
    }}
    .hero {{
      display: flex;
      justify-content: space-between;
      gap: 18px;
      align-items: flex-start;
      margin-bottom: 18px;
    }}
    .hero h1 {{
      margin: 0 0 6px;
      font-size: 2rem;
      line-height: 1.05;
    }}
    .subtitle {{
      margin: 0;
      color: var(--muted);
      max-width: 72ch;
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
      padding: 5px 10px;
      font-size: 0.82rem;
      color: var(--muted);
    }}
    .chip strong {{
      color: var(--ink);
      font-weight: 700;
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
      padding: 7px 11px;
      border-radius: 10px;
      font-size: 0.84rem;
      white-space: nowrap;
    }}
    .hero-actions a:hover {{
      border-color: var(--accent);
      color: var(--accent);
    }}
    .header-overview {{
      margin-bottom: 16px;
    }}
    .header-context {{
      margin-bottom: 16px;
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
    .review-tree-section h2 {{
      margin: 0;
      font-size: 1rem;
      font-weight: 700;
      color: var(--ink);
    }}
    .header-context .table-wrap {{
      overflow-x: visible;
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
    .status-chip.muted {{
      background: rgba(102, 92, 79, 0.08);
      color: var(--muted);
      border-color: rgba(102, 92, 79, 0.16);
    }}
    .summary-table {{
      width: 100%;
      min-width: 1040px;
    }}
    .summary-table tbody td {{
      vertical-align: top;
    }}
    .current-summary-row,
    .current-summary-row > td {{
      background: rgba(14, 107, 96, 0.09);
    }}
    .baseline-summary-row,
    .baseline-summary-row > td {{
      background: rgba(181, 126, 80, 0.11);
    }}
    .diff-summary-row,
    .diff-summary-row > td {{
      background: rgba(199, 160, 84, 0.12);
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
      box-shadow: 0 10px 30px rgba(39,28,18,0.06);
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
      box-shadow: 0 10px 30px rgba(39,28,18,0.05);
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
      border-bottom: 1px solid rgba(215,205,189,0.82);
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
      background: linear-gradient(90deg, var(--accent), #d98955);
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
      background: rgba(255,253,248,0.9);
      box-shadow: inset 0 1px 0 rgba(255,255,255,0.7);
      padding: 12px 12px 10px;
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
      background: rgba(248,243,234,0.92);
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
    }}
    .view-mode-controls {{
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
    }}
    .view-mode-controls button {{
      border: 1px solid var(--line);
      background: rgba(248,243,234,0.92);
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
    }}
    .view-mode-controls button.active {{
      border-color: rgba(14, 107, 96, 0.38);
      background: rgba(14, 107, 96, 0.12);
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
      background: rgba(14, 107, 96, 0.11);
    }}
    .scenario-row:hover,
    .scenario-row:hover > td {{
      background: rgba(14, 107, 96, 0.17);
    }}
    .baseline-scenario-row,
    .baseline-scenario-row > td {{
      background: rgba(181, 126, 80, 0.12);
    }}
    .baseline-scenario-row:hover,
    .baseline-scenario-row:hover > td {{
      background: rgba(181, 126, 80, 0.18);
    }}
    .summary-row.lane-controller-current,
    .summary-row.lane-controller-current > td {{
      background: rgba(47, 125, 74, 0.10);
    }}
    .summary-row.lane-controller-current:hover,
    .summary-row.lane-controller-current:hover > td {{
      background: rgba(47, 125, 74, 0.16);
    }}
    .summary-row.lane-controller-baseline,
    .summary-row.lane-controller-baseline > td {{
      background: rgba(181, 126, 80, 0.12);
    }}
    .summary-row.lane-controller-baseline:hover,
    .summary-row.lane-controller-baseline:hover > td {{
      background: rgba(181, 126, 80, 0.18);
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
      background: rgba(255, 249, 238, 0.92);
    }}
    .seed-row:hover,
    .seed-row:hover > td {{
      background: rgba(255, 245, 230, 0.96);
    }}
    .baseline-seed-row,
    .baseline-seed-row > td {{
      background: rgba(245, 236, 226, 0.84);
    }}
    .baseline-seed-row:hover,
    .baseline-seed-row:hover > td {{
      background: rgba(242, 230, 216, 0.92);
    }}
    .seed-row.lane-controller-current,
    .seed-row.lane-controller-current > td {{
      background: rgba(248, 252, 248, 0.94);
    }}
    .seed-row.lane-controller-current:hover,
    .seed-row.lane-controller-current:hover > td {{
      background: rgba(239, 247, 239, 0.98);
    }}
    .seed-row.lane-controller-baseline,
    .seed-row.lane-controller-baseline > td {{
      background: rgba(247, 239, 230, 0.92);
    }}
    .seed-row.lane-controller-baseline:hover,
    .seed-row.lane-controller-baseline:hover > td {{
      background: rgba(243, 232, 219, 0.96);
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

    {context_html}

    {overview_html}

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
          case "vehicle": return 3;
          case "arc": return 4;
          case "velocity": return 5;
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
        tree_controls = render_tree_controls(comparison.is_some()),
        review_tree = render_review_tree(
            candidate,
            baseline.map(|(_, report)| report),
            comparison,
            &output_dir,
            &candidate_record_links,
            &baseline_record_map,
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

#[derive(Clone, Copy, Default)]
struct SelectorScopeCounts {
    missions: usize,
    case_groups: usize,
    lanes: usize,
}

struct LaneRecordFocus<'a> {
    lane_id: &'static str,
    records: Vec<&'a crate::BatchRunRecord>,
}

struct LaneFocusSummary {
    lane_id: &'static str,
    run_count: usize,
    controller_html: String,
    scope: SelectorScopeCounts,
    review: ReviewAggregate,
    mean_sim_time_s: f64,
    max_sim_time_s: f64,
}

fn selector_scope_counts(candidate: &BatchReport) -> SelectorScopeCounts {
    let records = candidate.records.iter().collect::<Vec<_>>();
    selector_scope_counts_from_records(records.as_slice())
}

fn selector_scope_counts_from_records(records: &[&crate::BatchRunRecord]) -> SelectorScopeCounts {
    let missions = records
        .iter()
        .map(|record| record.resolved.selector.mission.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let case_groups = records
        .iter()
        .map(|record| selector_case_key(&record.resolved.selector))
        .collect::<BTreeSet<_>>()
        .len();
    let lanes = records
        .iter()
        .map(|record| record.resolved.lane_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    SelectorScopeCounts {
        missions,
        case_groups,
        lanes,
    }
}

fn preferred_current_lane_focus<'a>(report: &'a BatchReport) -> Option<LaneRecordFocus<'a>> {
    let records = report.records.iter().collect::<Vec<_>>();
    let lane_id = preferred_current_lane_id(records.as_slice())?;
    let records = controller_lane_records(records.as_slice(), lane_id);
    (!records.is_empty()).then_some(LaneRecordFocus { lane_id, records })
}

fn summarize_lane_focus(focus: &LaneRecordFocus<'_>) -> LaneFocusSummary {
    let (mean_sim_time_s, max_sim_time_s) = overview_timing_from_records(focus.records.as_slice());
    LaneFocusSummary {
        lane_id: focus.lane_id,
        run_count: focus.records.len(),
        controller_html: render_controller_summary_inline(focus.records.as_slice()),
        scope: selector_scope_counts_from_records(focus.records.as_slice()),
        review: review_aggregate_from_records(focus.records.as_slice()),
        mean_sim_time_s,
        max_sim_time_s,
    }
}

fn compare_basis_from_records(
    mode: &str,
    candidate_records: &[&crate::BatchRunRecord],
    baseline_records: &[&crate::BatchRunRecord],
) -> crate::BatchCompareBasis {
    let candidate_run_ids = candidate_records
        .iter()
        .map(|record| record.resolved.run_id.as_str())
        .collect::<BTreeSet<_>>();
    let baseline_run_ids = baseline_records
        .iter()
        .map(|record| record.resolved.run_id.as_str())
        .collect::<BTreeSet<_>>();
    let shared_runs = candidate_run_ids.intersection(&baseline_run_ids).count();
    let candidate_only_runs = candidate_run_ids.difference(&baseline_run_ids).count();
    let baseline_only_runs = baseline_run_ids.difference(&candidate_run_ids).count();
    crate::BatchCompareBasis {
        mode: mode.to_owned(),
        shared_runs,
        candidate_only_runs,
        baseline_only_runs,
    }
}

fn compare_scope_resolution(basis: &crate::BatchCompareBasis) -> (&'static str, &'static str) {
    if basis.shared_runs == 0 {
        (
            "no shared scope",
            "no shared run set was available for comparison",
        )
    } else if basis.candidate_only_runs == 0 && basis.baseline_only_runs == 0 {
        (
            "exact",
            "candidate and baseline cover the same resolved run set",
        )
    } else {
        (
            "shared intersection",
            "report deltas are limited to the shared run intersection",
        )
    }
}

fn short_digest(value: &str) -> String {
    value.chars().take(8).collect()
}

fn render_overview_row(
    row_class: &str,
    pack_html: String,
    ref_html: String,
    scope_html: String,
    result_html: String,
    timing_html: String,
    efficiency_html: String,
    tracking_html: String,
) -> String {
    format!(
        r#"<tr class="{row_class}">
  <td>{pack_html}</td>
  <td>{ref_html}</td>
  <td>{scope_html}</td>
  <td>{result_html}</td>
  <td>{timing_html}</td>
  <td>{efficiency_html}</td>
  <td>{tracking_html}</td>
</tr>"#,
        row_class = row_class,
        pack_html = pack_html,
        ref_html = ref_html,
        scope_html = scope_html,
        result_html = result_html,
        timing_html = timing_html,
        efficiency_html = efficiency_html,
        tracking_html = tracking_html,
    )
}

fn render_overview_result_cell(
    success_runs: usize,
    total_runs: usize,
    failure_runs: usize,
    success_delta: Option<f64>,
) -> String {
    let main = format!("{:.1}%", percentage(success_runs, total_runs));
    let sub = match success_delta {
        Some(delta) => format!(
            r#"<span class="compare-toggle-target"><span class="{}">{}</span> · {} fail</span><span class="standalone-toggle-target">{} success · {} fail</span>"#,
            delta_class(-delta),
            escape_html(&format_percent_delta(delta)),
            failure_runs,
            success_runs,
            failure_runs
        ),
        None => format!("{success_runs}/{total_runs} success · {failure_runs} fail"),
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&main),
        sub
    )
}

fn render_overview_scope_cell(
    scope: &SelectorScopeCounts,
    workers_used: usize,
    compare_basis: Option<&crate::BatchCompareBasis>,
) -> String {
    let main = format!(
        "{} missions · {} groups · {} lanes",
        scope.missions, scope.case_groups, scope.lanes
    );
    let sub = compare_basis
        .map(|basis| {
            format!(
                "workers {} · shared {} · +{} / -{}",
                workers_used,
                basis.shared_runs,
                basis.candidate_only_runs,
                basis.baseline_only_runs
            )
        })
        .unwrap_or_else(|| format!("workers {}", workers_used));
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&main),
        escape_html(&sub)
    )
}

fn render_overview_timing_cell(
    mean_sim_time_s: f64,
    max_sim_time_s: f64,
    deltas: Option<(f64, f64)>,
) -> String {
    let main = format!("{mean_sim_time_s:.2}s mean");
    let sub_html = match deltas {
        Some((mean_delta, max_delta)) => {
            format!(
                r#"<span class="compare-toggle-target">{} mean · {} max</span><span class="standalone-toggle-target">{max_sim_time_s:.2}s max</span>"#,
                escape_html(&format_signed_seconds(mean_delta)),
                escape_html(&format_signed_seconds(max_delta))
            )
        }
        None => escape_html(&format!("{max_sim_time_s:.2}s max")),
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&main),
        sub_html
    )
}

fn render_overview_efficiency_cell(
    review: &ReviewAggregate,
    baseline: Option<&ReviewAggregate>,
    show_delta: bool,
) -> String {
    let fuel = review
        .fuel_used_pct_of_max
        .as_ref()
        .map(|summary| format_metric_value(summary, MetricDisplayKind::Percent))
        .unwrap_or_else(|| "-".to_owned());
    let offset = review
        .landing_offset_abs_m
        .as_ref()
        .map(|summary| format_metric_value(summary, MetricDisplayKind::Meters))
        .unwrap_or_else(|| "-".to_owned());
    let main = format!("fuel {fuel}");
    let sub_html = if show_delta {
        let fuel_delta = metric_delta_value(
            review.fuel_used_pct_of_max.as_ref(),
            baseline.and_then(|item| item.fuel_used_pct_of_max.as_ref()),
        )
        .map(|delta| format_metric_delta_value(delta, MetricDisplayKind::Percent))
        .unwrap_or_else(|| "-".to_owned());
        let offset_delta = metric_delta_value(
            review.landing_offset_abs_m.as_ref(),
            baseline.and_then(|item| item.landing_offset_abs_m.as_ref()),
        )
        .map(|delta| format_metric_delta_value(delta, MetricDisplayKind::Meters))
        .unwrap_or_else(|| "-".to_owned());
        format!(
            r#"<span class="compare-toggle-target">offset {offset} · {fuel_delta} fuel · {offset_delta} off</span><span class="standalone-toggle-target">offset {offset}</span>"#
        )
    } else {
        escape_html(&format!("offset {offset}"))
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&main),
        sub_html
    )
}

fn render_overview_tracking_cell(
    review: &ReviewAggregate,
    baseline: Option<&ReviewAggregate>,
    show_delta: bool,
) -> String {
    let reference = review
        .reference_gap_mean_m
        .as_ref()
        .map(|summary| format_metric_value(summary, MetricDisplayKind::Meters))
        .unwrap_or_else(|| "-".to_owned());
    let sub_html = if show_delta {
        metric_delta_value(
            review.reference_gap_mean_m.as_ref(),
            baseline.and_then(|item| item.reference_gap_mean_m.as_ref()),
        )
        .map(|delta| {
            format!(
                r#"<span class="compare-toggle-target">Δ {}</span><span class="standalone-toggle-target">mean ref deviation</span>"#,
                escape_html(&format_metric_delta_value(delta, MetricDisplayKind::Meters))
            )
        })
        .unwrap_or_else(|| {
            r#"<span class="compare-toggle-target">Δ -</span><span class="standalone-toggle-target">mean ref deviation</span>"#
                .to_owned()
        })
    } else {
        escape_html("mean ref deviation")
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&reference),
        sub_html
    )
}

fn render_overview_efficiency_diff_cell(
    candidate: &ReviewAggregate,
    baseline: Option<&ReviewAggregate>,
) -> String {
    let Some(baseline) = baseline else {
        return r#"<div class="overview-stack"><div class="overview-main">-</div><div class="overview-sub">-</div></div>"#.to_owned();
    };
    let fuel_delta = metric_delta_value(
        candidate.fuel_used_pct_of_max.as_ref(),
        baseline.fuel_used_pct_of_max.as_ref(),
    )
    .map(|delta| format_metric_delta_value(delta, MetricDisplayKind::Percent))
    .unwrap_or_else(|| "-".to_owned());
    let offset_delta = metric_delta_value(
        candidate.landing_offset_abs_m.as_ref(),
        baseline.landing_offset_abs_m.as_ref(),
    )
    .map(|delta| format_metric_delta_value(delta, MetricDisplayKind::Meters))
    .unwrap_or_else(|| "-".to_owned());
    format!(
        r#"<div class="overview-stack"><div class="overview-main">fuel {}</div><div class="overview-sub">offset {}</div></div>"#,
        escape_html(&fuel_delta),
        escape_html(&offset_delta)
    )
}

fn render_overview_tracking_diff_cell(
    candidate: &ReviewAggregate,
    baseline: Option<&ReviewAggregate>,
) -> String {
    let Some(baseline) = baseline else {
        return r#"<div class="overview-stack"><div class="overview-main">-</div><div class="overview-sub">-</div></div>"#.to_owned();
    };
    let delta = metric_delta_value(
        candidate.reference_gap_mean_m.as_ref(),
        baseline.reference_gap_mean_m.as_ref(),
    )
    .map(|delta| format_metric_delta_value(delta, MetricDisplayKind::Meters))
    .unwrap_or_else(|| "-".to_owned());
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">ref deviation delta</div></div>"#,
        escape_html(&delta)
    )
}

fn overview_timing_from_records(records: &[&crate::BatchRunRecord]) -> (f64, f64) {
    if records.is_empty() {
        return (0.0, 0.0);
    }
    let mean = records
        .iter()
        .map(|record| record.manifest.sim_time_s)
        .sum::<f64>()
        / records.len() as f64;
    let max = records
        .iter()
        .map(|record| record.manifest.sim_time_s)
        .fold(0.0_f64, f64::max);
    (mean, max)
}

fn render_wall_clock_chip(candidate: &BatchReport) -> String {
    format!(
        r#"<span class="chip"><strong>wall</strong> {}</span>"#,
        escape_html(&format!("{:.2}s", candidate.wall_clock_s)),
    )
}

fn success_rate_ratio(success_runs: usize, total_runs: usize) -> f64 {
    if total_runs == 0 {
        0.0
    } else {
        success_runs as f64 / total_runs as f64
    }
}

fn batch_report_subtitle(
    candidate: &BatchReport,
    baseline: Option<&BatchReport>,
    comparison: Option<&BatchComparison>,
) -> String {
    if let Some(comparison) = comparison
        && let (Some(candidate_focus), Some(baseline)) =
            (preferred_current_lane_focus(candidate), baseline)
        && let Some(baseline_focus) = preferred_current_lane_focus(baseline)
    {
        let basis = compare_basis_from_records(
            &comparison.basis.mode,
            candidate_focus.records.as_slice(),
            baseline_focus.records.as_slice(),
        );
        return format!(
            "{}. {} total runs captured for this batch; overview below compares {} current runs against {} cached baseline runs.",
            candidate.pack_name,
            candidate.total_runs,
            basis.shared_runs + basis.candidate_only_runs,
            basis.shared_runs + basis.baseline_only_runs,
        );
    }

    if let Some(current_focus) = preferred_current_lane_focus(candidate) {
        let other_runs = candidate
            .total_runs
            .saturating_sub(current_focus.records.len());
        if other_runs > 0 {
            return format!(
                "{}. {} total runs captured for this batch; the page below focuses {} current controller-lane runs while {} other lane or reference runs are excluded from this current-lane view.",
                candidate.pack_name,
                candidate.total_runs,
                current_focus.records.len(),
                other_runs,
            );
        }
    }

    format!(
        "{}. {} total runs captured for this batch.",
        candidate.pack_name, candidate.total_runs
    )
}

fn render_context_table(
    candidate: &BatchReport,
    baseline: Option<&BatchReport>,
    comparison: Option<&BatchComparison>,
) -> String {
    let compare_provenance = &candidate.provenance.compare;
    let (mode, current_source, baseline_source, compare_basis, scope_resolution) = if let Some(
        comparison,
    ) = comparison
    {
        if let (Some(candidate_focus), Some(baseline_report)) =
            (preferred_current_lane_focus(candidate), baseline)
            && let Some(baseline_focus) = preferred_current_lane_focus(baseline_report)
        {
            let candidate_summary = summarize_lane_focus(&candidate_focus);
            let baseline_summary = summarize_lane_focus(&baseline_focus);
            let basis = compare_basis_from_records(
                &comparison.basis.mode,
                candidate_focus.records.as_slice(),
                baseline_focus.records.as_slice(),
            );
            let (scope_label, scope_note) = compare_scope_resolution(&basis);
            (
                "current-lane history compare",
                format!(
                    r#"<div class="context-value"><div class="context-main">current results from lane <code>{}</code> within <code>{}</code></div><div class="context-sub">{} · {} current runs · {}</div></div>"#,
                    escape_html(candidate_summary.lane_id),
                    escape_html(&candidate.pack_id),
                    escape_html(&candidate.pack_name),
                    candidate_summary.run_count,
                    candidate_summary.controller_html,
                ),
                format!(
                    r#"<div class="context-value"><div class="context-main">compare baseline from lane <code>{}</code> within <code>{}</code></div><div class="context-sub">{} · {} baseline runs · {} · {}</div></div>"#,
                    escape_html(baseline_summary.lane_id),
                    escape_html(&baseline_report.pack_id),
                    escape_html(&baseline_report.pack_name),
                    baseline_summary.run_count,
                    baseline_summary.controller_html,
                    escape_html(&baseline_resolution_summary(compare_provenance, true)),
                ),
                context_value(
                    &format!(
                        "current lane_id {} -> compare baseline lane_id {} · shared {} · current-only {} · baseline-only {}",
                        candidate_summary.lane_id,
                        baseline_summary.lane_id,
                        basis.shared_runs,
                        basis.candidate_only_runs,
                        basis.baseline_only_runs
                    ),
                    "baseline here means the compare target, not the built-in baseline controller",
                ),
                context_value(scope_label, scope_note),
            )
        } else {
            let (scope_label, scope_note) = compare_scope_resolution(&comparison.basis);
            (
                "external compare",
                format!(
                    r#"<div class="context-value"><div class="context-main"><code>{}</code> · {}</div><div class="context-sub">spec <code>{}</code> · resolved <code>{}</code></div></div>"#,
                    escape_html(&candidate.pack_id),
                    escape_html(&candidate.pack_name),
                    escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                    escape_html(&short_digest(&candidate.identity.resolved_run_digest)),
                ),
                baseline.map(|baseline| {
                    format!(
                        r#"<div class="context-value"><div class="context-main"><code>{}</code> · {}</div><div class="context-sub">spec <code>{}</code> · resolved <code>{}</code> · {}</div></div>"#,
                        escape_html(&baseline.pack_id),
                        escape_html(&baseline.pack_name),
                        escape_html(&short_digest(&baseline.identity.pack_spec_digest)),
                        escape_html(&short_digest(&baseline.identity.resolved_run_digest)),
                        escape_html(&baseline_resolution_summary(compare_provenance, true)),
                    )
                }).unwrap_or_else(|| {
                    missing_baseline_context_value(compare_provenance)
                }),
                context_value(
                    &format!(
                        "{} · shared {} · current-only {} · baseline-only {}",
                        comparison.basis.mode,
                        comparison.basis.shared_runs,
                        comparison.basis.candidate_only_runs,
                        comparison.basis.baseline_only_runs
                    ),
                    "candidate and baseline runs are paired by compare basis",
                ),
                context_value(
                    scope_label,
                    scope_note,
                ),
            )
        }
    } else {
        if let Some(current_focus) = preferred_current_lane_focus(candidate) {
            let current_summary = summarize_lane_focus(&current_focus);
            (
                "standalone",
                format!(
                    r#"<div class="context-value"><div class="context-main">current controller lane <code>{}</code> within <code>{}</code></div><div class="context-sub">{} · {} current runs · spec <code>{}</code> · resolved <code>{}</code> · {}</div></div>"#,
                    escape_html(current_summary.lane_id),
                    escape_html(&candidate.pack_id),
                    escape_html(&candidate.pack_name),
                    current_summary.run_count,
                    escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                    escape_html(&short_digest(&candidate.identity.resolved_run_digest)),
                    current_summary.controller_html,
                ),
                missing_baseline_context_value(compare_provenance),
                context_none_value("none"),
                context_value(
                    "current controller lane",
                    "the overview and tree focus the preferred current controller lane when no cached history compare is available",
                ),
            )
        } else {
            (
                "standalone",
                format!(
                    r#"<div class="context-value"><div class="context-main"><code>{}</code> · {}</div><div class="context-sub">spec <code>{}</code> · resolved <code>{}</code></div></div>"#,
                    escape_html(&candidate.pack_id),
                    escape_html(&candidate.pack_name),
                    escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                    escape_html(&short_digest(&candidate.identity.resolved_run_digest)),
                ),
                missing_baseline_context_value(compare_provenance),
                context_none_value("none"),
                context_value(
                    "full pack",
                    "the overview and tree reflect the full batch without a comparison basis",
                ),
            )
        }
    };

    let (compare_status_label, compare_status_class, compare_status_note) =
        if let Some(comparison) = comparison {
            if comparison.basis.shared_runs == 0 {
                (
                    "unavailable",
                    "warn",
                    "no shared runs were available for comparison",
                )
            } else if comparison.basis.candidate_only_runs == 0
                && comparison.basis.baseline_only_runs == 0
            {
                (
                    "available",
                    "ok",
                    "full compare available on the resolved run set",
                )
            } else {
                (
                    "partial",
                    "partial",
                    "compare is limited to the shared run intersection",
                )
            }
        } else if compare_provenance.status == BatchCompareResolutionStatus::Missing {
            (
                "missing",
                "warn",
                compare_provenance
                    .note
                    .as_deref()
                    .unwrap_or("requested compare cache was not available"),
            )
        } else {
            (
                "standalone",
                "muted",
                "no history compare was available for this batch page",
            )
        };

    let report_mode_html = format!(
        r#"<div class="context-value"><div class="context-main"><span class="status-chip {}">{}</span></div><div class="context-sub">explicit report provenance for this batch page</div></div>"#,
        compare_status_class,
        escape_html(mode),
    );
    let compare_status_html = format!(
        r#"<div class="context-value"><div class="context-main"><span class="status-chip {}">{}</span></div><div class="context-sub">{}</div></div>"#,
        compare_status_class,
        escape_html(compare_status_label),
        escape_html(compare_status_note),
    );

    format!(
        r#"<section class="header-context">
  <h2>Context</h2>
  <div class="table-wrap">
    <table class="context-table">
      <thead>
        <tr>
          <th>Report Mode</th>
          <th>Current Source</th>
          <th>Baseline Source</th>
          <th>Compare Basis</th>
          <th>Scope Resolution</th>
          <th>Compare Status</th>
          <th>Cache / Promotion</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td>{}</td>
          <td>{}</td>
          <td>{}</td>
          <td>{}</td>
          <td>{}</td>
          <td>{}</td>
          <td>{}</td>
        </tr>
      </tbody>
    </table>
  </div>
</section>"#,
        report_mode_html,
        current_source,
        baseline_source,
        compare_basis,
        scope_resolution,
        compare_status_html,
        render_cache_context(
            candidate.provenance.cache.as_ref(),
            baseline.and_then(|report| report.provenance.cache.as_ref())
        ),
    )
}

fn render_cache_context(
    candidate_cache: Option<&BatchCacheInfo>,
    baseline_cache: Option<&BatchCacheInfo>,
) -> String {
    let mut blocks = Vec::new();
    if let Some(cache) = candidate_cache {
        blocks.push(context_value_html(
            &format!(
                "candidate {}",
                render_cache_status_label(cache.status, cache.promotion.is_some())
            ),
            &format!(
                "workspace <code>{}</code> · commit <code>{}</code> · batch <code>{}</code>{}",
                escape_html(&cache.workspace_key),
                escape_html(&cache.commit_key),
                escape_html(&cache.batch_stem),
                cache
                    .promotion
                    .as_ref()
                    .map(|promotion| format!(
                        " · promoted from <code>{}</code>",
                        escape_html(&promotion.source_workspace_key)
                    ))
                    .unwrap_or_default(),
            ),
        ));
    }
    if let Some(cache) = baseline_cache {
        blocks.push(context_value_html(
            &format!(
                "baseline {}",
                render_cache_status_label(cache.status, cache.promotion.is_some())
            ),
            &format!(
                "workspace <code>{}</code> · commit <code>{}</code> · batch <code>{}</code>{}",
                escape_html(&cache.workspace_key),
                escape_html(&cache.commit_key),
                escape_html(&cache.batch_stem),
                cache
                    .promotion
                    .as_ref()
                    .map(|promotion| format!(
                        " · promoted from <code>{}</code>",
                        escape_html(&promotion.source_workspace_key)
                    ))
                    .unwrap_or_default(),
            ),
        ));
    }
    if blocks.is_empty() {
        context_value(
            "not cached",
            "this batch page was rendered without cache provenance",
        )
    } else {
        blocks.join("")
    }
}

fn render_cache_status_label(status: BatchCacheStatus, promoted: bool) -> &'static str {
    match (status, promoted) {
        (BatchCacheStatus::Fresh, false) => "fresh",
        (BatchCacheStatus::Fresh, true) => "fresh promoted-cache",
        (BatchCacheStatus::Reused, false) => "reused",
        (BatchCacheStatus::Reused, true) => "reused promoted-cache",
        (BatchCacheStatus::Promoted, _) => "promoted",
    }
}

fn baseline_resolution_summary(
    provenance: &crate::BatchCompareProvenance,
    fallback_external: bool,
) -> String {
    match provenance.source {
        BatchCompareSource::ExplicitDir => provenance
            .baseline_dir
            .as_ref()
            .map(|dir| format!("Baseline Resolution: explicit baseline report from {}", dir))
            .unwrap_or_else(|| "Baseline Resolution: explicit baseline report".to_owned()),
        BatchCompareSource::CacheRef => {
            let requested = provenance.requested_ref.as_deref().unwrap_or("auto");
            let resolved = provenance.resolved_ref.as_deref().unwrap_or("unresolved");
            format!(
                "Baseline Resolution: compare cache ref {} -> {}",
                requested, resolved
            )
        }
        BatchCompareSource::None => {
            if fallback_external {
                "Baseline Resolution: external baseline report provided for this render".to_owned()
            } else {
                "Baseline Resolution: none · not applicable for this batch page".to_owned()
            }
        }
    }
}

fn missing_baseline_context_value(provenance: &crate::BatchCompareProvenance) -> String {
    match provenance.source {
        BatchCompareSource::CacheRef
            if provenance.status == BatchCompareResolutionStatus::Missing =>
        {
            context_value(
                "missing compare cache",
                provenance
                    .note
                    .as_deref()
                    .unwrap_or("requested compare cache was not available"),
            )
        }
        BatchCompareSource::ExplicitDir => context_value(
            "explicit baseline pending",
            provenance
                .note
                .as_deref()
                .unwrap_or("baseline report directory will be resolved at render time"),
        ),
        _ => context_value(
            "none",
            "Baseline Resolution: none · not applicable for this batch page",
        ),
    }
}

fn context_value(main: &str, sub: &str) -> String {
    format!(
        r#"<div class="context-value"><div class="context-main">{}</div><div class="context-sub">{}</div></div>"#,
        escape_html(main),
        escape_html(sub),
    )
}

fn context_value_html(main_html: &str, sub_html: &str) -> String {
    format!(
        r#"<div class="context-value"><div class="context-main">{}</div><div class="context-sub">{}</div></div>"#,
        main_html, sub_html,
    )
}

fn context_none_value(label: &str) -> String {
    format!(
        r#"<div class="context-value"><div class="context-main">{}</div><div class="context-sub">not applicable for this batch page</div></div>"#,
        escape_html(label),
    )
}

fn render_overview_table(
    candidate: &BatchReport,
    baseline: Option<&BatchReport>,
    comparison: Option<&BatchComparison>,
    view_controls: &str,
) -> String {
    let candidate_scope = selector_scope_counts(candidate);
    let candidate_records = candidate.records.iter().collect::<Vec<_>>();
    let candidate_review = review_aggregate_from_records(candidate_records.as_slice());
    let baseline_scope = baseline.map(selector_scope_counts);
    let baseline_records = baseline
        .map(|report| report.records.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let baseline_review = (!baseline_records.is_empty())
        .then(|| review_aggregate_from_records(baseline_records.as_slice()));

    let rows = if let (Some(comparison), Some(candidate_focus), Some(baseline_report)) = (
        comparison,
        preferred_current_lane_focus(candidate),
        baseline,
    ) && let Some(baseline_focus) = preferred_current_lane_focus(baseline_report)
    {
        let candidate_summary = summarize_lane_focus(&candidate_focus);
        let baseline_summary = summarize_lane_focus(&baseline_focus);
        let basis = compare_basis_from_records(
            &comparison.basis.mode,
            candidate_focus.records.as_slice(),
            baseline_focus.records.as_slice(),
        );
        let success_rate_delta = success_rate_ratio(
            candidate_summary.review.success_runs,
            candidate_summary.review.total_runs,
        ) - success_rate_ratio(
            baseline_summary.review.success_runs,
            baseline_summary.review.total_runs,
        );

        vec![
            render_overview_row(
                "current-summary-row",
                format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag current">current</span><code>{}</code></div><div class="overview-sub">{} · {}</div></div>"#,
                    escape_html(&candidate.pack_id),
                    escape_html(&candidate.pack_name),
                    candidate_summary.controller_html,
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code> · lane <code>{}</code></div></div>"#,
                    escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                    escape_html(&short_digest(&candidate.identity.resolved_run_digest)),
                    escape_html(candidate_summary.lane_id),
                ),
                render_overview_scope_cell(
                    &candidate_summary.scope,
                    candidate.workers_used,
                    Some(&basis),
                ),
                render_overview_result_cell(
                    candidate_summary.review.success_runs,
                    candidate_summary.review.total_runs,
                    candidate_summary.review.failure_runs,
                    Some(success_rate_delta),
                ),
                render_overview_timing_cell(
                    candidate_summary.mean_sim_time_s,
                    candidate_summary.max_sim_time_s,
                    Some((
                        candidate_summary.mean_sim_time_s - baseline_summary.mean_sim_time_s,
                        candidate_summary.max_sim_time_s - baseline_summary.max_sim_time_s,
                    )),
                ),
                render_overview_efficiency_cell(
                    &candidate_summary.review,
                    Some(&baseline_summary.review),
                    true,
                ),
                render_overview_tracking_cell(
                    &candidate_summary.review,
                    Some(&baseline_summary.review),
                    true,
                ),
            ),
            render_overview_row(
                "baseline-summary-row baseline-row",
                format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag baseline">baseline</span><code>{}</code></div><div class="overview-sub">{} · {}</div></div>"#,
                    escape_html(&baseline_report.pack_id),
                    escape_html(&baseline_report.pack_name),
                    baseline_summary.controller_html,
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code> · lane <code>{}</code></div></div>"#,
                    escape_html(&short_digest(&baseline_report.identity.pack_spec_digest)),
                    escape_html(&short_digest(&baseline_report.identity.resolved_run_digest)),
                    escape_html(baseline_summary.lane_id),
                ),
                render_overview_scope_cell(
                    &baseline_summary.scope,
                    baseline_report.workers_used,
                    None,
                ),
                render_overview_result_cell(
                    baseline_summary.review.success_runs,
                    baseline_summary.review.total_runs,
                    baseline_summary.review.failure_runs,
                    None,
                ),
                render_overview_timing_cell(
                    baseline_summary.mean_sim_time_s,
                    baseline_summary.max_sim_time_s,
                    None,
                ),
                render_overview_efficiency_cell(&baseline_summary.review, None, false),
                render_overview_tracking_cell(&baseline_summary.review, None, false),
            ),
            render_overview_row(
                "diff-summary-row",
                format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag diff">diff</span>current-lane history compare</div><div class="overview-sub">shared {} · current-only {} · baseline-only {}</div></div>"#,
                    basis.shared_runs, basis.candidate_only_runs, basis.baseline_only_runs
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main"><code>{}</code></div><div class="overview-sub"><code>{}</code> -> <code>{}</code></div></div>"#,
                    escape_html(&baseline_report.pack_id),
                    escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                    escape_html(&short_digest(&baseline_report.identity.pack_spec_digest))
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">current results lane <code>{}</code> -> compare baseline lane <code>{}</code></div></div>"#,
                    escape_html(&format!("shared {}", basis.shared_runs)),
                    escape_html(candidate_summary.lane_id),
                    escape_html(baseline_summary.lane_id),
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main {}">{}</div><div class="overview-sub {}">{}</div></div>"#,
                    delta_class(-success_rate_delta),
                    escape_html(&format_percent_delta(success_rate_delta)),
                    delta_class(
                        (candidate_summary.review.failure_runs as i64
                            - baseline_summary.review.failure_runs as i64)
                            as f64
                    ),
                    escape_html(&format_signed_i64(
                        candidate_summary.review.failure_runs as i64
                            - baseline_summary.review.failure_runs as i64
                    ))
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main {}">{}</div><div class="overview-sub">{}</div></div>"#,
                    delta_class(
                        candidate_summary.mean_sim_time_s - baseline_summary.mean_sim_time_s
                    ),
                    escape_html(&format_signed_seconds(
                        candidate_summary.mean_sim_time_s - baseline_summary.mean_sim_time_s
                    )),
                    escape_html(&format!(
                        "max {}",
                        format_signed_seconds(
                            candidate_summary.max_sim_time_s - baseline_summary.max_sim_time_s
                        )
                    ))
                ),
                render_overview_efficiency_diff_cell(
                    &candidate_summary.review,
                    Some(&baseline_summary.review),
                ),
                render_overview_tracking_diff_cell(
                    &candidate_summary.review,
                    Some(&baseline_summary.review),
                ),
            ),
        ]
    } else if let Some(candidate_focus) = preferred_current_lane_focus(candidate) {
        let candidate_summary = summarize_lane_focus(&candidate_focus);
        vec![render_overview_row(
            "current-summary-row",
            format!(
                r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag current">current</span><code>{}</code></div><div class="overview-sub">{} · {}</div></div>"#,
                escape_html(&candidate.pack_id),
                escape_html(&candidate.pack_name),
                candidate_summary.controller_html
            ),
            format!(
                r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code> · lane <code>{}</code></div></div>"#,
                escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                escape_html(&short_digest(&candidate.identity.resolved_run_digest)),
                escape_html(candidate_summary.lane_id)
            ),
            render_overview_scope_cell(&candidate_summary.scope, candidate.workers_used, None),
            render_overview_result_cell(
                candidate_summary.review.success_runs,
                candidate_summary.review.total_runs,
                candidate_summary.review.failure_runs,
                None,
            ),
            render_overview_timing_cell(
                candidate_summary.mean_sim_time_s,
                candidate_summary.max_sim_time_s,
                None,
            ),
            render_overview_efficiency_cell(&candidate_summary.review, None, false),
            render_overview_tracking_cell(&candidate_summary.review, None, false),
        )]
    } else {
        let mut rows = vec![render_overview_row(
            "current-summary-row",
            format!(
                r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag current">current</span><code>{}</code></div><div class="overview-sub">{}</div></div>"#,
                escape_html(&candidate.pack_id),
                escape_html(&candidate.pack_name)
            ),
            format!(
                r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code></div></div>"#,
                escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                escape_html(&short_digest(&candidate.identity.resolved_run_digest))
            ),
            render_overview_scope_cell(
                &candidate_scope,
                candidate.workers_used,
                comparison.map(|cmp| &cmp.basis),
            ),
            render_overview_result_cell(
                candidate.summary.success_runs,
                candidate.summary.total_runs,
                candidate.summary.failure_runs,
                comparison.map(|cmp| cmp.summary.success_rate_delta),
            ),
            render_overview_timing_cell(
                candidate.summary.mean_sim_time_s,
                candidate.summary.max_sim_time_s,
                comparison.map(|cmp| {
                    (
                        cmp.summary.mean_sim_time_delta_s,
                        cmp.summary.max_sim_time_delta_s,
                    )
                }),
            ),
            render_overview_efficiency_cell(
                &candidate_review,
                baseline_review.as_ref(),
                comparison.is_some(),
            ),
            render_overview_tracking_cell(
                &candidate_review,
                baseline_review.as_ref(),
                comparison.is_some(),
            ),
        )];

        if let Some(baseline) = baseline {
            let baseline_scope = baseline_scope.expect("baseline scope");
            let baseline_review = baseline_review.as_ref().expect("baseline review");
            rows.push(render_overview_row(
                "baseline-summary-row baseline-row",
                format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag baseline">baseline</span><code>{}</code></div><div class="overview-sub">{}</div></div>"#,
                    escape_html(&baseline.pack_id),
                    escape_html(&baseline.pack_name)
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code></div></div>"#,
                    escape_html(&short_digest(&baseline.identity.pack_spec_digest)),
                    escape_html(&short_digest(&baseline.identity.resolved_run_digest))
                ),
                render_overview_scope_cell(&baseline_scope, baseline.workers_used, None),
                render_overview_result_cell(
                    baseline.summary.success_runs,
                    baseline.summary.total_runs,
                    baseline.summary.failure_runs,
                    None,
                ),
                render_overview_timing_cell(
                    baseline.summary.mean_sim_time_s,
                    baseline.summary.max_sim_time_s,
                    None,
                ),
                render_overview_efficiency_cell(baseline_review, None, false),
                render_overview_tracking_cell(baseline_review, None, false),
            ));
        }

        if let Some(comparison) = comparison {
            rows.push(render_overview_row(
                "diff-summary-row",
                format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag diff">diff</span>{}</div><div class="overview-sub">shared {} · current-only {} · baseline-only {}</div></div>"#,
                    escape_html(&comparison.basis.mode),
                    comparison.basis.shared_runs,
                    comparison.basis.candidate_only_runs,
                    comparison.basis.baseline_only_runs
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main"><code>{}</code></div><div class="overview-sub"><code>{}</code> -> <code>{}</code></div></div>"#,
                    escape_html(&comparison.baseline_pack_id),
                    escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                    escape_html(
                        &baseline
                            .map(|report| short_digest(&report.identity.pack_spec_digest))
                            .unwrap_or_else(|| "-".to_owned())
                    )
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">candidate-only {} · baseline-only {}</div></div>"#,
                    escape_html(&format!("shared {}", comparison.basis.shared_runs)),
                    comparison.basis.candidate_only_runs,
                    comparison.basis.baseline_only_runs
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main {}">{}</div><div class="overview-sub {}">{}</div></div>"#,
                    delta_class(-comparison.summary.success_rate_delta),
                    escape_html(&format_percent_delta(comparison.summary.success_rate_delta)),
                    delta_class(comparison.summary.failure_runs_delta as f64),
                    escape_html(&format_signed_i64(comparison.summary.failure_runs_delta))
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main {}">{}</div><div class="overview-sub">{}</div></div>"#,
                    delta_class(comparison.summary.mean_sim_time_delta_s),
                    escape_html(&format_signed_seconds(comparison.summary.mean_sim_time_delta_s)),
                    escape_html(&format!(
                        "max {}",
                        format_signed_seconds(comparison.summary.max_sim_time_delta_s)
                    ))
                ),
                render_overview_efficiency_diff_cell(&candidate_review, baseline_review.as_ref()),
                render_overview_tracking_diff_cell(&candidate_review, baseline_review.as_ref()),
            ));
        }
        rows
    };

    format!(
        r#"<section class="header-overview">
  <div class="section-head">
    <h2>Overview</h2>
    {view_controls}
  </div>
  <div class="table-wrap">
    <table class="summary-table">
      <thead>
        <tr>
          <th>Pack</th>
          <th>Ref</th>
          <th>Scope</th>
          <th>Result</th>
          <th>Timing</th>
          <th>Efficiency</th>
          <th>Tracking</th>
        </tr>
      </thead>
      <tbody>{}</tbody>
    </table>
  </div>
</section>"#,
        rows.join(""),
        view_controls = view_controls,
    )
}

fn render_tree_controls(has_compare: bool) -> String {
    let mut buttons = vec![
        r#"<button type="button" data-tree-action="expand-depth">Expand</button>"#.to_owned(),
        r#"<button type="button" data-tree-action="collapse-depth">Collapse</button>"#.to_owned(),
        r#"<button type="button" data-tree-action="expand-seeds">Expand Seeds</button>"#.to_owned(),
        r#"<button type="button" data-tree-action="collapse-seeds">Collapse Seeds</button>"#
            .to_owned(),
    ];
    if has_compare {
        buttons.push(
            r#"<button type="button" class="compare-only-control" data-tree-action="toggle-baseline">Hide Baseline</button>"#
                .to_owned(),
        );
        buttons.push(
            r#"<button type="button" class="compare-only-control" data-tree-action="toggle-diff">Show Changed Only</button>"#
                .to_owned(),
        );
    }
    format!(r#"<div class="tree-controls">{}</div>"#, buttons.join(""))
}

fn render_view_controls(has_compare_view: bool) -> String {
    if !has_compare_view {
        return String::new();
    }
    r#"<div class="view-mode-controls">
  <button type="button" class="active" data-view-mode="compare">Compare View</button>
  <button type="button" data-view-mode="current-only">Current Only</button>
</div>"#
        .to_owned()
}

const UNSPECIFIED_SELECTOR_VALUE: &str = "unspecified";

#[derive(Clone, Debug)]
struct ReviewAggregate {
    total_runs: usize,
    success_runs: usize,
    failure_runs: usize,
    sim_time_stats: Option<crate::BatchMetricSummary>,
    fuel_used_pct_of_max: Option<crate::BatchMetricSummary>,
    landing_offset_abs_m: Option<crate::BatchMetricSummary>,
    reference_gap_mean_m: Option<crate::BatchMetricSummary>,
    failed_seeds: Vec<u64>,
}

type LaneRecordGroups<'a> = BTreeMap<String, Vec<&'a crate::BatchRunRecord>>;
type VelocityRecordGroups<'a> = BTreeMap<String, LaneRecordGroups<'a>>;
type ArcRecordGroups<'a> = BTreeMap<String, VelocityRecordGroups<'a>>;
type VehicleRecordGroups<'a> = BTreeMap<String, ArcRecordGroups<'a>>;
type ConditionRecordGroups<'a> = BTreeMap<String, VehicleRecordGroups<'a>>;
type ArrivalRecordGroups<'a> = BTreeMap<String, ConditionRecordGroups<'a>>;
type MissionRecordGroups<'a> = BTreeMap<String, ArrivalRecordGroups<'a>>;

fn render_review_tree(
    candidate: &BatchReport,
    baseline: Option<&BatchReport>,
    comparison: Option<&BatchComparison>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
) -> String {
    let (candidate_tree, baseline_tree) = if comparison.is_some() {
        if let (Some(candidate_focus), Some(baseline_report)) =
            (preferred_current_lane_focus(candidate), baseline)
        {
            if let Some(baseline_focus) = preferred_current_lane_focus(baseline_report) {
                (
                    records_by_selector_hierarchy_from_records(candidate_focus.records.as_slice()),
                    records_by_selector_hierarchy_from_records(baseline_focus.records.as_slice()),
                )
            } else {
                (
                    records_by_selector_hierarchy(candidate),
                    baseline
                        .map(records_by_selector_hierarchy)
                        .unwrap_or_default(),
                )
            }
        } else {
            (
                records_by_selector_hierarchy(candidate),
                baseline
                    .map(records_by_selector_hierarchy)
                    .unwrap_or_default(),
            )
        }
    } else if let Some(candidate_focus) = preferred_current_lane_focus(candidate) {
        (
            records_by_selector_hierarchy_from_records(candidate_focus.records.as_slice()),
            MissionRecordGroups::new(),
        )
    } else {
        (
            records_by_selector_hierarchy(candidate),
            baseline
                .map(records_by_selector_hierarchy)
                .unwrap_or_default(),
        )
    };
    if candidate_tree.is_empty() && baseline_tree.is_empty() {
        return r#"<p class="muted">No batch records available.</p>"#.to_owned();
    }
    let run_change_map = comparison_change_map(comparison);

    let mission_keys = merged_map_keys(&candidate_tree, Some(&baseline_tree));
    let sections = mission_keys
        .iter()
        .map(|mission| {
            render_mission_review_section(
                mission,
                candidate_tree.get(mission),
                baseline_tree.get(mission),
                &run_change_map,
                comparison,
                output_dir,
                candidate_record_map,
                baseline_record_map,
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(r#"<div class="tree-stack">{sections}</div>"#)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum TreeRowTone {
    Current,
    Baseline,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SummaryMetricStyle {
    MeanStddev,
    MeanDelta,
}

fn render_mission_review_section(
    mission: &str,
    candidate_arrivals: Option<&ArrivalRecordGroups<'_>>,
    baseline_arrivals: Option<&ArrivalRecordGroups<'_>>,
    run_change_map: &BTreeMap<String, (&'static str, &'static str)>,
    comparison: Option<&BatchComparison>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
) -> String {
    let candidate_records = flatten_arrival_records(candidate_arrivals);
    let baseline_records = flatten_arrival_records(baseline_arrivals);
    let aggregate = (!candidate_records.is_empty())
        .then(|| review_aggregate_from_records(candidate_records.as_slice()));
    let baseline_aggregate = (!baseline_records.is_empty())
        .then(|| review_aggregate_from_records(baseline_records.as_slice()));
    let lane_current_aggregate = preferred_current_lane_aggregate(candidate_records.as_slice());
    let lane_baseline_aggregate =
        controller_lane_aggregate(candidate_records.as_slice(), "baseline");
    let split_by_lane = comparison.is_none()
        && lane_current_aggregate.is_some()
        && lane_baseline_aggregate.is_some();
    let current_row_aggregate = if split_by_lane {
        lane_current_aggregate.as_ref()
    } else {
        aggregate.as_ref()
    };
    let baseline_row_aggregate = if comparison.is_some() {
        baseline_aggregate.as_ref()
    } else if split_by_lane {
        lane_baseline_aggregate.as_ref()
    } else {
        None
    };
    let regression_count = count_regressions(comparison, |row| row.selector.mission == mission);
    let changed =
        comparison.is_some() && aggregate_changed(current_row_aggregate, baseline_row_aggregate);
    let empty_candidate = ArrivalRecordGroups::new();
    let empty_baseline = ArrivalRecordGroups::new();
    let candidate_arrivals = candidate_arrivals.unwrap_or(&empty_candidate);
    let baseline_arrivals = baseline_arrivals.unwrap_or(&empty_baseline);
    let arrival_keys = merged_map_keys(candidate_arrivals, Some(baseline_arrivals));
    let group_id = tree_group_id(&["mission", mission]);
    let arrival_rows = arrival_keys
        .iter()
        .map(|arrival_family| {
            render_arrival_review_section(
                mission,
                arrival_family,
                candidate_arrivals.get(arrival_family),
                baseline_arrivals.get(arrival_family),
                run_change_map,
                comparison,
                output_dir,
                candidate_record_map,
                baseline_record_map,
                1,
                Some(group_id.as_str()),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let mut rows = String::new();
    let current_note = render_summary_note(
        comparison.is_some(),
        TreeRowTone::Current,
        comparison.is_some().then_some(regression_count),
        None,
        current_row_aggregate.is_some(),
        baseline_row_aggregate.is_some(),
    );
    rows.push_str(&render_summary_row(
        mission,
        0,
        None,
        (!arrival_keys.is_empty()).then_some(group_id.as_str()),
        "mission",
        current_row_aggregate,
        baseline_row_aggregate,
        SummaryMetricStyle::MeanDelta,
        changed,
        current_note.as_str(),
        baseline_row_aggregate.is_some().then_some("cur"),
        TreeRowTone::Current,
    ));
    rows.push_str(&arrival_rows);

    format!(
        r#"<section class="tree-table-section">
  <div class="table-heading">
    <h3><code>{mission}</code></h3>
    <div class="section-meta">{success_rate} · {failure_count} fail · {fuel_used} fuel · {mean_sim} flight · {landing_offset} off · {reference_gap} ref</div>
  </div>
  <div class="table-wrap">
    <table class="scenario-table" data-tree-table="{table_id}">
      <thead>
        <tr>
          <th>Selector</th>
          <th>Success / Outcome</th>
          <th>Fuel Used</th>
          <th>Flight Time</th>
          <th>Landing Offset</th>
          <th>Ref Dev</th>
          <th>Preview</th>
        </tr>
      </thead>
      <tbody>{rows}</tbody>
    </table>
  </div>
</section>"#,
        mission = escape_html(mission),
        table_id = escape_html(&group_id),
        success_rate = aggregate
            .as_ref()
            .map(|item| inline_rate_text(item.success_runs, item.total_runs))
            .unwrap_or_else(|| "-".to_owned()),
        failure_count = aggregate
            .as_ref()
            .map(|item| item.failure_runs)
            .unwrap_or(0),
        fuel_used = format_metric_summary(
            aggregate
                .as_ref()
                .and_then(|item| item.fuel_used_pct_of_max.as_ref()),
            MetricDisplayKind::Percent
        ),
        mean_sim = format_metric_summary(
            aggregate
                .as_ref()
                .and_then(|item| item.sim_time_stats.as_ref()),
            MetricDisplayKind::Seconds
        ),
        landing_offset = format_metric_summary(
            aggregate
                .as_ref()
                .and_then(|item| item.landing_offset_abs_m.as_ref()),
            MetricDisplayKind::Meters
        ),
        reference_gap = format_metric_summary(
            aggregate
                .as_ref()
                .and_then(|item| item.reference_gap_mean_m.as_ref()),
            MetricDisplayKind::Meters
        ),
        rows = rows,
    )
}

fn render_arrival_review_section(
    mission: &str,
    arrival_family: &str,
    candidate_conditions: Option<&ConditionRecordGroups<'_>>,
    baseline_conditions: Option<&ConditionRecordGroups<'_>>,
    run_change_map: &BTreeMap<String, (&'static str, &'static str)>,
    comparison: Option<&BatchComparison>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let candidate_records = flatten_condition_records(candidate_conditions);
    let baseline_records = flatten_condition_records(baseline_conditions);
    let aggregate = (!candidate_records.is_empty())
        .then(|| review_aggregate_from_records(candidate_records.as_slice()));
    let baseline_aggregate = (!baseline_records.is_empty())
        .then(|| review_aggregate_from_records(baseline_records.as_slice()));
    let lane_current_aggregate = preferred_current_lane_aggregate(candidate_records.as_slice());
    let lane_baseline_aggregate =
        controller_lane_aggregate(candidate_records.as_slice(), "baseline");
    let split_by_lane = comparison.is_none()
        && lane_current_aggregate.is_some()
        && lane_baseline_aggregate.is_some();
    let current_row_aggregate = if split_by_lane {
        lane_current_aggregate.as_ref()
    } else {
        aggregate.as_ref()
    };
    let baseline_row_aggregate = if comparison.is_some() {
        baseline_aggregate.as_ref()
    } else if split_by_lane {
        lane_baseline_aggregate.as_ref()
    } else {
        None
    };
    let regression_count = count_regressions(comparison, |row| {
        row.selector.mission == mission && row.selector.arrival_family == arrival_family
    });
    let changed =
        comparison.is_some() && aggregate_changed(current_row_aggregate, baseline_row_aggregate);
    let empty_candidate = ConditionRecordGroups::new();
    let empty_baseline = ConditionRecordGroups::new();
    let candidate_conditions = candidate_conditions.unwrap_or(&empty_candidate);
    let baseline_conditions = baseline_conditions.unwrap_or(&empty_baseline);
    let condition_keys = merged_map_keys(candidate_conditions, Some(baseline_conditions));
    let group_id = tree_group_id(&["arrival", mission, arrival_family]);
    let condition_rows = condition_keys
        .iter()
        .map(|condition_set| {
            render_condition_review_section(
                mission,
                arrival_family,
                condition_set,
                candidate_conditions.get(condition_set),
                baseline_conditions.get(condition_set),
                run_change_map,
                comparison,
                output_dir,
                candidate_record_map,
                baseline_record_map,
                depth + 1,
                Some(group_id.as_str()),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let mut rows = String::new();
    let current_note = render_summary_note(
        comparison.is_some(),
        TreeRowTone::Current,
        comparison.is_some().then_some(regression_count),
        None,
        current_row_aggregate.is_some(),
        baseline_row_aggregate.is_some(),
    );
    rows.push_str(&render_summary_row(
        arrival_family,
        depth,
        parent_group_id,
        (!condition_keys.is_empty()).then_some(group_id.as_str()),
        "arrival",
        current_row_aggregate,
        baseline_row_aggregate,
        SummaryMetricStyle::MeanDelta,
        changed,
        current_note.as_str(),
        baseline_row_aggregate.is_some().then_some("cur"),
        TreeRowTone::Current,
    ));
    rows.push_str(&condition_rows);
    rows
}

fn render_condition_review_section(
    mission: &str,
    arrival_family: &str,
    condition_set: &str,
    candidate_vehicles: Option<&VehicleRecordGroups<'_>>,
    baseline_vehicles: Option<&VehicleRecordGroups<'_>>,
    run_change_map: &BTreeMap<String, (&'static str, &'static str)>,
    comparison: Option<&BatchComparison>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let candidate_records = flatten_vehicle_records(candidate_vehicles);
    let baseline_records = flatten_vehicle_records(baseline_vehicles);
    let aggregate = (!candidate_records.is_empty())
        .then(|| review_aggregate_from_records(candidate_records.as_slice()));
    let baseline_aggregate = (!baseline_records.is_empty())
        .then(|| review_aggregate_from_records(baseline_records.as_slice()));
    let lane_current_aggregate = preferred_current_lane_aggregate(candidate_records.as_slice());
    let lane_baseline_aggregate =
        controller_lane_aggregate(candidate_records.as_slice(), "baseline");
    let split_by_lane = comparison.is_none()
        && lane_current_aggregate.is_some()
        && lane_baseline_aggregate.is_some();
    let current_row_aggregate = if split_by_lane {
        lane_current_aggregate.as_ref()
    } else {
        aggregate.as_ref()
    };
    let baseline_row_aggregate = if comparison.is_some() {
        baseline_aggregate.as_ref()
    } else if split_by_lane {
        lane_baseline_aggregate.as_ref()
    } else {
        None
    };
    let regression_count = count_regressions(comparison, |row| {
        row.selector.mission == mission
            && row.selector.arrival_family == arrival_family
            && row.selector.condition_set == condition_set
    });
    let changed =
        comparison.is_some() && aggregate_changed(current_row_aggregate, baseline_row_aggregate);
    let empty_candidate = VehicleRecordGroups::new();
    let empty_baseline = VehicleRecordGroups::new();
    let candidate_vehicles = candidate_vehicles.unwrap_or(&empty_candidate);
    let baseline_vehicles = baseline_vehicles.unwrap_or(&empty_baseline);
    let vehicle_keys = merged_map_keys(candidate_vehicles, Some(baseline_vehicles));
    let group_id = tree_group_id(&["condition", mission, arrival_family, condition_set]);
    let vehicle_rows = vehicle_keys
        .iter()
        .map(|vehicle_variant| {
            render_vehicle_review_section(
                mission,
                arrival_family,
                condition_set,
                vehicle_variant,
                candidate_vehicles.get(vehicle_variant),
                baseline_vehicles.get(vehicle_variant),
                run_change_map,
                comparison,
                output_dir,
                candidate_record_map,
                baseline_record_map,
                depth + 1,
                Some(group_id.as_str()),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let mut rows = String::new();
    let current_note = render_summary_note(
        comparison.is_some(),
        TreeRowTone::Current,
        comparison.is_some().then_some(regression_count),
        None,
        current_row_aggregate.is_some(),
        baseline_row_aggregate.is_some(),
    );
    rows.push_str(&render_summary_row(
        condition_set,
        depth,
        parent_group_id,
        (!vehicle_keys.is_empty()).then_some(group_id.as_str()),
        "condition",
        current_row_aggregate,
        baseline_row_aggregate,
        SummaryMetricStyle::MeanDelta,
        changed,
        current_note.as_str(),
        baseline_row_aggregate.is_some().then_some("cur"),
        TreeRowTone::Current,
    ));
    rows.push_str(&vehicle_rows);
    rows
}

fn render_vehicle_review_section(
    mission: &str,
    arrival_family: &str,
    condition_set: &str,
    vehicle_variant: &str,
    candidate_arcs: Option<&ArcRecordGroups<'_>>,
    baseline_arcs: Option<&ArcRecordGroups<'_>>,
    run_change_map: &BTreeMap<String, (&'static str, &'static str)>,
    comparison: Option<&BatchComparison>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let candidate_records = flatten_arc_records(candidate_arcs);
    let baseline_records = flatten_arc_records(baseline_arcs);
    let aggregate = (!candidate_records.is_empty())
        .then(|| review_aggregate_from_records(candidate_records.as_slice()));
    let baseline_aggregate = (!baseline_records.is_empty())
        .then(|| review_aggregate_from_records(baseline_records.as_slice()));
    let lane_current_aggregate = preferred_current_lane_aggregate(candidate_records.as_slice());
    let lane_baseline_aggregate =
        controller_lane_aggregate(candidate_records.as_slice(), "baseline");
    let split_by_lane = comparison.is_none()
        && lane_current_aggregate.is_some()
        && lane_baseline_aggregate.is_some();
    let current_row_aggregate = if split_by_lane {
        lane_current_aggregate.as_ref()
    } else {
        aggregate.as_ref()
    };
    let baseline_row_aggregate = if comparison.is_some() {
        baseline_aggregate.as_ref()
    } else if split_by_lane {
        lane_baseline_aggregate.as_ref()
    } else {
        None
    };
    let regression_count = count_regressions(comparison, |row| {
        row.selector.mission == mission
            && row.selector.arrival_family == arrival_family
            && row.selector.condition_set == condition_set
            && row.selector.vehicle_variant == vehicle_variant
    });
    let changed =
        comparison.is_some() && aggregate_changed(current_row_aggregate, baseline_row_aggregate);
    let expectation = aggregate
        .as_ref()
        .and_then(|_| expectation_tier(candidate_records.as_slice()))
        .or_else(|| expectation_tier(baseline_records.as_slice()));
    let empty_candidate = ArcRecordGroups::new();
    let empty_baseline = ArcRecordGroups::new();
    let candidate_arcs = candidate_arcs.unwrap_or(&empty_candidate);
    let baseline_arcs = baseline_arcs.unwrap_or(&empty_baseline);
    let arc_keys = merged_map_keys(candidate_arcs, Some(baseline_arcs));
    let render_matrix_axes = has_meaningful_selector_keys(arc_keys.as_slice());
    let group_id = tree_group_id(&[
        "vehicle",
        mission,
        arrival_family,
        condition_set,
        vehicle_variant,
    ]);
    let child_rows = if render_matrix_axes {
        arc_keys
            .iter()
            .map(|arc_point| {
                render_arc_review_section(
                    mission,
                    arrival_family,
                    condition_set,
                    vehicle_variant,
                    arc_point,
                    candidate_arcs.get(arc_point),
                    baseline_arcs.get(arc_point),
                    run_change_map,
                    comparison,
                    output_dir,
                    candidate_record_map,
                    baseline_record_map,
                    depth + 1,
                    Some(group_id.as_str()),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        let empty_candidate_lanes = LaneRecordGroups::new();
        let empty_baseline_lanes = LaneRecordGroups::new();
        let candidate_lanes = extract_default_lane_groups_from_arcs(Some(candidate_arcs))
            .unwrap_or(&empty_candidate_lanes);
        let baseline_lanes = extract_default_lane_groups_from_arcs(Some(baseline_arcs))
            .unwrap_or(&empty_baseline_lanes);
        let mut lane_keys = merged_map_keys(candidate_lanes, Some(baseline_lanes));
        sort_lane_keys(&mut lane_keys);
        lane_keys
            .iter()
            .map(|lane_id| {
                render_lane_review_section(
                    mission,
                    arrival_family,
                    condition_set,
                    vehicle_variant,
                    None,
                    None,
                    lane_id,
                    candidate_lanes.get(lane_id),
                    baseline_lanes.get(lane_id),
                    run_change_map,
                    comparison,
                    output_dir,
                    candidate_record_map,
                    baseline_record_map,
                    depth + 1,
                    Some(group_id.as_str()),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    let mut rows = String::new();
    let current_note = render_summary_note(
        comparison.is_some(),
        TreeRowTone::Current,
        comparison.is_some().then_some(regression_count),
        expectation.as_deref(),
        current_row_aggregate.is_some(),
        baseline_row_aggregate.is_some(),
    );
    rows.push_str(&render_summary_row(
        vehicle_variant,
        depth,
        parent_group_id,
        ((!arc_keys.is_empty()) || !child_rows.is_empty()).then_some(group_id.as_str()),
        "vehicle",
        current_row_aggregate,
        baseline_row_aggregate,
        SummaryMetricStyle::MeanDelta,
        changed,
        current_note.as_str(),
        baseline_row_aggregate.is_some().then_some("cur"),
        TreeRowTone::Current,
    ));
    rows.push_str(&child_rows);
    rows
}

fn render_arc_review_section(
    mission: &str,
    arrival_family: &str,
    condition_set: &str,
    vehicle_variant: &str,
    arc_point: &str,
    candidate_velocities: Option<&VelocityRecordGroups<'_>>,
    baseline_velocities: Option<&VelocityRecordGroups<'_>>,
    run_change_map: &BTreeMap<String, (&'static str, &'static str)>,
    comparison: Option<&BatchComparison>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let candidate_records = flatten_velocity_records(candidate_velocities);
    let baseline_records = flatten_velocity_records(baseline_velocities);
    let aggregate = (!candidate_records.is_empty())
        .then(|| review_aggregate_from_records(candidate_records.as_slice()));
    let baseline_aggregate = (!baseline_records.is_empty())
        .then(|| review_aggregate_from_records(baseline_records.as_slice()));
    let lane_current_aggregate = preferred_current_lane_aggregate(candidate_records.as_slice());
    let lane_baseline_aggregate =
        controller_lane_aggregate(candidate_records.as_slice(), "baseline");
    let split_by_lane = comparison.is_none()
        && lane_current_aggregate.is_some()
        && lane_baseline_aggregate.is_some();
    let current_row_aggregate = if split_by_lane {
        lane_current_aggregate.as_ref()
    } else {
        aggregate.as_ref()
    };
    let baseline_row_aggregate = if comparison.is_some() {
        baseline_aggregate.as_ref()
    } else if split_by_lane {
        lane_baseline_aggregate.as_ref()
    } else {
        None
    };
    let regression_count = count_regressions(comparison, |row| {
        row.selector.mission == mission
            && row.selector.arrival_family == arrival_family
            && row.selector.condition_set == condition_set
            && row.selector.vehicle_variant == vehicle_variant
            && row.selector.arc_point == arc_point
    });
    let changed =
        comparison.is_some() && aggregate_changed(current_row_aggregate, baseline_row_aggregate);
    let empty_candidate = VelocityRecordGroups::new();
    let empty_baseline = VelocityRecordGroups::new();
    let candidate_velocities = candidate_velocities.unwrap_or(&empty_candidate);
    let baseline_velocities = baseline_velocities.unwrap_or(&empty_baseline);
    let velocity_keys = merged_map_keys(candidate_velocities, Some(baseline_velocities));
    let render_velocity_axes = has_meaningful_selector_keys(velocity_keys.as_slice());
    let group_id = tree_group_id(&[
        "arc",
        mission,
        arrival_family,
        condition_set,
        vehicle_variant,
        arc_point,
    ]);
    let child_rows = if render_velocity_axes {
        velocity_keys
            .iter()
            .map(|velocity_band| {
                render_velocity_review_section(
                    mission,
                    arrival_family,
                    condition_set,
                    vehicle_variant,
                    arc_point,
                    velocity_band,
                    candidate_velocities.get(velocity_band),
                    baseline_velocities.get(velocity_band),
                    run_change_map,
                    comparison,
                    output_dir,
                    candidate_record_map,
                    baseline_record_map,
                    depth + 1,
                    Some(group_id.as_str()),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        let empty_candidate_lanes = LaneRecordGroups::new();
        let empty_baseline_lanes = LaneRecordGroups::new();
        let candidate_lanes = candidate_velocities
            .get(UNSPECIFIED_SELECTOR_VALUE)
            .unwrap_or(&empty_candidate_lanes);
        let baseline_lanes = baseline_velocities
            .get(UNSPECIFIED_SELECTOR_VALUE)
            .unwrap_or(&empty_baseline_lanes);
        let mut lane_keys = merged_map_keys(candidate_lanes, Some(baseline_lanes));
        sort_lane_keys(&mut lane_keys);
        lane_keys
            .iter()
            .map(|lane_id| {
                render_lane_review_section(
                    mission,
                    arrival_family,
                    condition_set,
                    vehicle_variant,
                    Some(arc_point),
                    None,
                    lane_id,
                    candidate_lanes.get(lane_id),
                    baseline_lanes.get(lane_id),
                    run_change_map,
                    comparison,
                    output_dir,
                    candidate_record_map,
                    baseline_record_map,
                    depth + 1,
                    Some(group_id.as_str()),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    let mut rows = String::new();
    let current_note = render_summary_note(
        comparison.is_some(),
        TreeRowTone::Current,
        comparison.is_some().then_some(regression_count),
        None,
        current_row_aggregate.is_some(),
        baseline_row_aggregate.is_some(),
    );
    rows.push_str(&render_summary_row(
        arc_point,
        depth,
        parent_group_id,
        ((!velocity_keys.is_empty()) || !child_rows.is_empty()).then_some(group_id.as_str()),
        "arc",
        current_row_aggregate,
        baseline_row_aggregate,
        SummaryMetricStyle::MeanDelta,
        changed,
        current_note.as_str(),
        baseline_row_aggregate.is_some().then_some("cur"),
        TreeRowTone::Current,
    ));
    rows.push_str(&child_rows);
    rows
}

fn render_velocity_review_section(
    mission: &str,
    arrival_family: &str,
    condition_set: &str,
    vehicle_variant: &str,
    arc_point: &str,
    velocity_band: &str,
    candidate_lanes: Option<&LaneRecordGroups<'_>>,
    baseline_lanes: Option<&LaneRecordGroups<'_>>,
    run_change_map: &BTreeMap<String, (&'static str, &'static str)>,
    comparison: Option<&BatchComparison>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let candidate_records = flatten_lane_records(candidate_lanes);
    let baseline_records = flatten_lane_records(baseline_lanes);
    let aggregate = (!candidate_records.is_empty())
        .then(|| review_aggregate_from_records(candidate_records.as_slice()));
    let baseline_aggregate = (!baseline_records.is_empty())
        .then(|| review_aggregate_from_records(baseline_records.as_slice()));
    let lane_current_aggregate = preferred_current_lane_aggregate(candidate_records.as_slice());
    let lane_baseline_aggregate =
        controller_lane_aggregate(candidate_records.as_slice(), "baseline");
    let split_by_lane = comparison.is_none()
        && lane_current_aggregate.is_some()
        && lane_baseline_aggregate.is_some();
    let current_row_aggregate = if split_by_lane {
        lane_current_aggregate.as_ref()
    } else {
        aggregate.as_ref()
    };
    let baseline_row_aggregate = if comparison.is_some() {
        baseline_aggregate.as_ref()
    } else if split_by_lane {
        lane_baseline_aggregate.as_ref()
    } else {
        None
    };
    let regression_count = count_regressions(comparison, |row| {
        row.selector.mission == mission
            && row.selector.arrival_family == arrival_family
            && row.selector.condition_set == condition_set
            && row.selector.vehicle_variant == vehicle_variant
            && row.selector.arc_point == arc_point
            && row.selector.velocity_band == velocity_band
    });
    let changed =
        comparison.is_some() && aggregate_changed(current_row_aggregate, baseline_row_aggregate);
    let empty_candidate = LaneRecordGroups::new();
    let empty_baseline = LaneRecordGroups::new();
    let candidate_lanes = candidate_lanes.unwrap_or(&empty_candidate);
    let baseline_lanes = baseline_lanes.unwrap_or(&empty_baseline);
    let mut lane_keys = merged_map_keys(candidate_lanes, Some(baseline_lanes));
    sort_lane_keys(&mut lane_keys);
    let group_id = tree_group_id(&[
        "band",
        mission,
        arrival_family,
        condition_set,
        vehicle_variant,
        arc_point,
        velocity_band,
    ]);
    let lane_rows = lane_keys
        .iter()
        .map(|lane_id| {
            render_lane_review_section(
                mission,
                arrival_family,
                condition_set,
                vehicle_variant,
                Some(arc_point),
                Some(velocity_band),
                lane_id,
                candidate_lanes.get(lane_id),
                baseline_lanes.get(lane_id),
                run_change_map,
                comparison,
                output_dir,
                candidate_record_map,
                baseline_record_map,
                depth + 1,
                Some(group_id.as_str()),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let mut rows = String::new();
    let current_note = render_summary_note(
        comparison.is_some(),
        TreeRowTone::Current,
        comparison.is_some().then_some(regression_count),
        None,
        current_row_aggregate.is_some(),
        baseline_row_aggregate.is_some(),
    );
    rows.push_str(&render_summary_row(
        velocity_band,
        depth,
        parent_group_id,
        (!lane_keys.is_empty()).then_some(group_id.as_str()),
        "band",
        current_row_aggregate,
        baseline_row_aggregate,
        SummaryMetricStyle::MeanDelta,
        changed,
        current_note.as_str(),
        baseline_row_aggregate.is_some().then_some("cur"),
        TreeRowTone::Current,
    ));
    rows.push_str(&lane_rows);
    rows
}

fn render_lane_review_section(
    mission: &str,
    arrival_family: &str,
    condition_set: &str,
    vehicle_variant: &str,
    arc_point: Option<&str>,
    velocity_band: Option<&str>,
    lane_id: &str,
    candidate_records: Option<&Vec<&crate::BatchRunRecord>>,
    baseline_records: Option<&Vec<&crate::BatchRunRecord>>,
    run_change_map: &BTreeMap<String, (&'static str, &'static str)>,
    comparison: Option<&BatchComparison>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let candidate_records = candidate_records.cloned().unwrap_or_default();
    let baseline_records = baseline_records.cloned().unwrap_or_default();
    let aggregate = (!candidate_records.is_empty())
        .then(|| review_aggregate_from_records(candidate_records.as_slice()));
    let baseline_aggregate = (!baseline_records.is_empty())
        .then(|| review_aggregate_from_records(baseline_records.as_slice()));
    let regression_count = count_regressions(comparison, |row| {
        row.selector.mission == mission
            && row.selector.arrival_family == arrival_family
            && row.selector.condition_set == condition_set
            && row.selector.vehicle_variant == vehicle_variant
            && arc_point
                .map(|value| row.selector.arc_point == value)
                .unwrap_or(true)
            && velocity_band
                .map(|value| row.selector.velocity_band == value)
                .unwrap_or(true)
            && row.lane_id == lane_id
    });
    let changed = aggregate_changed(aggregate.as_ref(), baseline_aggregate.as_ref());
    let mut group_parts = vec![
        "lane",
        mission,
        arrival_family,
        condition_set,
        vehicle_variant,
    ];
    if let Some(arc_point) = arc_point {
        group_parts.push(arc_point);
    }
    if let Some(velocity_band) = velocity_band {
        group_parts.push(velocity_band);
    }
    group_parts.push(lane_id);
    let group_id = tree_group_id(group_parts.as_slice());
    let run_rows = render_entry_run_table(
        candidate_records.as_slice(),
        baseline_records.as_slice(),
        comparison.is_some(),
        depth + 1,
        group_id.as_str(),
        run_change_map,
        output_dir,
        candidate_record_map,
        baseline_record_map,
    );
    let mut rows = String::new();
    let current_lane_label = display_compare_role_label(comparison.is_some(), lane_id, TreeRowTone::Current);
    let current_note = render_summary_note(
        comparison.is_some(),
        TreeRowTone::Current,
        comparison.is_some().then_some(regression_count),
        None,
        aggregate.is_some(),
        baseline_aggregate.is_some(),
    );
    let current_note = render_summary_note_with_preview(
        current_note.as_str(),
        render_lane_preview(candidate_records.as_slice()).as_deref(),
    );
    rows.push_str(&render_summary_row(
        current_lane_label.as_str(),
        depth,
        parent_group_id,
        (!candidate_records.is_empty()).then_some(group_id.as_str()),
        "lane",
        aggregate.as_ref(),
        None,
        SummaryMetricStyle::MeanStddev,
        changed,
        current_note.as_str(),
        comparison.is_some().then_some("cur"),
        TreeRowTone::Current,
    ));
    if comparison.is_some() && baseline_aggregate.is_some() {
        let baseline_lane_label =
            display_compare_role_label(true, lane_id, TreeRowTone::Baseline);
        let baseline_group = (candidate_records.is_empty() && !baseline_records.is_empty())
            .then_some(group_id.as_str());
        let baseline_note = render_summary_note(
            true,
            TreeRowTone::Baseline,
            None,
            None,
            aggregate.is_some(),
            baseline_aggregate.is_some(),
        );
        let baseline_note = render_summary_note_with_preview(
            baseline_note.as_str(),
            render_lane_preview(baseline_records.as_slice()).as_deref(),
        );
        rows.push_str(&render_summary_row(
            baseline_lane_label.as_str(),
            depth,
            parent_group_id,
            baseline_group,
            "lane",
            baseline_aggregate.as_ref(),
            None,
            SummaryMetricStyle::MeanStddev,
            changed,
            baseline_note.as_str(),
            Some("base"),
            TreeRowTone::Baseline,
        ));
    }
    rows.push_str(&run_rows);
    rows
}

fn render_entry_run_table(
    candidate_records: &[&crate::BatchRunRecord],
    baseline_records: &[&crate::BatchRunRecord],
    show_baseline: bool,
    depth: usize,
    parent_group_id: &str,
    run_change_map: &BTreeMap<String, (&'static str, &'static str)>,
    output_dir: &Path,
    _candidate_record_map: &BTreeMap<String, String>,
    _baseline_record_map: &BTreeMap<String, String>,
) -> String {
    if candidate_records.is_empty() && baseline_records.is_empty() {
        return String::new();
    }

    let mut candidate_records = candidate_records.to_vec();
    candidate_records.sort_by(|lhs, rhs| {
        lhs.resolved
            .resolved_seed
            .cmp(&rhs.resolved.resolved_seed)
            .then(lhs.resolved.run_id.cmp(&rhs.resolved.run_id))
    });
    let mut baseline_by_run_id = baseline_records
        .iter()
        .map(|record| (record.resolved.run_id.clone(), *record))
        .collect::<BTreeMap<_, _>>();

    let mut rows = String::new();
    for record in candidate_records {
        rows.push_str(&render_seed_run_row(
            record,
            depth,
            parent_group_id,
            TreeRowTone::Current,
            run_change_map,
            output_dir,
        ));
        if show_baseline
            && let Some(baseline_record) = baseline_by_run_id.remove(&record.resolved.run_id)
        {
            rows.push_str(&render_seed_run_row(
                baseline_record,
                depth,
                parent_group_id,
                TreeRowTone::Baseline,
                run_change_map,
                output_dir,
            ));
        }
    }

    if show_baseline {
        let mut baseline_only = baseline_by_run_id.into_values().collect::<Vec<_>>();
        baseline_only.sort_by(|lhs, rhs| {
            lhs.resolved
                .resolved_seed
                .cmp(&rhs.resolved.resolved_seed)
                .then(lhs.resolved.run_id.cmp(&rhs.resolved.run_id))
        });
        for record in baseline_only {
            rows.push_str(&render_seed_run_row(
                record,
                depth,
                parent_group_id,
                TreeRowTone::Baseline,
                run_change_map,
                output_dir,
            ));
        }
    }

    rows
}

fn tree_group_id(parts: &[&str]) -> String {
    let mut tokens = parts
        .iter()
        .map(|part| {
            let mut out = String::new();
            let mut last_dash = false;
            for ch in part.chars() {
                let ch = ch.to_ascii_lowercase();
                if ch.is_ascii_alphanumeric() {
                    out.push(ch);
                    last_dash = false;
                } else if !last_dash {
                    out.push('-');
                    last_dash = true;
                }
            }
            let trimmed = out.trim_matches('-');
            if trimmed.is_empty() {
                "x".to_owned()
            } else {
                trimmed.to_owned()
            }
        })
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return "group".to_owned();
    }
    tokens.insert(0, "tree".to_owned());
    tokens.join("--")
}

fn render_summary_note(
    show_compare: bool,
    tone: TreeRowTone,
    regression_count: Option<usize>,
    expectation: Option<&str>,
    candidate_present: bool,
    baseline_present: bool,
) -> String {
    let mut items = Vec::new();
    if let Some(expectation) = expectation
        && !expectation.trim().is_empty()
    {
        items.push(format!(
            r#"<span class="emph">{}</span>"#,
            escape_html(expectation.trim())
        ));
    }
    match (candidate_present, baseline_present, tone) {
        (true, false, TreeRowTone::Current)
        | (false, true, TreeRowTone::Current)
        | (false, true, TreeRowTone::Baseline)
        | (true, false, TreeRowTone::Baseline) => {}
        _ => {}
    }
    if show_compare
        && tone == TreeRowTone::Current
        && let Some(regression_count) = regression_count
        && regression_count > 0
    {
        let suffix = if regression_count == 1 { "" } else { "s" };
        items.push(format!(
            r#"<span class="compare-toggle-target"><span class="bad">{} regression{}</span></span>"#,
            regression_count, suffix
        ));
    }
    if items.is_empty() {
        return r#"<span class="row-note muted">-</span>"#.to_owned();
    }
    format!(r#"<span class="row-note">{}</span>"#, items.join(" · "))
}

fn render_summary_row(
    label: &str,
    depth: usize,
    parent_group_id: Option<&str>,
    group_id: Option<&str>,
    kind: &str,
    aggregate: Option<&ReviewAggregate>,
    secondary_aggregate: Option<&ReviewAggregate>,
    metric_style: SummaryMetricStyle,
    changed: bool,
    note_html: &str,
    _compare_tag: Option<&str>,
    tone: TreeRowTone,
) -> String {
    let mut row_classes = vec!["summary-row"];
    let lane_identity_class = if kind == "lane" {
        match label {
            "current" => Some("lane-controller-current"),
            "baseline" => Some("lane-controller-baseline"),
            _ => None,
        }
    } else {
        None
    };
    match tone {
        TreeRowTone::Current => {
            row_classes.push("scenario-row");
            row_classes.push("current-row");
        }
        TreeRowTone::Baseline => {
            row_classes.push("baseline-scenario-row");
            row_classes.push("baseline-row");
        }
    }
    if let Some(class) = lane_identity_class {
        row_classes.push(class);
    }
    row_classes.push(if changed { "changed" } else { "unchanged" });

    let mut attrs = Vec::new();
    if let Some(parent_group_id) = parent_group_id {
        attrs.push(format!(r#"data-parent="{}""#, escape_html(parent_group_id)));
    }
    if let Some(group_id) = group_id {
        attrs.push(format!(r#"data-group="{}""#, escape_html(group_id)));
        attrs.push(r#"aria-expanded="false""#.to_owned());
        attrs.push(r#"tabindex="0""#.to_owned());
    }
    attrs.push(format!(r#"data-kind="{}""#, escape_html(kind)));

    let expander = if group_id.is_some() {
        r#"<span class="expander">+</span>"#.to_owned()
    } else {
        r#"<span class="expander muted">·</span>"#.to_owned()
    };
    let tag_html = String::new();
    let outcome_html = aggregate
        .map(|aggregate| format_summary_rate(aggregate, secondary_aggregate, metric_style))
        .unwrap_or_else(|| "-".to_owned());
    let fuel_html = format_metric_cell(
        aggregate.and_then(|aggregate| aggregate.fuel_used_pct_of_max.as_ref()),
        secondary_aggregate.and_then(|aggregate| aggregate.fuel_used_pct_of_max.as_ref()),
        MetricDisplayKind::Percent,
        metric_style,
    );
    let flight_html = format_metric_cell(
        aggregate.and_then(|aggregate| aggregate.sim_time_stats.as_ref()),
        secondary_aggregate.and_then(|aggregate| aggregate.sim_time_stats.as_ref()),
        MetricDisplayKind::Seconds,
        metric_style,
    );
    let offset_html = format_metric_cell(
        aggregate.and_then(|aggregate| aggregate.landing_offset_abs_m.as_ref()),
        secondary_aggregate.and_then(|aggregate| aggregate.landing_offset_abs_m.as_ref()),
        MetricDisplayKind::Meters,
        metric_style,
    );
    let ref_html = format_metric_cell(
        aggregate.and_then(|aggregate| aggregate.reference_gap_mean_m.as_ref()),
        secondary_aggregate.and_then(|aggregate| aggregate.reference_gap_mean_m.as_ref()),
        MetricDisplayKind::Meters,
        metric_style,
    );

    format!(
        r#"<tr class="{classes}" {attrs}>
  <td class="tree-label" style="--depth:{depth}">
    {expander}{tag_html}<span class="selector-inline">{kind}</span> <span class="selector-code">{label}</span>
  </td>
  <td>{outcome}</td>
  <td>{fuel}</td>
  <td>{flight}</td>
  <td>{offset}</td>
  <td>{reference}</td>
  <td>{note}</td>
</tr>"#,
        classes = row_classes.join(" "),
        attrs = attrs.join(" "),
        depth = depth,
        expander = expander,
        tag_html = tag_html,
        kind = escape_html(kind),
        label = escape_html(label),
        outcome = outcome_html,
        fuel = fuel_html,
        flight = flight_html,
        offset = offset_html,
        reference = ref_html,
        note = note_html,
    )
}

fn render_seed_run_row(
    record: &crate::BatchRunRecord,
    depth: usize,
    parent_group_id: &str,
    tone: TreeRowTone,
    run_change_map: &BTreeMap<String, (&'static str, &'static str)>,
    output_dir: &Path,
) -> String {
    let mut row_classes = vec!["seed-row"];
    match tone {
        TreeRowTone::Current => row_classes.push("current-row"),
        TreeRowTone::Baseline => {
            row_classes.push("baseline-row");
            row_classes.push("baseline-seed-row");
        }
    }
    if tone == TreeRowTone::Current {
        match record.resolved.lane_id.as_str() {
            "current" | "staged" => row_classes.push("lane-controller-current"),
            "baseline" => row_classes.push("lane-controller-baseline"),
            _ => {}
        }
    }
    let change = run_change_map.get(&record.resolved.run_id);
    row_classes.push(if change.is_some() {
        "changed"
    } else {
        "unchanged"
    });

    let tag_html = String::new();
    let seed_label = format!("seed {:04}", record.resolved.resolved_seed);
    let outcome = enum_label(&record.manifest.mission_outcome);
    let fuel = record
        .review
        .fuel_used_pct_of_max
        .map(|value| format!("{value:.1}%"))
        .unwrap_or_else(|| "-".to_owned());
    let sim_time = format!("{:.2}s", record.manifest.sim_time_s);
    let landing_offset = record
        .review
        .landing_offset_abs_m
        .map(|value| format!("{value:.2}m"))
        .unwrap_or_else(|| "-".to_owned());
    let reference_gap = record
        .review
        .reference_gap_mean_m
        .map(|value| format!("{value:.2}m"))
        .unwrap_or_else(|| "-".to_owned());
    let change_note = change
        .map(|(label, class)| {
            let class = match *class {
                "bad" => "bad",
                "good" => "good",
                _ => "emph",
            };
            format!(
                r#"<span class="{class}">{label}</span>"#,
                class = class,
                label = escape_html(label)
            )
        })
        .unwrap_or_default();
    let detail_note = if change_note.is_empty() {
        String::new()
    } else {
        format!(r#"<span class="row-note">{change_note}</span>"#)
    };
    let details = format!(
        r#"{detail_note}<div class="preview-cell">{preview}</div>"#,
        detail_note = detail_note,
        preview = render_run_preview(record, output_dir),
    );

    format!(
        r#"<tr class="{classes}" data-parent="{parent}" data-run-id="{run_id}" hidden>
  <td class="tree-label" style="--depth:{depth}">
    {tag_html}<span class="seed-label">{seed}</span>
  </td>
  <td>{outcome}</td>
  <td>{fuel}</td>
  <td>{sim_time}</td>
  <td>{landing_offset}</td>
  <td>{reference_gap}</td>
  <td>{details}</td>
</tr>"#,
        classes = row_classes.join(" "),
        parent = escape_html(parent_group_id),
        run_id = escape_html(&record.resolved.run_id),
        depth = depth,
        tag_html = tag_html,
        seed = escape_html(&seed_label),
        outcome = escape_html(&outcome),
        fuel = escape_html(&fuel),
        sim_time = escape_html(&sim_time),
        landing_offset = escape_html(&landing_offset),
        reference_gap = escape_html(&reference_gap),
        details = details,
    )
}

fn records_by_selector_hierarchy<'a>(candidate: &'a BatchReport) -> MissionRecordGroups<'a> {
    let records = candidate.records.iter().collect::<Vec<_>>();
    records_by_selector_hierarchy_from_records(records.as_slice())
}

fn records_by_selector_hierarchy_from_records<'a>(
    records: &[&'a crate::BatchRunRecord],
) -> MissionRecordGroups<'a> {
    let mut grouped = MissionRecordGroups::new();
    for &record in records {
        grouped
            .entry(record.resolved.selector.mission.clone())
            .or_default()
            .entry(record.resolved.selector.arrival_family.clone())
            .or_default()
            .entry(record.resolved.selector.condition_set.clone())
            .or_default()
            .entry(record.resolved.selector.vehicle_variant.clone())
            .or_default()
            .entry(record.resolved.selector.arc_point.clone())
            .or_default()
            .entry(record.resolved.selector.velocity_band.clone())
            .or_default()
            .entry(record.resolved.lane_id.clone())
            .or_default()
            .push(record);
    }
    grouped
}

fn merged_map_keys<T, U>(
    candidate: &BTreeMap<String, T>,
    baseline: Option<&BTreeMap<String, U>>,
) -> Vec<String> {
    let mut keys = candidate.keys().cloned().collect::<BTreeSet<_>>();
    if let Some(baseline) = baseline {
        keys.extend(baseline.keys().cloned());
    }
    let mut keys = keys.into_iter().collect::<Vec<_>>();
    sort_selector_keys(&mut keys);
    keys
}

fn sort_selector_keys(keys: &mut [String]) {
    keys.sort_by(|lhs, rhs| {
        match (
            lhs.as_str() == UNSPECIFIED_SELECTOR_VALUE,
            rhs.as_str() == UNSPECIFIED_SELECTOR_VALUE,
        ) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => selector_sort_rank(lhs)
                .cmp(&selector_sort_rank(rhs))
                .then(lhs.cmp(rhs)),
        }
    });
}

fn selector_sort_rank(key: &str) -> u8 {
    match key {
        "low" => 0,
        "mid" => 1,
        "high" => 2,
        "nominal" => 10,
        "low_margin" => 11,
        "low_fuel" => 12,
        "heavy_cargo" => 13,
        _ => 20,
    }
}

fn has_meaningful_selector_keys(keys: &[String]) -> bool {
    keys.iter().any(|key| key != UNSPECIFIED_SELECTOR_VALUE)
}

fn sort_lane_keys(keys: &mut [String]) {
    keys.sort_by(|lhs, rhs| {
        lane_sort_rank(lhs)
            .cmp(&lane_sort_rank(rhs))
            .then(lhs.cmp(rhs))
    });
}

fn lane_sort_rank(lane_id: &str) -> u8 {
    match lane_id {
        "current" | "staged" => 0,
        "baseline" => 1,
        _ => 2,
    }
}

fn display_lane_label(lane_id: &str) -> String {
    match lane_id {
        "current" | "staged" => "current".to_owned(),
        "baseline" => "baseline".to_owned(),
        _ => lane_id.to_owned(),
    }
}

fn display_compare_role_label(
    comparison_mode: bool,
    lane_id: &str,
    tone: TreeRowTone,
) -> String {
    if comparison_mode {
        match tone {
            TreeRowTone::Current => "current".to_owned(),
            TreeRowTone::Baseline => "baseline".to_owned(),
        }
    } else {
        display_lane_label(lane_id)
    }
}

fn controller_ids_for_records(records: &[&crate::BatchRunRecord]) -> Vec<String> {
    let mut ids = records
        .iter()
        .map(|record| record.resolved.controller_id.trim())
        .filter(|controller_id| !controller_id.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

fn render_controller_summary_inline(records: &[&crate::BatchRunRecord]) -> String {
    let controller_ids = controller_ids_for_records(records);
    if controller_ids.is_empty() {
        return "controller <code>unspecified</code>".to_owned();
    }
    let rendered = controller_ids
        .iter()
        .map(|controller_id| format!(r#"<code>{}</code>"#, escape_html(controller_id)))
        .collect::<Vec<_>>()
        .join(", ");
    if controller_ids.len() == 1 {
        format!("controller {}", rendered)
    } else {
        format!("controllers {}", rendered)
    }
}

fn flatten_arrival_records<'a>(
    groups: Option<&ArrivalRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
    let mut out = Vec::new();
    if let Some(groups) = groups {
        for conditions in groups.values() {
            for vehicles in conditions.values() {
                for arcs in vehicles.values() {
                    for velocities in arcs.values() {
                        for lanes in velocities.values() {
                            for records in lanes.values() {
                                out.extend(records.iter().copied());
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

fn flatten_condition_records<'a>(
    groups: Option<&ConditionRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
    let mut out = Vec::new();
    if let Some(groups) = groups {
        for vehicles in groups.values() {
            for arcs in vehicles.values() {
                for velocities in arcs.values() {
                    for lanes in velocities.values() {
                        for records in lanes.values() {
                            out.extend(records.iter().copied());
                        }
                    }
                }
            }
        }
    }
    out
}

fn flatten_vehicle_records<'a>(
    groups: Option<&VehicleRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
    let mut out = Vec::new();
    if let Some(groups) = groups {
        for arcs in groups.values() {
            for velocities in arcs.values() {
                for lanes in velocities.values() {
                    for records in lanes.values() {
                        out.extend(records.iter().copied());
                    }
                }
            }
        }
    }
    out
}

fn flatten_arc_records<'a>(groups: Option<&ArcRecordGroups<'a>>) -> Vec<&'a crate::BatchRunRecord> {
    let mut out = Vec::new();
    if let Some(groups) = groups {
        for velocities in groups.values() {
            for lanes in velocities.values() {
                for records in lanes.values() {
                    out.extend(records.iter().copied());
                }
            }
        }
    }
    out
}

fn flatten_velocity_records<'a>(
    groups: Option<&VelocityRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
    let mut out = Vec::new();
    if let Some(groups) = groups {
        for lanes in groups.values() {
            for records in lanes.values() {
                out.extend(records.iter().copied());
            }
        }
    }
    out
}

fn flatten_lane_records<'a>(
    groups: Option<&LaneRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
    groups
        .map(|groups| groups.values().flatten().copied().collect::<Vec<_>>())
        .unwrap_or_default()
}

fn selector_case_key(selector: &crate::SelectorAxes) -> String {
    let mut parts = vec![
        selector.mission.as_str(),
        selector.arrival_family.as_str(),
        selector.condition_set.as_str(),
        selector.vehicle_variant.as_str(),
    ];
    if selector.arc_point != UNSPECIFIED_SELECTOR_VALUE {
        parts.push(selector.arc_point.as_str());
    }
    if selector.velocity_band != UNSPECIFIED_SELECTOR_VALUE {
        parts.push(selector.velocity_band.as_str());
    }
    parts.join(" / ")
}

fn count_regressions<F>(comparison: Option<&BatchComparison>, predicate: F) -> usize
where
    F: Fn(&BatchRunComparison) -> bool,
{
    comparison
        .map(|comparison| {
            comparison
                .regressions
                .iter()
                .filter(|row| predicate(row))
                .count()
        })
        .unwrap_or(0)
}

fn expectation_tier(records: &[&crate::BatchRunRecord]) -> Option<String> {
    records
        .iter()
        .filter_map(|record| record.resolved.selector.expectation_tier.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .next()
}

fn controller_lane_records<'a>(
    records: &[&'a crate::BatchRunRecord],
    lane_id: &str,
) -> Vec<&'a crate::BatchRunRecord> {
    records
        .iter()
        .copied()
        .filter(|record| record.resolved.lane_id == lane_id)
        .collect::<Vec<_>>()
}

fn preferred_current_lane_id(records: &[&crate::BatchRunRecord]) -> Option<&'static str> {
    if records
        .iter()
        .any(|record| record.resolved.lane_id == "current")
    {
        Some("current")
    } else if records
        .iter()
        .any(|record| record.resolved.lane_id == "staged")
    {
        Some("staged")
    } else {
        None
    }
}

fn controller_lane_aggregate(
    records: &[&crate::BatchRunRecord],
    lane_id: &str,
) -> Option<ReviewAggregate> {
    let filtered = controller_lane_records(records, lane_id);
    (!filtered.is_empty()).then(|| review_aggregate_from_records(filtered.as_slice()))
}

fn preferred_current_lane_aggregate(records: &[&crate::BatchRunRecord]) -> Option<ReviewAggregate> {
    preferred_current_lane_id(records)
        .and_then(|lane_id| controller_lane_aggregate(records, lane_id))
}

fn extract_default_lane_groups_from_arcs<'a>(
    groups: Option<&'a ArcRecordGroups<'a>>,
) -> Option<&'a LaneRecordGroups<'a>> {
    groups
        .and_then(|groups| groups.get(UNSPECIFIED_SELECTOR_VALUE))
        .and_then(|velocities| velocities.get(UNSPECIFIED_SELECTOR_VALUE))
}

fn comparison_change_map(
    comparison: Option<&BatchComparison>,
) -> BTreeMap<String, (&'static str, &'static str)> {
    let mut out = BTreeMap::new();
    if let Some(comparison) = comparison {
        for row in &comparison.regressions {
            out.insert(row.run_id.clone(), ("new failure", "bad"));
        }
        for row in &comparison.improvements {
            out.insert(row.run_id.clone(), ("recovered", "good"));
        }
        for row in &comparison.outcome_changes {
            out.entry(row.run_id.clone()).or_insert(("changed", "warn"));
        }
    }
    out
}

fn review_aggregate_from_records(records: &[&crate::BatchRunRecord]) -> ReviewAggregate {
    let total_runs = records.len();
    let success_runs = records
        .iter()
        .filter(|record| {
            matches!(
                record.manifest.mission_outcome,
                pd_core::MissionOutcome::Success
            )
        })
        .count();
    let failure_runs = total_runs.saturating_sub(success_runs);
    let failed_seeds = records
        .iter()
        .filter(|record| {
            !matches!(
                record.manifest.mission_outcome,
                pd_core::MissionOutcome::Success
            )
        })
        .map(|record| record.resolved.resolved_seed)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let success_records = records
        .iter()
        .filter(|record| {
            matches!(
                record.manifest.mission_outcome,
                pd_core::MissionOutcome::Success
            )
        })
        .copied()
        .collect::<Vec<_>>();
    let sim_time_values = success_records
        .iter()
        .map(|record| record.manifest.sim_time_s)
        .collect::<Vec<_>>();
    let fuel_values = success_records
        .iter()
        .filter_map(|record| record.review.fuel_used_pct_of_max)
        .collect::<Vec<_>>();
    let landing_offset_values = success_records
        .iter()
        .filter_map(|record| record.review.landing_offset_abs_m)
        .collect::<Vec<_>>();
    let reference_gap_values = success_records
        .iter()
        .filter_map(|record| record.review.reference_gap_mean_m)
        .collect::<Vec<_>>();
    ReviewAggregate {
        total_runs,
        success_runs,
        failure_runs,
        sim_time_stats: crate::metric_summary(&sim_time_values),
        fuel_used_pct_of_max: crate::metric_summary(&fuel_values),
        landing_offset_abs_m: crate::metric_summary(&landing_offset_values),
        reference_gap_mean_m: crate::metric_summary(&reference_gap_values),
        failed_seeds,
    }
}

fn inline_rate_text(success_runs: usize, total_runs: usize) -> String {
    format!(
        "{:.1}% of {}",
        crate::success_rate(success_runs, total_runs) * 100.0,
        total_runs
    )
}

#[derive(Clone, Copy)]
enum MetricDisplayKind {
    Percent,
    Seconds,
    Meters,
}

fn format_metric_value(summary: &crate::BatchMetricSummary, kind: MetricDisplayKind) -> String {
    match kind {
        MetricDisplayKind::Percent => format!("{:.1}%", summary.mean),
        MetricDisplayKind::Seconds => format!("{:.2}s", summary.mean),
        MetricDisplayKind::Meters => format!("{:.2}m", summary.mean),
    }
}

fn format_metric_delta_value(delta: f64, kind: MetricDisplayKind) -> String {
    match kind {
        MetricDisplayKind::Percent => format!("{:+.1}%", delta),
        MetricDisplayKind::Seconds => format!("{:+.2}s", delta),
        MetricDisplayKind::Meters => format!("{:+.2}m", delta),
    }
}

fn format_metric_summary(
    summary: Option<&crate::BatchMetricSummary>,
    kind: MetricDisplayKind,
) -> String {
    let Some(summary) = summary else {
        return "-".to_owned();
    };
    match kind {
        MetricDisplayKind::Percent => format!(
            "{:.1} ± {:.1}%",
            summary.mean,
            summary.stddev.unwrap_or(0.0)
        ),
        MetricDisplayKind::Seconds => format!(
            "{:.2} ± {:.2}s",
            summary.mean,
            summary.stddev.unwrap_or(0.0)
        ),
        MetricDisplayKind::Meters => format!(
            "{:.2} ± {:.2}m",
            summary.mean,
            summary.stddev.unwrap_or(0.0)
        ),
    }
}

fn format_metric_cell(
    summary: Option<&crate::BatchMetricSummary>,
    baseline: Option<&crate::BatchMetricSummary>,
    kind: MetricDisplayKind,
    style: SummaryMetricStyle,
) -> String {
    let Some(summary) = summary else {
        return "-".to_owned();
    };
    match style {
        SummaryMetricStyle::MeanStddev => escape_html(&format_metric_summary(Some(summary), kind)),
        SummaryMetricStyle::MeanDelta => {
            let value = escape_html(&format_metric_value(summary, kind));
            match metric_delta_value(Some(summary), baseline) {
                Some(delta) => format!(
                    r#"{value}<span class="compare-toggle-target"> ({})</span>"#,
                    escape_html(&format_metric_delta_value(delta, kind))
                ),
                None => value,
            }
        }
    }
}

fn format_summary_rate(
    aggregate: &ReviewAggregate,
    baseline: Option<&ReviewAggregate>,
    style: SummaryMetricStyle,
) -> String {
    let base = format!(
        "{} · {} fail",
        inline_rate_text(aggregate.success_runs, aggregate.total_runs),
        aggregate.failure_runs
    );
    match style {
        SummaryMetricStyle::MeanStddev => escape_html(&base),
        SummaryMetricStyle::MeanDelta => {
            let Some(baseline) = baseline else {
                return escape_html(&base);
            };
            let delta = (crate::success_rate(aggregate.success_runs, aggregate.total_runs)
                - crate::success_rate(baseline.success_runs, baseline.total_runs))
                * 100.0;
            format!(
                r#"{}<span class="compare-toggle-target"> ({delta:+.1}pt)</span>"#,
                escape_html(&base)
            )
        }
    }
}

fn metric_delta_value(
    candidate: Option<&crate::BatchMetricSummary>,
    baseline: Option<&crate::BatchMetricSummary>,
) -> Option<f64> {
    Some(candidate?.mean - baseline?.mean)
}

fn aggregate_changed(
    candidate: Option<&ReviewAggregate>,
    baseline: Option<&ReviewAggregate>,
) -> bool {
    let Some(candidate) = candidate else {
        return baseline.is_some();
    };
    let Some(baseline) = baseline else {
        return true;
    };
    if candidate.total_runs != baseline.total_runs
        || candidate.success_runs != baseline.success_runs
        || candidate.failure_runs != baseline.failure_runs
        || candidate.failed_seeds != baseline.failed_seeds
    {
        return true;
    }
    metric_delta_value(
        candidate.fuel_used_pct_of_max.as_ref(),
        baseline.fuel_used_pct_of_max.as_ref(),
    )
    .is_some_and(|delta| delta.abs() > 1e-9)
        || metric_delta_value(
            candidate.sim_time_stats.as_ref(),
            baseline.sim_time_stats.as_ref(),
        )
        .is_some_and(|delta| delta.abs() > 1e-9)
        || metric_delta_value(
            candidate.landing_offset_abs_m.as_ref(),
            baseline.landing_offset_abs_m.as_ref(),
        )
        .is_some_and(|delta| delta.abs() > 1e-9)
        || metric_delta_value(
            candidate.reference_gap_mean_m.as_ref(),
            baseline.reference_gap_mean_m.as_ref(),
        )
        .is_some_and(|delta| delta.abs() > 1e-9)
}

fn render_run_pointer_table(
    rows: &[BatchRunPointer],
    output_dir: &Path,
    empty_text: &str,
) -> String {
    if rows.is_empty() {
        return format!(r#"<p class="muted">{}</p>"#, escape_html(empty_text));
    }
    let body = rows
        .iter()
        .map(|row| {
            format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{:.1}kg</td><td>{:.2}s</td><td>{}</td></tr>",
                escape_html(&row.run_id),
                escape_html(&row.mission_outcome),
                render_pointer_focus(row),
                render_pointer_margin(row),
                row.fuel_remaining_kg,
                row.sim_time_s,
                render_pointer_links(row, output_dir),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<table><thead><tr><th>Run</th><th>Outcome</th><th>Focus</th><th>Margin</th><th>Fuel</th><th>Sim</th><th>Links</th></tr></thead><tbody>{}</tbody></table>",
        body
    )
}

fn render_comparison_sections(
    output_dir: &Path,
    comparison: &BatchComparison,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
) -> String {
    format!(
        r#"<section class="layout">
  <div class="panel">
    <h2>Regressions</h2>
    <p>Shared runs that moved from success to failure.</p>
    <div class="table-wrap">{}</div>
  </div>
  <div class="panel">
    <h2>Recoveries</h2>
    <p>Shared runs that moved from failure to success.</p>
    <div class="table-wrap">{}</div>
  </div>
</section>
<details>
  <summary>Other Outcome Changes ({})</summary>
  <div class="table-wrap">{}</div>
</details>
<details>
  <summary>Candidate-only Runs ({})</summary>
  <div class="table-wrap">{}</div>
</details>
<details>
  <summary>Baseline-only Runs ({})</summary>
  <div class="table-wrap">{}</div>
</details>"#,
        render_run_comparison_table(
            &comparison.regressions,
            output_dir,
            candidate_record_map,
            baseline_record_map,
            "No new regressions recorded."
        ),
        render_run_comparison_table(
            &comparison.improvements,
            output_dir,
            candidate_record_map,
            baseline_record_map,
            "No recoveries recorded."
        ),
        comparison.outcome_changes.len(),
        render_run_comparison_table(
            &comparison.outcome_changes,
            output_dir,
            candidate_record_map,
            baseline_record_map,
            "No non-terminal shared-run differences recorded."
        ),
        comparison.candidate_only.len(),
        render_run_pointer_table(
            &comparison.candidate_only,
            output_dir,
            "No candidate-only runs."
        ),
        comparison.baseline_only.len(),
        render_run_pointer_table(
            &comparison.baseline_only,
            output_dir,
            "No baseline-only runs."
        ),
    )
}

fn render_run_comparison_table(
    rows: &[BatchRunComparison],
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
    empty_text: &str,
) -> String {
    if rows.is_empty() {
        return format!(r#"<p class="muted">{}</p>"#, escape_html(empty_text));
    }
    let body = rows
        .iter()
        .map(|row| {
            format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{} → {}</td><td>{}</td><td>{}</td><td>{:.2}s</td><td>{}</td></tr>",
                escape_html(&row.run_id),
                escape_html(&enum_label(&row.change_kind)),
                escape_html(&row.baseline_mission_outcome),
                escape_html(&row.candidate_mission_outcome),
                render_comparison_margin_delta(row),
                render_comparison_fuel_delta(row),
                row.sim_time_delta_s,
                render_dual_links(
                    &row.run_id,
                    candidate_record_map,
                    baseline_record_map,
                    output_dir,
                ),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<table><thead><tr><th>Run</th><th>Kind</th><th>Outcome</th><th>Margin Δ</th><th>Fuel Δ</th><th>Sim Δ</th><th>Links</th></tr></thead><tbody>{}</tbody></table>",
        body
    )
}

fn render_pointer_focus(pointer: &BatchRunPointer) -> String {
    if let Some(checkpoint) = pointer.summary.checkpoint.as_ref() {
        return format!(
            "seed {} | pos {:.2}m | vel {:.2}m/s | att {:.1}°",
            pointer.scenario_seed,
            checkpoint.position_error_m,
            checkpoint.velocity_error_mps,
            checkpoint.attitude_error_rad.to_degrees()
        );
    }
    if let Some(landing) = pointer.summary.landing.as_ref() {
        return format!(
            "seed {} | pad {:+.2}m | n {:.2} | t {:.2} | att {:.1}°",
            pointer.scenario_seed,
            landing.touchdown_center_offset_m,
            landing.normal_speed_mps,
            landing.tangential_speed_mps,
            landing.attitude_error_rad.to_degrees()
        );
    }
    format!("seed {}", pointer.scenario_seed)
}

fn render_pointer_margin(pointer: &BatchRunPointer) -> String {
    pointer
        .margin_ratio
        .map(|value| {
            format!(
                r#"<span class="{}">{}</span>"#,
                margin_class(value),
                format_margin_ratio(value)
            )
        })
        .unwrap_or_else(|| r#"<span class="muted">-</span>"#.to_owned())
}

fn render_comparison_margin_delta(row: &BatchRunComparison) -> String {
    match row.margin_ratio_delta {
        Some(delta) => format!(
            r#"<span class="{}">{}</span>"#,
            delta_class(-delta),
            format_margin_delta(delta)
        ),
        None => r#"<span class="muted">-</span>"#.to_owned(),
    }
}

fn render_comparison_fuel_delta(row: &BatchRunComparison) -> String {
    format!(
        r#"<span class="{}">{}</span>"#,
        delta_class(-row.fuel_remaining_delta_kg),
        format_signed_kg(row.fuel_remaining_delta_kg)
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

fn render_lane_preview(records: &[&crate::BatchRunRecord]) -> Option<String> {
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
        let Some(samples) = load_json_file::<Vec<SampleRecord>>(&bundle_dir.join("samples.json"))
        else {
            continue;
        };
        loaded.push((scenario, samples, &record.manifest));
    }
    if loaded.is_empty() {
        return None;
    }
    let series = loaded
        .iter()
        .map(|(scenario, samples, manifest)| PreviewSeries {
            scenario,
            manifest: *manifest,
            samples,
        })
        .collect::<Vec<_>>();
    Some(format!(
        r#"<div class="run-preview lane-preview">{}</div>"#,
        build_multi_run_preview_svg(&series)
    ))
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
    let site_report_path = report_site_output_for_batch_run(&bundle_dir);
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
    Some(
        crate::repo_root()
            .join("outputs")
            .join("reports")
            .join(relative)
            .join("index.html"),
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

fn update_report_site_indexes_for_file(report_file: &Path) -> Result<()> {
    let report_dir = report_file
        .parent()
        .ok_or_else(|| anyhow::anyhow!("report site output has no parent directory"))?;
    maybe_update_latest_link(report_dir)?;

    let reports_root = crate::repo_root().join("outputs").join("reports");
    let resolved_report_dir = resolve_repo_relative(report_dir);
    if !resolved_report_dir.starts_with(&reports_root) {
        return Ok(());
    }

    if let Some(collection_dir) = collection_dir_for_report_dir(&resolved_report_dir, &reports_root)
    {
        write_collection_index(&collection_dir, &reports_root)?;
    }
    if let Some(scope_dir) = scope_dir_for_report_dir(&resolved_report_dir, &reports_root) {
        write_scope_index(&scope_dir)?;
    }
    write_reports_home_index(&reports_root)?;
    write_outputs_root_index(&crate::repo_root().join("outputs"))?;
    Ok(())
}

fn scope_dir_for_report_dir(report_dir: &Path, reports_root: &Path) -> Option<PathBuf> {
    let relative = report_dir.strip_prefix(reports_root).ok()?;
    let scope = relative.iter().next()?;
    Some(reports_root.join(scope))
}

fn collection_dir_for_report_dir(report_dir: &Path, reports_root: &Path) -> Option<PathBuf> {
    let parent_dir = report_dir.parent()?;
    let relative = parent_dir.strip_prefix(reports_root).ok()?;
    (relative.components().count() > 1).then(|| parent_dir.to_path_buf())
}

fn write_reports_home_index(reports_root: &Path) -> Result<()> {
    fs::create_dir_all(reports_root)
        .with_context(|| format!("failed to create reports root {}", reports_root.display()))?;
    let scope_cards = ["runs", "replays", "eval"]
        .iter()
        .map(|scope| render_scope_card(reports_root, scope))
        .collect::<Vec<_>>()
        .join("");
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Powered Descent Lab Reports</title>
  <style>
    :root {{
      color-scheme: light;
      --bg: #f5f1e8;
      --surface: rgba(255,253,248,0.94);
      --line: #d8cebe;
      --ink: #1d1a16;
      --muted: #675d51;
      --accent: #b55d2d;
      --sans: "IBM Plex Sans", "Segoe UI", sans-serif;
      --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: var(--sans);
      color: var(--ink);
      background:
        radial-gradient(circle at top left, rgba(181,93,45,0.08), transparent 28rem),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
    }}
    .page {{
      max-width: 1100px;
      margin: 0 auto;
      padding: 28px 18px 40px;
    }}
    h1 {{ margin: 0 0 8px; font-size: 2rem; }}
    p {{ margin: 0; color: var(--muted); max-width: 72ch; }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 14px;
      margin-top: 22px;
    }}
    .card {{
      display: block;
      text-decoration: none;
      color: inherit;
      padding: 16px 17px;
      border-radius: 18px;
      border: 1px solid var(--line);
      background: var(--surface);
      box-shadow: 0 10px 30px rgba(39,28,18,0.06);
      min-height: 160px;
    }}
    .card:hover {{ border-color: var(--accent); transform: translateY(-1px); }}
    .eyebrow {{
      color: var(--muted);
      font-size: 0.78rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }}
    .title {{ font-size: 1.15rem; font-weight: 700; margin-top: 6px; }}
    .meta {{ margin-top: 10px; display: grid; gap: 6px; color: var(--muted); font-size: 0.9rem; }}
    .meta code {{ font-family: var(--mono); background: rgba(248,243,234,0.92); padding: 1px 5px; border-radius: 6px; }}
    @media (max-width: 860px) {{ .grid {{ grid-template-columns: 1fr; }} }}
  </style>
</head>
<body>
  <div class="page">
    <h1>Report Site</h1>
    <p>Stable HTML entrypoints live under <code>/reports/</code>. Raw artifacts remain outside this tree, but the default navigation surface now keeps runs, replays, and batch pages isolated from bundle JSON.</p>
    <div class="grid">{scope_cards}</div>
  </div>
</body>
</html>"#
    );
    fs::write(reports_root.join("index.html"), html).with_context(|| {
        format!(
            "failed to write reports home index {}",
            reports_root.join("index.html").display()
        )
    })?;
    Ok(())
}

fn write_outputs_root_index(outputs_root: &Path) -> Result<()> {
    fs::create_dir_all(outputs_root)
        .with_context(|| format!("failed to create outputs root {}", outputs_root.display()))?;

    let reports_root = crate::repo_root().join("outputs").join("reports");
    let latest_run = reports_root
        .join("runs")
        .join("latest")
        .exists()
        .then_some("reports/runs/latest/");
    let latest_eval = reports_root
        .join("eval")
        .join("latest")
        .exists()
        .then_some("reports/eval/latest/");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Powered Descent Lab Outputs</title>
  <style>
    :root {{
      color-scheme: light;
      --bg: #f5f1e8;
      --surface: rgba(255,253,248,0.94);
      --line: #d8cebe;
      --ink: #1d1a16;
      --muted: #675d51;
      --accent: #b55d2d;
      --sans: "IBM Plex Sans", "Segoe UI", sans-serif;
      --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: var(--sans);
      color: var(--ink);
      background:
        radial-gradient(circle at top left, rgba(181,93,45,0.08), transparent 28rem),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
    }}
    .page {{
      max-width: 1200px;
      margin: 0 auto;
      padding: 28px 18px 40px;
    }}
    h1 {{ margin: 0 0 8px; font-size: 2rem; }}
    p {{ margin: 0; color: var(--muted); max-width: 74ch; }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 14px;
      margin-top: 22px;
    }}
    .card {{
      display: block;
      text-decoration: none;
      color: inherit;
      padding: 17px 18px;
      border-radius: 18px;
      border: 1px solid var(--line);
      background: var(--surface);
      box-shadow: 0 10px 30px rgba(39,28,18,0.06);
      min-height: 180px;
    }}
    .card:hover {{ border-color: var(--accent); transform: translateY(-1px); }}
    .eyebrow {{
      color: var(--muted);
      font-size: 0.78rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }}
    .title {{ font-size: 1.25rem; font-weight: 700; margin-top: 6px; }}
    .meta {{ margin-top: 12px; display: grid; gap: 8px; color: var(--muted); font-size: 0.92rem; }}
    .meta code {{ font-family: var(--mono); background: rgba(248,243,234,0.92); padding: 1px 5px; border-radius: 6px; }}
    .link-row {{ display: flex; flex-wrap: wrap; gap: 8px; margin-top: 14px; }}
    .link-row a {{
      text-decoration: none;
      color: inherit;
      border: 1px solid var(--line);
      background: rgba(248,243,234,0.92);
      padding: 7px 10px;
      border-radius: 999px;
      font-size: 0.84rem;
    }}
    @media (max-width: 860px) {{ .grid {{ grid-template-columns: 1fr; }} }}
  </style>
</head>
<body>
  <div class="page">
    <h1>Outputs</h1>
    <p>The root landing page keeps stable HTML reports separate from raw bundles. Start with the report site unless you explicitly need artifact JSON or bundle directories.</p>
    <div class="grid">
      <div class="card">
        <div class="eyebrow">recommended</div>
        <div class="title">Report site</div>
        <div class="meta">
          <div>Clean HTML navigation for runs, replays, and batch reports.</div>
          <div>Entry: <code>reports/</code></div>
        </div>
        <div class="link-row">
          <a href="reports/">home</a>
          <a href="reports/runs/">runs</a>
          <a href="reports/eval/">eval</a>
          <a href="reports/replays/">replays</a>
        </div>
      </div>
      <div class="card">
        <div class="eyebrow">raw</div>
        <div class="title">Artifact directories</div>
        <div class="meta">
          <div>Direct access to raw bundle trees and JSON outputs.</div>
          <div>Use these when a report does not surface the data you need yet.</div>
        </div>
        <div class="link-row">
          <a href="runs/">runs/</a>
          <a href="eval/">eval/</a>
          <a href="replays/">replays/</a>
        </div>
      </div>
    </div>
    <div class="grid">
      <div class="card">
        <div class="eyebrow">latest</div>
        <div class="title">Fast paths</div>
        <div class="meta">
          <div>Use these when you mostly care about the most recent generated pages.</div>
        </div>
        <div class="link-row">
          {latest_run_link}
          {latest_eval_link}
        </div>
      </div>
      <div class="card">
        <div class="eyebrow">notes</div>
        <div class="title">Structure</div>
        <div class="meta">
          <div>Stable HTML: <code>reports/...</code></div>
          <div>Raw artifacts: <code>runs/</code>, <code>eval/</code>, <code>replays/</code></div>
        </div>
      </div>
    </div>
  </div>
</body>
</html>"#,
        latest_run_link = latest_run
            .map(|href| format!(r#"<a href="{href}">latest run</a>"#))
            .unwrap_or_else(|| r#"<span>latest run not yet created</span>"#.to_owned()),
        latest_eval_link = latest_eval
            .map(|href| format!(r#"<a href="{href}">latest batch</a>"#))
            .unwrap_or_else(|| r#"<span>latest batch not yet created</span>"#.to_owned()),
    );

    fs::write(outputs_root.join("index.html"), html).with_context(|| {
        format!(
            "failed to write outputs root index {}",
            outputs_root.join("index.html").display()
        )
    })?;
    Ok(())
}

fn render_scope_card(reports_root: &Path, scope: &str) -> String {
    let scope_dir = reports_root.join(scope);
    let latest_dir = scope_dir.join("latest");
    let latest_href = latest_dir.exists().then(|| format!("{scope}/latest/"));
    let entries = scope_entries(&scope_dir).unwrap_or_default();
    let latest_entry = entries
        .first()
        .map(|entry| entry.name.as_str())
        .unwrap_or("none");
    let total = entries.len();
    format!(
        r#"<a class="card" href="{scope}/">
  <div class="eyebrow">reports</div>
  <div class="title">{title}</div>
  <div class="meta">
    <div>entries: <strong>{total}</strong></div>
    <div>latest: <code>{latest_entry}</code></div>
    <div>{latest_line}</div>
  </div>
</a>"#,
        scope = scope,
        title = escape_html(&scope_title(scope)),
        total = total,
        latest_entry = escape_html(latest_entry),
        latest_line = latest_href
            .map(|href| format!(r#"latest url: <code>{}</code>"#, escape_html(&href)))
            .unwrap_or_else(|| "latest url: <code>not yet created</code>".to_owned()),
    )
}

fn write_scope_index(scope_dir: &Path) -> Result<()> {
    fs::create_dir_all(scope_dir)
        .with_context(|| format!("failed to create scope dir {}", scope_dir.display()))?;
    let entries = scope_entries(scope_dir)?;
    let scope = scope_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("reports");
    let title = scope_title(scope);
    let latest_href = scope_dir.join("latest").exists().then_some("latest/");
    let rows = if entries.is_empty() {
        r#"<tr><td colspan="3" class="muted">No reports yet.</td></tr>"#.to_owned()
    } else {
        entries
            .iter()
            .map(|entry| {
                format!(
                    r#"<tr><td><a href="{name}/">{name}</a></td><td>{modified}</td><td class="mono">{path}</td></tr>"#,
                    name = escape_html(&entry.name),
                    modified = escape_html(&entry.modified_label),
                    path = escape_html(&format!("{scope}/{}/", entry.name)),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title} reports</title>
  <style>
    :root {{
      color-scheme: light;
      --bg: #f5f1e8;
      --surface: rgba(255,253,248,0.94);
      --line: #d8cebe;
      --ink: #1d1a16;
      --muted: #675d51;
      --accent: #b55d2d;
      --sans: "IBM Plex Sans", "Segoe UI", sans-serif;
      --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: var(--sans);
      color: var(--ink);
      background:
        radial-gradient(circle at top left, rgba(181,93,45,0.08), transparent 28rem),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
    }}
    .page {{ max-width: 1100px; margin: 0 auto; padding: 28px 18px 40px; }}
    .top {{ display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; margin-bottom: 18px; }}
    h1 {{ margin: 0 0 6px; font-size: 1.8rem; }}
    p {{ margin: 0; color: var(--muted); max-width: 70ch; }}
    .actions {{ display: flex; gap: 8px; flex-wrap: wrap; }}
    .actions a {{ text-decoration: none; color: inherit; border: 1px solid var(--line); background: var(--surface); padding: 7px 11px; border-radius: 10px; font-size: 0.84rem; }}
    table {{ width: 100%; border-collapse: collapse; border: 1px solid var(--line); background: var(--surface); border-radius: 16px; overflow: hidden; box-shadow: 0 10px 30px rgba(39,28,18,0.06); }}
    th, td {{ text-align: left; padding: 10px 12px; border-bottom: 1px solid rgba(216,206,190,0.7); font-size: 0.92rem; }}
    th {{ color: var(--muted); font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.05em; }}
    .mono {{ font-family: var(--mono); }}
    .muted {{ color: var(--muted); }}
  </style>
</head>
<body>
  <div class="page">
    <div class="top">
      <div>
        <h1>{title}</h1>
        <p>Newest report directories first. Use these stable URLs instead of browsing raw artifact folders.</p>
      </div>
      <div class="actions">
        <a href="../">reports/</a>
        {latest_link}
      </div>
    </div>
    <table>
      <thead><tr><th>Name</th><th>Updated</th><th>URL</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>
  </div>
</body>
</html>"#,
        title = escape_html(&title),
        latest_link = latest_href
            .map(|href| format!(r#"<a href="{href}">latest</a>"#))
            .unwrap_or_default(),
        rows = rows,
    );
    fs::write(scope_dir.join("index.html"), html).with_context(|| {
        format!(
            "failed to write scope report index {}",
            scope_dir.join("index.html").display()
        )
    })?;
    Ok(())
}

fn write_collection_index(collection_dir: &Path, reports_root: &Path) -> Result<()> {
    fs::create_dir_all(collection_dir).with_context(|| {
        format!(
            "failed to create collection dir {}",
            collection_dir.display()
        )
    })?;
    let entries = scope_entries(collection_dir)?;
    let relative_dir = collection_dir
        .strip_prefix(reports_root)
        .unwrap_or(collection_dir);
    let title = collection_title(relative_dir);
    let latest_href = collection_dir.join("latest").exists().then_some("latest/");
    let back_href = "../";
    let rows = if entries.is_empty() {
        r#"<tr><td colspan="3" class="muted">No reports yet.</td></tr>"#.to_owned()
    } else {
        entries
            .iter()
            .map(|entry| {
                let relative_path = relative_dir.join(&entry.name);
                format!(
                    r#"<tr><td><a href="{name}/">{name}</a></td><td>{modified}</td><td class="mono">{path}/</td></tr>"#,
                    name = escape_html(&entry.name),
                    modified = escape_html(&entry.modified_label),
                    path = escape_html(&relative_path.display().to_string()),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>
    :root {{
      color-scheme: light;
      --bg: #f5f1e8;
      --surface: rgba(255,253,248,0.94);
      --line: #d8cebe;
      --ink: #1d1a16;
      --muted: #675d51;
      --accent: #b55d2d;
      --sans: "IBM Plex Sans", "Segoe UI", sans-serif;
      --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: var(--sans);
      color: var(--ink);
      background:
        radial-gradient(circle at top left, rgba(181,93,45,0.08), transparent 28rem),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
    }}
    .page {{ max-width: 1100px; margin: 0 auto; padding: 28px 18px 40px; }}
    .top {{ display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; margin-bottom: 18px; }}
    h1 {{ margin: 0 0 6px; font-size: 1.8rem; }}
    p {{ margin: 0; color: var(--muted); max-width: 70ch; }}
    .actions {{ display: flex; gap: 8px; flex-wrap: wrap; }}
    .actions a {{ text-decoration: none; color: inherit; border: 1px solid var(--line); background: var(--surface); padding: 7px 11px; border-radius: 10px; font-size: 0.84rem; }}
    table {{ width: 100%; border-collapse: collapse; border: 1px solid var(--line); background: var(--surface); border-radius: 16px; overflow: hidden; box-shadow: 0 10px 30px rgba(39,28,18,0.06); }}
    th, td {{ text-align: left; padding: 10px 12px; border-bottom: 1px solid rgba(216,206,190,0.7); font-size: 0.92rem; }}
    th {{ color: var(--muted); font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.05em; }}
    .mono {{ font-family: var(--mono); }}
    .muted {{ color: var(--muted); }}
  </style>
</head>
<body>
  <div class="page">
    <div class="top">
      <div>
        <h1>{title}</h1>
        <p>Nested report collection. Use this page instead of falling back to a raw directory listing.</p>
      </div>
      <div class="actions">
        <a href="{back_href}">up</a>
        <a href="../../">reports/</a>
        {latest_link}
      </div>
    </div>
    <table>
      <thead><tr><th>Name</th><th>Updated</th><th>URL</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>
  </div>
</body>
</html>"#,
        title = escape_html(&title),
        back_href = back_href,
        latest_link = latest_href
            .map(|href| format!(r#"<a href="{href}">latest</a>"#))
            .unwrap_or_default(),
        rows = rows,
    );
    fs::write(collection_dir.join("index.html"), html).with_context(|| {
        format!(
            "failed to write collection index {}",
            collection_dir.join("index.html").display()
        )
    })?;
    Ok(())
}

fn scope_entries(scope_dir: &Path) -> Result<Vec<ScopeEntry>> {
    let mut entries = Vec::new();
    if !scope_dir.exists() {
        return Ok(entries);
    }
    for dir_entry in fs::read_dir(scope_dir)
        .with_context(|| format!("failed to read scope dir {}", scope_dir.display()))?
    {
        let dir_entry = dir_entry?;
        let path = dir_entry.path();
        let name = dir_entry.file_name().to_string_lossy().into_owned();
        if name == "latest" || name == "index.html" {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)?;
        let file_type = metadata.file_type();
        if !(metadata.is_dir() || file_type.is_symlink()) {
            continue;
        }
        let modified = entry_modified_time(&path, &metadata);
        entries.push(ScopeEntry {
            name,
            modified,
            modified_label: modified_label(modified),
        });
    }
    entries.sort_by(|lhs, rhs| {
        rhs.modified
            .cmp(&lhs.modified)
            .then(lhs.name.cmp(&rhs.name))
    });
    Ok(entries)
}

fn modified_label(modified: SystemTime) -> String {
    match modified.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => format!("unix {}", duration.as_secs()),
        Err(_) => "unknown".to_owned(),
    }
}

fn scope_title(scope: &str) -> String {
    match scope {
        "runs" => "Run reports".to_owned(),
        "replays" => "Replay reports".to_owned(),
        "eval" => "Batch reports".to_owned(),
        other => format!("{other} reports"),
    }
}

fn collection_title(relative_dir: &Path) -> String {
    let parts = relative_dir
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        "Report collection".to_owned()
    } else {
        format!("{} index", parts.join(" / "))
    }
}

fn entry_modified_time(path: &Path, metadata: &fs::Metadata) -> SystemTime {
    let report_file = path.join("index.html");
    fs::metadata(&report_file)
        .and_then(|report_metadata| report_metadata.modified())
        .unwrap_or_else(|_| metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH))
}

fn maybe_update_latest_link(target_dir: &Path) -> Result<()> {
    let repo_root = crate::repo_root();
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
            return Ok(());
        }
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(PathBuf::from(target_name), &latest_path).with_context(|| {
        format!(
            "failed to create latest link {} -> {}",
            latest_path.display(),
            target_name.to_string_lossy()
        )
    })?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(PathBuf::from(target_name), &latest_path).with_context(
        || {
            format!(
                "failed to create latest link {} -> {}",
                latest_path.display(),
                target_name.to_string_lossy()
            )
        },
    )?;
    Ok(())
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

struct ScopeEntry {
    name: String,
    modified: SystemTime,
    modified_label: String,
}

#[cfg(test)]
mod report_tests {
    use std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
    };

    use crate::{
        ConcreteScenarioPackEntry, NumericPerturbationMode, NumericPerturbationSpec,
        ScenarioFamilyEntry, ScenarioPackEntry, ScenarioPackSpec, SeedRangeSpec,
        TerminalMatrixEntry, TerminalMatrixLaneSpec, TerminalSeedTier, compare_batch_reports,
        run_pack_with_workers,
    };

    use super::{render_batch_report, sort_selector_keys};

    fn fixtures_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures")
    }

    fn terminal_metadata(
        vehicle_variant: &str,
        expectation_tier: &str,
    ) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("mission".to_owned(), "terminal_guidance".to_owned()),
            (
                "arrival_family".to_owned(),
                "seeded_terminal_arrival_v0".to_owned(),
            ),
            ("condition_set".to_owned(), "clean".to_owned()),
            ("vehicle_variant".to_owned(), vehicle_variant.to_owned()),
            ("expectation_tier".to_owned(), expectation_tier.to_owned()),
        ])
    }

    fn checkpoint_metadata() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("mission".to_owned(), "checkpoint_validation".to_owned()),
            (
                "arrival_family".to_owned(),
                "checkpoint_reference_v0".to_owned(),
            ),
            ("condition_set".to_owned(), "clean".to_owned()),
            ("vehicle_variant".to_owned(), "nominal".to_owned()),
            ("expectation_tier".to_owned(), "reference".to_owned()),
        ])
    }

    fn terminal_family_entry(id: &str, family: &str, controller: &str) -> ScenarioPackEntry {
        ScenarioPackEntry::Family(ScenarioFamilyEntry {
            id: id.to_owned(),
            family: family.to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            controller: controller.to_owned(),
            controller_config: None,
            seeds: Vec::new(),
            seed_range: Some(SeedRangeSpec { start: 0, count: 2 }),
            perturbations: vec![
                NumericPerturbationSpec {
                    id: "spawn_dx".to_owned(),
                    path: "initial_state.position_m.x".to_owned(),
                    mode: NumericPerturbationMode::Offset,
                    min: -10.0,
                    max: 10.0,
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
            tags: vec!["test".to_owned(), "terminal".to_owned()],
            metadata: terminal_metadata("nominal", "core"),
        })
    }

    fn checkpoint_entry() -> ScenarioPackEntry {
        ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
            id: "checkpoint_idle_reference".to_owned(),
            scenario: "scenarios/timed_checkpoint_idle.json".to_owned(),
            controller: "idle".to_owned(),
            controller_config: None,
            metadata: checkpoint_metadata(),
        })
    }

    #[test]
    fn standalone_report_prefers_current_lane_context() {
        let pack = ScenarioPackSpec {
            id: "lane_compare_unit".to_owned(),
            name: "Lane compare unit".to_owned(),
            description: "lane compare unit".to_owned(),
            entries: vec![
                terminal_family_entry(
                    "terminal_guidance_clean_nominal_baseline",
                    "terminal_guidance_clean_nominal",
                    "baseline",
                ),
                terminal_family_entry(
                    "terminal_guidance_clean_nominal_staged",
                    "terminal_guidance_clean_nominal",
                    "staged",
                ),
                checkpoint_entry(),
            ],
        };

        let report = run_pack_with_workers(&pack, &fixtures_root(), None, 1).unwrap();
        let html = render_batch_report(
            Path::new("outputs/eval/lane_compare_unit"),
            &report,
            None,
            None,
        );

        assert!(html.contains("<h2>Context</h2>"));
        assert!(html.contains("Report Mode"));
        assert!(html.contains("standalone"));
        assert!(html.contains("current controller lane <code>staged</code>"));
        assert!(html.contains("controller <code>staged_descent_v1</code>"));
        assert!(html.contains("Compare Basis"));
        assert!(html.contains("none"));
        assert!(html.contains("Scope Resolution"));
        assert!(html.contains("current controller lane"));
        assert!(html.contains("Compare Status"));
        assert!(html.contains("standalone"));
        assert!(html.contains("Cache / Promotion"));
        assert!(html.contains("not cached"));
        assert!(!html.contains("data-view-mode=\"compare\""));
        assert!(!html.contains("baseline controller lane <code>baseline</code>"));
        assert!(!html.contains(
            r#"selector-inline">lane</span> <span class="selector-code">baseline</span>"#
        ));
    }

    #[test]
    fn external_compare_report_renders_context_section() {
        let baseline_pack = ScenarioPackSpec {
            id: "compare_baseline_unit".to_owned(),
            name: "Compare baseline unit".to_owned(),
            description: "compare baseline unit".to_owned(),
            entries: vec![
                terminal_family_entry(
                    "terminal_compare_baseline",
                    "terminal_guidance_fixture_nominal",
                    "staged",
                ),
                checkpoint_entry(),
            ],
        };
        let candidate_pack = ScenarioPackSpec {
            id: "compare_candidate_unit".to_owned(),
            name: "Compare candidate unit".to_owned(),
            description: "compare candidate unit".to_owned(),
            entries: vec![
                terminal_family_entry(
                    "terminal_compare_baseline",
                    "terminal_guidance_fixture_nominal",
                    "staged",
                ),
                checkpoint_entry(),
            ],
        };

        let baseline_report =
            run_pack_with_workers(&baseline_pack, &fixtures_root(), None, 1).unwrap();
        let candidate_report =
            run_pack_with_workers(&candidate_pack, &fixtures_root(), None, 1).unwrap();
        let comparison = compare_batch_reports(&candidate_report, &baseline_report);
        let html = render_batch_report(
            Path::new("outputs/eval/compare_candidate_unit"),
            &candidate_report,
            Some((
                Path::new("outputs/eval/compare_baseline_unit"),
                &baseline_report,
            )),
            Some(&comparison),
        );

        assert!(html.contains("<h2>Context</h2>"));
        assert!(html.contains("Report Mode"));
        assert!(html.contains("current-lane history compare"));
        assert!(html.contains("data-view-mode=\"compare\""));
        assert!(html.contains("data-view-mode=\"current-only\""));
        assert!(html.contains("Baseline Source"));
        assert!(html.contains("compare_baseline_unit"));
        assert!(html.contains("Compare Basis"));
        assert!(html.contains("compare baseline from lane"));
        assert!(html.contains(
            "baseline here means the compare target, not the built-in baseline controller"
        ));
        assert!(html.contains("lane <code>staged</code>"));
        assert!(html.contains("shared 2"));
        assert!(html.contains("Scope Resolution"));
        assert!(html.contains("exact"));
        assert!(html.contains("external baseline report provided for this render"));
        assert!(html.contains("Compare Status"));
        assert!(html.contains("available"));
    }

    #[test]
    fn terminal_matrix_report_renders_arc_and_band_levels() {
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_tree_unit".to_owned(),
            name: "Terminal matrix tree unit".to_owned(),
            description: "terminal matrix tree unit".to_owned(),
            entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_clean_nominal".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![
                    TerminalMatrixLaneSpec {
                        id: "baseline".to_owned(),
                        controller: "baseline".to_owned(),
                        controller_config: None,
                    },
                    TerminalMatrixLaneSpec {
                        id: "current".to_owned(),
                        controller: "staged".to_owned(),
                        controller_config: None,
                    },
                ],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "clean".to_owned(),
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                adjustments: Vec::new(),
                tags: vec!["terminal".to_owned(), "bot_lab".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let report = run_pack_with_workers(&pack, &fixtures_root(), None, 1).unwrap();
        let html = render_batch_report(
            Path::new("outputs/eval/terminal_matrix_tree_unit"),
            &report,
            None,
            None,
        );

        assert!(html.contains("selector-inline\">arc</span>"));
        assert!(html.contains("selector-inline\">band</span>"));
        assert!(html.contains("selector-code\">a00</span>"));
        assert!(html.contains("selector-code\">low</span>"));
    }

    #[test]
    fn selector_keys_use_semantic_velocity_band_order() {
        let mut keys = vec![
            "high".to_owned(),
            "low".to_owned(),
            "mid".to_owned(),
            "unspecified".to_owned(),
        ];
        sort_selector_keys(&mut keys);
        assert_eq!(
            keys,
            vec![
                "low".to_owned(),
                "mid".to_owned(),
                "high".to_owned(),
                "unspecified".to_owned(),
            ]
        );
    }

    #[test]
    fn selector_keys_use_semantic_vehicle_variant_order() {
        let mut keys = vec![
            "low_margin".to_owned(),
            "heavy_cargo".to_owned(),
            "nominal".to_owned(),
            "unspecified".to_owned(),
        ];
        sort_selector_keys(&mut keys);
        assert_eq!(
            keys,
            vec![
                "nominal".to_owned(),
                "low_margin".to_owned(),
                "heavy_cargo".to_owned(),
                "unspecified".to_owned(),
            ]
        );
    }
}
