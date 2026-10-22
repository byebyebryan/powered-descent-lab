use super::*;
use crate::verify::*;
use pd_control::{Controller, TerminalPdgController};
use pd_core::{BoundedRunStopCauseV1, Command, PhysicalOutcome};
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

fn scenario() -> ScenarioSpec {
    serde_json::from_str(include_str!(
        "../../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap()
}
fn program(activation: ActivationV1) -> ActivationProgramV1 {
    ActivationProgramV1 {
        schema_version: 1,
        activation,
    }
}
fn limits(tick: u64) -> BoundedRunLimitsV1 {
    BoundedRunLimitsV1 {
        command_coverage_end_physics_step: tick.div_ceil(2) * 2,
        hard_end_physics_step: tick,
    }
}
fn short_evaluation() -> (EpisodeEvaluationV1, ManeuverTraceV1) {
    let mut scenario = scenario();
    scenario.sim.max_time_s = 0.125;
    evaluate(
        &scenario,
        &program(ActivationV1::FixedTick { physics_step: 12 }),
        &source_scope(Vec::new()),
    )
    .unwrap()
}

#[test]
fn persisted_integer_telemetry_uses_the_existing_decoder_without_relaxing_commands_or_values() {
    let mut original = pd_run::AgentFrame::command_only(Command {
        throttle_frac: 0.4,
        target_attitude_rad: -0.2,
    });
    original
        .metrics
        .insert("count".into(), pd_run::TelemetryValue::Integer(0));
    let mut loaded: pd_run::AgentFrame =
        serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
    assert_ne!(original, loaded);
    assert!(frame_matches(&original, &loaded).unwrap());
    loaded
        .metrics
        .insert("count".into(), pd_run::TelemetryValue::Float(1.0));
    assert!(!frame_matches(&original, &loaded).unwrap());
    loaded = serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
    loaded.command.throttle_frac = loaded.command.throttle_frac.next_up();
    assert!(!frame_matches(&original, &loaded).unwrap());
}
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        static ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "pd-maneuver-tests-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn bundle(&self) -> std::path::PathBuf {
        self.0.join("bundle")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn native_scheduler_has_no_extra_coast_and_stops_planner_calls_after_commitment() {
    let s = scenario();
    for (activation, calls, backend, activation_tick) in [
        (ActivationV1::Immediate {}, 1, 13, Some(0)),
        (ActivationV1::FixedTick { physics_step: 12 }, 2, 7, Some(12)),
        (ActivationV1::Never {}, 3, 0, None),
        (ActivationV1::FixedTick { physics_step: 36 }, 3, 0, None),
    ] {
        let p = program(activation);
        let captured = run(&s, &p, limits(25), CaptureModeV1::Captured).unwrap();
        let summary = run(&s, &p, limits(25), CaptureModeV1::Summary).unwrap();
        assert_eq!(captured.episode.summary, summary.episode.summary);
        assert_eq!(captured.counters, summary.counters);
        assert_eq!(captured.feedback, summary.feedback);
        assert!(
            summary.trace.is_none()
                && summary.episode.capture.is_none()
                && summary.episode.agent_updates.is_none()
        );
        assert_eq!(
            captured.counters,
            CountersV1 {
                physics_steps: 25,
                command_updates: 13,
                backend_updates: backend,
                planner_calls: calls
            }
        );
        assert_eq!(
            captured
                .feedback
                .accepted
                .map(|a| a.activation_physics_step),
            activation_tick
        );
        let trace = captured.trace.unwrap();
        assert_eq!(
            trace
                .boundaries
                .iter()
                .filter_map(|b| b.planner.as_ref().map(|_| b.physics_step))
                .collect::<Vec<_>>(),
            match calls {
                1 => vec![0],
                2 => vec![0, 12],
                _ => vec![0, 12, 24],
            }
        );
        let actions = &captured.episode.capture.as_ref().unwrap().run.actions;
        let mut replay =
            Episode::reset(&s, IMPLEMENTATION, limits(25), CaptureModeV1::Summary).unwrap();
        let ctx = RunContext::from_scenario(&s).unwrap();
        let mut direct = TerminalPdgController::default();
        for (index, action) in actions.iter().enumerate() {
            if Some(action.physics_step) == activation_tick {
                direct.reset(&ctx);
                assert_eq!(
                    captured.episode.agent_updates.as_ref().unwrap()[index].frame,
                    direct.update(&ctx, &replay.observation())
                );
            }
            if activation_tick.is_none_or(|tick| action.physics_step < tick) {
                assert_eq!(action.command, Command::idle());
            }
            replay.step(action.command).unwrap();
        }
    }
}

#[test]
fn zero_step_and_partial_native_end_never_get_extra_callbacks() {
    let s = scenario();
    let p = program(ActivationV1::Immediate {});
    let zero = run(&s, &p, limits(0), CaptureModeV1::Captured).unwrap();
    assert_eq!(
        zero.counters,
        CountersV1 {
            physics_steps: 0,
            command_updates: 0,
            backend_updates: 0,
            planner_calls: 0
        }
    );
    assert!(zero.trace.unwrap().boundaries.is_empty());
    assert_eq!(zero.feedback.native_end.unwrap().physics_step, 0);
    let partial = run(&s, &p, limits(3), CaptureModeV1::Captured).unwrap();
    assert_eq!(partial.counters.command_updates, 2);
    assert_eq!(partial.feedback.native_end.unwrap().physics_step, 3);
    assert_eq!(
        partial.episode.summary.stop,
        BoundedRunStopCauseV1::HardDeadlineReached
    );
    assert!(
        program(ActivationV1::FixedTick { physics_step: 2 })
            .validate()
            .is_err()
    );
    assert!(
        serde_json::from_str::<ActivationProgramV1>(
            r#"{"schema_version":1,"activation":{"kind":"never","extra":1}}"#
        )
        .is_err()
    );
}

struct FaultController {
    inner: TerminalLandingController,
    reject: bool,
}
impl ManeuverController for FaultController {
    fn descriptor(&self) -> ManeuverDescriptorV1 {
        self.inner.descriptor()
    }
    fn submit(&mut self, _: &ManeuverRequestV1, _: u64) -> Result<AdmissionV1, String> {
        Ok(AdmissionV1::Unsupported {
            reason: "synthetic rejection".into(),
        })
    }
    fn frame(&mut self, obs: &Observation) -> Result<pd_run::AgentFrame, String> {
        if !self.reject && obs.physics_step == 2 {
            return Err("synthetic backend failure".into());
        }
        self.inner.frame(obs)
    }
    fn feedback(&self) -> ManeuverFeedbackV1 {
        self.inner.feedback()
    }
    fn native_end(&mut self, end: NativeEndV1) -> Result<(), String> {
        self.inner.native_end(end)
    }
}

#[test]
fn semantic_rejection_consumes_intervals_but_execution_error_keeps_native_prefix() {
    let s = scenario();
    let mut controller = FaultController {
        inner: TerminalLandingController::new(&s).unwrap(),
        reject: true,
    };
    let rejected = run_with_controller(
        &s,
        &program(ActivationV1::Immediate {}),
        limits(25),
        CaptureModeV1::Captured,
        &mut controller,
    )
    .unwrap();
    assert_eq!(rejected.counters.physics_steps, 25);
    assert_eq!(rejected.counters.command_updates, 13);
    assert!(rejected.episode.summary.failure.is_none());
    assert_eq!(rejected.counters.planner_calls, 3);
    assert!(matches!(
        rejected.trace.unwrap().boundaries[0]
            .planner
            .as_ref()
            .unwrap()
            .admission,
        Some(AdmissionV1::Unsupported { .. })
    ));
    let mut controller = FaultController {
        inner: TerminalLandingController::new(&s).unwrap(),
        reject: false,
    };
    let failed = run_with_controller(
        &s,
        &program(ActivationV1::Never {}),
        limits(25),
        CaptureModeV1::Captured,
        &mut controller,
    )
    .unwrap();
    assert_eq!(
        failed.episode.summary.stop,
        BoundedRunStopCauseV1::ExecutionInvalid
    );
    assert_eq!(failed.counters.physics_steps, 2);
    assert_eq!(failed.counters.command_updates, 1);
    assert_eq!(
        failed.episode.summary.manifest.physical_outcome,
        PhysicalOutcome::Flying
    );
    assert_eq!(
        failed.trace.unwrap().boundaries[1].error.as_deref(),
        Some("synthetic backend failure")
    );
    assert!(failed.feedback.native_end.is_some());
}

#[test]
fn complete_bundle_has_separate_replays_resource_counts_and_create_only_writes() {
    let temp = Temp::new();
    let (evaluation, trace) = short_evaluation();
    save_bundle(&temp.bundle(), &evaluation, &trace).unwrap();
    let verified = verify_bundle(&temp.bundle());
    assert!(verified.report.passed(), "{:?}", verified.report);
    assert!(!verified.report.neutral_verified_target_landing);
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
    assert_eq!(verified.report.resources.reconstruction_physics_steps, 30);
    let receipt: BundleReceiptV1 =
        serde_json::from_slice(&fs::read(temp.bundle().join("receipt.json")).unwrap()).unwrap();
    assert_eq!(
        receipt.preparation_resources.neutral_replay_passes_started,
        2
    );
    assert_eq!(
        receipt.preparation_resources.known_neutral_physics_steps,
        30
    );
    assert_eq!(
        receipt.preparation_resources.reconstruction_physics_steps,
        30
    );
    let original = fs::read(temp.bundle().join("receipt.json")).unwrap();
    assert!(save_bundle(&temp.bundle(), &evaluation, &trace).is_err());
    assert_eq!(
        fs::read(temp.bundle().join("receipt.json")).unwrap(),
        original
    );
}

#[test]
fn input_drift_fails_bundle_integrity_while_preserving_independent_replay() {
    let temp = Temp::new();
    let (mut evaluation, trace) = short_evaluation();
    evaluation
        .evidence_after
        .input_files
        .push(pd_eval::FileIdentityV1 {
            path: "synthetic-changed-input.json".into(),
            sha256: "0".repeat(64),
        });
    evaluation.integrity_passed = false;
    evaluation.verified_target_landing = false;
    save_bundle(&temp.bundle(), &evaluation, &trace).unwrap();
    let verified = verify_bundle(&temp.bundle());
    assert!(verified.report.physical_replay.passed());
    assert!(verified.report.controller_reconstruction.passed());
    assert!(verified.report.planner_reconstruction.passed());
    assert!(!verified.report.bundle_integrity.passed());
    assert!(!verified.report.passed());
}

#[test]
fn controller_and_planner_reconstruction_detect_independent_semantic_tampering() {
    let (evaluation, original) = short_evaluation();
    for mutation in [
        "tick",
        "payload",
        "admission",
        "owner",
        "final",
        "program",
        "feedback",
        "extra-call",
        "counter",
        "version",
        "too-many-boundaries",
    ] {
        let mut trace = original.clone();
        let active_index = trace
            .boundaries
            .iter()
            .position(|b| {
                b.planner
                    .as_ref()
                    .is_some_and(|p| matches!(p.decision, PlannerDecisionV1::Request { .. }))
            })
            .unwrap();
        match mutation {
            "tick" => trace.boundaries[active_index].physics_step += 2,
            "payload" => {
                if let PlannerDecisionV1::Request { request } = &mut trace.boundaries[active_index]
                    .planner
                    .as_mut()
                    .unwrap()
                    .decision
                {
                    request.goal = ManeuverGoalV1::LandOnMissionTarget {
                        target_pad_id: "absent".into(),
                    };
                }
            }
            "admission" => {
                trace.boundaries[active_index]
                    .planner
                    .as_mut()
                    .unwrap()
                    .admission = Some(AdmissionV1::Busy)
            }
            "owner" => trace.boundaries[active_index].owner = CommandOwnerV1::UprightCoast,
            "final" => {
                trace
                    .final_feedback
                    .native_end
                    .as_mut()
                    .unwrap()
                    .physics_step += 1
            }
            "program" => {
                trace.descriptor.configuration["program"]["activation"] =
                    serde_json::json!({"kind":"never"})
            }
            "feedback" => {
                trace.boundaries[active_index]
                    .planner
                    .as_mut()
                    .unwrap()
                    .feedback
                    .backend_updates += 1
            }
            "extra-call" => {
                trace.boundaries[active_index + 1].planner = trace.boundaries[0].planner.clone()
            }
            "counter" => trace.counters.physics_steps += 1,
            "version" => trace.schema_version = 2,
            "too-many-boundaries" => {
                trace.boundaries.push(trace.boundaries[0].clone());
                trace.boundaries.push(trace.boundaries[0].clone());
            }
            _ => unreachable!(),
        }
        let controller =
            reconstruct_controller(&evaluation, &trace, &mut ProofResourcesV1::default());
        let planner = reconstruct_planner(&evaluation, &trace, &mut ProofResourcesV1::default());
        assert!(!controller.passed() || !planner.passed(), "{mutation}");
        if ["admission", "owner", "final", "counter"].contains(&mutation) {
            assert!(!controller.passed(), "{mutation}");
            assert!(planner.passed(), "{mutation}: {planner:?}");
        }
        if ["feedback", "extra-call"].contains(&mutation) {
            assert!(!planner.passed(), "{mutation}");
        }
        if mutation == "feedback" {
            assert!(controller.passed(), "{controller:?}");
        }
    }
    let mut commands = evaluation.clone();
    commands.execution.controller_updates[0]
        .frame
        .command
        .throttle_frac = 0.75;
    assert!(
        !reconstruct_controller(&commands, &original, &mut ProofResourcesV1::default()).passed()
    );
}

#[test]
fn capture_preflight_bounds_native_work_before_calling_the_neutral_loader() {
    let (evaluation, _) = short_evaluation();
    for mutation in ["cadence", "horizon", "updates"] {
        let temp = Temp::new();
        fs::create_dir(temp.bundle()).unwrap();
        fs::create_dir(temp.bundle().join("capture")).unwrap();
        let mut invalid = evaluation.clone();
        match mutation {
            "cadence" => {
                invalid.scenario.sim.physics_hz = 2400;
                invalid.scenario.sim.controller_hz = 1;
            }
            "horizon" => invalid.scenario.sim.max_time_s = 601.0,
            _ => invalid
                .execution
                .episode
                .run
                .actions
                .push(invalid.execution.episode.run.actions[0].clone()),
        }
        fs::write(
            temp.bundle().join("capture/evaluation.json"),
            serde_json::to_vec(&invalid).unwrap(),
        )
        .unwrap();
        let verified = verify_bundle(&temp.bundle());
        assert!(!verified.report.physical_replay.passed(), "{mutation}");
        assert_eq!(
            verified.report.resources.neutral_replay_passes_started, 0,
            "{mutation}"
        );
    }
}

#[test]
fn byte_tampering_missing_receipt_partial_write_and_source_mismatch_keep_physical_proof_separate() {
    let (evaluation, trace) = short_evaluation();
    for mutation in [
        "trace-bytes",
        "receipt",
        "evaluation",
        "missing-receipt",
        "partial",
    ] {
        let temp = Temp::new();
        if mutation == "partial" {
            assert!(
                save_bundle_with_hook(&temp.bundle(), &evaluation, &trace, &mut || Err(
                    "injected I/O interruption".into()
                ))
                .is_err()
            );
            assert!(!temp.bundle().join("receipt.json").exists());
            assert!(save_bundle(&temp.bundle(), &evaluation, &trace).is_err());
        } else {
            save_bundle(&temp.bundle(), &evaluation, &trace).unwrap();
            let path = temp.bundle().join(match mutation {
                "trace-bytes" => "trace.json",
                "evaluation" => "capture/evaluation.json",
                _ => "receipt.json",
            });
            if mutation == "missing-receipt" {
                fs::remove_file(path).unwrap();
            } else if mutation == "receipt" {
                let mut receipt: BundleReceiptV1 =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                receipt.trace_sha256 = "0".repeat(64);
                fs::write(path, serde_json::to_vec(&receipt).unwrap()).unwrap();
            } else {
                let mut bytes = fs::read(&path).unwrap();
                bytes.push(b' ');
                fs::write(path, bytes).unwrap();
            }
        }
        let verified = verify_bundle(&temp.bundle());
        assert!(!verified.report.passed(), "{mutation}");
        if mutation != "evaluation" {
            assert!(
                verified.report.physical_replay.passed(),
                "{mutation}: {:?}",
                verified.report
            );
        } else {
            assert!(!verified.report.physical_replay.passed());
        }
    }
    let mut changed = evaluation.clone();
    changed.evidence_before.executable_sha256 = "0".repeat(64);
    changed.evidence_after = changed.evidence_before.clone();
    let temp = Temp::new();
    save_bundle(&temp.bundle(), &changed, &trace).unwrap();
    let verified = verify_bundle(&temp.bundle());
    assert!(verified.report.physical_replay.passed());
    assert!(!verified.report.controller_reconstruction.passed());
    assert!(!verified.report.planner_reconstruction.passed());
    let mut snapshot = source_scope(Vec::new()).snapshot().unwrap();
    snapshot.source_files[0].sha256 = "0".repeat(64);
    assert!(check_compiled_sources(&snapshot).is_err());
}

#[cfg(unix)]
#[test]
fn bounded_regular_inputs_reject_symlinks_and_oversize_files() {
    let temp = Temp::new();
    let input = temp.0.join("input");
    fs::write(&input, "12345").unwrap();
    assert!(read_regular(&input, 4).is_err());
    assert_eq!(read_regular(&input, 5).unwrap(), b"12345");
    let link = temp.0.join("link");
    std::os::unix::fs::symlink(&input, &link).unwrap();
    assert!(read_regular(&link, 10).is_err());
    let directory_link = temp.0.join("parent-link");
    std::os::unix::fs::symlink(&temp.0, &directory_link).unwrap();
    assert!(read_regular(&directory_link.join("input"), 10).is_err());
}
