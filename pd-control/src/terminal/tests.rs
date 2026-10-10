use super::*;

#[test]
fn terminal_coordination_is_explicit_and_survives_reset() {
    let mut controller = TerminalPdgController::default().with_ballistic_terminal_coordination();
    assert!(!TerminalPdgController::default().ballistic_terminal_coordination_enabled);
    controller.reset_state();
    assert!(controller.ballistic_terminal_coordination_enabled);
    assert!(controller.ballistic_landing_braking_guard_enabled);
    assert!(controller.ballistic_landing_body_centering_enabled);
}

#[test]
fn coordinated_braking_preserves_lateral_acceleration_when_lift_fits() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    observation.touchdown_clearance_m = 100.0;
    observation.velocity_mps = pd_core::Vec2::new(50.0, -35.0);
    let view = ControllerView::new(&ctx, &observation);
    let mut controller = TerminalPdgController::default().with_ballistic_terminal_coordination();
    let mut state = controller.compute_command_state(&view);
    state.throttle_frac = 0.4;
    state.max_tilt_rad = 1.0;
    for sign in [-1.0, 1.0] {
        state.target_attitude_rad = sign * 0.4;
        let command = controller.coordinated_braking_command(&view, &state);
        let max_accel = ctx.vehicle.max_thrust_n / observation.mass_kg;
        let nominal_applied = ctx.vehicle.min_throttle_frac
            + state.throttle_frac * (1.0 - ctx.vehicle.min_throttle_frac);
        let applied = ctx.vehicle.min_throttle_frac
            + command.throttle_frac * (1.0 - ctx.vehicle.min_throttle_frac);
        let expected_ax = max_accel * nominal_applied * state.target_attitude_rad.sin();
        assert!(
            (max_accel * applied * command.target_attitude_rad.sin() - expected_ax).abs() < 1e-9
        );
        let expected_ay = observation.gravity_mps2
            + required_braking_accel_mps2(
                35.0,
                controller.config.vy_touch_cap_mps,
                100.0 - controller.config.touchdown_idle_clearance_m,
            );
        assert!(
            (max_accel * applied * command.target_attitude_rad.cos() - expected_ay).abs() < 1e-9
        );
        assert!((0.0..=1.0).contains(&command.throttle_frac));
    }
}

#[test]
fn coordinated_braking_saturates_lift_before_sideways_demand() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    observation.touchdown_clearance_m = 10.0;
    observation.velocity_mps.y = -80.0;
    let view = ControllerView::new(&ctx, &observation);
    let mut controller = TerminalPdgController::default().with_ballistic_terminal_coordination();
    let mut state = controller.compute_command_state(&view);
    state.throttle_frac = 1.0;
    state.target_attitude_rad = -0.7;
    let command = controller.coordinated_braking_command(&view, &state);
    assert_eq!(command.throttle_frac, 1.0);
    assert_eq!(command.target_attitude_rad, 0.0);
}

#[test]
fn coordinated_touchdown_removes_small_inherited_tilt_only_when_body_safe() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    observation.touchdown_clearance_m = 0.4;
    observation.velocity_mps = pd_core::Vec2::new(0.1, -0.8);
    observation.target_dx_m = -2.0;
    observation.attitude_rad = -0.06;
    let mut controller = TerminalPdgController::default().with_ballistic_terminal_coordination();
    let view = ControllerView::new(&ctx, &observation);
    let mut state = controller.compute_command_state(&view);
    state.target_attitude_rad = -0.06;
    let command = controller.touchdown_cut_command(&view, &state).unwrap();
    assert_eq!(command.target_attitude_rad, 0.0);
    assert!(controller.touchdown_settle_active);
    controller.reset_state();
    observation.target_dx_m = -17.0;
    let view = ControllerView::new(&ctx, &observation);
    controller.touchdown_cut_command(&view, &state);
    assert!(!controller.touchdown_settle_active);
}

#[test]
fn ballistic_body_centering_is_explicit_and_survives_reset() {
    let mut controller = TerminalPdgController::default().with_ballistic_landing_body_centering();
    assert!(!TerminalPdgController::default().ballistic_landing_body_centering_enabled);
    controller.reset_state();
    assert!(controller.ballistic_landing_body_centering_enabled);
    assert!(!controller.ballistic_landing_braking_guard_enabled);
}

#[test]
fn ballistic_body_centering_checks_hull_feet_and_rotation_symmetrically() {
    let (mut ctx, mut observation) = ballistic_duration_query_fixture();
    let ordinary = TerminalPdgController::default();
    let controller = TerminalPdgController::default().with_ballistic_landing_body_centering();
    observation.attitude_rad = 0.0;
    for side in [-1.0, 1.0] {
        observation.target_dx_m = side * 14.0;
        assert!(controller.rescue_inside_pad(&ControllerView::new(&ctx, &observation)));
        observation.target_dx_m = side * 15.0;
        assert!(!controller.rescue_inside_pad(&ControllerView::new(&ctx, &observation)));
        assert!(ordinary.rescue_inside_pad(&ControllerView::new(&ctx, &observation)));
        observation.target_dx_m = side * 13.0;
        observation.attitude_rad = side * 0.5;
        assert!(!controller.rescue_inside_pad(&ControllerView::new(&ctx, &observation)));
        observation.attitude_rad = 0.0;
    }
    ctx.vehicle.geometry.touchdown_half_span_m = 6.0;
    observation.target_dx_m = 13.0;
    assert!(!controller.rescue_inside_pad(&ControllerView::new(&ctx, &observation)));
}

#[test]
fn ballistic_body_centering_recenters_slow_drift_without_spending_vertical_authority() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    for side in [-1.0, 1.0] {
        observation.position_m = pd_core::Vec2::new(side * 15.0, 25.0);
        observation.target_dx_m = -side * 15.0;
        observation.height_above_target_m = 25.0;
        observation.touchdown_clearance_m = 20.0;
        observation.attitude_rad = 0.0;
        for vx in [0.0, side * 0.8, -side * 0.8] {
            observation.velocity_mps = pd_core::Vec2::new(vx, -4.0);
            let view = ControllerView::new(&ctx, &observation);
            let mut baseline =
                TerminalPdgController::default().with_ballistic_landing_braking_guard();
            let mut controller = TerminalPdgController::default()
                .with_ballistic_landing_braking_guard()
                .with_ballistic_landing_body_centering();
            let state = baseline.compute_command_state(&view);
            let command = controller.touchdown_cut_command(&view, &state).unwrap();
            assert!(command.target_attitude_rad * side < 0.0);
            let applied = ctx.vehicle.min_throttle_frac
                + command.throttle_frac * (1.0 - ctx.vehicle.min_throttle_frac);
            let ay = ctx.vehicle.max_thrust_n / observation.mass_kg
                * applied
                * command.target_attitude_rad.cos();
            let required = observation.gravity_mps2
                + required_braking_accel_mps2(
                    4.0,
                    0.0,
                    20.0 - controller.config.touchdown_rescue_clearance_m,
                );
            assert!(ay + 1e-9 >= required);
        }
    }
}

#[test]
fn ballistic_body_centering_keeps_centered_safe_commands_exact() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    observation.position_m = pd_core::Vec2::new(0.0, 15.0);
    observation.target_dx_m = 0.0;
    observation.height_above_target_m = 15.0;
    observation.touchdown_clearance_m = 10.0;
    observation.attitude_rad = 0.0;
    observation.velocity_mps = pd_core::Vec2::new(0.4, -2.0);
    let mut baseline = TerminalPdgController::default().with_ballistic_landing_braking_guard();
    let mut controller = TerminalPdgController::default()
        .with_ballistic_landing_braking_guard()
        .with_ballistic_landing_body_centering();
    for _ in 0..3 {
        assert_eq!(
            baseline.update(&ctx, &observation),
            controller.update(&ctx, &observation)
        );
    }
}

#[test]
fn ballistic_braking_guard_is_explicit_and_survives_reset() {
    let mut controller = TerminalPdgController::default().with_ballistic_landing_braking_guard();
    assert!(!TerminalPdgController::default().ballistic_landing_braking_guard_enabled);
    controller.reset_state();
    assert!(controller.ballistic_landing_braking_guard_enabled);
    assert!(!controller.ballistic_landing_countdown_enabled);
    assert!(!controller.ballistic_landing_duration_fallback_enabled);
}

#[test]
fn ballistic_braking_guard_reuses_rescue_above_final_height() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    observation.position_m = pd_core::Vec2::new(0.0, 105.0);
    observation.height_above_target_m = 105.0;
    observation.touchdown_clearance_m = 100.0;
    observation.target_dx_m = 0.0;
    observation.velocity_mps = pd_core::Vec2::new(0.0, -35.0);
    let view = ControllerView::new(&ctx, &observation);
    let mut baseline = TerminalPdgController::default();
    let mut guard = TerminalPdgController::default().with_ballistic_landing_braking_guard();
    let mut state = baseline.compute_command_state(&view);
    state.throttle_frac = 0.1;
    state.target_attitude_rad = 0.0;
    assert!(!baseline.ballistic_braking_guard_active(&view, &state));
    assert!(guard.ballistic_braking_guard_active(&view, &state));
    assert!(baseline.touchdown_cut_command(&view, &state).is_none());
    let command = guard.touchdown_cut_command(&view, &state).unwrap();
    let applied = ctx.vehicle.min_throttle_frac
        + command.throttle_frac * (1.0 - ctx.vehicle.min_throttle_frac);
    let required = observation.gravity_mps2
        + required_braking_accel_mps2(
            35.0,
            guard.config.vy_touch_cap_mps,
            100.0 - guard.config.touchdown_idle_clearance_m,
        );
    assert!(applied * ctx.vehicle.max_thrust_n / observation.mass_kg + 1e-9 >= required);
    assert_eq!(command.target_attitude_rad, 0.0);
    state.throttle_frac = 1.0;
    assert!(!guard.ballistic_braking_guard_active(&view, &state));
    observation.velocity_mps.y = 5.0;
    state.throttle_frac = 0.0;
    assert!(
        !guard.ballistic_braking_guard_active(&ControllerView::new(&ctx, &observation), &state)
    );
}

#[test]
fn ballistic_braking_guard_preserves_final_height_rescue() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    observation.position_m = pd_core::Vec2::new(0.0, 8.0);
    observation.height_above_target_m = 8.0;
    observation.touchdown_clearance_m = 3.0;
    observation.target_dx_m = 0.0;
    observation.velocity_mps = pd_core::Vec2::new(3.0, -10.0);
    let view = ControllerView::new(&ctx, &observation);
    let mut baseline = TerminalPdgController::default();
    let mut guard = TerminalPdgController::default().with_ballistic_landing_braking_guard();
    let state = baseline.compute_command_state(&view);
    assert!(!guard.ballistic_braking_guard_active(&view, &state));
    assert_eq!(
        baseline.touchdown_cut_command(&view, &state),
        guard.touchdown_cut_command(&view, &state)
    );
}

#[test]
fn ballistic_braking_guard_leaves_safe_nominal_commands_alone() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    observation.velocity_mps = pd_core::Vec2::new(4.0, -10.0);
    let mut baseline = TerminalPdgController::default();
    let mut guard = TerminalPdgController::default().with_ballistic_landing_braking_guard();
    for _ in 0..3 {
        let old = baseline.update(&ctx, &observation);
        let new = guard.update(&ctx, &observation);
        assert_eq!(old.command, new.command);
        let mut new_metrics = new.metrics;
        assert_eq!(
            new_metrics.remove("guidance.ballistic_braking_guard_active"),
            Some(TelemetryValue::Bool(false))
        );
        assert_eq!(old.metrics, new_metrics);
    }
}

#[test]
fn explicit_waypoint_retention_matches_the_transfer_setup_and_survives_reset() {
    let config = crate::TransferPdgControllerConfig::default().terminal;
    let mut maintained = TerminalPdgController::new(config.clone());
    maintained.set_guidance_plan_retention_enabled(true);
    let mut explicit = TerminalPdgController::new(config).with_waypoint_guidance_plan_retention();
    assert_eq!(format!("{maintained:?}"), format!("{explicit:?}"));
    explicit.reset_state();
    maintained.reset_state();
    assert_eq!(format!("{maintained:?}"), format!("{explicit:?}"));
    assert!(explicit.guidance_plan_retention_enabled);
    assert!(!TerminalPdgController::default().guidance_plan_retention_enabled);
    assert!(!explicit.ballistic_landing_duration_fallback_enabled);
    assert!(!explicit.ballistic_landing_countdown_enabled);
}

#[test]
fn ballistic_landing_duration_is_bounded_and_not_clamped() {
    let controller = TerminalPdgController::default();
    assert_eq!(
        controller.ballistic_landing_duration_s(-100.0, -25.0, -10.0),
        Some(5.0)
    );
    assert_eq!(
        controller.ballistic_landing_duration_s(-60.0, -25.0, -10.0),
        Some(3.0)
    );
    assert_eq!(
        controller.ballistic_landing_duration_s(-280.0, -25.0, -10.0),
        Some(14.0)
    );
    for (dy, vy, target_vy) in [
        (-59.0, -25.0, -10.0),
        (-281.0, -25.0, -10.0),
        (0.0, 0.0, 0.0),
        (-100.0, 0.0, 0.0),
        (-100.0, 25.0, 10.0),
        (f64::NAN, -25.0, -10.0),
        (-100.0, f64::INFINITY, -10.0),
        (-100.0, -25.0, f64::NEG_INFINITY),
        (f64::MAX, f64::MAX, 0.0),
    ] {
        assert_eq!(
            controller.ballistic_landing_duration_s(dy, vy, target_vy),
            None
        );
    }
}

#[test]
fn ballistic_landing_duration_opt_in_survives_state_reset() {
    assert!(!TerminalPdgController::default().ballistic_landing_duration_fallback_enabled);
    let mut controller =
        TerminalPdgController::default().with_ballistic_landing_duration_fallback();
    controller.reset_state();
    assert!(controller.ballistic_landing_duration_fallback_enabled);
}

fn ballistic_duration_query_fixture() -> (RunContext, Observation) {
    let scenario: pd_core::ScenarioSpec = serde_json::from_str(include_str!(
        "../../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap();
    let ctx = RunContext::from_scenario(&scenario).unwrap();
    let mut state = pd_core::SimulationState::new(&ctx).unwrap();
    // Query-only synthetic descending state; not an airborne flight frontdoor.
    state.position_m = pd_core::Vec2::new(-66.0, 100.0);
    state.velocity_mps = pd_core::Vec2::new(34.0, -32.6);
    state.fuel_kg = 5485.0;
    let observation = state.build_observation(&ctx);
    (ctx, observation)
}

#[test]
fn ballistic_duration_is_shared_by_ignore_adapter_and_live_controller() {
    let (ctx, observation) = ballistic_duration_query_fixture();
    let baseline = TerminalPdgController::default();
    let request = TerminalEntryRequest {
        lateral_dx_m: -6.6,
        ready_ticks: 0,
        terrain_policy: TerminalEntryTerrainPolicy::Ignore,
    };
    let old_gate = baseline.assess_terminal_entry(&ctx, &observation, request);
    assert_eq!(old_gate.mode, TerminalEntryMode::LatestSafe);
    assert!(old_gate.required_accel_ratio > 1.0);
    let mut experimental =
        TerminalPdgController::default().with_ballistic_landing_duration_fallback();
    let gate = experimental.assess_terminal_entry(&ctx, &observation, request);
    assert!(gate.is_ready() && gate.required_accel_ratio <= 1.0);
    let mut ticks = 0;
    assert!(experimental.ballistic_entry_ready(&ctx, &observation, -6.6, &mut ticks));
    let live = experimental.compute_command_state(&ControllerView::new(&ctx, &observation));
    assert_eq!(live.mode, GuidanceMode::LatestSafe);
    assert!(live.candidate.ready);
    assert_eq!(live.candidate.burn_time_s, gate.burn_time_s);
    assert_eq!(
        live.candidate.required_accel_ratio,
        gate.required_accel_ratio
    );
}

#[test]
fn ballistic_duration_keeps_existing_ready_selection_and_commands() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    observation.velocity_mps = pd_core::Vec2::new(4.0, -10.0);
    let mut baseline = TerminalPdgController::default();
    let mut experimental =
        TerminalPdgController::default().with_ballistic_landing_duration_fallback();
    for _ in 0..3 {
        let old = baseline.update(&ctx, &observation);
        let new = experimental.update(&ctx, &observation);
        assert_eq!(
            serde_json::to_value(old).unwrap(),
            serde_json::to_value(new).unwrap()
        );
    }
}

#[test]
fn ballistic_duration_does_not_admit_a_full_vector_or_tilt_failure() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    observation.velocity_mps.x = 150.0;
    let baseline = TerminalPdgController::default();
    let experimental = TerminalPdgController::default().with_ballistic_landing_duration_fallback();
    let request = TerminalEntryRequest {
        lateral_dx_m: -300.0,
        ready_ticks: 0,
        terrain_policy: TerminalEntryTerrainPolicy::Ignore,
    };
    let old = baseline.assess_terminal_entry(&ctx, &observation, request);
    let new = experimental.assess_terminal_entry(&ctx, &observation, request);
    assert_eq!(old.burn_time_s, new.burn_time_s);
    assert_eq!(old.required_accel_ratio, new.required_accel_ratio);
}

#[test]
fn ballistic_duration_keeps_configured_terrain_veto() {
    let (mut ctx, observation) = ballistic_duration_query_fixture();
    // Query-only obstruction: no physical flight or artificial launch exemption.
    ctx.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![
            pd_core::Vec2::new(-1000.0, 200.0),
            pd_core::Vec2::new(1000.0, 200.0),
        ],
    };
    let baseline = TerminalPdgController::default();
    let experimental = TerminalPdgController::default().with_ballistic_landing_duration_fallback();
    let request = TerminalEntryRequest {
        lateral_dx_m: -6.6,
        ready_ticks: 0,
        terrain_policy: TerminalEntryTerrainPolicy::Configured,
    };
    let old = baseline.assess_terminal_entry(&ctx, &observation, request);
    let new = experimental.assess_terminal_entry(&ctx, &observation, request);
    assert!(!new.terrain_clearance_safe);
    assert_eq!(old, new);
}

#[test]
fn ballistic_countdown_keeps_arrival_and_counts_down_after_a_fallback() {
    let (ctx, observation) = ballistic_duration_query_fixture();
    let mut controller = TerminalPdgController::default().with_ballistic_landing_countdown();
    let first = controller.compute_command_state(&ControllerView::new(&ctx, &observation));
    let arrival = first
        .plan_arrival_time_s
        .expect("fallback admits a countdown");
    assert_eq!(
        arrival,
        observation.sim_time_s + first.candidate.burn_time_s
    );
    let mut state = pd_core::SimulationState::new(&ctx).unwrap();
    state.position_m = observation.position_m;
    state.velocity_mps = observation.velocity_mps;
    state.fuel_kg = observation.fuel_kg;
    state.set_command(pd_core::Command {
        throttle_frac: first.throttle_frac,
        target_attitude_rad: first.target_attitude_rad,
    });
    for _ in 0..2 {
        state.step_with_contact_report(&ctx);
    }
    let second = controller
        .compute_command_state(&ControllerView::new(&ctx, &state.build_observation(&ctx)));
    assert_eq!(second.plan_arrival_time_s, Some(arrival));
    assert_eq!(second.candidate.burn_time_s, arrival - state.sim_time_s);
    assert!(second.candidate.burn_time_s < first.candidate.burn_time_s);
    assert!(second.candidate.ready);
    assert_eq!(controller.guidance_replan_count, 0);
}

#[test]
fn ballistic_countdown_keeps_original_ready_choices_and_entry_assessment() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    let request = TerminalEntryRequest {
        lateral_dx_m: -6.6,
        ready_ticks: 0,
        terrain_policy: TerminalEntryTerrainPolicy::Ignore,
    };
    let mut baseline = TerminalPdgController::default().with_ballistic_landing_duration_fallback();
    let mut countdown = TerminalPdgController::default().with_ballistic_landing_countdown();
    assert_eq!(
        baseline.assess_terminal_entry(&ctx, &observation, request),
        countdown.assess_terminal_entry(&ctx, &observation, request)
    );
    observation.velocity_mps = pd_core::Vec2::new(4.0, -10.0);
    for _ in 0..3 {
        assert_eq!(
            baseline.update(&ctx, &observation),
            countdown.update(&ctx, &observation)
        );
    }
    assert!(!countdown.guidance_plan_admitted);
}

#[test]
fn ballistic_countdown_releases_infeasible_fit_once_and_reset_allows_new_mission() {
    let (ctx, observation) = ballistic_duration_query_fixture();
    let mut controller = TerminalPdgController::default().with_ballistic_landing_countdown();
    assert!(
        controller
            .compute_command_state(&ControllerView::new(&ctx, &observation))
            .plan_arrival_time_s
            .is_some()
    );
    let mut invalid = observation.clone();
    invalid.velocity_mps.x = 150.0;
    let released = controller.compute_command_state(&ControllerView::new(&ctx, &invalid));
    assert_eq!(
        released.plan_release_reason,
        Some(GuidancePlanReleaseReason::BallisticCountdownInfeasible)
    );
    assert!(released.plan_arrival_time_s.is_none());
    assert!(controller.guidance_plan_completed);
    let resumed = controller.compute_command_state(&ControllerView::new(&ctx, &observation));
    assert!(resumed.plan_arrival_time_s.is_none());
    controller.reset_state();
    assert!(controller.ballistic_landing_countdown_enabled);
    assert!(
        controller
            .compute_command_state(&ControllerView::new(&ctx, &observation))
            .plan_arrival_time_s
            .is_some()
    );
}

#[test]
fn ballistic_countdown_releases_at_expiry_without_extending_deadline() {
    let (ctx, mut observation) = ballistic_duration_query_fixture();
    let mut controller = TerminalPdgController::default().with_ballistic_landing_countdown();
    let first = controller.compute_command_state(&ControllerView::new(&ctx, &observation));
    observation.sim_time_s = first.plan_arrival_time_s.unwrap();
    let expired = controller.compute_command_state(&ControllerView::new(&ctx, &observation));
    assert_eq!(
        expired.plan_release_reason,
        Some(GuidancePlanReleaseReason::BallisticCountdownExpired)
    );
    assert_eq!(expired.plan_arrival_time_s, None);
    assert_eq!(controller.guidance_replan_count, 0);
    assert!(
        controller
            .compute_command_state(&ControllerView::new(&ctx, &observation))
            .plan_arrival_time_s
            .is_none()
    );
}

#[test]
fn ballistic_countdown_revalidates_configured_terrain() {
    let (mut ctx, observation) = ballistic_duration_query_fixture();
    let mut controller = TerminalPdgController::default().with_ballistic_landing_countdown();
    assert!(
        controller
            .compute_command_state(&ControllerView::new(&ctx, &observation))
            .plan_arrival_time_s
            .is_some()
    );
    ctx.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![
            pd_core::Vec2::new(-1000.0, 200.0),
            pd_core::Vec2::new(1000.0, 200.0),
        ],
    };
    let invalid = controller.compute_command_state(&ControllerView::new(&ctx, &observation));
    assert_eq!(
        invalid.plan_release_reason,
        Some(GuidancePlanReleaseReason::BallisticCountdownInfeasible)
    );
    assert_eq!(invalid.plan_arrival_time_s, None);
    assert!(!invalid.candidate.terrain_clearance_safe);
}

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

#[test]
fn touchdown_rescue_preserves_safe_targetward_closing_outside_pad() {
    for side in [-1.0, 1.0] {
        let target =
            touchdown_rescue_lateral_target(24.0 * side, 0.8 * side, false, 1.85, 1.85, 3.0, 0.55);
        assert_eq!(target.sign, side);
        assert!((target.weight - 0.35).abs() < 1e-9);
    }
}

#[test]
fn touchdown_rescue_brakes_excessive_targetward_closing_outside_pad() {
    for side in [-1.0, 1.0] {
        let target =
            touchdown_rescue_lateral_target(24.0 * side, 3.0 * side, false, 1.85, 1.85, 3.0, 0.55);
        assert_eq!(target.sign, -side);
        assert!((target.weight - (1.15 / 3.0)).abs() < 1e-9);
    }
}

#[test]
fn touchdown_rescue_reverses_motion_away_from_pad() {
    for side in [-1.0, 1.0] {
        let target =
            touchdown_rescue_lateral_target(24.0 * side, -0.8 * side, false, 1.85, 1.85, 3.0, 0.55);
        assert_eq!(target.sign, side);
        assert!((target.weight - (2.65 / 3.0)).abs() < 1e-9);
    }
}

#[test]
fn touchdown_rescue_brakes_meaningful_motion_inside_pad() {
    for side in [-1.0, 1.0] {
        let braking =
            touchdown_rescue_lateral_target(10.0 * side, 2.4 * side, true, 1.85, 1.85, 3.0, 0.55);
        assert_eq!(braking.sign, -side);
        assert!(braking.weight > 0.0);

        let holding =
            touchdown_rescue_lateral_target(10.0 * side, 0.4 * side, true, 1.85, 1.85, 3.0, 0.55);
        assert_eq!(holding, TouchdownRescueLateralTarget::default());
    }
}

#[test]
fn command_throttle_conversion_inverts_minimum_throttle_mapping() {
    let min_throttle_frac = 0.25;
    let applied_throttle_frac = 0.60;

    let command = command_throttle_for_applied_throttle(applied_throttle_frac, min_throttle_frac);
    let reconstructed_applied = min_throttle_frac + command * (1.0 - min_throttle_frac);

    assert!((reconstructed_applied - applied_throttle_frac).abs() < 1e-9);
}

#[test]
fn braking_acceleration_uses_velocity_squared_energy_difference() {
    let braking_accel_mps2 = required_braking_accel_mps2(4.0, 2.0, 6.0);

    assert!((braking_accel_mps2 - 1.0).abs() < 1e-9);
    assert_eq!(required_braking_accel_mps2(1.0, 2.0, 6.0), 0.0);
}

#[test]
fn vertical_authority_caps_rescue_tilt_to_preserve_required_lift() {
    let limit = vertical_authority_tilt_limit_rad(10.0, 12.0, 0.8);

    assert!((limit - (10.0_f64 / 12.0).acos()).abs() < 1e-12);
}

#[test]
fn vertical_authority_keeps_configured_tilt_when_lift_margin_is_available() {
    let limit = vertical_authority_tilt_limit_rad(8.0, 20.0, 0.6);

    assert_eq!(limit, 0.6);
}

#[test]
fn vertical_authority_requires_upright_thrust_when_fully_saturated() {
    assert_eq!(vertical_authority_tilt_limit_rad(12.0, 12.0, 0.8), 0.0);
    assert_eq!(vertical_authority_tilt_limit_rad(14.0, 12.0, 0.8), 0.0);
}
