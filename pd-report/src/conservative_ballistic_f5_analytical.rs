//! Display-only F5 analytical report.
//!
//! This module consumes evaluator-produced JSON.  It does not import planner,
//! controller, scenario, or simulator code and cannot regenerate a route.

use anyhow::{Context, Result};
use serde_json::Value;
use std::{fmt::Write as _, fs, path::Path};

pub fn write_conservative_ballistic_f5_analytical_report(path: &Path, data: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create F5 analytical report directory {}",
                parent.display()
            )
        })?;
    }
    let mut rows = String::new();
    let mut plots = String::new();
    for case in data
        .get("cases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = text(case, "case_id");
        let status = text(case, "status");
        let expected = "flat Direct; mesa OneWaypoint; selected V2 handoff contract passing";
        let observed = case
            .get("runtime_expectation")
            .and_then(|value| value.get("observed_handoff"))
            .and_then(|value| value.get("selection_kind"))
            .and_then(Value::as_str)
            .unwrap_or("not available");
        let stop = case
            .get("stop")
            .and_then(|value| value.get("reason"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let _ = write!(
            rows,
            "<tr><td><code>{}</code></td><td>{}</td><td class=\"{}\">{}</td><td><code>{}</code>{}</td></tr>",
            escape(&id),
            expected,
            if text(case, "eligible_for_controller") == "true" {
                "pass"
            } else {
                "stop"
            },
            escape(&status),
            escape(observed),
            if stop.is_empty() {
                String::new()
            } else {
                format!("<br><span class=\"muted\">{}</span>", escape(stop))
            }
        );
        let _ = write!(
            plots,
            "<section class=\"plot\"><h2>{}</h2><p class=\"muted\">Blue is the nominal direct ballistic path. Green is the selected analytical one-waypoint path. Gold marks the analytical waypoint; orange marks the selected runtime-V2 waypoint. This is display-only analytical evidence, not a controller command or simulator replay.</p>{}</section>",
            escape(&id),
            svg(case)
        );
    }
    let serialized = serde_json::to_string(data)?;
    let html = format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>F5 analytical result</title><style>{CSS}</style></head><body><main>
<header><div class="eyebrow">powered descent lab · F5 analytical/runtime-V2 reveal</div><h1>Fresh ridge probes: analytical result</h1><p class="lede">Expected qualitative F5 evidence is compared to generic analytical and runtime-V2 projections. A case is controller-eligible only when both stages match. This report contains no controller or simulator result.</p></header>
<section class="panel"><h2>Expected vs observed</h2><table><thead><tr><th>Case</th><th>Frozen prediction</th><th>Result</th><th>Observed semantic handoff / stop</th></tr></thead><tbody>{rows}</tbody></table><dl class="facts"><dt>Input seal</dt><dd><code>{input}</code></dd><dt>Prediction seal</dt><dd><code>{prediction}</code></dd><dt>Artifact</dt><dd><code>{artifact}</code></dd><dt>Byte-identical repeat</dt><dd>{repeat}</dd></dl></section>{plots}
<section class="panel"><h2>Scope boundary</h2><p class="muted">The compact result binds seals, per-case analytical/runtime identities, selected handoff evidence, eligibility, and typed stops. It intentionally has no controller or simulator outcome. The observed handoff kind is evidence, not a branch-specific gate.</p></section><script type="application/json" id="f5-analytical-data">{data}</script></main></body></html>"#,
        CSS = CSS,
        rows = rows,
        plots = plots,
        input = escape(&text(data, "input_manifest_identity")),
        prediction = escape(&text(data, "prediction_manifest_identity")),
        artifact = escape(&text(data, "identity")),
        repeat = escape(&text(data, "deterministic_repeat")),
        data = escape_json(&serialized)
    );
    fs::write(path, html)
        .with_context(|| format!("failed to write F5 analytical report {}", path.display()))?;
    Ok(())
}

fn svg(case: &Value) -> String {
    let visual = case.get("visual").unwrap_or(&Value::Null);
    let flat = points(visual.get("flat_twin_terrain_points_m"));
    let mesa = points(visual.get("derived_mesa_terrain_points_m"));
    let nominal = points(visual.get("nominal_direct_arc_points_m"));
    let source = points(visual.get("selected_waypoint_source_arc_points_m"));
    let target = points(visual.get("selected_waypoint_target_arc_points_m"));
    let analytical = point(visual.get("analytical_waypoint_position_m"));
    let runtime = case
        .get("runtime_expectation")
        .and_then(|value| value.get("observed_handoff"))
        .and_then(|value| value.get("waypoint_position_m"))
        .and_then(|value| point(Some(value)));
    let all = [&flat, &mesa, &nominal, &source, &target]
        .into_iter()
        .flatten()
        .copied()
        .chain(analytical)
        .chain(runtime)
        .collect::<Vec<_>>();
    if all.is_empty() {
        return "<p class=\"muted\">no analytical visual is available for this stopped case</p>"
            .to_owned();
    }
    let (min_x, max_x, min_y, max_y) = (
        all.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
        all.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
        all.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
        all.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
    );
    let bounds = (min_x, max_x.max(min_x + 1.0), min_y, max_y.max(min_y + 1.0));
    let mut out="<svg viewBox=\"0 0 960 360\" role=\"img\" aria-label=\"analytical path plot\"><rect width=\"100%\" height=\"100%\" fill=\"#10131a\"/>".to_owned();
    for (line, color, dash, width) in [
        (&flat, "#64748b", "5 4", 2.0),
        (&mesa, "#94a3b8", "", 3.0),
        (&nominal, "#38bdf8", "", 2.5),
        (&source, "#4ade80", "", 2.5),
        (&target, "#4ade80", "", 2.5),
    ] {
        out.push_str(&polyline(line, bounds, color, dash, width));
    }
    if let Some(p) = analytical {
        out.push_str(&marker(p, bounds, "#facc15", "analytical waypoint"));
    }
    if let Some(p) = runtime {
        out.push_str(&marker(p, bounds, "#fb923c", "selected runtime waypoint"));
    }
    out.push_str("<text x=\"20\" y=\"28\" fill=\"#e2e8f0\" font-size=\"15\">dashed flat twin · gray mesa · blue nominal direct · green selected waypoint</text></svg>");
    out
}
fn polyline(
    points: &[(f64, f64)],
    bounds: (f64, f64, f64, f64),
    color: &str,
    dash: &str,
    width: f64,
) -> String {
    if points.len() < 2 {
        return String::new();
    }
    let points = points
        .iter()
        .map(|p| {
            let (x, y) = scale(*p, bounds);
            format!("{x:.2},{y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let dash = if dash.is_empty() {
        String::new()
    } else {
        format!(" stroke-dasharray=\"{dash}\"")
    };
    format!(
        "<polyline fill=\"none\" stroke=\"{color}\" stroke-width=\"{width}\"{dash} points=\"{points}\"/>"
    )
}
fn marker(point: (f64, f64), bounds: (f64, f64, f64, f64), color: &str, label: &str) -> String {
    let (x, y) = scale(point, bounds);
    format!(
        "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"6\" fill=\"{color}\"><title>{}</title></circle>",
        escape(label)
    )
}
fn scale(point: (f64, f64), bounds: (f64, f64, f64, f64)) -> (f64, f64) {
    const P: f64 = 34.0;
    (
        P + (point.0 - bounds.0) / (bounds.1 - bounds.0) * (960.0 - 2.0 * P),
        360.0 - P - (point.1 - bounds.2) / (bounds.3 - bounds.2) * (360.0 - 2.0 * P),
    )
}
fn points(value: Option<&Value>) -> Vec<(f64, f64)> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
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
const CSS: &str = r#":root{color-scheme:dark;font-family:ui-sans-serif,system-ui,sans-serif;background:#0b0d12;color:#e2e8f0}body{margin:0}main{max-width:1160px;margin:0 auto;padding:32px 20px 56px}header,.panel,.plot{background:#121722;border:1px solid #273449;border-radius:12px;padding:22px;margin:0 0 20px}.eyebrow{color:#7dd3fc;font-size:.82rem;text-transform:uppercase;letter-spacing:.09em}h1{margin:.35rem 0 .6rem}h2{margin-top:0}.lede,.muted{color:#94a3b8}.lede{max-width:84ch}table{width:100%;border-collapse:collapse}th,td{padding:10px;text-align:left;border-bottom:1px solid #273449;vertical-align:top}.pass{color:#86efac;font-weight:700}.stop{color:#fca5a5;font-weight:700}code{color:#bae6fd;overflow-wrap:anywhere}svg{width:100%;height:auto;border-radius:8px;border:1px solid #273449;background:#10131a}.facts{display:grid;grid-template-columns:max-content 1fr;gap:8px 16px}.facts dt{color:#94a3b8}.facts dd{margin:0}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn renderer_only_consumes_dto() {
        let root = std::env::temp_dir().join(format!(
            "pd-report-f5-analytical-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("index.html");
        let data = json!({"input_manifest_identity":"input","prediction_manifest_identity":"prediction","identity":"artifact","deterministic_repeat":true,"cases":[{"case_id":"historical","status":"eligible_for_controller","eligible_for_controller":true,"runtime_expectation":{"observed_handoff":{"selection_kind":"primary_crossing","waypoint_position_m":{"x":5.0,"y":7.0}}},"visual":{"flat_twin_terrain_points_m":[{"x":0.0,"y":0.0},{"x":10.0,"y":0.0}],"derived_mesa_terrain_points_m":[{"x":0.0,"y":0.0},{"x":5.0,"y":4.0},{"x":10.0,"y":0.0}],"nominal_direct_arc_points_m":[{"x":0.0,"y":1.0},{"x":10.0,"y":1.0}],"selected_waypoint_source_arc_points_m":[{"x":0.0,"y":1.0},{"x":5.0,"y":7.0}],"selected_waypoint_target_arc_points_m":[{"x":5.0,"y":7.0},{"x":10.0,"y":1.0}],"analytical_waypoint_position_m":{"x":5.0,"y":7.0}}}]});
        write_conservative_ballistic_f5_analytical_report(&path, &data).unwrap();
        let html = fs::read_to_string(path).unwrap();
        assert!(html.contains("primary_crossing"));
        assert!(html.contains("no controller or simulator result"));
        assert!(html.contains("<html lang=\"en\">"));
        assert!(!html.contains("\\\""));
        let _ = fs::remove_dir_all(root);
    }
}
