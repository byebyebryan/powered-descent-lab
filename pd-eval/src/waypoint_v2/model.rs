//! Persisted flight records and additive session progress. Field order and
//! serde names are part of the retained evidence contract.

use std::collections::BTreeMap;

use pd_core::{
    FlightProgramUpdateV1, IncomingContactV1, MissionOutcome, PhysicalOutcome,
    SimulationStateSnapshotV1,
};
use serde::{Deserialize, Serialize};

use super::{WaypointV2Policy, WaypointV2Stop};
use crate::{AirborneDirectAuditV1, LocalClearingOrdinaryEvidenceV1, LocalClearingProposalV1};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointV2SegmentKind {
    InitialNominal,
    LocalCorrection,
    AirborneNominal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2Segment {
    pub kind: WaypointV2SegmentKind,
    pub start_physics_step: u64,
    pub end_physics_step: u64,
    pub proposal_identity: String,
    pub updates: Vec<FlightProgramUpdateV1>,
    pub entry_state: SimulationStateSnapshotV1,
    pub end_state: SimulationStateSnapshotV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2Entry {
    pub entry_id: String,
    pub physics_step: u64,
    pub admitted: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2LocalSearch {
    pub entries: Vec<WaypointV2Entry>,
    pub row_count: usize,
    pub boundary_count: usize,
    pub accepted_row_count: usize,
    pub row_status_counts: BTreeMap<String, usize>,
    pub row_stop_reason_counts: BTreeMap<String, usize>,
    pub boundary_status_counts: BTreeMap<String, usize>,
    pub selected: Option<LocalClearingProposalV1>,
    pub certificate_state: Option<SimulationStateSnapshotV1>,
    pub handoff_source_replay_passed: bool,
    pub certificate_source_replay_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointV2CycleDecision {
    NoNominal,
    Unsupported,
    NominalRejected,
    Direct,
    TerrainBlocked,
    LocalCleared,
    CorrectionLimit,
    NoClearing,
    NoProgress,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2Cycle {
    pub cycle_index: usize,
    pub current_state: SimulationStateSnapshotV1,
    pub nominal_search_identity: String,
    pub nominal_proposal_identity: Option<String>,
    pub nominal_peak_com_height_m: Option<f64>,
    pub nominal_attempt_status_counts: BTreeMap<String, usize>,
    pub nominal_rejection_reason_counts: BTreeMap<String, usize>,
    pub nominal_updates: Vec<FlightProgramUpdateV1>,
    pub audit: Option<AirborneDirectAuditV1>,
    pub fixed_consumed_prefix_proven: bool,
    pub decision: WaypointV2CycleDecision,
    pub conflict_state: Option<SimulationStateSnapshotV1>,
    pub conflict_incoming_contact: Option<IncomingContactV1>,
    pub local_search: Option<WaypointV2LocalSearch>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2Timings {
    pub planning_s: f64,
    pub execution_s: f64,
    pub replay_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2FlightResult {
    pub policy: WaypointV2Policy,
    pub input_identity: String,
    pub planning_stop: WaypointV2Stop,
    pub reason: Option<String>,
    pub correction_count: u32,
    pub initial_nominal_terrain_blocked: bool,
    pub integrity_passed: bool,
    pub physical_outcome: Option<PhysicalOutcome>,
    pub mission_outcome: Option<MissionOutcome>,
    pub absolute_deadline_physics_step: Option<u64>,
    pub cycles: Vec<WaypointV2Cycle>,
    pub segments: Vec<WaypointV2Segment>,
    pub ordinary_flight: Option<LocalClearingOrdinaryEvidenceV1>,
    pub final_source_replay_passed: bool,
    pub manifest: Option<pd_core::RunManifest>,
    pub failed_local_row: Option<crate::LocalClearingRowV1>,
    pub timings: WaypointV2Timings,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WaypointV2SessionProgress {
    Handoff {
        piece_index: usize,
        correction_count: u32,
        /// Origin of the whole piece (initial rest or previous actual H),
        /// not intervention E. E is recorded in the selected local schedule.
        entry_physics_step: u64,
        handoff_physics_step: u64,
    },
    Terminal {
        /// None when termination occurs before a new cycle is constructed.
        piece_index: Option<usize>,
        /// Whole-piece origin, not the local correction's intervention E.
        entry_physics_step: Option<u64>,
        planning_stop: WaypointV2Stop,
        correction_count: u32,
        physics_step: Option<u64>,
    },
}
