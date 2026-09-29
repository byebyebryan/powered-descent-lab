use serde::{Deserialize, Serialize};

use crate::{
    ContactClassification,
    model::{Command, EndReason, MissionOutcome, PhysicalOutcome, RunArtifacts, RunContext},
    sim::SimulationState,
};

pub const BOUNDED_RUN_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BoundedRunLimitsV1 {
    pub command_coverage_end_physics_step: u64,
    pub hard_end_physics_step: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundedRunStopCauseV1 {
    MissionTerminal,
    CoverageExhausted,
    HardDeadlineReached,
    ScenarioHorizonReached,
    SafetyRejected,
    ExecutionInvalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundedRunFailureStageV1 {
    InitialGuard,
    CommandSelection,
    PreTransitionGuard,
    PhysicsTransition,
    PostTransitionGuard,
    ReplayInput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundedGuardFailureDispositionV1 {
    SafetyRejected,
    ExecutionInvalid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundedGuardFailureV1 {
    pub disposition: BoundedGuardFailureDispositionV1,
    pub reason: String,
}

impl BoundedGuardFailureV1 {
    pub fn new(disposition: BoundedGuardFailureDispositionV1, reason: impl Into<String>) -> Self {
        Self {
            disposition,
            reason: reason.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundedRunNonFiniteCategoryV1 {
    Nan,
    PositiveInfinity,
    NegativeInfinity,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundedRunNonFiniteEvidenceV1 {
    pub field: String,
    pub category: BoundedRunNonFiniteCategoryV1,
    pub boundary_physics_step: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundedRunFailureV1 {
    pub stage: BoundedRunFailureStageV1,
    pub disposition: BoundedGuardFailureDispositionV1,
    pub boundary_physics_step: u64,
    pub reason: String,
    #[serde(default)]
    pub non_finite: Option<BoundedRunNonFiniteEvidenceV1>,
}

impl BoundedRunFailureV1 {
    pub fn new(
        stage: BoundedRunFailureStageV1,
        disposition: BoundedGuardFailureDispositionV1,
        boundary_physics_step: u64,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            stage,
            disposition,
            boundary_physics_step,
            reason: reason.into(),
            non_finite: None,
        }
    }

    pub fn non_finite(
        stage: BoundedRunFailureStageV1,
        field: impl Into<String>,
        category: BoundedRunNonFiniteCategoryV1,
        boundary_physics_step: u64,
    ) -> Self {
        let field = field.into();
        Self {
            stage,
            disposition: BoundedGuardFailureDispositionV1::ExecutionInvalid,
            boundary_physics_step,
            reason: format!("nonfinite numeric field {field}"),
            non_finite: Some(BoundedRunNonFiniteEvidenceV1 {
                field,
                category,
                boundary_physics_step,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationStateSnapshotV1 {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub position_m: crate::Vec2,
    pub velocity_mps: crate::Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
    pub held_command: Command,
    pub physical_outcome: PhysicalOutcome,
    pub mission_outcome: MissionOutcome,
    pub end_reason: EndReason,
    pub min_touchdown_clearance_m: f64,
    pub min_hull_clearance_m: f64,
    pub max_speed_mps: f64,
    pub max_abs_attitude_rad: f64,
    pub max_abs_angular_rate_radps: f64,
    pub waypoint_sequence_passed: usize,
    pub waypoint_sequence_first_failure_index: Option<usize>,
    pub waypoint_handoff_window_index: Option<usize>,
}

impl SimulationStateSnapshotV1 {
    pub fn from_state(state: &SimulationState) -> Self {
        Self {
            sim_time_s: state.sim_time_s,
            physics_step: state.physics_step,
            position_m: state.position_m,
            velocity_mps: state.velocity_mps,
            attitude_rad: state.attitude_rad,
            angular_rate_radps: state.angular_rate_radps,
            fuel_kg: state.fuel_kg,
            held_command: state.held_command,
            physical_outcome: state.physical_outcome.clone(),
            mission_outcome: state.mission_outcome.clone(),
            end_reason: state.end_reason.clone(),
            min_touchdown_clearance_m: state.min_touchdown_clearance_m,
            min_hull_clearance_m: state.min_hull_clearance_m,
            max_speed_mps: state.max_speed_mps,
            max_abs_attitude_rad: state.max_abs_attitude_rad,
            max_abs_angular_rate_radps: state.max_abs_angular_rate_radps,
            waypoint_sequence_passed: state.waypoint_sequence_passed,
            waypoint_sequence_first_failure_index: state.waypoint_sequence_first_failure_index,
            waypoint_handoff_window_index: state.waypoint_handoff_window_index,
        }
    }

    /// Reconstructs a query-only state view. The returned state must not be stepped.
    pub fn to_simulation_state(&self) -> SimulationState {
        SimulationState {
            sim_time_s: self.sim_time_s,
            physics_step: self.physics_step,
            position_m: self.position_m,
            velocity_mps: self.velocity_mps,
            attitude_rad: self.attitude_rad,
            angular_rate_radps: self.angular_rate_radps,
            fuel_kg: self.fuel_kg,
            held_command: self.held_command,
            physical_outcome: self.physical_outcome.clone(),
            mission_outcome: self.mission_outcome.clone(),
            end_reason: self.end_reason.clone(),
            min_touchdown_clearance_m: self.min_touchdown_clearance_m,
            min_hull_clearance_m: self.min_hull_clearance_m,
            max_speed_mps: self.max_speed_mps,
            max_abs_attitude_rad: self.max_abs_attitude_rad,
            max_abs_angular_rate_radps: self.max_abs_angular_rate_radps,
            waypoint_sequence_passed: self.waypoint_sequence_passed,
            waypoint_sequence_first_failure_index: self.waypoint_sequence_first_failure_index,
            waypoint_handoff_window_index: self.waypoint_handoff_window_index,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncomingContactV1 {
    pub classification: ContactClassification,
    pub state: SimulationStateSnapshotV1,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SimulationStepReportV1 {
    pub incoming_contact: Option<IncomingContactV1>,
    pub events: Vec<crate::EventRecord>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundedRunArtifactsV1 {
    pub schema_version: u32,
    pub limits: BoundedRunLimitsV1,
    pub stop: BoundedRunStopCauseV1,
    pub coverage_reached: bool,
    pub hard_end_reached: bool,
    pub final_state: SimulationStateSnapshotV1,
    #[serde(default)]
    pub incoming_contact: Option<IncomingContactV1>,
    #[serde(default)]
    pub failure: Option<BoundedRunFailureV1>,
    pub run: RunArtifacts,
}

/// A policy hook for opt-in bounded execution. The default is deliberately neutral.
pub trait BoundedRunGuard {
    fn initial(
        &mut self,
        _context: &RunContext,
        _state: &SimulationState,
    ) -> Result<(), BoundedGuardFailureV1> {
        Ok(())
    }

    fn before_transition(
        &mut self,
        _context: &RunContext,
        _state: &SimulationState,
        _command: Command,
    ) -> Result<(), BoundedGuardFailureV1> {
        Ok(())
    }

    fn after_transition(
        &mut self,
        _context: &RunContext,
        _state: &SimulationState,
        _incoming_contact: Option<&IncomingContactV1>,
    ) -> Result<(), BoundedGuardFailureV1> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AllowAllBoundedRunGuard;

impl BoundedRunGuard for AllowAllBoundedRunGuard {}
