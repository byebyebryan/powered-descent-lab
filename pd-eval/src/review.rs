use super::*;

pub(super) fn derive_run_review_metrics(
    scenario: &ScenarioSpec,
    artifacts: &ControlledRunArtifacts,
) -> BatchRunReviewMetrics {
    let run = &artifacts.run;
    let fuel_used_pct_of_max = (scenario.vehicle.max_fuel_kg > 1e-9)
        .then(|| (run.manifest.summary.fuel_used_kg / scenario.vehicle.max_fuel_kg) * 100.0);
    let landing_offset_abs_m = run
        .manifest
        .summary
        .landing
        .as_ref()
        .map(|landing| landing.touchdown_center_offset_m.abs());
    let (reference_gap_mean_m, reference_gap_max_m) = reference_gap_metrics(scenario, &run.samples)
        .map(|metrics| (Some(metrics.gap_mean_m), Some(metrics.gap_max_m)))
        .unwrap_or((None, None));
    let (low_altitude_dwell_s, low_altitude_unsafe_recovery_s) =
        low_altitude_recovery_metrics(scenario, &run.samples)
            .map(|metrics| {
                (
                    Some(metrics.low_altitude_dwell_s),
                    Some(metrics.low_altitude_unsafe_recovery_s),
                )
            })
            .unwrap_or((None, None));
    let transfer = transfer_review_metrics(&artifacts.controller_updates, &run.samples);
    let mut waypoint_history = waypoint_review_history(&artifacts.controller_updates);
    let waypoint_goal = waypoint_handoff_goal_review_metrics(
        scenario,
        &run.samples,
        &run.manifest,
        &artifacts.controller_updates,
    );
    if matches!(
        scenario.mission.goal,
        EvaluationGoal::WaypointHandoff { .. }
    ) {
        if let Some(terminal) = waypoint_goal.as_ref().filter(|metrics| {
            matches!(
                metrics.capture_status.as_deref(),
                Some("captured" | "missed")
            )
        }) {
            waypoint_history = vec![terminal.clone()];
        }
    } else if matches!(
        scenario.mission.goal,
        EvaluationGoal::WaypointSequence { .. }
    ) && matches!(
        run.manifest.end_reason,
        EndReason::CheckpointSatisfied | EndReason::CheckpointFailed
    ) {
        let terminal_index = run
            .manifest
            .summary
            .waypoint_sequence
            .as_ref()
            .and_then(|summary| {
                summary.first_failed_index.or_else(|| {
                    (summary.passed_handoffs == summary.total_handoffs
                        && summary.total_handoffs > 0)
                        .then_some(summary.total_handoffs - 1)
                })
            });
        if let Some(terminal_index) = terminal_index
            && !waypoint_history
                .iter()
                .any(|metrics| metrics.active_index == i64::try_from(terminal_index).ok())
            && let Some(terminal) = terminal_waypoint_review_metrics(
                scenario,
                &run.samples,
                &run.manifest,
                terminal_index,
                &artifacts.controller_updates,
            )
        {
            waypoint_history.push(terminal);
        }
    }
    waypoint_history.sort_by_key(|metrics| metrics.active_index);
    waypoint_history.dedup_by_key(|metrics| metrics.active_index);
    enrich_waypoint_candidate_history(
        scenario,
        &artifacts.controller_updates,
        &mut waypoint_history,
    );
    let waypoint = waypoint_history
        .first()
        .cloned()
        .or(waypoint_goal)
        .unwrap_or_else(|| waypoint_review_metrics(&artifacts.controller_updates));
    let waypoint_contract = waypoint_contract_review_metrics(scenario, &waypoint);
    let waypoint_handoffs = waypoint_history
        .iter()
        .filter_map(|metrics| batch_waypoint_handoff_metrics(scenario, metrics))
        .collect::<Vec<_>>();
    let waypoint_route = waypoint_route_review_metrics(scenario, &run.manifest, &waypoint_handoffs);
    let transfer_shape =
        transfer_shape_metrics(scenario, &run.samples, &artifacts.controller_updates);

    BatchRunReviewMetrics {
        fuel_used_pct_of_max,
        landing_offset_abs_m,
        low_altitude_dwell_s,
        low_altitude_unsafe_recovery_s,
        reference_gap_mean_m,
        reference_gap_max_m,
        transfer_shape_curve_rmse_m: transfer_shape.map(|metrics| metrics.curve_rmse_m),
        transfer_shape_apex_error_m: transfer_shape.map(|metrics| metrics.apex_error_m),
        transfer_shape_projected_dx_abs_mean_m: transfer_shape
            .and_then(|metrics| metrics.projected_dx_abs_mean_m),
        transfer_shape_projected_dx_abs_max_m: transfer_shape
            .and_then(|metrics| metrics.projected_dx_abs_max_m),
        transfer_shape_shortfall_ratio: transfer_shape.and_then(|metrics| metrics.shortfall_ratio),
        transfer_terminal_entry_kind: transfer.terminal_entry_kind,
        transfer_terminal_handoff_time_s: transfer.terminal_handoff_time_s,
        transfer_terminal_handoff_dx_m: transfer.terminal_handoff_dx_m,
        transfer_terminal_handoff_height_m: transfer.terminal_handoff_height_m,
        transfer_terminal_handoff_speed_mps: transfer.terminal_handoff_speed_mps,
        transfer_terminal_handoff_gate_mode: transfer.terminal_handoff_gate_mode,
        transfer_terminal_handoff_projected_dx_m: transfer.terminal_handoff_projected_dx_m,
        transfer_terminal_handoff_impact_angle_deg: transfer.terminal_handoff_impact_angle_deg,
        transfer_terminal_handoff_boost_quality: transfer.terminal_handoff_boost_quality,
        transfer_terminal_handoff_latest_safe_margin_s: transfer
            .terminal_handoff_latest_safe_margin_s,
        transfer_terminal_handoff_required_accel_ratio: transfer
            .terminal_handoff_required_accel_ratio,
        transfer_terminal_post_handoff_apex_gain_m: transfer.terminal_post_handoff_apex_gain_m,
        transfer_terminal_post_handoff_time_to_apex_s: transfer
            .terminal_post_handoff_time_to_apex_s,
        transfer_terminal_post_handoff_apex_dx_abs_m: transfer.terminal_post_handoff_apex_dx_abs_m,
        transfer_terminal_low_altitude_rebound_gain_m: transfer
            .terminal_low_altitude_rebound_gain_m,
        transfer_terminal_low_altitude_rebound_origin_dx_abs_m: transfer
            .terminal_low_altitude_rebound_origin_dx_abs_m,
        transfer_terminal_low_altitude_rebound_near_pad: transfer
            .terminal_low_altitude_rebound_near_pad,
        transfer_final_phase: transfer.final_phase,
        transfer_boost_projected_dx_m: transfer.boost_projected_dx_m,
        transfer_boost_impact_angle_deg: transfer.boost_impact_angle_deg,
        transfer_boost_apex_over_target_m: transfer.boost_apex_over_target_m,
        transfer_boost_quality: transfer.boost_quality,
        transfer_boost_selected_score: transfer.boost_selected_score,
        transfer_boost_settled_quality: transfer.boost_settled_quality,
        transfer_boost_settled_projected_dx_m: transfer.boost_settled_projected_dx_m,
        transfer_boost_cutoff_time_s: transfer.boost_cutoff_time_s,
        transfer_boost_cutoff_projected_dx_m: transfer.boost_cutoff_projected_dx_m,
        transfer_boost_cutoff_impact_angle_deg: transfer.boost_cutoff_impact_angle_deg,
        transfer_boost_cutoff_apex_over_target_m: transfer.boost_cutoff_apex_over_target_m,
        transfer_boost_cutoff_quality: transfer.boost_cutoff_quality,
        transfer_boost_burn_duration_s: transfer.boost_burn_duration_s,
        transfer_boost_burn_fuel_used_kg: transfer.boost_burn_fuel_used_kg,
        transfer_boost_burn_avg_throttle: transfer.boost_burn_avg_throttle,
        transfer_terminal_gate_mode: transfer.terminal_gate_mode,
        transfer_terminal_gate_latest_safe_margin_s: transfer.terminal_gate_latest_safe_margin_s,
        transfer_terminal_gate_required_accel_ratio: transfer.terminal_gate_required_accel_ratio,
        transfer_terminal_gate_deferred: transfer.terminal_gate_deferred,
        transfer_corridor_mode: transfer.corridor_mode,
        transfer_corridor_min_margin_m: transfer.corridor_min_margin_m,
        waypoint_capture_status: waypoint.capture_status,
        waypoint_contract_status: waypoint_contract.status,
        waypoint_contract_reasons: waypoint_contract.reasons,
        waypoint_active_index: waypoint.active_index,
        waypoint_capture_time_s: waypoint.capture_time_s,
        waypoint_window_entry: waypoint.window_entry.clone(),
        waypoint_handoff_resolution_reason: waypoint.resolution_reason.clone(),
        waypoint_handoff_window_duration_s: waypoint.window_duration_s,
        waypoint_closest_distance_m: waypoint.closest_distance_m,
        waypoint_distance_m: waypoint.distance_m,
        waypoint_cross_track_m: waypoint.cross_track_m,
        waypoint_plane_progress_m: waypoint.plane_progress_m,
        waypoint_outbound_heading_error_rad: waypoint.outbound_heading_error_rad,
        waypoint_outbound_progress_mps: waypoint.outbound_progress_mps,
        waypoint_outbound_cross_speed_mps: waypoint.outbound_cross_speed_mps,
        waypoint_speed_mps: waypoint.speed_mps,
        waypoint_vertical_speed_mps: waypoint.vertical_speed_mps,
        waypoint_remaining_to_plane_m: waypoint.remaining_to_plane_m,
        waypoint_time_to_plane_s: waypoint.time_to_plane_s,
        waypoint_required_turn_distance_m: waypoint.required_turn_distance_m,
        waypoint_shaping_start_distance_m: waypoint.shaping_start_distance_m,
        waypoint_turn_margin_m: waypoint.turn_margin_m,
        waypoint_handoffs,
        waypoint_route_status: waypoint_route.status,
        waypoint_route_passed: waypoint_route.passed,
        waypoint_route_total: waypoint_route.total,
        waypoint_route_first_failure_index: waypoint_route.first_failure_index,
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct WaypointReviewMetrics {
    pub(super) capture_status: Option<String>,
    pub(super) active_index: Option<i64>,
    pub(super) capture_time_s: Option<f64>,
    pub(super) window_entry: Option<BatchWaypointWindowEntryReviewMetrics>,
    pub(super) resolution_reason: Option<String>,
    pub(super) window_duration_s: Option<f64>,
    pub(super) closest_distance_m: Option<f64>,
    pub(super) distance_m: Option<f64>,
    pub(super) cross_track_m: Option<f64>,
    pub(super) plane_progress_m: Option<f64>,
    pub(super) outbound_heading_error_rad: Option<f64>,
    pub(super) outbound_progress_mps: Option<f64>,
    pub(super) outbound_cross_speed_mps: Option<f64>,
    pub(super) speed_mps: Option<f64>,
    pub(super) vertical_speed_mps: Option<f64>,
    pub(super) remaining_to_plane_m: Option<f64>,
    pub(super) time_to_plane_s: Option<f64>,
    pub(super) required_turn_distance_m: Option<f64>,
    pub(super) shaping_start_distance_m: Option<f64>,
    pub(super) turn_margin_m: Option<f64>,
    pub(super) center_x_m: Option<f64>,
    pub(super) center_y_m: Option<f64>,
    pub(super) nominal_handoff_target_x_m: Option<f64>,
    pub(super) nominal_handoff_target_y_m: Option<f64>,
    pub(super) handoff_target_x_m: Option<f64>,
    pub(super) handoff_target_y_m: Option<f64>,
    pub(super) handoff_target_mode: Option<String>,
    pub(super) remaining_to_handoff_m: Option<f64>,
    pub(super) time_to_handoff_s: Option<f64>,
    pub(super) target_vx_mps: Option<f64>,
    pub(super) target_vy_mps: Option<f64>,
    pub(super) target_deadline_remaining_s: Option<f64>,
    pub(super) target_velocity_error_mps: Option<f64>,
    pub(super) guidance_feasible: Option<bool>,
    pub(super) final_terminal_required_accel_ratio: Option<f64>,
    pub(super) final_terminal_recoverable: Option<bool>,
    pub(super) predicted_handoff_time_to_go_s: Option<f64>,
    pub(super) predicted_handoff_deadline_lead_s: Option<f64>,
    pub(super) predicted_handoff_contract_status: Option<String>,
    pub(super) predicted_handoff_contract_reasons: Vec<String>,
    pub(super) predicted_handoff_distance_m: Option<f64>,
    pub(super) predicted_handoff_cross_track_m: Option<f64>,
    pub(super) predicted_handoff_plane_progress_m: Option<f64>,
    pub(super) predicted_handoff_outbound_heading_error_rad: Option<f64>,
    pub(super) predicted_handoff_outbound_progress_mps: Option<f64>,
    pub(super) predicted_handoff_outbound_cross_speed_mps: Option<f64>,
    pub(super) predicted_handoff_speed_mps: Option<f64>,
    pub(super) predicted_handoff_vertical_speed_mps: Option<f64>,
    pub(super) candidate_contract_pass_ever: Option<bool>,
    pub(super) candidate_first_pass_time_s: Option<f64>,
    pub(super) candidate_last_pass_time_s: Option<f64>,
    pub(super) candidate_pass_lost_before_capture: Option<bool>,
    pub(super) candidate_best_heading_margin_rad: Option<f64>,
    pub(super) candidate_best_cross_speed_margin_mps: Option<f64>,
    pub(super) reachable_candidate_contract_pass_ever: Option<bool>,
    pub(super) reachable_candidate_first_pass_time_s: Option<f64>,
    pub(super) reachable_candidate_last_pass_time_s: Option<f64>,
    pub(super) reachable_candidate_pass_lost_before_capture: Option<bool>,
    pub(super) reachable_required_accel_ratio_max: Option<f64>,
    pub(super) reachable_thrust_saturated_time_max_s: Option<f64>,
    pub(super) reachable_tilt_saturated_time_max_s: Option<f64>,
    pub(super) continuation_next_waypoint_index: Option<usize>,
    pub(super) continuation_contract_pass: Option<bool>,
    pub(super) continuation_contract_reasons: Vec<String>,
    pub(super) continuation_outbound_heading_error_rad: Option<f64>,
    pub(super) continuation_required_accel_ratio_max: Option<f64>,
    pub(super) continuation_passing_candidate_count: Option<usize>,
    pub(super) transition_next_waypoint_index: Option<usize>,
    pub(super) transition_position_error_m: Option<f64>,
    pub(super) transition_velocity_error_mps: Option<f64>,
    pub(super) transition_attitude_error_rad: Option<f64>,
    pub(super) transition_mass_error_kg: Option<f64>,
    pub(super) transition_fuel_error_kg: Option<f64>,
    pub(super) transition_event_time_error_s: Option<f64>,
    pub(super) transition_continuation_contract_pass: Option<bool>,
    pub(super) transition_continuation_contract_reasons: Vec<String>,
    pub(super) transition_continuation_outbound_heading_error_rad: Option<f64>,
    pub(super) transition_continuation_required_accel_ratio_max: Option<f64>,
    pub(super) transition_continuation_passing_candidate_count: Option<usize>,
    pub(super) joint_next_waypoint_index: Option<usize>,
    pub(super) joint_evaluated_candidate_count: Option<usize>,
    pub(super) joint_passing_candidate_count: Option<usize>,
    pub(super) joint_contract_pass: Option<bool>,
    pub(super) joint_endpoint_x_m: Option<f64>,
    pub(super) joint_endpoint_y_m: Option<f64>,
    pub(super) joint_target_vx_mps: Option<f64>,
    pub(super) joint_target_vy_mps: Option<f64>,
    pub(super) joint_time_to_go_s: Option<f64>,
    pub(super) joint_continuation_outbound_heading_error_rad: Option<f64>,
    pub(super) joint_required_accel_ratio_max: Option<f64>,
    pub(super) joint_total_saturated_time_s: Option<f64>,
    pub(super) joint_continuation_passing_candidate_count: Option<usize>,
    pub(super) plan_reference_position_error_max_m: Option<f64>,
    pub(super) plan_reference_cross_error_max_abs_m: Option<f64>,
    pub(super) plan_reference_velocity_error_max_mps: Option<f64>,
    pub(super) plan_reference_cross_speed_error_max_abs_mps: Option<f64>,
    pub(super) guidance_required_accel_ratio_max: Option<f64>,
    pub(super) guidance_thrust_saturated_time_s: Option<f64>,
    pub(super) guidance_tilt_saturated_time_s: Option<f64>,
    pub(super) guidance_first_saturation_lead_s: Option<f64>,
    pub(super) last_pass_reference_position_error_m: Option<f64>,
    pub(super) last_pass_reference_velocity_error_mps: Option<f64>,
    pub(super) last_pass_required_accel_ratio: Option<f64>,
    pub(super) guidance_plan_revision_max: Option<i64>,
    pub(super) guidance_plan_reasons: Vec<String>,
    pub(super) handoff_turn_margin_m: Option<f64>,
    pub(super) guidance_snapshot_source: Option<String>,
    pub(super) guidance_snapshot_age_s: Option<f64>,
    pub(super) guidance_replan_count: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct WaypointContractReviewMetrics {
    pub(super) status: Option<String>,
    pub(super) reasons: Vec<String>,
}

pub(super) fn waypoint_review_metrics(
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> WaypointReviewMetrics {
    let update = controller_updates
        .iter()
        .find(|update| {
            matches!(
                telemetry_text(&update.frame.metrics, metric::WAYPOINT_CAPTURE_STATUS),
                Some("captured" | "missed")
            )
        })
        .or_else(|| {
            controller_updates.iter().rev().find(|update| {
                telemetry_bool(&update.frame.metrics, metric::WAYPOINT_GUIDANCE_ENABLED)
                    == Some(true)
            })
        });
    let Some(update) = update else {
        return WaypointReviewMetrics::default();
    };
    waypoint_review_metrics_from_update(update)
}

pub(super) fn waypoint_review_history(
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> Vec<WaypointReviewMetrics> {
    let mut handoffs = controller_updates
        .iter()
        .filter(|update| {
            update
                .frame
                .markers
                .iter()
                .any(|candidate| candidate.id == marker::WAYPOINT_HANDOFF)
        })
        .map(waypoint_review_metrics_from_update)
        .collect::<Vec<_>>();
    if handoffs.is_empty() {
        handoffs = controller_updates
            .iter()
            .filter(|update| {
                matches!(
                    telemetry_text(&update.frame.metrics, metric::WAYPOINT_CAPTURE_STATUS),
                    Some("captured" | "missed")
                )
            })
            .map(waypoint_review_metrics_from_update)
            .collect();
    }
    handoffs.sort_by_key(|handoff| handoff.active_index);
    handoffs.dedup_by_key(|handoff| handoff.active_index);
    handoffs
}

pub(super) fn enrich_waypoint_candidate_history(
    scenario: &ScenarioSpec,
    controller_updates: &[pd_control::ControllerUpdateRecord],
    handoffs: &mut [WaypointReviewMetrics],
) {
    let Some(route) = scenario.mission.transfer_route.as_ref() else {
        return;
    };

    for handoff in handoffs {
        let Some(index) = handoff
            .active_index
            .and_then(|index| usize::try_from(index).ok())
        else {
            continue;
        };
        let Some(waypoint) = route.waypoints.get(index) else {
            continue;
        };
        let expected_index = i64::try_from(index).ok();
        let updates = controller_updates
            .iter()
            .filter(|update| {
                telemetry_bool(&update.frame.metrics, metric::WAYPOINT_GUIDANCE_ENABLED)
                    == Some(true)
                    && telemetry_integer(
                        &update.frame.metrics,
                        metric::WAYPOINT_GUIDANCE_PLAN_INDEX,
                    )
                    .or_else(|| {
                        telemetry_integer(&update.frame.metrics, metric::WAYPOINT_ACTIVE_INDEX)
                    }) == expected_index
            })
            .collect::<Vec<_>>();

        let mut observed = false;
        let mut pass_ever = false;
        let mut first_pass_time_s = None;
        let mut last_pass_time_s = None;
        let mut last_observed_pass = false;
        let mut best_heading_margin_rad: Option<f64> = None;
        let mut best_cross_speed_margin_mps: Option<f64> = None;
        let mut reference_position_error_max_m: Option<f64> = None;
        let mut reference_cross_error_max_abs_m: Option<f64> = None;
        let mut reference_velocity_error_max_mps: Option<f64> = None;
        let mut reference_cross_speed_error_max_abs_mps: Option<f64> = None;
        let mut required_accel_ratio_max: Option<f64> = None;
        let mut thrust_saturated_time_s = 0.0;
        let mut tilt_saturated_time_s = 0.0;
        let mut first_saturation_time_s: Option<f64> = None;
        let mut trackability_observed = false;
        let mut plan_revision_max: Option<i64> = None;
        let mut plan_reasons = BTreeSet::new();
        let mut last_pass_reference_position_error_m = None;
        let mut last_pass_reference_velocity_error_mps = None;
        let mut last_pass_required_accel_ratio = None;
        let mut reachable_observed = false;
        let mut reachable_pass_ever = false;
        let mut reachable_first_pass_time_s = None;
        let mut reachable_last_pass_time_s = None;
        let mut reachable_last_observed_pass = false;
        let mut reachable_required_accel_ratio_max: Option<f64> = None;
        let mut reachable_thrust_saturated_time_max_s: Option<f64> = None;
        let mut reachable_tilt_saturated_time_max_s: Option<f64> = None;
        let mut continuation_observed = false;
        let mut continuation_next_waypoint_index = None;
        let mut continuation_contract_pass = None;
        let mut continuation_contract_reasons = Vec::new();
        let mut continuation_outbound_heading_error_rad = None;
        let mut continuation_required_accel_ratio_max = None;
        let mut continuation_passing_candidate_count = None;

        for (update_index, update) in updates.iter().enumerate() {
            let metrics = &update.frame.metrics;
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_GUIDANCE_REFERENCE_POSITION_ERROR_M,
            ) {
                trackability_observed = true;
                update_optional_max(&mut reference_position_error_max_m, value);
            }
            if let Some(value) =
                telemetry_float(metrics, metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_ERROR_M)
            {
                update_optional_max(&mut reference_cross_error_max_abs_m, value.abs());
            }
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_GUIDANCE_REFERENCE_VELOCITY_ERROR_MPS,
            ) {
                update_optional_max(&mut reference_velocity_error_max_mps, value);
            }
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_SPEED_ERROR_MPS,
            ) {
                update_optional_max(&mut reference_cross_speed_error_max_abs_mps, value.abs());
            }
            if let Some(value) =
                telemetry_float(metrics, metric::WAYPOINT_GUIDANCE_REQUIRED_ACCEL_RATIO)
            {
                update_optional_max(&mut required_accel_ratio_max, value);
            }
            if let Some(revision) =
                telemetry_integer(metrics, metric::WAYPOINT_GUIDANCE_PLAN_REVISION)
            {
                plan_revision_max =
                    Some(plan_revision_max.map_or(revision, |max| max.max(revision)));
            }
            if let Some(reason) = telemetry_text(metrics, metric::WAYPOINT_GUIDANCE_PLAN_REASON) {
                plan_reasons.insert(reason.to_owned());
            }
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_REACHABLE_HANDOFF_REQUIRED_ACCEL_RATIO_MAX,
            ) {
                update_optional_max(&mut reachable_required_accel_ratio_max, value);
            }
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_REACHABLE_HANDOFF_THRUST_SATURATED_TIME_S,
            ) {
                update_optional_max(&mut reachable_thrust_saturated_time_max_s, value);
            }
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_REACHABLE_HANDOFF_TILT_SATURATED_TIME_S,
            ) {
                update_optional_max(&mut reachable_tilt_saturated_time_max_s, value);
            }
            if let Some(passed) =
                telemetry_bool(metrics, metric::WAYPOINT_CONTINUATION_CONTRACT_PASS)
            {
                continuation_observed = true;
                continuation_contract_pass = Some(passed);
                continuation_next_waypoint_index =
                    telemetry_integer(metrics, metric::WAYPOINT_CONTINUATION_NEXT_INDEX)
                        .and_then(|index| usize::try_from(index).ok());
                continuation_contract_reasons =
                    telemetry_text(metrics, metric::WAYPOINT_CONTINUATION_CONTRACT_REASONS)
                        .map(|reasons| {
                            reasons
                                .split(',')
                                .filter(|reason| !reason.is_empty())
                                .map(ToOwned::to_owned)
                                .collect()
                        })
                        .unwrap_or_default();
                continuation_outbound_heading_error_rad = telemetry_float(
                    metrics,
                    metric::WAYPOINT_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD,
                );
                continuation_required_accel_ratio_max = telemetry_float(
                    metrics,
                    metric::WAYPOINT_CONTINUATION_REQUIRED_ACCEL_RATIO_MAX,
                );
                continuation_passing_candidate_count = telemetry_integer(
                    metrics,
                    metric::WAYPOINT_CONTINUATION_PASSING_CANDIDATE_COUNT,
                )
                .and_then(|count| usize::try_from(count).ok());
            }
            if let Some(passed) =
                telemetry_bool(metrics, metric::WAYPOINT_REACHABLE_HANDOFF_CONTRACT_PASS)
            {
                reachable_observed = true;
                reachable_last_observed_pass = passed;
                if passed {
                    reachable_pass_ever = true;
                    reachable_first_pass_time_s.get_or_insert(update.sim_time_s);
                    reachable_last_pass_time_s = Some(update.sim_time_s);
                }
            }

            let thrust_saturated =
                telemetry_bool(metrics, metric::WAYPOINT_GUIDANCE_THRUST_SATURATED) == Some(true);
            let tilt_saturated =
                telemetry_bool(metrics, metric::WAYPOINT_GUIDANCE_TILT_SATURATED) == Some(true);
            if thrust_saturated || tilt_saturated {
                first_saturation_time_s.get_or_insert(update.sim_time_s);
            }
            let interval_s = updates
                .get(update_index + 1)
                .map(|next| (next.sim_time_s - update.sim_time_s).max(0.0))
                .unwrap_or(0.0);
            if thrust_saturated {
                thrust_saturated_time_s += interval_s;
            }
            if tilt_saturated {
                tilt_saturated_time_s += interval_s;
            }

            let Some(passed) = telemetry_bool(
                &update.frame.metrics,
                metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS,
            ) else {
                continue;
            };
            observed = true;
            last_observed_pass = passed;
            if passed {
                pass_ever = true;
                first_pass_time_s.get_or_insert(update.sim_time_s);
                last_pass_time_s = Some(update.sim_time_s);
                last_pass_reference_position_error_m = telemetry_float(
                    metrics,
                    metric::WAYPOINT_GUIDANCE_REFERENCE_POSITION_ERROR_M,
                );
                last_pass_reference_velocity_error_mps = telemetry_float(
                    metrics,
                    metric::WAYPOINT_GUIDANCE_REFERENCE_VELOCITY_ERROR_MPS,
                );
                last_pass_required_accel_ratio =
                    telemetry_float(metrics, metric::WAYPOINT_GUIDANCE_REQUIRED_ACCEL_RATIO);
            }

            if let Some(error_rad) = telemetry_float(
                &update.frame.metrics,
                metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_HEADING_ERROR_RAD,
            ) {
                let margin = waypoint.max_outbound_heading_error_rad - error_rad;
                best_heading_margin_rad =
                    Some(best_heading_margin_rad.map_or(margin, |best| best.max(margin)));
            }
            if let Some(max_cross_speed_mps) = waypoint.max_outbound_cross_speed_mps
                && let Some(cross_speed_mps) = telemetry_float(
                    &update.frame.metrics,
                    metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_CROSS_SPEED_MPS,
                )
            {
                let margin = max_cross_speed_mps - cross_speed_mps;
                best_cross_speed_margin_mps =
                    Some(best_cross_speed_margin_mps.map_or(margin, |best| best.max(margin)));
            }
        }

        if observed {
            handoff.candidate_contract_pass_ever = Some(pass_ever);
            handoff.candidate_first_pass_time_s = first_pass_time_s;
            handoff.candidate_last_pass_time_s = last_pass_time_s;
            handoff.candidate_pass_lost_before_capture = Some(pass_ever && !last_observed_pass);
            handoff.candidate_best_heading_margin_rad = best_heading_margin_rad;
            handoff.candidate_best_cross_speed_margin_mps = best_cross_speed_margin_mps;
        }
        if reachable_observed {
            handoff.reachable_candidate_contract_pass_ever = Some(reachable_pass_ever);
            handoff.reachable_candidate_first_pass_time_s = reachable_first_pass_time_s;
            handoff.reachable_candidate_last_pass_time_s = reachable_last_pass_time_s;
            handoff.reachable_candidate_pass_lost_before_capture =
                Some(reachable_pass_ever && !reachable_last_observed_pass);
            handoff.reachable_required_accel_ratio_max = reachable_required_accel_ratio_max;
            handoff.reachable_thrust_saturated_time_max_s = reachable_thrust_saturated_time_max_s;
            handoff.reachable_tilt_saturated_time_max_s = reachable_tilt_saturated_time_max_s;
        }
        if continuation_observed {
            handoff.continuation_next_waypoint_index = continuation_next_waypoint_index;
            handoff.continuation_contract_pass = continuation_contract_pass;
            handoff.continuation_contract_reasons = continuation_contract_reasons;
            handoff.continuation_outbound_heading_error_rad =
                continuation_outbound_heading_error_rad;
            handoff.continuation_required_accel_ratio_max = continuation_required_accel_ratio_max;
            handoff.continuation_passing_candidate_count = continuation_passing_candidate_count;
        }
        if trackability_observed {
            handoff.plan_reference_position_error_max_m = reference_position_error_max_m;
            handoff.plan_reference_cross_error_max_abs_m = reference_cross_error_max_abs_m;
            handoff.plan_reference_velocity_error_max_mps = reference_velocity_error_max_mps;
            handoff.plan_reference_cross_speed_error_max_abs_mps =
                reference_cross_speed_error_max_abs_mps;
            handoff.guidance_required_accel_ratio_max = required_accel_ratio_max;
            handoff.guidance_thrust_saturated_time_s = Some(thrust_saturated_time_s);
            handoff.guidance_tilt_saturated_time_s = Some(tilt_saturated_time_s);
            handoff.guidance_first_saturation_lead_s = first_saturation_time_s
                .zip(handoff.capture_time_s)
                .map(|(first_saturation, capture)| (capture - first_saturation).max(0.0));
            handoff.last_pass_reference_position_error_m = last_pass_reference_position_error_m;
            handoff.last_pass_reference_velocity_error_mps = last_pass_reference_velocity_error_mps;
            handoff.last_pass_required_accel_ratio = last_pass_required_accel_ratio;
            handoff.guidance_plan_revision_max = plan_revision_max;
            handoff.guidance_plan_reasons = plan_reasons.into_iter().collect();
        }
    }
}

pub(super) fn update_optional_max(current: &mut Option<f64>, value: f64) {
    *current = Some(current.map_or(value, |maximum| maximum.max(value)));
}

pub(super) fn waypoint_review_metrics_from_update(
    update: &pd_control::ControllerUpdateRecord,
) -> WaypointReviewMetrics {
    let capture_time_s = telemetry_float(&update.frame.metrics, metric::WAYPOINT_CAPTURE_TIME_S)
        .filter(|value| *value >= 0.0);
    let handoff_marker = update
        .frame
        .markers
        .iter()
        .find(|candidate| candidate.id == marker::WAYPOINT_HANDOFF);
    let preferred_metrics = handoff_marker
        .map(|handoff| &handoff.metadata)
        .unwrap_or(&update.frame.metrics);
    let preferred_float = |key| {
        telemetry_float(preferred_metrics, key)
            .or_else(|| telemetry_float(&update.frame.metrics, key))
    };
    let preferred_text = |key| {
        telemetry_text(preferred_metrics, key)
            .or_else(|| telemetry_text(&update.frame.metrics, key))
    };
    let preferred_bool = |key| {
        telemetry_bool(preferred_metrics, key)
            .or_else(|| telemetry_bool(&update.frame.metrics, key))
    };
    let preferred_integer = |key| {
        telemetry_integer(preferred_metrics, key)
            .or_else(|| telemetry_integer(&update.frame.metrics, key))
    };
    let preferred_reasons = |key| {
        preferred_text(key)
            .map(|reasons| {
                reasons
                    .split(',')
                    .filter(|reason| !reason.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let window_entry = preferred_float(metric::WAYPOINT_WINDOW_ENTRY_TIME_S).map(|time_s| {
        BatchWaypointWindowEntryReviewMetrics {
            time_s: Some(time_s),
            position_x_m: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_POSITION_X_M),
            position_y_m: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_POSITION_Y_M),
            velocity_x_mps: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_VELOCITY_X_MPS),
            velocity_y_mps: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_VELOCITY_Y_MPS),
            distance_m: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_DISTANCE_M),
            cross_track_m: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_CROSS_TRACK_M),
            plane_progress_m: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_PLANE_PROGRESS_M),
            handoff_heading_error_rad: preferred_float(
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_HEADING_ERROR_RAD,
            ),
            handoff_progress_mps: preferred_float(
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_PROGRESS_MPS,
            ),
            handoff_cross_speed_mps: preferred_float(
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_CROSS_SPEED_MPS,
            ),
            speed_mps: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_SPEED_MPS),
            vertical_speed_mps: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_VERTICAL_SPEED_MPS),
            contract_pass: preferred_bool(metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_PASS),
            contract_reasons: preferred_reasons(metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_REASONS),
        }
    });
    let predicted_handoff_contract_reasons =
        preferred_text(metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_REASONS)
            .map(|reasons| {
                reasons
                    .split(',')
                    .filter(|reason| !reason.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
    let continuation_contract_reasons =
        preferred_text(metric::WAYPOINT_CONTINUATION_CONTRACT_REASONS)
            .map(|reasons| {
                reasons
                    .split(',')
                    .filter(|reason| !reason.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
    let transition_continuation_contract_reasons =
        preferred_text(metric::WAYPOINT_TRANSITION_CONTINUATION_CONTRACT_REASONS)
            .map(|reasons| {
                reasons
                    .split(',')
                    .filter(|reason| !reason.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
    WaypointReviewMetrics {
        capture_status: telemetry_text(&update.frame.metrics, metric::WAYPOINT_CAPTURE_STATUS)
            .map(ToOwned::to_owned),
        active_index: telemetry_integer(&update.frame.metrics, metric::WAYPOINT_ACTIVE_INDEX),
        capture_time_s,
        window_entry,
        resolution_reason: preferred_text(metric::WAYPOINT_HANDOFF_RESOLUTION_REASON)
            .map(ToOwned::to_owned),
        window_duration_s: preferred_float(metric::WAYPOINT_HANDOFF_WINDOW_DURATION_S)
            .filter(|duration| *duration >= 0.0),
        closest_distance_m: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_CLOSEST_DISTANCE_M,
        )
        .filter(|value| *value >= 0.0),
        distance_m: telemetry_float(&update.frame.metrics, metric::WAYPOINT_DISTANCE_M)
            .filter(|value| *value >= 0.0),
        cross_track_m: telemetry_float(&update.frame.metrics, metric::WAYPOINT_CROSS_TRACK_M)
            .filter(|value| *value >= 0.0),
        plane_progress_m: telemetry_float(&update.frame.metrics, metric::WAYPOINT_PLANE_PROGRESS_M),
        outbound_heading_error_rad: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_OUTBOUND_HEADING_ERROR_RAD,
        )
        .filter(|value| *value >= 0.0),
        outbound_progress_mps: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_OUTBOUND_PROGRESS_MPS,
        ),
        outbound_cross_speed_mps: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_OUTBOUND_CROSS_SPEED_MPS,
        )
        .filter(|value| *value >= 0.0),
        speed_mps: telemetry_float(&update.frame.metrics, metric::WAYPOINT_SPEED_MPS)
            .filter(|value| *value >= 0.0),
        vertical_speed_mps: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_VERTICAL_SPEED_MPS,
        ),
        remaining_to_plane_m: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_REMAINING_TO_PLANE_M,
        )
        .filter(|value| *value >= 0.0),
        time_to_plane_s: telemetry_float(&update.frame.metrics, metric::WAYPOINT_TIME_TO_PLANE_S)
            .filter(|value| *value >= 0.0 && value.is_finite()),
        required_turn_distance_m: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_REQUIRED_TURN_DISTANCE_M,
        )
        .filter(|value| *value >= 0.0),
        shaping_start_distance_m: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_SHAPING_START_DISTANCE_M,
        )
        .filter(|value| *value >= 0.0),
        turn_margin_m: telemetry_float(&update.frame.metrics, metric::WAYPOINT_TURN_MARGIN_M),
        center_x_m: preferred_float(metric::WAYPOINT_CENTER_X_M),
        center_y_m: preferred_float(metric::WAYPOINT_CENTER_Y_M),
        nominal_handoff_target_x_m: preferred_float(metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_X_M),
        nominal_handoff_target_y_m: preferred_float(metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_Y_M),
        handoff_target_x_m: preferred_float(metric::WAYPOINT_HANDOFF_TARGET_X_M),
        handoff_target_y_m: preferred_float(metric::WAYPOINT_HANDOFF_TARGET_Y_M),
        handoff_target_mode: preferred_text(metric::WAYPOINT_HANDOFF_TARGET_MODE)
            .map(ToOwned::to_owned),
        remaining_to_handoff_m: preferred_float(metric::WAYPOINT_REMAINING_TO_HANDOFF_M)
            .filter(|value| *value >= 0.0),
        time_to_handoff_s: preferred_float(metric::WAYPOINT_TIME_TO_HANDOFF_S)
            .filter(|value| *value >= 0.0 && value.is_finite()),
        target_vx_mps: preferred_float(metric::WAYPOINT_TARGET_VX_MPS),
        target_vy_mps: preferred_float(metric::WAYPOINT_TARGET_VY_MPS),
        target_deadline_remaining_s: preferred_float(metric::WAYPOINT_TARGET_DEADLINE_REMAINING_S),
        target_velocity_error_mps: preferred_float(metric::WAYPOINT_TARGET_VELOCITY_ERROR_MPS)
            .filter(|value| *value >= 0.0),
        guidance_feasible: preferred_bool(metric::WAYPOINT_GUIDANCE_FEASIBLE),
        final_terminal_required_accel_ratio: preferred_float(
            metric::WAYPOINT_FINAL_TERMINAL_REQUIRED_ACCEL_RATIO,
        )
        .filter(|value| *value >= 0.0),
        final_terminal_recoverable: preferred_bool(metric::WAYPOINT_FINAL_TERMINAL_RECOVERABLE),
        predicted_handoff_time_to_go_s: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_TIME_TO_GO_S,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_deadline_lead_s: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_DEADLINE_LEAD_S,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_contract_status: preferred_bool(
            metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS,
        )
        .map(|passed| if passed { "pass" } else { "fail" }.to_owned()),
        predicted_handoff_contract_reasons,
        predicted_handoff_distance_m: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_DISTANCE_M,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_cross_track_m: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_CROSS_TRACK_M,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_plane_progress_m: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_PLANE_PROGRESS_M,
        ),
        predicted_handoff_outbound_heading_error_rad: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_HEADING_ERROR_RAD,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_outbound_progress_mps: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_PROGRESS_MPS,
        ),
        predicted_handoff_outbound_cross_speed_mps: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_CROSS_SPEED_MPS,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_speed_mps: preferred_float(metric::WAYPOINT_PREDICTED_HANDOFF_SPEED_MPS)
            .filter(|value| *value >= 0.0),
        predicted_handoff_vertical_speed_mps: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_VERTICAL_SPEED_MPS,
        ),
        candidate_contract_pass_ever: None,
        candidate_first_pass_time_s: None,
        candidate_last_pass_time_s: None,
        candidate_pass_lost_before_capture: None,
        candidate_best_heading_margin_rad: None,
        candidate_best_cross_speed_margin_mps: None,
        reachable_candidate_contract_pass_ever: None,
        reachable_candidate_first_pass_time_s: None,
        reachable_candidate_last_pass_time_s: None,
        reachable_candidate_pass_lost_before_capture: None,
        reachable_required_accel_ratio_max: None,
        reachable_thrust_saturated_time_max_s: None,
        reachable_tilt_saturated_time_max_s: None,
        continuation_next_waypoint_index: preferred_integer(
            metric::WAYPOINT_CONTINUATION_NEXT_INDEX,
        )
        .and_then(|index| usize::try_from(index).ok()),
        continuation_contract_pass: preferred_bool(metric::WAYPOINT_CONTINUATION_CONTRACT_PASS),
        continuation_contract_reasons,
        continuation_outbound_heading_error_rad: preferred_float(
            metric::WAYPOINT_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD,
        ),
        continuation_required_accel_ratio_max: preferred_float(
            metric::WAYPOINT_CONTINUATION_REQUIRED_ACCEL_RATIO_MAX,
        ),
        continuation_passing_candidate_count: preferred_integer(
            metric::WAYPOINT_CONTINUATION_PASSING_CANDIDATE_COUNT,
        )
        .and_then(|count| usize::try_from(count).ok()),
        transition_next_waypoint_index: preferred_integer(metric::WAYPOINT_TRANSITION_NEXT_INDEX)
            .and_then(|index| usize::try_from(index).ok()),
        transition_position_error_m: preferred_float(metric::WAYPOINT_TRANSITION_POSITION_ERROR_M),
        transition_velocity_error_mps: preferred_float(
            metric::WAYPOINT_TRANSITION_VELOCITY_ERROR_MPS,
        ),
        transition_attitude_error_rad: preferred_float(
            metric::WAYPOINT_TRANSITION_ATTITUDE_ERROR_RAD,
        ),
        transition_mass_error_kg: preferred_float(metric::WAYPOINT_TRANSITION_MASS_ERROR_KG),
        transition_fuel_error_kg: preferred_float(metric::WAYPOINT_TRANSITION_FUEL_ERROR_KG),
        transition_event_time_error_s: preferred_float(
            metric::WAYPOINT_TRANSITION_EVENT_TIME_ERROR_S,
        ),
        transition_continuation_contract_pass: preferred_bool(
            metric::WAYPOINT_TRANSITION_CONTINUATION_CONTRACT_PASS,
        ),
        transition_continuation_contract_reasons,
        transition_continuation_outbound_heading_error_rad: preferred_float(
            metric::WAYPOINT_TRANSITION_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD,
        ),
        transition_continuation_required_accel_ratio_max: preferred_float(
            metric::WAYPOINT_TRANSITION_CONTINUATION_REQUIRED_ACCEL_RATIO_MAX,
        ),
        transition_continuation_passing_candidate_count: preferred_integer(
            metric::WAYPOINT_TRANSITION_CONTINUATION_PASSING_CANDIDATE_COUNT,
        )
        .and_then(|count| usize::try_from(count).ok()),
        joint_next_waypoint_index: preferred_integer(metric::WAYPOINT_JOINT_NEXT_INDEX)
            .and_then(|index| usize::try_from(index).ok()),
        joint_evaluated_candidate_count: preferred_integer(
            metric::WAYPOINT_JOINT_EVALUATED_CANDIDATE_COUNT,
        )
        .and_then(|count| usize::try_from(count).ok()),
        joint_passing_candidate_count: preferred_integer(
            metric::WAYPOINT_JOINT_PASSING_CANDIDATE_COUNT,
        )
        .and_then(|count| usize::try_from(count).ok()),
        joint_contract_pass: preferred_bool(metric::WAYPOINT_JOINT_CONTRACT_PASS),
        joint_endpoint_x_m: preferred_float(metric::WAYPOINT_JOINT_ENDPOINT_X_M),
        joint_endpoint_y_m: preferred_float(metric::WAYPOINT_JOINT_ENDPOINT_Y_M),
        joint_target_vx_mps: preferred_float(metric::WAYPOINT_JOINT_TARGET_VX_MPS),
        joint_target_vy_mps: preferred_float(metric::WAYPOINT_JOINT_TARGET_VY_MPS),
        joint_time_to_go_s: preferred_float(metric::WAYPOINT_JOINT_TIME_TO_GO_S),
        joint_continuation_outbound_heading_error_rad: preferred_float(
            metric::WAYPOINT_JOINT_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD,
        ),
        joint_required_accel_ratio_max: preferred_float(
            metric::WAYPOINT_JOINT_REQUIRED_ACCEL_RATIO_MAX,
        ),
        joint_total_saturated_time_s: preferred_float(
            metric::WAYPOINT_JOINT_TOTAL_SATURATED_TIME_S,
        ),
        joint_continuation_passing_candidate_count: preferred_integer(
            metric::WAYPOINT_JOINT_CONTINUATION_PASSING_CANDIDATE_COUNT,
        )
        .and_then(|count| usize::try_from(count).ok()),
        plan_reference_position_error_max_m: None,
        plan_reference_cross_error_max_abs_m: None,
        plan_reference_velocity_error_max_mps: None,
        plan_reference_cross_speed_error_max_abs_mps: None,
        guidance_required_accel_ratio_max: None,
        guidance_thrust_saturated_time_s: None,
        guidance_tilt_saturated_time_s: None,
        guidance_first_saturation_lead_s: None,
        last_pass_reference_position_error_m: None,
        last_pass_reference_velocity_error_mps: None,
        last_pass_required_accel_ratio: None,
        guidance_plan_revision_max: None,
        guidance_plan_reasons: Vec::new(),
        handoff_turn_margin_m: preferred_float(metric::WAYPOINT_HANDOFF_TURN_MARGIN_M),
        guidance_snapshot_source: Some(
            if handoff_marker.is_some() {
                "capture_update"
            } else {
                "controller_update"
            }
            .to_owned(),
        ),
        guidance_snapshot_age_s: handoff_marker.is_some().then_some(0.0),
        guidance_replan_count: handoff_marker
            .and_then(|handoff| {
                telemetry_integer(&handoff.metadata, metric::WAYPOINT_GUIDANCE_REPLAN_COUNT)
            })
            .or_else(|| {
                telemetry_integer(
                    &update.frame.metrics,
                    metric::WAYPOINT_GUIDANCE_REPLAN_COUNT,
                )
            }),
    }
}

pub(super) fn waypoint_handoff_goal_review_metrics(
    scenario: &ScenarioSpec,
    samples: &[SampleRecord],
    manifest: &RunManifest,
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> Option<WaypointReviewMetrics> {
    let EvaluationGoal::WaypointHandoff { waypoint_index, .. } = &scenario.mission.goal else {
        return None;
    };
    terminal_waypoint_review_metrics(
        scenario,
        samples,
        manifest,
        *waypoint_index,
        controller_updates,
    )
}

pub(super) fn terminal_waypoint_review_metrics(
    scenario: &ScenarioSpec,
    samples: &[SampleRecord],
    manifest: &RunManifest,
    waypoint_index: usize,
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> Option<WaypointReviewMetrics> {
    let route = scenario.mission.transfer_route.as_ref()?;
    let waypoint = route.waypoints.get(waypoint_index)?;
    let last_sample = samples.last()?;
    let stats = waypoint_sample_stats(scenario, &last_sample.observation, waypoint_index)?;
    let terminal_handoff = matches!(
        manifest.end_reason,
        EndReason::CheckpointSatisfied | EndReason::CheckpointFailed
    );
    let capture_status = if terminal_handoff {
        if waypoint_capture_passes_review(waypoint, &stats) {
            "captured"
        } else {
            "missed"
        }
    } else {
        "tracking"
    };
    let closest_distance_m = samples
        .iter()
        .filter_map(|sample| waypoint_sample_stats(scenario, &sample.observation, waypoint_index))
        .map(|stats| stats.distance_m)
        .min_by(|lhs, rhs| lhs.total_cmp(rhs));
    let sampled_window_entry = samples.iter().find_map(|sample| {
        let entry_stats = waypoint_sample_stats(scenario, &sample.observation, waypoint_index)?;
        (entry_stats.distance_m <= waypoint.capture_radius_m).then(|| {
            let assessment = waypoint.assess_handoff(entry_stats);
            BatchWaypointWindowEntryReviewMetrics {
                time_s: Some(sample.sim_time_s),
                position_x_m: Some(sample.observation.position_m.x),
                position_y_m: Some(sample.observation.position_m.y),
                velocity_x_mps: Some(sample.observation.velocity_mps.x),
                velocity_y_mps: Some(sample.observation.velocity_mps.y),
                distance_m: Some(entry_stats.distance_m),
                cross_track_m: Some(entry_stats.cross_track_m),
                plane_progress_m: Some(entry_stats.plane_progress_m),
                handoff_heading_error_rad: Some(entry_stats.outbound_heading_error_rad),
                handoff_progress_mps: Some(entry_stats.outbound_progress_mps),
                handoff_cross_speed_mps: Some(entry_stats.outbound_cross_speed_mps),
                speed_mps: Some(entry_stats.speed_mps),
                vertical_speed_mps: Some(entry_stats.vertical_speed_mps),
                contract_pass: Some(assessment.contract_pass()),
                contract_reasons: assessment
                    .violations
                    .iter()
                    .map(|violation| violation.as_str().to_owned())
                    .collect(),
            }
        })
    });
    let active_index = i64::try_from(waypoint_index).ok();
    let guidance_update = controller_updates.iter().rev().find(|update| {
        telemetry_bool(&update.frame.metrics, metric::WAYPOINT_GUIDANCE_ENABLED) == Some(true)
            && telemetry_integer(&update.frame.metrics, metric::WAYPOINT_ACTIVE_INDEX)
                == active_index
            && matches!(
                telemetry_text(&update.frame.metrics, metric::WAYPOINT_CAPTURE_STATUS),
                Some("tracking" | "capture_window")
            )
    });
    let guidance = guidance_update
        .map(waypoint_review_metrics_from_update)
        .unwrap_or_default();
    let guidance_snapshot_age_s =
        guidance_update.map(|update| (last_sample.sim_time_s - update.sim_time_s).max(0.0));
    let target_deadline_remaining_s = guidance
        .target_deadline_remaining_s
        .map(|remaining_s| remaining_s - guidance_snapshot_age_s.unwrap_or(0.0));
    let target_velocity_error_mps =
        guidance
            .target_vx_mps
            .zip(guidance.target_vy_mps)
            .map(|(vx_mps, vy_mps)| {
                (last_sample.observation.velocity_mps - Vec2::new(vx_mps, vy_mps)).length()
            });
    let predicted_handoff_time_to_go_s = guidance
        .predicted_handoff_time_to_go_s
        .map(|remaining_s| (remaining_s - guidance_snapshot_age_s.unwrap_or(0.0)).max(0.0));
    let remaining_to_handoff_m = if terminal_handoff {
        Some(0.0)
    } else {
        Some(((-stats.plane_progress_m) - waypoint.capture_radius_m).max(0.0))
    };
    let handoff_turn_margin_m = remaining_to_handoff_m
        .zip(guidance.required_turn_distance_m)
        .map(|(remaining_m, required_m)| remaining_m - required_m);
    let window_entry = guidance.window_entry.clone().or(sampled_window_entry);
    let final_assessment = waypoint.assess_handoff(stats);
    let resolution_reason = guidance.resolution_reason.clone().or_else(|| {
        terminal_handoff.then(|| {
            if final_assessment.contract_pass_in_window(window_entry.is_some()) {
                "contract_pass"
            } else {
                "plane_deadline"
            }
            .to_owned()
        })
    });
    let window_duration_s = guidance.window_duration_s.or_else(|| {
        terminal_handoff
            .then_some(last_sample.sim_time_s)
            .zip(window_entry.as_ref().and_then(|entry| entry.time_s))
            .map(|(resolution_time_s, entry_time_s)| (resolution_time_s - entry_time_s).max(0.0))
    });

    Some(WaypointReviewMetrics {
        capture_status: Some(capture_status.to_owned()),
        active_index,
        capture_time_s: terminal_handoff.then_some(last_sample.sim_time_s),
        window_entry,
        resolution_reason,
        window_duration_s,
        closest_distance_m,
        distance_m: Some(stats.distance_m),
        cross_track_m: Some(stats.cross_track_m),
        plane_progress_m: Some(stats.plane_progress_m),
        outbound_heading_error_rad: Some(stats.outbound_heading_error_rad),
        outbound_progress_mps: Some(stats.outbound_progress_mps),
        outbound_cross_speed_mps: Some(stats.outbound_cross_speed_mps),
        speed_mps: Some(stats.speed_mps),
        vertical_speed_mps: Some(stats.vertical_speed_mps),
        remaining_to_plane_m: Some((-stats.plane_progress_m).max(0.0)),
        time_to_plane_s: guidance.time_to_plane_s,
        required_turn_distance_m: guidance.required_turn_distance_m,
        shaping_start_distance_m: guidance.shaping_start_distance_m,
        turn_margin_m: guidance.turn_margin_m,
        center_x_m: guidance.center_x_m,
        center_y_m: guidance.center_y_m,
        nominal_handoff_target_x_m: guidance.nominal_handoff_target_x_m,
        nominal_handoff_target_y_m: guidance.nominal_handoff_target_y_m,
        handoff_target_x_m: guidance.handoff_target_x_m,
        handoff_target_y_m: guidance.handoff_target_y_m,
        handoff_target_mode: guidance.handoff_target_mode,
        remaining_to_handoff_m,
        time_to_handoff_s: terminal_handoff
            .then_some(0.0)
            .or(guidance.time_to_handoff_s),
        target_vx_mps: guidance.target_vx_mps,
        target_vy_mps: guidance.target_vy_mps,
        target_deadline_remaining_s,
        target_velocity_error_mps,
        guidance_feasible: guidance.guidance_feasible,
        final_terminal_required_accel_ratio: guidance.final_terminal_required_accel_ratio,
        final_terminal_recoverable: guidance.final_terminal_recoverable,
        predicted_handoff_time_to_go_s,
        predicted_handoff_deadline_lead_s: guidance.predicted_handoff_deadline_lead_s,
        predicted_handoff_contract_status: guidance.predicted_handoff_contract_status,
        predicted_handoff_contract_reasons: guidance.predicted_handoff_contract_reasons,
        predicted_handoff_distance_m: guidance.predicted_handoff_distance_m,
        predicted_handoff_cross_track_m: guidance.predicted_handoff_cross_track_m,
        predicted_handoff_plane_progress_m: guidance.predicted_handoff_plane_progress_m,
        predicted_handoff_outbound_heading_error_rad: guidance
            .predicted_handoff_outbound_heading_error_rad,
        predicted_handoff_outbound_progress_mps: guidance.predicted_handoff_outbound_progress_mps,
        predicted_handoff_outbound_cross_speed_mps: guidance
            .predicted_handoff_outbound_cross_speed_mps,
        predicted_handoff_speed_mps: guidance.predicted_handoff_speed_mps,
        predicted_handoff_vertical_speed_mps: guidance.predicted_handoff_vertical_speed_mps,
        candidate_contract_pass_ever: None,
        candidate_first_pass_time_s: None,
        candidate_last_pass_time_s: None,
        candidate_pass_lost_before_capture: None,
        candidate_best_heading_margin_rad: None,
        candidate_best_cross_speed_margin_mps: None,
        reachable_candidate_contract_pass_ever: None,
        reachable_candidate_first_pass_time_s: None,
        reachable_candidate_last_pass_time_s: None,
        reachable_candidate_pass_lost_before_capture: None,
        reachable_required_accel_ratio_max: None,
        reachable_thrust_saturated_time_max_s: None,
        reachable_tilt_saturated_time_max_s: None,
        continuation_next_waypoint_index: guidance.continuation_next_waypoint_index,
        continuation_contract_pass: guidance.continuation_contract_pass,
        continuation_contract_reasons: guidance.continuation_contract_reasons,
        continuation_outbound_heading_error_rad: guidance.continuation_outbound_heading_error_rad,
        continuation_required_accel_ratio_max: guidance.continuation_required_accel_ratio_max,
        continuation_passing_candidate_count: guidance.continuation_passing_candidate_count,
        transition_next_waypoint_index: guidance.transition_next_waypoint_index,
        transition_position_error_m: guidance.transition_position_error_m,
        transition_velocity_error_mps: guidance.transition_velocity_error_mps,
        transition_attitude_error_rad: guidance.transition_attitude_error_rad,
        transition_mass_error_kg: guidance.transition_mass_error_kg,
        transition_fuel_error_kg: guidance.transition_fuel_error_kg,
        transition_event_time_error_s: guidance.transition_event_time_error_s,
        transition_continuation_contract_pass: guidance.transition_continuation_contract_pass,
        transition_continuation_contract_reasons: guidance.transition_continuation_contract_reasons,
        transition_continuation_outbound_heading_error_rad: guidance
            .transition_continuation_outbound_heading_error_rad,
        transition_continuation_required_accel_ratio_max: guidance
            .transition_continuation_required_accel_ratio_max,
        transition_continuation_passing_candidate_count: guidance
            .transition_continuation_passing_candidate_count,
        joint_next_waypoint_index: guidance.joint_next_waypoint_index,
        joint_evaluated_candidate_count: guidance.joint_evaluated_candidate_count,
        joint_passing_candidate_count: guidance.joint_passing_candidate_count,
        joint_contract_pass: guidance.joint_contract_pass,
        joint_endpoint_x_m: guidance.joint_endpoint_x_m,
        joint_endpoint_y_m: guidance.joint_endpoint_y_m,
        joint_target_vx_mps: guidance.joint_target_vx_mps,
        joint_target_vy_mps: guidance.joint_target_vy_mps,
        joint_time_to_go_s: guidance.joint_time_to_go_s,
        joint_continuation_outbound_heading_error_rad: guidance
            .joint_continuation_outbound_heading_error_rad,
        joint_required_accel_ratio_max: guidance.joint_required_accel_ratio_max,
        joint_total_saturated_time_s: guidance.joint_total_saturated_time_s,
        joint_continuation_passing_candidate_count: guidance
            .joint_continuation_passing_candidate_count,
        plan_reference_position_error_max_m: None,
        plan_reference_cross_error_max_abs_m: None,
        plan_reference_velocity_error_max_mps: None,
        plan_reference_cross_speed_error_max_abs_mps: None,
        guidance_required_accel_ratio_max: None,
        guidance_thrust_saturated_time_s: None,
        guidance_tilt_saturated_time_s: None,
        guidance_first_saturation_lead_s: None,
        last_pass_reference_position_error_m: None,
        last_pass_reference_velocity_error_mps: None,
        last_pass_required_accel_ratio: None,
        guidance_plan_revision_max: None,
        guidance_plan_reasons: Vec::new(),
        handoff_turn_margin_m,
        guidance_snapshot_source: guidance_update.map(|_| "last_pre_capture_update".to_owned()),
        guidance_snapshot_age_s,
        guidance_replan_count: guidance.guidance_replan_count,
    })
}

pub(super) fn waypoint_sample_stats(
    scenario: &ScenarioSpec,
    observation: &Observation,
    waypoint_index: usize,
) -> Option<WaypointHandoffKinematics> {
    let route = scenario.mission.transfer_route.as_ref()?;
    let waypoint = route.waypoints.get(waypoint_index)?;
    let anchor_m = if waypoint_index == 0 {
        let source_pad = scenario.world.landing_pad(&route.source_pad_id)?;
        Vec2::new(source_pad.center_x_m, source_pad.surface_y_m)
    } else {
        route.waypoints.get(waypoint_index - 1)?.position_m
    };
    let next_target_m = route
        .waypoints
        .get(waypoint_index + 1)
        .map(|next| next.position_m)
        .or_else(|| {
            scenario
                .world
                .landing_pad(&route.target_pad_id)
                .map(|pad| Vec2::new(pad.center_x_m, pad.surface_y_m))
        })?;
    let target_m = waypoint.position_m;
    let leg_unit = waypoint_normalized(target_m - anchor_m)?;
    let next_leg_unit = waypoint_normalized(next_target_m - target_m)?;
    let handoff_tangent_unit = waypoint.handoff_tangent_unit.unwrap_or(next_leg_unit);
    let to_waypoint_m = observation.position_m - target_m;
    let speed_mps = observation.velocity_mps.length();
    let velocity_unit = if speed_mps > 1.0e-9 {
        observation.velocity_mps * (1.0 / speed_mps)
    } else {
        Vec2::new(0.0, 0.0)
    };
    let outbound_heading_error_rad = waypoint_dot(velocity_unit, handoff_tangent_unit)
        .clamp(-1.0, 1.0)
        .acos();
    Some(WaypointHandoffKinematics {
        distance_m: to_waypoint_m.length(),
        cross_track_m: waypoint_cross(to_waypoint_m, leg_unit).abs(),
        plane_progress_m: waypoint_dot(to_waypoint_m, leg_unit),
        outbound_heading_error_rad,
        outbound_progress_mps: waypoint_dot(observation.velocity_mps, handoff_tangent_unit),
        outbound_cross_speed_mps: waypoint_cross(observation.velocity_mps, handoff_tangent_unit)
            .abs(),
        speed_mps,
        vertical_speed_mps: observation.velocity_mps.y,
    })
}

pub(super) fn waypoint_capture_passes_review(
    waypoint: &TransferWaypointSpec,
    stats: &WaypointHandoffKinematics,
) -> bool {
    waypoint.assess_handoff(*stats).spatial_pass
}

pub(super) fn waypoint_normalized(vector: Vec2) -> Option<Vec2> {
    let length = vector.length();
    (length > 1.0e-9).then(|| vector * (1.0 / length))
}

pub(super) fn waypoint_dot(lhs: Vec2, rhs: Vec2) -> f64 {
    (lhs.x * rhs.x) + (lhs.y * rhs.y)
}

pub(super) fn waypoint_cross(lhs: Vec2, rhs: Vec2) -> f64 {
    (lhs.x * rhs.y) - (lhs.y * rhs.x)
}

pub(super) fn batch_waypoint_handoff_metrics(
    scenario: &ScenarioSpec,
    waypoint: &WaypointReviewMetrics,
) -> Option<BatchWaypointHandoffReviewMetrics> {
    let waypoint_index = waypoint
        .active_index
        .and_then(|index| usize::try_from(index).ok())?;
    let contract = waypoint_contract_review_metrics(scenario, waypoint);
    let waypoint_id = scenario
        .mission
        .transfer_route
        .as_ref()
        .and_then(|route| route.waypoints.get(waypoint_index))
        .map(|spec| spec.id.clone());
    let continuation_matches_handoff =
        waypoint.continuation_next_waypoint_index == waypoint_index.checked_add(1);
    let transition_matches_handoff =
        waypoint.transition_next_waypoint_index == waypoint_index.checked_add(1);
    let joint_matches_handoff = waypoint.joint_next_waypoint_index == waypoint_index.checked_add(1);
    Some(BatchWaypointHandoffReviewMetrics {
        waypoint_index,
        waypoint_id,
        capture_status: waypoint.capture_status.clone(),
        contract_status: contract.status,
        contract_reasons: contract.reasons,
        capture_time_s: waypoint.capture_time_s,
        window_entry: waypoint.window_entry.clone(),
        resolution_reason: waypoint.resolution_reason.clone(),
        window_duration_s: waypoint.window_duration_s,
        closest_distance_m: waypoint.closest_distance_m,
        distance_m: waypoint.distance_m,
        cross_track_m: waypoint.cross_track_m,
        plane_progress_m: waypoint.plane_progress_m,
        outbound_heading_error_rad: waypoint.outbound_heading_error_rad,
        outbound_progress_mps: waypoint.outbound_progress_mps,
        outbound_cross_speed_mps: waypoint.outbound_cross_speed_mps,
        speed_mps: waypoint.speed_mps,
        vertical_speed_mps: waypoint.vertical_speed_mps,
        remaining_to_plane_m: waypoint.remaining_to_plane_m,
        time_to_plane_s: waypoint.time_to_plane_s,
        required_turn_distance_m: waypoint.required_turn_distance_m,
        shaping_start_distance_m: waypoint.shaping_start_distance_m,
        turn_margin_m: waypoint.turn_margin_m,
        center_x_m: waypoint.center_x_m,
        center_y_m: waypoint.center_y_m,
        nominal_handoff_target_x_m: waypoint.nominal_handoff_target_x_m,
        nominal_handoff_target_y_m: waypoint.nominal_handoff_target_y_m,
        handoff_target_x_m: waypoint.handoff_target_x_m,
        handoff_target_y_m: waypoint.handoff_target_y_m,
        handoff_target_mode: waypoint.handoff_target_mode.clone(),
        remaining_to_handoff_m: waypoint.remaining_to_handoff_m,
        time_to_handoff_s: waypoint.time_to_handoff_s,
        target_vx_mps: waypoint.target_vx_mps,
        target_vy_mps: waypoint.target_vy_mps,
        target_deadline_remaining_s: waypoint.target_deadline_remaining_s,
        target_velocity_error_mps: waypoint.target_velocity_error_mps,
        guidance_feasible: waypoint.guidance_feasible,
        final_terminal_required_accel_ratio: waypoint.final_terminal_required_accel_ratio,
        final_terminal_recoverable: waypoint.final_terminal_recoverable,
        predicted_handoff_time_to_go_s: waypoint.predicted_handoff_time_to_go_s,
        predicted_handoff_deadline_lead_s: waypoint.predicted_handoff_deadline_lead_s,
        predicted_handoff_contract_status: waypoint.predicted_handoff_contract_status.clone(),
        predicted_handoff_contract_reasons: waypoint.predicted_handoff_contract_reasons.clone(),
        predicted_handoff_distance_m: waypoint.predicted_handoff_distance_m,
        predicted_handoff_cross_track_m: waypoint.predicted_handoff_cross_track_m,
        predicted_handoff_plane_progress_m: waypoint.predicted_handoff_plane_progress_m,
        predicted_handoff_outbound_heading_error_rad: waypoint
            .predicted_handoff_outbound_heading_error_rad,
        predicted_handoff_outbound_progress_mps: waypoint.predicted_handoff_outbound_progress_mps,
        predicted_handoff_outbound_cross_speed_mps: waypoint
            .predicted_handoff_outbound_cross_speed_mps,
        predicted_handoff_speed_mps: waypoint.predicted_handoff_speed_mps,
        predicted_handoff_vertical_speed_mps: waypoint.predicted_handoff_vertical_speed_mps,
        candidate_contract_pass_ever: waypoint.candidate_contract_pass_ever,
        candidate_first_pass_time_s: waypoint.candidate_first_pass_time_s,
        candidate_last_pass_time_s: waypoint.candidate_last_pass_time_s,
        candidate_pass_lost_before_capture: waypoint.candidate_pass_lost_before_capture,
        candidate_best_heading_margin_rad: waypoint.candidate_best_heading_margin_rad,
        candidate_best_cross_speed_margin_mps: waypoint.candidate_best_cross_speed_margin_mps,
        reachable_candidate_contract_pass_ever: waypoint.reachable_candidate_contract_pass_ever,
        reachable_candidate_first_pass_time_s: waypoint.reachable_candidate_first_pass_time_s,
        reachable_candidate_last_pass_time_s: waypoint.reachable_candidate_last_pass_time_s,
        reachable_candidate_pass_lost_before_capture: waypoint
            .reachable_candidate_pass_lost_before_capture,
        reachable_required_accel_ratio_max: waypoint.reachable_required_accel_ratio_max,
        reachable_thrust_saturated_time_max_s: waypoint.reachable_thrust_saturated_time_max_s,
        reachable_tilt_saturated_time_max_s: waypoint.reachable_tilt_saturated_time_max_s,
        continuation_next_waypoint_index: continuation_matches_handoff
            .then_some(waypoint.continuation_next_waypoint_index)
            .flatten(),
        continuation_contract_pass: continuation_matches_handoff
            .then_some(waypoint.continuation_contract_pass)
            .flatten(),
        continuation_contract_reasons: if continuation_matches_handoff {
            waypoint.continuation_contract_reasons.clone()
        } else {
            Vec::new()
        },
        continuation_outbound_heading_error_rad: continuation_matches_handoff
            .then_some(waypoint.continuation_outbound_heading_error_rad)
            .flatten(),
        continuation_required_accel_ratio_max: continuation_matches_handoff
            .then_some(waypoint.continuation_required_accel_ratio_max)
            .flatten(),
        continuation_passing_candidate_count: continuation_matches_handoff
            .then_some(waypoint.continuation_passing_candidate_count)
            .flatten(),
        transition_next_waypoint_index: transition_matches_handoff
            .then_some(waypoint.transition_next_waypoint_index)
            .flatten(),
        transition_position_error_m: transition_matches_handoff
            .then_some(waypoint.transition_position_error_m)
            .flatten(),
        transition_velocity_error_mps: transition_matches_handoff
            .then_some(waypoint.transition_velocity_error_mps)
            .flatten(),
        transition_attitude_error_rad: transition_matches_handoff
            .then_some(waypoint.transition_attitude_error_rad)
            .flatten(),
        transition_mass_error_kg: transition_matches_handoff
            .then_some(waypoint.transition_mass_error_kg)
            .flatten(),
        transition_fuel_error_kg: transition_matches_handoff
            .then_some(waypoint.transition_fuel_error_kg)
            .flatten(),
        transition_event_time_error_s: transition_matches_handoff
            .then_some(waypoint.transition_event_time_error_s)
            .flatten(),
        transition_continuation_contract_pass: transition_matches_handoff
            .then_some(waypoint.transition_continuation_contract_pass)
            .flatten(),
        transition_continuation_contract_reasons: if transition_matches_handoff {
            waypoint.transition_continuation_contract_reasons.clone()
        } else {
            Vec::new()
        },
        transition_continuation_outbound_heading_error_rad: transition_matches_handoff
            .then_some(waypoint.transition_continuation_outbound_heading_error_rad)
            .flatten(),
        transition_continuation_required_accel_ratio_max: transition_matches_handoff
            .then_some(waypoint.transition_continuation_required_accel_ratio_max)
            .flatten(),
        transition_continuation_passing_candidate_count: transition_matches_handoff
            .then_some(waypoint.transition_continuation_passing_candidate_count)
            .flatten(),
        joint_next_waypoint_index: joint_matches_handoff
            .then_some(waypoint.joint_next_waypoint_index)
            .flatten(),
        joint_evaluated_candidate_count: joint_matches_handoff
            .then_some(waypoint.joint_evaluated_candidate_count)
            .flatten(),
        joint_passing_candidate_count: joint_matches_handoff
            .then_some(waypoint.joint_passing_candidate_count)
            .flatten(),
        joint_contract_pass: joint_matches_handoff
            .then_some(waypoint.joint_contract_pass)
            .flatten(),
        joint_endpoint_x_m: joint_matches_handoff
            .then_some(waypoint.joint_endpoint_x_m)
            .flatten(),
        joint_endpoint_y_m: joint_matches_handoff
            .then_some(waypoint.joint_endpoint_y_m)
            .flatten(),
        joint_target_vx_mps: joint_matches_handoff
            .then_some(waypoint.joint_target_vx_mps)
            .flatten(),
        joint_target_vy_mps: joint_matches_handoff
            .then_some(waypoint.joint_target_vy_mps)
            .flatten(),
        joint_time_to_go_s: joint_matches_handoff
            .then_some(waypoint.joint_time_to_go_s)
            .flatten(),
        joint_continuation_outbound_heading_error_rad: joint_matches_handoff
            .then_some(waypoint.joint_continuation_outbound_heading_error_rad)
            .flatten(),
        joint_required_accel_ratio_max: joint_matches_handoff
            .then_some(waypoint.joint_required_accel_ratio_max)
            .flatten(),
        joint_total_saturated_time_s: joint_matches_handoff
            .then_some(waypoint.joint_total_saturated_time_s)
            .flatten(),
        joint_continuation_passing_candidate_count: joint_matches_handoff
            .then_some(waypoint.joint_continuation_passing_candidate_count)
            .flatten(),
        plan_reference_position_error_max_m: waypoint.plan_reference_position_error_max_m,
        plan_reference_cross_error_max_abs_m: waypoint.plan_reference_cross_error_max_abs_m,
        plan_reference_velocity_error_max_mps: waypoint.plan_reference_velocity_error_max_mps,
        plan_reference_cross_speed_error_max_abs_mps: waypoint
            .plan_reference_cross_speed_error_max_abs_mps,
        guidance_required_accel_ratio_max: waypoint.guidance_required_accel_ratio_max,
        guidance_thrust_saturated_time_s: waypoint.guidance_thrust_saturated_time_s,
        guidance_tilt_saturated_time_s: waypoint.guidance_tilt_saturated_time_s,
        guidance_first_saturation_lead_s: waypoint.guidance_first_saturation_lead_s,
        last_pass_reference_position_error_m: waypoint.last_pass_reference_position_error_m,
        last_pass_reference_velocity_error_mps: waypoint.last_pass_reference_velocity_error_mps,
        last_pass_required_accel_ratio: waypoint.last_pass_required_accel_ratio,
        guidance_plan_revision_max: waypoint.guidance_plan_revision_max,
        guidance_plan_reasons: waypoint.guidance_plan_reasons.clone(),
        handoff_turn_margin_m: waypoint.handoff_turn_margin_m,
        guidance_snapshot_source: waypoint.guidance_snapshot_source.clone(),
        guidance_snapshot_age_s: waypoint.guidance_snapshot_age_s,
        guidance_replan_count: waypoint.guidance_replan_count,
    })
}

#[derive(Clone, Debug, Default)]
pub(super) struct WaypointRouteReviewMetrics {
    pub(super) status: Option<String>,
    pub(super) passed: Option<usize>,
    pub(super) total: Option<usize>,
    pub(super) first_failure_index: Option<usize>,
}

pub(super) fn waypoint_route_review_metrics(
    scenario: &ScenarioSpec,
    manifest: &RunManifest,
    handoffs: &[BatchWaypointHandoffReviewMetrics],
) -> WaypointRouteReviewMetrics {
    if !matches!(
        scenario.mission.goal,
        EvaluationGoal::LandingOnPad { .. } | EvaluationGoal::WaypointSequence { .. }
    ) {
        return WaypointRouteReviewMetrics::default();
    }
    let Some(route) = scenario
        .mission
        .transfer_route
        .as_ref()
        .filter(|route| !route.waypoints.is_empty())
    else {
        return WaypointRouteReviewMetrics::default();
    };

    let total = route.waypoints.len();
    let sequence_summary = manifest.summary.waypoint_sequence.as_ref();
    let passed = sequence_summary.map_or_else(
        || {
            handoffs
                .iter()
                .enumerate()
                .take_while(|(expected_index, handoff)| {
                    handoff.waypoint_index == *expected_index
                        && handoff.contract_status.as_deref() == Some("pass")
                })
                .count()
        },
        |summary| summary.passed_handoffs,
    );
    let first_failure_index = sequence_summary
        .and_then(|summary| summary.first_failed_index)
        .or_else(|| {
            handoffs.iter().find_map(|handoff| {
                (!matches!(
                    handoff.contract_status.as_deref(),
                    Some("pass" | "incomplete") | None
                ))
                .then_some(handoff.waypoint_index)
            })
        });
    let status = if passed == total {
        "pass"
    } else if first_failure_index.is_some()
        || !matches!(manifest.mission_outcome, MissionOutcome::InProgress)
    {
        "failed"
    } else {
        "incomplete"
    };

    WaypointRouteReviewMetrics {
        status: Some(status.to_owned()),
        passed: Some(passed),
        total: Some(total),
        first_failure_index,
    }
}

pub(super) fn waypoint_contract_review_metrics(
    scenario: &ScenarioSpec,
    waypoint: &WaypointReviewMetrics,
) -> WaypointContractReviewMetrics {
    let Some(capture_status) = waypoint.capture_status.as_deref() else {
        return WaypointContractReviewMetrics::default();
    };
    let Some(route) = scenario.mission.transfer_route.as_ref() else {
        return WaypointContractReviewMetrics {
            status: Some("unknown".to_owned()),
            reasons: vec!["missing_route".to_owned()],
        };
    };
    let Some(active_index) = waypoint
        .active_index
        .and_then(|index| usize::try_from(index).ok())
    else {
        return WaypointContractReviewMetrics {
            status: Some("unknown".to_owned()),
            reasons: vec!["missing_waypoint_index".to_owned()],
        };
    };
    let Some(spec) = route.waypoints.get(active_index) else {
        return WaypointContractReviewMetrics {
            status: Some("unknown".to_owned()),
            reasons: vec!["missing_waypoint_spec".to_owned()],
        };
    };

    match capture_status {
        "tracking" | "capture_window" => WaypointContractReviewMetrics {
            status: Some("incomplete".to_owned()),
            reasons: vec![capture_status.to_owned()],
        },
        "missed" => WaypointContractReviewMetrics {
            status: Some("spatial_miss".to_owned()),
            reasons: waypoint_spatial_miss_reasons(spec, waypoint),
        },
        "captured" => waypoint_outbound_contract_review_metrics(spec, waypoint),
        _ => WaypointContractReviewMetrics {
            status: Some("unknown".to_owned()),
            reasons: vec![format!("unknown_capture_status:{capture_status}")],
        },
    }
}

pub(super) fn waypoint_spatial_miss_reasons(
    spec: &TransferWaypointSpec,
    waypoint: &WaypointReviewMetrics,
) -> Vec<String> {
    let mut reasons = Vec::new();
    match waypoint.distance_m {
        Some(distance_m) if distance_m <= spec.capture_radius_m => {}
        Some(_) => reasons.push("outside_capture_radius".to_owned()),
        None => reasons.push("missing_distance".to_owned()),
    }
    match waypoint.cross_track_m {
        Some(cross_track_m) if cross_track_m <= spec.max_cross_track_m => {}
        Some(_) => reasons.push("cross_track".to_owned()),
        None => reasons.push("missing_cross_track".to_owned()),
    }
    match waypoint.plane_progress_m {
        Some(plane_progress_m) if plane_progress_m >= -spec.capture_radius_m => {}
        Some(_) => reasons.push("before_waypoint_plane".to_owned()),
        None => reasons.push("missing_plane_progress".to_owned()),
    }
    if reasons.is_empty() {
        reasons.push("spatial_miss".to_owned());
    }
    reasons
}

pub(super) fn waypoint_outbound_contract_review_metrics(
    spec: &TransferWaypointSpec,
    waypoint: &WaypointReviewMetrics,
) -> WaypointContractReviewMetrics {
    let mut reasons = Vec::new();
    let Some(outbound_heading_error_rad) = waypoint.outbound_heading_error_rad else {
        reasons.push("missing_heading".to_owned());
        return WaypointContractReviewMetrics {
            status: Some("outbound_out_of_envelope".to_owned()),
            reasons,
        };
    };
    let Some(outbound_progress_mps) = waypoint.outbound_progress_mps else {
        reasons.push("missing_outbound_progress".to_owned());
        return WaypointContractReviewMetrics {
            status: Some("outbound_out_of_envelope".to_owned()),
            reasons,
        };
    };
    let Some(speed_mps) = waypoint.speed_mps else {
        reasons.push("missing_speed".to_owned());
        return WaypointContractReviewMetrics {
            status: Some("outbound_out_of_envelope".to_owned()),
            reasons,
        };
    };
    let outbound_cross_speed_mps = match (
        spec.max_outbound_cross_speed_mps,
        waypoint.outbound_cross_speed_mps,
    ) {
        (Some(_), None) => {
            reasons.push("missing_outbound_cross_speed".to_owned());
            0.0
        }
        (_, value) => value.unwrap_or(0.0),
    };
    let vertical_speed_mps = match (
        spec.min_vertical_speed_mps.is_some() || spec.max_vertical_speed_mps.is_some(),
        waypoint.vertical_speed_mps,
    ) {
        (true, None) => {
            reasons.push("missing_vertical_speed".to_owned());
            0.0
        }
        (_, value) => value.unwrap_or(0.0),
    };

    if reasons.is_empty() {
        reasons.extend(
            spec.assess_handoff(WaypointHandoffKinematics {
                distance_m: waypoint.distance_m.unwrap_or(0.0),
                cross_track_m: waypoint.cross_track_m.unwrap_or(0.0),
                plane_progress_m: waypoint.plane_progress_m.unwrap_or(0.0),
                outbound_heading_error_rad,
                outbound_progress_mps,
                outbound_cross_speed_mps,
                speed_mps,
                vertical_speed_mps,
            })
            .violations
            .into_iter()
            .map(|violation| violation.as_str().to_owned()),
        );
    }

    if reasons.is_empty() {
        WaypointContractReviewMetrics {
            status: Some("pass".to_owned()),
            reasons,
        }
    } else {
        WaypointContractReviewMetrics {
            status: Some("outbound_out_of_envelope".to_owned()),
            reasons,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct TransferReviewMetrics {
    pub(super) terminal_entry_kind: Option<String>,
    pub(super) terminal_handoff_time_s: Option<f64>,
    pub(super) terminal_handoff_dx_m: Option<f64>,
    pub(super) terminal_handoff_height_m: Option<f64>,
    pub(super) terminal_handoff_speed_mps: Option<f64>,
    pub(super) terminal_handoff_gate_mode: Option<String>,
    pub(super) terminal_handoff_projected_dx_m: Option<f64>,
    pub(super) terminal_handoff_impact_angle_deg: Option<f64>,
    pub(super) terminal_handoff_boost_quality: Option<String>,
    pub(super) terminal_handoff_latest_safe_margin_s: Option<f64>,
    pub(super) terminal_handoff_required_accel_ratio: Option<f64>,
    pub(super) terminal_post_handoff_apex_gain_m: Option<f64>,
    pub(super) terminal_post_handoff_time_to_apex_s: Option<f64>,
    pub(super) terminal_post_handoff_apex_dx_abs_m: Option<f64>,
    pub(super) terminal_low_altitude_rebound_gain_m: Option<f64>,
    pub(super) terminal_low_altitude_rebound_origin_dx_abs_m: Option<f64>,
    pub(super) terminal_low_altitude_rebound_near_pad: Option<bool>,
    pub(super) final_phase: Option<String>,
    pub(super) boost_projected_dx_m: Option<f64>,
    pub(super) boost_impact_angle_deg: Option<f64>,
    pub(super) boost_apex_over_target_m: Option<f64>,
    pub(super) boost_quality: Option<String>,
    pub(super) boost_selected_score: Option<f64>,
    pub(super) boost_settled_quality: Option<String>,
    pub(super) boost_settled_projected_dx_m: Option<f64>,
    pub(super) boost_cutoff_time_s: Option<f64>,
    pub(super) boost_cutoff_projected_dx_m: Option<f64>,
    pub(super) boost_cutoff_impact_angle_deg: Option<f64>,
    pub(super) boost_cutoff_apex_over_target_m: Option<f64>,
    pub(super) boost_cutoff_quality: Option<String>,
    pub(super) boost_burn_duration_s: Option<f64>,
    pub(super) boost_burn_fuel_used_kg: Option<f64>,
    pub(super) boost_burn_avg_throttle: Option<f64>,
    pub(super) terminal_gate_mode: Option<String>,
    pub(super) terminal_gate_latest_safe_margin_s: Option<f64>,
    pub(super) terminal_gate_required_accel_ratio: Option<f64>,
    pub(super) terminal_gate_deferred: Option<bool>,
    pub(super) corridor_mode: Option<String>,
    pub(super) corridor_min_margin_m: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TerminalLowAltitudeRebound {
    gain_m: f64,
    origin_dx_abs_m: f64,
    near_pad: bool,
}

pub(super) fn terminal_low_altitude_rebound(
    terminal_samples: &[&SampleRecord],
) -> Option<TerminalLowAltitudeRebound> {
    let mut minimum_height_m = None::<f64>;
    let mut minimum_dx_abs_m = 0.0;
    let mut minimum_pad_half_width_m = 0.0;
    let mut best = None::<TerminalLowAltitudeRebound>;

    for sample in terminal_samples {
        let observation = &sample.observation;
        if minimum_height_m.is_none()
            && (observation.height_above_target_m > TRANSFER_TERMINAL_REBOUND_ARM_HEIGHT_M
                || observation.velocity_mps.y > 0.0)
        {
            continue;
        }

        if minimum_height_m.is_none_or(|height_m| observation.height_above_target_m < height_m) {
            minimum_height_m = Some(observation.height_above_target_m);
            minimum_dx_abs_m = observation.target_dx_m.abs();
            minimum_pad_half_width_m = observation.target_pad_half_width_m.max(0.0);
        }

        let gain_m = (observation.height_above_target_m - minimum_height_m?).max(0.0);
        if best.is_none_or(|current| gain_m > current.gain_m) {
            best = Some(TerminalLowAltitudeRebound {
                gain_m,
                origin_dx_abs_m: minimum_dx_abs_m,
                near_pad: minimum_pad_half_width_m > 0.0
                    && minimum_dx_abs_m
                        <= TRANSFER_TERMINAL_REBOUND_NEAR_PAD_HALF_WIDTHS
                            * minimum_pad_half_width_m,
            });
        }
    }

    best
}

pub(super) fn transfer_review_metrics(
    controller_updates: &[pd_control::ControllerUpdateRecord],
    samples: &[SampleRecord],
) -> TransferReviewMetrics {
    let final_phase = controller_updates
        .iter()
        .rev()
        .find_map(|update| telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE))
        .map(ToOwned::to_owned);
    let Some((handoff_index, handoff)) =
        controller_updates.iter().enumerate().find(|(_, update)| {
            telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("terminal")
        })
    else {
        let mut metrics = transfer_review_metrics_without_handoff(controller_updates, samples);
        metrics.final_phase = final_phase;
        return metrics;
    };
    let terminal_entry_kind = if controller_updates[..handoff_index].iter().any(|update| {
        matches!(
            telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE),
            Some("boost" | "coast")
        )
    }) {
        "handoff"
    } else {
        "direct"
    };
    let terminal_handoff_dx_m = telemetry_float(&handoff.frame.metrics, metric::TARGET_DX_M);
    let terminal_handoff_height_m =
        telemetry_float(&handoff.frame.metrics, metric::HEIGHT_ABOVE_TARGET_M);
    let terminal_handoff_speed_mps =
        telemetry_float(&handoff.frame.metrics, metric::VERTICAL_SPEED_MPS)
            .zip(telemetry_float(
                &handoff.frame.metrics,
                metric::TANGENTIAL_SPEED_MPS,
            ))
            .map(|(vertical, tangential)| vertical.hypot(tangential));
    let terminal_handoff_gate_mode =
        telemetry_text(&handoff.frame.metrics, metric::TRANSFER_TERMINAL_GATE_MODE)
            .map(ToOwned::to_owned);
    let terminal_handoff_projected_dx_m =
        telemetry_float(&handoff.frame.metrics, metric::TRANSFER_PROJECTED_DX_M);
    let terminal_handoff_impact_angle_deg =
        telemetry_float(&handoff.frame.metrics, metric::TRANSFER_IMPACT_ANGLE_DEG)
            .filter(|value| *value >= 0.0);
    let terminal_handoff_boost_quality =
        telemetry_text(&handoff.frame.metrics, metric::TRANSFER_BOOST_QUALITY)
            .map(ToOwned::to_owned);
    let terminal_handoff_latest_safe_margin_s = telemetry_float(
        &handoff.frame.metrics,
        metric::TRANSFER_TERMINAL_GATE_LATEST_SAFE_MARGIN_S,
    );
    let terminal_handoff_required_accel_ratio = telemetry_float(
        &handoff.frame.metrics,
        metric::TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO,
    );
    let terminal_samples = samples
        .iter()
        .filter(|sample| sample.physics_step >= handoff.physics_step)
        .collect::<Vec<_>>();
    let terminal_apex = terminal_samples.iter().copied().max_by(|lhs, rhs| {
        lhs.observation
            .height_above_target_m
            .partial_cmp(&rhs.observation.height_above_target_m)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let terminal_entry = terminal_samples.first().copied();
    let terminal_post_handoff_apex_gain_m =
        terminal_entry.zip(terminal_apex).map(|(entry, apex)| {
            (apex.observation.height_above_target_m - entry.observation.height_above_target_m)
                .max(0.0)
        });
    let terminal_post_handoff_time_to_apex_s = terminal_entry
        .zip(terminal_apex)
        .map(|(entry, apex)| (apex.sim_time_s - entry.sim_time_s).max(0.0));
    let terminal_post_handoff_apex_dx_abs_m =
        terminal_apex.map(|apex| apex.observation.target_dx_m.abs());
    let terminal_low_altitude_rebound = terminal_low_altitude_rebound(&terminal_samples);
    let mut metrics = transfer_review_metrics_without_handoff(controller_updates, samples);
    metrics.terminal_entry_kind = Some(terminal_entry_kind.to_owned());
    metrics.terminal_handoff_time_s = Some(handoff.sim_time_s);
    metrics.terminal_handoff_dx_m = terminal_handoff_dx_m;
    metrics.terminal_handoff_height_m = terminal_handoff_height_m;
    metrics.terminal_handoff_speed_mps = terminal_handoff_speed_mps;
    metrics.terminal_handoff_gate_mode = terminal_handoff_gate_mode;
    metrics.terminal_handoff_projected_dx_m = terminal_handoff_projected_dx_m;
    metrics.terminal_handoff_impact_angle_deg = terminal_handoff_impact_angle_deg;
    metrics.terminal_handoff_boost_quality = terminal_handoff_boost_quality;
    metrics.terminal_handoff_latest_safe_margin_s = terminal_handoff_latest_safe_margin_s;
    metrics.terminal_handoff_required_accel_ratio = terminal_handoff_required_accel_ratio;
    metrics.terminal_post_handoff_apex_gain_m = terminal_post_handoff_apex_gain_m;
    metrics.terminal_post_handoff_time_to_apex_s = terminal_post_handoff_time_to_apex_s;
    metrics.terminal_post_handoff_apex_dx_abs_m = terminal_post_handoff_apex_dx_abs_m;
    metrics.terminal_low_altitude_rebound_gain_m =
        terminal_low_altitude_rebound.map(|rebound| rebound.gain_m);
    metrics.terminal_low_altitude_rebound_origin_dx_abs_m =
        terminal_low_altitude_rebound.map(|rebound| rebound.origin_dx_abs_m);
    metrics.terminal_low_altitude_rebound_near_pad =
        terminal_low_altitude_rebound.map(|rebound| rebound.near_pad);
    metrics.final_phase = final_phase;
    metrics
}

pub(super) fn transfer_review_metrics_without_handoff(
    controller_updates: &[pd_control::ControllerUpdateRecord],
    samples: &[SampleRecord],
) -> TransferReviewMetrics {
    let last_boost_update = controller_updates.iter().rev().find(|update| {
        telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("boost")
            && telemetry_text(&update.frame.metrics, metric::TRANSFER_BOOST_QUALITY).is_some()
    });
    let (
        boost_projected_dx_m,
        boost_impact_angle_deg,
        boost_apex_over_target_m,
        boost_quality,
        boost_selected_score,
        boost_settled_quality,
        boost_settled_projected_dx_m,
    ) = last_boost_update
        .map(|update| {
            (
                telemetry_float(&update.frame.metrics, metric::TRANSFER_PROJECTED_DX_M),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_IMPACT_ANGLE_DEG)
                    .filter(|value| *value >= 0.0),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_APEX_OVER_TARGET_M),
                telemetry_text(&update.frame.metrics, metric::TRANSFER_BOOST_QUALITY)
                    .map(ToOwned::to_owned),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_BOOST_SELECTED_SCORE),
                telemetry_text(
                    &update.frame.metrics,
                    metric::TRANSFER_BOOST_SETTLED_QUALITY,
                )
                .map(ToOwned::to_owned),
                telemetry_float(
                    &update.frame.metrics,
                    metric::TRANSFER_BOOST_SETTLED_PROJECTED_DX_M,
                ),
            )
        })
        .unwrap_or((None, None, None, None, None, None, None));
    let boost_cutoff = transfer_boost_cutoff_update(controller_updates);
    let (
        boost_cutoff_time_s,
        boost_cutoff_projected_dx_m,
        boost_cutoff_impact_angle_deg,
        boost_cutoff_apex_over_target_m,
        boost_cutoff_quality,
    ) = boost_cutoff
        .map(|update| {
            (
                Some(update.sim_time_s),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_PROJECTED_DX_M),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_IMPACT_ANGLE_DEG)
                    .filter(|value| *value >= 0.0),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_APEX_OVER_TARGET_M),
                telemetry_text(&update.frame.metrics, metric::TRANSFER_BOOST_QUALITY)
                    .map(ToOwned::to_owned),
            )
        })
        .unwrap_or((None, None, None, None, None));
    let boost_burn = transfer_boost_burn_metrics(controller_updates, samples, boost_cutoff);
    let terminal_gate_update = controller_updates.iter().rev().find(|update| {
        telemetry_text(&update.frame.metrics, metric::TRANSFER_TERMINAL_GATE_MODE).is_some()
    });
    let terminal_gate_deferred = if controller_updates.iter().any(|update| {
        telemetry_bool(
            &update.frame.metrics,
            metric::TRANSFER_TERMINAL_GATE_DEFERRED,
        ) == Some(true)
    }) {
        Some(true)
    } else {
        terminal_gate_update.and_then(|update| {
            telemetry_bool(
                &update.frame.metrics,
                metric::TRANSFER_TERMINAL_GATE_DEFERRED,
            )
        })
    };
    let (
        terminal_gate_mode,
        terminal_gate_latest_safe_margin_s,
        terminal_gate_required_accel_ratio,
    ) = terminal_gate_update
        .map(|update| {
            (
                telemetry_text(&update.frame.metrics, metric::TRANSFER_TERMINAL_GATE_MODE)
                    .map(ToOwned::to_owned),
                telemetry_float(
                    &update.frame.metrics,
                    metric::TRANSFER_TERMINAL_GATE_LATEST_SAFE_MARGIN_S,
                ),
                telemetry_float(
                    &update.frame.metrics,
                    metric::TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO,
                ),
            )
        })
        .unwrap_or((None, None, None));
    let corridor_min_margin_m = controller_updates
        .iter()
        .filter(|update| {
            telemetry_text(&update.frame.metrics, metric::TRANSFER_CORRIDOR_MODE)
                .is_some_and(|mode| mode != "inactive")
        })
        .filter_map(|update| {
            telemetry_float(&update.frame.metrics, metric::TRANSFER_CORRIDOR_MARGIN_M)
        })
        .filter(|margin_m| margin_m.is_finite())
        .reduce(f64::min);
    let last_corridor_mode = controller_updates
        .iter()
        .rev()
        .filter_map(|update| telemetry_text(&update.frame.metrics, metric::TRANSFER_CORRIDOR_MODE))
        .find(|mode| *mode != "inactive")
        .map(ToOwned::to_owned);
    let corridor_mode = if controller_updates.iter().any(|update| {
        telemetry_text(&update.frame.metrics, metric::TRANSFER_CORRIDOR_MODE) == Some("active")
    }) {
        Some("active".to_owned())
    } else {
        last_corridor_mode
    };

    TransferReviewMetrics {
        boost_projected_dx_m,
        boost_impact_angle_deg,
        boost_apex_over_target_m,
        boost_quality,
        boost_selected_score,
        boost_settled_quality,
        boost_settled_projected_dx_m,
        boost_cutoff_time_s,
        boost_cutoff_projected_dx_m,
        boost_cutoff_impact_angle_deg,
        boost_cutoff_apex_over_target_m,
        boost_cutoff_quality,
        boost_burn_duration_s: boost_burn.and_then(|metrics| metrics.duration_s),
        boost_burn_fuel_used_kg: boost_burn.and_then(|metrics| metrics.fuel_used_kg),
        boost_burn_avg_throttle: boost_burn.and_then(|metrics| metrics.avg_throttle),
        terminal_gate_mode,
        terminal_gate_latest_safe_margin_s,
        terminal_gate_required_accel_ratio,
        terminal_gate_deferred,
        corridor_mode,
        corridor_min_margin_m,
        ..TransferReviewMetrics::default()
    }
}

pub(super) fn transfer_boost_cutoff_update(
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> Option<&pd_control::ControllerUpdateRecord> {
    let mut last_boost = None;
    for update in controller_updates {
        match telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) {
            Some("boost") => last_boost = Some(update),
            Some(_) if last_boost.is_some() => return last_boost,
            _ => {}
        }
    }
    None
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferBoostBurnMetrics {
    duration_s: Option<f64>,
    fuel_used_kg: Option<f64>,
    avg_throttle: Option<f64>,
}

pub(super) fn transfer_boost_burn_metrics(
    controller_updates: &[pd_control::ControllerUpdateRecord],
    samples: &[SampleRecord],
    boost_cutoff: Option<&pd_control::ControllerUpdateRecord>,
) -> Option<TransferBoostBurnMetrics> {
    let first_boost = controller_updates.iter().find(|update| {
        telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("boost")
    })?;
    let cutoff = boost_cutoff
        .or_else(|| {
            controller_updates.iter().rev().find(|update| {
                telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("boost")
            })
        })
        .unwrap_or(first_boost);
    let duration_s = (cutoff.sim_time_s - first_boost.sim_time_s)
        .is_finite()
        .then_some((cutoff.sim_time_s - first_boost.sim_time_s).max(0.0));
    let fuel_used_kg = sample_at_or_after_step(samples, first_boost.physics_step)
        .zip(sample_at_or_after_step(samples, cutoff.physics_step))
        .map(|(start, end)| start.observation.fuel_kg - end.observation.fuel_kg)
        .filter(|value| value.is_finite())
        .map(|value| value.max(0.0));
    let avg_throttle = transfer_boost_avg_throttle(controller_updates, first_boost, cutoff);

    Some(TransferBoostBurnMetrics {
        duration_s,
        fuel_used_kg,
        avg_throttle,
    })
}

pub(super) fn transfer_boost_avg_throttle(
    controller_updates: &[pd_control::ControllerUpdateRecord],
    first_boost: &pd_control::ControllerUpdateRecord,
    cutoff: &pd_control::ControllerUpdateRecord,
) -> Option<f64> {
    let boost_updates = controller_updates
        .iter()
        .enumerate()
        .filter(|(_, update)| {
            update.physics_step >= first_boost.physics_step
                && update.physics_step <= cutoff.physics_step
                && telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("boost")
        })
        .collect::<Vec<_>>();
    if boost_updates.is_empty() {
        return None;
    }

    let mut weighted_sum = 0.0;
    let mut total_dt = 0.0;
    for (update_index, update) in boost_updates {
        let next_time_s = controller_updates
            .get(update_index + 1)
            .map(|next| next.sim_time_s)
            .unwrap_or(cutoff.sim_time_s)
            .min(cutoff.sim_time_s);
        let dt = (next_time_s - update.sim_time_s).max(0.0);
        if dt > 0.0 {
            weighted_sum += update.frame.command.throttle_frac * dt;
            total_dt += dt;
        }
    }
    if total_dt > 1e-9 {
        Some(weighted_sum / total_dt)
    } else {
        Some(
            controller_updates
                .iter()
                .filter(|update| {
                    update.physics_step >= first_boost.physics_step
                        && update.physics_step <= cutoff.physics_step
                        && telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE)
                            == Some("boost")
                })
                .map(|update| update.frame.command.throttle_frac)
                .sum::<f64>()
                / controller_updates
                    .iter()
                    .filter(|update| {
                        update.physics_step >= first_boost.physics_step
                            && update.physics_step <= cutoff.physics_step
                            && telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE)
                                == Some("boost")
                    })
                    .count()
                    .max(1) as f64,
        )
    }
}

pub(super) fn sample_at_or_after_step(
    samples: &[SampleRecord],
    physics_step: u64,
) -> Option<&SampleRecord> {
    samples
        .iter()
        .find(|sample| sample.physics_step >= physics_step)
        .or_else(|| samples.last())
}

pub(super) fn telemetry_text<'a>(
    metrics: &'a BTreeMap<String, TelemetryValue>,
    key: &str,
) -> Option<&'a str> {
    match metrics.get(key)? {
        TelemetryValue::Text(value) => Some(value),
        _ => None,
    }
}

pub(super) fn telemetry_float(
    metrics: &BTreeMap<String, TelemetryValue>,
    key: &str,
) -> Option<f64> {
    match metrics.get(key)? {
        TelemetryValue::Float(value) => Some(*value),
        TelemetryValue::Integer(value) => Some(*value as f64),
        _ => None,
    }
}

pub(super) fn telemetry_integer(
    metrics: &BTreeMap<String, TelemetryValue>,
    key: &str,
) -> Option<i64> {
    match metrics.get(key)? {
        TelemetryValue::Integer(value) => Some(*value),
        _ => None,
    }
}

pub(super) fn telemetry_bool(
    metrics: &BTreeMap<String, TelemetryValue>,
    key: &str,
) -> Option<bool> {
    match metrics.get(key)? {
        TelemetryValue::Bool(value) => Some(*value),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferShapeMetrics {
    pub(super) curve_rmse_m: f64,
    pub(super) apex_error_m: f64,
    pub(super) projected_dx_abs_mean_m: Option<f64>,
    pub(super) projected_dx_abs_max_m: Option<f64>,
    pub(super) shortfall_ratio: Option<f64>,
}

pub(super) fn transfer_shape_metrics(
    scenario: &ScenarioSpec,
    samples: &[SampleRecord],
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> Option<TransferShapeMetrics> {
    scenario.mission.transfer_route.as_ref()?;
    let target_pad = scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.id == scenario.mission.goal.target_pad_id())?;
    let first_boost = controller_updates.iter().find(|update| {
        telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("boost")
    })?;
    let start_sample = sample_at_or_after_step(samples, first_boost.physics_step)?;
    let terminal_step = controller_updates
        .iter()
        .find(|update| {
            update.physics_step >= first_boost.physics_step
                && telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("terminal")
        })
        .map(|update| update.physics_step)
        .unwrap_or_else(|| {
            samples
                .last()
                .map(|sample| sample.physics_step)
                .unwrap_or(first_boost.physics_step)
        });
    let window_samples = samples
        .iter()
        .filter(|sample| {
            sample.physics_step >= start_sample.physics_step && sample.physics_step <= terminal_step
        })
        .collect::<Vec<_>>();
    if window_samples.len() < 2 {
        return None;
    }

    let start_x = start_sample.observation.position_m.x;
    let start_y = start_sample.observation.position_m.y;
    let target_x = target_pad.center_x_m;
    let target_y = target_pad.surface_y_m;
    let dx_anchor_m = target_x - start_x;
    let dy_anchor_m = target_y - start_y;
    if !dx_anchor_m.is_finite() || !dy_anchor_m.is_finite() {
        return None;
    }
    let apex_target_over_target_m =
        transfer_shape_apex_target_over_target_m(dx_anchor_m.abs(), dy_anchor_m);

    let mut curve_sq_err_sum = 0.0;
    let mut curve_count = 0_usize;
    let mut apex_actual_over_target_m = 0.0_f64;
    for sample in &window_samples {
        let x = sample.observation.position_m.x;
        let y = sample.observation.position_m.y;
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        let y_ref = transfer_shape_reference_y_at_x(
            x,
            start_x,
            start_y,
            target_x,
            target_y,
            apex_target_over_target_m,
        );
        let y_err = y - y_ref;
        curve_sq_err_sum += y_err * y_err;
        curve_count += 1;
        apex_actual_over_target_m = apex_actual_over_target_m.max((y - target_y).max(0.0));
    }
    if curve_count == 0 {
        return None;
    }

    let mut projected_abs_sum = 0.0;
    let mut projected_abs_max = 0.0_f64;
    let mut projected_count = 0_usize;
    let mut shortfall_count = 0_usize;
    let mut shortfall_sample_count = 0_usize;
    for update in controller_updates {
        if update.physics_step < start_sample.physics_step || update.physics_step >= terminal_step {
            continue;
        }
        if telemetry_bool(&update.frame.metrics, metric::TRANSFER_TARGET_Y_SOLUTION) == Some(false)
        {
            continue;
        }
        let Some(projected_dx_m) =
            telemetry_float(&update.frame.metrics, metric::TRANSFER_PROJECTED_DX_M)
        else {
            continue;
        };
        if !projected_dx_m.is_finite() {
            continue;
        }
        let projected_abs = projected_dx_m.abs();
        projected_abs_sum += projected_abs;
        projected_abs_max = projected_abs_max.max(projected_abs);
        projected_count += 1;

        if let Some(route_dx_m) =
            telemetry_float(&update.frame.metrics, metric::TRANSFER_ROUTE_DX_M)
            && route_dx_m.abs() > 1e-3
        {
            shortfall_sample_count += 1;
            if projected_dx_m * route_dx_m.signum() > 0.0 {
                shortfall_count += 1;
            }
        }
    }

    Some(TransferShapeMetrics {
        curve_rmse_m: (curve_sq_err_sum / curve_count as f64).sqrt(),
        apex_error_m: (apex_actual_over_target_m - apex_target_over_target_m).abs(),
        projected_dx_abs_mean_m: (projected_count > 0)
            .then_some(projected_abs_sum / projected_count as f64),
        projected_dx_abs_max_m: (projected_count > 0).then_some(projected_abs_max),
        shortfall_ratio: (shortfall_sample_count > 0)
            .then_some(shortfall_count as f64 / shortfall_sample_count as f64),
    })
}

pub(super) fn transfer_shape_reference_y_at_x(
    x: f64,
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    apex_target_over_target_m: f64,
) -> f64 {
    let dx = target_x - start_x;
    if dx.abs() <= 1e-6 {
        return start_y.max(target_y);
    }
    let s = ((x - start_x) / dx).clamp(0.0, 1.0);
    let baseline = start_y + ((target_y - start_y) * s);
    baseline + (4.0 * apex_target_over_target_m * s * (1.0 - s))
}

pub(super) fn transfer_shape_apex_target_over_target_m(dx_abs_m: f64, dy_m: f64) -> f64 {
    const APEX_HEIGHT_PER_DX: f64 = 0.18;
    const APEX_HEIGHT_PER_UPHILL_DY: f64 = 0.15;
    const APEX_HEIGHT_MIN_M: f64 = 30.0;
    const APEX_HEIGHT_MAX_M: f64 = 240.0;

    (APEX_HEIGHT_PER_DX * dx_abs_m).clamp(APEX_HEIGHT_MIN_M, APEX_HEIGHT_MAX_M)
        + (dy_m * APEX_HEIGHT_PER_UPHILL_DY).max(0.0)
        + (-dy_m).max(0.0)
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LowAltitudeRecoveryMetrics {
    low_altitude_dwell_s: f64,
    low_altitude_unsafe_recovery_s: f64,
}

pub(super) fn low_altitude_recovery_metrics(
    scenario: &ScenarioSpec,
    samples: &[SampleRecord],
) -> Option<LowAltitudeRecoveryMetrics> {
    if !matches!(scenario.mission.goal, EvaluationGoal::LandingOnPad { .. }) {
        return None;
    }
    let target_pad = scenario
        .world
        .landing_pad(scenario.mission.goal.target_pad_id())?;
    let touchdown_center_limit_m =
        (target_pad.half_width_m() - scenario.vehicle.geometry.touchdown_half_span_m).max(0.0);
    let low_altitude_threshold_m = scenario
        .vehicle
        .geometry
        .touchdown_base_offset_m
        .abs()
        .max(1.0);
    let safe_tangential_speed_mps = scenario
        .vehicle
        .safe_touchdown_tangential_speed_mps
        .max(0.0);
    let mut low_altitude_dwell_s = 0.0;
    let mut low_altitude_unsafe_recovery_s = 0.0;

    for pair in samples.windows(2) {
        let current = &pair[0];
        let next = &pair[1];
        let dt_s = next.sim_time_s - current.sim_time_s;
        if !dt_s.is_finite() || dt_s <= 0.0 {
            continue;
        }
        let observation = &current.observation;
        if observation.touchdown_clearance_m >= low_altitude_threshold_m {
            continue;
        }

        low_altitude_dwell_s += dt_s;
        let laterally_unsafe = observation.target_dx_m.abs() > touchdown_center_limit_m
            || observation.velocity_mps.x.abs() > safe_tangential_speed_mps;
        if laterally_unsafe {
            low_altitude_unsafe_recovery_s += dt_s;
        }
    }

    Some(LowAltitudeRecoveryMetrics {
        low_altitude_dwell_s,
        low_altitude_unsafe_recovery_s,
    })
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ReferenceGapMetrics {
    gap_mean_m: f64,
    gap_max_m: f64,
}

pub(super) fn reference_gap_metrics(
    scenario: &ScenarioSpec,
    samples: &[SampleRecord],
) -> Option<ReferenceGapMetrics> {
    let target_pad = scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.id == scenario.mission.goal.target_pad_id())?;
    let actual_points = samples
        .iter()
        .map(|sample| {
            (
                sample.observation.position_m.x,
                sample.observation.position_m.y,
            )
        })
        .collect::<Vec<_>>();
    if actual_points.len() < 2 {
        return None;
    }
    let reference_points = idealized_reference_curve(
        scenario.initial_state.position_m.x,
        scenario.initial_state.position_m.y,
        target_pad.center_x_m,
        target_pad.surface_y_m,
        scenario.world.gravity_mps2,
    )?;
    if reference_points.len() < 2 {
        return None;
    }

    let (actual_cumulative, actual_length) = polyline_lengths(&actual_points)?;
    let mut gaps = Vec::with_capacity(actual_points.len());
    for point in &actual_points {
        let (_projection, distance) = project_point_to_polyline(*point, &reference_points)?;
        gaps.push(distance);
    }
    if gaps.len() < 2 {
        return None;
    }

    let gap_area = (0..(gaps.len() - 1))
        .map(|index| {
            0.5 * (gaps[index] + gaps[index + 1])
                * (actual_cumulative[index + 1] - actual_cumulative[index])
        })
        .sum::<f64>();
    Some(ReferenceGapMetrics {
        gap_mean_m: gap_area / actual_length,
        gap_max_m: gaps.iter().copied().fold(0.0_f64, f64::max),
    })
}

pub(super) fn idealized_reference_curve(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    gravity_mps2: f64,
) -> Option<Vec<(f64, f64)>> {
    let apex_y = idealized_reference_apex_y(start_x, start_y, target_x, target_y, gravity_mps2);
    let (flight_time, vx_mps, vy_up_mps) =
        idealized_reference_kinematics(start_x, start_y, target_x, target_y, apex_y, gravity_mps2)?;
    let g = gravity_mps2.abs().max(1e-6);
    let point_count = ((18.0 + (flight_time * 10.0)).round() as usize).clamp(24, 84);
    let mut points = Vec::with_capacity(point_count);
    for index in 0..point_count {
        let t = if point_count <= 1 {
            0.0
        } else {
            (flight_time * index as f64) / (point_count as f64 - 1.0)
        };
        points.push((
            start_x + (vx_mps * t),
            start_y + (vy_up_mps * t) - (0.5 * g * t * t),
        ));
    }
    if let Some(last) = points.last_mut() {
        *last = (target_x, target_y);
    }
    Some(points)
}

pub(super) fn idealized_reference_apex_y(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    gravity_mps2: f64,
) -> f64 {
    let dx = target_x - start_x;
    let dy = target_y - start_y;
    let base_peak = if target_y > start_y {
        start_y.max(target_y + 1.0)
    } else {
        start_y
    };
    if dx.abs() <= 1e-6 {
        return base_peak;
    }
    let meets_angle_floor = |peak_y: f64| {
        idealized_reference_impact_angle_deg(
            start_x,
            start_y,
            target_x,
            target_y,
            peak_y,
            gravity_mps2,
        )
        .is_some_and(|impact_angle| impact_angle >= 45.0)
    };
    if meets_angle_floor(base_peak) {
        return base_peak;
    }

    let mut low_peak = base_peak;
    let mut growth = 16.0_f64.max(0.25 * dx.abs().max(dy.abs()).max(1.0));
    let mut candidate_peak = base_peak;
    let mut high_peak = None;
    for _ in 0..16 {
        candidate_peak += growth;
        if meets_angle_floor(candidate_peak) {
            high_peak = Some(candidate_peak);
            break;
        }
        low_peak = candidate_peak;
        growth *= 2.0;
    }
    let Some(mut high_peak) = high_peak else {
        return candidate_peak;
    };
    for _ in 0..32 {
        let mid_peak = 0.5 * (low_peak + high_peak);
        if meets_angle_floor(mid_peak) {
            high_peak = mid_peak;
        } else {
            low_peak = mid_peak;
        }
    }
    high_peak
}

pub(super) fn idealized_reference_impact_angle_deg(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    apex_y: f64,
    gravity_mps2: f64,
) -> Option<f64> {
    let (flight_time, vx_mps, vy_up_mps) =
        idealized_reference_kinematics(start_x, start_y, target_x, target_y, apex_y, gravity_mps2)?;
    let g = gravity_mps2.abs().max(1e-6);
    let vy_target = vy_up_mps - (g * flight_time);
    Some(((-vy_target).max(0.0)).atan2(vx_mps.abs()) * (180.0 / std::f64::consts::PI))
}

pub(super) fn idealized_reference_kinematics(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    apex_y: f64,
    gravity_mps2: f64,
) -> Option<(f64, f64, f64)> {
    let g = gravity_mps2.abs().max(1e-6);
    let peak_y = start_y.max(apex_y);
    let vy_up_mps = (2.0 * g * (peak_y - start_y).max(0.0)).sqrt();
    let flight_time = ballistic_end_time(start_y, target_y, vy_up_mps, gravity_mps2)?;
    if !flight_time.is_finite() || flight_time <= 1e-6 {
        return None;
    }
    Some((flight_time, (target_x - start_x) / flight_time, vy_up_mps))
}

pub(super) fn ballistic_end_time(
    start_y: f64,
    target_y: f64,
    vy_mps: f64,
    gravity_mps2: f64,
) -> Option<f64> {
    let g = gravity_mps2.abs().max(1e-6);
    let a = 0.5 * g;
    let b = -vy_mps;
    let c = target_y - start_y;
    let discriminant = (b * b) - (4.0 * a * c);
    if !discriminant.is_finite() || discriminant < 0.0 {
        return None;
    }
    let sqrt = discriminant.sqrt();
    let mut roots = [(-b - sqrt) / (2.0 * a), (-b + sqrt) / (2.0 * a)]
        .into_iter()
        .filter(|value| value.is_finite() && *value > 1e-6)
        .collect::<Vec<_>>();
    roots.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal));
    roots.pop()
}

pub(super) fn polyline_lengths(points: &[(f64, f64)]) -> Option<(Vec<f64>, f64)> {
    if points.len() < 2 {
        return None;
    }
    let mut cumulative = Vec::with_capacity(points.len());
    cumulative.push(0.0);
    let mut length = 0.0;
    for index in 1..points.len() {
        let dx = points[index].0 - points[index - 1].0;
        let dy = points[index].1 - points[index - 1].1;
        length += (dx * dx + dy * dy).sqrt();
        cumulative.push(length);
    }
    (length > 1e-9).then_some((cumulative, length))
}

pub(super) fn project_point_to_polyline(
    point: (f64, f64),
    polyline: &[(f64, f64)],
) -> Option<((f64, f64), f64)> {
    if polyline.is_empty() {
        return None;
    }
    let mut best_projection = polyline[0];
    let mut best_distance = point_distance(point, best_projection);
    for index in 1..polyline.len() {
        let (projection, distance) =
            project_point_to_segment(point, polyline[index - 1], polyline[index]);
        if distance < best_distance {
            best_projection = projection;
            best_distance = distance;
        }
    }
    Some((best_projection, best_distance))
}

pub(super) fn project_point_to_segment(
    point: (f64, f64),
    start: (f64, f64),
    end: (f64, f64),
) -> ((f64, f64), f64) {
    let seg_dx = end.0 - start.0;
    let seg_dy = end.1 - start.1;
    let seg_len_sq = (seg_dx * seg_dx) + (seg_dy * seg_dy);
    if seg_len_sq <= 1e-12 {
        return (start, point_distance(point, start));
    }
    let mix = (((point.0 - start.0) * seg_dx) + ((point.1 - start.1) * seg_dy)) / seg_len_sq;
    let clamped_mix = mix.clamp(0.0, 1.0);
    let projection = (
        start.0 + (seg_dx * clamped_mix),
        start.1 + (seg_dy * clamped_mix),
    );
    (projection, point_distance(point, projection))
}

pub(super) fn point_distance(lhs: (f64, f64), rhs: (f64, f64)) -> f64 {
    let dx = lhs.0 - rhs.0;
    let dy = lhs.1 - rhs.1;
    (dx * dx + dy * dy).sqrt()
}
