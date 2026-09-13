//! Display-only report for the generic ridge runtime V2 development artifact.
//!
//! The renderer accepts serialized JSON so it cannot call the planner,
//! candidate search, runtime selector, controller, or simulator.  The
//! evaluator validates the artifact before handing it here; this module only
//! formats the supplied evidence into deterministic HTML and SVG.

use std::{fmt::Write as _, fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

const REPORT_TITLE: &str = "Conservative Ballistic Handoff Development Regression";

pub fn write_conservative_ballistic_handoff_development_report(
    path: &Path,
    data: &Value,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create handoff development report directory {}",
                parent.display()
            )
        })?;
    }
    let html = build_report_html(data)?;
    fs::write(path, html).with_context(|| {
        format!(
            "failed to write handoff development report {}",
            path.display()
        )
    })?;
    Ok(())
}

fn build_report_html(data: &Value) -> Result<String> {
    let cases = data
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut overview = String::new();
    let mut sections = String::new();
    for case in &cases {
        let case_id = text(case, "case_id");
        let historical = text_path(case, &["historical_v1", "status"]);
        let v2 = selected_kind(case).unwrap_or_else(|| "unavailable".to_owned());
        let _ = write!(
            overview,
            "<tr><td><code>{}</code></td><td>{}</td><td><code>{}</code></td><td class=\"{}\">{}</td></tr>",
            escape(&case_id),
            escape(&historical),
            escape(&v2),
            if historical == "passed" {
                "pass"
            } else {
                "stop"
            },
            if case_id == "ridge_progress_068_probe" {
                "primary heading stop → bridge-exit pass"
            } else {
                "primary crossing retained"
            },
        );
        sections.push_str(&render_case(case));
    }
    let serialized = serde_json::to_string(data)?;
    Ok(format!(
        r###"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>{title}</title>
  <style>{css}</style>
</head>
<body>
  <main>
    <header class="hero">
      <div class="eyebrow">powered descent lab · development regression</div>
      <h1>{title}</h1>
      <p class="lede"><strong>development regression</strong> · <strong>controller-free</strong> · <strong>no simulation</strong>. Current 050/068 are exposed development inputs, <strong>not held-out evidence</strong>.</p>
      <p class="lede"><strong>Historical H2/H4 STOP remains unchanged.</strong> The historical generic V1 crossing-only result is shown beside the new V2 two-state diagnostic; this page does not authorize a controller lane or physical execution.</p>
      <div class="chips"><span>schema {schema} / v{version}</span><span>setup {setup}</span><span>repeat {repeat}</span><span>controller_run {controller}</span><span>simulation_run {simulation}</span></div>
    </header>

    <section class="panel">
      <div class="panel-head"><div><div class="eyebrow">Decision summary</div><h2>Exposed development cases</h2></div><span class="badge">analytical only</span></div>
      <div class="table-wrap"><table><thead><tr><th>Case</th><th>Historical V1</th><th>V2 selected state</th><th>Causal reading</th></tr></thead><tbody>{overview}</tbody></table></div>
      <dl class="facts">
        <div><dt>Input manifest identity</dt><dd><code>{manifest}</code></dd></div>
        <div><dt>Artifact identity</dt><dd><code>{identity}</code></dd></div>
      </dl>
    </section>

    <section class="panel">
      <div class="panel-head"><div><div class="eyebrow">Visual legend</div><h2>Setup geometry and handoff states</h2></div></div>
      <div class="legend"><span><i class="swatch composed"></i>certified composed analytical reference</span><span><i class="swatch terrain"></i>terrain/ridge and pads</span><span><i class="swatch anchor"></i>virtual anchor</span><span><i class="swatch primary"></i>primary exact crossing</span><span><i class="swatch selected"></i>selected V2 state/route</span><span><i class="swatch velocity"></i>velocity direction</span><span><i class="swatch tangent"></i>route tangent</span></div>
      <p class="muted">Markers and arrows are deterministic setup visuals. They are not controller commands, simulation telemetry, or a claim of ordinary full-route validation.</p>
    </section>

    {sections}

    <section class="panel provenance">
      <div class="panel-head"><div><div class="eyebrow">Provenance and non-claims</div><h2>Identity-bound development artifact</h2></div></div>
      <dl class="facts">
        <div><dt>Schema/setup</dt><dd><code>{schema}</code> / <code>{setup}</code></dd></div>
        <div><dt>Deterministic repeat</dt><dd>{repeat}</dd></div>
        <div><dt>Controller run</dt><dd>{controller}</dd></div>
        <div><dt>Simulation run</dt><dd>{simulation}</dd></div>
        <div><dt>Physical execution</dt><dd>{physical}</dd></div>
        <div><dt>Input manifest</dt><dd><code>{manifest}</code></dd></div>
        <div><dt>Artifact identity</dt><dd><code>{identity}</code></dd></div>
      </dl>
      <ul class="nonclaims"><li>050/068 are exposed development/regression inputs, not held-out evidence.</li><li>Historical H2/H4 STOP behavior and controller-shadow-v4 identity remain outside this new development lane and unchanged.</li><li>Candidate certification, structural route checks, authority, and handoff assessment do not establish controller or physical-execution success.</li></ul>
    </section>
  </main>
  <script type="application/json" id="conservative-ballistic-handoff-development-data">{data}</script>
</body>
</html>"###,
        title = escape(REPORT_TITLE),
        css = CSS,
        schema = escape(&text(data, "schema_id")),
        version = number_text(data, "schema_version"),
        setup = escape(&text(data, "setup_id")),
        repeat = escape(&text(data, "deterministic_repeat")),
        controller = escape(&text_path(data, &["scope", "controller_run"])),
        simulation = escape(&text_path(data, &["scope", "simulation_run"])),
        physical = escape(&text_path(data, &["scope", "physical_execution_run"])),
        manifest = escape(&text(data, "input_manifest_identity")),
        identity = escape(&text(data, "identity")),
        overview = overview,
        sections = sections,
        data = escape_json(&serialized),
    ))
}

fn render_case(case: &Value) -> String {
    let case_id = text(case, "case_id");
    let historical_status = text_path(case, &["historical_v1", "status"]);
    let selected_kind = selected_kind(case).unwrap_or_else(|| "unavailable".to_owned());
    let story = if case_id == "ridge_progress_068_probe" {
        "068 causal story: the historical V1 result is HandoffContractFailed because the primary crossing misses the heading limit only; V2 records that failed attempt, then selects the certified intermediate bridge exit, whose handoff passes."
    } else {
        "050 unchanged story: historical V1 passes at the primary exact crossing, and V2 retains that same route, authority, kinematics, and assessment with one attempt."
    };
    let attempts = path(
        case,
        &[
            "runtime_v2",
            "derived_mesa",
            "OneWaypoint",
            "handoff_selection",
            "attempts",
        ],
    )
    .and_then(Value::as_array)
    .cloned()
    .unwrap_or_default();
    let selected_index = path(
        case,
        &[
            "runtime_v2",
            "derived_mesa",
            "OneWaypoint",
            "handoff_selection",
            "selected_attempt_index",
        ],
    )
    .and_then(Value::as_u64)
    .unwrap_or(usize::MAX as u64) as usize;
    let mut cards = String::new();
    for (index, attempt) in attempts.iter().enumerate() {
        cards.push_str(&render_attempt(attempt, index, index == selected_index));
    }
    let selected_attempt = attempts.get(selected_index).unwrap_or(&Value::Null);
    let svg = build_case_svg(case, selected_attempt);
    format!(
        r###"<section class="panel case">
  <div class="panel-head"><div><div class="eyebrow">case regression</div><h2>{case}</h2></div><span class="badge {status_class}">historical V1: {historical}</span></div>
  <div class="callout {callout_class}">{story}</div>
  <p class="muted">V2 selection: <code>{selected}</code>. Candidate identity <code>{candidate}</code>; V2 runtime identity <code>{runtime}</code>; case identity <code>{case_identity}</code>.</p>
  {svg}
  <h3>Handoff attempts</h3>
  <div class="attempt-grid">{cards}</div>
</section>"###,
        case = escape(&case_id),
        status_class = if historical_status == "passed" {
            "pass"
        } else {
            "stop"
        },
        historical = escape(&historical_status),
        callout_class = if case_id == "ridge_progress_068_probe" {
            "causal"
        } else {
            "stable"
        },
        story = story,
        selected = escape(&selected_kind),
        candidate = escape(&text_path(case, &["candidate_projection", "identity"])),
        runtime = escape(&text_path(case, &["runtime_v2", "identity"])),
        case_identity = escape(&text(case, "identity")),
        svg = svg,
        cards = cards,
    )
}

fn render_attempt(attempt: &Value, index: usize, selected: bool) -> String {
    let kind = text(attempt, "selection_kind");
    let kind_label = match kind.as_str() {
        "primary_crossing" => "PrimaryCrossing",
        "intermediate_bridge_exit" => "IntermediateBridgeExit",
        _ => kind.as_str(),
    };
    let pass = path(attempt, &["handoff_assessment", "contract_pass"])
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let violations = path(attempt, &["handoff_assessment", "violations"])
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(escape)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let violations_text = if violations.is_empty() {
        "none".to_owned()
    } else {
        violations
    };
    let _ = index;
    format!(
        r###"<article class="attempt {selected_class}">
  <div class="attempt-head"><h4>{kind}</h4><span class="badge {pass_class}">{pass_label}</span>{selected_badge}</div>
  <dl class="metrics">
    <div><dt>Applied bridge step</dt><dd><code>{step}</code></dd></div>
    <div><dt>Target-leg arc step</dt><dd><code>{arc}</code></dd></div>
    <div><dt>Position (m)</dt><dd><code>{position}</code></dd></div>
    <div><dt>Velocity (m/s)</dt><dd><code>{velocity}</code></dd></div>
    <div><dt>Heading error / limit (rad)</dt><dd><code>{heading}</code> / <code>{heading_limit}</code></dd></div>
    <div><dt>Progress / minimum (m/s)</dt><dd><code>{progress}</code> / <code>{progress_limit}</code></dd></div>
    <div><dt>Cross speed / maximum (m/s)</dt><dd><code>{cross}</code> / <code>{cross_limit}</code></dd></div>
    <div><dt>Speed / authority cap (m/s)</dt><dd><code>{speed}</code> / <code>{authority}</code></dd></div>
    <div><dt>Violations</dt><dd class="{violation_class}">{violations}</dd></div>
  </dl>
  <p class="muted attempt-id">attempt identity <code>{identity}</code></p>
</article>"###,
        kind = escape(kind_label),
        selected_class = if selected { "selected" } else { "" },
        pass_class = if pass { "pass" } else { "stop" },
        pass_label = if pass { "PASS" } else { "FAIL" },
        selected_badge = if selected {
            "<span class=\"badge selected-badge\">SELECTED</span>"
        } else {
            ""
        },
        step = number_text(attempt, "applied_steps"),
        arc = number_or_none(attempt, "target_leg_arc_step"),
        position = format_point(path(attempt, &["selected_state", "position_m"])),
        velocity = format_point(path(attempt, &["selected_state", "velocity_mps"])),
        heading = number_path(
            attempt,
            &["handoff_kinematics", "outbound_heading_error_rad"]
        ),
        heading_limit = number_path(
            attempt,
            &["route", "waypoints", "0", "max_outbound_heading_error_rad"]
        ),
        progress = number_path(attempt, &["handoff_kinematics", "outbound_progress_mps"]),
        progress_limit = number_path(
            attempt,
            &["route", "waypoints", "0", "min_outbound_progress_mps"]
        ),
        cross = number_path(attempt, &["handoff_kinematics", "outbound_cross_speed_mps"]),
        cross_limit = number_path(
            attempt,
            &["route", "waypoints", "0", "max_outbound_cross_speed_mps"]
        ),
        speed = number_path(attempt, &["handoff_kinematics", "speed_mps"]),
        authority = number_path(attempt, &["authority", "handoff_speed_cap_mps"]),
        violation_class = if violations_text == "none" {
            "pass"
        } else {
            "stop"
        },
        violations = violations_text,
        identity = escape(&text(attempt, "identity")),
    )
}

fn build_case_svg(case: &Value, selected_attempt: &Value) -> String {
    let probe = path(case, &["input", "probe"]);
    let terrain = points(path_opt(probe, &["terrain_points_m"]));
    let source = pad_point(path_opt(probe, &["source"]));
    let target = pad_point(path_opt(probe, &["target"]));
    let candidate = path(
        case,
        &[
            "candidate_projection",
            "derived_mesa",
            "OneWaypoint",
            "candidate",
        ],
    );
    let composed_reference = composed_reference_points(candidate);
    let crossing = path(
        case,
        &[
            "runtime_v2",
            "derived_mesa",
            "OneWaypoint",
            "handoff_selection",
            "crossing",
        ],
    );
    let anchor = point(path_opt(crossing, &["virtual_anchor_m"]));
    let attempts = path(
        case,
        &[
            "runtime_v2",
            "derived_mesa",
            "OneWaypoint",
            "handoff_selection",
            "attempts",
        ],
    )
    .and_then(Value::as_array)
    .cloned()
    .unwrap_or_default();
    let primary = attempts
        .first()
        .and_then(|attempt| point(path_opt(Some(attempt), &["selected_state", "position_m"])));
    let selected = point(path_opt(
        Some(selected_attempt),
        &["selected_state", "position_m"],
    ));
    let route_waypoint = point(path_opt(
        Some(selected_attempt),
        &["route", "waypoints", "0", "position_m"],
    ));
    let initial = point(path_opt(probe, &["initial_position_m"]));
    let mut all = terrain.clone();
    all.extend(
        [
            source,
            target,
            anchor,
            primary,
            selected,
            route_waypoint,
            initial,
        ]
        .into_iter()
        .flatten(),
    );
    all.extend(composed_reference.iter().copied());
    if all.is_empty() {
        return "<p class=\"muted\">validated geometry unavailable</p>".to_owned();
    }
    let bounds = Bounds::from_points(&all);
    let width = 1080.0;
    let height = 460.0;
    let mut svg = format!(
        "<svg viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"{case} handoff setup geometry\"><defs><marker id=\"arrow-head\" markerWidth=\"8\" markerHeight=\"8\" refX=\"6\" refY=\"3\" orient=\"auto\"><path d=\"M0,0 L0,6 L7,3 z\" fill=\"#f8fafc\"/></marker></defs><rect width=\"100%\" height=\"100%\" fill=\"#0f172a\"/>",
        case = escape(&text(case, "case_id")),
    );
    svg.push_str(&polyline(&terrain, &bounds, "#94a3b8", 4.0, ""));
    svg.push_str(&polyline(&composed_reference, &bounds, "#38bdf8", 2.5, ""));
    if let (Some(source), Some(target)) = (source, target) {
        svg.push_str(&pad(source, &bounds, "source", "#64748b"));
        svg.push_str(&pad(target, &bounds, "target", "#64748b"));
    }
    if let Some(point) = initial {
        svg.push_str(&marker(point, &bounds, "#f87171", "initial state"));
    }
    if let Some(point) = anchor {
        svg.push_str(&marker(point, &bounds, "#facc15", "virtual anchor"));
    }
    if let Some(point) = primary {
        svg.push_str(&marker(point, &bounds, "#fb923c", "primary exact crossing"));
    }
    if let Some(point) = selected {
        svg.push_str(&marker(
            point,
            &bounds,
            "#4ade80",
            "selected V2 state/route",
        ));
        if let Some(waypoint) = route_waypoint {
            svg.push_str(&line(point, waypoint, &bounds, "#4ade80", 2.0, ""));
        }
        if let Some(target) = target {
            svg.push_str(&line(
                point,
                target,
                &bounds,
                "#4ade80",
                2.0,
                "selected V2 route",
            ));
        }
        if let Some(source) = source {
            svg.push_str(&line(
                source,
                point,
                &bounds,
                "#4ade80",
                2.0,
                "selected V2 route",
            ));
        }
        let tangent = path_opt(
            Some(selected_attempt),
            &["route", "waypoints", "0", "handoff_tangent_unit"],
        )
        .and_then(vector);
        let velocity =
            path_opt(Some(selected_attempt), &["selected_state", "velocity_mps"]).and_then(vector);
        let span = (bounds.max_x - bounds.min_x).max(bounds.max_y - bounds.min_y);
        let arrow_length = span * 0.12;
        if let Some(tangent) = tangent {
            svg.push_str(&arrow(
                point,
                tangent,
                &bounds,
                arrow_length,
                "#c084fc",
                "route tangent",
            ));
        }
        if let Some(velocity) = velocity {
            svg.push_str(&arrow(
                point,
                velocity,
                &bounds,
                arrow_length,
                "#f8fafc",
                "velocity direction",
            ));
        }
    }
    svg.push_str(
        "<text x=\"24\" y=\"30\" fill=\"#e2e8f0\" font-size=\"16\">terrain/ridge · certified composed analytical reference · gold virtual anchor · orange primary crossing · green selected route · white velocity · purple tangent</text></svg>",
    );
    svg
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl Bounds {
    fn from_points(points: &[(f64, f64)]) -> Self {
        let min_x = points
            .iter()
            .map(|point| point.0)
            .fold(f64::INFINITY, f64::min);
        let max_x = points
            .iter()
            .map(|point| point.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = points
            .iter()
            .map(|point| point.1)
            .fold(f64::INFINITY, f64::min);
        let max_y = points
            .iter()
            .map(|point| point.1)
            .fold(f64::NEG_INFINITY, f64::max);
        Self {
            min_x,
            max_x: max_x.max(min_x + 1.0),
            min_y,
            max_y: max_y.max(min_y + 1.0),
        }
    }
}

fn polyline(points: &[(f64, f64)], bounds: &Bounds, color: &str, width: f64, dash: &str) -> String {
    if points.len() < 2 {
        return String::new();
    }
    let coords = points
        .iter()
        .map(|point| {
            let (x, y) = scale(*point, bounds);
            format!("{x:.2},{y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let dash_attr = if dash.is_empty() {
        String::new()
    } else {
        format!(" stroke-dasharray=\"{dash}\"")
    };
    format!(
        "<polyline fill=\"none\" stroke=\"{color}\" stroke-width=\"{width}\"{dash_attr} points=\"{coords}\"/>"
    )
}

fn line(
    from: (f64, f64),
    to: (f64, f64),
    bounds: &Bounds,
    color: &str,
    width: f64,
    marker_end: &str,
) -> String {
    let (x1, y1) = scale(from, bounds);
    let (x2, y2) = scale(to, bounds);
    let marker = if marker_end.is_empty() {
        String::new()
    } else {
        " marker-end=\"url(#arrow-head)\"".to_owned()
    };
    format!(
        "<line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" stroke=\"{color}\" stroke-width=\"{width}\"{marker}><title>{}</title></line>",
        escape(marker_end)
    )
}

fn marker(point: (f64, f64), bounds: &Bounds, color: &str, label: &str) -> String {
    let (x, y) = scale(point, bounds);
    format!(
        "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"7\" fill=\"{color}\"><title>{}</title></circle>",
        escape(label)
    )
}

fn pad(point: (f64, f64), bounds: &Bounds, label: &str, color: &str) -> String {
    let (x, y) = scale(point, bounds);
    format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"18\" height=\"10\" rx=\"3\" fill=\"{color}\"><title>{label} pad</title></rect>",
        x - 9.0,
        y - 5.0,
        label = escape(label)
    )
}

fn arrow(
    point: (f64, f64),
    direction: (f64, f64),
    bounds: &Bounds,
    length: f64,
    color: &str,
    label: &str,
) -> String {
    let norm = (direction.0 * direction.0 + direction.1 * direction.1).sqrt();
    if norm <= 1.0e-12 {
        return String::new();
    }
    let to = (
        point.0 + direction.0 / norm * length,
        point.1 + direction.1 / norm * length,
    );
    line(point, to, bounds, color, 3.0, label)
}

fn scale(point: (f64, f64), bounds: &Bounds) -> (f64, f64) {
    const WIDTH: f64 = 1080.0;
    const HEIGHT: f64 = 460.0;
    const PAD: f64 = 48.0;
    let x = PAD + (point.0 - bounds.min_x) / (bounds.max_x - bounds.min_x) * (WIDTH - 2.0 * PAD);
    let y = HEIGHT
        - PAD
        - (point.1 - bounds.min_y) / (bounds.max_y - bounds.min_y) * (HEIGHT - 2.0 * PAD);
    (x, y)
}

const BALLISTIC_REFERENCE_SEGMENTS: u64 = 96;
const POWERED_REFERENCE_SEGMENTS: u64 = 128;

/// Reconstruct the certified composed analytical reference from the compact
/// candidate coefficients carried in the evaluator artifact.  This is kept
/// deliberately local to the report crate: rendering must not call planner
/// code or depend on runtime materialization of bridge samples.
fn composed_reference_points(candidate: Option<&Value>) -> Vec<(f64, f64)> {
    let Some(candidate) = candidate else {
        return Vec::new();
    };
    let source_leg = path_opt(Some(candidate), &["source_leg"]);
    let target_leg = path_opt(Some(candidate), &["target_leg"]);
    let bridge = path_opt(Some(candidate), &["intermediate_bridge"]);
    let entry = path_opt(Some(candidate), &["intermediate_entry_handoff"]);
    let exit = path_opt(Some(candidate), &["intermediate_exit_handoff"]);
    let Some(entry_step) = path_opt(entry, &["arc_step"]).and_then(Value::as_u64) else {
        return Vec::new();
    };
    let Some(exit_step) = path_opt(exit, &["arc_step"]).and_then(Value::as_u64) else {
        return Vec::new();
    };
    let Some(bridge_steps) = path_opt(bridge, &["steps"]).and_then(Value::as_u64) else {
        return Vec::new();
    };
    let entry_position = point(path_opt(entry, &["state", "position_m"]));
    let exit_position = point(path_opt(exit, &["state", "position_m"]));

    let source_points = sample_ballistic_segment(source_leg, 0, entry_step, None, entry_position);
    let bridge_points = sample_affine_bridge(bridge, bridge_steps, entry_position, exit_position);
    let target_end_step = path_opt(target_leg, &["steps"])
        .and_then(Value::as_u64)
        .unwrap_or(exit_step);
    let target_points =
        sample_ballistic_segment(target_leg, exit_step, target_end_step, exit_position, None);

    let mut points = Vec::with_capacity(
        source_points
            .len()
            .saturating_add(bridge_points.len())
            .saturating_add(target_points.len()),
    );
    append_points(&mut points, source_points);
    append_points(&mut points, bridge_points);
    append_points(&mut points, target_points);
    points
}

fn sample_ballistic_segment(
    arc: Option<&Value>,
    start_step: u64,
    end_step: u64,
    start_override: Option<(f64, f64)>,
    end_override: Option<(f64, f64)>,
) -> Vec<(f64, f64)> {
    let Some(arc) = arc else {
        return Vec::new();
    };
    let Some(arc_steps) = path_opt(Some(arc), &["steps"]).and_then(Value::as_u64) else {
        return Vec::new();
    };
    if start_step > end_step || end_step > arc_steps {
        return Vec::new();
    }
    let Some(start) = point(path_opt(Some(arc), &["start_m"])) else {
        return Vec::new();
    };
    let Some(departure_velocity) =
        path_opt(Some(arc), &["departure_velocity_mps"]).and_then(vector)
    else {
        return Vec::new();
    };
    let Some(dt_s) = path_opt(Some(arc), &["dt_s"]).and_then(Value::as_f64) else {
        return Vec::new();
    };
    let Some(gravity_mps2) = path_opt(Some(arc), &["gravity_mps2"]).and_then(Value::as_f64) else {
        return Vec::new();
    };
    let steps = sample_step_indices(start_step, end_step, BALLISTIC_REFERENCE_SEGMENTS);
    let mut points = steps
        .into_iter()
        .filter_map(|step| ballistic_position(start, departure_velocity, dt_s, gravity_mps2, step))
        .collect::<Vec<_>>();
    if let Some(point) = start_override
        && let Some(first) = points.first_mut()
    {
        *first = point;
    }
    if let Some(point) = end_override
        && let Some(last) = points.last_mut()
    {
        *last = point;
    }
    points
}

fn ballistic_position(
    start: (f64, f64),
    departure_velocity: (f64, f64),
    dt_s: f64,
    gravity_mps2: f64,
    step: u64,
) -> Option<(f64, f64)> {
    let k = step as f64;
    let gravity_steps = dt_s * dt_s * k * (k + 1.0) * 0.5;
    Some((
        start.0 + departure_velocity.0 * (k * dt_s),
        start.1 + departure_velocity.1 * (k * dt_s) - gravity_mps2 * gravity_steps,
    ))
}

fn sample_affine_bridge(
    bridge: Option<&Value>,
    bridge_steps: u64,
    start_override: Option<(f64, f64)>,
    end_override: Option<(f64, f64)>,
) -> Vec<(f64, f64)> {
    let Some(bridge) = bridge else {
        return Vec::new();
    };
    if bridge_steps == 0 {
        return Vec::new();
    }
    let Some(start_position) = point(path_opt(Some(bridge), &["start_state", "position_m"])) else {
        return Vec::new();
    };
    let Some(start_velocity) =
        path_opt(Some(bridge), &["start_state", "velocity_mps"]).and_then(vector)
    else {
        return Vec::new();
    };
    let Some(initial_acceleration) =
        path_opt(Some(bridge), &["initial_net_acceleration_mps2"]).and_then(vector)
    else {
        return Vec::new();
    };
    let Some(acceleration_step) =
        path_opt(Some(bridge), &["net_acceleration_step_mps2"]).and_then(vector)
    else {
        return Vec::new();
    };
    let Some(duration_s) = path_opt(Some(bridge), &["duration_s"]).and_then(Value::as_f64) else {
        return Vec::new();
    };
    let dt_s = duration_s / bridge_steps as f64;
    if !dt_s.is_finite() {
        return Vec::new();
    }
    let steps = sample_step_indices(0, bridge_steps, POWERED_REFERENCE_SEGMENTS);
    let mut points = steps
        .into_iter()
        .filter_map(|step| {
            affine_bridge_position(
                start_position,
                start_velocity,
                initial_acceleration,
                acceleration_step,
                dt_s,
                step,
            )
        })
        .collect::<Vec<_>>();
    if let Some(point) = start_override
        && let Some(first) = points.first_mut()
    {
        *first = point;
    }
    if let Some(point) = end_override
        && let Some(last) = points.last_mut()
    {
        *last = point;
    }
    points
}

fn affine_bridge_position(
    start_position: (f64, f64),
    start_velocity: (f64, f64),
    initial_acceleration: (f64, f64),
    acceleration_step: (f64, f64),
    dt_s: f64,
    applied_steps: u64,
) -> Option<(f64, f64)> {
    let k = applied_steps as f64;
    let first_sum = k * (k + 1.0) * 0.5;
    let second_sum = k * (k + 1.0) * (k - 1.0) / 6.0;
    Some((
        start_position.0
            + start_velocity.0 * (k * dt_s)
            + (initial_acceleration.0 * first_sum + acceleration_step.0 * second_sum)
                * (dt_s * dt_s),
        start_position.1
            + start_velocity.1 * (k * dt_s)
            + (initial_acceleration.1 * first_sum + acceleration_step.1 * second_sum)
                * (dt_s * dt_s),
    ))
}

fn sample_step_indices(start_step: u64, end_step: u64, max_segments: u64) -> Vec<u64> {
    if start_step > end_step {
        return Vec::new();
    }
    if start_step == end_step {
        return vec![start_step];
    }
    let span = end_step - start_step;
    let segments = max_segments.min(span).max(1);
    (0..=segments)
        .map(|index| {
            let offset = ((span as u128 * index as u128) / segments as u128) as u64;
            start_step + offset
        })
        .collect()
}

fn append_points(target: &mut Vec<(f64, f64)>, points: Vec<(f64, f64)>) {
    for point in points {
        if target.last().copied() != Some(point) {
            target.push(point);
        }
    }
}

fn points(value: Option<&Value>) -> Vec<(f64, f64)> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(|item| point(Some(item))).collect())
        .unwrap_or_default()
}

fn point(value: Option<&Value>) -> Option<(f64, f64)> {
    let value = value?;
    Some((value.get("x")?.as_f64()?, value.get("y")?.as_f64()?))
}

fn pad_point(value: Option<&Value>) -> Option<(f64, f64)> {
    let value = value?;
    Some((
        value.get("center_x_m")?.as_f64()?,
        value.get("surface_y_m")?.as_f64()?,
    ))
}

fn vector(value: &Value) -> Option<(f64, f64)> {
    point(Some(value))
}

fn format_point(value: Option<&Value>) -> String {
    point(value).map_or_else(
        || "unavailable".to_owned(),
        |(x, y)| format!("({x:.6}, {y:.6})"),
    )
}

fn path<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in keys {
        current = if let Ok(index) = key.parse::<usize>() {
            current.get(index)?
        } else {
            current.get(*key)?
        };
    }
    Some(current)
}

fn path_opt<'a>(value: Option<&'a Value>, keys: &[&str]) -> Option<&'a Value> {
    path(value?, keys)
}

fn text(value: &Value, key: &str) -> String {
    text_path(value, &[key])
}

fn text_path(value: &Value, keys: &[&str]) -> String {
    path(value, keys).map_or_else(
        || "unavailable".to_owned(),
        |value| match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        },
    )
}

fn number_text(value: &Value, key: &str) -> String {
    path(value, &[key]).map_or_else(|| "unavailable".to_owned(), |value| value.to_string())
}

fn number_path(value: &Value, keys: &[&str]) -> String {
    path(value, keys).map_or_else(
        || "unavailable".to_owned(),
        |value| {
            value
                .as_f64()
                .map_or_else(|| value.to_string(), |number| format!("{number:.6}"))
        },
    )
}

fn number_or_none(value: &Value, key: &str) -> String {
    path(value, &[key]).map_or_else(
        || "unavailable".to_owned(),
        |value| {
            if value.is_null() {
                "none".to_owned()
            } else {
                value.to_string()
            }
        },
    )
}

fn selected_kind(case: &Value) -> Option<String> {
    let selection = path(
        case,
        &[
            "runtime_v2",
            "derived_mesa",
            "OneWaypoint",
            "handoff_selection",
        ],
    )?;
    let attempts = selection.get("attempts")?.as_array()?;
    let index = selection.get("selected_attempt_index")?.as_u64()? as usize;
    let kind = attempts.get(index)?.get("selection_kind")?.as_str()?;
    Some(match kind {
        "primary_crossing" => "PrimaryCrossing".to_owned(),
        "intermediate_bridge_exit" => "IntermediateBridgeExit".to_owned(),
        other => other.to_owned(),
    })
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_json(value: &str) -> String {
    value
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

const CSS: &str = r#"
:root { color-scheme: dark; font-family: ui-sans-serif, system-ui, sans-serif; background:#0b1120; color:#e2e8f0; }
body { margin:0; } main { max-width:1240px; margin:0 auto; padding:30px 20px 60px; }
.hero, .panel { background:#111827; border:1px solid #293548; border-radius:14px; padding:24px; margin:0 0 22px; }
.eyebrow { color:#7dd3fc; font-size:.78rem; text-transform:uppercase; letter-spacing:.1em; } h1 { margin:.35rem 0 .7rem; } h2,h3,h4 { margin-top:0; }
.lede { max-width:92ch; color:#cbd5e1; line-height:1.55; } .muted { color:#94a3b8; line-height:1.5; }
.chips, .legend { display:flex; flex-wrap:wrap; gap:8px; margin-top:16px; } .chips span, .legend span { border:1px solid #334155; border-radius:999px; padding:5px 10px; color:#cbd5e1; font-size:.82rem; }
.panel-head, .attempt-head { display:flex; justify-content:space-between; align-items:flex-start; gap:12px; } .badge { border:1px solid #475569; border-radius:999px; padding:4px 9px; font-size:.72rem; letter-spacing:.06em; text-transform:uppercase; white-space:nowrap; }
.pass { color:#86efac; } .stop { color:#fca5a5; } .selected-badge { color:#86efac; border-color:#166534; } .badge.pass { border-color:#166534; } .badge.stop { border-color:#991b1b; }
.table-wrap { overflow:auto; } table { width:100%; border-collapse:collapse; } th,td { text-align:left; vertical-align:top; padding:10px; border-bottom:1px solid #293548; } th { color:#94a3b8; font-size:.78rem; text-transform:uppercase; letter-spacing:.05em; }
.facts, .metrics { display:grid; grid-template-columns:max-content 1fr; gap:8px 16px; } .facts { margin-top:18px; } .facts dt, .metrics dt { color:#94a3b8; } .facts dd, .metrics dd { margin:0; overflow-wrap:anywhere; }
code { color:#bae6fd; overflow-wrap:anywhere; } .callout { border-left:4px solid #38bdf8; background:#0f2537; padding:13px 16px; margin:14px 0; line-height:1.5; } .callout.causal { border-color:#fb923c; background:#2a1b13; } .callout.stable { border-color:#4ade80; background:#10271c; }
.legend { margin-bottom:12px; } .swatch { display:inline-block; width:12px; height:12px; border-radius:3px; margin-right:6px; vertical-align:-1px; background:#94a3b8; } .swatch.composed { background:#38bdf8; } .swatch.anchor { background:#facc15; } .swatch.primary { background:#fb923c; } .swatch.selected { background:#4ade80; } .swatch.velocity { background:#f8fafc; } .swatch.tangent { background:#c084fc; }
svg { width:100%; height:auto; border:1px solid #293548; border-radius:10px; margin:12px 0 20px; background:#0f172a; } .attempt-grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(310px,1fr)); gap:14px; } .attempt { border:1px solid #293548; border-radius:10px; padding:16px; background:#0f172a; } .attempt.selected { border-color:#16a34a; box-shadow:0 0 0 1px #166534 inset; } .attempt h4 { margin-bottom:0; } .attempt-id { font-size:.78rem; margin-bottom:0; }
.nonclaims { color:#cbd5e1; line-height:1.55; } .provenance { border-color:#475569; }
@media (max-width:700px) { .facts, .metrics { grid-template-columns:1fr; gap:3px; } .facts dd, .metrics dd { margin-bottom:7px; } }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn composed_reference_samples_all_segments_and_keeps_bridge_boundaries() {
        let candidate = json!({
            "source_leg": {
                "start_m": {"x": 0.0, "y": 0.0},
                "departure_velocity_mps": {"x": 1.0, "y": 3.0},
                "dt_s": 1.0,
                "gravity_mps2": 2.0,
                "steps": 4
            },
            "intermediate_entry_handoff": {
                "arc_step": 4,
                "state": {"position_m": {"x": 4.0, "y": -8.0}}
            },
            "intermediate_bridge": {
                "start_state": {
                    "position_m": {"x": 4.0, "y": -8.0},
                    "velocity_mps": {"x": 1.0, "y": -5.0}
                },
                "duration_s": 3.0,
                "steps": 3,
                "initial_net_acceleration_mps2": {"x": 0.0, "y": 0.0},
                "net_acceleration_step_mps2": {"x": 1.0, "y": 0.0}
            },
            "intermediate_exit_handoff": {
                "arc_step": 2,
                "state": {"position_m": {"x": 11.0, "y": -23.0}}
            },
            "target_leg": {
                "start_m": {"x": 3.0, "y": -7.0},
                "departure_velocity_mps": {"x": 4.0, "y": -5.0},
                "dt_s": 1.0,
                "gravity_mps2": 2.0,
                "steps": 5,
                "end_m": {"x": 23.0, "y": -62.0}
            }
        });

        let points = composed_reference_points(Some(&candidate));
        assert!(
            points.len() > 3,
            "reference must be sampled, not triangular"
        );
        assert_eq!(points.first(), Some(&(0.0, 0.0)));
        assert_eq!(points.last(), Some(&(23.0, -62.0)));
        assert!(
            points.contains(&(4.0, -8.0)),
            "bridge entry boundary missing"
        );
        assert!(
            points.contains(&(5.0, -13.0)),
            "bridge interior sample missing"
        );
        assert!(
            points.contains(&(11.0, -23.0)),
            "bridge exit boundary missing"
        );
    }

    #[test]
    fn development_report_contains_scope_causal_markers_and_attempt_metrics() {
        let root = std::env::temp_dir().join(format!(
            "pd-report-handoff-development-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("index.html");
        let data = json!({
            "schema_id":"conservative-ballistic-handoff-development-v1",
            "schema_version":1,
            "setup_id":"conservative-ballistic-handoff-development-v1",
            "input_manifest_identity":"manifest",
            "deterministic_repeat":true,
            "identity":"artifact",
            "scope":{"controller_run":false,"simulation_run":false,"physical_execution_run":false},
            "cases":[
                {"case_id":"ridge_progress_050_probe","historical_v1":{"status":"passed"},"identity":"case050","candidate_projection":{"identity":"candidate050"},"runtime_v2":{"identity":"runtime050","derived_mesa":{"OneWaypoint":{"handoff_selection":{"selected_attempt_index":0,"attempts":[{"selection_kind":"primary_crossing","applied_steps":1406,"target_leg_arc_step":null,"selected_state":{"position_m":{"x":1.0,"y":2.0},"velocity_mps":{"x":3.0,"y":4.0}},"route":{"waypoints":[{"position_m":{"x":1.0,"y":2.0},"handoff_tangent_unit":{"x":1.0,"y":0.0},"max_outbound_heading_error_rad":0.35,"min_outbound_progress_mps":0.0,"max_outbound_cross_speed_mps":12.5}]},"authority":{"handoff_speed_cap_mps":90.0},"handoff_kinematics":{"outbound_heading_error_rad":0.1,"outbound_progress_mps":3.0,"outbound_cross_speed_mps":2.0,"speed_mps":5.0},"handoff_assessment":{"contract_pass":true,"violations":[]},"identity":"attempt050"}]}}}}},
                {"case_id":"ridge_progress_068_probe","historical_v1":{"status":"failed"},"identity":"case068","candidate_projection":{"identity":"candidate068"},"runtime_v2":{"identity":"runtime068","derived_mesa":{"OneWaypoint":{"handoff_selection":{"selected_attempt_index":1,"crossing":{"virtual_anchor_m":{"x":0.0,"y":0.0}},"attempts":[{"selection_kind":"primary_crossing","applied_steps":1406,"target_leg_arc_step":null,"selected_state":{"position_m":{"x":1.0,"y":2.0},"velocity_mps":{"x":3.0,"y":4.0}},"route":{"waypoints":[{"position_m":{"x":1.0,"y":2.0},"handoff_tangent_unit":{"x":1.0,"y":0.0},"max_outbound_heading_error_rad":0.35,"min_outbound_progress_mps":0.0,"max_outbound_cross_speed_mps":12.5}]},"authority":{"handoff_speed_cap_mps":90.0},"handoff_kinematics":{"outbound_heading_error_rad":0.39,"outbound_progress_mps":3.0,"outbound_cross_speed_mps":2.0,"speed_mps":5.0},"handoff_assessment":{"contract_pass":false,"violations":["heading"]},"identity":"attempt068a"},{"selection_kind":"intermediate_bridge_exit","applied_steps":3960,"target_leg_arc_step":1190,"selected_state":{"position_m":{"x":2.0,"y":3.0},"velocity_mps":{"x":4.0,"y":5.0}},"route":{"waypoints":[{"position_m":{"x":2.0,"y":3.0},"handoff_tangent_unit":{"x":1.0,"y":0.0},"max_outbound_heading_error_rad":0.35,"min_outbound_progress_mps":0.0,"max_outbound_cross_speed_mps":12.5}]},"authority":{"handoff_speed_cap_mps":93.2},"handoff_kinematics":{"outbound_heading_error_rad":0.19,"outbound_progress_mps":4.0,"outbound_cross_speed_mps":3.0,"speed_mps":6.0},"handoff_assessment":{"contract_pass":true,"violations":[]},"identity":"attempt068b"}]}}}}}
            ]
        });
        write_conservative_ballistic_handoff_development_report(&path, &data).unwrap();
        let html = fs::read_to_string(&path).unwrap();
        for marker in [
            "development regression",
            "controller-free",
            "no simulation",
            "not held-out evidence",
            "Historical H2/H4 STOP remains unchanged",
            "PrimaryCrossing",
            "IntermediateBridgeExit",
            "Applied bridge step",
            "Heading error / limit",
            "velocity direction",
        ] {
            assert!(html.contains(marker), "missing report marker {marker}");
        }
        let _ = fs::remove_dir_all(root);
    }
}
