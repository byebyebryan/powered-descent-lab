//! Tracked inputs for maintained planner regressions, without research runners.

use std::sync::OnceLock;

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
