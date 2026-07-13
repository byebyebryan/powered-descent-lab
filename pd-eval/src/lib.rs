use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_control::{
    ControlledRunArtifacts, ControllerSpec, TelemetryValue, built_in_controller_spec, marker,
    metric, run_controller_spec,
};
use pd_core::{
    EndReason, EvaluationGoal, LandingPadSpec, MissionOutcome, Observation, RunContext,
    RunManifest, RunSummary, SampleRecord, ScenarioSpec, TerrainDefinition, TransferRouteSpec,
    TransferWaypointSpec, Vec2, VehicleSpec, WaypointHandoffKinematics,
};
use rayon::{ThreadPoolBuilder, prelude::*};
use serde::{Deserialize, Serialize};

pub mod report;

#[cfg(unix)]
use std::os::unix::fs as platform_fs;
#[cfg(windows)]
use std::os::windows::fs as platform_fs;

pub const BATCH_REPORT_SCHEMA_VERSION: u32 = 32;
const REGRESSION_POLICY_EPSILON: f64 = 1.0e-9;
const REGRESSION_POLICY_MEAN_SIM_TIME_WARN_DELTA_S: f64 = 1.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchCacheStatus {
    #[default]
    Fresh,
    Reused,
    Promoted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchCachePromotion {
    pub source_workspace_key: String,
    pub source_cache_dir: String,
    pub promoted_at_unix_s: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchCacheInfo {
    pub workspace_key: String,
    pub commit_key: String,
    pub batch_stem: String,
    pub cache_dir: String,
    pub status: BatchCacheStatus,
    pub created_at_unix_s: u64,
    #[serde(default)]
    pub promotion: Option<BatchCachePromotion>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchCompareSource {
    #[default]
    None,
    ExplicitDir,
    CacheRef,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchCompareResolutionStatus {
    #[default]
    NotRequested,
    Resolved,
    Missing,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchCompareProvenance {
    #[serde(default)]
    pub source: BatchCompareSource,
    #[serde(default)]
    pub requested_ref: Option<String>,
    #[serde(default)]
    pub resolved_ref: Option<String>,
    #[serde(default)]
    pub baseline_dir: Option<String>,
    #[serde(default)]
    pub status: BatchCompareResolutionStatus,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchProvenance {
    #[serde(default)]
    pub cache: Option<BatchCacheInfo>,
    #[serde(default)]
    pub compare: BatchCompareProvenance,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchMetricSummary {
    pub mean: f64,
    #[serde(default)]
    pub stddev: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchWaypointWindowEntryReviewMetrics {
    #[serde(default)]
    pub time_s: Option<f64>,
    #[serde(default)]
    pub position_x_m: Option<f64>,
    #[serde(default)]
    pub position_y_m: Option<f64>,
    #[serde(default)]
    pub velocity_x_mps: Option<f64>,
    #[serde(default)]
    pub velocity_y_mps: Option<f64>,
    #[serde(default)]
    pub distance_m: Option<f64>,
    #[serde(default)]
    pub cross_track_m: Option<f64>,
    #[serde(default)]
    pub plane_progress_m: Option<f64>,
    #[serde(default)]
    pub handoff_heading_error_rad: Option<f64>,
    #[serde(default)]
    pub handoff_progress_mps: Option<f64>,
    #[serde(default)]
    pub handoff_cross_speed_mps: Option<f64>,
    #[serde(default)]
    pub speed_mps: Option<f64>,
    #[serde(default)]
    pub vertical_speed_mps: Option<f64>,
    #[serde(default)]
    pub contract_pass: Option<bool>,
    #[serde(default)]
    pub contract_reasons: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchWaypointHandoffReviewMetrics {
    pub waypoint_index: usize,
    #[serde(default)]
    pub waypoint_id: Option<String>,
    #[serde(default)]
    pub capture_status: Option<String>,
    #[serde(default)]
    pub contract_status: Option<String>,
    #[serde(default)]
    pub contract_reasons: Vec<String>,
    #[serde(default)]
    pub capture_time_s: Option<f64>,
    #[serde(default)]
    pub window_entry: Option<BatchWaypointWindowEntryReviewMetrics>,
    #[serde(default)]
    pub resolution_reason: Option<String>,
    #[serde(default)]
    pub window_duration_s: Option<f64>,
    #[serde(default)]
    pub closest_distance_m: Option<f64>,
    #[serde(default)]
    pub distance_m: Option<f64>,
    #[serde(default)]
    pub cross_track_m: Option<f64>,
    #[serde(default)]
    pub plane_progress_m: Option<f64>,
    #[serde(default)]
    pub outbound_heading_error_rad: Option<f64>,
    #[serde(default)]
    pub outbound_progress_mps: Option<f64>,
    #[serde(default)]
    pub outbound_cross_speed_mps: Option<f64>,
    #[serde(default)]
    pub speed_mps: Option<f64>,
    #[serde(default)]
    pub vertical_speed_mps: Option<f64>,
    #[serde(default)]
    pub remaining_to_plane_m: Option<f64>,
    #[serde(default)]
    pub time_to_plane_s: Option<f64>,
    #[serde(default)]
    pub required_turn_distance_m: Option<f64>,
    #[serde(default)]
    pub shaping_start_distance_m: Option<f64>,
    #[serde(default)]
    pub turn_margin_m: Option<f64>,
    #[serde(default)]
    pub center_x_m: Option<f64>,
    #[serde(default)]
    pub center_y_m: Option<f64>,
    #[serde(default)]
    pub nominal_handoff_target_x_m: Option<f64>,
    #[serde(default)]
    pub nominal_handoff_target_y_m: Option<f64>,
    #[serde(default)]
    pub handoff_target_x_m: Option<f64>,
    #[serde(default)]
    pub handoff_target_y_m: Option<f64>,
    #[serde(default)]
    pub handoff_target_mode: Option<String>,
    #[serde(default)]
    pub remaining_to_handoff_m: Option<f64>,
    #[serde(default)]
    pub time_to_handoff_s: Option<f64>,
    #[serde(default)]
    pub target_vx_mps: Option<f64>,
    #[serde(default)]
    pub target_vy_mps: Option<f64>,
    #[serde(default)]
    pub target_deadline_remaining_s: Option<f64>,
    #[serde(default)]
    pub target_velocity_error_mps: Option<f64>,
    #[serde(default)]
    pub guidance_feasible: Option<bool>,
    #[serde(default)]
    pub predicted_handoff_time_to_go_s: Option<f64>,
    #[serde(default)]
    pub predicted_handoff_deadline_lead_s: Option<f64>,
    #[serde(default)]
    pub predicted_handoff_contract_status: Option<String>,
    #[serde(default)]
    pub predicted_handoff_contract_reasons: Vec<String>,
    #[serde(default)]
    pub predicted_handoff_distance_m: Option<f64>,
    #[serde(default)]
    pub predicted_handoff_cross_track_m: Option<f64>,
    #[serde(default)]
    pub predicted_handoff_plane_progress_m: Option<f64>,
    #[serde(default)]
    pub predicted_handoff_outbound_heading_error_rad: Option<f64>,
    #[serde(default)]
    pub predicted_handoff_outbound_progress_mps: Option<f64>,
    #[serde(default)]
    pub predicted_handoff_outbound_cross_speed_mps: Option<f64>,
    #[serde(default)]
    pub predicted_handoff_speed_mps: Option<f64>,
    #[serde(default)]
    pub predicted_handoff_vertical_speed_mps: Option<f64>,
    #[serde(default)]
    pub candidate_contract_pass_ever: Option<bool>,
    #[serde(default)]
    pub candidate_first_pass_time_s: Option<f64>,
    #[serde(default)]
    pub candidate_last_pass_time_s: Option<f64>,
    #[serde(default)]
    pub candidate_pass_lost_before_capture: Option<bool>,
    #[serde(default)]
    pub candidate_best_heading_margin_rad: Option<f64>,
    #[serde(default)]
    pub candidate_best_cross_speed_margin_mps: Option<f64>,
    #[serde(default)]
    pub reachable_candidate_contract_pass_ever: Option<bool>,
    #[serde(default)]
    pub reachable_candidate_first_pass_time_s: Option<f64>,
    #[serde(default)]
    pub reachable_candidate_last_pass_time_s: Option<f64>,
    #[serde(default)]
    pub reachable_candidate_pass_lost_before_capture: Option<bool>,
    #[serde(default)]
    pub reachable_required_accel_ratio_max: Option<f64>,
    #[serde(default)]
    pub reachable_thrust_saturated_time_max_s: Option<f64>,
    #[serde(default)]
    pub reachable_tilt_saturated_time_max_s: Option<f64>,
    #[serde(default)]
    pub continuation_next_waypoint_index: Option<usize>,
    #[serde(default)]
    pub continuation_contract_pass: Option<bool>,
    #[serde(default)]
    pub continuation_contract_reasons: Vec<String>,
    #[serde(default)]
    pub continuation_outbound_heading_error_rad: Option<f64>,
    #[serde(default)]
    pub continuation_required_accel_ratio_max: Option<f64>,
    #[serde(default)]
    pub continuation_passing_candidate_count: Option<usize>,
    #[serde(default)]
    pub transition_next_waypoint_index: Option<usize>,
    #[serde(default)]
    pub transition_position_error_m: Option<f64>,
    #[serde(default)]
    pub transition_velocity_error_mps: Option<f64>,
    #[serde(default)]
    pub transition_attitude_error_rad: Option<f64>,
    #[serde(default)]
    pub transition_mass_error_kg: Option<f64>,
    #[serde(default)]
    pub transition_fuel_error_kg: Option<f64>,
    #[serde(default)]
    pub transition_event_time_error_s: Option<f64>,
    #[serde(default)]
    pub transition_continuation_contract_pass: Option<bool>,
    #[serde(default)]
    pub transition_continuation_contract_reasons: Vec<String>,
    #[serde(default)]
    pub transition_continuation_outbound_heading_error_rad: Option<f64>,
    #[serde(default)]
    pub transition_continuation_required_accel_ratio_max: Option<f64>,
    #[serde(default)]
    pub transition_continuation_passing_candidate_count: Option<usize>,
    #[serde(default)]
    pub joint_next_waypoint_index: Option<usize>,
    #[serde(default)]
    pub joint_evaluated_candidate_count: Option<usize>,
    #[serde(default)]
    pub joint_passing_candidate_count: Option<usize>,
    #[serde(default)]
    pub joint_contract_pass: Option<bool>,
    #[serde(default)]
    pub joint_endpoint_x_m: Option<f64>,
    #[serde(default)]
    pub joint_endpoint_y_m: Option<f64>,
    #[serde(default)]
    pub joint_target_vx_mps: Option<f64>,
    #[serde(default)]
    pub joint_target_vy_mps: Option<f64>,
    #[serde(default)]
    pub joint_time_to_go_s: Option<f64>,
    #[serde(default)]
    pub joint_continuation_outbound_heading_error_rad: Option<f64>,
    #[serde(default)]
    pub joint_required_accel_ratio_max: Option<f64>,
    #[serde(default)]
    pub joint_total_saturated_time_s: Option<f64>,
    #[serde(default)]
    pub joint_continuation_passing_candidate_count: Option<usize>,
    #[serde(default)]
    pub plan_reference_position_error_max_m: Option<f64>,
    #[serde(default)]
    pub plan_reference_cross_error_max_abs_m: Option<f64>,
    #[serde(default)]
    pub plan_reference_velocity_error_max_mps: Option<f64>,
    #[serde(default)]
    pub plan_reference_cross_speed_error_max_abs_mps: Option<f64>,
    #[serde(default)]
    pub guidance_required_accel_ratio_max: Option<f64>,
    #[serde(default)]
    pub guidance_thrust_saturated_time_s: Option<f64>,
    #[serde(default)]
    pub guidance_tilt_saturated_time_s: Option<f64>,
    #[serde(default)]
    pub guidance_first_saturation_lead_s: Option<f64>,
    #[serde(default)]
    pub last_pass_reference_position_error_m: Option<f64>,
    #[serde(default)]
    pub last_pass_reference_velocity_error_mps: Option<f64>,
    #[serde(default)]
    pub last_pass_required_accel_ratio: Option<f64>,
    #[serde(default)]
    pub guidance_plan_revision_max: Option<i64>,
    #[serde(default)]
    pub guidance_plan_reasons: Vec<String>,
    #[serde(default)]
    pub handoff_turn_margin_m: Option<f64>,
    #[serde(default)]
    pub guidance_snapshot_source: Option<String>,
    #[serde(default)]
    pub guidance_snapshot_age_s: Option<f64>,
    #[serde(default)]
    pub guidance_replan_count: Option<i64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchRunReviewMetrics {
    #[serde(default)]
    pub fuel_used_pct_of_max: Option<f64>,
    #[serde(default)]
    pub landing_offset_abs_m: Option<f64>,
    #[serde(default)]
    pub low_altitude_dwell_s: Option<f64>,
    #[serde(default)]
    pub low_altitude_unsafe_recovery_s: Option<f64>,
    #[serde(default)]
    pub reference_gap_mean_m: Option<f64>,
    #[serde(default)]
    pub reference_gap_max_m: Option<f64>,
    #[serde(default)]
    pub transfer_shape_curve_rmse_m: Option<f64>,
    #[serde(default)]
    pub transfer_shape_apex_error_m: Option<f64>,
    #[serde(default)]
    pub transfer_shape_projected_dx_abs_mean_m: Option<f64>,
    #[serde(default)]
    pub transfer_shape_projected_dx_abs_max_m: Option<f64>,
    #[serde(default)]
    pub transfer_shape_shortfall_ratio: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_entry_kind: Option<String>,
    #[serde(default)]
    pub transfer_terminal_handoff_time_s: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_handoff_dx_m: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_handoff_height_m: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_handoff_speed_mps: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_handoff_gate_mode: Option<String>,
    #[serde(default)]
    pub transfer_terminal_handoff_projected_dx_m: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_handoff_impact_angle_deg: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_handoff_boost_quality: Option<String>,
    #[serde(default)]
    pub transfer_terminal_handoff_latest_safe_margin_s: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_handoff_required_accel_ratio: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_post_handoff_apex_gain_m: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_post_handoff_time_to_apex_s: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_post_handoff_apex_dx_abs_m: Option<f64>,
    #[serde(default)]
    pub transfer_final_phase: Option<String>,
    #[serde(default)]
    pub transfer_boost_projected_dx_m: Option<f64>,
    #[serde(default)]
    pub transfer_boost_impact_angle_deg: Option<f64>,
    #[serde(default)]
    pub transfer_boost_apex_over_target_m: Option<f64>,
    #[serde(default)]
    pub transfer_boost_quality: Option<String>,
    #[serde(default)]
    pub transfer_boost_selected_score: Option<f64>,
    #[serde(default)]
    pub transfer_boost_settled_quality: Option<String>,
    #[serde(default)]
    pub transfer_boost_settled_projected_dx_m: Option<f64>,
    #[serde(default)]
    pub transfer_boost_cutoff_time_s: Option<f64>,
    #[serde(default)]
    pub transfer_boost_cutoff_projected_dx_m: Option<f64>,
    #[serde(default)]
    pub transfer_boost_cutoff_impact_angle_deg: Option<f64>,
    #[serde(default)]
    pub transfer_boost_cutoff_apex_over_target_m: Option<f64>,
    #[serde(default)]
    pub transfer_boost_cutoff_quality: Option<String>,
    #[serde(default)]
    pub transfer_boost_burn_duration_s: Option<f64>,
    #[serde(default)]
    pub transfer_boost_burn_fuel_used_kg: Option<f64>,
    #[serde(default)]
    pub transfer_boost_burn_avg_throttle: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_gate_mode: Option<String>,
    #[serde(default)]
    pub transfer_terminal_gate_latest_safe_margin_s: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_gate_required_accel_ratio: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_gate_deferred: Option<bool>,
    #[serde(default)]
    pub transfer_corridor_mode: Option<String>,
    #[serde(default)]
    pub transfer_corridor_min_margin_m: Option<f64>,
    #[serde(default)]
    pub waypoint_capture_status: Option<String>,
    #[serde(default)]
    pub waypoint_contract_status: Option<String>,
    #[serde(default)]
    pub waypoint_contract_reasons: Vec<String>,
    #[serde(default)]
    pub waypoint_active_index: Option<i64>,
    #[serde(default)]
    pub waypoint_capture_time_s: Option<f64>,
    #[serde(default)]
    pub waypoint_window_entry: Option<BatchWaypointWindowEntryReviewMetrics>,
    #[serde(default)]
    pub waypoint_handoff_resolution_reason: Option<String>,
    #[serde(default)]
    pub waypoint_handoff_window_duration_s: Option<f64>,
    #[serde(default)]
    pub waypoint_closest_distance_m: Option<f64>,
    #[serde(default)]
    pub waypoint_distance_m: Option<f64>,
    #[serde(default)]
    pub waypoint_cross_track_m: Option<f64>,
    #[serde(default)]
    pub waypoint_plane_progress_m: Option<f64>,
    #[serde(default)]
    pub waypoint_outbound_heading_error_rad: Option<f64>,
    #[serde(default)]
    pub waypoint_outbound_progress_mps: Option<f64>,
    #[serde(default)]
    pub waypoint_outbound_cross_speed_mps: Option<f64>,
    #[serde(default)]
    pub waypoint_speed_mps: Option<f64>,
    #[serde(default)]
    pub waypoint_vertical_speed_mps: Option<f64>,
    #[serde(default)]
    pub waypoint_remaining_to_plane_m: Option<f64>,
    #[serde(default)]
    pub waypoint_time_to_plane_s: Option<f64>,
    #[serde(default)]
    pub waypoint_required_turn_distance_m: Option<f64>,
    #[serde(default)]
    pub waypoint_shaping_start_distance_m: Option<f64>,
    #[serde(default)]
    pub waypoint_turn_margin_m: Option<f64>,
    #[serde(default)]
    pub waypoint_handoffs: Vec<BatchWaypointHandoffReviewMetrics>,
    #[serde(default)]
    pub waypoint_route_status: Option<String>,
    #[serde(default)]
    pub waypoint_route_passed: Option<usize>,
    #[serde(default)]
    pub waypoint_route_total: Option<usize>,
    #[serde(default)]
    pub waypoint_route_first_failure_index: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchRunAnalyticClass {
    #[default]
    Scored,
    Impossible,
    Frontier,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchRunAnalyticReason {
    VerticalStopHeight,
    CoupledStopAcceleration,
    LowThrustHighEnergy,
    NearVerticalTransferRoute,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchRunAnalyticFeasibility {
    #[serde(default)]
    pub class: BatchRunAnalyticClass,
    #[serde(default)]
    pub reason: Option<BatchRunAnalyticReason>,
    #[serde(default)]
    pub available_stop_height_m: Option<f64>,
    #[serde(default)]
    pub required_stop_height_m: Option<f64>,
    #[serde(default)]
    pub stop_height_margin_m: Option<f64>,
    #[serde(default)]
    pub available_stop_accel_mps2: Option<f64>,
    #[serde(default)]
    pub required_stop_accel_mps2: Option<f64>,
    #[serde(default)]
    pub stop_accel_margin_mps2: Option<f64>,
}

impl BatchRunAnalyticFeasibility {
    pub(crate) fn is_scored(&self) -> bool {
        matches!(
            self.class,
            BatchRunAnalyticClass::Scored | BatchRunAnalyticClass::Frontier
        )
    }
}

fn default_selector_value() -> String {
    "unspecified".to_owned()
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SelectorAxes {
    #[serde(default = "default_selector_value")]
    pub mission: String,
    #[serde(default = "default_selector_value")]
    pub arrival_family: String,
    #[serde(default = "default_selector_value")]
    pub condition_set: String,
    #[serde(default = "default_selector_value")]
    pub vehicle_variant: String,
    #[serde(default = "default_selector_value")]
    pub arc_point: String,
    #[serde(default = "default_selector_value")]
    pub velocity_band: String,
    #[serde(default = "default_selector_value")]
    pub route_family: String,
    #[serde(default = "default_selector_value")]
    pub route_angle: String,
    #[serde(default = "default_selector_value")]
    pub radius_tier: String,
    #[serde(default = "default_selector_value")]
    pub waypoint_profile: String,
    #[serde(default = "default_selector_value")]
    pub waypoint_handoff_envelope: String,
    #[serde(default)]
    pub expectation_tier: Option<String>,
}

impl Default for SelectorAxes {
    fn default() -> Self {
        Self {
            mission: default_selector_value(),
            arrival_family: default_selector_value(),
            condition_set: default_selector_value(),
            vehicle_variant: default_selector_value(),
            arc_point: default_selector_value(),
            velocity_band: default_selector_value(),
            route_family: default_selector_value(),
            route_angle: default_selector_value(),
            radius_tier: default_selector_value(),
            waypoint_profile: default_selector_value(),
            waypoint_handoff_envelope: default_selector_value(),
            expectation_tier: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioPackSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub terminal_matrix_max_time_s: Option<f64>,
    pub entries: Vec<ScenarioPackEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScenarioPackEntry {
    Scenario(ConcreteScenarioPackEntry),
    Family(ScenarioFamilyEntry),
    TerminalMatrix(TerminalMatrixEntry),
    TransferMatrix(TransferMatrixEntry),
}

impl ScenarioPackEntry {
    fn id(&self) -> &str {
        match self {
            Self::Scenario(entry) => &entry.id,
            Self::Family(entry) => &entry.id,
            Self::TerminalMatrix(entry) => &entry.id,
            Self::TransferMatrix(entry) => &entry.id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConcreteScenarioPackEntry {
    pub id: String,
    pub scenario: String,
    pub controller: String,
    #[serde(default)]
    pub controller_config: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioFamilyEntry {
    pub id: String,
    pub family: String,
    pub base_scenario: String,
    pub controller: String,
    #[serde(default)]
    pub controller_config: Option<String>,
    #[serde(default)]
    pub seeds: Vec<u64>,
    #[serde(default)]
    pub seed_range: Option<SeedRangeSpec>,
    #[serde(default)]
    pub perturbations: Vec<NumericPerturbationSpec>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalMatrixEntry {
    pub id: String,
    pub terminal_matrix: String,
    pub base_scenario: String,
    pub lanes: Vec<TerminalMatrixLaneSpec>,
    pub seed_tier: TerminalSeedTier,
    pub condition_set: String,
    pub vehicle_variant: String,
    pub expectation_tier: String,
    #[serde(default)]
    pub arc_points: Vec<String>,
    #[serde(default)]
    pub adjustments: Vec<NumericAdjustmentSpec>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalMatrixLaneSpec {
    pub id: String,
    pub controller: String,
    #[serde(default)]
    pub controller_config: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalSeedTier {
    Smoke,
    Full,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferMatrixEvaluationGoal {
    #[default]
    LandingOnPad,
    WaypointHandoff,
    WaypointSequence,
}

impl TransferMatrixEvaluationGoal {
    fn as_str(self) -> &'static str {
        match self {
            Self::LandingOnPad => "landing_on_pad",
            Self::WaypointHandoff => "waypoint_handoff",
            Self::WaypointSequence => "waypoint_sequence",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferMatrixEntry {
    pub id: String,
    pub transfer_matrix: String,
    pub base_scenario: String,
    pub lanes: Vec<TransferMatrixLaneSpec>,
    pub seed_tier: TransferSeedTier,
    pub vehicle_variant: String,
    pub expectation_tier: String,
    #[serde(default)]
    pub route_angles: Vec<String>,
    #[serde(default)]
    pub radius_tiers: Vec<String>,
    #[serde(default)]
    pub waypoint_profile: Option<String>,
    #[serde(default)]
    pub waypoint_handoff_envelope: Option<String>,
    #[serde(default)]
    pub evaluation_goal: TransferMatrixEvaluationGoal,
    #[serde(default)]
    pub adjustments: Vec<NumericAdjustmentSpec>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferMatrixLaneSpec {
    pub id: String,
    pub controller: String,
    #[serde(default)]
    pub controller_config: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferSeedTier {
    Smoke,
    Full,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NumericAdjustmentSpec {
    pub id: String,
    pub path: String,
    pub mode: NumericPerturbationMode,
    pub value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedRangeSpec {
    pub start: u64,
    pub count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NumericPerturbationSpec {
    pub id: String,
    pub path: String,
    pub mode: NumericPerturbationMode,
    pub min: f64,
    pub max: f64,
    #[serde(default)]
    pub quantize: Option<f64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericPerturbationMode {
    Set,
    Offset,
    Scale,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedRunSourceKind {
    ConcreteScenario,
    FamilySweep,
    TerminalMatrix,
    TransferMatrix,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedRunDescriptor {
    pub run_id: String,
    pub entry_id: String,
    pub source_kind: ResolvedRunSourceKind,
    pub scenario_source: String,
    pub resolved_scenario_id: String,
    pub resolved_scenario_name: String,
    pub family_id: Option<String>,
    #[serde(default)]
    pub selector: SelectorAxes,
    #[serde(default)]
    pub lane_id: String,
    pub resolved_seed: u64,
    pub resolved_parameters: BTreeMap<String, f64>,
    pub controller_id: String,
    pub controller_spec: ControllerSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchRunRecord {
    pub resolved: ResolvedRunDescriptor,
    pub manifest: RunManifest,
    #[serde(default)]
    pub review: BatchRunReviewMetrics,
    #[serde(default)]
    pub analytic: BatchRunAnalyticFeasibility,
    pub bundle_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchIdentity {
    pub schema_version: u32,
    pub pack_spec_digest: String,
    pub resolved_run_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchGroupSummary {
    pub key: String,
    pub total_runs: usize,
    pub success_runs: usize,
    pub failure_runs: usize,
    #[serde(default)]
    pub invalidated_runs: usize,
    pub mean_sim_time_s: f64,
    #[serde(default)]
    pub sim_time_stats: Option<BatchMetricSummary>,
    #[serde(default)]
    pub mean_success_fuel_remaining_kg: Option<f64>,
    #[serde(default)]
    pub fuel_used_pct_of_max: Option<BatchMetricSummary>,
    #[serde(default)]
    pub landing_offset_abs_m: Option<BatchMetricSummary>,
    #[serde(default)]
    pub low_altitude_dwell_s: Option<BatchMetricSummary>,
    #[serde(default)]
    pub low_altitude_unsafe_recovery_s: Option<BatchMetricSummary>,
    #[serde(default)]
    pub reference_gap_mean_m: Option<BatchMetricSummary>,
    #[serde(default)]
    pub transfer_shape_curve_rmse_m: Option<BatchMetricSummary>,
    #[serde(default)]
    pub transfer_shape_apex_error_m: Option<BatchMetricSummary>,
    #[serde(default)]
    pub transfer_shape_projected_dx_abs_mean_m: Option<BatchMetricSummary>,
    #[serde(default)]
    pub transfer_shape_shortfall_ratio: Option<BatchMetricSummary>,
    #[serde(default)]
    pub transfer_terminal_post_handoff_apex_gain_m: Option<BatchMetricSummary>,
    #[serde(default)]
    pub transfer_terminal_post_handoff_time_to_apex_s: Option<BatchMetricSummary>,
    #[serde(default)]
    pub transfer_terminal_post_handoff_apex_dx_abs_m: Option<BatchMetricSummary>,
    pub mission_outcomes: BTreeMap<String, usize>,
    pub end_reasons: BTreeMap<String, usize>,
    pub sample_run_ids: Vec<String>,
    pub failed_seeds: Vec<u64>,
    #[serde(default)]
    pub weakest_success_run_id: Option<String>,
    #[serde(default)]
    pub closest_failure_run_id: Option<String>,
    #[serde(default)]
    pub worst_failure_run_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchRunPointer {
    pub run_id: String,
    pub entry_id: String,
    pub family_id: Option<String>,
    #[serde(default)]
    pub selector: SelectorAxes,
    #[serde(default)]
    pub lane_id: String,
    pub scenario_id: String,
    pub scenario_seed: u64,
    pub controller_id: String,
    pub mission_outcome: String,
    pub end_reason: String,
    pub sim_time_s: f64,
    pub bundle_dir: Option<String>,
    #[serde(default)]
    pub margin_ratio: Option<f64>,
    #[serde(default)]
    pub fuel_remaining_kg: f64,
    #[serde(default)]
    pub review: BatchRunReviewMetrics,
    #[serde(default)]
    pub analytic: BatchRunAnalyticFeasibility,
    #[serde(default)]
    pub summary: RunSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchSummary {
    pub total_runs: usize,
    pub success_runs: usize,
    pub failure_runs: usize,
    #[serde(default)]
    pub invalidated_runs: usize,
    pub mean_sim_time_s: f64,
    pub max_sim_time_s: f64,
    pub mission_outcomes: BTreeMap<String, usize>,
    pub physical_outcomes: BTreeMap<String, usize>,
    pub end_reasons: BTreeMap<String, usize>,
    pub by_entry: Vec<BatchGroupSummary>,
    pub by_family: Vec<BatchGroupSummary>,
    pub failed_runs: Vec<BatchRunPointer>,
    pub slowest_runs: Vec<BatchRunPointer>,
    #[serde(default)]
    pub closest_failures: Vec<BatchRunPointer>,
    #[serde(default)]
    pub worst_failures: Vec<BatchRunPointer>,
    #[serde(default)]
    pub weakest_successes: Vec<BatchRunPointer>,
    #[serde(default)]
    pub lowest_fuel_successes: Vec<BatchRunPointer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchReport {
    pub schema_version: u32,
    pub pack_id: String,
    pub pack_name: String,
    pub total_runs: usize,
    #[serde(default)]
    pub wall_clock_s: f64,
    pub workers_requested: usize,
    pub workers_used: usize,
    pub identity: BatchIdentity,
    #[serde(default)]
    pub provenance: BatchProvenance,
    pub resolved_runs: Vec<ResolvedRunDescriptor>,
    pub records: Vec<BatchRunRecord>,
    pub summary: BatchSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchCacheMeta {
    pub schema_version: u32,
    pub pack_id: String,
    pub pack_name: String,
    pub identity: BatchIdentity,
    pub total_runs: usize,
    pub workers_used: usize,
    pub cache: BatchCacheInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchCompareBasis {
    pub mode: String,
    pub shared_runs: usize,
    pub candidate_only_runs: usize,
    pub baseline_only_runs: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchSummaryDelta {
    pub candidate_success_rate: f64,
    pub baseline_success_rate: f64,
    pub success_rate_delta: f64,
    pub candidate_success_runs: usize,
    pub baseline_success_runs: usize,
    pub success_runs_delta: i64,
    pub candidate_failure_runs: usize,
    pub baseline_failure_runs: usize,
    pub failure_runs_delta: i64,
    #[serde(default)]
    pub candidate_invalidated_runs: usize,
    #[serde(default)]
    pub baseline_invalidated_runs: usize,
    #[serde(default)]
    pub invalidated_runs_delta: i64,
    pub candidate_mean_sim_time_s: f64,
    pub baseline_mean_sim_time_s: f64,
    pub mean_sim_time_delta_s: f64,
    pub candidate_max_sim_time_s: f64,
    pub baseline_max_sim_time_s: f64,
    pub max_sim_time_delta_s: f64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchRegressionPolicyStatus {
    #[default]
    Pass,
    Warn,
    Fail,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchRegressionPolicyRuleResult {
    pub id: String,
    pub label: String,
    pub status: BatchRegressionPolicyStatus,
    pub observed: String,
    pub threshold: String,
    pub note: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchRegressionPolicyEvaluation {
    pub status: BatchRegressionPolicyStatus,
    pub summary: String,
    pub rules: Vec<BatchRegressionPolicyRuleResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchGroupComparison {
    pub key: String,
    pub candidate_total_runs: Option<usize>,
    pub baseline_total_runs: Option<usize>,
    pub candidate_success_rate: Option<f64>,
    pub baseline_success_rate: Option<f64>,
    pub success_rate_delta: Option<f64>,
    pub candidate_failure_runs: Option<usize>,
    pub baseline_failure_runs: Option<usize>,
    pub failure_runs_delta: Option<i64>,
    #[serde(default)]
    pub candidate_invalidated_runs: Option<usize>,
    #[serde(default)]
    pub baseline_invalidated_runs: Option<usize>,
    #[serde(default)]
    pub invalidated_runs_delta: Option<i64>,
    pub candidate_mean_sim_time_s: Option<f64>,
    pub baseline_mean_sim_time_s: Option<f64>,
    pub mean_sim_time_delta_s: Option<f64>,
    pub candidate_failed_seeds: Vec<u64>,
    pub baseline_failed_seeds: Vec<u64>,
    pub sample_run_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchRunChangeKind {
    NewFailure,
    Recovered,
    OutcomeChanged,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchRunComparison {
    pub run_id: String,
    pub entry_id: String,
    pub family_id: Option<String>,
    #[serde(default)]
    pub selector: SelectorAxes,
    #[serde(default)]
    pub lane_id: String,
    pub change_kind: BatchRunChangeKind,
    pub candidate_seed: u64,
    pub baseline_seed: u64,
    pub candidate_mission_outcome: String,
    pub baseline_mission_outcome: String,
    pub candidate_end_reason: String,
    pub baseline_end_reason: String,
    pub candidate_sim_time_s: f64,
    pub baseline_sim_time_s: f64,
    pub sim_time_delta_s: f64,
    pub candidate_bundle_dir: Option<String>,
    pub baseline_bundle_dir: Option<String>,
    #[serde(default)]
    pub candidate_margin_ratio: Option<f64>,
    #[serde(default)]
    pub baseline_margin_ratio: Option<f64>,
    #[serde(default)]
    pub margin_ratio_delta: Option<f64>,
    #[serde(default)]
    pub candidate_fuel_remaining_kg: f64,
    #[serde(default)]
    pub baseline_fuel_remaining_kg: f64,
    #[serde(default)]
    pub fuel_remaining_delta_kg: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchComparison {
    pub candidate_pack_id: String,
    pub candidate_pack_name: String,
    pub baseline_pack_id: String,
    pub baseline_pack_name: String,
    pub basis: BatchCompareBasis,
    pub summary: BatchSummaryDelta,
    #[serde(default)]
    pub policy: BatchRegressionPolicyEvaluation,
    pub by_entry: Vec<BatchGroupComparison>,
    pub by_family: Vec<BatchGroupComparison>,
    pub regressions: Vec<BatchRunComparison>,
    pub improvements: Vec<BatchRunComparison>,
    pub outcome_changes: Vec<BatchRunComparison>,
    pub candidate_only: Vec<BatchRunPointer>,
    pub baseline_only: Vec<BatchRunPointer>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MissingComparePolicy {
    #[default]
    Skip,
    Error,
}

#[derive(Clone, Debug)]
pub struct ResolvedBaselineReport {
    pub dir: PathBuf,
    pub report: BatchReport,
}

#[derive(Clone, Debug)]
pub struct CachedBatchRunOutcome {
    pub report: BatchReport,
    pub baseline: Option<ResolvedBaselineReport>,
    pub cache_dir: PathBuf,
}

#[derive(Clone, Debug)]
struct WorkspaceState {
    commit_key: String,
    workspace_key: String,
    dirty: bool,
}

#[derive(Clone, Debug)]
struct ResolvedBatchRun {
    descriptor: ResolvedRunDescriptor,
    scenario: ScenarioSpec,
}

pub fn load_pack(path: &Path) -> Result<ScenarioPackSpec> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario pack file {}", path.display()))?;
    let pack: ScenarioPackSpec = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse scenario pack json {}", path.display()))?;
    validate_pack(&pack)?;
    Ok(pack)
}

pub fn run_pack_file(path: &Path, output_dir: Option<&Path>) -> Result<BatchReport> {
    run_pack_file_with_workers(path, output_dir, 1)
}

pub fn load_batch_report(path: &Path) -> Result<BatchReport> {
    let summary_path = if path.is_dir() {
        path.join("summary.json")
    } else {
        path.to_path_buf()
    };
    let raw = fs::read_to_string(&summary_path)
        .with_context(|| format!("failed to read batch report {}", summary_path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse batch report {}", summary_path.display()))
}

pub fn run_pack_file_cached(
    path: &Path,
    output_dir: Option<&Path>,
    workers: usize,
    compare_ref: Option<&str>,
    baseline_dir: Option<&Path>,
    missing_compare: MissingComparePolicy,
    reuse_cache: bool,
) -> Result<CachedBatchRunOutcome> {
    let pack = load_pack(path)?;
    let base_dir = path
        .parent()
        .ok_or_else(|| anyhow!("pack path has no parent directory"))?;
    run_pack_cached(
        &pack,
        base_dir,
        output_dir,
        workers,
        compare_ref,
        baseline_dir,
        missing_compare,
        reuse_cache,
    )
}

pub fn resolve_pack_compare_baseline(
    path: &Path,
    compare_ref: Option<&str>,
    baseline_dir: Option<&Path>,
    missing_compare: MissingComparePolicy,
) -> Result<Option<ResolvedBaselineReport>> {
    let pack = load_pack(path)?;
    let base_dir = path
        .parent()
        .ok_or_else(|| anyhow!("pack path has no parent directory"))?;
    validate_pack(&pack)?;
    let resolved_runs = resolve_pack_runs(&pack, base_dir)?;
    let identity = batch_identity_for_pack(&pack, &resolved_runs)?;
    let workspace = current_workspace_state()?;
    let requested_compare =
        resolve_compare_provenance(baseline_dir, compare_ref, missing_compare, &workspace)?;
    let (_, baseline) =
        load_requested_baseline(&pack, &identity, requested_compare, missing_compare)?;
    Ok(baseline)
}

pub fn promote_pack_cache(
    path: &Path,
    source_workspace_key: Option<&str>,
    target_ref: &str,
) -> Result<PathBuf> {
    let pack = load_pack(path)?;
    let base_dir = path
        .parent()
        .ok_or_else(|| anyhow!("pack path has no parent directory"))?;
    validate_pack(&pack)?;
    let resolved_runs = resolve_pack_runs(&pack, base_dir)?;
    let identity = batch_identity_for_pack(&pack, &resolved_runs)?;
    let batch_stem = batch_cache_stem(&pack.id, &identity);
    let workspace = current_workspace_state()?;
    let target_commit_key = git_commit_key_for_ref(target_ref)?;
    let source_key = if let Some(source_workspace_key) = source_workspace_key {
        source_workspace_key.to_owned()
    } else if workspace.dirty
        && cache_dir_for_batch_key(&workspace.workspace_key, &batch_stem).exists()
    {
        workspace.workspace_key.clone()
    } else {
        find_latest_dirty_workspace_key(&target_commit_key, &batch_stem)?.ok_or_else(|| {
            anyhow!(
                "no dirty cache found for commit '{}' and batch '{}'",
                target_commit_key,
                batch_stem
            )
        })?
    };
    let source_dir = cache_dir_for_batch_key(&source_key, &batch_stem);
    let target_dir = cache_dir_for_batch_key(&target_commit_key, &batch_stem);
    if source_dir == target_dir {
        bail!(
            "source cache {} and target cache {} are identical",
            source_dir.display(),
            target_dir.display()
        );
    }

    let mut report = validate_cached_batch_dir(&source_dir, &pack, &identity)?
        .ok_or_else(|| anyhow!("no reusable cache found at {}", source_dir.display()))?;
    let source_cache = report.provenance.cache.clone().ok_or_else(|| {
        anyhow!(
            "cached batch at {} is missing cache provenance",
            source_dir.display()
        )
    })?;

    if target_dir.exists() {
        fs::remove_dir_all(&target_dir).with_context(|| {
            format!(
                "failed to remove existing promoted cache {}",
                target_dir.display()
            )
        })?;
    }
    copy_dir_recursive(&source_dir, &target_dir)?;
    rewrite_report_bundle_dirs(&mut report, &source_dir, &target_dir);
    report.provenance.compare = BatchCompareProvenance::default();
    report.provenance.cache = Some(BatchCacheInfo {
        workspace_key: target_commit_key.clone(),
        commit_key: target_commit_key.clone(),
        batch_stem,
        cache_dir: target_dir.to_string_lossy().into_owned(),
        status: BatchCacheStatus::Promoted,
        created_at_unix_s: current_unix_timestamp(),
        promotion: Some(BatchCachePromotion {
            source_workspace_key: source_key,
            source_cache_dir: source_cache.cache_dir,
            promoted_at_unix_s: current_unix_timestamp(),
        }),
    });
    write_batch_cache_dir(&target_dir, &pack, &report, false)?;
    Ok(target_dir)
}

pub fn compare_batch_reports(candidate: &BatchReport, baseline: &BatchReport) -> BatchComparison {
    let (candidate_scope_records, baseline_scope_records) =
        comparison_record_scope(candidate, baseline);
    let candidate_records = candidate_scope_records
        .iter()
        .map(|record| (record.resolved.run_id.clone(), *record))
        .collect::<BTreeMap<_, _>>();
    let baseline_records = baseline_scope_records
        .iter()
        .map(|record| (record.resolved.run_id.clone(), *record))
        .collect::<BTreeMap<_, _>>();

    let mut shared_run_ids = Vec::new();
    let mut candidate_only = Vec::new();
    for (run_id, record) in &candidate_records {
        if baseline_records.contains_key(run_id) {
            shared_run_ids.push(run_id.clone());
        } else {
            candidate_only.push(run_pointer(record));
        }
    }

    let mut baseline_only = Vec::new();
    for (run_id, record) in &baseline_records {
        if !candidate_records.contains_key(run_id) {
            baseline_only.push(run_pointer(record));
        }
    }

    let mut regressions = Vec::new();
    let mut improvements = Vec::new();
    let mut outcome_changes = Vec::new();
    for run_id in shared_run_ids.iter().cloned() {
        let candidate_record = candidate_records
            .get(&run_id)
            .expect("shared run ids should exist in candidate map");
        let baseline_record = baseline_records
            .get(&run_id)
            .expect("shared run ids should exist in baseline map");
        if let Some(comparison) = compare_run_pair(candidate_record, baseline_record) {
            match comparison.change_kind {
                BatchRunChangeKind::NewFailure => regressions.push(comparison),
                BatchRunChangeKind::Recovered => improvements.push(comparison),
                BatchRunChangeKind::OutcomeChanged => outcome_changes.push(comparison),
            }
        }
    }

    regressions.sort_by(run_comparison_sort_key);
    improvements.sort_by(run_comparison_sort_key);
    outcome_changes.sort_by(run_comparison_sort_key);
    candidate_only.sort_by(run_pointer_sort_key);
    baseline_only.sort_by(run_pointer_sort_key);

    let basis = BatchCompareBasis {
        mode: "run_id".to_owned(),
        shared_runs: shared_run_ids.len(),
        candidate_only_runs: candidate_only.len(),
        baseline_only_runs: baseline_only.len(),
    };
    let candidate_summary = summarize_record_refs(candidate_scope_records.as_slice());
    let baseline_summary = summarize_record_refs(baseline_scope_records.as_slice());
    let summary = compare_summary_delta(&candidate_summary, &baseline_summary);
    let policy = evaluate_regression_policy(&summary, &basis, &regressions);

    BatchComparison {
        candidate_pack_id: candidate.pack_id.clone(),
        candidate_pack_name: candidate.pack_name.clone(),
        baseline_pack_id: baseline.pack_id.clone(),
        baseline_pack_name: baseline.pack_name.clone(),
        basis,
        summary,
        policy,
        by_entry: compare_group_sets(&candidate_summary.by_entry, &baseline_summary.by_entry),
        by_family: compare_group_sets(&candidate_summary.by_family, &baseline_summary.by_family),
        regressions,
        improvements,
        outcome_changes,
        candidate_only,
        baseline_only,
    }
}

fn comparison_record_scope<'a, 'b>(
    candidate: &'a BatchReport,
    baseline: &'b BatchReport,
) -> (Vec<&'a BatchRunRecord>, Vec<&'b BatchRunRecord>) {
    let candidate_lane_id = preferred_compare_lane_id(&candidate.records);
    let baseline_lane_id = preferred_compare_lane_id(&baseline.records);
    if let (Some(candidate_lane_id), Some(baseline_lane_id)) = (candidate_lane_id, baseline_lane_id)
    {
        (
            records_for_lane(&candidate.records, candidate_lane_id),
            records_for_lane(&baseline.records, baseline_lane_id),
        )
    } else {
        (
            candidate.records.iter().collect(),
            baseline.records.iter().collect(),
        )
    }
}

fn preferred_compare_lane_id(records: &[BatchRunRecord]) -> Option<&'static str> {
    if records
        .iter()
        .any(|record| record.resolved.lane_id == "current")
    {
        Some("current")
    } else if records
        .iter()
        .any(|record| record.resolved.lane_id == "staged")
    {
        Some("staged")
    } else {
        None
    }
}

fn records_for_lane<'a>(records: &'a [BatchRunRecord], lane_id: &str) -> Vec<&'a BatchRunRecord> {
    records
        .iter()
        .filter(|record| record.resolved.lane_id == lane_id)
        .collect()
}

fn summarize_record_refs(records: &[&BatchRunRecord]) -> BatchSummary {
    let owned_records = records
        .iter()
        .map(|record| (*record).clone())
        .collect::<Vec<_>>();
    summarize_records(&owned_records)
}

fn evaluate_regression_policy(
    summary: &BatchSummaryDelta,
    basis: &BatchCompareBasis,
    regressions: &[BatchRunComparison],
) -> BatchRegressionPolicyEvaluation {
    let mut rules = Vec::new();
    let has_exact_coverage = basis.candidate_only_runs == 0 && basis.baseline_only_runs == 0;

    rules.push(regression_policy_rule(
        "new_failures",
        "New shared-run failures",
        if regressions.is_empty() {
            BatchRegressionPolicyStatus::Pass
        } else {
            BatchRegressionPolicyStatus::Fail
        },
        regressions.len().to_string(),
        "0",
        "shared runs must not move from success to failure",
    ));
    rules.push(regression_policy_rule(
        "scored_failure_delta",
        "Scored failure delta",
        if summary.failure_runs_delta > 0 {
            if has_exact_coverage {
                BatchRegressionPolicyStatus::Fail
            } else {
                BatchRegressionPolicyStatus::Warn
            }
        } else {
            BatchRegressionPolicyStatus::Pass
        },
        format!("{:+}", summary.failure_runs_delta),
        "<= 0 with exact coverage",
        "total scored failures must not increase when run sets match",
    ));
    rules.push(regression_policy_rule(
        "success_rate_delta",
        "Scored success-rate delta",
        if summary.success_rate_delta < -REGRESSION_POLICY_EPSILON {
            if has_exact_coverage {
                BatchRegressionPolicyStatus::Fail
            } else {
                BatchRegressionPolicyStatus::Warn
            }
        } else {
            BatchRegressionPolicyStatus::Pass
        },
        format_policy_percent_delta(summary.success_rate_delta),
        ">= 0.00 pp with exact coverage",
        "scored success rate must not drop when run sets match",
    ));
    rules.push(regression_policy_rule(
        "mean_sim_time_delta",
        "Mean sim-time delta",
        if summary.mean_sim_time_delta_s > REGRESSION_POLICY_MEAN_SIM_TIME_WARN_DELTA_S {
            BatchRegressionPolicyStatus::Warn
        } else {
            BatchRegressionPolicyStatus::Pass
        },
        format_policy_seconds_delta(summary.mean_sim_time_delta_s),
        format!(
            "<= {}",
            format_policy_seconds_delta(REGRESSION_POLICY_MEAN_SIM_TIME_WARN_DELTA_S)
        ),
        "mean simulated time should not drift upward materially",
    ));
    rules.push(regression_policy_rule(
        "invalidated_delta",
        "Invalidated-run delta",
        if summary.invalidated_runs_delta > 0 {
            BatchRegressionPolicyStatus::Warn
        } else {
            BatchRegressionPolicyStatus::Pass
        },
        format!("{:+}", summary.invalidated_runs_delta),
        "<= 0",
        "new invalidations can hide scored regressions",
    ));
    rules.push(regression_policy_rule(
        "compare_coverage",
        "Compare coverage",
        if basis.candidate_only_runs > 0 || basis.baseline_only_runs > 0 {
            BatchRegressionPolicyStatus::Warn
        } else {
            BatchRegressionPolicyStatus::Pass
        },
        format!(
            "current-only {}, baseline-only {}",
            basis.candidate_only_runs, basis.baseline_only_runs
        ),
        "0 unmatched runs",
        "policy is strongest when the candidate and baseline run sets match",
    ));

    let status = regression_policy_status_from_rules(&rules);
    let summary_text = regression_policy_summary(status, &rules);

    BatchRegressionPolicyEvaluation {
        status,
        summary: summary_text,
        rules,
    }
}

fn regression_policy_rule(
    id: impl Into<String>,
    label: impl Into<String>,
    status: BatchRegressionPolicyStatus,
    observed: impl Into<String>,
    threshold: impl Into<String>,
    note: impl Into<String>,
) -> BatchRegressionPolicyRuleResult {
    BatchRegressionPolicyRuleResult {
        id: id.into(),
        label: label.into(),
        status,
        observed: observed.into(),
        threshold: threshold.into(),
        note: note.into(),
    }
}

fn regression_policy_status_from_rules(
    rules: &[BatchRegressionPolicyRuleResult],
) -> BatchRegressionPolicyStatus {
    if rules
        .iter()
        .any(|rule| rule.status == BatchRegressionPolicyStatus::Fail)
    {
        BatchRegressionPolicyStatus::Fail
    } else if rules
        .iter()
        .any(|rule| rule.status == BatchRegressionPolicyStatus::Warn)
    {
        BatchRegressionPolicyStatus::Warn
    } else {
        BatchRegressionPolicyStatus::Pass
    }
}

fn regression_policy_summary(
    status: BatchRegressionPolicyStatus,
    rules: &[BatchRegressionPolicyRuleResult],
) -> String {
    let failures = rules
        .iter()
        .filter(|rule| rule.status == BatchRegressionPolicyStatus::Fail)
        .count();
    let warnings = rules
        .iter()
        .filter(|rule| rule.status == BatchRegressionPolicyStatus::Warn)
        .count();
    match status {
        BatchRegressionPolicyStatus::Pass => "passed all regression thresholds".to_owned(),
        BatchRegressionPolicyStatus::Warn => {
            format!(
                "passed required thresholds with {}",
                plural_count(warnings, "warning", "warnings")
            )
        }
        BatchRegressionPolicyStatus::Fail => {
            if warnings > 0 {
                format!(
                    "failed {} with {}",
                    plural_count(failures, "required threshold", "required thresholds"),
                    plural_count(warnings, "warning", "warnings")
                )
            } else {
                format!(
                    "failed {}",
                    plural_count(failures, "required threshold", "required thresholds")
                )
            }
        }
    }
}

fn plural_count(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

fn format_policy_percent_delta(value: f64) -> String {
    format!("{:+.2} pp", value * 100.0)
}

fn format_policy_seconds_delta(value: f64) -> String {
    format!("{value:+.2}s")
}

pub fn run_pack_cached(
    pack: &ScenarioPackSpec,
    base_dir: &Path,
    output_dir: Option<&Path>,
    workers: usize,
    compare_ref: Option<&str>,
    baseline_dir: Option<&Path>,
    missing_compare: MissingComparePolicy,
    reuse_cache: bool,
) -> Result<CachedBatchRunOutcome> {
    validate_pack(pack)?;

    let resolved_runs = resolve_pack_runs(pack, base_dir)?;
    let requested_workers = workers.max(1);
    let workers_used = effective_worker_count(requested_workers, resolved_runs.len());
    let identity = batch_identity_for_pack(pack, &resolved_runs)?;
    let workspace = current_workspace_state()?;
    let batch_stem = batch_cache_stem(&pack.id, &identity);
    let cache_dir = cache_dir_for_batch_key(&workspace.workspace_key, &batch_stem);

    let base_cache_report = if reuse_cache {
        validate_cached_batch_dir(&cache_dir, pack, &identity)?
    } else {
        None
    };

    let cache_report = if let Some(mut report) = base_cache_report {
        if let Some(cache) = report.provenance.cache.as_mut() {
            cache.status = BatchCacheStatus::Reused;
        }
        report
    } else {
        let started = Instant::now();
        let records = execute_resolved_runs(&resolved_runs, Some(&cache_dir), workers_used)?;
        let report = BatchReport {
            schema_version: BATCH_REPORT_SCHEMA_VERSION,
            pack_id: pack.id.clone(),
            pack_name: pack.name.clone(),
            total_runs: records.len(),
            wall_clock_s: started.elapsed().as_secs_f64(),
            workers_requested: requested_workers,
            workers_used,
            identity: identity.clone(),
            provenance: BatchProvenance {
                cache: Some(BatchCacheInfo {
                    workspace_key: workspace.workspace_key.clone(),
                    commit_key: workspace.commit_key.clone(),
                    batch_stem: batch_stem.clone(),
                    cache_dir: cache_dir.to_string_lossy().into_owned(),
                    status: BatchCacheStatus::Fresh,
                    created_at_unix_s: current_unix_timestamp(),
                    promotion: None,
                }),
                compare: BatchCompareProvenance::default(),
            },
            resolved_runs: resolved_runs
                .iter()
                .map(|run| run.descriptor.clone())
                .collect(),
            summary: summarize_records(&records),
            records,
        };
        write_batch_cache_dir(&cache_dir, pack, &report, true)?;
        report
    };

    let requested_compare =
        resolve_compare_provenance(baseline_dir, compare_ref, missing_compare, &workspace)?;
    let (compare_provenance, baseline) =
        load_requested_baseline(pack, &identity, requested_compare, missing_compare)?;

    let mut output_report = cache_report.clone();
    output_report.provenance.compare = compare_provenance;
    if let Some(cache) = output_report.provenance.cache.as_mut() {
        cache.status = match cache.status {
            BatchCacheStatus::Promoted => BatchCacheStatus::Reused,
            other => other,
        };
    }
    let final_report = if let Some(output_dir) = output_dir {
        let localized_report = localize_report_bundle_dirs(&output_report, output_dir);
        write_batch_output_dir(
            output_dir,
            pack,
            &output_report,
            baseline
                .as_ref()
                .map(|baseline| (baseline.dir.as_path(), &baseline.report)),
        )?;
        localized_report
    } else {
        output_report
    };

    Ok(CachedBatchRunOutcome {
        report: final_report,
        baseline,
        cache_dir,
    })
}

pub fn run_pack_file_with_workers(
    path: &Path,
    output_dir: Option<&Path>,
    workers: usize,
) -> Result<BatchReport> {
    let pack = load_pack(path)?;
    let base_dir = path
        .parent()
        .ok_or_else(|| anyhow!("pack path has no parent directory"))?;
    run_pack_with_workers(&pack, base_dir, output_dir, workers)
}

pub fn run_pack(
    pack: &ScenarioPackSpec,
    base_dir: &Path,
    output_dir: Option<&Path>,
) -> Result<BatchReport> {
    run_pack_with_workers(pack, base_dir, output_dir, 1)
}

pub fn run_pack_with_workers(
    pack: &ScenarioPackSpec,
    base_dir: &Path,
    output_dir: Option<&Path>,
    workers: usize,
) -> Result<BatchReport> {
    validate_pack(pack)?;

    let resolved_runs = resolve_pack_runs(pack, base_dir)?;
    let requested_workers = workers.max(1);
    let workers_used = effective_worker_count(requested_workers, resolved_runs.len());
    let identity = BatchIdentity {
        schema_version: BATCH_REPORT_SCHEMA_VERSION,
        pack_spec_digest: stable_digest(pack)?,
        resolved_run_digest: stable_digest(
            &resolved_runs
                .iter()
                .map(|run| &run.descriptor)
                .collect::<Vec<_>>(),
        )?,
    };

    let started = Instant::now();
    let records = execute_resolved_runs(&resolved_runs, output_dir, workers_used)?;
    let report = BatchReport {
        schema_version: BATCH_REPORT_SCHEMA_VERSION,
        pack_id: pack.id.clone(),
        pack_name: pack.name.clone(),
        total_runs: records.len(),
        wall_clock_s: started.elapsed().as_secs_f64(),
        workers_requested: requested_workers,
        workers_used,
        identity,
        provenance: BatchProvenance::default(),
        resolved_runs: resolved_runs
            .iter()
            .map(|run| run.descriptor.clone())
            .collect(),
        summary: summarize_records(&records),
        records,
    };

    if let Some(output_dir) = output_dir {
        fs::create_dir_all(output_dir).with_context(|| {
            format!(
                "failed to create batch eval output directory {}",
                output_dir.display()
            )
        })?;
        write_json(&output_dir.join("pack.json"), pack)?;
        write_json(
            &output_dir.join("resolved_runs.json"),
            &report.resolved_runs,
        )?;
        write_json(&output_dir.join("summary.json"), &report)?;
        maybe_update_latest_link(output_dir)?;
        if let Some(last_record) = report.records.last()
            && let Some(bundle_dir) = last_record.bundle_dir.as_deref()
        {
            maybe_update_latest_link(Path::new(bundle_dir))?;
        }
    }

    Ok(report)
}

fn validate_pack(pack: &ScenarioPackSpec) -> Result<()> {
    if pack.id.trim().is_empty() {
        bail!("pack id must not be empty");
    }
    if pack.name.trim().is_empty() {
        bail!("pack name must not be empty");
    }
    if pack.entries.is_empty() {
        bail!("pack must contain at least one entry");
    }
    if let Some(max_time_s) = pack.terminal_matrix_max_time_s
        && (!max_time_s.is_finite() || max_time_s <= 0.0)
    {
        bail!("terminal_matrix_max_time_s must be finite and > 0");
    }

    let mut seen_ids = BTreeSet::new();
    for entry in &pack.entries {
        if entry.id().trim().is_empty() {
            bail!("pack entry id must not be empty");
        }
        if !seen_ids.insert(entry.id().to_owned()) {
            bail!("duplicate pack entry id '{}'", entry.id());
        }

        match entry {
            ScenarioPackEntry::Scenario(entry) => {
                if entry.controller.trim().is_empty() {
                    bail!("pack entry controller must not be empty");
                }
                if entry.scenario.trim().is_empty() {
                    bail!("pack entry scenario path must not be empty");
                }
            }
            ScenarioPackEntry::Family(entry) => validate_family_entry(entry)?,
            ScenarioPackEntry::TerminalMatrix(entry) => validate_terminal_matrix_entry(entry)?,
            ScenarioPackEntry::TransferMatrix(entry) => validate_transfer_matrix_entry(entry)?,
        }
    }

    Ok(())
}

fn validate_family_entry(entry: &ScenarioFamilyEntry) -> Result<()> {
    if entry.controller.trim().is_empty() {
        bail!(
            "family entry '{}' must define a non-empty controller",
            entry.id
        );
    }
    if entry.family.trim().is_empty() {
        bail!(
            "family entry '{}' must define a non-empty family id",
            entry.id
        );
    }
    if entry.base_scenario.trim().is_empty() {
        bail!(
            "family entry '{}' must define a non-empty base_scenario path",
            entry.id
        );
    }
    if entry.tags.iter().any(|tag| tag.trim().is_empty()) {
        bail!(
            "family entry '{}' tags must not contain empty values",
            entry.id
        );
    }
    for (key, value) in &entry.metadata {
        if key.trim().is_empty() || value.trim().is_empty() {
            bail!(
                "family entry '{}' metadata keys and values must not be empty",
                entry.id
            );
        }
    }

    let explicit_seed_count = usize::from(!entry.seeds.is_empty());
    let range_seed_count = usize::from(entry.seed_range.is_some());
    if explicit_seed_count + range_seed_count != 1 {
        bail!(
            "family entry '{}' must define exactly one of 'seeds' or 'seed_range'",
            entry.id
        );
    }
    if let Some(seed_range) = &entry.seed_range
        && seed_range.count == 0
    {
        bail!("family entry '{}' seed_range.count must be > 0", entry.id);
    }
    if !entry.seeds.is_empty() {
        let mut seen = BTreeSet::new();
        for seed in &entry.seeds {
            if !seen.insert(*seed) {
                bail!(
                    "family entry '{}' seeds must not contain duplicates (duplicate seed {})",
                    entry.id,
                    seed
                );
            }
        }
    }

    let mut seen_ids = BTreeSet::new();
    for perturbation in &entry.perturbations {
        validate_numeric_perturbation(entry, perturbation, &mut seen_ids)?;
    }

    Ok(())
}

fn validate_terminal_matrix_entry(entry: &TerminalMatrixEntry) -> Result<()> {
    if entry.terminal_matrix.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty terminal_matrix",
            entry.id
        );
    }
    let family_spec = terminal_arrival_family_spec(&entry.terminal_matrix)?;
    if entry.base_scenario.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty base_scenario",
            entry.id
        );
    }
    if entry.condition_set.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty condition_set",
            entry.id
        );
    }
    terminal_condition_spec(&entry.condition_set)?;
    if entry.vehicle_variant.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty vehicle_variant",
            entry.id
        );
    }
    if entry.expectation_tier.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty expectation_tier",
            entry.id
        );
    }
    if entry.lanes.is_empty() {
        bail!(
            "terminal matrix entry '{}' must define at least one lane",
            entry.id
        );
    }
    let mut seen_lane_ids = BTreeSet::new();
    for lane in &entry.lanes {
        if lane.id.trim().is_empty() {
            bail!(
                "terminal matrix entry '{}' has a lane with an empty id",
                entry.id
            );
        }
        if lane.controller.trim().is_empty() {
            bail!(
                "terminal matrix entry '{}' lane '{}' must define a controller",
                entry.id,
                lane.id
            );
        }
        if !seen_lane_ids.insert(lane.id.clone()) {
            bail!(
                "terminal matrix entry '{}' has duplicate lane id '{}'",
                entry.id,
                lane.id
            );
        }
    }
    for (key, value) in &entry.metadata {
        if key.trim().is_empty() || value.trim().is_empty() {
            bail!(
                "terminal matrix entry '{}' metadata keys and values must not be empty",
                entry.id
            );
        }
    }
    let mut seen_arc_points = BTreeSet::new();
    for arc_point in &entry.arc_points {
        if arc_point.trim().is_empty() {
            bail!(
                "terminal matrix entry '{}' has an empty arc_point selector",
                entry.id
            );
        }
        if !seen_arc_points.insert(arc_point.clone()) {
            bail!(
                "terminal matrix entry '{}' has duplicate arc_point selector '{}'",
                entry.id,
                arc_point
            );
        }
        if !family_spec
            .arc_points
            .iter()
            .any(|candidate| candidate.id == arc_point)
        {
            bail!(
                "terminal matrix entry '{}' arc_point selector '{}' is not supported by matrix '{}'",
                entry.id,
                arc_point,
                entry.terminal_matrix
            );
        }
    }
    let mut seen_adjustment_ids = BTreeSet::new();
    for adjustment in &entry.adjustments {
        validate_numeric_adjustment(
            &entry.id,
            "terminal matrix",
            adjustment,
            &mut seen_adjustment_ids,
        )?;
    }
    Ok(())
}

fn validate_numeric_adjustment(
    entry_id: &str,
    entry_kind: &str,
    adjustment: &NumericAdjustmentSpec,
    seen_ids: &mut BTreeSet<String>,
) -> Result<()> {
    if adjustment.id.trim().is_empty() {
        bail!(
            "{entry_kind} entry '{}' has an adjustment with an empty id",
            entry_id
        );
    }
    if !seen_ids.insert(adjustment.id.clone()) {
        bail!(
            "{entry_kind} entry '{}' has duplicate adjustment id '{}'",
            entry_id,
            adjustment.id
        );
    }
    if !is_supported_terminal_adjustment_path(&adjustment.path) {
        bail!(
            "{entry_kind} entry '{}' adjustment '{}' uses unsupported path '{}'",
            entry_id,
            adjustment.id,
            adjustment.path
        );
    }
    if !adjustment.value.is_finite() {
        bail!(
            "{entry_kind} entry '{}' adjustment '{}' must be finite",
            entry_id,
            adjustment.id
        );
    }
    Ok(())
}

fn validate_transfer_matrix_entry(entry: &TransferMatrixEntry) -> Result<()> {
    if entry.transfer_matrix.trim().is_empty() {
        bail!(
            "transfer matrix entry '{}' must define a non-empty transfer_matrix",
            entry.id
        );
    }
    let route_spec = transfer_route_family_spec(&entry.transfer_matrix)?;
    if entry.base_scenario.trim().is_empty() {
        bail!(
            "transfer matrix entry '{}' must define a non-empty base_scenario",
            entry.id
        );
    }
    if entry.vehicle_variant.trim().is_empty() {
        bail!(
            "transfer matrix entry '{}' must define a non-empty vehicle_variant",
            entry.id
        );
    }
    if entry.expectation_tier.trim().is_empty() {
        bail!(
            "transfer matrix entry '{}' must define a non-empty expectation_tier",
            entry.id
        );
    }
    if let Some(profile) = &entry.waypoint_profile {
        validate_transfer_waypoint_profile(&entry.id, profile)?;
        if profile == TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1
            && entry.expectation_tier != TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC
        {
            bail!(
                "transfer matrix entry '{}' waypoint_profile '{}' requires expectation_tier '{}'",
                entry.id,
                profile,
                TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC
            );
        }
        if profile == TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1
            && entry.expectation_tier != TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC
        {
            bail!(
                "transfer matrix entry '{}' waypoint_profile '{}' requires expectation_tier '{}'",
                entry.id,
                profile,
                TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC
            );
        }
    }
    if let Some(envelope) = &entry.waypoint_handoff_envelope {
        validate_transfer_waypoint_envelope(&entry.id, envelope)?;
        if entry.waypoint_profile.is_none() {
            bail!(
                "transfer matrix entry '{}' waypoint_handoff_envelope requires waypoint_profile",
                entry.id
            );
        }
    }
    if matches!(
        entry.evaluation_goal,
        TransferMatrixEvaluationGoal::WaypointHandoff
            | TransferMatrixEvaluationGoal::WaypointSequence
    ) && entry.waypoint_profile.is_none()
    {
        bail!(
            "transfer matrix entry '{}' evaluation_goal '{}' requires waypoint_profile",
            entry.id,
            entry.evaluation_goal.as_str()
        );
    }
    if entry.lanes.is_empty() {
        bail!(
            "transfer matrix entry '{}' must define at least one lane",
            entry.id
        );
    }
    let mut seen_lane_ids = BTreeSet::new();
    for lane in &entry.lanes {
        if lane.id.trim().is_empty() {
            bail!(
                "transfer matrix entry '{}' has a lane with an empty id",
                entry.id
            );
        }
        if lane.controller.trim().is_empty() {
            bail!(
                "transfer matrix entry '{}' lane '{}' must define a controller",
                entry.id,
                lane.id
            );
        }
        if !seen_lane_ids.insert(lane.id.clone()) {
            bail!(
                "transfer matrix entry '{}' has duplicate lane id '{}'",
                entry.id,
                lane.id
            );
        }
    }
    for (key, value) in &entry.metadata {
        if key.trim().is_empty() || value.trim().is_empty() {
            bail!(
                "transfer matrix entry '{}' metadata keys and values must not be empty",
                entry.id
            );
        }
    }
    let mut seen_route_angles = BTreeSet::new();
    for route_angle in &entry.route_angles {
        if route_angle.trim().is_empty() {
            bail!(
                "transfer matrix entry '{}' has an empty route_angle selector",
                entry.id
            );
        }
        if !seen_route_angles.insert(route_angle.clone()) {
            bail!(
                "transfer matrix entry '{}' has duplicate route_angle selector '{}'",
                entry.id,
                route_angle
            );
        }
        if !route_spec
            .route_angles
            .iter()
            .any(|candidate| candidate.id == route_angle)
        {
            bail!(
                "transfer matrix entry '{}' route_angle selector '{}' is not supported by matrix '{}'",
                entry.id,
                route_angle,
                entry.transfer_matrix
            );
        }
    }
    let mut seen_radius_tiers = BTreeSet::new();
    for radius_tier in &entry.radius_tiers {
        if radius_tier.trim().is_empty() {
            bail!(
                "transfer matrix entry '{}' has an empty radius_tier selector",
                entry.id
            );
        }
        if !seen_radius_tiers.insert(radius_tier.clone()) {
            bail!(
                "transfer matrix entry '{}' has duplicate radius_tier selector '{}'",
                entry.id,
                radius_tier
            );
        }
        if !route_spec
            .radius_tiers
            .iter()
            .any(|candidate| candidate.id == radius_tier)
        {
            bail!(
                "transfer matrix entry '{}' radius_tier selector '{}' is not supported by matrix '{}'",
                entry.id,
                radius_tier,
                entry.transfer_matrix
            );
        }
    }
    let mut seen_adjustment_ids = BTreeSet::new();
    for adjustment in &entry.adjustments {
        validate_numeric_adjustment(
            &entry.id,
            "transfer matrix",
            adjustment,
            &mut seen_adjustment_ids,
        )?;
    }
    Ok(())
}

fn validate_transfer_waypoint_profile(entry_id: &str, profile: &str) -> Result<()> {
    if profile.trim().is_empty() {
        bail!("transfer matrix entry '{entry_id}' waypoint_profile must not be empty");
    }
    if !matches!(
        profile,
        TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1
            | TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1
            | TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1
            | TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1
            | TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1
            | TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1
            | TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1
    ) {
        bail!(
            "transfer matrix entry '{}' waypoint_profile '{}' is not supported",
            entry_id,
            profile
        );
    }
    Ok(())
}

fn validate_transfer_waypoint_envelope(entry_id: &str, envelope: &str) -> Result<()> {
    if envelope.trim().is_empty() {
        bail!("transfer matrix entry '{entry_id}' waypoint_handoff_envelope must not be empty");
    }
    if !matches!(
        envelope,
        TRANSFER_WAYPOINT_ENVELOPE_LEGACY_V1
            | TRANSFER_WAYPOINT_ENVELOPE_PASS_THROUGH_V1
            | TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1
            | TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1
    ) {
        bail!(
            "transfer matrix entry '{}' waypoint_handoff_envelope '{}' is not supported",
            entry_id,
            envelope
        );
    }
    Ok(())
}

fn validate_numeric_perturbation(
    entry: &ScenarioFamilyEntry,
    perturbation: &NumericPerturbationSpec,
    seen_ids: &mut BTreeSet<String>,
) -> Result<()> {
    if perturbation.id.trim().is_empty() {
        bail!(
            "family entry '{}' has a perturbation with an empty id",
            entry.id
        );
    }
    if !seen_ids.insert(perturbation.id.clone()) {
        bail!(
            "family entry '{}' has duplicate perturbation id '{}'",
            entry.id,
            perturbation.id
        );
    }
    if !is_supported_numeric_path(&perturbation.path) {
        bail!(
            "family entry '{}' perturbation '{}' uses unsupported path '{}'",
            entry.id,
            perturbation.id,
            perturbation.path
        );
    }
    if !perturbation.min.is_finite() || !perturbation.max.is_finite() {
        bail!(
            "family entry '{}' perturbation '{}' bounds must be finite",
            entry.id,
            perturbation.id
        );
    }
    if perturbation.max < perturbation.min {
        bail!(
            "family entry '{}' perturbation '{}' max must be >= min",
            entry.id,
            perturbation.id
        );
    }
    if let Some(step) = perturbation.quantize
        && (!step.is_finite() || step <= 0.0)
    {
        bail!(
            "family entry '{}' perturbation '{}' quantize must be > 0",
            entry.id,
            perturbation.id
        );
    }
    Ok(())
}

fn resolve_pack_runs(pack: &ScenarioPackSpec, base_dir: &Path) -> Result<Vec<ResolvedBatchRun>> {
    let mut resolved = Vec::new();
    for entry in &pack.entries {
        match entry {
            ScenarioPackEntry::Scenario(entry) => {
                let controller_spec = load_controller_spec(
                    base_dir,
                    entry.controller.as_str(),
                    entry.controller_config.as_deref(),
                )?;
                resolved.push(resolve_concrete_run(entry, base_dir, &controller_spec)?)
            }
            ScenarioPackEntry::Family(entry) => {
                let controller_spec = load_controller_spec(
                    base_dir,
                    entry.controller.as_str(),
                    entry.controller_config.as_deref(),
                )?;
                resolved.extend(resolve_family_runs(entry, base_dir, &controller_spec)?)
            }
            ScenarioPackEntry::TerminalMatrix(entry) => resolved.extend(
                resolve_terminal_matrix_runs(entry, base_dir, pack.terminal_matrix_max_time_s)?,
            ),
            ScenarioPackEntry::TransferMatrix(entry) => {
                resolved.extend(resolve_transfer_matrix_runs(entry, base_dir)?)
            }
        }
    }

    let mut seen_run_ids = BTreeSet::new();
    for run in &resolved {
        if !seen_run_ids.insert(run.descriptor.run_id.clone()) {
            bail!(
                "resolved pack contains duplicate run id '{}'",
                run.descriptor.run_id
            );
        }
    }
    Ok(resolved)
}

fn resolve_concrete_run(
    entry: &ConcreteScenarioPackEntry,
    base_dir: &Path,
    controller_spec: &ControllerSpec,
) -> Result<ResolvedBatchRun> {
    let scenario_path = base_dir.join(&entry.scenario);
    let mut scenario = load_scenario(&scenario_path)?;
    scenario.metadata.extend(entry.metadata.clone());
    let family_id = scenario.metadata.get("family").cloned();
    let selector = selector_axes_from_metadata(&scenario.metadata);
    let descriptor = ResolvedRunDescriptor {
        run_id: sanitize_token(&entry.id),
        entry_id: entry.id.clone(),
        source_kind: ResolvedRunSourceKind::ConcreteScenario,
        scenario_source: entry.scenario.clone(),
        resolved_scenario_id: scenario.id.clone(),
        resolved_scenario_name: scenario.name.clone(),
        family_id,
        selector,
        lane_id: entry.controller.clone(),
        resolved_seed: scenario.seed,
        resolved_parameters: BTreeMap::new(),
        controller_id: controller_spec.id().to_owned(),
        controller_spec: controller_spec.clone(),
    };

    Ok(ResolvedBatchRun {
        descriptor,
        scenario,
    })
}

fn resolve_family_runs(
    entry: &ScenarioFamilyEntry,
    base_dir: &Path,
    controller_spec: &ControllerSpec,
) -> Result<Vec<ResolvedBatchRun>> {
    let base_path = base_dir.join(&entry.base_scenario);
    let base_scenario = load_scenario(&base_path)?;
    let mut runs = Vec::new();

    for seed in family_entry_seeds(entry) {
        let (scenario, resolved_parameters) = resolve_family_scenario(entry, &base_scenario, seed)?;
        let selector = selector_axes_from_metadata(&scenario.metadata);
        let descriptor = ResolvedRunDescriptor {
            run_id: resolved_family_run_id(&entry.id, seed),
            entry_id: entry.id.clone(),
            source_kind: ResolvedRunSourceKind::FamilySweep,
            scenario_source: entry.base_scenario.clone(),
            resolved_scenario_id: scenario.id.clone(),
            resolved_scenario_name: scenario.name.clone(),
            family_id: Some(entry.family.clone()),
            selector,
            lane_id: entry.controller.clone(),
            resolved_seed: seed,
            resolved_parameters,
            controller_id: controller_spec.id().to_owned(),
            controller_spec: controller_spec.clone(),
        };
        runs.push(ResolvedBatchRun {
            descriptor,
            scenario,
        });
    }

    Ok(runs)
}

#[derive(Clone, Copy, Debug)]
struct TerminalArcPointSpec {
    id: &'static str,
    angle_deg: f64,
    nominal_ttg_s: f64,
}

#[derive(Clone, Copy, Debug)]
struct TerminalArrivalFamilySpec {
    arrival_family: &'static str,
    gravity_mps2: f64,
    radius_nominal_m: f64,
    low_multiplier: f64,
    high_multiplier: f64,
    clamp_low_to_descending: bool,
    arc_points: &'static [TerminalArcPointSpec],
}

#[derive(Clone, Copy, Debug)]
struct TerminalBandSpec {
    id: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct TerminalSeedSpec {
    index: u64,
    radial_pct: Option<f64>,
    speed_pct: Option<f64>,
    error_level_index: usize,
}

#[derive(Clone, Copy, Debug)]
struct TransferRouteAngleSpec {
    id: &'static str,
    angle_deg: f64,
}

#[derive(Clone, Copy, Debug)]
struct TransferRouteFamilySpec {
    route_family: &'static str,
    gravity_mps2: f64,
    default_radius_tier: &'static str,
    radius_tiers: &'static [TransferRadiusTierSpec],
    route_angles: &'static [TransferRouteAngleSpec],
    smoke_route_angles: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
struct TransferRadiusTierSpec {
    id: &'static str,
    radius_m: f64,
}

#[derive(Clone, Copy, Debug)]
struct TransferSeedSpec {
    index: u64,
    radius_pct: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalProjectedErrorKind {
    Undershoot,
    Overshoot,
}

impl TerminalProjectedErrorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Undershoot => "undershoot",
            Self::Overshoot => "overshoot",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TerminalProjectedErrorSpec {
    kind: TerminalProjectedErrorKind,
    severity: &'static str,
    magnitudes_m: [f64; 3],
}

#[derive(Clone, Copy, Debug)]
enum TerminalReactiveTerrainHazard {
    ContainmentBackstop,
    DescentClip,
}

impl TerminalReactiveTerrainHazard {
    fn as_str(self) -> &'static str {
        match self {
            Self::ContainmentBackstop => "containment_backstop",
            Self::DescentClip => "descent_clip",
        }
    }

    fn obstacle_kind(self) -> &'static str {
        match self {
            Self::ContainmentBackstop => "backstop",
            Self::DescentClip => "shoulder",
        }
    }

    fn obstacle_placement(self) -> &'static str {
        match self {
            Self::ContainmentBackstop => "target_side",
            Self::DescentClip => "terminal_approach",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TerminalReactiveTerrainSpec {
    hazard: TerminalReactiveTerrainHazard,
    variant: &'static str,
    height_offset_m: f64,
    pad_clearance_gap_m: f64,
    shoulder_width_m: f64,
    top_width_m: f64,
}

#[derive(Clone, Copy, Debug)]
enum TerminalConditionSpec {
    Clean,
    ProjectedError(TerminalProjectedErrorSpec),
    ReactiveTerrain(TerminalReactiveTerrainSpec),
}

impl TerminalConditionSpec {
    fn kind_label(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::ProjectedError(_) => "projected_error",
            Self::ReactiveTerrain(_) => "reactive_terrain",
        }
    }
}

const HALF_ARC_TERMINAL_V1_ARC_POINTS: [TerminalArcPointSpec; 7] = [
    TerminalArcPointSpec {
        id: "a00",
        angle_deg: 0.0,
        nominal_ttg_s: 9.00,
    },
    TerminalArcPointSpec {
        id: "a15",
        angle_deg: 15.0,
        nominal_ttg_s: 9.00,
    },
    TerminalArcPointSpec {
        id: "a30",
        angle_deg: 30.0,
        nominal_ttg_s: 8.75,
    },
    TerminalArcPointSpec {
        id: "a45",
        angle_deg: 45.0,
        nominal_ttg_s: 8.50,
    },
    TerminalArcPointSpec {
        id: "a60",
        angle_deg: 60.0,
        nominal_ttg_s: 8.25,
    },
    TerminalArcPointSpec {
        id: "a70",
        angle_deg: 70.0,
        nominal_ttg_s: 8.00,
    },
    TerminalArcPointSpec {
        id: "a80",
        angle_deg: 80.0,
        nominal_ttg_s: 8.00,
    },
];

const HALF_ARC_TERMINAL_V1_SPEC: TerminalArrivalFamilySpec = TerminalArrivalFamilySpec {
    arrival_family: "half_arc_terminal_v1",
    gravity_mps2: 9.81,
    radius_nominal_m: 800.0,
    low_multiplier: 1.25,
    high_multiplier: 0.75,
    clamp_low_to_descending: false,
    arc_points: &HALF_ARC_TERMINAL_V1_ARC_POINTS,
};

const TERMINAL_BANDS: [TerminalBandSpec; 3] = [
    TerminalBandSpec { id: "low" },
    TerminalBandSpec { id: "mid" },
    TerminalBandSpec { id: "high" },
];

const TERMINAL_SMOKE_SEEDS: [TerminalSeedSpec; 3] = [
    TerminalSeedSpec {
        index: 0,
        radial_pct: Some(0.015),
        speed_pct: None,
        error_level_index: 0,
    },
    TerminalSeedSpec {
        index: 1,
        radial_pct: Some(-0.015),
        speed_pct: None,
        error_level_index: 1,
    },
    TerminalSeedSpec {
        index: 6,
        radial_pct: None,
        speed_pct: Some(0.010),
        error_level_index: 2,
    },
];

const TERMINAL_FULL_SEEDS: [TerminalSeedSpec; 12] = [
    TerminalSeedSpec {
        index: 0,
        radial_pct: Some(0.015),
        speed_pct: None,
        error_level_index: 0,
    },
    TerminalSeedSpec {
        index: 1,
        radial_pct: Some(-0.015),
        speed_pct: None,
        error_level_index: 1,
    },
    TerminalSeedSpec {
        index: 2,
        radial_pct: Some(0.030),
        speed_pct: None,
        error_level_index: 2,
    },
    TerminalSeedSpec {
        index: 3,
        radial_pct: Some(-0.030),
        speed_pct: None,
        error_level_index: 0,
    },
    TerminalSeedSpec {
        index: 4,
        radial_pct: Some(0.045),
        speed_pct: None,
        error_level_index: 1,
    },
    TerminalSeedSpec {
        index: 5,
        radial_pct: Some(-0.045),
        speed_pct: None,
        error_level_index: 2,
    },
    TerminalSeedSpec {
        index: 6,
        radial_pct: None,
        speed_pct: Some(0.010),
        error_level_index: 2,
    },
    TerminalSeedSpec {
        index: 7,
        radial_pct: None,
        speed_pct: Some(-0.010),
        error_level_index: 0,
    },
    TerminalSeedSpec {
        index: 8,
        radial_pct: None,
        speed_pct: Some(0.020),
        error_level_index: 1,
    },
    TerminalSeedSpec {
        index: 9,
        radial_pct: None,
        speed_pct: Some(-0.020),
        error_level_index: 2,
    },
    TerminalSeedSpec {
        index: 10,
        radial_pct: None,
        speed_pct: Some(0.030),
        error_level_index: 0,
    },
    TerminalSeedSpec {
        index: 11,
        radial_pct: None,
        speed_pct: Some(-0.030),
        error_level_index: 1,
    },
];

const SIGNED_ROUTE_ARC_TRANSFER_V1_ROUTE_ANGLES: [TransferRouteAngleSpec; 11] = [
    TransferRouteAngleSpec {
        id: "r-80",
        angle_deg: -80.0,
    },
    TransferRouteAngleSpec {
        id: "r-60",
        angle_deg: -60.0,
    },
    TransferRouteAngleSpec {
        id: "r-45",
        angle_deg: -45.0,
    },
    TransferRouteAngleSpec {
        id: "r-30",
        angle_deg: -30.0,
    },
    TransferRouteAngleSpec {
        id: "r-15",
        angle_deg: -15.0,
    },
    TransferRouteAngleSpec {
        id: "r00",
        angle_deg: 0.0,
    },
    TransferRouteAngleSpec {
        id: "r+15",
        angle_deg: 15.0,
    },
    TransferRouteAngleSpec {
        id: "r+30",
        angle_deg: 30.0,
    },
    TransferRouteAngleSpec {
        id: "r+45",
        angle_deg: 45.0,
    },
    TransferRouteAngleSpec {
        id: "r+60",
        angle_deg: 60.0,
    },
    TransferRouteAngleSpec {
        id: "r+80",
        angle_deg: 80.0,
    },
];

const SIGNED_ROUTE_ARC_TRANSFER_V1_SMOKE_ROUTE_ANGLES: [&str; 5] =
    ["r-60", "r-30", "r00", "r+30", "r+60"];

const SIGNED_ROUTE_ARC_TRANSFER_V1_RADIUS_TIERS: [TransferRadiusTierSpec; 3] = [
    TransferRadiusTierSpec {
        id: "short",
        radius_m: 400.0,
    },
    TransferRadiusTierSpec {
        id: "nominal",
        radius_m: 800.0,
    },
    TransferRadiusTierSpec {
        id: "long",
        radius_m: 1200.0,
    },
];

const SIGNED_ROUTE_ARC_TRANSFER_V1_SPEC: TransferRouteFamilySpec = TransferRouteFamilySpec {
    route_family: "signed_route_arc_transfer_v1",
    gravity_mps2: 9.81,
    default_radius_tier: "nominal",
    radius_tiers: &SIGNED_ROUTE_ARC_TRANSFER_V1_RADIUS_TIERS,
    route_angles: &SIGNED_ROUTE_ARC_TRANSFER_V1_ROUTE_ANGLES,
    smoke_route_angles: &SIGNED_ROUTE_ARC_TRANSFER_V1_SMOKE_ROUTE_ANGLES,
};

const TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1: &str = "single_dogleg_v1";
const TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1: &str = "single_bend_v1";
const TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1: &str = "single_gentle_bend_v1";
const TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1: &str = "single_medium_bend_v1";
const TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1: &str = "single_sharp_bend_v1";
const TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1: &str = "double_bend_v1";
const TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1: &str = "late_bend_v1";
const TRANSFER_WAYPOINT_ENVELOPE_LEGACY_V1: &str = "legacy_v1";
const TRANSFER_WAYPOINT_ENVELOPE_PASS_THROUGH_V1: &str = "pass_through_v1";
const TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1: &str =
    "continuation_pass_through_v1";
const TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1: &str = "sequence_pass_through_v1";
const TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC: &str = "diagnostic";
const TRANSFER_WAYPOINT_SINGLE_BEND_PROGRESS_FRAC: f64 = 0.55;
const TRANSFER_WAYPOINT_SINGLE_BEND_LATERAL_OFFSET_RATIO: f64 = 0.20;
const TRANSFER_WAYPOINT_GEOMETRY_TOLERANCE: f64 = 1.0e-6;
const TRANSFER_WAYPOINT_TURN_TOLERANCE_DEG: f64 = 1.0e-3;
const TRANSFER_WAYPOINT_CONTINUATION_MAX_STOP_RATIO: f64 = 0.75;

#[derive(Clone, Copy)]
struct TransferWaypointBendProfileSpec {
    waypoint_id: &'static str,
    progress_frac: f64,
    lateral_offset_ratio: f64,
    capture_radius_ratio: f64,
    min_capture_radius_m: f64,
    max_capture_radius_m: f64,
    max_cross_track_factor: f64,
    min_route_angle_deg: Option<f64>,
}

#[derive(Clone, Copy)]
struct TransferWaypointGeometryExpectation {
    progress_frac: f64,
    lateral_offset_ratio: f64,
    signed_turn_deg: f64,
}

const SINGLE_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 1] =
    [TransferWaypointGeometryExpectation {
        progress_frac: 0.55,
        lateral_offset_ratio: 0.20,
        signed_turn_deg: -43.9456,
    }];
const SINGLE_GENTLE_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 1] =
    [TransferWaypointGeometryExpectation {
        progress_frac: 0.55,
        lateral_offset_ratio: 0.12,
        signed_turn_deg: -27.2394,
    }];
const SINGLE_MEDIUM_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 1] =
    [TransferWaypointGeometryExpectation {
        progress_frac: 0.55,
        lateral_offset_ratio: 0.20,
        signed_turn_deg: -43.9456,
    }];
const SINGLE_SHARP_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 1] =
    [TransferWaypointGeometryExpectation {
        progress_frac: 0.55,
        lateral_offset_ratio: 0.30,
        signed_turn_deg: -62.3005,
    }];
const DOUBLE_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 2] = [
    TransferWaypointGeometryExpectation {
        progress_frac: 0.33,
        lateral_offset_ratio: 0.20,
        signed_turn_deg: -31.2184,
    },
    TransferWaypointGeometryExpectation {
        progress_frac: 0.67,
        lateral_offset_ratio: 0.20,
        signed_turn_deg: -31.2184,
    },
];
const LATE_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 2] = [
    TransferWaypointGeometryExpectation {
        progress_frac: 0.33,
        lateral_offset_ratio: 0.13,
        signed_turn_deg: -0.5769,
    },
    TransferWaypointGeometryExpectation {
        progress_frac: 0.67,
        lateral_offset_ratio: 0.26,
        signed_turn_deg: -59.1583,
    },
];

const TRANSFER_SMOKE_SEEDS: [TransferSeedSpec; 3] = [
    TransferSeedSpec {
        index: 0,
        radius_pct: 0.0,
    },
    TransferSeedSpec {
        index: 1,
        radius_pct: -0.03,
    },
    TransferSeedSpec {
        index: 2,
        radius_pct: 0.03,
    },
];

const TRANSFER_FULL_SEEDS: [TransferSeedSpec; 12] = [
    TransferSeedSpec {
        index: 0,
        radius_pct: 0.0,
    },
    TransferSeedSpec {
        index: 1,
        radius_pct: -0.015,
    },
    TransferSeedSpec {
        index: 2,
        radius_pct: 0.015,
    },
    TransferSeedSpec {
        index: 3,
        radius_pct: -0.03,
    },
    TransferSeedSpec {
        index: 4,
        radius_pct: 0.03,
    },
    TransferSeedSpec {
        index: 5,
        radius_pct: -0.045,
    },
    TransferSeedSpec {
        index: 6,
        radius_pct: 0.045,
    },
    TransferSeedSpec {
        index: 7,
        radius_pct: -0.06,
    },
    TransferSeedSpec {
        index: 8,
        radius_pct: 0.06,
    },
    TransferSeedSpec {
        index: 9,
        radius_pct: -0.075,
    },
    TransferSeedSpec {
        index: 10,
        radius_pct: 0.075,
    },
    TransferSeedSpec {
        index: 11,
        radius_pct: -0.09,
    },
];

fn resolve_transfer_matrix_runs(
    entry: &TransferMatrixEntry,
    base_dir: &Path,
) -> Result<Vec<ResolvedBatchRun>> {
    let base_path = base_dir.join(&entry.base_scenario);
    let base_scenario = load_scenario(&base_path)?;
    let family_spec = transfer_route_family_spec(&entry.transfer_matrix)?;
    let route_angle_specs = selected_transfer_route_angle_specs(entry, family_spec)?;
    let radius_tier_specs = selected_transfer_radius_tier_specs(entry, family_spec)?;
    let seed_specs = transfer_seed_specs(entry.seed_tier);
    let mut runs = Vec::new();

    for lane in &entry.lanes {
        let controller_spec = load_controller_spec(
            base_dir,
            lane.controller.as_str(),
            lane.controller_config.as_deref(),
        )?;
        for route_angle in &route_angle_specs {
            for radius_tier in &radius_tier_specs {
                for seed_spec in seed_specs {
                    let run_id = resolved_transfer_matrix_run_id(
                        &entry.id,
                        route_angle.id,
                        radius_tier.id,
                        seed_spec.index,
                        &lane.id,
                    );
                    let (scenario, resolved_parameters, selector) =
                        resolve_transfer_matrix_scenario(
                            entry,
                            &base_scenario,
                            family_spec,
                            route_angle,
                            radius_tier,
                            seed_spec,
                            &lane.id,
                            &run_id,
                        )?;
                    let descriptor = ResolvedRunDescriptor {
                        run_id,
                        entry_id: entry.id.clone(),
                        source_kind: ResolvedRunSourceKind::TransferMatrix,
                        scenario_source: entry.base_scenario.clone(),
                        resolved_scenario_id: scenario.id.clone(),
                        resolved_scenario_name: scenario.name.clone(),
                        family_id: Some(entry.id.clone()),
                        selector,
                        lane_id: lane.id.clone(),
                        resolved_seed: seed_spec.index,
                        resolved_parameters,
                        controller_id: controller_spec.id().to_owned(),
                        controller_spec: controller_spec.clone(),
                    };
                    runs.push(ResolvedBatchRun {
                        descriptor,
                        scenario,
                    });
                }
            }
        }
    }

    Ok(runs)
}

fn selected_transfer_route_angle_specs<'a>(
    entry: &TransferMatrixEntry,
    family_spec: &'a TransferRouteFamilySpec,
) -> Result<Vec<&'a TransferRouteAngleSpec>> {
    if entry.route_angles.is_empty() {
        return Ok(match entry.seed_tier {
            TransferSeedTier::Smoke => family_spec
                .route_angles
                .iter()
                .filter(|candidate| family_spec.smoke_route_angles.contains(&candidate.id))
                .collect(),
            TransferSeedTier::Full => family_spec.route_angles.iter().collect(),
        });
    }

    entry
        .route_angles
        .iter()
        .map(|route_angle| {
            family_spec
                .route_angles
                .iter()
                .find(|candidate| candidate.id == route_angle)
                .with_context(|| {
                    format!(
                        "transfer matrix entry '{}' route_angle selector '{}' is not supported by matrix '{}'",
                        entry.id, route_angle, entry.transfer_matrix
                    )
                })
        })
        .collect()
}

fn selected_transfer_radius_tier_specs<'a>(
    entry: &TransferMatrixEntry,
    family_spec: &'a TransferRouteFamilySpec,
) -> Result<Vec<&'a TransferRadiusTierSpec>> {
    if entry.radius_tiers.is_empty() {
        return family_spec
            .radius_tiers
            .iter()
            .find(|candidate| candidate.id == family_spec.default_radius_tier)
            .map(|candidate| vec![candidate])
            .with_context(|| {
                format!(
                    "transfer matrix '{}' default radius_tier '{}' is not supported",
                    family_spec.route_family, family_spec.default_radius_tier
                )
            });
    }

    entry
        .radius_tiers
        .iter()
        .map(|radius_tier| {
            family_spec
                .radius_tiers
                .iter()
                .find(|candidate| candidate.id == radius_tier)
                .with_context(|| {
                    format!(
                        "transfer matrix entry '{}' radius_tier selector '{}' is not supported by matrix '{}'",
                        entry.id, radius_tier, entry.transfer_matrix
                    )
                })
        })
        .collect()
}

fn transfer_route_family_spec(name: &str) -> Result<&'static TransferRouteFamilySpec> {
    match name {
        "signed_route_arc_transfer_v1" => Ok(&SIGNED_ROUTE_ARC_TRANSFER_V1_SPEC),
        _ => bail!("unsupported transfer matrix '{}'", name),
    }
}

fn transfer_seed_specs(seed_tier: TransferSeedTier) -> &'static [TransferSeedSpec] {
    match seed_tier {
        TransferSeedTier::Smoke => &TRANSFER_SMOKE_SEEDS,
        TransferSeedTier::Full => &TRANSFER_FULL_SEEDS,
    }
}

fn resolved_transfer_matrix_run_id(
    entry_id: &str,
    route_angle: &str,
    radius_tier: &str,
    seed: u64,
    lane_id: &str,
) -> String {
    sanitize_token(&format!(
        "{entry_id}__{}__{radius_tier}__seed_{seed:02}__{lane_id}",
        signed_selector_token(route_angle)
    ))
}

fn signed_selector_token(value: &str) -> String {
    value.replace('+', "pos").replace('-', "neg")
}

fn resolve_transfer_matrix_scenario(
    entry: &TransferMatrixEntry,
    base_scenario: &ScenarioSpec,
    family_spec: &TransferRouteFamilySpec,
    route_angle: &TransferRouteAngleSpec,
    radius_tier: &TransferRadiusTierSpec,
    seed_spec: &TransferSeedSpec,
    lane_id: &str,
    run_id: &str,
) -> Result<(ScenarioSpec, BTreeMap<String, f64>, SelectorAxes)> {
    let mut scenario = base_scenario.clone();
    scenario.id = run_id.to_owned();
    scenario.name = format!(
        "{} [{} {} {} {} seed {} {}]",
        base_scenario.name,
        family_spec.route_family,
        entry.vehicle_variant,
        route_angle.id,
        radius_tier.id,
        seed_spec.index,
        lane_id
    );
    scenario.description = format!(
        "{} ({} {} {} {} {} seed {} lane {})",
        base_scenario.description,
        "transfer_matrix",
        family_spec.route_family,
        entry.vehicle_variant,
        route_angle.id,
        radius_tier.id,
        seed_spec.index,
        lane_id
    );
    scenario.seed = seed_spec.index;
    scenario.sim.max_time_s = scenario
        .sim
        .max_time_s
        .max(if entry.waypoint_profile.is_some() {
            130.0
        } else {
            90.0
        });
    scenario.tags = merge_unique_tags(&base_scenario.tags, &entry.tags);
    scenario.metadata.extend(entry.metadata.clone());
    scenario
        .metadata
        .insert("family".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("family_entry_id".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("resolved_seed".to_owned(), seed_spec.index.to_string());
    scenario
        .metadata
        .insert("mission".to_owned(), "transfer_guidance".to_owned());
    scenario.metadata.insert(
        "arrival_family".to_owned(),
        family_spec.route_family.to_owned(),
    );
    scenario.metadata.insert(
        "route_family".to_owned(),
        family_spec.route_family.to_owned(),
    );
    scenario
        .metadata
        .insert("condition_set".to_owned(), "clean".to_owned());
    scenario
        .metadata
        .insert("vehicle_variant".to_owned(), entry.vehicle_variant.clone());
    scenario.metadata.insert(
        "expectation_tier".to_owned(),
        entry.expectation_tier.clone(),
    );
    scenario
        .metadata
        .insert("arc_point".to_owned(), route_angle.id.to_owned());
    scenario
        .metadata
        .insert("velocity_band".to_owned(), radius_tier.id.to_owned());
    scenario
        .metadata
        .insert("route_angle".to_owned(), route_angle.id.to_owned());
    scenario
        .metadata
        .insert("radius_tier".to_owned(), radius_tier.id.to_owned());
    scenario
        .metadata
        .insert("lane_id".to_owned(), lane_id.to_owned());
    scenario.metadata.insert(
        "evaluation_goal".to_owned(),
        entry.evaluation_goal.as_str().to_owned(),
    );
    let resolved_waypoint_envelope = entry.waypoint_profile.as_ref().map(|_| {
        entry
            .waypoint_handoff_envelope
            .as_deref()
            .unwrap_or(TRANSFER_WAYPOINT_ENVELOPE_LEGACY_V1)
    });
    if let Some(envelope) = resolved_waypoint_envelope {
        scenario
            .metadata
            .insert("waypoint_handoff_envelope".to_owned(), envelope.to_owned());
    }

    scenario.world.gravity_mps2 = family_spec.gravity_mps2;
    let mut resolved_parameters = BTreeMap::new();
    resolved_parameters.insert("gravity_mps2".to_owned(), family_spec.gravity_mps2);
    resolved_parameters.insert("route_angle_deg".to_owned(), route_angle.angle_deg);
    let route_radius_jitter_m = radius_tier.radius_m * seed_spec.radius_pct;
    let route_radius_m = radius_tier.radius_m + route_radius_jitter_m;
    let resolved_radius_tier = TransferRadiusTierSpec {
        id: radius_tier.id,
        radius_m: route_radius_m,
    };
    resolved_parameters.insert("route_radius_nominal_m".to_owned(), radius_tier.radius_m);
    resolved_parameters.insert("route_radius_pct".to_owned(), seed_spec.radius_pct);
    resolved_parameters.insert("route_radius_jitter_m".to_owned(), route_radius_jitter_m);
    resolved_parameters.insert("route_radius_m".to_owned(), route_radius_m);
    scenario.metadata.insert(
        "resolved.route_radius_nominal_m".to_owned(),
        format!("{:.6}", radius_tier.radius_m),
    );
    scenario.metadata.insert(
        "resolved.route_radius_pct".to_owned(),
        format!("{:.6}", seed_spec.radius_pct),
    );
    scenario.metadata.insert(
        "resolved.route_radius_jitter_m".to_owned(),
        format!("{route_radius_jitter_m:.6}"),
    );
    scenario.metadata.insert(
        "resolved.seed_variation".to_owned(),
        if seed_spec.radius_pct.abs() > f64::EPSILON {
            "route_radius".to_owned()
        } else {
            "none".to_owned()
        },
    );

    for adjustment in &entry.adjustments {
        apply_numeric_adjustment(&mut scenario, adjustment)?;
        resolved_parameters.insert(adjustment.id.clone(), adjustment.value);
        scenario.metadata.insert(
            format!("resolved.{}", adjustment.id),
            format!("{:.6}", adjustment.value),
        );
    }

    let (source_pad, target_pad) = configure_transfer_route_geometry(
        &mut scenario,
        route_angle,
        &resolved_radius_tier,
        entry.waypoint_profile.as_deref(),
        resolved_waypoint_envelope,
    )?;
    resolved_parameters.insert("source_x_m".to_owned(), source_pad.center_x_m);
    resolved_parameters.insert("source_y_m".to_owned(), source_pad.surface_y_m);
    resolved_parameters.insert("target_x_m".to_owned(), target_pad.center_x_m);
    resolved_parameters.insert("target_y_m".to_owned(), target_pad.surface_y_m);
    resolved_parameters.insert(
        "route_dx_m".to_owned(),
        target_pad.center_x_m - source_pad.center_x_m,
    );
    resolved_parameters.insert(
        "route_dy_m".to_owned(),
        target_pad.surface_y_m - source_pad.surface_y_m,
    );
    resolved_parameters.insert("start_x_m".to_owned(), scenario.initial_state.position_m.x);
    resolved_parameters.insert("start_y_m".to_owned(), scenario.initial_state.position_m.y);
    if entry.evaluation_goal == TransferMatrixEvaluationGoal::WaypointHandoff {
        scenario.mission.goal = EvaluationGoal::WaypointHandoff {
            target_pad_id: target_pad.id.clone(),
            waypoint_index: 0,
        };
        resolved_parameters.insert("waypoint_handoff_index".to_owned(), 0.0);
    } else if entry.evaluation_goal == TransferMatrixEvaluationGoal::WaypointSequence {
        scenario.mission.goal = EvaluationGoal::WaypointSequence {
            target_pad_id: target_pad.id.clone(),
        };
    }
    if let Some(route) = scenario.mission.transfer_route.as_ref() {
        insert_waypoint_geometry_resolved_parameters(
            &mut resolved_parameters,
            Vec2::new(source_pad.center_x_m, source_pad.surface_y_m),
            Vec2::new(target_pad.center_x_m, target_pad.surface_y_m),
            &scenario.world.terrain,
            &route.waypoints,
            route.route_radius_m,
            &scenario.vehicle,
        )?;
        for (index, waypoint) in route.waypoints.iter().enumerate() {
            let prefix = format!("waypoint_{index}");
            resolved_parameters.insert(format!("{prefix}_x_m"), waypoint.position_m.x);
            resolved_parameters.insert(format!("{prefix}_y_m"), waypoint.position_m.y);
            resolved_parameters.insert(
                format!("{prefix}_capture_radius_m"),
                waypoint.capture_radius_m,
            );
            resolved_parameters.insert(
                format!("{prefix}_max_cross_track_m"),
                waypoint.max_cross_track_m,
            );
            resolved_parameters.insert(
                format!("{prefix}_max_outbound_heading_error_rad"),
                waypoint.max_outbound_heading_error_rad,
            );
            resolved_parameters.insert(
                format!("{prefix}_min_outbound_progress_mps"),
                waypoint.min_outbound_progress_mps,
            );
            if let Some(max_cross_speed_mps) = waypoint.max_outbound_cross_speed_mps {
                resolved_parameters.insert(
                    format!("{prefix}_max_outbound_cross_speed_mps"),
                    max_cross_speed_mps,
                );
            }
            resolved_parameters.insert(format!("{prefix}_min_speed_mps"), waypoint.min_speed_mps);
            resolved_parameters.insert(format!("{prefix}_max_speed_mps"), waypoint.max_speed_mps);
            if let Some(min_vertical_speed_mps) = waypoint.min_vertical_speed_mps {
                resolved_parameters.insert(
                    format!("{prefix}_min_vertical_speed_mps"),
                    min_vertical_speed_mps,
                );
            }
            if let Some(max_vertical_speed_mps) = waypoint.max_vertical_speed_mps {
                resolved_parameters.insert(
                    format!("{prefix}_max_vertical_speed_mps"),
                    max_vertical_speed_mps,
                );
            }
        }
    }

    scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "resolved transfer matrix scenario '{}' {} seed {} failed validation",
                entry.id, route_angle.id, seed_spec.index
            )
        })?;

    let selector = SelectorAxes {
        mission: "transfer_guidance".to_owned(),
        arrival_family: family_spec.route_family.to_owned(),
        condition_set: "clean".to_owned(),
        vehicle_variant: entry.vehicle_variant.clone(),
        arc_point: route_angle.id.to_owned(),
        velocity_band: radius_tier.id.to_owned(),
        route_family: family_spec.route_family.to_owned(),
        route_angle: route_angle.id.to_owned(),
        radius_tier: radius_tier.id.to_owned(),
        waypoint_profile: entry
            .waypoint_profile
            .clone()
            .unwrap_or_else(default_selector_value),
        waypoint_handoff_envelope: resolved_waypoint_envelope
            .map(ToOwned::to_owned)
            .unwrap_or_else(default_selector_value),
        expectation_tier: Some(entry.expectation_tier.clone()),
    };

    Ok((scenario, resolved_parameters, selector))
}

fn configure_transfer_route_geometry(
    scenario: &mut ScenarioSpec,
    route_angle: &TransferRouteAngleSpec,
    radius_tier: &TransferRadiusTierSpec,
    waypoint_profile: Option<&str>,
    waypoint_handoff_envelope: Option<&str>,
) -> Result<(LandingPadSpec, LandingPadSpec)> {
    let target_pad_id = scenario.mission.goal.target_pad_id().to_owned();
    let base_target_pad = scenario
        .world
        .landing_pad(&target_pad_id)
        .cloned()
        .ok_or_else(|| {
            anyhow!("transfer matrix base scenario is missing target pad '{target_pad_id}'")
        })?;
    let route_angle_rad = route_angle.angle_deg.to_radians();
    let dx_m = radius_tier.radius_m * route_angle_rad.cos();
    let dy_m = radius_tier.radius_m * route_angle_rad.sin();
    let target_pad = LandingPadSpec {
        id: target_pad_id,
        center_x_m: 0.0,
        surface_y_m: 0.0,
        width_m: base_target_pad.width_m,
    };
    let source_pad = LandingPadSpec {
        id: "pad_source".to_owned(),
        center_x_m: -dx_m,
        surface_y_m: -dy_m,
        width_m: base_target_pad.width_m,
    };
    let terrain_points = transfer_route_terrain_points(&source_pad, &target_pad)?;

    scenario.world.terrain = TerrainDefinition::Heightfield {
        points_m: terrain_points,
    };
    scenario.world.landing_pads = vec![source_pad.clone(), target_pad.clone()];
    scenario.initial_state.position_m = Vec2::new(
        source_pad.center_x_m,
        source_pad.surface_y_m + scenario.vehicle.geometry.touchdown_base_offset_m,
    );
    scenario.initial_state.velocity_mps = Vec2::new(0.0, 0.0);
    scenario.initial_state.attitude_rad = 0.0;
    scenario.initial_state.angular_rate_radps = 0.0;
    let mut waypoints = transfer_route_waypoints_for_profile(
        waypoint_profile,
        &source_pad,
        &target_pad,
        route_angle,
        radius_tier,
    )?;
    apply_transfer_waypoint_envelope(&mut waypoints, waypoint_handoff_envelope, radius_tier.id)?;
    if let Some(profile) =
        waypoint_profile.filter(|profile| *profile != TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1)
    {
        validate_transfer_waypoint_geometry(
            profile,
            &source_pad,
            &target_pad,
            &scenario.world.terrain,
            &waypoints,
            radius_tier.radius_m,
            scenario.vehicle.geometry.touchdown_base_offset_m,
        )?;
        validate_transfer_waypoint_continuation(
            profile,
            &target_pad,
            &waypoints,
            &scenario.vehicle,
        )?;
        if profile == TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1 {
            validate_transfer_waypoint_turn_authority(
                profile,
                &source_pad,
                &target_pad,
                &waypoints,
                &scenario.vehicle,
            )?;
        }
    }
    scenario.mission.transfer_route = Some(TransferRouteSpec {
        source_pad_id: source_pad.id.clone(),
        target_pad_id: target_pad.id.clone(),
        route_angle_deg: route_angle.angle_deg,
        route_radius_m: radius_tier.radius_m,
        waypoints,
    });

    scenario
        .metadata
        .insert("resolved.source_pad_id".to_owned(), source_pad.id.clone());
    scenario
        .metadata
        .insert("resolved.target_pad_id".to_owned(), target_pad.id.clone());
    scenario.metadata.insert(
        "resolved.route_angle_deg".to_owned(),
        format!("{:.6}", route_angle.angle_deg),
    );
    scenario.metadata.insert(
        "resolved.route_radius_m".to_owned(),
        format!("{:.6}", radius_tier.radius_m),
    );
    scenario.metadata.insert(
        "route_mode".to_owned(),
        waypoint_profile.unwrap_or("direct").to_owned(),
    );
    scenario.metadata.insert(
        "waypoint_profile".to_owned(),
        waypoint_profile.unwrap_or("direct").to_owned(),
    );
    if let Some(envelope) = waypoint_handoff_envelope {
        scenario
            .metadata
            .insert("waypoint_handoff_envelope".to_owned(), envelope.to_owned());
    }
    if let Some(route) = scenario.mission.transfer_route.as_ref() {
        for (index, waypoint) in route.waypoints.iter().enumerate() {
            scenario
                .metadata
                .insert(format!("resolved.waypoint_{index}.id"), waypoint.id.clone());
            scenario.metadata.insert(
                format!("resolved.waypoint_{index}.x_m"),
                format!("{:.6}", waypoint.position_m.x),
            );
            scenario.metadata.insert(
                format!("resolved.waypoint_{index}.y_m"),
                format!("{:.6}", waypoint.position_m.y),
            );
        }
    }

    Ok((source_pad, target_pad))
}

fn transfer_route_waypoints_for_profile(
    waypoint_profile: Option<&str>,
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
    route_angle: &TransferRouteAngleSpec,
    radius_tier: &TransferRadiusTierSpec,
) -> Result<Vec<TransferWaypointSpec>> {
    let Some(profile) = waypoint_profile else {
        return Ok(Vec::new());
    };
    validate_transfer_waypoint_profile("resolved transfer matrix", profile)?;
    match profile {
        TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1 => {
            if route_angle.angle_deg < 70.0 {
                bail!(
                    "waypoint profile '{}' requires a steep uphill route angle, got '{}'",
                    profile,
                    route_angle.id
                );
            }
            let route_dx_m = target_pad.center_x_m - source_pad.center_x_m;
            let direction = if route_dx_m >= 0.0 { 1.0 } else { -1.0 };
            let radius_m = radius_tier.radius_m;
            let capture_radius_m = (radius_m * 0.08).clamp(35.0, 95.0);
            Ok(vec![TransferWaypointSpec {
                id: "wp_dogleg_01".to_owned(),
                position_m: Vec2::new(
                    source_pad.center_x_m - (direction * radius_m * 0.70),
                    target_pad.surface_y_m + (radius_m * 0.45),
                ),
                handoff_tangent_unit: None,
                capture_radius_m,
                max_cross_track_m: capture_radius_m * 1.25,
                max_outbound_heading_error_rad: 0.85,
                min_outbound_progress_mps: 8.0,
                max_outbound_cross_speed_mps: None,
                min_speed_mps: 10.0,
                max_speed_mps: 130.0,
                min_vertical_speed_mps: Some(-80.0),
                max_vertical_speed_mps: Some(65.0),
            }])
        }
        TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1 | TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1 => {
            transfer_route_sequence_waypoints(profile, source_pad, target_pad, radius_tier)
        }
        _ => {
            let profile_spec = transfer_waypoint_bend_profile_spec(profile)
                .expect("validated bend waypoint profile should have geometry");
            if profile_spec
                .min_route_angle_deg
                .is_some_and(|minimum| route_angle.angle_deg < minimum)
            {
                bail!(
                    "waypoint profile '{}' requires a steep uphill route angle, got '{}'",
                    profile,
                    route_angle.id
                );
            }
            let source_m = Vec2::new(source_pad.center_x_m, source_pad.surface_y_m);
            let target_m = Vec2::new(target_pad.center_x_m, target_pad.surface_y_m);
            let route_m = target_m - source_m;
            let route_unit_m = waypoint_normalized(route_m).ok_or_else(|| {
                anyhow!("waypoint profile '{profile}' cannot resolve a zero-length route")
            })?;
            let direction = if route_m.x >= 0.0 { 1.0 } else { -1.0 };
            let source_side_normal_m =
                Vec2::new(-route_unit_m.y * direction, route_unit_m.x * direction);
            let radius_m = radius_tier.radius_m;
            let position_m = source_m
                + (route_m * profile_spec.progress_frac)
                + (source_side_normal_m * (radius_m * profile_spec.lateral_offset_ratio));
            let capture_radius_m = (radius_m * profile_spec.capture_radius_ratio).clamp(
                profile_spec.min_capture_radius_m,
                profile_spec.max_capture_radius_m,
            );
            Ok(vec![TransferWaypointSpec {
                id: profile_spec.waypoint_id.to_owned(),
                position_m,
                handoff_tangent_unit: None,
                capture_radius_m,
                max_cross_track_m: capture_radius_m * profile_spec.max_cross_track_factor,
                max_outbound_heading_error_rad: 0.85,
                min_outbound_progress_mps: 8.0,
                max_outbound_cross_speed_mps: None,
                min_speed_mps: 10.0,
                max_speed_mps: 130.0,
                min_vertical_speed_mps: Some(-80.0),
                max_vertical_speed_mps: Some(65.0),
            }])
        }
    }
}

fn transfer_route_sequence_waypoints(
    profile: &str,
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
    radius_tier: &TransferRadiusTierSpec,
) -> Result<Vec<TransferWaypointSpec>> {
    let source_m = Vec2::new(source_pad.center_x_m, source_pad.surface_y_m);
    let target_m = Vec2::new(target_pad.center_x_m, target_pad.surface_y_m);
    let route_m = target_m - source_m;
    let route_unit_m = waypoint_normalized(route_m).ok_or_else(|| {
        anyhow!("waypoint profile '{profile}' cannot resolve a zero-length route")
    })?;
    let direction = if route_m.x >= 0.0 { 1.0 } else { -1.0 };
    let source_side_normal_m = Vec2::new(-route_unit_m.y * direction, route_unit_m.x * direction);
    let radius_m = radius_tier.radius_m;
    let capture_radius_m = (radius_m * 0.08).clamp(35.0, 95.0);
    let node_specs = match profile {
        TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1 => [
            ("wp_double_bend_01", 0.33, 0.20, 55.0),
            ("wp_double_bend_02", 0.67, 0.20, 65.0),
        ],
        TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1 => [
            ("wp_late_bend_01", 0.33, 0.13, 45.0),
            ("wp_late_bend_02", 0.67, 0.26, 65.0),
        ],
        _ => unreachable!("validated sequence waypoint profile"),
    };
    let mut waypoints = node_specs
        .into_iter()
        .map(
            |(waypoint_id, progress_frac, lateral_offset_ratio, max_speed_mps)| {
                let position_m = source_m
                    + (route_m * progress_frac)
                    + (source_side_normal_m * (radius_m * lateral_offset_ratio));
                TransferWaypointSpec {
                    id: waypoint_id.to_owned(),
                    position_m,
                    handoff_tangent_unit: None,
                    capture_radius_m,
                    max_cross_track_m: capture_radius_m * 1.25,
                    max_outbound_heading_error_rad: 0.35,
                    min_outbound_progress_mps: 8.0,
                    max_outbound_cross_speed_mps: Some(20.0),
                    min_speed_mps: 10.0,
                    max_speed_mps,
                    min_vertical_speed_mps: None,
                    max_vertical_speed_mps: None,
                }
            },
        )
        .collect::<Vec<_>>();
    apply_transfer_waypoint_handoff_tangents(profile, source_m, target_m, &mut waypoints)?;
    Ok(waypoints)
}

fn apply_transfer_waypoint_handoff_tangents(
    profile: &str,
    source_m: Vec2,
    target_m: Vec2,
    waypoints: &mut [TransferWaypointSpec],
) -> Result<()> {
    for index in 0..waypoints.len() {
        let anchor_m = if index == 0 {
            source_m
        } else {
            waypoints[index - 1].position_m
        };
        let next_target_m = waypoints
            .get(index + 1)
            .map_or(target_m, |next| next.position_m);
        let inbound_unit = waypoint_normalized(waypoints[index].position_m - anchor_m)
            .ok_or_else(|| anyhow!("waypoint profile '{profile}' has a zero-length inbound leg"))?;
        let outbound_unit = waypoint_normalized(next_target_m - waypoints[index].position_m)
            .ok_or_else(|| {
                anyhow!("waypoint profile '{profile}' has a zero-length outbound leg")
            })?;
        let tangent = waypoint_normalized(inbound_unit + outbound_unit).ok_or_else(|| {
            anyhow!("waypoint profile '{profile}' cannot bisect opposing route legs")
        })?;
        waypoints[index].handoff_tangent_unit = Some(tangent);
    }
    Ok(())
}

fn validate_transfer_waypoint_geometry(
    profile: &str,
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
    terrain: &TerrainDefinition,
    waypoints: &[TransferWaypointSpec],
    route_radius_m: f64,
    touchdown_base_offset_m: f64,
) -> Result<()> {
    let expectations = transfer_waypoint_geometry_expectations(profile)
        .expect("maintained waypoint profile must define geometry expectations");
    if waypoints.len() != expectations.len() {
        bail!(
            "waypoint profile '{profile}' must resolve exactly {} waypoints, got {}",
            expectations.len(),
            waypoints.len()
        );
    }
    let source_m = Vec2::new(source_pad.center_x_m, source_pad.surface_y_m);
    let target_m = Vec2::new(target_pad.center_x_m, target_pad.surface_y_m);
    let route_m = target_m - source_m;
    let route_length_m = route_m.length();
    let route_unit_m = waypoint_normalized(route_m)
        .ok_or_else(|| anyhow!("waypoint profile '{profile}' resolved a zero-length route"))?;
    let direction = if route_m.x >= 0.0 { 1.0 } else { -1.0 };
    let source_side_normal_m = Vec2::new(-route_unit_m.y * direction, route_unit_m.x * direction);

    let mut previous_progress = 0.0;
    for (index, (waypoint, expected)) in waypoints.iter().zip(expectations).enumerate() {
        let from_source_m = waypoint.position_m - source_m;
        let progress = waypoint_dot(from_source_m, route_unit_m) / route_length_m;
        let signed_offset_ratio =
            waypoint_dot(from_source_m, source_side_normal_m) / route_radius_m;
        if !(progress > previous_progress && progress < 1.0) {
            bail!(
                "waypoint profile '{profile}' waypoint {index} must preserve strict route order, got progress {progress:.6}"
            );
        }
        if signed_offset_ratio <= 0.0 {
            bail!(
                "waypoint profile '{profile}' waypoint {index} must remain on the positive source-side normal, got {signed_offset_ratio:.6}"
            );
        }
        if (progress - expected.progress_frac).abs() > TRANSFER_WAYPOINT_GEOMETRY_TOLERANCE
            || (signed_offset_ratio - expected.lateral_offset_ratio).abs()
                > TRANSFER_WAYPOINT_GEOMETRY_TOLERANCE
        {
            bail!(
                "waypoint profile '{profile}' waypoint {index} route geometry ({progress:.6}, {signed_offset_ratio:.6}) does not match ({:.6}, {:.6})",
                expected.progress_frac,
                expected.lateral_offset_ratio
            );
        }
        previous_progress = progress;
    }

    for (index, pair) in waypoints.windows(2).enumerate() {
        let waypoint_separation_m = (pair[1].position_m - pair[0].position_m).length();
        if waypoint_separation_m <= pair[0].capture_radius_m + pair[1].capture_radius_m {
            bail!(
                "waypoint profile '{profile}' waypoints {index} and {} have overlapping capture regions ({waypoint_separation_m:.3}m)",
                index + 1
            );
        }
    }
    for waypoint in waypoints {
        let terrain_clearance_m =
            waypoint.position_m.y - terrain.sample_height(waypoint.position_m.x);
        let required_clearance_m =
            waypoint.capture_radius_m.max(waypoint.max_cross_track_m) + touchdown_base_offset_m;
        if terrain_clearance_m <= required_clearance_m {
            bail!(
                "waypoint profile '{profile}' waypoint '{}' terrain clearance {:.3}m must exceed {:.3}m",
                waypoint.id,
                terrain_clearance_m,
                required_clearance_m
            );
        }
    }

    let mut geometry_nodes = Vec::with_capacity(waypoints.len() + 2);
    geometry_nodes.push(source_m);
    geometry_nodes.extend(waypoints.iter().map(|waypoint| waypoint.position_m));
    geometry_nodes.push(target_m);
    let mut previous_heading_rad: Option<f64> = None;
    for (segment_index, segment) in geometry_nodes.windows(2).enumerate() {
        let segment_m = segment[1] - segment[0];
        let route_progress_m = waypoint_dot(segment_m, route_unit_m);
        if route_progress_m <= 0.0 {
            bail!(
                "waypoint profile '{profile}' segment {segment_index} reverses route progress ({route_progress_m:.6}m)"
            );
        }
        let heading_rad =
            waypoint_cross(route_unit_m, segment_m).atan2(waypoint_dot(route_unit_m, segment_m));
        if previous_heading_rad
            .is_some_and(|previous| heading_rad >= previous - TRANSFER_WAYPOINT_GEOMETRY_TOLERANCE)
        {
            bail!(
                "waypoint profile '{profile}' segment {segment_index} route-relative heading {:.6}deg must decrease from {:.6}deg",
                heading_rad.to_degrees(),
                previous_heading_rad.unwrap().to_degrees()
            );
        }
        previous_heading_rad = Some(heading_rad);
    }

    for (index, expected) in expectations.iter().enumerate() {
        let inbound_m = geometry_nodes[index + 1] - geometry_nodes[index];
        let outbound_m = geometry_nodes[index + 2] - geometry_nodes[index + 1];
        let signed_turn_deg = waypoint_cross(inbound_m, outbound_m)
            .atan2(waypoint_dot(inbound_m, outbound_m))
            .to_degrees();
        if (signed_turn_deg - expected.signed_turn_deg).abs() > TRANSFER_WAYPOINT_TURN_TOLERANCE_DEG
        {
            bail!(
                "waypoint profile '{profile}' waypoint {index} signed turn {signed_turn_deg:.6}deg must equal {:.6}deg",
                expected.signed_turn_deg
            );
        }
        if matches!(
            profile,
            TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1 | TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1
        ) {
            let expected_tangent = waypoint_normalized(
                waypoint_normalized(inbound_m).unwrap() + waypoint_normalized(outbound_m).unwrap(),
            )
            .expect("validated sequence legs cannot oppose");
            let tangent = waypoints[index]
                .handoff_tangent_unit
                .ok_or_else(|| anyhow!("waypoint profile '{profile}' waypoint {index} requires an explicit handoff tangent"))?;
            if (tangent - expected_tangent).length() > TRANSFER_WAYPOINT_GEOMETRY_TOLERANCE {
                bail!(
                    "waypoint profile '{profile}' waypoint {index} handoff tangent must bisect its route legs"
                );
            }
        }
    }

    // Endpoint legs are dynamically shaped launch/landing trajectories, not straight
    // route segments. Only an explicit multi-waypoint centerline must clear terrain.
    for segment in waypoints.windows(2) {
        for sample_index in 1..24 {
            let t = sample_index as f64 / 24.0;
            let point_m =
                segment[0].position_m + ((segment[1].position_m - segment[0].position_m) * t);
            if point_m.y + 1.0e-6 < terrain.sample_height(point_m.x) + touchdown_base_offset_m {
                bail!(
                    "waypoint profile '{profile}' centerline violates terrain clearance at x={:.3}m",
                    point_m.x
                );
            }
        }
    }
    Ok(())
}

fn transfer_waypoint_geometry_expectations(
    profile: &str,
) -> Option<&'static [TransferWaypointGeometryExpectation]> {
    match profile {
        TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1 => Some(&SINGLE_BEND_GEOMETRY),
        TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1 => Some(&SINGLE_GENTLE_BEND_GEOMETRY),
        TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1 => Some(&SINGLE_MEDIUM_BEND_GEOMETRY),
        TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1 => Some(&SINGLE_SHARP_BEND_GEOMETRY),
        TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1 => Some(&DOUBLE_BEND_GEOMETRY),
        TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1 => Some(&LATE_BEND_GEOMETRY),
        _ => None,
    }
}

fn transfer_waypoint_bend_profile_spec(profile: &str) -> Option<TransferWaypointBendProfileSpec> {
    let balanced = |waypoint_id, lateral_offset_ratio| TransferWaypointBendProfileSpec {
        waypoint_id,
        progress_frac: TRANSFER_WAYPOINT_SINGLE_BEND_PROGRESS_FRAC,
        lateral_offset_ratio,
        capture_radius_ratio: 0.08,
        min_capture_radius_m: 35.0,
        max_capture_radius_m: 95.0,
        max_cross_track_factor: 1.25,
        min_route_angle_deg: None,
    };
    match profile {
        TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1 => Some(TransferWaypointBendProfileSpec {
            waypoint_id: "wp_bend_01",
            progress_frac: TRANSFER_WAYPOINT_SINGLE_BEND_PROGRESS_FRAC,
            lateral_offset_ratio: TRANSFER_WAYPOINT_SINGLE_BEND_LATERAL_OFFSET_RATIO,
            capture_radius_ratio: 0.10,
            min_capture_radius_m: 40.0,
            max_capture_radius_m: 100.0,
            max_cross_track_factor: 1.75,
            min_route_angle_deg: Some(70.0),
        }),
        TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1 => {
            Some(balanced("wp_gentle_bend_01", 0.12))
        }
        TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1 => {
            Some(balanced("wp_medium_bend_01", 0.20))
        }
        TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1 => Some(balanced("wp_sharp_bend_01", 0.30)),
        _ => None,
    }
}

fn apply_transfer_waypoint_envelope(
    waypoints: &mut [TransferWaypointSpec],
    envelope: Option<&str>,
    radius_tier: &str,
) -> Result<()> {
    if !matches!(
        envelope,
        Some(
            TRANSFER_WAYPOINT_ENVELOPE_PASS_THROUGH_V1
                | TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1
                | TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1
        )
    ) {
        return Ok(());
    }
    let continuation_speed_cap_mps =
        if envelope == Some(TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1) {
            Some(match radius_tier {
                // Covers the full pack's -9% radius seed at full initial mass.
                "short" => 52.5,
                "nominal" => 65.0,
                "long" => 75.0,
                _ => bail!("unsupported continuation waypoint radius tier '{radius_tier}'"),
            })
        } else {
            None
        };
    for waypoint in waypoints {
        waypoint.max_outbound_heading_error_rad = 0.35;
        waypoint.min_outbound_progress_mps = 8.0;
        waypoint.max_outbound_cross_speed_mps = Some(20.0);
        waypoint.min_speed_mps = 10.0;
        if envelope == Some(TRANSFER_WAYPOINT_ENVELOPE_PASS_THROUGH_V1) {
            waypoint.max_speed_mps = 130.0;
        } else if let Some(max_speed_mps) = continuation_speed_cap_mps {
            waypoint.max_speed_mps = max_speed_mps;
        }
        waypoint.min_vertical_speed_mps = None;
        waypoint.max_vertical_speed_mps = None;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct TransferWaypointContinuationMetrics {
    available_distance_m: f64,
    optimistic_stop_distance_m: f64,
    stop_ratio: f64,
    max_acceleration_mps2: f64,
}

fn transfer_waypoint_continuation_metrics(
    waypoint: &TransferWaypointSpec,
    outbound_target_m: Vec2,
    vehicle: &VehicleSpec,
) -> Result<TransferWaypointContinuationMetrics> {
    let available_distance_m =
        (outbound_target_m - waypoint.position_m).length() - waypoint.capture_radius_m;
    if available_distance_m <= 0.0 {
        bail!(
            "waypoint '{}' has no continuation distance beyond its capture region",
            waypoint.id
        );
    }
    let initial_mass_kg = vehicle.dry_mass_kg + vehicle.initial_fuel_kg;
    if initial_mass_kg <= 0.0 || vehicle.max_thrust_n <= 0.0 {
        bail!("waypoint continuation requires positive initial mass and maximum thrust");
    }
    let max_acceleration_mps2 = vehicle.max_thrust_n / initial_mass_kg;
    let optimistic_stop_distance_m = waypoint.max_speed_mps.powi(2) / (2.0 * max_acceleration_mps2);
    Ok(TransferWaypointContinuationMetrics {
        available_distance_m,
        optimistic_stop_distance_m,
        stop_ratio: optimistic_stop_distance_m / available_distance_m,
        max_acceleration_mps2,
    })
}

fn validate_transfer_waypoint_continuation(
    profile: &str,
    target_pad: &LandingPadSpec,
    waypoints: &[TransferWaypointSpec],
    vehicle: &VehicleSpec,
) -> Result<()> {
    let target_m = Vec2::new(target_pad.center_x_m, target_pad.surface_y_m);
    for (index, waypoint) in waypoints.iter().enumerate() {
        let outbound_target_m = waypoints
            .get(index + 1)
            .map(|next| next.position_m)
            .unwrap_or(target_m);
        let metrics = transfer_waypoint_continuation_metrics(waypoint, outbound_target_m, vehicle)?;
        if metrics.stop_ratio
            > TRANSFER_WAYPOINT_CONTINUATION_MAX_STOP_RATIO + REGRESSION_POLICY_EPSILON
        {
            bail!(
                "waypoint profile '{profile}' waypoint {index} continuation stop ratio {:.6} exceeds 0.750000",
                metrics.stop_ratio
            );
        }
    }
    Ok(())
}

fn transfer_waypoint_turn_authority_ratio(
    waypoint: &TransferWaypointSpec,
    anchor_m: Vec2,
    next_target_m: Vec2,
    vehicle: &VehicleSpec,
) -> Result<f64> {
    let tangent = waypoint.handoff_tangent_unit.ok_or_else(|| {
        anyhow!(
            "waypoint '{}' requires an explicit handoff tangent",
            waypoint.id
        )
    })?;
    let inbound_m = waypoint.position_m - anchor_m;
    let outbound_m = next_target_m - waypoint.position_m;
    let inbound_unit = waypoint_normalized(inbound_m)
        .ok_or_else(|| anyhow!("waypoint '{}' has a zero-length inbound leg", waypoint.id))?;
    let outbound_unit = waypoint_normalized(outbound_m)
        .ok_or_else(|| anyhow!("waypoint '{}' has a zero-length outbound leg", waypoint.id))?;
    let initial_mass_kg = vehicle.dry_mass_kg + vehicle.initial_fuel_kg;
    let max_acceleration_mps2 = vehicle.max_thrust_n / initial_mass_kg.max(1.0);
    let side_ratio = |leg_m: Vec2, leg_unit: Vec2| {
        let available_distance_m = (leg_m.length() - waypoint.capture_radius_m).max(1.0);
        let deflection_rad = waypoint_dot(leg_unit, tangent).clamp(-1.0, 1.0).acos();
        2.0 * waypoint.max_speed_mps.powi(2) * (deflection_rad * 0.5).sin()
            / (max_acceleration_mps2 * available_distance_m)
    };
    Ok(side_ratio(inbound_m, inbound_unit).max(side_ratio(outbound_m, outbound_unit)))
}

fn validate_transfer_waypoint_turn_authority(
    profile: &str,
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
    waypoints: &[TransferWaypointSpec],
    vehicle: &VehicleSpec,
) -> Result<()> {
    let source_m = Vec2::new(source_pad.center_x_m, source_pad.surface_y_m);
    let target_m = Vec2::new(target_pad.center_x_m, target_pad.surface_y_m);
    for (index, waypoint) in waypoints.iter().enumerate() {
        let anchor_m = if index == 0 {
            source_m
        } else {
            waypoints[index - 1].position_m
        };
        let next_target_m = waypoints
            .get(index + 1)
            .map_or(target_m, |next| next.position_m);
        let ratio =
            transfer_waypoint_turn_authority_ratio(waypoint, anchor_m, next_target_m, vehicle)?;
        if ratio > TRANSFER_WAYPOINT_CONTINUATION_MAX_STOP_RATIO + REGRESSION_POLICY_EPSILON {
            bail!(
                "waypoint profile '{profile}' waypoint {index} optimistic turn-authority ratio {ratio:.6} exceeds 0.750000"
            );
        }
    }
    Ok(())
}

fn insert_waypoint_geometry_resolved_parameters(
    resolved_parameters: &mut BTreeMap<String, f64>,
    source_m: Vec2,
    target_m: Vec2,
    terrain: &TerrainDefinition,
    waypoints: &[TransferWaypointSpec],
    route_radius_m: f64,
    vehicle: &VehicleSpec,
) -> Result<()> {
    let route_m = target_m - source_m;
    let Some(route_unit_m) = waypoint_normalized(route_m) else {
        return Ok(());
    };
    let route_length_m = route_m.length();
    if route_length_m <= 1.0e-9 {
        return Ok(());
    }
    let direction = if route_m.x >= 0.0 { 1.0 } else { -1.0 };
    let source_side_normal_m = Vec2::new(-route_unit_m.y * direction, route_unit_m.x * direction);

    for (index, waypoint) in waypoints.iter().enumerate() {
        let prefix = format!("waypoint_{index}");
        let terrain_y_m = terrain.sample_height(waypoint.position_m.x);
        resolved_parameters.insert(format!("{prefix}_terrain_y_m"), terrain_y_m);
        resolved_parameters.insert(
            format!("{prefix}_terrain_clearance_m"),
            waypoint.position_m.y - terrain_y_m,
        );
        let anchor_m = if index == 0 {
            source_m
        } else {
            waypoints[index - 1].position_m
        };
        let next_target_m = waypoints
            .get(index + 1)
            .map(|next| next.position_m)
            .unwrap_or(target_m);
        let inbound_m = waypoint.position_m - anchor_m;
        let outbound_m = next_target_m - waypoint.position_m;
        let inbound_length_m = inbound_m.length();
        let outbound_length_m = outbound_m.length();
        resolved_parameters.insert(format!("{prefix}_inbound_leg_length_m"), inbound_length_m);
        resolved_parameters.insert(format!("{prefix}_outbound_leg_length_m"), outbound_length_m);
        if let Some(tangent) = waypoint.handoff_tangent_unit {
            let inbound_unit = waypoint_normalized(inbound_m).expect("resolved inbound leg");
            let outbound_unit = waypoint_normalized(outbound_m).expect("resolved outbound leg");
            resolved_parameters.insert(format!("{prefix}_handoff_tangent_x"), tangent.x);
            resolved_parameters.insert(format!("{prefix}_handoff_tangent_y"), tangent.y);
            resolved_parameters.insert(
                format!("{prefix}_handoff_tangent_heading_deg"),
                tangent.y.atan2(tangent.x).to_degrees(),
            );
            resolved_parameters.insert(
                format!("{prefix}_inbound_tangent_angle_deg"),
                waypoint_dot(inbound_unit, tangent)
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees(),
            );
            resolved_parameters.insert(
                format!("{prefix}_tangent_outbound_angle_deg"),
                waypoint_dot(tangent, outbound_unit)
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees(),
            );
            resolved_parameters.insert(
                format!("{prefix}_turn_authority_ratio"),
                transfer_waypoint_turn_authority_ratio(waypoint, anchor_m, next_target_m, vehicle)?,
            );
        }
        let metrics = transfer_waypoint_continuation_metrics(waypoint, next_target_m, vehicle)?;
        resolved_parameters.insert(
            format!("{prefix}_continuation_available_distance_m"),
            metrics.available_distance_m,
        );
        resolved_parameters.insert(
            format!("{prefix}_continuation_optimistic_stop_distance_m"),
            metrics.optimistic_stop_distance_m,
        );
        resolved_parameters.insert(
            format!("{prefix}_continuation_stop_ratio"),
            metrics.stop_ratio,
        );
        resolved_parameters.insert(
            format!("{prefix}_continuation_max_acceleration_mps2"),
            metrics.max_acceleration_mps2,
        );
        if inbound_length_m > 1.0e-9 && outbound_length_m > 1.0e-9 {
            let turn_angle_cos =
                waypoint_dot(inbound_m, outbound_m) / (inbound_length_m * outbound_length_m);
            resolved_parameters.insert(
                format!("{prefix}_turn_angle_deg"),
                turn_angle_cos.clamp(-1.0, 1.0).acos().to_degrees(),
            );
            resolved_parameters.insert(
                format!("{prefix}_signed_turn_angle_deg"),
                waypoint_cross(inbound_m, outbound_m)
                    .atan2(waypoint_dot(inbound_m, outbound_m))
                    .to_degrees(),
            );
        }

        let from_source_m = waypoint.position_m - source_m;
        let profile_progress_frac = waypoint_dot(from_source_m, route_unit_m) / route_length_m;
        let profile_lateral_offset_m = waypoint_cross(from_source_m, route_unit_m).abs();
        let route_signed_offset_m = waypoint_dot(from_source_m, source_side_normal_m);
        resolved_parameters.insert(
            format!("{prefix}_profile_progress_frac"),
            profile_progress_frac,
        );
        resolved_parameters.insert(
            format!("{prefix}_profile_lateral_offset_m"),
            profile_lateral_offset_m,
        );
        resolved_parameters.insert(
            format!("{prefix}_route_signed_offset_m"),
            route_signed_offset_m,
        );
        if route_radius_m > 1.0e-9 {
            resolved_parameters.insert(
                format!("{prefix}_profile_lateral_offset_ratio"),
                profile_lateral_offset_m / route_radius_m,
            );
            resolved_parameters.insert(
                format!("{prefix}_route_signed_offset_ratio"),
                route_signed_offset_m / route_radius_m,
            );
        }
    }
    Ok(())
}

fn transfer_route_terrain_points(
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
) -> Result<Vec<Vec2>> {
    let source_left_m = source_pad.center_x_m - source_pad.half_width_m();
    let source_right_m = source_pad.center_x_m + source_pad.half_width_m();
    let target_left_m = target_pad.center_x_m - target_pad.half_width_m();
    let target_right_m = target_pad.center_x_m + target_pad.half_width_m();
    if source_right_m >= target_left_m {
        bail!(
            "transfer route geometry overlaps source and target pads: source_right={source_right_m:.3}, target_left={target_left_m:.3}"
        );
    }
    let route_span_m = target_pad.center_x_m - source_pad.center_x_m;
    let margin_m = (route_span_m.abs() * 0.15).max(160.0);
    let points = vec![
        Vec2::new(source_pad.center_x_m - margin_m, source_pad.surface_y_m),
        Vec2::new(source_left_m, source_pad.surface_y_m),
        Vec2::new(source_right_m, source_pad.surface_y_m),
        Vec2::new(target_left_m, target_pad.surface_y_m),
        Vec2::new(target_right_m, target_pad.surface_y_m),
        Vec2::new(target_pad.center_x_m + margin_m, target_pad.surface_y_m),
    ];
    validate_transfer_route_terrain(&points, source_pad, target_pad)?;
    Ok(points)
}

fn validate_transfer_route_terrain(
    points: &[Vec2],
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
) -> Result<()> {
    TerrainDefinition::Heightfield {
        points_m: points.to_vec(),
    }
    .validate()
    .map_err(anyhow::Error::msg)?;
    let first_x = points
        .first()
        .map(|point| point.x)
        .ok_or_else(|| anyhow!("transfer route terrain has no points"))?;
    let last_x = points
        .last()
        .map(|point| point.x)
        .ok_or_else(|| anyhow!("transfer route terrain has no points"))?;
    if first_x > source_pad.center_x_m || last_x < target_pad.center_x_m {
        bail!("transfer route terrain does not contain source-to-target route domain");
    }
    let route_sign = (target_pad.surface_y_m - source_pad.surface_y_m).signum();
    if route_sign.abs() > f64::EPSILON {
        for pair in points.windows(2) {
            let delta_y = pair[1].y - pair[0].y;
            if delta_y.signum() != route_sign && delta_y.abs() > 1e-9 {
                bail!("transfer route terrain must be monotonic between source and target");
            }
        }
    }
    Ok(())
}

fn resolve_terminal_matrix_runs(
    entry: &TerminalMatrixEntry,
    base_dir: &Path,
    max_time_s: Option<f64>,
) -> Result<Vec<ResolvedBatchRun>> {
    let base_path = base_dir.join(&entry.base_scenario);
    let base_scenario = load_scenario(&base_path)?;
    let family_spec = terminal_arrival_family_spec(&entry.terminal_matrix)?;
    let arc_specs = selected_terminal_arc_specs(entry, family_spec)?;
    let seed_specs = terminal_seed_specs(entry.seed_tier);
    let mut runs = Vec::new();

    for lane in &entry.lanes {
        let controller_spec = load_controller_spec(
            base_dir,
            lane.controller.as_str(),
            lane.controller_config.as_deref(),
        )?;
        for arc in &arc_specs {
            for band in TERMINAL_BANDS {
                for seed_spec in seed_specs {
                    let run_id = resolved_terminal_matrix_run_id(
                        &entry.id,
                        arc.id,
                        band.id,
                        seed_spec.index,
                        &lane.id,
                    );
                    let (scenario, resolved_parameters, selector) =
                        resolve_terminal_matrix_scenario(
                            entry,
                            &base_scenario,
                            family_spec,
                            arc,
                            band,
                            seed_spec,
                            &lane.id,
                            &run_id,
                            max_time_s,
                        )?;
                    let descriptor = ResolvedRunDescriptor {
                        run_id,
                        entry_id: entry.id.clone(),
                        source_kind: ResolvedRunSourceKind::TerminalMatrix,
                        scenario_source: entry.base_scenario.clone(),
                        resolved_scenario_id: scenario.id.clone(),
                        resolved_scenario_name: scenario.name.clone(),
                        family_id: Some(entry.id.clone()),
                        selector,
                        lane_id: lane.id.clone(),
                        resolved_seed: seed_spec.index,
                        resolved_parameters,
                        controller_id: controller_spec.id().to_owned(),
                        controller_spec: controller_spec.clone(),
                    };
                    runs.push(ResolvedBatchRun {
                        descriptor,
                        scenario,
                    });
                }
            }
        }
    }

    Ok(runs)
}

fn selected_terminal_arc_specs<'a>(
    entry: &TerminalMatrixEntry,
    family_spec: &'a TerminalArrivalFamilySpec,
) -> Result<Vec<&'a TerminalArcPointSpec>> {
    if entry.arc_points.is_empty() {
        return Ok(family_spec.arc_points.iter().collect());
    }

    entry
        .arc_points
        .iter()
        .map(|arc_point| {
            family_spec
                .arc_points
                .iter()
                .find(|candidate| candidate.id == arc_point)
                .with_context(|| {
                    format!(
                        "terminal matrix entry '{}' arc_point selector '{}' is not supported by matrix '{}'",
                        entry.id, arc_point, entry.terminal_matrix
                    )
                })
        })
        .collect()
}

fn terminal_arrival_family_spec(name: &str) -> Result<&'static TerminalArrivalFamilySpec> {
    match name {
        "half_arc_terminal_v1" => Ok(&HALF_ARC_TERMINAL_V1_SPEC),
        _ => bail!("unsupported terminal matrix '{}'", name),
    }
}

fn terminal_seed_specs(seed_tier: TerminalSeedTier) -> &'static [TerminalSeedSpec] {
    match seed_tier {
        TerminalSeedTier::Smoke => &TERMINAL_SMOKE_SEEDS,
        TerminalSeedTier::Full => &TERMINAL_FULL_SEEDS,
    }
}

fn terminal_condition_spec(condition_set: &str) -> Result<TerminalConditionSpec> {
    match condition_set {
        "clean" => Ok(TerminalConditionSpec::Clean),
        "traj_undershoot_small" => Ok(TerminalConditionSpec::ProjectedError(
            TerminalProjectedErrorSpec {
                kind: TerminalProjectedErrorKind::Undershoot,
                severity: "small",
                magnitudes_m: [30.0, 45.0, 60.0],
            },
        )),
        "traj_undershoot_large" => Ok(TerminalConditionSpec::ProjectedError(
            TerminalProjectedErrorSpec {
                kind: TerminalProjectedErrorKind::Undershoot,
                severity: "large",
                magnitudes_m: [75.0, 90.0, 105.0],
            },
        )),
        "traj_overshoot_small" => Ok(TerminalConditionSpec::ProjectedError(
            TerminalProjectedErrorSpec {
                kind: TerminalProjectedErrorKind::Overshoot,
                severity: "small",
                magnitudes_m: [30.0, 45.0, 60.0],
            },
        )),
        "traj_overshoot_large" => Ok(TerminalConditionSpec::ProjectedError(
            TerminalProjectedErrorSpec {
                kind: TerminalProjectedErrorKind::Overshoot,
                severity: "large",
                magnitudes_m: [75.0, 90.0, 105.0],
            },
        )),
        "terrain_backstop_wall" => Ok(TerminalConditionSpec::ReactiveTerrain(
            TerminalReactiveTerrainSpec {
                hazard: TerminalReactiveTerrainHazard::ContainmentBackstop,
                variant: "wall",
                height_offset_m: 400.0,
                pad_clearance_gap_m: 30.0,
                shoulder_width_m: 8.0,
                top_width_m: 120.0,
            },
        )),
        "terrain_backstop_slanted" => Ok(TerminalConditionSpec::ReactiveTerrain(
            TerminalReactiveTerrainSpec {
                hazard: TerminalReactiveTerrainHazard::ContainmentBackstop,
                variant: "slanted",
                height_offset_m: 400.0,
                pad_clearance_gap_m: 30.0,
                shoulder_width_m: 90.0,
                top_width_m: 70.0,
            },
        )),
        "terrain_clip" => Ok(TerminalConditionSpec::ReactiveTerrain(
            TerminalReactiveTerrainSpec {
                hazard: TerminalReactiveTerrainHazard::DescentClip,
                variant: "clip",
                height_offset_m: 220.0,
                pad_clearance_gap_m: 24.0,
                shoulder_width_m: 36.0,
                top_width_m: 34.0,
            },
        )),
        _ => bail!("unsupported condition_set '{condition_set}'"),
    }
}

fn resolved_terminal_matrix_run_id(
    entry_id: &str,
    arc_point: &str,
    velocity_band: &str,
    seed: u64,
    lane_id: &str,
) -> String {
    sanitize_token(&format!(
        "{entry_id}__{arc_point}__{velocity_band}__seed_{seed:02}__{lane_id}"
    ))
}

fn resolve_terminal_matrix_scenario(
    entry: &TerminalMatrixEntry,
    base_scenario: &ScenarioSpec,
    family_spec: &TerminalArrivalFamilySpec,
    arc: &TerminalArcPointSpec,
    band: TerminalBandSpec,
    seed_spec: &TerminalSeedSpec,
    lane_id: &str,
    run_id: &str,
    max_time_s: Option<f64>,
) -> Result<(ScenarioSpec, BTreeMap<String, f64>, SelectorAxes)> {
    let mut scenario = base_scenario.clone();
    scenario.id = run_id.to_owned();
    scenario.name = format!(
        "{} [{} {} {} {} {} seed {} {}]",
        base_scenario.name,
        family_spec.arrival_family,
        entry.condition_set,
        entry.vehicle_variant,
        arc.id,
        band.id,
        seed_spec.index,
        lane_id
    );
    scenario.description = format!(
        "{} ({} {} {} {} {} {} seed {} lane {})",
        base_scenario.description,
        "terminal_matrix",
        family_spec.arrival_family,
        entry.condition_set,
        entry.vehicle_variant,
        arc.id,
        band.id,
        seed_spec.index,
        lane_id
    );
    scenario.seed = seed_spec.index;
    scenario.tags = merge_unique_tags(&base_scenario.tags, &entry.tags);
    scenario.metadata.extend(entry.metadata.clone());
    scenario
        .metadata
        .insert("family".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("family_entry_id".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("resolved_seed".to_owned(), seed_spec.index.to_string());
    scenario
        .metadata
        .insert("mission".to_owned(), "terminal_guidance".to_owned());
    scenario.metadata.insert(
        "arrival_family".to_owned(),
        family_spec.arrival_family.to_owned(),
    );
    scenario
        .metadata
        .insert("condition_set".to_owned(), entry.condition_set.clone());
    scenario
        .metadata
        .insert("vehicle_variant".to_owned(), entry.vehicle_variant.clone());
    scenario.metadata.insert(
        "expectation_tier".to_owned(),
        entry.expectation_tier.clone(),
    );
    scenario
        .metadata
        .insert("arc_point".to_owned(), arc.id.to_owned());
    scenario
        .metadata
        .insert("velocity_band".to_owned(), band.id.to_owned());
    scenario
        .metadata
        .insert("lane_id".to_owned(), lane_id.to_owned());

    scenario.world.gravity_mps2 = family_spec.gravity_mps2;
    let reachability_max_time_s = scenario.sim.max_time_s;
    scenario.metadata.insert(
        "resolved.reachability_max_time_s".to_owned(),
        format!("{:.6}", reachability_max_time_s),
    );
    if let Some(eval_max_time_s) = max_time_s {
        if eval_max_time_s < reachability_max_time_s {
            bail!(
                "terminal_matrix_max_time_s ({eval_max_time_s:.3}) must be >= scenario reachability max_time_s ({reachability_max_time_s:.3}) for terminal matrix entry '{}'",
                entry.id
            );
        }
        scenario.sim.max_time_s = eval_max_time_s;
        scenario.metadata.insert(
            "resolved.eval_max_time_s".to_owned(),
            format!("{:.6}", eval_max_time_s),
        );
    }

    let mut resolved_parameters = BTreeMap::new();
    resolved_parameters.insert("gravity_mps2".to_owned(), family_spec.gravity_mps2);
    resolved_parameters.insert(
        "reachability_max_time_s".to_owned(),
        reachability_max_time_s,
    );
    if let Some(eval_max_time_s) = max_time_s {
        resolved_parameters.insert("eval_max_time_s".to_owned(), eval_max_time_s);
    }
    resolved_parameters.insert("radius_nominal_m".to_owned(), family_spec.radius_nominal_m);
    resolved_parameters.insert("arc_angle_deg".to_owned(), arc.angle_deg);
    resolved_parameters.insert("mid_ttg_s".to_owned(), arc.nominal_ttg_s);

    for adjustment in &entry.adjustments {
        apply_numeric_adjustment(&mut scenario, adjustment)?;
        resolved_parameters.insert(adjustment.id.clone(), adjustment.value);
        scenario.metadata.insert(
            format!("resolved.{}", adjustment.id),
            format!("{:.6}", adjustment.value),
        );
    }

    let ttg_s = terminal_band_ttg(family_spec, arc, band.id);
    resolved_parameters.insert("ttg_s".to_owned(), ttg_s);

    let (side_label, side_sign) = resolved_side(arc.id, seed_spec.index);
    scenario
        .metadata
        .insert("resolved.side".to_owned(), side_label.to_owned());
    resolved_parameters.insert("side_sign".to_owned(), side_sign);

    let radial_jitter_m = seed_spec
        .radial_pct
        .map(|pct| (family_spec.radius_nominal_m * pct).clamp(-30.0, 30.0))
        .unwrap_or(0.0);
    let resolved_radius_m = family_spec.radius_nominal_m + radial_jitter_m;
    resolved_parameters.insert("radial_jitter_m".to_owned(), radial_jitter_m);
    resolved_parameters.insert("radius_m".to_owned(), resolved_radius_m);
    if let Some(radial_pct) = seed_spec.radial_pct {
        scenario
            .metadata
            .insert("resolved.seed_variation".to_owned(), "radial".to_owned());
        scenario
            .metadata
            .insert("resolved.radial_pct".to_owned(), format!("{radial_pct:.6}"));
    } else if let Some(speed_pct) = seed_spec.speed_pct {
        scenario
            .metadata
            .insert("resolved.seed_variation".to_owned(), "speed".to_owned());
        scenario
            .metadata
            .insert("resolved.speed_pct".to_owned(), format!("{speed_pct:.6}"));
    } else {
        scenario
            .metadata
            .insert("resolved.seed_variation".to_owned(), "none".to_owned());
    }

    let angle_rad = arc.angle_deg.to_radians();
    let x_m = if arc.id == "a00" {
        0.0
    } else {
        side_sign * resolved_radius_m * angle_rad.sin()
    };
    let y_m = resolved_radius_m * angle_rad.cos();
    resolved_parameters.insert("start_x_m".to_owned(), x_m);
    resolved_parameters.insert("start_y_m".to_owned(), y_m);

    let (clean_vx_mps, clean_vy_mps) =
        solve_ballistic_velocity(x_m, y_m, ttg_s, family_spec.gravity_mps2);
    let condition_spec = terminal_condition_spec(&entry.condition_set)?;
    scenario.metadata.insert(
        "resolved.condition_kind".to_owned(),
        condition_spec.kind_label().to_owned(),
    );
    let mut vx_mps = clean_vx_mps;
    let mut vy_mps = clean_vy_mps;
    let mut speed_scale = 1.0;
    let mut projected_dx_error_m = 0.0;
    let mut projected_dx_error_mag_m = 0.0;
    let mut traj_error_approach_sign = if x_m.abs() > f64::EPSILON {
        x_m.signum()
    } else if seed_spec.index % 2 == 0 {
        -1.0
    } else {
        1.0
    };

    if let TerminalConditionSpec::ProjectedError(error_spec) = condition_spec {
        let magnitude_index = seed_spec
            .error_level_index
            .min(error_spec.magnitudes_m.len().saturating_sub(1));
        projected_dx_error_mag_m = error_spec.magnitudes_m[magnitude_index];
        let error_sign = match error_spec.kind {
            TerminalProjectedErrorKind::Undershoot => traj_error_approach_sign,
            TerminalProjectedErrorKind::Overshoot => -traj_error_approach_sign,
        };
        projected_dx_error_m = error_sign * projected_dx_error_mag_m;
        vx_mps = (projected_dx_error_m - x_m) / ttg_s;
        scenario.metadata.insert(
            "resolved.traj_error_kind".to_owned(),
            error_spec.kind.as_str().to_owned(),
        );
        scenario.metadata.insert(
            "resolved.traj_error_severity".to_owned(),
            error_spec.severity.to_owned(),
        );
        scenario.metadata.insert(
            "resolved.seed_variation".to_owned(),
            "projected_error".to_owned(),
        );
        scenario.metadata.remove("resolved.speed_pct");
    } else {
        traj_error_approach_sign = 0.0;
        speed_scale = 1.0 + seed_spec.speed_pct.unwrap_or(0.0);
        vx_mps *= speed_scale;
        vy_mps *= speed_scale;
        scenario
            .metadata
            .insert("resolved.traj_error_kind".to_owned(), "none".to_owned());
        scenario
            .metadata
            .insert("resolved.traj_error_severity".to_owned(), "none".to_owned());
    }
    let engine_off_impact_x_m = x_m + (vx_mps * ttg_s);
    resolved_parameters.insert("clean_start_vx_mps".to_owned(), clean_vx_mps);
    resolved_parameters.insert("clean_start_vy_mps".to_owned(), clean_vy_mps);
    resolved_parameters.insert("projected_dx_error_m".to_owned(), projected_dx_error_m);
    resolved_parameters.insert(
        "projected_dx_error_mag_m".to_owned(),
        projected_dx_error_mag_m,
    );
    resolved_parameters.insert(
        "traj_error_approach_sign".to_owned(),
        traj_error_approach_sign,
    );
    resolved_parameters.insert("engine_off_impact_x_m".to_owned(), engine_off_impact_x_m);
    resolved_parameters.insert("speed_scale".to_owned(), speed_scale);
    resolved_parameters.insert("start_vx_mps".to_owned(), vx_mps);
    resolved_parameters.insert("start_vy_mps".to_owned(), vy_mps);
    resolved_parameters.insert(
        "start_speed_mps".to_owned(),
        (vx_mps.powi(2) + vy_mps.powi(2)).sqrt(),
    );

    scenario.initial_state.position_m.x = x_m;
    scenario.initial_state.position_m.y = y_m;
    scenario.initial_state.velocity_mps.x = vx_mps;
    scenario.initial_state.velocity_mps.y = vy_mps;

    if let TerminalConditionSpec::ReactiveTerrain(terrain_spec) = condition_spec {
        apply_terminal_reactive_terrain(
            &mut scenario,
            terrain_spec,
            side_sign,
            seed_spec.index,
            &mut resolved_parameters,
        )?;
    }

    scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "resolved terminal matrix scenario '{}' {} {} seed {} failed validation",
                entry.id, arc.id, band.id, seed_spec.index
            )
        })?;

    let selector = SelectorAxes {
        mission: "terminal_guidance".to_owned(),
        arrival_family: family_spec.arrival_family.to_owned(),
        condition_set: entry.condition_set.clone(),
        vehicle_variant: entry.vehicle_variant.clone(),
        arc_point: arc.id.to_owned(),
        velocity_band: band.id.to_owned(),
        route_family: default_selector_value(),
        route_angle: default_selector_value(),
        radius_tier: default_selector_value(),
        waypoint_profile: default_selector_value(),
        waypoint_handoff_envelope: default_selector_value(),
        expectation_tier: Some(entry.expectation_tier.clone()),
    };

    Ok((scenario, resolved_parameters, selector))
}

fn terminal_band_ttg(
    family_spec: &TerminalArrivalFamilySpec,
    arc: &TerminalArcPointSpec,
    band_id: &str,
) -> f64 {
    match band_id {
        "mid" => arc.nominal_ttg_s,
        "low" => {
            let low = arc.nominal_ttg_s * family_spec.low_multiplier;
            if family_spec.clamp_low_to_descending {
                let y_m = family_spec.radius_nominal_m * arc.angle_deg.to_radians().cos();
                let t_flat_s = ((2.0 * y_m) / family_spec.gravity_mps2).sqrt();
                low.min(t_flat_s * 0.98)
            } else {
                low
            }
        }
        "high" => arc.nominal_ttg_s * family_spec.high_multiplier,
        _ => arc.nominal_ttg_s,
    }
}

fn resolved_side(arc_point: &str, seed: u64) -> (&'static str, f64) {
    if arc_point == "a00" {
        ("center", 0.0)
    } else if seed % 2 == 0 {
        ("left", -1.0)
    } else {
        ("right", 1.0)
    }
}

fn solve_ballistic_velocity(x_m: f64, y_m: f64, ttg_s: f64, gravity_mps2: f64) -> (f64, f64) {
    let vx_mps = -x_m / ttg_s;
    let vy_mps = ((0.5 * gravity_mps2 * ttg_s * ttg_s) - y_m) / ttg_s;
    (vx_mps, vy_mps)
}

fn apply_terminal_reactive_terrain(
    scenario: &mut ScenarioSpec,
    terrain_spec: TerminalReactiveTerrainSpec,
    side_sign: f64,
    seed_index: u64,
    resolved_parameters: &mut BTreeMap<String, f64>,
) -> Result<()> {
    let Some(target_pad) = scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.id == scenario.mission.goal.target_pad_id())
    else {
        bail!(
            "terminal reactive terrain condition requires target pad '{}'",
            scenario.mission.goal.target_pad_id()
        );
    };
    let target_center_x_m = target_pad.center_x_m;
    let target_surface_y_m = target_pad.surface_y_m;
    let pad_half_width_m = target_pad.half_width_m();
    let approach_side_sign = terminal_terrain_approach_side_sign(side_sign, seed_index);
    let feature_side_sign = match terrain_spec.hazard {
        TerminalReactiveTerrainHazard::ContainmentBackstop => -approach_side_sign,
        TerminalReactiveTerrainHazard::DescentClip => approach_side_sign,
    };
    let inner_offset_m = pad_half_width_m + terrain_spec.pad_clearance_gap_m;
    let far_offset_m = (scenario.initial_state.position_m.x - target_center_x_m)
        .abs()
        .max(900.0)
        + 240.0;
    let terrain_points = match terrain_spec.hazard {
        TerminalReactiveTerrainHazard::ContainmentBackstop => terminal_backstop_profile_points(
            target_center_x_m,
            target_surface_y_m,
            feature_side_sign,
            far_offset_m,
            inner_offset_m,
            terrain_spec,
        ),
        TerminalReactiveTerrainHazard::DescentClip => terminal_clip_profile_points(
            target_center_x_m,
            target_surface_y_m,
            feature_side_sign,
            far_offset_m,
            inner_offset_m,
            terrain_spec,
        ),
    }?;

    scenario.world.terrain = TerrainDefinition::Heightfield {
        points_m: terrain_points,
    };
    scenario.metadata.insert(
        "resolved.reactive_contract".to_owned(),
        "execution_guardrail".to_owned(),
    );
    scenario.metadata.insert(
        "resolved.reactive_trigger".to_owned(),
        "execution_drift".to_owned(),
    );
    scenario.metadata.insert(
        "resolved.primary_navigation_owner".to_owned(),
        "terminal_guidance".to_owned(),
    );
    scenario.metadata.insert(
        "resolved.nominal_route_must_clear".to_owned(),
        "true".to_owned(),
    );
    scenario.metadata.insert(
        "resolved.hazard_driver".to_owned(),
        terrain_spec.hazard.as_str().to_owned(),
    );
    scenario.metadata.insert(
        "resolved.obstacle_kind".to_owned(),
        terrain_spec.hazard.obstacle_kind().to_owned(),
    );
    scenario.metadata.insert(
        "resolved.obstacle_placement".to_owned(),
        terrain_spec.hazard.obstacle_placement().to_owned(),
    );
    scenario.metadata.insert(
        "resolved.terrain_variant".to_owned(),
        terrain_spec.variant.to_owned(),
    );
    scenario.metadata.insert(
        "resolved.terrain_feature_side".to_owned(),
        side_label_for_sign(feature_side_sign).to_owned(),
    );
    scenario.metadata.insert(
        "resolved.terrain_visibility".to_owned(),
        "startup_context".to_owned(),
    );

    resolved_parameters.insert("terrain_feature_side_sign".to_owned(), feature_side_sign);
    resolved_parameters.insert("terrain_approach_side_sign".to_owned(), approach_side_sign);
    resolved_parameters.insert(
        "terrain_height_offset_m".to_owned(),
        terrain_spec.height_offset_m,
    );
    resolved_parameters.insert("terrain_inner_offset_m".to_owned(), inner_offset_m);
    resolved_parameters.insert(
        "terrain_pad_clearance_gap_m".to_owned(),
        terrain_spec.pad_clearance_gap_m,
    );
    resolved_parameters.insert(
        "terrain_shoulder_width_m".to_owned(),
        terrain_spec.shoulder_width_m,
    );
    resolved_parameters.insert("terrain_top_width_m".to_owned(), terrain_spec.top_width_m);
    resolved_parameters.insert("terrain_far_offset_m".to_owned(), far_offset_m);

    Ok(())
}

fn terminal_terrain_approach_side_sign(side_sign: f64, seed_index: u64) -> f64 {
    if side_sign.abs() > f64::EPSILON {
        side_sign.signum()
    } else if seed_index % 2 == 0 {
        -1.0
    } else {
        1.0
    }
}

fn side_label_for_sign(side_sign: f64) -> &'static str {
    if side_sign < 0.0 { "left" } else { "right" }
}

fn terminal_backstop_profile_points(
    target_center_x_m: f64,
    target_surface_y_m: f64,
    feature_side_sign: f64,
    far_offset_m: f64,
    inner_offset_m: f64,
    terrain_spec: TerminalReactiveTerrainSpec,
) -> Result<Vec<Vec2>> {
    let ramp_end_m = inner_offset_m + terrain_spec.shoulder_width_m;
    let plateau_end_m = ramp_end_m + terrain_spec.top_width_m;
    terrain_points_from_signed_profile(
        target_center_x_m,
        feature_side_sign,
        &[
            (-far_offset_m, target_surface_y_m),
            (inner_offset_m, target_surface_y_m),
            (
                ramp_end_m,
                target_surface_y_m + terrain_spec.height_offset_m,
            ),
            (
                plateau_end_m,
                target_surface_y_m + terrain_spec.height_offset_m,
            ),
            (
                far_offset_m,
                target_surface_y_m + terrain_spec.height_offset_m,
            ),
        ],
    )
}

fn terminal_clip_profile_points(
    target_center_x_m: f64,
    target_surface_y_m: f64,
    feature_side_sign: f64,
    far_offset_m: f64,
    inner_offset_m: f64,
    terrain_spec: TerminalReactiveTerrainSpec,
) -> Result<Vec<Vec2>> {
    let ramp_up_end_m = inner_offset_m + terrain_spec.shoulder_width_m;
    let plateau_end_m = ramp_up_end_m + terrain_spec.top_width_m;
    let outer_end_m = plateau_end_m + terrain_spec.shoulder_width_m;
    terrain_points_from_signed_profile(
        target_center_x_m,
        feature_side_sign,
        &[
            (-far_offset_m, target_surface_y_m),
            (inner_offset_m, target_surface_y_m),
            (
                ramp_up_end_m,
                target_surface_y_m + terrain_spec.height_offset_m,
            ),
            (
                plateau_end_m,
                target_surface_y_m + terrain_spec.height_offset_m,
            ),
            (outer_end_m, target_surface_y_m),
            (far_offset_m, target_surface_y_m),
        ],
    )
}

fn terrain_points_from_signed_profile(
    target_center_x_m: f64,
    feature_side_sign: f64,
    signed_profile: &[(f64, f64)],
) -> Result<Vec<Vec2>> {
    let mut points: Vec<Vec2> = signed_profile
        .iter()
        .map(|(signed_offset_m, y_m)| {
            Vec2::new(
                target_center_x_m + (feature_side_sign * signed_offset_m),
                *y_m,
            )
        })
        .collect();
    points.sort_by(|left, right| {
        left.x
            .partial_cmp(&right.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for pair in points.windows(2) {
        if pair[1].x <= pair[0].x {
            bail!("terminal reactive terrain produced duplicate terrain x coordinates");
        }
    }
    Ok(points)
}

fn family_entry_seeds(entry: &ScenarioFamilyEntry) -> Vec<u64> {
    if !entry.seeds.is_empty() {
        return entry.seeds.clone();
    }

    let range = entry
        .seed_range
        .as_ref()
        .expect("validated family entries always define seed source");
    (range.start..range.start.saturating_add(range.count)).collect()
}

fn resolve_family_scenario(
    entry: &ScenarioFamilyEntry,
    base_scenario: &ScenarioSpec,
    seed: u64,
) -> Result<(ScenarioSpec, BTreeMap<String, f64>)> {
    let mut scenario = base_scenario.clone();
    scenario.id = resolved_family_run_id(&entry.id, seed);
    scenario.name = format!("{} [{} seed {}]", base_scenario.name, entry.family, seed);
    scenario.description = format!(
        "{} (family {} seed {})",
        base_scenario.description, entry.family, seed
    );
    scenario.seed = seed;
    scenario.tags = merge_unique_tags(&base_scenario.tags, &entry.tags);
    scenario.metadata.extend(entry.metadata.clone());
    scenario
        .metadata
        .insert("family".to_owned(), entry.family.clone());
    scenario
        .metadata
        .insert("family_entry_id".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("resolved_seed".to_owned(), seed.to_string());

    let mut resolved_parameters = BTreeMap::new();
    for perturbation in &entry.perturbations {
        let sampled_value = sample_perturbation_value(entry, perturbation, seed);
        apply_numeric_perturbation(&mut scenario, perturbation, sampled_value)?;
        resolved_parameters.insert(perturbation.id.clone(), sampled_value);
        scenario.metadata.insert(
            format!("resolved.{}", perturbation.id),
            format!("{sampled_value:.6}"),
        );
    }

    scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "resolved family scenario '{}' seed {} failed validation",
                entry.family, seed
            )
        })?;

    Ok((scenario, resolved_parameters))
}

fn merge_unique_tags(base_tags: &[String], extra_tags: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(base_tags.len() + extra_tags.len());
    let mut seen = BTreeSet::new();
    for tag in base_tags.iter().chain(extra_tags.iter()) {
        if seen.insert(tag.clone()) {
            merged.push(tag.clone());
        }
    }
    merged
}

fn selector_axes_from_metadata(metadata: &BTreeMap<String, String>) -> SelectorAxes {
    SelectorAxes {
        mission: selector_value(metadata.get("mission"), "unspecified"),
        arrival_family: selector_value(metadata.get("arrival_family"), "unspecified"),
        condition_set: selector_value(metadata.get("condition_set"), "unspecified"),
        vehicle_variant: selector_value(metadata.get("vehicle_variant"), "unspecified"),
        arc_point: selector_value(metadata.get("arc_point"), "unspecified"),
        velocity_band: selector_value(metadata.get("velocity_band"), "unspecified"),
        route_family: selector_value(metadata.get("route_family"), "unspecified"),
        route_angle: selector_value(metadata.get("route_angle"), "unspecified"),
        radius_tier: selector_value(metadata.get("radius_tier"), "unspecified"),
        waypoint_profile: selector_value(
            metadata
                .get("waypoint_profile")
                .or_else(|| metadata.get("route_mode")),
            "unspecified",
        ),
        waypoint_handoff_envelope: selector_value(
            metadata.get("waypoint_handoff_envelope"),
            "unspecified",
        ),
        expectation_tier: metadata
            .get("expectation_tier")
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
    }
}

fn selector_value(value: Option<&String>, fallback: &str) -> String {
    value
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_owned()
}

fn is_supported_numeric_path(path: &str) -> bool {
    matches!(
        path,
        "world.gravity_mps2"
            | "vehicle.dry_mass_kg"
            | "vehicle.initial_fuel_kg"
            | "vehicle.max_fuel_kg"
            | "vehicle.max_thrust_n"
            | "vehicle.max_fuel_burn_kgps"
            | "vehicle.max_rotation_rate_radps"
            | "initial_state.position_m.x"
            | "initial_state.position_m.y"
            | "initial_state.velocity_mps.x"
            | "initial_state.velocity_mps.y"
            | "initial_state.attitude_rad"
            | "initial_state.angular_rate_radps"
    )
}

fn is_supported_terminal_adjustment_path(path: &str) -> bool {
    matches!(
        path,
        "vehicle.dry_mass_kg"
            | "vehicle.initial_fuel_kg"
            | "vehicle.max_fuel_kg"
            | "vehicle.max_thrust_n"
            | "vehicle.max_fuel_burn_kgps"
            | "vehicle.max_rotation_rate_radps"
            | "initial_state.attitude_rad"
            | "initial_state.angular_rate_radps"
    )
}

fn apply_numeric_perturbation(
    scenario: &mut ScenarioSpec,
    perturbation: &NumericPerturbationSpec,
    sampled_value: f64,
) -> Result<()> {
    match perturbation.path.as_str() {
        "world.gravity_mps2" => Ok(apply_numeric_mode(
            &mut scenario.world.gravity_mps2,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.dry_mass_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.dry_mass_kg,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.initial_fuel_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.initial_fuel_kg,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.max_fuel_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_fuel_kg,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.max_thrust_n" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_thrust_n,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.max_fuel_burn_kgps" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_fuel_burn_kgps,
            perturbation.mode,
            sampled_value,
        )),
        "vehicle.max_rotation_rate_radps" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_rotation_rate_radps,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.position_m.x" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.position_m.x,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.position_m.y" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.position_m.y,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.velocity_mps.x" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.velocity_mps.x,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.velocity_mps.y" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.velocity_mps.y,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.attitude_rad" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.attitude_rad,
            perturbation.mode,
            sampled_value,
        )),
        "initial_state.angular_rate_radps" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.angular_rate_radps,
            perturbation.mode,
            sampled_value,
        )),
        _ => bail!(
            "unsupported numeric perturbation path '{}'",
            perturbation.path
        ),
    }
}

fn apply_numeric_adjustment(
    scenario: &mut ScenarioSpec,
    adjustment: &NumericAdjustmentSpec,
) -> Result<()> {
    match adjustment.path.as_str() {
        "vehicle.dry_mass_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.dry_mass_kg,
            adjustment.mode,
            adjustment.value,
        )),
        "vehicle.initial_fuel_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.initial_fuel_kg,
            adjustment.mode,
            adjustment.value,
        )),
        "vehicle.max_fuel_kg" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_fuel_kg,
            adjustment.mode,
            adjustment.value,
        )),
        "vehicle.max_thrust_n" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_thrust_n,
            adjustment.mode,
            adjustment.value,
        )),
        "vehicle.max_fuel_burn_kgps" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_fuel_burn_kgps,
            adjustment.mode,
            adjustment.value,
        )),
        "vehicle.max_rotation_rate_radps" => Ok(apply_numeric_mode(
            &mut scenario.vehicle.max_rotation_rate_radps,
            adjustment.mode,
            adjustment.value,
        )),
        "initial_state.attitude_rad" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.attitude_rad,
            adjustment.mode,
            adjustment.value,
        )),
        "initial_state.angular_rate_radps" => Ok(apply_numeric_mode(
            &mut scenario.initial_state.angular_rate_radps,
            adjustment.mode,
            adjustment.value,
        )),
        _ => bail!("unsupported numeric adjustment path '{}'", adjustment.path),
    }
}

fn apply_numeric_mode(target: &mut f64, mode: NumericPerturbationMode, value: f64) {
    match mode {
        NumericPerturbationMode::Set => *target = value,
        NumericPerturbationMode::Offset => *target += value,
        NumericPerturbationMode::Scale => *target *= value,
    }
}

fn sample_perturbation_value(
    entry: &ScenarioFamilyEntry,
    perturbation: &NumericPerturbationSpec,
    seed: u64,
) -> f64 {
    let random_value = stable_unit_interval(
        seed,
        &format!("{}::{}::{}", entry.family, entry.id, perturbation.id),
    );
    let sampled = perturbation.min + ((perturbation.max - perturbation.min) * random_value);
    if let Some(step) = perturbation.quantize {
        (sampled / step).round() * step
    } else {
        sampled
    }
}

fn execute_resolved_runs(
    resolved_runs: &[ResolvedBatchRun],
    output_dir: Option<&Path>,
    workers_used: usize,
) -> Result<Vec<BatchRunRecord>> {
    if workers_used <= 1 {
        return resolved_runs
            .iter()
            .map(|run| execute_resolved_run(run, output_dir))
            .collect();
    }

    let pool = ThreadPoolBuilder::new()
        .num_threads(workers_used)
        .build()
        .context("failed to build pd-eval thread pool")?;

    let results = pool.install(|| {
        resolved_runs
            .par_iter()
            .map(|run| execute_resolved_run(run, output_dir))
            .collect::<Vec<_>>()
    });

    results.into_iter().collect()
}

fn analytic_feasibility_for_run(resolved_run: &ResolvedBatchRun) -> BatchRunAnalyticFeasibility {
    if near_vertical_transfer_route_frontier(resolved_run) {
        return BatchRunAnalyticFeasibility {
            class: BatchRunAnalyticClass::Frontier,
            reason: Some(BatchRunAnalyticReason::NearVerticalTransferRoute),
            ..Default::default()
        };
    }

    if !matches!(
        resolved_run.descriptor.source_kind,
        ResolvedRunSourceKind::TerminalMatrix
    ) {
        return BatchRunAnalyticFeasibility::default();
    }

    let scenario = &resolved_run.scenario;
    let Some(target_pad) = scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.id == scenario.mission.goal.target_pad_id())
    else {
        return BatchRunAnalyticFeasibility::default();
    };

    let mass_kg = scenario.vehicle.dry_mass_kg + scenario.vehicle.initial_fuel_kg;
    let gravity_mps2 = scenario.world.gravity_mps2.abs().max(1e-6);
    let max_upward_accel_mps2 = (scenario.vehicle.max_thrust_n / mass_kg.max(1.0)) - gravity_mps2;
    let downward_speed_mps = (-scenario.initial_state.velocity_mps.y).max(0.0);
    let safe_touchdown_speed_mps = scenario.vehicle.safe_touchdown_normal_speed_mps;
    let available_stop_height_m = scenario.initial_state.position_m.y
        - target_pad.surface_y_m
        - scenario.vehicle.geometry.touchdown_base_offset_m;

    let required_stop_height_m = if downward_speed_mps <= safe_touchdown_speed_mps {
        0.0
    } else if max_upward_accel_mps2 <= 0.0 {
        f64::INFINITY
    } else {
        ((downward_speed_mps * downward_speed_mps)
            - (safe_touchdown_speed_mps * safe_touchdown_speed_mps))
            / (2.0 * max_upward_accel_mps2)
    };
    let stop_height_margin_m = available_stop_height_m - required_stop_height_m;

    if stop_height_margin_m < 0.0 {
        BatchRunAnalyticFeasibility {
            class: BatchRunAnalyticClass::Impossible,
            reason: Some(BatchRunAnalyticReason::VerticalStopHeight),
            available_stop_height_m: Some(available_stop_height_m),
            required_stop_height_m: Some(required_stop_height_m),
            stop_height_margin_m: Some(stop_height_margin_m),
            ..Default::default()
        }
    } else {
        let coupled = coupled_stop_acceleration_bound(scenario, target_pad);
        if coupled.stop_accel_margin_mps2 < 0.0 {
            return BatchRunAnalyticFeasibility {
                class: BatchRunAnalyticClass::Impossible,
                reason: Some(BatchRunAnalyticReason::CoupledStopAcceleration),
                available_stop_height_m: Some(available_stop_height_m),
                required_stop_height_m: Some(required_stop_height_m),
                stop_height_margin_m: Some(stop_height_margin_m),
                available_stop_accel_mps2: Some(coupled.available_accel_mps2),
                required_stop_accel_mps2: Some(coupled.required_accel_mps2),
                stop_accel_margin_mps2: Some(coupled.stop_accel_margin_mps2),
            };
        }

        let class = if low_thrust_high_energy_frontier(
            resolved_run,
            max_upward_accel_mps2,
            gravity_mps2,
            coupled.stop_accel_margin_mps2,
        ) {
            BatchRunAnalyticClass::Frontier
        } else {
            BatchRunAnalyticClass::Scored
        };
        let reason = if matches!(class, BatchRunAnalyticClass::Frontier) {
            Some(BatchRunAnalyticReason::LowThrustHighEnergy)
        } else {
            None
        };

        BatchRunAnalyticFeasibility {
            class,
            reason,
            available_stop_height_m: Some(available_stop_height_m),
            required_stop_height_m: Some(required_stop_height_m),
            stop_height_margin_m: Some(stop_height_margin_m),
            available_stop_accel_mps2: Some(coupled.available_accel_mps2),
            required_stop_accel_mps2: Some(coupled.required_accel_mps2),
            stop_accel_margin_mps2: Some(coupled.stop_accel_margin_mps2),
        }
    }
}

const AUTHORITY_FRONTIER_MARGIN_GRAVITY_RATIO: f64 = 0.45;
const NEAR_VERTICAL_TRANSFER_ROUTE_MIN_DEG: f64 = 75.0;

fn near_vertical_transfer_route_frontier(resolved_run: &ResolvedBatchRun) -> bool {
    if !matches!(
        resolved_run.descriptor.source_kind,
        ResolvedRunSourceKind::TransferMatrix
    ) {
        return false;
    }

    resolved_run
        .scenario
        .mission
        .transfer_route
        .as_ref()
        .is_some_and(|route| route.route_angle_deg >= NEAR_VERTICAL_TRANSFER_ROUTE_MIN_DEG)
}

fn low_thrust_high_energy_frontier(
    resolved_run: &ResolvedBatchRun,
    upright_net_accel_mps2: f64,
    gravity_mps2: f64,
    stop_accel_margin_mps2: f64,
) -> bool {
    if resolved_run.descriptor.selector.velocity_band != "high" {
        return false;
    }

    let low_authority_margin_mps2 =
        AUTHORITY_FRONTIER_MARGIN_GRAVITY_RATIO * gravity_mps2.max(1e-6);
    upright_net_accel_mps2 <= low_authority_margin_mps2
        && stop_accel_margin_mps2 <= low_authority_margin_mps2
}

#[derive(Clone, Copy, Debug)]
struct CoupledStopAccelerationBound {
    available_accel_mps2: f64,
    required_accel_mps2: f64,
    stop_accel_margin_mps2: f64,
}

const REACHABILITY_TIME_STEP_S: f64 = 0.1;

fn coupled_stop_acceleration_bound(
    scenario: &ScenarioSpec,
    target_pad: &LandingPadSpec,
) -> CoupledStopAccelerationBound {
    let gravity_mps2 = scenario.world.gravity_mps2.abs().max(1e-6);
    let mass_kg = scenario.vehicle.dry_mass_kg + scenario.vehicle.initial_fuel_kg;
    let x0_m = scenario.initial_state.position_m.x;
    let y0_m = scenario.initial_state.position_m.y;
    let vx_mps = scenario.initial_state.velocity_mps.x;
    let vy_mps = scenario.initial_state.velocity_mps.y;
    let safe_lateral_speed_mps = scenario.vehicle.safe_touchdown_tangential_speed_mps;
    let touchdown_center_limit_m =
        (target_pad.half_width_m() - scenario.vehicle.geometry.touchdown_half_span_m).max(0.0);
    let safe_vertical_speed_mps = scenario.vehicle.safe_touchdown_normal_speed_mps;
    let target_y_m = target_pad.surface_y_m + scenario.vehicle.geometry.touchdown_base_offset_m;
    let target_min_x_m = target_pad.center_x_m - touchdown_center_limit_m;
    let target_max_x_m = target_pad.center_x_m + touchdown_center_limit_m;
    let max_time_s = scenario
        .metadata
        .get("resolved.reachability_max_time_s")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(scenario.sim.max_time_s)
        .max(REACHABILITY_TIME_STEP_S);
    let steps = (max_time_s / REACHABILITY_TIME_STEP_S).ceil() as u64;
    let mut best: Option<CoupledStopAccelerationBound> = None;

    // Sweep possible touchdown times and use optimistic double-integrator lower bounds.
    // If even this lower bound exceeds full-throttle authority, the run is outside the envelope.
    for step in 1..=steps {
        let time_s = (step as f64 * REACHABILITY_TIME_STEP_S).min(max_time_s);
        let ballistic_x_m = x0_m + vx_mps * time_s;
        let required_lateral_position_accel_mps2 = 2.0
            * distance_outside_interval_m(ballistic_x_m, target_min_x_m, target_max_x_m)
            / (time_s * time_s);
        let required_lateral_velocity_accel_mps2 =
            distance_outside_interval_m(vx_mps, -safe_lateral_speed_mps, safe_lateral_speed_mps)
                / time_s;
        let required_lateral_accel_mps2 =
            required_lateral_position_accel_mps2.max(required_lateral_velocity_accel_mps2);

        let freefall_y_m = y0_m + vy_mps * time_s - 0.5 * gravity_mps2 * time_s * time_s;
        let required_upward_displacement_m = target_y_m - freefall_y_m;
        if required_upward_displacement_m < -1e-6 {
            continue;
        }
        let required_vertical_position_accel_mps2 =
            (2.0 * required_upward_displacement_m.max(0.0)) / (time_s * time_s);
        let freefall_vy_mps = vy_mps - gravity_mps2 * time_s;
        if freefall_vy_mps > 1e-6 {
            continue;
        }
        let required_vertical_velocity_accel_mps2 = if freefall_vy_mps < -safe_vertical_speed_mps {
            (-safe_vertical_speed_mps - freefall_vy_mps) / time_s
        } else {
            0.0
        };
        let required_vertical_accel_mps2 =
            required_vertical_position_accel_mps2.max(required_vertical_velocity_accel_mps2);
        let required_accel_mps2 = (required_lateral_accel_mps2 * required_lateral_accel_mps2
            + required_vertical_accel_mps2 * required_vertical_accel_mps2)
            .sqrt();
        let available_accel_mps2 =
            full_throttle_average_accel_mps2(&scenario.vehicle, mass_kg, time_s);
        let stop_accel_margin_mps2 = available_accel_mps2 - required_accel_mps2;
        let candidate = CoupledStopAccelerationBound {
            available_accel_mps2,
            required_accel_mps2,
            stop_accel_margin_mps2,
        };

        if best
            .map(|best| stop_accel_margin_mps2 > best.stop_accel_margin_mps2)
            .unwrap_or(true)
        {
            best = Some(candidate);
        }
    }

    best.unwrap_or_else(|| {
        let available_accel_mps2 =
            full_throttle_average_accel_mps2(&scenario.vehicle, mass_kg, max_time_s);
        CoupledStopAccelerationBound {
            available_accel_mps2,
            required_accel_mps2: f64::INFINITY,
            stop_accel_margin_mps2: f64::NEG_INFINITY,
        }
    })
}

fn distance_outside_interval_m(value: f64, min_value: f64, max_value: f64) -> f64 {
    if value < min_value {
        min_value - value
    } else if value > max_value {
        value - max_value
    } else {
        0.0
    }
}

fn full_throttle_average_accel_mps2(
    vehicle: &VehicleSpec,
    initial_mass_kg: f64,
    terminal_window_s: f64,
) -> f64 {
    let mass0 = initial_mass_kg.max(1.0);
    let burn_window_s = terminal_window_s.max(1e-6);
    let fuel_used_kg =
        (vehicle.max_fuel_burn_kgps.max(0.0) * burn_window_s).min(vehicle.initial_fuel_kg.max(0.0));
    let mass1 = (mass0 - fuel_used_kg).max(vehicle.dry_mass_kg.max(1.0));
    if fuel_used_kg <= 1e-9 || vehicle.max_fuel_burn_kgps <= 1e-9 {
        return vehicle.max_thrust_n / mass0;
    }

    vehicle.max_thrust_n / (vehicle.max_fuel_burn_kgps * burn_window_s) * (mass0 / mass1).ln()
}

fn execute_resolved_run(
    resolved_run: &ResolvedBatchRun,
    output_dir: Option<&Path>,
) -> Result<BatchRunRecord> {
    let ctx = RunContext::from_scenario(&resolved_run.scenario)
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "failed to build run context for resolved run {}",
                resolved_run.descriptor.run_id
            )
        })?;
    let artifacts = run_controller_spec(&ctx, &resolved_run.descriptor.controller_spec)
        .with_context(|| {
            format!(
                "failed to run controller for resolved run {}",
                resolved_run.descriptor.run_id
            )
        })?;

    let bundle_dir = output_dir.map(|root| root.join("runs").join(&resolved_run.descriptor.run_id));
    if let Some(bundle_dir) = bundle_dir.as_deref() {
        write_artifact_bundle(
            bundle_dir,
            &resolved_run.scenario,
            &resolved_run.descriptor.controller_spec,
            &artifacts,
        )?;
    }

    let review = derive_run_review_metrics(&resolved_run.scenario, &artifacts);
    let analytic = analytic_feasibility_for_run(resolved_run);

    Ok(BatchRunRecord {
        resolved: resolved_run.descriptor.clone(),
        manifest: artifacts.run.manifest,
        review,
        analytic,
        bundle_dir: bundle_dir.map(|path| path.to_string_lossy().into_owned()),
    })
}

fn record_scored(record: &BatchRunRecord) -> bool {
    record.analytic.is_scored()
}

fn record_invalidated(record: &BatchRunRecord) -> bool {
    !record_scored(record)
}

fn record_success(record: &BatchRunRecord) -> bool {
    record_scored(record) && matches!(record.manifest.mission_outcome, MissionOutcome::Success)
}

fn record_failure(record: &BatchRunRecord) -> bool {
    record_scored(record) && !matches!(record.manifest.mission_outcome, MissionOutcome::Success)
}

fn summarize_records(records: &[BatchRunRecord]) -> BatchSummary {
    let total_runs = records.len();
    let invalidated_runs = records
        .iter()
        .filter(|record| record_invalidated(record))
        .count();
    let success_runs = records
        .iter()
        .filter(|record| record_success(record))
        .count();
    let failure_runs = records
        .iter()
        .filter(|record| record_failure(record))
        .count();
    let mean_sim_time_s = if total_runs == 0 {
        0.0
    } else {
        records
            .iter()
            .map(|record| record.manifest.sim_time_s)
            .sum::<f64>()
            / total_runs as f64
    };
    let max_sim_time_s = records
        .iter()
        .map(|record| record.manifest.sim_time_s)
        .fold(0.0_f64, f64::max);

    let mut mission_outcomes = BTreeMap::new();
    let mut physical_outcomes = BTreeMap::new();
    let mut end_reasons = BTreeMap::new();
    for record in records {
        *mission_outcomes
            .entry(enum_label(&record.manifest.mission_outcome))
            .or_insert(0) += 1;
        *physical_outcomes
            .entry(enum_label(&record.manifest.physical_outcome))
            .or_insert(0) += 1;
        *end_reasons
            .entry(enum_label(&record.manifest.end_reason))
            .or_insert(0) += 1;
    }

    let mut by_entry_groups = BTreeMap::<String, Vec<&BatchRunRecord>>::new();
    let mut by_family_groups = BTreeMap::<String, Vec<&BatchRunRecord>>::new();
    for record in records {
        by_entry_groups
            .entry(record.resolved.entry_id.clone())
            .or_default()
            .push(record);
        if let Some(family_id) = record.resolved.family_id.clone() {
            by_family_groups.entry(family_id).or_default().push(record);
        }
    }

    let by_entry = by_entry_groups
        .into_iter()
        .map(|(key, group)| summarize_group(&key, &group))
        .collect();
    let by_family = by_family_groups
        .into_iter()
        .map(|(key, group)| summarize_group(&key, &group))
        .collect();

    let mut failed_runs = records
        .iter()
        .filter(|record| record_failure(record))
        .map(run_pointer)
        .collect::<Vec<_>>();
    failed_runs.sort_by(|lhs, rhs| {
        lhs.entry_id
            .cmp(&rhs.entry_id)
            .then(lhs.scenario_seed.cmp(&rhs.scenario_seed))
            .then(lhs.run_id.cmp(&rhs.run_id))
    });

    let mut slowest_runs = records.iter().map(run_pointer).collect::<Vec<_>>();
    slowest_runs.sort_by(|lhs, rhs| {
        rhs.sim_time_s
            .partial_cmp(&lhs.sim_time_s)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(lhs.run_id.cmp(&rhs.run_id))
    });
    slowest_runs.truncate(10);

    let mut closest_failures = records
        .iter()
        .filter(|record| record_failure(record))
        .map(run_pointer)
        .collect::<Vec<_>>();
    closest_failures.sort_by(closest_failure_sort_key);
    closest_failures.truncate(10);

    let mut worst_failures = records
        .iter()
        .filter(|record| record_failure(record))
        .map(run_pointer)
        .collect::<Vec<_>>();
    worst_failures.sort_by(worst_failure_sort_key);
    worst_failures.truncate(10);

    let mut weakest_successes = records
        .iter()
        .filter(|record| record_success(record))
        .map(run_pointer)
        .collect::<Vec<_>>();
    weakest_successes.sort_by(weakest_success_sort_key);
    weakest_successes.truncate(10);

    let mut lowest_fuel_successes = records
        .iter()
        .filter(|record| record_success(record))
        .map(run_pointer)
        .collect::<Vec<_>>();
    lowest_fuel_successes.sort_by(lowest_fuel_success_sort_key);
    lowest_fuel_successes.truncate(10);

    BatchSummary {
        total_runs,
        success_runs,
        failure_runs,
        invalidated_runs,
        mean_sim_time_s,
        max_sim_time_s,
        mission_outcomes,
        physical_outcomes,
        end_reasons,
        by_entry,
        by_family,
        failed_runs,
        slowest_runs,
        closest_failures,
        worst_failures,
        weakest_successes,
        lowest_fuel_successes,
    }
}

fn compare_summary_delta(candidate: &BatchSummary, baseline: &BatchSummary) -> BatchSummaryDelta {
    BatchSummaryDelta {
        candidate_success_rate: success_rate(
            candidate.success_runs,
            candidate
                .total_runs
                .saturating_sub(candidate.invalidated_runs),
        ),
        baseline_success_rate: success_rate(
            baseline.success_runs,
            baseline
                .total_runs
                .saturating_sub(baseline.invalidated_runs),
        ),
        success_rate_delta: success_rate(
            candidate.success_runs,
            candidate
                .total_runs
                .saturating_sub(candidate.invalidated_runs),
        ) - success_rate(
            baseline.success_runs,
            baseline
                .total_runs
                .saturating_sub(baseline.invalidated_runs),
        ),
        candidate_success_runs: candidate.success_runs,
        baseline_success_runs: baseline.success_runs,
        success_runs_delta: candidate.success_runs as i64 - baseline.success_runs as i64,
        candidate_failure_runs: candidate.failure_runs,
        baseline_failure_runs: baseline.failure_runs,
        failure_runs_delta: candidate.failure_runs as i64 - baseline.failure_runs as i64,
        candidate_invalidated_runs: candidate.invalidated_runs,
        baseline_invalidated_runs: baseline.invalidated_runs,
        invalidated_runs_delta: candidate.invalidated_runs as i64
            - baseline.invalidated_runs as i64,
        candidate_mean_sim_time_s: candidate.mean_sim_time_s,
        baseline_mean_sim_time_s: baseline.mean_sim_time_s,
        mean_sim_time_delta_s: candidate.mean_sim_time_s - baseline.mean_sim_time_s,
        candidate_max_sim_time_s: candidate.max_sim_time_s,
        baseline_max_sim_time_s: baseline.max_sim_time_s,
        max_sim_time_delta_s: candidate.max_sim_time_s - baseline.max_sim_time_s,
    }
}

fn compare_group_sets(
    candidate_groups: &[BatchGroupSummary],
    baseline_groups: &[BatchGroupSummary],
) -> Vec<BatchGroupComparison> {
    let candidate_map = candidate_groups
        .iter()
        .map(|group| (group.key.clone(), group))
        .collect::<BTreeMap<_, _>>();
    let baseline_map = baseline_groups
        .iter()
        .map(|group| (group.key.clone(), group))
        .collect::<BTreeMap<_, _>>();
    let keys = candidate_map
        .keys()
        .chain(baseline_map.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    keys.into_iter()
        .map(|key| {
            let candidate = candidate_map.get(&key).copied();
            let baseline = baseline_map.get(&key).copied();
            BatchGroupComparison {
                key,
                candidate_total_runs: candidate.map(|group| group.total_runs),
                baseline_total_runs: baseline.map(|group| group.total_runs),
                candidate_success_rate: candidate.map(|group| {
                    success_rate(
                        group.success_runs,
                        group.total_runs.saturating_sub(group.invalidated_runs),
                    )
                }),
                baseline_success_rate: baseline.map(|group| {
                    success_rate(
                        group.success_runs,
                        group.total_runs.saturating_sub(group.invalidated_runs),
                    )
                }),
                success_rate_delta: match (candidate, baseline) {
                    (Some(candidate), Some(baseline)) => Some(
                        success_rate(
                            candidate.success_runs,
                            candidate
                                .total_runs
                                .saturating_sub(candidate.invalidated_runs),
                        ) - success_rate(
                            baseline.success_runs,
                            baseline
                                .total_runs
                                .saturating_sub(baseline.invalidated_runs),
                        ),
                    ),
                    _ => None,
                },
                candidate_failure_runs: candidate.map(|group| group.failure_runs),
                baseline_failure_runs: baseline.map(|group| group.failure_runs),
                failure_runs_delta: match (candidate, baseline) {
                    (Some(candidate), Some(baseline)) => {
                        Some(candidate.failure_runs as i64 - baseline.failure_runs as i64)
                    }
                    _ => None,
                },
                candidate_invalidated_runs: candidate.map(|group| group.invalidated_runs),
                baseline_invalidated_runs: baseline.map(|group| group.invalidated_runs),
                invalidated_runs_delta: match (candidate, baseline) {
                    (Some(candidate), Some(baseline)) => {
                        Some(candidate.invalidated_runs as i64 - baseline.invalidated_runs as i64)
                    }
                    _ => None,
                },
                candidate_mean_sim_time_s: candidate.map(|group| group.mean_sim_time_s),
                baseline_mean_sim_time_s: baseline.map(|group| group.mean_sim_time_s),
                mean_sim_time_delta_s: match (candidate, baseline) {
                    (Some(candidate), Some(baseline)) => {
                        Some(candidate.mean_sim_time_s - baseline.mean_sim_time_s)
                    }
                    _ => None,
                },
                candidate_failed_seeds: candidate
                    .map(|group| group.failed_seeds.clone())
                    .unwrap_or_default(),
                baseline_failed_seeds: baseline
                    .map(|group| group.failed_seeds.clone())
                    .unwrap_or_default(),
                sample_run_ids: candidate
                    .map(|group| group.sample_run_ids.clone())
                    .or_else(|| baseline.map(|group| group.sample_run_ids.clone()))
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn compare_run_pair(
    candidate_record: &BatchRunRecord,
    baseline_record: &BatchRunRecord,
) -> Option<BatchRunComparison> {
    let candidate_success = matches!(
        candidate_record.manifest.mission_outcome,
        MissionOutcome::Success
    );
    let baseline_success = matches!(
        baseline_record.manifest.mission_outcome,
        MissionOutcome::Success
    );
    let candidate_mission_outcome = enum_label(&candidate_record.manifest.mission_outcome);
    let baseline_mission_outcome = enum_label(&baseline_record.manifest.mission_outcome);
    let candidate_end_reason = enum_label(&candidate_record.manifest.end_reason);
    let baseline_end_reason = enum_label(&baseline_record.manifest.end_reason);
    let sim_time_delta_s =
        candidate_record.manifest.sim_time_s - baseline_record.manifest.sim_time_s;
    let candidate_margin_ratio = summary_margin_ratio(&candidate_record.manifest.summary);
    let baseline_margin_ratio = summary_margin_ratio(&baseline_record.manifest.summary);
    let margin_ratio_delta = match (candidate_margin_ratio, baseline_margin_ratio) {
        (Some(candidate), Some(baseline)) => Some(candidate - baseline),
        _ => None,
    };
    let candidate_fuel_remaining_kg = candidate_record.manifest.summary.fuel_remaining_kg;
    let baseline_fuel_remaining_kg = baseline_record.manifest.summary.fuel_remaining_kg;

    let change_kind = if baseline_success && !candidate_success {
        BatchRunChangeKind::NewFailure
    } else if !baseline_success && candidate_success {
        BatchRunChangeKind::Recovered
    } else if candidate_mission_outcome != baseline_mission_outcome
        || candidate_end_reason != baseline_end_reason
        || sim_time_delta_s.abs() > 1e-9
    {
        BatchRunChangeKind::OutcomeChanged
    } else {
        return None;
    };

    Some(BatchRunComparison {
        run_id: candidate_record.resolved.run_id.clone(),
        entry_id: candidate_record.resolved.entry_id.clone(),
        family_id: candidate_record.resolved.family_id.clone(),
        selector: candidate_record.resolved.selector.clone(),
        lane_id: candidate_record.resolved.lane_id.clone(),
        change_kind,
        candidate_seed: candidate_record.manifest.scenario_seed,
        baseline_seed: baseline_record.manifest.scenario_seed,
        candidate_mission_outcome,
        baseline_mission_outcome,
        candidate_end_reason,
        baseline_end_reason,
        candidate_sim_time_s: candidate_record.manifest.sim_time_s,
        baseline_sim_time_s: baseline_record.manifest.sim_time_s,
        sim_time_delta_s,
        candidate_bundle_dir: candidate_record.bundle_dir.clone(),
        baseline_bundle_dir: baseline_record.bundle_dir.clone(),
        candidate_margin_ratio,
        baseline_margin_ratio,
        margin_ratio_delta,
        candidate_fuel_remaining_kg,
        baseline_fuel_remaining_kg,
        fuel_remaining_delta_kg: candidate_fuel_remaining_kg - baseline_fuel_remaining_kg,
    })
}

fn summarize_group(key: &str, records: &[&BatchRunRecord]) -> BatchGroupSummary {
    let total_runs = records.len();
    let invalidated_runs = records
        .iter()
        .filter(|record| record_invalidated(record))
        .count();
    let success_runs = records
        .iter()
        .filter(|record| record_success(record))
        .count();
    let failure_runs = records
        .iter()
        .filter(|record| record_failure(record))
        .count();
    let mean_sim_time_s = if total_runs == 0 {
        0.0
    } else {
        records
            .iter()
            .map(|record| record.manifest.sim_time_s)
            .sum::<f64>()
            / total_runs as f64
    };

    let mut mission_outcomes = BTreeMap::new();
    let mut end_reasons = BTreeMap::new();
    let mut failed_seeds = BTreeSet::new();
    let mut sample_run_ids = Vec::new();
    let mut success_fuel_remaining = Vec::new();
    let mut success_fuel_used_pct = Vec::new();
    let mut success_landing_offset_abs_m = Vec::new();
    let mut success_low_altitude_dwell_s = Vec::new();
    let mut success_low_altitude_unsafe_recovery_s = Vec::new();
    let mut success_reference_gap_mean_m = Vec::new();
    let mut success_transfer_shape_curve_rmse_m = Vec::new();
    let mut success_transfer_shape_apex_error_m = Vec::new();
    let mut success_transfer_shape_projected_dx_abs_mean_m = Vec::new();
    let mut success_transfer_shape_shortfall_ratio = Vec::new();
    let mut success_transfer_terminal_post_handoff_apex_gain_m = Vec::new();
    let mut success_transfer_terminal_post_handoff_time_to_apex_s = Vec::new();
    let mut success_transfer_terminal_post_handoff_apex_dx_abs_m = Vec::new();
    let mut success_sim_time_s = Vec::new();
    let mut success_pointers = Vec::new();
    let mut failure_pointers = Vec::new();

    for record in records {
        *mission_outcomes
            .entry(enum_label(&record.manifest.mission_outcome))
            .or_insert(0) += 1;
        *end_reasons
            .entry(enum_label(&record.manifest.end_reason))
            .or_insert(0) += 1;
        let pointer = run_pointer(record);
        if record_failure(record) {
            failed_seeds.insert(record.resolved.resolved_seed);
            failure_pointers.push(pointer);
        } else if record_success(record) {
            success_fuel_remaining.push(record.manifest.summary.fuel_remaining_kg);
            success_sim_time_s.push(record.manifest.sim_time_s);
            if let Some(value) = record.review.fuel_used_pct_of_max {
                success_fuel_used_pct.push(value);
            }
            if let Some(value) = record.review.landing_offset_abs_m {
                success_landing_offset_abs_m.push(value);
            }
            if let Some(value) = record.review.low_altitude_dwell_s {
                success_low_altitude_dwell_s.push(value);
            }
            if let Some(value) = record.review.low_altitude_unsafe_recovery_s {
                success_low_altitude_unsafe_recovery_s.push(value);
            }
            if let Some(value) = record.review.reference_gap_mean_m {
                success_reference_gap_mean_m.push(value);
            }
            if let Some(value) = record.review.transfer_shape_curve_rmse_m {
                success_transfer_shape_curve_rmse_m.push(value);
            }
            if let Some(value) = record.review.transfer_shape_apex_error_m {
                success_transfer_shape_apex_error_m.push(value);
            }
            if let Some(value) = record.review.transfer_shape_projected_dx_abs_mean_m {
                success_transfer_shape_projected_dx_abs_mean_m.push(value);
            }
            if let Some(value) = record.review.transfer_shape_shortfall_ratio {
                success_transfer_shape_shortfall_ratio.push(value);
            }
            if let Some(value) = record.review.transfer_terminal_post_handoff_apex_gain_m {
                success_transfer_terminal_post_handoff_apex_gain_m.push(value);
            }
            if let Some(value) = record.review.transfer_terminal_post_handoff_time_to_apex_s {
                success_transfer_terminal_post_handoff_time_to_apex_s.push(value);
            }
            if let Some(value) = record.review.transfer_terminal_post_handoff_apex_dx_abs_m {
                success_transfer_terminal_post_handoff_apex_dx_abs_m.push(value);
            }
            success_pointers.push(pointer);
        }
        if sample_run_ids.len() < 5 {
            sample_run_ids.push(record.resolved.run_id.clone());
        }
    }
    let mean_success_fuel_remaining_kg = if success_fuel_remaining.is_empty() {
        None
    } else {
        Some(success_fuel_remaining.iter().sum::<f64>() / success_fuel_remaining.len() as f64)
    };
    success_pointers.sort_by(weakest_success_sort_key);
    failure_pointers.sort_by(closest_failure_sort_key);
    let closest_failure_run_id = failure_pointers
        .first()
        .map(|pointer| pointer.run_id.clone());
    failure_pointers.sort_by(worst_failure_sort_key);
    let worst_failure_run_id = failure_pointers
        .first()
        .map(|pointer| pointer.run_id.clone());

    BatchGroupSummary {
        key: key.to_owned(),
        total_runs,
        success_runs,
        failure_runs,
        invalidated_runs,
        mean_sim_time_s,
        sim_time_stats: metric_summary(&success_sim_time_s),
        mean_success_fuel_remaining_kg,
        fuel_used_pct_of_max: metric_summary(&success_fuel_used_pct),
        landing_offset_abs_m: metric_summary(&success_landing_offset_abs_m),
        low_altitude_dwell_s: metric_summary(&success_low_altitude_dwell_s),
        low_altitude_unsafe_recovery_s: metric_summary(&success_low_altitude_unsafe_recovery_s),
        reference_gap_mean_m: metric_summary(&success_reference_gap_mean_m),
        transfer_shape_curve_rmse_m: metric_summary(&success_transfer_shape_curve_rmse_m),
        transfer_shape_apex_error_m: metric_summary(&success_transfer_shape_apex_error_m),
        transfer_shape_projected_dx_abs_mean_m: metric_summary(
            &success_transfer_shape_projected_dx_abs_mean_m,
        ),
        transfer_shape_shortfall_ratio: metric_summary(&success_transfer_shape_shortfall_ratio),
        transfer_terminal_post_handoff_apex_gain_m: metric_summary(
            &success_transfer_terminal_post_handoff_apex_gain_m,
        ),
        transfer_terminal_post_handoff_time_to_apex_s: metric_summary(
            &success_transfer_terminal_post_handoff_time_to_apex_s,
        ),
        transfer_terminal_post_handoff_apex_dx_abs_m: metric_summary(
            &success_transfer_terminal_post_handoff_apex_dx_abs_m,
        ),
        mission_outcomes,
        end_reasons,
        sample_run_ids,
        failed_seeds: failed_seeds.into_iter().collect(),
        weakest_success_run_id: success_pointers
            .first()
            .map(|pointer| pointer.run_id.clone()),
        closest_failure_run_id,
        worst_failure_run_id,
    }
}

fn run_pointer(record: &BatchRunRecord) -> BatchRunPointer {
    BatchRunPointer {
        run_id: record.resolved.run_id.clone(),
        entry_id: record.resolved.entry_id.clone(),
        family_id: record.resolved.family_id.clone(),
        selector: record.resolved.selector.clone(),
        lane_id: record.resolved.lane_id.clone(),
        scenario_id: record.manifest.scenario_id.clone(),
        scenario_seed: record.manifest.scenario_seed,
        controller_id: record.manifest.controller_id.clone(),
        mission_outcome: enum_label(&record.manifest.mission_outcome),
        end_reason: enum_label(&record.manifest.end_reason),
        sim_time_s: record.manifest.sim_time_s,
        bundle_dir: record.bundle_dir.clone(),
        margin_ratio: summary_margin_ratio(&record.manifest.summary),
        fuel_remaining_kg: record.manifest.summary.fuel_remaining_kg,
        review: record.review.clone(),
        analytic: record.analytic.clone(),
        summary: record.manifest.summary.clone(),
    }
}

fn metric_summary(values: &[f64]) -> Option<BatchMetricSummary> {
    if values.is_empty() {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let delta = *value - mean;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    Some(BatchMetricSummary {
        mean,
        stddev: Some(variance.sqrt()),
    })
}

fn derive_run_review_metrics(
    scenario: &ScenarioSpec,
    artifacts: &ControlledRunArtifacts,
) -> BatchRunReviewMetrics {
    let run = &artifacts.run;
    let fuel_used_pct_of_max = (scenario.vehicle.max_fuel_kg > 1e-9)
        .then(|| (run.manifest.summary.fuel_used_kg / scenario.vehicle.max_fuel_kg) * 100.0);
    let landing_offset_abs_m = run
        .manifest
        .summary
        .landing
        .as_ref()
        .map(|landing| landing.touchdown_center_offset_m.abs());
    let (reference_gap_mean_m, reference_gap_max_m) = reference_gap_metrics(scenario, &run.samples)
        .map(|metrics| (Some(metrics.gap_mean_m), Some(metrics.gap_max_m)))
        .unwrap_or((None, None));
    let (low_altitude_dwell_s, low_altitude_unsafe_recovery_s) =
        low_altitude_recovery_metrics(scenario, &run.samples)
            .map(|metrics| {
                (
                    Some(metrics.low_altitude_dwell_s),
                    Some(metrics.low_altitude_unsafe_recovery_s),
                )
            })
            .unwrap_or((None, None));
    let transfer = transfer_review_metrics(&artifacts.controller_updates, &run.samples);
    let mut waypoint_history = waypoint_review_history(&artifacts.controller_updates);
    let waypoint_goal = waypoint_handoff_goal_review_metrics(
        scenario,
        &run.samples,
        &run.manifest,
        &artifacts.controller_updates,
    );
    if matches!(
        scenario.mission.goal,
        EvaluationGoal::WaypointHandoff { .. }
    ) {
        if let Some(terminal) = waypoint_goal.as_ref().filter(|metrics| {
            matches!(
                metrics.capture_status.as_deref(),
                Some("captured" | "missed")
            )
        }) {
            waypoint_history = vec![terminal.clone()];
        }
    } else if matches!(
        scenario.mission.goal,
        EvaluationGoal::WaypointSequence { .. }
    ) && matches!(
        run.manifest.end_reason,
        EndReason::CheckpointSatisfied | EndReason::CheckpointFailed
    ) {
        let terminal_index = run
            .manifest
            .summary
            .waypoint_sequence
            .as_ref()
            .and_then(|summary| {
                summary.first_failed_index.or_else(|| {
                    (summary.passed_handoffs == summary.total_handoffs
                        && summary.total_handoffs > 0)
                        .then_some(summary.total_handoffs - 1)
                })
            });
        if let Some(terminal_index) = terminal_index
            && !waypoint_history
                .iter()
                .any(|metrics| metrics.active_index == i64::try_from(terminal_index).ok())
            && let Some(terminal) = terminal_waypoint_review_metrics(
                scenario,
                &run.samples,
                &run.manifest,
                terminal_index,
                &artifacts.controller_updates,
            )
        {
            waypoint_history.push(terminal);
        }
    }
    waypoint_history.sort_by_key(|metrics| metrics.active_index);
    waypoint_history.dedup_by_key(|metrics| metrics.active_index);
    enrich_waypoint_candidate_history(
        scenario,
        &artifacts.controller_updates,
        &mut waypoint_history,
    );
    let waypoint = waypoint_history
        .first()
        .cloned()
        .or(waypoint_goal)
        .unwrap_or_else(|| waypoint_review_metrics(&artifacts.controller_updates));
    let waypoint_contract = waypoint_contract_review_metrics(scenario, &waypoint);
    let waypoint_handoffs = waypoint_history
        .iter()
        .filter_map(|metrics| batch_waypoint_handoff_metrics(scenario, metrics))
        .collect::<Vec<_>>();
    let waypoint_route = waypoint_route_review_metrics(scenario, &run.manifest, &waypoint_handoffs);
    let transfer_shape =
        transfer_shape_metrics(scenario, &run.samples, &artifacts.controller_updates);

    BatchRunReviewMetrics {
        fuel_used_pct_of_max,
        landing_offset_abs_m,
        low_altitude_dwell_s,
        low_altitude_unsafe_recovery_s,
        reference_gap_mean_m,
        reference_gap_max_m,
        transfer_shape_curve_rmse_m: transfer_shape.map(|metrics| metrics.curve_rmse_m),
        transfer_shape_apex_error_m: transfer_shape.map(|metrics| metrics.apex_error_m),
        transfer_shape_projected_dx_abs_mean_m: transfer_shape
            .and_then(|metrics| metrics.projected_dx_abs_mean_m),
        transfer_shape_projected_dx_abs_max_m: transfer_shape
            .and_then(|metrics| metrics.projected_dx_abs_max_m),
        transfer_shape_shortfall_ratio: transfer_shape.and_then(|metrics| metrics.shortfall_ratio),
        transfer_terminal_entry_kind: transfer.terminal_entry_kind,
        transfer_terminal_handoff_time_s: transfer.terminal_handoff_time_s,
        transfer_terminal_handoff_dx_m: transfer.terminal_handoff_dx_m,
        transfer_terminal_handoff_height_m: transfer.terminal_handoff_height_m,
        transfer_terminal_handoff_speed_mps: transfer.terminal_handoff_speed_mps,
        transfer_terminal_handoff_gate_mode: transfer.terminal_handoff_gate_mode,
        transfer_terminal_handoff_projected_dx_m: transfer.terminal_handoff_projected_dx_m,
        transfer_terminal_handoff_impact_angle_deg: transfer.terminal_handoff_impact_angle_deg,
        transfer_terminal_handoff_boost_quality: transfer.terminal_handoff_boost_quality,
        transfer_terminal_handoff_latest_safe_margin_s: transfer
            .terminal_handoff_latest_safe_margin_s,
        transfer_terminal_handoff_required_accel_ratio: transfer
            .terminal_handoff_required_accel_ratio,
        transfer_terminal_post_handoff_apex_gain_m: transfer.terminal_post_handoff_apex_gain_m,
        transfer_terminal_post_handoff_time_to_apex_s: transfer
            .terminal_post_handoff_time_to_apex_s,
        transfer_terminal_post_handoff_apex_dx_abs_m: transfer.terminal_post_handoff_apex_dx_abs_m,
        transfer_final_phase: transfer.final_phase,
        transfer_boost_projected_dx_m: transfer.boost_projected_dx_m,
        transfer_boost_impact_angle_deg: transfer.boost_impact_angle_deg,
        transfer_boost_apex_over_target_m: transfer.boost_apex_over_target_m,
        transfer_boost_quality: transfer.boost_quality,
        transfer_boost_selected_score: transfer.boost_selected_score,
        transfer_boost_settled_quality: transfer.boost_settled_quality,
        transfer_boost_settled_projected_dx_m: transfer.boost_settled_projected_dx_m,
        transfer_boost_cutoff_time_s: transfer.boost_cutoff_time_s,
        transfer_boost_cutoff_projected_dx_m: transfer.boost_cutoff_projected_dx_m,
        transfer_boost_cutoff_impact_angle_deg: transfer.boost_cutoff_impact_angle_deg,
        transfer_boost_cutoff_apex_over_target_m: transfer.boost_cutoff_apex_over_target_m,
        transfer_boost_cutoff_quality: transfer.boost_cutoff_quality,
        transfer_boost_burn_duration_s: transfer.boost_burn_duration_s,
        transfer_boost_burn_fuel_used_kg: transfer.boost_burn_fuel_used_kg,
        transfer_boost_burn_avg_throttle: transfer.boost_burn_avg_throttle,
        transfer_terminal_gate_mode: transfer.terminal_gate_mode,
        transfer_terminal_gate_latest_safe_margin_s: transfer.terminal_gate_latest_safe_margin_s,
        transfer_terminal_gate_required_accel_ratio: transfer.terminal_gate_required_accel_ratio,
        transfer_terminal_gate_deferred: transfer.terminal_gate_deferred,
        transfer_corridor_mode: transfer.corridor_mode,
        transfer_corridor_min_margin_m: transfer.corridor_min_margin_m,
        waypoint_capture_status: waypoint.capture_status,
        waypoint_contract_status: waypoint_contract.status,
        waypoint_contract_reasons: waypoint_contract.reasons,
        waypoint_active_index: waypoint.active_index,
        waypoint_capture_time_s: waypoint.capture_time_s,
        waypoint_window_entry: waypoint.window_entry.clone(),
        waypoint_handoff_resolution_reason: waypoint.resolution_reason.clone(),
        waypoint_handoff_window_duration_s: waypoint.window_duration_s,
        waypoint_closest_distance_m: waypoint.closest_distance_m,
        waypoint_distance_m: waypoint.distance_m,
        waypoint_cross_track_m: waypoint.cross_track_m,
        waypoint_plane_progress_m: waypoint.plane_progress_m,
        waypoint_outbound_heading_error_rad: waypoint.outbound_heading_error_rad,
        waypoint_outbound_progress_mps: waypoint.outbound_progress_mps,
        waypoint_outbound_cross_speed_mps: waypoint.outbound_cross_speed_mps,
        waypoint_speed_mps: waypoint.speed_mps,
        waypoint_vertical_speed_mps: waypoint.vertical_speed_mps,
        waypoint_remaining_to_plane_m: waypoint.remaining_to_plane_m,
        waypoint_time_to_plane_s: waypoint.time_to_plane_s,
        waypoint_required_turn_distance_m: waypoint.required_turn_distance_m,
        waypoint_shaping_start_distance_m: waypoint.shaping_start_distance_m,
        waypoint_turn_margin_m: waypoint.turn_margin_m,
        waypoint_handoffs,
        waypoint_route_status: waypoint_route.status,
        waypoint_route_passed: waypoint_route.passed,
        waypoint_route_total: waypoint_route.total,
        waypoint_route_first_failure_index: waypoint_route.first_failure_index,
    }
}

#[derive(Clone, Debug, Default)]
struct WaypointReviewMetrics {
    capture_status: Option<String>,
    active_index: Option<i64>,
    capture_time_s: Option<f64>,
    window_entry: Option<BatchWaypointWindowEntryReviewMetrics>,
    resolution_reason: Option<String>,
    window_duration_s: Option<f64>,
    closest_distance_m: Option<f64>,
    distance_m: Option<f64>,
    cross_track_m: Option<f64>,
    plane_progress_m: Option<f64>,
    outbound_heading_error_rad: Option<f64>,
    outbound_progress_mps: Option<f64>,
    outbound_cross_speed_mps: Option<f64>,
    speed_mps: Option<f64>,
    vertical_speed_mps: Option<f64>,
    remaining_to_plane_m: Option<f64>,
    time_to_plane_s: Option<f64>,
    required_turn_distance_m: Option<f64>,
    shaping_start_distance_m: Option<f64>,
    turn_margin_m: Option<f64>,
    center_x_m: Option<f64>,
    center_y_m: Option<f64>,
    nominal_handoff_target_x_m: Option<f64>,
    nominal_handoff_target_y_m: Option<f64>,
    handoff_target_x_m: Option<f64>,
    handoff_target_y_m: Option<f64>,
    handoff_target_mode: Option<String>,
    remaining_to_handoff_m: Option<f64>,
    time_to_handoff_s: Option<f64>,
    target_vx_mps: Option<f64>,
    target_vy_mps: Option<f64>,
    target_deadline_remaining_s: Option<f64>,
    target_velocity_error_mps: Option<f64>,
    guidance_feasible: Option<bool>,
    predicted_handoff_time_to_go_s: Option<f64>,
    predicted_handoff_deadline_lead_s: Option<f64>,
    predicted_handoff_contract_status: Option<String>,
    predicted_handoff_contract_reasons: Vec<String>,
    predicted_handoff_distance_m: Option<f64>,
    predicted_handoff_cross_track_m: Option<f64>,
    predicted_handoff_plane_progress_m: Option<f64>,
    predicted_handoff_outbound_heading_error_rad: Option<f64>,
    predicted_handoff_outbound_progress_mps: Option<f64>,
    predicted_handoff_outbound_cross_speed_mps: Option<f64>,
    predicted_handoff_speed_mps: Option<f64>,
    predicted_handoff_vertical_speed_mps: Option<f64>,
    candidate_contract_pass_ever: Option<bool>,
    candidate_first_pass_time_s: Option<f64>,
    candidate_last_pass_time_s: Option<f64>,
    candidate_pass_lost_before_capture: Option<bool>,
    candidate_best_heading_margin_rad: Option<f64>,
    candidate_best_cross_speed_margin_mps: Option<f64>,
    reachable_candidate_contract_pass_ever: Option<bool>,
    reachable_candidate_first_pass_time_s: Option<f64>,
    reachable_candidate_last_pass_time_s: Option<f64>,
    reachable_candidate_pass_lost_before_capture: Option<bool>,
    reachable_required_accel_ratio_max: Option<f64>,
    reachable_thrust_saturated_time_max_s: Option<f64>,
    reachable_tilt_saturated_time_max_s: Option<f64>,
    continuation_next_waypoint_index: Option<usize>,
    continuation_contract_pass: Option<bool>,
    continuation_contract_reasons: Vec<String>,
    continuation_outbound_heading_error_rad: Option<f64>,
    continuation_required_accel_ratio_max: Option<f64>,
    continuation_passing_candidate_count: Option<usize>,
    transition_next_waypoint_index: Option<usize>,
    transition_position_error_m: Option<f64>,
    transition_velocity_error_mps: Option<f64>,
    transition_attitude_error_rad: Option<f64>,
    transition_mass_error_kg: Option<f64>,
    transition_fuel_error_kg: Option<f64>,
    transition_event_time_error_s: Option<f64>,
    transition_continuation_contract_pass: Option<bool>,
    transition_continuation_contract_reasons: Vec<String>,
    transition_continuation_outbound_heading_error_rad: Option<f64>,
    transition_continuation_required_accel_ratio_max: Option<f64>,
    transition_continuation_passing_candidate_count: Option<usize>,
    joint_next_waypoint_index: Option<usize>,
    joint_evaluated_candidate_count: Option<usize>,
    joint_passing_candidate_count: Option<usize>,
    joint_contract_pass: Option<bool>,
    joint_endpoint_x_m: Option<f64>,
    joint_endpoint_y_m: Option<f64>,
    joint_target_vx_mps: Option<f64>,
    joint_target_vy_mps: Option<f64>,
    joint_time_to_go_s: Option<f64>,
    joint_continuation_outbound_heading_error_rad: Option<f64>,
    joint_required_accel_ratio_max: Option<f64>,
    joint_total_saturated_time_s: Option<f64>,
    joint_continuation_passing_candidate_count: Option<usize>,
    plan_reference_position_error_max_m: Option<f64>,
    plan_reference_cross_error_max_abs_m: Option<f64>,
    plan_reference_velocity_error_max_mps: Option<f64>,
    plan_reference_cross_speed_error_max_abs_mps: Option<f64>,
    guidance_required_accel_ratio_max: Option<f64>,
    guidance_thrust_saturated_time_s: Option<f64>,
    guidance_tilt_saturated_time_s: Option<f64>,
    guidance_first_saturation_lead_s: Option<f64>,
    last_pass_reference_position_error_m: Option<f64>,
    last_pass_reference_velocity_error_mps: Option<f64>,
    last_pass_required_accel_ratio: Option<f64>,
    guidance_plan_revision_max: Option<i64>,
    guidance_plan_reasons: Vec<String>,
    handoff_turn_margin_m: Option<f64>,
    guidance_snapshot_source: Option<String>,
    guidance_snapshot_age_s: Option<f64>,
    guidance_replan_count: Option<i64>,
}

#[derive(Clone, Debug, Default)]
struct WaypointContractReviewMetrics {
    status: Option<String>,
    reasons: Vec<String>,
}

fn waypoint_review_metrics(
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> WaypointReviewMetrics {
    let update = controller_updates
        .iter()
        .find(|update| {
            matches!(
                telemetry_text(&update.frame.metrics, metric::WAYPOINT_CAPTURE_STATUS),
                Some("captured" | "missed")
            )
        })
        .or_else(|| {
            controller_updates.iter().rev().find(|update| {
                telemetry_bool(&update.frame.metrics, metric::WAYPOINT_GUIDANCE_ENABLED)
                    == Some(true)
            })
        });
    let Some(update) = update else {
        return WaypointReviewMetrics::default();
    };
    waypoint_review_metrics_from_update(update)
}

fn waypoint_review_history(
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> Vec<WaypointReviewMetrics> {
    let mut handoffs = controller_updates
        .iter()
        .filter(|update| {
            update
                .frame
                .markers
                .iter()
                .any(|candidate| candidate.id == marker::WAYPOINT_HANDOFF)
        })
        .map(waypoint_review_metrics_from_update)
        .collect::<Vec<_>>();
    if handoffs.is_empty() {
        handoffs = controller_updates
            .iter()
            .filter(|update| {
                matches!(
                    telemetry_text(&update.frame.metrics, metric::WAYPOINT_CAPTURE_STATUS),
                    Some("captured" | "missed")
                )
            })
            .map(waypoint_review_metrics_from_update)
            .collect();
    }
    handoffs.sort_by_key(|handoff| handoff.active_index);
    handoffs.dedup_by_key(|handoff| handoff.active_index);
    handoffs
}

fn enrich_waypoint_candidate_history(
    scenario: &ScenarioSpec,
    controller_updates: &[pd_control::ControllerUpdateRecord],
    handoffs: &mut [WaypointReviewMetrics],
) {
    let Some(route) = scenario.mission.transfer_route.as_ref() else {
        return;
    };

    for handoff in handoffs {
        let Some(index) = handoff
            .active_index
            .and_then(|index| usize::try_from(index).ok())
        else {
            continue;
        };
        let Some(waypoint) = route.waypoints.get(index) else {
            continue;
        };
        let expected_index = i64::try_from(index).ok();
        let updates = controller_updates
            .iter()
            .filter(|update| {
                telemetry_bool(&update.frame.metrics, metric::WAYPOINT_GUIDANCE_ENABLED)
                    == Some(true)
                    && telemetry_integer(
                        &update.frame.metrics,
                        metric::WAYPOINT_GUIDANCE_PLAN_INDEX,
                    )
                    .or_else(|| {
                        telemetry_integer(&update.frame.metrics, metric::WAYPOINT_ACTIVE_INDEX)
                    }) == expected_index
            })
            .collect::<Vec<_>>();

        let mut observed = false;
        let mut pass_ever = false;
        let mut first_pass_time_s = None;
        let mut last_pass_time_s = None;
        let mut last_observed_pass = false;
        let mut best_heading_margin_rad: Option<f64> = None;
        let mut best_cross_speed_margin_mps: Option<f64> = None;
        let mut reference_position_error_max_m: Option<f64> = None;
        let mut reference_cross_error_max_abs_m: Option<f64> = None;
        let mut reference_velocity_error_max_mps: Option<f64> = None;
        let mut reference_cross_speed_error_max_abs_mps: Option<f64> = None;
        let mut required_accel_ratio_max: Option<f64> = None;
        let mut thrust_saturated_time_s = 0.0;
        let mut tilt_saturated_time_s = 0.0;
        let mut first_saturation_time_s: Option<f64> = None;
        let mut trackability_observed = false;
        let mut plan_revision_max: Option<i64> = None;
        let mut plan_reasons = BTreeSet::new();
        let mut last_pass_reference_position_error_m = None;
        let mut last_pass_reference_velocity_error_mps = None;
        let mut last_pass_required_accel_ratio = None;
        let mut reachable_observed = false;
        let mut reachable_pass_ever = false;
        let mut reachable_first_pass_time_s = None;
        let mut reachable_last_pass_time_s = None;
        let mut reachable_last_observed_pass = false;
        let mut reachable_required_accel_ratio_max: Option<f64> = None;
        let mut reachable_thrust_saturated_time_max_s: Option<f64> = None;
        let mut reachable_tilt_saturated_time_max_s: Option<f64> = None;
        let mut continuation_observed = false;
        let mut continuation_next_waypoint_index = None;
        let mut continuation_contract_pass = None;
        let mut continuation_contract_reasons = Vec::new();
        let mut continuation_outbound_heading_error_rad = None;
        let mut continuation_required_accel_ratio_max = None;
        let mut continuation_passing_candidate_count = None;

        for (update_index, update) in updates.iter().enumerate() {
            let metrics = &update.frame.metrics;
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_GUIDANCE_REFERENCE_POSITION_ERROR_M,
            ) {
                trackability_observed = true;
                update_optional_max(&mut reference_position_error_max_m, value);
            }
            if let Some(value) =
                telemetry_float(metrics, metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_ERROR_M)
            {
                update_optional_max(&mut reference_cross_error_max_abs_m, value.abs());
            }
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_GUIDANCE_REFERENCE_VELOCITY_ERROR_MPS,
            ) {
                update_optional_max(&mut reference_velocity_error_max_mps, value);
            }
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_SPEED_ERROR_MPS,
            ) {
                update_optional_max(&mut reference_cross_speed_error_max_abs_mps, value.abs());
            }
            if let Some(value) =
                telemetry_float(metrics, metric::WAYPOINT_GUIDANCE_REQUIRED_ACCEL_RATIO)
            {
                update_optional_max(&mut required_accel_ratio_max, value);
            }
            if let Some(revision) =
                telemetry_integer(metrics, metric::WAYPOINT_GUIDANCE_PLAN_REVISION)
            {
                plan_revision_max =
                    Some(plan_revision_max.map_or(revision, |max| max.max(revision)));
            }
            if let Some(reason) = telemetry_text(metrics, metric::WAYPOINT_GUIDANCE_PLAN_REASON) {
                plan_reasons.insert(reason.to_owned());
            }
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_REACHABLE_HANDOFF_REQUIRED_ACCEL_RATIO_MAX,
            ) {
                update_optional_max(&mut reachable_required_accel_ratio_max, value);
            }
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_REACHABLE_HANDOFF_THRUST_SATURATED_TIME_S,
            ) {
                update_optional_max(&mut reachable_thrust_saturated_time_max_s, value);
            }
            if let Some(value) = telemetry_float(
                metrics,
                metric::WAYPOINT_REACHABLE_HANDOFF_TILT_SATURATED_TIME_S,
            ) {
                update_optional_max(&mut reachable_tilt_saturated_time_max_s, value);
            }
            if let Some(passed) =
                telemetry_bool(metrics, metric::WAYPOINT_CONTINUATION_CONTRACT_PASS)
            {
                continuation_observed = true;
                continuation_contract_pass = Some(passed);
                continuation_next_waypoint_index =
                    telemetry_integer(metrics, metric::WAYPOINT_CONTINUATION_NEXT_INDEX)
                        .and_then(|index| usize::try_from(index).ok());
                continuation_contract_reasons =
                    telemetry_text(metrics, metric::WAYPOINT_CONTINUATION_CONTRACT_REASONS)
                        .map(|reasons| {
                            reasons
                                .split(',')
                                .filter(|reason| !reason.is_empty())
                                .map(ToOwned::to_owned)
                                .collect()
                        })
                        .unwrap_or_default();
                continuation_outbound_heading_error_rad = telemetry_float(
                    metrics,
                    metric::WAYPOINT_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD,
                );
                continuation_required_accel_ratio_max = telemetry_float(
                    metrics,
                    metric::WAYPOINT_CONTINUATION_REQUIRED_ACCEL_RATIO_MAX,
                );
                continuation_passing_candidate_count = telemetry_integer(
                    metrics,
                    metric::WAYPOINT_CONTINUATION_PASSING_CANDIDATE_COUNT,
                )
                .and_then(|count| usize::try_from(count).ok());
            }
            if let Some(passed) =
                telemetry_bool(metrics, metric::WAYPOINT_REACHABLE_HANDOFF_CONTRACT_PASS)
            {
                reachable_observed = true;
                reachable_last_observed_pass = passed;
                if passed {
                    reachable_pass_ever = true;
                    reachable_first_pass_time_s.get_or_insert(update.sim_time_s);
                    reachable_last_pass_time_s = Some(update.sim_time_s);
                }
            }

            let thrust_saturated =
                telemetry_bool(metrics, metric::WAYPOINT_GUIDANCE_THRUST_SATURATED) == Some(true);
            let tilt_saturated =
                telemetry_bool(metrics, metric::WAYPOINT_GUIDANCE_TILT_SATURATED) == Some(true);
            if thrust_saturated || tilt_saturated {
                first_saturation_time_s.get_or_insert(update.sim_time_s);
            }
            let interval_s = updates
                .get(update_index + 1)
                .map(|next| (next.sim_time_s - update.sim_time_s).max(0.0))
                .unwrap_or(0.0);
            if thrust_saturated {
                thrust_saturated_time_s += interval_s;
            }
            if tilt_saturated {
                tilt_saturated_time_s += interval_s;
            }

            let Some(passed) = telemetry_bool(
                &update.frame.metrics,
                metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS,
            ) else {
                continue;
            };
            observed = true;
            last_observed_pass = passed;
            if passed {
                pass_ever = true;
                first_pass_time_s.get_or_insert(update.sim_time_s);
                last_pass_time_s = Some(update.sim_time_s);
                last_pass_reference_position_error_m = telemetry_float(
                    metrics,
                    metric::WAYPOINT_GUIDANCE_REFERENCE_POSITION_ERROR_M,
                );
                last_pass_reference_velocity_error_mps = telemetry_float(
                    metrics,
                    metric::WAYPOINT_GUIDANCE_REFERENCE_VELOCITY_ERROR_MPS,
                );
                last_pass_required_accel_ratio =
                    telemetry_float(metrics, metric::WAYPOINT_GUIDANCE_REQUIRED_ACCEL_RATIO);
            }

            if let Some(error_rad) = telemetry_float(
                &update.frame.metrics,
                metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_HEADING_ERROR_RAD,
            ) {
                let margin = waypoint.max_outbound_heading_error_rad - error_rad;
                best_heading_margin_rad =
                    Some(best_heading_margin_rad.map_or(margin, |best| best.max(margin)));
            }
            if let Some(max_cross_speed_mps) = waypoint.max_outbound_cross_speed_mps
                && let Some(cross_speed_mps) = telemetry_float(
                    &update.frame.metrics,
                    metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_CROSS_SPEED_MPS,
                )
            {
                let margin = max_cross_speed_mps - cross_speed_mps;
                best_cross_speed_margin_mps =
                    Some(best_cross_speed_margin_mps.map_or(margin, |best| best.max(margin)));
            }
        }

        if observed {
            handoff.candidate_contract_pass_ever = Some(pass_ever);
            handoff.candidate_first_pass_time_s = first_pass_time_s;
            handoff.candidate_last_pass_time_s = last_pass_time_s;
            handoff.candidate_pass_lost_before_capture = Some(pass_ever && !last_observed_pass);
            handoff.candidate_best_heading_margin_rad = best_heading_margin_rad;
            handoff.candidate_best_cross_speed_margin_mps = best_cross_speed_margin_mps;
        }
        if reachable_observed {
            handoff.reachable_candidate_contract_pass_ever = Some(reachable_pass_ever);
            handoff.reachable_candidate_first_pass_time_s = reachable_first_pass_time_s;
            handoff.reachable_candidate_last_pass_time_s = reachable_last_pass_time_s;
            handoff.reachable_candidate_pass_lost_before_capture =
                Some(reachable_pass_ever && !reachable_last_observed_pass);
            handoff.reachable_required_accel_ratio_max = reachable_required_accel_ratio_max;
            handoff.reachable_thrust_saturated_time_max_s = reachable_thrust_saturated_time_max_s;
            handoff.reachable_tilt_saturated_time_max_s = reachable_tilt_saturated_time_max_s;
        }
        if continuation_observed {
            handoff.continuation_next_waypoint_index = continuation_next_waypoint_index;
            handoff.continuation_contract_pass = continuation_contract_pass;
            handoff.continuation_contract_reasons = continuation_contract_reasons;
            handoff.continuation_outbound_heading_error_rad =
                continuation_outbound_heading_error_rad;
            handoff.continuation_required_accel_ratio_max = continuation_required_accel_ratio_max;
            handoff.continuation_passing_candidate_count = continuation_passing_candidate_count;
        }
        if trackability_observed {
            handoff.plan_reference_position_error_max_m = reference_position_error_max_m;
            handoff.plan_reference_cross_error_max_abs_m = reference_cross_error_max_abs_m;
            handoff.plan_reference_velocity_error_max_mps = reference_velocity_error_max_mps;
            handoff.plan_reference_cross_speed_error_max_abs_mps =
                reference_cross_speed_error_max_abs_mps;
            handoff.guidance_required_accel_ratio_max = required_accel_ratio_max;
            handoff.guidance_thrust_saturated_time_s = Some(thrust_saturated_time_s);
            handoff.guidance_tilt_saturated_time_s = Some(tilt_saturated_time_s);
            handoff.guidance_first_saturation_lead_s = first_saturation_time_s
                .zip(handoff.capture_time_s)
                .map(|(first_saturation, capture)| (capture - first_saturation).max(0.0));
            handoff.last_pass_reference_position_error_m = last_pass_reference_position_error_m;
            handoff.last_pass_reference_velocity_error_mps = last_pass_reference_velocity_error_mps;
            handoff.last_pass_required_accel_ratio = last_pass_required_accel_ratio;
            handoff.guidance_plan_revision_max = plan_revision_max;
            handoff.guidance_plan_reasons = plan_reasons.into_iter().collect();
        }
    }
}

fn update_optional_max(current: &mut Option<f64>, value: f64) {
    *current = Some(current.map_or(value, |maximum| maximum.max(value)));
}

fn waypoint_review_metrics_from_update(
    update: &pd_control::ControllerUpdateRecord,
) -> WaypointReviewMetrics {
    let capture_time_s = telemetry_float(&update.frame.metrics, metric::WAYPOINT_CAPTURE_TIME_S)
        .filter(|value| *value >= 0.0);
    let handoff_marker = update
        .frame
        .markers
        .iter()
        .find(|candidate| candidate.id == marker::WAYPOINT_HANDOFF);
    let preferred_metrics = handoff_marker
        .map(|handoff| &handoff.metadata)
        .unwrap_or(&update.frame.metrics);
    let preferred_float = |key| {
        telemetry_float(preferred_metrics, key)
            .or_else(|| telemetry_float(&update.frame.metrics, key))
    };
    let preferred_text = |key| {
        telemetry_text(preferred_metrics, key)
            .or_else(|| telemetry_text(&update.frame.metrics, key))
    };
    let preferred_bool = |key| {
        telemetry_bool(preferred_metrics, key)
            .or_else(|| telemetry_bool(&update.frame.metrics, key))
    };
    let preferred_integer = |key| {
        telemetry_integer(preferred_metrics, key)
            .or_else(|| telemetry_integer(&update.frame.metrics, key))
    };
    let preferred_reasons = |key| {
        preferred_text(key)
            .map(|reasons| {
                reasons
                    .split(',')
                    .filter(|reason| !reason.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let window_entry = preferred_float(metric::WAYPOINT_WINDOW_ENTRY_TIME_S).map(|time_s| {
        BatchWaypointWindowEntryReviewMetrics {
            time_s: Some(time_s),
            position_x_m: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_POSITION_X_M),
            position_y_m: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_POSITION_Y_M),
            velocity_x_mps: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_VELOCITY_X_MPS),
            velocity_y_mps: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_VELOCITY_Y_MPS),
            distance_m: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_DISTANCE_M),
            cross_track_m: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_CROSS_TRACK_M),
            plane_progress_m: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_PLANE_PROGRESS_M),
            handoff_heading_error_rad: preferred_float(
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_HEADING_ERROR_RAD,
            ),
            handoff_progress_mps: preferred_float(
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_PROGRESS_MPS,
            ),
            handoff_cross_speed_mps: preferred_float(
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_CROSS_SPEED_MPS,
            ),
            speed_mps: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_SPEED_MPS),
            vertical_speed_mps: preferred_float(metric::WAYPOINT_WINDOW_ENTRY_VERTICAL_SPEED_MPS),
            contract_pass: preferred_bool(metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_PASS),
            contract_reasons: preferred_reasons(metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_REASONS),
        }
    });
    let predicted_handoff_contract_reasons =
        preferred_text(metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_REASONS)
            .map(|reasons| {
                reasons
                    .split(',')
                    .filter(|reason| !reason.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
    let continuation_contract_reasons =
        preferred_text(metric::WAYPOINT_CONTINUATION_CONTRACT_REASONS)
            .map(|reasons| {
                reasons
                    .split(',')
                    .filter(|reason| !reason.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
    let transition_continuation_contract_reasons =
        preferred_text(metric::WAYPOINT_TRANSITION_CONTINUATION_CONTRACT_REASONS)
            .map(|reasons| {
                reasons
                    .split(',')
                    .filter(|reason| !reason.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
    WaypointReviewMetrics {
        capture_status: telemetry_text(&update.frame.metrics, metric::WAYPOINT_CAPTURE_STATUS)
            .map(ToOwned::to_owned),
        active_index: telemetry_integer(&update.frame.metrics, metric::WAYPOINT_ACTIVE_INDEX),
        capture_time_s,
        window_entry,
        resolution_reason: preferred_text(metric::WAYPOINT_HANDOFF_RESOLUTION_REASON)
            .map(ToOwned::to_owned),
        window_duration_s: preferred_float(metric::WAYPOINT_HANDOFF_WINDOW_DURATION_S)
            .filter(|duration| *duration >= 0.0),
        closest_distance_m: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_CLOSEST_DISTANCE_M,
        )
        .filter(|value| *value >= 0.0),
        distance_m: telemetry_float(&update.frame.metrics, metric::WAYPOINT_DISTANCE_M)
            .filter(|value| *value >= 0.0),
        cross_track_m: telemetry_float(&update.frame.metrics, metric::WAYPOINT_CROSS_TRACK_M)
            .filter(|value| *value >= 0.0),
        plane_progress_m: telemetry_float(&update.frame.metrics, metric::WAYPOINT_PLANE_PROGRESS_M),
        outbound_heading_error_rad: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_OUTBOUND_HEADING_ERROR_RAD,
        )
        .filter(|value| *value >= 0.0),
        outbound_progress_mps: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_OUTBOUND_PROGRESS_MPS,
        ),
        outbound_cross_speed_mps: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_OUTBOUND_CROSS_SPEED_MPS,
        )
        .filter(|value| *value >= 0.0),
        speed_mps: telemetry_float(&update.frame.metrics, metric::WAYPOINT_SPEED_MPS)
            .filter(|value| *value >= 0.0),
        vertical_speed_mps: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_VERTICAL_SPEED_MPS,
        ),
        remaining_to_plane_m: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_REMAINING_TO_PLANE_M,
        )
        .filter(|value| *value >= 0.0),
        time_to_plane_s: telemetry_float(&update.frame.metrics, metric::WAYPOINT_TIME_TO_PLANE_S)
            .filter(|value| *value >= 0.0 && value.is_finite()),
        required_turn_distance_m: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_REQUIRED_TURN_DISTANCE_M,
        )
        .filter(|value| *value >= 0.0),
        shaping_start_distance_m: telemetry_float(
            &update.frame.metrics,
            metric::WAYPOINT_SHAPING_START_DISTANCE_M,
        )
        .filter(|value| *value >= 0.0),
        turn_margin_m: telemetry_float(&update.frame.metrics, metric::WAYPOINT_TURN_MARGIN_M),
        center_x_m: preferred_float(metric::WAYPOINT_CENTER_X_M),
        center_y_m: preferred_float(metric::WAYPOINT_CENTER_Y_M),
        nominal_handoff_target_x_m: preferred_float(metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_X_M),
        nominal_handoff_target_y_m: preferred_float(metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_Y_M),
        handoff_target_x_m: preferred_float(metric::WAYPOINT_HANDOFF_TARGET_X_M),
        handoff_target_y_m: preferred_float(metric::WAYPOINT_HANDOFF_TARGET_Y_M),
        handoff_target_mode: preferred_text(metric::WAYPOINT_HANDOFF_TARGET_MODE)
            .map(ToOwned::to_owned),
        remaining_to_handoff_m: preferred_float(metric::WAYPOINT_REMAINING_TO_HANDOFF_M)
            .filter(|value| *value >= 0.0),
        time_to_handoff_s: preferred_float(metric::WAYPOINT_TIME_TO_HANDOFF_S)
            .filter(|value| *value >= 0.0 && value.is_finite()),
        target_vx_mps: preferred_float(metric::WAYPOINT_TARGET_VX_MPS),
        target_vy_mps: preferred_float(metric::WAYPOINT_TARGET_VY_MPS),
        target_deadline_remaining_s: preferred_float(metric::WAYPOINT_TARGET_DEADLINE_REMAINING_S),
        target_velocity_error_mps: preferred_float(metric::WAYPOINT_TARGET_VELOCITY_ERROR_MPS)
            .filter(|value| *value >= 0.0),
        guidance_feasible: preferred_bool(metric::WAYPOINT_GUIDANCE_FEASIBLE),
        predicted_handoff_time_to_go_s: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_TIME_TO_GO_S,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_deadline_lead_s: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_DEADLINE_LEAD_S,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_contract_status: preferred_bool(
            metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS,
        )
        .map(|passed| if passed { "pass" } else { "fail" }.to_owned()),
        predicted_handoff_contract_reasons,
        predicted_handoff_distance_m: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_DISTANCE_M,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_cross_track_m: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_CROSS_TRACK_M,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_plane_progress_m: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_PLANE_PROGRESS_M,
        ),
        predicted_handoff_outbound_heading_error_rad: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_HEADING_ERROR_RAD,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_outbound_progress_mps: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_PROGRESS_MPS,
        ),
        predicted_handoff_outbound_cross_speed_mps: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_CROSS_SPEED_MPS,
        )
        .filter(|value| *value >= 0.0),
        predicted_handoff_speed_mps: preferred_float(metric::WAYPOINT_PREDICTED_HANDOFF_SPEED_MPS)
            .filter(|value| *value >= 0.0),
        predicted_handoff_vertical_speed_mps: preferred_float(
            metric::WAYPOINT_PREDICTED_HANDOFF_VERTICAL_SPEED_MPS,
        ),
        candidate_contract_pass_ever: None,
        candidate_first_pass_time_s: None,
        candidate_last_pass_time_s: None,
        candidate_pass_lost_before_capture: None,
        candidate_best_heading_margin_rad: None,
        candidate_best_cross_speed_margin_mps: None,
        reachable_candidate_contract_pass_ever: None,
        reachable_candidate_first_pass_time_s: None,
        reachable_candidate_last_pass_time_s: None,
        reachable_candidate_pass_lost_before_capture: None,
        reachable_required_accel_ratio_max: None,
        reachable_thrust_saturated_time_max_s: None,
        reachable_tilt_saturated_time_max_s: None,
        continuation_next_waypoint_index: preferred_integer(
            metric::WAYPOINT_CONTINUATION_NEXT_INDEX,
        )
        .and_then(|index| usize::try_from(index).ok()),
        continuation_contract_pass: preferred_bool(metric::WAYPOINT_CONTINUATION_CONTRACT_PASS),
        continuation_contract_reasons,
        continuation_outbound_heading_error_rad: preferred_float(
            metric::WAYPOINT_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD,
        ),
        continuation_required_accel_ratio_max: preferred_float(
            metric::WAYPOINT_CONTINUATION_REQUIRED_ACCEL_RATIO_MAX,
        ),
        continuation_passing_candidate_count: preferred_integer(
            metric::WAYPOINT_CONTINUATION_PASSING_CANDIDATE_COUNT,
        )
        .and_then(|count| usize::try_from(count).ok()),
        transition_next_waypoint_index: preferred_integer(metric::WAYPOINT_TRANSITION_NEXT_INDEX)
            .and_then(|index| usize::try_from(index).ok()),
        transition_position_error_m: preferred_float(metric::WAYPOINT_TRANSITION_POSITION_ERROR_M),
        transition_velocity_error_mps: preferred_float(
            metric::WAYPOINT_TRANSITION_VELOCITY_ERROR_MPS,
        ),
        transition_attitude_error_rad: preferred_float(
            metric::WAYPOINT_TRANSITION_ATTITUDE_ERROR_RAD,
        ),
        transition_mass_error_kg: preferred_float(metric::WAYPOINT_TRANSITION_MASS_ERROR_KG),
        transition_fuel_error_kg: preferred_float(metric::WAYPOINT_TRANSITION_FUEL_ERROR_KG),
        transition_event_time_error_s: preferred_float(
            metric::WAYPOINT_TRANSITION_EVENT_TIME_ERROR_S,
        ),
        transition_continuation_contract_pass: preferred_bool(
            metric::WAYPOINT_TRANSITION_CONTINUATION_CONTRACT_PASS,
        ),
        transition_continuation_contract_reasons,
        transition_continuation_outbound_heading_error_rad: preferred_float(
            metric::WAYPOINT_TRANSITION_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD,
        ),
        transition_continuation_required_accel_ratio_max: preferred_float(
            metric::WAYPOINT_TRANSITION_CONTINUATION_REQUIRED_ACCEL_RATIO_MAX,
        ),
        transition_continuation_passing_candidate_count: preferred_integer(
            metric::WAYPOINT_TRANSITION_CONTINUATION_PASSING_CANDIDATE_COUNT,
        )
        .and_then(|count| usize::try_from(count).ok()),
        joint_next_waypoint_index: preferred_integer(metric::WAYPOINT_JOINT_NEXT_INDEX)
            .and_then(|index| usize::try_from(index).ok()),
        joint_evaluated_candidate_count: preferred_integer(
            metric::WAYPOINT_JOINT_EVALUATED_CANDIDATE_COUNT,
        )
        .and_then(|count| usize::try_from(count).ok()),
        joint_passing_candidate_count: preferred_integer(
            metric::WAYPOINT_JOINT_PASSING_CANDIDATE_COUNT,
        )
        .and_then(|count| usize::try_from(count).ok()),
        joint_contract_pass: preferred_bool(metric::WAYPOINT_JOINT_CONTRACT_PASS),
        joint_endpoint_x_m: preferred_float(metric::WAYPOINT_JOINT_ENDPOINT_X_M),
        joint_endpoint_y_m: preferred_float(metric::WAYPOINT_JOINT_ENDPOINT_Y_M),
        joint_target_vx_mps: preferred_float(metric::WAYPOINT_JOINT_TARGET_VX_MPS),
        joint_target_vy_mps: preferred_float(metric::WAYPOINT_JOINT_TARGET_VY_MPS),
        joint_time_to_go_s: preferred_float(metric::WAYPOINT_JOINT_TIME_TO_GO_S),
        joint_continuation_outbound_heading_error_rad: preferred_float(
            metric::WAYPOINT_JOINT_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD,
        ),
        joint_required_accel_ratio_max: preferred_float(
            metric::WAYPOINT_JOINT_REQUIRED_ACCEL_RATIO_MAX,
        ),
        joint_total_saturated_time_s: preferred_float(
            metric::WAYPOINT_JOINT_TOTAL_SATURATED_TIME_S,
        ),
        joint_continuation_passing_candidate_count: preferred_integer(
            metric::WAYPOINT_JOINT_CONTINUATION_PASSING_CANDIDATE_COUNT,
        )
        .and_then(|count| usize::try_from(count).ok()),
        plan_reference_position_error_max_m: None,
        plan_reference_cross_error_max_abs_m: None,
        plan_reference_velocity_error_max_mps: None,
        plan_reference_cross_speed_error_max_abs_mps: None,
        guidance_required_accel_ratio_max: None,
        guidance_thrust_saturated_time_s: None,
        guidance_tilt_saturated_time_s: None,
        guidance_first_saturation_lead_s: None,
        last_pass_reference_position_error_m: None,
        last_pass_reference_velocity_error_mps: None,
        last_pass_required_accel_ratio: None,
        guidance_plan_revision_max: None,
        guidance_plan_reasons: Vec::new(),
        handoff_turn_margin_m: preferred_float(metric::WAYPOINT_HANDOFF_TURN_MARGIN_M),
        guidance_snapshot_source: Some(
            if handoff_marker.is_some() {
                "capture_update"
            } else {
                "controller_update"
            }
            .to_owned(),
        ),
        guidance_snapshot_age_s: handoff_marker.is_some().then_some(0.0),
        guidance_replan_count: handoff_marker
            .and_then(|handoff| {
                telemetry_integer(&handoff.metadata, metric::WAYPOINT_GUIDANCE_REPLAN_COUNT)
            })
            .or_else(|| {
                telemetry_integer(
                    &update.frame.metrics,
                    metric::WAYPOINT_GUIDANCE_REPLAN_COUNT,
                )
            }),
    }
}

fn waypoint_handoff_goal_review_metrics(
    scenario: &ScenarioSpec,
    samples: &[SampleRecord],
    manifest: &RunManifest,
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> Option<WaypointReviewMetrics> {
    let EvaluationGoal::WaypointHandoff { waypoint_index, .. } = &scenario.mission.goal else {
        return None;
    };
    terminal_waypoint_review_metrics(
        scenario,
        samples,
        manifest,
        *waypoint_index,
        controller_updates,
    )
}

fn terminal_waypoint_review_metrics(
    scenario: &ScenarioSpec,
    samples: &[SampleRecord],
    manifest: &RunManifest,
    waypoint_index: usize,
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> Option<WaypointReviewMetrics> {
    let route = scenario.mission.transfer_route.as_ref()?;
    let waypoint = route.waypoints.get(waypoint_index)?;
    let last_sample = samples.last()?;
    let stats = waypoint_sample_stats(scenario, &last_sample.observation, waypoint_index)?;
    let terminal_handoff = matches!(
        manifest.end_reason,
        EndReason::CheckpointSatisfied | EndReason::CheckpointFailed
    );
    let capture_status = if terminal_handoff {
        if waypoint_capture_passes_review(waypoint, &stats) {
            "captured"
        } else {
            "missed"
        }
    } else {
        "tracking"
    };
    let closest_distance_m = samples
        .iter()
        .filter_map(|sample| waypoint_sample_stats(scenario, &sample.observation, waypoint_index))
        .map(|stats| stats.distance_m)
        .min_by(|lhs, rhs| lhs.total_cmp(rhs));
    let sampled_window_entry = samples.iter().find_map(|sample| {
        let entry_stats = waypoint_sample_stats(scenario, &sample.observation, waypoint_index)?;
        (entry_stats.distance_m <= waypoint.capture_radius_m).then(|| {
            let assessment = waypoint.assess_handoff(entry_stats);
            BatchWaypointWindowEntryReviewMetrics {
                time_s: Some(sample.sim_time_s),
                position_x_m: Some(sample.observation.position_m.x),
                position_y_m: Some(sample.observation.position_m.y),
                velocity_x_mps: Some(sample.observation.velocity_mps.x),
                velocity_y_mps: Some(sample.observation.velocity_mps.y),
                distance_m: Some(entry_stats.distance_m),
                cross_track_m: Some(entry_stats.cross_track_m),
                plane_progress_m: Some(entry_stats.plane_progress_m),
                handoff_heading_error_rad: Some(entry_stats.outbound_heading_error_rad),
                handoff_progress_mps: Some(entry_stats.outbound_progress_mps),
                handoff_cross_speed_mps: Some(entry_stats.outbound_cross_speed_mps),
                speed_mps: Some(entry_stats.speed_mps),
                vertical_speed_mps: Some(entry_stats.vertical_speed_mps),
                contract_pass: Some(assessment.contract_pass()),
                contract_reasons: assessment
                    .violations
                    .iter()
                    .map(|violation| violation.as_str().to_owned())
                    .collect(),
            }
        })
    });
    let active_index = i64::try_from(waypoint_index).ok();
    let guidance_update = controller_updates.iter().rev().find(|update| {
        telemetry_bool(&update.frame.metrics, metric::WAYPOINT_GUIDANCE_ENABLED) == Some(true)
            && telemetry_integer(&update.frame.metrics, metric::WAYPOINT_ACTIVE_INDEX)
                == active_index
            && matches!(
                telemetry_text(&update.frame.metrics, metric::WAYPOINT_CAPTURE_STATUS),
                Some("tracking" | "capture_window")
            )
    });
    let guidance = guidance_update
        .map(waypoint_review_metrics_from_update)
        .unwrap_or_default();
    let guidance_snapshot_age_s =
        guidance_update.map(|update| (last_sample.sim_time_s - update.sim_time_s).max(0.0));
    let target_deadline_remaining_s = guidance
        .target_deadline_remaining_s
        .map(|remaining_s| remaining_s - guidance_snapshot_age_s.unwrap_or(0.0));
    let target_velocity_error_mps =
        guidance
            .target_vx_mps
            .zip(guidance.target_vy_mps)
            .map(|(vx_mps, vy_mps)| {
                (last_sample.observation.velocity_mps - Vec2::new(vx_mps, vy_mps)).length()
            });
    let predicted_handoff_time_to_go_s = guidance
        .predicted_handoff_time_to_go_s
        .map(|remaining_s| (remaining_s - guidance_snapshot_age_s.unwrap_or(0.0)).max(0.0));
    let remaining_to_handoff_m = if terminal_handoff {
        Some(0.0)
    } else {
        Some(((-stats.plane_progress_m) - waypoint.capture_radius_m).max(0.0))
    };
    let handoff_turn_margin_m = remaining_to_handoff_m
        .zip(guidance.required_turn_distance_m)
        .map(|(remaining_m, required_m)| remaining_m - required_m);
    let window_entry = guidance.window_entry.clone().or(sampled_window_entry);
    let final_assessment = waypoint.assess_handoff(stats);
    let resolution_reason = guidance.resolution_reason.clone().or_else(|| {
        terminal_handoff.then(|| {
            if final_assessment.contract_pass_in_window(window_entry.is_some()) {
                "contract_pass"
            } else {
                "plane_deadline"
            }
            .to_owned()
        })
    });
    let window_duration_s = guidance.window_duration_s.or_else(|| {
        terminal_handoff
            .then_some(last_sample.sim_time_s)
            .zip(window_entry.as_ref().and_then(|entry| entry.time_s))
            .map(|(resolution_time_s, entry_time_s)| (resolution_time_s - entry_time_s).max(0.0))
    });

    Some(WaypointReviewMetrics {
        capture_status: Some(capture_status.to_owned()),
        active_index,
        capture_time_s: terminal_handoff.then_some(last_sample.sim_time_s),
        window_entry,
        resolution_reason,
        window_duration_s,
        closest_distance_m,
        distance_m: Some(stats.distance_m),
        cross_track_m: Some(stats.cross_track_m),
        plane_progress_m: Some(stats.plane_progress_m),
        outbound_heading_error_rad: Some(stats.outbound_heading_error_rad),
        outbound_progress_mps: Some(stats.outbound_progress_mps),
        outbound_cross_speed_mps: Some(stats.outbound_cross_speed_mps),
        speed_mps: Some(stats.speed_mps),
        vertical_speed_mps: Some(stats.vertical_speed_mps),
        remaining_to_plane_m: Some((-stats.plane_progress_m).max(0.0)),
        time_to_plane_s: guidance.time_to_plane_s,
        required_turn_distance_m: guidance.required_turn_distance_m,
        shaping_start_distance_m: guidance.shaping_start_distance_m,
        turn_margin_m: guidance.turn_margin_m,
        center_x_m: guidance.center_x_m,
        center_y_m: guidance.center_y_m,
        nominal_handoff_target_x_m: guidance.nominal_handoff_target_x_m,
        nominal_handoff_target_y_m: guidance.nominal_handoff_target_y_m,
        handoff_target_x_m: guidance.handoff_target_x_m,
        handoff_target_y_m: guidance.handoff_target_y_m,
        handoff_target_mode: guidance.handoff_target_mode,
        remaining_to_handoff_m,
        time_to_handoff_s: terminal_handoff
            .then_some(0.0)
            .or(guidance.time_to_handoff_s),
        target_vx_mps: guidance.target_vx_mps,
        target_vy_mps: guidance.target_vy_mps,
        target_deadline_remaining_s,
        target_velocity_error_mps,
        guidance_feasible: guidance.guidance_feasible,
        predicted_handoff_time_to_go_s,
        predicted_handoff_deadline_lead_s: guidance.predicted_handoff_deadline_lead_s,
        predicted_handoff_contract_status: guidance.predicted_handoff_contract_status,
        predicted_handoff_contract_reasons: guidance.predicted_handoff_contract_reasons,
        predicted_handoff_distance_m: guidance.predicted_handoff_distance_m,
        predicted_handoff_cross_track_m: guidance.predicted_handoff_cross_track_m,
        predicted_handoff_plane_progress_m: guidance.predicted_handoff_plane_progress_m,
        predicted_handoff_outbound_heading_error_rad: guidance
            .predicted_handoff_outbound_heading_error_rad,
        predicted_handoff_outbound_progress_mps: guidance.predicted_handoff_outbound_progress_mps,
        predicted_handoff_outbound_cross_speed_mps: guidance
            .predicted_handoff_outbound_cross_speed_mps,
        predicted_handoff_speed_mps: guidance.predicted_handoff_speed_mps,
        predicted_handoff_vertical_speed_mps: guidance.predicted_handoff_vertical_speed_mps,
        candidate_contract_pass_ever: None,
        candidate_first_pass_time_s: None,
        candidate_last_pass_time_s: None,
        candidate_pass_lost_before_capture: None,
        candidate_best_heading_margin_rad: None,
        candidate_best_cross_speed_margin_mps: None,
        reachable_candidate_contract_pass_ever: None,
        reachable_candidate_first_pass_time_s: None,
        reachable_candidate_last_pass_time_s: None,
        reachable_candidate_pass_lost_before_capture: None,
        reachable_required_accel_ratio_max: None,
        reachable_thrust_saturated_time_max_s: None,
        reachable_tilt_saturated_time_max_s: None,
        continuation_next_waypoint_index: guidance.continuation_next_waypoint_index,
        continuation_contract_pass: guidance.continuation_contract_pass,
        continuation_contract_reasons: guidance.continuation_contract_reasons,
        continuation_outbound_heading_error_rad: guidance.continuation_outbound_heading_error_rad,
        continuation_required_accel_ratio_max: guidance.continuation_required_accel_ratio_max,
        continuation_passing_candidate_count: guidance.continuation_passing_candidate_count,
        transition_next_waypoint_index: guidance.transition_next_waypoint_index,
        transition_position_error_m: guidance.transition_position_error_m,
        transition_velocity_error_mps: guidance.transition_velocity_error_mps,
        transition_attitude_error_rad: guidance.transition_attitude_error_rad,
        transition_mass_error_kg: guidance.transition_mass_error_kg,
        transition_fuel_error_kg: guidance.transition_fuel_error_kg,
        transition_event_time_error_s: guidance.transition_event_time_error_s,
        transition_continuation_contract_pass: guidance.transition_continuation_contract_pass,
        transition_continuation_contract_reasons: guidance.transition_continuation_contract_reasons,
        transition_continuation_outbound_heading_error_rad: guidance
            .transition_continuation_outbound_heading_error_rad,
        transition_continuation_required_accel_ratio_max: guidance
            .transition_continuation_required_accel_ratio_max,
        transition_continuation_passing_candidate_count: guidance
            .transition_continuation_passing_candidate_count,
        joint_next_waypoint_index: guidance.joint_next_waypoint_index,
        joint_evaluated_candidate_count: guidance.joint_evaluated_candidate_count,
        joint_passing_candidate_count: guidance.joint_passing_candidate_count,
        joint_contract_pass: guidance.joint_contract_pass,
        joint_endpoint_x_m: guidance.joint_endpoint_x_m,
        joint_endpoint_y_m: guidance.joint_endpoint_y_m,
        joint_target_vx_mps: guidance.joint_target_vx_mps,
        joint_target_vy_mps: guidance.joint_target_vy_mps,
        joint_time_to_go_s: guidance.joint_time_to_go_s,
        joint_continuation_outbound_heading_error_rad: guidance
            .joint_continuation_outbound_heading_error_rad,
        joint_required_accel_ratio_max: guidance.joint_required_accel_ratio_max,
        joint_total_saturated_time_s: guidance.joint_total_saturated_time_s,
        joint_continuation_passing_candidate_count: guidance
            .joint_continuation_passing_candidate_count,
        plan_reference_position_error_max_m: None,
        plan_reference_cross_error_max_abs_m: None,
        plan_reference_velocity_error_max_mps: None,
        plan_reference_cross_speed_error_max_abs_mps: None,
        guidance_required_accel_ratio_max: None,
        guidance_thrust_saturated_time_s: None,
        guidance_tilt_saturated_time_s: None,
        guidance_first_saturation_lead_s: None,
        last_pass_reference_position_error_m: None,
        last_pass_reference_velocity_error_mps: None,
        last_pass_required_accel_ratio: None,
        guidance_plan_revision_max: None,
        guidance_plan_reasons: Vec::new(),
        handoff_turn_margin_m,
        guidance_snapshot_source: guidance_update.map(|_| "last_pre_capture_update".to_owned()),
        guidance_snapshot_age_s,
        guidance_replan_count: guidance.guidance_replan_count,
    })
}

fn waypoint_sample_stats(
    scenario: &ScenarioSpec,
    observation: &Observation,
    waypoint_index: usize,
) -> Option<WaypointHandoffKinematics> {
    let route = scenario.mission.transfer_route.as_ref()?;
    let waypoint = route.waypoints.get(waypoint_index)?;
    let anchor_m = if waypoint_index == 0 {
        let source_pad = scenario.world.landing_pad(&route.source_pad_id)?;
        Vec2::new(source_pad.center_x_m, source_pad.surface_y_m)
    } else {
        route.waypoints.get(waypoint_index - 1)?.position_m
    };
    let next_target_m = route
        .waypoints
        .get(waypoint_index + 1)
        .map(|next| next.position_m)
        .or_else(|| {
            scenario
                .world
                .landing_pad(&route.target_pad_id)
                .map(|pad| Vec2::new(pad.center_x_m, pad.surface_y_m))
        })?;
    let target_m = waypoint.position_m;
    let leg_unit = waypoint_normalized(target_m - anchor_m)?;
    let next_leg_unit = waypoint_normalized(next_target_m - target_m)?;
    let handoff_tangent_unit = waypoint.handoff_tangent_unit.unwrap_or(next_leg_unit);
    let to_waypoint_m = observation.position_m - target_m;
    let speed_mps = observation.velocity_mps.length();
    let velocity_unit = if speed_mps > 1.0e-9 {
        observation.velocity_mps * (1.0 / speed_mps)
    } else {
        Vec2::new(0.0, 0.0)
    };
    let outbound_heading_error_rad = waypoint_dot(velocity_unit, handoff_tangent_unit)
        .clamp(-1.0, 1.0)
        .acos();
    Some(WaypointHandoffKinematics {
        distance_m: to_waypoint_m.length(),
        cross_track_m: waypoint_cross(to_waypoint_m, leg_unit).abs(),
        plane_progress_m: waypoint_dot(to_waypoint_m, leg_unit),
        outbound_heading_error_rad,
        outbound_progress_mps: waypoint_dot(observation.velocity_mps, handoff_tangent_unit),
        outbound_cross_speed_mps: waypoint_cross(observation.velocity_mps, handoff_tangent_unit)
            .abs(),
        speed_mps,
        vertical_speed_mps: observation.velocity_mps.y,
    })
}

fn waypoint_capture_passes_review(
    waypoint: &TransferWaypointSpec,
    stats: &WaypointHandoffKinematics,
) -> bool {
    waypoint.assess_handoff(*stats).spatial_pass
}

fn waypoint_normalized(vector: Vec2) -> Option<Vec2> {
    let length = vector.length();
    (length > 1.0e-9).then(|| vector * (1.0 / length))
}

fn waypoint_dot(lhs: Vec2, rhs: Vec2) -> f64 {
    (lhs.x * rhs.x) + (lhs.y * rhs.y)
}

fn waypoint_cross(lhs: Vec2, rhs: Vec2) -> f64 {
    (lhs.x * rhs.y) - (lhs.y * rhs.x)
}

fn batch_waypoint_handoff_metrics(
    scenario: &ScenarioSpec,
    waypoint: &WaypointReviewMetrics,
) -> Option<BatchWaypointHandoffReviewMetrics> {
    let waypoint_index = waypoint
        .active_index
        .and_then(|index| usize::try_from(index).ok())?;
    let contract = waypoint_contract_review_metrics(scenario, waypoint);
    let waypoint_id = scenario
        .mission
        .transfer_route
        .as_ref()
        .and_then(|route| route.waypoints.get(waypoint_index))
        .map(|spec| spec.id.clone());
    let continuation_matches_handoff =
        waypoint.continuation_next_waypoint_index == waypoint_index.checked_add(1);
    let transition_matches_handoff =
        waypoint.transition_next_waypoint_index == waypoint_index.checked_add(1);
    let joint_matches_handoff = waypoint.joint_next_waypoint_index == waypoint_index.checked_add(1);
    Some(BatchWaypointHandoffReviewMetrics {
        waypoint_index,
        waypoint_id,
        capture_status: waypoint.capture_status.clone(),
        contract_status: contract.status,
        contract_reasons: contract.reasons,
        capture_time_s: waypoint.capture_time_s,
        window_entry: waypoint.window_entry.clone(),
        resolution_reason: waypoint.resolution_reason.clone(),
        window_duration_s: waypoint.window_duration_s,
        closest_distance_m: waypoint.closest_distance_m,
        distance_m: waypoint.distance_m,
        cross_track_m: waypoint.cross_track_m,
        plane_progress_m: waypoint.plane_progress_m,
        outbound_heading_error_rad: waypoint.outbound_heading_error_rad,
        outbound_progress_mps: waypoint.outbound_progress_mps,
        outbound_cross_speed_mps: waypoint.outbound_cross_speed_mps,
        speed_mps: waypoint.speed_mps,
        vertical_speed_mps: waypoint.vertical_speed_mps,
        remaining_to_plane_m: waypoint.remaining_to_plane_m,
        time_to_plane_s: waypoint.time_to_plane_s,
        required_turn_distance_m: waypoint.required_turn_distance_m,
        shaping_start_distance_m: waypoint.shaping_start_distance_m,
        turn_margin_m: waypoint.turn_margin_m,
        center_x_m: waypoint.center_x_m,
        center_y_m: waypoint.center_y_m,
        nominal_handoff_target_x_m: waypoint.nominal_handoff_target_x_m,
        nominal_handoff_target_y_m: waypoint.nominal_handoff_target_y_m,
        handoff_target_x_m: waypoint.handoff_target_x_m,
        handoff_target_y_m: waypoint.handoff_target_y_m,
        handoff_target_mode: waypoint.handoff_target_mode.clone(),
        remaining_to_handoff_m: waypoint.remaining_to_handoff_m,
        time_to_handoff_s: waypoint.time_to_handoff_s,
        target_vx_mps: waypoint.target_vx_mps,
        target_vy_mps: waypoint.target_vy_mps,
        target_deadline_remaining_s: waypoint.target_deadline_remaining_s,
        target_velocity_error_mps: waypoint.target_velocity_error_mps,
        guidance_feasible: waypoint.guidance_feasible,
        predicted_handoff_time_to_go_s: waypoint.predicted_handoff_time_to_go_s,
        predicted_handoff_deadline_lead_s: waypoint.predicted_handoff_deadline_lead_s,
        predicted_handoff_contract_status: waypoint.predicted_handoff_contract_status.clone(),
        predicted_handoff_contract_reasons: waypoint.predicted_handoff_contract_reasons.clone(),
        predicted_handoff_distance_m: waypoint.predicted_handoff_distance_m,
        predicted_handoff_cross_track_m: waypoint.predicted_handoff_cross_track_m,
        predicted_handoff_plane_progress_m: waypoint.predicted_handoff_plane_progress_m,
        predicted_handoff_outbound_heading_error_rad: waypoint
            .predicted_handoff_outbound_heading_error_rad,
        predicted_handoff_outbound_progress_mps: waypoint.predicted_handoff_outbound_progress_mps,
        predicted_handoff_outbound_cross_speed_mps: waypoint
            .predicted_handoff_outbound_cross_speed_mps,
        predicted_handoff_speed_mps: waypoint.predicted_handoff_speed_mps,
        predicted_handoff_vertical_speed_mps: waypoint.predicted_handoff_vertical_speed_mps,
        candidate_contract_pass_ever: waypoint.candidate_contract_pass_ever,
        candidate_first_pass_time_s: waypoint.candidate_first_pass_time_s,
        candidate_last_pass_time_s: waypoint.candidate_last_pass_time_s,
        candidate_pass_lost_before_capture: waypoint.candidate_pass_lost_before_capture,
        candidate_best_heading_margin_rad: waypoint.candidate_best_heading_margin_rad,
        candidate_best_cross_speed_margin_mps: waypoint.candidate_best_cross_speed_margin_mps,
        reachable_candidate_contract_pass_ever: waypoint.reachable_candidate_contract_pass_ever,
        reachable_candidate_first_pass_time_s: waypoint.reachable_candidate_first_pass_time_s,
        reachable_candidate_last_pass_time_s: waypoint.reachable_candidate_last_pass_time_s,
        reachable_candidate_pass_lost_before_capture: waypoint
            .reachable_candidate_pass_lost_before_capture,
        reachable_required_accel_ratio_max: waypoint.reachable_required_accel_ratio_max,
        reachable_thrust_saturated_time_max_s: waypoint.reachable_thrust_saturated_time_max_s,
        reachable_tilt_saturated_time_max_s: waypoint.reachable_tilt_saturated_time_max_s,
        continuation_next_waypoint_index: continuation_matches_handoff
            .then_some(waypoint.continuation_next_waypoint_index)
            .flatten(),
        continuation_contract_pass: continuation_matches_handoff
            .then_some(waypoint.continuation_contract_pass)
            .flatten(),
        continuation_contract_reasons: continuation_matches_handoff
            .then_some(waypoint.continuation_contract_reasons.clone())
            .unwrap_or_default(),
        continuation_outbound_heading_error_rad: continuation_matches_handoff
            .then_some(waypoint.continuation_outbound_heading_error_rad)
            .flatten(),
        continuation_required_accel_ratio_max: continuation_matches_handoff
            .then_some(waypoint.continuation_required_accel_ratio_max)
            .flatten(),
        continuation_passing_candidate_count: continuation_matches_handoff
            .then_some(waypoint.continuation_passing_candidate_count)
            .flatten(),
        transition_next_waypoint_index: transition_matches_handoff
            .then_some(waypoint.transition_next_waypoint_index)
            .flatten(),
        transition_position_error_m: transition_matches_handoff
            .then_some(waypoint.transition_position_error_m)
            .flatten(),
        transition_velocity_error_mps: transition_matches_handoff
            .then_some(waypoint.transition_velocity_error_mps)
            .flatten(),
        transition_attitude_error_rad: transition_matches_handoff
            .then_some(waypoint.transition_attitude_error_rad)
            .flatten(),
        transition_mass_error_kg: transition_matches_handoff
            .then_some(waypoint.transition_mass_error_kg)
            .flatten(),
        transition_fuel_error_kg: transition_matches_handoff
            .then_some(waypoint.transition_fuel_error_kg)
            .flatten(),
        transition_event_time_error_s: transition_matches_handoff
            .then_some(waypoint.transition_event_time_error_s)
            .flatten(),
        transition_continuation_contract_pass: transition_matches_handoff
            .then_some(waypoint.transition_continuation_contract_pass)
            .flatten(),
        transition_continuation_contract_reasons: if transition_matches_handoff {
            waypoint.transition_continuation_contract_reasons.clone()
        } else {
            Vec::new()
        },
        transition_continuation_outbound_heading_error_rad: transition_matches_handoff
            .then_some(waypoint.transition_continuation_outbound_heading_error_rad)
            .flatten(),
        transition_continuation_required_accel_ratio_max: transition_matches_handoff
            .then_some(waypoint.transition_continuation_required_accel_ratio_max)
            .flatten(),
        transition_continuation_passing_candidate_count: transition_matches_handoff
            .then_some(waypoint.transition_continuation_passing_candidate_count)
            .flatten(),
        joint_next_waypoint_index: joint_matches_handoff
            .then_some(waypoint.joint_next_waypoint_index)
            .flatten(),
        joint_evaluated_candidate_count: joint_matches_handoff
            .then_some(waypoint.joint_evaluated_candidate_count)
            .flatten(),
        joint_passing_candidate_count: joint_matches_handoff
            .then_some(waypoint.joint_passing_candidate_count)
            .flatten(),
        joint_contract_pass: joint_matches_handoff
            .then_some(waypoint.joint_contract_pass)
            .flatten(),
        joint_endpoint_x_m: joint_matches_handoff
            .then_some(waypoint.joint_endpoint_x_m)
            .flatten(),
        joint_endpoint_y_m: joint_matches_handoff
            .then_some(waypoint.joint_endpoint_y_m)
            .flatten(),
        joint_target_vx_mps: joint_matches_handoff
            .then_some(waypoint.joint_target_vx_mps)
            .flatten(),
        joint_target_vy_mps: joint_matches_handoff
            .then_some(waypoint.joint_target_vy_mps)
            .flatten(),
        joint_time_to_go_s: joint_matches_handoff
            .then_some(waypoint.joint_time_to_go_s)
            .flatten(),
        joint_continuation_outbound_heading_error_rad: joint_matches_handoff
            .then_some(waypoint.joint_continuation_outbound_heading_error_rad)
            .flatten(),
        joint_required_accel_ratio_max: joint_matches_handoff
            .then_some(waypoint.joint_required_accel_ratio_max)
            .flatten(),
        joint_total_saturated_time_s: joint_matches_handoff
            .then_some(waypoint.joint_total_saturated_time_s)
            .flatten(),
        joint_continuation_passing_candidate_count: joint_matches_handoff
            .then_some(waypoint.joint_continuation_passing_candidate_count)
            .flatten(),
        plan_reference_position_error_max_m: waypoint.plan_reference_position_error_max_m,
        plan_reference_cross_error_max_abs_m: waypoint.plan_reference_cross_error_max_abs_m,
        plan_reference_velocity_error_max_mps: waypoint.plan_reference_velocity_error_max_mps,
        plan_reference_cross_speed_error_max_abs_mps: waypoint
            .plan_reference_cross_speed_error_max_abs_mps,
        guidance_required_accel_ratio_max: waypoint.guidance_required_accel_ratio_max,
        guidance_thrust_saturated_time_s: waypoint.guidance_thrust_saturated_time_s,
        guidance_tilt_saturated_time_s: waypoint.guidance_tilt_saturated_time_s,
        guidance_first_saturation_lead_s: waypoint.guidance_first_saturation_lead_s,
        last_pass_reference_position_error_m: waypoint.last_pass_reference_position_error_m,
        last_pass_reference_velocity_error_mps: waypoint.last_pass_reference_velocity_error_mps,
        last_pass_required_accel_ratio: waypoint.last_pass_required_accel_ratio,
        guidance_plan_revision_max: waypoint.guidance_plan_revision_max,
        guidance_plan_reasons: waypoint.guidance_plan_reasons.clone(),
        handoff_turn_margin_m: waypoint.handoff_turn_margin_m,
        guidance_snapshot_source: waypoint.guidance_snapshot_source.clone(),
        guidance_snapshot_age_s: waypoint.guidance_snapshot_age_s,
        guidance_replan_count: waypoint.guidance_replan_count,
    })
}

#[derive(Clone, Debug, Default)]
struct WaypointRouteReviewMetrics {
    status: Option<String>,
    passed: Option<usize>,
    total: Option<usize>,
    first_failure_index: Option<usize>,
}

fn waypoint_route_review_metrics(
    scenario: &ScenarioSpec,
    manifest: &RunManifest,
    handoffs: &[BatchWaypointHandoffReviewMetrics],
) -> WaypointRouteReviewMetrics {
    if !matches!(
        scenario.mission.goal,
        EvaluationGoal::LandingOnPad { .. } | EvaluationGoal::WaypointSequence { .. }
    ) {
        return WaypointRouteReviewMetrics::default();
    }
    let Some(route) = scenario
        .mission
        .transfer_route
        .as_ref()
        .filter(|route| !route.waypoints.is_empty())
    else {
        return WaypointRouteReviewMetrics::default();
    };

    let total = route.waypoints.len();
    let sequence_summary = manifest.summary.waypoint_sequence.as_ref();
    let passed = sequence_summary.map_or_else(
        || {
            handoffs
                .iter()
                .enumerate()
                .take_while(|(expected_index, handoff)| {
                    handoff.waypoint_index == *expected_index
                        && handoff.contract_status.as_deref() == Some("pass")
                })
                .count()
        },
        |summary| summary.passed_handoffs,
    );
    let first_failure_index = sequence_summary
        .and_then(|summary| summary.first_failed_index)
        .or_else(|| {
            handoffs.iter().find_map(|handoff| {
                (!matches!(
                    handoff.contract_status.as_deref(),
                    Some("pass" | "incomplete") | None
                ))
                .then_some(handoff.waypoint_index)
            })
        });
    let status = if passed == total {
        "pass"
    } else if first_failure_index.is_some()
        || !matches!(manifest.mission_outcome, MissionOutcome::InProgress)
    {
        "failed"
    } else {
        "incomplete"
    };

    WaypointRouteReviewMetrics {
        status: Some(status.to_owned()),
        passed: Some(passed),
        total: Some(total),
        first_failure_index,
    }
}

fn waypoint_contract_review_metrics(
    scenario: &ScenarioSpec,
    waypoint: &WaypointReviewMetrics,
) -> WaypointContractReviewMetrics {
    let Some(capture_status) = waypoint.capture_status.as_deref() else {
        return WaypointContractReviewMetrics::default();
    };
    let Some(route) = scenario.mission.transfer_route.as_ref() else {
        return WaypointContractReviewMetrics {
            status: Some("unknown".to_owned()),
            reasons: vec!["missing_route".to_owned()],
        };
    };
    let Some(active_index) = waypoint
        .active_index
        .and_then(|index| usize::try_from(index).ok())
    else {
        return WaypointContractReviewMetrics {
            status: Some("unknown".to_owned()),
            reasons: vec!["missing_waypoint_index".to_owned()],
        };
    };
    let Some(spec) = route.waypoints.get(active_index) else {
        return WaypointContractReviewMetrics {
            status: Some("unknown".to_owned()),
            reasons: vec!["missing_waypoint_spec".to_owned()],
        };
    };

    match capture_status {
        "tracking" | "capture_window" => WaypointContractReviewMetrics {
            status: Some("incomplete".to_owned()),
            reasons: vec![capture_status.to_owned()],
        },
        "missed" => WaypointContractReviewMetrics {
            status: Some("spatial_miss".to_owned()),
            reasons: waypoint_spatial_miss_reasons(spec, waypoint),
        },
        "captured" => waypoint_outbound_contract_review_metrics(spec, waypoint),
        _ => WaypointContractReviewMetrics {
            status: Some("unknown".to_owned()),
            reasons: vec![format!("unknown_capture_status:{capture_status}")],
        },
    }
}

fn waypoint_spatial_miss_reasons(
    spec: &TransferWaypointSpec,
    waypoint: &WaypointReviewMetrics,
) -> Vec<String> {
    let mut reasons = Vec::new();
    match waypoint.distance_m {
        Some(distance_m) if distance_m <= spec.capture_radius_m => {}
        Some(_) => reasons.push("outside_capture_radius".to_owned()),
        None => reasons.push("missing_distance".to_owned()),
    }
    match waypoint.cross_track_m {
        Some(cross_track_m) if cross_track_m <= spec.max_cross_track_m => {}
        Some(_) => reasons.push("cross_track".to_owned()),
        None => reasons.push("missing_cross_track".to_owned()),
    }
    match waypoint.plane_progress_m {
        Some(plane_progress_m) if plane_progress_m >= -spec.capture_radius_m => {}
        Some(_) => reasons.push("before_waypoint_plane".to_owned()),
        None => reasons.push("missing_plane_progress".to_owned()),
    }
    if reasons.is_empty() {
        reasons.push("spatial_miss".to_owned());
    }
    reasons
}

fn waypoint_outbound_contract_review_metrics(
    spec: &TransferWaypointSpec,
    waypoint: &WaypointReviewMetrics,
) -> WaypointContractReviewMetrics {
    let mut reasons = Vec::new();
    let Some(outbound_heading_error_rad) = waypoint.outbound_heading_error_rad else {
        reasons.push("missing_heading".to_owned());
        return WaypointContractReviewMetrics {
            status: Some("outbound_out_of_envelope".to_owned()),
            reasons,
        };
    };
    let Some(outbound_progress_mps) = waypoint.outbound_progress_mps else {
        reasons.push("missing_outbound_progress".to_owned());
        return WaypointContractReviewMetrics {
            status: Some("outbound_out_of_envelope".to_owned()),
            reasons,
        };
    };
    let Some(speed_mps) = waypoint.speed_mps else {
        reasons.push("missing_speed".to_owned());
        return WaypointContractReviewMetrics {
            status: Some("outbound_out_of_envelope".to_owned()),
            reasons,
        };
    };
    let outbound_cross_speed_mps = match (
        spec.max_outbound_cross_speed_mps,
        waypoint.outbound_cross_speed_mps,
    ) {
        (Some(_), None) => {
            reasons.push("missing_outbound_cross_speed".to_owned());
            0.0
        }
        (_, value) => value.unwrap_or(0.0),
    };
    let vertical_speed_mps = match (
        spec.min_vertical_speed_mps.is_some() || spec.max_vertical_speed_mps.is_some(),
        waypoint.vertical_speed_mps,
    ) {
        (true, None) => {
            reasons.push("missing_vertical_speed".to_owned());
            0.0
        }
        (_, value) => value.unwrap_or(0.0),
    };

    if reasons.is_empty() {
        reasons.extend(
            spec.assess_handoff(WaypointHandoffKinematics {
                distance_m: waypoint.distance_m.unwrap_or(0.0),
                cross_track_m: waypoint.cross_track_m.unwrap_or(0.0),
                plane_progress_m: waypoint.plane_progress_m.unwrap_or(0.0),
                outbound_heading_error_rad,
                outbound_progress_mps,
                outbound_cross_speed_mps,
                speed_mps,
                vertical_speed_mps,
            })
            .violations
            .into_iter()
            .map(|violation| violation.as_str().to_owned()),
        );
    }

    if reasons.is_empty() {
        WaypointContractReviewMetrics {
            status: Some("pass".to_owned()),
            reasons,
        }
    } else {
        WaypointContractReviewMetrics {
            status: Some("outbound_out_of_envelope".to_owned()),
            reasons,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct TransferReviewMetrics {
    terminal_entry_kind: Option<String>,
    terminal_handoff_time_s: Option<f64>,
    terminal_handoff_dx_m: Option<f64>,
    terminal_handoff_height_m: Option<f64>,
    terminal_handoff_speed_mps: Option<f64>,
    terminal_handoff_gate_mode: Option<String>,
    terminal_handoff_projected_dx_m: Option<f64>,
    terminal_handoff_impact_angle_deg: Option<f64>,
    terminal_handoff_boost_quality: Option<String>,
    terminal_handoff_latest_safe_margin_s: Option<f64>,
    terminal_handoff_required_accel_ratio: Option<f64>,
    terminal_post_handoff_apex_gain_m: Option<f64>,
    terminal_post_handoff_time_to_apex_s: Option<f64>,
    terminal_post_handoff_apex_dx_abs_m: Option<f64>,
    final_phase: Option<String>,
    boost_projected_dx_m: Option<f64>,
    boost_impact_angle_deg: Option<f64>,
    boost_apex_over_target_m: Option<f64>,
    boost_quality: Option<String>,
    boost_selected_score: Option<f64>,
    boost_settled_quality: Option<String>,
    boost_settled_projected_dx_m: Option<f64>,
    boost_cutoff_time_s: Option<f64>,
    boost_cutoff_projected_dx_m: Option<f64>,
    boost_cutoff_impact_angle_deg: Option<f64>,
    boost_cutoff_apex_over_target_m: Option<f64>,
    boost_cutoff_quality: Option<String>,
    boost_burn_duration_s: Option<f64>,
    boost_burn_fuel_used_kg: Option<f64>,
    boost_burn_avg_throttle: Option<f64>,
    terminal_gate_mode: Option<String>,
    terminal_gate_latest_safe_margin_s: Option<f64>,
    terminal_gate_required_accel_ratio: Option<f64>,
    terminal_gate_deferred: Option<bool>,
    corridor_mode: Option<String>,
    corridor_min_margin_m: Option<f64>,
}

fn transfer_review_metrics(
    controller_updates: &[pd_control::ControllerUpdateRecord],
    samples: &[SampleRecord],
) -> TransferReviewMetrics {
    let final_phase = controller_updates
        .iter()
        .rev()
        .find_map(|update| telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE))
        .map(ToOwned::to_owned);
    let Some((handoff_index, handoff)) =
        controller_updates.iter().enumerate().find(|(_, update)| {
            telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("terminal")
        })
    else {
        let mut metrics = transfer_review_metrics_without_handoff(controller_updates, samples);
        metrics.final_phase = final_phase;
        return metrics;
    };
    let terminal_entry_kind = if controller_updates[..handoff_index].iter().any(|update| {
        matches!(
            telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE),
            Some("boost" | "coast")
        )
    }) {
        "handoff"
    } else {
        "direct"
    };
    let terminal_handoff_dx_m = telemetry_float(&handoff.frame.metrics, metric::TARGET_DX_M);
    let terminal_handoff_height_m =
        telemetry_float(&handoff.frame.metrics, metric::HEIGHT_ABOVE_TARGET_M);
    let terminal_handoff_speed_mps =
        telemetry_float(&handoff.frame.metrics, metric::VERTICAL_SPEED_MPS)
            .zip(telemetry_float(
                &handoff.frame.metrics,
                metric::TANGENTIAL_SPEED_MPS,
            ))
            .map(|(vertical, tangential)| vertical.hypot(tangential));
    let terminal_handoff_gate_mode =
        telemetry_text(&handoff.frame.metrics, metric::TRANSFER_TERMINAL_GATE_MODE)
            .map(ToOwned::to_owned);
    let terminal_handoff_projected_dx_m =
        telemetry_float(&handoff.frame.metrics, metric::TRANSFER_PROJECTED_DX_M);
    let terminal_handoff_impact_angle_deg =
        telemetry_float(&handoff.frame.metrics, metric::TRANSFER_IMPACT_ANGLE_DEG)
            .filter(|value| *value >= 0.0);
    let terminal_handoff_boost_quality =
        telemetry_text(&handoff.frame.metrics, metric::TRANSFER_BOOST_QUALITY)
            .map(ToOwned::to_owned);
    let terminal_handoff_latest_safe_margin_s = telemetry_float(
        &handoff.frame.metrics,
        metric::TRANSFER_TERMINAL_GATE_LATEST_SAFE_MARGIN_S,
    );
    let terminal_handoff_required_accel_ratio = telemetry_float(
        &handoff.frame.metrics,
        metric::TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO,
    );
    let terminal_samples = samples
        .iter()
        .filter(|sample| sample.physics_step >= handoff.physics_step)
        .collect::<Vec<_>>();
    let terminal_apex = terminal_samples.iter().copied().max_by(|lhs, rhs| {
        lhs.observation
            .height_above_target_m
            .partial_cmp(&rhs.observation.height_above_target_m)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let terminal_entry = terminal_samples.first().copied();
    let terminal_post_handoff_apex_gain_m =
        terminal_entry.zip(terminal_apex).map(|(entry, apex)| {
            (apex.observation.height_above_target_m - entry.observation.height_above_target_m)
                .max(0.0)
        });
    let terminal_post_handoff_time_to_apex_s = terminal_entry
        .zip(terminal_apex)
        .map(|(entry, apex)| (apex.sim_time_s - entry.sim_time_s).max(0.0));
    let terminal_post_handoff_apex_dx_abs_m =
        terminal_apex.map(|apex| apex.observation.target_dx_m.abs());
    let mut metrics = transfer_review_metrics_without_handoff(controller_updates, samples);
    metrics.terminal_entry_kind = Some(terminal_entry_kind.to_owned());
    metrics.terminal_handoff_time_s = Some(handoff.sim_time_s);
    metrics.terminal_handoff_dx_m = terminal_handoff_dx_m;
    metrics.terminal_handoff_height_m = terminal_handoff_height_m;
    metrics.terminal_handoff_speed_mps = terminal_handoff_speed_mps;
    metrics.terminal_handoff_gate_mode = terminal_handoff_gate_mode;
    metrics.terminal_handoff_projected_dx_m = terminal_handoff_projected_dx_m;
    metrics.terminal_handoff_impact_angle_deg = terminal_handoff_impact_angle_deg;
    metrics.terminal_handoff_boost_quality = terminal_handoff_boost_quality;
    metrics.terminal_handoff_latest_safe_margin_s = terminal_handoff_latest_safe_margin_s;
    metrics.terminal_handoff_required_accel_ratio = terminal_handoff_required_accel_ratio;
    metrics.terminal_post_handoff_apex_gain_m = terminal_post_handoff_apex_gain_m;
    metrics.terminal_post_handoff_time_to_apex_s = terminal_post_handoff_time_to_apex_s;
    metrics.terminal_post_handoff_apex_dx_abs_m = terminal_post_handoff_apex_dx_abs_m;
    metrics.final_phase = final_phase;
    metrics
}

fn transfer_review_metrics_without_handoff(
    controller_updates: &[pd_control::ControllerUpdateRecord],
    samples: &[SampleRecord],
) -> TransferReviewMetrics {
    let last_boost_update = controller_updates.iter().rev().find(|update| {
        telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("boost")
            && telemetry_text(&update.frame.metrics, metric::TRANSFER_BOOST_QUALITY).is_some()
    });
    let (
        boost_projected_dx_m,
        boost_impact_angle_deg,
        boost_apex_over_target_m,
        boost_quality,
        boost_selected_score,
        boost_settled_quality,
        boost_settled_projected_dx_m,
    ) = last_boost_update
        .map(|update| {
            (
                telemetry_float(&update.frame.metrics, metric::TRANSFER_PROJECTED_DX_M),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_IMPACT_ANGLE_DEG)
                    .filter(|value| *value >= 0.0),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_APEX_OVER_TARGET_M),
                telemetry_text(&update.frame.metrics, metric::TRANSFER_BOOST_QUALITY)
                    .map(ToOwned::to_owned),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_BOOST_SELECTED_SCORE),
                telemetry_text(
                    &update.frame.metrics,
                    metric::TRANSFER_BOOST_SETTLED_QUALITY,
                )
                .map(ToOwned::to_owned),
                telemetry_float(
                    &update.frame.metrics,
                    metric::TRANSFER_BOOST_SETTLED_PROJECTED_DX_M,
                ),
            )
        })
        .unwrap_or((None, None, None, None, None, None, None));
    let boost_cutoff = transfer_boost_cutoff_update(controller_updates);
    let (
        boost_cutoff_time_s,
        boost_cutoff_projected_dx_m,
        boost_cutoff_impact_angle_deg,
        boost_cutoff_apex_over_target_m,
        boost_cutoff_quality,
    ) = boost_cutoff
        .map(|update| {
            (
                Some(update.sim_time_s),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_PROJECTED_DX_M),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_IMPACT_ANGLE_DEG)
                    .filter(|value| *value >= 0.0),
                telemetry_float(&update.frame.metrics, metric::TRANSFER_APEX_OVER_TARGET_M),
                telemetry_text(&update.frame.metrics, metric::TRANSFER_BOOST_QUALITY)
                    .map(ToOwned::to_owned),
            )
        })
        .unwrap_or((None, None, None, None, None));
    let boost_burn = transfer_boost_burn_metrics(controller_updates, samples, boost_cutoff);
    let terminal_gate_update = controller_updates.iter().rev().find(|update| {
        telemetry_text(&update.frame.metrics, metric::TRANSFER_TERMINAL_GATE_MODE).is_some()
    });
    let terminal_gate_deferred = if controller_updates.iter().any(|update| {
        telemetry_bool(
            &update.frame.metrics,
            metric::TRANSFER_TERMINAL_GATE_DEFERRED,
        ) == Some(true)
    }) {
        Some(true)
    } else {
        terminal_gate_update.and_then(|update| {
            telemetry_bool(
                &update.frame.metrics,
                metric::TRANSFER_TERMINAL_GATE_DEFERRED,
            )
        })
    };
    let (
        terminal_gate_mode,
        terminal_gate_latest_safe_margin_s,
        terminal_gate_required_accel_ratio,
    ) = terminal_gate_update
        .map(|update| {
            (
                telemetry_text(&update.frame.metrics, metric::TRANSFER_TERMINAL_GATE_MODE)
                    .map(ToOwned::to_owned),
                telemetry_float(
                    &update.frame.metrics,
                    metric::TRANSFER_TERMINAL_GATE_LATEST_SAFE_MARGIN_S,
                ),
                telemetry_float(
                    &update.frame.metrics,
                    metric::TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO,
                ),
            )
        })
        .unwrap_or((None, None, None));
    let corridor_min_margin_m = controller_updates
        .iter()
        .filter(|update| {
            telemetry_text(&update.frame.metrics, metric::TRANSFER_CORRIDOR_MODE)
                .is_some_and(|mode| mode != "inactive")
        })
        .filter_map(|update| {
            telemetry_float(&update.frame.metrics, metric::TRANSFER_CORRIDOR_MARGIN_M)
        })
        .filter(|margin_m| margin_m.is_finite())
        .reduce(f64::min);
    let last_corridor_mode = controller_updates
        .iter()
        .rev()
        .filter_map(|update| telemetry_text(&update.frame.metrics, metric::TRANSFER_CORRIDOR_MODE))
        .find(|mode| *mode != "inactive")
        .map(ToOwned::to_owned);
    let corridor_mode = if controller_updates.iter().any(|update| {
        telemetry_text(&update.frame.metrics, metric::TRANSFER_CORRIDOR_MODE) == Some("active")
    }) {
        Some("active".to_owned())
    } else {
        last_corridor_mode
    };

    TransferReviewMetrics {
        boost_projected_dx_m,
        boost_impact_angle_deg,
        boost_apex_over_target_m,
        boost_quality,
        boost_selected_score,
        boost_settled_quality,
        boost_settled_projected_dx_m,
        boost_cutoff_time_s,
        boost_cutoff_projected_dx_m,
        boost_cutoff_impact_angle_deg,
        boost_cutoff_apex_over_target_m,
        boost_cutoff_quality,
        boost_burn_duration_s: boost_burn.and_then(|metrics| metrics.duration_s),
        boost_burn_fuel_used_kg: boost_burn.and_then(|metrics| metrics.fuel_used_kg),
        boost_burn_avg_throttle: boost_burn.and_then(|metrics| metrics.avg_throttle),
        terminal_gate_mode,
        terminal_gate_latest_safe_margin_s,
        terminal_gate_required_accel_ratio,
        terminal_gate_deferred,
        corridor_mode,
        corridor_min_margin_m,
        ..TransferReviewMetrics::default()
    }
}

fn transfer_boost_cutoff_update(
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> Option<&pd_control::ControllerUpdateRecord> {
    let mut last_boost = None;
    for update in controller_updates {
        match telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) {
            Some("boost") => last_boost = Some(update),
            Some(_) if last_boost.is_some() => return last_boost,
            _ => {}
        }
    }
    None
}

#[derive(Clone, Copy, Debug)]
struct TransferBoostBurnMetrics {
    duration_s: Option<f64>,
    fuel_used_kg: Option<f64>,
    avg_throttle: Option<f64>,
}

fn transfer_boost_burn_metrics(
    controller_updates: &[pd_control::ControllerUpdateRecord],
    samples: &[SampleRecord],
    boost_cutoff: Option<&pd_control::ControllerUpdateRecord>,
) -> Option<TransferBoostBurnMetrics> {
    let first_boost = controller_updates.iter().find(|update| {
        telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("boost")
    })?;
    let cutoff = boost_cutoff
        .or_else(|| {
            controller_updates.iter().rev().find(|update| {
                telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("boost")
            })
        })
        .unwrap_or(first_boost);
    let duration_s = (cutoff.sim_time_s - first_boost.sim_time_s)
        .is_finite()
        .then_some((cutoff.sim_time_s - first_boost.sim_time_s).max(0.0));
    let fuel_used_kg = sample_at_or_after_step(samples, first_boost.physics_step)
        .zip(sample_at_or_after_step(samples, cutoff.physics_step))
        .map(|(start, end)| start.observation.fuel_kg - end.observation.fuel_kg)
        .filter(|value| value.is_finite())
        .map(|value| value.max(0.0));
    let avg_throttle = transfer_boost_avg_throttle(controller_updates, first_boost, cutoff);

    Some(TransferBoostBurnMetrics {
        duration_s,
        fuel_used_kg,
        avg_throttle,
    })
}

fn transfer_boost_avg_throttle(
    controller_updates: &[pd_control::ControllerUpdateRecord],
    first_boost: &pd_control::ControllerUpdateRecord,
    cutoff: &pd_control::ControllerUpdateRecord,
) -> Option<f64> {
    let boost_updates = controller_updates
        .iter()
        .enumerate()
        .filter(|(_, update)| {
            update.physics_step >= first_boost.physics_step
                && update.physics_step <= cutoff.physics_step
                && telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("boost")
        })
        .collect::<Vec<_>>();
    if boost_updates.is_empty() {
        return None;
    }

    let mut weighted_sum = 0.0;
    let mut total_dt = 0.0;
    for (update_index, update) in boost_updates {
        let next_time_s = controller_updates
            .get(update_index + 1)
            .map(|next| next.sim_time_s)
            .unwrap_or(cutoff.sim_time_s)
            .min(cutoff.sim_time_s);
        let dt = (next_time_s - update.sim_time_s).max(0.0);
        if dt > 0.0 {
            weighted_sum += update.frame.command.throttle_frac * dt;
            total_dt += dt;
        }
    }
    if total_dt > 1e-9 {
        Some(weighted_sum / total_dt)
    } else {
        Some(
            controller_updates
                .iter()
                .filter(|update| {
                    update.physics_step >= first_boost.physics_step
                        && update.physics_step <= cutoff.physics_step
                        && telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE)
                            == Some("boost")
                })
                .map(|update| update.frame.command.throttle_frac)
                .sum::<f64>()
                / controller_updates
                    .iter()
                    .filter(|update| {
                        update.physics_step >= first_boost.physics_step
                            && update.physics_step <= cutoff.physics_step
                            && telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE)
                                == Some("boost")
                    })
                    .count()
                    .max(1) as f64,
        )
    }
}

fn sample_at_or_after_step(samples: &[SampleRecord], physics_step: u64) -> Option<&SampleRecord> {
    samples
        .iter()
        .find(|sample| sample.physics_step >= physics_step)
        .or_else(|| samples.last())
}

fn telemetry_text<'a>(metrics: &'a BTreeMap<String, TelemetryValue>, key: &str) -> Option<&'a str> {
    match metrics.get(key)? {
        TelemetryValue::Text(value) => Some(value),
        _ => None,
    }
}

fn telemetry_float(metrics: &BTreeMap<String, TelemetryValue>, key: &str) -> Option<f64> {
    match metrics.get(key)? {
        TelemetryValue::Float(value) => Some(*value),
        TelemetryValue::Integer(value) => Some(*value as f64),
        _ => None,
    }
}

fn telemetry_integer(metrics: &BTreeMap<String, TelemetryValue>, key: &str) -> Option<i64> {
    match metrics.get(key)? {
        TelemetryValue::Integer(value) => Some(*value),
        _ => None,
    }
}

fn telemetry_bool(metrics: &BTreeMap<String, TelemetryValue>, key: &str) -> Option<bool> {
    match metrics.get(key)? {
        TelemetryValue::Bool(value) => Some(*value),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
struct TransferShapeMetrics {
    curve_rmse_m: f64,
    apex_error_m: f64,
    projected_dx_abs_mean_m: Option<f64>,
    projected_dx_abs_max_m: Option<f64>,
    shortfall_ratio: Option<f64>,
}

fn transfer_shape_metrics(
    scenario: &ScenarioSpec,
    samples: &[SampleRecord],
    controller_updates: &[pd_control::ControllerUpdateRecord],
) -> Option<TransferShapeMetrics> {
    scenario.mission.transfer_route.as_ref()?;
    let target_pad = scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.id == scenario.mission.goal.target_pad_id())?;
    let first_boost = controller_updates.iter().find(|update| {
        telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("boost")
    })?;
    let start_sample = sample_at_or_after_step(samples, first_boost.physics_step)?;
    let terminal_step = controller_updates
        .iter()
        .find(|update| {
            update.physics_step >= first_boost.physics_step
                && telemetry_text(&update.frame.metrics, metric::TRANSFER_PHASE) == Some("terminal")
        })
        .map(|update| update.physics_step)
        .unwrap_or_else(|| {
            samples
                .last()
                .map(|sample| sample.physics_step)
                .unwrap_or(first_boost.physics_step)
        });
    let window_samples = samples
        .iter()
        .filter(|sample| {
            sample.physics_step >= start_sample.physics_step && sample.physics_step <= terminal_step
        })
        .collect::<Vec<_>>();
    if window_samples.len() < 2 {
        return None;
    }

    let start_x = start_sample.observation.position_m.x;
    let start_y = start_sample.observation.position_m.y;
    let target_x = target_pad.center_x_m;
    let target_y = target_pad.surface_y_m;
    let dx_anchor_m = target_x - start_x;
    let dy_anchor_m = target_y - start_y;
    if !dx_anchor_m.is_finite() || !dy_anchor_m.is_finite() {
        return None;
    }
    let apex_target_over_target_m =
        transfer_shape_apex_target_over_target_m(dx_anchor_m.abs(), dy_anchor_m);

    let mut curve_sq_err_sum = 0.0;
    let mut curve_count = 0_usize;
    let mut apex_actual_over_target_m = 0.0_f64;
    for sample in &window_samples {
        let x = sample.observation.position_m.x;
        let y = sample.observation.position_m.y;
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        let y_ref = transfer_shape_reference_y_at_x(
            x,
            start_x,
            start_y,
            target_x,
            target_y,
            apex_target_over_target_m,
        );
        let y_err = y - y_ref;
        curve_sq_err_sum += y_err * y_err;
        curve_count += 1;
        apex_actual_over_target_m = apex_actual_over_target_m.max((y - target_y).max(0.0));
    }
    if curve_count == 0 {
        return None;
    }

    let mut projected_abs_sum = 0.0;
    let mut projected_abs_max = 0.0_f64;
    let mut projected_count = 0_usize;
    let mut shortfall_count = 0_usize;
    let mut shortfall_sample_count = 0_usize;
    for update in controller_updates {
        if update.physics_step < start_sample.physics_step || update.physics_step >= terminal_step {
            continue;
        }
        if telemetry_bool(&update.frame.metrics, metric::TRANSFER_TARGET_Y_SOLUTION) == Some(false)
        {
            continue;
        }
        let Some(projected_dx_m) =
            telemetry_float(&update.frame.metrics, metric::TRANSFER_PROJECTED_DX_M)
        else {
            continue;
        };
        if !projected_dx_m.is_finite() {
            continue;
        }
        let projected_abs = projected_dx_m.abs();
        projected_abs_sum += projected_abs;
        projected_abs_max = projected_abs_max.max(projected_abs);
        projected_count += 1;

        if let Some(route_dx_m) =
            telemetry_float(&update.frame.metrics, metric::TRANSFER_ROUTE_DX_M)
            && route_dx_m.abs() > 1e-3
        {
            shortfall_sample_count += 1;
            if projected_dx_m * route_dx_m.signum() > 0.0 {
                shortfall_count += 1;
            }
        }
    }

    Some(TransferShapeMetrics {
        curve_rmse_m: (curve_sq_err_sum / curve_count as f64).sqrt(),
        apex_error_m: (apex_actual_over_target_m - apex_target_over_target_m).abs(),
        projected_dx_abs_mean_m: (projected_count > 0)
            .then_some(projected_abs_sum / projected_count as f64),
        projected_dx_abs_max_m: (projected_count > 0).then_some(projected_abs_max),
        shortfall_ratio: (shortfall_sample_count > 0)
            .then_some(shortfall_count as f64 / shortfall_sample_count as f64),
    })
}

fn transfer_shape_reference_y_at_x(
    x: f64,
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    apex_target_over_target_m: f64,
) -> f64 {
    let dx = target_x - start_x;
    if dx.abs() <= 1e-6 {
        return start_y.max(target_y);
    }
    let s = ((x - start_x) / dx).clamp(0.0, 1.0);
    let baseline = start_y + ((target_y - start_y) * s);
    baseline + (4.0 * apex_target_over_target_m * s * (1.0 - s))
}

fn transfer_shape_apex_target_over_target_m(dx_abs_m: f64, dy_m: f64) -> f64 {
    const APEX_HEIGHT_PER_DX: f64 = 0.18;
    const APEX_HEIGHT_PER_UPHILL_DY: f64 = 0.15;
    const APEX_HEIGHT_MIN_M: f64 = 30.0;
    const APEX_HEIGHT_MAX_M: f64 = 240.0;

    (APEX_HEIGHT_PER_DX * dx_abs_m).clamp(APEX_HEIGHT_MIN_M, APEX_HEIGHT_MAX_M)
        + (dy_m * APEX_HEIGHT_PER_UPHILL_DY).max(0.0)
        + (-dy_m).max(0.0)
}

#[derive(Clone, Copy, Debug)]
struct LowAltitudeRecoveryMetrics {
    low_altitude_dwell_s: f64,
    low_altitude_unsafe_recovery_s: f64,
}

fn low_altitude_recovery_metrics(
    scenario: &ScenarioSpec,
    samples: &[SampleRecord],
) -> Option<LowAltitudeRecoveryMetrics> {
    if !matches!(scenario.mission.goal, EvaluationGoal::LandingOnPad { .. }) {
        return None;
    }
    let target_pad = scenario
        .world
        .landing_pad(scenario.mission.goal.target_pad_id())?;
    let touchdown_center_limit_m =
        (target_pad.half_width_m() - scenario.vehicle.geometry.touchdown_half_span_m).max(0.0);
    let low_altitude_threshold_m = scenario
        .vehicle
        .geometry
        .touchdown_base_offset_m
        .abs()
        .max(1.0);
    let safe_tangential_speed_mps = scenario
        .vehicle
        .safe_touchdown_tangential_speed_mps
        .max(0.0);
    let mut low_altitude_dwell_s = 0.0;
    let mut low_altitude_unsafe_recovery_s = 0.0;

    for pair in samples.windows(2) {
        let current = &pair[0];
        let next = &pair[1];
        let dt_s = next.sim_time_s - current.sim_time_s;
        if !dt_s.is_finite() || dt_s <= 0.0 {
            continue;
        }
        let observation = &current.observation;
        if observation.touchdown_clearance_m >= low_altitude_threshold_m {
            continue;
        }

        low_altitude_dwell_s += dt_s;
        let laterally_unsafe = observation.target_dx_m.abs() > touchdown_center_limit_m
            || observation.velocity_mps.x.abs() > safe_tangential_speed_mps;
        if laterally_unsafe {
            low_altitude_unsafe_recovery_s += dt_s;
        }
    }

    Some(LowAltitudeRecoveryMetrics {
        low_altitude_dwell_s,
        low_altitude_unsafe_recovery_s,
    })
}

#[derive(Clone, Copy, Debug)]
struct ReferenceGapMetrics {
    gap_mean_m: f64,
    gap_max_m: f64,
}

fn reference_gap_metrics(
    scenario: &ScenarioSpec,
    samples: &[SampleRecord],
) -> Option<ReferenceGapMetrics> {
    let target_pad = scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.id == scenario.mission.goal.target_pad_id())?;
    let actual_points = samples
        .iter()
        .map(|sample| {
            (
                sample.observation.position_m.x,
                sample.observation.position_m.y,
            )
        })
        .collect::<Vec<_>>();
    if actual_points.len() < 2 {
        return None;
    }
    let reference_points = idealized_reference_curve(
        scenario.initial_state.position_m.x,
        scenario.initial_state.position_m.y,
        target_pad.center_x_m,
        target_pad.surface_y_m,
        scenario.world.gravity_mps2,
    )?;
    if reference_points.len() < 2 {
        return None;
    }

    let (actual_cumulative, actual_length) = polyline_lengths(&actual_points)?;
    let mut gaps = Vec::with_capacity(actual_points.len());
    for point in &actual_points {
        let (_projection, distance) = project_point_to_polyline(*point, &reference_points)?;
        gaps.push(distance);
    }
    if gaps.len() < 2 {
        return None;
    }

    let gap_area = (0..(gaps.len() - 1))
        .map(|index| {
            0.5 * (gaps[index] + gaps[index + 1])
                * (actual_cumulative[index + 1] - actual_cumulative[index])
        })
        .sum::<f64>();
    Some(ReferenceGapMetrics {
        gap_mean_m: gap_area / actual_length,
        gap_max_m: gaps.iter().copied().fold(0.0_f64, f64::max),
    })
}

fn idealized_reference_curve(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    gravity_mps2: f64,
) -> Option<Vec<(f64, f64)>> {
    let apex_y = idealized_reference_apex_y(start_x, start_y, target_x, target_y, gravity_mps2);
    let (flight_time, vx_mps, vy_up_mps) =
        idealized_reference_kinematics(start_x, start_y, target_x, target_y, apex_y, gravity_mps2)?;
    let g = gravity_mps2.abs().max(1e-6);
    let point_count = ((18.0 + (flight_time * 10.0)).round() as usize).clamp(24, 84);
    let mut points = Vec::with_capacity(point_count);
    for index in 0..point_count {
        let t = if point_count <= 1 {
            0.0
        } else {
            (flight_time * index as f64) / (point_count as f64 - 1.0)
        };
        points.push((
            start_x + (vx_mps * t),
            start_y + (vy_up_mps * t) - (0.5 * g * t * t),
        ));
    }
    if let Some(last) = points.last_mut() {
        *last = (target_x, target_y);
    }
    Some(points)
}

fn idealized_reference_apex_y(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    gravity_mps2: f64,
) -> f64 {
    let dx = target_x - start_x;
    let dy = target_y - start_y;
    let base_peak = if target_y > start_y {
        start_y.max(target_y + 1.0)
    } else {
        start_y
    };
    if dx.abs() <= 1e-6 {
        return base_peak;
    }
    let meets_angle_floor = |peak_y: f64| {
        idealized_reference_impact_angle_deg(
            start_x,
            start_y,
            target_x,
            target_y,
            peak_y,
            gravity_mps2,
        )
        .is_some_and(|impact_angle| impact_angle >= 45.0)
    };
    if meets_angle_floor(base_peak) {
        return base_peak;
    }

    let mut low_peak = base_peak;
    let mut growth = 16.0_f64.max(0.25 * dx.abs().max(dy.abs()).max(1.0));
    let mut candidate_peak = base_peak;
    let mut high_peak = None;
    for _ in 0..16 {
        candidate_peak += growth;
        if meets_angle_floor(candidate_peak) {
            high_peak = Some(candidate_peak);
            break;
        }
        low_peak = candidate_peak;
        growth *= 2.0;
    }
    let Some(mut high_peak) = high_peak else {
        return candidate_peak;
    };
    for _ in 0..32 {
        let mid_peak = 0.5 * (low_peak + high_peak);
        if meets_angle_floor(mid_peak) {
            high_peak = mid_peak;
        } else {
            low_peak = mid_peak;
        }
    }
    high_peak
}

fn idealized_reference_impact_angle_deg(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    apex_y: f64,
    gravity_mps2: f64,
) -> Option<f64> {
    let (flight_time, vx_mps, vy_up_mps) =
        idealized_reference_kinematics(start_x, start_y, target_x, target_y, apex_y, gravity_mps2)?;
    let g = gravity_mps2.abs().max(1e-6);
    let vy_target = vy_up_mps - (g * flight_time);
    Some(((-vy_target).max(0.0)).atan2(vx_mps.abs()) * (180.0 / std::f64::consts::PI))
}

fn idealized_reference_kinematics(
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    apex_y: f64,
    gravity_mps2: f64,
) -> Option<(f64, f64, f64)> {
    let g = gravity_mps2.abs().max(1e-6);
    let peak_y = start_y.max(apex_y);
    let vy_up_mps = (2.0 * g * (peak_y - start_y).max(0.0)).sqrt();
    let flight_time = ballistic_end_time(start_y, target_y, vy_up_mps, gravity_mps2)?;
    if !flight_time.is_finite() || flight_time <= 1e-6 {
        return None;
    }
    Some((flight_time, (target_x - start_x) / flight_time, vy_up_mps))
}

fn ballistic_end_time(start_y: f64, target_y: f64, vy_mps: f64, gravity_mps2: f64) -> Option<f64> {
    let g = gravity_mps2.abs().max(1e-6);
    let a = 0.5 * g;
    let b = -vy_mps;
    let c = target_y - start_y;
    let discriminant = (b * b) - (4.0 * a * c);
    if !discriminant.is_finite() || discriminant < 0.0 {
        return None;
    }
    let sqrt = discriminant.sqrt();
    let mut roots = [(-b - sqrt) / (2.0 * a), (-b + sqrt) / (2.0 * a)]
        .into_iter()
        .filter(|value| value.is_finite() && *value > 1e-6)
        .collect::<Vec<_>>();
    roots.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal));
    roots.pop()
}

fn polyline_lengths(points: &[(f64, f64)]) -> Option<(Vec<f64>, f64)> {
    if points.len() < 2 {
        return None;
    }
    let mut cumulative = Vec::with_capacity(points.len());
    cumulative.push(0.0);
    let mut length = 0.0;
    for index in 1..points.len() {
        let dx = points[index].0 - points[index - 1].0;
        let dy = points[index].1 - points[index - 1].1;
        length += (dx * dx + dy * dy).sqrt();
        cumulative.push(length);
    }
    (length > 1e-9).then_some((cumulative, length))
}

fn project_point_to_polyline(
    point: (f64, f64),
    polyline: &[(f64, f64)],
) -> Option<((f64, f64), f64)> {
    if polyline.is_empty() {
        return None;
    }
    let mut best_projection = polyline[0];
    let mut best_distance = point_distance(point, best_projection);
    for index in 1..polyline.len() {
        let (projection, distance) =
            project_point_to_segment(point, polyline[index - 1], polyline[index]);
        if distance < best_distance {
            best_projection = projection;
            best_distance = distance;
        }
    }
    Some((best_projection, best_distance))
}

fn project_point_to_segment(
    point: (f64, f64),
    start: (f64, f64),
    end: (f64, f64),
) -> ((f64, f64), f64) {
    let seg_dx = end.0 - start.0;
    let seg_dy = end.1 - start.1;
    let seg_len_sq = (seg_dx * seg_dx) + (seg_dy * seg_dy);
    if seg_len_sq <= 1e-12 {
        return (start, point_distance(point, start));
    }
    let mix = (((point.0 - start.0) * seg_dx) + ((point.1 - start.1) * seg_dy)) / seg_len_sq;
    let clamped_mix = mix.clamp(0.0, 1.0);
    let projection = (
        start.0 + (seg_dx * clamped_mix),
        start.1 + (seg_dy * clamped_mix),
    );
    (projection, point_distance(point, projection))
}

fn point_distance(lhs: (f64, f64), rhs: (f64, f64)) -> f64 {
    let dx = lhs.0 - rhs.0;
    let dy = lhs.1 - rhs.1;
    (dx * dx + dy * dy).sqrt()
}

fn summary_margin_ratio(summary: &RunSummary) -> Option<f64> {
    summary.envelope_margin_ratio
}

fn success_rate(success_runs: usize, total_runs: usize) -> f64 {
    if total_runs == 0 {
        0.0
    } else {
        success_runs as f64 / total_runs as f64
    }
}

fn run_pointer_sort_key(lhs: &BatchRunPointer, rhs: &BatchRunPointer) -> std::cmp::Ordering {
    lhs.entry_id
        .cmp(&rhs.entry_id)
        .then(lhs.scenario_seed.cmp(&rhs.scenario_seed))
        .then(lhs.run_id.cmp(&rhs.run_id))
}

fn closest_failure_sort_key(lhs: &BatchRunPointer, rhs: &BatchRunPointer) -> std::cmp::Ordering {
    rhs.margin_ratio
        .unwrap_or(f64::NEG_INFINITY)
        .partial_cmp(&lhs.margin_ratio.unwrap_or(f64::NEG_INFINITY))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(run_pointer_sort_key(lhs, rhs))
}

fn worst_failure_sort_key(lhs: &BatchRunPointer, rhs: &BatchRunPointer) -> std::cmp::Ordering {
    lhs.margin_ratio
        .unwrap_or(f64::INFINITY)
        .partial_cmp(&rhs.margin_ratio.unwrap_or(f64::INFINITY))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(run_pointer_sort_key(lhs, rhs))
}

fn weakest_success_sort_key(lhs: &BatchRunPointer, rhs: &BatchRunPointer) -> std::cmp::Ordering {
    lhs.margin_ratio
        .unwrap_or(f64::INFINITY)
        .partial_cmp(&rhs.margin_ratio.unwrap_or(f64::INFINITY))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(run_pointer_sort_key(lhs, rhs))
}

fn lowest_fuel_success_sort_key(
    lhs: &BatchRunPointer,
    rhs: &BatchRunPointer,
) -> std::cmp::Ordering {
    lhs.fuel_remaining_kg
        .partial_cmp(&rhs.fuel_remaining_kg)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(run_pointer_sort_key(lhs, rhs))
}

fn run_comparison_sort_key(
    lhs: &BatchRunComparison,
    rhs: &BatchRunComparison,
) -> std::cmp::Ordering {
    lhs.candidate_margin_ratio
        .unwrap_or(f64::INFINITY)
        .partial_cmp(&rhs.candidate_margin_ratio.unwrap_or(f64::INFINITY))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(
            lhs.margin_ratio_delta
                .unwrap_or(f64::INFINITY)
                .partial_cmp(&rhs.margin_ratio_delta.unwrap_or(f64::INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal),
        )
        .then(
            lhs.entry_id
                .cmp(&rhs.entry_id)
                .then(lhs.candidate_seed.cmp(&rhs.candidate_seed))
                .then(lhs.run_id.cmp(&rhs.run_id)),
        )
}

fn load_scenario(path: &Path) -> Result<ScenarioSpec> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario file {}", path.display()))?;
    let scenario: ScenarioSpec = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse scenario json {}", path.display()))?;
    scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("scenario '{}' failed validation", path.display()))?;
    Ok(scenario)
}

fn load_controller_spec(
    base_dir: &Path,
    controller_name: &str,
    controller_config_path: Option<&str>,
) -> Result<ControllerSpec> {
    if let Some(path) = controller_config_path {
        let full_path = base_dir.join(path);
        let raw = fs::read_to_string(&full_path).with_context(|| {
            format!(
                "failed to read controller config file {}",
                full_path.display()
            )
        })?;
        return serde_json::from_str(&raw).with_context(|| {
            format!(
                "failed to parse controller config json {}",
                full_path.display()
            )
        });
    }

    built_in_controller_spec(controller_name)
        .ok_or_else(|| anyhow!("unknown controller '{}'", controller_name))
}

fn write_artifact_bundle(
    path: &Path,
    scenario: &ScenarioSpec,
    controller_spec: &ControllerSpec,
    artifacts: &ControlledRunArtifacts,
) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create artifact bundle dir {}", path.display()))?;
    write_json(&path.join("scenario.json"), scenario)?;
    write_json(&path.join("controller.json"), controller_spec)?;
    write_json(
        &path.join("controller_updates.json"),
        &artifacts.controller_updates,
    )?;
    write_json(&path.join("performance.json"), &artifacts.performance)?;
    write_json(&path.join("manifest.json"), &artifacts.run.manifest)?;
    write_json(&path.join("actions.json"), &artifacts.run.actions)?;
    write_json(&path.join("events.json"), &artifacts.run.events)?;
    write_json(&path.join("samples.json"), &artifacts.run.samples)?;
    pd_report::write_run_report(
        &path.join("report.html"),
        scenario,
        Some(controller_spec),
        &artifacts.run.manifest,
        &artifacts.run.events,
        &artifacts.run.samples,
        &artifacts.controller_updates,
        Some(&artifacts.performance),
    )?;
    pd_report::write_run_preview_svg(
        &path.join("preview.svg"),
        scenario,
        &artifacts.run.manifest,
        &artifacts.run.samples,
        &artifacts.controller_updates,
    )?;
    Ok(())
}

fn batch_identity_for_pack(
    pack: &ScenarioPackSpec,
    resolved_runs: &[ResolvedBatchRun],
) -> Result<BatchIdentity> {
    Ok(BatchIdentity {
        schema_version: BATCH_REPORT_SCHEMA_VERSION,
        pack_spec_digest: stable_digest(pack)?,
        resolved_run_digest: stable_digest(
            &resolved_runs
                .iter()
                .map(|run| &run.descriptor)
                .collect::<Vec<_>>(),
        )?,
    })
}

fn batch_cache_stem(pack_id: &str, identity: &BatchIdentity) -> String {
    format!(
        "{}__spec_{}__runs_{}",
        sanitize_token(pack_id),
        short_digest(&identity.pack_spec_digest),
        short_digest(&identity.resolved_run_digest),
    )
}

fn cache_dir_for_batch_key(workspace_key: &str, batch_stem: &str) -> PathBuf {
    eval_cache_root().join(workspace_key).join(batch_stem)
}

fn eval_cache_root() -> PathBuf {
    repo_root().join("outputs").join("eval").join("cache")
}

fn current_workspace_state() -> Result<WorkspaceState> {
    let commit_key = git_commit_key_for_ref("HEAD")?;
    let status_output = git_stdout(&["status", "--porcelain=v1", "--untracked-files=normal"])?;
    let dirty = !status_output.trim().is_empty();
    let workspace_key = if dirty {
        format!(
            "{}-dirty-{}",
            commit_key,
            short_bytes_digest(status_output.as_bytes())
        )
    } else {
        commit_key.clone()
    };
    Ok(WorkspaceState {
        commit_key,
        workspace_key,
        dirty,
    })
}

fn git_commit_key_for_ref(reference: &str) -> Result<String> {
    let resolved = git_stdout(&["rev-parse", "--short=12", reference])?;
    let key = resolved.trim();
    if key.is_empty() {
        bail!("git rev-parse produced empty commit key for {}", reference);
    }
    Ok(key.to_owned())
}

fn git_stdout(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_root())
        .args(args)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn short_bytes_digest(bytes: &[u8]) -> String {
    format!("{:08x}", fnv1a64(bytes))
}

fn short_digest(value: &str) -> String {
    value.chars().take(8).collect()
}

fn resolve_compare_provenance(
    baseline_dir: Option<&Path>,
    compare_ref: Option<&str>,
    missing_compare: MissingComparePolicy,
    workspace: &WorkspaceState,
) -> Result<BatchCompareProvenance> {
    if let Some(baseline_dir) = baseline_dir {
        return Ok(BatchCompareProvenance {
            source: BatchCompareSource::ExplicitDir,
            requested_ref: None,
            resolved_ref: None,
            baseline_dir: Some(baseline_dir.to_string_lossy().into_owned()),
            status: BatchCompareResolutionStatus::Resolved,
            note: Some("explicit baseline report directory".to_owned()),
        });
    }

    let requested_ref = compare_ref.unwrap_or("auto");
    if requested_ref == "none" {
        return Ok(BatchCompareProvenance::default());
    }

    let resolved_ref = if requested_ref == "auto" {
        if workspace.dirty {
            Some(workspace.commit_key.clone())
        } else {
            git_commit_key_for_ref("HEAD^").ok()
        }
    } else {
        Some(git_commit_key_for_ref(requested_ref)?)
    };

    let Some(resolved_ref) = resolved_ref else {
        return Ok(BatchCompareProvenance {
            source: BatchCompareSource::CacheRef,
            requested_ref: Some(requested_ref.to_owned()),
            resolved_ref: None,
            baseline_dir: None,
            status: BatchCompareResolutionStatus::Missing,
            note: Some(match missing_compare {
                MissingComparePolicy::Skip => {
                    "no compare cache ref could be resolved; continuing without external compare"
                        .to_owned()
                }
                MissingComparePolicy::Error => "no compare cache ref could be resolved".to_owned(),
            }),
        });
    };

    Ok(BatchCompareProvenance {
        source: BatchCompareSource::CacheRef,
        requested_ref: Some(requested_ref.to_owned()),
        resolved_ref: Some(resolved_ref.clone()),
        baseline_dir: Some(
            eval_cache_root()
                .join(&resolved_ref)
                .to_string_lossy()
                .into_owned(),
        ),
        status: BatchCompareResolutionStatus::NotRequested,
        note: Some(if requested_ref == "auto" && workspace.dirty {
            "auto compare requested; using clean HEAD cache".to_owned()
        } else if requested_ref == "auto" {
            "auto compare requested; using previous clean commit cache".to_owned()
        } else {
            "explicit compare cache ref".to_owned()
        }),
    })
}

fn load_requested_baseline(
    pack: &ScenarioPackSpec,
    identity: &BatchIdentity,
    mut provenance: BatchCompareProvenance,
    missing_compare: MissingComparePolicy,
) -> Result<(BatchCompareProvenance, Option<ResolvedBaselineReport>)> {
    match provenance.source {
        BatchCompareSource::None => Ok((provenance, None)),
        BatchCompareSource::ExplicitDir => {
            let baseline_dir = provenance
                .baseline_dir
                .as_deref()
                .ok_or_else(|| anyhow!("explicit baseline compare is missing baseline_dir"))?;
            let dir = PathBuf::from(baseline_dir);
            provenance.status = BatchCompareResolutionStatus::Resolved;
            Ok((
                provenance,
                Some(ResolvedBaselineReport {
                    report: load_batch_report(&dir)?,
                    dir,
                }),
            ))
        }
        BatchCompareSource::CacheRef => {
            if provenance.resolved_ref.is_none() {
                return if missing_compare == MissingComparePolicy::Skip {
                    provenance.status = BatchCompareResolutionStatus::Missing;
                    Ok((provenance, None))
                } else {
                    bail!("cache compare is missing resolved_ref")
                };
            }
            let resolved_ref = provenance
                .resolved_ref
                .as_deref()
                .expect("resolved_ref handled above");
            let batch_stem = batch_cache_stem(&pack.id, identity);
            let baseline_dir = cache_dir_for_batch_key(resolved_ref, &batch_stem);
            if let Some(report) = validate_cached_batch_dir(&baseline_dir, pack, identity)? {
                provenance.status = BatchCompareResolutionStatus::Resolved;
                provenance.baseline_dir = Some(baseline_dir.to_string_lossy().into_owned());
                Ok((
                    provenance,
                    Some(ResolvedBaselineReport {
                        dir: baseline_dir,
                        report,
                    }),
                ))
            } else if missing_compare == MissingComparePolicy::Skip {
                provenance.status = BatchCompareResolutionStatus::Missing;
                provenance.baseline_dir = Some(baseline_dir.to_string_lossy().into_owned());
                provenance.note = Some(format!(
                    "no compare cache found for ref '{}' at {}; continuing without external compare",
                    resolved_ref,
                    baseline_dir.display()
                ));
                Ok((provenance, None))
            } else {
                bail!(
                    "no compare cache found for ref '{}' at {}",
                    resolved_ref,
                    baseline_dir.display()
                )
            }
        }
    }
}

fn validate_cached_batch_dir(
    cache_dir: &Path,
    pack: &ScenarioPackSpec,
    identity: &BatchIdentity,
) -> Result<Option<BatchReport>> {
    let required_files = [
        cache_dir.join("pack.json"),
        cache_dir.join("resolved_runs.json"),
        cache_dir.join("summary.json"),
        cache_dir.join("meta.json"),
        cache_dir.join("report.html"),
    ];
    if required_files.iter().any(|path| !path.exists()) {
        return Ok(None);
    }

    let Ok(meta) = read_json::<BatchCacheMeta>(&cache_dir.join("meta.json")) else {
        return Ok(None);
    };
    if meta.schema_version != BATCH_REPORT_SCHEMA_VERSION
        || meta.identity.schema_version != BATCH_REPORT_SCHEMA_VERSION
        || meta.pack_id != pack.id
        || meta.identity.pack_spec_digest != identity.pack_spec_digest
        || meta.identity.resolved_run_digest != identity.resolved_run_digest
    {
        return Ok(None);
    }

    let Ok(report) = load_batch_report(cache_dir) else {
        return Ok(None);
    };
    if report.schema_version != BATCH_REPORT_SCHEMA_VERSION
        || report.identity.schema_version != BATCH_REPORT_SCHEMA_VERSION
        || report.pack_id != pack.id
        || report.identity.pack_spec_digest != identity.pack_spec_digest
        || report.identity.resolved_run_digest != identity.resolved_run_digest
        || report.records.len() != report.resolved_runs.len()
    {
        return Ok(None);
    }
    if !validate_cached_run_bundles(&report.records) {
        return Ok(None);
    }
    Ok(Some(report))
}

fn validate_cached_run_bundles(records: &[BatchRunRecord]) -> bool {
    const REQUIRED_BUNDLE_FILES: [&str; 10] = [
        "scenario.json",
        "controller.json",
        "controller_updates.json",
        "performance.json",
        "manifest.json",
        "actions.json",
        "events.json",
        "samples.json",
        "report.html",
        "preview.svg",
    ];

    records.iter().all(|record| {
        let Some(bundle_dir) = record.bundle_dir.as_deref() else {
            return false;
        };
        let bundle_dir = if Path::new(bundle_dir).is_absolute() {
            PathBuf::from(bundle_dir)
        } else {
            repo_root().join(bundle_dir)
        };
        REQUIRED_BUNDLE_FILES
            .iter()
            .all(|name| bundle_dir.join(name).exists())
    })
}

fn write_batch_cache_dir(
    output_dir: &Path,
    pack: &ScenarioPackSpec,
    report: &BatchReport,
    update_latest: bool,
) -> Result<()> {
    write_batch_manifest_files(output_dir, pack, report)?;
    let cache = report
        .provenance
        .cache
        .clone()
        .ok_or_else(|| anyhow!("cannot write cache metadata without cache provenance"))?;
    write_json(
        &output_dir.join("meta.json"),
        &BatchCacheMeta {
            schema_version: BATCH_REPORT_SCHEMA_VERSION,
            pack_id: report.pack_id.clone(),
            pack_name: report.pack_name.clone(),
            identity: report.identity.clone(),
            total_runs: report.total_runs,
            workers_used: report.workers_used,
            cache,
        },
    )?;
    report::write_batch_report_artifacts(output_dir, report, None)?;
    if update_latest {
        maybe_update_latest_link(output_dir)?;
        if let Some(last_record) = report.records.last()
            && let Some(bundle_dir) = last_record.bundle_dir.as_deref()
        {
            maybe_update_latest_link(Path::new(bundle_dir))?;
        }
    }
    Ok(())
}

fn write_batch_output_dir(
    output_dir: &Path,
    pack: &ScenarioPackSpec,
    report: &BatchReport,
    baseline: Option<(&Path, &BatchReport)>,
) -> Result<()> {
    sync_output_run_bundles(output_dir, report)?;
    let localized_report = localize_report_bundle_dirs(report, output_dir);
    write_batch_manifest_files(output_dir, pack, &localized_report)?;
    report::write_batch_report_artifacts(output_dir, &localized_report, baseline)?;
    maybe_update_latest_link(output_dir)?;
    if let Some(last_record) = localized_report.records.last()
        && let Some(bundle_dir) = last_record.bundle_dir.as_deref()
    {
        maybe_update_latest_link(Path::new(bundle_dir))?;
    }
    Ok(())
}

fn write_batch_manifest_files(
    output_dir: &Path,
    pack: &ScenarioPackSpec,
    report: &BatchReport,
) -> Result<()> {
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create batch eval output directory {}",
            output_dir.display()
        )
    })?;
    write_json(&output_dir.join("pack.json"), pack)?;
    write_json(
        &output_dir.join("resolved_runs.json"),
        &report.resolved_runs,
    )?;
    write_json(&output_dir.join("summary.json"), report)?;
    Ok(())
}

fn rewrite_report_bundle_dirs(report: &mut BatchReport, source_dir: &Path, target_dir: &Path) {
    for record in &mut report.records {
        if let Some(bundle_dir) = record.bundle_dir.as_deref() {
            record.bundle_dir = Some(rewrite_dir_string(bundle_dir, source_dir, target_dir));
        }
    }
}

fn localize_report_bundle_dirs(report: &BatchReport, output_dir: &Path) -> BatchReport {
    let mut localized = report.clone();
    let runs_dir = output_dir.join("runs");
    for record in &mut localized.records {
        if record.bundle_dir.is_some() {
            record.bundle_dir = Some(
                runs_dir
                    .join(&record.resolved.run_id)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    localized
}

fn sync_output_run_bundles(output_dir: &Path, report: &BatchReport) -> Result<()> {
    let runs_dir = output_dir.join("runs");
    remove_path_if_exists(&runs_dir)?;
    let bundle_records = report
        .records
        .iter()
        .filter_map(|record| {
            record
                .bundle_dir
                .as_deref()
                .map(|bundle_dir| (record.resolved.run_id.as_str(), PathBuf::from(bundle_dir)))
        })
        .collect::<Vec<_>>();
    if bundle_records.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(&runs_dir)
        .with_context(|| format!("failed to create runs directory {}", runs_dir.display()))?;
    for (run_id, bundle_dir) in bundle_records {
        let target = runs_dir.join(run_id);
        platform_fs::symlink(&bundle_dir, &target).with_context(|| {
            format!(
                "failed to link stable output run {} -> {}",
                target.display(),
                bundle_dir.display()
            )
        })?;
    }
    Ok(())
}

fn remove_path_if_exists(path: &Path) -> Result<()> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)
            .with_context(|| format!("failed to remove path {}", path.display()))?;
    } else if metadata.is_dir() {
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove directory {}", path.display()))?;
    }
    Ok(())
}

fn rewrite_dir_string(path_str: &str, source_dir: &Path, target_dir: &Path) -> String {
    let path = Path::new(path_str);
    if let Ok(relative) = path.strip_prefix(source_dir) {
        target_dir.join(relative).to_string_lossy().into_owned()
    } else {
        path_str.to_owned()
    }
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)
        .with_context(|| format!("failed to create directory {}", target.display()))?;
    for entry in fs::read_dir(source)
        .with_context(|| format!("failed to read directory {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory {}", parent.display()))?;
            }
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn find_latest_dirty_workspace_key(commit_key: &str, batch_stem: &str) -> Result<Option<String>> {
    let mut candidates = Vec::<(u64, String)>::new();
    let cache_root = eval_cache_root();
    if !cache_root.exists() {
        return Ok(None);
    }
    for entry in fs::read_dir(&cache_root)
        .with_context(|| format!("failed to read cache root {}", cache_root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let workspace_key = entry.file_name().to_string_lossy().into_owned();
        if !workspace_key.starts_with(&format!("{commit_key}-dirty-")) {
            continue;
        }
        let meta_path = entry.path().join(batch_stem).join("meta.json");
        if !meta_path.exists() {
            continue;
        }
        let Ok(meta) = read_json::<BatchCacheMeta>(&meta_path) else {
            continue;
        };
        candidates.push((meta.cache.created_at_unix_s, workspace_key));
    }
    candidates.sort_by(|lhs, rhs| lhs.cmp(rhs));
    Ok(candidates.pop().map(|(_, key)| key))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read json file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse json file {}", path.display()))
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    let raw = serde_json::to_string_pretty(value)?;
    fs::write(path, raw)
        .with_context(|| format!("failed to write json file {}", path.display()))?;
    Ok(())
}

fn enum_label<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"unknown\"".to_owned())
        .trim_matches('"')
        .to_owned()
}

fn effective_worker_count(requested_workers: usize, total_runs: usize) -> usize {
    if total_runs == 0 {
        return 1;
    }
    requested_workers.max(1).min(total_runs)
}

fn stable_digest<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    Ok(format!("{:012x}", fnv1a64(&bytes)))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn stable_unit_interval(seed: u64, salt: &str) -> f64 {
    let mixed = splitmix64(seed ^ fnv1a64(salt.as_bytes()));
    let mantissa = mixed >> 11;
    (mantissa as f64) / ((1_u64 << 53) as f64)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = value;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn sanitize_token(token: &str) -> String {
    let mut out = String::with_capacity(token.len());
    let mut last_was_sep = false;
    for ch in token.chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            out.push(normalized);
            last_was_sep = false;
        } else if !last_was_sep {
            out.push('_');
            last_was_sep = true;
        }
    }
    out.trim_matches('_').to_owned()
}

fn resolved_family_run_id(entry_id: &str, seed: u64) -> String {
    sanitize_token(&format!("{entry_id}__seed_{seed:04}"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pd-eval crate should live under repo root")
        .to_path_buf()
}

fn maybe_update_latest_link(target_dir: &Path) -> Result<()> {
    let repo_root = repo_root();
    let outputs_root = repo_root.join("outputs");
    let resolved_target_dir = if target_dir.is_absolute() {
        target_dir.to_path_buf()
    } else {
        repo_root.join(target_dir)
    };

    if !resolved_target_dir.starts_with(&outputs_root) {
        return Ok(());
    }

    let Some(parent_dir) = resolved_target_dir.parent() else {
        return Ok(());
    };
    let Some(target_name) = resolved_target_dir.file_name() else {
        return Ok(());
    };

    let latest_path = parent_dir.join("latest");
    if let Ok(metadata) = fs::symlink_metadata(&latest_path) {
        if metadata.file_type().is_symlink() || metadata.is_file() {
            fs::remove_file(&latest_path).with_context(|| {
                format!(
                    "failed to remove existing latest link {}",
                    latest_path.display()
                )
            })?;
        } else {
            eprintln!(
                "skipping latest link update because '{}' exists and is not a symlink",
                latest_path.display()
            );
            return Ok(());
        }
    }

    platform_fs::symlink(PathBuf::from(target_name), &latest_path).with_context(|| {
        format!(
            "failed to create latest link {} -> {}",
            latest_path.display(),
            target_name.to_string_lossy()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use pd_core::{
        EvaluationGoal, LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig, TerrainDefinition,
        Vec2, VehicleGeometry, VehicleInitialState, VehicleSpec, WorldSpec,
    };

    fn fixtures_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures")
    }

    fn temp_fixture_root(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pd_eval_{prefix}_{unique}"));
        fs::create_dir_all(root.join("scenarios")).expect("temp fixture root should be creatable");
        root
    }

    fn write_scenario(root: &Path, relative_path: &str, scenario: &ScenarioSpec) {
        let path = root.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("scenario parent directory should be creatable");
        }
        fs::write(
            &path,
            serde_json::to_vec_pretty(scenario).expect("scenario json should serialize"),
        )
        .expect("scenario json should be writable");
    }

    fn report_with_records(mut report: BatchReport, records: Vec<BatchRunRecord>) -> BatchReport {
        report.total_runs = records.len();
        report.resolved_runs = records
            .iter()
            .map(|record| record.resolved.clone())
            .collect();
        report.summary = summarize_records(&records);
        report.records = records;
        report
    }

    fn easy_landing_scenario() -> ScenarioSpec {
        ScenarioSpec {
            id: "unit_flat_landing".to_owned(),
            name: "Unit flat landing".to_owned(),
            description: "Low-gravity flat landing fixture for eval tests".to_owned(),
            seed: 3,
            tags: vec!["test".to_owned(), "landing".to_owned()],
            metadata: BTreeMap::from([("suite".to_owned(), "eval".to_owned())]),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 45.0,
                sample_hz: Some(10),
            },
            world: WorldSpec {
                gravity_mps2: 1.62,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-120.0, 0.0), Vec2::new(120.0, 0.0)],
                },
                landing_pads: vec![LandingPadSpec {
                    id: "pad_a".to_owned(),
                    center_x_m: 0.0,
                    surface_y_m: 0.0,
                    width_m: 36.0,
                }],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 4.0,
                    hull_height_m: 6.0,
                    touchdown_half_span_m: 2.0,
                    touchdown_base_offset_m: 3.2,
                },
                dry_mass_kg: 700.0,
                initial_fuel_kg: 240.0,
                max_fuel_kg: 240.0,
                max_thrust_n: 16_000.0,
                max_fuel_burn_kgps: 11.0,
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.2,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(18.0, 140.0),
                velocity_mps: Vec2::new(-1.0, -12.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                transfer_route: None,
                goal: EvaluationGoal::LandingOnPad {
                    target_pad_id: "pad_a".to_owned(),
                },
            },
        }
    }

    fn waypoint_contract_scenario() -> ScenarioSpec {
        let mut scenario = easy_landing_scenario();
        scenario.world.landing_pads.push(LandingPadSpec {
            id: "source".to_owned(),
            center_x_m: -420.0,
            surface_y_m: 120.0,
            width_m: 36.0,
        });
        scenario.mission.transfer_route = Some(TransferRouteSpec {
            source_pad_id: "source".to_owned(),
            target_pad_id: "pad_a".to_owned(),
            route_angle_deg: 80.0,
            route_radius_m: 800.0,
            waypoints: vec![TransferWaypointSpec {
                id: "wp_0".to_owned(),
                position_m: Vec2::new(-220.0, 180.0),
                handoff_tangent_unit: None,
                capture_radius_m: 40.0,
                max_cross_track_m: 50.0,
                max_outbound_heading_error_rad: 0.85,
                min_outbound_progress_mps: 8.0,
                max_outbound_cross_speed_mps: None,
                min_speed_mps: 10.0,
                max_speed_mps: 130.0,
                min_vertical_speed_mps: Some(-80.0),
                max_vertical_speed_mps: Some(65.0),
            }],
        });
        scenario
    }

    fn easy_checkpoint_scenario() -> ScenarioSpec {
        ScenarioSpec {
            id: "unit_timed_checkpoint".to_owned(),
            name: "Unit timed checkpoint".to_owned(),
            description: "Stationary timed checkpoint fixture for eval tests".to_owned(),
            seed: 5,
            tags: vec!["test".to_owned(), "checkpoint".to_owned()],
            metadata: BTreeMap::from([("suite".to_owned(), "eval".to_owned())]),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 5.0,
                sample_hz: Some(10),
            },
            world: WorldSpec {
                gravity_mps2: 1.62,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-80.0, 0.0), Vec2::new(80.0, 0.0)],
                },
                landing_pads: vec![LandingPadSpec {
                    id: "pad_a".to_owned(),
                    center_x_m: 0.0,
                    surface_y_m: 0.0,
                    width_m: 36.0,
                }],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 4.0,
                    hull_height_m: 6.0,
                    touchdown_half_span_m: 2.0,
                    touchdown_base_offset_m: 3.2,
                },
                dry_mass_kg: 700.0,
                initial_fuel_kg: 40.0,
                max_fuel_kg: 40.0,
                max_thrust_n: 16_000.0,
                max_fuel_burn_kgps: 11.0,
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.2,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(0.0, 10.0),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                transfer_route: None,
                goal: EvaluationGoal::TimedCheckpoint {
                    target_pad_id: "pad_a".to_owned(),
                    end_time_s: 0.5,
                    desired_position_offset_m: Vec2::new(0.0, 9.794125),
                    max_position_error_m: 0.01,
                    desired_velocity_mps: Vec2::new(0.0, -0.81),
                    max_velocity_error_mps: 0.01,
                    max_attitude_error_rad: 0.01,
                },
            },
        }
    }

    #[test]
    fn run_pack_aggregates_nominal_suite() {
        let base_dir = temp_fixture_root("unit_pack");
        write_scenario(
            &base_dir,
            "scenarios/checkpoint_success.json",
            &easy_checkpoint_scenario(),
        );
        let pack = ScenarioPackSpec {
            id: "unit_pack".to_owned(),
            name: "Unit pack".to_owned(),
            description: "unit pack".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![
                ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                    id: "checkpoint_success_baseline".to_owned(),
                    scenario: "scenarios/checkpoint_success.json".to_owned(),
                    controller: "baseline".to_owned(),
                    controller_config: None,
                    metadata: BTreeMap::new(),
                }),
                ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                    id: "checkpoint_success_idle".to_owned(),
                    scenario: "scenarios/checkpoint_success.json".to_owned(),
                    controller: "idle".to_owned(),
                    controller_config: None,
                    metadata: BTreeMap::new(),
                }),
            ],
        };

        let report = run_pack(&pack, &base_dir, None).unwrap();

        assert_eq!(report.total_runs, 2);
        assert_eq!(report.summary.success_runs, 2);
        assert_eq!(
            report.summary.mission_outcomes.get("success").copied(),
            Some(2)
        );
        assert_eq!(report.identity.schema_version, BATCH_REPORT_SCHEMA_VERSION);
    }

    #[test]
    fn family_entry_expands_deterministically_across_workers() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "family_pack".to_owned(),
            name: "Family pack".to_owned(),
            description: "family pack".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![
                ScenarioPackEntry::Family(ScenarioFamilyEntry {
                    id: "terminal_sweep".to_owned(),
                    family: "terminal_nominal".to_owned(),
                    base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    controller: "baseline".to_owned(),
                    controller_config: None,
                    seeds: vec![0, 1, 2],
                    seed_range: None,
                    perturbations: vec![
                        NumericPerturbationSpec {
                            id: "spawn_dx".to_owned(),
                            path: "initial_state.position_m.x".to_owned(),
                            mode: NumericPerturbationMode::Offset,
                            min: -12.0,
                            max: 12.0,
                            quantize: Some(0.5),
                        },
                        NumericPerturbationSpec {
                            id: "spawn_vy".to_owned(),
                            path: "initial_state.velocity_mps.y".to_owned(),
                            mode: NumericPerturbationMode::Offset,
                            min: -2.0,
                            max: 2.0,
                            quantize: Some(0.25),
                        },
                    ],
                    tags: vec!["sweep".to_owned()],
                    metadata: BTreeMap::from([
                        ("difficulty".to_owned(), "sweep".to_owned()),
                        ("mission".to_owned(), "terminal_guidance".to_owned()),
                        (
                            "arrival_family".to_owned(),
                            "seeded_terminal_arrival_v0".to_owned(),
                        ),
                        ("condition_set".to_owned(), "clean".to_owned()),
                        ("vehicle_variant".to_owned(), "nominal".to_owned()),
                        ("expectation_tier".to_owned(), "core".to_owned()),
                    ]),
                }),
                ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                    id: "checkpoint".to_owned(),
                    scenario: "scenarios/timed_checkpoint_idle.json".to_owned(),
                    controller: "idle".to_owned(),
                    controller_config: None,
                    metadata: BTreeMap::new(),
                }),
            ],
        };

        let sequential = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
        let parallel = run_pack_with_workers(&pack, &base_dir, None, 2).unwrap();

        assert_eq!(sequential.total_runs, 4);
        assert_eq!(
            sequential.identity.resolved_run_digest,
            parallel.identity.resolved_run_digest
        );
        assert_eq!(
            sequential
                .records
                .iter()
                .map(|record| record.resolved.run_id.clone())
                .collect::<Vec<_>>(),
            parallel
                .records
                .iter()
                .map(|record| record.resolved.run_id.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            sequential
                .summary
                .by_entry
                .iter()
                .find(|group| group.key == "terminal_sweep")
                .map(|group| group.total_runs),
            Some(3)
        );
        assert!(sequential.records.iter().any(|record| {
            record.resolved.source_kind == ResolvedRunSourceKind::FamilySweep
                && !record.resolved.resolved_parameters.is_empty()
        }));
        let family_record = sequential
            .records
            .iter()
            .find(|record| record.resolved.entry_id == "terminal_sweep")
            .expect("family record present");
        assert_eq!(family_record.resolved.selector.mission, "terminal_guidance");
        assert_eq!(
            family_record.resolved.selector.arrival_family,
            "seeded_terminal_arrival_v0"
        );
        assert_eq!(family_record.resolved.selector.condition_set, "clean");
        assert_eq!(family_record.resolved.selector.vehicle_variant, "nominal");
        assert_eq!(
            family_record.resolved.selector.expectation_tier.as_deref(),
            Some("core")
        );
        assert_eq!(family_record.resolved.lane_id, "baseline");
        let pointer = run_pointer(family_record);
        assert_eq!(pointer.selector.vehicle_variant, "nominal");
        assert_eq!(pointer.lane_id, "baseline");
    }

    #[test]
    fn compare_reports_flags_regressions_on_shared_runs() {
        let base_dir = temp_fixture_root("compare_pack");
        write_scenario(
            &base_dir,
            "scenarios/landing_case.json",
            &easy_landing_scenario(),
        );
        let baseline_pack = ScenarioPackSpec {
            id: "compare_baseline".to_owned(),
            name: "Compare baseline".to_owned(),
            description: "compare baseline".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "landing_case".to_owned(),
                scenario: "scenarios/landing_case.json".to_owned(),
                controller: "baseline".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            })],
        };
        let candidate_pack = ScenarioPackSpec {
            id: "compare_candidate".to_owned(),
            name: "Compare candidate".to_owned(),
            description: "compare candidate".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "landing_case".to_owned(),
                scenario: "scenarios/landing_case.json".to_owned(),
                controller: "idle".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            })],
        };

        let baseline = run_pack(&baseline_pack, &base_dir, None).unwrap();
        let candidate = run_pack(&candidate_pack, &base_dir, None).unwrap();
        let comparison = compare_batch_reports(&candidate, &baseline);

        assert_eq!(comparison.basis.shared_runs, 1);
        assert_eq!(comparison.regressions.len(), 1);
        assert!(comparison.improvements.is_empty());
        assert_eq!(comparison.summary.failure_runs_delta, 1);
        assert_eq!(comparison.policy.status, BatchRegressionPolicyStatus::Fail);
        assert_eq!(
            comparison
                .policy
                .rules
                .iter()
                .find(|rule| rule.id == "new_failures")
                .expect("new failure policy rule should exist")
                .status,
            BatchRegressionPolicyStatus::Fail
        );
        assert_eq!(comparison.regressions[0].run_id, "landing_case");
        assert_eq!(
            comparison.regressions[0].baseline_mission_outcome,
            "success"
        );
        assert_eq!(
            comparison.regressions[0].candidate_mission_outcome,
            "failed_crash"
        );
    }

    #[test]
    fn compare_reports_passes_policy_for_identical_reports() {
        let base_dir = temp_fixture_root("compare_identical");
        write_scenario(
            &base_dir,
            "scenarios/landing_case.json",
            &easy_landing_scenario(),
        );
        let pack = ScenarioPackSpec {
            id: "compare_identical".to_owned(),
            name: "Compare identical".to_owned(),
            description: "compare identical".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "landing_case".to_owned(),
                scenario: "scenarios/landing_case.json".to_owned(),
                controller: "baseline".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            })],
        };

        let report = run_pack(&pack, &base_dir, None).unwrap();
        let comparison = compare_batch_reports(&report, &report);

        assert_eq!(comparison.policy.status, BatchRegressionPolicyStatus::Pass);
        assert!(comparison.regressions.is_empty());
        assert!(
            comparison
                .policy
                .summary
                .contains("passed all regression thresholds")
        );
    }

    #[test]
    fn compare_policy_warns_on_aggregate_deltas_when_coverage_differs() {
        let base_dir = temp_fixture_root("compare_coverage");
        write_scenario(
            &base_dir,
            "scenarios/landing_case.json",
            &easy_landing_scenario(),
        );
        let baseline_pack = ScenarioPackSpec {
            id: "compare_coverage_baseline".to_owned(),
            name: "Compare coverage baseline".to_owned(),
            description: "compare coverage baseline".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "landing_case".to_owned(),
                scenario: "scenarios/landing_case.json".to_owned(),
                controller: "baseline".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            })],
        };
        let candidate_pack = ScenarioPackSpec {
            id: "compare_coverage_candidate".to_owned(),
            name: "Compare coverage candidate".to_owned(),
            description: "compare coverage candidate".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![
                ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                    id: "landing_case".to_owned(),
                    scenario: "scenarios/landing_case.json".to_owned(),
                    controller: "baseline".to_owned(),
                    controller_config: None,
                    metadata: BTreeMap::new(),
                }),
                ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                    id: "extra_case".to_owned(),
                    scenario: "scenarios/landing_case.json".to_owned(),
                    controller: "idle".to_owned(),
                    controller_config: None,
                    metadata: BTreeMap::new(),
                }),
            ],
        };

        let baseline = run_pack(&baseline_pack, &base_dir, None).unwrap();
        let candidate = run_pack(&candidate_pack, &base_dir, None).unwrap();
        let comparison = compare_batch_reports(&candidate, &baseline);

        assert_eq!(comparison.basis.candidate_only_runs, 1);
        assert_eq!(comparison.summary.failure_runs_delta, 1);
        assert!(comparison.regressions.is_empty());
        assert_eq!(comparison.policy.status, BatchRegressionPolicyStatus::Warn);
        assert_eq!(
            comparison
                .policy
                .rules
                .iter()
                .find(|rule| rule.id == "scored_failure_delta")
                .expect("scored failure policy rule should exist")
                .status,
            BatchRegressionPolicyStatus::Warn
        );
    }

    #[test]
    fn compare_reports_prefer_current_lane_policy_scope() {
        let base_dir = temp_fixture_root("compare_current_lane_scope");
        write_scenario(
            &base_dir,
            "scenarios/landing_case.json",
            &easy_landing_scenario(),
        );
        let baseline_pack = ScenarioPackSpec {
            id: "compare_current_lane_baseline".to_owned(),
            name: "Compare current lane baseline".to_owned(),
            description: "compare current lane baseline".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "landing_case".to_owned(),
                scenario: "scenarios/landing_case.json".to_owned(),
                controller: "baseline".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            })],
        };
        let candidate_pack = ScenarioPackSpec {
            id: "compare_current_lane_candidate".to_owned(),
            name: "Compare current lane candidate".to_owned(),
            description: "compare current lane candidate".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "landing_case".to_owned(),
                scenario: "scenarios/landing_case.json".to_owned(),
                controller: "idle".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            })],
        };
        let baseline_single = run_pack(&baseline_pack, &base_dir, None).unwrap();
        let candidate_single = run_pack(&candidate_pack, &base_dir, None).unwrap();

        let mut current_success = baseline_single.records[0].clone();
        current_success.resolved.run_id = "landing_case__current".to_owned();
        current_success.resolved.lane_id = "current".to_owned();

        let mut hidden_baseline_success = baseline_single.records[0].clone();
        hidden_baseline_success.resolved.run_id = "landing_case__baseline".to_owned();
        hidden_baseline_success.resolved.lane_id = "baseline".to_owned();

        let mut hidden_candidate_failure = candidate_single.records[0].clone();
        hidden_candidate_failure.resolved.run_id = "landing_case__baseline".to_owned();
        hidden_candidate_failure.resolved.lane_id = "baseline".to_owned();

        let baseline = report_with_records(
            baseline_single.clone(),
            vec![current_success.clone(), hidden_baseline_success],
        );
        let candidate = report_with_records(
            candidate_single,
            vec![current_success, hidden_candidate_failure],
        );
        let comparison = compare_batch_reports(&candidate, &baseline);

        assert_eq!(comparison.basis.shared_runs, 1);
        assert_eq!(comparison.summary.failure_runs_delta, 0);
        assert!(comparison.regressions.is_empty());
        assert_eq!(comparison.policy.status, BatchRegressionPolicyStatus::Pass);
    }

    #[test]
    fn concrete_entry_metadata_overrides_selector_axes() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "concrete_metadata_override".to_owned(),
            name: "Concrete metadata override".to_owned(),
            description: "selector metadata override".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "checkpoint".to_owned(),
                scenario: "scenarios/timed_checkpoint_idle.json".to_owned(),
                controller: "idle".to_owned(),
                controller_config: None,
                metadata: BTreeMap::from([
                    ("mission".to_owned(), "terminal_guidance".to_owned()),
                    (
                        "arrival_family".to_owned(),
                        "override_arrival_family".to_owned(),
                    ),
                    ("condition_set".to_owned(), "stress".to_owned()),
                    ("vehicle_variant".to_owned(), "heavy_cargo".to_owned()),
                    ("expectation_tier".to_owned(), "frontier".to_owned()),
                ]),
            })],
        };

        let report = run_pack(&pack, &base_dir, None).unwrap();
        let record = report
            .records
            .iter()
            .find(|record| record.resolved.entry_id == "checkpoint")
            .expect("concrete record present");
        assert_eq!(record.resolved.selector.mission, "terminal_guidance");
        assert_eq!(
            record.resolved.selector.arrival_family,
            "override_arrival_family"
        );
        assert_eq!(record.resolved.selector.condition_set, "stress");
        assert_eq!(record.resolved.selector.vehicle_variant, "heavy_cargo");
        assert_eq!(
            record.resolved.selector.expectation_tier.as_deref(),
            Some("frontier")
        );
    }

    #[test]
    fn terminal_matrix_entry_expands_documented_smoke_axes() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_smoke".to_owned(),
            name: "Terminal matrix smoke".to_owned(),
            description: "terminal matrix smoke".to_owned(),
            terminal_matrix_max_time_s: Some(90.0),
            entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_clean_nominal".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "staged".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "clean".to_owned(),
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                arc_points: Vec::new(),
                adjustments: Vec::new(),
                tags: vec!["terminal".to_owned(), "smoke".to_owned()],
                metadata: BTreeMap::from([("difficulty".to_owned(), "nominal".to_owned())]),
            })],
        };

        let report = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
        assert_eq!(report.total_runs, 7 * 3 * 3);
        assert_eq!(report.summary.by_entry[0].total_runs, 7 * 3 * 3);

        let record = report
            .records
            .iter()
            .find(|record| {
                record.resolved.selector.arc_point == "a80"
                    && record.resolved.selector.velocity_band == "high"
                    && record.resolved.resolved_seed == 6
            })
            .expect("matrix record present");

        assert_eq!(
            record.resolved.source_kind,
            ResolvedRunSourceKind::TerminalMatrix
        );
        assert_eq!(record.resolved.selector.mission, "terminal_guidance");
        assert_eq!(
            record.resolved.selector.arrival_family,
            "half_arc_terminal_v1"
        );
        assert_eq!(record.resolved.selector.condition_set, "clean");
        assert_eq!(record.resolved.selector.vehicle_variant, "nominal");
        assert_eq!(record.resolved.selector.arc_point, "a80");
        assert_eq!(record.resolved.selector.velocity_band, "high");
        assert_eq!(record.resolved.lane_id, "current");
        assert!(
            record
                .resolved
                .resolved_scenario_name
                .contains("a80 high seed 6 current")
        );
        assert_eq!(
            record.resolved.selector.expectation_tier.as_deref(),
            Some("core")
        );
        assert!(
            (record.manifest.summary.max_speed_mps > 0.0)
                && (record.manifest.summary.fuel_remaining_kg >= 0.0)
        );
        assert_eq!(record.manifest.scenario_seed, 6);
        assert_eq!(
            record
                .resolved
                .resolved_parameters
                .get("gravity_mps2")
                .copied(),
            Some(9.81)
        );
        assert_eq!(
            record
                .resolved
                .resolved_parameters
                .get("eval_max_time_s")
                .copied(),
            Some(90.0)
        );
        assert_eq!(
            record
                .resolved
                .resolved_parameters
                .get("reachability_max_time_s")
                .copied(),
            Some(60.0)
        );
    }

    #[test]
    fn terminal_matrix_entry_respects_arc_point_selectors() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_selected_arcs".to_owned(),
            name: "Terminal matrix selected arcs".to_owned(),
            description: "terminal matrix selected arcs".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_selected_arcs".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "staged".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "clean".to_owned(),
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                arc_points: vec!["a70".to_owned(), "a80".to_owned()],
                adjustments: Vec::new(),
                tags: Vec::new(),
                metadata: BTreeMap::new(),
            })],
        };

        let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

        assert_eq!(resolved_runs.len(), 2 * 3 * 3);
        assert!(
            resolved_runs
                .iter()
                .all(|run| matches!(run.descriptor.selector.arc_point.as_str(), "a70" | "a80"))
        );
    }

    #[test]
    fn transfer_matrix_entry_expands_smoke_route_axes() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "transfer_matrix_smoke".to_owned(),
            name: "Transfer matrix smoke".to_owned(),
            description: "transfer matrix smoke".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_clean_nominal".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                route_angles: Vec::new(),
                radius_tiers: Vec::new(),
                waypoint_profile: None,
                waypoint_handoff_envelope: None,
                evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
                adjustments: Vec::new(),
                tags: vec!["transfer".to_owned(), "smoke".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

        assert_eq!(resolved_runs.len(), 5 * 3);
        let run = resolved_runs
            .iter()
            .find(|run| {
                run.descriptor.selector.route_angle == "r+60" && run.descriptor.resolved_seed == 2
            })
            .expect("r+60 seed 2 transfer run should be present");
        let route = run
            .scenario
            .mission
            .transfer_route
            .as_ref()
            .expect("transfer route should be present");
        let source_pad = run
            .scenario
            .world
            .landing_pad(&route.source_pad_id)
            .unwrap();
        let target_pad = run
            .scenario
            .world
            .landing_pad(&route.target_pad_id)
            .unwrap();

        assert_eq!(
            run.descriptor.source_kind,
            ResolvedRunSourceKind::TransferMatrix
        );
        assert_eq!(run.descriptor.selector.mission, "transfer_guidance");
        assert_eq!(
            run.descriptor.selector.route_family,
            "signed_route_arc_transfer_v1"
        );
        assert_eq!(
            run.descriptor.selector.arrival_family,
            "signed_route_arc_transfer_v1"
        );
        assert_eq!(run.descriptor.selector.condition_set, "clean");
        assert_eq!(run.descriptor.selector.arc_point, "r+60");
        assert_eq!(run.descriptor.selector.velocity_band, "nominal");
        assert_eq!(run.descriptor.selector.radius_tier, "nominal");
        assert!(source_pad.center_x_m < target_pad.center_x_m);
        assert!(source_pad.surface_y_m < target_pad.surface_y_m);
        assert!((route.route_radius_m - 824.0).abs() < 1e-9);
        assert!((route.route_angle_deg - 60.0).abs() < 1e-9);
        assert_eq!(
            run.descriptor
                .resolved_parameters
                .get("route_radius_nominal_m"),
            Some(&800.0)
        );
        assert_eq!(
            run.descriptor.resolved_parameters.get("route_radius_pct"),
            Some(&0.03)
        );
        assert_eq!(run.scenario.initial_state.velocity_mps, Vec2::new(0.0, 0.0));
    }

    #[test]
    fn transfer_matrix_entry_expands_selected_radius_tiers() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "transfer_matrix_radius_tiers".to_owned(),
            name: "Transfer matrix radius tiers".to_owned(),
            description: "transfer matrix radius tiers".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_clean_nominal".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                route_angles: vec!["r00".to_owned()],
                radius_tiers: vec!["short".to_owned(), "long".to_owned()],
                waypoint_profile: None,
                waypoint_handoff_envelope: None,
                evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
                adjustments: Vec::new(),
                tags: vec!["transfer".to_owned(), "radius".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

        assert_eq!(resolved_runs.len(), 2 * 3);
        let short = resolved_runs
            .iter()
            .find(|run| {
                run.descriptor.selector.radius_tier == "short" && run.descriptor.resolved_seed == 0
            })
            .expect("short transfer run should be present");
        let long = resolved_runs
            .iter()
            .find(|run| {
                run.descriptor.selector.radius_tier == "long" && run.descriptor.resolved_seed == 2
            })
            .expect("long transfer run should be present");

        let short_route = short.scenario.mission.transfer_route.as_ref().unwrap();
        let long_route = long.scenario.mission.transfer_route.as_ref().unwrap();
        assert_eq!(short.descriptor.selector.velocity_band, "short");
        assert_eq!(long.descriptor.selector.velocity_band, "long");
        assert!(
            short
                .descriptor
                .run_id
                .contains("_r00_short_seed_00_current")
        );
        assert!(long.descriptor.run_id.contains("_r00_long_seed_02_current"));
        assert!((short_route.route_radius_m - 400.0).abs() < 1e-9);
        assert!((long_route.route_radius_m - 1236.0).abs() < 1e-9);
        assert_eq!(
            short.descriptor.resolved_parameters.get("route_radius_m"),
            Some(&400.0)
        );
        assert_eq!(
            long.descriptor.resolved_parameters.get("route_radius_m"),
            Some(&1236.0)
        );
        assert_eq!(
            long.descriptor
                .resolved_parameters
                .get("route_radius_nominal_m"),
            Some(&1200.0)
        );
        assert_eq!(
            long.descriptor.resolved_parameters.get("route_radius_pct"),
            Some(&0.03)
        );
    }

    #[test]
    fn transfer_matrix_seed_tier_perturbs_route_radius() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "transfer_matrix_seed_variation".to_owned(),
            name: "Transfer matrix seed variation".to_owned(),
            description: "transfer matrix seed variation".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_seed_variation".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                route_angles: vec!["r00".to_owned()],
                radius_tiers: vec!["nominal".to_owned()],
                waypoint_profile: None,
                waypoint_handoff_envelope: None,
                evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
                adjustments: Vec::new(),
                tags: vec!["transfer".to_owned(), "seed_variation".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();
        let radius_for_seed = |seed| {
            resolved_runs
                .iter()
                .find(|run| run.descriptor.resolved_seed == seed)
                .and_then(|run| run.scenario.mission.transfer_route.as_ref())
                .map(|route| route.route_radius_m)
                .expect("seeded transfer run should include a route")
        };

        assert_eq!(radius_for_seed(0), 800.0);
        assert_eq!(radius_for_seed(1), 776.0);
        assert_eq!(radius_for_seed(2), 824.0);
    }

    #[test]
    fn transfer_matrix_waypoint_profile_injects_dogleg_waypoint() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "transfer_matrix_waypoint_profile".to_owned(),
            name: "Transfer matrix waypoint profile".to_owned(),
            description: "transfer matrix waypoint profile".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_waypoint_nominal".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC.to_owned(),
                route_angles: vec!["r+80".to_owned()],
                radius_tiers: vec!["nominal".to_owned()],
                waypoint_profile: Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1.to_owned()),
                waypoint_handoff_envelope: None,
                evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
                adjustments: Vec::new(),
                tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

        assert_eq!(resolved_runs.len(), 3);
        let run = resolved_runs
            .iter()
            .find(|run| run.descriptor.resolved_seed == 0)
            .expect("seed 0 waypoint transfer run should be present");
        let route = run
            .scenario
            .mission
            .transfer_route
            .as_ref()
            .expect("transfer route should be present");
        let waypoint = route
            .waypoints
            .first()
            .expect("dogleg waypoint should be present");
        let source_pad = run
            .scenario
            .world
            .landing_pad(&route.source_pad_id)
            .unwrap();

        assert_eq!(route.waypoints.len(), 1);
        assert_eq!(waypoint.id, "wp_dogleg_01");
        assert!(waypoint.position_m.x < source_pad.center_x_m);
        assert!(waypoint.position_m.y > 0.0);
        assert_eq!(
            run.scenario.metadata.get("route_mode").map(String::as_str),
            Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1)
        );
        assert_eq!(
            run.scenario
                .metadata
                .get("waypoint_profile")
                .map(String::as_str),
            Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1)
        );
        assert_eq!(
            run.descriptor.selector.waypoint_profile,
            TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1
        );
        assert_eq!(
            run.descriptor.selector.waypoint_handoff_envelope,
            TRANSFER_WAYPOINT_ENVELOPE_LEGACY_V1
        );
        assert_eq!(
            run.descriptor.resolved_parameters.get("waypoint_0_x_m"),
            Some(&waypoint.position_m.x)
        );
        assert_eq!(
            run.descriptor
                .resolved_parameters
                .get("waypoint_0_capture_radius_m"),
            Some(&waypoint.capture_radius_m)
        );
        assert_eq!(
            run.descriptor
                .resolved_parameters
                .get("waypoint_0_max_outbound_heading_error_rad"),
            Some(&waypoint.max_outbound_heading_error_rad)
        );
        assert_eq!(
            run.descriptor
                .resolved_parameters
                .get("waypoint_0_min_outbound_progress_mps"),
            Some(&waypoint.min_outbound_progress_mps)
        );
        assert_eq!(
            run.descriptor
                .resolved_parameters
                .get("waypoint_0_max_vertical_speed_mps"),
            waypoint.max_vertical_speed_mps.as_ref()
        );
        let turn_angle_deg = run
            .descriptor
            .resolved_parameters
            .get("waypoint_0_turn_angle_deg")
            .expect("dogleg profile should expose turn angle");
        assert!(*turn_angle_deg > 140.0);
    }

    #[test]
    fn transfer_matrix_dogleg_profile_requires_diagnostic_tier() {
        let entry = TransferMatrixEntry {
            id: "transfer_guidance_waypoint_dogleg".to_owned(),
            transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TransferMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "transfer_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TransferSeedTier::Smoke,
            vehicle_variant: "nominal".to_owned(),
            expectation_tier: "frontier_probe".to_owned(),
            route_angles: vec!["r+80".to_owned()],
            radius_tiers: vec!["nominal".to_owned()],
            waypoint_profile: Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1.to_owned()),
            waypoint_handoff_envelope: None,
            evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
            adjustments: Vec::new(),
            tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
            metadata: BTreeMap::new(),
        };

        let message = validate_transfer_matrix_entry(&entry)
            .unwrap_err()
            .to_string();

        assert!(message.contains("requires expectation_tier 'diagnostic'"));
    }

    #[test]
    fn transfer_matrix_late_bend_profile_requires_diagnostic_tier() {
        let entry = TransferMatrixEntry {
            id: "transfer_guidance_waypoint_late_bend".to_owned(),
            transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
            base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
            lanes: vec![TransferMatrixLaneSpec {
                id: "current".to_owned(),
                controller: "transfer_pdg".to_owned(),
                controller_config: None,
            }],
            seed_tier: TransferSeedTier::Smoke,
            vehicle_variant: "empty".to_owned(),
            expectation_tier: "contract_probe".to_owned(),
            route_angles: vec!["r+30".to_owned()],
            radius_tiers: vec!["nominal".to_owned()],
            waypoint_profile: Some(TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1.to_owned()),
            waypoint_handoff_envelope: Some(
                TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1.to_owned(),
            ),
            evaluation_goal: TransferMatrixEvaluationGoal::WaypointSequence,
            adjustments: Vec::new(),
            tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
            metadata: BTreeMap::new(),
        };

        let message = validate_transfer_matrix_entry(&entry)
            .unwrap_err()
            .to_string();

        assert!(message.contains("requires expectation_tier 'diagnostic'"));
    }

    #[test]
    fn transfer_matrix_waypoint_profile_injects_smooth_bend_waypoint() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "transfer_matrix_waypoint_bend_profile".to_owned(),
            name: "Transfer matrix waypoint bend profile".to_owned(),
            description: "transfer matrix waypoint bend profile".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_waypoint_bend".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "frontier_probe".to_owned(),
                route_angles: vec!["r+80".to_owned()],
                radius_tiers: vec!["short".to_owned()],
                waypoint_profile: Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1.to_owned()),
                waypoint_handoff_envelope: Some(
                    TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1.to_owned(),
                ),
                evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
                adjustments: Vec::new(),
                tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

        assert_eq!(resolved_runs.len(), 3);
        let run = resolved_runs
            .iter()
            .find(|run| run.descriptor.resolved_seed == 0)
            .expect("seed 0 waypoint transfer run should be present");
        let route = run
            .scenario
            .mission
            .transfer_route
            .as_ref()
            .expect("transfer route should be present");
        let waypoint = route
            .waypoints
            .first()
            .expect("bend waypoint should be present");

        assert_eq!(route.waypoints.len(), 1);
        assert_eq!(waypoint.id, "wp_bend_01");
        assert_eq!(
            run.descriptor.selector.waypoint_profile,
            TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1
        );
        assert_eq!(
            run.descriptor.selector.waypoint_handoff_envelope,
            TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1
        );
        assert_eq!(
            run.descriptor
                .resolved_parameters
                .get("waypoint_0_capture_radius_m"),
            Some(&40.0)
        );
        assert_eq!(
            run.descriptor
                .resolved_parameters
                .get("waypoint_0_max_cross_track_m"),
            Some(&70.0)
        );
        assert_eq!(waypoint.max_outbound_heading_error_rad, 0.35);
        assert_eq!(waypoint.max_outbound_cross_speed_mps, Some(20.0));
        assert_eq!(waypoint.max_speed_mps, 52.5);
        assert_eq!(waypoint.min_vertical_speed_mps, None);
        assert_eq!(waypoint.max_vertical_speed_mps, None);
        let params = &run.descriptor.resolved_parameters;
        let progress_frac = params
            .get("waypoint_0_profile_progress_frac")
            .expect("bend profile should expose progress fraction");
        let offset_ratio = params
            .get("waypoint_0_profile_lateral_offset_ratio")
            .expect("bend profile should expose offset ratio");
        let turn_angle_deg = params
            .get("waypoint_0_turn_angle_deg")
            .expect("bend profile should expose turn angle");
        let signed_turn_angle_deg = params
            .get("waypoint_0_signed_turn_angle_deg")
            .expect("bend profile should expose signed turn angle");
        assert!((*progress_frac - 0.55).abs() < 1.0e-9);
        assert!((*offset_ratio - 0.20).abs() < 1.0e-9);
        assert!(*turn_angle_deg > 40.0 && *turn_angle_deg < 48.0);
        assert!((*signed_turn_angle_deg + 43.9456).abs() < 1.0e-3);
        assert!((params["waypoint_0_route_signed_offset_ratio"] - *offset_ratio).abs() < 1.0e-9);
        assert!(
            params["waypoint_0_continuation_stop_ratio"]
                <= TRANSFER_WAYPOINT_CONTINUATION_MAX_STOP_RATIO
        );
    }

    #[test]
    fn transfer_waypoint_bend_packs_use_continuation_envelopes() {
        let packs_dir = fixtures_root().join("packs");
        for (filename, expected_runs) in [
            ("transfer_waypoint_bend_rpos80_smoke.json", 27),
            ("transfer_waypoint_bend_contract_rpos80_smoke.json", 27),
            ("transfer_waypoint_bend_rpos80_full.json", 108),
            ("transfer_waypoint_bend_contract_rpos80_full.json", 108),
        ] {
            let pack = load_pack(&packs_dir.join(filename)).unwrap();
            let runs = resolve_pack_runs(&pack, &packs_dir).unwrap();
            assert_eq!(runs.len(), expected_runs);
            for run in runs {
                assert_eq!(
                    run.descriptor.selector.waypoint_handoff_envelope,
                    TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1
                );
                let waypoint = &run
                    .scenario
                    .mission
                    .transfer_route
                    .as_ref()
                    .unwrap()
                    .waypoints[0];
                let expected_speed_cap_mps = match run.descriptor.selector.radius_tier.as_str() {
                    "short" => 52.5,
                    "nominal" => 65.0,
                    "long" => 75.0,
                    radius_tier => panic!("unexpected radius tier {radius_tier}"),
                };
                assert_eq!(waypoint.max_speed_mps, expected_speed_cap_mps);
                assert!(
                    run.descriptor.resolved_parameters["waypoint_0_continuation_stop_ratio"]
                        <= TRANSFER_WAYPOINT_CONTINUATION_MAX_STOP_RATIO
                );
            }
        }
    }

    #[test]
    fn transfer_waypoint_turn_packs_expand_balanced_paired_matrix() {
        let packs_dir = fixtures_root().join("packs");
        let mission_path = packs_dir.join("transfer_waypoint_turn_smoke.json");
        let contract_path = packs_dir.join("transfer_waypoint_turn_contract_smoke.json");
        let mission_pack = load_pack(&mission_path).unwrap();
        let contract_pack = load_pack(&contract_path).unwrap();
        let mission_runs = resolve_pack_runs(&mission_pack, &packs_dir).unwrap();
        let contract_runs = resolve_pack_runs(&contract_pack, &packs_dir).unwrap();

        assert_eq!(mission_runs.len(), 81);
        assert_eq!(contract_runs.len(), 81);
        for runs in [&mission_runs, &contract_runs] {
            let run_ids = runs
                .iter()
                .map(|run| run.descriptor.run_id.as_str())
                .collect::<BTreeSet<_>>();
            assert_eq!(run_ids.len(), 81);
            assert!(runs.iter().all(|run| {
                run.descriptor.selector.waypoint_handoff_envelope
                    == TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1
            }));
            for profile in [
                TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1,
                TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1,
                TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1,
            ] {
                assert_eq!(
                    runs.iter()
                        .filter(|run| run.descriptor.selector.waypoint_profile == profile)
                        .count(),
                    27
                );
            }
            for vehicle in ["empty", "half", "full"] {
                assert_eq!(
                    runs.iter()
                        .filter(|run| run.descriptor.selector.vehicle_variant == vehicle)
                        .count(),
                    27
                );
            }
            for route_angle in ["r-30", "r00", "r+30"] {
                assert_eq!(
                    runs.iter()
                        .filter(|run| run.descriptor.selector.route_angle == route_angle)
                        .count(),
                    27
                );
            }
            for run in runs.iter() {
                let route = run
                    .scenario
                    .mission
                    .transfer_route
                    .as_ref()
                    .expect("balanced waypoint profile should resolve a route");
                let waypoint = route
                    .waypoints
                    .first()
                    .expect("balanced waypoint profile should resolve one waypoint");
                let terrain_y_m = run
                    .scenario
                    .world
                    .terrain
                    .sample_height(waypoint.position_m.x);
                let waypoint_clearance_m = waypoint.position_m.y - terrain_y_m;
                let required_envelope_clearance_m =
                    waypoint.capture_radius_m.max(waypoint.max_cross_track_m)
                        + run.scenario.vehicle.geometry.touchdown_base_offset_m;
                assert!(
                    waypoint_clearance_m > required_envelope_clearance_m,
                    "profile {} route {} waypoint clearance {:.3} must exceed envelope and vehicle clearance {:.3}",
                    run.descriptor.selector.waypoint_profile,
                    run.descriptor.selector.route_angle,
                    waypoint_clearance_m,
                    required_envelope_clearance_m,
                );
                let params = &run.descriptor.resolved_parameters;
                assert!(
                    (params["waypoint_0_terrain_clearance_m"] - waypoint_clearance_m).abs()
                        < 1.0e-9
                );
            }
        }

        let selector_key = |run: &ResolvedBatchRun| {
            format!(
                "{}|{}|{}|{}|{}",
                run.descriptor.selector.waypoint_profile,
                run.descriptor.selector.vehicle_variant,
                run.descriptor.selector.route_angle,
                run.descriptor.selector.radius_tier,
                run.descriptor.resolved_seed,
            )
        };
        let mission_cells = mission_runs
            .iter()
            .map(selector_key)
            .collect::<BTreeSet<_>>();
        let contract_cells = contract_runs
            .iter()
            .map(selector_key)
            .collect::<BTreeSet<_>>();
        assert_eq!(mission_cells, contract_cells);
        assert!(mission_runs.iter().all(|run| matches!(
            run.scenario.mission.goal,
            EvaluationGoal::LandingOnPad { .. }
        )));
        assert!(contract_runs.iter().all(|run| matches!(
            run.scenario.mission.goal,
            EvaluationGoal::WaypointHandoff { .. }
        )));

        for (profile, expected_turn_angle_deg, expected_offset_ratio) in [
            (
                TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1,
                27.2394,
                0.12,
            ),
            (
                TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1,
                43.9456,
                0.20,
            ),
            (
                TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1,
                62.3005,
                0.30,
            ),
        ] {
            let run = mission_runs
                .iter()
                .find(|run| {
                    run.descriptor.selector.waypoint_profile == profile
                        && run.descriptor.selector.vehicle_variant == "empty"
                        && run.descriptor.selector.route_angle == "r00"
                        && run.descriptor.resolved_seed == 0
                })
                .expect("balanced waypoint profile cell should resolve");
            let params = &run.descriptor.resolved_parameters;
            let turn_angle_deg = params["waypoint_0_turn_angle_deg"];
            let signed_turn_angle_deg = params["waypoint_0_signed_turn_angle_deg"];
            assert!(
                (turn_angle_deg - expected_turn_angle_deg).abs() < 1.0,
                "profile {profile} resolved turn angle {turn_angle_deg:.3}"
            );
            assert!(
                (signed_turn_angle_deg + expected_turn_angle_deg).abs() < 1.0,
                "profile {profile} resolved signed turn angle {signed_turn_angle_deg:.3}"
            );
            assert_eq!(params["waypoint_0_profile_progress_frac"], 0.55);
            assert!(
                (params["waypoint_0_profile_lateral_offset_ratio"] - expected_offset_ratio).abs()
                    < 1.0e-9
            );
            assert_eq!(
                params["waypoint_0_max_cross_track_m"] / params["waypoint_0_capture_radius_m"],
                1.25
            );
        }
    }

    #[test]
    fn transfer_waypoint_sequence_packs_expand_balanced_paired_matrix() {
        let packs_dir = fixtures_root().join("packs");
        let mission_pack =
            load_pack(&packs_dir.join("transfer_waypoint_sequence_smoke.json")).unwrap();
        let contract_pack =
            load_pack(&packs_dir.join("transfer_waypoint_sequence_contract_smoke.json")).unwrap();
        let mission_runs = resolve_pack_runs(&mission_pack, &packs_dir).unwrap();
        let contract_runs = resolve_pack_runs(&contract_pack, &packs_dir).unwrap();

        assert_eq!(mission_runs.len(), 27);
        assert_eq!(contract_runs.len(), 27);
        for runs in [&mission_runs, &contract_runs] {
            let run_ids = runs
                .iter()
                .map(|run| run.descriptor.run_id.as_str())
                .collect::<BTreeSet<_>>();
            assert_eq!(run_ids.len(), 27);
            assert!(runs.iter().all(|run| {
                run.descriptor.selector.waypoint_handoff_envelope
                    == TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1
            }));
            assert!(runs.iter().all(|run| {
                run.descriptor.selector.waypoint_profile == TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1
            }));
            for vehicle in ["empty", "half", "full"] {
                assert_eq!(
                    runs.iter()
                        .filter(|run| run.descriptor.selector.vehicle_variant == vehicle)
                        .count(),
                    9
                );
            }
            for route_angle in ["r-30", "r00", "r+30"] {
                assert_eq!(
                    runs.iter()
                        .filter(|run| run.descriptor.selector.route_angle == route_angle)
                        .count(),
                    9
                );
            }
            for run in runs {
                let route = run
                    .scenario
                    .mission
                    .transfer_route
                    .as_ref()
                    .expect("sequence profile should resolve a route");
                assert_eq!(route.waypoints.len(), 2);
                assert!(
                    (route.waypoints[1].position_m - route.waypoints[0].position_m).length()
                        > route.waypoints[0].capture_radius_m + route.waypoints[1].capture_radius_m
                );
                for waypoint in &route.waypoints {
                    assert!(waypoint.handoff_tangent_unit.is_some());
                    let terrain_clearance_m = waypoint.position_m.y
                        - run
                            .scenario
                            .world
                            .terrain
                            .sample_height(waypoint.position_m.x);
                    assert!(
                        terrain_clearance_m
                            > waypoint.max_cross_track_m
                                + run.scenario.vehicle.geometry.touchdown_base_offset_m
                    );
                }
                let params = &run.descriptor.resolved_parameters;
                assert!(
                    params["waypoint_0_profile_progress_frac"]
                        < params["waypoint_1_profile_progress_frac"]
                );
                assert_eq!(
                    params["waypoint_0_max_cross_track_m"] / params["waypoint_0_capture_radius_m"],
                    1.25
                );
                let expected_speed_caps = [55.0, 65.0];
                assert_eq!(route.waypoints[0].max_speed_mps, expected_speed_caps[0]);
                assert_eq!(route.waypoints[1].max_speed_mps, expected_speed_caps[1]);
                assert!(params["waypoint_0_turn_authority_ratio"] <= 0.75);
                assert!(params["waypoint_1_turn_authority_ratio"] <= 0.75);
            }
        }

        let selector_key = |run: &ResolvedBatchRun| {
            format!(
                "{}|{}|{}|{}|{}",
                run.descriptor.selector.waypoint_profile,
                run.descriptor.selector.vehicle_variant,
                run.descriptor.selector.route_angle,
                run.descriptor.selector.radius_tier,
                run.descriptor.resolved_seed,
            )
        };
        let mission_cells = mission_runs
            .iter()
            .map(selector_key)
            .collect::<BTreeSet<_>>();
        let contract_cells = contract_runs
            .iter()
            .map(selector_key)
            .collect::<BTreeSet<_>>();
        assert_eq!(mission_cells, contract_cells);
        assert!(mission_runs.iter().all(|run| matches!(
            run.scenario.mission.goal,
            EvaluationGoal::LandingOnPad { .. }
        )));
        assert!(contract_runs.iter().all(|run| matches!(
            run.scenario.mission.goal,
            EvaluationGoal::WaypointSequence { .. }
        )));

        let mission_geometry = mission_runs
            .iter()
            .map(|run| {
                let route = run.scenario.mission.transfer_route.as_ref().unwrap();
                (
                    selector_key(run),
                    route
                        .waypoints
                        .iter()
                        .map(|waypoint| waypoint.position_m)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let contract_geometry = contract_runs
            .iter()
            .map(|run| {
                let route = run.scenario.mission.transfer_route.as_ref().unwrap();
                (
                    selector_key(run),
                    route
                        .waypoints
                        .iter()
                        .map(|waypoint| waypoint.position_m)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(mission_geometry, contract_geometry);

        for route_angle in ["r-30", "r00", "r+30"] {
            let run = mission_runs
                .iter()
                .find(|run| {
                    run.descriptor.selector.vehicle_variant == "empty"
                        && run.descriptor.selector.route_angle == route_angle
                        && run.descriptor.resolved_seed == 0
                })
                .expect("double-bend sequence cell should resolve");
            let params = &run.descriptor.resolved_parameters;
            for index in 0..2 {
                assert!(
                    (params[&format!("waypoint_{index}_turn_angle_deg")] - 31.2184).abs() < 1.0e-3
                );
                assert!(
                    (params[&format!("waypoint_{index}_signed_turn_angle_deg")] + 31.2184).abs()
                        < 1.0e-3
                );
                assert!(
                    (params[&format!("waypoint_{index}_inbound_tangent_angle_deg")] - 15.6092)
                        .abs()
                        < 1.0e-3
                );
                assert!(
                    (params[&format!("waypoint_{index}_tangent_outbound_angle_deg")] - 15.6092)
                        .abs()
                        < 1.0e-3
                );
            }
        }
        let uphill = mission_runs
            .iter()
            .find(|run| {
                run.descriptor.selector.vehicle_variant == "empty"
                    && run.descriptor.selector.route_angle == "r+30"
                    && run.descriptor.resolved_seed == 0
            })
            .unwrap();
        assert!(
            (uphill.descriptor.resolved_parameters["waypoint_0_handoff_tangent_heading_deg"]
                - 45.6092)
                .abs()
                < 1.0e-3
        );
        assert!(
            (uphill.descriptor.resolved_parameters["waypoint_1_handoff_tangent_heading_deg"]
                - 14.3908)
                .abs()
                < 1.0e-3
        );
    }

    #[test]
    fn transfer_waypoint_late_bend_diagnostic_preserves_full_matrix() {
        let packs_dir = fixtures_root().join("packs");
        let pack =
            load_pack(&packs_dir.join("transfer_waypoint_sequence_late_bend_diagnostic.json"))
                .unwrap();
        let runs = resolve_pack_runs(&pack, &packs_dir).unwrap();

        assert_eq!(runs.len(), 27);
        assert!(runs.iter().all(|run| {
            run.descriptor.selector.expectation_tier.as_deref() == Some("diagnostic")
                && run.descriptor.selector.radius_tier == "nominal"
                && run.descriptor.selector.waypoint_profile
                    == TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1
                && run.descriptor.selector.waypoint_handoff_envelope
                    == TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1
                && matches!(
                    run.scenario.mission.goal,
                    EvaluationGoal::LandingOnPad { .. }
                )
                && run
                    .scenario
                    .mission
                    .transfer_route
                    .as_ref()
                    .is_some_and(|route| route.waypoints.len() == 2)
        }));
        for vehicle in ["empty", "half", "full"] {
            assert_eq!(
                runs.iter()
                    .filter(|run| run.descriptor.selector.vehicle_variant == vehicle)
                    .count(),
                9
            );
        }
    }

    #[test]
    fn transfer_matrix_waypoint_handoff_goal_resolves_probe_goal() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "transfer_matrix_waypoint_handoff_goal".to_owned(),
            name: "Transfer matrix waypoint handoff goal".to_owned(),
            description: "transfer matrix waypoint handoff goal".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_waypoint_contract".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC.to_owned(),
                route_angles: vec!["r+80".to_owned()],
                radius_tiers: vec!["nominal".to_owned()],
                waypoint_profile: Some(TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1.to_owned()),
                waypoint_handoff_envelope: None,
                evaluation_goal: TransferMatrixEvaluationGoal::WaypointHandoff,
                adjustments: Vec::new(),
                tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();

        assert_eq!(resolved_runs.len(), 3);
        let run = resolved_runs
            .iter()
            .find(|run| run.descriptor.resolved_seed == 0)
            .expect("seed 0 waypoint handoff run should be present");
        assert!(matches!(
            run.scenario.mission.goal,
            EvaluationGoal::WaypointHandoff {
                waypoint_index: 0,
                ..
            }
        ));
        assert_eq!(
            run.scenario
                .metadata
                .get("evaluation_goal")
                .map(String::as_str),
            Some("waypoint_handoff")
        );
        assert_eq!(
            run.descriptor
                .resolved_parameters
                .get("waypoint_handoff_index"),
            Some(&0.0)
        );
    }

    #[test]
    fn transfer_matrix_waypoint_handoff_goal_requires_waypoint_profile() {
        let pack = ScenarioPackSpec {
            id: "transfer_matrix_invalid_waypoint_goal".to_owned(),
            name: "Transfer matrix invalid waypoint goal".to_owned(),
            description: "transfer matrix invalid waypoint goal".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_waypoint_contract".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "frontier_probe".to_owned(),
                route_angles: vec!["r+80".to_owned()],
                radius_tiers: vec!["nominal".to_owned()],
                waypoint_profile: None,
                waypoint_handoff_envelope: None,
                evaluation_goal: TransferMatrixEvaluationGoal::WaypointHandoff,
                adjustments: Vec::new(),
                tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let message = validate_pack(&pack).unwrap_err().to_string();

        assert!(message.contains("requires waypoint_profile"));
    }

    #[test]
    fn transfer_matrix_waypoint_envelope_requires_waypoint_profile() {
        let pack = ScenarioPackSpec {
            id: "transfer_matrix_invalid_waypoint_envelope".to_owned(),
            name: "Transfer matrix invalid waypoint envelope".to_owned(),
            description: "Transfer matrix invalid waypoint envelope".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_waypoint_envelope".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "frontier_probe".to_owned(),
                route_angles: vec!["r+80".to_owned()],
                radius_tiers: vec!["nominal".to_owned()],
                waypoint_profile: None,
                waypoint_handoff_envelope: Some(
                    TRANSFER_WAYPOINT_ENVELOPE_PASS_THROUGH_V1.to_owned(),
                ),
                evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
                adjustments: Vec::new(),
                tags: vec!["transfer".to_owned(), "waypoint".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let message = validate_pack(&pack).unwrap_err().to_string();

        assert!(message.contains("waypoint_handoff_envelope requires waypoint_profile"));
    }

    #[test]
    fn waypoint_contract_review_passes_when_capture_and_outbound_envelope_pass() {
        let scenario = waypoint_contract_scenario();
        let metrics = WaypointReviewMetrics {
            capture_status: Some("captured".to_owned()),
            active_index: Some(0),
            outbound_heading_error_rad: Some(0.4),
            outbound_progress_mps: Some(18.0),
            speed_mps: Some(42.0),
            vertical_speed_mps: Some(12.0),
            ..WaypointReviewMetrics::default()
        };

        let contract = waypoint_contract_review_metrics(&scenario, &metrics);

        assert_eq!(contract.status.as_deref(), Some("pass"));
        assert!(contract.reasons.is_empty());
    }

    #[test]
    fn waypoint_contract_review_flags_captured_bad_outbound_state() {
        let scenario = waypoint_contract_scenario();
        let metrics = WaypointReviewMetrics {
            capture_status: Some("captured".to_owned()),
            active_index: Some(0),
            outbound_heading_error_rad: Some(2.4),
            outbound_progress_mps: Some(-18.0),
            speed_mps: Some(42.0),
            vertical_speed_mps: Some(92.0),
            ..WaypointReviewMetrics::default()
        };

        let contract = waypoint_contract_review_metrics(&scenario, &metrics);

        assert_eq!(contract.status.as_deref(), Some("outbound_out_of_envelope"));
        assert_eq!(
            contract.reasons,
            vec![
                "heading".to_owned(),
                "outbound_progress".to_owned(),
                "vertical_speed".to_owned()
            ]
        );
    }

    #[test]
    fn waypoint_contract_review_scores_outbound_cross_speed_without_vertical_bounds() {
        let mut scenario = waypoint_contract_scenario();
        let waypoint = scenario
            .mission
            .transfer_route
            .as_mut()
            .and_then(|route| route.waypoints.first_mut())
            .expect("waypoint fixture");
        waypoint.max_outbound_cross_speed_mps = Some(20.0);
        waypoint.min_vertical_speed_mps = None;
        waypoint.max_vertical_speed_mps = None;
        let metrics = WaypointReviewMetrics {
            capture_status: Some("captured".to_owned()),
            active_index: Some(0),
            outbound_heading_error_rad: Some(0.4),
            outbound_progress_mps: Some(18.0),
            outbound_cross_speed_mps: Some(24.0),
            speed_mps: Some(42.0),
            vertical_speed_mps: Some(92.0),
            ..WaypointReviewMetrics::default()
        };

        let contract = waypoint_contract_review_metrics(&scenario, &metrics);

        assert_eq!(contract.status.as_deref(), Some("outbound_out_of_envelope"));
        assert_eq!(contract.reasons, vec!["outbound_cross_speed".to_owned()]);
    }

    #[test]
    fn waypoint_contract_review_flags_spatial_miss_separately() {
        let scenario = waypoint_contract_scenario();
        let metrics = WaypointReviewMetrics {
            capture_status: Some("missed".to_owned()),
            active_index: Some(0),
            distance_m: Some(90.0),
            cross_track_m: Some(72.0),
            plane_progress_m: Some(-65.0),
            outbound_heading_error_rad: Some(0.4),
            outbound_progress_mps: Some(18.0),
            speed_mps: Some(42.0),
            vertical_speed_mps: Some(12.0),
            ..WaypointReviewMetrics::default()
        };

        let contract = waypoint_contract_review_metrics(&scenario, &metrics);

        assert_eq!(contract.status.as_deref(), Some("spatial_miss"));
        assert_eq!(
            contract.reasons,
            vec![
                "outside_capture_radius".to_owned(),
                "cross_track".to_owned(),
                "before_waypoint_plane".to_owned()
            ]
        );
    }

    fn transfer_sample(
        physics_step: u64,
        sim_time_s: f64,
        position_m: Vec2,
        velocity_mps: Vec2,
        fuel_kg: f64,
    ) -> SampleRecord {
        SampleRecord {
            sim_time_s,
            physics_step,
            observation: pd_core::Observation {
                sim_time_s,
                physics_step,
                position_m,
                velocity_mps,
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
                mass_kg: 900.0 + fuel_kg,
                fuel_kg,
                gravity_mps2: 9.8,
                target_dx_m: -position_m.x,
                height_above_target_m: position_m.y,
                target_surface_y_m: 0.0,
                target_pad_half_width_m: 18.0,
                touchdown_clearance_m: position_m.y,
                min_hull_clearance_m: position_m.y,
            },
            held_command: pd_core::Command::idle(),
        }
    }

    fn transfer_update(
        physics_step: u64,
        sim_time_s: f64,
        transfer_phase: &str,
        route_dx_m: f64,
        projected_dx_m: f64,
        throttle_frac: f64,
    ) -> pd_control::ControllerUpdateRecord {
        pd_control::ControllerUpdateRecord {
            sim_time_s,
            physics_step,
            controller_update_index: physics_step,
            compute_time_us: None,
            frame: pd_control::ControllerFrame {
                command: pd_core::Command {
                    throttle_frac,
                    target_attitude_rad: 0.0,
                },
                status: transfer_phase.to_owned(),
                phase: Some(transfer_phase.to_owned()),
                metrics: BTreeMap::from([
                    (
                        metric::TRANSFER_PHASE.to_owned(),
                        TelemetryValue::from(transfer_phase),
                    ),
                    (
                        metric::TRANSFER_ROUTE_DX_M.to_owned(),
                        TelemetryValue::from(route_dx_m),
                    ),
                    (
                        metric::TRANSFER_TARGET_Y_SOLUTION.to_owned(),
                        TelemetryValue::from(true),
                    ),
                    (
                        metric::TRANSFER_PROJECTED_DX_M.to_owned(),
                        TelemetryValue::from(projected_dx_m),
                    ),
                ]),
                markers: Vec::new(),
            },
        }
    }

    #[test]
    fn waypoint_review_metrics_capture_first_waypoint_handoff() {
        let mut tracking = transfer_update(0, 0.0, "boost", 200.0, 80.0, 1.0);
        tracking.frame.metrics.extend(BTreeMap::from([
            (
                metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
                TelemetryValue::from(true),
            ),
            (
                metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
                TelemetryValue::from(0_i64),
            ),
            (
                metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
                TelemetryValue::from("tracking"),
            ),
        ]));
        let mut captured = transfer_update(60, 0.5, "boost", 100.0, 20.0, 0.8);
        captured.frame.metrics.extend(BTreeMap::from([
            (
                metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
                TelemetryValue::from(true),
            ),
            (
                metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
                TelemetryValue::from(0_i64),
            ),
            (
                metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
                TelemetryValue::from("captured"),
            ),
            (
                metric::WAYPOINT_CAPTURE_TIME_S.to_owned(),
                TelemetryValue::from(0.5),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_TIME_S.to_owned(),
                TelemetryValue::from(0.5),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_PASS.to_owned(),
                TelemetryValue::from(true),
            ),
            (
                metric::WAYPOINT_HANDOFF_RESOLUTION_REASON.to_owned(),
                TelemetryValue::from("contract_pass"),
            ),
            (
                metric::WAYPOINT_HANDOFF_WINDOW_DURATION_S.to_owned(),
                TelemetryValue::from(0.0),
            ),
            (
                metric::WAYPOINT_CLOSEST_DISTANCE_M.to_owned(),
                TelemetryValue::from(12.0),
            ),
            (
                metric::WAYPOINT_CROSS_TRACK_M.to_owned(),
                TelemetryValue::from(8.0),
            ),
            (
                metric::WAYPOINT_OUTBOUND_PROGRESS_MPS.to_owned(),
                TelemetryValue::from(24.0),
            ),
            (
                metric::WAYPOINT_REQUIRED_TURN_DISTANCE_M.to_owned(),
                TelemetryValue::from(72.0),
            ),
            (
                metric::WAYPOINT_TURN_MARGIN_M.to_owned(),
                TelemetryValue::from(38.0),
            ),
        ]));

        let metrics = waypoint_review_metrics(&[tracking, captured]);

        assert_eq!(metrics.capture_status.as_deref(), Some("captured"));
        assert_eq!(metrics.active_index, Some(0));
        assert_eq!(metrics.capture_time_s, Some(0.5));
        assert_eq!(
            metrics.window_entry.as_ref().and_then(|entry| entry.time_s),
            Some(0.5)
        );
        assert_eq!(
            metrics
                .window_entry
                .as_ref()
                .and_then(|entry| entry.contract_pass),
            Some(true)
        );
        assert_eq!(metrics.resolution_reason.as_deref(), Some("contract_pass"));
        assert_eq!(metrics.window_duration_s, Some(0.0));
        assert_eq!(metrics.closest_distance_m, Some(12.0));
        assert_eq!(metrics.cross_track_m, Some(8.0));
        assert_eq!(metrics.outbound_progress_mps, Some(24.0));
        assert_eq!(metrics.required_turn_distance_m, Some(72.0));
        assert_eq!(metrics.turn_margin_m, Some(38.0));
    }

    #[test]
    fn waypoint_review_history_preserves_ordered_handoff_markers() {
        let handoff_update = |physics_step: u64, sim_time_s: f64, index: i64, replans: i64| {
            let mut update = transfer_update(physics_step, sim_time_s, "boost", 100.0, 20.0, 0.8);
            update.frame.metrics.extend(BTreeMap::from([
                (
                    metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
                    TelemetryValue::from(true),
                ),
                (
                    metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
                    TelemetryValue::from(index),
                ),
                (
                    metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
                    TelemetryValue::from("captured"),
                ),
                (
                    metric::WAYPOINT_CAPTURE_TIME_S.to_owned(),
                    TelemetryValue::from(sim_time_s),
                ),
            ]));
            update.frame.markers.push(pd_control::ControllerMarker {
                id: marker::WAYPOINT_HANDOFF.to_owned(),
                label: format!("waypoint {index}"),
                x_m: None,
                y_m: None,
                metadata: BTreeMap::from([
                    (
                        metric::WAYPOINT_GUIDANCE_REPLAN_COUNT.to_owned(),
                        TelemetryValue::from(replans),
                    ),
                    (
                        metric::WAYPOINT_WINDOW_ENTRY_TIME_S.to_owned(),
                        TelemetryValue::from(sim_time_s - 0.2),
                    ),
                    (
                        metric::WAYPOINT_WINDOW_ENTRY_POSITION_X_M.to_owned(),
                        TelemetryValue::from(10.0 + index as f64),
                    ),
                    (
                        metric::WAYPOINT_WINDOW_ENTRY_POSITION_Y_M.to_owned(),
                        TelemetryValue::from(20.0),
                    ),
                    (
                        metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_PASS.to_owned(),
                        TelemetryValue::from(false),
                    ),
                    (
                        metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_REASONS.to_owned(),
                        TelemetryValue::from("heading"),
                    ),
                    (
                        metric::WAYPOINT_HANDOFF_RESOLUTION_REASON.to_owned(),
                        TelemetryValue::from("contract_pass"),
                    ),
                    (
                        metric::WAYPOINT_HANDOFF_WINDOW_DURATION_S.to_owned(),
                        TelemetryValue::from(0.2),
                    ),
                    (
                        metric::WAYPOINT_TARGET_VELOCITY_ERROR_MPS.to_owned(),
                        TelemetryValue::from(12.0 + index as f64),
                    ),
                    (
                        metric::WAYPOINT_TRANSITION_NEXT_INDEX.to_owned(),
                        TelemetryValue::from(index + 1),
                    ),
                    (
                        metric::WAYPOINT_TRANSITION_POSITION_ERROR_M.to_owned(),
                        TelemetryValue::from(4.0 + index as f64),
                    ),
                    (
                        metric::WAYPOINT_TRANSITION_CONTINUATION_CONTRACT_PASS.to_owned(),
                        TelemetryValue::from(true),
                    ),
                    (
                        metric::WAYPOINT_JOINT_NEXT_INDEX.to_owned(),
                        TelemetryValue::from(index + 1),
                    ),
                    (
                        metric::WAYPOINT_JOINT_EVALUATED_CANDIDATE_COUNT.to_owned(),
                        TelemetryValue::from(0_i64),
                    ),
                    (
                        metric::WAYPOINT_JOINT_PASSING_CANDIDATE_COUNT.to_owned(),
                        TelemetryValue::from(0_i64),
                    ),
                ]),
            });
            update
        };
        let second = handoff_update(120, 1.0, 1, 3);
        let first = handoff_update(60, 0.5, 0, 2);

        let history = waypoint_review_history(&[second, first]);

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].active_index, Some(0));
        assert_eq!(history[0].guidance_replan_count, Some(2));
        assert_eq!(history[0].target_velocity_error_mps, Some(12.0));
        assert_eq!(history[0].transition_next_waypoint_index, Some(1));
        assert_eq!(history[0].transition_position_error_m, Some(4.0));
        assert_eq!(history[0].transition_continuation_contract_pass, Some(true));
        assert_eq!(history[0].joint_next_waypoint_index, Some(1));
        assert_eq!(history[0].joint_evaluated_candidate_count, Some(0));
        assert_eq!(
            history[0]
                .window_entry
                .as_ref()
                .and_then(|entry| entry.contract_pass),
            Some(false)
        );
        assert_eq!(
            history[0]
                .window_entry
                .as_ref()
                .map(|entry| entry.contract_reasons.as_slice()),
            Some(["heading".to_owned()].as_slice())
        );
        assert_eq!(
            history[0].resolution_reason.as_deref(),
            Some("contract_pass")
        );
        assert_eq!(history[0].window_duration_s, Some(0.2));
        assert_eq!(history[1].active_index, Some(1));
        assert_eq!(history[1].guidance_replan_count, Some(3));
    }

    #[test]
    fn waypoint_transition_and_joint_evidence_require_the_immediate_next_index() {
        let scenario = waypoint_contract_scenario();
        let mut waypoint = WaypointReviewMetrics {
            active_index: Some(0),
            transition_next_waypoint_index: Some(2),
            transition_position_error_m: Some(4.0),
            transition_continuation_contract_pass: Some(true),
            joint_next_waypoint_index: Some(2),
            joint_evaluated_candidate_count: Some(4),
            joint_passing_candidate_count: Some(2),
            joint_contract_pass: Some(true),
            ..WaypointReviewMetrics::default()
        };

        let mismatched = batch_waypoint_handoff_metrics(&scenario, &waypoint).unwrap();
        assert_eq!(mismatched.transition_next_waypoint_index, None);
        assert_eq!(mismatched.transition_position_error_m, None);
        assert_eq!(mismatched.joint_next_waypoint_index, None);
        assert_eq!(mismatched.joint_evaluated_candidate_count, None);

        waypoint.transition_next_waypoint_index = Some(1);
        waypoint.joint_next_waypoint_index = Some(1);
        let matched = batch_waypoint_handoff_metrics(&scenario, &waypoint).unwrap();
        assert_eq!(matched.transition_next_waypoint_index, Some(1));
        assert_eq!(matched.transition_position_error_m, Some(4.0));
        assert_eq!(matched.joint_next_waypoint_index, Some(1));
        assert_eq!(matched.joint_evaluated_candidate_count, Some(4));
        assert_eq!(matched.joint_passing_candidate_count, Some(2));
        assert_eq!(matched.joint_contract_pass, Some(true));
    }

    #[test]
    fn waypoint_review_schema_reads_legacy_continuation_without_audit_fields() {
        let handoff: BatchWaypointHandoffReviewMetrics =
            serde_json::from_value(serde_json::json!({
                "waypoint_index": 0,
                "continuation_next_waypoint_index": 1,
                "continuation_contract_pass": true
            }))
            .unwrap();

        assert_eq!(handoff.continuation_next_waypoint_index, Some(1));
        assert_eq!(handoff.continuation_contract_pass, Some(true));
        assert_eq!(handoff.transition_next_waypoint_index, None);
        assert_eq!(handoff.joint_next_waypoint_index, None);
        assert!(handoff.window_entry.is_none());
        assert_eq!(handoff.resolution_reason, None);
        assert_eq!(handoff.window_duration_s, None);
    }

    #[test]
    fn waypoint_candidate_history_distinguishes_lost_and_never_passing_plans() {
        let mut scenario = waypoint_contract_scenario();
        let waypoint = &mut scenario.mission.transfer_route.as_mut().unwrap().waypoints[0];
        waypoint.max_outbound_heading_error_rad = 0.35;
        waypoint.max_outbound_cross_speed_mps = Some(20.0);

        let guidance_update = |physics_step: u64,
                               sim_time_s: f64,
                               passed: bool,
                               heading_error_rad: f64,
                               cross_speed_mps: f64| {
            let mut update =
                transfer_update(physics_step, sim_time_s, "waypoint", 100.0, 20.0, 0.8);
            update.frame.metrics.extend(BTreeMap::from([
                (
                    metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
                    TelemetryValue::from(true),
                ),
                (
                    metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
                    TelemetryValue::from(0_i64),
                ),
                (
                    metric::WAYPOINT_GUIDANCE_PLAN_INDEX.to_owned(),
                    TelemetryValue::from(0_i64),
                ),
                (
                    metric::WAYPOINT_GUIDANCE_PLAN_REVISION.to_owned(),
                    TelemetryValue::from(if passed { 1_i64 } else { 0_i64 }),
                ),
                (
                    metric::WAYPOINT_GUIDANCE_PLAN_REASON.to_owned(),
                    TelemetryValue::from(if passed {
                        "authority_recovery"
                    } else {
                        "initial"
                    }),
                ),
                (
                    metric::WAYPOINT_GUIDANCE_REFERENCE_POSITION_ERROR_M.to_owned(),
                    TelemetryValue::from(if passed { 20.0 } else { 10.0 }),
                ),
                (
                    metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_ERROR_M.to_owned(),
                    TelemetryValue::from(if passed { -8.0 } else { -3.0 }),
                ),
                (
                    metric::WAYPOINT_GUIDANCE_REFERENCE_VELOCITY_ERROR_MPS.to_owned(),
                    TelemetryValue::from(if passed { 4.0 } else { 2.0 }),
                ),
                (
                    metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_SPEED_ERROR_MPS.to_owned(),
                    TelemetryValue::from(if passed { -2.0 } else { -1.0 }),
                ),
                (
                    metric::WAYPOINT_GUIDANCE_REQUIRED_ACCEL_RATIO.to_owned(),
                    TelemetryValue::from(if passed { 0.8 } else { 1.2 }),
                ),
                (
                    metric::WAYPOINT_GUIDANCE_THRUST_SATURATED.to_owned(),
                    TelemetryValue::from(!passed),
                ),
                (
                    metric::WAYPOINT_GUIDANCE_TILT_SATURATED.to_owned(),
                    TelemetryValue::from(false),
                ),
                (
                    metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS.to_owned(),
                    TelemetryValue::from(passed),
                ),
                (
                    metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
                    TelemetryValue::from(heading_error_rad),
                ),
                (
                    metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
                    TelemetryValue::from(cross_speed_mps),
                ),
                (
                    metric::WAYPOINT_REACHABLE_HANDOFF_CONTRACT_PASS.to_owned(),
                    TelemetryValue::from(false),
                ),
                (
                    metric::WAYPOINT_REACHABLE_HANDOFF_REQUIRED_ACCEL_RATIO_MAX.to_owned(),
                    TelemetryValue::from(if passed { 1.4 } else { 1.1 }),
                ),
                (
                    metric::WAYPOINT_REACHABLE_HANDOFF_THRUST_SATURATED_TIME_S.to_owned(),
                    TelemetryValue::from(if passed { 0.2 } else { 0.1 }),
                ),
                (
                    metric::WAYPOINT_REACHABLE_HANDOFF_TILT_SATURATED_TIME_S.to_owned(),
                    TelemetryValue::from(if passed { 0.3 } else { 0.0 }),
                ),
            ]));
            update
        };
        let mut handoff = guidance_update(180, 1.5, false, 0.4, 21.0);
        handoff.frame.metrics.insert(
            metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
            TelemetryValue::from("captured"),
        );
        handoff.frame.metrics.insert(
            metric::WAYPOINT_CAPTURE_TIME_S.to_owned(),
            TelemetryValue::from(1.5),
        );
        handoff.frame.metrics.insert(
            metric::WAYPOINT_GUIDANCE_PLAN_INDEX.to_owned(),
            TelemetryValue::from(1_i64),
        );
        handoff.frame.markers.push(pd_control::ControllerMarker {
            id: marker::WAYPOINT_HANDOFF.to_owned(),
            label: "waypoint 0".to_owned(),
            x_m: None,
            y_m: None,
            metadata: BTreeMap::new(),
        });
        let updates = vec![
            guidance_update(60, 0.5, false, 0.5, 24.0),
            guidance_update(120, 1.0, true, 0.2, 18.0),
            handoff,
        ];
        let mut history = waypoint_review_history(&updates);

        enrich_waypoint_candidate_history(&scenario, &updates, &mut history);

        assert_eq!(history[0].candidate_contract_pass_ever, Some(true));
        assert_eq!(history[0].candidate_first_pass_time_s, Some(1.0));
        assert_eq!(history[0].candidate_last_pass_time_s, Some(1.0));
        assert_eq!(history[0].candidate_pass_lost_before_capture, Some(false));
        assert!((history[0].candidate_best_heading_margin_rad.unwrap() - 0.15).abs() < 1.0e-9);
        assert_eq!(history[0].candidate_best_cross_speed_margin_mps, Some(2.0));
        assert_eq!(
            history[0].reachable_candidate_contract_pass_ever,
            Some(false)
        );
        assert_eq!(
            history[0].reachable_candidate_pass_lost_before_capture,
            Some(false)
        );
        assert_eq!(history[0].reachable_required_accel_ratio_max, Some(1.4));
        assert_eq!(history[0].reachable_thrust_saturated_time_max_s, Some(0.2));
        assert_eq!(history[0].reachable_tilt_saturated_time_max_s, Some(0.3));
        assert_eq!(history[0].plan_reference_position_error_max_m, Some(20.0));
        assert_eq!(history[0].plan_reference_cross_error_max_abs_m, Some(8.0));
        assert_eq!(history[0].plan_reference_velocity_error_max_mps, Some(4.0));
        assert_eq!(history[0].guidance_required_accel_ratio_max, Some(1.2));
        assert_eq!(history[0].guidance_thrust_saturated_time_s, Some(0.5));
        assert_eq!(history[0].guidance_tilt_saturated_time_s, Some(0.0));
        assert_eq!(history[0].guidance_first_saturation_lead_s, Some(1.0));
        assert_eq!(history[0].last_pass_reference_position_error_m, Some(20.0));
        assert_eq!(history[0].last_pass_reference_velocity_error_mps, Some(4.0));
        assert_eq!(history[0].last_pass_required_accel_ratio, Some(0.8));
        assert_eq!(history[0].guidance_plan_revision_max, Some(1));
        assert_eq!(
            history[0].guidance_plan_reasons,
            vec!["authority_recovery", "initial"]
        );
    }

    #[test]
    fn waypoint_route_review_uses_authoritative_sequence_progress() {
        let mut scenario = waypoint_contract_scenario();
        scenario.mission.goal = EvaluationGoal::WaypointSequence {
            target_pad_id: "pad_a".to_owned(),
        };
        let route = scenario.mission.transfer_route.as_mut().unwrap();
        route.waypoints.push(TransferWaypointSpec {
            id: "wp_1".to_owned(),
            position_m: Vec2::new(-100.0, 120.0),
            ..route.waypoints[0].clone()
        });
        let manifest = RunManifest {
            schema_version: pd_core::model::RUN_SCHEMA_VERSION,
            scenario_id: scenario.id.clone(),
            scenario_name: scenario.name.clone(),
            scenario_seed: scenario.seed,
            scenario_tags: scenario.tags.clone(),
            controller_id: "transfer_waypoint_pdg_v1".to_owned(),
            physics_hz: scenario.sim.physics_hz,
            controller_hz: scenario.sim.controller_hz,
            sim_time_s: 1.0,
            physics_steps: 120,
            controller_updates: 2,
            physical_outcome: pd_core::PhysicalOutcome::Flying,
            mission_outcome: MissionOutcome::FailedCheckpoint,
            end_reason: EndReason::CheckpointFailed,
            summary: RunSummary {
                waypoint_sequence: Some(pd_core::model::WaypointSequenceRunSummary {
                    passed_handoffs: 1,
                    total_handoffs: 2,
                    first_failed_index: Some(1),
                }),
                ..RunSummary::default()
            },
        };
        let handoffs = vec![BatchWaypointHandoffReviewMetrics {
            waypoint_index: 0,
            contract_status: Some("pass".to_owned()),
            ..BatchWaypointHandoffReviewMetrics::default()
        }];

        let route = waypoint_route_review_metrics(&scenario, &manifest, &handoffs);

        assert_eq!(route.status.as_deref(), Some("failed"));
        assert_eq!(route.passed, Some(1));
        assert_eq!(route.total, Some(2));
        assert_eq!(route.first_failure_index, Some(1));
    }

    #[test]
    fn waypoint_handoff_goal_review_uses_terminal_sample_state() {
        let mut scenario = waypoint_contract_scenario();
        scenario.mission.goal = EvaluationGoal::WaypointHandoff {
            target_pad_id: "pad_a".to_owned(),
            waypoint_index: 0,
        };
        let samples = vec![
            transfer_sample(0, 0.0, Vec2::new(-420.0, 120.0), Vec2::new(0.0, 0.0), 120.0),
            transfer_sample(
                60,
                0.5,
                Vec2::new(-220.0, 180.0),
                Vec2::new(22.0, -18.0),
                118.0,
            ),
        ];
        let manifest = RunManifest {
            schema_version: pd_core::model::RUN_SCHEMA_VERSION,
            scenario_id: scenario.id.clone(),
            scenario_name: scenario.name.clone(),
            scenario_seed: scenario.seed,
            scenario_tags: scenario.tags.clone(),
            controller_id: "transfer_waypoint_pdg_v1".to_owned(),
            physics_hz: scenario.sim.physics_hz,
            controller_hz: scenario.sim.controller_hz,
            sim_time_s: 0.5,
            physics_steps: 60,
            controller_updates: 2,
            physical_outcome: pd_core::PhysicalOutcome::Flying,
            mission_outcome: MissionOutcome::Success,
            end_reason: EndReason::CheckpointSatisfied,
            summary: RunSummary::default(),
        };
        let mut guidance_update = transfer_update(48, 0.4, "boost", 100.0, 20.0, 0.8);
        guidance_update.frame.metrics.extend(BTreeMap::from([
            (
                metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
                TelemetryValue::from(true),
            ),
            (
                metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
                TelemetryValue::from(0_i64),
            ),
            (
                metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
                TelemetryValue::from("tracking"),
            ),
            (
                metric::WAYPOINT_TARGET_VX_MPS.to_owned(),
                TelemetryValue::from(30.0),
            ),
            (
                metric::WAYPOINT_TARGET_VY_MPS.to_owned(),
                TelemetryValue::from(0.0),
            ),
            (
                metric::WAYPOINT_TARGET_DEADLINE_REMAINING_S.to_owned(),
                TelemetryValue::from(2.5),
            ),
            (
                metric::WAYPOINT_REQUIRED_TURN_DISTANCE_M.to_owned(),
                TelemetryValue::from(30.0),
            ),
            (
                metric::WAYPOINT_GUIDANCE_FEASIBLE.to_owned(),
                TelemetryValue::from(false),
            ),
            (
                metric::WAYPOINT_PREDICTED_HANDOFF_TIME_TO_GO_S.to_owned(),
                TelemetryValue::from(0.2),
            ),
            (
                metric::WAYPOINT_PREDICTED_HANDOFF_DEADLINE_LEAD_S.to_owned(),
                TelemetryValue::from(2.3),
            ),
            (
                metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS.to_owned(),
                TelemetryValue::from(false),
            ),
            (
                metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_REASONS.to_owned(),
                TelemetryValue::from("heading,outbound_cross_speed"),
            ),
            (
                metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
                TelemetryValue::from(0.5),
            ),
        ]));

        let metrics = waypoint_handoff_goal_review_metrics(
            &scenario,
            &samples,
            &manifest,
            &[guidance_update],
        )
        .expect("waypoint handoff review should be available");
        let contract = waypoint_contract_review_metrics(&scenario, &metrics);

        assert_eq!(metrics.capture_status.as_deref(), Some("captured"));
        assert_eq!(metrics.active_index, Some(0));
        assert_eq!(metrics.capture_time_s, Some(0.5));
        assert_eq!(metrics.closest_distance_m, Some(0.0));
        assert_eq!(metrics.distance_m, Some(0.0));
        assert_eq!(metrics.target_deadline_remaining_s, Some(2.4));
        assert!((metrics.predicted_handoff_time_to_go_s.unwrap() - 0.1).abs() < 1.0e-9);
        assert_eq!(metrics.predicted_handoff_deadline_lead_s, Some(2.3));
        assert_eq!(
            metrics.predicted_handoff_contract_status.as_deref(),
            Some("fail")
        );
        assert_eq!(
            metrics.predicted_handoff_contract_reasons,
            vec!["heading", "outbound_cross_speed"]
        );
        assert_eq!(
            metrics.predicted_handoff_outbound_heading_error_rad,
            Some(0.5)
        );
        assert!((metrics.target_velocity_error_mps.unwrap() - 19.6977156036).abs() < 1.0e-9);
        assert_eq!(metrics.handoff_turn_margin_m, Some(-30.0));
        assert_eq!(
            metrics.guidance_snapshot_source.as_deref(),
            Some("last_pre_capture_update")
        );
        assert!((metrics.guidance_snapshot_age_s.unwrap() - 0.1).abs() < 1.0e-9);
        assert_eq!(contract.status.as_deref(), Some("pass"));
    }

    #[test]
    fn transfer_shape_metrics_use_boost_window_reference() {
        let mut scenario = easy_landing_scenario();
        scenario.world.landing_pads.push(LandingPadSpec {
            id: "source".to_owned(),
            center_x_m: -200.0,
            surface_y_m: 50.0,
            width_m: 36.0,
        });
        scenario.mission.transfer_route = Some(TransferRouteSpec {
            source_pad_id: "source".to_owned(),
            target_pad_id: "pad_a".to_owned(),
            route_angle_deg: 0.0,
            route_radius_m: 200.0,
            waypoints: Vec::new(),
        });
        let samples = vec![
            transfer_sample(
                0,
                0.0,
                Vec2::new(-200.0, 50.0),
                Vec2::new(40.0, 20.0),
                100.0,
            ),
            transfer_sample(
                60,
                0.5,
                Vec2::new(-100.0, 90.0),
                Vec2::new(40.0, -5.0),
                96.0,
            ),
            transfer_sample(120, 1.0, Vec2::new(0.0, 0.0), Vec2::new(0.0, -10.0), 94.0),
        ];
        let updates = vec![
            transfer_update(0, 0.0, "boost", 200.0, 80.0, 1.0),
            transfer_update(60, 0.5, "boost", 100.0, 20.0, 0.8),
            transfer_update(120, 1.0, "terminal", 0.0, 0.0, 0.0),
        ];

        let metrics = transfer_shape_metrics(&scenario, &samples, &updates)
            .expect("transfer shape metrics should be available");

        assert!((metrics.curve_rmse_m - 12.124_355_65).abs() < 1e-6);
        assert!((metrics.apex_error_m - 4.0).abs() < 1e-9);
        assert_eq!(metrics.projected_dx_abs_mean_m, Some(50.0));
        assert_eq!(metrics.projected_dx_abs_max_m, Some(80.0));
        assert_eq!(metrics.shortfall_ratio, Some(1.0));
    }

    #[test]
    fn transfer_burn_metrics_use_last_boost_when_no_cutoff_exists() {
        let samples = vec![
            transfer_sample(
                0,
                0.0,
                Vec2::new(-200.0, 50.0),
                Vec2::new(40.0, 20.0),
                100.0,
            ),
            transfer_sample(
                60,
                0.5,
                Vec2::new(-100.0, 90.0),
                Vec2::new(40.0, -5.0),
                96.0,
            ),
        ];
        let updates = vec![
            transfer_update(0, 0.0, "boost", 200.0, 80.0, 1.0),
            transfer_update(60, 0.5, "boost", 100.0, 20.0, 0.5),
        ];

        let metrics = transfer_review_metrics(&updates, &samples);

        assert_eq!(metrics.final_phase.as_deref(), Some("boost"));
        assert_eq!(metrics.boost_cutoff_time_s, None);
        assert_eq!(metrics.boost_burn_duration_s, Some(0.5));
        assert_eq!(metrics.boost_burn_fuel_used_kg, Some(4.0));
        assert_eq!(metrics.boost_burn_avg_throttle, Some(1.0));
    }

    #[test]
    fn transfer_review_metrics_capture_active_corridor_margin() {
        let mut updates = vec![
            transfer_update(0, 0.0, "boost", 200.0, 80.0, 1.0),
            transfer_update(60, 0.5, "boost", 100.0, 20.0, 1.0),
        ];
        updates[0].frame.metrics.insert(
            metric::TRANSFER_CORRIDOR_MODE.to_owned(),
            TelemetryValue::from("inactive"),
        );
        updates[0].frame.metrics.insert(
            metric::TRANSFER_CORRIDOR_MARGIN_M.to_owned(),
            TelemetryValue::from(1.0e9),
        );
        updates[1].frame.metrics.insert(
            metric::TRANSFER_CORRIDOR_MODE.to_owned(),
            TelemetryValue::from("active"),
        );
        updates[1].frame.metrics.insert(
            metric::TRANSFER_CORRIDOR_MARGIN_M.to_owned(),
            TelemetryValue::from(-32.0),
        );

        let metrics = transfer_review_metrics(&updates, &[]);

        assert_eq!(metrics.corridor_mode.as_deref(), Some("active"));
        assert_eq!(metrics.corridor_min_margin_m, Some(-32.0));
    }

    #[test]
    fn transfer_review_metrics_preserve_any_deferred_gate() {
        let mut updates = vec![
            transfer_update(0, 0.0, "boost", 200.0, 80.0, 1.0),
            transfer_update(60, 0.5, "terminal", 20.0, 0.0, 0.0),
        ];
        updates[0].frame.metrics.insert(
            metric::TRANSFER_TERMINAL_GATE_MODE.to_owned(),
            TelemetryValue::from("pending"),
        );
        updates[0].frame.metrics.insert(
            metric::TRANSFER_TERMINAL_GATE_DEFERRED.to_owned(),
            TelemetryValue::from(true),
        );
        updates[1].frame.metrics.insert(
            metric::TRANSFER_TERMINAL_GATE_MODE.to_owned(),
            TelemetryValue::from("latest_safe"),
        );
        updates[1].frame.metrics.insert(
            metric::TRANSFER_TERMINAL_GATE_DEFERRED.to_owned(),
            TelemetryValue::from(false),
        );

        let metrics = transfer_review_metrics(&updates, &[]);

        assert_eq!(metrics.terminal_gate_mode.as_deref(), Some("latest_safe"));
        assert_eq!(metrics.terminal_gate_deferred, Some(true));
    }

    #[test]
    fn transfer_review_metrics_marks_direct_terminal_entry() {
        let mut updates = vec![transfer_update(60, 0.5, "terminal", 120.0, 20.0, 0.0)];
        updates[0].frame.metrics.insert(
            metric::TRANSFER_TERMINAL_GATE_MODE.to_owned(),
            TelemetryValue::from("pending"),
        );

        let metrics = transfer_review_metrics(&updates, &[]);

        assert_eq!(metrics.terminal_entry_kind.as_deref(), Some("direct"));
        assert_eq!(
            metrics.terminal_handoff_gate_mode.as_deref(),
            Some("pending")
        );
    }

    #[test]
    fn transfer_review_metrics_capture_terminal_handoff() {
        let updates = vec![
            pd_control::ControllerUpdateRecord {
                sim_time_s: 1.0,
                physics_step: 120,
                controller_update_index: 0,
                compute_time_us: None,
                frame: pd_control::ControllerFrame {
                    command: pd_core::Command::idle(),
                    status: "boosting".to_owned(),
                    phase: Some("boost".to_owned()),
                    metrics: BTreeMap::from([
                        (
                            metric::TRANSFER_PHASE.to_owned(),
                            TelemetryValue::from("boost"),
                        ),
                        (
                            metric::TRANSFER_PROJECTED_DX_M.to_owned(),
                            TelemetryValue::from(42.0),
                        ),
                        (
                            metric::TRANSFER_IMPACT_ANGLE_DEG.to_owned(),
                            TelemetryValue::from(61.0),
                        ),
                        (
                            metric::TRANSFER_APEX_OVER_TARGET_M.to_owned(),
                            TelemetryValue::from(120.0),
                        ),
                        (
                            metric::TRANSFER_BOOST_QUALITY.to_owned(),
                            TelemetryValue::from("pass"),
                        ),
                    ]),
                    markers: Vec::new(),
                },
            },
            pd_control::ControllerUpdateRecord {
                sim_time_s: 3.5,
                physics_step: 420,
                controller_update_index: 1,
                compute_time_us: None,
                frame: pd_control::ControllerFrame {
                    command: pd_core::Command::idle(),
                    status: "terminal handoff".to_owned(),
                    phase: Some("terminal".to_owned()),
                    metrics: BTreeMap::from([
                        (
                            metric::TRANSFER_PHASE.to_owned(),
                            TelemetryValue::from("terminal"),
                        ),
                        (metric::TARGET_DX_M.to_owned(), TelemetryValue::from(120.0)),
                        (
                            metric::HEIGHT_ABOVE_TARGET_M.to_owned(),
                            TelemetryValue::from(80.0),
                        ),
                        (
                            metric::VERTICAL_SPEED_MPS.to_owned(),
                            TelemetryValue::from(-12.0),
                        ),
                        (
                            metric::TANGENTIAL_SPEED_MPS.to_owned(),
                            TelemetryValue::from(5.0),
                        ),
                        (
                            metric::TRANSFER_TERMINAL_GATE_MODE.to_owned(),
                            TelemetryValue::from("pending"),
                        ),
                        (
                            metric::TRANSFER_PROJECTED_DX_M.to_owned(),
                            TelemetryValue::from(-18.0),
                        ),
                        (
                            metric::TRANSFER_IMPACT_ANGLE_DEG.to_owned(),
                            TelemetryValue::from(47.0),
                        ),
                        (
                            metric::TRANSFER_BOOST_QUALITY.to_owned(),
                            TelemetryValue::from("angle"),
                        ),
                        (
                            metric::TRANSFER_TERMINAL_GATE_LATEST_SAFE_MARGIN_S.to_owned(),
                            TelemetryValue::from(1.25),
                        ),
                        (
                            metric::TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO.to_owned(),
                            TelemetryValue::from(0.83),
                        ),
                    ]),
                    markers: Vec::new(),
                },
            },
        ];

        let metrics = transfer_review_metrics(&updates, &[]);

        assert_eq!(metrics.final_phase.as_deref(), Some("terminal"));
        assert_eq!(metrics.terminal_entry_kind.as_deref(), Some("handoff"));
        assert_eq!(metrics.terminal_handoff_time_s, Some(3.5));
        assert_eq!(metrics.terminal_handoff_dx_m, Some(120.0));
        assert_eq!(metrics.terminal_handoff_height_m, Some(80.0));
        assert_eq!(metrics.terminal_handoff_speed_mps, Some(13.0));
        assert_eq!(
            metrics.terminal_handoff_gate_mode.as_deref(),
            Some("pending")
        );
        assert_eq!(metrics.terminal_handoff_projected_dx_m, Some(-18.0));
        assert_eq!(metrics.terminal_handoff_impact_angle_deg, Some(47.0));
        assert_eq!(
            metrics.terminal_handoff_boost_quality.as_deref(),
            Some("angle")
        );
        assert_eq!(metrics.terminal_handoff_latest_safe_margin_s, Some(1.25));
        assert_eq!(metrics.terminal_handoff_required_accel_ratio, Some(0.83));
        assert_eq!(metrics.boost_projected_dx_m, Some(42.0));
        assert_eq!(metrics.boost_impact_angle_deg, Some(61.0));
        assert_eq!(metrics.boost_apex_over_target_m, Some(120.0));
        assert_eq!(metrics.boost_quality.as_deref(), Some("pass"));
        assert_eq!(metrics.boost_cutoff_time_s, Some(1.0));
        assert_eq!(metrics.boost_cutoff_quality.as_deref(), Some("pass"));
    }

    #[test]
    fn transfer_review_metrics_capture_post_handoff_apex() {
        let updates = vec![
            transfer_update(60, 0.5, "boost", 200.0, 80.0, 1.0),
            transfer_update(120, 1.0, "terminal", 120.0, 20.0, 0.8),
        ];
        let samples = vec![
            transfer_sample(
                120,
                1.0,
                Vec2::new(-120.0, 80.0),
                Vec2::new(40.0, 15.0),
                95.0,
            ),
            transfer_sample(
                240,
                2.0,
                Vec2::new(-60.0, 110.0),
                Vec2::new(25.0, 0.0),
                90.0,
            ),
            transfer_sample(
                360,
                3.0,
                Vec2::new(-10.0, 100.0),
                Vec2::new(10.0, -10.0),
                88.0,
            ),
        ];

        let metrics = transfer_review_metrics(&updates, &samples);

        assert_eq!(metrics.terminal_post_handoff_apex_gain_m, Some(30.0));
        assert_eq!(metrics.terminal_post_handoff_time_to_apex_s, Some(1.0));
        assert_eq!(metrics.terminal_post_handoff_apex_dx_abs_m, Some(60.0));
    }

    #[test]
    fn transfer_matrix_entry_rejects_duplicate_route_angle_selectors() {
        let pack = ScenarioPackSpec {
            id: "transfer_matrix_duplicate_routes".to_owned(),
            name: "Transfer matrix duplicate routes".to_owned(),
            description: "transfer matrix duplicate routes".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_selected_routes".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                route_angles: vec!["r00".to_owned(), "r00".to_owned()],
                radius_tiers: Vec::new(),
                waypoint_profile: None,
                waypoint_handoff_envelope: None,
                evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
                adjustments: Vec::new(),
                tags: Vec::new(),
                metadata: BTreeMap::new(),
            })],
        };

        let message = validate_pack(&pack).unwrap_err().to_string();

        assert!(message.contains("duplicate route_angle selector"));
    }

    #[test]
    fn transfer_matrix_entry_rejects_duplicate_radius_tier_selectors() {
        let pack = ScenarioPackSpec {
            id: "transfer_matrix_duplicate_radii".to_owned(),
            name: "Transfer matrix duplicate radii".to_owned(),
            description: "transfer matrix duplicate radii".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_selected_radii".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                route_angles: Vec::new(),
                radius_tiers: vec!["short".to_owned(), "short".to_owned()],
                waypoint_profile: None,
                waypoint_handoff_envelope: None,
                evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
                adjustments: Vec::new(),
                tags: Vec::new(),
                metadata: BTreeMap::new(),
            })],
        };

        let message = validate_pack(&pack).unwrap_err().to_string();

        assert!(message.contains("duplicate radius_tier selector"));
    }

    #[test]
    fn transfer_matrix_entry_rejects_unknown_radius_tier_selector() {
        let pack = ScenarioPackSpec {
            id: "transfer_matrix_unknown_radius".to_owned(),
            name: "Transfer matrix unknown radius".to_owned(),
            description: "transfer matrix unknown radius".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                id: "transfer_guidance_unknown_radius".to_owned(),
                transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TransferMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "transfer_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TransferSeedTier::Smoke,
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                route_angles: Vec::new(),
                radius_tiers: vec!["extreme".to_owned()],
                waypoint_profile: None,
                waypoint_handoff_envelope: None,
                evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
                adjustments: Vec::new(),
                tags: Vec::new(),
                metadata: BTreeMap::new(),
            })],
        };

        let message = validate_pack(&pack).unwrap_err().to_string();

        assert!(message.contains("radius_tier selector 'extreme' is not supported"));
    }

    #[test]
    fn terminal_matrix_eval_timeout_must_not_shorten_reachability_window() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_short_eval_timeout".to_owned(),
            name: "Terminal matrix short eval timeout".to_owned(),
            description: "terminal matrix short eval timeout".to_owned(),
            terminal_matrix_max_time_s: Some(45.0),
            entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_clean_nominal".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "staged".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "clean".to_owned(),
                vehicle_variant: "nominal".to_owned(),
                expectation_tier: "core".to_owned(),
                arc_points: Vec::new(),
                adjustments: Vec::new(),
                tags: vec!["terminal".to_owned(), "smoke".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let error = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("terminal_matrix_max_time_s"));
        assert!(message.contains("must be >= scenario reachability max_time_s"));
    }

    #[test]
    fn terminal_matrix_projected_error_conditions_resolve_exact_engine_off_miss() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_traj_error".to_owned(),
            name: "Terminal matrix trajectory error".to_owned(),
            description: "terminal matrix trajectory error".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_traj_overshoot_small_half".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "terminal_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "traj_overshoot_small".to_owned(),
                vehicle_variant: "half".to_owned(),
                expectation_tier: "core".to_owned(),
                arc_points: Vec::new(),
                adjustments: vec![NumericAdjustmentSpec {
                    id: "payload_half_mass_kg".to_owned(),
                    path: "vehicle.dry_mass_kg".to_owned(),
                    mode: NumericPerturbationMode::Offset,
                    value: 2250.0,
                }],
                tags: vec!["terminal".to_owned(), "traj_error".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();
        let record = resolved_runs
            .iter()
            .find(|run| {
                run.descriptor.selector.arc_point == "a30"
                    && run.descriptor.selector.velocity_band == "mid"
                    && run.descriptor.resolved_seed == 0
            })
            .expect("projected-error matrix record present");

        let params = &record.descriptor.resolved_parameters;
        let start_x = params["start_x_m"];
        let start_vx = params["start_vx_mps"];
        let ttg = params["ttg_s"];
        let impact_x = start_x + (start_vx * ttg);

        assert_eq!(
            record.descriptor.selector.condition_set,
            "traj_overshoot_small"
        );
        assert_eq!(
            record
                .scenario
                .metadata
                .get("resolved.traj_error_kind")
                .map(String::as_str),
            Some("overshoot")
        );
        assert_eq!(params["projected_dx_error_mag_m"], 30.0);
        assert!((impact_x - params["projected_dx_error_m"]).abs() < 1e-9);
        assert!((params["speed_scale"] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn terminal_matrix_reactive_terrain_conditions_resolve_geometry() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_reactive_terrain".to_owned(),
            name: "Terminal matrix reactive terrain".to_owned(),
            description: "terminal matrix reactive terrain".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![
                ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                    id: "terminal_guidance_terrain_backstop_wall_half".to_owned(),
                    terminal_matrix: "half_arc_terminal_v1".to_owned(),
                    base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    lanes: vec![TerminalMatrixLaneSpec {
                        id: "current".to_owned(),
                        controller: "terminal_pdg".to_owned(),
                        controller_config: None,
                    }],
                    seed_tier: TerminalSeedTier::Smoke,
                    condition_set: "terrain_backstop_wall".to_owned(),
                    vehicle_variant: "half".to_owned(),
                    expectation_tier: "core".to_owned(),
                    arc_points: Vec::new(),
                    adjustments: vec![NumericAdjustmentSpec {
                        id: "payload_half_mass_kg".to_owned(),
                        path: "vehicle.dry_mass_kg".to_owned(),
                        mode: NumericPerturbationMode::Offset,
                        value: 2250.0,
                    }],
                    tags: vec!["terminal".to_owned(), "terrain".to_owned()],
                    metadata: BTreeMap::new(),
                }),
                ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                    id: "terminal_guidance_terrain_clip_half".to_owned(),
                    terminal_matrix: "half_arc_terminal_v1".to_owned(),
                    base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    lanes: vec![TerminalMatrixLaneSpec {
                        id: "current".to_owned(),
                        controller: "terminal_pdg".to_owned(),
                        controller_config: None,
                    }],
                    seed_tier: TerminalSeedTier::Smoke,
                    condition_set: "terrain_clip".to_owned(),
                    vehicle_variant: "half".to_owned(),
                    expectation_tier: "core".to_owned(),
                    arc_points: Vec::new(),
                    adjustments: vec![NumericAdjustmentSpec {
                        id: "payload_half_mass_kg".to_owned(),
                        path: "vehicle.dry_mass_kg".to_owned(),
                        mode: NumericPerturbationMode::Offset,
                        value: 2250.0,
                    }],
                    tags: vec!["terminal".to_owned(), "terrain".to_owned()],
                    metadata: BTreeMap::new(),
                }),
            ],
        };

        let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();
        let backstop = resolved_runs
            .iter()
            .find(|run| {
                run.descriptor.entry_id == "terminal_guidance_terrain_backstop_wall_half"
                    && run.descriptor.selector.arc_point == "a30"
                    && run.descriptor.selector.velocity_band == "mid"
                    && run.descriptor.resolved_seed == 0
            })
            .expect("backstop terrain matrix record present");
        let backstop_params = &backstop.descriptor.resolved_parameters;
        let backstop_feature_side = backstop_params["terrain_feature_side_sign"];
        let backstop_plateau_x = backstop_feature_side
            * (backstop_params["terrain_inner_offset_m"]
                + backstop_params["terrain_shoulder_width_m"]
                + (0.5 * backstop_params["terrain_top_width_m"]));

        assert_eq!(
            backstop
                .scenario
                .metadata
                .get("resolved.condition_kind")
                .map(String::as_str),
            Some("reactive_terrain")
        );
        assert_eq!(
            backstop
                .scenario
                .metadata
                .get("resolved.hazard_driver")
                .map(String::as_str),
            Some("containment_backstop")
        );
        assert_eq!(
            backstop
                .scenario
                .metadata
                .get("resolved.reactive_contract")
                .map(String::as_str),
            Some("execution_guardrail")
        );
        assert_eq!(
            backstop
                .scenario
                .metadata
                .get("resolved.terrain_variant")
                .map(String::as_str),
            Some("wall")
        );
        assert_eq!(backstop_params["terrain_height_offset_m"], 400.0);
        assert_eq!(backstop_feature_side, 1.0);
        assert_eq!(backstop.scenario.world.terrain.sample_height(0.0), 0.0);
        assert!(
            (backstop
                .scenario
                .world
                .terrain
                .sample_height(backstop_plateau_x)
                - backstop_params["terrain_height_offset_m"])
                .abs()
                < 1e-9
        );
        assert_eq!(
            backstop
                .scenario
                .world
                .terrain
                .sample_height(-backstop_plateau_x),
            0.0
        );

        let clip = resolved_runs
            .iter()
            .find(|run| {
                run.descriptor.entry_id == "terminal_guidance_terrain_clip_half"
                    && run.descriptor.selector.arc_point == "a30"
                    && run.descriptor.selector.velocity_band == "mid"
                    && run.descriptor.resolved_seed == 0
            })
            .expect("clip terrain matrix record present");
        let clip_params = &clip.descriptor.resolved_parameters;
        let clip_feature_side = clip_params["terrain_feature_side_sign"];
        let clip_plateau_x = clip_feature_side
            * (clip_params["terrain_inner_offset_m"]
                + clip_params["terrain_shoulder_width_m"]
                + (0.5 * clip_params["terrain_top_width_m"]));

        assert_eq!(
            clip.scenario
                .metadata
                .get("resolved.hazard_driver")
                .map(String::as_str),
            Some("descent_clip")
        );
        assert_eq!(
            clip.scenario
                .metadata
                .get("resolved.obstacle_placement")
                .map(String::as_str),
            Some("terminal_approach")
        );
        assert_eq!(clip_feature_side, -1.0);
        assert_eq!(clip.scenario.world.terrain.sample_height(0.0), 0.0);
        assert!(
            (clip.scenario.world.terrain.sample_height(clip_plateau_x)
                - clip_params["terrain_height_offset_m"])
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn terminal_matrix_entry_rejects_unknown_condition_set() {
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_unknown_condition".to_owned(),
            name: "Terminal matrix unknown condition".to_owned(),
            description: "terminal matrix unknown condition".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_traj_typo_half".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "terminal_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "traj_overshot_small".to_owned(),
                vehicle_variant: "half".to_owned(),
                expectation_tier: "core".to_owned(),
                arc_points: Vec::new(),
                adjustments: vec![],
                tags: vec![],
                metadata: BTreeMap::new(),
            })],
        };

        let err = validate_pack(&pack).expect_err("unknown condition set should be rejected");

        assert!(err.to_string().contains("unsupported condition_set"));
    }

    #[test]
    fn analytic_vertical_bound_invalidates_heavy_cargo_vertical_high_cases() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_heavy_cargo_full".to_owned(),
            name: "Terminal matrix heavy cargo full".to_owned(),
            description: "terminal matrix heavy cargo full".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_clean_heavy_cargo".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "terminal_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Full,
                condition_set: "clean".to_owned(),
                vehicle_variant: "heavy_cargo".to_owned(),
                expectation_tier: "core".to_owned(),
                arc_points: Vec::new(),
                adjustments: vec![NumericAdjustmentSpec {
                    id: "payload_full_mass_kg".to_owned(),
                    path: "vehicle.dry_mass_kg".to_owned(),
                    mode: NumericPerturbationMode::Offset,
                    value: 4500.0,
                }],
                tags: vec!["terminal".to_owned(), "analytic".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let report = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
        let record = report
            .records
            .iter()
            .find(|record| {
                record.resolved.selector.arc_point == "a00"
                    && record.resolved.selector.velocity_band == "high"
                    && record.resolved.resolved_seed == 0
            })
            .expect("heavy cargo a00 high seed 0 should be present");

        assert_eq!(record.analytic.class, BatchRunAnalyticClass::Impossible);
        assert_eq!(
            record.analytic.reason,
            Some(BatchRunAnalyticReason::VerticalStopHeight)
        );
        assert!(
            record
                .analytic
                .stop_height_margin_m
                .expect("vertical stop margin should be present")
                < 0.0
        );
        assert!(report.summary.invalidated_runs >= 24);
        assert!(
            report
                .summary
                .by_entry
                .iter()
                .find(|group| group.key == "terminal_guidance_clean_heavy_cargo")
                .expect("entry summary should exist")
                .invalidated_runs
                >= 24
        );
    }

    #[test]
    fn analytic_authority_frontier_marks_low_thrust_high_energy_cases_but_keeps_them_scored() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_full_payload_lateral".to_owned(),
            name: "Terminal matrix full payload lateral".to_owned(),
            description: "terminal matrix full payload lateral".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![
                ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                    id: "terminal_guidance_clean_half_payload".to_owned(),
                    terminal_matrix: "half_arc_terminal_v1".to_owned(),
                    base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    lanes: vec![TerminalMatrixLaneSpec {
                        id: "current".to_owned(),
                        controller: "terminal_pdg".to_owned(),
                        controller_config: None,
                    }],
                    seed_tier: TerminalSeedTier::Smoke,
                    condition_set: "clean".to_owned(),
                    vehicle_variant: "half".to_owned(),
                    expectation_tier: "core".to_owned(),
                    arc_points: Vec::new(),
                    adjustments: vec![NumericAdjustmentSpec {
                        id: "payload_half_mass_kg".to_owned(),
                        path: "vehicle.dry_mass_kg".to_owned(),
                        mode: NumericPerturbationMode::Offset,
                        value: 2250.0,
                    }],
                    tags: vec!["terminal".to_owned(), "analytic".to_owned()],
                    metadata: BTreeMap::new(),
                }),
                ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                    id: "terminal_guidance_clean_full_payload".to_owned(),
                    terminal_matrix: "half_arc_terminal_v1".to_owned(),
                    base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    lanes: vec![TerminalMatrixLaneSpec {
                        id: "current".to_owned(),
                        controller: "terminal_pdg".to_owned(),
                        controller_config: None,
                    }],
                    seed_tier: TerminalSeedTier::Full,
                    condition_set: "clean".to_owned(),
                    vehicle_variant: "full".to_owned(),
                    expectation_tier: "stress".to_owned(),
                    arc_points: Vec::new(),
                    adjustments: vec![NumericAdjustmentSpec {
                        id: "payload_full_mass_kg".to_owned(),
                        path: "vehicle.dry_mass_kg".to_owned(),
                        mode: NumericPerturbationMode::Offset,
                        value: 4500.0,
                    }],
                    tags: vec!["terminal".to_owned(), "analytic".to_owned()],
                    metadata: BTreeMap::new(),
                }),
            ],
        };

        let report = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
        let half_a45_high = report
            .records
            .iter()
            .find(|record| {
                record.resolved.entry_id == "terminal_guidance_clean_half_payload"
                    && record.resolved.selector.arc_point == "a45"
                    && record.resolved.selector.velocity_band == "high"
                    && record.resolved.resolved_seed == 0
            })
            .expect("half payload a45 high seed 0 should be present");
        let full_a45_high = report
            .records
            .iter()
            .find(|record| {
                record.resolved.entry_id == "terminal_guidance_clean_full_payload"
                    && record.resolved.selector.arc_point == "a45"
                    && record.resolved.selector.velocity_band == "high"
                    && record.resolved.resolved_seed == 0
            })
            .expect("full payload a45 high seed 0 should be present");
        let full_a80_high_frontier = report
            .records
            .iter()
            .find(|record| {
                record.resolved.entry_id == "terminal_guidance_clean_full_payload"
                    && record.resolved.selector.arc_point == "a80"
                    && record.resolved.selector.velocity_band == "high"
                    && record.resolved.resolved_seed == 3
            })
            .expect("full payload a80 high seed 3 should be present");

        assert_eq!(half_a45_high.analytic.class, BatchRunAnalyticClass::Scored);
        assert_eq!(
            full_a45_high.manifest.mission_outcome,
            MissionOutcome::FailedCrash
        );
        assert_eq!(
            full_a45_high.analytic.class,
            BatchRunAnalyticClass::Frontier
        );
        assert_eq!(
            full_a45_high.analytic.reason,
            Some(BatchRunAnalyticReason::LowThrustHighEnergy)
        );
        assert!(
            full_a45_high
                .analytic
                .stop_accel_margin_mps2
                .expect("reachability margin should be present")
                > 0.0
        );
        assert_eq!(
            full_a80_high_frontier.analytic.class,
            BatchRunAnalyticClass::Frontier
        );
        assert_eq!(
            full_a80_high_frontier.analytic.reason,
            Some(BatchRunAnalyticReason::LowThrustHighEnergy)
        );

        let impossible_runs = report
            .records
            .iter()
            .filter(|record| matches!(record.analytic.class, BatchRunAnalyticClass::Impossible))
            .count();
        let frontier_success_runs = report
            .records
            .iter()
            .filter(|record| {
                matches!(record.analytic.class, BatchRunAnalyticClass::Frontier)
                    && matches!(record.manifest.mission_outcome, MissionOutcome::Success)
            })
            .count();
        let frontier_failure_runs = report
            .records
            .iter()
            .filter(|record| {
                matches!(record.analytic.class, BatchRunAnalyticClass::Frontier)
                    && !matches!(record.manifest.mission_outcome, MissionOutcome::Success)
            })
            .count();

        assert!(frontier_failure_runs > 0);
        assert_eq!(report.summary.invalidated_runs, impossible_runs);
        assert!(report.summary.success_runs >= frontier_success_runs);
        assert!(report.summary.failure_runs >= frontier_failure_runs);
        assert_eq!(
            report.summary.success_runs
                + report.summary.failure_runs
                + report.summary.invalidated_runs,
            report.summary.total_runs
        );
    }

    #[test]
    fn analytic_transfer_frontier_marks_only_near_vertical_uphill_routes() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "transfer_frontier_classification".to_owned(),
            name: "Transfer frontier classification".to_owned(),
            description: "transfer frontier classification".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![
                ScenarioPackEntry::TransferMatrix(TransferMatrixEntry {
                    id: "transfer_guidance_route_angles_empty".to_owned(),
                    transfer_matrix: "signed_route_arc_transfer_v1".to_owned(),
                    base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    lanes: vec![TransferMatrixLaneSpec {
                        id: "current".to_owned(),
                        controller: "transfer_pdg".to_owned(),
                        controller_config: None,
                    }],
                    seed_tier: TransferSeedTier::Smoke,
                    vehicle_variant: "empty".to_owned(),
                    expectation_tier: "reference".to_owned(),
                    route_angles: vec!["r-80".to_owned(), "r+60".to_owned(), "r+80".to_owned()],
                    radius_tiers: Vec::new(),
                    waypoint_profile: None,
                    waypoint_handoff_envelope: None,
                    evaluation_goal: TransferMatrixEvaluationGoal::LandingOnPad,
                    adjustments: Vec::new(),
                    tags: vec!["transfer".to_owned(), "analytic".to_owned()],
                    metadata: BTreeMap::new(),
                }),
                ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                    id: "terminal_guidance_clean_empty".to_owned(),
                    terminal_matrix: "half_arc_terminal_v1".to_owned(),
                    base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    lanes: vec![TerminalMatrixLaneSpec {
                        id: "current".to_owned(),
                        controller: "terminal_pdg".to_owned(),
                        controller_config: None,
                    }],
                    seed_tier: TerminalSeedTier::Smoke,
                    condition_set: "clean".to_owned(),
                    vehicle_variant: "empty".to_owned(),
                    expectation_tier: "reference".to_owned(),
                    arc_points: vec!["a80".to_owned()],
                    adjustments: Vec::new(),
                    tags: vec!["terminal".to_owned(), "analytic".to_owned()],
                    metadata: BTreeMap::new(),
                }),
            ],
        };

        let resolved = resolve_pack_runs(&pack, &base_dir).unwrap();
        let analytic_for = |mission: &str, route_or_arc: &str| {
            let run = resolved
                .iter()
                .find(|run| {
                    run.descriptor.resolved_seed == 0
                        && run.descriptor.selector.mission == mission
                        && (run.descriptor.selector.route_angle == route_or_arc
                            || run.descriptor.selector.arc_point == route_or_arc)
                })
                .expect("resolved run should exist");
            analytic_feasibility_for_run(run)
        };

        let uphill_r80 = analytic_for("transfer_guidance", "r+80");
        assert_eq!(uphill_r80.class, BatchRunAnalyticClass::Frontier);
        assert_eq!(
            uphill_r80.reason,
            Some(BatchRunAnalyticReason::NearVerticalTransferRoute)
        );
        assert!(uphill_r80.is_scored());

        assert_eq!(
            analytic_for("transfer_guidance", "r+60").class,
            BatchRunAnalyticClass::Scored
        );
        assert_eq!(
            analytic_for("transfer_guidance", "r-80").class,
            BatchRunAnalyticClass::Scored
        );
        assert_eq!(
            analytic_for("terminal_guidance", "a80").reason,
            None,
            "terminal a80 should not inherit transfer-route frontier policy"
        );
    }

    #[test]
    fn analytic_coupled_bound_does_not_invalidate_projected_error_successes() {
        let base_dir = fixtures_root();
        let pack = ScenarioPackSpec {
            id: "terminal_matrix_projected_error_coupled_bound".to_owned(),
            name: "Terminal matrix projected error coupled bound".to_owned(),
            description: "terminal matrix projected error coupled bound".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                id: "terminal_guidance_traj_overshoot_large_half".to_owned(),
                terminal_matrix: "half_arc_terminal_v1".to_owned(),
                base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                lanes: vec![TerminalMatrixLaneSpec {
                    id: "current".to_owned(),
                    controller: "terminal_pdg".to_owned(),
                    controller_config: None,
                }],
                seed_tier: TerminalSeedTier::Smoke,
                condition_set: "traj_overshoot_large".to_owned(),
                vehicle_variant: "half".to_owned(),
                expectation_tier: "core".to_owned(),
                arc_points: Vec::new(),
                adjustments: vec![NumericAdjustmentSpec {
                    id: "payload_half_mass_kg".to_owned(),
                    path: "vehicle.dry_mass_kg".to_owned(),
                    mode: NumericPerturbationMode::Offset,
                    value: 2250.0,
                }],
                tags: vec!["terminal".to_owned(), "traj_error".to_owned()],
                metadata: BTreeMap::new(),
            })],
        };

        let report = run_pack_with_workers(&pack, &base_dir, None, 1).unwrap();
        let record = report
            .records
            .iter()
            .find(|record| {
                record.resolved.selector.arc_point == "a70"
                    && record.resolved.selector.velocity_band == "high"
                    && record.resolved.resolved_seed == 0
            })
            .expect("projected-error a70 high seed 0 should be present");

        assert_eq!(record.analytic.class, BatchRunAnalyticClass::Scored);
        assert!(
            record
                .analytic
                .stop_accel_margin_mps2
                .expect("coupled stop margin should be present")
                > 0.0
        );
        let mission_success_runs = report
            .records
            .iter()
            .filter(|record| matches!(record.manifest.mission_outcome, MissionOutcome::Success))
            .count();
        let impossible_non_success_runs = report
            .records
            .iter()
            .filter(|record| {
                !matches!(record.manifest.mission_outcome, MissionOutcome::Success)
                    && matches!(record.analytic.class, BatchRunAnalyticClass::Impossible)
            })
            .count();

        assert_eq!(report.summary.success_runs, mission_success_runs);
        assert_eq!(report.summary.invalidated_runs, impossible_non_success_runs);
    }

    #[test]
    fn terminal_matrix_entry_rejects_overwritten_adjustment_paths() {
        for path in ["world.gravity_mps2", "initial_state.position_m.x"] {
            let pack = ScenarioPackSpec {
                id: "terminal_matrix_invalid_adjustment".to_owned(),
                name: "Terminal matrix invalid adjustment".to_owned(),
                description: "terminal matrix invalid adjustment".to_owned(),
                terminal_matrix_max_time_s: None,
                entries: vec![ScenarioPackEntry::TerminalMatrix(TerminalMatrixEntry {
                    id: "terminal_guidance_clean_nominal".to_owned(),
                    terminal_matrix: "half_arc_terminal_v1".to_owned(),
                    base_scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    lanes: vec![TerminalMatrixLaneSpec {
                        id: "current".to_owned(),
                        controller: "staged".to_owned(),
                        controller_config: None,
                    }],
                    seed_tier: TerminalSeedTier::Smoke,
                    condition_set: "clean".to_owned(),
                    vehicle_variant: "nominal".to_owned(),
                    expectation_tier: "core".to_owned(),
                    arc_points: Vec::new(),
                    adjustments: vec![NumericAdjustmentSpec {
                        id: "invalid".to_owned(),
                        path: path.to_owned(),
                        mode: NumericPerturbationMode::Offset,
                        value: 1.0,
                    }],
                    tags: Vec::new(),
                    metadata: BTreeMap::new(),
                })],
            };

            let err = validate_pack(&pack).expect_err("path should be rejected");
            assert!(err.to_string().contains("uses unsupported path"), "{err}");
        }
    }

    #[test]
    fn missing_compare_skip_allows_unresolved_cache_ref() {
        let base_dir = temp_fixture_root("missing_compare_skip");
        write_scenario(
            &base_dir,
            "scenarios/checkpoint_success.json",
            &easy_checkpoint_scenario(),
        );
        let pack = ScenarioPackSpec {
            id: "missing_compare_skip".to_owned(),
            name: "Missing compare skip".to_owned(),
            description: "missing compare skip".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "checkpoint_success_idle".to_owned(),
                scenario: "scenarios/checkpoint_success.json".to_owned(),
                controller: "idle".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            })],
        };
        let resolved_runs = resolve_pack_runs(&pack, &base_dir).unwrap();
        let identity = batch_identity_for_pack(&pack, &resolved_runs).unwrap();
        let provenance = BatchCompareProvenance {
            source: BatchCompareSource::CacheRef,
            requested_ref: Some("auto".to_owned()),
            resolved_ref: None,
            baseline_dir: None,
            status: BatchCompareResolutionStatus::Missing,
            note: Some("no compare cache ref could be resolved".to_owned()),
        };

        let (resolved_provenance, baseline) =
            load_requested_baseline(&pack, &identity, provenance, MissingComparePolicy::Skip)
                .unwrap();

        assert!(baseline.is_none());
        assert_eq!(
            resolved_provenance.status,
            BatchCompareResolutionStatus::Missing
        );
    }

    #[test]
    fn cached_batch_validation_rejects_schema_mismatch() {
        let base_dir = temp_fixture_root("cache_schema");
        write_scenario(
            &base_dir,
            "scenarios/checkpoint_success.json",
            &easy_checkpoint_scenario(),
        );
        let output_dir = base_dir.join("cache_output");
        let pack = ScenarioPackSpec {
            id: "cache_schema".to_owned(),
            name: "Cache schema".to_owned(),
            description: "cache schema".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "checkpoint_success_idle".to_owned(),
                scenario: "scenarios/checkpoint_success.json".to_owned(),
                controller: "idle".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            })],
        };

        let report = run_pack_with_workers(&pack, &base_dir, Some(&output_dir), 1).unwrap();
        report::write_batch_report_artifacts(&output_dir, &report, None).unwrap();
        write_json(
            &output_dir.join("meta.json"),
            &BatchCacheMeta {
                schema_version: BATCH_REPORT_SCHEMA_VERSION - 1,
                pack_id: report.pack_id.clone(),
                pack_name: report.pack_name.clone(),
                identity: report.identity.clone(),
                total_runs: report.total_runs,
                workers_used: report.workers_used,
                cache: BatchCacheInfo {
                    workspace_key: "unit".to_owned(),
                    commit_key: "unit".to_owned(),
                    batch_stem: "cache_schema".to_owned(),
                    cache_dir: output_dir.to_string_lossy().into_owned(),
                    status: BatchCacheStatus::Fresh,
                    created_at_unix_s: current_unix_timestamp(),
                    promotion: None,
                },
            },
        )
        .unwrap();

        assert!(
            validate_cached_batch_dir(&output_dir, &pack, &report.identity)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn stable_output_runs_tree_is_refreshed_from_cache() {
        let base_dir = temp_fixture_root("stable_output_runs");
        write_scenario(
            &base_dir,
            "scenarios/checkpoint_success.json",
            &easy_checkpoint_scenario(),
        );
        let output_dir = base_dir.join("stable_output");
        fs::create_dir_all(output_dir.join("runs").join("stale_before"))
            .expect("stale runs dir should be creatable");
        let pack = ScenarioPackSpec {
            id: "stable_output_runs".to_owned(),
            name: "Stable output runs".to_owned(),
            description: "stable output runs".to_owned(),
            terminal_matrix_max_time_s: None,
            entries: vec![ScenarioPackEntry::Scenario(ConcreteScenarioPackEntry {
                id: "checkpoint_success_idle".to_owned(),
                scenario: "scenarios/checkpoint_success.json".to_owned(),
                controller: "idle".to_owned(),
                controller_config: None,
                metadata: BTreeMap::new(),
            })],
        };

        let outcome = run_pack_cached(
            &pack,
            &base_dir,
            Some(&output_dir),
            1,
            Some("none"),
            None,
            MissingComparePolicy::Skip,
            false,
        )
        .unwrap();
        let written = load_batch_report(&output_dir).unwrap();
        let expected_run_dir = output_dir.join("runs").join("checkpoint_success_idle");

        assert!(!output_dir.join("runs").join("stale_before").exists());
        assert!(expected_run_dir.exists());
        assert_eq!(
            written.records[0].bundle_dir.as_deref(),
            Some(expected_run_dir.to_string_lossy().as_ref())
        );
        assert_eq!(
            outcome.report.records[0].bundle_dir,
            Some(expected_run_dir.to_string_lossy().into_owned())
        );
    }
}
