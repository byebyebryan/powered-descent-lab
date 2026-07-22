use super::*;

pub fn compare_batch_reports(candidate: &BatchReport, baseline: &BatchReport) -> BatchComparison {
    let (candidate_scope_records, baseline_scope_records) =
        comparison_record_scope(candidate, baseline);
    let candidate_records = candidate_scope_records
        .iter()
        .map(|record| (record.resolved.run_id.clone(), *record))
        .collect::<BTreeMap<_, _>>();
    let baseline_records = baseline_scope_records
        .iter()
        .map(|record| (record.resolved.run_id.clone(), *record))
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
    for run_id in &shared_run_ids {
        let candidate_record = candidate_records
            .get(run_id)
            .expect("shared run ids should exist in candidate map");
        let baseline_record = baseline_records
            .get(run_id)
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

    let basis = BatchCompareBasis {
        mode: "run_id".to_owned(),
        shared_runs: shared_run_ids.len(),
        candidate_only_runs: candidate_only.len(),
        baseline_only_runs: baseline_only.len(),
    };
    let candidate_summary = summarize_record_refs(candidate_scope_records.as_slice());
    let baseline_summary = summarize_record_refs(baseline_scope_records.as_slice());
    let summary = compare_summary_delta(&candidate_summary, &baseline_summary);
    let policy = evaluate_regression_policy(&summary, &basis, &regressions);

    BatchComparison {
        candidate_pack_id: candidate.pack_id.clone(),
        candidate_pack_name: candidate.pack_name.clone(),
        baseline_pack_id: baseline.pack_id.clone(),
        baseline_pack_name: baseline.pack_name.clone(),
        basis,
        summary,
        policy,
        by_entry: compare_group_sets(&candidate_summary.by_entry, &baseline_summary.by_entry),
        by_family: compare_group_sets(&candidate_summary.by_family, &baseline_summary.by_family),
        regressions,
        improvements,
        outcome_changes,
        candidate_only,
        baseline_only,
    }
}

fn comparison_record_scope<'a, 'b>(
    candidate: &'a BatchReport,
    baseline: &'b BatchReport,
) -> (Vec<&'a BatchRunRecord>, Vec<&'b BatchRunRecord>) {
    let candidate_lane_id = preferred_compare_lane_id(&candidate.records);
    let baseline_lane_id = preferred_compare_lane_id(&baseline.records);
    if let (Some(candidate_lane_id), Some(baseline_lane_id)) = (candidate_lane_id, baseline_lane_id)
    {
        (
            records_for_lane(&candidate.records, candidate_lane_id),
            records_for_lane(&baseline.records, baseline_lane_id),
        )
    } else {
        (
            candidate.records.iter().collect(),
            baseline.records.iter().collect(),
        )
    }
}

fn preferred_compare_lane_id(records: &[BatchRunRecord]) -> Option<&'static str> {
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

fn records_for_lane<'a>(records: &'a [BatchRunRecord], lane_id: &str) -> Vec<&'a BatchRunRecord> {
    records
        .iter()
        .filter(|record| record.resolved.lane_id == lane_id)
        .collect()
}

fn summarize_record_refs(records: &[&BatchRunRecord]) -> BatchSummary {
    let owned_records = records
        .iter()
        .map(|record| (*record).clone())
        .collect::<Vec<_>>();
    summarize_records(&owned_records)
}

fn evaluate_regression_policy(
    summary: &BatchSummaryDelta,
    basis: &BatchCompareBasis,
    regressions: &[BatchRunComparison],
) -> BatchRegressionPolicyEvaluation {
    let mut rules = Vec::new();
    let has_exact_coverage = basis.candidate_only_runs == 0 && basis.baseline_only_runs == 0;

    rules.push(regression_policy_rule(
        "new_failures",
        "New shared-run failures",
        if regressions.is_empty() {
            BatchRegressionPolicyStatus::Pass
        } else {
            BatchRegressionPolicyStatus::Fail
        },
        regressions.len().to_string(),
        "0",
        "shared runs must not move from success to failure",
    ));
    rules.push(regression_policy_rule(
        "scored_failure_delta",
        "Scored failure delta",
        if summary.failure_runs_delta > 0 {
            if has_exact_coverage {
                BatchRegressionPolicyStatus::Fail
            } else {
                BatchRegressionPolicyStatus::Warn
            }
        } else {
            BatchRegressionPolicyStatus::Pass
        },
        format!("{:+}", summary.failure_runs_delta),
        "<= 0 with exact coverage",
        "total scored failures must not increase when run sets match",
    ));
    rules.push(regression_policy_rule(
        "success_rate_delta",
        "Scored success-rate delta",
        if summary.success_rate_delta < -REGRESSION_POLICY_EPSILON {
            if has_exact_coverage {
                BatchRegressionPolicyStatus::Fail
            } else {
                BatchRegressionPolicyStatus::Warn
            }
        } else {
            BatchRegressionPolicyStatus::Pass
        },
        format_policy_percent_delta(summary.success_rate_delta),
        ">= 0.00 pp with exact coverage",
        "scored success rate must not drop when run sets match",
    ));
    rules.push(regression_policy_rule(
        "mean_sim_time_delta",
        "Mean sim-time delta",
        if summary.mean_sim_time_delta_s > REGRESSION_POLICY_MEAN_SIM_TIME_WARN_DELTA_S {
            BatchRegressionPolicyStatus::Warn
        } else {
            BatchRegressionPolicyStatus::Pass
        },
        format_policy_seconds_delta(summary.mean_sim_time_delta_s),
        format!(
            "<= {}",
            format_policy_seconds_delta(REGRESSION_POLICY_MEAN_SIM_TIME_WARN_DELTA_S)
        ),
        "mean simulated time should not drift upward materially",
    ));
    rules.push(regression_policy_rule(
        "invalidated_delta",
        "Invalidated-run delta",
        if summary.invalidated_runs_delta > 0 {
            BatchRegressionPolicyStatus::Warn
        } else {
            BatchRegressionPolicyStatus::Pass
        },
        format!("{:+}", summary.invalidated_runs_delta),
        "<= 0",
        "new invalidations can hide scored regressions",
    ));
    rules.push(regression_policy_rule(
        "compare_coverage",
        "Compare coverage",
        if basis.candidate_only_runs > 0 || basis.baseline_only_runs > 0 {
            BatchRegressionPolicyStatus::Warn
        } else {
            BatchRegressionPolicyStatus::Pass
        },
        format!(
            "current-only {}, baseline-only {}",
            basis.candidate_only_runs, basis.baseline_only_runs
        ),
        "0 unmatched runs",
        "policy is strongest when the candidate and baseline run sets match",
    ));

    let status = regression_policy_status_from_rules(&rules);
    let summary_text = regression_policy_summary(status, &rules);

    BatchRegressionPolicyEvaluation {
        status,
        summary: summary_text,
        rules,
    }
}

fn regression_policy_rule(
    id: impl Into<String>,
    label: impl Into<String>,
    status: BatchRegressionPolicyStatus,
    observed: impl Into<String>,
    threshold: impl Into<String>,
    note: impl Into<String>,
) -> BatchRegressionPolicyRuleResult {
    BatchRegressionPolicyRuleResult {
        id: id.into(),
        label: label.into(),
        status,
        observed: observed.into(),
        threshold: threshold.into(),
        note: note.into(),
    }
}

fn regression_policy_status_from_rules(
    rules: &[BatchRegressionPolicyRuleResult],
) -> BatchRegressionPolicyStatus {
    if rules
        .iter()
        .any(|rule| rule.status == BatchRegressionPolicyStatus::Fail)
    {
        BatchRegressionPolicyStatus::Fail
    } else if rules
        .iter()
        .any(|rule| rule.status == BatchRegressionPolicyStatus::Warn)
    {
        BatchRegressionPolicyStatus::Warn
    } else {
        BatchRegressionPolicyStatus::Pass
    }
}

fn regression_policy_summary(
    status: BatchRegressionPolicyStatus,
    rules: &[BatchRegressionPolicyRuleResult],
) -> String {
    let failures = rules
        .iter()
        .filter(|rule| rule.status == BatchRegressionPolicyStatus::Fail)
        .count();
    let warnings = rules
        .iter()
        .filter(|rule| rule.status == BatchRegressionPolicyStatus::Warn)
        .count();
    match status {
        BatchRegressionPolicyStatus::Pass => "passed all regression thresholds".to_owned(),
        BatchRegressionPolicyStatus::Warn => {
            format!(
                "passed required thresholds with {}",
                plural_count(warnings, "warning", "warnings")
            )
        }
        BatchRegressionPolicyStatus::Fail => {
            if warnings > 0 {
                format!(
                    "failed {} with {}",
                    plural_count(failures, "required threshold", "required thresholds"),
                    plural_count(warnings, "warning", "warnings")
                )
            } else {
                format!(
                    "failed {}",
                    plural_count(failures, "required threshold", "required thresholds")
                )
            }
        }
    }
}

fn plural_count(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

fn format_policy_percent_delta(value: f64) -> String {
    format!("{:+.2} pp", value * 100.0)
}

fn format_policy_seconds_delta(value: f64) -> String {
    format!("{value:+.2}s")
}

fn record_scored(record: &BatchRunRecord) -> bool {
    record.analytic.is_scored()
}

fn record_invalidated(record: &BatchRunRecord) -> bool {
    !record_scored(record)
}

fn record_success(record: &BatchRunRecord) -> bool {
    record_scored(record) && matches!(record.manifest.mission_outcome, MissionOutcome::Success)
}

fn record_failure(record: &BatchRunRecord) -> bool {
    record_scored(record) && !matches!(record.manifest.mission_outcome, MissionOutcome::Success)
}

pub(crate) fn summarize_records(records: &[BatchRunRecord]) -> BatchSummary {
    let total_runs = records.len();
    let invalidated_runs = records
        .iter()
        .filter(|record| record_invalidated(record))
        .count();
    let success_runs = records
        .iter()
        .filter(|record| record_success(record))
        .count();
    let failure_runs = records
        .iter()
        .filter(|record| record_failure(record))
        .count();
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
        .filter(|record| record_failure(record))
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
        .filter(|record| record_failure(record))
        .map(run_pointer)
        .collect::<Vec<_>>();
    closest_failures.sort_by(closest_failure_sort_key);
    closest_failures.truncate(10);

    let mut worst_failures = records
        .iter()
        .filter(|record| record_failure(record))
        .map(run_pointer)
        .collect::<Vec<_>>();
    worst_failures.sort_by(worst_failure_sort_key);
    worst_failures.truncate(10);

    let mut weakest_successes = records
        .iter()
        .filter(|record| record_success(record))
        .map(run_pointer)
        .collect::<Vec<_>>();
    weakest_successes.sort_by(weakest_success_sort_key);
    weakest_successes.truncate(10);

    let mut lowest_fuel_successes = records
        .iter()
        .filter(|record| record_success(record))
        .map(run_pointer)
        .collect::<Vec<_>>();
    lowest_fuel_successes.sort_by(lowest_fuel_success_sort_key);
    lowest_fuel_successes.truncate(10);

    BatchSummary {
        total_runs,
        success_runs,
        failure_runs,
        invalidated_runs,
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
        candidate_success_rate: success_rate(
            candidate.success_runs,
            candidate
                .total_runs
                .saturating_sub(candidate.invalidated_runs),
        ),
        baseline_success_rate: success_rate(
            baseline.success_runs,
            baseline
                .total_runs
                .saturating_sub(baseline.invalidated_runs),
        ),
        success_rate_delta: success_rate(
            candidate.success_runs,
            candidate
                .total_runs
                .saturating_sub(candidate.invalidated_runs),
        ) - success_rate(
            baseline.success_runs,
            baseline
                .total_runs
                .saturating_sub(baseline.invalidated_runs),
        ),
        candidate_success_runs: candidate.success_runs,
        baseline_success_runs: baseline.success_runs,
        success_runs_delta: candidate.success_runs as i64 - baseline.success_runs as i64,
        candidate_failure_runs: candidate.failure_runs,
        baseline_failure_runs: baseline.failure_runs,
        failure_runs_delta: candidate.failure_runs as i64 - baseline.failure_runs as i64,
        candidate_invalidated_runs: candidate.invalidated_runs,
        baseline_invalidated_runs: baseline.invalidated_runs,
        invalidated_runs_delta: candidate.invalidated_runs as i64
            - baseline.invalidated_runs as i64,
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
                candidate_success_rate: candidate.map(|group| {
                    success_rate(
                        group.success_runs,
                        group.total_runs.saturating_sub(group.invalidated_runs),
                    )
                }),
                baseline_success_rate: baseline.map(|group| {
                    success_rate(
                        group.success_runs,
                        group.total_runs.saturating_sub(group.invalidated_runs),
                    )
                }),
                success_rate_delta: match (candidate, baseline) {
                    (Some(candidate), Some(baseline)) => Some(
                        success_rate(
                            candidate.success_runs,
                            candidate
                                .total_runs
                                .saturating_sub(candidate.invalidated_runs),
                        ) - success_rate(
                            baseline.success_runs,
                            baseline
                                .total_runs
                                .saturating_sub(baseline.invalidated_runs),
                        ),
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
                candidate_invalidated_runs: candidate.map(|group| group.invalidated_runs),
                baseline_invalidated_runs: baseline.map(|group| group.invalidated_runs),
                invalidated_runs_delta: match (candidate, baseline) {
                    (Some(candidate), Some(baseline)) => {
                        Some(candidate.invalidated_runs as i64 - baseline.invalidated_runs as i64)
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
    let invalidated_runs = records
        .iter()
        .filter(|record| record_invalidated(record))
        .count();
    let success_runs = records
        .iter()
        .filter(|record| record_success(record))
        .count();
    let failure_runs = records
        .iter()
        .filter(|record| record_failure(record))
        .count();
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
    let mut success_low_altitude_dwell_s = Vec::new();
    let mut success_low_altitude_unsafe_recovery_s = Vec::new();
    let mut success_reference_gap_mean_m = Vec::new();
    let mut success_transfer_shape_curve_rmse_m = Vec::new();
    let mut success_transfer_shape_apex_error_m = Vec::new();
    let mut success_transfer_shape_projected_dx_abs_mean_m = Vec::new();
    let mut success_transfer_shape_shortfall_ratio = Vec::new();
    let mut success_transfer_terminal_post_handoff_apex_gain_m = Vec::new();
    let mut success_transfer_terminal_post_handoff_time_to_apex_s = Vec::new();
    let mut success_transfer_terminal_post_handoff_apex_dx_abs_m = Vec::new();
    let mut success_transfer_terminal_low_altitude_rebound_gain_m = Vec::new();
    let mut success_transfer_terminal_low_altitude_rebound_origin_dx_abs_m = Vec::new();
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
        if record_failure(record) {
            failed_seeds.insert(record.resolved.resolved_seed);
            failure_pointers.push(pointer);
        } else if record_success(record) {
            success_fuel_remaining.push(record.manifest.summary.fuel_remaining_kg);
            success_sim_time_s.push(record.manifest.sim_time_s);
            if let Some(value) = record.review.fuel_used_pct_of_max {
                success_fuel_used_pct.push(value);
            }
            if let Some(value) = record.review.landing_offset_abs_m {
                success_landing_offset_abs_m.push(value);
            }
            if let Some(value) = record.review.low_altitude_dwell_s {
                success_low_altitude_dwell_s.push(value);
            }
            if let Some(value) = record.review.low_altitude_unsafe_recovery_s {
                success_low_altitude_unsafe_recovery_s.push(value);
            }
            if let Some(value) = record.review.reference_gap_mean_m {
                success_reference_gap_mean_m.push(value);
            }
            if let Some(value) = record.review.transfer_shape_curve_rmse_m {
                success_transfer_shape_curve_rmse_m.push(value);
            }
            if let Some(value) = record.review.transfer_shape_apex_error_m {
                success_transfer_shape_apex_error_m.push(value);
            }
            if let Some(value) = record.review.transfer_shape_projected_dx_abs_mean_m {
                success_transfer_shape_projected_dx_abs_mean_m.push(value);
            }
            if let Some(value) = record.review.transfer_shape_shortfall_ratio {
                success_transfer_shape_shortfall_ratio.push(value);
            }
            if let Some(value) = record.review.transfer_terminal_post_handoff_apex_gain_m {
                success_transfer_terminal_post_handoff_apex_gain_m.push(value);
            }
            if let Some(value) = record.review.transfer_terminal_post_handoff_time_to_apex_s {
                success_transfer_terminal_post_handoff_time_to_apex_s.push(value);
            }
            if let Some(value) = record.review.transfer_terminal_post_handoff_apex_dx_abs_m {
                success_transfer_terminal_post_handoff_apex_dx_abs_m.push(value);
            }
            if let Some(value) = record.review.transfer_terminal_low_altitude_rebound_gain_m {
                success_transfer_terminal_low_altitude_rebound_gain_m.push(value);
            }
            if let Some(value) = record
                .review
                .transfer_terminal_low_altitude_rebound_origin_dx_abs_m
            {
                success_transfer_terminal_low_altitude_rebound_origin_dx_abs_m.push(value);
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
        invalidated_runs,
        mean_sim_time_s,
        sim_time_stats: metric_summary(&success_sim_time_s),
        mean_success_fuel_remaining_kg,
        fuel_used_pct_of_max: metric_summary(&success_fuel_used_pct),
        landing_offset_abs_m: metric_summary(&success_landing_offset_abs_m),
        low_altitude_dwell_s: metric_summary(&success_low_altitude_dwell_s),
        low_altitude_unsafe_recovery_s: metric_summary(&success_low_altitude_unsafe_recovery_s),
        reference_gap_mean_m: metric_summary(&success_reference_gap_mean_m),
        transfer_shape_curve_rmse_m: metric_summary(&success_transfer_shape_curve_rmse_m),
        transfer_shape_apex_error_m: metric_summary(&success_transfer_shape_apex_error_m),
        transfer_shape_projected_dx_abs_mean_m: metric_summary(
            &success_transfer_shape_projected_dx_abs_mean_m,
        ),
        transfer_shape_shortfall_ratio: metric_summary(&success_transfer_shape_shortfall_ratio),
        transfer_terminal_post_handoff_apex_gain_m: metric_summary(
            &success_transfer_terminal_post_handoff_apex_gain_m,
        ),
        transfer_terminal_post_handoff_time_to_apex_s: metric_summary(
            &success_transfer_terminal_post_handoff_time_to_apex_s,
        ),
        transfer_terminal_post_handoff_apex_dx_abs_m: metric_summary(
            &success_transfer_terminal_post_handoff_apex_dx_abs_m,
        ),
        transfer_terminal_low_altitude_rebound_gain_m: metric_summary(
            &success_transfer_terminal_low_altitude_rebound_gain_m,
        ),
        transfer_terminal_low_altitude_rebound_origin_dx_abs_m: metric_summary(
            &success_transfer_terminal_low_altitude_rebound_origin_dx_abs_m,
        ),
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

pub(crate) fn run_pointer(record: &BatchRunRecord) -> BatchRunPointer {
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
        analytic: record.analytic.clone(),
        summary: record.manifest.summary.clone(),
    }
}

pub(crate) fn metric_summary(values: &[f64]) -> Option<BatchMetricSummary> {
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

fn summary_margin_ratio(summary: &RunSummary) -> Option<f64> {
    summary.envelope_margin_ratio
}

pub(crate) fn success_rate(success_runs: usize, total_runs: usize) -> f64 {
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
