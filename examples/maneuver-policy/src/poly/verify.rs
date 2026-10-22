//! V2 companion evidence reuses neutral sealing/replay. Unknown backends and
//! old companions cannot silently select this reconstructor.
use std::{fs, path::Path};

use pd_eval::{EpisodeEvaluationV1, load_verified_evaluation, save_evaluation};
use pd_missions::{sha256, write_json_create_only};
use serde::{Deserialize, Serialize};

use super::*;
use crate::verify::{CheckV1, read_regular};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedProofResourcesV2 {
    pub neutral_replay_passes_started: u64,
    pub known_neutral_physics_steps: u64,
    pub neutral_passes_without_step_count: u64,
    pub controller_reconstruction_passes_started: u64,
    pub planner_reconstruction_passes_started: u64,
    pub reconstruction_physics_steps: u64,
    pub reconstruction_command_updates: u64,
    pub reconstruction_reference_constructions: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub reconstruction_admission_actuation_checks: u64,
    pub reconstruction_planner_requests: u64,
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedBundleReceiptV2 {
    pub schema_version: u32,
    pub descriptor: AgentDescriptorV1,
    pub physical_sha256: String,
    pub input_files: Vec<pd_eval::FileIdentityV1>,
    pub evaluation_sha256: String,
    pub seal_sha256: String,
    pub trace_sha256: String,
    pub preparation_resources: TimedProofResourcesV2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedVerificationReportV2 {
    pub schema_version: u32,
    pub bundle_integrity: CheckV1,
    pub physical_replay: CheckV1,
    pub controller_reconstruction: CheckV1,
    pub planner_reconstruction: CheckV1,
    pub neutral_verified_target_landing: bool,
    pub resources: TimedProofResourcesV2,
}
impl TimedVerificationReportV2 {
    pub fn passed(&self) -> bool {
        self.bundle_integrity.passed()
            && self.physical_replay.passed()
            && self.controller_reconstruction.passed()
            && self.planner_reconstruction.passed()
    }
}
pub struct TimedVerifiedBundleV2 {
    pub evaluation: Option<EpisodeEvaluationV1>,
    pub report: TimedVerificationReportV2,
}

fn check(result: Result<(), String>) -> CheckV1 {
    match result {
        Ok(()) => CheckV1::Passed,
        Err(reason) => CheckV1::Failed(reason),
    }
}

fn same_wire<T: Serialize>(left: &T, right: &T) -> Result<bool, String> {
    Ok(serde_json::to_vec(left).map_err(|e| e.to_string())?
        == serde_json::to_vec(right).map_err(|e| e.to_string())?)
}

fn compatibility(
    evaluation: &EpisodeEvaluationV1,
    trace: &TimedTraceV2,
) -> Result<TimedCompositionConfigV2, String> {
    let current = crate::source_scope(Vec::new()).snapshot()?;
    crate::check_compiled_sources(&current)?;
    if evaluation.evidence_before.source_files != current.source_files
        || evaluation.evidence_after.source_files != current.source_files
        || evaluation.evidence_before.executable_sha256 != current.executable_sha256
        || evaluation.evidence_after.executable_sha256 != current.executable_sha256
    {
        return Err(
            "frozen compiled source/executable mismatch; native replay remains independent".into(),
        );
    }
    let config: TimedCompositionConfigV2 =
        serde_json::from_value(trace.descriptor.configuration.clone())
            .map_err(|e| e.to_string())?;
    let c = controller(&evaluation.scenario, &config.program)?;
    if config.schema_version != config.program.schema_version
        || trace.schema_version != config.program.schema_version
        || trace.descriptor != evaluation.execution.descriptor
        || trace.descriptor != descriptor(&config.program, &c)
        || trace.boundaries.len() as u64
            > config.program.absolute_deadline_physics_step.div_ceil(2) + 1
        || evaluation.execution.episode.limits.hard_end_physics_step
            != config.program.absolute_deadline_physics_step
        || evaluation
            .execution
            .episode
            .limits
            .command_coverage_end_physics_step
            != config.program.coverage_end()
    {
        return Err(
            "unsupported timed descriptor, backend, trace or original clock binding".into(),
        );
    }
    Ok(config)
}

fn reconstruct(
    evaluation: &EpisodeEvaluationV1,
    trace: &TimedTraceV2,
    recorded_requests: bool,
    resources: &mut TimedProofResourcesV2,
) -> Result<(), String> {
    let config = compatibility(evaluation, trace)?;
    if recorded_requests {
        resources.controller_reconstruction_passes_started += 1;
    } else {
        resources.planner_reconstruction_passes_started += 1;
    }
    let result = run_with_recorded_requests(
        &evaluation.scenario,
        &config.program,
        recorded_requests.then_some(trace.boundaries.as_slice()),
    )?;
    resources.reconstruction_physics_steps += result.trace.physics_steps;
    resources.reconstruction_command_updates += result.trace.command_updates;
    resources.reconstruction_reference_constructions += result.trace.feedback.constructions;
    resources.reconstruction_admission_actuation_checks +=
        result.trace.feedback.admission_actuation_checks;
    resources.reconstruction_planner_requests += result.trace.planner_requests;
    if !same_wire(&result.trace, trace)? {
        return Err(if recorded_requests {
            "timed controller admission/reference/lifecycle reconstruction differs"
        } else {
            "timed planner request/clock/ownership reconstruction differs"
        }
        .into());
    }
    let episode = result
        .episode
        .capture
        .ok_or("captured reconstruction missing")?;
    if !same_wire(&episode, &evaluation.execution.episode)?
        || !same_wire(
            &result.episode.agent_updates,
            &Some(evaluation.execution.controller_updates.clone()),
        )?
    {
        return Err(
            "timed commands, observations, physical ledger or final native prefix differs".into(),
        );
    }
    Ok(())
}

pub fn save_bundle(
    root: &Path,
    evaluation: &EpisodeEvaluationV1,
    trace: &TimedTraceV2,
) -> Result<(), String> {
    crate::verify::no_symlinks(root.parent().ok_or("timed bundle needs a parent")?)?;
    fs::create_dir(root).map_err(|e| format!("reserve new timed bundle: {e}"))?;
    save_evaluation(&root.join("capture"), evaluation)?;
    write_json_create_only(&root.join("trace.json"), trace)?;
    let verified = evaluation.replay.physical_prefix_verified;
    let mut resources = TimedProofResourcesV2 {
        neutral_replay_passes_started: 2,
        known_neutral_physics_steps: if verified {
            evaluation.execution.episode.final_state.physics_step * 2
        } else {
            0
        },
        neutral_passes_without_step_count: if verified { 0 } else { 2 },
        ..TimedProofResourcesV2::default()
    };
    let _controller = reconstruct(evaluation, trace, true, &mut resources);
    let _planner = reconstruct(evaluation, trace, false, &mut resources);
    let receipt = TimedBundleReceiptV2 {
        schema_version: 2,
        descriptor: evaluation.execution.descriptor.clone(),
        physical_sha256: evaluation.physical_sha256.clone(),
        input_files: evaluation.evidence_before.input_files.clone(),
        evaluation_sha256: sha256(&read_regular(
            &root.join("capture/evaluation.json"),
            64 * 1024 * 1024,
        )?),
        seal_sha256: sha256(&read_regular(&root.join("capture/seal.json"), 1024 * 1024)?),
        trace_sha256: sha256(&read_regular(&root.join("trace.json"), 32 * 1024 * 1024)?),
        preparation_resources: resources,
    };
    write_json_create_only(&root.join("receipt.json"), &receipt)
}

fn binding(
    root: &Path,
    evaluation: &EpisodeEvaluationV1,
    trace: &TimedTraceV2,
) -> Result<(), String> {
    let receipt: TimedBundleReceiptV2 =
        serde_json::from_slice(&read_regular(&root.join("receipt.json"), 32 * 1024 * 1024)?)
            .map_err(|e| e.to_string())?;
    if receipt.schema_version != 2
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
        || receipt.trace_sha256
            != sha256(&read_regular(&root.join("trace.json"), 32 * 1024 * 1024)?)
        || !evaluation.integrity_passed
    {
        return Err("timed companion byte/input/descriptor/integrity binding mismatch".into());
    }
    Ok(())
}

pub fn verify_bundle(root: &Path) -> TimedVerifiedBundleV2 {
    let mut resources = TimedProofResourcesV2::default();
    let physical = (|| {
        let bounded: EpisodeEvaluationV1 = serde_json::from_slice(&read_regular(
            &root.join("capture/evaluation.json"),
            64 * 1024 * 1024,
        )?)
        .map_err(|e| e.to_string())?;
        let ctx = RunContext::from_scenario(&bounded.scenario)?;
        let horizon = pd_run::scenario_horizon_limits(&ctx).map_err(|e| e.to_string())?;
        if ctx.sim.physics_hz != 120
            || ctx.sim.controller_hz != 60
            || horizon.hard_end_physics_step > crate::MAX_PHYSICS_STEPS
            || bounded.execution.episode.limits.hard_end_physics_step
                > horizon.hard_end_physics_step
            || bounded.execution.episode.run.actions.len() as u64
                > horizon.hard_end_physics_step.div_ceil(2)
        {
            return Err("timed capture exceeds native cadence/horizon/update bounds".into());
        }
        resources.neutral_replay_passes_started += 1;
        load_verified_evaluation(&root.join("capture"))
    })();
    let unavailable = || {
        CheckV1::Unavailable("requires bounded verified ordinary capture and timed trace".into())
    };
    let mut report = TimedVerificationReportV2 {
        schema_version: 2,
        bundle_integrity: unavailable(),
        physical_replay: unavailable(),
        controller_reconstruction: unavailable(),
        planner_reconstruction: unavailable(),
        neutral_verified_target_landing: false,
        resources: TimedProofResourcesV2::default(),
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
            } else {
                CheckV1::Failed("ordinary physical prefix/disposition replay failed".into())
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
    let trace = read_regular(&root.join("trace.json"), 32 * 1024 * 1024).and_then(|bytes| {
        serde_json::from_slice::<TimedTraceV2>(&bytes).map_err(|e| e.to_string())
    });
    match (&evaluation, trace) {
        (Some(evaluation), Ok(trace)) => {
            report.bundle_integrity = check(binding(root, evaluation, &trace));
            report.controller_reconstruction =
                check(reconstruct(evaluation, &trace, true, &mut resources));
            report.planner_reconstruction =
                check(reconstruct(evaluation, &trace, false, &mut resources));
        }
        (_, Err(reason)) => report.bundle_integrity = CheckV1::Failed(reason),
        _ => {}
    }
    report.resources = resources;
    TimedVerifiedBundleV2 { evaluation, report }
}

#[cfg(test)]
mod tests {
    #[test]
    fn exact_wire_comparison_retains_signed_zero_in_commands() {
        assert!(!super::same_wire(&0.0_f64, &-0.0_f64).unwrap());
    }
}
