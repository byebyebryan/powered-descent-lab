use super::*;

fn context_and_live() -> (RunContext, SimulationState) {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let (_, request) = crate::load_nominal_direct_operational_fresh_inputs(repo)
        .unwrap()
        .remove(0);
    let context = RunContext::from_scenario(&request.scenario).unwrap();
    let mut live = SimulationState::new(&context).unwrap();
    // Synthetic structural tests only. Flight acceptance reconstructs full prefixes.
    live.physics_step = 120;
    live.sim_time_s = 1.0;
    live.position_m = Vec2::new(context.target_pad.center_x_m - 400.0, 200.0);
    live.velocity_mps = Vec2::new(25.0, 10.0);
    live.fuel_kg -= 10.0;
    (context, live)
}

#[test]
fn acquisition_runtime_is_terrain_blind_first_valid_and_preserves_origin() {
    let (mut context, live) = context_and_live();
    let before = SimulationStateSnapshotV1::from_state(&live);
    let baseline = evaluate_airborne_acquisition_direct(&context, &live, 9600).unwrap();
    assert!(
        baseline.selected.is_some(),
        "nonvacuous synthetic comparison"
    );
    assert!(baseline.attempts.len() <= 3 && baseline.seeds.len() <= 13);
    assert_eq!(baseline.attempts.last().unwrap().status, "nominal_proposal");
    assert!(
        baseline.attempts[..baseline.attempts.len() - 1]
            .iter()
            .all(|a| a.status == "rejected")
    );
    let mut twin_live = live.clone();
    twin_live.min_hull_clearance_m = -999.0;
    twin_live.min_touchdown_clearance_m = -999.0;
    let mut points = context.world.terrain.points().to_vec();
    let x = (live.position_m.x + context.target_pad.center_x_m) * 0.5;
    points.retain(|p| p.x < x - 10.0 || p.x > x + 10.0);
    points.extend([
        Vec2::new(x - 10.0, 0.0),
        Vec2::new(x, 1000.0),
        Vec2::new(x + 10.0, 0.0),
    ]);
    points.sort_by(|a, b| a.x.total_cmp(&b.x));
    context.world.terrain = pd_core::TerrainDefinition::Heightfield { points_m: points };
    let twin = evaluate_airborne_acquisition_direct(&context, &twin_live, 9600).unwrap();
    assert_eq!(baseline, twin);
    assert_eq!(SimulationStateSnapshotV1::from_state(&live), before);
    let p = baseline.selected.unwrap();
    assert_eq!(p.incoming_state, AirborneFlightStateV1::from_live(&live));
    assert_eq!(p.absolute_deadline_physics_step, 9600);
    assert!(p.end_state.fuel_kg < live.fuel_kg);
}

#[test]
fn acquisition_runtime_rejects_nonfinite_and_preserves_supported_family() {
    let (context, mut live) = context_and_live();
    live.position_m.x = f64::NAN;
    assert!(evaluate_airborne_acquisition_direct(&context, &live, 9600).is_err());
    let (_, mut live) = context_and_live();
    live.held_command.throttle_frac = 0.5;
    let search = evaluate_airborne_acquisition_direct(&context, &live, 9600).unwrap();
    assert!(
        search.unsupported_reason.is_some()
            && search.seeds.is_empty()
            && search.attempts.is_empty()
    );
    live.held_command = Command::idle();
    live.sim_time_s = 0.0;
    assert!(
        evaluate_airborne_acquisition_direct(&context, &live, 9600)
            .unwrap()
            .unsupported_reason
            .is_some()
    );
}

#[test]
fn acquisition_audit_rejects_rehashed_gaps_phase_and_origin_tampering_before_stepping() {
    let (context, live) = context_and_live();
    let p = evaluate_airborne_acquisition_direct(&context, &live, 9600)
        .unwrap()
        .selected
        .unwrap();
    let before = SimulationStateSnapshotV1::from_state(&live);
    let mut variants = Vec::new();
    let mut gap = p.clone();
    gap.updates.remove(0);
    variants.push(gap);
    let mut phase = p.clone();
    phase.updates[0].phase = "source_bridge".into();
    variants.push(phase);
    let mut boundary = p.clone();
    boundary.terminal_entry_physics_step += 2;
    variants.push(boundary);
    let mut fuel = p.clone();
    fuel.incoming_state.fuel_kg += 1.0;
    variants.push(fuel);
    let mut deadline = p.clone();
    deadline.absolute_deadline_physics_step = live.physics_step;
    variants.push(deadline);
    let mut command = p.clone();
    command.updates[0].command.throttle_frac = 2.0;
    variants.push(command);
    for mut bad in variants {
        bad.identity = proposal_identity(&bad).unwrap();
        assert!(audit_airborne_acquisition_proposal(&context, &live, &bad, 5.0).is_err());
        assert_eq!(SimulationStateSnapshotV1::from_state(&live), before);
    }
}

#[test]
fn acquisition_program_covers_odd_endpoint_and_omits_zero_length_phases() {
    let (context, live) = context_and_live();
    let mut p = evaluate_airborne_acquisition_direct(&context, &live, 9600)
        .unwrap()
        .selected
        .unwrap();
    // Structural endpoint check only, not an invented landing claim.
    p.planned_end_physics_step = live.physics_step + 3;
    p.end_state.physics_step = p.planned_end_physics_step;
    p.end_state.sim_time_s = p.planned_end_physics_step as f64 / 120.0;
    p.acquisition_turn_end_physics_step = live.physics_step;
    p.acquisition_end_physics_step = live.physics_step;
    p.terminal_entry_physics_step = live.physics_step;
    p.updates.truncate(2);
    for u in &mut p.updates {
        u.phase = "terminal_bridge".into();
    }
    assert!(validate_program(&p).is_ok());
    p.updates.pop();
    assert!(validate_program(&p).is_err());
}

#[test]
fn acquisition_audit_uses_consumed_phase_not_legacy_coast_clock() {
    let (context, live) = context_and_live();
    let p = evaluate_airborne_acquisition_direct(&context, &live, 9600)
        .unwrap()
        .selected
        .unwrap();
    let audit = audit_airborne_acquisition_proposal(&context, &live, &p, 5.0).unwrap();
    assert!(audit.ordinary_neutral_parity);
    if let Some(v) = audit.clearance_scan.first_violation {
        let phase = if v.physics_step == live.physics_step {
            &p.updates[0].phase
        } else {
            &p.updates
                .iter()
                .rev()
                .find(|u| u.physics_step < v.physics_step)
                .unwrap()
                .phase
        };
        assert_eq!(&v.phase, phase);
    }
}

#[test]
fn acquisition_audit_phase_change_boundary_belongs_to_previous_command() {
    let (mut context, mut live) = context_and_live();
    let mut p = evaluate_airborne_acquisition_direct(&context, &live, 9600)
        .unwrap()
        .selected
        .unwrap();
    let mut points = context.world.terrain.points().to_vec();
    for point in &mut points {
        point.y = context.target_pad.surface_y_m;
    }
    context.world.terrain = pd_core::TerrainDefinition::Heightfield { points_m: points };
    live.velocity_mps.y = 0.0;
    p.incoming_state = AirborneFlightStateV1::from_live(&live);
    p.acquisition_turn_end_physics_step = live.physics_step + 2;
    p.acquisition_end_physics_step = live.physics_step + 4;
    p.terminal_entry_physics_step = live.physics_step + 6;
    p.planned_end_physics_step = live.physics_step + 8;
    p.updates = PHASES
        .iter()
        .enumerate()
        .map(|(i, phase)| FlightProgramUpdateV1 {
            physics_step: live.physics_step + 2 * i as u64,
            phase: (*phase).into(),
            command: Command {
                throttle_frac: if i == 0 || i == 2 { 0.0 } else { 0.2 },
                target_attitude_rad: 0.0,
            },
        })
        .collect();
    let mut state = live.clone();
    let mut first_clearance = 0.0;
    let mut second_clearance = 0.0;
    while state.physics_step < p.planned_end_physics_step {
        if state.physics_step.is_multiple_of(2) {
            state.set_command(
                p.updates[((state.physics_step - live.physics_step) / 2) as usize].command,
            );
        }
        state.step_physics_and_classify_contact(&context);
        let clearance = body_clearance(
            &context,
            &state,
            body_aabb(&state, &context.vehicle.geometry),
        )
        .unwrap();
        if state.physics_step == live.physics_step + 1 {
            first_clearance = clearance;
        }
        if state.physics_step == live.physics_step + 2 {
            second_clearance = clearance;
        }
    }
    assert!(first_clearance > second_clearance);
    p.end_state = AirborneFlightStateV1::from_live(&state);
    p.dynamics_identity = dynamics_identity(&context).unwrap();
    p.identity = proposal_identity(&p).unwrap();
    let audit = audit_airborne_acquisition_proposal(
        &context,
        &live,
        &p,
        (first_clearance + second_clearance) * 0.5,
    )
    .unwrap();
    let violation = audit.clearance_scan.first_violation.unwrap();
    assert_eq!(violation.physics_step, live.physics_step + 2);
    assert_eq!(violation.phase, "nominal_acquisition_turn");
    assert!(audit.commands_match && audit.ordinary_neutral_parity);
}

#[test]
fn acquisition_realizer_integrity_failure_is_not_a_finite_retry() {
    let (context, live) = context_and_live();
    let search = evaluate_airborne_acquisition_direct(&context, &live, 9600).unwrap();
    let proposal = search.selected.unwrap();
    let mut seed = search
        .seeds
        .into_iter()
        .find(|s| s.seed_id == proposal.seed_id)
        .unwrap();
    let entry = seed.entry_screens[proposal.entry_index].clone();
    seed.turn_physics_ticks += 2;
    assert!(matches!(
        realize_free_space_witness(&context, &live, 9600, &seed, &entry),
        Err(WitnessMaterializationFailure::Integrity(_))
    ));
    assert!(
        evaluate_airborne_acquisition_direct(&context, &live, 122)
            .unwrap()
            .selected
            .is_none()
    );
}

#[test]
fn acquisition_witness_retry_statuses_fail_closed() {
    let (context, live) = context_and_live();
    let search = evaluate_airborne_acquisition_direct(&context, &live, 9600).unwrap();
    let p = search.selected.unwrap();
    let seed = search
        .seeds
        .iter()
        .find(|s| s.seed_id == p.seed_id)
        .unwrap();
    let mut witness = realize_free_space_witness(
        &context,
        &live,
        9600,
        seed,
        &seed.entry_screens[p.entry_index],
    )
    .unwrap()
    .witness;
    assert!(executable_witness(&witness).unwrap());
    witness.status = "unrecognized_rejection".into();
    assert!(executable_witness(&witness).is_err());
    witness.status = "finite_miss".into();
    assert!(!executable_witness(&witness).unwrap());
    witness.endpoint_state_agreement = false;
    assert!(executable_witness(&witness).is_err());
    witness.status = "finite_backend_rejection".into();
    assert!(!executable_witness(&witness).unwrap());
    witness.independent_replay_passed = false;
    assert!(executable_witness(&witness).is_err());
}

#[test]
fn acquisition_audit_unexplained_endpoint_mismatch_is_integrity_not_nominal_rejection() {
    let (context, live) = context_and_live();
    let mut p = evaluate_airborne_acquisition_direct(&context, &live, 9600)
        .unwrap()
        .selected
        .unwrap();
    let baseline = audit_airborne_acquisition_proposal(&context, &live, &p, 0.0).unwrap();
    assert!(baseline.passed && baseline.commands_match);
    p.end_state.position_m.x += 1.0;
    p.identity = proposal_identity(&p).unwrap();
    let error = audit_airborne_acquisition_proposal(&context, &live, &p, 0.0).unwrap_err();
    assert!(format!("{error:#}").contains("endpoint mismatch without a genuine terrain conflict"));
}

#[test]
#[ignore = "full retained-prefix adapter gate; requires sealed local archives"]
fn acquisition_adapter_matches_all_39_retained_first_executable_proposals() {
    let root = repo_root().unwrap();
    let corpus_path =
        root.join("outputs/research/waypoint_v2_nominal_characterization_20261001/corpus.json");
    let bytes = fs::read(&corpus_path).unwrap();
    verify_approved_corpus_sha256(&sha256_bytes(&bytes).unwrap()).unwrap();
    let corpus: WaypointV2NominalCharacterizationCorpusV1 = serde_json::from_slice(&bytes).unwrap();
    validate_corpus(&corpus).unwrap();
    verify_bindings(&root, &corpus.bindings).unwrap();
    let baseline_path = "outputs/research/waypoint_v2_terminal_time_20261002/final_a/summary.json";
    let baseline_bytes = fs::read(root.join(baseline_path)).unwrap();
    assert_eq!(
        sha256_bytes(&baseline_bytes).unwrap(),
        "9570853b4a44c8a2d758698bfcabe4a5fcf467db04fe636e9e62375c3a3de2ae"
    );
    let baseline: serde_json::Value = serde_json::from_slice(&baseline_bytes).unwrap();
    let mut bindings =
        ground_diagnostic::diagnostic_bindings(&root, &corpus, &corpus_path).unwrap();
    for path in [
        baseline_path,
        "docs/waypoint_v2_airborne_integration_plan.md",
        "scripts/run_waypoint_v2_practical_suite.mjs",
    ] {
        bindings.push(WaypointV2NominalCharacterizationBindingV1 {
            path: path.into(),
            sha256: sha256_bytes(&fs::read(root.join(path)).unwrap()).unwrap(),
        });
    }
    bindings.sort_by(|a, b| a.path.cmp(&b.path));
    verify_bindings(&root, &bindings).unwrap();
    let mut rows = Vec::new();
    let mut attempts = 0;
    let mut powered = 0;
    let mut fallthrough = 0;
    let mut recoveries = 0;
    for row in corpus.rows.iter().filter(|r| r.expected_state.is_some()) {
        let context = RunContext::from_scenario(&row.scenario).unwrap();
        let expected = row.expected_state.as_ref().unwrap();
        let mut live = SimulationState::new(&context).unwrap();
        let mut index = 0;
        while live.physics_step < expected.physics_step {
            assert!(!live.is_terminal());
            if live.physics_step.is_multiple_of(2) {
                let update = &row.prefix_updates[index];
                assert_eq!(update.physics_step, live.physics_step);
                live.set_command(update.command);
                index += 1;
            }
            live.step(&context);
        }
        assert_eq!(index, row.prefix_updates.len());
        assert_eq!(SimulationStateSnapshotV1::from_state(&live), *expected);
        let old = baseline["retained_rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["evidence"]["row_id"] == row.id)
            .unwrap();
        let witnesses: Vec<NominalWitnessEvidenceV1> =
            serde_json::from_value(old["evidence"]["witnesses"].clone()).unwrap();
        let first = witnesses
            .iter()
            .find(|w| {
                w.status == "free_space_target_plane_witness"
                    && w.independent_replay_passed
                    && w.endpoint_state_agreement
                    && w.target_plane_witness.as_ref().is_some_and(|p| {
                        p.safe_by_existing_target_plane_mirror && p.proposal_endpoint_contact_match
                    })
            })
            .unwrap();
        let search = evaluate_airborne_acquisition_direct(
            &context,
            &live,
            row.absolute_deadline_physics_step,
        )
        .unwrap();
        let p = search.selected.as_ref().unwrap();
        assert_eq!(p.updates, first.commands, "{} commands", row.id);
        assert_eq!(p.incoming_state, AirborneFlightStateV1::from_live(&live));
        assert_eq!(
            p.planned_end_physics_step,
            first.target_plane_witness.as_ref().unwrap().physics_step
        );
        assert_eq!(p.seed_id, first.seed_id);
        assert_eq!(p.entry_index, first.entry_index);
        assert_eq!(
            search.attempts.len(),
            witnesses
                .iter()
                .position(|w| w.seed_id == first.seed_id && w.entry_index == first.entry_index)
                .unwrap()
                + 1
        );
        let audit = audit_airborne_acquisition_proposal(&context, &live, p, 5.0).unwrap();
        let terrain = first.terrain_audit.as_ref().unwrap();
        assert_eq!(
            audit.final_state, terrain.final_state,
            "{} terrain endpoint",
            row.id
        );
        assert_eq!(
            audit.first_contact, terrain.first_contact,
            "{} raw contact",
            row.id
        );
        assert!(audit.ordinary_neutral_parity);
        if [
            "local_handoff:v2_ridge_late:H2426",
            "local_handoff:v2_plateau_late:H2446",
            "local_handoff:v2_successive_rising:H2804",
            "local_handoff:v2_successive_plateaus:H3702",
        ]
        .contains(&row.id.as_str())
        {
            assert!(audit.passed, "{} recovery audit", row.id);
            assert!(
                audit.commands_match
                    && audit.safe_target_contact
                    && audit.clearance_scan.all_airborne_states_passed
            );
            recoveries += 1;
        }
        attempts += search.attempts.len();
        powered += usize::from(p.updates.iter().any(|u| u.phase == "nominal_acquisition"));
        fallthrough += usize::from(search.attempts.len() > 1);
        rows.push(serde_json::json!({"row_id":row.id,"full_prefix_matched":true,"selected_commands_match":true,"search":search,"audit":audit}));
    }
    assert_eq!(rows.len(), 39);
    assert_eq!(attempts, 40);
    assert_eq!(powered, 9);
    assert_eq!(fallthrough, 1);
    assert_eq!(recoveries, 4);
    verify_bindings(&root, &bindings).unwrap();
    let output = root.join(std::env::var("PD_LAB_ADAPTER_OUTPUT").unwrap_or_else(|_| {
        "outputs/research/waypoint_v2_airborne_integration_20261002/adapter".into()
    }));
    reserve_output_root(&output).unwrap();
    write_create_only(&output.join("summary.json"), &serde_json::json!({"scope":"adapter equivalence, not complete mission acceptance","bindings":bindings,"bindings_verified_before_and_after":true,"retained_rows":rows,"attempt_count":attempts,"powered_count":powered,"fallthrough_count":fallthrough,"former_no_nominal_recoveries":recoveries,"passed":true})).unwrap();
    eprintln!(
        "adapter gate: 39/39 exact first-valid programs, {attempts} attempts, {powered} powered, {fallthrough} finite fallthrough; {}",
        output.display()
    );
}
