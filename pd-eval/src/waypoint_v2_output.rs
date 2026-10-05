//! Create-only artifacts for the opt-in V2 flight mode.

use std::{fs, path::Path, time::Instant};

use anyhow::{Context, Result, ensure};
use pd_plan::waypoint_v2::WaypointV2Policy;
use serde::Serialize;

use crate::{
    WaypointDirectNominalDirectGenerationRequest, WaypointV2FlightResult,
    evidence_io::{
        reserve_output_root, write_bytes_create_only_with_context, write_json_create_only,
    },
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
    reserve_waypoint_v2_flight_output(output_dir)?;
    let result = run_waypoint_v2_flight(request, policy)?;

    write_waypoint_v2_flight_result(request, &result, output_dir, total_started)?;
    Ok(result)
}

/// Reserve a fresh output root before executing any live piece.
pub fn reserve_waypoint_v2_flight_output(output_dir: &Path) -> Result<()> {
    reserve_output_root(output_dir)
}

/// Rich reports require coherent executed-segment annotations. An
/// integrity-failed implementation error may retain a source-replayed manifest
/// alongside deliberately incomplete ledgers, so preserve its raw and compact
/// evidence without projecting a misleading rich plot.
pub(crate) fn waypoint_v2_rich_report_eligible(result: &WaypointV2FlightResult) -> bool {
    result.manifest.is_some()
        && result.ordinary_flight.is_some()
        && !(result.planning_stop == pd_plan::waypoint_v2::WaypointV2Stop::ImplementationError
            && !result.integrity_passed)
}

/// Write an already completed (or preflight-only) result exactly once.
/// Callers that drive a [`crate::WaypointV2Session`] directly use this entry
/// point so recording artifacts never re-executes the flight.
pub fn write_waypoint_v2_flight_result(
    request: &WaypointDirectNominalDirectGenerationRequest,
    result: &WaypointV2FlightResult,
    output_dir: &Path,
    total_started: Instant,
) -> Result<()> {
    let metadata = fs::symlink_metadata(output_dir)
        .with_context(|| format!("inspect reserved output root {}", output_dir.display()))?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "reserved output root must be a real directory"
    );

    let output_started = Instant::now();
    write_json_create_only(&output_dir.join("scenario.json"), &request.scenario)?;
    write_json_create_only(&output_dir.join("flight.json"), &result)?;
    if waypoint_v2_rich_report_eligible(result) {
        let navigation = pd_report::flight_annotations::AnnotationNavigation {
            source_links: vec![
                pd_report::flight_annotations::NavigationLink {
                    label: "Scenario JSON".into(),
                    href: "scenario.json".into(),
                },
                pd_report::flight_annotations::NavigationLink {
                    label: "Flight JSON".into(),
                    href: "flight.json".into(),
                },
                pd_report::flight_annotations::NavigationLink {
                    label: "Run summary JSON".into(),
                    href: "summary.json".into(),
                },
            ],
            ..Default::default()
        };
        let html = crate::waypoint_v2_report::render_rich_flight(
            &request.scenario,
            result,
            navigation,
            format!(
                "Native V2 flight · {} · executed handoffs",
                result.policy.policy_id
            ),
        )?;
        write_bytes_create_only_with_context(
            &output_dir.join("report.html"),
            html.as_bytes(),
            "create-only report",
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
    write_json_create_only(&output_dir.join("summary.json"), &summary)?;
    Ok(())
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
    fn supported_capture_uses_shared_rich_report_with_flight_annotations() {
        let root = fresh_test_root();
        let (_, request) = crate::load_nominal_direct_operational_fresh_inputs(&repository_root())
            .expect("supported fixture inputs")
            .remove(0);
        let result = write_waypoint_v2_flight(&request, &WaypointV2Policy::revision_3(), &root)
            .expect("write supported capture");
        let ordinary = result.ordinary_flight.as_ref().expect("simulated flight");
        let html = fs::read_to_string(root.join("report.html")).expect("rich report");
        assert!(html.contains("id=\"chart-spatial\""));
        assert!(html.contains("id=\"chart-metrics\""));
        for mode in ["mission", "guidance", "speed", "throttle", "vectors"] {
            assert!(html.contains(&format!("data-mode=\"{mode}\"")));
        }
        let payload_start =
            html.find("const reportData = ").expect("rich payload") + "const reportData = ".len();
        let payload = serde_json::Deserializer::from_str(&html[payload_start..])
            .into_iter::<serde_json::Value>()
            .next()
            .expect("payload present")
            .expect("valid rich payload");
        assert_eq!(payload["scenarioId"], request.scenario.id);
        assert_eq!(
            payload["samples"].as_array().unwrap().len(),
            ordinary.samples.len()
        );
        for key in [
            "events",
            "manifest",
            "flightStats",
            "botStats",
            "missionDetails",
        ] {
            assert!(payload.get(key).is_some(), "missing rich field: {key}");
        }
        assert_eq!(
            payload["flightAnnotations"]["corrections"],
            serde_json::json!([])
        );
        assert_eq!(
            payload["flightAnnotations"]["navigation"]["sourceLinks"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        fs::remove_dir_all(root).expect("remove this test's temporary output root");
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
