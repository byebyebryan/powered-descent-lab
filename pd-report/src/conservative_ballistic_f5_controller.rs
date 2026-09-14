//! Display-only F5 controller reveal report.
//!
//! The evaluator supplies serialized artifact evidence. This module imports no
//! planner, controller, scenario, or simulator API and cannot change a gate.

use std::{fmt::Write as _, fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

const TITLE: &str = "F5 Held-out Ridge Controller Reveal";

pub fn write_conservative_ballistic_f5_controller_report(
    path: &Path,
    preview_path: &Path,
    data: &Value,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create F5 controller report directory {}",
                parent.display()
            )
        })?;
    }
    if let Some(parent) = preview_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let svg = build_svg(data);
    fs::write(preview_path, &svg).with_context(|| {
        format!(
            "failed to write F5 controller preview {}",
            preview_path.display()
        )
    })?;
    fs::write(path, build_html(data, &svg)?)
        .with_context(|| format!("failed to write F5 controller report {}", path.display()))?;
    Ok(())
}

fn build_html(data: &Value, svg: &str) -> Result<String> {
    let case = first_case(data);
    let mut lane_rows = String::new();
    for lane in lanes(data) {
        let _ = write!(
            lane_rows,
            "<tr><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
            esc(&text(lane, "id")),
            esc(&text(lane, "controller_id")),
            esc(&text(lane, "end_reason")),
            esc(&text(lane, "causal_reason")),
            esc(&waypoint_summary(lane.get("waypoint_contract"))),
        );
    }
    let mut gate_rows = String::new();
    for key in [
        "all_lane_launch_valid",
        "flat_target_landing",
        "mesa_direct_non_target_terrain_contact_before_target_touchdown",
        "waypoint_exactly_one_captured_contract_pass",
        "waypoint_target_landing",
        "controller_predictions_matched",
    ] {
        let check = path(case, &["gate", key]);
        let passes = check
            .and_then(|value| value.get("passes"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let _ = write!(
            gate_rows,
            "<tr><td><code>{}</code></td><td class=\"{}\">{}</td><td>{}</td></tr>",
            esc(&key.replace('_', " ")),
            if passes { "pass" } else { "stop" },
            if passes { "PASS" } else { "NOT MET" },
            esc(&check.map_or_else(String::new, |value| text(value, "reason"))),
        );
    }
    let status = text(data, "evidence_status");
    let status_class = if status == "controller_predictions_matched" {
        "pass"
    } else {
        "stop"
    };
    let serialized = serde_json::to_string(data)?;
    Ok(format!(
        r###"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{title}</title><style>{css}</style></head><body><main>
<header><div class="eyebrow">powered descent lab · F5 held-out controller/simulator evidence</div><h1>{title}</h1>
<p class="lede">The three lanes below use the exact F5c-selected runtime-V2 route for the analytically eligible case only. The analytical bridge is visual context, not a controller command trace.</p>
<p class="warning"><strong>Scope:</strong> the stopped case is listed as analytically ineligible and has no controller lane or scenario data. A failed launch is <code>invalid_setup</code>; otherwise unmet frozen controller expectations are retained as prediction-mismatch evidence, never as tuning input.</p>
<div class="chips"><span>F5c result {analytical}</span><span>repeat {repeat}</span><span class="{status_class}">controller status {status}</span><span>artifact {identity}</span></div></header>
<section class="panel"><h2>Eligibility and provenance</h2><dl class="facts"><dt>Eligible controller cases</dt><dd>{eligible}</dd><dt>Analytically ineligible / not run</dt><dd>{ineligible}</dd><dt>Input seal</dt><dd><code>{input}</code></dd><dt>Prediction seal</dt><dd><code>{prediction}</code></dd><dt>Selected runtime attempt</dt><dd><code>{attempt}</code> / <code>{selection_kind}</code> / <code>{selection}</code></dd><dt>Selected runtime waypoint</dt><dd>{waypoint}</dd></dl></section>
<section class="panel"><h2>Frozen controller gate</h2><table><thead><tr><th>Gate</th><th>Result</th><th>Evidence / reason</th></tr></thead><tbody>{gate_rows}</tbody></table></section>
<section class="panel"><h2>Observed controller lanes</h2><table><thead><tr><th>Lane</th><th>Controller</th><th>End</th><th>Causal observation</th><th>Waypoint handoff</th></tr></thead><tbody>{lane_rows}</tbody></table></section>
<section class="panel"><h2>Trajectory overlay</h2><p class="muted">Gray is the derived mesa and pads. Solid paths are simulator samples. Dashed paths are analytical context. Gold marks the exact selected runtime waypoint; green ring marks a captured waypoint; red X marks reconstructed terrain contact.</p>{svg}</section>
<section class="panel"><h2>Evidence boundary</h2><p>The artifact binds sealed source identities, committed F5c identity, recomputed candidate/runtime identity, selected runtime attempt and route, exact scenarios/controllers, semantic lane outcomes, gates, and a repeat comparison that excludes timing metadata. It does not claim physical execution, production readiness, or a reason to retune.</p></section>
<script type="application/json" id="conservative-ballistic-f5-controller-data">{data}</script></main></body></html>"###,
        title = esc(TITLE),
        css = CSS,
        analytical = esc(&text(data, "analytical_result_identity")),
        repeat = esc(&text(data, "deterministic_repeat")),
        status_class = status_class,
        status = esc(&status),
        identity = esc(&text(data, "identity")),
        eligible = esc(&array_text(data, "eligible_case_ids")),
        ineligible = esc(&ineligible_text(data)),
        input = esc(&text(data, "input_manifest_identity")),
        prediction = esc(&text(data, "prediction_manifest_identity")),
        attempt = esc(&text(case, "selected_attempt_index")),
        selection_kind = esc(&text_path(case, &["selected_attempt", "selection_kind"])),
        selection = esc(&selected_handoff_selection(case)),
        waypoint = esc(&point_text(path(
            case,
            &["selected_attempt", "selected_state", "position_m"]
        ))),
        gate_rows = gate_rows,
        lane_rows = lane_rows,
        svg = svg,
        data = esc_json(&serialized),
    ))
}

fn build_svg(data: &Value) -> String {
    let case = first_case(data);
    let mut points = Vec::new();
    for lane in lanes(data) {
        points.extend(
            path(lane, &["scenario", "world", "terrain", "points_m"])
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(point),
        );
        points.extend(
            path(lane, &["run", "run", "samples"])
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|sample| path(sample, &["observation", "position_m"]))
                .filter_map(point),
        );
    }
    for key in [
        "nominal_direct",
        "waypoint_source_leg",
        "waypoint_target_leg",
    ] {
        points.extend(
            path(case, &["analytical_overlay", key, "points_m"])
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(point),
        );
    }
    let (min_x, max_x, min_y, max_y) = bounds(&points);
    let sx = |x: f64| 54.0 + (x - min_x) / (max_x - min_x).max(1.0) * 1012.0;
    let sy = |y: f64| 575.0 - (y - min_y) / (max_y - min_y).max(1.0) * 500.0;
    let mut svg = String::from(
        r###"<svg xmlns="http://www.w3.org/2000/svg" width="1120" height="630" viewBox="0 0 1120 630" role="img" aria-label="F5 held-out controller trajectories"><rect width="100%" height="100%" fill="#111722"/><text x="54" y="31" fill="#eff5ff" font-family="sans-serif" font-size="20">F5 held-out controller evidence</text><text x="54" y="53" fill="#aab7c7" font-family="sans-serif" font-size="12">solid simulation samples · dashed analytical context only · stopped cases not simulated</text><g aria-label="lane legend" font-family="sans-serif" font-size="11"><line x1="54" y1="74" x2="78" y2="74" stroke="#5ca9ff" stroke-width="3"/><text x="84" y="78" fill="#b7d9ff">flat direct</text><line x1="164" y1="74" x2="188" y2="74" stroke="#ef6868" stroke-width="3"/><text x="194" y="78" fill="#ffb2b2">mesa direct</text><line x1="282" y1="74" x2="306" y2="74" stroke="#67d99b" stroke-width="3"/><text x="312" y="78" fill="#b7f5d2">mesa waypoint</text></g>"###,
    );
    draw_terrain_and_pads(&mut svg, data, &sx, &sy);
    draw_analytical(&mut svg, case, &sx, &sy);
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
    case: &Value,
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
        let points = path(case, &["analytical_overlay", key, "points_m"])
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
    }
}

fn draw_markers(
    svg: &mut String,
    data: &Value,
    sx: &impl Fn(f64) -> f64,
    sy: &impl Fn(f64) -> f64,
) {
    let case = first_case(data);
    if let Some(point) =
        path(case, &["selected_attempt", "selected_state", "position_m"]).and_then(point)
    {
        let _ = write!(
            svg,
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"8\" fill=\"#f6c453\" stroke=\"#fff\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#ffdb82\" font-family=\"sans-serif\" font-size=\"11\">selected runtime waypoint</text>",
            sx(point.0),
            sy(point.1),
            sx(point.0) + 10.0,
            sy(point.1) - 12.0
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
            "<path d=\"M {:.2} {:.2} l 12 12 m 0 -12 l -12 12\" stroke=\"#ff6262\" stroke-width=\"3\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#ff7f7f\" font-family=\"sans-serif\" font-size=\"11\">reconstructed terrain contact</text>",
            sx(point.0) - 6.0,
            sy(point.1) - 6.0,
            sx(point.0) + 10.0,
            sy(point.1) - 11.0
        );
    }
}

fn first_case(data: &Value) -> &Value {
    data.get("cases")
        .and_then(Value::as_array)
        .and_then(|cases| cases.first())
        .unwrap_or(&Value::Null)
}
fn lanes(data: &Value) -> Vec<&Value> {
    first_case(data)
        .get("lanes")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |lanes| lanes.iter().collect())
}
fn lane<'a>(data: &'a Value, id: &str) -> Option<&'a Value> {
    lanes(data).into_iter().find(|lane| text(lane, "id") == id)
}
fn path<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().try_fold(value, |value, key| value.get(*key))
}
fn point(value: &Value) -> Option<(f64, f64)> {
    Some((value.get("x")?.as_f64()?, value.get("y")?.as_f64()?))
}
fn number(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}
fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .map(|value| match value {
            Value::String(value) => value.clone(),
            other => other.to_string(),
        })
        .unwrap_or_else(|| "unavailable".to_owned())
}
fn point_text(value: Option<&Value>) -> String {
    value.and_then(point).map_or_else(
        || "unavailable".to_owned(),
        |(x, y)| format!("({x:.6}, {y:.6})"),
    )
}
fn array_text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .map(|value| value.as_str().unwrap_or("unavailable"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "unavailable".to_owned())
}
fn ineligible_text(value: &Value) -> String {
    value
        .get("analytically_ineligible_cases")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .map(|case| format!("{} ({})", text(case, "case_id"), text(case, "reason")))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "unavailable".to_owned())
}
fn selected_handoff_selection(case: &Value) -> String {
    path(
        case,
        &[
            "runtime_v2",
            "derived_mesa",
            "OneWaypoint",
            "handoff_selection",
            "identity",
        ],
    )
    .and_then(Value::as_str)
    .unwrap_or("unavailable")
    .to_owned()
}
fn text_path(value: &Value, keys: &[&str]) -> String {
    path(value, keys)
        .map(|value| match value {
            Value::String(value) => value.clone(),
            other => other.to_string(),
        })
        .unwrap_or_else(|| "unavailable".to_owned())
}
fn waypoint_summary(contract: Option<&Value>) -> String {
    contract.map_or_else(
        || "not applicable".to_owned(),
        |contract| {
            format!(
                "markers={}; pass={}; reason={}",
                text(contract, "marker_count"),
                text(contract, "contract_pass"),
                text(contract, "resolution_reason")
            )
        },
    )
}
fn bounds(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    if points.is_empty() {
        return (0.0, 1.0, 0.0, 1.0);
    }
    let min_x = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let max_x = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let min_y = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let max_y = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let pad_x = ((max_x - min_x) * 0.05).max(50.0);
    let pad_y = ((max_y - min_y) * 0.08).max(50.0);
    (min_x - pad_x, max_x + pad_x, min_y - pad_y, max_y + pad_y)
}
fn points_to_svg(points: &[Value], sx: &impl Fn(f64) -> f64, sy: &impl Fn(f64) -> f64) -> String {
    points
        .iter()
        .filter_map(point)
        .map(|(x, y)| format!("{:.2},{:.2}", sx(x), sy(y)))
        .collect::<Vec<_>>()
        .join(" ")
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
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

const CSS: &str = r#"body{margin:0;background:#0b1018;color:#e6edf7;font:15px/1.5 system-ui,sans-serif}main{max-width:1180px;margin:auto;padding:30px}header,.panel{background:#121b28;border:1px solid #26364a;border-radius:12px;padding:22px;margin-bottom:18px}.eyebrow,.muted{color:#9fb0c6}.lede{font-size:17px}.warning{background:#2a2232;border-left:4px solid #f5bd55;padding:12px}.chips{display:flex;flex-wrap:wrap;gap:8px}.chips span{background:#1d2a3a;padding:5px 9px;border-radius:99px;font-size:12px}.pass{color:#72e8a4}.stop{color:#ff9b9b}table{width:100%;border-collapse:collapse}th,td{text-align:left;vertical-align:top;padding:9px;border-bottom:1px solid #27384d}code{font-size:.88em;color:#b9d7ff;word-break:break-word}.facts{display:grid;grid-template-columns:220px 1fr;gap:8px}.facts dt{color:#9fb0c6}.facts dd{margin:0}svg{max-width:100%;height:auto;border-radius:8px;display:block;background:#111722}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn report_renders_controller_scope_and_three_lane_labels_from_evaluator_data() {
        let data = serde_json::json!({
            "schema_id": "conservative_ballistic_ridge_f5_controller_v1",
            "analytical_result_identity": "f5c",
            "deterministic_repeat": true,
            "identity": "artifact",
            "evidence_status": "controller_prediction_mismatch",
            "eligible_case_ids": ["ridge_progress_056_probe"],
            "analytically_ineligible_cases": [{"case_id":"ridge_progress_072_probe","reason":"analytically_ineligible_not_run"}],
            "input_manifest_identity": "input",
            "prediction_manifest_identity": "prediction",
            "cases": [{
                "selected_attempt_index": 0,
                "evidence_status": "controller_prediction_mismatch",
                "runtime_v2": {"derived_mesa":{"OneWaypoint":{"handoff_selection":{"identity":"selection"}}}},
                "selected_attempt":{"selected_state":{"position_m":{"x":1.0,"y":2.0}}},
                "gate": {},
                "lanes": [
                    {"id":"flat-direct","controller_id":"transfer_pdg"},
                    {"id":"mesa-direct","controller_id":"transfer_pdg"},
                    {"id":"mesa-waypoint","controller_id":"transfer_waypoint_pdg"}
                ]
            }]
        });
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pd-report-f5-controller-{nonce}"));
        let html = root.join("index.html");
        let preview = root.join("preview.svg");
        write_conservative_ballistic_f5_controller_report(&html, &preview, &data).unwrap();
        let rendered = fs::read_to_string(&html).unwrap();
        assert!(rendered.contains("ridge_progress_056_probe"));
        assert!(rendered.contains("ridge_progress_072_probe"));
        assert!(rendered.contains("flat-direct"));
        assert!(rendered.contains("mesa-direct"));
        assert!(rendered.contains("mesa-waypoint"));
        assert!(preview.exists());
        let _ = fs::remove_dir_all(root);
    }
}
