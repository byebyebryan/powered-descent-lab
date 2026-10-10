use super::*;

pub(super) const UNSPECIFIED_SELECTOR_VALUE: &str = "unspecified";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct TransferShapeCellKey {
    condition_set: String,
    vehicle_variant: String,
    route_angle: String,
    radius_tier: String,
}

pub(super) struct TransferShapeCellSummary<'a> {
    key: TransferShapeCellKey,
    total_runs: usize,
    scored_runs: usize,
    success_runs: usize,
    shape_rmse_m: Option<crate::BatchMetricSummary>,
    apex_error_m: Option<crate::BatchMetricSummary>,
    shortfall_pct: Option<crate::BatchMetricSummary>,
    projected_dx_abs_max_m: Option<crate::BatchMetricSummary>,
    handoff_time_s: Option<crate::BatchMetricSummary>,
    handoff_gate_mode: Option<String>,
    boost_burn_duration_s: Option<crate::BatchMetricSummary>,
    boost_quality: Option<String>,
    gate_mode: Option<String>,
    corridor_mode: Option<String>,
    worst_shape_rmse_m: f64,
    worst_record: Option<&'a crate::BatchRunRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct WaypointCellKey {
    condition_set: String,
    vehicle_variant: String,
    route_angle: String,
    radius_tier: String,
    waypoint_profile: String,
    waypoint_handoff_envelope: String,
}

pub(super) struct WaypointCellSummary<'a> {
    key: WaypointCellKey,
    total_runs: usize,
    scored_runs: usize,
    success_runs: usize,
    contract_pass_runs: usize,
    spatial_miss_runs: usize,
    outbound_envelope_failure_runs: usize,
    incomplete_runs: usize,
    unknown_contract_runs: usize,
    captured_runs: usize,
    missed_runs: usize,
    tracking_runs: usize,
    planned_progress_frac: Option<crate::BatchMetricSummary>,
    planned_signed_offset_ratio: Option<crate::BatchMetricSummary>,
    planned_signed_turn_deg: Option<crate::BatchMetricSummary>,
    planned_max_speed_mps: Option<crate::BatchMetricSummary>,
    continuation_stop_ratio_max: Option<f64>,
    final_terminal_recovery_observed: usize,
    final_terminal_recoverable_runs: usize,
    final_terminal_required_accel_ratio_max: Option<f64>,
    closest_distance_m: Option<crate::BatchMetricSummary>,
    cross_track_m: Option<crate::BatchMetricSummary>,
    heading_error_deg: Option<crate::BatchMetricSummary>,
    outbound_progress_mps: Option<crate::BatchMetricSummary>,
    outbound_cross_speed_mps: Option<crate::BatchMetricSummary>,
    worst_record: Option<&'a crate::BatchRunRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct WaypointSequenceCellKey {
    condition_set: String,
    vehicle_variant: String,
    route_angle: String,
    radius_tier: String,
    waypoint_profile: String,
    waypoint_handoff_envelope: String,
    waypoint_index: usize,
}

pub(super) struct WaypointSequenceCellSummary {
    key: WaypointSequenceCellKey,
    waypoint_id: Option<String>,
    total_runs: usize,
    route_pass_runs: usize,
    route_failed_runs: usize,
    route_incomplete_runs: usize,
    handoff_runs: usize,
    contract_pass_runs: usize,
    spatial_miss_runs: usize,
    outbound_failure_runs: usize,
    incomplete_runs: usize,
    planned_progress_frac: Option<crate::BatchMetricSummary>,
    planned_signed_offset_ratio: Option<crate::BatchMetricSummary>,
    planned_signed_turn_deg: Option<crate::BatchMetricSummary>,
    planned_max_speed_mps: Option<crate::BatchMetricSummary>,
    continuation_stop_ratio_max: Option<f64>,
    final_terminal_recovery_observed: usize,
    final_terminal_recoverable_runs: usize,
    final_terminal_required_accel_ratio_max: Option<f64>,
    capture_time_s: Option<crate::BatchMetricSummary>,
    window_entry_time_s: Option<crate::BatchMetricSummary>,
    window_duration_s: Option<crate::BatchMetricSummary>,
    window_entry_observed: usize,
    window_entry_pass_runs: usize,
    window_recovery_runs: usize,
    deadline_resolution_runs: usize,
    cross_track_m: Option<crate::BatchMetricSummary>,
    heading_error_deg: Option<crate::BatchMetricSummary>,
    speed_mps: Option<crate::BatchMetricSummary>,
    target_velocity_error_mps: Option<crate::BatchMetricSummary>,
    target_deadline_remaining_s: Option<crate::BatchMetricSummary>,
    predicted_handoff_time_to_go_s: Option<crate::BatchMetricSummary>,
    predicted_handoff_deadline_lead_s: Option<crate::BatchMetricSummary>,
    predicted_contract_pass_runs: usize,
    predicted_contract_observed: usize,
    candidate_contract_pass_ever_runs: usize,
    candidate_contract_observed: usize,
    candidate_pass_lost_runs: usize,
    candidate_first_pass_time_s: Option<crate::BatchMetricSummary>,
    candidate_last_pass_time_s: Option<crate::BatchMetricSummary>,
    candidate_best_heading_margin_deg: Option<crate::BatchMetricSummary>,
    candidate_best_cross_speed_margin_mps: Option<crate::BatchMetricSummary>,
    reachable_candidate_contract_pass_ever_runs: usize,
    reachable_candidate_observed: usize,
    reachable_candidate_pass_lost_runs: usize,
    reachable_reference_disagreement_runs: usize,
    reachable_required_accel_ratio_max: Option<crate::BatchMetricSummary>,
    reachable_thrust_saturated_time_max_s: Option<crate::BatchMetricSummary>,
    reachable_tilt_saturated_time_max_s: Option<crate::BatchMetricSummary>,
    continuation_contract_pass_runs: usize,
    continuation_contract_observed: usize,
    continuation_outbound_heading_error_deg: Option<crate::BatchMetricSummary>,
    continuation_required_accel_ratio_max: Option<crate::BatchMetricSummary>,
    continuation_passing_candidate_count: Option<crate::BatchMetricSummary>,
    transition_observed: usize,
    transition_continuation_pass_runs: usize,
    transition_position_error_m: Option<crate::BatchMetricSummary>,
    transition_velocity_error_mps: Option<crate::BatchMetricSummary>,
    transition_attitude_error_deg: Option<crate::BatchMetricSummary>,
    transition_mass_error_kg: Option<crate::BatchMetricSummary>,
    transition_fuel_error_kg: Option<crate::BatchMetricSummary>,
    transition_event_time_error_s: Option<crate::BatchMetricSummary>,
    transition_continuation_outbound_heading_error_deg: Option<crate::BatchMetricSummary>,
    transition_continuation_required_accel_ratio_max: Option<crate::BatchMetricSummary>,
    transition_continuation_passing_candidate_count: Option<crate::BatchMetricSummary>,
    joint_observed: usize,
    joint_contract_pass_runs: usize,
    joint_evaluated_candidate_count: Option<crate::BatchMetricSummary>,
    joint_passing_candidate_count: Option<crate::BatchMetricSummary>,
    joint_time_to_go_s: Option<crate::BatchMetricSummary>,
    joint_continuation_outbound_heading_error_deg: Option<crate::BatchMetricSummary>,
    joint_required_accel_ratio_max: Option<crate::BatchMetricSummary>,
    joint_total_saturated_time_s: Option<crate::BatchMetricSummary>,
    plan_reference_position_error_max_m: Option<crate::BatchMetricSummary>,
    plan_reference_cross_error_max_abs_m: Option<crate::BatchMetricSummary>,
    plan_reference_velocity_error_max_mps: Option<crate::BatchMetricSummary>,
    plan_reference_cross_speed_error_max_abs_mps: Option<crate::BatchMetricSummary>,
    guidance_required_accel_ratio_max: Option<crate::BatchMetricSummary>,
    guidance_thrust_saturated_time_s: Option<crate::BatchMetricSummary>,
    guidance_tilt_saturated_time_s: Option<crate::BatchMetricSummary>,
    guidance_first_saturation_lead_s: Option<crate::BatchMetricSummary>,
    last_pass_reference_position_error_m: Option<crate::BatchMetricSummary>,
    last_pass_reference_velocity_error_mps: Option<crate::BatchMetricSummary>,
    last_pass_required_accel_ratio: Option<crate::BatchMetricSummary>,
    guidance_plan_revision_max: Option<crate::BatchMetricSummary>,
    guidance_plan_reasons: Vec<String>,
    handoff_turn_margin_m: Option<crate::BatchMetricSummary>,
    guidance_feasible_runs: usize,
    guidance_feasible_observed: usize,
    guidance_replans: Option<crate::BatchMetricSummary>,
}

pub(super) struct TransferHandoffCellSummary<'a> {
    key: TransferShapeCellKey,
    total_runs: usize,
    scored_runs: usize,
    success_runs: usize,
    frontier_runs: usize,
    failed_scored_runs: usize,
    terminal_entry_kind: Option<String>,
    handoff_gate_mode: Option<String>,
    handoff_height_m: Option<crate::BatchMetricSummary>,
    handoff_speed_mps: Option<crate::BatchMetricSummary>,
    handoff_projected_dx_abs_m: Option<crate::BatchMetricSummary>,
    handoff_impact_angle_deg: Option<crate::BatchMetricSummary>,
    cutoff_quality: Option<String>,
    cutoff_projected_dx_abs_m: Option<crate::BatchMetricSummary>,
    cutoff_impact_angle_deg: Option<crate::BatchMetricSummary>,
    post_handoff_apex_gain_m: Option<crate::BatchMetricSummary>,
    low_altitude_rebound_gain_m: Option<crate::BatchMetricSummary>,
    low_altitude_rebound_origin_dx_abs_m: Option<crate::BatchMetricSummary>,
    near_pad_rebound_runs: usize,
    worst_near_pad_rebound_m: f64,
    worst_record: Option<&'a crate::BatchRunRecord>,
}

pub(super) fn render_waypoint_sequence_section(candidate: &BatchReport) -> String {
    let candidate_records = preferred_current_lane_focus(candidate)
        .map(|focus| focus.records)
        .unwrap_or_else(|| candidate.records.iter().collect::<Vec<_>>());
    let sequence_records = candidate_records
        .iter()
        .copied()
        .filter(|record| {
            record
                .review
                .waypoint_route_total
                .is_some_and(|total| total > 1)
        })
        .collect::<Vec<_>>();
    if sequence_records.is_empty() {
        return String::new();
    }

    let route_pass_runs = sequence_records
        .iter()
        .filter(|record| record.review.waypoint_route_status.as_deref() == Some("pass"))
        .count();
    let route_failed_runs = sequence_records
        .iter()
        .filter(|record| record.review.waypoint_route_status.as_deref() == Some("failed"))
        .count();
    let route_incomplete_runs = sequence_records.len() - route_pass_runs - route_failed_runs;
    let mut rows = waypoint_sequence_cell_summaries(sequence_records.as_slice());
    rows.sort_by(compare_waypoint_sequence_cells);
    let row_html = rows
        .iter()
        .map(render_waypoint_sequence_row)
        .collect::<String>();
    let trackability_row_html = rows
        .iter()
        .filter(|summary| summary.plan_reference_position_error_max_m.is_some())
        .map(render_waypoint_trackability_row)
        .collect::<String>();
    let trackability_html = if trackability_row_html.is_empty() {
        String::new()
    } else {
        format!(
            r#"<details class="transfer-handoff-section waypoint-trackability-section">
  <summary class="section-head transfer-triage-summary">
    <h2>Waypoint Plan Trackability</h2>
    <div class="section-note">Hermite reference drift, actuated reachable forecast, control saturation, and state at the last reference-pass prediction.</div>
  </summary>
  <div class="table-wrap">
    <table class="transfer-handoff-table waypoint-trackability-table">
      <thead><tr><th>Route</th><th>Vehicle</th><th>Waypoint</th><th>Reference Max</th><th>Reachable Forecast</th><th>Continuation</th><th>Authority</th><th>Last Pass</th><th>Plans</th></tr></thead>
      <tbody>{trackability_row_html}</tbody>
    </table>
  </div>
</details>"#,
        )
    };
    let continuation_audit_row_html = rows
        .iter()
        .filter(|summary| {
            summary.continuation_contract_observed > 0
                || summary.transition_observed > 0
                || summary.joint_observed > 0
        })
        .map(render_waypoint_continuation_audit_row)
        .collect::<String>();
    let continuation_seed_row_html = render_waypoint_continuation_seed_rows(&sequence_records);
    let continuation_audit_html = if continuation_audit_row_html.is_empty() {
        String::new()
    } else {
        let seed_html = if continuation_seed_row_html.is_empty() {
            String::new()
        } else {
            format!(
                r#"<details class="transfer-handoff-section waypoint-continuation-seed-section">
  <summary class="section-head transfer-triage-summary">
    <h3>Seed Evidence</h3>
    <div class="section-note">Exact transition and selected joint-state evidence for each observed handoff.</div>
  </summary>
  <div class="table-wrap">
    <table class="transfer-handoff-table waypoint-continuation-seed-table">
      <thead><tr><th>Route</th><th>Vehicle</th><th>Seed</th><th>Waypoint</th><th>Planned</th><th>Transition Error</th><th>Actual Continuation</th><th>Joint Search</th></tr></thead>
      <tbody>{continuation_seed_row_html}</tbody>
    </table>
  </div>
</details>"#,
            )
        };
        format!(
            r#"<details class="transfer-handoff-section waypoint-continuation-audit-section">
  <summary class="section-head transfer-triage-summary">
    <h2>Waypoint Continuation Audit</h2>
    <div class="section-note">Planned projection, actual transition state, next-leg viability, and bounded joint-state search. Legacy packs retain planned evidence and show dashes for schema-31 fields.</div>
  </summary>
  <div class="table-wrap">
    <table class="transfer-handoff-table waypoint-continuation-audit-table">
      <thead><tr><th>Route</th><th>Vehicle</th><th>Waypoint</th><th>Planned Continuation</th><th>Transition Error</th><th>Actual Continuation</th><th>Joint Search</th></tr></thead>
      <tbody>{continuation_audit_row_html}</tbody>
    </table>
  </div>
  {seed_html}
</details>"#,
        )
    };

    format!(
        r#"<details class="transfer-handoff-section waypoint-sequence-section">
  <summary class="section-head transfer-triage-summary">
    <h2>Waypoint Sequence</h2>
    <div class="section-note">Current-lane ordered route result: {route_pass_runs}/{total_runs} pass · {route_failed_runs} failed · {route_incomplete_runs} incomplete. Each contract resolves on planned-tangent envelope pass or at the waypoint-plane deadline.</div>
  </summary>
  <div class="table-wrap">
    <table class="transfer-handoff-table waypoint-sequence-table">
      <thead>
        <tr>
          <th>Route</th>
          <th>Vehicle</th>
          <th>Profile</th>
          <th>Waypoint</th>
          <th>Plan</th>
          <th>Route Result</th>
          <th>Contract</th>
          <th>Terminal Recovery</th>
          <th>Entry / Resolution</th>
          <th>Cross Track</th>
          <th>Tangent Error</th>
          <th>Speed</th>
          <th>State Debt</th>
          <th>Replans</th>
        </tr>
      </thead>
      <tbody>{row_html}</tbody>
    </table>
  </div>
</details>
{trackability_html}
{continuation_audit_html}"#,
        total_runs = sequence_records.len(),
    )
}

pub(super) fn waypoint_sequence_cell_summaries(
    records: &[&crate::BatchRunRecord],
) -> Vec<WaypointSequenceCellSummary> {
    let mut grouped = BTreeMap::<WaypointSequenceCellKey, Vec<&crate::BatchRunRecord>>::new();
    for &record in records {
        let Some(total) = record
            .review
            .waypoint_route_total
            .filter(|total| *total > 1)
        else {
            continue;
        };
        let selector = &record.resolved.selector;
        for waypoint_index in 0..total {
            grouped
                .entry(WaypointSequenceCellKey {
                    condition_set: selector.condition_set.clone(),
                    vehicle_variant: selector.vehicle_variant.clone(),
                    route_angle: selector_preferred_value(
                        &selector.route_angle,
                        &selector.arc_point,
                    ),
                    radius_tier: selector_preferred_value(
                        &selector.radius_tier,
                        &selector.velocity_band,
                    ),
                    waypoint_profile: selector_value_or_unspecified(&selector.waypoint_profile),
                    waypoint_handoff_envelope: selector_value_or_unspecified(
                        &selector.waypoint_handoff_envelope,
                    ),
                    waypoint_index,
                })
                .or_default()
                .push(record);
        }
    }

    grouped
        .into_iter()
        .map(|(key, records)| waypoint_sequence_cell_summary(key, records.as_slice()))
        .collect()
}

pub(super) fn waypoint_sequence_cell_summary(
    key: WaypointSequenceCellKey,
    records: &[&crate::BatchRunRecord],
) -> WaypointSequenceCellSummary {
    let waypoint_index = key.waypoint_index;
    let waypoint_id = records.iter().find_map(|record| {
        waypoint_sequence_handoff(record, waypoint_index)
            .and_then(|handoff| handoff.waypoint_id.clone())
    });
    let route_pass_runs = records
        .iter()
        .filter(|record| record.review.waypoint_route_status.as_deref() == Some("pass"))
        .count();
    let route_failed_runs = records
        .iter()
        .filter(|record| record.review.waypoint_route_status.as_deref() == Some("failed"))
        .count();
    let route_incomplete_runs = records.len() - route_pass_runs - route_failed_runs;
    let handoff_runs = records
        .iter()
        .filter(|record| waypoint_sequence_handoff(record, waypoint_index).is_some())
        .count();
    let contract_count = |status: &str| {
        records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.contract_status.as_deref())
                    == Some(status)
            })
            .count()
    };
    let outbound_failure_runs = records
        .iter()
        .filter(|record| {
            matches!(
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.contract_status.as_deref()),
                Some("outbound_out_of_envelope" | "outbound_unviable")
            )
        })
        .count();
    let metric = |extractor: fn(&crate::BatchWaypointHandoffReviewMetrics) -> Option<f64>| {
        transfer_shape_record_metric_summary(records, |record| {
            waypoint_sequence_handoff(record, waypoint_index).and_then(extractor)
        })
    };
    let entry_metric =
        |extractor: fn(&crate::BatchWaypointWindowEntryReviewMetrics) -> Option<f64>| {
            transfer_shape_record_metric_summary(records, |record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.window_entry.as_ref())
                    .and_then(extractor)
            })
        };

    WaypointSequenceCellSummary {
        key,
        waypoint_id,
        total_runs: records.len(),
        route_pass_runs,
        route_failed_runs,
        route_incomplete_runs,
        handoff_runs,
        contract_pass_runs: contract_count("pass"),
        spatial_miss_runs: contract_count("spatial_miss"),
        outbound_failure_runs,
        incomplete_runs: contract_count("incomplete"),
        planned_progress_frac: waypoint_resolved_metric_summary(
            records,
            waypoint_index,
            "profile_progress_frac",
        ),
        planned_signed_offset_ratio: waypoint_resolved_metric_summary(
            records,
            waypoint_index,
            "route_signed_offset_ratio",
        ),
        planned_signed_turn_deg: waypoint_resolved_metric_summary(
            records,
            waypoint_index,
            "signed_turn_angle_deg",
        ),
        planned_max_speed_mps: waypoint_resolved_metric_summary(
            records,
            waypoint_index,
            "max_speed_mps",
        ),
        continuation_stop_ratio_max: waypoint_resolved_metric_max(
            records,
            waypoint_index,
            "continuation_stop_ratio",
        ),
        final_terminal_recovery_observed: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .is_some_and(|handoff| handoff.final_terminal_recoverable.is_some())
            })
            .count(),
        final_terminal_recoverable_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.final_terminal_recoverable)
                    == Some(true)
            })
            .count(),
        final_terminal_required_accel_ratio_max: records
            .iter()
            .filter_map(|record| waypoint_sequence_handoff(record, waypoint_index))
            .filter_map(|handoff| handoff.final_terminal_required_accel_ratio)
            .filter(|ratio| ratio.is_finite())
            .max_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(Ordering::Equal)),
        capture_time_s: metric(|handoff| handoff.capture_time_s),
        window_entry_time_s: entry_metric(|entry| entry.time_s),
        window_duration_s: metric(|handoff| handoff.window_duration_s),
        window_entry_observed: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .is_some_and(|handoff| handoff.window_entry.is_some())
            })
            .count(),
        window_entry_pass_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.window_entry.as_ref())
                    .and_then(|entry| entry.contract_pass)
                    == Some(true)
            })
            .count(),
        window_recovery_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index).is_some_and(|handoff| {
                    handoff
                        .window_entry
                        .as_ref()
                        .and_then(|entry| entry.contract_pass)
                        == Some(false)
                        && handoff.contract_status.as_deref() == Some("pass")
                })
            })
            .count(),
        deadline_resolution_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.resolution_reason.as_deref())
                    == Some("plane_deadline")
            })
            .count(),
        cross_track_m: metric(|handoff| handoff.cross_track_m),
        heading_error_deg: metric(|handoff| {
            handoff.outbound_heading_error_rad.map(f64::to_degrees)
        }),
        speed_mps: metric(|handoff| handoff.speed_mps),
        target_velocity_error_mps: metric(|handoff| handoff.target_velocity_error_mps),
        target_deadline_remaining_s: metric(|handoff| handoff.target_deadline_remaining_s),
        predicted_handoff_time_to_go_s: metric(|handoff| handoff.predicted_handoff_time_to_go_s),
        predicted_handoff_deadline_lead_s: metric(|handoff| {
            handoff.predicted_handoff_deadline_lead_s
        }),
        predicted_contract_pass_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.predicted_handoff_contract_status.as_deref())
                    == Some("pass")
            })
            .count(),
        predicted_contract_observed: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .is_some_and(|handoff| handoff.predicted_handoff_contract_status.is_some())
            })
            .count(),
        candidate_contract_pass_ever_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.candidate_contract_pass_ever)
                    == Some(true)
            })
            .count(),
        candidate_contract_observed: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .is_some_and(|handoff| handoff.candidate_contract_pass_ever.is_some())
            })
            .count(),
        candidate_pass_lost_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.candidate_pass_lost_before_capture)
                    == Some(true)
            })
            .count(),
        candidate_first_pass_time_s: metric(|handoff| handoff.candidate_first_pass_time_s),
        candidate_last_pass_time_s: metric(|handoff| handoff.candidate_last_pass_time_s),
        candidate_best_heading_margin_deg: metric(|handoff| {
            handoff
                .candidate_best_heading_margin_rad
                .map(f64::to_degrees)
        }),
        candidate_best_cross_speed_margin_mps: metric(|handoff| {
            handoff.candidate_best_cross_speed_margin_mps
        }),
        reachable_candidate_contract_pass_ever_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.reachable_candidate_contract_pass_ever)
                    == Some(true)
            })
            .count(),
        reachable_candidate_observed: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .is_some_and(|handoff| handoff.reachable_candidate_contract_pass_ever.is_some())
            })
            .count(),
        reachable_candidate_pass_lost_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.reachable_candidate_pass_lost_before_capture)
                    == Some(true)
            })
            .count(),
        reachable_reference_disagreement_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index).is_some_and(|handoff| {
                    handoff.candidate_contract_pass_ever.is_some()
                        && handoff.reachable_candidate_contract_pass_ever.is_some()
                        && handoff.candidate_contract_pass_ever
                            != handoff.reachable_candidate_contract_pass_ever
                })
            })
            .count(),
        reachable_required_accel_ratio_max: metric(|handoff| {
            handoff.reachable_required_accel_ratio_max
        }),
        reachable_thrust_saturated_time_max_s: metric(|handoff| {
            handoff.reachable_thrust_saturated_time_max_s
        }),
        reachable_tilt_saturated_time_max_s: metric(|handoff| {
            handoff.reachable_tilt_saturated_time_max_s
        }),
        continuation_contract_pass_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.continuation_contract_pass)
                    == Some(true)
            })
            .count(),
        continuation_contract_observed: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .is_some_and(|handoff| handoff.continuation_contract_pass.is_some())
            })
            .count(),
        continuation_outbound_heading_error_deg: metric(|handoff| {
            handoff
                .continuation_outbound_heading_error_rad
                .map(f64::to_degrees)
        }),
        continuation_required_accel_ratio_max: metric(|handoff| {
            handoff.continuation_required_accel_ratio_max
        }),
        continuation_passing_candidate_count: metric(|handoff| {
            handoff
                .continuation_passing_candidate_count
                .map(|count| count as f64)
        }),
        transition_observed: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .is_some_and(|handoff| handoff.transition_next_waypoint_index.is_some())
            })
            .count(),
        transition_continuation_pass_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.transition_continuation_contract_pass)
                    == Some(true)
            })
            .count(),
        transition_position_error_m: metric(|handoff| handoff.transition_position_error_m),
        transition_velocity_error_mps: metric(|handoff| handoff.transition_velocity_error_mps),
        transition_attitude_error_deg: metric(|handoff| {
            handoff.transition_attitude_error_rad.map(f64::to_degrees)
        }),
        transition_mass_error_kg: metric(|handoff| handoff.transition_mass_error_kg),
        transition_fuel_error_kg: metric(|handoff| handoff.transition_fuel_error_kg),
        transition_event_time_error_s: metric(|handoff| handoff.transition_event_time_error_s),
        transition_continuation_outbound_heading_error_deg: metric(|handoff| {
            handoff
                .transition_continuation_outbound_heading_error_rad
                .map(f64::to_degrees)
        }),
        transition_continuation_required_accel_ratio_max: metric(|handoff| {
            handoff.transition_continuation_required_accel_ratio_max
        }),
        transition_continuation_passing_candidate_count: metric(|handoff| {
            handoff
                .transition_continuation_passing_candidate_count
                .map(|count| count as f64)
        }),
        joint_observed: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .is_some_and(|handoff| handoff.joint_next_waypoint_index.is_some())
            })
            .count(),
        joint_contract_pass_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.joint_contract_pass)
                    == Some(true)
            })
            .count(),
        joint_evaluated_candidate_count: metric(|handoff| {
            handoff
                .joint_evaluated_candidate_count
                .map(|count| count as f64)
        }),
        joint_passing_candidate_count: metric(|handoff| {
            handoff
                .joint_passing_candidate_count
                .map(|count| count as f64)
        }),
        joint_time_to_go_s: metric(|handoff| handoff.joint_time_to_go_s),
        joint_continuation_outbound_heading_error_deg: metric(|handoff| {
            handoff
                .joint_continuation_outbound_heading_error_rad
                .map(f64::to_degrees)
        }),
        joint_required_accel_ratio_max: metric(|handoff| handoff.joint_required_accel_ratio_max),
        joint_total_saturated_time_s: metric(|handoff| handoff.joint_total_saturated_time_s),
        plan_reference_position_error_max_m: metric(|handoff| {
            handoff.plan_reference_position_error_max_m
        }),
        plan_reference_cross_error_max_abs_m: metric(|handoff| {
            handoff.plan_reference_cross_error_max_abs_m
        }),
        plan_reference_velocity_error_max_mps: metric(|handoff| {
            handoff.plan_reference_velocity_error_max_mps
        }),
        plan_reference_cross_speed_error_max_abs_mps: metric(|handoff| {
            handoff.plan_reference_cross_speed_error_max_abs_mps
        }),
        guidance_required_accel_ratio_max: metric(|handoff| {
            handoff.guidance_required_accel_ratio_max
        }),
        guidance_thrust_saturated_time_s: metric(|handoff| {
            handoff.guidance_thrust_saturated_time_s
        }),
        guidance_tilt_saturated_time_s: metric(|handoff| handoff.guidance_tilt_saturated_time_s),
        guidance_first_saturation_lead_s: metric(|handoff| {
            handoff.guidance_first_saturation_lead_s
        }),
        last_pass_reference_position_error_m: metric(|handoff| {
            handoff.last_pass_reference_position_error_m
        }),
        last_pass_reference_velocity_error_mps: metric(|handoff| {
            handoff.last_pass_reference_velocity_error_mps
        }),
        last_pass_required_accel_ratio: metric(|handoff| handoff.last_pass_required_accel_ratio),
        guidance_plan_revision_max: metric(|handoff| {
            handoff.guidance_plan_revision_max.map(|value| value as f64)
        }),
        guidance_plan_reasons: records
            .iter()
            .filter_map(|record| waypoint_sequence_handoff(record, waypoint_index))
            .flat_map(|handoff| handoff.guidance_plan_reasons.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        handoff_turn_margin_m: metric(|handoff| handoff.handoff_turn_margin_m),
        guidance_feasible_runs: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .and_then(|handoff| handoff.guidance_feasible)
                    == Some(true)
            })
            .count(),
        guidance_feasible_observed: records
            .iter()
            .filter(|record| {
                waypoint_sequence_handoff(record, waypoint_index)
                    .is_some_and(|handoff| handoff.guidance_feasible.is_some())
            })
            .count(),
        guidance_replans: metric(|handoff| handoff.guidance_replan_count.map(|value| value as f64)),
    }
}

pub(super) fn waypoint_sequence_handoff(
    record: &crate::BatchRunRecord,
    waypoint_index: usize,
) -> Option<&crate::BatchWaypointHandoffReviewMetrics> {
    record
        .review
        .waypoint_handoffs
        .iter()
        .find(|handoff| handoff.waypoint_index == waypoint_index)
}

pub(super) fn waypoint_final_handoff(
    record: &crate::BatchRunRecord,
) -> Option<&crate::BatchWaypointHandoffReviewMetrics> {
    record
        .review
        .waypoint_handoffs
        .iter()
        .max_by_key(|handoff| handoff.waypoint_index)
}

pub(super) fn waypoint_resolved_metric_summary(
    records: &[&crate::BatchRunRecord],
    waypoint_index: usize,
    metric: &str,
) -> Option<crate::BatchMetricSummary> {
    let key = format!("waypoint_{waypoint_index}_{metric}");
    transfer_shape_record_metric_summary(records, |record| {
        record.resolved.resolved_parameters.get(&key).copied()
    })
}

pub(super) fn waypoint_resolved_metric_max(
    records: &[&crate::BatchRunRecord],
    waypoint_index: usize,
    metric: &str,
) -> Option<f64> {
    let key = format!("waypoint_{waypoint_index}_{metric}");
    records
        .iter()
        .filter_map(|record| record.resolved.resolved_parameters.get(&key).copied())
        .filter(|value| value.is_finite())
        .max_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(Ordering::Equal))
}

pub(super) fn compare_waypoint_sequence_cells(
    lhs: &WaypointSequenceCellSummary,
    rhs: &WaypointSequenceCellSummary,
) -> Ordering {
    selector_sort_rank(&lhs.key.route_angle)
        .cmp(&selector_sort_rank(&rhs.key.route_angle))
        .then(lhs.key.route_angle.cmp(&rhs.key.route_angle))
        .then(
            selector_sort_rank(&lhs.key.radius_tier).cmp(&selector_sort_rank(&rhs.key.radius_tier)),
        )
        .then(
            selector_sort_rank(&lhs.key.vehicle_variant)
                .cmp(&selector_sort_rank(&rhs.key.vehicle_variant)),
        )
        .then(
            selector_sort_rank(&lhs.key.waypoint_profile)
                .cmp(&selector_sort_rank(&rhs.key.waypoint_profile)),
        )
        .then(
            lhs.key
                .waypoint_handoff_envelope
                .cmp(&rhs.key.waypoint_handoff_envelope),
        )
        .then(lhs.key.waypoint_index.cmp(&rhs.key.waypoint_index))
}

pub(super) fn render_waypoint_sequence_row(summary: &WaypointSequenceCellSummary) -> String {
    let route = format!(
        r#"<div class="overview-stack"><div class="overview-main"><code>{}</code></div><div class="overview-sub">{} · {}</div></div>"#,
        escape_html(&summary.key.route_angle),
        escape_html(&summary.key.condition_set),
        escape_html(&summary.key.radius_tier),
    );
    let waypoint = format!(
        r#"<div class="overview-stack"><div class="overview-main"><code>#{}</code> {}</div><div class="overview-sub">{} / {} observed</div></div>"#,
        summary.key.waypoint_index + 1,
        escape_html(summary.waypoint_id.as_deref().unwrap_or("waypoint")),
        summary.handoff_runs,
        summary.total_runs,
    );
    let route_result = format!(
        r#"<div class="overview-stack"><div class="overview-main">{} / {} pass</div><div class="overview-sub">{} failed · {} incomplete</div></div>"#,
        summary.route_pass_runs,
        summary.total_runs,
        summary.route_failed_runs,
        summary.route_incomplete_runs,
    );
    let missing_runs = summary.total_runs.saturating_sub(summary.handoff_runs);
    let contract = format!(
        r#"<div class="overview-stack"><div class="overview-main">{} / {} pass</div><div class="overview-sub">{} spatial · {} handoff envelope · {} incomplete · {} missing</div></div>"#,
        summary.contract_pass_runs,
        summary.total_runs,
        summary.spatial_miss_runs,
        summary.outbound_failure_runs,
        summary.incomplete_runs,
        missing_runs,
    );
    let replans = summary
        .guidance_replans
        .as_ref()
        .map(|value| {
            let spread = value
                .stddev
                .map(|stddev| format!("stddev {stddev:.1}"))
                .unwrap_or_else(|| "single value".to_owned());
            format!(
                r#"<div class="overview-stack"><div class="overview-main">{:.1}</div><div class="overview-sub">{}</div></div>"#,
                value.mean,
                escape_html(&spread),
            )
        })
        .unwrap_or_else(|| r#"<span class="muted">-</span>"#.to_owned());
    let state_debt = render_waypoint_sequence_state_debt(summary);
    let plan = render_waypoint_plan_cell(
        summary.planned_progress_frac.as_ref(),
        summary.planned_signed_offset_ratio.as_ref(),
        summary.planned_signed_turn_deg.as_ref(),
        summary.planned_max_speed_mps.as_ref(),
        summary.continuation_stop_ratio_max,
    );
    let profile = format!(
        r#"<div class="overview-stack"><div class="overview-main"><code>{}</code></div><div class="overview-sub">{}</div></div>"#,
        escape_html(&summary.key.waypoint_profile),
        escape_html(&summary.key.waypoint_handoff_envelope),
    );
    format!(
        r#"<tr>
  <td>{route}</td>
  <td><code>{vehicle}</code></td>
  <td>{profile}</td>
  <td>{waypoint}</td>
  <td>{plan}</td>
  <td>{route_result}</td>
  <td>{contract}</td>
  <td>{terminal_recovery}</td>
  <td>{window}</td>
  <td>{cross_track}</td>
  <td>{heading_error}</td>
  <td>{speed}</td>
  <td>{state_debt}</td>
  <td>{replans}</td>
</tr>"#,
        vehicle = escape_html(&summary.key.vehicle_variant),
        profile = profile,
        plan = plan,
        terminal_recovery = render_waypoint_terminal_recovery_cell(
            summary.final_terminal_recovery_observed,
            summary.final_terminal_recoverable_runs,
            summary.final_terminal_required_accel_ratio_max,
        ),
        window = render_waypoint_window_cell(summary),
        cross_track = render_transfer_handoff_metric_cell(
            summary.cross_track_m.as_ref(),
            MetricDisplayKind::Meters,
            None,
        ),
        heading_error = render_transfer_handoff_metric_cell(
            summary.heading_error_deg.as_ref(),
            MetricDisplayKind::Degrees,
            None,
        ),
        speed = render_transfer_handoff_metric_cell(
            summary.speed_mps.as_ref(),
            MetricDisplayKind::Speed,
            None,
        ),
    )
}

pub(super) fn render_waypoint_window_cell(summary: &WaypointSequenceCellSummary) -> String {
    if summary.window_entry_observed == 0 {
        return format!(
            r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">legacy resolution only</div></div>"#,
            render_transfer_handoff_metric_cell(
                summary.capture_time_s.as_ref(),
                MetricDisplayKind::Seconds,
                None,
            ),
        );
    }
    let mean = |metric: &Option<crate::BatchMetricSummary>| {
        metric
            .as_ref()
            .map(|value| format!("{:.2}s", value.mean))
            .unwrap_or_else(|| "-".to_owned())
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{} / {} clean entry</div><div class="overview-sub">{} recovered · {} deadline</div><div class="overview-sub">{} entry · {} resolve · {} window</div></div>"#,
        summary.window_entry_pass_runs,
        summary.window_entry_observed,
        summary.window_recovery_runs,
        summary.deadline_resolution_runs,
        mean(&summary.window_entry_time_s),
        mean(&summary.capture_time_s),
        mean(&summary.window_duration_s),
    )
}

pub(super) fn render_waypoint_trackability_row(summary: &WaypointSequenceCellSummary) -> String {
    let mean = |value: &Option<crate::BatchMetricSummary>, suffix: &str| {
        value
            .as_ref()
            .map(|value| format!("{:.2}{suffix}", value.mean))
            .unwrap_or_else(|| "-".to_owned())
    };
    let waypoint = format!(
        r#"<div class="overview-stack"><div class="overview-main"><code>#{}</code> {}</div><div class="overview-sub"><code>{}</code></div></div>"#,
        summary.key.waypoint_index + 1,
        escape_html(summary.waypoint_id.as_deref().unwrap_or("waypoint")),
        escape_html(&summary.key.waypoint_profile),
    );
    let reference = format!(
        r#"<div class="overview-stack"><div class="overview-main">pos {} · vel {}</div><div class="overview-sub">cross {} · cross vel {}</div></div>"#,
        mean(&summary.plan_reference_position_error_max_m, "m"),
        mean(&summary.plan_reference_velocity_error_max_mps, "m/s"),
        mean(&summary.plan_reference_cross_error_max_abs_m, "m"),
        mean(&summary.plan_reference_cross_speed_error_max_abs_mps, "m/s"),
    );
    let reachable = if summary.reachable_candidate_observed == 0 {
        r#"<span class="muted">legacy reference only</span>"#.to_owned()
    } else {
        format!(
            r#"<div class="overview-stack"><div class="overview-main">pass {}/{} · lost {}</div><div class="overview-sub">disagree {} · peak {:.2}x</div><div class="overview-sub">thrust {} · tilt {}</div></div>"#,
            summary.reachable_candidate_contract_pass_ever_runs,
            summary.reachable_candidate_observed,
            summary.reachable_candidate_pass_lost_runs,
            summary.reachable_reference_disagreement_runs,
            summary
                .reachable_required_accel_ratio_max
                .as_ref()
                .map_or(0.0, |value| value.mean),
            mean(&summary.reachable_thrust_saturated_time_max_s, "s"),
            mean(&summary.reachable_tilt_saturated_time_max_s, "s"),
        )
    };
    let continuation = if summary.continuation_contract_observed == 0 {
        r#"<span class="muted">not observed</span>"#.to_owned()
    } else {
        format!(
            r#"<div class="overview-stack"><div class="overview-main">pass {}/{}</div><div class="overview-sub">heading {} · peak {}</div><div class="overview-sub">passing candidates {}</div></div>"#,
            summary.continuation_contract_pass_runs,
            summary.continuation_contract_observed,
            mean(&summary.continuation_outbound_heading_error_deg, "deg"),
            mean(&summary.continuation_required_accel_ratio_max, "x"),
            mean(&summary.continuation_passing_candidate_count, ""),
        )
    };
    let authority = format!(
        r#"<div class="overview-stack"><div class="overview-main">peak {:.2}x</div><div class="overview-sub">thrust {} · tilt {}</div><div class="overview-sub">first lead {}</div></div>"#,
        summary
            .guidance_required_accel_ratio_max
            .as_ref()
            .map_or(0.0, |value| value.mean),
        mean(&summary.guidance_thrust_saturated_time_s, "s"),
        mean(&summary.guidance_tilt_saturated_time_s, "s"),
        mean(&summary.guidance_first_saturation_lead_s, "s"),
    );
    let last_pass = if summary.last_pass_reference_position_error_m.is_none() {
        r#"<span class="muted">never passing</span>"#.to_owned()
    } else {
        format!(
            r#"<div class="overview-stack"><div class="overview-main">pos {} · vel {}</div><div class="overview-sub">required {:.2}x</div></div>"#,
            mean(&summary.last_pass_reference_position_error_m, "m"),
            mean(&summary.last_pass_reference_velocity_error_mps, "m/s"),
            summary
                .last_pass_required_accel_ratio
                .as_ref()
                .map_or(0.0, |value| value.mean),
        )
    };
    let plans = format!(
        r#"<div class="overview-stack"><div class="overview-main">revision max {:.1}</div><div class="overview-sub">{}</div></div>"#,
        summary
            .guidance_plan_revision_max
            .as_ref()
            .map_or(0.0, |value| value.mean),
        escape_html(&summary.guidance_plan_reasons.join(", ")),
    );
    format!(
        r#"<tr><td><code>{}</code></td><td><code>{}</code></td><td>{waypoint}</td><td>{reference}</td><td>{reachable}</td><td>{continuation}</td><td>{authority}</td><td>{last_pass}</td><td>{plans}</td></tr>"#,
        escape_html(&summary.key.route_angle),
        escape_html(&summary.key.vehicle_variant),
    )
}

pub(super) fn render_waypoint_continuation_audit_row(
    summary: &WaypointSequenceCellSummary,
) -> String {
    let mean = |value: &Option<crate::BatchMetricSummary>, suffix: &str| {
        value
            .as_ref()
            .map(|value| format!("{:.2}{suffix}", value.mean))
            .unwrap_or_else(|| "-".to_owned())
    };
    let waypoint = format!(
        r#"<div class="overview-stack"><div class="overview-main"><code>#{}</code> {}</div><div class="overview-sub"><code>{}</code></div></div>"#,
        summary.key.waypoint_index + 1,
        escape_html(summary.waypoint_id.as_deref().unwrap_or("waypoint")),
        escape_html(&summary.key.waypoint_profile),
    );
    let planned = if summary.continuation_contract_observed == 0 {
        r#"<span class="muted">not observed</span>"#.to_owned()
    } else {
        format!(
            r#"<div class="overview-stack"><div class="overview-main">pass {}/{}</div><div class="overview-sub">heading {} · peak {}</div><div class="overview-sub">next candidates {}</div></div>"#,
            summary.continuation_contract_pass_runs,
            summary.continuation_contract_observed,
            mean(&summary.continuation_outbound_heading_error_deg, "deg"),
            mean(&summary.continuation_required_accel_ratio_max, "x"),
            mean(&summary.continuation_passing_candidate_count, ""),
        )
    };
    let transition = if summary.transition_observed == 0 {
        r#"<span class="muted">legacy schema or no transition</span>"#.to_owned()
    } else {
        format!(
            r#"<div class="overview-stack"><div class="overview-main">pos {} · vel {}</div><div class="overview-sub">att {} · time {}</div><div class="overview-sub">mass {} · fuel {}</div></div>"#,
            mean(&summary.transition_position_error_m, "m"),
            mean(&summary.transition_velocity_error_mps, "m/s"),
            mean(&summary.transition_attitude_error_deg, "deg"),
            mean(&summary.transition_event_time_error_s, "s"),
            mean(&summary.transition_mass_error_kg, "kg"),
            mean(&summary.transition_fuel_error_kg, "kg"),
        )
    };
    let actual = if summary.transition_observed == 0 {
        r#"<span class="muted">not observed</span>"#.to_owned()
    } else {
        format!(
            r#"<div class="overview-stack"><div class="overview-main">pass {}/{}</div><div class="overview-sub">heading {} · peak {}</div><div class="overview-sub">next candidates {}</div></div>"#,
            summary.transition_continuation_pass_runs,
            summary.transition_observed,
            mean(
                &summary.transition_continuation_outbound_heading_error_deg,
                "deg",
            ),
            mean(
                &summary.transition_continuation_required_accel_ratio_max,
                "x",
            ),
            mean(&summary.transition_continuation_passing_candidate_count, "",),
        )
    };
    let joint = if summary.joint_observed == 0 {
        r#"<span class="muted">not observed</span>"#.to_owned()
    } else {
        format!(
            r#"<div class="overview-stack"><div class="overview-main">pass {}/{} · eval {}</div><div class="overview-sub">passing {} · ttg {}</div><div class="overview-sub">heading {} · peak {} · saturated {}</div></div>"#,
            summary.joint_contract_pass_runs,
            summary.joint_observed,
            mean(&summary.joint_evaluated_candidate_count, ""),
            mean(&summary.joint_passing_candidate_count, ""),
            mean(&summary.joint_time_to_go_s, "s"),
            mean(
                &summary.joint_continuation_outbound_heading_error_deg,
                "deg",
            ),
            mean(&summary.joint_required_accel_ratio_max, "x"),
            mean(&summary.joint_total_saturated_time_s, "s"),
        )
    };
    format!(
        r#"<tr><td><code>{}</code></td><td><code>{}</code></td><td>{waypoint}</td><td>{planned}</td><td>{transition}</td><td>{actual}</td><td>{joint}</td></tr>"#,
        escape_html(&summary.key.route_angle),
        escape_html(&summary.key.vehicle_variant),
    )
}

pub(super) fn render_waypoint_continuation_seed_rows(records: &[&crate::BatchRunRecord]) -> String {
    let value = |value: Option<f64>, suffix: &str| {
        value
            .map(|value| format!("{value:.2}{suffix}"))
            .unwrap_or_else(|| "-".to_owned())
    };
    let status = |value: Option<bool>| match value {
        Some(true) => "pass",
        Some(false) => "fail",
        None => "-",
    };
    let mut rows = records
        .iter()
        .flat_map(|record| {
            record
                .review
                .waypoint_handoffs
                .iter()
                .filter(|handoff| {
                    handoff.continuation_next_waypoint_index.is_some()
                        || handoff.transition_next_waypoint_index.is_some()
                        || handoff.joint_next_waypoint_index.is_some()
                })
                .map(move |handoff| (*record, handoff))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|(lhs_record, lhs), (rhs_record, rhs)| {
        selector_sort_rank(&lhs_record.resolved.selector.route_angle)
            .cmp(&selector_sort_rank(
                &rhs_record.resolved.selector.route_angle,
            ))
            .then(
                selector_sort_rank(&lhs_record.resolved.selector.vehicle_variant).cmp(
                    &selector_sort_rank(&rhs_record.resolved.selector.vehicle_variant),
                ),
            )
            .then(lhs.waypoint_index.cmp(&rhs.waypoint_index))
            .then(
                lhs_record
                    .resolved
                    .resolved_seed
                    .cmp(&rhs_record.resolved.resolved_seed),
            )
    });
    rows.into_iter()
        .map(|(record, handoff)| {
            let planned = format!(
                r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">heading {} · peak {} · candidates {}</div></div>"#,
                status(handoff.continuation_contract_pass),
                value(
                    handoff.continuation_outbound_heading_error_rad.map(f64::to_degrees),
                    "deg",
                ),
                value(handoff.continuation_required_accel_ratio_max, "x"),
                handoff
                    .continuation_passing_candidate_count
                    .map_or_else(|| "-".to_owned(), |count| count.to_string()),
            );
            let transition = format!(
                r#"<div class="overview-stack"><div class="overview-main">pos {} · vel {}</div><div class="overview-sub">att {} · time {}</div><div class="overview-sub">mass {} · fuel {}</div></div>"#,
                value(handoff.transition_position_error_m, "m"),
                value(handoff.transition_velocity_error_mps, "m/s"),
                value(
                    handoff.transition_attitude_error_rad.map(f64::to_degrees),
                    "deg",
                ),
                value(handoff.transition_event_time_error_s, "s"),
                value(handoff.transition_mass_error_kg, "kg"),
                value(handoff.transition_fuel_error_kg, "kg"),
            );
            let actual = format!(
                r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">heading {} · peak {} · candidates {}</div></div>"#,
                status(handoff.transition_continuation_contract_pass),
                value(
                    handoff
                        .transition_continuation_outbound_heading_error_rad
                        .map(f64::to_degrees),
                    "deg",
                ),
                value(
                    handoff.transition_continuation_required_accel_ratio_max,
                    "x",
                ),
                handoff
                    .transition_continuation_passing_candidate_count
                    .map_or_else(|| "-".to_owned(), |count| count.to_string()),
            );
            let joint = format!(
                r#"<div class="overview-stack"><div class="overview-main">{} · eval {} · pass {}</div><div class="overview-sub">endpoint ({}, {}) · velocity ({}, {})</div><div class="overview-sub">ttg {} · heading {} · peak {} · saturated {}</div></div>"#,
                status(handoff.joint_contract_pass),
                handoff
                    .joint_evaluated_candidate_count
                    .map_or_else(|| "-".to_owned(), |count| count.to_string()),
                handoff
                    .joint_passing_candidate_count
                    .map_or_else(|| "-".to_owned(), |count| count.to_string()),
                value(handoff.joint_endpoint_x_m, "m"),
                value(handoff.joint_endpoint_y_m, "m"),
                value(handoff.joint_target_vx_mps, "m/s"),
                value(handoff.joint_target_vy_mps, "m/s"),
                value(handoff.joint_time_to_go_s, "s"),
                value(
                    handoff
                        .joint_continuation_outbound_heading_error_rad
                        .map(f64::to_degrees),
                    "deg",
                ),
                value(handoff.joint_required_accel_ratio_max, "x"),
                value(handoff.joint_total_saturated_time_s, "s"),
            );
            format!(
                r#"<tr><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td><code>#{}</code> {}</td><td>{planned}</td><td>{transition}</td><td>{actual}</td><td>{joint}</td></tr>"#,
                escape_html(&record.resolved.selector.route_angle),
                escape_html(&record.resolved.selector.vehicle_variant),
                record.resolved.resolved_seed,
                handoff.waypoint_index + 1,
                escape_html(handoff.waypoint_id.as_deref().unwrap_or("waypoint")),
            )
        })
        .collect()
}

pub(super) fn render_waypoint_sequence_state_debt(summary: &WaypointSequenceCellSummary) -> String {
    let Some(velocity_error) = summary.target_velocity_error_mps.as_ref() else {
        return r#"<span class="muted">-</span>"#.to_owned();
    };
    let deadline = summary
        .target_deadline_remaining_s
        .as_ref()
        .map(|value| format!("{:+.2}s", value.mean))
        .unwrap_or_else(|| "-".to_owned());
    let margin = summary
        .handoff_turn_margin_m
        .as_ref()
        .map(|value| format!("{:+.1}m", value.mean))
        .unwrap_or_else(|| "-".to_owned());
    let predicted_event = summary
        .predicted_handoff_time_to_go_s
        .as_ref()
        .map(|value| format!("{:.2}s", value.mean))
        .unwrap_or_else(|| "-".to_owned());
    let predicted_lead = summary
        .predicted_handoff_deadline_lead_s
        .as_ref()
        .map(|value| format!("{:+.2}s", value.mean))
        .unwrap_or_else(|| "-".to_owned());
    let prediction = if summary.predicted_contract_observed == 0 {
        "prediction unavailable".to_owned()
    } else {
        format!(
            "predict {}/{} pass · event {} · center lead {}",
            summary.predicted_contract_pass_runs,
            summary.predicted_contract_observed,
            predicted_event,
            predicted_lead,
        )
    };
    let candidate_history = if summary.candidate_contract_observed == 0 {
        "candidate history unavailable".to_owned()
    } else {
        let pass_window = summary
            .candidate_first_pass_time_s
            .as_ref()
            .zip(summary.candidate_last_pass_time_s.as_ref())
            .map(|(first, last)| format!(" · pass window {:.1}-{:.1}s", first.mean, last.mean))
            .unwrap_or_default();
        format!(
            "history {}/{} ever pass · lost {}{}",
            summary.candidate_contract_pass_ever_runs,
            summary.candidate_contract_observed,
            summary.candidate_pass_lost_runs,
            pass_window,
        )
    };
    let best_margins = match (
        summary.candidate_best_heading_margin_deg.as_ref(),
        summary.candidate_best_cross_speed_margin_mps.as_ref(),
    ) {
        (Some(heading), Some(cross)) => format!(
            "best margins heading {:+.1}deg · cross {:+.1}m/s",
            heading.mean, cross.mean
        ),
        (Some(heading), None) => format!("best heading margin {:+.1}deg", heading.mean),
        (None, Some(cross)) => format!("best cross margin {:+.1}m/s", cross.mean),
        (None, None) => "best margins unavailable".to_owned(),
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">Δv {:.1}m/s · deadline {}</div><div class="overview-sub">feasible {}/{} · handoff margin {}</div><div class="overview-sub">{}</div><div class="overview-sub">{}</div><div class="overview-sub">{}</div></div>"#,
        velocity_error.mean,
        escape_html(&deadline),
        summary.guidance_feasible_runs,
        summary.guidance_feasible_observed,
        escape_html(&margin),
        escape_html(&prediction),
        escape_html(&candidate_history),
        escape_html(&best_margins),
    )
}

pub(super) fn render_waypoint_triage_section(
    candidate: &BatchReport,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
) -> String {
    let candidate_records = preferred_current_lane_focus(candidate)
        .map(|focus| focus.records)
        .unwrap_or_else(|| candidate.records.iter().collect::<Vec<_>>());
    if !candidate_records
        .iter()
        .any(|record| record.review.waypoint_capture_status.is_some())
    {
        return String::new();
    }

    let mut rows = waypoint_cell_summaries(candidate_records.as_slice());
    rows.sort_by(compare_waypoint_cells);
    if rows.is_empty() {
        return String::new();
    }

    let row_html = rows
        .iter()
        .map(|summary| render_waypoint_triage_row(summary, output_dir, candidate_record_map))
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<details class="transfer-handoff-section">
  <summary class="section-head transfer-triage-summary">
    <h2>Waypoint Handoff Triage</h2>
    <div class="section-note">Current-lane waypoint cells, sorted by waypoint-contract warnings and scored failures. Handoff probe packs score the waypoint contract directly; final-landing packs keep route quality as diagnostics.</div>
  </summary>
  <div class="table-wrap">
    <table class="transfer-handoff-table">
      <thead>
        <tr>
          <th>Route</th>
          <th>Vehicle</th>
          <th>Profile</th>
          <th>Success</th>
          <th>Contract</th>
          <th>Terminal Recovery</th>
          <th>Waypoint</th>
          <th>Plan</th>
          <th>Closest</th>
          <th>Cross Track</th>
          <th>Heading Error</th>
          <th>Handoff Progress</th>
          <th>Handoff Cross</th>
          <th>Worst Seed</th>
        </tr>
      </thead>
      <tbody>{row_html}</tbody>
    </table>
  </div>
</details>"#
    )
}

pub(super) fn render_waypoint_triage_row(
    summary: &WaypointCellSummary<'_>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
) -> String {
    let route_html = format!(
        r#"<div class="overview-stack"><div class="overview-main"><code>{}</code></div><div class="overview-sub">{} · {}</div></div>"#,
        escape_html(&summary.key.route_angle),
        escape_html(&summary.key.condition_set),
        escape_html(&summary.key.radius_tier),
    );
    let vehicle_html = format!(
        r#"<code>{}</code>"#,
        escape_html(&summary.key.vehicle_variant)
    );
    let profile_html = render_waypoint_profile_cell(summary);
    let success_html = render_waypoint_success_cell(summary);
    let contract_html = render_waypoint_contract_cell(summary);
    let terminal_recovery_html = render_waypoint_terminal_recovery_cell(
        summary.final_terminal_recovery_observed,
        summary.final_terminal_recoverable_runs,
        summary.final_terminal_required_accel_ratio_max,
    );
    let waypoint_html = render_waypoint_capture_cell(summary);
    let plan_html = render_waypoint_plan_cell(
        summary.planned_progress_frac.as_ref(),
        summary.planned_signed_offset_ratio.as_ref(),
        summary.planned_signed_turn_deg.as_ref(),
        summary.planned_max_speed_mps.as_ref(),
        summary.continuation_stop_ratio_max,
    );
    let closest_html = render_transfer_handoff_metric_cell(
        summary.closest_distance_m.as_ref(),
        MetricDisplayKind::Meters,
        None,
    );
    let cross_track_html = render_transfer_handoff_metric_cell(
        summary.cross_track_m.as_ref(),
        MetricDisplayKind::Meters,
        None,
    );
    let heading_error_html = render_transfer_handoff_metric_cell(
        summary.heading_error_deg.as_ref(),
        MetricDisplayKind::Degrees,
        None,
    );
    let outbound_progress_html = render_transfer_handoff_metric_cell(
        summary.outbound_progress_mps.as_ref(),
        MetricDisplayKind::Speed,
        None,
    );
    let outbound_cross_html = render_transfer_handoff_metric_cell(
        summary.outbound_cross_speed_mps.as_ref(),
        MetricDisplayKind::Speed,
        None,
    );
    let worst_seed_html = render_waypoint_worst_seed(summary, output_dir, candidate_record_map);
    format!(
        r#"<tr>
  <td>{route}</td>
  <td>{vehicle}</td>
  <td>{profile}</td>
  <td>{success}</td>
  <td>{contract}</td>
  <td>{terminal_recovery}</td>
  <td>{waypoint}</td>
  <td>{plan}</td>
  <td>{closest}</td>
  <td>{cross_track}</td>
  <td>{heading_error}</td>
  <td>{outbound_progress}</td>
  <td>{outbound_cross}</td>
  <td>{worst_seed}</td>
</tr>"#,
        route = route_html,
        vehicle = vehicle_html,
        profile = profile_html,
        success = success_html,
        contract = contract_html,
        terminal_recovery = terminal_recovery_html,
        waypoint = waypoint_html,
        plan = plan_html,
        closest = closest_html,
        cross_track = cross_track_html,
        heading_error = heading_error_html,
        outbound_progress = outbound_progress_html,
        outbound_cross = outbound_cross_html,
        worst_seed = worst_seed_html,
    )
}

pub(super) fn waypoint_cell_summaries<'a>(
    records: &[&'a crate::BatchRunRecord],
) -> Vec<WaypointCellSummary<'a>> {
    let mut grouped = BTreeMap::<WaypointCellKey, Vec<&'a crate::BatchRunRecord>>::new();
    for &record in records {
        if record.review.waypoint_capture_status.is_none() {
            continue;
        }
        let selector = &record.resolved.selector;
        grouped
            .entry(WaypointCellKey {
                condition_set: selector.condition_set.clone(),
                vehicle_variant: selector.vehicle_variant.clone(),
                route_angle: selector_preferred_value(&selector.route_angle, &selector.arc_point),
                radius_tier: selector_preferred_value(
                    &selector.radius_tier,
                    &selector.velocity_band,
                ),
                waypoint_profile: selector_value_or_unspecified(&selector.waypoint_profile),
                waypoint_handoff_envelope: selector_value_or_unspecified(
                    &selector.waypoint_handoff_envelope,
                ),
            })
            .or_default()
            .push(record);
    }
    grouped
        .into_iter()
        .map(|(key, records)| waypoint_cell_summary(key, records.as_slice()))
        .collect()
}

pub(super) fn waypoint_cell_summary<'a>(
    key: WaypointCellKey,
    records: &[&'a crate::BatchRunRecord],
) -> WaypointCellSummary<'a> {
    let scored_runs = records
        .iter()
        .filter(|record| record.analytic.is_scored())
        .count();
    let success_runs = records
        .iter()
        .filter(|record| transfer_shape_record_success(record))
        .count();
    let contract_pass_runs = records
        .iter()
        .filter(|record| record.review.waypoint_contract_status.as_deref() == Some("pass"))
        .count();
    let spatial_miss_runs = records
        .iter()
        .filter(|record| {
            record.review.waypoint_contract_status.as_deref() == Some("spatial_miss")
                || (record.review.waypoint_contract_status.is_none()
                    && record.review.waypoint_capture_status.as_deref() == Some("missed"))
        })
        .count();
    let outbound_envelope_failure_runs = records
        .iter()
        .filter(|record| {
            matches!(
                record.review.waypoint_contract_status.as_deref(),
                Some("outbound_out_of_envelope" | "outbound_unviable")
            )
        })
        .count();
    let incomplete_runs = records
        .iter()
        .filter(|record| {
            record.review.waypoint_contract_status.as_deref() == Some("incomplete")
                || (record.review.waypoint_contract_status.is_none()
                    && matches!(
                        record.review.waypoint_capture_status.as_deref(),
                        Some("tracking" | "capture_window")
                    ))
        })
        .count();
    let unknown_contract_runs = records
        .iter()
        .filter(|record| record.review.waypoint_contract_status.as_deref() == Some("unknown"))
        .count();
    let captured_runs = records
        .iter()
        .filter(|record| record.review.waypoint_capture_status.as_deref() == Some("captured"))
        .count();
    let missed_runs = records
        .iter()
        .filter(|record| record.review.waypoint_capture_status.as_deref() == Some("missed"))
        .count();
    let tracking_runs = records
        .iter()
        .filter(|record| {
            matches!(
                record.review.waypoint_capture_status.as_deref(),
                Some("tracking" | "capture_window")
            )
        })
        .count();
    let worst_record = records
        .iter()
        .copied()
        .max_by(|lhs, rhs| compare_f64_asc(waypoint_record_score(lhs), waypoint_record_score(rhs)));
    WaypointCellSummary {
        key,
        total_runs: records.len(),
        scored_runs,
        success_runs,
        contract_pass_runs,
        spatial_miss_runs,
        outbound_envelope_failure_runs,
        incomplete_runs,
        unknown_contract_runs,
        captured_runs,
        missed_runs,
        tracking_runs,
        planned_progress_frac: waypoint_resolved_metric_summary(
            records,
            0,
            "profile_progress_frac",
        ),
        planned_signed_offset_ratio: waypoint_resolved_metric_summary(
            records,
            0,
            "route_signed_offset_ratio",
        ),
        planned_signed_turn_deg: waypoint_resolved_metric_summary(
            records,
            0,
            "signed_turn_angle_deg",
        ),
        planned_max_speed_mps: waypoint_resolved_metric_summary(records, 0, "max_speed_mps"),
        continuation_stop_ratio_max: waypoint_resolved_metric_max(
            records,
            0,
            "continuation_stop_ratio",
        ),
        final_terminal_recovery_observed: records
            .iter()
            .filter(|record| {
                waypoint_final_handoff(record)
                    .is_some_and(|handoff| handoff.final_terminal_recoverable.is_some())
            })
            .count(),
        final_terminal_recoverable_runs: records
            .iter()
            .filter(|record| {
                waypoint_final_handoff(record)
                    .and_then(|handoff| handoff.final_terminal_recoverable)
                    == Some(true)
            })
            .count(),
        final_terminal_required_accel_ratio_max: records
            .iter()
            .filter_map(|record| waypoint_final_handoff(record))
            .filter_map(|handoff| handoff.final_terminal_required_accel_ratio)
            .filter(|ratio| ratio.is_finite())
            .max_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(Ordering::Equal)),
        closest_distance_m: transfer_shape_metric_summary(records, |review| {
            review.waypoint_closest_distance_m
        }),
        cross_track_m: transfer_shape_metric_summary(records, |review| {
            review.waypoint_cross_track_m
        }),
        heading_error_deg: transfer_shape_metric_summary(records, |review| {
            review
                .waypoint_outbound_heading_error_rad
                .map(f64::to_degrees)
        }),
        outbound_progress_mps: transfer_shape_metric_summary(records, |review| {
            review.waypoint_outbound_progress_mps
        }),
        outbound_cross_speed_mps: transfer_shape_metric_summary(records, |review| {
            review.waypoint_outbound_cross_speed_mps
        }),
        worst_record,
    }
}

pub(super) fn compare_waypoint_cells(
    lhs: &WaypointCellSummary<'_>,
    rhs: &WaypointCellSummary<'_>,
) -> Ordering {
    rhs.missed_runs
        .cmp(&lhs.missed_runs)
        .then_with(|| {
            rhs.outbound_envelope_failure_runs
                .cmp(&lhs.outbound_envelope_failure_runs)
        })
        .then_with(|| rhs.tracking_runs.cmp(&lhs.tracking_runs))
        .then_with(|| rhs.unknown_contract_runs.cmp(&lhs.unknown_contract_runs))
        .then_with(|| {
            (rhs.scored_runs - rhs.success_runs).cmp(&(lhs.scored_runs - lhs.success_runs))
        })
        .then_with(|| lhs.key.cmp(&rhs.key))
}

pub(super) fn waypoint_record_score(record: &crate::BatchRunRecord) -> f64 {
    let mut score = 0.0;
    match record.review.waypoint_contract_status.as_deref() {
        Some("spatial_miss") => score += 12_000.0,
        Some("outbound_out_of_envelope" | "outbound_unviable") => score += 10_000.0,
        Some("incomplete") => score += 8_000.0,
        Some("unknown") => score += 6_000.0,
        Some("pass") => {}
        _ => match record.review.waypoint_capture_status.as_deref() {
            Some("missed") => score += 12_000.0,
            Some("tracking" | "capture_window") => score += 8_000.0,
            Some("captured") => {}
            _ => score += 1_000.0,
        },
    }
    if record.analytic.is_scored() && !transfer_shape_record_success(record) {
        score += 2_500.0;
    }
    score += record
        .review
        .waypoint_outbound_heading_error_rad
        .unwrap_or(0.0)
        * 100.0;
    score += record
        .review
        .waypoint_outbound_progress_mps
        .map(|progress_mps| (-progress_mps).max(0.0) * 10.0)
        .unwrap_or(0.0);
    score += record.review.waypoint_cross_track_m.unwrap_or(0.0);
    score += record.review.waypoint_closest_distance_m.unwrap_or(0.0) * 0.2;
    score
}

pub(super) fn render_waypoint_plan_cell(
    progress_frac: Option<&crate::BatchMetricSummary>,
    signed_offset_ratio: Option<&crate::BatchMetricSummary>,
    signed_turn_deg: Option<&crate::BatchMetricSummary>,
    max_speed_mps: Option<&crate::BatchMetricSummary>,
    continuation_stop_ratio_max: Option<f64>,
) -> String {
    let primary = [
        progress_frac.map(|value| format!("p {:.2}", value.mean)),
        signed_offset_ratio.map(|value| format!("n {:+.2}R", value.mean)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let detail = [
        signed_turn_deg.map(|value| format!("turn {:+.1}deg", value.mean)),
        max_speed_mps.map(|value| format!("vmax {:.1}m/s", value.mean)),
        continuation_stop_ratio_max.map(|value| format!("stop max {value:.2}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    if primary.is_empty() && detail.is_empty() {
        return r#"<span class="muted">-</span>"#.to_owned();
    }
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&primary.join(" · ")),
        escape_html(&detail.join(" · ")),
    )
}

pub(super) fn render_waypoint_terminal_recovery_cell(
    observed: usize,
    recoverable: usize,
    required_accel_ratio_max: Option<f64>,
) -> String {
    if observed == 0 {
        return r#"<span class="muted">-</span>"#.to_owned();
    }
    let ratio = required_accel_ratio_max
        .map(|value| format!("plan accel max {value:.2}x"))
        .unwrap_or_else(|| "plan accel unavailable".to_owned());
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{recoverable}/{observed} recoverable</div><div class="overview-sub">kinematic estimate · {}</div></div>"#,
        escape_html(&ratio),
    )
}

pub(super) fn render_waypoint_profile_cell(summary: &WaypointCellSummary<'_>) -> String {
    let profile = if summary.key.waypoint_profile.trim().is_empty() {
        UNSPECIFIED_SELECTOR_VALUE
    } else {
        summary.key.waypoint_profile.as_str()
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main"><code>{}</code></div><div class="overview-sub">{}</div></div>"#,
        escape_html(profile),
        escape_html(&summary.key.waypoint_handoff_envelope),
    )
}

pub(super) fn render_waypoint_success_cell(summary: &WaypointCellSummary<'_>) -> String {
    let main = format!("{}/{}", summary.success_runs, summary.scored_runs);
    let sub = if summary.scored_runs == 0 {
        format!("{} total", summary.total_runs)
    } else if summary.success_runs == summary.scored_runs && summary.contract_warning_runs() > 0 {
        "landed with waypoint warning".to_owned()
    } else {
        format!(
            "{:.1}% scored",
            percentage(summary.success_runs, summary.scored_runs)
        )
    };
    let class = if summary.success_runs < summary.scored_runs || summary.contract_warning_runs() > 0
    {
        "triage-risk"
    } else {
        ""
    };
    format!(
        r#"<div class="overview-stack {class}"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&main),
        escape_html(&sub),
    )
}

pub(super) fn render_waypoint_contract_cell(summary: &WaypointCellSummary<'_>) -> String {
    let class = if summary.contract_warning_runs() > 0 {
        "triage-risk"
    } else {
        ""
    };
    let sub = if summary.contract_warning_runs() == 0 {
        "all in envelope".to_owned()
    } else {
        waypoint_contract_warning_summary(summary)
    };
    format!(
        r#"<div class="overview-stack {class}"><div class="overview-main">{}/{}</div><div class="overview-sub">{}</div></div>"#,
        summary.contract_pass_runs,
        summary.total_runs,
        escape_html(&sub),
    )
}

pub(super) fn render_waypoint_capture_cell(summary: &WaypointCellSummary<'_>) -> String {
    let class = if summary.missed_runs > 0 || summary.tracking_runs > 0 {
        "triage-risk"
    } else {
        ""
    };
    let sub = if summary.missed_runs > 0 || summary.tracking_runs > 0 {
        format!(
            "{} contract warning · {} tracking",
            summary.missed_runs, summary.tracking_runs
        )
    } else {
        "all captured".to_owned()
    };
    format!(
        r#"<div class="overview-stack {class}"><div class="overview-main">{}/{}</div><div class="overview-sub">{}</div></div>"#,
        summary.captured_runs,
        summary.total_runs,
        escape_html(&sub),
    )
}

impl<'a> WaypointCellSummary<'a> {
    fn contract_warning_runs(&self) -> usize {
        self.spatial_miss_runs
            + self.outbound_envelope_failure_runs
            + self.incomplete_runs
            + self.unknown_contract_runs
    }
}

pub(super) fn waypoint_contract_warning_summary(summary: &WaypointCellSummary<'_>) -> String {
    let mut parts = Vec::new();
    if summary.spatial_miss_runs > 0 {
        parts.push(format!("{} spatial", summary.spatial_miss_runs));
    }
    if summary.outbound_envelope_failure_runs > 0 {
        parts.push(format!(
            "{} outbound envelope",
            summary.outbound_envelope_failure_runs
        ));
    }
    if summary.incomplete_runs > 0 {
        parts.push(format!("{} incomplete", summary.incomplete_runs));
    }
    if summary.unknown_contract_runs > 0 {
        parts.push(format!("{} unknown", summary.unknown_contract_runs));
    }
    parts.join(" · ")
}

pub(super) fn render_waypoint_worst_seed(
    summary: &WaypointCellSummary<'_>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
) -> String {
    let Some(record) = summary.worst_record else {
        return r#"<span class="muted">-</span>"#.to_owned();
    };
    let label = format!("seed {:04}", record.resolved.resolved_seed);
    let note = record
        .review
        .waypoint_contract_status
        .as_deref()
        .or(record.review.waypoint_capture_status.as_deref())
        .unwrap_or("waypoint");
    let Some(bundle_dir) = candidate_record_map
        .get(&record.resolved.run_id)
        .map(String::as_str)
        .or(record.bundle_dir.as_deref())
    else {
        return format!(
            r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
            escape_html(&label),
            escape_html(note),
        );
    };
    let bundle_dir = resolve_repo_relative(Path::new(bundle_dir));
    let href = best_bundle_href(&bundle_dir, output_dir);
    format!(
        r#"<div class="overview-stack"><div class="overview-main"><a href="{}">{}</a></div><div class="overview-sub">{}</div></div>"#,
        escape_html(&href),
        escape_html(&label),
        escape_html(note),
    )
}

pub(super) fn render_transfer_handoff_triage_section(
    candidate: &BatchReport,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
) -> String {
    let candidate_records = preferred_current_lane_focus(candidate)
        .map(|focus| focus.records)
        .unwrap_or_else(|| candidate.records.iter().collect::<Vec<_>>());
    if !candidate_records
        .iter()
        .any(|record| transfer_shape_record_has_transfer(record))
    {
        return String::new();
    }

    let mut rows = transfer_handoff_cell_summaries(candidate_records.as_slice());
    rows.sort_by(compare_transfer_handoff_cells);

    if rows.is_empty() {
        return r#"<details class="transfer-handoff-section">
  <summary class="section-head transfer-triage-summary">
    <h2>Transfer Handoff Triage</h2>
  </summary>
  <p class="muted">No current-lane transfer handoff diagnostics were available.</p>
</details>"#
            .to_owned();
    }

    let row_html = rows
        .iter()
        .map(|summary| {
            render_transfer_handoff_triage_row(summary, output_dir, candidate_record_map)
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<details class="transfer-handoff-section">
  <summary class="section-head transfer-triage-summary">
    <h2>Transfer Handoff Triage</h2>
    <div class="section-note">Current-lane transfer cells, sorted by failed/frontier status and handoff risk before visual shape.</div>
  </summary>
  <div class="table-wrap">
    <table class="transfer-handoff-table">
      <thead>
        <tr>
          <th>Route</th>
          <th>Vehicle</th>
          <th>Success</th>
          <th>Entry / Gate</th>
          <th>Handoff Height</th>
          <th>Handoff Speed</th>
          <th>Handoff pdx</th>
          <th>Handoff Angle</th>
          <th>Cutoff</th>
          <th>Cutoff pdx</th>
          <th>Terminal Rebound</th>
          <th>Worst Seed</th>
        </tr>
      </thead>
      <tbody>{row_html}</tbody>
    </table>
  </div>
</details>"#
    )
}

pub(super) fn render_transfer_handoff_triage_row(
    summary: &TransferHandoffCellSummary<'_>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
) -> String {
    let cell_id = transfer_shape_cell_id(&summary.key);
    let route_html = format!(
        r#"<div class="overview-stack"><div class="overview-main"><code>{}</code></div><div class="overview-sub">{} · {}</div></div>"#,
        escape_html(&summary.key.route_angle),
        escape_html(&summary.key.condition_set),
        escape_html(&summary.key.radius_tier),
    );
    let vehicle_html = format!(
        r#"<code>{}</code>"#,
        escape_html(&summary.key.vehicle_variant)
    );
    let success_html = render_transfer_handoff_success_cell(summary);
    let entry_gate_html = render_transfer_handoff_entry_gate(summary);
    let height_html = render_transfer_handoff_metric_cell(
        summary.handoff_height_m.as_ref(),
        MetricDisplayKind::Meters,
        handoff_height_class(summary.handoff_height_m.as_ref()),
    );
    let speed_html = render_transfer_handoff_metric_cell(
        summary.handoff_speed_mps.as_ref(),
        MetricDisplayKind::Speed,
        handoff_speed_class(summary.handoff_speed_mps.as_ref()),
    );
    let handoff_dx_html = render_transfer_handoff_metric_cell(
        summary.handoff_projected_dx_abs_m.as_ref(),
        MetricDisplayKind::Meters,
        projected_dx_class(summary.handoff_projected_dx_abs_m.as_ref()),
    );
    let handoff_angle_html = render_transfer_handoff_metric_cell(
        summary.handoff_impact_angle_deg.as_ref(),
        MetricDisplayKind::Degrees,
        impact_angle_class(summary.handoff_impact_angle_deg.as_ref()),
    );
    let cutoff_html = render_transfer_handoff_cutoff(summary);
    let cutoff_dx_html = render_transfer_handoff_metric_cell(
        summary.cutoff_projected_dx_abs_m.as_ref(),
        MetricDisplayKind::Meters,
        projected_dx_class(summary.cutoff_projected_dx_abs_m.as_ref()),
    );
    let terminal_rebound_html = render_transfer_terminal_rebound(summary);
    let worst_seed_html =
        render_transfer_handoff_worst_seed(summary, output_dir, candidate_record_map);

    format!(
        r#"<tr data-transfer-handoff-cell="{cell_id}">
  <td>{route}</td>
  <td>{vehicle}</td>
  <td>{success}</td>
  <td>{entry_gate}</td>
  <td>{height}</td>
  <td>{speed}</td>
  <td>{handoff_dx}</td>
  <td>{handoff_angle}</td>
  <td>{cutoff}</td>
  <td>{cutoff_dx}</td>
  <td>{terminal_rebound}</td>
  <td>{worst_seed}</td>
</tr>"#,
        cell_id = escape_html(&cell_id),
        route = route_html,
        vehicle = vehicle_html,
        success = success_html,
        entry_gate = entry_gate_html,
        height = height_html,
        speed = speed_html,
        handoff_dx = handoff_dx_html,
        handoff_angle = handoff_angle_html,
        cutoff = cutoff_html,
        cutoff_dx = cutoff_dx_html,
        terminal_rebound = terminal_rebound_html,
        worst_seed = worst_seed_html,
    )
}

pub(super) fn transfer_handoff_cell_summaries<'a>(
    records: &[&'a crate::BatchRunRecord],
) -> Vec<TransferHandoffCellSummary<'a>> {
    let mut grouped = BTreeMap::<TransferShapeCellKey, Vec<&'a crate::BatchRunRecord>>::new();
    for &record in records {
        let Some(key) = transfer_shape_cell_key(record) else {
            continue;
        };
        grouped.entry(key).or_default().push(record);
    }
    grouped
        .into_iter()
        .map(|(key, records)| summarize_transfer_handoff_cell(key, records.as_slice()))
        .collect()
}

pub(super) fn summarize_transfer_handoff_cell<'a>(
    key: TransferShapeCellKey,
    records: &[&'a crate::BatchRunRecord],
) -> TransferHandoffCellSummary<'a> {
    let success_runs = records
        .iter()
        .filter(|record| transfer_shape_record_success(record))
        .count();
    let scored_runs = records
        .iter()
        .filter(|record| record.analytic.is_scored())
        .count();
    let frontier_runs = records
        .iter()
        .filter(|record| {
            matches!(
                record.analytic.class,
                crate::BatchRunAnalyticClass::Frontier
            )
        })
        .count();
    let failed_scored_runs = records
        .iter()
        .filter(|record| record.analytic.is_scored() && !transfer_shape_record_success(record))
        .count();
    let worst_record = records.iter().copied().max_by(|lhs, rhs| {
        transfer_handoff_record_score(lhs)
            .partial_cmp(&transfer_handoff_record_score(rhs))
            .unwrap_or(Ordering::Equal)
    });
    let near_pad_rebounds = records
        .iter()
        .filter_map(|record| transfer_terminal_near_pad_rebound_gain_m(record))
        .collect::<Vec<_>>();

    TransferHandoffCellSummary {
        key,
        total_runs: records.len(),
        scored_runs,
        success_runs,
        frontier_runs,
        failed_scored_runs,
        terminal_entry_kind: dominant_transfer_shape_mode(records, |review| {
            review.transfer_terminal_entry_kind.as_deref()
        }),
        handoff_gate_mode: dominant_transfer_shape_mode(records, |review| {
            review.transfer_terminal_handoff_gate_mode.as_deref()
        }),
        handoff_height_m: transfer_shape_metric_summary(records, |review| {
            review.transfer_terminal_handoff_height_m
        }),
        handoff_speed_mps: transfer_shape_metric_summary(records, |review| {
            review.transfer_terminal_handoff_speed_mps
        }),
        handoff_projected_dx_abs_m: transfer_shape_metric_summary(records, |review| {
            review
                .transfer_terminal_handoff_projected_dx_m
                .map(f64::abs)
        }),
        handoff_impact_angle_deg: transfer_shape_metric_summary(records, |review| {
            review.transfer_terminal_handoff_impact_angle_deg
        }),
        cutoff_quality: dominant_transfer_shape_mode(records, |review| {
            review.transfer_boost_cutoff_quality.as_deref()
        }),
        cutoff_projected_dx_abs_m: transfer_shape_metric_summary(records, |review| {
            review.transfer_boost_cutoff_projected_dx_m.map(f64::abs)
        }),
        cutoff_impact_angle_deg: transfer_shape_metric_summary(records, |review| {
            review.transfer_boost_cutoff_impact_angle_deg
        }),
        post_handoff_apex_gain_m: transfer_shape_metric_summary(records, |review| {
            review.transfer_terminal_post_handoff_apex_gain_m
        }),
        low_altitude_rebound_gain_m: transfer_shape_metric_summary(records, |review| {
            review.transfer_terminal_low_altitude_rebound_gain_m
        }),
        low_altitude_rebound_origin_dx_abs_m: transfer_shape_metric_summary(records, |review| {
            review.transfer_terminal_low_altitude_rebound_origin_dx_abs_m
        }),
        near_pad_rebound_runs: near_pad_rebounds.len(),
        worst_near_pad_rebound_m: near_pad_rebounds.into_iter().fold(0.0, f64::max),
        worst_record,
    }
}

pub(super) fn compare_transfer_handoff_cells(
    lhs: &TransferHandoffCellSummary<'_>,
    rhs: &TransferHandoffCellSummary<'_>,
) -> Ordering {
    transfer_handoff_problem_rank(lhs)
        .cmp(&transfer_handoff_problem_rank(rhs))
        .then_with(|| rhs.near_pad_rebound_runs.cmp(&lhs.near_pad_rebound_runs))
        .then_with(|| compare_f64_desc(lhs.worst_near_pad_rebound_m, rhs.worst_near_pad_rebound_m))
        .then_with(|| {
            compare_f64_desc(
                metric_mean_or(lhs.post_handoff_apex_gain_m.as_ref(), 0.0),
                metric_mean_or(rhs.post_handoff_apex_gain_m.as_ref(), 0.0),
            )
        })
        .then_with(|| {
            compare_f64_asc(
                metric_mean_or(lhs.handoff_height_m.as_ref(), f64::INFINITY),
                metric_mean_or(rhs.handoff_height_m.as_ref(), f64::INFINITY),
            )
        })
        .then_with(|| {
            compare_f64_desc(
                metric_mean_or(lhs.handoff_speed_mps.as_ref(), 0.0),
                metric_mean_or(rhs.handoff_speed_mps.as_ref(), 0.0),
            )
        })
        .then_with(|| {
            compare_f64_desc(
                metric_mean_or(lhs.handoff_projected_dx_abs_m.as_ref(), 0.0),
                metric_mean_or(rhs.handoff_projected_dx_abs_m.as_ref(), 0.0),
            )
        })
        .then_with(|| {
            compare_f64_desc(
                metric_mean_or(lhs.cutoff_projected_dx_abs_m.as_ref(), 0.0),
                metric_mean_or(rhs.cutoff_projected_dx_abs_m.as_ref(), 0.0),
            )
        })
        .then_with(|| {
            selector_sort_rank(&lhs.key.condition_set)
                .cmp(&selector_sort_rank(&rhs.key.condition_set))
        })
        .then_with(|| {
            selector_sort_rank(&lhs.key.route_angle).cmp(&selector_sort_rank(&rhs.key.route_angle))
        })
        .then_with(|| {
            selector_sort_rank(&lhs.key.radius_tier).cmp(&selector_sort_rank(&rhs.key.radius_tier))
        })
        .then_with(|| {
            selector_sort_rank(&lhs.key.vehicle_variant)
                .cmp(&selector_sort_rank(&rhs.key.vehicle_variant))
        })
        .then_with(|| lhs.key.cmp(&rhs.key))
}

pub(super) fn transfer_handoff_problem_rank(summary: &TransferHandoffCellSummary<'_>) -> usize {
    if summary.failed_scored_runs > 0 || summary.frontier_runs > 0 {
        0
    } else {
        1
    }
}

pub(super) fn compare_f64_asc(lhs: f64, rhs: f64) -> Ordering {
    lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
}

pub(super) fn compare_f64_desc(lhs: f64, rhs: f64) -> Ordering {
    rhs.partial_cmp(&lhs).unwrap_or(Ordering::Equal)
}

pub(super) fn metric_mean_or(summary: Option<&crate::BatchMetricSummary>, fallback: f64) -> f64 {
    summary.map(|summary| summary.mean).unwrap_or(fallback)
}

pub(super) fn transfer_terminal_near_pad_rebound_gain_m(
    record: &crate::BatchRunRecord,
) -> Option<f64> {
    let gain_m = record
        .review
        .transfer_terminal_low_altitude_rebound_gain_m?;
    (record
        .review
        .transfer_terminal_low_altitude_rebound_near_pad
        == Some(true)
        && gain_m > TRANSFER_TERMINAL_REBOUND_RISK_GAIN_M)
        .then_some(gain_m)
}

pub(super) fn transfer_handoff_record_score(record: &crate::BatchRunRecord) -> f64 {
    let review = &record.review;
    let mut score = 0.0;
    if record.analytic.is_scored() && !transfer_shape_record_success(record) {
        score += 10_000.0;
    }
    if matches!(
        record.analytic.class,
        crate::BatchRunAnalyticClass::Frontier
    ) {
        score += 2_000.0;
    }
    if let Some(quality) = review.transfer_boost_cutoff_quality.as_deref()
        && quality != "pass"
    {
        score += 900.0;
    }
    if let Some(height_m) = review.transfer_terminal_handoff_height_m {
        score += (80.0 - height_m).max(0.0) * 10.0;
    }
    if let Some(speed_mps) = review.transfer_terminal_handoff_speed_mps {
        score += (speed_mps - 35.0).max(0.0) * 12.0;
    }
    if let Some(projected_dx_m) = review.transfer_terminal_handoff_projected_dx_m {
        score += projected_dx_m.abs();
    }
    if let Some(cutoff_projected_dx_m) = review.transfer_boost_cutoff_projected_dx_m {
        score += cutoff_projected_dx_m.abs() * 0.5;
    }
    if let Some(impact_angle_deg) = review.transfer_terminal_handoff_impact_angle_deg {
        score += (55.0 - impact_angle_deg).max(0.0) * 8.0;
    }
    if let Some(rebound_gain_m) = transfer_terminal_near_pad_rebound_gain_m(record) {
        score += 1_000.0 + rebound_gain_m * 20.0;
    }
    score
}

pub(super) fn render_transfer_terminal_rebound(summary: &TransferHandoffCellSummary<'_>) -> String {
    let gain = format_metric_mean(
        summary.low_altitude_rebound_gain_m.as_ref(),
        MetricDisplayKind::Meters,
    );
    let gain_stddev = format_metric_stddev(
        summary.low_altitude_rebound_gain_m.as_ref(),
        MetricDisplayKind::Meters,
    );
    let origin_dx = format_metric_mean(
        summary.low_altitude_rebound_origin_dx_abs_m.as_ref(),
        MetricDisplayKind::Meters,
    );
    let class = if summary.near_pad_rebound_runs > 0 {
        "triage-risk"
    } else {
        ""
    };
    let near_pad = if summary.near_pad_rebound_runs > 0 {
        format!(" · {} near-pad", summary.near_pad_rebound_runs)
    } else {
        String::new()
    };
    format!(
        r#"<div class="overview-stack {class}"><div class="overview-main">{}</div><div class="overview-sub">{} · origin dx {}{}</div></div>"#,
        escape_html(&gain),
        escape_html(&gain_stddev),
        escape_html(&origin_dx),
        escape_html(&near_pad),
    )
}

pub(super) fn render_transfer_handoff_success_cell(
    summary: &TransferHandoffCellSummary<'_>,
) -> String {
    let main = format!("{}/{}", summary.success_runs, summary.scored_runs);
    let sub = if summary.scored_runs == 0 {
        format!("{} total", summary.total_runs)
    } else if summary.failed_scored_runs > 0 || summary.frontier_runs > 0 {
        format!(
            "{} fail · {} frontier",
            summary.failed_scored_runs, summary.frontier_runs
        )
    } else {
        format!(
            "{:.1}% scored",
            percentage(summary.success_runs, summary.scored_runs)
        )
    };
    let class = if summary.failed_scored_runs > 0 {
        "triage-risk"
    } else if summary.frontier_runs > 0 {
        "triage-warn"
    } else {
        ""
    };
    format!(
        r#"<div class="overview-stack {class}"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&main),
        escape_html(&sub),
    )
}

pub(super) fn render_transfer_handoff_entry_gate(
    summary: &TransferHandoffCellSummary<'_>,
) -> String {
    let entry = summary.terminal_entry_kind.as_deref().unwrap_or("-");
    let gate = summary.handoff_gate_mode.as_deref().unwrap_or("-");
    let gate_label = if entry == "direct" {
        "terminal gate"
    } else {
        "handoff gate"
    };
    let class = if entry != "direct" && gate == "pending" {
        "triage-risk"
    } else {
        ""
    };
    format!(
        r#"<div class="overview-stack {class}"><div class="overview-main">terminal {}</div><div class="overview-sub">{} {}</div></div>"#,
        escape_html(entry),
        escape_html(gate_label),
        escape_html(gate),
    )
}

pub(super) fn render_transfer_handoff_metric_cell(
    summary: Option<&crate::BatchMetricSummary>,
    kind: MetricDisplayKind,
    class: Option<&'static str>,
) -> String {
    let class = class.unwrap_or("");
    format!(
        r#"<div class="overview-stack {class}"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&format_metric_mean(summary, kind)),
        escape_html(&format_metric_stddev(summary, kind)),
    )
}

pub(super) fn render_transfer_handoff_cutoff(summary: &TransferHandoffCellSummary<'_>) -> String {
    let quality = summary.cutoff_quality.as_deref().unwrap_or("-");
    let class = cutoff_quality_class(summary.cutoff_quality.as_deref()).unwrap_or("");
    let angle = format_metric_mean(
        summary.cutoff_impact_angle_deg.as_ref(),
        MetricDisplayKind::Degrees,
    );
    format!(
        r#"<div class="overview-stack {class}"><div class="overview-main">{}</div><div class="overview-sub">angle {}</div></div>"#,
        escape_html(quality),
        escape_html(&angle),
    )
}

pub(super) fn render_transfer_handoff_worst_seed(
    summary: &TransferHandoffCellSummary<'_>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
) -> String {
    let Some(record) = summary.worst_record else {
        return r#"<span class="muted">-</span>"#.to_owned();
    };
    let label = format!("seed {:04}", record.resolved.resolved_seed);
    let note = transfer_handoff_worst_seed_note(record);
    let Some(bundle_dir) = candidate_record_map
        .get(&record.resolved.run_id)
        .map(String::as_str)
        .or(record.bundle_dir.as_deref())
    else {
        return format!(
            r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
            escape_html(&label),
            escape_html(&note),
        );
    };
    let bundle_dir = resolve_repo_relative(Path::new(bundle_dir));
    let href = best_bundle_href(&bundle_dir, output_dir);
    format!(
        r#"<div class="overview-stack"><div class="overview-main"><a href="{}">{}</a></div><div class="overview-sub">{}</div></div>"#,
        escape_html(&href),
        escape_html(&label),
        escape_html(&note),
    )
}

pub(super) fn transfer_handoff_worst_seed_note(record: &crate::BatchRunRecord) -> String {
    if record.analytic.is_scored() && !transfer_shape_record_success(record) {
        return match record.analytic.class {
            crate::BatchRunAnalyticClass::Frontier => "frontier failure".to_owned(),
            _ => "scored failure".to_owned(),
        };
    }
    if let Some(rebound_gain_m) = transfer_terminal_near_pad_rebound_gain_m(record) {
        return format!("rebound {rebound_gain_m:.0}m near pad");
    }
    if let Some(rebound_gain_m) = record.review.transfer_terminal_low_altitude_rebound_gain_m
        && rebound_gain_m > TRANSFER_TERMINAL_REBOUND_RISK_GAIN_M
    {
        let origin_dx_m = record
            .review
            .transfer_terminal_low_altitude_rebound_origin_dx_abs_m
            .unwrap_or(0.0);
        return format!("recovery climb {rebound_gain_m:.0}m at dx {origin_dx_m:.0}m");
    }
    if let Some(rebound_gain_m) = record.review.transfer_terminal_low_altitude_rebound_gain_m
        && rebound_gain_m > 0.5
    {
        return format!("rebound {rebound_gain_m:.0}m");
    }
    if let Some(apex_gain_m) = record.review.transfer_terminal_post_handoff_apex_gain_m
        && apex_gain_m > 0.5
    {
        return format!("climb {apex_gain_m:.0}m");
    }
    if let Some(height_m) = record.review.transfer_terminal_handoff_height_m {
        return format!("height {height_m:.0}m");
    }
    if let Some(projected_dx_m) = record.review.transfer_terminal_handoff_projected_dx_m {
        return format!("handoff pdx {:.0}m", projected_dx_m.abs());
    }
    if let Some(shape_rmse_m) = record.review.transfer_shape_curve_rmse_m {
        return format!("shape {shape_rmse_m:.0}m");
    }
    "handoff diagnostics".to_owned()
}

pub(super) fn handoff_height_class(
    summary: Option<&crate::BatchMetricSummary>,
) -> Option<&'static str> {
    let mean = summary?.mean;
    if mean < 25.0 {
        Some("triage-risk")
    } else if mean < 80.0 {
        Some("triage-warn")
    } else {
        None
    }
}

pub(super) fn handoff_speed_class(
    summary: Option<&crate::BatchMetricSummary>,
) -> Option<&'static str> {
    let mean = summary?.mean;
    if mean > 50.0 {
        Some("triage-risk")
    } else if mean > 40.0 {
        Some("triage-warn")
    } else {
        None
    }
}

pub(super) fn projected_dx_class(
    summary: Option<&crate::BatchMetricSummary>,
) -> Option<&'static str> {
    let mean = summary?.mean;
    if mean > 120.0 {
        Some("triage-risk")
    } else if mean > 60.0 {
        Some("triage-warn")
    } else {
        None
    }
}

pub(super) fn impact_angle_class(
    summary: Option<&crate::BatchMetricSummary>,
) -> Option<&'static str> {
    let mean = summary?.mean;
    if mean < 40.0 {
        Some("triage-risk")
    } else if mean < 50.0 {
        Some("triage-warn")
    } else {
        None
    }
}

pub(super) fn cutoff_quality_class(quality: Option<&str>) -> Option<&'static str> {
    match quality {
        Some("pass") => None,
        Some(_) => Some("triage-risk"),
        None => None,
    }
}

pub(super) fn render_transfer_shape_triage_section(
    candidate: &BatchReport,
    baseline: Option<&BatchReport>,
    comparison: Option<&BatchComparison>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
) -> String {
    let candidate_records = preferred_current_lane_focus(candidate)
        .map(|focus| focus.records)
        .unwrap_or_else(|| candidate.records.iter().collect::<Vec<_>>());
    if !candidate_records
        .iter()
        .any(|record| transfer_shape_record_has_transfer(record))
    {
        return String::new();
    }

    let mut rows = transfer_shape_cell_summaries(candidate_records.as_slice())
        .into_iter()
        .filter(|summary| summary.shape_rmse_m.is_some())
        .collect::<Vec<_>>();
    rows.sort_by(|lhs, rhs| {
        rhs.worst_shape_rmse_m
            .partial_cmp(&lhs.worst_shape_rmse_m)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                selector_sort_rank(&lhs.key.condition_set)
                    .cmp(&selector_sort_rank(&rhs.key.condition_set))
            })
            .then_with(|| {
                selector_sort_rank(&lhs.key.route_angle)
                    .cmp(&selector_sort_rank(&rhs.key.route_angle))
            })
            .then_with(|| {
                selector_sort_rank(&lhs.key.radius_tier)
                    .cmp(&selector_sort_rank(&rhs.key.radius_tier))
            })
            .then_with(|| {
                selector_sort_rank(&lhs.key.vehicle_variant)
                    .cmp(&selector_sort_rank(&rhs.key.vehicle_variant))
            })
            .then_with(|| lhs.key.cmp(&rhs.key))
    });

    if rows.is_empty() {
        return r#"<details class="transfer-shape-section">
  <summary class="section-head transfer-triage-summary">
    <h2>Transfer Shape Triage</h2>
  </summary>
  <p class="muted">No successful current-lane transfer runs with shape metrics were available; use Transfer Handoff Triage and the Review Tree for failure-only cells.</p>
</details>"#
            .to_owned();
    }

    let baseline_records = if comparison.is_some() {
        baseline.map(|report| {
            preferred_current_lane_focus(report)
                .map(|focus| focus.records)
                .unwrap_or_else(|| report.records.iter().collect::<Vec<_>>())
        })
    } else {
        None
    };
    let baseline_cells = baseline_records.as_ref().map(|records| {
        transfer_shape_cell_summaries(records.as_slice())
            .into_iter()
            .filter(|summary| summary.shape_rmse_m.is_some())
            .map(|summary| (summary.key.clone(), summary))
            .collect::<BTreeMap<_, _>>()
    });
    let show_deltas = comparison.is_some()
        && baseline_cells
            .as_ref()
            .is_some_and(|cells| rows.iter().any(|summary| cells.contains_key(&summary.key)));

    let delta_headers = if show_deltas {
        r#"<th class="compare-toggle-target">Δ Shape</th>
          <th class="compare-toggle-target">Δ Success</th>"#
            .to_owned()
    } else {
        String::new()
    };
    let row_html = rows
        .iter()
        .map(|summary| {
            let baseline_summary = baseline_cells
                .as_ref()
                .and_then(|cells| cells.get(&summary.key));
            render_transfer_shape_triage_row(
                summary,
                baseline_summary,
                show_deltas,
                output_dir,
                candidate_record_map,
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<details class="transfer-shape-section">
  <summary class="section-head transfer-triage-summary">
    <h2>Transfer Shape Triage</h2>
    <div class="section-note">Visual-shape diagnostic sorted by worst successful RMSE. Use Transfer Handoff Triage first for controller tuning.</div>
  </summary>
  <div class="table-wrap">
    <table class="transfer-shape-table">
      <thead>
        <tr>
          <th>Route</th>
          <th>Vehicle</th>
          <th>Success</th>
          <th>Shape RMSE</th>
          <th>Apex Error</th>
          <th>Shortfall</th>
          <th>Projected dx max</th>
          <th>Handoff</th>
          <th>Boost Burn</th>
          <th>Modes</th>
          <th>Worst Seed</th>
          {delta_headers}
        </tr>
      </thead>
      <tbody>{row_html}</tbody>
    </table>
  </div>
</details>"#
    )
}

pub(super) fn render_transfer_shape_triage_row(
    summary: &TransferShapeCellSummary<'_>,
    baseline: Option<&TransferShapeCellSummary<'_>>,
    show_deltas: bool,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
) -> String {
    let cell_id = transfer_shape_cell_id(&summary.key);
    let route_html = format!(
        r#"<div class="overview-stack"><div class="overview-main"><code>{}</code></div><div class="overview-sub">{} · {}</div></div>"#,
        escape_html(&summary.key.route_angle),
        escape_html(&summary.key.condition_set),
        escape_html(&summary.key.radius_tier),
    );
    let vehicle_html = format!(
        r#"<code>{}</code>"#,
        escape_html(&summary.key.vehicle_variant)
    );
    let success_html = render_transfer_shape_success_cell(summary);
    let shape_html = escape_html(&format_metric_summary(
        summary.shape_rmse_m.as_ref(),
        MetricDisplayKind::Meters,
    ));
    let apex_html = escape_html(&format_metric_summary(
        summary.apex_error_m.as_ref(),
        MetricDisplayKind::Meters,
    ));
    let shortfall_html = escape_html(&format_metric_summary(
        summary.shortfall_pct.as_ref(),
        MetricDisplayKind::Percent,
    ));
    let projected_dx_html = escape_html(&format_metric_mean(
        summary.projected_dx_abs_max_m.as_ref(),
        MetricDisplayKind::Meters,
    ));
    let handoff_html = render_transfer_shape_handoff(summary);
    let boost_burn_html = escape_html(&format_metric_mean(
        summary.boost_burn_duration_s.as_ref(),
        MetricDisplayKind::Seconds,
    ));
    let modes_html = render_transfer_shape_modes(summary);
    let worst_seed_html =
        render_transfer_shape_worst_seed(summary, output_dir, candidate_record_map);
    let delta_cells = render_transfer_shape_delta_cells(summary, baseline, show_deltas);

    format!(
        r#"<tr data-transfer-shape-cell="{cell_id}">
  <td>{route}</td>
  <td>{vehicle}</td>
  <td>{success}</td>
  <td>{shape}</td>
  <td>{apex}</td>
  <td>{shortfall}</td>
  <td>{projected_dx}</td>
  <td>{handoff}</td>
  <td>{boost_burn}</td>
  <td>{modes}</td>
  <td>{worst_seed}</td>
  {delta_cells}
</tr>"#,
        cell_id = escape_html(&cell_id),
        route = route_html,
        vehicle = vehicle_html,
        success = success_html,
        shape = shape_html,
        apex = apex_html,
        shortfall = shortfall_html,
        projected_dx = projected_dx_html,
        handoff = handoff_html,
        boost_burn = boost_burn_html,
        modes = modes_html,
        worst_seed = worst_seed_html,
        delta_cells = delta_cells,
    )
}

pub(super) fn transfer_shape_cell_summaries<'a>(
    records: &[&'a crate::BatchRunRecord],
) -> Vec<TransferShapeCellSummary<'a>> {
    let mut grouped = BTreeMap::<TransferShapeCellKey, Vec<&'a crate::BatchRunRecord>>::new();
    for &record in records {
        let Some(key) = transfer_shape_cell_key(record) else {
            continue;
        };
        grouped.entry(key).or_default().push(record);
    }
    grouped
        .into_iter()
        .map(|(key, records)| summarize_transfer_shape_cell(key, records.as_slice()))
        .collect()
}

pub(super) fn summarize_transfer_shape_cell<'a>(
    key: TransferShapeCellKey,
    records: &[&'a crate::BatchRunRecord],
) -> TransferShapeCellSummary<'a> {
    let success_records = records
        .iter()
        .copied()
        .filter(|record| transfer_shape_record_success(record))
        .collect::<Vec<_>>();
    let mode_records = if success_records.is_empty() {
        records
    } else {
        success_records.as_slice()
    };
    let worst = success_records
        .iter()
        .filter_map(|record| {
            record
                .review
                .transfer_shape_curve_rmse_m
                .map(|value| (*record, value))
        })
        .max_by(|(_, lhs), (_, rhs)| lhs.partial_cmp(rhs).unwrap_or(Ordering::Equal));
    TransferShapeCellSummary {
        key,
        total_runs: records.len(),
        scored_runs: records
            .iter()
            .filter(|record| record.analytic.is_scored())
            .count(),
        success_runs: success_records.len(),
        shape_rmse_m: transfer_shape_metric_summary(success_records.as_slice(), |review| {
            review.transfer_shape_curve_rmse_m
        }),
        apex_error_m: transfer_shape_metric_summary(success_records.as_slice(), |review| {
            review.transfer_shape_apex_error_m
        }),
        shortfall_pct: transfer_shape_metric_summary(success_records.as_slice(), |review| {
            review
                .transfer_shape_shortfall_ratio
                .map(|value| value * 100.0)
        }),
        projected_dx_abs_max_m: transfer_shape_metric_summary(
            success_records.as_slice(),
            |review| review.transfer_shape_projected_dx_abs_max_m,
        ),
        handoff_time_s: transfer_shape_metric_summary(success_records.as_slice(), |review| {
            review.transfer_terminal_handoff_time_s
        }),
        handoff_gate_mode: dominant_transfer_shape_mode(success_records.as_slice(), |review| {
            review.transfer_terminal_handoff_gate_mode.as_deref()
        }),
        boost_burn_duration_s: transfer_shape_metric_summary(
            success_records.as_slice(),
            |review| review.transfer_boost_burn_duration_s,
        ),
        boost_quality: dominant_transfer_shape_mode(mode_records, |review| {
            review.transfer_boost_quality.as_deref()
        }),
        gate_mode: dominant_transfer_shape_mode(mode_records, |review| {
            review.transfer_terminal_gate_mode.as_deref()
        }),
        corridor_mode: dominant_transfer_shape_mode(mode_records, |review| {
            review.transfer_corridor_mode.as_deref()
        }),
        worst_shape_rmse_m: worst.map(|(_, value)| value).unwrap_or(0.0),
        worst_record: worst.map(|(record, _)| record),
    }
}

pub(super) fn transfer_shape_record_has_transfer(record: &crate::BatchRunRecord) -> bool {
    record.resolved.selector.mission == "transfer_guidance"
        || record.resolved.selector.route_angle != UNSPECIFIED_SELECTOR_VALUE
        || record.review.transfer_shape_curve_rmse_m.is_some()
        || record.review.transfer_final_phase.is_some()
}

pub(super) fn transfer_shape_record_success(record: &crate::BatchRunRecord) -> bool {
    record.analytic.is_scored()
        && matches!(
            record.manifest.mission_outcome,
            pd_core::MissionOutcome::Success
        )
}

pub(super) fn transfer_shape_cell_key(
    record: &crate::BatchRunRecord,
) -> Option<TransferShapeCellKey> {
    if !transfer_shape_record_has_transfer(record) {
        return None;
    }
    let selector = &record.resolved.selector;
    Some(TransferShapeCellKey {
        condition_set: selector_value_or_unspecified(&selector.condition_set),
        vehicle_variant: selector_value_or_unspecified(&selector.vehicle_variant),
        route_angle: selector_preferred_value(&selector.route_angle, &selector.arc_point),
        radius_tier: selector_preferred_value(&selector.radius_tier, &selector.velocity_band),
    })
}

pub(super) fn selector_value_or_unspecified(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        UNSPECIFIED_SELECTOR_VALUE.to_owned()
    } else {
        trimmed.to_owned()
    }
}

pub(super) fn selector_preferred_value(primary: &str, fallback: &str) -> String {
    let primary = primary.trim();
    if !primary.is_empty() && primary != UNSPECIFIED_SELECTOR_VALUE {
        primary.to_owned()
    } else {
        selector_value_or_unspecified(fallback)
    }
}

pub(super) fn transfer_shape_metric_summary<F>(
    records: &[&crate::BatchRunRecord],
    extractor: F,
) -> Option<crate::BatchMetricSummary>
where
    F: Fn(&crate::BatchRunReviewMetrics) -> Option<f64>,
{
    let values = records
        .iter()
        .filter_map(|record| extractor(&record.review))
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    crate::metric_summary(&values)
}

pub(super) fn transfer_shape_record_metric_summary<F>(
    records: &[&crate::BatchRunRecord],
    extractor: F,
) -> Option<crate::BatchMetricSummary>
where
    F: Fn(&crate::BatchRunRecord) -> Option<f64>,
{
    let values = records
        .iter()
        .filter_map(|record| extractor(record))
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    crate::metric_summary(&values)
}

pub(super) fn dominant_transfer_shape_mode<F>(
    records: &[&crate::BatchRunRecord],
    extractor: F,
) -> Option<String>
where
    F: Fn(&crate::BatchRunReviewMetrics) -> Option<&str>,
{
    let mut counts = BTreeMap::<String, usize>::new();
    for record in records {
        let Some(value) = extractor(&record.review) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        *counts.entry(value.to_owned()).or_insert(0) += 1;
    }
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|(lhs_value, lhs_count), (rhs_value, rhs_count)| {
        rhs_count
            .cmp(lhs_count)
            .then_with(|| lhs_value.cmp(rhs_value))
    });
    ranked.into_iter().next().map(|(value, _)| value)
}

pub(super) fn render_transfer_shape_success_cell(summary: &TransferShapeCellSummary<'_>) -> String {
    let scored_runs = summary.scored_runs;
    let main = format!("{}/{}", summary.success_runs, scored_runs);
    let sub = if scored_runs == 0 {
        format!("{} total", summary.total_runs)
    } else {
        format!(
            "{:.1}% scored",
            percentage(summary.success_runs, scored_runs)
        )
    };
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">{}</div></div>"#,
        escape_html(&main),
        escape_html(&sub),
    )
}

pub(super) fn render_transfer_shape_handoff(summary: &TransferShapeCellSummary<'_>) -> String {
    let time = format_metric_mean(summary.handoff_time_s.as_ref(), MetricDisplayKind::Seconds);
    let gate = summary.handoff_gate_mode.as_deref().unwrap_or("-");
    format!(
        r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">gate {}</div></div>"#,
        escape_html(&time),
        escape_html(gate),
    )
}

pub(super) fn render_transfer_shape_modes(summary: &TransferShapeCellSummary<'_>) -> String {
    let boost = summary.boost_quality.as_deref().unwrap_or("-");
    let handoff_gate = summary.handoff_gate_mode.as_deref().unwrap_or("-");
    let final_gate = summary.gate_mode.as_deref().unwrap_or("-");
    let corridor = summary.corridor_mode.as_deref().unwrap_or("-");
    format!(
        r#"<div class="overview-stack"><div class="overview-main">boost {}</div><div class="overview-sub">handoff gate {} · final gate {} · corridor {}</div></div>"#,
        escape_html(boost),
        escape_html(handoff_gate),
        escape_html(final_gate),
        escape_html(corridor),
    )
}

pub(super) fn render_transfer_shape_worst_seed(
    summary: &TransferShapeCellSummary<'_>,
    output_dir: &Path,
    candidate_record_map: &BTreeMap<String, String>,
) -> String {
    let Some(record) = summary.worst_record else {
        return r#"<span class="muted">-</span>"#.to_owned();
    };
    let label = format!("seed {:04}", record.resolved.resolved_seed);
    let shape = format!("{:.2}m", summary.worst_shape_rmse_m);
    let Some(bundle_dir) = candidate_record_map
        .get(&record.resolved.run_id)
        .map(String::as_str)
        .or(record.bundle_dir.as_deref())
    else {
        return format!(
            r#"<div class="overview-stack"><div class="overview-main">{}</div><div class="overview-sub">shape {}</div></div>"#,
            escape_html(&label),
            escape_html(&shape),
        );
    };
    let bundle_dir = resolve_repo_relative(Path::new(bundle_dir));
    let href = best_bundle_href(&bundle_dir, output_dir);
    format!(
        r#"<div class="overview-stack"><div class="overview-main"><a href="{}">{}</a></div><div class="overview-sub">shape {}</div></div>"#,
        escape_html(&href),
        escape_html(&label),
        escape_html(&shape),
    )
}

pub(super) fn render_transfer_shape_delta_cells(
    summary: &TransferShapeCellSummary<'_>,
    baseline: Option<&TransferShapeCellSummary<'_>>,
    show_deltas: bool,
) -> String {
    if !show_deltas {
        return String::new();
    }
    let shape_delta = metric_delta_value(
        summary.shape_rmse_m.as_ref(),
        baseline.and_then(|summary| summary.shape_rmse_m.as_ref()),
    );
    let shape_html = shape_delta
        .map(|delta| {
            format!(
                r#"<span class="{}">{}</span>"#,
                delta_class(delta),
                escape_html(&format_metric_delta_value(delta, MetricDisplayKind::Meters))
            )
        })
        .unwrap_or_else(|| r#"<span class="muted">-</span>"#.to_owned());
    let success_delta = baseline.map(|baseline| {
        success_rate_ratio(summary.success_runs, summary.scored_runs)
            - success_rate_ratio(baseline.success_runs, baseline.scored_runs)
    });
    let success_html = success_delta
        .map(|delta| {
            format!(
                r#"<span class="{}">{}</span>"#,
                delta_class(-delta),
                escape_html(&format_percent_delta(delta))
            )
        })
        .unwrap_or_else(|| r#"<span class="muted">-</span>"#.to_owned());
    format!(
        r#"<td class="compare-toggle-target">{shape_html}</td>
  <td class="compare-toggle-target">{success_html}</td>"#
    )
}

pub(super) fn transfer_shape_cell_id(key: &TransferShapeCellKey) -> String {
    [
        key.condition_set.as_str(),
        key.vehicle_variant.as_str(),
        key.route_angle.as_str(),
        key.radius_tier.as_str(),
    ]
    .join("|")
}
