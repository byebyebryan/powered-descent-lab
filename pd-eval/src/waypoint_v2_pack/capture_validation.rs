use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use pd_core::ScenarioSpec;
use pd_plan::waypoint_v2::WaypointV2Stop;
use serde_json::Value;

use super::{
    WAYPOINT_V2_BATCH_SCHEMA_ID,
    execution::{enum_text, source_path, summarize},
    input::{EXPECTED_CASE_COUNT, policy_for_version},
    model::{WaypointV2BatchCase, WaypointV2BatchReport, WaypointV2PackInput},
    provenance::digest_expanded_inputs,
};
use crate::{WaypointV2FlightResult, evidence_io::sha256_bytes};

pub(super) fn safe_capture_file(root: &Path, relative: &str) -> Result<PathBuf> {
    let root_metadata = fs::symlink_metadata(root)?;
    ensure!(
        root_metadata.is_dir() && !root_metadata.file_type().is_symlink(),
        "capture root must be a real directory"
    );
    let relative_path = Path::new(relative);
    ensure!(
        !relative_path.is_absolute()
            && relative_path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "unsafe capture-relative path {relative}"
    );
    let canonical_root = root.canonicalize()?;
    let mut cursor = canonical_root.clone();
    for component in relative_path.components() {
        cursor.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&cursor)
            .with_context(|| format!("missing batch artifact {relative}"))?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "symlink in captured artifact path {relative}"
        );
    }
    let resolved = cursor.canonicalize()?;
    ensure!(
        resolved.starts_with(canonical_root) && resolved.is_file(),
        "batch artifact escapes capture root or is not a regular file: {relative}"
    );
    Ok(resolved)
}

fn case_result_matches(case: &WaypointV2BatchCase, result: &WaypointV2FlightResult) -> Result<()> {
    ensure!(case.planning_stop.as_deref() == Some(enum_text(&result.planning_stop)?.as_str()));
    ensure!(case.reason == result.reason);
    ensure!(case.correction_count == Some(result.correction_count));
    ensure!(case.initial_nominal_terrain_blocked == Some(result.initial_nominal_terrain_blocked));
    ensure!(case.integrity_passed == Some(result.integrity_passed));
    ensure!(case.final_source_replay_passed == Some(result.final_source_replay_passed));
    ensure!(
        case.physical_outcome
            == result
                .physical_outcome
                .as_ref()
                .map(enum_text)
                .transpose()?
    );
    ensure!(case.mission_outcome == result.mission_outcome.as_ref().map(enum_text).transpose()?);
    ensure!(case.planning_s == Some(result.timings.planning_s));
    ensure!(case.execution_s == Some(result.timings.execution_s));
    ensure!(case.replay_s == Some(result.timings.replay_s));
    ensure!(
        case.status
            == if result.manifest.is_some() {
                "simulated"
            } else if result.ordinary_flight.is_some() {
                "simulation_unverified"
            } else {
                "preflight_rejected"
            }
    );
    Ok(())
}

pub(super) fn validate_full_evidence_consistency(
    input: &WaypointV2PackInput,
    result: &WaypointV2FlightResult,
    compact: &Value,
) -> Result<()> {
    crate::waypoint_v2::early_exit::validate_records(result)?;
    let manifest = result
        .manifest
        .as_ref()
        .context("simulated V2 result is missing its manifest")?;
    let ordinary = result
        .ordinary_flight
        .as_ref()
        .context("simulated V2 result is missing ordinary flight evidence")?;
    let final_state = &ordinary.final_state;
    let scenario_id = input.scenario.id.as_str();

    ensure!(
        manifest.scenario_id == scenario_id
            && manifest.physics_hz == input.scenario.sim.physics_hz
            && manifest.controller_hz == input.scenario.sim.controller_hz,
        "full-flight manifest scenario or clock differs from captured input for {scenario_id}"
    );
    ensure!(
        result.physical_outcome.as_ref() == Some(&manifest.physical_outcome)
            && result.mission_outcome.as_ref() == Some(&manifest.mission_outcome),
        "V2 result physical or mission outcome differs from manifest for {scenario_id}"
    );
    ensure!(
        manifest.physics_steps == final_state.physics_step
            && manifest.sim_time_s == final_state.sim_time_s
            && manifest.end_reason == final_state.end_reason
            && manifest.physical_outcome == final_state.physical_outcome
            && manifest.mission_outcome == final_state.mission_outcome,
        "manifest endpoint differs from ordinary final state for {scenario_id}"
    );
    ensure!(
        manifest.controller_updates == ordinary.actions.len() as u64,
        "manifest controller update count differs from saved action count for {scenario_id}"
    );
    if ordinary.actions.is_empty() {
        ensure!(
            manifest.physics_steps == 0
                && manifest.controller_updates == 0
                && manifest.sim_time_s == 0.0
                && ordinary.samples.len() == 1
                && ordinary.samples[0].physics_step == 0
                && ordinary.samples[0].sim_time_s == 0.0,
            "zero-command full-flight evidence must retain only the initial zero-step sample for {scenario_id}"
        );
    }
    ensure!(
        manifest.summary.fuel_remaining_kg == final_state.fuel_kg
            && manifest.summary.min_touchdown_clearance_m == final_state.min_touchdown_clearance_m
            && manifest.summary.min_hull_clearance_m == final_state.min_hull_clearance_m,
        "manifest fuel or clearance summary differs from ordinary final state for {scenario_id}"
    );

    let saved_run = &compact["run_summary"];
    ensure!(
        saved_run.is_object(),
        "simulated V2 result is missing compact run summary for {scenario_id}"
    );
    ensure!(
        saved_run["endpoint"]["physics_step"] == manifest.physics_steps
            && saved_run["endpoint"]["sim_time_s"] == manifest.sim_time_s
            && saved_run["endpoint"]["physical_outcome"]
                == serde_json::to_value(&manifest.physical_outcome)?
            && saved_run["endpoint"]["mission_outcome"]
                == serde_json::to_value(&manifest.mission_outcome)?
            && saved_run["endpoint"]["end_reason"] == serde_json::to_value(&manifest.end_reason)?,
        "compact endpoint differs from full-flight manifest for {scenario_id}"
    );
    ensure!(
        saved_run["fuel"]["remaining_kg"] == manifest.summary.fuel_remaining_kg
            && saved_run["fuel"]["used_kg"] == manifest.summary.fuel_used_kg
            && saved_run["minimum_clearance"]["touchdown_m"]
                == manifest.summary.min_touchdown_clearance_m
            && saved_run["minimum_clearance"]["hull_m"] == manifest.summary.min_hull_clearance_m
            && saved_run["minimum_clearance"]["landing"]
                == serde_json::to_value(&manifest.summary.landing)?,
        "compact fuel or minimum-clearance values differ from full-flight manifest for {scenario_id}"
    );
    Ok(())
}

pub(super) fn validate_batch_capture(
    root: &Path,
    report: &WaypointV2BatchReport,
) -> Result<Vec<(WaypointV2PackInput, WaypointV2FlightResult)>> {
    ensure!(report.schema_id == WAYPOINT_V2_BATCH_SCHEMA_ID);
    ensure!(report.status == "completed");
    ensure!(report.case_count == report.cases.len() && report.cases.len() == EXPECTED_CASE_COUNT);
    ensure!(report.pack_id == "planner_v2_lab_suite");
    let pack_bytes = fs::read(safe_capture_file(root, &report.pack_snapshot_path)?)?;
    ensure!(sha256_bytes(&pack_bytes)? == report.pack_snapshot_sha256);
    ensure!(report.pack_snapshot_sha256 == report.input_identity.pack_file_sha256);
    let inputs_bytes = fs::read(safe_capture_file(
        root,
        &report.expanded_inputs_snapshot_path,
    )?)?;
    ensure!(sha256_bytes(&inputs_bytes)? == report.expanded_inputs_snapshot_sha256);
    let inputs: Vec<WaypointV2PackInput> = serde_json::from_slice(&inputs_bytes)?;
    ensure!(inputs.len() == report.cases.len());
    ensure!(
        digest_expanded_inputs(&inputs)? == report.input_identity.rust_typed_expanded_inputs_sha256
    );
    let mut verified = Vec::with_capacity(inputs.len());
    let mut seen = BTreeSet::new();
    for (input, case) in inputs.into_iter().zip(&report.cases) {
        ensure!(
            seen.insert(case.case_id.as_str()),
            "duplicate report case ID"
        );
        ensure!(
            input.case_id == case.case_id
                && input.source_set == case.source_set
                && input.source_group == case.source_group
                && input.group == case.group
                && input.family == case.family
                && input.base_case_id == case.base_case_id
                && input.source_pad_id == case.source_pad_id
                && input.target_pad_id == case.target_pad_id
                && input.expected_preflight == case.expected_preflight,
            "case row does not match expanded input {}",
            case.case_id
        );
        ensure!(case.input_path == source_path(&input));
        let scenario_bytes = fs::read(safe_capture_file(root, &case.scenario_path)?)?;
        ensure!(
            case.artifact_sha256.get(&case.scenario_path) == Some(&sha256_bytes(&scenario_bytes)?),
            "scenario artifact seal mismatch for {}",
            case.case_id
        );
        let scenario: ScenarioSpec = serde_json::from_slice(&scenario_bytes)?;
        ensure!(
            scenario == input.scenario,
            "captured scenario drift for {}",
            case.case_id
        );
        let flight_bytes = fs::read(safe_capture_file(root, &case.flight_path)?)?;
        ensure!(
            case.artifact_sha256.get(&case.flight_path) == Some(&sha256_bytes(&flight_bytes)?),
            "flight artifact seal mismatch for {}",
            case.case_id
        );
        let result: WaypointV2FlightResult = serde_json::from_slice(&flight_bytes)?;
        case_result_matches(case, &result)?;
        ensure!(result.policy == policy_for_version(report.policy_version)?);
        ensure!(
            case.outcome
                == case
                    .physical_outcome
                    .clone()
                    .or_else(|| case.planning_stop.clone())
        );
        match case.expected_preflight.as_deref() {
            Some("unsupported") => ensure!(
                result.planning_stop == WaypointV2Stop::Unsupported
                    && result.manifest.is_none()
                    && result.ordinary_flight.is_none(),
                "unexpected execution for unsupported case {}",
                case.case_id
            ),
            Some("invalid_input") => ensure!(
                result.planning_stop == WaypointV2Stop::InvalidInput
                    && result.manifest.is_none()
                    && result.ordinary_flight.is_none(),
                "unexpected execution for invalid-input case {}",
                case.case_id
            ),
            None => ensure!(
                !matches!(
                    result.planning_stop,
                    WaypointV2Stop::Unsupported | WaypointV2Stop::InvalidInput
                ),
                "unexpected preflight rejection for {}",
                case.case_id
            ),
            Some(other) => bail!("unsupported expected preflight {other}"),
        }
        let run_summary_bytes = fs::read(safe_capture_file(root, &case.summary_path)?)?;
        ensure!(
            case.artifact_sha256.get(&case.summary_path)
                == Some(&sha256_bytes(&run_summary_bytes)?),
            "run summary seal mismatch for {}",
            case.case_id
        );
        let compact: Value = serde_json::from_slice(&run_summary_bytes)?;
        ensure!(compact["schema_id"] == "waypoint_v2_flight_summary_v1");
        ensure!(compact["input_identity"] == result.input_identity);
        ensure!(compact["result"]["planning_stop"] == serde_json::to_value(result.planning_stop)?);
        ensure!(compact["result"]["reason"] == serde_json::to_value(&result.reason)?);
        ensure!(compact["result"]["correction_count"] == result.correction_count);
        ensure!(
            compact["result"]["initial_nominal_terrain_blocked"]
                == result.initial_nominal_terrain_blocked
        );
        ensure!(compact["result"]["integrity_passed"] == result.integrity_passed);
        ensure!(
            compact["result"]["final_source_replay_passed"] == result.final_source_replay_passed
        );
        ensure!(
            compact["result"]["physical_outcome"]
                == serde_json::to_value(&result.physical_outcome)?
        );
        ensure!(
            compact["result"]["mission_outcome"] == serde_json::to_value(&result.mission_outcome)?
        );
        ensure!(compact["policy"] == serde_json::to_value(&result.policy)?);
        ensure!(compact["result"]["timings"] == serde_json::to_value(&result.timings)?);
        if result.manifest.is_none() {
            ensure!(
                compact["run_summary"].is_null(),
                "compact run summary claims simulator evidence without a manifest for {}",
                case.case_id
            );
        }
        if let Some(raw_report) = &case.rich_report_path {
            ensure!(raw_report == &format!("runs/{}/report.html", case.case_id));
            let bytes = fs::read(safe_capture_file(root, raw_report)?)?;
            ensure!(
                case.artifact_sha256.get(raw_report) == Some(&sha256_bytes(&bytes)?),
                "raw report seal mismatch for {}",
                case.case_id
            );
            ensure!(result.manifest.is_some() && result.ordinary_flight.is_some());
            validate_full_evidence_consistency(&input, &result, &compact)?;
        } else if case.status == "simulation_unverified" {
            ensure!(result.manifest.is_none() && result.ordinary_flight.is_some());
        } else {
            ensure!(result.manifest.is_none() && result.ordinary_flight.is_none());
            ensure!(case.status == "preflight_rejected");
        }
        ensure!(
            case.annotated_report_path == format!("runs/{}/index.html", case.case_id),
            "annotated report path mismatch for {}",
            case.case_id
        );
        verified.push((input, result));
    }
    ensure!(
        summarize(&report.cases)? == report.summary,
        "batch summary does not match case rows"
    );
    Ok(verified)
}
