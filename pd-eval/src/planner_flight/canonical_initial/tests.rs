use super::*;
use pd_core::Command;

fn synthetic_binding() -> (RunContext, PadInputV2, CanonicalInitialProposalV1) {
    let request = crate::test_inputs::planner_request("v2_clear_845");
    let context = RunContext::from_scenario(&request.scenario).unwrap();
    let pad = request
        .scenario
        .world
        .landing_pad(&request.source_pad_id)
        .unwrap();
    let source = PadInputV2 {
        center_x_m: pad.center_x_m,
        surface_y_m: pad.surface_y_m,
        width_m: pad.width_m,
    };
    let initial = AirborneFlightStateV1::from_live(&SimulationState::new(&context).unwrap());
    let mut proposal = CanonicalInitialProposalV1 {
        policy_id: CANONICAL_INITIAL_DIRECT_POLICY_ID.into(),
        dynamics_identity: dynamics_identity(&context, &source).unwrap(),
        absolute_deadline_physics_step: 9600,
        initial_state: initial.clone(),
        source_handoff_physics_step: 74,
        source_handoff_state: initial.clone(),
        coast_tick_count: 2,
        terminal_tick_count: 2,
        planned_end_physics_step: 80,
        updates: (0..40)
            .map(|i| FlightProgramUpdateV1 {
                physics_step: i * 2,
                phase: if i < 30 {
                    "upright"
                } else if i < 36 {
                    "tilt"
                } else if i < 37 {
                    "source_bridge"
                } else if i < 38 {
                    "ballistic_coast"
                } else {
                    "terminal_bridge"
                }
                .into(),
                command: Command {
                    throttle_frac: 1.0,
                    target_attitude_rad: 0.0,
                },
            })
            .collect(),
        end_state: initial,
        peak_com_height_m: 5.0,
        identity: String::new(),
    };
    proposal.identity = proposal_identity(&proposal).unwrap();
    (context, source, proposal)
}

#[test]
fn canonical_audit_rejects_rehashed_missing_command_before_execution() {
    let (context, source, mut proposal) = synthetic_binding();
    proposal.updates.remove(3);
    proposal.identity = proposal_identity(&proposal).unwrap();
    assert!(audit_canonical_initial_direct(&context, &source, &proposal, 5.0).is_err());
}

#[test]
fn canonical_audit_rejects_rehashed_phase_and_extended_deadline() {
    let (context, source, mut proposal) = synthetic_binding();
    proposal.updates[38].phase = "source_bridge".into();
    proposal.identity = proposal_identity(&proposal).unwrap();
    assert!(audit_canonical_initial_direct(&context, &source, &proposal, 5.0).is_err());
    let (context, source, mut proposal) = synthetic_binding();
    proposal.absolute_deadline_physics_step = 10_800;
    proposal.identity = proposal_identity(&proposal).unwrap();
    assert!(audit_canonical_initial_direct(&context, &source, &proposal, 5.0).is_err());
}

#[test]
fn canonical_proposal_rejects_unknown_fields() {
    let (_, _, proposal) = synthetic_binding();
    let mut json = serde_json::to_value(proposal).unwrap();
    json["terrain_adaptive"] = true.into();
    assert!(serde_json::from_value::<CanonicalInitialProposalV1>(json).is_err());
}
#[test]
fn canonical_short_budget_keeps_finite_ledger_without_terrain_seed() {
    let mut request = crate::test_inputs::planner_request("v2_clear_845");
    request.scenario.sim.max_time_s = 1.0;
    let search = evaluate_canonical_initial_direct(&request).unwrap();
    assert!(search.selected.is_none());
    assert!(search.preflight_rejection.is_some() || search.attempts.len() == 140);
}
