use super::*;
use crate::{
    trace::CountersV1,
    verify::{ProofResourcesV1, VerificationReportV1},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectParityV1 {
    pub passed: bool,
    pub reference_physics_steps: u64,
    pub reference_backend_updates: u64,
    pub reference_wall_time_us: u64,
    pub behavior_sha256: String,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptResultV1 {
    pub schema_version: u32,
    pub attempt_id: String,
    pub plan_sha256: String,
    pub case_id: String,
    pub physical_sha256: String,
    pub program_sha256: String,
    pub verification: VerificationReportV1,
    pub outer_integrity: bool,
    pub native: pd_run::EpisodeSummaryV1,
    pub verified_target_landing: bool,
    pub actual_activation_tick: Option<u64>,
    pub original: CountersV1,
    pub original_wall_time_us: u64,
    pub preparation_resources: ProofResourcesV1,
    pub reference: Option<DirectParityV1>,
    pub worker_wall_time_us: u64,
    pub bundle_bytes: u64,
    pub bundle_receipt: BoundFileV1,
    pub thread_cpu_time_us: Option<u64>,
    pub internal_forecast_work: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatusV1 {
    Verified,
    ReusedVerified,
    InputRejected,
    Interrupted,
    Fault,
    NotStarted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptRowV1 {
    pub schema_version: u32,
    pub attempt_id: String,
    pub case_index: usize,
    pub program_index: usize,
    pub status: AttemptStatusV1,
    pub reason: Option<String>,
    pub result: Option<BoundFileV1>,
    pub reused_from: Option<String>,
    pub reuse_resources: Option<ProofResourcesV1>,
    pub supervised_worker_wall_time_us: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRequestV1 {
    pub schema_version: u32,
    pub plan_root: String,
    pub plan_sha256: String,
    pub executable_sha256: String,
    pub source_files_sha256: String,
    pub source_snapshot_sha256: String,
    pub continuation_request: Option<BoundFileV1>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkTotalsV1 {
    pub original_physics_steps: u64,
    pub original_backend_updates: u64,
    pub original_planner_calls: u64,
    pub reference_physics_steps: u64,
    pub reference_backend_updates: u64,
    pub proof_resources: ProofResourcesV1,
    pub original_wall_time_us: u64,
    pub worker_wall_time_us: u64,
    pub source_unavailable_original_counts: usize,
    pub unavailable_proof_work_attempts: usize,
}
pub fn add_proof(total: &mut ProofResourcesV1, value: &ProofResourcesV1) {
    total.neutral_replay_passes_started += value.neutral_replay_passes_started;
    total.controller_reconstruction_passes_started +=
        value.controller_reconstruction_passes_started;
    total.planner_reconstruction_passes_started += value.planner_reconstruction_passes_started;
    total.known_neutral_physics_steps += value.known_neutral_physics_steps;
    total.neutral_passes_without_step_count += value.neutral_passes_without_step_count;
    total.reconstruction_physics_steps += value.reconstruction_physics_steps;
    total.reconstruction_backend_updates += value.reconstruction_backend_updates;
    total.reconstruction_planner_calls += value.reconstruction_planner_calls;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultGroupV1 {
    pub scheduled: usize,
    pub verified: usize,
    pub landings: usize,
    pub input_rejected: usize,
    pub missing: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationAssessmentV1 {
    pub planned_n: usize,
    pub observed_landings: usize,
    pub complete: bool,
    pub criterion: ConfirmationCriterionV1,
    pub lower_bound: Option<f64>,
    pub meets_required_reliability: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationSummaryV1 {
    pub schema_version: u32,
    pub plan_sha256: String,
    pub complete: bool,
    pub stop_reason: Option<String>,
    pub statuses: BTreeMap<String, usize>,
    pub by_role_stratum: BTreeMap<String, ResultGroupV1>,
    pub physical_outcomes: BTreeMap<String, usize>,
    pub work_this_run: WorkTotalsV1,
    pub confirmation: Option<ConfirmationAssessmentV1>,
    pub elapsed_wall_time_us: u64,
    pub bytes: u64,
}

pub fn tree_bytes(path: &Path) -> Result<u64, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.is_symlink() {
        return Err("symlink in result tree".into());
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Err("nonregular result artifact".into());
    }
    let mut total = 0_u64;
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        total = total
            .checked_add(tree_bytes(&entry.map_err(|e| e.to_string())?.path())?)
            .ok_or("result size overflow")?;
    }
    Ok(total)
}
pub fn write_limited(path: &Path, value: &impl Serialize, max: u64) -> Result<u64, String> {
    let bytes = pd_missions::json_bytes(value)?;
    if bytes.len() as u64 > max {
        return Err("serialized artifact exceeds reservation".into());
    }
    pd_missions::write_json_create_only(path, value)?;
    Ok(bytes.len() as u64)
}

pub fn read_attempt_result(bound: &BoundFileV1) -> Result<AttemptResultV1, String> {
    let result: AttemptResultV1 = read_bound_json(bound, 1024 * 1024)?;
    if result.schema_version != 1 {
        return Err("unknown attempt result schema".into());
    }
    let receipt: crate::verify::BundleReceiptV1 =
        read_bound_json(&result.bundle_receipt, 1024 * 1024)?;
    let parent = Path::new(&bound.path)
        .parent()
        .ok_or("result has no parent")?;
    if Path::new(&result.bundle_receipt.path) != parent.join("bundle/receipt.json")
        || receipt.physical_sha256 != result.physical_sha256
    {
        return Err("attempt bundle receipt binding differs".into());
    }
    let expected_landing = result.outer_integrity
        && result.verification.passed()
        && result.verification.neutral_verified_target_landing
        && result.reference.as_ref().is_none_or(|r| r.passed);
    if expected_landing != result.verified_target_landing {
        return Err("attempt landing assertion differs from proof tuple".into());
    }
    Ok(result)
}

pub fn check_attempt_payload_bytes(result: &AttemptResultV1) -> Result<(), String> {
    let receipt: crate::verify::BundleReceiptV1 =
        read_bound_json(&result.bundle_receipt, 1024 * 1024)?;
    let root = Path::new(&result.bundle_receipt.path)
        .parent()
        .ok_or("receipt lacks bundle root")?;
    for (file, digest, max) in [
        (
            "capture/evaluation.json",
            receipt.evaluation_sha256,
            64 * 1024 * 1024,
        ),
        ("capture/seal.json", receipt.seal_sha256, 1024 * 1024),
        ("trace.json", receipt.trace_sha256, 32 * 1024 * 1024),
    ] {
        if bind_file(&root.join(file), max)?.sha256 != digest {
            return Err("attempt payload bytes changed after verification".into());
        }
    }
    Ok(())
}

fn add_result_work(total: &mut WorkTotalsV1, result: &AttemptResultV1) {
    total.original_physics_steps += result.original.physics_steps;
    total.original_backend_updates += result.original.backend_updates;
    total.original_planner_calls += result.original.planner_calls;
    total.original_wall_time_us += result.original_wall_time_us;
    if let Some(reference) = &result.reference {
        total.reference_physics_steps += reference.reference_physics_steps;
        total.reference_backend_updates += reference.reference_backend_updates;
    }
    add_proof(&mut total.proof_resources, &result.preparation_resources);
    add_proof(&mut total.proof_resources, &result.verification.resources);
}

pub fn summarize_results(
    plan: &PreparedEvaluationV1,
    plan_sha: &str,
    rows: &[AttemptRowV1],
    stop_reason: Option<String>,
    elapsed: u64,
    bytes: u64,
) -> Result<EvaluationSummaryV1, String> {
    if rows.len() != plan.attempts.len() {
        return Err("incomplete row inventory".into());
    }
    let initially_complete = stop_reason.is_none();
    let mut summary = EvaluationSummaryV1 {
        schema_version: 1,
        plan_sha256: plan_sha.into(),
        complete: initially_complete,
        stop_reason,
        statuses: BTreeMap::new(),
        by_role_stratum: BTreeMap::new(),
        physical_outcomes: BTreeMap::new(),
        work_this_run: WorkTotalsV1::default(),
        confirmation: None,
        elapsed_wall_time_us: elapsed,
        bytes,
    };
    let mut landed = 0;
    for (row, attempt) in rows.iter().zip(&plan.attempts) {
        if let Some(wall) = row.supervised_worker_wall_time_us {
            summary.work_this_run.worker_wall_time_us += wall;
        }
        if row.attempt_id != attempt.id
            || row.case_index != attempt.case_index
            || row.program_index != attempt.program_index
        {
            return Err("result row schedule mismatch".into());
        }
        let case = &plan.cases[attempt.case_index];
        let status = serde_json::to_value(&row.status)
            .map_err(|e| e.to_string())?
            .as_str()
            .unwrap()
            .to_owned();
        *summary.statuses.entry(status).or_default() += 1;
        let group = summary
            .by_role_stratum
            .entry(format!("{}:{}", case.role, case.stratum))
            .or_default();
        group.scheduled += 1;
        match row.status {
            AttemptStatusV1::Verified | AttemptStatusV1::ReusedVerified => {
                let result =
                    read_attempt_result(row.result.as_ref().ok_or("verified row missing result")?)?;
                if result.attempt_id != attempt.id
                    || result.plan_sha256 != plan_sha
                    || result.physical_sha256 != case.physical_sha256
                    || result.program_sha256 != attempt.program_sha256
                    || !result.verification.passed()
                    || !result.outer_integrity
                    || result.reference.as_ref().is_some_and(|r| !r.passed)
                    || (plan.recipe.direct_parity && result.reference.is_none())
                {
                    return Err("verified result binding/proof mismatch".into());
                }
                group.verified += 1;
                if result.verified_target_landing {
                    group.landings += 1;
                    landed += 1;
                }
                *summary
                    .physical_outcomes
                    .entry(
                        serde_json::to_value(&result.native.manifest.physical_outcome)
                            .map_err(|e| e.to_string())?
                            .as_str()
                            .unwrap()
                            .into(),
                    )
                    .or_default() += 1;
                if row.status == AttemptStatusV1::Verified {
                    add_result_work(&mut summary.work_this_run, &result);
                } else if let Some(resources) = &row.reuse_resources {
                    add_proof(&mut summary.work_this_run.proof_resources, resources);
                }
            }
            AttemptStatusV1::InputRejected => {
                group.input_rejected += 1;
                if case.disposition == InputDispositionV1::Supported {
                    return Err("supported case falsely rejected".into());
                }
            }
            _ => {
                summary.complete = false;
                group.missing += 1;
                if matches!(
                    row.status,
                    AttemptStatusV1::Interrupted | AttemptStatusV1::Fault
                ) {
                    if let Some(bound) = &row.result {
                        add_result_work(&mut summary.work_this_run, &read_attempt_result(bound)?);
                    } else {
                        summary.work_this_run.source_unavailable_original_counts += 1;
                        summary.work_this_run.unavailable_proof_work_attempts += 1;
                    }
                }
            }
        }
    }
    if matches!(
        plan.recipe.population,
        PopulationV1::Local {
            independent: true,
            ..
        }
    ) {
        let complete = summary.complete
            && plan.recipe.programs.len() == 1
            && rows.iter().all(|r| {
                matches!(
                    r.status,
                    AttemptStatusV1::Verified | AttemptStatusV1::ReusedVerified
                )
            });
        let criterion = plan
            .recipe
            .confirmation
            .clone()
            .ok_or("missing frozen confirmation criterion")?;
        let lower = complete
            .then(|| one_sided_binomial_lower(landed, plan.cases.len(), criterion.significance));
        let meets = lower.map(|p| p >= criterion.minimum_reliability);
        summary.confirmation = Some(ConfirmationAssessmentV1 {
            planned_n: plan.cases.len(),
            observed_landings: landed,
            complete,
            criterion,
            lower_bound: lower,
            meets_required_reliability: meets,
        });
    }
    Ok(summary)
}

/// Exact binomial upper tail inverted for a one-sided lower confidence bound.
/// Log-sum-exp avoids endpoint underflow and does not use normal approximations.
pub fn one_sided_binomial_lower(successes: usize, n: usize, alpha: f64) -> f64 {
    if successes == 0 || n == 0 || successes > n || !(0.0..1.0).contains(&alpha) {
        return 0.0;
    }
    let log_choose = (1..=successes)
        .map(|i| ((n + 1 - i) as f64).ln() - (i as f64).ln())
        .sum::<f64>();
    let tail = |p: f64| {
        let mut log_term =
            log_choose + (successes as f64) * p.ln() + ((n - successes) as f64) * (-p).ln_1p();
        let mut maximum = log_term;
        let mut logs = vec![log_term];
        for k in successes..n {
            log_term += ((n - k) as f64).ln() - ((k + 1) as f64).ln() + p.ln() - (-p).ln_1p();
            maximum = maximum.max(log_term);
            logs.push(log_term);
        }
        maximum + logs.iter().map(|v| (*v - maximum).exp()).sum::<f64>().ln()
    };
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..64 {
        let mid = (lo + hi) * 0.5;
        if tail(mid) < alpha.ln() {
            lo = mid
        } else {
            hi = mid
        }
    }
    (lo + hi) * 0.5
}
