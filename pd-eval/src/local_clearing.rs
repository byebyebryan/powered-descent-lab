//! Current local-clearing execution, admission and consumed-prefix evidence.
//! The owned planner session handles nominal regeneration after actual handoff.
use crate::{
    BodyAwareTerminalPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    clearing_body_reserve_query, live_rejection, nominal_direct_flight_identity, paired_throttle,
};
use anyhow::{Context, Result, bail};
use pd_core::{
    ActionLogEntry, BoundedRunArtifactsV1, Command, EventKind, EventRecord, FlightProgramUpdateV1,
    IncomingContactV1, RunContext, SampleRecord, SimulationState, SimulationStateSnapshotV1,
};
use pd_plan::local_clearing::{
    LocalClearingGoalV1, LocalClearingPolicyV1, LocalClearingScheduleV1, LocalClearingTemplateV1,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingOrdinaryEvidenceV1 {
    pub final_state: SimulationStateSnapshotV1,
    pub incoming_contact: Option<IncomingContactV1>,
    pub actions: Vec<ActionLogEntry>,
    pub events: Vec<EventRecord>,
    pub samples: Vec<SampleRecord>,
}

#[derive(Clone)]
pub(crate) struct OrdinaryLive {
    pub(crate) state: SimulationState,
    pub(crate) evidence: LocalClearingOrdinaryEvidenceV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingBoundaryV1 {
    pub handoff_physics_step: u64,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingRowV1 {
    pub row_id: String,
    pub entry_id: String,
    pub entry_physics_step: u64,
    pub template: LocalClearingTemplateV1,
    pub desired_thrust_acceleration_mps2: f64,
    pub physically_propagated: bool,
    pub status: String,
    pub stop_reason: Option<String>,
    pub stop_state: Option<SimulationStateSnapshotV1>,
    pub incoming_contact: Option<IncomingContactV1>,
    pub minimum_clearance_m: Option<f64>,
    pub consumed_updates: Vec<FlightProgramUpdateV1>,
    pub boundaries: Vec<LocalClearingBoundaryV1>,
    pub selected_handoff_physics_step: Option<u64>,
    pub selected_proposal_identity: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingTraceStateV1 {
    pub state: SimulationStateSnapshotV1,
    pub body_clearance_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingProposalV1 {
    pub policy: LocalClearingPolicyV1,
    pub context_identity: String,
    pub goal: LocalClearingGoalV1,
    pub row_id: String,
    pub template: LocalClearingTemplateV1,
    pub entry_state: SimulationStateSnapshotV1,
    pub powered_end_state: SimulationStateSnapshotV1,
    pub handoff_state: SimulationStateSnapshotV1,
    pub continuation_end_state: SimulationStateSnapshotV1,
    pub minimum_progress_x_m: f64,
    pub actual_fuel_burn_to_handoff_kg: f64,
    pub schedule: LocalClearingScheduleV1,
    pub trajectory: Vec<LocalClearingTraceStateV1>,
    pub identity: String,
}

fn context_identity(request: &WaypointDirectNominalDirectGenerationRequest) -> Result<String> {
    nominal_direct_flight_identity(&(LocalClearingPolicyV1::default(), request))
}

fn proposal_identity(proposal: &LocalClearingProposalV1) -> Result<String> {
    let mut canonical = proposal.clone();
    canonical.identity.clear();
    nominal_direct_flight_identity(&canonical)
}

pub(crate) fn snapshot_finite(state: &SimulationState) -> bool {
    [
        state.sim_time_s,
        state.position_m.x,
        state.position_m.y,
        state.velocity_mps.x,
        state.velocity_mps.y,
        state.attitude_rad,
        state.angular_rate_radps,
        state.fuel_kg,
        state.held_command.throttle_frac,
        state.held_command.target_attitude_rad,
        state.min_touchdown_clearance_m,
        state.min_hull_clearance_m,
        state.max_speed_mps,
        state.max_abs_attitude_rad,
        state.max_abs_angular_rate_radps,
    ]
    .iter()
    .all(|x| x.is_finite())
}

pub(crate) fn entry_rejection(
    context: &RunContext,
    state: &SimulationState,
    deadline: u64,
) -> Option<String> {
    // Query-only clone removes just the powered-admission restriction. Never
    // stepped or used as the incoming state for trajectory generation.
    let mut query = state.clone();
    query.held_command.throttle_frac = 0.0;
    if !snapshot_finite(state)
        || state.held_command != state.held_command.clamped()
        || !state.held_command.throttle_frac.is_finite()
        || !state.held_command.target_attitude_rad.is_finite()
    {
        return Some("nonfinite or invalid full live entry".into());
    }
    live_rejection(context, &query, deadline)
}

fn reserve(
    request: &WaypointDirectNominalDirectGenerationRequest,
    context: &RunContext,
    state: &SimulationState,
    phase: &str,
    source_exception: bool,
) -> Result<f64> {
    if !snapshot_finite(state) {
        bail!("nonfinite full simulation state");
    }
    let (clearance, required) =
        clearing_body_reserve_query(context, request, state, phase, source_exception)?;
    if clearance < required {
        bail!(
            "body reserve {clearance} m below {required} m at tick {}",
            state.physics_step
        );
    }
    if state.is_terminal() {
        bail!("state terminated at tick {}", state.physics_step);
    }
    Ok(clearance)
}

fn push_sample(
    evidence: &mut LocalClearingOrdinaryEvidenceV1,
    state: &SimulationState,
    context: &RunContext,
) {
    if context
        .sim
        .sample_interval_steps()
        .is_some_and(|i| state.physics_step.is_multiple_of(i) || state.is_terminal())
        && evidence
            .samples
            .last()
            .is_none_or(|s| s.physics_step != state.physics_step)
    {
        evidence.samples.push(SampleRecord {
            sim_time_s: state.sim_time_s,
            physics_step: state.physics_step,
            observation: state.build_observation(context),
            held_command: state.held_command,
        });
    }
}

pub(crate) fn new_ordinary(context: &RunContext) -> Result<OrdinaryLive> {
    let state = SimulationState::new(context)?;
    let mut evidence = LocalClearingOrdinaryEvidenceV1 {
        final_state: SimulationStateSnapshotV1::from_state(&state),
        incoming_contact: None,
        actions: Vec::new(),
        events: Vec::new(),
        samples: Vec::new(),
    };
    push_sample(&mut evidence, &state, context);
    Ok(OrdinaryLive { state, evidence })
}

pub(crate) fn advance_ordinary(
    context: &RunContext,
    live: &mut OrdinaryLive,
    updates: &[FlightProgramUpdateV1],
    endpoint: u64,
) -> Result<()> {
    let mut index = 0;
    while live.state.physics_step < endpoint && !live.state.is_terminal() {
        if live.state.physics_step.is_multiple_of(2) {
            let update = updates
                .get(index)
                .context("ordinary command coverage exhausted")?;
            if update.physics_step != live.state.physics_step
                || update.command != update.command.clamped()
                || !update.command.throttle_frac.is_finite()
                || !update.command.target_attitude_rad.is_finite()
            {
                bail!("ordinary missing/duplicate/invalid global update");
            }
            live.state.set_command(update.command);
            live.evidence.actions.push(ActionLogEntry {
                sim_time_s: live.state.sim_time_s,
                physics_step: live.state.physics_step,
                controller_update_index: live.state.physics_step / 2,
                command: update.command,
            });
            live.evidence.events.push(EventRecord {
                sim_time_s: live.state.sim_time_s,
                physics_step: live.state.physics_step,
                kind: EventKind::ControllerUpdated,
                message: "controller_updated".into(),
            });
            index += 1;
        }
        let transition = live.state.step_with_contact_report(context);
        live.evidence.events.extend(transition.events);
        if let Some(contact) = transition.incoming_contact {
            live.evidence.incoming_contact = Some(contact);
        }
        push_sample(&mut live.evidence, &live.state, context);
        if !snapshot_finite(&live.state) {
            bail!("nonfinite ordinary live state");
        }
    }
    if index != updates.len() {
        bail!("ordinary extra commands after endpoint/contact");
    }
    live.evidence.final_state = SimulationStateSnapshotV1::from_state(&live.state);
    Ok(())
}

pub fn validate_local_clearing_proposal(
    request: &WaypointDirectNominalDirectGenerationRequest,
    entry: &SimulationState,
    proposal: &LocalClearingProposalV1,
) -> Result<()> {
    proposal.policy.validate().map_err(anyhow::Error::msg)?;
    proposal
        .schedule
        .validate(&proposal.template)
        .map_err(anyhow::Error::msg)?;
    proposal
        .goal
        .validate(entry.physics_step)
        .map_err(anyhow::Error::msg)?;
    if proposal.identity != proposal_identity(proposal)?
        || proposal.context_identity != context_identity(request)?
        || proposal.entry_state != SimulationStateSnapshotV1::from_state(entry)
        || proposal.schedule.entry_physics_step != entry.physics_step
        || proposal.schedule.absolute_deadline_physics_step
            != proposal.goal.absolute_deadline_physics_step
    {
        bail!("local proposal identity/context/live-entry/budget mismatch");
    }
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let original_deadline = (request
        .policy
        .analytical_policy
        .mission_budget_s()
        .min(context.sim.max_time_s)
        * 120.0)
        .floor() as u64;
    if proposal.goal.absolute_deadline_physics_step != original_deadline {
        bail!("local absolute deadline differs from original mission budget");
    }
    if entry_rejection(
        &context,
        entry,
        proposal.goal.absolute_deadline_physics_step,
    )
    .is_some()
    {
        bail!("unsupported live entry");
    }
    let expected_progress = proposal
        .goal
        .progress_x(entry.position_m.x, &context.vehicle.geometry);
    if proposal.minimum_progress_x_m != expected_progress
        || proposal.handoff_state.position_m.x < expected_progress
        || proposal.actual_fuel_burn_to_handoff_kg != entry.fuel_kg - proposal.handoff_state.fuel_kg
    {
        bail!("local progress or fuel evidence mismatch");
    }
    let count = proposal.schedule.continuation_end_physics_step - entry.physics_step + 1;
    if proposal.trajectory.len() as u64 != count
        || proposal.trajectory.first().map(|s| &s.state) != Some(&proposal.entry_state)
        || proposal.trajectory.last().map(|s| &s.state) != Some(&proposal.continuation_end_state)
    {
        bail!("local query trajectory coverage mismatch");
    }
    for (offset, sample) in proposal.trajectory.iter().enumerate() {
        if sample.state.physics_step != entry.physics_step + offset as u64
            || !sample.body_clearance_m.is_finite()
            || sample.body_clearance_m < proposal.policy.minimum_clearance_m
        {
            bail!("local trajectory clock/reserve mismatch");
        }
    }
    for (tick, snapshot) in [
        (
            proposal.schedule.powered_end_physics_step,
            &proposal.powered_end_state,
        ),
        (
            proposal.schedule.handoff_physics_step,
            &proposal.handoff_state,
        ),
    ] {
        if &proposal.trajectory[(tick - entry.physics_step) as usize].state != snapshot {
            bail!("local boundary snapshot mismatch");
        }
    }
    // Independently regenerate each query from the supplied live entry, never
    // from a serialized trajectory snapshot. Check commands as well as endpoints.
    let mut actual = entry.clone();
    let mut update_index = 0;
    for sample in &proposal.trajectory {
        let clearance = reserve(request, &context, &actual, "local", false)?;
        if sample.state != SimulationStateSnapshotV1::from_state(&actual)
            || sample.body_clearance_m != clearance
        {
            bail!("local full query state or exact terrain clearance mismatch");
        }
        if actual.physics_step >= proposal.schedule.handoff_physics_step
            && actual.physics_step.is_multiple_of(2)
            && live_rejection(&context, &actual, original_deadline).is_some()
        {
            bail!("handoff or certificate loses unchanged airborne admission");
        }
        if actual.physics_step == proposal.schedule.continuation_end_physics_step {
            break;
        }
        if actual.physics_step.is_multiple_of(2) {
            let update = &proposal.schedule.updates[update_index];
            let expected_throttle =
                if actual.physics_step < proposal.schedule.powered_end_physics_step {
                    paired_throttle(
                        &context,
                        &BodyAwareTerminalPolicyV1::default(),
                        actual.mass_kg(&context),
                        powered_cap(request, &context) * proposal.template.acceleration_factor,
                    )?
                } else {
                    0.0
                };
            if update.command.throttle_frac != expected_throttle {
                bail!("local throttle does not implement bound finite template");
            }
            actual.set_command(update.command);
            update_index += 1;
        }
        if actual
            .step_with_contact_report(&context)
            .incoming_contact
            .is_some()
        {
            bail!("unexpected actual contact in local proposal");
        }
    }
    if update_index != proposal.schedule.updates.len() {
        bail!("unconsumed local endpoint updates");
    }
    Ok(())
}

struct TraceState {
    live: SimulationState,
    clearance: f64,
}

fn powered_cap(
    request: &WaypointDirectNominalDirectGenerationRequest,
    context: &RunContext,
) -> f64 {
    let p = &request.policy.analytical_policy;
    p.thrust_derate * (1.0 - p.declared_robustness_margin) * context.vehicle.max_thrust_n
        / (context.vehicle.dry_mass_kg + context.vehicle.max_fuel_kg)
}

pub(crate) fn search_row(
    request: &WaypointDirectNominalDirectGenerationRequest,
    context: &RunContext,
    entry_id: &str,
    entry: Option<&SimulationState>,
    entry_status: (u64, Option<&str>),
    template: &LocalClearingTemplateV1,
    goal: &LocalClearingGoalV1,
) -> Result<(LocalClearingRowV1, Option<LocalClearingProposalV1>)> {
    let (entry_tick, exclusion) = entry_status;
    let policy = LocalClearingPolicyV1::default();
    let row_id = format!("{entry_id}_row_{:02}", template.row_index);
    let desired_acceleration = powered_cap(request, context) * template.acceleration_factor;
    let mut row = LocalClearingRowV1 {
        row_id: row_id.clone(),
        entry_id: entry_id.into(),
        entry_physics_step: entry_tick,
        template: template.clone(),
        desired_thrust_acceleration_mps2: desired_acceleration,
        physically_propagated: false,
        status: "rejected".into(),
        stop_reason: None,
        stop_state: None,
        incoming_contact: None,
        minimum_clearance_m: None,
        consumed_updates: Vec::new(),
        boundaries: Vec::new(),
        selected_handoff_physics_step: None,
        selected_proposal_identity: None,
    };
    let powered_end = entry_tick
        .checked_add(template.powered_ticks)
        .context("powered endpoint overflow")?;
    let trace_end = powered_end
        .checked_add(policy.maximum_coast_ticks)
        .and_then(|v| v.checked_add(policy.continuation_ticks))
        .context("trace endpoint overflow")?
        .min(goal.absolute_deadline_physics_step);
    let Some(entry) = entry else {
        row.status = "entry_excluded".into();
        row.stop_reason = Some(exclusion.unwrap_or("missing actual entry").into());
        for k in 1..=360 {
            row.boundaries.push(LocalClearingBoundaryV1 {
                handoff_physics_step: powered_end + 2 * k,
                status: "not_evaluated".into(),
                reason: row.stop_reason.clone(),
            });
        }
        return Ok((row, None));
    };
    let initial_clearance = reserve(request, context, entry, "local", false)?;
    let mut trace = vec![TraceState {
        live: entry.clone(),
        clearance: initial_clearance,
    }];
    let mut state = entry.clone();
    row.physically_propagated = true;
    row.minimum_clearance_m = Some(initial_clearance);
    while state.physics_step < trace_end {
        if state.physics_step.is_multiple_of(2) {
            let command = if state.physics_step < powered_end {
                match paired_throttle(
                    context,
                    &BodyAwareTerminalPolicyV1::default(),
                    state.mass_kg(context),
                    desired_acceleration,
                ) {
                    Ok(throttle) => Command {
                        throttle_frac: throttle,
                        target_attitude_rad: template.target_attitude_rad,
                    },
                    Err(error) => {
                        row.stop_reason = Some(format!("throttle_conversion: {error}"));
                        break;
                    }
                }
            } else {
                Command {
                    throttle_frac: 0.0,
                    target_attitude_rad: 0.0,
                }
            };
            row.consumed_updates.push(FlightProgramUpdateV1 {
                physics_step: state.physics_step,
                phase: if state.physics_step < powered_end {
                    "local_boost"
                } else {
                    "local_coast"
                }
                .into(),
                command,
            });
            state.set_command(command);
        }
        let transition = state.step_with_contact_report(context);
        if let Some(contact) = transition.incoming_contact {
            row.incoming_contact = Some(contact);
            row.stop_reason = Some("actual_contact".into());
            break;
        }
        match reserve(request, context, &state, "local", false) {
            Ok(clearance) => {
                row.minimum_clearance_m = Some(row.minimum_clearance_m.unwrap().min(clearance));
                trace.push(TraceState {
                    live: state.clone(),
                    clearance,
                });
            }
            Err(error) => {
                row.stop_reason = Some(format!("physical_trace: {error}"));
                break;
            }
        }
    }
    row.stop_state = Some(SimulationStateSnapshotV1::from_state(&state));
    if row.stop_reason.is_none() {
        row.stop_reason = Some(
            if trace_end == goal.absolute_deadline_physics_step {
                "original_deadline"
            } else {
                "finite_trace_bound"
            }
            .into(),
        );
    }
    let progress = goal.progress_x(entry.position_m.x, &context.vehicle.geometry);
    let safe_trace_end = trace.last().unwrap().live.physics_step;
    let mut selected = None;
    for k in 1..=360 {
        let h = powered_end.checked_add(2 * k).context("handoff overflow")?;
        let guard_end = h
            .checked_add(policy.continuation_ticks)
            .context("certificate overflow")?;
        let failure = if guard_end > goal.absolute_deadline_physics_step {
            Some((
                "budget_rejected",
                "certificate exceeds original absolute deadline".to_string(),
            ))
        } else if h > safe_trace_end {
            Some(("prefix_incomplete", row.stop_reason.clone().unwrap()))
        } else {
            let handoff = &trace[(h - entry_tick) as usize].live;
            if handoff.position_m.x < progress {
                Some((
                    "insufficient_progress",
                    format!("handoff x {} below {}", handoff.position_m.x, progress),
                ))
            } else if let Some(reason) =
                live_rejection(context, handoff, goal.absolute_deadline_physics_step)
            {
                Some(("handoff_unsupported", reason))
            } else if guard_end > safe_trace_end {
                Some(("unsafe_continuation", row.stop_reason.clone().unwrap()))
            } else {
                (h..=guard_end).step_by(2).find_map(|tick| {
                    live_rejection(
                        context,
                        &trace[(tick - entry_tick) as usize].live,
                        goal.absolute_deadline_physics_step,
                    )
                    .map(|reason| ("continuation_unsupported", format!("tick {tick}: {reason}")))
                })
            }
        };
        let (status, reason) = failure
            .map(|(s, r)| (s.to_string(), Some(r)))
            .unwrap_or(("eligible".into(), None));
        row.boundaries.push(LocalClearingBoundaryV1 {
            handoff_physics_step: h,
            status: status.clone(),
            reason,
        });
        if status == "eligible" && selected.is_none() {
            let at = |tick: u64| {
                SimulationStateSnapshotV1::from_state(&trace[(tick - entry_tick) as usize].live)
            };
            let mut proposal = LocalClearingProposalV1 {
                policy: policy.clone(),
                context_identity: context_identity(request)?,
                goal: goal.clone(),
                row_id: row_id.clone(),
                template: template.clone(),
                entry_state: at(entry_tick),
                powered_end_state: at(powered_end),
                handoff_state: at(h),
                continuation_end_state: at(guard_end),
                minimum_progress_x_m: progress,
                actual_fuel_burn_to_handoff_kg: entry.fuel_kg
                    - trace[(h - entry_tick) as usize].live.fuel_kg,
                schedule: LocalClearingScheduleV1 {
                    entry_physics_step: entry_tick,
                    powered_end_physics_step: powered_end,
                    handoff_physics_step: h,
                    continuation_end_physics_step: guard_end,
                    absolute_deadline_physics_step: goal.absolute_deadline_physics_step,
                    updates: row
                        .consumed_updates
                        .iter()
                        .take_while(|u| u.physics_step < guard_end)
                        .cloned()
                        .collect(),
                },
                trajectory: trace[..=(guard_end - entry_tick) as usize]
                    .iter()
                    .map(|s| LocalClearingTraceStateV1 {
                        state: SimulationStateSnapshotV1::from_state(&s.live),
                        body_clearance_m: s.clearance,
                    })
                    .collect(),
                identity: String::new(),
            };
            proposal.identity = proposal_identity(&proposal)?;
            validate_local_clearing_proposal(request, entry, &proposal)?;
            row.status = "locally_accepted".into();
            row.selected_handoff_physics_step = Some(h);
            row.selected_proposal_identity = Some(proposal.identity.clone());
            selected = Some(proposal);
        }
    }
    Ok((row, selected))
}

pub(crate) fn local_rank(
    a: &LocalClearingProposalV1,
    b: &LocalClearingProposalV1,
) -> std::cmp::Ordering {
    b.schedule
        .entry_physics_step
        .cmp(&a.schedule.entry_physics_step)
        .then(
            a.schedule
                .handoff_physics_step
                .cmp(&b.schedule.handoff_physics_step),
        )
        .then(
            a.actual_fuel_burn_to_handoff_kg
                .total_cmp(&b.actual_fuel_burn_to_handoff_kg),
        )
        .then(a.row_id.cmp(&b.row_id))
}

pub(crate) fn full_ordinary_matches(
    evidence: &LocalClearingOrdinaryEvidenceV1,
    bounded: &BoundedRunArtifactsV1,
) -> bool {
    evidence.final_state == bounded.final_state
        && evidence.incoming_contact == bounded.incoming_contact
        && evidence.actions == bounded.run.actions
        && evidence.events == bounded.run.events
        && evidence.samples == bounded.run.samples
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::{BoundedRunLimitsV1, run_simulation_bounded};
    const CONTROLLER: &str = "local_clearing_canary_v1";

    // Synthetic incoming states on the 700 m flat input validate contracts.
    // They are not captured mission states or physical obstacle evidence.
    fn synthetic() -> (
        WaypointDirectNominalDirectGenerationRequest,
        RunContext,
        SimulationState,
        LocalClearingGoalV1,
    ) {
        let mut request =
            crate::test_inputs::archived_obstacle_request("fresh_flat_control_span_700");
        request.probe_id = "synthetic_local_contract".into();
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let mut state = SimulationState::new(&context).unwrap();
        state.physics_step = 1200;
        state.sim_time_s = 10.0;
        state.position_m = pd_core::Vec2::new(-650.0, 40.0);
        state.velocity_mps = pd_core::Vec2::new(50.0, 5.0);
        state.attitude_rad = 0.0;
        state.angular_rate_radps = 0.0;
        state.held_command = Command {
            throttle_frac: 0.3,
            target_attitude_rad: 0.0,
        };
        let goal = LocalClearingGoalV1 {
            first_conflict_physics_step: 2500,
            first_conflict_position_m: pd_core::Vec2::new(-650.0, 20.0),
            absolute_deadline_physics_step: 9600,
        };
        (request, context, state, goal)
    }
    fn candidate() -> (
        WaypointDirectNominalDirectGenerationRequest,
        SimulationState,
        LocalClearingRowV1,
        LocalClearingProposalV1,
    ) {
        let (request, context, state, goal) = synthetic();
        let template = LocalClearingPolicyV1::default().templates().unwrap()[21].clone();
        let (row, proposal) = search_row(
            &request,
            &context,
            "synthetic",
            Some(&state),
            (state.physics_step, None),
            &template,
            &goal,
        )
        .unwrap();
        (
            request,
            state,
            row,
            proposal.expect("synthetic local contract candidate"),
        )
    }
    #[test]
    fn powered_entry_is_not_relaxed_airborne_admission() {
        let (_, context, state, goal) = synthetic();
        assert!(entry_rejection(&context, &state, goal.absolute_deadline_physics_step).is_none());
        assert!(live_rejection(&context, &state, goal.absolute_deadline_physics_step).is_some());
        let mut unsafe_state = state.clone();
        unsafe_state.fuel_kg = f64::NAN;
        assert!(entry_rejection(&context, &unsafe_state, 9600).is_some());
    }
    #[test]
    fn source_launch_exception_does_not_apply_to_local_entry() {
        let (request, context, _, _) = synthetic();
        let source = SimulationState::new(&context).unwrap();
        assert!(reserve(&request, &context, &source, "upright", true).is_ok());
        assert!(reserve(&request, &context, &source, "local", false).is_err());
    }
    #[test]
    fn completed_guard_survives_later_reserve_failure_and_requires_first_idle_hold() {
        let (request, state, row, proposal) = candidate();
        assert!(
            row.stop_reason
                .as_deref()
                .unwrap()
                .starts_with("physical_trace: body reserve")
        );
        assert!(
            row.stop_state.unwrap().physics_step > proposal.schedule.continuation_end_physics_step
        );
        assert_eq!(
            proposal.schedule.handoff_physics_step,
            proposal.schedule.powered_end_physics_step + 2
        );
        assert_eq!(proposal.handoff_state.held_command.throttle_frac, 0.0);
        validate_local_clearing_proposal(&request, &state, &proposal).unwrap();
    }
    #[test]
    fn progress_and_unsafe_continuation_are_distinct_rejections() {
        let (request, context, state, mut goal) = synthetic();
        goal.first_conflict_position_m.x = -10.0;
        let template = LocalClearingPolicyV1::default().templates().unwrap()[21].clone();
        let (row, proposal) = search_row(
            &request,
            &context,
            "s",
            Some(&state),
            (state.physics_step, None),
            &template,
            &goal,
        )
        .unwrap();
        assert!(proposal.is_none());
        assert!(
            row.boundaries
                .iter()
                .any(|b| b.status == "insufficient_progress")
        );
        goal.first_conflict_position_m.x = -650.0;
        let mut low = state;
        low.position_m.y = 25.0;
        low.velocity_mps.y = -5.0;
        let (row, proposal) = search_row(
            &request,
            &context,
            "s",
            Some(&low),
            (low.physics_step, None),
            &template,
            &goal,
        )
        .unwrap();
        assert!(proposal.is_none());
        assert!(
            row.boundaries
                .iter()
                .any(|b| b.status == "unsafe_continuation")
        );
    }
    #[test]
    fn safe_coast_that_leaves_replanning_domain_is_not_a_certificate() {
        let (request, context, mut state, mut goal) = synthetic();
        state.position_m = pd_core::Vec2::new(-100.0, 500.0);
        state.velocity_mps = pd_core::Vec2::new(50.0, 5.0);
        goal.first_conflict_position_m.x = -100.0;
        let template = LocalClearingPolicyV1::default().templates().unwrap()[21].clone();
        let (row, proposal) = search_row(
            &request,
            &context,
            "s",
            Some(&state),
            (state.physics_step, None),
            &template,
            &goal,
        )
        .unwrap();
        assert!(proposal.is_none());
        assert!(
            row.boundaries
                .iter()
                .any(|b| b.status == "continuation_unsupported")
        );
    }
    #[test]
    fn full_snapshot_clock_fuel_and_terrain_bindings_are_independent() {
        let (request, state, _, proposal) = candidate();
        let mut reset = state.clone();
        reset.physics_step = 0;
        reset.sim_time_s = 0.0;
        assert!(validate_local_clearing_proposal(&request, &reset, &proposal).is_err());
        let mut fuel_reset = state.clone();
        fuel_reset.fuel_kg += 1.0;
        assert!(validate_local_clearing_proposal(&request, &fuel_reset, &proposal).is_err());
        let mut altered = proposal.clone();
        altered.handoff_state.max_speed_mps += 1.0;
        altered.identity = proposal_identity(&altered).unwrap();
        assert!(validate_local_clearing_proposal(&request, &state, &altered).is_err());
        let mut altered = proposal.clone();
        altered.trajectory[1].state.fuel_kg += 1.0;
        altered.identity = proposal_identity(&altered).unwrap();
        assert!(validate_local_clearing_proposal(&request, &state, &altered).is_err());
        let mut request_twin = request.clone();
        request_twin.scenario.world.gravity_mps2 += 0.01;
        assert!(validate_local_clearing_proposal(&request_twin, &state, &proposal).is_err());
    }
    #[test]
    fn endpoint_coverage_rejects_gap_duplicate_extra_and_missing_idle_hold() {
        let (request, state, _, proposal) = candidate();
        for kind in 0..4 {
            let mut p = proposal.clone();
            match kind {
                0 => {
                    p.schedule.updates.remove(1);
                }
                1 => {
                    p.schedule.updates[1] = p.schedule.updates[0].clone();
                }
                2 => {
                    p.schedule
                        .updates
                        .push(p.schedule.updates.last().unwrap().clone());
                }
                _ => {
                    p.schedule.handoff_physics_step = p.schedule.powered_end_physics_step;
                }
            }
            p.identity = proposal_identity(&p).unwrap();
            assert!(validate_local_clearing_proposal(&request, &state, &p).is_err());
        }
    }
    #[test]
    fn full_contact_option_is_part_of_bounded_proof_comparison() {
        let (_, context, _, _) = synthetic();
        let mut guard = pd_core::AllowAllBoundedRunGuard;
        let bounded = run_simulation_bounded(
            &context,
            CONTROLLER,
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 2,
                hard_end_physics_step: 9600,
            },
            |_, _| {
                Ok(Command {
                    throttle_frac: 0.0,
                    target_attitude_rad: 0.0,
                })
            },
            &mut guard,
        )
        .unwrap();
        let evidence = LocalClearingOrdinaryEvidenceV1 {
            final_state: bounded.final_state.clone(),
            incoming_contact: bounded.incoming_contact.clone(),
            actions: bounded.run.actions.clone(),
            events: bounded.run.events.clone(),
            samples: bounded.run.samples.clone(),
        };
        assert!(full_ordinary_matches(&evidence, &bounded));
        let mut tamper = bounded.clone();
        tamper.incoming_contact = None;
        assert!(evidence.incoming_contact.is_some());
        assert!(!full_ordinary_matches(&evidence, &tamper));
        let mut tamper = bounded;
        tamper.final_state.max_speed_mps += 1.0;
        assert!(!full_ordinary_matches(&evidence, &tamper));
    }
}
