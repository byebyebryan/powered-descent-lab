use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use pd_control::{
    ControllerSpec, ControllerUpdateRecord, RunPerformanceStats, built_in_controller_spec,
    run_controller_spec,
};
use pd_core::{
    ActionLogEntry, EventRecord, RunArtifacts, RunContext, RunManifest, SampleRecord, ScenarioSpec,
    replay_simulation,
};
use pd_report::{write_run_preview_svg, write_run_report};
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
        Some(&artifacts.performance),
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
        bundle.performance.as_ref(),
    )?;
    println!("{}", serde_json::to_string_pretty(&replayed.manifest)?);
    Ok(())
}

fn render_report(args: ReportArgs) -> Result<()> {
    let bundle = load_artifact_bundle(&args.bundle_dir)?;
    let output = args
        .output
        .unwrap_or_else(|| default_report_site_output(&args.bundle_dir));
    write_run_report(
        &output,
        &bundle.scenario,
        bundle.controller_spec.as_ref(),
        &bundle.manifest,
        &bundle.events,
        &bundle.samples,
        &bundle.controller_updates,
        bundle.performance.as_ref(),
    )?;
    write_run_preview_svg(
        &args.bundle_dir.join("preview.svg"),
        &bundle.scenario,
        &bundle.manifest,
        &bundle.samples,
    )?;
    if output
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "index.html")
    {
        update_report_site_indexes_for_file(&output)?;
    } else {
        maybe_update_latest_link(&args.bundle_dir)?;
    }
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
    performance: Option<&RunPerformanceStats>,
) -> Result<()> {
    if let Some(path) = output {
        write_artifacts(
            path,
            scenario,
            controller_spec,
            artifacts,
            controller_updates,
            performance,
        )?;
    }
    if let Some(path) = output_dir {
        write_artifact_bundle(
            path,
            scenario,
            controller_spec,
            artifacts,
            controller_updates,
            performance,
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
    performance: Option<&RunPerformanceStats>,
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
        performance,
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
    performance: Option<&RunPerformanceStats>,
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
    if let Some(performance) = performance {
        write_json(&path.join("performance.json"), performance)?;
    }
    if let Some(scenario) = scenario {
        write_run_report(
            &path.join("report.html"),
            scenario,
            controller_spec,
            &artifacts.manifest,
            &artifacts.events,
            &artifacts.samples,
            controller_updates,
            performance,
        )?;
        write_run_preview_svg(
            &path.join("preview.svg"),
            scenario,
            &artifacts.manifest,
            &artifacts.samples,
        )?;
        if let Some(report_site_output) = default_report_site_output_for_bundle(path) {
            write_run_report(
                &report_site_output,
                scenario,
                controller_spec,
                &artifacts.manifest,
                &artifacts.events,
                &artifacts.samples,
                controller_updates,
                performance,
            )?;
            update_report_site_indexes_for_file(&report_site_output)?;
        }
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
        performance: read_optional_json(&path.join("performance.json"))?,
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
    performance: Option<RunPerformanceStats>,
}

#[derive(Serialize)]
struct StandaloneArtifacts<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    scenario: Option<&'a ScenarioSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    controller_spec: Option<&'a ControllerSpec>,
    run: &'a RunArtifacts,
    controller_updates: &'a [ControllerUpdateRecord],
    #[serde(skip_serializing_if = "Option::is_none")]
    performance: Option<&'a RunPerformanceStats>,
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

fn outputs_root() -> PathBuf {
    repo_root().join("outputs")
}

fn reports_root() -> PathBuf {
    outputs_root().join("reports")
}

fn maybe_update_latest_link(target_dir: &Path) -> Result<()> {
    let repo_root = repo_root();
    let outputs_root = outputs_root();
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

fn default_report_site_output(bundle_dir: &Path) -> PathBuf {
    default_report_site_output_for_bundle(bundle_dir)
        .unwrap_or_else(|| bundle_dir.join("report.html"))
}

fn default_report_site_output_for_bundle(bundle_dir: &Path) -> Option<PathBuf> {
    let resolved_bundle_dir = if bundle_dir.is_absolute() {
        bundle_dir.to_path_buf()
    } else {
        repo_root().join(bundle_dir)
    };
    let relative = resolved_bundle_dir.strip_prefix(outputs_root()).ok()?;
    Some(reports_root().join(relative).join("index.html"))
}

fn update_report_site_indexes_for_file(report_file: &Path) -> Result<()> {
    let report_dir = report_file
        .parent()
        .ok_or_else(|| anyhow::anyhow!("report output has no parent directory"))?;
    maybe_update_latest_link(report_dir)?;

    let reports_root = reports_root();
    let resolved_report_dir = if report_dir.is_absolute() {
        report_dir.to_path_buf()
    } else {
        repo_root().join(report_dir)
    };
    if !resolved_report_dir.starts_with(&reports_root) {
        return Ok(());
    }

    if let Some(collection_dir) = collection_dir_for_report_dir(&resolved_report_dir, &reports_root)
    {
        write_collection_index(&collection_dir, &reports_root)?;
    }
    if let Some(scope_dir) = scope_dir_for_report_dir(&resolved_report_dir, &reports_root) {
        write_scope_index(&scope_dir)?;
    }
    write_reports_home_index(&reports_root)?;
    write_outputs_root_index(&outputs_root())?;
    Ok(())
}

fn scope_dir_for_report_dir(report_dir: &Path, reports_root: &Path) -> Option<PathBuf> {
    let relative = report_dir.strip_prefix(reports_root).ok()?;
    let scope = relative.iter().next()?;
    Some(reports_root.join(scope))
}

fn collection_dir_for_report_dir(report_dir: &Path, reports_root: &Path) -> Option<PathBuf> {
    let parent_dir = report_dir.parent()?;
    let relative = parent_dir.strip_prefix(reports_root).ok()?;
    (relative.components().count() > 1).then(|| parent_dir.to_path_buf())
}

fn write_reports_home_index(reports_root: &Path) -> Result<()> {
    fs::create_dir_all(reports_root)
        .with_context(|| format!("failed to create reports root {}", reports_root.display()))?;

    let scope_cards = ["runs", "replays", "eval"]
        .iter()
        .map(|scope| render_scope_card(reports_root, scope))
        .collect::<Vec<_>>()
        .join("");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Powered Descent Lab Reports</title>
  <style>
    :root {{
      color-scheme: light;
      --bg: #f5f1e8;
      --surface: rgba(255,253,248,0.94);
      --line: #d8cebe;
      --ink: #1d1a16;
      --muted: #675d51;
      --accent: #b55d2d;
      --sans: "IBM Plex Sans", "Segoe UI", sans-serif;
      --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: var(--sans);
      color: var(--ink);
      background:
        radial-gradient(circle at top left, rgba(181,93,45,0.08), transparent 28rem),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
    }}
    .page {{
      max-width: 1100px;
      margin: 0 auto;
      padding: 28px 18px 40px;
    }}
    h1 {{ margin: 0 0 8px; font-size: 2rem; }}
    p {{ margin: 0; color: var(--muted); max-width: 72ch; }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 14px;
      margin-top: 22px;
    }}
    .card {{
      display: block;
      text-decoration: none;
      color: inherit;
      padding: 16px 17px;
      border-radius: 18px;
      border: 1px solid var(--line);
      background: var(--surface);
      box-shadow: 0 10px 30px rgba(39,28,18,0.06);
      min-height: 160px;
    }}
    .card:hover {{
      border-color: var(--accent);
      transform: translateY(-1px);
    }}
    .eyebrow {{
      color: var(--muted);
      font-size: 0.78rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }}
    .title {{
      font-size: 1.15rem;
      font-weight: 700;
      margin-top: 6px;
    }}
    .meta {{
      margin-top: 10px;
      display: grid;
      gap: 6px;
      color: var(--muted);
      font-size: 0.9rem;
    }}
    .meta code {{
      font-family: var(--mono);
      background: rgba(248,243,234,0.92);
      padding: 1px 5px;
      border-radius: 6px;
    }}
    @media (max-width: 860px) {{
      .grid {{ grid-template-columns: 1fr; }}
    }}
  </style>
</head>
<body>
  <div class="page">
    <h1>Report Site</h1>
    <p>Stable HTML entrypoints live under <code>/reports/</code>. Raw artifacts remain outside this tree, but the default navigation surface now keeps runs, replays, and batch pages isolated from bundle JSON.</p>
    <div class="grid">{scope_cards}</div>
  </div>
</body>
</html>"#
    );
    fs::write(reports_root.join("index.html"), html).with_context(|| {
        format!(
            "failed to write reports home index {}",
            reports_root.join("index.html").display()
        )
    })?;
    Ok(())
}

fn write_outputs_root_index(outputs_root: &Path) -> Result<()> {
    fs::create_dir_all(outputs_root)
        .with_context(|| format!("failed to create outputs root {}", outputs_root.display()))?;

    let reports_href = "reports/";
    let latest_run = reports_root()
        .join("runs")
        .join("latest")
        .exists()
        .then_some("reports/runs/latest/");
    let latest_eval = reports_root()
        .join("eval")
        .join("latest")
        .exists()
        .then_some("reports/eval/latest/");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Powered Descent Lab Outputs</title>
  <style>
    :root {{
      color-scheme: light;
      --bg: #f5f1e8;
      --surface: rgba(255,253,248,0.94);
      --line: #d8cebe;
      --ink: #1d1a16;
      --muted: #675d51;
      --accent: #b55d2d;
      --sans: "IBM Plex Sans", "Segoe UI", sans-serif;
      --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: var(--sans);
      color: var(--ink);
      background:
        radial-gradient(circle at top left, rgba(181,93,45,0.08), transparent 28rem),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
    }}
    .page {{
      max-width: 1200px;
      margin: 0 auto;
      padding: 28px 18px 40px;
    }}
    h1 {{ margin: 0 0 8px; font-size: 2rem; }}
    p {{ margin: 0; color: var(--muted); max-width: 74ch; }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 14px;
      margin-top: 22px;
    }}
    .card {{
      display: block;
      text-decoration: none;
      color: inherit;
      padding: 17px 18px;
      border-radius: 18px;
      border: 1px solid var(--line);
      background: var(--surface);
      box-shadow: 0 10px 30px rgba(39,28,18,0.06);
      min-height: 180px;
    }}
    .card:hover {{ border-color: var(--accent); transform: translateY(-1px); }}
    .eyebrow {{
      color: var(--muted);
      font-size: 0.78rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }}
    .title {{ font-size: 1.25rem; font-weight: 700; margin-top: 6px; }}
    .meta {{ margin-top: 12px; display: grid; gap: 8px; color: var(--muted); font-size: 0.92rem; }}
    .meta code {{ font-family: var(--mono); background: rgba(248,243,234,0.92); padding: 1px 5px; border-radius: 6px; }}
    .link-row {{ display: flex; flex-wrap: wrap; gap: 8px; margin-top: 14px; }}
    .link-row a {{
      text-decoration: none;
      color: inherit;
      border: 1px solid var(--line);
      background: rgba(248,243,234,0.92);
      padding: 7px 10px;
      border-radius: 999px;
      font-size: 0.84rem;
    }}
    @media (max-width: 860px) {{ .grid {{ grid-template-columns: 1fr; }} }}
  </style>
</head>
<body>
  <div class="page">
    <h1>Outputs</h1>
    <p>The root landing page keeps stable HTML reports separate from raw bundles. Start with the report site unless you explicitly need artifact JSON or bundle directories.</p>
    <div class="grid">
      <div class="card">
        <div class="eyebrow">recommended</div>
        <div class="title">Report site</div>
        <div class="meta">
          <div>Clean HTML navigation for runs, replays, and batch reports.</div>
          <div>Entry: <code>{reports_href}</code></div>
        </div>
        <div class="link-row">
          <a href="{reports_href}">home</a>
          <a href="reports/runs/">runs</a>
          <a href="reports/eval/">eval</a>
          <a href="reports/replays/">replays</a>
        </div>
      </div>
      <div class="card">
        <div class="eyebrow">raw</div>
        <div class="title">Artifact directories</div>
        <div class="meta">
          <div>Direct access to raw bundle trees and JSON outputs.</div>
          <div>Use these when a report does not surface the data you need yet.</div>
        </div>
        <div class="link-row">
          <a href="runs/">runs/</a>
          <a href="eval/">eval/</a>
          <a href="replays/">replays/</a>
        </div>
      </div>
    </div>
    <div class="grid">
      <div class="card">
        <div class="eyebrow">latest</div>
        <div class="title">Fast paths</div>
        <div class="meta">
          <div>Use these when you mostly care about the most recent generated pages.</div>
        </div>
        <div class="link-row">
          {latest_run_link}
          {latest_eval_link}
        </div>
      </div>
      <div class="card">
        <div class="eyebrow">notes</div>
        <div class="title">Structure</div>
        <div class="meta">
          <div>Stable HTML: <code>reports/...</code></div>
          <div>Raw artifacts: <code>runs/</code>, <code>eval/</code>, <code>replays/</code></div>
        </div>
      </div>
    </div>
  </div>
</body>
</html>"#,
        reports_href = reports_href,
        latest_run_link = latest_run
            .map(|href| format!(r#"<a href="{href}">latest run</a>"#))
            .unwrap_or_else(|| r#"<span>latest run not yet created</span>"#.to_owned()),
        latest_eval_link = latest_eval
            .map(|href| format!(r#"<a href="{href}">latest batch</a>"#))
            .unwrap_or_else(|| r#"<span>latest batch not yet created</span>"#.to_owned()),
    );

    fs::write(outputs_root.join("index.html"), html).with_context(|| {
        format!(
            "failed to write outputs root index {}",
            outputs_root.join("index.html").display()
        )
    })?;
    Ok(())
}

fn render_scope_card(reports_root: &Path, scope: &str) -> String {
    let scope_dir = reports_root.join(scope);
    let latest_dir = scope_dir.join("latest");
    let latest_href = latest_dir.exists().then(|| format!("{scope}/latest/"));
    let entries = scope_entries(&scope_dir).unwrap_or_default();
    let latest_entry = entries
        .first()
        .map(|entry| entry.name.as_str())
        .unwrap_or("none");
    let total = entries.len();

    format!(
        r#"<a class="card" href="{scope}/">
  <div class="eyebrow">reports</div>
  <div class="title">{title}</div>
  <div class="meta">
    <div>entries: <strong>{total}</strong></div>
    <div>latest: <code>{latest_entry}</code></div>
    <div>{latest_line}</div>
  </div>
</a>"#,
        scope = scope,
        title = escape_html(&scope_title(scope)),
        total = total,
        latest_entry = escape_html(latest_entry),
        latest_line = latest_href
            .map(|href| format!(r#"latest url: <code>{}</code>"#, escape_html(&href)))
            .unwrap_or_else(|| "latest url: <code>not yet created</code>".to_owned()),
    )
}

fn write_scope_index(scope_dir: &Path) -> Result<()> {
    fs::create_dir_all(scope_dir)
        .with_context(|| format!("failed to create scope dir {}", scope_dir.display()))?;

    let entries = scope_entries(scope_dir)?;
    let scope = scope_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("reports");
    let title = scope_title(scope);
    let latest_href = scope_dir.join("latest").exists().then_some("latest/");

    let rows = if entries.is_empty() {
        r#"<tr><td colspan="3" class="muted">No reports yet.</td></tr>"#.to_owned()
    } else {
        entries
            .iter()
            .map(|entry| {
                format!(
                    r#"<tr><td><a href="{name}/">{name}</a></td><td>{modified}</td><td class="mono">{path}</td></tr>"#,
                    name = escape_html(&entry.name),
                    modified = escape_html(&entry.modified_label),
                    path = escape_html(&format!("{scope}/{}/", entry.name)),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title} reports</title>
  <style>
    :root {{
      color-scheme: light;
      --bg: #f5f1e8;
      --surface: rgba(255,253,248,0.94);
      --line: #d8cebe;
      --ink: #1d1a16;
      --muted: #675d51;
      --accent: #b55d2d;
      --sans: "IBM Plex Sans", "Segoe UI", sans-serif;
      --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: var(--sans);
      color: var(--ink);
      background:
        radial-gradient(circle at top left, rgba(181,93,45,0.08), transparent 28rem),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
    }}
    .page {{
      max-width: 1100px;
      margin: 0 auto;
      padding: 28px 18px 40px;
    }}
    .top {{
      display: flex;
      justify-content: space-between;
      gap: 12px;
      align-items: flex-start;
      margin-bottom: 18px;
    }}
    h1 {{ margin: 0 0 6px; font-size: 1.8rem; }}
    p {{ margin: 0; color: var(--muted); max-width: 70ch; }}
    .actions {{
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
    }}
    .actions a {{
      text-decoration: none;
      color: inherit;
      border: 1px solid var(--line);
      background: var(--surface);
      padding: 7px 11px;
      border-radius: 10px;
      font-size: 0.84rem;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      border: 1px solid var(--line);
      background: var(--surface);
      border-radius: 16px;
      overflow: hidden;
      box-shadow: 0 10px 30px rgba(39,28,18,0.06);
    }}
    th, td {{
      text-align: left;
      padding: 10px 12px;
      border-bottom: 1px solid rgba(216,206,190,0.7);
      font-size: 0.92rem;
    }}
    th {{
      color: var(--muted);
      font-size: 0.78rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }}
    .mono {{ font-family: var(--mono); }}
    .muted {{ color: var(--muted); }}
  </style>
</head>
<body>
  <div class="page">
    <div class="top">
      <div>
        <h1>{title}</h1>
        <p>Newest report directories first. Use these stable URLs instead of browsing raw artifact folders.</p>
      </div>
      <div class="actions">
        <a href="../">reports/</a>
        {latest_link}
      </div>
    </div>
    <table>
      <thead>
        <tr><th>Name</th><th>Updated</th><th>URL</th></tr>
      </thead>
      <tbody>{rows}</tbody>
    </table>
  </div>
</body>
</html>"#,
        title = escape_html(&title),
        latest_link = latest_href
            .map(|href| format!(r#"<a href="{href}">latest</a>"#))
            .unwrap_or_default(),
        rows = rows,
    );

    fs::write(scope_dir.join("index.html"), html).with_context(|| {
        format!(
            "failed to write scope report index {}",
            scope_dir.join("index.html").display()
        )
    })?;
    Ok(())
}

fn write_collection_index(collection_dir: &Path, reports_root: &Path) -> Result<()> {
    fs::create_dir_all(collection_dir).with_context(|| {
        format!(
            "failed to create collection dir {}",
            collection_dir.display()
        )
    })?;

    let entries = scope_entries(collection_dir)?;
    let relative_dir = collection_dir
        .strip_prefix(reports_root)
        .unwrap_or(collection_dir);
    let title = collection_title(relative_dir);
    let latest_href = collection_dir.join("latest").exists().then_some("latest/");
    let back_href = "../";

    let rows = if entries.is_empty() {
        r#"<tr><td colspan="3" class="muted">No reports yet.</td></tr>"#.to_owned()
    } else {
        entries
            .iter()
            .map(|entry| {
                let relative_path = relative_dir.join(&entry.name);
                format!(
                    r#"<tr><td><a href="{name}/">{name}</a></td><td>{modified}</td><td class="mono">{path}/</td></tr>"#,
                    name = escape_html(&entry.name),
                    modified = escape_html(&entry.modified_label),
                    path = escape_html(&relative_path.display().to_string()),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>
    :root {{
      color-scheme: light;
      --bg: #f5f1e8;
      --surface: rgba(255,253,248,0.94);
      --line: #d8cebe;
      --ink: #1d1a16;
      --muted: #675d51;
      --accent: #b55d2d;
      --sans: "IBM Plex Sans", "Segoe UI", sans-serif;
      --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: var(--sans);
      color: var(--ink);
      background:
        radial-gradient(circle at top left, rgba(181,93,45,0.08), transparent 28rem),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
    }}
    .page {{
      max-width: 1100px;
      margin: 0 auto;
      padding: 28px 18px 40px;
    }}
    .top {{
      display: flex;
      justify-content: space-between;
      gap: 12px;
      align-items: flex-start;
      margin-bottom: 18px;
    }}
    h1 {{ margin: 0 0 6px; font-size: 1.8rem; }}
    p {{ margin: 0; color: var(--muted); max-width: 70ch; }}
    .actions {{
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
    }}
    .actions a {{
      text-decoration: none;
      color: inherit;
      border: 1px solid var(--line);
      background: var(--surface);
      padding: 7px 11px;
      border-radius: 10px;
      font-size: 0.84rem;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      border: 1px solid var(--line);
      background: var(--surface);
      border-radius: 16px;
      overflow: hidden;
      box-shadow: 0 10px 30px rgba(39,28,18,0.06);
    }}
    th, td {{
      text-align: left;
      padding: 10px 12px;
      border-bottom: 1px solid rgba(216,206,190,0.7);
      font-size: 0.92rem;
    }}
    th {{
      color: var(--muted);
      font-size: 0.78rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }}
    .mono {{ font-family: var(--mono); }}
    .muted {{ color: var(--muted); }}
  </style>
</head>
<body>
  <div class="page">
    <div class="top">
      <div>
        <h1>{title}</h1>
        <p>Nested report collection. Use this page instead of falling back to a raw directory listing.</p>
      </div>
      <div class="actions">
        <a href="{back_href}">up</a>
        <a href="../../">reports/</a>
        {latest_link}
      </div>
    </div>
    <table>
      <thead>
        <tr><th>Name</th><th>Updated</th><th>URL</th></tr>
      </thead>
      <tbody>{rows}</tbody>
    </table>
  </div>
</body>
</html>"#,
        title = escape_html(&title),
        back_href = back_href,
        latest_link = latest_href
            .map(|href| format!(r#"<a href="{href}">latest</a>"#))
            .unwrap_or_default(),
        rows = rows,
    );

    fs::write(collection_dir.join("index.html"), html).with_context(|| {
        format!(
            "failed to write collection index {}",
            collection_dir.join("index.html").display()
        )
    })?;
    Ok(())
}

fn scope_entries(scope_dir: &Path) -> Result<Vec<ScopeEntry>> {
    let mut entries = Vec::new();
    if !scope_dir.exists() {
        return Ok(entries);
    }

    for dir_entry in fs::read_dir(scope_dir)
        .with_context(|| format!("failed to read scope dir {}", scope_dir.display()))?
    {
        let dir_entry = dir_entry?;
        let path = dir_entry.path();
        let file_name = dir_entry.file_name();
        let name = file_name.to_string_lossy().into_owned();
        if name == "latest" || name == "index.html" {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)?;
        let file_type = metadata.file_type();
        if !(metadata.is_dir() || file_type.is_symlink()) {
            continue;
        }
        let modified = entry_modified_time(&path, &metadata);
        entries.push(ScopeEntry {
            name,
            modified,
            modified_label: modified_label(modified),
        });
    }

    entries.sort_by(|lhs, rhs| {
        rhs.modified
            .cmp(&lhs.modified)
            .then(lhs.name.cmp(&rhs.name))
    });
    Ok(entries)
}

fn modified_label(modified: SystemTime) -> String {
    match modified.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => format!("unix {}", duration.as_secs()),
        Err(_) => "unknown".to_owned(),
    }
}

fn scope_title(scope: &str) -> String {
    match scope {
        "runs" => "Run reports".to_owned(),
        "replays" => "Replay reports".to_owned(),
        "eval" => "Batch reports".to_owned(),
        other => format!("{other} reports"),
    }
}

fn collection_title(relative_dir: &Path) -> String {
    let parts = relative_dir
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        "Report collection".to_owned()
    } else {
        format!("{} index", parts.join(" / "))
    }
}

fn entry_modified_time(path: &Path, metadata: &fs::Metadata) -> SystemTime {
    let report_file = path.join("index.html");
    fs::metadata(&report_file)
        .and_then(|report_metadata| report_metadata.modified())
        .unwrap_or_else(|_| metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH))
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

struct ScopeEntry {
    name: String,
    modified: SystemTime,
    modified_label: String,
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
