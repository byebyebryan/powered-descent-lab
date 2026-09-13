//! Display-only H2b report for the analytical ridge held-out artifact.
//!
//! The evaluator validates and samples all analytical evidence before this
//! module sees it.  This renderer only lays out the supplied DTO/JSON and does
//! not invoke the planner, candidate search, or runtime projector.

use std::{fmt::Write as _, fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

pub fn write_conservative_ballistic_heldout_report(path: &Path, data: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create held-out analytical report directory {}",
                parent.display()
            )
        })?;
    }
    let cases = data
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut rows = String::new();
    let mut plots = String::new();
    for case in &cases {
        let id = text(case, "case_id");
        let expectation = case
            .get("expectation")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("unavailable");
        let _ = write!(
            rows,
            "<tr><td><code>{}</code></td><td>flat: <code>Direct</code>; mesa: <code>OneWaypoint</code></td><td class=\"pass\">{}</td><td><code>{}</code></td></tr>",
            escape(&id),
            escape(expectation),
            escape(&text(case, "analytical_canary_identity")),
        );
        let _ = write!(
            plots,
            "<section class=\"plot\"><h2>{}</h2><p class=\"muted\">Flat twin and derived mesa share the frozen raw case. Blue is the nominal direct ballistic arc. Green is the selected one-waypoint ballistic legs; the gold and orange markers are its analytical and runtime waypoints. These are analytical display samples, not controller commands or a simulator replay.</p>{}</section>",
            escape(&id),
            build_case_svg(case),
        );
    }
    if let Some(failure) = data.get("failure").filter(|value| !value.is_null()) {
        let id = text(failure, "case_id");
        let stage = text(failure, "stage");
        let error = text(failure, "error");
        let _ = write!(
            rows,
            "<tr><td><code>{}</code></td><td>frozen waypoint runtime projection</td><td class=\"stop\">stopped at {}</td><td><code>{}</code><br><span class=\"muted\">{}</span></td></tr>",
            escape(&id),
            escape(&stage),
            escape(&text(failure, "analytical_canary_identity")),
            escape(&error),
        );
        let _ = write!(
            plots,
            "<section class=\"plot stop-panel\"><h2>{}: fail-closed analytical stop</h2><p class=\"muted\">The generic candidate projection validated, but construction of the runtime waypoint projection stopped here. The frozen input and prediction remain unchanged. Error: <code>{}</code>.</p>{}</section>",
            escape(&id),
            escape(&error),
            build_case_svg(failure),
        );
    }
    let serialized = serde_json::to_string(data)?;
    let html = format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Ridge held-out analytical result</title><style>{CSS}</style></head><body><main>
<header><div class="eyebrow">powered descent lab · H2b analytical result</div><h1>Ridge held-out: analytical result</h1>
<p class="lede">Two frozen inputs were evaluated only through the generic analytical and runtime-projection APIs. Both must match their pre-evaluation predictions before any controller shadow is allowed. This report contains no controller or simulator outcomes.</p></header>
<section class="panel"><h2>Expected vs observed analytical result</h2><table><thead><tr><th>Case</th><th>Frozen prediction / observed topology</th><th>Comparison</th><th>Analytical canary</th></tr></thead><tbody>{rows}</tbody></table>
<dl class="facts"><dt>Input manifest</dt><dd><code>{input}</code></dd><dt>Prediction manifest</dt><dd><code>{prediction}</code></dd><dt>Artifact identity</dt><dd><code>{artifact}</code></dd><dt>Byte-identical repeat</dt><dd>{repeat}</dd></dl></section>
{plots}
<section class="panel"><h2>Scope boundary</h2><p class="muted">The compact tracked analytical result binds the input, prediction, canary, candidate, runtime where available, and expectation identities. A stopped result is not an H3 authority. Controller outcomes remain deliberately absent.</p></section>
<script type="application/json" id="heldout-analytical-data">{data}</script></main></body></html>"#,
        CSS = CSS,
        rows = rows,
        plots = plots,
        input = escape(&text(data, "input_manifest_identity")),
        prediction = escape(&text(data, "prediction_manifest_identity")),
        artifact = escape(&text(data, "identity")),
        repeat = escape(&text(data, "deterministic_repeat")),
        data = escape_json(&serialized),
    );
    fs::write(path, html).with_context(|| {
        format!(
            "failed to write held-out analytical report {}",
            path.display()
        )
    })?;
    Ok(())
}

fn build_case_svg(case: &Value) -> String {
    let visual = case.get("visual").unwrap_or(&Value::Null);
    let flat = points(visual.get("flat_twin_terrain_points_m"));
    let mesa = points(visual.get("derived_mesa_terrain_points_m"));
    let nominal = points(visual.get("nominal_direct_arc_points_m"));
    let source_leg = points(visual.get("selected_waypoint_source_arc_points_m"));
    let target_leg = points(visual.get("selected_waypoint_target_arc_points_m"));
    let analytical_waypoint = point(visual.get("analytical_waypoint_position_m"));
    let runtime_waypoint = point(visual.get("runtime_waypoint_position_m"));
    let all = [&flat, &mesa, &nominal, &source_leg, &target_leg]
        .into_iter()
        .flatten()
        .copied()
        .chain(analytical_waypoint)
        .chain(runtime_waypoint)
        .collect::<Vec<_>>();
    if all.is_empty() {
        return "<p class=\"muted\">validated visual evidence unavailable</p>".to_owned();
    }
    let min_x = all
        .iter()
        .map(|point| point.0)
        .fold(f64::INFINITY, f64::min);
    let max_x = all
        .iter()
        .map(|point| point.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = all
        .iter()
        .map(|point| point.1)
        .fold(f64::INFINITY, f64::min);
    let max_y = all
        .iter()
        .map(|point| point.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let bounds = Bounds {
        min_x,
        max_x: max_x.max(min_x + 1.0),
        min_y,
        max_y: max_y.max(min_y + 1.0),
    };
    let width = 960.0;
    let height = 360.0;
    let mut svg = format!(
        "<svg viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"analytical path plot\"><rect width=\"100%\" height=\"100%\" fill=\"#10131a\"/>"
    );
    svg.push_str(&polyline(
        &flat, &bounds, width, height, "#64748b", "5 4", 2.0,
    ));
    svg.push_str(&polyline(&mesa, &bounds, width, height, "#94a3b8", "", 3.0));
    svg.push_str(&polyline(
        &nominal, &bounds, width, height, "#38bdf8", "", 2.5,
    ));
    svg.push_str(&polyline(
        &source_leg,
        &bounds,
        width,
        height,
        "#4ade80",
        "",
        2.5,
    ));
    svg.push_str(&polyline(
        &target_leg,
        &bounds,
        width,
        height,
        "#4ade80",
        "",
        2.5,
    ));
    if let Some(point) = analytical_waypoint {
        svg.push_str(&marker(
            point,
            &bounds,
            width,
            height,
            "#facc15",
            "analytical waypoint",
        ));
    }
    if let Some(point) = runtime_waypoint {
        svg.push_str(&marker(
            point,
            &bounds,
            width,
            height,
            "#fb923c",
            "runtime waypoint",
        ));
    }
    svg.push_str("<text x=\"20\" y=\"28\" fill=\"#e2e8f0\" font-size=\"15\">dashed flat twin · gray mesa · blue nominal direct · green selected one-waypoint</text></svg>");
    svg
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

fn polyline(
    points: &[(f64, f64)],
    bounds: &Bounds,
    width: f64,
    height: f64,
    color: &str,
    dash: &str,
    stroke_width: f64,
) -> String {
    if points.len() < 2 {
        return String::new();
    }
    let coordinates = points
        .iter()
        .map(|point| {
            let (x, y) = scale(*point, bounds, width, height);
            format!("{x:.2},{y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let dash = if !dash.is_empty() {
        format!(" stroke-dasharray=\"{dash}\"")
    } else {
        String::new()
    };
    format!(
        "<polyline fill=\"none\" stroke=\"{color}\" stroke-width=\"{stroke_width}\"{dash} points=\"{coordinates}\"/>"
    )
}

fn marker(
    point: (f64, f64),
    bounds: &Bounds,
    width: f64,
    height: f64,
    color: &str,
    label: &str,
) -> String {
    let (x, y) = scale(point, bounds, width, height);
    format!(
        "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"6\" fill=\"{color}\"><title>{}</title></circle>",
        escape(label)
    )
}

fn scale(point: (f64, f64), bounds: &Bounds, width: f64, height: f64) -> (f64, f64) {
    const PAD: f64 = 34.0;
    let x = PAD + (point.0 - bounds.min_x) / (bounds.max_x - bounds.min_x) * (width - 2.0 * PAD);
    let y = height
        - PAD
        - (point.1 - bounds.min_y) / (bounds.max_y - bounds.min_y) * (height - 2.0 * PAD);
    (x, y)
}

fn points(value: Option<&Value>) -> Vec<(f64, f64)> {
    value
        .and_then(Value::as_array)
        .map(|points| {
            points
                .iter()
                .filter_map(|value| point(Some(value)))
                .collect()
        })
        .unwrap_or_default()
}

fn point(value: Option<&Value>) -> Option<(f64, f64)> {
    let value = value?;
    Some((value.get("x")?.as_f64()?, value.get("y")?.as_f64()?))
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

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_json(value: &str) -> String {
    value.replace('<', "\\u003c").replace('>', "\\u003e")
}

const CSS: &str = r#"
:root { color-scheme: dark; font-family: ui-sans-serif, system-ui, sans-serif; background:#0b0d12; color:#e2e8f0; }
body { margin:0; } main { max-width:1160px; margin:0 auto; padding:32px 20px 56px; }
header, .panel, .plot { background:#121722; border:1px solid #273449; border-radius:12px; padding:22px; margin:0 0 20px; }
.eyebrow { color:#7dd3fc; font-size:.82rem; text-transform:uppercase; letter-spacing:.09em; } h1 { margin:.35rem 0 .6rem; } h2 { margin-top:0; }
.lede { max-width:84ch; color:#cbd5e1; } .muted { color:#94a3b8; } table { width:100%; border-collapse:collapse; } th,td { padding:10px; text-align:left; border-bottom:1px solid #273449; vertical-align:top; }
.pass { color:#86efac; font-weight:700; } .stop { color:#fca5a5; font-weight:700; } .stop-panel { border-color:#7f1d1d; } code { color:#bae6fd; overflow-wrap:anywhere; } svg { width:100%; height:auto; border-radius:8px; border:1px solid #273449; background:#10131a; }
.facts { display:grid; grid-template-columns:max-content 1fr; gap:8px 16px; } .facts dt { color:#94a3b8; } .facts dd { margin:0; }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn renderer_consumes_display_dto_without_planner_dependency() {
        let root = std::env::temp_dir().join(format!(
            "pd-report-heldout-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("index.html");
        let data = json!({
            "input_manifest_identity":"inputs",
            "prediction_manifest_identity":"predictions",
            "identity":"artifact",
            "deterministic_repeat":true,
            "cases":[{"case_id":"ridge_progress_050_probe","analytical_canary_identity":"canary","expectation":{"status":"passed"},"visual":{
                "flat_twin_terrain_points_m":[{"x":0.0,"y":0.0},{"x":10.0,"y":0.0}],
                "derived_mesa_terrain_points_m":[{"x":0.0,"y":0.0},{"x":5.0,"y":4.0},{"x":10.0,"y":0.0}],
                "nominal_direct_arc_points_m":[{"x":0.0,"y":1.0},{"x":10.0,"y":1.0}],
                "selected_waypoint_source_arc_points_m":[{"x":0.0,"y":1.0},{"x":5.0,"y":7.0}],
                "selected_waypoint_target_arc_points_m":[{"x":5.0,"y":7.0},{"x":10.0,"y":1.0}],
                "analytical_waypoint_position_m":{"x":5.0,"y":7.0},"runtime_waypoint_position_m":{"x":5.1,"y":6.9}
            }}]
        });
        write_conservative_ballistic_heldout_report(&path, &data).unwrap();
        let html = fs::read_to_string(&path).unwrap();
        assert!(html.contains("ridge_progress_050_probe"));
        assert!(html.contains("Expected vs observed analytical result"));
        assert!(html.contains("controller or simulator outcomes"));
        let _ = fs::remove_dir_all(root);
    }
}
