//! Current planner terminal; no research orchestration.
use super::*;
use anyhow::{Context, Result};
use pd_core::{RunContext, SimulationState, Vec2};
use pd_plan::ballistic::{KinematicStateV2, PadInputV2};
use serde::{Deserialize, Serialize};

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

fn reference_identity(reference: &BodyAwareTerminalReferenceV1) -> Result<String> {
    let mut canonical = reference.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

pub(super) fn build_reference(
    context: &RunContext,
    policy: &BodyAwareTerminalPolicyV1,
    start: KinematicStateV2,
    steps: u64,
) -> Result<BodyAwareTerminalReferenceV1> {
    use pd_control::poly_landing::{LandingKinematicsV1, build_reference};
    let end = LandingKinematicsV1 {
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
    let shared = build_reference(
        context,
        LandingKinematicsV1 {
            position_m: start.position_m,
            velocity_mps: start.velocity_mps,
        },
        end,
        steps,
    )
    .map_err(anyhow::Error::msg)?;
    let mut reference = BodyAwareTerminalReferenceV1 {
        start,
        end: KinematicStateV2 {
            position_m: shared.end.position_m,
            velocity_mps: shared.end.velocity_mps,
        },
        physics_ticks: shared.physics_ticks,
        horizontal_coefficients_mps2: shared.horizontal_coefficients_mps2,
        initial_vertical_acceleration_mps2: shared.initial_vertical_acceleration_mps2,
        vertical_acceleration_delta_mps2: shared.vertical_acceleration_delta_mps2,
        identity: String::new(),
    };
    reference.identity = reference_identity(&reference)?;
    Ok(reference)
}

impl BodyAwareTerminalReferenceV1 {
    fn shared(&self) -> pd_control::poly_landing::PolynomialReferenceV1 {
        use pd_control::poly_landing::{LandingKinematicsV1, PolynomialReferenceV1};
        PolynomialReferenceV1 {
            start: LandingKinematicsV1 {
                position_m: self.start.position_m,
                velocity_mps: self.start.velocity_mps,
            },
            end: LandingKinematicsV1 {
                position_m: self.end.position_m,
                velocity_mps: self.end.velocity_mps,
            },
            physics_ticks: self.physics_ticks,
            horizontal_coefficients_mps2: self.horizontal_coefficients_mps2,
            initial_vertical_acceleration_mps2: self.initial_vertical_acceleration_mps2,
            vertical_acceleration_delta_mps2: self.vertical_acceleration_delta_mps2,
        }
    }

    pub(super) fn thrust(&self, context: &RunContext, tick: u64) -> Vec2 {
        self.shared().thrust(context, tick)
    }

    pub(super) fn state_at(&self, context: &RunContext, steps: u64) -> KinematicStateV2 {
        let state = self.shared().state_at(context, steps);
        KinematicStateV2 {
            position_m: state.position_m,
            velocity_mps: state.velocity_mps,
        }
    }
}

pub(super) fn paired_mean(
    reference: &BodyAwareTerminalReferenceV1,
    context: &RunContext,
    tick: u64,
) -> Vec2 {
    pd_control::poly_landing::paired_mean(&reference.shared(), context, tick)
}

/// Preserve the historical planner API and policy identity around shared math.
pub fn paired_throttle(
    context: &RunContext,
    policy: &BodyAwareTerminalPolicyV1,
    mass: f64,
    thrust: f64,
) -> Result<f64> {
    pd_control::poly_landing::paired_throttle(
        context,
        policy.scalar_inverse_iterations,
        mass,
        thrust,
    )
    .map_err(anyhow::Error::msg)
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

/// Reuse the historical exact conservative rotated-body query. The launch
/// exception is explicit and can never leak into the local maneuver.
pub fn clearing_body_reserve_query(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
    state: &SimulationState,
    phase: &str,
    allow_source_exception: bool,
) -> Result<(f64, f64)> {
    let policy = clearance_policy(context, request)?;
    let aabb = body_aabb(state, &context.vehicle.geometry);
    let corridor = corridor_for_step(
        phase,
        state.velocity_mps,
        aabb,
        policy.source_pad,
        policy.target_pad,
    );
    let required = if allow_source_exception && corridor == "source_pad_transition" {
        0.0
    } else {
        policy.minimum_clearance_m
    };
    Ok((body_clearance(context, state, aabb)?, required))
}

/// Existing nominal corridor policy, with source exemption disabled after the
/// original launch. Unlike local clearing, terminal descent may enter its pad.
pub fn nominal_body_reserve_query(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
    state: &SimulationState,
    phase: &str,
    allow_source_exception: bool,
) -> Result<(f64, f64)> {
    let mut policy = clearance_policy(context, request)?;
    if !allow_source_exception {
        policy.source_pad.flat = false;
    }
    let aabb = body_aabb(state, &context.vehicle.geometry);
    let corridor = corridor_for_step(
        phase,
        state.velocity_mps,
        aabb,
        policy.source_pad,
        policy.target_pad,
    );
    let required = if corridor == "none" {
        policy.minimum_clearance_m
    } else {
        0.0
    };
    Ok((body_clearance(context, state, aabb)?, required))
}

pub(super) fn predicted_coast_kinematics(
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
