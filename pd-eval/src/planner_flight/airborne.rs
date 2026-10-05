//! Current planner airborne; no research orchestration.
use super::*;
use anyhow::{Context, Result, bail};
use pd_core::{
    Command, EndReason, EvaluationGoal, FlightProgramUpdateV1, IncomingContactV1, MissionOutcome,
    PhysicalOutcome, RunContext, SimulationState, SimulationStateSnapshotV1, Vec2,
};
use serde::{Deserialize, Serialize};

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

pub(super) fn dynamics_identity(context: &RunContext) -> Result<String> {
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

pub(super) fn proposal_identity(proposal: &AirborneDirectProposalV1) -> Result<String> {
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
