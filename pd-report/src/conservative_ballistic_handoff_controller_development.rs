//! Display-only F4 report. The evaluator supplies serialized evidence only.

use std::{fmt::Write as _, fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

const TITLE: &str = "Generic 068 Handoff Controller Development";

pub fn write_conservative_ballistic_handoff_controller_development_report(
    path: &Path,
    preview_path: &Path,
    data: &Value,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create F4 report directory {}", parent.display())
        })?;
    }
    if let Some(parent) = preview_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let svg = build_svg(data);
    fs::write(preview_path, &svg)
        .with_context(|| format!("failed to write F4 preview {}", preview_path.display()))?;
    fs::write(path, build_html(data, &svg)?)
        .with_context(|| format!("failed to write F4 report {}", path.display()))?;
    Ok(())
}

fn build_html(data: &Value, svg: &str) -> Result<String> {
    let mut lane_rows = String::new();
    for lane in lanes(data) {
        let _ = write!(
            lane_rows,
            "<tr><td><code>{}</code></td><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
            esc(&text(lane, "id")),
            esc(&text(lane, "class")),
            esc(&text(lane, "end_reason")),
            esc(&text(lane, "causal_reason")),
            esc(&waypoint_summary(lane.get("waypoint_contract"))),
        );
    }
    let mut gate_rows = String::new();
    for key in [
        "all_lane_launch_valid",
        "flat_target_landing",
        "mesa_direct_ridge_span_crash",
        "waypoint_exactly_one_captured_contract_pass",
        "waypoint_target_landing",
        "overall_green",
    ] {
        let check = path(data, &["gate", key]);
        let passes = check
            .and_then(|check| check.get("passes"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let _ = write!(
            gate_rows,
            "<tr><td><code>{}</code></td><td class=\"{}\">{}</td><td>{}</td></tr>",
            esc(&key.replace('_', " ")),
            if passes { "pass" } else { "stop" },
            if passes { "PASS" } else { "NOT MET" },
            esc(&check.map_or_else(String::new, |check| text(check, "reason"))),
        );
    }
    let overall = bool_path(data, &["gate", "overall_green", "passes"]);
    let serialized = serde_json::to_string(data)?;
    Ok(format!(
        r###"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title}</title><style>{css}</style></head><body><main>
<header><div class="eyebrow">powered descent lab · development-only controller run</div><h1>{title}</h1>
<p class="lede"><strong>F4 execution evidence:</strong> unchanged built-in controllers at 120 Hz physics / 60 Hz controller / 180 s. The three lanes use the exact generic runtime V2 route projection for exposed input <code>ridge_progress_068_probe</code>.</p>
<p class="warning"><strong>Non-claims:</strong> this is not held-out or physical-execution evidence. Dashed analytical paths are context only; the analytical bridge is <strong>not</strong> a controller command trace. A non-green result is retained as evidence, not a prompt to retune.</p>
<div class="chips"><span>schema {schema}</span><span>repeat {repeat}</span><span>artifact {identity}</span><span class="{overall_class}">overall F4 {overall}</span></div></header>
<section class="panel"><h2>F4 gate</h2><table><thead><tr><th>Gate</th><th>Result</th><th>Evidence / reason</th></tr></thead><tbody>{gate_rows}</tbody></table></section>
<section class="panel"><h2>Observed controller lanes</h2><table><thead><tr><th>Lane</th><th>Class</th><th>End</th><th>Causal observation</th><th>Waypoint handoff</th></tr></thead><tbody>{lane_rows}</tbody></table></section>
<section class="panel"><h2>Trajectory overlay</h2><p class="muted">Gray is derived mesa terrain and pads. Solid paths are simulator observations. Dashed paths are analytical composition context. Orange marks the rejected primary state; green marks selected bridge-exit/runtime waypoint. A red X is reconstructed crash contact; a green ring is actual capture when available.</p>{svg}</section>
<section class="panel"><h2>Selected runtime handoff</h2><dl class="facts"><dt>Primary rejected state</dt><dd>{primary}</dd><dt>Selected bridge-exit state</dt><dd>{selected}</dd><dt>Selected attempt</dt><dd><code>{index}</code> / <code>{attempt_identity}</code></dd><dt>Candidate / runtime identity</dt><dd><code>{candidate}</code> / <code>{runtime}</code></dd><dt>Input / manifest identity</dt><dd><code>{input}</code> / <code>{manifest}</code></dd><dt>Controller configuration</dt><dd>built-in <code>transfer_pdg</code> and <code>transfer_waypoint_pdg</code>; 120 Hz physics / 60 Hz controller / 180 s</dd></dl></section>
<section class="panel"><h2>Scope and provenance</h2><p>The artifact binds source input, recomputed candidate/runtime selection, exact scenarios/controllers/runs, derived gates, and a semantic repeat that excludes wall, CPU, and controller-compute timing.</p><ul>{nonclaims}</ul></section>
<script type="application/json" id="conservative-ballistic-handoff-controller-development-data">{data}</script></main></body></html>"###,
        title = esc(TITLE),
        css = CSS,
        schema = esc(&text(data, "schema_id")),
        repeat = esc(&text(data, "deterministic_repeat")),
        identity = esc(&text(data, "identity")),
        overall_class = if overall { "pass" } else { "stop" },
        overall = if overall { "GREEN" } else { "NOT GREEN" },
        gate_rows = gate_rows,
        lane_rows = lane_rows,
        svg = svg,
        primary = esc(&attempt_summary(primary_attempt(data))),
        selected = esc(&attempt_summary(selected_attempt(data))),
        index = esc(&text(data, "selected_attempt_index")),
        attempt_identity = esc(&text(data, "selected_attempt_identity")),
        candidate = esc(&text_path(data, &["candidate_projection", "identity"])),
        runtime = esc(&text_path(data, &["runtime_v2", "identity"])),
        input = esc(&text(data, "input_identity")),
        manifest = esc(&text(data, "input_manifest_identity")),
        nonclaims = nonclaims(data),
        data = esc_json(&serialized),
    ))
}

fn build_svg(data: &Value) -> String {
    let (min_x, max_x, min_y, max_y) = bounds(data);
    let sx = |x: f64| 54.0 + (x - min_x) / (max_x - min_x).max(1.0) * 1012.0;
    let sy = |y: f64| 575.0 - (y - min_y) / (max_y - min_y).max(1.0) * 500.0;
    let mut svg = String::from(
        r###"<svg xmlns="http://www.w3.org/2000/svg" width="1120" height="630" viewBox="0 0 1120 630" role="img" aria-label="F4 generic 068 controller trajectories"><rect width="100%" height="100%" fill="#111722"/><text x="54" y="31" fill="#eff5ff" font-family="sans-serif" font-size="20">F4 generic 068 controller execution</text><text x="54" y="53" fill="#aab7c7" font-family="sans-serif" font-size="12">solid simulation samples · dashed analytical context only · bridge is not a command trace</text><g aria-label="solid lane legend" font-family="sans-serif" font-size="11"><line x1="54" y1="74" x2="78" y2="74" stroke="#5ca9ff" stroke-width="3"/><text x="84" y="78" fill="#b7d9ff">flat direct</text><line x1="164" y1="74" x2="188" y2="74" stroke="#ef6868" stroke-width="3"/><text x="194" y="78" fill="#ffb2b2">mesa direct</text><line x1="282" y1="74" x2="306" y2="74" stroke="#67d99b" stroke-width="3"/><text x="312" y="78" fill="#b7f5d2">mesa waypoint</text></g>"###,
    );
    draw_terrain_and_pads(&mut svg, data, &sx, &sy);
    draw_analytical(&mut svg, data, &sx, &sy);
    draw_lanes(&mut svg, data, &sx, &sy);
    draw_markers(&mut svg, data, &sx, &sy);
    svg.push_str("</svg>");
    svg
}

fn draw_terrain_and_pads(
    svg: &mut String,
    data: &Value,
    sx: &impl Fn(f64) -> f64,
    sy: &impl Fn(f64) -> f64,
) {
    let Some(mesa) = lane(data, "mesa-direct") else {
        return;
    };
    if let Some(points) =
        path(mesa, &["scenario", "world", "terrain", "points_m"]).and_then(Value::as_array)
    {
        let _ = write!(
            svg,
            "<polyline points=\"{}\" fill=\"none\" stroke=\"#919dad\" stroke-width=\"3\"/>",
            points_to_svg(points, sx, sy)
        );
    }
    if let Some(pads) = path(mesa, &["scenario", "world", "landing_pads"]).and_then(Value::as_array)
    {
        for pad in pads {
            let x = number(pad, "center_x_m");
            let half = number(pad, "width_m") * 0.5;
            let y = number(pad, "surface_y_m");
            let _ = write!(
                svg,
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#f2d46c\" stroke-width=\"6\" stroke-linecap=\"round\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#f2d46c\" font-family=\"sans-serif\" font-size=\"11\">{}</text>",
                sx(x - half),
                sy(y),
                sx(x + half),
                sy(y),
                sx(x - half),
                sy(y) - 9.0,
                esc(&text(pad, "id"))
            );
        }
    }
}

fn draw_analytical(
    svg: &mut String,
    data: &Value,
    sx: &impl Fn(f64) -> f64,
    sy: &impl Fn(f64) -> f64,
) {
    for (key, color) in [
        ("nominal_direct", "#f4be48"),
        ("waypoint_source_bridge", "#65aaff"),
        ("waypoint_source_leg", "#76d9f1"),
        ("waypoint_intermediate_bridge", "#a3b5ff"),
        ("waypoint_target_leg", "#e39dff"),
        ("waypoint_terminal_bridge", "#f09aff"),
    ] {
        let points = path(data, &["analytical_overlay", key, "points_m"])
            .and_then(Value::as_array)
            .map(|points| points_to_svg(points, sx, sy))
            .unwrap_or_default();
        if !points.is_empty() {
            let _ = write!(
                svg,
                "<polyline points=\"{points}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2.2\" stroke-dasharray=\"8 6\" opacity=\"0.8\"/>"
            );
        }
    }
}

fn draw_lanes(svg: &mut String, data: &Value, sx: &impl Fn(f64) -> f64, sy: &impl Fn(f64) -> f64) {
    for (id, color) in [
        ("flat-direct", "#5ca9ff"),
        ("mesa-direct", "#ef6868"),
        ("mesa-waypoint", "#67d99b"),
    ] {
        let Some(lane) = lane(data, id) else { continue };
        let samples = path(lane, &["run", "run", "samples"])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let points = samples
            .iter()
            .filter_map(|sample| path(sample, &["observation", "position_m"]))
            .filter_map(point)
            .map(|(x, y)| format!("{:.2},{:.2}", sx(x), sy(y)))
            .collect::<Vec<_>>()
            .join(" ");
        if !points.is_empty() {
            let _ = write!(
                svg,
                "<polyline points=\"{points}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2.7\"/>"
            );
        }
        if let Some(last) = samples
            .last()
            .and_then(|sample| path(sample, &["observation", "position_m"]))
            .and_then(point)
        {
            let _ = write!(
                svg,
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"7\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2\"/>",
                sx(last.0),
                sy(last.1),
            );
        }
    }
}

fn draw_markers(
    svg: &mut String,
    data: &Value,
    sx: &impl Fn(f64) -> f64,
    sy: &impl Fn(f64) -> f64,
) {
    if let Some(point) = primary_attempt(data).and_then(attempt_position) {
        let _ = write!(
            svg,
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"7\" fill=\"#ff9f43\" stroke=\"#fff\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#ffb66c\" font-family=\"sans-serif\" font-size=\"11\">primary rejected</text>",
            sx(point.0),
            sy(point.1),
            sx(point.0) + 10.0,
            sy(point.1) - 9.0
        );
    }
    if let Some(point) = selected_attempt(data).and_then(attempt_position) {
        let _ = write!(
            svg,
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"8\" fill=\"#49d88b\" stroke=\"#fff\"/><text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\" fill=\"#72e8a4\" font-family=\"sans-serif\" font-size=\"11\">selected runtime waypoint</text>",
            sx(point.0),
            sy(point.1),
            sx(point.0) - 10.0,
            sy(point.1) - 14.0
        );
    }
    if let Some(point) = lane(data, "mesa-waypoint")
        .and_then(|lane| path(lane, &["waypoint_contract", "capture_position_m"]))
        .and_then(point)
    {
        let _ = write!(
            svg,
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"12\" fill=\"none\" stroke=\"#97ffd0\" stroke-width=\"2\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#97ffd0\" font-family=\"sans-serif\" font-size=\"11\">actual capture</text>",
            sx(point.0),
            sy(point.1),
            sx(point.0) + 15.0,
            sy(point.1) + 18.0
        );
    }
    if let Some(point) = lane(data, "mesa-direct")
        .and_then(|lane| path(lane, &["terrain_contact", "contact_point_m"]))
        .and_then(point)
    {
        let _ = write!(
            svg,
            "<path d=\"M {:.2} {:.2} l 12 12 m 0 -12 l -12 12\" stroke=\"#ff6262\" stroke-width=\"3\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#ff7f7f\" font-family=\"sans-serif\" font-size=\"11\">reconstructed crash contact</text>",
            sx(point.0) - 6.0,
            sy(point.1) - 6.0,
            sx(point.0) + 10.0,
            sy(point.1) - 11.0
        );
    }
}

fn bounds(data: &Value) -> (f64, f64, f64, f64) {
    let mut points = Vec::new();
    for lane in lanes(data) {
        if let Some(terrain) =
            path(lane, &["scenario", "world", "terrain", "points_m"]).and_then(Value::as_array)
        {
            points.extend(terrain.iter().filter_map(point));
        }
        if let Some(samples) = path(lane, &["run", "run", "samples"]).and_then(Value::as_array) {
            points.extend(
                samples
                    .iter()
                    .filter_map(|sample| path(sample, &["observation", "position_m"]))
                    .filter_map(point),
            );
        }
    }
    for key in [
        "nominal_direct",
        "waypoint_source_leg",
        "waypoint_target_leg",
    ] {
        if let Some(path) =
            path(data, &["analytical_overlay", key, "points_m"]).and_then(Value::as_array)
        {
            points.extend(path.iter().filter_map(point));
        }
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for (x, y) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    if !min_x.is_finite() {
        return (0.0, 1.0, 0.0, 1.0);
    }
    let pad_x = ((max_x - min_x) * 0.05).max(50.0);
    let pad_y = ((max_y - min_y) * 0.08).max(50.0);
    (min_x - pad_x, max_x + pad_x, min_y - pad_y, max_y + pad_y)
}

fn lanes(data: &Value) -> Vec<&Value> {
    data.get("lanes")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |lanes| lanes.iter().collect())
}
fn lane<'a>(data: &'a Value, id: &str) -> Option<&'a Value> {
    lanes(data).into_iter().find(|lane| text(lane, "id") == id)
}
fn primary_attempt(data: &Value) -> Option<&Value> {
    handoff_attempts(data).and_then(|attempts| attempts.first())
}
fn selected_attempt(data: &Value) -> Option<&Value> {
    handoff_attempts(data)?.get(data.get("selected_attempt_index")?.as_u64()? as usize)
}
fn handoff_attempts(data: &Value) -> Option<&Vec<Value>> {
    path(
        data,
        &[
            "runtime_v2",
            "derived_mesa",
            "OneWaypoint",
            "handoff_selection",
            "attempts",
        ],
    )
    .and_then(Value::as_array)
}
fn attempt_position(attempt: &Value) -> Option<(f64, f64)> {
    path(attempt, &["selected_state", "position_m"]).and_then(point)
}
fn points_to_svg(points: &[Value], sx: &impl Fn(f64) -> f64, sy: &impl Fn(f64) -> f64) -> String {
    points
        .iter()
        .filter_map(point)
        .map(|(x, y)| format!("{:.2},{:.2}", sx(x), sy(y)))
        .collect::<Vec<_>>()
        .join(" ")
}
fn attempt_summary(attempt: Option<&Value>) -> String {
    let Some(attempt) = attempt else {
        return "unavailable".to_owned();
    };
    format!(
        "{} at {}; step {}; contract_pass={}",
        text(attempt, "selection_kind"),
        attempt_position(attempt).map_or_else(
            || "unavailable".to_owned(),
            |(x, y)| format!("({x:.3}, {y:.3})"),
        ),
        text(attempt, "applied_steps"),
        text_path(attempt, &["handoff_assessment", "contract_pass"])
    )
}
fn waypoint_summary(waypoint: Option<&Value>) -> String {
    waypoint.filter(|waypoint| !waypoint.is_null()).map_or_else(
        || "none".to_owned(),
        |waypoint| {
            format!(
                "markers={}; pass={}; capture={}",
                text(waypoint, "marker_count"),
                text(waypoint, "contract_pass"),
                point_text(waypoint.get("capture_position_m"))
            )
        },
    )
}
fn nonclaims(data: &Value) -> String {
    path(data, &["scope", "non_claims"])
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| format!("<li>{}</li>", esc(item)))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}
fn path<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().try_fold(value, |value, key| value.get(*key))
}
fn text(value: &Value, key: &str) -> String {
    value.get(key).map_or_else(String::new, value_text)
}
fn text_path(value: &Value, keys: &[&str]) -> String {
    path(value, keys).map_or_else(String::new, value_text)
}
fn value_text(value: &Value) -> String {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_bool().map(|value| value.to_string()))
        .or_else(|| value.as_u64().map(|value| value.to_string()))
        .or_else(|| value.as_f64().map(|value| format!("{value:.6}")))
        .unwrap_or_default()
}
fn number(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}
fn point(value: &Value) -> Option<(f64, f64)> {
    Some((value.get("x")?.as_f64()?, value.get("y")?.as_f64()?))
}
fn point_text(value: Option<&Value>) -> String {
    value.and_then(point).map_or_else(
        || "unavailable".to_owned(),
        |(x, y)| format!("({x:.3}, {y:.3})"),
    )
}
fn bool_path(value: &Value, keys: &[&str]) -> bool {
    path(value, keys).and_then(Value::as_bool).unwrap_or(false)
}
fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn esc_json(value: &str) -> String {
    value
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
}

const CSS: &str = r###"
:root{color-scheme:dark;font-family:ui-sans-serif,system-ui,sans-serif;background:#0d131c;color:#edf3fb}body{margin:0}main{max-width:1240px;margin:auto;padding:36px 24px 64px}header{margin-bottom:22px}h1{margin:5px 0 10px;font-size:30px}h2{margin:0 0 12px;font-size:20px}.eyebrow{color:#8eb9ff;letter-spacing:.09em;font-size:12px;text-transform:uppercase}.lede{max-width:1040px;color:#d2dce9;line-height:1.55}.warning{border-left:4px solid #f0b45c;background:#2a2117;padding:12px 14px;color:#ffe4b6;line-height:1.5}.chips{display:flex;flex-wrap:wrap;gap:8px;margin-top:14px}.chips span{background:#1d2938;border:1px solid #34465c;border-radius:999px;padding:5px 9px;font-size:12px}.panel{background:#151e2b;border:1px solid #293a50;border-radius:10px;padding:18px;margin-top:18px;overflow:auto}table{width:100%;border-collapse:collapse;font-size:13px}th,td{text-align:left;vertical-align:top;padding:9px;border-bottom:1px solid #293a50}th{color:#a8bbd2}code{color:#b8d9ff;overflow-wrap:anywhere}.pass{color:#72e3a2!important}.stop{color:#ff9d9d!important}.muted{color:#aebdcd;line-height:1.5}.facts{display:grid;grid-template-columns:minmax(190px,auto) 1fr;gap:8px 14px;margin:0}.facts dt{color:#a8bbd2}.facts dd{margin:0}svg{width:100%;min-width:900px;height:auto;display:block;border-radius:7px}li{margin:6px 0;color:#d2dce9}
"###;

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn renderer_contains_required_sections_embedded_data_and_preview() {
        let data = serde_json::json!({
            "schema_id":"conservative-ballistic-handoff-controller-development-v1","deterministic_repeat":true,"identity":"fixture","input_identity":"input","input_manifest_identity":"manifest","selected_attempt_index":1,"selected_attempt_identity":"attempt","candidate_projection":{"identity":"candidate"},"runtime_v2":{"identity":"runtime","derived_mesa":{"handoff_selection":{"attempts":[{"selection_kind":"primary_crossing","selected_state":{"x":1.0,"y":2.0},"applied_steps":1,"handoff_assessment":{"contract_pass":false}},{"selection_kind":"intermediate_bridge_exit","selected_state":{"x":3.0,"y":4.0},"applied_steps":2,"handoff_assessment":{"contract_pass":true}}]}}},"scope":{"non_claims":["not held out"]},"gate":{"overall_green":{"passes":false,"reason":"recorded"},"all_lane_launch_valid":{"passes":true,"reason":"ok"},"flat_target_landing":{"passes":true,"reason":"ok"},"mesa_direct_ridge_span_crash":{"passes":true,"reason":"ok"},"waypoint_exactly_one_captured_contract_pass":{"passes":false,"reason":"not captured"},"waypoint_target_landing":{"passes":false,"reason":"not landed"}},"analytical_overlay":{"nominal_direct":{"points_m":[{"x":0.0,"y":0.0},{"x":4.0,"y":5.0}]},"waypoint_source_bridge":{"points_m":[]},"waypoint_source_leg":{"points_m":[]},"waypoint_intermediate_bridge":{"points_m":[]},"waypoint_target_leg":{"points_m":[]},"waypoint_terminal_bridge":{"points_m":[]}},"lanes":[{"id":"flat-direct","class":"target_landing","end_reason":"touchdown_on_target","causal_reason":"landed","waypoint_contract":null,"scenario":{"world":{"terrain":{"points_m":[{"x":0.0,"y":0.0},{"x":10.0,"y":0.0}]},"landing_pads":[]}},"run":{"run":{"samples":[]}}},{"id":"mesa-direct","class":"terrain_crash","end_reason":"crash","causal_reason":"ridge","waypoint_contract":null,"terrain_contact":{"contact_point_m":{"x":3.0,"y":1.0}},"scenario":{"world":{"terrain":{"points_m":[{"x":0.0,"y":0.0},{"x":10.0,"y":0.0}]},"landing_pads":[]}},"run":{"run":{"samples":[]}}},{"id":"mesa-waypoint","class":"inconclusive","end_reason":"crash","causal_reason":"observed","waypoint_contract":{"marker_count":0,"contract_pass":false,"capture_position_m":null},"scenario":{"world":{"terrain":{"points_m":[{"x":0.0,"y":0.0},{"x":10.0,"y":0.0}]},"landing_pads":[]}},"run":{"run":{"samples":[]}}}]});
        // The production enum is externally tagged and each selected state
        // carries position beneath `selected_state.position_m`.
        let mut data = data;
        data["runtime_v2"]["derived_mesa"] = serde_json::json!({
            "OneWaypoint": {
                "handoff_selection": {
                    "attempts": [
                        {
                            "selection_kind": "primary_crossing",
                            "selected_state": {"position_m": {"x": 1.0, "y": 2.0}},
                            "applied_steps": 1,
                            "handoff_assessment": {"contract_pass": false}
                        },
                        {
                            "selection_kind": "intermediate_bridge_exit",
                            "selected_state": {"position_m": {"x": 3.0, "y": 4.0}},
                            "applied_steps": 2,
                            "handoff_assessment": {"contract_pass": true}
                        }
                    ]
                }
            }
        });
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pd-report-f4-{nonce}"));
        let report = root.join("index.html");
        let preview = root.join("preview.svg");
        write_conservative_ballistic_handoff_controller_development_report(
            &report, &preview, &data,
        )
        .unwrap();
        let html = fs::read_to_string(&report).unwrap();
        assert!(html.contains("F4 gate"));
        assert!(html.contains("Trajectory overlay"));
        assert!(html.contains("Selected runtime handoff"));
        assert!(html.contains("conservative-ballistic-handoff-controller-development-data"));
        assert!(html.contains("primary_crossing at (1.000, 2.000)"));
        assert!(html.contains("intermediate_bridge_exit at (3.000, 4.000)"));
        assert!(html.contains("<td>none</td>"));
        let preview = fs::read_to_string(&preview).unwrap();
        assert!(preview.contains("solid lane legend"));
        assert!(preview.contains("flat direct"));
        assert!(preview.contains("mesa direct"));
        assert!(preview.contains("mesa waypoint"));
        assert!(preview.contains("primary rejected"));
        assert!(preview.contains("selected runtime waypoint"));
        let _ = fs::remove_dir_all(root);
    }
}
