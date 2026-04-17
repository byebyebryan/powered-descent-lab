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
