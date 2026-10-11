//! Fresh-process verification of saved evidence. No incumbent or planner import.
use std::{collections::BTreeMap, path::PathBuf};

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: verify-collection COLLECTION SAVED_CAMPAIGN NEW_RECEIPT_JSON".into());
    }
    let collection = PathBuf::from(&args[0]);
    let root = PathBuf::from(&args[1]);
    let bytes = std::fs::read(root.join("summary.json")).map_err(|e| e.to_string())?;
    let campaign: pd_eval::CollectionEvaluationV1 =
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if !campaign.integrity_passed
        || campaign.evidence_before != campaign.evidence_after
        || campaign.request.manifest_sha256
            != pd_missions::sha256(
                &std::fs::read(collection.join("manifest.json")).map_err(|e| e.to_string())?,
            )
        || campaign.attempts.len() != campaign.request.attempt_indices.len()
    {
        return Err("campaign bindings, integrity or schedule differ".into());
    }
    for file in campaign
        .evidence_before
        .source_files
        .iter()
        .chain(&campaign.evidence_before.input_files)
    {
        let current = std::fs::read(&file.path).map_err(|e| e.to_string())?;
        if pd_missions::sha256(&current) != file.sha256 {
            return Err(format!("saved evidence changed: {}", file.path));
        }
    }
    let manifest = pd_missions::read_versioned_collection_manifest(&collection)?;
    let entries: BTreeMap<_, _> = manifest.entries().iter().map(|e| (e.index, e)).collect();
    let mut executed = 0;
    let mut landed = 0;
    let mut unsupported = 0;
    let mut failed = 0;
    for (ordinal, attempt) in campaign.attempts.iter().enumerate() {
        if attempt.attempt_index != ordinal as u64
            || attempt.mission_index != campaign.request.attempt_indices[ordinal]
        {
            return Err("attempt schedule differs".into());
        }
        match &attempt.disposition {
            pd_eval::AttemptDispositionV1::Executed {
                capture,
                stop,
                physical_outcome,
                mission_outcome,
                integrity_passed,
                physical_replay_verified,
                complete_replay_verified,
                verified_target_landing,
            } => {
                if capture != &format!("attempt-{ordinal:06}") {
                    return Err("capture path differs from schedule".into());
                }
                let evaluation = pd_eval::load_verified_evaluation(&root.join(capture))?;
                let input = pd_missions::load_versioned_collection_mission(
                    &collection,
                    &manifest,
                    entries
                        .get(&attempt.mission_index)
                        .ok_or("missing admitted input")?,
                )?;
                let episode = &evaluation.execution.episode;
                if &evaluation.scenario != input.scenario()
                    || evaluation.execution.descriptor != campaign.request.agent
                    || episode.stop != *stop
                    || episode.run.manifest.physical_outcome != *physical_outcome
                    || episode.run.manifest.mission_outcome != *mission_outcome
                    || evaluation.integrity_passed != *integrity_passed
                    || evaluation.replay.physical_prefix_verified != *physical_replay_verified
                    || evaluation.replay.complete_episode_verified != *complete_replay_verified
                    || evaluation.verified_target_landing != *verified_target_landing
                    || !evaluation.integrity_passed
                    || !evaluation.replay.physical_prefix_verified
                    || !evaluation.replay.complete_episode_verified
                {
                    return Err(format!(
                        "attempt {ordinal} input, outcome or replay differs"
                    ));
                }
                executed += 1;
                landed += usize::from(*verified_target_landing);
            }
            pd_eval::AttemptDispositionV1::Unsupported { .. } => unsupported += 1,
            _ => failed += 1,
        }
    }
    let result = serde_json::json!({"schema_version":1,"all_passed":true,"directory":root,"summary_sha256":pd_missions::sha256(&bytes),"scheduled":campaign.attempts.len(),"unique_inputs":campaign.unique_scheduled_missions,"executed":executed,"verified_target_landings":landed,"unsupported":unsupported,"failures":failed,"verifier_executable_sha256":pd_missions::sha256(&std::fs::read(std::env::current_exe().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?),"new_controller_flights":0});
    pd_missions::write_json_create_only(&PathBuf::from(&args[2]), &result)?;
    println!(
        "{executed} captures independently replayed, {landed} verified landings, {unsupported} unsupported, {failed} failed attempts"
    );
    Ok(())
}
