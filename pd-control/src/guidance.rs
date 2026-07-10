use pd_core::Command;

#[allow(clippy::too_many_arguments)]
pub(crate) fn required_control_accel(
    dx_m: f64,
    dy_m: f64,
    vx_mps: f64,
    vy_up_mps: f64,
    target_vx_mps: f64,
    target_vy_up_mps: f64,
    time_to_go_s: f64,
    gravity_mps2: f64,
) -> (f64, f64) {
    let t = time_to_go_s.max(1e-3);
    let t2 = t * t;
    let zem_x = dx_m - (vx_mps * t);
    let zem_y = dy_m - (vy_up_mps * t) + (0.5 * gravity_mps2 * t2);
    let zev_x = target_vx_mps - vx_mps;
    let zev_y = target_vy_up_mps - vy_up_mps + (gravity_mps2 * t);
    let ax = ((6.0 * zem_x) / t2) - ((2.0 * zev_x) / t);
    let ay = ((6.0 * zem_y) / t2) - ((2.0 * zev_y) / t);
    (ax, ay)
}

pub(crate) fn allocate_accel_command(
    ax_req_mps2: f64,
    ay_req_mps2: f64,
    max_thrust_accel_mps2: f64,
    max_tilt_rad: f64,
) -> Command {
    let ay = ay_req_mps2.clamp(0.0, max_thrust_accel_mps2);
    let tilt_tan = max_tilt_rad.tan();
    let ax = ax_req_mps2.clamp(-tilt_tan * ay.max(0.2), tilt_tan * ay.max(0.2));
    let thrust_accel = (ax * ax + ay * ay).sqrt();
    Command {
        throttle_frac: (thrust_accel / max_thrust_accel_mps2.max(1e-6)).clamp(0.0, 1.0),
        target_attitude_rad: ax.atan2(ay.max(0.2)).clamp(-max_tilt_rad, max_tilt_rad),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_target_accel_holds_stationary_position_against_gravity() {
        let (ax, ay) = required_control_accel(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 5.0, 9.81);

        assert!(ax.abs() < 1e-9);
        assert!((ay - 9.81).abs() < 1e-9);
    }

    #[test]
    fn state_target_accel_reaches_position_with_zero_terminal_velocity() {
        let (ax, ay) = required_control_accel(10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0);

        assert!((ax - 15.0).abs() < 1e-9);
        assert!(ay.abs() < 1e-9);
    }

    #[test]
    fn accel_allocator_respects_thrust_and_tilt_limits() {
        let max_tilt = 0.5;
        let command = allocate_accel_command(100.0, 4.0, 12.0, max_tilt);

        assert!(command.throttle_frac <= 1.0);
        assert!((command.target_attitude_rad - max_tilt).abs() < 1e-9);
    }

    #[test]
    fn accel_allocator_can_command_zero_thrust() {
        let command = allocate_accel_command(0.0, 0.0, 12.0, 0.5);

        assert_eq!(command, Command::default());
    }
}
