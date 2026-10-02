//! A sealed, evaluator-only comparison, not a new constructor or policy.
use super::*;
use pd_core::IncomingContactV1;
use serde_json::{Value, json};

const STUDY: &str =
    "outputs/research/waypoint_v2_nominal_characterization_20261001/final_verified_b/summary.json";
const CANONICAL: &str =
    "outputs/research/canonical_initial_direct_canary_20260930/final_b/summary.json";
const CONTROLS: &str = "outputs/research/waypoint_v2_practical_20261001/final_policy_2_b/runs";
const STUDY_SHA: &str = "0238b3632a71c8ae5f599fabfca0a7ca303584669b91b9c40f99b3f7fa397d0b";
const CANONICAL_SHA: &str = "bc60a5547a85a3205b18d3b544319e5757ceb650bb16b6296ec8f8d7eeb49ba5";
const CASES: [(&str, &str); 8] = [
    ("v2_clear_685", "operational_flat_span_685"),
    ("v2_clear_735", "completion_flat_span_735"),
    ("v2_clear_845", "operational_flat_span_845"),
    ("v2_clear_915", "completion_flat_span_915"),
    ("v2_clear_uphill_845", "operational_uphill_span_845"),
    ("v2_clear_uphill_915", "completion_uphill_span_915"),
    ("v2_clear_downhill_845", "operational_downhill_span_845"),
    ("v2_clear_downhill_915", "completion_downhill_span_915"),
];
const SHADOW_CASES: [&str; 3] = [
    "v2_clear_845",
    "v2_clear_uphill_845",
    "v2_clear_downhill_845",
];

fn load_json(root: &Path, path: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(root.join(path))?)?)
}

fn collect_sources(root: &Path, relative: &Path, files: &mut BTreeSet<String>) -> Result<()> {
    for item in fs::read_dir(root.join(relative))? {
        let item = item?;
        let rel = relative.join(item.file_name());
        let kind = item.file_type()?;
        if kind.is_symlink() {
            bail!("source-binding symlink {}", rel.display());
        }
        if kind.is_dir() {
            collect_sources(root, &rel, files)?;
        } else if kind.is_file() {
            files.insert(rel.to_string_lossy().into_owned());
        }
    }
    Ok(())
}

pub(super) fn diagnostic_bindings(
    root: &Path,
    corpus: &WaypointV2NominalCharacterizationCorpusV1,
    corpus_path: &Path,
) -> Result<Vec<WaypointV2NominalCharacterizationBindingV1>> {
    let mut paths: BTreeSet<String> = corpus.bindings.iter().map(|b| b.path.clone()).collect();
    let canonical_corpus = fs::canonicalize(corpus_path)?;
    paths.insert(
        canonical_corpus
            .strip_prefix(fs::canonicalize(root)?)?
            .to_string_lossy()
            .into_owned(),
    );
    for p in [
        STUDY,
        CANONICAL,
        "docs/waypoint_v2_ground_diagnostic_plan.md",
        "docs/waypoint_v2_nominal_characterization_protocol.md",
        "Cargo.toml",
        "Cargo.lock",
    ] {
        paths.insert(p.into());
    }
    for item in fs::read_dir(root)? {
        let item = item?;
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with("pd-") && item.file_type()?.is_dir() {
            collect_sources(root, &Path::new(&name).join("src"), &mut paths)?;
            paths.insert(format!("{name}/Cargo.toml"));
        }
    }
    paths
        .into_iter()
        .map(|path| {
            validate_relative_source_path(&path)?;
            Ok(WaypointV2NominalCharacterizationBindingV1 {
                sha256: sha256_bytes(&fs::read(root.join(&path))?)?,
                path,
            })
        })
        .collect()
}

fn pinned(root: &Path, path: &str, expected: &str) -> Result<()> {
    if sha256_bytes(&fs::read(root.join(path))?)? != expected {
        bail!("sealed artifact changed: {path}");
    }
    Ok(())
}

/// The corpus argument is sealed, not a mechanism for choosing easier controls.
pub fn run_waypoint_v2_ground_diagnostic(corpus_path: &Path, output: &Path) -> Result<Value> {
    let root = repo_root()?;
    let bytes = fs::read(corpus_path)?;
    verify_approved_corpus_sha256(&sha256_bytes(&bytes)?)?;
    let corpus: WaypointV2NominalCharacterizationCorpusV1 = serde_json::from_slice(&bytes)?;
    validate_corpus(&corpus)?;
    verify_bindings(&root, &corpus.bindings)?;
    pinned(&root, STUDY, STUDY_SHA)?;
    pinned(&root, CANONICAL, CANONICAL_SHA)?;
    let study: WaypointV2NominalCharacterizationArtifactV1 =
        serde_json::from_value(load_json(&root, STUDY)?)?;
    if !study.integrity_passed || study.corpus_sha256 != APPROVED_CORPUS_SHA256 {
        bail!("invalid sealed study");
    }
    let canonical = load_json(&root, CANONICAL)?;
    let bindings = diagnostic_bindings(&root, &corpus, corpus_path)?;
    verify_bindings(&root, &bindings)?;
    reserve_output_root(output)?;
    write_create_only(&output.join("bindings.json"), &bindings)?;
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    for (id, canonical_id) in CASES {
        let result = control_comparison(&root, &corpus, &study, &canonical, id, canonical_id);
        match result {
            Ok(row) => rows.push(row),
            Err(e) => {
                errors.push(format!("{id}: {e:#}"));
                rows.push(json!({"case_id":id,"integrity_error":format!("{e:#}")}));
            }
        }
    }
    let screen_question = errors.is_empty()
        && rows.iter().any(|row| {
            row["references"].as_array().is_some_and(|refs| {
                refs.iter().any(|r| {
                    r["coupled_bound_mps2"]
                        .as_f64()
                        .zip(r["exact_maximum_mps2"].as_f64())
                        .is_some_and(|(b, e)| b > e + 1.0e-9)
                })
            })
        });
    let mut shadows = Vec::new();
    for id in SHADOW_CASES {
        if !screen_question {
            shadows.push(json!({"case_id":id,"status":"not_needed","reason":"comparison does not leave a conservative-screen question, or control integrity failed"}));
            continue;
        }
        match shadow_probe(&corpus, &study, id) {
            Ok(row) => shadows.push(row),
            Err(e) => {
                errors.push(format!("{id} shadow: {e:#}"));
                shadows.push(
                    json!({"case_id":id,"status":"integrity_error","reason":format!("{e:#}")}),
                );
            }
        }
    }
    let bindings_passed = match verify_bindings(&root, &bindings) {
        Ok(()) => true,
        Err(e) => {
            errors.push(format!("post-run bindings: {e:#}"));
            false
        }
    };
    let reference_count: usize = rows
        .iter()
        .filter_map(|r| r["references"].as_array())
        .map(Vec::len)
        .sum();
    let shadow_count = shadows
        .iter()
        .filter(|r| r["physical_trial_attempted"] == true)
        .count();
    let mut artifact = json!({
        "schema_id":"waypoint_v2_ground_diagnostic_v1", "corpus_sha256":APPROVED_CORPUS_SHA256,
        "bindings":bindings, "bindings_verified_before_and_after":bindings_passed,
        "control_count":rows.len(), "reference_count":reference_count,"shadow_count":shadow_count,
        "maximum_shadow_trials":3, "controls":rows,"shadow_question_applicable":screen_question,"shadows":shadows,
        "shadow_trigger":"matched references show combined-bound conservatism and sealed ground ledgers still reject candidates; test raw flight versus actual headroom without admitting them",
        "integrity_passed":errors.is_empty(),"integrity_errors":errors,
        "nominal_acceptance_unchanged":true,"new_ground_coverage_claimed":false,
        "scope":"diagnostic and recommendation only; no tuning or runtime policy", "identity":""
    });
    artifact["identity"] = Value::String(stable_digest(&artifact)?);
    write_create_only(&output.join("summary.json"), &artifact)?;
    Ok(artifact)
}

pub(super) struct ControlReplay {
    pub(super) s: SimulationState,
    pub(super) t: SimulationState,
    tick204: SimulationStateSnapshotV1,
    pub(super) final_state: SimulationStateSnapshotV1,
    pub(super) contact: IncomingContactV1,
    terminal_actual: Value,
}

pub(super) fn replay_control(
    context: &RunContext,
    commands: &[FlightProgramUpdateV1],
    s_tick: u64,
    t_tick: u64,
    end: u64,
    reference: Option<&BodyAwareTerminalReferenceV1>,
) -> Result<ControlReplay> {
    validate_command_schedule(0, end, commands)?;
    let mut live = SimulationState::new(context)?;
    let mut neutral = SimulationState::new(context)?;
    let (mut s, mut t, mut tick204, mut contact) = (None, None, None, None);
    let mut index = 0;
    let mut maximum_applied_fraction: f64 = 0.0;
    let mut maximum_actual_acceleration: f64 = 0.0;
    let mut maximum_reference_command_delta: f64 = 0.0;
    let mut reference_command_error = None;
    while live.physics_step < end && !live.is_terminal() {
        if live.physics_step == 204 {
            tick204 = Some(SimulationStateSnapshotV1::from_state(&live));
        }
        if live.physics_step == s_tick {
            s = Some(live.clone());
        }
        if live.physics_step == t_tick {
            t = Some(live.clone());
        }
        if live.physics_step.is_multiple_of(HELD_TICKS) {
            let update = commands.get(index).context("control command gap")?;
            if update.physics_step != live.physics_step {
                bail!("control command clock mismatch");
            }
            if live.physics_step >= t_tick {
                maximum_applied_fraction = maximum_applied_fraction.max(applied_throttle(
                    update.command.throttle_frac,
                    context.vehicle.min_throttle_frac,
                ));
                if let Some(r) = reference {
                    let thrust = paired_reference_thrust(context, r, live.physics_step - t_tick);
                    match paired_throttle(
                        context,
                        &BodyAwareTerminalPolicyV1::default(),
                        live.mass_kg(context),
                        thrust.length(),
                    ) {
                        Ok(q) => {
                            maximum_reference_command_delta = maximum_reference_command_delta
                                .max((q - update.command.throttle_frac).abs())
                        }
                        Err(e) => {
                            reference_command_error.get_or_insert_with(|| format!("{e:#}"));
                        }
                    }
                }
            }
            live.set_command(update.command);
            neutral.set_command(update.command);
            index += 1;
        }
        let v_before = neutral.velocity_mps;
        let classification = neutral.step_physics_and_classify_contact(context);
        let report = live.step_with_contact_report(context);
        if live.physics_step > t_tick {
            let actual = (neutral.velocity_mps - v_before) * (1.0 / context.sim.physics_dt_s())
                + Vec2::new(0.0, context.world.gravity_mps2);
            maximum_actual_acceleration = maximum_actual_acceleration.max(actual.length());
        }
        if let Some(incoming) = report.incoming_contact {
            if incoming.classification != classification
                || incoming.state != SimulationStateSnapshotV1::from_state(&neutral)
            {
                bail!("control ordinary/neutral raw contact mismatch");
            }
            contact = Some(incoming);
            break;
        }
        if classification != ContactClassification::None
            || SimulationStateSnapshotV1::from_state(&live)
                != SimulationStateSnapshotV1::from_state(&neutral)
        {
            bail!("control ordinary/neutral prefix mismatch");
        }
    }
    if live.physics_step != end
        || index != commands.len()
        || live.physical_outcome != PhysicalOutcome::LandedOnTarget
        || live.mission_outcome != MissionOutcome::Success
    {
        bail!("control did not consume its program and land at the retained endpoint");
    }
    Ok(ControlReplay {
        s: s.context("S not observed")?,
        t: t.context("T not observed")?,
        tick204: tick204.context("H204 not observed")?,
        final_state: SimulationStateSnapshotV1::from_state(&live),
        contact: contact.context("no raw target contact")?,
        terminal_actual: json!({"maximum_applied_thrust_fraction":maximum_applied_fraction,
            "thrust_fraction_le_0_925":maximum_applied_fraction <= MAX_THRUST_FRACTION,
            "maximum_applied_acceleration_mps2":maximum_actual_acceleration,
            "derived_original_reference_maximum_command_delta":reference.map(|_| maximum_reference_command_delta),
            "derived_reference_command_error":reference_command_error}),
    })
}

pub(super) fn research_terminal_ticks(
    context: &RunContext,
    entry: &SimulationState,
) -> Result<u64> {
    let height = entry.position_m.y
        - context.target_pad.surface_y_m
        - context.vehicle.geometry.touchdown_base_offset_m;
    let down = -entry.velocity_mps.y;
    let target_down = 0.5 * context.vehicle.safe_touchdown_normal_speed_mps;
    let time =
        (2.0 * height + (down - target_down) * context.sim.physics_dt_s()) / (down + target_down);
    even_ticks_ceil(time, context.sim.physics_dt_s())
}

pub(super) fn reference_report(
    context: &RunContext,
    entry: &SimulationState,
    source: &SimulationState,
    ticks: u64,
    deadline: u64,
    provenance: &str,
) -> Result<Value> {
    let height = entry.position_m.y
        - context.target_pad.surface_y_m
        - context.vehicle.geometry.touchdown_base_offset_m;
    let finish = entry
        .physics_step
        .checked_add(ticks)
        .and_then(|t| t.checked_add(TERMINAL_EXTRA_TICKS))
        .context("finish overflow")?;
    if ticks > deadline {
        return Ok(
            json!({"provenance":provenance,"physics_ticks":ticks,"first_rejection":"reference exceeds bounded diagnostic duration","reference_built":false}),
        );
    }
    let reference = build_reference(
        context,
        &BodyAwareTerminalPolicyV1::default(),
        target_kinematics(context, entry),
        ticks,
    )?;
    let bound = conservative_terminal_thrust_bound(context, &reference);
    let mut exact: f64 = 0.0;
    let mut pair: f64 = 0.0;
    for tick in 0..ticks {
        exact = exact.max(reference.thrust(context, tick).length());
    }
    for tick in (0..ticks + TERMINAL_EXTRA_TICKS).step_by(HELD_TICKS as usize) {
        pair = pair.max(paired_reference_thrust(context, &reference, tick).length());
    }
    let limit = MAX_THRUST_FRACTION * context.vehicle.max_thrust_n / entry.mass_kg(context);
    let forward = terminal_forward_velocity_valid(context, &reference);
    let geometry = height > TARGET_ENTRY_RESERVE_M && entry.velocity_mps.y < 0.0;
    let first = paired_reference_thrust(context, &reference, 0);
    let (turn_from_source, _) = turn_ticks_for(context, source.attitude_rad, first)?;
    let (turn_at_entry, _) = turn_ticks_for(context, entry.attitude_rad, first)?;
    let coast = entry.physics_step - source.physics_step;
    let fuel = estimate_terminal_fuel(context, &reference, entry.fuel_kg);
    let first_rejection = if !geometry {
        Some("entry_geometry")
    } else if finish > deadline {
        Some("deadline")
    } else if !forward {
        Some("lateral_reversal")
    } else if bound > limit {
        Some("coupled_thrust_bound")
    } else if turn_from_source > coast {
        Some("coast_alignment")
    } else if fuel.is_err() {
        Some("throttle_or_fuel")
    } else {
        None
    };
    Ok(
        json!({"provenance":provenance,"derived_reference":true,"reference":reference,
        "physics_ticks":ticks,"duration_s":ticks as f64*context.sim.physics_dt_s(),"finish_physics_step":finish,
        "entry_descending_above_reserve":geometry,"deadline_passed":finish<=deadline,
        "forward_velocity_passed":forward,"coupled_bound_mps2":bound,"exact_maximum_mps2":exact,
        "paired_maximum_mps2":pair,"incoming_mass_screen_limit_mps2":limit,
        "bound_fits_0_925":bound<=limit,"exact_fits_0_925":exact<=limit,
        "raw_incoming_acceleration_limit_mps2":context.vehicle.max_thrust_n/entry.mass_kg(context),
        "actual_coast_ticks":coast,"first_thrust_turn_ticks_from_actual_S":turn_from_source,
        "remaining_turn_ticks_at_actual_T":turn_at_entry,"estimated_fuel_kg":fuel.as_ref().ok(),
        "estimated_fuel_error":fuel.err().map(|e|format!("{e:#}")),"first_rejection":first_rejection,
        "alternative_reference_physically_executed":false}),
    )
}

fn row_for<'a>(
    corpus: &'a WaypointV2NominalCharacterizationCorpusV1,
    id: &str,
) -> Result<&'a WaypointV2NominalCharacterizationCorpusRowV1> {
    corpus
        .rows
        .iter()
        .find(|r| r.id == format!("clear_start:{id}"))
        .context("missing sealed clear row")
}
fn study_for<'a>(
    study: &'a WaypointV2NominalCharacterizationArtifactV1,
    id: &str,
) -> Result<&'a NominalCharacterizationRowEvidenceV1> {
    study
        .retained_rows
        .iter()
        .find(|r| r.row_id == format!("clear_start:{id}"))
        .context("missing sealed clear ledger")
}

fn control_comparison(
    root: &Path,
    corpus: &WaypointV2NominalCharacterizationCorpusV1,
    study: &WaypointV2NominalCharacterizationArtifactV1,
    canonical: &Value,
    id: &str,
    canonical_id: &str,
) -> Result<Value> {
    let row = row_for(corpus, id)?;
    let scenario: ScenarioSpec =
        serde_json::from_value(load_json(root, &format!("{CONTROLS}/{id}/scenario.json"))?)?;
    if serde_json::to_value(&scenario)? != serde_json::to_value(&row.scenario)?
        || row.absolute_deadline_physics_step != 9600
    {
        bail!("V2 scenario/corpus mismatch");
    }
    let context = RunContext::from_scenario(&scenario).map_err(anyhow::Error::msg)?;
    let flight = load_json(root, &format!("{CONTROLS}/{id}/flight.json"))?;
    let control = canonical["cases"]
        .as_array()
        .context("canonical cases missing")?
        .iter()
        .find(|c| c["case_id"] == canonical_id)
        .context("canonical mapping missing")?;
    let old_context = RunContext::from_scenario(&serde_json::from_value::<ScenarioSpec>(
        control["request"]["scenario"].clone(),
    )?)
    .map_err(anyhow::Error::msg)?;
    if serde_json::to_value(&context.vehicle)? != serde_json::to_value(&old_context.vehicle)?
        || context.world.gravity_mps2 != old_context.world.gravity_mps2
        || context.sim.physics_hz != old_context.sim.physics_hz
        || context.sim.controller_hz != old_context.sim.controller_hz
        || serde_json::to_value(&scenario.initial_state)?
            != control["request"]["scenario"]["initial_state"]
        || serde_json::to_value(&context.target_pad)?
            != serde_json::to_value(&old_context.target_pad)?
        || serde_json::to_value(context.world.landing_pad(&row.source_pad_id))?
            != serde_json::to_value(old_context.world.landing_pad(&row.source_pad_id))?
    {
        bail!("canonical physical compatibility mismatch");
    }
    let p = &control["selected_program"];
    let commands: Vec<FlightProgramUpdateV1> = serde_json::from_value(p["updates"].clone())?;
    let actual_commands: Vec<FlightProgramUpdateV1> =
        serde_json::from_value(flight["cycles"][0]["nominal_updates"].clone())?;
    if commands != actual_commands {
        bail!("original command mapping mismatch");
    }
    let s = p["source_handoff_physics_step"]
        .as_u64()
        .context("S missing")?;
    let t = p["terminal_entry_physics_step"]
        .as_u64()
        .context("T missing")?;
    let end = p["planned_end_physics_step"]
        .as_u64()
        .context("end missing")?;
    let duration = control["search"]["selected"]["terminal_tick_count"]
        .as_u64()
        .context("original terminal duration missing")?;
    if s % 2 != 0 || t % 2 != 0 || end > 9600 || t <= s || t >= end {
        bail!("invalid original boundaries");
    }
    let replay = replay_control(&context, &commands, s, t, end, None)?;
    let archived_source: AirborneFlightStateV1 =
        serde_json::from_value(control["search"]["selected"]["source_handoff_state"].clone())?;
    if archived_source != AirborneFlightStateV1::from_live(&replay.s) {
        bail!("archived source boundary disagrees with fresh full-prefix replay");
    }
    let reference = build_reference(
        &context,
        &BodyAwareTerminalPolicyV1::default(),
        target_kinematics(&context, &replay.t),
        duration,
    )?;
    let independent = replay_control(&context, &commands, s, t, end, Some(&reference))?;
    let expected: SimulationStateSnapshotV1 =
        serde_json::from_value(flight["ordinary_flight"]["final_state"].clone())?;
    let expected_contact: IncomingContactV1 =
        serde_json::from_value(flight["ordinary_flight"]["incoming_contact"].clone())?;
    if replay.final_state != expected
        || replay.contact != expected_contact
        || independent.final_state != replay.final_state
        || independent.contact != replay.contact
        || SimulationStateSnapshotV1::from_state(&independent.s)
            != SimulationStateSnapshotV1::from_state(&replay.s)
        || SimulationStateSnapshotV1::from_state(&independent.t)
            != SimulationStateSnapshotV1::from_state(&replay.t)
    {
        bail!("retained or independent control replay mismatch");
    }
    let original = reference_report(
        &context,
        &replay.t,
        &replay.s,
        duration,
        9600,
        "canonical.search.selected.terminal_tick_count",
    )?;
    let new_ticks = research_terminal_ticks(&context, &replay.t)?;
    let research = reference_report(
        &context,
        &replay.t,
        &replay.s,
        new_ticks,
        9600,
        "unchanged_research_braking_formula",
    )?;
    let old = study_for(study, id)?;
    let acquisition_needed = old.status == "finite_miss";
    let acquisition = if acquisition_needed {
        Some(json!({
        "old_source_end":SimulationStateSnapshotV1::from_state(&replay.s),
        "old_source_phases":commands.iter().filter(|u|u.physics_step<s).fold(BTreeMap::<String,u64>::new(),|mut m,u|{*m.entry(u.phase.clone()).or_default()+=HELD_TICKS;m}),
        "research_preparation_end":old.ground_prep.as_ref().map(|p|&p.end_state),
        "existing_acquisition_estimates":old.seeds.iter().map(|s|json!({
            "seed_id":s.seed_id,"turn_physics_ticks":s.turn_physics_ticks,"burn_physics_ticks":s.burn_physics_ticks,
            "thrust_acceleration_mps2":s.requested_thrust_acceleration_mps2,"predicted_end":s.predicted_acquisition_end,
            "estimated_fuel_kg":s.estimated_acquisition_fuel_kg,"upward_impulse_mps":s.upward_impulse_mps,
            "status":s.status,"reason":s.reason,"predicted_not_executed":true})).collect::<Vec<_>>(),
        "existing_seed_status_counts":old.seeds.iter().fold(BTreeMap::<String,usize>::new(),|mut m,s|{*m.entry(s.status.clone()).or_default()+=1;m}),
        "comparison_only":true,"new_seed_evaluations":0,"continuation_witnesses":0,
        "remaining_gap":"Known-entry time mismatch does not prove this constant-vector acquisition reaches equivalent entries; no new acquisition search performed"}))
    } else {
        None
    };
    Ok(
        json!({"case_id":id,"canonical_case_id":canonical_id,"control_replay_passed":true,
        "full_program_command_identity":stable_digest(&commands)?,"commands":commands,"archived_source_boundary_match":true,
        "original_deadline_physics_step":9600,"source_boundary":s,"terminal_boundary":t,
        "tick204":replay.tick204,"actual_S":SimulationStateSnapshotV1::from_state(&replay.s),
        "actual_T":SimulationStateSnapshotV1::from_state(&replay.t),"raw_contact":replay.contact,
        "final_state":replay.final_state,"actual_terminal_demand":independent.terminal_actual,
        "references":[original,research],"research_ground_status":old.status,
        "acquisition_comparison":acquisition,"integrity_error":null}),
    )
}

fn recorded_limit(entry: &NominalEntryScreenEvidenceV1) -> Option<f64> {
    let reason = entry.reason.as_deref()?;
    if !reason.starts_with("coupled terminal thrust bound ") {
        return None;
    }
    reason
        .split("incoming-mass limit ")
        .nth(1)?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
}

fn select_shadow(
    row: &NominalCharacterizationRowEvidenceV1,
    positive: bool,
) -> Option<(&NominalSeedEvidenceV1, &NominalEntryScreenEvidenceV1)> {
    if positive {
        let witness = row.witnesses.first()?;
        let seed = row.seeds.iter().find(|s| s.seed_id == witness.seed_id)?;
        return Some((
            seed,
            seed.entry_screens
                .iter()
                .find(|e| e.entry_index == witness.entry_index)?,
        ));
    }
    row.seeds
        .iter()
        .filter(|s| {
            s.requested_thrust_acceleration_mps2.is_some() && s.predicted_acquisition_end.is_some()
        })
        .flat_map(|s| {
            s.entry_screens.iter().filter_map(move |e| {
                let limit = recorded_limit(e)?;
                let bound = e.coupled_thrust_bound_mps2?;
                if !bound.is_finite() || !e.lateral_velocity_reversal_checked || e.admissible {
                    return None;
                }
                Some((s, e, bound / limit))
            })
        })
        .min_by(|a, b| {
            a.2.total_cmp(&b.2)
                .then_with(|| a.0.seed_id.cmp(&b.0.seed_id))
                .then_with(|| a.1.entry_index.cmp(&b.1.entry_index))
        })
        .map(|(s, e, _)| (s, e))
}

pub(super) fn program_demand(
    context: &RunContext,
    incoming: &SimulationState,
    commands: &[FlightProgramUpdateV1],
    end: u64,
) -> Result<Value> {
    validate_command_schedule(incoming.physics_step, end, commands)?;
    let mut state = incoming.clone();
    let mut index = 0;
    let mut phases = BTreeMap::<String, (f64, f64)>::new();
    let mut phase = String::new();
    while state.physics_step < end {
        if state.physics_step.is_multiple_of(HELD_TICKS) {
            let u = &commands[index];
            state.set_command(u.command);
            phase = u.phase.clone();
            index += 1;
            let p = phases.entry(phase.clone()).or_default();
            p.0 = p.0.max(applied_throttle(
                u.command.throttle_frac,
                context.vehicle.min_throttle_frac,
            ));
        }
        let before = state.velocity_mps;
        state.step_physics_and_classify_contact(context);
        let actual = (state.velocity_mps - before) * (1.0 / context.sim.physics_dt_s())
            + Vec2::new(0.0, context.world.gravity_mps2);
        let metrics = phases.entry(phase.clone()).or_default();
        metrics.1 = metrics.1.max(actual.length());
    }
    Ok(json!(phases.into_iter().map(|(p,(q,a))|(p,json!({"maximum_applied_thrust_fraction":q,"thrust_fraction_le_0_925":q<=MAX_THRUST_FRACTION,"maximum_applied_acceleration_mps2":a}))).collect::<BTreeMap<_,_>>()))
}

fn shadow_probe(
    corpus: &WaypointV2NominalCharacterizationCorpusV1,
    study: &WaypointV2NominalCharacterizationArtifactV1,
    id: &str,
) -> Result<Value> {
    let row = row_for(corpus, id)?;
    let ledger = study_for(study, id)?;
    let Some((seed, entry)) = select_shadow(ledger, id == "v2_clear_downhill_845") else {
        return Ok(
            json!({"case_id":id,"status":"not_evaluated","reason":"no eligible sealed entry; no substitution"}),
        );
    };
    let context = RunContext::from_scenario(&row.scenario).map_err(anyhow::Error::msg)?;
    let mut live = SimulationState::new(&context)?;
    let prep = ground_prep(
        &context,
        &row.scenario,
        &row.source_pad_id,
        &row.target_pad_id,
        &mut live,
        9600,
    )?;
    let old_prep = ledger.ground_prep.as_ref().context("sealed prep missing")?;
    if !prep.success || prep.commands != old_prep.commands || prep.end_state != old_prep.end_state {
        bail!("research preparation does not reproduce sealed history");
    }
    let mut verified = prep.clone();
    verify_ground_prep_replay(&context, &mut verified)?;
    if !verified.independent_replay_passed || verified.integrity_error.is_some() {
        bail!("research prep replay failed");
    }
    let spec = SeedSpec {
        seed_id: seed.seed_id.clone(),
        kind: seed.kind.clone(),
        virtual_arrival_ticks: seed.virtual_arrival_physics_ticks,
        burn_fraction: seed.burn_fraction,
        natural_profile: seed.kind == "natural_profile",
        zero_acquisition: seed.kind == "zero_acquisition",
    };
    let plan = make_acquisition_plan(&context, &live, &spec)?;
    let ticks = entry
        .terminal_physics_ticks
        .context("chosen duration missing")?;
    let r = build_reference(
        &context,
        &BodyAwareTerminalPolicyV1::default(),
        entry.predicted_entry,
        ticks,
    )?;
    let first = paired_reference_thrust(&context, &r, 0);
    let acquisition_angle = if plan.turn_ticks > 0 {
        plan.target_attitude_rad
    } else {
        live.attitude_rad
    };
    let (align, _) = turn_ticks_for(&context, acquisition_angle, first)?;
    let fuel = estimate_terminal_fuel(&context, &r, live.fuel_kg - plan.estimated_fuel_kg);
    if entry.coast_physics_ticks < align || fuel.is_err() {
        return Ok(
            json!({"case_id":id,"status":"not_evaluated","seed_id":seed.seed_id,"entry_index":entry.entry_index,
            "original_screen":entry,"required_alignment_ticks":align,"available_coast_ticks":entry.coast_physics_ticks,
            "remaining_fuel_predicate_error":fuel.err().map(|e|format!("{e:#}")),"reason":"unchanged remaining alignment/fuel predicate failed; no retry"}),
        );
    }
    let request = research_request(&row.scenario, &row.source_pad_id, &row.target_pad_id);
    let witness = match materialize_witness(&context, &request, &live, 9600, seed, entry) {
        Ok(w) => w,
        Err(WitnessMaterializationFailure::FiniteMiss(reason)) => {
            return Ok(
                json!({"case_id":id,"status":"finite_physical_rejection","physical_trial_attempted":true,"seed_id":seed.seed_id,"entry_index":entry.entry_index,"reason":reason,"original_screen":entry}),
            );
        }
        Err(WitnessMaterializationFailure::Integrity(reason)) => {
            bail!("shadow integrity: {reason}")
        }
    };
    if witness.status == "integrity_error" || !witness.independent_replay_passed {
        bail!("shadow independent replay failed");
    }
    let end = witness
        .target_plane_witness
        .as_ref()
        .map(|p| p.physics_step)
        .unwrap_or(witness.actual_acquisition_end.physics_step);
    let demand = program_demand(&context, &live, &witness.commands, end)?;
    let body_ok = witness.terrain_audit.as_ref().is_some_and(|a| {
        a.reserve_phases.iter().all(|p| {
            p.first_reserve_violation_physics_step.is_none() && p.first_query_error.is_none()
        })
    });
    let terminal_q = demand["terminal_bridge"]["maximum_applied_thrust_fraction"].as_f64();
    Ok(
        json!({"case_id":id,"status":"shadow_only","physical_trial_attempted":true,"seed_id":seed.seed_id,"entry_index":entry.entry_index,
        "original_screen":entry,"predicted_bound_recorded_limit_ratio":entry.coupled_thrust_bound_mps2.zip(recorded_limit(entry)).map(|(b,l)|b/l),
        "bypassed_gate":if entry.admissible {"none_positive_control"} else {"conservative_terminal_bound_only"},"nominal_admissible_unchanged":entry.admissible,
        "preparation":verified,"witness":witness,"actual_phase_demand":demand,"body_reserve_passed":body_ok,
        "terminal_actual_thrust_fraction_le_0_925":terminal_q.map(|q|q<=MAX_THRUST_FRACTION),
        "ground_coverage_claimed":false}),
    )
}

#[cfg(test)]
mod tests;
