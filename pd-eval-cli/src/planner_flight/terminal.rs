//! Current planner terminal; no research orchestration.
use super::*;
use anyhow::{Context, Result, bail};
use pd_core::{RunContext, SimulationState, Vec2};
use pd_plan::ballistic::{KinematicStateV2, PadInputV2};
use serde::{Deserialize, Serialize};

const POLICY_VERSION: &str = "body_aware_quadratic_terminal_held60_v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyAwareTerminalPolicyV1 {
    pub version: String,
    pub reference_family: String,
    pub duration_offsets_ticks: Vec<u64>,
    pub terminal_target_downward_speed_fraction: f64,
    pub contact_undershoot_m: f64,
    pub maximum_after_reference_ticks: u64,
    pub terminal_reference_extra_clearance_m: f64,
    pub scalar_inverse_iterations: u32,
    pub maximum_coast_clock_alignment_ticks: u64,
    pub geometry_convention: String,
}

impl Default for BodyAwareTerminalPolicyV1 {
    fn default() -> Self {
        Self {
            version: POLICY_VERSION.to_owned(),
            reference_family: "quadratic_horizontal_affine_vertical_exact_discrete".to_owned(),
            duration_offsets_ticks: vec![0, 60, 120, 180, 240, 300, 360],
            terminal_target_downward_speed_fraction: 0.5,
            contact_undershoot_m: 0.005,
            maximum_after_reference_ticks: 4,
            terminal_reference_extra_clearance_m: 0.1,
            scalar_inverse_iterations: 48,
            maximum_coast_clock_alignment_ticks: 1,
            geometry_convention: GEOMETRY_CONVENTION.to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalReferenceV1 {
    pub start: KinematicStateV2,
    pub end: KinematicStateV2,
    pub physics_ticks: u64,
    /// Acceleration c0 + c1*u + c2*u^2, u=j/(N-1).
    pub horizontal_coefficients_mps2: [f64; 3],
    pub initial_vertical_acceleration_mps2: f64,
    pub vertical_acceleration_delta_mps2: f64,
    pub identity: String,
}

fn reference_identity(reference: &BodyAwareTerminalReferenceV1) -> Result<String> {
    let mut canonical = reference.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

fn solve_three(mut a: [[f64; 4]; 3]) -> Result<[f64; 3]> {
    for column in 0..3 {
        let pivot = (column..3)
            .max_by(|left, right| a[*left][column].abs().total_cmp(&a[*right][column].abs()))
            .context("quadratic constraint pivot missing")?;
        a.swap(column, pivot);
        let scale = a[column][column];
        if !scale.is_finite() || scale.abs() <= 1.0e-12 {
            bail!("singular quadratic terminal constraints");
        }
        for value in &mut a[column][column..] {
            *value /= scale;
        }
        let pivot_values = a[column];
        for (row, values) in a.iter_mut().enumerate() {
            if row == column {
                continue;
            }
            let factor = values[column];
            for (value, pivot_value) in values[column..].iter_mut().zip(&pivot_values[column..]) {
                *value -= factor * pivot_value;
            }
        }
    }
    let result = [a[0][3], a[1][3], a[2][3]];
    if result.iter().any(|value| !value.is_finite()) {
        bail!("non-finite quadratic terminal coefficients");
    }
    Ok(result)
}

pub(super) fn build_reference(
    context: &RunContext,
    policy: &BodyAwareTerminalPolicyV1,
    start: KinematicStateV2,
    steps: u64,
) -> Result<BodyAwareTerminalReferenceV1> {
    if steps < 2 || !steps.is_multiple_of(2) {
        bail!("terminal reference requires a positive complete two-tick duration");
    }
    let end = KinematicStateV2 {
        position_m: Vec2::new(
            context.target_pad.center_x_m,
            context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m
                - policy.contact_undershoot_m,
        ),
        velocity_mps: Vec2::new(
            0.0,
            -policy.terminal_target_downward_speed_fraction
                * context.vehicle.safe_touchdown_normal_speed_mps,
        ),
    };
    let dt = context.sim.physics_dt_s();
    let n = steps as f64;
    let sj = n * (n - 1.0) / 2.0;
    let sj2 = n * (n - 1.0) * (2.0 * n - 1.0) / 6.0;
    let sj3 = sj * sj;
    let horizontal = solve_three([
        [
            dt * n,
            dt * sj / (n - 1.0),
            dt * sj2 / (n - 1.0).powi(2),
            end.velocity_mps.x - start.velocity_mps.x,
        ],
        [
            dt * dt * n * (n + 1.0) / 2.0,
            dt * dt * (n * sj - sj2) / (n - 1.0),
            dt * dt * (n * sj2 - sj3) / (n - 1.0).powi(2),
            end.position_m.x - start.position_m.x - start.velocity_mps.x * n * dt,
        ],
        [1.0, 1.0, 1.0, 0.0],
    ])?;
    let d = (end.velocity_mps.y - start.velocity_mps.y) / dt;
    let s = (end.position_m.y - start.position_m.y - start.velocity_mps.y * n * dt) / (dt * dt);
    let mut reference = BodyAwareTerminalReferenceV1 {
        start,
        end,
        physics_ticks: steps,
        horizontal_coefficients_mps2: horizontal,
        initial_vertical_acceleration_mps2: 6.0 * s / (n * (n + 1.0)) - 2.0 * d / n,
        vertical_acceleration_delta_mps2: 6.0 * d / (n * (n - 1.0))
            - 12.0 * s / (n * (n + 1.0) * (n - 1.0)),
        identity: String::new(),
    };
    reference.identity = reference_identity(&reference)?;
    Ok(reference)
}

impl BodyAwareTerminalReferenceV1 {
    fn net_acceleration(&self, tick: u64) -> Vec2 {
        if tick >= self.physics_ticks {
            return Vec2::new(
                0.0,
                self.initial_vertical_acceleration_mps2
                    + self.vertical_acceleration_delta_mps2 * (self.physics_ticks - 1) as f64,
            );
        }
        let u = tick as f64 / (self.physics_ticks - 1) as f64;
        let c = self.horizontal_coefficients_mps2;
        Vec2::new(
            c[0] + c[1] * u + c[2] * u * u,
            self.initial_vertical_acceleration_mps2
                + self.vertical_acceleration_delta_mps2 * tick as f64,
        )
    }

    pub(super) fn thrust(&self, context: &RunContext, tick: u64) -> Vec2 {
        self.net_acceleration(tick) + Vec2::new(0.0, context.world.gravity_mps2)
    }

    pub(super) fn state_at(&self, context: &RunContext, steps: u64) -> KinematicStateV2 {
        let dt = context.sim.physics_dt_s();
        if steps > self.physics_ticks {
            let end = self.state_at(context, self.physics_ticks);
            let k = (steps - self.physics_ticks) as f64;
            let a = self.net_acceleration(self.physics_ticks);
            return KinematicStateV2 {
                position_m: end.position_m
                    + end.velocity_mps * (k * dt)
                    + a * (dt * dt * k * (k + 1.0) / 2.0),
                velocity_mps: end.velocity_mps + a * (k * dt),
            };
        }
        let k = steps as f64;
        let sj = k * (k - 1.0) / 2.0;
        let sj2 = k * (k - 1.0) * (2.0 * k - 1.0) / 6.0;
        let sj3 = sj * sj;
        let n1 = (self.physics_ticks - 1) as f64;
        let c = self.horizontal_coefficients_mps2;
        let vx =
            self.start.velocity_mps.x + dt * (c[0] * k + c[1] * sj / n1 + c[2] * sj2 / (n1 * n1));
        let x = self.start.position_m.x
            + self.start.velocity_mps.x * k * dt
            + dt * dt
                * (c[0] * k * (k + 1.0) / 2.0
                    + c[1] * (k * sj - sj2) / n1
                    + c[2] * (k * sj2 - sj3) / (n1 * n1));
        let ay = self.initial_vertical_acceleration_mps2;
        let day = self.vertical_acceleration_delta_mps2;
        KinematicStateV2 {
            position_m: Vec2::new(
                x,
                self.start.position_m.y
                    + self.start.velocity_mps.y * k * dt
                    + dt * dt * (ay * k * (k + 1.0) / 2.0 + day * k * (k + 1.0) * (k - 1.0) / 6.0),
            ),
            velocity_mps: Vec2::new(
                vx,
                self.start.velocity_mps.y + dt * (ay * k + day * k * (k - 1.0) / 2.0),
            ),
        }
    }
}

pub(super) fn paired_mean(
    reference: &BodyAwareTerminalReferenceV1,
    context: &RunContext,
    tick: u64,
) -> Vec2 {
    (reference.thrust(context, tick) + reference.thrust(context, tick + 1)) * 0.5
}

/// Invert the held throttle's mean acceleration over both post-burn masses.
/// This uses no core transition and is independently checked by physical replay.
pub(crate) fn paired_throttle(
    context: &RunContext,
    policy: &BodyAwareTerminalPolicyV1,
    mass: f64,
    thrust: f64,
) -> Result<f64> {
    if !mass.is_finite() || mass <= 0.0 || !thrust.is_finite() || thrust < 0.0 {
        bail!("invalid paired throttle input");
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
        bail!("paired throttle above maximum or unsupported remaining mass");
    }
    let minimum = context.vehicle.min_throttle_frac;
    if thrust < mean(minimum) {
        bail!("paired throttle below minimum");
    }
    let (mut low, mut high) = (minimum, 1.0);
    for _ in 0..policy.scalar_inverse_iterations {
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

pub(super) fn clearance_policy(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
) -> Result<ClearancePolicy> {
    let bounds = |id: &str| -> Result<FlatPadBounds> {
        let pad = request
            .scenario
            .world
            .landing_pad(id)
            .context("body-aware pad missing")?;
        Ok(flat_pad_bounds(
            context,
            &PadInputV2 {
                center_x_m: pad.center_x_m,
                surface_y_m: pad.surface_y_m,
                width_m: pad.width_m,
            },
        ))
    };
    Ok(ClearancePolicy {
        source_pad: bounds(&request.source_pad_id)?,
        target_pad: bounds(&request.target_pad_id)?,
        minimum_clearance_m: request.policy.analytical_policy.minimum_clearance_m,
    })
}

/// Reuse the historical exact conservative rotated-body query. The launch
/// exception is explicit and can never leak into the local maneuver.
pub(crate) fn clearing_body_reserve_query(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
    state: &SimulationState,
    phase: &str,
    allow_source_exception: bool,
) -> Result<(f64, f64)> {
    let policy = clearance_policy(context, request)?;
    let aabb = body_aabb(state, &context.vehicle.geometry);
    let corridor = corridor_for_step(
        phase,
        state.velocity_mps,
        aabb,
        policy.source_pad,
        policy.target_pad,
    );
    let required = if allow_source_exception && corridor == "source_pad_transition" {
        0.0
    } else {
        policy.minimum_clearance_m
    };
    Ok((body_clearance(context, state, aabb)?, required))
}

/// Existing nominal corridor policy, with source exemption disabled after the
/// original launch. Unlike local clearing, terminal descent may enter its pad.
pub(crate) fn nominal_body_reserve_query(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
    state: &SimulationState,
    phase: &str,
    allow_source_exception: bool,
) -> Result<(f64, f64)> {
    let mut policy = clearance_policy(context, request)?;
    if !allow_source_exception {
        policy.source_pad.flat = false;
    }
    let aabb = body_aabb(state, &context.vehicle.geometry);
    let corridor = corridor_for_step(
        phase,
        state.velocity_mps,
        aabb,
        policy.source_pad,
        policy.target_pad,
    );
    let required = if corridor == "none" {
        policy.minimum_clearance_m
    } else {
        0.0
    };
    Ok((body_clearance(context, state, aabb)?, required))
}

pub(super) fn predicted_coast_kinematics(
    context: &RunContext,
    source: &SimulationState,
    ticks: u64,
) -> KinematicStateV2 {
    let mut state = KinematicStateV2 {
        position_m: source.position_m,
        velocity_mps: source.velocity_mps,
    };
    for _ in 0..ticks {
        state.velocity_mps.y -= context.world.gravity_mps2 * context.sim.physics_dt_s();
        state.position_m += state.velocity_mps * context.sim.physics_dt_s();
    }
    state
}
