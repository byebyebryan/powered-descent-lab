//! Built-in ballistic capture frontdoor. Algorithms belong to `pd-planner`.
use crate::{
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    WaypointV2FlightResult,
    evidence_io::{reserve_output_root, sha256_bytes, write_json_create_only},
};
use anyhow::{Result, ensure};
use pd_core::{RunContext, ScenarioSpec};
pub use pd_planner::ballistic_feedback::*;
use std::{fs, path::Path};
mod report;
pub mod terminal_coast_diagnostic;
/// Explicit candidate frontdoor. An optional retained cycle is rebuilt from
/// original source commands; no arbitrary airborne snapshot can be supplied.
pub fn run(
    scenario_path: &Path,
    baseline_path: Option<&Path>,
    cycle_index: usize,
    neutral: bool,
    correction_cap: usize,
    experiment: WaypointExperiment,
    output: &Path,
) -> Result<FeedbackResult> {
    ensure!(
        !neutral || baseline_path.is_some(),
        "neutral diagnostic needs a proven retained prefix"
    );
    ensure!(
        [6, 24].contains(&correction_cap),
        "unsupported candidate correction cap"
    );
    reserve_output_root(output)?;
    let scenario_bytes = fs::read(scenario_path)?;
    let scenario: ScenarioSpec = serde_json::from_slice(&scenario_bytes)?;
    let ctx = RunContext::from_scenario(&scenario).map_err(anyhow::Error::msg)?;
    let request = WaypointDirectNominalDirectGenerationRequest {
        probe_id: scenario.id.clone(),
        scenario: scenario.clone(),
        source_pad_id: "pad_source".into(),
        target_pad_id: ctx.target_pad.id.clone(),
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    };
    let source_state = crate::current_workspace_state()?;
    let content_source = crate::waypoint_v2_pack::capture_source_state(&crate::repo_root())?;
    let report_script_sha256 = sha256_bytes(&fs::read(
        crate::repo_root().join("pd-report/src/planning_cycles.js"),
    )?)?;
    let binary_sha256 = sha256_bytes(&fs::read(std::env::current_exe()?)?)?;
    let transition_metadata = serde_json::json!({
        "queries_recorded": experiment.phase_queries(),
        "actual_coast_settling": experiment.coast_transition(),
        "configured_terminal_takeover": experiment.terminal_takeover(),
        "pad_reserve_command_adapter": experiment.pad_clearance(),
        "recovery_diagnostic_only": !experiment.queued_recovery(),
        "terminal_prefix_ticks": CONTINUATION_TICKS,
        "query_refresh_ticks": REFRESH_TICKS,
        "ordinary_default_changed": false,
        "physical_guards_changed": false,
    });
    write_json_create_only(
        &output.join("attempt.json"),
        &serde_json::json!({
            "candidate_id": experiment.candidate_id(), "commit": source_state.commit_key,
            "workspace": source_state.workspace_key, "dirty": source_state.dirty,
            "content_source": content_source, "report_script_sha256": report_script_sha256,
            "binary_sha256": binary_sha256, "scenario_sha256": sha256_bytes(&scenario_bytes)?,
            "baseline_path": baseline_path, "cycle_index": cycle_index,
            "terrain_neutral_diagnostic": neutral, "correction_cap": correction_cap,
            "refresh_ticks": REFRESH_TICKS, "minimum_descent_angle_rad": MIN_ANGLE,
            "short_prediction_ticks": REFRESH_TICKS, "continuation_ticks": CONTINUATION_TICKS,
            "maximum_profile_trials": MAX_PROFILE_TRIALS, "case_wall_limit_s": 60,
            "maximum_local_proposals": MAX_LOCAL_PROPOSALS,
            "terminal_coordination": {
                "enabled": experiment.terminal_coordination(),
                "early_braking": "preserve_nominal_lateral_acceleration_with_lift_constraint",
                "touchdown": "body_contained_low_energy_upright_settlement",
                "ordinary_default_changed": false,
                "standalone_coast_terminal_changed": false,
                "physical_guards_changed": false,
            },
            "landing_body_centering": {
                "enabled": experiment.landing_body_centering(),
                "rule": "rotated_hull_and_feet_rescue_pad_interval",
                "reuse_outside_pad_lateral_target": true,
                "vertical_authority_cap_unchanged": true,
                "standalone_coast_terminal_changed": false,
                "ordinary_default_changed": false,
                "short_command_guard_changed": false,
            },
            "landing_braking_guard": {
                "enabled": experiment.landing_braking_guard(),
                "rule": "existing_braking_envelope_activates_touchdown_rescue",
                "nominal_upward_acceleration_insufficient": true,
                "standalone_coast_terminal_changed": false,
                "ordinary_default_changed": false,
                "short_command_guard_changed": false,
            },
            "coast_terminal": {
                "enabled": experiment.coast_terminal(),
                "only_after_actual_handoff": true,
                "trigger": "blocked_destination_arc",
                "checkpoint": "first_crest_plus_half_body_diameter",
                "maximum_coast_ticks": coast_terminal::MAX_COAST_TICKS,
                "terminal_preview_ticks": CONTINUATION_TICKS,
                "terminal_setup": "standalone_defaults",
                "entry_query": "existing_dynamics_then_actual_feedback_preview",
                "configured_terminal_terrain_enabled": true,
                "complete_landing_suffix_required": false,
                "saved_clock_used": false,
            },
            "landing_countdown": {
                "enabled": experiment.landing_countdown(),
                "rule": "retain_first_selected_ballistic_fallback_arrival",
                "admissions_per_landing": 1,
                "remaining_time_floor_s": 0.5,
                "revalidate_each_update": true,
                "release_on_infeasible_or_expired": true,
                "target_convention_changed": false,
                "ordinary_default_changed": false,
            },
            "landing_duration": {
                "enabled": experiment.landing_duration(),
                "rule": "initial_vertical_balance_after_all_latest_safe_fits_fail",
                "additional_queries": 1,
                "burn_time_min_s": 3.0,
                "burn_time_max_s": 14.0,
                "shared_entry_and_live_controller": true,
                "ordinary_default_changed": false,
            },
            "local_waypoint": {
                "feature_local_height": experiment.local_height(),
                "negative_braking_room_destination_preview": experiment.early_target(),
                "preview_records_handoff": false,
                "destination_constructor_changed": false,
            },
            "waypoint_entry": {
                "least_added_thrust_effort": experiment.effort(),
                "forward_crossing_recovery": experiment.recovery(),
                "retain_safe_pending_coast": experiment.recovery(),
                "apex_constraint": false,
                "destination_constructor_changed": false,
            },
            "waypoint_clearance": {
                "repair": "fixed_x_body_envelope_height_lift",
                "continuation_ticks": CONTINUATION_TICKS,
                "numeric_height_margin_m": clearance::NUMERIC_HEIGHT_MARGIN_M,
                "direct_constructor_terrain_blind": true,
            },
            "ridge_placement": {
                "rule": "first_crest_body_diameter_drop",
                "valley_drop_vehicle_diameters": 1,
                "crest_offset_vehicle_diameters": 1,
                "unresolved_slope": "local_climb_stage",
                "landing_suffix_required": false,
            },
            "terrain_correction": {
                "maximum_commands": avoidance::MAX_COMMANDS,
                "maximum_episode_ticks": avoidance::MAX_RECOVERY_TICKS,
                "maximum_prediction_ticks": CONTINUATION_TICKS,
                "braking_tilt_rad": avoidance::BRAKING_TILT,
                "short_command_changes_goal": false,
            },
            "planner_mechanics": {
                "enabled": experiment.mechanics(),
                "waypoint_exit_consistency": experiment.exit_consistency(),
                "piecewise_early_target": experiment.piecewise_early_target(),
                "recovery_lead": experiment.recovery_lead(),
                "recovery_choices_recorded": experiment.mechanics(),
                "ordinary_default_changed": false,
                "physical_guards_changed": false,
            },
            "finite_correction": {
                "queries_recorded": experiment.finite_queries(),
                "admission_enabled": experiment.finite_correction(),
                "powered_prefix_plant_checked": experiment.finite_queries(),
                "optional_refresh_ticks": REFRESH_TICKS,
                "initial_nominal_changed": false,
                "waypoint_ranking_changed": false,
                "physical_guards_changed": false,
            },
            "phase_transition": transition_metadata,
            "acquisition_and_terminal_safety": {
                "open_destination_acquisition": experiment.open_destination_acquisition(),
                "terminal_safety_fallback": experiment.terminal_safety_fallback(),
                "fallback_choices": ["upright_coast", "upright_support"],
                "query_refresh_ticks": REFRESH_TICKS,
                "physical_guards_changed": false,
                "standalone_coast_terminal_changed": false,
            },
            "queued_recovery": {
                "enabled": experiment.queued_recovery(),
                "bounded_comparison": experiment.recovery_consistency(),
                "goals_unchanged": true,
            },
        }),
    )?;
    let configuration =
        pd_planner::BallisticFeedbackConfigV1::from_legacy(correction_cap, experiment)?;
    write_json_create_only(&output.join("configuration-v1.json"), &configuration)?;
    let preflight = crate::preflight_waypoint_v2_flight(
        &request,
        &pd_plan::waypoint_v2::WaypointV2Policy::revision_3(),
    );
    crate::evidence_io::write_bytes_create_only_with_context(
        &output.join("scenario.json"),
        &scenario_bytes,
        "unchanged candidate input",
    )?;
    write_json_create_only(&output.join("preflight.json"), &preflight)?;
    ensure!(preflight.supported, "candidate input unsupported");
    let mut prefix_hash = None;
    let mut prefix_state = None;
    let baseline = if let Some(path) = baseline_path {
        let bytes = fs::read(path)?;
        let baseline: WaypointV2FlightResult = serde_json::from_slice(&bytes)?;
        prefix_hash = Some(sha256_bytes(&bytes)?);
        Some(baseline)
    } else {
        None
    };
    let origin = BallisticOriginV1::from_source(&request, baseline.as_ref(), cycle_index)?;
    if baseline.is_some() {
        prefix_state = Some(origin.evidence().final_state.clone());
    }
    write_json_create_only(&output.join("prefix-proof.json"), origin.evidence())?;
    let BallisticExecutionV1 {
        live,
        refreshes,
        handoffs,
        handoff_goal_revisions,
        updates,
        stop,
        terrain_domain_stop,
        controller_updates,
    } = execute_configured(&request, &origin, neutral, &configuration)?;
    write_json_create_only(
        &output.join("executed-raw.json"),
        &serde_json::json!({
            "ordinary_flight": live.evidence, "refreshes": refreshes,
            "handoffs": handoffs, "updates": updates, "stop": stop,
            "handoff_goal_revisions": handoff_goal_revisions,
            "terrain_domain_stop": terrain_domain_stop,
            "controller_updates": controller_updates,
            "verification_pending": true,
        }),
    )?;
    let replay = origin.replay_commands(&request, &updates, neutral)?;
    write_json_create_only(
        &output.join("command-replay.json"),
        &serde_json::json!({
            "passed": replay == live.evidence,
            "final_state": replay.final_state,
        }),
    )?;
    ensure!(
        replay == live.evidence,
        "candidate exact command source replay differs"
    );
    let repeat = execute_configured(&request, &origin, neutral, &configuration)?;
    ensure!(
        repeat.live.evidence == live.evidence
            && repeat.refreshes == refreshes
            && repeat.handoffs == handoffs
            && repeat.handoff_goal_revisions == handoff_goal_revisions
            && repeat.updates == updates
            && repeat.stop == stop
            && repeat.terrain_domain_stop == terrain_domain_stop,
        "candidate feedback decisions not deterministic"
    );
    ensure!(
        repeat.controller_updates == controller_updates,
        "candidate feedback decisions not deterministic"
    );
    let final_source = crate::current_workspace_state()?;
    let final_content = crate::waypoint_v2_pack::capture_source_state(&crate::repo_root())?;
    ensure!(
        final_source.workspace_key == source_state.workspace_key
            && final_content.rust_source_tree_sha256 == content_source.rust_source_tree_sha256
            && sha256_bytes(&fs::read(
                crate::repo_root().join("pd-report/src/planning_cycles.js")
            )?)? == report_script_sha256
            && sha256_bytes(&fs::read(std::env::current_exe()?)?)? == binary_sha256,
        "candidate source contents or binary drift during attempt"
    );
    let result = FeedbackResult {
        candidate_id: experiment.candidate_id().into(),
        scenario_sha256: sha256_bytes(&scenario_bytes)?,
        prefix_flight_sha256: prefix_hash,
        prefix_origin: prefix_state,
        terrain_neutral_diagnostic: neutral,
        reason: stop.clone(),
        stop,
        refreshes,
        handoffs,
        handoff_goal_revisions,
        updates,
        final_state: live.evidence.final_state.clone(),
        integrity_passed: true,
        source_replay_passed: true,
        decisions_reproduced: true,
        ordinary_flight: live.evidence,
        terrain_domain_stop,
        controller_updates,
    };
    write_json_create_only(&output.join("feedback.json"), &result)?;
    report::write(&scenario, &result, &live.state, output)?;
    Ok(result)
}
