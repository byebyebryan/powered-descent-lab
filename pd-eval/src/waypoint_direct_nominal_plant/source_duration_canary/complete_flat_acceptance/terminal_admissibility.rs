//! Frozen-tail diagnosis, not a generator, controller, or accepted witness.
//! Reference poses are independently submitted to unchanged core contact
//! classification through a synthetic neutral step; they are not flights.
//! Actual cadence lanes clone a replayed terminal-entry state and retain the
//! original global command clock. No source commands or references are fitted.

use super::super::super::{
    ThrottleSaturation,
    launch_contact_contract::{
        ContactPredicateMirrorEvidence, PreterminalContactStateEvidence,
        first_contact_predicate_margins, mirror_contact_predicates, preterminal_contact_state,
    },
    plant_applied_throttle, shortest_angle_delta, throttle_request, vehicle_input_v2,
};
use super::*;
use crate::WaypointDirectNominalDirectGenerationArtifact;
use pd_plan::conservative_ballistic_bridge::{
    AnalyticalBridgeV2, BridgeKindV2, exact_discrete_bridge_v2,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalContactAuditEvidence {
    pub classification: String,
    pub state: PreterminalContactStateEvidence,
    pub predicates: ContactPredicateMirrorEvidence,
    pub margins: ScheduledFirstContactMarginsEvidence,
    pub core_matches_predicate_mirror: bool,
    pub body_within_strict_terrain_domain: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalReferenceAuditEvidence {
    pub bridge_identity: String,
    pub terminal_tick_count: u64,
    pub reference_binding_passed: bool,
    pub maximum_logged_angle_residual_rad: f64,
    pub maximum_logged_position_error_residual_m: f64,
    pub maximum_logged_velocity_error_residual_mps: f64,
    pub core_probe_count: u64,
    pub core_probes_match_reference_poses: bool,
    pub core_predicate_mirror_parity: bool,
    pub reference_slew_within_vehicle_limit: bool,
    pub first_contact: Option<TerminalContactAuditEvidence>,
    pub reference_maximum_foot_predicate_passed: bool,
    pub first_contact_stable_safe_on_target: bool,
    pub clearance_scan: GeometryClearanceScanEvidence,
    pub admissible_reference_pose_path: bool,
    pub evidence_scope: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalEntryAuditEvidence {
    pub state: PlantStateEvidence,
    pub held_command: Command,
    pub minimum_touchdown_clearance_m: f64,
    pub minimum_hull_clearance_m: f64,
    pub global_controller_phase_ticks: u64,
    pub controller_interval_ticks: u64,
    pub first_terminal_command_update_due: bool,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalFrozenBaselineEvidence {
    pub authoritative_replay_passed: bool,
    pub authoritative_contact_matches_frozen: bool,
    pub prefix_contact_free: bool,
    pub replayed_entry: TerminalEntryAuditEvidence,
    pub terminal_log_matches_replay: bool,
    pub ordinary_neutral_parity: bool,
    pub contact_matches_frozen: bool,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalCadenceTickEvidence {
    pub physics_step: u64,
    pub terminal_tick: u64,
    pub command_update_due: bool,
    pub desired_command_throttle_frac: f64,
    pub desired_applied_throttle_frac: f64,
    pub held_command: Command,
    pub plant_applied_throttle_frac: f64,
    pub reference_state: KinematicStateV2,
    pub reference_target_attitude_rad: f64,
    pub incoming_state: PlantStateEvidence,
    pub signed_position_residual_m: Vec2,
    pub signed_velocity_residual_mps: Vec2,
    pub contact_classification: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalCadenceLaneEvidence {
    pub mode: String,
    pub entry_identity: String,
    pub physics_ticks_advanced: u64,
    pub command_update_count: u64,
    pub first_tick_update_due: bool,
    pub ordinary_neutral_parity: bool,
    pub core_predicate_mirror_parity: bool,
    pub logged_commands_and_states_match: Option<bool>,
    pub below_minimum_saturation_count: u64,
    pub above_maximum_saturation_count: u64,
    pub first_contact: Option<TerminalContactAuditEvidence>,
    pub first_contact_stable_safe_on_target: bool,
    pub clearance_scan: GeometryClearanceScanEvidence,
    pub maximum_airborne_position_error_m: f64,
    pub maximum_airborne_velocity_error_mps: f64,
    pub trace: Vec<TerminalCadenceTickEvidence>,
    pub trace_identity: String,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalCommonLiveResidualEvidence {
    pub physics_step: u64,
    pub per_tick_minus_held_position_m: Vec2,
    pub per_tick_minus_held_velocity_mps: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalCadenceComparisonEvidence {
    pub matched_entry_and_global_phase: bool,
    pub held_60hz: TerminalCadenceLaneEvidence,
    pub terminal_only_120hz: TerminalCadenceLaneEvidence,
    pub common_live_residuals: Vec<TerminalCommonLiveResidualEvidence>,
    pub evidence_scope: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalAdmissibilityRowEvidence {
    pub row_index: usize,
    pub basis_index: usize,
    pub duration_offset_ticks: i64,
    pub wrapper_identity: String,
    pub frozen_complete_witness_accepted: bool,
    pub reference: TerminalReferenceAuditEvidence,
    pub frozen_baseline: TerminalFrozenBaselineEvidence,
    pub cadence_comparison: Option<TerminalCadenceComparisonEvidence>,
    pub identity: String,
}

/// Audit one retained scheduled row. The caller preflights the complete raw
/// artifact family before this function is allowed to construct any plant.
pub fn evaluate_waypoint_direct_terminal_row(
    generation: &WaypointDirectNominalDirectGenerationArtifact,
    row_index: usize,
    compare_cadence: bool,
) -> Result<TerminalAdmissibilityRowEvidence> {
    crate::validate_waypoint_direct_nominal_direct_generation_request(&generation.request)?;
    let row = generation
        .rows
        .get(row_index)
        .context("terminal row missing")?;
    let basis = generation
        .bases
        .get(row.basis_index)
        .context("terminal basis missing")?;
    let schedule = row
        .paired_schedule
        .as_ref()
        .context("terminal schedule missing")?;
    let wrapper = row.wrapper.as_ref().context("terminal wrapper missing")?;
    let analysis = row
        .source_duration
        .as_ref()
        .and_then(|source| source.launch_and_analytical_screen.as_ref())
        .context("terminal launch evidence missing")?;
    if row.row_index != row_index
        || basis.basis_index != row.basis_index
        || row.basis_candidate_identity != basis.candidate_identity
        || row.generated_basis_identity != basis.generated_basis_identity
        || row.accepted != wrapper.accepted
    {
        bail!("terminal row/basis/wrapper binding mismatch");
    }
    let run = LaunchFeasibilityCadenceRunEvidence {
        cadence: HELD_CADENCE.to_owned(),
        launch: analysis.launch.clone(),
        reseeded_bridge: analysis.reseeded_bridge.clone(),
        rollout: schedule
            .full_flight_rollout
            .clone()
            .context("terminal full rollout missing")?,
    };
    let context =
        RunContext::from_scenario(&generation.request.scenario).map_err(anyhow::Error::msg)?;
    let authoritative = replay_logged_cadence(&context, &run)?;
    let authoritative_contact_matches_frozen = authoritative.first_contact
        == schedule.full_flight_first_contact
        && authoritative.first_contact.as_ref() == wrapper.first_contact.as_ref();
    if !authoritative.trace.passed || !authoritative_contact_matches_frozen {
        bail!("terminal authoritative frozen replay failed");
    }
    let logs = replay_log_ticks(&run);
    let terminal_log_start = logs
        .iter()
        .position(|tick| tick.phase == "terminal_bridge")
        .context("terminal phase missing from frozen log")?;
    let terminal_start_step = logs[terminal_log_start].physics_step;
    let entry = replay_terminal_prefix(&context, &logs[..terminal_log_start])?;
    if entry.physics_step + 1 != terminal_start_step {
        bail!("terminal prefix does not end at original global phase boundary");
    }
    let entry_evidence = terminal_entry_evidence(&entry, &context)?;
    let source = basis
        .original_source_handoff
        .context("terminal source handoff missing")?;
    let coast_ticks = basis
        .coast_tick_count
        .context("terminal coast ticks missing")?;
    let terminal_ticks = basis
        .terminal_bridge_tick_count
        .context("terminal bridge ticks missing")?;
    let mut terminal_start = source;
    for _ in 0..coast_ticks {
        terminal_start.velocity_mps.y -= context.world.gravity_mps2 * context.sim.physics_dt_s();
        terminal_start.position_m += terminal_start.velocity_mps * context.sim.physics_dt_s();
    }
    let terminal_end = KinematicStateV2 {
        position_m: Vec2::new(
            context.target_pad.center_x_m,
            context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m,
        ),
        velocity_mps: Vec2::new(
            0.0,
            -generation
                .policy
                .analytical_policy
                .terminal_target_downward_speed_fraction
                * context.vehicle.safe_touchdown_normal_speed_mps,
        ),
    };
    let bridge = exact_discrete_bridge_v2(
        &generation.policy.analytical_policy,
        &vehicle_input_v2(&context.vehicle),
        BridgeKindV2::Terminal,
        terminal_start,
        terminal_end,
        terminal_ticks,
    )
    .map_err(anyhow::Error::msg)?;
    let angles = terminal_reference_angles(&bridge)?;
    let policy = terminal_clearance_policy(generation, &context)?;
    let frozen_ticks = &run.rollout.per_step[run
        .rollout
        .per_step
        .iter()
        .position(|tick| tick.phase == "terminal_bridge")
        .context("terminal rollout ticks missing")?..];
    let (held, residuals) = run_terminal_lane(
        &context,
        &entry,
        &entry_evidence.identity,
        &bridge,
        &angles,
        policy,
        Some(frozen_ticks),
    )?;
    let baseline_contact_matches = held
        .first_contact
        .as_ref()
        .zip(wrapper.first_contact.as_ref())
        .is_some_and(|(actual, frozen)| {
            actual.classification == frozen.classification
                && actual.state == frozen.state
                && actual.predicates == frozen.predicates
        });
    let baseline_passed = authoritative.trace.passed
        && authoritative_contact_matches_frozen
        && held.logged_commands_and_states_match == Some(true)
        && held.ordinary_neutral_parity
        && held.core_predicate_mirror_parity
        && baseline_contact_matches;
    let terminal_log_matches_replay = held.logged_commands_and_states_match == Some(true);
    let ordinary_neutral_parity = held.ordinary_neutral_parity;
    let mut reference = audit_terminal_reference(&context, &entry, &bridge, &angles, policy)?;
    reference.maximum_logged_angle_residual_rad = residuals.angle;
    reference.maximum_logged_position_error_residual_m = residuals.position;
    reference.maximum_logged_velocity_error_residual_mps = residuals.velocity;
    reference.reference_binding_passed = residuals.valid;
    reference.admissible_reference_pose_path &= residuals.valid;
    let comparison = if compare_cadence {
        let (per_tick, _) = run_terminal_lane(
            &context,
            &entry,
            &entry_evidence.identity,
            &bridge,
            &angles,
            policy,
            None,
        )?;
        let common_live_residuals = common_live_residuals(&held.trace, &per_tick.trace)?;
        Some(TerminalCadenceComparisonEvidence {
            matched_entry_and_global_phase: held.entry_identity == entry_evidence.identity
                && per_tick.entry_identity == entry_evidence.identity,
            held_60hz: held,
            terminal_only_120hz: per_tick,
            common_live_residuals,
            evidence_scope: "Matched-entry terminal-only cadence diagnostic with unchanged core and frozen reference. No generated/accepted witness, controller fix, robust certificate, or waypoint demand is inferred.".into(),
        })
    } else {
        None
    };
    let mut evidence = TerminalAdmissibilityRowEvidence {
        row_index,
        basis_index: row.basis_index,
        duration_offset_ticks: row.duration_offset_ticks,
        wrapper_identity: wrapper.wrapper_identity.clone(),
        frozen_complete_witness_accepted: row.accepted,
        reference,
        frozen_baseline: TerminalFrozenBaselineEvidence {
            authoritative_replay_passed: authoritative.trace.passed,
            authoritative_contact_matches_frozen,
            prefix_contact_free: true,
            replayed_entry: entry_evidence,
            terminal_log_matches_replay,
            ordinary_neutral_parity,
            contact_matches_frozen: baseline_contact_matches,
            passed: baseline_passed,
        },
        cadence_comparison: comparison,
        identity: String::new(),
    };
    evidence.identity = stable_digest(&evidence)?;
    Ok(evidence)
}

fn terminal_reference_angles(bridge: &AnalyticalBridgeV2) -> Result<Vec<f64>> {
    // This pass supports powered retained terminal tails; refuse rather than
    // invent the profile materializer's unpowered lookahead convention.
    bridge
        .samples
        .iter()
        .map(|sample| {
            if sample.thrust_acceleration_mps2.length() <= 1.0e-12 {
                bail!("unpowered terminal reference sample is outside this retained audit");
            }
            Ok(sample
                .thrust_acceleration_mps2
                .x
                .atan2(sample.thrust_acceleration_mps2.y))
        })
        .collect()
}

fn terminal_clearance_policy(
    generation: &WaypointDirectNominalDirectGenerationArtifact,
    context: &RunContext,
) -> Result<ClearancePolicy> {
    let pad = |id: &str| -> Result<FlatPadBounds> {
        let spec = generation
            .request
            .scenario
            .world
            .landing_pads
            .iter()
            .find(|pad| pad.id == id)
            .context("terminal audit pad missing")?;
        Ok(flat_pad_bounds(
            context,
            &PadInputV2 {
                center_x_m: spec.center_x_m,
                surface_y_m: spec.surface_y_m,
                width_m: spec.width_m,
            },
        ))
    };
    let policy = ClearancePolicy {
        source_pad: pad(&generation.source_pad_id)?,
        target_pad: pad(&generation.target_pad_id)?,
        minimum_clearance_m: generation.policy.analytical_policy.minimum_clearance_m,
    };
    if !policy.source_pad.flat || !policy.target_pad.flat {
        bail!("terminal audit requires unchanged flat pads");
    }
    Ok(policy)
}

fn replay_terminal_prefix(
    context: &RunContext,
    logs: &[ReplayLogTick<'_>],
) -> Result<SimulationState> {
    let mut state = SimulationState::new(context)?;
    for tick in logs {
        if tick.physics_step != state.physics_step + 1 || tick.expected_contact != "none" {
            bail!("terminal prefix is noncontiguous or contains contact");
        }
        if state
            .physics_step
            .is_multiple_of(context.sim.control_interval_steps())
        {
            state.set_command(Command {
                throttle_frac: tick.commanded_throttle_frac,
                target_attitude_rad: tick.desired_target_attitude_rad,
            });
        }
        if state.held_command.throttle_frac != tick.commanded_throttle_frac
            || state.held_command.target_attitude_rad != tick.held_target_attitude_rad
            || state.attitude_rad != tick.attitude_before_step_rad
        {
            bail!("terminal prefix command/prestep state mismatch");
        }
        if state.step_physics_and_classify_contact(context) != ContactClassification::None {
            bail!("terminal prefix contacted before entry");
        }
    }
    Ok(state)
}

fn terminal_entry_evidence(
    state: &SimulationState,
    context: &RunContext,
) -> Result<TerminalEntryAuditEvidence> {
    let interval = context.sim.control_interval_steps();
    let mut evidence = TerminalEntryAuditEvidence {
        state: plant_state_evidence(state, context),
        held_command: state.held_command,
        minimum_touchdown_clearance_m: state.min_touchdown_clearance_m,
        minimum_hull_clearance_m: state.min_hull_clearance_m,
        global_controller_phase_ticks: state.physics_step % interval,
        controller_interval_ticks: interval,
        first_terminal_command_update_due: state.physics_step.is_multiple_of(interval),
        identity: String::new(),
    };
    evidence.identity = stable_digest(&evidence)?;
    Ok(evidence)
}

pub(super) fn contact_audit(
    context: &RunContext,
    state: &SimulationState,
    classification: &ContactClassification,
) -> TerminalContactAuditEvidence {
    let predicates = mirror_contact_predicates(state, context);
    let contact_state = preterminal_contact_state(state, context);
    let margins = public_contact_margins(&first_contact_predicate_margins(&contact_state, context));
    let label = contact_classification_label(classification).to_owned();
    TerminalContactAuditEvidence {
        core_matches_predicate_mirror: predicates.predicted_classification == label,
        body_within_strict_terrain_domain: body_clearance(
            context,
            state,
            body_aabb(state, &context.vehicle.geometry),
        )
        .is_ok(),
        classification: label,
        state: contact_state,
        predicates,
        margins,
    }
}

/// A synthetic pre-state is inverted from the requested post-state so that
/// the unchanged neutral core step classifies this reference pose. It uses
/// idle thrust and constant fuel; it is *not* a physical reference rollout.
pub(super) fn core_reference_pose_probe(
    context: &RunContext,
    template: &SimulationState,
    expected: KinematicStateV2,
    angle: f64,
    signed_rate: f64,
    physics_step: u64,
) -> Result<(SimulationState, ContactClassification, bool)> {
    let dt = context.sim.physics_dt_s();
    let mut probe = template.clone();
    probe.physics_step = physics_step
        .checked_sub(1)
        .context("reference probe step zero")?;
    probe.sim_time_s = probe.physics_step as f64 / f64::from(context.sim.physics_hz);
    probe.position_m = expected.position_m - expected.velocity_mps * dt;
    probe.velocity_mps = expected.velocity_mps + Vec2::new(0.0, context.world.gravity_mps2 * dt);
    probe.attitude_rad = angle - signed_rate * dt;
    probe.set_command(Command {
        throttle_frac: 0.0,
        target_attitude_rad: angle,
    });
    let classification = probe.step_physics_and_classify_contact(context);
    let matches = (probe.position_m - expected.position_m).length() <= 1.0e-9
        && (probe.velocity_mps - expected.velocity_mps).length() <= 1.0e-9
        && shortest_angle_delta(probe.attitude_rad, angle).abs() <= 1.0e-9
        && (probe.angular_rate_radps - signed_rate).abs() <= 1.0e-9;
    Ok((probe, classification, matches))
}

fn audit_terminal_reference(
    context: &RunContext,
    entry: &SimulationState,
    bridge: &AnalyticalBridgeV2,
    angles: &[f64],
    policy: ClearancePolicy,
) -> Result<TerminalReferenceAuditEvidence> {
    let mut scan = empty_clearance_scan();
    let mut first_contact = None;
    let mut count = 0;
    let mut poses_match = true;
    let mut mirror_parity = true;
    let mut slew_passed = true;
    for (index, sample) in bridge.samples.iter().enumerate() {
        let previous = index.checked_sub(1).map_or(angles[0], |i| angles[i]);
        let signed_rate =
            shortest_angle_delta(previous, angles[index]) / context.sim.physics_dt_s();
        slew_passed &= signed_rate.abs() <= context.vehicle.max_rotation_rate_radps;
        let step = entry.physics_step + index as u64 + 1;
        let (pose, classification, matches) = core_reference_pose_probe(
            context,
            entry,
            sample.state_m,
            angles[index],
            signed_rate,
            step,
        )?;
        poses_match &= matches;
        count += 1;
        scan.poststep_state_count += 1;
        let contact = contact_audit(context, &pose, &classification);
        mirror_parity &= contact.core_matches_predicate_mirror;
        if classification != ContactClassification::None {
            first_contact = Some(contact);
            break;
        }
        record_airborne_clearance(context, &pose, step, "terminal_bridge", policy, &mut scan);
    }
    let foot_passed = first_contact
        .as_ref()
        .is_some_and(|contact| contact.predicates.stable_maximum_clearance_predicate);
    let stable = first_contact.as_ref().is_some_and(contact_is_stable_target);
    Ok(TerminalReferenceAuditEvidence {
        bridge_identity: bridge.identity.clone(), terminal_tick_count: bridge.steps,
        reference_binding_passed: false, maximum_logged_angle_residual_rad: 0.0,
        maximum_logged_position_error_residual_m: 0.0, maximum_logged_velocity_error_residual_mps: 0.0,
        core_probe_count: count, core_probes_match_reference_poses: poses_match,
        core_predicate_mirror_parity: mirror_parity, reference_slew_within_vehicle_limit: slew_passed,
        first_contact, reference_maximum_foot_predicate_passed: foot_passed,
        first_contact_stable_safe_on_target: stable,
        admissible_reference_pose_path: poses_match && mirror_parity && slew_passed
            && stable && scan.all_airborne_states_passed,
        clearance_scan: scan,
        evidence_scope: "Analytical thrust-aligned reference poses, with idle synthetic pre-state inversions into unchanged core contact probes. First sample assumes alignment and zero reference angular rate; later rates are consecutive angle differences. No entry-attitude join, physical flight, fuel/commandability proof, generated witness, or robustness certificate.".into(),
    })
}

fn contact_is_stable_target(contact: &TerminalContactAuditEvidence) -> bool {
    contact.classification == "stable_touchdown_on_target"
        && contact.core_matches_predicate_mirror
        && contact.body_within_strict_terrain_domain
        && stable_safe_margins_pass(&contact.margins)
}

#[derive(Default)]
struct ReferenceBindingResiduals {
    angle: f64,
    position: f64,
    velocity: f64,
    valid: bool,
}

fn run_terminal_lane(
    context: &RunContext,
    entry: &SimulationState,
    entry_identity: &str,
    bridge: &AnalyticalBridgeV2,
    angles: &[f64],
    policy: ClearancePolicy,
    frozen: Option<&[LaunchRolloutTickEvidence]>,
) -> Result<(TerminalCadenceLaneEvidence, ReferenceBindingResiduals)> {
    let mut neutral = entry.clone();
    let mut ordinary = entry.clone();
    let mut scan = empty_clearance_scan();
    let mut trace = Vec::new();
    let mut first_contact = None;
    let mut parity = true;
    let mut mirror_parity = true;
    let mut log_parity = true;
    let mut residuals = ReferenceBindingResiduals {
        valid: true,
        ..Default::default()
    };
    let mut updates = 0;
    let mut below = 0;
    let mut above = 0;
    let mut max_position = 0.0_f64;
    let mut max_velocity = 0.0_f64;
    for (index, sample) in bridge.samples.iter().enumerate() {
        let log = frozen
            .map(|ticks| {
                ticks
                    .get(index)
                    .context("frozen terminal log ended before contact")
            })
            .transpose()?;
        if let Some(log) = log
            && (log.phase != "terminal_bridge" || log.physics_step != neutral.physics_step + 1)
        {
            bail!("terminal frozen log phase/step mismatch");
        }
        let desired = throttle_request(
            sample.thrust_acceleration_mps2.length(),
            neutral.mass_kg(context),
            context.vehicle.max_thrust_n,
            context.vehicle.max_fuel_burn_kgps,
            context.sim.physics_dt_s(),
            context.vehicle.min_throttle_frac,
        )?;
        let update_due = frozen.is_none()
            || neutral
                .physics_step
                .is_multiple_of(context.sim.control_interval_steps());
        if update_due {
            let command = log.map_or(
                Command {
                    throttle_frac: desired.command_fraction,
                    target_attitude_rad: angles[index],
                },
                |log| Command {
                    throttle_frac: log.commanded_throttle_frac,
                    target_attitude_rad: log.desired_target_attitude_rad,
                },
            );
            neutral.set_command(command);
            ordinary.set_command(command);
            updates += 1;
            below += u64::from(matches!(
                desired.saturation,
                ThrottleSaturation::BelowMinimum
            ));
            above += u64::from(matches!(
                desired.saturation,
                ThrottleSaturation::AboveMaximum
            ));
        }
        let command = neutral.held_command;
        let applied =
            plant_applied_throttle(command, context.vehicle.min_throttle_frac, neutral.fuel_kg);
        let before_angle = neutral.attitude_rad;
        let classification = neutral.step_physics_and_classify_contact(context);
        let events = ordinary.step(context);
        let label = contact_classification_label(&classification);
        parity &= same_ordinary_neutral_state(&ordinary, &neutral, &classification)
            && event_contact_label(&events) == label;
        let contact = contact_audit(context, &neutral, &classification);
        mirror_parity &= contact.core_matches_predicate_mirror;
        let position_delta = neutral.position_m - sample.state_m.position_m;
        let velocity_delta = neutral.velocity_mps - sample.state_m.velocity_mps;
        if classification == ContactClassification::None {
            max_position = max_position.max(position_delta.length());
            max_velocity = max_velocity.max(velocity_delta.length());
            record_airborne_clearance(
                context,
                &neutral,
                neutral.physics_step,
                "terminal_bridge",
                policy,
                &mut scan,
            );
        }
        scan.poststep_state_count += 1;
        if let Some(log) = log {
            let observed = ordinary.build_observation(context);
            log_parity &= command.throttle_frac == log.commanded_throttle_frac
                && command.target_attitude_rad == log.held_target_attitude_rad
                && before_angle == log.attitude_before_step_rad
                && ordinary.attitude_rad == log.attitude_after_step_rad
                && ordinary.angular_rate_radps == log.angular_rate_radps
                && applied == log.applied_throttle_frac
                && ordinary.fuel_kg == log.fuel_kg
                && ordinary.sim_time_s == log.sim_time_s
                && label == log.contact_classification
                && observed.touchdown_clearance_m == log.touchdown_clearance_m
                && observed.min_hull_clearance_m == log.hull_clearance_m;
            let angle_residual =
                shortest_angle_delta(angles[index], log.desired_target_attitude_rad).abs();
            let position_residual = log
                .state_position_error_m
                .map(|error| (position_delta.length() - error).abs());
            // Frozen ordinary stable touchdown zeroes velocity. Bind its
            // recorded residual using ordinary, but retain neutral incoming
            // velocity for all new contact and cross-lane evidence.
            let velocity_residual = log.state_velocity_error_mps.map(|error| {
                ((ordinary.velocity_mps - sample.state_m.velocity_mps).length() - error).abs()
            });
            residuals.angle = residuals.angle.max(angle_residual);
            residuals.position = residuals
                .position
                .max(position_residual.unwrap_or(f64::INFINITY));
            residuals.velocity = residuals
                .velocity
                .max(velocity_residual.unwrap_or(f64::INFINITY));
            residuals.valid &= angle_residual <= 1.0e-12
                && position_residual.is_some_and(|r| r <= 1.0e-6)
                && velocity_residual.is_some_and(|r| r <= 1.0e-6);
        }
        trace.push(TerminalCadenceTickEvidence {
            physics_step: neutral.physics_step,
            terminal_tick: index as u64 + 1,
            command_update_due: update_due,
            desired_command_throttle_frac: desired.command_fraction,
            desired_applied_throttle_frac: desired.applied_fraction,
            held_command: command,
            plant_applied_throttle_frac: applied,
            reference_state: sample.state_m,
            reference_target_attitude_rad: angles[index],
            incoming_state: plant_state_evidence(&neutral, context),
            signed_position_residual_m: position_delta,
            signed_velocity_residual_mps: velocity_delta,
            contact_classification: label.to_owned(),
        });
        if classification != ContactClassification::None {
            first_contact = Some(contact);
            break;
        }
    }
    if let Some(logs) = frozen {
        log_parity &= logs.len() == trace.len();
    }
    let stable = first_contact.as_ref().is_some_and(contact_is_stable_target);
    let mut evidence = TerminalCadenceLaneEvidence {
        mode: if frozen.is_some() {
            "frozen_global_held_60hz"
        } else {
            "terminal_only_per_tick_120hz"
        }
        .into(),
        entry_identity: entry_identity.into(),
        physics_ticks_advanced: trace.len() as u64,
        command_update_count: updates,
        first_tick_update_due: trace.first().is_some_and(|tick| tick.command_update_due),
        ordinary_neutral_parity: parity,
        core_predicate_mirror_parity: mirror_parity,
        logged_commands_and_states_match: frozen.map(|_| log_parity),
        below_minimum_saturation_count: below,
        above_maximum_saturation_count: above,
        first_contact,
        first_contact_stable_safe_on_target: stable,
        clearance_scan: scan,
        maximum_airborne_position_error_m: max_position,
        maximum_airborne_velocity_error_mps: max_velocity,
        trace_identity: stable_digest(&trace)?,
        trace,
        identity: String::new(),
    };
    evidence.identity = stable_digest(&evidence)?;
    Ok((evidence, residuals))
}

fn common_live_residuals(
    held: &[TerminalCadenceTickEvidence],
    per_tick: &[TerminalCadenceTickEvidence],
) -> Result<Vec<TerminalCommonLiveResidualEvidence>> {
    let mut residuals = Vec::new();
    for (a, b) in held.iter().zip(per_tick) {
        if a.physics_step != b.physics_step || a.terminal_tick != b.terminal_tick {
            bail!("terminal comparison lost global event alignment");
        }
        if a.contact_classification != "none" || b.contact_classification != "none" {
            break;
        }
        residuals.push(TerminalCommonLiveResidualEvidence {
            physics_step: a.physics_step,
            per_tick_minus_held_position_m: b.incoming_state.position_m
                - a.incoming_state.position_m,
            per_tick_minus_held_velocity_mps: b.incoming_state.velocity_mps
                - a.incoming_state.velocity_mps,
        });
    }
    Ok(residuals)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> RunContext {
        super::super::tests::test_context()
    }

    fn policy() -> ClearancePolicy {
        let pad = FlatPadBounds {
            left_m: -20.0,
            right_m: 20.0,
            surface_y_m: 0.0,
            flat: true,
        };
        ClearancePolicy {
            source_pad: pad,
            target_pad: pad,
            minimum_clearance_m: 5.0,
        }
    }

    fn bridge(context: &RunContext, start: KinematicStateV2, ticks: u64) -> AnalyticalBridgeV2 {
        let mut policy =
            crate::WaypointDirectNominalDirectGenerationPolicyV1::default().analytical_policy;
        policy.gravity_mps2 = context.world.gravity_mps2;
        policy.physics_hz = context.sim.physics_hz;
        exact_discrete_bridge_v2(
            &policy,
            &vehicle_input_v2(&context.vehicle),
            BridgeKindV2::Terminal,
            start,
            KinematicStateV2 {
                position_m: start.position_m
                    + start.velocity_mps * (ticks as f64 * context.sim.physics_dt_s()),
                velocity_mps: start.velocity_mps,
            },
            ticks,
        )
        .unwrap()
    }

    #[test]
    fn neutral_pose_probe_retains_incoming_velocity_and_core_stable_contact() {
        let context = context();
        let template = SimulationState::new(&context).unwrap();
        let requested = KinematicStateV2 {
            position_m: Vec2::new(0.0, 4.999),
            velocity_mps: Vec2::new(0.0, -1.0),
        };
        let (pose, classification, matches) =
            core_reference_pose_probe(&context, &template, requested, 0.0, 0.0, 17).unwrap();
        assert!(matches);
        assert_eq!(pose.physics_step, 17);
        assert_eq!(pose.velocity_mps, requested.velocity_mps);
        let contact = contact_audit(&context, &pose, &classification);
        assert!(contact_is_stable_target(&contact));
        assert!(contact.core_matches_predicate_mirror);
        assert_eq!(pose.fuel_kg, template.fuel_kg);
    }

    #[test]
    fn thrust_aligned_tilt_can_fail_upper_foot_before_attitude_or_speed() {
        let context = context();
        let template = SimulationState::new(&context).unwrap();
        let angle = 0.08_f64;
        let height = context.vehicle.geometry.touchdown_base_offset_m * angle.cos()
            + context.vehicle.geometry.touchdown_half_span_m * angle.sin()
            - 0.001;
        let requested = KinematicStateV2 {
            position_m: Vec2::new(0.0, height),
            velocity_mps: Vec2::new(0.0, -1.0),
        };
        let (pose, classification, matches) =
            core_reference_pose_probe(&context, &template, requested, angle, 0.0, 1).unwrap();
        assert!(matches);
        let contact = contact_audit(&context, &pose, &classification);
        assert_eq!(contact.classification, "crash");
        assert!(contact.core_matches_predicate_mirror);
        assert!(!contact.predicates.stable_maximum_clearance_predicate);
        assert!(contact.margins.stable_maximum_clearance_margin_m < 0.0);
        assert!(contact.margins.safe_attitude_margin_rad > 0.0);
        assert!(contact.margins.safe_normal_speed_margin_mps > 0.0);
    }

    #[test]
    fn reference_probe_does_not_hide_a_slew_limit_violation() {
        let context = context();
        let template = SimulationState::new(&context).unwrap();
        let requested = KinematicStateV2 {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(0.0, -1.0),
        };
        let (_, _, matches) = core_reference_pose_probe(
            &context,
            &template,
            requested,
            0.0,
            context.vehicle.max_rotation_rate_radps * 2.0,
            1,
        )
        .unwrap();
        assert!(!matches);
    }

    #[test]
    fn prefix_refuses_noncontiguous_or_contacting_logs_before_entry() {
        let context = context();
        let tick = ReplayLogTick {
            physics_step: 2,
            phase: "coast",
            expected_contact: "none",
            desired_target_attitude_rad: 0.0,
            held_target_attitude_rad: 0.0,
            commanded_throttle_frac: 0.0,
            attitude_before_step_rad: 0.0,
            logged_applied_throttle_frac: None,
        };
        assert!(replay_terminal_prefix(&context, &[tick]).is_err());
        let tick = ReplayLogTick {
            physics_step: 1,
            phase: "coast",
            expected_contact: "crash",
            desired_target_attitude_rad: 0.0,
            held_target_attitude_rad: 0.0,
            commanded_throttle_frac: 0.0,
            attitude_before_step_rad: 0.0,
            logged_applied_throttle_frac: None,
        };
        assert!(replay_terminal_prefix(&context, &[tick]).is_err());
        assert!(
            core_reference_pose_probe(
                &context,
                &SimulationState::new(&context).unwrap(),
                KinematicStateV2 {
                    position_m: Vec2::new(0.0, 20.0),
                    velocity_mps: Vec2::new(0.0, 0.0)
                },
                0.0,
                0.0,
                0
            )
            .is_err()
        );
    }

    #[test]
    fn odd_global_entry_keeps_old_command_for_first_held_tick_only() {
        let context = context();
        let mut entry = SimulationState::new(&context).unwrap();
        entry.position_m = Vec2::new(0.0, 20.0);
        entry.velocity_mps = Vec2::new(0.0, -0.1);
        entry.physics_step = 101;
        entry.sim_time_s = 101.0 / 120.0;
        entry.set_command(Command {
            throttle_frac: 0.0,
            target_attitude_rad: 0.0,
        });
        let entry_evidence = terminal_entry_evidence(&entry, &context).unwrap();
        assert_eq!(entry_evidence.global_controller_phase_ticks, 1);
        assert!(!entry_evidence.first_terminal_command_update_due);
        let bridge = bridge(
            &context,
            KinematicStateV2 {
                position_m: entry.position_m,
                velocity_mps: entry.velocity_mps,
            },
            2,
        );
        let angles = terminal_reference_angles(&bridge).unwrap();
        // Synthetic frozen held log, created directly by the unchanged core.
        let mut stored = entry.clone();
        let mut logs = Vec::new();
        for (index, sample) in bridge.samples.iter().enumerate() {
            let before = stored.attitude_rad;
            if stored.physics_step.is_multiple_of(2) {
                let throttle = throttle_request(
                    sample.thrust_acceleration_mps2.length(),
                    stored.mass_kg(&context),
                    context.vehicle.max_thrust_n,
                    context.vehicle.max_fuel_burn_kgps,
                    context.sim.physics_dt_s(),
                    context.vehicle.min_throttle_frac,
                )
                .unwrap();
                stored.set_command(Command {
                    throttle_frac: throttle.command_fraction,
                    target_attitude_rad: angles[index],
                });
            }
            let applied = plant_applied_throttle(
                stored.held_command,
                context.vehicle.min_throttle_frac,
                stored.fuel_kg,
            );
            let classification = stored.step_physics_and_classify_contact(&context);
            let observation = stored.build_observation(&context);
            logs.push(LaunchRolloutTickEvidence {
                physics_step: stored.physics_step,
                phase: "terminal_bridge".into(),
                contact_classification: contact_classification_label(&classification).into(),
                desired_target_attitude_rad: angles[index],
                held_target_attitude_rad: stored.held_command.target_attitude_rad,
                attitude_before_step_rad: before,
                attitude_after_step_rad: stored.attitude_rad,
                angular_rate_radps: stored.angular_rate_radps,
                commanded_throttle_frac: stored.held_command.throttle_frac,
                applied_throttle_frac: applied,
                state_position_error_m: Some(
                    (stored.position_m - sample.state_m.position_m).length(),
                ),
                state_velocity_error_mps: Some(
                    (stored.velocity_mps - sample.state_m.velocity_mps).length(),
                ),
                touchdown_clearance_m: observation.touchdown_clearance_m,
                hull_clearance_m: observation.min_hull_clearance_m,
                fuel_kg: stored.fuel_kg,
                sim_time_s: stored.sim_time_s,
            });
        }
        let (held, bindings) = run_terminal_lane(
            &context,
            &entry,
            &entry_evidence.identity,
            &bridge,
            &angles,
            policy(),
            Some(&logs),
        )
        .unwrap();
        let (per_tick, _) = run_terminal_lane(
            &context,
            &entry,
            &entry_evidence.identity,
            &bridge,
            &angles,
            policy(),
            None,
        )
        .unwrap();
        assert_eq!(held.logged_commands_and_states_match, Some(true));
        assert!(bindings.valid);
        assert!(!held.first_tick_update_due);
        assert!(per_tick.first_tick_update_due);
        assert_eq!(held.trace[0].physics_step, 102);
        assert_eq!(per_tick.trace[0].physics_step, 102);
        assert_eq!(held.trace[0].held_command, entry.held_command);
        assert!(per_tick.trace[0].held_command.throttle_frac > 0.0);
        assert_eq!(held.entry_identity, per_tick.entry_identity);
        assert_eq!(entry.held_command.throttle_frac, 0.0); // cloned, never mutated
        assert_eq!(entry.physics_step, 101);
        assert!(held.first_contact.is_none());
        assert!(!held.first_contact_stable_safe_on_target); // no-contact is not success
        let mut bad_logs = logs.clone();
        bad_logs[0].state_position_error_m = Some(0.01);
        let (_, binding) = run_terminal_lane(
            &context,
            &entry,
            &entry_evidence.identity,
            &bridge,
            &angles,
            policy(),
            Some(&bad_logs),
        )
        .unwrap();
        assert!(!binding.valid);
        bad_logs = logs.clone();
        bad_logs[0].state_velocity_error_mps = Some(0.01);
        let (_, binding) = run_terminal_lane(
            &context,
            &entry,
            &entry_evidence.identity,
            &bridge,
            &angles,
            policy(),
            Some(&bad_logs),
        )
        .unwrap();
        assert!(!binding.valid);
        bad_logs = logs.clone();
        bad_logs[0].fuel_kg += 0.01;
        let (bad_lane, _) = run_terminal_lane(
            &context,
            &entry,
            &entry_evidence.identity,
            &bridge,
            &angles,
            policy(),
            Some(&bad_logs),
        )
        .unwrap();
        assert_eq!(bad_lane.logged_commands_and_states_match, Some(false));
        bad_logs[0].physics_step += 1;
        assert!(
            run_terminal_lane(
                &context,
                &entry,
                &entry_evidence.identity,
                &bridge,
                &angles,
                policy(),
                Some(&bad_logs)
            )
            .is_err()
        );
        let mut held_trace = held.trace.clone();
        held_trace[1].contact_classification = "crash".into();
        assert_eq!(
            common_live_residuals(&held_trace, &per_tick.trace)
                .unwrap()
                .len(),
            1
        );
        held_trace[0].physics_step += 1;
        assert!(common_live_residuals(&held_trace, &per_tick.trace).is_err());
    }

    #[test]
    fn unsupported_unpowered_terminal_reference_is_refused() {
        let context = context();
        let mut bridge = bridge(
            &context,
            KinematicStateV2 {
                position_m: Vec2::new(0.0, 20.0),
                velocity_mps: Vec2::new(0.0, -0.1),
            },
            2,
        );
        bridge.samples[0].thrust_acceleration_mps2 = Vec2::new(0.0, 0.0);
        assert!(terminal_reference_angles(&bridge).is_err());
    }

    #[test]
    fn stable_lane_binds_ordinary_zeroing_but_preserves_neutral_incoming_state() {
        let context = context();
        let mut entry = SimulationState::new(&context).unwrap();
        entry.physics_step = 100;
        entry.sim_time_s = 100.0 / 120.0;
        let start = KinematicStateV2 {
            position_m: entry.position_m,
            velocity_mps: entry.velocity_mps,
        };
        let ticks = 2_u64;
        let dt = context.sim.physics_dt_s();
        let angle = 0.001_f64;
        let net = Vec2::new(
            context.world.gravity_mps2 * angle.sin(),
            context.world.gravity_mps2 * (angle.cos() - 1.0),
        );
        let mut bridge_policy =
            crate::WaypointDirectNominalDirectGenerationPolicyV1::default().analytical_policy;
        bridge_policy.gravity_mps2 = context.world.gravity_mps2;
        let end = KinematicStateV2 {
            position_m: start.position_m
                + start.velocity_mps * (ticks as f64 * dt)
                + net * (dt * dt * (ticks * (ticks + 1)) as f64 / 2.0),
            velocity_mps: start.velocity_mps + net * (ticks as f64 * dt),
        };
        let bridge = exact_discrete_bridge_v2(
            &bridge_policy,
            &vehicle_input_v2(&context.vehicle),
            BridgeKindV2::Terminal,
            start,
            end,
            ticks,
        )
        .unwrap();
        let angles = terminal_reference_angles(&bridge).unwrap();
        let sample = bridge.samples[0];
        let desired = throttle_request(
            sample.thrust_acceleration_mps2.length(),
            entry.mass_kg(&context),
            context.vehicle.max_thrust_n,
            context.vehicle.max_fuel_burn_kgps,
            dt,
            context.vehicle.min_throttle_frac,
        )
        .unwrap();
        let mut ordinary = entry.clone();
        ordinary.set_command(Command {
            throttle_frac: desired.command_fraction,
            target_attitude_rad: angles[0],
        });
        let applied = plant_applied_throttle(
            ordinary.held_command,
            context.vehicle.min_throttle_frac,
            ordinary.fuel_kg,
        );
        let events = ordinary.step(&context);
        assert_eq!(event_contact_label(&events), "stable_touchdown_on_target");
        assert_eq!(ordinary.velocity_mps, Vec2::new(0.0, 0.0));
        assert_eq!(ordinary.angular_rate_radps, 0.0);
        let observation = ordinary.build_observation(&context);
        let log = LaunchRolloutTickEvidence {
            physics_step: ordinary.physics_step,
            phase: "terminal_bridge".into(),
            contact_classification: "stable_touchdown_on_target".into(),
            desired_target_attitude_rad: angles[0],
            held_target_attitude_rad: angles[0],
            attitude_before_step_rad: entry.attitude_rad,
            attitude_after_step_rad: ordinary.attitude_rad,
            angular_rate_radps: ordinary.angular_rate_radps,
            commanded_throttle_frac: desired.command_fraction,
            applied_throttle_frac: applied,
            state_position_error_m: Some(
                (ordinary.position_m - sample.state_m.position_m).length(),
            ),
            state_velocity_error_mps: Some(
                (ordinary.velocity_mps - sample.state_m.velocity_mps).length(),
            ),
            touchdown_clearance_m: observation.touchdown_clearance_m,
            hull_clearance_m: observation.min_hull_clearance_m,
            fuel_kg: ordinary.fuel_kg,
            sim_time_s: ordinary.sim_time_s,
        };
        let entry_evidence = terminal_entry_evidence(&entry, &context).unwrap();
        let (lane, binding) = run_terminal_lane(
            &context,
            &entry,
            &entry_evidence.identity,
            &bridge,
            &angles,
            policy(),
            Some(&[log]),
        )
        .unwrap();
        assert_eq!(lane.logged_commands_and_states_match, Some(true));
        assert!(binding.valid);
        assert!(lane.ordinary_neutral_parity);
        assert!(lane.first_contact_stable_safe_on_target);
        assert_eq!(lane.trace.len(), 1);
        let incoming = &lane.first_contact.as_ref().unwrap().state;
        assert!(incoming.velocity_mps.y < -0.09);
        assert!(incoming.angular_rate_radps > 0.1);
        assert_eq!(
            lane.trace[0].incoming_state.velocity_mps,
            incoming.velocity_mps
        );
        assert_eq!(
            lane.trace[0].incoming_state.angular_rate_radps,
            incoming.angular_rate_radps
        );
        assert!(
            common_live_residuals(&lane.trace, &lane.trace)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn contact_probe_does_not_promote_out_of_domain_body_geometry() {
        let context = context();
        let template = SimulationState::new(&context).unwrap();
        let (pose, classification, matches) = core_reference_pose_probe(
            &context,
            &template,
            KinematicStateV2 {
                position_m: Vec2::new(99.0, 4.999),
                velocity_mps: Vec2::new(0.0, -1.0),
            },
            0.0,
            0.0,
            1,
        )
        .unwrap();
        assert!(matches);
        let contact = contact_audit(&context, &pose, &classification);
        assert!(!contact.body_within_strict_terrain_domain);
        assert!(!contact_is_stable_target(&contact));
    }
}
