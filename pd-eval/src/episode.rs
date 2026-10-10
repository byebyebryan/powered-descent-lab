use pd_core::{
    AllowAllBoundedRunGuard, BoundedRunLimitsV1, BoundedRunStopCauseV1, MissionOutcome,
    PhysicalOutcome, RunContext, ScenarioSpec, replay_simulation_bounded,
};
use pd_missions::physical_identity;
use pd_run::{Agent, BoundedAgentRunArtifactsV1, run_agent};
use serde::{Deserialize, Serialize};

use crate::{EvidenceScope, EvidenceSnapshotV1};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayAssessmentV1 {
    pub physical_prefix_verified: bool,
    pub complete_episode_verified: bool,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeEvaluationV1 {
    pub schema_version: u32,
    pub scenario: ScenarioSpec,
    pub physical_sha256: String,
    pub execution: BoundedAgentRunArtifactsV1,
    pub evidence_before: EvidenceSnapshotV1,
    pub evidence_after: EvidenceSnapshotV1,
    pub agent_descriptor_unchanged: bool,
    pub integrity_passed: bool,
    pub replay: ReplayAssessmentV1,
    pub verified_target_landing: bool,
}

/// Always evaluates the actual mission. Failed decisions retain their prefix,
/// with physical replay kept separate from unrepeatable policy failure reasons.
pub fn evaluate_agent(
    scenario: &ScenarioSpec,
    agent: &mut dyn Agent,
    limits: BoundedRunLimitsV1,
    evidence: &EvidenceScope,
) -> Result<EpisodeEvaluationV1, String> {
    let ctx = RunContext::from_scenario(scenario)?;
    let before = evidence.snapshot()?;
    let execution = run_agent(&ctx, agent, limits).map_err(|e| e.to_string())?;
    let descriptor_unchanged = execution.descriptor == agent.descriptor();
    let after = evidence.snapshot()?;
    let replay = verify_execution(scenario, &execution)?;
    let integrity_passed = before == after && descriptor_unchanged;
    let mut evaluation = EpisodeEvaluationV1 {
        schema_version: 1,
        scenario: scenario.clone(),
        physical_sha256: physical_identity(scenario).map_err(|e| e.to_string())?,
        execution,
        evidence_before: before,
        evidence_after: after,
        agent_descriptor_unchanged: descriptor_unchanged,
        integrity_passed,
        replay,
        verified_target_landing: false,
    };
    evaluation.verified_target_landing = verified_landing(&evaluation);
    Ok(evaluation)
}

/// Executes commands only; never instantiates a policy or imports a strategy.
pub fn verify_execution(
    scenario: &ScenarioSpec,
    execution: &BoundedAgentRunArtifactsV1,
) -> Result<ReplayAssessmentV1, String> {
    let ctx = RunContext::from_scenario(scenario)?;
    execution.descriptor.validate()?;
    let expected = &execution.episode;
    let replay = replay_simulation_bounded(
        &ctx,
        &execution.descriptor.implementation,
        &expected.run.actions,
        expected.limits,
        &mut AllowAllBoundedRunGuard,
    )
    .map_err(|e| e.to_string())?;
    let updates_match = execution.controller_updates.len() == expected.run.actions.len()
        && execution
            .controller_updates
            .iter()
            .zip(&expected.run.actions)
            .all(|(update, action)| {
                update.physics_step == action.physics_step
                    && update.sim_time_s == action.sim_time_s
                    && update.controller_update_index == action.controller_update_index
                    && update.frame.command == action.command
            });
    let prefix = updates_match
        && expected.run == replay.run
        && expected.final_state == replay.final_state
        && expected.incoming_contact == replay.incoming_contact;
    let complete = prefix && *expected == replay;
    Ok(ReplayAssessmentV1 {
        physical_prefix_verified: prefix,
        complete_episode_verified: complete,
        detail: if complete {
            "Exact bounded episode replay"
        } else if prefix {
            "Physical prefix replayed; native policy/guard failure disposition remains separate"
        } else {
            "Command, state, event, sample or update ledger mismatch"
        }
        .to_owned(),
    })
}

pub fn validate_evaluation(evaluation: &EpisodeEvaluationV1) -> Result<(), String> {
    if evaluation.schema_version != 1
        || evaluation.physical_sha256
            != physical_identity(&evaluation.scenario).map_err(|e| e.to_string())?
        || evaluation.evidence_before.schema_version != 1
        || evaluation.evidence_after.schema_version != 1
        || evaluation.integrity_passed
            != (evaluation.evidence_before == evaluation.evidence_after
                && evaluation.agent_descriptor_unchanged)
        || evaluation.replay != verify_execution(&evaluation.scenario, &evaluation.execution)?
        || evaluation.verified_target_landing != verified_landing(evaluation)
    {
        return Err("evaluation identity, integrity or replay assessment differs from authoritative evidence".to_owned());
    }
    Ok(())
}

fn verified_landing(evaluation: &EpisodeEvaluationV1) -> bool {
    let episode = &evaluation.execution.episode;
    evaluation.integrity_passed
        && evaluation.replay.complete_episode_verified
        && episode.failure.is_none()
        && episode.stop == BoundedRunStopCauseV1::MissionTerminal
        && episode.run.manifest.physical_outcome == PhysicalOutcome::LandedOnTarget
        && episode.run.manifest.mission_outcome == MissionOutcome::Success
}
