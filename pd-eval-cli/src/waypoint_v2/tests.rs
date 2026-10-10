use super::execution::ledger_updates;
use crate::{clearing_body_reserve_query, nominal_body_reserve_query};
use pd_core::{BoundedRunStopCauseV1, SimulationState};

use super::*;

/// Explicitly opted-in numerical regression against a validated retained
/// capture. Runs the current planner without creating/publishing artifacts;
/// only the three nondeterministic flight wall-time fields are excluded.
#[test]
#[ignore = "set PD_V2_PARITY_CAPTURE to a retained accepted V2 capture"]
fn retained_capture_numerical_parity() {
    let capture = std::path::PathBuf::from(
        std::env::var_os("PD_V2_PARITY_CAPTURE").expect("PD_V2_PARITY_CAPTURE is required"),
    );
    // Cargo runs library tests from the crate directory. Resolve relative
    // evidence roots from the workspace; absolute paths remain absolute.
    let capture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(capture);
    let acceptance = crate::waypoint_v2_acceptance::check_waypoint_v2_acceptance(&capture)
        .expect("validate retained capture before using it as a baseline");
    assert!(acceptance.passed, "retained baseline must be accepted");
    let batch = crate::waypoint_v2_pack::validated_waypoint_v2_batch(&capture).unwrap();
    let (inputs, identity) = crate::waypoint_v2_pack::check_default_planner_v2_binding().unwrap();
    assert_eq!(batch.input_identity, identity);
    assert_eq!(batch.cases.len(), inputs.len());

    for input in inputs {
        let case = batch
            .cases
            .iter()
            .find(|case| case.case_id == input.case_id)
            .unwrap();
        let mut expected: WaypointV2FlightResult =
            serde_json::from_slice(&std::fs::read(capture.join(&case.flight_path)).unwrap())
                .unwrap();
        let request = WaypointDirectNominalDirectGenerationRequest {
            probe_id: input.scenario.id.clone(),
            scenario: input.scenario,
            source_pad_id: input.source_pad_id,
            target_pad_id: input.target_pad_id,
            policy: crate::WaypointDirectNominalDirectGenerationPolicyV1::default(),
        };
        let mut actual = run_waypoint_v2_flight(&request, &expected.policy).unwrap();
        actual.timings = WaypointV2Timings::default();
        expected.timings = WaypointV2Timings::default();
        let difference = crate::first_json_difference(
            &serde_json::to_value(actual).unwrap(),
            &serde_json::to_value(expected).unwrap(),
            "$",
        );
        assert!(difference.is_none(), "{}: {difference:?}", case.case_id);
        eprintln!("{}: exact numerical parity", case.case_id);
    }
}

fn request() -> WaypointDirectNominalDirectGenerationRequest {
    crate::test_inputs::planner_request("v2_clear_845")
}

#[test]
fn nominal_attempt_accounting_shares_status_and_rejection_counts() {
    let request = request();
    let context = RunContext::from_scenario(&request.scenario).unwrap();
    let state = new_ordinary(&context).unwrap().evidence.final_state;
    let mut cycle = WaypointV2Cycle {
        cycle_index: 0,
        current_state: state,
        nominal_search_identity: String::new(),
        nominal_proposal_identity: None,
        nominal_peak_com_height_m: None,
        nominal_attempt_status_counts: BTreeMap::new(),
        nominal_rejection_reason_counts: BTreeMap::new(),
        nominal_updates: Vec::new(),
        audit: None,
        fixed_consumed_prefix_proven: false,
        decision: WaypointV2CycleDecision::NoNominal,
        conflict_state: None,
        conflict_incoming_contact: None,
        local_search: None,
    };

    record_nominal_attempt(&mut cycle, "rejected", Some("no executable witness"));
    record_nominal_attempt(&mut cycle, "rejected", Some("no executable witness"));
    record_nominal_attempt(&mut cycle, "selected", None);
    record_nominal_rejection_reason(&mut cycle, "seed had no entry screen");

    assert_eq!(cycle.nominal_attempt_status_counts["rejected"], 2);
    assert_eq!(cycle.nominal_attempt_status_counts["selected"], 1);
    assert_eq!(
        cycle.nominal_rejection_reason_counts["no executable witness"],
        2
    );
    assert_eq!(
        cycle.nominal_rejection_reason_counts["seed had no entry screen"],
        1
    );
}

#[test]
fn nominal_audit_error_retains_the_pre_audit_pending_cycle() {
    let request = request();
    let mut session = WaypointV2Session::start(request, WaypointV2Policy::revision_3()).unwrap();
    // Nominal generation reads the original request. Make only the session's
    // audit context invalid so generation succeeds and audit fails after a
    // proposal has been selected.
    session
        .flight
        .as_mut()
        .expect("supported session")
        .context
        .sim
        .controller_hz = 119;

    let progress = session.advance_piece().unwrap();
    assert!(matches!(
        progress,
        WaypointV2SessionProgress::Terminal {
            planning_stop: WaypointV2Stop::ImplementationError,
            ..
        }
    ));
    let result = session.result();
    assert_eq!(result.cycles.len(), 1);
    let retained = &result.cycles[0];
    assert_eq!(retained.decision, WaypointV2CycleDecision::NoNominal);
    assert!(retained.nominal_search_identity.is_empty());
    assert!(retained.nominal_attempt_status_counts.is_empty());
    assert!(retained.nominal_rejection_reason_counts.is_empty());
    assert!(retained.nominal_proposal_identity.is_none());
    assert!(retained.nominal_updates.is_empty());
    assert!(retained.audit.is_none());
    assert!(!retained.fixed_consumed_prefix_proven);
}

#[test]
fn correction_certificate_replay_failure_does_not_commit_proof_flags() {
    let request = request();
    let mut session =
        WaypointV2Session::start(request.clone(), WaypointV2Policy::default()).unwrap();
    assert!(matches!(
        session.advance_piece().unwrap(),
        WaypointV2SessionProgress::Terminal {
            planning_stop: WaypointV2Stop::Landed,
            ..
        }
    ));
    let flight = session.flight.as_mut().expect("supported session");
    let mut certificate = flight.live.clone();
    certificate.evidence.final_state.physics_step += 1;
    let mut local = WaypointV2LocalSearch {
        entries: Vec::new(),
        row_count: 0,
        boundary_count: 0,
        accepted_row_count: 0,
        row_status_counts: BTreeMap::new(),
        row_stop_reason_counts: BTreeMap::new(),
        boundary_status_counts: BTreeMap::new(),
        selected: None,
        certificate_state: Some(certificate.evidence.final_state.clone()),
        handoff_source_replay_passed: false,
        certificate_source_replay_passed: false,
        row_diagnostics: Vec::new(),
        early_exit: None,
    };

    let error = flight
        .prove_correction_sources(&request, &certificate, &[], &mut local)
        .expect_err("the corrupted private certificate must fail its source proof");
    assert!(
        error
            .to_string()
            .contains("live state and accumulated evidence endpoint differ"),
        "{error:#}"
    );
    assert!(local.certificate_state.is_some());
    assert!(!local.handoff_source_replay_passed);
    assert!(!local.certificate_source_replay_passed);
    assert_eq!(flight.result.correction_count, 0);
    assert_eq!(
        flight.result.cycles.last().unwrap().decision,
        WaypointV2CycleDecision::Direct
    );
}

#[test]
fn terminal_before_next_cycle_does_not_reuse_previous_handoff_cycle_index() {
    let request = request();
    let context = RunContext::from_scenario(&request.scenario).unwrap();
    let initial = new_ordinary(&context).unwrap().evidence.final_state;
    let previous_cycle = WaypointV2Cycle {
        cycle_index: 0,
        current_state: initial,
        nominal_search_identity: "prior-correction".into(),
        nominal_proposal_identity: None,
        nominal_peak_com_height_m: None,
        nominal_attempt_status_counts: BTreeMap::new(),
        nominal_rejection_reason_counts: BTreeMap::new(),
        nominal_updates: Vec::new(),
        audit: None,
        fixed_consumed_prefix_proven: false,
        decision: WaypointV2CycleDecision::LocalCleared,
        conflict_state: None,
        conflict_incoming_contact: None,
        local_search: None,
    };
    assert_eq!(
        terminal_piece_index(std::slice::from_ref(&previous_cycle), 0),
        Some(0)
    );
    assert_eq!(
        terminal_piece_index(&[previous_cycle], 120),
        None,
        "a deadline at a later handoff precedes the next cycle"
    );
}

#[test]
fn local_search_checkpoint_retains_completed_rows_after_later_error() {
    let request = request();
    let context = RunContext::from_scenario(&request.scenario).unwrap();
    let initial = new_ordinary(&context).unwrap().evidence.final_state;
    let mut pending_cycle = Some(WaypointV2Cycle {
        cycle_index: 0,
        current_state: initial,
        nominal_search_identity: "checkpoint-test".into(),
        nominal_proposal_identity: None,
        nominal_peak_com_height_m: None,
        nominal_attempt_status_counts: BTreeMap::new(),
        nominal_rejection_reason_counts: BTreeMap::new(),
        nominal_updates: Vec::new(),
        audit: None,
        fixed_consumed_prefix_proven: false,
        decision: WaypointV2CycleDecision::TerrainBlocked,
        conflict_state: None,
        conflict_incoming_contact: None,
        local_search: None,
    });
    let local_search = WaypointV2LocalSearch {
        entries: vec![WaypointV2Entry {
            entry_id: "first-entry".into(),
            physics_step: 24,
            admitted: false,
            reason: Some("test row rejected".into()),
        }],
        row_count: 1,
        boundary_count: 2,
        accepted_row_count: 0,
        row_status_counts: BTreeMap::from([("rejected".into(), 1)]),
        row_stop_reason_counts: BTreeMap::from([("test".into(), 1)]),
        boundary_status_counts: BTreeMap::from([("blocked".into(), 2)]),
        selected: None,
        certificate_state: None,
        handoff_source_replay_passed: false,
        certificate_source_replay_passed: false,
        row_diagnostics: Vec::new(),
        early_exit: None,
    };

    checkpoint_local_search(&mut pending_cycle, &local_search);
    // A subsequent query/search error consumes this retained pending cycle.
    let retained = pending_cycle.take().expect("pending cycle retained");
    let retained_search = retained
        .local_search
        .expect("completed diagnostics retained");
    assert_eq!(retained_search.row_count, 1);
    assert_eq!(retained_search.boundary_count, 2);
    assert_eq!(retained_search.entries[0].entry_id, "first-entry");
    assert_eq!(retained.nominal_updates, Vec::new());
    assert_eq!(retained.decision, WaypointV2CycleDecision::TerrainBlocked);
}

#[test]
fn preflight_is_input_only_and_disallows_even_empty_authored_route() {
    let mut r = request();
    let p = WaypointV2Policy::default();
    assert!(preflight_waypoint_v2_flight(&r, &p).supported);
    r.scenario.mission.transfer_route = Some(pd_core::TransferRouteSpec {
        source_pad_id: r.source_pad_id.clone(),
        target_pad_id: r.target_pad_id.clone(),
        waypoints: Vec::new(),
        route_angle_deg: 0.0,
        route_radius_m: 900.0,
    });
    let preflight = preflight_waypoint_v2_flight(&r, &p);
    assert_eq!(preflight.rejection, Some(WaypointV2Stop::Unsupported));
    assert!(!preflight.simulation_created);
    let mut session = WaypointV2Session::start(r, p).unwrap();
    let first = session.advance_piece().unwrap();
    assert_eq!(
        first,
        WaypointV2SessionProgress::Terminal {
            piece_index: None,
            entry_physics_step: None,
            planning_stop: WaypointV2Stop::Unsupported,
            correction_count: 0,
            physics_step: None,
        }
    );
    let result = session.finish().unwrap().clone();
    assert_eq!(session.advance_piece().unwrap(), first);
    assert_eq!(session.finish().unwrap(), &result);
    assert!(result.ordinary_flight.is_none());
    assert!(!result.final_source_replay_passed);
}

#[test]
fn historical_policy_execution_is_rejected_without_creating_a_simulation() {
    let request = request();
    for policy in [
        WaypointV2Policy::revision_1(),
        WaypointV2Policy::revision_2(),
    ] {
        let preflight = preflight_waypoint_v2_flight(&request, &policy);
        assert_eq!(preflight.rejection, Some(WaypointV2Stop::Unsupported));
        assert!(preflight.reason.as_deref().unwrap().contains("retired"));
        assert!(!preflight.simulation_created);
        let mut session = WaypointV2Session::start(request.clone(), policy).unwrap();
        assert!(matches!(
            session.advance_piece().unwrap(),
            WaypointV2SessionProgress::Terminal {
                planning_stop: WaypointV2Stop::Unsupported,
                ..
            }
        ));
        let result = session.finish().unwrap();
        assert!(result.cycles.is_empty());
        assert!(result.ordinary_flight.is_none());
    }
}

#[test]
fn failed_finalization_updates_terminal_and_retains_partial_capture() {
    let request = request();
    let mut session =
        WaypointV2Session::start(request.clone(), WaypointV2Policy::revision_3()).unwrap();
    let (entry_step, endpoint_step) = {
        let flight = session.flight.as_mut().expect("supported session");
        let cycle_entry = flight.live.evidence.final_state.clone();
        flight.result.cycles.push(WaypointV2Cycle {
            cycle_index: 0,
            current_state: cycle_entry.clone(),
            nominal_search_identity: "injected-finalization-test".into(),
            nominal_proposal_identity: None,
            nominal_peak_com_height_m: None,
            nominal_attempt_status_counts: BTreeMap::new(),
            nominal_rejection_reason_counts: BTreeMap::new(),
            nominal_updates: Vec::new(),
            audit: None,
            fixed_consumed_prefix_proven: false,
            decision: WaypointV2CycleDecision::NoNominal,
            conflict_state: None,
            conflict_incoming_contact: None,
            local_search: None,
        });
        flight.result.planning_stop = WaypointV2Stop::NoNominal;
        flight.live.evidence.final_state.physics_step += 1;
        flight.live.evidence.final_state.sim_time_s += 1.0 / f64::from(request.policy.physics_hz);
        (
            cycle_entry.physics_step,
            flight.live.evidence.final_state.physics_step,
        )
    };
    session.terminal_stop = Some(WaypointV2Stop::NoNominal);
    session.terminal_progress = Some(WaypointV2SessionProgress::Terminal {
        piece_index: Some(0),
        entry_physics_step: Some(entry_step),
        planning_stop: WaypointV2Stop::NoNominal,
        correction_count: 0,
        physics_step: Some(endpoint_step),
    });

    let total_started = Instant::now();
    let result = session.finish().unwrap().clone();
    assert_eq!(result.planning_stop, WaypointV2Stop::ImplementationError);
    assert!(!result.integrity_passed);
    assert!(!result.final_source_replay_passed);
    assert!(result.manifest.is_none());
    assert_eq!(
        result
            .ordinary_flight
            .as_ref()
            .unwrap()
            .final_state
            .physics_step,
        endpoint_step,
        "failed proof retains the inconsistent partial endpoint for diagnosis"
    );
    let terminal = session.advance_piece().unwrap();
    assert!(matches!(
        terminal,
        WaypointV2SessionProgress::Terminal {
            planning_stop: WaypointV2Stop::ImplementationError,
            ..
        }
    ));

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "pd-v2-finalization-failure-{}-{nonce}",
        std::process::id()
    ));
    crate::reserve_waypoint_v2_flight_output(&root).unwrap();
    let progress = crate::WaypointV2CliProgressV1 {
        schema_id: crate::waypoint_v2_bundle::WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID.into(),
        entries: vec![crate::WaypointV2CliProgressEntryV1 {
            elapsed_s: 0.0,
            progress: terminal,
        }],
        finalization_elapsed_s: 0.0,
    };
    crate::write_waypoint_v2_cli_bundle(&request, &result, &root, &progress, total_started)
        .expect("failed flight evidence is still written");
    let saved: WaypointV2FlightResult =
        serde_json::from_slice(&std::fs::read(root.join("flight.json")).unwrap()).unwrap();
    assert_eq!(saved.planning_stop, WaypointV2Stop::ImplementationError);
    assert_eq!(
        saved.ordinary_flight.unwrap().final_state.physics_step,
        endpoint_step
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn nonterrain_terminal_rejection_is_not_an_implementation_error() {
    let r = request();
    let context = RunContext::from_scenario(&r.scenario).unwrap();
    let search = evaluate_canonical_initial_direct(&r).unwrap();
    let mut audit = audit_canonical_initial_direct(
        &context,
        &source_pad_input(&context, &r.source_pad_id).unwrap(),
        &search.selected.unwrap(),
        5.0,
    )
    .unwrap();
    assert!(audit.passed);
    audit.passed = false;
    audit.safe_target_contact = false;
    assert!(!fixed_outcome(&audit, true).unwrap());
    assert!(fixed_outcome(&audit, false).is_err());
    audit.ordinary_neutral_parity = false;
    assert!(fixed_outcome(&audit, true).is_err());
}
#[test]
fn clear_control_has_no_corrections_and_full_replay() {
    let request = request();
    let mut session =
        WaypointV2Session::start(request.clone(), WaypointV2Policy::default()).unwrap();
    assert!(
        session.finish().is_err(),
        "active flight cannot be finalized"
    );
    let progress = session.advance_piece().unwrap();
    assert!(matches!(
        progress,
        WaypointV2SessionProgress::Terminal {
            planning_stop: WaypointV2Stop::Landed,
            ..
        }
    ));
    let before_repeat = session.result().clone();
    assert_eq!(session.advance_piece().unwrap(), progress);
    assert_eq!(session.result(), &before_repeat);
    let result = session.finish().unwrap().clone();
    assert_eq!(session.finish().unwrap(), &result);
    assert!(session.is_finalized());
    assert_eq!(session.advance_piece().unwrap(), progress);
    assert_eq!(
        result.planning_stop,
        WaypointV2Stop::Landed,
        "{:?}",
        result.reason
    );
    assert_eq!(result.correction_count, 0);
    assert!(!result.initial_nominal_terrain_blocked);
    assert!(result.integrity_passed && result.final_source_replay_passed);
    let context = RunContext::from_scenario(&request.scenario).unwrap();
    let mut live = new_ordinary(&context).unwrap();
    let updates = ledger_updates(&result.segments);
    let endpoint = result
        .ordinary_flight
        .as_ref()
        .unwrap()
        .final_state
        .physics_step;
    advance_ordinary(&context, &mut live, &updates, endpoint).unwrap();
    assert!(prove_accumulated(&request, &context, &live, &result.segments, false, 9600).is_ok());
    let mut tampered = live.clone();
    tampered.evidence.actions[0].command.throttle_frac = 0.0;
    assert!(
        prove_accumulated(&request, &context, &tampered, &result.segments, false, 9600).is_err()
    );
    let mut tampered = live.clone();
    tampered
        .evidence
        .incoming_contact
        .as_mut()
        .unwrap()
        .state
        .fuel_kg += 1.0;
    assert!(
        prove_accumulated(&request, &context, &tampered, &result.segments, false, 9600).is_err()
    );
    let mut tampered = result.segments.clone();
    tampered[0].kind = WaypointV2SegmentKind::LocalCorrection;
    assert!(prove_accumulated(&request, &context, &live, &tampered, false, 9600).is_err());
}
#[test]
fn synthetic_corridor_queries_keep_launch_and_terminal_exemptions_separate() {
    let request = request();
    let context = RunContext::from_scenario(&request.scenario).unwrap();
    let mut state = SimulationState::new(&context).unwrap();
    // Synthetic poses test query policy only, not a flown route.
    assert_eq!(
        nominal_body_reserve_query(&context, &request, &state, "upright", true)
            .unwrap()
            .1,
        0.0
    );
    assert_eq!(
        nominal_body_reserve_query(&context, &request, &state, "upright", false)
            .unwrap()
            .1,
        5.0
    );
    assert_eq!(
        clearing_body_reserve_query(&context, &request, &state, "local", false)
            .unwrap()
            .1,
        5.0
    );
    state.position_m = pd_core::Vec2::new(
        context.target_pad.center_x_m,
        context.target_pad.surface_y_m + 10.0,
    );
    state.velocity_mps = pd_core::Vec2::new(1.0, -1.0);
    assert_eq!(
        nominal_body_reserve_query(&context, &request, &state, "terminal_bridge", false)
            .unwrap()
            .1,
        0.0
    );
    assert_eq!(
        nominal_body_reserve_query(&context, &request, &state, "ballistic_coast", false)
            .unwrap()
            .1,
        5.0
    );
    assert_eq!(
        clearing_body_reserve_query(&context, &request, &state, "local", false)
            .unwrap()
            .1,
        5.0
    );
}
#[test]
fn odd_actual_contact_proof_has_no_endpoint_padding_or_extra_command() {
    let mut request = crate::test_inputs::archived_obstacle_request("fresh_flat_control_span_900");
    request.probe_id = "odd_contact_proof".into();
    let mut points = request.scenario.world.terrain.points().to_vec();
    points.extend([
        pd_core::Vec2::new(-540.0, 0.0),
        pd_core::Vec2::new(-450.0, 315.0),
        pd_core::Vec2::new(-360.0, 0.0),
    ]);
    points.sort_by(|a, b| a.x.total_cmp(&b.x));
    request.scenario.world.terrain = pd_core::TerrainDefinition::Heightfield { points_m: points };
    let context = RunContext::from_scenario(&request.scenario).unwrap();
    let proposal = evaluate_canonical_initial_direct(&request)
        .unwrap()
        .selected
        .unwrap();
    let audit = audit_canonical_initial_direct(
        &context,
        &source_pad_input(&context, &request.source_pad_id).unwrap(),
        &proposal,
        5.0,
    )
    .unwrap();
    assert_eq!(audit.final_state.physics_step, 2121);
    let initial = new_ordinary(&context).unwrap();
    let query = query_to(&context, &initial, &proposal.updates, 2121).unwrap();
    let mut segments = Vec::new();
    append_segment(
        &mut segments,
        WaypointV2SegmentKind::InitialNominal,
        &proposal.identity,
        &initial.evidence.final_state,
        &query,
        consumed(&proposal.updates, 2121),
    )
    .unwrap();
    let proof = prove_accumulated(&request, &context, &query, &segments, true, 9600).unwrap();
    assert_eq!(proof.stop, BoundedRunStopCauseV1::MissionTerminal);
    assert_eq!(proof.final_state.physics_step, 2121);
    assert_eq!(proof.limits.command_coverage_end_physics_step, 2122);
    assert_eq!(proof.run.actions.len(), 1061);
    assert_eq!(proof.run.actions.last().unwrap().physics_step, 2120);
    assert_eq!(proof.final_state, audit.final_state);
    assert_eq!(proof.incoming_contact, audit.incoming_contact);
    assert!(fixed_outcome(&audit, true).unwrap());
}
#[test]
fn tracked_policy3_plateau_preserves_actual_handoff_across_multiple_cycles() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let input = crate::waypoint_v2_pack::load_waypoint_v2_pack_case_input(
        &repo.join(crate::waypoint_v2_pack::DEFAULT_PLANNER_PACK_PATH),
        "v2_plateau_reference_900",
    )
    .unwrap();
    let request = WaypointDirectNominalDirectGenerationRequest {
        probe_id: input.scenario.id.clone(),
        scenario: input.scenario,
        source_pad_id: input.source_pad_id,
        target_pad_id: input.target_pad_id,
        policy: crate::WaypointDirectNominalDirectGenerationPolicyV1::default(),
    };
    let policy = WaypointV2Policy::revision_3();
    let context = RunContext::from_scenario(&request.scenario).unwrap();
    let deadline = pd_plan::waypoint_v2::original_deadline(
        context.sim.max_time_s,
        request.policy.analytical_policy.mission_budget_s(),
    )
    .unwrap();
    let mut session = WaypointV2Session::start(request.clone(), policy.clone()).unwrap();
    let mut previous_handoff = new_ordinary(&context).unwrap().evidence.final_state;
    let mut previous_certificate = None;
    let mut handoff_count = 0usize;
    let mut saw_piece_origin_differ_from_entry = false;

    loop {
        match session.advance_piece().unwrap() {
            WaypointV2SessionProgress::Handoff {
                piece_index,
                correction_count,
                entry_physics_step,
                handoff_physics_step,
            } => {
                assert_eq!(piece_index, handoff_count);
                assert_eq!(correction_count as usize, handoff_count + 1);
                assert_eq!(entry_physics_step, previous_handoff.physics_step);
                let result = session.result();
                let cycle = result.cycles.get(piece_index).unwrap();
                assert_eq!(cycle.cycle_index, piece_index);
                assert_eq!(cycle.current_state, previous_handoff);
                if let Some(certificate) = &previous_certificate {
                    assert_ne!(
                        cycle.current_state, *certificate,
                        "the next cycle starts from actual H, not the private certificate"
                    );
                }
                let local = cycle.local_search.as_ref().unwrap();
                assert_eq!(local.entries.len(), 4);
                assert!(
                    local
                        .entries
                        .iter()
                        .all(|entry| !entry.entry_id.starts_with("fallback_"))
                );
                let selected = local.selected.as_ref().unwrap();
                assert_eq!(
                    selected.schedule.entry_physics_step,
                    selected.entry_state.physics_step
                );
                let actual = local.actual_handoff_state().unwrap();
                assert!(selected.schedule.handoff_physics_step >= handoff_physics_step);
                assert_eq!(actual.physics_step, handoff_physics_step);
                assert!(selected.schedule.entry_physics_step < handoff_physics_step);
                assert!(selected.schedule.continuation_end_physics_step <= deadline);
                assert_eq!(result.absolute_deadline_physics_step, Some(deadline));
                assert_eq!(
                    cycle.current_state.sim_time_s,
                    cycle.current_state.physics_step as f64 / f64::from(context.sim.physics_hz)
                );
                assert_eq!(
                    actual.sim_time_s,
                    handoff_physics_step as f64 / f64::from(context.sim.physics_hz)
                );
                assert!(actual.fuel_kg <= cycle.current_state.fuel_kg);
                assert!(actual.fuel_kg >= 0.0);
                saw_piece_origin_differ_from_entry |=
                    cycle.current_state.physics_step != selected.schedule.entry_physics_step;

                let certificate = local.certificate_state.as_ref().unwrap();
                let expected_certificate = local
                    .early_exit
                    .as_ref()
                    .filter(|e| e.disposition == WaypointV2EarlyExitDisposition::Committed)
                    .and_then(|e| e.continuation_end_state.as_ref())
                    .unwrap_or(&selected.continuation_end_state);
                assert_eq!(certificate, expected_certificate);
                assert!(certificate.physics_step > handoff_physics_step);
                assert_ne!(certificate, actual);
                assert!(local.handoff_source_replay_passed);
                assert!(local.certificate_source_replay_passed);

                let correction_segment = result
                    .segments
                    .iter()
                    .rev()
                    .find(|segment| segment.kind == WaypointV2SegmentKind::LocalCorrection)
                    .unwrap();
                assert_eq!(
                    correction_segment.start_physics_step,
                    selected.schedule.entry_physics_step
                );
                assert_eq!(correction_segment.end_physics_step, handoff_physics_step);
                assert_eq!(correction_segment.entry_state, selected.entry_state);
                assert_eq!(&correction_segment.end_state, actual);
                assert_eq!(
                    correction_segment.updates.len() as u64,
                    pd_plan::waypoint_v2::command_count(
                        correction_segment.start_physics_step,
                        correction_segment.end_physics_step
                    )
                    .unwrap()
                );
                assert_eq!(&result.segments.last().unwrap().end_state, actual);
                assert_ne!(result.segments.last().unwrap().end_state, *certificate);
                assert!(result.segments.iter().all(|segment| {
                    segment.end_physics_step <= deadline
                        && segment.entry_state.physics_step == segment.start_physics_step
                        && segment.end_state.physics_step == segment.end_physics_step
                        && segment.entry_state.sim_time_s
                            == segment.start_physics_step as f64 / f64::from(context.sim.physics_hz)
                        && segment.end_state.sim_time_s
                            == segment.end_physics_step as f64 / f64::from(context.sim.physics_hz)
                }));
                assert!(
                    result
                        .segments
                        .windows(2)
                        .all(|segments| { segments[0].end_state == segments[1].entry_state })
                );

                previous_handoff = actual.clone();
                previous_certificate = Some(certificate.clone());
                handoff_count += 1;
            }
            WaypointV2SessionProgress::Terminal {
                planning_stop: WaypointV2Stop::Landed,
                ..
            } => break,
            progress => panic!("unexpected policy-3 progress: {progress:?}"),
        }
    }

    let result = session.finish().unwrap();
    assert_eq!(result.policy, policy);
    assert!(handoff_count >= 2, "expected multiple actual H handoffs");
    assert!(
        saw_piece_origin_differ_from_entry,
        "piece origin was conflated with E"
    );
    assert_eq!(result.correction_count as usize, handoff_count);
    assert_eq!(result.absolute_deadline_physics_step, Some(deadline));
    assert_eq!(
        result.planning_stop,
        WaypointV2Stop::Landed,
        "{:?}",
        result.reason
    );
    assert_eq!(
        result.physical_outcome,
        Some(pd_core::PhysicalOutcome::LandedOnTarget)
    );
    assert_eq!(
        result.mission_outcome,
        Some(pd_core::MissionOutcome::Success)
    );
    assert!(result.integrity_passed);
    assert!(result.final_source_replay_passed);

    // The optional query is a single record on an already selected row, never
    // a list of new candidate boundaries. Exercise its saved/runtime bindings
    // on this maintained fixture, not on measured diagnostic subjects.
    early_exit::validate_records(result).unwrap();
    let index = result
        .cycles
        .iter()
        .position(|c| {
            c.local_search
                .as_ref()
                .and_then(|s| s.early_exit.as_ref())
                .is_some_and(|e| e.disposition == WaypointV2EarlyExitDisposition::Committed)
        })
        .expect("fixture exercises early commit");
    let local = result.cycles[index].local_search.as_ref().unwrap();
    let proposal = local.selected.as_ref().unwrap();
    let exit = local.early_exit.as_ref().unwrap();
    assert!(exit.query_state.as_ref().unwrap().physics_step < proposal.handoff_state.physics_step);
    let updates = ledger_updates(&result.segments);
    let entry = execution::query_to(
        &context,
        &new_ordinary(&context).unwrap(),
        &updates,
        proposal.entry_state.physics_step,
    )
    .unwrap();
    let mut no_boundary = proposal.clone();
    no_boundary.schedule.handoff_physics_step = exit.query_state.as_ref().unwrap().physics_step;
    let (record, ready) = early_exit::prepare(&context, &entry, &no_boundary).unwrap();
    assert_eq!(
        record.disposition,
        WaypointV2EarlyExitDisposition::NoEarlierBoundary
    );
    assert!(ready.is_none() && record.nominal_search.is_none());

    let mut insufficient_reserve = proposal.clone();
    insufficient_reserve
        .trajectory
        .iter_mut()
        .find(|s| s.state == *exit.query_state.as_ref().unwrap())
        .unwrap()
        .body_clearance_m = 4.0;
    let (record, ready) = early_exit::prepare(&context, &entry, &insufficient_reserve).unwrap();
    assert_eq!(
        record.disposition,
        WaypointV2EarlyExitDisposition::UnsupportedContinuation
    );
    assert!(ready.is_none() && record.nominal_search.is_none());
    let mut unsupported = context.clone();
    unsupported.target_pad.center_x_m = exit.query_state.as_ref().unwrap().position_m.x;
    let (record, ready) = early_exit::prepare(&unsupported, &entry, proposal).unwrap();
    assert_eq!(
        record.disposition,
        WaypointV2EarlyExitDisposition::UnsupportedContinuation
    );
    assert!(ready.is_none() && record.nominal_search.is_none());

    let mut broken_trace = proposal.clone();
    broken_trace
        .trajectory
        .iter_mut()
        .find(|s| s.state == *exit.query_state.as_ref().unwrap())
        .unwrap()
        .state
        .fuel_kg += 1.0;
    assert!(early_exit::prepare(&context, &entry, &broken_trace).is_err());

    let mut forged = result.clone();
    forged.cycles[index]
        .local_search
        .as_mut()
        .unwrap()
        .early_exit
        .as_mut()
        .unwrap()
        .witness_handoff_physics_step -= 2;
    assert!(early_exit::validate_records(&forged).is_err());
    forged = result.clone();
    forged.cycles[index + 1].nominal_updates[0]
        .command
        .throttle_frac = 0.123;
    assert!(early_exit::validate_records(&forged).is_err());
    forged = result.clone();
    forged.cycles[index]
        .local_search
        .as_mut()
        .unwrap()
        .early_exit
        .as_mut()
        .unwrap()
        .disposition = WaypointV2EarlyExitDisposition::ClearReady;
    assert!(early_exit::validate_records(&forged).is_err());
    forged.integrity_passed = false;
    early_exit::validate_records(&forged).unwrap();

    let mut historical = serde_json::to_value(local).unwrap();
    historical.as_object_mut().unwrap().remove("early_exit");
    let old: WaypointV2LocalSearch = serde_json::from_value(historical.clone()).unwrap();
    assert!(old.early_exit.is_none());
    assert_eq!(serde_json::to_value(old).unwrap(), historical);
    let mut no_nominal = result.clone();
    let record = no_nominal.cycles[index]
        .local_search
        .as_mut()
        .unwrap()
        .early_exit
        .as_mut()
        .unwrap();
    record.disposition = WaypointV2EarlyExitDisposition::NoNominal;
    record.nominal_search.as_mut().unwrap().selected = None;
    record.audit = None;
    early_exit::validate_records(&no_nominal).unwrap();
    let mut blocked = result.clone();
    let violation = result.cycles[index]
        .audit
        .as_ref()
        .unwrap()
        .clearance_scan
        .first_violation
        .clone()
        .unwrap();
    let record = blocked.cycles[index]
        .local_search
        .as_mut()
        .unwrap()
        .early_exit
        .as_mut()
        .unwrap();
    record.disposition = WaypointV2EarlyExitDisposition::TerrainBlocked;
    let audit = record.audit.as_mut().unwrap();
    audit.passed = false;
    audit.clearance_scan.first_violation = Some(violation);
    early_exit::validate_records(&blocked).unwrap();
    blocked.cycles[index]
        .local_search
        .as_mut()
        .unwrap()
        .early_exit
        .as_mut()
        .unwrap()
        .disposition = WaypointV2EarlyExitDisposition::Committed;
    assert!(early_exit::validate_records(&blocked).is_err());

    let mut mismatch = WaypointV2Session::start(request.clone(), policy).unwrap();
    mismatch.flight.as_mut().unwrap().queued_nominal = Some(early_exit::CheckedNominal {
        state: exit.query_state.as_ref().unwrap().clone(),
        search: exit.nominal_search.as_ref().unwrap().clone(),
        audit: exit.audit.as_ref().unwrap().clone(),
    });
    let flight = mismatch.flight.as_mut().unwrap();
    let mut cycle = result.cycles[index + 1].clone();
    assert!(
        flight
            .build_acquisition_nominal(&mut cycle)
            .err()
            .expect("queued origin mismatch must fail closed")
            .to_string()
            .contains("actual piece state/deadline")
    );
    let report = crate::waypoint_v2_report::project_flight(&request.scenario, result).unwrap();
    assert!(
        serde_json::to_string(&report)
            .unwrap()
            .contains("query evidence, not an executed waypoint")
    );
}
