//! Compatibility module; maintained execution is owned by `pd-planner`.
pub use pd_planner::waypoint_v2::*;
pub mod handoff_probe;
#[cfg(test)]
mod tests;
