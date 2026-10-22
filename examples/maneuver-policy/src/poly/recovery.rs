//! Bounded forensic recovery of a saved burn's declared duration. This is not
//! a new duration policy: it may only identify a unique exact original command.
use super::*;
use pd_control::poly_landing::{
    LandingKinematicsV1, build_reference, paired_mean, paired_throttle,
};
use pd_core::{ActionLogEntry, Vec2};
use pd_missions::{sha256, write_json_create_only};
use serde_json::json;

fn same_command(left: Command, right: Command) -> bool {
    left.throttle_frac.to_bits() == right.throttle_frac.to_bits()
        && left.target_attitude_rad.to_bits() == right.target_attitude_rad.to_bits()
}

/// One original-prefix forward execution, bounded pure coefficient/inverse
/// queries, then one separately verified full-prefix timed-adapter execution.
pub fn recover_saved_burn(
    scenario_path: &Path,
    flight_path: &Path,
    output: &Path,
) -> Result<(), String> {
    let scenario_bytes = crate::verify::read_regular(scenario_path, 16 * 1024 * 1024)?;
    let flight_bytes = crate::verify::read_regular(flight_path, 64 * 1024 * 1024)?;
    let scenario: ScenarioSpec =
        serde_json::from_slice(&scenario_bytes).map_err(|e| e.to_string())?;
    let flight: serde_json::Value =
        serde_json::from_slice(&flight_bytes).map_err(|e| e.to_string())?;
    let scope = crate::source_scope(vec![
        std::fs::canonicalize(scenario_path).map_err(|e| e.to_string())?,
        std::fs::canonicalize(flight_path).map_err(|e| e.to_string())?,
    ]);
    let before = scope.snapshot()?;
    crate::check_compiled_sources(&before)?;
    if !before
        .input_files
        .iter()
        .any(|f| f.sha256 == sha256(&scenario_bytes))
        || !before
            .input_files
            .iter()
            .any(|f| f.sha256 == sha256(&flight_bytes))
    {
        return Err("historical bytes changed during binding".into());
    }
    if flight["physical_outcome"] != "landed_on_target"
        || flight["integrity_passed"] != true
        || flight["final_source_replay_passed"] != true
    {
        return Err(
            "historical burn recovery requires an explicitly saved verified target landing".into(),
        );
    }
    let deadline = flight["absolute_deadline_physics_step"]
        .as_u64()
        .ok_or("missing original deadline")?;
    let ctx = RunContext::from_scenario(&scenario)?;
    let mut updates: Vec<FlightProgramUpdateV1> = Vec::new();
    for segment in flight["segments"]
        .as_array()
        .ok_or("missing original segment ledger")?
    {
        let segment_updates: Vec<FlightProgramUpdateV1> =
            serde_json::from_value(segment["updates"].clone()).map_err(|e| e.to_string())?;
        updates.extend(segment_updates);
    }
    if updates.len() as u64 > crate::MAX_PHYSICS_STEPS / 2
        || updates
            .iter()
            .enumerate()
            .any(|(i, u)| u.physics_step != i as u64 * 2)
    {
        return Err(
            "original command ledger has gaps, duplicates or exceeds the software bound".into(),
        );
    }
    let first_index = updates
        .iter()
        .position(|u| u.phase == "terminal_bridge")
        .ok_or("saved landing has no finite terminal burn")?;
    if updates[first_index..]
        .iter()
        .any(|u| u.phase != "terminal_bridge" || u.command.throttle_frac <= 0.0)
    {
        return Err("saved terminal interval is not a single uninterrupted powered suffix".into());
    }
    let activation = updates[first_index].physics_step;
    let c = PolynomialLandingController::new(&scenario, activation, deadline)
        .map_err(|e| format!("{e:?}"))?;
    // Validate support before allocating native work; no controller command yet.
    let _ = c.descriptor();
    crate::verify::no_symlinks(output.parent().ok_or("recovery output needs a parent")?)?;
    std::fs::create_dir(output).map_err(|e| format!("reserve new recovery: {e}"))?;
    write_json_create_only(
        &output.join("recovery-inputs.json"),
        &json!({
            "schema_version":1,"scenario_sha256":sha256(&scenario_bytes),
            "historical_flight_sha256":sha256(&flight_bytes),"evidence":before,
            "planned_prefix_physics_steps":activation,"absolute_deadline_physics_step":deadline,
            "maximum_reference_queries":deadline.saturating_sub(activation + 4) / 2,
        }),
    )?;
    let limits = BoundedRunLimitsV1 {
        command_coverage_end_physics_step: deadline,
        hard_end_physics_step: deadline,
    };
    let mut prefix = Episode::reset(
        &scenario,
        "historical-poly-entry-recovery",
        limits,
        CaptureModeV1::Summary,
    )
    .map_err(|e| e.to_string())?;
    for update in &updates[..first_index] {
        if prefix.stop().is_some() || prefix.observation().physics_step != update.physics_step {
            write_json_create_only(
                &output.join("prefix-failure.json"),
                &json!({
                    "observation":prefix.observation(),"stop":prefix.stop(),
                    "reason":"original prefix ended before its saved burn boundary",
                }),
            )?;
            return Err(
                "original prefix ended before its saved burn boundary; recovery retained".into(),
            );
        }
        prefix.step(update.command).map_err(|e| e.to_string())?;
    }
    let entry = prefix.observation();
    if entry.physics_step != activation || prefix.stop().is_some() {
        return Err("original live entry unavailable".into());
    }
    let end = LandingKinematicsV1 {
        position_m: Vec2::new(
            ctx.target_pad.center_x_m,
            ctx.target_pad.surface_y_m + ctx.vehicle.geometry.touchdown_base_offset_m - 0.005,
        ),
        velocity_mps: Vec2::new(0.0, -0.5 * ctx.vehicle.safe_touchdown_normal_speed_mps),
    };
    let start = LandingKinematicsV1 {
        position_m: entry.position_m,
        velocity_mps: entry.velocity_mps,
    };
    let expected = updates[first_index].command;
    let mut matches = Vec::new();
    let mut constructions = 0;
    let mut inverse_queries = 0;
    let maximum = deadline
        .checked_sub(activation + 4)
        .ok_or("original clock has no residual allowance")?;
    for ticks in (2..=maximum).step_by(2) {
        constructions += 1;
        let Ok(reference) = build_reference(&ctx, start, end, ticks) else {
            continue;
        };
        let thrust = paired_mean(&reference, &ctx, 0);
        let angle = thrust.x.atan2(thrust.y);
        if angle.to_bits() != expected.target_attitude_rad.to_bits() {
            continue;
        }
        inverse_queries += 1;
        if let Ok(setting) = paired_throttle(&ctx, 48, entry.mass_kg, thrust.length())
            && same_command(
                Command {
                    throttle_frac: setting,
                    target_attitude_rad: angle,
                },
                expected,
            )
        {
            matches.push(ticks);
        }
    }
    let after = scope.snapshot()?;
    write_json_create_only(
        &output.join("entry-recovery.json"),
        &json!({
            "schema_version":1,"case_id":scenario.id,"scenario_sha256":sha256(&scenario_bytes),
            "historical_flight_sha256":sha256(&flight_bytes),"evidence_before":before,"evidence_after":after,
            "entry":entry,"prefix_physics_steps":activation,"prefix_command_updates":first_index,
            "pure_reference_queries":constructions,"paired_inverse_queries":inverse_queries,
            "candidate_native_forecast_steps":0,"matching_duration_ticks":matches,
            "method":"unique exact first-command parameter recovery; not a duration policy or replay of old planner selection",
        }),
    )?;
    if before != after {
        return Err("recovery input/source drift".into());
    }
    if matches.len() != 1 {
        return Err(format!(
            "duration recovery requires one exact match, found {}; evidence retained",
            matches.len()
        ));
    }
    let program = TimedProgramV2 {
        schema_version: 2,
        backend: POLY_LANDING_BACKEND.into(),
        activation_physics_step: activation,
        absolute_deadline_physics_step: deadline,
        request: ManeuverRequestV2 {
            schema_version: 2,
            id: "historical-poly-landing".into(),
            goal: ManeuverGoalV2::LandOnMissionTargetAt {
                target_pad_id: ctx.target_pad.id.clone(),
                reference_end_physics_step: activation + matches[0],
            },
        },
        regression_prefix: updates[..first_index].to_vec(),
        rejection_coast: Command::idle(),
    };
    write_json_create_only(&output.join("program.json"), &program)?;
    let (evaluation, trace) = evaluate_files_with_inputs(
        scenario_path,
        &output.join("program.json"),
        &[flight_path.to_owned()],
    )?;
    verify::save_bundle(&output.join("bundle"), &evaluation, &trace)?;
    let verified = verify::verify_bundle(&output.join("bundle"));
    write_json_create_only(&output.join("verification.json"), &verified.report)?;
    let original_actions: Vec<ActionLogEntry> =
        serde_json::from_value(flight["ordinary_flight"]["actions"].clone())
            .map_err(|e| e.to_string())?;
    let actual = &evaluation.execution.episode;
    let commands_match = original_actions.len() == actual.run.actions.len()
        && original_actions
            .iter()
            .zip(&actual.run.actions)
            .all(|(a, b)| {
                a.physics_step == b.physics_step
                    && a.sim_time_s.to_bits() == b.sim_time_s.to_bits()
                    && a.controller_update_index == b.controller_update_index
                    && same_command(a.command, b.command)
            });
    let same_value =
        |value: serde_json::Value, original: &serde_json::Value| -> Result<bool, String> {
            Ok(serde_json::to_vec(&value).map_err(|e| e.to_string())?
                == serde_json::to_vec(original).map_err(|e| e.to_string())?)
        };
    let final_state_match = same_value(
        serde_json::to_value(&actual.final_state).map_err(|e| e.to_string())?,
        &flight["ordinary_flight"]["final_state"],
    )?;
    let contact_match = same_value(
        serde_json::to_value(&actual.incoming_contact).map_err(|e| e.to_string())?,
        &flight["ordinary_flight"]["incoming_contact"],
    )?;
    let samples_match = same_value(
        serde_json::to_value(&actual.run.samples).map_err(|e| e.to_string())?,
        &flight["ordinary_flight"]["samples"],
    )?;
    let passed = commands_match
        && final_state_match
        && contact_match
        && samples_match
        && evaluation.verified_target_landing
        && verified.report.passed();
    write_json_create_only(
        &output.join("parity.json"),
        &json!({
            "schema_version":1,"case_id":scenario.id,"passed":passed,"commands_bit_exact":commands_match,
            "final_state_exact":final_state_match,"incoming_contact_exact":contact_match,"samples_exact":samples_match,
            "verified_target_landing":evaluation.verified_target_landing,"composite_verification":verified.report.passed(),
            "activation_physics_step":activation,"reference_duration_ticks":matches[0],
            "original_episode_physics_steps":trace.physics_steps,"original_command_updates":trace.command_updates,
            "prefix_recovery_physics_steps":activation,"pure_reference_queries":constructions,
        }),
    )?;
    if !passed {
        return Err("historical adapter parity failed; every artifact retained".into());
    }
    Ok(())
}
