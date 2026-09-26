//! Display-only static report for the waypoint direct-route characterization.
//!
//! The evaluator owns all planner/controller execution. This module consumes
//! only the serialized artifact and therefore cannot alter ordinary planning
//! or controller behavior.

use std::{fmt::Write as _, fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

const TITLE: &str = "Waypoint direct-route characterization";

pub fn write_report(path: &Path, preview_path: &Path, data: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create waypoint characterization report directory {}",
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
            "failed to write waypoint characterization preview {}",
            preview_path.display()
        )
    })?;
    fs::write(path, build_html(data, &svg)?).with_context(|| {
        format!(
            "failed to write waypoint characterization report {}",
            path.display()
        )
    })?;
    Ok(())
}

fn build_html(data: &Value, svg: &str) -> Result<String> {
    let cases = data
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut rows = String::new();
    for case in &cases {
        let planner = case.get("planner").unwrap_or(&Value::Null);
        let chord = case.get("direct_chord").unwrap_or(&Value::Null);
        let controller = case.get("controller").unwrap_or(&Value::Null);
        let planner_status = text(planner, "status");
        let controller_status = text(controller, "status");
        let mission_outcome = text(controller, "mission_outcome");
        let controller_pass = mission_outcome == "success";
        let _ = write!(
            rows,
            "<tr><th scope=\"row\"><code>{}</code><br><span class=\"muted\">{}</span></th><td>{} / {}<br><span class=\"muted\">clear {} · clearance {} m</span></td><td class=\"{}\">{}<br><span class=\"muted\">{}</span></td><td class=\"{}\">mission {} / physical {}<br><span class=\"muted\">status {} · end {}<br>global min hull (touchdown-inclusive) {} m<br>en-route min hull {} m · apex {} m</span></td></tr>",
            esc(&text(case, "id")),
            esc(&text(case, "label")),
            esc(&text(case, "terrain_kind")),
            esc(&format!("angle {}°", text(case, "route_angle_deg"))),
            esc(&text(chord, "clear")),
            esc(&number(chord, "minimum_clearance_m")),
            if planner_status == "accepted" {
                "pass"
            } else {
                "stop"
            },
            esc(&format!(
                "{}{}",
                planner_status,
                planner
                    .get("topology")
                    .map(|value| format!(" / {}", label(value)))
                    .unwrap_or_default()
            )),
            esc(&planner_rejection(planner)),
            if controller_pass { "pass" } else { "stop" },
            esc(&mission_outcome),
            esc(&text(controller, "physical_outcome")),
            esc(&controller_status),
            esc(&text(controller, "end_reason")),
            esc(&optional_number(
                controller,
                "observed_global_min_hull_clearance_m"
            )),
            esc(&optional_number(
                controller,
                "observed_en_route_min_hull_clearance_m"
            )),
            esc(&optional_number(controller, "observed_apex_above_target_m")),
        );
    }
    let non_claims = data
        .get("scope")
        .and_then(|scope| scope.get("non_claims"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|claim| format!("<li>{}</li>", esc(claim.as_str().unwrap_or_default())))
        .collect::<String>();
    let claims = data
        .get("scope")
        .and_then(|scope| scope.get("claims"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|claim| format!("<li>{}</li>", esc(claim.as_str().unwrap_or_default())))
        .collect::<String>();
    let serialized = serde_json::to_string(data)?;
    Ok(format!(
        r###"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{title}</title><style>{css}</style></head><body><main>
<header><div class="eyebrow">powered descent lab · opt-in planning characterization</div><h1>{title}</h1>
<p class="lede">A deterministic comparison of the unchanged V1 planner's direct chord with the unchanged direct transfer controller over continuous terrain and bounded obstacle controls.</p>
<p class="warning"><strong>Read the lanes separately:</strong> the planner row is a typed setup decision, while the green trajectory is an observed controller trace. The trace is not a planner path, route certificate, or proof of planner acceptance.</p>
<div class="chips"><span>schema {schema}</span><span>fixture {fixture}</span><span>controller {controller}</span><span>identity {identity}</span></div></header>
<section class="panel"><h2>Frozen definitions</h2><dl class="facts"><dt>Direct</dt><dd><code>{direct}</code></dd><dt>Route composition</dt><dd>{route}</dd><dt>Planner policy</dt><dd><code>{policy}</code></dd><dt>Analytical lane</dt><dd><code>{analytical}</code>: {analytical_reason}</dd><dt>Repeat rule</dt><dd><code>{repeat}</code></dd></dl></section>
<section class="panel"><h2>Characterization rows</h2><table><thead><tr><th>Case</th><th>Terrain / geometry</th><th>Planner + direct chord</th><th>Controller observation</th></tr></thead><tbody>{rows}</tbody></table><p class="muted">Chord clearance is the exact core corridor query for the endpoint-shaped direct centerline. Controller clearance is sampled: global includes endpoint contact, while en-route uses the source-transition-end through target-transition-start full-envelope progress window; neither is a continuous-time certificate.</p></section>
<section class="panel"><h2>Terrain and observed traces</h2>{svg}</section>
<section class="panel"><h2>Claims in scope</h2><ul>{claims}</ul><h2>Explicit non-claims</h2><ul>{non_claims}</ul></section>
<script type="application/json" id="waypoint-direct-characterization-data">{data}</script></main></body></html>"###,
        title = esc(TITLE),
        css = CSS,
        schema = esc(&format!(
            "{} v{}",
            text(data, "schema_id"),
            text(data, "schema_version")
        )),
        fixture = esc(&text(data, "fixture_identity")),
        controller = esc(&text(data, "controller_id")),
        identity = esc(&text(data, "identity")),
        direct = esc(&text(data, "direct_definition")),
        route = esc(&text(data, "route_definition")),
        policy = esc(&text_path(data, &["planner_policy", "policy_version"])),
        analytical = esc(&text(
            data.get("analytical").unwrap_or(&Value::Null),
            "status"
        )),
        analytical_reason = esc(&text(
            data.get("analytical").unwrap_or(&Value::Null),
            "reason"
        )),
        repeat = esc(&text(data, "deterministic_repeat")),
        rows = rows,
        svg = svg,
        claims = claims,
        non_claims = non_claims,
        data = esc_json(&serialized),
    ))
}

fn build_svg(data: &Value) -> String {
    let cases = data
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let width = 1120.0;
    let panel_height = 128.0;
    let height = 78.0 + panel_height * cases.len().max(1) as f64;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.0}\" height=\"{height:.0}\" viewBox=\"0 0 {width:.0} {height:.0}\" role=\"img\" aria-label=\"Waypoint direct characterization terrain and controller traces\"><rect width=\"100%\" height=\"100%\" fill=\"#111722\"/><text x=\"28\" y=\"28\" fill=\"#eff5ff\" font-family=\"sans-serif\" font-size=\"18\">Terrain (gray), exact endpoint-shaped direct chord (gold dashed), observed controller trace (green)</text>"
    );
    for (index, case) in cases.iter().enumerate() {
        let y0 = 46.0 + panel_height * index as f64;
        let terrain = path(case, &["terrain_points_m"])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let trajectory = path(case, &["controller", "trajectory"])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let chord = path(case, &["direct_chord", "endpoint_shaped_centerline_m"])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let source = case
            .get("source_pad")
            .and_then(pad_point_from_value)
            .unwrap_or((0.0, 0.0));
        let target = case
            .get("target_pad")
            .and_then(pad_point_from_value)
            .unwrap_or((1.0, 0.0));
        let mut bounds_points = terrain
            .iter()
            .filter_map(point_from_value)
            .collect::<Vec<_>>();
        bounds_points.extend(
            trajectory
                .iter()
                .filter_map(|sample| sample.get("position_m").and_then(point_from_value)),
        );
        bounds_points.extend(chord.iter().filter_map(point_from_value));
        bounds_points.push(source);
        bounds_points.push(target);
        let (min_x, max_x, min_y, max_y) = bounds(&bounds_points);
        let sx = |x: f64| 190.0 + (x - min_x) / (max_x - min_x).max(1.0) * 884.0;
        let sy = |y: f64| y0 + 100.0 - (y - min_y) / (max_y - min_y).max(1.0) * 86.0;
        let _ = write!(
            svg,
            "<text x=\"28\" y=\"{:.2}\" fill=\"#f5d77b\" font-family=\"sans-serif\" font-size=\"13\">{}</text>",
            y0 + 18.0,
            esc(&text(case, "id"))
        );
        let _ = write!(
            svg,
            "<line x1=\"28\" y1=\"{:.2}\" x2=\"1092\" y2=\"{:.2}\" stroke=\"#2e394a\" stroke-width=\"1\"/>",
            y0 + panel_height - 3.0,
            y0 + panel_height - 3.0
        );
        if terrain.len() > 1 {
            let _ = write!(
                svg,
                "<polyline points=\"{}\" fill=\"none\" stroke=\"#9aa6b8\" stroke-width=\"3\"/>",
                points_to_svg(&terrain, &sx, &sy)
            );
        }
        if chord.len() > 1 {
            let _ = write!(
                svg,
                "<polyline points=\"{}\" fill=\"none\" stroke=\"#edc45f\" stroke-width=\"2\" stroke-dasharray=\"7 6\"/>",
                points_to_svg(&chord, &sx, &sy)
            );
        }
        if trajectory.len() > 1 {
            let trajectory_points = trajectory
                .iter()
                .filter_map(|sample| sample.get("position_m").cloned())
                .collect::<Vec<_>>();
            let _ = write!(
                svg,
                "<polyline points=\"{}\" fill=\"none\" stroke=\"#62d795\" stroke-width=\"2\"/>",
                points_to_svg(&trajectory_points, &sx, &sy)
            );
        }
    }
    svg.push_str("</svg>");
    svg
}

fn planner_rejection(planner: &Value) -> String {
    match planner.get("rejection_code").and_then(Value::as_str) {
        Some(code) => format!("rejection {code}"),
        None => "typed acceptance".to_owned(),
    }
}

fn points_to_svg<Fx, Fy>(points: &[Value], sx: &Fx, sy: &Fy) -> String
where
    Fx: Fn(f64) -> f64,
    Fy: Fn(f64) -> f64,
{
    points
        .iter()
        .filter_map(point_from_value)
        .map(|(x, y)| format!("{:.2},{:.2}", sx(x), sy(y)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn bounds(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in points.iter().copied() {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    if !min_x.is_finite() {
        return (0.0, 1.0, 0.0, 1.0);
    }
    (min_x, max_x, min_y, max_y)
}

fn point_from_value(value: &Value) -> Option<(f64, f64)> {
    Some((value.get("x")?.as_f64()?, value.get("y")?.as_f64()?))
}

fn pad_point_from_value(value: &Value) -> Option<(f64, f64)> {
    Some((
        value.get("center_x_m")?.as_f64()?,
        value.get("surface_y_m")?.as_f64()?,
    ))
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .map(label)
        .unwrap_or_else(|| "n/a".to_owned())
}

fn text_path(value: &Value, keys: &[&str]) -> String {
    path(value, keys)
        .map(label)
        .unwrap_or_else(|| "n/a".to_owned())
}

fn optional_number(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_f64)
        .map(|number| format!("{number:.2}"))
        .unwrap_or_else(|| "n/a".to_owned())
}

fn number(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_f64)
        .map(|number| format!("{number:.2}"))
        .unwrap_or_else(|| "n/a".to_owned())
}

fn label(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "n/a".to_owned(),
        _ => value.to_string(),
    }
}

fn path<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().try_fold(value, |value, key| value.get(*key))
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

const CSS: &str = r#"
:root{color-scheme:dark;--bg:#111722;--panel:#182231;--ink:#eff5ff;--muted:#aab7c7;--line:#344257;--pass:#62d795;--stop:#ff9475;--gold:#edc45f}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--ink);font:15px/1.5 system-ui,-apple-system,sans-serif}main{max-width:1220px;margin:0 auto;padding:42px 34px 70px}h1{font-size:36px;line-height:1.08;margin:8px 0 15px}h2{font-size:21px;margin:0 0 14px}.eyebrow{color:var(--gold);font-size:12px;letter-spacing:.13em;text-transform:uppercase}.lede{font-size:18px;max-width:920px;color:#d6e0ee}.warning{background:#332b1c;border:1px solid #715d2a;border-radius:8px;padding:13px 16px;max-width:1050px}.chips{display:flex;gap:8px;flex-wrap:wrap;margin:18px 0}.chips span{border:1px solid var(--line);border-radius:999px;padding:4px 9px;color:var(--muted);font-size:12px}.panel{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:22px;margin:18px 0;overflow:auto}.facts{display:grid;grid-template-columns:190px 1fr;gap:8px 16px;margin:0}.facts dt{color:var(--muted)}.facts dd{margin:0}code{color:#f7d884}table{border-collapse:collapse;width:100%;min-width:820px}th,td{border-bottom:1px solid var(--line);padding:11px 10px;text-align:left;vertical-align:top}thead th{color:var(--muted);font-size:12px;text-transform:uppercase;letter-spacing:.06em}.pass{color:var(--pass)}.stop{color:var(--stop)}.muted{color:var(--muted);font-size:13px}ul{padding-left:22px}svg{display:block;width:100%;height:auto;background:#111722;border-radius:7px}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renderer_keeps_case_rows_and_evidence_boundary_visible() {
        let data = json!({
            "schema_id": "waypoint_direct_characterization_v1",
            "schema_version": 1,
            "fixture_identity": "fixture",
            "controller_id": "transfer_pdg_v1",
            "identity": "artifact",
            "direct_definition": "zero operational handoffs",
            "route_definition": "one leg",
            "planner_algorithm_id": "heightfield_visibility_v1",
            "planner_policy": {"policy_version": "heightfield_visibility_policy_v1"},
            "deterministic_repeat": "semantic_json_excluding_wall_timing",
            "analytical": {"status": "not_mapped", "reason": "distinct fixture"},
            "scope": {
                "claims": ["planner acceptance is retained"],
                "non_claims": ["controller traces are not planner paths"]
            },
            "cases": [
                {"id":"continuous_flat_r00","label":"flat","terrain_kind":"continuous_flat","route_angle_deg":0.0,"direct_chord":{"clear":false,"minimum_clearance_m":-1.0,"endpoint_shaped_centerline_m":[{"x":0.0,"y":5.0},{"x":1.0,"y":5.0}]},"planner":{"status":"rejected","rejection_code":"no_route_within_policy"},"controller":{"status":"completed","mission_outcome":"success","physical_outcome":"landed_on_target","end_reason":"touchdown_on_target","observed_global_min_hull_clearance_m":-0.1,"observed_en_route_min_hull_clearance_m":1.0,"observed_apex_above_target_m":2.0},"source_pad":{"id":"source","center_x_m":0.0,"surface_y_m":0.0,"width_m":10.0},"target_pad":{"id":"target","center_x_m":1.0,"surface_y_m":0.0,"width_m":10.0},"terrain_points_m":[]},
                {"id":"continuous_uphill_r+30","label":"uphill","terrain_kind":"continuous_uphill","route_angle_deg":30.0,"direct_chord":{"clear":false,"minimum_clearance_m":-1.0,"endpoint_shaped_centerline_m":[{"x":0.0,"y":5.0},{"x":1.0,"y":5.0}]},"planner":{"status":"accepted","topology":"direct"},"controller":{"status":"completed","mission_outcome":"failed_timeout","physical_outcome":"timed_out","end_reason":"max_time_reached"},"source_pad":{"id":"source","center_x_m":0.0,"surface_y_m":0.0,"width_m":10.0},"target_pad":{"id":"target","center_x_m":1.0,"surface_y_m":0.0,"width_m":10.0},"terrain_points_m":[]},
                {"id":"continuous_downhill_r-30","label":"downhill","terrain_kind":"continuous_downhill","route_angle_deg":-30.0,"direct_chord":{"clear":false,"minimum_clearance_m":-1.0,"endpoint_shaped_centerline_m":[{"x":0.0,"y":5.0},{"x":1.0,"y":5.0}]},"planner":{"status":"accepted","topology":"direct"},"controller":{"status":"completed","mission_outcome":"failed_timeout","physical_outcome":"timed_out","end_reason":"max_time_reached"},"source_pad":{"id":"source","center_x_m":0.0,"surface_y_m":0.0,"width_m":10.0},"target_pad":{"id":"target","center_x_m":1.0,"surface_y_m":0.0,"width_m":10.0},"terrain_points_m":[]},
                {"id":"bounded_narrow_ridge","label":"ridge","terrain_kind":"bounded_narrow_ridge","route_angle_deg":0.0,"direct_chord":{"clear":false,"minimum_clearance_m":-1.0,"endpoint_shaped_centerline_m":[{"x":0.0,"y":5.0},{"x":1.0,"y":5.0}]},"planner":{"status":"rejected","rejection_code":"no_route_within_policy"},"controller":{"status":"completed","mission_outcome":"failed_crash","physical_outcome":"crashed","end_reason":"crash"},"source_pad":{"id":"source","center_x_m":0.0,"surface_y_m":0.0,"width_m":10.0},"target_pad":{"id":"target","center_x_m":1.0,"surface_y_m":0.0,"width_m":10.0},"terrain_points_m":[]},
                {"id":"bounded_broad_mesa","label":"mesa","terrain_kind":"bounded_broad_mesa","route_angle_deg":0.0,"direct_chord":{"clear":false,"minimum_clearance_m":-1.0,"endpoint_shaped_centerline_m":[{"x":0.0,"y":5.0},{"x":1.0,"y":5.0}]},"planner":{"status":"rejected","rejection_code":"loft_limit_exceeded"},"controller":{"status":"completed","mission_outcome":"failed_crash","physical_outcome":"crashed","end_reason":"crash"},"source_pad":{"id":"source","center_x_m":0.0,"surface_y_m":0.0,"width_m":10.0},"target_pad":{"id":"target","center_x_m":1.0,"surface_y_m":0.0,"width_m":10.0},"terrain_points_m":[]}
            ]
        });
        let html = build_html(&data, &build_svg(&data)).expect("report html");
        assert!(html.contains("continuous_flat_r00"));
        assert!(html.contains("bounded_broad_mesa"));
        assert!(html.contains("controller traces are not planner paths"));
        assert!(html.contains("semantic_json_excluding_wall_timing"));
        assert!(html.contains("exact endpoint-shaped direct chord"));
        assert!(html.contains("en-route min hull"));
        assert!(html.contains("mission success"));
        assert!(html.contains("physical timed_out"));
        assert!(html.contains("class=\"stop\">mission failed_timeout"));
        assert!(html.contains("global min hull (touchdown-inclusive)"));
        assert!(html.contains("center_x_m"));
    }
}
