//! New evaluator-only terminal policy. Old generators and ledgers are unchanged.
//! Reference poses are conditional geometry; only independent complete held-clock
//! command replay can accept a new nominal witness.

use super::super::super::{
    ThrottleSaturation, plant_applied_throttle, shortest_angle_delta, throttle_request,
};
use super::terminal_admissibility::{contact_audit, core_reference_pose_probe};
use super::*;
use crate::{
    WaypointDirectNominalDirectGenerationArtifact, WaypointDirectNominalDirectGenerationRequest,
    evaluate_waypoint_direct_nominal_direct_generation,
    validate_waypoint_direct_nominal_direct_generation_request,
};

const CASE_SCHEMA: &str = "waypoint_direct_body_aware_terminal_case_v1";
const POLICY_VERSION: &str = "body_aware_quadratic_terminal_held60_v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyAwareTerminalPolicyV1 {
    pub version: String,
    pub reference_family: String,
    pub duration_offsets_ticks: Vec<u64>,
    pub terminal_target_downward_speed_fraction: f64,
    pub contact_undershoot_m: f64,
    pub maximum_after_reference_ticks: u64,
    pub terminal_reference_extra_clearance_m: f64,
    pub scalar_inverse_iterations: u32,
    pub maximum_coast_clock_alignment_ticks: u64,
    pub geometry_convention: String,
}

impl Default for BodyAwareTerminalPolicyV1 {
    fn default() -> Self {
        Self {
            version: POLICY_VERSION.to_owned(),
            reference_family: "quadratic_horizontal_affine_vertical_exact_discrete".to_owned(),
            duration_offsets_ticks: vec![0, 60, 120, 180, 240, 300, 360],
            terminal_target_downward_speed_fraction: 0.5,
            contact_undershoot_m: 0.005,
            maximum_after_reference_ticks: 4,
            terminal_reference_extra_clearance_m: 0.1,
            scalar_inverse_iterations: 48,
            maximum_coast_clock_alignment_ticks: 1,
            geometry_convention: GEOMETRY_CONVENTION.to_owned(),
        }
    }
}

pub fn validate_body_aware_terminal_policy(policy: &BodyAwareTerminalPolicyV1) -> Result<()> {
    if *policy != BodyAwareTerminalPolicyV1::default() {
        bail!("unsupported body-aware terminal policy version or tuned field");
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalReferenceV1 {
    pub start: KinematicStateV2,
    pub end: KinematicStateV2,
    pub physics_ticks: u64,
    /// Acceleration c0 + c1*u + c2*u^2, u=j/(N-1).
    pub horizontal_coefficients_mps2: [f64; 3],
    pub initial_vertical_acceleration_mps2: f64,
    pub vertical_acceleration_delta_mps2: f64,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalReferenceAuditV1 {
    pub reference_identity: String,
    pub first_contact: Option<TerminalContactAuditEvidence>,
    pub clearance_scan: GeometryClearanceScanEvidence,
    pub minimum_outside_pad_clearance_m: Option<f64>,
    pub maximum_reference_slew_radps: f64,
    pub maximum_required_thrust_acceleration_mps2: f64,
    pub endpoint_position_error_m: f64,
    pub endpoint_velocity_error_mps: f64,
    pub final_horizontal_acceleration_mps2: f64,
    pub neutral_pose_parity_passed: bool,
    pub rejection_reasons: Vec<String>,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalCommandUpdateV1 {
    /// Post-step index to which this update applies; pre-step clock is index-1.
    pub physics_step: u64,
    pub phase: String,
    pub command: Command,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalVerificationV1 {
    pub ordinary_neutral_parity_passed: bool,
    pub generator_trace_parity_passed: bool,
    pub source_prefix_payload_matches: bool,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub actual_terminal_entry: Option<PlantStateEvidence>,
    pub first_contact: Option<TerminalContactAuditEvidence>,
    pub clearance_scan: GeometryClearanceScanEvidence,
    pub command_update_count: usize,
    pub physics_ticks_advanced: u64,
    pub maximum_airborne_reference_position_error_m: f64,
    pub maximum_airborne_reference_velocity_error_mps: f64,
    pub maximum_actual_slew_radps: f64,
    pub actual_fuel_used_kg: f64,
    pub planned_total_mission_time_s: f64,
    pub actual_contact_time_s: Option<f64>,
    pub rejection_reasons: Vec<String>,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalWitnessV1 {
    pub request_identity: String,
    pub terminal_policy_identity: String,
    pub baseline_generation_identity: String,
    pub row_index: usize,
    pub source_bridge_tick_count: u64,
    pub source_handoff_reference: KinematicStateV2,
    pub source_prefix_command_identity: String,
    pub coast_tick_count: u64,
    pub coast_clock_alignment_ticks: u64,
    pub reference: BodyAwareTerminalReferenceV1,
    pub terminal_entry: PlantStateEvidence,
    pub commands: Vec<BodyAwareTerminalCommandUpdateV1>,
    pub planned_physics_tick_count: u64,
    pub reference_audit: BodyAwareTerminalReferenceAuditV1,
    pub verification: BodyAwareTerminalVerificationV1,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalAttemptV1 {
    pub duration_offset_ticks: u64,
    pub terminal_tick_count: u64,
    pub status: String,
    pub first_failing_stage: Option<String>,
    pub reference_audit: Option<BodyAwareTerminalReferenceAuditV1>,
    pub verification: Option<BodyAwareTerminalVerificationV1>,
    pub witness_identity: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalRowV1 {
    pub row_index: usize,
    pub basis_index: usize,
    pub baseline_accepted: bool,
    pub baseline_wrapper_identity: Option<String>,
    pub source_status: String,
    pub source_skip_reason: Option<String>,
    pub attempts: Vec<BodyAwareTerminalAttemptV1>,
    pub accepted: bool,
    pub witness: Option<BodyAwareTerminalWitnessV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyAwareTerminalCaseArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub request: WaypointDirectNominalDirectGenerationRequest,
    pub terminal_policy: BodyAwareTerminalPolicyV1,
    pub baseline_generation_identity: String,
    pub baseline_generation_sha256: String,
    pub baseline_accepted_count: usize,
    pub baseline_scheduled_count: usize,
    pub rows: Vec<BodyAwareTerminalRowV1>,
    pub accepted_witness_count: usize,
    pub selected_row_index: Option<usize>,
    pub passed: bool,
    pub execution_status: String,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

pub fn body_aware_terminal_case_identity(
    artifact: &BodyAwareTerminalCaseArtifactV1,
) -> Result<String> {
    let mut canonical = artifact.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

fn reference_identity(reference: &BodyAwareTerminalReferenceV1) -> Result<String> {
    let mut canonical = reference.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

fn witness_identity(witness: &BodyAwareTerminalWitnessV1) -> Result<String> {
    let mut canonical = witness.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

fn solve_three(mut a: [[f64; 4]; 3]) -> Result<[f64; 3]> {
    for column in 0..3 {
        let pivot = (column..3)
            .max_by(|left, right| a[*left][column].abs().total_cmp(&a[*right][column].abs()))
            .context("quadratic constraint pivot missing")?;
        a.swap(column, pivot);
        let scale = a[column][column];
        if !scale.is_finite() || scale.abs() <= 1.0e-12 {
            bail!("singular quadratic terminal constraints");
        }
        for value in &mut a[column][column..] {
            *value /= scale;
        }
        let pivot_values = a[column];
        for (row, values) in a.iter_mut().enumerate() {
            if row == column {
                continue;
            }
            let factor = values[column];
            for (value, pivot_value) in values[column..].iter_mut().zip(&pivot_values[column..]) {
                *value -= factor * pivot_value;
            }
        }
    }
    let result = [a[0][3], a[1][3], a[2][3]];
    if result.iter().any(|value| !value.is_finite()) {
        bail!("non-finite quadratic terminal coefficients");
    }
    Ok(result)
}

fn build_reference(
    context: &RunContext,
    policy: &BodyAwareTerminalPolicyV1,
    start: KinematicStateV2,
    steps: u64,
) -> Result<BodyAwareTerminalReferenceV1> {
    if steps < 2 || !steps.is_multiple_of(2) {
        bail!("terminal reference requires a positive complete two-tick duration");
    }
    let end = KinematicStateV2 {
        position_m: Vec2::new(
            context.target_pad.center_x_m,
            context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m
                - policy.contact_undershoot_m,
        ),
        velocity_mps: Vec2::new(
            0.0,
            -policy.terminal_target_downward_speed_fraction
                * context.vehicle.safe_touchdown_normal_speed_mps,
        ),
    };
    let dt = context.sim.physics_dt_s();
    let n = steps as f64;
    let sj = n * (n - 1.0) / 2.0;
    let sj2 = n * (n - 1.0) * (2.0 * n - 1.0) / 6.0;
    let sj3 = sj * sj;
    let horizontal = solve_three([
        [
            dt * n,
            dt * sj / (n - 1.0),
            dt * sj2 / (n - 1.0).powi(2),
            end.velocity_mps.x - start.velocity_mps.x,
        ],
        [
            dt * dt * n * (n + 1.0) / 2.0,
            dt * dt * (n * sj - sj2) / (n - 1.0),
            dt * dt * (n * sj2 - sj3) / (n - 1.0).powi(2),
            end.position_m.x - start.position_m.x - start.velocity_mps.x * n * dt,
        ],
        [1.0, 1.0, 1.0, 0.0],
    ])?;
    let d = (end.velocity_mps.y - start.velocity_mps.y) / dt;
    let s = (end.position_m.y - start.position_m.y - start.velocity_mps.y * n * dt) / (dt * dt);
    let mut reference = BodyAwareTerminalReferenceV1 {
        start,
        end,
        physics_ticks: steps,
        horizontal_coefficients_mps2: horizontal,
        initial_vertical_acceleration_mps2: 6.0 * s / (n * (n + 1.0)) - 2.0 * d / n,
        vertical_acceleration_delta_mps2: 6.0 * d / (n * (n - 1.0))
            - 12.0 * s / (n * (n + 1.0) * (n - 1.0)),
        identity: String::new(),
    };
    reference.identity = reference_identity(&reference)?;
    Ok(reference)
}

impl BodyAwareTerminalReferenceV1 {
    fn net_acceleration(&self, tick: u64) -> Vec2 {
        if tick >= self.physics_ticks {
            return Vec2::new(
                0.0,
                self.initial_vertical_acceleration_mps2
                    + self.vertical_acceleration_delta_mps2 * (self.physics_ticks - 1) as f64,
            );
        }
        let u = tick as f64 / (self.physics_ticks - 1) as f64;
        let c = self.horizontal_coefficients_mps2;
        Vec2::new(
            c[0] + c[1] * u + c[2] * u * u,
            self.initial_vertical_acceleration_mps2
                + self.vertical_acceleration_delta_mps2 * tick as f64,
        )
    }

    fn thrust(&self, context: &RunContext, tick: u64) -> Vec2 {
        self.net_acceleration(tick) + Vec2::new(0.0, context.world.gravity_mps2)
    }

    fn state_at(&self, context: &RunContext, steps: u64) -> KinematicStateV2 {
        let dt = context.sim.physics_dt_s();
        if steps > self.physics_ticks {
            let end = self.state_at(context, self.physics_ticks);
            let k = (steps - self.physics_ticks) as f64;
            let a = self.net_acceleration(self.physics_ticks);
            return KinematicStateV2 {
                position_m: end.position_m
                    + end.velocity_mps * (k * dt)
                    + a * (dt * dt * k * (k + 1.0) / 2.0),
                velocity_mps: end.velocity_mps + a * (k * dt),
            };
        }
        let k = steps as f64;
        let sj = k * (k - 1.0) / 2.0;
        let sj2 = k * (k - 1.0) * (2.0 * k - 1.0) / 6.0;
        let sj3 = sj * sj;
        let n1 = (self.physics_ticks - 1) as f64;
        let c = self.horizontal_coefficients_mps2;
        let vx =
            self.start.velocity_mps.x + dt * (c[0] * k + c[1] * sj / n1 + c[2] * sj2 / (n1 * n1));
        let x = self.start.position_m.x
            + self.start.velocity_mps.x * k * dt
            + dt * dt
                * (c[0] * k * (k + 1.0) / 2.0
                    + c[1] * (k * sj - sj2) / n1
                    + c[2] * (k * sj2 - sj3) / (n1 * n1));
        let ay = self.initial_vertical_acceleration_mps2;
        let day = self.vertical_acceleration_delta_mps2;
        KinematicStateV2 {
            position_m: Vec2::new(
                x,
                self.start.position_m.y
                    + self.start.velocity_mps.y * k * dt
                    + dt * dt * (ay * k * (k + 1.0) / 2.0 + day * k * (k + 1.0) * (k - 1.0) / 6.0),
            ),
            velocity_mps: Vec2::new(
                vx,
                self.start.velocity_mps.y + dt * (ay * k + day * k * (k - 1.0) / 2.0),
            ),
        }
    }
}

fn paired_mean(reference: &BodyAwareTerminalReferenceV1, context: &RunContext, tick: u64) -> Vec2 {
    (reference.thrust(context, tick) + reference.thrust(context, tick + 1)) * 0.5
}

/// Invert the held throttle's mean acceleration over both post-burn masses.
/// This uses no core transition and is independently checked by physical replay.
fn paired_throttle(
    context: &RunContext,
    policy: &BodyAwareTerminalPolicyV1,
    mass: f64,
    thrust: f64,
) -> Result<f64> {
    if !mass.is_finite() || mass <= 0.0 || !thrust.is_finite() || thrust < 0.0 {
        bail!("invalid paired throttle input");
    }
    if thrust == 0.0 {
        return Ok(0.0);
    }
    let burn = context.vehicle.max_fuel_burn_kgps * context.sim.physics_dt_s();
    let max_thrust = context.vehicle.max_thrust_n;
    let mean = |q: f64| -> f64 {
        (max_thrust * q / (mass - burn * q) + max_thrust * q / (mass - 2.0 * burn * q)) * 0.5
    };
    if mass <= 2.0 * burn || thrust > mean(1.0) {
        bail!("paired throttle above maximum or unsupported remaining mass");
    }
    let minimum = context.vehicle.min_throttle_frac;
    if thrust < mean(minimum) {
        bail!("paired throttle below minimum");
    }
    let (mut low, mut high) = (minimum, 1.0);
    for _ in 0..policy.scalar_inverse_iterations {
        let mid = (low + high) * 0.5;
        if mean(mid) < thrust {
            low = mid;
        } else {
            high = mid;
        }
    }
    let applied = (low + high) * 0.5;
    if applied <= minimum {
        Ok(f64::MIN_POSITIVE)
    } else {
        Ok((applied - minimum) / (1.0 - minimum))
    }
}

pub(super) fn clearance_policy(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
) -> Result<ClearancePolicy> {
    let bounds = |id: &str| -> Result<FlatPadBounds> {
        let pad = request
            .scenario
            .world
            .landing_pad(id)
            .context("body-aware pad missing")?;
        Ok(flat_pad_bounds(
            context,
            &PadInputV2 {
                center_x_m: pad.center_x_m,
                surface_y_m: pad.surface_y_m,
                width_m: pad.width_m,
            },
        ))
    };
    Ok(ClearancePolicy {
        source_pad: bounds(&request.source_pad_id)?,
        target_pad: bounds(&request.target_pad_id)?,
        minimum_clearance_m: request.policy.analytical_policy.minimum_clearance_m,
    })
}

fn audit_reference(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
    entry: &SimulationState,
    reference: &BodyAwareTerminalReferenceV1,
) -> Result<BodyAwareTerminalReferenceAuditV1> {
    let mut scan = empty_clearance_scan();
    let mut cp = clearance_policy(context, request)?;
    cp.minimum_clearance_m += policy.terminal_reference_extra_clearance_m;
    let mut previous_angle = entry.attitude_rad;
    let mut contact = None;
    let mut pose_parity = true;
    let mut max_slew = 0.0_f64;
    let mut max_thrust = 0.0_f64;
    let mut minimum_outside = None::<f64>;
    let mut minimum_throttle_passed = true;
    let mut fuel = entry.fuel_kg;
    for tick in 0..reference.physics_ticks + policy.maximum_after_reference_ticks {
        let thrust = reference.thrust(context, tick);
        let angle = thrust.x.atan2(thrust.y);
        let rate = shortest_angle_delta(previous_angle, angle) / context.sim.physics_dt_s();
        previous_angle = angle;
        max_slew = max_slew.max(rate.abs());
        max_thrust = max_thrust.max(thrust.length());
        let throttle = throttle_request(
            thrust.length(),
            context.vehicle.dry_mass_kg + fuel,
            context.vehicle.max_thrust_n,
            context.vehicle.max_fuel_burn_kgps,
            context.sim.physics_dt_s(),
            context.vehicle.min_throttle_frac,
        )?;
        minimum_throttle_passed &= !matches!(
            throttle.saturation,
            ThrottleSaturation::BelowMinimum | ThrottleSaturation::AboveMaximum
        );
        fuel -= throttle.applied_fraction
            * context.vehicle.max_fuel_burn_kgps
            * context.sim.physics_dt_s();
        let (pose, classification, matches) = core_reference_pose_probe(
            context,
            entry,
            reference.state_at(context, tick + 1),
            angle,
            rate,
            entry.physics_step + tick + 1,
        )?;
        pose_parity &= matches;
        scan.poststep_state_count += 1;
        let audit = contact_audit(context, &pose, &classification);
        pose_parity &=
            audit.core_matches_predicate_mirror && audit.body_within_strict_terrain_domain;
        if classification != ContactClassification::None {
            contact = Some(audit);
            break;
        }
        record_airborne_clearance(
            context,
            &pose,
            pose.physics_step,
            "terminal_bridge",
            cp,
            &mut scan,
        );
        let aabb = body_aabb(&pose, &context.vehicle.geometry);
        if corridor_for_step(
            "terminal_bridge",
            pose.velocity_mps,
            aabb,
            cp.source_pad,
            cp.target_pad,
        ) == "none"
            && let Ok(clearance) = body_clearance(context, &pose, aabb)
        {
            minimum_outside = Some(minimum_outside.map_or(clearance, |old| old.min(clearance)));
        }
    }
    let endpoint = reference.state_at(context, reference.physics_ticks);
    let p_error = distance(endpoint.position_m, reference.end.position_m);
    let v_error = distance(endpoint.velocity_mps, reference.end.velocity_mps);
    let final_ax = reference.net_acceleration(reference.physics_ticks - 1).x;
    let ap = &request.policy.analytical_policy;
    let max_allowed = context.vehicle.max_thrust_n
        / (context.vehicle.dry_mass_kg + context.vehicle.initial_fuel_kg)
        * ap.thrust_derate
        * (1.0 - ap.declared_robustness_margin);
    let mut reasons = Vec::new();
    if !pose_parity {
        reasons.push("reference_core_pose_or_predicate_parity".to_owned());
    }
    if p_error > 1.0e-6 || v_error > 1.0e-6 || final_ax.abs() > 1.0e-9 {
        reasons.push("reference_endpoint_constraints".to_owned());
    }
    if max_slew > context.vehicle.max_rotation_rate_radps {
        reasons.push("reference_slew".to_owned());
    }
    if max_thrust > max_allowed {
        reasons.push("reference_derated_thrust".to_owned());
    }
    if !minimum_throttle_passed || fuel < 0.0 {
        reasons.push("reference_throttle_or_fuel".to_owned());
    }
    if !scan.all_airborne_states_passed {
        reasons.push("reference_airborne_clearance".to_owned());
    }
    if !contact.as_ref().is_some_and(|audit| {
        audit.classification == "stable_touchdown_on_target"
            && stable_safe_margins_pass(&audit.margins)
    }) {
        reasons.push("reference_first_contact".to_owned());
    }
    Ok(BodyAwareTerminalReferenceAuditV1 {
        reference_identity: reference.identity.clone(),
        first_contact: contact,
        clearance_scan: scan,
        minimum_outside_pad_clearance_m: minimum_outside,
        maximum_reference_slew_radps: max_slew,
        maximum_required_thrust_acceleration_mps2: max_thrust,
        endpoint_position_error_m: p_error,
        endpoint_velocity_error_mps: v_error,
        final_horizontal_acceleration_mps2: final_ax,
        neutral_pose_parity_passed: pose_parity,
        passed: reasons.is_empty(),
        rejection_reasons: reasons,
    })
}

struct ProgramFrame {
    state: SimulationState,
    classification: ContactClassification,
}

struct SourcePrefix {
    updates: Vec<BodyAwareTerminalCommandUpdateV1>,
    frames: Vec<ProgramFrame>,
    end: SimulationState,
    source_handoff: KinematicStateV2,
    source_bridge_ticks: u64,
}

fn extract_source_prefix(
    context: &RunContext,
    generation: &WaypointDirectNominalDirectGenerationArtifact,
    row_index: usize,
) -> Result<Option<SourcePrefix>> {
    let row = &generation.rows[row_index];
    let Some(schedule) = row.paired_schedule.as_ref() else {
        return Ok(None);
    };
    if !schedule.witness
        || !schedule.screens.strict_position_passed
        || !schedule.screens.strict_velocity_passed
    {
        return Ok(None);
    }
    let analysis = row
        .source_duration
        .as_ref()
        .and_then(|source| source.launch_and_analytical_screen.as_ref())
        .context("generated source schedule missing launch analysis")?;
    let source_ticks = row
        .source_bridge_tick_count
        .context("generated source duration missing")?;
    let handoff = row
        .source_handoff
        .context("generated source handoff missing")?;
    let rollout = schedule
        .full_flight_rollout
        .as_ref()
        .context("generated source rollout missing")?;
    let source_logs = rollout
        .per_step
        .iter()
        .filter(|tick| tick.phase == "source_bridge")
        .collect::<Vec<_>>();
    if analysis.launch.samples.len() as u64 != LAUNCH_TICKS
        || source_logs.len() as u64 != source_ticks
    {
        bail!("generated launch/source prefix is incomplete");
    }
    let mut state = SimulationState::new(context)?;
    let mut updates = Vec::new();
    let mut frames = Vec::new();
    for (step, phase, throttle, target, expected_kinematics, expected_fuel) in analysis
        .launch
        .samples
        .iter()
        .map(|tick| {
            (
                tick.physics_step,
                tick.phase.as_str(),
                tick.commanded_throttle_frac,
                tick.held_target_attitude_rad,
                Some((tick.position_m, tick.velocity_mps)),
                tick.fuel_kg,
            )
        })
        .chain(source_logs.into_iter().map(|tick| {
            (
                tick.physics_step,
                tick.phase.as_str(),
                tick.commanded_throttle_frac,
                tick.held_target_attitude_rad,
                None,
                tick.fuel_kg,
            )
        }))
    {
        if step != state.physics_step + 1 {
            bail!("generated source prefix has a gap or reordered physics tick");
        }
        let command = Command {
            throttle_frac: throttle,
            target_attitude_rad: target,
        };
        if state
            .physics_step
            .is_multiple_of(context.sim.control_interval_steps())
        {
            state.set_command(command);
            updates.push(BodyAwareTerminalCommandUpdateV1 {
                physics_step: step,
                phase: phase.to_owned(),
                command,
            });
        } else if state.held_command != command {
            bail!("generated source prefix changes its held command off-clock");
        }
        let classification = state.step_physics_and_classify_contact(context);
        if classification != ContactClassification::None
            || expected_kinematics.is_some_and(|(position, velocity)| {
                state.position_m != position || state.velocity_mps != velocity
            })
            || state.fuel_kg != expected_fuel
        {
            bail!("generated source prefix does not reproduce its independent physical state");
        }
        frames.push(ProgramFrame {
            state: state.clone(),
            classification,
        });
    }
    if distance(state.position_m, handoff.position_m) > STRICT_HANDOFF_TOLERANCE_M
        || distance(state.velocity_mps, handoff.velocity_mps) > STRICT_HANDOFF_TOLERANCE_MPS
    {
        return Ok(None);
    }
    let fitted_endpoint = schedule
        .endpoint_state
        .context("fitted source endpoint missing")?;
    if state.position_m != fitted_endpoint.position_m
        || state.velocity_mps != fitted_endpoint.velocity_mps
        || state.fuel_kg != fitted_endpoint.fuel_kg
    {
        bail!("replayed source prefix differs from the independently fitted source endpoint");
    }
    Ok(Some(SourcePrefix {
        updates,
        frames,
        end: state,
        source_handoff: handoff,
        source_bridge_ticks: source_ticks,
    }))
}

fn predicted_coast_kinematics(
    context: &RunContext,
    source: &SimulationState,
    ticks: u64,
) -> KinematicStateV2 {
    let mut state = KinematicStateV2 {
        position_m: source.position_m,
        velocity_mps: source.velocity_mps,
    };
    for _ in 0..ticks {
        state.velocity_mps.y -= context.world.gravity_mps2 * context.sim.physics_dt_s();
        state.position_m += state.velocity_mps * context.sim.physics_dt_s();
    }
    state
}

fn coast_program(
    context: &RunContext,
    prefix: &SourcePrefix,
    ticks: u64,
    target: f64,
) -> (
    Vec<BodyAwareTerminalCommandUpdateV1>,
    Vec<ProgramFrame>,
    SimulationState,
) {
    let mut state = prefix.end.clone();
    let mut updates = Vec::new();
    let mut frames = Vec::new();
    for _ in 0..ticks {
        if state
            .physics_step
            .is_multiple_of(context.sim.control_interval_steps())
        {
            let command = Command {
                throttle_frac: 0.0,
                target_attitude_rad: target,
            };
            state.set_command(command);
            updates.push(BodyAwareTerminalCommandUpdateV1 {
                physics_step: state.physics_step + 1,
                phase: "ballistic_coast".to_owned(),
                command,
            });
        }
        let classification = state.step_physics_and_classify_contact(context);
        frames.push(ProgramFrame {
            state: state.clone(),
            classification: classification.clone(),
        });
        if classification != ContactClassification::None {
            break;
        }
    }
    (updates, frames, state)
}

fn terminal_program(
    context: &RunContext,
    policy: &BodyAwareTerminalPolicyV1,
    entry: &SimulationState,
    reference: &BodyAwareTerminalReferenceV1,
) -> Result<(Vec<BodyAwareTerminalCommandUpdateV1>, Vec<ProgramFrame>)> {
    if !entry
        .physics_step
        .is_multiple_of(context.sim.control_interval_steps())
    {
        bail!("powered terminal program starts off the real global clock");
    }
    let mut state = entry.clone();
    let mut updates = Vec::new();
    let mut frames = Vec::new();
    for tick in 0..reference.physics_ticks + policy.maximum_after_reference_ticks {
        if state
            .physics_step
            .is_multiple_of(context.sim.control_interval_steps())
        {
            let thrust = paired_mean(reference, context, tick);
            let throttle =
                paired_throttle(context, policy, state.mass_kg(context), thrust.length())?;
            let command = Command {
                throttle_frac: throttle,
                target_attitude_rad: thrust.x.atan2(thrust.y),
            };
            state.set_command(command);
            updates.push(BodyAwareTerminalCommandUpdateV1 {
                physics_step: state.physics_step + 1,
                phase: "terminal_bridge".to_owned(),
                command,
            });
        }
        let classification = state.step_physics_and_classify_contact(context);
        frames.push(ProgramFrame {
            state: state.clone(),
            classification: classification.clone(),
        });
        if classification != ContactClassification::None {
            break;
        }
    }
    Ok((updates, frames))
}

fn empty_verification(context: &RunContext, planned_steps: u64) -> BodyAwareTerminalVerificationV1 {
    BodyAwareTerminalVerificationV1 {
        ordinary_neutral_parity_passed: true,
        generator_trace_parity_passed: true,
        source_prefix_payload_matches: false,
        source_handoff_position_error_m: None,
        source_handoff_velocity_error_mps: None,
        actual_terminal_entry: None,
        first_contact: None,
        clearance_scan: empty_clearance_scan(),
        command_update_count: 0,
        physics_ticks_advanced: 0,
        maximum_airborne_reference_position_error_m: 0.0,
        maximum_airborne_reference_velocity_error_mps: 0.0,
        maximum_actual_slew_radps: 0.0,
        actual_fuel_used_kg: 0.0,
        planned_total_mission_time_s: planned_steps as f64 / f64::from(context.sim.physics_hz),
        actual_contact_time_s: None,
        rejection_reasons: Vec::new(),
        passed: false,
    }
}

pub(super) fn phase_at(step: u64, source_end: u64, terminal_start: u64) -> &'static str {
    if step <= 60 {
        "upright"
    } else if step <= LAUNCH_TICKS {
        "tilt"
    } else if step <= source_end {
        "source_bridge"
    } else if step <= terminal_start {
        "ballistic_coast"
    } else {
        "terminal_bridge"
    }
}

fn verify_program(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
    witness: &BodyAwareTerminalWitnessV1,
    expected_frames: Option<&[ProgramFrame]>,
) -> Result<BodyAwareTerminalVerificationV1> {
    let source_end = LAUNCH_TICKS
        .checked_add(witness.source_bridge_tick_count)
        .context("source duration overflow")?;
    let terminal_start = source_end
        .checked_add(witness.coast_tick_count)
        .context("coast duration overflow")?;
    let expected_end = terminal_start
        .checked_add(witness.reference.physics_ticks)
        .and_then(|step| step.checked_add(policy.maximum_after_reference_ticks))
        .context("terminal duration overflow")?;
    let mut v = empty_verification(context, witness.planned_physics_tick_count);
    let prefix = witness
        .commands
        .iter()
        .filter(|update| update.physics_step <= source_end)
        .cloned()
        .collect::<Vec<_>>();
    v.source_prefix_payload_matches =
        stable_digest(&prefix)? == witness.source_prefix_command_identity;
    let clock_ok = witness.commands.iter().enumerate().all(|(index, update)| {
        update.physics_step == 1 + (index as u64) * context.sim.control_interval_steps()
            && update.phase == phase_at(update.physics_step, source_end, terminal_start)
            && update.command.throttle_frac.is_finite()
            && (0.0..=1.0).contains(&update.command.throttle_frac)
            && update.command.target_attitude_rad.is_finite()
    });
    let planned_ok = witness.planned_physics_tick_count == expected_end
        && v.planned_total_mission_time_s <= request.policy.analytical_policy.mission_budget_s()
        && v.planned_total_mission_time_s <= context.sim.max_time_s
        && terminal_start.is_multiple_of(context.sim.control_interval_steps())
        && witness.coast_clock_alignment_ticks <= policy.maximum_coast_clock_alignment_ticks;
    let reference = build_reference(
        context,
        policy,
        witness.reference.start,
        witness.reference.physics_ticks,
    )?;
    let bindings_ok = reference == witness.reference
        && witness.request_identity == stable_digest(request)?
        && witness.terminal_policy_identity == stable_digest(policy)?
        && witness.terminal_entry.physics_step == terminal_start;
    if !clock_ok {
        v.rejection_reasons
            .push("global_clock_command_coverage_or_payload".to_owned());
    }
    if !planned_ok {
        v.rejection_reasons
            .push("planned_time_or_phase_bounds".to_owned());
    }
    if !bindings_ok {
        v.rejection_reasons
            .push("reference_request_or_entry_binding".to_owned());
    }
    if !v.source_prefix_payload_matches {
        v.rejection_reasons.push("source_prefix_payload".to_owned());
    }
    if !clock_ok || !planned_ok || !bindings_ok || !v.source_prefix_payload_matches {
        return Ok(v);
    }
    let cp = clearance_policy(context, request)?;
    let mut ordinary = SimulationState::new(context)?;
    let mut neutral = SimulationState::new(context)?;
    let mut next_update = 0;
    let mut entry_ok = false;
    let mut join_ok = true;
    let mut actuator_ok = true;
    let mut reference_audit_ok = false;
    let mut command_reference_ok = true;
    for _ in 0..witness.planned_physics_tick_count {
        if neutral.physics_step == terminal_start {
            let observed = plant_state_evidence(&neutral, context);
            v.actual_terminal_entry = Some(observed.clone());
            entry_ok = observed == witness.terminal_entry
                && neutral.position_m == reference.start.position_m
                && neutral.velocity_mps == reference.start.velocity_mps
                && neutral.velocity_mps.y < 0.0
                && body_points_clear(context, &neutral)?;
            let audit = audit_reference(context, request, policy, &neutral, &reference)?;
            reference_audit_ok = audit.passed && audit == witness.reference_audit;
        }
        let step = neutral.physics_step + 1;
        let phase = phase_at(step, source_end, terminal_start);
        if neutral
            .physics_step
            .is_multiple_of(context.sim.control_interval_steps())
        {
            let Some(update) = witness.commands.get(next_update) else {
                v.rejection_reasons
                    .push("truncated_global_command_program".to_owned());
                break;
            };
            if update.physics_step != step {
                v.rejection_reasons
                    .push("global_command_step_mismatch".to_owned());
                break;
            }
            if phase == "ballistic_coast" {
                let mean = paired_mean(&reference, context, 0);
                command_reference_ok &= update.command.throttle_frac == 0.0
                    && update.command.target_attitude_rad == mean.x.atan2(mean.y);
            } else if phase == "terminal_bridge" {
                let tick = neutral.physics_step - terminal_start;
                let mean = paired_mean(&reference, context, tick);
                match paired_throttle(context, policy, neutral.mass_kg(context), mean.length()) {
                    Ok(throttle) => {
                        command_reference_ok &= update.command
                            == Command {
                                throttle_frac: throttle,
                                target_attitude_rad: mean.x.atan2(mean.y),
                            }
                    }
                    Err(_) => command_reference_ok = false,
                }
                join_ok &=
                    shortest_angle_delta(neutral.attitude_rad, update.command.target_attitude_rad)
                        .abs()
                        <= context.vehicle.max_rotation_rate_radps * context.sim.physics_dt_s()
                            + 1.0e-12;
            }
            ordinary.set_command(update.command);
            neutral.set_command(update.command);
            next_update += 1;
        }
        let applied = plant_applied_throttle(
            neutral.held_command,
            context.vehicle.min_throttle_frac,
            neutral.fuel_kg,
        );
        let expected_burn =
            applied * context.vehicle.max_fuel_burn_kgps * context.sim.physics_dt_s();
        actuator_ok &= neutral.fuel_kg >= expected_burn
            && (neutral.held_command.throttle_frac == 0.0 || neutral.fuel_kg > 0.0)
            && (applied == 0.0 || applied >= context.vehicle.min_throttle_frac)
            && applied <= 1.0;
        let classification = neutral.step_physics_and_classify_contact(context);
        let events = ordinary.step(context);
        v.ordinary_neutral_parity_passed &=
            same_ordinary_neutral_state(&ordinary, &neutral, &classification)
                && event_contact_label(&events) == contact_classification_label(&classification);
        if let Some(frames) = expected_frames {
            v.generator_trace_parity_passed &=
                frames.get((step - 1) as usize).is_some_and(|frame| {
                    frame.classification == classification
                        && frame.state.position_m == neutral.position_m
                        && frame.state.velocity_mps == neutral.velocity_mps
                        && frame.state.attitude_rad == neutral.attitude_rad
                        && frame.state.angular_rate_radps == neutral.angular_rate_radps
                        && frame.state.fuel_kg == neutral.fuel_kg
                        && frame.state.held_command == neutral.held_command
                        && frame.state.physics_step == neutral.physics_step
                        && frame.state.sim_time_s == neutral.sim_time_s
                });
        }
        v.physics_ticks_advanced = step;
        v.maximum_actual_slew_radps = v
            .maximum_actual_slew_radps
            .max(neutral.angular_rate_radps.abs());
        v.clearance_scan.poststep_state_count += 1;
        if step == source_end {
            v.source_handoff_position_error_m = Some(distance(
                neutral.position_m,
                witness.source_handoff_reference.position_m,
            ));
            v.source_handoff_velocity_error_mps = Some(distance(
                neutral.velocity_mps,
                witness.source_handoff_reference.velocity_mps,
            ));
        }
        if classification != ContactClassification::None {
            v.first_contact = Some(contact_audit(context, &neutral, &classification));
            v.actual_contact_time_s = Some(neutral.sim_time_s);
            break;
        }
        record_airborne_clearance(context, &neutral, step, phase, cp, &mut v.clearance_scan);
        if phase == "terminal_bridge" {
            let expected = reference.state_at(context, step - terminal_start);
            v.maximum_airborne_reference_position_error_m = v
                .maximum_airborne_reference_position_error_m
                .max(distance(neutral.position_m, expected.position_m));
            v.maximum_airborne_reference_velocity_error_mps = v
                .maximum_airborne_reference_velocity_error_mps
                .max(distance(neutral.velocity_mps, expected.velocity_mps));
        }
    }
    v.command_update_count = next_update;
    v.actual_fuel_used_kg = context.vehicle.initial_fuel_kg - neutral.fuel_kg;
    let source_ok = v
        .source_handoff_position_error_m
        .is_some_and(|error| error <= STRICT_HANDOFF_TOLERANCE_M)
        && v.source_handoff_velocity_error_mps
            .is_some_and(|error| error <= STRICT_HANDOFF_TOLERANCE_MPS);
    let stable = v.first_contact.as_ref().is_some_and(|audit| {
        audit.classification == "stable_touchdown_on_target"
            && audit.state.physics_step > terminal_start
            && audit.core_matches_predicate_mirror
            && audit.body_within_strict_terrain_domain
            && stable_safe_margins_pass(&audit.margins)
    });
    let budget_ok = neutral.fuel_kg >= 0.0
        && v.actual_contact_time_s.is_some_and(|time| {
            time <= request.policy.analytical_policy.mission_budget_s()
                && time <= context.sim.max_time_s
        })
        && v.maximum_actual_slew_radps <= context.vehicle.max_rotation_rate_radps + 1.0e-12;
    let frame_coverage =
        expected_frames.is_none_or(|frames| frames.len() as u64 == v.physics_ticks_advanced);
    for (passes, reason) in [
        (
            v.ordinary_neutral_parity_passed,
            "ordinary_neutral_contact_parity",
        ),
        (
            v.generator_trace_parity_passed && frame_coverage,
            "generated_program_trace_parity",
        ),
        (
            next_update == witness.commands.len(),
            "unused_or_missing_command_updates",
        ),
        (source_ok, "strict_source_handoff"),
        (entry_ok, "actual_descending_clear_terminal_entry"),
        (
            reference_audit_ok,
            "independent_reference_first_contact_contract",
        ),
        (join_ok, "actual_powered_attitude_join"),
        (actuator_ok, "actual_actuator_or_fuel_limits"),
        (
            command_reference_ok,
            "held_command_reference_inverse_binding",
        ),
        (
            v.clearance_scan.all_airborne_states_passed,
            "pointwise_body_clearance",
        ),
        (stable, "first_contact_stable_safe_on_target"),
        (budget_ok, "actual_fuel_time_or_slew_budgets"),
    ] {
        if !passes {
            v.rejection_reasons.push(reason.to_owned());
        }
    }
    v.passed = v.rejection_reasons.is_empty();
    Ok(v)
}

/// Independently replay a serialized witness. Neither saved passed flags nor
/// saved contact/entry states decide acceptance; they must match recomputation.
pub fn verify_body_aware_terminal_witness(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
    witness: &BodyAwareTerminalWitnessV1,
) -> Result<BodyAwareTerminalVerificationV1> {
    validate_waypoint_direct_nominal_direct_generation_request(request)?;
    validate_body_aware_terminal_policy(policy)?;
    if witness_identity(witness)? != witness.identity {
        bail!("body-aware witness identity mismatch");
    }
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let verification = replay_saved_witness(&context, request, policy, witness)?;
    let baseline = evaluate_waypoint_direct_nominal_direct_generation(request)?;
    verify_source_binding(&context, policy, witness, &baseline)?;
    Ok(verification)
}

fn replay_saved_witness(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
    witness: &BodyAwareTerminalWitnessV1,
) -> Result<BodyAwareTerminalVerificationV1> {
    if witness_identity(witness)? != witness.identity {
        bail!("body-aware witness identity mismatch");
    }
    let verification = verify_program(context, request, policy, witness, None)?;
    if verification != witness.verification {
        bail!("saved body-aware verifier evidence does not reproduce");
    }
    Ok(verification)
}

fn verify_source_binding(
    context: &RunContext,
    policy: &BodyAwareTerminalPolicyV1,
    witness: &BodyAwareTerminalWitnessV1,
    baseline: &WaypointDirectNominalDirectGenerationArtifact,
) -> Result<()> {
    let row = baseline
        .rows
        .get(witness.row_index)
        .context("witness source row out of bounds")?;
    let prefix = extract_source_prefix(context, baseline, witness.row_index)?
        .context("witness source row has no strict supported prefix")?;
    let basis = baseline
        .bases
        .get(row.basis_index)
        .context("witness basis out of bounds")?;
    let seed_coast = basis
        .coast_tick_count
        .context("witness seed coast missing")?;
    let seed_terminal = basis
        .terminal_bridge_tick_count
        .context("witness seed terminal missing")?;
    let interval = context.sim.control_interval_steps();
    let alignment = (interval - (prefix.end.physics_step + seed_coast) % interval) % interval;
    let saved_prefix = witness
        .commands
        .iter()
        .filter(|update| update.physics_step <= prefix.end.physics_step)
        .cloned()
        .collect::<Vec<_>>();
    if witness.baseline_generation_identity != baseline.identity
        || witness.source_bridge_tick_count != prefix.source_bridge_ticks
        || witness.source_handoff_reference != prefix.source_handoff
        || witness.source_prefix_command_identity != stable_digest(&prefix.updates)?
        || saved_prefix != prefix.updates
        || witness.coast_tick_count != seed_coast + alignment
        || witness.coast_clock_alignment_ticks != alignment
        || !policy.duration_offsets_ticks.iter().any(|offset| {
            seed_terminal.checked_add(*offset) == Some(witness.reference.physics_ticks)
        })
    {
        bail!("witness differs from unchanged input-generated source, coast or duration family");
    }
    Ok(())
}

/// Regenerate the complete finite first-success ledger from inputs, then
/// independently replay every accepted serialized witness. Rejected diagnostic
/// attempts remain reproducible generator evidence, not accepted physical flights.
pub fn verify_body_aware_terminal_case(artifact: &BodyAwareTerminalCaseArtifactV1) -> Result<()> {
    validate_waypoint_direct_nominal_direct_generation_request(&artifact.request)?;
    validate_body_aware_terminal_policy(&artifact.terminal_policy)?;
    if artifact.schema_id != CASE_SCHEMA
        || artifact.schema_version != 1
        || body_aware_terminal_case_identity(artifact)? != artifact.identity
    {
        bail!("body-aware case schema or semantic identity mismatch");
    }
    let baseline = evaluate_waypoint_direct_nominal_direct_generation(&artifact.request)?;
    let baseline_sha = crate::waypoint_direct_body_aware_terminal::sha256_bytes(
        &serde_json::to_vec_pretty(&baseline)?,
    )?;
    if artifact.baseline_generation_identity != baseline.identity
        || artifact.baseline_generation_sha256 != baseline_sha
        || artifact.baseline_accepted_count != baseline.family_proof.accepted_witness_count
        || artifact.baseline_scheduled_count != baseline.family_proof.scheduled_count
        || artifact.rows.len() != baseline.rows.len()
    {
        bail!("body-aware case unchanged-generator provenance mismatch");
    }
    let regenerated =
        evaluate_terminal_family(&artifact.request, &artifact.terminal_policy, &baseline)?;
    if artifact != &regenerated {
        bail!("body-aware complete finite ledger or first-success ranking does not reproduce");
    }
    let context =
        RunContext::from_scenario(&artifact.request.scenario).map_err(anyhow::Error::msg)?;
    for (row, old) in artifact.rows.iter().zip(&baseline.rows) {
        if row.row_index != old.row_index
            || row.basis_index != old.basis_index
            || row.baseline_accepted != old.accepted
            || row.baseline_wrapper_identity
                != old.wrapper.as_ref().map(|w| w.wrapper_identity.clone())
            || row.accepted != row.witness.is_some()
            || row.accepted
                != row
                    .attempts
                    .iter()
                    .any(|a| a.status == "complete_witness_accepted")
        {
            bail!("body-aware case row ledger mismatch at {}", row.row_index);
        }
        let prefix = extract_source_prefix(&context, &baseline, row.row_index)?;
        if prefix.is_none() {
            if !row.attempts.is_empty() || row.witness.is_some() {
                bail!("unsupported source row has terminal attempts");
            }
            continue;
        }
        let seed = baseline.bases[row.basis_index]
            .terminal_bridge_tick_count
            .context("supported source row seed terminal missing")?;
        if row.attempts.len() != artifact.terminal_policy.duration_offsets_ticks.len()
            || !row
                .attempts
                .iter()
                .zip(&artifact.terminal_policy.duration_offsets_ticks)
                .all(|(a, offset)| {
                    a.duration_offset_ticks == *offset && a.terminal_tick_count == seed + offset
                })
        {
            bail!("terminal duration attempt coverage mismatch");
        }
        if let Some(witness) = &row.witness {
            if witness.row_index != row.row_index {
                bail!("witness row binding mismatch");
            }
            verify_source_binding(&context, &artifact.terminal_policy, witness, &baseline)?;
            let replay = replay_saved_witness(
                &context,
                &artifact.request,
                &artifact.terminal_policy,
                witness,
            )?;
            let accepted = row
                .attempts
                .iter()
                .filter(|a| a.status == "complete_witness_accepted")
                .collect::<Vec<_>>();
            if !replay.passed
                || accepted.len() != 1
                || accepted[0].terminal_tick_count != witness.reference.physics_ticks
                || accepted[0].witness_identity.as_ref() != Some(&witness.identity)
                || accepted[0].verification.as_ref() != Some(&replay)
                || accepted[0].reference_audit.as_ref() != Some(&witness.reference_audit)
                || accepted[0].first_failing_stage.is_some()
            {
                bail!("accepted attempt does not match independently replayed witness");
            }
        }
    }
    let count = artifact.rows.iter().filter(|row| row.accepted).count();
    let selected = artifact
        .rows
        .iter()
        .filter_map(|row| row.witness.as_ref().map(|w| (row.row_index, w)))
        .min_by(|(_, a), (_, b)| {
            a.planned_physics_tick_count
                .cmp(&b.planned_physics_tick_count)
                .then_with(|| a.identity.cmp(&b.identity))
        })
        .map(|(index, _)| index);
    let status = if count > 0 {
        "nominal_replay_validated_direct"
    } else {
        "unknown_no_complete_accepted_witness_within_finite_family"
    };
    if artifact.accepted_witness_count != count
        || artifact.selected_row_index != selected
        || artifact.passed != (count > 0)
        || artifact.execution_status != status
    {
        bail!("body-aware case decision summary does not match verified witnesses");
    }
    Ok(())
}

pub fn evaluate_waypoint_direct_body_aware_terminal(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
) -> Result<BodyAwareTerminalCaseArtifactV1> {
    validate_waypoint_direct_nominal_direct_generation_request(request)?;
    validate_body_aware_terminal_policy(policy)?;
    // No historical artifact is opened: this is the unchanged input-only generator.
    let baseline = evaluate_waypoint_direct_nominal_direct_generation(request)?;
    evaluate_terminal_family(request, policy, &baseline)
}

fn evaluate_terminal_family(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
    baseline: &WaypointDirectNominalDirectGenerationArtifact,
) -> Result<BodyAwareTerminalCaseArtifactV1> {
    let baseline_sha = crate::waypoint_direct_body_aware_terminal::sha256_bytes(
        &serde_json::to_vec_pretty(baseline)?,
    )?;
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let request_id = stable_digest(request)?;
    let policy_id = stable_digest(policy)?;
    let mut rows = Vec::with_capacity(baseline.rows.len());
    for base_row in &baseline.rows {
        let mut row = BodyAwareTerminalRowV1 {
            row_index: base_row.row_index,
            basis_index: base_row.basis_index,
            baseline_accepted: base_row.accepted,
            baseline_wrapper_identity: base_row
                .wrapper
                .as_ref()
                .map(|wrapper| wrapper.wrapper_identity.clone()),
            source_status: base_row.status.clone(),
            source_skip_reason: base_row.skip_reason.clone(),
            attempts: Vec::new(),
            accepted: false,
            witness: None,
        };
        let Some(prefix) = extract_source_prefix(&context, baseline, base_row.row_index)? else {
            if base_row.paired_schedule.is_some() {
                row.source_skip_reason =
                    Some("no_strict_supported_paired_source_prefix".to_owned());
            }
            rows.push(row);
            continue;
        };
        let basis = &baseline.bases[base_row.basis_index];
        let seed_coast = basis
            .coast_tick_count
            .context("generated complete basis coast missing")?;
        let seed_terminal = basis
            .terminal_bridge_tick_count
            .context("generated complete basis terminal missing")?;
        let control_interval = context.sim.control_interval_steps();
        let original_end = prefix.end.physics_step + seed_coast;
        let alignment = (control_interval - original_end % control_interval) % control_interval;
        if alignment > policy.maximum_coast_clock_alignment_ticks {
            bail!("coast alignment exceeds declared one-tick bound");
        }
        let coast_ticks = seed_coast + alignment;
        let predicted = predicted_coast_kinematics(&context, &prefix.end, coast_ticks);
        for &offset in &policy.duration_offsets_ticks {
            let ticks = seed_terminal + offset;
            let planned_steps = prefix.end.physics_step
                + coast_ticks
                + ticks
                + policy.maximum_after_reference_ticks;
            let mut attempt = BodyAwareTerminalAttemptV1 {
                duration_offset_ticks: offset,
                terminal_tick_count: ticks,
                status: "not_attempted_after_accepted_witness".to_owned(),
                first_failing_stage: None,
                reference_audit: None,
                verification: None,
                witness_identity: None,
            };
            if row.accepted {
                row.attempts.push(attempt);
                continue;
            }
            if planned_steps as f64 / f64::from(context.sim.physics_hz)
                > request.policy.analytical_policy.mission_budget_s()
                || planned_steps as f64 / f64::from(context.sim.physics_hz) > context.sim.max_time_s
            {
                attempt.status = "planned_time_rejected".to_owned();
                attempt.first_failing_stage = Some("planned_time_reserve".to_owned());
                row.attempts.push(attempt);
                continue;
            }
            let provisional = build_reference(&context, policy, predicted, ticks)?;
            let mean = paired_mean(&provisional, &context, 0);
            let coast_target = mean.x.atan2(mean.y);
            let (coast_updates, mut coast_frames, entry) =
                coast_program(&context, &prefix, coast_ticks, coast_target);
            if coast_frames.len() as u64 != coast_ticks
                || coast_frames
                    .last()
                    .is_some_and(|frame| frame.classification != ContactClassification::None)
            {
                attempt.status = "coast_contact_rejected".to_owned();
                attempt.first_failing_stage = Some("coast_first_contact".to_owned());
                row.attempts.push(attempt);
                continue;
            }
            let reference = build_reference(
                &context,
                policy,
                KinematicStateV2 {
                    position_m: entry.position_m,
                    velocity_mps: entry.velocity_mps,
                },
                ticks,
            )?;
            let audit = audit_reference(&context, request, policy, &entry, &reference)?;
            attempt.reference_audit = Some(audit.clone());
            if !audit.passed {
                attempt.status = "reference_rejected".to_owned();
                attempt.first_failing_stage = audit.rejection_reasons.first().cloned();
                row.attempts.push(attempt);
                continue;
            }
            let (terminal_updates, terminal_frames) =
                match terminal_program(&context, policy, &entry, &reference) {
                    Ok(program) => program,
                    Err(_) => {
                        attempt.status = "held60_commandability_rejected".to_owned();
                        attempt.first_failing_stage =
                            Some("held60_throttle_inverse_or_clock".to_owned());
                        row.attempts.push(attempt);
                        continue;
                    }
                };
            let mut commands = prefix.updates.clone();
            commands.extend(coast_updates);
            commands.extend(terminal_updates);
            let mut frames = prefix
                .frames
                .iter()
                .map(|frame| ProgramFrame {
                    state: frame.state.clone(),
                    classification: frame.classification.clone(),
                })
                .collect::<Vec<_>>();
            frames.append(&mut coast_frames);
            frames.extend(terminal_frames);
            let mut witness = BodyAwareTerminalWitnessV1 {
                request_identity: request_id.clone(),
                terminal_policy_identity: policy_id.clone(),
                baseline_generation_identity: baseline.identity.clone(),
                row_index: row.row_index,
                source_bridge_tick_count: prefix.source_bridge_ticks,
                source_handoff_reference: prefix.source_handoff,
                source_prefix_command_identity: stable_digest(&prefix.updates)?,
                coast_tick_count: coast_ticks,
                coast_clock_alignment_ticks: alignment,
                reference,
                terminal_entry: plant_state_evidence(&entry, &context),
                commands,
                planned_physics_tick_count: planned_steps,
                reference_audit: audit,
                verification: empty_verification(&context, planned_steps),
                identity: String::new(),
            };
            let verification = verify_program(&context, request, policy, &witness, Some(&frames))?;
            witness.verification = verification.clone();
            witness.identity = witness_identity(&witness)?;
            attempt.status = if verification.passed {
                "complete_witness_accepted"
            } else {
                "complete_witness_rejected"
            }
            .to_owned();
            attempt.first_failing_stage = verification.rejection_reasons.first().cloned();
            attempt.verification = Some(verification);
            attempt.witness_identity = Some(witness.identity.clone());
            if witness.verification.passed {
                row.accepted = true;
                row.witness = Some(witness);
            }
            row.attempts.push(attempt);
        }
        rows.push(row);
    }
    let accepted_count = rows.iter().filter(|row| row.accepted).count();
    let selected = rows
        .iter()
        .filter_map(|row| row.witness.as_ref().map(|witness| (row.row_index, witness)))
        .min_by(|(_, left), (_, right)| {
            left.planned_physics_tick_count
                .cmp(&right.planned_physics_tick_count)
                .then_with(|| left.identity.cmp(&right.identity))
        })
        .map(|(index, _)| index);
    let mut artifact = BodyAwareTerminalCaseArtifactV1 {
        schema_id: CASE_SCHEMA.to_owned(), schema_version: 1, request: request.clone(), terminal_policy: policy.clone(),
        baseline_generation_identity: baseline.identity.clone(), baseline_generation_sha256: baseline_sha,
        baseline_accepted_count: baseline.family_proof.accepted_witness_count,
        baseline_scheduled_count: baseline.family_proof.scheduled_count, rows,
        accepted_witness_count: accepted_count, selected_row_index: selected, passed: accepted_count > 0,
        execution_status: if accepted_count > 0 { "nominal_replay_validated_direct" } else { "unknown_no_complete_accepted_witness_within_finite_family" }.to_owned(),
        scope_non_claims: vec![
            "New evaluator policy; original generator, frozen ledgers, core/controller/planner/default behavior are unchanged.".to_owned(),
            "Reference poses are conditional geometry; acceptance requires independent complete actual held60 replay.".to_owned(),
            "Finite Unknown is not physical impossibility or proof that waypoints are needed.".to_owned(),
            "Discrete nominal witnesses are not swept-path, perturbation-robustness, arbitrary waypoint-entry or default authority.".to_owned(),
        ], identity: String::new(),
    };
    artifact.identity = body_aware_terminal_case_identity(&artifact)?;
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use super::*;

    fn context() -> RunContext {
        super::super::tests::test_context()
    }

    fn known_flat_case() -> &'static BodyAwareTerminalCaseArtifactV1 {
        static CASE: OnceLock<BodyAwareTerminalCaseArtifactV1> = OnceLock::new();
        CASE.get_or_init(|| {
            let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
            let request = crate::waypoint_direct_known_flat_generation_request(repo).unwrap();
            evaluate_waypoint_direct_body_aware_terminal(
                &request,
                &BodyAwareTerminalPolicyV1::default(),
            )
            .unwrap()
        })
    }

    #[test]
    fn quadratic_reference_satisfies_endpoints_and_final_horizontal_acceleration() {
        let context = context();
        for steps in [60, 480, 5640, 5820] {
            let reference = build_reference(
                &context,
                &BodyAwareTerminalPolicyV1::default(),
                KinematicStateV2 {
                    position_m: Vec2::new(-350.6511627879, 401.1380543408),
                    velocity_mps: Vec2::new(39.0697674418, -0.0746947685),
                },
                steps,
            )
            .unwrap();
            let end = reference.state_at(&context, steps);
            assert!(distance(end.position_m, reference.end.position_m) < 1.0e-8);
            assert!(distance(end.velocity_mps, reference.end.velocity_mps) < 1.0e-9);
            assert!(reference.net_acceleration(steps - 1).x.abs() < 1.0e-10);
            let encoded = serde_json::to_vec(&reference).unwrap();
            let decoded: BodyAwareTerminalReferenceV1 = serde_json::from_slice(&encoded).unwrap();
            assert_eq!(reference_identity(&decoded).unwrap(), decoded.identity);
        }
    }

    #[test]
    fn closed_form_matches_velocity_first_discrete_recurrence() {
        let context = context();
        let reference = build_reference(
            &context,
            &BodyAwareTerminalPolicyV1::default(),
            KinematicStateV2 {
                position_m: Vec2::new(-80.0, 55.0),
                velocity_mps: Vec2::new(12.0, -0.5),
            },
            840,
        )
        .unwrap();
        let mut actual = reference.start;
        for tick in 0..844 {
            actual.velocity_mps += reference.net_acceleration(tick) * context.sim.physics_dt_s();
            actual.position_m += actual.velocity_mps * context.sim.physics_dt_s();
            let expected = reference.state_at(&context, tick + 1);
            assert!(distance(actual.position_m, expected.position_m) < 1.0e-9);
            assert!(distance(actual.velocity_mps, expected.velocity_mps) < 1.0e-10);
        }
    }

    #[test]
    fn reference_refuses_nonpaired_duration_and_tuned_policy() {
        let context = context();
        let start = KinematicStateV2 {
            position_m: Vec2::new(0.0, 10.0),
            velocity_mps: Vec2::new(0.0, -0.1),
        };
        let mut policy = BodyAwareTerminalPolicyV1::default();
        assert!(build_reference(&context, &policy, start, 1).is_err());
        assert!(build_reference(&context, &policy, start, 481).is_err());
        policy.contact_undershoot_m = 0.0;
        assert!(validate_body_aware_terminal_policy(&policy).is_err());
        policy = BodyAwareTerminalPolicyV1::default();
        policy.duration_offsets_ticks.push(420);
        assert!(validate_body_aware_terminal_policy(&policy).is_err());
    }

    #[test]
    fn deliberate_crossing_is_classified_by_unchanged_core_not_endpoint_epsilon() {
        let context = context();
        let reference = build_reference(
            &context,
            &BodyAwareTerminalPolicyV1::default(),
            KinematicStateV2 {
                position_m: Vec2::new(0.0, 10.0),
                velocity_mps: Vec2::new(0.0, -0.1),
            },
            480,
        )
        .unwrap();
        let template = SimulationState::new(&context).unwrap();
        let mut contact = None;
        for tick in 0..480 {
            let thrust = reference.thrust(&context, tick);
            assert!(thrust.y > 0.0);
            let (pose, classification, matches) = core_reference_pose_probe(
                &context,
                &template,
                reference.state_at(&context, tick + 1),
                0.0,
                0.0,
                tick + 1,
            )
            .unwrap();
            assert!(matches);
            if classification != ContactClassification::None {
                contact = Some(contact_audit(&context, &pose, &classification));
                break;
            }
        }
        let contact = contact.expect("deliberate crossing must reach contact");
        assert_eq!(contact.classification, "stable_touchdown_on_target");
        assert!(
            contact
                .state
                .touchdown_feet
                .iter()
                .all(|foot| foot.signed_clearance_m < -0.001)
        );
        assert!(contact.core_matches_predicate_mirror);
        assert!((contact.state.normal_closing_speed_mps - 1.5).abs() < 1.0e-9);
    }

    #[test]
    fn two_substep_inverse_matches_actual_core_mass_consumption() {
        let context = context();
        let policy = BodyAwareTerminalPolicyV1::default();
        for required in [2.0, 5.0, 10.0] {
            let mut state = SimulationState::new(&context).unwrap();
            state.position_m = Vec2::new(0.0, 50.0);
            state.velocity_mps = Vec2::new(0.0, 0.0);
            state.attitude_rad = 0.0;
            state.set_command(Command {
                throttle_frac: paired_throttle(
                    &context,
                    &policy,
                    state.mass_kg(&context),
                    required,
                )
                .unwrap(),
                target_attitude_rad: 0.0,
            });
            for _ in 0..2 {
                assert_eq!(
                    state.step_physics_and_classify_contact(&context),
                    ContactClassification::None
                );
            }
            let measured = state.velocity_mps.y / (2.0 * context.sim.physics_dt_s())
                + context.world.gravity_mps2;
            assert!((measured - required).abs() < 1.0e-12);
        }
    }

    #[test]
    fn scalar_inverse_refuses_below_minimum_and_above_maximum_without_saturation() {
        let mut context = context();
        context.vehicle.min_throttle_frac = 0.25;
        let policy = BodyAwareTerminalPolicyV1::default();
        assert!(paired_throttle(&context, &policy, 900.0, 0.1).is_err());
        assert!(paired_throttle(&context, &policy, 900.0, 20.0).is_err());
        assert_eq!(paired_throttle(&context, &policy, 900.0, 0.0).unwrap(), 0.0);
        assert!(paired_throttle(&context, &policy, f64::NAN, 5.0).is_err());
    }

    #[test]
    fn idle_coast_alignment_advances_physics_on_original_clock() {
        let context = context();
        let mut entry = SimulationState::new(&context).unwrap();
        entry.position_m = Vec2::new(0.0, 50.0);
        entry.physics_step = 4;
        entry.sim_time_s = 4.0 / 120.0;
        let prefix = SourcePrefix {
            updates: Vec::new(),
            frames: Vec::new(),
            end: entry.clone(),
            source_handoff: KinematicStateV2 {
                position_m: entry.position_m,
                velocity_mps: entry.velocity_mps,
            },
            source_bridge_ticks: 0,
        };
        let alignment = (2 - (entry.physics_step + 3) % 2) % 2;
        assert_eq!(alignment, 1);
        let (commands, frames, end) = coast_program(&context, &prefix, 3 + alignment, 0.0);
        assert_eq!(end.physics_step, 8);
        assert_eq!(frames.len(), 4);
        assert_eq!(
            commands
                .iter()
                .map(|command| command.physics_step)
                .collect::<Vec<_>>(),
            vec![5, 7]
        );
        assert!(end.velocity_mps.y < entry.velocity_mps.y);
        assert!(end.position_m.y < entry.position_m.y);
        assert!(end.physics_step.is_multiple_of(2));
    }

    #[test]
    fn known_flat_complete_witness_replays_and_retains_incoming_contact_velocity() {
        let case = known_flat_case();
        assert!(
            case.passed,
            "{:?}",
            case.rows
                .iter()
                .map(|row| (
                    row.row_index,
                    row.attempts
                        .iter()
                        .map(|attempt| &attempt.first_failing_stage)
                        .collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>()
        );
        let witness = case.rows[case.selected_row_index.unwrap()]
            .witness
            .as_ref()
            .unwrap();
        let replay =
            verify_body_aware_terminal_witness(&case.request, &case.terminal_policy, witness)
                .unwrap();
        assert!(replay.passed);
        let contact = replay.first_contact.unwrap();
        assert_eq!(contact.classification, "stable_touchdown_on_target");
        assert!(contact.state.velocity_mps.y < -1.0);
        assert!(contact.state.normal_closing_speed_mps > 1.0);
        assert!(contact.margins.stable_maximum_clearance_margin_m > 0.10);
        assert!(replay.ordinary_neutral_parity_passed);
    }

    #[test]
    fn independent_verifier_rejects_tampered_or_off_clock_or_truncated_commands() {
        let case = known_flat_case();
        let witness = case.rows[case
            .selected_row_index
            .expect("known-flat witness required")]
        .witness
        .as_ref()
        .unwrap();
        let mut tampered = witness.clone();
        let tail = tampered
            .commands
            .iter_mut()
            .find(|command| command.phase == "terminal_bridge")
            .unwrap();
        tail.command.target_attitude_rad += 0.01;
        tampered.identity = witness_identity(&tampered).unwrap();
        assert!(
            verify_body_aware_terminal_witness(&case.request, &case.terminal_policy, &tampered)
                .is_err()
        );
        let mut off_clock = witness.clone();
        off_clock.commands.last_mut().unwrap().physics_step += 1;
        off_clock.identity = witness_identity(&off_clock).unwrap();
        assert!(
            verify_body_aware_terminal_witness(&case.request, &case.terminal_policy, &off_clock)
                .is_err()
        );
        let mut truncated = witness.clone();
        truncated.commands.pop();
        truncated.identity = witness_identity(&truncated).unwrap();
        assert!(
            verify_body_aware_terminal_witness(&case.request, &case.terminal_policy, &truncated)
                .is_err()
        );
    }

    #[test]
    fn case_roundtrip_verifies_and_rehashed_source_claim_cannot_change_provenance() {
        let case = known_flat_case();
        let bytes = serde_json::to_vec_pretty(case).unwrap();
        let mut loaded: BodyAwareTerminalCaseArtifactV1 = serde_json::from_slice(&bytes).unwrap();
        verify_body_aware_terminal_case(&loaded).unwrap();
        let index = loaded.selected_row_index.unwrap();
        let witness = loaded.rows[index].witness.as_mut().unwrap();
        witness.source_prefix_command_identity =
            "self_consistent_but_not_the_regenerated_source".to_owned();
        witness.identity = witness_identity(witness).unwrap();
        let changed_witness_identity = witness.identity.clone();
        let attempt = loaded.rows[index]
            .attempts
            .iter_mut()
            .find(|a| a.status == "complete_witness_accepted")
            .unwrap();
        attempt.witness_identity = Some(changed_witness_identity);
        loaded.identity = body_aware_terminal_case_identity(&loaded).unwrap();
        assert!(verify_body_aware_terminal_case(&loaded).is_err());
    }

    #[test]
    fn case_verifier_rejects_rehashed_omitted_witness_and_forged_duration_ledger() {
        let case = known_flat_case();
        let index = case.selected_row_index.unwrap();
        let mut omitted = case.clone();
        omitted.rows[index].accepted = false;
        omitted.rows[index].witness = None;
        let attempt = omitted.rows[index]
            .attempts
            .iter_mut()
            .find(|attempt| attempt.status == "complete_witness_accepted")
            .unwrap();
        attempt.status = "reference_rejected".to_owned();
        attempt.first_failing_stage = Some("forged_rejection".to_owned());
        attempt.reference_audit = None;
        attempt.verification = None;
        attempt.witness_identity = None;
        omitted.accepted_witness_count -= 1;
        omitted.selected_row_index = omitted
            .rows
            .iter()
            .filter_map(|row| row.witness.as_ref().map(|witness| (row.row_index, witness)))
            .min_by(|(_, left), (_, right)| {
                left.planned_physics_tick_count
                    .cmp(&right.planned_physics_tick_count)
                    .then_with(|| left.identity.cmp(&right.identity))
            })
            .map(|(index, _)| index);
        omitted.identity = body_aware_terminal_case_identity(&omitted).unwrap();
        assert!(
            verify_body_aware_terminal_case(&omitted)
                .unwrap_err()
                .to_string()
                .contains("complete finite ledger")
        );

        let mut forged = case.clone();
        let after_acceptance = forged.rows[index]
            .attempts
            .iter_mut()
            .find(|attempt| attempt.status == "not_attempted_after_accepted_witness")
            .unwrap();
        after_acceptance.status = "planned_time_rejected".to_owned();
        after_acceptance.first_failing_stage = Some("forged_rejection".to_owned());
        forged.identity = body_aware_terminal_case_identity(&forged).unwrap();
        assert!(
            verify_body_aware_terminal_case(&forged)
                .unwrap_err()
                .to_string()
                .contains("complete finite ledger")
        );
    }

    #[test]
    fn exposed_high_controls_obtain_new_complete_body_safe_witnesses() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let manifest =
            crate::load_waypoint_direct_obstacle_discrimination_fresh_manifest(repo).unwrap();
        let mut tested = 0;
        for input in manifest
            .cases
            .into_iter()
            .filter(|input| input.profile == "high_obstacle")
        {
            let request = WaypointDirectNominalDirectGenerationRequest {
                scenario: input.scenario,
                source_pad_id: input.source_pad_id,
                target_pad_id: input.target_pad_id,
                probe_id: input.probe_id,
                policy: manifest.generation_policy.clone(),
            };
            let case = evaluate_waypoint_direct_body_aware_terminal(
                &request,
                &BodyAwareTerminalPolicyV1::default(),
            )
            .unwrap();
            eprintln!(
                "{}: {} new witnesses, selected {:?}",
                input.case_id, case.accepted_witness_count, case.selected_row_index
            );
            assert_eq!(case.baseline_accepted_count, 0);
            assert!(
                case.passed,
                "{}: {:?}",
                input.case_id,
                case.rows
                    .iter()
                    .map(|row| (
                        row.row_index,
                        row.attempts
                            .iter()
                            .map(|a| &a.first_failing_stage)
                            .collect::<Vec<_>>()
                    ))
                    .collect::<Vec<_>>()
            );
            let witness = case.rows[case.selected_row_index.unwrap()]
                .witness
                .as_ref()
                .unwrap();
            let contact = witness.verification.first_contact.as_ref().unwrap();
            assert!(contact.margins.stable_maximum_clearance_margin_m > 0.10);
            assert!(contact.margins.safe_normal_speed_margin_mps > 1.0);
            tested += 1;
        }
        assert_eq!(tested, 2);
    }
}
