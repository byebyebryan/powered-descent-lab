
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use super::*;
use pd_core::{
    EvaluationGoal, LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig, TerrainDefinition, Vec2,
    VehicleGeometry, VehicleInitialState, VehicleSpec, WorldSpec,
};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures")
}

fn temp_fixture_root(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("pd_eval_{prefix}_{unique}"));
    fs::create_dir_all(root.join("scenarios")).expect("temp fixture root should be creatable");
    root
}

fn write_scenario(root: &Path, relative_path: &str, scenario: &ScenarioSpec) {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("scenario parent directory should be creatable");
    }
    fs::write(
        &path,
        serde_json::to_vec_pretty(scenario).expect("scenario json should serialize"),
    )
    .expect("scenario json should be writable");
}

fn report_with_records(mut report: BatchReport, records: Vec<BatchRunRecord>) -> BatchReport {
    report.total_runs = records.len();
    report.resolved_runs = records
        .iter()
        .map(|record| record.resolved.clone())
        .collect();
    report.summary = summarize_records(&records);
    report.records = records;
    report
}

#[test]
fn maintained_clean_terminal_packs_expand_only_the_current_controller_lane() {
    let packs_dir = fixtures_root().join("packs");
    for (filename, expected_runs) in [
        ("terminal_bot_lab_suite.json", 189),
        ("terminal_bot_lab_full.json", 756),
    ] {
        let pack = load_pack(&packs_dir.join(filename)).unwrap();
        let runs = resolve_pack_runs(&pack, &packs_dir).unwrap();

        assert_eq!(runs.len(), expected_runs, "unexpected size for {filename}");
        assert!(runs.iter().all(|run| {
            run.descriptor.lane_id == "current" && run.descriptor.controller_id == "terminal_pdg_v1"
        }));
    }
}

fn easy_landing_scenario() -> ScenarioSpec {
    ScenarioSpec {
        id: "unit_flat_landing".to_owned(),
        name: "Unit flat landing".to_owned(),
        description: "Low-gravity flat landing fixture for eval tests".to_owned(),
        seed: 3,
        tags: vec!["test".to_owned(), "landing".to_owned()],
        metadata: BTreeMap::from([("suite".to_owned(), "eval".to_owned())]),
        sim: SimConfig {
            physics_hz: 120,
            controller_hz: 60,
            max_time_s: 45.0,
            sample_hz: Some(10),
        },
        world: WorldSpec {
            gravity_mps2: 1.62,
            terrain: TerrainDefinition::Heightfield {
                points_m: vec![Vec2::new(-120.0, 0.0), Vec2::new(120.0, 0.0)],
            },
            landing_pads: vec![LandingPadSpec {
                id: "pad_a".to_owned(),
                center_x_m: 0.0,
                surface_y_m: 0.0,
                width_m: 36.0,
            }],
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
            position_m: Vec2::new(18.0, 140.0),
            velocity_mps: Vec2::new(-1.0, -12.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        },
        mission: MissionSpec {
            transfer_route: None,
            goal: EvaluationGoal::LandingOnPad {
                target_pad_id: "pad_a".to_owned(),
            },
        },
    }
}

fn waypoint_contract_scenario() -> ScenarioSpec {
    let mut scenario = easy_landing_scenario();
    scenario.world.landing_pads.push(LandingPadSpec {
        id: "source".to_owned(),
        center_x_m: -420.0,
        surface_y_m: 120.0,
        width_m: 36.0,
    });
    scenario.mission.transfer_route = Some(TransferRouteSpec {
        source_pad_id: "source".to_owned(),
        target_pad_id: "pad_a".to_owned(),
        route_angle_deg: 80.0,
        route_radius_m: 800.0,
        waypoints: vec![TransferWaypointSpec {
            id: "wp_0".to_owned(),
            position_m: Vec2::new(-220.0, 180.0),
            handoff_tangent_unit: None,
            capture_radius_m: 40.0,
            max_cross_track_m: 50.0,
            max_outbound_heading_error_rad: 0.85,
            min_outbound_progress_mps: 8.0,
            max_outbound_cross_speed_mps: None,
            min_speed_mps: 10.0,
            max_speed_mps: 130.0,
            min_vertical_speed_mps: Some(-80.0),
            max_vertical_speed_mps: Some(65.0),
        }],
    });
    scenario
}

fn easy_checkpoint_scenario() -> ScenarioSpec {
    ScenarioSpec {
        id: "unit_timed_checkpoint".to_owned(),
        name: "Unit timed checkpoint".to_owned(),
        description: "Stationary timed checkpoint fixture for eval tests".to_owned(),
        seed: 5,
        tags: vec!["test".to_owned(), "checkpoint".to_owned()],
        metadata: BTreeMap::from([("suite".to_owned(), "eval".to_owned())]),
        sim: SimConfig {
            physics_hz: 120,
            controller_hz: 60,
            max_time_s: 5.0,
            sample_hz: Some(10),
        },
        world: WorldSpec {
            gravity_mps2: 1.62,
            terrain: TerrainDefinition::Heightfield {
                points_m: vec![Vec2::new(-80.0, 0.0), Vec2::new(80.0, 0.0)],
            },
            landing_pads: vec![LandingPadSpec {
                id: "pad_a".to_owned(),
                center_x_m: 0.0,
                surface_y_m: 0.0,
                width_m: 36.0,
            }],
        },
        vehicle: VehicleSpec {
            geometry: VehicleGeometry {
                hull_width_m: 4.0,
                hull_height_m: 6.0,
                touchdown_half_span_m: 2.0,
                touchdown_base_offset_m: 3.2,
            },
            dry_mass_kg: 700.0,
            initial_fuel_kg: 40.0,
            max_fuel_kg: 40.0,
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
            position_m: Vec2::new(0.0, 10.0),
            velocity_mps: Vec2::new(0.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        },
        mission: MissionSpec {
            transfer_route: None,
            goal: EvaluationGoal::TimedCheckpoint {
                target_pad_id: "pad_a".to_owned(),
                end_time_s: 0.5,
                desired_position_offset_m: Vec2::new(0.0, 9.794125),
                max_position_error_m: 0.01,
                desired_velocity_mps: Vec2::new(0.0, -0.81),
                max_velocity_error_mps: 0.01,
                max_attitude_error_rad: 0.01,
            },
        },
    }
}

#[test]
fn run_pack_aggregates_nominal_suite() {
    let base_dir = temp_fixture_root("unit_pack");
    write_scenario(
        &base_dir,
        "scenarios/checkpoint_success.json",
        &easy_checkpoint_scenario(),
    );
    let pack = ScenarioPackSpec {
        id: "unit_pack".to_owned(),
        name: "Unit pack".to_owned(),
        description: "unit pack".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![
            ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "checkpoint_success_baseline".to_owned(),
                scenario: "scenarios/checkpoint_success.json".to_owned(),
                controller: "baseline".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            }),
            ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "checkpoint_success_idle".to_owned(),
                scenario: "scenarios/checkpoint_success.json".to_owned(),
                controller: "idle".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            }),
        ],
    };

    let report = run_pack(&pack, &base_dir, None).unwrap();

    assert_eq!(report.total_runs, 2);
    assert_eq!(report.summary.success_runs, 2);
    assert_eq!(
        report.summary.mission_outcomes.get("success").copied(),
        Some(2)
    );
    assert_eq!(report.identity.schema_version, BATCH_REPORT_SCHEMA_VERSION);
}

#[test]
fn family_entry_expands_deterministically_across_workers() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "family_pack".to_owned(),
        name: "Family pack".to_owned(),
        description: "family pack".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![
            ScenarioPackEntry::Family(ScenarioFamilyEntry {
                id: "terminal_sweep".to_owned(),
                family: "terminal_nominal".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                controller: "baseline".to_owned(),
                controller_config: None,
                seeds: vec![0, 1, 2],
                seed_range: None,
                perturbations: vec![
                    NumericPerturbationSpec {
                        id: "spawn_dx".to_owned(),
                        path: "initial_state.position_m.x".to_owned(),
                        mode: NumericPerturbationMode::Offset,
                        min: -12.0,
                        max: 12.0,
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
                tags: vec!["sweep".to_owned()],
                metadata: BTreeMap::from([
                    ("difficulty".to_owned(), "sweep".to_owned()),
                    ("mission".to_owned(), "terminal_guidance".to_owned()),
                    (
                        "arrival_family".to_owned(),
                        "seeded_terminal_arrival_v0".to_owned(),
                    ),
                    ("condition_set".to_owned(), "clean".to_owned()),
                    ("vehicle_variant".to_owned(), "nominal".to_owned()),
                    ("expectation_tier".to_owned(), "core".to_owned()),
                ]),
            }),
            ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "checkpoint".to_owned(),
                scenario: "scenarios/timed_checkpoint_idle.json".to_owned(),
                controller: "idle".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            }),
        ],
    };

    let sequential = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
    let parallel = run_pack_with_workers(&pack, &base_dir, None, 2).unwrap();

    assert_eq!(sequential.total_runs, 4);
    assert_eq!(
        sequential.identity.resolved_run_digest,
        parallel.identity.resolved_run_digest
    );
    assert_eq!(
        sequential
            .records
            .iter()
            .map(|record| record.resolved.run_id.clone())
            .collect::<Vec<_>>(),
        parallel
            .records
            .iter()
            .map(|record| record.resolved.run_id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        sequential
            .summary
            .by_entry
            .iter()
            .find(|group| group.key == "terminal_sweep")
            .map(|group| group.total_runs),
        Some(3)
    );
    assert!(sequential.records.iter().any(|record| {
        record.resolved.source_kind == ResolvedRunSourceKind::FamilySweep
            && !record.resolved.resolved_parameters.is_empty()
    }));
    let family_record = sequential
        .records
        .iter()
        .find(|record| record.resolved.entry_id == "terminal_sweep")
        .expect("family record present");
    assert_eq!(family_record.resolved.selector.mission, "terminal_guidance");
    assert_eq!(
        family_record.resolved.selector.arrival_family,
        "seeded_terminal_arrival_v0"
    );
    assert_eq!(family_record.resolved.selector.condition_set, "clean");
    assert_eq!(family_record.resolved.selector.vehicle_variant, "nominal");
    assert_eq!(
        family_record.resolved.selector.expectation_tier.as_deref(),
        Some("core")
    );
    assert_eq!(family_record.resolved.lane_id, "baseline");
    let pointer = run_pointer(family_record);
    assert_eq!(pointer.selector.vehicle_variant, "nominal");
    assert_eq!(pointer.lane_id, "baseline");
}

#[test]
fn compare_reports_flags_regressions_on_shared_runs() {
    let base_dir = temp_fixture_root("compare_pack");
    write_scenario(
        &base_dir,
        "scenarios/landing_case.json",
        &easy_landing_scenario(),
    );
    let baseline_pack = ScenarioPackSpec {
        id: "compare_baseline".to_owned(),
        name: "Compare baseline".to_owned(),
        description: "compare baseline".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
            id: "landing_case".to_owned(),
            scenario: "scenarios/landing_case.json".to_owned(),
            controller: "baseline".to_owned(),
            controller_config: None,
            metadata: BTreeMap::new(),
        })],
    };
    let candidate_pack = ScenarioPackSpec {
        id: "compare_candidate".to_owned(),
        name: "Compare candidate".to_owned(),
        description: "compare candidate".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
            id: "landing_case".to_owned(),
            scenario: "scenarios/landing_case.json".to_owned(),
            controller: "idle".to_owned(),
            controller_config: None,
            metadata: BTreeMap::new(),
        })],
    };

    let baseline = run_pack(&baseline_pack, &base_dir, None).unwrap();
    let candidate = run_pack(&candidate_pack, &base_dir, None).unwrap();
    let comparison = compare_batch_reports(&candidate, &baseline);

    assert_eq!(comparison.basis.shared_runs, 1);
    assert_eq!(comparison.regressions.len(), 1);
    assert!(comparison.improvements.is_empty());
    assert_eq!(comparison.summary.failure_runs_delta, 1);
    assert_eq!(comparison.policy.status, BatchRegressionPolicyStatus::Fail);
    assert_eq!(
        comparison
            .policy
            .rules
            .iter()
            .find(|rule| rule.id == "new_failures")
            .expect("new failure policy rule should exist")
            .status,
        BatchRegressionPolicyStatus::Fail
    );
    assert_eq!(comparison.regressions[0].run_id, "landing_case");
    assert_eq!(
        comparison.regressions[0].baseline_mission_outcome,
        "success"
    );
    assert_eq!(
        comparison.regressions[0].candidate_mission_outcome,
        "failed_crash"
    );
}

#[test]
fn compare_reports_passes_policy_for_identical_reports() {
    let base_dir = temp_fixture_root("compare_identical");
    write_scenario(
        &base_dir,
        "scenarios/landing_case.json",
        &easy_landing_scenario(),
    );
    let pack = ScenarioPackSpec {
        id: "compare_identical".to_owned(),
        name: "Compare identical".to_owned(),
        description: "compare identical".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
            id: "landing_case".to_owned(),
            scenario: "scenarios/landing_case.json".to_owned(),
            controller: "baseline".to_owned(),
            controller_config: None,
            metadata: BTreeMap::new(),
        })],
    };

    let report = run_pack(&pack, &base_dir, None).unwrap();
    let comparison = compare_batch_reports(&report, &report);

    assert_eq!(comparison.policy.status, BatchRegressionPolicyStatus::Pass);
    assert!(comparison.regressions.is_empty());
    assert!(
        comparison
            .policy
            .summary
            .contains("passed all regression thresholds")
    );
}

#[test]
fn compare_policy_warns_on_aggregate_deltas_when_coverage_differs() {
    let base_dir = temp_fixture_root("compare_coverage");
    write_scenario(
        &base_dir,
        "scenarios/landing_case.json",
        &easy_landing_scenario(),
    );
    let baseline_pack = ScenarioPackSpec {
        id: "compare_coverage_baseline".to_owned(),
        name: "Compare coverage baseline".to_owned(),
        description: "compare coverage baseline".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
            id: "landing_case".to_owned(),
            scenario: "scenarios/landing_case.json".to_owned(),
            controller: "baseline".to_owned(),
            controller_config: None,
            metadata: BTreeMap::new(),
        })],
    };
    let candidate_pack = ScenarioPackSpec {
        id: "compare_coverage_candidate".to_owned(),
        name: "Compare coverage candidate".to_owned(),
        description: "compare coverage candidate".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![
            ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "landing_case".to_owned(),
                scenario: "scenarios/landing_case.json".to_owned(),
                controller: "baseline".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            }),
            ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "extra_case".to_owned(),
                scenario: "scenarios/landing_case.json".to_owned(),
                controller: "idle".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            }),
        ],
    };

    let baseline = run_pack(&baseline_pack, &base_dir, None).unwrap();
    let candidate = run_pack(&candidate_pack, &base_dir, None).unwrap();
    let comparison = compare_batch_reports(&candidate, &baseline);

    assert_eq!(comparison.basis.candidate_only_runs, 1);
    assert_eq!(comparison.summary.failure_runs_delta, 1);
    assert!(comparison.regressions.is_empty());
    assert_eq!(comparison.policy.status, BatchRegressionPolicyStatus::Warn);
    assert_eq!(
        comparison
            .policy
            .rules
            .iter()
            .find(|rule| rule.id == "scored_failure_delta")
            .expect("scored failure policy rule should exist")
            .status,
        BatchRegressionPolicyStatus::Warn
    );
}

#[test]
fn compare_reports_prefer_current_lane_policy_scope() {
    let base_dir = temp_fixture_root("compare_current_lane_scope");
    write_scenario(
        &base_dir,
        "scenarios/landing_case.json",
        &easy_landing_scenario(),
    );
    let baseline_pack = ScenarioPackSpec {
        id: "compare_current_lane_baseline".to_owned(),
        name: "Compare current lane baseline".to_owned(),
        description: "compare current lane baseline".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
            id: "landing_case".to_owned(),
            scenario: "scenarios/landing_case.json".to_owned(),
            controller: "baseline".to_owned(),
            controller_config: None,
            metadata: BTreeMap::new(),
        })],
    };
    let candidate_pack = ScenarioPackSpec {
        id: "compare_current_lane_candidate".to_owned(),
        name: "Compare current lane candidate".to_owned(),
        description: "compare current lane candidate".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
            id: "landing_case".to_owned(),
            scenario: "scenarios/landing_case.json".to_owned(),
            controller: "idle".to_owned(),
            controller_config: None,
            metadata: BTreeMap::new(),
        })],
    };
    let baseline_single = run_pack(&baseline_pack, &base_dir, None).unwrap();
    let candidate_single = run_pack(&candidate_pack, &base_dir, None).unwrap();

    let mut current_success = baseline_single.records[0].clone();
    current_success.resolved.run_id = "landing_case__current".to_owned();
    current_success.resolved.lane_id = "current".to_owned();

    let mut hidden_baseline_success = baseline_single.records[0].clone();
    hidden_baseline_success.resolved.run_id = "landing_case__baseline".to_owned();
    hidden_baseline_success.resolved.lane_id = "baseline".to_owned();

    let mut hidden_candidate_failure = candidate_single.records[0].clone();
    hidden_candidate_failure.resolved.run_id = "landing_case__baseline".to_owned();
    hidden_candidate_failure.resolved.lane_id = "baseline".to_owned();

    let baseline = report_with_records(
        baseline_single.clone(),
        vec![current_success.clone(), hidden_baseline_success],
    );
    let candidate = report_with_records(
        candidate_single,
        vec![current_success, hidden_candidate_failure],
    );
    let comparison = compare_batch_reports(&candidate, &baseline);

    assert_eq!(comparison.basis.shared_runs, 1);
    assert_eq!(comparison.summary.failure_runs_delta, 0);
    assert!(comparison.regressions.is_empty());
    assert_eq!(comparison.policy.status, BatchRegressionPolicyStatus::Pass);
}

#[test]
fn concrete_entry_metadata_overrides_selector_axes() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "concrete_metadata_override".to_owned(),
        name: "Concrete metadata override".to_owned(),
        description: "selector metadata override".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
            id: "checkpoint".to_owned(),
            scenario: "scenarios/timed_checkpoint_idle.json".to_owned(),
            controller: "idle".to_owned(),
            controller_config: None,
            metadata: BTreeMap::from([
                ("mission".to_owned(), "terminal_guidance".to_owned()),
                (
                    "arrival_family".to_owned(),
                    "override_arrival_family".to_owned(),
                ),
                ("condition_set".to_owned(), "stress".to_owned()),
                ("vehicle_variant".to_owned(), "heavy_cargo".to_owned()),
                ("expectation_tier".to_owned(), "frontier".to_owned()),
            ]),
        })],
    };

    let report = run_pack(&pack, &base_dir, None).unwrap();
    let record = report
        .records
        .iter()
        .find(|record| record.resolved.entry_id == "checkpoint")
        .expect("concrete record present");
    assert_eq!(record.resolved.selector.mission, "terminal_guidance");
    assert_eq!(
        record.resolved.selector.arrival_family,
        "override_arrival_family"
    );
    assert_eq!(record.resolved.selector.condition_set, "stress");
    assert_eq!(record.resolved.selector.vehicle_variant, "heavy_cargo");
    assert_eq!(
        record.resolved.selector.expectation_tier.as_deref(),
        Some("frontier")
    );
}

#[test]
fn terminal_matrix_entry_expands_documented_smoke_axes() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_smoke".to_owned(),
        name: "Terminal matrix smoke".to_owned(),
        description: "terminal matrix smoke".to_owned(),
        terminal_matrix_max_time_s: Some(90.0),
        entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
            id: "terminal_guidance_clean_nominal".to_owned(),
            terminal_matrix: "half_arc_terminal_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TerminalMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "staged".to_owned(),
                controller_config: None,
            }],
            seed_tier: TerminalSeedTier::Smoke,
            condition_set: "clean".to_owned(),
            vehicle_variant: "nominal".to_owned(),
            expectation_tier: "core".to_owned(),
            arc_points: Vec::new(),
            adjustments: Vec::new(),
            tags: vec!["terminal".to_owned(), "smoke".to_owned()],
            metadata: BTreeMap::from([("difficulty".to_owned(), "nominal".to_owned())]),
        })],
    };

    let report = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
    assert_eq!(report.total_runs, 7 * 3 * 3);
    assert_eq!(report.summary.by_entry[0].total_runs, 7 * 3 * 3);

    let record = report
        .records
        .iter()
        .find(|record| {
            record.resolved.selector.arc_point == "a80"
                && record.resolved.selector.velocity_band == "high"
                && record.resolved.resolved_seed == 6
        })
        .expect("matrix record present");

    assert_eq!(
        record.resolved.source_kind,
        ResolvedRunSourceKind::TerminalMatrix
    );
    assert_eq!(record.resolved.selector.mission, "terminal_guidance");
    assert_eq!(
        record.resolved.selector.arrival_family,
        "half_arc_terminal_v1"
    );
    assert_eq!(record.resolved.selector.condition_set, "clean");
    assert_eq!(record.resolved.selector.vehicle_variant, "nominal");
    assert_eq!(record.resolved.selector.arc_point, "a80");
    assert_eq!(record.resolved.selector.velocity_band, "high");
    assert_eq!(record.resolved.lane_id, "current");
    assert!(
        record
            .resolved
            .resolved_scenario_name
            .contains("a80 high seed 6 current")
    );
    assert_eq!(
        record.resolved.selector.expectation_tier.as_deref(),
        Some("core")
    );
    assert!(
        (record.manifest.summary.max_speed_mps > 0.0)
            && (record.manifest.summary.fuel_remaining_kg >= 0.0)
    );
    assert_eq!(record.manifest.scenario_seed, 6);
    assert_eq!(
        record
            .resolved
            .resolved_parameters
            .get("gravity_mps2")
            .copied(),
        Some(9.81)
    );
    assert_eq!(
        record
            .resolved
            .resolved_parameters
            .get("eval_max_time_s")
            .copied(),
        Some(90.0)
    );
    assert_eq!(
        record
            .resolved
            .resolved_parameters
            .get("reachability_max_time_s")
            .copied(),
        Some(60.0)
    );
}

#[test]
fn terminal_matrix_entry_respects_arc_point_selectors() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_selected_arcs".to_owned(),
        name: "Terminal matrix selected arcs".to_owned(),
        description: "terminal matrix selected arcs".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
            id: "terminal_guidance_selected_arcs".to_owned(),
            terminal_matrix: "half_arc_terminal_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TerminalMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "staged".to_owned(),
                controller_config: None,
            }],
            seed_tier: TerminalSeedTier::Smoke,
            condition_set: "clean".to_owned(),
            vehicle_variant: "nominal".to_owned(),
            expectation_tier: "core".to_owned(),
            arc_points: vec!["a70".to_owned(), "a80".to_owned()],
            adjustments: Vec::new(),
            tags: Vec::new(),
            metadata: BTreeMap::new(),
        })],
    };

    let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

    assert_eq!(resolved_runs.len(), 2 * 3 * 3);
    assert!(
        resolved_runs
            .iter()
            .all(|run| matches!(run.descriptor.selector.arc_point.as_str(), "a70" | "a80"))
    );
}

#[test]
fn transfer_matrix_entry_expands_smoke_route_axes() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_smoke".to_owned(),
        name: "Transfer matrix smoke".to_owned(),
        description: "transfer matrix smoke".to_owned(),
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
            route_angles: Vec::new(),
            radius_tiers: Vec::new(),
            waypoint_profile: None,
            waypoint_handoff_envelope: None,
            evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
            adjustments: Vec::new(),
            tags: vec!["transfer".to_owned(), "smoke".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

    assert_eq!(resolved_runs.len(), 5 * 3);
    let run = resolved_runs
        .iter()
        .find(|run| {
            run.descriptor.selector.route_angle == "r+60" && run.descriptor.resolved_seed == 2
        })
        .expect("r+60 seed 2 transfer run should be present");
    let route = run
        .scenario
        .mission
        .transfer_route
        .as_ref()
        .expect("transfer route should be present");
    let source_pad = run
        .scenario
        .world
        .landing_pad(&route.source_pad_id)
        .unwrap();
    let target_pad = run
        .scenario
        .world
        .landing_pad(&route.target_pad_id)
        .unwrap();

    assert_eq!(
        run.descriptor.source_kind,
        ResolvedRunSourceKind::TransferMatrix
    );
    assert_eq!(run.descriptor.selector.mission, "transfer_guidance");
    assert_eq!(
        run.descriptor.selector.route_family,
        "signed_route_arc_transfer_v1"
    );
    assert_eq!(
        run.descriptor.selector.arrival_family,
        "signed_route_arc_transfer_v1"
    );
    assert_eq!(run.descriptor.selector.condition_set, "clean");
    assert_eq!(run.descriptor.selector.arc_point, "r+60");
    assert_eq!(run.descriptor.selector.velocity_band, "nominal");
    assert_eq!(run.descriptor.selector.radius_tier, "nominal");
    assert!(source_pad.center_x_m < target_pad.center_x_m);
    assert!(source_pad.surface_y_m < target_pad.surface_y_m);
    assert!((route.route_radius_m - 824.0).abs() < 1e-9);
    assert!((route.route_angle_deg - 60.0).abs() < 1e-9);
    assert_eq!(
        run.descriptor
            .resolved_parameters
            .get("route_radius_nominal_m"),
        Some(&800.0)
    );
    assert_eq!(
        run.descriptor.resolved_parameters.get("route_radius_pct"),
        Some(&0.03)
    );
    assert_eq!(run.scenario.initial_state.velocity_mps, Vec2::new(0.0, 0.0));
}

#[test]
fn transfer_matrix_entry_expands_selected_radius_tiers() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_radius_tiers".to_owned(),
        name: "Transfer matrix radius tiers".to_owned(),
        description: "transfer matrix radius tiers".to_owned(),
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
            radius_tiers: vec!["short".to_owned(), "long".to_owned()],
            waypoint_profile: None,
            waypoint_handoff_envelope: None,
            evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
            adjustments: Vec::new(),
            tags: vec!["transfer".to_owned(), "radius".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

    assert_eq!(resolved_runs.len(), 2 * 3);
    let short = resolved_runs
        .iter()
        .find(|run| {
            run.descriptor.selector.radius_tier == "short" && run.descriptor.resolved_seed == 0
        })
        .expect("short transfer run should be present");
    let long = resolved_runs
        .iter()
        .find(|run| {
            run.descriptor.selector.radius_tier == "long" && run.descriptor.resolved_seed == 2
        })
        .expect("long transfer run should be present");

    let short_route = short.scenario.mission.transfer_route.as_ref().unwrap();
    let long_route = long.scenario.mission.transfer_route.as_ref().unwrap();
    assert_eq!(short.descriptor.selector.velocity_band, "short");
    assert_eq!(long.descriptor.selector.velocity_band, "long");
    assert!(
        short
            .descriptor
            .run_id
            .contains("_r00_short_seed_00_current")
    );
    assert!(long.descriptor.run_id.contains("_r00_long_seed_02_current"));
    assert!((short_route.route_radius_m - 400.0).abs() < 1e-9);
    assert!((long_route.route_radius_m - 1236.0).abs() < 1e-9);
    assert_eq!(
        short.descriptor.resolved_parameters.get("route_radius_m"),
        Some(&400.0)
    );
    assert_eq!(
        long.descriptor.resolved_parameters.get("route_radius_m"),
        Some(&1236.0)
    );
    assert_eq!(
        long.descriptor
            .resolved_parameters
            .get("route_radius_nominal_m"),
        Some(&1200.0)
    );
    assert_eq!(
        long.descriptor.resolved_parameters.get("route_radius_pct"),
        Some(&0.03)
    );
}

#[test]
fn transfer_matrix_seed_tier_perturbs_route_radius() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_seed_variation".to_owned(),
        name: "Transfer matrix seed variation".to_owned(),
        description: "transfer matrix seed variation".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
            id: "transfer_guidance_seed_variation".to_owned(),
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
            radius_tiers: vec!["nominal".to_owned()],
            waypoint_profile: None,
            waypoint_handoff_envelope: None,
            evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
            adjustments: Vec::new(),
            tags: vec!["transfer".to_owned(), "seed_variation".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();
    let radius_for_seed = |seed| {
        resolved_runs
            .iter()
            .find(|run| run.descriptor.resolved_seed == seed)
            .and_then(|run| run.scenario.mission.transfer_route.as_ref())
            .map(|route| route.route_radius_m)
            .expect("seeded transfer run should include a route")
    };

    assert_eq!(radius_for_seed(0), 800.0);
    assert_eq!(radius_for_seed(1), 776.0);
    assert_eq!(radius_for_seed(2), 824.0);
}

#[test]
fn transfer_matrix_waypoint_profile_injects_dogleg_waypoint() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_waypoint_profile".to_owned(),
        name: "Transfer matrix waypoint profile".to_owned(),
        description: "transfer matrix waypoint profile".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
            id: "transfer_guidance_waypoint_nominal".to_owned(),
            transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TransferMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "transfer_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TransferSeedTier::Smoke,
            vehicle_variant: "nominal".to_owned(),
            expectation_tier: TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC.to_owned(),
            route_angles: vec!["r+80".to_owned()],
            radius_tiers: vec!["nominal".to_owned()],
            waypoint_profile: Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1.to_owned()),
            waypoint_handoff_envelope: None,
            evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
            adjustments: Vec::new(),
            tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

    assert_eq!(resolved_runs.len(), 3);
    let run = resolved_runs
        .iter()
        .find(|run| run.descriptor.resolved_seed == 0)
        .expect("seed 0 waypoint transfer run should be present");
    let route = run
        .scenario
        .mission
        .transfer_route
        .as_ref()
        .expect("transfer route should be present");
    let waypoint = route
        .waypoints
        .first()
        .expect("dogleg waypoint should be present");
    let source_pad = run
        .scenario
        .world
        .landing_pad(&route.source_pad_id)
        .unwrap();

    assert_eq!(route.waypoints.len(), 1);
    assert_eq!(waypoint.id, "wp_dogleg_01");
    assert!(waypoint.position_m.x < source_pad.center_x_m);
    assert!(waypoint.position_m.y > 0.0);
    assert_eq!(
        run.scenario.metadata.get("route_mode").map(String::as_str),
        Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1)
    );
    assert_eq!(
        run.scenario
            .metadata
            .get("waypoint_profile")
            .map(String::as_str),
        Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1)
    );
    assert_eq!(
        run.descriptor.selector.waypoint_profile,
        TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1
    );
    assert_eq!(
        run.descriptor.selector.waypoint_handoff_envelope,
        TRANSFER_WAYPOINT_ENVELOPE_LEGACY_V1
    );
    assert_eq!(
        run.descriptor.resolved_parameters.get("waypoint_0_x_m"),
        Some(&waypoint.position_m.x)
    );
    assert_eq!(
        run.descriptor
            .resolved_parameters
            .get("waypoint_0_capture_radius_m"),
        Some(&waypoint.capture_radius_m)
    );
    assert_eq!(
        run.descriptor
            .resolved_parameters
            .get("waypoint_0_max_outbound_heading_error_rad"),
        Some(&waypoint.max_outbound_heading_error_rad)
    );
    assert_eq!(
        run.descriptor
            .resolved_parameters
            .get("waypoint_0_min_outbound_progress_mps"),
        Some(&waypoint.min_outbound_progress_mps)
    );
    assert_eq!(
        run.descriptor
            .resolved_parameters
            .get("waypoint_0_max_vertical_speed_mps"),
        waypoint.max_vertical_speed_mps.as_ref()
    );
    let turn_angle_deg = run
        .descriptor
        .resolved_parameters
        .get("waypoint_0_turn_angle_deg")
        .expect("dogleg profile should expose turn angle");
    assert!(*turn_angle_deg > 140.0);
}

#[test]
fn transfer_matrix_dogleg_profile_requires_diagnostic_tier() {
    let entry = TransferMatrixEntry {
        id: "transfer_guidance_waypoint_dogleg".to_owned(),
        transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
        base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
        lanes: vec![TransferMatrixLaneSpec {
            id: "current".to_owned(),
            controller: "transfer_pdg".to_owned(),
            controller_config: None,
        }],
        seed_tier: TransferSeedTier::Smoke,
        vehicle_variant: "nominal".to_owned(),
        expectation_tier: "frontier_probe".to_owned(),
        route_angles: vec!["r+80".to_owned()],
        radius_tiers: vec!["nominal".to_owned()],
        waypoint_profile: Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1.to_owned()),
        waypoint_handoff_envelope: None,
        evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
        adjustments: Vec::new(),
        tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
        metadata: BTreeMap::new(),
    };

    let message = validate_transfer_matrix_entry(&entry)
        .unwrap_err()
        .to_string();

    assert!(message.contains("requires expectation_tier 'diagnostic'"));
}

#[test]
fn transfer_matrix_late_bend_profile_requires_diagnostic_tier() {
    let entry = TransferMatrixEntry {
        id: "transfer_guidance_waypoint_late_bend".to_owned(),
        transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
        base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
        lanes: vec![TransferMatrixLaneSpec {
            id: "current".to_owned(),
            controller: "transfer_pdg".to_owned(),
            controller_config: None,
        }],
        seed_tier: TransferSeedTier::Smoke,
        vehicle_variant: "empty".to_owned(),
        expectation_tier: "contract_probe".to_owned(),
        route_angles: vec!["r+30".to_owned()],
        radius_tiers: vec!["nominal".to_owned()],
        waypoint_profile: Some(TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1.to_owned()),
        waypoint_handoff_envelope: Some(
            TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1.to_owned(),
        ),
        evaluation_goal: TransferMatrixEvaluationGoal::WaypointSequence,
        adjustments: Vec::new(),
        tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
        metadata: BTreeMap::new(),
    };

    let message = validate_transfer_matrix_entry(&entry)
        .unwrap_err()
        .to_string();

    assert!(message.contains("requires expectation_tier 'diagnostic'"));
}

#[test]
fn transfer_matrix_waypoint_profile_injects_smooth_bend_waypoint() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_waypoint_bend_profile".to_owned(),
        name: "Transfer matrix waypoint bend profile".to_owned(),
        description: "transfer matrix waypoint bend profile".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
            id: "transfer_guidance_waypoint_bend".to_owned(),
            transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TransferMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "transfer_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TransferSeedTier::Smoke,
            vehicle_variant: "nominal".to_owned(),
            expectation_tier: "frontier_probe".to_owned(),
            route_angles: vec!["r+80".to_owned()],
            radius_tiers: vec!["short".to_owned()],
            waypoint_profile: Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1.to_owned()),
            waypoint_handoff_envelope: Some(
                TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1.to_owned(),
            ),
            evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
            adjustments: Vec::new(),
            tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

    assert_eq!(resolved_runs.len(), 3);
    let run = resolved_runs
        .iter()
        .find(|run| run.descriptor.resolved_seed == 0)
        .expect("seed 0 waypoint transfer run should be present");
    let route = run
        .scenario
        .mission
        .transfer_route
        .as_ref()
        .expect("transfer route should be present");
    let waypoint = route
        .waypoints
        .first()
        .expect("bend waypoint should be present");

    assert_eq!(route.waypoints.len(), 1);
    assert_eq!(waypoint.id, "wp_bend_01");
    assert_eq!(
        run.descriptor.selector.waypoint_profile,
        TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1
    );
    assert_eq!(
        run.descriptor.selector.waypoint_handoff_envelope,
        TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1
    );
    assert_eq!(
        run.descriptor
            .resolved_parameters
            .get("waypoint_0_capture_radius_m"),
        Some(&40.0)
    );
    assert_eq!(
        run.descriptor
            .resolved_parameters
            .get("waypoint_0_max_cross_track_m"),
        Some(&70.0)
    );
    assert_eq!(waypoint.max_outbound_heading_error_rad, 0.35);
    assert_eq!(waypoint.max_outbound_cross_speed_mps, Some(20.0));
    assert_eq!(waypoint.max_speed_mps, 52.5);
    assert_eq!(waypoint.min_vertical_speed_mps, None);
    assert_eq!(waypoint.max_vertical_speed_mps, None);
    let params = &run.descriptor.resolved_parameters;
    let progress_frac = params
        .get("waypoint_0_profile_progress_frac")
        .expect("bend profile should expose progress fraction");
    let offset_ratio = params
        .get("waypoint_0_profile_lateral_offset_ratio")
        .expect("bend profile should expose offset ratio");
    let turn_angle_deg = params
        .get("waypoint_0_turn_angle_deg")
        .expect("bend profile should expose turn angle");
    let signed_turn_angle_deg = params
        .get("waypoint_0_signed_turn_angle_deg")
        .expect("bend profile should expose signed turn angle");
    assert!((*progress_frac - 0.55).abs() < 1.0e-9);
    assert!((*offset_ratio - 0.20).abs() < 1.0e-9);
    assert!(*turn_angle_deg > 40.0 && *turn_angle_deg < 48.0);
    assert!((*signed_turn_angle_deg + 43.9456).abs() < 1.0e-3);
    assert!((params["waypoint_0_route_signed_offset_ratio"] - *offset_ratio).abs() < 1.0e-9);
    assert!(
        params["waypoint_0_continuation_stop_ratio"]
            <= TRANSFER_WAYPOINT_CONTINUATION_MAX_STOP_RATIO
    );
}

#[test]
fn transfer_waypoint_bend_packs_use_continuation_envelopes() {
    let packs_dir = fixtures_root().join("packs");
    for (filename, expected_runs) in [
        ("transfer_waypoint_bend_rpos80_smoke.json", 27),
        ("transfer_waypoint_bend_contract_rpos80_smoke.json", 27),
        ("transfer_waypoint_bend_rpos80_full.json", 108),
        ("transfer_waypoint_bend_contract_rpos80_full.json", 108),
    ] {
        let pack = load_pack(&packs_dir.join(filename)).unwrap();
        let runs = resolve_pack_runs(&pack, &packs_dir).unwrap();
        assert_eq!(runs.len(), expected_runs);
        for run in runs {
            assert_eq!(
                run.descriptor.selector.waypoint_handoff_envelope,
                TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1
            );
            let waypoint = &run
                .scenario
                .mission
                .transfer_route
                .as_ref()
                .unwrap()
                .waypoints[0];
            let expected_speed_cap_mps = match run.descriptor.selector.radius_tier.as_str() {
                "short" => 52.5,
                "nominal" => 65.0,
                "long" => 75.0,
                radius_tier => panic!("unexpected radius tier {radius_tier}"),
            };
            assert_eq!(waypoint.max_speed_mps, expected_speed_cap_mps);
            assert!(
                run.descriptor.resolved_parameters["waypoint_0_continuation_stop_ratio"]
                    <= TRANSFER_WAYPOINT_CONTINUATION_MAX_STOP_RATIO
            );
        }
    }
}

#[test]
fn transfer_waypoint_turn_packs_expand_balanced_paired_matrix() {
    let packs_dir = fixtures_root().join("packs");
    let mission_path = packs_dir.join("transfer_waypoint_turn_smoke.json");
    let contract_path = packs_dir.join("transfer_waypoint_turn_contract_smoke.json");
    let mission_pack = load_pack(&mission_path).unwrap();
    let contract_pack = load_pack(&contract_path).unwrap();
    let mission_runs = resolve_pack_runs(&mission_pack, &packs_dir).unwrap();
    let contract_runs = resolve_pack_runs(&contract_pack, &packs_dir).unwrap();

    assert_eq!(mission_runs.len(), 81);
    assert_eq!(contract_runs.len(), 81);
    for runs in [&mission_runs, &contract_runs] {
        let run_ids = runs
            .iter()
            .map(|run| run.descriptor.run_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(run_ids.len(), 81);
        assert!(runs.iter().all(|run| {
            run.descriptor.selector.waypoint_handoff_envelope
                == TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1
        }));
        for profile in [
            TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1,
            TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1,
            TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1,
        ] {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.selector.waypoint_profile == profile)
                    .count(),
                27
            );
        }
        for vehicle in ["empty", "half", "full"] {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.selector.vehicle_variant == vehicle)
                    .count(),
                27
            );
        }
        for route_angle in ["r-30", "r00", "r+30"] {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.selector.route_angle == route_angle)
                    .count(),
                27
            );
        }
        for run in runs.iter() {
            let route = run
                .scenario
                .mission
                .transfer_route
                .as_ref()
                .expect("balanced waypoint profile should resolve a route");
            let waypoint = route
                .waypoints
                .first()
                .expect("balanced waypoint profile should resolve one waypoint");
            let terrain_y_m = run
                .scenario
                .world
                .terrain
                .sample_height(waypoint.position_m.x);
            let waypoint_clearance_m = waypoint.position_m.y - terrain_y_m;
            let required_envelope_clearance_m =
                waypoint.capture_radius_m.max(waypoint.max_cross_track_m)
                    + run.scenario.vehicle.geometry.touchdown_base_offset_m;
            assert!(
                waypoint_clearance_m > required_envelope_clearance_m,
                "profile {} route {} waypoint clearance {:.3} must exceed envelope and vehicle clearance {:.3}",
                run.descriptor.selector.waypoint_profile,
                run.descriptor.selector.route_angle,
                waypoint_clearance_m,
                required_envelope_clearance_m,
            );
            let params = &run.descriptor.resolved_parameters;
            assert!(
                (params["waypoint_0_terrain_clearance_m"] - waypoint_clearance_m).abs() < 1.0e-9
            );
        }
    }

    let selector_key = |run: &ResolvedBatchRun| {
        format!(
            "{}|{}|{}|{}|{}",
            run.descriptor.selector.waypoint_profile,
            run.descriptor.selector.vehicle_variant,
            run.descriptor.selector.route_angle,
            run.descriptor.selector.radius_tier,
            run.descriptor.resolved_seed,
        )
    };
    let mission_cells = mission_runs
        .iter()
        .map(selector_key)
        .collect::<BTreeSet<_>>();
    let contract_cells = contract_runs
        .iter()
        .map(selector_key)
        .collect::<BTreeSet<_>>();
    assert_eq!(mission_cells, contract_cells);
    assert!(mission_runs.iter().all(|run| matches!(
        run.scenario.mission.goal,
        EvaluationGoal::LandingOnPad { .. }
    )));
    assert!(contract_runs.iter().all(|run| matches!(
        run.scenario.mission.goal,
        EvaluationGoal::WaypointHandoff { .. }
    )));

    for (profile, expected_turn_angle_deg, expected_offset_ratio) in [
        (
            TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1,
            27.2394,
            0.12,
        ),
        (
            TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1,
            43.9456,
            0.20,
        ),
        (
            TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1,
            62.3005,
            0.30,
        ),
    ] {
        let run = mission_runs
            .iter()
            .find(|run| {
                run.descriptor.selector.waypoint_profile == profile
                    && run.descriptor.selector.vehicle_variant == "empty"
                    && run.descriptor.selector.route_angle == "r00"
                    && run.descriptor.resolved_seed == 0
            })
            .expect("balanced waypoint profile cell should resolve");
        let params = &run.descriptor.resolved_parameters;
        let turn_angle_deg = params["waypoint_0_turn_angle_deg"];
        let signed_turn_angle_deg = params["waypoint_0_signed_turn_angle_deg"];
        assert!(
            (turn_angle_deg - expected_turn_angle_deg).abs() < 1.0,
            "profile {profile} resolved turn angle {turn_angle_deg:.3}"
        );
        assert!(
            (signed_turn_angle_deg + expected_turn_angle_deg).abs() < 1.0,
            "profile {profile} resolved signed turn angle {signed_turn_angle_deg:.3}"
        );
        assert_eq!(params["waypoint_0_profile_progress_frac"], 0.55);
        assert!(
            (params["waypoint_0_profile_lateral_offset_ratio"] - expected_offset_ratio).abs()
                < 1.0e-9
        );
        assert_eq!(
            params["waypoint_0_max_cross_track_m"] / params["waypoint_0_capture_radius_m"],
            1.25
        );
    }
}

#[test]
fn transfer_waypoint_turn_route_angle_packs_expand_paired_matrix() {
    let packs_dir = fixtures_root().join("packs");
    let mission_pack =
        load_pack(&packs_dir.join("transfer_waypoint_turn_route_angle_smoke.json")).unwrap();
    let contract_pack =
        load_pack(&packs_dir.join("transfer_waypoint_turn_contract_route_angle_smoke.json"))
            .unwrap();
    let mission_runs = resolve_pack_runs(&mission_pack, &packs_dir).unwrap();
    let contract_runs = resolve_pack_runs(&contract_pack, &packs_dir).unwrap();

    assert_eq!(mission_runs.len(), 135);
    assert_eq!(contract_runs.len(), 135);
    for runs in [&mission_runs, &contract_runs] {
        let run_ids = runs
            .iter()
            .map(|run| run.descriptor.run_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(run_ids.len(), 135);
        assert!(runs.iter().all(|run| {
            run.descriptor.selector.radius_tier == "nominal"
                && run.descriptor.selector.waypoint_handoff_envelope
                    == TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1
        }));
        for profile in [
            TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1,
            TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1,
            TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1,
        ] {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.selector.waypoint_profile == profile)
                    .count(),
                45
            );
        }
        for vehicle in ["empty", "half", "full"] {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.selector.vehicle_variant == vehicle)
                    .count(),
                45
            );
        }
        for route_angle in ["r-60", "r-30", "r00", "r+30", "r+60"] {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.selector.route_angle == route_angle)
                    .count(),
                27
            );
        }
        for seed in 0..3 {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.resolved_seed == seed)
                    .count(),
                45
            );
        }
        for run in runs {
            let route = run
                .scenario
                .mission
                .transfer_route
                .as_ref()
                .expect("route-angle waypoint profile should resolve a route");
            assert_eq!(route.waypoints.len(), 1);
            let waypoint = &route.waypoints[0];
            let terrain_clearance_m = waypoint.position_m.y
                - run
                    .scenario
                    .world
                    .terrain
                    .sample_height(waypoint.position_m.x);
            assert!(
                terrain_clearance_m
                    > waypoint.max_cross_track_m
                        + run.scenario.vehicle.geometry.touchdown_base_offset_m
            );
        }
    }

    let selector_key = |run: &ResolvedBatchRun| {
        format!(
            "{}|{}|{}|{}|{}",
            run.descriptor.selector.waypoint_profile,
            run.descriptor.selector.vehicle_variant,
            run.descriptor.selector.route_angle,
            run.descriptor.selector.radius_tier,
            run.descriptor.resolved_seed,
        )
    };
    let mission_geometry = mission_runs
        .iter()
        .map(|run| {
            (
                selector_key(run),
                run.scenario
                    .mission
                    .transfer_route
                    .as_ref()
                    .unwrap()
                    .waypoints
                    .iter()
                    .map(|waypoint| waypoint.position_m)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let contract_geometry = contract_runs
        .iter()
        .map(|run| {
            (
                selector_key(run),
                run.scenario
                    .mission
                    .transfer_route
                    .as_ref()
                    .unwrap()
                    .waypoints
                    .iter()
                    .map(|waypoint| waypoint.position_m)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(mission_geometry, contract_geometry);
    assert!(mission_runs.iter().all(|run| matches!(
        run.scenario.mission.goal,
        EvaluationGoal::LandingOnPad { .. }
    )));
    assert!(contract_runs.iter().all(|run| matches!(
        run.scenario.mission.goal,
        EvaluationGoal::WaypointHandoff { .. }
    )));
}

#[test]
fn transfer_waypoint_sequence_packs_expand_balanced_paired_matrix() {
    let packs_dir = fixtures_root().join("packs");
    let mission_pack = load_pack(&packs_dir.join("transfer_waypoint_sequence_smoke.json")).unwrap();
    let contract_pack =
        load_pack(&packs_dir.join("transfer_waypoint_sequence_contract_smoke.json")).unwrap();
    let mission_runs = resolve_pack_runs(&mission_pack, &packs_dir).unwrap();
    let contract_runs = resolve_pack_runs(&contract_pack, &packs_dir).unwrap();

    assert_eq!(mission_runs.len(), 27);
    assert_eq!(contract_runs.len(), 27);
    for runs in [&mission_runs, &contract_runs] {
        let run_ids = runs
            .iter()
            .map(|run| run.descriptor.run_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(run_ids.len(), 27);
        assert!(runs.iter().all(|run| {
            run.descriptor.selector.waypoint_handoff_envelope
                == TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1
        }));
        assert!(runs.iter().all(|run| {
            run.descriptor.selector.waypoint_profile == TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1
        }));
        for vehicle in ["empty", "half", "full"] {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.selector.vehicle_variant == vehicle)
                    .count(),
                9
            );
        }
        for route_angle in ["r-30", "r00", "r+30"] {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.selector.route_angle == route_angle)
                    .count(),
                9
            );
        }
        for run in runs {
            let route = run
                .scenario
                .mission
                .transfer_route
                .as_ref()
                .expect("sequence profile should resolve a route");
            assert_eq!(route.waypoints.len(), 2);
            assert!(
                (route.waypoints[1].position_m - route.waypoints[0].position_m).length()
                    > route.waypoints[0].capture_radius_m + route.waypoints[1].capture_radius_m
            );
            for waypoint in &route.waypoints {
                assert!(waypoint.handoff_tangent_unit.is_some());
                let terrain_clearance_m = waypoint.position_m.y
                    - run
                        .scenario
                        .world
                        .terrain
                        .sample_height(waypoint.position_m.x);
                assert!(
                    terrain_clearance_m
                        > waypoint.max_cross_track_m
                            + run.scenario.vehicle.geometry.touchdown_base_offset_m
                );
            }
            let params = &run.descriptor.resolved_parameters;
            assert!(
                params["waypoint_0_profile_progress_frac"]
                    < params["waypoint_1_profile_progress_frac"]
            );
            assert_eq!(
                params["waypoint_0_max_cross_track_m"] / params["waypoint_0_capture_radius_m"],
                1.25
            );
            let speed_scale =
                (route.route_radius_m / SIGNED_ROUTE_ARC_TRANSFER_V1_NOMINAL_RADIUS_M).sqrt();
            let expected_speed_caps = [55.0 * speed_scale, 65.0 * speed_scale];
            assert_eq!(route.waypoints[0].max_speed_mps, expected_speed_caps[0]);
            assert_eq!(route.waypoints[1].max_speed_mps, expected_speed_caps[1]);
            assert!(params["waypoint_0_turn_authority_ratio"] <= 0.75);
            assert!(params["waypoint_1_turn_authority_ratio"] <= 0.75);
        }
    }

    let selector_key = |run: &ResolvedBatchRun| {
        format!(
            "{}|{}|{}|{}|{}",
            run.descriptor.selector.waypoint_profile,
            run.descriptor.selector.vehicle_variant,
            run.descriptor.selector.route_angle,
            run.descriptor.selector.radius_tier,
            run.descriptor.resolved_seed,
        )
    };
    let mission_cells = mission_runs
        .iter()
        .map(selector_key)
        .collect::<BTreeSet<_>>();
    let contract_cells = contract_runs
        .iter()
        .map(selector_key)
        .collect::<BTreeSet<_>>();
    assert_eq!(mission_cells, contract_cells);
    assert!(mission_runs.iter().all(|run| matches!(
        run.scenario.mission.goal,
        EvaluationGoal::LandingOnPad { .. }
    )));
    assert!(contract_runs.iter().all(|run| matches!(
        run.scenario.mission.goal,
        EvaluationGoal::WaypointSequence { .. }
    )));

    let mission_geometry = mission_runs
        .iter()
        .map(|run| {
            let route = run.scenario.mission.transfer_route.as_ref().unwrap();
            (
                selector_key(run),
                route
                    .waypoints
                    .iter()
                    .map(|waypoint| waypoint.position_m)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let contract_geometry = contract_runs
        .iter()
        .map(|run| {
            let route = run.scenario.mission.transfer_route.as_ref().unwrap();
            (
                selector_key(run),
                route
                    .waypoints
                    .iter()
                    .map(|waypoint| waypoint.position_m)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(mission_geometry, contract_geometry);

    for route_angle in ["r-30", "r00", "r+30"] {
        let run = mission_runs
            .iter()
            .find(|run| {
                run.descriptor.selector.vehicle_variant == "empty"
                    && run.descriptor.selector.route_angle == route_angle
                    && run.descriptor.resolved_seed == 0
            })
            .expect("double-bend sequence cell should resolve");
        let params = &run.descriptor.resolved_parameters;
        for index in 0..2 {
            assert!((params[&format!("waypoint_{index}_turn_angle_deg")] - 31.2184).abs() < 1.0e-3);
            assert!(
                (params[&format!("waypoint_{index}_signed_turn_angle_deg")] + 31.2184).abs()
                    < 1.0e-3
            );
            assert!(
                (params[&format!("waypoint_{index}_inbound_tangent_angle_deg")] - 15.6092).abs()
                    < 1.0e-3
            );
            assert!(
                (params[&format!("waypoint_{index}_tangent_outbound_angle_deg")] - 15.6092).abs()
                    < 1.0e-3
            );
        }
    }
    let uphill = mission_runs
        .iter()
        .find(|run| {
            run.descriptor.selector.vehicle_variant == "empty"
                && run.descriptor.selector.route_angle == "r+30"
                && run.descriptor.resolved_seed == 0
        })
        .unwrap();
    assert!(
        (uphill.descriptor.resolved_parameters["waypoint_0_handoff_tangent_heading_deg"] - 45.6092)
            .abs()
            < 1.0e-3
    );
    assert!(
        (uphill.descriptor.resolved_parameters["waypoint_1_handoff_tangent_heading_deg"] - 14.3908)
            .abs()
            < 1.0e-3
    );
}

#[test]
fn transfer_waypoint_sequence_route_angle_packs_expand_paired_matrix() {
    let packs_dir = fixtures_root().join("packs");
    let mission_pack =
        load_pack(&packs_dir.join("transfer_waypoint_sequence_route_angle_smoke.json")).unwrap();
    let contract_pack =
        load_pack(&packs_dir.join("transfer_waypoint_sequence_contract_route_angle_smoke.json"))
            .unwrap();
    let mission_runs = resolve_pack_runs(&mission_pack, &packs_dir).unwrap();
    let contract_runs = resolve_pack_runs(&contract_pack, &packs_dir).unwrap();

    assert_eq!(mission_runs.len(), 45);
    assert_eq!(contract_runs.len(), 45);
    for runs in [&mission_runs, &contract_runs] {
        let run_ids = runs
            .iter()
            .map(|run| run.descriptor.run_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(run_ids.len(), 45);
        assert!(runs.iter().all(|run| {
            run.descriptor.selector.radius_tier == "nominal"
                && run.descriptor.selector.waypoint_profile
                    == TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1
                && run.descriptor.selector.waypoint_handoff_envelope
                    == TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1
        }));
        for vehicle in ["empty", "half", "full"] {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.selector.vehicle_variant == vehicle)
                    .count(),
                15
            );
        }
        for route_angle in ["r-60", "r-30", "r00", "r+30", "r+60"] {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.selector.route_angle == route_angle)
                    .count(),
                9
            );
        }
        for seed in 0..3 {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.resolved_seed == seed)
                    .count(),
                15
            );
        }
        for run in runs {
            let route = run
                .scenario
                .mission
                .transfer_route
                .as_ref()
                .expect("route-angle waypoint sequence should resolve a route");
            assert_eq!(route.waypoints.len(), 2);
            assert!(
                (route.waypoints[1].position_m - route.waypoints[0].position_m).length()
                    > route.waypoints[0].capture_radius_m + route.waypoints[1].capture_radius_m
            );
            for waypoint in &route.waypoints {
                assert!(waypoint.handoff_tangent_unit.is_some());
                let terrain_clearance_m = waypoint.position_m.y
                    - run
                        .scenario
                        .world
                        .terrain
                        .sample_height(waypoint.position_m.x);
                assert!(
                    terrain_clearance_m
                        > waypoint.max_cross_track_m
                            + run.scenario.vehicle.geometry.touchdown_base_offset_m
                );
            }
        }
    }

    let selector_key = |run: &ResolvedBatchRun| {
        format!(
            "{}|{}|{}|{}|{}",
            run.descriptor.selector.waypoint_profile,
            run.descriptor.selector.vehicle_variant,
            run.descriptor.selector.route_angle,
            run.descriptor.selector.radius_tier,
            run.descriptor.resolved_seed,
        )
    };
    let mission_geometry = mission_runs
        .iter()
        .map(|run| {
            (
                selector_key(run),
                run.scenario
                    .mission
                    .transfer_route
                    .as_ref()
                    .unwrap()
                    .waypoints
                    .iter()
                    .map(|waypoint| waypoint.position_m)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let contract_geometry = contract_runs
        .iter()
        .map(|run| {
            (
                selector_key(run),
                run.scenario
                    .mission
                    .transfer_route
                    .as_ref()
                    .unwrap()
                    .waypoints
                    .iter()
                    .map(|waypoint| waypoint.position_m)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(mission_geometry, contract_geometry);
    assert!(mission_runs.iter().all(|run| matches!(
        run.scenario.mission.goal,
        EvaluationGoal::LandingOnPad { .. }
    )));
    assert!(contract_runs.iter().all(|run| matches!(
        run.scenario.mission.goal,
        EvaluationGoal::WaypointSequence { .. }
    )));
}

#[test]
fn transfer_waypoint_route_angle_full_packs_preserve_paired_cells() {
    let packs_dir = fixtures_root().join("packs");
    for (landing_filename, contract_filename, expected_runs, expected_waypoints) in [
        (
            "transfer_waypoint_turn_route_angle_full.json",
            "transfer_waypoint_turn_contract_route_angle_full.json",
            540,
            1,
        ),
        (
            "transfer_waypoint_sequence_route_angle_full.json",
            "transfer_waypoint_sequence_contract_route_angle_full.json",
            180,
            2,
        ),
    ] {
        let landing_pack = load_pack(&packs_dir.join(landing_filename)).unwrap();
        let contract_pack = load_pack(&packs_dir.join(contract_filename)).unwrap();
        let landing_runs = resolve_pack_runs(&landing_pack, &packs_dir).unwrap();
        let contract_runs = resolve_pack_runs(&contract_pack, &packs_dir).unwrap();

        assert_eq!(landing_runs.len(), expected_runs);
        assert_eq!(contract_runs.len(), expected_runs);
        for runs in [&landing_runs, &contract_runs] {
            assert!(runs.iter().all(|run| {
                run.descriptor.selector.radius_tier == "nominal"
                    && run
                        .scenario
                        .mission
                        .transfer_route
                        .as_ref()
                        .is_some_and(|route| route.waypoints.len() == expected_waypoints)
            }));
            for seed in 0..12 {
                assert_eq!(
                    runs.iter()
                        .filter(|run| run.descriptor.resolved_seed == seed)
                        .count(),
                    expected_runs / 12
                );
            }
        }

        let selector_key = |run: &ResolvedBatchRun| {
            format!(
                "{}|{}|{}|{}|{}",
                run.descriptor.selector.waypoint_profile,
                run.descriptor.selector.vehicle_variant,
                run.descriptor.selector.route_angle,
                run.descriptor.selector.radius_tier,
                run.descriptor.resolved_seed,
            )
        };
        let landing_cells = landing_runs
            .iter()
            .map(selector_key)
            .collect::<BTreeSet<_>>();
        let contract_cells = contract_runs
            .iter()
            .map(selector_key)
            .collect::<BTreeSet<_>>();
        assert_eq!(landing_cells, contract_cells);
        assert!(landing_runs.iter().all(|run| matches!(
            run.scenario.mission.goal,
            EvaluationGoal::LandingOnPad { .. }
        )));
        if expected_waypoints == 1 {
            assert!(contract_runs.iter().all(|run| matches!(
                run.scenario.mission.goal,
                EvaluationGoal::WaypointHandoff { .. }
            )));
        } else {
            assert!(contract_runs.iter().all(|run| matches!(
                run.scenario.mission.goal,
                EvaluationGoal::WaypointSequence { .. }
            )));
        }
    }
}

#[test]
fn transfer_waypoint_radius_smoke_packs_preserve_paired_cells() {
    let packs_dir = fixtures_root().join("packs");
    for (landing_filename, contract_filename, expected_runs, expected_waypoints) in [
        (
            "transfer_waypoint_turn_route_angle_radius_smoke.json",
            "transfer_waypoint_turn_contract_route_angle_radius_smoke.json",
            405,
            1,
        ),
        (
            "transfer_waypoint_sequence_route_angle_radius_smoke.json",
            "transfer_waypoint_sequence_contract_route_angle_radius_smoke.json",
            135,
            2,
        ),
    ] {
        let landing_pack = load_pack(&packs_dir.join(landing_filename)).unwrap();
        let contract_pack = load_pack(&packs_dir.join(contract_filename)).unwrap();
        let landing_runs = resolve_pack_runs(&landing_pack, &packs_dir).unwrap();
        let contract_runs = resolve_pack_runs(&contract_pack, &packs_dir).unwrap();

        assert_eq!(landing_runs.len(), expected_runs);
        assert_eq!(contract_runs.len(), expected_runs);
        for runs in [&landing_runs, &contract_runs] {
            for radius_tier in ["short", "nominal", "long"] {
                assert_eq!(
                    runs.iter()
                        .filter(|run| run.descriptor.selector.radius_tier == radius_tier)
                        .count(),
                    expected_runs / 3
                );
            }
            assert!(runs.iter().all(|run| {
                run.scenario
                    .mission
                    .transfer_route
                    .as_ref()
                    .is_some_and(|route| route.waypoints.len() == expected_waypoints)
            }));
        }

        let selector_key = |run: &ResolvedBatchRun| {
            format!(
                "{}|{}|{}|{}|{}",
                run.descriptor.selector.waypoint_profile,
                run.descriptor.selector.vehicle_variant,
                run.descriptor.selector.route_angle,
                run.descriptor.selector.radius_tier,
                run.descriptor.resolved_seed,
            )
        };
        let landing_cells = landing_runs
            .iter()
            .map(selector_key)
            .collect::<BTreeSet<_>>();
        let contract_cells = contract_runs
            .iter()
            .map(selector_key)
            .collect::<BTreeSet<_>>();
        assert_eq!(landing_cells, contract_cells);
    }
}

#[test]
fn maintained_waypoint_radius_profiles_resolve_at_full_seed_depth() {
    let packs_dir = fixtures_root().join("packs");
    for (filename, expected_runs, expected_waypoints) in [
        (
            "transfer_waypoint_turn_route_angle_radius_smoke.json",
            3_564,
            1,
        ),
        (
            "transfer_waypoint_sequence_route_angle_radius_smoke.json",
            1_188,
            2,
        ),
    ] {
        let mut pack = load_pack(&packs_dir.join(filename)).unwrap();
        for entry in &mut pack.entries {
            if let ScenarioPackEntry::TransferMatrix(entry) = entry {
                entry.seed_tier = TransferSeedTier::Full;
            }
        }
        let runs = resolve_pack_runs(&pack, &packs_dir).unwrap();

        assert_eq!(runs.len(), expected_runs);
        for run in &runs {
            let route = run
                .scenario
                .mission
                .transfer_route
                .as_ref()
                .expect("maintained waypoint profile should resolve a route");
            assert_eq!(route.waypoints.len(), expected_waypoints);
            for (index, waypoint) in route.waypoints.iter().enumerate() {
                let terrain_clearance_m = waypoint.position_m.y
                    - run
                        .scenario
                        .world
                        .terrain
                        .sample_height(waypoint.position_m.x);
                assert!(
                    terrain_clearance_m
                        > waypoint.max_cross_track_m
                            + run.scenario.vehicle.geometry.touchdown_base_offset_m
                );
                let authority_key = if expected_waypoints == 1 {
                    format!("waypoint_{index}_continuation_stop_ratio")
                } else {
                    format!("waypoint_{index}_turn_authority_ratio")
                };
                assert!(run.descriptor.resolved_parameters[&authority_key] <= 0.75);
            }
        }

        if expected_waypoints == 2 {
            for (radius_tier, speed_scale) in [
                ("short", 0.5_f64.sqrt()),
                ("nominal", 1.0),
                ("long", 1.5_f64.sqrt()),
            ] {
                let run = runs
                    .iter()
                    .find(|run| {
                        run.descriptor.selector.radius_tier == radius_tier
                            && run.descriptor.selector.vehicle_variant == "empty"
                            && run.descriptor.selector.route_angle == "r00"
                            && run.descriptor.resolved_seed == 0
                    })
                    .expect("radius profile seed-zero cell should resolve");
                let route = run.scenario.mission.transfer_route.as_ref().unwrap();
                assert!((route.waypoints[0].max_speed_mps - 55.0 * speed_scale).abs() < 1.0e-9);
                assert!((route.waypoints[1].max_speed_mps - 65.0 * speed_scale).abs() < 1.0e-9);
            }
        }
    }
}

#[test]
fn transfer_waypoint_late_bend_diagnostic_preserves_full_matrix() {
    let packs_dir = fixtures_root().join("packs");
    let pack =
        load_pack(&packs_dir.join("transfer_waypoint_sequence_late_bend_diagnostic.json")).unwrap();
    let runs = resolve_pack_runs(&pack, &packs_dir).unwrap();

    assert_eq!(runs.len(), 27);
    assert!(runs.iter().all(|run| {
        run.descriptor.selector.expectation_tier.as_deref() == Some("diagnostic")
            && run.descriptor.selector.radius_tier == "nominal"
            && run.descriptor.selector.waypoint_profile == TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1
            && run.descriptor.selector.waypoint_handoff_envelope
                == TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1
            && matches!(
                run.scenario.mission.goal,
                EvaluationGoal::LandingOnPad { .. }
            )
            && run
                .scenario
                .mission
                .transfer_route
                .as_ref()
                .is_some_and(|route| route.waypoints.len() == 2)
    }));
    for vehicle in ["empty", "half", "full"] {
        assert_eq!(
            runs.iter()
                .filter(|run| run.descriptor.selector.vehicle_variant == vehicle)
                .count(),
            9
        );
    }
}

#[test]
fn transfer_matrix_waypoint_handoff_goal_resolves_probe_goal() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_waypoint_handoff_goal".to_owned(),
        name: "Transfer matrix waypoint handoff goal".to_owned(),
        description: "transfer matrix waypoint handoff goal".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
            id: "transfer_guidance_waypoint_contract".to_owned(),
            transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TransferMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "transfer_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TransferSeedTier::Smoke,
            vehicle_variant: "nominal".to_owned(),
            expectation_tier: TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC.to_owned(),
            route_angles: vec!["r+80".to_owned()],
            radius_tiers: vec!["nominal".to_owned()],
            waypoint_profile: Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1.to_owned()),
            waypoint_handoff_envelope: None,
            evaluation_goal: TransferMatrixEvaluationGoal::WaypointHandoff,
            adjustments: Vec::new(),
            tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

    assert_eq!(resolved_runs.len(), 3);
    let run = resolved_runs
        .iter()
        .find(|run| run.descriptor.resolved_seed == 0)
        .expect("seed 0 waypoint handoff run should be present");
    assert!(matches!(
        run.scenario.mission.goal,
        EvaluationGoal::WaypointHandoff {
            waypoint_index: 0,
            ..
        }
    ));
    assert_eq!(
        run.scenario
            .metadata
            .get("evaluation_goal")
            .map(String::as_str),
        Some("waypoint_handoff")
    );
    assert_eq!(
        run.descriptor
            .resolved_parameters
            .get("waypoint_handoff_index"),
        Some(&0.0)
    );
}

#[test]
fn transfer_matrix_waypoint_handoff_goal_requires_waypoint_profile() {
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_invalid_waypoint_goal".to_owned(),
        name: "Transfer matrix invalid waypoint goal".to_owned(),
        description: "transfer matrix invalid waypoint goal".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
            id: "transfer_guidance_waypoint_contract".to_owned(),
            transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TransferMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "transfer_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TransferSeedTier::Smoke,
            vehicle_variant: "nominal".to_owned(),
            expectation_tier: "frontier_probe".to_owned(),
            route_angles: vec!["r+80".to_owned()],
            radius_tiers: vec!["nominal".to_owned()],
            waypoint_profile: None,
            waypoint_handoff_envelope: None,
            evaluation_goal: TransferMatrixEvaluationGoal::WaypointHandoff,
            adjustments: Vec::new(),
            tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let message = validate_pack(&pack).unwrap_err().to_string();

    assert!(message.contains("requires waypoint_profile"));
}

#[test]
fn transfer_matrix_waypoint_envelope_requires_waypoint_profile() {
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_invalid_waypoint_envelope".to_owned(),
        name: "Transfer matrix invalid waypoint envelope".to_owned(),
        description: "Transfer matrix invalid waypoint envelope".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
            id: "transfer_guidance_waypoint_envelope".to_owned(),
            transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TransferMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "transfer_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TransferSeedTier::Smoke,
            vehicle_variant: "nominal".to_owned(),
            expectation_tier: "frontier_probe".to_owned(),
            route_angles: vec!["r+80".to_owned()],
            radius_tiers: vec!["nominal".to_owned()],
            waypoint_profile: None,
            waypoint_handoff_envelope: Some(TRANSFER_WAYPOINT_ENVELOPE_PASS_THROUGH_V1.to_owned()),
            evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
            adjustments: Vec::new(),
            tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let message = validate_pack(&pack).unwrap_err().to_string();

    assert!(message.contains("waypoint_handoff_envelope requires waypoint_profile"));
}

#[test]
fn waypoint_contract_review_passes_when_capture_and_outbound_envelope_pass() {
    let scenario = waypoint_contract_scenario();
    let metrics = WaypointReviewMetrics {
        capture_status: Some("captured".to_owned()),
        active_index: Some(0),
        outbound_heading_error_rad: Some(0.4),
        outbound_progress_mps: Some(18.0),
        speed_mps: Some(42.0),
        vertical_speed_mps: Some(12.0),
        ..WaypointReviewMetrics::default()
    };

    let contract = waypoint_contract_review_metrics(&scenario, &metrics);

    assert_eq!(contract.status.as_deref(), Some("pass"));
    assert!(contract.reasons.is_empty());
}

#[test]
fn waypoint_contract_review_flags_captured_bad_outbound_state() {
    let scenario = waypoint_contract_scenario();
    let metrics = WaypointReviewMetrics {
        capture_status: Some("captured".to_owned()),
        active_index: Some(0),
        outbound_heading_error_rad: Some(2.4),
        outbound_progress_mps: Some(-18.0),
        speed_mps: Some(42.0),
        vertical_speed_mps: Some(92.0),
        ..WaypointReviewMetrics::default()
    };

    let contract = waypoint_contract_review_metrics(&scenario, &metrics);

    assert_eq!(contract.status.as_deref(), Some("outbound_out_of_envelope"));
    assert_eq!(
        contract.reasons,
        vec![
            "heading".to_owned(),
            "outbound_progress".to_owned(),
            "vertical_speed".to_owned()
        ]
    );
}

#[test]
fn waypoint_contract_review_scores_outbound_cross_speed_without_vertical_bounds() {
    let mut scenario = waypoint_contract_scenario();
    let waypoint = scenario
        .mission
        .transfer_route
        .as_mut()
        .and_then(|route| route.waypoints.first_mut())
        .expect("waypoint fixture");
    waypoint.max_outbound_cross_speed_mps = Some(20.0);
    waypoint.min_vertical_speed_mps = None;
    waypoint.max_vertical_speed_mps = None;
    let metrics = WaypointReviewMetrics {
        capture_status: Some("captured".to_owned()),
        active_index: Some(0),
        outbound_heading_error_rad: Some(0.4),
        outbound_progress_mps: Some(18.0),
        outbound_cross_speed_mps: Some(24.0),
        speed_mps: Some(42.0),
        vertical_speed_mps: Some(92.0),
        ..WaypointReviewMetrics::default()
    };

    let contract = waypoint_contract_review_metrics(&scenario, &metrics);

    assert_eq!(contract.status.as_deref(), Some("outbound_out_of_envelope"));
    assert_eq!(contract.reasons, vec!["outbound_cross_speed".to_owned()]);
}

#[test]
fn waypoint_contract_review_flags_spatial_miss_separately() {
    let scenario = waypoint_contract_scenario();
    let metrics = WaypointReviewMetrics {
        capture_status: Some("missed".to_owned()),
        active_index: Some(0),
        distance_m: Some(90.0),
        cross_track_m: Some(72.0),
        plane_progress_m: Some(-65.0),
        outbound_heading_error_rad: Some(0.4),
        outbound_progress_mps: Some(18.0),
        speed_mps: Some(42.0),
        vertical_speed_mps: Some(12.0),
        ..WaypointReviewMetrics::default()
    };

    let contract = waypoint_contract_review_metrics(&scenario, &metrics);

    assert_eq!(contract.status.as_deref(), Some("spatial_miss"));
    assert_eq!(
        contract.reasons,
        vec![
            "outside_capture_radius".to_owned(),
            "cross_track".to_owned(),
            "before_waypoint_plane".to_owned()
        ]
    );
}

fn transfer_sample(
    physics_step: u64,
    sim_time_s: f64,
    position_m: Vec2,
    velocity_mps: Vec2,
    fuel_kg: f64,
) -> SampleRecord {
    SampleRecord {
        sim_time_s,
        physics_step,
        observation: pd_core::Observation {
            sim_time_s,
            physics_step,
            position_m,
            velocity_mps,
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
            mass_kg: 900.0 + fuel_kg,
            fuel_kg,
            gravity_mps2: 9.8,
            target_dx_m: -position_m.x,
            height_above_target_m: position_m.y,
            target_surface_y_m: 0.0,
            target_pad_half_width_m: 18.0,
            touchdown_clearance_m: position_m.y,
            min_hull_clearance_m: position_m.y,
        },
        held_command: pd_core::Command::idle(),
    }
}

fn transfer_update(
    physics_step: u64,
    sim_time_s: f64,
    transfer_phase: &str,
    route_dx_m: f64,
    projected_dx_m: f64,
    throttle_frac: f64,
) -> pd_control::ControllerUpdateRecord {
    pd_control::ControllerUpdateRecord {
        sim_time_s,
        physics_step,
        controller_update_index: physics_step,
        compute_time_us: None,
        frame: pd_control::ControllerFrame {
            command: pd_core::Command {
                throttle_frac,
                target_attitude_rad: 0.0,
            },
            status: transfer_phase.to_owned(),
            phase: Some(transfer_phase.to_owned()),
            metrics: BTreeMap::from([
                (
                    metric::TRANSFER_PHASE.to_owned(),
                    TelemetryValue::from(transfer_phase),
                ),
                (
                    metric::TRANSFER_ROUTE_DX_M.to_owned(),
                    TelemetryValue::from(route_dx_m),
                ),
                (
                    metric::TRANSFER_TARGET_Y_SOLUTION.to_owned(),
                    TelemetryValue::from(true),
                ),
                (
                    metric::TRANSFER_PROJECTED_DX_M.to_owned(),
                    TelemetryValue::from(projected_dx_m),
                ),
            ]),
            markers: Vec::new(),
        },
    }
}

#[test]
fn waypoint_review_metrics_capture_first_waypoint_handoff() {
    let mut tracking = transfer_update(0, 0.0, "boost", 200.0, 80.0, 1.0);
    tracking.frame.metrics.extend(BTreeMap::from([
        (
            metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
            TelemetryValue::from(true),
        ),
        (
            metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
            TelemetryValue::from(0_i64),
        ),
        (
            metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
            TelemetryValue::from("tracking"),
        ),
    ]));
    let mut captured = transfer_update(60, 0.5, "boost", 100.0, 20.0, 0.8);
    captured.frame.metrics.extend(BTreeMap::from([
        (
            metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
            TelemetryValue::from(true),
        ),
        (
            metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
            TelemetryValue::from(0_i64),
        ),
        (
            metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
            TelemetryValue::from("captured"),
        ),
        (
            metric::WAYPOINT_CAPTURE_TIME_S.to_owned(),
            TelemetryValue::from(0.5),
        ),
        (
            metric::WAYPOINT_WINDOW_ENTRY_TIME_S.to_owned(),
            TelemetryValue::from(0.5),
        ),
        (
            metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_PASS.to_owned(),
            TelemetryValue::from(true),
        ),
        (
            metric::WAYPOINT_HANDOFF_RESOLUTION_REASON.to_owned(),
            TelemetryValue::from("contract_pass"),
        ),
        (
            metric::WAYPOINT_HANDOFF_WINDOW_DURATION_S.to_owned(),
            TelemetryValue::from(0.0),
        ),
        (
            metric::WAYPOINT_CLOSEST_DISTANCE_M.to_owned(),
            TelemetryValue::from(12.0),
        ),
        (
            metric::WAYPOINT_CROSS_TRACK_M.to_owned(),
            TelemetryValue::from(8.0),
        ),
        (
            metric::WAYPOINT_OUTBOUND_PROGRESS_MPS.to_owned(),
            TelemetryValue::from(24.0),
        ),
        (
            metric::WAYPOINT_REQUIRED_TURN_DISTANCE_M.to_owned(),
            TelemetryValue::from(72.0),
        ),
        (
            metric::WAYPOINT_TURN_MARGIN_M.to_owned(),
            TelemetryValue::from(38.0),
        ),
        (
            metric::WAYPOINT_FINAL_TERMINAL_REQUIRED_ACCEL_RATIO.to_owned(),
            TelemetryValue::from(0.72),
        ),
        (
            metric::WAYPOINT_FINAL_TERMINAL_RECOVERABLE.to_owned(),
            TelemetryValue::from(true),
        ),
    ]));

    let metrics = waypoint_review_metrics(&[tracking, captured]);

    assert_eq!(metrics.capture_status.as_deref(), Some("captured"));
    assert_eq!(metrics.active_index, Some(0));
    assert_eq!(metrics.capture_time_s, Some(0.5));
    assert_eq!(
        metrics.window_entry.as_ref().and_then(|entry| entry.time_s),
        Some(0.5)
    );
    assert_eq!(
        metrics
            .window_entry
            .as_ref()
            .and_then(|entry| entry.contract_pass),
        Some(true)
    );
    assert_eq!(metrics.resolution_reason.as_deref(), Some("contract_pass"));
    assert_eq!(metrics.window_duration_s, Some(0.0));
    assert_eq!(metrics.closest_distance_m, Some(12.0));
    assert_eq!(metrics.cross_track_m, Some(8.0));
    assert_eq!(metrics.outbound_progress_mps, Some(24.0));
    assert_eq!(metrics.required_turn_distance_m, Some(72.0));
    assert_eq!(metrics.turn_margin_m, Some(38.0));
    assert_eq!(metrics.final_terminal_required_accel_ratio, Some(0.72));
    assert_eq!(metrics.final_terminal_recoverable, Some(true));
}

#[test]
fn waypoint_review_history_preserves_ordered_handoff_markers() {
    let handoff_update = |physics_step: u64, sim_time_s: f64, index: i64, replans: i64| {
        let mut update = transfer_update(physics_step, sim_time_s, "boost", 100.0, 20.0, 0.8);
        update.frame.metrics.extend(BTreeMap::from([
            (
                metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
                TelemetryValue::from(true),
            ),
            (
                metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
                TelemetryValue::from(index),
            ),
            (
                metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
                TelemetryValue::from("captured"),
            ),
            (
                metric::WAYPOINT_CAPTURE_TIME_S.to_owned(),
                TelemetryValue::from(sim_time_s),
            ),
        ]));
        update.frame.markers.push(pd_control::ControllerMarker {
            id: marker::WAYPOINT_HANDOFF.to_owned(),
            label: format!("waypoint {index}"),
            x_m: None,
            y_m: None,
            metadata: BTreeMap::from([
                (
                    metric::WAYPOINT_GUIDANCE_REPLAN_COUNT.to_owned(),
                    TelemetryValue::from(replans),
                ),
                (
                    metric::WAYPOINT_WINDOW_ENTRY_TIME_S.to_owned(),
                    TelemetryValue::from(sim_time_s - 0.2),
                ),
                (
                    metric::WAYPOINT_WINDOW_ENTRY_POSITION_X_M.to_owned(),
                    TelemetryValue::from(10.0 + index as f64),
                ),
                (
                    metric::WAYPOINT_WINDOW_ENTRY_POSITION_Y_M.to_owned(),
                    TelemetryValue::from(20.0),
                ),
                (
                    metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_PASS.to_owned(),
                    TelemetryValue::from(false),
                ),
                (
                    metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_REASONS.to_owned(),
                    TelemetryValue::from("heading"),
                ),
                (
                    metric::WAYPOINT_HANDOFF_RESOLUTION_REASON.to_owned(),
                    TelemetryValue::from("contract_pass"),
                ),
                (
                    metric::WAYPOINT_HANDOFF_WINDOW_DURATION_S.to_owned(),
                    TelemetryValue::from(0.2),
                ),
                (
                    metric::WAYPOINT_TARGET_VELOCITY_ERROR_MPS.to_owned(),
                    TelemetryValue::from(12.0 + index as f64),
                ),
                (
                    metric::WAYPOINT_TRANSITION_NEXT_INDEX.to_owned(),
                    TelemetryValue::from(index + 1),
                ),
                (
                    metric::WAYPOINT_TRANSITION_POSITION_ERROR_M.to_owned(),
                    TelemetryValue::from(4.0 + index as f64),
                ),
                (
                    metric::WAYPOINT_TRANSITION_CONTINUATION_CONTRACT_PASS.to_owned(),
                    TelemetryValue::from(true),
                ),
                (
                    metric::WAYPOINT_JOINT_NEXT_INDEX.to_owned(),
                    TelemetryValue::from(index + 1),
                ),
                (
                    metric::WAYPOINT_JOINT_EVALUATED_CANDIDATE_COUNT.to_owned(),
                    TelemetryValue::from(0_i64),
                ),
                (
                    metric::WAYPOINT_JOINT_PASSING_CANDIDATE_COUNT.to_owned(),
                    TelemetryValue::from(0_i64),
                ),
            ]),
        });
        update
    };
    let second = handoff_update(120, 1.0, 1, 3);
    let first = handoff_update(60, 0.5, 0, 2);

    let history = waypoint_review_history(&[second, first]);

    assert_eq!(history.len(), 2);
    assert_eq!(history[0].active_index, Some(0));
    assert_eq!(history[0].guidance_replan_count, Some(2));
    assert_eq!(history[0].target_velocity_error_mps, Some(12.0));
    assert_eq!(history[0].transition_next_waypoint_index, Some(1));
    assert_eq!(history[0].transition_position_error_m, Some(4.0));
    assert_eq!(history[0].transition_continuation_contract_pass, Some(true));
    assert_eq!(history[0].joint_next_waypoint_index, Some(1));
    assert_eq!(history[0].joint_evaluated_candidate_count, Some(0));
    assert_eq!(
        history[0]
            .window_entry
            .as_ref()
            .and_then(|entry| entry.contract_pass),
        Some(false)
    );
    assert_eq!(
        history[0]
            .window_entry
            .as_ref()
            .map(|entry| entry.contract_reasons.as_slice()),
        Some(["heading".to_owned()].as_slice())
    );
    assert_eq!(
        history[0].resolution_reason.as_deref(),
        Some("contract_pass")
    );
    assert_eq!(history[0].window_duration_s, Some(0.2));
    assert_eq!(history[1].active_index, Some(1));
    assert_eq!(history[1].guidance_replan_count, Some(3));
}

#[test]
fn waypoint_transition_and_joint_evidence_require_the_immediate_next_index() {
    let scenario = waypoint_contract_scenario();
    let mut waypoint = WaypointReviewMetrics {
        active_index: Some(0),
        final_terminal_required_accel_ratio: Some(0.76),
        final_terminal_recoverable: Some(true),
        transition_next_waypoint_index: Some(2),
        transition_position_error_m: Some(4.0),
        transition_continuation_contract_pass: Some(true),
        joint_next_waypoint_index: Some(2),
        joint_evaluated_candidate_count: Some(4),
        joint_passing_candidate_count: Some(2),
        joint_contract_pass: Some(true),
        ..WaypointReviewMetrics::default()
    };

    let mismatched = batch_waypoint_handoff_metrics(&scenario, &waypoint).unwrap();
    assert_eq!(mismatched.transition_next_waypoint_index, None);
    assert_eq!(mismatched.transition_position_error_m, None);
    assert_eq!(mismatched.joint_next_waypoint_index, None);
    assert_eq!(mismatched.joint_evaluated_candidate_count, None);
    assert_eq!(mismatched.final_terminal_required_accel_ratio, Some(0.76));
    assert_eq!(mismatched.final_terminal_recoverable, Some(true));

    waypoint.transition_next_waypoint_index = Some(1);
    waypoint.joint_next_waypoint_index = Some(1);
    let matched = batch_waypoint_handoff_metrics(&scenario, &waypoint).unwrap();
    assert_eq!(matched.transition_next_waypoint_index, Some(1));
    assert_eq!(matched.transition_position_error_m, Some(4.0));
    assert_eq!(matched.joint_next_waypoint_index, Some(1));
    assert_eq!(matched.joint_evaluated_candidate_count, Some(4));
    assert_eq!(matched.joint_passing_candidate_count, Some(2));
    assert_eq!(matched.joint_contract_pass, Some(true));
}

#[test]
fn waypoint_review_schema_reads_legacy_continuation_without_audit_fields() {
    let handoff: BatchWaypointHandoffReviewMetrics = serde_json::from_value(serde_json::json!({
        "waypoint_index": 0,
        "continuation_next_waypoint_index": 1,
        "continuation_contract_pass": true
    }))
    .unwrap();

    assert_eq!(handoff.continuation_next_waypoint_index, Some(1));
    assert_eq!(handoff.continuation_contract_pass, Some(true));
    assert_eq!(handoff.transition_next_waypoint_index, None);
    assert_eq!(handoff.joint_next_waypoint_index, None);
    assert!(handoff.window_entry.is_none());
    assert_eq!(handoff.resolution_reason, None);
    assert_eq!(handoff.window_duration_s, None);
    assert_eq!(handoff.final_terminal_required_accel_ratio, None);
    assert_eq!(handoff.final_terminal_recoverable, None);
}

#[test]
fn waypoint_candidate_history_distinguishes_lost_and_never_passing_plans() {
    let mut scenario = waypoint_contract_scenario();
    let waypoint = &mut scenario.mission.transfer_route.as_mut().unwrap().waypoints[0];
    waypoint.max_outbound_heading_error_rad = 0.35;
    waypoint.max_outbound_cross_speed_mps = Some(20.0);

    let guidance_update = |physics_step: u64,
                           sim_time_s: f64,
                           passed: bool,
                           heading_error_rad: f64,
                           cross_speed_mps: f64| {
        let mut update = transfer_update(physics_step, sim_time_s, "waypoint", 100.0, 20.0, 0.8);
        update.frame.metrics.extend(BTreeMap::from([
            (
                metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
                TelemetryValue::from(true),
            ),
            (
                metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
                TelemetryValue::from(0_i64),
            ),
            (
                metric::WAYPOINT_GUIDANCE_PLAN_INDEX.to_owned(),
                TelemetryValue::from(0_i64),
            ),
            (
                metric::WAYPOINT_GUIDANCE_PLAN_REVISION.to_owned(),
                TelemetryValue::from(if passed { 1_i64 } else { 0_i64 }),
            ),
            (
                metric::WAYPOINT_GUIDANCE_PLAN_REASON.to_owned(),
                TelemetryValue::from(if passed {
                    "authority_recovery"
                } else {
                    "initial"
                }),
            ),
            (
                metric::WAYPOINT_GUIDANCE_REFERENCE_POSITION_ERROR_M.to_owned(),
                TelemetryValue::from(if passed { 20.0 } else { 10.0 }),
            ),
            (
                metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_ERROR_M.to_owned(),
                TelemetryValue::from(if passed { -8.0 } else { -3.0 }),
            ),
            (
                metric::WAYPOINT_GUIDANCE_REFERENCE_VELOCITY_ERROR_MPS.to_owned(),
                TelemetryValue::from(if passed { 4.0 } else { 2.0 }),
            ),
            (
                metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_SPEED_ERROR_MPS.to_owned(),
                TelemetryValue::from(if passed { -2.0 } else { -1.0 }),
            ),
            (
                metric::WAYPOINT_GUIDANCE_REQUIRED_ACCEL_RATIO.to_owned(),
                TelemetryValue::from(if passed { 0.8 } else { 1.2 }),
            ),
            (
                metric::WAYPOINT_GUIDANCE_THRUST_SATURATED.to_owned(),
                TelemetryValue::from(!passed),
            ),
            (
                metric::WAYPOINT_GUIDANCE_TILT_SATURATED.to_owned(),
                TelemetryValue::from(false),
            ),
            (
                metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS.to_owned(),
                TelemetryValue::from(passed),
            ),
            (
                metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
                TelemetryValue::from(heading_error_rad),
            ),
            (
                metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
                TelemetryValue::from(cross_speed_mps),
            ),
            (
                metric::WAYPOINT_REACHABLE_HANDOFF_CONTRACT_PASS.to_owned(),
                TelemetryValue::from(false),
            ),
            (
                metric::WAYPOINT_REACHABLE_HANDOFF_REQUIRED_ACCEL_RATIO_MAX.to_owned(),
                TelemetryValue::from(if passed { 1.4 } else { 1.1 }),
            ),
            (
                metric::WAYPOINT_REACHABLE_HANDOFF_THRUST_SATURATED_TIME_S.to_owned(),
                TelemetryValue::from(if passed { 0.2 } else { 0.1 }),
            ),
            (
                metric::WAYPOINT_REACHABLE_HANDOFF_TILT_SATURATED_TIME_S.to_owned(),
                TelemetryValue::from(if passed { 0.3 } else { 0.0 }),
            ),
        ]));
        update
    };
    let mut handoff = guidance_update(180, 1.5, false, 0.4, 21.0);
    handoff.frame.metrics.insert(
        metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
        TelemetryValue::from("captured"),
    );
    handoff.frame.metrics.insert(
        metric::WAYPOINT_CAPTURE_TIME_S.to_owned(),
        TelemetryValue::from(1.5),
    );
    handoff.frame.metrics.insert(
        metric::WAYPOINT_GUIDANCE_PLAN_INDEX.to_owned(),
        TelemetryValue::from(1_i64),
    );
    handoff.frame.markers.push(pd_control::ControllerMarker {
        id: marker::WAYPOINT_HANDOFF.to_owned(),
        label: "waypoint 0".to_owned(),
        x_m: None,
        y_m: None,
        metadata: BTreeMap::new(),
    });
    let updates = vec![
        guidance_update(60, 0.5, false, 0.5, 24.0),
        guidance_update(120, 1.0, true, 0.2, 18.0),
        handoff,
    ];
    let mut history = waypoint_review_history(&updates);

    enrich_waypoint_candidate_history(&scenario, &updates, &mut history);

    assert_eq!(history[0].candidate_contract_pass_ever, Some(true));
    assert_eq!(history[0].candidate_first_pass_time_s, Some(1.0));
    assert_eq!(history[0].candidate_last_pass_time_s, Some(1.0));
    assert_eq!(history[0].candidate_pass_lost_before_capture, Some(false));
    assert!((history[0].candidate_best_heading_margin_rad.unwrap() - 0.15).abs() < 1.0e-9);
    assert_eq!(history[0].candidate_best_cross_speed_margin_mps, Some(2.0));
    assert_eq!(
        history[0].reachable_candidate_contract_pass_ever,
        Some(false)
    );
    assert_eq!(
        history[0].reachable_candidate_pass_lost_before_capture,
        Some(false)
    );
    assert_eq!(history[0].reachable_required_accel_ratio_max, Some(1.4));
    assert_eq!(history[0].reachable_thrust_saturated_time_max_s, Some(0.2));
    assert_eq!(history[0].reachable_tilt_saturated_time_max_s, Some(0.3));
    assert_eq!(history[0].plan_reference_position_error_max_m, Some(20.0));
    assert_eq!(history[0].plan_reference_cross_error_max_abs_m, Some(8.0));
    assert_eq!(history[0].plan_reference_velocity_error_max_mps, Some(4.0));
    assert_eq!(history[0].guidance_required_accel_ratio_max, Some(1.2));
    assert_eq!(history[0].guidance_thrust_saturated_time_s, Some(0.5));
    assert_eq!(history[0].guidance_tilt_saturated_time_s, Some(0.0));
    assert_eq!(history[0].guidance_first_saturation_lead_s, Some(1.0));
    assert_eq!(history[0].last_pass_reference_position_error_m, Some(20.0));
    assert_eq!(history[0].last_pass_reference_velocity_error_mps, Some(4.0));
    assert_eq!(history[0].last_pass_required_accel_ratio, Some(0.8));
    assert_eq!(history[0].guidance_plan_revision_max, Some(1));
    assert_eq!(
        history[0].guidance_plan_reasons,
        vec!["authority_recovery", "initial"]
    );
}

#[test]
fn waypoint_route_review_uses_authoritative_sequence_progress() {
    let mut scenario = waypoint_contract_scenario();
    scenario.mission.goal = EvaluationGoal::WaypointSequence {
        target_pad_id: "pad_a".to_owned(),
    };
    let route = scenario.mission.transfer_route.as_mut().unwrap();
    route.waypoints.push(TransferWaypointSpec {
        id: "wp_1".to_owned(),
        position_m: Vec2::new(-100.0, 120.0),
        ..route.waypoints[0].clone()
    });
    let manifest = RunManifest {
        schema_version: pd_core::model::RUN_SCHEMA_VERSION,
        scenario_id: scenario.id.clone(),
        scenario_name: scenario.name.clone(),
        scenario_seed: scenario.seed,
        scenario_tags: scenario.tags.clone(),
        controller_id: "transfer_waypoint_pdg_v1".to_owned(),
        physics_hz: scenario.sim.physics_hz,
        controller_hz: scenario.sim.controller_hz,
        sim_time_s: 1.0,
        physics_steps: 120,
        controller_updates: 2,
        physical_outcome: pd_core::PhysicalOutcome::Flying,
        mission_outcome: MissionOutcome::FailedCheckpoint,
        end_reason: EndReason::CheckpointFailed,
        summary: RunSummary {
            waypoint_sequence: Some(pd_core::model::WaypointSequenceRunSummary {
                passed_handoffs: 1,
                total_handoffs: 2,
                first_failed_index: Some(1),
            }),
            ..RunSummary::default()
        },
    };
    let handoffs = vec![BatchWaypointHandoffReviewMetrics {
        waypoint_index: 0,
        contract_status: Some("pass".to_owned()),
        ..BatchWaypointHandoffReviewMetrics::default()
    }];

    let route = waypoint_route_review_metrics(&scenario, &manifest, &handoffs);

    assert_eq!(route.status.as_deref(), Some("failed"));
    assert_eq!(route.passed, Some(1));
    assert_eq!(route.total, Some(2));
    assert_eq!(route.first_failure_index, Some(1));
}

#[test]
fn waypoint_handoff_goal_review_uses_terminal_sample_state() {
    let mut scenario = waypoint_contract_scenario();
    scenario.mission.goal = EvaluationGoal::WaypointHandoff {
        target_pad_id: "pad_a".to_owned(),
        waypoint_index: 0,
    };
    let samples = vec![
        transfer_sample(0, 0.0, Vec2::new(-420.0, 120.0), Vec2::new(0.0, 0.0), 120.0),
        transfer_sample(
            60,
            0.5,
            Vec2::new(-220.0, 180.0),
            Vec2::new(22.0, -18.0),
            118.0,
        ),
    ];
    let manifest = RunManifest {
        schema_version: pd_core::model::RUN_SCHEMA_VERSION,
        scenario_id: scenario.id.clone(),
        scenario_name: scenario.name.clone(),
        scenario_seed: scenario.seed,
        scenario_tags: scenario.tags.clone(),
        controller_id: "transfer_waypoint_pdg_v1".to_owned(),
        physics_hz: scenario.sim.physics_hz,
        controller_hz: scenario.sim.controller_hz,
        sim_time_s: 0.5,
        physics_steps: 60,
        controller_updates: 2,
        physical_outcome: pd_core::PhysicalOutcome::Flying,
        mission_outcome: MissionOutcome::Success,
        end_reason: EndReason::CheckpointSatisfied,
        summary: RunSummary::default(),
    };
    let mut guidance_update = transfer_update(48, 0.4, "boost", 100.0, 20.0, 0.8);
    guidance_update.frame.metrics.extend(BTreeMap::from([
        (
            metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
            TelemetryValue::from(true),
        ),
        (
            metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
            TelemetryValue::from(0_i64),
        ),
        (
            metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
            TelemetryValue::from("tracking"),
        ),
        (
            metric::WAYPOINT_TARGET_VX_MPS.to_owned(),
            TelemetryValue::from(30.0),
        ),
        (
            metric::WAYPOINT_TARGET_VY_MPS.to_owned(),
            TelemetryValue::from(0.0),
        ),
        (
            metric::WAYPOINT_TARGET_DEADLINE_REMAINING_S.to_owned(),
            TelemetryValue::from(2.5),
        ),
        (
            metric::WAYPOINT_REQUIRED_TURN_DISTANCE_M.to_owned(),
            TelemetryValue::from(30.0),
        ),
        (
            metric::WAYPOINT_GUIDANCE_FEASIBLE.to_owned(),
            TelemetryValue::from(false),
        ),
        (
            metric::WAYPOINT_PREDICTED_HANDOFF_TIME_TO_GO_S.to_owned(),
            TelemetryValue::from(0.2),
        ),
        (
            metric::WAYPOINT_PREDICTED_HANDOFF_DEADLINE_LEAD_S.to_owned(),
            TelemetryValue::from(2.3),
        ),
        (
            metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS.to_owned(),
            TelemetryValue::from(false),
        ),
        (
            metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_REASONS.to_owned(),
            TelemetryValue::from("heading,outbound_cross_speed"),
        ),
        (
            metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
            TelemetryValue::from(0.5),
        ),
    ]));

    let metrics =
        waypoint_handoff_goal_review_metrics(&scenario, &samples, &manifest, &[guidance_update])
            .expect("waypoint handoff review should be available");
    let contract = waypoint_contract_review_metrics(&scenario, &metrics);

    assert_eq!(metrics.capture_status.as_deref(), Some("captured"));
    assert_eq!(metrics.active_index, Some(0));
    assert_eq!(metrics.capture_time_s, Some(0.5));
    assert_eq!(metrics.closest_distance_m, Some(0.0));
    assert_eq!(metrics.distance_m, Some(0.0));
    assert_eq!(metrics.target_deadline_remaining_s, Some(2.4));
    assert!((metrics.predicted_handoff_time_to_go_s.unwrap() - 0.1).abs() < 1.0e-9);
    assert_eq!(metrics.predicted_handoff_deadline_lead_s, Some(2.3));
    assert_eq!(
        metrics.predicted_handoff_contract_status.as_deref(),
        Some("fail")
    );
    assert_eq!(
        metrics.predicted_handoff_contract_reasons,
        vec!["heading", "outbound_cross_speed"]
    );
    assert_eq!(
        metrics.predicted_handoff_outbound_heading_error_rad,
        Some(0.5)
    );
    assert!((metrics.target_velocity_error_mps.unwrap() - 19.6977156036).abs() < 1.0e-9);
    assert_eq!(metrics.handoff_turn_margin_m, Some(-30.0));
    assert_eq!(
        metrics.guidance_snapshot_source.as_deref(),
        Some("last_pre_capture_update")
    );
    assert!((metrics.guidance_snapshot_age_s.unwrap() - 0.1).abs() < 1.0e-9);
    assert_eq!(contract.status.as_deref(), Some("pass"));
}

#[test]
fn transfer_shape_metrics_use_boost_window_reference() {
    let mut scenario = easy_landing_scenario();
    scenario.world.landing_pads.push(LandingPadSpec {
        id: "source".to_owned(),
        center_x_m: -200.0,
        surface_y_m: 50.0,
        width_m: 36.0,
    });
    scenario.mission.transfer_route = Some(TransferRouteSpec {
        source_pad_id: "source".to_owned(),
        target_pad_id: "pad_a".to_owned(),
        route_angle_deg: 0.0,
        route_radius_m: 200.0,
        waypoints: Vec::new(),
    });
    let samples = vec![
        transfer_sample(
            0,
            0.0,
            Vec2::new(-200.0, 50.0),
            Vec2::new(40.0, 20.0),
            100.0,
        ),
        transfer_sample(
            60,
            0.5,
            Vec2::new(-100.0, 90.0),
            Vec2::new(40.0, -5.0),
            96.0,
        ),
        transfer_sample(120, 1.0, Vec2::new(0.0, 0.0), Vec2::new(0.0, -10.0), 94.0),
    ];
    let updates = vec![
        transfer_update(0, 0.0, "boost", 200.0, 80.0, 1.0),
        transfer_update(60, 0.5, "boost", 100.0, 20.0, 0.8),
        transfer_update(120, 1.0, "terminal", 0.0, 0.0, 0.0),
    ];

    let metrics = transfer_shape_metrics(&scenario, &samples, &updates)
        .expect("transfer shape metrics should be available");

    assert!((metrics.curve_rmse_m - 12.124_355_65).abs() < 1e-6);
    assert!((metrics.apex_error_m - 4.0).abs() < 1e-9);
    assert_eq!(metrics.projected_dx_abs_mean_m, Some(50.0));
    assert_eq!(metrics.projected_dx_abs_max_m, Some(80.0));
    assert_eq!(metrics.shortfall_ratio, Some(1.0));
}

#[test]
fn transfer_burn_metrics_use_last_boost_when_no_cutoff_exists() {
    let samples = vec![
        transfer_sample(
            0,
            0.0,
            Vec2::new(-200.0, 50.0),
            Vec2::new(40.0, 20.0),
            100.0,
        ),
        transfer_sample(
            60,
            0.5,
            Vec2::new(-100.0, 90.0),
            Vec2::new(40.0, -5.0),
            96.0,
        ),
    ];
    let updates = vec![
        transfer_update(0, 0.0, "boost", 200.0, 80.0, 1.0),
        transfer_update(60, 0.5, "boost", 100.0, 20.0, 0.5),
    ];

    let metrics = transfer_review_metrics(&updates, &samples);

    assert_eq!(metrics.final_phase.as_deref(), Some("boost"));
    assert_eq!(metrics.boost_cutoff_time_s, None);
    assert_eq!(metrics.boost_burn_duration_s, Some(0.5));
    assert_eq!(metrics.boost_burn_fuel_used_kg, Some(4.0));
    assert_eq!(metrics.boost_burn_avg_throttle, Some(1.0));
}

#[test]
fn transfer_review_metrics_capture_active_corridor_margin() {
    let mut updates = vec![
        transfer_update(0, 0.0, "boost", 200.0, 80.0, 1.0),
        transfer_update(60, 0.5, "boost", 100.0, 20.0, 1.0),
    ];
    updates[0].frame.metrics.insert(
        metric::TRANSFER_CORRIDOR_MODE.to_owned(),
        TelemetryValue::from("inactive"),
    );
    updates[0].frame.metrics.insert(
        metric::TRANSFER_CORRIDOR_MARGIN_M.to_owned(),
        TelemetryValue::from(1.0e9),
    );
    updates[1].frame.metrics.insert(
        metric::TRANSFER_CORRIDOR_MODE.to_owned(),
        TelemetryValue::from("active"),
    );
    updates[1].frame.metrics.insert(
        metric::TRANSFER_CORRIDOR_MARGIN_M.to_owned(),
        TelemetryValue::from(-32.0),
    );

    let metrics = transfer_review_metrics(&updates, &[]);

    assert_eq!(metrics.corridor_mode.as_deref(), Some("active"));
    assert_eq!(metrics.corridor_min_margin_m, Some(-32.0));
}

#[test]
fn transfer_review_metrics_preserve_any_deferred_gate() {
    let mut updates = vec![
        transfer_update(0, 0.0, "boost", 200.0, 80.0, 1.0),
        transfer_update(60, 0.5, "terminal", 20.0, 0.0, 0.0),
    ];
    updates[0].frame.metrics.insert(
        metric::TRANSFER_TERMINAL_GATE_MODE.to_owned(),
        TelemetryValue::from("pending"),
    );
    updates[0].frame.metrics.insert(
        metric::TRANSFER_TERMINAL_GATE_DEFERRED.to_owned(),
        TelemetryValue::from(true),
    );
    updates[1].frame.metrics.insert(
        metric::TRANSFER_TERMINAL_GATE_MODE.to_owned(),
        TelemetryValue::from("latest_safe"),
    );
    updates[1].frame.metrics.insert(
        metric::TRANSFER_TERMINAL_GATE_DEFERRED.to_owned(),
        TelemetryValue::from(false),
    );

    let metrics = transfer_review_metrics(&updates, &[]);

    assert_eq!(metrics.terminal_gate_mode.as_deref(), Some("latest_safe"));
    assert_eq!(metrics.terminal_gate_deferred, Some(true));
}

#[test]
fn transfer_review_metrics_marks_direct_terminal_entry() {
    let mut updates = vec![transfer_update(60, 0.5, "terminal", 120.0, 20.0, 0.0)];
    updates[0].frame.metrics.insert(
        metric::TRANSFER_TERMINAL_GATE_MODE.to_owned(),
        TelemetryValue::from("pending"),
    );

    let metrics = transfer_review_metrics(&updates, &[]);

    assert_eq!(metrics.terminal_entry_kind.as_deref(), Some("direct"));
    assert_eq!(
        metrics.terminal_handoff_gate_mode.as_deref(),
        Some("pending")
    );
}

#[test]
fn transfer_review_metrics_capture_terminal_handoff() {
    let updates = vec![
        pd_control::ControllerUpdateRecord {
            sim_time_s: 1.0,
            physics_step: 120,
            controller_update_index: 0,
            compute_time_us: None,
            frame: pd_control::ControllerFrame {
                command: pd_core::Command::idle(),
                status: "boosting".to_owned(),
                phase: Some("boost".to_owned()),
                metrics: BTreeMap::from([
                    (
                        metric::TRANSFER_PHASE.to_owned(),
                        TelemetryValue::from("boost"),
                    ),
                    (
                        metric::TRANSFER_PROJECTED_DX_M.to_owned(),
                        TelemetryValue::from(42.0),
                    ),
                    (
                        metric::TRANSFER_IMPACT_ANGLE_DEG.to_owned(),
                        TelemetryValue::from(61.0),
                    ),
                    (
                        metric::TRANSFER_APEX_OVER_TARGET_M.to_owned(),
                        TelemetryValue::from(120.0),
                    ),
                    (
                        metric::TRANSFER_BOOST_QUALITY.to_owned(),
                        TelemetryValue::from("pass"),
                    ),
                ]),
                markers: Vec::new(),
            },
        },
        pd_control::ControllerUpdateRecord {
            sim_time_s: 3.5,
            physics_step: 420,
            controller_update_index: 1,
            compute_time_us: None,
            frame: pd_control::ControllerFrame {
                command: pd_core::Command::idle(),
                status: "terminal handoff".to_owned(),
                phase: Some("terminal".to_owned()),
                metrics: BTreeMap::from([
                    (
                        metric::TRANSFER_PHASE.to_owned(),
                        TelemetryValue::from("terminal"),
                    ),
                    (metric::TARGET_DX_M.to_owned(), TelemetryValue::from(120.0)),
                    (
                        metric::HEIGHT_ABOVE_TARGET_M.to_owned(),
                        TelemetryValue::from(80.0),
                    ),
                    (
                        metric::VERTICAL_SPEED_MPS.to_owned(),
                        TelemetryValue::from(-12.0),
                    ),
                    (
                        metric::TANGENTIAL_SPEED_MPS.to_owned(),
                        TelemetryValue::from(5.0),
                    ),
                    (
                        metric::TRANSFER_TERMINAL_GATE_MODE.to_owned(),
                        TelemetryValue::from("pending"),
                    ),
                    (
                        metric::TRANSFER_PROJECTED_DX_M.to_owned(),
                        TelemetryValue::from(-18.0),
                    ),
                    (
                        metric::TRANSFER_IMPACT_ANGLE_DEG.to_owned(),
                        TelemetryValue::from(47.0),
                    ),
                    (
                        metric::TRANSFER_BOOST_QUALITY.to_owned(),
                        TelemetryValue::from("angle"),
                    ),
                    (
                        metric::TRANSFER_TERMINAL_GATE_LATEST_SAFE_MARGIN_S.to_owned(),
                        TelemetryValue::from(1.25),
                    ),
                    (
                        metric::TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO.to_owned(),
                        TelemetryValue::from(0.83),
                    ),
                ]),
                markers: Vec::new(),
            },
        },
    ];

    let metrics = transfer_review_metrics(&updates, &[]);

    assert_eq!(metrics.final_phase.as_deref(), Some("terminal"));
    assert_eq!(metrics.terminal_entry_kind.as_deref(), Some("handoff"));
    assert_eq!(metrics.terminal_handoff_time_s, Some(3.5));
    assert_eq!(metrics.terminal_handoff_dx_m, Some(120.0));
    assert_eq!(metrics.terminal_handoff_height_m, Some(80.0));
    assert_eq!(metrics.terminal_handoff_speed_mps, Some(13.0));
    assert_eq!(
        metrics.terminal_handoff_gate_mode.as_deref(),
        Some("pending")
    );
    assert_eq!(metrics.terminal_handoff_projected_dx_m, Some(-18.0));
    assert_eq!(metrics.terminal_handoff_impact_angle_deg, Some(47.0));
    assert_eq!(
        metrics.terminal_handoff_boost_quality.as_deref(),
        Some("angle")
    );
    assert_eq!(metrics.terminal_handoff_latest_safe_margin_s, Some(1.25));
    assert_eq!(metrics.terminal_handoff_required_accel_ratio, Some(0.83));
    assert_eq!(metrics.boost_projected_dx_m, Some(42.0));
    assert_eq!(metrics.boost_impact_angle_deg, Some(61.0));
    assert_eq!(metrics.boost_apex_over_target_m, Some(120.0));
    assert_eq!(metrics.boost_quality.as_deref(), Some("pass"));
    assert_eq!(metrics.boost_cutoff_time_s, Some(1.0));
    assert_eq!(metrics.boost_cutoff_quality.as_deref(), Some("pass"));
}

#[test]
fn transfer_review_metrics_capture_post_handoff_apex() {
    let updates = vec![
        transfer_update(60, 0.5, "boost", 200.0, 80.0, 1.0),
        transfer_update(120, 1.0, "terminal", 120.0, 20.0, 0.8),
    ];
    let samples = vec![
        transfer_sample(
            120,
            1.0,
            Vec2::new(-120.0, 80.0),
            Vec2::new(40.0, 15.0),
            95.0,
        ),
        transfer_sample(
            240,
            2.0,
            Vec2::new(-60.0, 110.0),
            Vec2::new(25.0, 0.0),
            90.0,
        ),
        transfer_sample(
            360,
            3.0,
            Vec2::new(-10.0, 100.0),
            Vec2::new(10.0, -10.0),
            88.0,
        ),
    ];

    let metrics = transfer_review_metrics(&updates, &samples);

    assert_eq!(metrics.terminal_post_handoff_apex_gain_m, Some(30.0));
    assert_eq!(metrics.terminal_post_handoff_time_to_apex_s, Some(1.0));
    assert_eq!(metrics.terminal_post_handoff_apex_dx_abs_m, Some(60.0));
}

#[test]
fn transfer_review_metrics_ignore_low_altitude_ascent_before_descent() {
    let updates = vec![transfer_update(120, 1.0, "terminal", 30.0, 20.0, 0.8)];
    let samples = vec![
        transfer_sample(120, 1.0, Vec2::new(-30.0, 20.0), Vec2::new(5.0, 3.0), 95.0),
        transfer_sample(240, 2.0, Vec2::new(-20.0, 24.0), Vec2::new(5.0, 2.0), 90.0),
    ];

    let metrics = transfer_review_metrics(&updates, &samples);

    assert_eq!(metrics.terminal_low_altitude_rebound_gain_m, None);
    assert_eq!(metrics.terminal_low_altitude_rebound_origin_dx_abs_m, None);
    assert_eq!(metrics.terminal_low_altitude_rebound_near_pad, None);
}

#[test]
fn transfer_review_metrics_capture_near_pad_low_altitude_rebound() {
    let updates = vec![transfer_update(120, 1.0, "terminal", 30.0, 24.0, 0.8)];
    let samples = vec![
        transfer_sample(120, 1.0, Vec2::new(-30.0, 24.0), Vec2::new(5.0, -4.0), 95.0),
        transfer_sample(240, 2.0, Vec2::new(-20.0, 10.0), Vec2::new(3.0, -2.0), 90.0),
        transfer_sample(360, 3.0, Vec2::new(-15.0, 16.0), Vec2::new(2.0, 2.0), 88.0),
    ];

    let metrics = transfer_review_metrics(&updates, &samples);

    assert_eq!(metrics.terminal_low_altitude_rebound_gain_m, Some(6.0));
    assert_eq!(
        metrics.terminal_low_altitude_rebound_origin_dx_abs_m,
        Some(20.0)
    );
    assert_eq!(metrics.terminal_low_altitude_rebound_near_pad, Some(true));
}

#[test]
fn transfer_review_metrics_distinguish_far_target_recovery_climb() {
    let updates = vec![transfer_update(120, 1.0, "terminal", 220.0, 24.0, 0.8)];
    let samples = vec![
        transfer_sample(
            120,
            1.0,
            Vec2::new(-220.0, 24.0),
            Vec2::new(5.0, -4.0),
            95.0,
        ),
        transfer_sample(
            240,
            2.0,
            Vec2::new(-200.0, 10.0),
            Vec2::new(3.0, -2.0),
            90.0,
        ),
        transfer_sample(360, 3.0, Vec2::new(-180.0, 20.0), Vec2::new(2.0, 3.0), 88.0),
    ];

    let metrics = transfer_review_metrics(&updates, &samples);

    assert_eq!(metrics.terminal_low_altitude_rebound_gain_m, Some(10.0));
    assert_eq!(
        metrics.terminal_low_altitude_rebound_origin_dx_abs_m,
        Some(200.0)
    );
    assert_eq!(metrics.terminal_low_altitude_rebound_near_pad, Some(false));
}

#[test]
fn transfer_review_metrics_report_zero_for_monotonic_low_altitude_descent() {
    let updates = vec![transfer_update(120, 1.0, "terminal", 30.0, 24.0, 0.8)];
    let samples = vec![
        transfer_sample(120, 1.0, Vec2::new(-30.0, 24.0), Vec2::new(5.0, -4.0), 95.0),
        transfer_sample(240, 2.0, Vec2::new(-20.0, 10.0), Vec2::new(3.0, -2.0), 90.0),
    ];

    let metrics = transfer_review_metrics(&updates, &samples);

    assert_eq!(metrics.terminal_low_altitude_rebound_gain_m, Some(0.0));
    assert_eq!(
        metrics.terminal_low_altitude_rebound_origin_dx_abs_m,
        Some(30.0)
    );
    assert_eq!(metrics.terminal_low_altitude_rebound_near_pad, Some(true));
}

#[test]
fn transfer_matrix_entry_rejects_duplicate_route_angle_selectors() {
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_duplicate_routes".to_owned(),
        name: "Transfer matrix duplicate routes".to_owned(),
        description: "transfer matrix duplicate routes".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
            id: "transfer_guidance_selected_routes".to_owned(),
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
            route_angles: vec!["r00".to_owned(), "r00".to_owned()],
            radius_tiers: Vec::new(),
            waypoint_profile: None,
            waypoint_handoff_envelope: None,
            evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
            adjustments: Vec::new(),
            tags: Vec::new(),
            metadata: BTreeMap::new(),
        })],
    };

    let message = validate_pack(&pack).unwrap_err().to_string();

    assert!(message.contains("duplicate route_angle selector"));
}

#[test]
fn transfer_matrix_entry_rejects_duplicate_radius_tier_selectors() {
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_duplicate_radii".to_owned(),
        name: "Transfer matrix duplicate radii".to_owned(),
        description: "transfer matrix duplicate radii".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
            id: "transfer_guidance_selected_radii".to_owned(),
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
            route_angles: Vec::new(),
            radius_tiers: vec!["short".to_owned(), "short".to_owned()],
            waypoint_profile: None,
            waypoint_handoff_envelope: None,
            evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
            adjustments: Vec::new(),
            tags: Vec::new(),
            metadata: BTreeMap::new(),
        })],
    };

    let message = validate_pack(&pack).unwrap_err().to_string();

    assert!(message.contains("duplicate radius_tier selector"));
}

#[test]
fn transfer_matrix_entry_rejects_unknown_radius_tier_selector() {
    let pack = ScenarioPackSpec {
        id: "transfer_matrix_unknown_radius".to_owned(),
        name: "Transfer matrix unknown radius".to_owned(),
        description: "transfer matrix unknown radius".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
            id: "transfer_guidance_unknown_radius".to_owned(),
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
            route_angles: Vec::new(),
            radius_tiers: vec!["extreme".to_owned()],
            waypoint_profile: None,
            waypoint_handoff_envelope: None,
            evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
            adjustments: Vec::new(),
            tags: Vec::new(),
            metadata: BTreeMap::new(),
        })],
    };

    let message = validate_pack(&pack).unwrap_err().to_string();

    assert!(message.contains("radius_tier selector 'extreme' is not supported"));
}

#[test]
fn terminal_matrix_eval_timeout_must_not_shorten_reachability_window() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_short_eval_timeout".to_owned(),
        name: "Terminal matrix short eval timeout".to_owned(),
        description: "terminal matrix short eval timeout".to_owned(),
        terminal_matrix_max_time_s: Some(45.0),
        entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
            id: "terminal_guidance_clean_nominal".to_owned(),
            terminal_matrix: "half_arc_terminal_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TerminalMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "staged".to_owned(),
                controller_config: None,
            }],
            seed_tier: TerminalSeedTier::Smoke,
            condition_set: "clean".to_owned(),
            vehicle_variant: "nominal".to_owned(),
            expectation_tier: "core".to_owned(),
            arc_points: Vec::new(),
            adjustments: Vec::new(),
            tags: vec!["terminal".to_owned(), "smoke".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let error = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("terminal_matrix_max_time_s"));
    assert!(message.contains("must be >= scenario reachability max_time_s"));
}

#[test]
fn terminal_matrix_projected_error_conditions_resolve_exact_engine_off_miss() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_traj_error".to_owned(),
        name: "Terminal matrix trajectory error".to_owned(),
        description: "terminal matrix trajectory error".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
            id: "terminal_guidance_traj_overshoot_small_half".to_owned(),
            terminal_matrix: "half_arc_terminal_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TerminalMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "terminal_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TerminalSeedTier::Smoke,
            condition_set: "traj_overshoot_small".to_owned(),
            vehicle_variant: "half".to_owned(),
            expectation_tier: "core".to_owned(),
            arc_points: Vec::new(),
            adjustments: vec![NumericAdjustmentSpec {
                id: "payload_half_mass_kg".to_owned(),
                path: "vehicle.dry_mass_kg".to_owned(),
                mode: NumericPerturbationMode::Offset,
                value: 2250.0,
            }],
            tags: vec!["terminal".to_owned(), "traj_error".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();
    let record = resolved_runs
        .iter()
        .find(|run| {
            run.descriptor.selector.arc_point == "a30"
                && run.descriptor.selector.velocity_band == "mid"
                && run.descriptor.resolved_seed == 0
        })
        .expect("projected-error matrix record present");

    let params = &record.descriptor.resolved_parameters;
    let start_x = params["start_x_m"];
    let start_vx = params["start_vx_mps"];
    let ttg = params["ttg_s"];
    let impact_x = start_x + (start_vx * ttg);

    assert_eq!(
        record.descriptor.selector.condition_set,
        "traj_overshoot_small"
    );
    assert_eq!(
        record
            .scenario
            .metadata
            .get("resolved.traj_error_kind")
            .map(String::as_str),
        Some("overshoot")
    );
    assert_eq!(params["projected_dx_error_mag_m"], 30.0);
    assert!((impact_x - params["projected_dx_error_m"]).abs() < 1e-9);
    assert!((params["speed_scale"] - 1.0).abs() < 1e-12);
}

#[test]
fn terminal_matrix_reactive_terrain_conditions_resolve_geometry() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_reactive_terrain".to_owned(),
        name: "Terminal matrix reactive terrain".to_owned(),
        description: "terminal matrix reactive terrain".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![
            ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_terrain_backstop_wall_half".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "terminal_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "terrain_backstop_wall".to_owned(),
                vehicle_variant: "half".to_owned(),
                expectation_tier: "core".to_owned(),
                arc_points: Vec::new(),
                adjustments: vec![NumericAdjustmentSpec {
                    id: "payload_half_mass_kg".to_owned(),
                    path: "vehicle.dry_mass_kg".to_owned(),
                    mode: NumericPerturbationMode::Offset,
                    value: 2250.0,
                }],
                tags: vec!["terminal".to_owned(), "terrain".to_owned()],
                metadata: BTreeMap::new(),
            }),
            ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_terrain_clip_half".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "terminal_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "terrain_clip".to_owned(),
                vehicle_variant: "half".to_owned(),
                expectation_tier: "core".to_owned(),
                arc_points: Vec::new(),
                adjustments: vec![NumericAdjustmentSpec {
                    id: "payload_half_mass_kg".to_owned(),
                    path: "vehicle.dry_mass_kg".to_owned(),
                    mode: NumericPerturbationMode::Offset,
                    value: 2250.0,
                }],
                tags: vec!["terminal".to_owned(), "terrain".to_owned()],
                metadata: BTreeMap::new(),
            }),
        ],
    };

    let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();
    let backstop = resolved_runs
        .iter()
        .find(|run| {
            run.descriptor.entry_id == "terminal_guidance_terrain_backstop_wall_half"
                && run.descriptor.selector.arc_point == "a30"
                && run.descriptor.selector.velocity_band == "mid"
                && run.descriptor.resolved_seed == 0
        })
        .expect("backstop terrain matrix record present");
    let backstop_params = &backstop.descriptor.resolved_parameters;
    let backstop_feature_side = backstop_params["terrain_feature_side_sign"];
    let backstop_plateau_x = backstop_feature_side
        * (backstop_params["terrain_inner_offset_m"]
            + backstop_params["terrain_shoulder_width_m"]
            + (0.5 * backstop_params["terrain_top_width_m"]));

    assert_eq!(
        backstop
            .scenario
            .metadata
            .get("resolved.condition_kind")
            .map(String::as_str),
        Some("reactive_terrain")
    );
    assert_eq!(
        backstop
            .scenario
            .metadata
            .get("resolved.hazard_driver")
            .map(String::as_str),
        Some("containment_backstop")
    );
    assert_eq!(
        backstop
            .scenario
            .metadata
            .get("resolved.reactive_contract")
            .map(String::as_str),
        Some("execution_guardrail")
    );
    assert_eq!(
        backstop
            .scenario
            .metadata
            .get("resolved.terrain_variant")
            .map(String::as_str),
        Some("wall")
    );
    assert_eq!(backstop_params["terrain_height_offset_m"], 400.0);
    assert_eq!(backstop_feature_side, 1.0);
    assert_eq!(backstop.scenario.world.terrain.sample_height(0.0), 0.0);
    assert!(
        (backstop
            .scenario
            .world
            .terrain
            .sample_height(backstop_plateau_x)
            - backstop_params["terrain_height_offset_m"])
            .abs()
            < 1e-9
    );
    assert_eq!(
        backstop
            .scenario
            .world
            .terrain
            .sample_height(-backstop_plateau_x),
        0.0
    );

    let clip = resolved_runs
        .iter()
        .find(|run| {
            run.descriptor.entry_id == "terminal_guidance_terrain_clip_half"
                && run.descriptor.selector.arc_point == "a30"
                && run.descriptor.selector.velocity_band == "mid"
                && run.descriptor.resolved_seed == 0
        })
        .expect("clip terrain matrix record present");
    let clip_params = &clip.descriptor.resolved_parameters;
    let clip_feature_side = clip_params["terrain_feature_side_sign"];
    let clip_plateau_x = clip_feature_side
        * (clip_params["terrain_inner_offset_m"]
            + clip_params["terrain_shoulder_width_m"]
            + (0.5 * clip_params["terrain_top_width_m"]));

    assert_eq!(
        clip.scenario
            .metadata
            .get("resolved.hazard_driver")
            .map(String::as_str),
        Some("descent_clip")
    );
    assert_eq!(
        clip.scenario
            .metadata
            .get("resolved.obstacle_placement")
            .map(String::as_str),
        Some("terminal_approach")
    );
    assert_eq!(clip_feature_side, -1.0);
    assert_eq!(clip.scenario.world.terrain.sample_height(0.0), 0.0);
    assert!(
        (clip.scenario.world.terrain.sample_height(clip_plateau_x)
            - clip_params["terrain_height_offset_m"])
            .abs()
            < 1e-9
    );
}

#[test]
fn terminal_matrix_entry_rejects_unknown_condition_set() {
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_unknown_condition".to_owned(),
        name: "Terminal matrix unknown condition".to_owned(),
        description: "terminal matrix unknown condition".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
            id: "terminal_guidance_traj_typo_half".to_owned(),
            terminal_matrix: "half_arc_terminal_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TerminalMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "terminal_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TerminalSeedTier::Smoke,
            condition_set: "traj_overshot_small".to_owned(),
            vehicle_variant: "half".to_owned(),
            expectation_tier: "core".to_owned(),
            arc_points: Vec::new(),
            adjustments: vec![],
            tags: vec![],
            metadata: BTreeMap::new(),
        })],
    };

    let err = validate_pack(&pack).expect_err("unknown condition set should be rejected");

    assert!(err.to_string().contains("unsupported condition_set"));
}

#[test]
fn analytic_vertical_bound_invalidates_heavy_cargo_vertical_high_cases() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_heavy_cargo_full".to_owned(),
        name: "Terminal matrix heavy cargo full".to_owned(),
        description: "terminal matrix heavy cargo full".to_owned(),
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
            adjustments: vec![NumericAdjustmentSpec {
                id: "payload_full_mass_kg".to_owned(),
                path: "vehicle.dry_mass_kg".to_owned(),
                mode: NumericPerturbationMode::Offset,
                value: 4500.0,
            }],
            tags: vec!["terminal".to_owned(), "analytic".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let report = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
    let record = report
        .records
        .iter()
        .find(|record| {
            record.resolved.selector.arc_point == "a00"
                && record.resolved.selector.velocity_band == "high"
                && record.resolved.resolved_seed == 0
        })
        .expect("heavy cargo a00 high seed 0 should be present");

    assert_eq!(record.analytic.class, BatchRunAnalyticClass::Impossible);
    assert_eq!(
        record.analytic.reason,
        Some(BatchRunAnalyticReason::VerticalStopHeight)
    );
    assert!(
        record
            .analytic
            .stop_height_margin_m
            .expect("vertical stop margin should be present")
            < 0.0
    );
    assert!(report.summary.invalidated_runs >= 24);
    assert!(
        report
            .summary
            .by_entry
            .iter()
            .find(|group| group.key == "terminal_guidance_clean_heavy_cargo")
            .expect("entry summary should exist")
            .invalidated_runs
            >= 24
    );
}

#[test]
fn analytic_authority_frontier_marks_low_thrust_high_energy_cases_but_keeps_them_scored() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_full_payload_lateral".to_owned(),
        name: "Terminal matrix full payload lateral".to_owned(),
        description: "terminal matrix full payload lateral".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![
            ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_clean_half_payload".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "terminal_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "clean".to_owned(),
                vehicle_variant: "half".to_owned(),
                expectation_tier: "core".to_owned(),
                arc_points: Vec::new(),
                adjustments: vec![NumericAdjustmentSpec {
                    id: "payload_half_mass_kg".to_owned(),
                    path: "vehicle.dry_mass_kg".to_owned(),
                    mode: NumericPerturbationMode::Offset,
                    value: 2250.0,
                }],
                tags: vec!["terminal".to_owned(), "analytic".to_owned()],
                metadata: BTreeMap::new(),
            }),
            ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
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
                adjustments: vec![NumericAdjustmentSpec {
                    id: "payload_full_mass_kg".to_owned(),
                    path: "vehicle.dry_mass_kg".to_owned(),
                    mode: NumericPerturbationMode::Offset,
                    value: 4500.0,
                }],
                tags: vec!["terminal".to_owned(), "analytic".to_owned()],
                metadata: BTreeMap::new(),
            }),
        ],
    };

    let report = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
    let half_a45_high = report
        .records
        .iter()
        .find(|record| {
            record.resolved.entry_id == "terminal_guidance_clean_half_payload"
                && record.resolved.selector.arc_point == "a45"
                && record.resolved.selector.velocity_band == "high"
                && record.resolved.resolved_seed == 0
        })
        .expect("half payload a45 high seed 0 should be present");
    let full_a45_high = report
        .records
        .iter()
        .find(|record| {
            record.resolved.entry_id == "terminal_guidance_clean_full_payload"
                && record.resolved.selector.arc_point == "a45"
                && record.resolved.selector.velocity_band == "high"
                && record.resolved.resolved_seed == 0
        })
        .expect("full payload a45 high seed 0 should be present");
    let full_a80_high_frontier = report
        .records
        .iter()
        .find(|record| {
            record.resolved.entry_id == "terminal_guidance_clean_full_payload"
                && record.resolved.selector.arc_point == "a80"
                && record.resolved.selector.velocity_band == "high"
                && record.resolved.resolved_seed == 3
        })
        .expect("full payload a80 high seed 3 should be present");

    assert_eq!(half_a45_high.analytic.class, BatchRunAnalyticClass::Scored);
    assert_eq!(
        full_a45_high.manifest.mission_outcome,
        MissionOutcome::FailedCrash
    );
    assert_eq!(
        full_a45_high.analytic.class,
        BatchRunAnalyticClass::Frontier
    );
    assert_eq!(
        full_a45_high.analytic.reason,
        Some(BatchRunAnalyticReason::LowThrustHighEnergy)
    );
    assert!(
        full_a45_high
            .analytic
            .stop_accel_margin_mps2
            .expect("reachability margin should be present")
            > 0.0
    );
    assert_eq!(
        full_a80_high_frontier.analytic.class,
        BatchRunAnalyticClass::Frontier
    );
    assert_eq!(
        full_a80_high_frontier.analytic.reason,
        Some(BatchRunAnalyticReason::LowThrustHighEnergy)
    );

    let impossible_runs = report
        .records
        .iter()
        .filter(|record| matches!(record.analytic.class, BatchRunAnalyticClass::Impossible))
        .count();
    let frontier_success_runs = report
        .records
        .iter()
        .filter(|record| {
            matches!(record.analytic.class, BatchRunAnalyticClass::Frontier)
                && matches!(record.manifest.mission_outcome, MissionOutcome::Success)
        })
        .count();
    let frontier_failure_runs = report
        .records
        .iter()
        .filter(|record| {
            matches!(record.analytic.class, BatchRunAnalyticClass::Frontier)
                && !matches!(record.manifest.mission_outcome, MissionOutcome::Success)
        })
        .count();

    assert!(frontier_failure_runs > 0);
    assert_eq!(report.summary.invalidated_runs, impossible_runs);
    assert!(report.summary.success_runs >= frontier_success_runs);
    assert!(report.summary.failure_runs >= frontier_failure_runs);
    assert_eq!(
        report.summary.success_runs + report.summary.failure_runs + report.summary.invalidated_runs,
        report.summary.total_runs
    );
}

#[test]
fn analytic_transfer_frontier_marks_only_near_vertical_uphill_routes() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "transfer_frontier_classification".to_owned(),
        name: "Transfer frontier classification".to_owned(),
        description: "transfer frontier classification".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![
            ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_route_angles_empty".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "empty".to_owned(),
                expectation_tier: "reference".to_owned(),
                route_angles: vec!["r-80".to_owned(), "r+60".to_owned(), "r+80".to_owned()],
                radius_tiers: Vec::new(),
                waypoint_profile: None,
                waypoint_handoff_envelope: None,
                evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
                adjustments: Vec::new(),
                tags: vec!["transfer".to_owned(), "analytic".to_owned()],
                metadata: BTreeMap::new(),
            }),
            ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_clean_empty".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "terminal_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "clean".to_owned(),
                vehicle_variant: "empty".to_owned(),
                expectation_tier: "reference".to_owned(),
                arc_points: vec!["a80".to_owned()],
                adjustments: Vec::new(),
                tags: vec!["terminal".to_owned(), "analytic".to_owned()],
                metadata: BTreeMap::new(),
            }),
        ],
    };

    let resolved = resolve_pack_runs(&pack, &base_dir).unwrap();
    let analytic_for = |mission: &str, route_or_arc: &str| {
        let run = resolved
            .iter()
            .find(|run| {
                run.descriptor.resolved_seed == 0
                    && run.descriptor.selector.mission == mission
                    && (run.descriptor.selector.route_angle == route_or_arc
                        || run.descriptor.selector.arc_point == route_or_arc)
            })
            .expect("resolved run should exist");
        analytic_feasibility_for_run(run)
    };

    let uphill_r80 = analytic_for("transfer_guidance", "r+80");
    assert_eq!(uphill_r80.class, BatchRunAnalyticClass::Frontier);
    assert_eq!(
        uphill_r80.reason,
        Some(BatchRunAnalyticReason::NearVerticalTransferRoute)
    );
    assert!(uphill_r80.is_scored());

    assert_eq!(
        analytic_for("transfer_guidance", "r+60").class,
        BatchRunAnalyticClass::Scored
    );
    assert_eq!(
        analytic_for("transfer_guidance", "r-80").class,
        BatchRunAnalyticClass::Scored
    );
    assert_eq!(
        analytic_for("terminal_guidance", "a80").reason,
        None,
        "terminal a80 should not inherit transfer-route frontier policy"
    );
}

#[test]
fn analytic_coupled_bound_does_not_invalidate_projected_error_successes() {
    let base_dir = fixtures_root();
    let pack = ScenarioPackSpec {
        id: "terminal_matrix_projected_error_coupled_bound".to_owned(),
        name: "Terminal matrix projected error coupled bound".to_owned(),
        description: "terminal matrix projected error coupled bound".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
            id: "terminal_guidance_traj_overshoot_large_half".to_owned(),
            terminal_matrix: "half_arc_terminal_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TerminalMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "terminal_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TerminalSeedTier::Smoke,
            condition_set: "traj_overshoot_large".to_owned(),
            vehicle_variant: "half".to_owned(),
            expectation_tier: "core".to_owned(),
            arc_points: Vec::new(),
            adjustments: vec![NumericAdjustmentSpec {
                id: "payload_half_mass_kg".to_owned(),
                path: "vehicle.dry_mass_kg".to_owned(),
                mode: NumericPerturbationMode::Offset,
                value: 2250.0,
            }],
            tags: vec!["terminal".to_owned(), "traj_error".to_owned()],
            metadata: BTreeMap::new(),
        })],
    };

    let report = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
    let record = report
        .records
        .iter()
        .find(|record| {
            record.resolved.selector.arc_point == "a70"
                && record.resolved.selector.velocity_band == "high"
                && record.resolved.resolved_seed == 0
        })
        .expect("projected-error a70 high seed 0 should be present");

    assert_eq!(record.analytic.class, BatchRunAnalyticClass::Scored);
    assert!(
        record
            .analytic
            .stop_accel_margin_mps2
            .expect("coupled stop margin should be present")
            > 0.0
    );
    let mission_success_runs = report
        .records
        .iter()
        .filter(|record| matches!(record.manifest.mission_outcome, MissionOutcome::Success))
        .count();
    let impossible_non_success_runs = report
        .records
        .iter()
        .filter(|record| {
            !matches!(record.manifest.mission_outcome, MissionOutcome::Success)
                && matches!(record.analytic.class, BatchRunAnalyticClass::Impossible)
        })
        .count();

    assert_eq!(report.summary.success_runs, mission_success_runs);
    assert_eq!(report.summary.invalidated_runs, impossible_non_success_runs);
}

#[test]
fn terminal_matrix_entry_rejects_overwritten_adjustment_paths() {
    for path in ["world.gravity_mps2", "initial_state.position_m.x"] {
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_invalid_adjustment".to_owned(),
            name: "Terminal matrix invalid adjustment".to_owned(),
            description: "terminal matrix invalid adjustment".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_clean_nominal".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "staged".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "clean".to_owned(),
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                arc_points: Vec::new(),
                adjustments: vec![NumericAdjustmentSpec {
                    id: "invalid".to_owned(),
                    path: path.to_owned(),
                    mode: NumericPerturbationMode::Offset,
                    value: 1.0,
                }],
                tags: Vec::new(),
                metadata: BTreeMap::new(),
            })],
        };

        let err = validate_pack(&pack).expect_err("path should be rejected");
        assert!(err.to_string().contains("uses unsupported path"), "{err}");
    }
}

#[test]
fn missing_compare_skip_allows_unresolved_cache_ref() {
    let base_dir = temp_fixture_root("missing_compare_skip");
    write_scenario(
        &base_dir,
        "scenarios/checkpoint_success.json",
        &easy_checkpoint_scenario(),
    );
    let pack = ScenarioPackSpec {
        id: "missing_compare_skip".to_owned(),
        name: "Missing compare skip".to_owned(),
        description: "missing compare skip".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
            id: "checkpoint_success_idle".to_owned(),
            scenario: "scenarios/checkpoint_success.json".to_owned(),
            controller: "idle".to_owned(),
            controller_config: None,
            metadata: BTreeMap::new(),
        })],
    };
    let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();
    let identity = batch_identity_for_pack(&pack, &resolved_runs).unwrap();
    let provenance = BatchCompareProvenance {
        source: BatchCompareSource::CacheRef,
        requested_ref: Some("auto".to_owned()),
        resolved_ref: None,
        baseline_dir: None,
        status: BatchCompareResolutionStatus::Missing,
        note: Some("no compare cache ref could be resolved".to_owned()),
    };

    let (resolved_provenance, baseline) =
        load_requested_baseline(&pack, &identity, provenance, MissingComparePolicy::Skip).unwrap();

    assert!(baseline.is_none());
    assert_eq!(
        resolved_provenance.status,
        BatchCompareResolutionStatus::Missing
    );
}

#[test]
fn cached_batch_validation_rejects_schema_mismatch() {
    let base_dir = temp_fixture_root("cache_schema");
    write_scenario(
        &base_dir,
        "scenarios/checkpoint_success.json",
        &easy_checkpoint_scenario(),
    );
    let output_dir = base_dir.join("cache_output");
    let pack = ScenarioPackSpec {
        id: "cache_schema".to_owned(),
        name: "Cache schema".to_owned(),
        description: "cache schema".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
            id: "checkpoint_success_idle".to_owned(),
            scenario: "scenarios/checkpoint_success.json".to_owned(),
            controller: "idle".to_owned(),
            controller_config: None,
            metadata: BTreeMap::new(),
        })],
    };

    let report = run_pack_with_workers(&pack, &base_dir, Some(&output_dir), 1).unwrap();
    report::write_batch_report_artifacts(&output_dir, &report, None).unwrap();
    write_json(
        &output_dir.join("meta.json"),
        &BatchCacheMeta {
            schema_version: BATCH_REPORT_SCHEMA_VERSION - 1,
            pack_id: report.pack_id.clone(),
            pack_name: report.pack_name.clone(),
            identity: report.identity.clone(),
            total_runs: report.total_runs,
            workers_used: report.workers_used,
            cache: BatchCacheInfo {
                workspace_key: "unit".to_owned(),
                commit_key: "unit".to_owned(),
                batch_stem: "cache_schema".to_owned(),
                cache_dir: output_dir.to_string_lossy().into_owned(),
                status: BatchCacheStatus::Fresh,
                created_at_unix_s: current_unix_timestamp(),
                promotion: None,
            },
        },
    )
    .unwrap();

    assert!(
        validate_cached_batch_dir(&output_dir, &pack, &report.identity)
            .unwrap()
            .is_none()
    );
}

#[test]
fn stable_output_runs_tree_is_refreshed_from_cache() {
    let base_dir = temp_fixture_root("stable_output_runs");
    write_scenario(
        &base_dir,
        "scenarios/checkpoint_success.json",
        &easy_checkpoint_scenario(),
    );
    let output_dir = base_dir.join("stable_output");
    fs::create_dir_all(output_dir.join("runs").join("stale_before"))
        .expect("stale runs dir should be creatable");
    let pack = ScenarioPackSpec {
        id: "stable_output_runs".to_owned(),
        name: "Stable output runs".to_owned(),
        description: "stable output runs".to_owned(),
        terminal_matrix_max_time_s: None,
        entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
            id: "checkpoint_success_idle".to_owned(),
            scenario: "scenarios/checkpoint_success.json".to_owned(),
            controller: "idle".to_owned(),
            controller_config: None,
            metadata: BTreeMap::new(),
        })],
    };

    let outcome = run_pack_cached(
        &pack,
        &base_dir,
        Some(&output_dir),
        1,
        Some("none"),
        None,
        MissingComparePolicy::Skip,
        false,
    )
    .unwrap();
    let written = load_batch_report(&output_dir).unwrap();
    let expected_run_dir = output_dir.join("runs").join("checkpoint_success_idle");

    assert!(!output_dir.join("runs").join("stale_before").exists());
    assert!(expected_run_dir.exists());
    assert_eq!(
        written.records[0].bundle_dir.as_deref(),
        Some(expected_run_dir.to_string_lossy().as_ref())
    );
    assert_eq!(
        outcome.report.records[0].bundle_dir,
        Some(expected_run_dir.to_string_lossy().into_owned())
    );
}
