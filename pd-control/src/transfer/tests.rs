use super::*;
use crate::controllers::{ControllerSpec, built_in_controller_spec};
use crate::terminal::TerminalEntryMode;
use pd_core::{
    EvaluationGoal, LandingPadSpec, MissionSpec, RunContext, ScenarioSpec, SimConfig,
    TerrainDefinition, TransferRouteSpec, TransferWaypointSpec, Vec2, VehicleGeometry,
    VehicleInitialState, VehicleSpec, WorldSpec,
};
use std::collections::BTreeMap;

fn transfer_observation(
    target_dx_m: f64,
    height_above_target_m: f64,
    velocity_mps: Vec2,
    sim_time_s: f64,
) -> Observation {
    Observation {
        sim_time_s,
        physics_step: 0,
        position_m: Vec2::new(0.0, height_above_target_m),
        velocity_mps,
        attitude_rad: 0.0,
        angular_rate_radps: 0.0,
        mass_kg: 1_000.0,
        fuel_kg: 100.0,
        gravity_mps2: 9.81,
        target_dx_m,
        height_above_target_m,
        target_surface_y_m: 0.0,
        target_pad_half_width_m: 18.0,
        touchdown_clearance_m: height_above_target_m.abs() + 100.0,
        min_hull_clearance_m: height_above_target_m.abs() + 100.0,
    }
}

fn transfer_gate_fixture(
    latest_safe_margin_s: f64,
    required_accel_ratio: f64,
    terrain_clearance_safe: bool,
) -> TerminalEntryAssessment {
    TerminalEntryAssessment {
        mode: TerminalEntryMode::Pending,
        ready_ticks: 0,
        burn_time_s: 5.0,
        latest_safe_margin_s,
        required_accel_ratio,
        terrain_min_clearance_m: 20.0,
        terrain_clearance_safe,
        deferred: false,
    }
}

fn uphill_transfer_context() -> RunContext {
    let scenario = ScenarioSpec {
        id: "uphill_transfer".to_owned(),
        name: "Uphill transfer".to_owned(),
        description: "uphill transfer controller unit fixture".to_owned(),
        seed: 1,
        tags: vec!["test".to_owned()],
        metadata: BTreeMap::new(),
        sim: SimConfig {
            physics_hz: 120,
            controller_hz: 60,
            max_time_s: 90.0,
            sample_hz: Some(10),
        },
        world: WorldSpec {
            gravity_mps2: 9.81,
            terrain: TerrainDefinition::Heightfield {
                points_m: vec![
                    Vec2::new(-180.0, -780.0),
                    Vec2::new(-140.0, -780.0),
                    Vec2::new(0.0, 0.0),
                    Vec2::new(180.0, 0.0),
                ],
            },
            landing_pads: vec![
                LandingPadSpec {
                    id: "source".to_owned(),
                    center_x_m: -140.0,
                    surface_y_m: -780.0,
                    width_m: 36.0,
                },
                LandingPadSpec {
                    id: "target".to_owned(),
                    center_x_m: 0.0,
                    surface_y_m: 0.0,
                    width_m: 36.0,
                },
            ],
        },
        vehicle: VehicleSpec {
            geometry: VehicleGeometry {
                hull_width_m: 4.0,
                hull_height_m: 6.0,
                touchdown_half_span_m: 2.0,
                touchdown_base_offset_m: 3.2,
            },
            dry_mass_kg: 700.0,
            initial_fuel_kg: 240.0,
            max_fuel_kg: 240.0,
            max_thrust_n: 16_000.0,
            max_fuel_burn_kgps: 11.0,
            min_throttle_frac: 0.0,
            max_rotation_rate_radps: 1.2,
            safe_touchdown_normal_speed_mps: 3.0,
            safe_touchdown_tangential_speed_mps: 2.0,
            safe_touchdown_attitude_error_rad: 0.15,
            safe_touchdown_angular_rate_radps: 0.35,
        },
        initial_state: VehicleInitialState {
            position_m: Vec2::new(-140.0, -780.0),
            velocity_mps: Vec2::new(0.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        },
        mission: MissionSpec {
            transfer_route: Some(TransferRouteSpec {
                source_pad_id: "source".to_owned(),
                target_pad_id: "target".to_owned(),
                route_angle_deg: 80.0,
                route_radius_m: 800.0,
                waypoints: Vec::new(),
            }),
            goal: EvaluationGoal::LandingOnPad {
                target_pad_id: "target".to_owned(),
            },
        },
    };
    RunContext::from_scenario(&scenario).unwrap()
}

fn waypoint_fixture() -> TransferWaypointSpec {
    TransferWaypointSpec {
        id: "wp_0".to_owned(),
        position_m: Vec2::new(-220.0, -300.0),
        handoff_tangent_unit: None,
        capture_radius_m: 35.0,
        max_cross_track_m: 45.0,
        max_outbound_heading_error_rad: 0.6,
        min_outbound_progress_mps: 5.0,
        max_outbound_cross_speed_mps: None,
        min_speed_mps: 10.0,
        max_speed_mps: 80.0,
        min_vertical_speed_mps: Some(-60.0),
        max_vertical_speed_mps: Some(60.0),
    }
}

fn context_with_waypoint() -> RunContext {
    context_with_waypoint_spec(waypoint_fixture())
}

fn context_with_waypoint_spec(waypoint: TransferWaypointSpec) -> RunContext {
    let mut ctx = uphill_transfer_context();
    ctx.mission
        .transfer_route
        .as_mut()
        .expect("transfer route")
        .waypoints = vec![waypoint];
    ctx
}

fn waypoint_transfer_observation(
    ctx: &RunContext,
    position_m: Vec2,
    velocity_mps: Vec2,
    sim_time_s: f64,
) -> Observation {
    let terrain_y_m = ctx.world.terrain.sample_height(position_m.x);
    let touchdown_clearance_m =
        position_m.y - terrain_y_m - ctx.vehicle.geometry.touchdown_base_offset_m;
    let mut observation = transfer_observation(
        ctx.target_pad.center_x_m - position_m.x,
        position_m.y - ctx.target_pad.surface_y_m,
        velocity_mps,
        sim_time_s,
    );
    observation.position_m = position_m;
    observation.target_surface_y_m = ctx.target_pad.surface_y_m;
    observation.target_pad_half_width_m = ctx.target_pad.width_m * 0.5;
    observation.touchdown_clearance_m = touchdown_clearance_m;
    observation.min_hull_clearance_m = touchdown_clearance_m;
    observation
}

#[test]
fn transfer_waypoint_alias_enables_waypoint_guidance() {
    let spec = built_in_controller_spec("transfer_waypoint_pdg").unwrap();

    assert_eq!(spec.id(), "transfer_waypoint_pdg_v1");
    let ControllerSpec::TransferPdgV1 { config } = spec else {
        panic!("waypoint alias should use transfer controller");
    };
    assert!(config.waypoint_guidance_enabled);
}

#[test]
fn built_in_guidance_aliases_preserve_canonical_ids() {
    for (alias, expected_id) in [
        ("terminal_pdg", "terminal_pdg_v1"),
        ("tpdg", "terminal_pdg_v1"),
        ("transfer_pdg", "transfer_pdg_v1"),
        ("xpdg", "transfer_pdg_v1"),
        ("transfer_waypoint_pdg", "transfer_waypoint_pdg_v1"),
        ("xpdg_waypoint", "transfer_waypoint_pdg_v1"),
        ("transfer_pdg_pathwise", "transfer_pdg_pathwise_v1"),
        (
            "transfer_pdg_recoverability",
            "transfer_pdg_recoverability_v1",
        ),
    ] {
        assert_eq!(
            built_in_controller_spec(alias).map(|spec| spec.id()),
            Some(expected_id),
            "alias {alias}"
        );
    }
}

#[test]
fn guidance_controller_specs_preserve_serialized_mode_fields() {
    let waypoint = serde_json::to_value(
        built_in_controller_spec("transfer_waypoint_pdg").expect("waypoint spec"),
    )
    .expect("serialize waypoint spec");
    assert_eq!(waypoint["kind"], "transfer_pdg_v1");
    assert_eq!(waypoint["waypoint_guidance_enabled"], true);
    assert_eq!(waypoint["boost_pathwise_scoring_enabled"], false);
    assert_eq!(waypoint["boost_recoverability_scoring_enabled"], false);

    let no_terrain = serde_json::to_value(
        built_in_controller_spec("terminal_pdg_no_terrain").expect("no-terrain spec"),
    )
    .expect("serialize no-terrain spec");
    assert_eq!(no_terrain["kind"], "terminal_pdg_v1");
    assert_eq!(no_terrain["terrain_clearance_enabled"], false);
}

#[test]
fn guidance_telemetry_contract_keys_remain_stable() {
    assert_eq!(metric::GUIDANCE_MODE, "guidance.mode");
    assert_eq!(metric::TRANSFER_PHASE, "transfer.phase");
    assert_eq!(
        metric::TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO,
        "transfer.terminal_gate_required_accel_ratio"
    );
    assert_eq!(metric::WAYPOINT_CAPTURE_STATUS, "waypoint.capture_status");
    assert_eq!(
        metric::WAYPOINT_FINAL_TERMINAL_RECOVERABLE,
        "waypoint.final_terminal_recoverable"
    );
    assert_eq!(crate::kit::marker::TERMINAL_GATE, "gate/terminal_descent");
    assert_eq!(crate::kit::marker::WAYPOINT_HANDOFF, "waypoint/handoff");
}

fn transfer_metric_keys(frame: &ControllerFrame) -> Vec<&str> {
    frame
        .metrics
        .keys()
        .filter(|key| key.starts_with("transfer."))
        .map(String::as_str)
        .collect()
}

fn expected_transfer_metric_keys(include_boost_selection: bool) -> Vec<&'static str> {
    let mut keys = vec![
        metric::TRANSFER_APEX_OVER_TARGET_M,
        metric::TRANSFER_BOOST_APEX_TARGET_M,
        metric::TRANSFER_BOOST_QUALITY,
        metric::TRANSFER_BOOST_QUALITY_PASS,
        metric::TRANSFER_BOOST_SCORING_MODE,
        metric::TRANSFER_CORRIDOR_MARGIN_M,
        metric::TRANSFER_CORRIDOR_MODE,
        metric::TRANSFER_IMPACT_ANGLE_DEG,
        metric::TRANSFER_PROJECTED_DX_M,
        metric::TRANSFER_PROJECTED_TIME_S,
        metric::TRANSFER_ROUTE_DX_M,
        metric::TRANSFER_ROUTE_DY_M,
        metric::TRANSFER_SHAPE_ANCHOR_DX_M,
        metric::TRANSFER_SHAPE_ANCHOR_DY_M,
        metric::TRANSFER_TARGET_Y_SOLUTION,
        metric::TRANSFER_TERMINAL_GATE_DEFERRED,
        metric::TRANSFER_TERMINAL_GATE_LATEST_SAFE_MARGIN_S,
        metric::TRANSFER_TERMINAL_GATE_MODE,
        metric::TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO,
    ];
    if include_boost_selection {
        keys.extend([
            metric::TRANSFER_BOOST_SELECTED_SCORE,
            metric::TRANSFER_BOOST_SETTLED_PROJECTED_DX_M,
            metric::TRANSFER_BOOST_SETTLED_QUALITY,
        ]);
    }
    keys.sort_unstable();
    keys
}

#[test]
fn transfer_metric_paths_emit_stable_base_and_boost_keys() {
    let controller = TransferPdgController::default();
    let diagnostics = TransferDiagnostics {
        route_dx_m: 120.0,
        route_dy_m: 40.0,
        anchor: Some(TransferBoostAnchor {
            route_dx_m: 140.0,
            route_dy_m: 60.0,
        }),
        projection: TransferBallisticProjection {
            has_target_y_solution: true,
            projected_time_s: Some(4.0),
            projected_dx_m: Some(12.0),
            impact_angle_deg: Some(52.0),
            apex_over_target_m: 80.0,
        },
        boost_quality: TransferBoostQuality {
            verdict: "pass",
            passed: true,
            apex_target_over_target_m: 72.0,
        },
    };
    let gate = TerminalEntryAssessment {
        mode: TerminalEntryMode::NominalReady,
        ready_ticks: 2,
        burn_time_s: 5.0,
        latest_safe_margin_s: 1.5,
        required_accel_ratio: 0.7,
        terrain_min_clearance_m: 100.0,
        terrain_clearance_safe: true,
        deferred: false,
    };
    let corridor = TransferCorridorState {
        mode: "clear",
        active: false,
        tilt_limited: false,
        margin_m: 44.0,
    };
    let selection = TransferBoostCommandSelection {
        command: Command {
            throttle_frac: 0.8,
            target_attitude_rad: 0.2,
        },
        scoring_mode: "selected_mode",
        selected_score: 3.25,
        settled_projection: diagnostics.projection,
        settled_quality: diagnostics.boost_quality,
    };

    let open_loop_frame = controller
        .transfer_metrics_builder(
            ControllerFrameBuilder::new(selection.command),
            diagnostics,
            gate,
            corridor,
            Some(selection),
        )
        .build();
    assert_eq!(
        transfer_metric_keys(&open_loop_frame),
        expected_transfer_metric_keys(true)
    );
    assert_eq!(
        open_loop_frame
            .metrics
            .get(metric::TRANSFER_BOOST_SCORING_MODE),
        Some(&TelemetryValue::from("selected_mode"))
    );

    let mut terminal_frame = ControllerFrame::command_only(selection.command);
    controller.insert_transfer_metrics(&mut terminal_frame, diagnostics, gate, corridor);
    assert_eq!(
        transfer_metric_keys(&terminal_frame),
        expected_transfer_metric_keys(false)
    );
    assert_eq!(
        terminal_frame
            .metrics
            .get(metric::TRANSFER_BOOST_SCORING_MODE),
        Some(&TelemetryValue::from("legacy_endpoint"))
    );
}

#[test]
fn transfer_enables_retained_terminal_plans_only_for_waypoint_guidance() {
    let direct = TransferPdgController::default();
    let mut waypoint_config = TransferPdgControllerConfig::default();
    waypoint_config.waypoint_guidance_enabled = true;
    let waypoint = TransferPdgController::new(waypoint_config);

    assert!(!direct.terminal.guidance_plan_retention_enabled());
    assert!(waypoint.terminal.guidance_plan_retention_enabled());
}

#[test]
fn transfer_waypoint_guidance_tracks_active_leg_without_terminal_handoff() {
    let ctx = context_with_waypoint();
    let mut config = TransferPdgControllerConfig::default();
    config.waypoint_guidance_enabled = true;
    let mut controller = TransferPdgController::new(config);
    let mut observation = transfer_observation(140.0, -700.0, Vec2::new(-5.0, 24.0), 4.0);
    observation.position_m = Vec2::new(-145.0, -700.0);
    observation.target_surface_y_m = 0.0;

    let frame = controller.update(&ctx, &observation);

    assert_ne!(
        frame.metrics.get(metric::TRANSFER_PHASE),
        Some(&TelemetryValue::from("terminal"))
    );
    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_GUIDANCE_ENABLED),
        Some(&TelemetryValue::from(true))
    );
    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_ACTIVE_INDEX),
        Some(&TelemetryValue::from(0_i64))
    );
    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_CAPTURE_STATUS),
        Some(&TelemetryValue::from("tracking"))
    );
    assert!(frame.metrics.contains_key(metric::WAYPOINT_TURN_MARGIN_M));
    assert!(
        frame
            .metrics
            .contains_key(metric::WAYPOINT_REQUIRED_TURN_DISTANCE_M)
    );
    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_HANDOFF_TARGET_MODE),
        Some(&TelemetryValue::from("waypoint_center"))
    );
    assert!(
        frame
            .metrics
            .contains_key(metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_X_M)
    );
    assert!(
        frame
            .metrics
            .contains_key(metric::WAYPOINT_TARGET_DEADLINE_REMAINING_S)
    );
    assert!(
        frame
            .metrics
            .contains_key(metric::WAYPOINT_TARGET_VELOCITY_ERROR_MPS)
    );
    assert!(
        frame
            .metrics
            .contains_key(metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS)
    );
    assert!(
        frame
            .metrics
            .contains_key(metric::WAYPOINT_PREDICTED_HANDOFF_TIME_TO_GO_S)
    );
    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_REACHABLE_HANDOFF_MODEL),
        Some(&TelemetryValue::from("actuated_rollout"))
    );
    assert!(
        frame
            .metrics
            .contains_key(metric::WAYPOINT_REACHABLE_HANDOFF_CONTRACT_PASS)
    );
}

fn straight_waypoint_guidance() -> WaypointGuidanceFrame {
    WaypointGuidanceFrame {
        active_index: 0,
        center_m: Vec2::new(100.0, 0.0),
        nominal_handoff_target_m: Vec2::new(80.0, 0.0),
        handoff_target_m: Vec2::new(100.0, 0.0),
        handoff_target_mode: "waypoint_center",
        endpoint_m: Vec2::new(100.0, 0.0),
        steering_target_m: Vec2::new(100.0, 0.0),
        leg_unit: Vec2::new(1.0, 0.0),
        handoff_tangent_unit: Vec2::new(1.0, 0.0),
        envelope: WaypointGuidanceEnvelope {
            capture_radius_m: 20.0,
            max_cross_track_m: 30.0,
            max_outbound_heading_error_rad: 0.35,
            min_outbound_progress_mps: 5.0,
            max_outbound_cross_speed_mps: Some(10.0),
            min_speed_mps: 5.0,
            max_speed_mps: 30.0,
            min_vertical_speed_mps: None,
            max_vertical_speed_mps: None,
        },
        approach: WaypointApproachState {
            remaining_to_plane_m: 100.0,
            time_to_plane_s: 10.0,
            remaining_to_handoff_m: 80.0,
            time_to_handoff_s: 8.0,
            required_turn_distance_m: 20.0,
            shaping_start_distance_m: 100.0,
            turn_margin_m: 80.0,
            handoff_turn_margin_m: 60.0,
        },
    }
}

fn waypoint_candidate_fixture(
    contract_pass: bool,
    required_accel_ratio: f64,
    time_to_go_s: f64,
) -> WaypointGuidanceCandidate {
    WaypointGuidanceCandidate {
        target_velocity_mps: Vec2::new(20.0, 0.0),
        time_to_go_s,
        required_accel_mps2: Vec2::new(0.0, 10.0),
        required_accel_ratio,
        tilt_feasible: true,
        target_envelope_feasible: true,
        prediction: WaypointGuidancePrediction {
            time_to_event_s: time_to_go_s * 0.8,
            deadline_lead_s: time_to_go_s * 0.2,
            stats: WaypointLegStats {
                distance_m: 20.0,
                cross_track_m: 0.0,
                plane_progress_m: -20.0,
                outbound_heading_error_rad: if contract_pass { 0.0 } else { 0.7 },
                outbound_progress_mps: 20.0,
                outbound_cross_speed_mps: 0.0,
                speed_mps: 20.0,
                vertical_speed_mps: 0.0,
            },
            assessment: WaypointGuidanceAssessment {
                triggered: true,
                capture_window_open: true,
                deadline_reached: false,
                spatial_pass: true,
                violation_mask: if contract_pass {
                    0
                } else {
                    WAYPOINT_VIOLATION_HEADING
                },
            },
        },
    }
}

fn waypoint_reachable_fixture(contract_pass: bool) -> WaypointReachablePrediction {
    WaypointReachablePrediction {
        prediction: waypoint_candidate_fixture(contract_pass, 0.8, 4.0).prediction,
        event_state: TransferSimState {
            position_m: Vec2::new(80.0, 0.0),
            velocity_mps: Vec2::new(20.0, 0.0),
            attitude_rad: 0.0,
            fuel_kg: 200.0,
            dry_mass_kg: 700.0,
        },
        required_accel_ratio_max: 1.4,
        thrust_saturated_time_s: 0.2,
        tilt_saturated_time_s: 0.0,
    }
}

fn waypoint_reachable_candidate_fixture(
    contract_pass: bool,
    endpoint_x_m: f64,
    saturated_time_s: f64,
) -> WaypointReachableCandidate {
    let mut reachable_prediction = waypoint_reachable_fixture(contract_pass);
    reachable_prediction.thrust_saturated_time_s = saturated_time_s;
    WaypointReachableCandidate {
        candidate: waypoint_candidate_fixture(contract_pass, 0.8, 4.0),
        endpoint_m: Vec2::new(endpoint_x_m, 0.0),
        target_mode: "capture_envelope",
        reachable_prediction,
    }
}

fn waypoint_final_candidate_fixture(
    terminal_required_accel_ratio: f64,
    endpoint_x_m: f64,
    saturated_time_s: f64,
) -> WaypointFinalCandidate {
    WaypointFinalCandidate {
        reachable: waypoint_reachable_candidate_fixture(true, endpoint_x_m, saturated_time_s),
        terminal_gate: transfer_gate_fixture(1.0, terminal_required_accel_ratio, true),
    }
}

fn waypoint_joint_candidate_fixture(
    continuation_contract_pass: bool,
    endpoint_x_m: f64,
    saturated_time_s: f64,
) -> WaypointJointCandidatePrediction {
    WaypointJointCandidatePrediction {
        current: waypoint_reachable_candidate_fixture(true, endpoint_x_m, saturated_time_s),
        continuation: waypoint_reachable_fixture(continuation_contract_pass),
        continuation_passing_candidate_count: usize::from(continuation_contract_pass),
    }
}

#[test]
fn waypoint_reachable_prediction_tracks_an_actuated_straight_plan() {
    let ctx = context_with_waypoint();
    let controller = TransferPdgController::default();
    let guidance = straight_waypoint_guidance();
    let observation =
        waypoint_transfer_observation(&ctx, Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), 0.0);

    let reachable = controller.waypoint_reachable_prediction(
        &ctx,
        &observation,
        guidance,
        guidance.endpoint_m,
        Vec2::new(10.0, 0.0),
        10.0,
    );

    assert!(reachable.prediction.assessment.triggered);
    assert!(reachable.prediction.assessment.contract_pass());
    assert!(reachable.event_state.position_m.x >= 80.0);
    assert_eq!(
        reachable.event_state.dry_mass_kg,
        observation.mass_kg - observation.fuel_kg
    );
    assert!(reachable.required_accel_ratio_max <= 1.0);
    assert_eq!(reachable.thrust_saturated_time_s, 0.0);
}

#[test]
fn waypoint_reachable_prediction_exposes_attitude_limited_reference_failure() {
    let mut ctx = context_with_waypoint();
    ctx.vehicle.max_rotation_rate_radps = 0.0;
    let controller = TransferPdgController::default();
    let guidance = straight_waypoint_guidance();
    let mut observation =
        waypoint_transfer_observation(&ctx, Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), 0.0);
    observation.attitude_rad = 0.72;
    let reference =
        waypoint_guidance_prediction(&observation, guidance, Vec2::new(10.0, 0.0), 10.0);

    let reachable = controller.waypoint_reachable_prediction(
        &ctx,
        &observation,
        guidance,
        guidance.endpoint_m,
        Vec2::new(10.0, 0.0),
        10.0,
    );

    assert!(reference.assessment.contract_pass());
    assert!(!reachable.prediction.assessment.contract_pass());
    assert!(reachable.tilt_saturated_time_s > 0.0);
}

#[test]
fn waypoint_reachable_event_endpoints_stay_inside_the_capture_envelope() {
    let mut guidance = straight_waypoint_guidance();
    guidance.handoff_tangent_unit = normalized_or_none(Vec2::new(1.0, -1.0)).unwrap();

    let endpoints = waypoint_reachable_event_endpoints(guidance);

    assert_eq!(endpoints.len(), 3);
    assert!(endpoints.iter().all(|endpoint| {
        (*endpoint - guidance.center_m).length() < guidance.envelope.capture_radius_m
    }));
    assert!((endpoints[0].y - guidance.center_m.y).abs() < 1.0e-9);
    assert!(endpoints[2].y > guidance.center_m.y);
}

#[test]
fn waypoint_continuation_prediction_requires_and_targets_next_waypoint() {
    let mut ctx = context_with_waypoint();
    let controller = TransferPdgController::default();
    let first_geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let first_observation = waypoint_transfer_observation(
        &ctx,
        first_geometry.anchor_m + (first_geometry.leg_unit * 120.0),
        first_geometry.leg_unit * 25.0,
        5.0,
    );
    let first_stats = waypoint_leg_stats(&first_observation, &first_geometry);
    let first_approach =
        controller.waypoint_approach_state(&ctx, &first_observation, &first_geometry, first_stats);
    let first_guidance = waypoint_guidance_frame(&first_geometry, first_stats, first_approach);
    let current_reachable = waypoint_reachable_fixture(true);

    assert_eq!(
        controller.waypoint_continuation_prediction(
            &ctx,
            &first_observation,
            first_guidance,
            current_reachable,
        ),
        None
    );

    ctx.mission
        .transfer_route
        .as_mut()
        .unwrap()
        .waypoints
        .push(TransferWaypointSpec {
            id: "wp_1".to_owned(),
            position_m: Vec2::new(-100.0, -150.0),
            ..waypoint_fixture()
        });
    let first_geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let first_stats = waypoint_leg_stats(&first_observation, &first_geometry);
    let first_approach =
        controller.waypoint_approach_state(&ctx, &first_observation, &first_geometry, first_stats);
    let first_guidance = waypoint_guidance_frame(&first_geometry, first_stats, first_approach);
    let continuation = controller
        .waypoint_continuation_prediction(
            &ctx,
            &first_observation,
            first_guidance,
            current_reachable,
        )
        .unwrap();

    assert_eq!(continuation.next_waypoint_index, 1);
}

#[test]
fn waypoint_transition_audit_compares_projected_and_actual_event_state() {
    let mut ctx = context_with_waypoint();
    ctx.mission
        .transfer_route
        .as_mut()
        .unwrap()
        .waypoints
        .push(TransferWaypointSpec {
            id: "wp_1".to_owned(),
            position_m: Vec2::new(-100.0, -150.0),
            ..waypoint_fixture()
        });
    let controller = TransferPdgController::default();
    let source = waypoint_reachable_fixture(true);
    let continuation = WaypointContinuationPrediction {
        next_waypoint_index: 1,
        source_event_state: source.event_state,
        source_event_time_s: 7.0,
        prediction: source,
        passing_candidate_count: 1,
    };
    let mut observation = waypoint_transfer_observation(
        &ctx,
        source.event_state.position_m + Vec2::new(3.0, 4.0),
        source.event_state.velocity_mps + Vec2::new(0.0, 2.0),
        7.25,
    );
    observation.attitude_rad = source.event_state.attitude_rad + 0.1;
    observation.mass_kg = source.event_state.mass_kg() + 3.0;
    observation.fuel_kg = source.event_state.fuel_kg + 2.0;

    let audit = controller
        .waypoint_transition_audit(&ctx, &observation, continuation)
        .unwrap();

    assert_eq!(audit.next_waypoint_index, 1);
    assert!((audit.position_error_m - 5.0).abs() < 1.0e-9);
    assert!((audit.velocity_error_mps - 2.0).abs() < 1.0e-9);
    assert!((audit.attitude_error_rad - 0.1).abs() < 1.0e-9);
    assert!((audit.mass_error_kg - 3.0).abs() < 1.0e-9);
    assert!((audit.fuel_error_kg - 2.0).abs() < 1.0e-9);
    assert!((audit.event_time_error_s - 0.25).abs() < 1.0e-9);
}

#[test]
fn waypoint_transition_audit_requires_a_next_waypoint() {
    let ctx = context_with_waypoint();
    let controller = TransferPdgController::default();
    let source = waypoint_reachable_fixture(true);
    let continuation = WaypointContinuationPrediction {
        next_waypoint_index: 1,
        source_event_state: source.event_state,
        source_event_time_s: 7.0,
        prediction: source,
        passing_candidate_count: 1,
    };
    let observation = waypoint_transfer_observation(
        &ctx,
        source.event_state.position_m,
        source.event_state.velocity_mps,
        7.0,
    );

    assert_eq!(
        controller.waypoint_transition_audit(&ctx, &observation, continuation),
        None
    );
}

#[test]
fn waypoint_joint_search_requires_a_next_waypoint() {
    let ctx = context_with_waypoint();
    let controller = TransferPdgController::default();
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let observation = waypoint_transfer_observation(
        &ctx,
        geometry.anchor_m + (geometry.leg_unit * 120.0),
        geometry.leg_unit * 25.0,
        5.0,
    );
    let stats = waypoint_leg_stats(&observation, &geometry);
    let approach = controller.waypoint_approach_state(&ctx, &observation, &geometry, stats);
    let guidance = waypoint_guidance_frame(&geometry, stats, approach);

    assert_eq!(
        controller.waypoint_joint_search_prediction(&ctx, &observation, guidance),
        None
    );
}

#[test]
fn waypoint_joint_search_evaluates_at_most_four_current_candidates() {
    let mut ctx = context_with_waypoint();
    ctx.mission
        .transfer_route
        .as_mut()
        .unwrap()
        .waypoints
        .push(TransferWaypointSpec {
            id: "wp_1".to_owned(),
            position_m: Vec2::new(-100.0, -150.0),
            ..waypoint_fixture()
        });
    let controller = TransferPdgController::default();
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let observation = waypoint_transfer_observation(
        &ctx,
        geometry.anchor_m + (geometry.leg_unit * 120.0),
        geometry.leg_unit * 25.0,
        5.0,
    );
    let stats = waypoint_leg_stats(&observation, &geometry);
    let approach = controller.waypoint_approach_state(&ctx, &observation, &geometry, stats);
    let guidance = waypoint_guidance_frame(&geometry, stats, approach);

    let joint = controller
        .waypoint_joint_search_prediction(&ctx, &observation, guidance)
        .unwrap();

    assert!(joint.evaluated_candidate_count <= WAYPOINT_JOINT_MAX_CURRENT_CANDIDATES);
    assert!(joint.passing_candidate_count <= joint.evaluated_candidate_count);
    assert_eq!(
        joint
            .selected
            .is_some_and(|selected| selected.contract_pass()),
        joint.passing_candidate_count > 0
    );
}

#[test]
fn waypoint_joint_candidate_order_requires_both_contracts() {
    let passing = waypoint_joint_candidate_fixture(true, 80.0, 1.0);
    let failing = waypoint_joint_candidate_fixture(false, 80.0, 0.0);

    assert!(passing.contract_pass());
    assert!(!failing.contract_pass());
    assert_eq!(
        TransferPdgController::compare_waypoint_joint_candidates(passing, failing),
        std::cmp::Ordering::Less
    );
}

#[test]
fn waypoint_final_candidate_order_prefers_terminal_recoverability() {
    let recoverable = waypoint_final_candidate_fixture(0.95, 80.0, 1.0);
    let unrecoverable = waypoint_final_candidate_fixture(1.01, 70.0, 0.0);

    assert_eq!(
        TransferPdgController::compare_waypoint_final_candidates(recoverable, unrecoverable,),
        std::cmp::Ordering::Less
    );
}

#[test]
fn waypoint_final_candidate_order_prefers_lower_terminal_accel_ratio() {
    let lower_ratio = waypoint_final_candidate_fixture(0.80, 80.0, 1.0);
    let lower_rollout_saturation = waypoint_final_candidate_fixture(0.90, 70.0, 0.0);

    assert_eq!(
        TransferPdgController::compare_waypoint_final_candidates(
            lower_ratio,
            lower_rollout_saturation,
        ),
        std::cmp::Ordering::Less
    );
}

#[test]
fn waypoint_recoverable_final_capture_enters_terminal_guidance() {
    assert_eq!(
        waypoint_post_capture_phase(true, true, Some(true), true),
        TransferPhase::Terminal
    );
}

#[test]
fn waypoint_post_capture_phase_keeps_non_recoverable_paths_in_transfer() {
    for (final_waypoint, contract_pass, recoverable, spatial_ownership) in [
        (false, true, Some(true), true),
        (true, false, Some(true), true),
        (true, true, Some(false), true),
        (true, true, None, true),
        (true, true, Some(true), false),
    ] {
        assert_eq!(
            waypoint_post_capture_phase(
                final_waypoint,
                contract_pass,
                recoverable,
                spatial_ownership,
            ),
            TransferPhase::Boost
        );
    }
}

#[test]
fn waypoint_terminal_spatial_ownership_bounds_uphill_handoffs() {
    let config = TransferPdgControllerConfig::default();
    let mut observation = transfer_observation(220.0, 110.0, Vec2::new(20.0, 10.0), 10.0);
    observation.height_above_target_m = -110.0;
    assert!(waypoint_terminal_spatial_ownership(&config, &observation));

    observation.target_dx_m = config.terminal_gate_dx_m + 1.0;
    assert!(!waypoint_terminal_spatial_ownership(&config, &observation));
    observation.target_dx_m = 220.0;
    observation.height_above_target_m = -config.terminal_gate_altitude_m - 1.0;
    assert!(!waypoint_terminal_spatial_ownership(&config, &observation));

    observation.height_above_target_m = 1.0;
    observation.target_dx_m = config.terminal_gate_dx_m + 1.0;
    assert!(waypoint_terminal_spatial_ownership(&config, &observation));
}

#[test]
fn waypoint_joint_candidate_order_is_deterministic() {
    let earlier_endpoint = waypoint_joint_candidate_fixture(true, 70.0, 0.2);
    let later_endpoint = waypoint_joint_candidate_fixture(true, 80.0, 0.2);

    assert_eq!(
        TransferPdgController::compare_waypoint_joint_candidates(earlier_endpoint, later_endpoint,),
        std::cmp::Ordering::Less
    );
}

#[test]
fn waypoint_joint_search_is_cached_once_per_plan_revision() {
    let mut ctx = context_with_waypoint();
    ctx.mission
        .transfer_route
        .as_mut()
        .unwrap()
        .waypoints
        .push(TransferWaypointSpec {
            id: "wp_1".to_owned(),
            position_m: Vec2::new(-100.0, -150.0),
            ..waypoint_fixture()
        });
    let mut controller = TransferPdgController::default();
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let observation = waypoint_transfer_observation(
        &ctx,
        geometry.anchor_m + (geometry.leg_unit * 120.0),
        geometry.leg_unit * 25.0,
        5.0,
    );
    let stats = waypoint_leg_stats(&observation, &geometry);
    let approach = controller.waypoint_approach_state(&ctx, &observation, &geometry, stats);
    let guidance = waypoint_guidance_frame(&geometry, stats, approach);
    assert_eq!(
        controller.cached_waypoint_joint_search_prediction(
            &ctx,
            &observation,
            guidance,
            2,
            waypoint_reachable_fixture(false),
        ),
        None
    );
    assert_eq!(controller.waypoint_joint_snapshot, None);
    let first = controller
        .cached_waypoint_joint_search_prediction(
            &ctx,
            &observation,
            guidance,
            2,
            waypoint_reachable_fixture(true),
        )
        .unwrap();
    let mut changed_observation = observation.clone();
    changed_observation.position_m += Vec2::new(200.0, 100.0);

    let cached = controller
        .cached_waypoint_joint_search_prediction(
            &ctx,
            &changed_observation,
            guidance,
            2,
            waypoint_reachable_fixture(false),
        )
        .unwrap();
    controller.cached_waypoint_joint_search_prediction(
        &ctx,
        &changed_observation,
        guidance,
        3,
        waypoint_reachable_fixture(true),
    );

    assert_eq!(cached, first);
    assert_eq!(controller.waypoint_joint_snapshot.unwrap().0, 3);
}

#[test]
fn waypoint_reachable_search_preserves_a_reference_passing_center_plan() {
    let mut ctx = context_with_waypoint();
    ctx.vehicle.max_rotation_rate_radps = 0.0;
    let mut controller = TransferPdgController::default();
    let guidance = straight_waypoint_guidance();
    let mut observation =
        waypoint_transfer_observation(&ctx, Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), 0.0);
    observation.attitude_rad = 0.72;
    controller.waypoint_guidance_plan = Some(WaypointGuidancePlan {
        waypoint_index: 0,
        revision: 0,
        reason: WaypointGuidancePlanReason::Initial,
        created_time_s: 0.0,
        start_position_m: observation.position_m,
        start_velocity_mps: observation.velocity_mps,
        endpoint_m: guidance.endpoint_m,
        target_mode: "waypoint_center",
        target_velocity_mps: Vec2::new(10.0, 0.0),
        arrival_time_s: 10.0,
        target_envelope_feasible: true,
        final_terminal_required_accel_ratio: None,
        final_terminal_recoverable: None,
    });

    controller.current_waypoint_guidance_candidate(&ctx, &observation, guidance);

    assert!(controller.waypoint_reference_contract_pass_ever);
    assert_eq!(
        controller.waypoint_guidance_plan.unwrap().target_mode,
        "waypoint_center"
    );
}

#[test]
fn waypoint_candidate_order_prefers_contract_pass_over_shorter_failure() {
    let controller = TransferPdgController::default();
    let failing = waypoint_candidate_fixture(false, 0.2, 2.0);
    let passing = waypoint_candidate_fixture(true, 0.8, 8.0);

    assert_eq!(
        controller.compare_waypoint_guidance_candidates(passing, failing, true),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        controller.compare_waypoint_guidance_candidates(failing, passing, false),
        std::cmp::Ordering::Less
    );
}

#[test]
fn waypoint_candidate_order_preserves_shorter_horizon_within_passing_class() {
    let controller = TransferPdgController::default();
    let lower_effort = waypoint_candidate_fixture(true, 0.4, 8.0);
    let shorter = waypoint_candidate_fixture(true, 0.6, 4.0);

    assert_eq!(
        controller.compare_waypoint_guidance_candidates(shorter, lower_effort, true),
        std::cmp::Ordering::Less
    );
}

#[test]
fn waypoint_plan_replacement_requires_strict_contract_improvement() {
    let failing = waypoint_candidate_fixture(false, 0.4, 5.0);
    let another_failure = waypoint_candidate_fixture(false, 0.2, 4.0);
    let passing = waypoint_candidate_fixture(true, 0.8, 8.0);
    let near_equivalent_passing = waypoint_candidate_fixture(true, 0.8, 5.2);

    assert!(
        TransferPdgController::should_replace_waypoint_guidance_plan(failing, passing, false, true,)
    );
    assert!(
        !TransferPdgController::should_replace_waypoint_guidance_plan(
            failing,
            another_failure,
            false,
            true,
        )
    );
    assert!(
        !TransferPdgController::should_replace_waypoint_guidance_plan(
            passing, passing, false, true,
        )
    );
    assert!(
        !TransferPdgController::should_replace_waypoint_guidance_plan(
            failing, passing, false, false,
        )
    );
    assert!(
        !TransferPdgController::should_replace_waypoint_guidance_plan(
            failing,
            near_equivalent_passing,
            false,
            true,
        )
    );
}

#[test]
fn waypoint_plan_replacement_preserves_authority_recovery() {
    let mut over_authority = waypoint_candidate_fixture(false, 1.2, 4.0);
    over_authority.tilt_feasible = false;
    let feasible_failure = waypoint_candidate_fixture(false, 0.8, 6.0);

    assert!(
        TransferPdgController::should_replace_waypoint_guidance_plan(
            over_authority,
            feasible_failure,
            false,
            false,
        )
    );
}

#[test]
fn waypoint_authority_recovery_preserves_actuated_passing_plan() {
    let current = waypoint_candidate_fixture(true, 1.2, 4.0);
    let feasible_current = waypoint_candidate_fixture(true, 0.8, 4.0);
    let failing_current = waypoint_candidate_fixture(false, 1.2, 4.0);
    let current_pass = waypoint_reachable_fixture(true);
    let current_fail = waypoint_reachable_fixture(false);

    assert!(
        TransferPdgController::should_preserve_waypoint_plan_during_authority_recovery(
            current,
            current_pass,
            WaypointGuidancePlanReason::ReachableRecovery,
            false,
            true,
        )
    );
    assert!(
        !TransferPdgController::should_preserve_waypoint_plan_during_authority_recovery(
            current,
            current_fail,
            WaypointGuidancePlanReason::ReachableRecovery,
            false,
            true,
        )
    );
    assert!(
        !TransferPdgController::should_preserve_waypoint_plan_during_authority_recovery(
            failing_current,
            current_pass,
            WaypointGuidancePlanReason::ReachableRecovery,
            false,
            true,
        )
    );
    assert!(
        !TransferPdgController::should_preserve_waypoint_plan_during_authority_recovery(
            current,
            current_pass,
            WaypointGuidancePlanReason::ReachableRecovery,
            true,
            true,
        )
    );
    assert!(
        !TransferPdgController::should_preserve_waypoint_plan_during_authority_recovery(
            current,
            current_pass,
            WaypointGuidancePlanReason::ReachableRecovery,
            false,
            false,
        )
    );
    assert!(
        !TransferPdgController::should_preserve_waypoint_plan_during_authority_recovery(
            feasible_current,
            current_pass,
            WaypointGuidancePlanReason::ReachableRecovery,
            false,
            true,
        )
    );
    assert!(
        !TransferPdgController::should_preserve_waypoint_plan_during_authority_recovery(
            current,
            current_pass,
            WaypointGuidancePlanReason::Initial,
            false,
            true,
        )
    );
}

#[test]
fn waypoint_plan_reason_reports_replacement_trigger() {
    let feasible = waypoint_candidate_fixture(true, 0.8, 5.0);
    let mut over_authority = waypoint_candidate_fixture(false, 1.2, 5.0);
    over_authority.tilt_feasible = false;

    assert_eq!(
        TransferPdgController::waypoint_guidance_plan_reason(None, false),
        WaypointGuidancePlanReason::Initial
    );
    assert_eq!(
        TransferPdgController::waypoint_guidance_plan_reason(Some(feasible), true),
        WaypointGuidancePlanReason::Expired
    );
    assert_eq!(
        TransferPdgController::waypoint_guidance_plan_reason(Some(over_authority), false),
        WaypointGuidancePlanReason::AuthorityRecovery
    );
    assert_eq!(
        TransferPdgController::waypoint_guidance_plan_reason(Some(feasible), false),
        WaypointGuidancePlanReason::ContractRecovery
    );
}

#[test]
fn waypoint_contract_failure_waits_for_local_prediction_horizon() {
    let mut candidate = waypoint_candidate_fixture(false, 0.8, 20.0);
    candidate.prediction.time_to_event_s = WAYPOINT_GUIDANCE_PREDICTION_HORIZON_S + 0.1;
    assert!(!TransferPdgController::waypoint_guidance_contract_failure_is_actionable(candidate));

    candidate.prediction.time_to_event_s = WAYPOINT_GUIDANCE_PREDICTION_HORIZON_S;
    assert!(TransferPdgController::waypoint_guidance_contract_failure_is_actionable(candidate));
}

#[test]
fn waypoint_cubic_reference_preserves_endpoint_state() {
    let start_position_m = Vec2::new(-20.0, 30.0);
    let start_velocity_mps = Vec2::new(8.0, -2.0);
    let end_position_m = Vec2::new(100.0, 0.0);
    let end_velocity_mps = Vec2::new(12.0, -4.0);

    let start = waypoint_cubic_reference_state(
        start_position_m,
        start_velocity_mps,
        end_position_m,
        end_velocity_mps,
        6.0,
        0.0,
    );
    let end = waypoint_cubic_reference_state(
        start_position_m,
        start_velocity_mps,
        end_position_m,
        end_velocity_mps,
        6.0,
        6.0,
    );

    assert!((start.0 - start_position_m).length() < 1.0e-9);
    assert!((start.1 - start_velocity_mps).length() < 1.0e-9);
    assert!((end.0 - end_position_m).length() < 1.0e-9);
    assert!((end.1 - end_velocity_mps).length() < 1.0e-9);
}

#[test]
fn waypoint_plan_trackability_uses_immutable_creation_reference() {
    let guidance = straight_waypoint_guidance();
    let start_position_m = Vec2::new(0.0, 0.0);
    let start_velocity_mps = Vec2::new(8.0, -2.0);
    let plan = WaypointGuidancePlan {
        waypoint_index: 0,
        revision: 0,
        reason: WaypointGuidancePlanReason::Initial,
        created_time_s: 2.0,
        start_position_m,
        start_velocity_mps,
        endpoint_m: guidance.endpoint_m,
        target_mode: "waypoint_center",
        target_velocity_mps: Vec2::new(12.0, -4.0),
        arrival_time_s: 8.0,
        target_envelope_feasible: true,
        final_terminal_required_accel_ratio: None,
        final_terminal_recoverable: None,
    };
    let mut observation = transfer_observation(0.0, 0.0, start_velocity_mps, 5.0);
    let (position_m, velocity_mps) = waypoint_cubic_reference_state(
        start_position_m,
        start_velocity_mps,
        plan.endpoint_m,
        plan.target_velocity_mps,
        6.0,
        3.0,
    );
    observation.position_m = position_m;
    observation.velocity_mps = velocity_mps;

    let trackability =
        TransferPdgController::waypoint_guidance_trackability(&observation, guidance, plan);

    assert_eq!(trackability.plan_index, 0);
    assert_eq!(trackability.plan_revision, 0);
    assert_eq!(
        trackability.plan_reason,
        WaypointGuidancePlanReason::Initial
    );
    assert_eq!(trackability.plan_age_s, 3.0);
    assert!(trackability.reference_position_error_m < 1.0e-9);
    assert!(trackability.reference_cross_error_m.abs() < 1.0e-9);
    assert!(trackability.reference_velocity_error_mps < 1.0e-9);
    assert!(trackability.reference_cross_speed_error_mps.abs() < 1.0e-9);
}

#[test]
fn waypoint_prediction_finds_radius_entry_before_center_deadline() {
    let guidance = straight_waypoint_guidance();
    let mut observation = transfer_observation(100.0, 0.0, Vec2::new(10.0, 0.0), 0.0);
    observation.position_m = Vec2::new(0.0, 0.0);
    let prediction =
        waypoint_guidance_prediction(&observation, guidance, Vec2::new(10.0, 0.0), 10.0);

    assert!(prediction.assessment.triggered);
    assert!(prediction.assessment.contract_pass());
    assert!(prediction.time_to_event_s < 10.0);
    assert!(prediction.deadline_lead_s > 0.0);
    assert!(prediction.stats.distance_m <= guidance.envelope.capture_radius_m + 0.01);
    assert!(prediction.stats.plane_progress_m < 0.0);
}

#[test]
fn waypoint_prediction_finds_plane_crossing_before_radius_entry() {
    let mut guidance = straight_waypoint_guidance();
    guidance.handoff_tangent_unit = Vec2::new(0.0, -1.0);
    guidance.envelope.max_cross_track_m = 200.0;
    guidance.envelope.max_outbound_heading_error_rad = std::f64::consts::PI;
    guidance.envelope.max_outbound_cross_speed_mps = None;
    guidance.envelope.max_speed_mps = 200.0;
    let mut observation = transfer_observation(100.0, 100.0, Vec2::new(150.0, 0.0), 0.0);
    observation.position_m = Vec2::new(0.0, 100.0);
    let prediction =
        waypoint_guidance_prediction(&observation, guidance, Vec2::new(0.0, -20.0), 4.0);

    assert!(prediction.assessment.triggered);
    assert!(prediction.stats.plane_progress_m >= 0.0);
    assert!(prediction.stats.distance_m > guidance.envelope.capture_radius_m);
}

#[test]
fn waypoint_prediction_reports_already_triggered_state() {
    let guidance = straight_waypoint_guidance();
    let mut observation = transfer_observation(10.0, 0.0, Vec2::new(10.0, 0.0), 0.0);
    observation.position_m = Vec2::new(90.0, 0.0);
    let prediction =
        waypoint_guidance_prediction(&observation, guidance, Vec2::new(10.0, 0.0), 2.0);

    assert_eq!(prediction.time_to_event_s, 0.0);
    assert_eq!(prediction.deadline_lead_s, 2.0);
    assert!(prediction.assessment.contract_pass());
}

#[test]
fn waypoint_prediction_allows_horizon_to_end_before_handoff_resolution() {
    let mut guidance = straight_waypoint_guidance();
    guidance.endpoint_m = guidance.center_m - (guidance.leg_unit * 30.0);
    let mut observation = transfer_observation(100.0, 0.0, Vec2::new(10.0, 0.0), 0.0);
    observation.position_m = Vec2::new(0.0, 0.0);

    let prediction =
        waypoint_guidance_prediction(&observation, guidance, Vec2::new(10.0, 0.0), 2.0);

    assert_eq!(prediction.time_to_event_s, 2.0);
    assert!(!prediction.assessment.triggered);
    assert!(!prediction.assessment.contract_pass());
    assert!(!prediction.assessment.deadline_reached);
}

#[test]
fn waypoint_prediction_assessment_matches_authoritative_contract() {
    let mut guidance = straight_waypoint_guidance();
    guidance.handoff_tangent_unit = Vec2::new(0.0, 1.0);
    let mut observation = transfer_observation(100.0, 0.0, Vec2::new(10.0, 0.0), 0.0);
    observation.position_m = Vec2::new(0.0, 0.0);
    let prediction =
        waypoint_guidance_prediction(&observation, guidance, Vec2::new(10.0, 0.0), 10.0);
    let waypoint = TransferWaypointSpec {
        id: "prediction_contract".to_owned(),
        position_m: guidance.center_m,
        handoff_tangent_unit: None,
        capture_radius_m: guidance.envelope.capture_radius_m,
        max_cross_track_m: guidance.envelope.max_cross_track_m,
        max_outbound_heading_error_rad: guidance.envelope.max_outbound_heading_error_rad,
        min_outbound_progress_mps: guidance.envelope.min_outbound_progress_mps,
        max_outbound_cross_speed_mps: guidance.envelope.max_outbound_cross_speed_mps,
        min_speed_mps: guidance.envelope.min_speed_mps,
        max_speed_mps: guidance.envelope.max_speed_mps,
        min_vertical_speed_mps: guidance.envelope.min_vertical_speed_mps,
        max_vertical_speed_mps: guidance.envelope.max_vertical_speed_mps,
    };
    let authoritative = waypoint.assess_handoff(waypoint_handoff_kinematics(prediction.stats));
    let authoritative_reasons = authoritative
        .violations
        .iter()
        .map(|violation| violation.as_str())
        .collect::<Vec<_>>()
        .join(",");

    assert_eq!(prediction.assessment.triggered, authoritative.triggered);
    assert_eq!(
        prediction.assessment.spatial_pass,
        authoritative.spatial_pass
    );
    assert_eq!(
        prediction.assessment.envelope_pass(),
        authoritative.envelope_pass
    );
    assert_eq!(prediction.assessment.reasons(), authoritative_reasons);
    assert!(!prediction.assessment.contract_pass());
}

#[test]
fn transfer_waypoint_approach_state_reports_positive_turn_margin_for_viable_handoff() {
    let ctx = context_with_waypoint();
    let controller = TransferPdgController::new(TransferPdgControllerConfig::default());
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let mut observation = transfer_observation(0.0, 0.0, geometry.handoff_tangent_unit * 28.0, 4.0);
    observation.position_m =
        geometry.target_m - (geometry.leg_unit * geometry.waypoint.capture_radius_m * 6.0);
    observation.mass_kg = 940.0;

    let stats = waypoint_leg_stats(&observation, &geometry);
    let approach = controller.waypoint_approach_state(&ctx, &observation, &geometry, stats);

    assert!(approach.required_turn_distance_m <= geometry.waypoint.capture_radius_m * 1.1);
    assert!(approach.turn_margin_m > geometry.waypoint.capture_radius_m * 4.0);
    assert!(approach.shaping_start_distance_m >= approach.required_turn_distance_m);
}

#[test]
fn transfer_waypoint_approach_state_reports_negative_turn_margin_for_late_sharp_turn() {
    let ctx = context_with_waypoint();
    let controller = TransferPdgController::new(TransferPdgControllerConfig::default());
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let mut observation =
        transfer_observation(0.0, 0.0, geometry.handoff_tangent_unit * -70.0, 4.0);
    observation.position_m =
        geometry.target_m - (geometry.leg_unit * geometry.waypoint.capture_radius_m * 1.5);
    observation.mass_kg = 940.0;

    let stats = waypoint_leg_stats(&observation, &geometry);
    let approach = controller.waypoint_approach_state(&ctx, &observation, &geometry, stats);

    assert!(approach.turn_margin_m < 0.0);
    assert!(
        approach.shaping_start_distance_m
            > geometry.waypoint.capture_radius_m * WAYPOINT_OUTBOUND_BLEND_START_CAPTURE_RADII
    );
}

#[test]
fn transfer_waypoint_guidance_target_tracks_active_leg_lookahead() {
    let ctx = context_with_waypoint();
    let controller = TransferPdgController::new(TransferPdgControllerConfig::default());
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let current_progress_m = geometry.waypoint.capture_radius_m;
    let mut observation = transfer_observation(0.0, 0.0, geometry.leg_unit * 25.0, 4.0);
    observation.position_m = geometry.anchor_m
        + (geometry.leg_unit * current_progress_m)
        + (Vec2::new(-geometry.leg_unit.y, geometry.leg_unit.x)
            * geometry.waypoint.max_cross_track_m
            * 0.8);

    let stats = waypoint_leg_stats(&observation, &geometry);
    let target_m = waypoint_leg_steering_target_m(&geometry, stats);
    let target_from_anchor = target_m - geometry.anchor_m;
    let target_cross_track_m = vec_cross(target_from_anchor, geometry.leg_unit).abs();
    let target_progress_m = vec_dot(target_from_anchor, geometry.leg_unit);

    assert!(target_cross_track_m < 1.0e-6);
    assert!(target_progress_m > current_progress_m);
    assert!(target_progress_m < geometry.leg_length_m);
}

#[test]
fn transfer_waypoint_guidance_frame_separates_endpoint_from_steering_target() {
    let ctx = context_with_waypoint();
    let controller = TransferPdgController::new(TransferPdgControllerConfig::default());
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let mut observation = transfer_observation(0.0, 0.0, geometry.leg_unit * 25.0, 4.0);
    observation.position_m = geometry.anchor_m + (geometry.leg_unit * 80.0);

    let stats = waypoint_leg_stats(&observation, &geometry);
    let approach = controller.waypoint_approach_state(&ctx, &observation, &geometry, stats);
    let guidance = waypoint_guidance_frame(&geometry, stats, approach);

    assert_eq!(guidance.endpoint_m, geometry.target_m);
    assert_ne!(guidance.steering_target_m, guidance.endpoint_m);
    assert_eq!(
        guidance.envelope.capture_radius_m,
        geometry.waypoint.capture_radius_m
    );
}

#[test]
fn transfer_waypoint_guidance_endpoint_is_stable_while_steering_target_advances() {
    let ctx = context_with_waypoint();
    let controller = TransferPdgController::new(TransferPdgControllerConfig::default());
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let mut first = transfer_observation(0.0, 0.0, geometry.leg_unit * 25.0, 4.0);
    first.position_m = geometry.anchor_m + (geometry.leg_unit * 60.0);
    let mut second = first.clone();
    second.position_m += geometry.leg_unit * 90.0;

    let first_stats = waypoint_leg_stats(&first, &geometry);
    let first_approach = controller.waypoint_approach_state(&ctx, &first, &geometry, first_stats);
    let first_guidance = waypoint_guidance_frame(&geometry, first_stats, first_approach);
    let second_stats = waypoint_leg_stats(&second, &geometry);
    let second_approach =
        controller.waypoint_approach_state(&ctx, &second, &geometry, second_stats);
    let second_guidance = waypoint_guidance_frame(&geometry, second_stats, second_approach);

    assert_eq!(first_guidance.endpoint_m, second_guidance.endpoint_m);
    assert_ne!(
        first_guidance.steering_target_m,
        second_guidance.steering_target_m
    );
}

#[test]
fn transfer_waypoint_state_target_candidate_uses_outbound_envelope() {
    let ctx = context_with_waypoint();
    let controller = TransferPdgController::new(TransferPdgControllerConfig::default());
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let mut observation = waypoint_transfer_observation(
        &ctx,
        geometry.anchor_m + (geometry.leg_unit * 120.0),
        geometry.leg_unit * 30.0,
        5.0,
    );
    observation.mass_kg = 940.0;
    let stats = waypoint_leg_stats(&observation, &geometry);
    let approach = controller.waypoint_approach_state(&ctx, &observation, &geometry, stats);
    let guidance = waypoint_guidance_frame(&geometry, stats, approach);

    let candidate =
        controller.select_waypoint_guidance_candidate(&ctx, &observation, guidance, false);
    let candidates = controller.waypoint_guidance_candidates(&ctx, &observation, guidance);
    let target_speed_mps = candidate.target_velocity_mps.length();

    assert!(candidates.contains(&candidate));
    assert_eq!(
        candidates
            .into_iter()
            .min_by(|lhs, rhs| {
                controller.compare_waypoint_guidance_candidates(*lhs, *rhs, false)
            })
            .unwrap(),
        candidate
    );
    assert!(candidate.target_envelope_feasible);
    assert!(vec_cross(candidate.target_velocity_mps, geometry.handoff_tangent_unit).abs() < 1.0e-9);
    assert!(target_speed_mps >= geometry.waypoint.min_speed_mps);
    assert!(target_speed_mps <= geometry.waypoint.max_speed_mps);
    assert!(
        vec_dot(candidate.target_velocity_mps, geometry.handoff_tangent_unit)
            >= geometry.waypoint.min_outbound_progress_mps
    );
    assert!(candidate.time_to_go_s >= WAYPOINT_GUIDANCE_MIN_TIME_TO_GO_S);
}

#[test]
fn transfer_waypoint_target_velocity_accepts_max_speed_roundoff() {
    let controller = TransferPdgController::default();
    let mut guidance = straight_waypoint_guidance();
    guidance.handoff_tangent_unit = Vec2::new(0.866_025_403_784_438_7, -0.5);
    guidance.envelope.min_speed_mps = 10.0;
    guidance.envelope.max_speed_mps = 55.0;
    guidance.envelope.min_outbound_progress_mps = 8.0;
    let target_velocity_mps = guidance.handoff_tangent_unit * guidance.envelope.max_speed_mps;

    assert!(target_velocity_mps.length() > guidance.envelope.max_speed_mps);
    assert!(controller.waypoint_target_velocity_is_valid(guidance, target_velocity_mps));

    let over_limit_velocity_mps =
        guidance.handoff_tangent_unit * (guidance.envelope.max_speed_mps + 0.01);
    assert!(!controller.waypoint_target_velocity_is_valid(guidance, over_limit_velocity_mps));
}

#[test]
fn transfer_waypoint_state_target_horizon_does_not_use_sim_timeout() {
    let mut short_ctx = context_with_waypoint();
    short_ctx.sim.max_time_s = 20.0;
    let mut long_ctx = short_ctx.clone();
    long_ctx.sim.max_time_s = 200.0;
    let controller = TransferPdgController::new(TransferPdgControllerConfig::default());
    let geometry = controller.waypoint_leg_geometry(&short_ctx).unwrap();
    let mut observation = waypoint_transfer_observation(
        &short_ctx,
        geometry.anchor_m + (geometry.leg_unit * 120.0),
        geometry.leg_unit * 30.0,
        5.0,
    );
    observation.mass_kg = 940.0;
    let stats = waypoint_leg_stats(&observation, &geometry);
    let approach = controller.waypoint_approach_state(&short_ctx, &observation, &geometry, stats);
    let guidance = waypoint_guidance_frame(&geometry, stats, approach);

    let short =
        controller.select_waypoint_guidance_candidate(&short_ctx, &observation, guidance, false);
    let long =
        controller.select_waypoint_guidance_candidate(&long_ctx, &observation, guidance, false);

    assert_eq!(short.target_velocity_mps, long.target_velocity_mps);
    assert_eq!(short.time_to_go_s, long.time_to_go_s);
}

#[test]
fn transfer_waypoint_takeoff_regulates_geometry_scaled_vertical_speed() {
    let ctx = context_with_waypoint();
    let controller = TransferPdgController::new(TransferPdgControllerConfig::default());
    let mut guidance = straight_waypoint_guidance();
    let route = ctx.mission.transfer_route.as_ref().unwrap();
    let source_pad = ctx.world.landing_pad(&route.source_pad_id).unwrap();
    guidance.center_m = Vec2::new(source_pad.center_x_m + 225.0, source_pad.surface_y_m);
    guidance.leg_unit = Vec2::new(1.0, 0.0);
    let target_vertical_speed_mps =
        controller.waypoint_takeoff_target_vertical_speed_mps(&ctx, guidance);
    let mut observation = waypoint_transfer_observation(
        &ctx,
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, target_vertical_speed_mps),
        1.0,
    );
    let hover_throttle = observation.mass_kg * observation.gravity_mps2 / ctx.vehicle.max_thrust_n;

    let on_target = controller.waypoint_takeoff_command(&ctx, &observation, guidance);
    observation.velocity_mps.y -= 1.0;
    let below_target = controller.waypoint_takeoff_command(&ctx, &observation, guidance);
    observation.velocity_mps.y += 2.0;
    let above_target = controller.waypoint_takeoff_command(&ctx, &observation, guidance);

    assert!((target_vertical_speed_mps - 18.0).abs() < 1.0e-9);
    assert!((on_target.throttle_frac - hover_throttle).abs() < 1.0e-9);
    assert!(below_target.throttle_frac > on_target.throttle_frac);
    assert!(above_target.throttle_frac < on_target.throttle_frac);
    assert_eq!(on_target.target_attitude_rad, 0.0);

    guidance.center_m = Vec2::new(source_pad.center_x_m + 50.0, source_pad.surface_y_m);
    assert_eq!(
        controller.waypoint_takeoff_target_vertical_speed_mps(&ctx, guidance),
        controller.config.takeoff_min_vertical_speed_mps
    );
}

#[test]
fn transfer_waypoint_state_target_plan_is_stable_while_feasible() {
    let ctx = context_with_waypoint();
    let mut controller = TransferPdgController::new(TransferPdgControllerConfig::default());
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let mut first = waypoint_transfer_observation(
        &ctx,
        geometry.anchor_m + (geometry.leg_unit * 120.0),
        geometry.leg_unit * 30.0,
        5.0,
    );
    first.mass_kg = 940.0;
    let first_stats = waypoint_leg_stats(&first, &geometry);
    let first_approach = controller.waypoint_approach_state(&ctx, &first, &geometry, first_stats);
    let first_guidance = waypoint_guidance_frame(&geometry, first_stats, first_approach);
    controller.current_waypoint_guidance_candidate(&ctx, &first, first_guidance);
    let initial_plan = controller.waypoint_guidance_plan.unwrap();

    let mut second = first.clone();
    second.sim_time_s += 0.1;
    second.position_m += second.velocity_mps * 0.1;
    let second_stats = waypoint_leg_stats(&second, &geometry);
    let second_approach =
        controller.waypoint_approach_state(&ctx, &second, &geometry, second_stats);
    let second_guidance = waypoint_guidance_frame(&geometry, second_stats, second_approach);
    controller.current_waypoint_guidance_candidate(&ctx, &second, second_guidance);

    assert_eq!(controller.waypoint_guidance_plan, Some(initial_plan));
    assert_eq!(controller.waypoint_guidance_replan_count, 0);
}

#[test]
fn transfer_waypoint_path_correction_is_bounded_and_points_back_to_leg() {
    let ctx = context_with_waypoint();
    let controller = TransferPdgController::new(TransferPdgControllerConfig::default());
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let leg_normal = Vec2::new(-geometry.leg_unit.y, geometry.leg_unit.x);
    let mut observation = waypoint_transfer_observation(
        &ctx,
        geometry.anchor_m + (geometry.leg_unit * 120.0) + (leg_normal * 30.0),
        geometry.leg_unit * 35.0,
        5.0,
    );
    observation.mass_kg = 940.0;
    let stats = waypoint_leg_stats(&observation, &geometry);
    let approach = controller.waypoint_approach_state(&ctx, &observation, &geometry, stats);
    let guidance = waypoint_guidance_frame(&geometry, stats, approach);
    let state_target_accel = Vec2::new(0.0, observation.gravity_mps2);

    let correction =
        controller.waypoint_path_correction_mps2(&ctx, &observation, guidance, state_target_accel);
    let max_thrust_accel_mps2 = ctx.vehicle.max_thrust_n / observation.mass_kg;

    assert!(vec_dot(correction, leg_normal) < 0.0);
    assert!(
        correction.length()
            <= max_thrust_accel_mps2 * WAYPOINT_GUIDANCE_PATH_AUTHORITY_FRAC + 1.0e-9
    );
}

#[test]
fn transfer_waypoint_active_leg_uses_state_target_boost_guidance() {
    let ctx = context_with_waypoint();
    let mut config = TransferPdgControllerConfig::default();
    config.waypoint_guidance_enabled = true;
    let mut controller = TransferPdgController::new(config);
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let observation = waypoint_transfer_observation(
        &ctx,
        geometry.anchor_m + (geometry.leg_unit * 120.0) + Vec2::new(0.0, 80.0),
        geometry.leg_unit * 30.0,
        6.0,
    );

    let frame = controller.update(&ctx, &observation);

    assert_eq!(
        frame.metrics.get(metric::TRANSFER_PHASE),
        Some(&TelemetryValue::from("boost"))
    );
    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_GUIDANCE_MODE),
        Some(&TelemetryValue::from("state_target"))
    );
    assert!(
        frame
            .metrics
            .contains_key(metric::WAYPOINT_TARGET_SPEED_MPS)
    );
    assert!(
        frame
            .metrics
            .contains_key(metric::WAYPOINT_GUIDANCE_REQUIRED_ACCEL_RATIO)
    );
    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_GUIDANCE_PLAN_INDEX),
        Some(&TelemetryValue::from(0_i64))
    );
    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_GUIDANCE_PLAN_REASON),
        Some(&TelemetryValue::from("initial"))
    );
    for key in [
        metric::WAYPOINT_GUIDANCE_PLAN_AGE_S,
        metric::WAYPOINT_GUIDANCE_REFERENCE_POSITION_ERROR_M,
        metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_ERROR_M,
        metric::WAYPOINT_GUIDANCE_REFERENCE_VELOCITY_ERROR_MPS,
        metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_SPEED_ERROR_MPS,
        metric::WAYPOINT_GUIDANCE_AUTHORITY_MARGIN,
        metric::WAYPOINT_GUIDANCE_THRUST_SATURATED,
        metric::WAYPOINT_GUIDANCE_TILT_SATURATED,
    ] {
        assert!(frame.metrics.contains_key(key), "missing {key}");
    }
}

#[test]
fn transfer_waypoint_guidance_records_capture_and_advances_leg() {
    let ctx = context_with_waypoint();
    let waypoint = waypoint_fixture();
    let source = Vec2::new(-140.0, -780.0);
    let leg_unit = normalized_or_none(waypoint.position_m - source).unwrap();
    let final_leg_unit = normalized_or_none(Vec2::new(0.0, 0.0) - waypoint.position_m).unwrap();
    let mut config = TransferPdgControllerConfig::default();
    config.waypoint_guidance_enabled = true;
    let mut controller = TransferPdgController::new(config);
    let tracking_observation =
        waypoint_transfer_observation(&ctx, source + (leg_unit * 120.0), leg_unit * 30.0, 5.0);
    controller.update(&ctx, &tracking_observation);
    let mut observation = transfer_observation(205.0, -295.0, final_leg_unit * 40.0, 12.5);
    observation.position_m = waypoint.position_m + (leg_unit * 5.0);
    observation.target_surface_y_m = 0.0;

    let frame = controller.update(&ctx, &observation);

    assert_eq!(controller.waypoint_active_index, 1);
    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_CAPTURE_STATUS),
        Some(&TelemetryValue::from("captured"))
    );
    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_CAPTURE_TIME_S),
        Some(&TelemetryValue::from(12.5))
    );
    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_ACTIVE_INDEX),
        Some(&TelemetryValue::from(0_i64))
    );
    let handoff = frame
        .markers
        .iter()
        .find(|marker| marker.id == crate::kit::marker::WAYPOINT_HANDOFF)
        .expect("handoff marker");
    assert_eq!(
        handoff.metadata.get("waypoint.id"),
        Some(&TelemetryValue::from("wp_0"))
    );
    assert_eq!(
        handoff.metadata.get("waypoint.index"),
        Some(&TelemetryValue::from(0_i64))
    );
    assert!(
        handoff
            .metadata
            .contains_key(metric::WAYPOINT_GUIDANCE_REPLAN_COUNT)
    );
    assert_eq!(
        handoff.metadata.get(metric::WAYPOINT_HANDOFF_TARGET_MODE),
        Some(&TelemetryValue::from("waypoint_center"))
    );
    assert!(
        handoff
            .metadata
            .contains_key(metric::WAYPOINT_TARGET_DEADLINE_REMAINING_S)
    );
    assert!(
        handoff
            .metadata
            .contains_key(metric::WAYPOINT_TARGET_VELOCITY_ERROR_MPS)
    );
}

#[test]
fn transfer_waypoint_guidance_keeps_leg_active_until_window_contract_resolves() {
    let ctx = context_with_waypoint();
    let mut config = TransferPdgControllerConfig::default();
    config.waypoint_guidance_enabled = true;
    let mut controller = TransferPdgController::new(config);
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let entry = waypoint_transfer_observation(
        &ctx,
        geometry.target_m - (geometry.leg_unit * 5.0),
        geometry.handoff_tangent_unit * -30.0,
        10.0,
    );

    let entry_frame = controller.update(&ctx, &entry);

    assert_eq!(controller.waypoint_active_index, 0);
    assert_eq!(
        entry_frame.metrics.get(metric::WAYPOINT_CAPTURE_STATUS),
        Some(&TelemetryValue::from("capture_window"))
    );
    assert!(
        entry_frame
            .markers
            .iter()
            .all(|marker| marker.id != crate::kit::marker::WAYPOINT_HANDOFF)
    );

    let resolution = waypoint_transfer_observation(
        &ctx,
        geometry.target_m - geometry.leg_unit,
        geometry.handoff_tangent_unit * 30.0,
        10.5,
    );
    let resolution_frame = controller.update(&ctx, &resolution);

    assert_eq!(controller.waypoint_active_index, 1);
    let handoff = resolution_frame
        .markers
        .iter()
        .find(|marker| marker.id == crate::kit::marker::WAYPOINT_HANDOFF)
        .expect("resolved handoff marker");
    assert_eq!(
        handoff
            .metadata
            .get(metric::WAYPOINT_HANDOFF_RESOLUTION_REASON),
        Some(&TelemetryValue::from("contract_pass"))
    );
    assert_eq!(
        handoff
            .metadata
            .get(metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_PASS),
        Some(&TelemetryValue::from(false))
    );
    assert_eq!(
        handoff
            .metadata
            .get(metric::WAYPOINT_HANDOFF_WINDOW_DURATION_S),
        Some(&TelemetryValue::from(0.5))
    );
}

#[test]
fn transfer_waypoint_geometry_uses_explicit_handoff_tangent() {
    let mut ctx = context_with_waypoint();
    ctx.mission.transfer_route.as_mut().unwrap().waypoints[0].handoff_tangent_unit =
        Some(Vec2::new(0.0, 1.0));
    let controller = TransferPdgController::default();
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let observation =
        waypoint_transfer_observation(&ctx, geometry.target_m, Vec2::new(0.0, 30.0), 10.0);

    let stats = waypoint_leg_stats(&observation, &geometry);

    assert_eq!(geometry.handoff_tangent_unit, Vec2::new(0.0, 1.0));
    assert!(stats.outbound_heading_error_rad < 1.0e-9);
    assert!((stats.outbound_progress_mps - 30.0).abs() < 1.0e-9);
}

#[test]
fn transfer_waypoint_deadline_preserves_spatial_capture_status() {
    let ctx = context_with_waypoint();
    let mut config = TransferPdgControllerConfig::default();
    config.waypoint_guidance_enabled = true;
    let mut controller = TransferPdgController::new(config);
    let geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let entry = waypoint_transfer_observation(
        &ctx,
        geometry.target_m - (geometry.leg_unit * 5.0),
        geometry.handoff_tangent_unit * -30.0,
        10.0,
    );
    controller.update(&ctx, &entry);
    let deadline = waypoint_transfer_observation(
        &ctx,
        geometry.target_m + geometry.leg_unit,
        geometry.handoff_tangent_unit * -30.0,
        10.5,
    );

    let frame = controller.update(&ctx, &deadline);

    assert_eq!(
        frame.metrics.get(metric::WAYPOINT_CAPTURE_STATUS),
        Some(&TelemetryValue::from("captured"))
    );
    let handoff = frame
        .markers
        .iter()
        .find(|marker| marker.id == crate::kit::marker::WAYPOINT_HANDOFF)
        .unwrap();
    assert_eq!(
        handoff
            .metadata
            .get(metric::WAYPOINT_HANDOFF_RESOLUTION_REASON),
        Some(&TelemetryValue::from("plane_deadline"))
    );
}

#[test]
fn transfer_waypoint_guidance_emits_one_handoff_marker_per_switch() {
    let mut ctx = context_with_waypoint();
    let second_waypoint = TransferWaypointSpec {
        id: "wp_1".to_owned(),
        position_m: Vec2::new(-100.0, -150.0),
        ..waypoint_fixture()
    };
    ctx.mission
        .transfer_route
        .as_mut()
        .unwrap()
        .waypoints
        .push(second_waypoint);
    let explicit_second = TransferPdgController::waypoint_leg_geometry_at(&ctx, 1).unwrap();
    assert_eq!(explicit_second.active_index, 1);
    assert_eq!(
        explicit_second.anchor_m,
        ctx.mission.transfer_route.as_ref().unwrap().waypoints[0].position_m
    );
    let mut config = TransferPdgControllerConfig::default();
    config.waypoint_guidance_enabled = true;
    let mut controller = TransferPdgController::new(config);

    let first_geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let tracking_observation = waypoint_transfer_observation(
        &ctx,
        first_geometry.anchor_m + (first_geometry.leg_unit * 120.0),
        first_geometry.leg_unit * 30.0,
        5.0,
    );
    controller.update(&ctx, &tracking_observation);
    let first_observation = waypoint_transfer_observation(
        &ctx,
        first_geometry.target_m + (first_geometry.leg_unit * 5.0),
        first_geometry.handoff_tangent_unit * 40.0,
        10.0,
    );
    let first_frame = controller.update(&ctx, &first_observation);

    let second_geometry = controller.waypoint_leg_geometry(&ctx).unwrap();
    let second_observation = waypoint_transfer_observation(
        &ctx,
        second_geometry.target_m + (second_geometry.leg_unit * 5.0),
        second_geometry.handoff_tangent_unit * 40.0,
        20.0,
    );
    let second_frame = controller.update(&ctx, &second_observation);
    let repeated_frame = controller.update(&ctx, &second_observation);

    let handoff_indices = |frame: &ControllerFrame| {
        frame
            .markers
            .iter()
            .filter(|marker| marker.id == crate::kit::marker::WAYPOINT_HANDOFF)
            .map(|marker| marker.metadata.get("waypoint.index").cloned())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        handoff_indices(&first_frame),
        vec![Some(TelemetryValue::from(0_i64))]
    );
    assert_eq!(
        first_frame
            .metrics
            .get(metric::WAYPOINT_GUIDANCE_PLAN_INDEX),
        Some(&TelemetryValue::from(1_i64))
    );
    let first_handoff = first_frame
        .markers
        .iter()
        .find(|marker| marker.id == crate::kit::marker::WAYPOINT_HANDOFF)
        .unwrap();
    assert_eq!(
        first_handoff
            .metadata
            .get(metric::WAYPOINT_GUIDANCE_PLAN_INDEX),
        Some(&TelemetryValue::from(0_i64))
    );
    assert_eq!(
        handoff_indices(&second_frame),
        vec![Some(TelemetryValue::from(1_i64))]
    );
    assert!(handoff_indices(&repeated_frame).is_empty());
}

#[test]
fn transfer_projection_reports_descending_target_crossing() {
    let projection = transfer_ballistic_projection(100.0, -50.0, 10.0, 20.0, 10.0);

    assert!(projection.has_target_y_solution);
    assert!((projection.projected_time_s.unwrap() - 5.7416573868).abs() < 1.0e-6);
    assert!((projection.projected_dx_m.unwrap() - 42.583426132).abs() < 1.0e-6);
    assert!(projection.impact_angle_deg.unwrap() > 70.0);
    assert!((projection.apex_over_target_m - 70.0).abs() < 1.0e-9);
}

#[test]
fn transfer_projection_rejects_unreachable_target_y() {
    let projection = transfer_ballistic_projection(100.0, 100.0, 10.0, 20.0, 10.0);

    assert!(!projection.has_target_y_solution);
    assert_eq!(projection.projected_time_s, None);
    assert_eq!(projection.projected_dx_m, None);
    assert_eq!(projection.impact_angle_deg, None);
    assert!((projection.apex_over_target_m + 80.0).abs() < 1.0e-9);
}

#[test]
fn transfer_boost_quality_uses_projection_and_angle() {
    let controller = TransferPdgController::default();
    let good_projection = transfer_ballistic_projection(100.0, -50.0, 10.0, 20.0, 10.0);
    let good = controller.transfer_boost_quality(100.0, -50.0, good_projection);
    assert_eq!(good.verdict, "pass");
    assert!(good.passed);

    let no_solution = transfer_ballistic_projection(100.0, 100.0, 10.0, 20.0, 10.0);
    let no_solution_quality = controller.transfer_boost_quality(100.0, 100.0, no_solution);
    assert_eq!(no_solution_quality.verdict, "no_target_y_solution");
    assert!(!no_solution_quality.passed);

    let dx_miss = transfer_ballistic_projection(500.0, -50.0, 0.0, 20.0, 10.0);
    let dx_quality = controller.transfer_boost_quality(500.0, -50.0, dx_miss);
    assert_eq!(dx_quality.verdict, "dx");
    assert!(!dx_quality.passed);
}

#[test]
fn transfer_boost_continues_until_ballistic_quality_passes() {
    let ctx = uphill_transfer_context();
    let controller = TransferPdgController::default();
    let passing_observation = transfer_observation(100.0, 50.0, Vec2::new(10.0, 20.0), 6.0);
    let passing_diagnostics = controller.transfer_diagnostics(&passing_observation);
    assert!(passing_diagnostics.boost_quality.passed);
    assert!(!controller.boost_should_continue(&ctx, &passing_observation, passing_diagnostics));

    let missing_observation = transfer_observation(500.0, -100.0, Vec2::new(15.0, 5.0), 6.0);
    let missing_diagnostics = controller.transfer_diagnostics(&missing_observation);
    assert!(!missing_diagnostics.boost_quality.passed);
    assert!(controller.boost_should_continue(&ctx, &missing_observation, missing_diagnostics));
}

#[test]
fn transfer_uphill_boost_uses_vertical_bias_until_apex_is_safe() {
    let controller = TransferPdgController::default();
    let observation = transfer_observation(180.0, -360.0, Vec2::new(30.0, 10.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    assert!(diagnostics.route_dy_m > controller.config.uphill_boost_dy_min_m);
    assert!(
        !diagnostics.projection.has_target_y_solution
            || diagnostics.projection.apex_over_target_m
                < diagnostics.boost_quality.apex_target_over_target_m
    );
    assert_eq!(
        controller.boost_attitude_rad(&observation, diagnostics, TransferCorridorState::inactive()),
        controller.config.uphill_boost_tilt_rad
    );
}

#[test]
fn transfer_boost_quality_uses_frozen_shape_anchor() {
    let mut controller = TransferPdgController::default();
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 800.0,
        route_dy_m: 300.0,
    });
    let observation = transfer_observation(100.0, 50.0, Vec2::new(10.0, 20.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    assert_eq!(
        diagnostics.boost_quality.apex_target_over_target_m,
        controller.boost_apex_target_over_target_m(800.0, 300.0)
    );
    assert_ne!(
        diagnostics.boost_quality.apex_target_over_target_m,
        controller.boost_apex_target_over_target_m(100.0, -50.0)
    );
}

#[test]
fn transfer_steep_uphill_boost_stays_vertical_when_clearance_is_low() {
    let controller = TransferPdgController::default();
    let mut observation = transfer_observation(140.0, -780.0, Vec2::new(10.0, 15.0), 4.0);
    observation.touchdown_clearance_m = 35.0;
    let diagnostics = controller.transfer_diagnostics(&observation);

    let attitude_rad =
        controller.boost_attitude_rad(&observation, diagnostics, TransferCorridorState::inactive());

    assert!(attitude_rad > 0.0);
    assert!(attitude_rad < controller.config.uphill_boost_tilt_rad);
}

#[test]
fn transfer_boost_uses_projected_miss_direction_when_target_y_is_reachable() {
    let controller = TransferPdgController::default();
    let observation = transfer_observation(100.0, -50.0, Vec2::new(50.0, 50.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    assert!(diagnostics.projection.projected_dx_m.unwrap() < 0.0);
    assert!(
        controller.boost_attitude_rad(&observation, diagnostics, TransferCorridorState::inactive())
            < 0.0
    );
}

#[test]
fn transfer_boost_keeps_anchor_direction_until_target_y_is_reachable() {
    let mut controller = TransferPdgController::default();
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 700.0,
        route_dy_m: 400.0,
    });
    let observation = transfer_observation(500.0, -100.0, Vec2::new(15.0, 5.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    assert!(!diagnostics.projection.has_target_y_solution);
    assert!(
        controller.boost_attitude_rad(&observation, diagnostics, TransferCorridorState::inactive())
            > 0.0
    );
}

#[test]
fn transfer_boost_corrects_projected_overshoot_after_target_y_is_reachable() {
    let mut controller = TransferPdgController::default();
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 700.0,
        route_dy_m: 400.0,
    });
    let observation = transfer_observation(100.0, -50.0, Vec2::new(50.0, 50.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    assert!(diagnostics.projection.has_target_y_solution);
    assert!(
        diagnostics.projection.projected_dx_m.unwrap() < -controller.boost_dx_limit_m(&observation)
    );
    assert!(
        controller.boost_attitude_rad(&observation, diagnostics, TransferCorridorState::inactive())
            < 0.0
    );
    assert!(controller.boost_projected_overshoot(&observation, diagnostics));
}

#[test]
fn transfer_boost_cut_requires_clear_corridor_and_terminal_reserve() {
    let direct_controller = TransferPdgController::default();
    let mut waypoint_config = TransferPdgControllerConfig::default();
    waypoint_config.waypoint_guidance_enabled = true;
    let waypoint_controller = TransferPdgController::new(waypoint_config);
    let clear_corridor = TransferCorridorState::inactive();
    let active_corridor = TransferCorridorState {
        mode: "active",
        active: true,
        tilt_limited: false,
        margin_m: -1.0,
    };

    assert!(
        waypoint_controller
            .boost_cut_admissible(transfer_gate_fixture(2.0, 0.8, true), clear_corridor,)
    );
    assert!(
        !waypoint_controller
            .boost_cut_admissible(transfer_gate_fixture(2.0, 0.8, true), active_corridor,)
    );
    assert!(
        !waypoint_controller
            .boost_cut_admissible(transfer_gate_fixture(-0.1, 0.8, true), clear_corridor,)
    );
    assert!(
        !waypoint_controller
            .boost_cut_admissible(transfer_gate_fixture(2.0, 1.01, true), clear_corridor,)
    );
    assert!(
        !waypoint_controller
            .boost_cut_admissible(transfer_gate_fixture(2.0, 0.8, false), clear_corridor,)
    );
    assert!(
        direct_controller
            .boost_cut_admissible(transfer_gate_fixture(-0.1, 1.01, false), clear_corridor,)
    );
}

#[test]
fn transfer_boost_scorer_cannot_cut_thrust_without_terminal_reserve() {
    let ctx = uphill_transfer_context();
    let mut config = TransferPdgControllerConfig::default();
    config.waypoint_guidance_enabled = true;
    let mut controller = TransferPdgController::new(config);
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 700.0,
        route_dy_m: 400.0,
    });
    let observation = transfer_observation(100.0, -50.0, Vec2::new(50.0, 50.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let active_corridor = TransferCorridorState {
        mode: "active",
        active: true,
        tilt_limited: false,
        margin_m: -1.0,
    };

    assert!(controller.boost_projected_overshoot(&observation, diagnostics));
    let corridor_selection = controller.select_boost_command(
        &ctx,
        &observation,
        diagnostics,
        transfer_gate_fixture(2.0, 0.8, true),
        active_corridor,
    );
    let overdue_selection = controller.select_boost_command(
        &ctx,
        &observation,
        diagnostics,
        transfer_gate_fixture(-0.1, 0.8, true),
        TransferCorridorState::inactive(),
    );

    assert_eq!(corridor_selection.command.throttle_frac, 1.0);
    assert!(overdue_selection.command.throttle_frac > 0.0);
}

#[test]
fn transfer_coast_accepts_settled_ascending_solution_above_target_height() {
    let ctx = uphill_transfer_context();
    let mut controller = TransferPdgController::default();
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 700.0,
        route_dy_m: 400.0,
    });
    let observation = transfer_observation(300.0, 200.0, Vec2::new(25.0, 45.0), 12.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    assert!(diagnostics.boost_quality.passed);
    assert!(
        controller
            .boost_settled_quality(&ctx, &observation, diagnostics)
            .quality
            .passed
    );
    assert!(controller.should_coast(
        &ctx,
        &observation,
        diagnostics,
        TransferCorridorState::inactive()
    ));
}

#[test]
fn transfer_coast_prealigns_upright_retrograde() {
    let controller = TransferPdgController::default();
    let observation = transfer_observation(400.0, 300.0, Vec2::new(45.0, -8.0), 10.0);

    let attitude_rad = controller.coast_attitude_rad(&observation);

    assert!(attitude_rad < -0.5);
    assert!(attitude_rad.abs() <= controller.config.terminal.terminal_overshoot_tilt_max_rad);
}

#[test]
fn transfer_coast_avoids_max_retrograde_tilt_while_climbing() {
    let controller = TransferPdgController::default();
    let observation = transfer_observation(400.0, 300.0, Vec2::new(26.0, 35.0), 10.0);

    let attitude_rad = controller.coast_attitude_rad(&observation);

    assert!(attitude_rad < 0.0);
    assert!(attitude_rad.abs() < 0.8);
}

#[test]
fn transfer_reset_clears_boost_anchor_and_gate_state() {
    let ctx = uphill_transfer_context();
    let mut controller = TransferPdgController::default();
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 140.0,
        route_dy_m: 780.0,
    });
    controller.transfer_gate_ready_ticks = 3;
    controller.last_transfer_gate = Some(TerminalEntryAssessment {
        mode: TerminalEntryMode::NominalReady,
        ready_ticks: 3,
        burn_time_s: 5.0,
        latest_safe_margin_s: 2.0,
        required_accel_ratio: 0.4,
        terrain_min_clearance_m: 20.0,
        terrain_clearance_safe: true,
        deferred: false,
    });
    controller.last_corridor = TransferCorridorState {
        mode: "active",
        active: true,
        tilt_limited: true,
        margin_m: -10.0,
    };

    controller.reset(&ctx);

    assert!(controller.boost_anchor.is_none());
    assert_eq!(controller.transfer_gate_ready_ticks, 0);
    assert_eq!(controller.last_transfer_gate, None);
    assert_eq!(controller.last_corridor, TransferCorridorState::inactive());
}

#[test]
fn transfer_default_extends_terminal_gate_horizon_without_changing_terminal_default() {
    let terminal = TerminalPdgControllerConfig::default();
    let transfer = TransferPdgControllerConfig::default();

    assert_eq!(terminal.terminal_gate_burn_time_max_s, 14.0);
    assert_eq!(terminal.terminal_gate_burn_time_offset_long_s, 0.8);
    assert_eq!(transfer.terminal.terminal_gate_burn_time_max_s, 22.0);
    assert_eq!(transfer.terminal.terminal_gate_burn_time_offset_long_s, 2.0);
}

#[test]
fn transfer_gate_forces_pending_without_target_y_solution() {
    let ctx = uphill_transfer_context();
    let controller = TransferPdgController::default();
    let mut observation = transfer_observation(140.0, -780.0, Vec2::new(10.0, 15.0), 6.0);
    observation.position_m = Vec2::new(-140.0, -780.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    let gate = controller.transfer_gate_readiness(&ctx, &observation, diagnostics);

    assert!(!diagnostics.projection.has_target_y_solution);
    assert_eq!(gate.mode, TerminalEntryMode::Pending);
    assert_eq!(gate.ready_ticks, 0);
}

#[test]
fn transfer_uphill_corridor_caps_boost_tilt() {
    let ctx = uphill_transfer_context();
    let controller = TransferPdgController::default();
    let mut observation = transfer_observation(140.0, -780.0, Vec2::new(10.0, 15.0), 6.0);
    observation.position_m = Vec2::new(-140.0, -780.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

    let attitude_rad = controller.boost_attitude_rad(&observation, diagnostics, corridor);

    assert!(corridor.active);
    assert!(corridor.tilt_limited);
    assert!(corridor.margin_m < 0.0);
    assert!(attitude_rad.abs() <= TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD);
    assert!(attitude_rad.abs() < controller.config.uphill_boost_tilt_rad);
}

#[test]
fn transfer_uphill_corridor_brakes_targetward_lateral_speed() {
    let ctx = uphill_transfer_context();
    let controller = TransferPdgController::default();
    let mut observation = transfer_observation(140.0, -780.0, Vec2::new(10.0, 15.0), 6.0);
    observation.position_m = Vec2::new(-140.0, -780.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

    let brake_attitude_rad = controller
        .corridor_lateral_brake_attitude_rad(&observation, diagnostics, corridor)
        .expect("targetward speed should trigger corridor braking");
    let boost_attitude_rad = controller.boost_attitude_rad(&observation, diagnostics, corridor);
    let selection = controller.select_boost_command(
        &ctx,
        &observation,
        diagnostics,
        transfer_gate_fixture(2.0, 0.8, true),
        corridor,
    );

    assert!(corridor.active);
    assert!(corridor.tilt_limited);
    assert_eq!(brake_attitude_rad, -TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD);
    assert_eq!(boost_attitude_rad, brake_attitude_rad);
    assert!(selection.command.target_attitude_rad <= 0.0);
}

#[test]
fn transfer_uphill_corridor_brake_waits_for_targetward_speed() {
    let ctx = uphill_transfer_context();
    let controller = TransferPdgController::default();
    let mut observation = transfer_observation(140.0, -780.0, Vec2::new(1.0, 15.0), 6.0);
    observation.position_m = Vec2::new(-140.0, -780.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

    let brake_attitude_rad =
        controller.corridor_lateral_brake_attitude_rad(&observation, diagnostics, corridor);
    let boost_attitude_rad = controller.boost_attitude_rad(&observation, diagnostics, corridor);

    assert!(corridor.active);
    assert!(corridor.tilt_limited);
    assert_eq!(brake_attitude_rad, None);
    assert!(boost_attitude_rad > 0.0);
    assert!(boost_attitude_rad <= TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD);
}

#[test]
fn transfer_moderate_uphill_corridor_does_not_cap_boost_tilt() {
    let mut ctx = uphill_transfer_context();
    ctx.world.terrain = TerrainDefinition::Heightfield {
        points_m: vec![
            Vec2::new(-740.0, -400.0),
            Vec2::new(-700.0, -400.0),
            Vec2::new(-18.0, 0.0),
            Vec2::new(18.0, 0.0),
            Vec2::new(180.0, 0.0),
        ],
    };
    let controller = TransferPdgController::default();
    let mut observation = transfer_observation(700.0, -400.0, Vec2::new(10.0, 15.0), 6.0);
    observation.position_m = Vec2::new(-700.0, -400.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

    assert!(corridor.active);
    assert!(!corridor.tilt_limited);
}

#[test]
fn transfer_boost_throttle_eases_when_apex_is_already_high() {
    let ctx = uphill_transfer_context();
    let controller = TransferPdgController::default();
    let observation = transfer_observation(700.0, -200.0, Vec2::new(10.0, 100.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    let throttle = controller.boost_throttle_frac(
        &ctx,
        &observation,
        diagnostics,
        TransferCorridorState::inactive(),
        0.3,
    );

    assert!(
        diagnostics.projection.apex_over_target_m
            > diagnostics.boost_quality.apex_target_over_target_m
    );
    assert!(throttle < 1.0);
    assert!(throttle >= ctx.vehicle.min_throttle_frac);
}

#[test]
fn transfer_boost_throttle_stays_full_while_corridor_active() {
    let ctx = uphill_transfer_context();
    let controller = TransferPdgController::default();
    let observation = transfer_observation(700.0, -200.0, Vec2::new(10.0, 100.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let corridor = TransferCorridorState {
        mode: "active",
        active: true,
        tilt_limited: false,
        margin_m: 40.0,
    };

    let throttle = controller.boost_throttle_frac(&ctx, &observation, diagnostics, corridor, 0.3);

    assert_eq!(throttle, 1.0);
}

#[test]
fn transfer_boost_scorer_reduces_throttle_when_apex_is_high() {
    let ctx = uphill_transfer_context();
    let mut controller = TransferPdgController::default();
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 100.0,
        route_dy_m: -50.0,
    });
    let observation = transfer_observation(100.0, 50.0, Vec2::new(10.0, 100.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    let selection = controller.select_boost_command(
        &ctx,
        &observation,
        diagnostics,
        transfer_gate_fixture(2.0, 0.8, true),
        TransferCorridorState::inactive(),
    );

    assert!(selection.selected_score.is_finite());
    assert!(selection.command.throttle_frac < 1.0);
    assert!(selection.command.throttle_frac >= ctx.vehicle.min_throttle_frac);
}

#[test]
fn transfer_pathwise_alias_enables_pathwise_boost_scoring() {
    let spec = built_in_controller_spec("transfer_pdg_pathwise")
        .expect("pathwise transfer controller alias should exist");

    match spec {
        ControllerSpec::TransferPdgV1 { config } => {
            assert!(config.boost_pathwise_scoring_enabled);
            assert_eq!(
                ControllerSpec::TransferPdgV1 { config }.id(),
                "transfer_pdg_pathwise_v1"
            );
        }
        _ => panic!("pathwise alias should resolve to transfer controller"),
    }
}

#[test]
fn transfer_recoverability_alias_enables_recoverability_boost_scoring() {
    let spec = built_in_controller_spec("transfer_pdg_recoverability")
        .expect("recoverability transfer controller alias should exist");

    match spec {
        ControllerSpec::TransferPdgV1 { config } => {
            assert!(config.boost_recoverability_scoring_enabled);
            assert!(!config.boost_pathwise_scoring_enabled);
            assert_eq!(
                ControllerSpec::TransferPdgV1 { config }.id(),
                "transfer_pdg_recoverability_v1"
            );
        }
        _ => panic!("recoverability alias should resolve to transfer controller"),
    }
}

#[test]
fn transfer_pathwise_scorer_keeps_targetward_tilt_for_shortfall() {
    let ctx = uphill_transfer_context();
    let mut config = TransferPdgControllerConfig::default();
    config.boost_pathwise_scoring_enabled = true;
    let mut controller = TransferPdgController::new(config);
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 500.0,
        route_dy_m: 120.0,
    });
    let observation = transfer_observation(500.0, -120.0, Vec2::new(5.0, 30.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    let selection = controller.select_boost_command(
        &ctx,
        &observation,
        diagnostics,
        transfer_gate_fixture(2.0, 0.8, true),
        TransferCorridorState::inactive(),
    );

    assert_eq!(controller.boost_scoring_mode(), "pathwise_geometry");
    assert!(selection.selected_score.is_finite());
    assert!(selection.command.target_attitude_rad > 0.0);
    assert!(selection.command.throttle_frac >= 0.7);
}

#[test]
fn transfer_pathwise_scorer_penalizes_away_thrust_outside_corridor() {
    let mut config = TransferPdgControllerConfig::default();
    config.boost_pathwise_scoring_enabled = true;
    let mut controller = TransferPdgController::new(config);
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 500.0,
        route_dy_m: 120.0,
    });
    let observation = transfer_observation(500.0, -120.0, Vec2::new(5.0, 20.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let dx_limit_m = controller.boost_dx_limit_m(&observation);

    let away = controller.score_boost_no_away_penalty(
        &observation,
        diagnostics,
        Command {
            throttle_frac: 1.0,
            target_attitude_rad: -0.3,
        },
        dx_limit_m,
    );
    let targetward = controller.score_boost_no_away_penalty(
        &observation,
        diagnostics,
        Command {
            throttle_frac: 1.0,
            target_attitude_rad: 0.3,
        },
        dx_limit_m,
    );

    assert!(away > 0.0);
    assert_eq!(targetward, 0.0);
}

#[test]
fn transfer_pathwise_scorer_is_finite_without_target_y_solution() {
    let ctx = uphill_transfer_context();
    let mut config = TransferPdgControllerConfig::default();
    config.boost_pathwise_scoring_enabled = true;
    let controller = TransferPdgController::new(config);
    let observation = transfer_observation(500.0, -220.0, Vec2::new(5.0, 5.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    assert!(!diagnostics.projection.has_target_y_solution);
    let score = controller.score_boost_candidate(
        &ctx,
        &observation,
        diagnostics,
        TransferCorridorState::inactive(),
        Command {
            throttle_frac: 1.0,
            target_attitude_rad: 0.3,
        },
    );

    assert!(score.score.is_finite());
    assert!(!score.quality.passed);
}

#[test]
fn transfer_recoverability_scorer_penalizes_overdue_terminal_gate() {
    let controller = TransferPdgController::default();
    let observation = transfer_observation(220.0, 80.0, Vec2::new(35.0, -18.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let ready = TerminalEntryAssessment {
        mode: TerminalEntryMode::NominalReady,
        ready_ticks: 2,
        burn_time_s: 2.0,
        latest_safe_margin_s: 0.5,
        required_accel_ratio: 0.8,
        terrain_min_clearance_m: 80.0,
        terrain_clearance_safe: true,
        deferred: false,
    };
    let overdue = TerminalEntryAssessment {
        mode: TerminalEntryMode::LatestSafe,
        ready_ticks: 0,
        burn_time_s: 2.0,
        latest_safe_margin_s: -4.0,
        required_accel_ratio: 0.8,
        terrain_min_clearance_m: 80.0,
        terrain_clearance_safe: true,
        deferred: false,
    };

    let ready_score = controller.score_boost_candidate_recoverability_terms(
        &observation,
        diagnostics,
        ready,
        controller.boost_dx_limit_m(&observation),
    );
    let overdue_score = controller.score_boost_candidate_recoverability_terms(
        &observation,
        diagnostics,
        overdue,
        controller.boost_dx_limit_m(&observation),
    );

    assert!(overdue_score > ready_score + 100.0);
}

#[test]
fn transfer_recoverability_scorer_prefers_margin_over_lower_accel_ratio() {
    let controller = TransferPdgController::default();
    let observation = transfer_observation(220.0, 80.0, Vec2::new(35.0, -18.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let better_margin = TerminalEntryAssessment {
        mode: TerminalEntryMode::LatestSafe,
        ready_ticks: 0,
        burn_time_s: 2.0,
        latest_safe_margin_s: -2.0,
        required_accel_ratio: 8.0,
        terrain_min_clearance_m: 80.0,
        terrain_clearance_safe: true,
        deferred: false,
    };
    let lower_accel = TerminalEntryAssessment {
        mode: TerminalEntryMode::LatestSafe,
        ready_ticks: 0,
        burn_time_s: 2.0,
        latest_safe_margin_s: -4.0,
        required_accel_ratio: 2.0,
        terrain_min_clearance_m: 80.0,
        terrain_clearance_safe: true,
        deferred: false,
    };

    let better_margin_score = controller.score_boost_candidate_recoverability_terms(
        &observation,
        diagnostics,
        better_margin,
        controller.boost_dx_limit_m(&observation),
    );
    let lower_accel_score = controller.score_boost_candidate_recoverability_terms(
        &observation,
        diagnostics,
        lower_accel,
        controller.boost_dx_limit_m(&observation),
    );

    assert!(better_margin_score < lower_accel_score);
}

#[test]
fn transfer_recoverability_scorer_is_finite_without_target_y_solution() {
    let ctx = uphill_transfer_context();
    let mut config = TransferPdgControllerConfig::default();
    config.boost_recoverability_scoring_enabled = true;
    let controller = TransferPdgController::new(config);
    let observation = transfer_observation(500.0, -220.0, Vec2::new(5.0, 5.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    assert_eq!(controller.boost_scoring_mode(), "recoverability");
    assert!(!diagnostics.projection.has_target_y_solution);
    let score = controller.score_boost_candidate(
        &ctx,
        &observation,
        diagnostics,
        TransferCorridorState::inactive(),
        Command {
            throttle_frac: 1.0,
            target_attitude_rad: 0.3,
        },
    );

    assert!(score.score.is_finite());
    assert!(!score.quality.passed);
}

#[test]
fn transfer_boost_scorer_keeps_targetward_tilt_for_shortfall() {
    let ctx = uphill_transfer_context();
    let mut controller = TransferPdgController::default();
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 500.0,
        route_dy_m: 120.0,
    });
    let observation = transfer_observation(500.0, -120.0, Vec2::new(5.0, 30.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    let selection = controller.select_boost_command(
        &ctx,
        &observation,
        diagnostics,
        transfer_gate_fixture(2.0, 0.8, true),
        TransferCorridorState::inactive(),
    );

    assert!(selection.command.target_attitude_rad > 0.0);
    assert!(selection.command.throttle_frac >= 0.7);
}

#[test]
fn transfer_boost_scorer_respects_corridor_tilt_cap() {
    let ctx = uphill_transfer_context();
    let mut controller = TransferPdgController::default();
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 140.0,
        route_dy_m: 780.0,
    });
    let mut observation = transfer_observation(140.0, -780.0, Vec2::new(10.0, 15.0), 6.0);
    observation.position_m = Vec2::new(-140.0, -780.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

    let selection = controller.select_boost_command(
        &ctx,
        &observation,
        diagnostics,
        transfer_gate_fixture(2.0, 0.8, true),
        corridor,
    );

    assert!(corridor.tilt_limited);
    assert!(selection.command.target_attitude_rad.abs() <= TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD);
}

#[test]
fn transfer_boost_settled_quality_keeps_passive_projection_stable() {
    let ctx = uphill_transfer_context();
    let mut controller = TransferPdgController::default();
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 100.0,
        route_dy_m: -50.0,
    });
    let observation = transfer_observation(100.0, 50.0, Vec2::new(10.0, 20.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    let settled = controller.boost_settled_quality(&ctx, &observation, diagnostics);

    assert!(diagnostics.boost_quality.passed);
    assert!(settled.quality.passed);
    assert!(
        (settled.projection.projected_dx_m.unwrap()
            - diagnostics.projection.projected_dx_m.unwrap())
        .abs()
            < 2.0
    );
}

#[test]
fn transfer_latest_safe_deferral_respects_guard_conditions() {
    let ctx = uphill_transfer_context();
    let controller = TransferPdgController::default();
    let gate = TerminalEntryAssessment {
        mode: TerminalEntryMode::LatestSafe,
        ready_ticks: 0,
        burn_time_s: 5.0,
        latest_safe_margin_s: 0.0,
        required_accel_ratio: 0.8,
        terrain_min_clearance_m: 20.0,
        terrain_clearance_safe: true,
        deferred: false,
    };
    let descending = transfer_observation(100.0, 80.0, Vec2::new(8.0, -2.0), 6.0);
    let descending_diagnostics = controller.transfer_diagnostics(&descending);
    assert!(!controller.should_defer_latest_safe_transfer_gate(
        &ctx,
        &descending,
        descending_diagnostics,
        gate
    ));

    let out_of_band = transfer_observation(500.0, 80.0, Vec2::new(5.0, 12.0), 6.0);
    let out_of_band_diagnostics = controller.transfer_diagnostics(&out_of_band);
    assert!(!controller.should_defer_latest_safe_transfer_gate(
        &ctx,
        &out_of_band,
        out_of_band_diagnostics,
        gate
    ));
}

#[test]
fn transfer_uphill_corridor_releases_after_local_clearance() {
    let ctx = uphill_transfer_context();
    let controller = TransferPdgController::default();
    let mut observation = transfer_observation(140.0, -450.0, Vec2::new(10.0, 15.0), 6.0);
    observation.position_m = Vec2::new(-140.0, -450.0);
    let diagnostics = controller.transfer_diagnostics(&observation);

    let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

    assert_eq!(corridor.mode, "clear");
    assert!(!corridor.active);
    assert!(corridor.margin_m > 0.0);
}

#[test]
fn transfer_short_downhill_route_stays_terminal_owned() {
    let ctx = uphill_transfer_context();
    let controller = TransferPdgController::default();
    let mut observation = transfer_observation(140.0, 320.0, Vec2::new(0.0, -4.0), 6.0);
    observation.position_m = Vec2::new(-140.0, 320.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let gate = controller.transfer_gate_readiness(&ctx, &observation, diagnostics);
    let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

    let phase = controller.choose_phase(&ctx, &observation, diagnostics, gate, corridor);

    assert_eq!(phase, TransferPhase::Terminal);
}

#[test]
fn transfer_short_downhill_route_holds_takeoff_until_source_clearance_safe() {
    let mut ctx = uphill_transfer_context();
    ctx.world.terrain = TerrainDefinition::Heightfield {
        points_m: vec![
            Vec2::new(-120.0, 400.0),
            Vec2::new(-70.0, 400.0),
            Vec2::new(-51.0, 400.0),
            Vec2::new(-18.0, 0.0),
            Vec2::new(80.0, 0.0),
        ],
    };
    ctx.world.landing_pads = vec![
        LandingPadSpec {
            id: "source".to_owned(),
            center_x_m: -70.0,
            surface_y_m: 400.0,
            width_m: 36.0,
        },
        LandingPadSpec {
            id: "target".to_owned(),
            center_x_m: 0.0,
            surface_y_m: 0.0,
            width_m: 36.0,
        },
    ];
    ctx.target_pad = ctx.world.landing_pad("target").unwrap().clone();
    let route = ctx.mission.transfer_route.as_mut().unwrap();
    route.route_angle_deg = -80.0;
    route.route_radius_m = 400.0;

    let controller = TransferPdgController::default();
    let mut observation = transfer_observation(70.0, 405.0, Vec2::new(0.0, 8.2), 2.4);
    observation.position_m = Vec2::new(-70.0, 405.0);
    observation.touchdown_clearance_m = 5.0;
    let diagnostics = controller.transfer_diagnostics(&observation);
    let gate = controller.transfer_gate_readiness(&ctx, &observation, diagnostics);
    let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

    let held_phase = controller.choose_phase(&ctx, &observation, diagnostics, gate, corridor);

    let mut terminal_controller = TransferPdgController::default();
    terminal_controller.phase = TransferPhase::Terminal;
    assert!(terminal_controller.source_clearance_hold_needed(&ctx, &observation));
    let direct_terminal_phase =
        terminal_controller.choose_phase(&ctx, &observation, diagnostics, gate, corridor);

    ctx.mission
        .transfer_route
        .as_mut()
        .unwrap()
        .waypoints
        .push(waypoint_fixture());
    terminal_controller.waypoint_active_index = 1;
    let completed_waypoint_terminal_phase =
        terminal_controller.choose_phase(&ctx, &observation, diagnostics, gate, corridor);

    observation.position_m.y = 430.0;
    observation.height_above_target_m = 430.0;
    observation.touchdown_clearance_m = 30.0;
    let safe_diagnostics = controller.transfer_diagnostics(&observation);
    let safe_gate = controller.transfer_gate_readiness(&ctx, &observation, safe_diagnostics);
    let safe_corridor = controller.transfer_corridor_state(&ctx, &observation, safe_diagnostics);
    let released_phase = controller.choose_phase(
        &ctx,
        &observation,
        safe_diagnostics,
        safe_gate,
        safe_corridor,
    );

    assert_eq!(held_phase, TransferPhase::Takeoff);
    assert_eq!(direct_terminal_phase, TransferPhase::Takeoff);
    assert_eq!(completed_waypoint_terminal_phase, TransferPhase::Terminal);
    assert_eq!(released_phase, TransferPhase::Terminal);
}

#[test]
fn transfer_coast_captures_terminal_before_uphill_target_crossing() {
    let ctx = uphill_transfer_context();
    let mut controller = TransferPdgController::default();
    controller.phase = TransferPhase::Coast;
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 565.0,
        route_dy_m: 560.0,
    });
    let observation = transfer_observation(294.0, -60.0, Vec2::new(37.4, 50.8), 14.5);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let gate = controller.transfer_gate_readiness(&ctx, &observation, diagnostics);

    assert_eq!(gate.mode, TerminalEntryMode::Pending);
    assert!(diagnostics.boost_quality.passed);
    assert!(
        controller
            .next_target_y_crossing_time_s(&observation)
            .unwrap()
            <= TRANSFER_PRE_TARGET_CAPTURE_LOOKAHEAD_S
    );
    assert!(controller.pre_target_terminal_capture_ready(&observation, diagnostics, gate));

    let phase = controller.choose_phase(
        &ctx,
        &observation,
        diagnostics,
        gate,
        TransferCorridorState::inactive(),
    );

    assert_eq!(phase, TransferPhase::Terminal);
}

#[test]
fn transfer_coast_waits_when_uphill_target_crossing_is_not_imminent() {
    let ctx = uphill_transfer_context();
    let mut controller = TransferPdgController::default();
    controller.phase = TransferPhase::Coast;
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 565.0,
        route_dy_m: 560.0,
    });
    let observation = transfer_observation(410.0, -120.0, Vec2::new(37.4, 50.8), 13.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let gate = controller.transfer_gate_readiness(&ctx, &observation, diagnostics);

    assert!(
        controller
            .next_target_y_crossing_time_s(&observation)
            .unwrap()
            > TRANSFER_PRE_TARGET_CAPTURE_LOOKAHEAD_S
    );
    assert!(!controller.pre_target_terminal_capture_ready(&observation, diagnostics, gate));
}

#[test]
fn transfer_started_route_waits_for_ready_gate_before_terminal() {
    let ctx = uphill_transfer_context();
    let mut controller = TransferPdgController::default();
    controller.phase = TransferPhase::Boost;
    controller.boost_anchor = Some(TransferBoostAnchor {
        route_dx_m: 700.0,
        route_dy_m: 400.0,
    });
    let observation = transfer_observation(120.0, -20.0, Vec2::new(20.0, 5.0), 6.0);
    let diagnostics = controller.transfer_diagnostics(&observation);
    let gate = TerminalEntryAssessment {
        mode: TerminalEntryMode::Pending,
        ready_ticks: 0,
        burn_time_s: 5.0,
        latest_safe_margin_s: 2.0,
        required_accel_ratio: 0.8,
        terrain_min_clearance_m: 20.0,
        terrain_clearance_safe: true,
        deferred: false,
    };

    let phase = controller.choose_phase(
        &ctx,
        &observation,
        diagnostics,
        gate,
        TransferCorridorState::inactive(),
    );

    assert_ne!(phase, TransferPhase::Terminal);
}
