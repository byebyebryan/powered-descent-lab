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
#[command(about = "Deterministic maneuver activation software example; no training or campaign")]
struct Args {
    #[command(subcommand)]
    operation: Operation,
}
#[derive(Subcommand)]
enum Operation {
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
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
