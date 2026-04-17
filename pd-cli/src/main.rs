use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use pd_control::{BaselineController, IdleController, run_controller};
use pd_core::{
    ActionLogEntry, EventRecord, RunArtifacts, RunContext, RunManifest, ScenarioSpec,
    replay_simulation,
};
use serde::de::DeserializeOwned;

#[derive(Debug, Parser)]
#[command(name = "pd-cli")]
#[command(about = "Powered descent lab command-line entry point")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Run(RunArgs),
    Replay(ReplayArgs),
}

#[derive(Clone, Debug, ValueEnum)]
enum ControllerChoice {
    Baseline,
    Idle,
}

#[derive(Debug, Parser)]
struct RunArgs {
    #[arg(value_name = "SCENARIO_JSON")]
    scenario: PathBuf,

    #[arg(long, value_enum, default_value_t = ControllerChoice::Baseline)]
    controller: ControllerChoice,

    #[arg(long, value_name = "ARTIFACTS_JSON")]
    output: Option<PathBuf>,

    #[arg(long, value_name = "ARTIFACTS_DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct ReplayArgs {
    #[arg(value_name = "SCENARIO_JSON")]
    scenario: PathBuf,

    #[arg(long, value_name = "ARTIFACTS_DIR")]
    bundle_dir: PathBuf,

    #[arg(long, value_name = "ARTIFACTS_JSON")]
    output: Option<PathBuf>,

    #[arg(long, value_name = "ARTIFACTS_DIR")]
    output_dir: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(args) => run(args),
        Commands::Replay(args) => replay(args),
    }
}

fn run(args: RunArgs) -> Result<()> {
    let scenario = load_scenario(&args.scenario)?;
    let ctx = RunContext::from_scenario(&scenario)
        .map_err(anyhow::Error::msg)
        .context("failed to build run context from scenario")?;

    let artifacts = match args.controller {
        ControllerChoice::Baseline => {
            let mut controller = BaselineController;
            run_controller(&ctx, &mut controller)
        }
        ControllerChoice::Idle => {
            let mut controller = IdleController;
            run_controller(&ctx, &mut controller)
        }
    }
    .map_err(anyhow::Error::msg)
    .context("simulation run failed")?;

    write_outputs(
        args.output.as_deref(),
        args.output_dir.as_deref(),
        &artifacts,
    )?;
    println!("{}", serde_json::to_string_pretty(&artifacts.manifest)?);
    Ok(())
}

fn replay(args: ReplayArgs) -> Result<()> {
    let scenario = load_scenario(&args.scenario)?;
    let ctx = RunContext::from_scenario(&scenario)
        .map_err(anyhow::Error::msg)
        .context("failed to build run context from scenario")?;
    let original = load_artifact_bundle(&args.bundle_dir)?;

    let replayed = replay_simulation(&ctx, &original.manifest.controller_id, &original.actions)
        .map_err(anyhow::Error::msg)
        .context("replay failed from action log")?;

    if !manifest_matches(&replayed.manifest, &original.manifest) {
        anyhow::bail!("replayed manifest does not match original manifest");
    }
    if !event_streams_match(&replayed.events, &original.events) {
        anyhow::bail!("replayed events do not match original event log");
    }

    write_outputs(
        args.output.as_deref(),
        args.output_dir.as_deref(),
        &replayed,
    )?;
    println!("{}", serde_json::to_string_pretty(&replayed.manifest)?);
    Ok(())
}

fn load_scenario(path: &Path) -> Result<ScenarioSpec> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse scenario json {}", path.display()))
}

fn write_outputs(
    output: Option<&Path>,
    output_dir: Option<&Path>,
    artifacts: &RunArtifacts,
) -> Result<()> {
    if let Some(path) = output {
        write_artifacts(path, artifacts)?;
    }
    if let Some(path) = output_dir {
        write_artifact_bundle(path, artifacts)?;
    }
    Ok(())
}

fn write_artifacts(path: &Path, artifacts: &RunArtifacts) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create artifact output directory {}",
                parent.display()
            )
        })?;
    }
    let raw = serde_json::to_string_pretty(artifacts)?;
    fs::write(path, raw)
        .with_context(|| format!("failed to write artifacts file {}", path.display()))?;
    Ok(())
}

fn write_artifact_bundle(path: &Path, artifacts: &RunArtifacts) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create artifact bundle dir {}", path.display()))?;
    write_json(&path.join("manifest.json"), &artifacts.manifest)?;
    write_json(&path.join("actions.json"), &artifacts.actions)?;
    write_json(&path.join("events.json"), &artifacts.events)?;
    write_json(&path.join("samples.json"), &artifacts.samples)?;
    Ok(())
}

fn load_artifact_bundle(path: &Path) -> Result<ArtifactBundle> {
    Ok(ArtifactBundle {
        manifest: read_json(&path.join("manifest.json"))?,
        actions: read_json(&path.join("actions.json"))?,
        events: read_json(&path.join("events.json"))?,
    })
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let raw = serde_json::to_string_pretty(value)?;
    fs::write(path, raw)
        .with_context(|| format!("failed to write json file {}", path.display()))?;
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read json file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse json file {}", path.display()))
}

struct ArtifactBundle {
    manifest: RunManifest,
    actions: Vec<ActionLogEntry>,
    events: Vec<EventRecord>,
}

fn manifest_matches(lhs: &RunManifest, rhs: &RunManifest) -> bool {
    lhs.schema_version == rhs.schema_version
        && lhs.scenario_id == rhs.scenario_id
        && lhs.scenario_name == rhs.scenario_name
        && lhs.controller_id == rhs.controller_id
        && lhs.physics_hz == rhs.physics_hz
        && lhs.controller_hz == rhs.controller_hz
        && approx_eq(lhs.sim_time_s, rhs.sim_time_s)
        && lhs.physics_steps == rhs.physics_steps
        && lhs.controller_updates == rhs.controller_updates
        && lhs.physical_outcome == rhs.physical_outcome
        && lhs.mission_outcome == rhs.mission_outcome
        && lhs.end_reason == rhs.end_reason
}

fn event_streams_match(lhs: &[EventRecord], rhs: &[EventRecord]) -> bool {
    lhs.len() == rhs.len()
        && lhs.iter().zip(rhs.iter()).all(|(lhs, rhs)| {
            lhs.physics_step == rhs.physics_step
                && lhs.kind == rhs.kind
                && lhs.message == rhs.message
                && approx_eq(lhs.sim_time_s, rhs.sim_time_s)
        })
}

fn approx_eq(lhs: f64, rhs: f64) -> bool {
    (lhs - rhs).abs() <= 1e-9
}
