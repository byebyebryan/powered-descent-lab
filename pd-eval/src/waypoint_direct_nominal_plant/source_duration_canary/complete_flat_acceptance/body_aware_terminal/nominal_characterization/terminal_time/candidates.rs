use anyhow::bail;
use serde::Serialize;

const HELD_TICKS: u64 = 2;
const TERMINAL_EXTRA_TICKS: u64 = 4;
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
mod tests {
    use super::{DurationCandidate, alternative_durations};

    fn alternatives(
        distance_m: f64,
        forward_velocity_mps: f64,
        dt_s: f64,
        entry_tick: u64,
        deadline: u64,
        baseline_ticks: u64,
    ) -> Vec<DurationCandidate> {
        alternative_durations(
            distance_m,
            forward_velocity_mps,
            dt_s,
            entry_tick,
            deadline,
            baseline_ticks,
        )
        .unwrap()
    }

    fn horizontal_coefficients(
        distance_m: f64,
        velocity_mps: f64,
        dt_s: f64,
        n: f64,
    ) -> (f64, f64) {
        let b = velocity_mps / (n * (n - 1.0));
        let a = 12.0 * (distance_m / dt_s - velocity_mps * (n - 2.0) / 3.0)
            / (n * (n + 1.0) * (n - 1.0) * (n - 2.0));
        (a, b)
    }

    fn horizontal_velocity(n: f64, a: f64, b: f64, k: f64) -> f64 {
        (n - k) * (n - 1.0 - k) * (a * k + b)
    }

    #[test]
    fn zero_initial_acceleration_anchor_matches_integral_duration_identity() {
        let n = 7.0;
        let q = (n - 1.0) / 2.0;
        let (a, b) = horizontal_coefficients(q, 1.0, 1.0, n);
        let initial_step_acceleration =
            horizontal_velocity(n, a, b, 1.0) - horizontal_velocity(n, a, b, 0.0);
        let candidates = alternatives(q, 1.0, 1.0, 0, 1_000, 0);

        assert!(initial_step_acceleration.abs() <= 1.0e-12);
        assert_eq!(candidates[0].candidate_index, 1);
        assert_eq!(candidates[0].unrounded_ticks, n);
        assert_eq!(
            candidates[0].provenance,
            "zero_initial_horizontal_acceleration"
        );
    }

    #[test]
    fn affine_acceleration_anchor_matches_integral_duration_identity() {
        let n = 8.0;
        let q = (n - 2.0) / 3.0;
        let (a, _) = horizontal_coefficients(q, 1.0, 1.0, n);
        let candidates = alternatives(q, 1.0, 1.0, 0, 1_000, 0);

        assert!(a.abs() <= 1.0e-12);
        assert_eq!(candidates[1].candidate_index, 2);
        assert_eq!(candidates[1].unrounded_ticks, n);
        assert_eq!(candidates[1].provenance, "affine_horizontal_acceleration");
    }

    #[test]
    fn signed_and_near_zero_motion_keeps_only_the_baseline_branch() {
        assert!(alternatives(-1.0, 1.0, 1.0, 0, 100, 10).is_empty());
        assert!(alternatives(1.0e-9, 1.0, 1.0, 0, 100, 10).is_empty());
        assert!(alternatives(0.5e-9, 1.0, 1.0, 0, 100, 10).is_empty());
        assert!(alternatives(1.0, -1.0, 1.0, 0, 100, 10).is_empty());
        assert!(alternatives(1.0, 1.0e-9, 1.0, 0, 100, 10).is_empty());
        assert!(alternatives(1.0, 0.5e-9, 1.0, 0, 100, 10).is_empty());
    }

    #[test]
    fn nonfinite_inputs_and_unrepresentable_calculations_are_errors() {
        assert!(alternative_durations(f64::NAN, 1.0, 1.0, 0, 100, 0).is_err());
        assert!(alternative_durations(1.0, f64::INFINITY, 1.0, 0, 100, 0).is_err());
        assert!(alternative_durations(1.0, 1.0, f64::NAN, 0, 100, 0).is_err());
        assert!(alternative_durations(1.0, 1.0, 0.0, 0, 100, 0).is_err());
        assert!(alternative_durations(1.0, 1.0, -1.0, 0, 100, 0).is_err());
        assert!(alternative_durations(1.0e308, 1.0, 1.0, 0, 100, 0).is_err());
    }

    #[test]
    fn upward_even_rounding_clips_to_even_deadline_after_extra_ticks() {
        // The clock has 25 ticks after entry; four are reserved for terminal
        // extras, leaving 21, whose largest held-pair budget is 20.
        let candidates = alternatives(8.0, 1.0, 1.0, 100, 125, 0);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].deadline_limit_ticks, 20);
        assert_eq!(candidates[0].unrounded_ticks, 17.0);
        assert_eq!(candidates[0].rounded_ticks, 18.0);
        assert_eq!(candidates[0].physics_ticks, 18);
        assert!(!candidates[0].clipped);
        assert_eq!(candidates[1].unrounded_ticks, 26.0);
        assert_eq!(candidates[1].rounded_ticks, 26.0);
        assert_eq!(candidates[1].physics_ticks, 20);
        assert!(candidates[1].clipped);
    }

    #[test]
    fn positive_distance_interval_enforces_four_tick_floor_and_no_reversal_cap() {
        let candidates = alternatives(0.25, 1.0, 1.0, 0, 100, 0);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].unrounded_ticks, 1.5);
        assert_eq!(candidates[0].rounded_ticks, 4.0);
        assert_eq!(candidates[0].physics_ticks, 4);
        assert_eq!(candidates[0].no_reversal_limit_ticks, 4.0);

        // A smaller no-reversal interval cannot contain the required minimum.
        assert!(alternatives(0.2, 1.0, 1.0, 0, 100, 0).is_empty());
    }

    #[test]
    fn expired_or_too_short_deadline_returns_a_finite_empty_interval() {
        assert!(alternatives(5.0, 1.0, 1.0, 10, 10, 0).is_empty());
        assert!(alternatives(5.0, 1.0, 1.0, 10, 17, 0).is_empty());
    }

    #[test]
    fn deduplicates_baseline_and_clipped_alternatives_within_three_total_times() {
        let baseline = 12;
        let baseline_deduplicated = alternatives(5.0, 1.0, 1.0, 0, 100, baseline);
        assert_eq!(baseline_deduplicated.len(), 1);
        assert_eq!(baseline_deduplicated[0].candidate_index, 2);
        assert_eq!(baseline_deduplicated[0].physics_ticks, 18);
        assert!(
            baseline_deduplicated
                .iter()
                .all(|candidate| candidate.physics_ticks != baseline)
        );

        let clipped_duplicates = alternatives(0.7, 1.0, 1.0, 0, 100, 0);
        assert_eq!(clipped_duplicates.len(), 1);
        assert_eq!(clipped_duplicates[0].candidate_index, 1);
        assert_eq!(clipped_duplicates[0].physics_ticks, 4);

        let retained_with_baseline = 1 + alternatives(5.0, 1.0, 1.0, 0, 100, 0).len();
        assert!(retained_with_baseline <= 3);
    }

    #[test]
    fn finite_rounded_values_above_u64_clip_to_the_clock_without_casting() {
        let candidates = alternatives(1.0e100, 1.0, 1.0, 10, 1_000, 0);

        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].rounded_ticks > u64::MAX as f64);
        assert_eq!(candidates[0].deadline_limit_ticks, 986);
        assert_eq!(candidates[0].physics_ticks, 986);
        assert!(candidates[0].clipped);
    }
}
