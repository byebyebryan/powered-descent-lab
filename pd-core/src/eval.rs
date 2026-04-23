use crate::{
    math::Vec2,
    model::{
        EndReason, EvaluationGoal, EventKind, EventRecord, MissionOutcome, PhysicalOutcome,
        RunContext,
    },
    sim::SimulationState,
};

#[derive(Clone, Debug, PartialEq)]
pub enum ContactClassification {
    None,
    StableTouchdown { on_target: bool },
    Crash,
}

pub fn apply_contact_classification(
    ctx: &RunContext,
    state: &mut SimulationState,
    contact: ContactClassification,
) -> Vec<EventRecord> {
    if matches!(contact, ContactClassification::None) {
        return Vec::new();
    }

    match &ctx.mission.goal {
        EvaluationGoal::LandingOnPad { .. } => apply_landing_goal(state, contact),
        EvaluationGoal::TimedCheckpoint { .. } => apply_timed_checkpoint_contact(state, contact),
    }
}

pub fn apply_progress_evaluation(
    ctx: &RunContext,
    state: &mut SimulationState,
) -> Vec<EventRecord> {
    match &ctx.mission.goal {
        EvaluationGoal::LandingOnPad { .. } => Vec::new(),
        EvaluationGoal::TimedCheckpoint {
            end_time_s,
            desired_position_offset_m,
            max_position_error_m,
            desired_velocity_mps,
            max_velocity_error_mps,
            max_attitude_error_rad,
            ..
        } => {
            if state.sim_time_s < *end_time_s {
                return Vec::new();
            }

            let actual_position_offset_m = Vec2::new(
                state.position_m.x - ctx.target_pad.center_x_m,
                state.position_m.y - ctx.target_pad.surface_y_m,
            );
            let position_error_m = (actual_position_offset_m - *desired_position_offset_m).length();
            let velocity_error_mps = (state.velocity_mps - *desired_velocity_mps).length();
            let attitude_ok = state.attitude_rad.abs() <= *max_attitude_error_rad;

            if position_error_m <= *max_position_error_m
                && velocity_error_mps <= *max_velocity_error_mps
                && attitude_ok
            {
                state.mission_outcome = MissionOutcome::Success;
                state.end_reason = EndReason::CheckpointSatisfied;
                return vec![
                    EventRecord {
                        sim_time_s: state.sim_time_s,
                        physics_step: state.physics_step,
                        kind: EventKind::CheckpointSatisfied,
                        message: "checkpoint_satisfied".to_owned(),
                    },
                    EventRecord {
                        sim_time_s: state.sim_time_s,
                        physics_step: state.physics_step,
                        kind: EventKind::MissionEnded,
                        message: "checkpoint_satisfied".to_owned(),
                    },
                ];
            }

            state.mission_outcome = MissionOutcome::FailedCheckpoint;
            state.end_reason = EndReason::CheckpointFailed;
            vec![
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::CheckpointFailed,
                    message: "checkpoint_failed".to_owned(),
                },
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::MissionEnded,
                    message: "checkpoint_failed".to_owned(),
                },
            ]
        }
    }
}

pub fn apply_max_time(state: &mut SimulationState) -> Vec<EventRecord> {
    state.physical_outcome = PhysicalOutcome::TimedOut;
    state.mission_outcome = MissionOutcome::FailedTimeout;
    state.end_reason = EndReason::MaxTimeReached;
    vec![
        EventRecord {
            sim_time_s: state.sim_time_s,
            physics_step: state.physics_step,
            kind: EventKind::MaxTimeReached,
            message: "max_time_reached".to_owned(),
        },
        EventRecord {
            sim_time_s: state.sim_time_s,
            physics_step: state.physics_step,
            kind: EventKind::MissionEnded,
            message: "max_time_reached".to_owned(),
        },
    ]
}

fn apply_landing_goal(
    state: &mut SimulationState,
    contact: ContactClassification,
) -> Vec<EventRecord> {
    match contact {
        ContactClassification::None => Vec::new(),
        ContactClassification::StableTouchdown { on_target: true } => {
            state.velocity_mps = Vec2::new(0.0, 0.0);
            state.angular_rate_radps = 0.0;
            state.physical_outcome = PhysicalOutcome::LandedOnTarget;
            state.mission_outcome = MissionOutcome::Success;
            state.end_reason = EndReason::TouchdownOnTarget;
            vec![
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::TouchdownOnTarget,
                    message: "touchdown_on_target".to_owned(),
                },
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::MissionEnded,
                    message: "touchdown_on_target".to_owned(),
                },
            ]
        }
        ContactClassification::StableTouchdown { on_target: false } => {
            state.velocity_mps = Vec2::new(0.0, 0.0);
            state.angular_rate_radps = 0.0;
            state.physical_outcome = PhysicalOutcome::LandedOffTarget;
            state.mission_outcome = MissionOutcome::FailedOffTarget;
            state.end_reason = EndReason::TouchdownOffTarget;
            vec![
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::TouchdownOffTarget,
                    message: "touchdown_off_target".to_owned(),
                },
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::MissionEnded,
                    message: "touchdown_off_target".to_owned(),
                },
            ]
        }
        ContactClassification::Crash => {
            state.physical_outcome = PhysicalOutcome::Crashed;
            state.mission_outcome = MissionOutcome::FailedCrash;
            state.end_reason = EndReason::Crash;
            vec![
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::Crash,
                    message: "crash".to_owned(),
                },
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::MissionEnded,
                    message: "crash".to_owned(),
                },
            ]
        }
    }
}

fn apply_timed_checkpoint_contact(
    state: &mut SimulationState,
    contact: ContactClassification,
) -> Vec<EventRecord> {
    match contact {
        ContactClassification::None => Vec::new(),
        ContactClassification::StableTouchdown { on_target: true } => {
            state.velocity_mps = Vec2::new(0.0, 0.0);
            state.angular_rate_radps = 0.0;
            state.physical_outcome = PhysicalOutcome::LandedOnTarget;
            state.mission_outcome = MissionOutcome::FailedCheckpoint;
            state.end_reason = EndReason::TouchdownOnTarget;
            vec![
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::TouchdownOnTarget,
                    message: "touchdown_on_target".to_owned(),
                },
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::MissionEnded,
                    message: "touchdown_on_target".to_owned(),
                },
            ]
        }
        ContactClassification::StableTouchdown { on_target: false } => {
            state.velocity_mps = Vec2::new(0.0, 0.0);
            state.angular_rate_radps = 0.0;
            state.physical_outcome = PhysicalOutcome::LandedOffTarget;
            state.mission_outcome = MissionOutcome::FailedCheckpoint;
            state.end_reason = EndReason::TouchdownOffTarget;
            vec![
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::TouchdownOffTarget,
                    message: "touchdown_off_target".to_owned(),
                },
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::MissionEnded,
                    message: "touchdown_off_target".to_owned(),
                },
            ]
        }
        ContactClassification::Crash => {
            state.physical_outcome = PhysicalOutcome::Crashed;
            state.mission_outcome = MissionOutcome::FailedCrash;
            state.end_reason = EndReason::Crash;
            vec![
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::Crash,
                    message: "crash".to_owned(),
                },
                EventRecord {
                    sim_time_s: state.sim_time_s,
                    physics_step: state.physics_step,
                    kind: EventKind::MissionEnded,
                    message: "crash".to_owned(),
                },
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        model::{
            LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig, VehicleGeometry,
            VehicleInitialState, VehicleSpec, WorldSpec,
        },
        terrain::TerrainDefinition,
    };

    #[test]
    fn timed_checkpoint_succeeds_when_state_is_inside_envelope() {
        let scenario = ScenarioSpec {
            id: "timed_checkpoint".to_owned(),
            name: "Timed checkpoint".to_owned(),
            description: "checkpoint test".to_owned(),
            seed: 2,
            tags: vec!["test".to_owned(), "checkpoint".to_owned()],
            metadata: BTreeMap::from([("suite".to_owned(), "unit".to_owned())]),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 10.0,
                sample_hz: Some(10),
            },
            world: WorldSpec {
                gravity_mps2: 1.62,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-50.0, 0.0), Vec2::new(50.0, 0.0)],
                },
                landing_pads: vec![LandingPadSpec {
                    id: "pad_a".to_owned(),
                    center_x_m: 0.0,
                    surface_y_m: 0.0,
                    width_m: 30.0,
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
                initial_fuel_kg: 200.0,
                max_fuel_kg: 200.0,
                max_thrust_n: 14_000.0,
                max_fuel_burn_kgps: 10.0,
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.0,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(0.0, 20.0),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                goal: EvaluationGoal::TimedCheckpoint {
                    target_pad_id: "pad_a".to_owned(),
                    end_time_s: 1.0,
                    desired_position_offset_m: Vec2::new(0.0, 10.0),
                    max_position_error_m: 0.5,
                    desired_velocity_mps: Vec2::new(0.0, 0.0),
                    max_velocity_error_mps: 0.5,
                    max_attitude_error_rad: 0.1,
                },
            },
        };

        let ctx = RunContext::from_scenario(&scenario).unwrap();
        let mut state = SimulationState::new(&ctx).unwrap();
        state.sim_time_s = 1.0;
        state.physics_step = 120;
        state.position_m = Vec2::new(0.0, 10.0);
        state.velocity_mps = Vec2::new(0.0, 0.0);
        state.attitude_rad = 0.0;

        let events = apply_progress_evaluation(&ctx, &mut state);

        assert!(matches!(state.mission_outcome, MissionOutcome::Success));
        assert!(matches!(state.end_reason, EndReason::CheckpointSatisfied));
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0].kind, EventKind::CheckpointSatisfied));
    }
}
