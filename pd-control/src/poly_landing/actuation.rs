//! Native two-step mass-aware throttle inverse; no plant stepping.
use pd_core::RunContext;

/// Invert the held throttle's mean acceleration over both post-burn masses.
/// This uses no core transition and is independently checked by physical replay.
pub fn paired_throttle(
    context: &RunContext,
    scalar_inverse_iterations: u32,
    mass: f64,
    thrust: f64,
) -> Result<f64, String> {
    if !mass.is_finite() || mass <= 0.0 || !thrust.is_finite() || thrust < 0.0 {
        return Err("invalid paired throttle input".into());
    }
    if thrust == 0.0 {
        return Ok(0.0);
    }
    let burn = context.vehicle.max_fuel_burn_kgps * context.sim.physics_dt_s();
    let max_thrust = context.vehicle.max_thrust_n;
    let mean = |q: f64| -> f64 {
        (max_thrust * q / (mass - burn * q) + max_thrust * q / (mass - 2.0 * burn * q)) * 0.5
    };
    if mass <= 2.0 * burn || thrust > mean(1.0) {
        return Err("paired throttle above maximum or unsupported remaining mass".into());
    }
    let minimum = context.vehicle.min_throttle_frac;
    if thrust < mean(minimum) {
        return Err("paired throttle below minimum".into());
    }
    let (mut low, mut high) = (minimum, 1.0);
    for _ in 0..scalar_inverse_iterations {
        let mid = (low + high) * 0.5;
        if mean(mid) < thrust {
            low = mid;
        } else {
            high = mid;
        }
    }
    let applied = (low + high) * 0.5;
    if applied <= minimum {
        Ok(f64::MIN_POSITIVE)
    } else {
        Ok((applied - minimum) / (1.0 - minimum))
    }
}
