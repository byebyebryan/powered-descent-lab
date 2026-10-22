use super::*;
use crate::evaluation::{EvaluationBudgetV1, bind_file, identity};
use pd_core::ScenarioSpec;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn root() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "pd-timed-eval-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&p).unwrap();
    p
}
fn anchor() -> ScenarioSpec {
    serde_json::from_str(include_str!(
        "../../../../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap()
}
fn recipe() -> FlatRecipeV2 {
    FlatRecipeV2 {
        schema_version: 2,
        sampler: SAMPLER.into(),
        phase: "pilot".into(),
        cohort: "software-pilot".into(),
        seed: 0x92bc6712,
        originals: 3,
        max_proposals: 24,
        development_extensions: false,
        budget: EvaluationBudgetV1 {
            max_wall_seconds: 60,
            max_attempt_wall_seconds: 20,
            max_output_bytes: 128 * 1024 * 1024,
            max_bundle_bytes: 16 * 1024 * 1024,
            min_free_bytes: 0,
        },
        release_gate: None,
        exposure_plans: vec![],
    }
}

#[test]
fn input_only_streams_are_order_independent_and_keep_proposal_rejections() {
    let r = recipe();
    let a = anchor();
    let x = flat_proposal(&a, &r, 7).unwrap();
    let _ = flat_proposal(&a, &r, 2).unwrap();
    assert_eq!(flat_proposal(&a, &r, 7).unwrap(), x);
    let mut fresh = r.clone();
    fresh.cohort = "separate-fresh".into();
    assert_ne!(
        pd_missions::physical_identity(&flat_proposal(&a, &fresh, 7).unwrap().0).unwrap(),
        pd_missions::physical_identity(&x.0).unwrap()
    );
    let output = root();
    let recipe_file = output.join("recipe.json");
    let anchor_file = output.join("anchor.json");
    pd_missions::write_json_create_only(&recipe_file, &r).unwrap();
    pd_missions::write_json_create_only(&anchor_file, &a).unwrap();
    let prepared = output.join("prepared");
    let plan = prepare_timed_evaluation(&recipe_file, &anchor_file, &prepared).unwrap();
    assert_eq!(plan.cases.len(), 3);
    assert!(plan.proposals.len() >= 3);
    assert!(plan.proposals.len() <= 24);
    assert!(
        plan.proposals
            .iter()
            .all(|p| p["native_forecast_steps"] == 0)
    );
    assert_eq!(
        identity(&check_timed_evaluation(&prepared).unwrap()).unwrap(),
        identity(&plan).unwrap()
    );
    std::fs::remove_dir_all(output).unwrap();
}

#[test]
fn native_timed_attempt_preserves_shape_proof_and_complete_scored_accounting() {
    let output = root();
    let mut r = recipe();
    r.originals = 1;
    r.max_proposals = 8;
    let recipe_file = output.join("recipe.json");
    let anchor_file = output.join("anchor.json");
    pd_missions::write_json_create_only(&recipe_file, &r).unwrap();
    pd_missions::write_json_create_only(&anchor_file, &anchor()).unwrap();
    let prepared = output.join("prepared");
    let plan = prepare_timed_evaluation(&recipe_file, &anchor_file, &prepared).unwrap();
    let sha = bind_file(&prepared.join("plan.json"), 32 * 1024 * 1024)
        .unwrap()
        .sha256;
    let attempt = output.join("attempt");
    let item = execute_timed_attempt(&prepared, 0, &sha, &attempt).unwrap();
    assert!(item.verified_target_landing);
    assert!(item.shape.as_ref().unwrap().passed, "{:?}", item.shape);
    assert_eq!(item.original_reference_constructions, 1);
    assert!(item.original_admission_actuation_checks > 1);
    assert!(item.verification.as_ref().unwrap().passed());
    let row = TimedAttemptRowV2 {
        case_index: 0,
        case_id: plan.cases[0].id.clone(),
        status: "verified".into(),
        reason: None,
        result: Some(bind_file(&attempt.join("result.json"), 1024 * 1024).unwrap()),
        supervised_wall_seconds: Some(0.1),
    };
    let summary = summarize_timed(&plan, sha, vec![row], 0.1, 0, None).unwrap();
    assert!(summary.complete);
    assert_eq!(summary.verified_primary_landings, 1);
    assert_eq!(summary.primary_shape_failures, 0);
    assert!(!summary.qualified);
    assert_eq!(summary.proof_resources.neutral_replay_passes_started, 3);
    let results = output.join("results");
    std::fs::create_dir(&results).unwrap();
    pd_missions::write_json_create_only(&results.join("summary.json"), &summary).unwrap();
    let spec = TimedReportSpecV2 {
        schema_version: 2,
        title: "Timed software report".into(),
        populations: vec![TimedReportPopulationV2 {
            label: "Software input".into(),
            plan: prepared,
            results,
        }],
        historical_regression: None,
        historical_inventory: None,
        max_report_bytes: 64 * 1024 * 1024,
    };
    let spec_path = output.join("report-spec.json");
    pd_missions::write_json_create_only(&spec_path, &spec).unwrap();
    let report = output.join("report");
    render_timed_evaluation_report(&spec_path, &report).unwrap();
    let html = std::fs::read_to_string(report.join("report.html")).unwrap();
    assert!(html.contains("paired-review-data"));
    assert!(html.contains("Browse 12 missions per page"));
    let detail = std::fs::read_to_string(report.join("flight-0.html")).unwrap();
    assert!(detail.contains("chart-spatial"));
    assert!(detail.contains("data-back-review"));
    assert!(detail.contains("static-flight-fallback"));
    assert!(detail.contains("saved Poly Landing reference"));
    assert!(detail.contains("Raw incoming contact at"));
    assert!(detail.contains("Post-contact summary"));
    assert!(!detail.contains("IncomingContactV1 {"));
    assert!(!detail.contains("contact residual Some("));
    assert_eq!(
        std::fs::read(report.join("navigation.json")).unwrap(),
        std::fs::read(report.join("report-input.json")).unwrap()
    );
    let receipt: serde_json::Value =
        crate::evaluation::read_json(&report.join("receipt.json"), 1024 * 1024).unwrap();
    assert_eq!(receipt["new_physics_steps"], 0);
    assert_eq!(receipt["new_replays"], 0);
    assert_eq!(receipt["rich_views"], 1);
    std::fs::remove_dir_all(output).unwrap();
}

#[test]
fn structured_development_retains_boundary_proposals_and_separate_extension_roles() {
    let output = root();
    let mut r = recipe();
    r.phase = "development".into();
    r.originals = 64;
    r.max_proposals = 512;
    r.development_extensions = true;
    let recipe_path = output.join("recipe.json");
    let anchor_path = output.join("anchor.json");
    pd_missions::write_json_create_only(&recipe_path, &r).unwrap();
    pd_missions::write_json_create_only(&anchor_path, &anchor()).unwrap();
    let prepared = output.join("prepared");
    let plan = prepare_timed_evaluation(&recipe_path, &anchor_path, &prepared).unwrap();
    assert_eq!(
        plan.cases.iter().filter(|c| c.role == "primary").count(),
        64
    );
    assert_eq!(
        plan.cases.iter().filter(|c| c.role == "diagnostic").count(),
        96
    );
    assert_eq!(
        plan.cases.iter().filter(|c| c.role == "transform").count(),
        48
    );
    assert_eq!(
        plan.cases
            .iter()
            .filter(|c| c.native_support != "supported")
            .count(),
        12
    );
    assert!(
        plan.proposals
            .iter()
            .any(|p| p["admission"].get("Err").is_some())
    );
    check_timed_evaluation(&prepared).unwrap();
    std::fs::remove_dir_all(output).unwrap();
}

#[test]
fn missing_slots_cannot_form_a_smaller_successful_denominator() {
    let output = root();
    let r = recipe();
    let recipe_file = output.join("recipe.json");
    let anchor_file = output.join("anchor.json");
    pd_missions::write_json_create_only(&recipe_file, &r).unwrap();
    pd_missions::write_json_create_only(&anchor_file, &anchor()).unwrap();
    let prepared = output.join("prepared");
    let plan = prepare_timed_evaluation(&recipe_file, &anchor_file, &prepared).unwrap();
    let summary =
        summarize_timed(&plan, "test".into(), vec![], 0.0, 0, Some("limit".into())).unwrap();
    assert!(!summary.complete);
    assert_eq!(summary.primary_originals, 3);
    assert_eq!(summary.verified_primary_landings, 0);
    assert!(!summary.eligible_for_confirmation);
    let mut bad = r;
    bad.phase = "confirmation".into();
    bad.originals = 600;
    bad.max_proposals = 4800;
    assert!(bad.validate().is_err());
    std::fs::remove_dir_all(output).unwrap();
}
