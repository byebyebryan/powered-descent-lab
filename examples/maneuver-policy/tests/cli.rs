use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "pd-maneuver-cli-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn call(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pd-maneuver-example"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
    fn inputs(&self) {
        let mut scenario: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .unwrap();
        scenario["sim"]["max_time_s"] = serde_json::json!(0.125);
        fs::write(
            self.0.join("scenario.json"),
            serde_json::to_vec(&scenario).unwrap(),
        )
        .unwrap();
        fs::write(
            self.0.join("program.json"),
            br#"{"schema_version":1,"activation":{"kind":"immediate"}}"#,
        )
        .unwrap();
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn cli_runs_verifies_and_renders_explicit_create_only_artifacts() {
    let temp = Temp::new();
    temp.inputs();
    let run = temp.call(&[
        "run",
        "--scenario",
        "scenario.json",
        "--program",
        "program.json",
        "--output",
        "bundle",
    ]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(report["physical_replay"]["status"], "passed");
    assert_eq!(report["controller_reconstruction"]["status"], "passed");
    assert_eq!(report["planner_reconstruction"]["status"], "passed");
    let receipt = fs::read(temp.0.join("bundle/receipt.json")).unwrap();
    assert!(temp.call(&["verify", "bundle"]).status.success());
    assert!(
        temp.call(&["report", "bundle", "--output", "report.html"])
            .status
            .success()
    );
    let html = fs::read_to_string(temp.0.join("report.html")).unwrap();
    assert!(html.contains("<!doctype html>") || html.contains("<!DOCTYPE html>"));
    assert!(html.contains("trajectory") && html.contains("fuel"));
    assert!(
        !temp
            .call(&["report", "bundle", "--output", "report.html"])
            .status
            .success()
    );
    assert!(
        !temp
            .call(&[
                "run",
                "--scenario",
                "scenario.json",
                "--program",
                "program.json",
                "--output",
                "bundle"
            ])
            .status
            .success()
    );
    assert_eq!(
        fs::read(temp.0.join("bundle/receipt.json")).unwrap(),
        receipt
    );
    fs::remove_file(temp.0.join("bundle/receipt.json")).unwrap();
    let verify = temp.call(&["verify", "bundle"]);
    assert!(!verify.status.success());
    let result: serde_json::Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(result["physical_replay"]["status"], "passed");
    assert_ne!(result["bundle_integrity"]["status"], "passed");
}

#[test]
fn cli_rejects_bad_program_and_unsupported_input_without_fabricating_capture() {
    let temp = Temp::new();
    temp.inputs();
    fs::write(
        temp.0.join("program.json"),
        br#"{"schema_version":1,"activation":{"kind":"fixed_tick","physics_step":1}}"#,
    )
    .unwrap();
    assert!(
        !temp
            .call(&[
                "run",
                "--scenario",
                "scenario.json",
                "--program",
                "program.json",
                "--output",
                "bad-program"
            ])
            .status
            .success()
    );
    assert!(!temp.0.join("bad-program").exists());
    temp.inputs();
    let mut scenario: serde_json::Value =
        serde_json::from_slice(&fs::read(temp.0.join("scenario.json")).unwrap()).unwrap();
    scenario["world"]["gravity_mps2"] = serde_json::json!(1.62);
    fs::write(
        temp.0.join("scenario.json"),
        serde_json::to_vec(&scenario).unwrap(),
    )
    .unwrap();
    let unsupported = temp.call(&[
        "run",
        "--scenario",
        "scenario.json",
        "--program",
        "program.json",
        "--output",
        "unsupported",
    ]);
    assert!(!unsupported.status.success());
    assert!(String::from_utf8_lossy(&unsupported.stderr).contains("UnsupportedInput"));
    assert!(!temp.0.join("unsupported").exists());
}
