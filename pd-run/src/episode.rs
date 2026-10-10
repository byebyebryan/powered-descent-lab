//! Incremental ordinary episodes. Core owns every transition and artifact
//! projection; runtime owns cadence and optional recording, never rewards.

use pd_core::sim::{
    bounded_stop_at_state, build_manifest, first_nonfinite_artifact_projection_failure,
    first_nonfinite_transition_failure, invalid_bounded_command, maybe_push_sample,
    nonfinite_command_failure, validate_bounded_limits,
};
use pd_core::{
    ActionLogEntry, BOUNDED_RUN_SCHEMA_VERSION, BoundedGuardFailureDispositionV1,
    BoundedRunArtifactsV1, BoundedRunFailureStageV1, BoundedRunFailureV1, BoundedRunLimitsV1,
    BoundedRunStopCauseV1, Command, EndReason, EventKind, EventRecord, IncomingContactV1,
    Observation, RunArtifacts, RunContext, RunManifest, SampleRecord, ScenarioSpec,
    SimulationError, SimulationState, SimulationStateSnapshotV1,
};
use serde::{Deserialize, Serialize};

use crate::{AgentFrame, AgentUpdateRecord, MissionView};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureModeV1 {
    Summary,
    Captured,
}

/// Derive the declared scenario horizon, never a solver reserve or extended
/// mission budget. Overflow is unsupported input rather than a wrapped schedule.
pub fn scenario_horizon_limits(ctx: &RunContext) -> Result<BoundedRunLimitsV1, SimulationError> {
    ctx.sim
        .validate()
        .map_err(SimulationError::InvalidContext)?;
    let product = (ctx.sim.max_time_s * f64::from(ctx.sim.physics_hz)).ceil();
    if !product.is_finite() || product >= u64::MAX as f64 {
        return Err(SimulationError::InvalidContext(
            "scenario horizon is not representable in physics steps".to_owned(),
        ));
    }
    let end = product as u64;
    let interval = ctx.sim.control_interval_steps();
    let coverage = end
        .div_ceil(interval)
        .checked_mul(interval)
        .ok_or_else(|| {
            SimulationError::InvalidContext("control coverage overflows physics clock".to_owned())
        })?;
    Ok(BoundedRunLimitsV1 {
        command_coverage_end_physics_step: coverage,
        hard_end_physics_step: end,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeSummaryV1 {
    pub schema_version: u32,
    pub limits: BoundedRunLimitsV1,
    pub stop: BoundedRunStopCauseV1,
    pub final_state: SimulationStateSnapshotV1,
    pub incoming_contact: Option<IncomingContactV1>,
    pub failure: Option<BoundedRunFailureV1>,
    pub manifest: RunManifest,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EpisodeResultV1 {
    pub summary: EpisodeSummaryV1,
    /// Absent in summary mode: no action/event/sample/update vectors are retained.
    pub capture: Option<BoundedRunArtifactsV1>,
    pub agent_updates: Option<Vec<AgentUpdateRecord>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EpisodeStepV1 {
    pub observation: Observation,
    pub physics_steps_advanced: u64,
    pub stop: Option<BoundedRunStopCauseV1>,
    pub physical_outcome: pd_core::PhysicalOutcome,
    pub mission_outcome: pd_core::MissionOutcome,
}

#[derive(Default)]
struct Recording {
    actions: Vec<ActionLogEntry>,
    events: Vec<EventRecord>,
    samples: Vec<SampleRecord>,
    updates: Vec<AgentUpdateRecord>,
}

/// State can only be initialized from the mission, or advanced from its current
/// position. There is no arbitrary snapshot setter, hidden autoreset or reward.
pub struct Episode {
    ctx: RunContext,
    agent_id: String,
    state: SimulationState,
    limits: BoundedRunLimitsV1,
    stop: Option<BoundedRunStopCauseV1>,
    failure: Option<BoundedRunFailureV1>,
    incoming_contact: Option<IncomingContactV1>,
    updates: u64,
    recording: Option<Recording>,
}

impl Episode {
    pub fn reset(
        scenario: &ScenarioSpec,
        agent_id: &str,
        limits: BoundedRunLimitsV1,
        mode: CaptureModeV1,
    ) -> Result<Self, SimulationError> {
        let ctx = RunContext::from_scenario(scenario).map_err(SimulationError::InvalidContext)?;
        Self::from_context(&ctx, agent_id, limits, mode)
    }

    pub(crate) fn from_context(
        ctx: &RunContext,
        agent_id: &str,
        limits: BoundedRunLimitsV1,
        mode: CaptureModeV1,
    ) -> Result<Self, SimulationError> {
        let state = SimulationState::new(ctx)?;
        validate_bounded_limits(ctx, limits)?;
        if let Some(failure) = first_nonfinite_artifact_projection_failure(
            ctx,
            &state,
            BoundedRunFailureStageV1::InitialGuard,
            0,
        ) {
            return Err(SimulationError::InvalidContext(format!(
                "episode has no finite initial state: {}",
                failure.reason
            )));
        }
        let mut episode = Self {
            ctx: ctx.clone(),
            agent_id: agent_id.to_owned(),
            stop: bounded_stop_at_state(&state, limits),
            state,
            limits,
            failure: None,
            incoming_contact: None,
            updates: 0,
            recording: match mode {
                CaptureModeV1::Summary => None,
                CaptureModeV1::Captured => Some(Recording::default()),
            },
        };
        episode.record_sample();
        Ok(episode)
    }

    pub fn mission(&self) -> MissionView<'_> {
        (&self.ctx).into()
    }
    pub fn observation(&self) -> Observation {
        self.state.build_observation(&self.ctx)
    }
    pub fn stop(&self) -> Option<BoundedRunStopCauseV1> {
        self.stop
    }
    pub fn retained_record_counts(&self) -> (usize, usize, usize, usize) {
        self.recording.as_ref().map_or((0, 0, 0, 0), |r| {
            (
                r.actions.len(),
                r.events.len(),
                r.samples.len(),
                r.updates.len(),
            )
        })
    }

    /// One action advances up to the next command boundary. Contact/progress,
    /// numeric failures and finite endpoints can terminate between boundaries.
    pub fn step(&mut self, command: Command) -> Result<EpisodeStepV1, SimulationError> {
        self.step_frame(AgentFrame::command_only(command), None)
    }

    pub fn step_frame(
        &mut self,
        frame: AgentFrame,
        compute_time_us: Option<u64>,
    ) -> Result<EpisodeStepV1, SimulationError> {
        if self.stop.is_some() {
            return Err(SimulationError::InvalidContext(
                "cannot step a completed episode; reset explicitly".to_owned(),
            ));
        }
        let start = self.state.physics_step;
        let interval = self.ctx.sim.control_interval_steps();
        debug_assert!(start.is_multiple_of(interval));
        if let Some(failure) = nonfinite_command_failure(
            frame.command,
            BoundedRunFailureStageV1::CommandSelection,
            start,
        ) {
            self.reject(failure);
            return Ok(self.step_result(start));
        }
        if let Err(reason) = frame.validate_recordable() {
            return self.decision_failed(reason);
        }
        let frame = frame.clamped();
        if let Some(reason) = invalid_bounded_command(frame.command) {
            self.reject(BoundedRunFailureV1::new(
                BoundedRunFailureStageV1::CommandSelection,
                BoundedGuardFailureDispositionV1::ExecutionInvalid,
                start,
                reason,
            ));
            return Ok(self.step_result(start));
        }
        self.state.set_command(frame.command);
        if let Some(recording) = &mut self.recording {
            recording.actions.push(ActionLogEntry {
                sim_time_s: self.state.sim_time_s,
                physics_step: start,
                controller_update_index: self.updates,
                command: self.state.held_command,
            });
            recording.events.push(EventRecord {
                sim_time_s: self.state.sim_time_s,
                physics_step: start,
                kind: EventKind::ControllerUpdated,
                message: "controller_updated".to_owned(),
            });
            recording.updates.push(AgentUpdateRecord {
                sim_time_s: self.state.sim_time_s,
                physics_step: start,
                controller_update_index: self.updates,
                compute_time_us,
                frame,
            });
        }
        self.updates += 1;
        for _ in 0..interval {
            let last_valid = self.state.clone();
            let transition = self.state.step_with_contact_report(&self.ctx);
            if let Some(failure) =
                first_nonfinite_transition_failure(&self.ctx, &self.state, &transition)
            {
                self.state = last_valid;
                self.reject(failure);
                break;
            }
            if let Some(recording) = &mut self.recording {
                recording.events.extend(transition.events);
            }
            if let Some(contact) = transition.incoming_contact {
                self.incoming_contact = Some(contact);
            }
            self.record_sample();
            if self.state.is_terminal() {
                self.stop = Some(if self.state.end_reason == EndReason::MaxTimeReached {
                    BoundedRunStopCauseV1::ScenarioHorizonReached
                } else {
                    BoundedRunStopCauseV1::MissionTerminal
                });
            } else {
                self.stop = bounded_stop_at_state(&self.state, self.limits);
            }
            if self.stop.is_some() {
                break;
            }
        }
        Ok(self.step_result(start))
    }

    fn step_result(&self, start: u64) -> EpisodeStepV1 {
        EpisodeStepV1 {
            observation: self.observation(),
            physics_steps_advanced: self.state.physics_step - start,
            stop: self.stop,
            physical_outcome: self.state.physical_outcome.clone(),
            mission_outcome: self.state.mission_outcome.clone(),
        }
    }

    pub fn decision_failed(
        &mut self,
        reason: impl Into<String>,
    ) -> Result<EpisodeStepV1, SimulationError> {
        if self.stop.is_some() {
            return Err(SimulationError::InvalidContext(
                "cannot select a decision after completion".to_owned(),
            ));
        }
        let step = self.state.physics_step;
        self.reject(BoundedRunFailureV1::new(
            BoundedRunFailureStageV1::CommandSelection,
            BoundedGuardFailureDispositionV1::ExecutionInvalid,
            step,
            reason,
        ));
        Ok(self.step_result(step))
    }

    pub(crate) fn reset_failed(&mut self, reason: impl Into<String>) {
        self.reject(BoundedRunFailureV1::new(
            BoundedRunFailureStageV1::InitialGuard,
            BoundedGuardFailureDispositionV1::ExecutionInvalid,
            0,
            reason,
        ));
    }

    fn reject(&mut self, failure: BoundedRunFailureV1) {
        self.stop = Some(match failure.disposition {
            BoundedGuardFailureDispositionV1::ExecutionInvalid => {
                BoundedRunStopCauseV1::ExecutionInvalid
            }
            BoundedGuardFailureDispositionV1::SafetyRejected => {
                BoundedRunStopCauseV1::SafetyRejected
            }
        });
        self.failure = Some(failure);
    }

    fn record_sample(&mut self) {
        if let Some(recording) = &mut self.recording {
            maybe_push_sample(
                &mut recording.samples,
                &self.state,
                &self.ctx,
                self.ctx.sim.sample_interval_steps(),
            );
        }
    }

    /// A summary is authoritative physical/mission state, but is not saved-command
    /// replay evidence. Captures remain explicit and cannot be invented later.
    pub fn finish(self) -> Result<EpisodeResultV1, SimulationError> {
        let stop = self.stop.ok_or_else(|| {
            SimulationError::InvalidContext("episode is still in progress".to_owned())
        })?;
        let summary = EpisodeSummaryV1 {
            schema_version: BOUNDED_RUN_SCHEMA_VERSION,
            limits: self.limits,
            stop,
            final_state: SimulationStateSnapshotV1::from_state(&self.state),
            incoming_contact: self.incoming_contact,
            failure: self.failure,
            manifest: build_manifest(&self.ctx, &self.agent_id, &self.state, self.updates),
        };
        let (capture, agent_updates) = match self.recording {
            None => (None, None),
            Some(recording) => (
                Some(BoundedRunArtifactsV1 {
                    schema_version: summary.schema_version,
                    limits: summary.limits,
                    stop: summary.stop,
                    coverage_reached: summary.final_state.physics_step
                        >= summary.limits.command_coverage_end_physics_step,
                    hard_end_reached: summary.final_state.physics_step
                        >= summary.limits.hard_end_physics_step,
                    final_state: summary.final_state.clone(),
                    incoming_contact: summary.incoming_contact.clone(),
                    failure: summary.failure.clone(),
                    run: RunArtifacts {
                        manifest: summary.manifest.clone(),
                        actions: recording.actions,
                        events: recording.events,
                        samples: recording.samples,
                    },
                }),
                Some(recording.updates),
            ),
        };
        Ok(EpisodeResultV1 {
            summary,
            capture,
            agent_updates,
        })
    }
}
