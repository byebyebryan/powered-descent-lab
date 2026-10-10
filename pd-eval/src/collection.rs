use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use pd_core::{BoundedRunLimitsV1, BoundedRunStopCauseV1, MissionOutcome, PhysicalOutcome};
use pd_missions::{
    CollectionExposureV1, load_collection_mission, read_collection_manifest,
    record_collection_usage, sha256, write_json_create_only,
};
use pd_run::{Agent, AgentDescriptorV1};
use serde::{Deserialize, Serialize};

use crate::{EvidenceScope, evaluate_agent, save_evaluation};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionEvaluationRequestV1 {
    pub schema_version: u32,
    pub manifest_sha256: String,
    pub agent: AgentDescriptorV1,
    /// Repeated indices are explicit scheduled attempts, not new unique missions.
    pub attempt_indices: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AttemptDispositionV1 {
    Executed {
        capture: String,
        stop: BoundedRunStopCauseV1,
        physical_outcome: PhysicalOutcome,
        mission_outcome: MissionOutcome,
        integrity_passed: bool,
        physical_replay_verified: bool,
        complete_replay_verified: bool,
        verified_target_landing: bool,
    },
    InputInvalid {
        detail: String,
    },
    InitializationFailed {
        detail: String,
    },
    ExecutionFailed {
        detail: String,
    },
    CaptureFailed {
        detail: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionAttemptV1 {
    pub attempt_index: u64,
    pub mission_index: u64,
    pub disposition: AttemptDispositionV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionEvaluationV1 {
    pub schema_version: u32,
    pub request: CollectionEvaluationRequestV1,
    pub unique_scheduled_missions: usize,
    pub attempts: Vec<CollectionAttemptV1>,
}

/// Composition provides fresh implementations through an open factory. Mission
/// membership never reaches the factory or the agent's physical decision view.
/// Every scheduled attempt receives a disposition, including failed preparation.
pub fn evaluate_collection(
    collection_root: &Path,
    output: &Path,
    request: CollectionEvaluationRequestV1,
    factory: &mut dyn FnMut() -> Result<Box<dyn Agent>, String>,
    evidence: &EvidenceScope,
) -> Result<CollectionEvaluationV1, String> {
    request.agent.validate()?;
    let manifest = read_collection_manifest(collection_root)?;
    let manifest_sha =
        sha256(&fs::read(collection_root.join("manifest.json")).map_err(|e| e.to_string())?);
    let available: BTreeSet<_> = manifest.entries.iter().map(|entry| entry.index).collect();
    if request.schema_version != 1
        || request.manifest_sha256 != manifest_sha
        || request.attempt_indices.is_empty()
        || request.attempt_indices.len() > 100_000
        || request
            .attempt_indices
            .iter()
            .any(|index| !available.contains(index))
    {
        return Err(
            "evaluation request version, collection identity or schedule is invalid".to_owned(),
        );
    }
    fs::create_dir(output)
        .map_err(|e| format!("reserve collection evaluation {}: {e}", output.display()))?;
    write_json_create_only(&output.join("request.json"), &request)?;
    let usage_id = format!(
        "eval-{}",
        &sha256(output.to_string_lossy().as_bytes())[..24]
    );
    record_collection_usage(
        collection_root,
        &usage_id,
        CollectionExposureV1::Evaluation,
        "Consumed scheduled development/evaluation inputs; consult capture request for attempts",
    )?;
    let mut evaluation = CollectionEvaluationV1 {
        schema_version: 1,
        unique_scheduled_missions: request
            .attempt_indices
            .iter()
            .collect::<BTreeSet<_>>()
            .len(),
        request,
        attempts: Vec::new(),
    };
    for (position, &index) in evaluation.request.attempt_indices.iter().enumerate() {
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.index == index)
            .expect("validated schedule");
        let disposition = match load_collection_mission(collection_root, &manifest, entry) {
            Err(detail) => AttemptDispositionV1::InputInvalid { detail },
            Ok(mission) => match factory() {
                Err(detail) => AttemptDispositionV1::InitializationFailed { detail },
                Ok(agent) if agent.descriptor() != evaluation.request.agent => {
                    AttemptDispositionV1::InitializationFailed {
                        detail: "factory descriptor differs from declared approach".to_owned(),
                    }
                }
                Ok(mut agent) => {
                    let horizon = (mission.scenario.sim.max_time_s
                        * f64::from(mission.scenario.sim.physics_hz))
                    .ceil() as u64;
                    let interval = mission.scenario.sim.control_interval_steps();
                    let coverage = horizon.div_ceil(interval) * interval;
                    let limits = BoundedRunLimitsV1 {
                        command_coverage_end_physics_step: coverage,
                        hard_end_physics_step: horizon,
                    };
                    match evaluate_agent(&mission.scenario, agent.as_mut(), limits, evidence) {
                        Err(detail) => AttemptDispositionV1::ExecutionFailed { detail },
                        Ok(result) => {
                            let capture = format!("attempt-{position:06}");
                            match save_evaluation(&output.join(&capture), &result) {
                                Err(detail) => AttemptDispositionV1::CaptureFailed { detail },
                                Ok(_) => AttemptDispositionV1::Executed {
                                    capture,
                                    stop: result.execution.episode.stop,
                                    physical_outcome: result
                                        .execution
                                        .episode
                                        .run
                                        .manifest
                                        .physical_outcome,
                                    mission_outcome: result
                                        .execution
                                        .episode
                                        .run
                                        .manifest
                                        .mission_outcome,
                                    integrity_passed: result.integrity_passed,
                                    physical_replay_verified: result
                                        .replay
                                        .physical_prefix_verified,
                                    complete_replay_verified: result
                                        .replay
                                        .complete_episode_verified,
                                    verified_target_landing: result.verified_target_landing,
                                },
                            }
                        }
                    }
                }
            },
        };
        let attempt = CollectionAttemptV1 {
            attempt_index: position as u64,
            mission_index: index,
            disposition,
        };
        write_json_create_only(
            &output.join(format!("attempt-{position:06}.json")),
            &attempt,
        )?;
        evaluation.attempts.push(attempt);
    }
    write_json_create_only(&output.join("summary.json"), &evaluation)?;
    Ok(evaluation)
}
