//! Bounded saved-prefix capability probe, not a planner or an entry policy.
//! Replays the original source to H, coasts to a predeclared saved checkpoint,
//! then flies the maintained terminal controller under unchanged physical guards.
use super::*;
use pd_control::{ControllerFrame, ControllerUpdateRecord, TransferPdgControllerConfig};

#[derive(Clone, Copy, Debug, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalSetup {
    Standalone,
    Transfer,
    TransferWaypoint,
}

impl TerminalSetup {
    fn controller(self) -> TerminalPdgController {
        match self {
            Self::Standalone => TerminalPdgController::default(),
            Self::Transfer => {
                TerminalPdgController::new(TransferPdgControllerConfig::default().terminal)
            }
            Self::TransferWaypoint => {
                TerminalPdgController::new(TransferPdgControllerConfig::default().terminal)
                    .with_waypoint_guidance_plan_retention()
            }
        }
    }
}

pub struct Trial {
    pub live: OrdinaryLive,
    pub updates: Vec<FlightProgramUpdateV1>,
    pub frames: Vec<ControllerUpdateRecord>,
    pub refreshes: Vec<Refresh>,
    pub entry: Option<SimulationStateSnapshotV1>,
    pub entry_dynamics_ready: Option<bool>,
    pub stop: String,
}

pub fn source_updates(baseline: &FeedbackResult) -> Vec<FlightProgramUpdateV1> {
    baseline
        .ordinary_flight
        .actions
        .iter()
        .map(|a| FlightProgramUpdateV1 {
            physics_step: a.physics_step,
            command: a.command,
            phase: "verified_original_prefix".into(),
        })
        .collect()
}

pub fn validate_boundaries<'a>(
    baseline: &'a FeedbackResult,
    scenario_sha256: &str,
    prefix_handoff: usize,
    takeover_handoff: usize,
) -> Result<(&'a SimulationStateSnapshotV1, &'a SimulationStateSnapshotV1)> {
    ensure!(
        baseline.integrity_passed
            && baseline.source_replay_passed
            && baseline.decisions_reproduced
            && !baseline.terrain_neutral_diagnostic,
        "unverified or neutral source"
    );
    ensure!(
        baseline.scenario_sha256 == scenario_sha256,
        "scenario/source binding differs"
    );
    ensure!(
        prefix_handoff > 0 && takeover_handoff >= prefix_handoff,
        "invalid H indices"
    );
    let prefix = baseline
        .handoffs
        .get(prefix_handoff - 1)
        .context("missing prefix H")?;
    let takeover = baseline
        .handoffs
        .get(takeover_handoff - 1)
        .context("missing takeover H")?;
    ensure!(
        takeover.physics_step >= prefix.physics_step
            && prefix.physics_step.is_multiple_of(2)
            && takeover.physics_step.is_multiple_of(2),
        "invalid command boundary"
    );
    ensure!(
        baseline.handoff_goal_revisions.len() == baseline.handoffs.len(),
        "missing source H bindings"
    );
    Ok((prefix, takeover))
}

pub fn replay_prefix(
    ctx: &RunContext,
    baseline: &FeedbackResult,
    prefix: &SimulationStateSnapshotV1,
) -> Result<OrdinaryLive> {
    let mut live = new_ordinary(ctx)?;
    let updates: Vec<_> = source_updates(baseline)
        .into_iter()
        .filter(|u| u.physics_step < prefix.physics_step)
        .collect();
    advance_ordinary(ctx, &mut live, &updates, prefix.physics_step)?;
    ensure!(
        live.evidence.final_state == *prefix,
        "prefix source differs"
    );
    Ok(live)
}

pub fn fly(
    request: &WaypointDirectNominalDirectGenerationRequest,
    baseline: &FeedbackResult,
    prefix: &SimulationStateSnapshotV1,
    entry_step: u64,
    setup: TerminalSetup,
) -> Result<Trial> {
    let ctx = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let deadline = pd_plan::waypoint_v2::original_deadline(
        ctx.sim.max_time_s,
        request.policy.analytical_policy.mission_budget_s(),
    )
    .map_err(anyhow::Error::msg)?;
    let mut live = replay_prefix(&ctx, baseline, prefix)?;
    let mut terminal = setup.controller();
    let goal = target(&ctx);
    let mut updates = Vec::new();
    let mut frames = Vec::new();
    let mut refreshes = vec![refresh_record(
        &live.state,
        &goal,
        None,
        None,
        "diagnostic_preserve_existing_coast",
        None,
    )];
    let mut entry = None;
    let mut entry_dynamics_ready = None;
    let started = std::time::Instant::now();
    let stop = loop {
        let state = &live.state;
        if state.is_terminal() {
            break "physical_terminal".into();
        }
        if started.elapsed().as_secs() >= 60 {
            break "case_wall_bound".into();
        }
        if state.physics_step + REFRESH_TICKS >= deadline || state.fuel_kg <= 0.0 {
            break "original_budget".into();
        }
        let landing = state.physics_step >= entry_step;
        if let Err(e) = body_safe(request, &ctx, state, landing) {
            break format!("actual_body_reserve: {e}");
        }
        if landing && entry.is_none() {
            ensure!(state.physics_step == entry_step, "entry clock mismatch");
            let steps = aim::natural_arrival_steps(
                kinematics(state),
                goal.position_m.y,
                ctx.world.gravity_mps2,
                ctx.sim.physics_dt_s(),
            )
            .context("no natural arrival at diagnostic entry")?;
            let miss = goal.position_m.x
                - aim::project(
                    kinematics(state),
                    ctx.world.gravity_mps2,
                    ctx.sim.physics_dt_s(),
                    steps,
                )
                .position_m
                .x;
            // A query only. This experiment deliberately tests a predeclared
            // takeover clock; it does not invent automatic entry admission.
            entry_dynamics_ready = Some(terminal.ballistic_entry_ready(
                &ctx,
                &state.build_observation(&ctx),
                miss,
                &mut 0,
            ));
            entry = Some(SimulationStateSnapshotV1::from_state(state));
            refreshes.push(refresh_record(
                state,
                &goal,
                None,
                None,
                "diagnostic_scheduled_terminal_takeover",
                Some(miss),
            ));
        }
        let frame = if landing {
            terminal.update(&ctx, &state.build_observation(&ctx))
        } else {
            let mut frame = ControllerFrame::command_only(Command::default());
            frame.phase = Some("diagnostic_engine_off_coast".into());
            frame
        };
        if let Some(conflict) = short_conflict(request, &ctx, state, frame.command, landing)? {
            let mut record =
                refresh_record(state, &goal, None, None, "short_command_obstruction", None);
            record.predicted_command = Some(frame.command);
            record_conflict(&mut record, conflict.clone());
            refreshes.push(record);
            break format!("short_command_rejected: {}", conflict.cause);
        }
        let update = FlightProgramUpdateV1 {
            physics_step: state.physics_step,
            command: frame.command,
            phase: if landing {
                "diagnostic_maintained_terminal"
            } else {
                "diagnostic_engine_off_coast"
            }
            .into(),
        };
        frames.push(ControllerUpdateRecord {
            sim_time_s: state.sim_time_s,
            physics_step: state.physics_step,
            controller_update_index: state.physics_step / 2,
            compute_time_us: None,
            frame,
        });
        advance_one(&ctx, &mut live, &update, false)?;
        updates.push(update);
    };
    Ok(Trial {
        live,
        updates,
        frames,
        refreshes,
        entry,
        entry_dynamics_ready,
        stop,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved_shape() -> FeedbackResult {
        let scenario = crate::test_inputs::planner_request("v2_clear_845").scenario;
        let ctx = RunContext::from_scenario(&scenario).unwrap();
        let live = new_ordinary(&ctx).unwrap();
        let prefix = live.evidence.final_state.clone();
        let mut checkpoint = prefix.clone();
        checkpoint.physics_step = 2;
        FeedbackResult {
            candidate_id: "test-query-only".into(),
            scenario_sha256: "bound-scenario".into(),
            prefix_flight_sha256: None,
            prefix_origin: None,
            terrain_neutral_diagnostic: false,
            stop: "query-only".into(),
            reason: "query-only".into(),
            refreshes: vec![],
            handoffs: vec![prefix.clone(), checkpoint],
            handoff_goal_revisions: vec![1, 2],
            updates: vec![],
            final_state: prefix,
            integrity_passed: true,
            source_replay_passed: true,
            decisions_reproduced: true,
            ordinary_flight: live.evidence,
            terrain_domain_stop: None,
            controller_updates: vec![],
        }
    }

    #[test]
    fn source_binding_rejects_unverified_neutral_and_other_scenario() {
        let baseline = saved_shape();
        assert!(validate_boundaries(&baseline, "bound-scenario", 1, 2).is_ok());
        assert!(validate_boundaries(&baseline, "different-scenario", 1, 2).is_err());
        for field in 0..4 {
            let mut invalid = baseline.clone();
            match field {
                0 => invalid.integrity_passed = false,
                1 => invalid.source_replay_passed = false,
                2 => invalid.decisions_reproduced = false,
                _ => invalid.terrain_neutral_diagnostic = true,
            }
            assert!(validate_boundaries(&invalid, "bound-scenario", 1, 2).is_err());
        }
    }

    #[test]
    fn source_binding_rejects_missing_inverted_and_odd_boundaries() {
        let baseline = saved_shape();
        for (prefix, entry) in [(0, 1), (2, 1), (1, 3), (3, 3)] {
            assert!(validate_boundaries(&baseline, "bound-scenario", prefix, entry).is_err());
        }
        let mut invalid = baseline.clone();
        invalid.handoffs[1].physics_step = 1;
        assert!(validate_boundaries(&invalid, "bound-scenario", 1, 2).is_err());
        invalid = baseline.clone();
        invalid.handoffs[0].physics_step = 4;
        assert!(validate_boundaries(&invalid, "bound-scenario", 1, 2).is_err());
        invalid = baseline;
        invalid.handoff_goal_revisions.pop();
        assert!(validate_boundaries(&invalid, "bound-scenario", 1, 2).is_err());
    }
}
