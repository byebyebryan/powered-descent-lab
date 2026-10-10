//! Strategy-independent episode records and execution over the authoritative plant.
//! Concrete controllers, planning, rewards and mission generation are consumers.

use std::collections::BTreeMap;
use std::time::Instant;

use pd_core::{Observation, RunArtifacts, RunContext, SimulationError, run_simulation};
use serde::{Deserialize, Serialize};

mod agent;
mod prepared;
pub use agent::*;
pub use prepared::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TelemetryValue {
    Float(f64),
    Integer(i64),
    Bool(bool),
    Text(String),
}

impl From<f64> for TelemetryValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<i64> for TelemetryValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<bool> for TelemetryValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<String> for TelemetryValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for TelemetryValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentMarker {
    pub id: String,
    pub label: String,
    pub x_m: Option<f64>,
    pub y_m: Option<f64>,
    #[serde(default)]
    pub metadata: BTreeMap<String, TelemetryValue>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentFrame {
    pub command: pd_core::Command,
    pub status: String,
    pub phase: Option<String>,
    #[serde(default)]
    pub metrics: BTreeMap<String, TelemetryValue>,
    #[serde(default)]
    pub markers: Vec<AgentMarker>,
}

impl AgentFrame {
    pub fn command_only(command: pd_core::Command) -> Self {
        Self {
            command,
            status: String::new(),
            phase: None,
            metrics: BTreeMap::new(),
            markers: Vec::new(),
        }
    }

    pub fn clamped(mut self) -> Self {
        self.command = self.command.clamped();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentUpdateRecord {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub controller_update_index: u64,
    #[serde(default)]
    pub compute_time_us: Option<u64>,
    pub frame: AgentFrame,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunPerformanceStats {
    pub wall_time_us: u64,
    #[serde(default)]
    pub thread_cpu_time_us: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentRunArtifacts {
    pub run: RunArtifacts,
    pub controller_updates: Vec<AgentUpdateRecord>,
    pub performance: RunPerformanceStats,
}

/// Compatibility seam for existing context-aware implementations. New approaches
/// use `Agent` and its physical-only mission view.
pub fn run_frames<F>(
    ctx: &RunContext,
    agent_id: &str,
    mut update: F,
) -> Result<AgentRunArtifacts, SimulationError>
where
    F: FnMut(&RunContext, &Observation) -> AgentFrame,
{
    let mut controller_updates = Vec::new();
    let wall_started_at = Instant::now();
    let thread_cpu_started_at = current_thread_cpu_time_us();
    let run = run_simulation(ctx, agent_id, |ctx, observation| {
        let started_at = Instant::now();
        let frame = update(ctx, observation).clamped();
        let compute_time_us = started_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
        controller_updates.push(AgentUpdateRecord {
            sim_time_s: observation.sim_time_s,
            physics_step: observation.physics_step,
            controller_update_index: controller_updates.len() as u64,
            compute_time_us: Some(compute_time_us),
            frame: frame.clone(),
        });
        frame.command
    })?;

    Ok(AgentRunArtifacts {
        run,
        controller_updates,
        performance: RunPerformanceStats {
            wall_time_us: wall_started_at
                .elapsed()
                .as_micros()
                .min(u128::from(u64::MAX)) as u64,
            thread_cpu_time_us: current_thread_cpu_time_us()
                .zip(thread_cpu_started_at)
                .map(|(finished, started)| finished.saturating_sub(started)),
        },
    })
}

#[cfg(unix)]
fn current_thread_cpu_time_us() -> Option<u64> {
    let mut spec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut spec) };
    if rc != 0 || spec.tv_sec < 0 || spec.tv_nsec < 0 {
        return None;
    }
    let secs = spec.tv_sec as u128;
    let nanos = spec.tv_nsec as u128;
    Some(((secs * 1_000_000) + (nanos / 1_000)).min(u128::from(u64::MAX)) as u64)
}

#[cfg(not(unix))]
fn current_thread_cpu_time_us() -> Option<u64> {
    None
}
