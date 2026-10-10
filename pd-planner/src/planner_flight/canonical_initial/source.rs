//! Current planner canonical_initial source; no research orchestration.
use super::*;
use anyhow::{Context, Result, bail};
use pd_core::Command;
use pd_plan::ballistic::{BridgeKindV2, VehicleInputV2, exact_discrete_bridge_v2};

pub(super) struct CanonicalSourcePrefix {
    pub live: SimulationState,
    pub updates: Vec<FlightProgramUpdateV1>,
    pub peak: f64,
    pub position_error_m: f64,
    pub velocity_error_mps: f64,
    pub iterations: usize,
}

struct Evaluation {
    prefix: CanonicalSourcePrefix,
    residual: [f64; 4],
    objective: f64,
}

fn source_plane_nonpenetrating(
    context: &RunContext,
    source: &PadInputV2,
    state: &SimulationState,
) -> bool {
    let points = actual_body_points(state, &context.vehicle.geometry);
    let within = points.iter().all(|p| {
        p.x >= source.center_x_m - source.width_m * 0.5
            && p.x <= source.center_x_m + source.width_m * 0.5
    });
    !within || points.iter().all(|p| p.y >= source.surface_y_m)
}

fn advance(context: &RunContext, source: &PadInputV2, state: &mut SimulationState) -> Result<()> {
    // Classification and terrain-dependent accumulators are intentionally not inputs.
    let _ = state.step_physics_and_classify_contact(context);
    if !source_plane_nonpenetrating(context, source, state) {
        bail!("nominal source-pad plane penetration");
    }
    if state.fuel_kg <= 0.0 || state.velocity_mps.y <= 0.0 {
        bail!("nominal source phase requires fuel and continuous ascent");
    }
    Ok(())
}

pub(super) fn build_source_prefix(
    context: &RunContext,
    source: &PadInputV2,
    vehicle: &VehicleInputV2,
    policy: &pd_plan::ballistic::DirectBridgePolicyV2,
    target: KinematicStateV2,
    ticks: u64,
) -> Result<CanonicalSourcePrefix> {
    if ticks < 2 || !ticks.is_multiple_of(2) {
        bail!("source duration lacks full held pairs");
    }
    let initial = KinematicStateV2 {
        position_m: context.initial_state.position_m,
        velocity_mps: context.initial_state.velocity_mps,
    };
    let unlaunched = exact_discrete_bridge_v2(
        policy,
        vehicle,
        BridgeKindV2::Source,
        initial,
        target,
        ticks,
    )
    .map_err(anyhow::Error::msg)?;
    let launch_target = unlaunched
        .samples
        .iter()
        .find(|s| s.thrust_acceleration_mps2.length() > 1.0e-12)
        .map(|s| {
            s.thrust_acceleration_mps2
                .x
                .atan2(s.thrust_acceleration_mps2.y)
        })
        .context("nominal source seed has no powered direction")?;
    if launch_target.abs()
        > context.vehicle.safe_touchdown_attitude_error_rad
            * (1.0 - policy.declared_robustness_margin)
    {
        bail!("nominal source seed fails source-attitude envelope");
    }
    let mut launch = SimulationState::new(context)?;
    let mut launch_updates = Vec::new();
    let mut launch_peak = launch.position_m.y;
    for tick in 0..72 {
        if launch.physics_step.is_multiple_of(2) {
            let command = Command {
                throttle_frac: 1.0,
                target_attitude_rad: if tick < 60 { 0.0 } else { launch_target },
            };
            launch_updates.push(FlightProgramUpdateV1 {
                physics_step: launch.physics_step,
                phase: if tick < 60 { "upright" } else { "tilt" }.into(),
                command,
            });
            launch.set_command(command);
        }
        advance(context, source, &mut launch)?;
        launch_peak = launch_peak.max(launch.position_m.y);
    }
    let reference = exact_discrete_bridge_v2(
        policy,
        vehicle,
        BridgeKindV2::Source,
        KinematicStateV2 {
            position_m: launch.position_m,
            velocity_mps: launch.velocity_mps,
        },
        target,
        ticks,
    )
    .map_err(anyhow::Error::msg)?;
    let seed = paired_mean_seed(&reference, launch_target)?;
    let evaluate = |correction: Correction| -> Result<Evaluation> {
        let schedule = apply_correction(&seed, correction);
        let limit = policy.thrust_derate * vehicle.max_thrust_n
            / (vehicle.dry_mass_kg + vehicle.max_fuel_kg)
            * (1.0 - policy.declared_robustness_margin);
        if schedule
            .iter()
            .any(|c| c.thrust_acceleration_mps2.length() > limit + 1.0e-12)
            || maximum_powered_slew(&schedule)
                > vehicle.max_rotation_rate_radps * (1.0 - policy.declared_robustness_margin)
                    + 1.0e-12
        {
            bail!("nominal paired source thrust or slew limit");
        }
        let first = schedule.first().context("source seed empty")?;
        if (first.target_attitude_rad - launch.attitude_rad).abs()
            > vehicle.max_rotation_rate_radps * (1.0 - policy.declared_robustness_margin) / 120.0
                + 1.0e-12
        {
            bail!("nominal launch/source one-tick attitude join");
        }
        let mut state = launch.clone();
        let mut updates = launch_updates.clone();
        let mut peak = launch_peak;
        for held in &schedule {
            let throttle = throttle_request(
                held.thrust_acceleration_mps2.length(),
                state.mass_kg(context),
                vehicle.max_thrust_n,
                vehicle.max_fuel_burn_kgps,
                1.0 / 120.0,
                vehicle.min_throttle_frac,
            )?;
            if !matches!(
                throttle.saturation,
                ThrottleSaturation::None | ThrottleSaturation::ExactMinimumOnCommand
            ) {
                bail!("nominal paired source throttle saturation");
            }
            let command = Command {
                throttle_frac: throttle.command_fraction,
                target_attitude_rad: held.target_attitude_rad,
            };
            updates.push(FlightProgramUpdateV1 {
                physics_step: state.physics_step,
                phase: "source_bridge".into(),
                command,
            });
            state.set_command(command);
            for _ in 0..2 {
                advance(context, source, &mut state)?;
                peak = peak.max(state.position_m.y);
            }
        }
        let position = state.position_m - target.position_m;
        let velocity = state.velocity_mps - target.velocity_mps;
        let duration = ticks as f64 / 120.0;
        let residual = [
            position.x,
            position.y,
            velocity.x * duration,
            velocity.y * duration,
        ];
        let objective = residual.iter().map(|x| x * x).sum::<f64>().sqrt();
        Ok(Evaluation {
            prefix: CanonicalSourcePrefix {
                live: state,
                updates,
                peak,
                position_error_m: position.length(),
                velocity_error_mps: velocity.length(),
                iterations: 0,
            },
            residual,
            objective,
        })
    };
    let mut correction = Correction::default();
    let mut current = evaluate(correction)?;
    let mut damping = 1.0e-3;
    for iteration in 0..6 {
        if current.prefix.position_error_m <= 1.0e-6 && current.prefix.velocity_error_mps <= 1.0e-6
        {
            current.prefix.iterations = iteration;
            return Ok(current.prefix);
        }
        let mut jacobian = [[0.0; 4]; 4];
        for (parameter, _) in correction.0.iter().enumerate() {
            let mut plus = correction;
            plus.0[parameter] += 1.0e-4;
            plus = clamp_correction(plus);
            let mut minus = correction;
            minus.0[parameter] -= 1.0e-4;
            minus = clamp_correction(minus);
            let a = evaluate(plus)?;
            let b = evaluate(minus)?;
            let denominator = plus.0[parameter] - minus.0[parameter];
            if denominator.abs() <= f64::EPSILON {
                bail!("nominal source finite-difference column saturated");
            }
            for (row, values) in jacobian.iter_mut().enumerate() {
                values[parameter] = (a.residual[row] - b.residual[row]) / denominator;
            }
        }
        let delta = damped_least_squares_step(jacobian, current.residual, damping)
            .context("nominal source finite-difference system singular")?;
        let mut next = None;
        for trial in 0_i32..8 {
            let scale = 0.5_f64.powi(trial);
            let candidate = clamp_correction(Correction(std::array::from_fn(|i| {
                correction.0[i] + scale * delta[i]
            })));
            if let Ok(evaluation) = evaluate(candidate)
                && evaluation.objective < current.objective
            {
                next = Some((candidate, evaluation));
                break;
            }
        }
        if let Some((candidate, evaluation)) = next {
            correction = candidate;
            current = evaluation;
            damping = (damping * 0.3).max(1.0e-9);
        } else {
            damping = (damping * 10.0).min(1.0e8);
        }
    }
    if current.prefix.position_error_m <= 1.0e-6 && current.prefix.velocity_error_mps <= 1.0e-6 {
        current.prefix.iterations = 6;
        return Ok(current.prefix);
    }
    bail!(
        "finite source fitter exhausted: position={} velocity={}",
        current.prefix.position_error_m,
        current.prefix.velocity_error_mps
    )
}
