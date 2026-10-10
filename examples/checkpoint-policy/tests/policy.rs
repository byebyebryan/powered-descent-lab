use pd_core::{RunContext, SimulationState};
use pd_missions::{MissionRecipeV1, generate_mission};
use pd_policy_example::*;
use pd_run::Agent;

#[test]
fn checkpoint_has_explicit_mapping_and_bounded_identity_independent_commands() {
    let bytes = include_bytes!("../checkpoints/linear-demo-v1.json");
    let mut policy = LinearPolicy::from_bytes(bytes).unwrap();
    let mission = generate_mission(&MissionRecipeV1::default(), 123, 0)
        .unwrap()
        .mission
        .unwrap();
    let mut ctx = RunContext::from_scenario(&mission.scenario).unwrap();
    let observation = SimulationState::new(&ctx).unwrap().build_observation(&ctx);
    let first = policy.update((&ctx).into(), &observation).unwrap();
    assert!((0.0..=1.0).contains(&first.command.throttle_frac));
    assert!(first.command.target_attitude_rad.abs() <= 1.0);
    assert_eq!(
        policy.descriptor().checkpoint_sha256,
        Some(pd_missions::sha256(bytes))
    );
    ctx.scenario_id = "another collection".to_owned();
    ctx.scenario_seed = 999;
    ctx.scenario_tags = vec!["validation".to_owned()];
    assert_eq!(policy.update((&ctx).into(), &observation).unwrap(), first);
    let mut bad: LinearCheckpointV1 = serde_json::from_slice(bytes).unwrap();
    bad.observation_schema = "unknown".to_owned();
    assert!(bad.validate().is_err());
    bad.observation_schema = OBSERVATION_SCHEMA.to_owned();
    bad.input_scales[0] = 0.0;
    assert!(bad.validate().is_err());
}

#[test]
fn checkpoint_weights_affect_only_the_independent_approach() {
    let mut checkpoint: LinearCheckpointV1 =
        serde_json::from_slice(include_bytes!("../checkpoints/linear-demo-v1.json")).unwrap();
    let first = LinearPolicy::from_bytes(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
    checkpoint.bias[0] += 1.0;
    let second = LinearPolicy::from_bytes(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
    assert_ne!(first.descriptor(), second.descriptor());
    assert_ne!(
        first.descriptor().checkpoint_sha256,
        second.descriptor().checkpoint_sha256
    );
}
