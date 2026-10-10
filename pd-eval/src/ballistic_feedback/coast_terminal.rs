//! Failure-only alternative to changing already useful motion. No saved clocks,
//! waypoint goals, synthetic live states or complete landing-suffix search.
use super::*;
use pd_control::TelemetryValue;

pub(super) const MAX_COAST_TICKS: u64 = 4 * CONTINUATION_TICKS;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntryPreview {
    pub origin: SimulationStateSnapshotV1,
    pub ballistic_miss_m: f64,
    pub initial_command: Command,
    pub configured_initial_candidate_terrain_safe: bool,
    pub end_state: SimulationStateSnapshotV1,
    pub checked_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preview {
    pub crest_m: Vec2,
    pub diameter_m: f64,
    pub clearance_x_m: f64,
    pub coast_ticks: u64,
    pub entry: EntryPreview,
}

pub(super) enum Proposal {
    Accepted(Box<Preview>),
    Declined(&'static str),
}

pub(super) fn entry_preview(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    state: &SimulationState,
    deadline: u64,
) -> Result<Option<EntryPreview>> {
    let goal = target(ctx);
    if state.is_terminal()
        || state.held_command.throttle_frac != 0.0
        || state.velocity_mps.x <= 0.0
        || state.velocity_mps.y >= 0.0
        || state.position_m.x >= goal.position_m.x
        || state.position_m.y <= goal.position_m.y
        || state.physics_step + CONTINUATION_TICKS + REFRESH_TICKS >= deadline
    {
        return Ok(None);
    }
    let Some(steps) = aim::natural_arrival_steps(
        kinematics(state),
        goal.position_m.y,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
    ) else {
        return Ok(None);
    };
    let end = aim::project(
        kinematics(state),
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        steps,
    );
    if (-end.velocity_mps.y).atan2(end.velocity_mps.x.abs()) + 1e-10 < MIN_ANGLE {
        return Ok(None);
    }
    let miss = goal.position_m.x - end.position_m.x;
    let mut controller = TerminalPdgController::default();
    if reserve_conflict(request, ctx, state, true, "terminal_preview_reserve")?.is_some() {
        return Ok(None);
    }
    if !controller.ballistic_entry_ready(ctx, &state.build_observation(ctx), miss, &mut 0) {
        return Ok(None);
    }
    // Retain configured controller terrain handling. Instead of treating its
    // constant-initial-acceleration extrapolation as a trajectory certificate,
    // propagate a bounded stretch of its actual feedback commands and slew.
    let mut query = state.clone();
    let mut first = None;
    let mut configured_safe = false;
    for _ in 0..CONTINUATION_TICKS / 2 {
        let frame = controller.update(ctx, &query.build_observation(ctx));
        if first.is_none() {
            first = Some(frame.command);
            configured_safe = matches!(
                frame.metrics.get("guidance.terrain_clearance_safe"),
                Some(TelemetryValue::Bool(true))
            );
        }
        if short_conflict(request, ctx, &query, frame.command, true)?.is_some() {
            return Ok(None);
        }
        query.set_command(frame.command);
        for _ in 0..2 {
            query.step_with_contact_report(ctx);
            ensure!(snapshot_finite(&query), "nonfinite terminal preview");
            let reserve = reserve_conflict(request, ctx, &query, true, "terminal_preview_reserve")?;
            if query.is_terminal() {
                if query.physical_outcome != PhysicalOutcome::LandedOnTarget
                    || query.mission_outcome != MissionOutcome::Success
                {
                    return Ok(None);
                }
                break;
            }
            if reserve.is_some() {
                return Ok(None);
            }
        }
        if query.is_terminal() {
            break;
        }
    }
    Ok(Some(EntryPreview {
        origin: SimulationStateSnapshotV1::from_state(state),
        ballistic_miss_m: miss,
        initial_command: first.context("empty terminal preview")?,
        configured_initial_candidate_terrain_safe: configured_safe,
        end_state: SimulationStateSnapshotV1::from_state(&query),
        checked_ticks: query.physics_step - state.physics_step,
    }))
}

pub(super) fn propose(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    state: &SimulationState,
    conflict: &PredictedConflict,
    deadline: u64,
) -> Result<Proposal> {
    if state.held_command.throttle_frac != 0.0 || state.velocity_mps.x <= 0.0 {
        return Ok(Proposal::Declined("not_forward_engine_off_motion"));
    }
    let diameter = conservative_body_diameter(&ctx.vehicle.geometry);
    if conflict.state.position_m.x.max(state.position_m.x) + diameter >= ctx.target_pad.center_x_m {
        return Ok(Proposal::Declined("feature_too_close_to_destination"));
    }
    let (_, selection) = ridge::placement(
        &ctx.world.terrain,
        state.position_m.x,
        conflict.state.position_m.x,
        ctx.target_pad.center_x_m,
        diameter,
    )?;
    let Some(crest) = selection.crest_m else {
        return Ok(Proposal::Declined("no_resolved_crest"));
    };
    // Same conservative body-clear crossing used by the existing ridge H
    // window, but it is a coast checkpoint, not an inserted waypoint or H.
    let clearance_x = crest.x + diameter * 0.5;
    let mut query = state.clone();
    let limit = state.physics_step + MAX_COAST_TICKS;
    while query.position_m.x < clearance_x || !query.physics_step.is_multiple_of(2) {
        if query.physics_step >= limit
            || query.physics_step + CONTINUATION_TICKS + REFRESH_TICKS >= deadline
        {
            return Ok(Proposal::Declined("coast_preview_budget"));
        }
        query.set_command(Command::default());
        query.step_with_contact_report(ctx);
        ensure!(snapshot_finite(&query), "nonfinite coast preview");
        let reserve = reserve_conflict(request, ctx, &query, false, "coast_preview_reserve")?;
        if query.is_terminal() || reserve.is_some() {
            return Ok(Proposal::Declined("current_coast_blocked"));
        }
    }
    let Some(entry) = entry_preview(request, ctx, &query, deadline)? else {
        return Ok(Proposal::Declined("terminal_entry_or_preview_rejected"));
    };
    Ok(Proposal::Accepted(Box::new(Preview {
        crest_m: crest,
        diameter_m: diameter,
        clearance_x_m: clearance_x,
        coast_ticks: query.physics_step - state.physics_step,
        entry,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query_fixture(
        high_crest: bool,
    ) -> (
        WaypointDirectNominalDirectGenerationRequest,
        RunContext,
        SimulationState,
        PredictedConflict,
    ) {
        let request = crate::test_inputs::planner_request("v2_clear_845");
        let mut ctx = RunContext::from_scenario(&request.scenario).unwrap();
        let goal = target(&ctx).position_m;
        ctx.world.terrain = pd_core::TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(goal.x - 500.0, goal.y - 100.0),
                Vec2::new(
                    goal.x - 210.0,
                    goal.y + if high_crest { 500.0 } else { 350.0 },
                ),
                Vec2::new(goal.x - 190.0, goal.y + 320.0),
                Vec2::new(goal.x - 100.0, goal.y - 100.0),
                Vec2::new(goal.x + 500.0, goal.y - 100.0),
            ],
        };
        let mut state = SimulationState::new(&ctx).unwrap();
        // Query-only airborne state, never a flown or restart fixture.
        state.position_m = Vec2::new(goal.x - 320.0, goal.y + 460.0);
        state.velocity_mps = Vec2::new(58.0, -24.0);
        state.physics_step = 2400;
        state.sim_time_s = 20.0;
        let mut conflict = SimulationStateSnapshotV1::from_state(&state);
        conflict.position_m = Vec2::new(goal.x - 215.0, goal.y + 360.0);
        (
            request,
            ctx,
            state,
            PredictedConflict {
                state: conflict,
                cause: "ideal_arc_reserve".into(),
            },
        )
    }

    #[test]
    fn clear_coast_uses_body_derived_crossing_without_mutating_the_source() {
        let (request, ctx, state, conflict) = query_fixture(false);
        let before = SimulationStateSnapshotV1::from_state(&state);
        let Proposal::Accepted(preview) = propose(&request, &ctx, &state, &conflict, 9600).unwrap()
        else {
            panic!("expected a clear bounded coast and terminal preview");
        };
        assert_eq!(
            preview.clearance_x_m,
            preview.crest_m.x + preview.diameter_m * 0.5
        );
        assert!(preview.coast_ticks > 0 && preview.coast_ticks <= MAX_COAST_TICKS);
        assert_eq!(
            preview.entry.origin.physics_step,
            state.physics_step + preview.coast_ticks
        );
        assert!(preview.entry.origin.position_m.x >= preview.clearance_x_m);
        assert_eq!(preview.entry.origin.velocity_mps.x, state.velocity_mps.x);
        assert_eq!(preview.entry.origin.fuel_kg, state.fuel_kg);
        assert_eq!(preview.entry.checked_ticks, CONTINUATION_TICKS);
        assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
    }

    #[test]
    fn blocked_current_coast_does_not_borrow_the_desired_arc_velocity() {
        let (request, ctx, state, conflict) = query_fixture(true);
        assert!(matches!(
            propose(&request, &ctx, &state, &conflict, 9600).unwrap(),
            Proposal::Declined("current_coast_blocked")
        ));
    }

    #[test]
    fn admission_keeps_powered_state_deadline_and_domain_boundaries() {
        let (request, mut ctx, mut state, conflict) = query_fixture(false);
        state.set_command(Command {
            throttle_frac: 1.0,
            target_attitude_rad: 0.0,
        });
        assert!(matches!(
            propose(&request, &ctx, &state, &conflict, 9600).unwrap(),
            Proposal::Declined("not_forward_engine_off_motion")
        ));
        state.set_command(Command::default());
        assert!(matches!(
            propose(&request, &ctx, &state, &conflict, state.physics_step + 240).unwrap(),
            Proposal::Declined("coast_preview_budget")
        ));
        ctx.world.terrain = pd_core::TerrainDefinition::Heightfield {
            points_m: vec![
                state.position_m,
                Vec2::new(state.position_m.x + 1.0, state.position_m.y),
            ],
        };
        assert!(propose(&request, &ctx, &state, &conflict, 9600).is_err());
    }

    #[test]
    fn preview_rejects_future_terrain_without_mutating_the_actual_entry() {
        let (request, mut ctx, state, conflict) = query_fixture(false);
        let Proposal::Accepted(preview) = propose(&request, &ctx, &state, &conflict, 9600).unwrap()
        else {
            panic!("expected base preview");
        };
        let mut entry = state.clone();
        entry.set_command(Command::default());
        for _ in 0..preview.coast_ticks {
            entry.step_with_contact_report(&ctx);
        }
        let before = SimulationStateSnapshotV1::from_state(&entry);
        ctx.world.terrain = pd_core::TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(entry.position_m.x - 500.0, -100.0),
                Vec2::new(entry.position_m.x + 10.0, -100.0),
                Vec2::new(entry.position_m.x + 30.0, entry.position_m.y + 100.0),
                Vec2::new(entry.position_m.x + 500.0, -100.0),
            ],
        };
        assert!(
            entry_preview(&request, &ctx, &entry, 9600)
                .unwrap()
                .is_none()
        );
        assert_eq!(SimulationStateSnapshotV1::from_state(&entry), before);
    }
}
