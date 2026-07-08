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
        EvaluationGoal::WaypointHandoff { .. } => apply_timed_checkpoint_contact(state, contact),
        EvaluationGoal::TimedCheckpoint { .. } => apply_timed_checkpoint_contact(state, contact),
    }
}

pub fn apply_progress_evaluation(
    ctx: &RunContext,
    state: &mut SimulationState,
) -> Vec<EventRecord> {
    match &ctx.mission.goal {
        EvaluationGoal::LandingOnPad { .. } => Vec::new(),
        EvaluationGoal::WaypointHandoff { waypoint_index, .. } => {
            apply_waypoint_handoff_progress(ctx, state, *waypoint_index)
        }
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

fn apply_waypoint_handoff_progress(
    ctx: &RunContext,
    state: &mut SimulationState,
    waypoint_index: usize,
) -> Vec<EventRecord> {
    let Some(evaluation) = waypoint_handoff_evaluation(ctx, state, waypoint_index) else {
        return Vec::new();
    };
    if !evaluation.triggered {
        return Vec::new();
    }

    if evaluation.contract_pass {
        state.mission_outcome = MissionOutcome::Success;
        state.end_reason = EndReason::CheckpointSatisfied;
        vec![
            EventRecord {
                sim_time_s: state.sim_time_s,
                physics_step: state.physics_step,
                kind: EventKind::CheckpointSatisfied,
                message: "waypoint_handoff_satisfied".to_owned(),
            },
            EventRecord {
                sim_time_s: state.sim_time_s,
                physics_step: state.physics_step,
                kind: EventKind::MissionEnded,
                message: "waypoint_handoff_satisfied".to_owned(),
            },
        ]
    } else {
        state.mission_outcome = MissionOutcome::FailedCheckpoint;
        state.end_reason = EndReason::CheckpointFailed;
        vec![
            EventRecord {
                sim_time_s: state.sim_time_s,
                physics_step: state.physics_step,
                kind: EventKind::CheckpointFailed,
                message: "waypoint_handoff_failed".to_owned(),
            },
            EventRecord {
                sim_time_s: state.sim_time_s,
                physics_step: state.physics_step,
                kind: EventKind::MissionEnded,
                message: "waypoint_handoff_failed".to_owned(),
            },
        ]
    }
}

#[derive(Clone, Copy, Debug)]
struct WaypointHandoffEvaluation {
    triggered: bool,
    contract_pass: bool,
}

fn waypoint_handoff_evaluation(
    ctx: &RunContext,
    state: &SimulationState,
    waypoint_index: usize,
) -> Option<WaypointHandoffEvaluation> {
    let route = ctx.mission.transfer_route.as_ref()?;
    let waypoint = route.waypoints.get(waypoint_index)?;
    let anchor_m = if waypoint_index == 0 {
        let source_pad = ctx.world.landing_pad(&route.source_pad_id)?;
        Vec2::new(source_pad.center_x_m, source_pad.surface_y_m)
    } else {
        route.waypoints.get(waypoint_index - 1)?.position_m
    };
    let target_m = waypoint.position_m;
    let next_target_m = route
        .waypoints
        .get(waypoint_index + 1)
        .map(|next| next.position_m)
        .unwrap_or_else(|| Vec2::new(ctx.target_pad.center_x_m, ctx.target_pad.surface_y_m));
    let leg_unit = normalized(target_m - anchor_m)?;
    let next_leg_unit = normalized(next_target_m - target_m)?;
    let to_waypoint_m = state.position_m - target_m;
    let speed_mps = state.velocity_mps.length();
    let velocity_unit = if speed_mps > 1.0e-9 {
        state.velocity_mps * (1.0 / speed_mps)
    } else {
        Vec2::new(0.0, 0.0)
    };
    let outbound_heading_error_rad = vec_dot(velocity_unit, next_leg_unit)
        .clamp(-1.0, 1.0)
        .acos();
    let distance_m = to_waypoint_m.length();
    let cross_track_m = vec_cross(to_waypoint_m, leg_unit).abs();
    let plane_progress_m = vec_dot(to_waypoint_m, leg_unit);
    let outbound_progress_mps = vec_dot(state.velocity_mps, next_leg_unit);
    let triggered = plane_progress_m >= 0.0 || distance_m <= waypoint.capture_radius_m;
    let spatial_pass = distance_m <= waypoint.capture_radius_m
        || (cross_track_m <= waypoint.max_cross_track_m
            && plane_progress_m >= -waypoint.capture_radius_m);
    let outbound_pass = outbound_heading_error_rad <= waypoint.max_outbound_heading_error_rad
        && outbound_progress_mps >= waypoint.min_outbound_progress_mps
        && speed_mps >= waypoint.min_speed_mps
        && speed_mps <= waypoint.max_speed_mps
        && state.velocity_mps.y >= waypoint.min_vertical_speed_mps
        && state.velocity_mps.y <= waypoint.max_vertical_speed_mps;

    Some(WaypointHandoffEvaluation {
        triggered,
        contract_pass: spatial_pass && outbound_pass,
    })
}

fn normalized(vector: Vec2) -> Option<Vec2> {
    let length = vector.length();
    (length > 1.0e-9).then(|| vector * (1.0 / length))
}

fn vec_dot(lhs: Vec2, rhs: Vec2) -> f64 {
    (lhs.x * rhs.x) + (lhs.y * rhs.y)
}

fn vec_cross(lhs: Vec2, rhs: Vec2) -> f64 {
    (lhs.x * rhs.y) - (lhs.y * rhs.x)
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
            LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig, TransferRouteSpec,
            TransferWaypointSpec, VehicleGeometry, VehicleInitialState, VehicleSpec, WorldSpec,
        },
        terrain::TerrainDefinition,
    };

    fn waypoint_handoff_scenario() -> ScenarioSpec {
        ScenarioSpec {
            id: "waypoint_handoff".to_owned(),
            name: "Waypoint handoff".to_owned(),
            description: "waypoint handoff test".to_owned(),
            seed: 3,
            tags: vec!["test".to_owned(), "waypoint".to_owned()],
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
                    points_m: vec![Vec2::new(-150.0, 0.0), Vec2::new(150.0, 0.0)],
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
                        center_x_m: 100.0,
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
                position_m: Vec2::new(-100.0, 20.0),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                transfer_route: Some(TransferRouteSpec {
                    source_pad_id: "source".to_owned(),
                    target_pad_id: "target".to_owned(),
                    route_angle_deg: 0.0,
                    route_radius_m: 200.0,
                    waypoints: vec![TransferWaypointSpec {
                        id: "wp_1".to_owned(),
                        position_m: Vec2::new(0.0, 0.0),
                        capture_radius_m: 10.0,
                        max_cross_track_m: 15.0,
                        max_outbound_heading_error_rad: 0.7,
                        min_outbound_progress_mps: 5.0,
                        min_speed_mps: 10.0,
                        max_speed_mps: 90.0,
                        min_vertical_speed_mps: -40.0,
                        max_vertical_speed_mps: 40.0,
                    }],
                }),
                goal: EvaluationGoal::WaypointHandoff {
                    target_pad_id: "target".to_owned(),
                    waypoint_index: 0,
                },
            },
        }
    }

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
                transfer_route: None,
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

    #[test]
    fn waypoint_handoff_succeeds_when_state_passes_contract() {
        let scenario = waypoint_handoff_scenario();
        let ctx = RunContext::from_scenario(&scenario).unwrap();
        let mut state = SimulationState::new(&ctx).unwrap();
        state.sim_time_s = 1.0;
        state.physics_step = 120;
        state.position_m = Vec2::new(0.0, 0.0);
        state.velocity_mps = Vec2::new(25.0, 0.0);

        let events = apply_progress_evaluation(&ctx, &mut state);

        assert!(matches!(state.mission_outcome, MissionOutcome::Success));
        assert!(matches!(state.end_reason, EndReason::CheckpointSatisfied));
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0].kind, EventKind::CheckpointSatisfied));
        assert_eq!(events[0].message, "waypoint_handoff_satisfied");
    }

    #[test]
    fn waypoint_handoff_fails_when_crossing_outside_spatial_envelope() {
        let scenario = waypoint_handoff_scenario();
        let ctx = RunContext::from_scenario(&scenario).unwrap();
        let mut state = SimulationState::new(&ctx).unwrap();
        state.sim_time_s = 1.0;
        state.physics_step = 120;
        state.position_m = Vec2::new(1.0, 30.0);
        state.velocity_mps = Vec2::new(25.0, 0.0);

        let events = apply_progress_evaluation(&ctx, &mut state);

        assert!(matches!(
            state.mission_outcome,
            MissionOutcome::FailedCheckpoint
        ));
        assert!(matches!(state.end_reason, EndReason::CheckpointFailed));
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0].kind, EventKind::CheckpointFailed));
        assert_eq!(events[0].message, "waypoint_handoff_failed");
    }

    #[test]
    fn waypoint_handoff_fails_when_outbound_state_is_unviable() {
        let scenario = waypoint_handoff_scenario();
        let ctx = RunContext::from_scenario(&scenario).unwrap();
        let mut state = SimulationState::new(&ctx).unwrap();
        state.sim_time_s = 1.0;
        state.physics_step = 120;
        state.position_m = Vec2::new(0.0, 0.0);
        state.velocity_mps = Vec2::new(-25.0, 0.0);

        let events = apply_progress_evaluation(&ctx, &mut state);

        assert!(matches!(
            state.mission_outcome,
            MissionOutcome::FailedCheckpoint
        ));
        assert!(matches!(state.end_reason, EndReason::CheckpointFailed));
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0].kind, EventKind::CheckpointFailed));
        assert_eq!(events[0].message, "waypoint_handoff_failed");
    }
}
