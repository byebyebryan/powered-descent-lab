//! Renderer for the evaluator-only full-controller ridge-canary shadow.
//!
//! The renderer consumes the serialized shadow projection as JSON rather than
//! depending on `pd-eval`, keeping the report crate below the evaluator layer.

use std::{fmt::Write as _, fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

pub fn write_controller_shadow_report(path: &Path, data: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let serialized = serde_json::to_string(data)?;
    let lanes = data
        .get("lanes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut rows = String::new();
    for lane in &lanes {
        let id = text(lane, "id");
        let class = text(lane, "class");
        let reason = text(lane, "causal_reason");
        let end_reason = text(lane, "end_reason");
        let contact = contact_summary(lane);
        let _ = write!(
            rows,
            "<tr><td>{}</td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
            escape(&id),
            escape(&class),
            escape(&end_reason),
            escape(&reason),
            escape(&contact)
        );
    }
    let html = format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Ridge canary controller shadow</title><style>{CSS}</style></head><body><main>
<header><div class="eyebrow">powered descent lab · full controller shadow</div><h1>Ridge canary: observed controller lanes</h1>
<p class="lede"><strong>Three frozen lanes</strong> at 120 Hz physics / 60 Hz controller. Dashed paths are canonical analytical context; solid paths and markers are simulator observations.</p></header>
<section class="panel"><h2>Lane comparison</h2><table><thead><tr><th>Lane</th><th>Classification</th><th>End reason</th><th>Causal evidence</th><th>Terrain contact evidence</th></tr></thead><tbody>{rows}</tbody></table></section>
<section class="panel"><h2>Trajectory overlay</h2><p class="muted">Gray is the derived mesa terrain. Dashed gold/cyan/magenta paths are the selected nominal direct and waypoint-leg analytical witnesses. Solid blue/red/green paths are simulated lanes; circles mark simulation start and final/crash samples. The analytical bridge is not replayed as a command trace.</p>{svg}</section>
<section class="panel"><h2>Provenance and route adapter</h2><dl class="facts"><dt>Analytical report identity</dt><dd>{report}</dd><dt>Analytical canary identity</dt><dd>{canary}</dd><dt>Mesa identity</dt><dd>{mesa}</dd><dt>Waypoint candidate identity</dt><dd>{waypoint}</dd><dt>Controller direct route</dt><dd>{direct_route}</dd><dt>Adapter status</dt><dd>{adapter}</dd><dt>Mapping</dt><dd>{mapping}</dd></dl></section>
<script type="application/json" id="controller-shadow-data">{data}</script></main></body></html>"#,
        CSS = CSS,
        rows = rows,
        svg = build_svg(data),
        report = escape(&text(data, "analytical_report_identity")),
        canary = escape(&text(data, "analytical_canary_identity")),
        mesa = escape(&text(data, "mesa_identity")),
        waypoint = escape(&text(data, "waypoint_candidate_identity")),
        direct_route = escape(&direct_route_summary(data)),
        adapter = escape(
            &data
                .get("route_adapter")
                .map(|adapter| {
                    adapter
                        .get("error")
                        .and_then(Value::as_str)
                        .map_or_else(|| "valid".to_owned(), |error| format!("invalid: {error}"))
                })
                .unwrap_or_else(|| "unknown".to_owned()),
        ),
        mapping = escape(
            &data
                .get("route_adapter")
                .map_or_else(String::new, |adapter| {
                    text(adapter, "bridge_mapping_note")
                })
        ),
        data = escape_json(&serialized),
    );
    fs::write(path, html).with_context(|| {
        format!(
            "failed to write controller shadow report {}",
            path.display()
        )
    })?;
    Ok(())
}

fn direct_route_summary(data: &Value) -> String {
    let Some(route) = data
        .get("direct_route")
        .and_then(|direct_route| direct_route.get("route"))
    else {
        return "unavailable".to_owned();
    };
    let source = text(route, "source_pad_id");
    let target = text(route, "target_pad_id");
    let angle = route
        .get("route_angle_deg")
        .and_then(Value::as_f64)
        .map_or_else(
            || "unavailable".to_owned(),
            |value| format!("{value:.6} deg"),
        );
    let radius = route
        .get("route_radius_m")
        .and_then(Value::as_f64)
        .map_or_else(|| "unavailable".to_owned(), |value| format!("{value:.6} m"));
    let waypoints = route
        .get("waypoints")
        .and_then(Value::as_array)
        .map_or_else(|| "unavailable".to_owned(), |value| value.len().to_string());
    let validation = data
        .get("direct_route")
        .and_then(|direct_route| direct_route.get("structural_validation"))
        .and_then(Value::as_str)
        .unwrap_or("structural validation unavailable");
    format!(
        "{source} -> {target}; angle={angle}; radius={radius}; waypoints={waypoints}; {validation}"
    )
}

fn contact_summary(lane: &Value) -> String {
    let Some(contact) = lane.get("terrain_contact") else {
        return "unavailable".to_owned();
    };
    let center = point_text(contact.get("vehicle_center_m"));
    let point = point_text(contact.get("contact_point_m"));
    let kind = text(contact, "contact_kind");
    let residual = number_text(contact, "contact_residual_m");
    let contact_terrain = number_text(contact, "contact_terrain_height_m");
    let reconstruction = text(contact, "reconstruction_valid");
    let mesa_span = text(contact, "within_derived_mesa_span");
    format!(
        "vehicle center={center}; contact point={point}; kind={kind}; contact terrain={contact_terrain}; residual={residual}; reconstruction_valid={reconstruction}; within_derived_mesa_span={mesa_span}"
    )
}

fn point_text(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "unavailable".to_owned();
    };
    let (Some(x), Some(y)) = (
        value.get("x").and_then(Value::as_f64),
        value.get("y").and_then(Value::as_f64),
    ) else {
        return "unavailable".to_owned();
    };
    format!("({x:.6},{y:.6})")
}

fn number_text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_f64)
        .map_or_else(|| "unavailable".to_owned(), |number| format!("{number:.6}"))
}

pub fn write_controller_shadow_preview_svg(path: &Path, data: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, build_svg(data)).with_context(|| {
        format!(
            "failed to write controller shadow preview {}",
            path.display()
        )
    })?;
    Ok(())
}

fn build_svg(data: &Value) -> String {
    let width = 1120.0;
    let height = 620.0;
    let (min_x, max_x, min_y, max_y) = bounds(data);
    let sx = |x: f64| 48.0 + ((x - min_x) / (max_x - min_x).max(1.0)) * 1024.0;
    let sy = |y: f64| 570.0 - ((y - min_y) / (max_y - min_y).max(1.0)) * 500.0;
    let mut svg = format!(
        r###"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0}" height="{height:.0}" viewBox="0 0 {width:.0} {height:.0}" role="img" aria-label="Ridge canary controller shadow"><rect width="100%" height="100%" fill="#10151d"/><text x="48" y="30" fill="#edf2f7" font-family="sans-serif" font-size="20">Ridge canary controller shadow</text><text x="48" y="52" fill="#9aa8b6" font-family="sans-serif" font-size="12">gray terrain · dashed analytical witnesses · solid simulated lanes</text>"###
    );
    if let Some(points) = data
        .get("lanes")
        .and_then(Value::as_array)
        .and_then(|lanes| lanes.get(1))
        .and_then(|lane| lane.get("scenario"))
        .and_then(|scenario| scenario.get("world"))
        .and_then(|world| world.get("terrain"))
        .and_then(|terrain| terrain.get("points_m"))
        .and_then(Value::as_array)
    {
        let coords = points
            .iter()
            .filter_map(|point| Some((point.get("x")?.as_f64()?, point.get("y")?.as_f64()?)))
            .map(|(x, y)| format!("{:.2},{:.2}", sx(x), sy(y)))
            .collect::<Vec<_>>()
            .join(" ");
        let _ = write!(
            svg,
            "<polyline points=\"{coords}\" fill=\"none\" stroke=\"#9aa8b6\" stroke-width=\"2\"/>"
        );
    }
    let analytical_paths = [
        ("nominal_direct", "#ffc857", "analytical nominal direct"),
        (
            "waypoint_source_bridge",
            "#3aa7ff",
            "analytical waypoint source bridge",
        ),
        (
            "waypoint_source_leg",
            "#66d9ef",
            "analytical waypoint source leg",
        ),
        (
            "waypoint_intermediate_bridge",
            "#8ca6ff",
            "analytical waypoint intermediate bridge",
        ),
        (
            "waypoint_target_leg",
            "#d291ff",
            "analytical waypoint target leg",
        ),
        (
            "waypoint_terminal_bridge",
            "#e27dff",
            "analytical waypoint terminal bridge",
        ),
    ];
    for (path_id, color, _) in analytical_paths {
        let points = data
            .get("analytical_overlay")
            .and_then(|overlay| overlay.get(path_id))
            .and_then(|path| path.get("points_m"))
            .and_then(Value::as_array)
            .map(|points| {
                points
                    .iter()
                    .filter_map(|point| {
                        Some(format!(
                            "{:.2},{:.2}",
                            sx(point.get("x")?.as_f64()?),
                            sy(point.get("y")?.as_f64()?)
                        ))
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        if !points.is_empty() {
            let _ = write!(
                svg,
                "<polyline points=\"{points}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2.6\" stroke-dasharray=\"9 6\" opacity=\"0.9\"/>"
            );
        }
    }
    let colors = ["#4ea1ff", "#ef6262", "#5ed39b"];
    if let Some(lanes) = data.get("lanes").and_then(Value::as_array) {
        for (index, lane) in lanes.iter().enumerate() {
            let samples = lane
                .get("run")
                .and_then(|run| run.get("run"))
                .and_then(|run| run.get("samples"))
                .and_then(Value::as_array);
            let color = colors.get(index).unwrap_or(&"#ffffff");
            if let Some(samples) = samples {
                let points = samples
                    .iter()
                    .filter_map(|sample| {
                        let position = sample.get("observation")?.get("position_m")?;
                        Some(format!(
                            "{:.2},{:.2}",
                            sx(position.get("x")?.as_f64()?),
                            sy(position.get("y")?.as_f64()?)
                        ))
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                if !points.is_empty() {
                    let _ = write!(
                        svg,
                        "<polyline points=\"{points}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2.2\"/>"
                    );
                }
                if let Some(start) = samples.first().and_then(sample_position) {
                    let _ = write!(
                        svg,
                        "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"5\" fill=\"{color}\" stroke=\"#edf2f7\" stroke-width=\"1\"/>",
                        sx(start.0),
                        sy(start.1)
                    );
                }
                if let Some(end) = samples.last().and_then(sample_position) {
                    let _ = write!(
                        svg,
                        "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"8\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2\"/>",
                        sx(end.0),
                        sy(end.1)
                    );
                    if text(lane, "end_reason") == "crash" {
                        let _ = write!(
                            svg,
                            "<text x=\"{:.2}\" y=\"{:.2}\" fill=\"{color}\" font-family=\"sans-serif\" font-size=\"11\">crash</text>",
                            sx(end.0) + 9.0,
                            sy(end.1) - 8.0 - index as f64 * 13.0
                        );
                    }
                }
            }
            if let Some(point) = lane
                .get("terrain_contact")
                .and_then(|contact| contact.get("contact_point_m"))
                .and_then(|point| Some((point.get("x")?.as_f64()?, point.get("y")?.as_f64()?)))
            {
                let kind = text(
                    lane.get("terrain_contact").unwrap_or(&Value::Null),
                    "contact_kind",
                )
                .replace('_', " ");
                let _ = write!(
                    svg,
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"6\" fill=\"#ffd166\" stroke=\"#10151d\" stroke-width=\"2\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#ffd166\" font-family=\"sans-serif\" font-size=\"11\">contact: {}</text>",
                    sx(point.0),
                    sy(point.1),
                    sx(point.0) + 9.0,
                    sy(point.1) - 8.0,
                    escape(&kind)
                );
            }
        }
    }
    if let Some(point) = data
        .get("route_adapter")
        .and_then(|adapter| adapter.get("analytical_waypoint_position_m"))
        && let (Some(x), Some(y)) = (
            point.get("x").and_then(Value::as_f64),
            point.get("y").and_then(Value::as_f64),
        )
    {
        let _ = write!(
            svg,
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"5\" fill=\"#ffc857\"/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#ffc857\" font-family=\"sans-serif\" font-size=\"12\">analytical anchor</text>",
            sx(x),
            sy(y),
            sx(x) + 8.0,
            sy(y) - 8.0
        );
    }
    let legend = [
        ("#9aa8b6", "", "derived mesa terrain"),
        ("#ffc857", "9 6", "analytical nominal direct"),
        ("#3aa7ff", "9 6", "analytical waypoint source bridge"),
        ("#66d9ef", "9 6", "analytical waypoint source leg"),
        ("#8ca6ff", "9 6", "analytical waypoint intermediate bridge"),
        ("#d291ff", "9 6", "analytical waypoint target leg"),
        ("#e27dff", "9 6", "analytical waypoint terminal bridge"),
        ("#4ea1ff", "", "sim flat direct"),
        ("#ef6262", "", "sim mesa direct"),
        ("#5ed39b", "", "sim mesa waypoint (if mapped)"),
        ("#ffd166", "", "reconstructed terrain contact"),
    ];
    let _ = write!(
        svg,
        "<rect x=\"744\" y=\"64\" width=\"350\" height=\"256\" rx=\"6\" fill=\"#10151d\" fill-opacity=\"0.88\"/>"
    );
    for (index, (color, dash, label)) in legend.into_iter().enumerate() {
        let x = 760.0;
        let y = 78.0 + index as f64 * 17.0;
        let dash_attr = if dash.is_empty() {
            String::new()
        } else {
            format!(" stroke-dasharray=\"{dash}\"")
        };
        let _ = write!(
            svg,
            "<line x1=\"{x:.2}\" y1=\"{y:.2}\" x2=\"{:.2}\" y2=\"{y:.2}\" stroke=\"{color}\" stroke-width=\"2.8\"{dash_attr}/><text x=\"{:.2}\" y=\"{:.2}\" fill=\"#d6e0ea\" font-family=\"sans-serif\" font-size=\"11\">{label}</text>",
            x + 22.0,
            x + 30.0,
            y + 4.0
        );
    }
    let _ = write!(
        svg,
        "<circle cx=\"760\" cy=\"270\" r=\"5\" fill=\"#d6e0ea\"/><text x=\"770\" y=\"274\" fill=\"#d6e0ea\" font-family=\"sans-serif\" font-size=\"11\">sim start sample</text><circle cx=\"760\" cy=\"287\" r=\"8\" fill=\"none\" stroke=\"#d6e0ea\" stroke-width=\"2\"/><text x=\"775\" y=\"291\" fill=\"#d6e0ea\" font-family=\"sans-serif\" font-size=\"11\">sim final / crash sample</text>"
    );
    svg.push_str("</svg>");
    svg
}

fn bounds(data: &Value) -> (f64, f64, f64, f64) {
    let mut points = Vec::new();
    if let Some(lanes) = data.get("lanes").and_then(Value::as_array) {
        for lane in lanes {
            if let Some(samples) = lane
                .get("run")
                .and_then(|run| run.get("run"))
                .and_then(|run| run.get("samples"))
                .and_then(Value::as_array)
            {
                for sample in samples {
                    if let Some(point) = sample
                        .get("observation")
                        .and_then(|observation| observation.get("position_m"))
                        && let (Some(x), Some(y)) = (
                            point.get("x").and_then(Value::as_f64),
                            point.get("y").and_then(Value::as_f64),
                        )
                    {
                        points.push((x, y));
                    }
                }
            }
            if let Some(terrain) = lane
                .get("scenario")
                .and_then(|scenario| scenario.get("world"))
                .and_then(|world| world.get("terrain"))
                .and_then(|terrain| terrain.get("points_m"))
                .and_then(Value::as_array)
            {
                for point in terrain {
                    if let (Some(x), Some(y)) = (
                        point.get("x").and_then(Value::as_f64),
                        point.get("y").and_then(Value::as_f64),
                    ) {
                        points.push((x, y));
                    }
                }
            }
            if let Some(point) = lane
                .get("terrain_contact")
                .and_then(|contact| contact.get("contact_point_m"))
                && let (Some(x), Some(y)) = (
                    point.get("x").and_then(Value::as_f64),
                    point.get("y").and_then(Value::as_f64),
                )
            {
                points.push((x, y));
            }
        }
    }
    for path_id in [
        "nominal_direct",
        "waypoint_source_bridge",
        "waypoint_source_leg",
        "waypoint_intermediate_bridge",
        "waypoint_target_leg",
        "waypoint_terminal_bridge",
    ] {
        if let Some(overlay_points) = data
            .get("analytical_overlay")
            .and_then(|overlay| overlay.get(path_id))
            .and_then(|path| path.get("points_m"))
            .and_then(Value::as_array)
        {
            for point in overlay_points {
                if let (Some(x), Some(y)) = (
                    point.get("x").and_then(Value::as_f64),
                    point.get("y").and_then(Value::as_f64),
                ) {
                    points.push((x, y));
                }
            }
        }
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) = points.iter().fold(
        (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        |acc, (x, y)| (acc.0.min(*x), acc.1.max(*x), acc.2.min(*y), acc.3.max(*y)),
    );
    if !min_x.is_finite() {
        min_x = 0.0;
        max_x = 4000.0;
        min_y = 0.0;
        max_y = 1400.0;
    }
    let y_pad = ((max_y - min_y) * 0.08).max(20.0);
    (min_x, max_x, min_y - y_pad, max_y + y_pad)
}

fn sample_position(sample: &Value) -> Option<(f64, f64)> {
    let point = sample.get("observation")?.get("position_m")?;
    Some((point.get("x")?.as_f64()?, point.get("y")?.as_f64()?))
}

fn text(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::Null) => "unavailable".to_owned(),
        Some(value) => value
            .as_str()
            .map_or_else(|| value.to_string(), ToOwned::to_owned),
        None => String::new(),
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_json(value: &str) -> String {
    value.replace("</", "<\\/")
}

const CSS: &str = r#"
body{margin:0;background:#0e131a;color:#edf2f7;font-family:system-ui,sans-serif}.panel,header{max-width:1100px;margin:24px auto;padding:20px;background:#151c25;border:1px solid #2a3642;border-radius:12px}header{background:linear-gradient(135deg,#182434,#121820)}h1{margin:.2em 0 .4em;font-size:2rem}.eyebrow{color:#73b7ff;text-transform:uppercase;letter-spacing:.08em;font-size:.72rem}.lede,.muted{color:#aebbc8;line-height:1.5}.panel h2{margin-top:0}table{width:100%;border-collapse:collapse}th,td{padding:10px;border-bottom:1px solid #2a3642;text-align:left;vertical-align:top}th{color:#9aa8b6;font-size:.8rem;text-transform:uppercase}code,dd{font-family:ui-monospace,monospace;color:#bfe0ff;font-size:.85rem}.facts{display:grid;grid-template-columns:220px 1fr;gap:10px}.facts dt{color:#9aa8b6}.facts dd{margin:0;overflow-wrap:anywhere}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_emits_html_and_svg_from_projection() {
        let data = serde_json::json!({
            "lanes": [{
                "id": "flat-direct",
                "end_reason": "crash",
                "run": {"run": {"samples": [
                    {"observation": {"position_m": {"x": 18.0, "y": 5.0}}},
                    {"observation": {"position_m": {"x": 18.1, "y": 5.1}}}
                ]}},
                "terrain_contact": {
                    "vehicle_center_m": {"x": 18.1, "y": 5.1},
                    "contact_point_m": {"x": 20.0, "y": 3.0},
                    "contact_kind": "hull_vertex",
                    "contact_residual_m": -1.2,
                    "contact_terrain_height_m": 4.2,
                    "reconstruction_valid": true,
                    "within_derived_mesa_span": true
                }
            }],
            "analytical_overlay": {
                "nominal_direct": {"points_m": [{"x": 18.0, "y": 5.0}, {"x": 100.0, "y": 20.0}]},
                "waypoint_source_bridge": {"points_m": [{"x": 18.0, "y": 5.0}, {"x": 30.0, "y": 12.0}]},
                "waypoint_source_leg": {"points_m": [{"x": 18.0, "y": 5.0}, {"x": 50.0, "y": 40.0}]},
                "waypoint_intermediate_bridge": {"points_m": [{"x": 50.0, "y": 40.0}, {"x": 55.0, "y": 38.0}]},
                "waypoint_target_leg": {"points_m": [{"x": 50.0, "y": 40.0}, {"x": 100.0, "y": 5.0}]},
                "waypoint_terminal_bridge": {"points_m": [{"x": 100.0, "y": 5.0}, {"x": 110.0, "y": 5.0}]}
            },
            "direct_route": {
                "route": {
                    "source_pad_id": "source",
                    "target_pad_id": "target",
                    "route_angle_deg": 0.0,
                    "route_radius_m": 3982.0,
                    "waypoints": []
                },
                "structural_validation": "validated"
            },
            "route_adapter": {
                "analytical_waypoint_position_m": {"x": 50.0, "y": 40.0},
                "bridge_mapping_note": "not replayed"
            }
        });
        let html = tempfile_path("shadow-report.html");
        let svg = tempfile_path("shadow-report.svg");
        write_controller_shadow_report(&html, &data).unwrap();
        write_controller_shadow_preview_svg(&svg, &data).unwrap();
        let html_contents = fs::read_to_string(html).unwrap();
        assert!(html_contents.contains("observed controller lanes"));
        assert!(html_contents.contains("Controller direct route"));
        assert!(html_contents.contains("source -&gt; target; angle=0.000000 deg"));
        assert!(html_contents.contains("waypoints=0; validated"));
        assert!(html_contents.contains("contact point=(20.000000,3.000000)"));
        assert!(html_contents.contains("contact terrain=4.200000; residual=-1.200000"));
        let svg_contents = fs::read_to_string(svg).unwrap();
        assert!(svg_contents.contains("<svg"));
        assert!(svg_contents.contains("stroke-dasharray=\"9 6\""));
        assert!(svg_contents.contains("analytical nominal direct"));
        assert!(svg_contents.contains("analytical waypoint source bridge"));
        assert!(svg_contents.contains("analytical waypoint intermediate bridge"));
        assert!(svg_contents.contains("analytical waypoint terminal bridge"));
        assert!(svg_contents.contains("analytical anchor"));
        assert!(svg_contents.contains("sim start sample"));
        assert!(svg_contents.contains("sim final / crash sample"));
        assert!(svg_contents.contains("contact: hull vertex"));
        assert!(svg_contents.contains(">crash</text>"));
    }

    fn tempfile_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("pd-report-{name}-{}", std::process::id()))
    }
}
