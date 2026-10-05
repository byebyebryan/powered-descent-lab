//! Portable create-only bundles and source-bounded replay for the opt-in V2 CLI.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result, ensure};
use pd_core::{EndReason, MissionOutcome, PhysicalOutcome, RunContext};
use pd_plan::waypoint_v2::original_deadline;
use serde::{Deserialize, Serialize};

use crate::{
    WaypointDirectNominalDirectGenerationRequest, WaypointV2FlightResult, WaypointV2Policy,
    WaypointV2SessionProgress,
    evidence_io::{sha256_bytes, write_json_create_only},
    nominal_direct_flight::nominal_direct_flight_identity,
    waypoint_v2::{preflight_waypoint_v2_flight, replay_saved_waypoint_v2_evidence},
    waypoint_v2_output::{waypoint_v2_rich_report_eligible, write_waypoint_v2_flight_result},
};

pub const WAYPOINT_V2_CLI_BUNDLE_SCHEMA_ID: &str = "planner_v2_cli_bundle_v1";
pub const WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID: &str = "planner_v2_cli_progress_v1";
pub const WAYPOINT_V2_CLI_REPLAY_SCHEMA_ID: &str = "planner_v2_cli_replay_v1";

const REQUIRED_ARTIFACTS: [&str; 4] = [
    "scenario.json",
    "flight.json",
    "summary.json",
    "progress.json",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2CliBundleReceiptV1 {
    pub schema_id: String,
    pub request: WaypointDirectNominalDirectGenerationRequest,
    pub policy: WaypointV2Policy,
    pub input_identity: String,
    pub artifact_sha256: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2CliProgressEntryV1 {
    pub elapsed_s: f64,
    pub progress: WaypointV2SessionProgress,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2CliProgressV1 {
    pub schema_id: String,
    pub entries: Vec<WaypointV2CliProgressEntryV1>,
    pub finalization_elapsed_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2CliReplayV1 {
    pub schema_id: String,
    pub replay_passed: bool,
    pub input_identity: String,
    pub planning_stop: crate::WaypointV2Stop,
    pub physical_outcome: Option<PhysicalOutcome>,
    pub mission_outcome: Option<MissionOutcome>,
    pub physics_step: Option<u64>,
    pub sim_time_s: Option<f64>,
    pub comparison_sha256: Option<BTreeMap<String, String>>,
}

/// Write the shared rich capture files, the session trace, and a receipt that
/// binds the complete request and exact artifact bytes. `output_dir` must have
/// been reserved before the session executes any live piece.
pub fn write_waypoint_v2_cli_bundle(
    request: &WaypointDirectNominalDirectGenerationRequest,
    result: &WaypointV2FlightResult,
    output_dir: &Path,
    progress: &WaypointV2CliProgressV1,
    total_started: std::time::Instant,
) -> Result<WaypointV2CliBundleReceiptV1> {
    ensure!(
        result.policy == WaypointV2Policy::revision_3(),
        "V2 CLI bundles are sealed to planner policy revision 3"
    );
    ensure!(
        result.input_identity == nominal_direct_flight_identity(&(request, &result.policy))?,
        "V2 result identity differs from its CLI request"
    );

    write_waypoint_v2_flight_result(request, result, output_dir, total_started)?;
    write_json_create_only(&output_dir.join("progress.json"), progress)?;

    // Keep a diagnosable flight, summary and progress trace even when the
    // additive metadata validator rejects the trace. The receipt remains the
    // final create-only artifact and is written only after validation.
    ensure!(
        progress.schema_id == WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID,
        "unsupported V2 CLI progress schema"
    );
    validate_progress(progress, result)?;

    let mut artifact_sha256 = BTreeMap::new();
    for name in REQUIRED_ARTIFACTS {
        artifact_sha256.insert(name.to_owned(), hash_regular_artifact(output_dir, name)?);
    }
    if waypoint_v2_rich_report_eligible(result) {
        artifact_sha256.insert(
            "report.html".into(),
            hash_regular_artifact(output_dir, "report.html")?,
        );
    }
    let receipt = WaypointV2CliBundleReceiptV1 {
        schema_id: WAYPOINT_V2_CLI_BUNDLE_SCHEMA_ID.into(),
        request: request.clone(),
        policy: result.policy.clone(),
        input_identity: result.input_identity.clone(),
        artifact_sha256,
    };
    write_json_create_only(&output_dir.join("bundle.json"), &receipt)?;
    Ok(receipt)
}

/// Verify a saved bundle, then replay its recorded source from the original
/// scenario. This is read-only and never reconstructs a live plant from a
/// saved snapshot or proposes/continues a planner trajectory.
pub fn replay_waypoint_v2_cli_bundle(bundle_dir: &Path) -> Result<WaypointV2CliReplayV1> {
    let root_meta = fs::symlink_metadata(bundle_dir)
        .with_context(|| format!("inspect V2 bundle root {}", bundle_dir.display()))?;
    ensure!(
        root_meta.is_dir() && !root_meta.file_type().is_symlink(),
        "V2 bundle root must be a real directory"
    );
    let receipt: WaypointV2CliBundleReceiptV1 = read_json(&bundle_dir.join("bundle.json"))?;
    ensure!(
        receipt.schema_id == WAYPOINT_V2_CLI_BUNDLE_SCHEMA_ID,
        "unsupported V2 CLI bundle schema"
    );
    ensure!(
        receipt.policy == WaypointV2Policy::revision_3(),
        "V2 CLI bundle policy is not planner revision 3"
    );
    receipt.policy.validate().map_err(anyhow::Error::msg)?;
    ensure!(
        receipt.input_identity
            == nominal_direct_flight_identity(&(&receipt.request, &receipt.policy))?,
        "V2 CLI receipt identity does not bind its request and policy"
    );

    let result: WaypointV2FlightResult = read_json(&bundle_dir.join("flight.json"))?;
    ensure!(
        result.policy == receipt.policy && result.input_identity == receipt.input_identity,
        "saved V2 flight policy or identity differs from its receipt"
    );
    let expected_artifacts = expected_artifact_names(&result);
    ensure!(
        receipt
            .artifact_sha256
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            == expected_artifacts,
        "V2 CLI receipt has an unexpected or incomplete artifact allowlist"
    );
    for (name, expected_hash) in &receipt.artifact_sha256 {
        ensure!(
            hash_regular_artifact(bundle_dir, name)? == *expected_hash,
            "V2 CLI artifact hash mismatch: {name}"
        );
    }

    let scenario_value: serde_json::Value = read_json(&bundle_dir.join("scenario.json"))?;
    ensure!(
        scenario_value == serde_json::to_value(&receipt.request.scenario)?,
        "saved scenario does not match the complete request"
    );
    let flight_value = serde_json::to_value(&result)?;
    let stored_flight_value: serde_json::Value = read_json(&bundle_dir.join("flight.json"))?;
    ensure!(
        stored_flight_value == flight_value,
        "saved flight artifact could not be parsed canonically"
    );
    let summary: serde_json::Value = read_json(&bundle_dir.join("summary.json"))?;
    validate_compact_summary(&summary, &result)?;
    let progress: WaypointV2CliProgressV1 = read_json(&bundle_dir.join("progress.json"))?;
    ensure!(
        progress.schema_id == WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID,
        "unsupported V2 CLI progress schema"
    );
    validate_progress(&progress, &result)?;

    let preflight = preflight_waypoint_v2_flight(&receipt.request, &receipt.policy);
    if !preflight.supported {
        ensure!(
            result.planning_stop
                == preflight
                    .rejection
                    .unwrap_or(crate::WaypointV2Stop::Unsupported)
                && result.manifest.is_none()
                && result.ordinary_flight.is_none()
                && result.physical_outcome.is_none()
                && result.mission_outcome.is_none()
                && !result.final_source_replay_passed,
            "unsupported V2 bundle contains physical or replay claims"
        );
        return Ok(WaypointV2CliReplayV1 {
            schema_id: WAYPOINT_V2_CLI_REPLAY_SCHEMA_ID.into(),
            replay_passed: false,
            input_identity: receipt.input_identity,
            planning_stop: result.planning_stop,
            physical_outcome: None,
            mission_outcome: None,
            physics_step: None,
            sim_time_s: None,
            comparison_sha256: None,
        });
    }

    validate_supported_saved_result(&result, &receipt.request)?;
    let proof = replay_saved_waypoint_v2_evidence(&receipt.request, &result)?;
    let manifest = &proof.run.manifest;
    if let Some(saved) = &result.manifest {
        ensure!(
            saved == manifest,
            "saved V2 manifest differs from recomputed original-source replay"
        );
    }
    if result.final_source_replay_passed {
        ensure!(
            result.manifest.as_ref() == Some(manifest),
            "saved V2 result claims final source replay without its matching manifest"
        );
    }
    ensure!(
        result.physical_outcome.as_ref() == Some(&manifest.physical_outcome)
            && result.mission_outcome.as_ref() == Some(&manifest.mission_outcome),
        "saved V2 outcomes differ from recomputed source replay"
    );
    Ok(WaypointV2CliReplayV1 {
        schema_id: WAYPOINT_V2_CLI_REPLAY_SCHEMA_ID.into(),
        replay_passed: true,
        input_identity: receipt.input_identity,
        planning_stop: result.planning_stop,
        physical_outcome: result.physical_outcome,
        mission_outcome: result.mission_outcome,
        physics_step: Some(proof.final_state.physics_step),
        sim_time_s: Some(proof.final_state.sim_time_s),
        comparison_sha256: Some(proof_comparison_sha256(&proof)?),
    })
}

fn expected_artifact_names(result: &WaypointV2FlightResult) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = REQUIRED_ARTIFACTS
        .iter()
        .map(|name| (*name).into())
        .collect();
    if waypoint_v2_rich_report_eligible(result) {
        names.insert("report.html".into());
    }
    names
}

fn validate_progress(
    progress: &WaypointV2CliProgressV1,
    result: &WaypointV2FlightResult,
) -> Result<()> {
    ensure!(
        progress
            .entries
            .iter()
            .all(|entry| entry.elapsed_s.is_finite() && entry.elapsed_s >= 0.0)
            && progress.finalization_elapsed_s.is_finite()
            && progress.finalization_elapsed_s >= 0.0,
        "V2 progress timings must be finite and nonnegative"
    );
    let mut expected_piece = 0usize;
    let mut previous_handoff = 0u64;
    let mut terminal_count = 0usize;
    for (index, entry) in progress.entries.iter().enumerate() {
        match &entry.progress {
            WaypointV2SessionProgress::Handoff {
                piece_index,
                correction_count,
                entry_physics_step,
                handoff_physics_step,
            } => {
                ensure!(
                    terminal_count == 0
                        && *piece_index == expected_piece
                        && *correction_count as usize == expected_piece + 1
                        && *entry_physics_step == previous_handoff
                        && *handoff_physics_step >= *entry_physics_step,
                    "V2 progress handoff ordering or clocks are inconsistent"
                );
                let cycle = result
                    .cycles
                    .get(*piece_index)
                    .with_context(|| format!("progress references missing cycle {piece_index}"))?;
                ensure!(
                    cycle.cycle_index == *piece_index
                        && cycle.current_state.physics_step == *entry_physics_step
                        && cycle.decision == crate::WaypointV2CycleDecision::LocalCleared,
                    "V2 progress handoff entry differs from its cycle origin"
                );
                let correction_segment = result
                    .segments
                    .iter()
                    .filter(|segment| segment.kind == crate::WaypointV2SegmentKind::LocalCorrection)
                    .nth(*piece_index)
                    .context("V2 progress handoff has no executed correction segment")?;
                ensure!(
                    correction_segment.start_physics_step
                        == correction_segment.entry_state.physics_step
                        && correction_segment.end_physics_step
                            == correction_segment.end_state.physics_step
                        && correction_segment.start_physics_step >= *entry_physics_step
                        && correction_segment.start_physics_step < *handoff_physics_step
                        && correction_segment.end_physics_step == *handoff_physics_step,
                    "V2 progress handoff differs from its executed actual H segment"
                );
                let selected = cycle
                    .local_search
                    .as_ref()
                    .context("V2 progress handoff cycle is missing its local search")?
                    .selected
                    .as_ref()
                    .context("V2 progress handoff cycle is missing its selected proposal")?;
                let selected_handoff_updates = selected
                    .schedule
                    .updates
                    .iter()
                    .take_while(|update| update.physics_step < *handoff_physics_step)
                    .cloned()
                    .collect::<Vec<_>>();
                ensure!(
                    selected.schedule.entry_physics_step == correction_segment.start_physics_step
                        && selected.entry_state.physics_step
                            == correction_segment.start_physics_step
                        && selected.entry_state == correction_segment.entry_state
                        && selected.schedule.handoff_physics_step == *handoff_physics_step
                        && selected.handoff_state.physics_step == *handoff_physics_step
                        && selected.handoff_state == correction_segment.end_state
                        && selected.identity == correction_segment.proposal_identity
                        && correction_segment.updates == selected_handoff_updates,
                    "V2 progress correction segment differs from its selected E/H proposal"
                );
                expected_piece += 1;
                previous_handoff = *handoff_physics_step;
            }
            WaypointV2SessionProgress::Terminal {
                piece_index,
                entry_physics_step,
                planning_stop,
                correction_count,
                physics_step,
            } => {
                terminal_count += 1;
                ensure!(
                    index + 1 == progress.entries.len()
                        && terminal_count == 1
                        && *planning_stop == result.planning_stop
                        && *correction_count == result.correction_count,
                    "V2 progress terminal state differs from the flight result"
                );
                if let Some(ordinary) = &result.ordinary_flight {
                    ensure!(
                        *physics_step == Some(ordinary.final_state.physics_step),
                        "V2 terminal progress endpoint differs from saved ordinary evidence"
                    );
                    if let Some(piece_index) = piece_index {
                        let cycle = result
                            .cycles
                            .get(*piece_index)
                            .context("V2 terminal progress references a missing planner cycle")?;
                        ensure!(
                            cycle.cycle_index == *piece_index
                                && cycle.current_state.physics_step == previous_handoff
                                && *entry_physics_step == Some(cycle.current_state.physics_step),
                            "V2 terminal progress entry differs from its cycle origin"
                        );
                    } else {
                        ensure!(
                            *entry_physics_step == *physics_step,
                            "deadline before piece start must retain its unchanged clock"
                        );
                    }
                } else {
                    ensure!(
                        piece_index.is_none()
                            && entry_physics_step.is_none()
                            && physics_step.is_none(),
                        "unsupported V2 progress must not claim simulator clocks"
                    );
                }
            }
        }
    }
    ensure!(
        terminal_count == 1,
        "V2 progress must end with exactly one terminal entry"
    );
    ensure!(
        expected_piece == result.correction_count as usize,
        "V2 progress handoff count differs from the flight correction ledger"
    );
    Ok(())
}

fn validate_supported_saved_result(
    result: &WaypointV2FlightResult,
    request: &WaypointDirectNominalDirectGenerationRequest,
) -> Result<()> {
    let ordinary = result
        .ordinary_flight
        .as_ref()
        .context("supported V2 bundle is missing ordinary flight evidence")?;
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let expected_deadline = original_deadline(
        context.sim.max_time_s,
        request.policy.analytical_policy.mission_budget_s(),
    )
    .map_err(anyhow::Error::msg)?;
    let deadline = result
        .absolute_deadline_physics_step
        .context("supported V2 bundle is missing its original deadline")?;
    ensure!(
        deadline == expected_deadline
            && result.physical_outcome.as_ref() == Some(&ordinary.final_state.physical_outcome)
            && result.mission_outcome.as_ref() == Some(&ordinary.final_state.mission_outcome),
        "supported V2 result deadline or outcomes differ from its saved endpoint"
    );
    let correction_segments = result
        .segments
        .iter()
        .filter(|segment| segment.kind == crate::WaypointV2SegmentKind::LocalCorrection)
        .count();
    let corrected_cycles = result
        .cycles
        .iter()
        .filter(|cycle| cycle.decision == crate::WaypointV2CycleDecision::LocalCleared)
        .count();
    ensure!(
        correction_segments == result.correction_count as usize
            && corrected_cycles == result.correction_count as usize,
        "V2 correction count differs from its executed segment or cycle ledger"
    );
    ensure!(
        !result.final_source_replay_passed || result.manifest.is_some(),
        "V2 result claims final replay without its recomputed manifest"
    );
    if let Some(manifest) = &result.manifest {
        ensure!(
            manifest.physics_steps == ordinary.final_state.physics_step
                && manifest.sim_time_s == ordinary.final_state.sim_time_s
                && manifest.physical_outcome == ordinary.final_state.physical_outcome
                && manifest.mission_outcome == ordinary.final_state.mission_outcome,
            "saved V2 manifest differs from ordinary evidence endpoint"
        );
    }
    match result.planning_stop {
        crate::WaypointV2Stop::Landed => ensure!(
            ordinary.final_state.physical_outcome == PhysicalOutcome::LandedOnTarget
                && ordinary.final_state.mission_outcome == MissionOutcome::Success
                && ordinary.final_state.end_reason == EndReason::TouchdownOnTarget
                && result.integrity_passed
                && result.final_source_replay_passed
                && result.manifest.is_some(),
            "V2 Landed stop lacks a verified on-target mission-success endpoint"
        ),
        crate::WaypointV2Stop::NoNominal
        | crate::WaypointV2Stop::NominalRejected
        | crate::WaypointV2Stop::NoClearing
        | crate::WaypointV2Stop::CorrectionLimit
        | crate::WaypointV2Stop::Deadline
        | crate::WaypointV2Stop::NoProgress => {
            let finite_running = ordinary.final_state.physical_outcome == PhysicalOutcome::Flying
                && ordinary.final_state.mission_outcome == MissionOutcome::InProgress
                && ordinary.final_state.end_reason == EndReason::Running;
            let typed_deadline = result.planning_stop == crate::WaypointV2Stop::Deadline
                && ((finite_running
                    && (ordinary.final_state.physics_step >= deadline
                        || ordinary.final_state.fuel_kg <= 0.0))
                    || (ordinary.final_state.physical_outcome == PhysicalOutcome::TimedOut
                        && ordinary.final_state.mission_outcome == MissionOutcome::FailedTimeout
                        && ordinary.final_state.end_reason == EndReason::MaxTimeReached));
            ensure!(
                if result.planning_stop == crate::WaypointV2Stop::Deadline {
                    typed_deadline
                } else {
                    finite_running
                },
                "finite V2 planner stop is inconsistent with its physical and mission endpoint"
            );
        }
        crate::WaypointV2Stop::Unsupported | crate::WaypointV2Stop::InvalidInput => {
            anyhow::bail!("supported V2 evidence cannot use a preflight rejection stop")
        }
        crate::WaypointV2Stop::ImplementationError => {
            // Keep a safe finite prefix available for diagnosis even when a
            // later planner/replay stage failed; endpoint fields above remain
            // cross-bound to the retained ordinary evidence.
        }
    }
    Ok(())
}

fn validate_compact_summary(
    summary: &serde_json::Value,
    result: &WaypointV2FlightResult,
) -> Result<()> {
    let expected_identity = serde_json::to_value(&result.input_identity)?;
    ensure!(
        summary.get("schema_id").and_then(|value| value.as_str())
            == Some("waypoint_v2_flight_summary_v1")
            && summary.get("input_identity") == Some(&expected_identity)
            && summary.get("policy") == Some(&serde_json::to_value(&result.policy)?),
        "V2 compact summary schema, identity, or policy differs from flight"
    );
    let expected_result = serde_json::json!({
        "planning_stop": result.planning_stop,
        "reason": result.reason,
        "correction_count": result.correction_count,
        "initial_nominal_terrain_blocked": result.initial_nominal_terrain_blocked,
        "integrity_passed": result.integrity_passed,
        "physical_outcome": result.physical_outcome,
        "mission_outcome": result.mission_outcome,
        "final_source_replay_passed": result.final_source_replay_passed,
        "timings": result.timings,
    });
    ensure!(
        summary.get("result") == Some(&expected_result),
        "V2 compact result fields differ from full flight"
    );
    let expected_run_summary = result.manifest.as_ref().map(|manifest| {
        serde_json::json!({
            "minimum_clearance": {
                "touchdown_m": manifest.summary.min_touchdown_clearance_m,
                "hull_m": manifest.summary.min_hull_clearance_m,
                "landing": manifest.summary.landing,
            },
            "fuel": {
                "remaining_kg": manifest.summary.fuel_remaining_kg,
                "used_kg": manifest.summary.fuel_used_kg,
            },
            "endpoint": {
                "physics_step": manifest.physics_steps,
                "sim_time_s": manifest.sim_time_s,
                "physical_outcome": manifest.physical_outcome,
                "mission_outcome": manifest.mission_outcome,
                "end_reason": manifest.end_reason,
            }
        })
    });
    ensure!(
        summary.get("run_summary")
            == Some(&expected_run_summary.unwrap_or(serde_json::Value::Null)),
        "V2 compact run summary differs from saved manifest"
    );
    let timings = summary
        .get("timings")
        .context("V2 compact summary is missing timings")?;
    for key in ["output_s", "total_s"] {
        ensure!(
            timings
                .get(key)
                .and_then(|value| value.as_f64())
                .is_some_and(|value| value.is_finite() && value >= 0.0),
            "V2 compact summary timing {key} must be finite and nonnegative"
        );
    }
    Ok(())
}

fn hash_regular_artifact(root: &Path, name: &str) -> Result<String> {
    ensure!(
        REQUIRED_ARTIFACTS.contains(&name) || name == "report.html",
        "artifact name is outside the V2 CLI allowlist: {name}"
    );
    let path = root.join(name);
    let metadata = fs::symlink_metadata(&path)
        .with_context(|| format!("inspect V2 CLI artifact {}", path.display()))?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "V2 CLI artifact must be a regular file: {name}"
    );
    sha256_bytes(&fs::read(&path)?)
}

fn proof_comparison_sha256(
    proof: &pd_core::BoundedRunArtifactsV1,
) -> Result<BTreeMap<String, String>> {
    let components = [
        ("manifest", serde_json::to_vec(&proof.run.manifest)?),
        ("actions", serde_json::to_vec(&proof.run.actions)?),
        ("events", serde_json::to_vec(&proof.run.events)?),
        ("samples", serde_json::to_vec(&proof.run.samples)?),
        ("final_state", serde_json::to_vec(&proof.final_state)?),
        (
            "incoming_contact",
            serde_json::to_vec(&proof.incoming_contact)?,
        ),
    ];
    components
        .into_iter()
        .map(|(key, bytes)| Ok((key.into(), sha256_bytes(&bytes)?)))
        .collect()
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspect V2 bundle file {}", path.display()))?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "V2 bundle artifact must be a regular file: {}",
        path.display()
    );
    serde_json::from_slice(&fs::read(path)?)
        .with_context(|| format!("parse V2 bundle artifact {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use pd_core::{
        BoundedRunArtifactsV1, BoundedRunLimitsV1, BoundedRunStopCauseV1, Command, EndReason,
        RunArtifacts, RunManifest, RunSummary, ScenarioSpec, Vec2,
    };

    fn fresh_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "pd-v2-bundle-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn unsupported_request() -> WaypointDirectNominalDirectGenerationRequest {
        let scenario: ScenarioSpec = serde_json::from_str(include_str!(
            "../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .expect("tracked scenario fixture");
        WaypointDirectNominalDirectGenerationRequest {
            probe_id: scenario.id.clone(),
            scenario,
            source_pad_id: "pad_main".into(),
            target_pad_id: "missing_pad".into(),
            policy: crate::WaypointDirectNominalDirectGenerationPolicyV1::default(),
        }
    }

    fn supported_request() -> WaypointDirectNominalDirectGenerationRequest {
        crate::test_inputs::planner_request("v2_clear_845")
    }

    fn create_unsupported_bundle(root: &Path) -> WaypointV2CliBundleReceiptV1 {
        let request = unsupported_request();
        let policy = WaypointV2Policy::revision_3();
        let mut session = crate::WaypointV2Session::start(request.clone(), policy).unwrap();
        let started = std::time::Instant::now();
        let terminal = session.advance_piece().unwrap();
        let finalization_started = std::time::Instant::now();
        let result = session.finish().unwrap().clone();
        let progress = WaypointV2CliProgressV1 {
            schema_id: WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID.into(),
            entries: vec![WaypointV2CliProgressEntryV1 {
                elapsed_s: started.elapsed().as_secs_f64(),
                progress: terminal,
            }],
            finalization_elapsed_s: finalization_started.elapsed().as_secs_f64(),
        };
        crate::waypoint_v2_output::reserve_waypoint_v2_flight_output(root).unwrap();
        write_waypoint_v2_cli_bundle(&request, &result, root, &progress, started).unwrap()
    }

    fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
        fs::read_dir(root)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read(entry.path()).unwrap(),
                )
            })
            .collect()
    }

    fn distinct_origin_handoff_fixture() -> (WaypointV2FlightResult, WaypointV2CliProgressV1) {
        let request = supported_request();
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let deadline = original_deadline(
            context.sim.max_time_s,
            request.policy.analytical_policy.mission_budget_s(),
        )
        .unwrap();
        let mut ordinary = crate::local_clearing::new_ordinary(&context)
            .unwrap()
            .evidence;
        let origin = ordinary.final_state.clone();
        let snapshot_at = |step: u64| {
            let mut state = origin.clone();
            state.physics_step = step;
            state.sim_time_s = step as f64 / f64::from(request.policy.physics_hz);
            state.position_m.x += step as f64;
            state
        };
        let entry_state = snapshot_at(100);
        let powered_end_state = snapshot_at(120);
        let handoff_state = snapshot_at(200);
        let continuation_end_state = snapshot_at(400);
        ordinary.final_state = handoff_state.clone();

        let local_policy = pd_plan::local_clearing::LocalClearingPolicyV1::default();
        let template = local_policy.templates().unwrap().remove(0);
        let proposal = crate::local_clearing::LocalClearingProposalV1 {
            policy: local_policy,
            context_identity: "synthetic-context".into(),
            goal: pd_plan::local_clearing::LocalClearingGoalV1 {
                first_conflict_physics_step: 150,
                first_conflict_position_m: Vec2::default(),
                absolute_deadline_physics_step: deadline,
            },
            row_id: "synthetic-row".into(),
            template,
            entry_state: entry_state.clone(),
            powered_end_state,
            handoff_state: handoff_state.clone(),
            continuation_end_state: continuation_end_state.clone(),
            minimum_progress_x_m: 0.0,
            actual_fuel_burn_to_handoff_kg: 0.0,
            schedule: pd_plan::local_clearing::LocalClearingScheduleV1 {
                entry_physics_step: 100,
                powered_end_physics_step: 120,
                handoff_physics_step: 200,
                continuation_end_physics_step: 400,
                absolute_deadline_physics_step: deadline,
                updates: Vec::new(),
            },
            trajectory: Vec::new(),
            identity: "synthetic-proposal".into(),
        };
        let local_search = crate::waypoint_v2::WaypointV2LocalSearch {
            entries: Vec::new(),
            row_count: 1,
            boundary_count: 1,
            accepted_row_count: 1,
            row_status_counts: BTreeMap::new(),
            row_stop_reason_counts: BTreeMap::new(),
            boundary_status_counts: BTreeMap::new(),
            selected: Some(proposal),
            certificate_state: Some(continuation_end_state),
            handoff_source_replay_passed: true,
            certificate_source_replay_passed: true,
        };
        let cycle = crate::WaypointV2Cycle {
            cycle_index: 0,
            current_state: origin,
            nominal_search_identity: "synthetic-search".into(),
            nominal_proposal_identity: Some("synthetic-nominal".into()),
            nominal_peak_com_height_m: None,
            nominal_attempt_status_counts: BTreeMap::new(),
            nominal_rejection_reason_counts: BTreeMap::new(),
            nominal_updates: Vec::new(),
            audit: None,
            fixed_consumed_prefix_proven: true,
            decision: crate::WaypointV2CycleDecision::LocalCleared,
            conflict_state: None,
            conflict_incoming_contact: None,
            local_search: Some(local_search),
        };
        let terminal_cycle = crate::WaypointV2Cycle {
            cycle_index: 1,
            current_state: handoff_state.clone(),
            nominal_search_identity: "synthetic-terminal-search".into(),
            nominal_proposal_identity: None,
            nominal_peak_com_height_m: None,
            nominal_attempt_status_counts: BTreeMap::new(),
            nominal_rejection_reason_counts: BTreeMap::new(),
            nominal_updates: Vec::new(),
            audit: None,
            fixed_consumed_prefix_proven: false,
            decision: crate::WaypointV2CycleDecision::NoNominal,
            conflict_state: None,
            conflict_incoming_contact: None,
            local_search: None,
        };
        let segment = crate::WaypointV2Segment {
            kind: crate::WaypointV2SegmentKind::LocalCorrection,
            start_physics_step: 100,
            end_physics_step: 200,
            proposal_identity: "synthetic-proposal".into(),
            updates: Vec::new(),
            entry_state,
            end_state: handoff_state,
        };
        let result = WaypointV2FlightResult {
            policy: WaypointV2Policy::revision_3(),
            input_identity: String::new(),
            planning_stop: crate::WaypointV2Stop::NoNominal,
            reason: Some("synthetic corrected finite stop".into()),
            correction_count: 1,
            initial_nominal_terrain_blocked: true,
            integrity_passed: true,
            physical_outcome: Some(PhysicalOutcome::Flying),
            mission_outcome: Some(MissionOutcome::InProgress),
            absolute_deadline_physics_step: Some(deadline),
            cycles: vec![cycle, terminal_cycle],
            segments: vec![segment],
            ordinary_flight: Some(ordinary),
            final_source_replay_passed: true,
            manifest: None,
            failed_local_row: None,
            timings: Default::default(),
        };
        let progress = WaypointV2CliProgressV1 {
            schema_id: WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID.into(),
            entries: vec![
                WaypointV2CliProgressEntryV1 {
                    elapsed_s: 0.0,
                    progress: WaypointV2SessionProgress::Handoff {
                        piece_index: 0,
                        correction_count: 1,
                        entry_physics_step: 0,
                        handoff_physics_step: 200,
                    },
                },
                WaypointV2CliProgressEntryV1 {
                    elapsed_s: 0.0,
                    progress: WaypointV2SessionProgress::Terminal {
                        piece_index: Some(1),
                        entry_physics_step: Some(200),
                        planning_stop: crate::WaypointV2Stop::NoNominal,
                        correction_count: 1,
                        physics_step: Some(200),
                    },
                },
            ],
            finalization_elapsed_s: 0.0,
        };
        (result, progress)
    }

    #[test]
    fn progress_trace_requires_terminal_entry_and_monotonic_piece_origins() {
        let result = WaypointV2FlightResult {
            policy: WaypointV2Policy::revision_3(),
            input_identity: String::new(),
            planning_stop: crate::WaypointV2Stop::Unsupported,
            reason: Some("test only".into()),
            correction_count: 0,
            initial_nominal_terrain_blocked: false,
            integrity_passed: true,
            physical_outcome: None,
            mission_outcome: None,
            absolute_deadline_physics_step: None,
            cycles: Vec::new(),
            segments: Vec::new(),
            ordinary_flight: None,
            final_source_replay_passed: false,
            manifest: None,
            failed_local_row: None,
            timings: Default::default(),
        };
        let progress = WaypointV2CliProgressV1 {
            schema_id: WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID.into(),
            entries: vec![WaypointV2CliProgressEntryV1 {
                elapsed_s: 0.0,
                progress: WaypointV2SessionProgress::Terminal {
                    piece_index: None,
                    entry_physics_step: None,
                    planning_stop: crate::WaypointV2Stop::Unsupported,
                    correction_count: 0,
                    physics_step: None,
                },
            }],
            finalization_elapsed_s: 0.0,
        };
        validate_progress(&progress, &result).unwrap();
        let mut missing_terminal = progress.clone();
        missing_terminal.entries.clear();
        assert!(validate_progress(&missing_terminal, &result).is_err());
        let mut bad_terminal = progress;
        if let WaypointV2SessionProgress::Terminal { physics_step, .. } =
            &mut bad_terminal.entries[0].progress
        {
            *physics_step = Some(0);
        }
        assert!(validate_progress(&bad_terminal, &result).is_err());
    }

    #[test]
    fn corrected_progress_binds_piece_origin_entry_handoff_and_selected_proposal() {
        let (result, progress) = distinct_origin_handoff_fixture();
        let segment = result
            .segments
            .iter()
            .find(|segment| segment.kind == crate::WaypointV2SegmentKind::LocalCorrection)
            .unwrap();
        assert_eq!(result.cycles[0].current_state.physics_step, 0);
        assert_eq!(segment.entry_state.physics_step, 100);
        assert_eq!(segment.end_state.physics_step, 200);
        validate_progress(&progress, &result).unwrap();

        let mut bad_handoff = progress.clone();
        if let WaypointV2SessionProgress::Handoff {
            handoff_physics_step,
            ..
        } = &mut bad_handoff.entries[0].progress
        {
            *handoff_physics_step += 1;
        }
        assert!(validate_progress(&bad_handoff, &result).is_err());

        let mut bad_origin = progress.clone();
        if let WaypointV2SessionProgress::Handoff {
            entry_physics_step, ..
        } = &mut bad_origin.entries[0].progress
        {
            *entry_physics_step += 1;
        }
        assert!(validate_progress(&bad_origin, &result).is_err());

        let mut bad_entry = result.clone();
        let selected = bad_entry.cycles[0]
            .local_search
            .as_mut()
            .unwrap()
            .selected
            .as_mut()
            .unwrap();
        selected.entry_state.position_m.x += 1.0;
        assert!(validate_progress(&progress, &bad_entry).is_err());

        let mut missing_local_search = result.clone();
        missing_local_search.cycles[0].local_search = None;
        assert!(validate_progress(&progress, &missing_local_search).is_err());

        let mut missing_selected = result.clone();
        missing_selected.cycles[0]
            .local_search
            .as_mut()
            .unwrap()
            .selected = None;
        assert!(validate_progress(&progress, &missing_selected).is_err());

        let mut bad_proposal = result;
        bad_proposal.segments[0].proposal_identity = "different-proposal".into();
        assert!(validate_progress(&progress, &bad_proposal).is_err());
    }

    #[test]
    fn invalid_output_progress_retains_raw_evidence_without_receipt() {
        let root = fresh_root("invalid-output-progress");
        let request = unsupported_request();
        let policy = WaypointV2Policy::revision_3();
        let mut session = crate::WaypointV2Session::start(request.clone(), policy).unwrap();
        let _ = session.advance_piece().unwrap();
        let result = session.finish().unwrap().clone();
        crate::waypoint_v2_output::reserve_waypoint_v2_flight_output(&root).unwrap();

        let invalid_progress = WaypointV2CliProgressV1 {
            schema_id: WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID.into(),
            entries: Vec::new(),
            finalization_elapsed_s: 0.0,
        };
        assert!(
            write_waypoint_v2_cli_bundle(
                &request,
                &result,
                &root,
                &invalid_progress,
                std::time::Instant::now(),
            )
            .is_err()
        );

        let names = files(&root).keys().cloned().collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "flight.json",
                "progress.json",
                "scenario.json",
                "summary.json"
            ]
        );
        assert!(!root.join("bundle.json").exists());
        let saved_result: WaypointV2FlightResult = read_json(&root.join("flight.json")).unwrap();
        assert_eq!(
            saved_result.planning_stop,
            crate::WaypointV2Stop::InvalidInput
        );
        let saved_progress: WaypointV2CliProgressV1 =
            read_json(&root.join("progress.json")).unwrap();
        assert!(saved_progress.entries.is_empty());
        assert!(root.join("summary.json").is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn saved_preflight_bundle_replays_read_only_and_detects_tampering_before_physics() {
        let root = fresh_root("read-only");
        let _receipt = create_unsupported_bundle(&root);
        let before = files(&root);
        let replay = replay_waypoint_v2_cli_bundle(&root).unwrap();
        assert!(!replay.replay_passed);
        assert_eq!(replay.physical_outcome, None);
        assert_eq!(replay.mission_outcome, None);
        assert_eq!(replay.physics_step, None);
        assert_eq!(replay.sim_time_s, None);
        assert_eq!(replay.comparison_sha256, None);
        assert_eq!(
            files(&root),
            before,
            "saved replay must not write artifacts"
        );

        let mut scenario = fs::read(root.join("scenario.json")).unwrap();
        scenario.push(b' ');
        fs::write(root.join("scenario.json"), scenario).unwrap();
        assert!(replay_waypoint_v2_cli_bundle(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn integrity_failed_post_h_partial_result_retains_bundle_without_rich_report() {
        let root = fresh_root("partial-post-h");
        let request = supported_request();
        let policy = WaypointV2Policy::revision_3();
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let deadline = original_deadline(
            context.sim.max_time_s,
            request.policy.analytical_policy.mission_budget_s(),
        )
        .unwrap();
        let ordinary = crate::local_clearing::new_ordinary(&context)
            .unwrap()
            .evidence;
        let mut result = WaypointV2FlightResult {
            policy: policy.clone(),
            input_identity: nominal_direct_flight_identity(&(&request, &policy)).unwrap(),
            planning_stop: crate::WaypointV2Stop::NoNominal,
            reason: Some("synthetic source-replayed partial result".into()),
            correction_count: 0,
            initial_nominal_terrain_blocked: false,
            integrity_passed: true,
            physical_outcome: Some(PhysicalOutcome::Flying),
            mission_outcome: Some(MissionOutcome::InProgress),
            absolute_deadline_physics_step: Some(deadline),
            cycles: Vec::new(),
            segments: Vec::new(),
            ordinary_flight: Some(ordinary),
            final_source_replay_passed: false,
            manifest: None,
            failed_local_row: None,
            timings: Default::default(),
        };
        let proof = replay_saved_waypoint_v2_evidence(&request, &result).unwrap();
        result.manifest = Some(proof.run.manifest);
        result.final_source_replay_passed = true;
        result.planning_stop = crate::WaypointV2Stop::ImplementationError;
        result.integrity_passed = false;
        let endpoint = result.ordinary_flight.as_ref().unwrap().final_state.clone();
        // This is the narrow post-H window: a local segment was executed, but
        // the proof checkpoint failed before correction_count could advance.
        result.segments.push(crate::WaypointV2Segment {
            kind: crate::WaypointV2SegmentKind::LocalCorrection,
            start_physics_step: endpoint.physics_step,
            end_physics_step: endpoint.physics_step,
            proposal_identity: "injected-post-h-proof-failure".into(),
            updates: Vec::new(),
            entry_state: endpoint.clone(),
            end_state: endpoint.clone(),
        });

        crate::waypoint_v2_output::reserve_waypoint_v2_flight_output(&root).unwrap();
        let progress = WaypointV2CliProgressV1 {
            schema_id: WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID.into(),
            entries: vec![WaypointV2CliProgressEntryV1 {
                elapsed_s: 0.0,
                progress: WaypointV2SessionProgress::Terminal {
                    piece_index: None,
                    entry_physics_step: Some(endpoint.physics_step),
                    planning_stop: crate::WaypointV2Stop::ImplementationError,
                    correction_count: 0,
                    physics_step: Some(endpoint.physics_step),
                },
            }],
            finalization_elapsed_s: 0.0,
        };
        let receipt = write_waypoint_v2_cli_bundle(
            &request,
            &result,
            &root,
            &progress,
            std::time::Instant::now(),
        )
        .unwrap();

        assert!(receipt.artifact_sha256.contains_key("flight.json"));
        assert!(receipt.artifact_sha256.contains_key("summary.json"));
        assert!(receipt.artifact_sha256.contains_key("progress.json"));
        assert!(!receipt.artifact_sha256.contains_key("report.html"));
        assert!(root.join("scenario.json").is_file());
        assert!(root.join("flight.json").is_file());
        assert!(root.join("summary.json").is_file());
        assert!(root.join("progress.json").is_file());
        assert!(root.join("bundle.json").is_file());
        assert!(!root.join("report.html").exists());
        let saved_progress: WaypointV2CliProgressV1 =
            read_json(&root.join("progress.json")).unwrap();
        assert_eq!(
            saved_progress.entries.len(),
            1,
            "no unproved H handoff is invented"
        );
        assert!(matches!(
            saved_progress.entries[0].progress,
            WaypointV2SessionProgress::Terminal {
                planning_stop: crate::WaypointV2Stop::ImplementationError,
                correction_count: 0,
                ..
            }
        ));
        let saved_summary: serde_json::Value = read_json(&root.join("summary.json")).unwrap();
        assert_eq!(
            saved_summary["result"]["planning_stop"],
            "implementation_error"
        );
        assert_eq!(saved_summary["result"]["integrity_passed"], false);
        assert_eq!(saved_summary["result"]["final_source_replay_passed"], true);
        assert!(saved_summary["run_summary"].is_object());
        let before_replay = files(&root);
        let error = replay_waypoint_v2_cli_bundle(&root).unwrap_err();
        assert!(format!("{error:#}").contains("correction"));
        assert_eq!(files(&root), before_replay, "rejection is read-only");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn replay_digest_map_hashes_actual_bounded_proof_components() {
        let final_state = pd_core::SimulationStateSnapshotV1 {
            sim_time_s: 0.0,
            physics_step: 0,
            position_m: Vec2::default(),
            velocity_mps: Vec2::default(),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
            fuel_kg: 1.0,
            held_command: Command::idle(),
            physical_outcome: PhysicalOutcome::Flying,
            mission_outcome: MissionOutcome::InProgress,
            end_reason: EndReason::Running,
            min_touchdown_clearance_m: 0.0,
            min_hull_clearance_m: 0.0,
            max_speed_mps: 0.0,
            max_abs_attitude_rad: 0.0,
            max_abs_angular_rate_radps: 0.0,
            waypoint_sequence_passed: 0,
            waypoint_sequence_first_failure_index: None,
            waypoint_handoff_window_index: None,
        };
        let manifest = RunManifest {
            schema_version: 1,
            scenario_id: "digest_test".into(),
            scenario_name: "digest test".into(),
            scenario_seed: 1,
            scenario_tags: vec![],
            controller_id: "waypoint_v2_supplied_commands".into(),
            physics_hz: 120,
            controller_hz: 60,
            sim_time_s: 0.0,
            physics_steps: 0,
            controller_updates: 0,
            physical_outcome: PhysicalOutcome::Flying,
            mission_outcome: MissionOutcome::InProgress,
            end_reason: EndReason::Running,
            summary: RunSummary::default(),
        };
        let proof = BoundedRunArtifactsV1 {
            schema_version: 1,
            limits: BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 0,
                hard_end_physics_step: 0,
            },
            stop: BoundedRunStopCauseV1::CoverageExhausted,
            coverage_reached: true,
            hard_end_reached: false,
            final_state: final_state.clone(),
            incoming_contact: None,
            failure: None,
            run: RunArtifacts {
                manifest: manifest.clone(),
                actions: vec![],
                events: vec![],
                samples: vec![],
            },
        };
        let digests = proof_comparison_sha256(&proof).unwrap();
        assert_eq!(
            digests["manifest"],
            sha256_bytes(&serde_json::to_vec(&proof.run.manifest).unwrap()).unwrap()
        );
        assert_eq!(
            digests["actions"],
            sha256_bytes(&serde_json::to_vec(&proof.run.actions).unwrap()).unwrap()
        );
        assert_eq!(
            digests["events"],
            sha256_bytes(&serde_json::to_vec(&proof.run.events).unwrap()).unwrap()
        );
        assert_eq!(
            digests["samples"],
            sha256_bytes(&serde_json::to_vec(&proof.run.samples).unwrap()).unwrap()
        );
        assert_eq!(
            digests["final_state"],
            sha256_bytes(&serde_json::to_vec(&proof.final_state).unwrap()).unwrap()
        );
        assert_eq!(
            digests["incoming_contact"],
            sha256_bytes(&serde_json::to_vec(&proof.incoming_contact).unwrap()).unwrap()
        );
    }

    #[test]
    fn saved_landed_stop_cannot_be_resealed_over_a_flying_prefix() {
        let request = supported_request();
        let policy = WaypointV2Policy::revision_3();
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let deadline = original_deadline(
            context.sim.max_time_s,
            request.policy.analytical_policy.mission_budget_s(),
        )
        .unwrap();
        let ordinary = crate::local_clearing::new_ordinary(&context)
            .unwrap()
            .evidence;
        let base = WaypointV2FlightResult {
            policy: policy.clone(),
            input_identity: nominal_direct_flight_identity(&(&request, &policy)).unwrap(),
            planning_stop: crate::WaypointV2Stop::NoNominal,
            reason: Some("synthetic flying prefix".into()),
            correction_count: 0,
            initial_nominal_terrain_blocked: false,
            integrity_passed: true,
            physical_outcome: Some(PhysicalOutcome::Flying),
            mission_outcome: Some(MissionOutcome::InProgress),
            absolute_deadline_physics_step: Some(deadline),
            cycles: vec![],
            segments: vec![],
            ordinary_flight: Some(ordinary),
            final_source_replay_passed: false,
            manifest: None,
            failed_local_row: None,
            timings: Default::default(),
        };
        assert!(validate_supported_saved_result(&base, &request).is_ok());
        let mut forged_landed = base.clone();
        forged_landed.planning_stop = crate::WaypointV2Stop::Landed;
        forged_landed.physical_outcome = Some(PhysicalOutcome::LandedOnTarget);
        forged_landed.mission_outcome = Some(MissionOutcome::Success);
        forged_landed.final_source_replay_passed = true;
        forged_landed.manifest = Some(RunManifest {
            schema_version: 1,
            scenario_id: request.scenario.id.clone(),
            scenario_name: request.scenario.name.clone(),
            scenario_seed: request.scenario.seed,
            scenario_tags: request.scenario.tags.clone(),
            controller_id: "waypoint_v2_supplied_commands".into(),
            physics_hz: request.scenario.sim.physics_hz,
            controller_hz: request.scenario.sim.controller_hz,
            sim_time_s: 0.0,
            physics_steps: 0,
            controller_updates: 0,
            physical_outcome: PhysicalOutcome::LandedOnTarget,
            mission_outcome: MissionOutcome::Success,
            end_reason: EndReason::TouchdownOnTarget,
            summary: RunSummary::default(),
        });
        assert!(validate_supported_saved_result(&forged_landed, &request).is_err());

        let mut wrong_correction_count = base;
        wrong_correction_count.correction_count = 1;
        assert!(validate_supported_saved_result(&wrong_correction_count, &request).is_err());
    }

    #[test]
    fn zero_command_finite_stop_has_a_valid_portable_source_replay() {
        let root = fresh_root("zero-command");
        let request = supported_request();
        let policy = WaypointV2Policy::revision_3();
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let deadline = original_deadline(
            context.sim.max_time_s,
            request.policy.analytical_policy.mission_budget_s(),
        )
        .unwrap();
        let ordinary = crate::local_clearing::new_ordinary(&context)
            .unwrap()
            .evidence;
        let cycle = crate::WaypointV2Cycle {
            cycle_index: 0,
            current_state: ordinary.final_state.clone(),
            nominal_search_identity: "synthetic-zero-command-finite-stop".into(),
            nominal_proposal_identity: None,
            nominal_peak_com_height_m: None,
            nominal_attempt_status_counts: BTreeMap::new(),
            nominal_rejection_reason_counts: BTreeMap::new(),
            nominal_updates: Vec::new(),
            audit: None,
            fixed_consumed_prefix_proven: false,
            decision: crate::WaypointV2CycleDecision::NoNominal,
            conflict_state: None,
            conflict_incoming_contact: None,
            local_search: None,
        };
        let mut result = WaypointV2FlightResult {
            policy: policy.clone(),
            input_identity: nominal_direct_flight_identity(&(&request, &policy)).unwrap(),
            planning_stop: crate::WaypointV2Stop::NoNominal,
            reason: Some("synthetic finite stop before departure".into()),
            correction_count: 0,
            initial_nominal_terrain_blocked: false,
            integrity_passed: true,
            physical_outcome: Some(PhysicalOutcome::Flying),
            mission_outcome: Some(MissionOutcome::InProgress),
            absolute_deadline_physics_step: Some(deadline),
            cycles: vec![cycle],
            segments: Vec::new(),
            ordinary_flight: Some(ordinary),
            final_source_replay_passed: false,
            manifest: None,
            failed_local_row: None,
            timings: Default::default(),
        };
        let proof = replay_saved_waypoint_v2_evidence(&request, &result).unwrap();
        result.manifest = Some(proof.run.manifest);
        result.final_source_replay_passed = true;
        validate_supported_saved_result(&result, &request).unwrap();
        crate::waypoint_v2_output::reserve_waypoint_v2_flight_output(&root).unwrap();
        let progress = WaypointV2CliProgressV1 {
            schema_id: WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID.into(),
            entries: vec![WaypointV2CliProgressEntryV1 {
                elapsed_s: 0.0,
                progress: WaypointV2SessionProgress::Terminal {
                    piece_index: Some(0),
                    entry_physics_step: Some(0),
                    planning_stop: crate::WaypointV2Stop::NoNominal,
                    correction_count: 0,
                    physics_step: Some(0),
                },
            }],
            finalization_elapsed_s: 0.0,
        };
        write_waypoint_v2_cli_bundle(
            &request,
            &result,
            &root,
            &progress,
            std::time::Instant::now(),
        )
        .unwrap();
        let saved = replay_waypoint_v2_cli_bundle(&root).unwrap();
        assert!(saved.replay_passed);
        assert_eq!(saved.planning_stop, crate::WaypointV2Stop::NoNominal);
        assert_eq!(saved.physical_outcome, Some(PhysicalOutcome::Flying));
        assert_eq!(saved.mission_outcome, Some(MissionOutcome::InProgress));
        assert_eq!(saved.physics_step, Some(0));
        assert!(saved.comparison_sha256.is_some());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deadline_after_a_handoff_retains_bundle_without_fabricating_a_new_piece() {
        let root = fresh_root("deadline-after-handoff");
        let request = supported_request();
        let policy = WaypointV2Policy::revision_3();
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let deadline = original_deadline(
            context.sim.max_time_s,
            request.policy.analytical_policy.mission_budget_s(),
        )
        .unwrap();
        let mut ordinary = crate::local_clearing::new_ordinary(&context)
            .unwrap()
            .evidence;
        let piece_origin = ordinary.final_state.clone();
        let snapshot_at = |step: u64| {
            let mut state = piece_origin.clone();
            state.physics_step = step;
            state.sim_time_s = step as f64 / f64::from(request.policy.physics_hz);
            state.position_m.x += step as f64;
            state
        };
        let entry_step = 60;
        let powered_end_step = 120;
        let handoff_step = 180;
        let continuation_end_step = handoff_step + 240;
        let entry_state = snapshot_at(entry_step);
        let powered_end_state = snapshot_at(powered_end_step);
        let mut handoff_state = snapshot_at(handoff_step);
        handoff_state.fuel_kg = 0.0;
        let mut continuation_end_state = snapshot_at(continuation_end_step);
        continuation_end_state.fuel_kg = 0.0;
        ordinary.final_state = handoff_state.clone();

        let local_policy = pd_plan::local_clearing::LocalClearingPolicyV1::default();
        let template = local_policy.templates().unwrap().remove(0);
        let schedule_updates = (entry_step..continuation_end_step)
            .step_by(2)
            .map(|physics_step| {
                let powered = physics_step < powered_end_step;
                pd_core::FlightProgramUpdateV1 {
                    physics_step,
                    phase: if powered {
                        "local_powered".into()
                    } else {
                        "local_coast".into()
                    },
                    command: if powered {
                        Command {
                            throttle_frac: 0.5,
                            target_attitude_rad: template.target_attitude_rad,
                        }
                    } else {
                        Command::idle()
                    },
                }
            })
            .collect::<Vec<_>>();
        let selected_handoff_updates = schedule_updates
            .iter()
            .take_while(|update| update.physics_step < handoff_step)
            .cloned()
            .collect::<Vec<_>>();
        let schedule = pd_plan::local_clearing::LocalClearingScheduleV1 {
            entry_physics_step: entry_step,
            powered_end_physics_step: powered_end_step,
            handoff_physics_step: handoff_step,
            continuation_end_physics_step: continuation_end_step,
            absolute_deadline_physics_step: deadline,
            updates: schedule_updates,
        };
        schedule.validate(&template).unwrap();
        let selected = crate::local_clearing::LocalClearingProposalV1 {
            policy: local_policy,
            context_identity: "synthetic-deadline-context".into(),
            goal: pd_plan::local_clearing::LocalClearingGoalV1 {
                first_conflict_physics_step: 100,
                first_conflict_position_m: Vec2::default(),
                absolute_deadline_physics_step: deadline,
            },
            row_id: "synthetic-deadline-row".into(),
            template,
            entry_state: entry_state.clone(),
            powered_end_state,
            handoff_state: handoff_state.clone(),
            continuation_end_state: continuation_end_state.clone(),
            minimum_progress_x_m: 0.0,
            actual_fuel_burn_to_handoff_kg: entry_state.fuel_kg - handoff_state.fuel_kg,
            schedule,
            trajectory: Vec::new(),
            identity: "synthetic-handoff".into(),
        };
        let local_search = crate::waypoint_v2::WaypointV2LocalSearch {
            entries: Vec::new(),
            row_count: 1,
            boundary_count: 1,
            accepted_row_count: 1,
            row_status_counts: BTreeMap::new(),
            row_stop_reason_counts: BTreeMap::new(),
            boundary_status_counts: BTreeMap::new(),
            selected: Some(selected),
            certificate_state: Some(continuation_end_state),
            handoff_source_replay_passed: true,
            certificate_source_replay_passed: true,
        };
        let cycle = crate::WaypointV2Cycle {
            cycle_index: 0,
            current_state: piece_origin.clone(),
            nominal_search_identity: "synthetic-deadline-after-handoff".into(),
            nominal_proposal_identity: None,
            nominal_peak_com_height_m: None,
            nominal_attempt_status_counts: BTreeMap::new(),
            nominal_rejection_reason_counts: BTreeMap::new(),
            nominal_updates: Vec::new(),
            audit: None,
            fixed_consumed_prefix_proven: false,
            decision: crate::WaypointV2CycleDecision::LocalCleared,
            conflict_state: None,
            conflict_incoming_contact: None,
            local_search: Some(local_search),
        };
        let segment = crate::WaypointV2Segment {
            kind: crate::WaypointV2SegmentKind::LocalCorrection,
            start_physics_step: entry_step,
            end_physics_step: handoff_step,
            proposal_identity: "synthetic-handoff".into(),
            updates: selected_handoff_updates,
            entry_state,
            end_state: handoff_state,
        };
        let result = WaypointV2FlightResult {
            policy: policy.clone(),
            input_identity: nominal_direct_flight_identity(&(&request, &policy)).unwrap(),
            planning_stop: crate::WaypointV2Stop::Deadline,
            reason: Some("fuel deadline immediately after handoff".into()),
            correction_count: 1,
            initial_nominal_terrain_blocked: true,
            integrity_passed: true,
            physical_outcome: Some(PhysicalOutcome::Flying),
            mission_outcome: Some(MissionOutcome::InProgress),
            absolute_deadline_physics_step: Some(deadline),
            cycles: vec![cycle],
            segments: vec![segment],
            ordinary_flight: Some(ordinary),
            final_source_replay_passed: false,
            manifest: None,
            failed_local_row: None,
            timings: Default::default(),
        };
        let progress = WaypointV2CliProgressV1 {
            schema_id: WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID.into(),
            entries: vec![
                WaypointV2CliProgressEntryV1 {
                    elapsed_s: 0.0,
                    progress: WaypointV2SessionProgress::Handoff {
                        piece_index: 0,
                        correction_count: 1,
                        entry_physics_step: 0,
                        handoff_physics_step: handoff_step,
                    },
                },
                WaypointV2CliProgressEntryV1 {
                    elapsed_s: 0.0,
                    progress: WaypointV2SessionProgress::Terminal {
                        piece_index: None,
                        entry_physics_step: Some(handoff_step),
                        planning_stop: crate::WaypointV2Stop::Deadline,
                        correction_count: 1,
                        physics_step: Some(handoff_step),
                    },
                },
            ],
            finalization_elapsed_s: 0.0,
        };
        validate_progress(&progress, &result).unwrap();
        crate::waypoint_v2_output::reserve_waypoint_v2_flight_output(&root).unwrap();
        write_waypoint_v2_cli_bundle(
            &request,
            &result,
            &root,
            &progress,
            std::time::Instant::now(),
        )
        .unwrap();
        assert!(root.join("bundle.json").is_file());
        let saved: WaypointV2CliProgressV1 = read_json(&root.join("progress.json")).unwrap();
        match &saved.entries.last().unwrap().progress {
            WaypointV2SessionProgress::Terminal {
                piece_index,
                entry_physics_step,
                physics_step,
                planning_stop,
                ..
            } => {
                assert_eq!(*piece_index, None);
                assert_eq!(*entry_physics_step, Some(handoff_step));
                assert_eq!(*physics_step, Some(handoff_step));
                assert_eq!(*planning_stop, crate::WaypointV2Stop::Deadline);
            }
            WaypointV2SessionProgress::Handoff { .. } => {
                panic!("saved trace did not end in its deadline terminal")
            }
        }
        fs::remove_dir_all(root).unwrap();
    }
}
