use pd_missions::{
    NativeScenarioInputV1, ScenarioInputProvenanceV1, write_json_create_only, write_scenario_inputs,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "pd-evaluation-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pd-maneuver-example"))
        .args(args)
        .output()
        .unwrap()
}
fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}
fn setup(temp: &Temp, independent: bool, small_storage: bool) -> PathBuf {
    let input = temp.0.join("inputs");
    let scenario = serde_json::from_str(include_str!(
        "../../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap();
    write_scenario_inputs(
        &input,
        ScenarioInputProvenanceV1 {
            implementation: "software-test".into(),
            executable_sha256: "0".repeat(64),
            sources: vec![],
        },
        &[NativeScenarioInputV1 {
            id: "anchor".into(),
            scenario,
            labels: BTreeMap::new(),
            parameters: BTreeMap::new(),
            lineage: vec!["software-fixture".into()],
        }],
    )
    .unwrap();
    let interval = |min: f64, max: f64| json!({"min":min,"max":max});
    let recipe = json!({"schema_version":1,"task_id":"cli-software-panel","population":{"kind":"local","anchor_id":"anchor","cohort":"software-only","seed":997,"indices":if independent {json!([0,1,2])}else{json!([0,1,2,3,4])},"independent":independent,
        "bounds":{"clearance_m":interval(10.0,20.0),"offset_per_clearance":interval(0.0,0.2),"lateral_residual_mps":interval(-1.0,1.0),"braking_speed_ratio":interval(0.1,0.4),"attitude_rad":interval(-0.1,0.1),"angular_rate_radps":interval(-0.1,0.1),"dry_mass_kg":[7200.0],"fuel_kg":interval(3000.0,6300.0),"pad_width_m":interval(36.0,36.0),"pad_surface_y_m":interval(0.0,0.0),"deadline_s":interval(3.0/120.0,3.0/120.0),"terrain":[{"kind":"flat"}]},
        "transforms":[{"kind":"identity"}],"diagnostics":if independent {json!([])}else{json!([{"kind":"unsupported_gravity","mps2":1.62}])}},
        "programs":[{"schema_version":1,"activation":{"kind":"immediate"}}],"direct_parity":!independent,
        "budget":{"max_wall_seconds":60,"max_attempt_wall_seconds":20,"max_output_bytes":if small_storage {16*1024*1024}else{128*1024*1024},"max_bundle_bytes":16*1024*1024,"min_free_bytes":0},
        "exposure_inventories":[],"audit_bins":{"touchdown_clearance_m":[0.0,5.0,20.0],"vy_mps":[-20.0,-3.0,0.0]},"joint_groups":[["touchdown_clearance_m","vy_mps"]],
        "confirmation":if independent {json!({"minimum_reliability":0.99,"significance":0.05})}else{Value::Null}});
    let recipe_path = temp.0.join("recipe.json");
    write_json_create_only(&recipe_path, &recipe).unwrap();
    let plan = temp.0.join("plan");
    let output = cli(&[
        "prepare-evaluation",
        "--recipe",
        text(&recipe_path),
        "--inputs",
        text(&input),
        "--output",
        text(&plan),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    plan
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn real_batch_keeps_verified_misses_rejected_diagnostics_and_exact_parity_then_reuses_without_new_flights()
 {
    let temp = Temp::new();
    let plan = setup(&temp, false, false);
    let run = temp.0.join("run");
    let result = cli(&[
        "run-evaluation",
        "--plan",
        text(&plan),
        "--output",
        text(&run),
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let summary = read(&run.join("summary.json"));
    assert_eq!(summary["complete"], true);
    assert_eq!(summary["statuses"]["verified"], 4);
    assert_eq!(summary["statuses"]["input_rejected"], 1);
    assert_eq!(summary["physical_outcomes"]["timed_out"], 4);
    assert_eq!(summary["work_this_run"]["original_physics_steps"], 12);
    assert_eq!(summary["work_this_run"]["reference_physics_steps"], 12);
    assert_eq!(
        summary["work_this_run"]["proof_resources"]["neutral_replay_passes_started"],
        12
    );
    assert_eq!(
        summary["bytes"].as_u64().unwrap(),
        pd_maneuver_example::evaluation::tree_bytes(&run).unwrap()
    );
    let checked = cli(&[
        "check-evaluation-results",
        "--plan",
        text(&plan),
        "--results",
        text(&run),
    ]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(
        !cli(&[
            "run-evaluation",
            "--plan",
            text(&plan),
            "--output",
            text(&run)
        ])
        .status
        .success()
    );
    let resumed = temp.0.join("continued");
    let result = cli(&[
        "run-evaluation",
        "--plan",
        text(&plan),
        "--output",
        text(&resumed),
        "--continue-from",
        text(&run),
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let reused = read(&resumed.join("summary.json"));
    assert_eq!(reused["statuses"]["reused_verified"], 4);
    assert_eq!(reused["work_this_run"]["original_physics_steps"], 0);
    assert_eq!(
        reused["work_this_run"]["proof_resources"]["neutral_replay_passes_started"],
        4
    );
    let attempt = std::fs::read_dir(run.join("attempts"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let capture = attempt.join("bundle/capture/evaluation.json");
    let mut bytes = std::fs::read(&capture).unwrap();
    bytes.push(b' ');
    std::fs::write(&capture, bytes).unwrap();
    assert!(
        !cli(&[
            "check-evaluation-results",
            "--plan",
            text(&plan),
            "--results",
            text(&run)
        ])
        .status
        .success()
    );
}

#[test]
fn budget_exhaustion_retains_all_not_started_rows_without_shrinking_the_schedule() {
    let temp = Temp::new();
    let plan = setup(&temp, false, true);
    let run = temp.0.join("capped");
    assert!(
        !cli(&[
            "run-evaluation",
            "--plan",
            text(&plan),
            "--output",
            text(&run)
        ])
        .status
        .success()
    );
    let summary = read(&run.join("summary.json"));
    assert_eq!(summary["complete"], false);
    assert_eq!(summary["statuses"]["not_started"], 5);
    assert_eq!(std::fs::read_dir(run.join("records")).unwrap().count(), 5);
    assert_eq!(std::fs::read_dir(run.join("attempts")).unwrap().count(), 0);
    assert!(
        cli(&[
            "check-evaluation-results",
            "--plan",
            text(&plan),
            "--results",
            text(&run)
        ])
        .status
        .success()
    );
}

#[test]
fn fixed_confirmation_finishes_with_an_honest_negative_verdict_and_drift_fails_before_flight() {
    let temp = Temp::new();
    let plan = setup(&temp, true, false);
    let run = temp.0.join("confirmation");
    let result = cli(&[
        "run-evaluation",
        "--plan",
        text(&plan),
        "--output",
        text(&run),
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let summary = read(&run.join("summary.json"));
    assert_eq!(summary["confirmation"]["planned_n"], 3);
    assert_eq!(summary["confirmation"]["complete"], true);
    assert_eq!(summary["confirmation"]["observed_landings"], 0);
    assert_eq!(summary["confirmation"]["lower_bound"], 0.0);
    assert_eq!(summary["confirmation"]["meets_required_reliability"], false);
    std::fs::write(plan.join("scenario-000000.json"), b"{}").unwrap();
    let rejected = temp.0.join("drifted");
    assert!(
        !cli(&[
            "run-evaluation",
            "--plan",
            text(&plan),
            "--output",
            text(&rejected)
        ])
        .status
        .success()
    );
    assert!(!rejected.exists());
}

#[test]
fn supervisor_capability_error_still_seals_a_complete_fault_and_unstarted_inventory() {
    let temp = Temp::new();
    let plan = setup(&temp, false, false);
    let root = temp.0.join("fault");
    let output = Command::new(env!("CARGO_BIN_EXE_pd-maneuver-example"))
        .args([
            "run-evaluation",
            "--plan",
            text(&plan),
            "--output",
            text(&root),
        ])
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let summary = read(&root.join("summary.json"));
    assert_eq!(summary["complete"], false);
    assert_eq!(summary["statuses"]["fault"], 1);
    assert_eq!(summary["statuses"]["not_started"], 4);
    assert_eq!(std::fs::read_dir(root.join("attempts")).unwrap().count(), 0);
    assert!(
        cli(&[
            "check-evaluation-results",
            "--plan",
            text(&plan),
            "--results",
            text(&root)
        ])
        .status
        .success()
    );
}
