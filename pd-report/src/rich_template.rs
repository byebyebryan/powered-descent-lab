//! Common rich-detail HTML/CSS/JS, shared by controller and planner reports.
//! Data construction and placeholder substitution remain in the public facade.

pub(super) const PLANNER_PANEL_HTML: &str = r###"<section class="panel wide" id="planner-panel">
        <div class="panel-head">
          <div>
            <div class="eyebrow">Planner Evidence</div>
            <h2>Generated Route Plan</h2>
          </div>
        </div>
        <p class="muted" id="planner-summary"></p>
        <div class="fact-grid" id="planner-grid"></div>
      </section>"###;

pub(super) fn report_template() -> &'static str {
    r####"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>__REPORT_TITLE__</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f1ede5;
      --panel: #fffdf8;
      --panel-strong: #f8f2e8;
      --ink: #20211e;
      --muted: #6d665c;
      --accent: #176b5c;
      --rust: #b95024;
      --warn: #a43a2c;
      --success: #176b5c;
      --line: #d9cdbc;
      --shadow: rgba(45, 34, 23, 0.08);
      --chip: rgba(23, 107, 92, 0.075);
      --chip-border: rgba(23, 107, 92, 0.18);
      --display: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Palatino, Georgia, serif;
      --sans: "Avenir Next", "IBM Plex Sans", "Trebuchet MS", sans-serif;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: var(--sans);
      color: var(--ink);
      background:
        linear-gradient(rgba(69, 58, 44, 0.025) 1px, transparent 1px),
        linear-gradient(90deg, rgba(69, 58, 44, 0.025) 1px, transparent 1px),
        radial-gradient(circle at 12% 0%, rgba(185, 80, 36, 0.12), transparent 31rem),
        linear-gradient(180deg, #faf7f0 0%, var(--bg) 100%);
      background-size: 32px 32px, 32px 32px, auto, auto;
      background-attachment: fixed;
      overflow-x: hidden;
    }
    main {
      max-width: 1520px;
      margin: 0 auto;
      padding: 18px 16px 40px;
    }
    section, header {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 18px;
      box-shadow: 0 12px 34px var(--shadow);
    }
    header {
      position: relative;
      overflow: hidden;
      padding: 17px 19px 19px;
      margin-bottom: 14px;
      border-radius: 22px;
      background:
        radial-gradient(circle at 91% 15%, rgba(185,80,36,0.09), transparent 18rem),
        rgba(255,253,248,0.95);
      box-shadow: 0 18px 44px var(--shadow);
    }
    header::before {
      content: "";
      position: absolute;
      inset: 0 0 auto;
      height: 4px;
      background: linear-gradient(90deg, var(--rust) 0 42%, var(--accent) 42% 71%, #3568a8 71%);
    }
    header::after {
      content: "";
      position: absolute;
      width: 250px;
      height: 250px;
      right: -92px;
      top: -154px;
      border: 1px solid rgba(185,80,36,0.2);
      border-radius: 50%;
      pointer-events: none;
    }
    header > * {
      position: relative;
      z-index: 1;
    }
    .breadcrumbs {
      display: none;
      align-items: center;
      flex-wrap: wrap;
      gap: 7px;
      margin-bottom: 11px;
      font-family: var(--sans);
      font-size: 0.82rem;
    }
    .breadcrumbs.visible { display: flex; }
    .breadcrumbs a {
      color: var(--accent);
      text-decoration: none;
      border: 1px solid var(--chip-border);
      border-radius: 999px;
      padding: 5px 9px;
      background: var(--chip);
    }
    .breadcrumbs span { color: var(--muted); }
    h1, h2, h3 {
      margin: 0;
      font-family: var(--display);
    }
    p { margin: 0; }
    .hero {
      display: grid;
      grid-template-columns: 1fr;
      gap: 12px;
      align-items: start;
    }
    .hero-main {
      display: grid;
      gap: 10px;
      min-width: 0;
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
      max-width: 24ch;
      font-size: clamp(2rem, 4vw, 3.2rem);
      font-weight: 500;
      letter-spacing: -0.035em;
      line-height: 0.98;
      overflow-wrap: anywhere;
    }
    #scenario-subtitle { overflow-wrap: anywhere; }
    .banner {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 12rem;
      padding: 0.62rem 1rem;
      border-radius: 999px;
      font-weight: 700;
      text-align: center;
      background: rgba(23, 107, 92, 0.1);
      color: var(--accent);
      border: 1px solid rgba(23, 107, 92, 0.2);
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
      gap: 8px;
    }
    .stat, .panel-block {
      border: 1px solid var(--line);
      border-radius: 13px;
      background: rgba(255, 255, 255, 0.52);
      padding: 9px 11px;
      min-width: 0;
    }
    .stat {
      box-shadow: inset 0 2px 0 rgba(23,107,92,0.18);
    }
    .stat .label {
      font-size: 0.78rem;
      letter-spacing: 0.05em;
      text-transform: uppercase;
      color: var(--muted);
      margin-bottom: 3px;
    }
    .stat .value {
      font-size: 1.08rem;
      font-weight: 700;
      font-variant-numeric: tabular-nums;
      overflow-wrap: anywhere;
    }
    .main-grid {
      display: grid;
      grid-template-columns: minmax(0, 2.05fr) minmax(21rem, 0.92fr);
      gap: 12px;
      align-items: start;
    }
    .left-stack, .right-stack {
      display: grid;
      gap: 12px;
    }
    .right-stack {
      position: sticky;
      top: 14px;
    }
    .panel {
      padding: 12px 14px 14px;
      overflow: hidden;
      background: rgba(255,253,248,0.94);
    }
    .panel-head {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      gap: 10px;
      margin-bottom: 8px;
    }
    .panel-head h2 {
      font-size: 1.3rem;
      font-weight: 600;
      letter-spacing: -0.015em;
    }
    .plot-toolbar {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      justify-content: flex-end;
    }
    .plot-toolbar button {
      border: 1px solid var(--line);
      background: var(--panel-strong);
      color: var(--ink);
      border-radius: 999px;
      padding: 4px 10px;
      cursor: pointer;
      font: inherit;
      transition: transform 150ms ease, border-color 150ms ease, color 150ms ease;
    }
    .plot-toolbar button:hover {
      border-color: var(--rust);
      color: var(--rust);
      transform: translateY(-1px);
    }
    .plot-toolbar button.active {
      background: var(--accent);
      color: #fffdf8;
      border-color: var(--accent);
    }
    .chart {
      width: 100%;
      height: 408px;
      border: 1px solid var(--line);
      border-radius: 13px;
      background: #fcfaf5;
    }
    .metric-grid {
      display: grid;
      grid-template-columns: 1fr;
      gap: 12px;
    }
    .metric-grid .chart {
      height: 336px;
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
      min-width: 0;
    }
    .summary-grid .panel-block {
      display: grid;
      grid-template-columns: auto minmax(0, 1fr);
      align-items: center;
      gap: 10px 16px;
      padding: 9px 11px;
      border-radius: 13px;
      background: rgba(248,242,232,0.62);
    }
    .summary-grid .eyebrow {
      color: var(--ink);
      font-weight: 700;
    }
    #phase-chips {
      margin-top: 0;
    }
    .key-grid {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 10px;
    }
    .hero-main .key-grid {
      margin-top: 2px;
    }
    .key-grid .stat {
      min-width: 0;
      padding: 8px 10px;
      box-shadow: inset 0 2px 0 rgba(185,80,36,0.18);
    }
    .key-grid .stat .value {
      font-size: 1rem;
    }
    .stat .meta {
      margin-top: 3px;
      font-size: 0.8rem;
      color: var(--muted);
      line-height: 1.2;
    }
    .compact-list {
      display: grid;
      gap: 8px;
      max-height: 14rem;
      overflow: auto;
      padding-right: 2px;
    }
    .compact-item {
      display: grid;
      grid-template-columns: auto 1fr;
      gap: 8px 10px;
      align-items: start;
      padding: 8px 0;
      border-bottom: 1px solid rgba(217, 205, 188, 0.68);
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
      background: rgba(255, 255, 255, 0.62);
      box-shadow: inset 3px 0 0 rgba(23,107,92,0.24);
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
    .detail-grid {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 12px;
      margin-top: 12px;
    }
    .detail-grid .panel.wide {
      grid-column: span 4;
    }
    .fact-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
    }
    .fact {
      border: 1px solid var(--line);
      border-radius: 10px;
      padding: 8px 10px;
      background: rgba(255, 255, 255, 0.62);
    }
    .fact .label {
      font-size: 0.74rem;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      color: var(--muted);
      margin-bottom: 2px;
    }
    .fact .value {
      font-weight: 700;
      font-size: 0.98rem;
      line-height: 1.2;
      font-variant-numeric: tabular-nums;
      overflow-wrap: anywhere;
    }
    .stack {
      display: grid;
      gap: 10px;
    }
    .mission-grid {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 10px;
      margin-top: 8px;
    }
    .mission-card {
      border: 1px solid var(--line);
      border-radius: 12px;
      padding: 10px 11px;
      background: rgba(255, 255, 255, 0.6);
      box-shadow: inset 3px 0 0 rgba(185,80,36,0.25);
      min-width: 0;
    }
    .mission-card h3 {
      margin: 0 0 8px;
      font-size: 0.92rem;
    }
    .mission-list {
      display: grid;
      gap: 5px;
      font-size: 0.9rem;
      color: var(--muted);
    }
    .mission-list strong {
      color: var(--ink);
    }
    details {
      border-top: 1px solid rgba(216, 207, 191, 0.75);
      margin-top: 10px;
      padding-top: 8px;
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
      max-height: 14rem;
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
      .detail-grid {
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }
      .detail-grid .panel.wide {
        grid-column: span 2;
      }
    }
    @media (max-width: 860px) {
      .hero {
        grid-template-columns: 1fr;
      }
      .metric-grid {
        grid-template-columns: 1fr;
      }
      .detail-grid {
        grid-template-columns: 1fr;
      }
      .detail-grid .panel.wide {
        grid-column: span 1;
      }
      .key-grid {
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }
      .mission-grid {
        grid-template-columns: 1fr;
      }
      .fact-grid {
        grid-template-columns: 1fr 1fr;
      }
      .inspect-grid {
        grid-template-columns: 1fr 1fr;
      }
      .chart {
        height: 340px;
      }
      .metric-grid .chart {
        height: 250px;
      }
    }
    @media (max-width: 560px) {
      .stats {
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }
      .key-grid {
        grid-template-columns: 1fr;
      }
      main { padding-inline: 10px; }
      header, .panel { padding-inline: 11px; }
      .title-row { align-items: flex-start; }
      .banner { min-width: 0; width: 100%; }
      .summary-grid .panel-block {
        grid-template-columns: 1fr;
      }
      .plot-toolbar { justify-content: flex-start; }
      .chart { min-width: 0; }
      pre { max-width: 100%; overflow-x: auto; }
    }
    __FLIGHT_ANNOTATIONS_CSS__
  </style>
  <script src="__PLOTLY_HREF__"></script>
</head>
<body>
  <main>
    <header>
      <nav class="breadcrumbs" id="report-breadcrumbs" aria-label="Report navigation"></nav>
      __FLIGHT_ANNOTATIONS_BANNER__
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
            <div class="stat">
              <div class="label">Controller</div>
              <div class="value" id="controller-id"></div>
            </div>
          </div>
          <div class="key-grid">
            <div class="stat">
              <div class="label" id="key-quality-label"></div>
              <div class="value" id="key-quality-value"></div>
              <div class="meta" id="key-quality-meta"></div>
            </div>
            <div class="stat">
              <div class="label">Flight Time</div>
              <div class="value" id="key-flight-time"></div>
              <div class="meta" id="key-flight-meta"></div>
            </div>
            <div class="stat">
              <div class="label">Fuel Used</div>
              <div class="value" id="key-fuel-used"></div>
              <div class="meta" id="key-fuel-meta"></div>
            </div>
          </div>
        </div>
        <div class="summary-grid">
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
              <button type="button" data-mode="mission" class="active">Mission</button>
              <button type="button" data-mode="guidance">Guidance</button>
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
                <div class="eyebrow">Time</div>
                <h2>Velocity And Thrust</h2>
              </div>
            </div>
            <div id="chart-metrics" class="chart"></div>
          </section>
        </section>
      </div>

      <div class="right-stack">
        __FLIGHT_ANNOTATIONS_PANEL__
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

    <section class="detail-grid">
      <section class="panel">
        <div class="panel-head">
          <div>
            <div class="eyebrow">Landing</div>
            <h2 id="quality-title">Landing Quality</h2>
          </div>
        </div>
        <div class="fact-grid" id="quality-grid"></div>
      </section>

      <section class="panel">
        <div class="panel-head">
          <div>
            <div class="eyebrow">Flight</div>
            <h2>Flight Stats</h2>
          </div>
        </div>
        <div class="fact-grid" id="flight-grid"></div>
      </section>

      <section class="panel">
        <div class="panel-head">
          <div>
            <div class="eyebrow">Bot</div>
            <h2>Controller Stats</h2>
          </div>
        </div>
        <div class="fact-grid" id="bot-grid"></div>
      </section>

      <section class="panel">
        <div class="panel-head">
          <div>
            <div class="eyebrow">Run</div>
            <h2>Run And Sim Performance</h2>
          </div>
        </div>
        <div class="fact-grid" id="run-grid"></div>
      </section>

      __PLANNER_PANEL__

      <section class="panel wide">
        <div class="panel-head">
          <div>
            <div class="eyebrow">Mission</div>
            <h2>Mission Profile</h2>
          </div>
        </div>
        <p class="muted" id="mission-description"></p>
        <div class="mission-grid">
          <div class="mission-card">
            <h3>Scenario</h3>
            <div class="mission-list" id="mission-card-scenario"></div>
          </div>
          <div class="mission-card">
            <h3>Initial State</h3>
            <div class="mission-list" id="mission-card-initial"></div>
          </div>
          <div class="mission-card">
            <h3>Vehicle</h3>
            <div class="mission-list" id="mission-card-vehicle"></div>
          </div>
          <div class="mission-card">
            <h3>Goal And Target</h3>
            <div class="mission-list" id="mission-card-goal"></div>
          </div>
        </div>
      </section>
    </section>
  </main>

  <script>
    const reportData = __REPORT_DATA__;
    const paperBg = "#fffdf8";
    const plotBg = "#fcfaf5";
    const sharedConfig = {
      responsive: true,
      displaylogo: false,
      modeBarButtonsToRemove: [
        "lasso2d",
        "select2d",
        "toggleSpikelines",
        "hoverClosestCartesian",
        "hoverCompareCartesian",
        "autoScale2d",
      ],
    };
    const spatialConfig = sharedConfig;
    const compactConfig = { ...sharedConfig, displayModeBar: false };

    const fmt = (value, digits = 2) =>
      Number.isFinite(Number(value)) ? Number(value).toFixed(digits) : "n/a";

    const setText = (id, value) => {
      const node = document.getElementById(id);
      if (node) node.textContent = value;
    };

    const fmtOptional = (value, digits = 2, suffix = "") =>
      Number.isFinite(Number(value)) ? `${Number(value).toFixed(digits)}${suffix}` : "n/a";

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
    const flightCorrections = reportData.flightAnnotations?.corrections || [];
    let flightHandoffsVisible = true;
    let selectedFlightCorrection = 0;
    let flightHandoffTraceIndex = -1;
    let baseFlightMetricShapes = [];
    const buildFlightGuideShapes = () => flightHandoffsVisible ? flightCorrections.map((c, index) => ({
      type: "line", xref: "x", yref: "paper",
      x0: c.handoff.simTimeS, x1: c.handoff.simTimeS, y0: 0, y1: 1,
      line: { color: "#b45309", width: index === selectedFlightCorrection ? 2 : 1, dash: "dot" },
    })) : [];
    const buildFlightGuideLabels = () => flightHandoffsVisible ? flightCorrections.map(c => ({
      xref: "x", yref: "paper", x: c.handoff.simTimeS, y: 1,
      text: `H${c.number}`, showarrow: false, yanchor: "bottom",
      font: { color: "#b45309", size: 11 },
    })) : [];
    const selectFlightCorrection = (index) => {
      if (!Number.isInteger(index) || !flightCorrections[index] || index === selectedFlightCorrection) return;
      selectedFlightCorrection = index;
      document.querySelectorAll("[data-select-correction]").forEach(button => {
        button.setAttribute("aria-pressed", String(Number(button.dataset.selectCorrection) === index));
      });
      const spatial = document.getElementById("chart-spatial");
      if (flightHandoffTraceIndex >= 0) {
        Plotly.restyle(spatial, { "marker.size": [flightCorrections.map((_c, i) => i === index ? 13 : 9)] }, [flightHandoffTraceIndex]);
      }
      Plotly.relayout("chart-metrics", {
        shapes: [...baseFlightMetricShapes, ...buildFlightGuideShapes()],
        annotations: buildFlightGuideLabels(),
      });
    };
    const initFlightCorrections = () => {
      if (!flightCorrections.length) return;
      document.querySelectorAll("[data-select-correction]").forEach(button => {
        button.addEventListener("click", () => selectFlightCorrection(Number(button.dataset.selectCorrection)));
      });
      document.getElementById("flight-handoffs-visible").addEventListener("change", event => {
        flightHandoffsVisible = event.target.checked;
        Plotly.restyle("chart-spatial", { visible: flightHandoffsVisible }, [flightHandoffTraceIndex]);
        Plotly.relayout("chart-metrics", {
          shapes: [...baseFlightMetricShapes, ...buildFlightGuideShapes()],
          annotations: buildFlightGuideLabels(),
        });
      });
    };
    const pad = reportData.pad || null;
    const transferRoute = reportData.missionDetails?.transferRoute || null;
    const waypoints = Array.isArray(transferRoute?.waypoints) ? transferRoute.waypoints : [];
    const routePlan = reportData.routePlan || null;
    const plannerCompute = reportData.plannerCompute || null;
    const plannerDiagnostics = routePlan?.diagnostics || null;
    const plannerSafeProfile = Array.isArray(plannerDiagnostics?.safe_profile_points_m)
      ? plannerDiagnostics.safe_profile_points_m
      : [];
    const plannerCenterline = Array.isArray(plannerDiagnostics?.selected_centerline_m)
      ? plannerDiagnostics.selected_centerline_m
      : [];

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
    const throttleXValues = samples.map((sample) => Number(sample.throttleFrac) * Math.sin(Number(sample.attitudeRad || 0)));
    const throttleYValues = samples.map((sample) => Number(sample.throttleFrac) * Math.cos(Number(sample.attitudeRad || 0)));

    const speedColorScale = [
      [0.0, "#9a6b00"],
      [0.35, "#d88718"],
      [0.65, "#f76707"],
      [1.0, "#c1121f"],
    ];
    const throttleColorScale = [
      [0.0, "#4d8097"],
      [0.35, "#228be6"],
      [0.7, "#1c64d1"],
      [1.0, "#0b3d91"],
    ];
    const waypointLegStyles = [
      { name: "waypoint active leg", color: "#b45309", dash: "dash" },
      { name: "waypoint outbound leg", color: "#1d4ed8", dash: "dashdot" },
    ];

    const axisStyle = (extra = {}) => Object.assign({
      automargin: true,
      gridcolor: "rgba(217, 205, 188, 0.46)",
      zerolinecolor: "rgba(109, 102, 92, 0.42)",
      linecolor: "rgba(109, 102, 92, 0.32)",
    }, extra);

    const layoutBase = (extra = {}) => Object.assign({
      paper_bgcolor: paperBg,
      plot_bgcolor: plotBg,
      margin: { l: 58, r: 24, t: 16, b: 38 },
      legend: {
        orientation: "h",
        yanchor: "bottom",
        y: 1.03,
        xanchor: "left",
        x: 0,
        bgcolor: "rgba(255,253,248,0.9)",
        font: { size: 11 },
      },
      font: {
        family: "Avenir Next, IBM Plex Sans, Trebuchet MS, sans-serif",
        size: 12,
        color: "#20211e",
      },
      hoverlabel: {
        bgcolor: "#fffdf8",
        bordercolor: "#d9cdbc",
        font: { family: "Avenir Next, IBM Plex Sans, Trebuchet MS, sans-serif", size: 12, color: "#20211e" },
      },
    }, extra);

    const spatialLayout = (extra = {}) => layoutBase(Object.assign({
      margin: { l: 58, r: 76, t: 18, b: 34 },
      legend: {
        orientation: "h",
        yanchor: "bottom",
        y: 1.02,
        xanchor: "left",
        x: 0,
        bgcolor: "rgba(255,253,248,0.9)",
        font: { size: 11 },
      },
    }, extra));

    const metricLayout = (extra = {}) => layoutBase(Object.assign({
      margin: { l: 56, r: 54, t: 36, b: 54 },
      legend: {
        orientation: "h",
        yanchor: "bottom",
        y: 1.02,
        xanchor: "left",
        x: 0,
        bgcolor: "rgba(255,253,248,0.9)",
        font: { size: 11 },
      },
    }, extra));

    const maxFuelKg = Number(reportData.missionDetails?.vehicle?.maxFuelKg || 0);
    const fuelPercent = (fuelKg) => {
      const value = Number(fuelKg);
      if (!Number.isFinite(value) || !Number.isFinite(maxFuelKg) || maxFuelKg <= 1e-9) return null;
      return (value / maxFuelKg) * 100.0;
    };
    const fmtFuelPct = (fuelKg, digits = 1) => fmtOptional(fuelPercent(fuelKg), digits, " %");

    const summarizeOutcome = () => {
      setText("scenario-title", reportData.displayTitle || reportData.scenarioName);
      setText("scenario-subtitle", reportData.displaySubtitle || reportData.scenarioId);
      setText("controller-id", reportData.controllerId);
      setText("mission-outcome", reportData.manifest.missionOutcome);
      setText("physical-outcome", reportData.manifest.physicalOutcome);
      setText("end-reason", reportData.manifest.endReason);
      setText("sim-time", `${fmt(reportData.manifest.simTimeS, 2)} s`);
      setText("physics-steps", String(reportData.manifest.physicsSteps));
      setText("controller-updates", String(reportData.manifest.controllerUpdates));
      setText("wall-time", fmtOptional(reportData.runPerformance.wallTimeMs, 2, " ms"));
      setText("cpu-time", fmtOptional(reportData.runPerformance.threadCpuTimeMs, 2, " ms"));
      setText("cpu-per-tick", fmtOptional(reportData.runPerformance.cpuTimePerTickUs, 2, " us"));

      const banner = document.getElementById("outcome-banner");
      const isFailure = String(reportData.manifest.missionOutcome || "").startsWith("failed");
      banner.textContent = `${reportData.manifest.missionOutcome} · ${reportData.manifest.endReason}`;
      banner.classList.toggle("failure", isFailure);
    };

    const renderBreadcrumbs = () => {
      const root = document.getElementById("report-breadcrumbs");
      const context = reportData.reportContext || {};
      const links = [];
      if (context.parentReportHref) {
        links.push([context.parentReportLabel || "Batch report", context.parentReportHref]);
      }
      if (context.runIndexHref) links.push(["Run index", context.runIndexHref]);
      if (!links.length) return;
      links.forEach(([label, href], index) => {
        if (index) {
          const separator = document.createElement("span");
          separator.textContent = "/";
          root.appendChild(separator);
        }
        const anchor = document.createElement("a");
        anchor.textContent = label;
        anchor.href = href;
        root.appendChild(anchor);
      });
      root.classList.add("visible");
    };

    const renderFacts = (targetId, rows) => {
      const root = document.getElementById(targetId);
      root.innerHTML = "";
      rows.forEach(([label, value]) => {
        const fact = document.createElement("div");
        fact.className = "fact";
        fact.innerHTML = `<div class="label">${label}</div><div class="value">${value}</div>`;
        root.appendChild(fact);
      });
    };

    const renderPlannerEvidence = () => {
      const panel = document.getElementById("planner-panel");
      if (!panel) return;
      if (!routePlan || !plannerDiagnostics) {
        panel.hidden = true;
        return;
      }
      panel.hidden = false;
      const policy = routePlan.policy || {};
      const authority = Array.isArray(plannerDiagnostics.waypoint_authority)
        ? plannerDiagnostics.waypoint_authority
        : [];
      const caps = authority
        .map((entry) => Number(entry.handoff_speed_cap_mps))
        .filter((value) => Number.isFinite(value))
        .map((value) => `${value.toFixed(1)} m/s`)
        .join(", ") || "n/a";
      const ratios = authority
        .flatMap((entry) => [
          Number(entry.inbound_stopping_ratio_at_handoff),
          Number(entry.outbound_stopping_ratio_at_handoff),
          Number(entry.inbound_turn_ratio_at_handoff),
          Number(entry.outbound_turn_ratio_at_handoff),
        ])
        .filter((value) => Number.isFinite(value))
        .map((value) => value.toFixed(3))
        .join(", ") || "n/a";
      const direct = plannerDiagnostics.direct_path_clear
        ? "clear"
        : `blocked${plannerDiagnostics.direct_path_clearance?.worst_residual?.residual_m !== undefined
          ? ` · residual ${fmt(plannerDiagnostics.direct_path_clearance.worst_residual.residual_m)} m`
          : ""}`;
      setText(
        "planner-summary",
        `${routePlan.algorithm_id || "planner"} · ${routePlan.topology || "unknown"} · ${routePlan.route?.waypoints?.length || 0} emitted waypoint(s) · plan ${routePlan.plan_digest || "unknown"}`
      );
      renderFacts("planner-grid", [
        ["Algorithm / policy", `${routePlan.algorithm_id || "n/a"} · ${policy.policy_version || "n/a"}`],
        ["Request / plan digest", `${routePlan.request_digest || "n/a"} · ${routePlan.plan_digest || "n/a"}`],
        ["Topology / waypoints", `${routePlan.topology || "n/a"} · ${routePlan.route?.waypoints?.length || 0}`],
        ["Direct path", direct],
        ["Planned clearance", `${fmtOptional(plannerDiagnostics.minimum_planned_clearance_m)} m`],
        ["Route / direct / excess", `${fmtOptional(plannerDiagnostics.route_length_m)} / ${fmtOptional(plannerDiagnostics.direct_distance_m)} / ${fmtOptional(plannerDiagnostics.excess_length_m)} m`],
        ["Peak extra loft", `${fmtOptional(plannerDiagnostics.peak_extra_loft_m)} m`],
        ["Authority caps / ratios", `${caps} · ${ratios}`],
        ["Planner compute", plannerCompute
          ? `${fmt(Number(plannerCompute.wallTimeUs) / 1000.0, 2)} ms monotonic wall (pd_plan::plan)`
          : "n/a"],
      ]);
    };

    const renderMissionList = (targetId, rows) => {
      const root = document.getElementById(targetId);
      root.innerHTML = rows
        .map(([label, value]) => `<div><strong>${label}:</strong> ${value}</div>`)
        .join("");
    };

    const renderKeyStats = () => {
      const flight = reportData.flightStats;
      const bot = reportData.botStats;
      setText("key-fuel-used", fmtFuelPct(flight.fuelUsedKg));
      setText("key-fuel-meta", `${fmt(flight.fuelUsedKg)} kg used · ${fmtFuelPct(flight.fuelRemainingKg)} remaining`);
      setText("key-flight-time", `${fmt(flight.flightTimeS, 2)} s`);
      setText("key-flight-meta", `${fmt(flight.averageSpeedMps)} m/s avg · ${fmt(flight.pathDistanceM)} m path`);

      if (reportData.landingQuality) {
        const landing = reportData.landingQuality;
        setText("key-quality-label", "Landing Offset");
        setText("key-quality-value", `${fmt(landing.landingOffsetM)} m`);
        setText(
          "key-quality-meta",
          `${fmt(landing.impactSpeedMps)} m/s impact · ${fmt(landing.impactAttitudeDeg, 1)} deg`
        );
      } else if (reportData.checkpointQuality) {
        const checkpoint = reportData.checkpointQuality;
        setText("key-quality-label", "Checkpoint Error");
        setText("key-quality-value", `${fmt(checkpoint.positionErrorM)} m`);
        setText(
          "key-quality-meta",
          `${fmt(checkpoint.velocityErrorMps)} m/s vel err · ${fmt(checkpoint.attitudeErrorDeg, 1)} deg`
        );
      } else {
        setText("key-quality-label", "Mission Quality");
        setText("key-quality-value", "n/a");
        setText("key-quality-meta", "No landing or checkpoint quality summary captured");
      }

      setText("key-bot-step", fmtOptional(bot.meanComputeMs, 3, " ms"));
      setText(
        "key-bot-meta",
        `${fmtOptional(bot.p95ComputeMs, 3, " ms")} p95 · ${fmtOptional(bot.controlDutyCyclePct, 2, " %")} duty`
      );
    };

    const renderQuality = () => {
      if (reportData.landingQuality) {
        const landing = reportData.landingQuality;
        setText("quality-title", "Landing Quality");
        renderFacts("quality-grid", [
          ["Offset", `${fmt(landing.landingOffsetM)} m`],
          ["Pad margin", `${fmt(landing.padMarginM)} m`],
          ["Impact attitude", `${fmt(landing.impactAttitudeDeg, 1)} deg`],
          ["Normal speed", `${fmt(landing.impactNormalSpeedMps)} m/s`],
          ["Tangential speed", `${fmt(landing.impactTangentialSpeedMps)} m/s`],
          ["Impact speed", `${fmt(landing.impactSpeedMps)} m/s`],
          ["Angular rate", `${fmt(landing.angularRateDegps, 1)} deg/s`],
          ["Margin", `${fmt(landing.envelopeMarginRatio * 100, 1)} %`],
          ["Normal margin", `${fmt(landing.normalSpeedMarginMps)} m/s`],
          ["Tangential margin", `${fmt(landing.tangentialSpeedMarginMps)} m/s`],
          ["Attitude margin", `${fmt(landing.attitudeMarginDeg, 1)} deg`],
          ["Clearance", `${fmt(landing.minTouchdownClearanceM, 3)} / ${fmt(landing.minHullClearanceM, 3)} m`],
        ]);
        return;
      }

      if (reportData.checkpointQuality) {
        const checkpoint = reportData.checkpointQuality;
        setText("quality-title", "Checkpoint Quality");
        renderFacts("quality-grid", [
          ["Position error", `${fmt(checkpoint.positionErrorM)} m`],
          ["Velocity error", `${fmt(checkpoint.velocityErrorMps)} m/s`],
          ["Attitude error", `${fmt(checkpoint.attitudeErrorDeg, 1)} deg`],
          ["Position margin", `${fmt(checkpoint.positionMarginM)} m`],
          ["Velocity margin", `${fmt(checkpoint.velocityMarginMps)} m/s`],
          ["Attitude margin", `${fmt(checkpoint.attitudeMarginDeg, 1)} deg`],
          ["Envelope margin", `${fmt(checkpoint.envelopeMarginRatio * 100, 1)} %`],
        ]);
        return;
      }

      setText("quality-title", "Mission Quality");
      renderFacts("quality-grid", [["Status", "No mission-quality summary captured."]]);
    };

    const renderFlightStats = () => {
      const flight = reportData.flightStats;
      renderFacts("flight-grid", [
        ["Flight time", `${fmt(flight.flightTimeS, 2)} s`],
        ["Path distance", `${fmt(flight.pathDistanceM)} m`],
        ["Horizontal travel", `${fmt(flight.horizontalDistanceM)} m`],
        ["Net displacement", `${fmt(flight.netDisplacementM)} m`],
        ["Average speed", `${fmt(flight.averageSpeedMps)} m/s`],
        ["Max speed", `${fmt(flight.maxSpeedMps)} m/s`],
        ["Max altitude", `${fmt(flight.maxAltitudeM)} m`],
        ["Min altitude", `${fmt(flight.minAltitudeM)} m`],
        ["Fuel used", `${fmtFuelPct(flight.fuelUsedKg)} · ${fmt(flight.fuelUsedKg)} kg`],
        ["Fuel left", `${fmtFuelPct(flight.fuelRemainingKg)} · ${fmt(flight.fuelRemainingKg)} kg`],
      ]);
    };

    const renderBotStats = () => {
      const bot = reportData.botStats;
      renderFacts("bot-grid", [
        ["Updates", String(bot.controllerUpdates)],
        ["Total bot compute", fmtOptional(bot.totalComputeMs, 2, " ms")],
        ["Mean bot step", fmtOptional(bot.meanComputeMs, 3, " ms")],
        ["P95 bot step", fmtOptional(bot.p95ComputeMs, 3, " ms")],
        ["Max bot step", fmtOptional(bot.maxComputeMs, 3, " ms")],
        ["Mean control dt", fmtOptional(bot.meanControlDtMs, 2, " ms")],
        ["Bot duty cycle", fmtOptional(bot.controlDutyCyclePct, 2, " %")],
      ]);
    };

    const renderRunStats = () => {
      const run = reportData.runPerformance;
      renderFacts("run-grid", [
        ["Wall time", fmtOptional(run.wallTimeMs, 2, " ms")],
        ["CPU time", fmtOptional(run.threadCpuTimeMs, 2, " ms")],
        ["CPU / tick", fmtOptional(run.cpuTimePerTickUs, 2, " us")],
        ["Sim rate", fmtOptional(run.simRateX, 1, "x")],
        ["Step rate", fmtOptional(run.physicsStepsPerS, 0, " steps/s")],
        ["Sim time", `${fmt(reportData.manifest.simTimeS, 2)} s`],
        ["Physics steps", String(reportData.manifest.physicsSteps)],
        ["Control updates", String(reportData.manifest.controllerUpdates)],
      ]);
    };

    const renderMissionProfile = () => {
      const mission = reportData.missionDetails;
      setText("mission-description", mission.description || "No scenario description provided.");
      renderMissionList("mission-card-scenario", [
        ["Seed", String(mission.scenarioSeed)],
        ["Tags", Array.isArray(mission.tags) && mission.tags.length ? mission.tags.join(", ") : "none"],
        ["Terrain points", String(mission.terrainPointCount)],
        ["Rates", `${mission.sim.physicsHz} Hz physics / ${mission.sim.controllerHz} Hz control${mission.sim.sampleHz ? ` / ${mission.sim.sampleHz} Hz samples` : ""}`],
        ["Max time", `${fmt(mission.sim.maxTimeS, 1)} s`],
      ]);
      renderMissionList("mission-card-initial", [
        ["Position", `${fmt(mission.initialState.xM)} m, ${fmt(mission.initialState.yM)} m`],
        ["Velocity", `${fmt(mission.initialState.vxMps)} m/s, ${fmt(mission.initialState.vyMps)} m/s`],
        ["Speed", `${fmt(mission.initialState.speedMps)} m/s`],
        ["Attitude", `${fmt(mission.initialState.attitudeDeg, 1)} deg`],
        ["Angular rate", `${fmt(mission.initialState.angularRateDegps, 1)} deg/s`],
      ]);
      renderMissionList("mission-card-vehicle", [
        ["Hull", `${fmt(mission.vehicle.hullWidthM)} m × ${fmt(mission.vehicle.hullHeightM)} m`],
        ["Touchdown gear", `${fmt(mission.vehicle.touchdownHalfSpanM)} m span / ${fmt(mission.vehicle.touchdownBaseOffsetM)} m offset`],
        ["Mass", `${fmt(mission.vehicle.dryMassKg)} kg dry`],
        ["Fuel", `${fmtFuelPct(mission.vehicle.initialFuelKg)} start · ${fmt(mission.vehicle.initialFuelKg)} / ${fmt(mission.vehicle.maxFuelKg)} kg`],
        ["Thrust", `${fmt(mission.vehicle.maxThrustN, 0)} N max`],
        ["Burn / rotate", `${fmt(mission.vehicle.maxFuelBurnKgps)} kg/s / ${fmt(mission.vehicle.maxRotationRateDegps, 1)} deg/s`],
        ["Touchdown limits", `${fmt(mission.vehicle.safeTouchdownNormalSpeedMps)} n, ${fmt(mission.vehicle.safeTouchdownTangentialSpeedMps)} t, ${fmt(mission.vehicle.safeTouchdownAttitudeDeg, 1)} deg, ${fmt(mission.vehicle.safeTouchdownAngularRateDegps, 1)} deg/s`],
      ]);
      renderMissionList("mission-card-goal", [
        ["Goal", mission.mission.goalKind],
        ["Target pad", mission.targetPad.id],
        ["Pad geometry", `${fmt(mission.targetPad.centerXM)} m center / ${fmt(mission.targetPad.widthM)} m width`],
        ["Pad surface", `${fmt(mission.targetPad.surfaceYM)} m`],
        ["Transfer route", mission.transferRoute
          ? `${fmt(mission.transferRoute.routeAngleDeg, 1)} deg / ${fmt(mission.transferRoute.routeRadiusM)} m · ${mission.transferRoute.sourcePadId} -> ${mission.transferRoute.targetPadId}`
          : "n/a"],
        ["Waypoints", Array.isArray(mission.transferRoute?.waypoints) && mission.transferRoute.waypoints.length
          ? mission.transferRoute.waypoints.map((waypoint, index) =>
              `${index + 1}. ${waypoint.id}: (${fmt(waypoint.xM)}, ${fmt(waypoint.yM)}) m · r ${fmt(waypoint.captureRadiusM)} m / xtrack ${fmt(waypoint.maxCrossTrackM)} m · speed ${fmt(waypoint.minSpeedMps)}-${fmt(waypoint.maxSpeedMps)} m/s · vy ${fmt(waypoint.minVerticalSpeedMps)}-${fmt(waypoint.maxVerticalSpeedMps)} m/s`
            ).join("<br>")
          : "none"],
        ["Checkpoint", mission.mission.endTimeS != null ? `${fmt(mission.mission.endTimeS, 2)} s` : "n/a"],
        ["Evaluation", mission.evaluationBasis],
      ]);
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
      waypoints.forEach((waypoint) => {
        const x = Number(waypoint.xM);
        const y = Number(waypoint.yM);
        const radius = Math.max(
          Number(waypoint.captureRadiusM),
          Number(waypoint.maxCrossTrackM),
          0
        );
        if (![x, y, radius].every((value) => Number.isFinite(value))) return;
        xs.push(x - radius, x + radius);
        ys.push(y - radius, y + radius);
      });
      [...plannerSafeProfile, ...plannerCenterline].forEach((point) => {
        const x = Number(point.x);
        const y = Number(point.y);
        if (Number.isFinite(x) && Number.isFinite(y)) {
          xs.push(x);
          ys.push(y);
        }
      });
      if (!xs.length || !ys.length) return { span: 1 };
      const minX = Math.min(...xs);
      const maxX = Math.max(...xs);
      const minY = Math.min(...ys);
      const maxY = Math.max(...ys);
      return { span: Math.max(maxX - minX, maxY - minY, 1) };
    };

    const buildHoverCarrier = () => ({
      type: "scatter",
      mode: "lines",
      name: "hover-carrier",
      x: xValues,
      y: yValues,
      customdata: samples.map((sample, index) => [index]),
      line: { color: "rgba(14,107,96,0.002)", width: 18 },
      hovertemplate: "t=%{customdata[0]}<extra></extra>",
      showlegend: false,
    });

    const buildScalarSpatialTraces = ({ values, colorscale, colorbarTitle }) => {
      const traces = [{
        type: "scatter",
        mode: "lines",
        x: xValues,
        y: yValues,
        line: { color: "rgba(61,50,39,0.36)", width: 2.4 },
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
            width: 4.5,
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

    const buildEventGuideShapes = () =>
      keyEvents
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
            layer: "above",
            line: { color: eventStyle(event.kind).color, width: 1.2, dash: "dot" },
          };
        })
        .filter(Boolean);

    const buildPhaseBandShapes = () => {
      if (!samples.length) return [];
      const bands = [];
      let start = 0;
      const appendBand = (end, bandIndex) => {
        const x0 = Number(samples[start]?.simTimeS);
        const x1 = Number(samples[end]?.simTimeS);
        if (!Number.isFinite(x0) || !Number.isFinite(x1) || x1 <= x0) return;
        bands.push({
          type: "rect",
          xref: "x",
          yref: "paper",
          x0,
          x1,
          y0: 0,
          y1: 1,
          layer: "below",
          line: { width: 0 },
          fillcolor: bandIndex % 2 === 0
            ? "rgba(14,107,96,0.055)"
            : "rgba(217,119,6,0.045)",
        });
      };
      let bandIndex = 0;
      for (let index = 1; index < samples.length; index += 1) {
        if ((samples[index]?.phase || "") === (samples[start]?.phase || "")) continue;
        appendBand(index, bandIndex);
        start = index;
        bandIndex += 1;
      }
      appendBand(samples.length - 1, bandIndex);
      return bands;
    };

    const ballisticEndTime = ({ startY, targetY, vyMps, gravityMps2 }) => {
      const g = Math.max(1e-6, Math.abs(Number(gravityMps2)));
      const a = 0.5 * g;
      const b = -Number(vyMps);
      const c = Number(targetY) - Number(startY);
      const discriminant = (b * b) - (4 * a * c);
      if (!Number.isFinite(discriminant) || discriminant < 0) return null;
      const sqrt = Math.sqrt(discriminant);
      const roots = [(-b - sqrt) / (2 * a), (-b + sqrt) / (2 * a)]
        .filter((value) => Number.isFinite(value) && value > 1e-6);
      if (!roots.length) return null;
      return Math.max(...roots);
    };

    const ballisticCurveFromState = ({ startX, startY, vxMps, vyMps, targetY, gravityMps2 }) => {
      const endTime = ballisticEndTime({ startY, targetY, vyMps, gravityMps2 });
      if (!Number.isFinite(endTime)) return null;
      const g = Math.max(1e-6, Math.abs(Number(gravityMps2)));
      const pointCount = Math.max(20, Math.min(72, Math.round(16 + (endTime * 8))));
      const xs = [];
      const ys = [];
      for (let index = 0; index < pointCount; index += 1) {
        const t = (endTime * index) / (pointCount - 1);
        xs.push(Number(startX) + (Number(vxMps) * t));
        ys.push(Number(startY) + (Number(vyMps) * t) - (0.5 * g * t * t));
      }
      if (ys.length) ys[ys.length - 1] = Number(targetY);
      return { xs, ys, endTime };
    };

    const idealizedReferenceKinematics = ({ startX, startY, targetX, targetY, apexY, gravityMps2 }) => {
      const g = Math.max(1e-6, Math.abs(Number(gravityMps2)));
      const peakY = Math.max(Number(startY), Number(apexY));
      const vyUp = Math.sqrt(Math.max(0, 2 * g * (peakY - Number(startY))));
      const flightTime = ballisticEndTime({
        startY: Number(startY),
        targetY: Number(targetY),
        vyMps: vyUp,
        gravityMps2,
      });
      if (!Number.isFinite(flightTime) || flightTime <= 1e-6) return null;
      return {
        flightTime,
        vxMps: (Number(targetX) - Number(startX)) / flightTime,
        vyUpMps: vyUp,
      };
    };

    const idealizedReferenceImpactAngleDeg = (params) => {
      const solution = idealizedReferenceKinematics(params);
      if (!solution) return null;
      const g = Math.max(1e-6, Math.abs(Number(params.gravityMps2)));
      const vyTarget = solution.vyUpMps - (g * solution.flightTime);
      return Math.atan2(Math.max(0, -vyTarget), Math.abs(solution.vxMps)) * (180 / Math.PI);
    };

    const idealizedReferenceExitAngleDeg = (params) => {
      const solution = idealizedReferenceKinematics(params);
      if (!solution) return null;
      return Math.atan2(Math.max(0, solution.vyUpMps), Math.abs(solution.vxMps)) * (180 / Math.PI);
    };

    const idealizedReferenceApexY = ({ startX, startY, targetX, targetY, gravityMps2 }) => {
      const dx = Number(targetX) - Number(startX);
      const dy = Number(targetY) - Number(startY);
      let basePeak = Number(targetY) > Number(startY)
        ? Math.max(Number(startY), Number(targetY) + 1.0)
        : Number(startY);
      if (Math.abs(dx) <= 1e-6) return basePeak;
      const paramsBase = { startX, startY, targetX, targetY, gravityMps2 };
      const meetsAngleFloor = (peakY) => {
        const impactAngle = idealizedReferenceImpactAngleDeg({ ...paramsBase, apexY: peakY });
        return Number.isFinite(impactAngle) && impactAngle >= 45.0;
      };
      if (meetsAngleFloor(basePeak)) return basePeak;
      let lowPeak = basePeak;
      let growth = Math.max(16.0, 0.25 * Math.max(Math.abs(dx), Math.abs(dy), 1.0));
      let highPeak = null;
      let candidatePeak = basePeak;
      for (let index = 0; index < 16; index += 1) {
        candidatePeak += growth;
        if (meetsAngleFloor(candidatePeak)) {
          highPeak = candidatePeak;
          break;
        }
        lowPeak = candidatePeak;
        growth *= 2.0;
      }
      if (!Number.isFinite(highPeak)) return candidatePeak;
      for (let index = 0; index < 32; index += 1) {
        const midPeak = 0.5 * (lowPeak + highPeak);
        if (meetsAngleFloor(midPeak)) {
          highPeak = midPeak;
        } else {
          lowPeak = midPeak;
        }
      }
      return highPeak;
    };

    const idealizedReferenceCurve = ({ startX, startY, targetX, targetY, gravityMps2 }) => {
      const apexY = idealizedReferenceApexY({ startX, startY, targetX, targetY, gravityMps2 });
      const solution = idealizedReferenceKinematics({
        startX,
        startY,
        targetX,
        targetY,
        apexY,
        gravityMps2,
      });
      if (!solution) return null;
      const g = Math.max(1e-6, Math.abs(Number(gravityMps2)));
      const pointCount = Math.max(24, Math.min(84, Math.round(18 + (solution.flightTime * 10))));
      const xs = [];
      const ys = [];
      for (let index = 0; index < pointCount; index += 1) {
        const t = (solution.flightTime * index) / (pointCount - 1);
        xs.push(Number(startX) + (solution.vxMps * t));
        ys.push(Number(startY) + (solution.vyUpMps * t) - (0.5 * g * t * t));
      }
      if (xs.length) {
        xs[xs.length - 1] = Number(targetX);
        ys[ys.length - 1] = Number(targetY);
      }
      return { xs, ys };
    };

    const buildVectorSampleIndices = () => {
      if (timeValues.length <= 1) return timeValues.length ? [0] : [];
      const totalT = Math.max(0, Number(timeValues[timeValues.length - 1]) - Number(timeValues[0]));
      const intervalS = Math.max(0.22, totalT / 30.0);
      const picked = [];
      let nextT = Number(timeValues[0]);
      for (let index = 0; index < timeValues.length; index += 1) {
        const timeValue = Number(timeValues[index]);
        if (!Number.isFinite(timeValue)) continue;
        if (!picked.length || timeValue >= nextT - 1e-9) {
          picked.push(index);
          nextT = timeValue + intervalS;
        }
      }
      if (picked[picked.length - 1] !== timeValues.length - 1) {
        picked.push(timeValues.length - 1);
      }
      return picked;
    };

    const buildVectorAnnotations = () => {
      const annotations = [];
      const span = spatialBounds().span;
      const vectorLength = 0.048 * span;
      for (const index of buildVectorSampleIndices()) {
        const throttle = Number(samples[index].throttleFrac);
        if (!Number.isFinite(throttle) || throttle <= 0.015) continue;
        const attitudeRad = Number(samples[index].attitudeRad);
        const x0 = Number(samples[index].xM);
        const y0 = Number(samples[index].yM);
        if (![attitudeRad, x0, y0].every((value) => Number.isFinite(value))) continue;
        const dx = Math.sin(attitudeRad) * vectorLength * throttle;
        const dy = Math.cos(attitudeRad) * vectorLength * throttle;
        if (Math.hypot(dx, dy) <= 0.0045 * span) continue;
        const arrowBase = {
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
          arrowsize: 0.82,
        };
        annotations.push({
          ...arrowBase,
          arrowwidth: 6.2,
          arrowcolor: "rgba(255,250,240,0.94)",
        });
        annotations.push({
          ...arrowBase,
          arrowwidth: 3.2,
          arrowcolor: interpolateColor(throttleColorScale, throttle, 0.0, 1.0),
        });
      }
      return annotations;
    };

    const circlePoints = ({ centerX, centerY, radius, points = 80 }) => {
      const xs = [];
      const ys = [];
      for (let index = 0; index <= points; index += 1) {
        const theta = (Math.PI * 2 * index) / points;
        xs.push(Number(centerX) + (Math.cos(theta) * Number(radius)));
        ys.push(Number(centerY) + (Math.sin(theta) * Number(radius)));
      }
      return { xs, ys };
    };

    const buildWaypointRouteTraces = () => {
      if (!waypoints.length) return [];
      const traces = [];
      const initial = reportData.missionDetails.initialState || null;
      if (initial && pad) {
        const routePoints = [
          { x: Number(initial.xM), y: Number(initial.yM) },
          ...waypoints.map((waypoint) => ({ x: Number(waypoint.xM), y: Number(waypoint.yM) })),
          { x: Number(pad.centerXM), y: Number(pad.surfaceYM) },
        ].filter((point) => Number.isFinite(point.x) && Number.isFinite(point.y));
        for (let index = 1; index < routePoints.length; index += 1) {
          const previous = routePoints[index - 1];
          const current = routePoints[index];
          const legStyle = waypointLegStyles[Math.min(index - 1, waypointLegStyles.length - 1)];
          traces.push({
            type: "scatter",
            mode: "lines",
            name: legStyle.name,
            x: [previous.x, current.x],
            y: [previous.y, current.y],
            line: { color: legStyle.color, width: 1.5, dash: legStyle.dash },
            opacity: 0.58,
            hoverinfo: "skip",
            showlegend: true,
          });
        }
      }
      return traces;
    };

    const buildWaypointEnvelopeTraces = () => {
      if (!waypoints.length) return [];
      const traces = [];
      waypoints.forEach((waypoint, index) => {
        const x = Number(waypoint.xM);
        const y = Number(waypoint.yM);
        const captureRadius = Number(waypoint.captureRadiusM);
        const crossTrack = Number(waypoint.maxCrossTrackM);
        if (![x, y].every(Number.isFinite)) return;
        if (Number.isFinite(crossTrack) && crossTrack > 0) {
          const circle = circlePoints({ centerX: x, centerY: y, radius: crossTrack });
          traces.push({
            type: "scatter",
            mode: "lines",
            name: "waypoint cross-track",
            x: circle.xs,
            y: circle.ys,
            line: { color: "rgba(162,90,24,0.44)", width: 1.2, dash: "dot" },
            hoverinfo: "skip",
            showlegend: index === 0,
          });
        }
        if (Number.isFinite(captureRadius) && captureRadius > 0) {
          const circle = circlePoints({ centerX: x, centerY: y, radius: captureRadius });
          traces.push({
            type: "scatter",
            mode: "lines",
            name: "waypoint capture radius",
            x: circle.xs,
            y: circle.ys,
            line: { color: "#a25a18", width: 1.5 },
            fill: "toself",
            fillcolor: "rgba(240,140,0,0.07)",
            hoverinfo: "skip",
            showlegend: index === 0,
          });
        }
      });
      return traces;
    };

    const buildWaypointMarkerTraces = () => {
      if (!waypoints.length) return [];
      const traces = [];
      waypoints.forEach((waypoint, index) => {
        const x = Number(waypoint.xM);
        const y = Number(waypoint.yM);
        const captureRadius = Number(waypoint.captureRadiusM);
        const crossTrack = Number(waypoint.maxCrossTrackM);
        if (![x, y].every(Number.isFinite)) return;
        traces.push({
          type: "scatter",
          mode: "markers+text",
          name: "waypoint",
          x: [x],
          y: [y],
          text: [`WP${index + 1}`],
          textposition: "top right",
          textfont: { color: "#7a3d0f", size: 11 },
          customdata: [[
            waypoint.id || `waypoint_${index + 1}`,
            captureRadius,
            crossTrack,
            Number(waypoint.maxOutboundHeadingErrorDeg),
            Number(waypoint.minOutboundProgressMps),
            Number(waypoint.minSpeedMps),
            Number(waypoint.maxSpeedMps),
            Number(waypoint.minVerticalSpeedMps),
            Number(waypoint.maxVerticalSpeedMps),
            Number(waypoint.handoffTangentHeadingDeg),
          ]],
          hovertemplate:
            "%{customdata[0]}<br>x=%{x:.1f} m<br>y=%{y:.1f} m<br>capture=%{customdata[1]:.1f} m<br>xtrack=%{customdata[2]:.1f} m<br>planned tangent=%{customdata[9]:.1f} deg<br>tangent err <= %{customdata[3]:.1f} deg<br>handoff progress >= %{customdata[4]:.1f} m/s<br>speed %{customdata[5]:.1f}-%{customdata[6]:.1f} m/s<br>vy %{customdata[7]:.1f}-%{customdata[8]:.1f} m/s<extra></extra>",
          marker: {
            size: 12,
            color: "#7a3d0f",
            symbol: "circle",
            line: { width: 2, color: "#fffaf0" },
          },
          showlegend: index === 0,
        });
      });
      return traces;
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
        line: { color: "#0e6b60", width: 3.2 },
        hoverinfo: "skip",
      };
      const hoverTrace = {
        ...buildHoverCarrier(),
        customdata: samples.map((sample, index) => [index, Number(sample.simTimeS), Number(sample.speedMps), sample.phase || "", sample.status || "", Number(sample.throttleFrac)]),
        hovertemplate:
          "t=%{customdata[1]:.2f}s<br>x=%{x:.1f}<br>y=%{y:.1f}<br>speed=%{customdata[2]:.2f}<br>throttle=%{customdata[5]:.2f}<br>phase=%{customdata[3]}<br>%{customdata[4]}<extra></extra>",
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
      const waypointMarkerDetail = (marker) => {
        if (marker.id !== "waypoint/handoff") return "";
        const metrics = marker.metrics || {};
        const index = Number(metrics["waypoint.index"]);
        const speed = Number(metrics["waypoint.speed_mps"]);
        const heading = Number(metrics["waypoint.outbound_heading_error_rad"]);
        const turnMargin = Number(metrics["waypoint.turn_margin_m"]);
        const handoffMargin = Number(metrics["waypoint.handoff_turn_margin_m"]);
        const velocityError = Number(metrics["waypoint.target_velocity_error_mps"]);
        const deadlineRemaining = Number(metrics["waypoint.target_deadline_remaining_s"]);
        const targetMode = metrics["waypoint.handoff_target_mode"];
        const feasible = metrics["waypoint.guidance_feasible"];
        const replans = Number(metrics["waypoint.guidance_replan_count"]);
        const resolution = metrics["waypoint.handoff_resolution_reason"];
        const windowDuration = Number(metrics["waypoint.handoff_window_duration_s"]);
        const parts = [
          `${metrics["waypoint.id"] || `waypoint ${Number.isFinite(index) ? index + 1 : "?"}`} · ${metrics["waypoint.capture_status"] || "handoff"}`,
        ];
        if (Number.isFinite(speed)) parts.push(`speed=${speed.toFixed(1)}m/s`);
        if (Number.isFinite(heading)) parts.push(`tangent error=${(heading * 180 / Math.PI).toFixed(1)}deg`);
        if (resolution) parts.push(`resolution=${resolution}`);
        if (Number.isFinite(windowDuration)) parts.push(`window=${windowDuration.toFixed(2)}s`);
        if (Number.isFinite(turnMargin)) parts.push(`turn margin=${turnMargin.toFixed(1)}m`);
        if (Number.isFinite(velocityError)) parts.push(`target Δv=${velocityError.toFixed(1)}m/s`);
        if (Number.isFinite(deadlineRemaining)) parts.push(`deadline=${deadlineRemaining.toFixed(2)}s`);
        if (Number.isFinite(handoffMargin)) parts.push(`handoff margin=${handoffMargin.toFixed(1)}m`);
        if (targetMode) parts.push(`target=${targetMode}`);
        if (typeof feasible === "boolean") parts.push(`feasible=${feasible ? "yes" : "no"}`);
        if (Number.isFinite(replans)) parts.push(`replans=${replans.toFixed(0)}`);
        return parts.join("<br>");
      };
      const markerTrace = {
        type: "scatter",
        mode: "markers",
        name: "controller markers",
        x: markers.map((marker) => Number(marker.xM)),
        y: markers.map((marker) => Number(marker.yM)),
        customdata: markers.map((marker) => [
          marker.label,
          Number(marker.simTimeS),
          marker.phase || "",
          waypointMarkerDetail(marker),
        ]),
        hovertemplate: "%{customdata[0]}<br>%{customdata[3]}<br>phase=%{customdata[2]}<br>t=%{customdata[1]:.2f}s<extra></extra>",
        marker: {
          size: markers.map((marker) => marker.id === "waypoint/handoff" ? 9 : 5),
          color: markers.map((marker) => marker.id === "waypoint/handoff" ? "#b45309" : "#4c6ef5"),
          symbol: "diamond",
          opacity: markers.map((marker) => marker.id === "waypoint/handoff" ? 0.92 : 0.42),
          line: { width: 0 },
        },
      };

      const gravityMps2 = Number(reportData.missionDetails.gravityMps2 || 0);
      const initial = reportData.missionDetails.initialState || null;
      const ballisticCurve = (initial && pad)
        ? ballisticCurveFromState({
            startX: Number(initial.xM),
            startY: Number(initial.yM),
            vxMps: Number(initial.vxMps),
            vyMps: Number(initial.vyMps),
            targetY: Number(pad.surfaceYM),
            gravityMps2,
          })
        : null;
      const ballisticTrace = ballisticCurve ? {
        type: "scatter",
        mode: "lines",
        name: "start ballistic",
        x: ballisticCurve.xs,
        y: ballisticCurve.ys,
        line: { color: "#cf7b00", width: 1.5, dash: "dot" },
        hoverinfo: "skip",
      } : null;
      const referenceCurve = (initial && pad)
        ? idealizedReferenceCurve({
            startX: Number(initial.xM),
            startY: Number(initial.yM),
            targetX: Number(pad.centerXM),
            targetY: Number(pad.surfaceYM),
            gravityMps2,
          })
        : null;
      const referenceTrace = referenceCurve ? {
        type: "scatter",
        mode: "lines",
        name: "idealized reference",
        x: referenceCurve.xs,
        y: referenceCurve.ys,
        line: { color: "#5b73c6", width: 1.8, dash: "dash" },
        hoverinfo: "skip",
      } : null;

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
      const waypointRouteTraces = buildWaypointRouteTraces();
      const waypointEnvelopeTraces = buildWaypointEnvelopeTraces();
      const waypointMarkerTraces = buildWaypointMarkerTraces();
      const plannerTraces = [];
      if (plannerSafeProfile.length) {
        plannerTraces.push({
          type: "scatter",
          mode: "lines",
          name: "planner safe profile",
          x: plannerSafeProfile.map((point) => Number(point.x)),
          y: plannerSafeProfile.map((point) => Number(point.y)),
          line: { color: "#9b59b6", width: 1.4, dash: "dot" },
          hoverinfo: "skip",
        });
      }
      if (plannerCenterline.length) {
        plannerTraces.push({
          type: "scatter",
          mode: "lines",
          name: "planner selected centerline",
          x: plannerCenterline.map((point) => Number(point.x)),
          y: plannerCenterline.map((point) => Number(point.y)),
          line: { color: "#e8590c", width: 1.5, dash: "dashdot" },
          hoverinfo: "skip",
        });
      }
      const waypointWindowEntries = markers
        .filter((marker) => marker.id === "waypoint/handoff")
        .map((marker) => {
          const metrics = marker.metrics || {};
          return {
            x: Number(metrics["waypoint.window_entry_position_x_m"]),
            y: Number(metrics["waypoint.window_entry_position_y_m"]),
            time: Number(metrics["waypoint.window_entry_time_s"]),
            pass: metrics["waypoint.window_entry_contract_pass"],
            reasons: metrics["waypoint.window_entry_contract_reasons"] || "",
            id: metrics["waypoint.id"] || "waypoint",
          };
        })
        .filter((entry) => Number.isFinite(entry.x) && Number.isFinite(entry.y));
      if (waypointWindowEntries.length) {
        waypointMarkerTraces.push({
          type: "scatter",
          mode: "markers",
          name: "waypoint window entry",
          x: waypointWindowEntries.map((entry) => entry.x),
          y: waypointWindowEntries.map((entry) => entry.y),
          customdata: waypointWindowEntries.map((entry) => [
            entry.id,
            entry.time,
            entry.pass === true ? "pass" : "pending",
            entry.reasons,
          ]),
          hovertemplate: "%{customdata[0]} window entry<br>t=%{customdata[1]:.2f}s<br>entry contract=%{customdata[2]}<br>%{customdata[3]}<extra></extra>",
          marker: {
            size: 10,
            color: waypointWindowEntries.map((entry) => entry.pass === true ? "#2f9e44" : "#d97706"),
            symbol: "circle-open",
            line: { width: 2.2 },
          },
        });
      }
      const vectorAnnotations = buildVectorAnnotations();
      const flightHandoffTrace = flightCorrections.length ? {
        type: "scatter", mode: "markers+text", name: "Waypoint handoffs",
        x: flightCorrections.map(c => c.handoff.positionM.x),
        y: flightCorrections.map(c => c.handoff.positionM.y),
        text: flightCorrections.map(c => `H${c.number}`),
        textposition: flightCorrections.map((_c, i) => i % 2 ? "bottom center" : "top center"),
        customdata: flightCorrections.map(c => [c.number, c.handoff.physicsStep, c.handoff.simTimeS,
          c.handoff.velocityMps.x, c.handoff.velocityMps.y]),
        hovertemplate: "H%{customdata[0]} — replan from actual state<br>step=%{customdata[1]} · t=%{customdata[2]}s<br>position=(%{x}, %{y})m<br>velocity=(%{customdata[3]}, %{customdata[4]})m/s<extra></extra>",
        marker: { symbol: "circle-open", color: "#b45309", size: flightCorrections.map((_c, i) => i === selectedFlightCorrection ? 13 : 9), line: { width: 2 } },
        textfont: { color: "#b45309", size: 12 },
      } : null;
      const spatialTraces = [
        terrainTrace,
        ...(padTrace ? [padTrace] : []),
        ...plannerTraces,
        ...waypointEnvelopeTraces,
        markerTrace,
        plainTrace,
        ...speedTraces,
        ...throttleTraces,
        ...(ballisticTrace ? [ballisticTrace] : []),
        ...(referenceTrace ? [referenceTrace] : []),
        ...waypointRouteTraces,
        ...waypointMarkerTraces,
        eventTrace,
        hoverTrace,
        ...(flightHandoffTrace ? [flightHandoffTrace] : []),
      ];
      const plannerStart = padTrace ? 2 : 1;
      const plannerEnd = plannerStart + plannerTraces.length;
      const waypointEnvelopeStart = plannerEnd;
      const waypointEnvelopeEnd = waypointEnvelopeStart + waypointEnvelopeTraces.length;
      const markerIndex = waypointEnvelopeEnd;
      const plainIndex = markerIndex + 1;
      const speedStart = plainIndex + 1;
      const speedEnd = speedStart + speedTraces.length;
      const throttleStart = speedEnd;
      const throttleEnd = throttleStart + throttleTraces.length;
      const ballisticIndex = ballisticTrace ? throttleEnd : -1;
      const referenceIndex = referenceTrace ? throttleEnd + (ballisticTrace ? 1 : 0) : -1;
      const waypointRouteStart = throttleEnd + (ballisticTrace ? 1 : 0) + (referenceTrace ? 1 : 0);
      const waypointRouteEnd = waypointRouteStart + waypointRouteTraces.length;
      const waypointMarkerStart = waypointRouteEnd;
      const waypointMarkerEnd = waypointMarkerStart + waypointMarkerTraces.length;
      const eventIndex = spatialTraces.length - (flightHandoffTrace ? 3 : 2);
      const hoverIndex = eventIndex + 1;
      flightHandoffTraceIndex = flightHandoffTrace ? hoverIndex + 1 : -1;
      const baseVisible = new Set([0, plainIndex, eventIndex, hoverIndex]);
      if (padTrace) baseVisible.add(1);
      for (let index = plannerStart; index < plannerEnd; index += 1) baseVisible.add(index);
      const inRange = (index, start, end) => index >= start && index < end;
      const visibilityForMode = (mode) => spatialTraces.map((_trace, index) => {
        if (index === flightHandoffTraceIndex) return flightHandoffsVisible;
        if (baseVisible.has(index)) return true;
        if (mode === "mission") {
          return index === referenceIndex || inRange(index, waypointMarkerStart, waypointMarkerEnd);
        }
        if (mode === "guidance") {
          return inRange(index, waypointEnvelopeStart, waypointEnvelopeEnd)
            || index === markerIndex
            || index === ballisticIndex
            || index === referenceIndex
            || inRange(index, waypointRouteStart, waypointRouteEnd)
            || inRange(index, waypointMarkerStart, waypointMarkerEnd);
        }
        if (mode === "speed") return inRange(index, speedStart, speedEnd);
        if (mode === "throttle") return inRange(index, throttleStart, throttleEnd);
        return false;
      });

      const spatialElement = document.getElementById("chart-spatial");
      Plotly.newPlot(
        spatialElement,
        spatialTraces.map((trace, index) => ({ ...trace, visible: visibilityForMode("mission")[index] })),
        spatialLayout({
          hovermode: "closest",
          hoverdistance: 32,
          xaxis: axisStyle({ title: "" }),
          yaxis: axisStyle({ title: "", scaleanchor: "x", scaleratio: 1 }),
          annotations: [],
        }),
        spatialConfig,
      );

      const toolbar = document.getElementById("spatial-mode-toolbar");
      const applyMode = (mode) => {
        for (const button of toolbar.querySelectorAll("button[data-mode]")) {
          button.classList.toggle("active", button.dataset.mode === mode);
        }
        Plotly.update(
          spatialElement,
          { visible: visibilityForMode(mode) },
          { annotations: mode === "vectors" ? vectorAnnotations : [] }
        );
      };
      for (const button of toolbar.querySelectorAll("button[data-mode]")) {
        button.addEventListener("click", () => applyMode(button.dataset.mode || "mission"));
      }

      spatialElement.on("plotly_hover", (eventData) => {
        const points = Array.isArray(eventData?.points) ? eventData.points : [];
        const handoffPoint = points.find(point => point.curveNumber === flightHandoffTraceIndex);
        if (handoffPoint) { selectFlightCorrection(handoffPoint.pointIndex); return; }
        const hoverPoint = points.find((point) => point.curveNumber === hoverIndex);
        if (!hoverPoint || !Array.isArray(hoverPoint.customdata)) return;
        updateInspect(Number(hoverPoint.customdata[0]));
      });
    };

    const buildMetricsPlot = () => {
      baseFlightMetricShapes = [...buildPhaseBandShapes(), ...buildEventGuideShapes()];
      const guideShapes = [...baseFlightMetricShapes, ...buildFlightGuideShapes()];
      Plotly.newPlot(
        "chart-metrics",
        [
          {
            type: "scatter",
            mode: "lines",
            name: "velocity",
            x: timeValues,
            y: speedValues,
            line: { color: "#1f8f63", width: 3.4 },
          },
          {
            type: "scatter",
            mode: "lines",
            name: "vx",
            x: timeValues,
            y: vxValues,
            line: { color: "#2f9e44", width: 2.6, dash: "dot" },
            visible: "legendonly",
          },
          {
            type: "scatter",
            mode: "lines",
            name: "vy",
            x: timeValues,
            y: vyValues,
            line: { color: "#5b73c6", width: 2.6, dash: "dash" },
            visible: "legendonly",
          },
          {
            type: "scatter",
            mode: "lines",
            name: "thrust",
            x: timeValues,
            y: throttleValues,
            line: { color: "#d97706", width: 3.2 },
            yaxis: "y2",
          },
          {
            type: "scatter",
            mode: "lines",
            name: "tx",
            x: timeValues,
            y: throttleXValues,
            line: { color: "#0b7285", width: 2.4, dash: "dot" },
            yaxis: "y2",
            visible: "legendonly",
          },
          {
            type: "scatter",
            mode: "lines",
            name: "ty",
            x: timeValues,
            y: throttleYValues,
            line: { color: "#cf7b00", width: 2.4, dash: "dash" },
            yaxis: "y2",
            visible: "legendonly",
          },
        ],
        metricLayout({
          hovermode: "x unified",
          xaxis: axisStyle({ title: "Time (s)" }),
          yaxis: axisStyle({ title: "Velocity (m/s)", zeroline: true }),
          yaxis2: axisStyle({ title: "Thrust (0..1)", overlaying: "y", side: "right", zeroline: true }),
          shapes: guideShapes,
          annotations: buildFlightGuideLabels(),
        }),
        compactConfig,
      );

      const element = document.getElementById("chart-metrics");
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
      ["Fuel", `${fmtFuelPct(sample.fuelKg)} · ${fmt(sample.fuelKg)} kg`],
      ["Bot step", fmtOptional(sample.computeTimeMs, 3, " ms")],
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
      renderBreadcrumbs();
      summarizeOutcome();
      renderKeyStats();
      renderCountChips("event-chips", reportData.eventCounts, "No key events recorded.");
      renderCountChips("marker-chips", reportData.markerCounts, "No controller markers recorded.");
      renderPhaseChips();
      renderEventList();
      renderControllerSpec();
      renderQuality();
      renderFlightStats();
      renderBotStats();
      renderRunStats();
      renderPlannerEvidence();
      renderMissionProfile();
      buildSpatialPlot();
      buildMetricsPlot();
      initFlightCorrections();
      updateInspect(samples.length ? samples.length - 1 : 0);
    };

    init();
  </script>
</body>
</html>
"####
}
