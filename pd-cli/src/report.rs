use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result};
use pd_control::{ControllerSpec, ControllerUpdateRecord, TelemetryValue};
use pd_core::{EventRecord, RunManifest, SampleRecord, ScenarioSpec};
use serde::Serialize;

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
    let data_json = serde_json::to_string(&report_data)?;
    let html = report_template()
        .replace("__REPORT_TITLE__", &escape_html(&format!("{} report", scenario.name)))
        .replace("__REPORT_DATA__", &data_json);
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
    let report_events = events
        .iter()
        .map(|event| {
            let sample = nearest_sample(samples, event.physics_step);
            ReportEvent {
                sim_time_s: event.sim_time_s,
                physics_step: event.physics_step,
                kind: enum_label(&event.kind),
                message: event.message.clone(),
                x_m: sample.map(|sample| sample.observation.position_m.x),
                y_m: sample.map(|sample| sample.observation.position_m.y),
            }
        })
        .collect();
    let report_markers = controller_updates
        .iter()
        .flat_map(|update| {
            update.frame.markers.iter().map(move |marker| ReportMarker {
                sim_time_s: update.sim_time_s,
                physics_step: update.physics_step,
                label: marker.label.clone(),
                x_m: marker
                    .x_m
                    .or_else(|| nearest_sample(samples, update.physics_step).map(|s| s.observation.position_m.x)),
                y_m: marker
                    .y_m
                    .or_else(|| nearest_sample(samples, update.physics_step).map(|s| s.observation.position_m.y)),
                phase: update.frame.phase.clone(),
                metrics: marker.metadata.clone(),
            })
        })
        .collect();

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
        samples: report_samples,
        events: report_events,
        markers: report_markers,
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

fn nearest_sample(samples: &[SampleRecord], physics_step: u64) -> Option<&SampleRecord> {
    samples.iter().min_by_key(|sample| sample.physics_step.abs_diff(physics_step))
}

fn enum_label<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"unknown\"".to_owned())
        .trim_matches('"')
        .to_owned()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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

fn report_template() -> &'static str {
    r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>__REPORT_TITLE__</title>
  <style>
    :root {
      --bg: #f5efe4;
      --panel: #fff9ee;
      --ink: #1f1f1b;
      --muted: #5d5a52;
      --accent: #d7693d;
      --accent-2: #2f6a63;
      --accent-3: #b3245a;
      --line: #d6c9b0;
      --terrain: #7d5f3b;
      --grid: rgba(60, 53, 38, 0.12);
    }

    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: "Iowan Old Style", "Palatino Linotype", serif;
      background:
        radial-gradient(circle at top left, rgba(215, 105, 61, 0.14), transparent 32rem),
        linear-gradient(180deg, #faf4ea 0%, var(--bg) 100%);
      color: var(--ink);
    }
    main {
      max-width: 1280px;
      margin: 0 auto;
      padding: 2rem 1.2rem 3rem;
    }
    h1, h2, h3 { margin: 0; font-weight: 600; }
    p { margin: 0; color: var(--muted); }
    .hero {
      display: grid;
      gap: 1rem;
      grid-template-columns: 2fr 1fr;
      margin-bottom: 1.5rem;
    }
    .panel {
      background: rgba(255, 249, 238, 0.9);
      border: 1px solid rgba(125, 95, 59, 0.2);
      border-radius: 1rem;
      padding: 1rem 1.1rem;
      box-shadow: 0 0.8rem 2.5rem rgba(53, 44, 29, 0.08);
      backdrop-filter: blur(8px);
    }
    .hero h1 { font-size: clamp(2rem, 5vw, 3.2rem); line-height: 1; }
    .eyebrow {
      display: inline-block;
      margin-bottom: 0.75rem;
      letter-spacing: 0.12em;
      text-transform: uppercase;
      font-size: 0.78rem;
      color: var(--accent-2);
    }
    .stats {
      display: grid;
      gap: 0.8rem;
      grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
      margin-top: 1rem;
    }
    .stat {
      background: rgba(255,255,255,0.54);
      border: 1px solid rgba(125,95,59,0.18);
      border-radius: 0.8rem;
      padding: 0.75rem;
    }
    .stat strong {
      display: block;
      font-size: 1.1rem;
      margin-top: 0.25rem;
      color: var(--ink);
    }
    .layout {
      display: grid;
      gap: 1rem;
      grid-template-columns: minmax(0, 2fr) minmax(18rem, 0.95fr);
    }
    .stack {
      display: grid;
      gap: 1rem;
    }
    .chart-grid {
      display: grid;
      gap: 1rem;
      grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
    }
    .chart-shell svg, #trajectory {
      width: 100%;
      height: auto;
      display: block;
      border-radius: 0.75rem;
      background:
        linear-gradient(180deg, rgba(255,255,255,0.82), rgba(248,242,231,0.88));
      border: 1px solid rgba(125,95,59,0.14);
    }
    .inspect {
      display: grid;
      gap: 0.85rem;
    }
    #sample-slider { width: 100%; accent-color: var(--accent); }
    .sample-grid {
      display: grid;
      gap: 0.6rem;
      grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
    }
    .sample-card {
      padding: 0.7rem;
      border-radius: 0.7rem;
      background: rgba(255,255,255,0.65);
      border: 1px solid rgba(125,95,59,0.14);
    }
    .sample-card span {
      display: block;
      color: var(--muted);
      font-size: 0.8rem;
      margin-bottom: 0.2rem;
    }
    .tag-row {
      display: flex;
      gap: 0.45rem;
      flex-wrap: wrap;
      margin-top: 0.75rem;
    }
    .tag {
      padding: 0.32rem 0.55rem;
      border-radius: 999px;
      background: rgba(47,106,99,0.11);
      color: var(--accent-2);
      border: 1px solid rgba(47,106,99,0.18);
      font-size: 0.84rem;
    }
    details pre {
      margin: 0.75rem 0 0;
      overflow: auto;
      padding: 0.8rem;
      border-radius: 0.75rem;
      background: rgba(37, 33, 28, 0.93);
      color: #f6f1e9;
      font-size: 0.82rem;
    }
    .empty {
      color: var(--muted);
      font-style: italic;
      padding: 1rem 0;
    }
    @media (max-width: 980px) {
      .hero, .layout { grid-template-columns: 1fr; }
    }
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <div class="panel">
        <span class="eyebrow">Powered Descent Lab</span>
        <h1 id="scenario-title"></h1>
        <p id="scenario-subtitle"></p>
        <div class="stats">
          <div class="stat"><span>Controller</span><strong id="controller-id"></strong></div>
          <div class="stat"><span>Mission</span><strong id="mission-outcome"></strong></div>
          <div class="stat"><span>Physical</span><strong id="physical-outcome"></strong></div>
          <div class="stat"><span>End Reason</span><strong id="end-reason"></strong></div>
        </div>
      </div>
      <div class="panel">
        <span class="eyebrow">Run Summary</span>
        <div class="stats">
          <div class="stat"><span>Sim Time</span><strong id="sim-time"></strong></div>
          <div class="stat"><span>Physics Steps</span><strong id="physics-steps"></strong></div>
          <div class="stat"><span>Control Updates</span><strong id="controller-updates"></strong></div>
        </div>
        <div class="tag-row" id="report-tags"></div>
      </div>
    </section>

    <section class="layout">
      <div class="stack">
        <div class="panel chart-shell">
          <span class="eyebrow">Trajectory</span>
          <svg id="trajectory" viewBox="0 0 960 320" preserveAspectRatio="xMidYMid meet"></svg>
        </div>

        <div class="panel chart-shell">
          <span class="eyebrow">Inspect</span>
          <div class="inspect">
            <input id="sample-slider" type="range" min="0" max="0" value="0">
            <div id="sample-summary" class="tag-row"></div>
            <div class="sample-grid" id="sample-grid"></div>
            <details>
              <summary>Controller Metrics</summary>
              <pre id="metrics-json"></pre>
            </details>
          </div>
        </div>

        <div class="chart-grid">
          <div class="panel chart-shell">
            <span class="eyebrow">Altitude</span>
            <svg id="chart-altitude" viewBox="0 0 480 220"></svg>
          </div>
          <div class="panel chart-shell">
            <span class="eyebrow">Velocity</span>
            <svg id="chart-velocity" viewBox="0 0 480 220"></svg>
          </div>
          <div class="panel chart-shell">
            <span class="eyebrow">Throttle</span>
            <svg id="chart-throttle" viewBox="0 0 480 220"></svg>
          </div>
          <div class="panel chart-shell">
            <span class="eyebrow">Attitude</span>
            <svg id="chart-attitude" viewBox="0 0 480 220"></svg>
          </div>
        </div>
      </div>

      <div class="stack">
        <div class="panel">
          <span class="eyebrow">Markers</span>
          <div id="marker-list"></div>
        </div>
        <div class="panel">
          <span class="eyebrow">Events</span>
          <div id="event-list"></div>
        </div>
        <div class="panel">
          <span class="eyebrow">Controller Spec</span>
          <details open>
            <summary>Serialized controller config</summary>
            <pre id="controller-spec"></pre>
          </details>
        </div>
      </div>
    </section>
  </main>

  <script>
    const reportData = __REPORT_DATA__;
    const svgNS = "http://www.w3.org/2000/svg";

    const fmt = (value, digits = 2) =>
      Number.isFinite(value) ? value.toFixed(digits) : "n/a";

    const setText = (id, value) => {
      const node = document.getElementById(id);
      if (node) node.textContent = value;
    };

    function createSvg(tag, attrs = {}) {
      const node = document.createElementNS(svgNS, tag);
      Object.entries(attrs).forEach(([key, value]) => node.setAttribute(key, value));
      return node;
    }

    function buildSummary() {
      setText("scenario-title", reportData.scenarioName);
      setText(
        "scenario-subtitle",
        `${reportData.scenarioId} · ${reportData.samples.length} sampled states`
      );
      setText("controller-id", reportData.controllerId);
      setText("mission-outcome", reportData.manifest.missionOutcome);
      setText("physical-outcome", reportData.manifest.physicalOutcome);
      setText("end-reason", reportData.manifest.endReason);
      setText("sim-time", `${fmt(reportData.manifest.simTimeS, 2)} s`);
      setText("physics-steps", String(reportData.manifest.physicsSteps));
      setText("controller-updates", String(reportData.manifest.controllerUpdates));

      const tags = [
        `${reportData.events.length} events`,
        `${reportData.markers.length} controller markers`,
      ];
      const tagRow = document.getElementById("report-tags");
      tags.forEach((tag) => {
        const span = document.createElement("span");
        span.className = "tag";
        span.textContent = tag;
        tagRow.appendChild(span);
      });
      setText(
        "controller-spec",
        reportData.controllerSpec
          ? JSON.stringify(reportData.controllerSpec, null, 2)
          : "No controller config artifact captured"
      );
    }

    function sampleBounds() {
      const xs = [];
      const ys = [];
      reportData.terrain.forEach((p) => { xs.push(p.xM); ys.push(p.yM); });
      reportData.samples.forEach((p) => { xs.push(p.xM); ys.push(p.yM); });
      if (reportData.pad) {
        xs.push(reportData.pad.centerXM - reportData.pad.widthM / 2);
        xs.push(reportData.pad.centerXM + reportData.pad.widthM / 2);
        ys.push(reportData.pad.surfaceYM);
      }
      if (xs.length === 0 || ys.length === 0) {
        return { minX: -1, maxX: 1, minY: -1, maxY: 1 };
      }
      const minX = Math.min(...xs);
      const maxX = Math.max(...xs);
      const minY = Math.min(...ys);
      const maxY = Math.max(...ys);
      const padX = Math.max((maxX - minX) * 0.08, 8);
      const padY = Math.max((maxY - minY) * 0.12, 6);
      return { minX: minX - padX, maxX: maxX + padX, minY: minY - padY, maxY: maxY + padY };
    }

    function drawTrajectory(selectedIndex = 0) {
      const svg = document.getElementById("trajectory");
      svg.innerHTML = "";
      if (!reportData.samples.length) {
        const text = createSvg("text", { x: 40, y: 60, fill: "#5d5a52" });
        text.textContent = "No sampled trace captured for this run.";
        svg.appendChild(text);
        return;
      }

      const { minX, maxX, minY, maxY } = sampleBounds();
      const width = 960;
      const height = 320;
      const pad = { left: 36, right: 24, top: 22, bottom: 34 };
      const xScale = (value) =>
        pad.left + ((value - minX) / Math.max(maxX - minX, 1e-9)) * (width - pad.left - pad.right);
      const yScale = (value) =>
        height - pad.bottom - ((value - minY) / Math.max(maxY - minY, 1e-9)) * (height - pad.top - pad.bottom);

      for (let step = 0; step <= 4; step += 1) {
        const y = pad.top + ((height - pad.top - pad.bottom) * step) / 4;
        svg.appendChild(createSvg("line", {
          x1: pad.left,
          x2: width - pad.right,
          y1: y,
          y2: y,
          stroke: "rgba(60,53,38,0.12)",
          "stroke-width": 1,
        }));
      }

      const terrain = createSvg("polyline", {
        fill: "none",
        stroke: "#7d5f3b",
        "stroke-width": 3,
        points: reportData.terrain.map((p) => `${xScale(p.xM)},${yScale(p.yM)}`).join(" "),
      });
      svg.appendChild(terrain);

      if (reportData.pad) {
        const padLine = createSvg("line", {
          x1: xScale(reportData.pad.centerXM - reportData.pad.widthM / 2),
          x2: xScale(reportData.pad.centerXM + reportData.pad.widthM / 2),
          y1: yScale(reportData.pad.surfaceYM),
          y2: yScale(reportData.pad.surfaceYM),
          stroke: "#2f6a63",
          "stroke-width": 6,
          "stroke-linecap": "round",
        });
        svg.appendChild(padLine);
      }

      const trajectory = createSvg("polyline", {
        fill: "none",
        stroke: "#d7693d",
        "stroke-width": 3,
        points: reportData.samples.map((p) => `${xScale(p.xM)},${yScale(p.yM)}`).join(" "),
      });
      svg.appendChild(trajectory);

      reportData.events.forEach((event) => {
        if (event.xM == null || event.yM == null) return;
        svg.appendChild(createSvg("circle", {
          cx: xScale(event.xM),
          cy: yScale(event.yM),
          r: 4.5,
          fill: event.kind.includes("touchdown") || event.kind.includes("satisfied") ? "#2f6a63" : "#b3245a",
          opacity: 0.9,
        }));
      });

      reportData.markers.forEach((marker) => {
        if (marker.xM == null || marker.yM == null) return;
        const size = 5;
        const cx = xScale(marker.xM);
        const cy = yScale(marker.yM);
        const poly = createSvg("polygon", {
          points: `${cx},${cy - size} ${cx + size},${cy} ${cx},${cy + size} ${cx - size},${cy}`,
          fill: "#3f4f9f",
          opacity: 0.85,
        });
        svg.appendChild(poly);
      });

      const selected = reportData.samples[Math.max(0, Math.min(selectedIndex, reportData.samples.length - 1))];
      svg.appendChild(createSvg("circle", {
        cx: xScale(selected.xM),
        cy: yScale(selected.yM),
        r: 7,
        fill: "#fff9ee",
        stroke: "#111",
        "stroke-width": 2,
      }));
    }

    function drawLineChart(svgId, series) {
      const svg = document.getElementById(svgId);
      svg.innerHTML = "";
      if (!reportData.samples.length) {
        return;
      }

      const width = 480;
      const height = 220;
      const pad = { left: 42, right: 18, top: 14, bottom: 28 };
      const xs = reportData.samples.map((sample) => sample.simTimeS);
      const values = series.flatMap((entry) => reportData.samples.map((sample) => entry.value(sample)));
      let minY = Math.min(...values);
      let maxY = Math.max(...values);
      if (!Number.isFinite(minY) || !Number.isFinite(maxY)) {
        minY = -1;
        maxY = 1;
      }
      if (Math.abs(maxY - minY) < 1e-9) {
        minY -= 1;
        maxY += 1;
      }
      const minX = Math.min(...xs);
      const maxX = Math.max(...xs);
      const xScale = (value) =>
        pad.left + ((value - minX) / Math.max(maxX - minX, 1e-9)) * (width - pad.left - pad.right);
      const yScale = (value) =>
        height - pad.bottom - ((value - minY) / Math.max(maxY - minY, 1e-9)) * (height - pad.top - pad.bottom);

      for (let step = 0; step <= 4; step += 1) {
        const y = pad.top + ((height - pad.top - pad.bottom) * step) / 4;
        svg.appendChild(createSvg("line", {
          x1: pad.left,
          x2: width - pad.right,
          y1: y,
          y2: y,
          stroke: "rgba(60,53,38,0.12)",
          "stroke-width": 1,
        }));
      }

      series.forEach((entry) => {
        svg.appendChild(createSvg("polyline", {
          fill: "none",
          stroke: entry.color,
          "stroke-width": 2.5,
          points: reportData.samples.map((sample) => `${xScale(sample.simTimeS)},${yScale(entry.value(sample))}`).join(" "),
        }));
      });

      const legend = createSvg("text", {
        x: pad.left,
        y: height - 8,
        fill: "#5d5a52",
        "font-size": 11,
      });
      legend.textContent = series.map((entry) => entry.label).join(" · ");
      svg.appendChild(legend);
    }

    function buildLists() {
      const markerList = document.getElementById("marker-list");
      if (!reportData.markers.length) {
        markerList.innerHTML = '<div class="empty">No controller markers were emitted.</div>';
      } else {
        reportData.markers.forEach((marker) => {
          const item = document.createElement("div");
          item.className = "sample-card";
          item.innerHTML = `<span>${fmt(marker.simTimeS, 2)} s</span><strong>${marker.label}</strong>`;
          markerList.appendChild(item);
        });
      }

      const eventList = document.getElementById("event-list");
      if (!reportData.events.length) {
        eventList.innerHTML = '<div class="empty">No events captured.</div>';
      } else {
        reportData.events.forEach((event) => {
          const item = document.createElement("div");
          item.className = "sample-card";
          item.innerHTML = `<span>${fmt(event.simTimeS, 2)} s</span><strong>${event.kind}</strong><p>${event.message}</p>`;
          eventList.appendChild(item);
        });
      }
    }

    function updateSample(index) {
      const sample = reportData.samples[index];
      if (!sample) return;

      drawTrajectory(index);

      const summary = document.getElementById("sample-summary");
      summary.innerHTML = "";
      [
        `t=${fmt(sample.simTimeS, 2)}s`,
        sample.phase ? `phase=${sample.phase}` : "phase=n/a",
        sample.status ? sample.status : "status=n/a",
      ].forEach((value) => {
        const tag = document.createElement("span");
        tag.className = "tag";
        tag.textContent = value;
        summary.appendChild(tag);
      });

      const grid = document.getElementById("sample-grid");
      grid.innerHTML = "";
      const cards = [
        ["Position", `${fmt(sample.xM)} m, ${fmt(sample.yM)} m`],
        ["Velocity", `${fmt(sample.vxMps)} m/s, ${fmt(sample.vyMps)} m/s`],
        ["Altitude", `${fmt(sample.heightAboveTargetM)} m`],
        ["Clearance", `${fmt(sample.touchdownClearanceM)} m`],
        ["Throttle", `${fmt(sample.throttleFrac * 100, 1)} %`],
        ["Attitude", `${fmt(sample.attitudeDeg, 1)} deg`],
        ["Target dx", `${fmt(sample.targetDxM)} m`],
        ["Fuel", `${fmt(sample.fuelKg)} kg`],
      ];
      cards.forEach(([label, value]) => {
        const card = document.createElement("div");
        card.className = "sample-card";
        card.innerHTML = `<span>${label}</span><strong>${value}</strong>`;
        grid.appendChild(card);
      });

      setText("metrics-json", JSON.stringify(sample.metrics, null, 2));
    }

    function init() {
      buildSummary();
      buildLists();
      drawTrajectory(0);
      drawLineChart("chart-altitude", [
        { label: "height above target", color: "#d7693d", value: (sample) => sample.heightAboveTargetM },
        { label: "touchdown clearance", color: "#2f6a63", value: (sample) => sample.touchdownClearanceM },
      ]);
      drawLineChart("chart-velocity", [
        { label: "vx", color: "#3f4f9f", value: (sample) => sample.vxMps },
        { label: "vy", color: "#b3245a", value: (sample) => sample.vyMps },
      ]);
      drawLineChart("chart-throttle", [
        { label: "throttle", color: "#2f6a63", value: (sample) => sample.throttleFrac },
      ]);
      drawLineChart("chart-attitude", [
        { label: "attitude deg", color: "#d7693d", value: (sample) => sample.attitudeDeg },
        { label: "target deg", color: "#3f4f9f", value: (sample) => sample.targetAttitudeDeg },
      ]);

      const slider = document.getElementById("sample-slider");
      slider.max = String(Math.max(reportData.samples.length - 1, 0));
      slider.addEventListener("input", (event) => {
        updateSample(Number(event.target.value));
      });
      updateSample(0);
    }

    init();
  </script>
</body>
</html>
"##
}
