use pd_core::{
    AllowAllBoundedRunGuard, BoundedRunLimitsV1, BoundedRunStopCauseV1, Command, RunContext,
    ScenarioSpec, run_simulation_bounded,
};
use pd_run::{CaptureModeV1, Episode};

fn scenario() -> ScenarioSpec {
    serde_json::from_str(include_str!(
        "../../fixtures/scenarios/timed_checkpoint_idle.json"
    ))
    .unwrap()
}

#[test]
fn incremental_summary_and_capture_equal_native_ordinary_semantics_at_every_boundary() {
    let scenario = scenario();
    let ctx = RunContext::from_scenario(&scenario).unwrap();
    for end in [0, 1, 2, 7, 60, 120] {
        let limits = BoundedRunLimitsV1 {
            command_coverage_end_physics_step: 120,
            hard_end_physics_step: end,
        };
        let expected = run_simulation_bounded(
            &ctx,
            "external",
            limits,
            |_, _| Ok(Command::idle()),
            &mut AllowAllBoundedRunGuard,
        )
        .unwrap();
        let mut captured =
            Episode::reset(&scenario, "external", limits, CaptureModeV1::Captured).unwrap();
        let mut summary =
            Episode::reset(&scenario, "external", limits, CaptureModeV1::Summary).unwrap();
        assert_eq!(captured.observation(), summary.observation());
        while captured.stop().is_none() {
            let step = captured.step(Command::idle()).unwrap();
            assert_eq!(step, summary.step(Command::idle()).unwrap());
            assert!(step.physics_steps_advanced <= 2);
            assert_eq!(summary.retained_record_counts(), (0, 0, 0, 0));
        }
        assert!(captured.step(Command::idle()).is_err());
        let captured = captured.finish().unwrap();
        let summary = summary.finish().unwrap();
        assert_eq!(captured.summary, summary.summary);
        assert!(summary.capture.is_none());
        assert!(summary.agent_updates.is_none());
        assert_eq!(captured.capture.unwrap(), expected);
    }
}

#[test]
fn real_contact_and_horizon_are_handled_by_ordinary_plant_without_autoreset() {
    let mut scenario = scenario();
    scenario.mission.goal = pd_core::EvaluationGoal::LandingOnPad {
        target_pad_id: "pad_main".to_owned(),
    };
    scenario.initial_state.position_m.y = scenario.vehicle.geometry.touchdown_base_offset_m;
    let ctx = RunContext::from_scenario(&scenario).unwrap();
    let limits = BoundedRunLimitsV1 {
        command_coverage_end_physics_step: 120,
        hard_end_physics_step: 120,
    };
    let expected = run_simulation_bounded(
        &ctx,
        "contact",
        limits,
        |_, _| Ok(Command::idle()),
        &mut AllowAllBoundedRunGuard,
    )
    .unwrap();
    let mut episode =
        Episode::reset(&scenario, "contact", limits, CaptureModeV1::Captured).unwrap();
    let step = episode.step(Command::idle()).unwrap();
    assert_eq!(step.stop, Some(BoundedRunStopCauseV1::MissionTerminal));
    assert_eq!(step.physics_steps_advanced, 1);
    assert_eq!(episode.finish().unwrap().capture.unwrap(), expected);

    scenario.initial_state.position_m.y = 140.0;
    let mut episode = Episode::reset(&scenario, "horizon", limits, CaptureModeV1::Summary).unwrap();
    while episode.stop().is_none() {
        episode.step(Command::idle()).unwrap();
    }
    assert_eq!(
        episode.stop(),
        Some(BoundedRunStopCauseV1::ScenarioHorizonReached)
    );
    assert_eq!(
        episode.finish().unwrap().summary.final_state.physics_step,
        120
    );
}

#[test]
fn unavailable_decision_stops_at_current_clock_and_never_pads_a_command() {
    let scenario = scenario();
    let mut episode = Episode::reset(
        &scenario,
        "external",
        BoundedRunLimitsV1 {
            command_coverage_end_physics_step: 120,
            hard_end_physics_step: 120,
        },
        CaptureModeV1::Captured,
    )
    .unwrap();
    episode.step(Command::idle()).unwrap();
    let stopped = episode.decision_failed("checkpoint unavailable").unwrap();
    assert_eq!(stopped.physics_steps_advanced, 0);
    assert_eq!(stopped.observation.physics_step, 2);
    assert_eq!(stopped.stop, Some(BoundedRunStopCauseV1::ExecutionInvalid));
    let result = episode.finish().unwrap();
    assert_eq!(result.capture.unwrap().run.actions.len(), 1);
    assert_eq!(result.summary.manifest.controller_updates, 1);
}
