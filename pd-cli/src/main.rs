use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use pd_control::{BaselineController, IdleController, run_controller};
use pd_core::{RunArtifacts, RunContext, ScenarioSpec};

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(args) => run(args),
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

    if let Some(path) = args.output.as_ref() {
        write_artifacts(path, &artifacts)?;
    }

    println!("{}", serde_json::to_string_pretty(&artifacts.manifest)?);
    Ok(())
}

fn load_scenario(path: &Path) -> Result<ScenarioSpec> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse scenario json {}", path.display()))
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
