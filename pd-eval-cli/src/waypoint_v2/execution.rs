//! Forward-only execution queries, exclusive segment ownership, and
//! original-source replay proofs. Snapshots are evidence, never restart inputs.

use anyhow::{Context, Result, bail, ensure};
use pd_core::{
    BoundedGuardFailureDispositionV1, BoundedGuardFailureV1, BoundedRunArtifactsV1,
    BoundedRunGuard, BoundedRunLimitsV1, BoundedRunStopCauseV1, Command, EndReason,
    FlightProgramUpdateV1, IncomingContactV1, MissionOutcome, PhysicalOutcome, RunContext,
    SimulationState, SimulationStateSnapshotV1, replay_simulation_bounded, run_simulation_bounded,
};
use pd_plan::waypoint_v2::{command_count, original_deadline};

use super::{WaypointV2FlightResult, WaypointV2Segment, WaypointV2SegmentKind};
use crate::{
    LocalClearingOrdinaryEvidenceV1, WaypointDirectNominalDirectGenerationRequest,
    clearing_body_reserve_query,
    local_clearing::{OrdinaryLive, advance_ordinary, full_ordinary_matches, snapshot_finite},
    nominal_body_reserve_query, nominal_direct_flight_identity,
    planner_flight::input::PAD_REST_TOLERANCE_M,
    planner_flight::terrain::phase_at_tick,
};

const CONTROLLER: &str = "waypoint_v2_supplied_commands";

pub(super) fn consumed(updates: &[FlightProgramUpdateV1], end: u64) -> Vec<FlightProgramUpdateV1> {
    updates
        .iter()
        .take_while(|u| u.physics_step < end)
        .cloned()
        .collect()
}

pub(super) fn ledger_updates(segments: &[WaypointV2Segment]) -> Vec<FlightProgramUpdateV1> {
    segments
        .iter()
        .flat_map(|s| s.updates.iter().cloned())
        .collect()
}

/// Segment ownership uses the command strictly before a post-step boundary.
/// The source exemption can belong only to the very first nominal segment.
struct FlightGuard<'a> {
    request: &'a WaypointDirectNominalDirectGenerationRequest,
    segments: &'a [WaypointV2Segment],
    diagnostic: bool,
}

impl FlightGuard<'_> {
    fn check(
        &self,
        context: &RunContext,
        state: &SimulationState,
        contact: Option<&IncomingContactV1>,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        let check = || -> Result<()> {
            if !snapshot_finite(state) {
                bail!("nonfinite full state");
            }
            if self.diagnostic {
                return Ok(());
            }
            if let Some(contact) = contact {
                if state.physical_outcome == PhysicalOutcome::LandedOnTarget
                    && state.mission_outcome == MissionOutcome::Success
                    && state.end_reason == EndReason::TouchdownOnTarget
                    && contact.state.physics_step == state.physics_step
                    && self.segments.iter().any(|s| {
                        s.kind != WaypointV2SegmentKind::LocalCorrection
                            && s.start_physics_step < state.physics_step
                            && state.physics_step <= s.end_physics_step
                            && phase_at_tick(&s.updates, state.physics_step) == "terminal_bridge"
                    })
                {
                    return Ok(());
                }
                bail!("unexpected active contact");
            }
            let segment = self.segments.iter().find(|s| {
                if state.physics_step == 0 {
                    s.start_physics_step == 0
                } else {
                    s.start_physics_step < state.physics_step
                        && state.physics_step <= s.end_physics_step
                }
            });
            let phase = if state.physics_step == 0 {
                "upright".into()
            } else {
                phase_at_tick(
                    &segment.context("missing segment guard ownership")?.updates,
                    state.physics_step,
                )
            };
            let (clearance, required) = if segment
                .is_some_and(|s| s.kind == WaypointV2SegmentKind::LocalCorrection)
            {
                clearing_body_reserve_query(context, self.request, state, "local", false)?
            } else {
                nominal_body_reserve_query(
                    context,
                    self.request,
                    state,
                    &phase,
                    state.physics_step == 0
                        || segment.is_some_and(|s| s.kind == WaypointV2SegmentKind::InitialNominal),
                )?
            };
            // Input admission already permits nanometre-scale pad-rest
            // rounding. Apply it only to the unchanged original source at
            // step zero, never to airborne reserves or later pad transitions.
            let initial_pad_rest = state.physics_step == 0
                && required == 0.0
                && state.position_m == context.initial_state.position_m
                && state.velocity_mps == context.initial_state.velocity_mps
                && state.attitude_rad == context.initial_state.attitude_rad
                && state.angular_rate_radps == context.initial_state.angular_rate_radps;
            let minimum_clearance = if initial_pad_rest {
                -PAD_REST_TOLERANCE_M
            } else {
                required
            };
            if clearance < minimum_clearance {
                bail!(
                    "segment reserve {clearance} below {minimum_clearance} at {}",
                    state.physics_step
                );
            }
            Ok(())
        };
        check().map_err(|e| {
            BoundedGuardFailureV1::new(
                BoundedGuardFailureDispositionV1::SafetyRejected,
                e.to_string(),
            )
        })
    }
}

impl BoundedRunGuard for FlightGuard<'_> {
    fn initial(
        &mut self,
        ctx: &RunContext,
        state: &SimulationState,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        self.check(ctx, state, None)
    }
    fn before_transition(
        &mut self,
        ctx: &RunContext,
        state: &SimulationState,
        _: Command,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        self.check(ctx, state, None)
    }
    fn after_transition(
        &mut self,
        ctx: &RunContext,
        state: &SimulationState,
        contact: Option<&IncomingContactV1>,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        self.check(ctx, state, contact)
    }
}

pub(super) fn prove_accumulated(
    request: &WaypointDirectNominalDirectGenerationRequest,
    context: &RunContext,
    live: &OrdinaryLive,
    segments: &[WaypointV2Segment],
    diagnostic: bool,
    deadline: u64,
) -> Result<BoundedRunArtifactsV1> {
    let endpoint = live.state.physics_step;
    ensure!(
        live.evidence.final_state.physics_step == endpoint
            && live.evidence.final_state.sim_time_s == live.state.sim_time_s,
        "live state and accumulated evidence endpoint differ"
    );
    prove_accumulated_evidence(
        request,
        context,
        &live.evidence,
        live.state.is_terminal(),
        segments,
        diagnostic,
        deadline,
    )
}

fn prove_accumulated_evidence(
    request: &WaypointDirectNominalDirectGenerationRequest,
    context: &RunContext,
    expected: &LocalClearingOrdinaryEvidenceV1,
    terminal: bool,
    segments: &[WaypointV2Segment],
    diagnostic: bool,
    deadline: u64,
) -> Result<BoundedRunArtifactsV1> {
    let updates = ledger_updates(segments);
    let endpoint = expected.final_state.physics_step;
    // Coverage belongs to controller pairs; an actual contact may terminate
    // inside its final pair. Do not move the physical endpoint or add commands.
    let coverage_end = if endpoint.is_multiple_of(2) {
        endpoint
    } else {
        if !terminal {
            bail!("flying proof endpoint is off the control clock");
        }
        endpoint
            .checked_add(1)
            .context("terminal command-pair end overflow")?
    };
    let limits = BoundedRunLimitsV1 {
        command_coverage_end_physics_step: coverage_end,
        hard_end_physics_step: deadline,
    };
    let mut index = 0;
    let mut guard = FlightGuard {
        request,
        segments,
        diagnostic,
    };
    let source = run_simulation_bounded(
        context,
        CONTROLLER,
        limits,
        |_, observation| {
            let u = updates
                .get(index)
                .ok_or_else(|| "whole-source command coverage gap".to_string())?;
            if u.physics_step != observation.physics_step {
                return Err("whole-source clock mismatch".into());
            }
            index += 1;
            Ok(u.command)
        },
        &mut guard,
    )?;
    let mut guard = FlightGuard {
        request,
        segments,
        diagnostic,
    };
    let replay =
        replay_simulation_bounded(context, CONTROLLER, &expected.actions, limits, &mut guard)?;
    if index != updates.len()
        || !full_ordinary_matches(expected, &source)
        || !full_ordinary_matches(expected, &replay)
        || source != replay
        || source.failure.is_some()
        || (!terminal
            && (source.stop != BoundedRunStopCauseV1::CoverageExhausted
                || !source.coverage_reached))
    {
        bail!(
            "full accumulated replay mismatch: commands {}/{}, source_state={}, source_contact={}, source_actions={}, source_events={}, source_samples={}, official={}, envelopes={}, stop={:?}, failure={:?}",
            index,
            updates.len(),
            expected.final_state == source.final_state,
            expected.incoming_contact == source.incoming_contact,
            expected.actions == source.run.actions,
            expected.events == source.run.events,
            expected.samples == source.run.samples,
            full_ordinary_matches(expected, &replay),
            source == replay,
            source.stop,
            source.failure
        );
    }
    Ok(source)
}

/// Replay a saved supported result from the original scenario source. This
/// validates identity and its original deadline, then proves both the recorded
/// segment command source and saved action prefix against the retained ordinary
/// evidence; it never reconstructs a stepped plant from the saved snapshot.
pub(crate) fn replay_saved_waypoint_v2_evidence(
    request: &WaypointDirectNominalDirectGenerationRequest,
    result: &WaypointV2FlightResult,
) -> Result<BoundedRunArtifactsV1> {
    result.policy.validate().map_err(anyhow::Error::msg)?;
    super::early_exit::validate_records(result)?;
    ensure!(
        result.input_identity == nominal_direct_flight_identity(&(request, &result.policy))?,
        "saved V2 result input identity does not match its complete request and policy"
    );
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let deadline = original_deadline(
        context.sim.max_time_s,
        request.policy.analytical_policy.mission_budget_s(),
    )
    .map_err(anyhow::Error::msg)?;
    ensure!(
        result.absolute_deadline_physics_step == Some(deadline),
        "saved V2 result original deadline differs from its request"
    );
    let expected = result
        .ordinary_flight
        .as_ref()
        .context("supported saved V2 result is missing ordinary flight evidence")?;
    ensure!(
        result.physical_outcome.as_ref() == Some(&expected.final_state.physical_outcome)
            && result.mission_outcome.as_ref() == Some(&expected.final_state.mission_outcome),
        "saved V2 result outcomes differ from its full ordinary endpoint"
    );
    let terminal = expected.final_state.end_reason != EndReason::Running;
    let proof = prove_accumulated_evidence(
        request,
        &context,
        expected,
        terminal,
        &result.segments,
        false,
        deadline,
    )?;
    ensure!(
        result
            .manifest
            .as_ref()
            .is_none_or(|manifest| manifest == &proof.run.manifest),
        "saved V2 manifest differs from original-source replay"
    );
    Ok(proof)
}

pub(super) fn append_segment(
    segments: &mut Vec<WaypointV2Segment>,
    kind: WaypointV2SegmentKind,
    identity: &str,
    entry: &SimulationStateSnapshotV1,
    live: &OrdinaryLive,
    updates: Vec<FlightProgramUpdateV1>,
) -> Result<()> {
    let end = live.state.physics_step;
    if entry.physics_step == end {
        return Ok(());
    }
    if segments.last().is_some_and(|s| s.end_state != *entry)
        || updates.len() as u64
            != command_count(entry.physics_step, end).map_err(anyhow::Error::msg)?
        || updates
            .iter()
            .enumerate()
            .any(|(i, u)| u.physics_step != entry.physics_step + i as u64 * 2)
    {
        bail!("segment full-state continuity or exclusive clock ownership invalid");
    }
    segments.push(WaypointV2Segment {
        kind,
        start_physics_step: entry.physics_step,
        end_physics_step: end,
        proposal_identity: identity.into(),
        updates,
        entry_state: entry.clone(),
        end_state: live.evidence.final_state.clone(),
    });
    Ok(())
}

/// Forward propagation from actual C, never SimulationState::new or a snapshot.
pub(super) fn query_to(
    context: &RunContext,
    current: &OrdinaryLive,
    updates: &[FlightProgramUpdateV1],
    tick: u64,
) -> Result<OrdinaryLive> {
    if tick < current.state.physics_step {
        bail!("query attempted to rewind live clock");
    }
    let mut query = current.clone();
    advance_ordinary(context, &mut query, &consumed(updates, tick), tick)?;
    if query.state.physics_step != tick {
        bail!("contact before query endpoint");
    }
    Ok(query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::{SimulationStateSnapshotV1, TerrainDefinition, Vec2};

    fn flat_source(
        surface_y_m: f64,
    ) -> (
        WaypointDirectNominalDirectGenerationRequest,
        RunContext,
        SimulationState,
    ) {
        let mut request = crate::test_inputs::planner_request("v2_clear_685");
        let TerrainDefinition::Heightfield { points_m } = &mut request.scenario.world.terrain;
        for point in points_m {
            point.y = surface_y_m;
        }
        for pad in &mut request.scenario.world.landing_pads {
            pad.surface_y_m = surface_y_m;
        }
        request.scenario.initial_state.position_m.y =
            surface_y_m + request.scenario.vehicle.geometry.touchdown_base_offset_m;
        assert!(
            crate::preflight_nominal_direct_flight(
                &request,
                &crate::BodyAwareTerminalPolicyV1::default()
            )
            .supported
        );
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let state = SimulationState::new(&context).unwrap();
        (request, context, state)
    }

    fn initial_segment(state: &SimulationState) -> WaypointV2Segment {
        WaypointV2Segment {
            kind: WaypointV2SegmentKind::InitialNominal,
            start_physics_step: 0,
            end_physics_step: 2,
            proposal_identity: "source-geometry-regression".to_owned(),
            updates: vec![FlightProgramUpdateV1 {
                physics_step: 0,
                phase: "upright".to_owned(),
                command: Command::idle(),
            }],
            entry_state: SimulationStateSnapshotV1::from_state(state),
            end_state: SimulationStateSnapshotV1::from_state(state),
        }
    }

    #[test]
    fn source_rest_roundoff_passes_initial_and_first_command_guards() {
        for height in [
            0.0,
            23.206565037797983,
            -23.206565037797983,
            0.1,
            -0.1,
            27.000000000000004,
            -37.0,
        ] {
            let (request, context, state) = flat_source(height);
            let segments = [initial_segment(&state)];
            let mut guard = FlightGuard {
                request: &request,
                segments: &segments,
                diagnostic: false,
            };
            assert!(guard.initial(&context, &state).is_ok(), "height {height}");
            assert!(
                guard
                    .before_transition(&context, &state, Command::idle())
                    .is_ok(),
                "height {height}"
            );
        }
    }

    #[test]
    fn source_rest_guard_keeps_real_penetration_tilt_domain_and_nonfinite_rejected() {
        let (request, context, source) = flat_source(0.0);
        let segments = [initial_segment(&source)];
        let mut guard = FlightGuard {
            request: &request,
            segments: &segments,
            diagnostic: false,
        };
        let mut state = source.clone();
        state.position_m.y -= 1.0e-6;
        assert!(guard.initial(&context, &state).is_err());
        state = source.clone();
        state.attitude_rad = 0.2;
        assert!(guard.initial(&context, &state).is_err());
        state = source.clone();
        state.position_m.x = context.world.terrain.points().last().unwrap().x;
        assert!(guard.initial(&context, &state).is_err());
        state = source;
        state.position_m = Vec2::new(f64::NAN, state.position_m.y);
        assert!(guard.initial(&context, &state).is_err());
    }

    #[test]
    fn source_rest_allowance_does_not_leak_to_poststep_or_positive_reserve() {
        let (request, context, source) = flat_source(0.1);
        let segments = [initial_segment(&source)];
        let mut guard = FlightGuard {
            request: &request,
            segments: &segments,
            diagnostic: false,
        };
        let mut state = source.clone();
        state.physics_step = 2;
        state.sim_time_s = context.sim.physics_dt_s() * 2.0;
        assert!(guard.after_transition(&context, &state, None).is_err());

        // Even sub-tolerance discrepancies must not use the original-state
        // allowance when the supplied state no longer matches that source.
        state = source.clone();
        state.position_m.y -= 1.0e-10;
        assert!(guard.initial(&context, &state).is_err());
        state = source.clone();
        state.velocity_mps.y = 1.0e-12;
        assert!(guard.initial(&context, &state).is_err());
        state = source.clone();
        state.attitude_rad = 1.0e-12;
        assert!(guard.initial(&context, &state).is_err());
        state = source.clone();
        state.angular_rate_radps = 1.0e-12;
        assert!(guard.initial(&context, &state).is_err());

        state = source;
        let pad = context.world.landing_pad(&request.source_pad_id).unwrap();
        state.position_m.x += pad.width_m;
        state.position_m.y += request.policy.analytical_policy.minimum_clearance_m - 1.0e-10;
        assert!(guard.initial(&context, &state).is_err());
    }

    #[test]
    fn source_rest_guard_matches_admission_tolerance_without_widening_it() {
        let (mut request, _, _) = flat_source(0.0);
        request.scenario.initial_state.position_m.y -= PAD_REST_TOLERANCE_M * 0.5;
        let policy = crate::BodyAwareTerminalPolicyV1::default();
        assert!(crate::preflight_nominal_direct_flight(&request, &policy).supported);
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let state = SimulationState::new(&context).unwrap();
        let segments = [initial_segment(&state)];
        let mut guard = FlightGuard {
            request: &request,
            segments: &segments,
            diagnostic: false,
        };
        assert!(guard.initial(&context, &state).is_ok());

        request.scenario.initial_state.position_m.y -= PAD_REST_TOLERANCE_M * 2.0;
        assert!(!crate::preflight_nominal_direct_flight(&request, &policy).supported);
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let state = SimulationState::new(&context).unwrap();
        let segments = [initial_segment(&state)];
        let mut guard = FlightGuard {
            request: &request,
            segments: &segments,
            diagnostic: false,
        };
        assert!(guard.initial(&context, &state).is_err());
    }
}
