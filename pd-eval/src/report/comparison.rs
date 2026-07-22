use super::*;

pub(super) fn render_run_pointer_table(
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

pub(super) fn render_comparison_sections(
    output_dir: &Path,
    comparison: &BatchComparison,
    candidate_record_map: &BTreeMap<String, String>,
    baseline_record_map: &BTreeMap<String, String>,
) -> String {
    format!(
        r#"{}<section class="layout">
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
        render_regression_policy_panel(comparison),
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

pub(super) fn render_regression_policy_panel(comparison: &BatchComparison) -> String {
    format!(
        r#"<section class="layout">
  <div class="panel">
    <h2>Regression Policy</h2>
    <p>{} {}</p>
    <div class="table-wrap">{}</div>
  </div>
</section>
"#,
        render_policy_status_chip(comparison.policy.status),
        escape_html(&comparison.policy.summary),
        render_regression_policy_table(&comparison.policy.rules),
    )
}

pub(super) fn render_regression_policy_table(rules: &[BatchRegressionPolicyRuleResult]) -> String {
    if rules.is_empty() {
        return r#"<p class="muted">No regression policy rules recorded.</p>"#.to_owned();
    }

    let body = rules
        .iter()
        .map(|rule| {
            format!(
                "<tr><td><code>{}</code><div class=\"muted\">{}</div></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&rule.id),
                escape_html(&rule.label),
                render_policy_status_chip(rule.status),
                escape_html(&rule.observed),
                escape_html(&rule.threshold),
                escape_html(&rule.note),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        "<table><thead><tr><th>Rule</th><th>Status</th><th>Observed</th><th>Threshold</th><th>Note</th></tr></thead><tbody>{body}</tbody></table>"
    )
}

pub(super) fn render_run_comparison_table(
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

pub(super) fn render_pointer_focus(pointer: &BatchRunPointer) -> String {
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

pub(super) fn render_pointer_margin(pointer: &BatchRunPointer) -> String {
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

pub(super) fn render_comparison_margin_delta(row: &BatchRunComparison) -> String {
    match row.margin_ratio_delta {
        Some(delta) => format!(
            r#"<span class="{}">{}</span>"#,
            delta_class(-delta),
            format_margin_delta(delta)
        ),
        None => r#"<span class="muted">-</span>"#.to_owned(),
    }
}

pub(super) fn render_comparison_fuel_delta(row: &BatchRunComparison) -> String {
    format!(
        r#"<span class="{}">{}</span>"#,
        delta_class(-row.fuel_remaining_delta_kg),
        format_signed_kg(row.fuel_remaining_delta_kg)
    )
}
