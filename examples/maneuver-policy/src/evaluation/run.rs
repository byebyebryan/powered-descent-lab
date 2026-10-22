use super::*;
use pd_missions::{json_bytes, sha256};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunCompletionV1 {
    schema_version: u32,
    request_sha256: String,
    summary_sha256: String,
    row_sha256: Vec<BoundFileV1>,
}

pub fn available_bytes(path: &Path) -> Result<u64, String> {
    let output = Command::new("df")
        .args(["-P", "-B1"])
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("cannot establish available filesystem bytes".into());
    }
    let text = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    text.lines()
        .nth(1)
        .and_then(|line| line.split_whitespace().nth(3))
        .ok_or("unrecognized filesystem capacity")?
        .parse()
        .map_err(|_| "invalid filesystem capacity".into())
}

fn empty_row(
    attempt: &PreparedAttemptV1,
    status: AttemptStatusV1,
    reason: Option<String>,
) -> AttemptRowV1 {
    AttemptRowV1 {
        schema_version: 1,
        attempt_id: attempt.id.clone(),
        case_index: attempt.case_index,
        program_index: attempt.program_index,
        status,
        reason,
        result: None,
        reused_from: None,
        reuse_resources: None,
        supervised_worker_wall_time_us: None,
    }
}

struct AttemptSupervisor<'a> {
    plan_root: &'a Path,
    destination: &'a Path,
    plan: &'a PreparedEvaluationV1,
    plan_sha: &'a str,
    started: Instant,
    metadata_reserve: u64,
    continuation: Option<&'a Path>,
}

pub(super) struct OwnedChild(pub(super) std::process::Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn reserve_output(
    plan: &PreparedEvaluationV1,
    plan_root: &Path,
    output: &Path,
    continuation: Option<&Path>,
) -> Result<PathBuf, String> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    pd_missions::check_input_path(parent)?;
    let destination = fs::canonicalize(parent)
        .map_err(|e| e.to_string())?
        .join(output.file_name().ok_or("result root needs a filename")?);
    if destination.starts_with(plan_root) || plan_root.starts_with(&destination) {
        return Err("result destination overlaps prepared inputs".into());
    }
    if let Some(previous) = continuation {
        let previous = fs::canonicalize(previous).map_err(|e| e.to_string())?;
        if destination.starts_with(&previous) || previous.starts_with(&destination) {
            return Err("continuation destination overlaps old evidence".into());
        }
    }
    let scope = crate::source_scope(vec![]);
    for path in scope.sources.iter().chain(
        plan.input_files
            .iter()
            .map(|b| Path::new(&b.path).to_path_buf())
            .collect::<Vec<_>>()
            .iter(),
    ) {
        if destination.starts_with(path) || path.starts_with(&destination) {
            return Err("result root overlaps bound source/input".into());
        }
    }
    fs::create_dir(&destination).map_err(|e| format!("reserve new evaluation: {e}"))?;
    Ok(destination)
}

pub fn run_evaluation(
    plan_root: &Path,
    output: &Path,
    continuation: Option<&Path>,
) -> Result<EvaluationSummaryV1, String> {
    let started = Instant::now();
    let plan_root = fs::canonicalize(plan_root).map_err(|e| e.to_string())?;
    let audit = check_evaluation(&plan_root)?;
    if !audit.ready {
        return Err(format!(
            "input audit is not ready: {}",
            audit.reasons.join("; ")
        ));
    }
    let plan = load_plan(&plan_root)?;
    let bound = bind_file(&plan_root.join("plan.json"), 32 * 1024 * 1024)?;
    let snapshot = crate::source_scope(vec![]).snapshot()?;
    crate::check_compiled_sources(&snapshot)?;
    let request = RunRequestV1 {
        schema_version: 1,
        plan_root: plan_root.to_string_lossy().into(),
        plan_sha256: bound.sha256.clone(),
        executable_sha256: snapshot.executable_sha256.clone(),
        source_files_sha256: identity(&snapshot.source_files)?,
        source_snapshot_sha256: sha256(&json_bytes(&snapshot)?),
        continuation_request: continuation
            .map(|p| bind_file(&p.join("request.json"), 1024 * 1024))
            .transpose()?,
    };
    let previous = continuation
        .map(|p| super::recovery::continuation_rows(p, &request))
        .transpose()?
        .unwrap_or_default();
    if previous
        .keys()
        .any(|id| !plan.attempts.iter().any(|a| &a.id == id))
    {
        return Err("continuation has out-of-schedule rows".into());
    }
    let destination = reserve_output(&plan, &plan_root, output, continuation)?;
    let budget = &plan.recipe.budget;
    let metadata_reserve = (plan.attempts.len() as u64)
        .checked_mul(32 * 1024)
        .and_then(|v| v.checked_add(4 * 1024 * 1024))
        .ok_or("metadata size overflow")?;
    if metadata_reserve > budget.max_output_bytes {
        return Err("budget cannot hold complete result inventory".into());
    }
    write_limited(&destination.join("request.json"), &request, 1024 * 1024)?;
    write_limited(&destination.join("source.json"), &snapshot, 3 * 1024 * 1024)?;
    fs::create_dir(destination.join("attempts")).map_err(|e| e.to_string())?;
    fs::create_dir(destination.join("records")).map_err(|e| e.to_string())?;
    let mut rows = vec![];
    let mut stop_reason: Option<String> = None;
    let mut used = tree_bytes(&destination)?;
    let supervisor = AttemptSupervisor {
        plan_root: &plan_root,
        destination: &destination,
        plan: &plan,
        plan_sha: &bound.sha256,
        started,
        metadata_reserve,
        continuation,
    };
    for (index, attempt) in plan.attempts.iter().enumerate() {
        let case = &plan.cases[attempt.case_index];
        let row = if let Some(reason) = &stop_reason {
            empty_row(attempt, AttemptStatusV1::NotStarted, Some(reason.clone()))
        } else if started.elapsed().as_secs() >= budget.max_wall_seconds {
            stop_reason = Some("stage wall-time cap reached".into());
            empty_row(attempt, AttemptStatusV1::NotStarted, stop_reason.clone())
        } else if case.disposition != InputDispositionV1::Supported {
            empty_row(
                attempt,
                AttemptStatusV1::InputRejected,
                Some(format!("{:?}", case.disposition)),
            )
        } else if let Some(old) = previous.get(&attempt.id) {
            match old.status {
                AttemptStatusV1::Verified | AttemptStatusV1::ReusedVerified => {
                    run_new_attempt(&supervisor, index, &mut used, &mut stop_reason, true)?
                }
                AttemptStatusV1::Interrupted | AttemptStatusV1::Fault => {
                    stop_reason =
                        Some("prior incomplete attempt cannot be replaced by continuation".into());
                    empty_row(attempt, old.status.clone(), stop_reason.clone())
                }
                AttemptStatusV1::NotStarted => {
                    run_new_attempt(&supervisor, index, &mut used, &mut stop_reason, false)?
                }
                AttemptStatusV1::InputRejected => {
                    stop_reason = Some("prior row contradicts supported input".into());
                    empty_row(attempt, AttemptStatusV1::Fault, stop_reason.clone())
                }
            }
        } else {
            run_new_attempt(&supervisor, index, &mut used, &mut stop_reason, false)?
        };
        used += write_limited(
            &destination
                .join("records")
                .join(format!("{}.json", attempt.id)),
            &row,
            32 * 1024,
        )?;
        rows.push(row);
        if (index + 1) % 25 == 0 {
            eprintln!("evaluation progress {}/{}", index + 1, plan.attempts.len());
        }
    }
    if crate::source_scope(vec![]).snapshot()? != snapshot {
        stop_reason = Some("source/executable drift during evaluation".into());
    }
    let mut summary = summarize_results(
        &plan,
        &bound.sha256,
        &rows,
        stop_reason,
        started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
        used,
    )?;
    if summary.stop_reason.is_some() {
        summary.complete = false;
    }
    let mut sealed_rows = vec![];
    for attempt in &plan.attempts {
        sealed_rows.push(bind_file(
            &destination
                .join("records")
                .join(format!("{}.json", attempt.id)),
            32 * 1024,
        )?);
    }
    let before_closeout = tree_bytes(&destination)?;
    let mut completion = RunCompletionV1 {
        schema_version: 1,
        request_sha256: sha256(&json_bytes(&request)?),
        summary_sha256: String::new(),
        row_sha256: sealed_rows,
    };
    for _ in 0..8 {
        completion.summary_sha256 = sha256(&json_bytes(&summary)?);
        let final_bytes = before_closeout
            + json_bytes(&summary)?.len() as u64
            + json_bytes(&completion)?.len() as u64;
        if final_bytes == summary.bytes {
            break;
        }
        summary.bytes = final_bytes;
    }
    completion.summary_sha256 = sha256(&json_bytes(&summary)?);
    if summary.bytes > budget.max_output_bytes {
        return Err("closeout would exceed declared byte limit".into());
    }
    write_limited(&destination.join("summary.json"), &summary, 1024 * 1024)?;
    write_limited(
        &destination.join("completion.json"),
        &completion,
        metadata_reserve,
    )?;
    if tree_bytes(&destination)? > budget.max_output_bytes {
        return Err("result tree exceeded declared storage cap".into());
    }
    Ok(summary)
}

fn run_new_attempt(
    supervisor: &AttemptSupervisor<'_>,
    index: usize,
    used: &mut u64,
    stop: &mut Option<String>,
    reuse: bool,
) -> Result<AttemptRowV1, String> {
    match try_run_attempt(supervisor, index, used, stop, reuse) {
        Ok(row) => Ok(row),
        Err(reason) => {
            *stop = Some(reason.clone());
            Ok(empty_row(
                &supervisor.plan.attempts[index],
                AttemptStatusV1::Fault,
                Some(reason),
            ))
        }
    }
}

fn try_run_attempt(
    supervisor: &AttemptSupervisor<'_>,
    index: usize,
    used: &mut u64,
    stop: &mut Option<String>,
    reuse: bool,
) -> Result<AttemptRowV1, String> {
    let AttemptSupervisor {
        plan_root,
        destination,
        plan,
        plan_sha,
        started,
        metadata_reserve,
        continuation,
    } = *supervisor;
    let attempt = &plan.attempts[index];
    let budget = &plan.recipe.budget;
    if used
        .checked_add(budget.max_bundle_bytes)
        .and_then(|v| v.checked_add(metadata_reserve))
        .is_none_or(|v| v > budget.max_output_bytes)
        || available_bytes(destination)?
            < budget
                .min_free_bytes
                .saturating_add(budget.max_bundle_bytes)
    {
        *stop = Some("stage storage reservation/free-space cap reached".into());
        return Ok(empty_row(
            attempt,
            AttemptStatusV1::NotStarted,
            stop.clone(),
        ));
    }
    let output = destination.join("attempts").join(&attempt.id);
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command
        .arg("execute-prepared-attempt")
        .arg("--plan")
        .arg(plan_root)
        .arg("--index")
        .arg(index.to_string())
        .arg("--plan-sha256")
        .arg(plan_sha)
        .arg("--output")
        .arg(&output);
    if reuse {
        command
            .arg("--reuse-from")
            .arg(continuation.ok_or("reuse lacks continuation root")?);
    }
    let worker_started = Instant::now();
    let mut child = OwnedChild(
        command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?,
    );
    let (status, interrupted) = wait_supervised_child(
        &mut child.0,
        started,
        budget.max_wall_seconds,
        budget.max_attempt_wall_seconds,
    )?;
    let timed = |mut row: AttemptRowV1| {
        row.supervised_worker_wall_time_us = Some(
            worker_started
                .elapsed()
                .as_micros()
                .min(u128::from(u64::MAX)) as u64,
        );
        row
    };
    if output.exists() {
        *used = used
            .checked_add(tree_bytes(&output)?)
            .ok_or("result size overflow")?;
    }
    if interrupted {
        *stop = Some("attempt interrupted at wall-time cap; evidence retained".into());
        return Ok(timed(empty_row(
            attempt,
            AttemptStatusV1::Interrupted,
            stop.clone(),
        )));
    }
    let completed = output.join(if reuse { "reuse.json" } else { "result.json" });
    if !status.success() || !completed.is_file() {
        let reason = read_json::<serde_json::Value>(&output.join("error.json"), 64 * 1024)
            .ok()
            .and_then(|v| v.get("error").and_then(|s| s.as_str()).map(str::to_owned))
            .unwrap_or_else(|| format!("worker exited {status} without completed result"));
        *stop = Some(reason.clone());
        return Ok(timed(empty_row(
            attempt,
            AttemptStatusV1::Fault,
            Some(reason),
        )));
    }
    if reuse {
        let row: AttemptRowV1 = read_json(&completed, 32 * 1024)?;
        if row.attempt_id != attempt.id
            || row.case_index != attempt.case_index
            || row.program_index != attempt.program_index
            || row.status != AttemptStatusV1::ReusedVerified
        {
            *stop = Some("reuse worker returned inconsistent row".into());
            return Ok(timed(empty_row(
                attempt,
                AttemptStatusV1::Fault,
                stop.clone(),
            )));
        }
        return Ok(timed(row));
    }
    let bound = bind_file(&output.join("result.json"), 1024 * 1024)?;
    let result = read_attempt_result(&bound)?;
    if !result.verification.passed()
        || !result.outer_integrity
        || result.reference.as_ref().is_some_and(|r| !r.passed)
    {
        *stop = Some("attempt proof, outer integrity or direct parity failed".into());
        let mut row = empty_row(attempt, AttemptStatusV1::Fault, stop.clone());
        row.result = Some(bound);
        return Ok(timed(row));
    }
    Ok(AttemptRowV1 {
        schema_version: 1,
        attempt_id: attempt.id.clone(),
        case_index: attempt.case_index,
        program_index: attempt.program_index,
        status: AttemptStatusV1::Verified,
        reason: None,
        result: Some(bound),
        reused_from: None,
        reuse_resources: None,
        supervised_worker_wall_time_us: Some(
            worker_started
                .elapsed()
                .as_micros()
                .min(u128::from(u64::MAX)) as u64,
        ),
    })
}

pub(super) fn wait_supervised_child(
    child: &mut std::process::Child,
    started: Instant,
    stage_seconds: u64,
    attempt_seconds: u64,
) -> Result<(std::process::ExitStatus, bool), String> {
    let attempt_started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Ok((status, false));
        }
        if started.elapsed().as_secs() >= stage_seconds
            || attempt_started.elapsed().as_secs() >= attempt_seconds
        {
            child.kill().map_err(|e| e.to_string())?;
            return Ok((child.wait().map_err(|e| e.to_string())?, true));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub fn check_result_inventory(
    plan_root: &Path,
    run_root: &Path,
) -> Result<EvaluationSummaryV1, String> {
    let plan = load_plan(plan_root)?;
    let request: RunRequestV1 = read_json(&run_root.join("request.json"), 1024 * 1024)?;
    let completion: RunCompletionV1 =
        read_json(&run_root.join("completion.json"), 4 * 1024 * 1024)?;
    if completion.schema_version != 1
        || completion.request_sha256
            != bind_file(&run_root.join("request.json"), 1024 * 1024)?.sha256
        || request.plan_sha256 != bind_file(&plan_root.join("plan.json"), 32 * 1024 * 1024)?.sha256
        || completion.row_sha256.len() != plan.attempts.len()
        || bind_file(&run_root.join("source.json"), 4 * 1024 * 1024)?.sha256
            != request.source_snapshot_sha256
    {
        return Err("result completion/source/request mismatch".into());
    }
    let mut rows = vec![];
    for (bound, attempt) in completion.row_sha256.iter().zip(&plan.attempts) {
        let expected = fs::canonicalize(
            run_root
                .join("records")
                .join(format!("{}.json", attempt.id)),
        )
        .map_err(|e| e.to_string())?;
        if expected != Path::new(&bound.path) {
            return Err("result row outside declared run".into());
        }
        let row: AttemptRowV1 = read_bound_json(bound, 32 * 1024)?;
        if let Some(result) = &row.result {
            check_attempt_payload_bytes(&read_attempt_result(result)?)?;
        }
        rows.push(row);
    }
    let summary: EvaluationSummaryV1 = read_json(&run_root.join("summary.json"), 1024 * 1024)?;
    if bind_file(&run_root.join("summary.json"), 1024 * 1024)?.sha256 != completion.summary_sha256 {
        return Err("result summary bytes changed".into());
    }
    let expected = summarize_results(
        &plan,
        &request.plan_sha256,
        &rows,
        summary.stop_reason.clone(),
        summary.elapsed_wall_time_us,
        summary.bytes,
    )?;
    if identity(&expected)? != identity(&summary)? {
        return Err("result summary differs from complete row inventory".into());
    }
    Ok(summary)
}
