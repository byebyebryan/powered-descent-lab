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

fn horizontal_coefficients(distance_m: f64, velocity_mps: f64, dt_s: f64, n: f64) -> (f64, f64) {
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
