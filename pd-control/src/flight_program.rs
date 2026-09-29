use std::collections::BTreeMap;
use std::time::Instant;

use pd_core::{
    ActionLogEntry, BoundedGuardFailureDispositionV1, BoundedRunArtifactsV1,
    BoundedRunFailureStageV1, BoundedRunFailureV1, BoundedRunGuard, BoundedRunLimitsV1,
    BoundedRunNonFiniteCategoryV1, BoundedRunStopCauseV1, Command, FlightProgramUpdateV1,
    FlightProgramV1, Observation, RunContext, SimulationError, run_simulation_bounded,
};
use serde::{Deserialize, Serialize};

use crate::{
    ControlledRunArtifacts, Controller, ControllerFrame, ControllerUpdateRecord,
    RunPerformanceStats, TelemetryValue, run_controller,
};

pub const OPERATIONAL_FLIGHT_PROGRAM_CONTROLLER_ID: &str = "flight_program_operational_v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalFlightProgramArtifactsV1 {
    pub bounded: BoundedRunArtifactsV1,
    pub controller_updates: Vec<ControllerUpdateRecord>,
    pub performance: RunPerformanceStats,
}

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

/// Executes a validated program only through its full saved-command coverage
/// and planned hard end. This opt-in path leaves strict V1 playback unchanged.
pub fn run_flight_program_operational(
    context: &RunContext,
    program: &FlightProgramV1,
    guard: &mut dyn BoundedRunGuard,
) -> Result<OperationalFlightProgramArtifactsV1, SimulationError> {
    program
        .validate_against_context(context)
        .map_err(SimulationError::InvalidContext)?;

    run_operational_with_updates(context, program, guard, true, |index, observation| {
        let Some(update) = program.updates.get(index) else {
            return Err(format!(
                "flight program has no saved command for bounded callback {index} at physics step {}",
                observation.physics_step
            ));
        };
        Ok(update.clone())
    })
}

/// Replays the supplied action prefix against limits and commands derived from
/// the complete program, never from the observed prefix length.
pub fn replay_flight_program_operational(
    context: &RunContext,
    program: &FlightProgramV1,
    actions: &[ActionLogEntry],
    guard: &mut dyn BoundedRunGuard,
) -> Result<OperationalFlightProgramArtifactsV1, SimulationError> {
    program
        .validate_against_context(context)
        .map_err(SimulationError::InvalidContext)?;

    let mut nonfinite_action_failure = None;
    let mut artifacts =
        run_operational_with_updates(context, program, guard, false, |index, observation| {
            let Some(action) = actions.get(index) else {
                return Err(format!(
                    "action log ended before program update {index} at physics step {}",
                    observation.physics_step
                ));
            };
            if let Some(failure) = nonfinite_value_failure(
                "action.sim_time_s",
                action.sim_time_s,
                observation.physics_step,
            ) {
                let reason = failure.reason.clone();
                nonfinite_action_failure = Some(failure);
                return Err(reason);
            }
            if let Some(failure) =
                nonfinite_command_failure(action.command, observation.physics_step)
            {
                let reason = failure.reason.clone();
                nonfinite_action_failure = Some(failure);
                return Err(reason);
            }

            let Some(update) = program.updates.get(index) else {
                return Err(format!(
                    "program has no full-program update {index} at physics step {}",
                    observation.physics_step
                ));
            };
            validate_program_action(action, index, update, observation)?;
            Ok(update.clone())
        })?;

    if let Some(failure) = nonfinite_action_failure {
        artifacts.bounded.stop = BoundedRunStopCauseV1::ExecutionInvalid;
        artifacts.bounded.failure = Some(failure);
    } else if let Some(failure) = artifacts.bounded.failure.as_mut()
        && failure.stage == BoundedRunFailureStageV1::CommandSelection
    {
        failure.stage = BoundedRunFailureStageV1::ReplayInput;
    }

    if artifacts.bounded.failure.is_none() && actions.len() != artifacts.controller_updates.len() {
        let unused = actions
            .len()
            .saturating_sub(artifacts.controller_updates.len());
        let failure = BoundedRunFailureV1::new(
            BoundedRunFailureStageV1::ReplayInput,
            BoundedGuardFailureDispositionV1::ExecutionInvalid,
            artifacts.bounded.final_state.physics_step,
            format!(
                "action log contains {unused} unused controller updates after bounded termination"
            ),
        );
        artifacts.bounded.stop = BoundedRunStopCauseV1::ExecutionInvalid;
        artifacts.bounded.failure = Some(failure);
    } else if artifacts.bounded.failure.as_ref().is_some_and(|failure| {
        failure.disposition == BoundedGuardFailureDispositionV1::SafetyRejected
            && actions.len() != artifacts.controller_updates.len()
    }) {
        let prior = artifacts.bounded.failure.as_ref().expect("checked above");
        let unused = actions
            .len()
            .saturating_sub(artifacts.controller_updates.len());
        artifacts.bounded.failure = Some(BoundedRunFailureV1::new(
            BoundedRunFailureStageV1::ReplayInput,
            BoundedGuardFailureDispositionV1::ExecutionInvalid,
            artifacts.bounded.final_state.physics_step,
            format!(
                "action log contains {unused} unused controller updates after a safety rejection at physics step {}: {}",
                prior.boundary_physics_step, prior.reason
            ),
        ));
        artifacts.bounded.stop = BoundedRunStopCauseV1::ExecutionInvalid;
    }

    Ok(artifacts)
}

fn run_operational_with_updates<F>(
    context: &RunContext,
    program: &FlightProgramV1,
    guard: &mut dyn BoundedRunGuard,
    measure_controller_compute: bool,
    mut select_update: F,
) -> Result<OperationalFlightProgramArtifactsV1, SimulationError>
where
    F: FnMut(usize, &Observation) -> Result<FlightProgramUpdateV1, String>,
{
    let update_count = u64::try_from(program.updates.len()).map_err(|_| {
        SimulationError::InvalidContext(
            "flight program update count is not representable".to_owned(),
        )
    })?;
    let coverage_end = update_count
        .checked_mul(context.sim.control_interval_steps())
        .ok_or_else(|| {
            SimulationError::InvalidContext("flight program coverage end overflows".to_owned())
        })?;
    let limits = BoundedRunLimitsV1 {
        command_coverage_end_physics_step: coverage_end,
        hard_end_physics_step: program.planned_end_physics_step,
    };
    let mut controller_updates = Vec::with_capacity(program.updates.len());
    let wall_started_at = Instant::now();
    let thread_cpu_started_at = crate::current_thread_cpu_time_us();
    let bounded = run_simulation_bounded(
        context,
        OPERATIONAL_FLIGHT_PROGRAM_CONTROLLER_ID,
        limits,
        |_, observation| {
            let index = controller_updates.len();
            let started_at = Instant::now();
            let update = select_update(index, observation)?;
            let compute_time_us = measure_controller_compute
                .then(|| started_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64);
            if update.physics_step != observation.physics_step {
                return Err(format!(
                    "program callback {index} expected physics step {}, got {}",
                    observation.physics_step, update.physics_step
                ));
            }
            let update_index = u64::try_from(index)
                .map_err(|_| "controller update index is not representable".to_owned())?;
            controller_updates.push(ControllerUpdateRecord {
                sim_time_s: observation.sim_time_s,
                physics_step: observation.physics_step,
                controller_update_index: update_index,
                compute_time_us,
                frame: operational_program_frame(program, &update),
            });
            Ok(update.command)
        },
        guard,
    )?;

    Ok(OperationalFlightProgramArtifactsV1 {
        bounded,
        controller_updates,
        performance: RunPerformanceStats {
            wall_time_us: wall_started_at
                .elapsed()
                .as_micros()
                .min(u128::from(u64::MAX)) as u64,
            thread_cpu_time_us: crate::current_thread_cpu_time_us()
                .zip(thread_cpu_started_at)
                .map(|(finished, started)| finished.saturating_sub(started)),
        },
    })
}

fn operational_program_frame(
    program: &FlightProgramV1,
    update: &FlightProgramUpdateV1,
) -> ControllerFrame {
    let status = format!(
        "flight program {} -> {} | generation={} | terminal={} | witness={}",
        program.source_pad_id,
        program.target_pad_id,
        program.generation_policy_identity,
        program.terminal_policy_identity,
        program.witness_identity,
    );
    let metrics = BTreeMap::from([
        (
            "flight_program.source_pad_id".to_owned(),
            TelemetryValue::Text(program.source_pad_id.clone()),
        ),
        (
            "flight_program.target_pad_id".to_owned(),
            TelemetryValue::Text(program.target_pad_id.clone()),
        ),
        (
            "flight_program.generation_policy_identity".to_owned(),
            TelemetryValue::Text(program.generation_policy_identity.clone()),
        ),
        (
            "flight_program.terminal_policy_identity".to_owned(),
            TelemetryValue::Text(program.terminal_policy_identity.clone()),
        ),
        (
            "flight_program.witness_identity".to_owned(),
            TelemetryValue::Text(program.witness_identity.clone()),
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

fn validate_program_action(
    action: &ActionLogEntry,
    index: usize,
    update: &FlightProgramUpdateV1,
    observation: &Observation,
) -> Result<(), String> {
    let update_index =
        u64::try_from(index).map_err(|_| format!("action index {index} is not representable"))?;
    if action.controller_update_index != update_index {
        return Err(format!(
            "action {index} expected controller_update_index {update_index}, got {}",
            action.controller_update_index
        ));
    }
    if action.physics_step != update.physics_step || update.physics_step != observation.physics_step
    {
        return Err(format!(
            "action {index} does not match full-program physics step {} at callback {}",
            update.physics_step, observation.physics_step
        ));
    }
    if action.sim_time_s.to_bits() != observation.sim_time_s.to_bits() {
        return Err(format!(
            "action {index} has a noncanonical simulation timestamp at physics step {}",
            observation.physics_step
        ));
    }
    if !commands_match_exactly(action.command, update.command) {
        return Err(format!(
            "action {index} command differs from the validated full-program prefix"
        ));
    }
    Ok(())
}

fn commands_match_exactly(lhs: Command, rhs: Command) -> bool {
    lhs.throttle_frac.to_bits() == rhs.throttle_frac.to_bits()
        && lhs.target_attitude_rad.to_bits() == rhs.target_attitude_rad.to_bits()
}

fn nonfinite_command_failure(
    command: Command,
    boundary_physics_step: u64,
) -> Option<BoundedRunFailureV1> {
    [
        ("action.command.throttle_frac", command.throttle_frac),
        (
            "action.command.target_attitude_rad",
            command.target_attitude_rad,
        ),
    ]
    .into_iter()
    .find_map(|(field, value)| nonfinite_value_failure(field, value, boundary_physics_step))
}

fn nonfinite_value_failure(
    field: impl Into<String>,
    value: f64,
    boundary_physics_step: u64,
) -> Option<BoundedRunFailureV1> {
    let category = if value.is_nan() {
        BoundedRunNonFiniteCategoryV1::Nan
    } else if value == f64::INFINITY {
        BoundedRunNonFiniteCategoryV1::PositiveInfinity
    } else if value == f64::NEG_INFINITY {
        BoundedRunNonFiniteCategoryV1::NegativeInfinity
    } else {
        return None;
    };
    Some(BoundedRunFailureV1::non_finite(
        BoundedRunFailureStageV1::ReplayInput,
        field,
        category,
        boundary_physics_step,
    ))
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
        BoundedGuardFailureDispositionV1, BoundedGuardFailureV1, BoundedRunGuard,
        BoundedRunStopCauseV1, EndReason, EvaluationGoal, FlightProgramUpdateV1, LandingPadSpec,
        MissionSpec, RunContext, ScenarioSpec, SimConfig, TerrainDefinition, TransferRouteSpec,
        Vec2, VehicleGeometry, VehicleInitialState, VehicleSpec, WorldSpec,
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

    fn lifted_program_fixture() -> (RunContext, FlightProgramV1) {
        let (mut context, mut program) = fixture(4);
        context.initial_state.position_m.y += 100.0;
        program.binding = pd_core::FlightProgramBindingV1::from_context(&context);
        (context, program)
    }

    #[derive(Default)]
    struct InitialSafetyReject;

    impl BoundedRunGuard for InitialSafetyReject {
        fn initial(
            &mut self,
            _context: &RunContext,
            _state: &pd_core::SimulationState,
        ) -> Result<(), BoundedGuardFailureV1> {
            Err(BoundedGuardFailureV1::new(
                BoundedGuardFailureDispositionV1::SafetyRejected,
                "fixture initial safety rejection",
            ))
        }
    }

    #[test]
    fn operational_program_stops_at_full_saved_coverage_and_replays_prefix() {
        let (context, program) = lifted_program_fixture();
        let mut guard = pd_core::AllowAllBoundedRunGuard;
        let run = run_flight_program_operational(&context, &program, &mut guard).unwrap();

        assert_eq!(run.bounded.limits.command_coverage_end_physics_step, 4);
        assert_eq!(run.bounded.limits.hard_end_physics_step, 8);
        assert_eq!(run.bounded.stop, BoundedRunStopCauseV1::CoverageExhausted);
        assert_eq!(run.bounded.final_state.physics_step, 4);
        assert!(!run.bounded.hard_end_reached);
        assert_eq!(run.controller_updates.len(), 2);
        assert_eq!(run.bounded.run.actions.len(), 2);
        assert_eq!(run.controller_updates[0].physics_step, 0);
        assert_eq!(run.controller_updates[1].physics_step, 2);
        assert_eq!(run.bounded.run.manifest.end_reason, EndReason::Running);

        let mut replay_guard = pd_core::AllowAllBoundedRunGuard;
        let replay = replay_flight_program_operational(
            &context,
            &program,
            &run.bounded.run.actions,
            &mut replay_guard,
        )
        .unwrap();
        assert_eq!(replay.bounded.run, run.bounded.run);
        assert_eq!(replay.bounded.final_state, run.bounded.final_state);
        assert_eq!(replay.bounded.stop, run.bounded.stop);
        assert_eq!(
            replay.controller_updates.len(),
            run.controller_updates.len()
        );
        assert!(
            replay
                .controller_updates
                .iter()
                .all(|record| record.compute_time_us.is_none())
        );

        let mut truncated_guard = pd_core::AllowAllBoundedRunGuard;
        let truncated = replay_flight_program_operational(
            &context,
            &program,
            &run.bounded.run.actions[..1],
            &mut truncated_guard,
        )
        .unwrap();
        assert_eq!(
            truncated.bounded.stop,
            BoundedRunStopCauseV1::ExecutionInvalid
        );
        assert_eq!(truncated.bounded.final_state.physics_step, 2);
        assert_eq!(truncated.bounded.run.actions.len(), 1);
        assert_eq!(
            truncated.bounded.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::ReplayInput
        );

        let mut empty_guard = pd_core::AllowAllBoundedRunGuard;
        let empty =
            replay_flight_program_operational(&context, &program, &[], &mut empty_guard).unwrap();
        assert_eq!(empty.bounded.stop, BoundedRunStopCauseV1::ExecutionInvalid);
        assert_eq!(empty.bounded.final_state.physics_step, 0);
        assert!(empty.bounded.run.actions.is_empty());
    }

    #[test]
    fn operational_replay_rejects_altered_commands_and_extra_suffixes() {
        let (context, program) = lifted_program_fixture();
        let mut run_guard = pd_core::AllowAllBoundedRunGuard;
        let run = run_flight_program_operational(&context, &program, &mut run_guard).unwrap();

        let mut altered_actions = run.bounded.run.actions.clone();
        altered_actions[0].command.throttle_frac = 0.125;
        let mut altered_guard = pd_core::AllowAllBoundedRunGuard;
        let altered = replay_flight_program_operational(
            &context,
            &program,
            &altered_actions,
            &mut altered_guard,
        )
        .unwrap();
        assert_eq!(
            altered.bounded.stop,
            BoundedRunStopCauseV1::ExecutionInvalid
        );
        assert_eq!(altered.bounded.final_state.physics_step, 0);
        assert!(altered.bounded.run.actions.is_empty());
        assert_eq!(
            altered.bounded.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::ReplayInput
        );

        let mut nonfinite_timestamp_actions = run.bounded.run.actions.clone();
        nonfinite_timestamp_actions[0].sim_time_s = f64::NAN;
        let mut nonfinite_guard = pd_core::AllowAllBoundedRunGuard;
        let nonfinite_timestamp = replay_flight_program_operational(
            &context,
            &program,
            &nonfinite_timestamp_actions,
            &mut nonfinite_guard,
        )
        .unwrap();
        let timestamp_failure = nonfinite_timestamp.bounded.failure.as_ref().unwrap();
        assert_eq!(
            nonfinite_timestamp.bounded.stop,
            BoundedRunStopCauseV1::ExecutionInvalid
        );
        assert_eq!(
            timestamp_failure.non_finite.as_ref().unwrap().field,
            "action.sim_time_s"
        );
        assert_eq!(
            timestamp_failure.non_finite.as_ref().unwrap().category,
            pd_core::BoundedRunNonFiniteCategoryV1::Nan
        );
        assert!(nonfinite_timestamp.bounded.run.actions.is_empty());

        let mut extra_actions = run.bounded.run.actions.clone();
        extra_actions.push(ActionLogEntry {
            sim_time_s: 4.0 / f64::from(context.sim.physics_hz),
            physics_step: 4,
            controller_update_index: 2,
            command: program.updates[1].command,
        });
        let mut extra_guard = pd_core::AllowAllBoundedRunGuard;
        let extra =
            replay_flight_program_operational(&context, &program, &extra_actions, &mut extra_guard)
                .unwrap();
        assert_eq!(extra.bounded.stop, BoundedRunStopCauseV1::ExecutionInvalid);
        assert_eq!(extra.bounded.final_state.physics_step, 4);
        assert_eq!(extra.bounded.run.actions.len(), 2);
        assert_eq!(
            extra.bounded.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::ReplayInput
        );
    }

    #[test]
    fn operational_early_contact_accepts_only_the_recomputed_action_prefix() {
        let (mut context, mut program) = fixture(4);
        context.initial_state.position_m.y = 3.2005;
        program.binding = pd_core::FlightProgramBindingV1::from_context(&context);

        let mut run_guard = pd_core::AllowAllBoundedRunGuard;
        let run = run_flight_program_operational(&context, &program, &mut run_guard).unwrap();
        assert_eq!(run.bounded.stop, BoundedRunStopCauseV1::MissionTerminal);
        assert_eq!(run.bounded.final_state.physics_step, 1);
        assert!(run.bounded.incoming_contact.is_some());
        assert!(!run.bounded.coverage_reached);
        assert_eq!(run.bounded.run.actions.len(), 1);

        let strict_error = run_flight_program(&context, &program).unwrap_err();
        assert!(strict_error.to_string().contains("expected contact at 4"));

        let mut replay_guard = pd_core::AllowAllBoundedRunGuard;
        let replay = replay_flight_program_operational(
            &context,
            &program,
            &run.bounded.run.actions,
            &mut replay_guard,
        )
        .unwrap();
        assert_eq!(replay.bounded.stop, BoundedRunStopCauseV1::MissionTerminal);
        assert_eq!(replay.bounded.run, run.bounded.run);
        assert!(replay.bounded.failure.is_none());

        let mut empty_guard = pd_core::AllowAllBoundedRunGuard;
        let empty =
            replay_flight_program_operational(&context, &program, &[], &mut empty_guard).unwrap();
        assert_eq!(empty.bounded.stop, BoundedRunStopCauseV1::ExecutionInvalid);
        assert_eq!(empty.bounded.final_state.physics_step, 0);
        assert_eq!(
            empty.bounded.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::ReplayInput
        );
    }

    #[test]
    fn operational_odd_marker_allows_late_contact_at_rounded_coverage_but_strict_stays_strict() {
        let (mut context, mut program) = fixture(5);
        let dt_s = 1.0 / f64::from(context.sim.physics_hz);
        let fifth_step_fall = context.world.gravity_mps2 * dt_s * dt_s * 15.0;
        context.initial_state.position_m.y = 3.2 + fifth_step_fall + 0.001;
        program.binding = pd_core::FlightProgramBindingV1::from_context(&context);

        let strict_error = run_flight_program(&context, &program).unwrap_err();
        assert!(strict_error.to_string().contains("expected contact at 5"));

        let mut guard = pd_core::AllowAllBoundedRunGuard;
        let operational = run_flight_program_operational(&context, &program, &mut guard).unwrap();
        assert_eq!(
            operational.bounded.limits.command_coverage_end_physics_step,
            6
        );
        assert_eq!(operational.bounded.limits.hard_end_physics_step, 9);
        assert_eq!(operational.bounded.final_state.physics_step, 6);
        assert_eq!(
            operational.bounded.stop,
            BoundedRunStopCauseV1::MissionTerminal
        );
        assert!(operational.bounded.coverage_reached);
        assert!(!operational.bounded.hard_end_reached);
        assert_eq!(operational.bounded.run.actions.len(), 3);
        assert_eq!(operational.controller_updates.len(), 3);
        assert!(operational.bounded.incoming_contact.is_some());

        let mut replay_guard = pd_core::AllowAllBoundedRunGuard;
        let replay = replay_flight_program_operational(
            &context,
            &program,
            &operational.bounded.run.actions,
            &mut replay_guard,
        )
        .unwrap();
        assert_eq!(replay.bounded.run, operational.bounded.run);
        assert_eq!(replay.bounded.stop, BoundedRunStopCauseV1::MissionTerminal);
    }

    #[test]
    fn operational_empty_log_is_accepted_only_for_recomputed_initial_stop() {
        let (context, program) = lifted_program_fixture();
        let mut initial_guard = InitialSafetyReject;
        let stopped =
            run_flight_program_operational(&context, &program, &mut initial_guard).unwrap();
        assert_eq!(stopped.bounded.stop, BoundedRunStopCauseV1::SafetyRejected);
        assert!(stopped.bounded.run.actions.is_empty());

        let mut replay_guard = InitialSafetyReject;
        let replay =
            replay_flight_program_operational(&context, &program, &[], &mut replay_guard).unwrap();
        assert_eq!(replay.bounded.stop, BoundedRunStopCauseV1::SafetyRejected);
        assert!(replay.bounded.failure.is_some());
        assert!(replay.bounded.run.actions.is_empty());

        let extra = [ActionLogEntry {
            sim_time_s: 0.0,
            physics_step: 0,
            controller_update_index: 0,
            command: program.updates[0].command,
        }];
        let mut extra_guard = InitialSafetyReject;
        let invalid =
            replay_flight_program_operational(&context, &program, &extra, &mut extra_guard)
                .unwrap();
        assert_eq!(
            invalid.bounded.stop,
            BoundedRunStopCauseV1::ExecutionInvalid
        );
        assert!(
            invalid
                .bounded
                .failure
                .as_ref()
                .unwrap()
                .reason
                .contains("fixture initial safety rejection")
        );
    }
}
