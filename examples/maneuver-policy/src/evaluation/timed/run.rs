use super::{TimedEvaluationPlanV2, check_timed_evaluation};
use crate::{
    evaluation::{
        BoundFileV1, available_bytes, bind_file, identity, one_sided_binomial_lower,
        read_bound_json, read_json, tree_bytes, write_limited,
    },
    poly::{
        TimedProgramV2, TimedTraceV2,
        verify::{TimedProofResourcesV2, TimedVerificationReportV2},
    },
};
use pd_core::{ScenarioSpec, Vec2};
use pd_eval::EpisodeEvaluationV1;
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    process::{Command, Stdio},
    time::Instant,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BurnShapeV2 {
    pub passed: bool,
    pub violations: Vec<String>,
    pub native_samples: u64,
    pub maximum_flying_vertical_speed_mps: f64,
    pub emitted_engine_off_commands: u64,
    pub actual_unpowered_physics_steps: u64,
    pub minimum_actual_thrust_fraction: Option<f64>,
    pub maximum_actual_thrust_fraction: Option<f64>,
    pub contact_time_residual_ticks: Option<i64>,
    pub fuel_used_kg: f64,
    pub fuel_remaining_kg: f64,
}

pub fn assess_burn_shape(
    evaluation: &EpisodeEvaluationV1,
    trace: &TimedTraceV2,
    p: &TimedProgramV2,
) -> BurnShapeV2 {
    let episode = &evaluation.execution.episode;
    let samples = &episode.run.samples;
    let accepted = trace.feedback.accepted.as_ref();
    let mut violations = Vec::new();
    if accepted.is_none() {
        violations.push("no accepted finite burn".into());
    }
    if trace.planner_requests != 1 || trace.feedback.constructions != 1 {
        violations.push("request/construction count differs from one".into());
    }
    if accepted.is_some_and(|a| {
        a.request != p.request || a.entry.physics_step != p.activation_physics_step
    }) {
        violations.push("reference/request clock binding differs".into());
    }
    if trace.feedback.reference_exhausted {
        violations.push("reference exhausted before native contact".into());
    }
    if samples.len() as u64 != episode.final_state.physics_step + 1
        || samples
            .iter()
            .enumerate()
            .any(|(i, s)| s.physics_step != i as u64)
    {
        violations.push("native shape observations are incomplete".into());
    }
    let engine_off = episode
        .run
        .actions
        .iter()
        .filter(|a| a.physics_step >= p.activation_physics_step && a.command.throttle_frac <= 0.0)
        .count() as u64;
    if engine_off > 0 {
        violations.push("engine-off command after activation".into());
    }
    let mut max_vy = f64::NEG_INFINITY;
    let mut minimum = f64::INFINITY;
    let mut maximum = f64::NEG_INFINITY;
    let mut unpowered = 0;
    for sample in samples
        .iter()
        .filter(|s| s.physics_step >= p.activation_physics_step)
    {
        if episode
            .incoming_contact
            .as_ref()
            .is_none_or(|contact| sample.physics_step != contact.state.physics_step)
        {
            max_vy = max_vy.max(sample.observation.velocity_mps.y);
        }
    }
    if let Some(contact) = &episode.incoming_contact {
        max_vy = max_vy.max(contact.state.velocity_mps.y);
    }
    if !max_vy.is_finite() {
        max_vy = 0.0;
    }
    if max_vy > 1.0e-9 {
        violations.push("positive flying vertical velocity".into());
    }
    for pair in samples.windows(2) {
        let before = &pair[0];
        let after = &pair[1];
        if before.physics_step < p.activation_physics_step {
            continue;
        }
        let (velocity, mass) = if let Some(contact) = &episode.incoming_contact {
            if after.physics_step == contact.state.physics_step {
                (
                    contact.state.velocity_mps,
                    evaluation.scenario.vehicle.dry_mass_kg + contact.state.fuel_kg,
                )
            } else {
                (after.observation.velocity_mps, after.observation.mass_kg)
            }
        } else {
            (after.observation.velocity_mps, after.observation.mass_kg)
        };
        let thrust = (velocity - before.observation.velocity_mps) * 120.0 + Vec2::new(0.0, 9.81);
        let applied = thrust.length() * mass / evaluation.scenario.vehicle.max_thrust_n;
        minimum = minimum.min(applied);
        maximum = maximum.max(applied);
        if applied <= 1.0e-9 {
            unpowered += 1;
        }
    }
    if unpowered > 0 {
        violations.push("actual unpowered native transition after activation".into());
    }
    if accepted.is_some()
        && trace
            .boundaries
            .iter()
            .any(|b| b.physics_step >= p.activation_physics_step && b.owner != "poly_landing")
    {
        violations.push("command owner changed after activation".into());
    }
    let end = match p.request.goal {
        pd_control::maneuver::ManeuverGoalV2::LandOnMissionTargetAt {
            reference_end_physics_step,
            ..
        } => Some(reference_end_physics_step),
        _ => None,
    };
    BurnShapeV2 {
        passed: violations.is_empty(),
        violations,
        native_samples: samples.len() as u64,
        maximum_flying_vertical_speed_mps: max_vy,
        emitted_engine_off_commands: engine_off,
        actual_unpowered_physics_steps: unpowered,
        minimum_actual_thrust_fraction: minimum.is_finite().then_some(minimum),
        maximum_actual_thrust_fraction: maximum.is_finite().then_some(maximum),
        contact_time_residual_ticks: episode
            .incoming_contact
            .as_ref()
            .and(end)
            .map(|end| episode.final_state.physics_step as i64 - end as i64),
        fuel_used_kg: evaluation.scenario.vehicle.initial_fuel_kg - episode.final_state.fuel_kg,
        fuel_remaining_kg: episode.final_state.fuel_kg,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedAttemptResultV2 {
    pub schema_version: u32,
    pub plan_sha256: String,
    pub case_index: usize,
    pub case_id: String,
    pub disposition: String,
    pub support: String,
    pub verified_target_landing: bool,
    pub shape: Option<BurnShapeV2>,
    pub original_physics_steps: u64,
    pub original_command_updates: u64,
    pub original_reference_constructions: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub original_admission_actuation_checks: u64,
    pub original_planner_requests: u64,
    pub preparation_resources: TimedProofResourcesV2,
    pub verification: Option<TimedVerificationReportV2>,
    pub payload_files: Vec<BoundFileV1>,
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

fn current_sources(plan: &TimedEvaluationPlanV2) -> Result<(), String> {
    let source = crate::source_scope(vec![]).snapshot()?;
    crate::check_compiled_sources(&source)?;
    if source.source_files != plan.source_files {
        return Err("timed prepared source epoch changed".into());
    }
    Ok(())
}

fn proof_valid(item: &TimedAttemptResultV2, role: &str) -> bool {
    match item.disposition.as_str() {
        "unsupported" => {
            role == "diagnostic" && item.verification.is_none() && item.original_physics_steps == 0
        }
        "executed" => item.verification.as_ref().is_some_and(|v| v.passed()),
        _ => false,
    }
}

pub fn execute_timed_attempt(
    plan_root: &Path,
    index: usize,
    plan_sha: &str,
    output: &Path,
) -> Result<TimedAttemptResultV2, String> {
    let plan = check_timed_evaluation(plan_root)?;
    current_sources(&plan)?;
    if bind_file(&plan_root.join("plan.json"), 32 * 1024 * 1024)?.sha256 != plan_sha {
        return Err("timed worker plan differs from declaration".into());
    }
    let case = plan
        .cases
        .get(index)
        .ok_or("timed case index out of range")?;
    let scenario: ScenarioSpec = read_bound_json(&case.scenario, 1024 * 1024)?;
    let program: TimedProgramV2 = read_bound_json(&case.program, 1024 * 1024)?;
    crate::verify::no_symlinks(output.parent().ok_or("timed attempt needs a parent")?)?;
    std::fs::create_dir(output).map_err(|e| e.to_string())?;
    let mut result = TimedAttemptResultV2 {
        schema_version: 2,
        plan_sha256: plan_sha.into(),
        case_index: index,
        case_id: case.id.clone(),
        disposition: "unsupported".into(),
        support: case.native_support.clone(),
        verified_target_landing: false,
        shape: None,
        original_physics_steps: 0,
        original_command_updates: 0,
        original_reference_constructions: 0,
        original_admission_actuation_checks: 0,
        original_planner_requests: 0,
        preparation_resources: TimedProofResourcesV2::default(),
        verification: None,
        payload_files: vec![],
    };
    if crate::poly::controller(&scenario, &program).is_ok() {
        if case.native_support != "supported" {
            return Err("prepared timed support changed".into());
        }
        let (evaluation, trace) = crate::poly::evaluate_files_with_inputs(
            Path::new(&case.scenario.path),
            Path::new(&case.program.path),
            &[
                plan_root.join("plan.json"),
                std::path::PathBuf::from(&plan.recipe_file.path),
            ],
        )?;
        crate::poly::verify::save_bundle(&output.join("bundle"), &evaluation, &trace)?;
        let verified = crate::poly::verify::verify_bundle(&output.join("bundle"));
        write_limited(
            &output.join("verification.json"),
            &verified.report,
            1024 * 1024,
        )?;
        let receipt: crate::poly::verify::TimedBundleReceiptV2 =
            read_json(&output.join("bundle/receipt.json"), 32 * 1024 * 1024)?;
        result.disposition = "executed".into();
        result.verified_target_landing =
            evaluation.verified_target_landing && verified.report.passed();
        result.shape = Some(assess_burn_shape(&evaluation, &trace, &program));
        result.original_physics_steps = trace.physics_steps;
        result.original_command_updates = trace.command_updates;
        result.original_reference_constructions = trace.feedback.constructions;
        result.original_admission_actuation_checks = trace.feedback.admission_actuation_checks;
        result.original_planner_requests = trace.planner_requests;
        result.preparation_resources = receipt.preparation_resources;
        result.verification = Some(verified.report);
        for name in [
            "bundle/capture/evaluation.json",
            "bundle/capture/seal.json",
            "bundle/trace.json",
            "bundle/receipt.json",
            "verification.json",
        ] {
            result
                .payload_files
                .push(bind_file(&output.join(name), 64 * 1024 * 1024)?);
        }
    } else if case.native_support == "supported" {
        return Err("prepared supported case became unsupported".into());
    }
    current_sources(&plan)?;
    write_limited(&output.join("result.json"), &result, 1024 * 1024)?;
    Ok(result)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedAttemptRowV2 {
    pub case_index: usize,
    pub case_id: String,
    pub status: String,
    pub reason: Option<String>,
    pub result: Option<BoundFileV1>,
    pub supervised_wall_seconds: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedEvaluationSummaryV2 {
    pub schema_version: u32,
    pub sampler: String,
    pub phase: String,
    pub plan_sha256: String,
    pub source_files: Vec<pd_eval::FileIdentityV1>,
    pub complete: bool,
    pub eligible_for_confirmation: bool,
    pub primary_originals: usize,
    pub verified_primary_landings: usize,
    pub primary_shape_failures: usize,
    pub verified_transform_landings: usize,
    pub transforms: usize,
    pub diagnostics: usize,
    pub diagnostic_verified_landings: usize,
    pub diagnostic_unsupported: usize,
    pub confirmation_lower_95: Option<f64>,
    pub qualified: bool,
    pub wall_seconds: f64,
    pub output_bytes: u64,
    pub stop_reason: Option<String>,
    pub original_physics_steps: u64,
    pub proof_resources: TimedProofResourcesV2,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub original_admission_actuation_checks: u64,
    pub rows: Vec<TimedAttemptRowV2>,
}

pub fn summarize_timed(
    plan: &TimedEvaluationPlanV2,
    plan_sha: String,
    rows: Vec<TimedAttemptRowV2>,
    wall_seconds: f64,
    bytes: u64,
    stop: Option<String>,
) -> Result<TimedEvaluationSummaryV2, String> {
    let mut result = TimedEvaluationSummaryV2 {
        schema_version: 2,
        sampler: super::SAMPLER.into(),
        phase: plan.recipe.phase.clone(),
        plan_sha256: plan_sha.clone(),
        source_files: plan.source_files.clone(),
        complete: stop.is_none() && rows.len() == plan.cases.len(),
        eligible_for_confirmation: false,
        primary_originals: plan.recipe.originals,
        verified_primary_landings: 0,
        primary_shape_failures: 0,
        verified_transform_landings: 0,
        transforms: 0,
        diagnostics: 0,
        diagnostic_verified_landings: 0,
        diagnostic_unsupported: 0,
        confirmation_lower_95: None,
        qualified: false,
        wall_seconds,
        output_bytes: bytes,
        stop_reason: stop,
        original_physics_steps: 0,
        proof_resources: TimedProofResourcesV2::default(),
        original_admission_actuation_checks: 0,
        rows,
    };
    for (index, case) in plan.cases.iter().enumerate() {
        if case.role == "transform" {
            result.transforms += 1;
        }
        if case.role == "diagnostic" {
            result.diagnostics += 1;
        }
        let Some(row) = result.rows.get(index) else {
            result.complete = false;
            continue;
        };
        if row.case_index != index || row.case_id != case.id {
            return Err("timed result row differs from scheduled slot".into());
        }
        let Some(bound) = &row.result else {
            result.complete = false;
            continue;
        };
        let item: TimedAttemptResultV2 = read_bound_json(bound, 1024 * 1024)?;
        if item.schema_version != 2
            || item.plan_sha256 != plan_sha
            || item.case_index != index
            || item.case_id != case.id
        {
            return Err("timed attempt binding differs".into());
        }
        for payload in &item.payload_files {
            let _ = read_bound_bytes(payload, 64 * 1024 * 1024)?;
        }
        let declared_landing = item
            .verification
            .as_ref()
            .is_some_and(|v| v.passed() && v.neutral_verified_target_landing);
        if item.verified_target_landing != declared_landing {
            return Err("timed landing projection lacks its complete proof tuple".into());
        }
        let valid = proof_valid(&item, &case.role);
        if !valid {
            result.complete = false;
        }
        let shape = item.shape.as_ref().is_some_and(|s| s.passed);
        match case.role.as_str() {
            "primary" => {
                result.verified_primary_landings += usize::from(item.verified_target_landing);
                result.primary_shape_failures += usize::from(!shape);
            }
            "transform" => {
                result.verified_transform_landings +=
                    usize::from(item.verified_target_landing && shape);
            }
            "diagnostic" => {
                result.diagnostic_verified_landings += usize::from(item.verified_target_landing);
                result.diagnostic_unsupported += usize::from(item.disposition == "unsupported");
            }
            _ => return Err("unknown timed role".into()),
        }
        result.original_physics_steps += item.original_physics_steps;
        result.original_admission_actuation_checks += item.original_admission_actuation_checks;
        add_proofs(&mut result.proof_resources, &item.preparation_resources);
        if let Some(verified) = item.verification {
            add_proofs(&mut result.proof_resources, &verified.resources);
        }
    }
    let pass = result.complete
        && result.verified_primary_landings == result.primary_originals
        && result.primary_shape_failures == 0
        && result.verified_transform_landings == result.transforms;
    result.eligible_for_confirmation =
        pass && plan.recipe.phase == "development" && plan.recipe.development_extensions;
    if plan.recipe.phase == "confirmation" && result.complete {
        let lower = one_sided_binomial_lower(
            result.verified_primary_landings,
            result.primary_originals,
            0.05,
        );
        result.confirmation_lower_95 = Some(lower);
        result.qualified = lower >= 0.99 && result.primary_shape_failures == 0;
    }
    Ok(result)
}

fn read_bound_bytes(file: &BoundFileV1, max: u64) -> Result<Vec<u8>, String> {
    let bytes = crate::verify::read_regular(Path::new(&file.path), max)?;
    if pd_missions::sha256(&bytes) != file.sha256 {
        return Err("timed payload bytes differ".into());
    }
    Ok(bytes)
}
fn add_proofs(a: &mut TimedProofResourcesV2, b: &TimedProofResourcesV2) {
    a.neutral_replay_passes_started += b.neutral_replay_passes_started;
    a.known_neutral_physics_steps += b.known_neutral_physics_steps;
    a.neutral_passes_without_step_count += b.neutral_passes_without_step_count;
    a.controller_reconstruction_passes_started += b.controller_reconstruction_passes_started;
    a.planner_reconstruction_passes_started += b.planner_reconstruction_passes_started;
    a.reconstruction_physics_steps += b.reconstruction_physics_steps;
    a.reconstruction_command_updates += b.reconstruction_command_updates;
    a.reconstruction_reference_constructions += b.reconstruction_reference_constructions;
    a.reconstruction_admission_actuation_checks += b.reconstruction_admission_actuation_checks;
    a.reconstruction_planner_requests += b.reconstruction_planner_requests;
}

pub fn run_timed_evaluation(
    plan_root: &Path,
    output: &Path,
) -> Result<TimedEvaluationSummaryV2, String> {
    run_timed_evaluation_continued(plan_root, output, None)
}

pub fn run_timed_evaluation_continued(
    plan_root: &Path,
    output: &Path,
    continuation: Option<&Path>,
) -> Result<TimedEvaluationSummaryV2, String> {
    let plan = check_timed_evaluation(plan_root)?;
    current_sources(&plan)?;
    let plan_sha = bind_file(&plan_root.join("plan.json"), 32 * 1024 * 1024)?.sha256;
    let executable = crate::source_scope(vec![]).snapshot()?.executable_sha256;
    if let Some(previous) = continuation {
        let request: serde_json::Value =
            read_json(&previous.join("run-request.json"), 1024 * 1024)?;
        if request["plan"]["sha256"] != plan_sha || request["executable_sha256"] != executable {
            return Err("timed continuation plan/executable epoch differs".into());
        }
    }
    crate::verify::no_symlinks(output.parent().ok_or("timed result needs a parent")?)?;
    std::fs::create_dir(output).map_err(|e| e.to_string())?;
    std::fs::create_dir(output.join("attempts")).map_err(|e| e.to_string())?;
    write_limited(
        &output.join("run-request.json"),
        &serde_json::json!({"schema_version":2,"plan":bind_file(&plan_root.join("plan.json"),32*1024*1024)?,
            "executable_sha256":executable,"native_workers":1,"continuation":continuation,
            "reuse":"bound complete results only; no new flight or replay; partial attempts require a prospective replacement allowance"}),
        1024 * 1024,
    )?;
    let started = Instant::now();
    let mut rows = Vec::new();
    let mut stop: Option<String> = None;
    for (index, case) in plan.cases.iter().enumerate() {
        let mut row = TimedAttemptRowV2 {
            case_index: index,
            case_id: case.id.clone(),
            status: "not_started".into(),
            reason: None,
            result: None,
            supervised_wall_seconds: None,
        };
        if stop.is_none()
            && let Some(previous) = continuation
        {
            let prior = previous.join("attempts").join(index.to_string());
            if prior.join("result.json").is_file() {
                let bound = bind_file(&prior.join("result.json"), 1024 * 1024)?;
                let item: TimedAttemptResultV2 = read_bound_json(&bound, 1024 * 1024)?;
                if item.plan_sha256 != plan_sha
                    || item.case_id != case.id
                    || item.case_index != index
                    || !proof_valid(&item, &case.role)
                {
                    return Err("prior timed result cannot be reused".into());
                }
                for payload in &item.payload_files {
                    let _ = read_bound_bytes(payload, 64 * 1024 * 1024)?;
                }
                row.status = "reused".into();
                row.result = Some(bound);
                write_limited(&output.join(format!("row-{index}.json")), &row, 1024 * 1024)?;
                rows.push(row);
                continue;
            } else if prior.exists() {
                stop = Some(
                    "prior partial timed attempt retained; no automatic original retry".into(),
                );
            }
        }
        if stop.is_none()
            && (started.elapsed().as_secs() >= plan.recipe.budget.max_wall_seconds
                || tree_bytes(output)?
                    .saturating_add(plan.recipe.budget.max_bundle_bytes)
                    .saturating_add(1024 * 1024)
                    > plan.recipe.budget.max_output_bytes
                || available_bytes(output)?
                    < plan
                        .recipe
                        .budget
                        .min_free_bytes
                        .saturating_add(plan.recipe.budget.max_bundle_bytes))
        {
            stop = Some("prospective timed stage wall/storage/free-space limit".into());
        }
        if let Some(reason) = &stop {
            row.reason = Some(reason.clone());
            rows.push(row);
            continue;
        }
        let destination = output.join("attempts").join(index.to_string());
        let worker_started = Instant::now();
        let log = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(format!("attempt-{index}.log")))
            .map_err(|e| e.to_string())?;
        let mut child = super::super::run::OwnedChild(
            Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
                .args(["execute-timed-attempt", "--plan"])
                .arg(plan_root)
                .arg("--index")
                .arg(index.to_string())
                .arg("--plan-sha256")
                .arg(&plan_sha)
                .arg("--output")
                .arg(&destination)
                .stdout(Stdio::from(log.try_clone().map_err(|e| e.to_string())?))
                .stderr(Stdio::from(log))
                .spawn()
                .map_err(|e| e.to_string())?,
        );
        let (status, interrupted) = super::super::run::wait_supervised_child(
            &mut child.0,
            started,
            plan.recipe.budget.max_wall_seconds,
            plan.recipe.budget.max_attempt_wall_seconds,
        )?;
        row.supervised_wall_seconds = Some(worker_started.elapsed().as_secs_f64());
        if interrupted || !status.success() || !destination.join("result.json").is_file() {
            row.status = if interrupted { "interrupted" } else { "fault" }.into();
            let reason = format!("timed worker incomplete: {status}; interrupted={interrupted}");
            row.reason = Some(reason.clone());
            stop = Some(reason);
        } else {
            let bound = bind_file(&destination.join("result.json"), 1024 * 1024)?;
            let item: TimedAttemptResultV2 = read_bound_json(&bound, 1024 * 1024)?;
            row.status = "verified".into();
            row.result = Some(bound);
            if !proof_valid(&item, &case.role)
                || tree_bytes(&destination)? > plan.recipe.budget.max_bundle_bytes
            {
                row.status = "fault".into();
                let reason = "timed attempt proof/support/storage fault".to_string();
                row.reason = Some(reason.clone());
                stop = Some(reason);
            }
        }
        write_limited(&output.join(format!("row-{index}.json")), &row, 1024 * 1024)?;
        rows.push(row);
    }
    if let Err(reason) = current_sources(&plan) {
        stop.get_or_insert(reason);
    }
    let summary = summarize_timed(
        &plan,
        plan_sha,
        rows,
        started.elapsed().as_secs_f64(),
        tree_bytes(output)?,
        stop,
    )?;
    write_limited(&output.join("summary.json"), &summary, 8 * 1024 * 1024)?;
    Ok(summary)
}

pub fn check_timed_results(
    plan_root: &Path,
    result_root: &Path,
) -> Result<TimedEvaluationSummaryV2, String> {
    let plan = check_timed_evaluation(plan_root)?;
    let saved: TimedEvaluationSummaryV2 =
        read_json(&result_root.join("summary.json"), 8 * 1024 * 1024)?;
    let sha = bind_file(&plan_root.join("plan.json"), 32 * 1024 * 1024)?.sha256;
    let actual = summarize_timed(
        &plan,
        sha,
        saved.rows.clone(),
        saved.wall_seconds,
        saved.output_bytes,
        saved.stop_reason.clone(),
    )?;
    if identity(&actual)? != identity(&saved)? {
        return Err("timed summary accounting differs".into());
    }
    Ok(actual)
}
