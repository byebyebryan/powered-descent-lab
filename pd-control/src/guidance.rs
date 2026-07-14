use pd_core::{Command, Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StateTargetRequest {
    pub position_error_m: Vec2,
    pub velocity_mps: Vec2,
    pub target_velocity_mps: Vec2,
    pub time_to_go_s: f64,
    pub gravity_mps2: f64,
}

pub(crate) fn required_control_accel(request: StateTargetRequest) -> Vec2 {
    let t = request.time_to_go_s.max(1e-3);
    let t2 = t * t;
    let zem_x = request.position_error_m.x - (request.velocity_mps.x * t);
    let zem_y = request.position_error_m.y - (request.velocity_mps.y * t)
        + (0.5 * request.gravity_mps2 * t2);
    let zev_x = request.target_velocity_mps.x - request.velocity_mps.x;
    let zev_y = request.target_velocity_mps.y - request.velocity_mps.y
        + (request.gravity_mps2 * t);
    let ax = ((6.0 * zem_x) / t2) - ((2.0 * zev_x) / t);
    let ay = ((6.0 * zem_y) / t2) - ((2.0 * zev_y) / t);
    Vec2::new(ax, ay)
}

pub(crate) fn allocate_accel_command(
    requested_accel_mps2: Vec2,
    max_thrust_accel_mps2: f64,
    max_tilt_rad: f64,
) -> Command {
    let ay = requested_accel_mps2
        .y
        .clamp(0.0, max_thrust_accel_mps2);
    let tilt_tan = max_tilt_rad.tan();
    let ax = requested_accel_mps2
        .x
        .clamp(-tilt_tan * ay.max(0.2), tilt_tan * ay.max(0.2));
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
        let accel = required_control_accel(StateTargetRequest {
            position_error_m: Vec2::default(),
            velocity_mps: Vec2::default(),
            target_velocity_mps: Vec2::default(),
            time_to_go_s: 5.0,
            gravity_mps2: 9.81,
        });

        assert!(accel.x.abs() < 1e-9);
        assert!((accel.y - 9.81).abs() < 1e-9);
    }

    #[test]
    fn state_target_accel_reaches_position_with_zero_terminal_velocity() {
        let accel = required_control_accel(StateTargetRequest {
            position_error_m: Vec2::new(10.0, 0.0),
            velocity_mps: Vec2::default(),
            target_velocity_mps: Vec2::default(),
            time_to_go_s: 2.0,
            gravity_mps2: 0.0,
        });

        assert!((accel.x - 15.0).abs() < 1e-9);
        assert!(accel.y.abs() < 1e-9);
    }

    #[test]
    fn accel_allocator_respects_thrust_and_tilt_limits() {
        let max_tilt = 0.5;
        let command = allocate_accel_command(Vec2::new(100.0, 4.0), 12.0, max_tilt);

        assert!(command.throttle_frac <= 1.0);
        assert!((command.target_attitude_rad - max_tilt).abs() < 1e-9);
    }

    #[test]
    fn accel_allocator_can_command_zero_thrust() {
        let command = allocate_accel_command(Vec2::default(), 12.0, 0.5);

        assert_eq!(command, Command::default());
    }
}
