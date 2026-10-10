//! Saved-prefix diagnostic frontdoor; computations live in the solver crate.
use super::*;
use pd_control::{ControllerFrame, ControllerUpdateRecord};
pub use pd_planner::ballistic_feedback::terminal_coast_diagnostic::TerminalSetup;
use pd_planner::ballistic_feedback::terminal_coast_diagnostic::{
    fly, replay_prefix, source_updates, validate_boundaries,
};
use pd_planner::local_clearing::{advance_ordinary, new_ordinary};
pub fn run(
    scenario_path: &Path,
    baseline_path: &Path,
    prefix_handoff: usize,
    takeover_handoff: usize,
    setup: TerminalSetup,
    output: &Path,
) -> Result<FeedbackResult> {
    let scenario_bytes = fs::read(scenario_path)?;
    let scenario: ScenarioSpec = serde_json::from_slice(&scenario_bytes)?;
    let baseline_bytes = fs::read(baseline_path)?;
    let baseline: FeedbackResult = serde_json::from_slice(&baseline_bytes)?;
    let (prefix, takeover) = validate_boundaries(
        &baseline,
        &sha256_bytes(&scenario_bytes)?,
        prefix_handoff,
        takeover_handoff,
    )?;
    let ctx = RunContext::from_scenario(&scenario).map_err(anyhow::Error::msg)?;
    let request = WaypointDirectNominalDirectGenerationRequest {
        probe_id: scenario.id.clone(),
        scenario: scenario.clone(),
        source_pad_id: "pad_source".into(),
        target_pad_id: ctx.target_pad.id.clone(),
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    };
    ensure!(
        crate::preflight_waypoint_v2_flight(
            &request,
            &pd_plan::waypoint_v2::WaypointV2Policy::revision_3(),
        )
        .supported,
        "unsupported diagnostic source"
    );
    let source = crate::waypoint_v2_pack::capture_source_state(&crate::repo_root())?;
    let binary_sha256 = sha256_bytes(&fs::read(std::env::current_exe()?)?)?;
    let renderer_sha256 = sha256_bytes(&fs::read(
        crate::repo_root().join("pd-report/src/planning_cycles.js"),
    )?)?;
    reserve_output_root(output)?;
    write_json_create_only(
        &output.join("attempt.json"),
        &serde_json::json!({
            "diagnostic": "saved_prefix_coast_terminal_capability",
            "source": source, "binary_sha256": binary_sha256,
            "renderer_sha256": renderer_sha256,
            "scenario_sha256": baseline.scenario_sha256,
            "baseline_sha256": sha256_bytes(&baseline_bytes)?,
            "prefix_handoff": prefix_handoff, "takeover_handoff_clock": takeover_handoff,
            "entry_physics_step": takeover.physics_step, "terminal_setup": setup,
            "timing_is_forced_capability_probe_not_entry_policy": true,
            "new_waypoints_after_prefix": 0, "terrain_neutral": false,
            "short_prediction_ticks": REFRESH_TICKS, "case_wall_limit_s": 60,
        }),
    )?;
    crate::evidence_io::write_bytes_create_only_with_context(
        &output.join("scenario.json"),
        &scenario_bytes,
        "unchanged diagnostic source",
    )?;
    // Verify all saved source commands and the complete retained evidence first.
    let mut original = new_ordinary(&ctx)?;
    advance_ordinary(
        &ctx,
        &mut original,
        &source_updates(&baseline),
        baseline.final_state.physics_step,
    )?;
    ensure!(
        original.evidence == baseline.ordinary_flight,
        "saved source replay differs"
    );
    write_json_create_only(
        &output.join("prefix-proof.json"),
        &serde_json::json!({
            "original_complete_source_replay_passed": true,
            "prefix_origin": prefix, "baseline_clock_reference": takeover,
            "prefix_source_replay_passed": replay_prefix(&ctx, &baseline, prefix)?.evidence.final_state == *prefix,
        }),
    )?;
    let trial = fly(&request, &baseline, prefix, takeover.physics_step, setup)?;
    write_json_create_only(
        &output.join("executed-raw.json"),
        &serde_json::json!({
            "ordinary_flight": trial.live.evidence, "updates": trial.updates,
            "refreshes": trial.refreshes, "stop": trial.stop, "verification_pending": true,
        }),
    )?;
    // Rebuild from the original source, not from a restored/cloned H snapshot.
    let mut replay = replay_prefix(&ctx, &baseline, prefix)?;
    for update in &trial.updates {
        advance_one(&ctx, &mut replay, update, false)?;
    }
    ensure!(
        replay.evidence == trial.live.evidence,
        "diagnostic command replay differs"
    );
    write_json_create_only(
        &output.join("command-replay.json"),
        &serde_json::json!({
            "passed": true, "original_source_rebuilt": true,
            "final_state": replay.evidence.final_state,
        }),
    )?;
    let repeat = fly(&request, &baseline, prefix, takeover.physics_step, setup)?;
    ensure!(
        repeat.live.evidence == trial.live.evidence
            && repeat.updates == trial.updates
            && repeat.frames == trial.frames
            && repeat.refreshes == trial.refreshes
            && repeat.entry == trial.entry
            && repeat.entry_dynamics_ready == trial.entry_dynamics_ready
            && repeat.stop == trial.stop,
        "diagnostic decisions differ"
    );
    ensure!(
        crate::waypoint_v2_pack::capture_source_state(&crate::repo_root())?.rust_source_tree_sha256
            == source.rust_source_tree_sha256
            && sha256_bytes(&fs::read(std::env::current_exe()?)?)? == binary_sha256
            && sha256_bytes(&fs::read(
                crate::repo_root().join("pd-report/src/planning_cycles.js")
            )?)? == renderer_sha256
            && fs::read(scenario_path)? == scenario_bytes
            && fs::read(baseline_path)? == baseline_bytes,
        "diagnostic source/binary/input drift"
    );
    write_json_create_only(
        &output.join("terminal-diagnostic.json"),
        &serde_json::json!({
            "entry": trial.entry, "dynamics_only_entry_ready": trial.entry_dynamics_ready,
            "entry_timing_forced": true, "terminal_setup": setup,
            "coast_checkpoint_motion_matches_original": trial.entry.as_ref().is_some_and(|s|
                s.position_m == takeover.position_m && s.velocity_mps == takeover.velocity_mps && s.fuel_kg == takeover.fuel_kg),
            "controller_updates": trial.frames,
        }),
    )?;
    let mut refreshes: Vec<_> = baseline
        .refreshes
        .iter()
        .filter(|r| r.origin.physics_step < prefix.physics_step)
        .cloned()
        .collect();
    refreshes.extend(trial.refreshes);
    let result = FeedbackResult {
        candidate_id: format!("ballistic_terminal_coast_diagnostic_{setup:?}"),
        scenario_sha256: baseline.scenario_sha256.clone(),
        prefix_flight_sha256: Some(sha256_bytes(&baseline_bytes)?),
        prefix_origin: Some(prefix.clone()),
        terrain_neutral_diagnostic: false,
        stop: trial.stop.clone(),
        reason: trial.stop,
        refreshes,
        handoffs: baseline.handoffs[..prefix_handoff].to_vec(),
        handoff_goal_revisions: baseline.handoff_goal_revisions[..prefix_handoff].to_vec(),
        updates: trial.updates,
        final_state: trial.live.evidence.final_state.clone(),
        integrity_passed: true,
        source_replay_passed: true,
        decisions_reproduced: true,
        ordinary_flight: trial.live.evidence,
        terrain_domain_stop: None,
        controller_updates: vec![],
    };
    write_json_create_only(&output.join("feedback.json"), &result)?;
    let mut frames: Vec<_> = result
        .ordinary_flight
        .actions
        .iter()
        .filter(|a| a.physics_step < prefix.physics_step)
        .map(|a| ControllerUpdateRecord {
            sim_time_s: a.sim_time_s,
            physics_step: a.physics_step,
            controller_update_index: a.controller_update_index,
            compute_time_us: None,
            frame: ControllerFrame::command_only(a.command),
        })
        .collect();
    frames.extend(trial.frames);
    report::write_with_updates(&scenario, &result, &trial.live.state, output, Some(&frames))?;
    Ok(result)
}
