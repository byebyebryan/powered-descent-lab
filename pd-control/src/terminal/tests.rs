use super::*;

fn gate_candidate(
    burn_time_s: f64,
    required_accel_ratio: f64,
    upward_accel_mps2: f64,
    tilt_feasible: bool,
    ready: bool,
) -> TerminalGateCandidate {
    TerminalGateCandidate {
        burn_time_s,
        required_accel_ratio,
        upward_accel_mps2,
        tilt_feasible,
        ready,
        terrain_min_clearance_m: TERRAIN_CLEARANCE_UNCONSTRAINED_M,
        terrain_first_violation_time_s: None,
        terrain_clearance_safe: true,
    }
}

fn release_test_controller() -> TerminalPdgController {
    let mut config = TerminalPdgControllerConfig::default();
    config.terminal_gate_hysteresis_ticks = 2;
    config.terminal_gate_latest_safe_release_buffer_s = 0.20;
    TerminalPdgController::new(config)
}

fn step_guidance_mode(
    controller: &mut TerminalPdgController,
    latest_safe_margin_s: f64,
    nominal_ready: bool,
) -> GuidanceMode {
    let mode = controller.select_guidance_mode(latest_safe_margin_s, nominal_ready);
    controller.last_mode = Some(mode);
    mode
}

#[test]
fn latest_safe_release_holds_through_small_positive_margins() {
    let mut controller = release_test_controller();
    controller.last_mode = Some(GuidanceMode::LatestSafe);

    assert_eq!(
        step_guidance_mode(&mut controller, 0.02, false),
        GuidanceMode::LatestSafe
    );
    assert_eq!(controller.latest_safe_release_ticks, 0);
    assert_eq!(
        step_guidance_mode(&mut controller, 0.20, false),
        GuidanceMode::LatestSafe
    );
    assert_eq!(controller.latest_safe_release_ticks, 0);
}

#[test]
fn nominal_pending_can_start_when_not_previously_latest_safe() {
    let mut controller = release_test_controller();

    assert_eq!(
        step_guidance_mode(&mut controller, 0.02, false),
        GuidanceMode::NominalPending
    );
    assert_eq!(controller.latest_safe_release_ticks, 0);
}

#[test]
fn latest_safe_does_not_release_to_nominal_pending_above_buffer() {
    let mut controller = release_test_controller();
    controller.last_mode = Some(GuidanceMode::LatestSafe);

    assert_eq!(
        step_guidance_mode(&mut controller, 0.21, false),
        GuidanceMode::LatestSafe
    );
    assert_eq!(controller.latest_safe_release_ticks, 0);
    assert_eq!(
        step_guidance_mode(&mut controller, 0.22, false),
        GuidanceMode::LatestSafe
    );
    assert_eq!(controller.latest_safe_release_ticks, 0);
}

#[test]
fn nominal_ready_releases_latest_safe_after_buffered_consecutive_ticks() {
    let mut controller = release_test_controller();
    controller.last_mode = Some(GuidanceMode::LatestSafe);

    assert_eq!(
        step_guidance_mode(&mut controller, 0.21, true),
        GuidanceMode::LatestSafe
    );
    assert_eq!(controller.latest_safe_release_ticks, 1);
    assert_eq!(
        step_guidance_mode(&mut controller, 0.22, true),
        GuidanceMode::NominalReady
    );
    assert_eq!(controller.latest_safe_release_ticks, 0);
}

#[test]
fn non_positive_margin_stays_latest_safe_even_when_nominal_ready() {
    let mut controller = release_test_controller();
    controller.last_mode = Some(GuidanceMode::LatestSafe);
    controller.latest_safe_release_ticks = 1;

    assert_eq!(
        step_guidance_mode(&mut controller, 0.0, true),
        GuidanceMode::LatestSafe
    );
    assert_eq!(controller.latest_safe_release_ticks, 0);
    assert_eq!(
        step_guidance_mode(&mut controller, -0.01, true),
        GuidanceMode::LatestSafe
    );
    assert_eq!(controller.latest_safe_release_ticks, 0);
}

#[test]
fn reset_state_clears_latest_safe_release_state() {
    let mut controller = release_test_controller();
    controller.last_phase = Some("descent".to_owned());
    controller.last_mode = Some(GuidanceMode::LatestSafe);
    controller.nominal_ready_ticks = 3;
    controller.latest_safe_release_ticks = 1;
    controller.touchdown_settle_active = true;
    controller.guidance_plan_admitted = true;
    controller.guidance_plan_completed = true;
    controller.guidance_plan = Some(TerminalGuidancePlan {
        arrival_time_s: 22.0,
    });
    controller.guidance_replan_count = 2;

    controller.reset_state();

    assert_eq!(controller.last_phase, None);
    assert_eq!(controller.last_mode, None);
    assert_eq!(controller.nominal_ready_ticks, 0);
    assert_eq!(controller.latest_safe_release_ticks, 0);
    assert!(!controller.touchdown_settle_active);
    assert!(!controller.guidance_plan_admitted);
    assert!(!controller.guidance_plan_completed);
    assert_eq!(controller.guidance_plan, None);
    assert_eq!(controller.guidance_replan_count, 0);
}

#[test]
fn terminal_guidance_plan_counts_down_without_moving_arrival_time() {
    let mut controller = TerminalPdgController::default();

    let first = controller.maintain_guidance_plan(10.0, 22.0, true, true);
    let second = controller.maintain_guidance_plan(11.5, 22.0, true, true);

    assert_eq!(first.arrival_time_s, 32.0);
    assert_eq!(second.arrival_time_s, first.arrival_time_s);
    assert_eq!(second.arrival_time_s - 11.5, 20.5);
    assert_eq!(controller.guidance_replan_count, 0);
}

#[test]
fn terminal_guidance_plan_waits_for_long_capture_while_ascending() {
    let mut controller = TerminalPdgController::default();
    controller.set_guidance_plan_retention_enabled(true);

    controller.update_guidance_plan_admission(4.0, 11.0);
    assert!(!controller.guidance_plan_admitted);
    assert!(!controller.guidance_plan_completed);

    controller
        .update_guidance_plan_admission(controller.config.terminal_gate_burn_time_max_s, 10.0);
    assert!(controller.guidance_plan_admitted);
    assert!(!controller.guidance_plan_completed);
}

#[test]
fn terminal_guidance_plan_declines_short_capture_after_apex() {
    let mut controller = TerminalPdgController::default();
    controller.set_guidance_plan_retention_enabled(true);

    controller.update_guidance_plan_admission(10.0, -1.0);

    assert!(!controller.guidance_plan_admitted);
    assert!(controller.guidance_plan_completed);
}

#[test]
fn terminal_guidance_plan_release_reason_prioritizes_captured_boundary() {
    let controller = TerminalPdgController::default();

    assert_eq!(
        controller.guidance_plan_release_reason(-4.0, 4.0, 12.0, 7.0, -0.1, -1.0),
        Some(GuidancePlanReleaseReason::CapturedBrakingBoundary)
    );
    assert_eq!(
        controller.guidance_plan_release_reason(-15.0, 40.0, 12.0, 10.0, -0.1, -0.1),
        Some(GuidancePlanReleaseReason::VerticalBrakingMargin)
    );
    assert_eq!(
        controller.guidance_plan_release_reason(1.0, 40.0, 12.0, 10.0, -0.1, -0.1),
        None
    );
    assert_eq!(
        controller.guidance_plan_release_reason(-4.0, 40.0, 12.0, 10.0, -0.1, 0.1),
        None
    );
}

#[test]
fn vertical_braking_margin_accounts_for_attitude_and_sink_rate() {
    let controller = TerminalPdgController::default();
    let upright_margin = controller.vertical_braking_margin_m(40.0, -10.0, 0.0, 16.0, 9.81);
    let tilted_margin = controller.vertical_braking_margin_m(40.0, -10.0, 0.5, 16.0, 9.81);
    let exhausted_margin = controller.vertical_braking_margin_m(20.0, -16.0, 0.0, 16.0, 9.81);

    assert!(upright_margin > 0.0);
    assert!(tilted_margin < upright_margin);
    assert!(exhausted_margin < 0.0);
}

#[test]
fn terminal_guidance_mode_change_does_not_extend_plan() {
    let mut controller = release_test_controller();
    let first = controller.maintain_guidance_plan(4.0, 14.0, true, true);
    controller.last_mode = Some(GuidanceMode::LatestSafe);
    let mode = controller.select_guidance_mode(0.5, true);
    controller.last_mode = Some(mode);
    let second = controller.maintain_guidance_plan(4.5, 22.0, true, true);

    assert_eq!(mode, GuidanceMode::LatestSafe);
    assert_eq!(first.arrival_time_s, second.arrival_time_s);
    assert_eq!(controller.guidance_replan_count, 0);
}

#[test]
fn terminal_guidance_plan_replaces_materially_infeasible_horizon_once() {
    let mut controller = TerminalPdgController::default();
    controller.maintain_guidance_plan(0.0, 14.0, true, true);

    let replacement = controller.maintain_guidance_plan(2.0, 10.0, false, true);
    let retained = controller.maintain_guidance_plan(2.5, 10.0, true, true);

    assert_eq!(replacement.arrival_time_s, 12.0);
    assert_eq!(retained.arrival_time_s, replacement.arrival_time_s);
    assert_eq!(controller.guidance_replan_count, 1);
}

#[test]
fn terminal_guidance_plan_holds_when_no_feasible_replacement_exists() {
    let mut controller = TerminalPdgController::default();
    let initial = controller.maintain_guidance_plan(0.0, 14.0, true, true);

    let retained = controller.maintain_guidance_plan(2.0, 22.0, false, false);

    assert_eq!(retained.arrival_time_s, initial.arrival_time_s);
    assert_eq!(controller.guidance_replan_count, 0);
}

#[test]
fn terminal_guidance_plan_replaces_expired_horizon() {
    let mut controller = TerminalPdgController::default();
    controller.maintain_guidance_plan(0.0, 3.0, true, true);

    let replacement = controller.maintain_guidance_plan(3.0, 6.0, false, false);

    assert_eq!(replacement.arrival_time_s, 9.0);
    assert_eq!(controller.guidance_replan_count, 1);
}

#[test]
fn urgent_lateral_latest_safe_prefers_shorter_candidate() {
    let controller = TerminalPdgController::default();
    let mut candidates = vec![
        gate_candidate(14.0, 0.9, 3.0, true, true),
        gate_candidate(6.0, 1.2, 5.0, true, false),
    ];

    let selected = controller.select_latest_safe_candidate(&mut candidates, 32.0, 18.0);

    assert_eq!(selected.burn_time_s, 6.0);
    assert_eq!(selected.required_accel_ratio, 1.2);
}

#[test]
fn long_capture_only_activates_for_high_urgent_over_authority_candidate() {
    let controller = TerminalPdgController::default();
    let candidate = gate_candidate(6.0, 1.01, 5.0, true, false);

    assert!(controller.latest_safe_long_capture_needed(candidate, 80.0, -40.0, 32.0, 18.0));

    let feasible_candidate = TerminalGateCandidate {
        required_accel_ratio: 0.99,
        ready: true,
        ..candidate
    };
    assert!(!controller.latest_safe_long_capture_needed(
        feasible_candidate,
        80.0,
        -40.0,
        32.0,
        18.0
    ));
    assert!(!controller.latest_safe_long_capture_needed(candidate, 20.0, -40.0, 32.0, 18.0));
    assert!(!controller.latest_safe_long_capture_needed(candidate, 80.0, -40.0, 12.0, 18.0));
    assert!(!controller.latest_safe_long_capture_needed(candidate, 80.0, 40.0, 32.0, 18.0));
}

#[test]
fn long_capture_prefers_added_lower_ratio_candidate() {
    let controller = TerminalPdgController::default();
    let mut candidates = vec![
        gate_candidate(6.0, 1.2, 5.0, true, false),
        gate_candidate(14.0, 0.9, 3.0, true, true),
        gate_candidate(22.0, 1.2, 2.0, true, false),
        gate_candidate(30.0, 1.05, 2.0, true, false),
    ];

    let selected = controller
        .select_latest_safe_long_capture_candidate(&mut candidates)
        .unwrap();

    assert_eq!(selected.burn_time_s, 30.0);
    assert_eq!(selected.required_accel_ratio, 1.05);
}

#[test]
fn terrain_constrained_order_prefers_more_clearance_when_both_candidates_clip_terrain() {
    let mut lower_clearance = gate_candidate(6.0, 0.8, 4.0, true, true);
    lower_clearance.terrain_min_clearance_m = -40.0;
    lower_clearance.terrain_first_violation_time_s = Some(2.0);
    lower_clearance.terrain_clearance_safe = false;

    let mut higher_clearance = gate_candidate(9.0, 1.1, 4.0, true, true);
    higher_clearance.terrain_min_clearance_m = -5.0;
    higher_clearance.terrain_first_violation_time_s = Some(4.0);
    higher_clearance.terrain_clearance_safe = false;

    let mut candidates = [lower_clearance, higher_clearance];
    candidates.sort_by(candidate_preference_order);

    assert_eq!(candidates[0], higher_clearance);
}

#[test]
fn terrain_constrained_order_keeps_safe_candidates_ahead_of_unsafe_candidates() {
    let safe_high_ratio = gate_candidate(6.0, 1.4, 4.0, true, false);
    let mut unsafe_low_ratio = gate_candidate(6.0, 0.5, 4.0, true, true);
    unsafe_low_ratio.terrain_min_clearance_m = 0.0;
    unsafe_low_ratio.terrain_first_violation_time_s = Some(1.0);
    unsafe_low_ratio.terrain_clearance_safe = false;

    let mut candidates = [unsafe_low_ratio, safe_high_ratio];
    candidates.sort_by(latest_safe_preference_order);

    assert_eq!(candidates[0], safe_high_ratio);
}
