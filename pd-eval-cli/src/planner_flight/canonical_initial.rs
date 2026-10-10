//! Current planner canonical_initial; no research orchestration.
use super::*;
use anyhow::{Context, Result, anyhow, bail};
use pd_core::{
    ContactClassification, EndReason, FlightProgramUpdateV1, MissionOutcome, PhysicalOutcome,
    RunContext, SimulationState, SimulationStateSnapshotV1,
};
use pd_plan::ballistic::{
    CanonicalInitialBallisticBasisV1, CanonicalInitialEndpointsV1, KinematicStateV2, PadInputV2,
    canonical_initial_ballistic_bases_v1,
};
use serde::{Deserialize, Serialize};

mod source;

pub const CANONICAL_INITIAL_DIRECT_POLICY_ID: &str = "canonical_initial_direct_lowest_peak_v1";

const SOURCE_OFFSETS: [i64; 5] = [-240, -180, -120, -60, 0];

const TERMINAL_OFFSETS: [u64; 7] = [0, 60, 120, 180, 240, 300, 360];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialProposalV1 {
    pub policy_id: String,
    pub dynamics_identity: String,
    pub absolute_deadline_physics_step: u64,
    pub initial_state: AirborneFlightStateV1,
    pub source_handoff_physics_step: u64,
    pub source_handoff_state: AirborneFlightStateV1,
    pub coast_tick_count: u64,
    pub terminal_tick_count: u64,
    pub planned_end_physics_step: u64,
    pub updates: Vec<FlightProgramUpdateV1>,
    pub end_state: AirborneFlightStateV1,
    pub peak_com_height_m: f64,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialAttemptV1 {
    pub row_index: usize,
    pub basis_index: usize,
    pub source_duration_offset_ticks: i64,
    pub terminal_duration_offset_ticks: u64,
    pub source_bridge_tick_count: Option<u64>,
    pub source_fit_iterations: Option<usize>,
    pub source_position_error_m: Option<f64>,
    pub source_velocity_error_mps: Option<f64>,
    pub status: String,
    pub reason: Option<String>,
    pub proposal_identity: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialSearchV1 {
    pub policy_id: String,
    pub dynamics_identity: String,
    pub absolute_deadline_physics_step: u64,
    pub initial_state: AirborneFlightStateV1,
    pub bases: Vec<CanonicalInitialBallisticBasisV1>,
    pub attempts: Vec<CanonicalInitialAttemptV1>,
    pub selected: Option<CanonicalInitialProposalV1>,
    pub preflight_rejection: Option<NominalDirectFlightDecisionV1>,
    pub identity: String,
}

pub(super) fn dynamics_identity(context: &RunContext, source: &PadInputV2) -> Result<String> {
    stable_digest(&(
        CANONICAL_INITIAL_DIRECT_POLICY_ID,
        &context.sim,
        &context.vehicle,
        context.world.gravity_mps2,
        source,
        &context.target_pad,
        &context.initial_state,
        crate::WaypointDirectNominalDirectGenerationPolicyV1::default(),
        BodyAwareTerminalPolicyV1::default(),
    ))
}

pub(super) fn proposal_identity(proposal: &CanonicalInitialProposalV1) -> Result<String> {
    let mut copy = proposal.clone();
    copy.identity.clear();
    stable_digest(&copy)
}

pub fn evaluate_canonical_initial_direct(
    request: &WaypointDirectNominalDirectGenerationRequest,
) -> Result<CanonicalInitialSearchV1> {
    let preflight = preflight_nominal_direct_flight(request, &BodyAwareTerminalPolicyV1::default());
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let source_pad = request
        .scenario
        .world
        .landing_pad(&request.source_pad_id)
        .context("canonical source pad missing")?;
    let source = PadInputV2 {
        center_x_m: source_pad.center_x_m,
        surface_y_m: source_pad.surface_y_m,
        width_m: source_pad.width_m,
    };
    let initial = SimulationState::new(&context)?;
    let deadline = (request
        .policy
        .analytical_policy
        .mission_budget_s()
        .min(context.sim.max_time_s)
        * 120.0)
        .floor() as u64;
    let mut search = CanonicalInitialSearchV1 {
        policy_id: CANONICAL_INITIAL_DIRECT_POLICY_ID.into(),
        dynamics_identity: dynamics_identity(&context, &source)?,
        absolute_deadline_physics_step: deadline,
        initial_state: AirborneFlightStateV1::from_live(&initial),
        bases: Vec::new(),
        attempts: Vec::new(),
        selected: None,
        preflight_rejection: preflight.rejection,
        identity: String::new(),
    };
    if search.preflight_rejection.is_none() {
        let vehicle = vehicle_input_v2(&context.vehicle);
        let target = PadInputV2 {
            center_x_m: context.target_pad.center_x_m,
            surface_y_m: context.target_pad.surface_y_m,
            width_m: context.target_pad.width_m,
        };
        search.bases = canonical_initial_ballistic_bases_v1(
            &request.policy.analytical_policy,
            &vehicle,
            &CanonicalInitialEndpointsV1 {
                source: source.clone(),
                target,
                initial: KinematicStateV2 {
                    position_m: initial.position_m,
                    velocity_mps: initial.velocity_mps,
                },
            },
        )
        .map_err(anyhow::Error::msg)?;
        if search.bases.len() != 4 {
            bail!("canonical family requires exactly four ballistic bases");
        }
        for (basis_index, basis) in search.bases.iter().enumerate() {
            for offset in SOURCE_OFFSETS {
                let ticks = basis
                    .selected
                    .as_ref()
                    .and_then(|p| p.source_bridge_ticks.checked_add_signed(offset));
                let prefix = if let (Some(pair), Some(ticks)) = (&basis.selected, ticks) {
                    if ticks.checked_add(72).is_none_or(|end| end >= deadline) {
                        Err(anyhow!(
                            "nominal source prefix exhausts original absolute deadline"
                        ))
                    } else {
                        source::build_source_prefix(
                            &context,
                            &source,
                            &vehicle,
                            &request.policy.analytical_policy,
                            pair.source_handoff.state,
                            ticks,
                        )
                    }
                } else {
                    Err(anyhow!("unavailable nominal basis or source duration"))
                };
                for terminal_offset in TERMINAL_OFFSETS {
                    let mut attempt = CanonicalInitialAttemptV1 {
                        row_index: search.attempts.len(),
                        basis_index,
                        source_duration_offset_ticks: offset,
                        terminal_duration_offset_ticks: terminal_offset,
                        source_bridge_tick_count: ticks,
                        source_fit_iterations: prefix.as_ref().ok().map(|p| p.iterations),
                        source_position_error_m: prefix.as_ref().ok().map(|p| p.position_error_m),
                        source_velocity_error_mps: prefix
                            .as_ref()
                            .ok()
                            .map(|p| p.velocity_error_mps),
                        status: "rejected".into(),
                        reason: None,
                        proposal_identity: None,
                    };
                    let result = (|| -> Result<CanonicalInitialProposalV1> {
                        let prefix = prefix.as_ref().map_err(|e| anyhow!("{e:#}"))?;
                        let pair = basis.selected.as_ref().context("no nominal pair")?;
                        let coast = pair.coast_ticks.div_ceil(2) * 2;
                        let terminal = pair.terminal_bridge_ticks + terminal_offset;
                        let entry = predicted_coast_kinematics(&context, &prefix.live, coast);
                        let apex_tick = (prefix.live.velocity_mps.y / context.world.gravity_mps2
                            * 120.0)
                            .ceil() as u64;
                        let apex = predicted_coast_kinematics(&context, &prefix.live, apex_tick);
                        if prefix.live.velocity_mps.y <= 0.0
                            || entry.velocity_mps.y >= 0.0
                            || coast == 0
                            || apex_tick >= coast
                            || apex.position_m.y
                                <= context.target_pad.surface_y_m
                                    + context.vehicle.geometry.touchdown_base_offset_m
                                    + request.policy.analytical_policy.minimum_clearance_m
                        {
                            bail!(
                                "nominal coast must contain apex above target and descending terminal entry"
                            );
                        }
                        let suffix =
                            airborne::candidate(&context, &prefix.live, coast, terminal, deadline)?;
                        let mut updates = prefix.updates.clone();
                        updates.extend(suffix.updates);
                        let mut proposal = CanonicalInitialProposalV1 {
                            policy_id: search.policy_id.clone(),
                            dynamics_identity: search.dynamics_identity.clone(),
                            absolute_deadline_physics_step: deadline,
                            initial_state: search.initial_state.clone(),
                            source_handoff_physics_step: prefix.live.physics_step,
                            source_handoff_state: AirborneFlightStateV1::from_live(&prefix.live),
                            coast_tick_count: coast,
                            terminal_tick_count: terminal,
                            planned_end_physics_step: suffix.planned_end_physics_step,
                            updates,
                            end_state: suffix.end_state,
                            peak_com_height_m: prefix.peak.max(suffix.peak_com_height_m),
                            identity: String::new(),
                        };
                        proposal.identity = proposal_identity(&proposal)?;
                        Ok(proposal)
                    })();
                    match result {
                        Ok(proposal) => {
                            attempt.status = "nominal_proposal".into();
                            attempt.proposal_identity = Some(proposal.identity.clone());
                            if search.selected.as_ref().is_none_or(|old| {
                                proposal
                                    .peak_com_height_m
                                    .total_cmp(&old.peak_com_height_m)
                                    .then_with(|| {
                                        proposal
                                            .planned_end_physics_step
                                            .cmp(&old.planned_end_physics_step)
                                    })
                                    .then_with(|| proposal.identity.cmp(&old.identity))
                                    .is_lt()
                            }) {
                                search.selected = Some(proposal);
                            }
                        }
                        Err(error) => attempt.reason = Some(format!("{error:#}")),
                    }
                    search.attempts.push(attempt);
                }
            }
        }
        if search.attempts.len() != 140 {
            bail!("canonical nominal ledger omitted an outer row");
        }
    }
    search.identity = stable_digest(&search)?;
    Ok(search)
}

/// Independent ordinary/neutral full-source audit. It cannot rerank proposals.
pub fn audit_canonical_initial_direct(
    context: &RunContext,
    source: &PadInputV2,
    proposal: &CanonicalInitialProposalV1,
    minimum_clearance_m: f64,
) -> Result<AirborneDirectAuditV1> {
    let mut neutral = SimulationState::new(context)?;
    let mut ordinary = neutral.clone();
    if context.sim.physics_hz != 120
        || context.sim.controller_hz != 60
        || proposal.policy_id != CANONICAL_INITIAL_DIRECT_POLICY_ID
        || proposal.dynamics_identity != dynamics_identity(context, source)?
        || proposal.initial_state != AirborneFlightStateV1::from_live(&neutral)
        || proposal.identity != proposal_identity(proposal)?
        || !minimum_clearance_m.is_finite()
        || minimum_clearance_m < 0.0
        || proposal.planned_end_physics_step == 0
        || proposal.planned_end_physics_step > proposal.absolute_deadline_physics_step
        || proposal.absolute_deadline_physics_step
            != (crate::WaypointDirectNominalDirectGenerationPolicyV1::default()
                .analytical_policy
                .mission_budget_s()
                .min(context.sim.max_time_s)
                * 120.0)
                .floor() as u64
        || proposal.absolute_deadline_physics_step > (context.sim.max_time_s * 120.0).floor() as u64
        || proposal.source_handoff_physics_step <= 72
        || !proposal.source_handoff_physics_step.is_multiple_of(2)
        || proposal.coast_tick_count == 0
        || !proposal.coast_tick_count.is_multiple_of(2)
        || proposal.terminal_tick_count == 0
        || !proposal.terminal_tick_count.is_multiple_of(2)
        || proposal
            .source_handoff_physics_step
            .checked_add(proposal.coast_tick_count)
            .is_none_or(|entry| entry >= proposal.planned_end_physics_step)
        || proposal.updates.len() as u64 != proposal.planned_end_physics_step.div_ceil(2)
        || proposal.updates.iter().enumerate().any(|(i, u)| {
            u.physics_step != i as u64 * 2
                || u.phase
                    != if u.physics_step < 60 {
                        "upright"
                    } else if u.physics_step < 72 {
                        "tilt"
                    } else if u.physics_step < proposal.source_handoff_physics_step {
                        "source_bridge"
                    } else if u.physics_step
                        < proposal.source_handoff_physics_step + proposal.coast_tick_count
                    {
                        "ballistic_coast"
                    } else {
                        "terminal_bridge"
                    }
                || !u.command.throttle_frac.is_finite()
                || !u.command.target_attitude_rad.is_finite()
                || u.command != u.command.clamped()
        })
    {
        bail!("canonical proposal binding, clock, or coverage is invalid");
    }
    let target = PadInputV2 {
        center_x_m: context.target_pad.center_x_m,
        surface_y_m: context.target_pad.surface_y_m,
        width_m: context.target_pad.width_m,
    };
    let policy = ClearancePolicy {
        source_pad: flat_pad_bounds(context, source),
        target_pad: flat_pad_bounds(context, &target),
        minimum_clearance_m,
    };
    let mut scan = empty_clearance_scan();
    let mut parity = true;
    let mut index = 0;
    let mut contact = None;
    let mut incoming_contact = None;
    let mut source_handoff_matches = false;
    while neutral.physics_step < proposal.planned_end_physics_step && !ordinary.is_terminal() {
        if neutral.physics_step.is_multiple_of(2) {
            let update = &proposal.updates[index];
            neutral.set_command(update.command);
            ordinary.set_command(update.command);
            index += 1;
        }
        let phase = &proposal.updates[index.saturating_sub(1)].phase;
        let classification = neutral.step_physics_and_classify_contact(context);
        let report = ordinary.step_with_contact_report(context);
        scan.poststep_state_count += 1;
        if neutral.physics_step == proposal.source_handoff_physics_step {
            source_handoff_matches =
                AirborneFlightStateV1::from_live(&neutral) == proposal.source_handoff_state;
        }
        if classification == ContactClassification::None {
            parity &= report.incoming_contact.is_none()
                && SimulationStateSnapshotV1::from_state(&neutral)
                    == SimulationStateSnapshotV1::from_state(&ordinary);
            record_airborne_clearance(
                context,
                &neutral,
                neutral.physics_step,
                phase,
                policy,
                &mut scan,
            );
        } else {
            parity &= report.incoming_contact.as_ref().is_some_and(|incoming| {
                incoming.classification == classification
                    && incoming.state == SimulationStateSnapshotV1::from_state(&neutral)
            });
            contact = Some(contact_audit(context, &neutral, &classification));
            incoming_contact = report.incoming_contact;
            break;
        }
    }
    let commands_match = index == proposal.updates.len()
        && source_handoff_matches
        && neutral.physics_step == proposal.planned_end_physics_step
        && AirborneFlightStateV1::from_live(&neutral) == proposal.end_state;
    let safe = contact.as_ref().is_some_and(|c| {
        c.classification == "stable_touchdown_on_target"
            && c.core_matches_predicate_mirror
            && c.body_within_strict_terrain_domain
    }) && ordinary.physical_outcome == PhysicalOutcome::LandedOnTarget
        && ordinary.mission_outcome == MissionOutcome::Success
        && ordinary.end_reason == EndReason::TouchdownOnTarget;
    let mut reasons = Vec::new();
    if !safe {
        reasons.push("actual first contact is not a safe target landing".into());
    }
    if !commands_match {
        reasons
            .push("actual trajectory stops before or differs from fixed nominal endpoint".into());
    }
    if !parity {
        reasons.push("ordinary/neutral full-source replay mismatch".into());
    }
    if !scan.all_airborne_states_passed {
        reasons.push("actual terrain violates declared body clearance".into());
    }
    Ok(AirborneDirectAuditV1 {
        passed: reasons.is_empty(),
        safe_target_contact: safe,
        ordinary_neutral_parity: parity,
        commands_match,
        first_contact: contact,
        incoming_contact,
        clearance_scan: scan,
        final_state: SimulationStateSnapshotV1::from_state(&ordinary),
        rejection_reasons: reasons,
    })
}
#[cfg(test)]
mod tests;
