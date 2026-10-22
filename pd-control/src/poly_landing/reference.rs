//! Accepted finite polynomial reference arithmetic, independent of planners.
use pd_core::{RunContext, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LandingKinematicsV1 {
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
}

/// Numeric reference only. Historical callers retain their own serialized DTOs
/// and identities; this type does not reinterpret those records.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolynomialReferenceV1 {
    pub start: LandingKinematicsV1,
    pub end: LandingKinematicsV1,
    pub physics_ticks: u64,
    pub horizontal_coefficients_mps2: [f64; 3],
    pub initial_vertical_acceleration_mps2: f64,
    pub vertical_acceleration_delta_mps2: f64,
}

fn solve_three(mut a: [[f64; 4]; 3]) -> Result<[f64; 3], String> {
    for column in 0..3 {
        let pivot = (column..3)
            .max_by(|left, right| a[*left][column].abs().total_cmp(&a[*right][column].abs()))
            .ok_or("quadratic constraint pivot missing")?;
        a.swap(column, pivot);
        let scale = a[column][column];
        if !scale.is_finite() || scale.abs() <= 1.0e-12 {
            return Err("singular quadratic terminal constraints".into());
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
        return Err("non-finite quadratic terminal coefficients".into());
    }
    Ok(result)
}

/// Construct the established exact-discrete reference. No native transition,
/// duration selection, terrain search or command emission occurs here.
pub fn build_reference(
    context: &RunContext,
    start: LandingKinematicsV1,
    end: LandingKinematicsV1,
    steps: u64,
) -> Result<PolynomialReferenceV1, String> {
    if steps < 2 || !steps.is_multiple_of(2) {
        return Err("terminal reference requires a positive complete two-tick duration".into());
    }
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
    let reference = PolynomialReferenceV1 {
        start,
        end,
        physics_ticks: steps,
        horizontal_coefficients_mps2: horizontal,
        initial_vertical_acceleration_mps2: 6.0 * s / (n * (n + 1.0)) - 2.0 * d / n,
        vertical_acceleration_delta_mps2: 6.0 * d / (n * (n - 1.0))
            - 12.0 * s / (n * (n + 1.0) * (n - 1.0)),
    };
    Ok(reference)
}

impl PolynomialReferenceV1 {
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

    pub fn thrust(&self, context: &RunContext, tick: u64) -> Vec2 {
        self.net_acceleration(tick) + Vec2::new(0.0, context.world.gravity_mps2)
    }

    pub fn state_at(&self, context: &RunContext, steps: u64) -> LandingKinematicsV1 {
        let dt = context.sim.physics_dt_s();
        if steps > self.physics_ticks {
            let end = self.state_at(context, self.physics_ticks);
            let k = (steps - self.physics_ticks) as f64;
            let a = self.net_acceleration(self.physics_ticks);
            return LandingKinematicsV1 {
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
        LandingKinematicsV1 {
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

pub fn paired_mean(reference: &PolynomialReferenceV1, context: &RunContext, tick: u64) -> Vec2 {
    (reference.thrust(context, tick) + reference.thrust(context, tick + 1)) * 0.5
}
