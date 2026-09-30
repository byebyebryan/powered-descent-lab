//! Fresh old-policy control, never an input to canonical nominal selection.
use super::*;

pub(super) fn run_old_terrain_aware_source_only(
    request: &WaypointDirectNominalDirectGenerationRequest,
    canonical_peak_com_height_m: f64,
) -> CanonicalInitialDirectCanaryOldPolicyV1 {
    let started = Instant::now();
    let mut evidence = CanonicalInitialDirectCanaryOldPolicyV1 {
        status: "running".into(),
        source_binding_before_sha256: None,
        source_binding_after_sha256: None,
        source_unchanged_before_and_after: false,
        canonical_peak_com_height_m,
        complete_flown_peak_exceeds_canonical: None,
        decision: None,
        generation: None,
        execution: None,
        compute: None,
        peak_replay: None,
        elapsed_wall_time_us: 0,
        failures: Vec::new(),
    };
    if let Err(error) = evaluate_control(request, &mut evidence) {
        evidence
            .failures
            .push(format!("old-policy control: {error:#}"));
    }
    evidence.status = if evidence.failures.is_empty() {
        "passed"
    } else {
        "failed"
    }
    .into();
    evidence.elapsed_wall_time_us = elapsed_us(started);
    evidence
}

fn evaluate_control(
    request: &WaypointDirectNominalDirectGenerationRequest,
    evidence: &mut CanonicalInitialDirectCanaryOldPolicyV1,
) -> Result<()> {
    if !evidence.canonical_peak_com_height_m.is_finite() {
        bail!("canonical complete-flight peak must be finite");
    }
    // Only the physical request enters generation. No historical artifacts,
    // canonical commands, or a selected old row are supplied as seeds.
    let evaluation =
        evaluate_nominal_direct_flight(request, &BodyAwareTerminalPolicyV1::default())?;
    evidence.decision = Some(evaluation.decision);
    evidence.generation = evaluation.generation;
    evidence.compute = Some(evaluation.compute);
    let (selected_row_index, program) = match evidence.decision.as_ref() {
        Some(NominalDirectFlightDecisionV1::Direct {
            selected_row_index,
            program,
            ..
        }) => (*selected_row_index, program.as_ref().clone()),
        Some(other) => bail!(
            "fresh old-policy decision is {}, not Direct",
            other.status()
        ),
        None => bail!("fresh old-policy decision is missing"),
    };
    let witness = evidence
        .generation
        .as_ref()
        .and_then(|generation| generation.rows.get(selected_row_index))
        .and_then(|row| row.witness.as_ref())
        .context("fresh selected old-policy witness is missing")?;
    let (execution, ordinary) = execute_nominal_direct_flight_program(
        request,
        &BodyAwareTerminalPolicyV1::default(),
        witness,
        &program,
        evidence.compute.as_mut().expect("fresh compute retained"),
    )?;
    evidence.execution = Some(execution);
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let peak_replay = replay_peak(&context, &program, &ordinary.run)?;
    let exceeds = peak_replay.maximum_com_height_m > evidence.canonical_peak_com_height_m;
    evidence.complete_flown_peak_exceeds_canonical = Some(exceeds);
    let replay_passed = peak_replay.passed;
    evidence.peak_replay = Some(peak_replay);
    if !replay_passed {
        bail!("independent 120 Hz peak/full-state/contact replay did not pass");
    }
    if !exceeds {
        bail!("fresh old-policy complete flown peak does not exceed the fixed canonical peak");
    }
    Ok(())
}

struct PeakTrace {
    maximum_com_height_m: f64,
    maximum_com_height_physics_step: u64,
    final_state: SimulationStateSnapshotV1,
    incoming_contact: Option<pd_core::IncomingContactV1>,
}

fn step_peak(context: &RunContext, updates: &[FlightProgramUpdateV1]) -> Result<PeakTrace> {
    if context.sim.physics_hz != 120 || context.sim.controller_hz != 60 {
        bail!("old-policy peak control requires the original 120/60 Hz clocks");
    }
    if updates.is_empty()
        || updates.iter().enumerate().any(|(index, update)| {
            update.physics_step != index as u64 * 2
                || !update.command.throttle_frac.is_finite()
                || !update.command.target_attitude_rad.is_finite()
                || update.command != update.command.clamped()
        })
    {
        bail!("old-policy peak replay requires exact source-rest held-pair coverage");
    }
    let mut state = SimulationState::new(context)?;
    let mut maximum_com_height_m = state.position_m.y;
    let mut maximum_com_height_physics_step = state.physics_step;
    let mut incoming_contact = None;
    let mut index = 0;
    while !state.is_terminal() {
        if state.physics_step.is_multiple_of(2) {
            let update = updates
                .get(index)
                .context("old-policy peak schedule exhausted before ordinary termination")?;
            if update.physics_step != state.physics_step {
                bail!("old-policy peak replay update clock differs from actual state");
            }
            state.set_command(update.command);
            index += 1;
        }
        let transition = state.step_with_contact_report(context);
        if let Some(contact) = transition.incoming_contact {
            if contact.state.position_m.y > maximum_com_height_m {
                maximum_com_height_m = contact.state.position_m.y;
                maximum_com_height_physics_step = contact.state.physics_step;
            }
            if incoming_contact.replace(contact).is_some() {
                bail!("old-policy peak replay produced multiple first contacts");
            }
        }
        if state.position_m.y > maximum_com_height_m {
            maximum_com_height_m = state.position_m.y;
            maximum_com_height_physics_step = state.physics_step;
        }
    }
    if index != updates.len() || incoming_contact.is_none() {
        bail!("old-policy peak replay did not consume exactly the fresh program through contact");
    }
    Ok(PeakTrace {
        maximum_com_height_m,
        maximum_com_height_physics_step,
        final_state: SimulationStateSnapshotV1::from_state(&state),
        incoming_contact,
    })
}

fn replay_peak(
    context: &RunContext,
    program: &FlightProgramV1,
    ordinary: &RunArtifacts,
) -> Result<CanonicalInitialDirectCanaryPeakReplayV1> {
    program
        .validate_against_context(context)
        .map_err(anyhow::Error::msg)?;
    let peak = step_peak(context, &program.updates)?;
    let source = replay_stitched_from_source(context, &program.updates)?;
    let official = replay_simulation(context, &ordinary.manifest.controller_id, &source.actions)?;
    let source_actions_match =
        source.actions == ordinary.actions && source.actions == official.actions;
    let source_events_match = source.events == ordinary.events && source.events == official.events;
    let source_samples_match =
        source.samples == ordinary.samples && source.samples == official.samples;
    // Full state includes accumulated extrema and held commands, not only the
    // kinematics. Full IncomingContactV1 is before touchdown normalization.
    let ordinary_run_match =
        peak.final_state == source.final_state && peak.incoming_contact == source.incoming_contact;
    let ordinary_action_replay_match = official == *ordinary;
    let clock_matches = |run: &RunArtifacts| {
        run.manifest.physics_steps == peak.final_state.physics_step
            && run.manifest.sim_time_s == peak.final_state.sim_time_s
            && run.manifest.summary.fuel_remaining_kg.to_bits()
                == peak.final_state.fuel_kg.to_bits()
            && run.manifest.end_reason == peak.final_state.end_reason
            && run.manifest.physical_outcome == peak.final_state.physical_outcome
            && run.manifest.mission_outcome == peak.final_state.mission_outcome
    };
    let final_clock_match = clock_matches(ordinary) && clock_matches(&official);
    let safe_target_contact = peak.final_state.physical_outcome == PhysicalOutcome::LandedOnTarget
        && peak.final_state.mission_outcome == MissionOutcome::Success
        && peak.final_state.end_reason == EndReason::TouchdownOnTarget
        && peak.incoming_contact.as_ref().is_some_and(|contact| {
            matches!(
                contact.classification,
                pd_core::ContactClassification::StableTouchdown { on_target: true }
            )
        });
    let passed = ordinary_run_match
        && ordinary_action_replay_match
        && source_actions_match
        && source_events_match
        && source_samples_match
        && final_clock_match
        && safe_target_contact;
    Ok(CanonicalInitialDirectCanaryPeakReplayV1 {
        maximum_com_height_m: peak.maximum_com_height_m,
        maximum_com_height_physics_step: peak.maximum_com_height_physics_step,
        final_state: peak.final_state,
        incoming_contact: peak.incoming_contact,
        ordinary_run_match,
        ordinary_action_replay_match,
        official_replay: Some(official),
        ordinary_run: Some(ordinary.clone()),
        source_actions: source.actions,
        source_events: source.events,
        source_samples: source.samples,
        source_actions_match,
        source_events_match,
        source_samples_match,
        final_clock_match,
        safe_target_contact,
        passed,
        failure: (!passed)
            .then(|| "old-policy peak/source/official full replay equality gate failed".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_peak_replay_rejects_uncovered_initial_tick_without_flying() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let (_, request) = load_nominal_direct_operational_fresh_inputs(repo)
            .unwrap()
            .remove(0);
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let updates = [FlightProgramUpdateV1 {
            physics_step: 2,
            phase: "upright".into(),
            command: pd_core::Command {
                throttle_frac: 1.0,
                target_attitude_rad: 0.0,
            },
        }];
        assert!(step_peak(&context, &updates).is_err());
    }

    #[test]
    fn old_control_rejects_nonfinite_comparison_before_generation() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let (_, request) = load_nominal_direct_operational_fresh_inputs(repo)
            .unwrap()
            .remove(0);
        let evidence = run_old_terrain_aware_source_only(&request, f64::NAN);
        assert!(evidence.decision.is_none());
        assert!(evidence.generation.is_none());
        assert!(evidence.peak_replay.is_none());
        assert_eq!(evidence.failures.len(), 1);
    }
}
