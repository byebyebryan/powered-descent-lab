use super::*;

#[test]
fn combined_safety_is_explicit_and_only_composes_the_two_existing_changes() {
    let mode = WaypointExperiment::AcquisitionTerminalSafety;
    let parent = WaypointExperiment::AcquisitionGate;
    assert!(mode.open_destination_acquisition() && mode.terminal_safety_fallback());
    assert!(mode.phase_queries() && mode.finite_correction() && mode.queued_recovery());
    assert!(mode.coast_transition() && mode.terminal_takeover() && mode.pad_clearance());
    assert_eq!(mode.effort(), parent.effort());
    assert_eq!(mode.recovery(), parent.recovery());
    assert_eq!(mode.local_height(), parent.local_height());
    assert_eq!(mode.early_target(), parent.early_target());
    assert_eq!(mode.terminal_coordination(), parent.terminal_coordination());
    assert_eq!(WaypointExperiment::default(), WaypointExperiment::Ridge);
    let (request, ctx, state) = acquisition_gate_input();
    let (combined, combined_preview) =
        finite_correction::preview_for(&request, &ctx, &state, 120, 10800, mode).unwrap();
    let (acquisition, acquisition_preview) =
        finite_correction::preview_for(&request, &ctx, &state, 120, 10800, parent).unwrap();
    assert_eq!(combined, acquisition);
    assert_eq!(
        combined_preview.unwrap().goal,
        acquisition_preview.unwrap().goal
    );
}

#[test]
fn acquisition_and_terminal_ablations_are_independent_recovery_children() {
    let parent = WaypointExperiment::RecoveryConsistency;
    for mode in [
        WaypointExperiment::AcquisitionGate,
        WaypointExperiment::TerminalSafetyFallback,
    ] {
        assert!(mode.phase_queries() && mode.finite_correction());
        assert!(mode.coast_transition() && mode.terminal_takeover() && mode.pad_clearance());
        assert!(mode.queued_recovery() && mode.recovery_consistency());
        assert_eq!(mode.effort(), parent.effort());
        assert_eq!(mode.recovery(), parent.recovery());
        assert_eq!(mode.local_height(), parent.local_height());
        assert_eq!(mode.early_target(), parent.early_target());
        assert!(!mode.recovery_lead() && !mode.piecewise_early_target());
        assert_ne!(mode.candidate_id(), parent.candidate_id());
    }
    assert!(WaypointExperiment::AcquisitionGate.open_destination_acquisition());
    assert!(!WaypointExperiment::AcquisitionGate.terminal_safety_fallback());
    assert!(WaypointExperiment::TerminalSafetyFallback.terminal_safety_fallback());
    assert!(!WaypointExperiment::TerminalSafetyFallback.open_destination_acquisition());
    assert!(!parent.open_destination_acquisition() && !parent.terminal_safety_fallback());
    assert_eq!(WaypointExperiment::default(), WaypointExperiment::Ridge);
}

fn acquisition_gate_input() -> (
    WaypointDirectNominalDirectGenerationRequest,
    RunContext,
    SimulationState,
) {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-1100.0, 0.0), Vec2::new(1500.0, 0.0)],
    };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(ctx.target_pad.center_x_m - 588.0, 242.0);
    state.velocity_mps = Vec2::new(73.5, 23.0);
    state.fuel_kg = 5600.0;
    (request, ctx, state)
}

#[test]
fn acquisition_gate_queries_positive_room_with_unchanged_native_guards() {
    let (request, ctx, state) = acquisition_gate_input();
    let before = SimulationStateSnapshotV1::from_state(&state);
    let (old, preview) = finite_correction::preview_for(
        &request,
        &ctx,
        &state,
        120,
        10800,
        WaypointExperiment::RecoveryConsistency,
    )
    .unwrap();
    assert!(old.waypoint_room.as_ref().unwrap().remaining_room_m > 0.0);
    assert!(old.acquisition.is_none() && preview.is_none());
    let (query, preview) = finite_correction::preview_for(
        &request,
        &ctx,
        &state,
        120,
        10800,
        WaypointExperiment::AcquisitionGate,
    )
    .unwrap();
    assert_eq!(query.waypoint_room, old.waypoint_room);
    let audit = query.acquisition.unwrap();
    assert!(audit.accepted(), "{audit:?}");
    assert!(audit.coast_settling.as_ref().unwrap().accepted());
    assert!(preview.unwrap().goal.destination);
    assert_eq!(before, SimulationStateSnapshotV1::from_state(&state));
}

#[test]
fn acquisition_gate_declines_powered_obstruction_without_advancing_state() {
    let (request, mut ctx, state) = acquisition_gate_input();
    ctx.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![
            Vec2::new(-1100.0, 0.0),
            Vec2::new(state.position_m.x + 50.0, 1000.0),
            Vec2::new(1500.0, 0.0),
        ],
    };
    let before = SimulationStateSnapshotV1::from_state(&state);
    let (query, preview) = finite_correction::preview_for(
        &request,
        &ctx,
        &state,
        120,
        10800,
        WaypointExperiment::AcquisitionGate,
    )
    .unwrap();
    assert!(preview.is_none());
    assert_eq!(query.rejection.as_deref(), Some("powered_short_guard"));
    assert!(query.acquisition.unwrap().conflict.is_some());
    assert_eq!(before, SimulationStateSnapshotV1::from_state(&state));
}

#[test]
fn terminal_safety_fallback_avoids_reserve_losing_tilt_without_changing_state() {
    let (request, ctx, mut state) = acquisition_gate_input();
    state.position_m = Vec2::new(
        ctx.target_pad.center_x_m + 14.0,
        ctx.target_pad.surface_y_m + 10.06,
    );
    state.velocity_mps = Vec2::new(-1.85, 0.06);
    let before = SimulationStateSnapshotV1::from_state(&state);
    let original = Command {
        throttle_frac: 0.23,
        target_attitude_rad: 0.148,
    };
    let mut frame = pd_control::ControllerFrame::command_only(original);
    assert!(
        short_conflict(&request, &ctx, &state, original, true)
            .unwrap()
            .is_some()
    );
    phase_transition::protect_pad(&request, &ctx, &state, &mut frame).unwrap();
    assert_eq!(frame.command, original);
    phase_transition::terminal_safety_fallback(&request, &ctx, &state, &mut frame).unwrap();
    assert_eq!(frame.command, Command::default());
    assert_eq!(
        frame.metrics["guidance.terminal_safety_fallback"],
        "upright_coast".into()
    );
    assert!(
        short_conflict(&request, &ctx, &state, frame.command, true)
            .unwrap()
            .is_none()
    );
    assert_eq!(before, SimulationStateSnapshotV1::from_state(&state));
}

#[test]
fn terminal_safety_fallback_preserves_clear_commands_and_unrecoverable_stops() {
    let (request, ctx, mut state) = acquisition_gate_input();
    let original = Command {
        throttle_frac: 0.5,
        target_attitude_rad: 0.1,
    };
    let mut frame = pd_control::ControllerFrame::command_only(original);
    phase_transition::terminal_safety_fallback(&request, &ctx, &state, &mut frame).unwrap();
    assert_eq!(frame.command, original);
    assert!(
        !frame
            .metrics
            .contains_key("guidance.terminal_safety_fallback")
    );
    state.position_m = Vec2::new(ctx.target_pad.center_x_m + ctx.target_pad.width_m, 10.1);
    state.velocity_mps = Vec2::new(0.0, -100.0);
    let before = SimulationStateSnapshotV1::from_state(&state);
    let mut frame = pd_control::ControllerFrame::command_only(original);
    phase_transition::terminal_safety_fallback(&request, &ctx, &state, &mut frame).unwrap();
    assert_eq!(frame.command, original);
    assert!(
        !frame
            .metrics
            .contains_key("guidance.terminal_safety_fallback")
    );
    assert_eq!(before, SimulationStateSnapshotV1::from_state(&state));
}

#[test]
fn terminal_safety_fallback_uses_support_only_after_coast_fails() {
    let (request, ctx, mut state) = acquisition_gate_input();
    state.position_m = Vec2::new(ctx.target_pad.center_x_m + ctx.target_pad.width_m, 10.3);
    state.velocity_mps = Vec2::new(-2.0, -2.0);
    let before = SimulationStateSnapshotV1::from_state(&state);
    let mut frame = pd_control::ControllerFrame::command_only(Command::default());
    phase_transition::terminal_safety_fallback(&request, &ctx, &state, &mut frame).unwrap();
    assert_eq!(frame.command.throttle_frac, 1.0);
    assert_eq!(frame.command.target_attitude_rad, 0.0);
    assert_eq!(
        frame.metrics["guidance.terminal_safety_fallback"],
        "upright_support".into()
    );
    assert!(
        short_conflict(&request, &ctx, &state, frame.command, true)
            .unwrap()
            .is_none()
    );
    assert_eq!(before, SimulationStateSnapshotV1::from_state(&state));
}

#[test]
fn recovery_consistency_is_opt_in_and_keeps_transition_parent() {
    for mode in [
        WaypointExperiment::RecoveryConsistencyProbe,
        WaypointExperiment::RecoveryConsistency,
    ] {
        assert!(mode.phase_queries() && mode.finite_correction());
        assert!(mode.coast_transition() && mode.terminal_takeover() && mode.pad_clearance());
        assert!(!mode.recovery_lead() && !mode.piecewise_early_target());
        assert!(mode.recovery_consistency());
    }
    assert!(!WaypointExperiment::RecoveryConsistencyProbe.queued_recovery());
    assert!(WaypointExperiment::RecoveryConsistency.queued_recovery());
    assert!(!WaypointExperiment::PhaseTransitions.recovery_consistency());
    assert!(!WaypointExperiment::PhaseTransitions.queued_recovery());
    assert_eq!(WaypointExperiment::default(), WaypointExperiment::Ridge);
}

#[test]
fn bounded_queued_warning_does_not_borrow_extra_lookahead() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-1100.0, 0.0), Vec2::new(1500.0, 0.0)],
    };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(500.0, 100.0);
    state.velocity_mps = Vec2::new(0.0, 0.0);
    state.physics_step = 100;
    state.sim_time_s = 100.0 * ctx.sim.physics_dt_s();
    let ticks = avoidance::warning_ticks(&ctx, &state, Command::default());
    let (clearance, _, floor) = body_clearance(&request, &ctx, &state, false).unwrap();
    let n = (ticks + 2) as f64;
    state.position_m.y += floor - clearance
        + 0.5 * ctx.world.gravity_mps2 * ctx.sim.physics_dt_s().powi(2) * n * (n + 1.0);
    let before = SimulationStateSnapshotV1::from_state(&state);
    let old =
        phase_transition::recovery_comparison(&request, &ctx, &state, Command::default(), None)
            .unwrap();
    assert!(old.queued_program.conflict.unwrap().state.physics_step > state.physics_step + ticks);
    let bounded = phase_transition::bounded_recovery_comparison(
        &request,
        &ctx,
        &state,
        Command::default(),
        None,
    )
    .unwrap();
    assert!(bounded.queued_program.accepted());
    assert_eq!(bounded.queued_program.checked_ticks, ticks);
    assert!(bounded.commands.iter().all(|q| q.prediction_ticks == ticks));
    assert_eq!(before, SimulationStateSnapshotV1::from_state(&state));
}

#[test]
fn common_selector_skips_short_safe_support_and_preserves_choice_order() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-1100.0, 0.0), Vec2::new(1500.0, 0.0)],
    };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(500.0, 500.0);
    let mut comparison = phase_transition::bounded_recovery_comparison(
        &request,
        &ctx,
        &state,
        Command::default(),
        None,
    )
    .unwrap();
    // Selector consumes already-native comparison evidence, never a new goal.
    comparison.commands[0].conflict = Some(PredictedConflict {
        state: comparison.origin.clone(),
        cause: "test_common_horizon_conflict".into(),
    });
    let chosen_index = comparison
        .commands
        .iter()
        .position(|q| q.conflict.is_none())
        .unwrap();
    let (selected, query) = avoidance::select_common(&comparison, 100);
    let selected = selected.unwrap();
    assert_eq!(
        selected.selected_command,
        comparison.commands[chosen_index].command
    );
    assert_eq!(selected.prediction_ticks, comparison.prediction_ticks);
    assert_eq!(selected.episode_start_physics_step, 100);
    assert_eq!(query.commands.len(), chosen_index + 1);
    for q in &mut comparison.commands {
        q.conflict = Some(PredictedConflict {
            state: comparison.origin.clone(),
            cause: "test_blocked".into(),
        });
    }
    assert!(avoidance::select_common(&comparison, 100).0.is_none());
}

#[test]
fn bounded_queued_query_reproduces_the_known_powered_cutoff() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-1100.0, 0.0), Vec2::new(1500.0, 0.0)],
    };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(500.0, 500.0);
    state.physics_step = 100;
    state.sim_time_s = 100.0 * ctx.sim.physics_dt_s();
    let plan = Correction {
        arrival_physics_step: 1100,
        turn_end_physics_step: 100,
        burn_end_physics_step: 104,
        thrust_acceleration_mps2: Vec2::new(
            0.0,
            0.9 * ctx.vehicle.max_thrust_n / state.mass_kg(&ctx),
        ),
        target_attitude_rad: 0.0,
        predicted_cutoff: kinematics(&state),
    };
    let before = SimulationStateSnapshotV1::from_state(&state);
    let requested = correction_command(&ctx, &state, &plan).unwrap();
    let comparison = phase_transition::bounded_recovery_comparison(
        &request,
        &ctx,
        &state,
        requested,
        Some(&plan),
    )
    .unwrap();
    assert!(comparison.queued_program.accepted());
    let mut expected = state.clone();
    while expected.physics_step < state.physics_step + comparison.prediction_ticks {
        let command = if expected.physics_step < plan.burn_end_physics_step {
            correction_command(&ctx, &expected, &plan).unwrap()
        } else {
            Command::default()
        };
        expected.set_command(command);
        for _ in 0..2 {
            expected.step_with_contact_report(&ctx);
        }
    }
    assert_eq!(
        comparison.queued_program.end_state,
        Some(SimulationStateSnapshotV1::from_state(&expected))
    );
    assert_eq!(before, SimulationStateSnapshotV1::from_state(&state));
}

#[test]
fn short_waypoint_profile_declines_a_turn_longer_than_its_arrival_clock() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(500.0, 100.0);
    state.velocity_mps = Vec2::new(1.0, 0.0);
    state.attitude_rad = std::f64::consts::PI;
    let goal = Goal {
        position_m: Vec2::new(500.1, 100.0),
        destination: false,
        number: 1,
        revision: 1,
    };
    if let Some((_, correction)) = construct_for(
        &ctx,
        &state,
        &goal,
        9600,
        WaypointExperiment::PhaseTransitions,
    ) {
        assert!(correction.turn_end_physics_step < correction.burn_end_physics_step);
        assert!(correction.burn_end_physics_step < correction.arrival_physics_step);
    }
}

#[test]
fn phase_modes_are_separate_active_finite_children() {
    for mode in [
        WaypointExperiment::TransitionProbe,
        WaypointExperiment::CoastTransition,
        WaypointExperiment::TerminalTakeover,
        WaypointExperiment::PadClearance,
        WaypointExperiment::PhaseTransitions,
    ] {
        assert!(mode.finite_correction() && mode.finite_queries() && mode.exit_consistency());
        assert!(!mode.recovery_lead() && !mode.piecewise_early_target());
        assert!(mode.phase_queries());
    }
    assert!(!WaypointExperiment::TransitionProbe.coast_transition());
    assert!(!WaypointExperiment::TransitionProbe.terminal_takeover());
    assert!(!WaypointExperiment::TransitionProbe.pad_clearance());
    assert!(WaypointExperiment::CoastTransition.coast_transition());
    assert!(!WaypointExperiment::CoastTransition.terminal_takeover());
    assert!(!WaypointExperiment::TerminalTakeover.coast_transition());
    assert!(WaypointExperiment::TerminalTakeover.terminal_takeover());
    assert!(WaypointExperiment::PadClearance.pad_clearance());
    assert!(!WaypointExperiment::FiniteCorrection.phase_queries());
}

#[test]
fn rotation_and_common_horizon_queries_leave_actual_state_unchanged() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-1100.0, 0.0), Vec2::new(1500.0, 0.0)],
    };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(500.0, 14.0);
    state.velocity_mps = Vec2::new(0.0, -2.0);
    state.attitude_rad = -std::f64::consts::FRAC_PI_2;
    state.physics_step = 100;
    state.sim_time_s = 100.0 * ctx.sim.physics_dt_s();
    let before = SimulationStateSnapshotV1::from_state(&state);
    assert!(
        short_conflict(&request, &ctx, &state, Command::default(), false)
            .unwrap()
            .is_none()
    );
    let coast = phase_transition::coast_settling(&request, &ctx, &state).unwrap();
    assert!(coast.prediction_ticks > REFRESH_TICKS && !coast.accepted());
    let comparison =
        phase_transition::recovery_comparison(&request, &ctx, &state, Command::default(), None)
            .unwrap();
    assert!(comparison.commands.len() >= 3);
    assert!(
        comparison
            .commands
            .iter()
            .all(|q| q.checked && q.prediction_ticks == comparison.prediction_ticks)
    );
    assert_eq!(before, SimulationStateSnapshotV1::from_state(&state));
    // The legacy query wrapper remains usable without enabling phase modes.
    let legacy = finite_correction::audit(&request, &ctx, &state, &target(&ctx), None).unwrap();
    assert!(legacy.coast_settling.is_none());
}

#[test]
fn takeover_clones_exact_controller_and_keeps_budget_and_live_state() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(
        ctx.target_pad.center_x_m - 40.0,
        ctx.target_pad.surface_y_m + 120.0,
    );
    state.velocity_mps = Vec2::new(8.0, -10.0);
    state.physics_step = 100;
    state.sim_time_s = 100.0 * ctx.sim.physics_dt_s();
    let before = SimulationStateSnapshotV1::from_state(&state);
    let controller = TerminalPdgController::default()
        .with_ballistic_landing_duration_fallback()
        .with_ballistic_landing_countdown()
        .with_ballistic_landing_braking_guard()
        .with_ballistic_landing_body_centering()
        .with_ballistic_terminal_coordination();
    let mut expected = controller.clone();
    let frame = expected.update(&ctx, &state.build_observation(&ctx));
    let query = phase_transition::terminal_takeover(
        &request,
        &ctx,
        &state,
        &controller,
        10000,
        WaypointExperiment::TransitionProbe,
    )
    .unwrap();
    assert_eq!(query.first_frame, Some(frame));
    assert_eq!(before, SimulationStateSnapshotV1::from_state(&state));
    let no_budget = phase_transition::terminal_takeover(
        &request,
        &ctx,
        &state,
        &controller,
        124,
        WaypointExperiment::TerminalTakeover,
    )
    .unwrap();
    assert!(!no_budget.accepted());
    assert_eq!(no_budget.checked_ticks, 0);
    assert_eq!(no_budget.rejection.as_deref(), Some("original_budget"));
}

#[test]
fn pad_adapter_preserves_guard_and_nominal_lateral_acceleration() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-1100.0, 0.0), Vec2::new(1500.0, 0.0)],
    };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(ctx.target_pad.center_x_m + ctx.target_pad.width_m, 10.3);
    state.velocity_mps = Vec2::new(-2.0, -2.0);
    state.physics_step = 100;
    let before = SimulationStateSnapshotV1::from_state(&state);
    let mut frame = pd_control::ControllerFrame::command_only(Command::default());
    assert!(
        short_conflict(&request, &ctx, &state, frame.command, true)
            .unwrap()
            .is_some()
    );
    phase_transition::protect_pad(&request, &ctx, &state, &mut frame).unwrap();
    assert_eq!(frame.command.throttle_frac, 1.0);
    assert_eq!(frame.command.target_attitude_rad, 0.0);
    assert!(
        short_conflict(&request, &ctx, &state, frame.command, true)
            .unwrap()
            .is_none()
    );
    assert_eq!(before, SimulationStateSnapshotV1::from_state(&state));
}

#[test]
fn mechanics_modes_are_independent_and_preserve_parent_options() {
    let parent = WaypointExperiment::TerminalCoordination;
    for mode in [
        WaypointExperiment::ExitConsistency,
        WaypointExperiment::PiecewiseEarlyTarget,
        WaypointExperiment::RecoveryLead,
        WaypointExperiment::MechanicsCombined,
        WaypointExperiment::FiniteCorrectionProbe,
        WaypointExperiment::FiniteCorrection,
    ] {
        assert!(mode.mechanics());
        assert!(mode.terminal_coordination());
        assert_eq!(mode.effort(), parent.effort());
        assert_eq!(mode.recovery(), parent.recovery());
        assert_eq!(mode.local_height(), parent.local_height());
        assert_eq!(mode.early_target(), parent.early_target());
        assert_eq!(mode.landing_duration(), parent.landing_duration());
        assert_eq!(mode.landing_countdown(), parent.landing_countdown());
        assert_eq!(mode.coast_terminal(), parent.coast_terminal());
        assert_eq!(mode.landing_braking_guard(), parent.landing_braking_guard());
        assert_eq!(
            mode.landing_body_centering(),
            parent.landing_body_centering()
        );
        assert_ne!(mode.candidate_id(), parent.candidate_id());
    }
    assert!(WaypointExperiment::ExitConsistency.exit_consistency());
    assert!(!WaypointExperiment::ExitConsistency.piecewise_early_target());
    assert!(!WaypointExperiment::ExitConsistency.recovery_lead());
    assert!(WaypointExperiment::PiecewiseEarlyTarget.piecewise_early_target());
    assert!(!WaypointExperiment::PiecewiseEarlyTarget.exit_consistency());
    assert!(!WaypointExperiment::PiecewiseEarlyTarget.recovery_lead());
    assert!(WaypointExperiment::RecoveryLead.recovery_lead());
    assert!(!WaypointExperiment::RecoveryLead.exit_consistency());
    assert!(!WaypointExperiment::RecoveryLead.piecewise_early_target());
    assert!(!parent.mechanics());
    assert_eq!(WaypointExperiment::default(), WaypointExperiment::Ridge);
}

#[test]
fn finite_probe_and_candidate_are_explicit_exit_consistency_children() {
    for mode in [
        WaypointExperiment::FiniteCorrectionProbe,
        WaypointExperiment::FiniteCorrection,
    ] {
        assert!(mode.finite_queries() && mode.exit_consistency());
        assert!(!mode.piecewise_early_target() && !mode.recovery_lead());
    }
    assert!(!WaypointExperiment::FiniteCorrectionProbe.finite_correction());
    assert!(WaypointExperiment::FiniteCorrection.finite_correction());
    assert!(!WaypointExperiment::ExitConsistency.finite_queries());
}

#[test]
fn finite_destination_query_checks_real_cutoff_without_mutating_the_plant() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-1100.0, 0.0), Vec2::new(1500.0, 0.0)],
    };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(ctx.target_pad.center_x_m - 588.0, 242.0);
    state.velocity_mps = Vec2::new(73.5, 23.0);
    state.fuel_kg = 5600.0;
    let before = SimulationStateSnapshotV1::from_state(&state);
    let (query, preview) = finite_correction::preview(&request, &ctx, &state, 912, 10800).unwrap();
    let audit = query.acquisition.unwrap();
    assert!(audit.accepted(), "{audit:?}");
    let preview = preview.unwrap();
    let correction = preview.correction.unwrap();
    assert_eq!(
        audit.cutoff.as_ref().unwrap().physics_step,
        correction.burn_end_physics_step
    );
    assert_eq!(
        audit.checked_powered_ticks,
        correction.burn_end_physics_step - state.physics_step
    );
    assert!(audit.cutoff.as_ref().unwrap().fuel_kg < state.fuel_kg);
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
    let (query, preview) = finite_correction::preview(&request, &ctx, &state, 120, 10800).unwrap();
    assert!(preview.is_none() && query.acquisition.is_none());
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn finite_destination_query_rejects_a_real_powered_obstruction() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-1100.0, 0.0), Vec2::new(1500.0, 0.0)],
    };
    let mut ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(ctx.target_pad.center_x_m - 588.0, 242.0);
    state.velocity_mps = Vec2::new(73.5, 23.0);
    state.fuel_kg = 5600.0;
    ctx.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![
            Vec2::new(-1100.0, 0.0),
            Vec2::new(state.position_m.x + 50.0, 1000.0),
            Vec2::new(1500.0, 0.0),
        ],
    };
    let before = SimulationStateSnapshotV1::from_state(&state);
    let (query, preview) = finite_correction::preview(&request, &ctx, &state, 912, 10800).unwrap();
    assert!(preview.is_none());
    assert_eq!(query.rejection.as_deref(), Some("powered_short_guard"));
    assert!(query.acquisition.unwrap().conflict.is_some());
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn exit_consistency_checks_an_unrepaired_incoming_clear_waypoint() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-2000.0, 0.0), Vec2::new(2000.0, 0.0)],
    };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m.y = 50.0;
    let before = SimulationStateSnapshotV1::from_state(&state);
    let mut obstruction = before.clone();
    obstruction.position_m.x += 300.0;
    let conflict = PredictedConflict {
        state: obstruction,
        cause: "ideal_arc_reserve".into(),
    };
    let mut records = Vec::new();
    let mut revision = 1;
    let old = local_replan(
        ReplanQuery {
            request: &request,
            ctx: &ctx,
            state: &state,
            deadline: 9600,
            handoff_number: 1,
            experiment: WaypointExperiment::TerminalCoordination,
        },
        &target(&ctx),
        conflict.clone(),
        &mut revision,
        &mut records,
    )
    .unwrap();
    let LocalReplan::Accepted(goal, arc, _) = old else {
        panic!("expected incoming-clear baseline proposal")
    };
    assert!(
        first_conflict(&request, &ctx, &state, &arc)
            .unwrap()
            .is_none()
    );
    assert!(
        clearance::continuation_conflict(&request, &ctx, &state, &goal, &arc)
            .unwrap()
            .is_some()
    );
    assert!(records.last().unwrap().waypoint_height_repair.is_none());
    records.clear();
    let candidate = local_replan(
        ReplanQuery {
            request: &request,
            ctx: &ctx,
            state: &state,
            deadline: 9600,
            handoff_number: 1,
            experiment: WaypointExperiment::ExitConsistency,
        },
        &target(&ctx),
        conflict,
        &mut revision,
        &mut records,
    )
    .unwrap();
    assert_eq!(records[0].decision, "waypoint_proposal_blocked");
    assert_eq!(
        records[0].predicted_conflict.as_ref().unwrap().cause,
        "waypoint_ideal_continuation_reserve"
    );
    if let LocalReplan::Accepted(goal, arc, _) = candidate {
        assert!(
            clearance::continuation_conflict(&request, &ctx, &state, &goal, &arc)
                .unwrap()
                .is_none()
        );
    }
    assert!(records.len() <= MAX_LOCAL_PROPOSALS);
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn recovery_warning_uses_the_existing_command_family_and_records_failures() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m.x += 100.0;
    state.position_m.y += 10.0;
    state.velocity_mps.y = -100.0;
    let before = SimulationStateSnapshotV1::from_state(&state);
    let horizon = avoidance::warning_ticks(&ctx, &state, Command::default());
    assert!(horizon > REFRESH_TICKS && horizon <= CONTINUATION_TICKS);
    let (selected, query) = avoidance::select_with_evidence(
        &request,
        &ctx,
        &state,
        Command::default(),
        state.physics_step,
    )
    .unwrap();
    assert!(selected.is_none());
    assert_eq!(query.origin, before);
    assert!(!query.commands.is_empty() && query.commands.len() <= avoidance::MAX_COMMANDS);
    assert!(
        query
            .commands
            .iter()
            .all(|c| c.checked && c.conflict.is_some())
    );
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn terminal_coordination_keeps_all_parent_planner_options() {
    let parent = WaypointExperiment::LandingBodyCentering;
    let candidate = WaypointExperiment::TerminalCoordination;
    assert_ne!(candidate.candidate_id(), parent.candidate_id());
    assert_eq!(candidate.effort(), parent.effort());
    assert_eq!(candidate.recovery(), parent.recovery());
    assert_eq!(candidate.local_height(), parent.local_height());
    assert_eq!(candidate.early_target(), parent.early_target());
    assert_eq!(candidate.landing_duration(), parent.landing_duration());
    assert_eq!(candidate.landing_countdown(), parent.landing_countdown());
    assert_eq!(candidate.coast_terminal(), parent.coast_terminal());
    assert_eq!(
        candidate.landing_braking_guard(),
        parent.landing_braking_guard()
    );
    assert_eq!(
        candidate.landing_body_centering(),
        parent.landing_body_centering()
    );
}

#[test]
fn prediction_domain_stop_retains_origin_command_and_unexecuted_query() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    let max_x = ctx.world.terrain.points().last().unwrap().x;
    state.position_m = Vec2::new(max_x - 20.0, 2000.0);
    state.velocity_mps.x = 150.0;
    let command = Command::default();
    let error = short_conflict(&request, &ctx, &state, command, false).unwrap_err();
    let diagnostic = error.downcast_ref::<TerrainDomainStop>().unwrap();
    assert_eq!(
        diagnostic.query_origin,
        SimulationStateSnapshotV1::from_state(&state)
    );
    assert_eq!(diagnostic.requested_command, Some(command));
    assert!(diagnostic.query_state.physics_step > state.physics_step);
    assert_eq!(state.physics_step, 0);
    assert!(matches!(
        diagnostic.error,
        TerrainQueryError::DomainOverrun { .. }
    ));
}

#[test]
fn actual_domain_stop_is_not_labeled_as_an_unexecuted_prediction() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut live = new_ordinary(&ctx).unwrap();
    live.state.position_m = Vec2::new(ctx.world.terrain.points().last().unwrap().x + 10.0, 2000.0);
    let output = execute(
        &request,
        live,
        false,
        24,
        WaypointExperiment::TerminalCoordination,
    )
    .unwrap();
    assert_eq!(output.stop, "actual_terrain_domain");
    let diagnostic = output.terrain_domain_stop.unwrap();
    assert_eq!(diagnostic.query, "actual_body_reserve");
    assert_eq!(diagnostic.requested_command, None);
    assert_eq!(diagnostic.actual_state, diagnostic.query_state);
    assert!(output.updates.is_empty());
}

#[test]
fn coast_terminal_is_explicit_and_preserves_all_parent_acquisition_options() {
    let candidate = WaypointExperiment::CoastTerminal;
    let parent = WaypointExperiment::LandingCountdown;
    assert!(candidate.coast_terminal());
    assert_eq!(candidate.effort(), parent.effort());
    assert_eq!(candidate.recovery(), parent.recovery());
    assert_eq!(candidate.local_height(), parent.local_height());
    assert_eq!(candidate.early_target(), parent.early_target());
    assert_eq!(candidate.landing_duration(), parent.landing_duration());
    assert_eq!(candidate.landing_countdown(), parent.landing_countdown());
    assert!(!parent.coast_terminal());
    assert!(!WaypointExperiment::default().coast_terminal());
    assert_eq!(WaypointExperiment::default(), WaypointExperiment::Ridge);
}

#[test]
fn landing_duration_modes_are_explicit_and_keep_existing_defaults() {
    assert_eq!(WaypointExperiment::default(), WaypointExperiment::Ridge);
    for mode in [
        WaypointExperiment::LandingDuration,
        WaypointExperiment::EarlyTargetLandingDuration,
    ] {
        assert!(mode.landing_duration() && mode.effort() && mode.recovery() && mode.early_target());
    }
    assert!(WaypointExperiment::LandingDuration.local_height());
    assert!(!WaypointExperiment::EarlyTargetLandingDuration.local_height());
    for old in [
        WaypointExperiment::Ridge,
        WaypointExperiment::Effort,
        WaypointExperiment::Recovery,
        WaypointExperiment::Combined,
        WaypointExperiment::LocalHeight,
        WaypointExperiment::EarlyTarget,
        WaypointExperiment::LocalHeightEarlyTarget,
    ] {
        assert!(!old.landing_duration());
    }
}

#[test]
fn landing_countdown_is_explicit_and_inherits_only_its_parent_options() {
    for mode in [
        WaypointExperiment::LandingCountdown,
        WaypointExperiment::EarlyTargetLandingCountdown,
    ] {
        assert!(
            mode.landing_countdown()
                && mode.landing_duration()
                && mode.effort()
                && mode.recovery()
                && mode.early_target()
        );
    }
    assert!(WaypointExperiment::LandingCountdown.local_height());
    assert!(!WaypointExperiment::EarlyTargetLandingCountdown.local_height());
    for old in [
        WaypointExperiment::Ridge,
        WaypointExperiment::Combined,
        WaypointExperiment::LocalHeightEarlyTarget,
        WaypointExperiment::LandingDuration,
        WaypointExperiment::EarlyTargetLandingDuration,
    ] {
        assert!(!old.landing_countdown());
    }
}

fn context() -> RunContext {
    RunContext::from_scenario(&crate::test_inputs::planner_request("v2_clear_845").scenario)
        .unwrap()
}

#[test]
fn coast_accepts_already_correct_higher_motion() {
    let ctx = context();
    let goal = target(&ctx);
    let mut s = SimulationState::new(&ctx).unwrap();
    // Query-only synthetic state tests pure admission, not an airborne frontdoor.
    s.position_m = Vec2::new(goal.position_m.x - 500.0, goal.position_m.y + 300.0);
    let arc = aim::target_arc(
        kinematics(&s),
        goal.position_m,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        1800,
    )
    .unwrap();
    s.velocity_mps = arc.departure_velocity_mps;
    assert!(accepted_coast(&ctx, &s, &goal).is_some());
    let later = aim::project(
        kinematics(&s),
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        120,
    );
    s.position_m = later.position_m;
    s.velocity_mps = later.velocity_mps;
    assert!(accepted_coast(&ctx, &s, &goal).is_some());
}

#[test]
fn waypoint_does_not_require_landing_angle_or_zero_velocity() {
    let ctx = context();
    let s = KinematicStateV2 {
        position_m: Vec2::new(20.0, 100.0),
        velocity_mps: Vec2::new(80.0, 30.0),
    };
    let goal = Goal {
        position_m: Vec2::new(120.0, 110.0),
        destination: false,
        number: 1,
        revision: 0,
    };
    assert!(approach_ok(&ctx, s, &goal, 120));
}

#[test]
fn waypoint_acceptance_preserves_rising_and_higher_motion() {
    let ctx = context();
    let goal = Goal {
        position_m: Vec2::new(120.0, 110.0),
        destination: false,
        number: 1,
        revision: 0,
    };
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(20.0, 100.0);
    state.velocity_mps = Vec2::new(80.0, 30.0);
    assert!(accepted_coast(&ctx, &state, &goal).is_some());
    state.position_m.y = 150.0;
    assert!(accepted_coast(&ctx, &state, &goal).is_some());
    state.position_m.y = 20.0;
    assert!(accepted_coast(&ctx, &state, &goal).is_none());
}

#[test]
fn descending_safe_motion_is_preserved_without_rebuilding_an_apex() {
    let ctx = context();
    let goal = target(&ctx);
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(-300.0, 205.0);
    // Select exactly the descending coast's lateral speed to hit the pad.
    state.velocity_mps.y = -5.0;
    let n = aim::natural_arrival_steps(
        kinematics(&state),
        goal.position_m.y,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
    )
    .unwrap();
    state.velocity_mps.x = 300.0 / (n as f64 * ctx.sim.physics_dt_s());
    assert!(accepted_coast(&ctx, &state, &goal).is_some());
}

#[test]
fn aim_constructor_is_terrain_blind_and_declines_empty_fuel() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    let expected = construct(&ctx, &state, &target(&ctx), 9600).unwrap();
    let mut twin = ctx.clone();
    twin.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![
            Vec2::new(-1005.0, 0.0),
            Vec2::new(-700.0, 500.0),
            Vec2::new(-18.0, 0.0),
            Vec2::new(160.0, 0.0),
        ],
    };
    assert_eq!(
        construct(&twin, &state, &target(&twin), 9600).unwrap(),
        expected
    );
    state.fuel_kg = 0.0;
    assert!(construct(&ctx, &state, &target(&ctx), 9600).is_none());
}

#[test]
fn finite_burn_has_a_real_cutoff_command() {
    let ctx = context();
    let mut s = SimulationState::new(&ctx).unwrap();
    let c = Correction {
        arrival_physics_step: 300,
        turn_end_physics_step: 20,
        burn_end_physics_step: 100,
        thrust_acceleration_mps2: Vec2::new(0.0, 12.0),
        target_attitude_rad: 0.0,
        predicted_cutoff: kinematics(&s),
    };
    assert_eq!(correction_command(&ctx, &s, &c).unwrap().throttle_frac, 0.0);
    s.physics_step = 20;
    assert!(correction_command(&ctx, &s, &c).unwrap().throttle_frac > 0.0);
    s.physics_step = 100;
    assert_eq!(correction_command(&ctx, &s, &c).unwrap().throttle_frac, 0.0);
}

#[test]
fn waypoint_effort_prefers_a_cheaper_burn_with_gentler_entry_without_an_apex_rule() {
    let ctx = context();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(0.0, 100.0);
    state.velocity_mps = Vec2::new(1.0, 8.0);
    let goal = Goal {
        position_m: Vec2::new(675.0, 1200.0),
        destination: false,
        number: 1,
        revision: 1,
    };
    let before = SimulationStateSnapshotV1::from_state(&state);
    let (_, old) = construct(&ctx, &state, &goal, 9600).unwrap();
    let (_, cheap) = construct_for(&ctx, &state, &goal, 9600, WaypointExperiment::Effort).unwrap();
    let old_rank = correction_rank(&ctx, state.physics_step, &old);
    let cheap_rank = correction_rank(&ctx, state.physics_step, &cheap);
    assert!(cheap_rank.0 < old_rank.0);
    assert!(cheap_rank.1 < old_rank.1);
    assert!(
        cheap.arrival_physics_step - cheap.burn_end_physics_step
            > old.arrival_physics_step - old.burn_end_physics_step
    );
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
    assert_eq!(
        profile_steps(&ctx, &state, &goal, WaypointExperiment::Combined)
            .unwrap()
            .len(),
        MAX_PROFILE_TRIALS as usize
    );
}

#[test]
fn forward_reacquisition_can_correct_a_rising_waypoint_without_waiting_for_descent() {
    let ctx = context();
    let mut state = SimulationState::new(&ctx).unwrap();
    // Query-only rounded regression of 349's finite construction stop; this
    // never restores a snapshot into execution or claims a recovery flight.
    state.position_m = Vec2::new(562.712522, 715.701399);
    state.velocity_mps = Vec2::new(62.896317, 91.153);
    state.attitude_rad = -1.49415;
    state.fuel_kg = 5553.153746;
    let goal = Goal {
        position_m: Vec2::new(676.806248, 871.015783),
        destination: false,
        number: 1,
        revision: 1,
    };
    let before = SimulationStateSnapshotV1::from_state(&state);
    assert!(construct(&ctx, &state, &goal, 9600).is_none());
    let (_, correction) =
        construct_for(&ctx, &state, &goal, 9600, WaypointExperiment::Recovery).unwrap();
    assert!(correction.arrival_physics_step - state.physics_step < 240);
    let entry = aim::project(
        correction.predicted_cutoff,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        correction.arrival_physics_step - correction.burn_end_physics_step,
    );
    assert!(entry.velocity_mps.y > 0.0);
    assert!(entry.velocity_mps.x > 60.0);
    assert!((entry.position_m - goal.position_m).length() < 1e-8);
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn waypoint_experiments_leave_destination_construction_and_safe_overshoot_unchanged() {
    let ctx = context();
    let state = SimulationState::new(&ctx).unwrap();
    let expected = construct(&ctx, &state, &target(&ctx), 9600).unwrap();
    for experiment in [
        WaypointExperiment::Effort,
        WaypointExperiment::Recovery,
        WaypointExperiment::Combined,
        WaypointExperiment::LocalHeight,
        WaypointExperiment::EarlyTarget,
        WaypointExperiment::LocalHeightEarlyTarget,
    ] {
        assert_eq!(
            construct_for(&ctx, &state, &target(&ctx), 9600, experiment).unwrap(),
            expected
        );
    }
    let mut rising = state.clone();
    rising.position_m = Vec2::new(20.0, 200.0);
    rising.velocity_mps = Vec2::new(80.0, 50.0);
    let goal = Goal {
        position_m: Vec2::new(120.0, 110.0),
        destination: false,
        number: 1,
        revision: 1,
    };
    assert!(accepted_coast(&ctx, &rising, &goal).is_some());
}

#[test]
fn an_unsafe_current_coast_can_have_a_clear_pending_cutoff_coast() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![
            Vec2::new(-1100.0, 0.0),
            Vec2::new(400.0, 0.0),
            Vec2::new(580.0, 740.0),
            Vec2::new(600.0, 700.0),
            Vec2::new(1400.0, 0.0),
        ],
    };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(250.0, 440.0);
    state.velocity_mps = Vec2::new(60.0, 70.0);
    let actual = aim::target_arc(
        kinematics(&state),
        aim::project(kinematics(&state), 9.81, 1.0 / 120.0, 700).position_m,
        9.81,
        1.0 / 120.0,
        700,
    )
    .unwrap();
    assert!(
        first_conflict(&request, &ctx, &state, &actual)
            .unwrap()
            .is_some()
    );
    let correction = Correction {
        arrival_physics_step: 720,
        turn_end_physics_step: 0,
        burn_end_physics_step: 120,
        thrust_acceleration_mps2: Vec2::new(0.0, 12.0),
        target_attitude_rad: 0.0,
        predicted_cutoff: KinematicStateV2 {
            position_m: Vec2::new(300.0, 500.0),
            velocity_mps: Vec2::new(60.0, 80.0),
        },
    };
    let before = SimulationStateSnapshotV1::from_state(&state);
    assert!(pending_coast_clear(&request, &ctx, &state, &correction).unwrap());
    let mut bad = correction.clone();
    bad.predicted_cutoff.velocity_mps.y = 60.0;
    assert!(!pending_coast_clear(&request, &ctx, &state, &bad).unwrap());
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn early_target_preview_uses_negative_entry_room_without_advancing_the_state() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-1100.0, 0.0), Vec2::new(1500.0, 0.0)],
    };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    // Query-only generic version of a fast coast to a waypoint close to the
    // destination. A preview is not an airborne flight or an executed H.
    state.position_m = Vec2::new(ctx.target_pad.center_x_m - 588.0, 242.0);
    state.velocity_mps = Vec2::new(73.5, 23.0);
    state.attitude_rad = 0.0;
    state.fuel_kg = 5600.0;
    let before = SimulationStateSnapshotV1::from_state(&state);
    let preview = early_destination_preview(&request, &ctx, &state, 912, 10800, false)
        .unwrap()
        .unwrap();
    assert!(preview.goal.destination);
    assert!(preview.waypoint_room.unwrap().remaining_room_m < 0.0);
    assert!(preview.correction.is_some());
    assert_eq!(preview.arc.target_m, target(&ctx).position_m);
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
    // A much earlier waypoint leaves ample braking room; do not disturb it.
    assert!(
        early_destination_preview(&request, &ctx, &state, 120, 10800, false)
            .unwrap()
            .is_none()
    );
    let mut blocked = ctx.clone();
    blocked.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![
            Vec2::new(-1100.0, 0.0),
            Vec2::new(state.position_m.x + 150.0, 1000.0),
            Vec2::new(1500.0, 0.0),
        ],
    };
    assert!(
        early_destination_preview(&request, &blocked, &state, 912, 10800, false)
            .unwrap()
            .is_none()
    );
    let piecewise = early_destination_preview(&request, &blocked, &state, 912, 10800, true)
        .unwrap()
        .unwrap();
    assert!(piecewise.goal.destination);
    assert_eq!(piecewise.obstruction.unwrap().cause, "ideal_arc_reserve");
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn an_early_destination_preview_preserves_an_already_safe_destination_coast() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(
        ctx.target_pad.center_x_m - 500.0,
        target(&ctx).position_m.y + 300.0,
    );
    let arc = aim::target_arc(
        kinematics(&state),
        target(&ctx).position_m,
        9.81,
        1.0 / 120.0,
        1200,
    )
    .unwrap();
    state.velocity_mps = arc.departure_velocity_mps;
    state.held_command = Command::default();
    assert!(accepted_coast(&ctx, &state, &target(&ctx)).is_some());
    let before = SimulationStateSnapshotV1::from_state(&state);
    let preview = early_destination_preview(&request, &ctx, &state, 1160, 10800, false)
        .unwrap()
        .unwrap();
    assert!(preview.correction.is_none());
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn ideal_touchdown_endpoint_roundoff_is_not_a_terrain_obstruction() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    let shelf = 23.166584115242756;
    for pad in &mut request.scenario.world.landing_pads {
        pad.surface_y_m = shelf;
    }
    request.scenario.initial_state.position_m.y = shelf + 5.0;
    let source_x = request.scenario.initial_state.position_m.x;
    let target_x = request
        .scenario
        .world
        .landing_pad(&request.target_pad_id)
        .unwrap()
        .center_x_m;
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![
            Vec2::new(source_x - 160.0, shelf),
            Vec2::new(target_x + 160.0, shelf),
        ],
    };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let state = SimulationState::new(&ctx).unwrap();
    let arc = aim::target_arc(
        kinematics(&state),
        target(&ctx).position_m,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        2902,
    )
    .unwrap();
    let conflict = first_conflict(&request, &ctx, &state, &arc).unwrap();
    assert!(conflict.is_none(), "query conflict: {conflict:?}");
}

#[test]
fn paired_coast_crossing_stops_geometric_query_at_the_target_shelf() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m = Vec2::new(-500.0, 200.0);
    let ideal = aim::target_arc(
        kinematics(&state),
        target(&ctx).position_m,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        2001,
    )
    .unwrap();
    state.velocity_mps = ideal.departure_velocity_mps;
    let (n, _) = accepted_coast(&ctx, &state, &target(&ctx)).unwrap();
    let end = aim::project(
        kinematics(&state),
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        n,
    );
    assert!(end.position_m.y < target(&ctx).position_m.y);
    let coast = aim::target_arc(
        kinematics(&state),
        end.position_m,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        n,
    )
    .unwrap();
    assert!(
        first_conflict(&request, &ctx, &state, &coast)
            .unwrap()
            .is_none()
    );
}

#[test]
fn local_replan_replaces_a_waypoint_without_advancing_the_plant() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let state = SimulationState::new(&ctx).unwrap();
    let before = SimulationStateSnapshotV1::from_state(&state);
    let mut old_goal = target(&ctx);
    old_goal.destination = false;
    old_goal.number = 1;
    old_goal.revision = 1;
    old_goal.position_m.y += 50.0;
    // Synthetic obstruction input tests replacement mechanics, not flight coverage.
    let mut conflict_state = before.clone();
    conflict_state.position_m.x += 400.0;
    let conflict = PredictedConflict {
        state: conflict_state,
        cause: "ideal_arc_reserve".into(),
    };
    let mut revision = 2;
    let mut refreshes = vec![];
    let outcome = local_replan(
        ReplanQuery {
            request: &request,
            ctx: &ctx,
            state: &state,
            deadline: 9600,
            handoff_number: 1,
            experiment: WaypointExperiment::Ridge,
        },
        &old_goal,
        conflict,
        &mut revision,
        &mut refreshes,
    )
    .unwrap();
    let LocalReplan::Accepted(goal, arc, _) = outcome else {
        panic!("expected admitted replacement")
    };
    assert_eq!(goal.number, old_goal.number);
    assert_eq!(goal.revision, 2);
    assert!(goal.position_m.x < old_goal.position_m.x);
    assert!(
        first_conflict(&request, &ctx, &state, &arc)
            .unwrap()
            .is_none()
    );
    assert_eq!(refreshes.last().unwrap().decision, "waypoint_replaced");
    assert_eq!(
        refreshes.last().unwrap().previous_goal.as_ref(),
        Some(&old_goal)
    );
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn same_goal_can_be_reacquired_without_inventing_a_replacement_or_handoff() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    let mut conflict_state = SimulationStateSnapshotV1::from_state(&state);
    conflict_state.position_m.x += 400.0;
    let conflict = PredictedConflict {
        state: conflict_state,
        cause: "ideal_arc_reserve".into(),
    };
    let (goal, _) = ridge::waypoint(&ctx, &state, conflict.state.position_m, 1).unwrap();
    let mut revision = 2;
    let mut refreshes = vec![];
    let result = local_replan(
        ReplanQuery {
            request: &request,
            ctx: &ctx,
            state: &state,
            deadline: 9600,
            handoff_number: 1,
            experiment: WaypointExperiment::Ridge,
        },
        &goal,
        conflict.clone(),
        &mut revision,
        &mut refreshes,
    )
    .unwrap();
    let LocalReplan::Accepted(accepted, _, _) = result else {
        panic!("same goal should get one fresh query")
    };
    assert_eq!(accepted.revision, goal.revision);
    assert_eq!(accepted.position_m, goal.position_m);
    assert_eq!(refreshes.len(), 1);
    assert_eq!(refreshes[0].decision, "waypoint_reacquired");
    assert_eq!(revision, 2);
    refreshes.clear();
    state.fuel_kg = 0.0;
    let result = local_replan(
        ReplanQuery {
            request: &request,
            ctx: &ctx,
            state: &state,
            deadline: 9600,
            handoff_number: 1,
            experiment: WaypointExperiment::Ridge,
        },
        &target(&ctx),
        conflict,
        &mut revision,
        &mut refreshes,
    )
    .unwrap();
    assert!(matches!(result, LocalReplan::Stopped(ref s) if s == "waypoint_aim_construction_miss"));
    assert_eq!(refreshes.len(), 1);
    assert_eq!(state.physics_step, 0);
}

#[test]
fn failed_proposals_are_bounded_at_one_unchanged_actual_state() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    // Query-only negative state: it must never produce an executable proposal.
    state.position_m.y -= 10.0;
    let before = SimulationStateSnapshotV1::from_state(&state);
    let conflict = PredictedConflict {
        state: before.clone(),
        cause: "ideal_arc_reserve".into(),
    };
    let mut refreshes = vec![];
    let mut revision = 1;
    let result = local_replan(
        ReplanQuery {
            request: &request,
            ctx: &ctx,
            state: &state,
            deadline: 9600,
            handoff_number: 1,
            experiment: WaypointExperiment::Ridge,
        },
        &target(&ctx),
        conflict,
        &mut revision,
        &mut refreshes,
    )
    .unwrap();
    assert!(matches!(result, LocalReplan::Stopped(_)));
    assert!(refreshes.len() <= MAX_LOCAL_PROPOSALS);
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn waypoint_height_repair_uses_full_body_width_and_keeps_the_plant_unchanged() {
    let (request, ctx, state, goal, arc) = height_repair_query(
        vec![
            Vec2::new(-1005.0, 0.0),
            Vec2::new(-400.0, 0.0),
            Vec2::new(-360.0, 280.0),
            Vec2::new(-300.0, 0.0),
            Vec2::new(160.0, 0.0),
        ],
        Vec2::new(-500.0, 20.0),
        Vec2::new(-365.0, 276.0),
        720,
    );
    let before = SimulationStateSnapshotV1::from_state(&state);
    let repair = clearance::height_repair(&request, &ctx, &state, &goal, &arc)
        .unwrap()
        .unwrap();
    // Terrain at the centre is 245 m, but at the forward body edge it is 273 m.
    assert!(repair.height_increase_m > 7.0);
    assert_eq!(repair.from_goal, goal);
    let mut lifted = goal.clone();
    lifted.position_m.y += repair.height_increase_m;
    let twin = aim::target_arc(
        kinematics(&state),
        lifted.position_m,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        arc.steps,
    )
    .unwrap();
    assert!(
        first_conflict(&request, &ctx, &state, &twin)
            .unwrap()
            .is_none()
    );
    assert!(
        clearance::continuation_conflict(&request, &ctx, &state, &lifted, &twin)
            .unwrap()
            .is_none()
    );
    assert!(
        clearance::height_repair(&request, &ctx, &state, &lifted, &twin)
            .unwrap()
            .is_none()
    );
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

fn height_repair_query(
    points_m: Vec<Vec2>,
    start: Vec2,
    end: Vec2,
    steps: u64,
) -> (
    WaypointDirectNominalDirectGenerationRequest,
    RunContext,
    SimulationState,
    Goal,
    BallisticAim,
) {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield { points_m };
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    // Synthetic geometry query only; never an executable airborne restart.
    state.position_m = start;
    let goal = Goal {
        position_m: end,
        destination: false,
        number: 1,
        revision: 1,
    };
    let arc = aim::target_arc(
        kinematics(&state),
        end,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        steps,
    )
    .unwrap();
    (request, ctx, state, goal, arc)
}

#[test]
fn waypoint_height_repair_clears_incoming_crest_not_just_the_endpoint() {
    let (request, ctx, state, goal, arc) = height_repair_query(
        vec![
            Vec2::new(-1005.0, 0.0),
            Vec2::new(-500.0, 0.0),
            Vec2::new(-320.0, 350.0),
            Vec2::new(-300.0, 100.0),
            Vec2::new(160.0, 0.0),
        ],
        Vec2::new(-500.0, 20.0),
        Vec2::new(-300.0, 340.0),
        720,
    );
    let endpoint = clearance::projected_view(&ctx, &state, &arc, arc.steps);
    assert!(body_safe(&request, &ctx, &endpoint, false).is_ok());
    assert!(
        first_conflict(&request, &ctx, &state, &arc)
            .unwrap()
            .is_some()
    );
    let repair = clearance::height_repair(&request, &ctx, &state, &goal, &arc)
        .unwrap()
        .unwrap();
    assert_eq!(
        repair.limiting_conflict.cause,
        "waypoint_incoming_body_reserve"
    );
    let end = Vec2::new(
        goal.position_m.x,
        goal.position_m.y + repair.height_increase_m,
    );
    let lifted = aim::target_arc(
        kinematics(&state),
        end,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        arc.steps,
    )
    .unwrap();
    assert!(
        first_conflict(&request, &ctx, &state, &lifted)
            .unwrap()
            .is_none()
    );
}

#[test]
fn waypoint_height_repair_accounts_for_uphill_motion_after_the_endpoint() {
    let (request, ctx, state, goal, arc) = height_repair_query(
        vec![
            Vec2::new(-1005.0, 0.0),
            Vec2::new(-300.0, 0.0),
            Vec2::new(-270.0, 200.0),
            Vec2::new(160.0, 200.0),
        ],
        Vec2::new(-500.0, 20.0),
        Vec2::new(-300.0, 40.0),
        1200,
    );
    assert!(
        first_conflict(&request, &ctx, &state, &arc)
            .unwrap()
            .is_none()
    );
    assert!(
        clearance::continuation_conflict(&request, &ctx, &state, &goal, &arc)
            .unwrap()
            .is_some()
    );
    let repair = clearance::height_repair(&request, &ctx, &state, &goal, &arc)
        .unwrap()
        .unwrap();
    assert_eq!(
        repair.limiting_conflict.cause,
        "waypoint_ideal_continuation_reserve"
    );
    assert_eq!(
        repair.checked_through_relative_tick,
        arc.steps + CONTINUATION_TICKS
    );
}

#[test]
fn waypoint_height_repair_reserves_the_existing_handoff_height_window() {
    let (request, ctx, state, goal, arc) = height_repair_query(
        vec![Vec2::new(-1005.0, 0.0), Vec2::new(160.0, 0.0)],
        Vec2::new(-500.0, 20.0),
        Vec2::new(-300.0, 35.0),
        240,
    );
    let end = clearance::projected_view(&ctx, &state, &arc, arc.steps + CONTINUATION_TICKS);
    assert!(body_safe(&request, &ctx, &end, false).is_ok());
    assert!(
        clearance::continuation_conflict(&request, &ctx, &state, &goal, &arc)
            .unwrap()
            .is_some()
    );
    let repair = clearance::height_repair(&request, &ctx, &state, &goal, &arc)
        .unwrap()
        .unwrap();
    assert_eq!(repair.handoff_height_tolerance_m, tolerance(&ctx, &goal));
}

#[test]
fn waypoint_height_repair_propagates_terrain_domain_errors() {
    let (request, ctx, state, goal, arc) = height_repair_query(
        vec![Vec2::new(-1005.0, 0.0), Vec2::new(-295.0, 0.0)],
        Vec2::new(-500.0, 20.0),
        Vec2::new(-300.0, 40.0),
        1200,
    );
    assert!(clearance::height_repair(&request, &ctx, &state, &goal, &arc).is_err());
}

#[test]
fn short_prediction_records_actual_command_drift_without_mutating_state() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m.x += 100.0;
    state.position_m.y = 20.0;
    state.velocity_mps.y = -100.0;
    let before = SimulationStateSnapshotV1::from_state(&state);
    let conflict = short_conflict(&request, &ctx, &state, Command::default(), false)
        .unwrap()
        .unwrap();
    assert!(conflict.state.physics_step > state.physics_step);
    assert!(conflict.state.physics_step <= state.physics_step + REFRESH_TICKS);
    assert_eq!(conflict.cause, "short_command_reserve");
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn terrain_query_errors_are_not_waypoint_obstructions() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let mut ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let state = SimulationState::new(&ctx).unwrap();
    let (arc, _) = construct(&ctx, &state, &target(&ctx), 9600).unwrap();
    ctx.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(2000.0, 0.0), Vec2::new(2001.0, 0.0)],
    };
    assert!(first_conflict(&request, &ctx, &state, &arc).is_err());
    assert!(short_conflict(&request, &ctx, &state, Command::default(), false).is_err());
}

#[test]
fn terrain_protection_can_support_a_turn_without_mutating_the_actual_state() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m.x += 100.0;
    state.physics_step = 2;
    state.sim_time_s = 2.0 * ctx.sim.physics_dt_s();
    state.attitude_rad = 0.15;
    let (clearance, _, floor) = body_clearance(&request, &ctx, &state, false).unwrap();
    state.position_m.y += floor - clearance + 0.001;
    let requested = Command {
        throttle_frac: 0.0,
        target_attitude_rad: -0.2,
    };
    let before = SimulationStateSnapshotV1::from_state(&state);
    assert!(
        short_conflict(&request, &ctx, &state, requested, false)
            .unwrap()
            .is_some()
    );
    let selected = avoidance::select(&request, &ctx, &state, requested, state.physics_step)
        .unwrap()
        .unwrap();
    assert_eq!(selected.selected_choice, "support_requested_turn");
    assert_eq!(selected.selected_command.throttle_frac, 1.0);
    assert_eq!(
        selected.selected_command.target_attitude_rad,
        requested.target_attitude_rad
    );
    assert!(selected.prediction_ticks >= REFRESH_TICKS);
    assert!(
        short_conflict_ticks(
            &request,
            &ctx,
            &state,
            selected.selected_command,
            false,
            selected.prediction_ticks
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(
        avoidance::select(&request, &ctx, &state, requested, state.physics_step).unwrap(),
        Some(selected)
    );
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn terrain_protection_reports_no_command_instead_of_relaxing_clearance() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.position_m.x += 100.0;
    let (clearance, _, floor) = body_clearance(&request, &ctx, &state, false).unwrap();
    state.position_m.y += floor - clearance + 0.01;
    state.velocity_mps.y = -100.0;
    let before = SimulationStateSnapshotV1::from_state(&state);
    assert!(
        avoidance::select(&request, &ctx, &state, Command::default(), 0)
            .unwrap()
            .is_none()
    );
    assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
}

#[test]
fn response_estimate_accounts_for_turn_and_has_finite_bounds() {
    let mut ctx = context();
    let state = SimulationState::new(&ctx).unwrap();
    assert_eq!(
        avoidance::response_ticks(&ctx, &state, Command::default()),
        REFRESH_TICKS
    );
    let command = Command {
        throttle_frac: 1.0,
        target_attitude_rad: 1.0,
    };
    assert!(avoidance::response_ticks(&ctx, &state, command) > REFRESH_TICKS);
    ctx.vehicle.max_rotation_rate_radps = 0.001;
    assert_eq!(
        avoidance::response_ticks(&ctx, &state, command),
        CONTINUATION_TICKS
    );
}

#[test]
fn recovery_queries_propagate_invalid_terrain_instead_of_selecting_escape() {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let mut ctx = RunContext::from_scenario(&request.scenario).unwrap();
    let state = SimulationState::new(&ctx).unwrap();
    ctx.world.terrain = pd_core::TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(2000.0, 0.0), Vec2::new(2001.0, 0.0)],
    };
    assert!(avoidance::select(&request, &ctx, &state, Command::default(), 0).is_err());
}
