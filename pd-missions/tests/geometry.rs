use pd_core::{LandingPadSpec, TerrainDefinition, Vec2};
use pd_missions::*;

fn terrain(points: &[(f64, f64)]) -> TerrainDefinition {
    TerrainDefinition::Heightfield {
        points_m: points.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
    }
}

fn pad(id: &str, x: f64, y: f64) -> LandingPadSpec {
    LandingPadSpec {
        id: id.into(),
        center_x_m: x,
        surface_y_m: y,
        width_m: 20.0,
    }
}

fn measure(points: &[(f64, f64)]) -> TransferGeometryV1 {
    measure_transfer_geometry(
        &terrain(points),
        &pad("source", 0.0, 0.0),
        &pad("target", 1000.0, 0.0),
        &GeometrySpecV1::default(),
        &[],
    )
    .unwrap()
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-7, "{a} != {b}");
}

fn geometry_close(a: &TransferGeometryV1, b: &TransferGeometryV1) {
    fn compare(a: &serde_json::Value, b: &serde_json::Value) {
        match (a, b) {
            (serde_json::Value::Number(x), serde_json::Value::Number(y)) => {
                close(x.as_f64().unwrap(), y.as_f64().unwrap())
            }
            (serde_json::Value::Array(x), serde_json::Value::Array(y)) => {
                assert_eq!(x.len(), y.len());
                for (a, b) in x.iter().zip(y) {
                    compare(a, b);
                }
            }
            (serde_json::Value::Object(x), serde_json::Value::Object(y)) => {
                assert_eq!(x.len(), y.len());
                for (key, value) in x {
                    compare(value, &y[key]);
                }
            }
            _ => assert_eq!(a, b),
        }
    }
    compare(
        &serde_json::to_value(a).unwrap(),
        &serde_json::to_value(b).unwrap(),
    );
}

#[test]
fn triangular_obstacle_has_exact_width_prominence_and_position() {
    let value = measure(&[(0.0, 0.0), (500.0, 200.0), (1000.0, 0.0)]);
    assert_eq!(value.relief_m, 200.0);
    assert_eq!(value.max_chord_barrier_m, 200.0);
    assert_eq!(value.barriers[1].longest_width_m, 500.0);
    assert_eq!(value.barriers[1].departure_distance_m, Some(250.0));
    assert_eq!(value.barriers[1].approach_distance_m, Some(250.0));
    assert_eq!(value.crests.len(), 1);
    let crest = &value.crests[0];
    assert_eq!(crest.position_m, 500.0);
    assert_eq!(crest.position_fraction, 0.5);
    assert_eq!(crest.prominence_m, 200.0);
    assert_eq!(crest.half_prominence_support.width_m(), 500.0);
    assert_eq!(value.min_crest_spacing_m, None);
    assert_eq!(value.slopes.p95_abs, 0.4);
}

#[test]
fn uphill_and_downhill_demands_are_preserved_without_a_chord_barrier() {
    for rise in [-400.0, 400.0] {
        let value = measure_transfer_geometry(
            &terrain(&[
                (-100.0, -rise / 10.0),
                (500.0, rise / 2.0),
                (1100.0, rise * 1.1),
            ]),
            &pad("source", 0.0, 0.0),
            &pad("target", 1000.0, rise),
            &GeometrySpecV1::default(),
            &[],
        )
        .unwrap();
        close(value.signed_pad_rise_m, rise);
        close(value.relief_m, 400.0);
        close(value.max_chord_barrier_m, 0.0);
        assert!(value.crests.is_empty());
        assert!(value.barriers.iter().all(|v| v.intervals.is_empty()));
        close(value.slopes.p95_abs, 0.4);
    }
}

#[test]
fn equal_height_separated_ridges_keep_intervals_positions_and_spacing() {
    let value = measure(&[
        (0.0, 0.0),
        (200.0, 200.0),
        (400.0, 0.0),
        (600.0, 200.0),
        (800.0, 0.0),
        (1000.0, 0.0),
    ]);
    assert_eq!(value.barriers[1].intervals.len(), 2);
    assert_eq!(value.barriers[1].longest_width_m, 200.0);
    assert_eq!(value.barriers[1].total_width_m, 400.0);
    assert_eq!(value.crests.len(), 2);
    assert_eq!(value.crests[0].position_m, 200.0);
    assert_eq!(value.crests[1].position_m, 600.0);
    assert_eq!(value.min_crest_spacing_m, Some(400.0));
    assert_eq!(value.max_crest_spacing_m, Some(400.0));
}

#[test]
fn identical_aggregate_shapes_retain_different_departure_and_approach_demands() {
    let departure = measure(&[
        (0.0, 0.0),
        (40.0, 0.0),
        (100.0, 200.0),
        (160.0, 0.0),
        (1000.0, 0.0),
    ]);
    let approach = measure(&[
        (0.0, 0.0),
        (840.0, 0.0),
        (900.0, 200.0),
        (960.0, 0.0),
        (1000.0, 0.0),
    ]);
    assert_eq!(departure.relief_m, approach.relief_m);
    assert_eq!(departure.max_chord_barrier_m, approach.max_chord_barrier_m);
    assert_eq!(departure.crests.len(), approach.crests.len());
    assert_eq!(
        departure.barriers[1].longest_width_m,
        approach.barriers[1].longest_width_m
    );
    assert_eq!(departure.barriers[1].departure_distance_m, Some(70.0));
    assert_eq!(approach.barriers[1].departure_distance_m, Some(870.0));
    assert_eq!(departure.barriers[1].approach_distance_m, Some(870.0));
    assert_eq!(approach.barriers[1].approach_distance_m, Some(70.0));
    assert_eq!(departure.source_inward.max_rise_above_pad_m, 200.0);
    assert_eq!(approach.source_inward.max_rise_above_pad_m, 0.0);
    assert_eq!(approach.target_inward.max_rise_above_pad_m, 200.0);
}

#[test]
fn plateau_centers_and_threshold_equality_are_defined() {
    let value = measure(&[
        (0.0, 0.0),
        (100.0, 100.0),
        (300.0, 100.0),
        (400.0, 0.0),
        (1000.0, 0.0),
    ]);
    assert_eq!(value.barriers[1].longest_width_m, 200.0);
    assert_eq!(value.crests.len(), 1);
    assert_eq!(value.crests[0].position_m, 200.0);
    assert_eq!(value.crests[0].prominence_m, 100.0);
    assert_eq!(value.crests[0].half_prominence_support.width_m(), 300.0);
    let flat = measure(&[(0.0, 0.0), (1000.0, 0.0)]);
    assert!(flat.crests.is_empty());
    assert!(
        flat.barriers
            .iter()
            .all(|b| b.departure_distance_m.is_none())
    );
    assert_eq!(flat.slopes.max_abs, 0.0);
}

#[test]
fn subsidiary_crest_prominence_uses_the_higher_neighbor_saddle() {
    let value = measure(&[
        (0.0, 0.0),
        (100.0, 200.0),
        (200.0, 100.0),
        (300.0, 150.0),
        (400.0, 0.0),
        (1000.0, 0.0),
    ]);
    assert_eq!(value.crests.len(), 2);
    assert_eq!(value.crests[0].prominence_m, 200.0);
    assert_eq!(value.crests[1].prominence_m, 50.0);
    close(value.crests[1].half_prominence_support.start_m, 250.0);
    close(
        value.crests[1].half_prominence_support.end_m,
        316.6666666666667,
    );
}

#[test]
fn tolerance_adjacent_peak_heights_do_not_erase_a_greater_neighbor() {
    // Approximate equality is not transitive: A~B and B~C do not imply A~C.
    // A must still bound C's prominence even when B is within tolerance of C.
    let value = measure(&[
        (0.0, 0.0),
        (100.0, 200.0 + 1.6e-8),
        (200.0, 100.0),
        (300.0, 200.0 + 0.8e-8),
        (400.0, 50.0),
        (500.0, 200.0),
        (600.0, 0.0),
        (1000.0, 0.0),
    ]);
    assert_eq!(value.crests.len(), 3);
    close(value.crests[2].prominence_m, 150.0);
}

#[test]
fn crest_separation_prefers_prominence_with_declared_tie_order() {
    let spec = GeometrySpecV1 {
        crest_min_separation_m: 150.0,
        ..GeometrySpecV1::default()
    };
    let value = measure_transfer_geometry(
        &terrain(&[
            (0.0, 0.0),
            (100.0, 200.0),
            (150.0, 0.0),
            (200.0, 150.0),
            (300.0, 0.0),
            (1000.0, 0.0),
        ]),
        &pad("source", 0.0, 0.0),
        &pad("target", 1000.0, 0.0),
        &spec,
        &[],
    )
    .unwrap();
    assert_eq!(value.crests.len(), 1);
    assert_eq!(value.crests[0].position_m, 100.0);
}

#[test]
fn clipping_interpolates_endpoints_and_ignores_external_relief() {
    let value = measure(&[
        (-100.0, -40.0),
        (500.0, 200.0),
        (1100.0, -40.0),
        (2000.0, 5000.0),
    ]);
    assert_eq!(value.relief_m, 200.0);
    assert_eq!(value.signed_pad_rise_m, 0.0);
    assert_eq!(value.max_chord_barrier_m, 200.0);
    assert_eq!(value.source_outward.measured_length_m, 100.0);
    close(value.source_outward.max_fall_below_pad_m, 40.0);
    assert_eq!(value.target_outward.measured_length_m, 200.0);
    assert!(value.target_outward.max_rise_above_pad_m > 500.0);
}

#[test]
fn slope_quantiles_weight_horizontal_distance_and_survive_subdivision() {
    let coarse = measure(&[(0.0, 0.0), (960.0, 0.0), (1000.0, 400.0)]);
    assert_eq!(coarse.slopes.p95_abs, 0.0);
    let mut fine = vec![(0.0, 0.0), (960.0, 0.0)];
    fine.extend((961..=1000).map(|x| (x as f64, (x - 960) as f64 * 10.0)));
    geometry_close(&coarse, &measure(&fine));
    let triangle = measure(&[
        (-200.0, 0.0),
        (0.0, 0.0),
        (500.0, 200.0),
        (1000.0, 0.0),
        (1200.0, 0.0),
    ]);
    let subdivided = measure(&[
        (-200.0, 0.0),
        (0.0, 0.0),
        (250.0, 100.0),
        (500.0, 200.0),
        (750.0, 100.0),
        (1000.0, 0.0),
        (1200.0, 0.0),
    ]);
    geometry_close(&triangle, &subdivided);
}

#[test]
fn exclusion_splits_segments_at_edit_boundaries_and_excludes_crest_support() {
    let points = terrain(&[
        (0.0, 0.0),
        (100.0, 0.0),
        (101.0, 100.0),
        (200.0, 100.0),
        (300.0, 0.0),
        (1000.0, 0.0),
    ]);
    let edits = [GeometryIntervalV1 {
        start_m: 99.5,
        end_m: 101.5,
    }];
    let value = measure_transfer_geometry(
        &points,
        &pad("source", 0.0, 0.0),
        &pad("target", 1000.0, 0.0),
        &GeometrySpecV1::default(),
        &edits,
    )
    .unwrap();
    assert_eq!(value.slopes.max_abs, 100.0);
    let natural = value.slopes_outside_pad_edits.unwrap();
    assert_eq!(natural.max_abs, 1.0);
    assert_eq!(natural.measured_length_m, 998.0);
    assert!(value.crests.is_empty());
    let all = [GeometryIntervalV1 {
        start_m: 0.0,
        end_m: 1000.0,
    }];
    let value = measure_transfer_geometry(
        &points,
        &pad("source", 0.0, 0.0),
        &pad("target", 1000.0, 0.0),
        &GeometrySpecV1::default(),
        &all,
    )
    .unwrap();
    assert!(value.slopes_outside_pad_edits.is_none());
}

#[test]
fn reflection_and_translation_preserve_source_relative_geometry_and_pad_edits() {
    let original = terrain(&[
        (-200.0, -20.0),
        (0.0, 0.0),
        (100.0, 150.0),
        (300.0, 0.0),
        (650.0, 250.0),
        (1000.0, 100.0),
        (1200.0, 120.0),
    ]);
    let source = pad("source", 0.0, 0.0);
    let target = pad("target", 1000.0, 100.0);
    let edits = vec![
        GeometryIntervalV1 {
            start_m: -40.0,
            end_m: 40.0,
        },
        GeometryIntervalV1 {
            start_m: 960.0,
            end_m: 1040.0,
        },
    ];
    let spec = GeometrySpecV1::default();
    let expected = measure_transfer_geometry(&original, &source, &target, &spec, &edits).unwrap();
    for reflection in [1.0, -1.0] {
        let mut points: Vec<_> = original
            .points()
            .iter()
            .map(|p| ((p.x * reflection) + 71.0, p.y - 250.0))
            .collect();
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut transformed_edits: Vec<_> = edits
            .iter()
            .map(|e| {
                let a = e.start_m * reflection + 71.0;
                let b = e.end_m * reflection + 71.0;
                GeometryIntervalV1 {
                    start_m: a.min(b),
                    end_m: a.max(b),
                }
            })
            .collect();
        transformed_edits.sort_by(|a, b| a.start_m.total_cmp(&b.start_m));
        let value = measure_transfer_geometry(
            &terrain(&points),
            &pad("source", 71.0, -250.0),
            &pad("target", 1000.0 * reflection + 71.0, -150.0),
            &spec,
            &transformed_edits,
        )
        .unwrap();
        geometry_close(&expected, &value);
    }
}

#[test]
fn receipts_bind_physics_and_metric_definition_without_membership() {
    let mission = generate_mission(&MissionRecipeV1::default(), 987, 7)
        .unwrap()
        .mission
        .unwrap();
    let spec = GeometrySpecV1::default();
    let receipt = measure_mission_geometry(&mission.scenario, &spec, &[]).unwrap();
    assert_eq!(receipt.physical_sha256, mission.physical_sha256);
    assert_eq!(receipt.spec_sha256, spec.identity().unwrap());
    receipt.verify(&mission.scenario).unwrap();
    let round_trip: MissionGeometryReceiptV1 =
        serde_json::from_slice(&json_bytes(&receipt).unwrap()).unwrap();
    round_trip.verify(&mission.scenario).unwrap();
    let mut altered = receipt.clone();
    altered.geometry.max_chord_barrier_m += 1.0;
    assert!(altered.verify(&mission.scenario).is_err());
    let mut scenario = mission.scenario.clone();
    scenario.tags.push("different-review-label".into());
    scenario.seed += 1;
    assert_eq!(
        measure_mission_geometry(&scenario, &spec, &[]).unwrap(),
        receipt
    );
    scenario.vehicle.initial_fuel_kg -= 1.0;
    assert_ne!(
        measure_mission_geometry(&scenario, &spec, &[])
            .unwrap()
            .physical_sha256,
        receipt.physical_sha256
    );
    let different_spec = GeometrySpecV1 {
        endpoint_radius_m: 100.0,
        ..spec.clone()
    };
    assert_ne!(different_spec.identity().unwrap(), receipt.spec_sha256);
    scenario.world.landing_pads[0].surface_y_m += 1.0;
    assert!(measure_mission_geometry(&scenario, &spec, &[]).is_err());
}

#[test]
fn invalid_specification_domain_edits_and_unbounded_geometry_fail_explicitly() {
    let points = terrain(&[(0.0, 0.0), (1000.0, 0.0)]);
    let source = pad("source", 0.0, 0.0);
    let target = pad("target", 1000.0, 0.0);
    let spec = GeometrySpecV1::default();
    for bad in [
        GeometrySpecV1 {
            crest_min_separation_m: 0.0,
            ..spec.clone()
        },
        GeometrySpecV1 {
            barrier_heights_m: vec![100.0, 50.0],
            ..spec.clone()
        },
        GeometrySpecV1 {
            endpoint_radius_m: f64::NAN,
            ..spec.clone()
        },
        GeometrySpecV1 {
            schema_version: 2,
            ..spec.clone()
        },
    ] {
        assert!(measure_transfer_geometry(&points, &source, &target, &bad, &[]).is_err());
    }
    for edits in [
        vec![GeometryIntervalV1 {
            start_m: -1.0,
            end_m: 40.0,
        }],
        vec![GeometryIntervalV1 {
            start_m: 100.0,
            end_m: 50.0,
        }],
        vec![
            GeometryIntervalV1 {
                start_m: 10.0,
                end_m: 50.0,
            },
            GeometryIntervalV1 {
                start_m: 40.0,
                end_m: 60.0,
            },
        ],
    ] {
        assert!(measure_transfer_geometry(&points, &source, &target, &spec, &edits).is_err());
    }
    assert!(
        measure_transfer_geometry(&points, &pad("source", -1.0, 0.0), &target, &spec, &[]).is_err()
    );
    let mut unknown = serde_json::to_value(&spec).unwrap();
    unknown["difficulty"] = serde_json::json!("easy");
    assert!(serde_json::from_value::<GeometrySpecV1>(unknown).is_err());
    assert!(
        measure_transfer_geometry(
            &terrain(&[(0.0, 0.0), (0.0, 1.0)]),
            &source,
            &target,
            &spec,
            &[]
        )
        .is_err()
    );
    assert!(
        measure_transfer_geometry(
            &terrain(&[(0.0, 0.0), (1000.0, f64::INFINITY)]),
            &source,
            &target,
            &spec,
            &[]
        )
        .is_err()
    );
    assert!(
        measure_transfer_geometry(
            &terrain(&[(0.0, 0.0), (1000.0, 20_000_000.0)]),
            &source,
            &target,
            &spec,
            &[]
        )
        .is_err()
    );
}

#[test]
fn many_equal_peaks_keep_bounded_queries_and_do_not_collapse_to_one_crest() {
    let points: Vec<_> = (0..=20_000)
        .map(|x| (x as f64, if x % 2 == 0 { 0.0 } else { 100.0 }))
        .collect();
    let spec = GeometrySpecV1 {
        crest_min_half_prominence_width_m: 0.5,
        crest_min_separation_m: 0.8,
        ..GeometrySpecV1::default()
    };
    let value = measure_transfer_geometry(
        &terrain(&points),
        &pad("source", 0.0, 0.0),
        &pad("target", 20_000.0, 0.0),
        &spec,
        &[],
    )
    .unwrap();
    assert_eq!(value.crests.len(), 10_000);
    assert_eq!(value.barriers[0].intervals.len(), 10_000);
    assert_eq!(value.min_crest_spacing_m, Some(2.0));
}
