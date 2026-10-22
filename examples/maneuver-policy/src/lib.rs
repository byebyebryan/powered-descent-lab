//! Deterministic activation programs above an episode-local maneuver controller.
//! Software composition only: no learner, inherited landing rate or campaign.
use std::{
    fmt,
    path::{Path, PathBuf},
    time::Instant,
};

use pd_control::maneuver::*;
use pd_core::{BoundedRunLimitsV1, Observation, RunContext, ScenarioSpec};
use pd_eval::{
    EpisodeEvaluationV1, EvidenceScope, EvidenceSnapshotV1, FileIdentityV1, assess_execution,
};
use pd_run::{
    AgentDescriptorV1, BoundedAgentRunArtifactsV1, CaptureModeV1, Episode, EpisodeResultV1,
    MissionView, RunPerformanceStats, scenario_horizon_limits,
};
use serde::{Deserialize, Serialize};

pub mod evaluation;
pub mod poly;
pub mod trace;
pub mod verify;
use trace::*;

pub const MAX_PHYSICS_STEPS: u64 = 72_000;
pub const IMPLEMENTATION: &str = "deterministic-maneuver-composition-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActivationV1 {
    Immediate {},
    Never {},
    FixedTick { physics_step: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationProgramV1 {
    pub schema_version: u32,
    pub activation: ActivationV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlannerDecisionV1 {
    Keep {},
    Request { request: ManeuverRequestV1 },
}

impl ActivationProgramV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("program requires schema version 1".into());
        }
        if let ActivationV1::FixedTick { physics_step } = self.activation
            && !physics_step.is_multiple_of(12)
        {
            return Err("activation tick must be on the 10 Hz grid (multiples of 12)".into());
        }
        Ok(())
    }
    pub fn decide(
        &self,
        mission: MissionView<'_>,
        observation: &Observation,
        feedback: &ManeuverFeedbackV1,
    ) -> PlannerDecisionV1 {
        let activate = match self.activation {
            ActivationV1::Immediate {} => observation.physics_step == 0,
            ActivationV1::Never {} => false,
            ActivationV1::FixedTick { physics_step } => physics_step == observation.physics_step,
        };
        if activate && feedback.accepted.is_none() && feedback.native_end.is_none() {
            PlannerDecisionV1::Request {
                request: ManeuverRequestV1 {
                    schema_version: 1,
                    id: "landing-1".into(),
                    goal: ManeuverGoalV1::LandOnMissionTarget {
                        target_pad_id: mission.target_pad.id.clone(),
                    },
                },
            }
        } else {
            PlannerDecisionV1::Keep {}
        }
    }
}

#[derive(Debug)]
pub enum ApproachErrorV1 {
    Input(ManeuverInputErrorV1),
    Execution(String),
}
impl fmt::Display for ApproachErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ApproachErrorV1 {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionConfigV1 {
    pub schema_version: u32,
    pub program: ActivationProgramV1,
    pub controller: ManeuverDescriptorV1,
    pub planner_hz: u32,
    pub first_decision_physics_step: u64,
}

pub fn descriptor(
    program: &ActivationProgramV1,
    controller: &dyn ManeuverController,
) -> AgentDescriptorV1 {
    AgentDescriptorV1 {
        schema_version: 1,
        implementation: IMPLEMENTATION.into(),
        checkpoint_sha256: None,
        configuration: serde_json::to_value(CompositionConfigV1 {
            schema_version: 1,
            program: program.clone(),
            controller: controller.descriptor(),
            planner_hz: 10,
            first_decision_physics_step: 0,
        })
        .expect("finite frozen config"),
    }
}

pub struct CompositionRunV1 {
    pub descriptor: AgentDescriptorV1,
    pub episode: EpisodeResultV1,
    pub trace: Option<ManeuverTraceV1>,
    pub counters: CountersV1,
    pub feedback: ManeuverFeedbackV1,
    pub performance: RunPerformanceStats,
}

pub fn run(
    scenario: &ScenarioSpec,
    program: &ActivationProgramV1,
    limits: BoundedRunLimitsV1,
    mode: CaptureModeV1,
) -> Result<CompositionRunV1, ApproachErrorV1> {
    program
        .validate()
        .map_err(|reason| ApproachErrorV1::Input(ManeuverInputErrorV1::InvalidInput(reason)))?;
    let mut controller =
        TerminalLandingController::new(scenario).map_err(ApproachErrorV1::Input)?;
    let ctx = RunContext::from_scenario(scenario)
        .map_err(|reason| ApproachErrorV1::Input(ManeuverInputErrorV1::InvalidInput(reason)))?;
    let horizon = scenario_horizon_limits(&ctx).map_err(|error| {
        ApproachErrorV1::Input(ManeuverInputErrorV1::InvalidInput(error.to_string()))
    })?;
    if horizon.hard_end_physics_step > MAX_PHYSICS_STEPS
        || limits.hard_end_physics_step > horizon.hard_end_physics_step
    {
        return Err(ApproachErrorV1::Input(
            ManeuverInputErrorV1::UnsupportedInput(
                "single-episode software bound is at most 600 seconds and original horizon".into(),
            ),
        ));
    }
    run_with_controller(scenario, program, limits, mode, &mut controller)
        .map_err(ApproachErrorV1::Execution)
}

fn run_with_controller(
    scenario: &ScenarioSpec,
    program: &ActivationProgramV1,
    limits: BoundedRunLimitsV1,
    mode: CaptureModeV1,
    controller: &mut dyn ManeuverController,
) -> Result<CompositionRunV1, String> {
    program.validate()?;
    let ctx = RunContext::from_scenario(scenario)?;
    let horizon = scenario_horizon_limits(&ctx).map_err(|e| e.to_string())?;
    if horizon.hard_end_physics_step > MAX_PHYSICS_STEPS
        || limits.hard_end_physics_step > horizon.hard_end_physics_step
    {
        return Err(
            "single-episode software bound is at most 600 seconds and original horizon".into(),
        );
    }
    let descriptor = descriptor(program, controller);
    let mut episode =
        Episode::reset(scenario, IMPLEMENTATION, limits, mode).map_err(|e| e.to_string())?;
    let mut boundaries = (mode == CaptureModeV1::Captured).then(Vec::new);
    let mut planner_calls = 0;
    let started = Instant::now();
    while episode.stop().is_none() {
        let observation = episode.observation();
        let feedback = controller.feedback();
        let mut boundary = BoundaryV1 {
            physics_step: observation.physics_step,
            planner: None,
            owner: feedback.owner.clone(),
            error: None,
        };
        let update_started = Instant::now();
        let attempt = (|| {
            if feedback.accepted.is_none() && observation.physics_step.is_multiple_of(12) {
                planner_calls += 1;
                let decision = program.decide(episode.mission(), &observation, &feedback);
                boundary.planner = Some(PlannerCallV1 {
                    feedback: feedback.clone(),
                    decision: decision.clone(),
                    admission: None,
                });
                if let PlannerDecisionV1::Request { request } = decision {
                    let admission = controller.submit(&request, observation.physics_step)?;
                    boundary.planner.as_mut().unwrap().admission = Some(admission);
                }
            }
            controller.frame(&observation)
        })();
        match attempt {
            Ok(frame) => {
                episode
                    .step_frame(
                        frame,
                        Some(
                            update_started
                                .elapsed()
                                .as_micros()
                                .min(u128::from(u64::MAX)) as u64,
                        ),
                    )
                    .map_err(|e| e.to_string())?;
            }
            Err(reason) => {
                episode
                    .decision_failed(reason.clone())
                    .map_err(|e| e.to_string())?;
                boundary.error = Some(reason);
            }
        }
        boundary.owner = controller.feedback().owner;
        if let Some(boundaries) = &mut boundaries {
            boundaries.push(boundary);
        }
    }
    let episode = episode.finish().map_err(|e| e.to_string())?;
    controller.native_end((&episode.summary).into())?;
    let feedback = controller.feedback();
    let counters = CountersV1 {
        physics_steps: episode.summary.final_state.physics_step,
        command_updates: episode.summary.manifest.controller_updates,
        backend_updates: feedback.backend_updates,
        planner_calls,
    };
    let trace = boundaries.map(|boundaries| ManeuverTraceV1 {
        schema_version: 1,
        descriptor: descriptor.clone(),
        boundaries,
        final_feedback: feedback.clone(),
        counters: counters.clone(),
    });
    Ok(CompositionRunV1 {
        descriptor,
        episode,
        trace,
        counters,
        feedback,
        performance: RunPerformanceStats {
            wall_time_us: started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
            thread_cpu_time_us: None,
        },
    })
}

/// Participating source only; archives, report outputs and unrelated docs stay
/// outside execution identity. The executable SHA is supplied by EvidenceScope.
pub fn source_scope(inputs: Vec<PathBuf>) -> EvidenceScope {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut sources: Vec<_> = [
        "Cargo.toml",
        "Cargo.lock",
        "examples/maneuver-policy/build.rs",
    ]
    .iter()
    .map(|p| root.join(p))
    .collect();
    for package in [
        "pd-core",
        "pd-control",
        "pd-run",
        "pd-missions",
        "pd-eval",
        "pd-report",
        "examples/maneuver-policy",
    ] {
        sources.push(root.join(package).join("Cargo.toml"));
        sources.push(root.join(package).join("src"));
    }
    EvidenceScope { sources, inputs }
}

pub fn check_compiled_sources(snapshot: &EvidenceSnapshotV1) -> Result<(), String> {
    let compiled: Vec<FileIdentityV1> = serde_json::from_str(include_str!(concat!(
        env!("OUT_DIR"),
        "/compiled-sources.json"
    )))
    .map_err(|e| e.to_string())?;
    if snapshot.schema_version != 1 || snapshot.source_files != compiled {
        return Err(
            "sources differ from this compiled build; rebuild before execution/reconstruction"
                .into(),
        );
    }
    Ok(())
}

pub fn evaluate(
    scenario: &ScenarioSpec,
    program: &ActivationProgramV1,
    scope: &EvidenceScope,
) -> Result<(EpisodeEvaluationV1, ManeuverTraceV1), String> {
    let before = scope.snapshot()?;
    check_compiled_sources(&before)?;
    evaluate_from_snapshot(scenario, program, scope, before)
}

pub fn evaluate_files(
    scenario_path: &Path,
    program_path: &Path,
) -> Result<(EpisodeEvaluationV1, ManeuverTraceV1), String> {
    evaluate_files_with_inputs(scenario_path, program_path, &[])
}

/// Additive binding for immutable outer schedules. The ordinary two-file API
/// keeps its existing behavior and evidence schemas.
pub fn evaluate_files_with_inputs(
    scenario_path: &Path,
    program_path: &Path,
    additional_inputs: &[PathBuf],
) -> Result<(EpisodeEvaluationV1, ManeuverTraceV1), String> {
    verify::read_regular(scenario_path, 16 * 1024 * 1024)?;
    verify::read_regular(program_path, 64 * 1024)?;
    let scenario_path = std::fs::canonicalize(scenario_path).map_err(|e| e.to_string())?;
    let program_path = std::fs::canonicalize(program_path).map_err(|e| e.to_string())?;
    let mut inputs = vec![scenario_path.clone(), program_path.clone()];
    for path in additional_inputs {
        verify::read_regular(path, 32 * 1024 * 1024)?;
        inputs.push(std::fs::canonicalize(path).map_err(|e| e.to_string())?);
    }
    let scope = source_scope(inputs);
    let before = scope.snapshot()?;
    check_compiled_sources(&before)?;
    let scenario_bytes = verify::read_regular(&scenario_path, 16 * 1024 * 1024)?;
    let program_bytes = verify::read_regular(&program_path, 64 * 1024)?;
    for (path, bytes) in [
        (&scenario_path, &scenario_bytes),
        (&program_path, &program_bytes),
    ] {
        if !before.input_files.iter().any(|file| {
            file.path == path.to_string_lossy() && file.sha256 == pd_missions::sha256(bytes)
        }) {
            return Err("input bytes changed between binding and decoding".into());
        }
    }
    let scenario = serde_json::from_slice(&scenario_bytes).map_err(|e| e.to_string())?;
    let program = serde_json::from_slice(&program_bytes).map_err(|e| e.to_string())?;
    evaluate_from_snapshot(&scenario, &program, &scope, before)
}

fn evaluate_from_snapshot(
    scenario: &ScenarioSpec,
    program: &ActivationProgramV1,
    scope: &EvidenceScope,
    before: EvidenceSnapshotV1,
) -> Result<(EpisodeEvaluationV1, ManeuverTraceV1), String> {
    let ctx = RunContext::from_scenario(scenario)?;
    let result = run(
        scenario,
        program,
        scenario_horizon_limits(&ctx).map_err(|e| e.to_string())?,
        CaptureModeV1::Captured,
    )
    .map_err(|e| e.to_string())?;
    let after = scope.snapshot()?;
    let execution = BoundedAgentRunArtifactsV1 {
        descriptor: result.descriptor,
        episode: result.episode.capture.unwrap(),
        controller_updates: result.episode.agent_updates.unwrap(),
        performance: result.performance,
    };
    let evaluation = assess_execution(scenario, execution, before, after, true)?;
    Ok((evaluation, result.trace.unwrap()))
}

#[cfg(test)]
mod tests;
