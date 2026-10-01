//! Bounded coast plus terminal regeneration from an actual in-memory state.
//! Interior contact results and terrain-derived extrema never select a nominal
//! proposal. Real terrain is a separate audit, never a second ranking input.

use super::*;
use pd_core::{
    EndReason, EvaluationGoal, FlightProgramUpdateV1, IncomingContactV1, MissionOutcome,
    PhysicalOutcome, SimulationStateSnapshotV1,
};

pub const AIRBORNE_DIRECT_POLICY_ID: &str = "airborne_coast_terminal_regeneration_v1";
const COAST_FRACTIONS: [f64; 8] = [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875];
const TERMINAL_FACTORS: [f64; 7] = [1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];

/// Serializable evidence only: this type has no execution/restoration method.
/// Cumulative terrain-clearance metrics are deliberately not proposal inputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirborneFlightStateV1 {
    pub physics_step: u64,
    pub sim_time_s: f64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
    pub held_command: Command,
}

impl AirborneFlightStateV1 {
    pub fn from_live(state: &SimulationState) -> Self {
        Self {
            physics_step: state.physics_step,
            sim_time_s: state.sim_time_s,
            position_m: state.position_m,
            velocity_mps: state.velocity_mps,
            attitude_rad: state.attitude_rad,
            angular_rate_radps: state.angular_rate_radps,
            fuel_kg: state.fuel_kg,
            held_command: state.held_command,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirborneDirectProposalV1 {
    pub policy_id: String,
    pub dynamics_identity: String,
    pub absolute_deadline_physics_step: u64,
    pub incoming_state: AirborneFlightStateV1,
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
pub struct AirborneDirectAttemptV1 {
    pub row_index: usize,
    pub coast_fraction: f64,
    pub terminal_duration_factor: f64,
    pub coast_tick_count: u64,
    pub terminal_tick_count: u64,
    pub status: String,
    pub reason: Option<String>,
    pub proposal_identity: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirborneDirectSearchV1 {
    pub policy_id: String,
    pub absolute_deadline_physics_step: u64,
    pub incoming_state: AirborneFlightStateV1,
    pub attempts: Vec<AirborneDirectAttemptV1>,
    pub selected: Option<AirborneDirectProposalV1>,
    pub unsupported_reason: Option<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirborneDirectAuditV1 {
    pub passed: bool,
    pub safe_target_contact: bool,
    pub ordinary_neutral_parity: bool,
    pub commands_match: bool,
    pub first_contact: Option<TerminalContactAuditEvidence>,
    pub incoming_contact: Option<IncomingContactV1>,
    pub clearance_scan: GeometryClearanceScanEvidence,
    pub final_state: SimulationStateSnapshotV1,
    pub rejection_reasons: Vec<String>,
}

fn dynamics_identity(context: &RunContext) -> Result<String> {
    stable_digest(&(
        AIRBORNE_DIRECT_POLICY_ID,
        &context.sim,
        &context.vehicle,
        context.world.gravity_mps2,
        &context.target_pad,
        BodyAwareTerminalPolicyV1::default(),
        COAST_FRACTIONS,
        TERMINAL_FACTORS,
    ))
}

fn proposal_identity(proposal: &AirborneDirectProposalV1) -> Result<String> {
    let mut canonical = proposal.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

pub(crate) fn live_rejection(
    context: &RunContext,
    live: &SimulationState,
    deadline: u64,
) -> Option<String> {
    let finite = [
        live.sim_time_s,
        live.position_m.x,
        live.position_m.y,
        live.velocity_mps.x,
        live.velocity_mps.y,
        live.attitude_rad,
        live.angular_rate_radps,
        live.fuel_kg,
        live.held_command.throttle_frac,
        live.held_command.target_attitude_rad,
    ]
    .iter()
    .all(|x| x.is_finite());
    if !finite {
        return Some("nonfinite live dynamics or held command".into());
    }
    if context.sim.physics_hz != 120
        || context.sim.controller_hz != 60
        || !matches!(context.mission.goal, EvaluationGoal::LandingOnPad { .. })
        || context.mission.transfer_route.is_some()
    {
        return Some("only route-free landing with the original 120/60 clock is supported".into());
    }
    if live.physics_step == 0
        || !live.physics_step.is_multiple_of(2)
        || live.sim_time_s != live.physics_step as f64 / 120.0
        || live.physical_outcome != PhysicalOutcome::Flying
        || live.mission_outcome != MissionOutcome::InProgress
        || live.end_reason != EndReason::Running
    {
        return Some("live state is not a running airborne aligned global boundary".into());
    }
    if live.fuel_kg <= 0.0
        || live.fuel_kg > context.vehicle.initial_fuel_kg
        || live.held_command != live.held_command.clamped()
        || live.held_command.throttle_frac != 0.0
        || live.velocity_mps.x <= 0.0
        || live.position_m.x >= context.target_pad.center_x_m
        || live.angular_rate_radps.abs() > context.vehicle.max_rotation_rate_radps + 1.0e-12
    {
        return Some("bounded incoming family requires fuel and forward ballistic coast".into());
    }
    let horizon = (context.sim.max_time_s * 120.0).floor() as u64;
    if deadline <= live.physics_step || deadline > horizon {
        return Some(
            "absolute deadline is exhausted or beyond the original scenario horizon".into(),
        );
    }
    None
}

fn even_ticks(seconds: f64) -> u64 {
    // The supported input family is finite and below the original hard horizon.
    // Saturation here makes a huge crossing time a finite deadline rejection.
    ((seconds * 120.0 / 2.0).ceil() as u64)
        .saturating_mul(2)
        .max(2)
}

/// A target-only flat-pad predicate, not an interior terrain collision test.
/// None means no intended-pad contact; Some(false) is a bad target entry.
/// The actual core predicate and terrain clearance are checked separately.
pub(super) fn target_plane_contact(context: &RunContext, state: &SimulationState) -> Option<bool> {
    let aabb = body_aabb(state, &context.vehicle.geometry);
    let pad = &context.target_pad;
    if aabb.feet_x_min_m < pad.center_x_m - pad.half_width_m()
        || aabb.feet_x_max_m > pad.center_x_m + pad.half_width_m()
        || aabb.hull_x_min_m < pad.center_x_m - pad.half_width_m()
        || aabb.hull_x_max_m > pad.center_x_m + pad.half_width_m()
    {
        return None;
    }
    let points = actual_body_points(state, &context.vehicle.geometry);
    let min_feet = points[..2]
        .iter()
        .map(|p| p.y - pad.surface_y_m)
        .fold(f64::INFINITY, f64::min);
    let max_feet = points[..2]
        .iter()
        .map(|p| p.y - pad.surface_y_m)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_hull = points[2..]
        .iter()
        .map(|p| p.y - pad.surface_y_m)
        .fold(f64::INFINITY, f64::min);
    if min_feet > 0.0 && min_hull > 0.0 {
        return None;
    }
    let normal_speed = (-state.velocity_mps.y).max(0.0);
    let rate = state.angular_rate_radps.abs();
    let radius = (context.vehicle.geometry.hull_width_m * 0.5)
        .hypot(context.vehicle.geometry.hull_height_m * 0.5);
    let tolerance = 0.012_f64.max((normal_speed + rate * radius) / 120.0);
    Some(
        state.velocity_mps.y < 0.0
            && min_feet <= 0.05
            && max_feet <= 0.15
            && min_hull >= -tolerance
            && normal_speed <= context.vehicle.safe_touchdown_normal_speed_mps
            && state.velocity_mps.x.abs() <= context.vehicle.safe_touchdown_tangential_speed_mps
            && state.attitude_rad.cos().clamp(-1.0, 1.0).acos()
                <= context.vehicle.safe_touchdown_attitude_error_rad
            && rate <= context.vehicle.safe_touchdown_angular_rate_radps,
    )
}

pub(super) fn target_plane_clearance(context: &RunContext, state: &SimulationState) -> f64 {
    actual_body_points(state, &context.vehicle.geometry)
        .iter()
        .map(|point| point.y - context.target_pad.surface_y_m)
        .fold(f64::INFINITY, f64::min)
}

pub(super) fn candidate(
    context: &RunContext,
    live: &SimulationState,
    coast: u64,
    terminal: u64,
    deadline: u64,
) -> Result<AirborneDirectProposalV1> {
    let policy = BodyAwareTerminalPolicyV1::default();
    let limit = live
        .physics_step
        .checked_add(coast)
        .and_then(|n| n.checked_add(terminal))
        .and_then(|n| n.checked_add(policy.maximum_after_reference_ticks))
        .context("absolute candidate clock overflow")?;
    if limit > deadline {
        bail!("candidate exceeds original absolute mission deadline");
    }
    let predicted = predicted_coast_kinematics(context, live, coast);
    let reference = build_reference(context, &policy, predicted, terminal)?;
    let coast_attitude = paired_mean(&reference, context, 0);
    let coast_attitude = coast_attitude.x.atan2(coast_attitude.y);
    let mut state = live.clone();
    let mut updates = Vec::new();
    let mut peak = state.position_m.y;
    let mut descending = state.velocity_mps.y <= 0.0;
    for tick in 0..coast {
        if state.physics_step.is_multiple_of(2) {
            let command = Command {
                throttle_frac: 0.0,
                target_attitude_rad: coast_attitude,
            };
            updates.push(FlightProgramUpdateV1 {
                physics_step: state.physics_step,
                phase: "ballistic_coast".into(),
                command,
            });
            state.set_command(command);
        }
        // Neutral dynamics return contact, but neither that result nor
        // terrain-derived metrics affect the free-space candidate ledger.
        let _ = state.step_physics_and_classify_contact(context);
        descending |= state.velocity_mps.y <= 0.0;
        peak = peak.max(state.position_m.y);
        if target_plane_contact(context, &state).is_some() {
            bail!("intended pad reached before powered terminal at coast tick {tick}");
        }
    }
    for tick in 0..terminal + policy.maximum_after_reference_ticks {
        if state.physics_step.is_multiple_of(2) {
            let thrust = paired_mean(&reference, context, tick);
            let command = Command {
                throttle_frac: paired_throttle(
                    context,
                    &policy,
                    state.mass_kg(context),
                    thrust.length(),
                )?,
                target_attitude_rad: thrust.x.atan2(thrust.y),
            };
            updates.push(FlightProgramUpdateV1 {
                physics_step: state.physics_step,
                phase: "terminal_bridge".into(),
                command,
            });
            state.set_command(command);
        }
        let previous_target_clearance = target_plane_clearance(context, &state);
        let _ = state.step_physics_and_classify_contact(context);
        if descending && state.velocity_mps.y > 0.0 {
            bail!("nominal shape requires at most one future apex, not dive and recover");
        }
        descending |= state.velocity_mps.y <= 0.0;
        peak = peak.max(state.position_m.y);
        if state.fuel_kg <= 0.0 {
            bail!("candidate exhausts actual incoming fuel");
        }
        if let Some(safe) = target_plane_contact(context, &state) {
            if !safe || previous_target_clearance <= 0.0 {
                bail!("first intended-pad contact is unsafe or enters from below");
            }
            let mut proposal = AirborneDirectProposalV1 {
                policy_id: AIRBORNE_DIRECT_POLICY_ID.into(),
                dynamics_identity: dynamics_identity(context)?,
                absolute_deadline_physics_step: deadline,
                incoming_state: AirborneFlightStateV1::from_live(live),
                coast_tick_count: coast,
                terminal_tick_count: terminal,
                planned_end_physics_step: state.physics_step,
                updates,
                end_state: AirborneFlightStateV1::from_live(&state),
                peak_com_height_m: peak,
                identity: String::new(),
            };
            proposal.identity = proposal_identity(&proposal)?;
            return Ok(proposal);
        }
    }
    bail!("finite terminal window contains no safe intended-pad contact")
}

/// There is deliberately no source-program, suffix, row, or terrain-obstacle
/// seed argument. The caller must supply a fresh real in-memory live state.
pub fn evaluate_airborne_nominal_direct(
    context: &RunContext,
    live: &SimulationState,
    absolute_deadline_tick: u64,
) -> Result<AirborneDirectSearchV1> {
    context.sim.validate().map_err(anyhow::Error::msg)?;
    context.vehicle.validate().map_err(anyhow::Error::msg)?;
    context.target_pad.validate().map_err(anyhow::Error::msg)?;
    if !context.world.gravity_mps2.is_finite() || context.world.gravity_mps2 <= 0.0 {
        bail!("invalid gravity");
    }
    let mut search = AirborneDirectSearchV1 {
        policy_id: AIRBORNE_DIRECT_POLICY_ID.into(),
        absolute_deadline_physics_step: absolute_deadline_tick,
        incoming_state: AirborneFlightStateV1::from_live(live),
        attempts: Vec::new(),
        selected: None,
        unsupported_reason: live_rejection(context, live, absolute_deadline_tick),
        identity: String::new(),
    };
    if search.unsupported_reason.is_none() {
        let crossing_s = (context.target_pad.center_x_m - live.position_m.x) / live.velocity_mps.x;
        for coast_fraction in COAST_FRACTIONS {
            let coast = if coast_fraction == 0.0 {
                0
            } else {
                even_ticks(crossing_s * coast_fraction)
            };
            let remaining_s = (crossing_s - coast as f64 / 120.0).max(1.0 / 120.0);
            for factor in TERMINAL_FACTORS {
                let terminal = even_ticks(remaining_s * factor);
                let row_index = search.attempts.len();
                let result = candidate(context, live, coast, terminal, absolute_deadline_tick);
                let mut attempt = AirborneDirectAttemptV1 {
                    row_index,
                    coast_fraction,
                    terminal_duration_factor: factor,
                    coast_tick_count: coast,
                    terminal_tick_count: terminal,
                    status: "rejected".into(),
                    reason: None,
                    proposal_identity: None,
                };
                match result {
                    Ok(proposal) => {
                        attempt.status = "nominal_proposal".into();
                        attempt.proposal_identity = Some(proposal.identity.clone());
                        let earlier = search.selected.as_ref().is_none_or(|selected| {
                            proposal
                                .peak_com_height_m
                                .total_cmp(&selected.peak_com_height_m)
                                .then_with(|| {
                                    proposal
                                        .planned_end_physics_step
                                        .cmp(&selected.planned_end_physics_step)
                                })
                                .then_with(|| proposal.identity.cmp(&selected.identity))
                                .is_lt()
                        });
                        if earlier {
                            search.selected = Some(proposal);
                        }
                    }
                    Err(error) => attempt.reason = Some(format!("{error:#}")),
                }
                search.attempts.push(attempt);
            }
        }
    }
    search.identity = stable_digest(&search)?;
    Ok(search)
}

/// Audit exactly the selected proposal. A blocked/unsafe selection does not
/// cause selection of a higher trajectory or any reranking against terrain.
pub fn audit_airborne_direct_proposal(
    context: &RunContext,
    live: &SimulationState,
    proposal: &AirborneDirectProposalV1,
    minimum_clearance_m: f64,
) -> Result<AirborneDirectAuditV1> {
    if proposal.policy_id != AIRBORNE_DIRECT_POLICY_ID
        || proposal.dynamics_identity != dynamics_identity(context)?
        || proposal.incoming_state != AirborneFlightStateV1::from_live(live)
        || proposal.identity != proposal_identity(proposal)?
        || proposal.updates.is_empty()
        || proposal.planned_end_physics_step <= live.physics_step
        || !minimum_clearance_m.is_finite()
        || minimum_clearance_m < 0.0
        || proposal.planned_end_physics_step > proposal.absolute_deadline_physics_step
        || live_rejection(context, live, proposal.absolute_deadline_physics_step).is_some()
    {
        bail!("airborne proposal binding or clearance is invalid");
    }
    let end = proposal.planned_end_physics_step;
    let count = (end - live.physics_step).div_ceil(2);
    if proposal.updates.len() as u64 != count
        || proposal.updates.iter().enumerate().any(|(i, u)| {
            u.physics_step != live.physics_step + 2 * i as u64
                || u.command != u.command.clamped()
                || !u.command.throttle_frac.is_finite()
                || !u.command.target_attitude_rad.is_finite()
        })
    {
        bail!("airborne proposal does not cover every original global update exactly once");
    }
    let bounds = flat_pad_bounds(
        context,
        &PadInputV2 {
            center_x_m: context.target_pad.center_x_m,
            surface_y_m: context.target_pad.surface_y_m,
            width_m: context.target_pad.width_m,
        },
    );
    // An airborne suffix has no source-pad launch/contact exemption.
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
    let mut reasons = Vec::new();
    record_airborne_clearance(
        context,
        live,
        live.physics_step,
        "ballistic_coast",
        policy,
        &mut scan,
    );
    while neutral.physics_step < end && !ordinary.is_terminal() {
        if neutral.physics_step.is_multiple_of(2) {
            let update = &proposal.updates[index];
            neutral.set_command(update.command);
            ordinary.set_command(update.command);
            index += 1;
        }
        let classification = neutral.step_physics_and_classify_contact(context);
        let report = ordinary.step_with_contact_report(context);
        scan.poststep_state_count += 1;
        if classification == ContactClassification::None {
            parity &= report.incoming_contact.is_none()
                && SimulationStateSnapshotV1::from_state(&neutral)
                    == SimulationStateSnapshotV1::from_state(&ordinary);
            let phase = if neutral.physics_step <= live.physics_step + proposal.coast_tick_count {
                "ballistic_coast"
            } else {
                "terminal_bridge"
            };
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
        && neutral.physics_step == end
        && AirborneFlightStateV1::from_live(&neutral) == proposal.end_state;
    let safe = contact.as_ref().is_some_and(|c| {
        c.classification == "stable_touchdown_on_target"
            && c.core_matches_predicate_mirror
            && c.body_within_strict_terrain_domain
    }) && ordinary.physical_outcome == PhysicalOutcome::LandedOnTarget
        && ordinary.mission_outcome == MissionOutcome::Success
        && ordinary.end_reason == EndReason::TouchdownOnTarget;
    if !safe {
        reasons.push("actual first contact is not a safe target landing".into());
    }
    if !commands_match {
        reasons.push(
            "actual trajectory stops before or differs from selected nominal endpoint".into(),
        );
    }
    if !parity {
        reasons.push("ordinary/neutral plant replay mismatch".into());
    }
    if !scan.all_airborne_states_passed {
        reasons.push("actual terrain violates declared airborne body clearance".into());
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
mod tests {
    use super::*;

    fn context_and_live() -> (RunContext, SimulationState) {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let (_, request) = crate::load_nominal_direct_operational_fresh_inputs(repo)
            .expect("sealed physical inputs")
            .remove(0);
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let mut live = SimulationState::new(&context).unwrap();
        // Synthetic tests validate structure; acceptance uses real prefix states.
        live.physics_step = 120;
        live.sim_time_s = 1.0;
        live.position_m = Vec2::new(context.target_pad.center_x_m - 400.0, 200.0);
        live.velocity_mps = Vec2::new(25.0, 10.0);
        live.fuel_kg -= 10.0;
        (context, live)
    }

    #[test]
    fn airborne_preflight_rejects_reset_clock_terminal_and_powered_entry() {
        let (context, mut live) = context_and_live();
        assert!(live_rejection(&context, &live, 9600).is_none());
        live.sim_time_s = 0.0;
        assert!(live_rejection(&context, &live, 9600).is_some());
        live.sim_time_s = 1.0;
        live.physics_step = 121;
        assert!(live_rejection(&context, &live, 9600).is_some());
        live.physics_step = 120;
        live.end_reason = EndReason::Crash;
        assert!(live_rejection(&context, &live, 9600).is_some());
        live.end_reason = EndReason::Running;
        live.held_command.throttle_frac = 0.5;
        assert!(live_rejection(&context, &live, 9600).is_some());
    }

    #[test]
    fn airborne_nominal_ledger_is_finite_and_does_not_mutate_live_state() {
        let (context, live) = context_and_live();
        let before = SimulationStateSnapshotV1::from_state(&live);
        let search = evaluate_airborne_nominal_direct(&context, &live, 122).unwrap();
        assert!(search.unsupported_reason.is_none());
        assert_eq!(search.attempts.len(), 56);
        assert!(search.selected.is_none());
        assert!(search.attempts.iter().all(|a| a.reason.is_some()));
        assert_eq!(SimulationStateSnapshotV1::from_state(&live), before);
    }

    #[test]
    fn airborne_target_plane_does_not_act_as_an_interior_floor() {
        let (context, mut live) = context_and_live();
        live.position_m.y = -10.0;
        assert!(target_plane_contact(&context, &live).is_none());
        live.position_m.x = context.target_pad.center_x_m;
        live.velocity_mps.y = 1.0;
        assert_eq!(target_plane_contact(&context, &live), Some(false));
    }

    #[test]
    fn airborne_nominal_identity_excludes_interior_terrain_and_clearance_extrema() {
        let (mut context, mut live) = context_and_live();
        let baseline = evaluate_airborne_nominal_direct(&context, &live, 9600).unwrap();
        assert!(
            baseline.selected.is_some(),
            "nonvacuous terrain-twin comparison"
        );
        live.min_hull_clearance_m = -999.0;
        live.min_touchdown_clearance_m = -999.0;
        // Valid counterfactual terrain, retained target shelf.
        let mut points = context.world.terrain.points().to_vec();
        let x = (live.position_m.x + context.target_pad.center_x_m) * 0.5;
        points.retain(|p| p.x < x - 10.0 || p.x > x + 10.0);
        points.push(Vec2::new(x - 10.0, 0.0));
        points.push(Vec2::new(x, 1000.0));
        points.push(Vec2::new(x + 10.0, 0.0));
        points.sort_by(|a, b| a.x.total_cmp(&b.x));
        context.world.terrain =
            pd_core::terrain::TerrainDefinition::Heightfield { points_m: points };
        let twin = evaluate_airborne_nominal_direct(&context, &live, 9600).unwrap();
        assert_eq!(baseline, twin);
        assert_eq!(baseline.attempts.len(), 56);
    }

    #[test]
    fn airborne_audit_rejects_rehashed_command_gap_without_stepping() {
        let (context, live) = context_and_live();
        let mut proposal = evaluate_airborne_nominal_direct(&context, &live, 9600)
            .unwrap()
            .selected
            .expect("synthetic proposal for structural negative test");
        proposal.updates.remove(0);
        proposal.identity = proposal_identity(&proposal).unwrap();
        let before = SimulationStateSnapshotV1::from_state(&live);
        assert!(audit_airborne_direct_proposal(&context, &live, &proposal, 5.0).is_err());
        assert_eq!(SimulationStateSnapshotV1::from_state(&live), before);
    }

    #[test]
    fn airborne_synthetic_suffix_preserves_live_clock_fuel_and_one_apex_shape() {
        let (context, live) = context_and_live();
        let proposal = evaluate_airborne_nominal_direct(&context, &live, 9600)
            .unwrap()
            .selected
            .expect("synthetic shape/control test");
        let mut state = live.clone();
        let mut descending = false;
        let mut index = 0;
        while state.physics_step < proposal.planned_end_physics_step {
            if state.physics_step.is_multiple_of(2) {
                let update = &proposal.updates[index];
                assert_eq!(update.physics_step, state.physics_step);
                state.set_command(update.command);
                index += 1;
            }
            let previous_fuel = state.fuel_kg;
            let previous_tick = state.physics_step;
            let _ = state.step_physics_and_classify_contact(&context);
            assert_eq!(state.physics_step, previous_tick + 1);
            assert_eq!(state.sim_time_s, state.physics_step as f64 / 120.0);
            assert!(state.fuel_kg <= previous_fuel);
            assert!(!descending || state.velocity_mps.y <= 0.0);
            descending |= state.velocity_mps.y <= 0.0;
        }
        assert_eq!(AirborneFlightStateV1::from_live(&state), proposal.end_state);
        assert_eq!(index, proposal.updates.len());
        let audit = audit_airborne_direct_proposal(&context, &live, &proposal, 0.0).unwrap();
        assert!(audit.ordinary_neutral_parity);
        assert!(audit.commands_match);
        assert!(audit.safe_target_contact);
    }
}
