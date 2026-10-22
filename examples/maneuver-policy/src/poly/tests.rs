use super::*;
use pd_control::poly_landing::{LandingKinematicsV1, build_reference, paired_mean};
use pd_core::Vec2;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn temp() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "pd-poly-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&root).unwrap();
    root
}
fn input() -> (ScenarioSpec, TimedProgramV2) {
    let mut s: ScenarioSpec = serde_json::from_str(include_str!(
        "../../../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap();
    s.vehicle.initial_fuel_kg = 5806.024;
    s.initial_state.position_m = Vec2::new(-317.915, 197.882);
    s.initial_state.velocity_mps = Vec2::new(62.85, -13.552);
    let ctx = RunContext::from_scenario(&s).unwrap();
    let reference = build_reference(
        &ctx,
        LandingKinematicsV1 {
            position_m: s.initial_state.position_m,
            velocity_mps: s.initial_state.velocity_mps,
        },
        LandingKinematicsV1 {
            position_m: Vec2::new(0.0, 4.995),
            velocity_mps: Vec2::new(0.0, -1.5),
        },
        1740,
    )
    .unwrap();
    let first = paired_mean(&reference, &ctx, 0);
    s.initial_state.attitude_rad = first.x.atan2(first.y);
    let p = TimedProgramV2 {
        schema_version: 2,
        backend: POLY_LANDING_BACKEND.into(),
        activation_physics_step: 0,
        absolute_deadline_physics_step: 5400,
        command_coverage_end_physics_step: None,
        request: ManeuverRequestV2 {
            schema_version: 2,
            id: "landing".into(),
            goal: ManeuverGoalV2::LandOnMissionTargetAt {
                target_pad_id: "pad_main".into(),
                reference_end_physics_step: 1740,
            },
        },
        regression_prefix: vec![],
        rejection_coast: Command::idle(),
    };
    (s, p)
}

#[test]
fn timed_capture_has_independent_proofs_and_detects_resealed_reference_tampering() {
    let root = temp();
    let (s, p) = input();
    let scenario = root.join("scenario.json");
    let program = root.join("program.json");
    pd_missions::write_json_create_only(&scenario, &s).unwrap();
    pd_missions::write_json_create_only(&program, &p).unwrap();
    let (evaluation, trace) = evaluate_files(&scenario, &program).unwrap();
    assert!(evaluation.verified_target_landing);
    assert_eq!(trace.feedback.constructions, 1);
    assert_eq!(trace.planner_requests, 1);
    assert!(
        evaluation
            .evidence_before
            .source_files
            .iter()
            .any(|f| f.path.ends_with("pd-control/src/poly_landing/reference.rs"))
    );
    let bundle = root.join("bundle");
    verify::save_bundle(&bundle, &evaluation, &trace).unwrap();
    assert!(verify::save_bundle(&bundle, &evaluation, &trace).is_err());
    let verified = verify::verify_bundle(&bundle);
    assert!(verified.report.passed(), "{:?}", verified.report);
    assert!(verified.report.neutral_verified_target_landing);
    assert_eq!(verified.report.resources.neutral_replay_passes_started, 1);
    assert_eq!(
        verified
            .report
            .resources
            .controller_reconstruction_passes_started,
        1
    );
    assert_eq!(
        verified
            .report
            .resources
            .planner_reconstruction_passes_started,
        1
    );
    assert_eq!(
        verified.report.resources.reconstruction_physics_steps,
        trace.physics_steps * 2
    );
    assert_eq!(
        verified
            .report
            .resources
            .reconstruction_reference_constructions,
        2
    );
    assert!(!crate::verify::verify_bundle(&bundle).report.passed());

    let mut changed = trace.clone();
    changed
        .feedback
        .accepted
        .as_mut()
        .unwrap()
        .reference
        .horizontal_coefficients_mps2[0] += 0.1;
    let bytes = serde_json::to_vec_pretty(&changed).unwrap();
    std::fs::write(bundle.join("trace.json"), &bytes).unwrap();
    let mut receipt: verify::TimedBundleReceiptV2 =
        serde_json::from_slice(&std::fs::read(bundle.join("receipt.json")).unwrap()).unwrap();
    receipt.trace_sha256 = pd_missions::sha256(&bytes);
    std::fs::write(
        bundle.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    let tampered = verify::verify_bundle(&bundle);
    assert!(tampered.report.bundle_integrity.passed());
    assert!(tampered.report.physical_replay.passed());
    assert!(!tampered.report.controller_reconstruction.passed());
    assert!(!tampered.report.planner_reconstruction.passed());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejected_timed_construction_stays_an_honest_native_miss_with_no_retry() {
    let (s, mut p) = input();
    let ManeuverGoalV2::LandOnMissionTargetAt {
        reference_end_physics_step,
        ..
    } = &mut p.request.goal
    else {
        panic!()
    };
    *reference_end_physics_step = 4;
    let result = run(&s, &p).unwrap();
    assert!(matches!(
        result.trace.boundaries[0].admission,
        Some(AdmissionV2::NoPlan { .. })
    ));
    assert_eq!(result.trace.planner_requests, 1);
    assert_eq!(result.trace.feedback.constructions, 1);
    assert_eq!(result.trace.feedback.command_updates, 0);
    assert!(result.trace.feedback.accepted.is_none());
    assert!(result.trace.feedback.native_end.is_some());
    assert_ne!(
        result.episode.summary.manifest.physical_outcome,
        pd_core::PhysicalOutcome::LandedOnTarget
    );
}

#[test]
fn exact_command_handoff_does_not_round_to_the_old_planner_grid_or_reset_fuel() {
    let (s, mut p) = input();
    p.activation_physics_step = 6;
    p.regression_prefix = (0..6)
        .step_by(2)
        .map(|tick| FlightProgramUpdateV1 {
            physics_step: tick,
            phase: "test_prefix_coast".into(),
            command: Command {
                throttle_frac: 0.0,
                target_attitude_rad: s.initial_state.attitude_rad,
            },
        })
        .collect();
    let result = run(&s, &p).unwrap();
    let accepted = result.trace.feedback.accepted.unwrap();
    assert_eq!(accepted.entry.physics_step, 6);
    assert_eq!(accepted.entry.sim_time_s, 6.0 / 120.0);
    assert_eq!(accepted.entry.fuel_kg, s.vehicle.initial_fuel_kg);
    assert_eq!(accepted.reference.physics_ticks, 1734);
    assert_eq!(
        result.trace.boundaries[3].admission,
        Some(AdmissionV2::Accepted)
    );
    assert_eq!(result.trace.boundaries[2].owner, "regression_prefix");
    assert_eq!(result.trace.boundaries[3].owner, "poly_landing");
}

#[test]
fn backend_selection_is_explicit_and_prefix_gaps_fail_before_native_work() {
    let (s, p) = input();
    let mut bad = p.clone();
    bad.backend = "ordinary-terminal-pdg-v1".into();
    assert!(controller(&s, &bad).is_err());
    bad.backend = "unknown".into();
    assert!(controller(&s, &bad).is_err());
    bad = p.clone();
    bad.schema_version = 1;
    assert!(controller(&s, &bad).is_err());
    bad = p.clone();
    bad.command_coverage_end_physics_step = Some(1744);
    assert!(controller(&s, &bad).is_err());
    bad.schema_version = 3;
    assert!(controller(&s, &bad).is_err());
    bad.backend = CHECKED_POLY_LANDING_BACKEND.into();
    assert!(controller(&s, &bad).is_ok());
    bad.command_coverage_end_physics_step = Some(1746);
    assert!(controller(&s, &bad).is_err());
    bad = p;
    bad.activation_physics_step = 6;
    assert!(controller(&s, &bad).is_err());
}

#[test]
fn checked_admission_preserves_every_native_command_sample_and_contact_for_an_admitted_burn() {
    let (s, mut p) = input();
    let legacy = run(&s, &p).unwrap();
    p.schema_version = 3;
    p.backend = CHECKED_POLY_LANDING_BACKEND.into();
    p.command_coverage_end_physics_step = Some(1744);
    let checked = run(&s, &p).unwrap();
    assert_eq!(
        legacy.episode.summary.final_state,
        checked.episode.summary.final_state
    );
    let a = legacy.episode.capture.unwrap();
    let b = checked.episode.capture.unwrap();
    assert_eq!(
        serde_json::to_vec(&a.run.samples).unwrap(),
        serde_json::to_vec(&b.run.samples).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&a.incoming_contact).unwrap(),
        serde_json::to_vec(&b.incoming_contact).unwrap()
    );
    let commands = |updates: Option<Vec<pd_run::AgentUpdateRecord>>| {
        updates
            .unwrap()
            .into_iter()
            .map(|u| u.frame.command)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        commands(legacy.episode.agent_updates),
        commands(checked.episode.agent_updates)
    );
    assert_eq!(checked.trace.feedback.admission_actuation_checks, 872);
    assert_eq!(legacy.trace.feedback.admission_actuation_checks, 0);
}

#[test]
fn checked_reference_expiry_is_a_replayable_finite_prefix_without_extra_command() {
    let root = temp();
    let (mut s, mut p) = input();
    s.sim.max_time_s = 40.0;
    s.vehicle.initial_fuel_kg = 1800.0;
    s.initial_state.position_m = Vec2::new(-53.760000000000005, 37.794999999999995);
    s.initial_state.velocity_mps = Vec2::new(24.0, -8.0);
    s.initial_state.attitude_rad = -0.0174281368974879;
    s.initial_state.angular_rate_radps = 0.8;
    p.schema_version = 3;
    p.backend = CHECKED_POLY_LANDING_BACKEND.into();
    p.absolute_deadline_physics_step = 4800;
    p.command_coverage_end_physics_step = Some(964);
    p.request.goal = ManeuverGoalV2::LandOnMissionTargetAt {
        target_pad_id: "pad_main".into(),
        reference_end_physics_step: 960,
    };
    let scenario = root.join("scenario.json");
    let program = root.join("program.json");
    pd_missions::write_json_create_only(&scenario, &s).unwrap();
    pd_missions::write_json_create_only(&program, &p).unwrap();
    let (evaluation, trace) = evaluate_files(&scenario, &program).unwrap();
    let episode = &evaluation.execution.episode;
    assert_eq!(
        episode.stop,
        pd_core::BoundedRunStopCauseV1::CoverageExhausted
    );
    assert!(episode.failure.is_none());
    assert_eq!(episode.final_state.physics_step, 964);
    assert_eq!(episode.limits.hard_end_physics_step, 4800);
    assert_eq!(episode.limits.command_coverage_end_physics_step, 964);
    assert_eq!(trace.command_updates, 482);
    assert_eq!(trace.boundaries.last().unwrap().physics_step, 962);
    assert!(trace.feedback.reference_exhausted);
    assert_eq!(trace.feedback.admission_actuation_checks, 482);
    assert_eq!(
        episode.run.manifest.physical_outcome,
        pd_core::PhysicalOutcome::Flying
    );
    assert!(!evaluation.verified_target_landing);
    assert!(evaluation.replay.complete_episode_verified);
    let result = run(&s, &p).unwrap();
    assert_eq!(
        trace.feedback.accepted.as_ref().unwrap().entry.physics_step,
        0
    );
    assert_eq!(
        result.episode.summary.final_state.fuel_kg,
        episode.final_state.fuel_kg
    );
    let bundle = root.join("bundle");
    verify::save_bundle(&bundle, &evaluation, &trace).unwrap();
    let verified = verify::verify_bundle(&bundle);
    assert!(verified.report.passed(), "{:?}", verified.report);
    assert!(!verified.report.neutral_verified_target_landing);
    std::fs::remove_dir_all(root).unwrap();
}
