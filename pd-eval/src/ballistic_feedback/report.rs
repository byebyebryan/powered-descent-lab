//! Display-only adaptation to the maintained rich report. Origins are replayed
//! from source actions, never restored from serialized snapshots.
use super::*;
use pd_control::{ControllerFrame, ControllerUpdateRecord};
use pd_core::RunManifest;
use pd_report::{
    flight_annotations::{
        AnnotationNavigation, ExecutedCorrection, FlightAnnotations, FlightBoundary, NavigationLink,
    },
    planning_cycles::{CycleReview, PathPoint, PlanningReview},
};

fn boundary(s: &SimulationStateSnapshotV1) -> FlightBoundary {
    FlightBoundary {
        physics_step: s.physics_step,
        sim_time_s: s.sim_time_s,
        position_m: s.position_m,
        velocity_mps: s.velocity_mps,
        attitude_rad: s.attitude_rad,
        fuel_kg: s.fuel_kg,
    }
}

fn executed_entry(
    refreshes: &[Refresh],
    number: usize,
    revision: Option<usize>,
) -> Result<&Refresh> {
    refreshes
        .iter()
        .find(|r| {
            r.goal.number == number
                && !r.goal.destination
                && revision.is_none_or(|revision| {
                    r.goal.revision == revision
                        && matches!(
                            r.decision.as_str(),
                            "waypoint_selected" | "waypoint_replaced"
                        )
                })
        })
        .context("missing candidate waypoint entry")
}

fn point(s: &SimulationStateSnapshotV1) -> PathPoint {
    PathPoint {
        time_s: s.sim_time_s,
        x_m: s.position_m.x,
        y_m: s.position_m.y,
    }
}

fn projection(
    ctx: &RunContext,
    origin: &SimulationStateSnapshotV1,
    velocity: Vec2,
    steps: u64,
) -> Vec<PathPoint> {
    let state = KinematicStateV2 {
        position_m: origin.position_m,
        velocity_mps: velocity,
    };
    (0..=steps)
        .filter(|k| k % 12 == 0 || *k == steps)
        .map(|k| {
            let p =
                aim::project(state, ctx.world.gravity_mps2, ctx.sim.physics_dt_s(), k).position_m;
            PathPoint {
                time_s: origin.sim_time_s + k as f64 * ctx.sim.physics_dt_s(),
                x_m: p.x,
                y_m: p.y,
            }
        })
        .collect()
}

fn review(ctx: &RunContext, result: &FeedbackResult) -> Result<PlanningReview> {
    let mut source = new_ordinary(ctx)?;
    let mut action_index = 0;
    let prefix_end = result.prefix_origin.as_ref().map_or(0, |s| s.physics_step);
    let mut cycles = Vec::new();
    for (index, r) in result.refreshes.iter().enumerate() {
        while source.state.physics_step < r.origin.physics_step {
            let action = result
                .ordinary_flight
                .actions
                .get(action_index)
                .context("report source action missing")?;
            let update = FlightProgramUpdateV1 {
                physics_step: action.physics_step,
                command: action.command,
                phase: "report_source_replay".into(),
            };
            let neutral = result.terrain_neutral_diagnostic && action.physics_step >= prefix_end;
            advance_one(ctx, &mut source, &update, neutral)?;
            action_index += 1;
        }
        ensure!(
            SimulationStateSnapshotV1::from_state(&source.state) == r.origin,
            "report refresh origin differs from actual source replay"
        );
        let nominal = r
            .desired_arc
            .as_ref()
            .map(|a| projection(ctx, &r.origin, a.departure_velocity_mps, a.steps))
            .unwrap_or_default();
        let ballistic_steps = aim::natural_arrival_steps(
            kinematics(&source.state),
            r.goal.position_m.y,
            ctx.world.gravity_mps2,
            ctx.sim.physics_dt_s(),
        )
        .unwrap_or(CONTINUATION_TICKS);
        let ballistic = projection(ctx, &r.origin, r.origin.velocity_mps, ballistic_steps);
        let mut short = Vec::new();
        let command = r.predicted_command.or_else(|| {
            result
                .ordinary_flight
                .actions
                .get(action_index)
                .filter(|a| {
                    a.physics_step == r.origin.physics_step && r.terrain_conflict_m.is_none()
                })
                .map(|a| a.command)
        });
        if let Some(command) = command {
            let mut query = source.state.clone();
            query.set_command(command);
            short.push(point(&r.origin));
            for _ in 0..REFRESH_TICKS {
                if result.terrain_neutral_diagnostic {
                    query.step_physics_and_classify_contact(ctx);
                } else {
                    query.step_with_contact_report(ctx);
                }
                short.push(point(&SimulationStateSnapshotV1::from_state(&query)));
                if query.is_terminal() {
                    break;
                }
            }
        }
        let end = result
            .refreshes
            .get(index + 1)
            .map_or(&result.final_state, |next| &next.origin);
        let mut executed = vec![point(&r.origin)];
        executed.extend(
            result
                .ordinary_flight
                .samples
                .iter()
                .filter(|s| {
                    s.physics_step > r.origin.physics_step && s.physics_step < end.physics_step
                })
                .map(|s| PathPoint {
                    time_s: s.sim_time_s,
                    x_m: s.observation.position_m.x,
                    y_m: s.observation.position_m.y,
                }),
        );
        if end.physics_step > r.origin.physics_step {
            executed.push(point(end));
        }
        let label = if r.goal.destination {
            "Destination".into()
        } else {
            format!(
                "{} {} · revision {}",
                if matches!(
                    r.decision.as_str(),
                    "waypoint_proposal_blocked"
                        | "waypoint_command_blocked"
                        | "waypoint_aim_construction_miss"
                ) {
                    "Proposed waypoint"
                } else {
                    "Waypoint"
                },
                r.goal.number,
                r.goal.revision
            )
        };
        let conflict = r
            .predicted_conflict
            .as_ref()
            .or(r.replan_trigger.as_ref())
            .map(|c| boundary(&c.state))
            .or_else(|| {
                r.terrain_conflict_m.map(|p| {
                    let mut b = boundary(&r.origin);
                    b.position_m = p;
                    if let Some(a) = &r.desired_arc
                        && a.departure_velocity_mps.x != 0.0
                    {
                        let ticks = ((p.x - r.origin.position_m.x)
                            / (a.departure_velocity_mps.x * ctx.sim.physics_dt_s()))
                        .round()
                        .max(0.0) as u64;
                        b.physics_step += ticks;
                        b.sim_time_s += ticks as f64 * ctx.sim.physics_dt_s();
                        b.velocity_mps = aim::project(
                            KinematicStateV2 {
                                position_m: r.origin.position_m,
                                velocity_mps: a.departure_velocity_mps,
                            },
                            ctx.world.gravity_mps2,
                            ctx.sim.physics_dt_s(),
                            ticks,
                        )
                        .velocity_mps;
                    }
                    b
                })
            });
        let ridge = r.ridge_selection.as_ref().or_else(|| {
            result.refreshes.iter().find_map(|other| {
                (other.goal.revision == r.goal.revision && !r.goal.destination)
                    .then_some(other.ridge_selection.as_ref())
                    .flatten()
            })
        });
        let ridge_summary = ridge.map_or(String::new(), |selection| match selection.crest_m {
            Some(crest) => format!("Blocking ridge crest at ({:.3}, {:.3}) m; waypoint beyond its crest by {:.3} m. Dips smaller than one {:.3} m vehicle diameter stay in this feature. Selection is a geometric aim, not proof of an executed clearing or landing.", crest.x, crest.y, r.goal.position_m.x - crest.x, selection.diameter_m),
            None => "Local climb stage: no separating crest resolved before the destination boundary. This goal is not labelled as clearing a complete ridge.".into(),
        });
        let mut cutoff_coast = Vec::new();
        let mut entry_summary = String::new();
        if let Some(c) = &r.correction {
            let remaining = c
                .arrival_physics_step
                .saturating_sub(c.burn_end_physics_step);
            let entry = aim::project(
                c.predicted_cutoff,
                ctx.world.gravity_mps2,
                ctx.sim.physics_dt_s(),
                remaining,
            );
            let apex_ticks = (c.predicted_cutoff.velocity_mps.y
                / (ctx.world.gravity_mps2 * ctx.sim.physics_dt_s()))
            .ceil()
            .max(0.0) as u64;
            let mut cutoff = r.origin.clone();
            cutoff.position_m = c.predicted_cutoff.position_m;
            cutoff.velocity_mps = c.predicted_cutoff.velocity_mps;
            cutoff.physics_step = c.burn_end_physics_step;
            cutoff.sim_time_s = c.burn_end_physics_step as f64 * ctx.sim.physics_dt_s();
            let report_bound = (ctx.sim.max_time_s / ctx.sim.physics_dt_s()) as u64;
            cutoff_coast = projection(
                ctx,
                &cutoff,
                cutoff.velocity_mps,
                remaining
                    .max(apex_ticks)
                    .saturating_add(CONTINUATION_TICKS)
                    .min(report_bound.saturating_sub(c.burn_end_physics_step)),
            );
            let effort = c.thrust_acceleration_mps2.length()
                * c.burn_end_physics_step
                    .saturating_sub(c.turn_end_physics_step.max(r.origin.physics_step))
                    as f64
                * ctx.sim.physics_dt_s();
            entry_summary = format!(
                " Predicted cutoff coast (teal, not flown): goal-time velocity ({:.2}, {:.2}) m/s, speed {:.2} m/s; remaining thrust-impulse estimate {:.2} m/s. Entry and apex are predictions, not physical arrivals.",
                entry.velocity_mps.x,
                entry.velocity_mps.y,
                entry.velocity_mps.length(),
                effort
            );
        }
        let local_height_summary = ridge.and_then(|selection| selection.local_height.as_ref()).map_or(String::new(),
            |height| format!(" Local-feature height seed: terrain {:.2} m plus unchanged allowance {:.2} m. Incoming-corridor maximum {:.2} m does not set the goal height; incoming trajectory checks still cover all terrain.",
                height.terrain_height_m, height.allowance_m, height.incoming_corridor_max_m));
        let room_summary = r.waypoint_braking_room.as_ref().map_or(String::new(), |room|
            if r.decision == "early_destination_obstruction" {
                format!(" Early piecewise destination attempt: projected waypoint entry needs {:.2} m of lateral braking room, leaving {:.2} m. A destination fit exists but its ideal arc is obstructed; normal waypoint replanning starts from this actual state. This attempt is not a clear destination suffix, executed H or terminal admission.", room.required_distance_m, room.remaining_room_m)
            } else {
                format!(" Early destination reacquisition: projected waypoint entry needs {:.2} m of lateral braking room, leaving {:.2} m after that estimate. The previous waypoint is superseded before its crossing, not recorded as an actual H. Desired and cutoff-coast queries plus immediate command checks passed; powered execution remains guarded.", room.required_distance_m, room.remaining_room_m)
            });
        let recovery_summary = r.recovery_query.as_ref().map_or(String::new(), |query| {
            let choices = query.commands.iter().map(|command| {
                let result = if !command.checked { "not checked: response clock unavailable".into() }
                    else { command.conflict.as_ref().map_or("clear over queried horizon".into(), |c| format!("{} at tick {} ({:.3} s ahead)", c.cause, c.state.physics_step, c.state.sim_time_s - query.origin.sim_time_s)) };
                format!("{}: throttle {:.3}, attitude {:.3} rad, {} ticks, {}", command.choice, command.command.throttle_frac, command.command.target_attitude_rad, command.prediction_ticks, result)
            }).collect::<Vec<_>>().join("; ");
            format!(" Recovery query at actual tick {}: {}. These are ordinary-plant clone checks, not executed commands or a landing certificate.", query.origin.physics_step, choices)
        });
        let coast_summary = r.coast_terminal.as_ref().map_or(String::new(), |preview|
            format!(" Coast-to-terminal alternative: existing motion clears crest ({:.2}, {:.2}) m to body-clear x={:.2} m without a new waypoint or H. Engine-off query {} ticks; queried terminal entry at {:.3} s, ({:.2}, {:.2}) m, with lateral ballistic miss {:.2} m. Standalone terminal feedback preview {} ticks; configured initial-acceleration terrain approximation safe: {}. This is a bounded preview, not a complete landing suffix or restored state. Actual coast and terminal commands remain guarded.",
                preview.crest_m.x, preview.crest_m.y, preview.clearance_x_m, preview.coast_ticks,
                preview.entry.origin.sim_time_s, preview.entry.origin.position_m.x, preview.entry.origin.position_m.y,
                preview.entry.ballistic_miss_m, preview.entry.checked_ticks,
                preview.entry.configured_initial_candidate_terrain_safe));
        let rejection_summary = r.coast_terminal_rejection.as_ref().map_or(String::new(), |reason|
            format!(" Coast-to-terminal declined: {reason}. Original aiming/waypoint planning resumes at actual state."));
        let ridge_summary = format!(
            "{ridge_summary}{entry_summary}{local_height_summary}{room_summary}{coast_summary}{rejection_summary}{recovery_summary}"
        );
        cycles.push(CycleReview { index, decision: r.decision.clone(), current: boundary(&r.origin), nominal_identity: None,
            nominal, nominal_extension: short, ballistic, conflict, conflict_phase: r.predicted_conflict.as_ref().map(|c| c.cause.clone()).or_else(|| r.terrain_conflict_m.map(|_| "ideal_arc_query_not_flown".into())),
            correction: None, executed,
            summary: format!("{} at ({:.2}, {:.2}) m. Decision: {}. Current ballistic lateral miss: {}. {} {} {}",
                label, r.goal.position_m.x, r.goal.position_m.y, format_args!("{}{}{}", r.decision, r.previous_goal.as_ref().map_or(String::new(), |old| format!("; previous {} revision {} at ({:.2}, {:.2}) m. Proposal changes are not executed handoffs", if old.destination { "destination".into() } else { format!("waypoint {}", old.number) }, old.revision, old.position_m.x, old.position_m.y)), r.replan_trigger.as_ref().map_or(String::new(), |c| format!("; trigger {} at ({:.2}, {:.2}) m, {:.3} s ahead of actual state", c.cause, c.state.position_m.x, c.state.position_m.y, c.state.sim_time_s-r.origin.sim_time_s))),
                r.current_ballistic_miss_m.map_or("not admitted".into(), |v| format!("{v:.3} m")),
                r.correction.as_ref().map_or("No active burn estimate.".into(), |c| format!("Absolute turn/burn/arrival ticks: {}/{}/{}; predicted cutoff ({:.2}, {:.2}) m with velocity ({:.2}, {:.2}) m/s. These are estimates, not restored states.",
                    c.turn_end_physics_step, c.burn_end_physics_step, c.arrival_physics_step,
                    c.predicted_cutoff.position_m.x, c.predicted_cutoff.position_m.y, c.predicted_cutoff.velocity_mps.x, c.predicted_cutoff.velocity_mps.y)),
                format_args!("{} {}", r.terrain_correction.as_ref().map_or(String::new(), |c| format!("Terrain protection keeps this exact goal/revision. Rejected command: throttle {:.3}, attitude {:.3} rad. Selected {}: throttle {:.3}, attitude {:.3} rad, checked for {} ticks including attitude slew. Pale purple shows the selected command; any red warning belongs to the rejected command. Episode starts at tick {}. This is not waypoint selection, handoff or a whole-route certificate.", c.rejected_command.throttle_frac, c.rejected_command.target_attitude_rad, c.selected_choice, c.selected_command.throttle_frac, c.selected_command.target_attitude_rad, c.prediction_ticks, c.episode_start_physics_step)),
                r.waypoint_height_repair.as_ref().map_or(String::new(), |repair| format!("Waypoint height repair: keeps x = {:.3} m, lifts the previous proposal by {:.3} m. Limiting {} at ({:.3}, {:.3}) m; complete incoming arc plus two-second ideal continuation checked, including {:.3} m of existing handoff height tolerance. This is geometric proposal repair, not an executed handoff or a landing certificate.", repair.from_goal.position_m.x, repair.height_increase_m, repair.limiting_conflict.cause, repair.limiting_conflict.state.position_m.x, repair.limiting_conflict.state.position_m.y, repair.handoff_height_tolerance_m))), ridge_summary),
            boundary_counts: Default::default(), nominal_rejections: Default::default(), queries: vec![],
            active_goal: Some(r.goal.position_m), goal_label: Some(label),
            previous_goal: r.previous_goal.as_ref().map(|g| g.position_m),
            clearing_crest: ridge.and_then(|selection| selection.crest_m).or_else(|| r.coast_terminal.as_ref().map(|p| p.crest_m)), cutoff_coast });
    }
    Ok(PlanningReview {
        schema: "pd-lab.planning-cycle-review.v1".into(),
        provenance: format!(
            "Ballistic feedback candidate · all refresh origins reconstructed from original source actions and compared as complete states. Desired arcs are geometric counterfactuals; short held-command predictions are not whole-route certificates. Actual handoffs and refreshes are distinct. Source/binary/input receipt: attempt.json. Exact command replay and decision reproduction: feedback.json. Terrain-neutral diagnostic: {} (never landing evidence).",
            result.terrain_neutral_diagnostic
        ),
        cycles,
    })
}

pub(super) fn write(
    scenario: &ScenarioSpec,
    result: &FeedbackResult,
    live: &SimulationState,
    output: &Path,
) -> Result<()> {
    write_with_updates(scenario, result, live, output, None)
}

pub(super) fn write_with_updates(
    scenario: &ScenarioSpec,
    result: &FeedbackResult,
    live: &SimulationState,
    output: &Path,
    recorded_updates: Option<&[ControllerUpdateRecord]>,
) -> Result<()> {
    let ctx = RunContext::from_scenario(scenario).map_err(anyhow::Error::msg)?;
    if matches!(
        result.candidate_id.as_str(),
        "ballistic_feedback_v2_replan_r2"
            | "ballistic_feedback_v3_terrain_correction"
            | CANDIDATE_ID
            | "ballistic_feedback_v6_waypoint_effort"
            | "ballistic_feedback_v6_waypoint_recovery"
            | "ballistic_feedback_v6_waypoint_combined"
            | "ballistic_feedback_v7_local_height"
            | "ballistic_feedback_v7_early_target"
            | "ballistic_feedback_v7_local_height_early_target"
            | "ballistic_feedback_v8_landing_duration"
            | "ballistic_feedback_v8_early_target_landing_duration"
            | "ballistic_feedback_v9_landing_countdown"
            | "ballistic_feedback_v9_early_target_landing_countdown"
            | "ballistic_feedback_v10_coast_terminal"
    ) {
        ensure!(
            result.handoff_goal_revisions.len() == result.handoffs.len(),
            "missing executed goal revision binding"
        );
    }
    let s = &result.final_state;
    let manifest = RunManifest {
        schema_version: pd_core::model::RUN_SCHEMA_VERSION,
        scenario_id: scenario.id.clone(),
        scenario_name: scenario.name.clone(),
        scenario_seed: scenario.seed,
        scenario_tags: scenario.tags.clone(),
        controller_id: result.candidate_id.clone(),
        physics_hz: ctx.sim.physics_hz,
        controller_hz: ctx.sim.controller_hz,
        physics_steps: s.physics_step,
        controller_updates: result.ordinary_flight.actions.len() as u64,
        sim_time_s: s.sim_time_s,
        physical_outcome: s.physical_outcome.clone(),
        mission_outcome: s.mission_outcome.clone(),
        end_reason: s.end_reason.clone(),
        summary: live.build_run_summary(&ctx),
    };
    for record in &result.controller_updates {
        ensure!(
            result
                .ordinary_flight
                .actions
                .iter()
                .any(|a| record.physics_step == a.physics_step
                    && record.frame.command == a.command
                    && record.sim_time_s == a.sim_time_s
                    && record.controller_update_index == a.controller_update_index),
            "terminal report frame binding differs"
        );
    }
    let controller_updates: Vec<_> = result
        .ordinary_flight
        .actions
        .iter()
        .map(|a| {
            if let Some(record) = result
                .controller_updates
                .iter()
                .find(|u| u.physics_step == a.physics_step)
            {
                return record.clone();
            }
            let mut frame = ControllerFrame::command_only(a.command);
            frame.phase = Some(
                result
                    .updates
                    .iter()
                    .find(|u| u.physics_step == a.physics_step)
                    .map_or("retained_prefix", |u| u.phase.as_str())
                    .into(),
            );
            ControllerUpdateRecord {
                sim_time_s: a.sim_time_s,
                physics_step: a.physics_step,
                controller_update_index: a.controller_update_index,
                compute_time_us: None,
                frame,
            }
        })
        .collect();
    let corrections = result
        .handoffs
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let entry = executed_entry(
                &result.refreshes,
                i + 1,
                result.handoff_goal_revisions.get(i).copied(),
            )?;
            Ok(ExecutedCorrection {
                number: i + 1,
                entry: boundary(&entry.origin),
                handoff: boundary(h),
                reason:
                    "Executed pass-through waypoint; replanning uses actual position and velocity."
                        .into(),
                after_handoff: "No complete landing suffix was required to admit this waypoint."
                    .into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let annotations = FlightAnnotations {
        caption: format!(
            "Opt-in {} · stop: {} · {}. {}",
            result.candidate_id,
            result.stop,
            if result.terrain_neutral_diagnostic {
                "terrain-neutral construction diagnostic, not a mission landing"
            } else {
                "original terrain, no default promotion"
            },
            if result
                .candidate_id
                .starts_with("ballistic_terminal_coast_diagnostic_")
            {
                "Capability diagnostic with a predeclared takeover clock, not an automatic entry policy. H markers are retained actual prefix handoffs. The coast checkpoint is not an additional waypoint or H."
            } else {
                "H numbers refer only to this candidate's executed waypoint handoffs; retained source-prefix handoffs are not renumbered as candidate decisions."
            },
        ),
        corrections,
        navigation: AnnotationNavigation {
            source_links: [
                "scenario.json",
                "attempt.json",
                "feedback.json",
                "command-replay.json",
            ]
            .into_iter()
            .map(|name| NavigationLink {
                label: name.into(),
                href: name.into(),
            })
            .collect(),
            ..Default::default()
        },
    };
    let html = pd_report::render_run_report_with_flight_annotations(
        scenario,
        None,
        &manifest,
        &result.ordinary_flight.events,
        &result.ordinary_flight.samples,
        recorded_updates.unwrap_or(&controller_updates),
        None,
        None,
        None,
        None,
        Some(&annotations),
    )?;
    let review = review(&ctx, result)?;
    write_json_create_only(&output.join("planning-cycles.json"), &review)?;
    let html = if review.cycles.is_empty() {
        html
    } else {
        pd_report::planning_cycles::attach(&html, &review)?
    };
    crate::evidence_io::write_bytes_create_only_with_context(
        &output.join("report.html"),
        html.as_bytes(),
        "candidate rich report",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executed_handoff_uses_the_accepted_revision_not_the_first_proposal() {
        let request = crate::test_inputs::planner_request("v2_clear_845");
        let ctx = RunContext::from_scenario(&request.scenario).unwrap();
        let state = SimulationState::new(&ctx).unwrap();
        let mut goal = target(&ctx);
        goal.destination = false;
        goal.number = 1;
        goal.revision = 1;
        let first = refresh_record(&state, &goal, None, None, "waypoint_selected", None);
        goal.revision = 2;
        let rejected = refresh_record(&state, &goal, None, None, "waypoint_proposal_blocked", None);
        goal.revision = 3;
        let accepted = refresh_record(&state, &goal, None, None, "waypoint_replaced", None);
        let refreshes = vec![first, rejected, accepted];
        assert_eq!(
            executed_entry(&refreshes, 1, Some(3)).unwrap(),
            &refreshes[2]
        );
        assert!(executed_entry(&refreshes, 1, Some(2)).is_err());
        assert!(executed_entry(&refreshes, 1, Some(4)).is_err());
        assert_eq!(executed_entry(&refreshes, 1, None).unwrap(), &refreshes[0]);
    }
}
