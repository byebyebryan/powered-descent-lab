use std::collections::BTreeMap;

use pd_core::{Command, Observation, RunContext, Vec2};

use crate::{ControllerFrame, ControllerMarker, TelemetryValue};

pub mod phase {
    pub const IDLE: &str = "idle";
    pub const ACQUIRE: &str = "acquire";
    pub const TRANSLATE: &str = "translate";
    pub const DESCENT: &str = "descent";
    pub const FLARE: &str = "flare";
    pub const TOUCHDOWN: &str = "touchdown";
}

pub mod metric {
    pub const ALTITUDE_M: &str = "kit.altitude_m";
    pub const HEIGHT_ABOVE_TARGET_M: &str = "kit.height_above_target_m";
    pub const TARGET_DX_M: &str = "kit.target_dx_m";
    pub const PAD_MARGIN_M: &str = "kit.pad_margin_m";
    pub const FUEL_FRACTION: &str = "kit.fuel_fraction";
    pub const VERTICAL_SPEED_MPS: &str = "kit.vertical_speed_mps";
    pub const TANGENTIAL_SPEED_MPS: &str = "kit.tangential_speed_mps";
    pub const NORMAL_SPEED_MPS: &str = "kit.normal_speed_mps";
    pub const HOVER_THROTTLE: &str = "guidance.hover_throttle";
    pub const GUIDANCE_ACTIVE: &str = "guidance.active";
    pub const DESIRED_TANGENTIAL_SPEED_MPS: &str = "guidance.desired_tangential_speed_mps";
    pub const DESIRED_VERTICAL_SPEED_MPS: &str = "guidance.desired_vertical_speed_mps";
    pub const DESIRED_ATTITUDE_RAD: &str = "guidance.desired_attitude_rad";
    pub const VERTICAL_ERROR_MPS: &str = "guidance.vertical_error_mps";
    pub const LATERAL_ERROR_MPS: &str = "guidance.lateral_error_mps";
    pub const PROJECTED_DX_M: &str = "guidance.projected_dx_m";
    pub const PROJECTED_TIME_S: &str = "guidance.projected_time_s";
    pub const GUIDANCE_MODE: &str = "guidance.mode";
    pub const GUIDANCE_BURN_TIME_S: &str = "guidance.burn_time_s";
    pub const GUIDANCE_CANDIDATE_BURN_TIME_S: &str = "guidance.candidate_burn_time_s";
    pub const GUIDANCE_PLAN_ACTIVE: &str = "guidance.plan_active";
    pub const GUIDANCE_PLAN_ARRIVAL_TIME_S: &str = "guidance.plan_arrival_time_s";
    pub const GUIDANCE_PLAN_REPLAN_COUNT: &str = "guidance.plan_replan_count";
    pub const GUIDANCE_PLAN_RELEASE_REASON: &str = "guidance.plan_release_reason";
    pub const GUIDANCE_VERTICAL_BRAKING_MARGIN_M: &str = "guidance.vertical_braking_margin_m";
    pub const GUIDANCE_REQUIRED_ACCEL_RATIO: &str = "guidance.required_accel_ratio";
    pub const GUIDANCE_MAX_TILT_RAD: &str = "guidance.max_tilt_rad";
    pub const GUIDANCE_LATEST_SAFE_MARGIN_S: &str = "guidance.latest_safe_margin_s";
    pub const GUIDANCE_TERRAIN_MIN_CLEARANCE_M: &str = "guidance.terrain_min_clearance_m";
    pub const GUIDANCE_TERRAIN_FIRST_VIOLATION_TIME_S: &str =
        "guidance.terrain_first_violation_time_s";
    pub const GUIDANCE_TERRAIN_CLEARANCE_SAFE: &str = "guidance.terrain_clearance_safe";
    pub const TRANSFER_PHASE: &str = "transfer.phase";
    pub const TRANSFER_ROUTE_DX_M: &str = "transfer.route_dx_m";
    pub const TRANSFER_ROUTE_DY_M: &str = "transfer.route_dy_m";
    pub const TRANSFER_SHAPE_ANCHOR_DX_M: &str = "transfer.shape_anchor_dx_m";
    pub const TRANSFER_SHAPE_ANCHOR_DY_M: &str = "transfer.shape_anchor_dy_m";
    pub const TRANSFER_TARGET_Y_SOLUTION: &str = "transfer.target_y_solution";
    pub const TRANSFER_PROJECTED_TIME_S: &str = "transfer.projected_time_s";
    pub const TRANSFER_PROJECTED_DX_M: &str = "transfer.projected_dx_m";
    pub const TRANSFER_IMPACT_ANGLE_DEG: &str = "transfer.impact_angle_deg";
    pub const TRANSFER_APEX_OVER_TARGET_M: &str = "transfer.apex_over_target_m";
    pub const TRANSFER_BOOST_APEX_TARGET_M: &str = "transfer.boost_apex_target_m";
    pub const TRANSFER_BOOST_QUALITY: &str = "transfer.boost_quality";
    pub const TRANSFER_BOOST_QUALITY_PASS: &str = "transfer.boost_quality_pass";
    pub const TRANSFER_BOOST_SCORING_MODE: &str = "transfer.boost_scoring_mode";
    pub const TRANSFER_BOOST_SELECTED_SCORE: &str = "transfer.boost_selected_score";
    pub const TRANSFER_BOOST_SETTLED_QUALITY: &str = "transfer.boost_settled_quality";
    pub const TRANSFER_BOOST_SETTLED_PROJECTED_DX_M: &str = "transfer.boost_settled_projected_dx_m";
    pub const TRANSFER_TERMINAL_GATE_MODE: &str = "transfer.terminal_gate_mode";
    pub const TRANSFER_TERMINAL_GATE_LATEST_SAFE_MARGIN_S: &str =
        "transfer.terminal_gate_latest_safe_margin_s";
    pub const TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO: &str =
        "transfer.terminal_gate_required_accel_ratio";
    pub const TRANSFER_TERMINAL_GATE_DEFERRED: &str = "transfer.terminal_gate_deferred";
    pub const TRANSFER_CORRIDOR_MODE: &str = "transfer.corridor_mode";
    pub const TRANSFER_CORRIDOR_MARGIN_M: &str = "transfer.corridor_margin_m";
    pub const WAYPOINT_GUIDANCE_ENABLED: &str = "waypoint.guidance_enabled";
    pub const WAYPOINT_ACTIVE_INDEX: &str = "waypoint.active_index";
    pub const WAYPOINT_ACTIVE_LEG_INDEX: &str = "waypoint.active_leg_index";
    pub const WAYPOINT_CAPTURE_STATUS: &str = "waypoint.capture_status";
    pub const WAYPOINT_CAPTURE_TIME_S: &str = "waypoint.capture_time_s";
    pub const WAYPOINT_CLOSEST_DISTANCE_M: &str = "waypoint.closest_distance_m";
    pub const WAYPOINT_DISTANCE_M: &str = "waypoint.distance_m";
    pub const WAYPOINT_CROSS_TRACK_M: &str = "waypoint.cross_track_m";
    pub const WAYPOINT_PLANE_PROGRESS_M: &str = "waypoint.plane_progress_m";
    pub const WAYPOINT_OUTBOUND_HEADING_ERROR_RAD: &str = "waypoint.outbound_heading_error_rad";
    pub const WAYPOINT_OUTBOUND_PROGRESS_MPS: &str = "waypoint.outbound_progress_mps";
    pub const WAYPOINT_OUTBOUND_CROSS_SPEED_MPS: &str = "waypoint.outbound_cross_speed_mps";
    pub const WAYPOINT_SPEED_MPS: &str = "waypoint.speed_mps";
    pub const WAYPOINT_VERTICAL_SPEED_MPS: &str = "waypoint.vertical_speed_mps";
    pub const WAYPOINT_REMAINING_TO_PLANE_M: &str = "waypoint.remaining_to_plane_m";
    pub const WAYPOINT_TIME_TO_PLANE_S: &str = "waypoint.time_to_plane_s";
    pub const WAYPOINT_REQUIRED_TURN_DISTANCE_M: &str = "waypoint.required_turn_distance_m";
    pub const WAYPOINT_SHAPING_START_DISTANCE_M: &str = "waypoint.shaping_start_distance_m";
    pub const WAYPOINT_TURN_MARGIN_M: &str = "waypoint.turn_margin_m";
    pub const WAYPOINT_CENTER_X_M: &str = "waypoint.center_x_m";
    pub const WAYPOINT_CENTER_Y_M: &str = "waypoint.center_y_m";
    pub const WAYPOINT_NOMINAL_HANDOFF_TARGET_X_M: &str = "waypoint.nominal_handoff_target_x_m";
    pub const WAYPOINT_NOMINAL_HANDOFF_TARGET_Y_M: &str = "waypoint.nominal_handoff_target_y_m";
    pub const WAYPOINT_HANDOFF_TARGET_X_M: &str = "waypoint.handoff_target_x_m";
    pub const WAYPOINT_HANDOFF_TARGET_Y_M: &str = "waypoint.handoff_target_y_m";
    pub const WAYPOINT_HANDOFF_TARGET_MODE: &str = "waypoint.handoff_target_mode";
    pub const WAYPOINT_REMAINING_TO_HANDOFF_M: &str = "waypoint.remaining_to_handoff_m";
    pub const WAYPOINT_TIME_TO_HANDOFF_S: &str = "waypoint.time_to_handoff_s";
    pub const WAYPOINT_HANDOFF_TURN_MARGIN_M: &str = "waypoint.handoff_turn_margin_m";
    pub const WAYPOINT_ENDPOINT_X_M: &str = "waypoint.endpoint_x_m";
    pub const WAYPOINT_ENDPOINT_Y_M: &str = "waypoint.endpoint_y_m";
    pub const WAYPOINT_STEERING_TARGET_X_M: &str = "waypoint.steering_target_x_m";
    pub const WAYPOINT_STEERING_TARGET_Y_M: &str = "waypoint.steering_target_y_m";
    pub const WAYPOINT_GUIDANCE_MODE: &str = "waypoint.guidance_mode";
    pub const WAYPOINT_TARGET_VX_MPS: &str = "waypoint.target_vx_mps";
    pub const WAYPOINT_TARGET_VY_MPS: &str = "waypoint.target_vy_mps";
    pub const WAYPOINT_TARGET_SPEED_MPS: &str = "waypoint.target_speed_mps";
    pub const WAYPOINT_GUIDANCE_TIME_TO_GO_S: &str = "waypoint.guidance_time_to_go_s";
    pub const WAYPOINT_GUIDANCE_REQUIRED_ACCEL_RATIO: &str =
        "waypoint.guidance_required_accel_ratio";
    pub const WAYPOINT_GUIDANCE_FEASIBLE: &str = "waypoint.guidance_feasible";
    pub const WAYPOINT_GUIDANCE_PLAN_INDEX: &str = "waypoint.guidance_plan_index";
    pub const WAYPOINT_GUIDANCE_PLAN_REVISION: &str = "waypoint.guidance_plan_revision";
    pub const WAYPOINT_GUIDANCE_PLAN_REASON: &str = "waypoint.guidance_plan_reason";
    pub const WAYPOINT_GUIDANCE_PLAN_AGE_S: &str = "waypoint.guidance_plan_age_s";
    pub const WAYPOINT_GUIDANCE_REFERENCE_POSITION_ERROR_M: &str =
        "waypoint.guidance_reference_position_error_m";
    pub const WAYPOINT_GUIDANCE_REFERENCE_CROSS_ERROR_M: &str =
        "waypoint.guidance_reference_cross_error_m";
    pub const WAYPOINT_GUIDANCE_REFERENCE_VELOCITY_ERROR_MPS: &str =
        "waypoint.guidance_reference_velocity_error_mps";
    pub const WAYPOINT_GUIDANCE_REFERENCE_CROSS_SPEED_ERROR_MPS: &str =
        "waypoint.guidance_reference_cross_speed_error_mps";
    pub const WAYPOINT_GUIDANCE_AUTHORITY_MARGIN: &str = "waypoint.guidance_authority_margin";
    pub const WAYPOINT_GUIDANCE_THRUST_SATURATED: &str = "waypoint.guidance_thrust_saturated";
    pub const WAYPOINT_GUIDANCE_TILT_SATURATED: &str = "waypoint.guidance_tilt_saturated";
    pub const WAYPOINT_TARGET_DEADLINE_REMAINING_S: &str = "waypoint.target_deadline_remaining_s";
    pub const WAYPOINT_TARGET_VELOCITY_ERROR_MPS: &str = "waypoint.target_velocity_error_mps";
    pub const WAYPOINT_PREDICTED_HANDOFF_TIME_TO_GO_S: &str =
        "waypoint.predicted_handoff_time_to_go_s";
    pub const WAYPOINT_PREDICTED_HANDOFF_DEADLINE_LEAD_S: &str =
        "waypoint.predicted_handoff_deadline_lead_s";
    pub const WAYPOINT_PREDICTED_HANDOFF_TRIGGERED: &str = "waypoint.predicted_handoff_triggered";
    pub const WAYPOINT_PREDICTED_HANDOFF_SPATIAL_PASS: &str =
        "waypoint.predicted_handoff_spatial_pass";
    pub const WAYPOINT_PREDICTED_HANDOFF_ENVELOPE_PASS: &str =
        "waypoint.predicted_handoff_envelope_pass";
    pub const WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS: &str =
        "waypoint.predicted_handoff_contract_pass";
    pub const WAYPOINT_PREDICTED_HANDOFF_CONTRACT_REASONS: &str =
        "waypoint.predicted_handoff_contract_reasons";
    pub const WAYPOINT_PREDICTED_HANDOFF_DISTANCE_M: &str = "waypoint.predicted_handoff_distance_m";
    pub const WAYPOINT_PREDICTED_HANDOFF_CROSS_TRACK_M: &str =
        "waypoint.predicted_handoff_cross_track_m";
    pub const WAYPOINT_PREDICTED_HANDOFF_PLANE_PROGRESS_M: &str =
        "waypoint.predicted_handoff_plane_progress_m";
    pub const WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_HEADING_ERROR_RAD: &str =
        "waypoint.predicted_handoff_outbound_heading_error_rad";
    pub const WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_PROGRESS_MPS: &str =
        "waypoint.predicted_handoff_outbound_progress_mps";
    pub const WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_CROSS_SPEED_MPS: &str =
        "waypoint.predicted_handoff_outbound_cross_speed_mps";
    pub const WAYPOINT_PREDICTED_HANDOFF_SPEED_MPS: &str = "waypoint.predicted_handoff_speed_mps";
    pub const WAYPOINT_PREDICTED_HANDOFF_VERTICAL_SPEED_MPS: &str =
        "waypoint.predicted_handoff_vertical_speed_mps";
    pub const WAYPOINT_PATH_CORRECTION_MPS2: &str = "waypoint.path_correction_mps2";
    pub const WAYPOINT_GUIDANCE_REPLAN_COUNT: &str = "waypoint.guidance_replan_count";
}

pub mod marker {
    pub const GUIDANCE_PLAN_RELEASE: &str = "guidance/plan_release";
    pub const LATERAL_CAPTURE: &str = "gate/lateral_capture";
    pub const TERMINAL_GATE: &str = "gate/terminal_descent";
    pub const WAYPOINT_HANDOFF: &str = "waypoint/handoff";
}

pub struct ControllerView<'a> {
    pub ctx: &'a RunContext,
    pub observation: &'a Observation,
    target_surface_normal: Vec2,
    target_surface_tangent: Vec2,
}

impl<'a> ControllerView<'a> {
    pub fn new(ctx: &'a RunContext, observation: &'a Observation) -> Self {
        let target_surface_normal = ctx
            .world
            .terrain
            .sample_surface_normal(ctx.target_pad.center_x_m);
        let target_surface_tangent = Vec2::new(target_surface_normal.y, -target_surface_normal.x);
        Self {
            ctx,
            observation,
            target_surface_normal,
            target_surface_tangent,
        }
    }

    pub fn altitude_m(&self) -> f64 {
        self.observation.touchdown_clearance_m.max(0.0)
    }

    pub fn touchdown_clearance_m(&self) -> f64 {
        self.observation.touchdown_clearance_m.max(0.0)
    }

    pub fn height_above_target_m(&self) -> f64 {
        self.observation.height_above_target_m
    }

    pub fn target_dx_m(&self) -> f64 {
        self.observation.target_dx_m
    }

    pub fn pad_margin_m(&self) -> f64 {
        self.observation.target_pad_half_width_m - self.target_dx_m().abs()
    }

    pub fn vertical_speed_mps(&self) -> f64 {
        self.observation.velocity_mps.y
    }

    pub fn tangential_velocity_mps(&self) -> f64 {
        dot(self.observation.velocity_mps, self.target_surface_tangent)
    }

    pub fn tangential_speed_mps(&self) -> f64 {
        self.tangential_velocity_mps().abs()
    }

    pub fn normal_speed_mps(&self) -> f64 {
        (-dot(self.observation.velocity_mps, self.target_surface_normal)).max(0.0)
    }

    pub fn fuel_fraction(&self) -> f64 {
        if self.ctx.vehicle.max_fuel_kg <= f64::EPSILON {
            0.0
        } else {
            (self.observation.fuel_kg / self.ctx.vehicle.max_fuel_kg).clamp(0.0, 1.0)
        }
    }

    pub fn hover_throttle_frac(&self) -> f64 {
        let max_accel_mps2 = self.ctx.vehicle.max_thrust_n / self.observation.mass_kg.max(1.0);
        self.observation.gravity_mps2 / max_accel_mps2.max(f64::EPSILON)
    }

    pub fn terrain_height_at(&self, x_m: f64) -> f64 {
        self.ctx.world.terrain.sample_height(x_m)
    }

    pub fn terrain_slope_at(&self, x_m: f64) -> f64 {
        self.ctx.world.terrain.sample_slope(x_m)
    }

    pub fn terrain_normal_at(&self, x_m: f64) -> Vec2 {
        self.ctx.world.terrain.sample_surface_normal(x_m)
    }

    pub fn terrain_profile(&self, x0_m: f64, x1_m: f64, step_m: f64) -> Vec<Vec2> {
        let step_m = step_m.abs().max(0.5);
        let direction = if x1_m >= x0_m { 1.0 } else { -1.0 };
        let mut points = Vec::new();
        let mut x_m = x0_m;
        loop {
            points.push(Vec2::new(x_m, self.terrain_height_at(x_m)));
            if (direction > 0.0 && x_m >= x1_m) || (direction < 0.0 && x_m <= x1_m) {
                break;
            }
            let next_x_m = x_m + (step_m * direction);
            x_m = if direction > 0.0 {
                next_x_m.min(x1_m)
            } else {
                next_x_m.max(x1_m)
            };
        }
        points
    }

    pub fn desired_attitude_for_tangential_speed(
        &self,
        desired_tangential_speed_mps: f64,
        velocity_gain: f64,
        attitude_limit_rad: f64,
    ) -> f64 {
        ((desired_tangential_speed_mps - self.tangential_velocity_mps()) * velocity_gain)
            .clamp(-attitude_limit_rad, attitude_limit_rad)
    }

    pub fn throttle_for_vertical_target(
        &self,
        desired_vertical_speed_mps: f64,
        vertical_speed_gain: f64,
        tilt_throttle_gain: f64,
        target_attitude_rad: f64,
    ) -> (f64, f64) {
        let vertical_error_mps = desired_vertical_speed_mps - self.vertical_speed_mps();
        let throttle_frac = (self.hover_throttle_frac()
            + (vertical_error_mps * vertical_speed_gain)
            + (target_attitude_rad.abs() * tilt_throttle_gain))
            .clamp(0.0, 1.0);
        (throttle_frac, vertical_error_mps)
    }
}

pub struct ControllerFrameBuilder {
    frame: ControllerFrame,
}

impl ControllerFrameBuilder {
    pub fn new(command: Command) -> Self {
        Self {
            frame: ControllerFrame::command_only(command),
        }
    }

    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.frame.status = status.into();
        self
    }

    pub fn phase(mut self, phase: impl Into<String>) -> Self {
        self.frame.phase = Some(phase.into());
        self
    }

    pub fn metric(mut self, key: impl Into<String>, value: impl Into<TelemetryValue>) -> Self {
        self.frame.metrics.insert(key.into(), value.into());
        self
    }

    pub fn marker(mut self, marker: ControllerMarker) -> Self {
        self.frame.markers.push(marker);
        self
    }

    pub fn standard_kinematics(mut self, view: &ControllerView<'_>) -> Self {
        self = self.metric(metric::ALTITUDE_M, view.altitude_m());
        self = self.metric(metric::HEIGHT_ABOVE_TARGET_M, view.height_above_target_m());
        self = self.metric(metric::TARGET_DX_M, view.target_dx_m());
        self = self.metric(metric::PAD_MARGIN_M, view.pad_margin_m());
        self = self.metric(metric::FUEL_FRACTION, view.fuel_fraction());
        self = self.metric(metric::VERTICAL_SPEED_MPS, view.vertical_speed_mps());
        self = self.metric(metric::TANGENTIAL_SPEED_MPS, view.tangential_speed_mps());
        self.metric(metric::NORMAL_SPEED_MPS, view.normal_speed_mps())
    }

    pub fn phase_transition_marker(
        mut self,
        previous_phase: Option<&str>,
        phase: &str,
        view: &ControllerView<'_>,
    ) -> Self {
        if previous_phase != Some(phase) {
            self = self.marker(standard_marker(
                &format!("phase/{phase}"),
                &format!("phase: {phase}"),
                view,
                BTreeMap::from([
                    ("kind".to_owned(), TelemetryValue::from("phase_transition")),
                    ("phase".to_owned(), TelemetryValue::from(phase)),
                ]),
            ));
        }
        self
    }

    pub fn build(self) -> ControllerFrame {
        self.frame
    }
}

pub fn standard_marker(
    id: &str,
    label: &str,
    view: &ControllerView<'_>,
    mut metadata: BTreeMap<String, TelemetryValue>,
) -> ControllerMarker {
    metadata
        .entry(metric::TARGET_DX_M.to_owned())
        .or_insert_with(|| TelemetryValue::from(view.target_dx_m()));
    metadata
        .entry(metric::ALTITUDE_M.to_owned())
        .or_insert_with(|| TelemetryValue::from(view.altitude_m()));
    metadata
        .entry(metric::PAD_MARGIN_M.to_owned())
        .or_insert_with(|| TelemetryValue::from(view.pad_margin_m()));
    ControllerMarker {
        id: id.to_owned(),
        label: label.to_owned(),
        x_m: Some(view.observation.position_m.x),
        y_m: Some(view.observation.position_m.y),
        metadata,
    }
}

fn dot(lhs: Vec2, rhs: Vec2) -> f64 {
    (lhs.x * rhs.x) + (lhs.y * rhs.y)
}
