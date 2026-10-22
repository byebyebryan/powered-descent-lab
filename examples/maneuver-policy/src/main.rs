use clap::{Parser, Subcommand};
use pd_maneuver_example::{
    evaluate_files, source_scope,
    verify::{read_regular, save_bundle, verify_bundle},
};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(
    about = "Deterministic maneuver activation and bounded controller evaluation; no training"
)]
struct Args {
    #[command(subcommand)]
    operation: Operation,
}
#[derive(Subcommand)]
enum Operation {
    ReportTimedEvaluation {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    PrepareTimedEvaluation {
        #[arg(long)]
        recipe: PathBuf,
        #[arg(long)]
        anchor: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    RunTimedEvaluation {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        continue_from: Option<PathBuf>,
    },
    CheckTimedEvaluationResults {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        results: PathBuf,
    },
    #[command(hide = true)]
    ExecuteTimedAttempt {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        index: usize,
        #[arg(long)]
        plan_sha256: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Recover a uniquely matching saved burn duration and verify exact native handoff.
    RecoverSavedBurn {
        #[arg(long)]
        scenario: PathBuf,
        #[arg(long)]
        flight: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Timed Poly Landing v2; starts the burn at the exact declared boundary.
    RunTimed {
        #[arg(long)]
        scenario: PathBuf,
        #[arg(long)]
        program: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Verify a timed v2 companion plus independent ordinary native replay.
    VerifyTimed {
        bundle: PathBuf,
    },
    /// Render a timed run through the established common rich detail template.
    ReportTimed {
        bundle: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Execute a frozen schedule with supervised attempts and complete result rows.
    RunEvaluation {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        continue_from: Option<PathBuf>,
    },
    /// Validate saved result identities, seals and aggregate accounting without replay.
    CheckEvaluationResults {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        results: PathBuf,
    },
    #[command(hide = true)]
    ExecutePreparedAttempt {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        index: usize,
        #[arg(long)]
        plan_sha256: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        reuse_from: Option<PathBuf>,
    },
    /// Resolve an immutable evaluation schedule and coverage audit; no flights.
    PrepareEvaluation {
        #[arg(long)]
        recipe: PathBuf,
        #[arg(long, required = true)]
        inputs: Vec<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Check all prepared input identities, support, schedule and coverage read-only.
    CheckEvaluation {
        plan: PathBuf,
    },
    Run {
        #[arg(long)]
        scenario: PathBuf,
        #[arg(long)]
        program: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Verify {
        bundle: PathBuf,
    },
    Report {
        bundle: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

fn output_preflight(output: &Path, inputs: Vec<PathBuf>) -> Result<(), String> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = std::fs::canonicalize(parent).map_err(|e| e.to_string())?;
    let absolute = parent.join(output.file_name().ok_or("output needs a filename")?);
    if std::fs::symlink_metadata(&absolute).is_ok() {
        return Err("output already exists".into());
    }
    let scope = source_scope(inputs);
    for path in scope.sources.iter().chain(&scope.inputs) {
        let bound = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
        if absolute.starts_with(&bound) || bound.starts_with(&absolute) {
            return Err("generated destination overlaps bound sources/inputs".into());
        }
    }
    Ok(())
}

fn main() -> Result<(), String> {
    match Args::parse().operation {
        Operation::ReportTimedEvaluation { spec, output } => {
            output_preflight(&output, vec![spec.clone()])?;
            pd_maneuver_example::evaluation::timed::render_timed_evaluation_report(&spec, &output)?;
        }
        Operation::PrepareTimedEvaluation {
            recipe,
            anchor,
            output,
        } => {
            output_preflight(&output, vec![recipe.clone(), anchor.clone()])?;
            let plan = pd_maneuver_example::evaluation::timed::prepare_timed_evaluation(
                &recipe, &anchor, &output,
            )?;
            println!("{}",serde_json::to_string_pretty(&serde_json::json!({"cases":plan.cases.len(),"proposals":plan.proposals.len(),"coverage":plan.coverage})).map_err(|e|e.to_string())?);
        }
        Operation::RunTimedEvaluation {
            plan,
            output,
            continue_from,
        } => {
            let mut inputs = vec![plan.join("plan.json")];
            if let Some(previous) = &continue_from {
                inputs.push(previous.clone());
            }
            output_preflight(&output, inputs)?;
            let summary = pd_maneuver_example::evaluation::timed::run_timed_evaluation_continued(
                &plan,
                &output,
                continue_from.as_deref(),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?
            );
            if !summary.complete {
                return Err("timed evaluation incomplete; every scheduled slot retained".into());
            }
        }
        Operation::CheckTimedEvaluationResults { plan, results } => {
            let summary =
                pd_maneuver_example::evaluation::timed::check_timed_results(&plan, &results)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?
            );
        }
        Operation::ExecuteTimedAttempt {
            plan,
            index,
            plan_sha256,
            output,
        } => {
            if let Err(error) = pd_maneuver_example::evaluation::timed::execute_timed_attempt(
                &plan,
                index,
                &plan_sha256,
                &output,
            ) {
                if output.is_dir() {
                    let _ = pd_missions::write_json_create_only(
                        &output.join("error.json"),
                        &serde_json::json!({"error":error.chars().take(4096).collect::<String>()}),
                    );
                }
                return Err(error);
            }
        }
        Operation::RecoverSavedBurn {
            scenario,
            flight,
            output,
        } => {
            output_preflight(&output, vec![scenario.clone(), flight.clone()])?;
            if let Err(error) =
                pd_maneuver_example::poly::recovery::recover_saved_burn(&scenario, &flight, &output)
            {
                if output.is_dir() {
                    let _ = pd_missions::write_json_create_only(
                        &output.join("error.json"),
                        &serde_json::json!({"error":error.chars().take(4096).collect::<String>()}),
                    );
                }
                return Err(error);
            }
        }
        Operation::RunTimed {
            scenario,
            program,
            output,
        } => {
            output_preflight(&output, vec![scenario.clone(), program.clone()])?;
            let (evaluation, trace) =
                pd_maneuver_example::poly::evaluate_files(&scenario, &program)?;
            pd_maneuver_example::poly::verify::save_bundle(&output, &evaluation, &trace)?;
            let verified = pd_maneuver_example::poly::verify::verify_bundle(&output);
            println!(
                "{}",
                serde_json::to_string_pretty(&verified.report).map_err(|e| e.to_string())?
            );
            if !verified.report.passed() {
                return Err("timed composite verification failed; evidence retained".into());
            }
        }
        Operation::VerifyTimed { bundle } => {
            let verified = pd_maneuver_example::poly::verify::verify_bundle(&bundle);
            println!(
                "{}",
                serde_json::to_string_pretty(&verified.report).map_err(|e| e.to_string())?
            );
            if !verified.report.passed() {
                return Err("timed composite verification failed".into());
            }
        }
        Operation::ReportTimed { bundle, output } => {
            output_preflight(&output, Vec::new())?;
            let verified = pd_maneuver_example::poly::verify::verify_bundle(&bundle);
            println!(
                "{}",
                serde_json::to_string_pretty(&verified.report).map_err(|e| e.to_string())?
            );
            let evaluation = verified
                .evaluation
                .ok_or("ordinary physical capture failed verification")?;
            write_rich_report(&evaluation, &output)?;
        }
        Operation::RunEvaluation {
            plan,
            output,
            continue_from,
        } => {
            let summary = pd_maneuver_example::evaluation::run_evaluation(
                &plan,
                &output,
                continue_from.as_deref(),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?
            );
            if !summary.complete {
                return Err(
                    "evaluation incomplete; evidence and every scheduled row retained".into(),
                );
            }
        }
        Operation::CheckEvaluationResults { plan, results } => {
            let summary = pd_maneuver_example::evaluation::check_result_inventory(&plan, &results)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?
            );
        }
        Operation::ExecutePreparedAttempt {
            plan,
            index,
            plan_sha256,
            output,
            reuse_from,
        } => {
            let result = if let Some(previous) = reuse_from {
                pd_maneuver_example::evaluation::execute_reuse_attempt(
                    &plan,
                    index,
                    &plan_sha256,
                    &previous,
                    &output,
                )
                .map(|_| ())
            } else {
                pd_maneuver_example::evaluation::execute_prepared_attempt(
                    &plan,
                    index,
                    &plan_sha256,
                    &output,
                )
                .map(|_| ())
            };
            if let Err(error) = result {
                if output.is_dir() {
                    let bounded = error.chars().take(4096).collect::<String>();
                    let _ = pd_maneuver_example::evaluation::write_limited(
                        &output.join("error.json"),
                        &serde_json::json!({"error":bounded}),
                        64 * 1024,
                    );
                }
                return Err(error);
            }
        }
        Operation::PrepareEvaluation {
            recipe,
            inputs,
            output,
        } => {
            pd_maneuver_example::evaluation::prepare_evaluation(&recipe, &inputs, &output)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&pd_maneuver_example::evaluation::check_evaluation(
                    &output
                )?)
                .map_err(|e| e.to_string())?
            );
        }
        Operation::CheckEvaluation { plan } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&pd_maneuver_example::evaluation::check_evaluation(
                    &plan
                )?)
                .map_err(|e| e.to_string())?
            );
        }
        Operation::Run {
            scenario,
            program,
            output,
        } => {
            output_preflight(&output, vec![scenario.clone(), program.clone()])?;
            // Reject input symlinks before canonicalizing their evidence paths.
            read_regular(&scenario, 16 * 1024 * 1024)?;
            read_regular(&program, 64 * 1024)?;
            let (evaluation, trace) = evaluate_files(&scenario, &program)?;
            save_bundle(&output, &evaluation, &trace)?;
            // Verification is explicit in the Run receipt printed here. These
            // three additional passes are separate from preparation_resources.
            let verified = verify_bundle(&output);
            println!(
                "{}",
                serde_json::to_string_pretty(&verified.report).map_err(|e| e.to_string())?
            );
            if !verified.report.passed() {
                return Err(
                    "saved attempt has failed composite verification; evidence retained".into(),
                );
            }
        }
        Operation::Verify { bundle } => {
            let result = verify_bundle(&bundle);
            println!(
                "{}",
                serde_json::to_string_pretty(&result.report).map_err(|e| e.to_string())?
            );
            if !result.report.passed() {
                return Err("composite verification failed".into());
            }
        }
        Operation::Report { bundle, output } => {
            output_preflight(&output, Vec::new())?;
            let result = verify_bundle(&bundle);
            println!(
                "{}",
                serde_json::to_string_pretty(&result.report).map_err(|e| e.to_string())?
            );
            let evaluation = result
                .evaluation
                .ok_or("ordinary physical capture failed verification")?;
            write_rich_report(&evaluation, &output)?;
        }
    }
    Ok(())
}

fn write_rich_report(
    evaluation: &pd_eval::EpisodeEvaluationV1,
    output: &Path,
) -> Result<(), String> {
    let html = pd_report::render_agent_run_report(
        &evaluation.scenario,
        &evaluation.execution,
        pd_report::AgentReportEvidenceV1 {
            integrity_passed: evaluation.integrity_passed,
            physical_prefix_verified: evaluation.replay.physical_prefix_verified,
            complete_episode_verified: evaluation.replay.complete_episode_verified,
            verified_target_landing: evaluation.verified_target_landing,
        },
        pd_report::agent_capture_navigation(),
    )
    .map_err(|e| e.to_string())?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    file.write_all(html.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}
