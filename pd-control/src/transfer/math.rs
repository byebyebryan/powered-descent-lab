use super::*;

pub(super) fn transfer_ballistic_projection(
    dx_m: f64,
    dy_m: f64,
    vx_mps: f64,
    vy_up_mps: f64,
    gravity_mps2: f64,
) -> TransferBallisticProjection {
    let gravity_mps2 = gravity_mps2.max(1.0e-6);
    let discriminant = (vy_up_mps * vy_up_mps) - (2.0 * gravity_mps2 * dy_m);
    let apex_over_target_m = if vy_up_mps > 0.0 {
        -dy_m + ((vy_up_mps * vy_up_mps) / (2.0 * gravity_mps2))
    } else {
        -dy_m
    };

    if discriminant < 0.0 {
        return TransferBallisticProjection {
            has_target_y_solution: false,
            projected_time_s: None,
            projected_dx_m: None,
            impact_angle_deg: None,
            apex_over_target_m,
        };
    }

    let sqrt_discriminant = discriminant.sqrt();
    let t0 = (vy_up_mps - sqrt_discriminant) / gravity_mps2;
    let t1 = (vy_up_mps + sqrt_discriminant) / gravity_mps2;
    let projected_time_s = [t0, t1]
        .into_iter()
        .filter(|time_s| *time_s >= 0.0)
        .max_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap())
        .unwrap_or(0.0);
    let projected_dx_m = dx_m - (vx_mps * projected_time_s);
    let impact_vy_up_mps = vy_up_mps - (gravity_mps2 * projected_time_s);
    let impact_down_speed_mps = (-impact_vy_up_mps).max(0.0);
    let impact_angle_deg = impact_down_speed_mps.atan2(vx_mps.abs()).to_degrees();

    TransferBallisticProjection {
        has_target_y_solution: true,
        projected_time_s: Some(projected_time_s),
        projected_dx_m: Some(projected_dx_m),
        impact_angle_deg: Some(impact_angle_deg),
        apex_over_target_m,
    }
}

pub(super) fn applied_throttle_frac(
    ctx: &RunContext,
    commanded_throttle_frac: f64,
    fuel_kg: f64,
) -> f64 {
    if fuel_kg <= 0.0 {
        return 0.0;
    }
    let commanded = commanded_throttle_frac.clamp(0.0, 1.0);
    if commanded <= 0.0 {
        return 0.0;
    }
    let min_throttle = ctx.vehicle.min_throttle_frac.clamp(0.0, 1.0);
    min_throttle + (commanded * (1.0 - min_throttle))
}

pub(super) fn shortest_angle_delta(from_rad: f64, to_rad: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    (to_rad - from_rad + std::f64::consts::PI).rem_euclid(tau) - std::f64::consts::PI
}
