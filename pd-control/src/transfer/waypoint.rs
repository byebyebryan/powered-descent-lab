//! Pure waypoint geometry and capture-contract prediction helpers.

use super::*;

pub(super) fn waypoint_guidance_frame(
    geometry: &WaypointLegGeometry<'_>,
    stats: WaypointLegStats,
    approach: WaypointApproachState,
) -> WaypointGuidanceFrame {
    let capture_radius_m = geometry.waypoint.capture_radius_m;
    let nominal_handoff_target_m = geometry.target_m - (geometry.leg_unit * capture_radius_m);
    WaypointGuidanceFrame {
        active_index: geometry.active_index,
        center_m: geometry.target_m,
        nominal_handoff_target_m,
        handoff_target_m: geometry.target_m,
        handoff_target_mode: "waypoint_center",
        endpoint_m: geometry.target_m,
        steering_target_m: waypoint_leg_steering_target_m(geometry, stats),
        leg_unit: geometry.leg_unit,
        handoff_tangent_unit: geometry.handoff_tangent_unit,
        envelope: WaypointGuidanceEnvelope {
            capture_radius_m: geometry.waypoint.capture_radius_m,
            max_cross_track_m: geometry.waypoint.max_cross_track_m,
            max_outbound_heading_error_rad: geometry.waypoint.max_outbound_heading_error_rad,
            min_outbound_progress_mps: geometry.waypoint.min_outbound_progress_mps,
            max_outbound_cross_speed_mps: geometry.waypoint.max_outbound_cross_speed_mps,
            min_speed_mps: geometry.waypoint.min_speed_mps,
            max_speed_mps: geometry.waypoint.max_speed_mps,
            min_vertical_speed_mps: geometry.waypoint.min_vertical_speed_mps,
            max_vertical_speed_mps: geometry.waypoint.max_vertical_speed_mps,
        },
        approach,
    }
}

pub(super) fn waypoint_reachable_event_endpoints(guidance: WaypointGuidanceFrame) -> Vec<Vec2> {
    let capture_radius_m = (guidance.envelope.capture_radius_m - 1.0e-6).max(1.0);
    let mut endpoints = Vec::new();
    for blend in [0.0, 0.5, 1.0] {
        let Some(direction) = normalized_or_none(
            (guidance.leg_unit * (1.0 - blend)) + (guidance.handoff_tangent_unit * blend),
        ) else {
            continue;
        };
        let endpoint_m = guidance.center_m - (direction * capture_radius_m);
        if !endpoints
            .iter()
            .any(|existing: &Vec2| (*existing - endpoint_m).length() < 1.0e-6)
        {
            endpoints.push(endpoint_m);
        }
    }
    endpoints
}

pub(super) fn waypoint_adjusted_observation(
    observation: &Observation,
    target_m: Vec2,
    capture_radius_m: f64,
) -> Observation {
    let mut adjusted = observation.clone();
    adjusted.target_dx_m = target_m.x - observation.position_m.x;
    adjusted.height_above_target_m = observation.position_m.y - target_m.y;
    adjusted.target_surface_y_m = target_m.y;
    adjusted.target_pad_half_width_m = capture_radius_m.max(1.0);
    adjusted
}

pub(super) fn waypoint_cubic_reference_state(
    start_position_m: Vec2,
    start_velocity_mps: Vec2,
    end_position_m: Vec2,
    end_velocity_mps: Vec2,
    time_to_go_s: f64,
    elapsed_s: f64,
) -> (Vec2, Vec2) {
    let time_to_go_s = time_to_go_s.max(1.0e-3);
    let u = (elapsed_s / time_to_go_s).clamp(0.0, 1.0);
    let u2 = u * u;
    let u3 = u2 * u;
    let h00 = (2.0 * u3) - (3.0 * u2) + 1.0;
    let h10 = u3 - (2.0 * u2) + u;
    let h01 = (-2.0 * u3) + (3.0 * u2);
    let h11 = u3 - u2;
    let dh00 = ((6.0 * u2) - (6.0 * u)) / time_to_go_s;
    let dh10 = (3.0 * u2) - (4.0 * u) + 1.0;
    let dh01 = ((-6.0 * u2) + (6.0 * u)) / time_to_go_s;
    let dh11 = (3.0 * u2) - (2.0 * u);
    let position_m = (start_position_m * h00)
        + (start_velocity_mps * (h10 * time_to_go_s))
        + (end_position_m * h01)
        + (end_velocity_mps * (h11 * time_to_go_s));
    let velocity_mps = (start_position_m * dh00)
        + (start_velocity_mps * dh10)
        + (end_position_m * dh01)
        + (end_velocity_mps * dh11);
    (position_m, velocity_mps)
}

pub(super) fn waypoint_guidance_prediction(
    observation: &Observation,
    guidance: WaypointGuidanceFrame,
    target_velocity_mps: Vec2,
    time_to_go_s: f64,
) -> WaypointGuidancePrediction {
    let stats_at = |elapsed_s| {
        let (position_m, velocity_mps) = waypoint_cubic_reference_state(
            observation.position_m,
            observation.velocity_mps,
            guidance.endpoint_m,
            target_velocity_mps,
            time_to_go_s,
            elapsed_s,
        );
        waypoint_leg_stats_from_axes(
            position_m,
            velocity_mps,
            guidance.center_m,
            guidance.leg_unit,
            guidance.handoff_tangent_unit,
        )
    };
    let prediction_at = |elapsed_s| {
        let stats = stats_at(elapsed_s);
        WaypointGuidancePrediction {
            time_to_event_s: elapsed_s,
            deadline_lead_s: (time_to_go_s - elapsed_s).max(0.0),
            stats,
            assessment: guidance.envelope.assess(stats),
        }
    };

    let mut window_open = false;
    let mut initial = prediction_at(0.0);
    window_open |= initial.assessment.capture_window_open;
    initial.assessment = initial.assessment.with_window_open(window_open);
    if initial.assessment.resolved_in_window(window_open) {
        return initial;
    }

    let scan_step_s = time_to_go_s / WAYPOINT_GUIDANCE_TRIGGER_SCAN_STEPS as f64;
    for step in 1..=WAYPOINT_GUIDANCE_TRIGGER_SCAN_STEPS {
        let upper_s = scan_step_s * step as f64;
        let window_open_before_step = window_open;
        let mut upper = prediction_at(upper_s);
        window_open |= upper.assessment.capture_window_open;
        upper.assessment = upper.assessment.with_window_open(window_open);
        if !upper.assessment.resolved_in_window(window_open) {
            continue;
        }
        let mut lower_s = upper_s - scan_step_s;
        let mut upper_s = upper_s;
        for _ in 0..WAYPOINT_GUIDANCE_TRIGGER_BISECTION_STEPS {
            let midpoint_s = (lower_s + upper_s) * 0.5;
            let midpoint = prediction_at(midpoint_s);
            let midpoint_window_open =
                window_open_before_step || midpoint.assessment.capture_window_open;
            if midpoint.assessment.resolved_in_window(midpoint_window_open) {
                upper_s = midpoint_s;
            } else {
                lower_s = midpoint_s;
            }
        }
        let mut resolved = prediction_at(upper_s);
        resolved.assessment = resolved.assessment.with_window_open(window_open);
        return resolved;
    }

    let mut endpoint = prediction_at(time_to_go_s);
    window_open |= endpoint.assessment.capture_window_open;
    endpoint.assessment = endpoint.assessment.with_window_open(window_open);
    endpoint
}

pub(super) fn waypoint_leg_stats(
    observation: &Observation,
    geometry: &WaypointLegGeometry<'_>,
) -> WaypointLegStats {
    waypoint_leg_stats_from_kinematics(observation.position_m, observation.velocity_mps, geometry)
}

pub(super) fn waypoint_leg_stats_from_kinematics(
    position_m: Vec2,
    velocity_mps: Vec2,
    geometry: &WaypointLegGeometry<'_>,
) -> WaypointLegStats {
    waypoint_leg_stats_from_axes(
        position_m,
        velocity_mps,
        geometry.target_m,
        geometry.leg_unit,
        geometry.handoff_tangent_unit,
    )
}

pub(super) fn waypoint_leg_stats_from_axes(
    position_m: Vec2,
    velocity_mps: Vec2,
    target_m: Vec2,
    leg_unit: Vec2,
    handoff_tangent_unit: Vec2,
) -> WaypointLegStats {
    let to_waypoint_m = position_m - target_m;
    let speed_mps = velocity_mps.length();
    let velocity_unit = if speed_mps > 1.0e-9 {
        velocity_mps * (1.0 / speed_mps)
    } else {
        Vec2::new(0.0, 0.0)
    };
    let heading_cos = vec_dot(velocity_unit, handoff_tangent_unit).clamp(-1.0, 1.0);
    WaypointLegStats {
        distance_m: to_waypoint_m.length(),
        cross_track_m: vec_cross(to_waypoint_m, leg_unit).abs(),
        plane_progress_m: vec_dot(to_waypoint_m, leg_unit),
        outbound_heading_error_rad: if speed_mps > 1.0e-9 {
            heading_cos.acos()
        } else {
            std::f64::consts::PI
        },
        outbound_progress_mps: vec_dot(velocity_mps, handoff_tangent_unit),
        outbound_cross_speed_mps: vec_cross(velocity_mps, handoff_tangent_unit).abs(),
        speed_mps,
        vertical_speed_mps: velocity_mps.y,
    }
}

pub(super) fn waypoint_approach_state(
    ctx: &RunContext,
    observation: &Observation,
    geometry: &WaypointLegGeometry<'_>,
    stats: WaypointLegStats,
    max_tilt_rad: f64,
) -> WaypointApproachState {
    let capture_radius_m = geometry.waypoint.capture_radius_m.max(1.0);
    let remaining_to_plane_m = (-stats.plane_progress_m).max(0.0);
    let remaining_to_handoff_m = (remaining_to_plane_m - capture_radius_m).max(0.0);
    let closing_speed_mps = vec_dot(observation.velocity_mps, geometry.leg_unit).max(0.0);
    let time_to_plane_s = if remaining_to_plane_m <= 1.0e-6 {
        0.0
    } else if closing_speed_mps > 1.0e-6 {
        remaining_to_plane_m / closing_speed_mps
    } else {
        WAYPOINT_APPROACH_TIME_TO_PLANE_MAX_S
    };
    let time_to_handoff_s = if remaining_to_handoff_m <= 1.0e-6 {
        0.0
    } else if closing_speed_mps > 1.0e-6 {
        remaining_to_handoff_m / closing_speed_mps
    } else {
        WAYPOINT_APPROACH_TIME_TO_PLANE_MAX_S
    };
    let max_lateral_accel_mps2 = waypoint_max_lateral_accel_mps2(ctx, observation, max_tilt_rad);
    let turn_delta_v_mps = 2.0 * stats.speed_mps * (0.5 * stats.outbound_heading_error_rad).sin();
    let required_turn_distance_m =
        (stats.speed_mps * turn_delta_v_mps / max_lateral_accel_mps2.max(1.0e-6)).clamp(
            capture_radius_m,
            geometry.leg_length_m.max(capture_radius_m),
        );
    let fixed_shaping_start_m = capture_radius_m * WAYPOINT_OUTBOUND_BLEND_START_CAPTURE_RADII;
    let turn_shaping_start_m =
        required_turn_distance_m + (capture_radius_m * WAYPOINT_OUTBOUND_TURN_MARGIN_CAPTURE_RADII);
    let shaping_start_distance_m = fixed_shaping_start_m.max(turn_shaping_start_m).clamp(
        capture_radius_m,
        geometry.leg_length_m.max(capture_radius_m),
    );
    let turn_margin_m = remaining_to_plane_m - required_turn_distance_m;
    let handoff_turn_margin_m = remaining_to_handoff_m - required_turn_distance_m;

    WaypointApproachState {
        remaining_to_plane_m,
        time_to_plane_s,
        remaining_to_handoff_m,
        time_to_handoff_s,
        required_turn_distance_m,
        shaping_start_distance_m,
        turn_margin_m,
        handoff_turn_margin_m,
    }
}

pub(super) fn waypoint_max_lateral_accel_mps2(
    ctx: &RunContext,
    observation: &Observation,
    max_tilt_rad: f64,
) -> f64 {
    let tilt_rad = observation
        .attitude_rad
        .abs()
        .max(TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD)
        .max(max_tilt_rad.max(0.0))
        .min(std::f64::consts::FRAC_PI_2);
    (ctx.vehicle.max_thrust_n / observation.mass_kg.max(1.0)) * tilt_rad.sin().abs().max(0.05)
}

pub(super) fn waypoint_leg_steering_target_m(
    geometry: &WaypointLegGeometry<'_>,
    stats: WaypointLegStats,
) -> Vec2 {
    let capture_radius_m = geometry.waypoint.capture_radius_m.max(1.0);
    let progress_m =
        (stats.plane_progress_m + geometry.leg_length_m).clamp(0.0, geometry.leg_length_m);
    let remaining_m = (geometry.leg_length_m - progress_m).max(0.0);
    let lookahead_m = (stats.speed_mps * WAYPOINT_LEG_LOOKAHEAD_TIME_S)
        .clamp(
            capture_radius_m * WAYPOINT_LEG_LOOKAHEAD_MIN_CAPTURE_RADII,
            capture_radius_m * WAYPOINT_LEG_LOOKAHEAD_MAX_CAPTURE_RADII,
        )
        .min(geometry.leg_length_m);
    let downrange_lookahead_m = remaining_m * WAYPOINT_LEG_REMAINING_LOOKAHEAD_FRAC;
    let target_progress_m =
        (progress_m + lookahead_m.max(downrange_lookahead_m)).min(geometry.leg_length_m);
    geometry.anchor_m + (geometry.leg_unit * target_progress_m)
}

pub(super) fn waypoint_handoff_kinematics(stats: WaypointLegStats) -> WaypointHandoffKinematics {
    WaypointHandoffKinematics {
        distance_m: stats.distance_m,
        cross_track_m: stats.cross_track_m,
        plane_progress_m: stats.plane_progress_m,
        outbound_heading_error_rad: stats.outbound_heading_error_rad,
        outbound_progress_mps: stats.outbound_progress_mps,
        outbound_cross_speed_mps: stats.outbound_cross_speed_mps,
        speed_mps: stats.speed_mps,
        vertical_speed_mps: stats.vertical_speed_mps,
    }
}

pub(super) fn normalized_or_none(vector: Vec2) -> Option<Vec2> {
    let length = vector.length();
    (length > 1.0e-9).then(|| vector * (1.0 / length))
}

pub(super) fn vec_dot(lhs: Vec2, rhs: Vec2) -> f64 {
    (lhs.x * rhs.x) + (lhs.y * rhs.y)
}

pub(super) fn vec_cross(lhs: Vec2, rhs: Vec2) -> f64 {
    (lhs.x * rhs.y) - (lhs.y * rhs.x)
}
