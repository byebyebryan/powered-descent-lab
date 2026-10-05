use super::*;
use pd_core::ScenarioSpec;
use std::{fs, path::Path};

fn fixture_context() -> RunContext {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let manifest: Value = serde_json::from_slice(
        &fs::read(
            root.join("fixtures/research/waypoint_direct_body_aware_terminal_fresh_inputs_v1.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let scenario: ScenarioSpec =
        serde_json::from_value(manifest["cases"][0]["scenario"].clone()).unwrap();
    RunContext::from_scenario(&scenario).unwrap()
}

#[test]
fn valid_baselines_are_identical_and_cached_acquisition_matches_the_original_solver() {
    let context = fixture_context();
    let mut live = SimulationState::new(&context).unwrap();
    live.position_m.y =
        context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m + 700.0;
    live.velocity_mps = Vec2::new(24.0, -24.0);
    let mut retained = 0;
    let mut cache_checks = 0;
    for spec in generate_seed_specs(&context, &live).unwrap() {
        let old = evaluate_seed(&context, &live, 9600, spec.clone()).unwrap();
        let (new, traces) = evaluate_timed_seed(&context, &live, 9600, spec.clone()).unwrap();
        assert_eq!(old.entry_screens.len(), new.entry_screens.len());
        if !old.entry_screens.is_empty() {
            let cached = plan_from_seed(&old, &live).unwrap();
            let original = make_acquisition_plan(&context, &live, &spec).unwrap();
            assert_eq!(
                cached.thrust_acceleration_mps2,
                original.thrust_acceleration_mps2
            );
            assert_eq!(cached.turn_ticks, original.turn_ticks);
            assert_eq!(cached.burn_ticks, original.burn_ticks);
            assert_eq!(cached.predicted_end, original.predicted_end);
            assert_eq!(
                cached.virtual_target_error_m,
                original.virtual_target_error_m
            );
            assert_eq!(cached.estimated_fuel_kg, original.estimated_fuel_kg);
            assert_eq!(cached.upward_impulse_mps, original.upward_impulse_mps);
            assert_eq!(cached.turn_updates, original.turn_updates);
            assert_eq!(cached.target_attitude_rad, original.target_attitude_rad);
            cache_checks += 1;
        }
        for ((before, after), trace) in old
            .entry_screens
            .iter()
            .zip(&new.entry_screens)
            .zip(&traces)
        {
            assert!(trace["candidates"].as_array().unwrap().len() <= 3);
            assert_eq!(trace["screens"][0], serde_json::to_value(before).unwrap());
            if before.admissible {
                assert_eq!(before, after);
                assert_eq!(trace["selected_candidate_index"], 0);
                assert_eq!(trace["candidates"].as_array().unwrap().len(), 1);
                // The alternative screener uses precisely the existing predicates.
                let rebuilt = screen_duration(
                    &context,
                    &live,
                    9600,
                    &plan_from_seed(&old, &live).unwrap(),
                    before,
                    before.terminal_physics_ticks.unwrap(),
                )
                .unwrap();
                assert!(rebuilt.admissible);
                assert_eq!(
                    rebuilt.coupled_thrust_bound_mps2,
                    before.coupled_thrust_bound_mps2
                );
                assert_eq!(rebuilt.terminal_fuel_kg, before.terminal_fuel_kg);
                assert_eq!(
                    rebuilt.predicted_finish_physics_step,
                    before.predicted_finish_physics_step
                );
                retained += 1;
            }
        }
    }
    assert!(retained > 0);
    assert!(cache_checks > 0);
}

#[test]
fn time_independent_geometry_rejection_does_not_generate_alternatives() {
    let context = fixture_context();
    let mut live = SimulationState::new(&context).unwrap();
    live.velocity_mps = Vec2::new(24.0, 20.0);
    let spec = SeedSpec {
        seed_id: "test_zero".into(),
        kind: "zero_acquisition".into(),
        virtual_arrival_ticks: 100,
        burn_fraction: 0.0,
        natural_profile: true,
        zero_acquisition: true,
    };
    let (_, traces) = evaluate_timed_seed(&context, &live, 9600, spec).unwrap();
    assert!(!traces.is_empty());
    for trace in traces {
        assert_eq!(trace["candidates"].as_array().unwrap().len(), 1);
        assert!(trace["selected_candidate_index"].is_null());
        assert!(
            trace["screens"][0]["reason"]
                .as_str()
                .unwrap()
                .contains("not descending")
        );
    }
}

#[test]
fn nominal_duration_choice_is_blind_to_interior_terrain() {
    let context = fixture_context();
    let mut live = SimulationState::new(&context).unwrap();
    live.position_m.y =
        context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m + 700.0;
    live.velocity_mps = Vec2::new(24.0, -24.0);
    let mut changed = context.clone();
    let pd_core::terrain::TerrainDefinition::Heightfield { points_m } = &mut changed.world.terrain;
    for point in points_m {
        point.y += 10_000.0;
    }
    let specs = generate_seed_specs(&context, &live).unwrap();
    assert_eq!(specs, generate_seed_specs(&changed, &live).unwrap());
    for spec in specs {
        let before = evaluate_timed_seed(&context, &live, 9600, spec.clone()).unwrap();
        let after = evaluate_timed_seed(&changed, &live, 9600, spec).unwrap();
        assert_eq!(before, after);
    }
}
