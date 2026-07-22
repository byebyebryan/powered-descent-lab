use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use pd_control::ControllerSpec;
use pd_core::{RunManifest, RunSummary};
use serde::{Deserialize, Serialize};

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
    pub final_terminal_required_accel_ratio: Option<f64>,
    #[serde(default)]
    pub final_terminal_recoverable: Option<bool>,
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
    pub transfer_terminal_low_altitude_rebound_gain_m: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_low_altitude_rebound_origin_dx_abs_m: Option<f64>,
    #[serde(default)]
    pub transfer_terminal_low_altitude_rebound_near_pad: Option<bool>,
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

pub(crate) fn default_selector_value() -> String {
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
    pub(crate) fn id(&self) -> &str {
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
    pub(crate) fn as_str(self) -> &'static str {
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
    #[serde(default)]
    pub transfer_terminal_low_altitude_rebound_gain_m: Option<BatchMetricSummary>,
    #[serde(default)]
    pub transfer_terminal_low_altitude_rebound_origin_dx_abs_m: Option<BatchMetricSummary>,
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

#[derive(Clone, Copy, Debug)]
pub struct CachedBatchRunOptions<'a> {
    pub output_dir: Option<&'a Path>,
    pub workers: usize,
    pub compare_ref: Option<&'a str>,
    pub baseline_dir: Option<&'a Path>,
    pub missing_compare: MissingComparePolicy,
    pub reuse_cache: bool,
}
