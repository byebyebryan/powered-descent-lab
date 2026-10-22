use super::*;
use pd_core::{Command, RunContext, ScenarioSpec, SimulationState, Vec2};

fn context() -> RunContext {
    let scenario: ScenarioSpec = serde_json::from_str(include_str!(
        "../../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap();
    RunContext::from_scenario(&scenario).unwrap()
}

#[test]
fn polynomial_constraints_match_independent_discrete_integration() {
    let ctx = context();
    let end = LandingKinematicsV1 {
        position_m: Vec2::new(0.0, 4.995),
        velocity_mps: Vec2::new(0.0, -1.5),
    };
    for ticks in [240, 600, 1740, 3194] {
        let start = LandingKinematicsV1 {
            position_m: Vec2::new(-317.915, 197.882),
            velocity_mps: Vec2::new(62.85, -13.552),
        };
        let reference = build_reference(&ctx, start, end, ticks).unwrap();
        let mut state = start;
        let dt = ctx.sim.physics_dt_s();
        for tick in 0..ticks {
            let acceleration =
                reference.thrust(&ctx, tick) - Vec2::new(0.0, ctx.world.gravity_mps2);
            state.velocity_mps += acceleration * dt;
            state.position_m += state.velocity_mps * dt;
        }
        assert!((state.position_m - end.position_m).length() < 1.0e-8);
        assert!((state.velocity_mps - end.velocity_mps).length() < 1.0e-9);
        let closed = reference.state_at(&ctx, ticks);
        assert!((closed.position_m - state.position_m).length() < 1.0e-8);
        assert!((closed.velocity_mps - state.velocity_mps).length() < 1.0e-9);
        assert!(reference.thrust(&ctx, ticks - 1).x.abs() < 1.0e-10);
    }
}

#[test]
fn paired_inverse_matches_native_minimum_mapping_and_two_post_burn_masses() {
    let ctx = context();
    for fuel in [2000.0, 4500.0, 6300.0] {
        let mut state = SimulationState::new(&ctx).unwrap();
        state.fuel_kg = fuel;
        state.attitude_rad = 0.0;
        state.angular_rate_radps = 0.0;
        let before = state.velocity_mps.y;
        let desired = 12.0;
        let setting = paired_throttle(&ctx, 48, state.mass_kg(&ctx), desired).unwrap();
        state.set_command(Command {
            throttle_frac: setting,
            target_attitude_rad: 0.0,
        });
        for _ in 0..2 {
            let _ = state.step_physics_and_classify_contact(&ctx);
        }
        let mean = (state.velocity_mps.y - before) / (2.0 * ctx.sim.physics_dt_s())
            + ctx.world.gravity_mps2;
        assert!((mean - desired).abs() < 1.0e-10);
    }
}

#[test]
fn finite_reference_and_inverse_reject_unrealizable_inputs_without_clamping() {
    let ctx = context();
    let state = LandingKinematicsV1 {
        position_m: Vec2::new(0.0, 100.0),
        velocity_mps: Vec2::new(0.0, -10.0),
    };
    for ticks in [0, 1, 3] {
        assert!(build_reference(&ctx, state, state, ticks).is_err());
    }
    let mass = ctx.vehicle.dry_mass_kg + ctx.vehicle.initial_fuel_kg;
    assert_eq!(paired_throttle(&ctx, 48, mass, 0.0).unwrap(), 0.0);
    for thrust in [-1.0, f64::NAN, 0.001, 1.0e6] {
        assert!(paired_throttle(&ctx, 48, mass, thrust).is_err());
    }
}
