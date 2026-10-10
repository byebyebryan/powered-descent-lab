//! Opt-in diagnostic reenactment, never a planner or snapshot restoration path.
//! All cycle origins are cloned from the original source-command execution.
use super::*;
use crate::waypoint_v2::{WaypointV2Cycle, WaypointV2RowDiagnostic};
use pd_core::{FlightProgramUpdateV1, RunContext, SimulationState, SimulationStateSnapshotV1};
use pd_plan::local_clearing::{LocalClearingGoalV1, LocalClearingPolicyV1};
use pd_report::{
    flight_annotations::{ExecutedCorrection, FlightBoundary},
    planning_cycles::{CycleReview, PathPoint, PlanningReview, QueryReview},
};

/// Predeclared examples: departure, approach, post-H clearing and nominal stops,
/// plus direct and multi-H successes. Other pages still get every nominal cycle.
pub(super) const QUERY_CASES: &[&str] = &[
    "random-715",
    "random-280",
    "random-983",
    "random-327",
    "random-999",
    "random-000",
    "random-030",
    "random-565",
];

/// Bind the compiled plant and fixed-row implementation, not just files read
/// after compilation. This is source-matched reconstruction, not the old binary.
pub(super) fn engine_binding(authenticated: &impl Fn(&str) -> Result<Vec<u8>>) -> Result<Value> {
    macro_rules! embedded {
        ($($path:literal),+ $(,)?) => { [$(($path, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../", $path)).as_slice())),+] };
    }
    let files = embedded!(
        "Cargo.toml",
        "Cargo.lock",
        "pd-core/Cargo.toml",
        "pd-core/src/lib.rs",
        "pd-core/src/math.rs",
        "pd-core/src/terrain.rs",
        "pd-core/src/model.rs",
        "pd-core/src/sim.rs",
        "pd-core/src/eval.rs",
        "pd-core/src/bounded_run.rs",
        "pd-core/src/flight_program.rs",
        "pd-core/src/planning.rs",
        "pd-plan/src/local_clearing.rs",
        "pd-eval/src/local_clearing.rs",
        "pd-eval/src/planner_flight.rs",
        "pd-eval/src/planner_flight/input.rs",
        "pd-eval/src/planner_flight/airborne.rs",
        "pd-eval/src/planner_flight/terminal.rs",
        "pd-eval/src/planner_flight/geometry.rs",
        "pd-eval/src/planner_flight/math.rs",
    );
    let seal: Value = serde_json::from_slice(&authenticated("candidate/source-seal.json")?)?;
    let mut bound = BTreeMap::new();
    for (path, compiled) in files {
        let hash = sha256_bytes(compiled)?;
        ensure!(
            seal["files"][path] == hash,
            "compiled diagnostic engine differs: {path}"
        );
        ensure!(
            authenticated(&format!("candidate/source/{path}"))? == compiled,
            "retained engine source differs: {path}"
        );
        bound.insert(path, hash);
    }
    Ok(
        json!({"compiled_source_files": bound, "capture_executable_sha256": seal["executable_sha256"],
        "renderer_executable_sha256": sha256_bytes(&fs::read(std::env::current_exe()?)?)?,
        "kind": "source-matched fixed-program diagnostic reenactment; not a new mission or acceptance proof"}),
    )
}

fn point(state: &SimulationState) -> PathPoint {
    PathPoint {
        time_s: state.sim_time_s,
        x_m: state.position_m.x,
        y_m: state.position_m.y,
    }
}

fn boundary(s: &SimulationStateSnapshotV1) -> FlightBoundary {
    FlightBoundary {
        physics_step: s.physics_step,
        sim_time_s: s.sim_time_s,
        position_m: s.position_m,
        velocity_mps: s.velocity_mps,
        attitude_rad: s.attitude_rad,
        fuel_kg: s.fuel_kg,
    }
}

fn live_boundary(s: &SimulationState) -> FlightBoundary {
    boundary(&SimulationStateSnapshotV1::from_state(s))
}

fn command_at(
    state: &mut SimulationState,
    updates: &[FlightProgramUpdateV1],
    index: &mut usize,
) -> Result<()> {
    if state.physics_step.is_multiple_of(2) {
        let update = updates
            .get(*index)
            .context("diagnostic command coverage exhausted")?;
        ensure!(
            update.physics_step == state.physics_step && update.command == update.command.clamped(),
            "diagnostic update missing, reordered or invalid"
        );
        state.set_command(update.command);
        *index += 1;
    }
    Ok(())
}

fn prefix(
    context: &RunContext,
    origin: &SimulationState,
    updates: &[FlightProgramUpdateV1],
    tick: u64,
) -> Result<SimulationState> {
    ensure!(
        tick >= origin.physics_step,
        "diagnostic prefix before origin"
    );
    let mut state = origin.clone();
    let mut index = 0;
    while state.physics_step < tick {
        ensure!(
            !state.is_terminal(),
            "diagnostic prefix terminated before query entry"
        );
        command_at(&mut state, updates, &mut index)?;
        state.step_with_contact_report(context);
    }
    Ok(state)
}

fn nominal(
    context: &RunContext,
    origin: &SimulationState,
    cycle: &WaypointV2Cycle,
) -> Result<(Vec<PathPoint>, Vec<PathPoint>)> {
    if cycle.nominal_updates.is_empty() {
        ensure!(cycle.audit.is_none(), "audit without nominal commands");
        return Ok((vec![], vec![]));
    }
    let audit = cycle
        .audit
        .as_ref()
        .context("recorded nominal has no audit")?;
    let end = cycle.nominal_updates.last().unwrap().physics_step + 2;
    ensure!(
        end >= audit.final_state.physics_step && end <= (context.sim.max_time_s * 120.0) as u64,
        "nominal diagnostic endpoint outside recorded program/horizon"
    );
    let mut neutral = origin.clone();
    let mut ordinary = origin.clone();
    let mut index = 0;
    let mut audited = vec![point(origin)];
    let mut extension = vec![];
    let mut conflict_matched = cycle.conflict_state.is_none();
    let mut audit_matched = false;
    while neutral.physics_step < end {
        command_at(&mut neutral, &cycle.nominal_updates, &mut index)?;
        if ordinary.physics_step < audit.final_state.physics_step {
            ordinary.set_command(neutral.held_command);
            ordinary.step_with_contact_report(context);
        }
        neutral.step_physics_and_classify_contact(context);
        if let Some(conflict) = &cycle.conflict_state
            && neutral.physics_step == conflict.physics_step
        {
            ensure!(
                SimulationStateSnapshotV1::from_state(&neutral) == *conflict,
                "reconstructed nominal conflict differs at cycle {}",
                cycle.cycle_index
            );
            conflict_matched = true;
        }
        if neutral.physics_step == audit.final_state.physics_step {
            ensure!(
                SimulationStateSnapshotV1::from_state(&ordinary) == audit.final_state,
                "reconstructed nominal audit endpoint differs at cycle {}",
                cycle.cycle_index
            );
            if let Some(contact) = &audit.incoming_contact {
                ensure!(
                    SimulationStateSnapshotV1::from_state(&neutral) == contact.state,
                    "reconstructed incoming contact differs"
                );
            }
            audit_matched = true;
            extension.push(point(&neutral));
        }
        if neutral.physics_step.is_multiple_of(12)
            || neutral.physics_step == audit.final_state.physics_step
            || neutral.physics_step == end
            || cycle
                .conflict_state
                .as_ref()
                .is_some_and(|c| c.physics_step == neutral.physics_step)
        {
            if neutral.physics_step <= audit.final_state.physics_step {
                audited.push(point(&neutral));
            } else {
                extension.push(point(&neutral));
            }
        }
    }
    ensure!(
        index == cycle.nominal_updates.len() && conflict_matched && audit_matched,
        "diagnostic program/checkpoint coverage incomplete"
    );
    if extension.len() <= 1 {
        extension.clear();
    }
    Ok((audited, extension))
}

fn ballistic(context: &RunContext, state: &SimulationState) -> Vec<PathPoint> {
    let target_time = if state.velocity_mps.x > 0.0 {
        (context.target_pad.center_x_m - state.position_m.x) / state.velocity_mps.x
    } else {
        20.0
    };
    let duration = target_time
        .clamp(2.0, 60.0)
        .min((context.sim.max_time_s - state.sim_time_s).max(0.0));
    (0..=120)
        .map(|i| {
            let t = duration * i as f64 / 120.0;
            PathPoint {
                time_s: state.sim_time_s + t,
                x_m: state.position_m.x + state.velocity_mps.x * t,
                y_m: state.position_m.y + state.velocity_mps.y * t
                    - 0.5 * context.world.gravity_mps2 * t * t,
            }
        })
        .take_while(|p| p.y_m >= context.target_pad.surface_y_m.min(state.position_m.y) - 100.0)
        .collect()
}

/// Decompose only the recorded compound incoming-family rejection. The original
/// runtime predicate remains authoritative; this does not introduce new guards.
fn incoming_failures(context: &RunContext, s: &SimulationState) -> Vec<String> {
    let mut failures = vec![];
    if s.fuel_kg <= 0.0 || s.fuel_kg > context.vehicle.initial_fuel_kg {
        failures.push("fuel outside supported range".into());
    }
    if s.velocity_mps.x <= 0.0 {
        failures.push("forward velocity vx ≤ 0".into());
    }
    if s.position_m.x >= context.target_pad.center_x_m {
        failures
            .push("x reached/passed the target: forward-only incoming family unsupported".into());
    }
    if s.held_command.throttle_frac != 0.0 {
        failures.push("held throttle is not idle".into());
    }
    if s.held_command != s.held_command.clamped() {
        failures.push("held command is unclamped".into());
    }
    if s.angular_rate_radps.abs() > context.vehicle.max_rotation_rate_radps + 1.0e-12 {
        failures.push("angular rate outside supported range".into());
    }
    failures
}

fn query_review(
    context: &RunContext,
    scenario: &ScenarioSpec,
    origin: &SimulationState,
    cycle: &WaypointV2Cycle,
    diagnostic: &WaypointV2RowDiagnostic,
    deadline: u64,
) -> Result<QueryReview> {
    let (entry_id, number) = diagnostic
        .row_id
        .rsplit_once("_row_")
        .context("recorded row identity")?;
    let templates = LocalClearingPolicyV1::default()
        .templates()
        .map_err(anyhow::Error::msg)?;
    let template = templates
        .get(number.parse::<usize>()?)
        .context("recorded row index")?;
    let conflict = cycle
        .conflict_state
        .as_ref()
        .context("row without conflict")?;
    let goal = LocalClearingGoalV1 {
        first_conflict_physics_step: conflict.physics_step,
        first_conflict_position_m: conflict.position_m,
        absolute_deadline_physics_step: deadline,
    };
    let entry = prefix(
        context,
        origin,
        &cycle.nominal_updates,
        diagnostic.entry_physics_step,
    )?;
    let source_pads = context
        .world
        .landing_pads
        .iter()
        .filter(|p| {
            p.id != context.target_pad.id && p.center_x_m == context.initial_state.position_m.x
        })
        .collect::<Vec<_>>();
    ensure!(source_pads.len() == 1, "ambiguous original source pad");
    let request = WaypointDirectNominalDirectGenerationRequest {
        scenario: scenario.clone(),
        source_pad_id: source_pads[0].id.clone(),
        target_pad_id: context.target_pad.id.clone(),
        probe_id: "diagnostic-fixed-row".into(),
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    };
    let (row, proposal) = crate::local_clearing::search_row(
        &request,
        context,
        entry_id,
        Some(&entry),
        (entry.physics_step, None),
        template,
        &goal,
    )?;
    ensure!(
        proposal.is_none(),
        "recorded rejected row now admits a proposal"
    );
    let counts = row.boundaries.iter().fold(BTreeMap::new(), |mut m, b| {
        *m.entry(b.status.clone()).or_insert(0) += 1;
        m
    });
    let first = row.boundaries.iter().find(|b| {
        matches!(
            b.status.as_str(),
            "unsafe_continuation" | "continuation_unsupported" | "handoff_unsupported"
        )
    });
    ensure!(
        row.row_id == diagnostic.row_id
            && row.stop_state == diagnostic.stop_state
            && row.stop_reason == diagnostic.stop_reason
            && row.minimum_clearance_m == diagnostic.minimum_clearance_m
            && counts == diagnostic.boundary_status_counts
            && first == diagnostic.first_continuation_rejection.as_ref()
            && goal.progress_x(entry.position_m.x, &context.vehicle.geometry)
                == diagnostic.minimum_progress_x_m,
        "fixed-row reenactment differs from saved diagnostic {}",
        diagnostic.row_id
    );
    let handoff_tick = first.map(|b| b.handoff_physics_step);
    let failure_tick = first
        .and_then(|b| b.reason.as_deref())
        .and_then(|r| r.strip_prefix("tick "))
        .and_then(|r| r.split_once(':'))
        .and_then(|(n, _)| n.parse::<u64>().ok())
        .or(handoff_tick);
    let stop = row.stop_state.as_ref().context("fixed-row stop state")?;
    let mut state = entry;
    let mut index = 0;
    let mut path = vec![point(&state)];
    let mut handoff = None;
    let mut rejection_state = None;
    let mut predicates = vec![];
    while state.physics_step < stop.physics_step {
        command_at(&mut state, &row.consumed_updates, &mut index)?;
        state.step_with_contact_report(context);
        if Some(state.physics_step) == handoff_tick {
            handoff = Some(live_boundary(&state));
        }
        if Some(state.physics_step) == failure_tick {
            rejection_state = Some(live_boundary(&state));
            if first
                .and_then(|b| b.reason.as_deref())
                .is_some_and(|r| r.contains("bounded incoming family"))
            {
                predicates = incoming_failures(context, &state);
                ensure!(
                    !predicates.is_empty(),
                    "compound rejection without failing predicate"
                );
            }
        }
        if state.physics_step.is_multiple_of(12)
            || Some(state.physics_step) == failure_tick
            || state.physics_step == stop.physics_step
        {
            path.push(point(&state));
        }
    }
    ensure!(
        SimulationStateSnapshotV1::from_state(&state) == *stop
            && index == row.consumed_updates.len(),
        "fixed query display trace differs from saved stop"
    );
    Ok(QueryReview {
        row_id: row.row_id,
        path,
        progress_x_m: diagnostic.minimum_progress_x_m,
        stop_reason: diagnostic.stop_reason.clone().unwrap_or_default(),
        handoff,
        rejection_state,
        rejection_status: first.map(|b| b.status.clone()),
        rejection_reason: first.and_then(|b| b.reason.clone()),
        failed_predicates: predicates,
    })
}

fn selected_queries(cycle: &WaypointV2Cycle) -> Vec<&WaypointV2RowDiagnostic> {
    let Some(search) = &cycle.local_search else {
        return vec![];
    };
    // One first recorded continuation-unsupported example, plus the farthest
    // progress-limited finite trace. These are explanations, not new candidates.
    let rows = &search.row_diagnostics;
    let first = rows.iter().find(|r| {
        r.physically_propagated
            && r.first_continuation_rejection
                .as_ref()
                .is_some_and(|b| b.status == "continuation_unsupported")
    });
    let second = rows
        .iter()
        .filter(|r| {
            r.physically_propagated
                && r.first_continuation_rejection.is_none()
                && r.stop_state.is_some()
                && r.boundary_status_counts
                    .contains_key("insufficient_progress")
        })
        .max_by(|a, b| {
            a.stop_state
                .as_ref()
                .unwrap()
                .position_m
                .x
                .total_cmp(&b.stop_state.as_ref().unwrap().position_m.x)
        });
    first.into_iter().chain(second).collect()
}

pub(super) fn reconstruct(
    scenario: &ScenarioSpec,
    flight: &WaypointV2FlightResult,
    queries: bool,
) -> Result<PlanningReview> {
    let context = RunContext::from_scenario(scenario).map_err(anyhow::Error::msg)?;
    ensure!(
        context.sim.physics_hz == 120 && context.sim.controller_hz == 60,
        "unsupported diagnostic clock"
    );
    let ordinary = flight
        .ordinary_flight
        .as_ref()
        .context("missing original source execution")?;
    let mut requested = BTreeSet::new();
    for cycle in &flight.cycles {
        requested.insert(cycle.current_state.physics_step);
    }
    for search in flight.cycles.iter().filter_map(|c| c.local_search.as_ref()) {
        if let (Some(p), Some(h)) = (&search.selected, search.actual_handoff_state()) {
            requested.insert(p.entry_state.physics_step);
            requested.insert(h.physics_step);
        }
    }
    let mut state = SimulationState::new(&context)?;
    let mut origins = BTreeMap::new();
    let mut actual = vec![point(&state)];
    let mut action = 0;
    let mut sample = 0;
    loop {
        if let Some(saved) = ordinary.samples.get(sample)
            && saved.physics_step == state.physics_step
        {
            ensure!(
                saved.sim_time_s == state.sim_time_s
                    && saved.held_command == state.held_command
                    && saved.observation == state.build_observation(&context),
                "source diagnostic sample differs at step {}",
                state.physics_step
            );
            sample += 1;
        }
        if requested.contains(&state.physics_step) {
            origins.insert(state.physics_step, state.clone());
        }
        if state.physics_step == ordinary.final_state.physics_step {
            break;
        }
        ensure!(
            !state.is_terminal(),
            "source execution ended before saved endpoint"
        );
        if state.physics_step.is_multiple_of(2) {
            let a = ordinary
                .actions
                .get(action)
                .context("source actions exhausted")?;
            ensure!(
                a.physics_step == state.physics_step
                    && a.sim_time_s == state.sim_time_s
                    && a.controller_update_index == state.physics_step / 2
                    && a.command == a.command.clamped(),
                "original source command/clock differs"
            );
            state.set_command(a.command);
            action += 1;
        }
        state.step_with_contact_report(&context);
        if state.physics_step.is_multiple_of(12)
            || requested.contains(&state.physics_step)
            || state.physics_step == ordinary.final_state.physics_step
        {
            actual.push(point(&state));
        }
    }
    ensure!(
        SimulationStateSnapshotV1::from_state(&state) == ordinary.final_state
            && action == ordinary.actions.len()
            && sample == ordinary.samples.len(),
        "source diagnostic reenactment differs from saved endpoint"
    );
    let mut cycles = vec![];
    for cycle in &flight.cycles {
        let origin = origins
            .get(&cycle.current_state.physics_step)
            .context("source cycle boundary missing")?;
        ensure!(
            SimulationStateSnapshotV1::from_state(origin) == cycle.current_state,
            "cycle origin differs from source flight"
        );
        let (nominal, nominal_extension) = nominal(&context, origin, cycle)?;
        let mut correction = None;
        let mut executed = vec![];
        if let Some(search) = &cycle.local_search
            && let (Some(p), Some(h)) = (&search.selected, search.actual_handoff_state())
        {
            for snapshot in [&p.entry_state, h] {
                ensure!(
                    origins
                        .get(&snapshot.physics_step)
                        .is_some_and(|s| SimulationStateSnapshotV1::from_state(s) == *snapshot),
                    "executed E/H annotation differs from source flight"
                );
            }
            executed = actual
                .iter()
                .filter(|point| {
                    point.time_s >= p.entry_state.sim_time_s && point.time_s <= h.sim_time_s
                })
                .cloned()
                .collect();
            correction = Some(ExecutedCorrection {
                number: cycle.cycle_index + 1,
                entry: boundary(&p.entry_state),
                handoff: boundary(h),
                reason: "Recorded nominal terrain conflict; local clearing executed".into(),
                after_handoff: "Replan from actual H state".into(),
            });
        }
        let decision = enum_text(&cycle.decision);
        let search = cycle.local_search.as_ref();
        let mut summary = format!(
            "{}: {}.",
            if cycle.cycle_index == 0 {
                "Launch".into()
            } else {
                format!("After H{}", cycle.cycle_index)
            },
            decision
        );
        if let Some(c) = &cycle.conflict_state {
            summary.push_str(&format!(" Proposed obstruction at t={:.2}s, x={:.2}m ({}) — a future query, not a flown collision.", c.sim_time_s,c.position_m.x,nominal_conflict_phase(cycle)));
        }
        if let Some(c) = &correction {
            summary.push_str(&format!(" Actual H{} changes velocity from ({:.2}, {:.2}) to ({:.2}, {:.2})m/s; height change {:+.2}m, Δvx={:+.2}, Δvy={:+.2}m/s, fuel burn {:.2}kg. The next cycle starts there, not at the certificate endpoint.", c.number,c.entry.velocity_mps.x,c.entry.velocity_mps.y,c.handoff.velocity_mps.x,c.handoff.velocity_mps.y,c.handoff.position_m.y-c.entry.position_m.y,c.handoff.velocity_mps.x-c.entry.velocity_mps.x,c.handoff.velocity_mps.y-c.entry.velocity_mps.y,c.entry.fuel_kg-c.handoff.fuel_kg));
        }
        if let Some(s) = search {
            summary.push_str(&format!(
                " Local search: {} admitted entries, {} rows, {} locally accepted rows.",
                s.entries.iter().filter(|e| e.admitted).count(),
                s.row_count,
                s.accepted_row_count
            ));
        }
        if decision == "no_clearing" {
            summary.push_str(" No row met every progress, body-reserve and supported-continuation guard. Execution stops at this cycle’s actual start. This is bounded search exhaustion, not proof that no route exists.");
        }
        if decision == "no_nominal" {
            summary.push_str(" No supported state-derived target proposal was found. No nominal path is invented for this cycle; inspect the recorded rejection reasons.");
        }
        let row_reviews =
            if queries && cycle.cycle_index + 1 == flight.cycles.len() && decision == "no_clearing"
            {
                selected_queries(cycle)
                    .into_iter()
                    .map(|r| {
                        query_review(
                            &context,
                            scenario,
                            origin,
                            cycle,
                            r,
                            flight
                                .absolute_deadline_physics_step
                                .context("diagnostic deadline")?,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                vec![]
            };
        cycles.push(CycleReview {
            index: cycle.cycle_index,
            decision,
            current: boundary(&cycle.current_state),
            nominal_identity: cycle.nominal_proposal_identity.clone(),
            nominal,
            nominal_extension,
            ballistic: ballistic(&context, origin),
            conflict: cycle.conflict_state.as_ref().map(boundary),
            conflict_phase: cycle
                .conflict_state
                .as_ref()
                .map(|_| nominal_conflict_phase(cycle).to_owned()),
            correction,
            executed,
            summary,
            boundary_counts: search
                .map(|s| s.boundary_status_counts.clone())
                .unwrap_or_default(),
            nominal_rejections: cycle.nominal_rejection_reason_counts.clone(),
            queries: row_reviews,
            active_goal: None,
            goal_label: None,
            previous_goal: None,
            clearing_crest: None,
            cutoff_coast: vec![],
        });
    }
    Ok(PlanningReview { schema: "pd-lab.planning-cycle-review.v1".into(),
        provenance: "Source-matched diagnostic reconstruction: original source actions → exact saved cycle origins; recorded nominal commands → exact saved conflict and audit endpoint. Terrain-blind extension is counterfactual after contact, not audited clearance or a landing proof. Rejected examples reenact only already-recorded rows and must match saved end states, boundary counts and first rejection. No new nominal search, route selection or mission flight. Raw capture and replay proof remain unchanged.".into(), cycles })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compiled_binding_rejects_mismatched_capture_without_simulating() {
        assert!(engine_binding(&|_| Ok(b"{\"files\":{}}".to_vec())).is_err());
    }
    #[test]
    fn incoming_rejection_identifies_target_crossing_not_a_terrain_crash() {
        let scenario: ScenarioSpec = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../fixtures/scenarios/flat_terminal_descent.json"
        )))
        .unwrap();
        let context = RunContext::from_scenario(&scenario).unwrap();
        let mut state = SimulationState::new(&context).unwrap();
        state.position_m.x = context.target_pad.center_x_m;
        state.velocity_mps.x = 10.0;
        assert_eq!(
            incoming_failures(&context, &state),
            vec!["x reached/passed the target: forward-only incoming family unsupported"]
        );
    }

    /// Optional local evidence check; ordinary tests never require captures.
    #[test]
    #[ignore = "requires an explicitly supplied retained early-exit capture"]
    fn retained_cycle_diagnostics_match_saved_programs() {
        let root = std::env::var("PD_PLANNING_REVIEW_CAPTURE").expect("explicit retained capture");
        let root = Path::new(&root);
        let receipt: Value =
            serde_json::from_slice(&fs::read(root.join("receipt.json")).unwrap()).unwrap();
        let auth = |relative: &str| -> Result<Vec<u8>> {
            let bytes = fs::read(safe_file(root, relative)?)?;
            ensure!(
                receipt["files"][relative] == sha256_bytes(&bytes)?,
                "retained file differs"
            );
            Ok(bytes)
        };
        engine_binding(&auth).unwrap();
        for id in QUERY_CASES {
            let scenario =
                serde_json::from_slice(&auth(&format!("runs/{id}/scenario.json")).unwrap())
                    .unwrap();
            let flight =
                serde_json::from_slice(&auth(&format!("runs/{id}/flight.json")).unwrap()).unwrap();
            let data =
                reconstruct(&scenario, &flight, true).unwrap_or_else(|e| panic!("{id}: {e:#}"));
            println!(
                "{id}: {} cycles, {} fixed rejected rows",
                data.cycles.len(),
                data.cycles.iter().map(|c| c.queries.len()).sum::<usize>()
            );
            if *id == "random-715" {
                assert!(data.cycles[1].queries.iter().any(|q| {
                    q.failed_predicates
                        .iter()
                        .any(|p| p.contains("passed the target"))
                }));
                assert_eq!(data.cycles[1].current.physics_step, 2480);
            }
        }
    }
}
