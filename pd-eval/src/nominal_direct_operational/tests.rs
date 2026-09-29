use super::*;
use std::{path::Path, sync::OnceLock};

fn request() -> WaypointDirectNominalDirectGenerationRequest {
    crate::load_waypoint_direct_generation_fresh_manifest(
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap(),
    )
    .unwrap()
    .request("fresh_flat_span_600_delta_000")
    .unwrap()
}

fn temp_root(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "pd-operational-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    root
}

fn evaluated() -> &'static crate::NominalDirectFlightEvaluationV1 {
    static CASE: OnceLock<crate::NominalDirectFlightEvaluationV1> = OnceLock::new();
    CASE.get_or_init(|| {
        evaluate_nominal_direct_flight(&request(), &BodyAwareTerminalPolicyV1::default()).unwrap()
    })
}

fn selected() -> (
    &'static FlightProgramV1,
    &'static BodyAwareTerminalWitnessV1,
) {
    let evaluation = evaluated();
    let NominalDirectFlightDecisionV1::Direct {
        program,
        selected_row_index,
        ..
    } = &evaluation.decision
    else {
        panic!("flat Direct expected")
    };
    (
        program,
        evaluation.generation.as_ref().unwrap().rows[*selected_row_index]
            .witness
            .as_ref()
            .unwrap(),
    )
}

fn assert_unknown_field_rejected<T: Serialize + serde::de::DeserializeOwned>(value: &T) {
    let mut json = serde_json::to_value(value).unwrap();
    assert!(serde_json::from_value::<T>(json.clone()).is_ok());
    json.as_object_mut()
        .unwrap()
        .insert("future_unadmitted_field".into(), serde_json::json!(true));
    let error = serde_json::from_value::<T>(json).err().unwrap();
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn admitted_nominal_run_is_safe_match_with_incoming_contact_and_fixed_guard_replay() {
    let (program, witness) = selected();
    let mut compute = NominalDirectFlightComputeV1::default();
    let (evidence, controlled) = execute_nominal_direct_operational_program(
        &request(),
        &BodyAwareTerminalPolicyV1::default(),
        witness,
        program,
        &mut compute,
    )
    .unwrap();
    assert_eq!(
        evidence.operational_outcome,
        NominalDirectOperationalOutcomeV1::CompletedSafeTarget
    );
    assert_eq!(evidence.nominal_comparison, NominalComparisonV1::Match);
    assert!(evidence.nominal_checks.all_passed());
    assert!(evidence.replay.passed && evidence.validity.initial_state_checked);
    assert_unknown_field_rejected(&evidence);
    assert_unknown_field_rejected(&evidence.nominal_checks);
    assert_unknown_field_rejected(&evidence.replay);
    assert_unknown_field_rejected(&evidence.validity);
    assert_unknown_field_rejected(&crate::OperationalGuardFailureEvidenceV1 {
        stage: "before_transition".into(),
        physics_step: 0,
        failure: pd_core::BoundedGuardFailureV1::new(
            pd_core::BoundedGuardFailureDispositionV1::SafetyRejected,
            "schema fixture",
        ),
    });
    assert_eq!(
        evidence.validity.pretransition_checks,
        controlled.bounded.run.manifest.physics_steps
    );
    assert_eq!(
        evidence.validity.posttransition_checks,
        evidence.validity.pretransition_checks
    );
    assert_eq!(
        evidence
            .validity
            .airborne_clearance_scan
            .airborne_state_count
            + 1,
        evidence
            .validity
            .airborne_clearance_scan
            .poststep_state_count
    );
    let incoming = controlled.bounded.incoming_contact.as_ref().unwrap();
    assert!(incoming.state.velocity_mps.y < 0.0);
    assert_eq!(
        controlled.bounded.final_state.velocity_mps,
        pd_core::Vec2::new(0.0, 0.0)
    );
    assert_eq!(
        controlled.bounded.run.actions,
        pd_control::run_flight_program(
            &RunContext::from_scenario(&request().scenario).unwrap(),
            program,
        )
        .unwrap()
        .run
        .actions
    );
}

#[test]
fn altered_self_rehashed_program_or_witness_is_rejected_before_motion() {
    let (program, witness) = selected();
    let mut altered = program.clone();
    altered.updates[0].command.throttle_frac *= 0.99;
    assert!(
        execute_nominal_direct_operational_program(
            &request(),
            &BodyAwareTerminalPolicyV1::default(),
            witness,
            &altered,
            &mut NominalDirectFlightComputeV1::default()
        )
        .is_err()
    );
    let mut altered = witness.clone();
    altered.source_handoff_reference.position_m.x += 1.0;
    assert!(
        execute_nominal_direct_operational_program(
            &request(),
            &BodyAwareTerminalPolicyV1::default(),
            &altered,
            program,
            &mut NominalDirectFlightComputeV1::default()
        )
        .is_err()
    );
}

#[test]
fn unsupported_bundle_is_create_only_and_has_no_executed_prefix() {
    let mut unsupported = request();
    unsupported.scenario.initial_state.velocity_mps.x = 0.01;
    let temp = temp_root("unsupported");
    let output = temp.join("bundle");
    let artifact = run_nominal_direct_operational_flight(
        &unsupported,
        &BodyAwareTerminalPolicyV1::default(),
        &output,
    )
    .unwrap();
    assert_eq!(artifact.decision.status(), "unsupported");
    assert_unknown_field_rejected(&artifact);
    assert!(artifact.execution.is_none());
    assert!(!output.join("actions.json").exists());
    assert!(output.join("summary.json").exists() && output.join("report.html").exists());
    let before = std::fs::read(output.join("summary.json")).unwrap();
    assert!(
        run_nominal_direct_operational_flight(
            &unsupported,
            &BodyAwareTerminalPolicyV1::default(),
            &output
        )
        .is_err()
    );
    assert_eq!(before, std::fs::read(output.join("summary.json")).unwrap());
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn timings_do_not_affect_identity_but_outcome_and_guard_do() {
    let mut unsupported = request();
    unsupported.scenario.initial_state.velocity_mps.x = 0.01;
    let temp = temp_root("identity");
    let mut artifact = run_nominal_direct_operational_flight(
        &unsupported,
        &BodyAwareTerminalPolicyV1::default(),
        &temp.join("bundle"),
    )
    .unwrap();
    let identity = artifact.identity.clone();
    artifact.compute.generation_wall_time_us += 999;
    assert_eq!(
        identity,
        nominal_direct_operational_artifact_identity(&artifact).unwrap()
    );
    artifact
        .scope_non_claims
        .push("changed deterministic evidence".into());
    assert_ne!(
        identity,
        nominal_direct_operational_artifact_identity(&artifact).unwrap()
    );
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn invalid_replay_never_turns_a_safe_observation_into_completion() {
    let (program, witness) = selected();
    let (evidence, _) = execute_nominal_direct_operational_program(
        &request(),
        &BodyAwareTerminalPolicyV1::default(),
        witness,
        program,
        &mut NominalDirectFlightComputeV1::default(),
    )
    .unwrap();
    assert_eq!(
        outcome(&evidence.observed, &evidence.validity, false),
        NominalDirectOperationalOutcomeV1::ExecutionInvalid
    );
    // Synthetic classifier-only records, not alternate admitted flights.
    let mut bounded = evidence.observed.clone();
    let mut validity = evidence.validity.clone();
    bounded.stop = BoundedRunStopCauseV1::SafetyRejected;
    assert_eq!(
        outcome(&bounded, &validity, true),
        NominalDirectOperationalOutcomeV1::SafetyRejected
    );
    bounded.stop = BoundedRunStopCauseV1::MissionTerminal;
    validity.first_contact.as_mut().unwrap().classification = "crash".into();
    bounded.run.manifest.physical_outcome = pd_core::PhysicalOutcome::Crashed;
    bounded.run.manifest.mission_outcome = pd_core::MissionOutcome::FailedCrash;
    assert_eq!(
        outcome(&bounded, &validity, true),
        NominalDirectOperationalOutcomeV1::ExecutionInvalid
    );
    bounded.run.manifest.end_reason = pd_core::EndReason::Crash;
    assert_eq!(
        outcome(&bounded, &validity, true),
        NominalDirectOperationalOutcomeV1::UnsafeContact
    );
    validity.first_contact.as_mut().unwrap().classification = "stable_touchdown_off_target".into();
    bounded.run.manifest.physical_outcome = pd_core::PhysicalOutcome::LandedOffTarget;
    bounded.run.manifest.mission_outcome = pd_core::MissionOutcome::FailedOffTarget;
    bounded.run.manifest.end_reason = pd_core::EndReason::TouchdownOffTarget;
    assert_eq!(
        outcome(&bounded, &validity, true),
        NominalDirectOperationalOutcomeV1::OffTargetContact
    );
}
