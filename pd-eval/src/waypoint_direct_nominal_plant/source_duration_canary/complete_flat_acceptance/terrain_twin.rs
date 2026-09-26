//! Terrain-only counterfactual over an accepted generated witness. Diagnostic,
//! not a generated witness and never eligible for planner selection.
use super::*;
use crate::WaypointDirectNominalDirectGenerationArtifact;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FixedCommandTerrainTwinEvidence {
    pub baseline_wrapper_identity: String,
    pub terrain_scenario_identity: String,
    pub logged_tick_count: u64,
    pub physics_ticks_replayed: u64,
    pub commands_and_states_identical_until_terrain_contact: bool,
    pub ordinary_neutral_states_match: bool,
    pub first_terrain_contact_step: Option<u64>,
    pub first_contact_classification: Option<String>,
    pub first_contact_state: Option<PlantStateEvidence>,
    pub clearance_scan: GeometryClearanceScanEvidence,
    pub terrain_blocked: bool,
    pub first_command_mismatch: Option<ReplayValidationMismatchEvidence>,
    pub identity: String,
}

/// Replays exact requested commands at their original global held-command
/// cadence, changing only terrain and descriptive labels. The baseline's
/// stored flags are never substituted for recomputed counterfactual evidence.
pub fn evaluate_waypoint_direct_fixed_command_terrain_twin(
    flat: &WaypointDirectNominalDirectGenerationArtifact,
    terrain_scenario: &pd_core::ScenarioSpec,
) -> Result<FixedCommandTerrainTwinEvidence> {
    let mut input = flat.clone();
    input.identity.clear();
    if stable_digest(&input)? != flat.identity {
        bail!("terrain twin baseline artifact semantic identity mismatch");
    }
    validate_terrain_only_change(&flat.request.scenario, terrain_scenario)?;
    let selection = flat
        .selection
        .as_ref()
        .context("terrain twin needs accepted flat selection")?;
    let row = flat
        .rows
        .get(selection.row_index)
        .context("selected row missing")?;
    let wrapper = row.wrapper.as_ref().context("selected wrapper missing")?;
    if !row.accepted || !wrapper.accepted || wrapper.wrapper_identity != selection.wrapper_identity
    {
        bail!("terrain twin baseline must be a complete accepted witness");
    }
    let analysis = row
        .source_duration
        .as_ref()
        .and_then(|source| source.launch_and_analytical_screen.as_ref())
        .context("selected launch evidence missing")?;
    let schedule = row
        .paired_schedule
        .as_ref()
        .context("selected paired schedule missing")?;
    let run = LaunchFeasibilityCadenceRunEvidence {
        cadence: HELD_CADENCE.to_owned(),
        launch: analysis.launch.clone(),
        reseeded_bridge: analysis.reseeded_bridge.clone(),
        rollout: schedule
            .full_flight_rollout
            .clone()
            .context("selected full rollout missing")?,
    };
    let baseline_context =
        RunContext::from_scenario(&flat.request.scenario).map_err(anyhow::Error::msg)?;
    let authoritative = replay_logged_cadence(&baseline_context, &run)?;
    if !authoritative.trace.passed
        || authoritative.first_contact != schedule.full_flight_first_contact
        || authoritative
            .first_contact
            .as_ref()
            .is_none_or(|contact| contact.classification != "stable_touchdown_on_target")
    {
        bail!("terrain twin baseline authoritative replay failed");
    }
    let terrain_context =
        RunContext::from_scenario(terrain_scenario).map_err(anyhow::Error::msg)?;
    let find_pad = |id: &str| -> Result<PadInputV2> {
        let pad = terrain_scenario
            .world
            .landing_pads
            .iter()
            .find(|pad| pad.id == id)
            .context("terrain twin pad missing")?;
        Ok(PadInputV2 {
            center_x_m: pad.center_x_m,
            surface_y_m: pad.surface_y_m,
            width_m: pad.width_m,
        })
    };
    let policy = ClearancePolicy {
        source_pad: flat_pad_bounds(&terrain_context, &find_pad(&flat.source_pad_id)?),
        target_pad: flat_pad_bounds(&terrain_context, &find_pad(&flat.target_pad_id)?),
        minimum_clearance_m: flat.policy.analytical_policy.minimum_clearance_m,
    };
    if !policy.source_pad.flat || !policy.target_pad.flat {
        bail!("terrain twin must retain supported flat source and target pads");
    }
    replay_terrain_ticks(
        &baseline_context,
        &terrain_context,
        &replay_log_ticks(&run),
        policy,
        &selection.wrapper_identity,
        stable_digest(terrain_scenario)?,
    )
}

fn validate_terrain_only_change(
    baseline: &pd_core::ScenarioSpec,
    twin: &pd_core::ScenarioSpec,
) -> Result<()> {
    let mut expected = baseline.clone();
    expected.world.terrain = twin.world.terrain.clone();
    expected.id.clone_from(&twin.id);
    expected.name.clone_from(&twin.name);
    expected.description.clone_from(&twin.description);
    expected.tags.clone_from(&twin.tags);
    expected.metadata.clone_from(&twin.metadata);
    if expected != *twin {
        bail!("counterfactual changes more than terrain and descriptive labels");
    }
    Ok(())
}

fn replay_terrain_ticks(
    baseline: &RunContext,
    terrain: &RunContext,
    logs: &[ReplayLogTick<'_>],
    policy: ClearancePolicy,
    wrapper_identity: &str,
    terrain_identity: String,
) -> Result<FixedCommandTerrainTwinEvidence> {
    let mut reference = SimulationState::new(baseline)?;
    let mut ordinary = SimulationState::new(terrain)?;
    let mut neutral = SimulationState::new(terrain)?;
    let mut evidence = FixedCommandTerrainTwinEvidence {
        baseline_wrapper_identity: wrapper_identity.to_owned(),
        terrain_scenario_identity: terrain_identity,
        logged_tick_count: logs.len() as u64,
        physics_ticks_replayed: 0,
        commands_and_states_identical_until_terrain_contact: true,
        ordinary_neutral_states_match: true,
        first_terrain_contact_step: None,
        first_contact_classification: None,
        first_contact_state: None,
        clearance_scan: empty_clearance_scan(),
        terrain_blocked: false,
        first_command_mismatch: None,
        identity: String::new(),
    };
    for tick in logs {
        if tick.physics_step != reference.physics_step + 1
            || tick.physics_step != neutral.physics_step + 1
        {
            record_replay_mismatch(
                &mut evidence.first_command_mismatch,
                Some(tick.physics_step),
                "physics_step",
                (reference.physics_step + 1).to_string(),
                tick.physics_step.to_string(),
            );
            evidence.commands_and_states_identical_until_terrain_contact = false;
            break;
        }
        if reference
            .physics_step
            .is_multiple_of(baseline.sim.control_interval_steps())
        {
            let command = Command {
                throttle_frac: tick.commanded_throttle_frac,
                target_attitude_rad: tick.desired_target_attitude_rad,
            };
            reference.set_command(command);
            ordinary.set_command(command);
            neutral.set_command(command);
        }
        if reference.held_command.throttle_frac != tick.commanded_throttle_frac
            || reference.held_command.target_attitude_rad != tick.held_target_attitude_rad
            || reference.attitude_rad != tick.attitude_before_step_rad
            || reference.held_command != neutral.held_command
            || !same_motion_and_fuel(&reference, &neutral)
        {
            record_replay_mismatch(
                &mut evidence.first_command_mismatch,
                Some(tick.physics_step),
                "frozen_command_or_prefix_state",
                "exact frozen command and motion/fuel prefix",
                "mismatch",
            );
            evidence.commands_and_states_identical_until_terrain_contact = false;
        }
        let reference_contact = reference.step_physics_and_classify_contact(baseline);
        let contact = neutral.step_physics_and_classify_contact(terrain);
        let ordinary_events = ordinary.step(terrain);
        if !same_motion_and_fuel(&reference, &neutral) {
            evidence.commands_and_states_identical_until_terrain_contact = false;
        }
        evidence.ordinary_neutral_states_match &=
            same_ordinary_neutral_state(&ordinary, &neutral, &contact)
                && event_contact_label(&ordinary_events) == contact_classification_label(&contact);
        evidence.physics_ticks_replayed += 1;
        evidence.clearance_scan.poststep_state_count += 1;
        let label = contact_classification_label(&contact);
        if label != "none" {
            evidence.first_contact_classification = Some(label.to_owned());
            evidence.first_contact_state = Some(plant_state_evidence(&neutral, terrain));
            if contact != reference_contact || label != tick.expected_contact {
                evidence.first_terrain_contact_step = Some(tick.physics_step);
            }
            break;
        }
        record_airborne_clearance(
            terrain,
            &neutral,
            tick.physics_step,
            tick.phase,
            policy,
            &mut evidence.clearance_scan,
        );
        if reference_contact != ContactClassification::None {
            evidence.commands_and_states_identical_until_terrain_contact = false;
            record_replay_mismatch(
                &mut evidence.first_command_mismatch,
                Some(tick.physics_step),
                "reference_contact_before_twin",
                "no reference contact before twin",
                "reference contacted first",
            );
            break;
        }
    }
    evidence.terrain_blocked = !evidence.clearance_scan.all_airborne_states_passed
        || evidence.first_terrain_contact_step.is_some();
    evidence.identity = stable_digest(&evidence)?;
    Ok(evidence)
}

/// Terrain-dependent cached clearances deliberately excluded. Neutral stepping
/// preserves incoming contact velocity, so the final prefix comparison is exact.
fn same_motion_and_fuel(a: &SimulationState, b: &SimulationState) -> bool {
    a.physics_step == b.physics_step
        && a.sim_time_s == b.sim_time_s
        && a.position_m == b.position_m
        && a.velocity_mps == b.velocity_mps
        && a.attitude_rad == b.attitude_rad
        && a.angular_rate_radps == b.angular_rate_radps
        && a.fuel_kg == b.fuel_kg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_only_change_refuses_physical_context_changes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let baseline = crate::waypoint_direct_known_flat_generation_request(root)
            .unwrap()
            .scenario;
        let mut twin = baseline.clone();
        twin.id = "different-label".into();
        assert!(validate_terrain_only_change(&baseline, &twin).is_ok());
        twin.sim.controller_hz = 120;
        assert!(validate_terrain_only_change(&baseline, &twin).is_err());
        twin = baseline.clone();
        twin.initial_state.angular_rate_radps = 0.01;
        assert!(validate_terrain_only_change(&baseline, &twin).is_err());
    }

    #[test]
    fn fixed_replay_detects_noncontiguous_and_off_cadence_commands() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let scenario = crate::waypoint_direct_known_flat_generation_request(root)
            .unwrap()
            .scenario;
        let context = RunContext::from_scenario(&scenario).unwrap();
        let pad = |x| FlatPadBounds {
            left_m: x - 18.0,
            right_m: x + 18.0,
            surface_y_m: 0.0,
            flat: true,
        };
        let policy = ClearancePolicy {
            source_pad: pad(-800.0),
            target_pad: pad(0.0),
            minimum_clearance_m: 5.0,
        };
        let tick = |step, throttle| ReplayLogTick {
            physics_step: step,
            phase: "upright",
            expected_contact: "none",
            desired_target_attitude_rad: 0.0,
            held_target_attitude_rad: 0.0,
            commanded_throttle_frac: throttle,
            attitude_before_step_rad: 0.0,
            logged_applied_throttle_frac: None,
        };
        let good = replay_terrain_ticks(
            &context,
            &context,
            &[tick(1, 1.0), tick(2, 1.0)],
            policy,
            "test",
            "terrain".into(),
        )
        .unwrap();
        assert!(good.commands_and_states_identical_until_terrain_contact);
        assert!(good.ordinary_neutral_states_match);
        assert!(!good.terrain_blocked);
        let bad = replay_terrain_ticks(
            &context,
            &context,
            &[tick(1, 1.0), tick(2, 0.5)],
            policy,
            "test",
            "terrain".into(),
        )
        .unwrap();
        assert!(!bad.commands_and_states_identical_until_terrain_contact);
        assert!(bad.first_command_mismatch.is_some());
        let missing = replay_terrain_ticks(
            &context,
            &context,
            &[tick(2, 1.0)],
            policy,
            "test",
            "terrain".into(),
        )
        .unwrap();
        assert!(!missing.commands_and_states_identical_until_terrain_contact);
    }

    #[test]
    fn terrain_contact_preserves_incoming_motion_and_stops_the_twin() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let baseline = crate::waypoint_direct_known_flat_generation_request(root)
            .unwrap()
            .scenario;
        let mut raised = baseline.clone();
        // Synthetic replay-mechanics test, not a valid source-pad mission.
        let pd_core::TerrainDefinition::Heightfield { points_m } = &mut raised.world.terrain;
        for point in points_m {
            point.y = 0.001;
        }
        let baseline_context = RunContext::from_scenario(&baseline).unwrap();
        let raised_context = RunContext::from_scenario(&raised).unwrap();
        let bounds = FlatPadBounds {
            left_m: -818.0,
            right_m: -782.0,
            surface_y_m: 0.0,
            flat: true,
        };
        let policy = ClearancePolicy {
            source_pad: bounds,
            target_pad: bounds,
            minimum_clearance_m: 5.0,
        };
        let tick = ReplayLogTick {
            physics_step: 1,
            phase: "upright",
            expected_contact: "none",
            desired_target_attitude_rad: 0.0,
            held_target_attitude_rad: 0.0,
            commanded_throttle_frac: 1.0,
            attitude_before_step_rad: 0.0,
            logged_applied_throttle_frac: None,
        };
        let result = replay_terrain_ticks(
            &baseline_context,
            &raised_context,
            &[tick],
            policy,
            "synthetic",
            "raised".into(),
        )
        .unwrap();
        assert!(result.commands_and_states_identical_until_terrain_contact);
        assert!(result.ordinary_neutral_states_match);
        assert!(result.terrain_blocked);
        assert_eq!(result.first_terrain_contact_step, Some(1));
        assert_eq!(result.physics_ticks_replayed, 1);
        assert!(result.first_contact_state.unwrap().velocity_mps.y > 0.0);
    }
}
