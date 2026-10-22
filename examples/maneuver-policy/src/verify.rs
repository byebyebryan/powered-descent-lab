//! Companion evidence; neutral evaluation/seal schemas remain unchanged.
use std::{
    fs::{self, File},
    io::Read,
    path::{Component, Path},
};

use pd_control::maneuver::*;
use pd_core::RunContext;
use pd_eval::{EpisodeEvaluationV1, FileIdentityV1, load_verified_evaluation, save_evaluation};
use pd_missions::{sha256, write_json_create_only};
use pd_run::{AgentDescriptorV1, CaptureModeV1, Episode};
use serde::{Deserialize, Serialize};

use crate::{
    CompositionConfigV1, MAX_PHYSICS_STEPS, PlannerDecisionV1, check_compiled_sources, descriptor,
    source_scope, trace::*,
};

const MAX_TRACE_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "detail",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CheckV1 {
    Passed,
    Failed(String),
    Unavailable(String),
}
impl CheckV1 {
    pub fn passed(&self) -> bool {
        *self == Self::Passed
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofResourcesV1 {
    pub neutral_replay_passes_started: u64,
    pub controller_reconstruction_passes_started: u64,
    pub planner_reconstruction_passes_started: u64,
    /// Exact for neutral passes that returned a verified evaluation. An errored
    /// neutral loader exposes no count; it is never silently counted as zero.
    pub known_neutral_physics_steps: u64,
    pub neutral_passes_without_step_count: u64,
    pub reconstruction_physics_steps: u64,
    pub reconstruction_backend_updates: u64,
    pub reconstruction_planner_calls: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleReceiptV1 {
    pub schema_version: u32,
    pub descriptor: AgentDescriptorV1,
    pub physical_sha256: String,
    pub input_files: Vec<FileIdentityV1>,
    pub evaluation_sha256: String,
    pub seal_sha256: String,
    pub trace_sha256: String,
    pub preparation_resources: ProofResourcesV1,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationReportV1 {
    pub schema_version: u32,
    pub bundle_integrity: CheckV1,
    pub physical_replay: CheckV1,
    pub controller_reconstruction: CheckV1,
    pub planner_reconstruction: CheckV1,
    /// The neutral evaluator's physical/mission/integrity/replay tuple, separate
    /// from companion trace integrity and decision reconstruction.
    pub neutral_verified_target_landing: bool,
    pub resources: ProofResourcesV1,
}
impl VerificationReportV1 {
    pub fn passed(&self) -> bool {
        self.bundle_integrity.passed()
            && self.physical_replay.passed()
            && self.controller_reconstruction.passed()
            && self.planner_reconstruction.passed()
    }
}

pub struct VerifiedBundleV1 {
    pub evaluation: Option<EpisodeEvaluationV1>,
    pub report: VerificationReportV1,
}

pub(crate) fn no_symlinks(path: &Path) -> Result<(), String> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let mut prefix = std::path::PathBuf::new();
    for component in absolute.components() {
        if matches!(component, Component::ParentDir) {
            return Err("parent traversal in bundle path".into());
        }
        prefix.push(component);
        if fs::symlink_metadata(&prefix)
            .map_err(|e| e.to_string())?
            .is_symlink()
        {
            return Err("symlink in bundle path".into());
        }
    }
    Ok(())
}

pub fn read_regular(path: &Path, max_bytes: u64) -> Result<Vec<u8>, String> {
    no_symlinks(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return Err("requires bounded regular input files".into());
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max_bytes {
        return Err("input grew beyond byte limit".into());
    }
    Ok(bytes)
}

fn compatibility(
    evaluation: &EpisodeEvaluationV1,
    trace: &ManeuverTraceV1,
) -> Result<CompositionConfigV1, String> {
    let current = source_scope(Vec::new()).snapshot()?;
    check_compiled_sources(&current)?;
    if evaluation.evidence_before.source_files != current.source_files
        || evaluation.evidence_after.source_files != current.source_files
        || evaluation.evidence_before.executable_sha256 != current.executable_sha256
        || evaluation.evidence_after.executable_sha256 != current.executable_sha256
    {
        return Err(
            "frozen compiled source/executable mismatch; physical replay remains independent"
                .into(),
        );
    }
    let config: CompositionConfigV1 =
        serde_json::from_value(trace.descriptor.configuration.clone())
            .map_err(|e| e.to_string())?;
    config.program.validate()?;
    let controller =
        TerminalLandingController::new(&evaluation.scenario).map_err(|e| format!("{e:?}"))?;
    if config.schema_version != 1
        || trace.schema_version != 1
        || trace.descriptor != evaluation.execution.descriptor
        || trace.descriptor != descriptor(&config.program, &controller)
    {
        return Err("unsupported composition descriptor/configuration or trace version".into());
    }
    Ok(config)
}

#[derive(Clone, Copy)]
enum Reconstruction {
    Controller,
    Planner,
}

fn persisted_frame(frame: &pd_run::AgentFrame) -> Result<pd_run::AgentFrame, String> {
    // Existing untagged TelemetryValue decodes JSON integer metrics as Float.
    // Compare the unchanged neutral wire projection on both sides. Commands
    // are additionally compared directly, before this projection.
    serde_json::from_slice(&serde_json::to_vec(frame).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

pub(crate) fn frame_matches(
    expected: &pd_run::AgentFrame,
    recorded: &pd_run::AgentFrame,
) -> Result<bool, String> {
    Ok(expected.command == recorded.command
        && persisted_frame(expected)? == persisted_frame(recorded)?)
}

/// Both checks start fresh. Observations/ticks come from native replay of the
/// ordinary saved commands. Controller checks ignore the program's choices;
/// planner checks ignore recorded admissions/authority and recompute feedback.
fn reconstruct(
    evaluation: &EpisodeEvaluationV1,
    trace: &ManeuverTraceV1,
    kind: Reconstruction,
    resources: &mut ProofResourcesV1,
) -> Result<(), String> {
    let config = compatibility(evaluation, trace)?;
    let ctx = RunContext::from_scenario(&evaluation.scenario)?;
    let horizon = pd_run::scenario_horizon_limits(&ctx).map_err(|e| e.to_string())?;
    let saved = &evaluation.execution;
    if horizon.hard_end_physics_step > MAX_PHYSICS_STEPS
        || saved.episode.limits.hard_end_physics_step > horizon.hard_end_physics_step
        || trace.boundaries.len() as u64
            > saved.episode.limits.hard_end_physics_step.div_ceil(2) + 1
        || saved.controller_updates.len() != saved.episode.run.actions.len()
    {
        return Err("trace or execution exceeds declared native update/horizon bounds".into());
    }
    match kind {
        Reconstruction::Controller => resources.controller_reconstruction_passes_started += 1,
        Reconstruction::Planner => resources.planner_reconstruction_passes_started += 1,
    }
    let mut controller =
        TerminalLandingController::new(&evaluation.scenario).map_err(|e| format!("{e:?}"))?;
    let mut episode = Episode::reset(
        &evaluation.scenario,
        &saved.descriptor.implementation,
        saved.episode.limits,
        CaptureModeV1::Summary,
    )
    .map_err(|e| e.to_string())?;
    let mut ordinal = 0;
    let mut planner_calls = 0;
    for boundary in &trace.boundaries {
        if episode.stop().is_some() || boundary.physics_step != episode.observation().physics_step {
            return Err("trace boundary differs from live native command boundary".into());
        }
        let observation = episode.observation();
        let before = controller.feedback();
        let due = before.accepted.is_none() && observation.physics_step.is_multiple_of(12);
        if matches!(kind, Reconstruction::Planner) && boundary.planner.is_some() != due {
            return Err("missing/extra planner opportunity or call after commitment".into());
        }
        let mut actual_error = None;
        if let Some(call) = &boundary.planner {
            planner_calls += 1;
            if matches!(kind, Reconstruction::Planner) {
                resources.reconstruction_planner_calls += 1;
                if call.feedback != before
                    || call.decision
                        != config
                            .program
                            .decide(episode.mission(), &observation, &before)
                {
                    return Err("planner decision or input feedback mismatch".into());
                }
            }
            let admission = match &call.decision {
                PlannerDecisionV1::Keep {} => None,
                PlannerDecisionV1::Request { request } => {
                    match controller.submit(request, observation.physics_step) {
                        Ok(admission) => Some(admission),
                        Err(error) => {
                            actual_error = Some(error);
                            None
                        }
                    }
                }
            };
            if matches!(kind, Reconstruction::Controller) && call.admission != admission {
                return Err("controller admission mismatch".into());
            }
        }
        let frame = if actual_error.is_none() {
            match controller.frame(&observation) {
                Ok(frame) => Some(frame),
                Err(error) => {
                    actual_error = Some(error);
                    None
                }
            }
        } else {
            None
        };
        resources.reconstruction_backend_updates +=
            controller.feedback().backend_updates - before.backend_updates;
        if matches!(kind, Reconstruction::Controller)
            && (controller.feedback().owner != boundary.owner || actual_error != boundary.error)
        {
            return Err("command authority or execution error mismatch".into());
        }
        if let Some(reason) = actual_error {
            episode.decision_failed(reason).map_err(|e| e.to_string())?;
        } else if let Some(frame) = frame {
            let admitted = saved
                .controller_updates
                .get(ordinal)
                .filter(|update| update.physics_step == observation.physics_step);
            if let Some(update) = admitted {
                if matches!(kind, Reconstruction::Controller)
                    && !frame_matches(&frame.clamped(), &update.frame)?
                {
                    return Err(format!(
                        "controller frame/command mismatch at {}",
                        observation.physics_step
                    ));
                }
                let step = episode
                    .step_frame(update.frame.clone(), None)
                    .map_err(|e| e.to_string())?;
                resources.reconstruction_physics_steps += step.physics_steps_advanced;
                ordinal += 1;
            } else {
                // A rejected nonfinite frame has no admitted command. Recreate
                // that genuine boundary failure, never fabricate a missing frame.
                let step = episode.step_frame(frame, None).map_err(|e| e.to_string())?;
                resources.reconstruction_physics_steps += step.physics_steps_advanced;
                if step.physics_steps_advanced != 0 || step.stop.is_none() {
                    return Err("missing admitted native command".into());
                }
            }
        }
    }
    if ordinal != saved.controller_updates.len() || episode.stop().is_none() {
        return Err("trace does not cover the actual native prefix/end".into());
    }
    let result = episode.finish().map_err(|e| e.to_string())?;
    if result.summary.final_state != saved.episode.final_state
        || result.summary.stop != saved.episode.stop
        || result.summary.failure != saved.episode.failure
        || result.summary.incoming_contact != saved.episode.incoming_contact
    {
        return Err("reconstructed native end differs".into());
    }
    controller.native_end((&result.summary).into())?;
    if matches!(kind, Reconstruction::Controller) {
        let expected = CountersV1 {
            physics_steps: result.summary.final_state.physics_step,
            command_updates: result.summary.manifest.controller_updates,
            backend_updates: controller.feedback().backend_updates,
            planner_calls,
        };
        if trace.final_feedback != controller.feedback() || trace.counters != expected {
            return Err("final lifecycle/counters mismatch".into());
        }
    }
    Ok(())
}

fn check(result: Result<(), String>) -> CheckV1 {
    match result {
        Ok(()) => CheckV1::Passed,
        Err(reason) => CheckV1::Failed(reason),
    }
}

pub fn reconstruct_controller(
    evaluation: &EpisodeEvaluationV1,
    trace: &ManeuverTraceV1,
    resources: &mut ProofResourcesV1,
) -> CheckV1 {
    check(reconstruct(
        evaluation,
        trace,
        Reconstruction::Controller,
        resources,
    ))
}
pub fn reconstruct_planner(
    evaluation: &EpisodeEvaluationV1,
    trace: &ManeuverTraceV1,
    resources: &mut ProofResourcesV1,
) -> CheckV1 {
    check(reconstruct(
        evaluation,
        trace,
        Reconstruction::Planner,
        resources,
    ))
}

/// This receives an already assessed execution (one neutral replay). Saving
/// validates it once more. Receipt last; failure never repairs a prior attempt.
pub fn save_bundle(
    root: &Path,
    evaluation: &EpisodeEvaluationV1,
    trace: &ManeuverTraceV1,
) -> Result<(), String> {
    save_bundle_with_hook(root, evaluation, trace, &mut || Ok(()))
}

/// Hook is internal and permits real filesystem interruption checks without
/// adding failure switches to the command-line or persisted contracts.
pub(crate) fn save_bundle_with_hook(
    root: &Path,
    evaluation: &EpisodeEvaluationV1,
    trace: &ManeuverTraceV1,
    after_capture: &mut dyn FnMut() -> Result<(), String>,
) -> Result<(), String> {
    no_symlinks(root.parent().ok_or("bundle needs a parent directory")?)?;
    fs::create_dir(root).map_err(|e| format!("reserve new bundle: {e}"))?;
    save_evaluation(&root.join("capture"), evaluation)?;
    after_capture()?;
    write_json_create_only(&root.join("trace.json"), trace)?;
    let mut resources = ProofResourcesV1 {
        neutral_replay_passes_started: 2,
        known_neutral_physics_steps: if evaluation.replay.physical_prefix_verified {
            evaluation.execution.episode.final_state.physics_step * 2
        } else {
            0
        },
        neutral_passes_without_step_count: if evaluation.replay.physical_prefix_verified {
            0
        } else {
            2
        },
        ..ProofResourcesV1::default()
    };
    let _controller = reconstruct_controller(evaluation, trace, &mut resources);
    let _planner = reconstruct_planner(evaluation, trace, &mut resources);
    let receipt = BundleReceiptV1 {
        schema_version: 1,
        descriptor: evaluation.execution.descriptor.clone(),
        physical_sha256: evaluation.physical_sha256.clone(),
        input_files: evaluation.evidence_before.input_files.clone(),
        evaluation_sha256: sha256(&read_regular(
            &root.join("capture/evaluation.json"),
            64 * 1024 * 1024,
        )?),
        seal_sha256: sha256(&read_regular(&root.join("capture/seal.json"), 1024 * 1024)?),
        trace_sha256: sha256(&read_regular(&root.join("trace.json"), MAX_TRACE_BYTES)?),
        preparation_resources: resources,
    };
    write_json_create_only(&root.join("receipt.json"), &receipt)
}

fn binding(
    root: &Path,
    evaluation: &EpisodeEvaluationV1,
    trace: &ManeuverTraceV1,
) -> Result<(), String> {
    let receipt: BundleReceiptV1 =
        serde_json::from_slice(&read_regular(&root.join("receipt.json"), 4 * 1024 * 1024)?)
            .map_err(|e| e.to_string())?;
    if receipt.schema_version != 1
        || receipt.descriptor != evaluation.execution.descriptor
        || receipt.descriptor != trace.descriptor
        || receipt.physical_sha256 != evaluation.physical_sha256
        || receipt.input_files != evaluation.evidence_before.input_files
        || receipt.evaluation_sha256
            != sha256(&read_regular(
                &root.join("capture/evaluation.json"),
                64 * 1024 * 1024,
            )?)
        || receipt.seal_sha256
            != sha256(&read_regular(&root.join("capture/seal.json"), 1024 * 1024)?)
        || receipt.trace_sha256 != sha256(&read_regular(&root.join("trace.json"), MAX_TRACE_BYTES)?)
    {
        return Err("companion byte/input/descriptor binding mismatch".into());
    }
    // Sealed physical replay remains meaningful after input/source drift, but
    // unchanged companion bytes cannot turn a failed run-integrity verdict into
    // a passing composite verification.
    if !evaluation.integrity_passed {
        return Err("execution source/input/descriptor integrity failed".into());
    }
    Ok(())
}

pub fn verify_bundle(root: &Path) -> VerifiedBundleV1 {
    let mut resources = ProofResourcesV1::default();
    let physical = (|| {
        no_symlinks(&root.join("capture"))?;
        // Bound the neutral loader's possible work before invoking native replay.
        // A small file can otherwise declare billions of transitions per action.
        let bounded: EpisodeEvaluationV1 = serde_json::from_slice(&read_regular(
            &root.join("capture/evaluation.json"),
            64 * 1024 * 1024,
        )?)
        .map_err(|e| e.to_string())?;
        let ctx = RunContext::from_scenario(&bounded.scenario)?;
        let horizon = pd_run::scenario_horizon_limits(&ctx).map_err(|e| e.to_string())?;
        if ctx.sim.physics_hz != 120
            || ctx.sim.controller_hz != 60
            || horizon.hard_end_physics_step > MAX_PHYSICS_STEPS
            || bounded.execution.episode.limits.hard_end_physics_step
                > horizon.hard_end_physics_step
            || bounded.execution.episode.run.actions.len() as u64
                > horizon.hard_end_physics_step.div_ceil(2)
        {
            return Err("capture exceeds composition cadence/horizon/update bounds".into());
        }
        resources.neutral_replay_passes_started += 1;
        load_verified_evaluation(&root.join("capture"))
    })();
    let unavailable =
        || CheckV1::Unavailable("requires verified ordinary capture and bounded trace".into());
    let mut report = VerificationReportV1 {
        schema_version: 1,
        bundle_integrity: unavailable(),
        physical_replay: unavailable(),
        controller_reconstruction: unavailable(),
        planner_reconstruction: unavailable(),
        neutral_verified_target_landing: false,
        resources: ProofResourcesV1::default(),
    };
    let evaluation = match physical {
        Ok(evaluation) => {
            if evaluation.replay.physical_prefix_verified {
                resources.known_neutral_physics_steps =
                    evaluation.execution.episode.final_state.physics_step;
            } else {
                resources.neutral_passes_without_step_count = 1;
            }
            report.physical_replay = if evaluation.replay.complete_episode_verified {
                CheckV1::Passed
            } else if evaluation.replay.physical_prefix_verified {
                CheckV1::Failed(
                    "native physical prefix verified; complete disposition differs".into(),
                )
            } else {
                CheckV1::Failed("physical replay mismatch".into())
            };
            report.neutral_verified_target_landing = evaluation.verified_target_landing;
            Some(evaluation)
        }
        Err(reason) => {
            resources.neutral_passes_without_step_count = resources.neutral_replay_passes_started;
            report.physical_replay = CheckV1::Failed(reason);
            None
        }
    };
    let trace: Result<ManeuverTraceV1, String> =
        read_regular(&root.join("trace.json"), MAX_TRACE_BYTES)
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()));
    match (&evaluation, trace) {
        (Some(evaluation), Ok(trace)) => {
            report.bundle_integrity = check(binding(root, evaluation, &trace));
            report.controller_reconstruction =
                reconstruct_controller(evaluation, &trace, &mut resources);
            report.planner_reconstruction = reconstruct_planner(evaluation, &trace, &mut resources);
        }
        (_, Err(reason)) => report.bundle_integrity = CheckV1::Failed(reason),
        _ => {}
    }
    report.resources = resources;
    VerifiedBundleV1 { evaluation, report }
}
