//! Create-only artifacts for the opt-in V2 flight mode.

use std::{path::Path, time::Instant};

use anyhow::Result;
use pd_plan::waypoint_v2::WaypointV2Policy;
use serde::Serialize;

use crate::{
    WaypointDirectNominalDirectGenerationRequest, WaypointV2FlightResult,
    nominal_direct_flight::{reserve_output_root, write_create_only},
    run_waypoint_v2_flight,
};

const SUMMARY_SCHEMA_ID: &str = "waypoint_v2_flight_summary_v1";

#[derive(Serialize)]
struct CompactResult<'a> {
    planning_stop: &'a pd_plan::waypoint_v2::WaypointV2Stop,
    reason: &'a Option<String>,
    correction_count: u32,
    initial_nominal_terrain_blocked: bool,
    integrity_passed: bool,
    physical_outcome: &'a Option<pd_core::PhysicalOutcome>,
    mission_outcome: &'a Option<pd_core::MissionOutcome>,
    final_source_replay_passed: bool,
    timings: &'a crate::WaypointV2Timings,
}

#[derive(Serialize)]
struct OutputTimings {
    output_s: f64,
    total_s: f64,
}

#[derive(Serialize)]
struct MinClearance<'a> {
    touchdown_m: f64,
    hull_m: f64,
    landing: &'a Option<pd_core::LandingRunSummary>,
}

#[derive(Serialize)]
struct FuelUse {
    remaining_kg: f64,
    used_kg: f64,
}

#[derive(Serialize)]
struct Endpoint<'a> {
    physics_step: u64,
    sim_time_s: f64,
    physical_outcome: &'a pd_core::PhysicalOutcome,
    mission_outcome: &'a pd_core::MissionOutcome,
    end_reason: &'a pd_core::EndReason,
}

#[derive(Serialize)]
struct CompactRunSummary<'a> {
    minimum_clearance: MinClearance<'a>,
    fuel: FuelUse,
    endpoint: Endpoint<'a>,
}

#[derive(Serialize)]
struct FlightSummary<'a> {
    schema_id: &'static str,
    input_identity: &'a str,
    policy: &'a WaypointV2Policy,
    result: CompactResult<'a>,
    run_summary: Option<CompactRunSummary<'a>>,
    timings: OutputTimings,
}

/// Execute a V2 flight and write its complete source, result and compact
/// summary beneath a fresh output root. Invalid or unsupported preflight
/// results are still recorded, but receive no trajectory report.
pub fn write_waypoint_v2_flight(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &WaypointV2Policy,
    output_dir: &Path,
) -> Result<WaypointV2FlightResult> {
    let total_started = Instant::now();
    reserve_output_root(output_dir)?;
    let result = run_waypoint_v2_flight(request, policy)?;

    let output_started = Instant::now();
    write_create_only(&output_dir.join("scenario.json"), &request.scenario)?;
    write_create_only(&output_dir.join("flight.json"), &result)?;
    if let (Some(manifest), Some(ordinary)) = (&result.manifest, &result.ordinary_flight) {
        pd_report::write_run_report(
            &output_dir.join("report.html"),
            &request.scenario,
            None,
            manifest,
            &ordinary.events,
            &ordinary.samples,
            &[],
            None,
        )?;
    }
    let output_s = output_started.elapsed().as_secs_f64();
    let total_s = total_started.elapsed().as_secs_f64();
    let run_summary = result.manifest.as_ref().map(|manifest| CompactRunSummary {
        minimum_clearance: MinClearance {
            touchdown_m: manifest.summary.min_touchdown_clearance_m,
            hull_m: manifest.summary.min_hull_clearance_m,
            landing: &manifest.summary.landing,
        },
        fuel: FuelUse {
            remaining_kg: manifest.summary.fuel_remaining_kg,
            used_kg: manifest.summary.fuel_used_kg,
        },
        endpoint: Endpoint {
            physics_step: manifest.physics_steps,
            sim_time_s: manifest.sim_time_s,
            physical_outcome: &manifest.physical_outcome,
            mission_outcome: &manifest.mission_outcome,
            end_reason: &manifest.end_reason,
        },
    });
    let summary = FlightSummary {
        schema_id: SUMMARY_SCHEMA_ID,
        input_identity: &result.input_identity,
        policy: &result.policy,
        result: CompactResult {
            planning_stop: &result.planning_stop,
            reason: &result.reason,
            correction_count: result.correction_count,
            initial_nominal_terrain_blocked: result.initial_nominal_terrain_blocked,
            integrity_passed: result.integrity_passed,
            physical_outcome: &result.physical_outcome,
            mission_outcome: &result.mission_outcome,
            final_source_replay_passed: result.final_source_replay_passed,
            timings: &result.timings,
        },
        run_summary,
        timings: OutputTimings { output_s, total_s },
    };
    write_create_only(&output_dir.join("summary.json"), &summary)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use pd_plan::waypoint_v2::WaypointV2Policy;

    use super::write_waypoint_v2_flight;

    fn repository_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval lives below workspace root")
            .to_path_buf()
    }

    fn fresh_test_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "pd-waypoint-v2-output-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn unsupported_preflight_is_recorded_without_report_and_root_is_create_only() {
        let root = fresh_test_root();
        let request = crate::waypoint_direct_known_flat_generation_request(&repository_root())
            .expect("known flat input");
        let mut policy = WaypointV2Policy::default();
        policy.maximum_corrections -= 1;

        let result =
            write_waypoint_v2_flight(&request, &policy, &root).expect("write typed result");
        assert_eq!(
            result.planning_stop,
            pd_plan::waypoint_v2::WaypointV2Stop::Unsupported
        );
        assert!(!result.final_source_replay_passed);
        assert!(root.join("scenario.json").is_file());
        assert!(root.join("flight.json").is_file());
        assert!(root.join("summary.json").is_file());
        assert!(!root.join("report.html").exists());
        let summary: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("summary.json")).expect("summary bytes"))
                .expect("valid compact summary");
        assert_eq!(summary["result"]["planning_stop"], "unsupported");
        assert!(summary["result"]["timings"]["planning_s"].is_number());
        assert!(summary["timings"]["output_s"].is_number());
        assert!(summary["run_summary"].is_null());

        let summary_before = fs::read(root.join("summary.json")).expect("summary exists");
        assert!(write_waypoint_v2_flight(&request, &policy, &root).is_err());
        assert_eq!(
            fs::read(root.join("summary.json")).expect("summary remains"),
            summary_before,
        );
        fs::remove_dir_all(root).expect("remove this test's temporary output root");
    }
}
