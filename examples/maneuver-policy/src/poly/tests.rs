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
    bad = p;
    bad.activation_physics_step = 6;
    assert!(controller(&s, &bad).is_err());
}
