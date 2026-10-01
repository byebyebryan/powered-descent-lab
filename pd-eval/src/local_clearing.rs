//! One-obstruction evaluator experiment. Local physical acceptance and later
//! nominal regeneration are deliberately separate decisions.
use crate::{
    AirborneDirectAuditV1, AirborneDirectSearchV1, BodyAwareTerminalPolicyV1,
    CanonicalInitialDirectCanaryFirstConflictV1, CanonicalInitialDirectCanaryReplayV1,
    CanonicalInitialSearchV1, WaypointDirectNominalDirectGenerationRequest,
    audit_airborne_direct_proposal, audit_canonical_initial_direct,
    canonical_initial_direct::{
        first_conflict_evidence, has_actual_terrain_conflict, phase_at_tick, source_pad_input,
        source_replay_evidence,
    },
    clearing_body_reserve_query, evaluate_airborne_nominal_direct,
    evaluate_canonical_initial_direct, live_rejection,
    nominal_airborne_direct::replay_stitched_from_source,
    nominal_direct_flight_identity, paired_throttle,
};
use anyhow::{Context, Result, bail};
use pd_core::{
    ActionLogEntry, BoundedGuardFailureDispositionV1, BoundedGuardFailureV1, BoundedRunArtifactsV1,
    BoundedRunGuard, BoundedRunLimitsV1, BoundedRunStopCauseV1, Command, EndReason, EventKind,
    EventRecord, FlightProgramUpdateV1, IncomingContactV1, MissionOutcome, PhysicalOutcome,
    RunArtifacts, RunContext, SampleRecord, SimulationState, SimulationStateSnapshotV1,
    replay_simulation, replay_simulation_bounded, run_simulation_bounded,
};
use pd_plan::local_clearing::{
    LocalClearingGoalV1, LocalClearingPolicyV1, LocalClearingScheduleV1, LocalClearingTemplateV1,
    intervention_boundaries,
};
use serde::{Deserialize, Serialize};

const CONTROLLER: &str = "local_clearing_canary_v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingGateV1 {
    pub gate_id: String,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingOrdinaryEvidenceV1 {
    pub final_state: SimulationStateSnapshotV1,
    pub incoming_contact: Option<IncomingContactV1>,
    pub actions: Vec<ActionLogEntry>,
    pub events: Vec<EventRecord>,
    pub samples: Vec<SampleRecord>,
}

struct OrdinaryLive {
    state: SimulationState,
    evidence: LocalClearingOrdinaryEvidenceV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingEntryV1 {
    pub entry_id: String,
    pub requested_physics_step: u64,
    pub admitted: bool,
    pub exclusion_reason: Option<String>,
    pub prefix_updates: Vec<FlightProgramUpdateV1>,
    pub prefix: Option<LocalClearingOrdinaryEvidenceV1>,
    pub prefix_bounded_run: Option<BoundedRunArtifactsV1>,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingProofV1 {
    pub ordinary_live: LocalClearingOrdinaryEvidenceV1,
    pub independent_source_run: BoundedRunArtifactsV1,
    pub official_source_replay: BoundedRunArtifactsV1,
    pub full_state_contact_actions_events_samples_match: bool,
    pub expected_flying_coverage_stop: bool,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingLandingProofV1 {
    pub combined_updates: Vec<FlightProgramUpdateV1>,
    pub live_suffix: LocalClearingOrdinaryEvidenceV1,
    pub ordinary_source: LocalClearingOrdinaryEvidenceV1,
    pub official_replay: RunArtifacts,
    pub passed: bool,
}

/// Independent diagnostic execution of the fixed audit's consumed prefix.
/// An obstructed prefix is run only on a private clone, never on the active H.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingAuditPrefixProofV1 {
    pub combined_updates: Vec<FlightProgramUpdateV1>,
    pub query_branch: LocalClearingOrdinaryEvidenceV1,
    pub ordinary_source: LocalClearingOrdinaryEvidenceV1,
    pub official_replay: RunArtifacts,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingExperimentV1 {
    pub policy: LocalClearingPolicyV1,
    pub flat_request: WaypointDirectNominalDirectGenerationRequest,
    pub obstacle_request: WaypointDirectNominalDirectGenerationRequest,
    pub gates: Vec<LocalClearingGateV1>,
    pub verdict: String,
    pub flat_search: Option<CanonicalInitialSearchV1>,
    pub obstacle_search: Option<CanonicalInitialSearchV1>,
    pub full_canonical_searches_equal: Option<bool>,
    pub flat_audit: Option<AirborneDirectAuditV1>,
    pub flat_source_replay: Option<LocalClearingOrdinaryEvidenceV1>,
    pub flat_official_replay: Option<RunArtifacts>,
    pub obstacle_audit: Option<AirborneDirectAuditV1>,
    pub obstacle_consumed_prefix_replay: Option<CanonicalInitialDirectCanaryReplayV1>,
    pub first_conflict: Option<CanonicalInitialDirectCanaryFirstConflictV1>,
    pub entries: Vec<LocalClearingEntryV1>,
    pub rows: Vec<LocalClearingRowV1>,
    pub selected: Option<LocalClearingProposalV1>,
    pub handoff_proof: Option<LocalClearingProofV1>,
    pub continuation_proof: Option<LocalClearingProofV1>,
    pub regeneration: Option<AirborneDirectSearchV1>,
    pub regeneration_audit: Option<AirborneDirectAuditV1>,
    pub regeneration_consumed_prefix_proof: Option<LocalClearingAuditPrefixProofV1>,
    pub landing_proof: Option<LocalClearingLandingProofV1>,
    /// Last explicitly executed branch for failure diagnostics. The active
    /// handoff remains in handoff_proof; its separate continuation branch is
    /// never consumed before calling regeneration from the retained H.
    pub last_execution_branch_evidence: Option<LocalClearingOrdinaryEvidenceV1>,
}

fn context_identity(request: &WaypointDirectNominalDirectGenerationRequest) -> Result<String> {
    nominal_direct_flight_identity(&(LocalClearingPolicyV1::default(), request))
}

fn proposal_identity(proposal: &LocalClearingProposalV1) -> Result<String> {
    let mut canonical = proposal.clone();
    canonical.identity.clear();
    nominal_direct_flight_identity(&canonical)
}

fn snapshot_finite(state: &SimulationState) -> bool {
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

fn entry_rejection(context: &RunContext, state: &SimulationState, deadline: u64) -> Option<String> {
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

fn new_ordinary(context: &RunContext) -> Result<OrdinaryLive> {
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

fn advance_ordinary(
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

struct LocalGuard<'a> {
    request: &'a WaypointDirectNominalDirectGenerationRequest,
    canonical: &'a [FlightProgramUpdateV1],
    entry_tick: u64,
}

impl BoundedRunGuard for LocalGuard<'_> {
    fn initial(
        &mut self,
        ctx: &RunContext,
        state: &SimulationState,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        self.check(ctx, state, None)
    }
    fn before_transition(
        &mut self,
        ctx: &RunContext,
        state: &SimulationState,
        _: Command,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        self.check(ctx, state, None)
    }
    fn after_transition(
        &mut self,
        ctx: &RunContext,
        state: &SimulationState,
        contact: Option<&IncomingContactV1>,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        self.check(ctx, state, contact)
    }
}
impl LocalGuard<'_> {
    fn check(
        &self,
        ctx: &RunContext,
        state: &SimulationState,
        contact: Option<&IncomingContactV1>,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        let source = state.physics_step < self.entry_tick;
        let phase = if state.physics_step == 0 {
            "upright".into()
        } else {
            phase_at_tick(self.canonical, state.physics_step)
        };
        let error = if contact.is_some() {
            Some("unexpected incoming contact".into())
        } else {
            reserve(
                self.request,
                ctx,
                state,
                if source { &phase } else { "local" },
                source,
            )
            .err()
            .map(|e| e.to_string())
        };
        if let Some(error) = error {
            return Err(BoundedGuardFailureV1::new(
                BoundedGuardFailureDispositionV1::SafetyRejected,
                error,
            ));
        }
        Ok(())
    }
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

fn search_row(
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

fn local_rank(a: &LocalClearingProposalV1, b: &LocalClearingProposalV1) -> std::cmp::Ordering {
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

fn full_ordinary_matches(
    evidence: &LocalClearingOrdinaryEvidenceV1,
    bounded: &BoundedRunArtifactsV1,
) -> bool {
    evidence.final_state == bounded.final_state
        && evidence.incoming_contact == bounded.incoming_contact
        && evidence.actions == bounded.run.actions
        && evidence.events == bounded.run.events
        && evidence.samples == bounded.run.samples
}

fn flying_coverage(bounded: &BoundedRunArtifactsV1, endpoint: u64) -> bool {
    bounded.stop == BoundedRunStopCauseV1::CoverageExhausted
        && bounded.coverage_reached
        && !bounded.hard_end_reached
        && bounded.failure.is_none()
        && bounded.final_state.physics_step == endpoint
        && bounded.incoming_contact.is_none()
        && bounded.final_state.physical_outcome == PhysicalOutcome::Flying
        && bounded.final_state.mission_outcome == MissionOutcome::InProgress
        && bounded.final_state.end_reason == EndReason::Running
}

fn prove_endpoint(
    request: &WaypointDirectNominalDirectGenerationRequest,
    context: &RunContext,
    canonical: &[FlightProgramUpdateV1],
    entry_tick: u64,
    updates: &[FlightProgramUpdateV1],
    live: &OrdinaryLive,
    expected: &SimulationStateSnapshotV1,
) -> Result<LocalClearingProofV1> {
    let endpoint = expected.physics_step;
    let limits = BoundedRunLimitsV1 {
        command_coverage_end_physics_step: endpoint,
        hard_end_physics_step: (request
            .policy
            .analytical_policy
            .mission_budget_s()
            .min(context.sim.max_time_s)
            * 120.0)
            .floor() as u64,
    };
    let mut index = 0;
    let mut guard = LocalGuard {
        request,
        canonical,
        entry_tick,
    };
    let source = run_simulation_bounded(
        context,
        CONTROLLER,
        limits,
        |_, observation| {
            let update = updates
                .get(index)
                .ok_or_else(|| "source bounded command coverage exhausted".to_string())?;
            if update.physics_step != observation.physics_step {
                return Err("source bounded global update gap".into());
            }
            index += 1;
            Ok(update.command)
        },
        &mut guard,
    )?;
    let mut replay_guard = LocalGuard {
        request,
        canonical,
        entry_tick,
    };
    let replay = replay_simulation_bounded(
        context,
        CONTROLLER,
        &live.evidence.actions,
        limits,
        &mut replay_guard,
    )?;
    let matches = index == updates.len()
        && &live.evidence.final_state == expected
        && full_ordinary_matches(&live.evidence, &source)
        && full_ordinary_matches(&live.evidence, &replay)
        && source == replay;
    let coverage = flying_coverage(&source, endpoint) && flying_coverage(&replay, endpoint);
    Ok(LocalClearingProofV1 {
        ordinary_live: live.evidence.clone(),
        independent_source_run: source,
        official_source_replay: replay,
        full_state_contact_actions_events_samples_match: matches,
        expected_flying_coverage_stop: coverage,
        passed: matches && coverage,
    })
}

fn fail_gate(
    experiment: &mut LocalClearingExperimentV1,
    index: usize,
    verdict: &str,
    reason: impl Into<String>,
) {
    experiment.gates[index].status = "failed".into();
    experiment.gates[index].reason = Some(reason.into());
    experiment.verdict = verdict.into();
}
fn pass_gate(experiment: &mut LocalClearingExperimentV1, index: usize) {
    experiment.gates[index].status = "passed".into();
}

fn prove_fixed_audit_prefix(
    context: &RunContext,
    handoff: &OrdinaryLive,
    prefix_updates: &[FlightProgramUpdateV1],
    suffix_updates: &[FlightProgramUpdateV1],
    audit: &AirborneDirectAuditV1,
) -> Result<LocalClearingAuditPrefixProofV1> {
    let consumed: Vec<_> = suffix_updates
        .iter()
        .take_while(|update| update.physics_step < audit.final_state.physics_step)
        .cloned()
        .collect();
    let mut query = OrdinaryLive {
        state: handoff.state.clone(),
        evidence: handoff.evidence.clone(),
    };
    advance_ordinary(
        context,
        &mut query,
        &consumed,
        audit.final_state.physics_step,
    )?;
    let mut combined_updates = prefix_updates.to_vec();
    combined_updates.extend(consumed);
    let source = replay_stitched_from_source(context, &combined_updates)?;
    let official_replay = replay_simulation(context, CONTROLLER, &source.actions)?;
    let ordinary_source = LocalClearingOrdinaryEvidenceV1 {
        final_state: source.final_state,
        incoming_contact: source.incoming_contact,
        actions: source.actions,
        events: source.events,
        samples: source.samples,
    };
    let passed = query.evidence == ordinary_source
        && query.evidence.final_state == audit.final_state
        && query.evidence.incoming_contact == audit.incoming_contact
        && official_replay.actions == ordinary_source.actions
        && official_replay.events == ordinary_source.events
        && official_replay.samples == ordinary_source.samples;
    Ok(LocalClearingAuditPrefixProofV1 {
        combined_updates,
        query_branch: query.evidence,
        ordinary_source,
        official_replay,
        passed,
    })
}

fn fixed_regeneration_outcome(
    audit: &AirborneDirectAuditV1,
    consumed_prefix_proven: bool,
) -> &'static str {
    if !consumed_prefix_proven {
        return "EvidenceFailure";
    }
    if audit.passed
        && audit.safe_target_contact
        && audit.ordinary_neutral_parity
        && audit.commands_match
    {
        return "Direct";
    }
    let finite_reserve_conflict = audit
        .clearance_scan
        .first_violation
        .as_ref()
        .is_some_and(|v| {
            v.clearance_m
                .is_some_and(|c| c.is_finite() && c < v.required_clearance_m)
        });
    let mut contact_only = audit.clone();
    contact_only.clearance_scan.first_violation = None;
    if (finite_reserve_conflict || has_actual_terrain_conflict(&contact_only))
        && audit.ordinary_neutral_parity
        // commands_match means the entire nominal endpoint was reached. A
        // terrain contact legitimately prevents it; prove consumed commands
        // independently instead of demanding the obstructed future endpoint.
        && consumed_prefix_proven
    {
        "TerrainBlocked"
    } else {
        "EvidenceFailure"
    }
}

/// All physical requests must be sealed by the caller before this is invoked.
/// Finite failures return complete partial evidence rather than dropping it.
pub fn evaluate_local_clearing_experiment(
    flat_request: WaypointDirectNominalDirectGenerationRequest,
    obstacle_request: WaypointDirectNominalDirectGenerationRequest,
) -> Result<LocalClearingExperimentV1> {
    let mut e = LocalClearingExperimentV1 {
        policy: LocalClearingPolicyV1::default(),
        flat_request,
        obstacle_request,
        gates: [
            "baseline_and_entries",
            "local_generation",
            "local_physical_proof",
            "handoff_compatibility",
            "closure_and_preservation",
        ]
        .into_iter()
        .map(|gate_id| LocalClearingGateV1 {
            gate_id: gate_id.into(),
            status: "not_evaluated".into(),
            reason: None,
        })
        .collect(),
        verdict: "not_evaluated".into(),
        flat_search: None,
        obstacle_search: None,
        full_canonical_searches_equal: None,
        flat_audit: None,
        flat_source_replay: None,
        flat_official_replay: None,
        obstacle_audit: None,
        obstacle_consumed_prefix_replay: None,
        first_conflict: None,
        entries: Vec::new(),
        rows: Vec::new(),
        selected: None,
        handoff_proof: None,
        continuation_proof: None,
        regeneration: None,
        regeneration_audit: None,
        regeneration_consumed_prefix_proof: None,
        landing_proof: None,
        last_execution_branch_evidence: None,
    };
    macro_rules! gated {
        ($result:expr, $index:expr, $verdict:expr, $stage:expr) => {
            match $result {
                Ok(value) => value,
                Err(error) => {
                    fail_gate(&mut e, $index, $verdict, format!("{}: {error}", $stage));
                    return Ok(e);
                }
            }
        };
    }
    e.policy.validate().map_err(anyhow::Error::msg)?;
    let flat_context =
        RunContext::from_scenario(&e.flat_request.scenario).map_err(anyhow::Error::msg)?;
    let context =
        RunContext::from_scenario(&e.obstacle_request.scenario).map_err(anyhow::Error::msg)?;
    let flat_search = gated!(
        evaluate_canonical_initial_direct(&e.flat_request),
        0,
        "BaselineEvidenceFailure",
        "flat generation"
    );
    e.flat_search = Some(flat_search.clone());
    let obstacle_search = gated!(
        evaluate_canonical_initial_direct(&e.obstacle_request),
        0,
        "BaselineEvidenceFailure",
        "obstacle generation"
    );
    let equal = flat_search == obstacle_search;
    e.full_canonical_searches_equal = Some(equal);
    e.flat_search = Some(flat_search.clone());
    e.obstacle_search = Some(obstacle_search.clone());
    let (Some(flat_proposal), Some(canonical)) = (&flat_search.selected, &obstacle_search.selected)
    else {
        fail_gate(
            &mut e,
            0,
            "BaselineUnknown",
            "canonical initial finite family has no proposal",
        );
        return Ok(e);
    };
    let flat_audit = audit_canonical_initial_direct(
        &flat_context,
        &source_pad_input(&flat_context, &e.flat_request.source_pad_id)?,
        flat_proposal,
        e.policy.minimum_clearance_m,
    )?;
    let obstacle_audit = audit_canonical_initial_direct(
        &context,
        &source_pad_input(&context, &e.obstacle_request.source_pad_id)?,
        canonical,
        e.policy.minimum_clearance_m,
    )?;
    e.flat_audit = Some(flat_audit.clone());
    e.obstacle_audit = Some(obstacle_audit.clone());
    let flat_updates: Vec<_> = flat_proposal
        .updates
        .iter()
        .take_while(|u| u.physics_step < flat_audit.final_state.physics_step)
        .cloned()
        .collect();
    let flat_source = gated!(
        replay_stitched_from_source(&flat_context, &flat_updates),
        0,
        "BaselineEvidenceFailure",
        "flat ordinary source replay"
    );
    let official_flat = gated!(
        replay_simulation(&flat_context, CONTROLLER, &flat_source.actions),
        0,
        "BaselineEvidenceFailure",
        "flat official replay"
    );
    let flat_parity = flat_source.final_state == flat_audit.final_state
        && flat_source.incoming_contact == flat_audit.incoming_contact
        && official_flat.actions == flat_source.actions
        && official_flat.events == flat_source.events
        && official_flat.samples == flat_source.samples;
    e.flat_source_replay = Some(LocalClearingOrdinaryEvidenceV1 {
        final_state: flat_source.final_state,
        incoming_contact: flat_source.incoming_contact,
        actions: flat_source.actions,
        events: flat_source.events,
        samples: flat_source.samples,
    });
    e.flat_official_replay = Some(official_flat);
    let first_conflict = first_conflict_evidence(&context, &canonical.updates, &obstacle_audit)?;
    let obstacle_prefix = source_replay_evidence(&context, &canonical.updates, &obstacle_audit);
    let obstacle_prefix_matches = obstacle_prefix.passed;
    e.obstacle_consumed_prefix_replay = Some(obstacle_prefix);
    let baseline_ok = equal
        && flat_audit.passed
        && flat_audit.safe_target_contact
        && flat_parity
        && fixed_regeneration_outcome(&obstacle_audit, obstacle_prefix_matches) == "TerrainBlocked"
        && first_conflict.is_some();
    e.flat_audit = Some(flat_audit);
    e.obstacle_audit = Some(obstacle_audit);
    e.first_conflict = first_conflict.clone();
    if !baseline_ok {
        fail_gate(
            &mut e,
            0,
            "BaselineEvidenceFailure",
            "canonical twin, safe control, actual conflict or replay/parity mismatch",
        );
        return Ok(e);
    }
    let first_conflict = first_conflict.unwrap();
    let goal = LocalClearingGoalV1 {
        first_conflict_physics_step: first_conflict.physics_step,
        first_conflict_position_m: first_conflict.position_m,
        absolute_deadline_physics_step: canonical.absolute_deadline_physics_step,
    };
    let mut entries = Vec::<Option<OrdinaryLive>>::new();
    for (entry_id, tick) in intervention_boundaries(canonical.source_handoff_physics_step)
        .map_err(anyhow::Error::msg)?
    {
        let prefix_updates: Vec<_> = canonical
            .updates
            .iter()
            .take_while(|u| u.physics_step < tick)
            .cloned()
            .collect();
        let mut live = new_ordinary(&context)?;
        let result = advance_ordinary(&context, &mut live, &prefix_updates, tick);
        let exclusion = result
            .err()
            .map(|e| e.to_string())
            .or_else(|| goal.validate(tick).err())
            .or_else(|| entry_rejection(&context, &live.state, goal.absolute_deadline_physics_step))
            .or_else(|| {
                reserve(&e.obstacle_request, &context, &live.state, "local", false)
                    .err()
                    .map(|e| e.to_string())
            });
        // Independently guard every preceding canonical state, including the
        // legitimate source launch exception, before accepting the entry.
        let mut prefix_guard = LocalGuard {
            request: &e.obstacle_request,
            canonical: &canonical.updates,
            entry_tick: tick,
        };
        let mut update_index = 0;
        let prefix_run = run_simulation_bounded(
            &context,
            CONTROLLER,
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: tick,
                hard_end_physics_step: goal.absolute_deadline_physics_step,
            },
            |_, obs| {
                let update = prefix_updates
                    .get(update_index)
                    .ok_or_else(|| "prefix missing command".to_string())?;
                if update.physics_step != obs.physics_step {
                    return Err("prefix clock mismatch".into());
                }
                update_index += 1;
                Ok(update.command)
            },
            &mut prefix_guard,
        )?;
        let exclusion = exclusion.or_else(|| {
            (!flying_coverage(&prefix_run, tick)
                || !full_ordinary_matches(&live.evidence, &prefix_run))
            .then(|| {
                format!(
                    "canonical prefix guard/parity failed: {:?}",
                    prefix_run.failure
                )
            })
        });
        let admitted = exclusion.is_none();
        e.entries.push(LocalClearingEntryV1 {
            entry_id,
            requested_physics_step: tick,
            admitted,
            exclusion_reason: exclusion,
            prefix_updates,
            prefix: Some(live.evidence.clone()),
            prefix_bounded_run: Some(prefix_run),
        });
        entries.push(admitted.then_some(live));
    }
    if entries.iter().all(Option::is_none) {
        fail_gate(
            &mut e,
            0,
            "NoSupportedIntervention",
            "all sealed actual intervention states were excluded",
        );
        return Ok(e);
    }
    pass_gate(&mut e, 0);
    let mut proposals = Vec::new();
    for (i, entry) in e.entries.iter().enumerate() {
        for template in e.policy.templates().map_err(anyhow::Error::msg)? {
            let (row, proposal) = search_row(
                &e.obstacle_request,
                &context,
                &entry.entry_id,
                entries[i].as_ref().map(|l| &l.state),
                (
                    entry.requested_physics_step,
                    entry.exclusion_reason.as_deref(),
                ),
                &template,
                &goal,
            )?;
            e.rows.push(row);
            if let Some(p) = proposal {
                proposals.push(p);
            }
        }
    }
    proposals.sort_by(local_rank);
    let Some(selected) = proposals.into_iter().next() else {
        fail_gate(
            &mut e,
            1,
            "LocalUnknown",
            "sealed local family exhausted; not a physical impossibility proof",
        );
        return Ok(e);
    };
    e.selected = Some(selected.clone());
    pass_gate(&mut e, 1);
    let index = e
        .entries
        .iter()
        .position(|entry| entry.requested_physics_step == selected.schedule.entry_physics_step)
        .unwrap();
    let mut live = entries[index].take().unwrap();
    gated!(
        validate_local_clearing_proposal(&e.obstacle_request, &live.state, &selected),
        2,
        "LocalEvidenceFailure",
        "local proposal validation"
    );
    let local_handoff_updates: Vec<_> = selected
        .schedule
        .updates
        .iter()
        .take_while(|u| u.physics_step < selected.schedule.handoff_physics_step)
        .cloned()
        .collect();
    let run = advance_ordinary(
        &context,
        &mut live,
        &local_handoff_updates,
        selected.schedule.handoff_physics_step,
    );
    live.evidence.final_state = SimulationStateSnapshotV1::from_state(&live.state);
    e.last_execution_branch_evidence = Some(live.evidence.clone());
    gated!(
        run,
        2,
        "LocalEvidenceFailure",
        "ordinary live handoff execution"
    );
    let mut combined_updates = e.entries[index].prefix_updates.clone();
    combined_updates.extend(local_handoff_updates);
    let handoff_proof = gated!(
        prove_endpoint(
            &e.obstacle_request,
            &context,
            &canonical.updates,
            selected.schedule.entry_physics_step,
            &combined_updates,
            &live,
            &selected.handoff_state,
        ),
        2,
        "LocalEvidenceFailure",
        "handoff source bounded proof"
    );
    e.handoff_proof = Some(handoff_proof.clone());
    let mut continuation = OrdinaryLive {
        state: live.state.clone(),
        evidence: live.evidence.clone(),
    };
    let continuation_updates: Vec<_> = selected
        .schedule
        .updates
        .iter()
        .filter(|u| u.physics_step >= selected.schedule.handoff_physics_step)
        .cloned()
        .collect();
    let run = advance_ordinary(
        &context,
        &mut continuation,
        &continuation_updates,
        selected.schedule.continuation_end_physics_step,
    );
    continuation.evidence.final_state = SimulationStateSnapshotV1::from_state(&continuation.state);
    e.last_execution_branch_evidence = Some(continuation.evidence.clone());
    gated!(
        run,
        2,
        "LocalEvidenceFailure",
        "ordinary live certificate execution"
    );
    let mut guard_combined = combined_updates.clone();
    guard_combined.extend(continuation_updates);
    let continuation_proof = gated!(
        prove_endpoint(
            &e.obstacle_request,
            &context,
            &canonical.updates,
            selected.schedule.entry_physics_step,
            &guard_combined,
            &continuation,
            &selected.continuation_end_state,
        ),
        2,
        "LocalEvidenceFailure",
        "certificate source bounded proof"
    );
    let proof_ok = handoff_proof.passed && continuation_proof.passed;
    e.handoff_proof = Some(handoff_proof);
    e.continuation_proof = Some(continuation_proof);
    if !proof_ok {
        fail_gate(
            &mut e,
            2,
            "LocalEvidenceFailure",
            "selected live/whole-source bounded/replay/coverage parity failed",
        );
        return Ok(e);
    }
    pass_gate(&mut e, 2);
    let regeneration = gated!(
        evaluate_airborne_nominal_direct(
            &context,
            &live.state,
            goal.absolute_deadline_physics_step,
        ),
        3,
        "CompositionEvidenceFailure",
        "fresh nominal regeneration"
    );
    e.regeneration = Some(regeneration.clone());
    let Some(proposal) = &regeneration.selected else {
        fail_gate(&mut e,3,"CompositionUnknown",regeneration.unsupported_reason.clone().unwrap_or_else(||"unchanged airborne finite family exhausted from actual locally accepted handoff".into()));
        return Ok(e);
    };
    let audit = gated!(
        audit_airborne_direct_proposal(
            &context,
            &live.state,
            proposal,
            e.policy.minimum_clearance_m,
        ),
        3,
        "CompositionEvidenceFailure",
        "fixed actual-terrain audit"
    );
    e.regeneration_audit = Some(audit.clone());
    let consumed_prefix = gated!(
        prove_fixed_audit_prefix(
            &context,
            &live,
            &combined_updates,
            &proposal.updates,
            &audit
        ),
        3,
        "CompositionEvidenceFailure",
        "fixed audit consumed-prefix proof"
    );
    let outcome = fixed_regeneration_outcome(&audit, consumed_prefix.passed);
    e.regeneration_consumed_prefix_proof = Some(consumed_prefix);
    if outcome == "Direct" {
        let suffix: Vec<_> = proposal
            .updates
            .iter()
            .take_while(|u| u.physics_step < audit.final_state.physics_step)
            .cloned()
            .collect();
        let run = advance_ordinary(&context, &mut live, &suffix, audit.final_state.physics_step);
        live.evidence.final_state = SimulationStateSnapshotV1::from_state(&live.state);
        e.last_execution_branch_evidence = Some(live.evidence.clone());
        gated!(
            run,
            3,
            "CompositionEvidenceFailure",
            "ordinary Direct suffix execution"
        );
        combined_updates.extend(suffix);
        let source = gated!(
            replay_stitched_from_source(&context, &combined_updates),
            3,
            "CompositionEvidenceFailure",
            "complete real-prefix source landing replay"
        );
        let official = gated!(
            replay_simulation(&context, CONTROLLER, &source.actions),
            3,
            "CompositionEvidenceFailure",
            "official landing replay"
        );
        let source_evidence = LocalClearingOrdinaryEvidenceV1 {
            final_state: source.final_state,
            incoming_contact: source.incoming_contact,
            actions: source.actions,
            events: source.events,
            samples: source.samples,
        };
        let passed = source_evidence == live.evidence
            && live.evidence.final_state == audit.final_state
            && live.evidence.incoming_contact == audit.incoming_contact
            && official.actions == source_evidence.actions
            && official.events == source_evidence.events
            && official.samples == source_evidence.samples
            && official.manifest.physical_outcome == PhysicalOutcome::LandedOnTarget
            && official.manifest.mission_outcome == MissionOutcome::Success;
        e.landing_proof = Some(LocalClearingLandingProofV1 {
            combined_updates,
            live_suffix: live.evidence,
            ordinary_source: source_evidence,
            official_replay: official,
            passed,
        });
        if !passed {
            fail_gate(
                &mut e,
                3,
                "CompositionEvidenceFailure",
                "Direct suffix failed complete combined-prefix landing replay parity",
            );
            return Ok(e);
        }
        e.verdict = "LocalClearingThenDirect".into();
        pass_gate(&mut e, 3);
    } else if outcome == "TerrainBlocked" {
        e.verdict = "LocalClearingThenTerrainBlocked".into();
        pass_gate(&mut e, 3);
    } else {
        fail_gate(
            &mut e,
            3,
            "CompositionEvidenceFailure",
            "fixed audit did not land and lacks a parity-backed actual terrain conflict",
        );
    }
    Ok(e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // Synthetic incoming states on the 700 m flat input validate contracts.
    // They are not captured mission states or physical obstacle evidence.
    fn synthetic() -> (
        WaypointDirectNominalDirectGenerationRequest,
        RunContext,
        SimulationState,
        LocalClearingGoalV1,
    ) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let manifest =
            crate::load_waypoint_direct_obstacle_discrimination_fresh_manifest(root).unwrap();
        let case = &manifest.cases[0];
        let request = WaypointDirectNominalDirectGenerationRequest {
            scenario: case.scenario.clone(),
            source_pad_id: case.source_pad_id.clone(),
            target_pad_id: case.target_pad_id.clone(),
            probe_id: "synthetic_local_contract".into(),
            policy: manifest.generation_policy,
        };
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
    #[test]
    fn terrain_blockage_is_not_an_unrelated_query_or_parity_error_or_rerank() {
        let (_, _, _, p) = candidate();
        let mut q = p.clone();
        q.schedule.entry_physics_step -= 2;
        let before = local_rank(&p, &q);
        let mut audit = AirborneDirectAuditV1 {
            passed: false,
            safe_target_contact: false,
            ordinary_neutral_parity: true,
            commands_match: true,
            first_contact: None,
            incoming_contact: None,
            final_state: p.handoff_state.clone(),
            rejection_reasons: Vec::new(),
            clearance_scan: crate::GeometryClearanceScanEvidence {
                poststep_state_count: 1,
                airborne_state_count: 1,
                source_corridor_state_count: 0,
                terminal_corridor_state_count: 0,
                exact_clearance_query_count: 1,
                all_airborne_states_passed: false,
                minimum_airborne: None,
                first_violation: Some(crate::GeometryClearanceViolationEvidence {
                    physics_step: 3000,
                    phase: "coast".into(),
                    reason: "query domain error".into(),
                    clearance_m: None,
                    required_clearance_m: 5.0,
                    corridor: "none".into(),
                }),
            },
        };
        assert_eq!(fixed_regeneration_outcome(&audit, true), "EvidenceFailure");
        audit
            .clearance_scan
            .first_violation
            .as_mut()
            .unwrap()
            .clearance_m = Some(2.0);
        // Early terrain contact need not reach the unexecutable full endpoint.
        audit.commands_match = false;
        assert_eq!(fixed_regeneration_outcome(&audit, true), "TerrainBlocked");
        assert_eq!(fixed_regeneration_outcome(&audit, false), "EvidenceFailure");
        assert_eq!(local_rank(&p, &q), before);
        audit.ordinary_neutral_parity = false;
        assert_eq!(fixed_regeneration_outcome(&audit, true), "EvidenceFailure");
        audit.ordinary_neutral_parity = true;
        audit.passed = true;
        audit.safe_target_contact = true;
        audit.commands_match = true;
        audit.clearance_scan.first_violation = None;
        assert_eq!(fixed_regeneration_outcome(&audit, true), "Direct");
        assert_eq!(fixed_regeneration_outcome(&audit, false), "EvidenceFailure");
    }

    #[test]
    fn truncated_prefix_checks_commands_and_full_contact_not_nominal_completion() {
        let (_, context, _, _) = synthetic();
        // A synthetic idle source-pad prefix contacts immediately. It tests
        // evidence truncation only, not the 900 m clearing experiment.
        let updates = vec![FlightProgramUpdateV1 {
            physics_step: 0,
            phase: "synthetic_idle_contact".into(),
            command: Command {
                throttle_frac: 0.0,
                target_attitude_rad: 0.0,
            },
        }];
        let source = replay_stitched_from_source(&context, &updates).unwrap();
        let audit = AirborneDirectAuditV1 {
            passed: false,
            safe_target_contact: false,
            ordinary_neutral_parity: true,
            commands_match: false,
            first_contact: None,
            incoming_contact: source.incoming_contact.clone(),
            final_state: source.final_state,
            rejection_reasons: Vec::new(),
            clearance_scan: crate::GeometryClearanceScanEvidence {
                poststep_state_count: 1,
                airborne_state_count: 0,
                source_corridor_state_count: 0,
                terminal_corridor_state_count: 0,
                exact_clearance_query_count: 0,
                all_airborne_states_passed: true,
                minimum_airborne: None,
                first_violation: None,
            },
        };
        assert!(source_replay_evidence(&context, &updates, &audit).passed);
        let mut changed = updates.clone();
        changed[0].command.target_attitude_rad = 0.1;
        assert!(!source_replay_evidence(&context, &changed, &audit).passed);
        let mut changed = audit;
        changed
            .incoming_contact
            .as_mut()
            .unwrap()
            .state
            .max_speed_mps += 1.0;
        assert!(!source_replay_evidence(&context, &updates, &changed).passed);
    }
}
