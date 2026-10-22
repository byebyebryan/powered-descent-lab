use crate::PlannerDecisionV1;
use pd_control::maneuver::*;
use pd_run::AgentDescriptorV1;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountersV1 {
    pub physics_steps: u64,
    pub command_updates: u64,
    pub backend_updates: u64,
    pub planner_calls: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerCallV1 {
    pub feedback: ManeuverFeedbackV1,
    pub decision: PlannerDecisionV1,
    pub admission: Option<AdmissionV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryV1 {
    pub physics_step: u64,
    pub planner: Option<PlannerCallV1>,
    pub owner: CommandOwnerV1,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManeuverTraceV1 {
    pub schema_version: u32,
    pub descriptor: AgentDescriptorV1,
    pub boundaries: Vec<BoundaryV1>,
    pub final_feedback: ManeuverFeedbackV1,
    pub counters: CountersV1,
}
