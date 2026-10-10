//! Current planner math; no research orchestration.
use anyhow::{Result, bail};
use pd_core::VehicleSpec;
use pd_plan::ballistic::{VehicleGeometryInputV2, VehicleInputV2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ThrottleSaturation {
    None,
    BelowMinimum,
    AboveMaximum,
    ExactMinimumOnCommand,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ThrottleRequest {
    pub(super) command_fraction: f64,
    pub(super) saturation: ThrottleSaturation,
}

pub(super) fn throttle_request(
    thrust_acceleration_mps2: f64,
    pre_burn_mass_kg: f64,
    max_thrust_n: f64,
    max_fuel_burn_kgps: f64,
    dt_s: f64,
    min_throttle_frac: f64,
) -> Result<ThrottleRequest> {
    if thrust_acceleration_mps2 <= 0.0 {
        return Ok(ThrottleRequest {
            command_fraction: 0.0,
            saturation: ThrottleSaturation::None,
        });
    }
    let denominator = max_thrust_n + thrust_acceleration_mps2 * max_fuel_burn_kgps * dt_s;
    let requested = thrust_acceleration_mps2 * pre_burn_mass_kg / denominator;
    if !requested.is_finite() || !pre_burn_mass_kg.is_finite() || pre_burn_mass_kg <= 0.0 {
        bail!("throttle inversion received non-finite or non-positive plant state");
    }
    let (command_fraction, saturation) = if requested == min_throttle_frac {
        (f64::MIN_POSITIVE, ThrottleSaturation::ExactMinimumOnCommand)
    } else if requested < min_throttle_frac {
        (f64::MIN_POSITIVE, ThrottleSaturation::BelowMinimum)
    } else if requested > 1.0 {
        (1.0, ThrottleSaturation::AboveMaximum)
    } else if min_throttle_frac >= 1.0 {
        (f64::MIN_POSITIVE, ThrottleSaturation::ExactMinimumOnCommand)
    } else {
        (
            (requested - min_throttle_frac) / (1.0 - min_throttle_frac),
            ThrottleSaturation::None,
        )
    };
    Ok(ThrottleRequest {
        command_fraction,
        saturation,
    })
}

pub(super) fn shortest_angle_delta(current_rad: f64, target_rad: f64) -> f64 {
    let turn = std::f64::consts::TAU;
    (target_rad - current_rad + std::f64::consts::PI).rem_euclid(turn) - std::f64::consts::PI
}

pub(super) fn vehicle_input_v2(vehicle: &VehicleSpec) -> VehicleInputV2 {
    VehicleInputV2 {
        geometry: VehicleGeometryInputV2 {
            hull_width_m: vehicle.geometry.hull_width_m,
            hull_height_m: vehicle.geometry.hull_height_m,
            touchdown_half_span_m: vehicle.geometry.touchdown_half_span_m,
            touchdown_base_offset_m: vehicle.geometry.touchdown_base_offset_m,
        },
        dry_mass_kg: vehicle.dry_mass_kg,
        initial_fuel_kg: vehicle.initial_fuel_kg,
        max_fuel_kg: vehicle.max_fuel_kg,
        max_fuel_burn_kgps: vehicle.max_fuel_burn_kgps,
        max_thrust_n: vehicle.max_thrust_n,
        min_throttle_frac: vehicle.min_throttle_frac,
        max_rotation_rate_radps: vehicle.max_rotation_rate_radps,
        safe_touchdown_normal_speed_mps: vehicle.safe_touchdown_normal_speed_mps,
        safe_touchdown_tangential_speed_mps: vehicle.safe_touchdown_tangential_speed_mps,
        safe_touchdown_attitude_error_rad: vehicle.safe_touchdown_attitude_error_rad,
        safe_touchdown_angular_rate_radps: vehicle.safe_touchdown_angular_rate_radps,
    }
}
