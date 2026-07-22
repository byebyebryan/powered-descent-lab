use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct SelectorScopeCounts {
    missions: usize,
    case_groups: usize,
    lanes: usize,
}

pub(super) struct LaneRecordFocus<'a> {
    lane_id: &'static str,
    pub(super) records: Vec<&'a crate::BatchRunRecord>,
}

pub(super) struct LaneFocusSummary {
    lane_id: &'static str,
    run_count: usize,
    controller_html: String,
    scope: SelectorScopeCounts,
    review: ReviewAggregate,
    mean_sim_time_s: f64,
    max_sim_time_s: f64,
}

pub(super) fn selector_scope_counts(candidate: &BatchReport) -> SelectorScopeCounts {
    let records = candidate.records.iter().collect::<Vec<_>>();
    selector_scope_counts_from_records(records.as_slice())
}

pub(super) fn selector_scope_counts_from_records(
    records: &[&crate::BatchRunRecord],
) -> SelectorScopeCounts {
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

pub(super) fn preferred_current_lane_focus<'a>(
    report: &'a BatchReport,
) -> Option<LaneRecordFocus<'a>> {
    let records = report.records.iter().collect::<Vec<_>>();
    let lane_id = preferred_current_lane_id(records.as_slice())?;
    let records = controller_lane_records(records.as_slice(), lane_id);
    (!records.is_empty()).then_some(LaneRecordFocus { lane_id, records })
}

pub(super) fn summarize_lane_focus(focus: &LaneRecordFocus<'_>) -> LaneFocusSummary {
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

pub(super) fn compare_basis_from_records(
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

pub(super) fn compare_scope_resolution(
    basis: &crate::BatchCompareBasis,
) -> (&'static str, &'static str) {
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

pub(super) fn short_digest(value: &str) -> String {
    value.chars().take(8).collect()
}

pub(super) struct OverviewRow {
    row_class: &'static str,
    pack_html: String,
    ref_html: String,
    scope_html: String,
    result_html: String,
    timing_html: String,
    efficiency_html: String,
    tracking_html: String,
}

pub(super) fn render_overview_row(row: OverviewRow) -> String {
    let OverviewRow {
        row_class,
        pack_html,
        ref_html,
        scope_html,
        result_html,
        timing_html,
        efficiency_html,
        tracking_html,
    } = row;
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

pub(super) fn render_overview_result_cell(
    success_runs: usize,
    total_runs: usize,
    failure_runs: usize,
    invalidated_runs: usize,
    success_delta: Option<f64>,
) -> String {
    let scored_runs = total_runs.saturating_sub(invalidated_runs);
    let main = if scored_runs == 0 {
        "n/a".to_owned()
    } else {
        format!("{:.1}%", percentage(success_runs, scored_runs))
    };
    let invalidated_html = if invalidated_runs > 0 {
        format!(r#" · <span class="warn">{invalidated_runs} warning</span>"#)
    } else {
        String::new()
    };
    let sub = match success_delta {
        Some(delta) => {
            let compare_html = format!(
                r#"<span class="{}">{}</span> · {} fail{}"#,
                delta_class(-delta),
                escape_html(&format_percent_delta(delta)),
                failure_runs,
                invalidated_html
            );
            let standalone_html = format!(
                "{} success · {} fail{}",
                success_runs, failure_runs, invalidated_html
            );
            format!(
                r#"<span class="compare-toggle-target">{compare_html}</span><span class="standalone-toggle-target">{standalone_html}</span>"#
            )
        }
        None => {
            format!("{success_runs}/{scored_runs} success · {failure_runs} fail{invalidated_html}")
        }
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&main),
        sub
    )
}

pub(super) fn render_overview_scope_cell(
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

pub(super) fn render_overview_timing_cell(
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

pub(super) fn render_overview_efficiency_cell(
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

pub(super) fn render_overview_tracking_cell(
    review: &ReviewAggregate,
    baseline: Option<&ReviewAggregate>,
    show_delta: bool,
) -> String {
    let reference_label = aggregate_ref_dev_label(review);
    let reference = aggregate_ref_dev_metric(review)
        .map(|summary| format_metric_value(summary, MetricDisplayKind::Meters))
        .unwrap_or_else(|| "-".to_owned());
    let low_unsafe = review
        .low_altitude_unsafe_recovery_s
        .as_ref()
        .map(|summary| format_metric_value(summary, MetricDisplayKind::Seconds))
        .unwrap_or_else(|| "-".to_owned());
    let sub_html = if show_delta {
        let reference_delta = metric_delta_value(
            aggregate_ref_dev_metric(review),
            baseline.and_then(aggregate_ref_dev_metric),
        )
        .map(|delta| format_metric_delta_value(delta, MetricDisplayKind::Meters))
        .unwrap_or_else(|| "-".to_owned());
        let low_unsafe_delta = metric_delta_value(
            review.low_altitude_unsafe_recovery_s.as_ref(),
            baseline.and_then(|item| item.low_altitude_unsafe_recovery_s.as_ref()),
        )
        .map(|delta| format_metric_delta_value(delta, MetricDisplayKind::Seconds))
        .unwrap_or_else(|| "-".to_owned());
        format!(
            r#"<span class="compare-toggle-target">low unsafe {low_unsafe} · Δ {reference_label} {reference_delta} · Δ low {low_unsafe_delta}</span><span class="standalone-toggle-target">low unsafe {low_unsafe}</span>"#,
            low_unsafe = escape_html(&low_unsafe),
            reference_label = escape_html(reference_label),
            reference_delta = escape_html(&reference_delta),
            low_unsafe_delta = escape_html(&low_unsafe_delta),
        )
    } else {
        escape_html(&format!("{reference_label} dev · low unsafe {low_unsafe}"))
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&reference),
        sub_html
    )
}

pub(super) fn render_overview_efficiency_diff_cell(
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

pub(super) fn render_overview_tracking_diff_cell(
    candidate: &ReviewAggregate,
    baseline: Option<&ReviewAggregate>,
) -> String {
    let Some(baseline) = baseline else {
        return r#"<div class="overview-stack"><div class="overview-main">-</div><div class="overview-sub">-</div></div>"#.to_owned();
    };
    let delta = metric_delta_value(
        aggregate_ref_dev_metric(candidate),
        aggregate_ref_dev_metric(baseline),
    )
    .map(|delta| format_metric_delta_value(delta, MetricDisplayKind::Meters))
    .unwrap_or_else(|| "-".to_owned());
    let low_unsafe_delta = metric_delta_value(
        candidate.low_altitude_unsafe_recovery_s.as_ref(),
        baseline.low_altitude_unsafe_recovery_s.as_ref(),
    )
    .map(|delta| format_metric_delta_value(delta, MetricDisplayKind::Seconds))
    .unwrap_or_else(|| "-".to_owned());
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">low unsafe {}</div></div>"#,
        escape_html(&delta),
        escape_html(&low_unsafe_delta)
    )
}

pub(super) fn overview_timing_from_records(records: &[&crate::BatchRunRecord]) -> (f64, f64) {
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

pub(super) fn render_wall_clock_chip(candidate: &BatchReport) -> String {
    format!(
        r#"<span class="chip"><strong>wall</strong> {}</span>"#,
        escape_html(&format!("{:.2}s", candidate.wall_clock_s)),
    )
}

pub(super) fn success_rate_ratio(success_runs: usize, total_runs: usize) -> f64 {
    if total_runs == 0 {
        0.0
    } else {
        success_runs as f64 / total_runs as f64
    }
}

pub(super) fn batch_report_subtitle(
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

pub(super) fn render_context_table(
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
    let needs_attention = context_requires_attention(candidate, comparison);

    format!(
        r#"<details class="header-context{attention_class}"{open_attr}>
  <summary><h2>Context</h2><span class="status-chip {compare_status_class}">{compare_status_label}</span></summary>
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
</details>"#,
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
        attention_class = if needs_attention { " attention" } else { "" },
        open_attr = if needs_attention { " open" } else { "" },
        compare_status_class = compare_status_class,
        compare_status_label = escape_html(compare_status_label),
    )
}

pub(super) fn context_requires_attention(
    candidate: &BatchReport,
    comparison: Option<&BatchComparison>,
) -> bool {
    if let Some(comparison) = comparison {
        return comparison.policy.status == BatchRegressionPolicyStatus::Fail
            || comparison.basis.shared_runs == 0
            || comparison.basis.candidate_only_runs > 0
            || comparison.basis.baseline_only_runs > 0;
    }
    let provenance = &candidate.provenance.compare;
    provenance.status == BatchCompareResolutionStatus::Missing
        && (provenance.source == crate::BatchCompareSource::ExplicitDir
            || provenance
                .requested_ref
                .as_deref()
                .is_some_and(|requested| requested != "auto"))
}

pub(super) fn render_cache_context(
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

pub(super) fn render_cache_status_label(status: BatchCacheStatus, promoted: bool) -> &'static str {
    match (status, promoted) {
        (BatchCacheStatus::Fresh, false) => "fresh",
        (BatchCacheStatus::Fresh, true) => "fresh promoted-cache",
        (BatchCacheStatus::Reused, false) => "reused",
        (BatchCacheStatus::Reused, true) => "reused promoted-cache",
        (BatchCacheStatus::Promoted, _) => "promoted",
    }
}

pub(super) fn baseline_resolution_summary(
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

pub(super) fn missing_baseline_context_value(provenance: &crate::BatchCompareProvenance) -> String {
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

pub(super) fn context_value(main: &str, sub: &str) -> String {
    format!(
        r#"<div class="context-value"><div class="context-main">{}</div><div class="context-sub">{}</div></div>"#,
        escape_html(main),
        escape_html(sub),
    )
}

pub(super) fn context_value_html(main_html: &str, sub_html: &str) -> String {
    format!(
        r#"<div class="context-value"><div class="context-main">{}</div><div class="context-sub">{}</div></div>"#,
        main_html, sub_html,
    )
}

pub(super) fn context_none_value(label: &str) -> String {
    format!(
        r#"<div class="context-value"><div class="context-main">{}</div><div class="context-sub">not applicable for this batch page</div></div>"#,
        escape_html(label),
    )
}

pub(super) fn render_overview_table(
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
            candidate_summary
                .review
                .total_runs
                .saturating_sub(candidate_summary.review.invalidated_runs),
        ) - success_rate_ratio(
            baseline_summary.review.success_runs,
            baseline_summary
                .review
                .total_runs
                .saturating_sub(baseline_summary.review.invalidated_runs),
        );

        vec![
            render_overview_row(OverviewRow {
                row_class: "current-summary-row",
                pack_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag current">current</span><code>{}</code></div><div class="overview-sub">{} · {}</div></div>"#,
                    escape_html(&candidate.pack_id),
                    escape_html(&candidate.pack_name),
                    candidate_summary.controller_html,
                ),
                ref_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code> · lane <code>{}</code></div></div>"#,
                    escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                    escape_html(&short_digest(&candidate.identity.resolved_run_digest)),
                    escape_html(candidate_summary.lane_id),
                ),
                scope_html: render_overview_scope_cell(
                    &candidate_summary.scope,
                    candidate.workers_used,
                    Some(&basis),
                ),
                result_html: render_overview_result_cell(
                    candidate_summary.review.success_runs,
                    candidate_summary.review.total_runs,
                    candidate_summary.review.failure_runs,
                    candidate_summary.review.invalidated_runs,
                    Some(success_rate_delta),
                ),
                timing_html: render_overview_timing_cell(
                    candidate_summary.mean_sim_time_s,
                    candidate_summary.max_sim_time_s,
                    Some((
                        candidate_summary.mean_sim_time_s - baseline_summary.mean_sim_time_s,
                        candidate_summary.max_sim_time_s - baseline_summary.max_sim_time_s,
                    )),
                ),
                efficiency_html: render_overview_efficiency_cell(
                    &candidate_summary.review,
                    Some(&baseline_summary.review),
                    true,
                ),
                tracking_html: render_overview_tracking_cell(
                    &candidate_summary.review,
                    Some(&baseline_summary.review),
                    true,
                ),
            }),
            render_overview_row(OverviewRow {
                row_class: "baseline-summary-row baseline-row",
                pack_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag baseline">baseline</span><code>{}</code></div><div class="overview-sub">{} · {}</div></div>"#,
                    escape_html(&baseline_report.pack_id),
                    escape_html(&baseline_report.pack_name),
                    baseline_summary.controller_html,
                ),
                ref_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code> · lane <code>{}</code></div></div>"#,
                    escape_html(&short_digest(&baseline_report.identity.pack_spec_digest)),
                    escape_html(&short_digest(&baseline_report.identity.resolved_run_digest)),
                    escape_html(baseline_summary.lane_id),
                ),
                scope_html: render_overview_scope_cell(
                    &baseline_summary.scope,
                    baseline_report.workers_used,
                    None,
                ),
                result_html: render_overview_result_cell(
                    baseline_summary.review.success_runs,
                    baseline_summary.review.total_runs,
                    baseline_summary.review.failure_runs,
                    baseline_summary.review.invalidated_runs,
                    None,
                ),
                timing_html: render_overview_timing_cell(
                    baseline_summary.mean_sim_time_s,
                    baseline_summary.max_sim_time_s,
                    None,
                ),
                efficiency_html: render_overview_efficiency_cell(
                    &baseline_summary.review,
                    None,
                    false,
                ),
                tracking_html: render_overview_tracking_cell(&baseline_summary.review, None, false),
            }),
            render_overview_row(OverviewRow {
                row_class: "diff-summary-row",
                pack_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag diff">diff</span>current-lane history compare</div><div class="overview-sub">shared {} · current-only {} · baseline-only {} · {}</div></div>"#,
                    basis.shared_runs,
                    basis.candidate_only_runs,
                    basis.baseline_only_runs,
                    render_policy_status_chip(comparison.policy.status)
                ),
                ref_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main"><code>{}</code></div><div class="overview-sub"><code>{}</code> -> <code>{}</code></div></div>"#,
                    escape_html(&baseline_report.pack_id),
                    escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                    escape_html(&short_digest(&baseline_report.identity.pack_spec_digest))
                ),
                scope_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">current results lane <code>{}</code> -> compare baseline lane <code>{}</code></div></div>"#,
                    escape_html(&format!("shared {}", basis.shared_runs)),
                    escape_html(candidate_summary.lane_id),
                    escape_html(baseline_summary.lane_id),
                ),
                result_html: format!(
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
                timing_html: format!(
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
                efficiency_html: render_overview_efficiency_diff_cell(
                    &candidate_summary.review,
                    Some(&baseline_summary.review),
                ),
                tracking_html: render_overview_tracking_diff_cell(
                    &candidate_summary.review,
                    Some(&baseline_summary.review),
                ),
            }),
        ]
    } else if let Some(candidate_focus) = preferred_current_lane_focus(candidate) {
        let candidate_summary = summarize_lane_focus(&candidate_focus);
        vec![render_overview_row(OverviewRow {
            row_class: "current-summary-row",
            pack_html: format!(
                r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag current">current</span><code>{}</code></div><div class="overview-sub">{} · {}</div></div>"#,
                escape_html(&candidate.pack_id),
                escape_html(&candidate.pack_name),
                candidate_summary.controller_html
            ),
            ref_html: format!(
                r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code> · lane <code>{}</code></div></div>"#,
                escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                escape_html(&short_digest(&candidate.identity.resolved_run_digest)),
                escape_html(candidate_summary.lane_id)
            ),
            scope_html: render_overview_scope_cell(
                &candidate_summary.scope,
                candidate.workers_used,
                None,
            ),
            result_html: render_overview_result_cell(
                candidate_summary.review.success_runs,
                candidate_summary.review.total_runs,
                candidate_summary.review.failure_runs,
                candidate_summary.review.invalidated_runs,
                None,
            ),
            timing_html: render_overview_timing_cell(
                candidate_summary.mean_sim_time_s,
                candidate_summary.max_sim_time_s,
                None,
            ),
            efficiency_html: render_overview_efficiency_cell(
                &candidate_summary.review,
                None,
                false,
            ),
            tracking_html: render_overview_tracking_cell(&candidate_summary.review, None, false),
        })]
    } else {
        let mut rows = vec![render_overview_row(OverviewRow {
            row_class: "current-summary-row",
            pack_html: format!(
                r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag current">current</span><code>{}</code></div><div class="overview-sub">{}</div></div>"#,
                escape_html(&candidate.pack_id),
                escape_html(&candidate.pack_name)
            ),
            ref_html: format!(
                r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code></div></div>"#,
                escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                escape_html(&short_digest(&candidate.identity.resolved_run_digest))
            ),
            scope_html: render_overview_scope_cell(
                &candidate_scope,
                candidate.workers_used,
                comparison.map(|cmp| &cmp.basis),
            ),
            result_html: render_overview_result_cell(
                candidate.summary.success_runs,
                candidate.summary.total_runs,
                candidate.summary.failure_runs,
                candidate.summary.invalidated_runs,
                comparison.map(|cmp| cmp.summary.success_rate_delta),
            ),
            timing_html: render_overview_timing_cell(
                candidate.summary.mean_sim_time_s,
                candidate.summary.max_sim_time_s,
                comparison.map(|cmp| {
                    (
                        cmp.summary.mean_sim_time_delta_s,
                        cmp.summary.max_sim_time_delta_s,
                    )
                }),
            ),
            efficiency_html: render_overview_efficiency_cell(
                &candidate_review,
                baseline_review.as_ref(),
                comparison.is_some(),
            ),
            tracking_html: render_overview_tracking_cell(
                &candidate_review,
                baseline_review.as_ref(),
                comparison.is_some(),
            ),
        })];

        if let Some(baseline) = baseline {
            let baseline_scope = baseline_scope.expect("baseline scope");
            let baseline_review = baseline_review.as_ref().expect("baseline review");
            rows.push(render_overview_row(OverviewRow {
                row_class: "baseline-summary-row baseline-row",
                pack_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag baseline">baseline</span><code>{}</code></div><div class="overview-sub">{}</div></div>"#,
                    escape_html(&baseline.pack_id),
                    escape_html(&baseline.pack_name)
                ),
                ref_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main">spec <code>{}</code></div><div class="overview-sub">resolved <code>{}</code></div></div>"#,
                    escape_html(&short_digest(&baseline.identity.pack_spec_digest)),
                    escape_html(&short_digest(&baseline.identity.resolved_run_digest))
                ),
                scope_html: render_overview_scope_cell(
                    &baseline_scope,
                    baseline.workers_used,
                    None,
                ),
                result_html: render_overview_result_cell(
                    baseline.summary.success_runs,
                    baseline.summary.total_runs,
                    baseline.summary.failure_runs,
                    baseline.summary.invalidated_runs,
                    None,
                ),
                timing_html: render_overview_timing_cell(
                    baseline.summary.mean_sim_time_s,
                    baseline.summary.max_sim_time_s,
                    None,
                ),
                efficiency_html: render_overview_efficiency_cell(baseline_review, None, false),
                tracking_html: render_overview_tracking_cell(baseline_review, None, false),
            }));
        }

        if let Some(comparison) = comparison {
            rows.push(render_overview_row(OverviewRow {
                row_class: "diff-summary-row",
                pack_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main"><span class="row-tag diff">diff</span>{}</div><div class="overview-sub">shared {} · current-only {} · baseline-only {} · {}</div></div>"#,
                    escape_html(&comparison.basis.mode),
                    comparison.basis.shared_runs,
                    comparison.basis.candidate_only_runs,
                    comparison.basis.baseline_only_runs,
                    render_policy_status_chip(comparison.policy.status)
                ),
                ref_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main"><code>{}</code></div><div class="overview-sub"><code>{}</code> -> <code>{}</code></div></div>"#,
                    escape_html(&comparison.baseline_pack_id),
                    escape_html(&short_digest(&candidate.identity.pack_spec_digest)),
                    escape_html(
                        &baseline
                            .map(|report| short_digest(&report.identity.pack_spec_digest))
                            .unwrap_or_else(|| "-".to_owned())
                    )
                ),
                scope_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">candidate-only {} · baseline-only {}</div></div>"#,
                    escape_html(&format!("shared {}", comparison.basis.shared_runs)),
                    comparison.basis.candidate_only_runs,
                    comparison.basis.baseline_only_runs
                ),
                result_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main {}">{}</div><div class="overview-sub {}">{}</div></div>"#,
                    delta_class(-comparison.summary.success_rate_delta),
                    escape_html(&format_percent_delta(comparison.summary.success_rate_delta)),
                    delta_class(comparison.summary.failure_runs_delta as f64),
                    escape_html(&format_signed_i64(comparison.summary.failure_runs_delta))
                ),
                timing_html: format!(
                    r#"<div class="overview-stack"><div class="overview-main {}">{}</div><div class="overview-sub">{}</div></div>"#,
                    delta_class(comparison.summary.mean_sim_time_delta_s),
                    escape_html(&format_signed_seconds(comparison.summary.mean_sim_time_delta_s)),
                    escape_html(&format!(
                        "max {}",
                        format_signed_seconds(comparison.summary.max_sim_time_delta_s)
                    ))
                ),
                efficiency_html: render_overview_efficiency_diff_cell(
                    &candidate_review,
                    baseline_review.as_ref(),
                ),
                tracking_html: render_overview_tracking_diff_cell(
                    &candidate_review,
                    baseline_review.as_ref(),
                ),
            }));
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
          <th>Reference</th>
          <th>Scope</th>
          <th>Result</th>
          <th>Timing</th>
          <th>Efficiency</th>
          <th>Reference / recovery</th>
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

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct CoveragePaneKey {
    mission: String,
    condition: String,
    vehicle: String,
    profile: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct CoverageCellKey {
    pane: CoveragePaneKey,
    row: String,
    column: String,
}

#[derive(Clone, Copy, Default)]
pub(super) struct CoverageCellSummary {
    scored_success: usize,
    scored_failure: usize,
    invalidated: usize,
    frontier: usize,
}

impl CoverageCellSummary {
    fn scored_runs(self) -> usize {
        self.scored_success + self.scored_failure
    }

    fn success_rate(self) -> Option<f64> {
        (self.scored_runs() > 0).then(|| self.scored_success as f64 / self.scored_runs() as f64)
    }
}

#[derive(Clone, Copy)]
pub(super) enum CoverageMode {
    Terminal,
    Transfer,
}

pub(super) fn render_coverage_matrix(
    candidate: &BatchReport,
    baseline: Option<&BatchReport>,
    comparison: Option<&BatchComparison>,
) -> String {
    let Some(candidate_focus) = preferred_current_lane_focus(candidate) else {
        return String::new();
    };
    let candidate_records = candidate_focus.records;
    let mode = if candidate_records
        .iter()
        .any(|record| meaningful_selector_value(&record.resolved.selector.route_angle).is_some())
    {
        CoverageMode::Transfer
    } else {
        CoverageMode::Terminal
    };
    let candidate_cells = coverage_cells(candidate_records.as_slice(), mode);
    if candidate_cells.is_empty() {
        return String::new();
    }
    let baseline_cells = if comparison.is_some() {
        baseline
            .and_then(preferred_current_lane_focus)
            .map(|focus| coverage_cells(focus.records.as_slice(), mode))
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    let panes = candidate_cells
        .keys()
        .map(|key| key.pane.clone())
        .collect::<BTreeSet<_>>();
    let mut rows = candidate_cells
        .keys()
        .map(|key| key.row.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut columns = candidate_cells
        .keys()
        .map(|key| key.column.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    sort_selector_keys(&mut rows);
    sort_selector_keys(&mut columns);

    let controls = render_coverage_filters(&panes);
    let pane_html = panes
        .iter()
        .map(|pane| {
            render_coverage_pane(
                pane,
                rows.as_slice(),
                columns.as_slice(),
                &candidate_cells,
                &baseline_cells,
                comparison.is_some(),
                mode,
            )
        })
        .collect::<String>();
    let axis_note = match mode {
        CoverageMode::Terminal => "Energy band by arrival arc",
        CoverageMode::Transfer => "Travel radius by route angle",
    };
    format!(
        r#"<section class="coverage-section"><div class="section-head"><div><h2>Coverage</h2><span class="section-note">{axis_note}; click a cell to inspect its review-tree branch.</span></div>{controls}</div>{pane_html}</section>"#,
        axis_note = axis_note,
        controls = controls,
        pane_html = pane_html,
    )
}

pub(super) fn coverage_cells(
    records: &[&crate::BatchRunRecord],
    mode: CoverageMode,
) -> BTreeMap<CoverageCellKey, CoverageCellSummary> {
    let mut cells = BTreeMap::<CoverageCellKey, CoverageCellSummary>::new();
    for record in records {
        let selector = &record.resolved.selector;
        let (row, column) = match mode {
            CoverageMode::Terminal => (&selector.velocity_band, &selector.arc_point),
            CoverageMode::Transfer => (&selector.radius_tier, &selector.route_angle),
        };
        let (Some(row), Some(column)) = (
            meaningful_selector_value(row),
            meaningful_selector_value(column),
        ) else {
            continue;
        };
        let key = CoverageCellKey {
            pane: CoveragePaneKey {
                mission: coverage_filter_value(&selector.mission),
                condition: coverage_filter_value(&selector.condition_set),
                vehicle: coverage_filter_value(&selector.vehicle_variant),
                profile: coverage_filter_value(&selector.waypoint_profile),
            },
            row: row.to_owned(),
            column: column.to_owned(),
        };
        let cell = cells.entry(key).or_default();
        match record.analytic.class {
            crate::BatchRunAnalyticClass::Impossible => cell.invalidated += 1,
            crate::BatchRunAnalyticClass::Frontier => {
                cell.frontier += 1;
                if matches!(
                    record.manifest.mission_outcome,
                    pd_core::MissionOutcome::Success
                ) {
                    cell.scored_success += 1;
                } else {
                    cell.scored_failure += 1;
                }
            }
            crate::BatchRunAnalyticClass::Scored => {
                if matches!(
                    record.manifest.mission_outcome,
                    pd_core::MissionOutcome::Success
                ) {
                    cell.scored_success += 1;
                } else {
                    cell.scored_failure += 1;
                }
            }
        }
    }
    cells
}

pub(super) fn meaningful_selector_value(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty() && value != UNSPECIFIED_SELECTOR_VALUE).then_some(value)
}

pub(super) fn coverage_filter_value(value: &str) -> String {
    meaningful_selector_value(value).unwrap_or("all").to_owned()
}

pub(super) fn render_coverage_filters(panes: &BTreeSet<CoveragePaneKey>) -> String {
    let dimensions = [
        (
            "mission",
            "Mission",
            panes
                .iter()
                .map(|pane| pane.mission.clone())
                .collect::<Vec<_>>(),
        ),
        (
            "condition",
            "Condition",
            panes
                .iter()
                .map(|pane| pane.condition.clone())
                .collect::<Vec<_>>(),
        ),
        (
            "vehicle",
            "Payload",
            panes
                .iter()
                .map(|pane| pane.vehicle.clone())
                .collect::<Vec<_>>(),
        ),
        (
            "profile",
            "Waypoint profile",
            panes
                .iter()
                .map(|pane| pane.profile.clone())
                .collect::<Vec<_>>(),
        ),
    ];
    let controls = dimensions
        .into_iter()
        .filter_map(|(id, label, values)| {
            let mut values = values
                .into_iter()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            sort_selector_keys(&mut values);
            (values.len() > 1).then(|| {
                let options = values
                    .iter()
                    .map(|value| {
                        format!(
                            r#"<option value="{}">{}</option>"#,
                            escape_html(value),
                            escape_html(&selector_display_label(value))
                        )
                    })
                    .collect::<String>();
                format!(
                    r#"<label>{label}<select data-coverage-filter="{id}">{options}</select></label>"#
                )
            })
        })
        .collect::<String>();
    if controls.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="coverage-filters">{controls}</div>"#)
    }
}

pub(super) fn render_coverage_pane(
    pane: &CoveragePaneKey,
    rows: &[String],
    columns: &[String],
    candidate_cells: &BTreeMap<CoverageCellKey, CoverageCellSummary>,
    baseline_cells: &BTreeMap<CoverageCellKey, CoverageCellSummary>,
    show_compare: bool,
    mode: CoverageMode,
) -> String {
    let header = columns
        .iter()
        .map(|column| format!("<th>{}</th>", escape_html(&selector_display_label(column))))
        .collect::<String>();
    let body = rows
        .iter()
        .map(|row| {
            let cells = columns
                .iter()
                .map(|column| {
                    let key = CoverageCellKey {
                        pane: pane.clone(),
                        row: row.clone(),
                        column: column.clone(),
                    };
                    render_coverage_cell(
                        candidate_cells.get(&key).copied(),
                        baseline_cells.get(&key).copied(),
                        &key,
                        show_compare,
                    )
                })
                .collect::<String>();
            format!(
                "<tr><th>{}</th>{cells}</tr>",
                escape_html(&selector_display_label(row))
            )
        })
        .collect::<String>();
    let corner = match mode {
        CoverageMode::Terminal => "Band / arc",
        CoverageMode::Transfer => "Radius / route",
    };
    format!(
        r#"<div class="coverage-pane table-wrap" data-coverage-pane data-mission="{mission}" data-condition="{condition}" data-vehicle="{vehicle}" data-profile="{profile}"><table class="coverage-table"><thead><tr><th>{corner}</th>{header}</tr></thead><tbody>{body}</tbody></table></div>"#,
        mission = escape_html(&pane.mission),
        condition = escape_html(&pane.condition),
        vehicle = escape_html(&pane.vehicle),
        profile = escape_html(&pane.profile),
        corner = corner,
        header = header,
        body = body,
    )
}

pub(super) fn render_coverage_cell(
    current: Option<CoverageCellSummary>,
    baseline: Option<CoverageCellSummary>,
    key: &CoverageCellKey,
    show_compare: bool,
) -> String {
    let Some(current) = current else {
        return r#"<td><div class="coverage-cell invalid-only"><strong>—</strong><span>not covered</span></div></td>"#.to_owned();
    };
    let class = if current.scored_failure > 0 {
        " has-failure"
    } else if current.scored_runs() == 0 && current.invalidated > 0 {
        " invalid-only"
    } else {
        ""
    };
    let annotations = match (current.invalidated, current.frontier) {
        (0, 0) => String::new(),
        (invalidated, 0) => format!(" · {invalidated} invalid"),
        (0, frontier) => format!(" · {frontier} frontier"),
        (invalidated, frontier) => format!(" · {invalidated} invalid · {frontier} frontier"),
    };
    let delta = if show_compare {
        current
            .success_rate()
            .zip(baseline.and_then(CoverageCellSummary::success_rate))
            .map(|(current, baseline)| current - baseline)
            .filter(|delta| delta.abs() > 1e-9)
            .map(|delta| {
                format!(
                    r#"<span class="coverage-delta compare-toggle-target{}"> · {:+.1} pp</span>"#,
                    if delta > 0.0 { " improved" } else { "" },
                    delta * 100.0,
                )
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    let tokens = [
        key.pane.mission.as_str(),
        key.pane.condition.as_str(),
        key.pane.vehicle.as_str(),
        key.pane.profile.as_str(),
        key.column.as_str(),
        key.row.as_str(),
    ]
    .into_iter()
    .filter(|value| *value != "all")
    .collect::<Vec<_>>()
    .join("|");
    format!(
        r#"<td><div class="coverage-cell{class}" data-tree-tokens="{tokens}" tabindex="0"><strong>{success}/{scored} success</strong><span>{failure} fail{annotations}{delta}</span></div></td>"#,
        class = class,
        tokens = escape_html(&tokens),
        success = current.scored_success,
        scored = current.scored_runs(),
        failure = current.scored_failure,
        annotations = escape_html(&annotations),
        delta = delta,
    )
}

pub(super) fn selector_display_label(value: &str) -> String {
    if value == "all" {
        return "All".to_owned();
    }
    value.strip_suffix("_v1").unwrap_or(value).replace('_', " ")
}

pub(super) fn render_guidance_diagnostics(
    waypoint_sequence: String,
    waypoint_triage: String,
    transfer_handoff: String,
    transfer_shape: String,
) -> String {
    let contents = [
        waypoint_sequence,
        waypoint_triage,
        transfer_handoff,
        transfer_shape,
    ]
    .into_iter()
    .filter(|section| !section.trim().is_empty())
    .collect::<String>();
    if contents.is_empty() {
        return String::new();
    }
    format!(
        r#"<details class="guidance-diagnostics"><summary><h2>Guidance Diagnostics</h2></summary>{contents}</details>"#
    )
}

pub(super) fn render_tree_controls(has_compare: bool) -> String {
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

pub(super) fn render_view_controls(has_compare_view: bool) -> String {
    if !has_compare_view {
        return String::new();
    }
    r#"<div class="view-mode-controls">
  <button type="button" class="active" data-view-mode="compare">Compare View</button>
  <button type="button" data-view-mode="current-only">Current Only</button>
</div>"#
        .to_owned()
}
