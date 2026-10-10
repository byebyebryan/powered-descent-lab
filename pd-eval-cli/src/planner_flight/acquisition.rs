//! Current planner acquisition; no research orchestration.
use super::*;
use anyhow::{Context, Result, bail};
use pd_core::{
    Command, FlightProgramUpdateV1, RunContext, SimulationState, SimulationStateSnapshotV1, Vec2,
};
use pd_plan::ballistic::KinematicStateV2;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod runtime;
mod terminal_time;

pub(super) const HELD_TICKS: u64 = 2;

pub(super) const MAX_TURN_UPDATES: usize = 3;

pub(super) const MAX_SEEDS: usize = 13;

const MAX_ENTRY_SCREENS: usize = 4;

pub(super) const MAX_PHYSICAL_WITNESSES: usize = 3;

pub(super) const TERMINAL_EXTRA_TICKS: u64 = 4;

pub(crate) const MAX_THRUST_FRACTION: f64 = 0.925;

const FLOAT_COMPARE_TOLERANCE: f64 = 1.0e-9;

pub(super) const TARGET_ENTRY_RESERVE_M: f64 = 5.0;

pub(super) const ENTRY_COAST_FRACTIONS: [f64; MAX_ENTRY_SCREENS] = [0.0, 0.25, 0.5, 0.75];

pub(super) const ACQUISITION_BURN_FRACTIONS: [f64; 3] = [0.25, 0.45, 0.65];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMatchEvidenceV1 {
    pub matched: bool,
    pub comparison_rule: String,
    pub maximum_float_delta: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalSeedEvidenceV1 {
    pub seed_id: String,
    pub kind: String,
    pub preserve_vertical: bool,
    pub virtual_arrival_physics_ticks: u64,
    pub burn_fraction: f64,
    pub turn_physics_ticks: u64,
    pub burn_physics_ticks: u64,
    pub remaining_virtual_physics_ticks: u64,
    pub requested_thrust_acceleration_mps2: Option<Vec2>,
    pub predicted_acquisition_end: Option<KinematicStateV2>,
    pub virtual_target_error_m: Option<f64>,
    pub estimated_acquisition_fuel_kg: Option<f64>,
    pub upward_impulse_mps: Option<f64>,
    pub turn_updates: usize,
    pub status: String,
    pub reason: Option<String>,
    pub entry_screens: Vec<NominalEntryScreenEvidenceV1>,
    pub selected_entry_index: Option<usize>,
    pub estimated_fuel_kg: Option<f64>,
    pub predicted_finish_physics_step: Option<u64>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalEntryScreenEvidenceV1 {
    pub entry_index: usize,
    pub coast_fraction: f64,
    pub coast_physics_ticks: u64,
    pub predicted_entry: KinematicStateV2,
    pub predicted_entry_angle_rad: f64,
    pub terminal_time_s: Option<f64>,
    pub terminal_physics_ticks: Option<u64>,
    pub coupled_thrust_bound_mps2: Option<f64>,
    pub terminal_fuel_kg: Option<f64>,
    pub total_predicted_fuel_kg: Option<f64>,
    pub predicted_finish_physics_step: Option<u64>,
    pub lateral_velocity_reversal_checked: bool,
    pub admissible: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalWitnessEvidenceV1 {
    pub seed_id: String,
    pub entry_index: usize,
    pub status: String,
    pub reason: Option<String>,
    pub acquisition_start: SimulationStateSnapshotV1,
    pub predicted_acquisition_end: KinematicStateV2,
    pub actual_acquisition_end: SimulationStateSnapshotV1,
    pub acquisition_position_error_m: f64,
    pub acquisition_velocity_error_mps: f64,
    pub acquisition_fuel_error_kg: f64,
    pub acquisition_turn_command_identity: String,
    pub acquisition_command_identity: String,
    pub actual_terminal_entry: Option<SimulationStateSnapshotV1>,
    pub actual_terminal_entry_angle_rad: Option<f64>,
    pub actual_entry_position_error_m: Option<f64>,
    pub actual_entry_velocity_error_mps: Option<f64>,
    pub commands: Vec<FlightProgramUpdateV1>,
    pub command_identity: String,
    pub independent_replay_passed: bool,
    pub endpoint_state_agreement: bool,
    pub target_plane_witness: Option<TargetPlaneWitnessEvidenceV1>,
    pub terrain_audit: Option<RealTerrainAuditEvidenceV1>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TargetPlaneWitnessEvidenceV1 {
    pub physics_step: u64,
    pub safe_by_existing_target_plane_mirror: bool,
    pub proposal_endpoint_contact_match: bool,
    pub free_space_endpoint_match: bool,
    pub actual_landing_claimed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealTerrainAuditEvidenceV1 {
    pub ordinary_neutral_prefix_parity: bool,
    pub first_contact: Option<TerminalContactAuditEvidence>,
    pub first_contact_physics_step: Option<u64>,
    pub final_state: SimulationStateSnapshotV1,
    pub stopped_on_contact: bool,
    pub landed_on_target: bool,
    pub target_plane_witness_reached_before_terrain_contact: bool,
    pub terrain_contact_blocks_landing_claim: bool,
    pub minimum_hull_clearance_m: Option<f64>,
    pub reserve_phases: Vec<BodyReservePhaseEvidenceV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyReservePhaseEvidenceV1 {
    pub phase: String,
    pub minimum_reserve_margin_m: Option<f64>,
    pub first_reserve_violation_physics_step: Option<u64>,
    pub first_query_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum WitnessMaterializationFailure {
    FiniteMiss(String),
    Integrity(String),
}

fn validate_update(update: &FlightProgramUpdateV1) -> Result<()> {
    if update.phase.trim().is_empty()
        || !update.command.throttle_frac.is_finite()
        || !(0.0..=1.0).contains(&update.command.throttle_frac)
        || !update.command.target_attitude_rad.is_finite()
        || !(-std::f64::consts::PI..=std::f64::consts::PI)
            .contains(&update.command.target_attitude_rad)
    {
        bail!("command or phase is invalid");
    }
    Ok(())
}

fn compare_full_snapshot(
    actual: &SimulationStateSnapshotV1,
    expected: &SimulationStateSnapshotV1,
) -> SnapshotMatchEvidenceV1 {
    if actual == expected {
        return SnapshotMatchEvidenceV1 {
            matched: true,
            comparison_rule: "exact_full_snapshot".into(),
            maximum_float_delta: 0.0,
        };
    }
    let discrete_equal = actual.sim_time_s == expected.sim_time_s
        && actual.physics_step == expected.physics_step
        && actual.held_command == expected.held_command
        && actual.physical_outcome == expected.physical_outcome
        && actual.mission_outcome == expected.mission_outcome
        && actual.end_reason == expected.end_reason
        && actual.waypoint_sequence_passed == expected.waypoint_sequence_passed
        && actual.waypoint_sequence_first_failure_index
            == expected.waypoint_sequence_first_failure_index
        && actual.waypoint_handoff_window_index == expected.waypoint_handoff_window_index;
    let deltas = [
        (actual.position_m.x, expected.position_m.x),
        (actual.position_m.y, expected.position_m.y),
        (actual.velocity_mps.x, expected.velocity_mps.x),
        (actual.velocity_mps.y, expected.velocity_mps.y),
        (actual.attitude_rad, expected.attitude_rad),
        (actual.angular_rate_radps, expected.angular_rate_radps),
        (actual.fuel_kg, expected.fuel_kg),
        (
            actual.min_touchdown_clearance_m,
            expected.min_touchdown_clearance_m,
        ),
        (actual.min_hull_clearance_m, expected.min_hull_clearance_m),
        (actual.max_speed_mps, expected.max_speed_mps),
        (actual.max_abs_attitude_rad, expected.max_abs_attitude_rad),
        (
            actual.max_abs_angular_rate_radps,
            expected.max_abs_angular_rate_radps,
        ),
    ];
    let maximum_float_delta = deltas
        .iter()
        .map(|(left, right)| (left - right).abs())
        .fold(0.0_f64, f64::max);
    let matched = discrete_equal
        && deltas.iter().all(|(left, right)| {
            left.is_finite() && right.is_finite() && (left - right).abs() <= FLOAT_COMPARE_TOLERANCE
        });
    SnapshotMatchEvidenceV1 {
        matched,
        comparison_rule: if matched {
            "exact_clocks_enums_commands_and_float_tolerance_le_1e-9".into()
        } else {
            "full_snapshot_mismatch".into()
        },
        maximum_float_delta,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SeedSpec {
    seed_id: String,
    kind: String,
    virtual_arrival_ticks: u64,
    burn_fraction: f64,
    natural_profile: bool,
    zero_acquisition: bool,
}

#[derive(Clone, Debug)]
pub(super) struct AcquisitionPlan {
    thrust_acceleration_mps2: Vec2,
    turn_ticks: u64,
    burn_ticks: u64,
    predicted_end: KinematicStateV2,
    virtual_target_error_m: f64,
    estimated_fuel_kg: f64,
    upward_impulse_mps: f64,
    turn_updates: usize,
    target_attitude_rad: f64,
}

pub(super) fn generate_seed_specs(
    context: &RunContext,
    live: &SimulationState,
) -> Result<Vec<SeedSpec>> {
    let target_com_y =
        context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m;
    let height = live.position_m.y - target_com_y;
    let gravity = context.world.gravity_mps2;
    let discriminant = live.velocity_mps.y.powi(2) + 2.0 * gravity * height;
    if !height.is_finite() || !discriminant.is_finite() || !gravity.is_finite() || gravity <= 0.0 {
        return Ok(Vec::new());
    }
    let natural_time = if discriminant >= 0.0 {
        (live.velocity_mps.y + discriminant.sqrt()) / gravity
    } else {
        f64::NAN
    };
    let natural_time = (natural_time.is_finite() && natural_time > 0.0).then_some(natural_time);
    let dt = context.sim.physics_dt_s();
    let lateral_distance = (context.target_pad.center_x_m - live.position_m.x).abs();
    let lateral_time = (2.0 * lateral_distance / gravity).sqrt();
    let baseline_time = natural_time.unwrap_or(0.0).max(lateral_time);
    if !baseline_time.is_finite() || baseline_time <= 0.0 {
        return Ok(Vec::new());
    }
    let baseline_ticks = even_ticks_ceil(baseline_time, dt)?;
    let zero_ticks = match natural_time {
        Some(time) => even_ticks_ceil(time, dt)?,
        None => baseline_ticks,
    };
    let mut arrivals = BTreeMap::<u64, bool>::new();
    if let Some(natural_time) = natural_time {
        let ticks = even_ticks_ceil(natural_time, dt)?;
        arrivals.insert(ticks, true);
    }
    arrivals
        .entry(baseline_ticks)
        .and_modify(|existing| *existing |= natural_time == Some(baseline_time))
        .or_insert(natural_time == Some(baseline_time));
    for multiplier in [1.25, 1.5] {
        let ticks = even_ticks_ceil(baseline_time * multiplier, dt)?;
        arrivals.entry(ticks).or_insert(false);
    }
    let mut seeds = vec![SeedSpec {
        seed_id: format!("zero_acquisition_n{zero_ticks}"),
        kind: "zero_acquisition".into(),
        virtual_arrival_ticks: zero_ticks,
        burn_fraction: 0.0,
        natural_profile: true,
        zero_acquisition: true,
    }];
    for (arrival_ticks, natural_profile) in arrivals {
        for burn_fraction in ACQUISITION_BURN_FRACTIONS {
            let fraction_code = (burn_fraction * 100.0).round() as u32;
            let kind = if natural_profile {
                "natural_profile"
            } else {
                "upward_shaping"
            };
            seeds.push(SeedSpec {
                seed_id: format!("{kind}_n{arrival_ticks}_b{fraction_code}"),
                kind: kind.into(),
                virtual_arrival_ticks: arrival_ticks,
                burn_fraction,
                natural_profile,
                zero_acquisition: false,
            });
        }
    }
    if seeds.len() > MAX_SEEDS {
        bail!(
            "bounded seed construction exceeded its proven 1 + 4-by-3 budget: {}",
            seeds.len()
        );
    }
    Ok(seeds)
}

fn even_ticks_ceil(seconds: f64, dt: f64) -> Result<u64> {
    if !seconds.is_finite() || seconds <= 0.0 || !dt.is_finite() || dt <= 0.0 {
        bail!("non-positive or non-finite duration");
    }
    let pairs = (seconds / (HELD_TICKS as f64 * dt)).ceil();
    if !pairs.is_finite() || pairs > (u64::MAX / HELD_TICKS) as f64 {
        bail!("duration exceeds representable held-pair count");
    }
    Ok((pairs as u64).saturating_mul(HELD_TICKS).max(HELD_TICKS))
}

fn even_ticks_nearest_fraction(ticks: u64, fraction: f64) -> u64 {
    let pairs = ((ticks as f64 * fraction) / HELD_TICKS as f64).round() as u64;
    pairs.saturating_mul(HELD_TICKS).min(ticks)
}

fn solve_constant_acquisition(
    incoming: KinematicStateV2,
    target: Vec2,
    arrival_ticks: u64,
    turn_ticks: u64,
    burn_ticks: u64,
    dt: f64,
    gravity_mps2: f64,
) -> Result<Vec2> {
    if arrival_ticks == 0 || burn_ticks == 0 || turn_ticks + burn_ticks > arrival_ticks {
        bail!("acquisition has no positive burn gain before virtual arrival");
    }
    let n = arrival_ticks as f64;
    let turn = turn_ticks as f64;
    let burn = burn_ticks as f64;
    let ballistic_end = incoming.position_m
        + incoming.velocity_mps * (n * dt)
        + Vec2::new(0.0, -gravity_mps2) * (dt * dt * n * (n + 1.0) / 2.0);
    let gain = dt * dt * burn * (n - turn - (burn - 1.0) / 2.0);
    if !gain.is_finite() || gain <= 0.0 {
        bail!("acquisition constant-acceleration gain is not positive");
    }
    let acceleration = (target - ballistic_end) * (1.0 / gain);
    if !acceleration.x.is_finite() || !acceleration.y.is_finite() {
        bail!("acquisition constant-acceleration demand is non-finite");
    }
    Ok(acceleration)
}

fn acquisition_end_prediction(
    incoming: KinematicStateV2,
    turn_ticks: u64,
    burn_ticks: u64,
    thrust_acceleration_mps2: Vec2,
    dt: f64,
    gravity_mps2: f64,
) -> KinematicStateV2 {
    let total = (turn_ticks + burn_ticks) as f64;
    let burn = burn_ticks as f64;
    KinematicStateV2 {
        position_m: incoming.position_m
            + incoming.velocity_mps * (total * dt)
            + Vec2::new(0.0, -gravity_mps2) * (dt * dt * total * (total + 1.0) / 2.0)
            + thrust_acceleration_mps2 * (dt * dt * burn * (burn + 1.0) / 2.0),
        velocity_mps: incoming.velocity_mps
            + Vec2::new(0.0, -gravity_mps2) * (total * dt)
            + thrust_acceleration_mps2 * (burn * dt),
    }
}

fn virtual_arrival_prediction(
    incoming: KinematicStateV2,
    arrival_ticks: u64,
    turn_ticks: u64,
    burn_ticks: u64,
    thrust_acceleration_mps2: Vec2,
    dt: f64,
    gravity_mps2: f64,
) -> KinematicStateV2 {
    let n = arrival_ticks as f64;
    let burn = burn_ticks as f64;
    let turn = turn_ticks as f64;
    let gain = dt * dt * burn * (n - turn - (burn - 1.0) / 2.0);
    KinematicStateV2 {
        position_m: incoming.position_m
            + incoming.velocity_mps * (n * dt)
            + Vec2::new(0.0, -gravity_mps2) * (dt * dt * n * (n + 1.0) / 2.0)
            + thrust_acceleration_mps2 * gain,
        velocity_mps: incoming.velocity_mps
            + Vec2::new(0.0, -gravity_mps2) * (n * dt)
            + thrust_acceleration_mps2 * (burn * dt),
    }
}

fn coast_prediction(
    start: KinematicStateV2,
    coast_ticks: u64,
    dt: f64,
    gravity_mps2: f64,
) -> KinematicStateV2 {
    let ticks = coast_ticks as f64;
    KinematicStateV2 {
        position_m: start.position_m
            + start.velocity_mps * (ticks * dt)
            + Vec2::new(0.0, -gravity_mps2) * (dt * dt * ticks * (ticks + 1.0) / 2.0),
        velocity_mps: start.velocity_mps + Vec2::new(0.0, -gravity_mps2) * (ticks * dt),
    }
}

fn target_kinematics(_context: &RunContext, live: &SimulationState) -> KinematicStateV2 {
    KinematicStateV2 {
        position_m: live.position_m,
        velocity_mps: live.velocity_mps,
    }
}

pub(crate) fn turn_ticks_for(
    context: &RunContext,
    current_attitude_rad: f64,
    target_acceleration_mps2: Vec2,
) -> Result<(u64, f64)> {
    if !current_attitude_rad.is_finite()
        || !target_acceleration_mps2.x.is_finite()
        || !target_acceleration_mps2.y.is_finite()
    {
        bail!("turn estimate contains non-finite state");
    }
    let target_attitude_rad = if target_acceleration_mps2.length() <= 1.0e-12 {
        current_attitude_rad
    } else {
        target_acceleration_mps2.x.atan2(target_acceleration_mps2.y)
    };
    let delta = shortest_angle_delta(current_attitude_rad, target_attitude_rad).abs();
    let max_angle_per_tick = context.vehicle.max_rotation_rate_radps * context.sim.physics_dt_s();
    if !max_angle_per_tick.is_finite() || max_angle_per_tick <= 0.0 {
        bail!("turn estimate has invalid vehicle rotation rate");
    }
    let raw_ticks = (delta / max_angle_per_tick).ceil();
    if !raw_ticks.is_finite() || raw_ticks > (u64::MAX - 1) as f64 {
        bail!("turn duration exceeds representable physics ticks");
    }
    let ticks = (raw_ticks as u64).div_ceil(HELD_TICKS) * HELD_TICKS;
    Ok((ticks, target_attitude_rad))
}

fn estimate_acquisition_fuel(
    context: &RunContext,
    incoming_fuel_kg: f64,
    acceleration_mps2: Vec2,
    burn_ticks: u64,
) -> Result<f64> {
    if burn_ticks == 0 || !burn_ticks.is_multiple_of(HELD_TICKS) {
        bail!("acquisition burn is not a complete held-command duration");
    }
    let policy = BodyAwareTerminalPolicyV1::default();
    let mut fuel = incoming_fuel_kg;
    for _ in 0..burn_ticks / HELD_TICKS {
        let mass = context.vehicle.dry_mass_kg + fuel;
        let command_throttle = paired_throttle(context, &policy, mass, acceleration_mps2.length())?;
        let applied = applied_throttle(command_throttle, context.vehicle.min_throttle_frac);
        let used = applied
            * context.vehicle.max_fuel_burn_kgps
            * context.sim.physics_dt_s()
            * HELD_TICKS as f64;
        if !used.is_finite() || used < 0.0 || fuel - used <= 0.0 {
            bail!("acquisition burn exceeds available fuel");
        }
        fuel -= used;
    }
    Ok(incoming_fuel_kg - fuel)
}

fn applied_throttle(command_throttle: f64, minimum: f64) -> f64 {
    if command_throttle <= 0.0 {
        0.0
    } else {
        minimum + command_throttle.clamp(0.0, 1.0) * (1.0 - minimum)
    }
}

fn make_acquisition_plan(
    context: &RunContext,
    live: &SimulationState,
    seed: &SeedSpec,
) -> Result<AcquisitionPlan> {
    if seed.zero_acquisition {
        let target = Vec2::new(
            context.target_pad.center_x_m,
            context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m,
        );
        let arrival = virtual_arrival_prediction(
            target_kinematics(context, live),
            seed.virtual_arrival_ticks,
            0,
            0,
            Vec2::new(0.0, 0.0),
            context.sim.physics_dt_s(),
            context.world.gravity_mps2,
        );
        return Ok(AcquisitionPlan {
            thrust_acceleration_mps2: Vec2::new(0.0, 0.0),
            turn_ticks: 0,
            burn_ticks: 0,
            predicted_end: target_kinematics(context, live),
            virtual_target_error_m: (arrival.position_m - target).length(),
            estimated_fuel_kg: 0.0,
            upward_impulse_mps: 0.0,
            turn_updates: 0,
            target_attitude_rad: live.attitude_rad,
        });
    }
    let dt = context.sim.physics_dt_s();
    let burn_ticks = even_ticks_nearest_fraction(seed.virtual_arrival_ticks, seed.burn_fraction);
    if burn_ticks == 0 {
        bail!("acquisition burn rounded to zero held pairs");
    }
    let target = Vec2::new(
        context.target_pad.center_x_m,
        context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m,
    );
    let incoming = target_kinematics(context, live);
    let mut turn_ticks = 0_u64;
    let mut updates = 0_usize;
    let mut solved = Vec2::new(0.0, 0.0);
    let mut target_attitude_rad = live.attitude_rad;
    let mut converged = false;
    for _ in 0..MAX_TURN_UPDATES {
        if turn_ticks + burn_ticks > seed.virtual_arrival_ticks {
            bail!("turn plus burn exceeds virtual acquisition time");
        }
        solved = solve_constant_acquisition(
            incoming,
            target,
            seed.virtual_arrival_ticks,
            turn_ticks,
            burn_ticks,
            dt,
            context.world.gravity_mps2,
        )?;
        if seed.natural_profile {
            // Natural-profile timing preserves the vertical arc exactly; any
            // rounded target-height discrepancy remains visible in the result.
            solved.y = 0.0;
        } else if solved.y < -1.0e-10 {
            bail!("upward-only acquisition would require downward vertical thrust");
        }
        let (next_turn, angle) = turn_ticks_for(context, live.attitude_rad, solved)?;
        target_attitude_rad = angle;
        updates += 1;
        if next_turn == turn_ticks {
            converged = true;
            break;
        }
        turn_ticks = next_turn;
    }
    if !converged {
        bail!("coast-turn consistency did not converge within three updates");
    }
    if turn_ticks + burn_ticks > seed.virtual_arrival_ticks {
        bail!("turn plus burn exceeds virtual acquisition time");
    }
    let incoming_max_accel = context.vehicle.max_thrust_n / live.mass_kg(context).max(1.0);
    if solved.length() > MAX_THRUST_FRACTION * incoming_max_accel {
        bail!("acquisition demand exceeds 0.925 of incoming max-thrust acceleration");
    }
    let estimated_fuel_kg = estimate_acquisition_fuel(context, live.fuel_kg, solved, burn_ticks)?;
    let predicted_end = acquisition_end_prediction(
        incoming,
        turn_ticks,
        burn_ticks,
        solved,
        dt,
        context.world.gravity_mps2,
    );
    let predicted_arrival = virtual_arrival_prediction(
        incoming,
        seed.virtual_arrival_ticks,
        turn_ticks,
        burn_ticks,
        solved,
        dt,
        context.world.gravity_mps2,
    );
    let virtual_target_error_m = (predicted_arrival.position_m - target).length();
    let upward_impulse_mps = solved.y.max(0.0) * burn_ticks as f64 * dt;
    Ok(AcquisitionPlan {
        thrust_acceleration_mps2: solved,
        turn_ticks,
        burn_ticks,
        predicted_end,
        virtual_target_error_m,
        estimated_fuel_kg,
        upward_impulse_mps,
        turn_updates: updates,
        target_attitude_rad,
    })
}

pub(super) fn evaluate_seed(
    context: &RunContext,
    live: &SimulationState,
    deadline: u64,
    spec: SeedSpec,
) -> Result<NominalSeedEvidenceV1> {
    let mut evidence = NominalSeedEvidenceV1 {
        seed_id: spec.seed_id.clone(),
        kind: spec.kind.clone(),
        preserve_vertical: true,
        virtual_arrival_physics_ticks: spec.virtual_arrival_ticks,
        burn_fraction: spec.burn_fraction,
        turn_physics_ticks: 0,
        burn_physics_ticks: 0,
        remaining_virtual_physics_ticks: 0,
        requested_thrust_acceleration_mps2: None,
        predicted_acquisition_end: None,
        virtual_target_error_m: None,
        estimated_acquisition_fuel_kg: None,
        upward_impulse_mps: None,
        turn_updates: 0,
        status: "finite_miss".into(),
        reason: None,
        entry_screens: Vec::new(),
        selected_entry_index: None,
        estimated_fuel_kg: None,
        predicted_finish_physics_step: None,
        identity: String::new(),
    };
    let plan = match make_acquisition_plan(context, live, &spec) {
        Ok(plan) => plan,
        Err(error) => {
            evidence.reason = Some(format!("{error:#}"));
            if is_integrity_acquisition_plan_error(evidence.reason.as_deref().unwrap_or_default()) {
                evidence.status = "integrity_error".into();
            }
            evidence.identity = seed_identity(&evidence)?;
            return Ok(evidence);
        }
    };
    evidence.preserve_vertical = plan.thrust_acceleration_mps2.y.abs() <= 1.0e-12;
    evidence.turn_physics_ticks = plan.turn_ticks;
    evidence.burn_physics_ticks = plan.burn_ticks;
    evidence.remaining_virtual_physics_ticks = spec
        .virtual_arrival_ticks
        .saturating_sub(plan.turn_ticks + plan.burn_ticks);
    evidence.requested_thrust_acceleration_mps2 = Some(plan.thrust_acceleration_mps2);
    evidence.predicted_acquisition_end = Some(plan.predicted_end);
    evidence.virtual_target_error_m = Some(plan.virtual_target_error_m);
    evidence.estimated_acquisition_fuel_kg = Some(plan.estimated_fuel_kg);
    evidence.upward_impulse_mps = Some(plan.upward_impulse_mps);
    evidence.turn_updates = plan.turn_updates;
    let forward_direction = forward_direction(context, live.position_m.x, live.velocity_mps.x);
    if forward_direction * plan.predicted_end.velocity_mps.x < -1.0e-9 {
        evidence.reason = Some("predicted acquisition end has negative forward velocity".into());
        evidence.identity = seed_identity(&evidence)?;
        return Ok(evidence);
    }
    let mut screens = Vec::new();
    for (index, fraction) in ENTRY_COAST_FRACTIONS.iter().copied().enumerate() {
        let coast_ticks =
            even_ticks_nearest_fraction(evidence.remaining_virtual_physics_ticks, fraction);
        if screens
            .iter()
            .any(|screen: &NominalEntryScreenEvidenceV1| screen.coast_physics_ticks == coast_ticks)
        {
            continue;
        }
        screens.push(screen_entry(
            context,
            live,
            deadline,
            &plan,
            index,
            fraction,
            coast_ticks,
        )?);
    }
    if screens.len() > MAX_ENTRY_SCREENS {
        bail!("entry screen budget exceeded for {}", spec.seed_id);
    }
    let selected = screens
        .iter()
        .enumerate()
        .filter(|(_, screen)| screen.admissible)
        .min_by(|(left_index, left), (right_index, right)| {
            left.total_predicted_fuel_kg
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.total_predicted_fuel_kg.unwrap_or(f64::INFINITY))
                .then_with(|| {
                    left.predicted_finish_physics_step
                        .unwrap_or(u64::MAX)
                        .cmp(&right.predicted_finish_physics_step.unwrap_or(u64::MAX))
                })
                .then_with(|| left_index.cmp(right_index))
        })
        .map(|(index, _)| index);
    evidence.selected_entry_index = selected;
    if let Some(index) = selected {
        let screen = &screens[index];
        evidence.status = "shortlisted_estimate".into();
        evidence.estimated_fuel_kg = screen.total_predicted_fuel_kg;
        evidence.predicted_finish_physics_step = screen.predicted_finish_physics_step;
    } else {
        evidence.reason = Some(
            screens
                .iter()
                .filter_map(|screen| screen.reason.as_deref())
                .take(4)
                .collect::<Vec<_>>()
                .join("; "),
        );
    }
    evidence.entry_screens = screens;
    evidence.identity = seed_identity(&evidence)?;
    Ok(evidence)
}

pub(super) fn seed_identity(seed: &NominalSeedEvidenceV1) -> Result<String> {
    let mut canonical = seed.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

pub(super) fn forward_direction(context: &RunContext, x: f64, velocity_x: f64) -> f64 {
    let delta = context.target_pad.center_x_m - x;
    if delta.abs() > 1.0e-9 {
        delta.signum()
    } else if velocity_x.abs() > 1.0e-9 {
        velocity_x.signum()
    } else {
        1.0
    }
}

fn screen_entry(
    context: &RunContext,
    live: &SimulationState,
    deadline: u64,
    plan: &AcquisitionPlan,
    entry_index: usize,
    fraction: f64,
    coast_ticks: u64,
) -> Result<NominalEntryScreenEvidenceV1> {
    let dt = context.sim.physics_dt_s();
    let entry = coast_prediction(
        plan.predicted_end,
        coast_ticks,
        dt,
        context.world.gravity_mps2,
    );
    let mut screen = NominalEntryScreenEvidenceV1 {
        entry_index,
        coast_fraction: fraction,
        coast_physics_ticks: coast_ticks,
        predicted_entry: entry,
        predicted_entry_angle_rad: (-entry.velocity_mps.y).atan2(entry.velocity_mps.x.abs()),
        terminal_time_s: None,
        terminal_physics_ticks: None,
        coupled_thrust_bound_mps2: None,
        terminal_fuel_kg: None,
        total_predicted_fuel_kg: None,
        predicted_finish_physics_step: None,
        lateral_velocity_reversal_checked: false,
        admissible: false,
        reason: None,
    };
    let target_com_y =
        context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m;
    let height = entry.position_m.y - target_com_y;
    let down_speed = -entry.velocity_mps.y;
    if !height.is_finite()
        || height <= TARGET_ENTRY_RESERVE_M
        || !entry.velocity_mps.y.is_finite()
        || entry.velocity_mps.y >= 0.0
    {
        screen.reason = Some("terminal entry is not descending above target COM + 5 m".into());
        return Ok(screen);
    }
    let target_down_speed = 0.5 * context.vehicle.safe_touchdown_normal_speed_mps;
    let denominator = down_speed + target_down_speed;
    if !denominator.is_finite() || denominator <= 0.0 {
        screen.reason = Some("terminal braking time has a non-positive denominator".into());
        return Ok(screen);
    }
    let terminal_time_s = (2.0 * height + (down_speed - target_down_speed) * dt) / denominator;
    if !terminal_time_s.is_finite() || terminal_time_s <= 0.0 {
        screen.reason = Some("protocol terminal braking time is not positive and finite".into());
        return Ok(screen);
    }
    let terminal_ticks = match even_ticks_ceil(terminal_time_s, dt) {
        Ok(ticks) => ticks,
        Err(error) => {
            screen.reason = Some(format!("{error:#}"));
            return Ok(screen);
        }
    };
    screen.terminal_time_s = Some(terminal_time_s);
    screen.terminal_physics_ticks = Some(terminal_ticks);
    let future_finish = live
        .physics_step
        .checked_add(plan.turn_ticks)
        .and_then(|tick| tick.checked_add(plan.burn_ticks))
        .and_then(|tick| tick.checked_add(coast_ticks))
        .and_then(|tick| tick.checked_add(terminal_ticks))
        .and_then(|tick| tick.checked_add(TERMINAL_EXTRA_TICKS));
    let Some(future_finish) = future_finish else {
        screen.reason = Some("predicted absolute finish clock overflow".into());
        return Ok(screen);
    };
    screen.predicted_finish_physics_step = Some(future_finish);
    if future_finish > deadline {
        screen.reason = Some("predicted terminal finish exceeds original absolute deadline".into());
        return Ok(screen);
    }

    let reference = match build_reference(
        context,
        &BodyAwareTerminalPolicyV1::default(),
        entry,
        terminal_ticks,
    ) {
        Ok(reference) => reference,
        Err(error) => return Err(error).context("terminal reference construction failed"),
    };
    if !terminal_forward_velocity_valid(context, &reference) {
        screen.lateral_velocity_reversal_checked = true;
        screen.reason = Some("terminal reference reverses forward horizontal velocity".into());
        return Ok(screen);
    }
    screen.lateral_velocity_reversal_checked = true;
    let bound = conservative_terminal_thrust_bound(context, &reference);
    screen.coupled_thrust_bound_mps2 = Some(bound);
    let estimated_terminal_start_fuel = (live.fuel_kg - plan.estimated_fuel_kg).max(0.0);
    let terminal_limit = MAX_THRUST_FRACTION * context.vehicle.max_thrust_n
        / (context.vehicle.dry_mass_kg + estimated_terminal_start_fuel).max(1.0);
    if !bound.is_finite() || bound > terminal_limit {
        screen.reason = Some(format!(
            "coupled terminal thrust bound {bound:.6} exceeds 0.925 incoming-mass limit {terminal_limit:.6}"
        ));
        return Ok(screen);
    }
    let first_terminal_thrust = paired_reference_thrust(context, &reference, 0);
    let first_angle = first_terminal_thrust.x.atan2(first_terminal_thrust.y);
    let acquisition_end_attitude = if plan.turn_ticks > 0 {
        plan.target_attitude_rad
    } else {
        live.attitude_rad
    };
    let align_ticks = match turn_ticks_for(
        context,
        acquisition_end_attitude,
        Vec2::new(first_angle.sin(), first_angle.cos()),
    ) {
        Ok((ticks, _)) => ticks,
        Err(error) => {
            screen.reason = Some(format!("terminal first-thrust alignment: {error:#}"));
            return Ok(screen);
        }
    };
    if coast_ticks < align_ticks {
        screen.reason = Some(format!(
            "terminal entry has {coast_ticks} coasting ticks for {align_ticks} ticks of first-thrust alignment"
        ));
        return Ok(screen);
    }
    let terminal_start_fuel = live.fuel_kg - plan.estimated_fuel_kg;
    let terminal_fuel = match estimate_terminal_fuel(context, &reference, terminal_start_fuel) {
        Ok(fuel) => fuel,
        Err(error) => {
            screen.reason = Some(format!("terminal throttle/fuel screen: {error:#}"));
            return Ok(screen);
        }
    };
    screen.terminal_fuel_kg = Some(terminal_fuel);
    screen.total_predicted_fuel_kg = Some(plan.estimated_fuel_kg + terminal_fuel);
    screen.admissible = true;
    Ok(screen)
}

pub(super) fn paired_reference_thrust(
    context: &RunContext,
    reference: &BodyAwareTerminalReferenceV1,
    tick: u64,
) -> Vec2 {
    (reference.thrust(context, tick) + reference.thrust(context, tick + 1)) * 0.5
}

pub(super) fn conservative_terminal_thrust_bound(
    context: &RunContext,
    reference: &BodyAwareTerminalReferenceV1,
) -> f64 {
    let [c0, c1, c2] = reference.horizontal_coefficients_mps2;
    let mut horizontal_samples = vec![c0, c0 + c1 + c2];
    if c2.abs() > 1.0e-14 {
        let vertex = -c1 / (2.0 * c2);
        if (0.0..=1.0).contains(&vertex) {
            horizontal_samples.push(c0 + c1 * vertex + c2 * vertex * vertex);
        }
    }
    let maximum_horizontal = horizontal_samples
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let first_vertical = reference.initial_vertical_acceleration_mps2 + context.world.gravity_mps2;
    let last_vertical = reference.initial_vertical_acceleration_mps2
        + reference.vertical_acceleration_delta_mps2 * (reference.physics_ticks - 1) as f64
        + context.world.gravity_mps2;
    maximum_horizontal.hypot(first_vertical.abs().max(last_vertical.abs()))
}

pub(super) fn terminal_forward_velocity_valid(
    context: &RunContext,
    reference: &BodyAwareTerminalReferenceV1,
) -> bool {
    let direction = forward_direction(
        context,
        reference.start.position_m.x,
        reference.start.velocity_mps.x,
    );
    let [c0, c1, c2] = reference.horizontal_coefficients_mps2;
    let mut sample_ticks = BTreeSet::from([0_u64, reference.physics_ticks]);
    for root in quadratic_roots(c2, c1, c0) {
        if (0.0..=1.0).contains(&root) {
            let tick = (root * (reference.physics_ticks - 1) as f64).round() as i64;
            for delta in -2_i64..=2 {
                let sample = tick + delta;
                if sample >= 0 && sample <= reference.physics_ticks as i64 {
                    sample_ticks.insert(sample as u64);
                }
            }
        }
    }
    sample_ticks.into_iter().all(|tick| {
        let state = reference.state_at(context, tick);
        state.velocity_mps.x.is_finite() && direction * state.velocity_mps.x >= -1.0e-9
    })
}

fn quadratic_roots(a: f64, b: f64, c: f64) -> Vec<f64> {
    if a.abs() <= 1.0e-14 {
        if b.abs() <= 1.0e-14 {
            Vec::new()
        } else {
            vec![-c / b]
        }
    } else {
        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 || !discriminant.is_finite() {
            Vec::new()
        } else {
            let root = discriminant.sqrt();
            vec![(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)]
        }
    }
}

pub(super) fn estimate_terminal_fuel(
    context: &RunContext,
    reference: &BodyAwareTerminalReferenceV1,
    initial_fuel_kg: f64,
) -> Result<f64> {
    if !initial_fuel_kg.is_finite() || initial_fuel_kg <= 0.0 {
        bail!("terminal starts without positive fuel");
    }
    let policy = BodyAwareTerminalPolicyV1::default();
    let mut fuel = initial_fuel_kg;
    let end_tick = reference.physics_ticks + TERMINAL_EXTRA_TICKS;
    for tick in (0..end_tick).step_by(HELD_TICKS as usize) {
        let thrust = paired_reference_thrust(context, reference, tick);
        let mass = context.vehicle.dry_mass_kg + fuel;
        let throttle = paired_throttle(context, &policy, mass, thrust.length())?;
        let applied = applied_throttle(throttle, context.vehicle.min_throttle_frac);
        let used = applied
            * context.vehicle.max_fuel_burn_kgps
            * context.sim.physics_dt_s()
            * HELD_TICKS as f64;
        if !used.is_finite() || used < 0.0 || fuel - used <= 0.0 {
            bail!("terminal reference exhausts estimated fuel");
        }
        fuel -= used;
    }
    Ok(initial_fuel_kg - fuel)
}

pub(super) fn compare_rank(
    left_seed: &NominalSeedEvidenceV1,
    left_entry_index: usize,
    right_seed: &NominalSeedEvidenceV1,
    right_entry_index: usize,
) -> std::cmp::Ordering {
    let left = &left_seed.entry_screens[left_entry_index];
    let right = &right_seed.entry_screens[right_entry_index];
    left_seed
        .preserve_vertical
        .cmp(&right_seed.preserve_vertical)
        .reverse()
        .then_with(|| {
            left_seed
                .upward_impulse_mps
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right_seed.upward_impulse_mps.unwrap_or(f64::INFINITY))
        })
        .then_with(|| {
            left.total_predicted_fuel_kg
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.total_predicted_fuel_kg.unwrap_or(f64::INFINITY))
        })
        .then_with(|| {
            left.predicted_finish_physics_step
                .unwrap_or(u64::MAX)
                .cmp(&right.predicted_finish_physics_step.unwrap_or(u64::MAX))
        })
        .then_with(|| left_seed.seed_id.cmp(&right_seed.seed_id))
        .then_with(|| left_entry_index.cmp(&right_entry_index))
}

#[derive(Clone, Debug)]
struct AcquisitionExecution {
    end_state: SimulationState,
    turn_updates: Vec<FlightProgramUpdateV1>,
    burn_updates: Vec<FlightProgramUpdateV1>,
}

struct NeutralReplay {
    final_state: SimulationState,
    acquisition_end_match: bool,
    terminal_entry: Option<SimulationStateSnapshotV1>,
    target_plane: Option<(u64, bool)>,
    peak_com_height_m: f64,
}

/// Shared command realization has no ordinary terrain audit or corpus inputs.
/// The maintained acquisition runtime audits the realized commands separately.
struct FreeSpaceWitness {
    witness: NominalWitnessEvidenceV1,
    end_state: AirborneFlightStateV1,
    peak_com_height_m: f64,
}

fn realize_free_space_witness(
    context: &RunContext,
    incoming: &SimulationState,
    deadline: u64,
    seed: &NominalSeedEvidenceV1,
    entry: &NominalEntryScreenEvidenceV1,
) -> std::result::Result<FreeSpaceWitness, WitnessMaterializationFailure> {
    let spec = SeedSpec {
        seed_id: seed.seed_id.clone(),
        kind: seed.kind.clone(),
        virtual_arrival_ticks: seed.virtual_arrival_physics_ticks,
        burn_fraction: seed.burn_fraction,
        natural_profile: seed.kind == "natural_profile",
        zero_acquisition: seed.kind == "zero_acquisition",
    };
    let plan = make_acquisition_plan(context, incoming, &spec).map_err(|error| {
        WitnessMaterializationFailure::Integrity(format!(
            "shortlisted seed no longer reconstructs its acquisition estimate: {error:#}"
        ))
    })?;
    if seed.requested_thrust_acceleration_mps2 != Some(plan.thrust_acceleration_mps2)
        || seed.predicted_acquisition_end != Some(plan.predicted_end)
        || seed.turn_physics_ticks != plan.turn_ticks
        || seed.burn_physics_ticks != plan.burn_ticks
    {
        return Err(WitnessMaterializationFailure::Integrity(
            "shortlist estimate does not bind to its recomputed acquisition plan".into(),
        ));
    }
    let acquisition = run_acquisition(context, incoming, &plan)?;
    let acquisition_start = SimulationStateSnapshotV1::from_state(incoming);
    let actual_acquisition_end = SimulationStateSnapshotV1::from_state(&acquisition.end_state);
    let acquisition_position_error_m = distance(
        plan.predicted_end.position_m,
        acquisition.end_state.position_m,
    );
    let acquisition_velocity_error_mps = distance(
        plan.predicted_end.velocity_mps,
        acquisition.end_state.velocity_mps,
    );
    let acquisition_fuel_error_kg =
        acquisition.end_state.fuel_kg - (incoming.fuel_kg - plan.estimated_fuel_kg);
    let turn_identity = stable_digest(&acquisition.turn_updates).map_err(|error| {
        WitnessMaterializationFailure::Integrity(format!(
            "hash acquisition turn commands: {error:#}"
        ))
    })?;
    let acquisition_identity = stable_digest(
        &acquisition
            .turn_updates
            .iter()
            .chain(&acquisition.burn_updates)
            .cloned()
            .collect::<Vec<_>>(),
    )
    .map_err(|error| {
        WitnessMaterializationFailure::Integrity(format!("hash acquisition commands: {error:#}"))
    })?;
    let acquisition_updates = acquisition
        .turn_updates
        .iter()
        .chain(&acquisition.burn_updates)
        .cloned()
        .collect::<Vec<_>>();
    let entry_ticks = entry.terminal_physics_ticks.ok_or_else(|| {
        WitnessMaterializationFailure::Integrity(
            "shortlisted entry is missing terminal duration".into(),
        )
    })?;
    let terminal_start_step = acquisition
        .end_state
        .physics_step
        .checked_add(entry.coast_physics_ticks)
        .ok_or_else(|| {
            WitnessMaterializationFailure::Integrity(
                "terminal-entry boundary overflows original clock".into(),
            )
        })?;

    let proposal = match super::airborne::candidate(
        context,
        &acquisition.end_state,
        entry.coast_physics_ticks,
        entry_ticks,
        deadline,
    ) {
        Ok(proposal) => proposal,
        Err(error) => {
            let candidate_error = match classify_candidate_failure(format!("{error:#}")) {
                WitnessMaterializationFailure::FiniteMiss(reason) => reason,
                failure @ WitnessMaterializationFailure::Integrity(_) => return Err(failure),
            };
            let mut witness = NominalWitnessEvidenceV1 {
                seed_id: seed.seed_id.clone(),
                entry_index: entry.entry_index,
                status: "finite_backend_rejection".into(),
                reason: Some(candidate_error),
                acquisition_start,
                predicted_acquisition_end: plan.predicted_end,
                actual_acquisition_end: actual_acquisition_end.clone(),
                acquisition_position_error_m,
                acquisition_velocity_error_mps,
                acquisition_fuel_error_kg,
                acquisition_turn_command_identity: turn_identity,
                acquisition_command_identity: acquisition_identity.clone(),
                actual_terminal_entry: None,
                actual_terminal_entry_angle_rad: None,
                actual_entry_position_error_m: None,
                actual_entry_velocity_error_mps: None,
                commands: acquisition_updates.clone(),
                command_identity: acquisition_identity,
                independent_replay_passed: false,
                endpoint_state_agreement: false,
                target_plane_witness: None,
                terrain_audit: None,
                identity: String::new(),
            };
            let replay = replay_neutral(
                context,
                incoming,
                &acquisition_updates,
                acquisition.end_state.physics_step,
                acquisition.end_state.physics_step,
                acquisition.end_state.physics_step,
                &actual_acquisition_end,
            )
            .map_err(|error| WitnessMaterializationFailure::Integrity(format!("{error:#}")))?;
            witness.independent_replay_passed = replay.acquisition_end_match;
            if !replay.acquisition_end_match {
                witness.status = "integrity_error".into();
                witness.reason = Some("independent acquisition replay disagrees".into());
            }
            return Ok(FreeSpaceWitness {
                witness,
                end_state: AirborneFlightStateV1::from_live(&replay.final_state),
                peak_com_height_m: replay.peak_com_height_m,
            });
        }
    };
    let mut commands = acquisition_updates.clone();
    if proposal
        .updates
        .first()
        .is_some_and(|update| update.physics_step != acquisition.end_state.physics_step)
    {
        return Err(WitnessMaterializationFailure::Integrity(
            "terminal candidate does not continue the actual acquisition clock".into(),
        ));
    }
    commands.extend(proposal.updates.iter().cloned());
    validate_command_schedule(
        incoming.physics_step,
        proposal.planned_end_physics_step,
        &commands,
    )
    .map_err(|error| WitnessMaterializationFailure::Integrity(format!("{error:#}")))?;
    if !commands.iter().all(|update| {
        update.command.throttle_frac.is_finite()
            && update.command.target_attitude_rad.is_finite()
            && update.command == update.command.clamped()
    }) {
        return Err(WitnessMaterializationFailure::Integrity(
            "realized command list contains a nonfinite or unclamped command".into(),
        ));
    }
    let command_identity = stable_digest(&commands).map_err(|error| {
        WitnessMaterializationFailure::Integrity(format!("hash realized commands: {error:#}"))
    })?;
    let replay = replay_neutral(
        context,
        incoming,
        &commands,
        acquisition.end_state.physics_step,
        terminal_start_step,
        proposal.planned_end_physics_step,
        &actual_acquisition_end,
    )
    .map_err(|error| WitnessMaterializationFailure::Integrity(format!("{error:#}")))?;
    let endpoint_state_agreement = AirborneFlightStateV1::from_live(&replay.final_state)
        == proposal.end_state
        && replay.final_state.physics_step == proposal.planned_end_physics_step;
    let entry_snapshot = replay.terminal_entry.clone();
    let mut status = "free_space_target_plane_witness".to_owned();
    let mut reason = None;
    if !replay.acquisition_end_match || !endpoint_state_agreement {
        status = "integrity_error".into();
        reason = Some(if !replay.acquisition_end_match {
            "independent replay disagrees at actual acquisition endpoint".into()
        } else {
            "independent replay disagrees at commanded proposal endpoint".into()
        });
    } else if replay
        .target_plane
        .is_none_or(|(tick, safe)| !safe || tick != proposal.planned_end_physics_step)
    {
        status = "finite_miss".into();
        reason = Some(
            "first free-space target-plane contact is unsafe or differs from the proposal endpoint"
                .into(),
        );
    }
    let target_plane_witness =
        replay
            .target_plane
            .map(|(physics_step, safe)| TargetPlaneWitnessEvidenceV1 {
                physics_step,
                safe_by_existing_target_plane_mirror: safe,
                proposal_endpoint_contact_match: physics_step == proposal.planned_end_physics_step,
                free_space_endpoint_match: endpoint_state_agreement,
                actual_landing_claimed: false,
            });
    let (actual_entry_angle_rad, actual_entry_position_error_m, actual_entry_velocity_error_mps) =
        if let Some(snapshot) = &entry_snapshot {
            (
                Some((-snapshot.velocity_mps.y).atan2(snapshot.velocity_mps.x.abs())),
                Some(distance(
                    entry.predicted_entry.position_m,
                    snapshot.position_m,
                )),
                Some(distance(
                    entry.predicted_entry.velocity_mps,
                    snapshot.velocity_mps,
                )),
            )
        } else {
            (None, None, None)
        };
    let witness = NominalWitnessEvidenceV1 {
        seed_id: seed.seed_id.clone(),
        entry_index: entry.entry_index,
        status,
        reason,
        acquisition_start,
        predicted_acquisition_end: plan.predicted_end,
        actual_acquisition_end,
        acquisition_position_error_m,
        acquisition_velocity_error_mps,
        acquisition_fuel_error_kg,
        acquisition_turn_command_identity: turn_identity,
        acquisition_command_identity: acquisition_identity,
        actual_terminal_entry: entry_snapshot,
        actual_terminal_entry_angle_rad: actual_entry_angle_rad,
        actual_entry_position_error_m,
        actual_entry_velocity_error_mps,
        commands,
        command_identity,
        independent_replay_passed: replay.acquisition_end_match && endpoint_state_agreement,
        endpoint_state_agreement,
        target_plane_witness,
        terrain_audit: None,
        identity: String::new(),
    };
    Ok(FreeSpaceWitness {
        witness,
        end_state: AirborneFlightStateV1::from_live(&replay.final_state),
        peak_com_height_m: replay.peak_com_height_m,
    })
}

fn run_acquisition(
    context: &RunContext,
    incoming: &SimulationState,
    plan: &AcquisitionPlan,
) -> std::result::Result<AcquisitionExecution, WitnessMaterializationFailure> {
    if !incoming.physics_step.is_multiple_of(HELD_TICKS) {
        return Err(WitnessMaterializationFailure::Integrity(
            "acquisition input is off the original held-command boundary".into(),
        ));
    }
    let mut state = incoming.clone();
    let mut turn_updates = Vec::new();
    let mut burn_updates = Vec::new();
    for _ in 0..plan.turn_ticks {
        if state.physics_step.is_multiple_of(HELD_TICKS) {
            let command = Command {
                throttle_frac: 0.0,
                target_attitude_rad: plan.target_attitude_rad,
            };
            turn_updates.push(FlightProgramUpdateV1 {
                physics_step: state.physics_step,
                phase: "nominal_acquisition_turn".into(),
                command,
            });
            state.set_command(command);
        }
        state.step_physics_and_classify_contact(context);
        if !finite_live_state(&state) {
            return Err(WitnessMaterializationFailure::Integrity(
                "nonfinite live state during acquisition turn".into(),
            ));
        }
    }
    let policy = BodyAwareTerminalPolicyV1::default();
    for _ in 0..plan.burn_ticks / HELD_TICKS {
        if !state.physics_step.is_multiple_of(HELD_TICKS) {
            return Err(WitnessMaterializationFailure::Integrity(
                "acquisition burn lost the original held-command cadence".into(),
            ));
        }
        let throttle = paired_throttle(
            context,
            &policy,
            state.mass_kg(context),
            plan.thrust_acceleration_mps2.length(),
        )
        .map_err(|error| {
            WitnessMaterializationFailure::FiniteMiss(format!(
                "paired acquisition throttle rejected: {error:#}"
            ))
        })?;
        let command = Command {
            throttle_frac: throttle,
            target_attitude_rad: plan.target_attitude_rad,
        };
        burn_updates.push(FlightProgramUpdateV1 {
            physics_step: state.physics_step,
            phase: "nominal_acquisition".into(),
            command,
        });
        state.set_command(command);
        for _ in 0..HELD_TICKS {
            state.step_physics_and_classify_contact(context);
            if !finite_live_state(&state) {
                return Err(WitnessMaterializationFailure::Integrity(
                    "nonfinite live state during acquisition burn".into(),
                ));
            }
        }
    }
    if state.physics_step != incoming.physics_step + plan.turn_ticks + plan.burn_ticks {
        return Err(WitnessMaterializationFailure::Integrity(
            "acquisition ended at an unexpected absolute physics step".into(),
        ));
    }
    Ok(AcquisitionExecution {
        end_state: state,
        turn_updates,
        burn_updates,
    })
}

pub(super) fn validate_command_schedule(
    start: u64,
    end: u64,
    updates: &[FlightProgramUpdateV1],
) -> Result<()> {
    let covered_pairs = end.saturating_sub(start).div_ceil(HELD_TICKS);
    if start > end || !start.is_multiple_of(HELD_TICKS) || updates.len() as u64 != covered_pairs {
        bail!("realized command list does not cover its complete held-command interval");
    }
    for (index, update) in updates.iter().enumerate() {
        let expected = start
            .checked_add(HELD_TICKS * index as u64)
            .context("realized command clock overflow")?;
        if update.physics_step != expected {
            bail!("realized command list has a gap or duplicate at H{expected}");
        }
        validate_update(update)?;
    }
    Ok(())
}

fn replay_neutral(
    context: &RunContext,
    incoming: &SimulationState,
    updates: &[FlightProgramUpdateV1],
    acquisition_end_step: u64,
    terminal_entry_step: u64,
    planned_end_step: u64,
    expected_acquisition_end: &SimulationStateSnapshotV1,
) -> Result<NeutralReplay> {
    let end = planned_end_step;
    validate_command_schedule(incoming.physics_step, end, updates)?;
    if acquisition_end_step < incoming.physics_step || acquisition_end_step > end {
        bail!("independent replay acquisition boundary lies outside its interval");
    }
    let mut state = incoming.clone();
    let mut command_index = 0_usize;
    let mut acquisition_end_match = if acquisition_end_step == incoming.physics_step {
        compare_full_snapshot(
            &SimulationStateSnapshotV1::from_state(&state),
            expected_acquisition_end,
        )
        .matched
    } else {
        false
    };
    let mut terminal_entry = (terminal_entry_step == incoming.physics_step)
        .then(|| SimulationStateSnapshotV1::from_state(&state));
    let mut target_plane = None;
    let mut peak_com_height_m = state.position_m.y;
    while state.physics_step < end {
        if state.physics_step.is_multiple_of(HELD_TICKS) {
            let update = updates
                .get(command_index)
                .context("independent replay command ended before endpoint")?;
            if update.physics_step != state.physics_step {
                bail!("independent replay command gap at H{}", state.physics_step);
            }
            state.set_command(update.command);
            command_index += 1;
        }
        state.step_physics_and_classify_contact(context);
        peak_com_height_m = peak_com_height_m.max(state.position_m.y);
        if !finite_live_state(&state) {
            bail!(
                "independent replay produced nonfinite state at H{}",
                state.physics_step
            );
        }
        if target_plane.is_none()
            && let Some(safe) = super::airborne::target_plane_contact(context, &state)
        {
            target_plane = Some((state.physics_step, safe));
        }
        if state.physics_step == acquisition_end_step {
            acquisition_end_match = compare_full_snapshot(
                &SimulationStateSnapshotV1::from_state(&state),
                expected_acquisition_end,
            )
            .matched;
        }
        if state.physics_step == terminal_entry_step {
            terminal_entry = Some(SimulationStateSnapshotV1::from_state(&state));
        }
    }
    if command_index != updates.len() || state.physics_step != end {
        bail!("independent replay failed to consume the complete realized schedule");
    }
    Ok(NeutralReplay {
        final_state: state,
        acquisition_end_match,
        terminal_entry,
        target_plane,
        peak_com_height_m,
    })
}

pub(super) fn finite_live_state(state: &SimulationState) -> bool {
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
    ]
    .iter()
    .all(|value| value.is_finite())
}

fn is_known_finite_candidate_rejection(message: &str) -> bool {
    [
        "candidate exceeds original absolute mission deadline",
        "intended pad reached before powered terminal at coast tick",
        "nominal shape requires at most one future apex, not dive and recover",
        "candidate exhausts actual incoming fuel",
        "first intended-pad contact is unsafe or enters from below",
        "finite terminal window contains no safe intended-pad contact",
        "paired throttle above maximum or unsupported remaining mass",
        "paired throttle below minimum",
    ]
    .iter()
    .any(|known| message.contains(known))
}

fn classify_candidate_failure(message: String) -> WitnessMaterializationFailure {
    if is_known_finite_candidate_rejection(&message) {
        WitnessMaterializationFailure::FiniteMiss(message)
    } else {
        WitnessMaterializationFailure::Integrity(format!(
            "unclassified terminal candidate failure: {message}"
        ))
    }
}

fn is_integrity_acquisition_plan_error(message: &str) -> bool {
    [
        "non-finite",
        "invalid vehicle rotation rate",
        "turn duration exceeds representable",
        "gain is not positive",
        "demand is non-finite",
    ]
    .iter()
    .any(|known| message.contains(known))
}
