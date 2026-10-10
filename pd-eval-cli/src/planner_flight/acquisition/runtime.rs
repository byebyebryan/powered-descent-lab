//! Current planner acquisition runtime; no research orchestration.
use super::*;
use anyhow::{Context, Result, bail};
use pd_core::{ContactClassification, EndReason, MissionOutcome, PhysicalOutcome};
use pd_plan::ballistic::PadInputV2;
use serde::{Deserialize, Serialize};

pub const AIRBORNE_ACQUISITION_POLICY_ID: &str = "airborne_acquisition_terminal_time_v1";

const PHASES: [&str; 4] = [
    "nominal_acquisition_turn",
    "nominal_acquisition",
    "ballistic_coast",
    "terminal_bridge",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirborneAcquisitionProposalV1 {
    pub policy_id: String,
    pub dynamics_identity: String,
    pub absolute_deadline_physics_step: u64,
    pub incoming_state: AirborneFlightStateV1,
    pub seed_id: String,
    pub entry_index: usize,
    pub acquisition_turn_end_physics_step: u64,
    pub acquisition_end_physics_step: u64,
    pub terminal_entry_physics_step: u64,
    pub planned_end_physics_step: u64,
    pub updates: Vec<FlightProgramUpdateV1>,
    pub end_state: AirborneFlightStateV1,
    pub peak_com_height_m: f64,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirborneAcquisitionAttemptV1 {
    pub seed_id: String,
    pub entry_index: usize,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirborneAcquisitionSearchV1 {
    pub policy_id: String,
    pub absolute_deadline_physics_step: u64,
    pub incoming_state: AirborneFlightStateV1,
    pub seeds: Vec<NominalSeedEvidenceV1>,
    pub attempts: Vec<AirborneAcquisitionAttemptV1>,
    pub selected: Option<AirborneAcquisitionProposalV1>,
    pub unsupported_reason: Option<String>,
    pub identity: String,
}

pub(super) fn dynamics_identity(context: &RunContext) -> Result<String> {
    // Interior terrain and accumulated clearance metrics are not selection inputs.
    stable_digest(&(
        AIRBORNE_ACQUISITION_POLICY_ID,
        &context.sim,
        &context.vehicle,
        context.world.gravity_mps2,
        &context.target_pad,
        BodyAwareTerminalPolicyV1::default(),
        ACQUISITION_BURN_FRACTIONS,
        ENTRY_COAST_FRACTIONS,
        MAX_TURN_UPDATES,
        MAX_PHYSICAL_WITNESSES,
        MAX_THRUST_FRACTION,
        "baseline_retaining_terminal_time_v1",
    ))
}

pub(super) fn proposal_identity(proposal: &AirborneAcquisitionProposalV1) -> Result<String> {
    let mut canonical = proposal.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

fn executable_witness(witness: &NominalWitnessEvidenceV1) -> Result<bool> {
    match witness.status.as_str() {
        "free_space_target_plane_witness" => {
            if !witness.independent_replay_passed
                || !witness.endpoint_state_agreement
                || !witness.target_plane_witness.as_ref().is_some_and(|plane| {
                    plane.safe_by_existing_target_plane_mirror
                        && plane.proposal_endpoint_contact_match
                        && plane.free_space_endpoint_match
                })
            {
                bail!("airborne witness success contradicts its replay/endpoint evidence");
            }
            Ok(true)
        }
        "finite_backend_rejection" if witness.independent_replay_passed => Ok(false),
        "finite_miss" if witness.independent_replay_passed && witness.endpoint_state_agreement => {
            Ok(false)
        }
        _ => bail!(
            "airborne realization is not an explicit finite rejection or replay-proven success: {}: {:?}",
            witness.status,
            witness.reason
        ),
    }
}

/// Construct from the caller's actual state and original deadline. Stop at the
/// first executable proposal in the unchanged analytical rank, before terrain.
pub fn evaluate_airborne_acquisition_direct(
    context: &RunContext,
    live: &SimulationState,
    deadline: u64,
) -> Result<AirborneAcquisitionSearchV1> {
    context.sim.validate().map_err(anyhow::Error::msg)?;
    context.vehicle.validate().map_err(anyhow::Error::msg)?;
    context.target_pad.validate().map_err(anyhow::Error::msg)?;
    if !finite_live_state(live)
        || !context.world.gravity_mps2.is_finite()
        || context.world.gravity_mps2 <= 0.0
    {
        bail!("nonfinite airborne origin or invalid gravity");
    }
    let mut search = AirborneAcquisitionSearchV1 {
        policy_id: AIRBORNE_ACQUISITION_POLICY_ID.into(),
        absolute_deadline_physics_step: deadline,
        incoming_state: AirborneFlightStateV1::from_live(live),
        seeds: Vec::new(),
        attempts: Vec::new(),
        selected: None,
        unsupported_reason: super::super::airborne::live_rejection(context, live, deadline),
        identity: String::new(),
    };
    if search.unsupported_reason.is_none() {
        for spec in generate_seed_specs(context, live)? {
            let (seed, _) = terminal_time::evaluate_timed_seed(context, live, deadline, spec)?;
            if seed.status == "integrity_error" {
                bail!(
                    "acquisition estimate integrity failure: {}: {:?}",
                    seed.seed_id,
                    seed.reason
                );
            }
            search.seeds.push(seed);
        }
        if search.seeds.len() > MAX_SEEDS {
            bail!("airborne seed budget exceeded");
        }
        let mut ranked = search
            .seeds
            .iter()
            .enumerate()
            .filter_map(|(i, seed)| seed.selected_entry_index.map(|entry| (i, entry)))
            .collect::<Vec<_>>();
        ranked.sort_by(|(a, ae), (b, be)| {
            compare_rank(&search.seeds[*a], *ae, &search.seeds[*b], *be)
        });
        for (seed_index, entry_index) in ranked.into_iter().take(MAX_PHYSICAL_WITNESSES) {
            let seed = &search.seeds[seed_index];
            let realized = realize_free_space_witness(
                context,
                live,
                deadline,
                seed,
                &seed.entry_screens[entry_index],
            );
            let mut attempt = AirborneAcquisitionAttemptV1 {
                seed_id: seed.seed_id.clone(),
                entry_index,
                status: "rejected".into(),
                reason: None,
            };
            match realized {
                Err(WitnessMaterializationFailure::Integrity(reason)) => {
                    bail!("airborne realization integrity failure: {reason}")
                }
                Err(WitnessMaterializationFailure::FiniteMiss(reason)) => {
                    attempt.reason = Some(reason)
                }
                Ok(realized) => {
                    let w = realized.witness;
                    if executable_witness(&w)? {
                        if w.target_plane_witness
                            .as_ref()
                            .map(|plane| plane.physics_step)
                            != Some(realized.end_state.physics_step)
                        {
                            bail!("airborne target-plane clock contradicts its realized endpoint");
                        }
                        let mut proposal = AirborneAcquisitionProposalV1 {
                            policy_id: AIRBORNE_ACQUISITION_POLICY_ID.into(),
                            dynamics_identity: dynamics_identity(context)?,
                            absolute_deadline_physics_step: deadline,
                            incoming_state: search.incoming_state.clone(),
                            seed_id: seed.seed_id.clone(),
                            entry_index,
                            acquisition_turn_end_physics_step: live
                                .physics_step
                                .checked_add(seed.turn_physics_ticks)
                                .context("turn endpoint overflow")?,
                            acquisition_end_physics_step: w.actual_acquisition_end.physics_step,
                            terminal_entry_physics_step: w
                                .actual_terminal_entry
                                .as_ref()
                                .context("executable proposal missing actual terminal entry")?
                                .physics_step,
                            planned_end_physics_step: realized.end_state.physics_step,
                            updates: w.commands,
                            end_state: realized.end_state,
                            peak_com_height_m: realized.peak_com_height_m,
                            identity: String::new(),
                        };
                        validate_program(&proposal)?;
                        proposal.identity = proposal_identity(&proposal)?;
                        search.selected = Some(proposal);
                        attempt.status = "nominal_proposal".into();
                    } else {
                        attempt.reason = w.reason.or_else(|| {
                            Some("no executable free-space target-plane witness".into())
                        });
                    }
                }
            }
            search.attempts.push(attempt);
            if search.selected.is_some() {
                break;
            }
        }
    }
    search.identity = stable_digest(&search)?;
    Ok(search)
}

fn validate_program(proposal: &AirborneAcquisitionProposalV1) -> Result<()> {
    let start = proposal.incoming_state.physics_step;
    let end = proposal.planned_end_physics_step;
    if end <= start
        || end > proposal.absolute_deadline_physics_step
        || proposal.end_state.physics_step != end
        || !proposal.peak_com_height_m.is_finite()
    {
        bail!("airborne program endpoint or deadline is invalid");
    }
    let endpoint = &proposal.end_state;
    if ![
        endpoint.sim_time_s,
        endpoint.position_m.x,
        endpoint.position_m.y,
        endpoint.velocity_mps.x,
        endpoint.velocity_mps.y,
        endpoint.attitude_rad,
        endpoint.angular_rate_radps,
        endpoint.fuel_kg,
        endpoint.held_command.throttle_frac,
        endpoint.held_command.target_attitude_rad,
    ]
    .iter()
    .all(|v| v.is_finite())
        || endpoint.sim_time_s != end as f64 / 120.0
        || endpoint.fuel_kg <= 0.0
        || endpoint.fuel_kg > proposal.incoming_state.fuel_kg
        || endpoint.held_command != endpoint.held_command.clamped()
    {
        bail!("airborne program has invalid endpoint dynamics");
    }
    validate_command_schedule(start, end, &proposal.updates)?;
    let turn = proposal.acquisition_turn_end_physics_step;
    let acquisition = proposal.acquisition_end_physics_step;
    let terminal = proposal.terminal_entry_physics_step;
    if !(start <= turn && turn <= acquisition && acquisition <= terminal && terminal < end)
        || ![turn, acquisition, terminal]
            .iter()
            .all(|t| t.is_multiple_of(HELD_TICKS))
    {
        bail!("airborne phase boundaries are invalid");
    }
    let mut last_phase = 0;
    for u in &proposal.updates {
        let phase = PHASES
            .iter()
            .position(|p| *p == u.phase)
            .context("unsupported airborne command phase")?;
        let expected_phase = if u.physics_step < turn {
            0
        } else if u.physics_step < acquisition {
            1
        } else if u.physics_step < terminal {
            2
        } else {
            3
        };
        if phase != expected_phase {
            bail!("airborne phase does not own its bound command interval");
        }
        if phase < last_phase {
            bail!("airborne command phases reverse their ownership order");
        }
        if matches!(phase, 0 | 2) && u.command.throttle_frac != 0.0 {
            bail!("airborne turn/coast phase has powered thrust");
        }
        last_phase = phase;
    }
    if last_phase != 3 {
        bail!("airborne program does not end in terminal phase");
    }
    Ok(())
}

/// Audit exactly the complete selected command program on actual terrain.
/// Post-step boundaries belong to the command consumed, including odd contact.
pub fn audit_airborne_acquisition_proposal(
    context: &RunContext,
    live: &SimulationState,
    proposal: &AirborneAcquisitionProposalV1,
    minimum_clearance_m: f64,
) -> Result<AirborneDirectAuditV1> {
    if !finite_live_state(live)
        || proposal.policy_id != AIRBORNE_ACQUISITION_POLICY_ID
        || proposal.dynamics_identity != dynamics_identity(context)?
        || proposal.incoming_state != AirborneFlightStateV1::from_live(live)
        || proposal.identity != proposal_identity(proposal)?
        || !minimum_clearance_m.is_finite()
        || minimum_clearance_m < 0.0
        || super::super::airborne::live_rejection(
            context,
            live,
            proposal.absolute_deadline_physics_step,
        )
        .is_some()
    {
        bail!("airborne acquisition proposal binding or clearance is invalid");
    }
    validate_program(proposal)?;
    let bounds = flat_pad_bounds(
        context,
        &PadInputV2 {
            center_x_m: context.target_pad.center_x_m,
            surface_y_m: context.target_pad.surface_y_m,
            width_m: context.target_pad.width_m,
        },
    );
    let policy = ClearancePolicy {
        source_pad: FlatPadBounds {
            flat: false,
            ..bounds
        },
        target_pad: bounds,
        minimum_clearance_m,
    };
    let mut neutral = live.clone();
    let mut ordinary = live.clone();
    let mut scan = empty_clearance_scan();
    let mut parity = true;
    let mut index = 0;
    let mut contact = None;
    let mut incoming_contact = None;
    record_airborne_clearance(
        context,
        live,
        live.physics_step,
        &proposal.updates[0].phase,
        policy,
        &mut scan,
    );
    while neutral.physics_step < proposal.planned_end_physics_step && !ordinary.is_terminal() {
        if neutral.physics_step.is_multiple_of(HELD_TICKS) {
            let update = &proposal.updates[index];
            neutral.set_command(update.command);
            ordinary.set_command(update.command);
            index += 1;
        }
        let phase = &proposal.updates[index - 1].phase;
        let classification = neutral.step_physics_and_classify_contact(context);
        let report = ordinary.step_with_contact_report(context);
        if !finite_live_state(&neutral) || !finite_live_state(&ordinary) {
            bail!("airborne audit produced nonfinite state");
        }
        scan.poststep_state_count += 1;
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
        && neutral.physics_step == proposal.planned_end_physics_step
        && AirborneFlightStateV1::from_live(&neutral) == proposal.end_state;
    let safe = contact.as_ref().is_some_and(|c| {
        c.classification == "stable_touchdown_on_target"
            && c.core_matches_predicate_mirror
            && c.body_within_strict_terrain_domain
    }) && ordinary.physical_outcome == PhysicalOutcome::LandedOnTarget
        && ordinary.mission_outcome == MissionOutcome::Success
        && ordinary.end_reason == EndReason::TouchdownOnTarget
        && proposal.updates[index - 1].phase == "terminal_bridge";
    let mut reasons = Vec::new();
    if !safe {
        reasons.push("actual first contact is not a safe terminal target landing".into());
    }
    if !commands_match {
        reasons.push(
            "actual trajectory stops before or differs from selected nominal endpoint".into(),
        );
    }
    if !parity {
        bail!("airborne audit ordinary/neutral plant replay mismatch");
    }
    if !scan.all_airborne_states_passed {
        reasons.push("actual terrain violates declared airborne body clearance".into());
    }
    let audit = AirborneDirectAuditV1 {
        passed: reasons.is_empty(),
        safe_target_contact: safe,
        ordinary_neutral_parity: parity,
        commands_match,
        first_contact: contact,
        incoming_contact,
        clearance_scan: scan,
        final_state: SimulationStateSnapshotV1::from_state(&ordinary),
        rejection_reasons: reasons,
    };
    if audit
        .clearance_scan
        .first_violation
        .as_ref()
        .is_some_and(|v| {
            v.clearance_m.is_none_or(|c| !c.is_finite()) || !v.required_clearance_m.is_finite()
        })
    {
        bail!("airborne audit has unqueryable/nonfinite terrain evidence");
    }
    // Early contact may legitimately consume only a terrain-blocked prefix.
    // Without a genuine conflict, endpoint disagreement is an integrity error,
    // not a finite NominalRejected flight or permission to clear an obstacle.
    if !audit.commands_match && !crate::planner_flight::terrain::has_actual_terrain_conflict(&audit)
    {
        bail!("airborne audit endpoint mismatch without a genuine terrain conflict");
    }
    Ok(audit)
}
#[cfg(test)]
mod tests;
