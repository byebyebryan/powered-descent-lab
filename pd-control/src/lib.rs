use std::collections::BTreeMap;
use std::time::Instant;

use pd_core::{Observation, RunArtifacts, RunContext, SimulationError, run_simulation};
use serde::{Deserialize, Serialize};

mod controllers;
pub mod kit;
mod terminal_pdg;

pub use controllers::{
    BaselineController, BaselineControllerConfig, ControllerSpec, IdleController,
    StagedDescentController, StagedDescentControllerConfig, built_in_controller_spec,
};
pub use kit::{ControllerFrameBuilder, ControllerView, marker, metric, phase, standard_marker};
pub use terminal_pdg::{TerminalPdgController, TerminalPdgControllerConfig};

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
    #[serde(default)]
    pub compute_time_us: Option<u64>,
    pub frame: ControllerFrame,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunPerformanceStats {
    pub wall_time_us: u64,
    #[serde(default)]
    pub thread_cpu_time_us: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlledRunArtifacts {
    pub run: RunArtifacts,
    pub controller_updates: Vec<ControllerUpdateRecord>,
    pub performance: RunPerformanceStats,
}

pub fn run_controller(
    ctx: &RunContext,
    controller: &mut dyn Controller,
) -> Result<ControlledRunArtifacts, SimulationError> {
    controller.reset(ctx);
    let controller_id = controller.id().to_owned();
    let mut controller_updates = Vec::new();
    let wall_started_at = Instant::now();
    let thread_cpu_started_at = current_thread_cpu_time_us();
    let run = run_simulation(ctx, &controller_id, |ctx, observation| {
        let started_at = Instant::now();
        let frame = controller.update(ctx, observation).clamped();
        let compute_time_us = started_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
        controller_updates.push(ControllerUpdateRecord {
            sim_time_s: observation.sim_time_s,
            physics_step: observation.physics_step,
            controller_update_index: controller_updates.len() as u64,
            compute_time_us: Some(compute_time_us),
            frame: frame.clone(),
        });
        frame.command
    })?;

    Ok(ControlledRunArtifacts {
        run,
        controller_updates,
        performance: RunPerformanceStats {
            wall_time_us: wall_started_at
                .elapsed()
                .as_micros()
                .min(u128::from(u64::MAX)) as u64,
            thread_cpu_time_us: current_thread_cpu_time_us()
                .zip(thread_cpu_started_at)
                .map(|(finished, started)| finished.saturating_sub(started)),
        },
    })
}

#[cfg(unix)]
fn current_thread_cpu_time_us() -> Option<u64> {
    let mut spec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut spec) };
    if rc != 0 || spec.tv_sec < 0 || spec.tv_nsec < 0 {
        return None;
    }
    let secs = spec.tv_sec as u128;
    let nanos = spec.tv_nsec as u128;
    Some(((secs * 1_000_000) + (nanos / 1_000)).min(u128::from(u64::MAX)) as u64)
}

#[cfg(not(unix))]
fn current_thread_cpu_time_us() -> Option<u64> {
    None
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
                min_throttle_frac: 0.0,
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

    fn earth_terminal_reference_scenario() -> ScenarioSpec {
        let mut scenario = flat_scenario();
        scenario.id = "terminal_pdg_earth_reference".to_owned();
        scenario.name = "Terminal PDG Earth reference".to_owned();
        scenario.description = "Representative Earth half-arc terminal reference case".to_owned();
        scenario.seed = 0;
        scenario.world.gravity_mps2 = 9.81;
        scenario.vehicle.geometry.hull_width_m = 8.0;
        scenario.vehicle.geometry.hull_height_m = 10.0;
        scenario.vehicle.geometry.touchdown_half_span_m = 4.0;
        scenario.vehicle.geometry.touchdown_base_offset_m = 5.0;
        scenario.vehicle.dry_mass_kg = 7_200.0;
        scenario.vehicle.initial_fuel_kg = 6_300.0;
        scenario.vehicle.max_fuel_kg = 6_300.0;
        scenario.vehicle.max_thrust_n = 240_000.0;
        scenario.vehicle.max_fuel_burn_kgps = 49.5;
        scenario.vehicle.min_throttle_frac = 0.25;
        scenario.vehicle.max_rotation_rate_radps = std::f64::consts::FRAC_PI_2;
        scenario.initial_state.position_m = Vec2::new(18.0, 140.0);
        scenario.initial_state.velocity_mps = Vec2::new(-1.0, -12.0);
        scenario
    }

    fn frame_metric_f64(frame: &ControllerFrame, key: &str) -> f64 {
        match frame.metrics.get(key) {
            Some(TelemetryValue::Float(value)) => *value,
            other => panic!("expected float metric {key}, got {other:?}"),
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

    #[test]
    fn terminal_pdg_controller_lands_flat_fixture() {
        let ctx = RunContext::from_scenario(&flat_scenario()).unwrap();
        let artifacts = run_controller_spec(
            &ctx,
            &ControllerSpec::TerminalPdgV1 {
                config: TerminalPdgControllerConfig::default(),
            },
        )
        .unwrap();

        assert!(matches!(
            artifacts.run.manifest.end_reason,
            EndReason::TouchdownOnTarget
        ));
    }

    #[test]
    fn terminal_pdg_controller_stabilizes_earth_terminal_reference_fixture() {
        let ctx = RunContext::from_scenario(&earth_terminal_reference_scenario()).unwrap();
        let artifacts = run_controller_spec(
            &ctx,
            &ControllerSpec::TerminalPdgV1 {
                config: TerminalPdgControllerConfig::default(),
            },
        )
        .unwrap();

        let landing = artifacts
            .run
            .manifest
            .summary
            .landing
            .as_ref()
            .expect("landing summary should be present for landing scenarios");

        assert!(landing.on_target);
        assert!(landing.envelope_margin_ratio > 0.95);
        assert!(artifacts.run.manifest.summary.min_touchdown_clearance_m < 0.5);
    }

    #[test]
    fn terminal_pdg_lands_scored_shallow_half_high_terminal_case() {
        let mut scenario = earth_terminal_reference_scenario();
        scenario.id = "terminal_pdg_shallow_half_high".to_owned();
        scenario.name = "Terminal PDG shallow half high".to_owned();
        scenario.sim.max_time_s = 60.0;
        scenario.vehicle.dry_mass_kg = 9_450.0;
        scenario.initial_state.position_m = Vec2::new(-799.6638954459129, 141.0023202655475);
        scenario.initial_state.velocity_mps = Vec2::new(133.27731590765214, 5.929613289075415);

        let ctx = RunContext::from_scenario(&scenario).unwrap();
        let artifacts = run_controller_spec(
            &ctx,
            &ControllerSpec::TerminalPdgV1 {
                config: TerminalPdgControllerConfig::default(),
            },
        )
        .unwrap();

        assert_eq!(
            artifacts.run.manifest.end_reason,
            EndReason::TouchdownOnTarget
        );
    }

    #[test]
    fn terminal_pdg_preserves_more_altitude_when_lateral_cleanup_is_behind() {
        let ctx = RunContext::from_scenario(&earth_terminal_reference_scenario()).unwrap();
        let mut high_vx_observation = pd_core::SimulationState::new(&ctx)
            .unwrap()
            .build_observation(&ctx);
        high_vx_observation.position_m = Vec2::new(-18.0, 29.0);
        high_vx_observation.velocity_mps = Vec2::new(12.0, -3.8);
        high_vx_observation.target_dx_m = 18.0;
        high_vx_observation.height_above_target_m = 29.0;
        high_vx_observation.touchdown_clearance_m = 24.0;
        high_vx_observation.min_hull_clearance_m = 24.0;

        let mut controller = TerminalPdgController::default();
        let high_vx_frame = controller.update(&ctx, &high_vx_observation);

        let mut low_vx_observation = high_vx_observation.clone();
        low_vx_observation.velocity_mps.x = 1.2;

        let mut controller = TerminalPdgController::default();
        let low_vx_frame = controller.update(&ctx, &low_vx_observation);

        let high_vx_descent = frame_metric_f64(&high_vx_frame, metric::DESIRED_VERTICAL_SPEED_MPS);
        let low_vx_descent = frame_metric_f64(&low_vx_frame, metric::DESIRED_VERTICAL_SPEED_MPS);

        assert!(high_vx_descent > low_vx_descent + 0.5);
    }

    #[test]
    fn terminal_pdg_touchdown_rescue_scales_lateral_braking_tilt() {
        let ctx = RunContext::from_scenario(&earth_terminal_reference_scenario()).unwrap();
        let mut observation = pd_core::SimulationState::new(&ctx)
            .unwrap()
            .build_observation(&ctx);
        observation.position_m = Vec2::new(-6.0, 6.0);
        observation.velocity_mps = Vec2::new(2.8, -1.2);
        observation.target_dx_m = 6.0;
        observation.height_above_target_m = 6.0;
        observation.touchdown_clearance_m = 1.0;
        observation.min_hull_clearance_m = 1.0;

        let mut controller = TerminalPdgController::default();
        let frame = controller.update(&ctx, &observation);

        assert!(frame.command.target_attitude_rad < -0.25);
        assert!(frame.command.throttle_frac > 0.0);
    }

    #[test]
    fn terminal_pdg_touchdown_rescue_does_not_late_recenter_on_pad_with_safe_vx() {
        let ctx = RunContext::from_scenario(&earth_terminal_reference_scenario()).unwrap();
        let mut observation = pd_core::SimulationState::new(&ctx)
            .unwrap()
            .build_observation(&ctx);
        observation.position_m = Vec2::new(-4.0, 5.8);
        observation.velocity_mps = Vec2::new(-0.55, -1.1);
        observation.target_dx_m = 4.0;
        observation.height_above_target_m = 5.8;
        observation.touchdown_clearance_m = 0.8;
        observation.min_hull_clearance_m = 0.8;

        let mut controller = TerminalPdgController::default();
        let frame = controller.update(&ctx, &observation);

        assert!(frame.command.target_attitude_rad.abs() < 0.08);
        assert!(frame.command.throttle_frac > 0.0);
    }

    #[test]
    fn terminal_pdg_touchdown_rescue_builds_small_inside_pad_vx_reserve() {
        let ctx = RunContext::from_scenario(&earth_terminal_reference_scenario()).unwrap();
        let mut observation = pd_core::SimulationState::new(&ctx)
            .unwrap()
            .build_observation(&ctx);
        observation.position_m = Vec2::new(-4.0, 5.8);
        observation.velocity_mps = Vec2::new(1.95, -1.1);
        observation.target_dx_m = 4.0;
        observation.height_above_target_m = 5.8;
        observation.touchdown_clearance_m = 0.8;
        observation.min_hull_clearance_m = 0.8;

        let mut controller = TerminalPdgController::default();
        let frame = controller.update(&ctx, &observation);

        assert!(frame.command.target_attitude_rad < -0.015);
        assert!(frame.command.throttle_frac > 0.0);
    }

    #[test]
    fn terminal_pdg_touchdown_rescue_keeps_small_latest_safe_tilt_on_pad() {
        let ctx = RunContext::from_scenario(&earth_terminal_reference_scenario()).unwrap();
        let mut observation = pd_core::SimulationState::new(&ctx)
            .unwrap()
            .build_observation(&ctx);
        observation.position_m = Vec2::new(0.18, 5.11);
        observation.velocity_mps = Vec2::new(0.10, -0.63);
        observation.target_dx_m = -0.18;
        observation.height_above_target_m = 5.11;
        observation.touchdown_clearance_m = 0.11;
        observation.min_hull_clearance_m = 0.11;

        let mut controller = TerminalPdgController::default();
        let frame = controller.update(&ctx, &observation);

        assert!(frame.command.target_attitude_rad < -0.01);
        assert!(frame.command.target_attitude_rad > -0.08);
        assert!(frame.command.throttle_frac > 0.0);
    }

    #[test]
    fn terminal_pdg_controller_emits_guidance_metrics_and_gate_markers() {
        let ctx = RunContext::from_scenario(&flat_scenario()).unwrap();
        let observation = pd_core::SimulationState::new(&ctx)
            .unwrap()
            .build_observation(&ctx);
        let mut controller = TerminalPdgController::default();

        let frame = controller.update(&ctx, &observation);

        assert_eq!(frame.phase.as_deref(), Some(phase::DESCENT));
        assert_eq!(
            frame.metrics.get(metric::GUIDANCE_MODE),
            Some(&TelemetryValue::from("nominal pending"))
        );
        assert!(frame.metrics.contains_key(metric::GUIDANCE_BURN_TIME_S));
        assert!(
            frame
                .metrics
                .contains_key(metric::GUIDANCE_REQUIRED_ACCEL_RATIO)
        );
        assert!(
            frame
                .markers
                .iter()
                .any(|marker| marker.id == marker::TERMINAL_GATE)
        );
    }
}
