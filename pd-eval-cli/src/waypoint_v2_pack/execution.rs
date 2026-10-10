use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, bail, ensure};
use pd_plan::waypoint_v2::{WaypointV2Policy, WaypointV2Stop};
use rayon::prelude::*;
use serde::Serialize;

use super::{
    WAYPOINT_V2_BATCH_SCHEMA_ID,
    input::{
        EXPECTED_CASE_COUNT, FRESH_PATH, MAX_WORKERS, PRACTICAL_PATH, load_and_expand,
        policy_for_version, repository_root,
    },
    model::{
        WaypointV2BatchCase, WaypointV2BatchProvenance, WaypointV2BatchReport,
        WaypointV2BatchSummary, WaypointV2PackGroup, WaypointV2PackInput,
    },
    presentation::render_waypoint_v2_batch,
    provenance::{capture_source_state, input_identity_still_matches},
};
use crate::{
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    evidence_io::{
        reserve_output_root, sha256_bytes, write_bytes_create_only_with_context,
        write_json_create_only,
    },
    preflight_waypoint_v2_flight,
    waypoint_v2_output::write_waypoint_v2_flight,
};

pub(super) fn request_for(
    input: &WaypointV2PackInput,
) -> WaypointDirectNominalDirectGenerationRequest {
    WaypointDirectNominalDirectGenerationRequest {
        probe_id: input.scenario.id.clone(),
        scenario: input.scenario.clone(),
        source_pad_id: input.source_pad_id.clone(),
        target_pad_id: input.target_pad_id.clone(),
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    }
}

pub(super) fn enum_text<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_value(value)?
        .as_str()
        .map(str::to_owned)
        .context("expected a string-serialized enum")
}

fn expected_preflight_matches(
    input: &WaypointV2PackInput,
    preflight_stop: Option<WaypointV2Stop>,
) -> Result<()> {
    match input.expected_preflight.as_deref() {
        Some("unsupported") => ensure!(
            preflight_stop == Some(WaypointV2Stop::Unsupported),
            "{}: expected unsupported preflight, got {:?}",
            input.case_id,
            preflight_stop
        ),
        Some("invalid_input") => ensure!(
            preflight_stop == Some(WaypointV2Stop::InvalidInput),
            "{}: expected invalid-input preflight, got {:?}",
            input.case_id,
            preflight_stop
        ),
        Some(other) => bail!("{}: unsupported expected_preflight {other}", input.case_id),
        None => ensure!(
            preflight_stop.is_none(),
            "{}: unexpected V2 preflight rejection {:?}",
            input.case_id,
            preflight_stop
        ),
    }
    Ok(())
}

pub(super) fn source_path(input: &WaypointV2PackInput) -> &'static str {
    match input.source_set.as_str() {
        "practical_suite" => PRACTICAL_PATH,
        "fresh_terrain_inputs" => FRESH_PATH,
        _ => "",
    }
}

pub(super) fn bounded_worker_count(requested: usize) -> Result<usize> {
    ensure!(requested > 0, "workers must be at least one");
    Ok(requested.min(MAX_WORKERS))
}

fn run_one_case(
    input: &WaypointV2PackInput,
    policy: &WaypointV2Policy,
    capture_root: &Path,
) -> Result<WaypointV2BatchCase> {
    let run_dir = capture_root.join("runs").join(&input.case_id);
    let request = request_for(input);
    let result = write_waypoint_v2_flight(&request, policy, &run_dir)
        .with_context(|| format!("run V2 case {}", input.case_id))?;
    let manifest_present = result.manifest.is_some();
    let flight_started = result.ordinary_flight.is_some();
    let status = if manifest_present {
        "simulated"
    } else if flight_started {
        "simulation_unverified"
    } else {
        "preflight_rejected"
    };
    let report_file = run_dir.join("report.html");
    let rich_report_path = if report_file.is_file() {
        Some(format!("runs/{}/report.html", input.case_id))
    } else {
        None
    };
    ensure!(
        manifest_present == rich_report_path.is_some(),
        "{}: raw report presence disagrees with replay manifest",
        input.case_id
    );
    let mut artifact_sha256 = BTreeMap::new();
    for name in ["scenario.json", "flight.json", "summary.json"] {
        let relative = format!("runs/{}/{name}", input.case_id);
        artifact_sha256.insert(relative, sha256_bytes(&fs::read(run_dir.join(name))?)?);
    }
    if rich_report_path.is_some() {
        let relative = format!("runs/{}/report.html", input.case_id);
        artifact_sha256.insert(relative, sha256_bytes(&fs::read(&report_file)?)?);
    }
    let planning_stop = enum_text(&result.planning_stop)?;
    let physical_outcome = result
        .physical_outcome
        .as_ref()
        .map(enum_text)
        .transpose()?;
    let mission_outcome = result.mission_outcome.as_ref().map(enum_text).transpose()?;
    let outcome = physical_outcome
        .clone()
        .or_else(|| Some(planning_stop.clone()));
    Ok(WaypointV2BatchCase {
        case_id: input.case_id.clone(),
        source_set: input.source_set.clone(),
        source_group: input.source_group.clone(),
        group: input.group.clone(),
        family: input.family.clone(),
        base_case_id: input.base_case_id.clone(),
        source_pad_id: input.source_pad_id.clone(),
        target_pad_id: input.target_pad_id.clone(),
        expected_preflight: input.expected_preflight.clone(),
        status: status.into(),
        outcome,
        planning_stop: Some(planning_stop),
        reason: result.reason.clone(),
        correction_count: Some(result.correction_count),
        initial_nominal_terrain_blocked: Some(result.initial_nominal_terrain_blocked),
        integrity_passed: Some(result.integrity_passed),
        final_source_replay_passed: Some(result.final_source_replay_passed),
        physical_outcome,
        mission_outcome,
        planning_s: Some(result.timings.planning_s),
        execution_s: Some(result.timings.execution_s),
        replay_s: Some(result.timings.replay_s),
        input_path: source_path(input).into(),
        scenario_path: format!("runs/{}/scenario.json", input.case_id),
        flight_path: format!("runs/{}/flight.json", input.case_id),
        summary_path: format!("runs/{}/summary.json", input.case_id),
        rich_report_path,
        annotated_report_path: format!("runs/{}/index.html", input.case_id),
        artifact_sha256,
        error: result
            .planning_stop
            .eq(&WaypointV2Stop::ImplementationError)
            .then(|| result.reason.clone())
            .flatten(),
    })
}

pub(crate) fn summarize(cases: &[WaypointV2BatchCase]) -> Result<WaypointV2BatchSummary> {
    let mut summary = WaypointV2BatchSummary::default();
    for case in cases {
        match case.group {
            WaypointV2PackGroup::Clear => summary.clear_count += 1,
            WaypointV2PackGroup::Ordinary => summary.ordinary_count += 1,
            WaypointV2PackGroup::AdditionalTerrain => summary.additional_terrain_count += 1,
            WaypointV2PackGroup::Diagnostic => summary.diagnostic_count += 1,
        }
        if let Some(stop) = &case.planning_stop {
            *summary.planning_stops.entry(stop.clone()).or_default() += 1;
            if stop == "unsupported" {
                summary.unsupported_count += 1;
            }
        }
        if case.initial_nominal_terrain_blocked == Some(true) {
            summary.initially_blocked_count += 1;
        }
        if case.integrity_passed == Some(true) {
            summary.integrity_passed_count += 1;
        } else if case.integrity_passed == Some(false) {
            summary.integrity_failed_count += 1;
        }
        if case.physical_outcome == Some("crashed".into()) {
            summary.crash_count += 1;
        }
        if let Some(outcome) = &case.physical_outcome {
            *summary
                .physical_outcomes
                .entry(outcome.clone())
                .or_default() += 1;
        }
        if let Some(outcome) = &case.mission_outcome {
            *summary.mission_outcomes.entry(outcome.clone()).or_default() += 1;
        }
        let simulated = matches!(case.status.as_str(), "simulated" | "simulation_unverified");
        if case.status == "simulation_unverified" {
            summary.simulation_unverified_count += 1;
        }
        if simulated && case.final_source_replay_passed == Some(true) {
            summary.final_source_replay_passed_count += 1;
        } else if simulated && case.final_source_replay_passed == Some(false) {
            summary.final_source_replay_failed_count += 1;
        }
        let valid_landing = case.physical_outcome.as_deref() == Some("landed_on_target")
            && case.mission_outcome.as_deref() == Some("success")
            && case.planning_stop.as_deref() == Some("landed")
            && case.integrity_passed == Some(true)
            && case.final_source_replay_passed == Some(true);
        let diagnostic = case.group == WaypointV2PackGroup::Diagnostic;
        if valid_landing && diagnostic {
            summary.diagnostic_landing_count += 1;
        } else if valid_landing {
            summary.valid_landing_count += 1;
            if case.correction_count == Some(0) {
                summary.direct_landing_count += 1;
            } else {
                summary.corrected_landing_count += 1;
            }
        } else if simulated && diagnostic {
            summary.diagnostic_non_landing_count += 1;
        } else if simulated {
            summary.non_landing_count += 1;
        }
    }
    ensure!(
        summary.clear_count == 11
            && summary.ordinary_count == 16
            && summary.additional_terrain_count == 9
            && summary.diagnostic_count == 8,
        "native V2 pack group denominator changed"
    );
    ensure!(
        summary.clear_count + summary.ordinary_count + summary.additional_terrain_count == 36,
        "native V2 landing denominator is not 36"
    );
    Ok(summary)
}

pub fn run_waypoint_v2_pack(
    pack_path: &Path,
    capture_root: &Path,
    workers: usize,
) -> Result<WaypointV2BatchReport> {
    let effective_workers = bounded_worker_count(workers)?;
    let expanded = load_and_expand(pack_path)?;
    let policy = policy_for_version(expanded.definition.policy_version)?;
    policy
        .validate_for_execution()
        .map_err(anyhow::Error::msg)?;
    // Complete admission checks before reserving any output or beginning a flight.
    for input in &expanded.inputs {
        let preflight = preflight_waypoint_v2_flight(&request_for(input), &policy);
        expected_preflight_matches(input, preflight.rejection)?;
    }
    ensure!(
        !capture_root.exists() && fs::symlink_metadata(capture_root).is_err(),
        "capture root already exists: {}",
        capture_root.display()
    );
    let source_before = capture_source_state(&repository_root())?;
    reserve_output_root(capture_root)?;
    let runs = capture_root.join("runs");
    fs::create_dir(&runs)?;
    write_bytes_create_only_with_context(
        &capture_root.join("pack.json"),
        &expanded.pack_bytes,
        "create-only artifact",
    )?;
    let expanded_inputs_bytes = serde_json::to_vec_pretty(&expanded.inputs)?;
    write_bytes_create_only_with_context(
        &capture_root.join("expanded-inputs.json"),
        &expanded_inputs_bytes,
        "create-only artifact",
    )?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(effective_workers)
        .build()
        .context("build bounded V2 evaluator worker pool")?;
    let outcomes = pool.install(|| {
        expanded
            .inputs
            .par_iter()
            .map(|input| run_one_case(input, &policy, capture_root))
            .collect::<Vec<_>>()
    });
    let mut cases = Vec::with_capacity(outcomes.len());
    let mut first_error = None;
    for outcome in outcomes {
        match outcome {
            Ok(case) => cases.push(case),
            Err(error) if first_error.is_none() => first_error = Some(error),
            Err(_) => {}
        }
    }
    if let Some(error) = first_error {
        return Err(error.context(format!(
            "V2 batch incomplete; partial capture preserved at {}",
            capture_root.display()
        )));
    }
    ensure!(
        cases.len() == EXPECTED_CASE_COUNT,
        "V2 capture did not record all cases"
    );
    let source_after = capture_source_state(&repository_root())?;
    let inputs_unchanged = input_identity_still_matches(&repository_root(), pack_path, &expanded)?;
    let summary = summarize(&cases)?;
    let report = WaypointV2BatchReport {
        schema_id: WAYPOINT_V2_BATCH_SCHEMA_ID.into(),
        pack_id: expanded.definition.id.clone(),
        name: expanded.definition.name.clone(),
        status: "completed".into(),
        policy_version: expanded.definition.policy_version,
        case_count: cases.len(),
        cases,
        summary,
        input_identity: expanded.input_identity,
        pack_snapshot_sha256: sha256_bytes(&expanded.pack_bytes)?,
        expanded_inputs_snapshot_sha256: sha256_bytes(&expanded_inputs_bytes)?,
        pack_snapshot_path: "pack.json".into(),
        expanded_inputs_snapshot_path: "expanded-inputs.json".into(),
        provenance: WaypointV2BatchProvenance {
            unchanged_during_capture: Some(source_before == source_after && inputs_unchanged),
            source_before,
            source_after: Some(source_after),
        },
    };
    write_json_create_only(&capture_root.join("summary.json"), &report)?;
    render_waypoint_v2_batch(capture_root)
}
