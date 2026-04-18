use std::collections::BTreeMap;

use pd_core::{Command, Observation, RunArtifacts, RunContext, SimulationError, run_simulation};
use serde::{Deserialize, Serialize};

pub trait Controller {
    fn id(&self) -> &str;

    fn reset(&mut self, _ctx: &RunContext) {}

    fn update(&mut self, ctx: &RunContext, observation: &Observation) -> ControllerFrame;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TelemetryValue {
    Float(f64),
    Integer(i64),
    Bool(bool),
    Text(String),
}

impl From<f64> for TelemetryValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<i64> for TelemetryValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<bool> for TelemetryValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<String> for TelemetryValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for TelemetryValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControllerMarker {
    pub id: String,
    pub label: String,
    pub x_m: Option<f64>,
    pub y_m: Option<f64>,
    #[serde(default)]
    pub metadata: BTreeMap<String, TelemetryValue>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControllerFrame {
    pub command: Command,
    pub status: String,
    pub phase: Option<String>,
    #[serde(default)]
    pub metrics: BTreeMap<String, TelemetryValue>,
    #[serde(default)]
    pub markers: Vec<ControllerMarker>,
}

impl ControllerFrame {
    pub fn command_only(command: Command) -> Self {
        Self {
            command,
            status: String::new(),
            phase: None,
            metrics: BTreeMap::new(),
            markers: Vec::new(),
        }
    }

    pub fn clamped(mut self) -> Self {
        self.command = self.command.clamped();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControllerUpdateRecord {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub controller_update_index: u64,
    pub frame: ControllerFrame,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlledRunArtifacts {
    pub run: RunArtifacts,
    pub controller_updates: Vec<ControllerUpdateRecord>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaselineControllerConfig {
    pub horizontal_position_gain: f64,
    pub horizontal_velocity_limit_mps: f64,
    pub horizontal_velocity_gain: f64,
    pub high_altitude_m: f64,
    pub medium_altitude_m: f64,
    pub low_altitude_m: f64,
    pub high_descent_rate_mps: f64,
    pub medium_descent_rate_mps: f64,
    pub low_descent_rate_mps: f64,
    pub touchdown_descent_rate_mps: f64,
    pub high_attitude_limit_rad: f64,
    pub medium_attitude_limit_rad: f64,
    pub low_attitude_limit_rad: f64,
    pub vertical_speed_gain: f64,
    pub tilt_throttle_gain: f64,
}

impl Default for BaselineControllerConfig {
    fn default() -> Self {
        Self {
            horizontal_position_gain: 0.08,
            horizontal_velocity_limit_mps: 5.0,
            horizontal_velocity_gain: 0.08,
            high_altitude_m: 80.0,
            medium_altitude_m: 30.0,
            low_altitude_m: 12.0,
            high_descent_rate_mps: -18.0,
            medium_descent_rate_mps: -10.0,
            low_descent_rate_mps: -5.0,
            touchdown_descent_rate_mps: -2.0,
            high_attitude_limit_rad: 0.45,
            medium_attitude_limit_rad: 0.25,
            low_attitude_limit_rad: 0.12,
            vertical_speed_gain: 0.09,
            tilt_throttle_gain: 0.04,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ControllerSpec {
    Idle,
    BaselineV1 {
        #[serde(flatten)]
        config: BaselineControllerConfig,
    },
}

impl ControllerSpec {
    pub fn id(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::BaselineV1 { .. } => "baseline_v1",
        }
    }

    pub fn instantiate(&self) -> Box<dyn Controller> {
        match self {
            Self::Idle => Box::new(IdleController),
            Self::BaselineV1 { config } => Box::new(BaselineController::new(config.clone())),
        }
    }
}

pub fn built_in_controller_spec(name: &str) -> Option<ControllerSpec> {
    match name {
        "idle" => Some(ControllerSpec::Idle),
        "baseline" | "baseline_v1" => Some(ControllerSpec::BaselineV1 {
            config: BaselineControllerConfig::default(),
        }),
        _ => None,
    }
}

#[derive(Debug, Default)]
pub struct IdleController;

impl Controller for IdleController {
    fn id(&self) -> &str {
        "idle"
    }

    fn update(&mut self, _ctx: &RunContext, _observation: &Observation) -> ControllerFrame {
        ControllerFrame {
            command: Command::idle(),
            status: "idle".to_owned(),
            phase: Some("idle".to_owned()),
            metrics: BTreeMap::from([
                ("throttle_mode".to_owned(), TelemetryValue::from("off")),
                ("guidance_active".to_owned(), TelemetryValue::from(false)),
            ]),
            markers: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct BaselineController {
    config: BaselineControllerConfig,
    last_phase: Option<String>,
}

impl Default for BaselineController {
    fn default() -> Self {
        Self::new(BaselineControllerConfig::default())
    }
}

impl BaselineController {
    pub fn new(config: BaselineControllerConfig) -> Self {
        Self {
            config,
            last_phase: None,
        }
    }

    fn phase_for_altitude(&self, altitude_m: f64) -> &'static str {
        if altitude_m > self.config.high_altitude_m {
            "acquire"
        } else if altitude_m > self.config.medium_altitude_m {
            "descent"
        } else if altitude_m > self.config.low_altitude_m {
            "flare"
        } else {
            "touchdown"
        }
    }

    fn desired_vertical_speed_mps(&self, altitude_m: f64) -> f64 {
        if altitude_m > self.config.high_altitude_m {
            self.config.high_descent_rate_mps
        } else if altitude_m > self.config.medium_altitude_m {
            self.config.medium_descent_rate_mps
        } else if altitude_m > self.config.low_altitude_m {
            self.config.low_descent_rate_mps
        } else {
            self.config.touchdown_descent_rate_mps
        }
    }
}

impl Controller for BaselineController {
    fn id(&self) -> &str {
        "baseline_v1"
    }

    fn reset(&mut self, _ctx: &RunContext) {
        self.last_phase = None;
    }

    fn update(&mut self, ctx: &RunContext, observation: &Observation) -> ControllerFrame {
        let max_accel_mps2 = ctx.vehicle.max_thrust_n / observation.mass_kg.max(1.0);
        let hover_throttle = observation.gravity_mps2 / max_accel_mps2.max(f64::EPSILON);
        let altitude_m = observation.touchdown_clearance_m.max(0.0);
        let desired_vx_mps = (observation.target_dx_m * self.config.horizontal_position_gain)
            .clamp(
                -self.config.horizontal_velocity_limit_mps,
                self.config.horizontal_velocity_limit_mps,
            );
        let raw_attitude_rad = ((desired_vx_mps - observation.velocity_mps.x)
            * self.config.horizontal_velocity_gain)
            .clamp(
                -self.config.high_attitude_limit_rad,
                self.config.high_attitude_limit_rad,
            );
        let attitude_limit_rad = if altitude_m > self.config.medium_altitude_m {
            self.config.high_attitude_limit_rad
        } else if altitude_m > self.config.low_altitude_m {
            self.config.medium_attitude_limit_rad
        } else {
            self.config.low_attitude_limit_rad
        };
        let target_attitude_rad = raw_attitude_rad.clamp(-attitude_limit_rad, attitude_limit_rad);

        let desired_vy_mps = self.desired_vertical_speed_mps(altitude_m);
        let vertical_error_mps = desired_vy_mps - observation.velocity_mps.y;
        let throttle_frac = (hover_throttle
            + (vertical_error_mps * self.config.vertical_speed_gain)
            + target_attitude_rad.abs() * self.config.tilt_throttle_gain)
            .clamp(0.0, 1.0);

        let phase = self.phase_for_altitude(altitude_m).to_owned();
        let status = match phase.as_str() {
            "acquire" => "tracking target pad",
            "descent" => "stabilizing descent rate",
            "flare" => "reducing sink and tilt",
            "touchdown" => "final touchdown envelope",
            _ => "guiding",
        }
        .to_owned();

        let mut markers = Vec::new();
        if self.last_phase.as_deref() != Some(phase.as_str()) {
            markers.push(ControllerMarker {
                id: format!("phase_{}", phase),
                label: format!("phase: {}", phase),
                x_m: Some(observation.position_m.x),
                y_m: Some(observation.position_m.y),
                metadata: BTreeMap::from([
                    ("phase".to_owned(), TelemetryValue::from(phase.as_str())),
                    (
                        "target_dx_m".to_owned(),
                        TelemetryValue::from(observation.target_dx_m),
                    ),
                ]),
            });
        }
        self.last_phase = Some(phase.clone());

        ControllerFrame {
            command: Command {
                throttle_frac,
                target_attitude_rad,
            },
            status,
            phase: Some(phase),
            metrics: BTreeMap::from([
                ("altitude_m".to_owned(), TelemetryValue::from(altitude_m)),
                (
                    "target_dx_m".to_owned(),
                    TelemetryValue::from(observation.target_dx_m),
                ),
                (
                    "desired_vx_mps".to_owned(),
                    TelemetryValue::from(desired_vx_mps),
                ),
                (
                    "desired_vy_mps".to_owned(),
                    TelemetryValue::from(desired_vy_mps),
                ),
                (
                    "hover_throttle".to_owned(),
                    TelemetryValue::from(hover_throttle),
                ),
                (
                    "vertical_error_mps".to_owned(),
                    TelemetryValue::from(vertical_error_mps),
                ),
            ]),
            markers,
        }
    }
}

pub fn run_controller(
    ctx: &RunContext,
    controller: &mut dyn Controller,
) -> Result<ControlledRunArtifacts, SimulationError> {
    controller.reset(ctx);
    let controller_id = controller.id().to_owned();
    let mut controller_updates = Vec::new();
    let run = run_simulation(ctx, &controller_id, |ctx, observation| {
        let frame = controller.update(ctx, observation).clamped();
        controller_updates.push(ControllerUpdateRecord {
            sim_time_s: observation.sim_time_s,
            physics_step: observation.physics_step,
            controller_update_index: controller_updates.len() as u64,
            frame: frame.clone(),
        });
        frame.command
    })?;

    Ok(ControlledRunArtifacts {
        run,
        controller_updates,
    })
}

pub fn run_controller_spec(
    ctx: &RunContext,
    spec: &ControllerSpec,
) -> Result<ControlledRunArtifacts, SimulationError> {
    let mut controller = spec.instantiate();
    run_controller(ctx, controller.as_mut())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use pd_core::{
        EndReason, EvaluationGoal, LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig,
        TerrainDefinition, Vec2, VehicleGeometry, VehicleInitialState, VehicleSpec, WorldSpec,
    };

    fn flat_scenario() -> ScenarioSpec {
        ScenarioSpec {
            id: "controller_smoke".to_owned(),
            name: "Controller smoke".to_owned(),
            description: "controller smoke".to_owned(),
            seed: 3,
            tags: vec!["test".to_owned(), "landing".to_owned()],
            metadata: BTreeMap::from([("suite".to_owned(), "control".to_owned())]),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 45.0,
                sample_hz: Some(10),
            },
            world: WorldSpec {
                gravity_mps2: 1.62,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-120.0, 0.0), Vec2::new(120.0, 0.0)],
                },
                landing_pads: vec![LandingPadSpec {
                    id: "pad_a".to_owned(),
                    center_x_m: 0.0,
                    surface_y_m: 0.0,
                    width_m: 36.0,
                }],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 4.0,
                    hull_height_m: 6.0,
                    touchdown_half_span_m: 2.0,
                    touchdown_base_offset_m: 3.2,
                },
                dry_mass_kg: 700.0,
                initial_fuel_kg: 240.0,
                max_fuel_kg: 240.0,
                max_thrust_n: 16_000.0,
                max_fuel_burn_kgps: 11.0,
                max_rotation_rate_radps: 1.2,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(18.0, 140.0),
                velocity_mps: Vec2::new(-1.0, -12.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                goal: EvaluationGoal::LandingOnPad {
                    target_pad_id: "pad_a".to_owned(),
                },
            },
        }
    }

    #[test]
    fn baseline_controller_emits_bounded_commands() {
        let ctx = RunContext::from_scenario(&flat_scenario()).unwrap();
        let observation = pd_core::SimulationState::new(&ctx)
            .unwrap()
            .build_observation(&ctx);
        let mut controller = BaselineController::default();
        let frame = controller.update(&ctx, &observation);

        assert!((0.0..=1.0).contains(&frame.command.throttle_frac));
        assert!(frame.command.target_attitude_rad.is_finite());
        assert_eq!(frame.phase.as_deref(), Some("acquire"));
        assert_eq!(frame.markers.len(), 1);
    }

    #[test]
    fn baseline_controller_lands_flat_fixture() {
        let ctx = RunContext::from_scenario(&flat_scenario()).unwrap();
        let artifacts = run_controller_spec(
            &ctx,
            &ControllerSpec::BaselineV1 {
                config: BaselineControllerConfig::default(),
            },
        )
        .unwrap();

        assert!(matches!(
            artifacts.run.manifest.end_reason,
            EndReason::TouchdownOnTarget
        ));
        assert_eq!(
            artifacts.controller_updates.len() as u64,
            artifacts.run.manifest.controller_updates
        );
    }

    #[test]
    fn baseline_controller_emits_phase_markers_when_phase_changes() {
        let ctx = RunContext::from_scenario(&flat_scenario()).unwrap();
        let mut observation = pd_core::SimulationState::new(&ctx)
            .unwrap()
            .build_observation(&ctx);
        let mut controller = BaselineController::default();

        let first = controller.update(&ctx, &observation);
        observation.touchdown_clearance_m = 8.0;
        observation.position_m.y = 8.0;
        observation.velocity_mps.y = -1.0;
        let second = controller.update(&ctx, &observation);

        assert_eq!(first.phase.as_deref(), Some("acquire"));
        assert_eq!(second.phase.as_deref(), Some("touchdown"));
        assert_eq!(second.markers.len(), 1);
    }
}
