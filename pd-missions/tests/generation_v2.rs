use pd_core::{TerrainDefinition, Vec2};
use pd_missions::*;

fn resolved(recipe: &MissionRecipeV2, index: u64) -> ResolvedMissionV2 {
    generate_mission_v2(recipe, &GeometrySpecV1::default(), 20261011000, index)
        .unwrap()
        .mission
        .unwrap()
}

#[test]
fn both_v2_backends_retain_pinned_default_recipe_and_physical_identities() {
    for (recipe, recipe_id, physical_id) in [
        (
            MissionRecipeV2::default(),
            "f61a819d7b83d56fb9accf24e21f7958924648c10cb8e7d4ce5b50f6e3fbf8d9",
            "aa16e776c9450f0f6fc2f7ebe0f4269c20f3c5ace461d0180eae12c922c7d01c",
        ),
        (
            MissionRecipeV2::value_noise(),
            "bc915932b0c02c720ec1775e4e9326510d851d55cf13cd7ab66e1f809c3a3e54",
            "30f96d996ee2852c25f9af9ab3b0b23af1e020035b0f7b36f3da47a30720265e",
        ),
    ] {
        let mission = generate_mission_v2(&recipe, &GeometrySpecV1::default(), 987, 7)
            .unwrap()
            .mission
            .unwrap();
        assert_eq!(mission.recipe_sha256, recipe_id);
        assert_eq!(mission.physical_sha256, physical_id);
    }
}

#[test]
fn configurable_generation_is_deterministic_in_reverse_and_parallel_order() {
    let recipe = MissionRecipeV2::default();
    let expected: Vec<_> = (0..16)
        .map(|i| generate_mission_v2(&recipe, &GeometrySpecV1::default(), 1234, i).unwrap())
        .collect();
    for i in (0..16).rev() {
        let actual = generate_mission_v2(&recipe, &GeometrySpecV1::default(), 1234, i).unwrap();
        assert_eq!(actual, expected[i as usize]);
        let mission = actual.mission.unwrap();
        mission.validate().unwrap();
        assert_eq!(mission.direction, if i % 2 == 0 { 1 } else { -1 });
        assert!(mission.scenario.mission.transfer_route.is_none());
        assert!(!mission.scenario.metadata.contains_key("stratum"));
    }
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..16)
            .rev()
            .map(|i| {
                let recipe = &recipe;
                scope.spawn(move || {
                    (
                        i,
                        generate_mission_v2(recipe, &GeometrySpecV1::default(), 1234, i).unwrap(),
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    for (i, actual) in results {
        assert_eq!(actual, expected[i as usize]);
    }
}

#[test]
fn backends_have_explicit_identities_and_distinct_physical_inputs() {
    let layered = MissionRecipeV2::default();
    let value = MissionRecipeV2::value_noise();
    assert_ne!(layered.identity().unwrap(), value.identity().unwrap());
    for i in 0..8 {
        let a = resolved(&layered, i);
        let b = resolved(&value, i);
        assert_eq!(a.terrain_parameters, b.terrain_parameters);
        assert_eq!(a.proposal, b.proposal);
        assert_ne!(a.physical_sha256, b.physical_sha256);
        a.validate().unwrap();
        b.validate().unwrap();
    }
}

#[test]
fn pad_edits_preserve_all_outside_vertices_and_entire_outside_segments() {
    for i in 0..12 {
        let mission = resolved(&MissionRecipeV2::default(), i);
        let raw = &mission.raw_terrain;
        let prepared = &mission.scenario.world.terrain;
        let edits = &mission.geometry.pad_edit_intervals_x_m;
        assert_eq!(edits.len(), 2);
        for point in raw.points() {
            if !edits
                .iter()
                .any(|e| point.x >= e.start_m && point.x <= e.end_m)
            {
                assert!(prepared.points().contains(point));
            }
        }
        for pair in prepared.points().windows(2) {
            let x = (pair[0].x + pair[1].x) * 0.5;
            if !edits.iter().any(|e| x > e.start_m && x < e.end_m) {
                assert!((raw.sample_height(x) - prepared.sample_height(x)).abs() < 1e-8);
            }
        }
        for edit in edits {
            assert_eq!(
                prepared.sample_height(edit.start_m),
                raw.sample_height(edit.start_m)
            );
            assert_eq!(
                prepared.sample_height(edit.end_m),
                raw.sample_height(edit.end_m)
            );
        }
        for pad in &mission.scenario.world.landing_pads {
            assert!((prepared.sample_height(pad.center_x_m) - pad.surface_y_m).abs() < 1e-8);
            for offset in [-pad.half_width_m(), 0.0, pad.half_width_m()] {
                assert!(
                    (prepared.sample_height(pad.center_x_m + offset) - pad.surface_y_m).abs()
                        < 1e-8
                );
            }
        }
    }
}

#[test]
fn preparation_has_local_linear_transitions_and_handles_grid_adjacent_edges() {
    let raw = TerrainDefinition::Heightfield {
        points_m: (-100..=100)
            .map(|x| Vec2::new(x as f64, 2.0 * x as f64))
            .collect(),
    };
    let pad = pd_core::LandingPadSpec {
        id: "source".into(),
        center_x_m: 0.0000001,
        surface_y_m: 0.0,
        width_m: 20.0,
    };
    let (prepared, edits) = prepare_pads_v2(&raw, &[pad], 24.0).unwrap();
    assert_eq!(edits[0].start_m, -34.0);
    assert_eq!(edits[0].end_m, 34.0);
    assert_eq!(prepared.sample_height(0.0), 0.0);
    let halfway = (edits[0].end_m + 10.0000001) * 0.5;
    assert!(
        (prepared.sample_height(halfway) - raw.sample_height(edits[0].end_m) * 0.5).abs() < 1e-8
    );
    for point in raw.points() {
        if point.x.abs() > 34.0 {
            assert!(prepared.points().contains(point));
        }
    }
}

#[test]
fn finite_overlap_rejections_preserve_every_structural_attempt() {
    let recipe = MissionRecipeV2 {
        travel_distance_m: RangeV1 { min: 1.0, max: 2.0 },
        ..MissionRecipeV2::default()
    };
    let result = generate_mission_v2(&recipe, &GeometrySpecV1::default(), 1234, 0).unwrap();
    assert!(result.mission.is_none());
    assert_eq!(result.rejections.len(), recipe.max_attempts as usize);
    for (i, rejection) in result.rejections.iter().enumerate() {
        assert_eq!(rejection.proposal.attempt as usize, i);
        assert_eq!(rejection.reason, StructuralRejectionV2::OverlappingPadEdits);
    }
}

#[test]
fn structural_generation_does_not_hide_mild_proposals_or_depend_on_metric_settings() {
    let mut flat = MissionRecipeV2::default();
    let zero = RangeV1 { min: 0.0, max: 0.0 };
    flat.terrain.macro_amplitude_m = zero;
    flat.terrain.structure_amplitude_m = zero;
    flat.terrain.feature_amplitude_m = zero;
    let mission = resolved(&flat, 0);
    assert_eq!(mission.geometry.geometry.relief_m, 0.0);
    mission.validate().unwrap();
    let recipe = MissionRecipeV2::default();
    let a = resolved(&recipe, 0);
    let spec = GeometrySpecV1 {
        endpoint_radius_m: 100.0,
        ..GeometrySpecV1::default()
    };
    let b = generate_mission_v2(&recipe, &spec, a.proposal.seed, a.proposal.index)
        .unwrap()
        .mission
        .unwrap();
    assert_eq!(a.scenario, b.scenario);
    assert_eq!(a.physical_sha256, b.physical_sha256);
    assert_ne!(a.geometry.spec_sha256, b.geometry.spec_sha256);
}

#[test]
fn resolved_readers_reject_changed_geometry_raw_preparation_and_physical_tiers() {
    let original = resolved(&MissionRecipeV2::default(), 2);
    let round_trip: ResolvedMissionV2 =
        serde_json::from_slice(&json_bytes(&original).unwrap()).unwrap();
    round_trip.validate().unwrap();
    assert_eq!(round_trip, original);
    let mut changed = original.clone();
    changed.raw_geometry.relief_m += 1.0;
    assert!(changed.validate().is_err());
    changed = original.clone();
    changed.geometry.geometry.distance_m += 1.0;
    assert!(changed.validate().is_err());
    changed = original.clone();
    changed.terrain_parameters.ridge_mix = 100.0;
    assert!(changed.validate().is_err());
    changed = original.clone();
    changed.scenario.vehicle.initial_fuel_kg -= 1.0;
    changed.physical_sha256 = physical_identity(&changed.scenario).unwrap();
    assert!(changed.validate().is_err());
    changed = original.clone();
    let TerrainDefinition::Heightfield { points_m } = &mut changed.raw_terrain;
    points_m[0].y += 10.0;
    changed.raw_terrain_sha256 = sha256(&serde_json::to_vec(&changed.raw_terrain).unwrap());
    assert!(
        changed
            .validate()
            .unwrap_err()
            .contains("bounded raw pad edits")
    );
    changed = original.clone();
    changed.raw_terrain = TerrainDefinition::Heightfield { points_m: vec![] };
    assert!(changed.validate().is_err());
    let mut json = serde_json::to_value(original).unwrap();
    json["terrain_stratum"] = serde_json::json!(3);
    assert!(serde_json::from_value::<ResolvedMissionV2>(json).is_err());
}

#[test]
fn finer_reference_is_explicit_and_retains_the_sealed_physical_input() {
    let mission = resolved(&MissionRecipeV2::default(), 0);
    let bytes = json_bytes(&mission).unwrap();
    let same = mission.raw_reference(4.0).unwrap();
    assert_eq!(same, mission.raw_terrain);
    let fine = mission.raw_reference(1.0).unwrap();
    for point in mission.raw_terrain.points() {
        assert_eq!(fine.sample_height(point.x), point.y);
    }
    assert_eq!(json_bytes(&mission).unwrap(), bytes);
    assert!(mission.raw_reference(0.1).is_err());
    assert!(mission.raw_reference(8.0).is_err());
}

#[test]
fn recipes_fail_before_generation_for_invalid_controls_and_resolution() {
    let recipe = MissionRecipeV2::default();
    recipe.validate().unwrap();
    for mode in 0..5 {
        let mut bad = recipe.clone();
        match mode {
            0 => bad.terrain.ridge_mix.max = 2.0,
            1 => bad.terrain.warp_amplitude_m.min = f64::NAN,
            2 => {
                bad.octaves = 4;
                bad.terrain_spacing_m = 16.0;
            }
            3 => bad.schema_version = 1,
            _ => bad.max_attempts = 0,
        }
        assert!(generate_mission_v2(&bad, &GeometrySpecV1::default(), 0, 0).is_err());
    }
    let mut unknown = serde_json::to_value(recipe).unwrap();
    unknown["family"] = serde_json::json!("mountain");
    assert!(serde_json::from_value::<MissionRecipeV2>(unknown).is_err());
}
