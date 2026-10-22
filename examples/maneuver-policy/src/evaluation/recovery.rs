use super::*;
use crate::verify;
use std::{collections::BTreeMap, path::Path};

pub(super) fn continuation_rows(
    previous: &Path,
    request: &RunRequestV1,
) -> Result<BTreeMap<String, AttemptRowV1>, String> {
    let old: RunRequestV1 = read_json(&previous.join("request.json"), 1024 * 1024)?;
    if old.schema_version != 1
        || old.plan_root != request.plan_root
        || old.plan_sha256 != request.plan_sha256
        || old.executable_sha256 != request.executable_sha256
        || old.source_files_sha256 != request.source_files_sha256
    {
        return Err("continuation requires identical plan, source paths and executable".into());
    }
    if bind_file(&previous.join("source.json"), 4 * 1024 * 1024)?.sha256
        != old.source_snapshot_sha256
    {
        return Err("continuation source record changed".into());
    }
    let mut rows = BTreeMap::new();
    let directory = previous.join("records");
    pd_missions::check_input_path(&directory)?;
    for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let row: AttemptRowV1 = read_json(&entry.path(), 1024 * 1024)?;
        if row.schema_version != 1
            || entry.file_name().to_string_lossy() != format!("{}.json", row.attempt_id)
            || rows.insert(row.attempt_id.clone(), row).is_some()
        {
            return Err("invalid prior row identity".into());
        }
    }
    Ok(rows)
}

pub(super) fn reuse_row(
    old: &AttemptRowV1,
    attempt: &PreparedAttemptV1,
    case: &PreparedCaseV1,
    plan_sha: &str,
) -> Result<Option<AttemptRowV1>, String> {
    if old.attempt_id != attempt.id
        || old.case_index != attempt.case_index
        || old.program_index != attempt.program_index
    {
        return Err("continuation row schedule mismatch".into());
    }
    if !matches!(
        old.status,
        AttemptStatusV1::Verified | AttemptStatusV1::ReusedVerified
    ) {
        return Ok(None);
    }
    let bound = old
        .result
        .as_ref()
        .ok_or("completed prior row has no result")?;
    let result = read_attempt_result(bound)?;
    if result.attempt_id != attempt.id
        || result.plan_sha256 != plan_sha
        || result.physical_sha256 != case.physical_sha256
        || result.program_sha256 != attempt.program_sha256
        || !result.outer_integrity
        || !result.verification.passed()
        || result.reference.as_ref().is_some_and(|r| !r.passed)
    {
        return Err("prior result binding mismatch".into());
    }
    let root = Path::new(&bound.path)
        .parent()
        .ok_or("result has no parent")?;
    let verified = verify::verify_bundle(&root.join("bundle"));
    if !verified.report.passed() {
        return Err("prior attempt no longer verifies".into());
    }
    let evaluation = verified
        .evaluation
        .ok_or("prior neutral evaluation unavailable")?;
    let episode = &evaluation.execution.episode;
    let trace: crate::trace::ManeuverTraceV1 =
        read_json(&root.join("bundle/trace.json"), 32 * 1024 * 1024)?;
    if result.native.manifest != episode.run.manifest
        || result.native.final_state != episode.final_state
        || result.native.incoming_contact != episode.incoming_contact
        || result.native.limits != episode.limits
        || result.native.failure != episode.failure
        || result.native.stop != episode.stop
        || result.original != trace.counters
        || result.verified_target_landing != evaluation.verified_target_landing
        || result.actual_activation_tick
            != trace
                .final_feedback
                .accepted
                .as_ref()
                .map(|a| a.activation_physics_step)
    {
        return Err("prior result does not match authoritative capture/trace".into());
    }
    Ok(Some(AttemptRowV1 {
        schema_version: 1,
        attempt_id: attempt.id.clone(),
        case_index: attempt.case_index,
        program_index: attempt.program_index,
        status: AttemptStatusV1::ReusedVerified,
        reason: None,
        result: Some(bound.clone()),
        reused_from: Some(bound.path.clone()),
        reuse_resources: Some(verified.report.resources),
        supervised_worker_wall_time_us: None,
    }))
}
