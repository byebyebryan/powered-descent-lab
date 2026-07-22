use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use pd_core::Vec2;
use pd_report::site::{eval_report_entry_is_fixture_backed, load_fixture_pack_ids};

use crate::{
    BatchReport, BatchRunAnalyticClass, BatchRunAnalyticReason, BatchRunRecord,
    ConcreteScenarioPackEntry, NumericPerturbationMode, NumericPerturbationSpec,
    ScenarioFamilyEntry, ScenarioPackEntry, ScenarioPackSpec, SeedRangeSpec, TerminalMatrixEntry,
    TerminalMatrixLaneSpec, TerminalSeedTier, TransferMatrixEntry, TransferMatrixEvaluationGoal,
    TransferMatrixLaneSpec, TransferSeedTier, compare_batch_reports, run_pack_with_workers,
};

use super::{
    BatchReportRenderCache, directory_href, html_with_base_href, load_preview_trajectory,
    records_by_waypoint_profile, render_batch_report, render_lane_preview,
    render_waypoint_terminal_recovery_cell, report_site_output_for_batch, sort_selector_keys,
    tree_group_id, waypoint_checkpoint_failure_detail,
};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures")
}

fn temp_report_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "pd-eval-report-{label}-{}-{nonce}",
        std::process::id()
    ))
}

#[test]
fn batch_report_site_alias_targets_stable_output_without_rendering_cache_sites() {
    let outputs = crate::repo_root().join("outputs");
    let stable_dir = outputs.join("eval").join("fixture_pack");
    let site_dir = outputs.join("reports").join("eval").join("fixture_pack");
    assert_eq!(
        directory_href(&site_dir, &stable_dir),
        "../../../eval/fixture_pack/"
    );
    assert_eq!(
        report_site_output_for_batch(&stable_dir),
        Some(site_dir.join("index.html"))
    );
    assert!(
        report_site_output_for_batch(
            &outputs
                .join("eval")
                .join("cache")
                .join("workspace")
                .join("fixture_pack")
        )
        .is_none()
    );

    let rendered = "<!DOCTYPE html><html><head><title>report</title></head><body></body></html>";
    let aliased = html_with_base_href(rendered, "../../../eval/fixture_pack/");
    assert!(aliased.contains(r#"<base href="../../../eval/fixture_pack/" />"#));
    assert_eq!(aliased.matches("<title>report</title>").count(), 1);
}

#[test]
fn eval_report_index_keeps_only_current_fixture_pack_ids() {
    let root = temp_report_dir("fixture-backed-index");
    let fixtures_dir = root.join("fixtures");
    let raw_eval_dir = root.join("eval");
    fs::create_dir_all(&fixtures_dir).unwrap();
    fs::create_dir_all(raw_eval_dir.join("custom-output")).unwrap();
    fs::create_dir_all(raw_eval_dir.join("diagnostic-output")).unwrap();
    fs::create_dir_all(raw_eval_dir.join("orphan-output")).unwrap();
    fs::write(
        fixtures_dir.join("maintained.json"),
        r#"{"id":"maintained_pack"}"#,
    )
    .unwrap();
    fs::write(
        fixtures_dir.join("diagnostic.json"),
        r#"{"id":"diagnostic_pack"}"#,
    )
    .unwrap();
    fs::write(
        raw_eval_dir.join("custom-output").join("pack.json"),
        r#"{"id":"maintained_pack"}"#,
    )
    .unwrap();
    fs::write(
        raw_eval_dir.join("diagnostic-output").join("pack.json"),
        r#"{"id":"diagnostic_pack"}"#,
    )
    .unwrap();
    fs::write(
        raw_eval_dir.join("orphan-output").join("pack.json"),
        r#"{"id":"removed_pack"}"#,
    )
    .unwrap();

    let fixture_ids = load_fixture_pack_ids(&fixtures_dir).unwrap();
    assert!(eval_report_entry_is_fixture_backed(
        &raw_eval_dir,
        "custom-output",
        &fixture_ids
    ));
    assert!(eval_report_entry_is_fixture_backed(
        &raw_eval_dir,
        "diagnostic-output",
        &fixture_ids
    ));
    assert!(!eval_report_entry_is_fixture_backed(
        &raw_eval_dir,
        "orphan-output",
        &fixture_ids
    ));
    assert!(!eval_report_entry_is_fixture_backed(
        &raw_eval_dir,
        "cache",
        &fixture_ids
    ));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn aggregate_lane_preview_uses_compact_samples_and_reuses_cached_svg() {
    let bundle_dir = temp_report_dir("lane-preview-cache");
    fs::create_dir_all(&bundle_dir).unwrap();
    fs::copy(
        fixtures_root().join("scenarios/flat_terminal_descent.json"),
        bundle_dir.join("scenario.json"),
    )
    .unwrap();
    let samples_path = bundle_dir.join("samples.json");
    fs::write(
        &samples_path,
        r#"[
              {"observation":{"position_m":{"x":-120.0,"y":80.0},"ignored":true},"ignored":1},
              {"observation":{"position_m":{"x":0.0,"y":0.0},"ignored":true},"ignored":2}
            ]"#,
    )
    .unwrap();

    let positions = load_preview_trajectory(&samples_path).unwrap();
    assert_eq!(positions.len(), 2);
    assert_eq!(positions[0], Vec2::new(-120.0, 80.0));

    let mut report = synthetic_transfer_shape_report(
        "aggregate_lane_preview_unit",
        &[("r00", "empty", 20.0, 0)],
    );
    report.records[0].bundle_dir = Some(bundle_dir.to_string_lossy().into_owned());
    let record = &report.records[0];
    let cache = BatchReportRenderCache::default();
    let first = render_lane_preview(&[record], &cache).unwrap();
    assert!(first.contains("run trajectory preview"));
    assert!(first.contains("polyline"));

    fs::remove_file(&samples_path).unwrap();
    let second = render_lane_preview(&[record], &cache).unwrap();
    assert_eq!(second, first);
    fs::remove_dir_all(&bundle_dir).unwrap();
}

fn terminal_metadata(vehicle_variant: &str, expectation_tier: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("mission".to_owned(), "terminal_guidance".to_owned()),
        (
            "arrival_family".to_owned(),
            "seeded_terminal_arrival_v0".to_owned(),
        ),
        ("condition_set".to_owned(), "clean".to_owned()),
        ("vehicle_variant".to_owned(), vehicle_variant.to_owned()),
        ("expectation_tier".to_owned(), expectation_tier.to_owned()),
    ])
}

fn checkpoint_metadata() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("mission".to_owned(), "checkpoint_validation".to_owned()),
        (
            "arrival_family".to_owned(),
            "checkpoint_reference_v0".to_owned(),
        ),
        ("condition_set".to_owned(), "clean".to_owned()),
        ("vehicle_variant".to_owned(), "nominal".to_owned()),
        ("expectation_tier".to_owned(), "reference".to_owned()),
    ])
}

fn terminal_family_entry(id: &str, family: &str, controller: &str) -> ScenarioPackEntry {
    ScenarioPackEntry::Family(ScenarioFamilyEntry {
        id: id.to_owned(),
        family: family.to_owned(),
        base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
        controller: controller.to_owned(),
        controller_config: None,
        seeds: Vec::new(),
        seed_range: Some(SeedRangeSpec { start: 0, count: 2 }),
        perturbations: vec![
            NumericPerturbationSpec {
                id: "spawn_dx".to_owned(),
                path: "initial_state.position_m.x".to_owned(),
                mode: NumericPerturbationMode::Offset,
                min: -10.0,
                max: 10.0,
                quantize: Some(0.5),
            },
            NumericPerturbationSpec {
                id: "spawn_vy".to_owned(),
                path: "initial_state.velocity_mps.y".to_owned(),
                mode: NumericPerturbationMode::Offset,
                min: -2.0,
                max: 2.0,
                quantize: Some(0.25),
            },
        ],
        tags: vec!["test".to_owned(), "terminal".to_owned()],
        metadata: terminal_metadata("nominal", "core"),
    })
}

fn checkpoint_entry() -> ScenarioPackEntry {
    ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
        id: "checkpoint_idle_reference".to_owned(),
        scenario: "scenarios/timed_checkpoint_idle.json".to_owned(),
        controller: "idle".to_owned(),
        controller_config: None,
        metadata: checkpoint_metadata(),
    })
}

fn report_with_records(mut report: BatchReport, records: Vec<BatchRunRecord>) -> BatchReport {
    report.total_runs = records.len();
    report.resolved_runs = records
        .iter()
        .map(|record| record.resolved.clone())
        .collect();
    report.summary = crate::summarize_records(&records);
    report.records = records;
    report
}

fn synthetic_transfer_shape_report(id: &str, cells: &[(&str, &str, f64, u64)]) -> BatchReport {
    let pack = ScenarioPackSpec {
        id: id.to_owned(),
        name: format!("{id} report"),
        description: "synthetic transfer shape report".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![checkpoint_entry()],
    };
    let base_report = run_pack_with_workers(&pack, &fixtures_root(), None, 1).unwrap();
    let template = base_report
        .records
        .first()
        .expect("checkpoint report should contain one record")
        .clone();
    let records = cells
        .iter()
        .map(|(route_angle, vehicle_variant, shape_rmse_m, seed)| {
            synthetic_transfer_shape_record(
                &template,
                route_angle,
                vehicle_variant,
                *shape_rmse_m,
                *seed,
            )
        })
        .collect::<Vec<_>>();
    report_with_records(base_report, records)
}

fn synthetic_transfer_shape_record(
    template: &BatchRunRecord,
    route_angle: &str,
    vehicle_variant: &str,
    shape_rmse_m: f64,
    seed: u64,
) -> BatchRunRecord {
    let mut record = template.clone();
    record.resolved.run_id = tree_group_id(&[
        "transfer_shape",
        route_angle,
        vehicle_variant,
        &format!("seed_{seed:02}"),
        "current",
    ]);
    record.resolved.entry_id = "transfer_guidance_clean".to_owned();
    record.resolved.family_id = Some("transfer_guidance_clean".to_owned());
    record.resolved.selector.mission = "transfer_guidance".to_owned();
    record.resolved.selector.arrival_family = "signed_route_arc_transfer_v1".to_owned();
    record.resolved.selector.condition_set = "clean".to_owned();
    record.resolved.selector.vehicle_variant = vehicle_variant.to_owned();
    record.resolved.selector.arc_point = route_angle.to_owned();
    record.resolved.selector.velocity_band = "nominal".to_owned();
    record.resolved.selector.route_family = "signed_route_arc_transfer_v1".to_owned();
    record.resolved.selector.route_angle = route_angle.to_owned();
    record.resolved.selector.radius_tier = "nominal".to_owned();
    record.resolved.selector.waypoint_profile = "single_bend_v1".to_owned();
    record.resolved.selector.waypoint_handoff_envelope = "continuation_pass_through_v1".to_owned();
    record.resolved.selector.expectation_tier = Some("core".to_owned());
    record.resolved.lane_id = "current".to_owned();
    record.resolved.resolved_seed = seed;
    record
        .resolved
        .resolved_parameters
        .insert("waypoint_0_turn_angle_deg".to_owned(), 44.0);
    for (key, value) in [
        ("waypoint_0_profile_progress_frac", 0.55),
        ("waypoint_0_route_signed_offset_ratio", 0.20),
        ("waypoint_0_signed_turn_angle_deg", -43.9456),
        ("waypoint_0_max_speed_mps", 65.0),
        ("waypoint_0_continuation_stop_ratio", 0.52),
    ] {
        record
            .resolved
            .resolved_parameters
            .insert(key.to_owned(), value);
    }
    record.resolved.controller_id = "transfer_pdg_v1".to_owned();
    record.manifest.scenario_seed = seed;
    record.manifest.sim_time_s = 20.0 + seed as f64;
    record.manifest.physical_outcome = pd_core::PhysicalOutcome::LandedOnTarget;
    record.manifest.mission_outcome = pd_core::MissionOutcome::Success;
    record.manifest.end_reason = pd_core::EndReason::TouchdownOnTarget;
    record.review.transfer_shape_curve_rmse_m = Some(shape_rmse_m);
    record.review.transfer_shape_apex_error_m = Some(shape_rmse_m * 0.25);
    record.review.transfer_shape_projected_dx_abs_max_m = Some(shape_rmse_m * 1.5);
    record.review.transfer_shape_shortfall_ratio = Some(0.04);
    record.review.transfer_terminal_entry_kind = Some("handoff".to_owned());
    record.review.transfer_terminal_handoff_time_s = Some(12.0);
    record.review.transfer_terminal_handoff_height_m = Some(120.0);
    record.review.transfer_terminal_handoff_speed_mps = Some(24.0);
    record.review.transfer_terminal_handoff_gate_mode = Some("ready".to_owned());
    record.review.transfer_terminal_handoff_projected_dx_m = Some(18.0);
    record.review.transfer_terminal_handoff_impact_angle_deg = Some(58.0);
    record.review.transfer_terminal_post_handoff_apex_gain_m = Some(shape_rmse_m * 0.5);
    record.review.transfer_terminal_post_handoff_time_to_apex_s = Some(8.0);
    record.review.transfer_terminal_post_handoff_apex_dx_abs_m = Some(4.0);
    record.review.transfer_terminal_low_altitude_rebound_gain_m = Some(0.0);
    record
        .review
        .transfer_terminal_low_altitude_rebound_origin_dx_abs_m = Some(4.0);
    record
        .review
        .transfer_terminal_low_altitude_rebound_near_pad = Some(true);
    record.review.transfer_final_phase = Some("terminal".to_owned());
    record.review.transfer_boost_quality = Some("balanced".to_owned());
    record.review.transfer_boost_cutoff_quality = Some("pass".to_owned());
    record.review.transfer_boost_cutoff_projected_dx_m = Some(12.0);
    record.review.transfer_boost_cutoff_impact_angle_deg = Some(61.0);
    record.review.transfer_boost_burn_duration_s = Some(5.0);
    record.review.transfer_terminal_gate_mode = Some("ready".to_owned());
    record.review.transfer_corridor_mode = Some("inactive".to_owned());
    record.bundle_dir = None;
    record
}

#[test]
fn waypoint_triage_renders_for_waypoint_records() {
    let mut report = synthetic_transfer_shape_report(
        "waypoint_triage_unit",
        &[("r+80", "empty", 40.0, 0), ("r+80", "empty", 35.0, 1)],
    );
    report.records[0].review.waypoint_capture_status = Some("captured".to_owned());
    report.records[0].review.waypoint_contract_status = Some("pass".to_owned());
    report.records[0].review.waypoint_closest_distance_m = Some(14.0);
    report.records[0].review.waypoint_cross_track_m = Some(8.0);
    report.records[0].review.waypoint_outbound_heading_error_rad = Some(0.25);
    report.records[0].review.waypoint_outbound_progress_mps = Some(22.0);
    report.records[1].review.waypoint_capture_status = Some("missed".to_owned());
    report.records[1].review.waypoint_contract_status = Some("spatial_miss".to_owned());
    report.records[1].review.waypoint_contract_reasons = vec!["cross_track".to_owned()];
    report.records[1].review.waypoint_closest_distance_m = Some(60.0);
    report.records[1].review.waypoint_cross_track_m = Some(52.0);
    report.records[1].review.waypoint_outbound_heading_error_rad = Some(1.4);
    report.records[1].review.waypoint_outbound_progress_mps = Some(3.0);
    for (record, (ratio, recoverable)) in
        report.records.iter_mut().zip([(0.72, true), (1.08, false)])
    {
        record.review.waypoint_handoffs = vec![crate::BatchWaypointHandoffReviewMetrics {
            waypoint_index: 0,
            final_terminal_required_accel_ratio: Some(ratio),
            final_terminal_recoverable: Some(recoverable),
            ..crate::BatchWaypointHandoffReviewMetrics::default()
        }];
    }

    let html = render_batch_report(
        Path::new("outputs/eval/waypoint_triage_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("<h2>Waypoint Handoff Triage</h2>"));
    assert!(html.contains("1 spatial"));
    assert!(html.contains("landed with waypoint warning"));
    assert!(html.contains("<th>Profile</th>"));
    assert!(html.contains("<th>Plan</th>"));
    assert!(html.contains("<th>Terminal Recovery</th>"));
    assert!(html.contains("1/2 recoverable"));
    assert!(html.contains("plan accel max 1.08x"));
    assert!(html.contains("kinematic estimate"));
    assert!(html.contains("single_bend_v1"));
    assert!(html.contains("continuation_pass_through_v1"));
    assert!(html.contains("p 0.55"));
    assert!(html.contains("n +0.20R"));
    assert!(html.contains("turn -43.9deg"));
    assert!(html.contains("vmax 65.0m/s"));
    assert!(html.contains("stop max 0.52"));
    assert!(html.contains("Heading Error"));
    assert!(html.contains("Handoff Progress"));
    assert!(!html.contains(r#"data-kind="waypoint profile""#));
    assert!(!html.contains("<h2>Waypoint Sequence</h2>"));
}

#[test]
fn waypoint_sequence_renders_route_summary_and_each_handoff() {
    let mut report = synthetic_transfer_shape_report(
        "waypoint_sequence_unit",
        &[("r00", "empty", 20.0, 0), ("r00", "empty", 22.0, 1)],
    );
    for (index, record) in report.records.iter_mut().enumerate() {
        let route_pass = index == 0;
        record.resolved.selector.waypoint_profile = "double_bend_v1".to_owned();
        record.resolved.selector.waypoint_handoff_envelope = "sequence_pass_through_v1".to_owned();
        for (waypoint_index, progress_frac, max_speed_mps, stop_ratio) in
            [(0, 0.33, 55.0, 0.55), (1, 0.67, 65.0, 0.65)]
        {
            for (metric, value) in [
                ("profile_progress_frac", progress_frac),
                ("route_signed_offset_ratio", 0.20),
                ("signed_turn_angle_deg", -31.2184),
                ("max_speed_mps", max_speed_mps),
                ("continuation_stop_ratio", stop_ratio),
            ] {
                record
                    .resolved
                    .resolved_parameters
                    .insert(format!("waypoint_{waypoint_index}_{metric}"), value);
            }
        }
        record.review.waypoint_route_status =
            Some(if route_pass { "pass" } else { "failed" }.to_owned());
        record.review.waypoint_route_passed = Some(if route_pass { 2 } else { 1 });
        record.review.waypoint_route_total = Some(2);
        record.review.waypoint_route_first_failure_index = (!route_pass).then_some(1);
        record.review.waypoint_handoffs = vec![
            crate::BatchWaypointHandoffReviewMetrics {
                waypoint_index: 0,
                waypoint_id: Some("wp_0".to_owned()),
                capture_status: Some("captured".to_owned()),
                contract_status: Some("pass".to_owned()),
                capture_time_s: Some(8.0 + index as f64),
                window_entry: Some(crate::BatchWaypointWindowEntryReviewMetrics {
                    time_s: Some(7.5 + index as f64),
                    contract_pass: Some(false),
                    contract_reasons: vec!["heading".to_owned()],
                    ..crate::BatchWaypointWindowEntryReviewMetrics::default()
                }),
                resolution_reason: Some("contract_pass".to_owned()),
                window_duration_s: Some(0.5),
                cross_track_m: Some(5.0),
                outbound_heading_error_rad: Some(0.2),
                speed_mps: Some(42.0),
                turn_margin_m: Some(30.0),
                target_velocity_error_mps: Some(8.0 + index as f64),
                target_deadline_remaining_s: Some(1.5),
                guidance_feasible: Some(true),
                predicted_handoff_time_to_go_s: Some(0.02),
                predicted_handoff_deadline_lead_s: Some(1.48),
                predicted_handoff_contract_status: Some("pass".to_owned()),
                candidate_contract_pass_ever: Some(true),
                candidate_first_pass_time_s: Some(6.0 + index as f64),
                candidate_last_pass_time_s: Some(7.5 + index as f64),
                candidate_pass_lost_before_capture: Some(false),
                candidate_best_heading_margin_rad: Some(0.1),
                candidate_best_cross_speed_margin_mps: Some(3.0),
                continuation_next_waypoint_index: Some(1),
                continuation_contract_pass: Some(true),
                continuation_outbound_heading_error_rad: Some(0.18),
                continuation_required_accel_ratio_max: Some(0.82),
                continuation_passing_candidate_count: Some(4),
                transition_next_waypoint_index: Some(1),
                transition_position_error_m: Some(6.0 + index as f64),
                transition_velocity_error_mps: Some(2.5),
                transition_attitude_error_rad: Some(0.08),
                transition_mass_error_kg: Some(1.2),
                transition_fuel_error_kg: Some(0.4),
                transition_event_time_error_s: Some(0.15),
                transition_continuation_contract_pass: Some(route_pass),
                transition_continuation_outbound_heading_error_rad: Some(0.22),
                transition_continuation_required_accel_ratio_max: Some(0.91),
                transition_continuation_passing_candidate_count: Some(if route_pass {
                    2
                } else {
                    0
                }),
                joint_next_waypoint_index: Some(1),
                joint_evaluated_candidate_count: Some(4),
                joint_passing_candidate_count: Some(if route_pass { 2 } else { 0 }),
                joint_contract_pass: Some(route_pass),
                joint_endpoint_x_m: Some(-120.0),
                joint_endpoint_y_m: Some(80.0),
                joint_target_vx_mps: Some(30.0),
                joint_target_vy_mps: Some(12.0),
                joint_time_to_go_s: Some(5.5),
                joint_continuation_outbound_heading_error_rad: Some(0.15),
                joint_required_accel_ratio_max: Some(0.88),
                joint_total_saturated_time_s: Some(0.2),
                joint_continuation_passing_candidate_count: Some(3),
                plan_reference_position_error_max_m: Some(18.0 + index as f64),
                plan_reference_cross_error_max_abs_m: Some(7.0),
                plan_reference_velocity_error_max_mps: Some(5.0),
                plan_reference_cross_speed_error_max_abs_mps: Some(2.0),
                guidance_required_accel_ratio_max: Some(1.2),
                guidance_thrust_saturated_time_s: Some(0.5),
                guidance_tilt_saturated_time_s: Some(0.1),
                guidance_first_saturation_lead_s: Some(1.0),
                last_pass_reference_position_error_m: Some(12.0),
                last_pass_reference_velocity_error_mps: Some(3.0),
                last_pass_required_accel_ratio: Some(0.9),
                guidance_plan_revision_max: Some(1),
                guidance_plan_reasons: vec!["initial".to_owned()],
                handoff_turn_margin_m: Some(-12.0),
                guidance_replan_count: Some(1),
                ..crate::BatchWaypointHandoffReviewMetrics::default()
            },
            crate::BatchWaypointHandoffReviewMetrics {
                waypoint_index: 1,
                waypoint_id: Some("wp_1".to_owned()),
                capture_status: Some(if route_pass { "captured" } else { "missed" }.to_owned()),
                contract_status: Some(if route_pass { "pass" } else { "spatial_miss" }.to_owned()),
                capture_time_s: Some(14.0 + index as f64),
                window_entry: Some(crate::BatchWaypointWindowEntryReviewMetrics {
                    time_s: Some(13.5 + index as f64),
                    contract_pass: Some(route_pass),
                    ..crate::BatchWaypointWindowEntryReviewMetrics::default()
                }),
                resolution_reason: Some(
                    if route_pass {
                        "contract_pass"
                    } else {
                        "plane_deadline"
                    }
                    .to_owned(),
                ),
                window_duration_s: Some(0.5),
                cross_track_m: Some(if route_pass { 6.0 } else { 55.0 }),
                outbound_heading_error_rad: Some(0.3),
                speed_mps: Some(38.0),
                turn_margin_m: Some(18.0),
                target_velocity_error_mps: Some(16.0 + index as f64),
                target_deadline_remaining_s: Some(0.4),
                guidance_feasible: Some(route_pass),
                final_terminal_required_accel_ratio: Some(if route_pass { 0.82 } else { 1.12 }),
                final_terminal_recoverable: Some(route_pass),
                predicted_handoff_time_to_go_s: Some(0.02),
                predicted_handoff_deadline_lead_s: Some(0.38),
                predicted_handoff_contract_status: Some(
                    if route_pass { "pass" } else { "fail" }.to_owned(),
                ),
                candidate_contract_pass_ever: Some(route_pass),
                candidate_first_pass_time_s: route_pass.then_some(12.0),
                candidate_last_pass_time_s: route_pass.then_some(13.0),
                candidate_pass_lost_before_capture: Some(false),
                candidate_best_heading_margin_rad: Some(if route_pass { 0.05 } else { -0.1 }),
                candidate_best_cross_speed_margin_mps: Some(if route_pass { 2.0 } else { -4.0 }),
                reachable_candidate_contract_pass_ever: Some(false),
                reachable_candidate_pass_lost_before_capture: Some(false),
                reachable_required_accel_ratio_max: Some(1.6),
                reachable_thrust_saturated_time_max_s: Some(0.6),
                reachable_tilt_saturated_time_max_s: Some(0.2),
                plan_reference_position_error_max_m: Some(30.0 + index as f64),
                plan_reference_cross_error_max_abs_m: Some(12.0),
                plan_reference_velocity_error_max_mps: Some(8.0),
                plan_reference_cross_speed_error_max_abs_mps: Some(4.0),
                guidance_required_accel_ratio_max: Some(1.8),
                guidance_thrust_saturated_time_s: Some(1.5),
                guidance_tilt_saturated_time_s: Some(0.4),
                guidance_first_saturation_lead_s: Some(2.0),
                last_pass_reference_position_error_m: route_pass.then_some(20.0),
                last_pass_reference_velocity_error_mps: route_pass.then_some(6.0),
                last_pass_required_accel_ratio: route_pass.then_some(1.1),
                guidance_plan_revision_max: Some(2),
                guidance_plan_reasons: vec!["initial".to_owned(), "authority_recovery".to_owned()],
                handoff_turn_margin_m: Some(-24.0),
                guidance_replan_count: Some(2),
                ..crate::BatchWaypointHandoffReviewMetrics::default()
            },
        ];
    }

    let html = render_batch_report(
        Path::new("outputs/eval/waypoint_sequence_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("<h2>Waypoint Sequence</h2>"));
    assert!(html.contains("1/2 pass"));
    assert!(html.contains("<code>#1</code> wp_0"));
    assert!(html.contains("<code>#2</code> wp_1"));
    assert!(html.contains("sequence_pass_through_v1"));
    assert!(html.contains("p 0.33"));
    assert!(html.contains("p 0.67"));
    assert!(html.contains("turn -31.2deg"));
    assert!(html.contains("stop max 0.65"));
    assert!(html.contains("1 spatial"));
    assert!(html.contains("2 recovered"));
    assert!(html.contains("1 deadline"));
    assert!(html.contains("Entry / Resolution"));
    assert!(html.contains("Tangent Error"));
    assert!(html.contains("<th>State Debt</th>"));
    assert!(html.contains("<th>Terminal Recovery</th>"));
    assert!(html.contains("1/2 recoverable"));
    assert!(html.contains("plan accel max 1.12x"));
    assert!(html.contains("Δv 8.5m/s"));
    assert!(html.contains("feasible 2/2"));
    assert!(html.contains("predict 2/2 pass"));
    assert!(html.contains("predict 1/2 pass"));
    assert!(html.contains("history 2/2 ever pass"));
    assert!(html.contains("history 1/2 ever pass"));
    assert!(html.contains("best margins heading"));
    assert!(html.contains("<h2>Waypoint Plan Trackability</h2>"));
    assert!(html.contains("actuated reachable forecast"));
    assert!(html.contains("<th>Continuation</th>"));
    assert!(html.contains("passing candidates 4.00"));
    assert!(html.contains("<h2>Waypoint Continuation Audit</h2>"));
    assert!(html.contains("<h3>Seed Evidence</h3>"));
    assert!(html.contains("Transition Error"));
    assert!(html.contains("Actual Continuation"));
    assert!(html.contains("Joint Search"));
    assert!(html.contains("eval 4.00"));
    assert!(html.contains("endpoint (-120.00m, 80.00m)"));
    assert!(html.contains("pass 0/2"));
    assert!(html.contains("disagree 1"));
    assert!(html.contains("peak 1.20x"));
    assert!(html.contains("authority_recovery"));

    for record in &mut report.records {
        let first = &mut record.review.waypoint_handoffs[0];
        first.transition_next_waypoint_index = None;
        first.joint_next_waypoint_index = None;
    }
    let legacy_html = render_batch_report(
        Path::new("outputs/eval/waypoint_sequence_unit"),
        &report,
        None,
        None,
    );
    assert!(legacy_html.contains("<h2>Waypoint Continuation Audit</h2>"));
    assert!(legacy_html.contains("legacy schema or no transition"));
}

#[test]
fn waypoint_terminal_recovery_cell_marks_missing_legacy_evidence() {
    assert_eq!(
        render_waypoint_terminal_recovery_cell(0, 0, None),
        r#"<span class="muted">-</span>"#
    );
}

#[test]
fn waypoint_checkpoint_failure_detail_exposes_limits_and_pass_loss() {
    let report = synthetic_transfer_shape_report(
        "waypoint_checkpoint_detail_unit",
        &[("r-30", "full", 20.0, 0)],
    );
    let mut record = report.records[0].clone();
    record.manifest.mission_outcome = pd_core::MissionOutcome::FailedCheckpoint;
    record.manifest.end_reason = pd_core::EndReason::CheckpointFailed;
    record.review.waypoint_route_first_failure_index = Some(1);
    for (metric, value) in [
        ("max_outbound_heading_error_rad", 0.35),
        ("max_outbound_cross_speed_mps", 20.0),
    ] {
        record
            .resolved
            .resolved_parameters
            .insert(format!("waypoint_1_{metric}"), value);
    }
    record.review.waypoint_handoffs = vec![crate::BatchWaypointHandoffReviewMetrics {
        waypoint_index: 1,
        contract_status: Some("outbound_out_of_envelope".to_owned()),
        contract_reasons: vec!["heading".to_owned(), "outbound_cross_speed".to_owned()],
        outbound_heading_error_rad: Some(0.36521479774317),
        outbound_cross_speed_mps: Some(-21.25),
        reachable_candidate_pass_lost_before_capture: Some(true),
        ..crate::BatchWaypointHandoffReviewMetrics::default()
    }];

    assert_eq!(
        waypoint_checkpoint_failure_detail(&record).as_deref(),
        Some(
            "WP2 heading 20.93deg > 20.05deg (+0.87deg) · cross speed 21.25m/s > 20.00m/s (+1.25m/s) · reachable pass lost"
        )
    );

    record.resolved.resolved_parameters.clear();
    assert_eq!(waypoint_checkpoint_failure_detail(&record), None);
}

#[test]
fn waypoint_checkpoint_failure_detail_falls_back_to_spatial_status() {
    let report = synthetic_transfer_shape_report(
        "waypoint_checkpoint_spatial_unit",
        &[("r00", "empty", 20.0, 0)],
    );
    let mut record = report.records[0].clone();
    record.manifest.mission_outcome = pd_core::MissionOutcome::FailedCheckpoint;
    record.review.waypoint_route_first_failure_index = Some(0);
    record
        .resolved
        .resolved_parameters
        .insert("waypoint_0_capture_radius_m".to_owned(), 64.0);
    record.review.waypoint_handoffs = vec![crate::BatchWaypointHandoffReviewMetrics {
        waypoint_index: 0,
        contract_status: Some("spatial_miss".to_owned()),
        closest_distance_m: Some(70.25),
        ..crate::BatchWaypointHandoffReviewMetrics::default()
    }];

    assert_eq!(
        waypoint_checkpoint_failure_detail(&record).as_deref(),
        Some("WP1 spatial miss: closest 70.25m, capture 64.00m")
    );
}

#[test]
fn waypoint_triage_sorts_landed_misses_before_clean_captures() {
    let mut report = synthetic_transfer_shape_report(
        "waypoint_triage_sort_unit",
        &[("r+60", "empty", 35.0, 0), ("r+80", "empty", 40.0, 1)],
    );
    report.records[0].review.waypoint_capture_status = Some("captured".to_owned());
    report.records[0].review.waypoint_contract_status = Some("pass".to_owned());
    report.records[0].review.waypoint_closest_distance_m = Some(16.0);
    report.records[0].review.waypoint_cross_track_m = Some(7.0);
    report.records[0].review.waypoint_outbound_heading_error_rad = Some(0.2);
    report.records[0].review.waypoint_outbound_progress_mps = Some(24.0);
    report.records[1].review.waypoint_capture_status = Some("missed".to_owned());
    report.records[1].review.waypoint_contract_status = Some("spatial_miss".to_owned());
    report.records[1].review.waypoint_contract_reasons = vec!["cross_track".to_owned()];
    report.records[1].review.waypoint_closest_distance_m = Some(120.0);
    report.records[1].review.waypoint_cross_track_m = Some(88.0);
    report.records[1].review.waypoint_outbound_heading_error_rad = Some(2.4);
    report.records[1].review.waypoint_outbound_progress_mps = Some(-12.0);

    let html = render_batch_report(
        Path::new("outputs/eval/waypoint_triage_sort_unit"),
        &report,
        None,
        None,
    );

    let missed_index = html
        .find("<code>r+80</code>")
        .expect("missed waypoint route row should render");
    let clean_index = html
        .find("<code>r+60</code>")
        .expect("captured waypoint route row should render");
    assert!(missed_index < clean_index);
    assert!(html.contains("landed with waypoint warning"));
}

#[test]
fn waypoint_review_tree_groups_multiple_profiles_in_defined_order() {
    let mut report = synthetic_transfer_shape_report(
        "waypoint_profile_tree_unit",
        &[("r00", "empty", 20.0, 7), ("r00", "empty", 22.0, 9)],
    );
    report.records[0].resolved.selector.waypoint_profile = "single_sharp_bend_v1".to_owned();
    report.records[1].resolved.selector.waypoint_profile = "single_gentle_bend_v1".to_owned();

    let grouped = records_by_waypoint_profile(&report.records.iter().collect::<Vec<_>>());
    assert_eq!(grouped["single_sharp_bend_v1"][0].resolved.resolved_seed, 7);
    assert_eq!(
        grouped["single_gentle_bend_v1"][0].resolved.resolved_seed,
        9
    );

    let html = render_batch_report(
        Path::new("outputs/eval/waypoint_profile_tree_unit"),
        &report,
        None,
        None,
    );

    assert_eq!(html.matches(r#"data-kind="waypoint profile""#).count(), 2);
    let gentle_index = html
            .find(
                r#"selector-inline">waypoint profile</span> <span class="selector-code">single_gentle_bend_v1</span>"#,
            )
            .expect("gentle waypoint profile row should render");
    let sharp_index = html
            .find(
                r#"selector-inline">waypoint profile</span> <span class="selector-code">single_sharp_bend_v1</span>"#,
            )
            .expect("sharp waypoint profile row should render");
    assert!(gentle_index < sharp_index);
}

#[test]
fn standalone_report_prefers_current_lane_context() {
    let pack = ScenarioPackSpec {
        id: "lane_compare_unit".to_owned(),
        name: "Lane compare unit".to_owned(),
        description: "lane compare unit".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![
            terminal_family_entry(
                "terminal_guidance_clean_nominal_baseline",
                "terminal_guidance_clean_nominal",
                "baseline",
            ),
            terminal_family_entry(
                "terminal_guidance_clean_nominal_staged",
                "terminal_guidance_clean_nominal",
                "staged",
            ),
            checkpoint_entry(),
        ],
    };

    let mut report = run_pack_with_workers(&pack, &fixtures_root(), None, 1).unwrap();
    let html = render_batch_report(
        Path::new("outputs/eval/lane_compare_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("<h2>Context</h2>"));
    assert!(html.contains(r#"<details class="header-context">"#));
    assert!(!html.contains(r#"<details class="header-context attention" open>"#));
    assert!(html.contains("Report Mode"));
    assert!(html.contains("standalone"));
    assert!(html.contains("current controller lane <code>staged</code>"));
    assert!(html.contains("controller <code>staged_descent_v1</code>"));
    assert!(html.contains("Compare Basis"));
    assert!(html.contains("none"));
    assert!(html.contains("Scope Resolution"));
    assert!(html.contains("current controller lane"));
    assert!(html.contains("Compare Status"));
    assert!(html.contains("standalone"));
    assert!(html.contains("Cache / Promotion"));
    assert!(html.contains("not cached"));
    assert!(!html.contains("data-view-mode=\"compare\""));
    assert!(!html.contains("baseline controller lane <code>baseline</code>"));
    assert!(!html.contains("<h2>Transfer Handoff Triage</h2>"));
    assert!(!html.contains("<h2>Transfer Shape Triage</h2>"));
    assert!(
        !html.contains(
            r#"selector-inline">lane</span> <span class="selector-code">baseline</span>"#
        )
    );

    report.provenance.compare.source = crate::BatchCompareSource::ExplicitDir;
    report.provenance.compare.status = crate::BatchCompareResolutionStatus::Missing;
    let attention_html = render_batch_report(
        Path::new("outputs/eval/lane_compare_unit"),
        &report,
        None,
        None,
    );
    assert!(attention_html.contains(r#"<details class="header-context attention" open>"#));
}

#[test]
fn external_compare_report_renders_context_section() {
    let baseline_pack = ScenarioPackSpec {
        id: "compare_baseline_unit".to_owned(),
        name: "Compare baseline unit".to_owned(),
        description: "compare baseline unit".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![
            terminal_family_entry(
                "terminal_compare_baseline",
                "terminal_guidance_fixture_nominal",
                "staged",
            ),
            checkpoint_entry(),
        ],
    };
    let candidate_pack = ScenarioPackSpec {
        id: "compare_candidate_unit".to_owned(),
        name: "Compare candidate unit".to_owned(),
        description: "compare candidate unit".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![
            terminal_family_entry(
                "terminal_compare_baseline",
                "terminal_guidance_fixture_nominal",
                "staged",
            ),
            checkpoint_entry(),
        ],
    };

    let baseline_report = run_pack_with_workers(&baseline_pack, &fixtures_root(), None, 1).unwrap();
    let candidate_report =
        run_pack_with_workers(&candidate_pack, &fixtures_root(), None, 1).unwrap();
    let comparison = compare_batch_reports(&candidate_report, &baseline_report);
    let html = render_batch_report(
        Path::new("outputs/eval/compare_candidate_unit"),
        &candidate_report,
        Some((
            Path::new("outputs/eval/compare_baseline_unit"),
            &baseline_report,
        )),
        Some(&comparison),
    );

    assert!(html.contains("<h2>Context</h2>"));
    assert!(html.contains("Report Mode"));
    assert!(html.contains("current-lane history compare"));
    assert!(html.contains("data-view-mode=\"compare\""));
    assert!(html.contains("data-view-mode=\"current-only\""));
    assert!(html.contains("Baseline Source"));
    assert!(html.contains("compare_baseline_unit"));
    assert!(html.contains("Compare Basis"));
    assert!(html.contains("compare baseline from lane"));
    assert!(
        html.contains(
            "baseline here means the compare target, not the built-in baseline controller"
        )
    );
    assert!(html.contains("lane <code>staged</code>"));
    assert!(html.contains("shared 2"));
    assert!(html.contains("Scope Resolution"));
    assert!(html.contains("exact"));
    assert!(html.contains("external baseline report provided for this render"));
    assert!(html.contains("Compare Status"));
    assert!(html.contains("available"));
    assert!(html.contains("<h2>Regression Policy</h2>"));
    assert!(html.contains("Compare coverage"));
    assert!(html.contains(r#"baseline-only 0 · <span class="status-chip ok">pass</span>"#));
}

#[test]
fn terminal_matrix_report_renders_arc_and_band_levels() {
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_tree_unit".to_owned(),
        name: "Terminal matrix tree unit".to_owned(),
        description: "terminal matrix tree unit".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
            id: "terminal_guidance_clean_nominal".to_owned(),
            terminal_matrix: "half_arc_terminal_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![
                TerminalMatrixLaneSpec {
                    id: "baseline".to_owned(),
                    controller: "baseline".to_owned(),
                    controller_config: None,
                },
                TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "staged".to_owned(),
                    controller_config: None,
                },
            ],
            seed_tier: TerminalSeedTier::Smoke,
            condition_set: "clean".to_owned(),
            vehicle_variant: "nominal".to_owned(),
            expectation_tier: "core".to_owned(),
            arc_points: Vec::new(),
            adjustments: Vec::new(),
            tags: vec!["terminal".to_owned(), "bot_lab".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let report = run_pack_with_workers(&pack, &fixtures_root(), None, 1).unwrap();
    let html = render_batch_report(
        Path::new("outputs/eval/terminal_matrix_tree_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("selector-inline\">arc</span>"));
    assert!(html.contains("selector-inline\">band</span>"));
    assert!(html.contains("selector-code\">a00</span>"));
    assert!(html.contains("selector-code\">low</span>"));
    assert!(html.contains("<h2>Coverage</h2>"));
    assert!(html.contains("Energy band by arrival arc"));
    assert!(html.contains(r#"data-tree-tokens="terminal_guidance|clean|nominal|a00|low""#));
    let tree_section_css = html
        .split_once(".tree-table-section {")
        .and_then(|(_, css)| css.split_once('}'))
        .map(|(css, _)| css)
        .expect("tree table section css should render");
    assert!(tree_section_css.contains("min-width: 0"));
    assert!(tree_section_css.contains("max-width: 100%"));

    let condition_pos = html
        .find(r#"selector-inline">condition</span> <span class="selector-code">clean</span>"#)
        .expect("condition row should render");
    let arc_pos = html
        .find(r#"selector-inline">arc</span> <span class="selector-code">a00</span>"#)
        .expect("arc row should render");
    let band_pos = html
        .find(r#"selector-inline">band</span> <span class="selector-code">low</span>"#)
        .expect("band row should render");
    let vehicle_pos = html
        .find(r#"selector-inline">vehicle</span> <span class="selector-code">nominal</span>"#)
        .expect("vehicle row should render");

    assert!(
        condition_pos < arc_pos && arc_pos < band_pos && band_pos < vehicle_pos,
        "terminal matrix tree should render condition -> arc -> band -> vehicle"
    );
    assert!(html.contains(r#"case "band": return 4;"#));
    assert!(html.contains(r#"case "vehicle": return 5;"#));
}

#[test]
fn transfer_matrix_report_renders_route_and_radius_levels() {
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_tree_unit".to_owned(),
        name: "Transfer matrix tree unit".to_owned(),
        description: "transfer matrix tree unit".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
            id: "transfer_guidance_clean_nominal".to_owned(),
            transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TransferMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "transfer_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TransferSeedTier::Smoke,
            vehicle_variant: "nominal".to_owned(),
            expectation_tier: "core".to_owned(),
            route_angles: vec!["r00".to_owned()],
            radius_tiers: Vec::new(),
            waypoint_profile: None,
            waypoint_handoff_envelope: None,
            evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
            adjustments: Vec::new(),
            tags: vec!["transfer".to_owned(), "bot_lab".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let report = run_pack_with_workers(&pack, &fixtures_root(), None, 1).unwrap();
    let html = render_batch_report(
        Path::new("outputs/eval/transfer_matrix_tree_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("selector-inline\">route</span>"));
    assert!(html.contains("selector-inline\">radius</span>"));
    assert!(html.contains("selector-code\">r00</span>"));
    assert!(html.contains("selector-code\">nominal</span>"));
    assert!(html.contains("<h2>Coverage</h2>"));
    assert!(html.contains("Travel radius by route angle"));
    assert!(html.contains(r#"data-tree-tokens="transfer_guidance|clean|nominal|r00|nominal""#));
    assert!(html.contains(r#"case "route": return 3;"#));
    assert!(html.contains(r#"case "radius": return 4;"#));
    assert!(html.contains("transfer terminal"));
    assert!(html.contains("handoff"));
}

#[test]
fn transfer_shape_triage_renders_for_transfer_records_without_standalone_deltas() {
    let report =
        synthetic_transfer_shape_report("transfer_shape_unit", &[("r00", "empty", 24.0, 0)]);
    let html = render_batch_report(
        Path::new("outputs/eval/transfer_shape_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("<h2>Transfer Shape Triage</h2>"));
    assert!(html.contains("<h2>Guidance Diagnostics</h2>"));
    assert!(html.contains(r#"<details class="transfer-shape-section">"#));
    assert!(!html.contains(r#"<details class="transfer-shape-section" open"#));
    assert!(html.contains("Visual-shape diagnostic"));
    assert!(html.contains("Shape RMSE"));
    assert!(html.contains("Worst Seed"));
    assert!(html.contains(r#"data-transfer-shape-cell="clean|empty|r00|nominal""#));
    assert!(!html.contains("Δ Shape"));
    assert!(!html.contains("Δ Success"));
}

#[test]
fn transfer_handoff_triage_renders_quality_columns() {
    let report =
        synthetic_transfer_shape_report("transfer_handoff_unit", &[("r00", "empty", 24.0, 0)]);
    let html = render_batch_report(
        Path::new("outputs/eval/transfer_handoff_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("<h2>Transfer Handoff Triage</h2>"));
    assert!(html.contains(r#"<details class="transfer-handoff-section">"#));
    assert!(!html.contains(r#"<details class="transfer-handoff-section" open"#));
    assert!(html.contains("Entry / Gate"));
    assert!(html.contains("Handoff Height"));
    assert!(html.contains("Handoff Speed"));
    assert!(html.contains("Handoff pdx"));
    assert!(html.contains("Cutoff pdx"));
    assert!(html.contains("Terminal Rebound"));
    assert!(html.contains("origin dx 4.00m"));
    assert!(html.contains(r#"data-transfer-handoff-cell="clean|empty|r00|nominal""#));
    assert!(html.contains("terminal handoff"));
    assert!(html.contains("handoff gate ready"));
    assert!(html.contains("24.00m/s"));
    assert!(html.contains("58.0deg"));
}

#[test]
fn transfer_handoff_triage_labels_direct_terminal_entries() {
    let mut report = synthetic_transfer_shape_report(
        "transfer_handoff_direct_unit",
        &[("r-80", "empty", 8.0, 0)],
    );
    report.records[0].review.transfer_terminal_entry_kind = Some("direct".to_owned());
    report.records[0].review.transfer_terminal_handoff_gate_mode = Some("latest_safe".to_owned());
    report.records[0].review.transfer_boost_cutoff_quality = None;
    report.records[0]
        .review
        .transfer_boost_cutoff_projected_dx_m = None;
    report.records[0]
        .review
        .transfer_boost_cutoff_impact_angle_deg = None;
    let html = render_batch_report(
        Path::new("outputs/eval/transfer_handoff_direct_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("<h2>Transfer Handoff Triage</h2>"));
    assert!(html.contains("terminal direct"));
    assert!(html.contains("terminal gate latest_safe"));
    assert!(html.contains(r#"data-transfer-handoff-cell="clean|empty|r-80|nominal""#));
}

#[test]
fn transfer_handoff_triage_sorts_risky_cells_before_clean_cells() {
    let report = synthetic_transfer_shape_report(
        "transfer_handoff_sort_unit",
        &[("r00", "empty", 12.0, 0), ("r+45", "full", 88.0, 1)],
    );
    let mut records = report.records.clone();
    records[0].review.transfer_terminal_handoff_height_m = Some(180.0);
    records[0].review.transfer_terminal_handoff_speed_mps = Some(18.0);
    records[0].review.transfer_terminal_handoff_projected_dx_m = Some(12.0);
    records[0].review.transfer_boost_cutoff_projected_dx_m = Some(10.0);
    records[1].review.transfer_terminal_handoff_height_m = Some(8.0);
    records[1].review.transfer_terminal_handoff_speed_mps = Some(58.0);
    records[1].review.transfer_terminal_handoff_projected_dx_m = Some(170.0);
    records[1].review.transfer_terminal_handoff_impact_angle_deg = Some(34.0);
    records[1].review.transfer_boost_cutoff_quality = Some("dx".to_owned());
    records[1].review.transfer_boost_cutoff_projected_dx_m = Some(190.0);
    let report = report_with_records(report, records);
    let html = render_batch_report(
        Path::new("outputs/eval/transfer_handoff_sort_unit"),
        &report,
        None,
        None,
    );

    let risky_pos = html
        .find(r#"data-transfer-handoff-cell="clean|full|r+45|nominal""#)
        .expect("risky handoff cell should render");
    let clean_pos = html
        .find(r#"data-transfer-handoff-cell="clean|empty|r00|nominal""#)
        .expect("clean handoff cell should render");
    assert!(
        risky_pos < clean_pos,
        "handoff triage rows should sort risky cells before clean cells"
    );
    assert!(html.contains("triage-risk"));
}

#[test]
fn transfer_handoff_triage_sorts_successful_cells_by_near_pad_rebound() {
    let report = synthetic_transfer_shape_report(
        "transfer_handoff_climb_sort_unit",
        &[("r00", "empty", 90.0, 0), ("r+30", "empty", 12.0, 1)],
    );
    let mut records = report.records.clone();
    records[0]
        .review
        .transfer_terminal_low_altitude_rebound_gain_m = Some(8.0);
    records[1]
        .review
        .transfer_terminal_low_altitude_rebound_gain_m = Some(75.0);
    let report = report_with_records(report, records);
    let html = render_batch_report(
        Path::new("outputs/eval/transfer_handoff_climb_sort_unit"),
        &report,
        None,
        None,
    );

    let high_pos = html
        .find(r#"data-transfer-handoff-cell="clean|empty|r+30|nominal""#)
        .expect("high-climb handoff cell should render");
    let low_pos = html
        .find(r#"data-transfer-handoff-cell="clean|empty|r00|nominal""#)
        .expect("low-climb handoff cell should render");
    assert!(high_pos < low_pos);
}

#[test]
fn transfer_handoff_triage_uses_highest_near_pad_rebound_as_worst_successful_seed() {
    let report = synthetic_transfer_shape_report(
        "transfer_handoff_climb_seed_unit",
        &[("r00", "empty", 12.0, 0), ("r00", "empty", 14.0, 1)],
    );
    let mut records = report.records.clone();
    records[0]
        .review
        .transfer_terminal_low_altitude_rebound_gain_m = Some(10.0);
    records[1]
        .review
        .transfer_terminal_low_altitude_rebound_gain_m = Some(90.0);
    let report = report_with_records(report, records);
    let html = render_batch_report(
        Path::new("outputs/eval/transfer_handoff_climb_seed_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("seed 0001"));
    assert!(html.contains("rebound 90m near pad"));
}

#[test]
fn transfer_handoff_triage_does_not_rank_far_recovery_as_near_pad_rebound() {
    let report = synthetic_transfer_shape_report(
        "transfer_handoff_far_recovery_unit",
        &[("r00", "empty", 12.0, 0), ("r00", "empty", 14.0, 1)],
    );
    let mut records = report.records.clone();
    records[0]
        .review
        .transfer_terminal_low_altitude_rebound_gain_m = Some(8.0);
    records[0]
        .review
        .transfer_terminal_low_altitude_rebound_near_pad = Some(true);
    records[1]
        .review
        .transfer_terminal_low_altitude_rebound_gain_m = Some(90.0);
    records[1]
        .review
        .transfer_terminal_low_altitude_rebound_origin_dx_abs_m = Some(200.0);
    records[1]
        .review
        .transfer_terminal_low_altitude_rebound_near_pad = Some(false);
    let report = report_with_records(report, records);
    let html = render_batch_report(
        Path::new("outputs/eval/transfer_handoff_far_recovery_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("seed 0000"));
    assert!(html.contains("rebound 8m near pad"));
}

#[test]
fn transfer_shape_triage_sorts_by_worst_successful_shape_rmse() {
    let report = synthetic_transfer_shape_report(
        "transfer_shape_sort_unit",
        &[("r00", "empty", 12.0, 0), ("r+60", "empty", 88.0, 1)],
    );
    let html = render_batch_report(
        Path::new("outputs/eval/transfer_shape_sort_unit"),
        &report,
        None,
        None,
    );

    let high_pos = html
        .find(r#"data-transfer-shape-cell="clean|empty|r+60|nominal""#)
        .expect("higher RMSE cell should render");
    let low_pos = html
        .find(r#"data-transfer-shape-cell="clean|empty|r00|nominal""#)
        .expect("lower RMSE cell should render");
    assert!(
        high_pos < low_pos,
        "transfer shape rows should sort by descending worst successful RMSE"
    );
}

#[test]
fn transfer_shape_triage_renders_compare_deltas_for_matching_cells() {
    let candidate = synthetic_transfer_shape_report(
        "transfer_shape_candidate_unit",
        &[("r+60", "empty", 50.0, 0), ("r00", "empty", 10.0, 1)],
    );
    let baseline = synthetic_transfer_shape_report(
        "transfer_shape_baseline_unit",
        &[("r+60", "empty", 20.0, 0), ("r00", "empty", 10.0, 1)],
    );
    let comparison = compare_batch_reports(&candidate, &baseline);
    let html = render_batch_report(
        Path::new("outputs/eval/transfer_shape_candidate_unit"),
        &candidate,
        Some((
            Path::new("outputs/eval/transfer_shape_baseline_unit"),
            &baseline,
        )),
        Some(&comparison),
    );

    assert!(html.contains("Δ Shape"));
    assert!(html.contains("Δ Success"));
    assert!(html.contains("+30.00m"));
    assert!(html.contains("+0.0 pp"));
}

#[test]
fn tree_group_ids_preserve_signed_route_labels() {
    assert_ne!(
        tree_group_id(&["arc", "r-30"]),
        tree_group_id(&["arc", "r+30"])
    );
}

#[test]
fn terminal_matrix_report_surfaces_impossible_runs_as_warnings() {
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_heavy_cargo_unit".to_owned(),
        name: "Terminal matrix heavy cargo unit".to_owned(),
        description: "terminal matrix heavy cargo unit".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
            id: "terminal_guidance_clean_heavy_cargo".to_owned(),
            terminal_matrix: "half_arc_terminal_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TerminalMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "terminal_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TerminalSeedTier::Full,
            condition_set: "clean".to_owned(),
            vehicle_variant: "heavy_cargo".to_owned(),
            expectation_tier: "core".to_owned(),
            arc_points: Vec::new(),
            adjustments: vec![crate::NumericAdjustmentSpec {
                id: "payload_full_mass_kg".to_owned(),
                path: "vehicle.dry_mass_kg".to_owned(),
                mode: crate::NumericPerturbationMode::Offset,
                value: 4500.0,
            }],
            tags: vec!["terminal".to_owned(), "analytic".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let report = run_pack_with_workers(&pack, &fixtures_root(), None, 1).unwrap();
    let html = render_batch_report(
        Path::new("outputs/eval/terminal_matrix_heavy_cargo_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("impossible"));
    assert!(html.contains("impossible vertical brake"));
    assert!(html.contains("0 fail · <span class=\"warn\">12 warning</span>"));
}

#[test]
fn terminal_matrix_report_surfaces_frontier_runs_as_scored_annotations() {
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_frontier_unit".to_owned(),
        name: "Terminal matrix frontier unit".to_owned(),
        description: "terminal matrix frontier unit".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
            id: "terminal_guidance_clean_full_payload".to_owned(),
            terminal_matrix: "half_arc_terminal_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TerminalMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "terminal_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TerminalSeedTier::Full,
            condition_set: "clean".to_owned(),
            vehicle_variant: "full".to_owned(),
            expectation_tier: "stress".to_owned(),
            arc_points: Vec::new(),
            adjustments: vec![crate::NumericAdjustmentSpec {
                id: "payload_full_mass_kg".to_owned(),
                path: "vehicle.dry_mass_kg".to_owned(),
                mode: crate::NumericPerturbationMode::Offset,
                value: 4500.0,
            }],
            tags: vec!["terminal".to_owned(), "analytic".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let report = run_pack_with_workers(&pack, &fixtures_root(), None, 1).unwrap();
    let html = render_batch_report(
        Path::new("outputs/eval/terminal_matrix_frontier_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("low-thrust high-energy frontier"));
    assert!(html.contains(
        "<span class=\"outcome-bad\">failed_crash · low-thrust high-energy frontier</span>"
    ));
    assert!(
        !html
            .contains("<span class=\"warn\">failed_crash · low-thrust high-energy frontier</span>")
    );
}

#[test]
fn transfer_report_surfaces_near_vertical_frontier_as_scored_annotation() {
    let base_report = synthetic_transfer_shape_report(
        "transfer_route_frontier_unit",
        &[("r+80", "empty", 50.0, 0)],
    );
    let mut record = base_report.records[0].clone();
    record.manifest.physical_outcome = pd_core::PhysicalOutcome::Crashed;
    record.manifest.mission_outcome = pd_core::MissionOutcome::FailedCrash;
    record.manifest.end_reason = pd_core::EndReason::Crash;
    record.analytic.class = BatchRunAnalyticClass::Frontier;
    record.analytic.reason = Some(BatchRunAnalyticReason::NearVerticalTransferRoute);
    let report = report_with_records(base_report, vec![record]);

    let html = render_batch_report(
        Path::new("outputs/eval/transfer_route_frontier_unit"),
        &report,
        None,
        None,
    );

    assert!(html.contains("near-vertical transfer-route frontier"));
    assert!(html.contains(
        "<span class=\"outcome-bad\">failed_crash · near-vertical transfer-route frontier</span>"
    ));
    assert!(!html.contains(
        "<span class=\"warn\">failed_crash · near-vertical transfer-route frontier</span>"
    ));
}

#[test]
fn selector_keys_use_semantic_velocity_band_order() {
    let mut keys = vec![
        "high".to_owned(),
        "low".to_owned(),
        "mid".to_owned(),
        "unspecified".to_owned(),
    ];
    sort_selector_keys(&mut keys);
    assert_eq!(
        keys,
        vec![
            "low".to_owned(),
            "mid".to_owned(),
            "high".to_owned(),
            "unspecified".to_owned(),
        ]
    );
}

#[test]
fn selector_keys_use_semantic_radius_tier_order() {
    let mut keys = vec![
        "long".to_owned(),
        "nominal".to_owned(),
        "short".to_owned(),
        "unspecified".to_owned(),
    ];
    sort_selector_keys(&mut keys);
    assert_eq!(
        keys,
        vec![
            "short".to_owned(),
            "nominal".to_owned(),
            "long".to_owned(),
            "unspecified".to_owned(),
        ]
    );
}

#[test]
fn selector_keys_use_semantic_vehicle_variant_order() {
    let mut keys = vec![
        "full".to_owned(),
        "empty".to_owned(),
        "half".to_owned(),
        "unspecified".to_owned(),
    ];
    sort_selector_keys(&mut keys);
    assert_eq!(
        keys,
        vec![
            "empty".to_owned(),
            "half".to_owned(),
            "full".to_owned(),
            "unspecified".to_owned(),
        ]
    );
}

#[test]
fn selector_keys_use_semantic_condition_order() {
    let mut keys = vec![
        "traj_overshoot_large".to_owned(),
        "traj_undershoot_small".to_owned(),
        "clean".to_owned(),
        "traj_overshoot_small".to_owned(),
        "traj_undershoot_large".to_owned(),
        "terrain_clip".to_owned(),
        "terrain_backstop_slanted".to_owned(),
        "terrain_backstop_wall".to_owned(),
    ];
    sort_selector_keys(&mut keys);
    assert_eq!(
        keys,
        vec![
            "clean".to_owned(),
            "traj_undershoot_small".to_owned(),
            "traj_undershoot_large".to_owned(),
            "traj_overshoot_small".to_owned(),
            "traj_overshoot_large".to_owned(),
            "terrain_backstop_wall".to_owned(),
            "terrain_backstop_slanted".to_owned(),
            "terrain_clip".to_owned(),
        ]
    );
}
