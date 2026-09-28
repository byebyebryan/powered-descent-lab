use std::{path::Path, sync::OnceLock};

use super::*;

fn request() -> WaypointDirectNominalDirectGenerationRequest {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let manifest = crate::load_waypoint_direct_generation_fresh_manifest(repo).unwrap();
    manifest.request("fresh_flat_span_600_delta_000").unwrap()
}

fn evaluated() -> &'static NominalDirectFlightEvaluationV1 {
    static CASE: OnceLock<NominalDirectFlightEvaluationV1> = OnceLock::new();
    CASE.get_or_init(|| {
        evaluate_nominal_direct_flight(&request(), &BodyAwareTerminalPolicyV1::default()).unwrap()
    })
}

#[test]
fn typed_preflight_rejections_create_no_simulation_or_generation() {
    let base = request();
    let policy = BodyAwareTerminalPolicyV1::default();
    let mut variants = Vec::new();
    let mut malformed = base.clone();
    malformed.target_pad_id = "missing".into();
    variants.push((malformed, "invalid"));
    let mut malformed = base.clone();
    malformed.scenario.sim.controller_hz = 0;
    variants.push((malformed, "invalid"));
    let mut malformed = base.clone();
    let source = malformed
        .scenario
        .world
        .landing_pad(&malformed.source_pad_id)
        .unwrap()
        .clone();
    malformed.scenario.world.landing_pads.push(source);
    variants.push((malformed, "invalid"));
    let mut unsupported = base.clone();
    unsupported.scenario.sim.controller_hz = 120;
    variants.push((unsupported, "unsupported"));
    let mut unsupported = base.clone();
    unsupported.scenario.vehicle.dry_mass_kg += 1.0;
    variants.push((unsupported, "unsupported"));
    let mut unsupported = base.clone();
    unsupported.scenario.initial_state.velocity_mps.x = 0.01;
    variants.push((unsupported, "unsupported"));
    let mut unsupported = base.clone();
    unsupported.policy.maximum_variants = 21;
    variants.push((unsupported, "unsupported"));
    for (request, status) in variants {
        let preflight = preflight_nominal_direct_flight(&request, &policy);
        assert!(!preflight.supported && !preflight.simulation_created);
        assert_eq!(preflight.rejection.unwrap().status(), status);
        let result = evaluate_nominal_direct_flight(&request, &policy).unwrap();
        assert_eq!(result.decision.status(), status);
        assert!(result.generation.is_none());
        assert_eq!(result.compute.ordinary_execution_physics_ticks, 0);
        assert_eq!(result.compute.source_rows_predeclared, 0);
    }
    let mut tuned = policy.clone();
    tuned.contact_undershoot_m = 0.006;
    assert_eq!(
        preflight_nominal_direct_flight(&base, &tuned)
            .rejection
            .unwrap()
            .status(),
        "unsupported"
    );
}

#[test]
fn supported_input_preflight_never_constructs_a_flight() {
    let preflight =
        preflight_nominal_direct_flight(&request(), &BodyAwareTerminalPolicyV1::default());
    assert!(preflight.supported && preflight.rejection.is_none() && !preflight.simulation_created);
}

#[test]
fn real_direct_program_maps_every_post_step_command_to_normal_callback_clock() {
    let evaluated = evaluated();
    let NominalDirectFlightDecisionV1::Direct {
        selected_row_index,
        program,
        program_identity,
        ..
    } = &evaluated.decision
    else {
        panic!("flat direct expected")
    };
    let generation = evaluated.generation.as_ref().unwrap();
    let witness = generation.rows[*selected_row_index]
        .witness
        .as_ref()
        .unwrap();
    assert_eq!(*selected_row_index, 5);
    assert_eq!(program.updates.len(), witness.commands.len());
    for (program, witness) in program.updates.iter().zip(&witness.commands) {
        assert_eq!(program.physics_step + 1, witness.physics_step);
        assert_eq!(program.command, witness.command);
        assert_eq!(program.phase, witness.phase);
    }
    assert_eq!(
        program.expected_contact_physics_step,
        witness.verification.physics_ticks_advanced
    );
    assert!(program.expected_contact_physics_step < program.planned_end_physics_step);
    assert_eq!(
        nominal_direct_flight_identity(program).unwrap(),
        *program_identity
    );
    let encoded = serde_json::to_vec(&evaluated.decision).unwrap();
    let decoded: NominalDirectFlightDecisionV1 = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, evaluated.decision);
}

#[test]
fn ordinary_execution_matches_full_independent_safety_and_action_replay() {
    let evaluated = evaluated();
    let NominalDirectFlightDecisionV1::Direct {
        selected_row_index,
        program,
        ..
    } = &evaluated.decision
    else {
        panic!()
    };
    let witness = evaluated.generation.as_ref().unwrap().rows[*selected_row_index]
        .witness
        .as_ref()
        .unwrap();
    let mut compute = evaluated.compute.clone();
    let (evidence, artifacts) = execute_nominal_direct_flight_program(
        &request(),
        &BodyAwareTerminalPolicyV1::default(),
        witness,
        program,
        &mut compute,
    )
    .unwrap();
    assert!(
        evidence.passed
            && evidence.exact_command_and_clock_parity
            && evidence.ordinary_action_replay_parity
    );
    assert_eq!(
        artifacts.run.manifest.physics_steps,
        program.expected_contact_physics_step
    );
    assert_eq!(
        compute.ordinary_execution_physics_ticks,
        compute.action_replay_physics_ticks
    );
    assert!(compute.total_generation_physics_ticks.is_none());
}

#[test]
fn self_rehashed_finite_command_tamper_is_rejected_before_execution() {
    let evaluated = evaluated();
    let NominalDirectFlightDecisionV1::Direct {
        selected_row_index,
        program,
        ..
    } = &evaluated.decision
    else {
        panic!()
    };
    let witness = evaluated.generation.as_ref().unwrap().rows[*selected_row_index]
        .witness
        .as_ref()
        .unwrap();
    let mut altered = (**program).clone();
    altered.updates[0].command.throttle_frac *= 0.99;
    let _rehashed = nominal_direct_flight_identity(&altered).unwrap();
    let mut compute = NominalDirectFlightComputeV1::default();
    let error = execute_nominal_direct_flight_program(
        &request(),
        &BodyAwareTerminalPolicyV1::default(),
        witness,
        &altered,
        &mut compute,
    )
    .unwrap_err();
    assert!(error.to_string().contains("differs from"));
    assert_eq!(compute.ordinary_execution_physics_ticks, 0);
    assert_eq!(compute.selected_witness_verification_physics_ticks, 0);
}

#[test]
fn finite_unknown_is_not_an_unsupported_request_and_runs_no_fallback() {
    let mut request = request();
    // Valid supported physical input, but not enough time for this finite family.
    // This is exhaustion under the declared input budget, not impossibility.
    request.scenario.sim.max_time_s = 1.0;
    assert!(
        preflight_nominal_direct_flight(&request, &BodyAwareTerminalPolicyV1::default()).supported
    );
    let evaluated =
        evaluate_nominal_direct_flight(&request, &BodyAwareTerminalPolicyV1::default()).unwrap();
    assert_eq!(evaluated.decision.status(), "unknown");
    assert_eq!(evaluated.generation.as_ref().unwrap().rows.len(), 20);
    assert_eq!(evaluated.compute.ordinary_execution_physics_ticks, 0);
    assert_eq!(evaluated.compute.accepted_witnesses, 0);
}

#[test]
fn valid_tall_terrain_barrier_exhausts_finite_direct_family_without_waypoint_claim() {
    let mut request = request();
    let source = request
        .scenario
        .world
        .landing_pad(&request.source_pad_id)
        .unwrap();
    let target = request
        .scenario
        .world
        .landing_pad(&request.target_pad_id)
        .unwrap();
    let midpoint = (source.center_x_m + target.center_x_m) * 0.5;
    let mut points = request.scenario.world.terrain.points().to_vec();
    points.retain(|point| point.x < midpoint - 20.0 || point.x > midpoint + 20.0);
    points.extend([
        Vec2::new(midpoint - 20.0, 0.0),
        Vec2::new(midpoint, 5000.0),
        Vec2::new(midpoint + 20.0, 0.0),
    ]);
    points.sort_by(|a, b| a.x.total_cmp(&b.x));
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield { points_m: points };
    assert!(
        preflight_nominal_direct_flight(&request, &BodyAwareTerminalPolicyV1::default()).supported
    );
    let evaluated =
        evaluate_nominal_direct_flight(&request, &BodyAwareTerminalPolicyV1::default()).unwrap();
    let NominalDirectFlightDecisionV1::Unknown { reason, .. } = &evaluated.decision else {
        panic!("finite Unknown expected")
    };
    assert!(reason.contains("necessity is not established"));
    assert_eq!(evaluated.compute.source_rows_predeclared, 20);
    assert_eq!(evaluated.compute.accepted_witnesses, 0);
    assert_eq!(evaluated.compute.ordinary_execution_physics_ticks, 0);
}

#[test]
fn timing_does_not_participate_in_deterministic_flight_identity() {
    let evaluated = evaluated();
    let artifact = NominalDirectFlightArtifactV1 {
        schema_id: "nominal_direct_flight_v1".into(),
        schema_version: 1,
        request_identity: nominal_direct_flight_identity(&request()).unwrap(),
        decision: evaluated.decision.clone(),
        execution: None,
        compute: evaluated.compute.clone(),
        scope_non_claims: NON_CLAIMS.iter().map(|s| (*s).into()).collect(),
        identity: String::new(),
    };
    let id = nominal_direct_flight_artifact_identity(&artifact).unwrap();
    let mut changed = artifact.clone();
    changed.compute.generation_wall_time_us += 123;
    changed.compute.selected_verification_wall_time_us = 999;
    changed.compute.ordinary_execution_wall_time_us = 999;
    changed.compute.action_replay_wall_time_us = 999;
    changed.compute.artifact_writing_wall_time_us = 999;
    assert_eq!(
        id,
        nominal_direct_flight_artifact_identity(&changed).unwrap()
    );
    changed.compute.source_rows_predeclared += 1;
    assert_ne!(
        id,
        nominal_direct_flight_artifact_identity(&changed).unwrap()
    );
}

#[test]
fn non_direct_bundle_has_no_flight_and_existing_root_is_never_replaced() {
    let root = std::env::temp_dir().join(format!(
        "pd-lab-nominal-direct-rejection-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    let mut request = request();
    request.scenario.initial_state.velocity_mps.x = 0.01;
    let artifact =
        run_nominal_direct_flight(&request, &BodyAwareTerminalPolicyV1::default(), &root).unwrap();
    assert_eq!(artifact.decision.status(), "unsupported");
    assert!(artifact.execution.is_none());
    for absent in [
        "program.json",
        "witness.json",
        "actions.json",
        "manifest.json",
        "report.html",
    ] {
        assert!(!root.join(absent).exists());
    }
    let before = std::fs::read(root.join("summary.json")).unwrap();
    assert!(
        run_nominal_direct_flight(&request, &BodyAwareTerminalPolicyV1::default(), &root).is_err()
    );
    assert_eq!(std::fs::read(root.join("summary.json")).unwrap(), before);
    std::fs::remove_dir_all(root).unwrap();
}
