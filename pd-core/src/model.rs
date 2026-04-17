use serde::{Deserialize, Serialize};

use crate::{math::Vec2, terrain::TerrainDefinition};

pub const RUN_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
        if self.touchdown_half_span_m > self.hull_width_m * 0.5 {
            return Err("touchdown_half_span_m cannot exceed half hull width".to_owned());
        }
        if self.touchdown_base_offset_m > self.hull_height_m * 0.5 {
            return Err("touchdown_base_offset_m cannot exceed half hull height".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VehicleSpec {
    pub geometry: VehicleGeometry,
    pub dry_mass_kg: f64,
    pub initial_fuel_kg: f64,
    pub max_fuel_kg: f64,
    pub max_thrust_n: f64,
    pub max_fuel_burn_kgps: f64,
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
        if self.initial_fuel_kg > self.max_fuel_kg {
            return Err("initial_fuel_kg cannot exceed max_fuel_kg".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionSpec {
    pub goal: EvaluationGoal,
}

impl MissionSpec {
    pub fn validate(&self) -> Result<(), String> {
        self.goal.validate()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvaluationGoal {
    LandingOnPad { target_pad_id: String },
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
        }
    }

    pub fn target_pad_id(&self) -> &str {
        match self {
            Self::LandingOnPad { target_pad_id } => target_pad_id.as_str(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioSpec {
    pub id: String,
    pub name: String,
    pub description: String,
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
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct RunContext {
    pub scenario_id: String,
    pub scenario_name: String,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalOutcome {
    Flying,
    LandedOnTarget,
    LandedOffTarget,
    Crashed,
    TimedOut,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionOutcome {
    InProgress,
    Success,
    FailedOffTarget,
    FailedCrash,
    FailedTimeout,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    Running,
    TouchdownOnTarget,
    TouchdownOffTarget,
    Crash,
    MaxTimeReached,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    ControllerUpdated,
    TouchdownOnTarget,
    TouchdownOffTarget,
    Crash,
    MaxTimeReached,
    MissionEnded,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventRecord {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub kind: EventKind,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionLogEntry {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub controller_update_index: u64,
    pub command: Command,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SampleRecord {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub observation: Observation,
    pub held_command: Command,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema_version: u32,
    pub scenario_id: String,
    pub scenario_name: String,
    pub controller_id: String,
    pub physics_hz: u32,
    pub controller_hz: u32,
    pub sim_time_s: f64,
    pub physics_steps: u64,
    pub controller_updates: u64,
    pub physical_outcome: PhysicalOutcome,
    pub mission_outcome: MissionOutcome,
    pub end_reason: EndReason,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunArtifacts {
    pub manifest: RunManifest,
    pub actions: Vec<ActionLogEntry>,
    pub events: Vec<EventRecord>,
    pub samples: Vec<SampleRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
