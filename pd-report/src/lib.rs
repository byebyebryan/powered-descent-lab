use std::{collections::BTreeMap, fmt::Write as _, fs, path::Path};

use anyhow::{Context, Result};
use pd_control::{ControllerSpec, ControllerUpdateRecord, RunPerformanceStats, TelemetryValue};
use pd_core::{EvaluationGoal, EventKind, EventRecord, RunManifest, SampleRecord, ScenarioSpec};
use serde::Serialize;

const PLOTLY_CDN_URL: &str = "https://cdn.plot.ly/plotly-basic-2.35.2.min.js";

pub fn write_run_report(
    path: &Path,
    scenario: &ScenarioSpec,
    controller_spec: Option<&ControllerSpec>,
    manifest: &RunManifest,
    events: &[EventRecord],
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
    performance: Option<&RunPerformanceStats>,
) -> Result<()> {
    let report_data = build_report_data(
        scenario,
        controller_spec,
        manifest,
        events,
        samples,
        controller_updates,
        performance,
    );
    let html = report_template()
        .replace(
            "__REPORT_TITLE__",
            &escape_html(&format!("{} report", scenario.name)),
        )
        .replace("__PLOTLY_HREF__", PLOTLY_CDN_URL)
        .replace("__REPORT_DATA__", &json_html(&report_data));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create report output directory {}",
                parent.display()
            )
        })?;
    }
    fs::write(path, html)
        .with_context(|| format!("failed to write report file {}", path.display()))?;
    Ok(())
}

pub fn write_run_preview_svg(
    path: &Path,
    scenario: &ScenarioSpec,
    manifest: &RunManifest,
    samples: &[SampleRecord],
) -> Result<()> {
    let svg = build_run_preview_svg(scenario, manifest, samples);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create preview output directory {}",
                parent.display()
            )
        })?;
    }
    fs::write(path, svg)
        .with_context(|| format!("failed to write preview file {}", path.display()))?;
    Ok(())
}

pub struct PreviewSeries<'a> {
    pub scenario: &'a ScenarioSpec,
    pub manifest: &'a RunManifest,
    pub samples: &'a [SampleRecord],
}

pub fn build_multi_run_preview_svg(series: &[PreviewSeries<'_>]) -> String {
    build_preview_svg(series)
}

fn build_report_data(
    scenario: &ScenarioSpec,
    controller_spec: Option<&ControllerSpec>,
    manifest: &RunManifest,
    events: &[EventRecord],
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
    performance: Option<&RunPerformanceStats>,
) -> ReportData {
    let report_samples = build_report_samples(samples, controller_updates);
    let report_markers = build_report_markers(&report_samples, controller_updates);
    let report_events = build_report_events(&report_samples, events);

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
        run_performance: build_run_performance(manifest, performance),
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
        samples: report_samples.clone(),
        events: report_events.clone(),
        markers: report_markers.clone(),
        event_counts: summarize_counts(
            report_events
                .iter()
                .map(|event| event.kind.clone())
                .collect::<Vec<_>>()
                .as_slice(),
        ),
        marker_counts: summarize_counts(
            report_markers
                .iter()
                .map(|marker| marker.label.clone())
                .collect::<Vec<_>>()
                .as_slice(),
        ),
        phase_summary: summarize_phases(&report_samples),
        landing_quality: build_landing_quality(manifest),
        checkpoint_quality: build_checkpoint_quality(manifest),
        flight_stats: build_flight_stats(&report_samples, manifest),
        bot_stats: build_bot_stats(manifest, controller_updates),
        mission_details: build_mission_details(scenario),
    }
}

fn build_run_preview_svg(
    scenario: &ScenarioSpec,
    manifest: &RunManifest,
    samples: &[SampleRecord],
) -> String {
    build_preview_svg(&[PreviewSeries {
        scenario,
        manifest,
        samples,
    }])
}

fn build_preview_svg(series: &[PreviewSeries<'_>]) -> String {
    const WIDTH_PX: f64 = 156.0;
    const HEIGHT_PX: f64 = 92.0;
    const PADDING_PX: f64 = 6.0;
    let multi_run = series.len() > 1;

    let pad_world = series.iter().find_map(|series| {
        series
            .scenario
            .world
            .landing_pad(series.scenario.mission.goal.target_pad_id())
            .map(|pad| (pad.center_x_m, pad.surface_y_m, pad.width_m.max(1.0)))
    });
    let normalize_center_x = pad_world
        .map(|(center_x_m, _, _)| center_x_m)
        .unwrap_or(0.0);
    let first_flip_sign = series
        .first()
        .map(|series| {
            preview_flip_sign(series.scenario.initial_state.position_m.x - normalize_center_x)
        })
        .unwrap_or(1.0);
    let transform_x = |x_world: f64, flip_sign: f64| (x_world - normalize_center_x) * flip_sign;
    let terrain = series
        .first()
        .map(|series| {
            series
                .scenario
                .world
                .terrain
                .points()
                .iter()
                .map(|point| (transform_x(point.x, first_flip_sign), point.y))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect::<Vec<_>>();
    let pad = pad_world.map(|(_, surface_y_m, width_m)| (0.0, surface_y_m, width_m));
    let trajectories = series
        .iter()
        .map(|series| {
            let flip_sign =
                preview_flip_sign(series.scenario.initial_state.position_m.x - normalize_center_x);
            let trajectory = series
                .samples
                .iter()
                .map(|sample| {
                    (
                        transform_x(sample.observation.position_m.x, flip_sign),
                        sample.observation.position_m.y,
                    )
                })
                .filter(|(x, y)| x.is_finite() && y.is_finite())
                .collect::<Vec<_>>();
            let reference = if multi_run {
                Vec::new()
            } else {
                pad.map(|(target_x_m, target_y_m, _)| {
                    idealized_reference_curve(
                        series.scenario.initial_state.position_m.x,
                        series.scenario.initial_state.position_m.y,
                        normalize_center_x + target_x_m,
                        target_y_m,
                        series.scenario.world.gravity_mps2,
                    )
                    .into_iter()
                    .map(|(x, y)| (transform_x(x, flip_sign), y))
                    .collect::<Vec<_>>()
                })
                .unwrap_or_default()
            };
            (
                trajectory,
                reference,
                enum_label(&series.manifest.mission_outcome),
            )
        })
        .collect::<Vec<_>>();

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut include_point = |x: f64, y: f64| {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    };
    for &(x, y) in terrain.iter() {
        include_point(x, y);
    }
    for (trajectory, reference, _) in &trajectories {
        for &(x, y) in trajectory.iter().chain(reference.iter()) {
            include_point(x, y);
        }
    }
    if let Some((center_x_m, surface_y_m, width_m)) = pad {
        include_point(center_x_m - (0.5 * width_m), surface_y_m);
        include_point(center_x_m + (0.5 * width_m), surface_y_m);
    }

    if !min_x.is_finite() || !max_x.is_finite() || !min_y.is_finite() || !max_y.is_finite() {
        return format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH_PX}" height="{HEIGHT_PX}" viewBox="0 0 {WIDTH_PX} {HEIGHT_PX}"><rect width="100%" height="100%" rx="10" fill="#fbf7ee"/><text x="50%" y="50%" text-anchor="middle" dominant-baseline="middle" font-family="ui-monospace, SFMono-Regular, Menlo, monospace" font-size="11" fill="#7c6d5c">no preview</text></svg>"##
        );
    }

    let x_span = (max_x - min_x).max(1.0);
    let y_span = (max_y - min_y).max(1.0);
    let x_margin = x_span * 0.06;
    let y_margin = y_span * 0.08;
    min_x -= x_margin;
    max_x += x_margin;
    min_y -= y_margin.max(2.0);
    max_y += y_margin.max(2.0);

    let x_span = (max_x - min_x).max(1.0);
    let y_span = (max_y - min_y).max(1.0);
    let scale = ((WIDTH_PX - (2.0 * PADDING_PX)) / x_span)
        .min((HEIGHT_PX - (2.0 * PADDING_PX)) / y_span)
        .max(1e-6);
    let inner_width = x_span * scale;
    let inner_height = y_span * scale;
    let x_origin = PADDING_PX + ((WIDTH_PX - (2.0 * PADDING_PX) - inner_width) * 0.5);
    let bottom_padding = PADDING_PX + ((HEIGHT_PX - (2.0 * PADDING_PX) - inner_height) * 0.5);
    let project = |x: f64, y: f64| -> (f64, f64) {
        let px = x_origin + ((x - min_x) * scale);
        let py = HEIGHT_PX - bottom_padding - ((y - min_y) * scale);
        (px, py)
    };
    let polyline_points = |points: &[(f64, f64)]| -> String {
        let mut out = String::new();
        for (index, &(x, y)) in points.iter().enumerate() {
            let (px, py) = project(x, y);
            if index > 0 {
                out.push(' ');
            }
            let _ = write!(out, "{px:.2},{py:.2}");
        }
        out
    };

    let terrain_points = polyline_points(&terrain);
    let start_markers = (!multi_run)
        .then(|| {
            trajectories
                .iter()
                .filter_map(|(trajectory, _, _)| {
                    trajectory.first().copied().map(|(x, y)| {
                        let (px, py) = project(x, y);
                        format!(
                            r##"<circle cx="{px:.2}" cy="{py:.2}" r="2.3" fill="#fffaf2" stroke="#5d5143" stroke-width="1"/>"##
                        )
                    })
                })
                .collect::<String>()
        })
        .unwrap_or_default();
    let end_markers = trajectories
        .iter()
        .enumerate()
        .filter_map(|(index, (trajectory, _, outcome))| {
            trajectory.last().copied().map(|(x, y)| {
                let (px, py) = project(x, y);
                let seed_color = preview_seed_color(index, trajectories.len());
                match outcome.as_str() {
                    "success" => format!(
                        r##"<circle cx="{px:.2}" cy="{py:.2}" r="{radius:.2}" fill="{fill}" stroke="#fffaf2" stroke-width="{stroke:.2}"/>"##,
                        radius = if multi_run { 2.6 } else { 3.4 },
                        fill = if multi_run { seed_color.as_str() } else { "#2f9e44" },
                        stroke = if multi_run { 1.0 } else { 1.4 },
                    ),
                    "failed_off_target" => format!(
                        r##"<path d="M {x1:.2} {py:.2} L {px:.2} {y1:.2} L {x2:.2} {py:.2} L {px:.2} {y2:.2} Z" fill="#d6a237" stroke="{stroke_color}" stroke-width="{stroke:.2}"/>"##,
                        x1 = px - if multi_run { 2.9 } else { 3.8 },
                        y1 = py - if multi_run { 2.9 } else { 3.8 },
                        x2 = px + if multi_run { 2.9 } else { 3.8 },
                        y2 = py + if multi_run { 2.9 } else { 3.8 },
                        stroke_color = if multi_run { seed_color.as_str() } else { "#fffaf2" },
                        stroke = if multi_run { 0.9 } else { 1.1 },
                    ),
                    _ => format!(
                        r##"<path d="M {x1:.2} {y1:.2} L {x2:.2} {y2:.2} M {x2:.2} {y1:.2} L {x1:.2} {y2:.2}" stroke="#b5542d" stroke-width="{stroke:.2}" stroke-linecap="round"/>"##,
                        x1 = px - if multi_run { 2.8 } else { 3.6 },
                        y1 = py - if multi_run { 2.8 } else { 3.6 },
                        x2 = px + if multi_run { 2.8 } else { 3.6 },
                        y2 = py + if multi_run { 2.8 } else { 3.6 },
                        stroke = if multi_run { 1.35 } else { 1.8 },
                    ),
                }
            })
        })
        .collect::<String>();
    let pad_svg = pad.map(|(center_x_m, surface_y_m, width_m)| {
        let (x1, y1) = project(center_x_m - (0.5 * width_m), surface_y_m);
        let (x2, y2) = project(center_x_m + (0.5 * width_m), surface_y_m);
        format!(
            r##"<line x1="{x1:.2}" y1="{y1:.2}" x2="{x2:.2}" y2="{y2:.2}" stroke="#2f9e44" stroke-width="3.2" stroke-linecap="round"/>"##
        )
    });
    let reference_svg = trajectories
        .iter()
        .map(|(_, reference, _)| polyline_points(reference))
        .filter(|points| !points.is_empty())
        .map(|points| {
            format!(
                r##"<polyline points="{points}" fill="none" stroke="#8c9cb1" stroke-width="{width:.1}" stroke-opacity="{opacity:.2}" stroke-dasharray="4 3" stroke-linecap="round" stroke-linejoin="round"/>"##,
                width = if multi_run { 1.0 } else { 1.3 },
                opacity = if multi_run { 0.38 } else { 1.0 },
            )
        })
        .collect::<String>();
    let trajectory_svg = trajectories
        .iter()
        .enumerate()
        .map(|(index, (trajectory, _, _))| (index, polyline_points(trajectory)))
        .filter(|(_, points)| !points.is_empty())
        .map(|(index, points)| {
            let stroke = if multi_run {
                preview_seed_color(index, trajectories.len())
            } else {
                "#1d5e7a".to_owned()
            };
            format!(
                r##"<polyline points="{points}" fill="none" stroke="{stroke}" stroke-width="{width:.1}" stroke-opacity="{opacity:.2}" stroke-linecap="round" stroke-linejoin="round"/>"##,
                stroke = stroke,
                width = if multi_run { 1.3 } else { 1.9 },
                opacity = if multi_run { 0.88 } else { 1.0 },
            )
        })
        .collect::<String>();

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH_PX}" height="{HEIGHT_PX}" viewBox="0 0 {WIDTH_PX} {HEIGHT_PX}" role="img" aria-label="run trajectory preview">
  <rect width="100%" height="100%" rx="10" fill="#fbf7ee"/>
  <rect x="0.5" y="0.5" width="{border_w:.1}" height="{border_h:.1}" rx="9.5" fill="none" stroke="#d7cdbd"/>
  {reference_svg}
  {trajectory_svg}
  {terrain_svg}
  {pad_svg}
  {start_markers}
  {end_markers}
</svg>"##,
        border_w = WIDTH_PX - 1.0,
        border_h = HEIGHT_PX - 1.0,
        reference_svg = reference_svg,
        trajectory_svg = trajectory_svg,
        terrain_svg = if terrain_points.is_empty() {
            String::new()
        } else {
            format!(
                r##"<polyline points="{terrain_points}" fill="none" stroke="#7a5d3e" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"##
            )
        },
        pad_svg = pad_svg.unwrap_or_default(),
        start_markers = start_markers,
        end_markers = end_markers,
    )
}

fn preview_seed_color(index: usize, total: usize) -> String {
    if total <= 1 {
        return "#1d5e7a".to_owned();
    }
    let t = index as f64 / (total.saturating_sub(1)) as f64;
    let hue = 210.0 - (165.0 * t);
    format!("hsl({hue:.0},74%,40%)")
}

fn preview_flip_sign(initial_dx: f64) -> f64 {
    if initial_dx > 0.0 { -1.0 } else { 1.0 }
}

fn ballistic_end_time(start_y: f64, target_y: f64, vy_mps: f64, gravity_mps2: f64) -> Option<f64> {
    let g = gravity_mps2.abs().max(1e-6);
    let a = 0.5 * g;
    let b = -vy_mps;
    let c = target_y - start_y;
    let discriminant = (b * b) - (4.0 * a * c);
    if !discriminant.is_finite() || discriminant < 0.0 {
        return None;
    }
    let sqrt = discriminant.sqrt();
    [(-b - sqrt) / (2.0 * a), (-b + sqrt) / (2.0 * a)]
        .into_iter()
        .filter(|value| value.is_finite() && *value > 1e-6)
        .max_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal))
}

fn idealized_reference_kinematics(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    apex_y: f64,
    gravity_mps2: f64,
) -> Option<(f64, f64, f64)> {
    let g = gravity_mps2.abs().max(1e-6);
    let peak_y = start_y.max(apex_y);
    let vy_up = (2.0 * g * (peak_y - start_y).max(0.0)).sqrt();
    let flight_time = ballistic_end_time(start_y, target_y, vy_up, gravity_mps2)?;
    if flight_time <= 1e-6 {
        return None;
    }
    Some((flight_time, (target_x - start_x) / flight_time, vy_up))
}

fn idealized_reference_impact_angle_deg(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    apex_y: f64,
    gravity_mps2: f64,
) -> Option<f64> {
    let (flight_time, vx_mps, vy_up_mps) =
        idealized_reference_kinematics(start_x, start_y, target_x, target_y, apex_y, gravity_mps2)?;
    let g = gravity_mps2.abs().max(1e-6);
    let vy_target = vy_up_mps - (g * flight_time);
    Some((-vy_target).max(0.0).atan2(vx_mps.abs()).to_degrees())
}

fn idealized_reference_apex_y(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    gravity_mps2: f64,
) -> f64 {
    let dx = target_x - start_x;
    let dy = target_y - start_y;
    let base_peak = if target_y > start_y {
        start_y.max(target_y + 1.0)
    } else {
        start_y
    };
    if dx.abs() <= 1e-6 {
        return base_peak;
    }
    let meets_angle_floor = |peak_y: f64| {
        idealized_reference_impact_angle_deg(
            start_x,
            start_y,
            target_x,
            target_y,
            peak_y,
            gravity_mps2,
        )
        .is_some_and(|angle| angle >= 45.0)
    };
    if meets_angle_floor(base_peak) {
        return base_peak;
    }
    let mut low_peak = base_peak;
    let mut growth = 16.0_f64.max(0.25 * dx.abs().max(dy.abs()).max(1.0));
    let mut candidate_peak = base_peak;
    let mut high_peak = None;
    for _ in 0..16 {
        candidate_peak += growth;
        if meets_angle_floor(candidate_peak) {
            high_peak = Some(candidate_peak);
            break;
        }
        low_peak = candidate_peak;
        growth *= 2.0;
    }
    let Some(mut high_peak) = high_peak else {
        return candidate_peak;
    };
    for _ in 0..32 {
        let mid_peak = 0.5 * (low_peak + high_peak);
        if meets_angle_floor(mid_peak) {
            high_peak = mid_peak;
        } else {
            low_peak = mid_peak;
        }
    }
    high_peak
}

fn idealized_reference_curve(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    gravity_mps2: f64,
) -> Vec<(f64, f64)> {
    let apex_y = idealized_reference_apex_y(start_x, start_y, target_x, target_y, gravity_mps2);
    let Some((flight_time, vx_mps, vy_up_mps)) =
        idealized_reference_kinematics(start_x, start_y, target_x, target_y, apex_y, gravity_mps2)
    else {
        return Vec::new();
    };
    let g = gravity_mps2.abs().max(1e-6);
    let point_count = (18.0 + (flight_time * 10.0)).round().clamp(24.0, 84.0) as usize;
    let mut points = Vec::with_capacity(point_count);
    for index in 0..point_count {
        let t = if point_count <= 1 {
            0.0
        } else {
            flight_time * (index as f64) / ((point_count - 1) as f64)
        };
        points.push((
            start_x + (vx_mps * t),
            start_y + (vy_up_mps * t) - (0.5 * g * t * t),
        ));
    }
    if let Some(last) = points.last_mut() {
        *last = (target_x, target_y);
    }
    points
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
            speed_mps: sample.observation.velocity_mps.length(),
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
            compute_time_ms: update_compute_time_ms(current_update),
            status,
            phase,
            metrics,
        });
    }

    report_samples
}

fn build_report_events(samples: &[ReportSample], events: &[EventRecord]) -> Vec<ReportEvent> {
    events
        .iter()
        .filter(|event| {
            !matches!(
                event.kind,
                EventKind::ControllerUpdated | EventKind::MissionEnded
            )
        })
        .map(|event| {
            let sample = nearest_sample(samples, event.physics_step);
            ReportEvent {
                sim_time_s: event.sim_time_s,
                physics_step: event.physics_step,
                kind: enum_label(&event.kind),
                label: humanize_label(&enum_label(&event.kind)),
                message: event.message.clone(),
                x_m: sample.map(|sample| sample.x_m),
                y_m: sample.map(|sample| sample.y_m),
            }
        })
        .collect()
}

fn build_report_markers(
    samples: &[ReportSample],
    controller_updates: &[ControllerUpdateRecord],
) -> Vec<ReportMarker> {
    controller_updates
        .iter()
        .flat_map(|update| {
            update.frame.markers.iter().map(move |marker| {
                let sample = nearest_sample(samples, update.physics_step);
                ReportMarker {
                    sim_time_s: update.sim_time_s,
                    physics_step: update.physics_step,
                    label: marker.label.clone(),
                    x_m: marker.x_m.or_else(|| sample.map(|sample| sample.x_m)),
                    y_m: marker.y_m.or_else(|| sample.map(|sample| sample.y_m)),
                    phase: update.frame.phase.clone(),
                    metrics: marker.metadata.clone(),
                }
            })
        })
        .collect()
}

fn summarize_counts(values: &[String]) -> Vec<ReportCount> {
    let mut counts = BTreeMap::<String, usize>::new();
    for value in values {
        *counts.entry(value.clone()).or_insert(0) += 1;
    }

    let mut summary = counts
        .into_iter()
        .map(|(label, count)| ReportCount { label, count })
        .collect::<Vec<_>>();
    summary.sort_by(|lhs, rhs| rhs.count.cmp(&lhs.count).then(lhs.label.cmp(&rhs.label)));
    summary
}

fn summarize_phases(samples: &[ReportSample]) -> Vec<ReportPhaseSummary> {
    let mut order = Vec::<String>::new();
    let mut durations = BTreeMap::<String, f64>::new();
    let mut sample_counts = BTreeMap::<String, usize>::new();

    for (index, sample) in samples.iter().enumerate() {
        let Some(phase) = sample.phase.clone() else {
            continue;
        };
        if !durations.contains_key(&phase) {
            order.push(phase.clone());
        }
        let dt_s = samples
            .get(index + 1)
            .map(|next| (next.sim_time_s - sample.sim_time_s).max(0.0))
            .unwrap_or(0.0);
        *durations.entry(phase.clone()).or_insert(0.0) += dt_s;
        *sample_counts.entry(phase).or_insert(0) += 1;
    }

    order
        .into_iter()
        .map(|phase| ReportPhaseSummary {
            label: phase.clone(),
            duration_s: *durations.get(&phase).unwrap_or(&0.0),
            sample_count: *sample_counts.get(&phase).unwrap_or(&0),
        })
        .collect()
}

fn build_landing_quality(manifest: &RunManifest) -> Option<ReportLandingQuality> {
    let landing = manifest.summary.landing.as_ref()?;
    Some(ReportLandingQuality {
        landing_offset_m: landing.touchdown_center_offset_m,
        pad_margin_m: landing.pad_margin_m,
        impact_attitude_deg: landing.attitude_error_rad.to_degrees(),
        impact_normal_speed_mps: landing.normal_speed_mps,
        impact_tangential_speed_mps: landing.tangential_speed_mps,
        impact_speed_mps: landing.normal_speed_mps.hypot(landing.tangential_speed_mps),
        angular_rate_degps: landing.angular_rate_radps.to_degrees(),
        normal_speed_margin_mps: landing.normal_speed_margin_mps,
        tangential_speed_margin_mps: landing.tangential_speed_margin_mps,
        attitude_margin_deg: landing.attitude_margin_rad.to_degrees(),
        angular_rate_margin_degps: landing.angular_rate_margin_radps.to_degrees(),
        envelope_margin_ratio: landing.envelope_margin_ratio,
        on_target: landing.on_target,
        min_touchdown_clearance_m: manifest.summary.min_touchdown_clearance_m,
        min_hull_clearance_m: manifest.summary.min_hull_clearance_m,
    })
}

fn build_checkpoint_quality(manifest: &RunManifest) -> Option<ReportCheckpointQuality> {
    let checkpoint = manifest.summary.checkpoint.as_ref()?;
    Some(ReportCheckpointQuality {
        position_error_m: checkpoint.position_error_m,
        velocity_error_mps: checkpoint.velocity_error_mps,
        attitude_error_deg: checkpoint.attitude_error_rad.to_degrees(),
        position_margin_m: checkpoint.position_margin_m,
        velocity_margin_mps: checkpoint.velocity_margin_mps,
        attitude_margin_deg: checkpoint.attitude_margin_rad.to_degrees(),
        envelope_margin_ratio: checkpoint.envelope_margin_ratio,
    })
}

fn build_flight_stats(samples: &[ReportSample], manifest: &RunManifest) -> ReportFlightStats {
    let mut path_distance_m = 0.0;
    let mut horizontal_distance_m = 0.0;
    let mut max_altitude_m = f64::NEG_INFINITY;
    let mut min_altitude_m = f64::INFINITY;

    for window in samples.windows(2) {
        let lhs = &window[0];
        let rhs = &window[1];
        path_distance_m += (rhs.x_m - lhs.x_m).hypot(rhs.y_m - lhs.y_m);
        horizontal_distance_m += (rhs.x_m - lhs.x_m).abs();
    }

    for sample in samples {
        max_altitude_m = max_altitude_m.max(sample.height_above_target_m);
        min_altitude_m = min_altitude_m.min(sample.height_above_target_m);
    }

    let net_displacement_m = match (samples.first(), samples.last()) {
        (Some(first), Some(last)) => (last.x_m - first.x_m).hypot(last.y_m - first.y_m),
        _ => 0.0,
    };
    let average_speed_mps = if manifest.sim_time_s > f64::EPSILON {
        path_distance_m / manifest.sim_time_s
    } else {
        0.0
    };

    ReportFlightStats {
        flight_time_s: manifest.sim_time_s,
        path_distance_m,
        horizontal_distance_m,
        net_displacement_m,
        average_speed_mps,
        max_speed_mps: manifest.summary.max_speed_mps,
        max_altitude_m: if max_altitude_m.is_finite() {
            max_altitude_m
        } else {
            0.0
        },
        min_altitude_m: if min_altitude_m.is_finite() {
            min_altitude_m
        } else {
            0.0
        },
        fuel_used_kg: manifest.summary.fuel_used_kg,
        fuel_remaining_kg: manifest.summary.fuel_remaining_kg,
    }
}

fn build_run_performance(
    manifest: &RunManifest,
    performance: Option<&RunPerformanceStats>,
) -> ReportRunPerformance {
    let wall_time_ms = performance.map(|stats| stats.wall_time_us as f64 / 1000.0);
    let thread_cpu_time_ms = performance
        .and_then(|stats| stats.thread_cpu_time_us)
        .map(|value| value as f64 / 1000.0);
    let cpu_time_per_tick_us = performance
        .and_then(|stats| stats.thread_cpu_time_us)
        .and_then(|value| {
            (manifest.physics_steps > 0).then(|| value as f64 / manifest.physics_steps as f64)
        });
    let sim_rate_x = wall_time_ms.and_then(|wall| {
        if wall <= f64::EPSILON {
            None
        } else {
            Some((manifest.sim_time_s * 1000.0) / wall)
        }
    });
    let physics_steps_per_s = wall_time_ms.and_then(|wall| {
        if wall <= f64::EPSILON {
            None
        } else {
            Some(manifest.physics_steps as f64 / (wall / 1000.0))
        }
    });

    ReportRunPerformance {
        wall_time_ms,
        thread_cpu_time_ms,
        cpu_time_per_tick_us,
        sim_rate_x,
        physics_steps_per_s,
    }
}

fn build_bot_stats(
    manifest: &RunManifest,
    controller_updates: &[ControllerUpdateRecord],
) -> ReportBotStats {
    let mut compute_ms = controller_updates
        .iter()
        .filter_map(|update| update.compute_time_us.map(|value| value as f64 / 1000.0))
        .collect::<Vec<_>>();
    compute_ms.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal));

    let total_compute_ms = (!compute_ms.is_empty()).then(|| compute_ms.iter().sum::<f64>());
    let mean_compute_ms = total_compute_ms.map(|total| total / compute_ms.len() as f64);
    let p95_compute_ms = percentile(&compute_ms, 0.95);
    let max_compute_ms = compute_ms.last().copied();
    let mean_control_dt_ms = (manifest.controller_updates > 1)
        .then(|| (manifest.sim_time_s * 1000.0) / manifest.controller_updates as f64);
    let control_duty_cycle_pct = total_compute_ms.map(|total| {
        if manifest.sim_time_s <= f64::EPSILON {
            0.0
        } else {
            (total / (manifest.sim_time_s * 1000.0)) * 100.0
        }
    });

    ReportBotStats {
        controller_updates: manifest.controller_updates,
        total_compute_ms,
        mean_compute_ms,
        p95_compute_ms,
        max_compute_ms,
        mean_control_dt_ms,
        control_duty_cycle_pct,
    }
}

fn build_mission_details(scenario: &ScenarioSpec) -> ReportMissionDetails {
    let target_pad = scenario
        .world
        .landing_pad(scenario.mission.goal.target_pad_id())
        .expect("validated scenario should contain mission target pad");
    ReportMissionDetails {
        description: scenario.description.clone(),
        scenario_seed: scenario.seed,
        tags: scenario.tags.clone(),
        gravity_mps2: scenario.world.gravity_mps2,
        sim: ReportSimDetails {
            physics_hz: scenario.sim.physics_hz,
            controller_hz: scenario.sim.controller_hz,
            sample_hz: scenario.sim.sample_hz,
            max_time_s: scenario.sim.max_time_s,
        },
        initial_state: ReportInitialState {
            x_m: scenario.initial_state.position_m.x,
            y_m: scenario.initial_state.position_m.y,
            vx_mps: scenario.initial_state.velocity_mps.x,
            vy_mps: scenario.initial_state.velocity_mps.y,
            speed_mps: scenario.initial_state.velocity_mps.length(),
            attitude_deg: scenario.initial_state.attitude_rad.to_degrees(),
            angular_rate_degps: scenario.initial_state.angular_rate_radps.to_degrees(),
        },
        vehicle: ReportVehicleDetails {
            hull_width_m: scenario.vehicle.geometry.hull_width_m,
            hull_height_m: scenario.vehicle.geometry.hull_height_m,
            touchdown_half_span_m: scenario.vehicle.geometry.touchdown_half_span_m,
            touchdown_base_offset_m: scenario.vehicle.geometry.touchdown_base_offset_m,
            dry_mass_kg: scenario.vehicle.dry_mass_kg,
            initial_fuel_kg: scenario.vehicle.initial_fuel_kg,
            max_fuel_kg: scenario.vehicle.max_fuel_kg,
            max_thrust_n: scenario.vehicle.max_thrust_n,
            max_fuel_burn_kgps: scenario.vehicle.max_fuel_burn_kgps,
            max_rotation_rate_degps: scenario.vehicle.max_rotation_rate_radps.to_degrees(),
            safe_touchdown_normal_speed_mps: scenario.vehicle.safe_touchdown_normal_speed_mps,
            safe_touchdown_tangential_speed_mps: scenario
                .vehicle
                .safe_touchdown_tangential_speed_mps,
            safe_touchdown_attitude_deg: scenario
                .vehicle
                .safe_touchdown_attitude_error_rad
                .to_degrees(),
            safe_touchdown_angular_rate_degps: scenario
                .vehicle
                .safe_touchdown_angular_rate_radps
                .to_degrees(),
        },
        target_pad: ReportTargetPadDetails {
            id: target_pad.id.clone(),
            center_x_m: target_pad.center_x_m,
            surface_y_m: target_pad.surface_y_m,
            width_m: target_pad.width_m,
        },
        mission: ReportMissionGoalDetails::from_goal(&scenario.mission.goal),
        terrain_point_count: scenario.world.terrain.points().len(),
        evaluation_basis: mission_evaluation_basis(&scenario.mission.goal),
    }
}

fn mission_evaluation_basis(goal: &EvaluationGoal) -> String {
    match goal {
        EvaluationGoal::LandingOnPad { .. } => "Landing success currently uses stable contact plus pad overlap, normal/tangential touchdown speed, attitude error, and angular rate. No force or impulse check is used yet.".to_owned(),
        EvaluationGoal::TimedCheckpoint { .. } => "Checkpoint success currently uses position, velocity, and attitude envelope checks at the configured end time.".to_owned(),
    }
}

fn update_compute_time_ms(update: Option<&ControllerUpdateRecord>) -> Option<f64> {
    update
        .and_then(|update| update.compute_time_us)
        .map(|value| value as f64 / 1000.0)
}

fn percentile(values: &[f64], quantile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let quantile = quantile.clamp(0.0, 1.0);
    let index = ((values.len() - 1) as f64 * quantile).round() as usize;
    values.get(index).copied()
}

fn nearest_sample<T>(samples: &[T], physics_step: u64) -> Option<&T>
where
    T: HasPhysicsStep,
{
    samples
        .iter()
        .min_by_key(|sample| sample.physics_step().abs_diff(physics_step))
}

fn enum_label<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"unknown\"".to_owned())
        .trim_matches('"')
        .to_owned()
}

fn humanize_label(label: &str) -> String {
    label
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn json_html<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .expect("report data should serialize")
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

trait HasPhysicsStep {
    fn physics_step(&self) -> u64;
}

impl HasPhysicsStep for ReportSample {
    fn physics_step(&self) -> u64 {
        self.physics_step
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportData {
    scenario_id: String,
    scenario_name: String,
    controller_id: String,
    controller_spec: Option<ControllerSpec>,
    manifest: ReportManifest,
    run_performance: ReportRunPerformance,
    terrain: Vec<ReportVec2>,
    pad: Option<ReportPad>,
    samples: Vec<ReportSample>,
    events: Vec<ReportEvent>,
    markers: Vec<ReportMarker>,
    event_counts: Vec<ReportCount>,
    marker_counts: Vec<ReportCount>,
    phase_summary: Vec<ReportPhaseSummary>,
    landing_quality: Option<ReportLandingQuality>,
    checkpoint_quality: Option<ReportCheckpointQuality>,
    flight_stats: ReportFlightStats,
    bot_stats: ReportBotStats,
    mission_details: ReportMissionDetails,
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
struct ReportRunPerformance {
    wall_time_ms: Option<f64>,
    thread_cpu_time_ms: Option<f64>,
    cpu_time_per_tick_us: Option<f64>,
    sim_rate_x: Option<f64>,
    physics_steps_per_s: Option<f64>,
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
    speed_mps: f64,
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
    compute_time_ms: Option<f64>,
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
    label: String,
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportCount {
    label: String,
    count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportPhaseSummary {
    label: String,
    duration_s: f64,
    sample_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportLandingQuality {
    landing_offset_m: f64,
    pad_margin_m: f64,
    impact_attitude_deg: f64,
    impact_normal_speed_mps: f64,
    impact_tangential_speed_mps: f64,
    impact_speed_mps: f64,
    angular_rate_degps: f64,
    normal_speed_margin_mps: f64,
    tangential_speed_margin_mps: f64,
    attitude_margin_deg: f64,
    angular_rate_margin_degps: f64,
    envelope_margin_ratio: f64,
    on_target: bool,
    min_touchdown_clearance_m: f64,
    min_hull_clearance_m: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportCheckpointQuality {
    position_error_m: f64,
    velocity_error_mps: f64,
    attitude_error_deg: f64,
    position_margin_m: f64,
    velocity_margin_mps: f64,
    attitude_margin_deg: f64,
    envelope_margin_ratio: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportFlightStats {
    flight_time_s: f64,
    path_distance_m: f64,
    horizontal_distance_m: f64,
    net_displacement_m: f64,
    average_speed_mps: f64,
    max_speed_mps: f64,
    max_altitude_m: f64,
    min_altitude_m: f64,
    fuel_used_kg: f64,
    fuel_remaining_kg: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportBotStats {
    controller_updates: u64,
    total_compute_ms: Option<f64>,
    mean_compute_ms: Option<f64>,
    p95_compute_ms: Option<f64>,
    max_compute_ms: Option<f64>,
    mean_control_dt_ms: Option<f64>,
    control_duty_cycle_pct: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportMissionDetails {
    description: String,
    scenario_seed: u64,
    tags: Vec<String>,
    gravity_mps2: f64,
    sim: ReportSimDetails,
    initial_state: ReportInitialState,
    vehicle: ReportVehicleDetails,
    target_pad: ReportTargetPadDetails,
    mission: ReportMissionGoalDetails,
    terrain_point_count: usize,
    evaluation_basis: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportSimDetails {
    physics_hz: u32,
    controller_hz: u32,
    sample_hz: Option<u32>,
    max_time_s: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportInitialState {
    x_m: f64,
    y_m: f64,
    vx_mps: f64,
    vy_mps: f64,
    speed_mps: f64,
    attitude_deg: f64,
    angular_rate_degps: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportVehicleDetails {
    hull_width_m: f64,
    hull_height_m: f64,
    touchdown_half_span_m: f64,
    touchdown_base_offset_m: f64,
    dry_mass_kg: f64,
    initial_fuel_kg: f64,
    max_fuel_kg: f64,
    max_thrust_n: f64,
    max_fuel_burn_kgps: f64,
    max_rotation_rate_degps: f64,
    safe_touchdown_normal_speed_mps: f64,
    safe_touchdown_tangential_speed_mps: f64,
    safe_touchdown_attitude_deg: f64,
    safe_touchdown_angular_rate_degps: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportTargetPadDetails {
    id: String,
    center_x_m: f64,
    surface_y_m: f64,
    width_m: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportMissionGoalDetails {
    goal_kind: String,
    target_pad_id: String,
    end_time_s: Option<f64>,
}

impl ReportMissionGoalDetails {
    fn from_goal(goal: &EvaluationGoal) -> Self {
        match goal {
            EvaluationGoal::LandingOnPad { target_pad_id } => Self {
                goal_kind: "landing_on_pad".to_owned(),
                target_pad_id: target_pad_id.clone(),
                end_time_s: None,
            },
            EvaluationGoal::TimedCheckpoint {
                target_pad_id,
                end_time_s,
                ..
            } => Self {
                goal_kind: "timed_checkpoint".to_owned(),
                target_pad_id: target_pad_id.clone(),
                end_time_s: Some(*end_time_s),
            },
        }
    }
}

fn report_template() -> &'static str {
    r####"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>__REPORT_TITLE__</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f4f1e8;
      --panel: #fffaf0;
      --ink: #1d1f24;
      --muted: #575f66;
      --accent: #0e6b60;
      --warn: #8e3b2e;
      --success: #2f9e44;
      --line: #d8cfbf;
      --shadow: rgba(29, 31, 36, 0.08);
      --chip: rgba(14, 107, 96, 0.08);
      --chip-border: rgba(14, 107, 96, 0.18);
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: Georgia, "Times New Roman", serif;
      color: var(--ink);
      background: linear-gradient(180deg, #f7f4ec 0%, var(--bg) 100%);
    }
    main {
      max-width: 1520px;
      margin: 0 auto;
      padding: 18px 16px 24px;
    }
    section, header {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 16px;
      box-shadow: 0 6px 18px var(--shadow);
    }
    header {
      padding: 14px 16px;
      margin-bottom: 12px;
    }
    h1, h2, h3 {
      margin: 0;
      font-family: "Palatino Linotype", "Book Antiqua", Palatino, serif;
    }
    p { margin: 0; }
    .hero {
      display: grid;
      grid-template-columns: 1.6fr 1fr;
      gap: 14px;
      align-items: start;
    }
    .hero-main {
      display: grid;
      gap: 10px;
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
      font-size: clamp(1.9rem, 4vw, 3rem);
      line-height: 0.98;
    }
    .banner {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 12rem;
      padding: 0.7rem 1rem;
      border-radius: 999px;
      font-weight: 700;
      text-align: center;
      background: rgba(14, 107, 96, 0.11);
      color: var(--accent);
      border: 1px solid rgba(14, 107, 96, 0.18);
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
      border-radius: 12px;
      background: rgba(255, 255, 255, 0.45);
      padding: 9px 11px;
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
    }
    .panel-head {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      gap: 10px;
      margin-bottom: 8px;
    }
    .panel-head h2 {
      font-size: 1.18rem;
    }
    .plot-toolbar {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      justify-content: flex-end;
    }
    .plot-toolbar button {
      border: 1px solid var(--line);
      background: #f7f1e4;
      color: var(--ink);
      border-radius: 999px;
      padding: 4px 10px;
      cursor: pointer;
      font: inherit;
    }
    .plot-toolbar button.active {
      background: var(--accent);
      color: #fffaf0;
      border-color: var(--accent);
    }
    .chart {
      width: 100%;
      height: 408px;
      border: 1px solid var(--line);
      border-radius: 12px;
      background: #fbf8f1;
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
    }
    .key-grid {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 10px;
    }
    .hero-main .key-grid {
      margin-top: 2px;
    }
    .key-grid .stat {
      min-width: 0;
      padding: 8px 10px;
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
      border-bottom: 1px solid rgba(216, 207, 191, 0.75);
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
      background: rgba(255, 255, 255, 0.58);
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
      background: rgba(255, 255, 255, 0.58);
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
      background: rgba(255, 255, 255, 0.52);
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
      .key-grid {
        grid-template-columns: 1fr;
      }
    }
  </style>
  <script src="__PLOTLY_HREF__"></script>
</head>
<body>
  <main>
    <header>
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
              <div class="label">Controller</div>
              <div class="value" id="controller-id"></div>
            </div>
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
          </div>
          <div class="key-grid">
            <div class="stat">
              <div class="label">Fuel Used</div>
              <div class="value" id="key-fuel-used"></div>
              <div class="meta" id="key-fuel-meta"></div>
            </div>
            <div class="stat">
              <div class="label">Flight Time</div>
              <div class="value" id="key-flight-time"></div>
              <div class="meta" id="key-flight-meta"></div>
            </div>
            <div class="stat">
              <div class="label" id="key-quality-label"></div>
              <div class="value" id="key-quality-value"></div>
              <div class="meta" id="key-quality-meta"></div>
            </div>
            <div class="stat">
              <div class="label">Bot Step</div>
              <div class="value" id="key-bot-step"></div>
              <div class="meta" id="key-bot-meta"></div>
            </div>
          </div>
        </div>
        <div class="summary-grid">
          <div class="panel-block">
            <div class="eyebrow">Run</div>
            <div class="stats">
              <div class="stat">
                <div class="label">Sim Time</div>
                <div class="value" id="sim-time"></div>
              </div>
              <div class="stat">
                <div class="label">Physics Steps</div>
                <div class="value" id="physics-steps"></div>
              </div>
              <div class="stat">
                <div class="label">Control Updates</div>
                <div class="value" id="controller-updates"></div>
              </div>
              <div class="stat">
                <div class="label">Wall Time</div>
                <div class="value" id="wall-time"></div>
              </div>
              <div class="stat">
                <div class="label">CPU Time</div>
                <div class="value" id="cpu-time"></div>
              </div>
              <div class="stat">
                <div class="label">CPU / Tick</div>
                <div class="value" id="cpu-per-tick"></div>
              </div>
            </div>
          </div>
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
              <button type="button" data-mode="plain" class="active">Plain</button>
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
                <h2>Velocity And Thrust Components</h2>
              </div>
            </div>
            <div id="chart-metrics" class="chart"></div>
          </section>
        </section>
      </div>

      <div class="right-stack">
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
    const paperBg = "#fffaf0";
    const plotBg = "#fbf8f1";
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
    const pad = reportData.pad || null;

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
      [0.0, "#fff3bf"],
      [0.35, "#ffd166"],
      [0.65, "#f77f00"],
      [1.0, "#c1121f"],
    ];
    const throttleColorScale = [
      [0.0, "#d9f0ff"],
      [0.35, "#74c0fc"],
      [0.7, "#2b8aeb"],
      [1.0, "#0b3d91"],
    ];

    const axisStyle = (extra = {}) => Object.assign({
      automargin: true,
      gridcolor: "rgba(216, 207, 191, 0.5)",
      zerolinecolor: "rgba(108, 97, 77, 0.45)",
      linecolor: "rgba(108, 97, 77, 0.35)",
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
        bgcolor: "rgba(255,250,240,0.92)",
        font: { size: 11 },
      },
      font: {
        family: "Georgia, Times New Roman, serif",
        size: 12,
        color: "#1d1f24",
      },
      hoverlabel: {
        bgcolor: "#fffaf0",
        bordercolor: "#d8cfbf",
        font: { family: "Georgia, serif", size: 12, color: "#1d1f24" },
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
        bgcolor: "rgba(255,250,240,0.92)",
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
        bgcolor: "rgba(255,250,240,0.92)",
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
      setText("scenario-title", reportData.scenarioName);
      setText(
        "scenario-subtitle",
        `${reportData.scenarioId} · ${markers.length} controller markers`
      );
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
        line: { color: "#d6cdbd", width: 2 },
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
            line: { color: eventStyle(event.kind).color, width: 1.2, dash: "dot" },
          };
        })
        .filter(Boolean);

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
      const markerTrace = {
        type: "scatter",
        mode: "markers",
        name: "controller markers",
        x: markers.map((marker) => Number(marker.xM)),
        y: markers.map((marker) => Number(marker.yM)),
        customdata: markers.map((marker) => [marker.label, Number(marker.simTimeS), marker.phase || ""]),
        hovertemplate: "%{customdata[0]}<br>phase=%{customdata[2]}<br>t=%{customdata[1]:.2f}s<extra></extra>",
        marker: {
          size: 10,
          color: "#4c6ef5",
          symbol: "diamond",
          line: { width: 1.5, color: "#fffaf0" },
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
        line: { color: "#cf7b00", width: 2.2, dash: "dot" },
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
        line: { color: "#5b73c6", width: 2.4, dash: "dash" },
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
      const vectorAnnotations = buildVectorAnnotations();
      const spatialTraces = [
        terrainTrace,
        ...(padTrace ? [padTrace] : []),
        plainTrace,
        ...speedTraces,
        ...throttleTraces,
        ...(ballisticTrace ? [ballisticTrace] : []),
        ...(referenceTrace ? [referenceTrace] : []),
        markerTrace,
        eventTrace,
        hoverTrace,
      ];
      const plainIndex = padTrace ? 2 : 1;
      const speedStart = plainIndex + 1;
      const speedEnd = speedStart + speedTraces.length;
      const throttleStart = speedEnd;
      const throttleEnd = throttleStart + throttleTraces.length;
      const markerIndex = spatialTraces.length - 3;
      const eventIndex = spatialTraces.length - 2;
      const hoverIndex = spatialTraces.length - 1;
      const alwaysVisible = new Set([0, eventIndex, hoverIndex]);
      if (padTrace) alwaysVisible.add(1);
      if (ballisticTrace) alwaysVisible.add(throttleEnd);
      if (referenceTrace) alwaysVisible.add(throttleEnd + (ballisticTrace ? 1 : 0));

      const visibilityForMode = (mode) =>
        spatialTraces.map((_trace, index) => {
          if (index === markerIndex) return mode !== "vectors";
          if (alwaysVisible.has(index)) return true;
          if (index === plainIndex) return true;
          if (mode === "speed") return index >= speedStart && index < speedEnd;
          if (mode === "throttle") return index >= throttleStart && index < throttleEnd;
          return false;
        });

      const spatialElement = document.getElementById("chart-spatial");
      Plotly.newPlot(
        spatialElement,
        spatialTraces.map((trace, index) => ({ ...trace, visible: visibilityForMode("plain")[index] })),
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
        button.addEventListener("click", () => applyMode(button.dataset.mode || "plain"));
      }

      spatialElement.on("plotly_hover", (eventData) => {
        const points = Array.isArray(eventData?.points) ? eventData.points : [];
        const hoverPoint = points.find((point) => point.curveNumber === hoverIndex);
        if (!hoverPoint || !Array.isArray(hoverPoint.customdata)) return;
        updateInspect(Number(hoverPoint.customdata[0]));
      });
    };

    const buildMetricsPlot = () => {
      const eventGuideShapes = buildEventGuideShapes();
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
          shapes: eventGuideShapes,
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
      renderMissionProfile();
      buildSpatialPlot();
      buildMetricsPlot();
      updateInspect(samples.length ? samples.length - 1 : 0);
    };

    init();
  </script>
</body>
</html>
"####
}
