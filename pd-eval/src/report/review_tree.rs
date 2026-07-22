use super::*;

#[derive(Clone, Debug)]
pub(super) struct ReviewAggregate {
    pub(super) total_runs: usize,
    pub(super) success_runs: usize,
    pub(super) failure_runs: usize,
    pub(super) invalidated_runs: usize,
    pub(super) sim_time_stats: Option<crate::BatchMetricSummary>,
    pub(super) fuel_used_pct_of_max: Option<crate::BatchMetricSummary>,
    pub(super) landing_offset_abs_m: Option<crate::BatchMetricSummary>,
    pub(super) low_altitude_dwell_s: Option<crate::BatchMetricSummary>,
    pub(super) low_altitude_unsafe_recovery_s: Option<crate::BatchMetricSummary>,
    pub(super) reference_gap_mean_m: Option<crate::BatchMetricSummary>,
    pub(super) transfer_shape_curve_rmse_m: Option<crate::BatchMetricSummary>,
    pub(super) failed_seeds: Vec<u64>,
}

pub(super) fn aggregate_ref_dev_metric(
    aggregate: &ReviewAggregate,
) -> Option<&crate::BatchMetricSummary> {
    aggregate
        .transfer_shape_curve_rmse_m
        .as_ref()
        .or(aggregate.reference_gap_mean_m.as_ref())
}

pub(super) fn aggregate_ref_dev_label(aggregate: &ReviewAggregate) -> &'static str {
    if aggregate.transfer_shape_curve_rmse_m.is_some() {
        "shape"
    } else {
        "ref"
    }
}

pub(super) type LaneRecordGroups<'a> = BTreeMap<String, Vec<&'a crate::BatchRunRecord>>;
pub(super) type VelocityRecordGroups<'a> = BTreeMap<String, LaneRecordGroups<'a>>;
pub(super) type ArcRecordGroups<'a> = BTreeMap<String, VelocityRecordGroups<'a>>;
pub(super) type VehicleRecordGroups<'a> = BTreeMap<String, ArcRecordGroups<'a>>;
pub(super) type ConditionRecordGroups<'a> = BTreeMap<String, VehicleRecordGroups<'a>>;
pub(super) type ArrivalRecordGroups<'a> = BTreeMap<String, ConditionRecordGroups<'a>>;
pub(super) type MissionRecordGroups<'a> = BTreeMap<String, ArrivalRecordGroups<'a>>;
pub(super) type VehicleLaneRecordGroups<'a> = BTreeMap<String, LaneRecordGroups<'a>>;
pub(super) type VelocityVehicleRecordGroups<'a> = BTreeMap<String, VehicleLaneRecordGroups<'a>>;
pub(super) type ArcVelocityVehicleRecordGroups<'a> =
    BTreeMap<String, VelocityVehicleRecordGroups<'a>>;
pub(super) type WaypointProfileRecordGroups<'a> = BTreeMap<String, Vec<&'a crate::BatchRunRecord>>;

#[derive(Clone, Copy)]
pub(super) struct ConditionReviewPath<'a> {
    mission: &'a str,
    arrival_family: &'a str,
    condition_set: &'a str,
}

#[derive(Clone, Copy)]
pub(super) struct MatrixReviewPath<'a> {
    condition: ConditionReviewPath<'a>,
    waypoint_profile: Option<&'a str>,
    vehicle_variant: Option<&'a str>,
    arc_point: Option<&'a str>,
    velocity_band: Option<&'a str>,
}

impl<'a> MatrixReviewPath<'a> {
    fn new(condition: ConditionReviewPath<'a>) -> Self {
        Self {
            condition,
            waypoint_profile: None,
            vehicle_variant: None,
            arc_point: None,
            velocity_band: None,
        }
    }

    fn with_waypoint_profile(self, waypoint_profile: Option<&'a str>) -> Self {
        Self {
            waypoint_profile,
            ..self
        }
    }

    fn with_vehicle(self, vehicle_variant: &'a str) -> Self {
        Self {
            vehicle_variant: Some(vehicle_variant),
            ..self
        }
    }

    fn with_arc(self, arc_point: Option<&'a str>) -> Self {
        Self { arc_point, ..self }
    }

    fn with_velocity(self, velocity_band: Option<&'a str>) -> Self {
        Self {
            velocity_band,
            ..self
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ReviewTreeRenderContext<'a> {
    run_change_map: &'a BTreeMap<String, (&'static str, &'static str)>,
    comparison: Option<&'a BatchComparison>,
    output_dir: &'a Path,
    render_cache: &'a BatchReportRenderCache,
}

pub(super) fn render_review_tree(
    candidate: &BatchReport,
    baseline: Option<&BatchReport>,
    comparison: Option<&BatchComparison>,
    output_dir: &Path,
    render_cache: &BatchReportRenderCache,
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
    let context = ReviewTreeRenderContext {
        run_change_map: &run_change_map,
        comparison,
        output_dir,
        render_cache,
    };

    let mission_keys = merged_map_keys(&candidate_tree, Some(&baseline_tree));
    let sections = mission_keys
        .iter()
        .map(|mission| {
            render_mission_review_section(
                mission,
                candidate_tree.get(mission),
                baseline_tree.get(mission),
                &context,
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(r#"<div class="tree-stack">{sections}</div>"#)
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum TreeRowTone {
    Current,
    Baseline,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum SummaryMetricStyle {
    MeanStddev,
    MeanDelta,
}

pub(super) fn render_mission_review_section(
    mission: &str,
    candidate_arrivals: Option<&ArrivalRecordGroups<'_>>,
    baseline_arrivals: Option<&ArrivalRecordGroups<'_>>,
    context: &ReviewTreeRenderContext<'_>,
) -> String {
    let ReviewTreeRenderContext { comparison, .. } = *context;
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
                context,
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
    rows.push_str(&render_summary_row(SummaryRow {
        label: mission,
        depth: 0,
        parent_group_id: None,
        group_id: (!arrival_keys.is_empty()).then_some(group_id.as_str()),
        kind: "mission",
        aggregate: current_row_aggregate,
        secondary_aggregate: baseline_row_aggregate,
        metric_style: SummaryMetricStyle::MeanDelta,
        changed,
        note_html: current_note.as_str(),
        tone: TreeRowTone::Current,
    }));
    rows.push_str(&arrival_rows);

    format!(
        r#"<section class="tree-table-section">
  <div class="table-heading">
    <h3><code>{mission}</code></h3>
    <div class="section-meta">{success_rate} · {failure_count} fail · {fuel_used} fuel · {mean_sim} flight · {landing_offset} off · {reference_gap} ref · {low_unsafe} low unsafe</div>
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
          <th>Reference deviation</th>
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
            aggregate.as_ref().and_then(aggregate_ref_dev_metric),
            MetricDisplayKind::Meters
        ),
        low_unsafe = format_metric_summary(
            aggregate
                .as_ref()
                .and_then(|item| item.low_altitude_unsafe_recovery_s.as_ref()),
            MetricDisplayKind::Seconds
        ),
        rows = rows,
    )
}

pub(super) fn render_arrival_review_section(
    mission: &str,
    arrival_family: &str,
    candidate_conditions: Option<&ConditionRecordGroups<'_>>,
    baseline_conditions: Option<&ConditionRecordGroups<'_>>,
    context: &ReviewTreeRenderContext<'_>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let ReviewTreeRenderContext { comparison, .. } = *context;
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
                ConditionReviewPath {
                    mission,
                    arrival_family,
                    condition_set,
                },
                candidate_conditions.get(condition_set),
                baseline_conditions.get(condition_set),
                context,
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
    rows.push_str(&render_summary_row(SummaryRow {
        label: arrival_family,
        depth,
        parent_group_id,
        group_id: (!condition_keys.is_empty()).then_some(group_id.as_str()),
        kind: "arrival",
        aggregate: current_row_aggregate,
        secondary_aggregate: baseline_row_aggregate,
        metric_style: SummaryMetricStyle::MeanDelta,
        changed,
        note_html: current_note.as_str(),
        tone: TreeRowTone::Current,
    }));
    rows.push_str(&condition_rows);
    rows
}

pub(super) fn render_condition_review_section(
    path: ConditionReviewPath<'_>,
    candidate_vehicles: Option<&VehicleRecordGroups<'_>>,
    baseline_vehicles: Option<&VehicleRecordGroups<'_>>,
    context: &ReviewTreeRenderContext<'_>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let ConditionReviewPath {
        mission,
        arrival_family,
        condition_set,
    } = path;
    let ReviewTreeRenderContext { comparison, .. } = *context;
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
    let candidate_arcs = records_by_arc_velocity_vehicle_from_records(candidate_records.as_slice());
    let baseline_arcs = records_by_arc_velocity_vehicle_from_records(baseline_records.as_slice());
    let arc_keys = merged_map_keys(&candidate_arcs, Some(&baseline_arcs));
    let render_matrix_axes = has_meaningful_selector_keys(arc_keys.as_slice());
    let candidate_profiles = records_by_waypoint_profile(candidate_records.as_slice());
    let baseline_profiles = records_by_waypoint_profile(baseline_records.as_slice());
    let profile_keys = merged_map_keys(&candidate_profiles, Some(&baseline_profiles));
    let render_waypoint_profiles = mission == "transfer_guidance"
        && profile_keys
            .iter()
            .filter(|profile| profile.as_str() != UNSPECIFIED_SELECTOR_VALUE)
            .count()
            > 1;
    let group_id = tree_group_id(&["condition", mission, arrival_family, condition_set]);
    let child_rows = if render_waypoint_profiles {
        profile_keys
            .iter()
            .map(|waypoint_profile| {
                render_waypoint_profile_review_section(
                    MatrixReviewPath::new(path).with_waypoint_profile(Some(waypoint_profile)),
                    candidate_profiles.get(waypoint_profile),
                    baseline_profiles.get(waypoint_profile),
                    context,
                    depth + 1,
                    Some(group_id.as_str()),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    } else if render_matrix_axes {
        arc_keys
            .iter()
            .map(|arc_point| {
                render_condition_arc_review_section(
                    MatrixReviewPath::new(path).with_arc(Some(arc_point)),
                    candidate_arcs.get(arc_point),
                    baseline_arcs.get(arc_point),
                    context,
                    depth + 1,
                    Some(group_id.as_str()),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        vehicle_keys
            .iter()
            .map(|vehicle_variant| {
                render_vehicle_review_section(
                    MatrixReviewPath::new(path).with_vehicle(vehicle_variant),
                    candidate_vehicles.get(vehicle_variant),
                    baseline_vehicles.get(vehicle_variant),
                    context,
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
    rows.push_str(&render_summary_row(SummaryRow {
        label: condition_set,
        depth,
        parent_group_id,
        group_id: (!child_rows.is_empty()).then_some(group_id.as_str()),
        kind: "condition",
        aggregate: current_row_aggregate,
        secondary_aggregate: baseline_row_aggregate,
        metric_style: SummaryMetricStyle::MeanDelta,
        changed,
        note_html: current_note.as_str(),
        tone: TreeRowTone::Current,
    }));
    rows.push_str(&child_rows);
    rows
}

pub(super) fn render_waypoint_profile_review_section(
    path: MatrixReviewPath<'_>,
    candidate_records: Option<&Vec<&crate::BatchRunRecord>>,
    baseline_records: Option<&Vec<&crate::BatchRunRecord>>,
    context: &ReviewTreeRenderContext<'_>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let ConditionReviewPath {
        mission,
        arrival_family,
        condition_set,
    } = path.condition;
    let waypoint_profile = path
        .waypoint_profile
        .expect("waypoint profile rows require a profile");
    let ReviewTreeRenderContext { comparison, .. } = *context;
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
            && row.selector.waypoint_profile == waypoint_profile
    });
    let changed =
        comparison.is_some() && aggregate_changed(aggregate.as_ref(), baseline_aggregate.as_ref());
    let candidate_arcs = records_by_arc_velocity_vehicle_from_records(candidate_records.as_slice());
    let baseline_arcs = records_by_arc_velocity_vehicle_from_records(baseline_records.as_slice());
    let arc_keys = merged_map_keys(&candidate_arcs, Some(&baseline_arcs));
    let render_matrix_axes = has_meaningful_selector_keys(arc_keys.as_slice());
    let group_id = tree_group_id(&[
        "waypoint-profile",
        mission,
        arrival_family,
        condition_set,
        waypoint_profile,
    ]);
    let child_rows = if render_matrix_axes {
        arc_keys
            .iter()
            .map(|arc_point| {
                render_condition_arc_review_section(
                    path.with_arc(Some(arc_point)),
                    candidate_arcs.get(arc_point),
                    baseline_arcs.get(arc_point),
                    context,
                    depth + 1,
                    Some(group_id.as_str()),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        let candidate_vehicles = records_by_vehicle_lane_from_records(candidate_records.as_slice());
        let baseline_vehicles = records_by_vehicle_lane_from_records(baseline_records.as_slice());
        merged_map_keys(&candidate_vehicles, Some(&baseline_vehicles))
            .iter()
            .map(|vehicle_variant| {
                render_deep_vehicle_review_section(
                    path.with_vehicle(vehicle_variant),
                    candidate_vehicles.get(vehicle_variant),
                    baseline_vehicles.get(vehicle_variant),
                    context,
                    depth + 1,
                    Some(group_id.as_str()),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    let current_note = render_summary_note(
        comparison.is_some(),
        TreeRowTone::Current,
        comparison.is_some().then_some(regression_count),
        None,
        aggregate.is_some(),
        baseline_aggregate.is_some(),
    );
    let mut rows = render_summary_row(SummaryRow {
        label: waypoint_profile,
        depth,
        parent_group_id,
        group_id: (!child_rows.is_empty()).then_some(group_id.as_str()),
        kind: "waypoint profile",
        aggregate: aggregate.as_ref(),
        secondary_aggregate: baseline_aggregate.as_ref(),
        metric_style: SummaryMetricStyle::MeanDelta,
        changed,
        note_html: current_note.as_str(),
        tone: TreeRowTone::Current,
    });
    rows.push_str(&child_rows);
    rows
}

pub(super) fn render_condition_arc_review_section(
    path: MatrixReviewPath<'_>,
    candidate_velocities: Option<&VelocityVehicleRecordGroups<'_>>,
    baseline_velocities: Option<&VelocityVehicleRecordGroups<'_>>,
    context: &ReviewTreeRenderContext<'_>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let ConditionReviewPath {
        mission,
        arrival_family,
        condition_set,
    } = path.condition;
    let waypoint_profile = path.waypoint_profile;
    let arc_point = path.arc_point.expect("arc rows require an arc point");
    let ReviewTreeRenderContext { comparison, .. } = *context;
    let candidate_records = flatten_velocity_vehicle_records(candidate_velocities);
    let baseline_records = flatten_velocity_vehicle_records(baseline_velocities);
    let aggregate =
        (!candidate_records.is_empty()).then(|| review_aggregate_from_records(&candidate_records));
    let baseline_aggregate =
        (!baseline_records.is_empty()).then(|| review_aggregate_from_records(&baseline_records));
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
            && waypoint_profile
                .map(|value| row.selector.waypoint_profile == value)
                .unwrap_or(true)
            && row.selector.arc_point == arc_point
    });
    let changed =
        comparison.is_some() && aggregate_changed(current_row_aggregate, baseline_row_aggregate);
    let empty_candidate = VelocityVehicleRecordGroups::new();
    let empty_baseline = VelocityVehicleRecordGroups::new();
    let candidate_velocities = candidate_velocities.unwrap_or(&empty_candidate);
    let baseline_velocities = baseline_velocities.unwrap_or(&empty_baseline);
    let velocity_keys = merged_map_keys(candidate_velocities, Some(baseline_velocities));
    let render_velocity_axes = has_meaningful_selector_keys(velocity_keys.as_slice());
    let mut group_parts = vec!["arc", mission, arrival_family, condition_set];
    if let Some(waypoint_profile) = waypoint_profile {
        group_parts.push(waypoint_profile);
    }
    group_parts.push(arc_point);
    let group_id = tree_group_id(group_parts.as_slice());
    let child_rows = if render_velocity_axes {
        velocity_keys
            .iter()
            .map(|velocity_band| {
                render_condition_velocity_review_section(
                    path.with_velocity(Some(velocity_band)),
                    candidate_velocities.get(velocity_band),
                    baseline_velocities.get(velocity_band),
                    context,
                    depth + 1,
                    Some(group_id.as_str()),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        let empty_candidate_vehicles = VehicleLaneRecordGroups::new();
        let empty_baseline_vehicles = VehicleLaneRecordGroups::new();
        let candidate_vehicles = candidate_velocities
            .get(UNSPECIFIED_SELECTOR_VALUE)
            .unwrap_or(&empty_candidate_vehicles);
        let baseline_vehicles = baseline_velocities
            .get(UNSPECIFIED_SELECTOR_VALUE)
            .unwrap_or(&empty_baseline_vehicles);
        let vehicle_keys = merged_map_keys(candidate_vehicles, Some(baseline_vehicles));
        vehicle_keys
            .iter()
            .map(|vehicle_variant| {
                render_deep_vehicle_review_section(
                    path.with_vehicle(vehicle_variant),
                    candidate_vehicles.get(vehicle_variant),
                    baseline_vehicles.get(vehicle_variant),
                    context,
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
    rows.push_str(&render_summary_row(SummaryRow {
        label: arc_point,
        depth,
        parent_group_id,
        group_id: (!child_rows.is_empty()).then_some(group_id.as_str()),
        kind: matrix_route_level_kind(mission),
        aggregate: current_row_aggregate,
        secondary_aggregate: baseline_row_aggregate,
        metric_style: SummaryMetricStyle::MeanDelta,
        changed,
        note_html: current_note.as_str(),
        tone: TreeRowTone::Current,
    }));
    rows.push_str(&child_rows);
    rows
}

pub(super) fn render_condition_velocity_review_section(
    path: MatrixReviewPath<'_>,
    candidate_vehicles: Option<&VehicleLaneRecordGroups<'_>>,
    baseline_vehicles: Option<&VehicleLaneRecordGroups<'_>>,
    context: &ReviewTreeRenderContext<'_>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let ConditionReviewPath {
        mission,
        arrival_family,
        condition_set,
    } = path.condition;
    let waypoint_profile = path.waypoint_profile;
    let arc_point = path.arc_point.expect("velocity rows require an arc point");
    let velocity_band = path
        .velocity_band
        .expect("velocity rows require a velocity band");
    let ReviewTreeRenderContext { comparison, .. } = *context;
    let candidate_records = flatten_vehicle_lane_records(candidate_vehicles);
    let baseline_records = flatten_vehicle_lane_records(baseline_vehicles);
    let aggregate =
        (!candidate_records.is_empty()).then(|| review_aggregate_from_records(&candidate_records));
    let baseline_aggregate =
        (!baseline_records.is_empty()).then(|| review_aggregate_from_records(&baseline_records));
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
            && waypoint_profile
                .map(|value| row.selector.waypoint_profile == value)
                .unwrap_or(true)
            && row.selector.arc_point == arc_point
            && row.selector.velocity_band == velocity_band
    });
    let changed =
        comparison.is_some() && aggregate_changed(current_row_aggregate, baseline_row_aggregate);
    let empty_candidate = VehicleLaneRecordGroups::new();
    let empty_baseline = VehicleLaneRecordGroups::new();
    let candidate_vehicles = candidate_vehicles.unwrap_or(&empty_candidate);
    let baseline_vehicles = baseline_vehicles.unwrap_or(&empty_baseline);
    let vehicle_keys = merged_map_keys(candidate_vehicles, Some(baseline_vehicles));
    let mut group_parts = vec!["band", mission, arrival_family, condition_set];
    if let Some(waypoint_profile) = waypoint_profile {
        group_parts.push(waypoint_profile);
    }
    group_parts.extend([arc_point, velocity_band]);
    let group_id = tree_group_id(group_parts.as_slice());
    let vehicle_rows = vehicle_keys
        .iter()
        .map(|vehicle_variant| {
            render_deep_vehicle_review_section(
                path.with_vehicle(vehicle_variant),
                candidate_vehicles.get(vehicle_variant),
                baseline_vehicles.get(vehicle_variant),
                context,
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
    rows.push_str(&render_summary_row(SummaryRow {
        label: velocity_band,
        depth,
        parent_group_id,
        group_id: (!vehicle_rows.is_empty()).then_some(group_id.as_str()),
        kind: matrix_radius_level_kind(mission),
        aggregate: current_row_aggregate,
        secondary_aggregate: baseline_row_aggregate,
        metric_style: SummaryMetricStyle::MeanDelta,
        changed,
        note_html: current_note.as_str(),
        tone: TreeRowTone::Current,
    }));
    rows.push_str(&vehicle_rows);
    rows
}

pub(super) fn render_deep_vehicle_review_section(
    path: MatrixReviewPath<'_>,
    candidate_lanes: Option<&LaneRecordGroups<'_>>,
    baseline_lanes: Option<&LaneRecordGroups<'_>>,
    context: &ReviewTreeRenderContext<'_>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let ConditionReviewPath {
        mission,
        arrival_family,
        condition_set,
    } = path.condition;
    let waypoint_profile = path.waypoint_profile;
    let vehicle_variant = path
        .vehicle_variant
        .expect("vehicle rows require a vehicle variant");
    let arc_point = path.arc_point;
    let velocity_band = path.velocity_band;
    let ReviewTreeRenderContext { comparison, .. } = *context;
    let candidate_records = flatten_lane_records(candidate_lanes);
    let baseline_records = flatten_lane_records(baseline_lanes);
    let aggregate =
        (!candidate_records.is_empty()).then(|| review_aggregate_from_records(&candidate_records));
    let baseline_aggregate =
        (!baseline_records.is_empty()).then(|| review_aggregate_from_records(&baseline_records));
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
            && waypoint_profile
                .map(|value| row.selector.waypoint_profile == value)
                .unwrap_or(true)
            && row.selector.vehicle_variant == vehicle_variant
            && arc_point
                .map(|value| row.selector.arc_point == value)
                .unwrap_or(true)
            && velocity_band
                .map(|value| row.selector.velocity_band == value)
                .unwrap_or(true)
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
    let mut group_parts = vec!["vehicle", mission, arrival_family, condition_set];
    if let Some(waypoint_profile) = waypoint_profile {
        group_parts.push(waypoint_profile);
    }
    if let Some(arc_point) = arc_point {
        group_parts.push(arc_point);
    }
    if let Some(velocity_band) = velocity_band {
        group_parts.push(velocity_band);
    }
    group_parts.push(vehicle_variant);
    let group_id = tree_group_id(group_parts.as_slice());
    let lane_rows = lane_keys
        .iter()
        .map(|lane_id| {
            render_lane_review_section(
                path,
                lane_id,
                candidate_lanes.get(lane_id),
                baseline_lanes.get(lane_id),
                context,
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
    rows.push_str(&render_summary_row(SummaryRow {
        label: vehicle_variant,
        depth,
        parent_group_id,
        group_id: (!lane_rows.is_empty()).then_some(group_id.as_str()),
        kind: "vehicle",
        aggregate: current_row_aggregate,
        secondary_aggregate: baseline_row_aggregate,
        metric_style: SummaryMetricStyle::MeanDelta,
        changed,
        note_html: current_note.as_str(),
        tone: TreeRowTone::Current,
    }));
    rows.push_str(&lane_rows);
    rows
}

pub(super) fn render_vehicle_review_section(
    path: MatrixReviewPath<'_>,
    candidate_arcs: Option<&ArcRecordGroups<'_>>,
    baseline_arcs: Option<&ArcRecordGroups<'_>>,
    context: &ReviewTreeRenderContext<'_>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let ConditionReviewPath {
        mission,
        arrival_family,
        condition_set,
    } = path.condition;
    let vehicle_variant = path
        .vehicle_variant
        .expect("vehicle rows require a vehicle variant");
    let ReviewTreeRenderContext { comparison, .. } = *context;
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
                    path.with_arc(Some(arc_point)),
                    candidate_arcs.get(arc_point),
                    baseline_arcs.get(arc_point),
                    context,
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
                    path,
                    lane_id,
                    candidate_lanes.get(lane_id),
                    baseline_lanes.get(lane_id),
                    context,
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
    rows.push_str(&render_summary_row(SummaryRow {
        label: vehicle_variant,
        depth,
        parent_group_id,
        group_id: ((!arc_keys.is_empty()) || !child_rows.is_empty()).then_some(group_id.as_str()),
        kind: "vehicle",
        aggregate: current_row_aggregate,
        secondary_aggregate: baseline_row_aggregate,
        metric_style: SummaryMetricStyle::MeanDelta,
        changed,
        note_html: current_note.as_str(),
        tone: TreeRowTone::Current,
    }));
    rows.push_str(&child_rows);
    rows
}

pub(super) fn render_arc_review_section(
    path: MatrixReviewPath<'_>,
    candidate_velocities: Option<&VelocityRecordGroups<'_>>,
    baseline_velocities: Option<&VelocityRecordGroups<'_>>,
    context: &ReviewTreeRenderContext<'_>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let ConditionReviewPath {
        mission,
        arrival_family,
        condition_set,
    } = path.condition;
    let vehicle_variant = path
        .vehicle_variant
        .expect("arc rows require a vehicle variant");
    let arc_point = path.arc_point.expect("arc rows require an arc point");
    let ReviewTreeRenderContext { comparison, .. } = *context;
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
                    path.with_velocity(Some(velocity_band)),
                    candidate_velocities.get(velocity_band),
                    baseline_velocities.get(velocity_band),
                    context,
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
                    path,
                    lane_id,
                    candidate_lanes.get(lane_id),
                    baseline_lanes.get(lane_id),
                    context,
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
    rows.push_str(&render_summary_row(SummaryRow {
        label: arc_point,
        depth,
        parent_group_id,
        group_id: ((!velocity_keys.is_empty()) || !child_rows.is_empty())
            .then_some(group_id.as_str()),
        kind: matrix_route_level_kind(mission),
        aggregate: current_row_aggregate,
        secondary_aggregate: baseline_row_aggregate,
        metric_style: SummaryMetricStyle::MeanDelta,
        changed,
        note_html: current_note.as_str(),
        tone: TreeRowTone::Current,
    }));
    rows.push_str(&child_rows);
    rows
}

pub(super) fn render_velocity_review_section(
    path: MatrixReviewPath<'_>,
    candidate_lanes: Option<&LaneRecordGroups<'_>>,
    baseline_lanes: Option<&LaneRecordGroups<'_>>,
    context: &ReviewTreeRenderContext<'_>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let ConditionReviewPath {
        mission,
        arrival_family,
        condition_set,
    } = path.condition;
    let vehicle_variant = path
        .vehicle_variant
        .expect("velocity rows require a vehicle variant");
    let arc_point = path.arc_point.expect("velocity rows require an arc point");
    let velocity_band = path
        .velocity_band
        .expect("velocity rows require a velocity band");
    let ReviewTreeRenderContext { comparison, .. } = *context;
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
                path,
                lane_id,
                candidate_lanes.get(lane_id),
                baseline_lanes.get(lane_id),
                context,
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
    rows.push_str(&render_summary_row(SummaryRow {
        label: velocity_band,
        depth,
        parent_group_id,
        group_id: (!lane_keys.is_empty()).then_some(group_id.as_str()),
        kind: matrix_radius_level_kind(mission),
        aggregate: current_row_aggregate,
        secondary_aggregate: baseline_row_aggregate,
        metric_style: SummaryMetricStyle::MeanDelta,
        changed,
        note_html: current_note.as_str(),
        tone: TreeRowTone::Current,
    }));
    rows.push_str(&lane_rows);
    rows
}

pub(super) fn render_lane_review_section(
    path: MatrixReviewPath<'_>,
    lane_id: &str,
    candidate_records: Option<&Vec<&crate::BatchRunRecord>>,
    baseline_records: Option<&Vec<&crate::BatchRunRecord>>,
    context: &ReviewTreeRenderContext<'_>,
    depth: usize,
    parent_group_id: Option<&str>,
) -> String {
    let ConditionReviewPath {
        mission,
        arrival_family,
        condition_set,
    } = path.condition;
    let waypoint_profile = path.waypoint_profile;
    let vehicle_variant = path
        .vehicle_variant
        .expect("lane rows require a vehicle variant");
    let arc_point = path.arc_point;
    let velocity_band = path.velocity_band;
    let ReviewTreeRenderContext {
        run_change_map,
        comparison,
        output_dir,
        render_cache,
    } = *context;
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
            && waypoint_profile
                .map(|value| row.selector.waypoint_profile == value)
                .unwrap_or(true)
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
    let mut group_parts = vec!["lane", mission, arrival_family, condition_set];
    if let Some(waypoint_profile) = waypoint_profile {
        group_parts.push(waypoint_profile);
    }
    group_parts.push(vehicle_variant);
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
    );
    let mut rows = String::new();
    let current_lane_label =
        display_compare_role_label(comparison.is_some(), lane_id, TreeRowTone::Current);
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
        render_lane_preview(candidate_records.as_slice(), render_cache).as_deref(),
    );
    rows.push_str(&render_summary_row(SummaryRow {
        label: current_lane_label.as_str(),
        depth,
        parent_group_id,
        group_id: (!candidate_records.is_empty()).then_some(group_id.as_str()),
        kind: "lane",
        aggregate: aggregate.as_ref(),
        secondary_aggregate: None,
        metric_style: SummaryMetricStyle::MeanStddev,
        changed,
        note_html: current_note.as_str(),
        tone: TreeRowTone::Current,
    }));
    if comparison.is_some() && baseline_aggregate.is_some() {
        let baseline_lane_label = display_compare_role_label(true, lane_id, TreeRowTone::Baseline);
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
            render_lane_preview(baseline_records.as_slice(), render_cache).as_deref(),
        );
        rows.push_str(&render_summary_row(SummaryRow {
            label: baseline_lane_label.as_str(),
            depth,
            parent_group_id,
            group_id: baseline_group,
            kind: "lane",
            aggregate: baseline_aggregate.as_ref(),
            secondary_aggregate: None,
            metric_style: SummaryMetricStyle::MeanStddev,
            changed,
            note_html: baseline_note.as_str(),
            tone: TreeRowTone::Baseline,
        }));
    }
    rows.push_str(&run_rows);
    rows
}

pub(super) fn render_entry_run_table(
    candidate_records: &[&crate::BatchRunRecord],
    baseline_records: &[&crate::BatchRunRecord],
    show_baseline: bool,
    depth: usize,
    parent_group_id: &str,
    run_change_map: &BTreeMap<String, (&'static str, &'static str)>,
    output_dir: &Path,
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

pub(super) fn tree_group_id(parts: &[&str]) -> String {
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
                } else if ch == '+' {
                    out.push_str("plus");
                    last_dash = false;
                } else if ch == '-' {
                    out.push_str("minus");
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

pub(super) fn render_summary_note(
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

pub(super) struct SummaryRow<'a> {
    label: &'a str,
    depth: usize,
    parent_group_id: Option<&'a str>,
    group_id: Option<&'a str>,
    kind: &'a str,
    aggregate: Option<&'a ReviewAggregate>,
    secondary_aggregate: Option<&'a ReviewAggregate>,
    metric_style: SummaryMetricStyle,
    changed: bool,
    note_html: &'a str,
    tone: TreeRowTone,
}

pub(super) fn render_summary_row(row: SummaryRow<'_>) -> String {
    let SummaryRow {
        label,
        depth,
        parent_group_id,
        group_id,
        kind,
        aggregate,
        secondary_aggregate,
        metric_style,
        changed,
        note_html,
        tone,
    } = row;
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
        aggregate.and_then(aggregate_ref_dev_metric),
        secondary_aggregate.and_then(aggregate_ref_dev_metric),
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

pub(super) fn render_seed_run_row(
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
    let outcome_label = enum_label(&record.manifest.mission_outcome);
    let analytic_note = analytic_reason_note(&record.analytic);
    let outcome = if !record.analytic.is_scored() {
        let label = if let Some(note) = analytic_note {
            format!("{outcome_label} · {note}")
        } else {
            format!("{outcome_label} · impossible")
        };
        format!(r#"<span class="warn">{}</span>"#, escape_html(&label))
    } else if matches!(
        record.manifest.mission_outcome,
        pd_core::MissionOutcome::Success
    ) {
        if let Some(note) = analytic_note {
            escape_html(&format!("{outcome_label} · {note}"))
        } else {
            escape_html(&outcome_label)
        }
    } else {
        let label = if let Some(note) = analytic_note {
            format!("{outcome_label} · {note}")
        } else {
            outcome_label
        };
        if let Some(detail) = waypoint_checkpoint_failure_detail(record) {
            format!(
                r#"<div class="overview-stack"><div class="overview-main"><span class="outcome-bad">{}</span></div><div class="overview-sub">{}</div></div>"#,
                escape_html(&label),
                escape_html(&detail),
            )
        } else {
            format!(
                r#"<span class="outcome-bad">{}</span>"#,
                escape_html(&label)
            )
        }
    };
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
        .transfer_shape_curve_rmse_m
        .or(record.review.reference_gap_mean_m)
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
    let detail_note = match (change_note.is_empty(), analytic_note) {
        (false, Some(analytic_note)) => format!(
            r#"<span class="row-note">{change_note} · <span class="warn">{}</span></span>"#,
            escape_html(analytic_note)
        ),
        (false, None) => format!(r#"<span class="row-note">{change_note}</span>"#),
        (true, Some(analytic_note)) => format!(
            r#"<span class="row-note"><span class="warn">{}</span></span>"#,
            escape_html(analytic_note)
        ),
        (true, None) => String::new(),
    };
    let transfer_note = render_transfer_review_note(&record.review);
    let details = format!(
        r#"{detail_note}{transfer_note}<div class="preview-cell">{preview}</div>"#,
        detail_note = detail_note,
        transfer_note = transfer_note,
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
        outcome = outcome,
        fuel = escape_html(&fuel),
        sim_time = escape_html(&sim_time),
        landing_offset = escape_html(&landing_offset),
        reference_gap = escape_html(&reference_gap),
        details = details,
    )
}

pub(super) fn waypoint_checkpoint_failure_detail(record: &crate::BatchRunRecord) -> Option<String> {
    if !matches!(
        record.manifest.mission_outcome,
        pd_core::MissionOutcome::FailedCheckpoint
    ) {
        return None;
    }
    let waypoint_index = record.review.waypoint_route_first_failure_index?;
    let handoff = record
        .review
        .waypoint_handoffs
        .iter()
        .find(|handoff| handoff.waypoint_index == waypoint_index)?;
    let parameter = |metric: &str| {
        record
            .resolved
            .resolved_parameters
            .get(&format!("waypoint_{waypoint_index}_{metric}"))
            .copied()
    };
    let mut violations = Vec::new();
    for reason in &handoff.contract_reasons {
        match reason.as_str() {
            "heading" => {
                if let (Some(actual), Some(limit)) = (
                    handoff.outbound_heading_error_rad,
                    parameter("max_outbound_heading_error_rad"),
                ) {
                    violations.push(format!(
                        "heading {:.2}deg > {:.2}deg (+{:.2}deg)",
                        actual.to_degrees(),
                        limit.to_degrees(),
                        (actual - limit).max(0.0).to_degrees(),
                    ));
                }
            }
            "outbound_progress" => {
                if let (Some(actual), Some(limit)) = (
                    handoff.outbound_progress_mps,
                    parameter("min_outbound_progress_mps"),
                ) {
                    violations.push(format!(
                        "progress {actual:.2}m/s < {limit:.2}m/s ({:.2}m/s)",
                        actual - limit,
                    ));
                }
            }
            "outbound_cross_speed" => {
                if let (Some(actual), Some(limit)) = (
                    handoff.outbound_cross_speed_mps,
                    parameter("max_outbound_cross_speed_mps"),
                ) {
                    let actual = actual.abs();
                    violations.push(format!(
                        "cross speed {actual:.2}m/s > {limit:.2}m/s (+{:.2}m/s)",
                        (actual - limit).max(0.0),
                    ));
                }
            }
            "speed" => {
                if let Some(actual) = handoff.speed_mps {
                    let min = parameter("min_speed_mps");
                    let max = parameter("max_speed_mps");
                    match (min, max) {
                        (Some(min), _) if actual < min => violations.push(format!(
                            "speed {actual:.2}m/s < {min:.2}m/s ({:.2}m/s)",
                            actual - min,
                        )),
                        (_, Some(max)) if actual > max => violations.push(format!(
                            "speed {actual:.2}m/s > {max:.2}m/s (+{:.2}m/s)",
                            actual - max,
                        )),
                        _ => {}
                    }
                }
            }
            "vertical_speed" => {
                if let Some(actual) = handoff.vertical_speed_mps {
                    let min = parameter("min_vertical_speed_mps");
                    let max = parameter("max_vertical_speed_mps");
                    match (min, max) {
                        (Some(min), _) if actual < min => violations.push(format!(
                            "vertical speed {actual:.2}m/s < {min:.2}m/s ({:.2}m/s)",
                            actual - min,
                        )),
                        (_, Some(max)) if actual > max => violations.push(format!(
                            "vertical speed {actual:.2}m/s > {max:.2}m/s (+{:.2}m/s)",
                            actual - max,
                        )),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    if violations.is_empty() && handoff.contract_status.as_deref() == Some("spatial_miss") {
        if let (Some(closest), Some(capture)) = (
            handoff.closest_distance_m.or(handoff.distance_m),
            parameter("capture_radius_m"),
        ) {
            violations.push(format!(
                "spatial miss: closest {closest:.2}m, capture {capture:.2}m"
            ));
        } else {
            violations.push("spatial miss".to_owned());
        }
    }
    if violations.is_empty() {
        return None;
    }
    let mut detail = format!("WP{} {}", waypoint_index + 1, violations.join(" · "));
    if handoff.reachable_candidate_pass_lost_before_capture == Some(true) {
        detail.push_str(" · reachable pass lost");
    }
    Some(detail)
}

pub(super) fn render_transfer_review_note(review: &crate::BatchRunReviewMetrics) -> String {
    let Some(final_phase) = review.transfer_final_phase.as_deref() else {
        return String::new();
    };
    let mut parts = vec![format!("transfer {final_phase}")];
    let terminal_entry_label = match review.transfer_terminal_entry_kind.as_deref() {
        Some("direct") => "terminal entry",
        _ => "handoff",
    };
    let direct_terminal_entry = review.transfer_terminal_entry_kind.as_deref() == Some("direct");
    if let Some(time_s) = review.transfer_terminal_handoff_time_s {
        parts.push(format!("{terminal_entry_label} {time_s:.1}s"));
    }
    if let Some(gate_mode) = review.transfer_terminal_handoff_gate_mode.as_deref() {
        parts.push(format!("{terminal_entry_label} gate {gate_mode}"));
    }
    if let Some(quality) = review.transfer_terminal_handoff_boost_quality.as_deref()
        && Some(quality) != review.transfer_boost_quality.as_deref()
        && !direct_terminal_entry
    {
        parts.push(format!("{terminal_entry_label} boost {quality}"));
    }
    if let Some(projected_dx_m) = review.transfer_terminal_handoff_projected_dx_m {
        parts.push(format!("{terminal_entry_label} pdx {projected_dx_m:.0}m"));
    }
    if let Some(impact_angle_deg) = review.transfer_terminal_handoff_impact_angle_deg {
        parts.push(format!(
            "{terminal_entry_label} angle {impact_angle_deg:.0}deg"
        ));
    }
    if let Some(required_accel_ratio) = review.transfer_terminal_handoff_required_accel_ratio
        && review.transfer_terminal_handoff_gate_mode.as_deref() == Some("pending")
        && !direct_terminal_entry
    {
        parts.push(format!(
            "{terminal_entry_label} accel {required_accel_ratio:.2}x"
        ));
    }
    if let Some(quality) = review.transfer_boost_quality.as_deref() {
        parts.push(format!("boost {quality}"));
    }
    if let Some(settled_quality) = review.transfer_boost_settled_quality.as_deref()
        && settled_quality != review.transfer_boost_quality.as_deref().unwrap_or("")
    {
        let settled_dx = review
            .transfer_boost_settled_projected_dx_m
            .map(|value| format!(" pdx {value:.0}m"))
            .unwrap_or_default();
        parts.push(format!("settled {settled_quality}{settled_dx}"));
    }
    if let Some(cutoff_quality) = review.transfer_boost_cutoff_quality.as_deref() {
        let cutoff = review
            .transfer_boost_cutoff_time_s
            .map(|time_s| format!("{time_s:.1}s"))
            .unwrap_or_else(|| "?".to_owned());
        let cutoff_dx = review
            .transfer_boost_cutoff_projected_dx_m
            .map(|value| format!(" pdx {value:.0}m"))
            .unwrap_or_default();
        let cutoff_angle = review
            .transfer_boost_cutoff_impact_angle_deg
            .map(|value| format!(" angle {value:.0}deg"))
            .unwrap_or_default();
        parts.push(format!(
            "cut {cutoff} {cutoff_quality}{cutoff_dx}{cutoff_angle}"
        ));
    }
    if let Some(shape_rmse_m) = review.transfer_shape_curve_rmse_m {
        parts.push(format!("shape {shape_rmse_m:.0}m"));
    }
    if let Some(apex_error_m) = review.transfer_shape_apex_error_m {
        parts.push(format!("apex err {apex_error_m:.0}m"));
    }
    if let Some(rebound_gain_m) = review.transfer_terminal_low_altitude_rebound_gain_m
        && rebound_gain_m > TRANSFER_TERMINAL_REBOUND_RISK_GAIN_M
    {
        let origin_dx_m = review
            .transfer_terminal_low_altitude_rebound_origin_dx_abs_m
            .unwrap_or(0.0);
        let label = if review.transfer_terminal_low_altitude_rebound_near_pad == Some(true) {
            "near-pad rebound"
        } else {
            "recovery climb"
        };
        parts.push(format!(
            "{label} {rebound_gain_m:.0}m at dx {origin_dx_m:.0}m"
        ));
    }
    if let Some(shortfall_ratio) = review.transfer_shape_shortfall_ratio {
        parts.push(format!("short {:.0}%", shortfall_ratio * 100.0));
    }
    if let Some(burn_duration_s) = review.transfer_boost_burn_duration_s {
        if let Some(fuel_used_kg) = review.transfer_boost_burn_fuel_used_kg {
            parts.push(format!("burn {burn_duration_s:.1}s/{fuel_used_kg:.0}kg"));
        } else {
            parts.push(format!("burn {burn_duration_s:.1}s"));
        }
    }
    if let Some(gate_mode) = review.transfer_terminal_gate_mode.as_deref() {
        parts.push(format!("gate {gate_mode}"));
    }
    if review.transfer_terminal_gate_deferred == Some(true) {
        parts.push("gate deferred".to_owned());
    }
    if let Some(latest_safe_margin_s) = review.transfer_terminal_gate_latest_safe_margin_s {
        parts.push(format!("margin {latest_safe_margin_s:.1}s"));
    }
    if let Some(corridor_margin_m) = review.transfer_corridor_min_margin_m
        && (review.transfer_corridor_mode.as_deref() == Some("active") || corridor_margin_m < 0.0)
    {
        parts.push(format!("corridor {corridor_margin_m:.0}m"));
    }
    if let Some(dx_m) = review.transfer_terminal_handoff_dx_m {
        parts.push(format!("dx {dx_m:.0}m"));
    }
    if let Some(height_m) = review.transfer_terminal_handoff_height_m {
        parts.push(format!("h {height_m:.0}m"));
    }
    if let Some(speed_mps) = review.transfer_terminal_handoff_speed_mps {
        parts.push(format!("v {speed_mps:.1}m/s"));
    }
    format!(
        r#"<span class="row-note transfer-note">{}</span>"#,
        escape_html(&parts.join(" · "))
    )
}

pub(super) fn records_by_selector_hierarchy<'a>(
    candidate: &'a BatchReport,
) -> MissionRecordGroups<'a> {
    let records = candidate.records.iter().collect::<Vec<_>>();
    records_by_selector_hierarchy_from_records(records.as_slice())
}

pub(super) fn records_by_selector_hierarchy_from_records<'a>(
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

pub(super) fn records_by_arc_velocity_vehicle_from_records<'a>(
    records: &[&'a crate::BatchRunRecord],
) -> ArcVelocityVehicleRecordGroups<'a> {
    let mut grouped = ArcVelocityVehicleRecordGroups::new();
    for &record in records {
        grouped
            .entry(record.resolved.selector.arc_point.clone())
            .or_default()
            .entry(record.resolved.selector.velocity_band.clone())
            .or_default()
            .entry(record.resolved.selector.vehicle_variant.clone())
            .or_default()
            .entry(record.resolved.lane_id.clone())
            .or_default()
            .push(record);
    }
    grouped
}

pub(super) fn records_by_waypoint_profile<'a>(
    records: &[&'a crate::BatchRunRecord],
) -> WaypointProfileRecordGroups<'a> {
    let mut grouped = WaypointProfileRecordGroups::new();
    for &record in records {
        grouped
            .entry(selector_value_or_unspecified(
                &record.resolved.selector.waypoint_profile,
            ))
            .or_default()
            .push(record);
    }
    grouped
}

pub(super) fn records_by_vehicle_lane_from_records<'a>(
    records: &[&'a crate::BatchRunRecord],
) -> VehicleLaneRecordGroups<'a> {
    let mut grouped = VehicleLaneRecordGroups::new();
    for &record in records {
        grouped
            .entry(record.resolved.selector.vehicle_variant.clone())
            .or_default()
            .entry(record.resolved.lane_id.clone())
            .or_default()
            .push(record);
    }
    grouped
}

pub(super) fn merged_map_keys<T, U>(
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

pub(super) fn sort_selector_keys(keys: &mut [String]) {
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

pub(super) fn selector_sort_rank(key: &str) -> u8 {
    match key {
        "clean" => 0,
        "traj_undershoot_small" => 1,
        "traj_undershoot_large" => 2,
        "traj_overshoot_small" => 3,
        "traj_overshoot_large" => 4,
        "terrain_backstop_wall" => 5,
        "terrain_backstop_slanted" => 6,
        "terrain_clip" => 7,
        "r-80" => 0,
        "r-60" => 1,
        "r-45" => 2,
        "r-30" => 3,
        "r-15" => 4,
        "r00" => 5,
        "r+15" => 6,
        "r+30" => 7,
        "r+45" => 8,
        "r+60" => 9,
        "r+80" => 10,
        "single_gentle_bend_v1" => 0,
        "single_medium_bend_v1" | "single_bend_v1" => 1,
        "single_sharp_bend_v1" => 2,
        "single_dogleg_v1" => 3,
        "double_bend_v1" => 4,
        "late_bend_v1" => 5,
        "low" => 0,
        "mid" => 1,
        "high" => 2,
        "empty" => 10,
        "half" => 11,
        "full" => 12,
        "short" => 8,
        "nominal" => 9,
        "long" => 10,
        "low_margin" => 11,
        "low_fuel" => 12,
        "heavy_cargo" => 13,
        _ => 20,
    }
}

pub(super) fn matrix_route_level_kind(mission: &str) -> &'static str {
    if mission == "transfer_guidance" {
        "route"
    } else {
        "arc"
    }
}

pub(super) fn matrix_radius_level_kind(mission: &str) -> &'static str {
    if mission == "transfer_guidance" {
        "radius"
    } else {
        "band"
    }
}

pub(super) fn has_meaningful_selector_keys(keys: &[String]) -> bool {
    keys.iter().any(|key| key != UNSPECIFIED_SELECTOR_VALUE)
}

pub(super) fn sort_lane_keys(keys: &mut [String]) {
    keys.sort_by(|lhs, rhs| {
        lane_sort_rank(lhs)
            .cmp(&lane_sort_rank(rhs))
            .then(lhs.cmp(rhs))
    });
}

pub(super) fn lane_sort_rank(lane_id: &str) -> u8 {
    match lane_id {
        "current" | "staged" => 0,
        "baseline" => 1,
        _ => 2,
    }
}

pub(super) fn display_lane_label(lane_id: &str) -> String {
    match lane_id {
        "current" | "staged" => "current".to_owned(),
        "baseline" => "baseline".to_owned(),
        _ => lane_id.to_owned(),
    }
}

pub(super) fn display_compare_role_label(
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

pub(super) fn controller_ids_for_records(records: &[&crate::BatchRunRecord]) -> Vec<String> {
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

pub(super) fn render_controller_summary_inline(records: &[&crate::BatchRunRecord]) -> String {
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

pub(super) fn flatten_arrival_records<'a>(
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

pub(super) fn flatten_condition_records<'a>(
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

pub(super) fn flatten_velocity_vehicle_records<'a>(
    groups: Option<&VelocityVehicleRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
    let mut out = Vec::new();
    if let Some(groups) = groups {
        for vehicles in groups.values() {
            for lanes in vehicles.values() {
                for records in lanes.values() {
                    out.extend(records.iter().copied());
                }
            }
        }
    }
    out
}

pub(super) fn flatten_vehicle_lane_records<'a>(
    groups: Option<&VehicleLaneRecordGroups<'a>>,
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

pub(super) fn flatten_vehicle_records<'a>(
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

pub(super) fn flatten_arc_records<'a>(
    groups: Option<&ArcRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
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

pub(super) fn flatten_velocity_records<'a>(
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

pub(super) fn flatten_lane_records<'a>(
    groups: Option<&LaneRecordGroups<'a>>,
) -> Vec<&'a crate::BatchRunRecord> {
    groups
        .map(|groups| groups.values().flatten().copied().collect::<Vec<_>>())
        .unwrap_or_default()
}

pub(super) fn selector_case_key(selector: &crate::SelectorAxes) -> String {
    let mut parts = vec![
        selector.mission.as_str(),
        selector.arrival_family.as_str(),
        selector.condition_set.as_str(),
    ];
    if selector.waypoint_profile != UNSPECIFIED_SELECTOR_VALUE {
        parts.push(selector.waypoint_profile.as_str());
    }
    if selector.waypoint_handoff_envelope != UNSPECIFIED_SELECTOR_VALUE {
        parts.push(selector.waypoint_handoff_envelope.as_str());
    }
    parts.push(selector.vehicle_variant.as_str());
    if selector.arc_point != UNSPECIFIED_SELECTOR_VALUE {
        parts.push(selector.arc_point.as_str());
    } else if selector.route_angle != UNSPECIFIED_SELECTOR_VALUE {
        parts.push(selector.route_angle.as_str());
    }
    if selector.velocity_band != UNSPECIFIED_SELECTOR_VALUE {
        parts.push(selector.velocity_band.as_str());
    } else if selector.radius_tier != UNSPECIFIED_SELECTOR_VALUE {
        parts.push(selector.radius_tier.as_str());
    }
    parts.join(" / ")
}

pub(super) fn count_regressions<F>(comparison: Option<&BatchComparison>, predicate: F) -> usize
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

pub(super) fn expectation_tier(records: &[&crate::BatchRunRecord]) -> Option<String> {
    records
        .iter()
        .filter_map(|record| record.resolved.selector.expectation_tier.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .next()
}

pub(super) fn controller_lane_records<'a>(
    records: &[&'a crate::BatchRunRecord],
    lane_id: &str,
) -> Vec<&'a crate::BatchRunRecord> {
    records
        .iter()
        .copied()
        .filter(|record| record.resolved.lane_id == lane_id)
        .collect::<Vec<_>>()
}

pub(super) fn preferred_current_lane_id(
    records: &[&crate::BatchRunRecord],
) -> Option<&'static str> {
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

pub(super) fn controller_lane_aggregate(
    records: &[&crate::BatchRunRecord],
    lane_id: &str,
) -> Option<ReviewAggregate> {
    let filtered = controller_lane_records(records, lane_id);
    (!filtered.is_empty()).then(|| review_aggregate_from_records(filtered.as_slice()))
}

pub(super) fn preferred_current_lane_aggregate(
    records: &[&crate::BatchRunRecord],
) -> Option<ReviewAggregate> {
    preferred_current_lane_id(records)
        .and_then(|lane_id| controller_lane_aggregate(records, lane_id))
}

pub(super) fn extract_default_lane_groups_from_arcs<'a>(
    groups: Option<&'a ArcRecordGroups<'a>>,
) -> Option<&'a LaneRecordGroups<'a>> {
    groups
        .and_then(|groups| groups.get(UNSPECIFIED_SELECTOR_VALUE))
        .and_then(|velocities| velocities.get(UNSPECIFIED_SELECTOR_VALUE))
}

pub(super) fn comparison_change_map(
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

pub(super) fn review_aggregate_from_records(records: &[&crate::BatchRunRecord]) -> ReviewAggregate {
    let total_runs = records.len();
    let success_runs = records
        .iter()
        .filter(|record| {
            record.analytic.is_scored()
                && matches!(
                    record.manifest.mission_outcome,
                    pd_core::MissionOutcome::Success
                )
        })
        .count();
    let invalidated_runs = records
        .iter()
        .filter(|record| !record.analytic.is_scored())
        .count();
    let failure_runs = records
        .iter()
        .filter(|record| {
            record.analytic.is_scored()
                && !matches!(
                    record.manifest.mission_outcome,
                    pd_core::MissionOutcome::Success
                )
        })
        .count();
    let failed_seeds = records
        .iter()
        .filter(|record| {
            record.analytic.is_scored()
                && !matches!(
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
            record.analytic.is_scored()
                && matches!(
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
    let low_altitude_dwell_values = success_records
        .iter()
        .filter_map(|record| record.review.low_altitude_dwell_s)
        .collect::<Vec<_>>();
    let low_altitude_unsafe_values = success_records
        .iter()
        .filter_map(|record| record.review.low_altitude_unsafe_recovery_s)
        .collect::<Vec<_>>();
    let reference_gap_values = success_records
        .iter()
        .filter_map(|record| record.review.reference_gap_mean_m)
        .collect::<Vec<_>>();
    let transfer_shape_curve_values = success_records
        .iter()
        .filter_map(|record| record.review.transfer_shape_curve_rmse_m)
        .collect::<Vec<_>>();
    ReviewAggregate {
        total_runs,
        success_runs,
        failure_runs,
        invalidated_runs,
        sim_time_stats: crate::metric_summary(&sim_time_values),
        fuel_used_pct_of_max: crate::metric_summary(&fuel_values),
        landing_offset_abs_m: crate::metric_summary(&landing_offset_values),
        low_altitude_dwell_s: crate::metric_summary(&low_altitude_dwell_values),
        low_altitude_unsafe_recovery_s: crate::metric_summary(&low_altitude_unsafe_values),
        reference_gap_mean_m: crate::metric_summary(&reference_gap_values),
        transfer_shape_curve_rmse_m: crate::metric_summary(&transfer_shape_curve_values),
        failed_seeds,
    }
}

pub(super) fn inline_rate_text(success_runs: usize, total_runs: usize) -> String {
    if total_runs == 0 {
        return "n/a of 0 scored".to_owned();
    }
    format!(
        "{:.1}% of {} scored",
        crate::success_rate(success_runs, total_runs) * 100.0,
        total_runs
    )
}

#[derive(Clone, Copy)]
pub(super) enum MetricDisplayKind {
    Percent,
    Seconds,
    Meters,
    Speed,
    Degrees,
}

pub(super) fn format_metric_value(
    summary: &crate::BatchMetricSummary,
    kind: MetricDisplayKind,
) -> String {
    match kind {
        MetricDisplayKind::Percent => format!("{:.1}%", summary.mean),
        MetricDisplayKind::Seconds => format!("{:.2}s", summary.mean),
        MetricDisplayKind::Meters => format!("{:.2}m", summary.mean),
        MetricDisplayKind::Speed => format!("{:.2}m/s", summary.mean),
        MetricDisplayKind::Degrees => format!("{:.1}deg", summary.mean),
    }
}

pub(super) fn format_metric_delta_value(delta: f64, kind: MetricDisplayKind) -> String {
    match kind {
        MetricDisplayKind::Percent => format!("{:+.1}%", delta),
        MetricDisplayKind::Seconds => format!("{:+.2}s", delta),
        MetricDisplayKind::Meters => format!("{:+.2}m", delta),
        MetricDisplayKind::Speed => format!("{:+.2}m/s", delta),
        MetricDisplayKind::Degrees => format!("{:+.1}deg", delta),
    }
}

pub(super) fn format_metric_mean(
    summary: Option<&crate::BatchMetricSummary>,
    kind: MetricDisplayKind,
) -> String {
    summary
        .map(|summary| format_metric_value(summary, kind))
        .unwrap_or_else(|| "-".to_owned())
}

pub(super) fn format_metric_stddev(
    summary: Option<&crate::BatchMetricSummary>,
    kind: MetricDisplayKind,
) -> String {
    let Some(summary) = summary else {
        return "std -".to_owned();
    };
    let stddev = summary.stddev.unwrap_or(0.0);
    match kind {
        MetricDisplayKind::Percent => format!("std {stddev:.1}%"),
        MetricDisplayKind::Seconds => format!("std {stddev:.2}s"),
        MetricDisplayKind::Meters => format!("std {stddev:.2}m"),
        MetricDisplayKind::Speed => format!("std {stddev:.2}m/s"),
        MetricDisplayKind::Degrees => format!("std {stddev:.1}deg"),
    }
}

pub(super) fn format_metric_summary(
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
        MetricDisplayKind::Speed => format!(
            "{:.2} ± {:.2}m/s",
            summary.mean,
            summary.stddev.unwrap_or(0.0)
        ),
        MetricDisplayKind::Degrees => format!(
            "{:.1} ± {:.1}deg",
            summary.mean,
            summary.stddev.unwrap_or(0.0)
        ),
    }
}

pub(super) fn format_metric_cell(
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

pub(super) fn analytic_reason_note(
    analytic: &crate::BatchRunAnalyticFeasibility,
) -> Option<&'static str> {
    match analytic.reason {
        Some(crate::BatchRunAnalyticReason::VerticalStopHeight) => {
            Some("impossible vertical brake")
        }
        Some(crate::BatchRunAnalyticReason::CoupledStopAcceleration) => {
            Some("impossible coupled brake")
        }
        Some(crate::BatchRunAnalyticReason::LowThrustHighEnergy) => {
            Some("low-thrust high-energy frontier")
        }
        Some(crate::BatchRunAnalyticReason::NearVerticalTransferRoute) => {
            Some("near-vertical transfer-route frontier")
        }
        None => None,
    }
}

pub(super) fn format_summary_rate(
    aggregate: &ReviewAggregate,
    baseline: Option<&ReviewAggregate>,
    style: SummaryMetricStyle,
) -> String {
    let scored_runs = aggregate
        .total_runs
        .saturating_sub(aggregate.invalidated_runs);
    let failure_html = if aggregate.failure_runs > 0 {
        format!(
            r#"<span class="outcome-bad">{} fail</span>"#,
            aggregate.failure_runs
        )
    } else {
        "0 fail".to_owned()
    };
    let invalidated_html = if aggregate.invalidated_runs > 0 {
        format!(
            r#" · <span class="warn">{} warning</span>"#,
            aggregate.invalidated_runs
        )
    } else {
        String::new()
    };
    let base = format!(
        "{} · {}{}",
        escape_html(&inline_rate_text(aggregate.success_runs, scored_runs)),
        failure_html,
        invalidated_html
    );
    match style {
        SummaryMetricStyle::MeanStddev => base,
        SummaryMetricStyle::MeanDelta => {
            let Some(baseline) = baseline else {
                return base;
            };
            let delta = (crate::success_rate(
                aggregate.success_runs,
                aggregate
                    .total_runs
                    .saturating_sub(aggregate.invalidated_runs),
            ) - crate::success_rate(
                baseline.success_runs,
                baseline
                    .total_runs
                    .saturating_sub(baseline.invalidated_runs),
            )) * 100.0;
            format!(
                r#"{}<span class="compare-toggle-target"> ({delta:+.1}pt)</span>"#,
                base
            )
        }
    }
}

pub(super) fn metric_delta_value(
    candidate: Option<&crate::BatchMetricSummary>,
    baseline: Option<&crate::BatchMetricSummary>,
) -> Option<f64> {
    Some(candidate?.mean - baseline?.mean)
}

pub(super) fn aggregate_changed(
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
        || candidate.invalidated_runs != baseline.invalidated_runs
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
            candidate.low_altitude_dwell_s.as_ref(),
            baseline.low_altitude_dwell_s.as_ref(),
        )
        .is_some_and(|delta| delta.abs() > 1e-9)
        || metric_delta_value(
            candidate.low_altitude_unsafe_recovery_s.as_ref(),
            baseline.low_altitude_unsafe_recovery_s.as_ref(),
        )
        .is_some_and(|delta| delta.abs() > 1e-9)
        || metric_delta_value(
            aggregate_ref_dev_metric(candidate),
            aggregate_ref_dev_metric(baseline),
        )
        .is_some_and(|delta| delta.abs() > 1e-9)
}
