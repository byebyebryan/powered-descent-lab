//! Display-only report for the F6 opt-in route integration evidence.
//!
//! The evaluator supplies a serialized artifact. This module imports no
//! planner, controller, scenario, or simulator API and cannot change a gate.

use std::{fmt::Write as _, fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

const TITLE: &str = "F6 Conservative-Ballistic Route Integration";

pub fn write_conservative_ballistic_f6_integration_report(
    path: &Path,
    preview_path: &Path,
    data: &Value,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create F6 report directory {}", parent.display())
        })?;
    }
    if let Some(parent) = preview_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let svg = build_svg(data);
    fs::write(preview_path, &svg)
        .with_context(|| format!("failed to write F6 preview {}", preview_path.display()))?;
    fs::write(path, build_html(data, &svg)?)
        .with_context(|| format!("failed to write F6 report {}", path.display()))?;
    Ok(())
}

fn build_html(data: &Value, svg: &str) -> Result<String> {
    let status = text(data, "status");
    let status_class = if status == "integration_pass" {
        "pass"
    } else {
        "stop"
    };
    let lane = data.get("lane").unwrap_or(&Value::Null);
    let mut gate_rows = String::new();
    for key in [
        "decision_supported_one_waypoint",
        "route_application_injected",
        "exact_route_join",
        "launch",
        "exactly_one_contract_pass_capture",
        "target_landing",
        "integration_pass",
    ] {
        let check = path(data, &["gate", key]);
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
    let non_claims = path(data, &["scope", "non_claims"])
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|value| format!("<li>{}</li>", esc(value.as_str().unwrap_or_default())))
        .collect::<String>();
    let capture = lane.get("waypoint_contract").unwrap_or(&Value::Null);
    let serialized = serde_json::to_string(data)?;
    Ok(format!(
        r###"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{title}</title><style>{css}</style></head><body><main>
<header><div class="eyebrow">powered descent lab · F6 opt-in development evidence</div><h1>{title}</h1>
<p class="lede">One retained <code>056</code> derived-mesa mission is resolved by the typed adapter, injected into the real scenario seam, and executed by the unchanged waypoint controller.</p>
<p class="warning"><strong>Reading the plot:</strong> the solid green curve is the sampled vehicle trajectory. The dashed gold segments show route topology only—they are not a commanded piecewise flight path or an analytical trajectory.</p>
<div class="chips"><span class="{status_class}">status {status}</span><span>repeat {repeat}</span><span>artifact {identity}</span><span>one controller lane</span></div></header>
<section class="panel"><h2>Mission and selected route</h2><dl class="facts"><dt>Source input</dt><dd><code>{input}</code></dd><dt>Decision</dt><dd><code>{decision}</code></dd><dt>Application</dt><dd><code>{application}</code></dd><dt>Selected route</dt><dd><code>{route}</code></dd><dt>Selected waypoint</dt><dd>{waypoint}</dd><dt>Controller / scenario</dt><dd><code>{controller}</code> / <code>{scenario}</code></dd></dl></section>
<section class="panel"><h2>Integrated mission view</h2><p class="muted">Gray is the exact derived mesa terrain, yellow bars are the source and target pads, gold is the selected route topology, the gold diamond is the selected waypoint, cyan is the observed contract-pass capture, and green is the target touchdown.</p>{svg}</section>
<section class="panel"><h2>Integration gate</h2><table><thead><tr><th>Gate</th><th>Result</th><th>Evidence / reason</th></tr></thead><tbody>{gate_rows}</tbody></table></section>
<section class="panel"><h2>Observed controller result</h2><dl class="facts"><dt>End reason</dt><dd>{end_reason}</dd><dt>Physical / mission outcome</dt><dd>{physical} / {mission}</dd><dt>Simulation time / physics steps</dt><dd>{sim_time} s / {steps}</dd><dt>Fuel remaining</dt><dd>{fuel} kg</dd><dt>Waypoint marker</dt><dd>{marker_count} marker, contract pass {contract_pass}, reason <code>{contract_reason}</code></dd><dt>Capture time / position</dt><dd>{capture_time} s / {capture_position}</dd></dl></section>
<section class="panel"><h2>Evidence boundary</h2><ul>{non_claims}</ul><p class="muted">This is an opt-in development seam. It does not promote conservative-ballistic selection into the ordinary planner and does not expand the demonstrated mission family.</p></section>
<script type="application/json" id="conservative-ballistic-f6-integration-data">{data}</script></main></body></html>"###,
        title = esc(TITLE),
        css = CSS,
        status_class = status_class,
        status = esc(&status),
        repeat = esc(&text(data, "deterministic_repeat")),
        identity = esc(&text(data, "identity")),
        input = esc(&text(data, "fixture_identity")),
        decision = esc(&text_path(data, &["decision", "identity"])),
        application = esc(&text_path(data, &["application", "identity"])),
        route = esc(&text(data, "selected_route_identity")),
        waypoint = esc(&point_text(data.get("selected_waypoint_position_m"))),
        controller = esc(&text(lane, "controller_id")),
        scenario = esc(&text(lane, "scenario_id")),
        gate_rows = gate_rows,
        svg = svg,
        end_reason = esc(&text(lane, "end_reason")),
        physical = esc(&text(lane, "physical_outcome")),
        mission = esc(&text(lane, "mission_outcome")),
        sim_time = esc(&text(lane, "sim_time_s")),
        steps = esc(&text(lane, "physics_steps")),
        fuel = esc(&text(lane, "fuel_remaining_kg")),
        marker_count = esc(&text(capture, "marker_count")),
        contract_pass = esc(&text(capture, "contract_pass")),
        contract_reason = esc(&text(capture, "resolution_reason")),
        capture_time = esc(&text(capture, "capture_time_s")),
        capture_position = esc(&point_text(capture.get("capture_position_m"))),
        non_claims = non_claims,
        data = esc_json(&serialized),
    ))
}

fn build_svg(data: &Value) -> String {
    let lane = data.get("lane").unwrap_or(&Value::Null);
    let terrain = path(lane, &["scenario", "world", "terrain", "points_m"])
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let samples = path(lane, &["run", "run", "samples"])
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let trajectory = samples
        .iter()
        .filter_map(|sample| path(sample, &["observation", "position_m"]))
        .cloned()
        .collect::<Vec<_>>();
    let pads = path(lane, &["scenario", "world", "landing_pads"])
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let waypoint = data.get("selected_waypoint_position_m").and_then(point);
    let capture = path(lane, &["waypoint_contract", "capture_position_m"]).and_then(point);
    let mut bounds_points = terrain.iter().filter_map(point).collect::<Vec<_>>();
    bounds_points.extend(trajectory.iter().filter_map(point));
    bounds_points.extend(waypoint);
    bounds_points.extend(capture);
    for pad in &pads {
        bounds_points.push((number(pad, "center_x_m"), number(pad, "surface_y_m")));
    }
    let (min_x, max_x, min_y, max_y) = bounds(&bounds_points);
    let sx = |x: f64| 58.0 + (x - min_x) / (max_x - min_x).max(1.0) * 1004.0;
    let sy = |y: f64| 574.0 - (y - min_y) / (max_y - min_y).max(1.0) * 470.0;
    let mut svg = String::from(
        r###"<svg xmlns="http://www.w3.org/2000/svg" width="1120" height="630" viewBox="0 0 1120 630" role="img" aria-label="F6 injected route and observed controller trajectory"><rect width="100%" height="100%" fill="#111722"/><text x="58" y="31" fill="#eff5ff" font-family="sans-serif" font-size="20">F6 integrated 056 mission</text><text x="58" y="54" fill="#aab7c7" font-family="sans-serif" font-size="12">solid observed trajectory · dashed route topology · one selected waypoint</text>"###,
    );
    if !terrain.is_empty() {
        let _ = write!(
            svg,
            "<polyline points=\"{}\" fill=\"none\" stroke=\"#8f9bad\" stroke-width=\"4\"/>",
            points_to_svg(&terrain, &sx, &sy)
        );
    }
    for pad in &pads {
        let x = number(pad, "center_x_m");
        let y = number(pad, "surface_y_m");
        let half = number(pad, "width_m") * 0.5;
        let _ = write!(
            svg,
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#f2d46c\" stroke-width=\"7\" stroke-linecap=\"round\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#f2d46c\" font-family=\"sans-serif\" font-size=\"12\">{}</text>",
            sx(x - half),
            sy(y),
            sx(x + half),
            sy(y),
            sx(x - half),
            sy(y) - 10.0,
            esc(&text(pad, "id")),
        );
    }
    if pads.len() >= 2
        && let Some((waypoint_x, waypoint_y)) = waypoint
    {
        let source = (
            number(&pads[0], "center_x_m"),
            number(&pads[0], "surface_y_m"),
        );
        let target = (
            number(&pads[1], "center_x_m"),
            number(&pads[1], "surface_y_m"),
        );
        let _ = write!(
            svg,
            "<polyline points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"none\" stroke=\"#e7b84c\" stroke-width=\"2\" stroke-dasharray=\"9 8\" opacity=\"0.85\"/>",
            sx(source.0),
            sy(source.1),
            sx(waypoint_x),
            sy(waypoint_y),
            sx(target.0),
            sy(target.1),
        );
    }
    if !trajectory.is_empty() {
        let _ = write!(
            svg,
            "<polyline points=\"{}\" fill=\"none\" stroke=\"#62d795\" stroke-width=\"3\"/>",
            points_to_svg(&trajectory, &sx, &sy)
        );
    }
    if let Some((x, y)) = waypoint {
        let _ = write!(
            svg,
            "<path d=\"M {:.2} {:.2} l 8 8 l -8 8 l -8 -8 z\" fill=\"#f3c658\" stroke=\"#fff0a8\" stroke-width=\"2\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#fff0a8\" font-family=\"sans-serif\" font-size=\"12\">selected waypoint</text>",
            sx(x),
            sy(y) - 8.0,
            sx(x) + 13.0,
            sy(y) - 13.0,
        );
    }
    if let Some((x, y)) = capture {
        let _ = write!(
            svg,
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"7\" fill=\"none\" stroke=\"#61dff2\" stroke-width=\"3\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#9cecf7\" font-family=\"sans-serif\" font-size=\"12\">contract-pass capture</text>",
            sx(x),
            sy(y),
            sx(x) + 11.0,
            sy(y) + 18.0,
        );
    }
    if let Some((x, y)) = trajectory.last().and_then(point) {
        let _ = write!(
            svg,
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"6\" fill=\"#62d795\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#a8f2ca\" font-family=\"sans-serif\" font-size=\"12\">target touchdown</text>",
            sx(x),
            sy(y),
            sx(x) - 112.0,
            sy(y) - 12.0,
        );
    }
    svg.push_str("</svg>");
    svg
}

fn path<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().try_fold(value, |current, key| current.get(key))
}

fn text_path(value: &Value, keys: &[&str]) -> String {
    path(value, keys).map_or_else(String::new, value_text)
}

fn text(value: &Value, key: &str) -> String {
    value.get(key).map_or_else(String::new, value_text)
}

fn value_text(value: &Value) -> String {
    match value {
        Value::Null => "—".to_owned(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        other => other.to_string(),
    }
}

fn number(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn point(value: &Value) -> Option<(f64, f64)> {
    Some((value.get("x")?.as_f64()?, value.get("y")?.as_f64()?))
}

fn point_text(value: Option<&Value>) -> String {
    value
        .and_then(point)
        .map_or_else(|| "—".to_owned(), |(x, y)| format!("({x:.3}, {y:.3}) m"))
}

fn points_to_svg(points: &[Value], sx: &impl Fn(f64) -> f64, sy: &impl Fn(f64) -> f64) -> String {
    points
        .iter()
        .filter_map(point)
        .map(|(x, y)| format!("{:.2},{:.2}", sx(x), sy(y)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn bounds(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    if points.is_empty() {
        return (0.0, 1.0, 0.0, 1.0);
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (
        points[0].0,
        points[0].0,
        0.0_f64.min(points[0].1),
        points[0].1,
    );
    for &(x, y) in &points[1..] {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let x_pad = ((max_x - min_x) * 0.025).max(1.0);
    let y_pad = ((max_y - min_y) * 0.08).max(1.0);
    (min_x - x_pad, max_x + x_pad, min_y - y_pad, max_y + y_pad)
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
    value.replace('<', "\\u003c").replace('>', "\\u003e")
}

const CSS: &str = r###"
:root{color-scheme:dark;font-family:Inter,ui-sans-serif,system-ui,sans-serif;background:#0b1018;color:#e8eef7}*{box-sizing:border-box}body{margin:0;background:radial-gradient(circle at 15% 0,#1b2a3e 0,#0b1018 42rem)}main{max-width:1180px;margin:auto;padding:42px 24px 72px}header{margin-bottom:24px}.eyebrow{text-transform:uppercase;letter-spacing:.13em;color:#79b8ff;font-size:.78rem;font-weight:700}h1{font-size:clamp(2rem,5vw,3.6rem);line-height:1.02;margin:.35rem 0 1rem}.lede{font-size:1.08rem;line-height:1.65;max-width:900px;color:#c6d0de}.warning{border-left:4px solid #e7b84c;background:#171b20;padding:14px 17px;line-height:1.55}.chips{display:flex;flex-wrap:wrap;gap:8px;margin-top:18px}.chips span{background:#1a2432;border:1px solid #34465e;border-radius:999px;padding:6px 10px;font-family:ui-monospace,monospace;font-size:.78rem}.chips .pass,.pass{color:#8ee7b4}.chips .stop,.stop{color:#ff9c9c}.panel{background:#111925;border:1px solid #27364a;border-radius:14px;padding:20px;margin-top:18px;box-shadow:0 14px 36px #0005}.panel h2{margin-top:0}.panel svg{width:100%;height:auto;border-radius:10px;border:1px solid #2b3a4e}.facts{display:grid;grid-template-columns:minmax(170px,.35fr) 1fr;gap:9px 20px}.facts dt{color:#91a2b7}.facts dd{margin:0;overflow-wrap:anywhere}table{width:100%;border-collapse:collapse}th,td{text-align:left;padding:10px;border-bottom:1px solid #27364a;vertical-align:top}th{color:#9fb0c4;font-size:.82rem;text-transform:uppercase;letter-spacing:.06em}code{font-family:ui-monospace,SFMono-Regular,monospace;color:#c7ddff}.muted{color:#9eacbd;line-height:1.55}li{margin:.45rem 0;line-height:1.45}@media(max-width:700px){main{padding:28px 14px}.facts{grid-template-columns:1fr}.facts dt{margin-top:8px}.panel{padding:15px}table{font-size:.82rem}}
"###;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn f6_report_labels_route_topology_and_observed_trajectory() {
        let data = json!({
            "status": "integration_pass",
            "deterministic_repeat": true,
            "identity": "artifact",
            "fixture_identity": "fixture",
            "selected_route_identity": "route",
            "selected_waypoint_position_m": {"x": 50.0, "y": 80.0},
            "scope": {"non_claims": ["not production"]},
            "decision": {"identity": "decision"},
            "application": {"identity": "application"},
            "gate": {
                "decision_supported_one_waypoint": {"passes": true, "reason": "pass"},
                "route_application_injected": {"passes": true, "reason": "pass"},
                "exact_route_join": {"passes": true, "reason": "pass"},
                "launch": {"passes": true, "reason": "pass"},
                "exactly_one_contract_pass_capture": {"passes": true, "reason": "pass"},
                "target_landing": {"passes": true, "reason": "pass"},
                "integration_pass": {"passes": true, "reason": "pass"}
            },
            "lane": {
                "controller_id": "transfer_waypoint_pdg",
                "scenario_id": "scenario",
                "end_reason": "touchdown_on_target",
                "physical_outcome": "landed_on_target",
                "mission_outcome": "success",
                "sim_time_s": 2.0,
                "physics_steps": 240,
                "fuel_remaining_kg": 10.0,
                "waypoint_contract": {
                    "marker_count": 1,
                    "contract_pass": true,
                    "resolution_reason": "contract_pass",
                    "capture_time_s": 1.0,
                    "capture_position_m": {"x": 48.0, "y": 82.0}
                },
                "scenario": {"world": {
                    "terrain": {"points_m": [{"x": 0.0, "y": 0.0}, {"x": 100.0, "y": 0.0}]},
                    "landing_pads": [
                        {"id": "source", "center_x_m": 0.0, "surface_y_m": 0.0, "width_m": 10.0},
                        {"id": "target", "center_x_m": 100.0, "surface_y_m": 0.0, "width_m": 10.0}
                    ]
                }},
                "run": {"run": {"samples": [
                    {"observation": {"position_m": {"x": 0.0, "y": 5.0}}},
                    {"observation": {"position_m": {"x": 100.0, "y": 5.0}}}
                ]}}
            }
        });
        let svg = build_svg(&data);
        let html = build_html(&data, &svg).unwrap();
        assert!(html.contains(TITLE));
        assert!(html.contains("not a commanded piecewise flight path"));
        assert!(html.contains("contract-pass capture"));
        assert!(svg.contains("observed controller trajectory"));
        assert!(svg.contains("target touchdown"));
    }
}
