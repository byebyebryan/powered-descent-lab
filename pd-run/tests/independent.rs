use pd_core::{
    AllowAllBoundedRunGuard, BoundedRunLimitsV1, BoundedRunStopCauseV1, Command,
    FlightProgramBindingV1, FlightProgramUpdateV1, Observation, RunContext, ScenarioSpec,
    replay_simulation_bounded, run_simulation,
};
use pd_run::{
    Agent, AgentDescriptorV1, AgentFrame, MissionView, PreparedProgramV1, run_agent,
    run_prepared_program,
};

fn context() -> RunContext {
    let scenario: ScenarioSpec = serde_json::from_str(include_str!(
        "../../fixtures/scenarios/timed_checkpoint_idle.json"
    ))
    .unwrap();
    RunContext::from_scenario(&scenario).unwrap()
}

struct Coast {
    fail_at: Option<u64>,
    observed: Vec<u64>,
}

impl Agent for Coast {
    fn descriptor(&self) -> AgentDescriptorV1 {
        AgentDescriptorV1 {
            schema_version: 1,
            implementation: "independent-coast-v1".to_owned(),
            configuration: serde_json::json!({"fail_at": self.fail_at}),
            checkpoint_sha256: None,
        }
    }

    fn reset(&mut self, mission: MissionView<'_>) -> Result<(), String> {
        assert_eq!(mission.world.gravity_mps2, 1.62);
        self.observed.clear();
        Ok(())
    }

    fn update(&mut self, _: MissionView<'_>, obs: &Observation) -> Result<AgentFrame, String> {
        if self.fail_at == Some(obs.physics_step) {
            return Err("deliberate unavailable decision".to_owned());
        }
        self.observed.push(obs.physics_step);
        Ok(AgentFrame::command_only(Command::idle()))
    }
}

fn coast(fail_at: Option<u64>) -> Coast {
    Coast {
        fail_at,
        observed: Vec::new(),
    }
}

#[test]
fn independent_agent_matches_core_and_replays_complete_physical_tuple() {
    let ctx = context();
    let limits = BoundedRunLimitsV1 {
        command_coverage_end_physics_step: 120,
        hard_end_physics_step: 120,
    };
    let mut agent = coast(None);
    let actual = run_agent(&ctx, &mut agent, limits).unwrap();
    let expected = run_simulation(&ctx, "independent-coast-v1", |_, _| Command::idle()).unwrap();
    assert_eq!(actual.episode.run, expected);
    assert_eq!(actual.episode.stop, BoundedRunStopCauseV1::MissionTerminal);
    assert_eq!(agent.observed, (0..60).step_by(2).collect::<Vec<_>>());
    let replay = replay_simulation_bounded(
        &ctx,
        "independent-coast-v1",
        &actual.episode.run.actions,
        limits,
        &mut AllowAllBoundedRunGuard,
    )
    .unwrap();
    assert_eq!(actual.episode, replay);
}

#[test]
fn prepared_schedule_and_online_agent_share_host_and_physical_binding() {
    let ctx = context();
    let limits = BoundedRunLimitsV1 {
        command_coverage_end_physics_step: 8,
        hard_end_physics_step: 7,
    };
    let descriptor = coast(None).descriptor();
    let mut program = PreparedProgramV1 {
        schema_version: 1,
        descriptor,
        binding: FlightProgramBindingV1 {
            sim: ctx.sim.clone(),
            world: ctx.world.clone(),
            vehicle: ctx.vehicle.clone(),
            initial_state: ctx.initial_state.clone(),
            mission: ctx.mission.clone(),
        },
        limits,
        updates: (0..8)
            .step_by(2)
            .map(|physics_step| FlightProgramUpdateV1 {
                physics_step,
                phase: "coast".to_owned(),
                command: Command::idle(),
            })
            .collect(),
    };
    let online = run_agent(&ctx, &mut coast(None), limits).unwrap();
    let prepared = run_prepared_program(&ctx, &program).unwrap();
    assert_eq!(online.episode, prepared.episode);
    assert_eq!(prepared.episode.final_state.physics_step, 7);
    assert_eq!(
        prepared.episode.stop,
        BoundedRunStopCauseV1::HardDeadlineReached
    );
    program.binding.vehicle.initial_fuel_kg += 1.0;
    assert!(run_prepared_program(&ctx, &program).is_err());
    program.binding.vehicle = ctx.vehicle.clone();
    program.updates[1].physics_step = 3;
    assert!(run_prepared_program(&ctx, &program).is_err());
}

#[test]
fn zero_command_and_failed_decision_retain_truthful_boundaries() {
    let ctx = context();
    let limits = BoundedRunLimitsV1 {
        command_coverage_end_physics_step: 0,
        hard_end_physics_step: 0,
    };
    let mut agent = coast(None);
    let zero = run_agent(&ctx, &mut agent, limits).unwrap();
    assert!(zero.episode.run.actions.is_empty());
    assert!(zero.controller_updates.is_empty());
    assert!(agent.observed.is_empty());
    assert_eq!(zero.episode.final_state.physics_step, 0);
    assert_eq!(
        zero.episode,
        replay_simulation_bounded(
            &ctx,
            "independent-coast-v1",
            &[],
            limits,
            &mut AllowAllBoundedRunGuard,
        )
        .unwrap()
    );

    let failed = run_agent(
        &ctx,
        &mut coast(Some(4)),
        BoundedRunLimitsV1 {
            command_coverage_end_physics_step: 10,
            hard_end_physics_step: 10,
        },
    )
    .unwrap();
    assert_eq!(failed.episode.stop, BoundedRunStopCauseV1::ExecutionInvalid);
    assert_eq!(failed.episode.final_state.physics_step, 4);
    assert_eq!(failed.controller_updates.len(), 2);
    assert_eq!(failed.episode.run.actions.len(), 2);
    assert!(
        failed
            .episode
            .failure
            .unwrap()
            .reason
            .contains("unavailable")
    );
}

#[test]
fn decision_inputs_ignore_labels_seeds_and_membership() {
    let mut ctx = context();
    let limits = BoundedRunLimitsV1 {
        command_coverage_end_physics_step: 120,
        hard_end_physics_step: 120,
    };
    let first = run_agent(&ctx, &mut coast(None), limits).unwrap();
    ctx.scenario_id = "different-collection".to_owned();
    ctx.scenario_seed = 999;
    ctx.scenario_name = "another label".to_owned();
    ctx.scenario_tags = vec!["heldout".to_owned()];
    let second = run_agent(&ctx, &mut coast(None), limits).unwrap();
    assert_eq!(first.episode.run.actions, second.episode.run.actions);
    assert_eq!(first.episode.run.samples, second.episode.run.samples);
    assert_eq!(first.episode.final_state, second.episode.final_state);
}

#[test]
fn reset_failure_and_nonfinite_or_noncanonical_commands_are_not_admitted() {
    struct Invalid {
        reset_fails: bool,
        command: Command,
    }
    impl Agent for Invalid {
        fn descriptor(&self) -> AgentDescriptorV1 {
            coast(None).descriptor()
        }
        fn reset(&mut self, _: MissionView<'_>) -> Result<(), String> {
            if self.reset_fails {
                Err("missing checkpoint".to_owned())
            } else {
                Ok(())
            }
        }
        fn update(&mut self, _: MissionView<'_>, _: &Observation) -> Result<AgentFrame, String> {
            Ok(AgentFrame::command_only(self.command))
        }
    }
    let ctx = context();
    for (reset_fails, command) in [
        (true, Command::idle()),
        (
            false,
            Command {
                throttle_frac: f64::INFINITY,
                target_attitude_rad: 0.0,
            },
        ),
        (
            false,
            Command {
                throttle_frac: 0.0,
                target_attitude_rad: 4.0,
            },
        ),
    ] {
        let result = run_agent(
            &ctx,
            &mut Invalid {
                reset_fails,
                command,
            },
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 10,
                hard_end_physics_step: 10,
            },
        )
        .unwrap();
        assert_eq!(result.episode.stop, BoundedRunStopCauseV1::ExecutionInvalid);
        assert_eq!(result.episode.final_state.physics_step, 0);
        assert!(result.episode.run.actions.is_empty());
        assert!(result.controller_updates.is_empty());
    }
}
