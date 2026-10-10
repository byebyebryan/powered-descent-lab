use pd_core::{
    BoundedRunLimitsV1, FlightProgramBindingV1, FlightProgramUpdateV1, Observation, RunContext,
    SimulationError,
};
use serde::{Deserialize, Serialize};

use crate::{
    Agent, AgentDescriptorV1, AgentFrame, BoundedAgentRunArtifactsV1, MissionView, run_agent,
};

/// An ordinary prepared command schedule. This is not a planner witness or a
/// proof of feasibility. Execution retains the authoritative bounded stop.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedProgramV1 {
    pub schema_version: u32,
    pub binding: FlightProgramBindingV1,
    pub descriptor: AgentDescriptorV1,
    pub limits: BoundedRunLimitsV1,
    pub updates: Vec<FlightProgramUpdateV1>,
}

impl PreparedProgramV1 {
    pub fn validate(&self, ctx: &RunContext) -> Result<(), String> {
        self.descriptor.validate()?;
        if self.schema_version != 1 {
            return Err("unsupported prepared program version".to_owned());
        }
        let binding = FlightProgramBindingV1 {
            sim: ctx.sim.clone(),
            world: ctx.world.clone(),
            vehicle: ctx.vehicle.clone(),
            initial_state: ctx.initial_state.clone(),
            mission: ctx.mission.clone(),
        };
        if self.binding != binding {
            return Err("prepared program physical binding differs from mission".to_owned());
        }
        let interval = ctx.sim.control_interval_steps();
        if !self
            .limits
            .command_coverage_end_physics_step
            .is_multiple_of(interval)
            || self.updates.len() as u64 != self.limits.command_coverage_end_physics_step / interval
        {
            return Err(
                "prepared program must cover each control boundary exactly once".to_owned(),
            );
        }
        for (index, update) in self.updates.iter().enumerate() {
            if update.physics_step != index as u64 * interval
                || !update.command.throttle_frac.is_finite()
                || !(0.0..=1.0).contains(&update.command.throttle_frac)
                || !update.command.target_attitude_rad.is_finite()
                || update.command.target_attitude_rad.abs() > std::f64::consts::PI
            {
                return Err(format!("invalid prepared program update {index}"));
            }
        }
        Ok(())
    }
}

struct PreparedAgent<'a> {
    program: &'a PreparedProgramV1,
    next: usize,
}

impl Agent for PreparedAgent<'_> {
    fn descriptor(&self) -> AgentDescriptorV1 {
        self.program.descriptor.clone()
    }

    fn update(
        &mut self,
        _: MissionView<'_>,
        observation: &Observation,
    ) -> Result<AgentFrame, String> {
        let update = self
            .program
            .updates
            .get(self.next)
            .ok_or("program exhausted")?;
        if update.physics_step != observation.physics_step {
            return Err("prepared program command clock mismatch".to_owned());
        }
        self.next += 1;
        let mut frame = AgentFrame::command_only(update.command);
        frame.phase = Some(update.phase.clone());
        Ok(frame)
    }
}

pub fn run_prepared_program(
    ctx: &RunContext,
    program: &PreparedProgramV1,
) -> Result<BoundedAgentRunArtifactsV1, SimulationError> {
    program
        .validate(ctx)
        .map_err(SimulationError::InvalidContext)?;
    run_agent(ctx, &mut PreparedAgent { program, next: 0 }, program.limits)
}
