use pd_benchmark::{IncumbentAdapterV1, reflect_scenario};
use pd_eval::{AttemptExecutionErrorV1, EvidenceScope};
use pd_missions::{MissionRecipeV1, generate_mission, physical_identity};

#[test]
fn reflection_is_global_involution_and_preserves_physical_budgets() {
    for index in 0..8 {
        let scenario = generate_mission(&MissionRecipeV1::layered(), 811, index)
            .unwrap()
            .mission
            .unwrap()
            .scenario;
        let reflected = reflect_scenario(&scenario);
        reflected.validate().unwrap();
        assert_eq!(scenario, reflect_scenario(&reflected));
        assert_eq!(scenario.sim, reflected.sim);
        assert_eq!(scenario.vehicle, reflected.vehicle);
        for x in [-700.25, 0., 701.5] {
            assert!(
                (scenario.world.terrain.sample_height(x)
                    - reflected.world.terrain.sample_height(-x))
                .abs()
                    < 1e-9
            );
        }
    }
}

#[test]
fn raw_incumbent_rejects_leftward_input_without_reclassifying_it() {
    let scenario = generate_mission(&MissionRecipeV1::default(), 812, 4)
        .unwrap()
        .mission
        .unwrap()
        .scenario;
    let result = IncumbentAdapterV1::new(false, 24)
        .unwrap()
        .execute(&scenario, &EvidenceScope::default());
    assert!(matches!(
        result,
        Err(AttemptExecutionErrorV1::Unsupported(_))
    ));
}

#[test]
fn both_adapters_replay_commands_on_the_original_input() {
    for (index, mirror) in [(0, false), (4, true)] {
        let scenario = generate_mission(&MissionRecipeV1::default(), 813, index)
            .unwrap()
            .mission
            .unwrap()
            .scenario;
        let (evaluation, details) = IncumbentAdapterV1::new(mirror, 24)
            .unwrap()
            .execute(&scenario, &EvidenceScope::default())
            .unwrap_or_else(|_| panic!("adapter failed"));
        assert_eq!(evaluation.scenario, scenario);
        assert_eq!(
            evaluation.physical_sha256,
            physical_identity(&scenario).unwrap()
        );
        assert!(evaluation.integrity_passed);
        assert!(evaluation.replay.complete_episode_verified);
        assert_eq!(details["reflected"], mirror);
        assert!(
            evaluation.execution.episode.final_state.sim_time_s <= scenario.sim.max_time_s + 1e-9
        );
    }
}
