use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result};
use pd_control::{ControllerSpec, ControllerUpdateRecord, TelemetryValue};
use pd_core::{EventKind, EventRecord, RunManifest, SampleRecord, ScenarioSpec};
use serde::Serialize;

const PLOTLY_CDN_URL: &str = "https://cdn.plot.ly/plotly-basic-2.35.2.min.js";

pub fn write_run_report(
    path: &Path,
    scenario: &ScenarioSpec,
    controller_spec: Option<&ControllerSpec>,
    manifest: &RunManifest,
    events: &[EventRecord],
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
) -> Result<()> {
    let report_data = build_report_data(
        scenario,
        controller_spec,
        manifest,
        events,
        samples,
        controller_updates,
    );
    let html = report_template()
        .replace("__REPORT_TITLE__", &escape_html(&format!("{} report", scenario.name)))
        .replace("__PLOTLY_HREF__", PLOTLY_CDN_URL)
        .replace("__REPORT_DATA__", &json_html(&report_data));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create report output directory {}", parent.display())
        })?;
    }
    fs::write(path, html)
        .with_context(|| format!("failed to write report file {}", path.display()))?;
    Ok(())
}

fn build_report_data(
    scenario: &ScenarioSpec,
    controller_spec: Option<&ControllerSpec>,
    manifest: &RunManifest,
    events: &[EventRecord],
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
) -> ReportData {
    let report_samples = build_report_samples(samples, controller_updates);
    let report_markers = build_report_markers(&report_samples, controller_updates);
    let report_events = build_report_events(&report_samples, events);

    ReportData {
        scenario_id: scenario.id.clone(),
        scenario_name: scenario.name.clone(),
        controller_id: manifest.controller_id.clone(),
        controller_spec: controller_spec.cloned(),
        manifest: ReportManifest {
            sim_time_s: manifest.sim_time_s,
            physics_steps: manifest.physics_steps,
            controller_updates: manifest.controller_updates,
            physical_outcome: enum_label(&manifest.physical_outcome),
            mission_outcome: enum_label(&manifest.mission_outcome),
            end_reason: enum_label(&manifest.end_reason),
        },
        terrain: scenario
            .world
            .terrain
            .points()
            .iter()
            .map(|point| ReportVec2 {
                x_m: point.x,
                y_m: point.y,
            })
            .collect(),
        pad: scenario
            .world
            .landing_pad(scenario.mission.goal.target_pad_id())
            .map(|pad| ReportPad {
                id: pad.id.clone(),
                center_x_m: pad.center_x_m,
                surface_y_m: pad.surface_y_m,
                width_m: pad.width_m,
            }),
        samples: report_samples.clone(),
        events: report_events.clone(),
        markers: report_markers.clone(),
        event_counts: summarize_counts(
            report_events
                .iter()
                .map(|event| event.kind.clone())
                .collect::<Vec<_>>()
                .as_slice(),
        ),
        marker_counts: summarize_counts(
            report_markers
                .iter()
                .map(|marker| marker.label.clone())
                .collect::<Vec<_>>()
                .as_slice(),
        ),
        phase_summary: summarize_phases(&report_samples),
    }
}

fn build_report_samples(
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
) -> Vec<ReportSample> {
    let mut controller_index = 0_usize;
    let mut current_update = controller_updates.first();

    let mut report_samples = Vec::with_capacity(samples.len());
    for sample in samples {
        while controller_index + 1 < controller_updates.len()
            && controller_updates[controller_index + 1].physics_step <= sample.physics_step
        {
            controller_index += 1;
            current_update = controller_updates.get(controller_index);
        }

        let frame = current_update
            .filter(|update| update.physics_step <= sample.physics_step)
            .map(|update| &update.frame);
        let (status, phase, metrics, throttle_frac, target_attitude_rad) = match frame {
            Some(frame) => (
                frame.status.clone(),
                frame.phase.clone(),
                frame.metrics.clone(),
                frame.command.throttle_frac,
                frame.command.target_attitude_rad,
            ),
            None => (
                String::new(),
                None,
                BTreeMap::new(),
                sample.held_command.throttle_frac,
                sample.held_command.target_attitude_rad,
            ),
        };

        report_samples.push(ReportSample {
            sim_time_s: sample.sim_time_s,
            physics_step: sample.physics_step,
            x_m: sample.observation.position_m.x,
            y_m: sample.observation.position_m.y,
            vx_mps: sample.observation.velocity_mps.x,
            vy_mps: sample.observation.velocity_mps.y,
            speed_mps: sample.observation.velocity_mps.length(),
            attitude_rad: sample.observation.attitude_rad,
            attitude_deg: sample.observation.attitude_rad.to_degrees(),
            fuel_kg: sample.observation.fuel_kg,
            height_above_target_m: sample.observation.height_above_target_m,
            touchdown_clearance_m: sample.observation.touchdown_clearance_m,
            min_hull_clearance_m: sample.observation.min_hull_clearance_m,
            target_dx_m: sample.observation.target_dx_m,
            throttle_frac,
            target_attitude_rad,
            target_attitude_deg: target_attitude_rad.to_degrees(),
            status,
            phase,
            metrics,
        });
    }

    report_samples
}

fn build_report_events(samples: &[ReportSample], events: &[EventRecord]) -> Vec<ReportEvent> {
    events
        .iter()
        .filter(|event| !matches!(event.kind, EventKind::ControllerUpdated | EventKind::MissionEnded))
        .map(|event| {
            let sample = nearest_sample(samples, event.physics_step);
            ReportEvent {
                sim_time_s: event.sim_time_s,
                physics_step: event.physics_step,
                kind: enum_label(&event.kind),
                label: humanize_label(&enum_label(&event.kind)),
                message: event.message.clone(),
                x_m: sample.map(|sample| sample.x_m),
                y_m: sample.map(|sample| sample.y_m),
            }
        })
        .collect()
}

fn build_report_markers(
    samples: &[ReportSample],
    controller_updates: &[ControllerUpdateRecord],
) -> Vec<ReportMarker> {
    controller_updates
        .iter()
        .flat_map(|update| {
            update.frame.markers.iter().map(move |marker| {
                let sample = nearest_sample(samples, update.physics_step);
                ReportMarker {
                    sim_time_s: update.sim_time_s,
                    physics_step: update.physics_step,
                    label: marker.label.clone(),
                    x_m: marker.x_m.or_else(|| sample.map(|sample| sample.x_m)),
                    y_m: marker.y_m.or_else(|| sample.map(|sample| sample.y_m)),
                    phase: update.frame.phase.clone(),
                    metrics: marker.metadata.clone(),
                }
            })
        })
        .collect()
}

fn summarize_counts(values: &[String]) -> Vec<ReportCount> {
    let mut counts = BTreeMap::<String, usize>::new();
    for value in values {
        *counts.entry(value.clone()).or_insert(0) += 1;
    }

    let mut summary = counts
        .into_iter()
        .map(|(label, count)| ReportCount { label, count })
        .collect::<Vec<_>>();
    summary.sort_by(|lhs, rhs| rhs.count.cmp(&lhs.count).then(lhs.label.cmp(&rhs.label)));
    summary
}

fn summarize_phases(samples: &[ReportSample]) -> Vec<ReportPhaseSummary> {
    let mut order = Vec::<String>::new();
    let mut durations = BTreeMap::<String, f64>::new();
    let mut sample_counts = BTreeMap::<String, usize>::new();

    for (index, sample) in samples.iter().enumerate() {
        let Some(phase) = sample.phase.clone() else {
            continue;
        };
        if !durations.contains_key(&phase) {
            order.push(phase.clone());
        }
        let dt_s = samples
            .get(index + 1)
            .map(|next| (next.sim_time_s - sample.sim_time_s).max(0.0))
            .unwrap_or(0.0);
        *durations.entry(phase.clone()).or_insert(0.0) += dt_s;
        *sample_counts.entry(phase).or_insert(0) += 1;
    }

    order
        .into_iter()
        .map(|phase| ReportPhaseSummary {
            label: phase.clone(),
            duration_s: *durations.get(&phase).unwrap_or(&0.0),
            sample_count: *sample_counts.get(&phase).unwrap_or(&0),
        })
        .collect()
}

fn nearest_sample<T>(samples: &[T], physics_step: u64) -> Option<&T>
where
    T: HasPhysicsStep,
{
    samples
        .iter()
        .min_by_key(|sample| sample.physics_step().abs_diff(physics_step))
}

fn enum_label<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"unknown\"".to_owned())
        .trim_matches('"')
        .to_owned()
}

fn humanize_label(label: &str) -> String {
    label
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn json_html<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .expect("report data should serialize")
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

trait HasPhysicsStep {
    fn physics_step(&self) -> u64;
}

impl HasPhysicsStep for ReportSample {
    fn physics_step(&self) -> u64 {
        self.physics_step
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportData {
    scenario_id: String,
    scenario_name: String,
    controller_id: String,
    controller_spec: Option<ControllerSpec>,
    manifest: ReportManifest,
    terrain: Vec<ReportVec2>,
    pad: Option<ReportPad>,
    samples: Vec<ReportSample>,
    events: Vec<ReportEvent>,
    markers: Vec<ReportMarker>,
    event_counts: Vec<ReportCount>,
    marker_counts: Vec<ReportCount>,
    phase_summary: Vec<ReportPhaseSummary>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportManifest {
    sim_time_s: f64,
    physics_steps: u64,
    controller_updates: u64,
    physical_outcome: String,
    mission_outcome: String,
    end_reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportVec2 {
    x_m: f64,
    y_m: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportPad {
    id: String,
    center_x_m: f64,
    surface_y_m: f64,
    width_m: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportSample {
    sim_time_s: f64,
    physics_step: u64,
    x_m: f64,
    y_m: f64,
    vx_mps: f64,
    vy_mps: f64,
    speed_mps: f64,
    attitude_rad: f64,
    attitude_deg: f64,
    fuel_kg: f64,
    height_above_target_m: f64,
    touchdown_clearance_m: f64,
    min_hull_clearance_m: f64,
    target_dx_m: f64,
    throttle_frac: f64,
    target_attitude_rad: f64,
    target_attitude_deg: f64,
    status: String,
    phase: Option<String>,
    metrics: BTreeMap<String, TelemetryValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportEvent {
    sim_time_s: f64,
    physics_step: u64,
    kind: String,
    label: String,
    message: String,
    x_m: Option<f64>,
    y_m: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportMarker {
    sim_time_s: f64,
    physics_step: u64,
    label: String,
    x_m: Option<f64>,
    y_m: Option<f64>,
    phase: Option<String>,
    metrics: BTreeMap<String, TelemetryValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportCount {
    label: String,
    count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportPhaseSummary {
    label: String,
    duration_s: f64,
    sample_count: usize,
}

fn report_template() -> &'static str {
    r####"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>__REPORT_TITLE__</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f4f1e8;
      --panel: #fffaf0;
      --ink: #1d1f24;
      --muted: #575f66;
      --accent: #0e6b60;
      --warn: #8e3b2e;
      --success: #2f9e44;
      --line: #d8cfbf;
      --shadow: rgba(29, 31, 36, 0.08);
      --chip: rgba(14, 107, 96, 0.08);
      --chip-border: rgba(14, 107, 96, 0.18);
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: Georgia, "Times New Roman", serif;
      color: var(--ink);
      background: linear-gradient(180deg, #f7f4ec 0%, var(--bg) 100%);
    }
    main {
      max-width: 1540px;
      margin: 0 auto;
      padding: 20px 18px 30px;
    }
    section, header {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 16px;
      box-shadow: 0 8px 24px var(--shadow);
    }
    header {
      padding: 16px 18px;
      margin-bottom: 14px;
    }
    h1, h2, h3 {
      margin: 0;
      font-family: "Palatino Linotype", "Book Antiqua", Palatino, serif;
    }
    p { margin: 0; }
    .hero {
      display: grid;
      grid-template-columns: 1.6fr 1fr;
      gap: 14px;
      align-items: start;
    }
    .hero-main {
      display: grid;
      gap: 10px;
    }
    .eyebrow {
      font-size: 0.78rem;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      color: var(--muted);
    }
    .title-row {
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 12px;
      flex-wrap: wrap;
    }
    .title-row h1 {
      font-size: clamp(1.9rem, 4vw, 3rem);
      line-height: 0.98;
    }
    .banner {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 12rem;
      padding: 0.7rem 1rem;
      border-radius: 999px;
      font-weight: 700;
      text-align: center;
      background: rgba(14, 107, 96, 0.11);
      color: var(--accent);
      border: 1px solid rgba(14, 107, 96, 0.18);
    }
    .banner.failure {
      background: rgba(142, 59, 46, 0.12);
      color: var(--warn);
      border-color: rgba(142, 59, 46, 0.18);
    }
    .muted { color: var(--muted); }
    .stats {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
      gap: 10px;
    }
    .stat, .panel-block {
      border: 1px solid var(--line);
      border-radius: 12px;
      background: rgba(255, 255, 255, 0.45);
      padding: 10px 12px;
    }
    .stat .label {
      font-size: 0.78rem;
      letter-spacing: 0.05em;
      text-transform: uppercase;
      color: var(--muted);
      margin-bottom: 3px;
    }
    .stat .value {
      font-size: 1.15rem;
      font-weight: 700;
    }
    .main-grid {
      display: grid;
      grid-template-columns: minmax(0, 1.9fr) minmax(22rem, 0.95fr);
      gap: 14px;
      align-items: start;
    }
    .left-stack, .right-stack {
      display: grid;
      gap: 14px;
    }
    .right-stack {
      position: sticky;
      top: 14px;
    }
    .panel {
      padding: 14px 16px 16px;
    }
    .panel-head {
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 10px;
      margin-bottom: 10px;
    }
    .panel-head h2 {
      font-size: 1.18rem;
    }
    .plot-toolbar {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
    }
    .plot-toolbar button {
      border: 1px solid var(--line);
      background: #f7f1e4;
      color: var(--ink);
      border-radius: 999px;
      padding: 5px 12px;
      cursor: pointer;
      font: inherit;
    }
    .plot-toolbar button.active {
      background: var(--accent);
      color: #fffaf0;
      border-color: var(--accent);
    }
    .chart {
      width: 100%;
      height: 430px;
      border: 1px solid var(--line);
      border-radius: 12px;
      background: #fbf8f1;
    }
    .metric-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 14px;
    }
    .metric-grid .chart {
      height: 285px;
    }
    .chip-row {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-top: 6px;
    }
    .chip {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 6px 10px;
      border-radius: 999px;
      background: var(--chip);
      border: 1px solid var(--chip-border);
      color: var(--accent);
      font-size: 0.88rem;
      line-height: 1;
      white-space: nowrap;
    }
    .chip .count {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 1.55rem;
      height: 1.55rem;
      border-radius: 999px;
      background: rgba(14, 107, 96, 0.12);
      color: var(--accent);
      font-weight: 700;
      font-size: 0.8rem;
    }
    .summary-grid {
      display: grid;
      gap: 12px;
    }
    .compact-list {
      display: grid;
      gap: 8px;
    }
    .compact-item {
      display: grid;
      grid-template-columns: auto 1fr;
      gap: 8px 10px;
      align-items: start;
      padding: 8px 0;
      border-bottom: 1px solid rgba(216, 207, 191, 0.75);
      font-size: 0.93rem;
    }
    .compact-item:last-child {
      border-bottom: 0;
      padding-bottom: 0;
    }
    .time-pill {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 4.8rem;
      padding: 5px 8px;
      border-radius: 999px;
      background: rgba(0, 0, 0, 0.04);
      color: var(--muted);
      font-size: 0.82rem;
      font-variant-numeric: tabular-nums;
    }
    .inspect-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
      margin-top: 10px;
    }
    .inspect-card {
      border: 1px solid var(--line);
      border-radius: 10px;
      padding: 8px 10px;
      background: rgba(255, 255, 255, 0.58);
    }
    .inspect-card .label {
      font-size: 0.74rem;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      color: var(--muted);
      margin-bottom: 2px;
    }
    .inspect-card .value {
      font-weight: 700;
      font-size: 0.98rem;
      line-height: 1.2;
      font-variant-numeric: tabular-nums;
    }
    details {
      border-top: 1px solid rgba(216, 207, 191, 0.75);
      margin-top: 12px;
      padding-top: 10px;
    }
    details summary {
      cursor: pointer;
      color: var(--muted);
      font-size: 0.92rem;
    }
    pre {
      margin: 10px 0 0;
      padding: 10px 12px;
      border-radius: 12px;
      background: #23252a;
      color: #f5f0e6;
      font-size: 0.82rem;
      overflow: auto;
      max-height: 16rem;
    }
    .empty {
      color: var(--muted);
      font-style: italic;
      padding-top: 4px;
    }
    @media (max-width: 1180px) {
      .main-grid {
        grid-template-columns: 1fr;
      }
      .right-stack {
        position: static;
      }
    }
    @media (max-width: 860px) {
      .hero {
        grid-template-columns: 1fr;
      }
      .metric-grid {
        grid-template-columns: 1fr;
      }
      .inspect-grid {
        grid-template-columns: 1fr 1fr;
      }
      .chart {
        height: 360px;
      }
      .metric-grid .chart {
        height: 260px;
      }
    }
  </style>
  <script src="__PLOTLY_HREF__"></script>
</head>
<body>
  <main>
    <header>
      <div class="hero">
        <div class="hero-main">
          <div class="eyebrow">Powered Descent Lab</div>
          <div class="title-row">
            <div>
              <h1 id="scenario-title"></h1>
              <p class="muted" id="scenario-subtitle"></p>
            </div>
            <div class="banner" id="outcome-banner"></div>
          </div>
          <div class="stats">
            <div class="stat">
              <div class="label">Controller</div>
              <div class="value" id="controller-id"></div>
            </div>
            <div class="stat">
              <div class="label">Mission</div>
              <div class="value" id="mission-outcome"></div>
            </div>
            <div class="stat">
              <div class="label">Physical</div>
              <div class="value" id="physical-outcome"></div>
            </div>
            <div class="stat">
              <div class="label">End Reason</div>
              <div class="value" id="end-reason"></div>
            </div>
          </div>
        </div>
        <div class="summary-grid">
          <div class="panel-block">
            <div class="eyebrow">Run</div>
            <div class="stats">
              <div class="stat">
                <div class="label">Sim Time</div>
                <div class="value" id="sim-time"></div>
              </div>
              <div class="stat">
                <div class="label">Physics Steps</div>
                <div class="value" id="physics-steps"></div>
              </div>
              <div class="stat">
                <div class="label">Control Updates</div>
                <div class="value" id="controller-updates"></div>
              </div>
              <div class="stat">
                <div class="label">Samples</div>
                <div class="value" id="sample-count"></div>
              </div>
            </div>
          </div>
          <div class="panel-block">
            <div class="eyebrow">Phases</div>
            <div class="chip-row" id="phase-chips"></div>
          </div>
        </div>
      </div>
    </header>

    <section class="main-grid">
      <div class="left-stack">
        <section class="panel">
          <div class="panel-head">
            <div>
              <div class="eyebrow">Spatial</div>
              <h2>Trajectory</h2>
            </div>
            <div class="plot-toolbar" id="spatial-mode-toolbar">
              <button type="button" data-mode="plain" class="active">Plain</button>
              <button type="button" data-mode="speed">Speed</button>
              <button type="button" data-mode="throttle">Throttle</button>
              <button type="button" data-mode="vectors">Vectors</button>
            </div>
          </div>
          <div id="chart-spatial" class="chart"></div>
        </section>

        <section class="metric-grid">
          <section class="panel">
            <div class="panel-head">
              <div>
                <div class="eyebrow">Flight State</div>
                <h2>Altitude And Velocity</h2>
              </div>
            </div>
            <div id="chart-state" class="chart"></div>
          </section>
          <section class="panel">
            <div class="panel-head">
              <div>
                <div class="eyebrow">Control</div>
                <h2>Throttle And Attitude</h2>
              </div>
            </div>
            <div id="chart-control" class="chart"></div>
          </section>
        </section>
      </div>

      <div class="right-stack">
        <section class="panel">
          <div class="panel-head">
            <div>
              <div class="eyebrow">Inspect</div>
              <h2>Hovered Sample</h2>
            </div>
          </div>
          <p class="muted" id="inspect-caption">Hover the trajectory or charts to inspect a sample.</p>
          <div class="inspect-grid" id="inspect-grid"></div>
          <details>
            <summary>Controller metrics at hovered sample</summary>
            <pre id="hover-metrics">{"message":"hover a sample"}</pre>
          </details>
        </section>

        <section class="panel">
          <div class="panel-head">
            <div>
              <div class="eyebrow">Events</div>
              <h2>What Happened</h2>
            </div>
          </div>
          <div class="chip-row" id="event-chips"></div>
          <div class="compact-list" id="event-list"></div>
        </section>

        <section class="panel">
          <div class="panel-head">
            <div>
              <div class="eyebrow">Controller</div>
              <h2>Markers And Config</h2>
            </div>
          </div>
          <div class="chip-row" id="marker-chips"></div>
          <details>
            <summary>Controller config</summary>
            <pre id="controller-spec"></pre>
          </details>
        </section>
      </div>
    </section>
  </main>

  <script>
    const reportData = __REPORT_DATA__;
    const paperBg = "#fffaf0";
    const plotBg = "#fbf8f1";
    const baseConfig = { responsive: true, displaylogo: false };

    const fmt = (value, digits = 2) =>
      Number.isFinite(Number(value)) ? Number(value).toFixed(digits) : "n/a";

    const setText = (id, value) => {
      const node = document.getElementById(id);
      if (node) node.textContent = value;
    };

    const valueExtent = (values) => {
      const finite = values.map((value) => Number(value)).filter((value) => Number.isFinite(value));
      if (!finite.length) return { min: 0, max: 1 };
      const min = Math.min(...finite);
      const max = Math.max(...finite);
      return max > min ? { min, max } : { min, max: min + 1 };
    };

    const clamp01 = (value) => Math.max(0, Math.min(1, value));
    const hexToRgb = (hex) => {
      const normalized = String(hex || "").replace("#", "");
      if (normalized.length !== 6) return [0, 0, 0];
      return [
        Number.parseInt(normalized.slice(0, 2), 16),
        Number.parseInt(normalized.slice(2, 4), 16),
        Number.parseInt(normalized.slice(4, 6), 16),
      ];
    };
    const rgbToHex = (rgb) =>
      "#" + rgb.map((value) => Math.round(value).toString(16).padStart(2, "0")).join("");
    const interpolateColor = (scale, value, minValue, maxValue) => {
      const safeMin = Number.isFinite(minValue) ? minValue : 0;
      const safeMax = Number.isFinite(maxValue) && maxValue > safeMin ? maxValue : safeMin + 1;
      const t = clamp01((Number(value) - safeMin) / (safeMax - safeMin));
      for (let index = 1; index < scale.length; index += 1) {
        const [stopB, colorB] = scale[index];
        if (t > stopB) continue;
        const [stopA, colorA] = scale[index - 1];
        const localT = stopB <= stopA ? 0 : (t - stopA) / (stopB - stopA);
        const rgbA = hexToRgb(colorA);
        const rgbB = hexToRgb(colorB);
        return rgbToHex(rgbA.map((channel, rgbIndex) => channel + ((rgbB[rgbIndex] - channel) * localT)));
      }
      return scale[scale.length - 1][1];
    };

    const samples = Array.isArray(reportData.samples) ? reportData.samples : [];
    const terrain = Array.isArray(reportData.terrain) ? reportData.terrain : [];
    const keyEvents = Array.isArray(reportData.events) ? reportData.events : [];
    const markers = Array.isArray(reportData.markers) ? reportData.markers : [];
    const pad = reportData.pad || null;

    const xValues = samples.map((sample) => Number(sample.xM));
    const yValues = samples.map((sample) => Number(sample.yM));
    const timeValues = samples.map((sample) => Number(sample.simTimeS));
    const speedValues = samples.map((sample) => Number(sample.speedMps));
    const vxValues = samples.map((sample) => Number(sample.vxMps));
    const vyValues = samples.map((sample) => Number(sample.vyMps));
    const altitudeValues = samples.map((sample) => Number(sample.heightAboveTargetM));
    const touchdownClearanceValues = samples.map((sample) => Number(sample.touchdownClearanceM));
    const hullClearanceValues = samples.map((sample) => Number(sample.minHullClearanceM));
    const throttleValues = samples.map((sample) => Number(sample.throttleFrac));
    const attitudeValues = samples.map((sample) => Number(sample.attitudeDeg));
    const targetAttitudeValues = samples.map((sample) => Number(sample.targetAttitudeDeg));
    const fuelValues = samples.map((sample) => Number(sample.fuelKg));
    const thrustXValues = samples.map((sample) => Number(sample.throttleFrac) * Math.sin(Number(sample.attitudeRad || 0)));
    const thrustYValues = samples.map((sample) => Number(sample.throttleFrac) * Math.cos(Number(sample.attitudeRad || 0)));

    const speedColorScale = [
      [0.0, "#fff3bf"],
      [0.35, "#ffd166"],
      [0.65, "#f77f00"],
      [1.0, "#c1121f"],
    ];
    const throttleColorScale = [
      [0.0, "#d9f0ff"],
      [0.35, "#74c0fc"],
      [0.7, "#2b8aeb"],
      [1.0, "#0b3d91"],
    ];

    const layoutBase = (title, extra = {}) => Object.assign({
      title,
      paper_bgcolor: paperBg,
      plot_bgcolor: plotBg,
      margin: { l: 58, r: 50, t: 54, b: 38 },
      legend: {
        orientation: "h",
        yanchor: "bottom",
        y: 1.02,
        xanchor: "left",
        x: 0,
        bgcolor: "rgba(255,250,240,0.92)",
      },
      hoverlabel: {
        bgcolor: "#fffaf0",
        bordercolor: "#d8cfbf",
        font: { family: "Georgia, serif", size: 12, color: "#1d1f24" },
      },
    }, extra);

    const summarizeOutcome = () => {
      setText("scenario-title", reportData.scenarioName);
      setText(
        "scenario-subtitle",
        `${reportData.scenarioId} · ${samples.length} sampled states · ${markers.length} controller markers`
      );
      setText("controller-id", reportData.controllerId);
      setText("mission-outcome", reportData.manifest.missionOutcome);
      setText("physical-outcome", reportData.manifest.physicalOutcome);
      setText("end-reason", reportData.manifest.endReason);
      setText("sim-time", `${fmt(reportData.manifest.simTimeS, 2)} s`);
      setText("physics-steps", String(reportData.manifest.physicsSteps));
      setText("controller-updates", String(reportData.manifest.controllerUpdates));
      setText("sample-count", String(samples.length));

      const banner = document.getElementById("outcome-banner");
      const isFailure = String(reportData.manifest.missionOutcome || "").startsWith("failed");
      banner.textContent = `${reportData.manifest.missionOutcome} · ${reportData.manifest.endReason}`;
      banner.classList.toggle("failure", isFailure);
    };

    const renderCountChips = (targetId, counts, emptyLabel) => {
      const root = document.getElementById(targetId);
      root.innerHTML = "";
      if (!Array.isArray(counts) || !counts.length) {
        const empty = document.createElement("div");
        empty.className = "empty";
        empty.textContent = emptyLabel;
        root.appendChild(empty);
        return;
      }
      counts.forEach((item) => {
        const chip = document.createElement("span");
        chip.className = "chip";
        chip.innerHTML = `<span>${item.label}</span><span class="count">${item.count}</span>`;
        root.appendChild(chip);
      });
    };

    const renderPhaseChips = () => {
      const root = document.getElementById("phase-chips");
      root.innerHTML = "";
      if (!Array.isArray(reportData.phaseSummary) || !reportData.phaseSummary.length) {
        root.innerHTML = '<div class="empty">No controller phases recorded.</div>';
        return;
      }
      reportData.phaseSummary.forEach((phase) => {
        const chip = document.createElement("span");
        chip.className = "chip";
        chip.innerHTML = `<span>${phase.label}</span><span class="count">${fmt(phase.durationS, 1)}s</span>`;
        root.appendChild(chip);
      });
    };

    const renderEventList = () => {
      const root = document.getElementById("event-list");
      root.innerHTML = "";
      if (!keyEvents.length) {
        root.innerHTML = '<div class="empty">No key events beyond steady controller updates.</div>';
        return;
      }
      keyEvents.forEach((event) => {
        const item = document.createElement("div");
        item.className = "compact-item";
        item.innerHTML = `
          <div class="time-pill">${fmt(event.simTimeS, 2)}s</div>
          <div>
            <strong>${event.label}</strong>
            <div class="muted">${event.message || event.kind}</div>
          </div>
        `;
        root.appendChild(item);
      });
    };

    const renderControllerSpec = () => {
      setText(
        "controller-spec",
        reportData.controllerSpec
          ? JSON.stringify(reportData.controllerSpec, null, 2)
          : "No controller config artifact captured."
      );
    };

    const spatialBounds = () => {
      const xs = [];
      const ys = [];
      terrain.forEach((point) => {
        xs.push(Number(point.xM));
        ys.push(Number(point.yM));
      });
      samples.forEach((sample) => {
        xs.push(Number(sample.xM));
        ys.push(Number(sample.yM));
      });
      if (pad) {
        xs.push(Number(pad.centerXM) - (Number(pad.widthM) / 2));
        xs.push(Number(pad.centerXM) + (Number(pad.widthM) / 2));
        ys.push(Number(pad.surfaceYM));
      }
      if (!xs.length || !ys.length) {
        return { span: 1 };
      }
      const minX = Math.min(...xs);
      const maxX = Math.max(...xs);
      const minY = Math.min(...ys);
      const maxY = Math.max(...ys);
      return {
        span: Math.max(maxX - minX, maxY - minY, 1),
      };
    };

    const buildHoverCarrier = () => ({
      type: "scatter",
      mode: "lines",
      name: "hover-carrier",
      x: xValues,
      y: yValues,
      customdata: samples.map((sample, index) => [index]),
      line: { color: "rgba(14,107,96,0.002)", width: 18 },
      hovertemplate:
        "t=%{customdata[0]}<extra></extra>",
      showlegend: false,
    });

    const buildScalarSpatialTraces = ({ values, colorscale, colorbarTitle }) => {
      const traces = [{
        type: "scatter",
        mode: "lines",
        x: xValues,
        y: yValues,
        line: { color: "#d6cdbd", width: 2 },
        hoverinfo: "skip",
        showlegend: false,
      }];
      const extent = valueExtent(values);
      for (let index = 1; index < Math.min(xValues.length, yValues.length, values.length); index += 1) {
        const x0 = Number(xValues[index - 1]);
        const y0 = Number(yValues[index - 1]);
        const x1 = Number(xValues[index]);
        const y1 = Number(yValues[index]);
        const value0 = Number(values[index - 1]);
        const value1 = Number(values[index]);
        if (![x0, y0, x1, y1, value0, value1].every((value) => Number.isFinite(value))) continue;
        traces.push({
          type: "scatter",
          mode: "lines",
          x: [x0, x1],
          y: [y0, y1],
          line: {
            color: interpolateColor(colorscale, 0.5 * (value0 + value1), extent.min, extent.max),
            width: 4,
          },
          hoverinfo: "skip",
          showlegend: false,
        });
      }
      traces.push({
        type: "scatter",
        mode: "markers",
        x: xValues,
        y: yValues,
        marker: {
          size: traces.length > 1 ? 0.1 : 6,
          opacity: traces.length > 1 ? 0.001 : 0.8,
          color: values,
          cmin: extent.min,
          cmax: extent.max,
          colorscale,
          colorbar: {
            title: colorbarTitle,
            outlinecolor: "#d8cfbf",
            len: 0.72,
            thickness: 12,
            x: 0.98,
            xanchor: "left",
          },
        },
        hoverinfo: "skip",
        showlegend: false,
      });
      return traces;
    };

    const buildVectorAnnotations = () => {
      const annotations = [];
      const span = spatialBounds().span;
      const vectorScale = 0.06 * span;
      let nextTarget = null;
      for (let index = 0; index < samples.length; index += 1) {
        const timeValue = Number(samples[index].simTimeS);
        if (!Number.isFinite(timeValue)) continue;
        if (nextTarget === null) nextTarget = timeValue;
        if (timeValue + 1e-9 < nextTarget) continue;
        const throttle = Number(samples[index].throttleFrac);
        const attitudeRad = Number(samples[index].attitudeRad);
        const x0 = Number(samples[index].xM);
        const y0 = Number(samples[index].yM);
        if (![throttle, attitudeRad, x0, y0].every((value) => Number.isFinite(value))) continue;
        const dx = Math.sin(attitudeRad) * throttle * vectorScale;
        const dy = Math.cos(attitudeRad) * throttle * vectorScale;
        annotations.push({
          x: x0 + dx,
          y: y0 + dy,
          ax: x0,
          ay: y0,
          xref: "x",
          yref: "y",
          axref: "x",
          ayref: "y",
          text: "",
          showarrow: true,
          arrowhead: 3,
          arrowsize: 0.6,
          arrowwidth: 2.5,
          arrowcolor: interpolateColor(throttleColorScale, throttle, 0.0, 1.0),
        });
        nextTarget += 1.0;
      }
      return annotations;
    };

    const eventStyle = (kind) => {
      const normalized = String(kind || "").toLowerCase();
      if (normalized.includes("touchdown") || normalized.includes("satisfied")) {
        return { color: "#2f9e44", symbol: "star" };
      }
      if (normalized.includes("crash") || normalized.includes("failed")) {
        return { color: "#c92a2a", symbol: "x" };
      }
      if (normalized.includes("time")) {
        return { color: "#b26b00", symbol: "triangle-down" };
      }
      return { color: "#8e3b2e", symbol: "diamond" };
    };

    const buildSpatialPlot = () => {
      const terrainTrace = {
        type: "scatter",
        mode: "lines",
        name: "terrain",
        x: terrain.map((point) => Number(point.xM)),
        y: terrain.map((point) => Number(point.yM)),
        line: { color: "#6c614d", width: 2 },
        hoverinfo: "skip",
      };
      const padTrace = pad ? {
        type: "scatter",
        mode: "lines",
        name: "target pad",
        x: [Number(pad.centerXM) - (Number(pad.widthM) / 2), Number(pad.centerXM) + (Number(pad.widthM) / 2)],
        y: [Number(pad.surfaceYM), Number(pad.surfaceYM)],
        line: { color: "#2f9e44", width: 6 },
        hoverinfo: "skip",
      } : null;
      const plainTrace = {
        type: "scatter",
        mode: "lines",
        name: "trajectory",
        x: xValues,
        y: yValues,
        line: { color: "#0e6b60", width: 3 },
        hoverinfo: "skip",
      };
      const hoverTrace = {
        ...buildHoverCarrier(),
        customdata: samples.map((sample, index) => [index, Number(sample.simTimeS), Number(sample.speedMps), sample.phase || "", sample.status || ""]),
        hovertemplate:
          "t=%{customdata[1]:.2f}s<br>x=%{x:.1f}<br>y=%{y:.1f}<br>speed=%{customdata[2]:.2f}<br>phase=%{customdata[3]}<br>%{customdata[4]}<extra></extra>",
      };
      const eventTrace = {
        type: "scatter",
        mode: "markers",
        name: "events",
        x: keyEvents.map((event) => Number(event.xM)),
        y: keyEvents.map((event) => Number(event.yM)),
        customdata: keyEvents.map((event) => [event.label, Number(event.simTimeS), event.message || event.kind]),
        hovertemplate: "%{customdata[0]}<br>%{customdata[2]}<br>t=%{customdata[1]:.2f}s<extra></extra>",
        marker: {
          size: 14,
          color: keyEvents.map((event) => eventStyle(event.kind).color),
          symbol: keyEvents.map((event) => eventStyle(event.kind).symbol),
          line: { width: 1.8, color: "#fffaf0" },
        },
      };
      const markerTrace = {
        type: "scatter",
        mode: "markers",
        name: "controller markers",
        x: markers.map((marker) => Number(marker.xM)),
        y: markers.map((marker) => Number(marker.yM)),
        customdata: markers.map((marker) => [marker.label, Number(marker.simTimeS), marker.phase || ""]),
        hovertemplate: "%{customdata[0]}<br>phase=%{customdata[2]}<br>t=%{customdata[1]:.2f}s<extra></extra>",
        marker: {
          size: 10,
          color: "#4c6ef5",
          symbol: "diamond",
          line: { width: 1.5, color: "#fffaf0" },
        },
      };

      const speedTraces = buildScalarSpatialTraces({
        values: speedValues,
        colorscale: speedColorScale,
        colorbarTitle: "speed",
      });
      const throttleTraces = buildScalarSpatialTraces({
        values: throttleValues,
        colorscale: throttleColorScale,
        colorbarTitle: "throttle",
      });
      const vectorAnnotations = buildVectorAnnotations();
      const spatialTraces = [
        terrainTrace,
        ...(padTrace ? [padTrace] : []),
        plainTrace,
        ...speedTraces,
        ...throttleTraces,
        markerTrace,
        eventTrace,
        hoverTrace,
      ];
      const plainEnd = (padTrace ? 3 : 2);
      const speedStart = plainEnd;
      const speedEnd = speedStart + speedTraces.length;
      const throttleStart = speedEnd;
      const throttleEnd = throttleStart + throttleTraces.length;
      const markerIndex = throttleEnd;
      const eventIndex = markerIndex + 1;
      const hoverIndex = eventIndex + 1;

      const visibilityForMode = (mode) =>
        spatialTraces.map((_trace, index) => {
          const alwaysVisible = index === 0 || (padTrace && index === 1) || index === markerIndex || index === eventIndex || index === hoverIndex;
          if (alwaysVisible) return true;
          if (mode === "speed") return index >= speedStart && index < speedEnd;
          if (mode === "throttle") return index >= throttleStart && index < throttleEnd;
          if (mode === "vectors") return index === plainEnd - 1;
          return index === plainEnd - 1;
        });

      const spatialElement = document.getElementById("chart-spatial");
      Plotly.newPlot(
        spatialElement,
        spatialTraces.map((trace, index) => ({ ...trace, visible: visibilityForMode("plain")[index] })),
        layoutBase("Trajectory", {
          hovermode: "closest",
          hoverdistance: 32,
          xaxis: { title: "" },
          yaxis: { title: "", scaleanchor: "x", scaleratio: 1 },
          annotations: [],
        }),
        baseConfig,
      );

      const toolbar = document.getElementById("spatial-mode-toolbar");
      const applyMode = (mode) => {
        for (const button of toolbar.querySelectorAll("button[data-mode]")) {
          button.classList.toggle("active", button.dataset.mode === mode);
        }
        Plotly.update(
          spatialElement,
          { visible: visibilityForMode(mode) },
          { title: mode === "speed" ? "Trajectory (speed-colored)" : mode === "throttle" ? "Trajectory (throttle-colored)" : mode === "vectors" ? "Trajectory (thrust vectors)" : "Trajectory", annotations: mode === "vectors" ? vectorAnnotations : [] }
        );
      };
      for (const button of toolbar.querySelectorAll("button[data-mode]")) {
        button.addEventListener("click", () => applyMode(button.dataset.mode || "plain"));
      }

      spatialElement.on("plotly_hover", (eventData) => {
        const points = Array.isArray(eventData?.points) ? eventData.points : [];
        const hoverPoint = points.find((point) => point.curveNumber === hoverIndex);
        if (!hoverPoint || !Array.isArray(hoverPoint.customdata)) return;
        updateInspect(Number(hoverPoint.customdata[0]));
      });
    };

    const buildStatePlot = () => {
      const eventGuideShapes = keyEvents
        .map((event) => {
          const timeValue = Number(event.simTimeS);
          if (!Number.isFinite(timeValue)) return null;
          return {
            type: "line",
            xref: "x",
            yref: "paper",
            x0: timeValue,
            x1: timeValue,
            y0: 0,
            y1: 1,
            line: { color: eventStyle(event.kind).color, width: 1.2, dash: "dot" },
          };
        })
        .filter(Boolean);

      Plotly.newPlot(
        "chart-state",
        [
          { type: "scatter", mode: "lines", name: "height", x: timeValues, y: altitudeValues, line: { color: "#d97706", width: 3 } },
          { type: "scatter", mode: "lines", name: "touchdown clearance", x: timeValues, y: touchdownClearanceValues, line: { color: "#0e6b60", width: 2.5 } },
          { type: "scatter", mode: "lines", name: "hull clearance", x: timeValues, y: hullClearanceValues, line: { color: "#8a6d3b", width: 2, dash: "dot" }, visible: "legendonly" },
          { type: "scatter", mode: "lines", name: "speed", x: timeValues, y: speedValues, line: { color: "#3f6ad8", width: 3 }, yaxis: "y2" },
          { type: "scatter", mode: "lines", name: "vx", x: timeValues, y: vxValues, line: { color: "#8e3b2e", width: 2 }, yaxis: "y2", visible: "legendonly" },
          { type: "scatter", mode: "lines", name: "vy", x: timeValues, y: vyValues, line: { color: "#6d28d9", width: 2 }, yaxis: "y2", visible: "legendonly" },
        ],
        layoutBase("Altitude And Velocity", {
          hovermode: "x unified",
          xaxis: { title: "Time (s)" },
          yaxis: { title: "Meters", zeroline: true },
          yaxis2: { title: "Meters / sec", overlaying: "y", side: "right", zeroline: true },
          shapes: eventGuideShapes,
        }),
        baseConfig,
      );

      const element = document.getElementById("chart-state");
      element.on("plotly_hover", (eventData) => {
        const points = Array.isArray(eventData?.points) ? eventData.points : [];
        const point = points.find((candidate) => Number.isInteger(candidate.pointIndex));
        if (!point) return;
        updateInspect(Number(point.pointIndex));
      });
    };

    const buildControlPlot = () => {
      const eventGuideShapes = keyEvents
        .map((event) => {
          const timeValue = Number(event.simTimeS);
          if (!Number.isFinite(timeValue)) return null;
          return {
            type: "line",
            xref: "x",
            yref: "paper",
            x0: timeValue,
            x1: timeValue,
            y0: 0,
            y1: 1,
            line: { color: eventStyle(event.kind).color, width: 1.2, dash: "dot" },
          };
        })
        .filter(Boolean);

      Plotly.newPlot(
        "chart-control",
        [
          { type: "scatter", mode: "lines", name: "attitude deg", x: timeValues, y: attitudeValues, line: { color: "#d97706", width: 3 } },
          { type: "scatter", mode: "lines", name: "target deg", x: timeValues, y: targetAttitudeValues, line: { color: "#4c6ef5", width: 2.5, dash: "dash" } },
          { type: "scatter", mode: "lines", name: "throttle", x: timeValues, y: throttleValues, line: { color: "#0e6b60", width: 3 }, yaxis: "y2" },
          { type: "scatter", mode: "lines", name: "fuel", x: timeValues, y: fuelValues, line: { color: "#8a6d3b", width: 2, dash: "dot" }, yaxis: "y3", visible: "legendonly" },
        ],
        layoutBase("Throttle And Attitude", {
          hovermode: "x unified",
          xaxis: { title: "Time (s)" },
          yaxis: { title: "Degrees", zeroline: true },
          yaxis2: { title: "Throttle", overlaying: "y", side: "right", range: [0, 1.05], zeroline: true },
          yaxis3: { title: "Fuel kg", overlaying: "y", side: "right", position: 0.95, showgrid: false, visible: false },
          shapes: eventGuideShapes,
        }),
        baseConfig,
      );

      const element = document.getElementById("chart-control");
      element.on("plotly_hover", (eventData) => {
        const points = Array.isArray(eventData?.points) ? eventData.points : [];
        const point = points.find((candidate) => Number.isInteger(candidate.pointIndex));
        if (!point) return;
        updateInspect(Number(point.pointIndex));
      });
    };

    const inspectFields = (sample) => [
      ["Time", `${fmt(sample.simTimeS, 2)} s`],
      ["Phase", sample.phase || "n/a"],
      ["Status", sample.status || "n/a"],
      ["Position", `${fmt(sample.xM)} m, ${fmt(sample.yM)} m`],
      ["Velocity", `${fmt(sample.vxMps)} m/s, ${fmt(sample.vyMps)} m/s`],
      ["Speed", `${fmt(sample.speedMps)} m/s`],
      ["Altitude", `${fmt(sample.heightAboveTargetM)} m`],
      ["Clearance", `${fmt(sample.touchdownClearanceM)} m`],
      ["Target dx", `${fmt(sample.targetDxM)} m`],
      ["Throttle", `${fmt(sample.throttleFrac * 100, 1)} %`],
      ["Attitude", `${fmt(sample.attitudeDeg, 1)} deg`],
      ["Fuel", `${fmt(sample.fuelKg)} kg`],
    ];

    const updateInspect = (index) => {
      const sample = samples[Math.max(0, Math.min(Number(index) || 0, samples.length - 1))];
      if (!sample) return;
      setText(
        "inspect-caption",
        `Sample ${sample.physicsStep} at ${fmt(sample.simTimeS, 2)} s${sample.phase ? ` · ${sample.phase}` : ""}`
      );
      const grid = document.getElementById("inspect-grid");
      grid.innerHTML = "";
      inspectFields(sample).forEach(([label, value]) => {
        const card = document.createElement("div");
        card.className = "inspect-card";
        card.innerHTML = `<div class="label">${label}</div><div class="value">${value}</div>`;
        grid.appendChild(card);
      });
      setText("hover-metrics", JSON.stringify(sample.metrics, null, 2));
    };

    const init = () => {
      summarizeOutcome();
      renderCountChips("event-chips", reportData.eventCounts, "No key events recorded.");
      renderCountChips("marker-chips", reportData.markerCounts, "No controller markers recorded.");
      renderPhaseChips();
      renderEventList();
      renderControllerSpec();
      buildSpatialPlot();
      buildStatePlot();
      buildControlPlot();
      updateInspect(samples.length ? samples.length - 1 : 0);
    };

    init();
  </script>
</body>
</html>
"####
}
