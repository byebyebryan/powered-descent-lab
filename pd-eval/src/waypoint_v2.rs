//! Opt-in piecewise ballistic flight. Nominal generation is terrain-blind;
//! corrections are selected locally and never scored by a landing suffix.
use std::{collections::BTreeMap, time::Instant};

use anyhow::{Context, Result, bail};
use pd_core::{
    BoundedGuardFailureDispositionV1, BoundedGuardFailureV1, BoundedRunArtifactsV1,
    BoundedRunGuard, BoundedRunLimitsV1, BoundedRunStopCauseV1, Command, EndReason,
    FlightProgramUpdateV1, IncomingContactV1, MissionOutcome, PhysicalOutcome, RunContext,
    SimulationState, SimulationStateSnapshotV1, replay_simulation_bounded, run_simulation_bounded,
};
use pd_plan::{
    local_clearing::{LocalClearingGoalV1, LocalClearingPolicyV1},
    waypoint_v2::{
        WaypointV2Policy, WaypointV2Stop, command_count, later_intervention_boundaries,
        original_deadline,
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    AirborneDirectAuditV1, BodyAwareTerminalPolicyV1, LocalClearingOrdinaryEvidenceV1,
    LocalClearingProposalV1, NominalDirectFlightDecisionV1,
    WaypointDirectNominalDirectGenerationRequest, audit_airborne_acquisition_proposal,
    audit_airborne_direct_proposal, audit_canonical_initial_direct,
    canonical_initial_direct::{has_actual_terrain_conflict, phase_at_tick, source_pad_input},
    clearing_body_reserve_query, evaluate_airborne_acquisition_direct,
    evaluate_airborne_nominal_direct, evaluate_canonical_initial_direct,
    local_clearing::{
        OrdinaryLive, advance_ordinary, entry_rejection, full_ordinary_matches, local_rank,
        new_ordinary, search_row, snapshot_finite,
    },
    nominal_body_reserve_query, nominal_direct_flight_identity, preflight_nominal_direct_flight,
    validate_local_clearing_proposal,
};

const CONTROLLER: &str = "waypoint_v2_supplied_commands";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2Preflight {
    pub supported: bool,
    pub rejection: Option<WaypointV2Stop>,
    pub reason: Option<String>,
    pub simulation_created: bool,
}

pub fn preflight_waypoint_v2_flight(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &WaypointV2Policy,
) -> WaypointV2Preflight {
    let base = preflight_nominal_direct_flight(request, &BodyAwareTerminalPolicyV1::default());
    let (rejection, reason) = if let Some(decision) = base.rejection {
        match decision {
            NominalDirectFlightDecisionV1::Invalid { reason } => {
                (Some(WaypointV2Stop::InvalidInput), Some(reason))
            }
            NominalDirectFlightDecisionV1::Unsupported { reason } => {
                (Some(WaypointV2Stop::Unsupported), Some(reason))
            }
            _ => unreachable!("input-only preflight cannot return a flight"),
        }
    } else if let Err(reason) = policy.validate() {
        (Some(WaypointV2Stop::Unsupported), Some(reason))
    } else if request.scenario.mission.transfer_route.is_some() {
        (
            Some(WaypointV2Stop::Unsupported),
            Some("V2 requires route-free LandingOnPad, including no empty authored route".into()),
        )
    } else {
        (None, None)
    };
    WaypointV2Preflight {
        supported: rejection.is_none(),
        rejection,
        reason,
        simulation_created: false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointV2SegmentKind {
    InitialNominal,
    LocalCorrection,
    AirborneNominal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2Segment {
    pub kind: WaypointV2SegmentKind,
    pub start_physics_step: u64,
    pub end_physics_step: u64,
    pub proposal_identity: String,
    pub updates: Vec<FlightProgramUpdateV1>,
    pub entry_state: SimulationStateSnapshotV1,
    pub end_state: SimulationStateSnapshotV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2Entry {
    pub entry_id: String,
    pub physics_step: u64,
    pub admitted: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2LocalSearch {
    pub entries: Vec<WaypointV2Entry>,
    pub row_count: usize,
    pub boundary_count: usize,
    pub accepted_row_count: usize,
    pub row_status_counts: BTreeMap<String, usize>,
    pub row_stop_reason_counts: BTreeMap<String, usize>,
    pub boundary_status_counts: BTreeMap<String, usize>,
    pub selected: Option<LocalClearingProposalV1>,
    pub certificate_state: Option<SimulationStateSnapshotV1>,
    pub handoff_source_replay_passed: bool,
    pub certificate_source_replay_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointV2CycleDecision {
    NoNominal,
    Unsupported,
    NominalRejected,
    Direct,
    TerrainBlocked,
    LocalCleared,
    CorrectionLimit,
    NoClearing,
    NoProgress,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2Cycle {
    pub cycle_index: usize,
    pub current_state: SimulationStateSnapshotV1,
    pub nominal_search_identity: String,
    pub nominal_proposal_identity: Option<String>,
    pub nominal_peak_com_height_m: Option<f64>,
    pub nominal_attempt_status_counts: BTreeMap<String, usize>,
    pub nominal_rejection_reason_counts: BTreeMap<String, usize>,
    pub nominal_updates: Vec<FlightProgramUpdateV1>,
    pub audit: Option<AirborneDirectAuditV1>,
    pub fixed_consumed_prefix_proven: bool,
    pub decision: WaypointV2CycleDecision,
    pub conflict_state: Option<SimulationStateSnapshotV1>,
    pub conflict_incoming_contact: Option<IncomingContactV1>,
    pub local_search: Option<WaypointV2LocalSearch>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2Timings {
    pub planning_s: f64,
    pub execution_s: f64,
    pub replay_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2FlightResult {
    pub policy: WaypointV2Policy,
    pub input_identity: String,
    pub planning_stop: WaypointV2Stop,
    pub reason: Option<String>,
    pub correction_count: u32,
    pub initial_nominal_terrain_blocked: bool,
    pub integrity_passed: bool,
    pub physical_outcome: Option<PhysicalOutcome>,
    pub mission_outcome: Option<MissionOutcome>,
    pub absolute_deadline_physics_step: Option<u64>,
    pub cycles: Vec<WaypointV2Cycle>,
    pub segments: Vec<WaypointV2Segment>,
    pub ordinary_flight: Option<LocalClearingOrdinaryEvidenceV1>,
    pub final_source_replay_passed: bool,
    pub manifest: Option<pd_core::RunManifest>,
    pub failed_local_row: Option<crate::LocalClearingRowV1>,
    pub timings: WaypointV2Timings,
}

fn consumed(updates: &[FlightProgramUpdateV1], end: u64) -> Vec<FlightProgramUpdateV1> {
    updates
        .iter()
        .take_while(|u| u.physics_step < end)
        .cloned()
        .collect()
}

fn ledger_updates(segments: &[WaypointV2Segment]) -> Vec<FlightProgramUpdateV1> {
    segments
        .iter()
        .flat_map(|s| s.updates.iter().cloned())
        .collect()
}

/// Segment ownership uses the command strictly before a post-step boundary.
/// The source exemption can belong only to the very first nominal segment.
struct FlightGuard<'a> {
    request: &'a WaypointDirectNominalDirectGenerationRequest,
    segments: &'a [WaypointV2Segment],
    diagnostic: bool,
}

impl FlightGuard<'_> {
    fn check(
        &self,
        context: &RunContext,
        state: &SimulationState,
        contact: Option<&IncomingContactV1>,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        let check = || -> Result<()> {
            if !snapshot_finite(state) {
                bail!("nonfinite full state");
            }
            if self.diagnostic {
                return Ok(());
            }
            if let Some(contact) = contact {
                if state.physical_outcome == PhysicalOutcome::LandedOnTarget
                    && state.mission_outcome == MissionOutcome::Success
                    && state.end_reason == EndReason::TouchdownOnTarget
                    && contact.state.physics_step == state.physics_step
                    && self.segments.iter().any(|s| {
                        s.kind != WaypointV2SegmentKind::LocalCorrection
                            && s.start_physics_step < state.physics_step
                            && state.physics_step <= s.end_physics_step
                            && phase_at_tick(&s.updates, state.physics_step) == "terminal_bridge"
                    })
                {
                    return Ok(());
                }
                bail!("unexpected active contact");
            }
            let segment = self.segments.iter().find(|s| {
                if state.physics_step == 0 {
                    s.start_physics_step == 0
                } else {
                    s.start_physics_step < state.physics_step
                        && state.physics_step <= s.end_physics_step
                }
            });
            let phase = if state.physics_step == 0 {
                "upright".into()
            } else {
                phase_at_tick(
                    &segment.context("missing segment guard ownership")?.updates,
                    state.physics_step,
                )
            };
            let (clearance, required) = if segment
                .is_some_and(|s| s.kind == WaypointV2SegmentKind::LocalCorrection)
            {
                clearing_body_reserve_query(context, self.request, state, "local", false)?
            } else {
                nominal_body_reserve_query(
                    context,
                    self.request,
                    state,
                    &phase,
                    state.physics_step == 0
                        || segment.is_some_and(|s| s.kind == WaypointV2SegmentKind::InitialNominal),
                )?
            };
            if clearance < required {
                bail!(
                    "segment reserve {clearance} below {required} at {}",
                    state.physics_step
                );
            }
            Ok(())
        };
        check().map_err(|e| {
            BoundedGuardFailureV1::new(
                BoundedGuardFailureDispositionV1::SafetyRejected,
                e.to_string(),
            )
        })
    }
}

impl BoundedRunGuard for FlightGuard<'_> {
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

fn prove_accumulated(
    request: &WaypointDirectNominalDirectGenerationRequest,
    context: &RunContext,
    live: &OrdinaryLive,
    segments: &[WaypointV2Segment],
    diagnostic: bool,
    deadline: u64,
) -> Result<BoundedRunArtifactsV1> {
    let updates = ledger_updates(segments);
    let endpoint = live.state.physics_step;
    // Coverage belongs to controller pairs; an actual contact may terminate
    // inside its final pair. Do not move the physical endpoint or add commands.
    let coverage_end = if endpoint.is_multiple_of(2) {
        endpoint
    } else {
        if !live.state.is_terminal() {
            bail!("flying proof endpoint is off the control clock");
        }
        endpoint
            .checked_add(1)
            .context("terminal command-pair end overflow")?
    };
    let limits = BoundedRunLimitsV1 {
        command_coverage_end_physics_step: coverage_end,
        hard_end_physics_step: deadline,
    };
    let mut index = 0;
    let mut guard = FlightGuard {
        request,
        segments,
        diagnostic,
    };
    let source = run_simulation_bounded(
        context,
        CONTROLLER,
        limits,
        |_, observation| {
            let u = updates
                .get(index)
                .ok_or_else(|| "whole-source command coverage gap".to_string())?;
            if u.physics_step != observation.physics_step {
                return Err("whole-source clock mismatch".into());
            }
            index += 1;
            Ok(u.command)
        },
        &mut guard,
    )?;
    let mut guard = FlightGuard {
        request,
        segments,
        diagnostic,
    };
    let replay = replay_simulation_bounded(
        context,
        CONTROLLER,
        &live.evidence.actions,
        limits,
        &mut guard,
    )?;
    if index != updates.len()
        || !full_ordinary_matches(&live.evidence, &source)
        || !full_ordinary_matches(&live.evidence, &replay)
        || source != replay
        || source.failure.is_some()
        || (!live.state.is_terminal()
            && (source.stop != BoundedRunStopCauseV1::CoverageExhausted
                || !source.coverage_reached))
    {
        bail!(
            "full accumulated replay mismatch: commands {}/{}, source_state={}, source_contact={}, source_actions={}, source_events={}, source_samples={}, official={}, envelopes={}, stop={:?}, failure={:?}",
            index,
            updates.len(),
            live.evidence.final_state == source.final_state,
            live.evidence.incoming_contact == source.incoming_contact,
            live.evidence.actions == source.run.actions,
            live.evidence.events == source.run.events,
            live.evidence.samples == source.run.samples,
            full_ordinary_matches(&live.evidence, &replay),
            source == replay,
            source.stop,
            source.failure
        );
    }
    Ok(source)
}

fn append_segment(
    segments: &mut Vec<WaypointV2Segment>,
    kind: WaypointV2SegmentKind,
    identity: &str,
    entry: &SimulationStateSnapshotV1,
    live: &OrdinaryLive,
    updates: Vec<FlightProgramUpdateV1>,
) -> Result<()> {
    let end = live.state.physics_step;
    if entry.physics_step == end {
        return Ok(());
    }
    if segments.last().is_some_and(|s| s.end_state != *entry)
        || updates.len() as u64
            != command_count(entry.physics_step, end).map_err(anyhow::Error::msg)?
        || updates
            .iter()
            .enumerate()
            .any(|(i, u)| u.physics_step != entry.physics_step + i as u64 * 2)
    {
        bail!("segment full-state continuity or exclusive clock ownership invalid");
    }
    segments.push(WaypointV2Segment {
        kind,
        start_physics_step: entry.physics_step,
        end_physics_step: end,
        proposal_identity: identity.into(),
        updates,
        entry_state: entry.clone(),
        end_state: live.evidence.final_state.clone(),
    });
    Ok(())
}

fn fixed_outcome(audit: &AirborneDirectAuditV1, prefix_proven: bool) -> Result<bool> {
    if !prefix_proven || !audit.ordinary_neutral_parity {
        bail!("invalid nominal audit consumed-prefix/neutral parity");
    }
    if audit
        .clearance_scan
        .first_violation
        .as_ref()
        .is_some_and(|v| {
            v.clearance_m.is_none_or(|c| !c.is_finite()) || !v.required_clearance_m.is_finite()
        })
    {
        bail!("nominal audit has unqueryable/nonfinite terrain evidence");
    }
    if audit.passed {
        if !audit.safe_target_contact
            || !audit.commands_match
            || !audit.clearance_scan.all_airborne_states_passed
        {
            bail!("inconsistent accepted nominal audit");
        }
        return Ok(false);
    }
    let mut contact_only = audit.clone();
    contact_only.clearance_scan.first_violation = None;
    Ok(audit
        .clearance_scan
        .first_violation
        .as_ref()
        .is_some_and(|v| v.clearance_m.is_some_and(|c| c < v.required_clearance_m))
        || has_actual_terrain_conflict(&contact_only))
}

fn conflict_tick(audit: &AirborneDirectAuditV1) -> Result<u64> {
    let clearance = audit
        .clearance_scan
        .first_violation
        .as_ref()
        .map(|v| v.physics_step);
    let mut contact_only = audit.clone();
    contact_only.clearance_scan.first_violation = None;
    let contact = has_actual_terrain_conflict(&contact_only)
        .then(|| {
            audit
                .incoming_contact
                .as_ref()
                .map(|c| c.state.physics_step)
        })
        .flatten();
    clearance
        .into_iter()
        .chain(contact)
        .min()
        .context("terrain-blocked audit has no genuine conflict tick")
}

/// Forward propagation from actual C, never SimulationState::new or a snapshot.
fn query_to(
    context: &RunContext,
    current: &OrdinaryLive,
    updates: &[FlightProgramUpdateV1],
    tick: u64,
) -> Result<OrdinaryLive> {
    if tick < current.state.physics_step {
        bail!("query attempted to rewind live clock");
    }
    let mut query = current.clone();
    advance_ordinary(context, &mut query, &consumed(updates, tick), tick)?;
    if query.state.physics_step != tick {
        bail!("contact before query endpoint");
    }
    Ok(query)
}

struct FlightLoop<'a> {
    request: &'a WaypointDirectNominalDirectGenerationRequest,
    context: RunContext,
    deadline: u64,
    live: OrdinaryLive,
    result: WaypointV2FlightResult,
    pending_cycle: Option<WaypointV2Cycle>,
}

impl FlightLoop<'_> {
    fn run(&mut self) -> Result<WaypointV2Stop> {
        loop {
            if self.live.state.physics_step >= self.deadline || self.live.state.fuel_kg <= 0.0 {
                return Ok(WaypointV2Stop::Deadline);
            }
            let planning_start = Instant::now();
            let initial = self.result.cycles.is_empty();
            let kind = if initial {
                WaypointV2SegmentKind::InitialNominal
            } else {
                WaypointV2SegmentKind::AirborneNominal
            };
            let mut cycle = WaypointV2Cycle {
                cycle_index: self.result.cycles.len(),
                current_state: self.live.evidence.final_state.clone(),
                nominal_search_identity: String::new(),
                nominal_proposal_identity: None,
                nominal_peak_com_height_m: None,
                nominal_attempt_status_counts: BTreeMap::new(),
                nominal_rejection_reason_counts: BTreeMap::new(),
                nominal_updates: Vec::new(),
                audit: None,
                fixed_consumed_prefix_proven: false,
                decision: WaypointV2CycleDecision::NoNominal,
                conflict_state: None,
                conflict_incoming_contact: None,
                local_search: None,
            };
            self.pending_cycle = Some(cycle.clone());
            let source_handoff;
            let audit;
            if initial {
                let search = evaluate_canonical_initial_direct(self.request)?;
                for attempt in &search.attempts {
                    if let Some(reason) = &attempt.reason {
                        *cycle
                            .nominal_rejection_reason_counts
                            .entry(reason.clone())
                            .or_default() += 1;
                    }
                    *cycle
                        .nominal_attempt_status_counts
                        .entry(attempt.status.clone())
                        .or_default() += 1;
                }
                cycle.nominal_search_identity = search.identity;
                let Some(proposal) = search.selected else {
                    self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
                    self.result.cycles.push(cycle);
                    return Ok(WaypointV2Stop::NoNominal);
                };
                if proposal.absolute_deadline_physics_step != self.deadline {
                    bail!("initial deadline binding differs");
                }
                source_handoff = Some(proposal.source_handoff_physics_step);
                audit = audit_canonical_initial_direct(
                    &self.context,
                    &source_pad_input(&self.context, &self.request.source_pad_id)?,
                    &proposal,
                    5.0,
                )?;
                cycle.nominal_proposal_identity = Some(proposal.identity);
                cycle.nominal_peak_com_height_m = Some(proposal.peak_com_height_m);
                cycle.nominal_updates = proposal.updates;
            } else if self.result.policy == WaypointV2Policy::revision_3() {
                let search = evaluate_airborne_acquisition_direct(
                    &self.context,
                    &self.live.state,
                    self.deadline,
                )?;
                for seed in &search.seeds {
                    if seed.selected_entry_index.is_none()
                        && let Some(reason) = &seed.reason
                    {
                        *cycle
                            .nominal_rejection_reason_counts
                            .entry(reason.clone())
                            .or_default() += 1;
                    }
                }
                for attempt in &search.attempts {
                    if let Some(reason) = &attempt.reason {
                        *cycle
                            .nominal_rejection_reason_counts
                            .entry(reason.clone())
                            .or_default() += 1;
                    }
                    *cycle
                        .nominal_attempt_status_counts
                        .entry(attempt.status.clone())
                        .or_default() += 1;
                }
                cycle.nominal_search_identity = search.identity;
                if search.unsupported_reason.is_some() {
                    cycle.decision = WaypointV2CycleDecision::Unsupported;
                    self.result.reason = search.unsupported_reason;
                    self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
                    self.result.cycles.push(cycle);
                    return Ok(WaypointV2Stop::Unsupported);
                }
                let Some(proposal) = search.selected else {
                    self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
                    self.result.cycles.push(cycle);
                    return Ok(WaypointV2Stop::NoNominal);
                };
                source_handoff = None;
                audit = audit_airborne_acquisition_proposal(
                    &self.context,
                    &self.live.state,
                    &proposal,
                    5.0,
                )?;
                cycle.nominal_proposal_identity = Some(proposal.identity);
                cycle.nominal_peak_com_height_m = Some(proposal.peak_com_height_m);
                cycle.nominal_updates = proposal.updates;
            } else {
                let search = evaluate_airborne_nominal_direct(
                    &self.context,
                    &self.live.state,
                    self.deadline,
                )?;
                for attempt in &search.attempts {
                    if let Some(reason) = &attempt.reason {
                        *cycle
                            .nominal_rejection_reason_counts
                            .entry(reason.clone())
                            .or_default() += 1;
                    }
                    *cycle
                        .nominal_attempt_status_counts
                        .entry(attempt.status.clone())
                        .or_default() += 1;
                }
                cycle.nominal_search_identity = search.identity;
                if search.unsupported_reason.is_some() {
                    cycle.decision = WaypointV2CycleDecision::Unsupported;
                    self.result.reason = search.unsupported_reason;
                    self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
                    self.result.cycles.push(cycle);
                    return Ok(WaypointV2Stop::Unsupported);
                }
                let Some(proposal) = search.selected else {
                    self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
                    self.result.cycles.push(cycle);
                    return Ok(WaypointV2Stop::NoNominal);
                };
                source_handoff = None;
                audit = audit_airborne_direct_proposal(
                    &self.context,
                    &self.live.state,
                    &proposal,
                    5.0,
                )?;
                cycle.nominal_proposal_identity = Some(proposal.identity);
                cycle.nominal_peak_com_height_m = Some(proposal.peak_com_height_m);
                cycle.nominal_updates = proposal.updates;
            }
            cycle.audit = Some(audit.clone());
            self.pending_cycle = Some(cycle.clone());
            let diagnostic = query_to(
                &self.context,
                &self.live,
                &cycle.nominal_updates,
                audit.final_state.physics_step,
            )?;
            if diagnostic.evidence.final_state != audit.final_state
                || diagnostic.evidence.incoming_contact != audit.incoming_contact
            {
                bail!("fixed live-origin audit state/contact mismatch");
            }
            self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
            let replay_start = Instant::now();
            let mut diagnostic_segments = self.result.segments.clone();
            append_segment(
                &mut diagnostic_segments,
                kind,
                cycle.nominal_proposal_identity.as_deref().unwrap(),
                &cycle.current_state,
                &diagnostic,
                consumed(&cycle.nominal_updates, audit.final_state.physics_step),
            )?;
            prove_accumulated(
                self.request,
                &self.context,
                &diagnostic,
                &diagnostic_segments,
                true,
                self.deadline,
            )?;
            self.result.timings.replay_s += replay_start.elapsed().as_secs_f64();
            cycle.fixed_consumed_prefix_proven = true;
            let blocked = fixed_outcome(&audit, true)?;
            if initial {
                self.result.initial_nominal_terrain_blocked = blocked;
            }
            cycle.decision = if blocked {
                WaypointV2CycleDecision::TerrainBlocked
            } else {
                WaypointV2CycleDecision::NominalRejected
            };
            self.pending_cycle = Some(cycle.clone());
            if !blocked {
                if !audit.passed {
                    self.result.cycles.push(cycle);
                    return Ok(WaypointV2Stop::NominalRejected);
                }
                let start = Instant::now();
                let updates = consumed(&cycle.nominal_updates, audit.final_state.physics_step);
                advance_ordinary(
                    &self.context,
                    &mut self.live,
                    &updates,
                    audit.final_state.physics_step,
                )?;
                if self.live.evidence != diagnostic.evidence {
                    bail!("active Direct does not match audited actual flight");
                }
                append_segment(
                    &mut self.result.segments,
                    kind,
                    cycle.nominal_proposal_identity.as_deref().unwrap(),
                    &cycle.current_state,
                    &self.live,
                    updates,
                )?;
                self.result.timings.execution_s += start.elapsed().as_secs_f64();
                cycle.decision = WaypointV2CycleDecision::Direct;
                self.result.cycles.push(cycle);
                return Ok(WaypointV2Stop::Landed);
            }
            if !self.result.policy.can_correct(self.result.correction_count) {
                cycle.decision = WaypointV2CycleDecision::CorrectionLimit;
                self.result.cycles.push(cycle);
                return Ok(WaypointV2Stop::CorrectionLimit);
            }
            let planning_start = Instant::now();
            let f = conflict_tick(&audit)?;
            let conflict = query_to(&self.context, &self.live, &cycle.nominal_updates, f)?;
            let conflict_state = conflict
                .evidence
                .incoming_contact
                .as_ref()
                .map(|c| c.state.clone())
                .unwrap_or_else(|| conflict.evidence.final_state.clone());
            if let Some(v) = audit
                .clearance_scan
                .first_violation
                .as_ref()
                .filter(|v| v.physics_step == f)
            {
                let (clearance, required) = nominal_body_reserve_query(
                    &self.context,
                    self.request,
                    &conflict.state,
                    &v.phase,
                    initial,
                )?;
                if Some(clearance) != v.clearance_m || required != v.required_clearance_m {
                    bail!("live-origin first reserve conflict differs from fixed audit");
                }
            }
            cycle.conflict_state = Some(conflict_state.clone());
            cycle.conflict_incoming_contact = conflict.evidence.incoming_contact.clone();
            self.pending_cycle = Some(cycle.clone());
            let entries = if let Some(s) = source_handoff {
                self.result.policy.initial_intervention_boundaries(s)
            } else {
                later_intervention_boundaries(self.live.state.physics_step, f)
            }
            .map_err(anyhow::Error::msg)?;
            let goal = LocalClearingGoalV1 {
                first_conflict_physics_step: f,
                first_conflict_position_m: conflict_state.position_m,
                absolute_deadline_physics_step: self.deadline,
            };
            let mut local = WaypointV2LocalSearch {
                entries: Vec::new(),
                row_count: 0,
                boundary_count: 0,
                accepted_row_count: 0,
                row_status_counts: BTreeMap::new(),
                row_stop_reason_counts: BTreeMap::new(),
                boundary_status_counts: BTreeMap::new(),
                selected: None,
                certificate_state: None,
                handoff_source_replay_passed: false,
                certificate_source_replay_passed: false,
            };
            let templates = LocalClearingPolicyV1::default()
                .templates()
                .map_err(anyhow::Error::msg)?;
            let mut selected_entry = None;
            for (id, tick) in entries {
                let mut entry = None;
                let mut reason = if tick >= f {
                    Some("entry is not before new conflict".into())
                } else {
                    None
                };
                if reason.is_none() {
                    let query = query_to(&self.context, &self.live, &cycle.nominal_updates, tick)?;
                    reason = entry_rejection(&self.context, &query.state, self.deadline);
                    if reason.is_none() {
                        let (clearance, required) = clearing_body_reserve_query(
                            &self.context,
                            self.request,
                            &query.state,
                            "local",
                            false,
                        )?;
                        if clearance < required {
                            reason = Some("local entry lacks full body reserve".into());
                        }
                    }
                    if reason.is_none() {
                        entry = Some(query);
                    }
                }
                local.entries.push(WaypointV2Entry {
                    entry_id: id.clone(),
                    physics_step: tick,
                    admitted: entry.is_some(),
                    reason: reason.clone(),
                });
                for template in &templates {
                    let (row, proposal) = search_row(
                        self.request,
                        &self.context,
                        &id,
                        entry.as_ref().map(|e| &e.state),
                        (tick, reason.as_deref()),
                        template,
                        &goal,
                    )?;
                    if row.stop_reason.as_deref()
                        == Some("physical_trace: nonfinite full simulation state")
                    {
                        self.result.failed_local_row = Some(row);
                        bail!("nonfinite local physical propagation is not a coverage miss");
                    }
                    local.row_count += 1;
                    if let Some(reason) = &row.stop_reason {
                        *local
                            .row_stop_reason_counts
                            .entry(reason.split(':').next().unwrap_or(reason).to_string())
                            .or_default() += 1;
                    }
                    local.boundary_count += row.boundaries.len();
                    *local.row_status_counts.entry(row.status).or_default() += 1;
                    for boundary in row.boundaries {
                        *local
                            .boundary_status_counts
                            .entry(boundary.status)
                            .or_default() += 1;
                    }
                    if let Some(proposal) = proposal {
                        local.accepted_row_count += 1;
                        if local
                            .selected
                            .as_ref()
                            .is_none_or(|p| local_rank(&proposal, p).is_lt())
                        {
                            local.selected = Some(proposal);
                            selected_entry = entry.clone();
                        }
                    }
                }
            }
            let Some(proposal) = local.selected.as_ref() else {
                // The old search tests progress before certificate safety, so
                // insufficient_progress alone cannot prove a safe no-progress
                // family. Keep the finite exhaustion typed NoClearing.
                let stop = WaypointV2Stop::NoClearing;
                self.result.reason = Some(
                    "finite local clearing grid exhausted; boundary rejection counts retained"
                        .into(),
                );
                cycle.decision = if stop == WaypointV2Stop::NoProgress {
                    WaypointV2CycleDecision::NoProgress
                } else {
                    WaypointV2CycleDecision::NoClearing
                };
                cycle.local_search = Some(local);
                self.result.cycles.push(cycle);
                self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
                return Ok(stop);
            };
            let entry = selected_entry.context("selected row lacks genuine live query entry")?;
            cycle.local_search = Some(local.clone());
            self.pending_cycle = Some(cycle.clone());
            validate_local_clearing_proposal(self.request, &entry.state, proposal)?;
            self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
            let execute_start = Instant::now();
            let prefix = consumed(&cycle.nominal_updates, proposal.schedule.entry_physics_step);
            advance_ordinary(
                &self.context,
                &mut self.live,
                &prefix,
                proposal.schedule.entry_physics_step,
            )?;
            if self.live.evidence != entry.evidence {
                bail!("actual selected E differs from full forward query");
            }
            append_segment(
                &mut self.result.segments,
                kind,
                cycle.nominal_proposal_identity.as_deref().unwrap(),
                &cycle.current_state,
                &self.live,
                prefix,
            )?;
            let entry_state = self.live.evidence.final_state.clone();
            let updates = consumed(
                &proposal.schedule.updates,
                proposal.schedule.handoff_physics_step,
            );
            advance_ordinary(
                &self.context,
                &mut self.live,
                &updates,
                proposal.schedule.handoff_physics_step,
            )?;
            if self.live.evidence.final_state != proposal.handoff_state {
                bail!("actual H differs from local validated handoff");
            }
            append_segment(
                &mut self.result.segments,
                WaypointV2SegmentKind::LocalCorrection,
                &proposal.identity,
                &entry_state,
                &self.live,
                updates,
            )?;
            let mut certificate = self.live.clone();
            let guard_updates: Vec<_> = proposal
                .schedule
                .updates
                .iter()
                .filter(|u| u.physics_step >= proposal.schedule.handoff_physics_step)
                .cloned()
                .collect();
            advance_ordinary(
                &self.context,
                &mut certificate,
                &guard_updates,
                proposal.schedule.continuation_end_physics_step,
            )?;
            if certificate.evidence.final_state != proposal.continuation_end_state {
                bail!("separate actual certificate state mismatch");
            }
            let mut certificate_segments = self.result.segments.clone();
            append_segment(
                &mut certificate_segments,
                WaypointV2SegmentKind::LocalCorrection,
                &proposal.identity,
                &self.live.evidence.final_state,
                &certificate,
                guard_updates,
            )?;
            local.certificate_state = Some(certificate.evidence.final_state.clone());
            self.result.timings.execution_s += execute_start.elapsed().as_secs_f64();
            let replay_start = Instant::now();
            prove_accumulated(
                self.request,
                &self.context,
                &self.live,
                &self.result.segments,
                false,
                self.deadline,
            )?;
            prove_accumulated(
                self.request,
                &self.context,
                &certificate,
                &certificate_segments,
                false,
                self.deadline,
            )?;
            local.handoff_source_replay_passed = true;
            local.certificate_source_replay_passed = true;
            self.result.timings.replay_s += replay_start.elapsed().as_secs_f64();
            cycle.decision = WaypointV2CycleDecision::LocalCleared;
            cycle.local_search = Some(local);
            self.result.cycles.push(cycle);
            self.result.correction_count += 1;
        }
    }
}

/// Runs no preservation canaries and writes no files. Every continuation starts
/// from the same retained in-memory plant; snapshots are output evidence only.
pub fn run_waypoint_v2_flight(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &WaypointV2Policy,
) -> Result<WaypointV2FlightResult> {
    let preflight = preflight_waypoint_v2_flight(request, policy);
    let result = WaypointV2FlightResult {
        policy: policy.clone(),
        input_identity: nominal_direct_flight_identity(&(request, policy))?,
        planning_stop: preflight.rejection.unwrap_or(WaypointV2Stop::NoNominal),
        reason: preflight.reason,
        correction_count: 0,
        initial_nominal_terrain_blocked: false,
        integrity_passed: true,
        physical_outcome: None,
        mission_outcome: None,
        absolute_deadline_physics_step: None,
        cycles: Vec::new(),
        segments: Vec::new(),
        ordinary_flight: None,
        final_source_replay_passed: false,
        manifest: None,
        failed_local_row: None,
        timings: WaypointV2Timings::default(),
    };
    if !preflight.supported {
        return Ok(result);
    }
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let deadline = original_deadline(
        context.sim.max_time_s,
        request.policy.analytical_policy.mission_budget_s(),
    )
    .map_err(anyhow::Error::msg)?;
    let live = new_ordinary(&context)?;
    let mut flight = FlightLoop {
        request,
        context,
        deadline,
        live,
        result,
        pending_cycle: None,
    };
    flight.result.absolute_deadline_physics_step = Some(deadline);
    match flight.run() {
        Ok(stop) => flight.result.planning_stop = stop,
        Err(error) => {
            flight.result.planning_stop = WaypointV2Stop::ImplementationError;
            flight.result.integrity_passed = false;
            flight.result.reason = Some(format!("{error:#}"));
            flight.live.evidence.final_state =
                SimulationStateSnapshotV1::from_state(&flight.live.state);
            if let Some(cycle) = flight
                .pending_cycle
                .take()
                .filter(|c| c.cycle_index == flight.result.cycles.len())
            {
                flight.result.cycles.push(cycle);
            }
        }
    }
    // Even a finite coverage miss must retain and replay its actual partial flight.
    let start = Instant::now();
    match prove_accumulated(
        request,
        &flight.context,
        &flight.live,
        &flight.result.segments,
        false,
        deadline,
    ) {
        Ok(proof) => {
            flight.result.final_source_replay_passed = true;
            flight.result.manifest = Some(proof.run.manifest);
        }
        Err(error) => {
            flight.result.planning_stop = WaypointV2Stop::ImplementationError;
            flight.result.integrity_passed = false;
            flight.result.reason = Some(format!("final accumulated replay: {error:#}"));
        }
    }
    flight.result.timings.replay_s += start.elapsed().as_secs_f64();
    flight.result.physical_outcome = Some(flight.live.state.physical_outcome);
    flight.result.mission_outcome = Some(flight.live.state.mission_outcome);
    flight.result.ordinary_flight = Some(flight.live.evidence);
    Ok(flight.result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> WaypointDirectNominalDirectGenerationRequest {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        crate::load_nominal_direct_operational_fresh_inputs(repo)
            .unwrap()
            .remove(0)
            .1
    }
    #[test]
    fn preflight_is_input_only_and_disallows_even_empty_authored_route() {
        let mut r = request();
        let p = WaypointV2Policy::default();
        assert!(preflight_waypoint_v2_flight(&r, &p).supported);
        r.scenario.mission.transfer_route = Some(pd_core::TransferRouteSpec {
            source_pad_id: r.source_pad_id.clone(),
            target_pad_id: r.target_pad_id.clone(),
            waypoints: Vec::new(),
            route_angle_deg: 0.0,
            route_radius_m: 900.0,
        });
        let preflight = preflight_waypoint_v2_flight(&r, &p);
        assert_eq!(preflight.rejection, Some(WaypointV2Stop::Unsupported));
        assert!(!preflight.simulation_created);
    }
    #[test]
    fn nonterrain_terminal_rejection_is_not_an_implementation_error() {
        let r = request();
        let context = RunContext::from_scenario(&r.scenario).unwrap();
        let search = evaluate_canonical_initial_direct(&r).unwrap();
        let mut audit = audit_canonical_initial_direct(
            &context,
            &source_pad_input(&context, &r.source_pad_id).unwrap(),
            &search.selected.unwrap(),
            5.0,
        )
        .unwrap();
        assert!(audit.passed);
        audit.passed = false;
        audit.safe_target_contact = false;
        assert!(!fixed_outcome(&audit, true).unwrap());
        assert!(fixed_outcome(&audit, false).is_err());
        audit.ordinary_neutral_parity = false;
        assert!(fixed_outcome(&audit, true).is_err());
    }
    #[test]
    fn clear_control_has_no_corrections_and_full_replay() {
        let request = request();
        let result = run_waypoint_v2_flight(&request, &WaypointV2Policy::default()).unwrap();
        assert_eq!(
            result.planning_stop,
            WaypointV2Stop::Landed,
            "{:?}",
            result.reason
        );
        assert_eq!(result.correction_count, 0);
        assert!(!result.initial_nominal_terrain_blocked);
        assert!(result.integrity_passed && result.final_source_replay_passed);
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let mut live = new_ordinary(&context).unwrap();
        let updates = ledger_updates(&result.segments);
        let endpoint = result
            .ordinary_flight
            .as_ref()
            .unwrap()
            .final_state
            .physics_step;
        advance_ordinary(&context, &mut live, &updates, endpoint).unwrap();
        assert!(
            prove_accumulated(&request, &context, &live, &result.segments, false, 9600).is_ok()
        );
        let mut tampered = live.clone();
        tampered.evidence.actions[0].command.throttle_frac = 0.0;
        assert!(
            prove_accumulated(&request, &context, &tampered, &result.segments, false, 9600)
                .is_err()
        );
        let mut tampered = live.clone();
        tampered
            .evidence
            .incoming_contact
            .as_mut()
            .unwrap()
            .state
            .fuel_kg += 1.0;
        assert!(
            prove_accumulated(&request, &context, &tampered, &result.segments, false, 9600)
                .is_err()
        );
        let mut tampered = result.segments.clone();
        tampered[0].kind = WaypointV2SegmentKind::LocalCorrection;
        assert!(prove_accumulated(&request, &context, &live, &tampered, false, 9600).is_err());
    }
    #[test]
    fn synthetic_corridor_queries_keep_launch_and_terminal_exemptions_separate() {
        let request = request();
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let mut state = SimulationState::new(&context).unwrap();
        // Synthetic poses test query policy only, not a flown route.
        assert_eq!(
            nominal_body_reserve_query(&context, &request, &state, "upright", true)
                .unwrap()
                .1,
            0.0
        );
        assert_eq!(
            nominal_body_reserve_query(&context, &request, &state, "upright", false)
                .unwrap()
                .1,
            5.0
        );
        assert_eq!(
            clearing_body_reserve_query(&context, &request, &state, "local", false)
                .unwrap()
                .1,
            5.0
        );
        state.position_m = pd_core::Vec2::new(
            context.target_pad.center_x_m,
            context.target_pad.surface_y_m + 10.0,
        );
        state.velocity_mps = pd_core::Vec2::new(1.0, -1.0);
        assert_eq!(
            nominal_body_reserve_query(&context, &request, &state, "terminal_bridge", false)
                .unwrap()
                .1,
            0.0
        );
        assert_eq!(
            nominal_body_reserve_query(&context, &request, &state, "ballistic_coast", false)
                .unwrap()
                .1,
            5.0
        );
        assert_eq!(
            clearing_body_reserve_query(&context, &request, &state, "local", false)
                .unwrap()
                .1,
            5.0
        );
    }
    #[test]
    fn odd_actual_contact_proof_has_no_endpoint_padding_or_extra_command() {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let manifest =
            crate::load_waypoint_direct_obstacle_discrimination_fresh_manifest(repo).unwrap();
        let base = manifest
            .cases
            .iter()
            .find(|c| c.case_id == "fresh_flat_control_span_900")
            .unwrap();
        let mut request = WaypointDirectNominalDirectGenerationRequest {
            scenario: base.scenario.clone(),
            source_pad_id: base.source_pad_id.clone(),
            target_pad_id: base.target_pad_id.clone(),
            probe_id: "odd_contact_proof".into(),
            policy: manifest.generation_policy,
        };
        let mut points = request.scenario.world.terrain.points().to_vec();
        points.extend([
            pd_core::Vec2::new(-540.0, 0.0),
            pd_core::Vec2::new(-450.0, 315.0),
            pd_core::Vec2::new(-360.0, 0.0),
        ]);
        points.sort_by(|a, b| a.x.total_cmp(&b.x));
        request.scenario.world.terrain =
            pd_core::TerrainDefinition::Heightfield { points_m: points };
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let proposal = evaluate_canonical_initial_direct(&request)
            .unwrap()
            .selected
            .unwrap();
        let audit = audit_canonical_initial_direct(
            &context,
            &source_pad_input(&context, &request.source_pad_id).unwrap(),
            &proposal,
            5.0,
        )
        .unwrap();
        assert_eq!(audit.final_state.physics_step, 2121);
        let initial = new_ordinary(&context).unwrap();
        let query = query_to(&context, &initial, &proposal.updates, 2121).unwrap();
        let mut segments = Vec::new();
        append_segment(
            &mut segments,
            WaypointV2SegmentKind::InitialNominal,
            &proposal.identity,
            &initial.evidence.final_state,
            &query,
            consumed(&proposal.updates, 2121),
        )
        .unwrap();
        let proof = prove_accumulated(&request, &context, &query, &segments, true, 9600).unwrap();
        assert_eq!(proof.stop, BoundedRunStopCauseV1::MissionTerminal);
        assert_eq!(proof.final_state.physics_step, 2121);
        assert_eq!(proof.limits.command_coverage_end_physics_step, 2122);
        assert_eq!(proof.run.actions.len(), 1061);
        assert_eq!(proof.run.actions.last().unwrap().physics_step, 2120);
        assert_eq!(proof.final_state, audit.final_state);
        assert_eq!(proof.incoming_contact, audit.incoming_contact);
        assert!(fixed_outcome(&audit, true).unwrap());
    }
    #[test]
    fn reference_plateau_composes_actual_handoffs_without_reset() {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let manifest =
            crate::load_waypoint_direct_obstacle_discrimination_fresh_manifest(repo).unwrap();
        let case = manifest
            .cases
            .iter()
            .find(|c| c.case_id == "fresh_late_broad_span_900")
            .unwrap();
        let request = WaypointDirectNominalDirectGenerationRequest {
            scenario: case.scenario.clone(),
            source_pad_id: case.source_pad_id.clone(),
            target_pad_id: case.target_pad_id.clone(),
            probe_id: "v2_reference_test".into(),
            policy: manifest.generation_policy,
        };
        let result = run_waypoint_v2_flight(&request, &WaypointV2Policy::default()).unwrap();
        assert!(result.integrity_passed, "{:?}", result.reason);
        assert!(result.final_source_replay_passed);
        assert!(result.initial_nominal_terrain_blocked);
        assert!(result.correction_count >= 1);
        assert_eq!(result.planning_stop, WaypointV2Stop::Landed);
        assert!(result.correction_count >= 2);
        let selected = result.cycles[0]
            .local_search
            .as_ref()
            .unwrap()
            .selected
            .as_ref()
            .unwrap();
        assert_eq!(selected.row_id, "source_75_percent_row_26");
        assert_eq!(selected.schedule.entry_physics_step, 1512);
        assert_eq!(selected.schedule.handoff_physics_step, 2820);
        assert_eq!(
            selected.handoff_state.position_m,
            pd_core::Vec2::new(-337.77126006831395, 382.8205898542399)
        );
        for cycle in &result.cycles[1..] {
            assert!(cycle.current_state.physics_step >= 2820);
            assert_eq!(
                cycle.current_state.sim_time_s,
                cycle.current_state.physics_step as f64 / 120.0
            );
        }
        eprintln!(
            "reference result: {:?}, corrections {}, timings {:?}",
            result.planning_stop, result.correction_count, result.timings
        );
    }
}
