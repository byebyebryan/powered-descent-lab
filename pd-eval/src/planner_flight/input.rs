//! Neutral typed input validation for the maintained nominal flight planner.

use anyhow::{Result, anyhow, bail};
use pd_core::{EvaluationGoal, FlightProgramV1, RunContext, ScenarioSpec, Vec2};
use pd_plan::ballistic::{DirectBridgePolicyV2, VehicleInputV2};
use serde::{Deserialize, Serialize};

use crate::BodyAwareTerminalPolicyV1;

/// Existing admission tolerance for source-pad rest and flat pad heights.
pub(crate) const PAD_REST_TOLERANCE_M: f64 = 1.0e-9;

pub(crate) const GENERATION_POLICY_VERSION: &str =
    "launch_aware_held60_nominal_direct_generation_v1";
const GEOMETRY_CONVENTION: &str = "core_current_rotated_feet_and_hull";
const SOLVER_VERSION: &str = "paired_mean_vector_damped_finite_difference_shooting_v1";
pub(crate) const DURATION_OFFSETS_TICKS: [i64; 5] = [-240, -180, -120, -60, 0];

fn vehicle_spec_from_v2(vehicle: &VehicleInputV2) -> pd_core::VehicleSpec {
    pd_core::VehicleSpec {
        geometry: pd_core::VehicleGeometry {
            hull_width_m: vehicle.geometry.hull_width_m,
            hull_height_m: vehicle.geometry.hull_height_m,
            touchdown_half_span_m: vehicle.geometry.touchdown_half_span_m,
            touchdown_base_offset_m: vehicle.geometry.touchdown_base_offset_m,
        },
        dry_mass_kg: vehicle.dry_mass_kg,
        initial_fuel_kg: vehicle.initial_fuel_kg,
        max_fuel_kg: vehicle.max_fuel_kg,
        max_thrust_n: vehicle.max_thrust_n,
        max_fuel_burn_kgps: vehicle.max_fuel_burn_kgps,
        min_throttle_frac: vehicle.min_throttle_frac,
        max_rotation_rate_radps: vehicle.max_rotation_rate_radps,
        safe_touchdown_normal_speed_mps: vehicle.safe_touchdown_normal_speed_mps,
        safe_touchdown_tangential_speed_mps: vehicle.safe_touchdown_tangential_speed_mps,
        safe_touchdown_attitude_error_rad: vehicle.safe_touchdown_attitude_error_rad,
        safe_touchdown_angular_rate_radps: vehicle.safe_touchdown_angular_rate_radps,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointDirectNominalDirectGenerationRequest {
    pub scenario: ScenarioSpec,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub probe_id: String,
    pub policy: WaypointDirectNominalDirectGenerationPolicyV1,
}

/// Sealed controls for the first version of input-driven nominal generation.
/// Every field is identity-bound and validation rejects values outside this
/// declared policy; callers can serialize the policy but cannot tune a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointDirectNominalDirectGenerationPolicyV1 {
    pub version: String,
    pub analytical_policy: DirectBridgePolicyV2,
    pub physics_hz: u32,
    pub controller_hz: u32,
    pub launch_upright_ticks: u64,
    pub launch_tilt_ticks: u64,
    pub source_duration_offsets_ticks: Vec<i64>,
    pub maximum_basis_candidates: usize,
    pub maximum_variants: usize,
    pub solver_version: String,
    pub maximum_solver_iterations: usize,
    pub maximum_line_search_steps: usize,
    pub finite_difference_step_mps2: f64,
    pub maximum_correction_abs_mps2: f64,
    pub initial_damping: f64,
    pub position_tolerance_m: f64,
    pub velocity_tolerance_mps: f64,
    pub geometry_convention: String,
    pub selection_rule: String,
}

impl Default for WaypointDirectNominalDirectGenerationPolicyV1 {
    fn default() -> Self {
        Self {
            version: GENERATION_POLICY_VERSION.to_owned(),
            analytical_policy: DirectBridgePolicyV2 {
                physics_hz: 120,
                gravity_mps2: 9.81,
                duration_multipliers: vec![0.75, 1.0, 1.25, 1.5],
                minimum_clearance_m: 5.0,
                maximum_mission_time_s: 90.0,
                mission_time_reserve_s: 10.0,
                thrust_derate: 0.81,
                declared_robustness_margin: 0.075,
                handoff_interval_s: 0.25,
                bridge_duration_interval_s: 0.5,
                terminal_target_downward_speed_fraction: 0.5,
            },
            physics_hz: 120,
            controller_hz: 60,
            launch_upright_ticks: 60,
            launch_tilt_ticks: 12,
            source_duration_offsets_ticks: DURATION_OFFSETS_TICKS.to_vec(),
            maximum_basis_candidates: 4,
            maximum_variants: 20,
            solver_version: SOLVER_VERSION.to_owned(),
            maximum_solver_iterations: 6,
            maximum_line_search_steps: 8,
            finite_difference_step_mps2: 1.0e-4,
            maximum_correction_abs_mps2: 0.25,
            initial_damping: 1.0e-3,
            position_tolerance_m: 1.0e-6,
            velocity_tolerance_mps: 1.0e-6,
            geometry_convention: GEOMETRY_CONVENTION.to_owned(),
            selection_rule:
                "complete accepted witnesses only; planned total mission time then stable witness identity"
                    .to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectNominalDirectGenerationValidation {
    pub probe_id: String,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub scenario_identity: String,
    pub policy_identity: String,
    pub vehicle_identity: String,
    pub source_target_horizontal_span_m: f64,
    pub source_pad_supported: bool,
    pub maximum_basis_candidates: usize,
    pub maximum_variants: usize,
    pub physics_hz: u32,
    pub controller_hz: u32,
    pub simulation_created: bool,
}

/// Validate policy and scenario support without constructing `SimulationState`
/// or evaluating any V2 candidate.
pub fn validate_waypoint_direct_nominal_direct_generation_request(
    request: &WaypointDirectNominalDirectGenerationRequest,
) -> Result<WaypointDirectNominalDirectGenerationValidation> {
    request
        .scenario
        .validate()
        .map_err(|error| anyhow!("malformed request scenario: {error}"))?;
    if request.policy != WaypointDirectNominalDirectGenerationPolicyV1::default() {
        bail!("unsupported request policy version or tuned parameter");
    }
    if request.probe_id.trim().is_empty()
        || request.source_pad_id.trim().is_empty()
        || request.target_pad_id.trim().is_empty()
        || request.source_pad_id == request.target_pad_id
    {
        bail!("malformed request: probe and distinct source/target pad IDs are required");
    }
    let scenario = &request.scenario;
    if scenario.sim.physics_hz != request.policy.physics_hz
        || scenario.sim.controller_hz != request.policy.controller_hz
        || scenario.world.gravity_mps2 != request.policy.analytical_policy.gravity_mps2
    {
        bail!("unsupported request physics rate, controller rate, or gravity setting");
    }
    let context = RunContext::from_scenario(scenario)
        .map_err(|error| anyhow!("malformed request scenario: {error}"))?;
    let supported_vehicle = pd_plan::ballistic::supported_vehicle_input_v2();
    let supported_vehicle_spec = vehicle_spec_from_v2(&supported_vehicle);
    if scenario.vehicle != supported_vehicle_spec {
        bail!("unsupported request full VehicleSpec differs from current vehicle parameters");
    }
    if scenario.initial_state.attitude_rad != 0.0
        || scenario.initial_state.angular_rate_radps != 0.0
        || scenario.initial_state.velocity_mps != pd_core::Vec2::new(0.0, 0.0)
    {
        bail!("unsupported source state: expected upright, zero angular rate, and rest");
    }
    if let Some(route) = &scenario.mission.transfer_route {
        if !route.waypoints.is_empty() {
            bail!("unsupported authored operational waypoints");
        }
        if route.source_pad_id != request.source_pad_id
            || route.target_pad_id != request.target_pad_id
        {
            bail!("malformed request: route pad IDs differ from the requested pads");
        }
    }
    if scenario.mission.goal.target_pad_id() != request.target_pad_id {
        bail!("malformed request: target pad ID must match the scenario landing goal");
    }
    let source_pad = scenario
        .world
        .landing_pad(&request.source_pad_id)
        .ok_or_else(|| {
            anyhow!(
                "malformed request: source pad {} is missing",
                request.source_pad_id
            )
        })?;
    let target_pad = scenario
        .world
        .landing_pad(&request.target_pad_id)
        .ok_or_else(|| {
            anyhow!(
                "malformed request: target pad {} is missing",
                request.target_pad_id
            )
        })?;
    if target_pad.center_x_m <= source_pad.center_x_m {
        bail!("unsupported geometry: direct generation requires a forward source-to-target span");
    }
    if (scenario.initial_state.position_m.x - source_pad.center_x_m).abs() > PAD_REST_TOLERANCE_M
        || (scenario.initial_state.position_m.y
            - (source_pad.surface_y_m + scenario.vehicle.geometry.touchdown_base_offset_m))
            .abs()
            > PAD_REST_TOLERANCE_M
    {
        bail!("unsupported source state: initial pose is not supported source-pad rest");
    }
    let body_half_width = (scenario.vehicle.geometry.hull_width_m * 0.5)
        .max(scenario.vehicle.geometry.touchdown_half_span_m);
    let source_left = source_pad.center_x_m - source_pad.width_m * 0.5;
    let source_right = source_pad.center_x_m + source_pad.width_m * 0.5;
    if source_pad.width_m <= 0.0
        || !pad_has_flat_in_domain(&scenario.world.terrain, source_pad)
        || source_pad.center_x_m - body_half_width < source_left
        || source_pad.center_x_m + body_half_width > source_right
    {
        bail!("unsupported source pad: not flat or not wide enough for the actual feet and hull");
    }
    if target_pad.width_m <= 0.0 || !pad_has_flat_in_domain(&scenario.world.terrain, target_pad) {
        bail!("unsupported target pad: not a flat shelf inside the terrain domain");
    }
    let source_target_horizontal_span_m = target_pad.center_x_m - source_pad.center_x_m;
    let scenario_identity = nominal_direct_flight_identity(scenario)?;
    let policy_identity = nominal_direct_flight_identity(&request.policy)?;
    let vehicle_identity = nominal_direct_flight_identity(&scenario.vehicle)?;
    Ok(WaypointDirectNominalDirectGenerationValidation {
        probe_id: request.probe_id.clone(),
        source_pad_id: request.source_pad_id.clone(),
        target_pad_id: request.target_pad_id.clone(),
        scenario_identity,
        policy_identity,
        vehicle_identity,
        source_target_horizontal_span_m,
        source_pad_supported: true,
        maximum_basis_candidates: request.policy.maximum_basis_candidates,
        maximum_variants: request.policy.maximum_variants,
        physics_hz: context.sim.physics_hz,
        controller_hz: context.sim.controller_hz,
        simulation_created: false,
    })
}

pub(crate) fn pad_has_flat_in_domain(
    terrain: &pd_core::TerrainDefinition,
    pad: &pd_core::LandingPadSpec,
) -> bool {
    let left = pad.center_x_m - pad.width_m * 0.5;
    let right = pad.center_x_m + pad.width_m * 0.5;
    let edge_matches = |x| {
        let Ok(sampled_height) = terrain.sample_height_strict(x) else {
            return false;
        };
        // At a stored vertex, interpolation from the preceding slope can
        // round away from its exact height. Pad admission should prove the
        // supplied polyline is flat, not require that redundant arithmetic
        // reproduce a vertex bit-for-bit. No tolerance or terrain mutation.
        let height = terrain
            .points()
            .iter()
            .find(|point| point.x == x)
            .map_or(sampled_height, |point| point.y);
        height == pad.surface_y_m
    };
    edge_matches(left)
        && edge_matches(right)
        && terrain
            .points()
            .iter()
            .all(|point| point.x < left || point.x > right || point.y == pad.surface_y_m)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum NominalDirectFlightDecisionV1 {
    Direct {
        generation_identity: String,
        selected_row_index: usize,
        program_identity: String,
        program: Box<FlightProgramV1>,
    },
    Unknown {
        generation_identity: String,
        reason: String,
    },
    Unsupported {
        reason: String,
    },
    Invalid {
        reason: String,
    },
}

impl NominalDirectFlightDecisionV1 {
    pub fn status(&self) -> &'static str {
        match self {
            Self::Direct { .. } => "direct",
            Self::Unknown { .. } => "unknown",
            Self::Unsupported { .. } => "unsupported",
            Self::Invalid { .. } => "invalid",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalDirectFlightPreflightV1 {
    pub supported: bool,
    pub rejection: Option<NominalDirectFlightDecisionV1>,
    pub simulation_created: bool,
}

/// Cheap, typed input rejection. Never run a candidate or construct physical
/// simulation state, and never classify a valid finite failure as Unsupported.
pub fn preflight_nominal_direct_flight(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
) -> NominalDirectFlightPreflightV1 {
    let invalid = |reason: String| NominalDirectFlightDecisionV1::Invalid { reason };
    let unsupported = |reason: &str| NominalDirectFlightDecisionV1::Unsupported {
        reason: reason.to_owned(),
    };
    let check = || -> std::result::Result<(), NominalDirectFlightDecisionV1> {
        request.scenario.validate().map_err(invalid)?;
        if request.probe_id.trim().is_empty()
            || request.source_pad_id.trim().is_empty()
            || request.target_pad_id.trim().is_empty()
            || request.source_pad_id == request.target_pad_id
        {
            return Err(invalid(
                "probe and distinct source/target pad IDs are required".into(),
            ));
        }
        let scenario = &request.scenario;
        for pad_id in [&request.source_pad_id, &request.target_pad_id] {
            if scenario
                .world
                .landing_pads
                .iter()
                .filter(|pad| &pad.id == pad_id)
                .count()
                != 1
            {
                return Err(invalid("requested pad must resolve exactly once".into()));
            }
        }
        let source = scenario
            .world
            .landing_pad(&request.source_pad_id)
            .ok_or_else(|| invalid("requested source pad is missing".into()))?;
        let target = scenario
            .world
            .landing_pad(&request.target_pad_id)
            .ok_or_else(|| invalid("requested target pad is missing".into()))?;
        if scenario.mission.goal.target_pad_id() != request.target_pad_id {
            return Err(invalid(
                "requested target differs from the mission target".into(),
            ));
        }
        if let Some(route) = &scenario.mission.transfer_route {
            if route.source_pad_id != request.source_pad_id
                || route.target_pad_id != request.target_pad_id
            {
                return Err(invalid(
                    "requested pad IDs differ from the authored route".into(),
                ));
            }
            if !route.waypoints.is_empty() {
                return Err(unsupported(
                    "authored operational waypoints are unsupported",
                ));
            }
        }
        if !matches!(scenario.mission.goal, EvaluationGoal::LandingOnPad { .. }) {
            return Err(unsupported("only landing_on_pad missions are supported"));
        }
        if request.policy != WaypointDirectNominalDirectGenerationPolicyV1::default()
            || *policy != BodyAwareTerminalPolicyV1::default()
        {
            return Err(unsupported(
                "unsupported or tuned generation/terminal policy",
            ));
        }
        if scenario.sim.physics_hz != request.policy.physics_hz
            || scenario.sim.controller_hz != request.policy.controller_hz
            || scenario.world.gravity_mps2 != request.policy.analytical_policy.gravity_mps2
        {
            return Err(unsupported(
                "physics rate, controller rate or gravity is unsupported",
            ));
        }
        let supported_vehicle =
            vehicle_spec_from_v2(&pd_plan::ballistic::supported_vehicle_input_v2());
        if scenario.vehicle != supported_vehicle {
            return Err(unsupported(
                "full VehicleSpec differs from the supported vehicle",
            ));
        }
        if scenario.initial_state.attitude_rad != 0.0
            || scenario.initial_state.angular_rate_radps != 0.0
            || scenario.initial_state.velocity_mps != Vec2::new(0.0, 0.0)
            || (scenario.initial_state.position_m.x - source.center_x_m).abs()
                > PAD_REST_TOLERANCE_M
            || (scenario.initial_state.position_m.y
                - source.surface_y_m
                - scenario.vehicle.geometry.touchdown_base_offset_m)
                .abs()
                > PAD_REST_TOLERANCE_M
        {
            return Err(unsupported(
                "source must be upright source-pad rest with zero angular rate",
            ));
        }
        if target.center_x_m <= source.center_x_m {
            return Err(unsupported(
                "only forward source-to-target geometry is supported",
            ));
        }
        let half_width = (scenario.vehicle.geometry.hull_width_m * 0.5)
            .max(scenario.vehicle.geometry.touchdown_half_span_m);
        if !pad_has_flat_in_domain(&scenario.world.terrain, source)
            || half_width > source.half_width_m()
            || !pad_has_flat_in_domain(&scenario.world.terrain, target)
        {
            return Err(unsupported(
                "pads must be flat in-domain shelves with a supported source footprint",
            ));
        }
        // Guard against this typed front-end drifting from the original backend.
        // Backend disagreement is an integration error, not a finite Unknown.
        validate_waypoint_direct_nominal_direct_generation_request(request)
            .map_err(|error| invalid(format!("backend input-contract disagreement: {error}")))?;
        Ok(())
    };
    let rejection = check().err();
    NominalDirectFlightPreflightV1 {
        supported: rejection.is_none(),
        rejection,
        simulation_created: false,
    }
}

pub fn nominal_direct_flight_identity<T: Serialize>(value: &T) -> Result<String> {
    Ok(format!(
        "fnv1a64:{:016x}",
        crate::runtime::fnv1a64(&serde_json::to_vec(value)?)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shelf() -> (pd_core::TerrainDefinition, pd_core::LandingPadSpec) {
        let height = 7.765405875857499;
        (
            pd_core::TerrainDefinition::Heightfield {
                points_m: vec![
                    Vec2::new(-42.0, 346.11841901493983),
                    Vec2::new(-20.0, 35.96149030411436),
                    Vec2::new(-18.0, height),
                    Vec2::new(0.0, height),
                    Vec2::new(18.0, height),
                    Vec2::new(42.0, 100.0),
                ],
            },
            pd_core::LandingPadSpec {
                id: "source".into(),
                center_x_m: 0.0,
                width_m: 36.0,
                surface_y_m: height,
            },
        )
    }

    #[test]
    fn exact_stored_shelf_edge_is_not_rejected_by_interpolation_roundoff() {
        let (terrain, pad) = shelf();
        let rounded = terrain.sample_height_strict(-18.0).unwrap();
        assert_ne!(rounded, pad.surface_y_m);
        assert_eq!(rounded - pad.surface_y_m, 8.881784197001252e-16);
        assert!(pad_has_flat_in_domain(&terrain, &pad));

        let mut request = crate::test_inputs::planner_request("v2_clear_845");
        let mut target = request
            .scenario
            .world
            .landing_pad(&request.target_pad_id)
            .unwrap()
            .clone();
        target.center_x_m = 1200.0;
        target.width_m = 36.0;
        let mut points = terrain.points().to_vec();
        points.extend([
            Vec2::new(
                target.center_x_m - target.half_width_m(),
                target.surface_y_m,
            ),
            Vec2::new(
                target.center_x_m + target.half_width_m(),
                target.surface_y_m,
            ),
            Vec2::new(
                target.center_x_m + target.half_width_m() + 100.0,
                target.surface_y_m,
            ),
        ]);
        request.scenario.world.terrain =
            pd_core::TerrainDefinition::Heightfield { points_m: points };
        request.scenario.world.landing_pads = vec![
            pd_core::LandingPadSpec {
                id: request.source_pad_id.clone(),
                ..pad.clone()
            },
            target,
        ];
        request.scenario.initial_state.position_m = Vec2::new(
            0.0,
            pad.surface_y_m + request.scenario.vehicle.geometry.touchdown_base_offset_m,
        );
        let preflight =
            preflight_nominal_direct_flight(&request, &BodyAwareTerminalPolicyV1::default());
        assert!(preflight.supported, "{:?}", preflight.rejection);
        assert!(!preflight.simulation_created);
    }

    #[test]
    fn shelf_vertex_proof_remains_exact_and_in_domain() {
        let (terrain, mut pad) = shelf();
        for index in [2, 3, 4] {
            let mut points = terrain.points().to_vec();
            points[index].y += 1.0e-12;
            let changed = pd_core::TerrainDefinition::Heightfield { points_m: points };
            assert!(!pad_has_flat_in_domain(&changed, &pad));
        }
        pad.width_m = 100.0;
        assert!(!pad_has_flat_in_domain(&terrain, &pad));
        pad.center_x_m = f64::NAN;
        assert!(!pad_has_flat_in_domain(&terrain, &pad));
    }
}
