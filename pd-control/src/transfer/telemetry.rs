use std::collections::BTreeMap;

use pd_core::{Observation, RunContext};

use crate::kit::{ControllerView, metric, standard_marker};
use crate::terminal::TerminalEntryAssessment;
use crate::{ControllerFrame, TelemetryValue};

use super::{
    TransferBoostCommandSelection, TransferCorridorState, TransferDiagnostics,
    WaypointCaptureSnapshot, WaypointGuidanceTargetState, WaypointJointSearchPrediction,
    WaypointTelemetry, WaypointTransitionAudit,
};

pub(super) fn insert_transfer_metrics(
    frame: &mut ControllerFrame,
    diagnostics: TransferDiagnostics,
    gate: TerminalEntryAssessment,
    corridor: TransferCorridorState,
    boost_selection: Option<TransferBoostCommandSelection>,
    default_scoring_mode: &'static str,
) {
    frame.metrics.insert(
        metric::TRANSFER_ROUTE_DX_M.to_owned(),
        TelemetryValue::from(diagnostics.route_dx_m),
    );
    frame.metrics.insert(
        metric::TRANSFER_ROUTE_DY_M.to_owned(),
        TelemetryValue::from(diagnostics.route_dy_m),
    );
    frame.metrics.insert(
        metric::TRANSFER_SHAPE_ANCHOR_DX_M.to_owned(),
        TelemetryValue::from(
            diagnostics
                .anchor
                .map(|anchor| anchor.route_dx_m)
                .unwrap_or(diagnostics.route_dx_m),
        ),
    );
    frame.metrics.insert(
        metric::TRANSFER_SHAPE_ANCHOR_DY_M.to_owned(),
        TelemetryValue::from(
            diagnostics
                .anchor
                .map(|anchor| anchor.route_dy_m)
                .unwrap_or(diagnostics.route_dy_m),
        ),
    );
    frame.metrics.insert(
        metric::TRANSFER_TARGET_Y_SOLUTION.to_owned(),
        TelemetryValue::from(diagnostics.projection.has_target_y_solution),
    );
    frame.metrics.insert(
        metric::TRANSFER_PROJECTED_TIME_S.to_owned(),
        TelemetryValue::from(diagnostics.projection.projected_time_s.unwrap_or(-1.0)),
    );
    frame.metrics.insert(
        metric::TRANSFER_PROJECTED_DX_M.to_owned(),
        TelemetryValue::from(
            diagnostics
                .projection
                .projected_dx_m
                .unwrap_or(diagnostics.route_dx_m),
        ),
    );
    frame.metrics.insert(
        metric::TRANSFER_IMPACT_ANGLE_DEG.to_owned(),
        TelemetryValue::from(diagnostics.projection.impact_angle_deg.unwrap_or(-1.0)),
    );
    frame.metrics.insert(
        metric::TRANSFER_APEX_OVER_TARGET_M.to_owned(),
        TelemetryValue::from(diagnostics.projection.apex_over_target_m),
    );
    frame.metrics.insert(
        metric::TRANSFER_BOOST_APEX_TARGET_M.to_owned(),
        TelemetryValue::from(diagnostics.boost_quality.apex_target_over_target_m),
    );
    frame.metrics.insert(
        metric::TRANSFER_BOOST_QUALITY.to_owned(),
        TelemetryValue::from(diagnostics.boost_quality.verdict),
    );
    frame.metrics.insert(
        metric::TRANSFER_BOOST_QUALITY_PASS.to_owned(),
        TelemetryValue::from(diagnostics.boost_quality.passed),
    );
    frame.metrics.insert(
        metric::TRANSFER_BOOST_SCORING_MODE.to_owned(),
        TelemetryValue::from(
            boost_selection
                .map(|selection| selection.scoring_mode)
                .unwrap_or(default_scoring_mode),
        ),
    );
    frame.metrics.insert(
        metric::TRANSFER_TERMINAL_GATE_MODE.to_owned(),
        TelemetryValue::from(gate.mode.label()),
    );
    frame.metrics.insert(
        metric::TRANSFER_TERMINAL_GATE_LATEST_SAFE_MARGIN_S.to_owned(),
        TelemetryValue::from(gate.latest_safe_margin_s),
    );
    frame.metrics.insert(
        metric::TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO.to_owned(),
        TelemetryValue::from(gate.required_accel_ratio),
    );
    frame.metrics.insert(
        metric::TRANSFER_TERMINAL_GATE_DEFERRED.to_owned(),
        TelemetryValue::from(gate.deferred),
    );
    frame.metrics.insert(
        metric::TRANSFER_CORRIDOR_MODE.to_owned(),
        TelemetryValue::from(corridor.mode),
    );
    frame.metrics.insert(
        metric::TRANSFER_CORRIDOR_MARGIN_M.to_owned(),
        TelemetryValue::from(corridor.margin_m),
    );
    if let Some(selection) = boost_selection {
        frame.metrics.insert(
            metric::TRANSFER_BOOST_SELECTED_SCORE.to_owned(),
            TelemetryValue::from(selection.selected_score),
        );
        frame.metrics.insert(
            metric::TRANSFER_BOOST_SETTLED_QUALITY.to_owned(),
            TelemetryValue::from(selection.settled_quality.verdict),
        );
        frame.metrics.insert(
            metric::TRANSFER_BOOST_SETTLED_PROJECTED_DX_M.to_owned(),
            TelemetryValue::from(
                selection
                    .settled_projection
                    .projected_dx_m
                    .unwrap_or(diagnostics.route_dx_m),
            ),
        );
    }
}

pub(super) fn insert_waypoint_metrics(
    frame: &mut ControllerFrame,
    waypoint_telemetry: Option<WaypointTelemetry>,
) {
    let Some(telemetry) = waypoint_telemetry else {
        return;
    };
    frame.metrics.insert(
        metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
        TelemetryValue::from(true),
    );
    frame.metrics.insert(
        metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
        TelemetryValue::from(telemetry.active_index),
    );
    frame.metrics.insert(
        metric::WAYPOINT_ACTIVE_LEG_INDEX.to_owned(),
        TelemetryValue::from(telemetry.active_leg_index),
    );
    frame.metrics.insert(
        metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
        TelemetryValue::from(telemetry.capture_status),
    );
    frame.metrics.insert(
        metric::WAYPOINT_CAPTURE_TIME_S.to_owned(),
        TelemetryValue::from(telemetry.capture_time_s.unwrap_or(-1.0)),
    );
    frame.metrics.insert(
        metric::WAYPOINT_CLOSEST_DISTANCE_M.to_owned(),
        TelemetryValue::from(telemetry.closest_distance_m),
    );
    frame.metrics.insert(
        metric::WAYPOINT_DISTANCE_M.to_owned(),
        TelemetryValue::from(telemetry.distance_m),
    );
    frame.metrics.insert(
        metric::WAYPOINT_CROSS_TRACK_M.to_owned(),
        TelemetryValue::from(telemetry.cross_track_m),
    );
    frame.metrics.insert(
        metric::WAYPOINT_PLANE_PROGRESS_M.to_owned(),
        TelemetryValue::from(telemetry.plane_progress_m),
    );
    frame.metrics.insert(
        metric::WAYPOINT_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
        TelemetryValue::from(telemetry.outbound_heading_error_rad),
    );
    frame.metrics.insert(
        metric::WAYPOINT_OUTBOUND_PROGRESS_MPS.to_owned(),
        TelemetryValue::from(telemetry.outbound_progress_mps),
    );
    frame.metrics.insert(
        metric::WAYPOINT_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
        TelemetryValue::from(telemetry.outbound_cross_speed_mps),
    );
    frame.metrics.insert(
        metric::WAYPOINT_SPEED_MPS.to_owned(),
        TelemetryValue::from(telemetry.speed_mps),
    );
    frame.metrics.insert(
        metric::WAYPOINT_VERTICAL_SPEED_MPS.to_owned(),
        TelemetryValue::from(telemetry.vertical_speed_mps),
    );
    frame.metrics.insert(
        metric::WAYPOINT_REMAINING_TO_PLANE_M.to_owned(),
        TelemetryValue::from(telemetry.remaining_to_plane_m),
    );
    frame.metrics.insert(
        metric::WAYPOINT_TIME_TO_PLANE_S.to_owned(),
        TelemetryValue::from(telemetry.time_to_plane_s),
    );
    frame.metrics.insert(
        metric::WAYPOINT_REQUIRED_TURN_DISTANCE_M.to_owned(),
        TelemetryValue::from(telemetry.required_turn_distance_m),
    );
    frame.metrics.insert(
        metric::WAYPOINT_SHAPING_START_DISTANCE_M.to_owned(),
        TelemetryValue::from(telemetry.shaping_start_distance_m),
    );
    frame.metrics.insert(
        metric::WAYPOINT_TURN_MARGIN_M.to_owned(),
        TelemetryValue::from(telemetry.turn_margin_m),
    );
    frame.metrics.insert(
        metric::WAYPOINT_CENTER_X_M.to_owned(),
        TelemetryValue::from(telemetry.center_m.x),
    );
    frame.metrics.insert(
        metric::WAYPOINT_CENTER_Y_M.to_owned(),
        TelemetryValue::from(telemetry.center_m.y),
    );
    frame.metrics.insert(
        metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_X_M.to_owned(),
        TelemetryValue::from(telemetry.nominal_handoff_target_m.x),
    );
    frame.metrics.insert(
        metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_Y_M.to_owned(),
        TelemetryValue::from(telemetry.nominal_handoff_target_m.y),
    );
    frame.metrics.insert(
        metric::WAYPOINT_HANDOFF_TARGET_X_M.to_owned(),
        TelemetryValue::from(telemetry.handoff_target_m.x),
    );
    frame.metrics.insert(
        metric::WAYPOINT_HANDOFF_TARGET_Y_M.to_owned(),
        TelemetryValue::from(telemetry.handoff_target_m.y),
    );
    frame.metrics.insert(
        metric::WAYPOINT_HANDOFF_TARGET_MODE.to_owned(),
        TelemetryValue::from(telemetry.handoff_target_mode),
    );
    frame.metrics.insert(
        metric::WAYPOINT_REMAINING_TO_HANDOFF_M.to_owned(),
        TelemetryValue::from(telemetry.remaining_to_handoff_m),
    );
    frame.metrics.insert(
        metric::WAYPOINT_TIME_TO_HANDOFF_S.to_owned(),
        TelemetryValue::from(telemetry.time_to_handoff_s),
    );
    frame.metrics.insert(
        metric::WAYPOINT_HANDOFF_TURN_MARGIN_M.to_owned(),
        TelemetryValue::from(telemetry.handoff_turn_margin_m),
    );
    frame.metrics.insert(
        metric::WAYPOINT_ENDPOINT_X_M.to_owned(),
        TelemetryValue::from(telemetry.endpoint_m.x),
    );
    frame.metrics.insert(
        metric::WAYPOINT_ENDPOINT_Y_M.to_owned(),
        TelemetryValue::from(telemetry.endpoint_m.y),
    );
    frame.metrics.insert(
        metric::WAYPOINT_STEERING_TARGET_X_M.to_owned(),
        TelemetryValue::from(telemetry.steering_target_m.x),
    );
    frame.metrics.insert(
        metric::WAYPOINT_STEERING_TARGET_Y_M.to_owned(),
        TelemetryValue::from(telemetry.steering_target_m.y),
    );
    if let Some(target_state) = telemetry.target_state {
        insert_waypoint_target_state_metrics(&mut frame.metrics, target_state);
    }
}

impl WaypointTelemetry {
    pub(super) fn from_capture(capture: WaypointCaptureSnapshot) -> Self {
        Self {
            active_index: capture.index as i64,
            active_leg_index: capture.index as i64,
            capture_status: capture.status,
            capture_time_s: Some(capture.capture_time_s),
            closest_distance_m: capture.closest_distance_m,
            distance_m: capture.distance_m,
            cross_track_m: capture.cross_track_m,
            plane_progress_m: capture.plane_progress_m,
            outbound_heading_error_rad: capture.outbound_heading_error_rad,
            outbound_progress_mps: capture.outbound_progress_mps,
            outbound_cross_speed_mps: capture.outbound_cross_speed_mps,
            speed_mps: capture.speed_mps,
            vertical_speed_mps: capture.vertical_speed_mps,
            remaining_to_plane_m: capture.approach.remaining_to_plane_m,
            time_to_plane_s: capture.approach.time_to_plane_s,
            required_turn_distance_m: capture.approach.required_turn_distance_m,
            shaping_start_distance_m: capture.approach.shaping_start_distance_m,
            turn_margin_m: capture.approach.turn_margin_m,
            center_m: capture.center_m,
            nominal_handoff_target_m: capture.nominal_handoff_target_m,
            handoff_target_m: capture.handoff_target_m,
            handoff_target_mode: capture.handoff_target_mode,
            remaining_to_handoff_m: capture.approach.remaining_to_handoff_m,
            time_to_handoff_s: capture.approach.time_to_handoff_s,
            handoff_turn_margin_m: capture.approach.handoff_turn_margin_m,
            endpoint_m: capture.endpoint_m,
            steering_target_m: capture.steering_target_m,
            target_state: capture.target_state,
        }
    }
}

pub(super) fn insert_waypoint_target_state_metrics(
    metrics: &mut BTreeMap<String, TelemetryValue>,
    target_state: WaypointGuidanceTargetState,
) {
    metrics.insert(
        metric::WAYPOINT_TARGET_VX_MPS.to_owned(),
        TelemetryValue::from(target_state.target_velocity_mps.x),
    );
    metrics.insert(
        metric::WAYPOINT_TARGET_VY_MPS.to_owned(),
        TelemetryValue::from(target_state.target_velocity_mps.y),
    );
    metrics.insert(
        metric::WAYPOINT_TARGET_SPEED_MPS.to_owned(),
        TelemetryValue::from(target_state.target_velocity_mps.length()),
    );
    metrics.insert(
        metric::WAYPOINT_TARGET_DEADLINE_REMAINING_S.to_owned(),
        TelemetryValue::from(target_state.deadline_remaining_s),
    );
    metrics.insert(
        metric::WAYPOINT_TARGET_VELOCITY_ERROR_MPS.to_owned(),
        TelemetryValue::from(target_state.velocity_error_mps),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_FEASIBLE.to_owned(),
        TelemetryValue::from(target_state.feasible),
    );
    if let Some(required_accel_ratio) = target_state.final_terminal_required_accel_ratio {
        metrics.insert(
            metric::WAYPOINT_FINAL_TERMINAL_REQUIRED_ACCEL_RATIO.to_owned(),
            TelemetryValue::from(required_accel_ratio),
        );
    }
    if let Some(recoverable) = target_state.final_terminal_recoverable {
        metrics.insert(
            metric::WAYPOINT_FINAL_TERMINAL_RECOVERABLE.to_owned(),
            TelemetryValue::from(recoverable),
        );
    }
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_PLAN_INDEX.to_owned(),
        TelemetryValue::from(target_state.trackability.plan_index as i64),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_PLAN_REVISION.to_owned(),
        TelemetryValue::from(target_state.trackability.plan_revision as i64),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_PLAN_REASON.to_owned(),
        TelemetryValue::from(target_state.trackability.plan_reason.as_str()),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_PLAN_AGE_S.to_owned(),
        TelemetryValue::from(target_state.trackability.plan_age_s),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_REFERENCE_POSITION_ERROR_M.to_owned(),
        TelemetryValue::from(target_state.trackability.reference_position_error_m),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_ERROR_M.to_owned(),
        TelemetryValue::from(target_state.trackability.reference_cross_error_m),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_REFERENCE_VELOCITY_ERROR_MPS.to_owned(),
        TelemetryValue::from(target_state.trackability.reference_velocity_error_mps),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_SPEED_ERROR_MPS.to_owned(),
        TelemetryValue::from(target_state.trackability.reference_cross_speed_error_mps),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_AUTHORITY_MARGIN.to_owned(),
        TelemetryValue::from(target_state.authority_margin),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_THRUST_SATURATED.to_owned(),
        TelemetryValue::from(target_state.thrust_saturated),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_TILT_SATURATED.to_owned(),
        TelemetryValue::from(target_state.tilt_saturated),
    );
    let reachable = target_state.reachable_prediction;
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_MODEL.to_owned(),
        TelemetryValue::from("actuated_rollout"),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_TIME_TO_GO_S.to_owned(),
        TelemetryValue::from(reachable.prediction.time_to_event_s),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_TRIGGERED.to_owned(),
        TelemetryValue::from(reachable.prediction.assessment.triggered),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_CONTRACT_PASS.to_owned(),
        TelemetryValue::from(reachable.prediction.assessment.contract_pass()),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_CONTRACT_REASONS.to_owned(),
        TelemetryValue::from(reachable.prediction.assessment.reasons()),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
        TelemetryValue::from(reachable.prediction.stats.outbound_heading_error_rad),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
        TelemetryValue::from(reachable.prediction.stats.outbound_cross_speed_mps),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_REQUIRED_ACCEL_RATIO_MAX.to_owned(),
        TelemetryValue::from(reachable.required_accel_ratio_max),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_THRUST_SATURATED_TIME_S.to_owned(),
        TelemetryValue::from(reachable.thrust_saturated_time_s),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_TILT_SATURATED_TIME_S.to_owned(),
        TelemetryValue::from(reachable.tilt_saturated_time_s),
    );
    if let Some(continuation) = target_state.continuation_prediction {
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_NEXT_INDEX.to_owned(),
            TelemetryValue::from(continuation.next_waypoint_index as i64),
        );
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_CONTRACT_PASS.to_owned(),
            TelemetryValue::from(
                continuation
                    .prediction
                    .prediction
                    .assessment
                    .contract_pass(),
            ),
        );
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_CONTRACT_REASONS.to_owned(),
            TelemetryValue::from(continuation.prediction.prediction.assessment.reasons()),
        );
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
            TelemetryValue::from(
                continuation
                    .prediction
                    .prediction
                    .stats
                    .outbound_heading_error_rad,
            ),
        );
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_REQUIRED_ACCEL_RATIO_MAX.to_owned(),
            TelemetryValue::from(continuation.prediction.required_accel_ratio_max),
        );
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_PASSING_CANDIDATE_COUNT.to_owned(),
            TelemetryValue::from(continuation.passing_candidate_count as i64),
        );
    }
    if let Some(joint) = target_state.joint_prediction {
        insert_waypoint_joint_prediction_metrics(metrics, joint);
    }
    let prediction = target_state.prediction;
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_TIME_TO_GO_S.to_owned(),
        TelemetryValue::from(prediction.time_to_event_s),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_DEADLINE_LEAD_S.to_owned(),
        TelemetryValue::from(prediction.deadline_lead_s),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_TRIGGERED.to_owned(),
        TelemetryValue::from(prediction.assessment.triggered),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_SPATIAL_PASS.to_owned(),
        TelemetryValue::from(prediction.assessment.spatial_pass),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_ENVELOPE_PASS.to_owned(),
        TelemetryValue::from(prediction.assessment.envelope_pass()),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS.to_owned(),
        TelemetryValue::from(prediction.assessment.contract_pass()),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_REASONS.to_owned(),
        TelemetryValue::from(prediction.assessment.reasons()),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_DISTANCE_M.to_owned(),
        TelemetryValue::from(prediction.stats.distance_m),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_CROSS_TRACK_M.to_owned(),
        TelemetryValue::from(prediction.stats.cross_track_m),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_PLANE_PROGRESS_M.to_owned(),
        TelemetryValue::from(prediction.stats.plane_progress_m),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
        TelemetryValue::from(prediction.stats.outbound_heading_error_rad),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_PROGRESS_MPS.to_owned(),
        TelemetryValue::from(prediction.stats.outbound_progress_mps),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
        TelemetryValue::from(prediction.stats.outbound_cross_speed_mps),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_SPEED_MPS.to_owned(),
        TelemetryValue::from(prediction.stats.speed_mps),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_VERTICAL_SPEED_MPS.to_owned(),
        TelemetryValue::from(prediction.stats.vertical_speed_mps),
    );
}

fn insert_waypoint_joint_prediction_metrics(
    metrics: &mut BTreeMap<String, TelemetryValue>,
    joint: WaypointJointSearchPrediction,
) {
    metrics.insert(
        metric::WAYPOINT_JOINT_NEXT_INDEX.to_owned(),
        TelemetryValue::from(joint.next_waypoint_index as i64),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_EVALUATED_CANDIDATE_COUNT.to_owned(),
        TelemetryValue::from(joint.evaluated_candidate_count as i64),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_PASSING_CANDIDATE_COUNT.to_owned(),
        TelemetryValue::from(joint.passing_candidate_count as i64),
    );
    let Some(selected) = joint.selected else {
        return;
    };
    metrics.insert(
        metric::WAYPOINT_JOINT_CONTRACT_PASS.to_owned(),
        TelemetryValue::from(selected.contract_pass()),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_ENDPOINT_X_M.to_owned(),
        TelemetryValue::from(selected.current.endpoint_m.x),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_ENDPOINT_Y_M.to_owned(),
        TelemetryValue::from(selected.current.endpoint_m.y),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_TARGET_VX_MPS.to_owned(),
        TelemetryValue::from(selected.current.candidate.target_velocity_mps.x),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_TARGET_VY_MPS.to_owned(),
        TelemetryValue::from(selected.current.candidate.target_velocity_mps.y),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_TIME_TO_GO_S.to_owned(),
        TelemetryValue::from(selected.current.candidate.time_to_go_s),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
        TelemetryValue::from(
            selected
                .continuation
                .prediction
                .stats
                .outbound_heading_error_rad,
        ),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_REQUIRED_ACCEL_RATIO_MAX.to_owned(),
        TelemetryValue::from(selected.required_accel_ratio_max()),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_TOTAL_SATURATED_TIME_S.to_owned(),
        TelemetryValue::from(selected.total_saturated_time_s()),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_CONTINUATION_PASSING_CANDIDATE_COUNT.to_owned(),
        TelemetryValue::from(selected.continuation_passing_candidate_count as i64),
    );
}

fn insert_waypoint_transition_audit_metrics(
    metrics: &mut BTreeMap<String, TelemetryValue>,
    audit: WaypointTransitionAudit,
) {
    metrics.insert(
        metric::WAYPOINT_TRANSITION_NEXT_INDEX.to_owned(),
        TelemetryValue::from(audit.next_waypoint_index as i64),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_POSITION_ERROR_M.to_owned(),
        TelemetryValue::from(audit.position_error_m),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_VELOCITY_ERROR_MPS.to_owned(),
        TelemetryValue::from(audit.velocity_error_mps),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_ATTITUDE_ERROR_RAD.to_owned(),
        TelemetryValue::from(audit.attitude_error_rad),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_MASS_ERROR_KG.to_owned(),
        TelemetryValue::from(audit.mass_error_kg),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_FUEL_ERROR_KG.to_owned(),
        TelemetryValue::from(audit.fuel_error_kg),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_EVENT_TIME_ERROR_S.to_owned(),
        TelemetryValue::from(audit.event_time_error_s),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_CONTINUATION_CONTRACT_PASS.to_owned(),
        TelemetryValue::from(
            audit
                .continuation_prediction
                .prediction
                .assessment
                .contract_pass(),
        ),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_CONTINUATION_CONTRACT_REASONS.to_owned(),
        TelemetryValue::from(
            audit
                .continuation_prediction
                .prediction
                .assessment
                .reasons(),
        ),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
        TelemetryValue::from(
            audit
                .continuation_prediction
                .prediction
                .stats
                .outbound_heading_error_rad,
        ),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_CONTINUATION_REQUIRED_ACCEL_RATIO_MAX.to_owned(),
        TelemetryValue::from(audit.continuation_prediction.required_accel_ratio_max),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_CONTINUATION_PASSING_CANDIDATE_COUNT.to_owned(),
        TelemetryValue::from(audit.passing_candidate_count as i64),
    );
}

pub(super) fn waypoint_handoff_marker(
    ctx: &RunContext,
    observation: &Observation,
    capture: WaypointCaptureSnapshot,
) -> crate::ControllerMarker {
    let waypoint_id = ctx
        .mission
        .transfer_route
        .as_ref()
        .and_then(|route| route.waypoints.get(capture.index))
        .map_or("unknown", |waypoint| waypoint.id.as_str());
    let view = ControllerView::new(ctx, observation);
    let mut metadata = BTreeMap::from([
        ("kind".to_owned(), TelemetryValue::from("waypoint_handoff")),
        ("waypoint.id".to_owned(), TelemetryValue::from(waypoint_id)),
        (
            "waypoint.index".to_owned(),
            TelemetryValue::from(capture.index as i64),
        ),
        (
            metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
            TelemetryValue::from(capture.status),
        ),
        (
            metric::WAYPOINT_CAPTURE_TIME_S.to_owned(),
            TelemetryValue::from(capture.capture_time_s),
        ),
        (
            "waypoint.position_x_m".to_owned(),
            TelemetryValue::from(observation.position_m.x),
        ),
        (
            "waypoint.position_y_m".to_owned(),
            TelemetryValue::from(observation.position_m.y),
        ),
        (
            "waypoint.velocity_x_mps".to_owned(),
            TelemetryValue::from(observation.velocity_mps.x),
        ),
        (
            "waypoint.velocity_y_mps".to_owned(),
            TelemetryValue::from(observation.velocity_mps.y),
        ),
        (
            metric::WAYPOINT_DISTANCE_M.to_owned(),
            TelemetryValue::from(capture.distance_m),
        ),
        (
            metric::WAYPOINT_CROSS_TRACK_M.to_owned(),
            TelemetryValue::from(capture.cross_track_m),
        ),
        (
            metric::WAYPOINT_PLANE_PROGRESS_M.to_owned(),
            TelemetryValue::from(capture.plane_progress_m),
        ),
        (
            metric::WAYPOINT_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
            TelemetryValue::from(capture.outbound_heading_error_rad),
        ),
        (
            metric::WAYPOINT_OUTBOUND_PROGRESS_MPS.to_owned(),
            TelemetryValue::from(capture.outbound_progress_mps),
        ),
        (
            metric::WAYPOINT_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
            TelemetryValue::from(capture.outbound_cross_speed_mps),
        ),
        (
            metric::WAYPOINT_SPEED_MPS.to_owned(),
            TelemetryValue::from(capture.speed_mps),
        ),
        (
            metric::WAYPOINT_VERTICAL_SPEED_MPS.to_owned(),
            TelemetryValue::from(capture.vertical_speed_mps),
        ),
        (
            metric::WAYPOINT_TURN_MARGIN_M.to_owned(),
            TelemetryValue::from(capture.approach.turn_margin_m),
        ),
        (
            metric::WAYPOINT_CENTER_X_M.to_owned(),
            TelemetryValue::from(capture.center_m.x),
        ),
        (
            metric::WAYPOINT_CENTER_Y_M.to_owned(),
            TelemetryValue::from(capture.center_m.y),
        ),
        (
            metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_X_M.to_owned(),
            TelemetryValue::from(capture.nominal_handoff_target_m.x),
        ),
        (
            metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_Y_M.to_owned(),
            TelemetryValue::from(capture.nominal_handoff_target_m.y),
        ),
        (
            metric::WAYPOINT_HANDOFF_TARGET_X_M.to_owned(),
            TelemetryValue::from(capture.handoff_target_m.x),
        ),
        (
            metric::WAYPOINT_HANDOFF_TARGET_Y_M.to_owned(),
            TelemetryValue::from(capture.handoff_target_m.y),
        ),
        (
            metric::WAYPOINT_HANDOFF_TARGET_MODE.to_owned(),
            TelemetryValue::from(capture.handoff_target_mode),
        ),
        (
            metric::WAYPOINT_REMAINING_TO_HANDOFF_M.to_owned(),
            TelemetryValue::from(capture.approach.remaining_to_handoff_m),
        ),
        (
            metric::WAYPOINT_TIME_TO_HANDOFF_S.to_owned(),
            TelemetryValue::from(capture.approach.time_to_handoff_s),
        ),
        (
            metric::WAYPOINT_HANDOFF_TURN_MARGIN_M.to_owned(),
            TelemetryValue::from(capture.approach.handoff_turn_margin_m),
        ),
        (
            metric::WAYPOINT_GUIDANCE_REPLAN_COUNT.to_owned(),
            TelemetryValue::from(capture.guidance_replan_count as i64),
        ),
        (
            metric::WAYPOINT_HANDOFF_RESOLUTION_REASON.to_owned(),
            TelemetryValue::from(capture.resolution_reason),
        ),
    ]);
    if let Some(entry) = capture.window_entry {
        metadata.extend([
            (
                metric::WAYPOINT_WINDOW_ENTRY_TIME_S.to_owned(),
                TelemetryValue::from(entry.time_s),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_POSITION_X_M.to_owned(),
                TelemetryValue::from(entry.position_m.x),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_POSITION_Y_M.to_owned(),
                TelemetryValue::from(entry.position_m.y),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_VELOCITY_X_MPS.to_owned(),
                TelemetryValue::from(entry.velocity_mps.x),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_VELOCITY_Y_MPS.to_owned(),
                TelemetryValue::from(entry.velocity_mps.y),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_DISTANCE_M.to_owned(),
                TelemetryValue::from(entry.stats.distance_m),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_CROSS_TRACK_M.to_owned(),
                TelemetryValue::from(entry.stats.cross_track_m),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_PLANE_PROGRESS_M.to_owned(),
                TelemetryValue::from(entry.stats.plane_progress_m),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
                TelemetryValue::from(entry.stats.outbound_heading_error_rad),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_PROGRESS_MPS.to_owned(),
                TelemetryValue::from(entry.stats.outbound_progress_mps),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
                TelemetryValue::from(entry.stats.outbound_cross_speed_mps),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_SPEED_MPS.to_owned(),
                TelemetryValue::from(entry.stats.speed_mps),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_VERTICAL_SPEED_MPS.to_owned(),
                TelemetryValue::from(entry.stats.vertical_speed_mps),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_PASS.to_owned(),
                TelemetryValue::from(entry.assessment.contract_pass()),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_REASONS.to_owned(),
                TelemetryValue::from(entry.assessment.reasons()),
            ),
            (
                metric::WAYPOINT_HANDOFF_WINDOW_DURATION_S.to_owned(),
                TelemetryValue::from((capture.capture_time_s - entry.time_s).max(0.0)),
            ),
        ]);
    }
    if let Some(target_state) = capture.target_state {
        insert_waypoint_target_state_metrics(&mut metadata, target_state);
    }
    if let Some(audit) = capture.transition_audit {
        insert_waypoint_transition_audit_metrics(&mut metadata, audit);
    }
    standard_marker(
        crate::kit::marker::WAYPOINT_HANDOFF,
        &format!("waypoint handoff: {waypoint_id}"),
        &view,
        metadata,
    )
}
