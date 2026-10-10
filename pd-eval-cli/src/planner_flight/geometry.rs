//! Current planner geometry; no research orchestration.
use anyhow::{Result, bail};
use pd_core::{ContactClassification, CorridorEnvelope, RunContext, SimulationState, Vec2};
use pd_plan::ballistic::PadInputV2;
use serde::{Deserialize, Serialize};

use super::input::PAD_REST_TOLERANCE_M;

pub(super) const GEOMETRY_CONVENTION: &str = "core_current_rotated_feet_and_hull";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometryClearanceScanEvidence {
    pub poststep_state_count: u64,
    pub airborne_state_count: u64,
    pub source_corridor_state_count: u64,
    pub terminal_corridor_state_count: u64,
    pub exact_clearance_query_count: u64,
    pub all_airborne_states_passed: bool,
    pub first_violation: Option<GeometryClearanceViolationEvidence>,
    pub minimum_airborne: Option<AirborneClearanceMinimumEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AirborneClearanceMinimumEvidence {
    pub physics_step: u64,
    pub phase: String,
    pub clearance_m: f64,
    pub required_clearance_m: f64,
    pub corridor: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometryClearanceViolationEvidence {
    pub physics_step: u64,
    pub phase: String,
    pub reason: String,
    pub clearance_m: Option<f64>,
    pub required_clearance_m: f64,
    pub corridor: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BodyAabb {
    pub(super) horizontal_extent_m: f64,
    pub(super) vertical_extent_m: f64,
    pub(super) feet_x_min_m: f64,
    pub(super) feet_x_max_m: f64,
    pub(super) hull_x_min_m: f64,
    pub(super) hull_x_max_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FlatPadBounds {
    pub(super) left_m: f64,
    pub(super) right_m: f64,
    pub(super) surface_y_m: f64,
    pub(super) flat: bool,
}

#[derive(Clone, Copy)]
pub(super) struct ClearancePolicy {
    pub(super) source_pad: FlatPadBounds,
    pub(super) target_pad: FlatPadBounds,
    pub(super) minimum_clearance_m: f64,
}

pub(super) fn flat_pad_bounds(context: &RunContext, pad: &PadInputV2) -> FlatPadBounds {
    let half = pad.width_m * 0.5;
    let left = pad.center_x_m - half;
    let right = pad.center_x_m + half;
    let terrain = &context.world.terrain;
    let flat = match (
        terrain.sample_height_strict(left),
        terrain.sample_height_strict(right),
    ) {
        (Ok(left_y), Ok(right_y)) => {
            (left_y - pad.surface_y_m).abs() <= PAD_REST_TOLERANCE_M
                && (right_y - pad.surface_y_m).abs() <= PAD_REST_TOLERANCE_M
                && terrain.points().iter().all(|point| {
                    point.x < left
                        || point.x > right
                        || (point.y - pad.surface_y_m).abs() <= PAD_REST_TOLERANCE_M
                })
        }
        _ => false,
    };
    FlatPadBounds {
        left_m: left,
        right_m: right,
        surface_y_m: pad.surface_y_m,
        flat,
    }
}

pub(super) fn body_aabb(state: &SimulationState, geometry: &pd_core::VehicleGeometry) -> BodyAabb {
    body_aabb_from_pose(state.position_m, state.attitude_rad, geometry)
}

fn body_aabb_from_pose(
    position_m: Vec2,
    attitude_rad: f64,
    geometry: &pd_core::VehicleGeometry,
) -> BodyAabb {
    let half_w = geometry.hull_width_m * 0.5;
    let half_h = geometry.hull_height_m * 0.5;
    let feet = [
        Vec2::new(
            -geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        ),
        Vec2::new(
            geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        ),
    ]
    .map(|point| point.rotated(attitude_rad));
    let hull = [
        Vec2::new(-half_w, -half_h),
        Vec2::new(half_w, -half_h),
        Vec2::new(half_w, half_h),
        Vec2::new(-half_w, half_h),
    ]
    .map(|point| point.rotated(attitude_rad));
    // Size belongs to rotated local geometry. Adding/subtracting a world
    // position can inflate an extent by an ulp and invent pad penetration.
    let (mut horizontal_extent_m, mut vertical_extent_m) = (0.0_f64, 0.0_f64);
    for point in feet.into_iter().chain(hull) {
        horizontal_extent_m = horizontal_extent_m.max(point.x.abs());
        vertical_extent_m = vertical_extent_m.max(point.y.abs());
    }
    let feet = feet.map(|point| position_m + point);
    let hull = hull.map(|point| position_m + point);
    BodyAabb {
        horizontal_extent_m,
        vertical_extent_m,
        feet_x_min_m: feet[0].x.min(feet[1].x),
        feet_x_max_m: feet[0].x.max(feet[1].x),
        hull_x_min_m: hull
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min),
        hull_x_max_m: hull
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max),
    }
}

pub(super) fn body_clearance(
    context: &RunContext,
    state: &SimulationState,
    aabb: BodyAabb,
) -> Result<f64> {
    let envelope = CorridorEnvelope::new(aabb.horizontal_extent_m, aabb.vertical_extent_m);
    let clearance = context
        .world
        .terrain
        .exact_point_clearance(state.position_m, envelope)
        .map_err(|error| {
            let message = format!("exact body-envelope terrain query failed: {error}");
            anyhow::Error::new(error).context(message)
        })?;
    if !clearance.minimum_clearance_m.is_finite() {
        bail!("exact body-envelope clearance is non-finite");
    }
    Ok(clearance.minimum_clearance_m)
}

pub(super) fn actual_body_points(
    state: &SimulationState,
    geometry: &pd_core::VehicleGeometry,
) -> Vec<Vec2> {
    let feet = [
        Vec2::new(
            -geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        ),
        Vec2::new(
            geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        ),
    ]
    .map(|point| state.position_m + point.rotated(state.attitude_rad));
    let half_w = geometry.hull_width_m * 0.5;
    let half_h = geometry.hull_height_m * 0.5;
    let hull = [
        Vec2::new(-half_w, -half_h),
        Vec2::new(half_w, -half_h),
        Vec2::new(half_w, half_h),
        Vec2::new(-half_w, half_h),
    ]
    .map(|point| state.position_m + point.rotated(state.attitude_rad));
    feet.into_iter().chain(hull).collect()
}

fn body_within_pad(aabb: BodyAabb, pad: FlatPadBounds) -> bool {
    pad.flat
        && aabb.feet_x_min_m >= pad.left_m
        && aabb.feet_x_max_m <= pad.right_m
        && aabb.hull_x_min_m >= pad.left_m
        && aabb.hull_x_max_m <= pad.right_m
}

pub(super) fn corridor_for_step(
    phase: &str,
    velocity: Vec2,
    aabb: BodyAabb,
    source_pad: FlatPadBounds,
    target_pad: FlatPadBounds,
) -> &'static str {
    if matches!(phase, "upright" | "tilt" | "source_bridge") && body_within_pad(aabb, source_pad) {
        "source_pad_transition"
    } else if phase == "terminal_bridge" && velocity.y < 0.0 && body_within_pad(aabb, target_pad) {
        "descending_terminal_pad_transition"
    } else {
        "none"
    }
}

fn public_contact_margins(
    margins: &FirstContactPredicateMarginsEvidence,
) -> ScheduledFirstContactMarginsEvidence {
    ScheduledFirstContactMarginsEvidence {
        no_contact_minimum_foot_clearance_m: margins.no_contact_minimum_foot_clearance_m,
        no_contact_minimum_hull_clearance_m: margins.no_contact_minimum_hull_clearance_m,
        stable_minimum_clearance_margin_m: margins.stable_minimum_clearance_margin_m,
        stable_maximum_clearance_margin_m: margins.stable_maximum_clearance_margin_m,
        stable_hull_penetration_margin_m: margins.stable_hull_penetration_margin_m,
        safe_normal_speed_margin_mps: margins.safe_normal_speed_margin_mps,
        safe_tangential_speed_margin_mps: margins.safe_tangential_speed_margin_mps,
        safe_attitude_margin_rad: margins.safe_attitude_margin_rad,
        safe_angular_rate_margin_radps: margins.safe_angular_rate_margin_radps,
        touchdown_pad_left_margin_m: margins.touchdown_pad_left_margin_m,
        touchdown_pad_right_margin_m: margins.touchdown_pad_right_margin_m,
    }
}

pub(super) fn distance(left: Vec2, right: Vec2) -> f64 {
    (left - right).length()
}

pub(super) fn empty_clearance_scan() -> GeometryClearanceScanEvidence {
    GeometryClearanceScanEvidence {
        poststep_state_count: 0,
        airborne_state_count: 0,
        source_corridor_state_count: 0,
        terminal_corridor_state_count: 0,
        exact_clearance_query_count: 0,
        all_airborne_states_passed: true,
        first_violation: None,
        minimum_airborne: None,
    }
}

pub(super) fn record_airborne_clearance(
    context: &RunContext,
    state: &SimulationState,
    physics_step: u64,
    phase: &str,
    policy: ClearancePolicy,
    scan: &mut GeometryClearanceScanEvidence,
) {
    scan.airborne_state_count += 1;
    scan.exact_clearance_query_count += 1;
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
        match corridor {
            "source_pad_transition" => scan.source_corridor_state_count += 1,
            "descending_terminal_pad_transition" => scan.terminal_corridor_state_count += 1,
            _ => {}
        }
        0.0
    };
    match body_clearance(context, state, aabb) {
        Ok(clearance) => {
            let sample = AirborneClearanceMinimumEvidence {
                physics_step,
                phase: phase.to_owned(),
                clearance_m: clearance,
                required_clearance_m: required,
                corridor: corridor.to_owned(),
            };
            if scan
                .minimum_airborne
                .as_ref()
                .is_none_or(|minimum| clearance < minimum.clearance_m)
            {
                scan.minimum_airborne = Some(sample);
            }
            if clearance < required {
                scan.all_airborne_states_passed = false;
                if scan.first_violation.is_none() {
                    scan.first_violation = Some(GeometryClearanceViolationEvidence {
                        physics_step,
                        phase: phase.to_owned(),
                        reason: "actual core-rotated body envelope is below the declared clearance"
                            .to_owned(),
                        clearance_m: Some(clearance),
                        required_clearance_m: required,
                        corridor: corridor.to_owned(),
                    });
                }
            }
        }
        Err(error) => {
            scan.all_airborne_states_passed = false;
            if scan.first_violation.is_none() {
                scan.first_violation = Some(GeometryClearanceViolationEvidence {
                    physics_step,
                    phase: phase.to_owned(),
                    reason: error.to_string(),
                    clearance_m: None,
                    required_clearance_m: required,
                    corridor: corridor.to_owned(),
                });
            }
        }
    }
}

fn contact_classification_label(classification: &ContactClassification) -> &'static str {
    match classification {
        ContactClassification::None => "none",
        ContactClassification::StableTouchdown { on_target: true } => "stable_touchdown_on_target",
        ContactClassification::StableTouchdown { on_target: false } => {
            "stable_touchdown_off_target"
        }
        ContactClassification::Crash => "crash",
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScheduledFirstContactMarginsEvidence {
    pub no_contact_minimum_foot_clearance_m: f64,
    pub no_contact_minimum_hull_clearance_m: f64,
    pub stable_minimum_clearance_margin_m: f64,
    pub stable_maximum_clearance_margin_m: f64,
    pub stable_hull_penetration_margin_m: f64,
    pub safe_normal_speed_margin_mps: f64,
    pub safe_tangential_speed_margin_mps: f64,
    pub safe_attitude_margin_rad: f64,
    pub safe_angular_rate_margin_radps: f64,
    pub touchdown_pad_left_margin_m: f64,
    pub touchdown_pad_right_margin_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalContactAuditEvidence {
    pub classification: String,
    pub state: PreterminalContactStateEvidence,
    pub predicates: ContactPredicateMirrorEvidence,
    pub margins: ScheduledFirstContactMarginsEvidence,
    pub core_matches_predicate_mirror: bool,
    pub body_within_strict_terrain_domain: bool,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreterminalContactStateEvidence {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
    pub touchdown_feet: [ContactFootEvidence; 2],
    pub minimum_hull_clearance_m: f64,
    pub touchdown_pad_contains_both_feet: bool,
    pub normal_closing_speed_mps: f64,
    pub tangential_closing_speed_mps: f64,
    pub attitude_error_rad: f64,
    pub absolute_angular_rate_radps: f64,
    pub dynamic_hull_penetration_allowance_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactFootEvidence {
    pub position_m: Vec2,
    pub signed_clearance_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactPredicateMirrorEvidence {
    pub no_contact_clearance_gate: bool,
    pub stable_minimum_clearance_predicate: bool,
    pub stable_maximum_clearance_predicate: bool,
    pub stable_hull_penetration_predicate: bool,
    pub stable_touchdown_predicate: bool,
    pub safe_normal_speed_predicate: bool,
    pub safe_tangential_speed_predicate: bool,
    pub safe_attitude_predicate: bool,
    pub safe_angular_rate_predicate: bool,
    pub safe_touchdown_predicate: bool,
    pub predicted_classification: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct FirstContactPredicateMarginsEvidence {
    pub no_contact_minimum_foot_clearance_m: f64,
    pub no_contact_minimum_hull_clearance_m: f64,
    pub stable_minimum_clearance_margin_m: f64,
    pub stable_maximum_clearance_margin_m: f64,
    pub stable_hull_penetration_margin_m: f64,
    pub safe_normal_speed_margin_mps: f64,
    pub safe_tangential_speed_margin_mps: f64,
    pub safe_attitude_margin_rad: f64,
    pub safe_angular_rate_margin_radps: f64,
    pub touchdown_pad_left_margin_m: f64,
    pub touchdown_pad_right_margin_m: f64,
}

pub(super) fn first_contact_predicate_margins(
    state: &PreterminalContactStateEvidence,
    context: &RunContext,
) -> FirstContactPredicateMarginsEvidence {
    let minimum_foot_clearance = state
        .touchdown_feet
        .iter()
        .map(|foot| foot.signed_clearance_m)
        .fold(f64::INFINITY, f64::min);
    let maximum_foot_clearance = state
        .touchdown_feet
        .iter()
        .map(|foot| foot.signed_clearance_m)
        .fold(f64::NEG_INFINITY, f64::max);
    let minimum_foot_x = state
        .touchdown_feet
        .iter()
        .map(|foot| foot.position_m.x)
        .fold(f64::INFINITY, f64::min);
    let maximum_foot_x = state
        .touchdown_feet
        .iter()
        .map(|foot| foot.position_m.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let pad_left = context.target_pad.center_x_m - context.target_pad.half_width_m();
    let pad_right = context.target_pad.center_x_m + context.target_pad.half_width_m();
    FirstContactPredicateMarginsEvidence {
        no_contact_minimum_foot_clearance_m: minimum_foot_clearance,
        no_contact_minimum_hull_clearance_m: state.minimum_hull_clearance_m,
        stable_minimum_clearance_margin_m: 0.05 - minimum_foot_clearance,
        stable_maximum_clearance_margin_m: 0.15 - maximum_foot_clearance,
        stable_hull_penetration_margin_m: state.minimum_hull_clearance_m
            + state.dynamic_hull_penetration_allowance_m,
        safe_normal_speed_margin_mps: context.vehicle.safe_touchdown_normal_speed_mps
            - state.normal_closing_speed_mps,
        safe_tangential_speed_margin_mps: context.vehicle.safe_touchdown_tangential_speed_mps
            - state.tangential_closing_speed_mps,
        safe_attitude_margin_rad: context.vehicle.safe_touchdown_attitude_error_rad
            - state.attitude_error_rad,
        safe_angular_rate_margin_radps: context.vehicle.safe_touchdown_angular_rate_radps
            - state.absolute_angular_rate_radps,
        touchdown_pad_left_margin_m: minimum_foot_x - pad_left,
        touchdown_pad_right_margin_m: pad_right - maximum_foot_x,
    }
}

pub(super) fn mirror_contact_predicates(
    state: &SimulationState,
    context: &RunContext,
) -> ContactPredicateMirrorEvidence {
    let feet = touchdown_feet(state.position_m, state.attitude_rad, context);
    let minimum_touchdown_clearance_m = feet[0].signed_clearance_m.min(feet[1].signed_clearance_m);
    let maximum_touchdown_clearance_m = feet[0].signed_clearance_m.max(feet[1].signed_clearance_m);
    let minimum_hull_clearance_m = hull_vertices(state.position_m, state.attitude_rad, context)
        .into_iter()
        .map(|point| point.y - context.world.terrain.sample_height(point.x))
        .fold(f64::INFINITY, f64::min);
    let normal = context
        .world
        .terrain
        .sample_surface_normal(context.target_pad.center_x_m);
    let tangent = Vec2::new(normal.y, -normal.x);
    let normal_closing_speed_mps = (-dot(state.velocity_mps, normal)).max(0.0);
    let tangential_closing_speed_mps = dot(state.velocity_mps, tangent).abs();
    let vehicle_up = Vec2::new(state.attitude_rad.sin(), state.attitude_rad.cos());
    let attitude_error_rad = dot(vehicle_up, normal).clamp(-1.0, 1.0).acos();
    let absolute_angular_rate_radps = state.angular_rate_radps.abs();
    let hull_radius_m = (context.vehicle.geometry.hull_width_m * 0.5)
        .hypot(context.vehicle.geometry.hull_height_m * 0.5);
    let dynamic_hull_penetration_allowance_m = 0.012_f64.max(
        (normal_closing_speed_mps + absolute_angular_rate_radps * hull_radius_m)
            * context.sim.physics_dt_s(),
    );
    let pad_left_m = context.target_pad.center_x_m - context.target_pad.half_width_m();
    let pad_right_m = context.target_pad.center_x_m + context.target_pad.half_width_m();
    let touchdown_min_x = feet[0].position_m.x.min(feet[1].position_m.x);
    let touchdown_max_x = feet[0].position_m.x.max(feet[1].position_m.x);
    let on_target = touchdown_min_x >= pad_left_m && touchdown_max_x <= pad_right_m;
    let no_contact_clearance_gate =
        minimum_touchdown_clearance_m > 0.0 && minimum_hull_clearance_m > 0.0;
    let stable_minimum_clearance_predicate = minimum_touchdown_clearance_m <= 0.05;
    let stable_maximum_clearance_predicate = maximum_touchdown_clearance_m <= 0.15;
    let stable_hull_penetration_predicate =
        minimum_hull_clearance_m >= -dynamic_hull_penetration_allowance_m;
    let stable_touchdown_predicate = stable_minimum_clearance_predicate
        && stable_maximum_clearance_predicate
        && stable_hull_penetration_predicate;
    let safe_normal_speed_predicate =
        normal_closing_speed_mps <= context.vehicle.safe_touchdown_normal_speed_mps;
    let safe_tangential_speed_predicate =
        tangential_closing_speed_mps <= context.vehicle.safe_touchdown_tangential_speed_mps;
    let safe_attitude_predicate =
        attitude_error_rad <= context.vehicle.safe_touchdown_attitude_error_rad;
    let safe_angular_rate_predicate =
        absolute_angular_rate_radps <= context.vehicle.safe_touchdown_angular_rate_radps;
    let safe_touchdown_predicate = safe_normal_speed_predicate
        && safe_tangential_speed_predicate
        && safe_attitude_predicate
        && safe_angular_rate_predicate;
    let predicted_classification = if no_contact_clearance_gate {
        "none"
    } else if stable_touchdown_predicate && safe_touchdown_predicate {
        if on_target {
            "stable_touchdown_on_target"
        } else {
            "stable_touchdown_off_target"
        }
    } else {
        "crash"
    };
    ContactPredicateMirrorEvidence {
        no_contact_clearance_gate,
        stable_minimum_clearance_predicate,
        stable_maximum_clearance_predicate,
        stable_hull_penetration_predicate,
        stable_touchdown_predicate,
        safe_normal_speed_predicate,
        safe_tangential_speed_predicate,
        safe_attitude_predicate,
        safe_angular_rate_predicate,
        safe_touchdown_predicate,
        predicted_classification: predicted_classification.to_owned(),
    }
}

pub(super) fn preterminal_contact_state(
    state: &SimulationState,
    context: &RunContext,
) -> PreterminalContactStateEvidence {
    let feet = touchdown_feet(state.position_m, state.attitude_rad, context);
    let minimum_hull_clearance_m = hull_vertices(state.position_m, state.attitude_rad, context)
        .into_iter()
        .map(|point| point.y - context.world.terrain.sample_height(point.x))
        .fold(f64::INFINITY, f64::min);
    let normal = context
        .world
        .terrain
        .sample_surface_normal(context.target_pad.center_x_m);
    let tangent = Vec2::new(normal.y, -normal.x);
    let normal_closing_speed_mps = (-dot(state.velocity_mps, normal)).max(0.0);
    let tangential_closing_speed_mps = dot(state.velocity_mps, tangent).abs();
    let vehicle_up = Vec2::new(state.attitude_rad.sin(), state.attitude_rad.cos());
    let attitude_error_rad = dot(vehicle_up, normal).clamp(-1.0, 1.0).acos();
    let absolute_angular_rate_radps = state.angular_rate_radps.abs();
    let hull_radius_m = (context.vehicle.geometry.hull_width_m * 0.5)
        .hypot(context.vehicle.geometry.hull_height_m * 0.5);
    let dynamic_hull_penetration_allowance_m = 0.012_f64.max(
        (normal_closing_speed_mps + absolute_angular_rate_radps * hull_radius_m)
            * context.sim.physics_dt_s(),
    );
    let left = context.target_pad.center_x_m - context.target_pad.half_width_m();
    let right = context.target_pad.center_x_m + context.target_pad.half_width_m();
    let foot_min_x = feet[0].position_m.x.min(feet[1].position_m.x);
    let foot_max_x = feet[0].position_m.x.max(feet[1].position_m.x);
    PreterminalContactStateEvidence {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        position_m: state.position_m,
        velocity_mps: state.velocity_mps,
        attitude_rad: state.attitude_rad,
        angular_rate_radps: state.angular_rate_radps,
        fuel_kg: state.fuel_kg,
        touchdown_feet: feet,
        minimum_hull_clearance_m,
        touchdown_pad_contains_both_feet: foot_min_x >= left && foot_max_x <= right,
        normal_closing_speed_mps,
        tangential_closing_speed_mps,
        attitude_error_rad,
        absolute_angular_rate_radps,
        dynamic_hull_penetration_allowance_m,
    }
}

fn touchdown_feet(
    center: Vec2,
    attitude_rad: f64,
    context: &RunContext,
) -> [ContactFootEvidence; 2] {
    let half_span = context.vehicle.geometry.touchdown_half_span_m;
    let base = context.vehicle.geometry.touchdown_base_offset_m;
    [Vec2::new(-half_span, -base), Vec2::new(half_span, -base)].map(|local| {
        let position_m = center + local.rotated(attitude_rad);
        ContactFootEvidence {
            position_m,
            signed_clearance_m: position_m.y - context.world.terrain.sample_height(position_m.x),
        }
    })
}

fn hull_vertices(center: Vec2, attitude_rad: f64, context: &RunContext) -> [Vec2; 4] {
    let half_width = context.vehicle.geometry.hull_width_m * 0.5;
    let half_height = context.vehicle.geometry.hull_height_m * 0.5;
    [
        Vec2::new(-half_width, -half_height),
        Vec2::new(half_width, -half_height),
        Vec2::new(half_width, half_height),
        Vec2::new(-half_width, half_height),
    ]
    .map(|point| center + point.rotated(attitude_rad))
}

fn dot(left: Vec2, right: Vec2) -> f64 {
    left.x * right.x + left.y * right.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::TerrainDefinition;

    fn flat_context(surface_y_m: f64) -> RunContext {
        let request = crate::test_inputs::planner_request("v2_clear_685");
        let mut context = RunContext::from_scenario(&request.scenario).unwrap();
        context.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-5000.0, surface_y_m),
                Vec2::new(5000.0, surface_y_m),
            ],
        };
        context.initial_state.position_m.y =
            surface_y_m + context.vehicle.geometry.touchdown_base_offset_m;
        context
    }

    #[test]
    fn source_rest_roundoff_does_not_inflate_body_extent() {
        // Exact height from stopped random-001; no local capture is needed.
        let context = flat_context(23.206565037797983);
        let state = SimulationState::new(&context).unwrap();
        let aabb = body_aabb(&state, &context.vehicle.geometry);
        assert_eq!(aabb.vertical_extent_m, 5.0);
        assert_eq!(body_clearance(&context, &state, aabb).unwrap(), 0.0);
    }

    #[test]
    fn rotated_body_extents_are_independent_of_world_translation() {
        let mut geometry = flat_context(0.0).vehicle.geometry;
        // Exercise feet extending beyond the hull, as well as the normal body.
        for (span, base) in [(4.0, 5.0), (6.0, 7.0)] {
            geometry.touchdown_half_span_m = span;
            geometry.touchdown_base_offset_m = base;
            for angle in [0.0, 0.2, -0.7, std::f64::consts::FRAC_PI_2] {
                let reference = body_aabb_from_pose(Vec2::new(0.0, 0.0), angle, &geometry);
                for position in [
                    Vec2::new(0.0, 28.206565037797983),
                    Vec2::new(1200.1, -32.1),
                    Vec2::new(-1.0e6, 1.0e6),
                ] {
                    let translated = body_aabb_from_pose(position, angle, &geometry);
                    assert_eq!(
                        translated.horizontal_extent_m,
                        reference.horizontal_extent_m
                    );
                    assert_eq!(translated.vertical_extent_m, reference.vertical_extent_m);
                    let expected_feet = [Vec2::new(-span, -base), Vec2::new(span, -base)]
                        .map(|point| position + point.rotated(angle));
                    assert_eq!(
                        translated.feet_x_min_m,
                        expected_feet[0].x.min(expected_feet[1].x)
                    );
                    assert_eq!(
                        translated.feet_x_max_m,
                        expected_feet[0].x.max(expected_feet[1].x)
                    );
                }
            }
        }
    }

    #[test]
    fn body_query_does_not_clamp_fractional_pad_contact_roundoff() {
        let context = flat_context(0.1);
        let state = SimulationState::new(&context).unwrap();
        assert!(
            body_clearance(
                &context,
                &state,
                body_aabb(&state, &context.vehicle.geometry)
            )
            .unwrap()
                < 0.0
        );
    }

    #[test]
    fn body_query_keeps_penetration_tilt_domain_and_airborne_reserve_strict() {
        let context = flat_context(0.0);
        let mut state = SimulationState::new(&context).unwrap();
        state.position_m.y -= 1.0e-6;
        assert!(
            body_clearance(
                &context,
                &state,
                body_aabb(&state, &context.vehicle.geometry)
            )
            .unwrap()
                < 0.0
        );
        state.position_m.y = context.initial_state.position_m.y;
        state.attitude_rad = 0.2;
        assert!(
            body_clearance(
                &context,
                &state,
                body_aabb(&state, &context.vehicle.geometry)
            )
            .unwrap()
                < 0.0
        );
        state.attitude_rad = 0.0;
        state.position_m.x = 4999.0;
        assert!(
            body_clearance(
                &context,
                &state,
                body_aabb(&state, &context.vehicle.geometry)
            )
            .is_err()
        );

        state.position_m.x = context.initial_state.position_m.x;
        state.position_m.y += 10.0 - 1.0e-6;
        let no_pad = FlatPadBounds {
            left_m: 0.0,
            right_m: 0.0,
            surface_y_m: 0.0,
            flat: false,
        };
        let mut scan = empty_clearance_scan();
        record_airborne_clearance(
            &context,
            &state,
            2,
            "coast",
            ClearancePolicy {
                source_pad: no_pad,
                target_pad: no_pad,
                minimum_clearance_m: 10.0,
            },
            &mut scan,
        );
        assert!(!scan.all_airborne_states_passed);
        assert!(scan.first_violation.unwrap().clearance_m.unwrap() > 0.0);
    }
}
