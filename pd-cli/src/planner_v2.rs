use std::{fs, path::PathBuf, time::Instant};

use anyhow::{Context, Result, ensure};
use clap::Parser;
use pd_core::{MissionOutcome, PhysicalOutcome, ScenarioSpec};
use pd_eval::{
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    WaypointV2CliProgressEntryV1, WaypointV2CliProgressV1, WaypointV2FlightResult,
    WaypointV2Policy, WaypointV2Session, WaypointV2SessionProgress, WaypointV2Stop,
    nominal_direct_flight_identity, preflight_waypoint_v2_flight, replay_waypoint_v2_cli_bundle,
    reserve_waypoint_v2_flight_output, write_waypoint_v2_cli_bundle,
};
use serde::Serialize;

const FLIGHT_SCHEMA_ID: &str = "planner_v2_cli_flight_v1";

#[derive(Debug, Parser)]
pub(super) struct WaypointV2FlightArgs {
    #[arg(value_name = "SCENARIO_JSON")]
    scenario: PathBuf,
    #[arg(long, value_name = "ID")]
    source_pad: String,
    #[arg(long, value_name = "ID")]
    target_pad: String,
    #[arg(
        long,
        value_name = "DIR",
        required_unless_present = "preflight_only",
        conflicts_with = "preflight_only"
    )]
    output_dir: Option<PathBuf>,
    #[arg(long, conflicts_with = "output_dir")]
    preflight_only: bool,
}

#[derive(Debug, Parser)]
pub(super) struct WaypointV2ReplayArgs {
    #[arg(long, value_name = "DIR")]
    bundle_dir: PathBuf,
}

#[derive(Serialize)]
struct FlightOutput<'a> {
    schema_id: &'static str,
    status: &'static str,
    supported: bool,
    input_identity: String,
    policy: &'a WaypointV2Policy,
    planning_stop: Option<WaypointV2Stop>,
    reason: Option<String>,
    correction_count: u32,
    integrity_passed: Option<bool>,
    final_source_replay_passed: Option<bool>,
    physical_outcome: Option<PhysicalOutcome>,
    mission_outcome: Option<MissionOutcome>,
    output_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub(super) fn flight(args: WaypointV2FlightArgs) -> Result<()> {
    let scenario: ScenarioSpec = serde_json::from_slice(
        &fs::read(&args.scenario)
            .with_context(|| format!("read V2 scenario {}", args.scenario.display()))?,
    )
    .with_context(|| format!("parse V2 scenario {}", args.scenario.display()))?;
    let request = WaypointDirectNominalDirectGenerationRequest {
        probe_id: scenario.id.clone(),
        scenario,
        source_pad_id: args.source_pad,
        target_pad_id: args.target_pad,
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    };
    let policy = WaypointV2Policy::revision_3();

    if args.preflight_only {
        let preflight = preflight_waypoint_v2_flight(&request, &policy);
        let input_identity = nominal_direct_flight_identity(&(&request, &policy))?;
        let output = FlightOutput {
            schema_id: FLIGHT_SCHEMA_ID,
            status: "preflight_only",
            supported: preflight.supported,
            input_identity,
            policy: &policy,
            planning_stop: preflight.rejection,
            reason: preflight.reason,
            correction_count: 0,
            integrity_passed: None,
            final_source_replay_passed: None,
            physical_outcome: None,
            mission_outcome: None,
            output_dir: None,
            error: None,
        };
        print_json(&output)?;
        ensure!(
            preflight.supported,
            "V2 preflight rejected the request; no simulator was created"
        );
        return Ok(());
    }

    // The fresh root is acquired before session construction can allocate a
    // live simulator, and certainly before the first direct/correction piece.
    let output_dir = args
        .output_dir
        .as_deref()
        .context("--output-dir is required unless --preflight-only is selected")?;
    reserve_waypoint_v2_flight_output(output_dir)?;
    let total_started = Instant::now();
    let mut session = WaypointV2Session::start(request.clone(), policy.clone())?;
    let mut entries = Vec::new();
    loop {
        let step_started = Instant::now();
        let progress = session.advance_piece()?;
        let terminal = matches!(progress, WaypointV2SessionProgress::Terminal { .. });
        entries.push(WaypointV2CliProgressEntryV1 {
            elapsed_s: step_started.elapsed().as_secs_f64(),
            progress,
        });
        if terminal {
            break;
        }
    }
    let finalization_started = Instant::now();
    let finalized_result = session.finish()?;
    let finalization_elapsed_s = finalization_started.elapsed().as_secs_f64();
    let result = finalized_result.clone();
    if let Some(last) = entries.last_mut()
        && let WaypointV2SessionProgress::Terminal { planning_stop, .. } = &mut last.progress
    {
        *planning_stop = result.planning_stop;
    }
    let progress = WaypointV2CliProgressV1 {
        schema_id: pd_eval::waypoint_v2_bundle::WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID.into(),
        entries,
        finalization_elapsed_s,
    };
    if let Err(error) =
        write_waypoint_v2_cli_bundle(&request, &result, output_dir, &progress, total_started)
    {
        let failure = bundle_failed_output(&result, output_dir.to_path_buf(), &error);
        print_json(&failure)?;
        return Err(
            error.context("V2 flight bundle writing failed; inspect any retained artifacts")
        );
    }

    let output = result_output(&result, Some(output_dir.to_path_buf()));
    print_json(&output)?;
    ensure!(
        flight_succeeded(&result),
        "V2 flight retained its bundle but did not satisfy the landing success gate"
    );
    Ok(())
}

pub(super) fn replay(args: WaypointV2ReplayArgs) -> Result<()> {
    let replay = replay_waypoint_v2_cli_bundle(&args.bundle_dir)?;
    print_json(&replay)?;
    ensure!(
        replay.replay_passed,
        "V2 bundle is valid but unsupported or did not establish a successful saved-evidence replay"
    );
    Ok(())
}

fn result_output(result: &WaypointV2FlightResult, output_dir: Option<PathBuf>) -> FlightOutput<'_> {
    FlightOutput {
        schema_id: FLIGHT_SCHEMA_ID,
        status: "completed",
        supported: result.absolute_deadline_physics_step.is_some(),
        input_identity: result.input_identity.clone(),
        policy: &result.policy,
        planning_stop: Some(result.planning_stop),
        reason: result.reason.clone(),
        correction_count: result.correction_count,
        integrity_passed: Some(result.integrity_passed),
        final_source_replay_passed: Some(result.final_source_replay_passed),
        physical_outcome: result.physical_outcome.clone(),
        mission_outcome: result.mission_outcome.clone(),
        output_dir,
        error: None,
    }
}

fn bundle_failed_output<'a>(
    result: &'a WaypointV2FlightResult,
    output_dir: PathBuf,
    error: &anyhow::Error,
) -> FlightOutput<'a> {
    let mut output = result_output(result, Some(output_dir));
    output.status = "bundle_failed";
    output.error = Some(format!("{error:#}"));
    output
}

fn flight_succeeded(result: &WaypointV2FlightResult) -> bool {
    result.planning_stop == WaypointV2Stop::Landed
        && result.physical_outcome == Some(PhysicalOutcome::LandedOnTarget)
        && result.mission_outcome == Some(MissionOutcome::Success)
        && result.integrity_passed
        && result.final_source_replay_passed
}

fn print_json(value: &impl Serialize) -> Result<()> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn flight_parser_requires_explicit_pads_and_output_root() {
        let parsed = crate::Cli::try_parse_from([
            "pd-cli",
            "waypoint-v2-flight",
            "scenario.json",
            "--source-pad",
            "source",
            "--target-pad",
            "target",
            "--output-dir",
            "out",
            "--preflight-only",
        ]);
        assert!(
            parsed.is_err(),
            "preflight-only must not need an output path"
        );
        let parsed = crate::Cli::try_parse_from([
            "pd-cli",
            "waypoint-v2-flight",
            "scenario.json",
            "--source-pad",
            "source",
            "--target-pad",
            "target",
            "--preflight-only",
        ]);
        assert!(parsed.is_ok());
        assert!(
            crate::Cli::try_parse_from(["pd-cli", "waypoint-v2-flight", "scenario.json"]).is_err()
        );
    }

    #[test]
    fn replay_parser_uses_a_read_only_bundle_directory() {
        let parsed =
            crate::Cli::try_parse_from(["pd-cli", "waypoint-v2-replay", "--bundle-dir", "capture"]);
        assert!(parsed.is_ok());
        assert!(crate::Cli::try_parse_from(["pd-cli", "waypoint-v2-replay"]).is_err());
    }

    #[test]
    fn strict_success_gate_requires_all_flight_claims() {
        let result = WaypointV2FlightResult {
            policy: WaypointV2Policy::revision_3(),
            input_identity: String::new(),
            planning_stop: WaypointV2Stop::Landed,
            reason: None,
            correction_count: 0,
            initial_nominal_terrain_blocked: false,
            integrity_passed: true,
            physical_outcome: Some(PhysicalOutcome::LandedOnTarget),
            mission_outcome: Some(MissionOutcome::Success),
            absolute_deadline_physics_step: Some(100),
            cycles: Vec::new(),
            segments: Vec::new(),
            ordinary_flight: None,
            final_source_replay_passed: true,
            manifest: None,
            failed_local_row: None,
            timings: Default::default(),
        };
        assert!(flight_succeeded(&result));
        let mutations: [fn(&mut WaypointV2FlightResult); 7] = [
            |r| r.planning_stop = WaypointV2Stop::NoClearing,
            |r| r.physical_outcome = Some(PhysicalOutcome::LandedOffTarget),
            |r| r.physical_outcome = None,
            |r| r.mission_outcome = Some(MissionOutcome::InProgress),
            |r| r.mission_outcome = None,
            |r| r.integrity_passed = false,
            |r| r.final_source_replay_passed = false,
        ];
        for mutate in mutations {
            let mut incomplete = result.clone();
            mutate(&mut incomplete);
            assert!(!flight_succeeded(&incomplete));
        }
    }

    #[test]
    fn bundle_failure_status_preserves_flight_outcomes_without_changing_normal_json() {
        let result = WaypointV2FlightResult {
            policy: WaypointV2Policy::revision_3(),
            input_identity: "input-identity".into(),
            planning_stop: WaypointV2Stop::NoClearing,
            reason: Some("local search exhausted".into()),
            correction_count: 1,
            initial_nominal_terrain_blocked: true,
            integrity_passed: true,
            physical_outcome: Some(PhysicalOutcome::Flying),
            mission_outcome: Some(MissionOutcome::InProgress),
            absolute_deadline_physics_step: Some(100),
            cycles: Vec::new(),
            segments: Vec::new(),
            ordinary_flight: None,
            final_source_replay_passed: false,
            manifest: None,
            failed_local_row: None,
            timings: Default::default(),
        };

        let normal = serde_json::to_value(result_output(&result, None)).unwrap();
        assert_eq!(normal["status"], "completed");
        assert!(normal.get("error").is_none());

        let error = anyhow::anyhow!("V2 progress handoff differs from actual H");
        let failure = bundle_failed_output(&result, PathBuf::from("capture"), &error);
        let failed_json = serde_json::to_value(failure).unwrap();
        assert_eq!(failed_json["schema_id"], FLIGHT_SCHEMA_ID);
        assert_eq!(failed_json["status"], "bundle_failed");
        assert_eq!(failed_json["planning_stop"], "no_clearing");
        assert_eq!(failed_json["physical_outcome"], "flying");
        assert_eq!(failed_json["mission_outcome"], "in_progress");
        assert_eq!(failed_json["reason"], "local search exhausted");
        assert!(failed_json["error"].as_str().unwrap().contains("actual H"));
        assert_eq!(failed_json["output_dir"], "capture");
    }
}
