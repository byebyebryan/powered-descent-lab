//! Opt-in state-based ballistic correction candidate. Never selected by policy 3.
//! Owns the live plant, goals and executed decisions. Prefixes are reconstructed
//! from original commands; serialized snapshots are comparison evidence only.

use std::{fs, path::Path};

use anyhow::{Context, Result, ensure};
use pd_control::{Controller, TerminalPdgController};
use pd_core::{
    ActionLogEntry, Command, EventKind, EventRecord, FlightProgramUpdateV1, MissionOutcome,
    PhysicalOutcome, RunContext, SampleRecord, ScenarioSpec, SimulationState,
    SimulationStateSnapshotV1, TerrainQueryError, Vec2,
};
use pd_plan::ballistic::{
    KinematicStateV2,
    aim::{self, BallisticAim},
};
use pd_plan::local_clearing::{
    HandoffBrakingRoomEstimate, conservative_body_diameter, handoff_braking_room,
};
use serde::{Deserialize, Serialize};

use crate::{
    BodyAwareTerminalPolicyV1, WaypointDirectNominalDirectGenerationPolicyV1,
    WaypointDirectNominalDirectGenerationRequest, WaypointV2FlightResult,
    evidence_io::{reserve_output_root, sha256_bytes, write_json_create_only},
    local_clearing::{OrdinaryLive, advance_ordinary, new_ordinary, snapshot_finite},
    nominal_body_reserve_query, paired_throttle,
};

mod avoidance;
mod clearance;
mod coast_terminal;
mod finite_correction;
mod phase_transition;
mod report;
mod ridge;
pub mod terminal_coast_diagnostic;

pub const CANDIDATE_ID: &str = "ballistic_feedback_v5_ridge_waypoint";

/// Explicit ablations of waypoint acquisition. Ordinary planner policy 3 and
/// the existing ridge candidate default remain unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum WaypointExperiment {
    #[default]
    Ridge,
    Effort,
    Recovery,
    Combined,
    LocalHeight,
    EarlyTarget,
    LocalHeightEarlyTarget,
    LandingDuration,
    EarlyTargetLandingDuration,
    LandingCountdown,
    EarlyTargetLandingCountdown,
    CoastTerminal,
    LandingBrakingGuard,
    LandingBodyCentering,
    TerminalCoordination,
    ExitConsistency,
    PiecewiseEarlyTarget,
    RecoveryLead,
    MechanicsCombined,
    FiniteCorrectionProbe,
    FiniteCorrection,
    TransitionProbe,
    CoastTransition,
    TerminalTakeover,
    PadClearance,
    PhaseTransitions,
    RecoveryConsistencyProbe,
    RecoveryConsistency,
}

impl WaypointExperiment {
    pub fn candidate_id(self) -> &'static str {
        match self {
            Self::Ridge => CANDIDATE_ID,
            Self::Effort => "ballistic_feedback_v6_waypoint_effort",
            Self::Recovery => "ballistic_feedback_v6_waypoint_recovery",
            Self::Combined => "ballistic_feedback_v6_waypoint_combined",
            Self::LocalHeight => "ballistic_feedback_v7_local_height",
            Self::EarlyTarget => "ballistic_feedback_v7_early_target",
            Self::LocalHeightEarlyTarget => "ballistic_feedback_v7_local_height_early_target",
            Self::LandingDuration => "ballistic_feedback_v8_landing_duration",
            Self::EarlyTargetLandingDuration => {
                "ballistic_feedback_v8_early_target_landing_duration"
            }
            Self::LandingCountdown => "ballistic_feedback_v9_landing_countdown",
            Self::EarlyTargetLandingCountdown => {
                "ballistic_feedback_v9_early_target_landing_countdown"
            }
            Self::CoastTerminal => "ballistic_feedback_v10_coast_terminal",
            Self::LandingBrakingGuard => "ballistic_feedback_v11_landing_braking_guard",
            Self::LandingBodyCentering => "ballistic_feedback_v12_landing_body_centering",
            Self::TerminalCoordination => "ballistic_feedback_v13_terminal_coordination",
            Self::ExitConsistency => "ballistic_feedback_v14_exit_consistency",
            Self::PiecewiseEarlyTarget => "ballistic_feedback_v14_piecewise_early_target",
            Self::RecoveryLead => "ballistic_feedback_v14_recovery_lead",
            Self::MechanicsCombined => "ballistic_feedback_v14_mechanics_combined",
            Self::FiniteCorrectionProbe => "ballistic_feedback_v15_finite_correction_probe",
            Self::FiniteCorrection => "ballistic_feedback_v15_finite_correction",
            Self::TransitionProbe => "ballistic_feedback_v16_transition_probe",
            Self::CoastTransition => "ballistic_feedback_v16_coast_transition",
            Self::TerminalTakeover => "ballistic_feedback_v16_terminal_takeover",
            Self::PadClearance => "ballistic_feedback_v16_pad_clearance",
            Self::PhaseTransitions => "ballistic_feedback_v16_phase_transitions",
            Self::RecoveryConsistencyProbe => "ballistic_feedback_v17_recovery_consistency_probe",
            Self::RecoveryConsistency => "ballistic_feedback_v17_recovery_consistency",
        }
    }

    fn mechanics(self) -> bool {
        self.phase_queries()
            || matches!(
                self,
                Self::ExitConsistency
                    | Self::PiecewiseEarlyTarget
                    | Self::RecoveryLead
                    | Self::MechanicsCombined
                    | Self::FiniteCorrectionProbe
                    | Self::FiniteCorrection
            )
    }

    fn exit_consistency(self) -> bool {
        self.phase_queries()
            || matches!(
                self,
                Self::ExitConsistency
                    | Self::MechanicsCombined
                    | Self::FiniteCorrectionProbe
                    | Self::FiniteCorrection
            )
    }

    fn finite_queries(self) -> bool {
        self.phase_queries() || matches!(self, Self::FiniteCorrectionProbe | Self::FiniteCorrection)
    }

    fn finite_correction(self) -> bool {
        self.phase_queries() || self == Self::FiniteCorrection
    }

    fn phase_queries(self) -> bool {
        matches!(
            self,
            Self::TransitionProbe
                | Self::CoastTransition
                | Self::TerminalTakeover
                | Self::PadClearance
                | Self::PhaseTransitions
                | Self::RecoveryConsistencyProbe
                | Self::RecoveryConsistency
        )
    }

    fn coast_transition(self) -> bool {
        matches!(self, Self::CoastTransition | Self::PhaseTransitions)
            || self.recovery_consistency()
    }

    fn terminal_takeover(self) -> bool {
        matches!(self, Self::TerminalTakeover | Self::PhaseTransitions)
            || self.recovery_consistency()
    }

    fn pad_clearance(self) -> bool {
        matches!(self, Self::PadClearance | Self::PhaseTransitions) || self.recovery_consistency()
    }

    fn recovery_consistency(self) -> bool {
        matches!(
            self,
            Self::RecoveryConsistencyProbe | Self::RecoveryConsistency
        )
    }

    fn queued_recovery(self) -> bool {
        self == Self::RecoveryConsistency
    }

    fn piecewise_early_target(self) -> bool {
        matches!(self, Self::PiecewiseEarlyTarget | Self::MechanicsCombined)
    }

    fn recovery_lead(self) -> bool {
        matches!(self, Self::RecoveryLead | Self::MechanicsCombined)
    }

    fn terminal_coordination(self) -> bool {
        self == Self::TerminalCoordination || self.mechanics()
    }

    fn effort(self) -> bool {
        matches!(self, Self::Effort | Self::Combined) || self.local_height() || self.early_target()
    }

    fn recovery(self) -> bool {
        matches!(self, Self::Recovery | Self::Combined)
            || self.local_height()
            || self.early_target()
    }

    fn local_height(self) -> bool {
        self.mechanics()
            || matches!(
                self,
                Self::LocalHeight
                    | Self::LocalHeightEarlyTarget
                    | Self::LandingDuration
                    | Self::LandingCountdown
                    | Self::CoastTerminal
                    | Self::LandingBrakingGuard
                    | Self::LandingBodyCentering
                    | Self::TerminalCoordination
            )
    }

    fn early_target(self) -> bool {
        self.mechanics()
            || matches!(
                self,
                Self::EarlyTarget
                    | Self::LocalHeightEarlyTarget
                    | Self::LandingDuration
                    | Self::EarlyTargetLandingDuration
                    | Self::LandingCountdown
                    | Self::EarlyTargetLandingCountdown
                    | Self::CoastTerminal
                    | Self::LandingBrakingGuard
                    | Self::LandingBodyCentering
                    | Self::TerminalCoordination
            )
    }

    fn landing_duration(self) -> bool {
        self.mechanics()
            || matches!(
                self,
                Self::LandingDuration
                    | Self::EarlyTargetLandingDuration
                    | Self::LandingCountdown
                    | Self::EarlyTargetLandingCountdown
                    | Self::CoastTerminal
                    | Self::LandingBrakingGuard
                    | Self::LandingBodyCentering
                    | Self::TerminalCoordination
            )
    }

    fn landing_countdown(self) -> bool {
        self.mechanics()
            || matches!(
                self,
                Self::LandingCountdown
                    | Self::EarlyTargetLandingCountdown
                    | Self::CoastTerminal
                    | Self::LandingBrakingGuard
                    | Self::LandingBodyCentering
                    | Self::TerminalCoordination
            )
    }

    fn coast_terminal(self) -> bool {
        self.mechanics()
            || matches!(
                self,
                Self::CoastTerminal
                    | Self::LandingBrakingGuard
                    | Self::LandingBodyCentering
                    | Self::TerminalCoordination
            )
    }

    fn landing_braking_guard(self) -> bool {
        self.mechanics()
            || matches!(
                self,
                Self::LandingBrakingGuard | Self::LandingBodyCentering | Self::TerminalCoordination
            )
    }

    fn landing_body_centering(self) -> bool {
        self.mechanics()
            || matches!(
                self,
                Self::LandingBodyCentering | Self::TerminalCoordination
            )
    }
}
const REFRESH_TICKS: u64 = 24;
const MIN_ANGLE: f64 = std::f64::consts::FRAC_PI_4;
const THRUST_DERATE: f64 = 0.925;
const CONTINUATION_TICKS: u64 = 240;
const MAX_PROFILE_TRIALS: u64 = 32;
const MAX_LOCAL_PROPOSALS: usize = 4;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    pub position_m: Vec2,
    pub destination: bool,
    pub number: usize,
    #[serde(default)]
    pub revision: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PredictedConflict {
    pub state: SimulationStateSnapshotV1,
    pub cause: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Correction {
    pub arrival_physics_step: u64,
    pub turn_end_physics_step: u64,
    pub burn_end_physics_step: u64,
    pub thrust_acceleration_mps2: Vec2,
    pub target_attitude_rad: f64,
    pub predicted_cutoff: KinematicStateV2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Refresh {
    pub origin: SimulationStateSnapshotV1,
    pub goal: Goal,
    pub desired_arc: Option<BallisticAim>,
    pub correction: Option<Correction>,
    pub decision: String,
    pub current_ballistic_miss_m: Option<f64>,
    pub terrain_conflict_m: Option<Vec2>,
    #[serde(default)]
    pub predicted_conflict: Option<PredictedConflict>,
    #[serde(default)]
    pub predicted_command: Option<Command>,
    #[serde(default)]
    pub previous_goal: Option<Goal>,
    #[serde(default)]
    pub replan_trigger: Option<PredictedConflict>,
    #[serde(default)]
    pub terrain_correction: Option<avoidance::TerrainCorrection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_query: Option<avoidance::RecoveryQuery>,
    #[serde(default)]
    pub waypoint_height_repair: Option<clearance::WaypointHeightRepair>,
    #[serde(default)]
    pub ridge_selection: Option<ridge::RidgeSelection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waypoint_braking_room: Option<HandoffBrakingRoomEstimate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coast_terminal: Option<coast_terminal::Preview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coast_terminal_rejection: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finite_destination_query: Option<finite_correction::Query>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_query: Option<phase_transition::Query>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_common_query: Option<phase_transition::RecoveryComparison>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedbackResult {
    pub candidate_id: String,
    pub scenario_sha256: String,
    pub prefix_flight_sha256: Option<String>,
    pub prefix_origin: Option<SimulationStateSnapshotV1>,
    pub terrain_neutral_diagnostic: bool,
    pub stop: String,
    pub reason: String,
    pub refreshes: Vec<Refresh>,
    pub handoffs: Vec<SimulationStateSnapshotV1>,
    #[serde(default)]
    pub handoff_goal_revisions: Vec<usize>,
    pub updates: Vec<FlightProgramUpdateV1>,
    pub final_state: SimulationStateSnapshotV1,
    pub integrity_passed: bool,
    pub source_replay_passed: bool,
    pub decisions_reproduced: bool,
    pub ordinary_flight: crate::LocalClearingOrdinaryEvidenceV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain_domain_stop: Option<TerrainDomainStop>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controller_updates: Vec<pd_control::ControllerUpdateRecord>,
}

/// Query failure evidence, never an invented physical outcome. The executed
/// prefix and command replay remain separate from this unexecuted prediction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainDomainStop {
    pub query: String,
    pub actual_state: SimulationStateSnapshotV1,
    pub query_origin: SimulationStateSnapshotV1,
    pub query_state: SimulationStateSnapshotV1,
    pub requested_command: Option<Command>,
    pub error: TerrainQueryError,
}

impl std::fmt::Display for TerrainDomainStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exact body-envelope terrain query failed: {}",
            self.error
        )
    }
}

impl std::error::Error for TerrainDomainStop {}

fn kinematics(s: &SimulationState) -> KinematicStateV2 {
    KinematicStateV2 {
        position_m: s.position_m,
        velocity_mps: s.velocity_mps,
    }
}

fn target(ctx: &RunContext) -> Goal {
    Goal {
        position_m: Vec2::new(
            ctx.target_pad.center_x_m,
            ctx.target_pad.surface_y_m + ctx.vehicle.geometry.touchdown_base_offset_m,
        ),
        destination: true,
        number: 0,
        revision: 0,
    }
}

fn tolerance(ctx: &RunContext, goal: &Goal) -> f64 {
    if goal.destination {
        ((ctx.target_pad.width_m * 0.5
            - ctx
                .vehicle
                .geometry
                .touchdown_half_span_m
                .max(ctx.vehicle.geometry.hull_width_m * 0.5))
            * 0.5)
            .max(0.0)
    } else {
        conservative_body_diameter(&ctx.vehicle.geometry) * 0.5
    }
}

fn approach_ok(ctx: &RunContext, state: KinematicStateV2, goal: &Goal, steps: u64) -> bool {
    if !goal.destination {
        return true;
    }
    let dt = ctx.sim.physics_dt_s();
    let g = ctx.world.gravity_mps2;
    let end = aim::project(state, g, dt, steps);
    if end.velocity_mps.y >= 0.0
        || end.velocity_mps.x <= 0.0
        || (-end.velocity_mps.y).atan2(end.velocity_mps.x.abs()) + 1e-10 < MIN_ANGLE
    {
        return false;
    }
    // First descending state: current state if descending, otherwise the apex.
    let apex_ticks = (state.velocity_mps.y / (g * dt)).ceil().max(0.0) as u64;
    let entry = aim::project(state, g, dt, apex_ticks.min(steps));
    let available = THRUST_DERATE * ctx.vehicle.max_thrust_n
        / (ctx.vehicle.dry_mass_kg + ctx.vehicle.initial_fuel_kg);
    let room = handoff_braking_room(
        (goal.position_m.x - entry.position_m.x).max(0.0),
        entry.velocity_mps.x.max(0.0),
        0.0,
        available,
        g,
        ctx.vehicle.max_rotation_rate_radps,
        2.0 * dt,
    );
    room.is_some_and(|r| r.remaining_room_m >= 0.0)
        && available > g
        && entry.velocity_mps.y.min(0.0).powi(2) / (2.0 * (available - g))
            <= entry.position_m.y - goal.position_m.y
}

fn accepted_coast(ctx: &RunContext, s: &SimulationState, goal: &Goal) -> Option<(u64, f64)> {
    if !goal.destination {
        // A pass-through goal is a forward clearance region, not a touchdown
        // height crossing. Rising and already-higher motion are valid too.
        if s.velocity_mps.x <= 0.0 || goal.position_m.x <= s.position_m.x {
            return None;
        }
        let n = (((goal.position_m.x - s.position_m.x)
            / (s.velocity_mps.x * 2.0 * ctx.sim.physics_dt_s()))
        .ceil() as u64
            * 2)
        .max(2);
        let end = aim::project(
            kinematics(s),
            ctx.world.gravity_mps2,
            ctx.sim.physics_dt_s(),
            n,
        );
        return (end.position_m.y >= goal.position_m.y - tolerance(ctx, goal))
            .then_some((n, goal.position_m.x - end.position_m.x));
    }
    let n = aim::natural_arrival_steps(
        kinematics(s),
        goal.position_m.y,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
    )?;
    let end = aim::project(
        kinematics(s),
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        n,
    );
    let miss = goal.position_m.x - end.position_m.x;
    (miss.abs() <= tolerance(ctx, goal) && approach_ok(ctx, kinematics(s), goal, n))
        .then_some((n, miss))
}

#[cfg(test)]
fn construct(
    ctx: &RunContext,
    s: &SimulationState,
    goal: &Goal,
    deadline: u64,
) -> Option<(BallisticAim, Correction)> {
    construct_for(ctx, s, goal, deadline, WaypointExperiment::Ridge)
}

fn profile_steps(
    ctx: &RunContext,
    s: &SimulationState,
    goal: &Goal,
    experiment: WaypointExperiment,
) -> Option<Vec<u64>> {
    let dt = ctx.sim.physics_dt_s();
    let g = ctx.world.gravity_mps2;
    let current = kinematics(s);
    let dx = goal.position_m.x - current.position_m.x;
    if dx <= 0.0 || s.fuel_kg <= 0.0 {
        return None;
    }
    let dy = goal.position_m.y - current.position_m.y;
    let min_t = if goal.destination {
        (dt + (dt * dt + 8.0 * (dy + dx * MIN_ANGLE.tan()).max(0.0) / g).sqrt()) * 0.5
    } else {
        (2.0 * dx / g).sqrt()
    };
    let geometric = ((min_t / (2.0 * dt)).ceil() as u64 * 2).max(2);
    let recovering = !goal.destination && experiment.recovery();
    let base = if recovering {
        geometric
    } else {
        geometric.max(aim::natural_arrival_steps(current, goal.position_m.y, g, dt).unwrap_or(0))
    };
    let mut steps = Vec::new();
    if recovering && current.velocity_mps.x > 0.0 {
        let pairs = (dx / (current.velocity_mps.x * 2.0 * dt)).ceil();
        if pairs.is_finite() && pairs >= 1.0 && pairs < (u64::MAX / 2) as f64 {
            steps.push(pairs as u64 * 2);
        }
    }
    for trial in 0..MAX_PROFILE_TRIALS {
        let n = base.checked_add(trial * 60)?;
        if !steps.contains(&n) {
            steps.push(n);
        }
        if steps.len() == MAX_PROFILE_TRIALS as usize {
            break;
        }
    }
    Some(steps)
}

fn correction_rank(ctx: &RunContext, origin_tick: u64, c: &Correction) -> (f64, f64, u64) {
    let burn = c.burn_end_physics_step - c.turn_end_physics_step.max(origin_tick);
    let effort = c.thrust_acceleration_mps2.length() * burn as f64 * ctx.sim.physics_dt_s();
    let entry = aim::project(
        c.predicted_cutoff,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        c.arrival_physics_step - c.burn_end_physics_step,
    );
    (effort, entry.velocity_mps.length(), c.arrival_physics_step)
}

fn construct_for(
    ctx: &RunContext,
    s: &SimulationState,
    goal: &Goal,
    deadline: u64,
    experiment: WaypointExperiment,
) -> Option<(BallisticAim, Correction)> {
    let dt = ctx.sim.physics_dt_s();
    let g = ctx.world.gravity_mps2;
    let current = kinematics(s);
    let available = THRUST_DERATE * ctx.vehicle.max_thrust_n / s.mass_kg(ctx);
    let prefer_effort = !goal.destination && experiment.effort();
    let mut best: Option<(BallisticAim, Correction)> = None;
    for n in profile_steps(ctx, s, goal, experiment)? {
        if s.physics_step.checked_add(n)? > deadline {
            continue;
        }
        let arc = aim::target_arc(current, goal.position_m, g, dt, n)?;
        let miss = goal.position_m - aim::project(current, g, dt, n).position_m;
        let mut turn = 0;
        let mut solution = None;
        for _ in 0..3 {
            let Some(remaining_ticks) = n.checked_sub(turn) else {
                // This profile cannot accommodate the estimated slew. Try
                // the next existing duration, without wrapping its clock.
                break;
            };
            let remaining = remaining_ticks as f64 * dt + dt * 0.5;
            let disc = remaining * remaining - 2.0 * miss.length() / available;
            if disc < 0.0 {
                break;
            }
            let seconds = remaining - disc.sqrt();
            let burn = ((seconds / (2.0 * dt)).ceil() as u64 * 2).max(2);
            if turn + burn + REFRESH_TICKS >= n {
                break;
            }
            let accel =
                aim::correction_acceleration(current, goal.position_m, g, dt, n, turn, burn)?;
            if accel.y < -1e-8 || accel.length() > available + 1e-8 {
                break;
            }
            let (next_turn, attitude) =
                crate::planner_flight::turn_ticks_for(ctx, s.attitude_rad, accel).ok()?;
            if next_turn != turn {
                turn = next_turn;
                continue;
            }
            let end = aim::correction_end(current, g, dt, turn, burn, accel);
            if end.velocity_mps.x <= 0.0 || !approach_ok(ctx, end, goal, n - turn - burn) {
                break;
            }
            if paired_throttle(
                ctx,
                &BodyAwareTerminalPolicyV1::default(),
                s.mass_kg(ctx),
                accel.length(),
            )
            .is_err()
            {
                break;
            }
            solution = Some(Correction {
                arrival_physics_step: s.physics_step + n,
                turn_end_physics_step: s.physics_step + turn,
                burn_end_physics_step: s.physics_step + turn + burn,
                thrust_acceleration_mps2: accel,
                target_attitude_rad: attitude,
                predicted_cutoff: end,
            });
            break;
        }
        if let Some(correction) = solution {
            if !prefer_effort {
                return Some((arc, correction));
            }
            let rank = correction_rank(ctx, s.physics_step, &correction);
            if best.as_ref().is_none_or(|(_, old)| {
                let old_rank = correction_rank(ctx, s.physics_step, old);
                rank.0
                    .total_cmp(&old_rank.0)
                    .then_with(|| rank.1.total_cmp(&old_rank.1))
                    .then_with(|| rank.2.cmp(&old_rank.2))
                    .is_lt()
            }) {
                best = Some((arc, correction));
            }
        }
    }
    best
}

/// Refresh the remaining finite burn without moving its absolute clocks.
fn retained_correction(
    ctx: &RunContext,
    s: &SimulationState,
    goal: &Goal,
    old: &Correction,
) -> Option<(BallisticAim, Correction)> {
    let n = old.arrival_physics_step.checked_sub(s.physics_step)?;
    let turn = old.turn_end_physics_step.saturating_sub(s.physics_step);
    let burn = old
        .burn_end_physics_step
        .checked_sub(s.physics_step + turn)?;
    if burn == 0 {
        return None;
    }
    let accel = aim::correction_acceleration(
        kinematics(s),
        goal.position_m,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        n,
        turn,
        burn,
    )?;
    let (needed_turn, _) =
        crate::planner_flight::turn_ticks_for(ctx, s.attitude_rad, accel).ok()?;
    if accel.y < -1e-8
        || accel.length() > THRUST_DERATE * ctx.vehicle.max_thrust_n / s.mass_kg(ctx)
        || needed_turn > turn + 2
    {
        return None;
    }
    let end = aim::correction_end(
        kinematics(s),
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        turn,
        burn,
        accel,
    );
    if !approach_ok(ctx, end, goal, n - turn - burn) {
        return None;
    }
    Some((
        aim::target_arc(
            kinematics(s),
            goal.position_m,
            ctx.world.gravity_mps2,
            ctx.sim.physics_dt_s(),
            n,
        )?,
        Correction {
            thrust_acceleration_mps2: accel,
            predicted_cutoff: end,
            ..old.clone()
        },
    ))
}

/// Read-only geometric coast from the refreshed cutoff estimate. This does not
/// certify the powered acquisition; ordinary command/body checks still own it.
fn pending_coast_clear(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    c: &Correction,
) -> Result<bool> {
    let n = c
        .arrival_physics_step
        .checked_sub(c.burn_end_physics_step)
        .context("pending coast clock inversion")?;
    for tick in 0..=n + CONTINUATION_TICKS {
        let state = aim::project(
            c.predicted_cutoff,
            ctx.world.gravity_mps2,
            ctx.sim.physics_dt_s(),
            tick,
        );
        let mut view = s.clone();
        view.position_m = state.position_m;
        view.velocity_mps = state.velocity_mps;
        view.attitude_rad = 0.0;
        view.physics_step = c.burn_end_physics_step + tick;
        if reserve_conflict(request, ctx, &view, false, "pending_cutoff_coast_reserve")?.is_some() {
            return Ok(false);
        }
    }
    Ok(true)
}

struct DestinationPreview {
    goal: Goal,
    arc: BallisticAim,
    correction: Option<Correction>,
    waypoint_room: HandoffBrakingRoomEstimate,
    obstruction: Option<PredictedConflict>,
}

/// Prefer reacquiring the destination before a fast, late waypoint entry. This
/// is a pure preview, not a handoff or a new powered-recovery maneuver family.
fn early_destination_preview(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    coast_steps: u64,
    deadline: u64,
    piecewise: bool,
) -> Result<Option<DestinationPreview>> {
    let entry = aim::project(
        kinematics(s),
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        coast_steps,
    );
    let Some(room) = handoff_braking_room(
        (ctx.target_pad.center_x_m - entry.position_m.x).max(0.0),
        entry.velocity_mps.x.max(0.0),
        0.0,
        THRUST_DERATE * ctx.vehicle.max_thrust_n / s.mass_kg(ctx),
        ctx.world.gravity_mps2,
        ctx.vehicle.max_rotation_rate_radps,
        2.0 * ctx.sim.physics_dt_s(),
    ) else {
        return Ok(None);
    };
    if room.remaining_room_m >= 0.0 {
        return Ok(None);
    }
    let goal = target(ctx);
    let (arc, correction) = if let Some((steps, _)) = accepted_coast(ctx, s, &goal) {
        let arc = aim::target_arc(
            kinematics(s),
            goal.position_m,
            ctx.world.gravity_mps2,
            ctx.sim.physics_dt_s(),
            steps,
        )
        .context("invalid accepted destination preview")?;
        (arc, None)
    } else if let Some((arc, correction)) =
        construct_for(ctx, s, &goal, deadline, WaypointExperiment::Combined)
    {
        (arc, Some(correction))
    } else {
        return Ok(None);
    };
    let obstruction = first_conflict(request, ctx, s, &arc)?;
    if obstruction.is_some() && !piecewise {
        return Ok(None);
    }
    if obstruction.is_none()
        && let Some(c) = &correction
    {
        let mut cutoff = s.clone();
        cutoff.position_m = c.predicted_cutoff.position_m;
        cutoff.velocity_mps = c.predicted_cutoff.velocity_mps;
        cutoff.attitude_rad = 0.0;
        cutoff.physics_step = c.burn_end_physics_step;
        cutoff.sim_time_s = cutoff.physics_step as f64 * ctx.sim.physics_dt_s();
        let arc = aim::target_arc(
            kinematics(&cutoff),
            goal.position_m,
            ctx.world.gravity_mps2,
            ctx.sim.physics_dt_s(),
            c.arrival_physics_step - c.burn_end_physics_step,
        )
        .context("invalid cutoff destination preview")?;
        if first_conflict(request, ctx, &cutoff, &arc)?.is_some() {
            return Ok(None);
        }
    }
    let command = correction
        .as_ref()
        .map(|c| correction_command(ctx, s, c))
        .transpose()?
        .unwrap_or_default();
    if obstruction.is_none() && short_conflict(request, ctx, s, command, false)?.is_some() {
        return Ok(None);
    }
    Ok(Some(DestinationPreview {
        goal,
        arc,
        correction,
        waypoint_room: room,
        obstruction,
    }))
}

fn body_clearance(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    landing: bool,
) -> Result<(f64, f64, f64)> {
    let source = ctx
        .world
        .landing_pad(&request.source_pad_id)
        .context("source pad missing")?;
    let source_transition = s.position_m.x >= source.center_x_m - source.width_m * 0.5
        && s.position_m.x <= source.center_x_m + source.width_m * 0.5;
    let phase = if landing {
        "terminal_bridge"
    } else if source_transition {
        "source_bridge"
    } else {
        "ballistic_coast"
    };
    let (clearance, required) =
        nominal_body_reserve_query(ctx, request, s, phase, source_transition)?;
    let floor = if s.physics_step == 0 {
        required - 1e-8
    } else {
        required
    };
    Ok((clearance, required, floor))
}

fn body_safe(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    landing: bool,
) -> Result<()> {
    let (clearance, required, floor) = body_clearance(request, ctx, s, landing)?;
    ensure!(
        clearance >= floor,
        "body reserve {clearance} below {required} at tick {}",
        s.physics_step
    );
    Ok(())
}

/// Geometric query only: desired velocities are never assigned to the live plant.
fn first_conflict(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    arc: &BallisticAim,
) -> Result<Option<PredictedConflict>> {
    let initial = KinematicStateV2 {
        position_m: s.position_m,
        velocity_mps: arc.departure_velocity_mps,
    };
    let touchdown_y = ctx.target_pad.surface_y_m + ctx.vehicle.geometry.touchdown_base_offset_m;
    let terminal_query = (arc.target_m.x - ctx.target_pad.center_x_m).abs()
        <= tolerance(ctx, &target(ctx))
        && arc.target_m.y <= touchdown_y;
    for k in 1..=arc.steps {
        let predicted = aim::project(initial, ctx.world.gravity_mps2, ctx.sim.physics_dt_s(), k);
        let mut view = s.clone();
        view.position_m = predicted.position_m;
        if k == arc.steps {
            // The ideal arc ends at this exact geometric target by definition.
            // Do not turn cancellation roundoff into a fictitious obstruction.
            // Actual plant/contact and short-command checks remain untouched.
            view.position_m = arc.target_m;
        }
        view.velocity_mps = predicted.velocity_mps;
        view.attitude_rad = 0.0;
        if terminal_query && predicted.velocity_mps.y < 0.0 && predicted.position_m.y <= touchdown_y
        {
            // A paired-tick height projection can finish a fraction of a tick
            // below the shelf. Query only up to its first touchdown-height
            // crossing, not an invented continuation through the ground.
            // Actual landing and held-command contact checks are unchanged.
            let previous = aim::project(
                initial,
                ctx.world.gravity_mps2,
                ctx.sim.physics_dt_s(),
                k - 1,
            );
            if previous.position_m.y >= touchdown_y {
                let fraction = (previous.position_m.y - touchdown_y)
                    / (previous.position_m.y - predicted.position_m.y);
                view.position_m = Vec2::new(
                    previous.position_m.x
                        + fraction * (predicted.position_m.x - previous.position_m.x),
                    touchdown_y,
                );
                let mut conflict =
                    reserve_conflict(request, ctx, &view, true, "ideal_arc_reserve")?;
                if let Some(c) = &mut conflict {
                    c.state.physics_step += k;
                    c.state.sim_time_s += k as f64 * ctx.sim.physics_dt_s();
                }
                return Ok(conflict);
            }
        }
        let target_transition =
            (view.position_m.x - ctx.target_pad.center_x_m).abs() <= ctx.target_pad.width_m * 0.5;
        if let Some(mut conflict) =
            reserve_conflict(request, ctx, &view, target_transition, "ideal_arc_reserve")?
        {
            conflict.state.physics_step += k;
            conflict.state.sim_time_s += k as f64 * ctx.sim.physics_dt_s();
            return Ok(Some(conflict));
        }
    }
    Ok(None)
}

fn correction_command(
    ctx: &RunContext,
    s: &SimulationState,
    correction: &Correction,
) -> Result<Command> {
    let mut command = Command {
        throttle_frac: 0.0,
        target_attitude_rad: correction.target_attitude_rad,
    };
    if s.physics_step >= correction.turn_end_physics_step
        && s.physics_step < correction.burn_end_physics_step
    {
        command.throttle_frac = paired_throttle(
            ctx,
            &BodyAwareTerminalPolicyV1::default(),
            s.mass_kg(ctx),
            correction.thrust_acceleration_mps2.length(),
        )?;
    }
    Ok(command)
}

fn reserve_conflict(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    landing: bool,
    cause: &str,
) -> Result<Option<PredictedConflict>> {
    let (clearance, _, floor) = body_clearance(request, ctx, s, landing)?;
    Ok((clearance < floor).then(|| PredictedConflict {
        state: SimulationStateSnapshotV1::from_state(s),
        cause: cause.into(),
    }))
}

fn short_conflict(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    command: Command,
    landing: bool,
) -> Result<Option<PredictedConflict>> {
    short_conflict_ticks(request, ctx, s, command, landing, REFRESH_TICKS)
}

fn short_conflict_ticks(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    command: Command,
    landing: bool,
    ticks: u64,
) -> Result<Option<PredictedConflict>> {
    let mut query = s.clone();
    query.set_command(command);
    for _ in 0..ticks {
        query.step_with_contact_report(ctx);
        ensure!(snapshot_finite(&query), "nonfinite short prediction");
        // Validate the terrain query even when contact classification ended the
        // clone. An invalid domain must not masquerade as a replannable crash.
        let reserve = reserve_conflict(request, ctx, &query, landing, "short_command_reserve")
            .map_err(|error| {
                if let Some(domain @ TerrainQueryError::DomainOverrun { .. }) =
                    error.downcast_ref::<TerrainQueryError>()
                {
                    anyhow::Error::new(TerrainDomainStop {
                        query: "short_command_prediction".into(),
                        actual_state: SimulationStateSnapshotV1::from_state(s),
                        query_origin: SimulationStateSnapshotV1::from_state(s),
                        query_state: SimulationStateSnapshotV1::from_state(&query),
                        requested_command: Some(command),
                        error: domain.clone(),
                    })
                } else {
                    error
                }
            })?;
        if query.is_terminal() {
            if landing
                && query.physical_outcome == PhysicalOutcome::LandedOnTarget
                && query.mission_outcome == MissionOutcome::Success
            {
                return Ok(None);
            }
            ensure!(
                query.physical_outcome != PhysicalOutcome::TimedOut,
                "predicted timeout"
            );
            return Ok(Some(PredictedConflict {
                state: SimulationStateSnapshotV1::from_state(&query),
                cause: "short_command_contact".into(),
            }));
        }
        if let Some(conflict) = reserve {
            return Ok(Some(conflict));
        }
    }
    Ok(None)
}

fn advance_one(
    ctx: &RunContext,
    live: &mut OrdinaryLive,
    update: &FlightProgramUpdateV1,
    neutral: bool,
) -> Result<()> {
    if !neutral {
        return advance_ordinary(
            ctx,
            live,
            std::slice::from_ref(update),
            live.state.physics_step + 2,
        );
    }
    let state = &mut live.state;
    ensure!(
        update.physics_step == state.physics_step,
        "neutral command clock mismatch"
    );
    state.set_command(update.command);
    live.evidence.actions.push(ActionLogEntry {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        controller_update_index: state.physics_step / 2,
        command: update.command,
    });
    live.evidence.events.push(EventRecord {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        kind: EventKind::ControllerUpdated,
        message: "controller_updated".into(),
    });
    for _ in 0..2 {
        state.step_physics_and_classify_contact(ctx);
        ensure!(snapshot_finite(state), "nonfinite neutral correction state");
        if ctx
            .sim
            .sample_interval_steps()
            .is_some_and(|k| state.physics_step.is_multiple_of(k))
        {
            live.evidence.samples.push(SampleRecord {
                sim_time_s: state.sim_time_s,
                physics_step: state.physics_step,
                observation: state.build_observation(ctx),
                held_command: state.held_command,
            });
        }
    }
    live.evidence.final_state = SimulationStateSnapshotV1::from_state(state);
    Ok(())
}

fn refresh_record(
    s: &SimulationState,
    goal: &Goal,
    arc: Option<BallisticAim>,
    correction: Option<Correction>,
    decision: &str,
    miss: Option<f64>,
) -> Refresh {
    Refresh {
        origin: SimulationStateSnapshotV1::from_state(s),
        goal: goal.clone(),
        desired_arc: arc,
        correction,
        decision: decision.into(),
        current_ballistic_miss_m: miss,
        terrain_conflict_m: None,
        predicted_conflict: None,
        predicted_command: None,
        previous_goal: None,
        replan_trigger: None,
        terrain_correction: None,
        recovery_query: None,
        waypoint_height_repair: None,
        ridge_selection: None,
        waypoint_braking_room: None,
        coast_terminal: None,
        coast_terminal_rejection: None,
        finite_destination_query: None,
        transition_query: None,
        recovery_common_query: None,
    }
}

fn record_conflict(record: &mut Refresh, conflict: PredictedConflict) {
    record.terrain_conflict_m = Some(conflict.state.position_m);
    record.predicted_conflict = Some(conflict);
}

struct ReplanQuery<'a> {
    request: &'a WaypointDirectNominalDirectGenerationRequest,
    ctx: &'a RunContext,
    state: &'a SimulationState,
    deadline: u64,
    handoff_number: usize,
    experiment: WaypointExperiment,
}

enum LocalReplan {
    Accepted(Goal, Box<BallisticAim>, Correction),
    Stopped(String),
}

/// A bounded query transaction. Never advances the live plant or records H.
fn local_replan(
    query: ReplanQuery<'_>,
    old_goal: &Goal,
    mut conflict: PredictedConflict,
    next_revision: &mut usize,
    refreshes: &mut Vec<Refresh>,
) -> Result<LocalReplan> {
    // The active goal is not a failed proposal at this actual state. A fresh
    // correction to it may be safe even when its current unpowered coast is not.
    let mut seen = Vec::new();
    let trigger = conflict.clone();
    let mut height_repair: Option<clearance::WaypointHeightRepair> = None;
    let mut ridge_selection = None;
    for _ in 0..MAX_LOCAL_PROPOSALS {
        let mut goal = if let Some(repair) = &height_repair {
            let mut goal = repair.from_goal.clone();
            goal.position_m.y += repair.height_increase_m;
            goal
        } else {
            match ridge::waypoint_for(
                query.ctx,
                query.state,
                conflict.state.position_m,
                query.handoff_number,
                query.experiment.local_height(),
            ) {
                Ok((goal, selection)) => {
                    ridge_selection = Some(selection);
                    goal
                }
                Err(e) => return Ok(LocalReplan::Stopped(format!("no_local_waypoint: {e}"))),
            }
        };
        if seen.contains(&goal.position_m) {
            return Ok(LocalReplan::Stopped("local_replan_duplicate_goal".into()));
        }
        seen.push(goal.position_m);
        let same_goal = !old_goal.destination && goal.position_m == old_goal.position_m;
        if same_goal {
            goal.revision = old_goal.revision;
        } else {
            goal.revision = *next_revision;
            *next_revision += 1;
        }
        let proposal = construct_for(
            query.ctx,
            query.state,
            &goal,
            query.deadline,
            query.experiment,
        );
        let mut record = refresh_record(
            query.state,
            &goal,
            proposal.as_ref().map(|p| p.0.clone()),
            proposal.as_ref().map(|p| p.1.clone()),
            "waypoint_aim_construction_miss",
            None,
        );
        record.previous_goal = Some(old_goal.clone());
        record.replan_trigger = Some(trigger.clone());
        record.waypoint_height_repair = height_repair.clone();
        record.ridge_selection = ridge_selection.clone();
        let Some((arc, correction)) = proposal else {
            refreshes.push(record);
            return Ok(LocalReplan::Stopped(
                "waypoint_aim_construction_miss".into(),
            ));
        };
        let blocked = first_conflict(query.request, query.ctx, query.state, &arc)?;
        let blocked = if blocked.is_none()
            && (height_repair.is_some() || query.experiment.exit_consistency())
        {
            clearance::continuation_conflict(query.request, query.ctx, query.state, &goal, &arc)?
        } else {
            blocked
        };
        if let Some(blocked) = blocked {
            record.decision = "waypoint_proposal_blocked".into();
            record_conflict(&mut record, blocked.clone());
            refreshes.push(record);
            conflict = blocked;
            height_repair =
                clearance::height_repair(query.request, query.ctx, query.state, &goal, &arc)?;
            if height_repair.is_none() {
                return Ok(LocalReplan::Stopped(
                    "waypoint_clearance_repair_miss".into(),
                ));
            }
            continue;
        }
        // Goal selection owns geometric obstruction, not immediate actuator
        // safety. The mandatory command guard/recovery runs before execution.
        record.decision = if same_goal {
            "waypoint_reacquired"
        } else if old_goal.destination {
            "waypoint_selected"
        } else {
            "waypoint_replaced"
        }
        .into();
        refreshes.push(record);
        return Ok(LocalReplan::Accepted(goal, Box::new(arc), correction));
    }
    Ok(LocalReplan::Stopped("local_replan_proposal_budget".into()))
}

struct Execution {
    live: OrdinaryLive,
    refreshes: Vec<Refresh>,
    handoffs: Vec<SimulationStateSnapshotV1>,
    handoff_goal_revisions: Vec<usize>,
    updates: Vec<FlightProgramUpdateV1>,
    stop: String,
    terrain_domain_stop: Option<TerrainDomainStop>,
    controller_updates: Vec<pd_control::ControllerUpdateRecord>,
}

fn execute(
    request: &WaypointDirectNominalDirectGenerationRequest,
    mut live: OrdinaryLive,
    neutral: bool,
    correction_cap: usize,
    experiment: WaypointExperiment,
) -> Result<Execution> {
    let ctx = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let deadline = pd_plan::waypoint_v2::original_deadline(
        ctx.sim.max_time_s,
        request.policy.analytical_policy.mission_budget_s(),
    )
    .map_err(anyhow::Error::msg)?;
    let mut goal = target(&ctx);
    let mut correction: Option<Correction> = None;
    let mut next_refresh = live.state.physics_step;
    let mut refreshes = Vec::new();
    let mut handoffs = Vec::new();
    let mut handoff_goal_revisions = Vec::new();
    let mut next_revision = 1;
    let mut updates = Vec::new();
    let mut controller_updates = Vec::new();
    let mut next_transition_query = live.state.physics_step;
    let mut next_takeover_query = live.state.physics_step;
    let mut next_common_query = live.state.physics_step;
    let mut terminal = TerminalPdgController::default();
    if experiment.landing_duration() {
        terminal = terminal.with_ballistic_landing_duration_fallback();
    }
    if experiment.landing_countdown() {
        terminal = terminal.with_ballistic_landing_countdown();
    }
    if experiment.landing_braking_guard() {
        terminal = terminal.with_ballistic_landing_braking_guard();
    }
    if experiment.landing_body_centering() {
        terminal = terminal.with_ballistic_landing_body_centering();
    }
    if experiment.terminal_coordination() {
        terminal = terminal.with_ballistic_terminal_coordination();
    }
    let mut landing = false;
    let mut standalone_terminal = false;
    let mut ready_ticks = 0;
    let mut source_cleared = live.state.physics_step > 0;
    let mut coast_started: Option<u64> = None;
    let mut recovery_started: Option<u64> = None;
    let mut pending_terminal_coast: Option<coast_terminal::Preview> = None;
    let mut coast_declined_at: Option<u64> = None;
    let mut next_recovery_warning = live.state.physics_step;
    let started = std::time::Instant::now();
    let mut terrain_domain_stop = None;
    let execution_result = (|| -> Result<String> {
        let stop = loop {
            let s = &live.state;
            if started.elapsed().as_secs() >= 60 {
                break "case_wall_bound".into();
            }
            if s.is_terminal() {
                break "physical_terminal".into();
            }
            if s.physics_step + REFRESH_TICKS >= deadline || s.fuel_kg <= 0.0 {
                break "original_budget".into();
            }
            if !neutral && let Err(e) = body_safe(request, &ctx, s, landing) {
                if experiment.terminal_coordination()
                    && let Some(domain @ TerrainQueryError::DomainOverrun { .. }) =
                        e.downcast_ref::<TerrainQueryError>()
                {
                    let state = SimulationStateSnapshotV1::from_state(s);
                    terrain_domain_stop = Some(TerrainDomainStop {
                        query: "actual_body_reserve".into(),
                        actual_state: state.clone(),
                        query_origin: state.clone(),
                        query_state: state,
                        requested_command: None,
                        error: domain.clone(),
                    });
                    break "actual_terrain_domain".into();
                }
                break format!("actual_body_reserve: {e}");
            }
            if !source_cleared {
                let source = ctx
                    .world
                    .landing_pad(&request.source_pad_id)
                    .context("source pad missing")?;
                if s.position_m.y - source.surface_y_m
                    >= ctx.vehicle.geometry.touchdown_base_offset_m + 5.0
                {
                    // Upright departure changed the state during acquisition. Do
                    // not reuse turn/cutoff clocks from before that safety phase.
                    source_cleared = true;
                    correction = None;
                    next_refresh = s.physics_step;
                }
            }
            if let Some(episode_start) = recovery_started {
                ensure!(
                    !landing && source_cleared && !neutral,
                    "invalid recovery ownership"
                );
                if s.physics_step - episode_start >= avoidance::MAX_RECOVERY_TICKS {
                    break "terrain_recovery_budget".into();
                }
                let proposal = construct_for(&ctx, s, &goal, deadline, experiment);
                let requested = proposal
                    .as_ref()
                    .map(|(_, plan)| correction_command(&ctx, s, plan))
                    .transpose()?
                    .unwrap_or_default();
                let horizon = if experiment.recovery_lead() {
                    avoidance::warning_ticks(&ctx, s, requested)
                } else {
                    avoidance::response_ticks(&ctx, s, requested)
                };
                let common = if experiment.queued_recovery() {
                    let comparison = phase_transition::bounded_recovery_comparison(
                        request,
                        &ctx,
                        s,
                        requested,
                        proposal.as_ref().map(|p| &p.1),
                    )?;
                    if s.physics_step >= next_common_query {
                        next_common_query = s.physics_step + REFRESH_TICKS;
                        let mut record = refresh_record(
                            s,
                            &goal,
                            None,
                            proposal.as_ref().map(|p| p.1.clone()),
                            "recovery_common_horizon_query",
                            None,
                        );
                        record.recovery_common_query = Some(comparison.clone());
                        refreshes.push(record);
                    }
                    Some(comparison)
                } else {
                    None
                };
                let resume = if let Some(common) = &common {
                    common.queued_program.accepted()
                        && (proposal.is_some() || accepted_coast(&ctx, s, &goal).is_some())
                } else {
                    short_conflict_ticks(request, &ctx, s, requested, false, horizon)?.is_none()
                };
                if resume {
                    let mut record = refresh_record(
                        s,
                        &goal,
                        proposal.as_ref().map(|p| p.0.clone()),
                        proposal.as_ref().map(|p| p.1.clone()),
                        "terrain_recovery_resumed",
                        None,
                    );
                    record.predicted_command = Some(requested);
                    refreshes.push(record);
                    recovery_started = None;
                    correction = None;
                    next_refresh = s.physics_step;
                    coast_started = None;
                    ready_ticks = 0;
                } else {
                    let (selected, query) = if let Some(common) = &common {
                        avoidance::select_common(common, episode_start)
                    } else {
                        avoidance::select_with_evidence(request, &ctx, s, requested, episode_start)?
                    };
                    let Some(selected) = selected else {
                        if experiment.mechanics() {
                            let mut record = refresh_record(
                                s,
                                &goal,
                                None,
                                None,
                                "terrain_recovery_exhausted",
                                None,
                            );
                            record.recovery_query = Some(query);
                            refreshes.push(record);
                        }
                        break "terrain_recovery_no_safe_command".into();
                    };
                    if s.physics_step >= next_refresh {
                        let mut record = refresh_record(
                            s,
                            &goal,
                            proposal.as_ref().map(|p| p.0.clone()),
                            proposal.as_ref().map(|p| p.1.clone()),
                            "terrain_recovery_command",
                            None,
                        );
                        record.predicted_command = Some(selected.selected_command);
                        record.terrain_correction = Some(selected.clone());
                        if experiment.mechanics() {
                            record.recovery_query = Some(query);
                        }
                        refreshes.push(record);
                        next_refresh = s.physics_step + REFRESH_TICKS;
                    }
                    let update = FlightProgramUpdateV1 {
                        physics_step: s.physics_step,
                        command: selected.selected_command,
                        phase: "terrain_clearance_recovery".into(),
                    };
                    advance_one(&ctx, &mut live, &update, false)?;
                    updates.push(update);
                    continue;
                }
            }
            if !goal.destination
                && s.held_command.throttle_frac == 0.0
                && s.position_m.x >= goal.position_m.x - tolerance(&ctx, &goal)
                && s.position_m.y >= goal.position_m.y - tolerance(&ctx, &goal)
            {
                let mut query = s.clone();
                query.set_command(Command::default());
                let mut safe = true;
                for _ in 0..CONTINUATION_TICKS {
                    query.step_with_contact_report(&ctx);
                    if query.is_terminal() || body_safe(request, &ctx, &query, false).is_err() {
                        safe = false;
                        break;
                    }
                }
                if !safe {
                    break "waypoint_continuation_rejected".into();
                }
                handoffs.push(SimulationStateSnapshotV1::from_state(s));
                handoff_goal_revisions.push(goal.revision);
                goal = target(&ctx);
                goal.revision = next_revision;
                next_revision += 1;
                correction = None;
                next_refresh = s.physics_step;
                coast_started = None;
                ready_ticks = 0;
            }
            if let Some(mut preview) = pending_terminal_coast.take() {
                ensure!(
                    !landing && goal.destination,
                    "invalid coast-terminal ownership"
                );
                if s.position_m.x >= preview.clearance_x_m {
                    if let Some(entry) = coast_terminal::entry_preview(request, &ctx, s, deadline)?
                    {
                        preview.entry = entry;
                        let mut record = refresh_record(
                            s,
                            &goal,
                            None,
                            None,
                            "coast_terminal_entry",
                            Some(preview.entry.ballistic_miss_m),
                        );
                        record.predicted_command = Some(preview.entry.initial_command);
                        record.coast_terminal = Some(preview);
                        refreshes.push(record);
                        terminal = TerminalPdgController::default();
                        standalone_terminal = true;
                        landing = true;
                        correction = None;
                    } else {
                        let mut record =
                            refresh_record(s, &goal, None, None, "coast_through_cancelled", None);
                        record.coast_terminal = Some(preview);
                        record.coast_terminal_rejection =
                            Some("actual_entry_revalidation_rejected".into());
                        refreshes.push(record);
                        coast_declined_at = Some(s.physics_step);
                        correction = None;
                        next_refresh = s.physics_step;
                    }
                } else if let Some(conflict) =
                    short_conflict(request, &ctx, s, Command::default(), false)?
                {
                    let mut record =
                        refresh_record(s, &goal, None, None, "coast_through_cancelled", None);
                    record.coast_terminal = Some(preview);
                    record.coast_terminal_rejection = Some("actual_coast_command_rejected".into());
                    record_conflict(&mut record, conflict);
                    refreshes.push(record);
                    coast_declined_at = Some(s.physics_step);
                    correction = None;
                    next_refresh = s.physics_step;
                } else {
                    let update = FlightProgramUpdateV1 {
                        physics_step: s.physics_step,
                        command: Command::default(),
                        phase: "coast_through_to_terminal".into(),
                    };
                    advance_one(&ctx, &mut live, &update, false)?;
                    updates.push(update);
                    pending_terminal_coast = Some(preview);
                    continue;
                }
            }
            let mut coast = accepted_coast(&ctx, s, &goal);
            let coast_refresh_due = s.physics_step >= next_refresh
                || (!experiment.finite_correction() && correction.is_some());
            if !neutral
                && !landing
                && let Some((n, miss)) = coast
                && (s.physics_step >= next_refresh || correction.is_some())
            {
                let end = aim::project(
                    kinematics(s),
                    ctx.world.gravity_mps2,
                    ctx.sim.physics_dt_s(),
                    n,
                );
                let actual_arc = aim::target_arc(
                    kinematics(s),
                    end.position_m,
                    ctx.world.gravity_mps2,
                    ctx.sim.physics_dt_s(),
                    n,
                )
                .context("invalid coast query")?;
                let mut obstruction = first_conflict(request, &ctx, s, &actual_arc)?;
                if experiment.phase_queries()
                    && (s.physics_step >= next_transition_query || correction.is_some())
                {
                    next_transition_query = s.physics_step + REFRESH_TICKS;
                    let query = phase_transition::coast_settling(request, &ctx, s)?;
                    if experiment.coast_transition() && obstruction.is_none() {
                        obstruction = query.conflict.clone();
                        if let Some(domain) = &query.domain_stop {
                            return Err(anyhow::Error::new(domain.clone()));
                        }
                    }
                    let mut record = refresh_record(
                        s,
                        &goal,
                        None,
                        correction.clone(),
                        "actual_coast_settling_query",
                        Some(miss),
                    );
                    record.transition_query = Some(query);
                    refreshes.push(record);
                }
                if let Some(conflict) = obstruction {
                    let finite_destination =
                        experiment.finite_correction() && goal.destination && goal.revision > 0;
                    let continuing =
                        if experiment.recovery() && (!goal.destination || finite_destination) {
                            correction
                                .as_ref()
                                .and_then(|old| retained_correction(&ctx, s, &goal, old))
                        } else {
                            None
                        };
                    let mut finite_query = None;
                    let pending_clear = if let Some((arc, plan)) = &continuing {
                        if finite_destination {
                            if s.physics_step >= next_refresh {
                                let audit = finite_correction::audit_for(
                                    request,
                                    &ctx,
                                    s,
                                    &goal,
                                    Some(plan),
                                    experiment,
                                )?;
                                let accepted = audit.accepted();
                                finite_query = Some(finite_correction::Query::retention(
                                    arc, plan, None, audit,
                                ));
                                accepted
                            } else {
                                // Carry the already admitted absolute-clock leg
                                // between scheduled checks; the mandatory actual
                                // short-command guard below still owns every pair.
                                true
                            }
                        } else {
                            pending_coast_clear(request, &ctx, s, plan)?
                        }
                    } else {
                        false
                    };
                    if let Some((arc, plan)) = continuing
                        && pending_clear
                    {
                        // An unsafe coast now is not a route obstruction while a
                        // viable correction is still establishing its safe coast.
                        if s.physics_step >= next_refresh {
                            let mut record = refresh_record(
                                s,
                                &goal,
                                Some(arc),
                                Some(plan.clone()),
                                if finite_destination {
                                    "destination_acquisition_continues"
                                } else {
                                    "waypoint_acquisition_continues"
                                },
                                Some(miss),
                            );
                            record.replan_trigger = Some(conflict);
                            record.finite_destination_query = finite_query;
                            refreshes.push(record);
                            next_refresh = s.physics_step + REFRESH_TICKS;
                        }
                        correction = Some(plan);
                        coast = None;
                    } else {
                        let mut record = refresh_record(
                            s,
                            &goal,
                            Some(actual_arc),
                            None,
                            "ballistic_obstruction",
                            Some(miss),
                        );
                        record_conflict(&mut record, conflict.clone());
                        record.finite_destination_query = finite_query;
                        refreshes.push(record);
                        if handoffs.len() >= correction_cap {
                            break "correction_budget".into();
                        }
                        match local_replan(
                            ReplanQuery {
                                request,
                                ctx: &ctx,
                                state: s,
                                deadline,
                                handoff_number: handoffs.len() + 1,
                                experiment,
                            },
                            &goal,
                            conflict,
                            &mut next_revision,
                            &mut refreshes,
                        )? {
                            LocalReplan::Accepted(new_goal, _, plan) => {
                                goal = new_goal;
                                correction = Some(plan);
                            }
                            LocalReplan::Stopped(reason) => break reason,
                        }
                        next_refresh = s.physics_step + REFRESH_TICKS;
                        coast_started = None;
                        ready_ticks = 0;
                        coast = None;
                    }
                } else {
                    next_refresh = s.physics_step + REFRESH_TICKS;
                }
            }
            let mut finite_preview = None;
            if experiment.finite_queries()
                && !neutral
                && !landing
                && !goal.destination
                && s.held_command.throttle_frac == 0.0
                && coast_refresh_due
                && let Some((steps, _)) = coast
            {
                let (query, preview) =
                    finite_correction::preview_for(request, &ctx, s, steps, deadline, experiment)?;
                let mut record = refresh_record(
                    s,
                    &target(&ctx),
                    query.desired_arc.clone(),
                    query.correction.clone(),
                    "finite_destination_query",
                    None,
                );
                record.previous_goal = Some(goal.clone());
                record.finite_destination_query = Some(query);
                refreshes.push(record);
                finite_preview = preview;
            }
            if experiment.early_target()
                && !neutral
                && !landing
                && !goal.destination
                && s.held_command.throttle_frac == 0.0
                && coast_refresh_due
                && let Some((steps, _)) = coast
                && let Some(mut preview) = if experiment.finite_correction() {
                    finite_preview
                } else {
                    early_destination_preview(
                        request,
                        &ctx,
                        s,
                        steps,
                        deadline,
                        experiment.piecewise_early_target(),
                    )?
                }
            {
                preview.goal.revision = next_revision;
                next_revision += 1;
                let mut record = refresh_record(
                    s,
                    &preview.goal,
                    Some(preview.arc),
                    preview.correction.clone(),
                    if preview.obstruction.is_some() {
                        "early_destination_obstruction"
                    } else {
                        "destination_reacquired_before_waypoint"
                    },
                    None,
                );
                record.previous_goal = Some(goal.clone());
                record.waypoint_braking_room = Some(preview.waypoint_room);
                if let Some(conflict) = &preview.obstruction {
                    record_conflict(&mut record, conflict.clone());
                }
                refreshes.push(record);
                if let Some(conflict) = preview.obstruction {
                    match local_replan(
                        ReplanQuery {
                            request,
                            ctx: &ctx,
                            state: s,
                            deadline,
                            handoff_number: handoffs.len() + 1,
                            experiment,
                        },
                        &goal,
                        conflict,
                        &mut next_revision,
                        &mut refreshes,
                    )? {
                        LocalReplan::Accepted(new_goal, _, plan) => {
                            goal = new_goal;
                            correction = Some(plan);
                        }
                        LocalReplan::Stopped(reason) => break reason,
                    }
                } else {
                    goal = preview.goal;
                    correction = preview.correction;
                }
                coast = if correction.is_some() {
                    None
                } else {
                    accepted_coast(&ctx, s, &goal)
                };
                next_refresh = s.physics_step + REFRESH_TICKS;
                coast_started = None;
                ready_ticks = 0;
            }
            if !landing && let Some((n, miss)) = coast {
                if correction.is_some() || refreshes.is_empty() {
                    refreshes.push(refresh_record(
                        s,
                        &goal,
                        aim::target_arc(
                            kinematics(s),
                            goal.position_m,
                            ctx.world.gravity_mps2,
                            ctx.sim.physics_dt_s(),
                            n,
                        ),
                        None,
                        "accepted_ballistic_continuation",
                        Some(miss),
                    ));
                }
                correction = None;
                let coast_start = *coast_started.get_or_insert(s.physics_step);
                if neutral
                    && s.held_command.throttle_frac == 0.0
                    && s.physics_step >= coast_start + REFRESH_TICKS
                {
                    refreshes.push(refresh_record(
                        s,
                        &goal,
                        aim::target_arc(
                            kinematics(s),
                            goal.position_m,
                            ctx.world.gravity_mps2,
                            ctx.sim.physics_dt_s(),
                            n,
                        ),
                        None,
                        "verified_engine_off_coast",
                        Some(miss),
                    ));
                    break "ballistic_established_diagnostic".into();
                }
                if !neutral
                    && goal.destination
                    && s.held_command.throttle_frac == 0.0
                    && s.physics_step > coast_start
                    && terminal.ballistic_entry_ready(
                        &ctx,
                        &s.build_observation(&ctx),
                        miss,
                        &mut ready_ticks,
                    )
                {
                    let accepted =
                        if experiment.phase_queries() && s.physics_step >= next_takeover_query {
                            next_takeover_query = s.physics_step + REFRESH_TICKS;
                            let query = phase_transition::terminal_takeover(
                                request, &ctx, s, &terminal, deadline, experiment,
                            )?;
                            let accepted = query.accepted();
                            let mut record = refresh_record(
                                s,
                                &goal,
                                None,
                                None,
                                "configured_terminal_takeover_query",
                                Some(miss),
                            );
                            record.transition_query = Some(query);
                            refreshes.push(record);
                            !experiment.terminal_takeover() || accepted
                        } else {
                            !experiment.terminal_takeover()
                        };
                    if accepted {
                        landing = true;
                        refreshes.push(refresh_record(
                            s,
                            &goal,
                            None,
                            None,
                            "maintained_landing_entry",
                            Some(miss),
                        ));
                    }
                }
            }
            if coast.is_none() {
                coast_started = None;
            }
            if !landing
                && coast.is_none()
                && (s.physics_step >= next_refresh || correction.is_none())
            {
                let retained = correction
                    .as_ref()
                    .and_then(|old| retained_correction(&ctx, s, &goal, old));
                let Some((mut arc, mut plan)) =
                    retained.or_else(|| construct_for(&ctx, s, &goal, deadline, experiment))
                else {
                    refreshes.push(refresh_record(
                        s,
                        &goal,
                        None,
                        None,
                        "finite_aim_construction_miss",
                        None,
                    ));
                    break "no_ballistic_aim".into();
                };
                let mut acquisition_query = None;
                let mut obstruction = if !neutral {
                    first_conflict(request, &ctx, s, &arc)?
                } else {
                    None
                };
                if !neutral
                    && experiment.finite_correction()
                    && goal.destination
                    && goal.revision > 0
                {
                    let audit = finite_correction::audit_for(
                        request,
                        &ctx,
                        s,
                        &goal,
                        Some(&plan),
                        experiment,
                    )?;
                    if audit.accepted() {
                        obstruction = None;
                    } else {
                        obstruction = audit.conflict.clone().or(obstruction);
                    }
                    acquisition_query = Some(finite_correction::Query::retention(
                        &arc,
                        &plan,
                        first_conflict(request, &ctx, s, &arc)?,
                        audit,
                    ));
                    if obstruction.is_none()
                        && acquisition_query.as_ref().unwrap().rejection.is_some()
                    {
                        let mut record = refresh_record(
                            s,
                            &goal,
                            Some(arc),
                            Some(plan),
                            "destination_acquisition_declined",
                            None,
                        );
                        record.finite_destination_query = acquisition_query;
                        refreshes.push(record);
                        break "finite_destination_acquisition_rejected".into();
                    }
                }
                if let Some(conflict) = obstruction {
                    let mut record = refresh_record(
                        s,
                        &goal,
                        Some(arc.clone()),
                        Some(plan.clone()),
                        "ballistic_obstruction",
                        None,
                    );
                    record_conflict(&mut record, conflict.clone());
                    record.finite_destination_query = acquisition_query.take();
                    refreshes.push(record);
                    if experiment.coast_terminal()
                        && goal.destination
                        && !handoffs.is_empty()
                        && s.held_command.throttle_frac == 0.0
                        && coast_declined_at != Some(s.physics_step)
                    {
                        match coast_terminal::propose(request, &ctx, s, &conflict, deadline)? {
                            coast_terminal::Proposal::Accepted(preview) => {
                                let mut record = refresh_record(
                                    s,
                                    &goal,
                                    Some(arc.clone()),
                                    None,
                                    "coast_through_selected",
                                    None,
                                );
                                record_conflict(&mut record, conflict.clone());
                                record.coast_terminal = Some(*preview.clone());
                                refreshes.push(record);
                                pending_terminal_coast = Some(*preview);
                                correction = None;
                                continue;
                            }
                            coast_terminal::Proposal::Declined(reason) => {
                                let mut record = refresh_record(
                                    s,
                                    &goal,
                                    None,
                                    None,
                                    "coast_through_rejected",
                                    None,
                                );
                                record.coast_terminal_rejection = Some(reason.into());
                                refreshes.push(record);
                            }
                        }
                    }
                    if handoffs.len() >= correction_cap {
                        break "correction_budget".into();
                    }
                    match local_replan(
                        ReplanQuery {
                            request,
                            ctx: &ctx,
                            state: s,
                            deadline,
                            handoff_number: handoffs.len() + 1,
                            experiment,
                        },
                        &goal,
                        conflict,
                        &mut next_revision,
                        &mut refreshes,
                    )? {
                        LocalReplan::Accepted(new_goal, new_arc, new_plan) => {
                            goal = new_goal;
                            arc = *new_arc;
                            plan = new_plan;
                        }
                        LocalReplan::Stopped(reason) => break reason,
                    }
                    coast_started = None;
                    ready_ticks = 0;
                }
                let mut record = refresh_record(
                    s,
                    &goal,
                    Some(arc),
                    Some(plan.clone()),
                    "powered_correction",
                    None,
                );
                record.finite_destination_query = acquisition_query;
                refreshes.push(record);
                correction = Some(plan);
                next_refresh = s.physics_step + REFRESH_TICKS;
            }
            let mut terminal_frame = None;
            let (mut command, mut phase) = if landing {
                let mut frame = terminal.update(&ctx, &s.build_observation(&ctx));
                if experiment.pad_clearance() && !standalone_terminal {
                    phase_transition::protect_pad(request, &ctx, s, &mut frame)?;
                }
                let command = frame.command;
                terminal_frame = Some(frame);
                (command, "maintained_terminal")
            } else if !source_cleared {
                // Original source support: establish upright clearance before tilt.
                (
                    Command {
                        throttle_frac: 1.0,
                        target_attitude_rad: 0.0,
                    },
                    "source_clearance",
                )
            } else if let Some(plan) = &correction {
                match correction_command(&ctx, s, plan) {
                    Ok(c) => (c, "ballistic_correction"),
                    Err(e) => break format!("correction_actuator: {e}"),
                }
            } else {
                (Command::default(), "ballistic_coast")
            };
            let mut common_query = None;
            if experiment.phase_queries()
                && !landing
                && source_cleared
                && s.physics_step >= next_common_query
            {
                next_common_query = s.physics_step + REFRESH_TICKS;
                let mut record = refresh_record(
                    s,
                    &goal,
                    None,
                    correction.clone(),
                    "recovery_common_horizon_query",
                    None,
                );
                let comparison = if experiment.recovery_consistency() {
                    phase_transition::bounded_recovery_comparison(
                        request,
                        &ctx,
                        s,
                        command,
                        correction.as_ref(),
                    )?
                } else {
                    phase_transition::recovery_comparison(
                        request,
                        &ctx,
                        s,
                        command,
                        correction.as_ref(),
                    )?
                };
                record.recovery_common_query = Some(comparison.clone());
                common_query = Some(comparison);
                refreshes.push(record);
            }
            let warning_ticks = if experiment.recovery_lead()
                && !landing
                && source_cleared
                && s.physics_step >= next_recovery_warning
            {
                next_recovery_warning = s.physics_step + REFRESH_TICKS;
                avoidance::warning_ticks(&ctx, s, command)
            } else {
                REFRESH_TICKS
            };
            let immediate_conflict = if neutral {
                None
            } else {
                short_conflict(request, &ctx, s, command, landing)?
            };
            let mut command_conflict =
                if !neutral && immediate_conflict.is_none() && warning_ticks > REFRESH_TICKS {
                    short_conflict_ticks(request, &ctx, s, command, landing, warning_ticks)?
                } else {
                    immediate_conflict
                };
            // A diagnostic warning alone cannot terminate an otherwise clear
            // live prefix. Intervene early only with a passing existing response.
            if experiment.queued_recovery()
                && command_conflict.is_none()
                && let Some(common) = &common_query
                && common
                    .commands
                    .iter()
                    .any(|q| q.checked && q.conflict.is_none())
            {
                command_conflict = common.queued_program.conflict.clone();
            }
            if let Some(conflict) = command_conflict {
                let mut record = refresh_record(
                    s,
                    &goal,
                    None,
                    correction.clone(),
                    "short_command_obstruction",
                    None,
                );
                record.predicted_command = Some(command);
                record_conflict(&mut record, conflict.clone());
                refreshes.push(record);
                if landing || !source_cleared {
                    break format!("short_command_rejected: {}", conflict.cause);
                }
                let (selected, query) = if experiment.queued_recovery() {
                    if common_query.is_none() {
                        let comparison = phase_transition::bounded_recovery_comparison(
                            request,
                            &ctx,
                            s,
                            command,
                            correction.as_ref(),
                        )?;
                        let mut record = refresh_record(
                            s,
                            &goal,
                            None,
                            correction.clone(),
                            "recovery_common_horizon_query",
                            None,
                        );
                        record.recovery_common_query = Some(comparison.clone());
                        refreshes.push(record);
                        common_query = Some(comparison);
                    }
                    avoidance::select_common(common_query.as_ref().unwrap(), s.physics_step)
                } else {
                    avoidance::select_with_evidence(request, &ctx, s, command, s.physics_step)?
                };
                let Some(selected) = selected else {
                    if experiment.mechanics() {
                        let mut record = refresh_record(
                            s,
                            &goal,
                            None,
                            None,
                            "terrain_recovery_exhausted",
                            None,
                        );
                        record.recovery_query = Some(query);
                        refreshes.push(record);
                    }
                    break "terrain_recovery_no_safe_command".into();
                };
                let mut record = refresh_record(
                    s,
                    &goal,
                    None,
                    correction.clone(),
                    "terrain_recovery_started",
                    None,
                );
                record_conflict(&mut record, conflict);
                record.predicted_command = Some(selected.selected_command);
                record.terrain_correction = Some(selected.clone());
                if experiment.mechanics() {
                    record.recovery_query = Some(query);
                }
                refreshes.push(record);
                command = selected.selected_command;
                phase = "terrain_clearance_recovery";
                recovery_started = Some(s.physics_step);
                correction = None;
                next_refresh = s.physics_step + REFRESH_TICKS;
                coast_started = None;
                ready_ticks = 0;
            }
            let update = FlightProgramUpdateV1 {
                physics_step: s.physics_step,
                command,
                phase: phase.into(),
            };
            if experiment.phase_queries()
                && let Some(frame) = terminal_frame
            {
                ensure!(
                    frame.command == command,
                    "issued terminal frame binding differs"
                );
                controller_updates.push(pd_control::ControllerUpdateRecord {
                    sim_time_s: s.sim_time_s,
                    physics_step: s.physics_step,
                    controller_update_index: s.physics_step / 2,
                    compute_time_us: None,
                    frame,
                });
            }
            advance_one(&ctx, &mut live, &update, neutral)?;
            updates.push(update);
        };
        Ok(stop)
    })();
    let stop = match execution_result {
        Err(error)
            if experiment.terminal_coordination()
                && error.downcast_ref::<TerrainDomainStop>().is_some() =>
        {
            let mut diagnostic = error.downcast::<TerrainDomainStop>().unwrap();
            diagnostic.actual_state = SimulationStateSnapshotV1::from_state(&live.state);
            terrain_domain_stop = Some(diagnostic);
            "prediction_terrain_domain".into()
        }
        result => result?,
    };
    Ok(Execution {
        live,
        refreshes,
        handoffs,
        handoff_goal_revisions,
        updates,
        stop,
        terrain_domain_stop,
        controller_updates,
    })
}

/// Explicit candidate frontdoor. An optional retained cycle is rebuilt from
/// original source commands; no arbitrary airborne snapshot can be supplied.
pub fn run(
    scenario_path: &Path,
    baseline_path: Option<&Path>,
    cycle_index: usize,
    neutral: bool,
    correction_cap: usize,
    experiment: WaypointExperiment,
    output: &Path,
) -> Result<FeedbackResult> {
    ensure!(
        !neutral || baseline_path.is_some(),
        "neutral diagnostic needs a proven retained prefix"
    );
    ensure!(
        [6, 24].contains(&correction_cap),
        "unsupported candidate correction cap"
    );
    reserve_output_root(output)?;
    let scenario_bytes = fs::read(scenario_path)?;
    let scenario: ScenarioSpec = serde_json::from_slice(&scenario_bytes)?;
    let ctx = RunContext::from_scenario(&scenario).map_err(anyhow::Error::msg)?;
    let request = WaypointDirectNominalDirectGenerationRequest {
        probe_id: scenario.id.clone(),
        scenario: scenario.clone(),
        source_pad_id: "pad_source".into(),
        target_pad_id: ctx.target_pad.id.clone(),
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    };
    let source_state = crate::current_workspace_state()?;
    let content_source = crate::waypoint_v2_pack::capture_source_state(&crate::repo_root())?;
    let report_script_sha256 = sha256_bytes(&fs::read(
        crate::repo_root().join("pd-report/src/planning_cycles.js"),
    )?)?;
    let binary_sha256 = sha256_bytes(&fs::read(std::env::current_exe()?)?)?;
    let transition_metadata = serde_json::json!({
        "queries_recorded": experiment.phase_queries(),
        "actual_coast_settling": experiment.coast_transition(),
        "configured_terminal_takeover": experiment.terminal_takeover(),
        "pad_reserve_command_adapter": experiment.pad_clearance(),
        "recovery_diagnostic_only": !experiment.queued_recovery(),
        "terminal_prefix_ticks": CONTINUATION_TICKS,
        "query_refresh_ticks": REFRESH_TICKS,
        "ordinary_default_changed": false,
        "physical_guards_changed": false,
    });
    write_json_create_only(
        &output.join("attempt.json"),
        &serde_json::json!({
            "candidate_id": experiment.candidate_id(), "commit": source_state.commit_key,
            "workspace": source_state.workspace_key, "dirty": source_state.dirty,
            "content_source": content_source, "report_script_sha256": report_script_sha256,
            "binary_sha256": binary_sha256, "scenario_sha256": sha256_bytes(&scenario_bytes)?,
            "baseline_path": baseline_path, "cycle_index": cycle_index,
            "terrain_neutral_diagnostic": neutral, "correction_cap": correction_cap,
            "refresh_ticks": REFRESH_TICKS, "minimum_descent_angle_rad": MIN_ANGLE,
            "short_prediction_ticks": REFRESH_TICKS, "continuation_ticks": CONTINUATION_TICKS,
            "maximum_profile_trials": MAX_PROFILE_TRIALS, "case_wall_limit_s": 60,
            "maximum_local_proposals": MAX_LOCAL_PROPOSALS,
            "terminal_coordination": {
                "enabled": experiment.terminal_coordination(),
                "early_braking": "preserve_nominal_lateral_acceleration_with_lift_constraint",
                "touchdown": "body_contained_low_energy_upright_settlement",
                "ordinary_default_changed": false,
                "standalone_coast_terminal_changed": false,
                "physical_guards_changed": false,
            },
            "landing_body_centering": {
                "enabled": experiment.landing_body_centering(),
                "rule": "rotated_hull_and_feet_rescue_pad_interval",
                "reuse_outside_pad_lateral_target": true,
                "vertical_authority_cap_unchanged": true,
                "standalone_coast_terminal_changed": false,
                "ordinary_default_changed": false,
                "short_command_guard_changed": false,
            },
            "landing_braking_guard": {
                "enabled": experiment.landing_braking_guard(),
                "rule": "existing_braking_envelope_activates_touchdown_rescue",
                "nominal_upward_acceleration_insufficient": true,
                "standalone_coast_terminal_changed": false,
                "ordinary_default_changed": false,
                "short_command_guard_changed": false,
            },
            "coast_terminal": {
                "enabled": experiment.coast_terminal(),
                "only_after_actual_handoff": true,
                "trigger": "blocked_destination_arc",
                "checkpoint": "first_crest_plus_half_body_diameter",
                "maximum_coast_ticks": coast_terminal::MAX_COAST_TICKS,
                "terminal_preview_ticks": CONTINUATION_TICKS,
                "terminal_setup": "standalone_defaults",
                "entry_query": "existing_dynamics_then_actual_feedback_preview",
                "configured_terminal_terrain_enabled": true,
                "complete_landing_suffix_required": false,
                "saved_clock_used": false,
            },
            "landing_countdown": {
                "enabled": experiment.landing_countdown(),
                "rule": "retain_first_selected_ballistic_fallback_arrival",
                "admissions_per_landing": 1,
                "remaining_time_floor_s": 0.5,
                "revalidate_each_update": true,
                "release_on_infeasible_or_expired": true,
                "target_convention_changed": false,
                "ordinary_default_changed": false,
            },
            "landing_duration": {
                "enabled": experiment.landing_duration(),
                "rule": "initial_vertical_balance_after_all_latest_safe_fits_fail",
                "additional_queries": 1,
                "burn_time_min_s": 3.0,
                "burn_time_max_s": 14.0,
                "shared_entry_and_live_controller": true,
                "ordinary_default_changed": false,
            },
            "local_waypoint": {
                "feature_local_height": experiment.local_height(),
                "negative_braking_room_destination_preview": experiment.early_target(),
                "preview_records_handoff": false,
                "destination_constructor_changed": false,
            },
            "waypoint_entry": {
                "least_added_thrust_effort": experiment.effort(),
                "forward_crossing_recovery": experiment.recovery(),
                "retain_safe_pending_coast": experiment.recovery(),
                "apex_constraint": false,
                "destination_constructor_changed": false,
            },
            "waypoint_clearance": {
                "repair": "fixed_x_body_envelope_height_lift",
                "continuation_ticks": CONTINUATION_TICKS,
                "numeric_height_margin_m": clearance::NUMERIC_HEIGHT_MARGIN_M,
                "direct_constructor_terrain_blind": true,
            },
            "ridge_placement": {
                "rule": "first_crest_body_diameter_drop",
                "valley_drop_vehicle_diameters": 1,
                "crest_offset_vehicle_diameters": 1,
                "unresolved_slope": "local_climb_stage",
                "landing_suffix_required": false,
            },
            "terrain_correction": {
                "maximum_commands": avoidance::MAX_COMMANDS,
                "maximum_episode_ticks": avoidance::MAX_RECOVERY_TICKS,
                "maximum_prediction_ticks": CONTINUATION_TICKS,
                "braking_tilt_rad": avoidance::BRAKING_TILT,
                "short_command_changes_goal": false,
            },
            "planner_mechanics": {
                "enabled": experiment.mechanics(),
                "waypoint_exit_consistency": experiment.exit_consistency(),
                "piecewise_early_target": experiment.piecewise_early_target(),
                "recovery_lead": experiment.recovery_lead(),
                "recovery_choices_recorded": experiment.mechanics(),
                "ordinary_default_changed": false,
                "physical_guards_changed": false,
            },
            "finite_correction": {
                "queries_recorded": experiment.finite_queries(),
                "admission_enabled": experiment.finite_correction(),
                "powered_prefix_plant_checked": experiment.finite_queries(),
                "optional_refresh_ticks": REFRESH_TICKS,
                "initial_nominal_changed": false,
                "waypoint_ranking_changed": false,
                "physical_guards_changed": false,
            },
            "phase_transition": transition_metadata,
            "queued_recovery": {
                "enabled": experiment.queued_recovery(),
                "bounded_comparison": experiment.recovery_consistency(),
                "goals_unchanged": true,
            },
        }),
    )?;
    let preflight = crate::preflight_waypoint_v2_flight(
        &request,
        &pd_plan::waypoint_v2::WaypointV2Policy::revision_3(),
    );
    crate::evidence_io::write_bytes_create_only_with_context(
        &output.join("scenario.json"),
        &scenario_bytes,
        "unchanged candidate input",
    )?;
    write_json_create_only(&output.join("preflight.json"), &preflight)?;
    ensure!(preflight.supported, "candidate input unsupported");
    let mut origin = new_ordinary(&ctx)?;
    let mut prefix_hash = None;
    let mut prefix_state = None;
    if let Some(path) = baseline_path {
        let bytes = fs::read(path)?;
        let baseline: WaypointV2FlightResult = serde_json::from_slice(&bytes)?;
        ensure!(
            baseline.integrity_passed && baseline.final_source_replay_passed,
            "unverified prefix source"
        );
        ensure!(
            baseline.input_identity
                == crate::nominal_direct_flight_identity(&(&request, &baseline.policy))?,
            "retained prefix input binding differs"
        );
        let expected = &baseline
            .cycles
            .get(cycle_index)
            .context("missing retained cycle")?
            .current_state;
        for segment in baseline
            .segments
            .iter()
            .filter(|v| v.end_physics_step <= expected.physics_step)
        {
            ensure!(
                segment.entry_state == origin.evidence.final_state,
                "retained prefix discontinuity"
            );
            advance_ordinary(
                &ctx,
                &mut origin,
                &segment.updates,
                segment.end_physics_step,
            )?;
            ensure!(
                segment.end_state == origin.evidence.final_state,
                "retained prefix state mismatch"
            );
        }
        ensure!(
            origin.evidence.final_state == *expected,
            "retained origin mismatch"
        );
        prefix_hash = Some(sha256_bytes(&bytes)?);
        prefix_state = Some(expected.clone());
    }
    write_json_create_only(&output.join("prefix-proof.json"), &origin.evidence)?;
    let Execution {
        live,
        refreshes,
        handoffs,
        handoff_goal_revisions,
        updates,
        stop,
        terrain_domain_stop,
        controller_updates,
    } = execute(
        &request,
        origin.clone(),
        neutral,
        correction_cap,
        experiment,
    )?;
    write_json_create_only(
        &output.join("executed-raw.json"),
        &serde_json::json!({
            "ordinary_flight": live.evidence, "refreshes": refreshes,
            "handoffs": handoffs, "updates": updates, "stop": stop,
            "handoff_goal_revisions": handoff_goal_revisions,
            "terrain_domain_stop": terrain_domain_stop,
            "controller_updates": controller_updates,
            "verification_pending": true,
        }),
    )?;
    let mut replay = origin.clone();
    for update in &updates {
        advance_one(&ctx, &mut replay, update, neutral)?;
    }
    write_json_create_only(
        &output.join("command-replay.json"),
        &serde_json::json!({
            "passed": replay.evidence == live.evidence,
            "final_state": replay.evidence.final_state,
        }),
    )?;
    ensure!(
        replay.evidence == live.evidence,
        "candidate exact command source replay differs"
    );
    let repeat = execute(&request, origin, neutral, correction_cap, experiment)?;
    ensure!(
        repeat.live.evidence == live.evidence
            && repeat.refreshes == refreshes
            && repeat.handoffs == handoffs
            && repeat.handoff_goal_revisions == handoff_goal_revisions
            && repeat.updates == updates
            && repeat.stop == stop
            && repeat.terrain_domain_stop == terrain_domain_stop,
        "candidate feedback decisions not deterministic"
    );
    ensure!(
        repeat.controller_updates == controller_updates,
        "candidate feedback decisions not deterministic"
    );
    let final_source = crate::current_workspace_state()?;
    let final_content = crate::waypoint_v2_pack::capture_source_state(&crate::repo_root())?;
    ensure!(
        final_source.workspace_key == source_state.workspace_key
            && final_content.rust_source_tree_sha256 == content_source.rust_source_tree_sha256
            && sha256_bytes(&fs::read(
                crate::repo_root().join("pd-report/src/planning_cycles.js")
            )?)? == report_script_sha256
            && sha256_bytes(&fs::read(std::env::current_exe()?)?)? == binary_sha256,
        "candidate source contents or binary drift during attempt"
    );
    let result = FeedbackResult {
        candidate_id: experiment.candidate_id().into(),
        scenario_sha256: sha256_bytes(&scenario_bytes)?,
        prefix_flight_sha256: prefix_hash,
        prefix_origin: prefix_state,
        terrain_neutral_diagnostic: neutral,
        reason: stop.clone(),
        stop,
        refreshes,
        handoffs,
        handoff_goal_revisions,
        updates,
        final_state: live.evidence.final_state.clone(),
        integrity_passed: true,
        source_replay_passed: true,
        decisions_reproduced: true,
        ordinary_flight: live.evidence,
        terrain_domain_stop,
        controller_updates,
    };
    write_json_create_only(&output.join("feedback.json"), &result)?;
    report::write(&scenario, &result, &live.state, output)?;
    Ok(result)
}

#[cfg(test)]
mod tests;
