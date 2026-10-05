use std::collections::BTreeMap;

use pd_core::ScenarioSpec;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointV2PackGroup {
    Clear,
    Ordinary,
    AdditionalTerrain,
    Diagnostic,
}

impl WaypointV2PackGroup {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Ordinary => "ordinary",
            Self::AdditionalTerrain => "additional_terrain",
            Self::Diagnostic => "diagnostic",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2PackInput {
    pub case_id: String,
    pub source_set: String,
    pub source_group: String,
    pub group: WaypointV2PackGroup,
    pub family: String,
    pub base_case_id: String,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub expected_preflight: Option<String>,
    pub scenario: ScenarioSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaypointV2BatchReport {
    pub schema_id: String,
    pub pack_id: String,
    pub name: String,
    pub status: String,
    pub policy_version: u8,
    pub case_count: usize,
    pub cases: Vec<WaypointV2BatchCase>,
    pub summary: WaypointV2BatchSummary,
    pub input_identity: WaypointV2PackInputIdentity,
    pub pack_snapshot_sha256: String,
    pub expanded_inputs_snapshot_sha256: String,
    pub pack_snapshot_path: String,
    pub expanded_inputs_snapshot_path: String,
    pub provenance: WaypointV2BatchProvenance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaypointV2BatchCase {
    pub case_id: String,
    pub source_set: String,
    pub source_group: String,
    pub group: WaypointV2PackGroup,
    pub family: String,
    pub base_case_id: String,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub expected_preflight: Option<String>,
    pub status: String,
    pub outcome: Option<String>,
    pub planning_stop: Option<String>,
    pub reason: Option<String>,
    pub correction_count: Option<u32>,
    pub initial_nominal_terrain_blocked: Option<bool>,
    pub integrity_passed: Option<bool>,
    pub final_source_replay_passed: Option<bool>,
    pub physical_outcome: Option<String>,
    pub mission_outcome: Option<String>,
    pub planning_s: Option<f64>,
    pub execution_s: Option<f64>,
    pub replay_s: Option<f64>,
    pub input_path: String,
    pub scenario_path: String,
    pub flight_path: String,
    pub summary_path: String,
    pub rich_report_path: Option<String>,
    pub annotated_report_path: String,
    pub artifact_sha256: BTreeMap<String, String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2BatchSummary {
    pub clear_count: usize,
    pub ordinary_count: usize,
    pub additional_terrain_count: usize,
    pub diagnostic_count: usize,
    pub direct_landing_count: usize,
    pub corrected_landing_count: usize,
    pub valid_landing_count: usize,
    pub non_landing_count: usize,
    pub diagnostic_landing_count: usize,
    pub diagnostic_non_landing_count: usize,
    pub simulation_unverified_count: usize,
    pub unsupported_count: usize,
    pub crash_count: usize,
    pub integrity_passed_count: usize,
    pub integrity_failed_count: usize,
    pub final_source_replay_passed_count: usize,
    pub final_source_replay_failed_count: usize,
    pub initially_blocked_count: usize,
    pub planning_stops: BTreeMap<String, usize>,
    pub physical_outcomes: BTreeMap<String, usize>,
    pub mission_outcomes: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2PackInputIdentity {
    pub pack_file_sha256: String,
    pub source_fixture_sha256: BTreeMap<String, String>,
    pub source_manifest_sha256: BTreeMap<String, String>,
    /// SHA-256 of `serde_json::to_vec` over the ordered typed V2PackInput array.
    /// This Rust-native digest is not the JavaScript runner's canonical JSON seal.
    pub rust_typed_expanded_inputs_sha256: String,
    pub rust_typed_expanded_input_count: usize,
    pub digest_scheme: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaypointV2BatchProvenance {
    pub source_before: WaypointV2SourceState,
    pub source_after: Option<WaypointV2SourceState>,
    pub unchanged_during_capture: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2SourceState {
    pub git_commit: Option<String>,
    pub git_dirty: Option<bool>,
    pub rust_source_tree_sha256: String,
    pub executable_sha256: String,
}
