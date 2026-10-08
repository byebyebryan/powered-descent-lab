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

/// Compact query-only diagnostics. A trace may stop after a valid handoff's
/// certificate; its stop reason is not an executed-flight failure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2RowDiagnostic {
    pub row_id: String,
    pub entry_physics_step: u64,
    pub physically_propagated: bool,
    pub stop_reason: Option<String>,
    pub stop_state: Option<SimulationStateSnapshotV1>,
    pub minimum_clearance_m: Option<f64>,
    pub minimum_progress_x_m: f64,
    pub boundary_status_counts: BTreeMap<String, usize>,
    pub first_continuation_rejection: Option<crate::LocalClearingBoundaryV1>,
    /// First locally accepted actual query state; never a landing suffix proof.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eligible_handoff: Option<WaypointV2EligibleHandoffDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2EligibleHandoffDiagnostic {
    pub state: SimulationStateSnapshotV1,
    pub actual_fuel_burn_to_handoff_kg: f64,
    pub braking_room: Option<pd_plan::local_clearing::HandoffBrakingRoomEstimate>,
}

impl WaypointV2RowDiagnostic {
    pub(super) fn from_row(row: &crate::LocalClearingRowV1, progress: f64) -> Self {
        let mut counts = BTreeMap::new();
        for boundary in &row.boundaries {
            *counts.entry(boundary.status.clone()).or_default() += 1;
        }
        Self {
            row_id: row.row_id.clone(),
            entry_physics_step: row.entry_physics_step,
            physically_propagated: row.physically_propagated,
            stop_reason: row.stop_reason.clone(),
            stop_state: row.stop_state.clone(),
            minimum_clearance_m: row.minimum_clearance_m,
            minimum_progress_x_m: progress,
            boundary_status_counts: counts,
            first_continuation_rejection: row
                .boundaries
                .iter()
                .find(|b| {
                    matches!(
                        b.status.as_str(),
                        "unsafe_continuation" | "continuation_unsupported" | "handoff_unsupported"
                    )
                })
                .cloned(),
            eligible_handoff: None,
        }
    }
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
    /// Absent in historical captures; never an input to selection or proof.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub row_diagnostics: Vec<WaypointV2RowDiagnostic>,
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn historical_search_roundtrips_without_new_metadata() {
        let old = json!({
            "entries": [], "row_count": 0, "boundary_count": 0,
            "accepted_row_count": 0, "row_status_counts": {},
            "row_stop_reason_counts": {}, "boundary_status_counts": {},
            "selected": null, "certificate_state": null,
            "handoff_source_replay_passed": false,
            "certificate_source_replay_passed": false
        });
        let search: WaypointV2LocalSearch = serde_json::from_value(old.clone()).unwrap();
        assert!(search.row_diagnostics.is_empty());
        assert_eq!(serde_json::to_value(search).unwrap(), old);
    }

    #[test]
    fn diagnostic_keeps_exact_reason_but_not_commands_or_all_boundaries() {
        let row = crate::LocalClearingRowV1 {
            row_id: "entry_row_00".into(),
            entry_id: "entry".into(),
            entry_physics_step: 100,
            template: pd_plan::local_clearing::LocalClearingPolicyV1::default()
                .templates()
                .unwrap()[0]
                .clone(),
            desired_thrust_acceleration_mps2: 13.32,
            physically_propagated: true,
            status: "rejected".into(),
            stop_reason: Some("physical_trace: body reserve 4 m below 5 m at tick 200".into()),
            stop_state: None,
            incoming_contact: None,
            minimum_clearance_m: Some(5.1),
            consumed_updates: Vec::new(),
            boundaries: vec![
                crate::LocalClearingBoundaryV1 {
                    handoff_physics_step: 160,
                    status: "insufficient_progress".into(),
                    reason: Some("handoff x 10 below 20".into()),
                },
                crate::LocalClearingBoundaryV1 {
                    handoff_physics_step: 162,
                    status: "unsafe_continuation".into(),
                    reason: Some("exact cutoff".into()),
                },
                crate::LocalClearingBoundaryV1 {
                    handoff_physics_step: 164,
                    status: "unsafe_continuation".into(),
                    reason: Some("later cutoff".into()),
                },
            ],
            selected_handoff_physics_step: None,
            selected_proposal_identity: None,
        };
        let diagnostic = WaypointV2RowDiagnostic::from_row(&row, 20.0);
        assert_eq!(diagnostic.stop_reason, row.stop_reason);
        assert_eq!(diagnostic.boundary_status_counts["unsafe_continuation"], 2);
        assert_eq!(
            diagnostic
                .first_continuation_rejection
                .as_ref()
                .unwrap()
                .handoff_physics_step,
            162
        );
        let value = serde_json::to_value(diagnostic).unwrap();
        assert!(value.get("consumed_updates").is_none());
        assert!(value.get("boundaries").is_none());
        assert!(value.get("eligible_handoff").is_none());
        let roundtrip: WaypointV2RowDiagnostic = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(roundtrip).unwrap(), value);
    }
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
