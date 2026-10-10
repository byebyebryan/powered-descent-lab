//! Explicit counterfactual diagnostics. Never called by the flight session.
//! Saved snapshots are comparison evidence, never plant restart inputs.

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, ensure};
use pd_core::{RunContext, SimulationStateSnapshotV1};
use pd_plan::local_clearing::LocalClearingPolicyV1;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::execution::{append_segment, consumed, prove_accumulated, query_to};
use super::{WaypointV2FlightResult, WaypointV2Segment, WaypointV2SegmentKind};
use crate::{
    LocalClearingProposalV1, WaypointDirectNominalDirectGenerationPolicyV1,
    WaypointDirectNominalDirectGenerationRequest, audit_airborne_acquisition_proposal,
    audit_canonical_initial_direct, evaluate_airborne_acquisition_direct,
    evaluate_canonical_initial_direct,
    evidence_io::{reserve_output_root, sha256_bytes, write_json_create_only},
    local_clearing::{OrdinaryLive, advance_ordinary, new_ordinary, search_row},
    planner_flight::terrain::{has_actual_terrain_conflict, source_pad_input},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeArm {
    Original,
    Early,
    Alternative,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeSpec {
    pub case_id: String,
    pub arm: ProbeArm,
    pub scenario_sha256: String,
    pub flight_sha256: String,
    pub cycle_index: usize,
    pub row_id: Option<String>,
    pub physics_step: u64,
}

fn validate_spec(spec: &ProbeSpec) -> Result<()> {
    ensure!(
        matches!(
            spec.case_id.as_str(),
            "random-983"
                | "random-967"
                | "random-955"
                | "random-024"
                | "random-516"
                | "random-271"
                | "random-000"
        ),
        "unknown handoff diagnostic case"
    );
    for hash in [&spec.scenario_sha256, &spec.flight_sha256] {
        ensure!(
            hash.len() == 64 && hash.bytes().all(|v| v.is_ascii_hexdigit()),
            "invalid diagnostic input digest"
        );
    }
    ensure!(
        spec.physics_step.is_multiple_of(2),
        "unaligned diagnostic clock"
    );
    ensure!(
        spec.case_id != "random-000"
            || (spec.arm == ProbeArm::Original
                && spec.cycle_index == 0
                && spec.row_id.is_none()
                && spec.physics_step == 0),
        "direct control is not an airborne restart"
    );
    ensure!(
        spec.case_id != "random-271" || spec.arm == ProbeArm::Original,
        "airborne control must retain its original handoff"
    );
    Ok(())
}

/// Recompute the baseline's completed pieces from the original scenario source.
fn cycle_origin(
    context: &RunContext,
    baseline: &WaypointV2FlightResult,
    index: usize,
) -> Result<(OrdinaryLive, Vec<WaypointV2Segment>)> {
    let expected = &baseline
        .cycles
        .get(index)
        .context("missing probe cycle")?
        .current_state;
    let mut live = new_ordinary(context)?;
    let mut segments = Vec::new();
    for segment in baseline
        .segments
        .iter()
        .filter(|s| s.end_physics_step <= expected.physics_step)
    {
        ensure!(
            segment.entry_state == live.evidence.final_state,
            "baseline source-piece discontinuity"
        );
        advance_ordinary(
            context,
            &mut live,
            &segment.updates,
            segment.end_physics_step,
        )?;
        ensure!(
            segment.end_state == live.evidence.final_state,
            "baseline source-piece replay differs"
        );
        segments.push(segment.clone());
    }
    ensure!(
        live.evidence.final_state == *expected,
        "baseline cycle origin replay differs"
    );
    Ok((live, segments))
}

fn early_boundary(proposal: &LocalClearingProposalV1) -> Result<&SimulationStateSnapshotV1> {
    let policy = LocalClearingPolicyV1::default();
    let state = proposal
        .trajectory
        .iter()
        .map(|v| &v.state)
        .find(|s| {
            s.physics_step > proposal.schedule.powered_end_physics_step
                && s.physics_step.is_multiple_of(2)
                && s.held_command.throttle_frac == 0.0
                && s.attitude_rad == 0.0
                && s.angular_rate_radps == 0.0
        })
        .context("selected maneuver has no settled coasting boundary")?;
    let end = state
        .physics_step
        .checked_add(policy.continuation_ticks)
        .context("early guard overflow")?;
    let guard = proposal
        .trajectory
        .iter()
        .filter(|v| (state.physics_step..=end).contains(&v.state.physics_step));
    let mut count = 0_u64;
    for sample in guard {
        ensure!(
            sample.body_clearance_m >= policy.minimum_clearance_m,
            "early query lacks its unchanged short-coast reserve"
        );
        count += 1;
    }
    ensure!(
        count == policy.continuation_ticks + 1,
        "incomplete early short-coast trace"
    );
    Ok(state)
}

fn best_room_row(search: &super::WaypointV2LocalSearch) -> Result<&super::WaypointV2RowDiagnostic> {
    search
        .row_diagnostics
        .iter()
        .filter(|r| {
            r.eligible_handoff
                .as_ref()
                .and_then(|h| h.braking_room.as_ref())
                .is_some_and(|room| room.remaining_room_m.is_finite())
        })
        .min_by(|a, b| {
            let room = |r: &super::WaypointV2RowDiagnostic| {
                r.eligible_handoff
                    .as_ref()
                    .unwrap()
                    .braking_room
                    .as_ref()
                    .unwrap()
                    .remaining_room_m
            };
            room(b).total_cmp(&room(a)).then(a.row_id.cmp(&b.row_id))
        })
        .context("no accepted braking-room alternative")
}

fn guard_early_live(
    context: &RunContext,
    live: &OrdinaryLive,
    proposal: &LocalClearingProposalV1,
) -> Result<()> {
    let mut state = live.state.clone();
    let end = state.physics_step + proposal.policy.continuation_ticks;
    for tick in (state.physics_step..=end).step_by(2) {
        ensure!(
            crate::live_rejection(
                context,
                &state,
                proposal.goal.absolute_deadline_physics_step
            )
            .is_none(),
            "early continuation leaves unchanged supported family at {tick}"
        );
        if tick == end {
            break;
        }
        let update = proposal
            .schedule
            .updates
            .iter()
            .find(|u| u.physics_step == tick)
            .context("early continuation command gap")?;
        ensure!(
            update.command.throttle_frac == 0.0,
            "early continuation is not coasting"
        );
        state.set_command(update.command);
        for _ in 0..2 {
            let transition = state.step_with_contact_report(context);
            ensure!(
                transition.incoming_contact.is_none(),
                "early continuation contacts terrain"
            );
            let expected = proposal
                .trajectory
                .iter()
                .find(|v| v.state.physics_step == state.physics_step)
                .context("early continuation trace gap")?;
            ensure!(
                SimulationStateSnapshotV1::from_state(&state) == expected.state,
                "early continuation forward state differs"
            );
        }
    }
    Ok(())
}

fn reconstruct_probe(
    request: &WaypointDirectNominalDirectGenerationRequest,
    context: &RunContext,
    baseline: &WaypointV2FlightResult,
    spec: &ProbeSpec,
) -> Result<(
    OrdinaryLive,
    Vec<WaypointV2Segment>,
    Option<LocalClearingProposalV1>,
)> {
    let (mut live, mut segments) = cycle_origin(context, baseline, spec.cycle_index)?;
    if spec.case_id == "random-000" {
        return Ok((live, segments, None));
    }
    let cycle = &baseline.cycles[spec.cycle_index];
    let search = cycle
        .local_search
        .as_ref()
        .context("missing baseline local search")?;
    let selected = search
        .selected
        .as_ref()
        .context("missing baseline selected maneuver")?;
    let expected = if spec.arm == ProbeArm::Alternative {
        best_room_row(search)?
    } else {
        search
            .row_diagnostics
            .iter()
            .find(|r| r.row_id == selected.row_id)
            .context("missing selected row diagnostic")?
    };
    ensure!(
        spec.row_id.as_deref() == Some(&expected.row_id),
        "preselected row differs"
    );
    let (entry_id, row_index) = expected
        .row_id
        .rsplit_once("_row_")
        .context("invalid row identity")?;
    let template = LocalClearingPolicyV1::default()
        .templates()
        .map_err(anyhow::Error::msg)?
        .get(row_index.parse::<usize>()?)
        .context("unknown clearing template")?
        .clone();
    let origin = live.evidence.final_state.clone();
    live = query_to(
        context,
        &live,
        &cycle.nominal_updates,
        expected.entry_physics_step,
    )?;
    append_segment(
        &mut segments,
        if spec.cycle_index == 0 {
            WaypointV2SegmentKind::InitialNominal
        } else {
            WaypointV2SegmentKind::AirborneNominal
        },
        cycle
            .nominal_proposal_identity
            .as_deref()
            .context("missing fixed nominal identity")?,
        &origin,
        &live,
        consumed(&cycle.nominal_updates, expected.entry_physics_step),
    )?;
    let (_, proposal) = search_row(
        request,
        context,
        entry_id,
        Some(&live.state),
        (expected.entry_physics_step, None),
        &template,
        &selected.goal,
    )?;
    let proposal = proposal.context("previously accepted row no longer has a handoff")?;
    let saved = expected
        .eligible_handoff
        .as_ref()
        .context("missing accepted query state")?;
    ensure!(
        proposal.handoff_state == saved.state
            && proposal.actual_fuel_burn_to_handoff_kg == saved.actual_fuel_burn_to_handoff_kg,
        "recomputed row differs from recorded accepted state"
    );
    if spec.arm != ProbeArm::Alternative {
        ensure!(
            proposal == *selected,
            "same-maneuver replay differs from saved proposal"
        );
    }
    let expected_state = if spec.arm == ProbeArm::Early {
        early_boundary(&proposal)?
    } else {
        &proposal.handoff_state
    };
    ensure!(
        expected_state.physics_step == spec.physics_step,
        "preselected probe clock differs"
    );
    let entry = live.evidence.final_state.clone();
    live = query_to(
        context,
        &live,
        &proposal.schedule.updates,
        spec.physics_step,
    )?;
    ensure!(
        live.evidence.final_state == *expected_state,
        "forward probe state differs from query trace"
    );
    if spec.arm == ProbeArm::Early {
        guard_early_live(context, &live, &proposal)?;
    }
    append_segment(
        &mut segments,
        WaypointV2SegmentKind::LocalCorrection,
        &proposal.identity,
        &entry,
        &live,
        consumed(&proposal.schedule.updates, spec.physics_step),
    )?;
    Ok((live, segments, Some(proposal)))
}

fn nominal_counts(
    search: &crate::AirborneAcquisitionSearchV1,
) -> (BTreeMap<String, usize>, BTreeMap<String, usize>) {
    let mut statuses = BTreeMap::new();
    let mut reasons = BTreeMap::new();
    for seed in &search.seeds {
        if seed.selected_entry_index.is_none()
            && let Some(reason) = &seed.reason
        {
            *reasons.entry(reason.clone()).or_default() += 1;
        }
    }
    for attempt in &search.attempts {
        *statuses.entry(attempt.status.clone()).or_default() += 1;
        if let Some(reason) = &attempt.reason {
            *reasons.entry(reason.clone()).or_default() += 1;
        }
    }
    (statuses, reasons)
}

/// One explicit diagnostic probe. It cannot advance or mutate a V2 session.
pub fn run(
    scenario_path: &Path,
    flight_path: &Path,
    spec_path: &Path,
    output: &Path,
) -> Result<Value> {
    let spec: ProbeSpec = serde_json::from_slice(&fs::read(spec_path)?)?;
    validate_spec(&spec)?;
    let scenario_bytes = fs::read(scenario_path)?;
    let flight_bytes = fs::read(flight_path)?;
    ensure!(
        sha256_bytes(&scenario_bytes)? == spec.scenario_sha256
            && sha256_bytes(&flight_bytes)? == spec.flight_sha256,
        "changed diagnostic source artifacts"
    );
    let request = WaypointDirectNominalDirectGenerationRequest {
        scenario: serde_json::from_slice(&scenario_bytes)?,
        source_pad_id: "pad_source".into(),
        target_pad_id: "pad_main".into(),
        probe_id: String::new(),
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    };
    let request = WaypointDirectNominalDirectGenerationRequest {
        probe_id: request.scenario.id.clone(),
        ..request
    };
    crate::validate_waypoint_direct_nominal_direct_generation_request(&request)?;
    let baseline: WaypointV2FlightResult = serde_json::from_slice(&flight_bytes)?;
    ensure!(
        baseline.integrity_passed
            && baseline.final_source_replay_passed
            && baseline.policy.policy_id == "piecewise_local_clearing_v2_policy_3_cap_probe_24"
            && baseline.policy.maximum_corrections == 24,
        "unverified or unexpected saved baseline"
    );
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let deadline = pd_plan::waypoint_v2::original_deadline(
        context.sim.max_time_s,
        request.policy.analytical_policy.mission_budget_s(),
    )
    .map_err(anyhow::Error::msg)?;
    ensure!(
        baseline.absolute_deadline_physics_step == Some(deadline),
        "original deadline differs"
    );
    reserve_output_root(output)?;
    write_json_create_only(&output.join("request.json"), &spec)?;
    let (live, segments, local) = reconstruct_probe(&request, &context, &baseline, &spec)?;
    let proof = prove_accumulated(&request, &context, &live, &segments, false, deadline)?;
    write_json_create_only(&output.join("prefix-proof.json"), &proof)?;
    write_json_create_only(&output.join("prefix-segments.json"), &segments)?;
    if let Some(proposal) = &local {
        write_json_create_only(&output.join("clearing-query.json"), proposal)?;
    }
    let (search_identity, selected_identity, statuses, reasons, audit) =
        if spec.case_id == "random-000" {
            let search = evaluate_canonical_initial_direct(&request)?;
            write_json_create_only(&output.join("nominal-search.json"), &search)?;
            let proposal = search
                .selected
                .as_ref()
                .context("direct control lost its nominal")?;
            let audit = audit_canonical_initial_direct(
                &context,
                &source_pad_input(&context, "pad_source")?,
                proposal,
                5.0,
            )?;
            ensure!(
                baseline.cycles[0].nominal_search_identity == search.identity
                    && baseline.cycles[0].nominal_proposal_identity.as_deref()
                        == Some(&proposal.identity)
                    && baseline.cycles[0].audit.as_ref() == Some(&audit),
                "direct control changed"
            );
            (
                search.identity,
                Some(proposal.identity.clone()),
                BTreeMap::new(),
                BTreeMap::new(),
                Some(audit),
            )
        } else {
            let search = evaluate_airborne_acquisition_direct(&context, &live.state, deadline)?;
            ensure!(
                search.unsupported_reason.is_none(),
                "probe state is outside unchanged incoming family"
            );
            write_json_create_only(&output.join("nominal-search.json"), &search)?;
            let (statuses, reasons) = nominal_counts(&search);
            let audit = search
                .selected
                .as_ref()
                .map(|p| audit_airborne_acquisition_proposal(&context, &live.state, p, 5.0))
                .transpose()?;
            if spec.arm == ProbeArm::Original {
                let next = baseline
                    .cycles
                    .get(spec.cycle_index + 1)
                    .context("missing original resumed cycle")?;
                ensure!(
                    next.current_state == live.evidence.final_state
                        && next.nominal_search_identity == search.identity
                        && next.nominal_attempt_status_counts == statuses
                        && next.nominal_rejection_reason_counts == reasons
                        && next.nominal_proposal_identity
                            == search.selected.as_ref().map(|p| p.identity.clone())
                        && next.audit == audit,
                    "original resumed planner differs from baseline"
                );
            }
            (
                search.identity,
                search.selected.as_ref().map(|p| p.identity.clone()),
                statuses,
                reasons,
                audit,
            )
        };
    let disposition = if let Some(audit) = &audit {
        ensure!(
            audit.ordinary_neutral_parity
                && (audit.commands_match || has_actual_terrain_conflict(audit)),
            "nominal audit integrity differs"
        );
        write_json_create_only(&output.join("terrain-audit.json"), audit)?;
        if audit.passed {
            "nominal_clear"
        } else if has_actual_terrain_conflict(audit) {
            "nominal_terrain_blocked"
        } else {
            "nominal_other_rejected"
        }
    } else {
        "no_nominal"
    };
    let result = json!({
        "schema": "pd-lab.terrain-handoff-probe.v1", "request": spec,
        "evidence_kind": "counterfactual_query", "executed_mission_landing_claimed": false,
        "probe_state": live.evidence.final_state, "absolute_deadline_physics_step": deadline,
        "prefix_source_replay_passed": true, "original_result_matched": spec.arm == ProbeArm::Original,
        "production_boundary_admitted": spec.arm != ProbeArm::Early,
        "clearing_witness_physics_step": local.as_ref().map(|p| p.handoff_state.physics_step),
        "disposition": disposition, "nominal_search_identity": search_identity,
        "nominal_proposal_identity": selected_identity, "nominal_attempt_status_counts": statuses,
        "nominal_rejection_reason_counts": reasons,
        "audit_passed": audit.as_ref().map(|a| a.passed),
        "first_terrain_violation": audit.as_ref().and_then(|a| a.clearance_scan.first_violation.as_ref()),
    });
    write_json_create_only(&output.join("probe.json"), &result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> ProbeSpec {
        ProbeSpec {
            case_id: "random-983".into(),
            arm: ProbeArm::Early,
            scenario_sha256: "a".repeat(64),
            flight_sha256: "b".repeat(64),
            cycle_index: 0,
            row_id: Some("source_row_25".into()),
            physics_step: 2428,
        }
    }

    #[test]
    fn probe_admission_is_bounded_and_direct_control_is_not_an_airborne_restart() {
        validate_spec(&spec()).unwrap();
        let mut changed = spec();
        changed.case_id = "random-999".into();
        assert!(validate_spec(&changed).is_err());
        changed = spec();
        changed.physics_step += 1;
        assert!(validate_spec(&changed).is_err());
        changed = spec();
        changed.flight_sha256 = "invalid".into();
        assert!(validate_spec(&changed).is_err());
        changed = spec();
        changed.case_id = "random-000".into();
        assert!(validate_spec(&changed).is_err());
        changed.arm = ProbeArm::Original;
        changed.row_id = None;
        changed.physics_step = 0;
        validate_spec(&changed).unwrap();
    }

    #[test]
    fn empty_search_has_no_invented_attempts_or_reasons() {
        let search: crate::AirborneAcquisitionSearchV1 = serde_json::from_value(json!({
            "policy_id": "synthetic", "absolute_deadline_physics_step": 9600,
            "incoming_state": {"physics_step": 2, "sim_time_s": 2.0 / 120.0,
                "position_m": {"x": 1.0, "y": 100.0}, "velocity_mps": {"x": 1.0, "y": 0.0},
                "attitude_rad": 0.0, "angular_rate_radps": 0.0, "fuel_kg": 6300.0,
                "held_command": {"throttle_frac": 0.0, "target_attitude_rad": 0.0}},
            "seeds": [], "attempts": [], "selected": null, "unsupported_reason": null,
            "identity": "synthetic"
        }))
        .unwrap();
        assert_eq!(nominal_counts(&search), (BTreeMap::new(), BTreeMap::new()));
    }

    #[test]
    fn original_rest_prefix_uses_the_ordinary_source_and_empty_command_coverage() {
        let request = crate::test_inputs::planner_request("v2_clear_685");
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let live = new_ordinary(&context).unwrap();
        let proof = prove_accumulated(&request, &context, &live, &[], false, 9600).unwrap();
        assert!(proof.failure.is_none());
        assert!(proof.run.actions.is_empty());
        assert_eq!(proof.final_state, live.evidence.final_state);
        assert_eq!(proof.final_state.physics_step, 0);
    }

    #[test]
    fn alternative_is_maximum_accepted_room_with_fixed_row_identity_tie_break() {
        let request = crate::test_inputs::planner_request("v2_clear_685");
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let state = new_ordinary(&context).unwrap().evidence.final_state;
        let mut search: super::super::WaypointV2LocalSearch = serde_json::from_value(json!({
            "entries": [], "row_count": 0, "boundary_count": 0, "accepted_row_count": 0,
            "row_status_counts": {}, "row_stop_reason_counts": {}, "boundary_status_counts": {},
            "selected": null, "certificate_state": null, "handoff_source_replay_passed": false,
            "certificate_source_replay_passed": false
        }))
        .unwrap();
        for (id, room) in [("z", 20.0), ("a", 20.0), ("b", -10.0)] {
            search.row_diagnostics.push(
                serde_json::from_value(json!({
                    "row_id": id, "entry_physics_step": 0, "physically_propagated": true,
                    "stop_reason": null, "stop_state": null, "minimum_clearance_m": 5.0,
                    "minimum_progress_x_m": 0.0, "boundary_status_counts": {},
                    "first_continuation_rejection": null,
                    "eligible_handoff": {"state": state, "actual_fuel_burn_to_handoff_kg": 0.0,
                        "braking_room": {"available_acceleration_mps2": 10.0,
                            "horizontal_braking_acceleration_mps2": 5.0, "turn_time_s": 1.0,
                            "required_distance_m": 10.0, "remaining_room_m": room}}
                }))
                .unwrap(),
            );
        }
        assert_eq!(best_room_row(&search).unwrap().row_id, "a");
        search.row_diagnostics[1].eligible_handoff = None;
        assert_eq!(best_room_row(&search).unwrap().row_id, "z");
        search.row_diagnostics[0]
            .eligible_handoff
            .as_mut()
            .unwrap()
            .braking_room
            .as_mut()
            .unwrap()
            .remaining_room_m = f64::NAN;
        assert_eq!(best_room_row(&search).unwrap().row_id, "b");
    }
}
