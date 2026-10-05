use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use pd_core::{EvaluationGoal, ScenarioSpec, Vec2};
use pd_plan::waypoint_v2::{WaypointV2Policy, WaypointV2Stop};
use serde_json::Value;

use super::{
    capture_validation::validate_full_evidence_consistency,
    execution::{bounded_worker_count, request_for, source_path},
    input::{load_and_expand, repository_root},
    presentation::{detail_html, preview_destination},
    provenance::digest_expanded_inputs,
    *,
};
use crate::{
    WaypointV2FlightResult,
    evidence_io::{sha256_bytes, write_bytes_create_only_with_context, write_json_create_only},
    preflight_waypoint_v2_flight,
};

use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("pd-v2-pack-{label}-{}-{nonce}", std::process::id()))
}

#[test]
fn named_case_input_loader_is_input_only_and_rejects_unknown_ids() {
    let input =
        load_waypoint_v2_pack_case_input(Path::new(DEFAULT_PLANNER_PACK_PATH), "v2_ridge_early")
            .unwrap();
    assert_eq!(input.case_id, "v2_ridge_early");
    assert_eq!(input.scenario.id, "v2_ridge_early");
    assert_eq!(input.group, WaypointV2PackGroup::Ordinary);
    assert_eq!(input.expected_preflight, None);
    assert!(
        load_waypoint_v2_pack_case_input(Path::new(DEFAULT_PLANNER_PACK_PATH), "not-a-pack-case")
            .is_err()
    );
}

#[test]
fn preview_destination_is_research_only_without_writes() {
    let fixture = temp_dir("preview-guard");
    let outputs = fixture.join("outputs");
    let research = outputs.join("research");
    let source = research.join("capture");
    fs::create_dir_all(&source).expect("fixture");
    let allowed = research.join("new-preview");
    assert_eq!(
        preview_destination(&outputs, &source, &allowed).unwrap(),
        allowed
    );
    assert!(!allowed.exists(), "validation must not create anything");
    for rejected in [
        outputs.join("reports/new-preview"),
        outputs.join("eval/new-preview"),
        research.clone(),
        fixture.join("outside"),
        research.join("nested/../new-preview"),
        source.join("new-preview"),
    ] {
        assert!(
            preview_destination(&outputs, &source, &rejected).is_err(),
            "{}",
            rejected.display()
        );
        assert!(!rejected.exists() || rejected == research);
    }
    #[cfg(unix)]
    {
        let link = research.join("linked");
        std::os::unix::fs::symlink(fixture.join("missing"), &link).expect("symlink");
        assert!(preview_destination(&outputs, &source, &link.join("preview")).is_err());
    }
    fs::remove_dir_all(&fixture).expect("remove owned test fixture");
}

fn report_case(input: &WaypointV2PackInput, index: usize) -> WaypointV2BatchCase {
    let preflight_rejected = input.expected_preflight.as_deref() == Some("unsupported");
    let landed = index < 2;
    WaypointV2BatchCase {
        case_id: input.case_id.clone(),
        source_set: input.source_set.clone(),
        source_group: input.source_group.clone(),
        group: input.group.clone(),
        family: input.family.clone(),
        base_case_id: input.base_case_id.clone(),
        source_pad_id: input.source_pad_id.clone(),
        target_pad_id: input.target_pad_id.clone(),
        expected_preflight: input.expected_preflight.clone(),
        status: if preflight_rejected {
            "preflight_rejected"
        } else {
            "simulated"
        }
        .into(),
        outcome: Some(
            if landed {
                "landed_on_target"
            } else {
                "no_clearing"
            }
            .into(),
        ),
        planning_stop: Some(if preflight_rejected {
            "unsupported".into()
        } else if landed {
            "landed".into()
        } else {
            "no_clearing".into()
        }),
        reason: None,
        correction_count: Some(if index == 1 { 2 } else { 0 }),
        initial_nominal_terrain_blocked: Some(index >= 11),
        integrity_passed: Some(true),
        final_source_replay_passed: Some(!preflight_rejected),
        physical_outcome: if preflight_rejected {
            None
        } else if landed {
            Some("landed_on_target".into())
        } else {
            Some("flying".into())
        },
        mission_outcome: if preflight_rejected {
            None
        } else if landed {
            Some("success".into())
        } else {
            Some("in_progress".into())
        },
        planning_s: Some(index as f64),
        execution_s: Some(0.1),
        replay_s: Some(0.2),
        input_path: source_path(input).into(),
        scenario_path: format!("runs/{}/scenario.json", input.case_id),
        flight_path: format!("runs/{}/flight.json", input.case_id),
        summary_path: format!("runs/{}/summary.json", input.case_id),
        rich_report_path: (!preflight_rejected)
            .then(|| format!("runs/{}/report.html", input.case_id)),
        annotated_report_path: format!("runs/{}/index.html", input.case_id),
        artifact_sha256: BTreeMap::new(),
        error: None,
    }
}

fn full_evidence_fixture() -> (
    WaypointV2PackInput,
    WaypointV2FlightResult,
    serde_json::Value,
) {
    let input = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH))
        .unwrap()
        .inputs
        .remove(0);
    let final_state = pd_core::SimulationStateSnapshotV1 {
        sim_time_s: 0.1,
        physics_step: 12,
        position_m: Vec2::default(),
        velocity_mps: Vec2::default(),
        attitude_rad: 0.0,
        angular_rate_radps: 0.0,
        fuel_kg: 99.0,
        held_command: pd_core::Command::idle(),
        physical_outcome: pd_core::PhysicalOutcome::LandedOnTarget,
        mission_outcome: pd_core::MissionOutcome::Success,
        end_reason: pd_core::EndReason::TouchdownOnTarget,
        min_touchdown_clearance_m: 0.1,
        min_hull_clearance_m: 0.2,
        max_speed_mps: 0.0,
        max_abs_attitude_rad: 0.0,
        max_abs_angular_rate_radps: 0.0,
        waypoint_sequence_passed: 0,
        waypoint_sequence_first_failure_index: None,
        waypoint_handoff_window_index: None,
    };
    let summary = pd_core::RunSummary {
        fuel_remaining_kg: 99.0,
        fuel_used_kg: 1.0,
        min_touchdown_clearance_m: 0.1,
        min_hull_clearance_m: 0.2,
        landing: Some(pd_core::LandingRunSummary::default()),
        ..pd_core::RunSummary::default()
    };
    let manifest = pd_core::RunManifest {
        schema_version: 1,
        scenario_id: input.scenario.id.clone(),
        scenario_name: input.scenario.name.clone(),
        scenario_seed: 0,
        scenario_tags: input.scenario.tags.clone(),
        controller_id: "synthetic-test-controller".into(),
        physics_hz: input.scenario.sim.physics_hz,
        controller_hz: input.scenario.sim.controller_hz,
        sim_time_s: 0.1,
        physics_steps: 12,
        controller_updates: 1,
        physical_outcome: pd_core::PhysicalOutcome::LandedOnTarget,
        mission_outcome: pd_core::MissionOutcome::Success,
        end_reason: pd_core::EndReason::TouchdownOnTarget,
        summary,
    };
    let ordinary = crate::LocalClearingOrdinaryEvidenceV1 {
        final_state: final_state.clone(),
        incoming_contact: None,
        actions: vec![pd_core::ActionLogEntry {
            sim_time_s: 0.0,
            physics_step: 0,
            controller_update_index: 1,
            command: pd_core::Command::idle(),
        }],
        events: Vec::new(),
        samples: Vec::new(),
    };
    let result = WaypointV2FlightResult {
        policy: WaypointV2Policy::revision_3(),
        input_identity: "synthetic-consistency-test".into(),
        planning_stop: WaypointV2Stop::Landed,
        reason: None,
        correction_count: 0,
        initial_nominal_terrain_blocked: false,
        integrity_passed: true,
        physical_outcome: Some(pd_core::PhysicalOutcome::LandedOnTarget),
        mission_outcome: Some(pd_core::MissionOutcome::Success),
        absolute_deadline_physics_step: None,
        cycles: Vec::new(),
        segments: Vec::new(),
        ordinary_flight: Some(ordinary),
        final_source_replay_passed: true,
        manifest: Some(manifest.clone()),
        failed_local_row: None,
        timings: crate::WaypointV2Timings::default(),
    };
    let compact = serde_json::json!({
        "run_summary": {
            "minimum_clearance": {
                "touchdown_m": manifest.summary.min_touchdown_clearance_m,
                "hull_m": manifest.summary.min_hull_clearance_m,
                "landing": manifest.summary.landing,
            },
            "fuel": {
                "remaining_kg": manifest.summary.fuel_remaining_kg,
                "used_kg": manifest.summary.fuel_used_kg,
            },
            "endpoint": {
                "physics_step": manifest.physics_steps,
                "sim_time_s": manifest.sim_time_s,
                "physical_outcome": manifest.physical_outcome,
                "mission_outcome": manifest.mission_outcome,
                "end_reason": manifest.end_reason,
            }
        }
    });
    (input, result, compact)
}

#[test]
fn full_flight_manifest_and_compact_endpoint_are_cross_bound() {
    let (input, result, compact) = full_evidence_fixture();
    validate_full_evidence_consistency(&input, &result, &compact).unwrap();

    let mut wrong_compact_clock = compact.clone();
    wrong_compact_clock["run_summary"]["endpoint"]["physics_step"] = serde_json::json!(13);
    assert!(validate_full_evidence_consistency(&input, &result, &wrong_compact_clock).is_err());

    let mut wrong_action_count = result.clone();
    wrong_action_count
        .manifest
        .as_mut()
        .unwrap()
        .controller_updates += 1;
    assert!(validate_full_evidence_consistency(&input, &wrong_action_count, &compact).is_err());

    let mut wrong_final_outcome = result;
    wrong_final_outcome
        .ordinary_flight
        .as_mut()
        .unwrap()
        .final_state
        .physical_outcome = pd_core::PhysicalOutcome::Flying;
    assert!(validate_full_evidence_consistency(&input, &wrong_final_outcome, &compact).is_err());
}

#[test]
fn zero_command_flight_requires_zero_clock_and_only_initial_sample() {
    let (input, mut result, mut compact) = full_evidence_fixture();
    let ordinary = result.ordinary_flight.as_mut().unwrap();
    ordinary.actions.clear();
    ordinary.samples = vec![pd_core::SampleRecord {
        sim_time_s: 0.0,
        physics_step: 0,
        observation: pd_core::Observation {
            sim_time_s: 0.0,
            physics_step: 0,
            position_m: Vec2::default(),
            velocity_mps: Vec2::default(),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
            mass_kg: 100.0,
            fuel_kg: 99.0,
            gravity_mps2: 1.62,
            target_dx_m: 0.0,
            height_above_target_m: 0.0,
            target_surface_y_m: 0.0,
            target_pad_half_width_m: 2.0,
            touchdown_clearance_m: 0.0,
            min_hull_clearance_m: 0.0,
        },
        held_command: pd_core::Command::idle(),
    }];
    ordinary.final_state.physics_step = 0;
    ordinary.final_state.sim_time_s = 0.0;
    let manifest = result.manifest.as_mut().unwrap();
    manifest.physics_steps = 0;
    manifest.controller_updates = 0;
    manifest.sim_time_s = 0.0;
    compact["run_summary"]["endpoint"]["physics_step"] = serde_json::json!(0);
    compact["run_summary"]["endpoint"]["sim_time_s"] = serde_json::json!(0.0);
    validate_full_evidence_consistency(&input, &result, &compact).unwrap();

    let mut nonzero_clock = result.clone();
    nonzero_clock
        .ordinary_flight
        .as_mut()
        .unwrap()
        .final_state
        .physics_step = 1;
    nonzero_clock.manifest.as_mut().unwrap().physics_steps = 1;
    let mut nonzero_compact = compact.clone();
    nonzero_compact["run_summary"]["endpoint"]["physics_step"] = serde_json::json!(1);
    assert!(validate_full_evidence_consistency(&input, &nonzero_clock, &nonzero_compact).is_err());

    let mut missing_initial_sample = result;
    missing_initial_sample
        .ordinary_flight
        .as_mut()
        .unwrap()
        .samples
        .clear();
    assert!(validate_full_evidence_consistency(&input, &missing_initial_sample, &compact).is_err());
}

#[test]
fn native_pack_expands_exactly_44_route_free_inputs_in_source_order() {
    let expanded = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH)).unwrap();
    assert_eq!(expanded.inputs.len(), 44);
    assert_eq!(expanded.inputs[0].case_id, "v2_clear_685");
    assert_eq!(expanded.inputs[31].case_id, "v2_diag_other_vehicle");
    assert_eq!(expanded.inputs[32].case_id, "fresh_clear_flat_805");
    assert_eq!(
        expanded.inputs[43].case_id,
        "fresh_compound_downhill_plateau_805"
    );
    let counts = expanded
        .inputs
        .iter()
        .fold(BTreeMap::new(), |mut counts, input| {
            *counts.entry(input.group.as_str()).or_insert(0usize) += 1;
            counts
        });
    assert_eq!(counts["clear"], 11);
    assert_eq!(counts["ordinary"], 16);
    assert_eq!(counts["additional_terrain"], 9);
    assert_eq!(counts["diagnostic"], 8);
    assert!(expanded.inputs.iter().all(|input| {
        input.scenario.id == input.case_id
            && input.scenario.mission.transfer_route.is_none()
            && matches!(
                input.scenario.mission.goal,
                EvaluationGoal::LandingOnPad { .. }
            )
    }));
    let all_cases = expanded
        .inputs
        .iter()
        .enumerate()
        .map(|(index, input)| report_case(input, index))
        .collect::<Vec<_>>();
    let diagnostic = expanded
        .inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.expected_preflight.is_some())
        .collect::<Vec<_>>();
    assert_eq!(diagnostic.len(), 2);
    for (index, input) in diagnostic {
        let preflight =
            preflight_waypoint_v2_flight(&request_for(input), &WaypointV2Policy::revision_3());
        assert_eq!(preflight.rejection, Some(WaypointV2Stop::Unsupported));
        assert!(!preflight.simulation_created);
        let result =
            crate::run_waypoint_v2_flight(&request_for(input), &WaypointV2Policy::revision_3())
                .unwrap();
        assert!(result.manifest.is_none());
        assert!(result.ordinary_flight.is_none());
        let page = detail_html(input, &all_cases[index], &result, index, &all_cases, None).unwrap();
        assert!(page.contains("No simulator trajectory or rich flight report was created."));
        assert!(!page.contains("<svg"));
        assert!(page.contains("Previous case"));
        assert!(page.contains("Next case"));
    }
    assert_eq!(
        digest_expanded_inputs(&expanded.inputs).unwrap(),
        expanded.input_identity.rust_typed_expanded_inputs_sha256
    );
}

#[test]
fn worker_count_clamps_to_cases_and_rejects_zero() {
    assert_eq!(bounded_worker_count(1).unwrap(), 1);
    assert_eq!(bounded_worker_count(44).unwrap(), 44);
    assert_eq!(bounded_worker_count(128).unwrap(), 44);
    assert!(bounded_worker_count(0).is_err());
}

#[test]
#[ignore = "compares typed expansion with retained accepted archive captures"]
fn expanded_scenarios_match_retained_archive_values_exactly() {
    const LEGACY_CAPTURE: &str =
        "outputs/research/waypoint_v2_airborne_integration_20261002/final_hardened_policy_3_a";
    let repo = repository_root();
    let expanded = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH)).unwrap();
    for input in &expanded.inputs {
        let run_root = if input.source_set == "practical_suite" {
            repo.join(LEGACY_CAPTURE).join("runs")
        } else {
            repo.join("outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs")
        };
        let archive: ScenarioSpec = serde_json::from_slice(
            &fs::read(run_root.join(&input.case_id).join("scenario.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            input.scenario, archive,
            "typed scenario drift for {}",
            input.case_id
        );
    }
}

#[test]
fn invalid_policy_fails_before_reserving_capture_root() {
    let temp = temp_dir("invalid-policy");
    fs::create_dir_all(&temp).unwrap();
    let original = fs::read(repository_root().join(DEFAULT_PLANNER_PACK_PATH)).unwrap();
    let mut value: Value = serde_json::from_slice(&original).unwrap();
    value["policy_version"] = Value::from(4);
    let bad_pack = temp.join("bad-pack.json");
    fs::write(&bad_pack, serde_json::to_vec(&value).unwrap()).unwrap();
    let capture = temp.join("capture");
    assert!(run_waypoint_v2_pack(&bad_pack, &capture, 1).is_err());
    assert!(!capture.exists());
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn summary_separates_not_applicable_diagnostic_replay_from_finite_misses() {
    let expanded = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH)).unwrap();
    let cases = expanded
        .inputs
        .iter()
        .enumerate()
        .map(|(index, input)| report_case(input, index))
        .collect::<Vec<_>>();
    let summary = summarize(&cases).unwrap();
    assert_eq!(summary.valid_landing_count, 2);
    assert_eq!(summary.direct_landing_count, 1);
    assert_eq!(summary.corrected_landing_count, 1);
    assert_eq!(summary.unsupported_count, 2);
    assert_eq!(summary.non_landing_count, 34);
    assert_eq!(summary.diagnostic_landing_count, 0);
    assert_eq!(summary.diagnostic_non_landing_count, 6);
    assert_eq!(summary.final_source_replay_passed_count, 42);
    assert_eq!(summary.final_source_replay_failed_count, 0);
    assert_eq!(summary.planning_stops["no_clearing"], 40);
    assert_eq!(summary.integrity_passed_count, 44);
}

#[test]
fn report_only_rejects_missing_raw_input_before_writing_any_derived_pages() {
    let temp = temp_dir("tamper");
    fs::create_dir_all(&temp).unwrap();
    let expanded = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH)).unwrap();
    let pack_bytes = expanded.pack_bytes.clone();
    let inputs_bytes = serde_json::to_vec_pretty(&expanded.inputs).unwrap();
    write_bytes_create_only_with_context(
        &temp.join("pack.json"),
        &pack_bytes,
        "create-only artifact",
    )
    .unwrap();
    write_bytes_create_only_with_context(
        &temp.join("expanded-inputs.json"),
        &inputs_bytes,
        "create-only artifact",
    )
    .unwrap();
    let cases = expanded
        .inputs
        .iter()
        .enumerate()
        .map(|(index, input)| report_case(input, index))
        .collect::<Vec<_>>();
    let summary = summarize(&cases).unwrap();
    let report = WaypointV2BatchReport {
        schema_id: WAYPOINT_V2_BATCH_SCHEMA_ID.into(),
        pack_id: expanded.definition.id.clone(),
        name: expanded.definition.name.clone(),
        status: "completed".into(),
        policy_version: expanded.definition.policy_version,
        case_count: cases.len(),
        cases,
        summary,
        input_identity: expanded.input_identity,
        pack_snapshot_sha256: sha256_bytes(&pack_bytes).unwrap(),
        expanded_inputs_snapshot_sha256: sha256_bytes(&inputs_bytes).unwrap(),
        pack_snapshot_path: "pack.json".into(),
        expanded_inputs_snapshot_path: "expanded-inputs.json".into(),
        provenance: WaypointV2BatchProvenance {
            source_before: WaypointV2SourceState {
                git_commit: None,
                git_dirty: None,
                rust_source_tree_sha256: "test".into(),
                executable_sha256: "test".into(),
            },
            source_after: None,
            unchanged_during_capture: None,
        },
    };
    write_json_create_only(&temp.join("summary.json"), &report).unwrap();
    assert!(validated_waypoint_v2_batch(&temp).is_err());
    assert!(render_waypoint_v2_site_pages(&temp, "/eval/capture/").is_err());
    assert!(render_waypoint_v2_batch(&temp).is_err());
    assert!(!temp.join("index.html").exists());
    assert!(!temp.join("report.html").exists());
    assert!(render_waypoint_v2_batch_preview(&temp, &temp.join("preview")).is_err());
    assert!(!temp.join("preview").exists());
    fs::remove_dir_all(temp).unwrap();
}
