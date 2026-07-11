use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{math::Vec2, terrain::TerrainDefinition};

pub const RUN_SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimConfig {
    pub physics_hz: u32,
    pub controller_hz: u32,
    pub max_time_s: f64,
    pub sample_hz: Option<u32>,
}

impl SimConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.physics_hz == 0 {
            return Err("physics_hz must be > 0".to_owned());
        }
        if self.controller_hz == 0 {
            return Err("controller_hz must be > 0".to_owned());
        }
        if self.controller_hz > self.physics_hz {
            return Err("controller_hz cannot exceed physics_hz".to_owned());
        }
        if self.physics_hz % self.controller_hz != 0 {
            return Err("controller_hz must evenly divide physics_hz".to_owned());
        }
        if !self.max_time_s.is_finite() || self.max_time_s <= 0.0 {
            return Err("max_time_s must be a positive finite value".to_owned());
        }
        if let Some(sample_hz) = self.sample_hz {
            if sample_hz == 0 {
                return Err("sample_hz must be > 0 when provided".to_owned());
            }
            if sample_hz > self.physics_hz {
                return Err("sample_hz cannot exceed physics_hz".to_owned());
            }
            if self.physics_hz % sample_hz != 0 {
                return Err("sample_hz must evenly divide physics_hz".to_owned());
            }
        }
        Ok(())
    }

    pub fn physics_dt_s(&self) -> f64 {
        1.0 / f64::from(self.physics_hz)
    }

    pub fn control_interval_steps(&self) -> u64 {
        u64::from(self.physics_hz / self.controller_hz)
    }

    pub fn sample_interval_steps(&self) -> Option<u64> {
        self.sample_hz
            .map(|sample_hz| u64::from(self.physics_hz / sample_hz))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldSpec {
    pub gravity_mps2: f64,
    pub terrain: TerrainDefinition,
    pub landing_pads: Vec<LandingPadSpec>,
}

impl WorldSpec {
    pub fn validate(&self) -> Result<(), String> {
        if !self.gravity_mps2.is_finite() || self.gravity_mps2 <= 0.0 {
            return Err("gravity_mps2 must be a positive finite value".to_owned());
        }
        self.terrain.validate()?;
        if self.landing_pads.is_empty() {
            return Err("world requires at least one landing pad".to_owned());
        }
        for pad in &self.landing_pads {
            pad.validate()?;
        }
        Ok(())
    }

    pub fn landing_pad(&self, pad_id: &str) -> Option<&LandingPadSpec> {
        self.landing_pads.iter().find(|pad| pad.id == pad_id)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LandingPadSpec {
    pub id: String,
    pub center_x_m: f64,
    pub surface_y_m: f64,
    pub width_m: f64,
}

impl LandingPadSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("landing pad id must not be empty".to_owned());
        }
        if !self.center_x_m.is_finite() || !self.surface_y_m.is_finite() {
            return Err("landing pad coordinates must be finite".to_owned());
        }
        if !self.width_m.is_finite() || self.width_m <= 0.0 {
            return Err("landing pad width must be positive".to_owned());
        }
        Ok(())
    }

    pub fn half_width_m(&self) -> f64 {
        self.width_m * 0.5
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleGeometry {
    pub hull_width_m: f64,
    pub hull_height_m: f64,
    pub touchdown_half_span_m: f64,
    pub touchdown_base_offset_m: f64,
}

impl VehicleGeometry {
    pub fn validate(&self) -> Result<(), String> {
        if !self.hull_width_m.is_finite() || self.hull_width_m <= 0.0 {
            return Err("hull_width_m must be positive".to_owned());
        }
        if !self.hull_height_m.is_finite() || self.hull_height_m <= 0.0 {
            return Err("hull_height_m must be positive".to_owned());
        }
        if !self.touchdown_half_span_m.is_finite() || self.touchdown_half_span_m <= 0.0 {
            return Err("touchdown_half_span_m must be positive".to_owned());
        }
        if !self.touchdown_base_offset_m.is_finite() || self.touchdown_base_offset_m <= 0.0 {
            return Err("touchdown_base_offset_m must be positive".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleSpec {
    pub geometry: VehicleGeometry,
    pub dry_mass_kg: f64,
    pub initial_fuel_kg: f64,
    pub max_fuel_kg: f64,
    pub max_thrust_n: f64,
    pub max_fuel_burn_kgps: f64,
    #[serde(default)]
    pub min_throttle_frac: f64,
    pub max_rotation_rate_radps: f64,
    pub safe_touchdown_normal_speed_mps: f64,
    pub safe_touchdown_tangential_speed_mps: f64,
    pub safe_touchdown_attitude_error_rad: f64,
    pub safe_touchdown_angular_rate_radps: f64,
}

impl VehicleSpec {
    pub fn validate(&self) -> Result<(), String> {
        self.geometry.validate()?;
        for (label, value) in [
            ("dry_mass_kg", self.dry_mass_kg),
            ("initial_fuel_kg", self.initial_fuel_kg),
            ("max_fuel_kg", self.max_fuel_kg),
            ("max_thrust_n", self.max_thrust_n),
            ("max_fuel_burn_kgps", self.max_fuel_burn_kgps),
            ("max_rotation_rate_radps", self.max_rotation_rate_radps),
            (
                "safe_touchdown_normal_speed_mps",
                self.safe_touchdown_normal_speed_mps,
            ),
            (
                "safe_touchdown_tangential_speed_mps",
                self.safe_touchdown_tangential_speed_mps,
            ),
            (
                "safe_touchdown_attitude_error_rad",
                self.safe_touchdown_attitude_error_rad,
            ),
            (
                "safe_touchdown_angular_rate_radps",
                self.safe_touchdown_angular_rate_radps,
            ),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("{label} must be positive and finite"));
            }
        }
        if !self.min_throttle_frac.is_finite()
            || self.min_throttle_frac < 0.0
            || self.min_throttle_frac > 1.0
        {
            return Err("min_throttle_frac must be finite and within [0, 1]".to_owned());
        }
        if self.initial_fuel_kg > self.max_fuel_kg {
            return Err("initial_fuel_kg cannot exceed max_fuel_kg".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleInitialState {
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
}

impl VehicleInitialState {
    pub fn validate(&self) -> Result<(), String> {
        for (label, value) in [
            ("position_m.x", self.position_m.x),
            ("position_m.y", self.position_m.y),
            ("velocity_mps.x", self.velocity_mps.x),
            ("velocity_mps.y", self.velocity_mps.y),
            ("attitude_rad", self.attitude_rad),
            ("angular_rate_radps", self.angular_rate_radps),
        ] {
            if !value.is_finite() {
                return Err(format!("{label} must be finite"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionSpec {
    #[serde(default)]
    pub transfer_route: Option<TransferRouteSpec>,
    pub goal: EvaluationGoal,
}

impl MissionSpec {
    pub fn validate(&self) -> Result<(), String> {
        self.goal.validate()?;
        if let Some(route) = &self.transfer_route {
            route.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransferRouteSpec {
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub route_angle_deg: f64,
    pub route_radius_m: f64,
    #[serde(default)]
    pub waypoints: Vec<TransferWaypointSpec>,
}

impl TransferRouteSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_pad_id.trim().is_empty() {
            return Err("transfer_route source_pad_id must not be empty".to_owned());
        }
        if self.target_pad_id.trim().is_empty() {
            return Err("transfer_route target_pad_id must not be empty".to_owned());
        }
        if self.source_pad_id == self.target_pad_id {
            return Err("transfer_route source_pad_id and target_pad_id must differ".to_owned());
        }
        if !self.route_angle_deg.is_finite() {
            return Err("transfer_route route_angle_deg must be finite".to_owned());
        }
        if !self.route_radius_m.is_finite() || self.route_radius_m <= 0.0 {
            return Err("transfer_route route_radius_m must be positive and finite".to_owned());
        }
        for waypoint in &self.waypoints {
            waypoint.validate()?;
        }
        let mut waypoint_ids = std::collections::BTreeSet::new();
        for waypoint in &self.waypoints {
            if !waypoint_ids.insert(waypoint.id.as_str()) {
                return Err(format!(
                    "transfer_route waypoint id '{}' must be unique",
                    waypoint.id
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransferWaypointSpec {
    pub id: String,
    pub position_m: Vec2,
    pub capture_radius_m: f64,
    pub max_cross_track_m: f64,
    pub max_outbound_heading_error_rad: f64,
    pub min_outbound_progress_mps: f64,
    #[serde(default)]
    pub max_outbound_cross_speed_mps: Option<f64>,
    pub min_speed_mps: f64,
    pub max_speed_mps: f64,
    #[serde(default)]
    pub min_vertical_speed_mps: Option<f64>,
    #[serde(default)]
    pub max_vertical_speed_mps: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaypointHandoffKinematics {
    pub distance_m: f64,
    pub cross_track_m: f64,
    pub plane_progress_m: f64,
    pub outbound_heading_error_rad: f64,
    pub outbound_progress_mps: f64,
    pub outbound_cross_speed_mps: f64,
    pub speed_mps: f64,
    pub vertical_speed_mps: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaypointHandoffViolation {
    Heading,
    OutboundProgress,
    OutboundCrossSpeed,
    Speed,
    VerticalSpeed,
}

impl WaypointHandoffViolation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Heading => "heading",
            Self::OutboundProgress => "outbound_progress",
            Self::OutboundCrossSpeed => "outbound_cross_speed",
            Self::Speed => "speed",
            Self::VerticalSpeed => "vertical_speed",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WaypointHandoffAssessment {
    pub triggered: bool,
    pub spatial_pass: bool,
    pub envelope_pass: bool,
    pub violations: Vec<WaypointHandoffViolation>,
}

impl WaypointHandoffAssessment {
    pub fn contract_pass(&self) -> bool {
        self.spatial_pass && self.envelope_pass
    }
}

impl TransferWaypointSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("transfer_route waypoint id must not be empty".to_owned());
        }
        for (label, value) in [
            ("position_m.x", self.position_m.x),
            ("position_m.y", self.position_m.y),
            ("capture_radius_m", self.capture_radius_m),
            ("max_cross_track_m", self.max_cross_track_m),
            (
                "max_outbound_heading_error_rad",
                self.max_outbound_heading_error_rad,
            ),
            ("min_outbound_progress_mps", self.min_outbound_progress_mps),
            ("min_speed_mps", self.min_speed_mps),
            ("max_speed_mps", self.max_speed_mps),
        ] {
            if !value.is_finite() {
                return Err(format!("transfer_route waypoint {label} must be finite"));
            }
        }
        if self.capture_radius_m <= 0.0 {
            return Err("transfer_route waypoint capture_radius_m must be positive".to_owned());
        }
        if self.max_cross_track_m <= 0.0 {
            return Err("transfer_route waypoint max_cross_track_m must be positive".to_owned());
        }
        if self.max_outbound_heading_error_rad <= 0.0 {
            return Err(
                "transfer_route waypoint max_outbound_heading_error_rad must be positive"
                    .to_owned(),
            );
        }
        if let Some(max_cross_speed_mps) = self.max_outbound_cross_speed_mps {
            if !max_cross_speed_mps.is_finite() || max_cross_speed_mps <= 0.0 {
                return Err(
                    "transfer_route waypoint max_outbound_cross_speed_mps must be positive and finite"
                        .to_owned(),
                );
            }
        }
        if self.min_speed_mps < 0.0 || self.max_speed_mps < self.min_speed_mps {
            return Err(
                "transfer_route waypoint speed bounds must be non-negative and ordered".to_owned(),
            );
        }
        for (label, value) in [
            ("min_vertical_speed_mps", self.min_vertical_speed_mps),
            ("max_vertical_speed_mps", self.max_vertical_speed_mps),
        ] {
            if value.is_some_and(|value| !value.is_finite()) {
                return Err(format!("transfer_route waypoint {label} must be finite"));
            }
        }
        if self
            .min_vertical_speed_mps
            .zip(self.max_vertical_speed_mps)
            .is_some_and(|(min_value, max_value)| max_value < min_value)
        {
            return Err("transfer_route waypoint vertical speed bounds must be ordered".to_owned());
        }
        Ok(())
    }

    pub fn assess_handoff(
        &self,
        kinematics: WaypointHandoffKinematics,
    ) -> WaypointHandoffAssessment {
        let triggered =
            kinematics.plane_progress_m >= 0.0 || kinematics.distance_m <= self.capture_radius_m;
        let spatial_pass = kinematics.distance_m <= self.capture_radius_m
            || (kinematics.cross_track_m <= self.max_cross_track_m
                && kinematics.plane_progress_m >= -self.capture_radius_m);
        let mut violations = Vec::new();

        if kinematics.outbound_heading_error_rad > self.max_outbound_heading_error_rad {
            violations.push(WaypointHandoffViolation::Heading);
        }
        if kinematics.outbound_progress_mps < self.min_outbound_progress_mps {
            violations.push(WaypointHandoffViolation::OutboundProgress);
        }
        if self
            .max_outbound_cross_speed_mps
            .is_some_and(|limit| kinematics.outbound_cross_speed_mps.abs() > limit)
        {
            violations.push(WaypointHandoffViolation::OutboundCrossSpeed);
        }
        if kinematics.speed_mps < self.min_speed_mps || kinematics.speed_mps > self.max_speed_mps {
            violations.push(WaypointHandoffViolation::Speed);
        }
        if self
            .min_vertical_speed_mps
            .is_some_and(|limit| kinematics.vertical_speed_mps < limit)
            || self
                .max_vertical_speed_mps
                .is_some_and(|limit| kinematics.vertical_speed_mps > limit)
        {
            violations.push(WaypointHandoffViolation::VerticalSpeed);
        }

        WaypointHandoffAssessment {
            triggered,
            spatial_pass,
            envelope_pass: violations.is_empty(),
            violations,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvaluationGoal {
    LandingOnPad {
        target_pad_id: String,
    },
    WaypointHandoff {
        target_pad_id: String,
        waypoint_index: usize,
    },
    WaypointSequence {
        target_pad_id: String,
    },
    TimedCheckpoint {
        target_pad_id: String,
        end_time_s: f64,
        desired_position_offset_m: Vec2,
        max_position_error_m: f64,
        desired_velocity_mps: Vec2,
        max_velocity_error_mps: f64,
        max_attitude_error_rad: f64,
    },
}

impl EvaluationGoal {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::LandingOnPad { target_pad_id } => {
                if target_pad_id.trim().is_empty() {
                    Err("target_pad_id must not be empty".to_owned())
                } else {
                    Ok(())
                }
            }
            Self::WaypointHandoff { target_pad_id, .. }
            | Self::WaypointSequence { target_pad_id } => {
                if target_pad_id.trim().is_empty() {
                    Err("target_pad_id must not be empty".to_owned())
                } else {
                    Ok(())
                }
            }
            Self::TimedCheckpoint {
                target_pad_id,
                end_time_s,
                desired_position_offset_m,
                max_position_error_m,
                desired_velocity_mps,
                max_velocity_error_mps,
                max_attitude_error_rad,
            } => {
                if target_pad_id.trim().is_empty() {
                    return Err("target_pad_id must not be empty".to_owned());
                }
                if !end_time_s.is_finite() || *end_time_s < 0.0 {
                    return Err("end_time_s must be a finite value >= 0".to_owned());
                }
                for (label, value) in [
                    ("desired_position_offset_m.x", desired_position_offset_m.x),
                    ("desired_position_offset_m.y", desired_position_offset_m.y),
                    ("desired_velocity_mps.x", desired_velocity_mps.x),
                    ("desired_velocity_mps.y", desired_velocity_mps.y),
                    ("max_position_error_m", *max_position_error_m),
                    ("max_velocity_error_mps", *max_velocity_error_mps),
                    ("max_attitude_error_rad", *max_attitude_error_rad),
                ] {
                    if !value.is_finite() {
                        return Err(format!("{label} must be finite"));
                    }
                }
                if *max_position_error_m <= 0.0 {
                    return Err("max_position_error_m must be > 0".to_owned());
                }
                if *max_velocity_error_mps <= 0.0 {
                    return Err("max_velocity_error_mps must be > 0".to_owned());
                }
                if *max_attitude_error_rad <= 0.0 {
                    return Err("max_attitude_error_rad must be > 0".to_owned());
                }
                Ok(())
            }
        }
    }

    pub fn target_pad_id(&self) -> &str {
        match self {
            Self::LandingOnPad { target_pad_id } => target_pad_id.as_str(),
            Self::WaypointHandoff { target_pad_id, .. } => target_pad_id.as_str(),
            Self::WaypointSequence { target_pad_id } => target_pad_id.as_str(),
            Self::TimedCheckpoint { target_pad_id, .. } => target_pad_id.as_str(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScenarioSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub seed: u64,
    pub tags: Vec<String>,
    pub metadata: BTreeMap<String, String>,
    pub sim: SimConfig,
    pub world: WorldSpec,
    pub vehicle: VehicleSpec,
    pub initial_state: VehicleInitialState,
    pub mission: MissionSpec,
}

impl ScenarioSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("scenario id must not be empty".to_owned());
        }
        if self.name.trim().is_empty() {
            return Err("scenario name must not be empty".to_owned());
        }
        for tag in &self.tags {
            if tag.trim().is_empty() {
                return Err("scenario tags must not contain empty values".to_owned());
            }
        }
        for (key, value) in &self.metadata {
            if key.trim().is_empty() {
                return Err("scenario metadata keys must not be empty".to_owned());
            }
            if value.trim().is_empty() {
                return Err(format!(
                    "scenario metadata value for key '{key}' must not be empty"
                ));
            }
        }
        self.sim.validate()?;
        self.world.validate()?;
        self.vehicle.validate()?;
        self.initial_state.validate()?;
        self.mission.validate()?;
        let target_pad_id = self.mission.goal.target_pad_id();
        if self.world.landing_pad(target_pad_id).is_none() {
            return Err(format!(
                "mission target pad '{target_pad_id}' is not present in world.landing_pads"
            ));
        }
        if let Some(route) = &self.mission.transfer_route {
            if self.world.landing_pad(&route.source_pad_id).is_none() {
                return Err(format!(
                    "transfer_route source pad '{}' is not present in world.landing_pads",
                    route.source_pad_id
                ));
            }
            if self.world.landing_pad(&route.target_pad_id).is_none() {
                return Err(format!(
                    "transfer_route target pad '{}' is not present in world.landing_pads",
                    route.target_pad_id
                ));
            }
            if route.target_pad_id != target_pad_id {
                return Err(format!(
                    "transfer_route target pad '{}' must match mission goal target pad '{target_pad_id}'",
                    route.target_pad_id
                ));
            }
        }
        if let EvaluationGoal::WaypointHandoff { waypoint_index, .. } = &self.mission.goal {
            let Some(route) = &self.mission.transfer_route else {
                return Err("waypoint_handoff goal requires mission.transfer_route".to_owned());
            };
            if route.waypoints.get(*waypoint_index).is_none() {
                return Err(format!(
                    "waypoint_handoff waypoint_index {} is not present in transfer_route.waypoints",
                    waypoint_index
                ));
            }
        }
        if matches!(&self.mission.goal, EvaluationGoal::WaypointSequence { .. }) {
            let Some(route) = &self.mission.transfer_route else {
                return Err("waypoint_sequence goal requires mission.transfer_route".to_owned());
            };
            if route.waypoints.is_empty() {
                return Err(
                    "waypoint_sequence goal requires at least one transfer waypoint".to_owned(),
                );
            }
        }
        if let EvaluationGoal::TimedCheckpoint { end_time_s, .. } = &self.mission.goal
            && *end_time_s > self.sim.max_time_s
        {
            return Err("timed checkpoint end_time_s cannot exceed sim.max_time_s".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct RunContext {
    pub scenario_id: String,
    pub scenario_name: String,
    pub scenario_seed: u64,
    pub scenario_tags: Vec<String>,
    pub sim: SimConfig,
    pub world: WorldSpec,
    pub vehicle: VehicleSpec,
    pub initial_state: VehicleInitialState,
    pub mission: MissionSpec,
    pub target_pad: LandingPadSpec,
}

impl RunContext {
    pub fn from_scenario(spec: &ScenarioSpec) -> Result<Self, String> {
        spec.validate()?;
        let target_pad = spec
            .world
            .landing_pad(spec.mission.goal.target_pad_id())
            .cloned()
            .ok_or_else(|| "missing target pad".to_owned())?;
        Ok(Self {
            scenario_id: spec.id.clone(),
            scenario_name: spec.name.clone(),
            scenario_seed: spec.seed,
            scenario_tags: spec.tags.clone(),
            sim: spec.sim.clone(),
            world: spec.world.clone(),
            vehicle: spec.vehicle.clone(),
            initial_state: spec.initial_state.clone(),
            mission: spec.mission.clone(),
            target_pad,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Command {
    pub throttle_frac: f64,
    pub target_attitude_rad: f64,
}

impl Command {
    pub const fn idle() -> Self {
        Self {
            throttle_frac: 0.0,
            target_attitude_rad: 0.0,
        }
    }

    pub fn clamped(self) -> Self {
        Self {
            throttle_frac: self.throttle_frac.clamp(0.0, 1.0),
            target_attitude_rad: self.target_attitude_rad,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub mass_kg: f64,
    pub fuel_kg: f64,
    pub gravity_mps2: f64,
    pub target_dx_m: f64,
    pub height_above_target_m: f64,
    pub target_surface_y_m: f64,
    pub target_pad_half_width_m: f64,
    pub touchdown_clearance_m: f64,
    pub min_hull_clearance_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalOutcome {
    Flying,
    LandedOnTarget,
    LandedOffTarget,
    Crashed,
    TimedOut,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionOutcome {
    InProgress,
    Success,
    FailedOffTarget,
    FailedCheckpoint,
    FailedCrash,
    FailedTimeout,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    Running,
    CheckpointSatisfied,
    CheckpointFailed,
    TouchdownOnTarget,
    TouchdownOffTarget,
    Crash,
    MaxTimeReached,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    ControllerUpdated,
    WaypointHandoffSatisfied,
    WaypointHandoffFailed,
    CheckpointSatisfied,
    CheckpointFailed,
    TouchdownOnTarget,
    TouchdownOffTarget,
    Crash,
    MaxTimeReached,
    MissionEnded,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventRecord {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub kind: EventKind,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionLogEntry {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub controller_update_index: u64,
    pub command: Command,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleRecord {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub observation: Observation,
    pub held_command: Command,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LandingRunSummary {
    pub touchdown_center_offset_m: f64,
    pub pad_margin_m: f64,
    pub normal_speed_mps: f64,
    pub tangential_speed_mps: f64,
    pub attitude_error_rad: f64,
    pub angular_rate_radps: f64,
    pub normal_speed_margin_mps: f64,
    pub tangential_speed_margin_mps: f64,
    pub attitude_margin_rad: f64,
    pub angular_rate_margin_radps: f64,
    pub envelope_margin_ratio: f64,
    pub on_target: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CheckpointRunSummary {
    pub position_error_m: f64,
    pub velocity_error_mps: f64,
    pub attitude_error_rad: f64,
    pub position_margin_m: f64,
    pub velocity_margin_mps: f64,
    pub attitude_margin_rad: f64,
    pub envelope_margin_ratio: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WaypointSequenceRunSummary {
    pub passed_handoffs: usize,
    pub total_handoffs: usize,
    pub first_failed_index: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RunSummary {
    pub fuel_remaining_kg: f64,
    pub fuel_used_kg: f64,
    pub min_touchdown_clearance_m: f64,
    pub min_hull_clearance_m: f64,
    pub max_speed_mps: f64,
    pub max_abs_attitude_rad: f64,
    pub max_abs_angular_rate_radps: f64,
    pub envelope_margin_ratio: Option<f64>,
    #[serde(default)]
    pub landing: Option<LandingRunSummary>,
    #[serde(default)]
    pub checkpoint: Option<CheckpointRunSummary>,
    #[serde(default)]
    pub waypoint_sequence: Option<WaypointSequenceRunSummary>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema_version: u32,
    pub scenario_id: String,
    pub scenario_name: String,
    pub scenario_seed: u64,
    pub scenario_tags: Vec<String>,
    pub controller_id: String,
    pub physics_hz: u32,
    pub controller_hz: u32,
    pub sim_time_s: f64,
    pub physics_steps: u64,
    pub controller_updates: u64,
    pub physical_outcome: PhysicalOutcome,
    pub mission_outcome: MissionOutcome,
    pub end_reason: EndReason,
    #[serde(default)]
    pub summary: RunSummary,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunArtifacts {
    pub manifest: RunManifest,
    pub actions: Vec<ActionLogEntry>,
    pub events: Vec<EventRecord>,
    pub samples: Vec<SampleRecord>,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{math::Vec2, terrain::TerrainDefinition};

    #[test]
    fn sim_config_rejects_non_divisible_rates() {
        let config = SimConfig {
            physics_hz: 120,
            controller_hz: 50,
            max_time_s: 30.0,
            sample_hz: Some(7),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn transfer_route_requires_source_and_matching_target_pad() {
        let scenario = ScenarioSpec {
            id: "transfer_validation".to_owned(),
            name: "Transfer validation".to_owned(),
            description: "validation fixture".to_owned(),
            seed: 1,
            tags: vec!["test".to_owned()],
            metadata: BTreeMap::new(),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 90.0,
                sample_hz: Some(10),
            },
            world: WorldSpec {
                gravity_mps2: 9.81,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-120.0, 0.0), Vec2::new(120.0, 0.0)],
                },
                landing_pads: vec![
                    LandingPadSpec {
                        id: "source".to_owned(),
                        center_x_m: -100.0,
                        surface_y_m: 0.0,
                        width_m: 30.0,
                    },
                    LandingPadSpec {
                        id: "target".to_owned(),
                        center_x_m: 0.0,
                        surface_y_m: 0.0,
                        width_m: 30.0,
                    },
                ],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 4.0,
                    hull_height_m: 6.0,
                    touchdown_half_span_m: 2.0,
                    touchdown_base_offset_m: 3.0,
                },
                dry_mass_kg: 700.0,
                initial_fuel_kg: 240.0,
                max_fuel_kg: 240.0,
                max_thrust_n: 16_000.0,
                max_fuel_burn_kgps: 11.0,
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.2,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(-100.0, 3.0),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                transfer_route: Some(TransferRouteSpec {
                    source_pad_id: "source".to_owned(),
                    target_pad_id: "target".to_owned(),
                    route_angle_deg: 0.0,
                    route_radius_m: 100.0,
                    waypoints: Vec::new(),
                }),
                goal: EvaluationGoal::LandingOnPad {
                    target_pad_id: "target".to_owned(),
                },
            },
        };

        assert!(scenario.validate().is_ok());

        let mut mismatched_target = scenario.clone();
        mismatched_target
            .mission
            .transfer_route
            .as_mut()
            .expect("route present")
            .target_pad_id = "source".to_owned();
        assert!(mismatched_target.validate().is_err());

        let mut missing_source = scenario;
        missing_source
            .world
            .landing_pads
            .retain(|pad| pad.id != "source");
        assert!(missing_source.validate().is_err());
    }

    #[test]
    fn transfer_route_validates_waypoint_contracts() {
        let waypoint = TransferWaypointSpec {
            id: "wp_1".to_owned(),
            position_m: Vec2::new(-80.0, 140.0),
            capture_radius_m: 50.0,
            max_cross_track_m: 80.0,
            max_outbound_heading_error_rad: 0.7,
            min_outbound_progress_mps: 5.0,
            max_outbound_cross_speed_mps: None,
            min_speed_mps: 10.0,
            max_speed_mps: 90.0,
            min_vertical_speed_mps: Some(-45.0),
            max_vertical_speed_mps: Some(35.0),
        };
        let route = TransferRouteSpec {
            source_pad_id: "source".to_owned(),
            target_pad_id: "target".to_owned(),
            route_angle_deg: 80.0,
            route_radius_m: 800.0,
            waypoints: vec![waypoint.clone()],
        };
        assert!(route.validate().is_ok());

        let duplicate_waypoints = TransferRouteSpec {
            waypoints: vec![waypoint.clone(), waypoint.clone()],
            ..route.clone()
        };
        assert!(duplicate_waypoints.validate().is_err());

        let mut invalid_bounds = waypoint;
        invalid_bounds.max_speed_mps = invalid_bounds.min_speed_mps - 1.0;
        let invalid_route = TransferRouteSpec {
            waypoints: vec![invalid_bounds],
            ..route
        };
        assert!(invalid_route.validate().is_err());
    }

    #[test]
    fn waypoint_handoff_assessment_uses_optional_outbound_bounds() {
        let waypoint = TransferWaypointSpec {
            id: "wp_1".to_owned(),
            position_m: Vec2::new(0.0, 0.0),
            capture_radius_m: 20.0,
            max_cross_track_m: 30.0,
            max_outbound_heading_error_rad: 0.35,
            min_outbound_progress_mps: 8.0,
            max_outbound_cross_speed_mps: Some(20.0),
            min_speed_mps: 10.0,
            max_speed_mps: 130.0,
            min_vertical_speed_mps: None,
            max_vertical_speed_mps: None,
        };
        let kinematics = WaypointHandoffKinematics {
            distance_m: 10.0,
            cross_track_m: 5.0,
            plane_progress_m: -5.0,
            outbound_heading_error_rad: 0.2,
            outbound_progress_mps: 30.0,
            outbound_cross_speed_mps: 12.0,
            speed_mps: 50.0,
            vertical_speed_mps: 90.0,
        };

        let pass = waypoint.assess_handoff(kinematics);
        assert!(pass.triggered);
        assert!(pass.contract_pass());

        let cross_speed_failure = waypoint.assess_handoff(WaypointHandoffKinematics {
            outbound_cross_speed_mps: 25.0,
            ..kinematics
        });
        assert_eq!(
            cross_speed_failure.violations,
            vec![WaypointHandoffViolation::OutboundCrossSpeed]
        );

        let vertically_bounded = TransferWaypointSpec {
            max_outbound_cross_speed_mps: None,
            max_vertical_speed_mps: Some(65.0),
            ..waypoint
        };
        let vertical_failure = vertically_bounded.assess_handoff(kinematics);
        assert_eq!(
            vertical_failure.violations,
            vec![WaypointHandoffViolation::VerticalSpeed]
        );
    }

    #[test]
    fn waypoint_handoff_goal_requires_matching_route_waypoint() {
        let waypoint = TransferWaypointSpec {
            id: "wp_1".to_owned(),
            position_m: Vec2::new(-50.0, 60.0),
            capture_radius_m: 25.0,
            max_cross_track_m: 30.0,
            max_outbound_heading_error_rad: 0.8,
            min_outbound_progress_mps: 5.0,
            max_outbound_cross_speed_mps: None,
            min_speed_mps: 8.0,
            max_speed_mps: 90.0,
            min_vertical_speed_mps: Some(-50.0),
            max_vertical_speed_mps: Some(40.0),
        };
        let mut scenario = ScenarioSpec {
            id: "waypoint_handoff_validation".to_owned(),
            name: "Waypoint handoff validation".to_owned(),
            description: "validation fixture".to_owned(),
            seed: 1,
            tags: vec!["test".to_owned()],
            metadata: BTreeMap::new(),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 90.0,
                sample_hz: Some(10),
            },
            world: WorldSpec {
                gravity_mps2: 9.81,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-120.0, 0.0), Vec2::new(120.0, 0.0)],
                },
                landing_pads: vec![
                    LandingPadSpec {
                        id: "source".to_owned(),
                        center_x_m: -100.0,
                        surface_y_m: 0.0,
                        width_m: 30.0,
                    },
                    LandingPadSpec {
                        id: "target".to_owned(),
                        center_x_m: 0.0,
                        surface_y_m: 0.0,
                        width_m: 30.0,
                    },
                ],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 4.0,
                    hull_height_m: 6.0,
                    touchdown_half_span_m: 2.0,
                    touchdown_base_offset_m: 3.0,
                },
                dry_mass_kg: 700.0,
                initial_fuel_kg: 240.0,
                max_fuel_kg: 240.0,
                max_thrust_n: 16_000.0,
                max_fuel_burn_kgps: 11.0,
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.2,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(-100.0, 3.0),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                transfer_route: Some(TransferRouteSpec {
                    source_pad_id: "source".to_owned(),
                    target_pad_id: "target".to_owned(),
                    route_angle_deg: 80.0,
                    route_radius_m: 800.0,
                    waypoints: vec![waypoint],
                }),
                goal: EvaluationGoal::WaypointHandoff {
                    target_pad_id: "target".to_owned(),
                    waypoint_index: 0,
                },
            },
        };
        assert!(scenario.validate().is_ok());

        scenario
            .mission
            .transfer_route
            .as_mut()
            .expect("route present")
            .waypoints
            .clear();
        assert!(scenario.validate().is_err());

        scenario.mission.transfer_route = None;
        assert!(scenario.validate().is_err());
    }
}
