use crate::guidance::{StateTargetRequest, required_control_accel};
use crate::kit::ControllerView;

use super::{
    TERRAIN_CLEARANCE_MARGIN_M, TERRAIN_CLEARANCE_MAX_HORIZONTAL_STEP_M,
    TERRAIN_CLEARANCE_MAX_SAMPLE_COUNT, TERRAIN_CLEARANCE_MAX_TIME_STEP_S,
    TERRAIN_CLEARANCE_MIN_SAMPLE_COUNT, TERRAIN_CLEARANCE_UNCONSTRAINED_M,
    TERRAIN_OBSTACLE_RELIEF_FLOOR_M, TerrainClearanceEstimate,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct TerrainClearanceRequest {
    pub(super) position_error_m: pd_core::Vec2,
    pub(super) velocity_mps: pd_core::Vec2,
    pub(super) burn_time_s: f64,
    pub(super) target_vertical_speed_mps: f64,
    pub(super) target_attitude_rad: f64,
    pub(super) gravity_mps2: f64,
}

pub(super) fn estimate_candidate_terrain_clearance(
    view: &ControllerView<'_>,
    request: TerrainClearanceRequest,
) -> TerrainClearanceEstimate {
    let touchdown_center_error_m = pd_core::Vec2::new(
        request.position_error_m.x,
        request.position_error_m.y + view.ctx.vehicle.geometry.touchdown_base_offset_m,
    );
    let requested_accel_mps2 = required_control_accel(StateTargetRequest {
        position_error_m: touchdown_center_error_m,
        velocity_mps: request.velocity_mps,
        target_velocity_mps: pd_core::Vec2::new(0.0, request.target_vertical_speed_mps),
        time_to_go_s: request.burn_time_s,
        gravity_mps2: request.gravity_mps2,
    });
    let mut min_clearance_m = TERRAIN_CLEARANCE_UNCONSTRAINED_M;
    let mut first_violation_time_s = None;
    let sample_count = terrain_clearance_sample_count(
        request.velocity_mps.x,
        requested_accel_mps2.x,
        request.burn_time_s,
    );
    for sample_index in 0..=sample_count {
        let ratio = sample_index as f64 / sample_count as f64;
        let t = request.burn_time_s.max(0.0) * ratio;
        let t2 = t * t;
        let center_x_m = view.observation.position_m.x
            + (request.velocity_mps.x * t)
            + (0.5 * requested_accel_mps2.x * t2);
        let center_y_m = view.observation.position_m.y
            + (request.velocity_mps.y * t)
            + (0.5 * (requested_accel_mps2.y - request.gravity_mps2) * t2);
        let Some(clearance_m) =
            planned_hull_clearance_m(view, center_x_m, center_y_m, request.target_attitude_rad)
        else {
            continue;
        };
        min_clearance_m = min_clearance_m.min(clearance_m);
        if first_violation_time_s.is_none() && clearance_m < TERRAIN_CLEARANCE_MARGIN_M {
            first_violation_time_s = Some(t);
        }
    }

    TerrainClearanceEstimate {
        min_clearance_m,
        first_violation_time_s,
        safe: first_violation_time_s.is_none(),
    }
}

fn terrain_clearance_sample_count(vx_mps: f64, ax_mps2: f64, burn_time_s: f64) -> usize {
    let burn_time_s = burn_time_s.max(0.0);
    let time_samples = (burn_time_s / TERRAIN_CLEARANCE_MAX_TIME_STEP_S).ceil() as usize;
    let peak_horizontal_speed_mps = vx_mps.abs().max((vx_mps + (ax_mps2 * burn_time_s)).abs());
    let horizontal_samples = ((peak_horizontal_speed_mps * burn_time_s)
        / TERRAIN_CLEARANCE_MAX_HORIZONTAL_STEP_M)
        .ceil() as usize;
    time_samples.max(horizontal_samples).clamp(
        TERRAIN_CLEARANCE_MIN_SAMPLE_COUNT,
        TERRAIN_CLEARANCE_MAX_SAMPLE_COUNT,
    )
}

fn planned_hull_clearance_m(
    view: &ControllerView<'_>,
    center_x_m: f64,
    center_y_m: f64,
    attitude_rad: f64,
) -> Option<f64> {
    let geometry = &view.ctx.vehicle.geometry;
    let half_width_m = geometry.hull_width_m * 0.5;
    let half_height_m = geometry.hull_height_m * 0.5;
    let cos_a = attitude_rad.cos();
    let sin_a = attitude_rad.sin();
    let mut min_clearance_m = f64::INFINITY;
    let mut sampled_any = false;

    for (local_x_m, local_y_m) in [
        (-half_width_m, -half_height_m),
        (half_width_m, -half_height_m),
        (half_width_m, half_height_m),
        (-half_width_m, half_height_m),
    ] {
        let point_x_m = center_x_m + (local_x_m * cos_a) - (local_y_m * sin_a);
        let point_y_m = center_y_m + (local_x_m * sin_a) + (local_y_m * cos_a);
        let terrain_y_m = view.terrain_height_at(point_x_m);
        if terrain_y_m <= view.ctx.target_pad.surface_y_m + TERRAIN_OBSTACLE_RELIEF_FLOOR_M {
            continue;
        }
        min_clearance_m = min_clearance_m.min(point_y_m - terrain_y_m);
        sampled_any = true;
    }

    sampled_any.then_some(min_clearance_m)
}
