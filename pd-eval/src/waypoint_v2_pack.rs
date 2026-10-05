//! Native input expansion, execution and report regeneration for the named
//! policy-3 V2 lab pack. This is evaluator-only; it does not change the
//! controller or the ordinary ScenarioPackSpec workflow.

pub const DEFAULT_PLANNER_PACK_PATH: &str = "fixtures/packs/planner_v2_lab_suite.json";
pub const WAYPOINT_V2_BATCH_SCHEMA_ID: &str = "planner_v2_eval_batch_v1";

mod capture_validation;
mod execution;
mod input;
mod model;
mod presentation;
mod provenance;
mod tree_report;

pub use execution::run_waypoint_v2_pack;
#[cfg(test)]
pub(crate) use execution::summarize;
pub(crate) use input::check_default_planner_v2_binding;
pub use input::{is_waypoint_v2_pack, load_waypoint_v2_pack_case_input};
pub use model::*;
pub use presentation::{
    render_waypoint_v2_batch, render_waypoint_v2_batch_preview, render_waypoint_v2_site_pages,
    validated_waypoint_v2_batch,
};
pub(crate) use provenance::capture_source_state;

#[cfg(test)]
mod tests;
