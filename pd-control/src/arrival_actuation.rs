//! Opt-in arrival allocation. The legacy directional/authority projection stays
//! fixed; only force acceleration to native held engine setting is calibrated.
use crate::{guidance::allocate_accel_command, poly_landing::paired_throttle};
use pd_core::{Command, Observation, RunContext, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArrivalActuationProfile {
    #[default]
    LegacyV1,
    NativePairV2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllocationRegime {
    Legacy,
    Off,
    Tracking,
    GapOff,
    GapMinimum,
    FuelLimited,
    NoFuel,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ArrivalAllocation {
    pub command: Command,
    pub regime: AllocationRegime,
    pub requested_accel_mps2: Vec2,
    pub projected_accel_mps2: Vec2,
    pub projected_magnitude_mps2: f64,
    /// Scalar mean assuming alignment throughout the pair; rotation is separate.
    pub aligned_mean_accel_mps2: f64,
    pub aligned_error_mps2: f64,
    pub geometry_limited: bool,
    pub fuel_limited: bool,
}

pub fn allocate_arrival_acceleration(
    ctx: &RunContext,
    o: &Observation,
    requested: Vec2,
    max_tilt: f64,
    profile: ArrivalActuationProfile,
) -> Result<ArrivalAllocation, String> {
    if ctx.sim.physics_hz != 120
        || ctx.sim.controller_hz != 60
        || [requested.x, requested.y, o.mass_kg, o.fuel_kg, max_tilt]
            .iter()
            .any(|v| !v.is_finite())
        || o.fuel_kg < 0.0
        || o.mass_kg != ctx.vehicle.dry_mass_kg + o.fuel_kg
        || !(0.0..std::f64::consts::FRAC_PI_2).contains(&max_tilt)
    {
        return Err("invalid arrival allocation state, cadence or projection".into());
    }
    let maximum = ctx.vehicle.max_thrust_n / o.mass_kg;
    let legacy = allocate_accel_command(requested, maximum, max_tilt);
    let desired = legacy.throttle_frac * maximum;
    let direction = Vec2::new(
        legacy.target_attitude_rad.sin(),
        legacy.target_attitude_rad.cos(),
    );
    let projected = direction * desired;
    let mut command = legacy;
    let mut regime = AllocationRegime::Legacy;
    let mut fuel_limited = false;
    if profile == ArrivalActuationProfile::NativePairV2 {
        let burn = ctx.vehicle.max_fuel_burn_kgps * ctx.sim.physics_dt_s();
        let qmax = if burn > 0.0 {
            (o.fuel_kg / (2.0 * burn)).min(1.0)
        } else {
            1.0
        };
        let minimum = ctx.vehicle.min_throttle_frac;
        let mean = |q: f64| {
            (ctx.vehicle.max_thrust_n * q / (o.mass_kg - burn * q)
                + ctx.vehicle.max_thrust_n * q / (o.mass_kg - 2.0 * burn * q))
                * 0.5
        };
        let target;
        if desired == 0.0 {
            regime = AllocationRegime::Off;
            target = 0.0;
        } else if o.fuel_kg == 0.0 {
            regime = AllocationRegime::NoFuel;
            target = 0.0;
            fuel_limited = true;
        } else if qmax < minimum {
            // Conservative complete-pair policy, not a physical impossibility
            // claim: the plant can still thrust during a partially fueled pair.
            regime = AllocationRegime::FuelLimited;
            target = 0.0;
            fuel_limited = true;
        } else if desired < mean(minimum) {
            if desired <= 0.5 * mean(minimum) {
                regime = AllocationRegime::GapOff;
                target = 0.0;
            } else {
                regime = AllocationRegime::GapMinimum;
                target = mean(minimum);
            }
        } else {
            target = desired.min(mean(qmax));
            fuel_limited = target < desired;
            regime = if fuel_limited {
                AllocationRegime::FuelLimited
            } else {
                AllocationRegime::Tracking
            };
        }
        command.throttle_frac = if regime == AllocationRegime::GapMinimum {
            f64::MIN_POSITIVE
        } else {
            paired_throttle(ctx, 48, o.mass_kg, target)?
        };
        if fuel_limited && command.throttle_frac > 0.0 && minimum < 1.0 {
            command.throttle_frac = command
                .throttle_frac
                .min(((qmax - minimum) / (1.0 - minimum)).max(f64::MIN_POSITIVE));
        }
    }
    let aligned = aligned_native_pair_mean(ctx, o, command.throttle_frac);
    Ok(ArrivalAllocation {
        command,
        regime,
        requested_accel_mps2: requested,
        projected_accel_mps2: projected,
        projected_magnitude_mps2: desired,
        aligned_mean_accel_mps2: aligned,
        aligned_error_mps2: aligned - desired,
        geometry_limited: (projected - requested).length() > 1e-9,
        fuel_limited,
    })
}

/// Forward calculation deliberately includes the native fuel-exhaustion rule.
fn aligned_native_pair_mean(ctx: &RunContext, o: &Observation, setting: f64) -> f64 {
    let mut fuel = o.fuel_kg;
    let mut sum = 0.0;
    for _ in 0..2 {
        let q = if fuel <= 0.0 || setting <= 0.0 {
            0.0
        } else {
            ctx.vehicle.min_throttle_frac
                + setting.clamp(0.0, 1.0) * (1.0 - ctx.vehicle.min_throttle_frac)
        };
        fuel -= (ctx.vehicle.max_fuel_burn_kgps * q * ctx.sim.physics_dt_s()).min(fuel);
        sum += ctx.vehicle.max_thrust_n * q / (ctx.vehicle.dry_mass_kg + fuel);
    }
    sum * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::{ScenarioSpec, SimulationState};
    fn fixture() -> (RunContext, SimulationState) {
        let mut s: ScenarioSpec = serde_json::from_str(include_str!(
            "../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .unwrap();
        s.initial_state.position_m = Vec2::new(0.0, 600.0);
        let c = RunContext::from_scenario(&s).unwrap();
        let st = SimulationState::new(&c).unwrap();
        (c, st)
    }
    #[test]
    fn calibrated_pairs_match_native_mass_minimum_and_fuel_boundaries() {
        let (ctx, initial) = fixture();
        for fuel in [0.0, 0.05, 0.20625, 0.4, 2000.0, 4500.0, 6300.0] {
            for demand in [0.0, 0.1, 2.0, 4.0, 6.0, 10.0, 12.0, 17.0, 30.0] {
                let mut state = initial.clone();
                state.fuel_kg = fuel;
                let o = state.build_observation(&ctx);
                let allocation = allocate_arrival_acceleration(
                    &ctx,
                    &o,
                    Vec2::new(0.0, demand),
                    0.72,
                    ArrivalActuationProfile::NativePairV2,
                )
                .unwrap();
                state.set_command(allocation.command);
                for _ in 0..2 {
                    let _ = state.step_physics_and_classify_contact(&ctx);
                }
                let native = (state.velocity_mps.y - o.velocity_mps.y) * 60.0 + 9.81;
                assert!(
                    (native - allocation.aligned_mean_accel_mps2).abs() < 1e-8,
                    "fuel={fuel} demand={demand}"
                );
                if allocation.regime == AllocationRegime::Tracking {
                    assert!((native - allocation.projected_magnitude_mps2).abs() < 1e-8);
                }
                if allocation.regime == AllocationRegime::GapOff {
                    assert_eq!(allocation.command.throttle_frac, 0.0);
                }
            }
        }
    }
    #[test]
    fn legacy_commands_are_exact_and_rotation_is_not_instantaneous() {
        let (ctx, mut state) = fixture();
        let o = state.build_observation(&ctx);
        let requested = Vec2::new(-6.0, 10.0);
        let legacy = allocate_arrival_acceleration(
            &ctx,
            &o,
            requested,
            0.72,
            ArrivalActuationProfile::LegacyV1,
        )
        .unwrap();
        assert_eq!(
            legacy.command,
            allocate_accel_command(requested, ctx.vehicle.max_thrust_n / o.mass_kg, 0.72)
        );
        let mapped = allocate_arrival_acceleration(
            &ctx,
            &o,
            requested,
            0.72,
            ArrivalActuationProfile::NativePairV2,
        )
        .unwrap();
        state.set_command(mapped.command);
        for _ in 0..2 {
            let _ = state.step_physics_and_classify_contact(&ctx);
        }
        assert!(state.attitude_rad.abs() <= ctx.vehicle.max_rotation_rate_radps / 60.0 + 1e-12);
        assert!(state.attitude_rad.abs() < mapped.command.target_attitude_rad.abs());
    }
}
