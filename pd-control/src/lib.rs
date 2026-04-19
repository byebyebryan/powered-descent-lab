use std::collections::BTreeMap;

use pd_core::{Observation, RunArtifacts, RunContext, SimulationError, run_simulation};
use serde::{Deserialize, Serialize};

mod controllers;
pub mod kit;

pub use controllers::{
    BaselineController, BaselineControllerConfig, ControllerSpec, IdleController,
    StagedDescentController, StagedDescentControllerConfig, built_in_controller_spec,
};
pub use kit::{ControllerFrameBuilder, ControllerView, marker, metric, phase, standard_marker};

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
    pub command: pd_core::Command,
    pub status: String,
    pub phase: Option<String>,
    #[serde(default)]
    pub metrics: BTreeMap<String, TelemetryValue>,
    #[serde(default)]
    pub markers: Vec<ControllerMarker>,
}

impl ControllerFrame {
    pub fn command_only(command: pd_core::Command) -> Self {
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
        assert_eq!(frame.phase.as_deref(), Some(phase::ACQUIRE));
        assert_eq!(frame.markers.len(), 1);
        assert!(frame.metrics.contains_key(metric::ALTITUDE_M));
        assert!(frame.metrics.contains_key(metric::TARGET_DX_M));
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

        assert_eq!(first.phase.as_deref(), Some(phase::ACQUIRE));
        assert_eq!(second.phase.as_deref(), Some(phase::TOUCHDOWN));
        assert_eq!(second.markers.len(), 1);
    }

    #[test]
    fn staged_controller_lands_flat_fixture() {
        let ctx = RunContext::from_scenario(&flat_scenario()).unwrap();
        let artifacts = run_controller_spec(
            &ctx,
            &ControllerSpec::StagedDescentV1 {
                config: StagedDescentControllerConfig::default(),
            },
        )
        .unwrap();

        assert!(matches!(
            artifacts.run.manifest.end_reason,
            EndReason::TouchdownOnTarget
        ));
    }

    #[test]
    fn staged_controller_emits_gate_markers() {
        let ctx = RunContext::from_scenario(&flat_scenario()).unwrap();
        let mut observation = pd_core::SimulationState::new(&ctx)
            .unwrap()
            .build_observation(&ctx);
        let mut controller = StagedDescentController::default();

        let _first = controller.update(&ctx, &observation);
        observation.position_m.x = 1.0;
        observation.position_m.y = 40.0;
        observation.touchdown_clearance_m = 40.0;
        observation.target_dx_m = -1.0;
        let second = controller.update(&ctx, &observation);
        observation.position_m.y = 12.0;
        observation.touchdown_clearance_m = 12.0;
        let third = controller.update(&ctx, &observation);

        assert!(
            second
                .markers
                .iter()
                .any(|marker| marker.id == marker::LATERAL_CAPTURE)
        );
        assert!(
            third
                .markers
                .iter()
                .any(|marker| marker.id == marker::TERMINAL_GATE)
        );
    }
}
