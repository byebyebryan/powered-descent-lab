//! Input-only arc construction. No native transition or outcome query.
use pd_control::poly_landing::{LandingKinematicsV1, build_reference, paired_mean};
use pd_core::{RunContext, ScenarioSpec, TerrainDefinition, Vec2};
use serde_json::{Value, json};

pub fn proposal(
    anchor: &ScenarioSpec,
    seed: u64,
    cohort: &str,
    index: u64,
) -> Result<Value, String> {
    let u: Vec<f64> = (0..7)
        .map(|axis| {
            let hash = pd_missions::sha256(
                format!("coast-arc-v1:{seed}:{cohort}:{index}:{axis}").as_bytes(),
            );
            (u64::from_str_radix(&hash[..16], 16).unwrap() >> 11) as f64 / (1_u64 << 53) as f64
        })
        .collect();
    let n = 2 * ((8.0 + 16.0 * u[0]) * 60.0).floor() as u64;
    let t = n as f64 / 120.0;
    let vx = (0.5 + 2.5 * u[1]) * t;
    let sink = 8.0 + 47.0 * u[2];
    let b = ((0.28 + 0.14 * u[3]) * n as f64).round() as u64;
    let c = 12 * ((0.5 + 5.5 * u[4]) * 10.0).round() as u64;
    let dt = 1.0 / 120.0;
    let witness_y = 5.0 + sink * b as f64 * dt + 9.81 * dt * dt * (b * (b + 1)) as f64 / 2.0;
    let q = (((witness_y - 4.995) / t) - 1.5) / (sink - 1.5);
    let mut s = anchor.clone();
    s.id = format!("coast-{cohort}-{index}");
    s.name = s.id.clone();
    s.description =
        "Supplied target-directed ballistic arc; controller-owned coast and fixed timed landing"
            .into();
    s.seed = seed.wrapping_add(index);
    s.tags = vec!["coast_ignite".into(), "flat".into(), "prepared_arc".into()];
    s.metadata.clear();
    s.sim.max_time_s = 40.0;
    s.sim.sample_hz = Some(120);
    s.mission.transfer_route = None;
    let mut pad = s
        .world
        .landing_pads
        .iter()
        .find(|p| p.id == s.mission.goal.target_pad_id())
        .ok_or("missing pad")?
        .clone();
    pad.center_x_m = 0.0;
    pad.surface_y_m = 0.0;
    pad.width_m = 36.0;
    s.world.landing_pads = vec![pad];
    s.vehicle.initial_fuel_kg = 1800.0 + 4500.0 * u[5];
    let vy0 = -sink + 9.81 * c as f64 * dt;
    let cb = (c + b) as f64;
    s.initial_state.position_m = Vec2::new(
        -vx * cb * dt,
        5.0 - vy0 * cb * dt + 9.81 * dt * dt * cb * (cb + 1.0) / 2.0,
    );
    s.initial_state.velocity_mps = Vec2::new(vx, vy0);
    s.initial_state.angular_rate_radps = 0.0;
    let mut witness = s.clone();
    witness.initial_state.position_m = Vec2::new(-vx * b as f64 * dt, witness_y);
    witness.initial_state.velocity_mps = Vec2::new(vx, -sink);
    let ctx = RunContext::from_scenario(&witness)?;
    let reference = build_reference(
        &ctx,
        LandingKinematicsV1 {
            position_m: witness.initial_state.position_m,
            velocity_mps: witness.initial_state.velocity_mps,
        },
        LandingKinematicsV1 {
            position_m: Vec2::new(0.0, 4.995),
            velocity_mps: Vec2::new(0.0, -1.5),
        },
        n,
    )?;
    let first = paired_mean(&reference, &ctx, 0);
    s.initial_state.attitude_rad = first.x.atan2(first.y);
    let mut queries = 0;
    let admission = (|| {
        if !(0.4..=0.6).contains(&q) || !(0.28..=0.42).contains(&(b as f64 / n as f64)) {
            return Err("vertical mixture or crossing fraction".to_string());
        }
        let remaining =
            s.vehicle.initial_fuel_kg - s.vehicle.max_fuel_burn_kgps * (n + 4) as f64 / 120.0;
        if remaining <= 0.0 {
            return Err("conservative fuel bound".into());
        }
        let lower = s.vehicle.max_thrust_n * s.vehicle.min_throttle_frac
            / (s.vehicle.dry_mass_kg + remaining);
        let upper =
            0.925 * s.vehicle.max_thrust_n / (s.vehicle.dry_mass_kg + s.vehicle.initial_fuel_kg);
        let mut previous = s.initial_state.attitude_rad;
        for tick in 0..n + 4 {
            queries += 1;
            let state = reference.state_at(&ctx, tick);
            if state.velocity_mps.x < -1e-9 || state.velocity_mps.y > 1e-9 {
                return Err("reference velocity shape".into());
            }
            if tick.is_multiple_of(2) {
                let thrust = paired_mean(&reference, &ctx, tick);
                let angle = thrust.x.atan2(thrust.y);
                if thrust.y <= 0.0 || thrust.length() < lower || thrust.length() > upper {
                    return Err("conservative coupled authority".into());
                }
                if (angle - previous).abs() > s.vehicle.max_rotation_rate_radps / 120.0 {
                    return Err("one-tick servo bound".into());
                }
                previous = angle;
            }
        }
        let a = s.initial_state.attitude_rad;
        let body = 5.0 * a.cos().abs() + 4.0 * a.sin().abs();
        for tick in 0..=c {
            queries += 1;
            let k = tick as f64;
            let y =
                s.initial_state.position_m.y + vy0 * k * dt - 9.81 * dt * dt * k * (k + 1.0) / 2.0;
            if y <= body {
                return Err("coast body clearance".into());
            }
        }
        Ok(())
    })();
    let direction = if u[6] < 0.5 { 1.0 } else { -1.0 };
    s.initial_state.position_m.x *= direction;
    s.initial_state.velocity_mps.x *= direction;
    s.initial_state.attitude_rad *= direction;
    s.world.terrain = TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-2500.0, 0.0), Vec2::new(2500.0, 0.0)],
    };
    Ok(
        json!({"scenario":s,"reference_end":c+n,"admitted":admission.is_ok(),"exclusion":admission.err(),"physical_sha256":pd_missions::physical_identity(&s).map_err(|e|e.to_string())?,"audit":{"proposal_index":index,"nominal_coast_ticks":c,"nominal_burn_ticks":n,"ballistic_ticks":b,"vertical_mixture":q,"pure_grid_queries":queries,"native_forecast_steps":0}}),
    )
}
