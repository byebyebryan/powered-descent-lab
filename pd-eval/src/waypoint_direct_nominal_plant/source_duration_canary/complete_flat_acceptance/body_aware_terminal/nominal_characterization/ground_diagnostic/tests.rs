use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn sealed_study() -> (PathBuf, WaypointV2NominalCharacterizationArtifactV1) {
    let root = repo_root().unwrap();
    let bytes = fs::read(root.join(STUDY)).unwrap();
    assert_eq!(sha256_bytes(&bytes).unwrap(), STUDY_SHA);
    let study: WaypointV2NominalCharacterizationArtifactV1 =
        serde_json::from_slice(&bytes).unwrap();
    assert!(study.integrity_passed);
    (root, study)
}

fn temporary_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "waypoint-v2-ground-diagnostic-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn fixture_context() -> RunContext {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let manifest: Value = serde_json::from_slice(
        &fs::read(
            root.join("fixtures/research/waypoint_direct_body_aware_terminal_fresh_inputs_v1.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let scenario: ScenarioSpec =
        serde_json::from_value(manifest["cases"][0]["scenario"].clone()).unwrap();
    RunContext::from_scenario(&scenario).unwrap()
}

#[test]
#[ignore = "requires retained opt-in ground diagnostic archives"]
fn sealed_selector_is_deterministic_terrain_independent_and_never_substitutes() {
    let (_, study) = sealed_study();
    for id in ["v2_clear_845", "v2_clear_uphill_845"] {
        let row = study_for(&study, id).unwrap();
        let (seed, entry) = select_shadow(row, false).expect("finite coupled-bound candidate");
        assert_eq!(seed.seed_id, "upward_shaping_n2364_b65");
        assert_eq!(entry.entry_index, 2);
        assert!(seed.requested_thrust_acceleration_mps2.is_some());
        assert!(seed.predicted_acquisition_end.is_some());
        assert!(
            entry
                .reason
                .as_deref()
                .unwrap()
                .starts_with("coupled terminal thrust bound ")
        );
    }

    // The negative selector only sees the sealed screen ledger, never witness terrain outcomes.
    let flat = study_for(&study, "v2_clear_845").unwrap();
    let downhill = study_for(&study, "v2_clear_downhill_845").unwrap();
    let mut selected_screen = select_shadow(flat, false).unwrap().1.clone();
    assert!((recorded_limit(&selected_screen).unwrap() - 17.035739).abs() < 1.0e-6);
    for reason in [
        None,
        Some("coupled terminal thrust bound 18.0 exceeds other limit 17.0"),
        Some("coupled terminal thrust bound 18.0 exceeds incoming-mass limit NaN"),
        Some("coupled terminal thrust bound 18.0 exceeds incoming-mass limit inf"),
        Some("coupled terminal thrust bound 18.0 exceeds incoming-mass limit 0"),
        Some("coupled terminal thrust bound 18.0 exceeds incoming-mass limit -1"),
    ] {
        selected_screen.reason = reason.map(str::to_owned);
        assert_eq!(recorded_limit(&selected_screen), None, "reason {reason:?}");
    }

    let mut changed_terrain = flat.clone();
    let mut witness = downhill.witnesses[0].clone();
    let terrain = witness
        .terrain_audit
        .as_mut()
        .expect("retained terrain audit");
    terrain.landed_on_target = !terrain.landed_on_target;
    terrain.terrain_contact_blocks_landing_claim = !terrain.terrain_contact_blocks_landing_claim;
    changed_terrain.witnesses.push(witness);
    let (again, again_entry) = select_shadow(&changed_terrain, false).unwrap();
    assert_eq!(
        (again.seed_id.as_str(), again_entry.entry_index),
        ("upward_shaping_n2364_b65", 2)
    );

    // With no acquisition-passing finite-bound entry, do not substitute another seed.
    let mut no_candidate = flat.clone();
    for candidate in &mut no_candidate.seeds {
        candidate.requested_thrust_acceleration_mps2 = None;
    }
    assert!(select_shadow(&no_candidate, false).is_none());

    let first = downhill.witnesses.first().unwrap();
    let (seed, entry) = select_shadow(downhill, true).expect("existing positive-control witness");
    assert_eq!(seed.seed_id, "upward_shaping_n1970_b45");
    assert_eq!(entry.entry_index, 2);
    assert_eq!(
        (seed.seed_id.as_str(), entry.entry_index),
        (first.seed_id.as_str(), first.entry_index)
    );

    let mut no_positive_substitute = downhill.clone();
    no_positive_substitute.witnesses[0].entry_index = usize::MAX;
    assert!(select_shadow(&no_positive_substitute, true).is_none());
}

#[test]
#[ignore = "requires retained opt-in ground diagnostic archives"]
fn original_duration_uses_the_canonical_selection_field() {
    let (root, _) = sealed_study();
    let canonical_bytes = fs::read(root.join(CANONICAL)).unwrap();
    assert_eq!(sha256_bytes(&canonical_bytes).unwrap(), CANONICAL_SHA);
    let canonical: Value = serde_json::from_slice(&canonical_bytes).unwrap();
    let case = canonical["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["case_id"] == "operational_flat_span_845")
        .unwrap();
    let selected_ticks = case["search"]["selected"]["terminal_tick_count"]
        .as_u64()
        .unwrap();
    assert_eq!(selected_ticks, 1860);
    assert_ne!(
        selected_ticks,
        case["selected_program"]["planned_end_physics_step"]
            .as_u64()
            .unwrap()
            - case["selected_program"]["source_handoff_physics_step"]
                .as_u64()
                .unwrap()
    );

    let context = fixture_context();
    let mut entry = SimulationState::new(&context).unwrap();
    entry.position_m.y =
        context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m + 700.0;
    entry.velocity_mps = Vec2::new(24.0, -32.0);
    let report = reference_report(
        &context,
        &entry,
        &entry,
        selected_ticks,
        9600,
        "canonical.search.selected.terminal_tick_count",
    )
    .unwrap();
    assert_eq!(report["physics_ticks"].as_u64(), Some(selected_ticks));
    assert_eq!(
        report["provenance"],
        "canonical.search.selected.terminal_tick_count"
    );
}

#[test]
fn research_braking_formula_uses_discrete_even_held_ticks() {
    let context = fixture_context();
    let mut entry = SimulationState::new(&context).unwrap();
    entry.position_m.y =
        context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m + 700.0;
    entry.velocity_mps = Vec2::new(24.0, -32.0);
    let dt = context.sim.physics_dt_s();
    let down = -entry.velocity_mps.y;
    let target_down = 0.5 * context.vehicle.safe_touchdown_normal_speed_mps;
    let expected = even_ticks_ceil(
        (2.0 * 700.0 + (down - target_down) * dt) / (down + target_down),
        dt,
    )
    .unwrap();
    let ticks = research_terminal_ticks(&context, &entry).unwrap();
    assert_eq!(ticks, expected);
    assert_eq!(ticks % HELD_TICKS, 0);
}

#[test]
fn combined_conservative_norm_is_not_the_exact_discrete_vector_maximum() {
    let context = fixture_context();
    let state = SimulationState::new(&context).unwrap();
    let reference = BodyAwareTerminalReferenceV1 {
        start: target_kinematics(&context, &state),
        end: target_kinematics(&context, &state),
        physics_ticks: 4,
        horizontal_coefficients_mps2: [0.0, 8.0, -8.0],
        initial_vertical_acceleration_mps2: 1.0 - context.world.gravity_mps2,
        vertical_acceleration_delta_mps2: 3.0,
        identity: String::new(),
    };
    let exact = (0..reference.physics_ticks)
        .map(|tick| reference.thrust(&context, tick).length())
        .fold(0.0_f64, f64::max);
    let combined = conservative_terminal_thrust_bound(&context, &reference);
    assert!((exact - 10.0).abs() < 1.0e-12);
    assert!((combined - (104.0_f64).sqrt()).abs() < 1.0e-12);
    assert!(combined > exact);

    // Both values and the incoming-mass limit are accelerations; force is divided by mass.
    let raw_limit = context.vehicle.max_thrust_n / state.mass_kg(&context);
    let policy_limit = MAX_THRUST_FRACTION * raw_limit;
    assert!(raw_limit > policy_limit);
    assert!((raw_limit * state.mass_kg(&context) - context.vehicle.max_thrust_n).abs() < 1.0e-9);
}

#[test]
fn command_gaps_and_bound_source_tampering_fail_closed_and_output_is_create_only() {
    let update = |physics_step| FlightProgramUpdateV1 {
        physics_step,
        phase: "terminal_bridge".into(),
        command: Command {
            throttle_frac: 0.2,
            target_attitude_rad: 0.0,
        },
    };
    let updates = vec![update(0), update(2), update(4)];
    assert!(validate_command_schedule(0, 6, &updates).is_ok());
    assert!(validate_command_schedule(0, 5, &updates).is_ok()); // odd first-contact endpoint, complete held pairs
    assert!(validate_command_schedule(0, 6, &updates[..2]).is_err());
    assert!(validate_command_schedule(0, 5, &updates[..2]).is_err());
    let mut gapped = updates.clone();
    gapped[1].physics_step = 4;
    assert!(validate_command_schedule(0, 6, &gapped).is_err());
    let original_identity = stable_digest(&updates).unwrap();
    let mut tampered = updates.clone();
    tampered[1].command.throttle_frac = 0.21;
    assert!(validate_command_schedule(0, 6, &tampered).is_ok());
    assert_ne!(stable_digest(&tampered).unwrap(), original_identity);

    let root = temporary_path("binding");
    fs::create_dir(&root).unwrap();
    let input = root.join("source.json");
    fs::write(&input, b"sealed input").unwrap();
    let binding = WaypointV2NominalCharacterizationBindingV1 {
        path: "source.json".into(),
        sha256: sha256_bytes(b"sealed input").unwrap(),
    };
    assert!(verify_bindings(&root, std::slice::from_ref(&binding)).is_ok());
    fs::write(&input, b"tampered input").unwrap();
    assert!(verify_bindings(&root, std::slice::from_ref(&binding)).is_err());
    let missing = WaypointV2NominalCharacterizationBindingV1 {
        path: "missing.json".into(),
        sha256: "0".repeat(64),
    };
    assert!(verify_bindings(&root, &[missing]).is_err());
    fs::remove_file(&input).unwrap();
    fs::remove_dir(&root).unwrap();

    let output = temporary_path("output");
    reserve_output_root(&output).unwrap();
    let payload = json!({"schema_id":"test"});
    write_create_only(&output.join("summary.json"), &payload).unwrap();
    let round_trip: Value =
        serde_json::from_slice(&fs::read(output.join("summary.json")).unwrap()).unwrap();
    assert_eq!(round_trip, payload);
    assert!(write_create_only(&output.join("summary.json"), &payload).is_err());
    assert!(reserve_output_root(&output).is_err());
    fs::remove_file(output.join("summary.json")).unwrap();
    fs::remove_dir(output).unwrap();
}
