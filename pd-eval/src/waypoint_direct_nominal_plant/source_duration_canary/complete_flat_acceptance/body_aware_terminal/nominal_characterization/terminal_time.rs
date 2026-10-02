//! Bounded terminal-time research; old generators and runtime policies remain intact.
use super::ground_diagnostic as diagnostic;
use super::*;
use serde_json::{Value, json};
mod candidates;
use candidates::alternative_durations;

const DIAGNOSTIC: &str =
    "outputs/research/waypoint_v2_ground_diagnostic_20261002/final_verified_b/summary.json";
const DIAGNOSTIC_SHA: &str = "bc05aca22a6491518c463188b91440198302997e4239d5099d2e855190c535f4";
const BASELINE: &str =
    "outputs/research/waypoint_v2_nominal_characterization_20261001/final_verified_b/summary.json";
const BASELINE_SHA: &str = "0238b3632a71c8ae5f599fabfca0a7ca303584669b91b9c40f99b3f7fa397d0b";
const ID: &str = "waypoint-v2-terminal-time-v1";

fn read_pinned(root: &Path, path: &str, expected: &str) -> Result<Value> {
    let bytes = fs::read(root.join(path))?;
    if sha256_bytes(&bytes)? != expected {
        bail!("sealed research input changed: {path}");
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn choose_report(reports: &[Value]) -> Option<usize> {
    reports
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            r.get("first_rejection") == Some(&Value::Null)
                && r["estimated_fuel_kg"].as_f64().is_some_and(f64::is_finite)
                && r["finish_physics_step"].as_u64().is_some()
        })
        .min_by(|(ia, a), (ib, b)| {
            a["estimated_fuel_kg"]
                .as_f64()
                .unwrap_or(f64::INFINITY)
                .total_cmp(&b["estimated_fuel_kg"].as_f64().unwrap_or(f64::INFINITY))
                .then_with(|| {
                    a["finish_physics_step"]
                        .as_u64()
                        .cmp(&b["finish_physics_step"].as_u64())
                })
                .then_with(|| ia.cmp(ib))
        })
        .map(|(i, _)| i)
}

fn matched_control(
    row: &WaypointV2NominalCharacterizationCorpusRowV1,
    old: &Value,
) -> Result<Value> {
    let context = RunContext::from_scenario(&row.scenario).map_err(anyhow::Error::msg)?;
    let commands: Vec<FlightProgramUpdateV1> = serde_json::from_value(old["commands"].clone())?;
    let s = old["source_boundary"].as_u64().context("missing S")?;
    let t = old["terminal_boundary"].as_u64().context("missing T")?;
    let end = old["final_state"]["physics_step"]
        .as_u64()
        .context("missing contact clock")?;
    let observed = diagnostic::replay_control(&context, &commands, s, t, end, None)?;
    let independent = diagnostic::replay_control(&context, &commands, s, t, end, None)?;
    for state in [&observed, &independent] {
        if serde_json::to_value(SimulationStateSnapshotV1::from_state(&state.s))? != old["actual_S"]
            || serde_json::to_value(SimulationStateSnapshotV1::from_state(&state.t))?
                != old["actual_T"]
            || serde_json::to_value(&state.final_state)? != old["final_state"]
            || serde_json::to_value(&state.contact)? != old["raw_contact"]
        {
            bail!("original live-prefix/contact replay mismatch");
        }
    }
    let ticks = diagnostic::research_terminal_ticks(&context, &observed.t)?;
    let baseline = diagnostic::reference_report(
        &context,
        &observed.t,
        &observed.s,
        ticks,
        row.absolute_deadline_physics_step,
        "unchanged_vertical_baseline",
    )?;
    let mut reports = vec![baseline];
    let mut metadata = vec![
        json!({"candidate_index":0,"provenance":"unchanged_vertical_baseline","physics_ticks":ticks}),
    ];
    if !reports[0]["first_rejection"].is_null() {
        let direction =
            forward_direction(&context, observed.t.position_m.x, observed.t.velocity_mps.x);
        for c in alternative_durations(
            direction * (context.target_pad.center_x_m - observed.t.position_m.x),
            direction * observed.t.velocity_mps.x,
            context.sim.physics_dt_s(),
            observed.t.physics_step,
            row.absolute_deadline_physics_step,
            ticks,
        )? {
            reports.push(diagnostic::reference_report(
                &context,
                &observed.t,
                &observed.s,
                c.physics_ticks,
                row.absolute_deadline_physics_step,
                &c.provenance,
            )?);
            metadata.push(serde_json::to_value(c)?);
        }
    }
    let selected = if reports[0]["first_rejection"].is_null() {
        Some(0)
    } else {
        choose_report(&reports)
    };
    Ok(
        json!({"case_id":old["case_id"],"control_replay_passed":true,
        "full_program_command_identity":stable_digest(&commands)?,
        "actual_S":SimulationStateSnapshotV1::from_state(&observed.s),
        "actual_T":SimulationStateSnapshotV1::from_state(&observed.t),
        "final_state":observed.final_state,"raw_contact":observed.contact,
        "original_duration_comparison_only":old["references"][0]["duration_s"],
        "duration_candidates":metadata,"references":reports,"selected_candidate_index":selected,
        "screen_passed":selected.is_some(),"alternative_physically_executed":false}),
    )
}

fn plan_from_seed(seed: &NominalSeedEvidenceV1, live: &SimulationState) -> Result<AcquisitionPlan> {
    let thrust = seed
        .requested_thrust_acceleration_mps2
        .context("missing cached acquisition thrust")?;
    Ok(AcquisitionPlan {
        thrust_acceleration_mps2: thrust,
        turn_ticks: seed.turn_physics_ticks,
        burn_ticks: seed.burn_physics_ticks,
        predicted_end: seed
            .predicted_acquisition_end
            .context("missing cached acquisition endpoint")?,
        virtual_target_error_m: seed
            .virtual_target_error_m
            .context("missing acquisition target error")?,
        estimated_fuel_kg: seed
            .estimated_acquisition_fuel_kg
            .context("missing acquisition fuel")?,
        upward_impulse_mps: seed
            .upward_impulse_mps
            .context("missing acquisition impulse")?,
        turn_updates: seed.turn_updates,
        target_attitude_rad: if seed.turn_physics_ticks > 0 {
            thrust.x.atan2(thrust.y)
        } else {
            live.attitude_rad
        },
    })
}

fn screen_duration(
    context: &RunContext,
    live: &SimulationState,
    deadline: u64,
    plan: &AcquisitionPlan,
    original: &NominalEntryScreenEvidenceV1,
    ticks: u64,
) -> Result<NominalEntryScreenEvidenceV1> {
    let mut e = original.clone();
    e.terminal_physics_ticks = Some(ticks);
    e.terminal_time_s = Some(ticks as f64 * context.sim.physics_dt_s());
    e.coupled_thrust_bound_mps2 = None;
    e.terminal_fuel_kg = None;
    e.total_predicted_fuel_kg = None;
    e.admissible = false;
    e.reason = None;
    e.lateral_velocity_reversal_checked = false;
    let finish = live
        .physics_step
        .checked_add(plan.turn_ticks)
        .and_then(|t| t.checked_add(plan.burn_ticks))
        .and_then(|t| t.checked_add(e.coast_physics_ticks))
        .and_then(|t| t.checked_add(ticks))
        .and_then(|t| t.checked_add(TERMINAL_EXTRA_TICKS))
        .context("finish clock overflow")?;
    e.predicted_finish_physics_step = Some(finish);
    if finish > deadline {
        e.reason = Some("predicted terminal finish exceeds original absolute deadline".into());
        return Ok(e);
    }
    let r = build_reference(
        context,
        &BodyAwareTerminalPolicyV1::default(),
        e.predicted_entry,
        ticks,
    )?;
    if !terminal_forward_velocity_valid(context, &r) {
        e.lateral_velocity_reversal_checked = true;
        e.reason = Some("terminal reference reverses forward horizontal velocity".into());
        return Ok(e);
    }
    e.lateral_velocity_reversal_checked = true;
    let bound = conservative_terminal_thrust_bound(context, &r);
    if !bound.is_finite() {
        bail!("nonfinite terminal bound");
    }
    e.coupled_thrust_bound_mps2 = Some(bound);
    let fuel = live.fuel_kg - plan.estimated_fuel_kg;
    let limit = MAX_THRUST_FRACTION * context.vehicle.max_thrust_n
        / (context.vehicle.dry_mass_kg + fuel.max(0.0)).max(1.0);
    if bound > limit {
        e.reason = Some(format!(
            "coupled terminal thrust bound {bound:.6} exceeds 0.925 incoming-mass limit {limit:.6}"
        ));
        return Ok(e);
    }
    let first = paired_reference_thrust(context, &r, 0);
    let first_angle = first.x.atan2(first.y);
    let (align, _) = match turn_ticks_for(
        context,
        if plan.turn_ticks > 0 {
            plan.target_attitude_rad
        } else {
            live.attitude_rad
        },
        Vec2::new(first_angle.sin(), first_angle.cos()),
    ) {
        Ok(turn) => turn,
        Err(error) => {
            e.reason = Some(format!("terminal first-thrust alignment: {error:#}"));
            return Ok(e);
        }
    };
    if e.coast_physics_ticks < align {
        e.reason = Some(format!(
            "terminal entry has {} coasting ticks for {align} ticks of first-thrust alignment",
            e.coast_physics_ticks
        ));
        return Ok(e);
    }
    match estimate_terminal_fuel(context, &r, fuel) {
        Ok(used) => {
            e.terminal_fuel_kg = Some(used);
            e.total_predicted_fuel_kg = Some(plan.estimated_fuel_kg + used);
            e.admissible = true;
        }
        Err(error) => e.reason = Some(format!("terminal throttle/fuel screen: {error:#}")),
    }
    Ok(e)
}

pub(super) fn evaluate_timed_seed(
    context: &RunContext,
    live: &SimulationState,
    deadline: u64,
    spec: SeedSpec,
) -> Result<(NominalSeedEvidenceV1, Vec<Value>)> {
    // The unchanged evaluator performs acquisition once and supplies the baseline screens.
    let mut seed = evaluate_seed(context, live, deadline, spec)?;
    let mut traces = Vec::new();
    if seed.entry_screens.is_empty() {
        return Ok((seed, traces));
    }
    let plan = plan_from_seed(&seed, live)?;
    for e in &mut seed.entry_screens {
        let baseline = e.clone();
        let mut screens = vec![baseline.clone()];
        let mut metadata = vec![
            json!({"candidate_index":0,"provenance":"unchanged_vertical_baseline","physics_ticks":e.terminal_physics_ticks}),
        ];
        let entry = e.predicted_entry;
        if !entry.position_m.x.is_finite()
            || !entry.position_m.y.is_finite()
            || !entry.velocity_mps.x.is_finite()
            || !entry.velocity_mps.y.is_finite()
            || e.coupled_thrust_bound_mps2.is_some_and(|b| !b.is_finite())
        {
            bail!("nonfinite entry screen");
        }
        let height = entry.position_m.y
            - context.target_pad.surface_y_m
            - context.vehicle.geometry.touchdown_base_offset_m;
        let direction = forward_direction(context, entry.position_m.x, entry.velocity_mps.x);
        let entry_tick = live
            .physics_step
            .checked_add(plan.turn_ticks)
            .and_then(|t| t.checked_add(plan.burn_ticks))
            .and_then(|t| t.checked_add(e.coast_physics_ticks))
            .context("entry clock overflow")?;
        if !baseline.admissible && height > TARGET_ENTRY_RESERVE_M && entry.velocity_mps.y < 0.0 {
            for c in alternative_durations(
                direction * (context.target_pad.center_x_m - entry.position_m.x),
                direction * entry.velocity_mps.x,
                context.sim.physics_dt_s(),
                entry_tick,
                deadline,
                baseline.terminal_physics_ticks.unwrap_or(0),
            )? {
                screens.push(screen_duration(
                    context,
                    live,
                    deadline,
                    &plan,
                    &baseline,
                    c.physics_ticks,
                )?);
                metadata.push(serde_json::to_value(c)?);
            }
        }
        let selected = if baseline.admissible {
            Some(0)
        } else {
            screens
                .iter()
                .enumerate()
                .filter(|(_, s)| s.admissible)
                .min_by(|(ia, a), (ib, b)| {
                    a.total_predicted_fuel_kg
                        .unwrap_or(f64::INFINITY)
                        .total_cmp(&b.total_predicted_fuel_kg.unwrap_or(f64::INFINITY))
                        .then_with(|| {
                            a.predicted_finish_physics_step
                                .cmp(&b.predicted_finish_physics_step)
                        })
                        .then_with(|| ia.cmp(ib))
                })
                .map(|(i, _)| i)
        };
        traces.push(json!({"seed_id":seed.seed_id,"entry_index":e.entry_index,
            "candidates":metadata,"screens":screens,"selected_candidate_index":selected,
            "baseline_retained":baseline.admissible,"maximum_candidates":3}));
        if let Some(i) = selected {
            *e = screens[i].clone();
        }
    }
    seed.selected_entry_index = seed
        .entry_screens
        .iter()
        .enumerate()
        .filter(|(_, s)| s.admissible)
        .min_by(|(ia, a), (ib, b)| {
            a.total_predicted_fuel_kg
                .unwrap_or(f64::INFINITY)
                .total_cmp(&b.total_predicted_fuel_kg.unwrap_or(f64::INFINITY))
                .then_with(|| {
                    a.predicted_finish_physics_step
                        .cmp(&b.predicted_finish_physics_step)
                })
                .then_with(|| ia.cmp(ib))
        })
        .map(|(i, _)| i);
    seed.status = if seed.selected_entry_index.is_some() {
        "shortlisted_estimate"
    } else {
        "finite_miss"
    }
    .into();
    if let Some(i) = seed.selected_entry_index {
        seed.estimated_fuel_kg = seed.entry_screens[i].total_predicted_fuel_kg;
        seed.predicted_finish_physics_step = seed.entry_screens[i].predicted_finish_physics_step;
        seed.reason = None;
    } else {
        seed.reason = Some(
            seed.entry_screens
                .iter()
                .filter_map(|s| s.reason.as_deref())
                .take(4)
                .collect::<Vec<_>>()
                .join("; "),
        );
    }
    seed.identity = seed_identity(&seed)?;
    Ok((seed, traces))
}

fn witness_metrics(
    context: &RunContext,
    live: &SimulationState,
    w: &NominalWitnessEvidenceV1,
) -> Result<Value> {
    let demand = if let Some(p) = &w.target_plane_witness {
        Some(diagnostic::program_demand(
            context,
            live,
            &w.commands,
            p.physics_step,
        )?)
    } else {
        None
    };
    let terminal_budget = demand
        .as_ref()
        .and_then(|d| d["terminal_bridge"]["thrust_fraction_le_0_925"].as_bool());
    let acquisition_budget = demand
        .as_ref()
        .and_then(|d| d["nominal_acquisition"]["thrust_fraction_le_0_925"].as_bool());
    let body = w
        .terrain_audit
        .as_ref()
        .filter(|a| !a.reserve_phases.is_empty())
        .map(|a| {
            a.reserve_phases.iter().all(|p| {
                p.first_reserve_violation_physics_step.is_none() && p.first_query_error.is_none()
            })
        });
    Ok(
        json!({"seed_id":w.seed_id,"entry_index":w.entry_index,"phase_demand":demand,
        "terminal_thrust_budget_passed":terminal_budget,"acquisition_thrust_budget_passed":acquisition_budget,
        "body_reserve_passed":body,"terrain_landed":w.terrain_audit.as_ref().is_some_and(|a|a.landed_on_target),
        "replay_passed":w.independent_replay_passed&&w.endpoint_state_agreement}),
    )
}

pub fn run_waypoint_v2_terminal_time(corpus_path: &Path, output: &Path) -> Result<Value> {
    let root = repo_root()?;
    let bytes = fs::read(corpus_path)?;
    verify_approved_corpus_sha256(&sha256_bytes(&bytes)?)?;
    let corpus: WaypointV2NominalCharacterizationCorpusV1 = serde_json::from_slice(&bytes)?;
    validate_corpus(&corpus)?;
    verify_bindings(&root, &corpus.bindings)?;
    let old = read_pinned(&root, DIAGNOSTIC, DIAGNOSTIC_SHA)?;
    let baseline = read_pinned(&root, BASELINE, BASELINE_SHA)?;
    let mut bindings = diagnostic::diagnostic_bindings(&root, &corpus, corpus_path)?;
    for path in [
        DIAGNOSTIC,
        BASELINE,
        "docs/waypoint_v2_terminal_time_plan.md",
    ] {
        if !bindings.iter().any(|b| b.path == path) {
            bindings.push(WaypointV2NominalCharacterizationBindingV1 {
                path: path.into(),
                sha256: sha256_bytes(&fs::read(root.join(path))?)?,
            });
        }
    }
    bindings.sort_by(|a, b| a.path.cmp(&b.path));
    verify_bindings(&root, &bindings)?;
    reserve_output_root(output)?;
    write_create_only(&output.join("bindings.json"), &bindings)?;
    let mut errors = Vec::new();
    let mut controls = Vec::new();
    for control in old["controls"]
        .as_array()
        .context("missing sealed controls")?
    {
        let id = control["case_id"].as_str().context("control ID missing")?;
        let row = corpus
            .rows
            .iter()
            .find(|r| r.id == format!("clear_start:{id}"))
            .context("control row missing")?;
        match matched_control(row, control) {
            Ok(r) => controls.push(r),
            Err(e) => {
                errors.push(format!("{id}: {e:#}"));
                controls.push(
                    json!({"case_id":id,"screen_passed":false,"integrity_error":format!("{e:#}")}),
                );
            }
        }
    }
    let matched_gate = errors.is_empty()
        && controls.len() == 8
        && controls.iter().all(|r| r["screen_passed"] == true);
    let mut rows = Vec::new();
    let mut synthetics = Vec::new();
    let mut timings = Vec::new();
    if matched_gate {
        for row in &corpus.rows {
            let start = std::time::Instant::now();
            let result = evaluate_timed_row(row);
            timings.push(json!({"row_id":row.id,"elapsed_s":start.elapsed().as_secs_f64()}));
            match result {
                Ok((e, traces, metrics)) => {
                    errors.extend(
                        e.integrity_errors
                            .iter()
                            .map(|x| format!("{}: {x}", row.id)),
                    );
                    rows.push(
                        json!({"evidence":e,"duration_traces":traces,"witness_metrics":metrics}),
                    );
                }
                Err(e) => {
                    errors.push(format!("{}: {e:#}", row.id));
                    rows.push(json!({"row_id":row.id,"integrity_error":format!("{e:#}")}));
                }
            }
        }
        let source = corpus
            .rows
            .iter()
            .find(|r| r.population == "clear_start")
            .context("no synthetic source")?;
        for (scenario, label) in synthetic_scenarios(
            &source.scenario,
            &source.source_pad_id,
            &source.target_pad_id,
        )? {
            let row = WaypointV2NominalCharacterizationCorpusRowV1 {
                id: format!("synthetic:{}", scenario.id),
                population: "synthetic_condition".into(),
                label,
                scenario,
                source_pad_id: source.source_pad_id.clone(),
                target_pad_id: source.target_pad_id.clone(),
                absolute_deadline_physics_step: source.absolute_deadline_physics_step,
                expected_state: None,
                prefix_updates: Vec::new(),
            };
            let start = std::time::Instant::now();
            let result = evaluate_timed_row(&row);
            timings.push(json!({"row_id":row.id,"elapsed_s":start.elapsed().as_secs_f64()}));
            match result {
                Ok((e, traces, metrics)) => {
                    errors.extend(
                        e.integrity_errors
                            .iter()
                            .map(|x| format!("{}: {x}", row.id)),
                    );
                    synthetics.push(
                        json!({"evidence":e,"duration_traces":traces,"witness_metrics":metrics}),
                    );
                }
                Err(e) => {
                    errors.push(format!("{}: {e:#}", row.id));
                    synthetics.push(json!({"row_id":row.id,"integrity_error":format!("{e:#}")}));
                }
            }
        }
    }
    let verified = match verify_bindings(&root, &bindings) {
        Ok(()) => true,
        Err(e) => {
            errors.push(format!("post-run bindings: {e:#}"));
            false
        }
    };
    let mut artifact = json!({"schema_id":"waypoint_v2_terminal_time_v1","research_id":ID,
        "corpus_sha256":APPROVED_CORPUS_SHA256,"bindings":bindings,"bindings_verified_before_and_after":verified,
        "matched_entry_gate_passed":matched_gate,"controls":controls,"retained_rows":rows,"synthetic_rows":synthetics,
        "baseline_comparison":baseline["retained_rows"].as_array().context("baseline rows missing")?.iter()
            .map(|r|json!({"row_id":r["row_id"],"status":r["status"],"witness_count":r["witnesses"].as_array().map(Vec::len)})).collect::<Vec<_>>(),
        "broad_study_executed":matched_gate,"integrity_passed":errors.is_empty(),"integrity_errors":errors,
        "maximum_duration_candidates_per_entry":3,"maximum_physical_witnesses_per_row":3,
        "runtime_policy_changed":false,"scope":"bounded research and review; not usable planner acceptance","identity":""});
    artifact["identity"] = Value::String(stable_digest(&artifact)?);
    write_create_only(&output.join("summary.json"), &artifact)?;
    write_create_only(&output.join("timings.json"), &timings)?;
    Ok(artifact)
}

fn evaluate_timed_row(
    row: &WaypointV2NominalCharacterizationCorpusRowV1,
) -> Result<(NominalCharacterizationRowEvidenceV1, Vec<Value>, Vec<Value>)> {
    let mut duration_traces = Vec::new();
    let mut metrics = Vec::new();
    let context = RunContext::from_scenario(&row.scenario).map_err(anyhow::Error::msg)?;
    let mut live = SimulationState::new(&context)?;
    let mut expected_state_match = None;
    if let Some(expected) = &row.expected_state {
        let mut prefix_index = 0_usize;
        while live.physics_step < expected.physics_step {
            if live.is_terminal() {
                bail!(
                    "ordinary original-prefix replay became terminal at H{} before expected H{}",
                    live.physics_step,
                    expected.physics_step
                );
            }
            if live.physics_step.is_multiple_of(HELD_TICKS) {
                let update = row
                    .prefix_updates
                    .get(prefix_index)
                    .context("original command prefix ended before its H boundary")?;
                if update.physics_step != live.physics_step {
                    bail!(
                        "original command prefix lost cadence at H{}",
                        live.physics_step
                    );
                }
                live.set_command(update.command);
                prefix_index += 1;
            }
            // Retained handoffs are reconstructed through the same ordinary
            // contact/progress transition as the origin run, never from the
            // expected snapshot or the neutral witness replay seam.
            live.step(&context);
        }
        if prefix_index != row.prefix_updates.len() {
            bail!("original source replay did not consume its full prefix");
        }
        let actual = SimulationStateSnapshotV1::from_state(&live);
        let match_evidence = compare_full_snapshot(&actual, expected);
        if !match_evidence.matched {
            bail!(
                "fresh full-prefix replay disagrees with expected H{} snapshot (max float delta {})",
                expected.physics_step,
                match_evidence.maximum_float_delta
            );
        }
        expected_state_match = Some(match_evidence);
    }

    let incoming_state = SimulationStateSnapshotV1::from_state(&live);
    let mut ground_prep = if row.population == "clear_start" {
        Some(ground_prep(
            &context,
            &row.scenario,
            &row.source_pad_id,
            &row.target_pad_id,
            &mut live,
            row.absolute_deadline_physics_step,
        )?)
    } else {
        None
    };
    if let Some(prep) = &mut ground_prep {
        verify_ground_prep_replay(&context, prep)?;
    }
    let request = research_request(&row.scenario, &row.source_pad_id, &row.target_pad_id);
    let mut evidence = NominalCharacterizationRowEvidenceV1 {
        row_id: row.id.clone(),
        population: row.population.clone(),
        label: row.label.clone(),
        source_pad_id: row.source_pad_id.clone(),
        target_pad_id: row.target_pad_id.clone(),
        initial_physics_step: incoming_state.physics_step,
        initial_fuel_kg: incoming_state.fuel_kg,
        original_deadline_physics_step: row.absolute_deadline_physics_step,
        expected_state_match,
        prefix_update_count: row.prefix_updates.len(),
        ground_prep,
        seeds: Vec::new(),
        shortlist_seed_ids: Vec::new(),
        witnesses: Vec::new(),
        finite_miss_reasons: Vec::new(),
        integrity_errors: Vec::new(),
        status: "finite_miss".to_owned(),
        identity: String::new(),
    };
    if let Some(prep) = &evidence.ground_prep
        && let Some(error) = &prep.integrity_error
    {
        evidence.integrity_errors.push(error.clone());
        evidence.status = "integrity_error".into();
        evidence.identity = row_identity(&evidence)?;
        return Ok((evidence, duration_traces, metrics));
    }
    if let Some(prep) = &evidence.ground_prep
        && !prep.success
    {
        evidence.finite_miss_reasons.push(
            prep.reason
                .clone()
                .unwrap_or_else(|| "ground_prep_not_cleared".into()),
        );
        evidence.identity = row_identity(&evidence)?;
        return Ok((evidence, duration_traces, metrics));
    }

    let seed_specs = generate_seed_specs(&context, &live)?;
    if seed_specs.is_empty() {
        evidence
            .finite_miss_reasons
            .push("no_finite_virtual_arrival_seed".into());
    }
    let mut evaluated = Vec::with_capacity(seed_specs.len());
    for seed in seed_specs {
        let (seed_evidence, traces) =
            evaluate_timed_seed(&context, &live, row.absolute_deadline_physics_step, seed)?;
        duration_traces.extend(traces);
        if seed_evidence.status == "integrity_error" {
            evidence.integrity_errors.push(format!(
                "{}: {}",
                seed_evidence.seed_id,
                seed_evidence
                    .reason
                    .as_deref()
                    .unwrap_or("acquisition estimate integrity failure")
            ));
        }
        evaluated.push(seed_evidence);
    }
    if evaluated.len() > MAX_SEEDS {
        bail!("seed budget exceeded");
    }
    let mut ranked = evaluated
        .iter()
        .enumerate()
        .filter_map(|(index, seed)| {
            seed.selected_entry_index
                .map(|entry_index| (index, entry_index))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_seed, left_entry), (right_seed, right_entry)| {
        compare_rank(
            &evaluated[*left_seed],
            *left_entry,
            &evaluated[*right_seed],
            *right_entry,
        )
    });
    let shortlist = ranked
        .into_iter()
        .take(MAX_PHYSICAL_WITNESSES)
        .collect::<Vec<_>>();
    evidence.shortlist_seed_ids = shortlist
        .iter()
        .map(|(index, _)| evaluated[*index].seed_id.clone())
        .collect();
    for (seed_index, entry_index) in shortlist {
        let seed = &evaluated[seed_index];
        let entry = &seed.entry_screens[entry_index];
        match materialize_witness(
            &context,
            &request,
            &live,
            row.absolute_deadline_physics_step,
            seed,
            entry,
        ) {
            Ok(witness) => {
                metrics.push(witness_metrics(&context, &live, &witness)?);
                if witness.status == "integrity_error" {
                    evidence.integrity_errors.push(format!(
                        "{}: independent replay or endpoint mismatch for {}",
                        row.id, witness.seed_id
                    ));
                }
                evidence.witnesses.push(witness);
            }
            Err(failure) => record_materialization_failure(&mut evidence, &seed.seed_id, failure),
        }
    }
    evidence.finite_miss_reasons.extend(
        evaluated
            .iter()
            .filter(|seed| seed.selected_entry_index.is_none())
            .map(|seed| {
                format!(
                    "{}: {}",
                    seed.seed_id,
                    seed.reason.as_deref().unwrap_or("no admissible entry")
                )
            }),
    );
    evidence.seeds = evaluated;
    evidence.status = if !evidence.integrity_errors.is_empty() {
        "integrity_error".to_owned()
    } else if evidence.witnesses.iter().any(|witness| {
        witness.independent_replay_passed
            && witness.target_plane_witness.as_ref().is_some_and(|plane| {
                plane.safe_by_existing_target_plane_mirror && plane.proposal_endpoint_contact_match
            })
    }) {
        "free_space_target_plane_witness".to_owned()
    } else {
        "finite_miss".to_owned()
    };
    evidence.identity = row_identity(&evidence)?;
    Ok((evidence, duration_traces, metrics))
}

#[cfg(test)]
mod tests;
