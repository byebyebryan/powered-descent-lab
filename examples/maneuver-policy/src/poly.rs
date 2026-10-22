//! Explicit timed Poly Landing composition over the ordinary native Episode.
//! Historical prefixes are regression drivers; they are not planner decisions.
use std::{path::Path, time::Instant};

use pd_control::maneuver::*;
use pd_core::{BoundedRunLimitsV1, Command, FlightProgramUpdateV1, RunContext, ScenarioSpec};
use pd_eval::{EpisodeEvaluationV1, EvidenceSnapshotV1, assess_execution};
use pd_run::{
    AgentDescriptorV1, BoundedAgentRunArtifactsV1, CaptureModeV1, Episode, EpisodeResultV1,
};
use serde::{Deserialize, Serialize};

pub mod recovery;
#[cfg(test)]
mod tests;
pub mod verify;

pub const IMPLEMENTATION: &str = "timed-poly-landing-composition-v2";
pub const CHECKED_IMPLEMENTATION: &str = "timed-poly-landing-composition-v3";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedProgramV2 {
    pub schema_version: u32,
    pub backend: String,
    pub activation_physics_step: u64,
    pub absolute_deadline_physics_step: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_coverage_end_physics_step: Option<u64>,
    pub request: ManeuverRequestV2,
    /// Exact original 60 Hz prefix, or empty for a synthetic local start.
    pub regression_prefix: Vec<FlightProgramUpdateV1>,
    /// One explicitly declared planner command after admission rejection. No retry.
    pub rejection_coast: Command,
}
impl TimedProgramV2 {
    pub fn validate(&self) -> Result<(), String> {
        self.request.validate()?;
        match self.schema_version {
            2 if self.backend == POLY_LANDING_BACKEND
                && self.command_coverage_end_physics_step.is_none() => {}
            3 if self.backend == CHECKED_POLY_LANDING_BACKEND => {
                let ManeuverGoalV2::LandOnMissionTargetAt {
                    reference_end_physics_step,
                    ..
                } = self.request.goal
                else {
                    return Err("finite timed v3 requires a landing endpoint".into());
                };
                if reference_end_physics_step.checked_add(4)
                    != self.command_coverage_end_physics_step
                    || reference_end_physics_step <= self.activation_physics_step
                    || self.coverage_end() > self.absolute_deadline_physics_step
                    || self.coverage_end() <= self.activation_physics_step
                    || !reference_end_physics_step.is_multiple_of(2)
                {
                    return Err("finite coverage must equal reference end plus four under the original deadline".into());
                }
            }
            _ => return Err("requires an explicit supported timed protocol/backend pair".into()),
        }
        if !self.activation_physics_step.is_multiple_of(2)
            || self.activation_physics_step >= self.absolute_deadline_physics_step
            || self.absolute_deadline_physics_step > crate::MAX_PHYSICS_STEPS
            || self.regression_prefix.len() as u64 != self.activation_physics_step / 2
        {
            return Err("invalid original clock or incomplete regression prefix".into());
        }
        let valid = |command: Command| {
            command.throttle_frac.is_finite()
                && (0.0..=1.0).contains(&command.throttle_frac)
                && command.target_attitude_rad.is_finite()
        };
        if !valid(self.rejection_coast) || self.rejection_coast.throttle_frac != 0.0 {
            return Err("rejection policy must declare a finite engine-off coast".into());
        }
        for (index, update) in self.regression_prefix.iter().enumerate() {
            if update.physics_step != index as u64 * 2
                || !valid(update.command)
                || update.phase.is_empty()
            {
                return Err(
                    "regression prefix has gaps, duplicate clocks or invalid commands".into(),
                );
            }
        }
        Ok(())
    }

    pub fn coverage_end(&self) -> u64 {
        self.command_coverage_end_physics_step
            .unwrap_or(self.absolute_deadline_physics_step)
    }

    fn implementation(&self) -> &'static str {
        if self.schema_version == 3 {
            CHECKED_IMPLEMENTATION
        } else {
            IMPLEMENTATION
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedCompositionConfigV2 {
    pub schema_version: u32,
    pub program: TimedProgramV2,
    pub controller: ManeuverDescriptorV2,
    pub driver: String,
}

pub fn controller(
    scenario: &ScenarioSpec,
    program: &TimedProgramV2,
) -> Result<PolynomialLandingController, String> {
    program.validate()?;
    let create = if program.schema_version == 3 {
        PolynomialLandingController::new_checked
    } else {
        PolynomialLandingController::new
    };
    create(
        scenario,
        program.activation_physics_step,
        program.absolute_deadline_physics_step,
    )
    .map_err(|e| format!("{e:?}"))
}

pub fn descriptor(
    program: &TimedProgramV2,
    controller: &dyn ManeuverControllerV2,
) -> AgentDescriptorV1 {
    AgentDescriptorV1 {
        schema_version: 1,
        implementation: program.implementation().into(),
        checkpoint_sha256: None,
        configuration: serde_json::to_value(TimedCompositionConfigV2 {
            schema_version: program.schema_version,
            program: program.clone(),
            controller: controller.descriptor(),
            driver: if program.regression_prefix.is_empty() {
                "synthetic-local-immediate"
            } else {
                "historical-prefix-exact-60hz-regression"
            }
            .into(),
        })
        .expect("finite validated timed program"),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryV2 {
    pub physics_step: u64,
    pub request: Option<ManeuverRequestV2>,
    pub admission: Option<AdmissionV2>,
    pub owner: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedTraceV2 {
    pub schema_version: u32,
    pub descriptor: AgentDescriptorV1,
    pub boundaries: Vec<BoundaryV2>,
    pub feedback: ManeuverFeedbackV2,
    pub physics_steps: u64,
    pub command_updates: u64,
    pub planner_requests: u64,
}

pub struct TimedRunV2 {
    pub descriptor: AgentDescriptorV1,
    pub episode: EpisodeResultV1,
    pub trace: TimedTraceV2,
    pub performance: pd_run::RunPerformanceStats,
}

pub fn run(scenario: &ScenarioSpec, program: &TimedProgramV2) -> Result<TimedRunV2, String> {
    run_with_recorded_requests(scenario, program, None)
}

/// Controller reconstruction consumes recorded requests; planner reconstruction
/// independently derives the sole request from the frozen program.
pub(super) fn run_with_recorded_requests(
    scenario: &ScenarioSpec,
    program: &TimedProgramV2,
    recorded: Option<&[BoundaryV2]>,
) -> Result<TimedRunV2, String> {
    let mut controller = controller(scenario, program)?;
    let descriptor = descriptor(program, &controller);
    let limits = BoundedRunLimitsV1 {
        hard_end_physics_step: program.absolute_deadline_physics_step,
        command_coverage_end_physics_step: program.coverage_end(),
    };
    let mut episode = Episode::reset(
        scenario,
        program.implementation(),
        limits,
        CaptureModeV1::Captured,
    )
    .map_err(|e| e.to_string())?;
    let started = Instant::now();
    let mut boundaries = Vec::new();
    let mut planner_requests = 0;
    let mut active = false;
    while episode.stop().is_none() {
        let obs = episode.observation();
        let tick = obs.physics_step;
        let mut boundary = BoundaryV2 {
            physics_step: tick,
            request: None,
            admission: None,
            owner: "regression_prefix".into(),
            error: None,
        };
        let request = if let Some(recorded) = recorded {
            match recorded.get(boundaries.len()) {
                Some(saved) if saved.physics_step == tick => saved.request.clone(),
                _ => None,
            }
        } else if tick == program.activation_physics_step {
            Some(program.request.clone())
        } else {
            None
        };
        let attempt: Result<pd_run::AgentFrame, String> = (|| {
            if let Some(request) = request {
                planner_requests += 1;
                boundary.request = Some(request.clone());
                let admission = controller.submit(&request, &obs)?;
                active |= matches!(
                    admission,
                    AdmissionV2::Accepted | AdmissionV2::AlreadyActive
                );
                boundary.admission = Some(admission);
            }
            if active {
                boundary.owner = "poly_landing".into();
                return match controller.frame(&obs)? {
                    ManeuverFrameV2::Command(frame) => Ok(frame),
                    ManeuverFrameV2::ReferenceExhausted => {
                        Err("timed landing reference exhausted without native contact".into())
                    }
                };
            }
            if tick < program.activation_physics_step {
                let prefix = &program.regression_prefix[(tick / 2) as usize];
                let mut frame = pd_run::AgentFrame::command_only(prefix.command);
                frame.phase = Some(prefix.phase.clone());
                Ok(frame)
            } else {
                boundary.owner = "planner_rejection_coast".into();
                Ok(pd_run::AgentFrame::command_only(program.rejection_coast))
            }
        })();
        match attempt {
            Ok(frame) => {
                episode.step_frame(frame, None).map_err(|e| e.to_string())?;
            }
            Err(reason) => {
                episode
                    .decision_failed(reason.clone())
                    .map_err(|e| e.to_string())?;
                boundary.error = Some(reason);
            }
        }
        boundaries.push(boundary);
    }
    let episode = episode.finish().map_err(|e| e.to_string())?;
    controller.native_end((&episode.summary).into())?;
    let trace = TimedTraceV2 {
        schema_version: program.schema_version,
        descriptor: descriptor.clone(),
        boundaries,
        feedback: controller.feedback(),
        physics_steps: episode.summary.final_state.physics_step,
        command_updates: episode.summary.manifest.controller_updates,
        planner_requests,
    };
    Ok(TimedRunV2 {
        descriptor,
        episode,
        trace,
        performance: pd_run::RunPerformanceStats {
            wall_time_us: started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
            thread_cpu_time_us: None,
        },
    })
}

pub fn evaluate_files(
    scenario_path: &Path,
    program_path: &Path,
) -> Result<(EpisodeEvaluationV1, TimedTraceV2), String> {
    evaluate_files_with_inputs(scenario_path, program_path, &[])
}

pub fn evaluate_files_with_inputs(
    scenario_path: &Path,
    program_path: &Path,
    additional_inputs: &[std::path::PathBuf],
) -> Result<(EpisodeEvaluationV1, TimedTraceV2), String> {
    let scenario_bytes = crate::verify::read_regular(scenario_path, 16 * 1024 * 1024)?;
    let program_bytes = crate::verify::read_regular(program_path, 16 * 1024 * 1024)?;
    let scenario_path = std::fs::canonicalize(scenario_path).map_err(|e| e.to_string())?;
    let program_path = std::fs::canonicalize(program_path).map_err(|e| e.to_string())?;
    let mut inputs = vec![scenario_path.clone(), program_path.clone()];
    for path in additional_inputs {
        crate::verify::read_regular(path, 64 * 1024 * 1024)?;
        inputs.push(std::fs::canonicalize(path).map_err(|e| e.to_string())?);
    }
    let scope = crate::source_scope(inputs);
    let before = scope.snapshot()?;
    crate::check_compiled_sources(&before)?;
    for (path, bytes) in [
        (&scenario_path, &scenario_bytes),
        (&program_path, &program_bytes),
    ] {
        if !before
            .input_files
            .iter()
            .any(|f| f.path == path.to_string_lossy() && f.sha256 == pd_missions::sha256(bytes))
        {
            return Err("timed input bytes changed during binding".into());
        }
    }
    let scenario = serde_json::from_slice(&scenario_bytes).map_err(|e| e.to_string())?;
    let program = serde_json::from_slice(&program_bytes).map_err(|e| e.to_string())?;
    evaluate_from_snapshot(&scenario, &program, before, &scope)
}

fn evaluate_from_snapshot(
    scenario: &ScenarioSpec,
    program: &TimedProgramV2,
    before: EvidenceSnapshotV1,
    scope: &pd_eval::EvidenceScope,
) -> Result<(EpisodeEvaluationV1, TimedTraceV2), String> {
    let result = run(scenario, program)?;
    let after = scope.snapshot()?;
    let execution = BoundedAgentRunArtifactsV1 {
        descriptor: result.descriptor,
        episode: result.episode.capture.unwrap(),
        controller_updates: result.episode.agent_updates.unwrap(),
        performance: result.performance,
    };
    let evaluation = assess_execution(scenario, execution, before, after, true)?;
    Ok((evaluation, result.trace))
}
