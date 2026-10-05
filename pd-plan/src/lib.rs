//! Shared deterministic planning math and sealed current planner policies.
//!
//! Simulation-dependent nominal realization and the owned piecewise flight
//! session live in pd-eval. Saved route contracts remain in pd-core; the retired
//! chord-based V1 search and research candidate exposure are not executable.

pub mod ballistic;
pub mod local_clearing;
pub mod waypoint_v2;
