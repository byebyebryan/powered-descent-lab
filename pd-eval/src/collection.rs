use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use pd_core::{BoundedRunStopCauseV1, MissionOutcome, PhysicalOutcome, ScenarioSpec};
use pd_missions::{
    CollectionExposureV1, load_collection_mission, read_collection_manifest,
    record_collection_usage, sha256, write_json_create_only,
};
use pd_run::{Agent, AgentDescriptorV1};
use serde::{Deserialize, Serialize};

use crate::{EpisodeEvaluationV1, EvidenceScope, evaluate_agent, save_evaluation};

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
    Unsupported {
        detail: String,
    },
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

/// Preparation failures preserve their approach-specific meaning and remain
/// in the common scheduled denominator. No strategy dependency is needed here.
pub enum AttemptExecutionErrorV1 {
    Unsupported(String),
    Initialization(String),
    Execution(String),
}

pub type CollectionExecutorV1<'a> = dyn FnMut(&ScenarioSpec, &EvidenceScope) -> Result<EpisodeEvaluationV1, AttemptExecutionErrorV1>
    + 'a;

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
    pub evidence_before: crate::EvidenceSnapshotV1,
    pub evidence_after: crate::EvidenceSnapshotV1,
    pub integrity_passed: bool,
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
    let descriptor = request.agent.clone();
    evaluate_collection_with_executor(
        collection_root,
        output,
        request,
        &mut |scenario, scope| {
            let mut agent = factory().map_err(AttemptExecutionErrorV1::Initialization)?;
            if agent.descriptor() != descriptor {
                return Err(AttemptExecutionErrorV1::Initialization(
                    "factory descriptor differs from declared approach".to_owned(),
                ));
            }
            let ctx = pd_core::RunContext::from_scenario(scenario)
                .map_err(AttemptExecutionErrorV1::Execution)?;
            let limits = pd_run::scenario_horizon_limits(&ctx)
                .map_err(|e| AttemptExecutionErrorV1::Execution(e.to_string()))?;
            evaluate_agent(scenario, agent.as_mut(), limits, scope)
                .map_err(AttemptExecutionErrorV1::Execution)
        },
        evidence,
    )
}

/// Common campaign driver for incremental agents and offline approaches.
/// The complete collection is bound once before/after the campaign; each
/// episode binds its selected mission, checkpoint and explicit source scope.
/// Overall integrity must pass before publishing campaign landing counts.
pub fn evaluate_collection_with_executor(
    collection_root: &Path,
    output: &Path,
    request: CollectionEvaluationRequestV1,
    executor: &mut CollectionExecutorV1<'_>,
    evidence: &EvidenceScope,
) -> Result<CollectionEvaluationV1, String> {
    request.agent.validate()?;
    let manifest = read_collection_manifest(collection_root)?;
    let manifest_sha =
        sha256(&fs::read(collection_root.join("manifest.json")).map_err(|e| e.to_string())?);
    let available: BTreeMap<_, _> = manifest
        .entries
        .iter()
        .map(|entry| (entry.index, entry))
        .collect();
    if request.schema_version != 1
        || request.manifest_sha256 != manifest_sha
        || request.attempt_indices.is_empty()
        || request.attempt_indices.len() > 100_000
        || request
            .attempt_indices
            .iter()
            .any(|index| !available.contains_key(index))
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
    pd_missions::read_collection_usage(collection_root)?;
    let collection_path = fs::canonicalize(collection_root).map_err(|e| e.to_string())?;
    // Bind declared immutable inputs and the exposure records present at entry.
    // A later append-only usage receipt is not a mutation of physical inputs.
    let mut collection_files = vec![
        collection_root.join("manifest.json"),
        collection_root.join("request.json"),
    ];
    collection_files.extend(
        manifest
            .entries
            .iter()
            .map(|entry| collection_root.join(&entry.file)),
    );
    for entry in fs::read_dir(collection_root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name().to_string_lossy().starts_with("usage-") {
            collection_files.push(entry.path());
        }
    }
    let physical_files: BTreeSet<_> = collection_files
        .iter()
        .map(|path| fs::canonicalize(path).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let mut campaign_scope = EvidenceScope {
        sources: evidence.sources.clone(),
        inputs: collection_files,
    };
    let mut episode_scope = EvidenceScope {
        sources: evidence.sources.clone(),
        inputs: Vec::new(),
    };
    for path in &evidence.inputs {
        let canonical = fs::canonicalize(path).map_err(|e| e.to_string())?;
        if canonical != collection_path {
            if collection_path.starts_with(&canonical) {
                return Err(
                    "input evidence must not bind an ancestor of the collection per episode"
                        .to_owned(),
                );
            }
            campaign_scope.inputs.push(path.clone());
            if !physical_files.contains(&canonical) {
                episode_scope.inputs.push(path.clone());
            }
        }
    }
    let evidence_before = campaign_scope.snapshot()?;
    let mut evaluation = CollectionEvaluationV1 {
        schema_version: 1,
        unique_scheduled_missions: request
            .attempt_indices
            .iter()
            .collect::<BTreeSet<_>>()
            .len(),
        request,
        attempts: Vec::new(),
        evidence_after: evidence_before.clone(),
        evidence_before,
        integrity_passed: false,
    };
    for (position, &index) in evaluation.request.attempt_indices.iter().enumerate() {
        let entry = available[&index];
        let mut selected_scope = episode_scope.clone();
        selected_scope
            .inputs
            .push(collection_root.join(&entry.file));
        let disposition = match load_collection_mission(collection_root, &manifest, entry) {
            Err(detail) => AttemptDispositionV1::InputInvalid { detail },
            Ok(mission) => match executor(&mission.scenario, &selected_scope) {
                Err(AttemptExecutionErrorV1::Unsupported(detail)) => {
                    AttemptDispositionV1::Unsupported { detail }
                }
                Err(AttemptExecutionErrorV1::Initialization(detail)) => {
                    AttemptDispositionV1::InitializationFailed { detail }
                }
                Err(AttemptExecutionErrorV1::Execution(detail)) => {
                    AttemptDispositionV1::ExecutionFailed { detail }
                }
                Ok(result)
                    if result.scenario != mission.scenario
                        || result.execution.descriptor != evaluation.request.agent =>
                {
                    AttemptDispositionV1::ExecutionFailed {
                        detail:
                            "executor result differs from the scheduled physical input or approach"
                                .to_owned(),
                    }
                }
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
                            mission_outcome: result.execution.episode.run.manifest.mission_outcome,
                            integrity_passed: result.integrity_passed,
                            physical_replay_verified: result.replay.physical_prefix_verified,
                            complete_replay_verified: result.replay.complete_episode_verified,
                            verified_target_landing: result.verified_target_landing,
                        },
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
    evaluation.evidence_after = campaign_scope.snapshot()?;
    evaluation.integrity_passed = evaluation.evidence_before == evaluation.evidence_after;
    write_json_create_only(&output.join("summary.json"), &evaluation)?;
    Ok(evaluation)
}
