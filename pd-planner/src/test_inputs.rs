//! Tracked inputs for maintained planner regressions, without research runners.

use std::path::Path;

use crate::WaypointDirectNominalDirectGenerationRequest;

pub(crate) fn planner_request(case_id: &str) -> WaypointDirectNominalDirectGenerationRequest {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let plan: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo.join("fixtures/research/waypoint_v2_practical_suite_plan_v1.json"))
            .unwrap(),
    )
    .unwrap();
    let recipe = plan["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == case_id)
        .expect("tracked case ID");
    assert!(
        recipe.get("features").is_none() && recipe.get("mutation").is_none(),
        "solver test helper only reads unmodified frozen base cases"
    );
    let base = recipe["base"].as_str().unwrap();
    for manifest in plan["source_manifests"].as_object().unwrap().values() {
        let value: serde_json::Value = serde_json::from_slice(
            &std::fs::read(repo.join(manifest["path"].as_str().unwrap())).unwrap(),
        )
        .unwrap();
        if let Some(case) = value["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["case_id"] == base)
        {
            let mut scenario: pd_core::ScenarioSpec =
                serde_json::from_value(case["scenario"].clone()).unwrap();
            scenario.id = case_id.into();
            scenario.name = case_id.into();
            return WaypointDirectNominalDirectGenerationRequest {
                probe_id: scenario.id.clone(),
                scenario,
                source_pad_id: case["source_pad_id"].as_str().unwrap().into(),
                target_pad_id: case["target_pad_id"].as_str().unwrap().into(),
                policy: crate::WaypointDirectNominalDirectGenerationPolicyV1::default(),
            };
        }
    }
    panic!("missing frozen base {base}")
}

/// Preserve an exact historical diagnostic geometry, not its executable gate.
pub(crate) fn archived_obstacle_request(
    case_id: &str,
) -> WaypointDirectNominalDirectGenerationRequest {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo.join(
            "fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json",
        ))
        .unwrap(),
    )
    .unwrap();
    let case = manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["case_id"] == case_id)
        .expect("archived diagnostic ID");
    WaypointDirectNominalDirectGenerationRequest {
        scenario: serde_json::from_value(case["scenario"].clone()).unwrap(),
        source_pad_id: case["source_pad_id"].as_str().unwrap().into(),
        target_pad_id: case["target_pad_id"].as_str().unwrap().into(),
        probe_id: case_id.into(),
        policy: serde_json::from_value(manifest["generation_policy"].clone()).unwrap(),
    }
}
