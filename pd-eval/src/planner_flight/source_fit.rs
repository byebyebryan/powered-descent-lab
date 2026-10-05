//! Current planner source_fit; no research orchestration.
use anyhow::{Result, anyhow, bail};
use pd_core::Vec2;
use pd_plan::ballistic::AnalyticalBridgeV2;
use serde::{Deserialize, Serialize};

const MAX_CORRECTION_MPS2: f64 = 0.25;

/// One desired thrust vector held for a complete two-physics-tick interval.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct HeldSourceCommand {
    pub(super) thrust_acceleration_mps2: Vec2,
    pub(super) target_attitude_rad: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Correction(pub(super) [f64; 4]);

pub(super) fn paired_mean_seed(
    bridge: &AnalyticalBridgeV2,
    launch_target: f64,
) -> Result<Vec<HeldSourceCommand>> {
    if !bridge.steps.is_multiple_of(2) || bridge.samples.len() != bridge.steps as usize {
        bail!("reference source bridge does not split into complete two-tick pairs");
    }
    let pair_count = bridge.samples.len() / 2;
    let first_powered = bridge
        .samples
        .iter()
        .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
        .map(|sample| direction_angle(sample.thrust_acceleration_mps2))
        .ok_or_else(|| anyhow!("reference source bridge has no powered direction"))?;
    let paired_means = bridge
        .samples
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| (pair[0].thrust_acceleration_mps2 + pair[1].thrust_acceleration_mps2) * 0.5)
        .collect::<Vec<_>>();
    let first_powered_pair_index = paired_means
        .iter()
        .position(|mean| mean.length() > 1.0e-12)
        .ok_or_else(|| anyhow!("paired reference bridge has no powered command"))?;
    let mut previous_target = launch_target;
    let mut schedule = Vec::with_capacity(pair_count);
    for (pair_index, mean) in paired_means.into_iter().enumerate() {
        let mut mean = mean;
        if pair_index == first_powered_pair_index {
            mean = Vec2::new(first_powered.sin(), first_powered.cos()) * mean.length();
        }
        let target = if mean.length() > 1.0e-12 {
            direction_angle(mean)
        } else {
            previous_target
        };
        previous_target = target;
        schedule.push(HeldSourceCommand {
            thrust_acceleration_mps2: mean,
            target_attitude_rad: target,
        });
    }
    Ok(schedule)
}

pub(super) fn apply_correction(
    seed: &[HeldSourceCommand],
    correction: Correction,
) -> Vec<HeldSourceCommand> {
    let first_powered_index = seed
        .iter()
        .position(|command| command.thrust_acceleration_mps2.length() > 1.0e-12)
        .unwrap_or(0);
    let correction_span = seed.len().saturating_sub(1 + first_powered_index);
    seed.iter()
        .enumerate()
        .map(|(index, command)| {
            if index <= first_powered_index || correction_span == 0 {
                return *command;
            }
            let progress = (index - first_powered_index) as f64 / correction_span as f64;
            let normalized_time = 2.0 * progress - 1.0;
            let ramp = Vec2::new(
                correction.0[0] + correction.0[2] * normalized_time,
                correction.0[1] + correction.0[3] * normalized_time,
            ) * progress;
            let corrected = command.thrust_acceleration_mps2 + ramp;
            let target = if corrected.length() > 1.0e-12 {
                direction_angle(corrected)
            } else {
                command.target_attitude_rad
            };
            HeldSourceCommand {
                thrust_acceleration_mps2: corrected,
                target_attitude_rad: target,
            }
        })
        .collect()
}

pub(super) fn maximum_powered_slew(schedule: &[HeldSourceCommand]) -> f64 {
    let mut previous: Option<(usize, f64)> = None;
    let mut maximum = 0.0_f64;
    for (index, command) in schedule.iter().enumerate() {
        if command.thrust_acceleration_mps2.length() <= 1.0e-12 {
            continue;
        }
        if let Some((previous_index, previous_target)) = previous {
            let elapsed_s = (index - previous_index) as f64 * 2.0 / 120.0;
            maximum = maximum.max(
                shortest_angle_delta(previous_target, command.target_attitude_rad).abs()
                    / elapsed_s.max(f64::EPSILON),
            );
        }
        previous = Some((index, command.target_attitude_rad));
    }
    maximum
}

// These small fixed-size loops preserve the original row-major accumulation
// order to keep the bounded experiment's floating-point schedule unchanged.
#[allow(clippy::needless_range_loop)]
pub(super) fn damped_least_squares_step(
    jacobian: [[f64; 4]; 4],
    residual: [f64; 4],
    damping: f64,
) -> Option<[f64; 4]> {
    let mut normal = [[0.0_f64; 4]; 4];
    let mut rhs = [0.0_f64; 4];
    for row in 0..4 {
        for column in 0..4 {
            rhs[column] -= jacobian[row][column] * residual[row];
            for other in 0..4 {
                normal[column][other] += jacobian[row][column] * jacobian[row][other];
            }
        }
    }
    for (index, row) in normal.iter_mut().enumerate() {
        row[index] += damping;
    }
    solve_linear_system(normal, rhs)
}

// Pivot/elimination loops intentionally retain their original numerical order.
#[allow(clippy::needless_range_loop)]
fn solve_linear_system(mut matrix: [[f64; 4]; 4], mut rhs: [f64; 4]) -> Option<[f64; 4]> {
    for pivot in 0..4 {
        let best_row = (pivot..4).max_by(|left, right| {
            matrix[*left][pivot]
                .abs()
                .total_cmp(&matrix[*right][pivot].abs())
        })?;
        if matrix[best_row][pivot].abs() <= 1.0e-14 {
            return None;
        }
        matrix.swap(pivot, best_row);
        rhs.swap(pivot, best_row);
        let divisor = matrix[pivot][pivot];
        for column in pivot..4 {
            matrix[pivot][column] /= divisor;
        }
        rhs[pivot] /= divisor;
        for row in 0..4 {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for column in pivot..4 {
                matrix[row][column] -= factor * matrix[pivot][column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs.iter().all(|value| value.is_finite()).then_some(rhs)
}

pub(super) fn clamp_correction(correction: Correction) -> Correction {
    Correction(std::array::from_fn(|index| {
        correction.0[index].clamp(-MAX_CORRECTION_MPS2, MAX_CORRECTION_MPS2)
    }))
}

fn shortest_angle_delta(from: f64, to: f64) -> f64 {
    (to - from + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

fn direction_angle(thrust_acceleration: Vec2) -> f64 {
    thrust_acceleration.x.atan2(thrust_acceleration.y)
}
