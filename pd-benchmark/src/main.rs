use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use pd_benchmark::IncumbentAdapterV1;
use pd_eval::{AttemptDispositionV1, CollectionEvaluationRequestV1, EvidenceScope};

#[derive(Parser)]
#[command(about = "Create-only common native incumbent campaign; no publication")]
struct Args {
    #[arg(long)]
    collection: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    mirror_leftward: bool,
    #[arg(long, default_value_t = 24)]
    correction_cap: usize,
    #[arg(long)]
    count: Option<usize>,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    let adapter = IncumbentAdapterV1::new(args.mirror_leftward, args.correction_cap)?;
    let manifest = pd_missions::read_collection_manifest(&args.collection)?;
    let count = args.count.unwrap_or(manifest.entries.len());
    if count == 0 || count > manifest.entries.len() {
        return Err("count must select a nonempty declared prefix".into());
    }
    let request = CollectionEvaluationRequestV1 {
        schema_version: 1,
        manifest_sha256: pd_missions::sha256(
            &std::fs::read(args.collection.join("manifest.json")).map_err(|e| e.to_string())?,
        ),
        agent: adapter.descriptor(),
        attempt_indices: manifest
            .entries
            .iter()
            .take(count)
            .map(|e| e.index)
            .collect(),
    };
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace")
        .to_owned();
    let mut sources = vec![workspace.join("Cargo.toml"), workspace.join("Cargo.lock")];
    for package in [
        "pd-core",
        "pd-run",
        "pd-missions",
        "pd-eval",
        "pd-report",
        "pd-planner",
        "pd-plan",
        "pd-control",
        "pd-benchmark",
    ] {
        sources.extend([
            workspace.join(package).join("src"),
            workspace.join(package).join("Cargo.toml"),
        ]);
    }
    let scope = EvidenceScope {
        sources,
        inputs: vec![],
    };
    let mut details = Vec::new();
    let result = pd_eval::evaluate_collection_with_executor(
        &args.collection,
        &args.output,
        request,
        &mut |scenario, scope| {
            let (evaluation, detail) = adapter.execute(scenario, scope)?;
            details.push(detail);
            Ok(evaluation)
        },
        &scope,
    )?;
    pd_missions::write_json_create_only(&args.output.join("approach-attempts.json"), &details)?;
    let mut landed = 0;
    let mut unsupported = 0;
    let mut failed = 0;
    let mut rows = Vec::new();
    let entries: std::collections::BTreeMap<_, _> = manifest
        .entries
        .iter()
        .map(|entry| (entry.index, entry))
        .collect();
    for attempt in &result.attempts {
        let entry = entries[&attempt.mission_index];
        let disposition = serde_json::to_value(&attempt.disposition).map_err(|e| e.to_string())?;
        let mut row = pd_report::campaign::CampaignRowV1 {
            attempt_index: attempt.attempt_index,
            mission_index: attempt.mission_index,
            physical_sha256: entry.physical_sha256.clone(),
            terrain_family: manifest.request.recipe.generator_version.clone(),
            direction: if (attempt.mission_index / 4).is_multiple_of(2) {
                1
            } else {
                -1
            },
            terrain_stratum: (attempt.mission_index % 4) as u32,
            disposition: disposition["kind"].as_str().unwrap_or("invalid").to_owned(),
            detail: disposition["detail"].as_str().map(str::to_owned),
            stop: None,
            physical_outcome: None,
            mission_outcome: None,
            execution_valid: false,
            integrity_passed: false,
            physical_replay_verified: false,
            complete_replay_verified: false,
            verified_target_landing: false,
            sim_time_s: None,
            fuel_kg: None,
            wall_time_us: None,
            report_href: None,
        };
        match &attempt.disposition {
            AttemptDispositionV1::Unsupported { .. } => unsupported += 1,
            AttemptDispositionV1::Executed {
                capture,
                verified_target_landing,
                ..
            } => {
                landed += usize::from(*verified_target_landing);
                let evaluation = pd_eval::load_verified_evaluation(&args.output.join(capture))?;
                let episode = &evaluation.execution.episode;
                row.stop = Some(episode.stop);
                row.physical_outcome = Some(episode.run.manifest.physical_outcome.clone());
                row.mission_outcome = Some(episode.run.manifest.mission_outcome.clone());
                row.execution_valid = episode.failure.is_none();
                row.integrity_passed = evaluation.integrity_passed;
                row.physical_replay_verified = evaluation.replay.physical_prefix_verified;
                row.complete_replay_verified = evaluation.replay.complete_episode_verified;
                row.verified_target_landing = evaluation.verified_target_landing;
                row.sim_time_s = Some(episode.run.manifest.sim_time_s);
                row.fuel_kg = Some(episode.final_state.fuel_kg);
                row.wall_time_us = Some(evaluation.execution.performance.wall_time_us);
                row.report_href = Some(format!("{capture}/report.html"));
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
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(args.output.join(capture).join("report.html"))
                    .map_err(|e| e.to_string())?;
                file.write_all(html.as_bytes()).map_err(|e| e.to_string())?;
            }
            _ => failed += 1,
        }
        rows.push(row);
    }
    let usage = pd_missions::read_collection_usage(&args.collection)?;
    let page =
        pd_report::campaign::CampaignPageV1 {
            schema_version: 1,
            title: format!(
                "Procedural incumbent · {:?} · {} · cap {}",
                manifest.request.role,
                if args.mirror_leftward {
                    "reflected"
                } else {
                    "raw"
                },
                args.correction_cap
            ),
            agent: result.request.agent.clone(),
            integrity_passed: result.integrity_passed,
            unique_scheduled_missions: result.unique_scheduled_missions,
            manifest_sha256: result.request.manifest_sha256.clone(),
            collection_role: format!("{:?}", manifest.request.role),
            prior_training_or_tuning: Some(usage.iter().any(|receipt| {
                matches!(
                    receipt.exposure,
                    pd_missions::CollectionExposureV1::Training
                        | pd_missions::CollectionExposureV1::Tuning
                )
            })),
            recorded_input_inspection: Some(usage.iter().any(|receipt| {
                receipt.exposure == pd_missions::CollectionExposureV1::InputInspection
            })),
            attempts: rows,
            summary_href: "summary.json".into(),
        };
    pd_missions::write_json_create_only(&args.output.join("report-input.json"), &page)?;
    let html = pd_report::campaign::render_campaign_page(&page)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(args.output.join("report.html"))
        .map_err(|e| e.to_string())?;
    file.write_all(html.as_bytes()).map_err(|e| e.to_string())?;
    println!(
        "{} scheduled, {} verified target landings, {} unsupported, {} preparation/capture failures; campaign integrity {}",
        result.attempts.len(),
        landed,
        unsupported,
        failed,
        result.integrity_passed
    );
    if !result.integrity_passed {
        return Err("campaign input or source integrity failed".into());
    }
    Ok(())
}
