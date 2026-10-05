#![cfg(feature = "planner-v2")]

use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use pd_core::{MissionOutcome, PhysicalOutcome};
use pd_eval::{
    LocalClearingOrdinaryEvidenceV1, WaypointDirectNominalDirectGenerationRequest,
    WaypointV2CliProgressEntryV1, WaypointV2CliProgressV1, WaypointV2Cycle,
    WaypointV2CycleDecision, WaypointV2FlightResult, WaypointV2Policy, WaypointV2SessionProgress,
    WaypointV2Stop, load_nominal_direct_operational_fresh_inputs,
    nominal_direct_flight::nominal_direct_flight_identity, reserve_waypoint_v2_flight_output,
    write_waypoint_v2_cli_bundle,
};
use serde_json::Value;

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pd-cli-v2-integration-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create owned test root");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        // Keep the exact test-owned directory when an assertion fails so a
        // reviewer can inspect the retained bundle before cleaning it up.
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).expect("remove owned integration-test directory");
        }
    }
}

fn supported_request() -> WaypointDirectNominalDirectGenerationRequest {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    load_nominal_direct_operational_fresh_inputs(repository)
        .expect("tracked, sealed operational request")
        .remove(0)
        .1
}

fn tracked_pack_request(case_id: &str) -> WaypointDirectNominalDirectGenerationRequest {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let pack_path = repository.join(pd_eval::waypoint_v2_pack::DEFAULT_PLANNER_PACK_PATH);
    let input = pd_eval::waypoint_v2_pack::load_waypoint_v2_pack_case_input(&pack_path, case_id)
        .expect("tracked V2 pack input");
    WaypointDirectNominalDirectGenerationRequest {
        probe_id: input.scenario.id.clone(),
        scenario: input.scenario,
        source_pad_id: input.source_pad_id,
        target_pad_id: input.target_pad_id,
        policy: pd_eval::WaypointDirectNominalDirectGenerationPolicyV1::default(),
    }
}

fn write_copied_scenario(root: &Path, request: &WaypointDirectNominalDirectGenerationRequest) {
    fs::write(
        root.join("scenario.json"),
        serde_json::to_vec_pretty(&request.scenario).unwrap(),
    )
    .unwrap();
}

fn cli(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pd-cli"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("start pd-cli subprocess")
}

fn flight_args(request: &WaypointDirectNominalDirectGenerationRequest) -> Vec<String> {
    vec![
        "waypoint-v2-flight".into(),
        "scenario.json".into(),
        "--source-pad".into(),
        request.source_pad_id.clone(),
        "--target-pad".into(),
        request.target_pad_id.clone(),
        "--output-dir".into(),
        "capture".into(),
    ]
}

fn string_args(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

fn inventory(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(root)
        .expect("read owned directory")
        .map(|entry| {
            let entry = entry.expect("directory entry");
            (
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path()).expect("regular artifact bytes"),
            )
        })
        .collect()
}

fn sha256(bytes: &[u8]) -> String {
    let mut child = Command::new("sha256sum")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("start sha256sum");
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned()
}

#[test]
fn copied_scenario_cli_flight_is_portable_receipted_and_replay_is_read_only() {
    let root = TestRoot::new();
    let request = supported_request();
    write_copied_scenario(root.path(), &request);

    let preflight = vec![
        "waypoint-v2-flight".to_owned(),
        "scenario.json".to_owned(),
        "--source-pad".to_owned(),
        request.source_pad_id.clone(),
        "--target-pad".to_owned(),
        request.target_pad_id.clone(),
        "--preflight-only".to_owned(),
    ];
    let preflight_result = cli(root.path(), &string_args(&preflight));
    assert!(
        preflight_result.status.success(),
        "preflight stderr: {}",
        String::from_utf8_lossy(&preflight_result.stderr)
    );
    let preflight_json: Value = serde_json::from_slice(&preflight_result.stdout).unwrap();
    assert_eq!(preflight_json["status"], "preflight_only");
    assert_eq!(preflight_json["supported"], true);
    assert!(!root.path().join("capture").exists());
    assert_eq!(
        inventory(root.path())
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["scenario.json"]
    );

    let flight = flight_args(&request);
    let flight_result = cli(root.path(), &string_args(&flight));
    assert!(
        flight_result.status.success(),
        "flight stderr: {}",
        String::from_utf8_lossy(&flight_result.stderr)
    );
    let flight_json: Value = serde_json::from_slice(&flight_result.stdout).unwrap();
    assert_eq!(flight_json["schema_id"], "planner_v2_cli_flight_v1");
    assert_eq!(flight_json["planning_stop"], "landed");
    assert_eq!(flight_json["physical_outcome"], "landed_on_target");
    assert_eq!(flight_json["mission_outcome"], "success");
    assert_eq!(flight_json["integrity_passed"], true);
    assert_eq!(flight_json["final_source_replay_passed"], true);

    let capture = root.path().join("capture");
    let before_replay = inventory(&capture);
    let receipt: Value = serde_json::from_slice(&before_replay["bundle.json"]).unwrap();
    assert_eq!(receipt["schema_id"], "planner_v2_cli_bundle_v1");
    let artifact_hashes = receipt["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        artifact_hashes
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "flight.json",
            "progress.json",
            "report.html",
            "scenario.json",
            "summary.json",
        ]
    );
    for (name, expected) in artifact_hashes {
        assert_eq!(
            sha256(&before_replay[name.as_str()]),
            expected.as_str().unwrap(),
            "receipt digest for {name}"
        );
    }

    let saved_flight: WaypointV2FlightResult =
        serde_json::from_slice(&before_replay["flight.json"]).unwrap();
    let other_cwd = root.path().join("elsewhere");
    fs::create_dir(&other_cwd).unwrap();
    let replay_result = cli(
        &other_cwd,
        &["waypoint-v2-replay", "--bundle-dir", "../capture"],
    );
    assert!(
        replay_result.status.success(),
        "replay stderr: {}",
        String::from_utf8_lossy(&replay_result.stderr)
    );
    let replay_json: Value = serde_json::from_slice(&replay_result.stdout).unwrap();
    assert_eq!(replay_json["schema_id"], "planner_v2_cli_replay_v1");
    assert_eq!(replay_json["replay_passed"], true);
    assert_eq!(replay_json["input_identity"], flight_json["input_identity"]);
    assert_eq!(replay_json["physical_outcome"], "landed_on_target");
    assert_eq!(replay_json["mission_outcome"], "success");
    assert_eq!(
        replay_json["comparison_sha256"]["manifest"],
        sha256(&serde_json::to_vec(saved_flight.manifest.as_ref().unwrap()).unwrap())
    );
    let saved_ordinary = saved_flight.ordinary_flight.as_ref().unwrap();
    for (key, actual) in [
        (
            "actions",
            serde_json::to_vec(&saved_ordinary.actions).unwrap(),
        ),
        (
            "events",
            serde_json::to_vec(&saved_ordinary.events).unwrap(),
        ),
        (
            "samples",
            serde_json::to_vec(&saved_ordinary.samples).unwrap(),
        ),
        (
            "final_state",
            serde_json::to_vec(&saved_ordinary.final_state).unwrap(),
        ),
        (
            "incoming_contact",
            serde_json::to_vec(&saved_ordinary.incoming_contact).unwrap(),
        ),
    ] {
        assert_eq!(replay_json["comparison_sha256"][key], sha256(&actual));
    }
    assert_eq!(
        replay_json["physics_step"],
        saved_ordinary.final_state.physics_step
    );
    assert_eq!(
        replay_json["sim_time_s"],
        saved_ordinary.final_state.sim_time_s
    );
    assert_eq!(
        inventory(&capture),
        before_replay,
        "replay must be read-only"
    );

    let original_flight = before_replay["flight.json"].clone();
    let mut tampered_flight = original_flight.clone();
    tampered_flight.push(b' ');
    fs::write(capture.join("flight.json"), tampered_flight).unwrap();
    let tampered_artifact = cli(
        other_cwd.as_path(),
        &["waypoint-v2-replay", "--bundle-dir", "../capture"],
    );
    assert!(!tampered_artifact.status.success());
    fs::write(capture.join("flight.json"), original_flight).unwrap();

    let original_receipt = before_replay["bundle.json"].clone();
    let mut tampered_receipt: Value = serde_json::from_slice(&original_receipt).unwrap();
    tampered_receipt["request"]["target_pad_id"] = Value::String("different_pad".into());
    fs::write(
        capture.join("bundle.json"),
        serde_json::to_vec(&tampered_receipt).unwrap(),
    )
    .unwrap();
    let tampered_request = cli(
        &other_cwd,
        &["waypoint-v2-replay", "--bundle-dir", "../capture"],
    );
    assert!(!tampered_request.status.success());
    fs::write(capture.join("bundle.json"), original_receipt).unwrap();
}

#[test]
fn corrected_tracked_scenario_cli_flight_writes_complete_bundle_and_portable_replay() {
    let root = TestRoot::new();
    let request = tracked_pack_request("v2_ridge_early");
    write_copied_scenario(root.path(), &request);

    let flight = flight_args(&request);
    let flight_result = cli(root.path(), &string_args(&flight));
    assert!(
        flight_result.status.success(),
        "corrected flight stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&flight_result.stdout),
        String::from_utf8_lossy(&flight_result.stderr)
    );
    let flight_json: Value = serde_json::from_slice(&flight_result.stdout).unwrap();
    assert_eq!(flight_json["status"], "completed");
    assert_eq!(flight_json["planning_stop"], "landed");
    assert_eq!(flight_json["physical_outcome"], "landed_on_target");
    assert_eq!(flight_json["mission_outcome"], "success");
    assert_eq!(flight_json["integrity_passed"], true);
    assert_eq!(flight_json["final_source_replay_passed"], true);
    assert_eq!(flight_json["correction_count"], 1);

    let capture = root.path().join("capture");
    let before_replay = inventory(&capture);
    assert_eq!(
        before_replay.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "bundle.json",
            "flight.json",
            "progress.json",
            "report.html",
            "scenario.json",
            "summary.json",
        ]
    );
    let receipt: Value = serde_json::from_slice(&before_replay["bundle.json"]).unwrap();
    let artifact_hashes = receipt["artifact_sha256"].as_object().unwrap();
    for (name, expected) in artifact_hashes {
        assert_eq!(
            sha256(&before_replay[name.as_str()]),
            expected.as_str().unwrap(),
            "receipt digest for {name}"
        );
    }

    let saved_flight: WaypointV2FlightResult =
        serde_json::from_slice(&before_replay["flight.json"]).unwrap();
    let saved_progress: WaypointV2CliProgressV1 =
        serde_json::from_slice(&before_replay["progress.json"]).unwrap();
    let handoff = saved_progress
        .entries
        .iter()
        .find_map(|entry| match entry.progress {
            WaypointV2SessionProgress::Handoff {
                piece_index,
                entry_physics_step,
                handoff_physics_step,
                ..
            } => Some((piece_index, entry_physics_step, handoff_physics_step)),
            WaypointV2SessionProgress::Terminal { .. } => None,
        })
        .expect("corrected trace has a handoff");
    let (piece_index, piece_origin, actual_h) = handoff;
    let cycle = &saved_flight.cycles[piece_index];
    let segment = saved_flight
        .segments
        .iter()
        .filter(|segment| segment.kind == pd_eval::WaypointV2SegmentKind::LocalCorrection)
        .nth(piece_index)
        .unwrap();
    let selected = cycle
        .local_search
        .as_ref()
        .and_then(|search| search.selected.as_ref())
        .expect("corrected cycle retains selected proposal");
    assert_eq!(piece_origin, cycle.current_state.physics_step);
    assert_eq!(
        segment.start_physics_step,
        selected.schedule.entry_physics_step
    );
    assert_eq!(segment.entry_state, selected.entry_state);
    assert_eq!(segment.end_physics_step, actual_h);
    assert_eq!(segment.end_state, selected.handoff_state);
    assert_eq!(segment.proposal_identity, selected.identity);
    assert_ne!(piece_origin, segment.start_physics_step);
    assert_ne!(segment.start_physics_step, segment.end_physics_step);

    let other_cwd = root.path().join("elsewhere");
    fs::create_dir(&other_cwd).unwrap();
    let replay_result = cli(
        &other_cwd,
        &["waypoint-v2-replay", "--bundle-dir", "../capture"],
    );
    assert!(
        replay_result.status.success(),
        "corrected replay stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&replay_result.stdout),
        String::from_utf8_lossy(&replay_result.stderr)
    );
    let replay_json: Value = serde_json::from_slice(&replay_result.stdout).unwrap();
    assert_eq!(replay_json["replay_passed"], true);
    assert_eq!(replay_json["input_identity"], flight_json["input_identity"]);
    assert_eq!(replay_json["physical_outcome"], "landed_on_target");
    assert_eq!(replay_json["mission_outcome"], "success");
    assert_eq!(inventory(&capture), before_replay);
}

#[test]
fn unsupported_cli_capture_and_replay_are_typed_nonzero_without_trajectories() {
    let root = TestRoot::new();
    let mut request = supported_request();
    request.target_pad_id = "missing_pad".into();
    write_copied_scenario(root.path(), &request);
    let args = flight_args(&request);
    let flight_result = cli(root.path(), &string_args(&args));
    assert!(!flight_result.status.success());
    let outcome: Value = serde_json::from_slice(&flight_result.stdout).unwrap();
    assert_eq!(outcome["supported"], false);
    assert_eq!(outcome["planning_stop"], "invalid_input");
    assert_eq!(outcome["physical_outcome"], Value::Null);
    assert_eq!(outcome["mission_outcome"], Value::Null);

    let capture = root.path().join("capture");
    let names = inventory(&capture).keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "bundle.json",
            "flight.json",
            "progress.json",
            "scenario.json",
            "summary.json",
        ]
    );
    let saved: WaypointV2FlightResult =
        serde_json::from_slice(&inventory(&capture)["flight.json"]).unwrap();
    assert!(saved.ordinary_flight.is_none());
    assert!(saved.manifest.is_none());
    assert!(saved.segments.is_empty());

    let before_replay = inventory(&capture);
    let other_cwd = root.path().join("elsewhere");
    fs::create_dir(&other_cwd).unwrap();
    let replay_result = cli(
        &other_cwd,
        &["waypoint-v2-replay", "--bundle-dir", "../capture"],
    );
    assert!(!replay_result.status.success());
    let replay_json: Value = serde_json::from_slice(&replay_result.stdout).unwrap();
    assert_eq!(replay_json["replay_passed"], false);
    assert_eq!(replay_json["physical_outcome"], Value::Null);
    assert_eq!(replay_json["comparison_sha256"], Value::Null);
    assert_eq!(inventory(&capture), before_replay);
}

#[test]
fn zero_command_finite_bundle_replays_in_cli_without_claiming_landing() {
    let root = TestRoot::new();
    let request = supported_request();
    write_copied_scenario(root.path(), &request);

    let context = pd_core::RunContext::from_scenario(&request.scenario).unwrap();
    let deadline = (context
        .sim
        .max_time_s
        .min(request.policy.analytical_policy.mission_budget_s())
        * 120.0)
        .floor() as u64;
    let mut guard = pd_core::AllowAllBoundedRunGuard;
    let source = pd_core::run_simulation_bounded(
        &context,
        "waypoint_v2_supplied_commands",
        pd_core::BoundedRunLimitsV1 {
            command_coverage_end_physics_step: 0,
            hard_end_physics_step: deadline,
        },
        |_context, _observation| Err("zero-command trace must not request a command".into()),
        &mut guard,
    )
    .unwrap();
    assert!(source.run.actions.is_empty());
    assert_eq!(source.final_state.physics_step, 0);
    let ordinary = LocalClearingOrdinaryEvidenceV1 {
        final_state: source.final_state.clone(),
        incoming_contact: source.incoming_contact.clone(),
        actions: source.run.actions.clone(),
        events: source.run.events.clone(),
        samples: source.run.samples.clone(),
    };
    let policy = WaypointV2Policy::revision_3();
    let cycle = WaypointV2Cycle {
        cycle_index: 0,
        current_state: ordinary.final_state.clone(),
        nominal_search_identity: "integration-zero-command".into(),
        nominal_proposal_identity: None,
        nominal_peak_com_height_m: None,
        nominal_attempt_status_counts: BTreeMap::new(),
        nominal_rejection_reason_counts: BTreeMap::new(),
        nominal_updates: Vec::new(),
        audit: None,
        fixed_consumed_prefix_proven: false,
        decision: WaypointV2CycleDecision::NoNominal,
        conflict_state: None,
        conflict_incoming_contact: None,
        local_search: None,
    };
    let result = WaypointV2FlightResult {
        policy: policy.clone(),
        input_identity: nominal_direct_flight_identity(&(&request, &policy)).unwrap(),
        planning_stop: WaypointV2Stop::NoNominal,
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
    let capture = root.path().join("zero-bundle");
    reserve_waypoint_v2_flight_output(&capture).unwrap();
    let progress = WaypointV2CliProgressV1 {
        schema_id: pd_eval::waypoint_v2_bundle::WAYPOINT_V2_CLI_PROGRESS_SCHEMA_ID.into(),
        entries: vec![WaypointV2CliProgressEntryV1 {
            elapsed_s: 0.0,
            progress: WaypointV2SessionProgress::Terminal {
                piece_index: Some(0),
                entry_physics_step: Some(0),
                planning_stop: WaypointV2Stop::NoNominal,
                correction_count: 0,
                physics_step: Some(0),
            },
        }],
        finalization_elapsed_s: 0.0,
    };
    write_waypoint_v2_cli_bundle(
        &request,
        &result,
        &capture,
        &progress,
        std::time::Instant::now(),
    )
    .unwrap();

    let other_cwd = root.path().join("elsewhere");
    fs::create_dir(&other_cwd).unwrap();
    let replay_result = cli(
        &other_cwd,
        &["waypoint-v2-replay", "--bundle-dir", "../zero-bundle"],
    );
    assert!(
        replay_result.status.success(),
        "zero-command saved evidence stderr: {}",
        String::from_utf8_lossy(&replay_result.stderr)
    );
    let replay: Value = serde_json::from_slice(&replay_result.stdout).unwrap();
    assert_eq!(replay["replay_passed"], true);
    assert_eq!(replay["planning_stop"], "no_nominal");
    assert_eq!(replay["physical_outcome"], "flying");
    assert_eq!(replay["mission_outcome"], "in_progress");
    assert_eq!(replay["physics_step"], 0);
    assert_eq!(replay["sim_time_s"], 0.0);
}

#[cfg(unix)]
#[test]
fn create_only_roots_reject_existing_paths_and_dangling_symlinks_before_flight() {
    use std::os::unix::fs::symlink;

    let root = TestRoot::new();
    let request = supported_request();
    write_copied_scenario(root.path(), &request);
    let args = flight_args(&request);

    let existing = root.path().join("capture");
    fs::create_dir(&existing).unwrap();
    fs::write(existing.join("sentinel"), b"preserve").unwrap();
    let rejected_existing = cli(root.path(), &string_args(&args));
    assert!(!rejected_existing.status.success());
    assert_eq!(fs::read(existing.join("sentinel")).unwrap(), b"preserve");
    assert_eq!(fs::read_dir(&existing).unwrap().count(), 1);

    let mut symlink_args = args;
    let missing = root.path().join("not-created");
    let dangling = root.path().join("dangling-capture");
    symlink(&missing, &dangling).unwrap();
    let output_arg = symlink_args
        .iter_mut()
        .position(|arg| arg == "capture")
        .unwrap();
    symlink_args[output_arg] = "dangling-capture".into();
    let rejected_dangling = cli(root.path(), &string_args(&symlink_args));
    assert!(!rejected_dangling.status.success());
    assert!(
        fs::symlink_metadata(&dangling)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(!missing.exists());
}
