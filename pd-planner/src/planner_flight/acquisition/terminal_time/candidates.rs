//! Current planner acquisition terminal_time candidates; no research orchestration.
use anyhow::bail;
use serde::Serialize;

pub(super) const HELD_TICKS: u64 = 2;

pub(super) const TERMINAL_EXTRA_TICKS: u64 = 4;

const MINIMUM_TERMINAL_TICKS: u64 = 4;

const NEAR_ZERO_TOLERANCE: f64 = 1.0e-9;

// `u64::MAX as f64` rounds up to this exclusive limit.
const U64_EXCLUSIVE_LIMIT_F64: f64 = 18_446_744_073_709_551_616.0;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(super) struct DurationCandidate {
    pub(super) candidate_index: usize,
    pub(super) provenance: String,
    pub(super) unrounded_ticks: f64,
    pub(super) rounded_ticks: f64,
    pub(super) physics_ticks: u64,
    pub(super) no_reversal_limit_ticks: f64,
    pub(super) deadline_limit_ticks: u64,
    pub(super) clipped: bool,
}

/// Build the two bounded horizontal-shape duration alternatives.
///
/// This helper only constructs tick counts. The caller remains responsible
/// for retaining the baseline and screening every duration with the existing
/// analytical predicates.
pub(super) fn alternative_durations(
    distance_m: f64,
    forward_velocity_mps: f64,
    dt_s: f64,
    entry_tick: u64,
    deadline: u64,
    baseline_ticks: u64,
) -> anyhow::Result<Vec<DurationCandidate>> {
    if !distance_m.is_finite() || !forward_velocity_mps.is_finite() || !dt_s.is_finite() {
        bail!("terminal duration inputs must be finite");
    }
    if dt_s <= 0.0 {
        bail!("physics timestep must be positive");
    }

    // These branches retain only the caller's baseline candidate. They must
    // run before division so direction and near-zero motion cannot seed a
    // second duration family.
    if distance_m < 0.0 || distance_m <= NEAR_ZERO_TOLERANCE {
        return Ok(Vec::new());
    }
    if forward_velocity_mps <= NEAR_ZERO_TOLERANCE {
        return Ok(Vec::new());
    }

    let distance_per_tick = forward_velocity_mps * dt_s;
    if !distance_per_tick.is_finite() || distance_per_tick <= 0.0 {
        bail!("terminal duration distance-per-tick calculation overflowed");
    }
    let distance_in_velocity_ticks = distance_m / distance_per_tick;
    if !distance_in_velocity_ticks.is_finite() || distance_in_velocity_ticks <= 0.0 {
        bail!("terminal duration ratio is not representable");
    }

    let no_reversal_limit_ticks = 4.0 * distance_in_velocity_ticks + 3.0;
    if !no_reversal_limit_ticks.is_finite() {
        bail!("terminal no-reversal bound overflowed");
    }

    let anchors = [
        (
            1,
            "zero_initial_horizontal_acceleration",
            2.0 * distance_in_velocity_ticks + 1.0,
        ),
        (
            2,
            "affine_horizontal_acceleration",
            3.0 * distance_in_velocity_ticks + 2.0,
        ),
    ];
    let mut prepared_anchors = Vec::with_capacity(anchors.len());
    for (candidate_index, provenance, unrounded_ticks) in anchors {
        if !unrounded_ticks.is_finite() || unrounded_ticks <= 0.0 {
            bail!("terminal duration anchor calculation overflowed");
        }
        let even_rounded_ticks = (unrounded_ticks / HELD_TICKS as f64).ceil() * HELD_TICKS as f64;
        if !even_rounded_ticks.is_finite() {
            bail!("terminal duration rounding overflowed");
        }
        let rounded_ticks = even_rounded_ticks.max(MINIMUM_TERMINAL_TICKS as f64);
        prepared_anchors.push((candidate_index, provenance, unrounded_ticks, rounded_ticks));
    }

    let deadline_limit_ticks = deadline
        .checked_sub(entry_tick)
        .and_then(|remaining| remaining.checked_sub(TERMINAL_EXTRA_TICKS))
        .map(|remaining| remaining - remaining % HELD_TICKS)
        .unwrap_or(0);
    if deadline_limit_ticks < MINIMUM_TERMINAL_TICKS {
        return Ok(Vec::new());
    }

    let no_reversal_even_limit =
        (no_reversal_limit_ticks / HELD_TICKS as f64).floor() * HELD_TICKS as f64;
    if !no_reversal_even_limit.is_finite() || no_reversal_even_limit < 0.0 {
        bail!("terminal no-reversal even bound is not representable");
    }

    // Do not cast a floating value at or above 2^64. In that case the
    // finite, integer deadline cap is necessarily the tighter bound.
    let no_reversal_cap_ticks = if no_reversal_even_limit >= U64_EXCLUSIVE_LIMIT_F64 {
        u64::MAX
    } else {
        no_reversal_even_limit as u64
    };
    let upper_limit_ticks = no_reversal_cap_ticks.min(deadline_limit_ticks);
    if upper_limit_ticks < MINIMUM_TERMINAL_TICKS {
        return Ok(Vec::new());
    }

    let mut candidates = Vec::with_capacity(prepared_anchors.len());
    for (candidate_index, provenance, unrounded_ticks, rounded_ticks) in prepared_anchors {
        // The upward-rounded request may exceed u64 while the deadline still
        // provides a finite cap. Preserve that rounded value for provenance,
        // and apply the cap before any float-to-integer conversion.
        let (requested_ticks, request_exceeds_u64) = if rounded_ticks >= U64_EXCLUSIVE_LIMIT_F64 {
            (u64::MAX, true)
        } else {
            (rounded_ticks as u64, false)
        };
        let physics_ticks = requested_ticks.min(upper_limit_ticks);
        let clipped = request_exceeds_u64 || requested_ticks > upper_limit_ticks;

        if physics_ticks == baseline_ticks
            || candidates
                .iter()
                .any(|candidate: &DurationCandidate| candidate.physics_ticks == physics_ticks)
        {
            continue;
        }

        candidates.push(DurationCandidate {
            candidate_index,
            provenance: provenance.to_owned(),
            unrounded_ticks,
            rounded_ticks,
            physics_ticks,
            no_reversal_limit_ticks,
            deadline_limit_ticks,
            clipped,
        });
    }

    Ok(candidates)
}
#[cfg(test)]
mod tests;
