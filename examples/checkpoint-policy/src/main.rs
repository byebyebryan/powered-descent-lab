use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::{Parser, Subcommand};
use pd_core::{BoundedRunLimitsV1, RunContext};
use pd_eval::{
    CollectionEvaluationRequestV1, EvidenceScope, evaluate_agent, evaluate_collection,
    load_verified_evaluation,
};
use pd_missions::{load_collection_mission, sha256, verify_collection, write_json_create_only};
use pd_policy_example::LinearPolicy;
use pd_run::{Agent, CaptureModeV1, run_agent_with_capture_mode};

#[derive(Parser)]
#[command(about = "Independent checkpoint policy integration example; weights are untrained")]
struct Args {
    #[command(subcommand)]
    operation: Operation,
}

#[derive(Subcommand)]
enum Operation {
    Evaluate {
        #[arg(long)]
        collection: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        repeat_index: Vec<u64>,
    },
    Replay {
        capture: PathBuf,
    },
    Report {
        capture: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Benchmark {
        #[arg(long)]
        collection: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 4)]
        repeats: u32,
    },
}

fn evidence(collection: &Path, checkpoint: &Path) -> EvidenceScope {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut sources = vec![repo.join("Cargo.toml"), repo.join("Cargo.lock")];
    for package in [
        "pd-core",
        "pd-run",
        "pd-missions",
        "pd-eval",
        "pd-report",
        "examples/checkpoint-policy",
    ] {
        sources.push(repo.join(package).join("Cargo.toml"));
        sources.push(repo.join(package).join("src"));
    }
    EvidenceScope {
        sources,
        inputs: vec![checkpoint.to_owned(), collection.to_owned()],
    }
}

fn render(capture: &Path, output: &Path) -> Result<(), String> {
    let result = load_verified_evaluation(capture)?;
    let html = pd_report::render_agent_run_report(
        &result.scenario,
        &result.execution,
        pd_report::AgentReportEvidenceV1 {
            integrity_passed: result.integrity_passed,
            physical_prefix_verified: result.replay.physical_prefix_verified,
            complete_episode_verified: result.replay.complete_episode_verified,
            verified_target_landing: result.verified_target_landing,
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

fn limits(ctx: &RunContext) -> Result<BoundedRunLimitsV1, String> {
    pd_run::scenario_horizon_limits(ctx).map_err(|e| e.to_string())
}

fn peak_rss_kib() -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find(|line| line.starts_with("VmHWM:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

fn benchmark(
    collection: &Path,
    checkpoint: &Path,
    output: &Path,
    repeats: u32,
) -> Result<(), String> {
    if repeats == 0 || repeats > 32 {
        return Err("bounded benchmark repeats must be 1..32".to_owned());
    }
    let manifest = verify_collection(collection)?;
    let entries: Vec<_> = manifest.entries.iter().take(8).collect();
    if entries.is_empty() {
        return Err("benchmark needs resolved inputs".to_owned());
    }
    if std::fs::symlink_metadata(output).is_ok() {
        return Err("benchmark output already exists".to_owned());
    }
    let usage_id = format!(
        "benchmark-{}",
        &sha256(output.to_string_lossy().as_bytes())[..24]
    );
    pd_missions::record_collection_usage(
        collection,
        &usage_id,
        pd_missions::CollectionExposureV1::Evaluation,
        "Consumed first eight inputs for bounded runtime/capture cost measurement",
    )?;
    let source_scope = evidence(collection, checkpoint);
    let before = source_scope.snapshot()?;
    let mut measurements = Vec::new();
    let mut summaries = Vec::new();
    let mut summary_capture_equivalent = true;
    for mode in [CaptureModeV1::Summary, CaptureModeV1::Captured] {
        let started = Instant::now();
        let mut steps = 0_u64;
        let mut updates = 0_u64;
        let mut retained_records = 0_usize;
        let mut serialized_bytes = 0_usize;
        let mut serialization_s = 0.0;
        let mut ordinal = 0;
        for _ in 0..repeats {
            for entry in &entries {
                let mission = load_collection_mission(collection, &manifest, entry)?;
                let ctx = RunContext::from_scenario(&mission.scenario)?;
                let result = run_agent_with_capture_mode(
                    &ctx,
                    &mut LinearPolicy::load(checkpoint)?,
                    limits(&ctx)?,
                    mode,
                )
                .map_err(|e| e.to_string())?;
                steps += result.episode.summary.final_state.physics_step;
                updates += result.episode.summary.manifest.controller_updates;
                if mode == CaptureModeV1::Summary {
                    summaries.push(result.episode.summary.clone());
                } else {
                    // Compare by ordinal; summary values include the complete
                    // physical/mission state and all clocks/fuel, with no timings.
                    summary_capture_equivalent &= result.episode.summary == summaries[ordinal];
                    if let Some(capture) = &result.episode.capture {
                        retained_records += capture.run.actions.len()
                            + capture.run.events.len()
                            + capture.run.samples.len();
                        let serialize = Instant::now();
                        serialized_bytes += serde_json::to_vec(capture)
                            .map_err(|e| e.to_string())?
                            .len();
                        serialization_s += serialize.elapsed().as_secs_f64();
                    }
                }
                ordinal += 1;
            }
        }
        measurements.push(serde_json::json!({"mode": mode, "episodes": entries.len() as u32 * repeats,
            "physics_steps": steps, "agent_updates": updates, "wall_s": started.elapsed().as_secs_f64(),
            "forecast_steps": 0, "retained_trace_records": retained_records,
            "serialized_capture_bytes": serialized_bytes, "serialization_s": serialization_s}));
    }
    let proof_started = Instant::now();
    let mut complete_replays = 0;
    for entry in &entries {
        let mission = load_collection_mission(collection, &manifest, entry)?;
        let ctx = RunContext::from_scenario(&mission.scenario)?;
        let result = evaluate_agent(
            &mission.scenario,
            &mut LinearPolicy::load(checkpoint)?,
            limits(&ctx)?,
            &source_scope,
        )?;
        if result.replay.complete_episode_verified {
            complete_replays += 1;
        }
    }
    let after = source_scope.snapshot()?;
    write_json_create_only(
        output,
        &serde_json::json!({"schema_version": 1, "input_manifest_sha256": sha256(&std::fs::read(collection.join("manifest.json")).map_err(|e| e.to_string())?),
        "measurements": measurements, "proof_episodes": entries.len(), "proof_complete_replays": complete_replays,
        "proof_wall_s": proof_started.elapsed().as_secs_f64(), "peak_process_rss_kib": peak_rss_kib(),
        "source_and_inputs_unchanged": before == after, "summary_capture_equivalent": summary_capture_equivalent,
        "untrained_example": true}),
    )?;
    if !summary_capture_equivalent || before != after {
        return Err("benchmark physics or evidence drift".to_owned());
    }
    Ok(())
}

fn main() -> Result<(), String> {
    match Args::parse().operation {
        Operation::Evaluate {
            collection,
            checkpoint,
            output,
            repeat_index,
        } => {
            let manifest = verify_collection(&collection)?;
            let mut indices: Vec<_> = manifest.entries.iter().map(|entry| entry.index).collect();
            indices.extend(repeat_index);
            let prototype = LinearPolicy::load(&checkpoint)?;
            let request = CollectionEvaluationRequestV1 {
                schema_version: 1,
                manifest_sha256: sha256(
                    &std::fs::read(collection.join("manifest.json")).map_err(|e| e.to_string())?,
                ),
                agent: prototype.descriptor(),
                attempt_indices: indices,
            };
            let scope = evidence(&collection, &checkpoint);
            let result = evaluate_collection(
                &collection,
                &output,
                request,
                &mut || Ok(Box::new(LinearPolicy::load(&checkpoint)?) as Box<dyn Agent>),
                &scope,
            )?;
            for attempt in &result.attempts {
                if let pd_eval::AttemptDispositionV1::Executed { capture, .. } =
                    &attempt.disposition
                {
                    render(
                        &output.join(capture),
                        &output.join(capture).join("report.html"),
                    )?;
                }
            }
            println!(
                "Saved {} scheduled attempts on {} unique missions; weights are an untrained integration example.",
                result.attempts.len(),
                result.unique_scheduled_missions
            );
        }
        Operation::Replay { capture } => {
            let result = load_verified_evaluation(&capture)?;
            println!(
                "Stop: {:?}; physical: {:?}; mission: {:?}; integrity: {}; complete replay: {}; verified target landing: {}",
                result.execution.episode.stop,
                result.execution.episode.run.manifest.physical_outcome,
                result.execution.episode.run.manifest.mission_outcome,
                result.integrity_passed,
                result.replay.complete_episode_verified,
                result.verified_target_landing
            );
        }
        Operation::Report { capture, output } => render(&capture, &output)?,
        Operation::Benchmark {
            collection,
            checkpoint,
            output,
            repeats,
        } => benchmark(&collection, &checkpoint, &output, repeats)?,
    }
    Ok(())
}
