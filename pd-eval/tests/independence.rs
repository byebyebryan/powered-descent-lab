use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use pd_core::{BoundedRunLimitsV1, Command, Observation, ScenarioSpec};
use pd_eval::*;
use pd_missions::*;
use pd_run::{Agent, AgentDescriptorV1, AgentFrame, MissionView};

struct External {
    mutate: Option<PathBuf>,
    fail: bool,
}

impl Agent for External {
    fn descriptor(&self) -> AgentDescriptorV1 {
        AgentDescriptorV1 {
            schema_version: 1,
            implementation: "external-test-v1".to_owned(),
            configuration: serde_json::json!({}),
            checkpoint_sha256: None,
        }
    }
    fn update(&mut self, _: MissionView<'_>, _: &Observation) -> Result<AgentFrame, String> {
        if let Some(file) = self.mutate.take() {
            std::fs::write(file, b"changed source").unwrap();
        }
        if self.fail {
            return Err("missing decision".to_owned());
        }
        Ok(AgentFrame::command_only(Command::idle()))
    }
}

fn temporary_parent() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "pd-neutral-eval-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    root
}

#[test]
fn v2_executor_consumes_only_admitted_physical_inputs_and_binds_excluded_proposals() {
    for corrupt_excluded in [false, true] {
        let parent = temporary_parent();
        let collection = parent.join("v2");
        let manifest = write_collection_v2(
            &collection,
            CollectionRequestV2::broad(MissionRecipeV2::default(), 981, 32),
        )
        .unwrap();
        let excluded = manifest
            .receipts
            .iter()
            .find(|r| r.disposition == SelectionDispositionV2::InsufficientChallenge)
            .unwrap()
            .proposal
            .as_ref()
            .unwrap();
        let entry = &manifest.entries[0];
        let resolved = load_collection_mission_v2(&collection, &manifest, entry).unwrap();
        let request = CollectionEvaluationRequestV1 {
            schema_version: 1,
            manifest_sha256: sha256(&std::fs::read(collection.join("manifest.json")).unwrap()),
            agent: External {
                mutate: None,
                fail: false,
            }
            .descriptor(),
            attempt_indices: vec![entry.index, entry.index],
        };
        let mut calls = 0;
        let result = evaluate_collection_with_executor(
            &collection,
            &parent.join("evaluation"),
            request,
            &mut |scenario, scope| {
                calls += 1;
                assert_eq!(scenario, &resolved.scenario);
                assert_eq!(scope.inputs, vec![collection.join(&entry.file)]);
                assert!(!scenario.metadata.contains_key("labels"));
                if corrupt_excluded && calls == 1 {
                    std::fs::write(collection.join(&excluded.file), b"changed excluded input")
                        .unwrap();
                }
                Err(AttemptExecutionErrorV1::Unsupported(
                    "test executor deliberately does not fly".into(),
                ))
            },
            &EvidenceScope {
                sources: vec![],
                inputs: vec![],
            },
        )
        .unwrap();
        assert_eq!(result.attempts.len(), 2);
        assert_eq!(result.unique_scheduled_missions, 1);
        assert_eq!(calls, 2);
        assert_eq!(result.integrity_passed, !corrupt_excluded);
        assert_eq!(
            result.evidence_before.input_files.len(),
            manifest
                .receipts
                .iter()
                .filter(|r| r.proposal.is_some())
                .count()
                + 3
        );
        assert!(
            result
                .attempts
                .iter()
                .all(|a| matches!(a.disposition, AttemptDispositionV1::Unsupported { .. }))
        );
        std::fs::remove_dir_all(parent).unwrap();
    }
}

fn landed_scenario() -> ScenarioSpec {
    let mut scenario: ScenarioSpec = serde_json::from_str(include_str!(
        "../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap();
    scenario.initial_state.position_m = pd_core::Vec2::new(0.0, 5.0);
    scenario.initial_state.velocity_mps = pd_core::Vec2::new(0.0, 0.0);
    scenario
}

fn limits() -> BoundedRunLimitsV1 {
    BoundedRunLimitsV1 {
        command_coverage_end_physics_step: 120,
        hard_end_physics_step: 120,
    }
}

#[test]
fn external_agent_capture_replays_and_renders_shared_rich_report() {
    let parent = temporary_parent();
    let source = parent.join("source.rs");
    std::fs::write(&source, b"independent source").unwrap();
    let evidence = EvidenceScope {
        sources: vec![source],
        inputs: Vec::new(),
    };
    let result = evaluate_agent(
        &landed_scenario(),
        &mut External {
            mutate: None,
            fail: false,
        },
        limits(),
        &evidence,
    )
    .unwrap();
    assert!(result.verified_target_landing);
    assert!(result.replay.complete_episode_verified);
    let capture = parent.join("capture");
    save_evaluation(&capture, &result).unwrap();
    assert_eq!(load_verified_evaluation(&capture).unwrap(), result);
    assert!(save_evaluation(&capture, &result).is_err());
    let configuration = serde_json::to_value(&result.execution.descriptor).unwrap();
    let html = pd_report::render_run_report_with_flight_annotations(
        &result.scenario,
        Some(&configuration),
        &result.execution.episode.run.manifest,
        &result.execution.episode.run.events,
        &result.execution.episode.run.samples,
        &result.execution.controller_updates,
        Some(&result.execution.performance),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(html.contains("external-test-v1"));
    assert!(html.contains("Plotly"));
    assert!(html.contains("controllerSpec"));
    assert!(html.contains("trajectory"));
    let report = pd_report::render_agent_run_report(
        &result.scenario,
        &result.execution,
        pd_report::AgentReportEvidenceV1 {
            integrity_passed: result.integrity_passed,
            physical_prefix_verified: result.replay.physical_prefix_verified,
            complete_episode_verified: result.replay.complete_episode_verified,
            verified_target_landing: result.verified_target_landing,
        },
        pd_report::agent_capture_navigation(),
    )
    .unwrap();
    assert!(report.contains("Verified target landing: true"));
    assert!(report.contains("seal.json"));
    let mut altered = std::fs::read(capture.join("evaluation.json")).unwrap();
    altered.push(b' ');
    std::fs::write(capture.join("evaluation.json"), altered).unwrap();
    assert!(load_verified_evaluation(&capture).is_err());
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn source_drift_and_policy_failure_cannot_claim_verified_landing() {
    let parent = temporary_parent();
    let source = parent.join("source.rs");
    std::fs::write(&source, b"before").unwrap();
    let evidence = EvidenceScope {
        sources: vec![source.clone()],
        inputs: Vec::new(),
    };
    let drift = evaluate_agent(
        &landed_scenario(),
        &mut External {
            mutate: Some(source),
            fail: false,
        },
        limits(),
        &evidence,
    )
    .unwrap();
    assert!(!drift.integrity_passed);
    assert!(!drift.verified_target_landing);
    assert!(drift.replay.complete_episode_verified);
    save_evaluation(&parent.join("drift"), &drift).unwrap();
    let failed = evaluate_agent(
        &landed_scenario(),
        &mut External {
            mutate: None,
            fail: true,
        },
        limits(),
        &evidence,
    )
    .unwrap();
    assert!(!failed.verified_target_landing);
    assert!(failed.replay.physical_prefix_verified);
    assert!(!failed.replay.complete_episode_verified);
    assert!(failed.execution.episode.run.actions.is_empty());
    save_evaluation(&parent.join("failed"), &failed).unwrap();
    let mut invented = drift;
    invented.verified_target_landing = true;
    assert!(validate_evaluation(&invented).is_err());
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn collection_schedule_keeps_initialization_failures_and_explicit_repeats() {
    let parent = temporary_parent();
    let collection = parent.join("inputs");
    let manifest = write_collection(
        &collection,
        CollectionRequestV1 {
            schema_version: 1,
            role: CollectionRoleV1::Development,
            recipe: MissionRecipeV1::default(),
            seed: 42,
            indices: vec![0, 1],
        },
    )
    .unwrap();
    let agent = External {
        mutate: None,
        fail: false,
    };
    let request = CollectionEvaluationRequestV1 {
        schema_version: 1,
        manifest_sha256: sha256(&std::fs::read(collection.join("manifest.json")).unwrap()),
        agent: agent.descriptor(),
        attempt_indices: vec![0, 1, 0],
    };
    let mut count = 0;
    let result = evaluate_collection(
        &collection,
        &parent.join("evaluation"),
        request,
        &mut || {
            count += 1;
            if count == 2 {
                return Err("checkpoint unavailable".to_owned());
            }
            Ok(Box::new(External {
                mutate: None,
                fail: false,
            }) as Box<dyn Agent>)
        },
        &EvidenceScope::default(),
    )
    .unwrap();
    assert_eq!(result.unique_scheduled_missions, 2);
    assert_eq!(result.attempts.len(), 3);
    assert!(matches!(
        result.attempts[1].disposition,
        AttemptDispositionV1::InitializationFailed { .. }
    ));
    assert!(matches!(
        result.attempts[2].disposition,
        AttemptDispositionV1::Executed { .. }
    ));
    assert_eq!(read_collection_usage(&collection).unwrap().len(), 1);
    assert_eq!(verify_collection(&collection).unwrap(), manifest);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn offline_executor_keeps_unsupported_and_binds_only_selected_input_per_episode() {
    let parent = temporary_parent();
    let collection = parent.join("collection");
    let manifest = write_collection(
        &collection,
        CollectionRequestV1 {
            schema_version: 1,
            role: CollectionRoleV1::Development,
            recipe: MissionRecipeV1::default(),
            seed: 81,
            indices: vec![0, 1],
        },
    )
    .unwrap();
    let descriptor = External {
        mutate: None,
        fail: false,
    }
    .descriptor();
    let request = CollectionEvaluationRequestV1 {
        schema_version: 1,
        manifest_sha256: sha256(&std::fs::read(collection.join("manifest.json")).unwrap()),
        agent: descriptor,
        attempt_indices: vec![0, 1],
    };
    let mut ordinal = 0;
    let result = evaluate_collection_with_executor(
        &collection,
        &parent.join("campaign"),
        request,
        &mut |scenario, scope| {
            ordinal += 1;
            assert_eq!(scope.inputs.len(), 1);
            assert!(scope.inputs[0].is_file());
            if ordinal == 1 {
                record_collection_usage(
                    &collection,
                    "concurrent-exposure",
                    CollectionExposureV1::Tuning,
                    "Concurrent exposure is recorded without changing immutable inputs",
                )
                .unwrap();
            }
            if ordinal == 2 {
                return Err(AttemptExecutionErrorV1::Unsupported(
                    "approach does not support this input".to_owned(),
                ));
            }
            let ctx = pd_core::RunContext::from_scenario(scenario).unwrap();
            evaluate_agent(
                scenario,
                &mut External {
                    mutate: None,
                    fail: false,
                },
                pd_run::scenario_horizon_limits(&ctx).unwrap(),
                scope,
            )
            .map_err(AttemptExecutionErrorV1::Execution)
        },
        &EvidenceScope {
            sources: vec![],
            inputs: vec![collection.clone()],
        },
    )
    .unwrap();
    assert!(result.integrity_passed);
    assert_eq!(result.attempts.len(), 2);
    assert!(matches!(
        result.attempts[1].disposition,
        AttemptDispositionV1::Unsupported { .. }
    ));
    let first = load_verified_evaluation(&parent.join("campaign/attempt-000000")).unwrap();
    assert_eq!(first.evidence_before.input_files.len(), 1);
    assert_eq!(verify_collection(&collection).unwrap(), manifest);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn campaign_detects_unselected_input_drift_without_quadratic_episode_binding() {
    let parent = temporary_parent();
    let collection = parent.join("collection");
    let manifest = write_collection(
        &collection,
        CollectionRequestV1 {
            schema_version: 1,
            role: CollectionRoleV1::Development,
            recipe: MissionRecipeV1::default(),
            seed: 82,
            indices: vec![0, 1],
        },
    )
    .unwrap();
    let request = CollectionEvaluationRequestV1 {
        schema_version: 1,
        manifest_sha256: sha256(&std::fs::read(collection.join("manifest.json")).unwrap()),
        agent: External {
            mutate: None,
            fail: false,
        }
        .descriptor(),
        attempt_indices: vec![0, 1],
    };
    let other = collection.join(&manifest.entries[1].file);
    let result = evaluate_collection_with_executor(
        &collection,
        &parent.join("campaign"),
        request,
        &mut |scenario, scope| {
            let ctx = pd_core::RunContext::from_scenario(scenario).unwrap();
            evaluate_agent(
                scenario,
                &mut External {
                    mutate: Some(other.clone()),
                    fail: false,
                },
                pd_run::scenario_horizon_limits(&ctx).unwrap(),
                scope,
            )
            .map_err(AttemptExecutionErrorV1::Execution)
        },
        &EvidenceScope::default(),
    )
    .unwrap();
    assert!(!result.integrity_passed);
    assert!(matches!(
        result.attempts[1].disposition,
        AttemptDispositionV1::InputInvalid { .. }
    ));
    let first = load_verified_evaluation(&parent.join("campaign/attempt-000000")).unwrap();
    assert!(first.integrity_passed);
    assert_eq!(first.evidence_before.input_files.len(), 1);
    std::fs::remove_dir_all(parent).unwrap();
}
