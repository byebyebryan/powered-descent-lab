use super::*;
use crate::{
    IMPLEMENTATION, evaluate_files_with_inputs,
    verify::{self, BundleReceiptV1, ProofResourcesV1},
};
use pd_control::{Controller, TerminalPdgController};
use pd_core::RunContext;
use pd_eval::{EpisodeCaptureSealV1, EpisodeEvaluationV1};
use pd_missions::json_bytes;
use pd_run::{CaptureModeV1, Episode};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

fn elapsed_us(start: Instant) -> u64 {
    start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

/// Upper bound includes exact evaluation/trace/seal and the largest decimal
/// counters in the unchanged companion receipt. No neutral schema is changed.
fn bundle_reservation(
    evaluation: &EpisodeEvaluationV1,
    trace: &crate::trace::ManeuverTraceV1,
) -> Result<u64, String> {
    let eval_size = json_bytes(evaluation)?.len() as u64;
    let trace_size = json_bytes(trace)?.len() as u64;
    if eval_size > 64 * 1024 * 1024 || trace_size > 32 * 1024 * 1024 {
        return Err("capture exceeds existing bounded-loader limits".into());
    }
    let seal = EpisodeCaptureSealV1 {
        schema_version: 1,
        evaluation_sha256: "0".repeat(64),
    };
    let maximum = ProofResourcesV1 {
        neutral_replay_passes_started: u64::MAX,
        controller_reconstruction_passes_started: u64::MAX,
        planner_reconstruction_passes_started: u64::MAX,
        known_neutral_physics_steps: u64::MAX,
        neutral_passes_without_step_count: u64::MAX,
        reconstruction_physics_steps: u64::MAX,
        reconstruction_backend_updates: u64::MAX,
        reconstruction_planner_calls: u64::MAX,
    };
    let receipt = BundleReceiptV1 {
        schema_version: 1,
        descriptor: evaluation.execution.descriptor.clone(),
        physical_sha256: evaluation.physical_sha256.clone(),
        input_files: evaluation.evidence_before.input_files.clone(),
        evaluation_sha256: "0".repeat(64),
        seal_sha256: "0".repeat(64),
        trace_sha256: "0".repeat(64),
        preparation_resources: maximum,
    };
    Ok(eval_size
        + trace_size
        + json_bytes(&seal)?.len() as u64
        + json_bytes(&receipt)?.len() as u64)
}

fn direct_reference(
    scenario: &pd_core::ScenarioSpec,
    evaluation: &EpisodeEvaluationV1,
) -> Result<(DirectParityV1, pd_run::EpisodeResultV1), String> {
    let start = Instant::now();
    let ctx = RunContext::from_scenario(scenario)?;
    let mut controller = TerminalPdgController::default();
    controller.reset(&ctx);
    let mut episode = Episode::reset(
        scenario,
        IMPLEMENTATION,
        evaluation.execution.episode.limits,
        CaptureModeV1::Captured,
    )
    .map_err(|e| e.to_string())?;
    while episode.stop().is_none() {
        let frame = controller.update(&ctx, &episode.observation());
        episode.step_frame(frame, None).map_err(|e| e.to_string())?;
    }
    let reference = episode.finish().map_err(|e| e.to_string())?;
    let mut expected = evaluation.execution.controller_updates.clone();
    for update in &mut expected {
        update.compute_time_us = None;
    }
    let capture = reference
        .capture
        .as_ref()
        .ok_or("reference capture absent")?;
    let updates = reference
        .agent_updates
        .as_ref()
        .ok_or("reference update capture absent")?;
    let passed = *capture == evaluation.execution.episode && *updates == expected;
    let receipt = DirectParityV1 {
        passed,
        reference_physics_steps: reference.summary.final_state.physics_step,
        reference_backend_updates: reference.summary.manifest.controller_updates,
        reference_wall_time_us: elapsed_us(start),
        behavior_sha256: identity(&(capture, updates))?,
        detail: if passed {
            "Exact native artifacts and full frames; only per-update compute timing excluded"
        } else {
            "Native artifact or full-frame mismatch"
        }
        .into(),
    };
    Ok((receipt, reference))
}

pub fn execute_prepared_attempt(
    plan_root: &Path,
    index: usize,
    expected_plan_sha: &str,
    output: &Path,
) -> Result<AttemptResultV1, String> {
    let started = Instant::now();
    let plan_root = fs::canonicalize(plan_root).map_err(|e| e.to_string())?;
    let plan_bound = bind_file(&plan_root.join("plan.json"), 32 * 1024 * 1024)?;
    if plan_bound.sha256 != expected_plan_sha {
        return Err("worker plan differs from supervisor declaration".into());
    }
    let plan = load_plan(&plan_root)?;
    let attempt = plan
        .attempts
        .get(index)
        .ok_or("attempt index out of range")?;
    let case = &plan.cases[attempt.case_index];
    if case.disposition != InputDispositionV1::Supported {
        return Err("worker cannot execute unsupported input".into());
    }
    if attempt.id != attempt_identity(&plan, case, attempt.program_index)? {
        return Err("worker attempt identity differs".into());
    }
    let scenario = load_plan_case(&plan_root, &plan, attempt.case_index)?;
    let controller = pd_control::maneuver::TerminalLandingController::new(&scenario)
        .map_err(|e| format!("{e:?}"))?;
    use pd_control::maneuver::ManeuverController;
    if Some(controller.descriptor()) != plan.controller {
        return Err("frozen controller descriptor mismatch".into());
    }
    let program_path = plan_root.join(program_file(attempt.program_index));
    let program: crate::ActivationProgramV1 = read_json(&program_path, 64 * 1024)?;
    if program != plan.recipe.programs[attempt.program_index]
        || bind_file(&program_path, 64 * 1024)?.sha256 != attempt.program_sha256
    {
        return Err("prepared program changed".into());
    }
    let mut additional = vec![];
    for name in ["plan.json", "recipe.json", "audit.json", "completion.json"] {
        additional.push(bind_file(&plan_root.join(name), 32 * 1024 * 1024)?);
    }
    additional.extend(plan.input_files.clone());
    additional.push(plan.recipe_file.clone());
    additional.extend(plan.recipe.exposure_inventories.clone());
    for file in &additional {
        if bind_file(Path::new(&file.path), 32 * 1024 * 1024)? != *file {
            return Err("outer input changed before execution".into());
        }
    }
    fs::create_dir(output).map_err(|e| format!("reserve new attempt: {e}"))?;
    write_limited(
        &output.join("request.json"),
        &serde_json::json!({"schema_version":1,"plan_sha256":expected_plan_sha,"attempt_id":attempt.id,"attempt_index":index}),
        64 * 1024,
    )?;
    let additional_paths = additional
        .iter()
        .map(|b| PathBuf::from(&b.path))
        .collect::<Vec<_>>();
    let (evaluation, trace) = evaluate_files_with_inputs(
        &plan_root.join(&case.scenario_file),
        &program_path,
        &additional_paths,
    )?;
    let outer_integrity = evaluation.physical_sha256 == case.physical_sha256
        && additional.iter().all(|expected| {
            evaluation
                .evidence_before
                .input_files
                .iter()
                .any(|actual| actual.path == expected.path && actual.sha256 == expected.sha256)
        });
    let reservation = bundle_reservation(&evaluation, &trace)?;
    let maximum = plan.recipe.budget.max_bundle_bytes;
    if reservation
        .checked_add(2 * 1024 * 1024)
        .ok_or("bundle size overflow")?
        > maximum
    {
        return Err("capture cannot fit its declared storage reservation".into());
    }
    verify::save_bundle(&output.join("bundle"), &evaluation, &trace)?;
    let receipt: BundleReceiptV1 = read_json(&output.join("bundle/receipt.json"), 1024 * 1024)?;
    let verified = verify::verify_bundle(&output.join("bundle"));
    let reference = if plan.recipe.direct_parity {
        let (receipt, reference) = direct_reference(&scenario, &evaluation)?;
        if !receipt.passed {
            let available = maximum.saturating_sub(tree_bytes(output)? + 1024 * 1024);
            write_limited(
                &output.join("mismatched-reference.json"),
                &reference,
                available,
            )?;
        }
        Some(receipt)
    } else {
        None
    };
    let capture = &evaluation.execution.episode;
    let native = pd_run::EpisodeSummaryV1 {
        schema_version: 1,
        limits: capture.limits,
        stop: capture.stop,
        final_state: capture.final_state.clone(),
        incoming_contact: capture.incoming_contact.clone(),
        failure: capture.failure.clone(),
        manifest: capture.run.manifest.clone(),
    };
    let bytes = tree_bytes(output)?;
    let result = AttemptResultV1 {
        schema_version: 1,
        attempt_id: attempt.id.clone(),
        plan_sha256: expected_plan_sha.into(),
        case_id: case.id.clone(),
        physical_sha256: case.physical_sha256.clone(),
        program_sha256: attempt.program_sha256.clone(),
        verified_target_landing: outer_integrity
            && evaluation.verified_target_landing
            && verified.report.passed()
            && reference.as_ref().is_none_or(|r| r.passed),
        verification: verified.report,
        outer_integrity,
        native,
        actual_activation_tick: trace
            .final_feedback
            .accepted
            .as_ref()
            .map(|a| a.activation_physics_step),
        original: trace.counters,
        original_wall_time_us: evaluation.execution.performance.wall_time_us,
        preparation_resources: receipt.preparation_resources,
        reference,
        worker_wall_time_us: elapsed_us(started),
        bundle_bytes: tree_bytes(&output.join("bundle"))?,
        bundle_receipt: bind_file(&output.join("bundle/receipt.json"), 1024 * 1024)?,
        thread_cpu_time_us: None,
        internal_forecast_work: None,
    };
    write_limited(
        &output.join("result.json"),
        &result,
        maximum
            .checked_sub(bytes)
            .ok_or("attempt exceeded byte reservation")?,
    )?;
    Ok(result)
}

pub fn execute_reuse_attempt(
    plan_root: &Path,
    index: usize,
    expected_plan_sha: &str,
    previous: &Path,
    output: &Path,
) -> Result<AttemptRowV1, String> {
    let plan = load_plan(plan_root)?;
    if bind_file(&plan_root.join("plan.json"), 32 * 1024 * 1024)?.sha256 != expected_plan_sha {
        return Err("reuse worker plan changed".into());
    }
    let attempt = plan.attempts.get(index).ok_or("reuse index out of range")?;
    let current = output
        .parent()
        .and_then(Path::parent)
        .ok_or("reuse result needs run parent")?;
    let request: RunRequestV1 = read_json(&current.join("request.json"), 1024 * 1024)?;
    let snapshot = crate::source_scope(vec![]).snapshot()?;
    crate::check_compiled_sources(&snapshot)?;
    if request.executable_sha256 != snapshot.executable_sha256
        || request.source_files_sha256 != identity(&snapshot.source_files)?
        || request.plan_sha256 != expected_plan_sha
    {
        return Err("reuse source/executable mismatch".into());
    }
    let rows = super::recovery::continuation_rows(previous, &request)?;
    let old = rows.get(&attempt.id).ok_or("prior completed row missing")?;
    fs::create_dir(output).map_err(|e| e.to_string())?;
    let row = super::recovery::reuse_row(
        old,
        attempt,
        &plan.cases[attempt.case_index],
        expected_plan_sha,
    )?
    .ok_or("prior attempt not reusable")?;
    write_limited(&output.join("reuse.json"), &row, 32 * 1024)?;
    Ok(row)
}
