use pd_maneuver_example::evaluation::timed::{FlatRecipeV2, SAMPLER, TimedEvaluationSummaryV2};
use pd_maneuver_example::evaluation::{EvaluationBudgetV1, read_json};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "pd-timed-cli-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&root).unwrap();
    root
}
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pd-maneuver-example"))
        .args(args)
        .output()
        .unwrap()
}
fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}
fn inputs(root: &Path, bounded_out: bool) -> (PathBuf, PathBuf) {
    let recipe = FlatRecipeV2 {
        schema_version: 2,
        sampler: SAMPLER.into(),
        phase: "pilot".into(),
        cohort: "cli-software".into(),
        seed: 9812734,
        originals: 2,
        max_proposals: 16,
        development_extensions: false,
        budget: EvaluationBudgetV1 {
            max_wall_seconds: 60,
            max_attempt_wall_seconds: 20,
            max_output_bytes: if bounded_out {
                16 * 1024 * 1024
            } else {
                128 * 1024 * 1024
            },
            max_bundle_bytes: 16 * 1024 * 1024,
            min_free_bytes: 0,
        },
        release_gate: None,
        exposure_plans: vec![],
    };
    let recipe_path = root.join("recipe.json");
    pd_missions::write_json_create_only(&recipe_path, &recipe).unwrap();
    let anchor = root.join("anchor.json");
    std::fs::write(
        &anchor,
        include_bytes!("../../../fixtures/scenarios/flat_terminal_descent.json"),
    )
    .unwrap();
    (recipe_path, anchor)
}

#[test]
fn timed_supervisor_uses_bound_complete_results_for_recovery_without_new_native_work() {
    let root = root();
    let (recipe, anchor) = inputs(&root, false);
    let plan = root.join("plan");
    assert!(
        run(&[
            "prepare-timed-evaluation",
            "--recipe",
            text(&recipe),
            "--anchor",
            text(&anchor),
            "--output",
            text(&plan)
        ])
        .status
        .success()
    );
    let results = root.join("results");
    let executed = run(&[
        "run-timed-evaluation",
        "--plan",
        text(&plan),
        "--output",
        text(&results),
    ]);
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stderr)
    );
    let first: TimedEvaluationSummaryV2 =
        read_json(&results.join("summary.json"), 8 * 1024 * 1024).unwrap();
    assert!(first.complete);
    assert_eq!(first.verified_primary_landings, 2);
    assert_eq!(first.primary_shape_failures, 0);
    let continued = root.join("continued");
    let reused = run(&[
        "run-timed-evaluation",
        "--plan",
        text(&plan),
        "--output",
        text(&continued),
        "--continue-from",
        text(&results),
    ]);
    assert!(
        reused.status.success(),
        "{}",
        String::from_utf8_lossy(&reused.stderr)
    );
    let next: TimedEvaluationSummaryV2 =
        read_json(&continued.join("summary.json"), 8 * 1024 * 1024).unwrap();
    assert!(next.complete);
    assert!(next.rows.iter().all(|r| r.status == "reused"));
    assert_eq!(
        std::fs::read_dir(continued.join("attempts"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(next.original_physics_steps, first.original_physics_steps);
    assert!(
        run(&[
            "check-timed-evaluation-results",
            "--plan",
            text(&plan),
            "--results",
            text(&continued)
        ])
        .status
        .success()
    );
    let changed = plan.join("inputs/0-scenario.json");
    std::fs::write(&changed, b"{}").unwrap();
    let drift = root.join("drift");
    assert!(
        !run(&[
            "run-timed-evaluation",
            "--plan",
            text(&plan),
            "--output",
            text(&drift)
        ])
        .status
        .success()
    );
    assert!(!drift.exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn timed_budget_stop_keeps_all_unstarted_originals_in_the_schedule() {
    let root = root();
    let (recipe, anchor) = inputs(&root, true);
    let plan = root.join("plan");
    assert!(
        run(&[
            "prepare-timed-evaluation",
            "--recipe",
            text(&recipe),
            "--anchor",
            text(&anchor),
            "--output",
            text(&plan)
        ])
        .status
        .success()
    );
    let results = root.join("results");
    assert!(
        !run(&[
            "run-timed-evaluation",
            "--plan",
            text(&plan),
            "--output",
            text(&results)
        ])
        .status
        .success()
    );
    let summary: TimedEvaluationSummaryV2 =
        read_json(&results.join("summary.json"), 8 * 1024 * 1024).unwrap();
    assert!(!summary.complete);
    assert_eq!(summary.primary_originals, 2);
    assert_eq!(summary.rows.len(), 2);
    assert!(summary.rows.iter().all(|r| r.status == "not_started"));
    assert_eq!(summary.original_physics_steps, 0);
    assert!(!summary.qualified);
    std::fs::remove_dir_all(root).unwrap();
}
