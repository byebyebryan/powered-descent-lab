use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use pd_control::{
    ControllerSpec, ControllerUpdateRecord, RunPerformanceStats, TelemetryValue, metric,
};
use pd_core::{
    EvaluationGoal, EventKind, EventRecord, PlannerComputeEvidence, RoutePlan, RunManifest,
    SampleRecord, ScenarioSpec, Vec2,
};
use serde::Serialize;

pub mod batch;
pub mod batch_tree;
pub mod flight_annotations;
pub mod report_navigation;
mod rich_template;
pub mod site;
pub mod waypoint_v2;

use rich_template::{PLANNER_PANEL_HTML, report_template};

const PLOTLY_CDN_URL: &str = "https://cdn.plot.ly/plotly-basic-2.35.2.min.js";

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReportContext {
    pub parent_report_href: Option<String>,
    pub parent_report_label: Option<String>,
    pub run_index_href: Option<String>,
}

#[allow(clippy::too_many_arguments)]
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
    write_run_report_with_context(
        path,
        scenario,
        controller_spec,
        manifest,
        events,
        samples,
        controller_updates,
        performance,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn write_run_report_with_context(
    path: &Path,
    scenario: &ScenarioSpec,
    controller_spec: Option<&ControllerSpec>,
    manifest: &RunManifest,
    events: &[EventRecord],
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
    performance: Option<&RunPerformanceStats>,
    context: Option<&RunReportContext>,
) -> Result<()> {
    write_run_report_with_plan_context(
        path,
        scenario,
        controller_spec,
        manifest,
        events,
        samples,
        controller_updates,
        performance,
        context,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn write_run_report_with_plan_context(
    path: &Path,
    scenario: &ScenarioSpec,
    controller_spec: Option<&ControllerSpec>,
    manifest: &RunManifest,
    events: &[EventRecord],
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
    performance: Option<&RunPerformanceStats>,
    context: Option<&RunReportContext>,
    route_plan: Option<&RoutePlan>,
) -> Result<()> {
    write_run_report_with_plan_context_and_compute(
        path,
        scenario,
        controller_spec,
        manifest,
        events,
        samples,
        controller_updates,
        performance,
        context,
        route_plan,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn write_run_report_with_plan_context_and_compute(
    path: &Path,
    scenario: &ScenarioSpec,
    controller_spec: Option<&ControllerSpec>,
    manifest: &RunManifest,
    events: &[EventRecord],
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
    performance: Option<&RunPerformanceStats>,
    context: Option<&RunReportContext>,
    route_plan: Option<&RoutePlan>,
    planner_compute: Option<&PlannerComputeEvidence>,
) -> Result<()> {
    let html = render_run_report_with_flight_annotations(
        scenario,
        controller_spec,
        manifest,
        events,
        samples,
        controller_updates,
        performance,
        context,
        route_plan,
        planner_compute,
        None,
    )?;
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

/// Render the original rich run report, optionally with executed-flight annotations.
/// Callers with create-only evidence roots own persistence; this function never writes.
#[allow(clippy::too_many_arguments)]
pub fn render_run_report_with_flight_annotations(
    scenario: &ScenarioSpec,
    controller_spec: Option<&ControllerSpec>,
    manifest: &RunManifest,
    events: &[EventRecord],
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
    performance: Option<&RunPerformanceStats>,
    context: Option<&RunReportContext>,
    route_plan: Option<&RoutePlan>,
    planner_compute: Option<&PlannerComputeEvidence>,
    annotations: Option<&flight_annotations::FlightAnnotations>,
) -> Result<String> {
    let annotations = annotations.filter(|value| !value.is_empty());
    if let Some(annotations) = annotations {
        annotations.validate(manifest)?;
    }
    let mut report_data = build_report_data(
        scenario,
        controller_spec,
        manifest,
        events,
        samples,
        controller_updates,
        performance,
        context,
        route_plan,
        planner_compute,
    );
    report_data.flight_annotations = annotations.cloned();
    let display_title = friendly_report_title(scenario);
    let planner_panel = if route_plan.is_some() {
        PLANNER_PANEL_HTML
    } else {
        ""
    };
    let html = report_template()
        .replace(
            "__REPORT_TITLE__",
            &escape_html(&format!("{display_title} report")),
        )
        .replace("__PLOTLY_HREF__", PLOTLY_CDN_URL)
        .replace("__PLANNER_PANEL__", planner_panel)
        .replace(
            "__FLIGHT_ANNOTATIONS_CSS__",
            if annotations.is_some() {
                flight_annotations::CSS
            } else {
                ""
            },
        )
        .replace(
            "__FLIGHT_ANNOTATIONS_BANNER__",
            &annotations.map(|a| a.banner_html()).unwrap_or_default(),
        )
        .replace(
            "__FLIGHT_ANNOTATIONS_PANEL__",
            &annotations.map(|a| a.panel_html()).unwrap_or_default(),
        )
        .replace("__REPORT_DATA__", &json_html(&report_data));
    Ok(html)
}

pub fn write_run_preview_svg(
    path: &Path,
    scenario: &ScenarioSpec,
    manifest: &RunManifest,
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
) -> Result<()> {
    write_run_preview_svg_with_plan(path, scenario, manifest, samples, controller_updates, None)
}

pub fn write_run_preview_svg_with_plan(
    path: &Path,
    scenario: &ScenarioSpec,
    manifest: &RunManifest,
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
    route_plan: Option<&RoutePlan>,
) -> Result<()> {
    let svg = build_run_preview_svg_with_plan(
        scenario,
        manifest,
        samples,
        controller_updates,
        route_plan,
    );
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

pub struct AggregatePreviewSeries<'a> {
    pub scenario: &'a ScenarioSpec,
    pub manifest: &'a RunManifest,
    pub trajectory_positions_m: &'a [Vec2],
}

#[derive(Clone, Copy)]
enum PreviewTrajectory<'a> {
    Samples(&'a [SampleRecord]),
    Positions(&'a [Vec2]),
}

struct PreviewRenderSeries<'a> {
    scenario: &'a ScenarioSpec,
    manifest: Option<&'a RunManifest>,
    trajectory: PreviewTrajectory<'a>,
    controller_updates: Option<&'a [ControllerUpdateRecord]>,
    route_plan: Option<&'a RoutePlan>,
}

#[derive(Clone, Copy)]
struct PreviewOptions {
    show_context: bool,
    show_waypoints: bool,
    show_waypoint_guides: bool,
    show_reference: bool,
    show_endpoint_markers: bool,
}

impl PreviewOptions {
    const fn full() -> Self {
        Self {
            show_context: true,
            show_waypoints: true,
            show_waypoint_guides: true,
            show_reference: true,
            show_endpoint_markers: true,
        }
    }

    const fn aggregate_lane() -> Self {
        Self {
            show_context: true,
            show_waypoints: true,
            show_waypoint_guides: false,
            show_reference: false,
            show_endpoint_markers: true,
        }
    }

    const fn saved_flight() -> Self {
        Self {
            show_context: true,
            show_waypoints: false,
            show_waypoint_guides: false,
            show_reference: false,
            show_endpoint_markers: true,
        }
    }
}

struct PreviewWaypoint {
    id: String,
    x_m: f64,
    y_m: f64,
    capture_radius_m: f64,
    max_cross_track_m: f64,
    min_outbound_progress_mps: f64,
    max_outbound_cross_speed_mps: Option<f64>,
    min_speed_mps: f64,
    max_speed_mps: f64,
    min_vertical_speed_mps: Option<f64>,
    max_vertical_speed_mps: Option<f64>,
}

pub fn build_multi_run_trajectory_preview_svg(series: &[AggregatePreviewSeries<'_>]) -> String {
    let render_series = series
        .iter()
        .map(|series| PreviewRenderSeries {
            scenario: series.scenario,
            manifest: Some(series.manifest),
            trajectory: PreviewTrajectory::Positions(series.trajectory_positions_m),
            controller_updates: None,
            route_plan: None,
        })
        .collect::<Vec<_>>();
    build_preview_svg(&render_series, PreviewOptions::aggregate_lane())
}

/// Render a saved flight using actual sampled positions and executed handoffs.
///
/// The optional manifest controls outcome markers only. When it is absent, the
/// SVG still shows the scenario terrain and target pad without inventing a
/// departure, trajectory, or outcome.
pub fn build_saved_flight_preview_svg(
    scenario: &ScenarioSpec,
    manifest: Option<&RunManifest>,
    samples: &[SampleRecord],
    handoffs: &[Vec2],
) -> String {
    let series = [PreviewRenderSeries {
        scenario,
        manifest,
        trajectory: PreviewTrajectory::Samples(samples),
        controller_updates: None,
        route_plan: None,
    }];
    build_preview_svg_with_handoffs(&series, PreviewOptions::saved_flight(), handoffs)
}

#[allow(clippy::too_many_arguments)]
fn build_report_data(
    scenario: &ScenarioSpec,
    controller_spec: Option<&ControllerSpec>,
    manifest: &RunManifest,
    events: &[EventRecord],
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
    performance: Option<&RunPerformanceStats>,
    context: Option<&RunReportContext>,
    route_plan: Option<&RoutePlan>,
    planner_compute: Option<&PlannerComputeEvidence>,
) -> ReportData {
    let report_samples = build_report_samples(samples, controller_updates);
    let report_markers = build_report_markers(&report_samples, controller_updates);
    let report_events = build_report_events(&report_samples, events);
    let event_counts = summarize_counts(
        report_events
            .iter()
            .map(|event| event.kind.clone())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    let marker_counts = summarize_counts(
        report_markers
            .iter()
            .map(|marker| marker.label.clone())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    let phase_summary = summarize_phases(&report_samples);
    let flight_stats = build_flight_stats(&report_samples, manifest);

    ReportData {
        display_title: friendly_report_title(scenario),
        display_subtitle: friendly_report_subtitle(scenario),
        report_context: context.cloned().unwrap_or_default(),
        flight_annotations: None,
        route_plan: route_plan.cloned(),
        planner_compute: planner_compute.map(|timing| ReportPlannerCompute {
            wall_time_us: timing.wall_time_us,
        }),
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
        samples: report_samples,
        events: report_events,
        markers: report_markers,
        event_counts,
        marker_counts,
        phase_summary,
        landing_quality: build_landing_quality(manifest),
        checkpoint_quality: build_checkpoint_quality(manifest),
        flight_stats,
        bot_stats: build_bot_stats(manifest, controller_updates),
        mission_details: build_mission_details(scenario),
    }
}

fn friendly_report_title(scenario: &ScenarioSpec) -> String {
    let metadata = &scenario.metadata;
    let mission = metadata.get("mission").map(String::as_str).unwrap_or("");
    let vehicle = metadata
        .get("vehicle_variant")
        .map(String::as_str)
        .unwrap_or("vehicle");
    match mission {
        "terminal_guidance" => {
            let arc = metadata
                .get("arc_point")
                .map(String::as_str)
                .unwrap_or("arrival");
            let band = metadata
                .get("velocity_band")
                .map(String::as_str)
                .unwrap_or("energy");
            format!(
                "Terminal arrival · {} / {} / {}",
                friendly_selector(arc),
                friendly_selector(band),
                friendly_selector(vehicle)
            )
        }
        "transfer_guidance" => {
            let route = metadata
                .get("route_angle")
                .or_else(|| metadata.get("arc_point"))
                .map(String::as_str)
                .unwrap_or("route");
            let radius = metadata
                .get("radius_tier")
                .or_else(|| metadata.get("velocity_band"))
                .map(String::as_str)
                .unwrap_or("radius");
            let profile = metadata
                .get("waypoint_profile")
                .map(String::as_str)
                .filter(|profile| !profile.is_empty());
            let responsibility = match profile {
                Some(profile) if profile.starts_with("double_") => "Waypoint sequence",
                Some("direct") | None => "Direct transfer",
                Some(_) => "Waypoint turn",
            };
            let profile = profile
                .filter(|profile| *profile != "direct")
                .map(|value| format!("{} / ", friendly_waypoint_profile(value)))
                .unwrap_or_default();
            format!(
                "{responsibility} · {profile}{} / {} / {}",
                friendly_selector(route),
                friendly_selector(radius),
                friendly_selector(vehicle)
            )
        }
        _ => scenario.name.clone(),
    }
}

fn friendly_report_subtitle(scenario: &ScenarioSpec) -> String {
    let mut selectors = [
        scenario.metadata.get("condition_set"),
        scenario.metadata.get("arrival_family"),
        scenario.metadata.get("waypoint_handoff_envelope"),
    ]
    .into_iter()
    .flatten()
    .filter(|value| !value.is_empty())
    .cloned()
    .collect::<Vec<_>>();
    selectors.dedup();
    if selectors.is_empty() {
        scenario.id.clone()
    } else {
        format!("{} · {}", scenario.id, selectors.join(" · "))
    }
}

fn friendly_selector(value: &str) -> String {
    let mut chars = value.replace('_', " ").chars().collect::<Vec<_>>();
    if let Some(first) = chars.first_mut() {
        first.make_ascii_uppercase();
    }
    chars.into_iter().collect()
}

fn friendly_waypoint_profile(value: &str) -> String {
    let profile = value
        .strip_prefix("single_")
        .or_else(|| value.strip_prefix("double_"))
        .unwrap_or(value);
    profile
        .strip_suffix("_v1")
        .unwrap_or(profile)
        .replace('_', " ")
}

fn build_run_preview_svg_with_plan(
    scenario: &ScenarioSpec,
    manifest: &RunManifest,
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
    route_plan: Option<&RoutePlan>,
) -> String {
    build_preview_svg(
        &[PreviewRenderSeries {
            scenario,
            manifest: Some(manifest),
            trajectory: PreviewTrajectory::Samples(samples),
            controller_updates: Some(controller_updates),
            route_plan,
        }],
        PreviewOptions::full(),
    )
}

fn build_preview_svg(series: &[PreviewRenderSeries<'_>], options: PreviewOptions) -> String {
    build_preview_svg_with_handoffs(series, options, &[])
}

fn build_preview_svg_with_handoffs(
    series: &[PreviewRenderSeries<'_>],
    options: PreviewOptions,
    handoffs: &[Vec2],
) -> String {
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
    let terrain = if options.show_context {
        series
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
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let pad = pad_world.map(|(_, surface_y_m, width_m)| (0.0, surface_y_m, width_m));
    let mut waypoint_keys = BTreeSet::new();
    let mut waypoints = Vec::new();
    let mut route_guides = Vec::new();
    if options.show_waypoints || options.show_waypoint_guides {
        for series in series {
            let Some(route) = series.scenario.mission.transfer_route.as_ref() else {
                continue;
            };
            if route.waypoints.is_empty() {
                continue;
            }
            let flip_sign =
                preview_flip_sign(series.scenario.initial_state.position_m.x - normalize_center_x);
            let mut route_guide = Vec::with_capacity(route.waypoints.len() + 2);
            route_guide.push((
                transform_x(series.scenario.initial_state.position_m.x, flip_sign),
                series.scenario.initial_state.position_m.y,
            ));
            for waypoint in &route.waypoints {
                let x = transform_x(waypoint.position_m.x, flip_sign);
                let y = waypoint.position_m.y;
                let key = format!(
                    "{}:{x:.2}:{y:.2}:{:.2}:{:.2}",
                    waypoint.id, waypoint.capture_radius_m, waypoint.max_cross_track_m
                );
                if options.show_waypoints && waypoint_keys.insert(key) {
                    waypoints.push(PreviewWaypoint {
                        id: waypoint.id.clone(),
                        x_m: x,
                        y_m: y,
                        capture_radius_m: waypoint.capture_radius_m,
                        max_cross_track_m: waypoint.max_cross_track_m,
                        min_outbound_progress_mps: waypoint.min_outbound_progress_mps,
                        max_outbound_cross_speed_mps: waypoint.max_outbound_cross_speed_mps,
                        min_speed_mps: waypoint.min_speed_mps,
                        max_speed_mps: waypoint.max_speed_mps,
                        min_vertical_speed_mps: waypoint.min_vertical_speed_mps,
                        max_vertical_speed_mps: waypoint.max_vertical_speed_mps,
                    });
                }
                route_guide.push((x, y));
            }
            if let Some((target_x_m, target_y_m, _)) = pad {
                route_guide.push((target_x_m, target_y_m));
            }
            if options.show_waypoint_guides {
                route_guides.push(route_guide);
            }
        }
    }
    let planner_overlays = series
        .first()
        .and_then(|series| series.route_plan)
        .map(|plan| {
            let flip_sign = first_flip_sign;
            let safe_profile = plan
                .diagnostics
                .safe_profile_points_m
                .iter()
                .map(|point| (transform_x(point.x, flip_sign), point.y))
                .collect::<Vec<_>>();
            let centerline = plan
                .diagnostics
                .selected_centerline_m
                .iter()
                .map(|point| (transform_x(point.x, flip_sign), point.y))
                .collect::<Vec<_>>();
            (safe_profile, centerline)
        });
    let trajectories = series
        .iter()
        .map(|series| {
            let flip_sign =
                preview_flip_sign(series.scenario.initial_state.position_m.x - normalize_center_x);
            let trajectory = match series.trajectory {
                PreviewTrajectory::Samples(samples) => samples
                    .iter()
                    .map(|sample| sample.observation.position_m)
                    .collect::<Vec<_>>(),
                PreviewTrajectory::Positions(positions) => positions.to_vec(),
            }
            .into_iter()
            .map(|position| (transform_x(position.x, flip_sign), position.y))
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .collect::<Vec<_>>();
            let reference = if multi_run || !options.show_reference {
                Vec::new()
            } else {
                pad.map(|(target_x_m, target_y_m, _)| {
                    let target_x_world = normalize_center_x + target_x_m;
                    transfer_preview_reference_curve(series, target_x_world, target_y_m)
                        .unwrap_or_else(|| {
                            idealized_reference_curve(
                                series.scenario.initial_state.position_m.x,
                                series.scenario.initial_state.position_m.y,
                                target_x_world,
                                target_y_m,
                                series.scenario.world.gravity_mps2,
                            )
                        })
                        .into_iter()
                        .map(|(x, y)| (transform_x(x, flip_sign), y))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
            };
            (
                trajectory,
                reference,
                series
                    .manifest
                    .map(|manifest| enum_label(&manifest.mission_outcome)),
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
    for route_guide in &route_guides {
        for &(x, y) in route_guide {
            include_point(x, y);
        }
    }
    let handoff_points = handoffs
        .iter()
        .enumerate()
        .filter(|(_, handoff)| handoff.x.is_finite() && handoff.y.is_finite())
        .map(|(index, handoff)| {
            let flip_sign = series
                .first()
                .map(|series| {
                    preview_flip_sign(
                        series.scenario.initial_state.position_m.x - normalize_center_x,
                    )
                })
                .unwrap_or(1.0);
            let x = transform_x(handoff.x, flip_sign);
            let y = handoff.y;
            include_point(x, y);
            (index + 1, handoff.x, handoff.y, x, y)
        })
        .collect::<Vec<_>>();
    if let Some((safe_profile, centerline)) = planner_overlays.as_ref() {
        for &(x, y) in safe_profile.iter().chain(centerline.iter()) {
            include_point(x, y);
        }
    }
    for waypoint in &waypoints {
        if options.show_waypoint_guides {
            let envelope_radius_m = waypoint
                .capture_radius_m
                .max(waypoint.max_cross_track_m)
                .max(1.0);
            include_point(
                waypoint.x_m - envelope_radius_m,
                waypoint.y_m - envelope_radius_m,
            );
            include_point(
                waypoint.x_m + envelope_radius_m,
                waypoint.y_m + envelope_radius_m,
            );
        } else {
            include_point(waypoint.x_m, waypoint.y_m);
        }
    }
    if options.show_context
        && let Some((center_x_m, surface_y_m, width_m)) = pad
    {
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
    let start_markers = if options.show_endpoint_markers && !multi_run {
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
    } else {
        String::new()
    };
    let end_markers = if options.show_endpoint_markers {
        trajectories
            .iter()
            .enumerate()
            .filter_map(|(index, (trajectory, _, outcome))| {
                let outcome = outcome.as_deref()?;
                trajectory.last().copied().map(|(x, y)| {
                    let (px, py) = project(x, y);
                    let seed_color = preview_seed_color(index, trajectories.len());
                    match outcome {
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
            .collect::<String>()
    } else {
        String::new()
    };
    let handoff_svg = handoff_points
        .iter()
        .map(|(number, world_x, world_y, x, y)| {
            let (px, py) = project(*x, *y);
            format!(
                r##"<g class="handoff-marker" data-handoff="{number}" data-world-x="{world_x}" data-world-y="{world_y}"><title>Executed handoff H{number}</title><circle cx="{px:.2}" cy="{py:.2}" r="3.7" fill="#fffaf2" stroke="#b95024" stroke-width="1.5"/><text x="{label_x:.2}" y="{label_y:.2}" font-family="ui-monospace, SFMono-Regular, Menlo, monospace" font-size="6.2" font-weight="700" fill="#8a3c1b">H{number}</text></g>"##,
                label_x = px + 4.8,
                label_y = py - 4.2,
            )
        })
        .collect::<String>();
    let pad_svg = if options.show_context {
        pad.map(|(center_x_m, surface_y_m, width_m)| {
            let (x1, y1) = project(center_x_m - (0.5 * width_m), surface_y_m);
            let (x2, y2) = project(center_x_m + (0.5 * width_m), surface_y_m);
            format!(
                r##"<line x1="{x1:.2}" y1="{y1:.2}" x2="{x2:.2}" y2="{y2:.2}" stroke="#2f9e44" stroke-width="3.2" stroke-linecap="round"/>"##
            )
        })
    } else {
        None
    };
    let mut route_guide_keys = BTreeSet::new();
    let route_guide_svg = route_guides
        .iter()
        .flat_map(|points| {
            points
                .windows(2)
                .enumerate()
                .filter_map(|(index, segment)| {
                    let segment_points = polyline_points(segment);
                    (!segment_points.is_empty()).then_some((index, segment_points))
                })
                .collect::<Vec<_>>()
        })
        .filter(|(_, points)| route_guide_keys.insert(points.clone()))
        .map(|(index, points)| {
            let color = preview_route_leg_color(index);
            let dasharray = preview_route_leg_dash(index);
            let title = if index == 0 {
                "waypoint active leg"
            } else {
                "waypoint outbound leg"
            };
            format!(
                r##"<g><title>{title}</title><polyline points="{points}" fill="none" stroke="{color}" stroke-width="{width:.1}" stroke-opacity="0.76" stroke-dasharray="{dasharray}" stroke-linecap="round" stroke-linejoin="round"/></g>"##,
                width = if multi_run { 0.95 } else { 1.15 },
            )
        })
        .collect::<String>();
    let waypoint_svg = waypoints
        .iter()
        .map(|waypoint| {
            let (px, py) = project(waypoint.x_m, waypoint.y_m);
            if !options.show_waypoint_guides {
                return format!(
                    r##"<g><title>{title}</title><path d="M {px:.2} {top:.2} L {right:.2} {py:.2} L {px:.2} {bottom:.2} L {left:.2} {py:.2} Z" fill="#f08c00" stroke="#fffaf2" stroke-width="0.9"/></g>"##,
                    title = escape_html(&format!("{} waypoint", waypoint.id)),
                    top = py - 3.5,
                    right = px + 3.5,
                    bottom = py + 3.5,
                    left = px - 3.5,
                );
            }
            let capture_radius_px = (waypoint.capture_radius_m * scale).clamp(3.0, 18.0);
            let cross_track_px = (waypoint.max_cross_track_m * scale)
                .clamp(capture_radius_px + 1.0, 24.0);
            let label = if multi_run {
                String::new()
            } else {
                format!(
                    r##"<text x="{label_x:.2}" y="{label_y:.2}" font-family="ui-monospace, SFMono-Regular, Menlo, monospace" font-size="6.2" font-weight="700" fill="#7a3d0f">WP</text>"##,
                    label_x = px + capture_radius_px + 1.8,
                    label_y = py - 1.8,
                )
            };
            let outbound_cross = waypoint
                .max_outbound_cross_speed_mps
                .map(|value| format!("cross <= {value:.1} m/s"))
                .unwrap_or_else(|| "cross unbounded".to_owned());
            let vertical = match (
                waypoint.min_vertical_speed_mps,
                waypoint.max_vertical_speed_mps,
            ) {
                (Some(min_value), Some(max_value)) => {
                    format!("vertical {min_value:.1}-{max_value:.1} m/s")
                }
                (Some(min_value), None) => format!("vertical >= {min_value:.1} m/s"),
                (None, Some(max_value)) => format!("vertical <= {max_value:.1} m/s"),
                (None, None) => "vertical unbounded".to_owned(),
            };
            format!(
                r##"<g>
  <title>{title}</title>
  <circle cx="{px:.2}" cy="{py:.2}" r="{cross_track_px:.2}" fill="none" stroke="#a25a18" stroke-width="0.85" stroke-opacity="0.36" stroke-dasharray="2 2"/>
  <circle cx="{px:.2}" cy="{py:.2}" r="{capture_radius_px:.2}" fill="#f08c00" fill-opacity="0.16" stroke="#a25a18" stroke-width="1.1"/>
  <circle cx="{px:.2}" cy="{py:.2}" r="1.8" fill="#7a3d0f" stroke="#fffaf2" stroke-width="0.75"/>
  {label}
</g>"##,
                title = escape_html(&format!(
                    "{}: capture radius {:.1} m, cross-track {:.1} m, outbound >= {:.1} m/s, {}, speed {:.1}-{:.1} m/s, {}",
                    waypoint.id,
                    waypoint.capture_radius_m,
                    waypoint.max_cross_track_m,
                    waypoint.min_outbound_progress_mps,
                    outbound_cross,
                    waypoint.min_speed_mps,
                    waypoint.max_speed_mps,
                    vertical,
                )),
            )
        })
        .collect::<String>();
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
    let planner_overlay_svg = planner_overlays
        .map(|(safe_profile, centerline)| {
            let safe = polyline_points(&safe_profile);
            let selected = polyline_points(&centerline);
            format!(
                r##"<g><title>planner safe profile and selected centerline</title><polyline points="{safe}" fill="none" stroke="#9b59b6" stroke-width="1.1" stroke-dasharray="2 2" stroke-opacity="0.78"/><polyline points="{selected}" fill="none" stroke="#e8590c" stroke-width="1.0" stroke-dasharray="5 2" stroke-opacity="0.86"/></g>"##
            )
        })
        .unwrap_or_default();

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH_PX}" height="{HEIGHT_PX}" viewBox="0 0 {WIDTH_PX} {HEIGHT_PX}" role="img" aria-label="run trajectory preview">
  <rect width="100%" height="100%" rx="10" fill="#fbf7ee"/>
  <rect x="0.5" y="0.5" width="{border_w:.1}" height="{border_h:.1}" rx="9.5" fill="none" stroke="#d7cdbd"/>
  {reference_svg}
  {trajectory_svg}
  {planner_overlay_svg}
  {terrain_svg}
  {pad_svg}
  {route_guide_svg}
  {waypoint_svg}
  {start_markers}
  {end_markers}{handoff_svg}
</svg>"##,
        border_w = WIDTH_PX - 1.0,
        border_h = HEIGHT_PX - 1.0,
        route_guide_svg = route_guide_svg,
        reference_svg = reference_svg,
        trajectory_svg = trajectory_svg,
        planner_overlay_svg = planner_overlay_svg,
        terrain_svg = if terrain_points.is_empty() {
            String::new()
        } else {
            format!(
                r##"<polyline points="{terrain_points}" fill="none" stroke="#7a5d3e" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"##
            )
        },
        pad_svg = pad_svg.unwrap_or_default(),
        waypoint_svg = waypoint_svg,
        start_markers = start_markers,
        end_markers = end_markers,
        handoff_svg = handoff_svg,
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

fn preview_route_leg_color(index: usize) -> &'static str {
    if index == 0 { "#b45309" } else { "#1d4ed8" }
}

fn preview_route_leg_dash(index: usize) -> &'static str {
    if index == 0 { "6 3" } else { "7 3 2 3" }
}

fn preview_flip_sign(initial_dx: f64) -> f64 {
    if initial_dx > 0.0 { -1.0 } else { 1.0 }
}

fn transfer_preview_reference_curve(
    series: &PreviewRenderSeries<'_>,
    target_x: f64,
    target_y: f64,
) -> Option<Vec<(f64, f64)>> {
    series.scenario.mission.transfer_route.as_ref()?;
    let controller_updates = series.controller_updates?;
    let PreviewTrajectory::Samples(samples) = series.trajectory else {
        return None;
    };
    let first_boost = controller_updates.iter().find(|update| {
        telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("boost")
    })?;
    let start_sample = samples
        .iter()
        .find(|sample| sample.physics_step >= first_boost.physics_step)
        .or_else(|| samples.first())?;
    let start_x = start_sample.observation.position_m.x;
    let start_y = start_sample.observation.position_m.y;
    let dx = target_x - start_x;
    let dy = target_y - start_y;
    if !dx.is_finite() || !dy.is_finite() || dx.abs() <= 1e-6 {
        return None;
    }
    let apex_over_target = transfer_shape_apex_target_over_target_m(dx.abs(), dy);
    let point_count = 48_usize;
    let mut points = Vec::with_capacity(point_count);
    for index in 0..point_count {
        let s = index as f64 / (point_count - 1) as f64;
        let x = start_x + (dx * s);
        let baseline = start_y + (dy * s);
        let y = baseline + (4.0 * apex_over_target * s * (1.0 - s));
        points.push((x, y));
    }
    if let Some(last) = points.last_mut() {
        *last = (target_x, target_y);
    }
    Some(points)
}

fn transfer_shape_apex_target_over_target_m(dx_abs_m: f64, dy_m: f64) -> f64 {
    const APEX_HEIGHT_PER_DX: f64 = 0.18;
    const APEX_HEIGHT_PER_UPHILL_DY: f64 = 0.15;
    const APEX_HEIGHT_MIN_M: f64 = 30.0;
    const APEX_HEIGHT_MAX_M: f64 = 240.0;

    (APEX_HEIGHT_PER_DX * dx_abs_m).clamp(APEX_HEIGHT_MIN_M, APEX_HEIGHT_MAX_M)
        + (dy_m * APEX_HEIGHT_PER_UPHILL_DY).max(0.0)
        + (-dy_m).max(0.0)
}

fn telemetry_text<'a>(metrics: &'a BTreeMap<String, TelemetryValue>, key: &str) -> Option<&'a str> {
    match metrics.get(key)? {
        TelemetryValue::Text(value) => Some(value),
        _ => None,
    }
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
                    id: marker.id.clone(),
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
            min_throttle_frac: scenario.vehicle.min_throttle_frac,
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
        transfer_route: scenario.mission.transfer_route.as_ref().map(|route| {
            ReportTransferRouteDetails {
                source_pad_id: route.source_pad_id.clone(),
                target_pad_id: route.target_pad_id.clone(),
                route_angle_deg: route.route_angle_deg,
                route_radius_m: route.route_radius_m,
                waypoints: route
                    .waypoints
                    .iter()
                    .map(|waypoint| ReportWaypointDetails {
                        id: waypoint.id.clone(),
                        x_m: waypoint.position_m.x,
                        y_m: waypoint.position_m.y,
                        handoff_tangent_x: waypoint.handoff_tangent_unit.map(|tangent| tangent.x),
                        handoff_tangent_y: waypoint.handoff_tangent_unit.map(|tangent| tangent.y),
                        handoff_tangent_heading_deg: waypoint
                            .handoff_tangent_unit
                            .map(|tangent| tangent.y.atan2(tangent.x).to_degrees()),
                        capture_radius_m: waypoint.capture_radius_m,
                        max_cross_track_m: waypoint.max_cross_track_m,
                        max_outbound_heading_error_deg: waypoint
                            .max_outbound_heading_error_rad
                            .to_degrees(),
                        min_outbound_progress_mps: waypoint.min_outbound_progress_mps,
                        max_outbound_cross_speed_mps: waypoint.max_outbound_cross_speed_mps,
                        min_speed_mps: waypoint.min_speed_mps,
                        max_speed_mps: waypoint.max_speed_mps,
                        min_vertical_speed_mps: waypoint.min_vertical_speed_mps,
                        max_vertical_speed_mps: waypoint.max_vertical_speed_mps,
                    })
                    .collect(),
            }
        }),
        terrain_point_count: scenario.world.terrain.points().len(),
        evaluation_basis: mission_evaluation_basis(&scenario.mission.goal),
    }
}

fn mission_evaluation_basis(goal: &EvaluationGoal) -> String {
    match goal {
        EvaluationGoal::LandingOnPad { .. } => "Landing success currently uses stable contact plus pad overlap, normal/tangential touchdown speed, attitude error, and angular rate. No force or impulse check is used yet.".to_owned(),
        EvaluationGoal::WaypointHandoff { .. } => "Waypoint handoff success opens a capture window on radius entry, accepts the spatial and planned-tangent velocity envelope before crossing, and fails only at the waypoint-plane deadline.".to_owned(),
        EvaluationGoal::WaypointSequence { .. } => "Waypoint sequence success requires every transfer waypoint to resolve its spatial and planned-tangent velocity envelope in route order before the corresponding waypoint-plane deadline.".to_owned(),
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
    display_title: String,
    display_subtitle: String,
    report_context: RunReportContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    flight_annotations: Option<flight_annotations::FlightAnnotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    route_plan: Option<RoutePlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    planner_compute: Option<ReportPlannerCompute>,
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
struct ReportPlannerCompute {
    wall_time_us: u64,
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
    id: String,
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
    transfer_route: Option<ReportTransferRouteDetails>,
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
    min_throttle_frac: f64,
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportTransferRouteDetails {
    source_pad_id: String,
    target_pad_id: String,
    route_angle_deg: f64,
    route_radius_m: f64,
    waypoints: Vec<ReportWaypointDetails>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportWaypointDetails {
    id: String,
    x_m: f64,
    y_m: f64,
    handoff_tangent_x: Option<f64>,
    handoff_tangent_y: Option<f64>,
    handoff_tangent_heading_deg: Option<f64>,
    capture_radius_m: f64,
    max_cross_track_m: f64,
    max_outbound_heading_error_deg: f64,
    min_outbound_progress_mps: f64,
    max_outbound_cross_speed_mps: Option<f64>,
    min_speed_mps: f64,
    max_speed_mps: f64,
    min_vertical_speed_mps: Option<f64>,
    max_vertical_speed_mps: Option<f64>,
}

impl ReportMissionGoalDetails {
    fn from_goal(goal: &EvaluationGoal) -> Self {
        match goal {
            EvaluationGoal::LandingOnPad { target_pad_id } => Self {
                goal_kind: "landing_on_pad".to_owned(),
                target_pad_id: target_pad_id.clone(),
                end_time_s: None,
            },
            EvaluationGoal::WaypointHandoff { target_pad_id, .. } => Self {
                goal_kind: "waypoint_handoff".to_owned(),
                target_pad_id: target_pad_id.clone(),
                end_time_s: None,
            },
            EvaluationGoal::WaypointSequence { target_pad_id } => Self {
                goal_kind: "waypoint_sequence".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::{
        CorridorClearance, CorridorResidual, EndReason, MissionOutcome, NormalizedRouteGeometry,
        PhysicalOutcome, RoutePlanDiagnostics, RoutePlanningPolicy, RouteTopology, RunManifest,
        TransferRouteSpec,
    };
    use std::fs;

    fn fixture_scenario() -> ScenarioSpec {
        serde_json::from_str(
            &fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../fixtures/scenarios/flat_terminal_descent.json"),
            )
            .expect("fixture scenario should be readable"),
        )
        .expect("fixture scenario should parse")
    }

    fn fixture_manifest(scenario: &ScenarioSpec) -> RunManifest {
        RunManifest {
            schema_version: 1,
            scenario_id: scenario.id.clone(),
            scenario_name: scenario.name.clone(),
            scenario_seed: scenario.seed,
            scenario_tags: scenario.tags.clone(),
            controller_id: "planner_test".to_owned(),
            physics_hz: scenario.sim.physics_hz,
            controller_hz: scenario.sim.controller_hz,
            sim_time_s: 0.0,
            physics_steps: 0,
            controller_updates: 0,
            physical_outcome: PhysicalOutcome::Flying,
            mission_outcome: MissionOutcome::InProgress,
            end_reason: EndReason::Running,
            summary: Default::default(),
        }
    }

    fn fixture_plan() -> RoutePlan {
        let residual = CorridorResidual {
            residual_m: -1.0,
            centerline_position_m: Vec2::new(0.0, 5.0),
            terrain_position_m: Vec2::new(0.0, 0.0),
            terrain_segment_index: 0,
            required_envelope_y_m: 4.0,
            centerline_y_m: 5.0,
            vertical_extent_m: 1.0,
        };
        RoutePlan {
            algorithm_id: "heightfield_visibility_v1".to_owned(),
            policy: RoutePlanningPolicy::default(),
            request_digest: "request-test".to_owned(),
            plan_digest: "plan-test".to_owned(),
            topology: RouteTopology::Waypoint,
            route: TransferRouteSpec {
                source_pad_id: "source".to_owned(),
                target_pad_id: "pad_main".to_owned(),
                route_angle_deg: 0.0,
                route_radius_m: 100.0,
                waypoints: Vec::new(),
            },
            normalized_geometry: NormalizedRouteGeometry {
                horizontal_sign: 1,
                direct_horizontal_span_m: 100.0,
                direct_distance_m: 100.0,
                route_angle_rad: 0.0,
                route_angle_deg: 0.0,
            },
            diagnostics: RoutePlanDiagnostics {
                direct_path_clear: false,
                direct_path_clearance: Some(CorridorClearance {
                    clear: false,
                    minimum_clearance_m: -1.0,
                    worst_residual: residual,
                }),
                route_length_m: 120.0,
                direct_distance_m: 100.0,
                excess_length_m: 20.0,
                peak_extra_loft_m: 12.0,
                minimum_planned_clearance_m: 3.0,
                leg_diagnostics: Vec::new(),
                selected_node_ids: vec!["node-0001-0".to_owned()],
                safe_profile_points_m: vec![Vec2::new(0.0, 5.0), Vec2::new(100.0, 5.0)],
                selected_centerline_m: vec![
                    Vec2::new(0.0, 6.0),
                    Vec2::new(50.0, 16.0),
                    Vec2::new(100.0, 5.0),
                ],
                waypoint_authority: Vec::new(),
            },
        }
    }

    #[test]
    fn flight_annotation_empty_path_preserves_non_v2_report_data_and_features() {
        let scenario = fixture_scenario();
        let manifest = fixture_manifest(&scenario);
        let render = |annotations: Option<&flight_annotations::FlightAnnotations>| {
            render_run_report_with_flight_annotations(
                &scenario,
                None,
                &manifest,
                &[],
                &[],
                &[],
                None,
                None,
                None,
                None,
                annotations,
            )
            .unwrap()
        };
        let baseline = render(None);
        assert_eq!(
            baseline,
            render(Some(&flight_annotations::FlightAnnotations::default()))
        );
        assert!(!baseline.contains("\"flightAnnotations\":"));
        assert!(!baseline.contains("id=\"flight-corrections-panel\""));
        for retained in [
            "data-mode=\"mission\"",
            "data-mode=\"guidance\"",
            "data-mode=\"speed\"",
            "data-mode=\"throttle\"",
            "data-mode=\"vectors\"",
            "chart-metrics",
            "Hovered Sample",
            "What Happened",
            "Markers And Config",
            "Landing Quality",
            "Plotly.newPlot",
        ] {
            assert!(baseline.contains(retained), "missing {retained}");
        }
    }

    #[test]
    fn flight_annotation_retains_odd_exact_boundary_and_rejects_invalid_state() {
        use flight_annotations::{ExecutedCorrection, FlightAnnotations, FlightBoundary};
        let scenario = fixture_scenario();
        let mut manifest = fixture_manifest(&scenario);
        manifest.physics_steps = 24;
        let boundary = |step| FlightBoundary {
            physics_step: step,
            sim_time_s: step as f64 / manifest.physics_hz as f64,
            position_m: Vec2::new(2.0, 3.0),
            velocity_mps: Vec2::new(1.0, -1.0),
            attitude_rad: 0.1,
            fuel_kg: 100.0,
        };
        let mut annotations = FlightAnnotations {
            corrections: vec![ExecutedCorrection {
                number: 1,
                entry: boundary(0),
                handoff: boundary(13),
                reason: "<terrain>".into(),
                after_handoff: "replan".into(),
            }],
            ..Default::default()
        };
        let render = |annotations: &FlightAnnotations| {
            render_run_report_with_flight_annotations(
                &scenario,
                None,
                &manifest,
                &[],
                &[],
                &[],
                None,
                None,
                None,
                None,
                Some(annotations),
            )
        };
        let html = render(&annotations).unwrap();
        assert!(html.contains("\"physicsStep\":13"));
        assert!(html.contains("&lt;terrain&gt;"));
        annotations.corrections[0].handoff.attitude_rad = f64::NAN;
        assert!(render(&annotations).is_err());
        annotations.corrections[0].handoff = boundary(13);
        annotations.corrections[0].handoff.sim_time_s = 1.0;
        assert!(render(&annotations).is_err());
        annotations.corrections[0].handoff = boundary(25);
        assert!(render(&annotations).is_err());
    }

    #[test]
    fn planner_report_and_preview_show_plan_evidence_but_legacy_stays_unchanged() {
        let scenario = fixture_scenario();
        let manifest = fixture_manifest(&scenario);
        let plan = fixture_plan();
        let report_path = std::env::temp_dir().join(format!(
            "pd_report_planner_{}_{}.html",
            std::process::id(),
            plan.plan_digest
        ));
        write_run_report_with_plan_context(
            &report_path,
            &scenario,
            None,
            &manifest,
            &[],
            &[],
            &[],
            None,
            None,
            Some(&plan),
        )
        .expect("planner report should render");
        let html = fs::read_to_string(&report_path).expect("planner report should be readable");
        assert!(html.contains("Generated Route Plan"));
        assert!(html.contains("heightfield_visibility_v1"));
        assert!(html.contains("plan-test"));
        assert!(html.contains(r#""algorithm_id":"heightfield_visibility_v1""#));
        assert!(html.contains("safe_profile_points_m"));
        assert!(html.contains("plannerDiagnostics?.safe_profile_points_m"));
        assert!(html.contains("planner selected centerline"));
        let legacy_path = report_path.with_file_name("pd_report_legacy.html");
        write_run_report(
            &legacy_path,
            &scenario,
            None,
            &manifest,
            &[],
            &[],
            &[],
            None,
        )
        .expect("legacy report should render");
        let legacy = fs::read_to_string(&legacy_path).expect("legacy report should be readable");
        assert!(!legacy.contains("Generated Route Plan"));
        assert!(!legacy.contains("heightfield_visibility_v1"));
        let svg = build_run_preview_svg_with_plan(&scenario, &manifest, &[], &[], Some(&plan));
        assert!(svg.contains("planner safe profile and selected centerline"));
        assert!(svg.contains("points=\""));
        let legacy_svg = build_run_preview_svg_with_plan(&scenario, &manifest, &[], &[], None);
        assert!(!legacy_svg.contains("planner safe profile and selected centerline"));
        fs::remove_file(report_path).expect("planner report should be removable");
        fs::remove_file(legacy_path).expect("legacy report should be removable");
    }

    #[test]
    fn saved_flight_preview_marks_actual_handoffs_and_keeps_empty_marker_path_stable() {
        let scenario = fixture_scenario();
        let manifest = fixture_manifest(&scenario);
        let ordinary_svg = build_run_preview_svg_with_plan(&scenario, &manifest, &[], &[], None);
        let ordinary_series = [PreviewRenderSeries {
            scenario: &scenario,
            manifest: Some(&manifest),
            trajectory: PreviewTrajectory::Samples(&[]),
            controller_updates: Some(&[]),
            route_plan: None,
        }];
        let empty_handoff_svg =
            build_preview_svg_with_handoffs(&ordinary_series, PreviewOptions::full(), &[]);
        assert_eq!(ordinary_svg, empty_handoff_svg);
        assert!(!ordinary_svg.contains("data-handoff="));

        let handoffs = [Vec2::new(142.125, 31.75), Vec2::new(-19.5, -8.25)];
        let saved_svg = build_saved_flight_preview_svg(&scenario, Some(&manifest), &[], &handoffs);
        assert_eq!(saved_svg.matches("class=\"handoff-marker\"").count(), 2);
        assert!(
            saved_svg
                .contains("data-handoff=\"1\" data-world-x=\"142.125\" data-world-y=\"31.75\"")
        );
        assert!(
            saved_svg.contains("data-handoff=\"2\" data-world-x=\"-19.5\" data-world-y=\"-8.25\"")
        );
        assert!(!saved_svg.contains("planner safe profile and selected centerline"));

        let context_svg = build_saved_flight_preview_svg(&scenario, None, &[], &[]);
        assert!(context_svg.contains("stroke=\"#7a5d3e\""));
        assert!(context_svg.contains("stroke=\"#2f9e44\""));
        assert!(!context_svg.contains("data-handoff="));
        assert!(!context_svg.contains("stroke=\"#1d5e7a\""));
    }
}
#[cfg(test)]
mod report_tests {
    use super::*;

    fn fixture_scenario() -> ScenarioSpec {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/scenarios/flat_terminal_descent.json");
        serde_json::from_slice(&fs::read(path).expect("scenario fixture should be readable"))
            .expect("scenario fixture should parse")
    }

    #[test]
    fn terminal_run_title_uses_readable_selectors() {
        let mut scenario = fixture_scenario();
        scenario
            .metadata
            .insert("mission".to_owned(), "terminal_guidance".to_owned());
        scenario
            .metadata
            .insert("arc_point".to_owned(), "a80".to_owned());
        scenario
            .metadata
            .insert("velocity_band".to_owned(), "high".to_owned());
        scenario
            .metadata
            .insert("vehicle_variant".to_owned(), "full".to_owned());

        assert_eq!(
            friendly_report_title(&scenario),
            "Terminal arrival · A80 / High / Full"
        );
    }

    #[test]
    fn waypoint_run_title_strips_profile_plumbing() {
        let mut scenario = fixture_scenario();
        scenario
            .metadata
            .insert("mission".to_owned(), "transfer_guidance".to_owned());
        scenario
            .metadata
            .insert("route_angle".to_owned(), "r+30".to_owned());
        scenario
            .metadata
            .insert("radius_tier".to_owned(), "nominal".to_owned());
        scenario
            .metadata
            .insert("waypoint_profile".to_owned(), "double_bend_v1".to_owned());
        scenario
            .metadata
            .insert("vehicle_variant".to_owned(), "empty".to_owned());

        assert_eq!(
            friendly_report_title(&scenario),
            "Waypoint sequence · bend / R+30 / Nominal / Empty"
        );
    }

    #[test]
    fn direct_transfer_profile_is_not_labeled_as_a_waypoint() {
        let mut scenario = fixture_scenario();
        scenario
            .metadata
            .insert("mission".to_owned(), "transfer_guidance".to_owned());
        scenario
            .metadata
            .insert("route_angle".to_owned(), "r-80".to_owned());
        scenario
            .metadata
            .insert("radius_tier".to_owned(), "short".to_owned());
        scenario
            .metadata
            .insert("waypoint_profile".to_owned(), "direct".to_owned());
        scenario
            .metadata
            .insert("vehicle_variant".to_owned(), "empty".to_owned());

        assert_eq!(
            friendly_report_title(&scenario),
            "Direct transfer · R-80 / Short / Empty"
        );
    }

    #[test]
    fn run_template_defaults_to_mission_evidence() {
        let template = report_template();
        assert!(template.contains("data-mode=\"mission\" class=\"active\""));
        assert!(template.contains("const buildPhaseBandShapes"));
        assert!(template.contains("visibilityForMode(\"mission\")"));
        assert!(template.contains("data-mode=\"guidance\""));
    }
}
