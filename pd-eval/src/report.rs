use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
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
    .header-overview h2 {{
      margin: 0 0 10px;
      font-size: 0.9rem;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--muted);
    }}
    .context-table {{
      width: 100%;
      min-width: 1080px;
    }}
    .context-table thead th {{
      white-space: nowrap;
      font-size: 0.74rem;
      letter-spacing: 0.05em;
      text-transform: uppercase;
      color: var(--muted);
      background: rgba(248,243,234,0.92);
    }}
    .context-table td {{
      vertical-align: top;
    }}
    .context-value {{
      display: grid;
      gap: 2px;
      font-variant-numeric: tabular-nums;
    }}
    .context-main {{
      color: var(--ink);
      font-weight: 700;
      line-height: 1.25;
    }}
    .context-sub {{
      color: var(--muted);
      font-size: 0.78rem;
      line-height: 1.25;
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
      margin-bottom: 12px;
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
    .detail-links {{
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
    }}
    .detail-links a {{
      display: inline-flex;
      padding: 3px 7px;
      border-radius: 999px;
      border: 1px solid var(--line);
      background: var(--surface-strong);
      font-size: 0.77rem;
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
  <div class="page">
    <header class="hero">
      <div>
        <h1>{title_html}</h1>
        <p class="subtitle">{subtitle_html}</p>
        <div class="chip-row">
          <span class="chip"><strong>pack</strong> <span class="mono">{pack_id}</span></span>
          <span class="chip"><strong>runs</strong> {total_runs}</span>
          <span class="chip"><strong>workers</strong> {workers_used}/{workers_requested}</span>
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

	    <section class="panel">
	      <h2>Review Tree</h2>
	      <p>Start here. The selector hierarchy is rendered as a dense tree table, with aggregate rows up top and exact seeded runs hidden underneath the lane rows.</p>
	      {tree_controls}
	      <div id="review-tree-root" class="review-tree-root">{review_tree}</div>
	    </section>

	    {comparison_sections}
	  </div>
  <script>
    (() => {{
      const root = document.getElementById("review-tree-root");
      if (!root) return;
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
      const expandGroups = (table) => {{
        collapseGroups(table);
        summaryRows(table).forEach((row) => {{
          if (row.dataset.kind && row.dataset.kind !== "lane" && row.dataset.group) {{
            row.setAttribute("aria-expanded", "true");
            showImmediateChildren(table, row, false);
          }}
        }});
      }};
      const expandAll = (table) => {{
        allRows(table).forEach((row) => {{
          row.hidden = false;
          if (row.classList.contains("summary-row") && row.dataset.group) {{
            row.setAttribute("aria-expanded", "true");
          }}
        }});
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
          if (action === "expand-groups") {{
            tables().forEach(expandGroups);
          }} else if (action === "collapse-groups") {{
            tables().forEach(collapseGroups);
          }} else if (action === "expand-all") {{
            tables().forEach(expandAll);
          }} else if (action === "collapse-all") {{
            tables().forEach(collapseGroups);
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
        overview_html =
            render_overview_table(candidate, baseline.map(|(_, report)| report), comparison,),
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
            r#"<span class="{}">{}</span> · {} fail"#,
            delta_class(-delta),
            escape_html(&format_percent_delta(delta)),
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
    let sub = match deltas {
        Some((mean_delta, max_delta)) => format!(
            "{} mean · {} max",
            format_signed_seconds(mean_delta),
            format_signed_seconds(max_delta)
        ),
        None => format!("{max_sim_time_s:.2}s max"),
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&main),
        escape_html(&sub)
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
    let sub = if show_delta {
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
        format!("offset {offset} · {fuel_delta} fuel · {offset_delta} off")
    } else {
        format!("offset {offset}")
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&main),
        escape_html(&sub)
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
    let sub = if show_delta {
        metric_delta_value(
            review.reference_gap_mean_m.as_ref(),
            baseline.and_then(|item| item.reference_gap_mean_m.as_ref()),
        )
        .map(|delta| {
            format!(
                "Δ {}",
                format_metric_delta_value(delta, MetricDisplayKind::Meters)
            )
        })
        .unwrap_or_else(|| "Δ -".to_owned())
    } else {
        "mean ref deviation".to_owned()
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&reference),
        escape_html(&sub)
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

fn success_rate_ratio(success_runs: usize, total_runs: usize) -> f64 {
    if total_runs == 0 {
        0.0
    } else {
        success_runs as f64 / total_runs as f64
    }
}

fn overview_lane_split_counts(
    candidate: &BatchReport,
    baseline: Option<&BatchReport>,
    comparison: Option<&BatchComparison>,
) -> Option<(usize, usize, usize)> {
    if baseline.is_some() || comparison.is_some() {
        return None;
    }
    let candidate_records = candidate.records.iter().collect::<Vec<_>>();
    let lane_current_records = controller_lane_records(candidate_records.as_slice(), "staged");
    let lane_baseline_records = controller_lane_records(candidate_records.as_slice(), "baseline");
    if lane_current_records.is_empty() || lane_baseline_records.is_empty() {
        return None;
    }
    let covered = lane_current_records.len() + lane_baseline_records.len();
    Some((
        lane_current_records.len(),
        lane_baseline_records.len(),
        candidate.total_runs.saturating_sub(covered),
    ))
}

fn batch_report_subtitle(
    candidate: &BatchReport,
    baseline: Option<&BatchReport>,
    comparison: Option<&BatchComparison>,
) -> String {
    if let Some((current_runs, baseline_runs, other_runs)) =
        overview_lane_split_counts(candidate, baseline, comparison)
    {
        if other_runs > 0 {
            let reference_label = if other_runs == 1 {
                "reference run"
            } else {
                "reference runs"
            };
            return format!(
                "{}. {} total runs captured for this batch; overview below compares {} current vs {} baseline lane runs and leaves {} {} outside the lane summary.",
                candidate.pack_name,
                candidate.total_runs,
                current_runs,
                baseline_runs,
                other_runs,
                reference_label
            );
        }
        return format!(
            "{}. {} total runs captured for this batch; overview below compares {} current vs {} baseline lane runs.",
            candidate.pack_name, candidate.total_runs, current_runs, baseline_runs
        );
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
    let lane_split = overview_lane_split_counts(candidate, baseline, comparison);
    let (mode, current_source, baseline_source, compare_basis, scope_resolution) =
        if let Some(comparison) = comparison {
        let scope_resolution = if comparison.basis.shared_runs == 0 {
            "no shared scope"
        } else if comparison.basis.candidate_only_runs == 0
            && comparison.basis.baseline_only_runs == 0
        {
            "exact"
        } else {
            "shared intersection"
        };
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
                        r#"<div class="context-value"><div class="context-main"><code>{}</code> · {}</div><div class="context-sub">spec <code>{}</code> · resolved <code>{}</code> · Baseline Resolution: explicit baseline report · resolved from --baseline-dir for this render</div></div>"#,
                        escape_html(&baseline.pack_id),
                        escape_html(&baseline.pack_name),
                        escape_html(&short_digest(&baseline.identity.pack_spec_digest)),
                        escape_html(&short_digest(&baseline.identity.resolved_run_digest)),
                    )
                }).unwrap_or_else(|| {
                    context_value(
                        "missing baseline report",
                        "Baseline Resolution: none · not applicable for this batch page",
                    )
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
                    scope_resolution,
                    if scope_resolution == "exact" {
                        "candidate and baseline cover the same resolved run set"
                    } else if scope_resolution == "shared intersection" {
                        "report deltas are limited to the shared run intersection"
                    } else {
                        "no shared run set was available for comparison"
                    },
                ),
            )
        } else if let Some((current_runs, baseline_runs, other_runs)) = lane_split {
            (
                "lane compare",
                format!(
                    r#"<div class="context-value"><div class="context-main">current lane <code>staged</code> within <code>{}</code></div><div class="context-sub">{} · {} current runs</div></div>"#,
                    escape_html(&candidate.pack_id),
                    escape_html(&candidate.pack_name),
                    current_runs,
                ),
                format!(
                    r#"<div class="context-value"><div class="context-main">baseline lane <code>baseline</code> within <code>{}</code></div><div class="context-sub">{} baseline runs{} · Baseline Resolution: internal lane pairing · current and baseline are lanes inside the same pack</div></div>"#,
                    escape_html(&candidate.pack_id),
                    baseline_runs,
                    if other_runs > 0 {
                        format!(" · {} reference runs excluded", other_runs)
                    } else {
                        String::new()
                    },
                ),
                context_value(
                    &format!(
                        "lane_id within pack · current {} · baseline {}{}",
                        current_runs,
                        baseline_runs,
                        if other_runs > 0 {
                            format!(" · other {}", other_runs)
                        } else {
                            String::new()
                        }
                    ),
                    "lane rows are compared within the shared selector space of this pack",
                ),
                context_value(
                    "internal lane",
                    "this is a within-pack lane comparison, not an external baseline",
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
                context_value(
                    "none",
                    "Baseline Resolution: none · not applicable for this batch page",
                ),
                context_none_value("none"),
                context_value(
                    "full pack",
                    "the overview and tree reflect the full batch without a comparison basis",
                ),
            )
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
        } else if lane_split.is_some() {
            (
                "available",
                "ok",
                "internal lane compare available within this pack",
            )
        } else {
            (
                "standalone",
                "muted",
                "no baseline or internal lane compare requested",
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
        context_value(
            "not modeled",
            "pd-lab does not yet expose cached result reuse, promotion, or invalidation state",
        ),
    )
}

fn context_value(main: &str, sub: &str) -> String {
    format!(
        r#"<div class="context-value"><div class="context-main">{}</div><div class="context-sub">{}</div></div>"#,
        escape_html(main),
        escape_html(sub),
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
) -> String {
    let candidate_scope = selector_scope_counts(candidate);
    let candidate_records = candidate.records.iter().collect::<Vec<_>>();
    let candidate_review = review_aggregate_from_records(candidate_records.as_slice());
    let lane_current_records = controller_lane_records(candidate_records.as_slice(), "staged");
    let lane_baseline_records = controller_lane_records(candidate_records.as_slice(), "baseline");
    let lane_current_aggregate = (!lane_current_records.is_empty())
        .then(|| review_aggregate_from_records(lane_current_records.as_slice()));
    let lane_baseline_aggregate = (!lane_baseline_records.is_empty())
        .then(|| review_aggregate_from_records(lane_baseline_records.as_slice()));
    let split_by_lane = overview_lane_split_counts(candidate, baseline, comparison).is_some()
        && lane_current_aggregate.is_some()
        && lane_baseline_aggregate.is_some();
    let baseline_scope = baseline.map(selector_scope_counts);
    let baseline_records = baseline
        .map(|report| report.records.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let baseline_review = (!baseline_records.is_empty())
        .then(|| review_aggregate_from_records(baseline_records.as_slice()));

    let mut rows = if split_by_lane {
        let current_scope = selector_scope_counts_from_records(lane_current_records.as_slice());
        let baseline_lane_scope =
            selector_scope_counts_from_records(lane_baseline_records.as_slice());
        let current_review = lane_current_aggregate
            .as_ref()
            .expect("current lane aggregate");
        let baseline_lane_review = lane_baseline_aggregate
            .as_ref()
            .expect("baseline lane aggregate");
        let (current_mean_sim_time_s, current_max_sim_time_s) =
            overview_timing_from_records(lane_current_records.as_slice());
        let (baseline_mean_sim_time_s, baseline_max_sim_time_s) =
            overview_timing_from_records(lane_baseline_records.as_slice());
        let success_rate_delta =
            success_rate_ratio(current_review.success_runs, current_review.total_runs)
                - success_rate_ratio(
                    baseline_lane_review.success_runs,
                    baseline_lane_review.total_runs,
                );

        vec![
            render_overview_row(
                "current-summary-row",
                format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag current">current</span><code>{}</code></div><div class="overview-sub">{} · lane current</div></div>"#,
                    escape_html(&candidate.pack_id),
                    escape_html(&candidate.pack_name)
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code></div></div>"#,
                    escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                    escape_html(&short_digest(&candidate.identity.resolved_run_digest))
                ),
                render_overview_scope_cell(&current_scope, candidate.workers_used, None),
                render_overview_result_cell(
                    current_review.success_runs,
                    current_review.total_runs,
                    current_review.failure_runs,
                    Some(success_rate_delta),
                ),
                render_overview_timing_cell(
                    current_mean_sim_time_s,
                    current_max_sim_time_s,
                    Some((
                        current_mean_sim_time_s - baseline_mean_sim_time_s,
                        current_max_sim_time_s - baseline_max_sim_time_s,
                    )),
                ),
                render_overview_efficiency_cell(current_review, Some(baseline_lane_review), true),
                render_overview_tracking_cell(current_review, Some(baseline_lane_review), true),
            ),
            render_overview_row(
                "baseline-summary-row baseline-row",
                format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag baseline">baseline</span><code>{}</code></div><div class="overview-sub">{} · lane baseline</div></div>"#,
                    escape_html(&candidate.pack_id),
                    escape_html(&candidate.pack_name)
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code></div></div>"#,
                    escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                    escape_html(&short_digest(&candidate.identity.resolved_run_digest))
                ),
                render_overview_scope_cell(&baseline_lane_scope, candidate.workers_used, None),
                render_overview_result_cell(
                    baseline_lane_review.success_runs,
                    baseline_lane_review.total_runs,
                    baseline_lane_review.failure_runs,
                    None,
                ),
                render_overview_timing_cell(
                    baseline_mean_sim_time_s,
                    baseline_max_sim_time_s,
                    None,
                ),
                render_overview_efficiency_cell(baseline_lane_review, None, false),
                render_overview_tracking_cell(baseline_lane_review, None, false),
            ),
            render_overview_row(
                "diff-summary-row",
                format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag diff">diff</span>lane compare</div><div class="overview-sub">current - baseline within pack</div></div>"#
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main"><code>{}</code></div><div class="overview-sub">shared selector space</div></div>"#,
                    escape_html(&candidate.pack_id)
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main">{} groups · {} missions</div><div class="overview-sub">workers {} · same pack lanes</div></div>"#,
                    current_scope.case_groups, current_scope.missions, candidate.workers_used
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main {}">{}</div><div class="overview-sub {}">{}</div></div>"#,
                    delta_class(-success_rate_delta),
                    escape_html(&format_percent_delta(success_rate_delta)),
                    delta_class(
                        (current_review.failure_runs as i64
                            - baseline_lane_review.failure_runs as i64)
                            as f64
                    ),
                    escape_html(&format_signed_i64(
                        current_review.failure_runs as i64
                            - baseline_lane_review.failure_runs as i64
                    ))
                ),
                format!(
                    r#"<div class="overview-stack"><div class="overview-main {}">{}</div><div class="overview-sub">{}</div></div>"#,
                    delta_class(current_mean_sim_time_s - baseline_mean_sim_time_s),
                    escape_html(&format_signed_seconds(
                        current_mean_sim_time_s - baseline_mean_sim_time_s
                    )),
                    escape_html(&format!(
                        "max {}",
                        format_signed_seconds(current_max_sim_time_s - baseline_max_sim_time_s)
                    ))
                ),
                render_overview_efficiency_diff_cell(current_review, Some(baseline_lane_review)),
                render_overview_tracking_diff_cell(current_review, Some(baseline_lane_review)),
            ),
        ]
    } else {
        vec![render_overview_row(
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
        )]
    };

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
                escape_html(&format!("max {}", format_signed_seconds(comparison.summary.max_sim_time_delta_s)))
            ),
            render_overview_efficiency_diff_cell(&candidate_review, baseline_review.as_ref()),
            render_overview_tracking_diff_cell(&candidate_review, baseline_review.as_ref()),
        ));
    }

    format!(
        r#"<section class="header-overview">
  <h2>Overview</h2>
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
        rows.join("")
    )
}

fn render_tree_controls(has_compare: bool) -> String {
    let mut buttons = vec![
        r#"<button type="button" data-tree-action="expand-groups">Expand Groups</button>"#
            .to_owned(),
        r#"<button type="button" data-tree-action="collapse-groups">Collapse Groups</button>"#
            .to_owned(),
        r#"<button type="button" data-tree-action="expand-all">Expand All</button>"#.to_owned(),
        r#"<button type="button" data-tree-action="collapse-all">Collapse All</button>"#.to_owned(),
    ];
    if has_compare {
        buttons.push(
            r#"<button type="button" data-tree-action="toggle-baseline">Hide Baseline</button>"#
                .to_owned(),
        );
        buttons.push(
            r#"<button type="button" data-tree-action="toggle-diff">Show Changed Only</button>"#
                .to_owned(),
        );
    }
    format!(r#"<div class="tree-controls">{}</div>"#, buttons.join(""))
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
type VehicleRecordGroups<'a> = BTreeMap<String, LaneRecordGroups<'a>>;
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
    let candidate_tree = records_by_selector_hierarchy(candidate);
    let baseline_tree = baseline
        .map(records_by_selector_hierarchy)
        .unwrap_or_default();
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
    let lane_current_aggregate = controller_lane_aggregate(candidate_records.as_slice(), "staged");
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
          <th>Details</th>
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
    let lane_current_aggregate = controller_lane_aggregate(candidate_records.as_slice(), "staged");
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
    let lane_current_aggregate = controller_lane_aggregate(candidate_records.as_slice(), "staged");
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
    let lane_current_aggregate = controller_lane_aggregate(candidate_records.as_slice(), "staged");
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
    let empty_candidate = LaneRecordGroups::new();
    let empty_baseline = LaneRecordGroups::new();
    let candidate_lanes = candidate_lanes.unwrap_or(&empty_candidate);
    let baseline_lanes = baseline_lanes.unwrap_or(&empty_baseline);
    let mut lane_keys = merged_map_keys(candidate_lanes, Some(baseline_lanes));
    sort_lane_keys(&mut lane_keys);
    let group_id = tree_group_id(&[
        "vehicle",
        mission,
        arrival_family,
        condition_set,
        vehicle_variant,
    ]);
    let lane_rows = lane_keys
        .iter()
        .map(|lane_id| {
            render_lane_review_section(
                mission,
                arrival_family,
                condition_set,
                vehicle_variant,
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
        expectation.as_deref(),
        current_row_aggregate.is_some(),
        baseline_row_aggregate.is_some(),
    );
    rows.push_str(&render_summary_row(
        vehicle_variant,
        depth,
        parent_group_id,
        (!lane_keys.is_empty()).then_some(group_id.as_str()),
        "vehicle",
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
            && row.lane_id == lane_id
    });
    let changed = aggregate_changed(aggregate.as_ref(), baseline_aggregate.as_ref());
    let group_id = tree_group_id(&[
        "lane",
        mission,
        arrival_family,
        condition_set,
        vehicle_variant,
        lane_id,
    ]);
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
    let lane_label = display_lane_label(lane_id);
    let current_note = render_summary_note(
        comparison.is_some(),
        TreeRowTone::Current,
        comparison.is_some().then_some(regression_count),
        None,
        aggregate.is_some(),
        baseline_aggregate.is_some(),
    );
    rows.push_str(&render_summary_row(
        lane_label.as_str(),
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
        rows.push_str(&render_summary_row(
            lane_label.as_str(),
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
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
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
            show_baseline.then_some("cur"),
            TreeRowTone::Current,
            run_change_map,
            output_dir,
            candidate_record_map,
            baseline_record_map,
        ));
        if show_baseline
            && let Some(baseline_record) = baseline_by_run_id.remove(&record.resolved.run_id)
        {
            rows.push_str(&render_seed_run_row(
                baseline_record,
                depth,
                parent_group_id,
                Some("base"),
                TreeRowTone::Baseline,
                run_change_map,
                output_dir,
                candidate_record_map,
                baseline_record_map,
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
                Some("base"),
                TreeRowTone::Baseline,
                run_change_map,
                output_dir,
                candidate_record_map,
                baseline_record_map,
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
        (true, false, TreeRowTone::Current) => {
            items.push(r#"<span class="emph">current only</span>"#.to_owned());
        }
        (false, true, TreeRowTone::Current) => {
            items.push(r#"<span class="bad">missing in current</span>"#.to_owned());
        }
        (false, true, TreeRowTone::Baseline) => {
            items.push(r#"<span class="emph">baseline only</span>"#.to_owned());
        }
        (true, false, TreeRowTone::Baseline) => {
            items.push(r#"<span class="muted">not in baseline</span>"#.to_owned());
        }
        _ => {}
    }
    if show_compare
        && tone == TreeRowTone::Current
        && let Some(regression_count) = regression_count
        && regression_count > 0
    {
        let suffix = if regression_count == 1 { "" } else { "s" };
        items.push(format!(
            r#"<span class="bad">{} regression{}</span>"#,
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
    compare_tag: Option<&str>,
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
    let tag_html = compare_tag
        .map(|tag| {
            let tag_class = match (kind, tone) {
                ("lane", TreeRowTone::Current) => "row-tag lane-current",
                ("lane", TreeRowTone::Baseline) => "row-tag lane-baseline",
                (_, TreeRowTone::Current) => "row-tag current",
                (_, TreeRowTone::Baseline) => "row-tag baseline",
            };
            let tag_text = match tag {
                "cur" => "current",
                "base" => "baseline",
                other => other,
            };
            format!(
                r#"<span class="{}">{}</span>"#,
                tag_class,
                escape_html(tag_text)
            )
        })
        .or_else(|| {
            if kind == "lane" {
                let class = match label {
                    "current" => Some("row-tag lane-current"),
                    "baseline" => Some("row-tag lane-baseline"),
                    _ => None,
                }?;
                Some(format!(
                    r#"<span class="{}">{}</span>"#,
                    class,
                    escape_html(label)
                ))
            } else {
                None
            }
        })
        .unwrap_or_default();
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
        outcome = escape_html(&outcome_html),
        fuel = escape_html(&fuel_html),
        flight = escape_html(&flight_html),
        offset = escape_html(&offset_html),
        reference = escape_html(&ref_html),
        note = note_html,
    )
}

fn render_seed_run_row(
    record: &crate::BatchRunRecord,
    depth: usize,
    parent_group_id: &str,
    compare_tag: Option<&str>,
    tone: TreeRowTone,
    run_change_map: &BTreeMap<String, (&'static str, &'static str)>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
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
            "staged" => row_classes.push("lane-controller-current"),
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

    let tag_html = compare_tag
        .map(|tag| {
            let class = match tone {
                TreeRowTone::Current => "row-tag current",
                TreeRowTone::Baseline => "row-tag baseline",
            };
            let label = match tag {
                "cur" => "current",
                "base" => "baseline",
                other => other,
            };
            format!(r#"<span class="{}">{}</span>"#, class, escape_html(label))
        })
        .or_else(|| {
            if tone == TreeRowTone::Current {
                let (class, label) = match record.resolved.lane_id.as_str() {
                    "staged" => ("row-tag lane-current", Some("current")),
                    "baseline" => ("row-tag lane-baseline", Some("baseline")),
                    _ => ("", None),
                };
                label.map(|label| {
                    format!(r#"<span class="{}">{}</span>"#, class, escape_html(label))
                })
            } else {
                None
            }
        })
        .unwrap_or_default();
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
        r#"{detail_note}<div class="detail-links">{links}</div>"#,
        detail_note = detail_note,
        links = render_dual_links(
            &record.resolved.run_id,
            candidate_record_map,
            baseline_record_map,
            output_dir,
        ),
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
    let mut grouped = MissionRecordGroups::new();
    for record in &candidate.records {
        grouped
            .entry(record.resolved.selector.mission.clone())
            .or_default()
            .entry(record.resolved.selector.arrival_family.clone())
            .or_default()
            .entry(record.resolved.selector.condition_set.clone())
            .or_default()
            .entry(record.resolved.selector.vehicle_variant.clone())
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
            _ => lhs.cmp(rhs),
        }
    });
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
        "staged" => 0,
        "baseline" => 1,
        _ => 2,
    }
}

fn display_lane_label(lane_id: &str) -> String {
    match lane_id {
        "staged" => "current".to_owned(),
        "baseline" => "baseline".to_owned(),
        _ => lane_id.to_owned(),
    }
}

fn flatten_arrival_records<'a>(
    groups: Option<&ArrivalRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
    groups
        .map(|groups| {
            groups
                .values()
                .flat_map(|conditions| conditions.values())
                .flat_map(|vehicles| vehicles.values())
                .flat_map(|lanes| lanes.values())
                .flatten()
                .copied()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn flatten_condition_records<'a>(
    groups: Option<&ConditionRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
    groups
        .map(|groups| {
            groups
                .values()
                .flat_map(|vehicles| vehicles.values())
                .flat_map(|lanes| lanes.values())
                .flatten()
                .copied()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn flatten_vehicle_records<'a>(
    groups: Option<&VehicleRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
    groups
        .map(|groups| {
            groups
                .values()
                .flat_map(|lanes| lanes.values())
                .flatten()
                .copied()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn flatten_lane_records<'a>(
    groups: Option<&LaneRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
    groups
        .map(|groups| groups.values().flatten().copied().collect::<Vec<_>>())
        .unwrap_or_default()
}

fn selector_case_key(selector: &crate::SelectorAxes) -> String {
    format!(
        "{} / {} / {} / {}",
        selector.mission, selector.arrival_family, selector.condition_set, selector.vehicle_variant
    )
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

fn controller_lane_aggregate(
    records: &[&crate::BatchRunRecord],
    lane_id: &str,
) -> Option<ReviewAggregate> {
    let filtered = controller_lane_records(records, lane_id);
    (!filtered.is_empty()).then(|| review_aggregate_from_records(filtered.as_slice()))
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
        SummaryMetricStyle::MeanStddev => format_metric_summary(Some(summary), kind),
        SummaryMetricStyle::MeanDelta => {
            let value = format_metric_value(summary, kind);
            match metric_delta_value(Some(summary), baseline) {
                Some(delta) => format!("{value} ({})", format_metric_delta_value(delta, kind)),
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
        SummaryMetricStyle::MeanStddev => base,
        SummaryMetricStyle::MeanDelta => {
            let Some(baseline) = baseline else {
                return base;
            };
            let delta = (crate::success_rate(aggregate.success_runs, aggregate.total_runs)
                - crate::success_rate(baseline.success_runs, baseline.total_runs))
                * 100.0;
            format!("{base} ({delta:+.1}pt)")
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

fn render_link_row_for_bundle(label: &str, bundle_dir: &str, output_dir: &Path) -> String {
    let bundle_dir = resolve_repo_relative(Path::new(bundle_dir));
    let site_report_path = report_site_output_for_batch_run(&bundle_dir);
    let report_path = bundle_dir.join("report.html");
    let manifest_path = bundle_dir.join("manifest.json");
    let href = if site_report_path.as_ref().is_some_and(|path| path.is_file()) {
        relative_href(
            output_dir,
            site_report_path.as_ref().expect("checked above"),
        )
    } else if report_path.is_file() {
        relative_href(output_dir, &report_path)
    } else {
        relative_href(output_dir, &manifest_path)
    };
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
        compare_batch_reports, run_pack_with_workers,
    };

    use super::render_batch_report;

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
    fn lane_compare_report_renders_context_section() {
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
        assert!(html.contains("lane compare"));
        assert!(html.contains("Compare Basis"));
        assert!(html.contains("lane_id within pack"));
        assert!(html.contains("Scope Resolution"));
        assert!(html.contains("internal lane"));
        assert!(html.contains("Compare Status"));
        assert!(html.contains("available"));
        assert!(html.contains("Cache / Promotion"));
        assert!(html.contains("not modeled"));
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
                    "baseline",
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
                    "idle",
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
        assert!(html.contains("external compare"));
        assert!(html.contains("Baseline Source"));
        assert!(html.contains("compare_baseline_unit"));
        assert!(html.contains("Compare Basis"));
        assert!(html.contains("run_id"));
        assert!(html.contains("shared 3"));
        assert!(html.contains("Scope Resolution"));
        assert!(html.contains("exact"));
        assert!(html.contains("Compare Status"));
        assert!(html.contains("available"));
    }
}
