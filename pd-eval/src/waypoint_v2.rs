//! Owned synchronous piecewise ballistic flight. Nominal generation is
//! terrain-blind; corrections are local and never scored by a landing suffix.
//! Persisted records live in `model`; forward execution and whole-source safety
//! proofs live in `execution`. This module owns planning and session lifecycle.
use std::{collections::BTreeMap, time::Instant};

use anyhow::{Context, Result, bail, ensure};
use pd_core::{RunContext, SimulationStateSnapshotV1};
use pd_plan::{
    local_clearing::{LocalClearingGoalV1, LocalClearingPolicyV1},
    waypoint_v2::{later_intervention_boundaries, original_deadline},
};
use serde::{Deserialize, Serialize};

pub use pd_plan::waypoint_v2::{WaypointV2Policy, WaypointV2Stop};

use crate::{
    AirborneDirectAuditV1, BodyAwareTerminalPolicyV1, NominalDirectFlightDecisionV1,
    WaypointDirectNominalDirectGenerationRequest, audit_airborne_acquisition_proposal,
    audit_canonical_initial_direct, clearing_body_reserve_query,
    evaluate_airborne_acquisition_direct, evaluate_canonical_initial_direct,
    local_clearing::{
        OrdinaryLive, advance_ordinary, entry_rejection, local_rank, new_ordinary, search_row,
    },
    nominal_body_reserve_query, nominal_direct_flight_identity,
    planner_flight::terrain::{has_actual_terrain_conflict, source_pad_input},
    preflight_nominal_direct_flight, validate_local_clearing_proposal,
};

mod execution;
mod model;

pub(crate) use execution::replay_saved_waypoint_v2_evidence;
use execution::{append_segment, consumed, prove_accumulated, query_to};
pub use model::*;

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
    } else if let Err(reason) = policy.validate_for_execution() {
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

fn checkpoint_local_search(
    pending_cycle: &mut Option<WaypointV2Cycle>,
    local_search: &WaypointV2LocalSearch,
) {
    if let Some(cycle) = pending_cycle.as_mut() {
        cycle.local_search = Some(local_search.clone());
    }
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

enum PieceAdvance {
    Handoff {
        piece_index: usize,
        entry_physics_step: u64,
        handoff_physics_step: u64,
    },
    Terminal {
        planning_stop: WaypointV2Stop,
        piece_index: Option<usize>,
        entry_physics_step: Option<u64>,
        physics_step: Option<u64>,
    },
}

enum NominalBuild {
    Ready {
        source_handoff: Option<u64>,
        audit: Box<AirborneDirectAuditV1>,
    },
    Unsupported(String),
    NoNominal,
}

struct SelectedClearing {
    local: WaypointV2LocalSearch,
    entry: OrdinaryLive,
}

struct FixedPrefixProof {
    diagnostic: OrdinaryLive,
    terrain_blocked: bool,
}

fn record_nominal_rejection_reason(cycle: &mut WaypointV2Cycle, reason: &str) {
    *cycle
        .nominal_rejection_reason_counts
        .entry(reason.to_owned())
        .or_default() += 1;
}

fn record_nominal_attempt(cycle: &mut WaypointV2Cycle, status: &str, reason: Option<&str>) {
    if let Some(reason) = reason {
        record_nominal_rejection_reason(cycle, reason);
    }
    *cycle
        .nominal_attempt_status_counts
        .entry(status.to_owned())
        .or_default() += 1;
}

fn retain_selected_nominal(
    cycle: &mut WaypointV2Cycle,
    identity: String,
    peak_com_height_m: f64,
    updates: Vec<pd_core::FlightProgramUpdateV1>,
) {
    cycle.nominal_proposal_identity = Some(identity);
    cycle.nominal_peak_com_height_m = Some(peak_com_height_m);
    cycle.nominal_updates = updates;
}

fn terminal_piece_index(cycles: &[WaypointV2Cycle], entry_physics_step: u64) -> Option<usize> {
    cycles
        .last()
        .filter(|cycle| cycle.current_state.physics_step == entry_physics_step)
        .map(|cycle| cycle.cycle_index)
}

struct FlightLoop {
    context: RunContext,
    deadline: u64,
    live: OrdinaryLive,
    result: WaypointV2FlightResult,
    pending_cycle: Option<WaypointV2Cycle>,
}

impl FlightLoop {
    fn advance_piece(
        &mut self,
        request: &WaypointDirectNominalDirectGenerationRequest,
    ) -> Result<PieceAdvance> {
        let piece_entry_physics_step = self.live.state.physics_step;
        if self.live.state.physics_step >= self.deadline || self.live.state.fuel_kg <= 0.0 {
            return Ok(PieceAdvance::Terminal {
                planning_stop: WaypointV2Stop::Deadline,
                piece_index: None,
                entry_physics_step: Some(piece_entry_physics_step),
                physics_step: Some(piece_entry_physics_step),
            });
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
        let nominal = match self.build_nominal_program(request, &mut cycle, initial)? {
            NominalBuild::Ready {
                source_handoff,
                audit,
            } => (source_handoff, audit),
            NominalBuild::Unsupported(reason) => {
                cycle.decision = WaypointV2CycleDecision::Unsupported;
                self.result.reason = Some(reason);
                self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
                self.result.cycles.push(cycle);
                return Ok(
                    self.terminal_piece(WaypointV2Stop::Unsupported, piece_entry_physics_step)
                );
            }
            NominalBuild::NoNominal => {
                self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
                self.result.cycles.push(cycle);
                return Ok(self.terminal_piece(WaypointV2Stop::NoNominal, piece_entry_physics_step));
            }
        };
        let (source_handoff, audit) = nominal;
        let fixed = self.prove_fixed_consumed_prefix(
            request,
            &mut cycle,
            kind,
            initial,
            &audit,
            planning_start,
        )?;
        if !fixed.terrain_blocked {
            if !audit.passed {
                self.result.cycles.push(cycle);
                return Ok(
                    self.terminal_piece(WaypointV2Stop::NominalRejected, piece_entry_physics_step)
                );
            }
            return self.execute_direct_piece(
                cycle,
                kind,
                &fixed.diagnostic,
                piece_entry_physics_step,
            );
        }
        if !self.result.policy.can_correct(self.result.correction_count) {
            cycle.decision = WaypointV2CycleDecision::CorrectionLimit;
            self.result.cycles.push(cycle);
            return Ok(
                self.terminal_piece(WaypointV2Stop::CorrectionLimit, piece_entry_physics_step)
            );
        }
        let Some(selected) =
            self.search_local_clearing(request, &mut cycle, &audit, source_handoff, initial)?
        else {
            return Ok(self.terminal_piece(WaypointV2Stop::NoClearing, piece_entry_physics_step));
        };
        self.execute_correction(request, cycle, kind, selected, piece_entry_physics_step)
    }

    fn build_nominal_program(
        &self,
        request: &WaypointDirectNominalDirectGenerationRequest,
        cycle: &mut WaypointV2Cycle,
        initial: bool,
    ) -> Result<NominalBuild> {
        if initial {
            self.build_initial_nominal(request, cycle)
        } else {
            self.build_acquisition_nominal(cycle)
        }
    }

    fn build_initial_nominal(
        &self,
        request: &WaypointDirectNominalDirectGenerationRequest,
        cycle: &mut WaypointV2Cycle,
    ) -> Result<NominalBuild> {
        let search = evaluate_canonical_initial_direct(request)?;
        for attempt in &search.attempts {
            record_nominal_attempt(cycle, &attempt.status, attempt.reason.as_deref());
        }
        cycle.nominal_search_identity = search.identity;
        let Some(proposal) = search.selected else {
            return Ok(NominalBuild::NoNominal);
        };
        if proposal.absolute_deadline_physics_step != self.deadline {
            bail!("initial deadline binding differs");
        }
        let source_handoff = proposal.source_handoff_physics_step;
        let audit = audit_canonical_initial_direct(
            &self.context,
            &source_pad_input(&self.context, &request.source_pad_id)?,
            &proposal,
            5.0,
        )?;
        retain_selected_nominal(
            cycle,
            proposal.identity,
            proposal.peak_com_height_m,
            proposal.updates,
        );
        Ok(NominalBuild::Ready {
            source_handoff: Some(source_handoff),
            audit: Box::new(audit),
        })
    }

    fn build_acquisition_nominal(&self, cycle: &mut WaypointV2Cycle) -> Result<NominalBuild> {
        let search =
            evaluate_airborne_acquisition_direct(&self.context, &self.live.state, self.deadline)?;
        for seed in &search.seeds {
            if seed.selected_entry_index.is_none()
                && let Some(reason) = &seed.reason
            {
                record_nominal_rejection_reason(cycle, reason);
            }
        }
        for attempt in &search.attempts {
            record_nominal_attempt(cycle, &attempt.status, attempt.reason.as_deref());
        }
        cycle.nominal_search_identity = search.identity;
        if let Some(reason) = search.unsupported_reason {
            return Ok(NominalBuild::Unsupported(reason));
        }
        let Some(proposal) = search.selected else {
            return Ok(NominalBuild::NoNominal);
        };
        let audit =
            audit_airborne_acquisition_proposal(&self.context, &self.live.state, &proposal, 5.0)?;
        retain_selected_nominal(
            cycle,
            proposal.identity,
            proposal.peak_com_height_m,
            proposal.updates,
        );
        Ok(NominalBuild::Ready {
            source_handoff: None,
            audit: Box::new(audit),
        })
    }

    fn prove_fixed_consumed_prefix(
        &mut self,
        request: &WaypointDirectNominalDirectGenerationRequest,
        cycle: &mut WaypointV2Cycle,
        kind: WaypointV2SegmentKind,
        initial: bool,
        audit: &AirborneDirectAuditV1,
        planning_start: Instant,
    ) -> Result<FixedPrefixProof> {
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
            request,
            &self.context,
            &diagnostic,
            &diagnostic_segments,
            true,
            self.deadline,
        )?;
        self.result.timings.replay_s += replay_start.elapsed().as_secs_f64();
        cycle.fixed_consumed_prefix_proven = true;
        let blocked = fixed_outcome(audit, true)?;
        if initial {
            self.result.initial_nominal_terrain_blocked = blocked;
        }
        cycle.decision = if blocked {
            WaypointV2CycleDecision::TerrainBlocked
        } else {
            WaypointV2CycleDecision::NominalRejected
        };
        self.pending_cycle = Some(cycle.clone());
        Ok(FixedPrefixProof {
            diagnostic,
            terrain_blocked: blocked,
        })
    }

    fn execute_direct_piece(
        &mut self,
        mut cycle: WaypointV2Cycle,
        kind: WaypointV2SegmentKind,
        diagnostic: &OrdinaryLive,
        piece_entry_physics_step: u64,
    ) -> Result<PieceAdvance> {
        let start = Instant::now();
        let updates = consumed(&cycle.nominal_updates, diagnostic.state.physics_step);
        advance_ordinary(
            &self.context,
            &mut self.live,
            &updates,
            diagnostic.state.physics_step,
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
        Ok(self.terminal_piece(WaypointV2Stop::Landed, piece_entry_physics_step))
    }

    fn search_local_clearing(
        &mut self,
        request: &WaypointDirectNominalDirectGenerationRequest,
        cycle: &mut WaypointV2Cycle,
        audit: &AirborneDirectAuditV1,
        source_handoff: Option<u64>,
        initial: bool,
    ) -> Result<Option<SelectedClearing>> {
        let planning_start = Instant::now();
        let f = conflict_tick(audit)?;
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
                request,
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
        let mut entries = if let Some(source_handoff) = source_handoff {
            self.result
                .policy
                .initial_intervention_boundaries(source_handoff)
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
            row_diagnostics: Vec::new(),
        };
        let templates = LocalClearingPolicyV1::default()
            .templates()
            .map_err(anyhow::Error::msg)?;
        let mut selected_entry = None;
        let mut nonnegative_room_alternative: Option<(
            crate::LocalClearingProposalV1,
            OrdinaryLive,
        )> = None;
        for stage in 0..2 {
            if stage == 1 {
                // Preserve the entire primary search and its ranking. Fallback
                // rows cannot displace an already accepted primary proposal.
                if local.selected.is_some() {
                    break;
                }
                let tested = local
                    .entries
                    .iter()
                    .map(|entry| entry.physics_step)
                    .collect::<Vec<_>>();
                entries = self
                    .result
                    .policy
                    .fallback_intervention_boundaries(self.live.state.physics_step, f, &tested)
                    .map_err(anyhow::Error::msg)?;
            }
            for (id, tick) in entries.drain(..) {
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
                            request,
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
                checkpoint_local_search(&mut self.pending_cycle, &local);
                for template in &templates {
                    let (row, proposal) = search_row(
                        request,
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
                    local
                        .row_diagnostics
                        .push(WaypointV2RowDiagnostic::from_row(
                            &row,
                            goal.progress_x(
                                entry
                                    .as_ref()
                                    .map_or(self.live.state.position_m.x, |e| e.state.position_m.x),
                                &self.context.vehicle.geometry,
                            ),
                        ));
                    if let Some(proposal) = proposal.as_ref() {
                        local.row_diagnostics.last_mut().unwrap().eligible_handoff =
                            Some(WaypointV2EligibleHandoffDiagnostic {
                                state: proposal.handoff_state.clone(),
                                actual_fuel_burn_to_handoff_kg: proposal
                                    .actual_fuel_burn_to_handoff_kg,
                                braking_room: crate::local_clearing::braking_room_at_handoff(
                                    &self.context,
                                    &proposal.handoff_state,
                                ),
                            });
                    }
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
                        if crate::local_clearing::braking_room_at_handoff(
                            &self.context,
                            &proposal.handoff_state,
                        )
                        .is_some_and(|room| room.remaining_room_m >= 0.0)
                            && nonnegative_room_alternative
                                .as_ref()
                                .is_none_or(|(p, _)| local_rank(&proposal, p).is_lt())
                        {
                            nonnegative_room_alternative = Some((
                                proposal.clone(),
                                entry
                                    .clone()
                                    .context("accepted row lacks live query entry")?,
                            ));
                        }
                        if local
                            .selected
                            .as_ref()
                            .is_none_or(|p| local_rank(&proposal, p).is_lt())
                        {
                            local.selected = Some(proposal);
                            selected_entry = entry.clone();
                        }
                    }
                    checkpoint_local_search(&mut self.pending_cycle, &local);
                }
            }
        }
        // Keep primary/fallback admission independent of this scalar heuristic.
        // Only after the complete accepted stage may it replace a negative-room
        // winner with the original-rank-best nonnegative-room candidate.
        if let (Some(original), Some((alternative, entry))) =
            (local.selected.as_ref(), nonnegative_room_alternative)
            && pd_plan::local_clearing::prefer_nonnegative_braking_room(
                crate::local_clearing::braking_room_at_handoff(
                    &self.context,
                    &original.handoff_state,
                )
                .as_ref(),
                crate::local_clearing::braking_room_at_handoff(
                    &self.context,
                    &alternative.handoff_state,
                )
                .as_ref(),
            )
        {
            local.selected = Some(alternative);
            selected_entry = Some(entry);
        }
        let Some(proposal) = local.selected.as_ref() else {
            // The old search tests progress before certificate safety, so
            // insufficient_progress alone cannot prove a safe no-progress
            // family. Keep the finite exhaustion typed NoClearing.
            self.result.reason = Some(
                "finite local clearing grid exhausted; boundary rejection counts retained".into(),
            );
            cycle.decision = WaypointV2CycleDecision::NoClearing;
            cycle.local_search = Some(local);
            self.result.cycles.push(cycle.clone());
            self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
            return Ok(None);
        };
        let entry = selected_entry.context("selected row lacks genuine live query entry")?;
        cycle.local_search = Some(local.clone());
        self.pending_cycle = Some(cycle.clone());
        validate_local_clearing_proposal(request, &entry.state, proposal)?;
        self.result.timings.planning_s += planning_start.elapsed().as_secs_f64();
        Ok(Some(SelectedClearing { local, entry }))
    }

    fn execute_correction(
        &mut self,
        request: &WaypointDirectNominalDirectGenerationRequest,
        mut cycle: WaypointV2Cycle,
        kind: WaypointV2SegmentKind,
        selected: SelectedClearing,
        piece_entry_physics_step: u64,
    ) -> Result<PieceAdvance> {
        let SelectedClearing { mut local, entry } = selected;
        let proposal = local
            .selected
            .as_ref()
            .context("selected clearing row lost its proposal")?;
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
        self.prove_correction_sources(request, &certificate, &certificate_segments, &mut local)?;
        self.result.timings.replay_s += replay_start.elapsed().as_secs_f64();
        cycle.decision = WaypointV2CycleDecision::LocalCleared;
        cycle.local_search = Some(local);
        self.result.cycles.push(cycle);
        self.result.correction_count += 1;
        Ok(PieceAdvance::Handoff {
            piece_index: self.result.cycles.len() - 1,
            entry_physics_step: piece_entry_physics_step,
            handoff_physics_step: self.live.state.physics_step,
        })
    }

    fn prove_correction_sources(
        &self,
        request: &WaypointDirectNominalDirectGenerationRequest,
        certificate: &OrdinaryLive,
        certificate_segments: &[WaypointV2Segment],
        local: &mut WaypointV2LocalSearch,
    ) -> Result<()> {
        prove_accumulated(
            request,
            &self.context,
            &self.live,
            &self.result.segments,
            false,
            self.deadline,
        )?;
        prove_accumulated(
            request,
            &self.context,
            certificate,
            certificate_segments,
            false,
            self.deadline,
        )?;
        local.handoff_source_replay_passed = true;
        local.certificate_source_replay_passed = true;
        Ok(())
    }

    fn terminal_piece(&self, stop: WaypointV2Stop, entry_physics_step: u64) -> PieceAdvance {
        PieceAdvance::Terminal {
            planning_stop: stop,
            piece_index: terminal_piece_index(&self.result.cycles, entry_physics_step),
            entry_physics_step: Some(entry_physics_step),
            physics_step: Some(self.live.state.physics_step),
        }
    }
}

/// A synchronous V2 flight session that owns the original request and live
/// simulation. One call to [`advance_piece`](Self::advance_piece) executes one
/// direct piece or one complete nominal-to-actual-handoff correction.
pub struct WaypointV2Session {
    request: WaypointDirectNominalDirectGenerationRequest,
    flight: Option<FlightLoop>,
    preflight_result: Option<WaypointV2FlightResult>,
    terminal_stop: Option<WaypointV2Stop>,
    terminal_progress: Option<WaypointV2SessionProgress>,
    finalized: bool,
}

impl WaypointV2Session {
    /// Run input-only preflight and create the owned live simulation only for
    /// supported requests. A rejected preflight remains a typed terminal
    /// session with no simulation evidence.
    pub fn start(
        request: WaypointDirectNominalDirectGenerationRequest,
        policy: WaypointV2Policy,
    ) -> Result<Self> {
        let preflight = preflight_waypoint_v2_flight(&request, &policy);
        let initial_result = WaypointV2FlightResult {
            policy: policy.clone(),
            input_identity: nominal_direct_flight_identity(&(&request, &policy))?,
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
            let terminal_stop = preflight
                .rejection
                .context("unsupported V2 preflight omitted its typed stop")?;
            return Ok(Self {
                request,
                flight: None,
                preflight_result: Some(initial_result),
                terminal_stop: Some(terminal_stop),
                terminal_progress: Some(WaypointV2SessionProgress::Terminal {
                    piece_index: None,
                    entry_physics_step: None,
                    planning_stop: terminal_stop,
                    correction_count: 0,
                    physics_step: None,
                }),
                finalized: false,
            });
        }

        let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
        let deadline = original_deadline(
            context.sim.max_time_s,
            request.policy.analytical_policy.mission_budget_s(),
        )
        .map_err(anyhow::Error::msg)?;
        let live = new_ordinary(&context)?;
        let mut result = initial_result;
        result.absolute_deadline_physics_step = Some(deadline);
        Ok(Self {
            request,
            flight: Some(FlightLoop {
                context,
                deadline,
                live,
                result,
                pending_cycle: None,
            }),
            preflight_result: None,
            terminal_stop: None,
            terminal_progress: None,
            finalized: false,
        })
    }

    pub fn request(&self) -> &WaypointDirectNominalDirectGenerationRequest {
        &self.request
    }

    /// Inspect retained progress. Until [`finish`](Self::finish) succeeds,
    /// terminal landing progress has not passed the final whole-source proof.
    pub fn result(&self) -> &WaypointV2FlightResult {
        self.flight
            .as_ref()
            .map(|flight| &flight.result)
            .or(self.preflight_result.as_ref())
            .expect("session always retains a typed flight result")
    }

    pub fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Advance exactly one executed direct piece or nominal-to-handoff
    /// correction. An implementation error is captured as a terminal typed
    /// result after the partial state and pending cycle have been retained.
    pub fn advance_piece(&mut self) -> Result<WaypointV2SessionProgress> {
        if self.finalized || self.terminal_stop.is_some() {
            return Ok(self.terminal_progress());
        }

        let Some(flight) = self.flight.as_mut() else {
            return Ok(self.terminal_progress());
        };
        match flight.advance_piece(&self.request) {
            Ok(PieceAdvance::Handoff {
                piece_index,
                entry_physics_step,
                handoff_physics_step,
            }) => Ok(WaypointV2SessionProgress::Handoff {
                piece_index,
                correction_count: flight.result.correction_count,
                entry_physics_step,
                handoff_physics_step,
            }),
            Ok(PieceAdvance::Terminal {
                planning_stop,
                piece_index,
                entry_physics_step,
                physics_step,
            }) => {
                flight.result.planning_stop = planning_stop;
                flight.pending_cycle = None;
                self.terminal_stop = Some(planning_stop);
                let progress = WaypointV2SessionProgress::Terminal {
                    piece_index,
                    entry_physics_step,
                    planning_stop,
                    correction_count: flight.result.correction_count,
                    physics_step,
                };
                self.terminal_progress = Some(progress.clone());
                Ok(progress)
            }
            Err(error) => {
                flight.result.planning_stop = WaypointV2Stop::ImplementationError;
                flight.result.integrity_passed = false;
                flight.result.reason = Some(format!("{error:#}"));
                flight.live.evidence.final_state =
                    SimulationStateSnapshotV1::from_state(&flight.live.state);
                let pending_entry = flight
                    .pending_cycle
                    .as_ref()
                    .filter(|cycle| cycle.cycle_index == flight.result.cycles.len())
                    .map(|cycle| (cycle.cycle_index, cycle.current_state.physics_step));
                if let Some(cycle) = flight
                    .pending_cycle
                    .take()
                    .filter(|cycle| cycle.cycle_index == flight.result.cycles.len())
                {
                    flight.result.cycles.push(cycle);
                }
                self.terminal_stop = Some(WaypointV2Stop::ImplementationError);
                let active_entry = pending_entry.or_else(|| {
                    flight
                        .result
                        .cycles
                        .last()
                        .filter(|cycle| {
                            cycle.current_state.physics_step == flight.live.state.physics_step
                        })
                        .map(|cycle| (cycle.cycle_index, cycle.current_state.physics_step))
                });
                let progress = WaypointV2SessionProgress::Terminal {
                    piece_index: active_entry.map(|(index, _)| index),
                    entry_physics_step: Some(
                        active_entry
                            .map(|(_, entry)| entry)
                            .unwrap_or(flight.live.state.physics_step),
                    ),
                    planning_stop: WaypointV2Stop::ImplementationError,
                    correction_count: flight.result.correction_count,
                    physics_step: Some(flight.live.state.physics_step),
                };
                self.terminal_progress = Some(progress.clone());
                // Physical/evidence failures become a retained typed terminal
                // result. The caller can still finalize and write that bundle.
                Ok(progress)
            }
        }
    }

    /// Finalize only a terminal session. Final accumulated source replay runs
    /// once here; its failure overrides an earlier landed planning stop.
    pub fn finish(&mut self) -> Result<&WaypointV2FlightResult> {
        if self.finalized {
            return Ok(self.result());
        }
        ensure!(
            self.terminal_stop.is_some(),
            "cannot finish an active V2 session before a terminal stop"
        );

        if let Some(flight) = self.flight.as_mut() {
            let start = Instant::now();
            match prove_accumulated(
                &self.request,
                &flight.context,
                &flight.live,
                &flight.result.segments,
                false,
                flight.deadline,
            ) {
                Ok(proof) => {
                    flight.result.final_source_replay_passed = true;
                    flight.result.manifest = Some(proof.run.manifest);
                }
                Err(error) => {
                    flight.result.planning_stop = WaypointV2Stop::ImplementationError;
                    flight.result.integrity_passed = false;
                    flight.result.reason = Some(format!("final accumulated replay: {error:#}"));
                    self.terminal_stop = Some(WaypointV2Stop::ImplementationError);
                }
            }
            flight.result.timings.replay_s += start.elapsed().as_secs_f64();
            flight.result.physical_outcome = Some(flight.live.state.physical_outcome.clone());
            flight.result.mission_outcome = Some(flight.live.state.mission_outcome.clone());
            flight.result.ordinary_flight = Some(flight.live.evidence.clone());
        }

        self.finalized = true;
        let final_stop = self.result().planning_stop;
        if let Some(progress) = self.terminal_progress.as_mut()
            && let WaypointV2SessionProgress::Terminal { planning_stop, .. } = progress
        {
            *planning_stop = final_stop;
        }
        Ok(self.result())
    }

    fn terminal_progress(&self) -> WaypointV2SessionProgress {
        self.terminal_progress
            .clone()
            .expect("terminal sessions retain their typed final progress")
    }
}

/// Runs no preservation canaries and writes no files. Every continuation starts
/// from the same retained in-memory plant; snapshots are output evidence only.
pub fn run_waypoint_v2_flight(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &WaypointV2Policy,
) -> Result<WaypointV2FlightResult> {
    let mut session = WaypointV2Session::start(request.clone(), policy.clone())?;
    loop {
        if matches!(
            session.advance_piece()?,
            WaypointV2SessionProgress::Terminal { .. }
        ) {
            break;
        }
    }
    Ok(session.finish()?.clone())
}

#[cfg(test)]
mod tests;
