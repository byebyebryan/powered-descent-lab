use std::collections::BTreeMap;

use pd_core::{Command, FlightProgramV1, Observation, RunContext, SimulationError};

use crate::{ControlledRunArtifacts, Controller, ControllerFrame, TelemetryValue, run_controller};

pub fn run_flight_program(
    context: &RunContext,
    program: &FlightProgramV1,
) -> Result<ControlledRunArtifacts, SimulationError> {
    program
        .validate_against_context(context)
        .map_err(SimulationError::InvalidContext)?;

    let mut controller = FlightProgramController::new(program.clone());
    let artifacts = run_controller(context, &mut controller)?;

    if let Some(error) = controller.failure() {
        return Err(SimulationError::InvalidContext(format!(
            "flight program playback failed: {error}"
        )));
    }

    let expected_updates = program.updates.len();
    let expected_updates_u64 = u64::try_from(expected_updates).map_err(|_| {
        SimulationError::InvalidContext(
            "flight program update count is not representable".to_owned(),
        )
    })?;
    if artifacts.run.manifest.physics_steps != program.expected_contact_physics_step {
        return Err(SimulationError::InvalidContext(format!(
            "flight program ended at physics step {}, expected contact at {}",
            artifacts.run.manifest.physics_steps, program.expected_contact_physics_step
        )));
    }
    if artifacts.controller_updates.len() != expected_updates
        || artifacts.run.manifest.controller_updates != expected_updates_u64
        || artifacts.run.actions.len() != expected_updates
    {
        return Err(SimulationError::InvalidContext(format!(
            "flight program executed {} controller updates/actions, expected {expected_updates}",
            artifacts.controller_updates.len()
        )));
    }

    for (index, (record, update)) in artifacts
        .controller_updates
        .iter()
        .zip(&program.updates)
        .enumerate()
    {
        if record.physics_step != update.physics_step || record.frame.command != update.command {
            return Err(SimulationError::InvalidContext(format!(
                "flight program playback diverged from update {index}"
            )));
        }
    }

    Ok(artifacts)
}

struct FlightProgramController {
    program: FlightProgramV1,
    next_update_index: usize,
    failure: Option<String>,
}

impl FlightProgramController {
    fn new(program: FlightProgramV1) -> Self {
        Self {
            program,
            next_update_index: 0,
            failure: None,
        }
    }

    fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    fn playback_frame(&self, update_index: usize) -> ControllerFrame {
        let update = &self.program.updates[update_index];
        let status = format!(
            "flight program {} -> {} | generation={} | terminal={} | witness={}",
            self.program.source_pad_id,
            self.program.target_pad_id,
            self.program.generation_policy_identity,
            self.program.terminal_policy_identity,
            self.program.witness_identity,
        );
        let metrics = BTreeMap::from([
            (
                "flight_program.source_pad_id".to_owned(),
                TelemetryValue::Text(self.program.source_pad_id.clone()),
            ),
            (
                "flight_program.target_pad_id".to_owned(),
                TelemetryValue::Text(self.program.target_pad_id.clone()),
            ),
            (
                "flight_program.generation_policy_identity".to_owned(),
                TelemetryValue::Text(self.program.generation_policy_identity.clone()),
            ),
            (
                "flight_program.terminal_policy_identity".to_owned(),
                TelemetryValue::Text(self.program.terminal_policy_identity.clone()),
            ),
            (
                "flight_program.witness_identity".to_owned(),
                TelemetryValue::Text(self.program.witness_identity.clone()),
            ),
        ]);

        ControllerFrame {
            command: update.command,
            status,
            phase: Some(update.phase.clone()),
            metrics,
            markers: Vec::new(),
        }
    }

    fn failure_frame(&self) -> ControllerFrame {
        ControllerFrame {
            command: Command::idle(),
            status: self
                .failure
                .clone()
                .unwrap_or_else(|| "flight program playback failed".to_owned()),
            phase: None,
            metrics: BTreeMap::new(),
            markers: Vec::new(),
        }
    }
}

impl Controller for FlightProgramController {
    fn id(&self) -> &str {
        "flight_program_v1"
    }

    fn reset(&mut self, _context: &RunContext) {
        self.next_update_index = 0;
        self.failure = None;
    }

    fn update(&mut self, _context: &RunContext, observation: &Observation) -> ControllerFrame {
        if self.failure.is_some() {
            return self.failure_frame();
        }

        let Some(update) = self.program.updates.get(self.next_update_index) else {
            self.failure = Some(format!(
                "unexpected controller callback at physics step {} after command coverage ended",
                observation.physics_step
            ));
            return self.failure_frame();
        };
        if update.physics_step != observation.physics_step {
            self.failure = Some(format!(
                "expected callback at physics step {}, got {}",
                update.physics_step, observation.physics_step
            ));
            return self.failure_frame();
        }

        let frame = self.playback_frame(self.next_update_index);
        self.next_update_index += 1;
        frame
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use pd_core::{
        EvaluationGoal, FlightProgramUpdateV1, LandingPadSpec, MissionSpec, RunContext,
        ScenarioSpec, SimConfig, TerrainDefinition, TransferRouteSpec, Vec2, VehicleGeometry,
        VehicleInitialState, VehicleSpec, WorldSpec,
    };

    use super::*;

    fn fixture(expected_contact_physics_step: u64) -> (RunContext, FlightProgramV1) {
        let scenario = ScenarioSpec {
            id: "flight_program_playback_test".to_owned(),
            name: "Flight program playback test".to_owned(),
            description: "short real-runner playback fixture".to_owned(),
            seed: 4,
            tags: vec!["test".to_owned()],
            metadata: BTreeMap::new(),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 2.0,
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
                        center_x_m: -40.0,
                        surface_y_m: 0.0,
                        width_m: 36.0,
                    },
                    LandingPadSpec {
                        id: "target".to_owned(),
                        center_x_m: 0.0,
                        surface_y_m: 0.0,
                        width_m: 36.0,
                    },
                ],
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
                position_m: Vec2::new(0.0, 3.205),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                transfer_route: Some(TransferRouteSpec {
                    source_pad_id: "source".to_owned(),
                    target_pad_id: "target".to_owned(),
                    route_angle_deg: 0.0,
                    route_radius_m: 40.0,
                    waypoints: Vec::new(),
                }),
                goal: EvaluationGoal::LandingOnPad {
                    target_pad_id: "target".to_owned(),
                },
            },
        };
        let context = RunContext::from_scenario(&scenario).unwrap();
        let interval = context.sim.control_interval_steps();
        let expected_updates = expected_contact_physics_step / interval
            + u64::from(!expected_contact_physics_step.is_multiple_of(interval));
        let updates = (0..expected_updates)
            .map(|index| FlightProgramUpdateV1 {
                physics_step: index * interval,
                phase: if index == 0 {
                    "upright".to_owned()
                } else {
                    "terminal_bridge".to_owned()
                },
                command: Command::idle(),
            })
            .collect();
        let program = FlightProgramV1 {
            schema_version: 1,
            binding: pd_core::FlightProgramBindingV1::from_context(&context),
            source_pad_id: "source".to_owned(),
            target_pad_id: "target".to_owned(),
            generation_policy_identity: "test-generator-v1".to_owned(),
            terminal_policy_identity: "test-terminal-v1".to_owned(),
            witness_identity: "test-witness".to_owned(),
            source_handoff_physics_step: 1,
            terminal_entry_physics_step: 2,
            planned_end_physics_step: expected_contact_physics_step + 4,
            expected_contact_physics_step,
            updates,
        };
        (context, program)
    }

    #[test]
    fn playback_maps_initial_callback_to_tick_zero_and_holds_commands_for_two_steps() {
        let (context, program) = fixture(4);
        let artifacts = run_flight_program(&context, &program).unwrap();

        assert_eq!(artifacts.controller_updates.len(), 2);
        assert_eq!(artifacts.controller_updates[0].physics_step, 0);
        assert_eq!(
            artifacts.controller_updates[0].frame.phase.as_deref(),
            Some("upright")
        );
        assert_eq!(artifacts.controller_updates[1].physics_step, 2);
        assert_eq!(
            artifacts.controller_updates[1].frame.phase.as_deref(),
            Some("terminal_bridge")
        );
        assert_eq!(artifacts.run.actions[0].physics_step, 0);
        assert_eq!(artifacts.run.actions[1].physics_step, 2);
        assert_eq!(artifacts.run.manifest.physics_steps, 4);
        assert_eq!(artifacts.run.manifest.controller_updates, 2);
        assert_eq!(
            artifacts.controller_updates[0]
                .frame
                .metrics
                .get("flight_program.witness_identity"),
            Some(&TelemetryValue::Text("test-witness".to_owned()))
        );
    }

    #[test]
    fn playback_rejects_an_end_step_different_from_expected_contact() {
        let (context, program) = fixture(6);
        let error = run_flight_program(&context, &program).unwrap_err();

        assert!(matches!(error, SimulationError::InvalidContext(_)));
        assert!(error.to_string().contains("expected contact at 6"));
    }

    #[test]
    fn controller_reset_repeats_the_same_playback() {
        let (context, program) = fixture(4);
        let mut controller = FlightProgramController::new(program);
        let first = run_controller(&context, &mut controller).unwrap();
        let second = run_controller(&context, &mut controller).unwrap();

        assert_eq!(first.run, second.run);
        assert_eq!(
            first
                .controller_updates
                .iter()
                .map(|record| (&record.frame, record.physics_step))
                .collect::<Vec<_>>(),
            second
                .controller_updates
                .iter()
                .map(|record| (&record.frame, record.physics_step))
                .collect::<Vec<_>>()
        );
    }
}
