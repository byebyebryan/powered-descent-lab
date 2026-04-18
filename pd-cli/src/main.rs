mod report;

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use pd_control::{
    ControllerSpec, ControllerUpdateRecord, built_in_controller_spec, run_controller_spec,
};
use pd_core::{
    ActionLogEntry, EventRecord, RunArtifacts, RunContext, RunManifest, SampleRecord, ScenarioSpec,
    replay_simulation,
};
use serde::{Serialize, de::DeserializeOwned};

#[cfg(unix)]
use std::os::unix::fs as platform_fs;
#[cfg(windows)]
use std::os::windows::fs as platform_fs;

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
    Report(ReportArgs),
}

#[derive(Debug, Parser)]
struct RunArgs {
    #[arg(value_name = "SCENARIO_JSON")]
    scenario: PathBuf,

    #[arg(long, default_value = "baseline")]
    controller: String,

    #[arg(long, value_name = "CONTROLLER_JSON")]
    controller_config: Option<PathBuf>,

    #[arg(long, value_name = "ARTIFACTS_JSON")]
    output: Option<PathBuf>,

    #[arg(long, value_name = "ARTIFACTS_DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct ReplayArgs {
    #[arg(value_name = "SCENARIO_JSON")]
    scenario: Option<PathBuf>,

    #[arg(long, value_name = "ARTIFACTS_DIR")]
    bundle_dir: PathBuf,

    #[arg(long, value_name = "ARTIFACTS_JSON")]
    output: Option<PathBuf>,

    #[arg(long, value_name = "ARTIFACTS_DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct ReportArgs {
    #[arg(long, value_name = "ARTIFACTS_DIR")]
    bundle_dir: PathBuf,

    #[arg(long, value_name = "REPORT_HTML")]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(args) => run(args),
        Commands::Replay(args) => replay(args),
        Commands::Report(args) => render_report(args),
    }
}

fn run(args: RunArgs) -> Result<()> {
    let scenario = load_scenario(&args.scenario)?;
    let controller_spec =
        resolve_controller_spec(&args.controller, args.controller_config.as_deref())?;
    let ctx = RunContext::from_scenario(&scenario)
        .map_err(anyhow::Error::msg)
        .context("failed to build run context from scenario")?;
    let default_output_dir = (args.output.is_none() && args.output_dir.is_none())
        .then(|| default_run_output_dir(&args.scenario, &controller_spec));

    let artifacts = run_controller_spec(&ctx, &controller_spec)
        .map_err(anyhow::Error::msg)
        .context("simulation run failed")?;

    write_outputs(
        args.output.as_deref(),
        args.output_dir.as_deref().or(default_output_dir.as_deref()),
        Some(&scenario),
        Some(&controller_spec),
        &artifacts.run,
        &artifacts.controller_updates,
    )?;
    println!("{}", serde_json::to_string_pretty(&artifacts.run.manifest)?);
    Ok(())
}

fn replay(args: ReplayArgs) -> Result<()> {
    let bundle = load_artifact_bundle(&args.bundle_dir)?;
    let scenario = match args.scenario.as_deref() {
        Some(path) => load_scenario(path)?,
        None => bundle.scenario.clone(),
    };
    let default_output_dir = (args.output.is_none() && args.output_dir.is_none())
        .then(|| default_replay_output_dir(&args.bundle_dir));
    let ctx = RunContext::from_scenario(&scenario)
        .map_err(anyhow::Error::msg)
        .context("failed to build run context from scenario")?;

    let replayed = replay_simulation(&ctx, &bundle.manifest.controller_id, &bundle.actions)
        .map_err(anyhow::Error::msg)
        .context("replay failed from action log")?;

    if !manifest_matches(&replayed.manifest, &bundle.manifest) {
        anyhow::bail!("replayed manifest does not match original manifest");
    }
    if !event_streams_match(&replayed.events, &bundle.events) {
        anyhow::bail!("replayed events do not match original event log");
    }

    write_outputs(
        args.output.as_deref(),
        args.output_dir.as_deref().or(default_output_dir.as_deref()),
        Some(&scenario),
        bundle.controller_spec.as_ref(),
        &replayed,
        &bundle.controller_updates,
    )?;
    println!("{}", serde_json::to_string_pretty(&replayed.manifest)?);
    Ok(())
}

fn render_report(args: ReportArgs) -> Result<()> {
    let bundle = load_artifact_bundle(&args.bundle_dir)?;
    let output = args
        .output
        .unwrap_or_else(|| args.bundle_dir.join("report.html"));
    report::write_run_report(
        &output,
        &bundle.scenario,
        bundle.controller_spec.as_ref(),
        &bundle.manifest,
        &bundle.events,
        &bundle.samples,
        &bundle.controller_updates,
    )?;
    maybe_update_latest_link(&args.bundle_dir)?;
    println!("{}", output.display());
    Ok(())
}

fn resolve_controller_spec(name: &str, config_path: Option<&Path>) -> Result<ControllerSpec> {
    if let Some(path) = config_path {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read controller config file {}", path.display()))?;
        return serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse controller config json {}", path.display()));
    }

    built_in_controller_spec(name).ok_or_else(|| anyhow::anyhow!("unknown controller '{}'", name))
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
    scenario: Option<&ScenarioSpec>,
    controller_spec: Option<&ControllerSpec>,
    artifacts: &RunArtifacts,
    controller_updates: &[ControllerUpdateRecord],
) -> Result<()> {
    if let Some(path) = output {
        write_artifacts(
            path,
            scenario,
            controller_spec,
            artifacts,
            controller_updates,
        )?;
    }
    if let Some(path) = output_dir {
        write_artifact_bundle(
            path,
            scenario,
            controller_spec,
            artifacts,
            controller_updates,
        )?;
    }
    Ok(())
}

fn write_artifacts(
    path: &Path,
    scenario: Option<&ScenarioSpec>,
    controller_spec: Option<&ControllerSpec>,
    artifacts: &RunArtifacts,
    controller_updates: &[ControllerUpdateRecord],
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create artifact output directory {}",
                parent.display()
            )
        })?;
    }
    let raw = serde_json::to_string_pretty(&StandaloneArtifacts {
        scenario,
        controller_spec,
        run: artifacts,
        controller_updates,
    })?;
    fs::write(path, raw)
        .with_context(|| format!("failed to write artifacts file {}", path.display()))?;
    Ok(())
}

fn write_artifact_bundle(
    path: &Path,
    scenario: Option<&ScenarioSpec>,
    controller_spec: Option<&ControllerSpec>,
    artifacts: &RunArtifacts,
    controller_updates: &[ControllerUpdateRecord],
) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create artifact bundle dir {}", path.display()))?;
    if let Some(scenario) = scenario {
        write_json(&path.join("scenario.json"), scenario)?;
    }
    if let Some(controller_spec) = controller_spec {
        write_json(&path.join("controller.json"), controller_spec)?;
    }
    write_json(&path.join("manifest.json"), &artifacts.manifest)?;
    write_json(&path.join("actions.json"), &artifacts.actions)?;
    write_json(&path.join("events.json"), &artifacts.events)?;
    write_json(&path.join("samples.json"), &artifacts.samples)?;
    write_json(&path.join("controller_updates.json"), controller_updates)?;
    if let Some(scenario) = scenario {
        report::write_run_report(
            &path.join("report.html"),
            scenario,
            controller_spec,
            &artifacts.manifest,
            &artifacts.events,
            &artifacts.samples,
            controller_updates,
        )?;
        maybe_update_latest_link(path)?;
    }
    Ok(())
}

fn load_artifact_bundle(path: &Path) -> Result<ArtifactBundle> {
    Ok(ArtifactBundle {
        scenario: read_json(&path.join("scenario.json"))?,
        controller_spec: read_optional_json(&path.join("controller.json"))?,
        manifest: read_json(&path.join("manifest.json"))?,
        actions: read_json(&path.join("actions.json"))?,
        events: read_json(&path.join("events.json"))?,
        samples: read_json(&path.join("samples.json"))?,
        controller_updates: read_optional_json(&path.join("controller_updates.json"))?
            .unwrap_or_default(),
    })
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
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

fn read_optional_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    read_json(path).map(Some)
}

struct ArtifactBundle {
    scenario: ScenarioSpec,
    controller_spec: Option<ControllerSpec>,
    manifest: RunManifest,
    actions: Vec<ActionLogEntry>,
    events: Vec<EventRecord>,
    samples: Vec<SampleRecord>,
    controller_updates: Vec<ControllerUpdateRecord>,
}

#[derive(Serialize)]
struct StandaloneArtifacts<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    scenario: Option<&'a ScenarioSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    controller_spec: Option<&'a ControllerSpec>,
    run: &'a RunArtifacts,
    controller_updates: &'a [ControllerUpdateRecord],
}

fn manifest_matches(lhs: &RunManifest, rhs: &RunManifest) -> bool {
    lhs.schema_version == rhs.schema_version
        && lhs.scenario_id == rhs.scenario_id
        && lhs.scenario_name == rhs.scenario_name
        && lhs.scenario_seed == rhs.scenario_seed
        && lhs.scenario_tags == rhs.scenario_tags
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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pd-cli crate should live under repo root")
        .to_path_buf()
}

fn maybe_update_latest_link(target_dir: &Path) -> Result<()> {
    let repo_root = repo_root();
    let outputs_root = repo_root.join("outputs");
    let resolved_target_dir = if target_dir.is_absolute() {
        target_dir.to_path_buf()
    } else {
        repo_root.join(target_dir)
    };

    if !resolved_target_dir.starts_with(&outputs_root) {
        return Ok(());
    }

    let Some(parent_dir) = resolved_target_dir.parent() else {
        return Ok(());
    };
    let Some(target_name) = resolved_target_dir.file_name() else {
        return Ok(());
    };

    let latest_path = parent_dir.join("latest");
    if let Ok(metadata) = fs::symlink_metadata(&latest_path) {
        if metadata.file_type().is_symlink() || metadata.is_file() {
            fs::remove_file(&latest_path).with_context(|| {
                format!(
                    "failed to remove existing latest link {}",
                    latest_path.display()
                )
            })?;
        } else {
            eprintln!(
                "skipping latest link update because '{}' exists and is not a symlink",
                latest_path.display()
            );
            return Ok(());
        }
    }

    platform_fs::symlink(PathBuf::from(target_name), &latest_path).with_context(|| {
        format!(
            "failed to create latest link {} -> {}",
            latest_path.display(),
            target_name.to_string_lossy()
        )
    })?;
    Ok(())
}

fn default_run_output_dir(scenario_path: &Path, controller_spec: &ControllerSpec) -> PathBuf {
    repo_root().join("outputs").join("runs").join(format!(
        "{}__{}",
        file_stem_or_fallback(scenario_path, "run"),
        controller_spec.id()
    ))
}

fn default_replay_output_dir(bundle_dir: &Path) -> PathBuf {
    repo_root()
        .join("outputs")
        .join("replays")
        .join(file_name_or_fallback(bundle_dir, "replay"))
}

fn file_stem_or_fallback(path: &Path, fallback: &str) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(fallback)
        .to_owned()
}

fn file_name_or_fallback(path: &Path, fallback: &str) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(fallback)
        .to_owned()
}
