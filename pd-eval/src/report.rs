use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    BatchComparison, BatchGroupComparison, BatchReport, BatchRunComparison, BatchRunPointer,
    compare_batch_reports,
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
    .summary-grid {{
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 12px;
      margin-bottom: 16px;
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
      font-size: 0.86rem;
      font-variant-numeric: tabular-nums;
    }}
    th, td {{
      text-align: left;
      padding: 8px 9px;
      border-bottom: 1px solid rgba(215,205,189,0.75);
      vertical-align: top;
    }}
    th {{
      color: var(--muted);
      font-weight: 700;
      font-size: 0.78rem;
      text-transform: uppercase;
      letter-spacing: 0.04em;
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
    .delta-pos {{ color: var(--bad); }}
    .delta-neg {{ color: var(--good); }}
    .delta-flat {{ color: var(--muted); }}
    details {{
      border: 1px solid var(--line);
      border-radius: 14px;
      background: rgba(255,253,248,0.88);
      padding: 10px 12px;
      margin-top: 10px;
    }}
    details summary {{
      cursor: pointer;
      font-weight: 700;
    }}
    .link-row {{
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
    }}
    .link-row a {{
      display: inline-flex;
      padding: 3px 7px;
      border-radius: 999px;
      border: 1px solid var(--line);
      background: var(--surface-strong);
      font-size: 0.77rem;
    }}
    @media (max-width: 1100px) {{
      .summary-grid {{
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }}
      .layout {{
        grid-template-columns: 1fr;
      }}
      .hero {{
        flex-direction: column;
      }}
      .hero-actions {{
        justify-content: flex-start;
      }}
    }}
    @media (max-width: 700px) {{
      .summary-grid {{
        grid-template-columns: 1fr;
      }}
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

    <section class="summary-grid">
      {candidate_cards}
      {comparison_cards}
    </section>

    <section class="layout">
      <div class="panel">
        <h2>By Family</h2>
        <p>Grouped view for seeded sweeps and pinned scenario families.</p>
        <div class="table-wrap">{family_table}</div>
      </div>
      <div class="panel">
        <h2>By Entry</h2>
        <p>Pack-entry view with success rate, failures, and representative runs.</p>
        <div class="table-wrap">{entry_table}</div>
      </div>
    </section>

    <section class="layout">
      <div class="panel">
        <h2>Failed Runs</h2>
        <p>Candidate failures first, with direct links back to the recorded bundle.</p>
        <div class="table-wrap">{failed_runs_table}</div>
      </div>
      <div class="panel">
        <h2>Slowest Runs</h2>
        <p>Longest-running candidate executions, useful for controller and profile triage.</p>
        <div class="table-wrap">{slowest_runs_table}</div>
      </div>
    </section>

    {comparison_sections}
  </div>
</body>
</html>"#,
        title = escape_html(&title),
        title_html = escape_html(&title),
        subtitle_html = escape_html(&format!(
            "{}. {} total runs captured for this batch.",
            candidate.pack_name, candidate.total_runs
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
        candidate_cards = render_candidate_cards(candidate),
        comparison_cards = comparison.map(render_comparison_cards).unwrap_or_default(),
        family_table = render_group_table(
            comparison
                .map(|comparison| &comparison.by_family)
                .map(|groups| groups.as_slice()),
            &candidate.summary.by_family,
            &candidate_record_links,
            &baseline_record_map,
            &output_dir,
        ),
        entry_table = render_group_table(
            comparison
                .map(|comparison| &comparison.by_entry)
                .map(|groups| groups.as_slice()),
            &candidate.summary.by_entry,
            &candidate_record_links,
            &baseline_record_map,
            &output_dir,
        ),
        failed_runs_table = render_run_pointer_table(
            &candidate.summary.failed_runs,
            &output_dir,
            "No candidate failures recorded."
        ),
        slowest_runs_table = render_run_pointer_table(
            &candidate.summary.slowest_runs,
            &output_dir,
            "No candidate runs recorded."
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

fn render_candidate_cards(candidate: &BatchReport) -> String {
    let success_rate = percentage(candidate.summary.success_runs, candidate.summary.total_runs);
    [
        format!(
            r#"<article class="card">
  <h2>Results</h2>
  <div class="metric"><span>Success</span><strong class="good">{}/{}</strong></div>
  <div class="metric"><span>Failure</span><strong class="bad">{}</strong></div>
  <div class="metric"><span>Success rate</span><strong>{:.1}%</strong></div>
</article>"#,
            candidate.summary.success_runs,
            candidate.summary.total_runs,
            candidate.summary.failure_runs,
            success_rate
        ),
        format!(
            r#"<article class="card">
  <h2>Timing</h2>
  <div class="metric"><span>Mean sim time</span><strong>{:.2}s</strong></div>
  <div class="metric"><span>Max sim time</span><strong>{:.2}s</strong></div>
  <div class="metric"><span>Workers used</span><strong>{}</strong></div>
</article>"#,
            candidate.summary.mean_sim_time_s,
            candidate.summary.max_sim_time_s,
            candidate.workers_used
        ),
        format!(
            r#"<article class="card">
  <h2>Outcomes</h2>
  {}
</article>"#,
            candidate
                .summary
                .end_reasons
                .iter()
                .take(4)
                .map(|(label, count)| {
                    format!(
                        r#"<div class="metric"><span>{}</span><strong>{}</strong></div>"#,
                        escape_html(label),
                        count
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        ),
        format!(
            r#"<article class="card">
  <h2>Batch Shape</h2>
  <div class="metric"><span>Families</span><strong>{}</strong></div>
  <div class="metric"><span>Entries</span><strong>{}</strong></div>
  <div class="metric"><span>Failed runs listed</span><strong>{}</strong></div>
</article>"#,
            candidate.summary.by_family.len(),
            candidate.summary.by_entry.len(),
            candidate.summary.failed_runs.len()
        ),
    ]
    .join("")
}

fn render_comparison_cards(comparison: &BatchComparison) -> String {
    [
        format!(
            r#"<article class="card">
  <h2>Baseline</h2>
  <div class="metric"><span>Pack</span><strong class="mono">{}</strong></div>
  <div class="metric"><span>Name</span><strong>{}</strong></div>
  <div class="metric"><span>Mode</span><strong>{}</strong></div>
</article>"#,
            escape_html(&comparison.baseline_pack_id),
            escape_html(&comparison.baseline_pack_name),
            escape_html(&comparison.basis.mode)
        ),
        format!(
            r#"<article class="card">
  <h2>Compare Basis</h2>
  <div class="metric"><span>Shared runs</span><strong>{}</strong></div>
  <div class="metric"><span>Candidate only</span><strong>{}</strong></div>
  <div class="metric"><span>Baseline only</span><strong>{}</strong></div>
</article>"#,
            comparison.basis.shared_runs,
            comparison.basis.candidate_only_runs,
            comparison.basis.baseline_only_runs
        ),
        format!(
            r#"<article class="card">
  <h2>Global Delta</h2>
  <div class="metric"><span>Success rate</span><strong class="{success_class}">{success_delta}</strong></div>
  <div class="metric"><span>Failures</span><strong class="{failure_class}">{failure_delta}</strong></div>
  <div class="metric"><span>Mean sim time</span><strong class="{time_class}">{time_delta}</strong></div>
</article>"#,
            success_class = delta_class(-comparison.summary.success_rate_delta),
            success_delta = format_percent_delta(comparison.summary.success_rate_delta),
            failure_class = delta_class(comparison.summary.failure_runs_delta as f64),
            failure_delta = format_signed_i64(comparison.summary.failure_runs_delta),
            time_class = delta_class(comparison.summary.mean_sim_time_delta_s),
            time_delta = format_signed_seconds(comparison.summary.mean_sim_time_delta_s)
        ),
        format!(
            r#"<article class="card">
  <h2>Run Changes</h2>
  <div class="metric"><span>New failures</span><strong class="bad">{}</strong></div>
  <div class="metric"><span>Recovered</span><strong class="good">{}</strong></div>
  <div class="metric"><span>Other changes</span><strong>{}</strong></div>
</article>"#,
            comparison.regressions.len(),
            comparison.improvements.len(),
            comparison.outcome_changes.len()
        ),
    ]
    .join("")
}

fn render_group_table(
    comparisons: Option<&[BatchGroupComparison]>,
    candidate_groups: &[crate::BatchGroupSummary],
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
    output_dir: &Path,
) -> String {
    if let Some(comparisons) = comparisons {
        let rows = comparisons
            .iter()
            .map(|group| {
                format!(
                    "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    escape_html(&group.key),
                    render_optional_runs(group.candidate_total_runs, group.candidate_success_rate),
                    render_optional_runs(group.baseline_total_runs, group.baseline_success_rate),
                    render_optional_delta(group.success_rate_delta, ValueKind::PercentPoints),
                    render_optional_delta(group.failure_runs_delta.map(|value| value as f64), ValueKind::Count),
                    render_optional_delta(group.mean_sim_time_delta_s, ValueKind::Seconds),
                    render_sample_links(
                        &group.sample_run_ids,
                        candidate_record_map,
                        baseline_record_map,
                        output_dir,
                    ),
                )
            })
            .collect::<Vec<_>>()
            .join("");
        return format!(
            "<table><thead><tr><th>Key</th><th>Candidate</th><th>Baseline</th><th>Success Δ</th><th>Fail Δ</th><th>Mean Δ</th><th>Samples</th></tr></thead><tbody>{}</tbody></table>",
            rows
        );
    }

    let rows = candidate_groups
        .iter()
        .map(|group| {
            format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{:.2}s</td><td class=\"seed-list\">{}</td><td>{}</td></tr>",
                escape_html(&group.key),
                render_rate(group.success_runs, group.total_runs),
                group.failure_runs,
                group.mean_sim_time_s,
                render_seed_list(&group.failed_seeds),
                render_sample_links(
                    &group.sample_run_ids,
                    candidate_record_map,
                    &BTreeMap::new(),
                    output_dir,
                ),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<table><thead><tr><th>Key</th><th>Success</th><th>Failures</th><th>Mean</th><th>Failed seeds</th><th>Samples</th></tr></thead><tbody>{}</tbody></table>",
        rows
    )
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
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{:.2}s</td><td>{}</td></tr>",
                escape_html(&row.run_id),
                escape_html(&row.mission_outcome),
                escape_html(&row.end_reason),
                row.scenario_seed,
                row.sim_time_s,
                render_pointer_links(row, output_dir),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<table><thead><tr><th>Run</th><th>Outcome</th><th>Reason</th><th>Seed</th><th>Sim</th><th>Links</th></tr></thead><tbody>{}</tbody></table>",
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
                "<tr><td><code>{}</code></td><td>{}</td><td>{} → {}</td><td>{:.2}s</td><td>{}</td></tr>",
                escape_html(&row.run_id),
                escape_html(&enum_label(&row.change_kind)),
                escape_html(&row.baseline_mission_outcome),
                escape_html(&row.candidate_mission_outcome),
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
        "<table><thead><tr><th>Run</th><th>Kind</th><th>Outcome</th><th>Sim Δ</th><th>Links</th></tr></thead><tbody>{}</tbody></table>",
        body
    )
}

fn render_optional_runs(total_runs: Option<usize>, success_rate: Option<f64>) -> String {
    match (total_runs, success_rate) {
        (Some(total_runs), Some(success_rate)) => {
            render_rate_from_fraction(success_rate, total_runs)
        }
        _ => r#"<span class="muted">-</span>"#.to_owned(),
    }
}

fn render_optional_delta(value: Option<f64>, kind: ValueKind) -> String {
    match value {
        Some(value) => {
            let (class, label) = match kind {
                ValueKind::PercentPoints => (delta_class(-value), format_percent_delta(value)),
                ValueKind::Count => (delta_class(value), format_signed_i64(value as i64)),
                ValueKind::Seconds => (delta_class(value), format_signed_seconds(value)),
            };
            format!(r#"<span class="{}">{}</span>"#, class, label)
        }
        None => r#"<span class="muted">-</span>"#.to_owned(),
    }
}

fn render_rate(success_runs: usize, total_runs: usize) -> String {
    render_rate_from_fraction(crate::success_rate(success_runs, total_runs), total_runs)
}

fn render_rate_from_fraction(rate: f64, total_runs: usize) -> String {
    format!(
        r#"<div class="rate"><div>{:.1}% of {}</div><div class="rate-bar"><div class="rate-fill" style="width: {:.3}%"></div></div></div>"#,
        rate * 100.0,
        total_runs,
        (rate * 100.0).clamp(0.0, 100.0)
    )
}

fn render_seed_list(seeds: &[u64]) -> String {
    if seeds.is_empty() {
        return "none".to_owned();
    }
    let mut rendered = seeds
        .iter()
        .take(6)
        .map(|seed| seed.to_string())
        .collect::<Vec<_>>();
    if seeds.len() > 6 {
        rendered.push(format!("+{}", seeds.len() - 6));
    }
    rendered.join(", ")
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

fn render_sample_links(
    run_ids: &[String],
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
    output_dir: &Path,
) -> String {
    if run_ids.is_empty() {
        return r#"<span class="muted">-</span>"#.to_owned();
    }
    let links = run_ids
        .iter()
        .take(4)
        .map(|run_id| {
            render_dual_links(
                run_id,
                candidate_record_map,
                baseline_record_map,
                output_dir,
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(r#"<div class="link-row">{}</div>"#, links)
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
    let report_path = bundle_dir.join("report.html");
    let manifest_path = bundle_dir.join("manifest.json");
    let href = if report_path.is_file() {
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

fn format_signed_i64(value: i64) -> String {
    format!("{value:+}")
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

enum ValueKind {
    PercentPoints,
    Count,
    Seconds,
}
