use std::time::Instant;

use pd_core::{
    BoundedGuardFailureDispositionV1, BoundedGuardFailureV1, BoundedRunArtifactsV1,
    BoundedRunGuard, BoundedRunLimitsV1, Command, IncomingContactV1, LandingPadSpec, MissionSpec,
    Observation, RunContext, SimConfig, SimulationError, SimulationState, VehicleInitialState,
    VehicleSpec, WorldSpec, run_simulation_bounded,
};
use serde::{Deserialize, Serialize};

use crate::{AgentFrame, AgentUpdateRecord, RunPerformanceStats, current_thread_cpu_time_us};
use crate::{CaptureModeV1, Episode, EpisodeResultV1};

/// Serializable identity, independent of the application's built-in registry.
/// Configuration and checkpoint identity are recorded before execution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDescriptorV1 {
    pub schema_version: u32,
    pub implementation: String,
    pub configuration: serde_json::Value,
    pub checkpoint_sha256: Option<String>,
}

impl AgentDescriptorV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.implementation.trim().is_empty() {
            return Err("unsupported agent descriptor version or empty implementation".to_owned());
        }
        if let Some(digest) = &self.checkpoint_sha256
            && (digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err("checkpoint identity must be a SHA-256 hex digest".to_owned());
        }
        Ok(())
    }
}

/// Physical inputs visible at decision time. Labels, seeds, tags and evaluation
/// metadata stay outside this view; identifiers here refer to physical pad/route
/// entities rather than generator membership.
#[derive(Clone, Copy, Debug)]
pub struct MissionView<'a> {
    pub sim: &'a SimConfig,
    pub world: &'a WorldSpec,
    pub vehicle: &'a VehicleSpec,
    pub initial_state: &'a VehicleInitialState,
    pub mission: &'a MissionSpec,
    pub target_pad: &'a LandingPadSpec,
}

impl<'a> From<&'a RunContext> for MissionView<'a> {
    fn from(ctx: &'a RunContext) -> Self {
        Self {
            sim: &ctx.sim,
            world: &ctx.world,
            vehicle: &ctx.vehicle,
            initial_state: &ctx.initial_state,
            mission: &ctx.mission,
            target_pad: &ctx.target_pad,
        }
    }
}

/// An open extension point. No strategy registration or existing controller is
/// needed. Errors produce a typed bounded stop and retain the valid prefix.
pub trait Agent {
    fn descriptor(&self) -> AgentDescriptorV1;
    fn reset(&mut self, _mission: MissionView<'_>) -> Result<(), String> {
        Ok(())
    }
    fn update(
        &mut self,
        mission: MissionView<'_>,
        observation: &Observation,
    ) -> Result<AgentFrame, String>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedAgentRunArtifactsV1 {
    pub descriptor: AgentDescriptorV1,
    pub episode: BoundedRunArtifactsV1,
    pub controller_updates: Vec<AgentUpdateRecord>,
    pub performance: RunPerformanceStats,
}

pub fn run_agent(
    ctx: &RunContext,
    agent: &mut dyn Agent,
    limits: BoundedRunLimitsV1,
) -> Result<BoundedAgentRunArtifactsV1, SimulationError> {
    let result = run_agent_with_capture_mode(ctx, agent, limits, CaptureModeV1::Captured)?;
    Ok(BoundedAgentRunArtifactsV1 {
        descriptor: result.descriptor,
        episode: result.episode.capture.expect("captured mode"),
        controller_updates: result.episode.agent_updates.expect("captured mode"),
        performance: result.performance,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentEpisodeResultV1 {
    pub descriptor: AgentDescriptorV1,
    pub episode: EpisodeResultV1,
    pub performance: RunPerformanceStats,
}

pub fn run_agent_with_capture_mode(
    ctx: &RunContext,
    agent: &mut dyn Agent,
    limits: BoundedRunLimitsV1,
    mode: CaptureModeV1,
) -> Result<AgentEpisodeResultV1, SimulationError> {
    let descriptor = agent.descriptor();
    descriptor
        .validate()
        .map_err(SimulationError::InvalidContext)?;
    let reset_failure = agent.reset(ctx.into()).err();
    let wall_started_at = Instant::now();
    let cpu_started = current_thread_cpu_time_us();
    let mut episode = Episode::from_context(ctx, &descriptor.implementation, limits, mode)?;
    if let Some(reason) = reset_failure {
        episode.reset_failed(format!("agent reset failed: {reason}"));
    }
    while episode.stop().is_none() {
        let started = Instant::now();
        match agent.update(episode.mission(), &episode.observation()) {
            Ok(frame) => {
                episode.step_frame(
                    frame,
                    Some(started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64),
                )?;
            }
            Err(reason) => {
                episode.decision_failed(reason)?;
            }
        }
    }
    Ok(AgentEpisodeResultV1 {
        descriptor,
        episode: episode.finish()?,
        performance: RunPerformanceStats {
            wall_time_us: wall_started_at
                .elapsed()
                .as_micros()
                .min(u128::from(u64::MAX)) as u64,
            thread_cpu_time_us: current_thread_cpu_time_us()
                .zip(cpu_started)
                .map(|(end, start)| end.saturating_sub(start)),
        },
    })
}

/// Caller-owned guards can impose capability restrictions; the host does not
/// inherit any maintained solver's forecast, deadline or recovery policy.
pub fn run_agent_guarded(
    ctx: &RunContext,
    agent: &mut dyn Agent,
    limits: BoundedRunLimitsV1,
    guard: &mut dyn BoundedRunGuard,
) -> Result<BoundedAgentRunArtifactsV1, SimulationError> {
    let descriptor = agent.descriptor();
    descriptor
        .validate()
        .map_err(SimulationError::InvalidContext)?;
    let reset_failure = agent.reset(ctx.into()).err();
    let mut guard = ResetGuard {
        reset_failure,
        inner: guard,
    };
    let mut controller_updates = Vec::new();
    let wall_started_at = Instant::now();
    let thread_cpu_started_at = current_thread_cpu_time_us();
    let episode = run_simulation_bounded(
        ctx,
        &descriptor.implementation,
        limits,
        |ctx, observation| {
            let started_at = Instant::now();
            let frame = agent.update(ctx.into(), observation)?;
            frame.validate_recordable()?;
            // The core validates finite commands before admitting a transition.
            // Record only admitted finite decisions, including a truthful prefix.
            if !frame.command.throttle_frac.is_finite()
                || !frame.command.target_attitude_rad.is_finite()
                || frame.command.target_attitude_rad.abs() > std::f64::consts::PI
            {
                return Err(
                    "agent command must be finite with canonical attitude in [-pi, pi]".to_owned(),
                );
            }
            let frame = frame.clamped();
            controller_updates.push(AgentUpdateRecord {
                sim_time_s: observation.sim_time_s,
                physics_step: observation.physics_step,
                controller_update_index: controller_updates.len() as u64,
                compute_time_us: Some(
                    started_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
                ),
                frame: frame.clone(),
            });
            Ok(frame.command)
        },
        &mut guard,
    )?;
    Ok(BoundedAgentRunArtifactsV1 {
        descriptor,
        episode,
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

struct ResetGuard<'a> {
    reset_failure: Option<String>,
    inner: &'a mut dyn BoundedRunGuard,
}

impl BoundedRunGuard for ResetGuard<'_> {
    fn initial(
        &mut self,
        ctx: &RunContext,
        state: &SimulationState,
    ) -> Result<(), BoundedGuardFailureV1> {
        if let Some(reason) = self.reset_failure.take() {
            return Err(BoundedGuardFailureV1::new(
                BoundedGuardFailureDispositionV1::ExecutionInvalid,
                format!("agent reset failed: {reason}"),
            ));
        }
        self.inner.initial(ctx, state)
    }

    fn before_transition(
        &mut self,
        ctx: &RunContext,
        state: &SimulationState,
        command: Command,
    ) -> Result<(), BoundedGuardFailureV1> {
        self.inner.before_transition(ctx, state, command)
    }

    fn after_transition(
        &mut self,
        ctx: &RunContext,
        state: &SimulationState,
        contact: Option<&IncomingContactV1>,
    ) -> Result<(), BoundedGuardFailureV1> {
        self.inner.after_transition(ctx, state, contact)
    }
}
