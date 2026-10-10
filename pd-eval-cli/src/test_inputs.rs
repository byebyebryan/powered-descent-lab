//! Tracked inputs for maintained planner regressions, without research runners.

use std::{path::Path, sync::OnceLock};

use crate::{WaypointDirectNominalDirectGenerationRequest, waypoint_v2_pack::WaypointV2PackInput};

pub(crate) fn planner_request(case_id: &str) -> WaypointDirectNominalDirectGenerationRequest {
    static INPUTS: OnceLock<Vec<WaypointV2PackInput>> = OnceLock::new();
    let inputs = INPUTS.get_or_init(|| {
        crate::waypoint_v2_pack::check_default_planner_v2_binding()
            .expect("tracked current planner inputs")
            .0
    });
    let input = inputs
        .iter()
        .find(|input| input.case_id == case_id)
        .expect("tracked case ID");
    WaypointDirectNominalDirectGenerationRequest {
        scenario: input.scenario.clone(),
        source_pad_id: input.source_pad_id.clone(),
        target_pad_id: input.target_pad_id.clone(),
        probe_id: input.scenario.id.clone(),
        policy: crate::WaypointDirectNominalDirectGenerationPolicyV1::default(),
    }
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
