//! Terrain-blind ballistic targeting for the opt-in feedback candidate.
//! Thrust is a finite correction to an unpowered continuation, not a terminal
//! position/zero-velocity fit. All clocks use the plant's semi-implicit ticks.

use pd_core::Vec2;
use serde::{Deserialize, Serialize};

use super::KinematicStateV2;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BallisticAim {
    pub origin: KinematicStateV2,
    pub target_m: Vec2,
    pub steps: u64,
    pub departure_velocity_mps: Vec2,
    pub arrival_velocity_mps: Vec2,
    pub apex_step: u64,
    pub apex_position_m: Vec2,
}

pub fn project(state: KinematicStateV2, gravity: f64, dt: f64, steps: u64) -> KinematicStateV2 {
    let k = steps as f64;
    KinematicStateV2 {
        position_m: state.position_m
            + state.velocity_mps * (k * dt)
            + Vec2::new(0.0, -gravity) * (dt * dt * k * (k + 1.0) * 0.5),
        velocity_mps: state.velocity_mps + Vec2::new(0.0, -gravity) * (k * dt),
    }
}

pub fn target_arc(
    state: KinematicStateV2,
    target_m: Vec2,
    gravity: f64,
    dt: f64,
    steps: u64,
) -> Option<BallisticAim> {
    if steps == 0 || !valid_inputs(state, target_m, gravity, dt) {
        return None;
    }
    let n = steps as f64;
    let velocity = Vec2::new(
        (target_m.x - state.position_m.x) / (n * dt),
        (target_m.y - state.position_m.y + gravity * dt * dt * n * (n + 1.0) * 0.5) / (n * dt),
    );
    if !velocity.x.is_finite() || !velocity.y.is_finite() {
        return None;
    }
    let departure = KinematicStateV2 {
        position_m: state.position_m,
        velocity_mps: velocity,
    };
    // The maximum is at one of the adjacent integer ticks; no full-arc scan.
    let left = (velocity.y / (gravity * dt) - 0.5).floor().clamp(0.0, n) as u64;
    let right = left.saturating_add(1).min(steps);
    let apex_step = if project(departure, gravity, dt, right).position_m.y
        > project(departure, gravity, dt, left).position_m.y
    {
        right
    } else {
        left
    };
    Some(BallisticAim {
        origin: state,
        target_m,
        steps,
        departure_velocity_mps: velocity,
        arrival_velocity_mps: project(departure, gravity, dt, steps).velocity_mps,
        apex_step,
        apex_position_m: project(departure, gravity, dt, apex_step).position_m,
    })
}

/// Positive descending target-height crossing, rounded to held-command pairs.
/// A descending state may have its apex in the past; no new climb is required.
pub fn natural_arrival_steps(
    state: KinematicStateV2,
    target_y: f64,
    gravity: f64,
    dt: f64,
) -> Option<u64> {
    if !valid_inputs(state, Vec2::new(state.position_m.x, target_y), gravity, dt) {
        return None;
    }
    let b = state.velocity_mps.y - gravity * dt * 0.5;
    let discriminant = b * b - 2.0 * gravity * (target_y - state.position_m.y);
    if discriminant < 0.0 {
        return None;
    }
    let t = (b + discriminant.sqrt()) / gravity;
    let pairs = (t / (2.0 * dt)).round();
    (pairs.is_finite() && pairs >= 1.0 && pairs < (u64::MAX / 2) as f64).then(|| pairs as u64 * 2)
}

/// Thrust acceleration during a coast-turn, constant-acceleration burn and
/// coast. This is the same finite-burn gain used by current acquisition math.
pub fn correction_acceleration(
    state: KinematicStateV2,
    target_m: Vec2,
    gravity: f64,
    dt: f64,
    arrival_steps: u64,
    turn_steps: u64,
    burn_steps: u64,
) -> Option<Vec2> {
    if !valid_inputs(state, target_m, gravity, dt)
        || burn_steps == 0
        || turn_steps.checked_add(burn_steps)? >= arrival_steps
    {
        return None;
    }
    let n = arrival_steps as f64;
    let burn = burn_steps as f64;
    let gain = dt * dt * burn * (n - turn_steps as f64 - (burn - 1.0) * 0.5);
    if !gain.is_finite() || gain <= 0.0 {
        return None;
    }
    let accel = (target_m - project(state, gravity, dt, arrival_steps).position_m) * (1.0 / gain);
    (accel.x.is_finite() && accel.y.is_finite()).then_some(accel)
}

pub fn correction_end(
    state: KinematicStateV2,
    gravity: f64,
    dt: f64,
    turn_steps: u64,
    burn_steps: u64,
    thrust: Vec2,
) -> KinematicStateV2 {
    let mut end = project(state, gravity, dt, turn_steps + burn_steps);
    let burn = burn_steps as f64;
    end.position_m += thrust * (dt * dt * burn * (burn + 1.0) * 0.5);
    end.velocity_mps += thrust * (burn * dt);
    end
}

fn valid_inputs(state: KinematicStateV2, target: Vec2, gravity: f64, dt: f64) -> bool {
    [
        state.position_m.x,
        state.position_m.y,
        state.velocity_mps.x,
        state.velocity_mps.y,
        target.x,
        target.y,
        gravity,
        dt,
    ]
    .iter()
    .all(|v| v.is_finite())
        && gravity > 0.0
        && dt > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(p: Vec2, v: Vec2) -> KinematicStateV2 {
        KinematicStateV2 {
            position_m: p,
            velocity_mps: v,
        }
    }

    #[test]
    fn valid_arc_needs_no_correction_after_clock_advances() {
        let start = state(Vec2::new(0.0, 100.0), Vec2::default());
        let goal = Vec2::new(300.0, 0.0);
        let arc = target_arc(start, goal, 9.81, 1.0 / 120.0, 1200).unwrap();
        let motion = state(start.position_m, arc.departure_velocity_mps);
        for elapsed in [0, 120, 600, 1000] {
            let current = project(motion, 9.81, 1.0 / 120.0, elapsed);
            let aim = target_arc(current, goal, 9.81, 1.0 / 120.0, 1200 - elapsed).unwrap();
            assert!((aim.departure_velocity_mps - current.velocity_mps).length() < 1e-10);
            let correction =
                correction_acceleration(current, goal, 9.81, 1.0 / 120.0, 1200 - elapsed, 0, 24)
                    .unwrap();
            assert!(correction.length() < 1e-9);
        }
    }

    #[test]
    fn finite_burn_then_coast_reaches_endpoint() {
        let start = state(Vec2::new(20.0, 100.0), Vec2::new(12.0, 25.0));
        let goal = Vec2::new(500.0, 0.0);
        let thrust =
            correction_acceleration(start, goal, 9.81, 1.0 / 120.0, 1400, 120, 300).unwrap();
        let end = correction_end(start, 9.81, 1.0 / 120.0, 120, 300, thrust);
        let arrived = project(end, 9.81, 1.0 / 120.0, 980);
        assert!((arrived.position_m - goal).length() < 1e-9);
    }

    #[test]
    fn descending_motion_does_not_require_a_future_apex() {
        let start = state(Vec2::new(0.0, 100.0), Vec2::new(30.0, -20.0));
        let n = natural_arrival_steps(start, 0.0, 9.81, 1.0 / 120.0).unwrap();
        let target = project(start, 9.81, 1.0 / 120.0, n).position_m;
        let arc = target_arc(start, target, 9.81, 1.0 / 120.0, n).unwrap();
        assert_eq!(arc.apex_step, 0);
        assert!((arc.departure_velocity_mps - start.velocity_mps).length() < 1e-10);
    }

    #[test]
    fn invalid_and_uphill_no_crossing_inputs_return_none() {
        let s = state(Vec2::default(), Vec2::default());
        assert!(natural_arrival_steps(s, 100.0, 9.81, 1.0 / 120.0).is_none());
        assert!(target_arc(s, Vec2::default(), 9.81, 0.0, 10).is_none());
        assert!(
            correction_acceleration(s, Vec2::default(), 9.81, 1.0 / 120.0, 20, 10, 10).is_none()
        );
    }
}
