//! One optional early direct exit, never an alternative clearing-row search.

use anyhow::{Context, Result, ensure};
use pd_core::{RunContext, SimulationStateSnapshotV1};

use super::execution::query_to;
use super::{WaypointV2EarlyExit, WaypointV2EarlyExitDisposition as Disposition};
use crate::{
    AirborneAcquisitionSearchV1, AirborneDirectAuditV1, LocalClearingProposalV1,
    audit_airborne_acquisition_proposal, evaluate_airborne_acquisition_direct,
    local_clearing::{OrdinaryLive, advance_ordinary},
    planner_flight::terrain::has_actual_terrain_conflict,
};

pub(super) struct CheckedNominal {
    pub state: SimulationStateSnapshotV1,
    pub search: AirborneAcquisitionSearchV1,
    pub audit: AirborneDirectAuditV1,
}

pub(super) struct PreparedExit {
    pub live: OrdinaryLive,
    pub certificate: OrdinaryLive,
    pub nominal: CheckedNominal,
}

pub(super) fn first_boundary(
    proposal: &LocalClearingProposalV1,
) -> Option<&SimulationStateSnapshotV1> {
    proposal
        .trajectory
        .iter()
        .map(|v| &v.state)
        .find(|s| {
            s.physics_step > proposal.schedule.powered_end_physics_step
                && s.physics_step.is_multiple_of(2)
                && s.held_command.throttle_frac == 0.0
                && s.held_command.target_attitude_rad == 0.0
                && s.attitude_rad == 0.0
                && s.angular_rate_radps == 0.0
        })
        .filter(|s| s.physics_step < proposal.schedule.handoff_physics_step)
}

/// All plant queries advance clones of actual E. Snapshots are comparisons only.
pub(super) fn prepare(
    context: &RunContext,
    entry: &OrdinaryLive,
    proposal: &LocalClearingProposalV1,
) -> Result<(WaypointV2EarlyExit, Option<PreparedExit>)> {
    let mut record = WaypointV2EarlyExit {
        witness_handoff_physics_step: proposal.schedule.handoff_physics_step,
        query_state: None,
        continuation_end_state: None,
        minimum_continuation_clearance_m: None,
        disposition: Disposition::NoEarlierBoundary,
        reason: None,
        nominal_search: None,
        audit: None,
    };
    let Some(expected) = first_boundary(proposal) else {
        return Ok((record, None));
    };
    let live = query_to(
        context,
        entry,
        &proposal.schedule.updates,
        expected.physics_step,
    )?;
    ensure!(
        live.evidence.final_state == *expected,
        "early exit query differs from selected trace"
    );
    record.query_state = Some(expected.clone());
    let end = expected
        .physics_step
        .checked_add(proposal.policy.continuation_ticks)
        .context("early exit clock overflow")?;
    ensure!(
        end <= proposal.goal.absolute_deadline_physics_step,
        "early exit continuation exceeds original deadline"
    );
    let guard: Vec<_> = proposal
        .trajectory
        .iter()
        .filter(|s| (expected.physics_step..=end).contains(&s.state.physics_step))
        .collect();
    ensure!(
        guard.len() as u64 == proposal.policy.continuation_ticks + 1,
        "incomplete early exit continuation trace"
    );
    let minimum = guard
        .iter()
        .map(|s| s.body_clearance_m)
        .fold(f64::INFINITY, f64::min);
    ensure!(
        guard.iter().all(|s| s.body_clearance_m.is_finite()) && minimum.is_finite(),
        "early exit continuation has nonfinite clearance"
    );
    record.minimum_continuation_clearance_m = Some(minimum);
    if minimum < proposal.policy.minimum_clearance_m {
        record.disposition = Disposition::UnsupportedContinuation;
        record.reason = Some("selected coast lacks the unchanged body reserve".into());
        return Ok((record, None));
    }
    let mut certificate = live.clone();
    for tick in (expected.physics_step..=end).step_by(2) {
        if let Some(reason) = crate::live_rejection(
            context,
            &certificate.state,
            proposal.goal.absolute_deadline_physics_step,
        ) {
            record.disposition = Disposition::UnsupportedContinuation;
            record.reason = Some(format!("tick {tick}: {reason}"));
            return Ok((record, None));
        }
        if tick == end {
            break;
        }
        let update = proposal
            .schedule
            .updates
            .iter()
            .find(|u| u.physics_step == tick)
            .context("early exit coast command gap")?;
        ensure!(
            update.command.throttle_frac == 0.0,
            "early exit continuation is not coasting"
        );
        advance_ordinary(
            context,
            &mut certificate,
            std::slice::from_ref(update),
            tick + 2,
        )?;
        let expected = proposal
            .trajectory
            .iter()
            .find(|s| s.state.physics_step == tick + 2)
            .context("early exit coast trace gap")?;
        ensure!(
            certificate.evidence.final_state == expected.state,
            "early exit coast state differs from selected trace"
        );
    }
    record.continuation_end_state = Some(certificate.evidence.final_state.clone());
    // Exactly one existing construction call, after the unchanged local guards.
    let search = evaluate_airborne_acquisition_direct(
        context,
        &live.state,
        proposal.goal.absolute_deadline_physics_step,
    )?;
    ensure!(
        search.unsupported_reason.is_none(),
        "guarded early exit became unsupported"
    );
    record.nominal_search = Some(search.clone());
    let Some(selected) = search.selected.as_ref() else {
        record.disposition = Disposition::NoNominal;
        return Ok((record, None));
    };
    let audit = audit_airborne_acquisition_proposal(
        context,
        &live.state,
        selected,
        proposal.policy.minimum_clearance_m,
    )?;
    ensure!(
        audit.ordinary_neutral_parity
            && (audit.commands_match || has_actual_terrain_conflict(&audit)),
        "early exit audit integrity mismatch"
    );
    record.audit = Some(audit.clone());
    if !audit.passed {
        record.disposition = if has_actual_terrain_conflict(&audit) {
            Disposition::TerrainBlocked
        } else {
            Disposition::NominalRejected
        };
        return Ok((record, None));
    }
    ensure!(
        audit.safe_target_contact
            && audit.commands_match
            && audit.clearance_scan.all_airborne_states_passed,
        "early exit audit claims inconsistent safety"
    );
    record.disposition = Disposition::ClearReady;
    Ok((
        record,
        Some(PreparedExit {
            nominal: CheckedNominal {
                state: live.evidence.final_state.clone(),
                search,
                audit,
            },
            live,
            certificate,
        }),
    ))
}

/// Saved evidence must distinguish an actual early end from the untouched H
/// witness and bind the next piece to the exact checked nominal. No replay here.
pub(crate) fn validate_records(result: &super::WaypointV2FlightResult) -> Result<()> {
    for cycle in &result.cycles {
        let Some(local) = &cycle.local_search else {
            continue;
        };
        let Some(exit) = &local.early_exit else {
            continue;
        };
        let proposal = local
            .selected
            .as_ref()
            .context("early query lacks selected clearing witness")?;
        ensure!(
            exit.witness_handoff_physics_step == proposal.schedule.handoff_physics_step
                && proposal.handoff_state.physics_step == exit.witness_handoff_physics_step,
            "early query original witness binding differs"
        );
        let Some(expected) = first_boundary(proposal) else {
            ensure!(
                exit.disposition == Disposition::NoEarlierBoundary
                    && exit.query_state.is_none()
                    && exit.nominal_search.is_none()
                    && exit.audit.is_none()
                    && exit.continuation_end_state.is_none()
                    && exit.minimum_continuation_clearance_m.is_none(),
                "invented early query without an earlier boundary"
            );
            continue;
        };
        ensure!(
            exit.query_state.as_ref() == Some(expected),
            "early query is not the first settled coast boundary"
        );
        let end = expected
            .physics_step
            .checked_add(proposal.policy.continuation_ticks)
            .context("saved early continuation overflow")?;
        let guard: Vec<_> = proposal
            .trajectory
            .iter()
            .filter(|s| (expected.physics_step..=end).contains(&s.state.physics_step))
            .collect();
        let minimum = guard
            .iter()
            .map(|s| s.body_clearance_m)
            .fold(f64::INFINITY, f64::min);
        ensure!(
            end <= proposal.goal.absolute_deadline_physics_step
                && guard.len() as u64 == proposal.policy.continuation_ticks + 1
                && guard.iter().all(|s| s.body_clearance_m.is_finite())
                && exit.minimum_continuation_clearance_m == Some(minimum),
            "saved early coast reserve/clock differs"
        );
        if exit.disposition == Disposition::UnsupportedContinuation {
            ensure!(
                exit.reason.is_some()
                    && exit.nominal_search.is_none()
                    && exit.audit.is_none()
                    && exit.continuation_end_state.is_none(),
                "unsupported early continuation has a nominal query"
            );
            continue;
        }
        ensure!(
            minimum >= proposal.policy.minimum_clearance_m
                && exit.continuation_end_state.as_ref() == guard.last().map(|s| &s.state),
            "saved early continuation endpoint differs"
        );
        let search = exit
            .nominal_search
            .as_ref()
            .context("early nominal query missing its search")?;
        let state = &search.incoming_state;
        ensure!(
            state.physics_step == expected.physics_step
                && state.sim_time_s == expected.sim_time_s
                && state.position_m == expected.position_m
                && state.velocity_mps == expected.velocity_mps
                && state.fuel_kg == expected.fuel_kg
                && state.attitude_rad == expected.attitude_rad
                && state.angular_rate_radps == expected.angular_rate_radps
                && state.held_command == expected.held_command
                && search.absolute_deadline_physics_step
                    == proposal.goal.absolute_deadline_physics_step
                && search.unsupported_reason.is_none(),
            "early nominal query changed incoming state or deadline"
        );
        let Some(selected) = &search.selected else {
            ensure!(
                exit.disposition == Disposition::NoNominal && exit.audit.is_none(),
                "early no-nominal query was relabeled"
            );
            continue;
        };
        ensure!(
            selected.incoming_state == search.incoming_state
                && selected.absolute_deadline_physics_step == search.absolute_deadline_physics_step,
            "early selected nominal origin differs"
        );
        let audit = exit
            .audit
            .as_ref()
            .context("early selected nominal lacks terrain audit")?;
        ensure!(
            audit.ordinary_neutral_parity
                && (audit.commands_match || has_actual_terrain_conflict(audit)),
            "saved early audit integrity differs"
        );
        if !audit.passed {
            let expected = if has_actual_terrain_conflict(audit) {
                Disposition::TerrainBlocked
            } else {
                Disposition::NominalRejected
            };
            ensure!(
                exit.disposition == expected,
                "rejected early audit was committed or relabeled"
            );
            continue;
        }
        ensure!(
            audit.safe_target_contact
                && audit.commands_match
                && audit.clearance_scan.all_airborne_states_passed
                && audit.clearance_scan.first_violation.is_none()
                && audit.rejection_reasons.is_empty()
                && selected.end_state.physics_step == audit.final_state.physics_step
                && audit.final_state.physical_outcome == pd_core::PhysicalOutcome::LandedOnTarget
                && audit.final_state.mission_outcome == pd_core::MissionOutcome::Success,
            "saved early clear audit lacks safety or endpoint agreement"
        );
        // The selected neutral endpoint is incoming contact, not the ordinary
        // post-contact state whose velocity/position the plant may settle.
        let contact = audit
            .incoming_contact
            .as_ref()
            .context("early clear audit lacks incoming contact")?;
        let state = &selected.end_state;
        let incoming = &contact.state;
        ensure!(
            state.physics_step == incoming.physics_step
                && state.sim_time_s == incoming.sim_time_s
                && state.position_m == incoming.position_m
                && state.velocity_mps == incoming.velocity_mps
                && state.fuel_kg == incoming.fuel_kg
                && state.attitude_rad == incoming.attitude_rad
                && state.angular_rate_radps == incoming.angular_rate_radps
                && state.held_command == incoming.held_command,
            "early checked nominal incoming-contact endpoint differs"
        );
        if exit.disposition == Disposition::ClearReady {
            ensure!(
                !result.integrity_passed,
                "uncommitted early clear query in a verified complete result"
            );
            continue;
        }
        ensure!(
            exit.disposition == Disposition::Committed
                && local.handoff_source_replay_passed
                && local.certificate_source_replay_passed
                && local.certificate_state == exit.continuation_end_state
                && cycle.decision == super::WaypointV2CycleDecision::LocalCleared,
            "early commit lacks executed piece/certificate proofs"
        );
        let segment = result
            .segments
            .iter()
            .find(|s| {
                s.kind == super::WaypointV2SegmentKind::LocalCorrection
                    && s.start_physics_step == proposal.entry_state.physics_step
            })
            .context("early commit lacks actual correction segment")?;
        ensure!(
            segment.entry_state == proposal.entry_state
                && segment.end_state == *expected
                && segment.end_physics_step == expected.physics_step
                && segment.proposal_identity == proposal.identity
                && segment.updates
                    == super::execution::consumed(
                        &proposal.schedule.updates,
                        expected.physics_step
                    ),
            "early executed segment differs from selected prefix"
        );
        if let Some(next) = result.cycles.get(cycle.cycle_index + 1) {
            ensure!(
                next.current_state == *expected
                    && next.nominal_search_identity == search.identity
                    && next.nominal_proposal_identity.as_ref() == Some(&selected.identity)
                    && next.nominal_updates == selected.updates
                    && next.audit.as_ref() == Some(audit),
                "next piece did not consume the exact checked early nominal"
            );
        } else {
            ensure!(
                result.planning_stop != super::WaypointV2Stop::Landed,
                "early exit claims landing without consuming checked nominal"
            );
        }
    }
    Ok(())
}
