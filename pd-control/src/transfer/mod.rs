use crate::guidance::{StateTargetRequest, allocate_accel_command, required_control_accel};
use crate::kit::{ControllerFrameBuilder, ControllerView, metric, standard_marker};
use crate::terminal::{
    TerminalEntryAssessment, TerminalEntryMode, TerminalEntryRequest, TerminalEntryTerrainPolicy,
    TerminalPdgController, TerminalPdgControllerConfig,
};
use crate::{Controller, ControllerFrame, TelemetryValue};
use pd_core::{
    Command, Observation, RunContext, TransferWaypointSpec, Vec2, WaypointHandoffKinematics,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod experimental;
mod waypoint;

use experimental::*;
use waypoint::*;

const TRANSFER_UPHILL_STEEP_TILT_SCALE: f64 = 0.25;
const TRANSFER_UPHILL_STEEP_TILT_MIN_RAD: f64 = 0.0;
const TRANSFER_UPHILL_LOW_CLEARANCE_M: f64 = 240.0;
const TRANSFER_UPHILL_CLEARANCE_BLEND_FLOOR_M: f64 = 20.0;
const TRANSFER_BOOST_APEX_THROTTLE_DEADBAND_M: f64 = 25.0;
const TRANSFER_BOOST_APEX_THROTTLE_RANGE_M: f64 = 160.0;
const TRANSFER_UPHILL_CORRIDOR_CLEARANCE_MARGIN_M: f64 = 35.0;
const TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD: f64 = 0.12;
const TRANSFER_UPHILL_CORRIDOR_LOOKAHEAD_FRAC: f64 = 0.35;
const TRANSFER_UPHILL_CORRIDOR_LOOKAHEAD_MIN_M: f64 = 35.0;
const TRANSFER_UPHILL_CORRIDOR_LOOKAHEAD_MAX_M: f64 = 160.0;
const TRANSFER_UPHILL_CORRIDOR_TILT_SLOPE_MIN: f64 = 1.25;
const TRANSFER_UPHILL_CORRIDOR_BRAKE_VX_MPS: f64 = 3.0;
const TRANSFER_CORRIDOR_SAMPLE_COUNT: usize = 24;
const TRANSFER_GATE_DEFER_MAX_NEGATIVE_MARGIN_S: f64 = -0.75;
const TRANSFER_PRE_TARGET_CAPTURE_MAX_LATEST_SAFE_MARGIN_S: f64 = 0.75;
const TRANSFER_PRE_TARGET_CAPTURE_LOOKAHEAD_S: f64 = 1.5;
const TRANSFER_SOURCE_CLEARANCE_SAMPLE_COUNT: usize = 16;
const WAYPOINT_LEG_LOOKAHEAD_TIME_S: f64 = 5.0;
const WAYPOINT_LEG_LOOKAHEAD_MIN_CAPTURE_RADII: f64 = 3.0;
const WAYPOINT_LEG_LOOKAHEAD_MAX_CAPTURE_RADII: f64 = 12.0;
const WAYPOINT_LEG_REMAINING_LOOKAHEAD_FRAC: f64 = 0.75;
const WAYPOINT_OUTBOUND_BLEND_START_CAPTURE_RADII: f64 = 8.0;
const WAYPOINT_OUTBOUND_TURN_MARGIN_CAPTURE_RADII: f64 = 2.0;
const WAYPOINT_APPROACH_TIME_TO_PLANE_MAX_S: f64 = 12.0;
const WAYPOINT_GUIDANCE_PATH_AUTHORITY_FRAC: f64 = 0.15;
const WAYPOINT_GUIDANCE_MIN_TIME_TO_GO_S: f64 = 0.5;
const WAYPOINT_GUIDANCE_MIN_CLOSURE_MPS: f64 = 1.0;
const WAYPOINT_GUIDANCE_L1_MIN_SPEED_MPS: f64 = 1.0;
const WAYPOINT_GUIDANCE_UNIQUE_EPS: f64 = 1.0e-6;
const WAYPOINT_GUIDANCE_ENVELOPE_EPS_MPS: f64 = 1.0e-6;
const WAYPOINT_GUIDANCE_TRIGGER_SCAN_STEPS: usize = 64;
const WAYPOINT_GUIDANCE_TRIGGER_BISECTION_STEPS: usize = 12;
const WAYPOINT_GUIDANCE_CONTRACT_FAILURE_HYSTERESIS_TICKS: u32 = 2;
const WAYPOINT_GUIDANCE_REPLAN_MATERIALITY_RATIO: f64 = 0.1;
const WAYPOINT_GUIDANCE_PREDICTION_HORIZON_S: f64 = 12.0;
const WAYPOINT_TAKEOFF_SPEED_RESPONSE_TIME_S: f64 = 1.0;
const WAYPOINT_TAKEOFF_SPEED_PER_INBOUND_DISTANCE_HZ: f64 = 0.08;
const WAYPOINT_JOINT_MAX_CURRENT_CANDIDATES: usize = 4;
const WAYPOINT_VIOLATION_HEADING: u8 = 1 << 0;
const WAYPOINT_VIOLATION_OUTBOUND_PROGRESS: u8 = 1 << 1;
const WAYPOINT_VIOLATION_OUTBOUND_CROSS_SPEED: u8 = 1 << 2;
const WAYPOINT_VIOLATION_SPEED: u8 = 1 << 3;
const WAYPOINT_VIOLATION_VERTICAL_SPEED: u8 = 1 << 4;

fn default_transfer_boost_projected_dx_limit_m() -> f64 {
    55.0
}

fn default_transfer_boost_descent_angle_min_deg() -> f64 {
    45.0
}

fn default_transfer_boost_descent_angle_target_deg() -> f64 {
    55.0
}

fn default_transfer_boost_apex_height_per_dx() -> f64 {
    0.18
}

fn default_transfer_boost_apex_height_per_uphill_dy() -> f64 {
    0.15
}

fn default_transfer_boost_apex_height_min_m() -> f64 {
    30.0
}

fn default_transfer_boost_apex_height_max_m() -> f64 {
    240.0
}

fn default_transfer_uphill_boost_dy_min_m() -> f64 {
    20.0
}

fn default_transfer_uphill_boost_tilt_rad() -> f64 {
    0.30
}

fn default_transfer_boost_candidate_horizon_s() -> f64 {
    3.0
}

fn default_transfer_boost_candidate_step_s() -> f64 {
    0.25
}

fn default_transfer_boost_settle_lookahead_s() -> f64 {
    0.35
}

fn default_transfer_boost_pathwise_scoring_enabled() -> bool {
    false
}

fn default_transfer_boost_recoverability_scoring_enabled() -> bool {
    false
}

fn default_transfer_waypoint_guidance_enabled() -> bool {
    false
}

fn default_transfer_gate_defer_lookahead_s() -> f64 {
    2.0
}

fn default_transfer_gate_defer_step_s() -> f64 {
    0.25
}

fn default_transfer_gate_defer_min_ratio_improvement() -> f64 {
    0.03
}

fn default_transfer_source_clearance_margin_m() -> f64 {
    24.0
}

fn default_transfer_source_clearance_lookahead_m() -> f64 {
    96.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransferPdgControllerConfig {
    pub takeoff_clearance_m: f64,
    pub takeoff_min_vertical_speed_mps: f64,
    pub max_takeoff_time_s: f64,
    #[serde(default = "default_transfer_source_clearance_margin_m")]
    pub source_clearance_margin_m: f64,
    #[serde(default = "default_transfer_source_clearance_lookahead_m")]
    pub source_clearance_lookahead_m: f64,
    pub boost_max_time_s: f64,
    pub boost_tilt_rad: f64,
    pub boost_speed_mps: f64,
    #[serde(default = "default_transfer_boost_projected_dx_limit_m")]
    pub boost_projected_dx_limit_m: f64,
    #[serde(default = "default_transfer_boost_descent_angle_min_deg")]
    pub boost_descent_angle_min_deg: f64,
    #[serde(default = "default_transfer_boost_descent_angle_target_deg")]
    pub boost_descent_angle_target_deg: f64,
    #[serde(default = "default_transfer_boost_apex_height_per_dx")]
    pub boost_apex_height_per_dx: f64,
    #[serde(default = "default_transfer_boost_apex_height_per_uphill_dy")]
    pub boost_apex_height_per_uphill_dy: f64,
    #[serde(default = "default_transfer_boost_apex_height_min_m")]
    pub boost_apex_height_min_m: f64,
    #[serde(default = "default_transfer_boost_apex_height_max_m")]
    pub boost_apex_height_max_m: f64,
    #[serde(default = "default_transfer_uphill_boost_dy_min_m")]
    pub uphill_boost_dy_min_m: f64,
    #[serde(default = "default_transfer_uphill_boost_tilt_rad")]
    pub uphill_boost_tilt_rad: f64,
    #[serde(default = "default_transfer_boost_candidate_horizon_s")]
    pub boost_candidate_horizon_s: f64,
    #[serde(default = "default_transfer_boost_candidate_step_s")]
    pub boost_candidate_step_s: f64,
    #[serde(default = "default_transfer_boost_settle_lookahead_s")]
    pub boost_settle_lookahead_s: f64,
    #[serde(default = "default_transfer_boost_pathwise_scoring_enabled")]
    pub boost_pathwise_scoring_enabled: bool,
    #[serde(default = "default_transfer_boost_recoverability_scoring_enabled")]
    pub boost_recoverability_scoring_enabled: bool,
    #[serde(default = "default_transfer_waypoint_guidance_enabled")]
    pub waypoint_guidance_enabled: bool,
    #[serde(default = "default_transfer_gate_defer_lookahead_s")]
    pub transfer_gate_defer_lookahead_s: f64,
    #[serde(default = "default_transfer_gate_defer_step_s")]
    pub transfer_gate_defer_step_s: f64,
    #[serde(default = "default_transfer_gate_defer_min_ratio_improvement")]
    pub transfer_gate_defer_min_ratio_improvement: f64,
    pub coast_min_altitude_m: f64,
    pub terminal_gate_dx_m: f64,
    pub terminal_gate_altitude_m: f64,
    #[serde(default)]
    pub terminal: TerminalPdgControllerConfig,
}

impl Default for TransferPdgControllerConfig {
    fn default() -> Self {
        let terminal = TerminalPdgControllerConfig {
            terminal_gate_burn_time_max_s: 22.0,
            terminal_gate_burn_time_offset_long_s: 2.0,
            ..Default::default()
        };

        Self {
            takeoff_clearance_m: 45.0,
            takeoff_min_vertical_speed_mps: 8.0,
            max_takeoff_time_s: 5.0,
            source_clearance_margin_m: default_transfer_source_clearance_margin_m(),
            source_clearance_lookahead_m: default_transfer_source_clearance_lookahead_m(),
            boost_max_time_s: 18.0,
            boost_tilt_rad: 0.72,
            boost_speed_mps: 75.0,
            boost_projected_dx_limit_m: default_transfer_boost_projected_dx_limit_m(),
            boost_descent_angle_min_deg: default_transfer_boost_descent_angle_min_deg(),
            boost_descent_angle_target_deg: default_transfer_boost_descent_angle_target_deg(),
            boost_apex_height_per_dx: default_transfer_boost_apex_height_per_dx(),
            boost_apex_height_per_uphill_dy: default_transfer_boost_apex_height_per_uphill_dy(),
            boost_apex_height_min_m: default_transfer_boost_apex_height_min_m(),
            boost_apex_height_max_m: default_transfer_boost_apex_height_max_m(),
            uphill_boost_dy_min_m: default_transfer_uphill_boost_dy_min_m(),
            uphill_boost_tilt_rad: default_transfer_uphill_boost_tilt_rad(),
            boost_candidate_horizon_s: default_transfer_boost_candidate_horizon_s(),
            boost_candidate_step_s: default_transfer_boost_candidate_step_s(),
            boost_settle_lookahead_s: default_transfer_boost_settle_lookahead_s(),
            boost_pathwise_scoring_enabled: default_transfer_boost_pathwise_scoring_enabled(),
            boost_recoverability_scoring_enabled:
                default_transfer_boost_recoverability_scoring_enabled(),
            waypoint_guidance_enabled: default_transfer_waypoint_guidance_enabled(),
            transfer_gate_defer_lookahead_s: default_transfer_gate_defer_lookahead_s(),
            transfer_gate_defer_step_s: default_transfer_gate_defer_step_s(),
            transfer_gate_defer_min_ratio_improvement:
                default_transfer_gate_defer_min_ratio_improvement(),
            coast_min_altitude_m: 80.0,
            terminal_gate_dx_m: 260.0,
            terminal_gate_altitude_m: 260.0,
            terminal,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransferPhase {
    Takeoff,
    Boost,
    Coast,
    Terminal,
}

impl TransferPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Takeoff => "takeoff",
            Self::Boost => "boost",
            Self::Coast => "coast",
            Self::Terminal => "terminal",
        }
    }
}

fn waypoint_post_capture_phase(
    final_waypoint: bool,
    contract_pass: bool,
    final_terminal_recoverable: Option<bool>,
    terminal_spatial_ownership: bool,
) -> TransferPhase {
    if final_waypoint
        && contract_pass
        && final_terminal_recoverable == Some(true)
        && terminal_spatial_ownership
    {
        TransferPhase::Terminal
    } else {
        TransferPhase::Boost
    }
}

fn waypoint_terminal_spatial_ownership(
    config: &TransferPdgControllerConfig,
    observation: &Observation,
) -> bool {
    observation.height_above_target_m >= 0.0
        || (observation.target_dx_m.abs() <= config.terminal_gate_dx_m
            && -observation.height_above_target_m <= config.terminal_gate_altitude_m)
}

#[derive(Clone, Copy, Debug)]
struct TransferBallisticProjection {
    has_target_y_solution: bool,
    projected_time_s: Option<f64>,
    projected_dx_m: Option<f64>,
    impact_angle_deg: Option<f64>,
    apex_over_target_m: f64,
}

#[derive(Clone, Copy, Debug)]
struct TransferBoostQuality {
    verdict: &'static str,
    passed: bool,
    apex_target_over_target_m: f64,
}

#[derive(Clone, Copy, Debug)]
struct TransferBoostAnchor {
    route_dx_m: f64,
    route_dy_m: f64,
}

#[derive(Clone, Copy, Debug)]
struct TransferDiagnostics {
    route_dx_m: f64,
    route_dy_m: f64,
    anchor: Option<TransferBoostAnchor>,
    projection: TransferBallisticProjection,
    boost_quality: TransferBoostQuality,
}

#[derive(Clone, Debug)]
struct WaypointUpdateContext {
    observation: Observation,
    allow_terminal: bool,
    telemetry: WaypointTelemetry,
    guidance: Option<WaypointGuidanceFrame>,
    capture: Option<WaypointCaptureSnapshot>,
}

#[derive(Clone, Copy, Debug)]
struct WaypointTelemetry {
    active_index: i64,
    active_leg_index: i64,
    capture_status: &'static str,
    capture_time_s: Option<f64>,
    closest_distance_m: f64,
    distance_m: f64,
    cross_track_m: f64,
    plane_progress_m: f64,
    outbound_heading_error_rad: f64,
    outbound_progress_mps: f64,
    outbound_cross_speed_mps: f64,
    speed_mps: f64,
    vertical_speed_mps: f64,
    remaining_to_plane_m: f64,
    time_to_plane_s: f64,
    required_turn_distance_m: f64,
    shaping_start_distance_m: f64,
    turn_margin_m: f64,
    center_m: Vec2,
    nominal_handoff_target_m: Vec2,
    handoff_target_m: Vec2,
    handoff_target_mode: &'static str,
    remaining_to_handoff_m: f64,
    time_to_handoff_s: f64,
    handoff_turn_margin_m: f64,
    endpoint_m: Vec2,
    steering_target_m: Vec2,
    target_state: Option<WaypointGuidanceTargetState>,
}

#[derive(Clone, Copy, Debug)]
struct WaypointCaptureSnapshot {
    index: usize,
    capture_time_s: f64,
    status: &'static str,
    closest_distance_m: f64,
    distance_m: f64,
    cross_track_m: f64,
    plane_progress_m: f64,
    outbound_heading_error_rad: f64,
    outbound_progress_mps: f64,
    outbound_cross_speed_mps: f64,
    speed_mps: f64,
    vertical_speed_mps: f64,
    approach: WaypointApproachState,
    center_m: Vec2,
    nominal_handoff_target_m: Vec2,
    handoff_target_m: Vec2,
    handoff_target_mode: &'static str,
    endpoint_m: Vec2,
    steering_target_m: Vec2,
    target_state: Option<WaypointGuidanceTargetState>,
    transition_audit: Option<WaypointTransitionAudit>,
    guidance_replan_count: u32,
    window_entry: Option<WaypointWindowEntrySnapshot>,
    resolution_reason: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct WaypointWindowEntrySnapshot {
    time_s: f64,
    position_m: Vec2,
    velocity_mps: Vec2,
    stats: WaypointLegStats,
    assessment: WaypointGuidanceAssessment,
}

#[derive(Clone, Copy, Debug)]
struct WaypointLegGeometry<'a> {
    active_index: usize,
    waypoint: &'a TransferWaypointSpec,
    anchor_m: Vec2,
    target_m: Vec2,
    leg_unit: Vec2,
    leg_length_m: f64,
    handoff_tangent_unit: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointLegStats {
    distance_m: f64,
    cross_track_m: f64,
    plane_progress_m: f64,
    outbound_heading_error_rad: f64,
    outbound_progress_mps: f64,
    outbound_cross_speed_mps: f64,
    speed_mps: f64,
    vertical_speed_mps: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointApproachState {
    remaining_to_plane_m: f64,
    time_to_plane_s: f64,
    remaining_to_handoff_m: f64,
    time_to_handoff_s: f64,
    required_turn_distance_m: f64,
    shaping_start_distance_m: f64,
    turn_margin_m: f64,
    handoff_turn_margin_m: f64,
}

#[derive(Clone, Copy, Debug)]
struct WaypointGuidanceFrame {
    active_index: usize,
    center_m: Vec2,
    nominal_handoff_target_m: Vec2,
    handoff_target_m: Vec2,
    handoff_target_mode: &'static str,
    endpoint_m: Vec2,
    steering_target_m: Vec2,
    leg_unit: Vec2,
    handoff_tangent_unit: Vec2,
    envelope: WaypointGuidanceEnvelope,
    approach: WaypointApproachState,
}

#[derive(Clone, Copy, Debug)]
struct WaypointGuidanceEnvelope {
    capture_radius_m: f64,
    max_cross_track_m: f64,
    max_outbound_heading_error_rad: f64,
    min_outbound_progress_mps: f64,
    max_outbound_cross_speed_mps: Option<f64>,
    min_speed_mps: f64,
    max_speed_mps: f64,
    min_vertical_speed_mps: Option<f64>,
    max_vertical_speed_mps: Option<f64>,
}

impl WaypointGuidanceEnvelope {
    fn assess(self, stats: WaypointLegStats) -> WaypointGuidanceAssessment {
        let capture_window_open = stats.distance_m <= self.capture_radius_m;
        let deadline_reached = stats.plane_progress_m >= 0.0;
        let triggered = capture_window_open || deadline_reached;
        let spatial_pass = stats.distance_m <= self.capture_radius_m
            || (stats.cross_track_m <= self.max_cross_track_m
                && stats.plane_progress_m >= -self.capture_radius_m);
        let mut violation_mask = 0;
        if stats.outbound_heading_error_rad > self.max_outbound_heading_error_rad {
            violation_mask |= WAYPOINT_VIOLATION_HEADING;
        }
        if stats.outbound_progress_mps < self.min_outbound_progress_mps {
            violation_mask |= WAYPOINT_VIOLATION_OUTBOUND_PROGRESS;
        }
        if self
            .max_outbound_cross_speed_mps
            .is_some_and(|limit| stats.outbound_cross_speed_mps.abs() > limit)
        {
            violation_mask |= WAYPOINT_VIOLATION_OUTBOUND_CROSS_SPEED;
        }
        if stats.speed_mps < self.min_speed_mps || stats.speed_mps > self.max_speed_mps {
            violation_mask |= WAYPOINT_VIOLATION_SPEED;
        }
        if self
            .min_vertical_speed_mps
            .is_some_and(|limit| stats.vertical_speed_mps < limit)
            || self
                .max_vertical_speed_mps
                .is_some_and(|limit| stats.vertical_speed_mps > limit)
        {
            violation_mask |= WAYPOINT_VIOLATION_VERTICAL_SPEED;
        }
        WaypointGuidanceAssessment {
            triggered,
            capture_window_open,
            deadline_reached,
            spatial_pass,
            violation_mask,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointGuidanceAssessment {
    triggered: bool,
    capture_window_open: bool,
    deadline_reached: bool,
    spatial_pass: bool,
    violation_mask: u8,
}

impl WaypointGuidanceAssessment {
    fn envelope_pass(self) -> bool {
        self.violation_mask == 0
    }

    fn contract_pass(self) -> bool {
        self.triggered && self.spatial_pass && self.envelope_pass()
    }

    fn contract_pass_in_window(self, window_open: bool) -> bool {
        (self.triggered || window_open) && self.spatial_pass && self.envelope_pass()
    }

    fn resolved_in_window(self, window_open: bool) -> bool {
        self.contract_pass_in_window(window_open) || self.deadline_reached
    }

    fn with_window_open(mut self, window_open: bool) -> Self {
        self.capture_window_open |= window_open;
        self.triggered |= window_open;
        self
    }

    fn reasons(self) -> String {
        let mut reasons = Vec::new();
        if self.violation_mask & WAYPOINT_VIOLATION_HEADING != 0 {
            reasons.push("heading");
        }
        if self.violation_mask & WAYPOINT_VIOLATION_OUTBOUND_PROGRESS != 0 {
            reasons.push("outbound_progress");
        }
        if self.violation_mask & WAYPOINT_VIOLATION_OUTBOUND_CROSS_SPEED != 0 {
            reasons.push("outbound_cross_speed");
        }
        if self.violation_mask & WAYPOINT_VIOLATION_SPEED != 0 {
            reasons.push("speed");
        }
        if self.violation_mask & WAYPOINT_VIOLATION_VERTICAL_SPEED != 0 {
            reasons.push("vertical_speed");
        }
        reasons.join(",")
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointGuidancePrediction {
    time_to_event_s: f64,
    deadline_lead_s: f64,
    stats: WaypointLegStats,
    assessment: WaypointGuidanceAssessment,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointReachablePrediction {
    prediction: WaypointGuidancePrediction,
    event_state: TransferSimState,
    required_accel_ratio_max: f64,
    thrust_saturated_time_s: f64,
    tilt_saturated_time_s: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointContinuationPrediction {
    next_waypoint_index: usize,
    source_event_state: TransferSimState,
    source_event_time_s: f64,
    prediction: WaypointReachablePrediction,
    passing_candidate_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointTransitionAudit {
    next_waypoint_index: usize,
    position_error_m: f64,
    velocity_error_mps: f64,
    attitude_error_rad: f64,
    mass_error_kg: f64,
    fuel_error_kg: f64,
    event_time_error_s: f64,
    continuation_prediction: WaypointReachablePrediction,
    passing_candidate_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointGuidancePlan {
    waypoint_index: usize,
    revision: u32,
    reason: WaypointGuidancePlanReason,
    created_time_s: f64,
    start_position_m: Vec2,
    start_velocity_mps: Vec2,
    endpoint_m: Vec2,
    target_mode: &'static str,
    target_velocity_mps: Vec2,
    arrival_time_s: f64,
    target_envelope_feasible: bool,
    final_terminal_required_accel_ratio: Option<f64>,
    final_terminal_recoverable: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WaypointGuidancePlanReason {
    Initial,
    Expired,
    AuthorityRecovery,
    ContractRecovery,
    ReachableRecovery,
}

impl WaypointGuidancePlanReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Expired => "expired",
            Self::AuthorityRecovery => "authority_recovery",
            Self::ContractRecovery => "contract_recovery",
            Self::ReachableRecovery => "reachable_recovery",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct WaypointGuidanceTrackability {
    plan_index: usize,
    plan_revision: u32,
    plan_reason: WaypointGuidancePlanReason,
    plan_age_s: f64,
    reference_position_error_m: f64,
    reference_cross_error_m: f64,
    reference_velocity_error_mps: f64,
    reference_cross_speed_error_mps: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointGuidanceCandidate {
    target_velocity_mps: Vec2,
    time_to_go_s: f64,
    required_accel_mps2: Vec2,
    required_accel_ratio: f64,
    tilt_feasible: bool,
    target_envelope_feasible: bool,
    prediction: WaypointGuidancePrediction,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointReachableCandidate {
    candidate: WaypointGuidanceCandidate,
    endpoint_m: Vec2,
    target_mode: &'static str,
    reachable_prediction: WaypointReachablePrediction,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointFinalCandidate {
    reachable: WaypointReachableCandidate,
    terminal_gate: TerminalEntryAssessment,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointGuidancePlanSelection {
    candidate: WaypointGuidanceCandidate,
    endpoint_m: Vec2,
    target_mode: &'static str,
    terminal_gate: Option<TerminalEntryAssessment>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointJointCandidatePrediction {
    current: WaypointReachableCandidate,
    continuation: WaypointReachablePrediction,
    continuation_passing_candidate_count: usize,
}

impl WaypointJointCandidatePrediction {
    fn contract_pass(self) -> bool {
        self.current
            .reachable_prediction
            .prediction
            .assessment
            .contract_pass()
            && self.continuation.prediction.assessment.contract_pass()
    }

    fn total_saturated_time_s(self) -> f64 {
        self.current.reachable_prediction.thrust_saturated_time_s
            + self.current.reachable_prediction.tilt_saturated_time_s
            + self.continuation.thrust_saturated_time_s
            + self.continuation.tilt_saturated_time_s
    }

    fn required_accel_ratio_max(self) -> f64 {
        self.current
            .reachable_prediction
            .required_accel_ratio_max
            .max(self.continuation.required_accel_ratio_max)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaypointJointSearchPrediction {
    next_waypoint_index: usize,
    selected: Option<WaypointJointCandidatePrediction>,
    passing_candidate_count: usize,
    evaluated_candidate_count: usize,
}

#[derive(Clone, Copy, Debug)]
struct WaypointGuidanceCommandState {
    command: Command,
    target_velocity_mps: Vec2,
    time_to_go_s: f64,
    required_accel_ratio: f64,
    feasible: bool,
    path_correction_mps2: Vec2,
    deadline_remaining_s: f64,
    velocity_error_mps: f64,
    authority_margin: f64,
    thrust_saturated: bool,
    tilt_saturated: bool,
    trackability: WaypointGuidanceTrackability,
    prediction: WaypointGuidancePrediction,
    reachable_prediction: WaypointReachablePrediction,
    continuation_prediction: Option<WaypointContinuationPrediction>,
    joint_prediction: Option<WaypointJointSearchPrediction>,
    final_terminal_required_accel_ratio: Option<f64>,
    final_terminal_recoverable: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
struct WaypointGuidanceTargetState {
    target_velocity_mps: Vec2,
    deadline_remaining_s: f64,
    velocity_error_mps: f64,
    feasible: bool,
    authority_margin: f64,
    thrust_saturated: bool,
    tilt_saturated: bool,
    trackability: WaypointGuidanceTrackability,
    prediction: WaypointGuidancePrediction,
    reachable_prediction: WaypointReachablePrediction,
    continuation_prediction: Option<WaypointContinuationPrediction>,
    joint_prediction: Option<WaypointJointSearchPrediction>,
    final_terminal_required_accel_ratio: Option<f64>,
    final_terminal_recoverable: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TransferSimState {
    position_m: Vec2,
    velocity_mps: Vec2,
    attitude_rad: f64,
    fuel_kg: f64,
    dry_mass_kg: f64,
}

impl TransferSimState {
    fn mass_kg(self) -> f64 {
        (self.dry_mass_kg + self.fuel_kg.max(0.0)).max(1.0)
    }
}

#[derive(Clone, Copy, Debug)]
struct TransferBoostCandidateScore {
    score: f64,
    projection: TransferBallisticProjection,
    quality: TransferBoostQuality,
}

#[derive(Clone, Copy, Debug)]
struct TransferBoostCommandSelection {
    command: Command,
    scoring_mode: &'static str,
    selected_score: f64,
    settled_projection: TransferBallisticProjection,
    settled_quality: TransferBoostQuality,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TransferCorridorState {
    mode: &'static str,
    active: bool,
    tilt_limited: bool,
    margin_m: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransferGuidanceMode {
    Direct,
    Waypoint,
}

impl TransferGuidanceMode {
    fn from_config(config: &TransferPdgControllerConfig) -> Self {
        if config.waypoint_guidance_enabled {
            Self::Waypoint
        } else {
            Self::Direct
        }
    }

    fn uses_waypoints(self) -> bool {
        self == Self::Waypoint
    }
}

impl TransferCorridorState {
    fn inactive() -> Self {
        Self {
            mode: "inactive",
            active: false,
            tilt_limited: false,
            margin_m: 1.0e9,
        }
    }
}

#[derive(Debug)]
pub struct TransferPdgController {
    config: TransferPdgControllerConfig,
    guidance_mode: TransferGuidanceMode,
    boost_scoring_mode: TransferBoostScoringMode,
    terminal: TerminalPdgController,
    phase: TransferPhase,
    boost_anchor: Option<TransferBoostAnchor>,
    transfer_gate_ready_ticks: u32,
    last_transfer_gate: Option<TerminalEntryAssessment>,
    last_corridor: TransferCorridorState,
    last_phase: Option<String>,
    waypoint_active_index: usize,
    waypoint_closest_distance_m: f64,
    last_waypoint_capture: Option<WaypointCaptureSnapshot>,
    waypoint_window_entry: Option<WaypointWindowEntrySnapshot>,
    waypoint_guidance_plan: Option<WaypointGuidancePlan>,
    waypoint_guidance_replan_count: u32,
    waypoint_guidance_contract_failure_ticks: u32,
    waypoint_reachable_search_attempted_revision: Option<u32>,
    waypoint_reference_contract_pass_ever: bool,
    waypoint_continuation_snapshot: Option<(u32, WaypointContinuationPrediction)>,
    waypoint_joint_snapshot: Option<(u32, WaypointJointSearchPrediction)>,
}

impl Default for TransferPdgController {
    fn default() -> Self {
        Self::new(TransferPdgControllerConfig::default())
    }
}

impl TransferPdgController {
    pub fn new(config: TransferPdgControllerConfig) -> Self {
        let guidance_mode = TransferGuidanceMode::from_config(&config);
        let boost_scoring_mode = TransferBoostScoringMode::from_config(&config);
        let mut terminal = TerminalPdgController::new(config.terminal.clone());
        terminal.set_guidance_plan_retention_enabled(guidance_mode.uses_waypoints());
        Self {
            config,
            guidance_mode,
            boost_scoring_mode,
            terminal,
            phase: TransferPhase::Takeoff,
            boost_anchor: None,
            transfer_gate_ready_ticks: 0,
            last_transfer_gate: None,
            last_corridor: TransferCorridorState::inactive(),
            last_phase: None,
            waypoint_active_index: 0,
            waypoint_closest_distance_m: f64::INFINITY,
            last_waypoint_capture: None,
            waypoint_window_entry: None,
            waypoint_guidance_plan: None,
            waypoint_guidance_replan_count: 0,
            waypoint_guidance_contract_failure_ticks: 0,
            waypoint_reachable_search_attempted_revision: None,
            waypoint_reference_contract_pass_ever: false,
            waypoint_continuation_snapshot: None,
            waypoint_joint_snapshot: None,
        }
    }

    fn transfer_diagnostics(&self, observation: &Observation) -> TransferDiagnostics {
        let route_dx_m = observation.target_dx_m;
        let route_dy_m = -observation.height_above_target_m;
        let quality_anchor = self.boost_anchor.unwrap_or(TransferBoostAnchor {
            route_dx_m,
            route_dy_m,
        });
        let projection = transfer_ballistic_projection(
            route_dx_m,
            route_dy_m,
            observation.velocity_mps.x,
            observation.velocity_mps.y,
            observation.gravity_mps2,
        );
        let boost_quality = self.transfer_boost_quality(
            quality_anchor.route_dx_m,
            quality_anchor.route_dy_m,
            projection,
        );

        TransferDiagnostics {
            route_dx_m,
            route_dy_m,
            anchor: self.boost_anchor,
            projection,
            boost_quality,
        }
    }

    fn update_transfer_frame(
        &mut self,
        ctx: &RunContext,
        observation: &Observation,
        allow_terminal: bool,
        waypoint_telemetry: Option<WaypointTelemetry>,
    ) -> ControllerFrame {
        let preliminary_diagnostics = self.transfer_diagnostics(observation);
        let gate = self.transfer_gate_readiness(ctx, observation, preliminary_diagnostics);
        let corridor = self.transfer_corridor_state(ctx, observation, preliminary_diagnostics);
        self.transfer_gate_ready_ticks = gate.ready_ticks;
        self.last_transfer_gate = Some(gate);
        self.last_corridor = corridor;

        let mut phase =
            self.choose_phase(ctx, observation, preliminary_diagnostics, gate, corridor);
        if phase == TransferPhase::Terminal && !allow_terminal {
            phase = TransferPhase::Coast;
        }
        if phase == TransferPhase::Boost && self.boost_anchor.is_none() {
            self.boost_anchor = Some(TransferBoostAnchor {
                route_dx_m: observation.target_dx_m,
                route_dy_m: -observation.height_above_target_m,
            });
        }
        let diagnostics = self.transfer_diagnostics(observation);
        self.phase = phase;
        let mut frame = match phase {
            TransferPhase::Takeoff => self.frame_for_open_loop_phase(
                ctx,
                observation,
                phase,
                Command {
                    throttle_frac: 1.0,
                    target_attitude_rad: 0.0,
                },
                "lifting off from source pad",
                diagnostics,
                gate,
                corridor,
                None,
            ),
            TransferPhase::Boost => {
                let selection =
                    self.select_boost_command(ctx, observation, diagnostics, gate, corridor);
                self.frame_for_open_loop_phase(
                    ctx,
                    observation,
                    phase,
                    selection.command,
                    "boosting toward terminal gate",
                    diagnostics,
                    gate,
                    corridor,
                    Some(selection),
                )
            }
            TransferPhase::Coast => self.frame_for_open_loop_phase(
                ctx,
                observation,
                phase,
                Command {
                    throttle_frac: 0.0,
                    target_attitude_rad: self.coast_attitude_rad(observation),
                },
                "coasting before terminal handoff",
                diagnostics,
                gate,
                corridor,
                None,
            ),
            TransferPhase::Terminal => {
                let mut frame = self.terminal.update(ctx, observation);
                self.insert_transfer_metrics(&mut frame, diagnostics, gate, corridor);
                frame.metrics.insert(
                    metric::TRANSFER_PHASE.to_owned(),
                    TelemetryValue::from("terminal"),
                );
                frame.status = format!("transfer handoff: {}", frame.status);
                self.last_phase = frame.phase.clone();
                frame
            }
        };
        self.insert_waypoint_metrics(&mut frame, waypoint_telemetry);
        frame
    }

    fn waypoint_update_context(
        &mut self,
        ctx: &RunContext,
        observation: &Observation,
    ) -> Option<WaypointUpdateContext> {
        if !self.guidance_mode.uses_waypoints() {
            return None;
        }
        let route = ctx.mission.transfer_route.as_ref()?;
        if route.waypoints.is_empty() {
            return None;
        }
        if self.waypoint_active_index >= route.waypoints.len() {
            let telemetry = self.last_waypoint_capture.map_or_else(
                || self.waypoint_complete_telemetry(route.waypoints.len()),
                WaypointTelemetry::from_capture,
            );
            return Some(WaypointUpdateContext {
                observation: observation.clone(),
                allow_terminal: true,
                telemetry,
                guidance: None,
                capture: None,
            });
        }

        let geometry = self.waypoint_leg_geometry(ctx)?;
        let stats = waypoint_leg_stats(observation, &geometry);
        let approach = self.waypoint_approach_state(ctx, observation, &geometry, stats);
        let guidance = waypoint_guidance_frame(&geometry, stats, approach);
        self.waypoint_closest_distance_m = self.waypoint_closest_distance_m.min(stats.distance_m);
        let handoff = geometry
            .waypoint
            .assess_handoff(waypoint_handoff_kinematics(stats));
        if handoff.capture_window_open && self.waypoint_window_entry.is_none() {
            self.waypoint_window_entry = Some(WaypointWindowEntrySnapshot {
                time_s: observation.sim_time_s,
                position_m: observation.position_m,
                velocity_mps: observation.velocity_mps,
                stats,
                assessment: guidance.envelope.assess(stats),
            });
        }
        let window_open = self.waypoint_window_entry.is_some();
        if handoff.resolved_in_window(window_open) {
            let contract_pass = handoff.contract_pass_in_window(window_open);
            let final_waypoint = geometry.active_index + 1 >= route.waypoints.len();
            let status = if handoff.spatial_pass {
                "captured"
            } else {
                "missed"
            };
            let target_state =
                self.waypoint_guidance_target_state_for_current_plan(ctx, observation, guidance);
            let transition_audit = target_state
                .and_then(|state| state.continuation_prediction)
                .and_then(|continuation| {
                    self.waypoint_transition_audit(ctx, observation, continuation)
                });
            let guidance_plan = self
                .waypoint_guidance_plan
                .filter(|plan| plan.waypoint_index == guidance.active_index);
            let capture = WaypointCaptureSnapshot {
                index: geometry.active_index,
                capture_time_s: observation.sim_time_s,
                status,
                closest_distance_m: self.waypoint_closest_distance_m,
                distance_m: stats.distance_m,
                cross_track_m: stats.cross_track_m,
                plane_progress_m: stats.plane_progress_m,
                outbound_heading_error_rad: stats.outbound_heading_error_rad,
                outbound_progress_mps: stats.outbound_progress_mps,
                outbound_cross_speed_mps: stats.outbound_cross_speed_mps,
                speed_mps: stats.speed_mps,
                vertical_speed_mps: stats.vertical_speed_mps,
                approach,
                center_m: guidance.center_m,
                nominal_handoff_target_m: guidance.nominal_handoff_target_m,
                handoff_target_m: guidance_plan
                    .map_or(guidance.handoff_target_m, |plan| plan.endpoint_m),
                handoff_target_mode: guidance_plan
                    .map_or(guidance.handoff_target_mode, |plan| plan.target_mode),
                endpoint_m: guidance_plan.map_or(guidance.endpoint_m, |plan| plan.endpoint_m),
                steering_target_m: guidance.steering_target_m,
                target_state,
                transition_audit,
                guidance_replan_count: self.waypoint_guidance_replan_count,
                window_entry: self.waypoint_window_entry,
                resolution_reason: if contract_pass {
                    "contract_pass"
                } else {
                    "plane_deadline"
                },
            };
            self.last_waypoint_capture = Some(capture);
            self.waypoint_active_index += 1;
            self.waypoint_closest_distance_m = f64::INFINITY;
            self.waypoint_window_entry = None;
            self.waypoint_guidance_plan = None;
            self.waypoint_guidance_replan_count = 0;
            self.waypoint_guidance_contract_failure_ticks = 0;
            self.waypoint_reachable_search_attempted_revision = None;
            self.waypoint_reference_contract_pass_ever = false;
            self.waypoint_continuation_snapshot = None;
            self.waypoint_joint_snapshot = None;
            self.boost_anchor = None;
            self.transfer_gate_ready_ticks = 0;
            self.phase = waypoint_post_capture_phase(
                final_waypoint,
                contract_pass,
                target_state.and_then(|state| state.final_terminal_recoverable),
                waypoint_terminal_spatial_ownership(&self.config, observation),
            );
            self.terminal.reset(ctx);

            if final_waypoint {
                return Some(WaypointUpdateContext {
                    observation: observation.clone(),
                    allow_terminal: true,
                    telemetry: WaypointTelemetry::from_capture(capture),
                    guidance: None,
                    capture: Some(capture),
                });
            }
            let next_geometry = self.waypoint_leg_geometry(ctx)?;
            let next_stats = waypoint_leg_stats(observation, &next_geometry);
            let next_approach =
                self.waypoint_approach_state(ctx, observation, &next_geometry, next_stats);
            let next_guidance = waypoint_guidance_frame(&next_geometry, next_stats, next_approach);
            return Some(WaypointUpdateContext {
                observation: observation.clone(),
                allow_terminal: false,
                telemetry: WaypointTelemetry::from_capture(capture),
                guidance: Some(next_guidance),
                capture: Some(capture),
            });
        }

        Some(WaypointUpdateContext {
            observation: observation.clone(),
            allow_terminal: false,
            telemetry: WaypointTelemetry {
                active_index: geometry.active_index as i64,
                active_leg_index: geometry.active_index as i64,
                capture_status: if window_open {
                    "capture_window"
                } else {
                    "tracking"
                },
                capture_time_s: None,
                closest_distance_m: self.waypoint_closest_distance_m,
                distance_m: stats.distance_m,
                cross_track_m: stats.cross_track_m,
                plane_progress_m: stats.plane_progress_m,
                outbound_heading_error_rad: stats.outbound_heading_error_rad,
                outbound_progress_mps: stats.outbound_progress_mps,
                outbound_cross_speed_mps: stats.outbound_cross_speed_mps,
                speed_mps: stats.speed_mps,
                vertical_speed_mps: stats.vertical_speed_mps,
                remaining_to_plane_m: approach.remaining_to_plane_m,
                time_to_plane_s: approach.time_to_plane_s,
                required_turn_distance_m: approach.required_turn_distance_m,
                shaping_start_distance_m: approach.shaping_start_distance_m,
                turn_margin_m: approach.turn_margin_m,
                center_m: guidance.center_m,
                nominal_handoff_target_m: guidance.nominal_handoff_target_m,
                handoff_target_m: self
                    .waypoint_guidance_plan
                    .filter(|plan| plan.waypoint_index == guidance.active_index)
                    .map_or(guidance.handoff_target_m, |plan| plan.endpoint_m),
                handoff_target_mode: self
                    .waypoint_guidance_plan
                    .filter(|plan| plan.waypoint_index == guidance.active_index)
                    .map_or(guidance.handoff_target_mode, |plan| plan.target_mode),
                remaining_to_handoff_m: approach.remaining_to_handoff_m,
                time_to_handoff_s: approach.time_to_handoff_s,
                handoff_turn_margin_m: approach.handoff_turn_margin_m,
                endpoint_m: self
                    .waypoint_guidance_plan
                    .filter(|plan| plan.waypoint_index == guidance.active_index)
                    .map_or(guidance.endpoint_m, |plan| plan.endpoint_m),
                steering_target_m: guidance.steering_target_m,
                target_state: None,
            },
            guidance: Some(guidance),
            capture: None,
        })
    }

    fn waypoint_leg_geometry<'a>(&self, ctx: &'a RunContext) -> Option<WaypointLegGeometry<'a>> {
        Self::waypoint_leg_geometry_at(ctx, self.waypoint_active_index)
    }

    fn waypoint_leg_geometry_at(
        ctx: &RunContext,
        active_index: usize,
    ) -> Option<WaypointLegGeometry<'_>> {
        let route = ctx.mission.transfer_route.as_ref()?;
        let waypoint = route.waypoints.get(active_index)?;
        let anchor_m = if active_index == 0 {
            ctx.world
                .landing_pad(&route.source_pad_id)
                .map(|pad| Vec2::new(pad.center_x_m, pad.surface_y_m))?
        } else {
            route.waypoints[active_index - 1].position_m
        };
        let target_m = waypoint.position_m;
        let next_target_m = route
            .waypoints
            .get(active_index + 1)
            .map(|next| next.position_m)
            .unwrap_or_else(|| Vec2::new(ctx.target_pad.center_x_m, ctx.target_pad.surface_y_m));
        let leg_vector = target_m - anchor_m;
        let leg_length_m = leg_vector.length();
        let leg_unit = normalized_or_none(leg_vector)?;
        let next_leg_unit = normalized_or_none(next_target_m - target_m)?;
        let handoff_tangent_unit = waypoint.handoff_tangent_unit.unwrap_or(next_leg_unit);
        Some(WaypointLegGeometry {
            active_index,
            waypoint,
            anchor_m,
            target_m,
            leg_unit,
            leg_length_m,
            handoff_tangent_unit,
        })
    }

    fn waypoint_complete_telemetry(&self, waypoint_count: usize) -> WaypointTelemetry {
        WaypointTelemetry {
            active_index: waypoint_count as i64,
            active_leg_index: waypoint_count as i64,
            capture_status: "complete",
            capture_time_s: None,
            closest_distance_m: -1.0,
            distance_m: -1.0,
            cross_track_m: -1.0,
            plane_progress_m: -1.0,
            outbound_heading_error_rad: -1.0,
            outbound_progress_mps: -1.0,
            outbound_cross_speed_mps: -1.0,
            speed_mps: -1.0,
            vertical_speed_mps: -1.0,
            remaining_to_plane_m: -1.0,
            time_to_plane_s: -1.0,
            required_turn_distance_m: -1.0,
            shaping_start_distance_m: -1.0,
            turn_margin_m: -1.0,
            center_m: Vec2::new(-1.0, -1.0),
            nominal_handoff_target_m: Vec2::new(-1.0, -1.0),
            handoff_target_m: Vec2::new(-1.0, -1.0),
            handoff_target_mode: "complete",
            remaining_to_handoff_m: -1.0,
            time_to_handoff_s: -1.0,
            handoff_turn_margin_m: -1.0,
            endpoint_m: Vec2::new(-1.0, -1.0),
            steering_target_m: Vec2::new(-1.0, -1.0),
            target_state: None,
        }
    }

    fn insert_waypoint_metrics(
        &self,
        frame: &mut ControllerFrame,
        waypoint_telemetry: Option<WaypointTelemetry>,
    ) {
        let Some(telemetry) = waypoint_telemetry else {
            return;
        };
        frame.metrics.insert(
            metric::WAYPOINT_GUIDANCE_ENABLED.to_owned(),
            TelemetryValue::from(true),
        );
        frame.metrics.insert(
            metric::WAYPOINT_ACTIVE_INDEX.to_owned(),
            TelemetryValue::from(telemetry.active_index),
        );
        frame.metrics.insert(
            metric::WAYPOINT_ACTIVE_LEG_INDEX.to_owned(),
            TelemetryValue::from(telemetry.active_leg_index),
        );
        frame.metrics.insert(
            metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
            TelemetryValue::from(telemetry.capture_status),
        );
        frame.metrics.insert(
            metric::WAYPOINT_CAPTURE_TIME_S.to_owned(),
            TelemetryValue::from(telemetry.capture_time_s.unwrap_or(-1.0)),
        );
        frame.metrics.insert(
            metric::WAYPOINT_CLOSEST_DISTANCE_M.to_owned(),
            TelemetryValue::from(telemetry.closest_distance_m),
        );
        frame.metrics.insert(
            metric::WAYPOINT_DISTANCE_M.to_owned(),
            TelemetryValue::from(telemetry.distance_m),
        );
        frame.metrics.insert(
            metric::WAYPOINT_CROSS_TRACK_M.to_owned(),
            TelemetryValue::from(telemetry.cross_track_m),
        );
        frame.metrics.insert(
            metric::WAYPOINT_PLANE_PROGRESS_M.to_owned(),
            TelemetryValue::from(telemetry.plane_progress_m),
        );
        frame.metrics.insert(
            metric::WAYPOINT_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
            TelemetryValue::from(telemetry.outbound_heading_error_rad),
        );
        frame.metrics.insert(
            metric::WAYPOINT_OUTBOUND_PROGRESS_MPS.to_owned(),
            TelemetryValue::from(telemetry.outbound_progress_mps),
        );
        frame.metrics.insert(
            metric::WAYPOINT_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
            TelemetryValue::from(telemetry.outbound_cross_speed_mps),
        );
        frame.metrics.insert(
            metric::WAYPOINT_SPEED_MPS.to_owned(),
            TelemetryValue::from(telemetry.speed_mps),
        );
        frame.metrics.insert(
            metric::WAYPOINT_VERTICAL_SPEED_MPS.to_owned(),
            TelemetryValue::from(telemetry.vertical_speed_mps),
        );
        frame.metrics.insert(
            metric::WAYPOINT_REMAINING_TO_PLANE_M.to_owned(),
            TelemetryValue::from(telemetry.remaining_to_plane_m),
        );
        frame.metrics.insert(
            metric::WAYPOINT_TIME_TO_PLANE_S.to_owned(),
            TelemetryValue::from(telemetry.time_to_plane_s),
        );
        frame.metrics.insert(
            metric::WAYPOINT_REQUIRED_TURN_DISTANCE_M.to_owned(),
            TelemetryValue::from(telemetry.required_turn_distance_m),
        );
        frame.metrics.insert(
            metric::WAYPOINT_SHAPING_START_DISTANCE_M.to_owned(),
            TelemetryValue::from(telemetry.shaping_start_distance_m),
        );
        frame.metrics.insert(
            metric::WAYPOINT_TURN_MARGIN_M.to_owned(),
            TelemetryValue::from(telemetry.turn_margin_m),
        );
        frame.metrics.insert(
            metric::WAYPOINT_CENTER_X_M.to_owned(),
            TelemetryValue::from(telemetry.center_m.x),
        );
        frame.metrics.insert(
            metric::WAYPOINT_CENTER_Y_M.to_owned(),
            TelemetryValue::from(telemetry.center_m.y),
        );
        frame.metrics.insert(
            metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_X_M.to_owned(),
            TelemetryValue::from(telemetry.nominal_handoff_target_m.x),
        );
        frame.metrics.insert(
            metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_Y_M.to_owned(),
            TelemetryValue::from(telemetry.nominal_handoff_target_m.y),
        );
        frame.metrics.insert(
            metric::WAYPOINT_HANDOFF_TARGET_X_M.to_owned(),
            TelemetryValue::from(telemetry.handoff_target_m.x),
        );
        frame.metrics.insert(
            metric::WAYPOINT_HANDOFF_TARGET_Y_M.to_owned(),
            TelemetryValue::from(telemetry.handoff_target_m.y),
        );
        frame.metrics.insert(
            metric::WAYPOINT_HANDOFF_TARGET_MODE.to_owned(),
            TelemetryValue::from(telemetry.handoff_target_mode),
        );
        frame.metrics.insert(
            metric::WAYPOINT_REMAINING_TO_HANDOFF_M.to_owned(),
            TelemetryValue::from(telemetry.remaining_to_handoff_m),
        );
        frame.metrics.insert(
            metric::WAYPOINT_TIME_TO_HANDOFF_S.to_owned(),
            TelemetryValue::from(telemetry.time_to_handoff_s),
        );
        frame.metrics.insert(
            metric::WAYPOINT_HANDOFF_TURN_MARGIN_M.to_owned(),
            TelemetryValue::from(telemetry.handoff_turn_margin_m),
        );
        frame.metrics.insert(
            metric::WAYPOINT_ENDPOINT_X_M.to_owned(),
            TelemetryValue::from(telemetry.endpoint_m.x),
        );
        frame.metrics.insert(
            metric::WAYPOINT_ENDPOINT_Y_M.to_owned(),
            TelemetryValue::from(telemetry.endpoint_m.y),
        );
        frame.metrics.insert(
            metric::WAYPOINT_STEERING_TARGET_X_M.to_owned(),
            TelemetryValue::from(telemetry.steering_target_m.x),
        );
        frame.metrics.insert(
            metric::WAYPOINT_STEERING_TARGET_Y_M.to_owned(),
            TelemetryValue::from(telemetry.steering_target_m.y),
        );
        if let Some(target_state) = telemetry.target_state {
            insert_waypoint_target_state_metrics(&mut frame.metrics, target_state);
        }
    }

    fn waypoint_takeoff_required(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
    ) -> bool {
        let Some(route) = ctx.mission.transfer_route.as_ref() else {
            return false;
        };
        let Some(source_pad) = ctx.world.landing_pad(&route.source_pad_id) else {
            return false;
        };
        let source_clearance_m = observation.position_m.y
            - source_pad.surface_y_m
            - ctx.vehicle.geometry.touchdown_base_offset_m;
        if source_clearance_m < self.config.takeoff_clearance_m
            && observation.velocity_mps.y < self.config.takeoff_min_vertical_speed_mps
            && observation.sim_time_s < self.config.max_takeoff_time_s
        {
            return true;
        }

        let endpoint_observation = waypoint_adjusted_observation(
            observation,
            guidance.endpoint_m,
            guidance.envelope.capture_radius_m,
        );
        source_clearance_m < self.config.takeoff_clearance_m
            && self.source_clearance_hold_needed(ctx, &endpoint_observation)
    }

    fn waypoint_takeoff_target_vertical_speed_mps(
        &self,
        ctx: &RunContext,
        guidance: WaypointGuidanceFrame,
    ) -> f64 {
        let planned_inbound_length_m = ctx
            .mission
            .transfer_route
            .as_ref()
            .and_then(|route| ctx.world.landing_pad(&route.source_pad_id))
            .map(|source_pad| {
                let source_m = Vec2::new(source_pad.center_x_m, source_pad.surface_y_m);
                vec_dot(guidance.center_m - source_m, guidance.leg_unit).max(0.0)
            })
            .unwrap_or(guidance.approach.remaining_to_plane_m);
        (planned_inbound_length_m * WAYPOINT_TAKEOFF_SPEED_PER_INBOUND_DISTANCE_HZ)
            .max(self.config.takeoff_min_vertical_speed_mps)
    }

    fn waypoint_takeoff_command(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
    ) -> Command {
        let max_thrust_accel_mps2 = ctx.vehicle.max_thrust_n / observation.mass_kg.max(1.0);
        let speed_error_mps = self.waypoint_takeoff_target_vertical_speed_mps(ctx, guidance)
            - observation.velocity_mps.y;
        let required_thrust_accel_mps2 =
            observation.gravity_mps2 + (speed_error_mps / WAYPOINT_TAKEOFF_SPEED_RESPONSE_TIME_S);
        Command {
            throttle_frac: (required_thrust_accel_mps2 / max_thrust_accel_mps2.max(1.0e-6))
                .clamp(0.0, 1.0),
            target_attitude_rad: 0.0,
        }
    }

    fn waypoint_target_velocity_is_valid(
        &self,
        guidance: WaypointGuidanceFrame,
        target_velocity_mps: Vec2,
    ) -> bool {
        let speed_mps = target_velocity_mps.length();
        let outbound_progress_mps = vec_dot(target_velocity_mps, guidance.handoff_tangent_unit);
        if speed_mps + WAYPOINT_GUIDANCE_ENVELOPE_EPS_MPS < guidance.envelope.min_speed_mps
            || speed_mps > guidance.envelope.max_speed_mps + WAYPOINT_GUIDANCE_ENVELOPE_EPS_MPS
            || outbound_progress_mps + WAYPOINT_GUIDANCE_ENVELOPE_EPS_MPS
                < guidance.envelope.min_outbound_progress_mps
        {
            return false;
        }
        if guidance
            .envelope
            .min_vertical_speed_mps
            .is_some_and(|minimum| {
                target_velocity_mps.y + WAYPOINT_GUIDANCE_ENVELOPE_EPS_MPS < minimum
            })
            || guidance
                .envelope
                .max_vertical_speed_mps
                .is_some_and(|maximum| {
                    target_velocity_mps.y > maximum + WAYPOINT_GUIDANCE_ENVELOPE_EPS_MPS
                })
        {
            return false;
        }
        true
    }

    fn waypoint_guidance_candidate(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
        target_velocity_mps: Vec2,
        time_to_go_s: f64,
        target_envelope_feasible: bool,
    ) -> WaypointGuidanceCandidate {
        let required_accel_mps2 = required_control_accel(StateTargetRequest {
            position_error_m: guidance.endpoint_m - observation.position_m,
            velocity_mps: observation.velocity_mps,
            target_velocity_mps,
            time_to_go_s,
            gravity_mps2: observation.gravity_mps2,
        });
        let max_thrust_accel_mps2 = ctx.vehicle.max_thrust_n / observation.mass_kg.max(1.0);
        let max_tilt_rad = self
            .config
            .boost_tilt_rad
            .max(self.config.uphill_boost_tilt_rad);
        let tilt_feasible = required_accel_mps2.y > 0.0
            && required_accel_mps2.x.abs() <= max_tilt_rad.tan() * required_accel_mps2.y;
        let prediction =
            waypoint_guidance_prediction(observation, guidance, target_velocity_mps, time_to_go_s);
        WaypointGuidanceCandidate {
            target_velocity_mps,
            time_to_go_s,
            required_accel_mps2,
            required_accel_ratio: required_accel_mps2.length() / max_thrust_accel_mps2.max(1.0e-6),
            tilt_feasible,
            target_envelope_feasible,
            prediction,
        }
    }

    // Endpoint substitution is a narrow waypoint-search adapter; keep its
    // typed state and envelope inputs explicit at the search call sites.
    #[allow(clippy::too_many_arguments)]
    fn waypoint_guidance_candidate_at_endpoint(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        mut guidance: WaypointGuidanceFrame,
        endpoint_m: Vec2,
        target_velocity_mps: Vec2,
        time_to_go_s: f64,
        target_envelope_feasible: bool,
    ) -> WaypointGuidanceCandidate {
        guidance.endpoint_m = endpoint_m;
        self.waypoint_guidance_candidate(
            ctx,
            observation,
            guidance,
            target_velocity_mps,
            time_to_go_s,
            target_envelope_feasible,
        )
    }

    fn waypoint_guidance_candidate_class(
        candidate: WaypointGuidanceCandidate,
        contract_aware: bool,
    ) -> u8 {
        if !candidate.target_envelope_feasible {
            4
        } else if candidate.tilt_feasible
            && candidate.required_accel_ratio <= 1.0
            && (!contract_aware || candidate.prediction.assessment.contract_pass())
        {
            0
        } else if contract_aware && candidate.tilt_feasible && candidate.required_accel_ratio <= 1.0
        {
            1
        } else if candidate.tilt_feasible {
            2
        } else {
            3
        }
    }

    fn compare_waypoint_guidance_candidates(
        &self,
        lhs: WaypointGuidanceCandidate,
        rhs: WaypointGuidanceCandidate,
        contract_aware: bool,
    ) -> std::cmp::Ordering {
        let lhs_class = Self::waypoint_guidance_candidate_class(lhs, contract_aware);
        let rhs_class = Self::waypoint_guidance_candidate_class(rhs, contract_aware);
        let class_order = lhs_class.cmp(&rhs_class);
        if class_order != std::cmp::Ordering::Equal {
            return class_order;
        }

        let speed_preference = || {
            (lhs.target_velocity_mps.length() - self.config.boost_speed_mps)
                .abs()
                .total_cmp(&(rhs.target_velocity_mps.length() - self.config.boost_speed_mps).abs())
        };
        lhs.time_to_go_s
            .total_cmp(&rhs.time_to_go_s)
            .then_with(speed_preference)
            .then_with(|| {
                lhs.required_accel_ratio
                    .total_cmp(&rhs.required_accel_ratio)
            })
    }

    fn waypoint_guidance_candidate_has_control_authority(
        candidate: WaypointGuidanceCandidate,
    ) -> bool {
        candidate.tilt_feasible && candidate.required_accel_ratio <= 1.0
    }

    fn waypoint_guidance_contract_failure_is_actionable(
        candidate: WaypointGuidanceCandidate,
    ) -> bool {
        !candidate.prediction.assessment.contract_pass()
            && candidate.prediction.time_to_event_s <= WAYPOINT_GUIDANCE_PREDICTION_HORIZON_S
    }

    fn waypoint_guidance_replacement_is_material(
        current: WaypointGuidanceCandidate,
        replacement: WaypointGuidanceCandidate,
    ) -> bool {
        let time_change_ratio = (replacement.time_to_go_s - current.time_to_go_s).abs()
            / current.time_to_go_s.max(WAYPOINT_GUIDANCE_MIN_TIME_TO_GO_S);
        let velocity_change_ratio = (replacement.target_velocity_mps - current.target_velocity_mps)
            .length()
            / current.target_velocity_mps.length().max(1.0);
        time_change_ratio >= WAYPOINT_GUIDANCE_REPLAN_MATERIALITY_RATIO
            || velocity_change_ratio >= WAYPOINT_GUIDANCE_REPLAN_MATERIALITY_RATIO
    }

    fn should_replace_waypoint_guidance_plan(
        current: WaypointGuidanceCandidate,
        replacement: WaypointGuidanceCandidate,
        expired: bool,
        contract_failure_confirmed: bool,
    ) -> bool {
        if expired {
            return true;
        }
        let replacement_dynamically_feasible = replacement.target_envelope_feasible
            && Self::waypoint_guidance_candidate_has_control_authority(replacement);
        if !Self::waypoint_guidance_candidate_has_control_authority(current)
            && replacement_dynamically_feasible
        {
            return true;
        }
        contract_failure_confirmed
            && !current.prediction.assessment.contract_pass()
            && replacement_dynamically_feasible
            && replacement.prediction.assessment.contract_pass()
            && Self::waypoint_guidance_replacement_is_material(current, replacement)
    }

    fn should_preserve_waypoint_plan_during_authority_recovery(
        current: WaypointGuidanceCandidate,
        current_reachable: WaypointReachablePrediction,
        plan_reason: WaypointGuidancePlanReason,
        expired: bool,
        reference_contract_pass_ever: bool,
    ) -> bool {
        !expired
            && reference_contract_pass_ever
            && plan_reason == WaypointGuidancePlanReason::ReachableRecovery
            && !Self::waypoint_guidance_candidate_has_control_authority(current)
            && current.prediction.assessment.contract_pass()
            && current_reachable.prediction.assessment.contract_pass()
    }

    fn waypoint_guidance_plan_reason(
        current: Option<WaypointGuidanceCandidate>,
        expired: bool,
    ) -> WaypointGuidancePlanReason {
        if current.is_none() {
            WaypointGuidancePlanReason::Initial
        } else if expired {
            WaypointGuidancePlanReason::Expired
        } else if current.is_some_and(|candidate| {
            !Self::waypoint_guidance_candidate_has_control_authority(candidate)
        }) {
            WaypointGuidancePlanReason::AuthorityRecovery
        } else {
            WaypointGuidancePlanReason::ContractRecovery
        }
    }

    fn select_waypoint_guidance_candidate(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
        contract_aware: bool,
    ) -> WaypointGuidanceCandidate {
        self.waypoint_guidance_candidates(ctx, observation, guidance)
            .into_iter()
            .min_by(|lhs, rhs| {
                self.compare_waypoint_guidance_candidates(*lhs, *rhs, contract_aware)
            })
            .expect("waypoint guidance always creates at least one candidate")
    }

    fn waypoint_guidance_candidates(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
    ) -> Vec<WaypointGuidanceCandidate> {
        let minimum_speed_mps = guidance
            .envelope
            .min_speed_mps
            .max(guidance.envelope.min_outbound_progress_mps)
            .max(WAYPOINT_GUIDANCE_MIN_CLOSURE_MPS);
        let maximum_speed_mps = guidance.envelope.max_speed_mps.max(minimum_speed_mps);
        let mut target_speeds_mps = Vec::new();
        for speed_mps in [
            self.config.boost_speed_mps,
            vec_dot(observation.velocity_mps, guidance.handoff_tangent_unit),
            minimum_speed_mps,
        ] {
            self.push_unique_candidate(
                &mut target_speeds_mps,
                speed_mps.clamp(minimum_speed_mps, maximum_speed_mps),
            );
        }
        if guidance.handoff_tangent_unit.y.abs() > WAYPOINT_GUIDANCE_UNIQUE_EPS {
            for vertical_speed_mps in [
                guidance.envelope.min_vertical_speed_mps,
                guidance.envelope.max_vertical_speed_mps,
            ]
            .into_iter()
            .flatten()
            {
                let speed_mps = vertical_speed_mps / guidance.handoff_tangent_unit.y;
                if speed_mps >= minimum_speed_mps && speed_mps <= maximum_speed_mps {
                    self.push_unique_candidate(&mut target_speeds_mps, speed_mps);
                }
            }
        }

        let remaining_m = guidance.approach.remaining_to_plane_m.max(0.0);
        let current_closing_mps = vec_dot(observation.velocity_mps, guidance.leg_unit).max(0.0);
        let minimum_time_to_go_s =
            (remaining_m / maximum_speed_mps.max(1.0)).max(WAYPOINT_GUIDANCE_MIN_TIME_TO_GO_S);
        let maximum_time_to_go_s =
            (2.0 * remaining_m / minimum_speed_mps.max(1.0)).max(minimum_time_to_go_s);
        let mut candidates = Vec::new();

        for speed_mps in target_speeds_mps {
            let target_velocity_mps = guidance.handoff_tangent_unit * speed_mps;
            let target_envelope_feasible =
                self.waypoint_target_velocity_is_valid(guidance, target_velocity_mps);
            let target_closing_mps = vec_dot(target_velocity_mps, guidance.leg_unit).max(0.0);
            let nominal_time_to_go_s = 2.0 * remaining_m
                / (current_closing_mps + target_closing_mps).max(WAYPOINT_GUIDANCE_MIN_CLOSURE_MPS);
            let cruise_time_to_go_s = remaining_m
                / self
                    .config
                    .boost_speed_mps
                    .clamp(minimum_speed_mps, maximum_speed_mps)
                    .max(WAYPOINT_GUIDANCE_MIN_CLOSURE_MPS);
            let mut times_to_go_s = Vec::new();
            for time_to_go_s in [
                nominal_time_to_go_s * 0.8,
                nominal_time_to_go_s,
                nominal_time_to_go_s * 1.25,
                cruise_time_to_go_s,
            ] {
                self.push_unique_candidate(
                    &mut times_to_go_s,
                    time_to_go_s.clamp(minimum_time_to_go_s, maximum_time_to_go_s),
                );
            }
            for time_to_go_s in times_to_go_s {
                candidates.push(self.waypoint_guidance_candidate(
                    ctx,
                    observation,
                    guidance,
                    target_velocity_mps,
                    time_to_go_s,
                    target_envelope_feasible,
                ));
            }
        }

        if !candidates
            .iter()
            .any(|candidate| candidate.target_envelope_feasible)
        {
            let target_velocity_mps = guidance.handoff_tangent_unit
                * self
                    .config
                    .boost_speed_mps
                    .clamp(minimum_speed_mps, maximum_speed_mps);
            candidates.push(self.waypoint_guidance_candidate(
                ctx,
                observation,
                guidance,
                target_velocity_mps,
                maximum_time_to_go_s,
                false,
            ));
        }

        candidates
    }

    fn waypoint_reachable_event_candidates(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
    ) -> Vec<WaypointReachableCandidate> {
        let minimum_speed_mps = guidance
            .envelope
            .min_speed_mps
            .max(guidance.envelope.min_outbound_progress_mps)
            .max(WAYPOINT_GUIDANCE_MIN_CLOSURE_MPS);
        let maximum_speed_mps = guidance.envelope.max_speed_mps.max(minimum_speed_mps);
        let mut speeds_mps = Vec::new();
        for speed_mps in [
            minimum_speed_mps,
            vec_dot(observation.velocity_mps, guidance.handoff_tangent_unit),
            self.config.boost_speed_mps,
        ] {
            self.push_unique_candidate(
                &mut speeds_mps,
                speed_mps.clamp(minimum_speed_mps, maximum_speed_mps),
            );
        }

        let endpoints = waypoint_reachable_event_endpoints(guidance);

        let turn_rad = vec_cross(guidance.handoff_tangent_unit, guidance.leg_unit)
            .atan2(vec_dot(guidance.handoff_tangent_unit, guidance.leg_unit));
        let mut reachable_candidates = Vec::new();
        for endpoint_m in endpoints {
            let to_endpoint_m = endpoint_m - observation.position_m;
            let remaining_m = to_endpoint_m.length();
            let Some(endpoint_unit) = normalized_or_none(to_endpoint_m) else {
                continue;
            };
            let current_closing_mps = vec_dot(observation.velocity_mps, endpoint_unit).max(0.0);
            let minimum_time_to_go_s =
                (remaining_m / maximum_speed_mps.max(1.0)).max(WAYPOINT_GUIDANCE_MIN_TIME_TO_GO_S);
            let maximum_time_to_go_s =
                (2.0 * remaining_m / minimum_speed_mps.max(1.0)).max(minimum_time_to_go_s);

            for speed_mps in speeds_mps.iter().copied() {
                let cross_speed_angle_rad = guidance
                    .envelope
                    .max_outbound_cross_speed_mps
                    .map_or(std::f64::consts::FRAC_PI_2, |limit| {
                        (limit / speed_mps.max(1.0e-6)).clamp(0.0, 1.0).asin()
                    });
                let allowed_heading_rad = guidance
                    .envelope
                    .max_outbound_heading_error_rad
                    .min(cross_speed_angle_rad)
                    .max(0.0);
                let boundary_angle_rad =
                    turn_rad.signum() * turn_rad.abs().min((allowed_heading_rad - 1.0e-6).max(0.0));
                let (sin_boundary, cos_boundary) = boundary_angle_rad.sin_cos();
                let boundary_direction = Vec2::new(
                    (guidance.handoff_tangent_unit.x * cos_boundary)
                        - (guidance.handoff_tangent_unit.y * sin_boundary),
                    (guidance.handoff_tangent_unit.x * sin_boundary)
                        + (guidance.handoff_tangent_unit.y * cos_boundary),
                );
                let mut velocity_directions = vec![guidance.handoff_tangent_unit];
                if (boundary_direction - guidance.handoff_tangent_unit).length() > 1.0e-6 {
                    velocity_directions.push(boundary_direction);
                }

                for velocity_direction in velocity_directions {
                    let target_velocity_mps = velocity_direction * speed_mps;
                    if !self.waypoint_target_velocity_is_valid(guidance, target_velocity_mps) {
                        continue;
                    }
                    let target_closing_mps = vec_dot(target_velocity_mps, endpoint_unit).max(0.0);
                    let nominal_time_to_go_s = 2.0 * remaining_m
                        / (current_closing_mps + target_closing_mps)
                            .max(WAYPOINT_GUIDANCE_MIN_CLOSURE_MPS);
                    let cruise_time_to_go_s = remaining_m
                        / self
                            .config
                            .boost_speed_mps
                            .clamp(minimum_speed_mps, maximum_speed_mps)
                            .max(WAYPOINT_GUIDANCE_MIN_CLOSURE_MPS);
                    let mut times_to_go_s = Vec::new();
                    for time_to_go_s in [
                        nominal_time_to_go_s * 0.8,
                        nominal_time_to_go_s,
                        nominal_time_to_go_s * 1.25,
                        cruise_time_to_go_s,
                    ] {
                        self.push_unique_candidate(
                            &mut times_to_go_s,
                            time_to_go_s.clamp(minimum_time_to_go_s, maximum_time_to_go_s),
                        );
                    }
                    let candidate = times_to_go_s
                        .into_iter()
                        .map(|time_to_go_s| {
                            self.waypoint_guidance_candidate_at_endpoint(
                                ctx,
                                observation,
                                guidance,
                                endpoint_m,
                                target_velocity_mps,
                                time_to_go_s,
                                true,
                            )
                        })
                        .filter(|candidate| {
                            Self::waypoint_guidance_candidate_has_control_authority(*candidate)
                        })
                        .min_by(|lhs, rhs| {
                            self.compare_waypoint_guidance_candidates(*lhs, *rhs, true)
                        });
                    let Some(candidate) = candidate else {
                        continue;
                    };
                    let reachable_prediction = self.waypoint_reachable_prediction(
                        ctx,
                        observation,
                        guidance,
                        endpoint_m,
                        candidate.target_velocity_mps,
                        candidate.time_to_go_s,
                    );
                    if reachable_prediction.prediction.assessment.contract_pass() {
                        reachable_candidates.push(WaypointReachableCandidate {
                            candidate,
                            endpoint_m,
                            target_mode: "capture_envelope",
                            reachable_prediction,
                        });
                    }
                }
            }
        }

        reachable_candidates
    }

    fn compare_waypoint_reachable_candidates(
        lhs: WaypointReachableCandidate,
        rhs: WaypointReachableCandidate,
    ) -> std::cmp::Ordering {
        (lhs.reachable_prediction.thrust_saturated_time_s
            + lhs.reachable_prediction.tilt_saturated_time_s)
            .total_cmp(
                &(rhs.reachable_prediction.thrust_saturated_time_s
                    + rhs.reachable_prediction.tilt_saturated_time_s),
            )
            .then_with(|| {
                lhs.reachable_prediction
                    .required_accel_ratio_max
                    .total_cmp(&rhs.reachable_prediction.required_accel_ratio_max)
            })
            .then_with(|| {
                lhs.reachable_prediction
                    .prediction
                    .stats
                    .outbound_heading_error_rad
                    .total_cmp(
                        &rhs.reachable_prediction
                            .prediction
                            .stats
                            .outbound_heading_error_rad,
                    )
            })
            .then_with(|| {
                lhs.candidate
                    .time_to_go_s
                    .total_cmp(&rhs.candidate.time_to_go_s)
            })
            .then_with(|| lhs.endpoint_m.x.total_cmp(&rhs.endpoint_m.x))
            .then_with(|| lhs.endpoint_m.y.total_cmp(&rhs.endpoint_m.y))
            .then_with(|| {
                lhs.candidate
                    .target_velocity_mps
                    .x
                    .total_cmp(&rhs.candidate.target_velocity_mps.x)
            })
            .then_with(|| {
                lhs.candidate
                    .target_velocity_mps
                    .y
                    .total_cmp(&rhs.candidate.target_velocity_mps.y)
            })
    }

    fn waypoint_is_final(ctx: &RunContext, guidance: WaypointGuidanceFrame) -> bool {
        ctx.mission
            .transfer_route
            .as_ref()
            .is_some_and(|route| guidance.active_index + 1 == route.waypoints.len())
    }

    fn target_relative_observation_at_transfer_state(
        ctx: &RunContext,
        observation: &Observation,
        state: TransferSimState,
        elapsed_s: f64,
    ) -> Observation {
        let target_x_m = observation.position_m.x + observation.target_dx_m;
        let height_above_target_m = state.position_m.y - observation.target_surface_y_m;
        let touchdown_clearance_m =
            height_above_target_m - ctx.vehicle.geometry.touchdown_base_offset_m;
        Observation {
            sim_time_s: observation.sim_time_s + elapsed_s,
            physics_step: observation.physics_step,
            position_m: state.position_m,
            velocity_mps: state.velocity_mps,
            attitude_rad: state.attitude_rad,
            angular_rate_radps: 0.0,
            mass_kg: state.mass_kg(),
            fuel_kg: state.fuel_kg,
            gravity_mps2: observation.gravity_mps2,
            target_dx_m: target_x_m - state.position_m.x,
            height_above_target_m,
            target_surface_y_m: observation.target_surface_y_m,
            target_pad_half_width_m: observation.target_pad_half_width_m,
            touchdown_clearance_m,
            min_hull_clearance_m: touchdown_clearance_m,
        }
    }

    fn waypoint_final_candidate(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        reachable: WaypointReachableCandidate,
    ) -> Option<WaypointFinalCandidate> {
        if !reachable
            .reachable_prediction
            .prediction
            .assessment
            .contract_pass()
        {
            return None;
        }
        let terminal_observation = Self::target_relative_observation_at_transfer_state(
            ctx,
            observation,
            reachable.reachable_prediction.event_state,
            reachable.reachable_prediction.prediction.time_to_event_s,
        );
        let diagnostics = self.transfer_diagnostics(&terminal_observation);
        let lateral_dx_m = diagnostics
            .projection
            .projected_dx_m
            .filter(|_| diagnostics.projection.has_target_y_solution)
            .unwrap_or(diagnostics.route_dx_m);
        Some(WaypointFinalCandidate {
            reachable,
            terminal_gate: self.terminal.assess_terminal_entry(
                ctx,
                &terminal_observation,
                TerminalEntryRequest {
                    lateral_dx_m,
                    ready_ticks: 0,
                    terrain_policy: TerminalEntryTerrainPolicy::Ignore,
                },
            ),
        })
    }

    fn compare_waypoint_final_candidates(
        lhs: WaypointFinalCandidate,
        rhs: WaypointFinalCandidate,
    ) -> std::cmp::Ordering {
        (lhs.terminal_gate.required_accel_ratio > 1.0)
            .cmp(&(rhs.terminal_gate.required_accel_ratio > 1.0))
            .then_with(|| {
                lhs.terminal_gate
                    .required_accel_ratio
                    .total_cmp(&rhs.terminal_gate.required_accel_ratio)
            })
            .then_with(|| Self::compare_waypoint_reachable_candidates(lhs.reachable, rhs.reachable))
    }

    fn select_final_waypoint_event_candidate(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
    ) -> Option<WaypointFinalCandidate> {
        self.waypoint_reachable_event_candidates(ctx, observation, guidance)
            .into_iter()
            .filter_map(|candidate| self.waypoint_final_candidate(ctx, observation, candidate))
            .min_by(|lhs, rhs| Self::compare_waypoint_final_candidates(*lhs, *rhs))
    }

    fn select_waypoint_guidance_plan_candidate(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
        contract_aware: bool,
    ) -> WaypointGuidancePlanSelection {
        let candidate =
            self.select_waypoint_guidance_candidate(ctx, observation, guidance, contract_aware);
        let reachable_prediction = self.waypoint_reachable_prediction(
            ctx,
            observation,
            guidance,
            guidance.endpoint_m,
            candidate.target_velocity_mps,
            candidate.time_to_go_s,
        );
        let reachable = WaypointReachableCandidate {
            candidate,
            endpoint_m: guidance.endpoint_m,
            target_mode: "waypoint_center",
            reachable_prediction,
        };
        let terminal_gate = Self::waypoint_is_final(ctx, guidance)
            .then(|| self.waypoint_final_candidate(ctx, observation, reachable))
            .flatten()
            .map(|final_candidate| final_candidate.terminal_gate);
        let fallback = WaypointGuidancePlanSelection {
            candidate,
            endpoint_m: guidance.endpoint_m,
            target_mode: "waypoint_center",
            terminal_gate,
        };
        if !Self::waypoint_is_final(ctx, guidance) {
            return fallback;
        }

        let Some(best) = self.select_final_waypoint_event_candidate(ctx, observation, guidance)
        else {
            return fallback;
        };
        if best.terminal_gate.required_accel_ratio > 1.0 {
            return fallback;
        }
        if let Some(fallback_gate) = terminal_gate {
            let fallback_final = WaypointFinalCandidate {
                reachable,
                terminal_gate: fallback_gate,
            };
            if Self::compare_waypoint_final_candidates(fallback_final, best)
                != std::cmp::Ordering::Greater
            {
                return fallback;
            }
        }

        WaypointGuidancePlanSelection {
            candidate: best.reachable.candidate,
            endpoint_m: best.reachable.endpoint_m,
            target_mode: best.reachable.target_mode,
            terminal_gate: Some(best.terminal_gate),
        }
    }

    fn select_reachable_waypoint_event_candidate(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
    ) -> Option<WaypointReachableCandidate> {
        if Self::waypoint_is_final(ctx, guidance)
            && let Some(candidate) =
                self.select_final_waypoint_event_candidate(ctx, observation, guidance)
            && candidate.terminal_gate.required_accel_ratio <= 1.0
        {
            return Some(candidate.reachable);
        }
        self.waypoint_reachable_event_candidates(ctx, observation, guidance)
            .into_iter()
            .min_by(|lhs, rhs| Self::compare_waypoint_reachable_candidates(*lhs, *rhs))
    }

    fn waypoint_guidance_candidate_for_plan(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
        plan: WaypointGuidancePlan,
    ) -> WaypointGuidanceCandidate {
        self.waypoint_guidance_candidate_at_endpoint(
            ctx,
            observation,
            guidance,
            plan.endpoint_m,
            plan.target_velocity_mps,
            (plan.arrival_time_s - observation.sim_time_s).max(WAYPOINT_GUIDANCE_MIN_TIME_TO_GO_S),
            plan.target_envelope_feasible,
        )
    }

    fn waypoint_guidance_trackability(
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
        plan: WaypointGuidancePlan,
    ) -> WaypointGuidanceTrackability {
        let duration_s = (plan.arrival_time_s - plan.created_time_s).max(1.0e-6);
        let elapsed_s = (observation.sim_time_s - plan.created_time_s).clamp(0.0, duration_s);
        let (reference_position_m, reference_velocity_mps) = waypoint_cubic_reference_state(
            plan.start_position_m,
            plan.start_velocity_mps,
            plan.endpoint_m,
            plan.target_velocity_mps,
            duration_s,
            elapsed_s,
        );
        let position_error_m = observation.position_m - reference_position_m;
        let velocity_error_mps = observation.velocity_mps - reference_velocity_mps;
        WaypointGuidanceTrackability {
            plan_index: plan.waypoint_index,
            plan_revision: plan.revision,
            plan_reason: plan.reason,
            plan_age_s: (observation.sim_time_s - plan.created_time_s).max(0.0),
            reference_position_error_m: position_error_m.length(),
            reference_cross_error_m: vec_cross(position_error_m, guidance.leg_unit),
            reference_velocity_error_mps: velocity_error_mps.length(),
            reference_cross_speed_error_mps: vec_cross(velocity_error_mps, guidance.leg_unit),
        }
    }

    fn waypoint_reachable_prediction(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
        endpoint_m: Vec2,
        target_velocity_mps: Vec2,
        time_to_go_s: f64,
    ) -> WaypointReachablePrediction {
        let initial_state = self.initial_transfer_sim_state(observation);
        let initial_stats = waypoint_leg_stats_from_axes(
            observation.position_m,
            observation.velocity_mps,
            guidance.center_m,
            guidance.leg_unit,
            guidance.handoff_tangent_unit,
        );
        let mut window_open = guidance.envelope.assess(initial_stats).capture_window_open;
        let initial_assessment = guidance
            .envelope
            .assess(initial_stats)
            .with_window_open(window_open);
        if initial_assessment.resolved_in_window(window_open) {
            return WaypointReachablePrediction {
                prediction: WaypointGuidancePrediction {
                    time_to_event_s: 0.0,
                    deadline_lead_s: time_to_go_s.max(0.0),
                    stats: initial_stats,
                    assessment: initial_assessment,
                },
                event_state: initial_state,
                required_accel_ratio_max: 0.0,
                thrust_saturated_time_s: 0.0,
                tilt_saturated_time_s: 0.0,
            };
        }

        let max_tilt_rad = self
            .config
            .boost_tilt_rad
            .max(self.config.uphill_boost_tilt_rad);
        let dt_s = 1.0 / f64::from(ctx.sim.controller_hz.max(1));
        let horizon_s = time_to_go_s.clamp(0.0, WAYPOINT_GUIDANCE_PREDICTION_HORIZON_S);
        let mut state = initial_state;
        let mut elapsed_s = 0.0;
        let mut required_accel_ratio_max: f64 = 0.0;
        let mut thrust_saturated_time_s = 0.0;
        let mut tilt_saturated_time_s = 0.0;
        let mut last_stats = initial_stats;
        let mut last_assessment = initial_assessment;

        while elapsed_s + 1.0e-9 < horizon_s {
            let step_s = (horizon_s - elapsed_s).min(dt_s);
            let remaining_s = (time_to_go_s - elapsed_s).max(WAYPOINT_GUIDANCE_MIN_TIME_TO_GO_S);
            let state_target_accel_mps2 = required_control_accel(StateTargetRequest {
                position_error_m: endpoint_m - state.position_m,
                velocity_mps: state.velocity_mps,
                target_velocity_mps,
                time_to_go_s: remaining_s,
                gravity_mps2: observation.gravity_mps2,
            });
            let mut simulated_observation = observation.clone();
            simulated_observation.position_m = state.position_m;
            simulated_observation.velocity_mps = state.velocity_mps;
            simulated_observation.attitude_rad = state.attitude_rad;
            simulated_observation.mass_kg = state.mass_kg();
            simulated_observation.fuel_kg = state.fuel_kg;
            simulated_observation.sim_time_s = observation.sim_time_s + elapsed_s;
            let path_correction_mps2 = self.waypoint_path_correction_mps2(
                ctx,
                &simulated_observation,
                guidance,
                state_target_accel_mps2,
            );
            let required_accel_mps2 = state_target_accel_mps2 + path_correction_mps2;
            let max_thrust_accel_mps2 = ctx.vehicle.max_thrust_n / state.mass_kg();
            let required_accel_ratio =
                required_accel_mps2.length() / max_thrust_accel_mps2.max(1.0e-6);
            let tilt_feasible = required_accel_mps2.y > 0.0
                && required_accel_mps2.x.abs() <= max_tilt_rad.tan() * required_accel_mps2.y;
            required_accel_ratio_max = required_accel_ratio_max.max(required_accel_ratio);
            if required_accel_ratio > 1.0 {
                thrust_saturated_time_s += step_s;
            }
            if !tilt_feasible {
                tilt_saturated_time_s += step_s;
            }

            let command =
                allocate_accel_command(required_accel_mps2, max_thrust_accel_mps2, max_tilt_rad);
            let max_delta = ctx.vehicle.max_rotation_rate_radps.max(0.0) * step_s;
            let delta = shortest_angle_delta(state.attitude_rad, command.target_attitude_rad);
            state.attitude_rad += delta.clamp(-max_delta, max_delta);
            let throttle_frac = applied_throttle_frac(ctx, command.throttle_frac, state.fuel_kg);
            let fuel_used_kg = (ctx.vehicle.max_fuel_burn_kgps.max(0.0) * throttle_frac * step_s)
                .min(state.fuel_kg);
            state.fuel_kg -= fuel_used_kg;
            let thrust_n = ctx.vehicle.max_thrust_n.max(0.0) * throttle_frac;
            let (sin_a, cos_a) = state.attitude_rad.sin_cos();
            let thrust_accel_mps2 = Vec2::new(
                (thrust_n / state.mass_kg()) * sin_a,
                (thrust_n / state.mass_kg()) * cos_a,
            );
            state.velocity_mps += Vec2::new(
                thrust_accel_mps2.x,
                thrust_accel_mps2.y - observation.gravity_mps2,
            ) * step_s;
            state.position_m += state.velocity_mps * step_s;
            elapsed_s += step_s;

            last_stats = waypoint_leg_stats_from_axes(
                state.position_m,
                state.velocity_mps,
                guidance.center_m,
                guidance.leg_unit,
                guidance.handoff_tangent_unit,
            );
            let assessment = guidance.envelope.assess(last_stats);
            window_open |= assessment.capture_window_open;
            last_assessment = assessment.with_window_open(window_open);
            if last_assessment.resolved_in_window(window_open) {
                break;
            }
        }

        WaypointReachablePrediction {
            prediction: WaypointGuidancePrediction {
                time_to_event_s: elapsed_s,
                deadline_lead_s: (time_to_go_s - elapsed_s).max(0.0),
                stats: last_stats,
                assessment: last_assessment,
            },
            event_state: state,
            required_accel_ratio_max,
            thrust_saturated_time_s,
            tilt_saturated_time_s,
        }
    }

    fn observation_at_transfer_state(
        observation: &Observation,
        state: TransferSimState,
        elapsed_s: f64,
    ) -> Observation {
        let mut projected = observation.clone();
        projected.position_m = state.position_m;
        projected.velocity_mps = state.velocity_mps;
        projected.attitude_rad = state.attitude_rad;
        projected.mass_kg = state.mass_kg();
        projected.fuel_kg = state.fuel_kg;
        projected.sim_time_s = observation.sim_time_s + elapsed_s;
        projected
    }

    fn waypoint_continuation_prediction(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
        current_reachable: WaypointReachablePrediction,
    ) -> Option<WaypointContinuationPrediction> {
        if !current_reachable.prediction.assessment.contract_pass() {
            return None;
        }
        let next_waypoint_index = guidance.active_index + 1;
        let next_observation = Self::observation_at_transfer_state(
            observation,
            current_reachable.event_state,
            current_reachable.prediction.time_to_event_s,
        );
        let (prediction, passing_candidate_count) =
            self.waypoint_leg_reachability(ctx, &next_observation, next_waypoint_index)?;
        Some(WaypointContinuationPrediction {
            next_waypoint_index,
            source_event_state: current_reachable.event_state,
            source_event_time_s: observation.sim_time_s
                + current_reachable.prediction.time_to_event_s,
            prediction,
            passing_candidate_count,
        })
    }

    fn waypoint_leg_reachability(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        waypoint_index: usize,
    ) -> Option<(WaypointReachablePrediction, usize)> {
        let next_geometry = Self::waypoint_leg_geometry_at(ctx, waypoint_index)?;
        let next_stats = waypoint_leg_stats(observation, &next_geometry);
        let next_approach =
            self.waypoint_approach_state(ctx, observation, &next_geometry, next_stats);
        let next_guidance = waypoint_guidance_frame(&next_geometry, next_stats, next_approach);
        let passing_candidates =
            self.waypoint_reachable_event_candidates(ctx, observation, next_guidance);
        let passing_candidate_count = passing_candidates.len();
        let prediction = passing_candidates
            .into_iter()
            .min_by(|lhs, rhs| {
                self.compare_waypoint_guidance_candidates(lhs.candidate, rhs.candidate, true)
            })
            .map(|candidate| candidate.reachable_prediction)
            .unwrap_or_else(|| {
                let candidate =
                    self.select_waypoint_guidance_candidate(ctx, observation, next_guidance, true);
                self.waypoint_reachable_prediction(
                    ctx,
                    observation,
                    next_guidance,
                    next_guidance.endpoint_m,
                    candidate.target_velocity_mps,
                    candidate.time_to_go_s,
                )
            });
        Some((prediction, passing_candidate_count))
    }

    fn waypoint_transition_audit(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        continuation: WaypointContinuationPrediction,
    ) -> Option<WaypointTransitionAudit> {
        let (continuation_prediction, passing_candidate_count) =
            self.waypoint_leg_reachability(ctx, observation, continuation.next_waypoint_index)?;
        Some(WaypointTransitionAudit {
            next_waypoint_index: continuation.next_waypoint_index,
            position_error_m: (observation.position_m - continuation.source_event_state.position_m)
                .length(),
            velocity_error_mps: (observation.velocity_mps
                - continuation.source_event_state.velocity_mps)
                .length(),
            attitude_error_rad: shortest_angle_delta(
                continuation.source_event_state.attitude_rad,
                observation.attitude_rad,
            )
            .abs(),
            mass_error_kg: (observation.mass_kg - continuation.source_event_state.mass_kg()).abs(),
            fuel_error_kg: (observation.fuel_kg - continuation.source_event_state.fuel_kg).abs(),
            event_time_error_s: observation.sim_time_s - continuation.source_event_time_s,
            continuation_prediction,
            passing_candidate_count,
        })
    }

    fn compare_waypoint_joint_candidates(
        lhs: WaypointJointCandidatePrediction,
        rhs: WaypointJointCandidatePrediction,
    ) -> std::cmp::Ordering {
        (!lhs.contract_pass())
            .cmp(&(!rhs.contract_pass()))
            .then_with(|| {
                lhs.total_saturated_time_s()
                    .total_cmp(&rhs.total_saturated_time_s())
            })
            .then_with(|| {
                lhs.required_accel_ratio_max()
                    .total_cmp(&rhs.required_accel_ratio_max())
            })
            .then_with(|| {
                lhs.continuation
                    .prediction
                    .stats
                    .outbound_heading_error_rad
                    .total_cmp(&rhs.continuation.prediction.stats.outbound_heading_error_rad)
            })
            .then_with(|| {
                lhs.current
                    .candidate
                    .time_to_go_s
                    .total_cmp(&rhs.current.candidate.time_to_go_s)
            })
            .then_with(|| {
                lhs.current
                    .endpoint_m
                    .x
                    .total_cmp(&rhs.current.endpoint_m.x)
            })
            .then_with(|| {
                lhs.current
                    .endpoint_m
                    .y
                    .total_cmp(&rhs.current.endpoint_m.y)
            })
            .then_with(|| {
                lhs.current
                    .candidate
                    .target_velocity_mps
                    .x
                    .total_cmp(&rhs.current.candidate.target_velocity_mps.x)
            })
            .then_with(|| {
                lhs.current
                    .candidate
                    .target_velocity_mps
                    .y
                    .total_cmp(&rhs.current.candidate.target_velocity_mps.y)
            })
    }

    fn waypoint_joint_search_prediction(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
    ) -> Option<WaypointJointSearchPrediction> {
        let next_waypoint_index = guidance.active_index + 1;
        Self::waypoint_leg_geometry_at(ctx, next_waypoint_index)?;

        let mut current_candidates =
            self.waypoint_reachable_event_candidates(ctx, observation, guidance);
        current_candidates
            .sort_by(|lhs, rhs| Self::compare_waypoint_reachable_candidates(*lhs, *rhs));
        current_candidates.truncate(WAYPOINT_JOINT_MAX_CURRENT_CANDIDATES);

        let mut joint_candidates = Vec::with_capacity(current_candidates.len());
        for current in current_candidates {
            let next_observation = Self::observation_at_transfer_state(
                observation,
                current.reachable_prediction.event_state,
                current.reachable_prediction.prediction.time_to_event_s,
            );
            let Some((continuation, continuation_passing_candidate_count)) =
                self.waypoint_leg_reachability(ctx, &next_observation, next_waypoint_index)
            else {
                continue;
            };
            joint_candidates.push(WaypointJointCandidatePrediction {
                current,
                continuation,
                continuation_passing_candidate_count,
            });
        }

        let evaluated_candidate_count = joint_candidates.len();
        let passing_candidate_count = joint_candidates
            .iter()
            .filter(|candidate| candidate.contract_pass())
            .count();
        let selected = joint_candidates
            .into_iter()
            .min_by(|lhs, rhs| Self::compare_waypoint_joint_candidates(*lhs, *rhs));
        Some(WaypointJointSearchPrediction {
            next_waypoint_index,
            selected,
            passing_candidate_count,
            evaluated_candidate_count,
        })
    }

    fn cached_waypoint_continuation_prediction(
        &mut self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
        plan_revision: u32,
        current_reachable: WaypointReachablePrediction,
    ) -> Option<WaypointContinuationPrediction> {
        if let Some((revision, prediction)) = self.waypoint_continuation_snapshot
            && revision == plan_revision
        {
            return Some(prediction);
        }
        let prediction =
            self.waypoint_continuation_prediction(ctx, observation, guidance, current_reachable)?;
        self.waypoint_continuation_snapshot = Some((plan_revision, prediction));
        Some(prediction)
    }

    fn cached_waypoint_joint_search_prediction(
        &mut self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
        plan_revision: u32,
        current_reachable: WaypointReachablePrediction,
    ) -> Option<WaypointJointSearchPrediction> {
        if let Some((revision, prediction)) = self.waypoint_joint_snapshot
            && revision == plan_revision
        {
            return Some(prediction);
        }
        if !current_reachable.prediction.assessment.contract_pass() {
            return None;
        }
        let prediction = self.waypoint_joint_search_prediction(ctx, observation, guidance)?;
        self.waypoint_joint_snapshot = Some((plan_revision, prediction));
        Some(prediction)
    }

    fn waypoint_guidance_target_state_for_current_plan(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
    ) -> Option<WaypointGuidanceTargetState> {
        let plan = self
            .waypoint_guidance_plan
            .filter(|plan| plan.waypoint_index == guidance.active_index)?;
        let candidate = self.waypoint_guidance_candidate_for_plan(ctx, observation, guidance, plan);
        let path_correction_mps2 = self.waypoint_path_correction_mps2(
            ctx,
            observation,
            guidance,
            candidate.required_accel_mps2,
        );
        let required_accel_mps2 = candidate.required_accel_mps2 + path_correction_mps2;
        let max_thrust_accel_mps2 = ctx.vehicle.max_thrust_n / observation.mass_kg.max(1.0);
        let required_accel_ratio = required_accel_mps2.length() / max_thrust_accel_mps2.max(1.0e-6);
        let max_tilt_rad = self
            .config
            .boost_tilt_rad
            .max(self.config.uphill_boost_tilt_rad);
        let tilt_feasible = required_accel_mps2.y > 0.0
            && required_accel_mps2.x.abs() <= max_tilt_rad.tan() * required_accel_mps2.y;
        let reachable_prediction = self.waypoint_reachable_prediction(
            ctx,
            observation,
            guidance,
            plan.endpoint_m,
            candidate.target_velocity_mps,
            candidate.time_to_go_s,
        );
        let continuation_prediction = self
            .waypoint_continuation_snapshot
            .filter(|(revision, _)| *revision == plan.revision)
            .map(|(_, prediction)| prediction);
        let joint_prediction = self
            .waypoint_joint_snapshot
            .filter(|(revision, _)| *revision == plan.revision)
            .map(|(_, prediction)| prediction);
        Some(WaypointGuidanceTargetState {
            target_velocity_mps: candidate.target_velocity_mps,
            deadline_remaining_s: plan.arrival_time_s - observation.sim_time_s,
            velocity_error_mps: (observation.velocity_mps - candidate.target_velocity_mps).length(),
            feasible: candidate.target_envelope_feasible
                && tilt_feasible
                && required_accel_ratio <= 1.0,
            authority_margin: 1.0 - required_accel_ratio,
            thrust_saturated: required_accel_ratio > 1.0,
            tilt_saturated: !tilt_feasible,
            trackability: Self::waypoint_guidance_trackability(observation, guidance, plan),
            prediction: candidate.prediction,
            reachable_prediction,
            continuation_prediction,
            joint_prediction,
            final_terminal_required_accel_ratio: plan.final_terminal_required_accel_ratio,
            final_terminal_recoverable: plan.final_terminal_recoverable,
        })
    }

    fn current_waypoint_guidance_candidate(
        &mut self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
    ) -> WaypointGuidanceCandidate {
        let plan_is_current = self
            .waypoint_guidance_plan
            .is_some_and(|plan| plan.waypoint_index == guidance.active_index);
        if !plan_is_current {
            self.waypoint_guidance_plan = None;
            self.waypoint_guidance_contract_failure_ticks = 0;
            self.waypoint_reachable_search_attempted_revision = None;
            self.waypoint_continuation_snapshot = None;
            self.waypoint_joint_snapshot = None;
        }

        let current = self.waypoint_guidance_plan.map(|plan| {
            self.waypoint_guidance_candidate_for_plan(ctx, observation, guidance, plan)
        });
        if current.is_some_and(|candidate| candidate.prediction.assessment.contract_pass()) {
            self.waypoint_reference_contract_pass_ever = true;
        }
        let expired = self
            .waypoint_guidance_plan
            .is_some_and(|plan| plan.arrival_time_s <= observation.sim_time_s);
        let contract_failure_confirmed = current.is_some_and(|candidate| {
            if !Self::waypoint_guidance_contract_failure_is_actionable(candidate) {
                self.waypoint_guidance_contract_failure_ticks = 0;
                false
            } else {
                self.waypoint_guidance_contract_failure_ticks = self
                    .waypoint_guidance_contract_failure_ticks
                    .saturating_add(1);
                self.waypoint_guidance_contract_failure_ticks
                    >= WAYPOINT_GUIDANCE_CONTRACT_FAILURE_HYSTERESIS_TICKS
            }
        });
        let needs_replacement = current.is_some_and(|candidate| {
            !Self::waypoint_guidance_candidate_has_control_authority(candidate)
                || contract_failure_confirmed
        });
        if self.waypoint_guidance_plan.is_none() || expired || needs_replacement {
            let replacement = self.select_waypoint_guidance_plan_candidate(
                ctx,
                observation,
                guidance,
                contract_failure_confirmed,
            );
            let mut should_replace = current.is_none_or(|current| {
                Self::should_replace_waypoint_guidance_plan(
                    current,
                    replacement.candidate,
                    expired,
                    contract_failure_confirmed,
                )
            });
            if should_replace
                && let (Some(current), Some(plan)) = (current, self.waypoint_guidance_plan)
            {
                let current_reachable = self.waypoint_reachable_prediction(
                    ctx,
                    observation,
                    guidance,
                    plan.endpoint_m,
                    current.target_velocity_mps,
                    current.time_to_go_s,
                );
                if Self::should_preserve_waypoint_plan_during_authority_recovery(
                    current,
                    current_reachable,
                    plan.reason,
                    expired,
                    self.waypoint_reference_contract_pass_ever,
                ) {
                    should_replace = false;
                }
            }
            if should_replace {
                let reason = Self::waypoint_guidance_plan_reason(current, expired);
                if self.waypoint_guidance_plan.is_some() {
                    self.waypoint_guidance_replan_count =
                        self.waypoint_guidance_replan_count.saturating_add(1);
                }
                self.waypoint_guidance_plan = Some(WaypointGuidancePlan {
                    waypoint_index: guidance.active_index,
                    revision: self.waypoint_guidance_replan_count,
                    reason,
                    created_time_s: observation.sim_time_s,
                    start_position_m: observation.position_m,
                    start_velocity_mps: observation.velocity_mps,
                    endpoint_m: replacement.endpoint_m,
                    target_mode: replacement.target_mode,
                    target_velocity_mps: replacement.candidate.target_velocity_mps,
                    arrival_time_s: observation.sim_time_s + replacement.candidate.time_to_go_s,
                    target_envelope_feasible: replacement.candidate.target_envelope_feasible,
                    final_terminal_required_accel_ratio: replacement
                        .terminal_gate
                        .map(|gate| gate.required_accel_ratio),
                    final_terminal_recoverable: replacement
                        .terminal_gate
                        .map(|gate| gate.required_accel_ratio <= 1.0),
                });
                self.waypoint_guidance_contract_failure_ticks = 0;
            }
        }

        let plan = self
            .waypoint_guidance_plan
            .expect("active waypoint guidance always has a plan");
        let planned_candidate =
            self.waypoint_guidance_candidate_for_plan(ctx, observation, guidance, plan);
        let reachable_prediction = self.waypoint_reachable_prediction(
            ctx,
            observation,
            guidance,
            plan.endpoint_m,
            planned_candidate.target_velocity_mps,
            planned_candidate.time_to_go_s,
        );
        let reachable_failure_actionable = planned_candidate.time_to_go_s
            <= WAYPOINT_GUIDANCE_PREDICTION_HORIZON_S
            && !reachable_prediction.prediction.assessment.contract_pass();
        if reachable_failure_actionable
            && contract_failure_confirmed
            && !self.waypoint_reference_contract_pass_ever
            && self.waypoint_reachable_search_attempted_revision != Some(plan.revision)
        {
            self.waypoint_reachable_search_attempted_revision = Some(plan.revision);
            if let Some(replacement) =
                self.select_reachable_waypoint_event_candidate(ctx, observation, guidance)
            {
                let terminal_gate = Self::waypoint_is_final(ctx, guidance)
                    .then(|| self.waypoint_final_candidate(ctx, observation, replacement))
                    .flatten()
                    .map(|candidate| candidate.terminal_gate);
                self.waypoint_guidance_replan_count =
                    self.waypoint_guidance_replan_count.saturating_add(1);
                self.waypoint_guidance_plan = Some(WaypointGuidancePlan {
                    waypoint_index: guidance.active_index,
                    revision: self.waypoint_guidance_replan_count,
                    reason: WaypointGuidancePlanReason::ReachableRecovery,
                    created_time_s: observation.sim_time_s,
                    start_position_m: observation.position_m,
                    start_velocity_mps: observation.velocity_mps,
                    endpoint_m: replacement.endpoint_m,
                    target_mode: replacement.target_mode,
                    target_velocity_mps: replacement.candidate.target_velocity_mps,
                    arrival_time_s: observation.sim_time_s + replacement.candidate.time_to_go_s,
                    target_envelope_feasible: replacement.candidate.target_envelope_feasible,
                    final_terminal_required_accel_ratio: terminal_gate
                        .map(|gate| gate.required_accel_ratio),
                    final_terminal_recoverable: terminal_gate
                        .map(|gate| gate.required_accel_ratio <= 1.0),
                });
                self.waypoint_guidance_contract_failure_ticks = 0;
                self.waypoint_reachable_search_attempted_revision = None;
            }
        }

        self.waypoint_guidance_candidate_for_plan(
            ctx,
            observation,
            guidance,
            self.waypoint_guidance_plan
                .expect("active waypoint guidance always has a plan"),
        )
    }

    fn waypoint_path_correction_mps2(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
        state_target_accel_mps2: Vec2,
    ) -> Vec2 {
        let speed_mps = observation.velocity_mps.length();
        let to_steering_target_m = guidance.steering_target_m - observation.position_m;
        let lookahead_m = to_steering_target_m.length();
        if speed_mps < WAYPOINT_GUIDANCE_L1_MIN_SPEED_MPS
            || lookahead_m <= WAYPOINT_GUIDANCE_UNIQUE_EPS
        {
            return Vec2::new(0.0, 0.0);
        }

        let velocity_unit = observation.velocity_mps * (1.0 / speed_mps);
        let lookahead_unit = to_steering_target_m * (1.0 / lookahead_m);
        let left_normal = Vec2::new(-velocity_unit.y, velocity_unit.x);
        let lateral_accel_mps2 =
            2.0 * speed_mps * speed_mps / lookahead_m * vec_cross(velocity_unit, lookahead_unit);
        let fade = (guidance.approach.remaining_to_plane_m
            / guidance.approach.shaping_start_distance_m.max(1.0))
        .clamp(0.0, 1.0);
        let max_thrust_accel_mps2 = ctx.vehicle.max_thrust_n / observation.mass_kg.max(1.0);
        let authority_cap_mps2 = max_thrust_accel_mps2 * WAYPOINT_GUIDANCE_PATH_AUTHORITY_FRAC;
        let remaining_authority_mps2 =
            (max_thrust_accel_mps2 - state_target_accel_mps2.length()).max(0.0);
        let cap_mps2 = authority_cap_mps2.min(remaining_authority_mps2) * fade;
        let raw = left_normal * lateral_accel_mps2;
        let raw_length = raw.length();
        if raw_length <= cap_mps2 || raw_length <= WAYPOINT_GUIDANCE_UNIQUE_EPS {
            raw
        } else {
            raw * (cap_mps2 / raw_length)
        }
    }

    fn waypoint_guidance_command_state(
        &mut self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
    ) -> WaypointGuidanceCommandState {
        let candidate = self.current_waypoint_guidance_candidate(ctx, observation, guidance);
        let path_correction_mps2 = self.waypoint_path_correction_mps2(
            ctx,
            observation,
            guidance,
            candidate.required_accel_mps2,
        );
        let required_accel_mps2 = candidate.required_accel_mps2 + path_correction_mps2;
        let max_thrust_accel_mps2 = ctx.vehicle.max_thrust_n / observation.mass_kg.max(1.0);
        let required_accel_ratio = required_accel_mps2.length() / max_thrust_accel_mps2.max(1.0e-6);
        let max_tilt_rad = self
            .config
            .boost_tilt_rad
            .max(self.config.uphill_boost_tilt_rad);
        let tilt_feasible = required_accel_mps2.y > 0.0
            && required_accel_mps2.x.abs() <= max_tilt_rad.tan() * required_accel_mps2.y;
        let plan = self
            .waypoint_guidance_plan
            .expect("active waypoint guidance always has a plan");
        let reachable_prediction = self.waypoint_reachable_prediction(
            ctx,
            observation,
            guidance,
            plan.endpoint_m,
            candidate.target_velocity_mps,
            candidate.time_to_go_s,
        );
        let continuation_prediction = self.cached_waypoint_continuation_prediction(
            ctx,
            observation,
            guidance,
            plan.revision,
            reachable_prediction,
        );
        let joint_prediction = self.cached_waypoint_joint_search_prediction(
            ctx,
            observation,
            guidance,
            plan.revision,
            reachable_prediction,
        );
        WaypointGuidanceCommandState {
            command: allocate_accel_command(
                required_accel_mps2,
                max_thrust_accel_mps2,
                max_tilt_rad,
            ),
            target_velocity_mps: candidate.target_velocity_mps,
            time_to_go_s: candidate.time_to_go_s,
            required_accel_ratio,
            feasible: candidate.target_envelope_feasible
                && tilt_feasible
                && required_accel_ratio <= 1.0,
            path_correction_mps2,
            deadline_remaining_s: plan.arrival_time_s - observation.sim_time_s,
            velocity_error_mps: (observation.velocity_mps - candidate.target_velocity_mps).length(),
            authority_margin: 1.0 - required_accel_ratio,
            thrust_saturated: required_accel_ratio > 1.0,
            tilt_saturated: !tilt_feasible,
            trackability: Self::waypoint_guidance_trackability(observation, guidance, plan),
            prediction: candidate.prediction,
            reachable_prediction,
            continuation_prediction,
            joint_prediction,
            final_terminal_required_accel_ratio: plan.final_terminal_required_accel_ratio,
            final_terminal_recoverable: plan.final_terminal_recoverable,
        }
    }

    fn waypoint_pending_gate(
        &self,
        command_state: Option<WaypointGuidanceCommandState>,
    ) -> TerminalEntryAssessment {
        TerminalEntryAssessment {
            mode: TerminalEntryMode::Pending,
            ready_ticks: 0,
            burn_time_s: command_state.map_or(0.0, |state| state.time_to_go_s),
            latest_safe_margin_s: -1.0,
            required_accel_ratio: command_state.map_or(0.0, |state| state.required_accel_ratio),
            terrain_min_clearance_m: 1.0e9,
            terrain_clearance_safe: true,
            deferred: false,
        }
    }

    fn update_active_waypoint_frame(
        &mut self,
        ctx: &RunContext,
        observation: &Observation,
        guidance: WaypointGuidanceFrame,
        telemetry: WaypointTelemetry,
    ) -> ControllerFrame {
        let endpoint_observation = waypoint_adjusted_observation(
            observation,
            guidance.endpoint_m,
            guidance.envelope.capture_radius_m,
        );
        let diagnostics = self.transfer_diagnostics(&endpoint_observation);
        let corridor = TransferCorridorState::inactive();
        let takeoff = self.waypoint_takeoff_required(ctx, observation, guidance);
        let command_state =
            (!takeoff).then(|| self.waypoint_guidance_command_state(ctx, observation, guidance));
        let gate = self.waypoint_pending_gate(command_state);
        self.transfer_gate_ready_ticks = 0;
        self.last_transfer_gate = Some(gate);
        self.last_corridor = corridor;

        let phase = if takeoff {
            TransferPhase::Takeoff
        } else {
            TransferPhase::Boost
        };
        self.phase = phase;
        if !takeoff && self.boost_anchor.is_none() {
            self.boost_anchor = Some(TransferBoostAnchor {
                route_dx_m: endpoint_observation.target_dx_m,
                route_dy_m: -endpoint_observation.height_above_target_m,
            });
        }
        let command = command_state.map_or_else(
            || self.waypoint_takeoff_command(ctx, observation, guidance),
            |state| state.command,
        );
        let status = if takeoff {
            "lifting off for waypoint leg"
        } else {
            "guiding active waypoint handoff"
        };
        let mut frame = self.frame_for_open_loop_phase(
            ctx,
            &endpoint_observation,
            phase,
            command,
            status,
            diagnostics,
            gate,
            corridor,
            None,
        );
        self.insert_waypoint_metrics(&mut frame, Some(telemetry));
        if let Some(state) = command_state {
            frame.metrics.insert(
                metric::GUIDANCE_MODE.to_owned(),
                TelemetryValue::from("waypoint_state_target"),
            );
            frame.metrics.insert(
                metric::GUIDANCE_BURN_TIME_S.to_owned(),
                TelemetryValue::from(state.time_to_go_s),
            );
            frame.metrics.insert(
                metric::GUIDANCE_REQUIRED_ACCEL_RATIO.to_owned(),
                TelemetryValue::from(state.required_accel_ratio),
            );
            frame.metrics.insert(
                metric::WAYPOINT_GUIDANCE_MODE.to_owned(),
                TelemetryValue::from("state_target"),
            );
            frame.metrics.insert(
                metric::WAYPOINT_TARGET_VX_MPS.to_owned(),
                TelemetryValue::from(state.target_velocity_mps.x),
            );
            frame.metrics.insert(
                metric::WAYPOINT_TARGET_VY_MPS.to_owned(),
                TelemetryValue::from(state.target_velocity_mps.y),
            );
            frame.metrics.insert(
                metric::WAYPOINT_TARGET_SPEED_MPS.to_owned(),
                TelemetryValue::from(state.target_velocity_mps.length()),
            );
            frame.metrics.insert(
                metric::WAYPOINT_GUIDANCE_TIME_TO_GO_S.to_owned(),
                TelemetryValue::from(state.time_to_go_s),
            );
            frame.metrics.insert(
                metric::WAYPOINT_GUIDANCE_REQUIRED_ACCEL_RATIO.to_owned(),
                TelemetryValue::from(state.required_accel_ratio),
            );
            frame.metrics.insert(
                metric::WAYPOINT_GUIDANCE_FEASIBLE.to_owned(),
                TelemetryValue::from(state.feasible),
            );
            frame.metrics.insert(
                metric::WAYPOINT_PATH_CORRECTION_MPS2.to_owned(),
                TelemetryValue::from(state.path_correction_mps2.length()),
            );
            frame.metrics.insert(
                metric::WAYPOINT_GUIDANCE_REPLAN_COUNT.to_owned(),
                TelemetryValue::from(i64::from(self.waypoint_guidance_replan_count)),
            );
            insert_waypoint_target_state_metrics(
                &mut frame.metrics,
                WaypointGuidanceTargetState {
                    target_velocity_mps: state.target_velocity_mps,
                    deadline_remaining_s: state.deadline_remaining_s,
                    velocity_error_mps: state.velocity_error_mps,
                    feasible: state.feasible,
                    authority_margin: state.authority_margin,
                    thrust_saturated: state.thrust_saturated,
                    tilt_saturated: state.tilt_saturated,
                    trackability: state.trackability,
                    prediction: state.prediction,
                    reachable_prediction: state.reachable_prediction,
                    continuation_prediction: state.continuation_prediction,
                    joint_prediction: state.joint_prediction,
                    final_terminal_required_accel_ratio: state.final_terminal_required_accel_ratio,
                    final_terminal_recoverable: state.final_terminal_recoverable,
                },
            );
        }
        frame
    }

    fn transfer_boost_quality(
        &self,
        route_dx_m: f64,
        route_dy_m: f64,
        projection: TransferBallisticProjection,
    ) -> TransferBoostQuality {
        let apex_target_over_target_m =
            self.boost_apex_target_over_target_m(route_dx_m, route_dy_m);
        let (verdict, passed) = if !projection.has_target_y_solution {
            ("no_target_y_solution", false)
        } else if projection.projected_dx_m.is_none_or(|projected_dx_m| {
            projected_dx_m.abs() > self.config.boost_projected_dx_limit_m
        }) {
            ("dx", false)
        } else if projection.impact_angle_deg.is_none_or(|impact_angle_deg| {
            impact_angle_deg < self.config.boost_descent_angle_min_deg
        }) {
            ("angle", false)
        } else {
            ("pass", true)
        };
        TransferBoostQuality {
            verdict,
            passed,
            apex_target_over_target_m,
        }
    }

    fn boost_apex_target_over_target_m(&self, route_dx_m: f64, route_dy_m: f64) -> f64 {
        let base = (self.config.boost_apex_height_per_dx * route_dx_m.abs()).clamp(
            self.config.boost_apex_height_min_m,
            self.config.boost_apex_height_max_m,
        );
        base + (route_dy_m * self.config.boost_apex_height_per_uphill_dy).max(0.0)
            + (-route_dy_m).max(0.0)
    }

    fn boost_scoring_mode(&self) -> &'static str {
        self.boost_scoring_mode.label()
    }

    fn transfer_metrics_builder(
        &self,
        builder: ControllerFrameBuilder,
        diagnostics: TransferDiagnostics,
        gate: TerminalEntryAssessment,
        corridor: TransferCorridorState,
        boost_selection: Option<TransferBoostCommandSelection>,
    ) -> ControllerFrameBuilder {
        let builder = builder
            .metric(metric::TRANSFER_ROUTE_DX_M, diagnostics.route_dx_m)
            .metric(metric::TRANSFER_ROUTE_DY_M, diagnostics.route_dy_m)
            .metric(
                metric::TRANSFER_SHAPE_ANCHOR_DX_M,
                diagnostics
                    .anchor
                    .map(|anchor| anchor.route_dx_m)
                    .unwrap_or(diagnostics.route_dx_m),
            )
            .metric(
                metric::TRANSFER_SHAPE_ANCHOR_DY_M,
                diagnostics
                    .anchor
                    .map(|anchor| anchor.route_dy_m)
                    .unwrap_or(diagnostics.route_dy_m),
            )
            .metric(
                metric::TRANSFER_TARGET_Y_SOLUTION,
                diagnostics.projection.has_target_y_solution,
            )
            .metric(
                metric::TRANSFER_PROJECTED_TIME_S,
                diagnostics.projection.projected_time_s.unwrap_or(-1.0),
            )
            .metric(
                metric::TRANSFER_PROJECTED_DX_M,
                diagnostics
                    .projection
                    .projected_dx_m
                    .unwrap_or(diagnostics.route_dx_m),
            )
            .metric(
                metric::TRANSFER_IMPACT_ANGLE_DEG,
                diagnostics.projection.impact_angle_deg.unwrap_or(-1.0),
            )
            .metric(
                metric::TRANSFER_APEX_OVER_TARGET_M,
                diagnostics.projection.apex_over_target_m,
            )
            .metric(
                metric::TRANSFER_BOOST_APEX_TARGET_M,
                diagnostics.boost_quality.apex_target_over_target_m,
            )
            .metric(
                metric::TRANSFER_BOOST_QUALITY,
                diagnostics.boost_quality.verdict,
            )
            .metric(
                metric::TRANSFER_BOOST_QUALITY_PASS,
                diagnostics.boost_quality.passed,
            )
            .metric(
                metric::TRANSFER_BOOST_SCORING_MODE,
                boost_selection
                    .map(|selection| selection.scoring_mode)
                    .unwrap_or_else(|| self.boost_scoring_mode()),
            )
            .metric(metric::TRANSFER_TERMINAL_GATE_MODE, gate.mode.label())
            .metric(
                metric::TRANSFER_TERMINAL_GATE_LATEST_SAFE_MARGIN_S,
                gate.latest_safe_margin_s,
            )
            .metric(
                metric::TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO,
                gate.required_accel_ratio,
            )
            .metric(metric::TRANSFER_TERMINAL_GATE_DEFERRED, gate.deferred)
            .metric(metric::TRANSFER_CORRIDOR_MODE, corridor.mode)
            .metric(metric::TRANSFER_CORRIDOR_MARGIN_M, corridor.margin_m);

        if let Some(selection) = boost_selection {
            builder
                .metric(
                    metric::TRANSFER_BOOST_SELECTED_SCORE,
                    selection.selected_score,
                )
                .metric(
                    metric::TRANSFER_BOOST_SETTLED_QUALITY,
                    selection.settled_quality.verdict,
                )
                .metric(
                    metric::TRANSFER_BOOST_SETTLED_PROJECTED_DX_M,
                    selection
                        .settled_projection
                        .projected_dx_m
                        .unwrap_or(diagnostics.route_dx_m),
                )
        } else {
            builder
        }
    }

    fn insert_transfer_metrics(
        &self,
        frame: &mut ControllerFrame,
        diagnostics: TransferDiagnostics,
        gate: TerminalEntryAssessment,
        corridor: TransferCorridorState,
    ) {
        frame.metrics.insert(
            metric::TRANSFER_ROUTE_DX_M.to_owned(),
            TelemetryValue::from(diagnostics.route_dx_m),
        );
        frame.metrics.insert(
            metric::TRANSFER_ROUTE_DY_M.to_owned(),
            TelemetryValue::from(diagnostics.route_dy_m),
        );
        frame.metrics.insert(
            metric::TRANSFER_SHAPE_ANCHOR_DX_M.to_owned(),
            TelemetryValue::from(
                diagnostics
                    .anchor
                    .map(|anchor| anchor.route_dx_m)
                    .unwrap_or(diagnostics.route_dx_m),
            ),
        );
        frame.metrics.insert(
            metric::TRANSFER_SHAPE_ANCHOR_DY_M.to_owned(),
            TelemetryValue::from(
                diagnostics
                    .anchor
                    .map(|anchor| anchor.route_dy_m)
                    .unwrap_or(diagnostics.route_dy_m),
            ),
        );
        frame.metrics.insert(
            metric::TRANSFER_TARGET_Y_SOLUTION.to_owned(),
            TelemetryValue::from(diagnostics.projection.has_target_y_solution),
        );
        frame.metrics.insert(
            metric::TRANSFER_PROJECTED_TIME_S.to_owned(),
            TelemetryValue::from(diagnostics.projection.projected_time_s.unwrap_or(-1.0)),
        );
        frame.metrics.insert(
            metric::TRANSFER_PROJECTED_DX_M.to_owned(),
            TelemetryValue::from(
                diagnostics
                    .projection
                    .projected_dx_m
                    .unwrap_or(diagnostics.route_dx_m),
            ),
        );
        frame.metrics.insert(
            metric::TRANSFER_IMPACT_ANGLE_DEG.to_owned(),
            TelemetryValue::from(diagnostics.projection.impact_angle_deg.unwrap_or(-1.0)),
        );
        frame.metrics.insert(
            metric::TRANSFER_APEX_OVER_TARGET_M.to_owned(),
            TelemetryValue::from(diagnostics.projection.apex_over_target_m),
        );
        frame.metrics.insert(
            metric::TRANSFER_BOOST_APEX_TARGET_M.to_owned(),
            TelemetryValue::from(diagnostics.boost_quality.apex_target_over_target_m),
        );
        frame.metrics.insert(
            metric::TRANSFER_BOOST_QUALITY.to_owned(),
            TelemetryValue::from(diagnostics.boost_quality.verdict),
        );
        frame.metrics.insert(
            metric::TRANSFER_BOOST_QUALITY_PASS.to_owned(),
            TelemetryValue::from(diagnostics.boost_quality.passed),
        );
        frame.metrics.insert(
            metric::TRANSFER_BOOST_SCORING_MODE.to_owned(),
            TelemetryValue::from(self.boost_scoring_mode()),
        );
        frame.metrics.insert(
            metric::TRANSFER_TERMINAL_GATE_MODE.to_owned(),
            TelemetryValue::from(gate.mode.label()),
        );
        frame.metrics.insert(
            metric::TRANSFER_TERMINAL_GATE_LATEST_SAFE_MARGIN_S.to_owned(),
            TelemetryValue::from(gate.latest_safe_margin_s),
        );
        frame.metrics.insert(
            metric::TRANSFER_TERMINAL_GATE_REQUIRED_ACCEL_RATIO.to_owned(),
            TelemetryValue::from(gate.required_accel_ratio),
        );
        frame.metrics.insert(
            metric::TRANSFER_TERMINAL_GATE_DEFERRED.to_owned(),
            TelemetryValue::from(gate.deferred),
        );
        frame.metrics.insert(
            metric::TRANSFER_CORRIDOR_MODE.to_owned(),
            TelemetryValue::from(corridor.mode),
        );
        frame.metrics.insert(
            metric::TRANSFER_CORRIDOR_MARGIN_M.to_owned(),
            TelemetryValue::from(corridor.margin_m),
        );
    }

    fn transfer_gate_readiness(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
    ) -> TerminalEntryAssessment {
        let lateral_dx_m = diagnostics
            .projection
            .projected_dx_m
            .filter(|_| diagnostics.projection.has_target_y_solution)
            .unwrap_or(diagnostics.route_dx_m);
        let gate = self.terminal.assess_terminal_entry(
            ctx,
            observation,
            TerminalEntryRequest {
                lateral_dx_m,
                ready_ticks: self.transfer_gate_ready_ticks,
                terrain_policy: TerminalEntryTerrainPolicy::Configured,
            },
        );

        if !diagnostics.projection.has_target_y_solution || observation.height_above_target_m <= 0.0
        {
            return gate.forced_pending();
        }

        if self.should_defer_latest_safe_transfer_gate(ctx, observation, diagnostics, gate) {
            gate.deferred_pending()
        } else {
            gate
        }
    }

    fn should_defer_latest_safe_transfer_gate(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        gate: TerminalEntryAssessment,
    ) -> bool {
        if gate.mode != TerminalEntryMode::LatestSafe {
            return false;
        }
        if gate.latest_safe_margin_s < TRANSFER_GATE_DEFER_MAX_NEGATIVE_MARGIN_S {
            return false;
        }
        if observation.velocity_mps.y <= 0.0 || !diagnostics.projection.has_target_y_solution {
            return false;
        }
        let Some(projected_dx_m) = diagnostics.projection.projected_dx_m else {
            return false;
        };
        let dx_tolerance_m = self.config.boost_projected_dx_limit_m.max(1.0);
        if projected_dx_m.abs() > dx_tolerance_m {
            return false;
        }

        let lookahead_s = self.config.transfer_gate_defer_lookahead_s.max(0.0);
        let step_s = self
            .config
            .transfer_gate_defer_step_s
            .clamp(1.0e-3, lookahead_s.max(1.0e-3));
        let mut elapsed_s = 0.0;
        let mut ready_ticks = self.transfer_gate_ready_ticks;
        while elapsed_s + 1.0e-9 < lookahead_s {
            elapsed_s = (elapsed_s + step_s).min(lookahead_s);
            let predicted = self.passive_coast_observation(ctx, observation, elapsed_s);
            if predicted.height_above_target_m <= 0.0 || predicted.velocity_mps.y <= 0.0 {
                return false;
            }
            let predicted_diagnostics = self.transfer_diagnostics(&predicted);
            if !predicted_diagnostics.projection.has_target_y_solution {
                return false;
            }
            let Some(predicted_projected_dx_m) = predicted_diagnostics.projection.projected_dx_m
            else {
                return false;
            };
            if predicted_projected_dx_m.abs() > dx_tolerance_m {
                return false;
            }

            let future_gate = self.transfer_gate_readiness_without_deferral(
                ctx,
                &predicted,
                predicted_diagnostics,
                ready_ticks,
            );
            if !future_gate.terrain_clearance_safe {
                return false;
            }
            if future_gate.mode == TerminalEntryMode::NominalReady {
                return true;
            }
            let ratio_improvement = gate.required_accel_ratio - future_gate.required_accel_ratio;
            if future_gate.mode == TerminalEntryMode::LatestSafe
                && ratio_improvement >= self.config.transfer_gate_defer_min_ratio_improvement
            {
                return true;
            }
            ready_ticks = future_gate.ready_ticks;
        }

        false
    }

    fn transfer_gate_readiness_without_deferral(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        ready_ticks: u32,
    ) -> TerminalEntryAssessment {
        let lateral_dx_m = diagnostics
            .projection
            .projected_dx_m
            .filter(|_| diagnostics.projection.has_target_y_solution)
            .unwrap_or(diagnostics.route_dx_m);
        let gate = self.terminal.assess_terminal_entry(
            ctx,
            observation,
            TerminalEntryRequest {
                lateral_dx_m,
                ready_ticks,
                terrain_policy: TerminalEntryTerrainPolicy::Configured,
            },
        );
        if !diagnostics.projection.has_target_y_solution || observation.height_above_target_m <= 0.0
        {
            gate.forced_pending()
        } else {
            gate
        }
    }

    fn transfer_corridor_state(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
    ) -> TransferCorridorState {
        if diagnostics.route_dy_m < self.config.uphill_boost_dy_min_m {
            return TransferCorridorState::inactive();
        }

        let route_dx_m = diagnostics.route_dx_m;
        if route_dx_m.abs() <= f64::EPSILON {
            return TransferCorridorState::inactive();
        }

        let x0_m = observation.position_m.x;
        let lookahead_m = (route_dx_m.abs() * TRANSFER_UPHILL_CORRIDOR_LOOKAHEAD_FRAC)
            .clamp(
                TRANSFER_UPHILL_CORRIDOR_LOOKAHEAD_MIN_M,
                TRANSFER_UPHILL_CORRIDOR_LOOKAHEAD_MAX_M,
            )
            .min(route_dx_m.abs());
        let x1_m = x0_m + (route_dx_m.signum() * lookahead_m);
        let sample_points = (0..=TRANSFER_CORRIDOR_SAMPLE_COUNT)
            .map(|sample_index| sample_index as f64 / TRANSFER_CORRIDOR_SAMPLE_COUNT as f64)
            .map(|t| x0_m + ((x1_m - x0_m) * t))
            .collect::<Vec<_>>();
        let max_terrain_y_m = sample_points
            .iter()
            .map(|x_m| ctx.world.terrain.sample_height(*x_m))
            .fold(f64::NEG_INFINITY, f64::max);
        let max_slope_abs = sample_points
            .iter()
            .map(|x_m| ctx.world.terrain.sample_slope(*x_m).abs())
            .fold(0.0, f64::max);
        let required_y_m = max_terrain_y_m
            + ctx.vehicle.geometry.touchdown_base_offset_m
            + TRANSFER_UPHILL_CORRIDOR_CLEARANCE_MARGIN_M;
        let margin_m = observation.position_m.y - required_y_m;
        let tilt_limited =
            margin_m < 0.0 && max_slope_abs >= TRANSFER_UPHILL_CORRIDOR_TILT_SLOPE_MIN;
        if margin_m < 0.0 {
            TransferCorridorState {
                mode: "active",
                active: true,
                tilt_limited,
                margin_m,
            }
        } else {
            TransferCorridorState {
                mode: "clear",
                active: false,
                tilt_limited: false,
                margin_m,
            }
        }
    }

    fn source_clearance_hold_needed(&self, ctx: &RunContext, observation: &Observation) -> bool {
        if self.config.source_clearance_margin_m <= 0.0 {
            return false;
        }
        if observation.target_dx_m.abs() <= observation.target_pad_half_width_m {
            return false;
        }

        let direction = observation.target_dx_m.signum();
        if direction == 0.0 {
            return false;
        }

        let lookahead_m = self
            .config
            .source_clearance_lookahead_m
            .max(0.0)
            .min(observation.target_dx_m.abs());
        if lookahead_m <= f64::EPSILON {
            return false;
        }

        let x0_m = observation.position_m.x;
        let x1_m = x0_m + (direction * lookahead_m);
        let max_terrain_y_m = (0..=TRANSFER_SOURCE_CLEARANCE_SAMPLE_COUNT)
            .map(|sample_index| {
                let t = sample_index as f64 / TRANSFER_SOURCE_CLEARANCE_SAMPLE_COUNT as f64;
                x0_m + ((x1_m - x0_m) * t)
            })
            .map(|x_m| ctx.world.terrain.sample_height(x_m))
            .fold(f64::NEG_INFINITY, f64::max);
        let required_y_m = max_terrain_y_m
            + ctx.vehicle.geometry.touchdown_base_offset_m
            + self.config.source_clearance_margin_m;

        observation.position_m.y < required_y_m
    }

    fn choose_phase(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        gate: TerminalEntryAssessment,
        corridor: TransferCorridorState,
    ) -> TransferPhase {
        let Some(route) = ctx.mission.transfer_route.as_ref() else {
            return TransferPhase::Terminal;
        };
        let Some(source_pad) = ctx.world.landing_pad(&route.source_pad_id) else {
            return TransferPhase::Terminal;
        };

        let source_clearance_m = observation.position_m.y
            - source_pad.surface_y_m
            - ctx.vehicle.geometry.touchdown_base_offset_m;
        if source_clearance_m < self.config.takeoff_clearance_m
            && observation.velocity_mps.y < self.config.takeoff_min_vertical_speed_mps
            && observation.sim_time_s < self.config.max_takeoff_time_s
        {
            return TransferPhase::Takeoff;
        }

        let route_needs_transfer_burn = observation.target_dx_m.abs()
            > self.config.terminal_gate_dx_m
            || diagnostics.route_dy_m > self.config.uphill_boost_dy_min_m;
        let transfer_burn_started = self.boost_anchor.is_some()
            || matches!(self.phase, TransferPhase::Boost | TransferPhase::Coast);
        let completed_waypoint_route =
            !route.waypoints.is_empty() && self.waypoint_active_index >= route.waypoints.len();
        if self.phase == TransferPhase::Terminal && completed_waypoint_route {
            return TransferPhase::Terminal;
        }
        if !route_needs_transfer_burn
            && !transfer_burn_started
            && source_clearance_m < self.config.takeoff_clearance_m
            && self.source_clearance_hold_needed(ctx, observation)
        {
            return TransferPhase::Takeoff;
        }

        if self.phase == TransferPhase::Terminal {
            return TransferPhase::Terminal;
        }

        if !route_needs_transfer_burn && !transfer_burn_started {
            return TransferPhase::Terminal;
        }

        if self.phase != TransferPhase::Coast
            && (route_needs_transfer_burn || transfer_burn_started)
            && (self.boost_should_continue(ctx, observation, diagnostics)
                || self.transfer_recovery_boost_should_continue(observation, diagnostics))
        {
            return TransferPhase::Boost;
        }

        if gate.is_ready() {
            return TransferPhase::Terminal;
        }

        if self.phase == TransferPhase::Coast
            && self.pre_target_terminal_capture_ready(observation, diagnostics, gate)
        {
            return TransferPhase::Terminal;
        }

        if diagnostics.boost_quality.passed
            && self.should_coast(ctx, observation, diagnostics, corridor)
        {
            return TransferPhase::Coast;
        }

        if (route_needs_transfer_burn || transfer_burn_started)
            && self.phase != TransferPhase::Coast
        {
            return TransferPhase::Boost;
        }

        TransferPhase::Coast
    }

    fn pre_target_terminal_capture_ready(
        &self,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        gate: TerminalEntryAssessment,
    ) -> bool {
        if diagnostics.route_dy_m < self.config.uphill_boost_dy_min_m {
            return false;
        }
        if !diagnostics.boost_quality.passed || !diagnostics.projection.has_target_y_solution {
            return false;
        }
        if observation.height_above_target_m >= 0.0 || observation.velocity_mps.y <= 0.0 {
            return false;
        }
        if observation.touchdown_clearance_m <= self.config.coast_min_altitude_m {
            return false;
        }
        if !gate.terrain_clearance_safe {
            return false;
        }
        if gate.latest_safe_margin_s > TRANSFER_PRE_TARGET_CAPTURE_MAX_LATEST_SAFE_MARGIN_S {
            return false;
        }

        let Some(projected_dx_m) = diagnostics.projection.projected_dx_m else {
            return false;
        };
        if projected_dx_m.abs() > self.boost_dx_limit_m(observation) {
            return false;
        }

        self.next_target_y_crossing_time_s(observation)
            .is_some_and(|time_s| time_s <= TRANSFER_PRE_TARGET_CAPTURE_LOOKAHEAD_S)
    }

    fn next_target_y_crossing_time_s(&self, observation: &Observation) -> Option<f64> {
        let gravity_mps2 = observation.gravity_mps2.max(1.0e-6);
        let discriminant = observation.velocity_mps.y * observation.velocity_mps.y
            + (2.0 * gravity_mps2 * observation.height_above_target_m);
        if discriminant < 0.0 {
            return None;
        }
        let sqrt_discriminant = discriminant.sqrt();
        [
            (observation.velocity_mps.y - sqrt_discriminant) / gravity_mps2,
            (observation.velocity_mps.y + sqrt_discriminant) / gravity_mps2,
        ]
        .into_iter()
        .filter(|time_s| *time_s >= 0.0)
        .min_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap())
    }

    fn boost_should_continue(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
    ) -> bool {
        if observation.sim_time_s >= self.config.boost_max_time_s {
            return false;
        }
        if diagnostics.boost_quality.passed
            && self
                .boost_settled_quality(ctx, observation, diagnostics)
                .quality
                .passed
        {
            return false;
        }

        true
    }

    fn transfer_recovery_boost_should_continue(
        &self,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
    ) -> bool {
        diagnostics.route_dy_m >= self.config.uphill_boost_dy_min_m
            && !diagnostics.projection.has_target_y_solution
            && observation.height_above_target_m <= 0.0
    }

    fn should_coast(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        corridor: TransferCorridorState,
    ) -> bool {
        if corridor.active {
            return false;
        }
        if !self
            .boost_settled_quality(ctx, observation, diagnostics)
            .quality
            .passed
        {
            return false;
        }
        let clear_to_coast = observation.touchdown_clearance_m > self.config.coast_min_altitude_m;
        if !clear_to_coast {
            return false;
        }

        if observation.velocity_mps.y > 0.0 {
            return true;
        }

        if observation.target_dx_m.abs() > self.config.terminal_gate_dx_m
            && observation.height_above_target_m > self.config.terminal_gate_altitude_m
            && observation.velocity_mps.y > -18.0
        {
            return true;
        }

        diagnostics.route_dy_m > self.config.uphill_boost_dy_min_m
            && (observation.height_above_target_m < 0.0 || observation.velocity_mps.y > 0.0)
    }

    fn boost_attitude_rad(
        &self,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        corridor: TransferCorridorState,
    ) -> f64 {
        let direction = self.boost_lateral_direction(observation, diagnostics);
        if direction == 0.0 {
            return 0.0;
        }

        let needs_uphill_bias = diagnostics.route_dy_m >= self.config.uphill_boost_dy_min_m
            && (observation.height_above_target_m < 0.0
                || !diagnostics.projection.has_target_y_solution
                || diagnostics.projection.apex_over_target_m
                    < diagnostics.boost_quality.apex_target_over_target_m);
        let tilt_rad = if needs_uphill_bias {
            self.uphill_clearance_limited_boost_tilt_rad(observation, diagnostics)
        } else {
            self.config.boost_tilt_rad
        };
        if let Some(brake_attitude_rad) =
            self.corridor_lateral_brake_attitude_rad(observation, diagnostics, corridor)
        {
            return brake_attitude_rad;
        }
        if corridor.tilt_limited {
            return direction * tilt_rad.min(TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD);
        }

        direction * tilt_rad
    }

    fn boost_lateral_direction(
        &self,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
    ) -> f64 {
        if let Some(projected_dx_m) = diagnostics.projection.projected_dx_m
            && diagnostics.projection.has_target_y_solution
            && projected_dx_m.abs() > self.boost_dx_limit_m(observation)
        {
            return projected_dx_m.signum();
        }

        if let Some(anchor) = diagnostics.anchor {
            let anchor_direction = anchor.route_dx_m.signum();
            if anchor_direction != 0.0 {
                return anchor_direction;
            }
        }

        diagnostics
            .projection
            .projected_dx_m
            .filter(|projected_dx_m| projected_dx_m.abs() > observation.target_pad_half_width_m)
            .map_or_else(|| observation.target_dx_m.signum(), f64::signum)
    }

    fn corridor_lateral_brake_attitude_rad(
        &self,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        corridor: TransferCorridorState,
    ) -> Option<f64> {
        if !corridor.tilt_limited {
            return None;
        }
        let direction = self.boost_lateral_direction(observation, diagnostics);
        if direction == 0.0 {
            return None;
        }
        let targetward_velocity_mps = observation.velocity_mps.x * direction;
        if targetward_velocity_mps <= TRANSFER_UPHILL_CORRIDOR_BRAKE_VX_MPS {
            return None;
        }

        Some(-direction * TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD)
    }

    fn boost_dx_limit_m(&self, observation: &Observation) -> f64 {
        self.config
            .boost_projected_dx_limit_m
            .max(observation.target_pad_half_width_m)
            .max(1.0)
    }

    fn boost_projected_overshoot(
        &self,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
    ) -> bool {
        let route_direction = diagnostics.route_dx_m.signum();
        let Some(projected_dx_m) = diagnostics.projection.projected_dx_m else {
            return false;
        };
        diagnostics.projection.has_target_y_solution
            && route_direction != 0.0
            && projected_dx_m * route_direction < -self.boost_dx_limit_m(observation)
    }

    fn uphill_clearance_limited_boost_tilt_rad(
        &self,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
    ) -> f64 {
        let route_tilt_rad = observation
            .target_dx_m
            .abs()
            .atan2(diagnostics.route_dy_m.max(1.0));
        let steep_tilt_rad = (route_tilt_rad * TRANSFER_UPHILL_STEEP_TILT_SCALE).clamp(
            TRANSFER_UPHILL_STEEP_TILT_MIN_RAD,
            self.config.uphill_boost_tilt_rad,
        );
        let clearance_blend = ((observation.touchdown_clearance_m
            - TRANSFER_UPHILL_CLEARANCE_BLEND_FLOOR_M)
            / (TRANSFER_UPHILL_LOW_CLEARANCE_M - TRANSFER_UPHILL_CLEARANCE_BLEND_FLOOR_M))
            .clamp(0.0, 1.0);

        steep_tilt_rad + ((self.config.uphill_boost_tilt_rad - steep_tilt_rad) * clearance_blend)
    }

    fn boost_throttle_frac(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        corridor: TransferCorridorState,
        target_attitude_rad: f64,
    ) -> f64 {
        if corridor.active {
            return 1.0;
        }

        let apex_excess_m = diagnostics.projection.apex_over_target_m
            - diagnostics.boost_quality.apex_target_over_target_m
            - TRANSFER_BOOST_APEX_THROTTLE_DEADBAND_M;
        if apex_excess_m <= 0.0 {
            return 1.0;
        }

        let max_accel_mps2 = ctx.vehicle.max_thrust_n / observation.mass_kg.max(1.0);
        let attitude_vertical = target_attitude_rad.cos().max(0.2);
        let hover_throttle = (observation.gravity_mps2 / (max_accel_mps2 * attitude_vertical))
            .clamp(ctx.vehicle.min_throttle_frac, 1.0);
        let weight = (apex_excess_m / TRANSFER_BOOST_APEX_THROTTLE_RANGE_M).clamp(0.0, 1.0);
        (1.0 - (weight * (1.0 - hover_throttle))).clamp(ctx.vehicle.min_throttle_frac, 1.0)
    }

    fn select_boost_command(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        gate: TerminalEntryAssessment,
        corridor: TransferCorridorState,
    ) -> TransferBoostCommandSelection {
        let base_attitude = self.boost_attitude_rad(observation, diagnostics, corridor);
        let corridor_brake_attitude =
            self.corridor_lateral_brake_attitude_rad(observation, diagnostics, corridor);
        let eased_throttle =
            self.boost_throttle_frac(ctx, observation, diagnostics, corridor, base_attitude);
        let settled = self.boost_settled_quality(ctx, observation, diagnostics);

        let mut attitude_candidates = Vec::new();
        if let Some(brake_attitude) = corridor_brake_attitude {
            self.push_unique_candidate(&mut attitude_candidates, 0.0);
            self.push_unique_candidate(&mut attitude_candidates, brake_attitude * 0.6);
            self.push_unique_candidate(&mut attitude_candidates, brake_attitude);
        } else if base_attitude.abs() <= 1.0e-6 {
            self.push_unique_candidate(&mut attitude_candidates, 0.0);
        } else {
            self.push_unique_candidate(&mut attitude_candidates, base_attitude * 0.6);
            self.push_unique_candidate(&mut attitude_candidates, base_attitude);
        }
        if corridor_brake_attitude.is_none() {
            let uphill_attitude = self.apply_corridor_tilt_cap(
                self.boost_lateral_direction(observation, diagnostics)
                    * self.uphill_clearance_limited_boost_tilt_rad(observation, diagnostics),
                corridor,
            );
            self.push_unique_candidate(&mut attitude_candidates, uphill_attitude);
        }
        let mut throttle_candidates = Vec::new();
        if corridor.active {
            self.push_unique_candidate(&mut throttle_candidates, 1.0);
        } else {
            for throttle in [0.45, 0.70, 1.0, eased_throttle] {
                self.push_unique_candidate(
                    &mut throttle_candidates,
                    throttle.clamp(ctx.vehicle.min_throttle_frac, 1.0),
                );
            }
        }
        if self.boost_projected_overshoot(observation, diagnostics)
            && self.boost_cut_admissible(gate, corridor)
        {
            self.push_unique_candidate(&mut throttle_candidates, 0.0);
        }

        let mut best_command = Command {
            throttle_frac: eased_throttle,
            target_attitude_rad: base_attitude,
        };
        let mut best_score =
            self.score_boost_candidate(ctx, observation, diagnostics, corridor, best_command);
        for attitude in attitude_candidates {
            for throttle in &throttle_candidates {
                let command = Command {
                    throttle_frac: *throttle,
                    target_attitude_rad: self.apply_corridor_tilt_cap(attitude, corridor),
                };
                let score =
                    self.score_boost_candidate(ctx, observation, diagnostics, corridor, command);
                if score.score < best_score.score {
                    best_command = command;
                    best_score = score;
                }
            }
        }

        TransferBoostCommandSelection {
            command: best_command.clamped(),
            scoring_mode: self.boost_scoring_mode(),
            selected_score: best_score.score,
            settled_projection: settled.projection,
            settled_quality: settled.quality,
        }
    }

    fn boost_cut_admissible(
        &self,
        gate: TerminalEntryAssessment,
        corridor: TransferCorridorState,
    ) -> bool {
        !corridor.active
            && (!self.guidance_mode.uses_waypoints()
                || (gate.terrain_clearance_safe
                    && gate.latest_safe_margin_s > 0.0
                    && gate.required_accel_ratio <= 1.0))
    }

    fn score_boost_candidate(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        corridor: TransferCorridorState,
        command: Command,
    ) -> TransferBoostCandidateScore {
        match self.boost_scoring_mode {
            TransferBoostScoringMode::ExperimentalRecoverability => self
                .score_boost_candidate_recoverability(
                    ctx,
                    observation,
                    diagnostics,
                    corridor,
                    command,
                ),
            TransferBoostScoringMode::ExperimentalPathwise => self.score_boost_candidate_pathwise(
                ctx,
                observation,
                diagnostics,
                corridor,
                command,
            ),
            TransferBoostScoringMode::Endpoint => {
                self.score_boost_candidate_endpoint(ctx, observation, corridor, command)
            }
        }
    }

    fn score_boost_candidate_endpoint(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        corridor: TransferCorridorState,
        command: Command,
    ) -> TransferBoostCandidateScore {
        let simulated = self.simulate_transfer_command(
            ctx,
            observation,
            command,
            self.config.boost_candidate_horizon_s,
            self.config.boost_candidate_step_s,
        );
        let predicted = self.observation_from_sim_state(ctx, observation, simulated);
        let predicted_diagnostics = self.transfer_diagnostics(&predicted);
        let projection = predicted_diagnostics.projection;
        let quality = predicted_diagnostics.boost_quality;
        let score = self.score_boost_candidate_endpoint_terms(
            ctx,
            observation,
            &predicted,
            predicted_diagnostics,
            corridor,
            command,
        );

        TransferBoostCandidateScore {
            score,
            projection,
            quality,
        }
    }

    fn score_boost_candidate_endpoint_terms(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        predicted: &Observation,
        predicted_diagnostics: TransferDiagnostics,
        corridor: TransferCorridorState,
        command: Command,
    ) -> f64 {
        let mut score = self.score_boost_candidate_geometry(
            observation,
            predicted,
            predicted_diagnostics,
            self.boost_dx_limit_m(observation),
        );

        score +=
            self.score_boost_candidate_corridor(ctx, predicted, predicted_diagnostics, corridor);
        score += self.score_boost_candidate_effort(command);
        score
    }

    fn score_boost_candidate_geometry(
        &self,
        observation: &Observation,
        predicted: &Observation,
        predicted_diagnostics: TransferDiagnostics,
        dx_limit_m: f64,
    ) -> f64 {
        let projection = predicted_diagnostics.projection;
        let quality = predicted_diagnostics.boost_quality;
        let mut score = 0.0;

        let projected_dx_m = projection.projected_dx_m.unwrap_or(predicted.target_dx_m);
        if projection.has_target_y_solution {
            let projected_dx_excess_ratio =
                ((projected_dx_m.abs() - dx_limit_m).max(0.0) / dx_limit_m).min(8.0);
            score += TRANSFER_BOOST_SCORE_PROJECTED_DX
                * projected_dx_excess_ratio
                * projected_dx_excess_ratio;
            let centering_ratio = (projected_dx_m.abs() / dx_limit_m).min(8.0);
            score +=
                TRANSFER_BOOST_SCORE_PROJECTED_DX_CENTERING * centering_ratio * centering_ratio;
            if let Some(impact_angle_deg) = projection.impact_angle_deg {
                let min_angle_gap = (self.config.boost_descent_angle_min_deg - impact_angle_deg)
                    .max(0.0)
                    / self.config.boost_descent_angle_min_deg.max(1.0);
                score += TRANSFER_BOOST_SCORE_MIN_ANGLE * min_angle_gap * min_angle_gap;
                let target_angle_gap =
                    (self.config.boost_descent_angle_target_deg - impact_angle_deg).max(0.0)
                        / self.config.boost_descent_angle_target_deg.max(1.0);
                score += TRANSFER_BOOST_SCORE_TARGET_ANGLE * target_angle_gap * target_angle_gap;
            } else {
                score += TRANSFER_BOOST_SCORE_MIN_ANGLE;
            }
        } else {
            score += TRANSFER_BOOST_SCORE_NO_TARGET_Y;
            let no_solution_lateral_ratio = (predicted.target_dx_m.abs() / dx_limit_m).min(20.0);
            score += TRANSFER_BOOST_SCORE_SHORTFALL
                * no_solution_lateral_ratio
                * no_solution_lateral_ratio;
            let current_dx_abs_m = observation.target_dx_m.abs().max(1.0);
            let progress_deficit_ratio =
                (predicted.target_dx_m.abs() / current_dx_abs_m).clamp(0.0, 2.0);
            score +=
                TRANSFER_BOOST_SCORE_PROJECTED_DX * progress_deficit_ratio * progress_deficit_ratio;
        }

        let apex_scale_m = quality.apex_target_over_target_m.abs().max(50.0);
        let apex_error_m = projection.apex_over_target_m - quality.apex_target_over_target_m;
        let apex_error_ratio = (apex_error_m.abs() / apex_scale_m).min(8.0);
        if apex_error_m < 0.0 {
            score += TRANSFER_BOOST_SCORE_APEX_UNDERSHOOT * apex_error_ratio * apex_error_ratio;
        } else {
            score += TRANSFER_BOOST_SCORE_APEX_OVERSHOOT * apex_error_ratio * apex_error_ratio;
        }

        score
    }

    fn score_boost_candidate_corridor(
        &self,
        ctx: &RunContext,
        predicted: &Observation,
        predicted_diagnostics: TransferDiagnostics,
        corridor: TransferCorridorState,
    ) -> f64 {
        let mut score = 0.0;
        if corridor.active {
            let predicted_corridor =
                self.transfer_corridor_state(ctx, predicted, predicted_diagnostics);
            if predicted_corridor.margin_m < 0.0 {
                let corridor_error_ratio = (-predicted_corridor.margin_m / 100.0).min(8.0);
                score += 80.0 * corridor_error_ratio * corridor_error_ratio;
            }
            if predicted_corridor.tilt_limited {
                score += 250.0;
            }
        }
        score
    }

    fn score_boost_candidate_effort(&self, command: Command) -> f64 {
        let mut score = 0.0;
        score += TRANSFER_BOOST_SCORE_THROTTLE_EFFORT
            * command.throttle_frac.clamp(0.0, 1.0)
            * command.throttle_frac.clamp(0.0, 1.0);
        let tilt_ratio = (command.target_attitude_rad.abs()
            / self
                .config
                .boost_tilt_rad
                .max(self.config.uphill_boost_tilt_rad)
                .max(1.0e-6))
        .min(4.0);
        score += TRANSFER_BOOST_SCORE_TILT_EFFORT * tilt_ratio * tilt_ratio;
        score
    }

    fn score_boost_candidate_pathwise(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        corridor: TransferCorridorState,
        command: Command,
    ) -> TransferBoostCandidateScore {
        let samples = self.simulate_transfer_command_samples(
            ctx,
            observation,
            command,
            self.config.boost_candidate_horizon_s,
            self.config.boost_candidate_step_s,
        );
        let final_state = samples
            .last()
            .copied()
            .unwrap_or_else(|| self.initial_transfer_sim_state(observation));
        let predicted = self.observation_from_sim_state(ctx, observation, final_state);
        let predicted_diagnostics = self.transfer_diagnostics(&predicted);
        let projection = predicted_diagnostics.projection;
        let quality = predicted_diagnostics.boost_quality;
        let dx_limit_m = self.boost_dx_limit_m(observation);

        let mut path_score = 0.0;
        let mut weight_sum = 0.0;
        for (index, state) in samples.iter().enumerate() {
            let sample_observation = self.observation_from_sim_state(ctx, observation, *state);
            let sample_diagnostics = self.transfer_diagnostics(&sample_observation);
            let weight = (index + 1) as f64;
            path_score += weight
                * (self.score_boost_candidate_geometry(
                    observation,
                    &sample_observation,
                    sample_diagnostics,
                    dx_limit_m,
                ) + self.score_boost_candidate_corridor(
                    ctx,
                    &sample_observation,
                    sample_diagnostics,
                    corridor,
                ) + self.score_boost_no_away_penalty(
                    &sample_observation,
                    sample_diagnostics,
                    command,
                    dx_limit_m,
                ));
            weight_sum += weight;
        }
        if weight_sum > 0.0 {
            path_score /= weight_sum;
        }

        let endpoint_score = self.score_boost_candidate_endpoint_terms(
            ctx,
            observation,
            &predicted,
            predicted_diagnostics,
            corridor,
            command,
        );
        let score = endpoint_score
            + (0.25 * path_score)
            + self.score_boost_no_away_penalty(observation, diagnostics, command, dx_limit_m);

        TransferBoostCandidateScore {
            score,
            projection,
            quality,
        }
    }

    fn score_boost_no_away_penalty(
        &self,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        command: Command,
        dx_limit_m: f64,
    ) -> f64 {
        let target_dx_m = observation.target_dx_m;
        if target_dx_m.abs() <= dx_limit_m || command.throttle_frac <= 0.0 {
            return 0.0;
        }

        let target_sign = target_dx_m.signum();
        let thrust_lateral_sign = command.target_attitude_rad.sin().signum();
        if thrust_lateral_sign == 0.0 || thrust_lateral_sign * target_sign >= 0.0 {
            return 0.0;
        }

        let projected_overshoot = diagnostics
            .projection
            .projected_dx_m
            .is_some_and(|projected_dx_m| projected_dx_m * target_sign < -dx_limit_m);
        if projected_overshoot {
            return 0.0;
        }

        let away_ratio = (target_dx_m.abs() / dx_limit_m).min(8.0);
        60.0 * away_ratio * away_ratio * command.throttle_frac.clamp(0.0, 1.0)
    }

    fn waypoint_approach_state(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        geometry: &WaypointLegGeometry<'_>,
        stats: WaypointLegStats,
    ) -> WaypointApproachState {
        waypoint_approach_state(
            ctx,
            observation,
            geometry,
            stats,
            self.config
                .boost_tilt_rad
                .max(self.config.uphill_boost_tilt_rad),
        )
    }

    fn score_boost_candidate_recoverability(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        _diagnostics: TransferDiagnostics,
        corridor: TransferCorridorState,
        command: Command,
    ) -> TransferBoostCandidateScore {
        let simulated = self.simulate_transfer_command(
            ctx,
            observation,
            command,
            self.config.boost_candidate_horizon_s,
            self.config.boost_candidate_step_s,
        );
        let predicted = self.observation_from_sim_state(ctx, observation, simulated);
        let predicted_diagnostics = self.transfer_diagnostics(&predicted);
        let projection = predicted_diagnostics.projection;
        let quality = predicted_diagnostics.boost_quality;
        let predicted_gate = self.transfer_gate_readiness_without_deferral(
            ctx,
            &predicted,
            predicted_diagnostics,
            self.transfer_gate_ready_ticks,
        );

        let settled_simulated = self.simulate_transfer_command(
            ctx,
            &predicted,
            Command {
                throttle_frac: 0.0,
                target_attitude_rad: self.coast_attitude_rad(&predicted),
            },
            self.config.boost_settle_lookahead_s,
            self.config.boost_candidate_step_s,
        );
        let settled = self.observation_from_sim_state(ctx, &predicted, settled_simulated);
        let settled_diagnostics = self.transfer_diagnostics(&settled);
        let settled_gate = self.transfer_gate_readiness_without_deferral(
            ctx,
            &settled,
            settled_diagnostics,
            predicted_gate.ready_ticks,
        );

        let endpoint_score = self.score_boost_candidate_endpoint_terms(
            ctx,
            observation,
            &predicted,
            predicted_diagnostics,
            corridor,
            command,
        );
        let recovery_score = self.score_boost_candidate_recoverability_terms(
            &predicted,
            predicted_diagnostics,
            predicted_gate,
            self.boost_dx_limit_m(observation),
        );
        let settled_recovery_score = self.score_boost_candidate_recoverability_terms(
            &settled,
            settled_diagnostics,
            settled_gate,
            self.boost_dx_limit_m(observation),
        );
        let score = endpoint_score
            + (TRANSFER_BOOST_RECOVERY_SCORE_ENDPOINT_WEIGHT * recovery_score)
            + (TRANSFER_BOOST_RECOVERY_SCORE_SETTLED_WEIGHT * settled_recovery_score);

        TransferBoostCandidateScore {
            score,
            projection,
            quality,
        }
    }

    fn score_boost_candidate_recoverability_terms(
        &self,
        predicted: &Observation,
        predicted_diagnostics: TransferDiagnostics,
        gate: TerminalEntryAssessment,
        dx_limit_m: f64,
    ) -> f64 {
        let mut score = 0.0;
        if !gate.terrain_clearance_safe {
            score += TRANSFER_BOOST_RECOVERY_SCORE_TERRAIN_UNSAFE;
            score += (-gate.terrain_min_clearance_m).clamp(0.0, 200.0);
        }

        if predicted.height_above_target_m <= 0.0 {
            score += 600.0 + (-predicted.height_above_target_m).min(200.0);
        }

        let negative_margin_s = (-gate.latest_safe_margin_s).clamp(0.0, 12.0);
        score += TRANSFER_BOOST_RECOVERY_SCORE_LATEST_SAFE_MARGIN
            * negative_margin_s
            * negative_margin_s;

        let accel_excess_ratio = (gate.required_accel_ratio - 1.0).clamp(0.0, 12.0);
        score +=
            TRANSFER_BOOST_RECOVERY_SCORE_ACCEL_RATIO * accel_excess_ratio * accel_excess_ratio;

        if predicted_diagnostics.boost_quality.passed
            && gate.mode != TerminalEntryMode::NominalReady
        {
            let projected_dx_ratio = predicted_diagnostics
                .projection
                .projected_dx_m
                .map(|projected_dx_m| projected_dx_m.abs() / dx_limit_m.max(1.0))
                .unwrap_or(2.0)
                .min(8.0);
            score += TRANSFER_BOOST_RECOVERY_SCORE_PASS_NOT_READY * projected_dx_ratio;
        }

        score
    }

    fn boost_settled_quality(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        _diagnostics: TransferDiagnostics,
    ) -> TransferBoostCandidateScore {
        let simulated = self.simulate_transfer_command(
            ctx,
            observation,
            Command {
                throttle_frac: 0.0,
                target_attitude_rad: self.coast_attitude_rad(observation),
            },
            self.config.boost_settle_lookahead_s,
            self.config.boost_candidate_step_s,
        );
        let predicted = self.observation_from_sim_state(ctx, observation, simulated);
        let predicted_diagnostics = self.transfer_diagnostics(&predicted);
        TransferBoostCandidateScore {
            score: 0.0,
            projection: predicted_diagnostics.projection,
            quality: predicted_diagnostics.boost_quality,
        }
    }

    fn apply_corridor_tilt_cap(
        &self,
        target_attitude_rad: f64,
        corridor: TransferCorridorState,
    ) -> f64 {
        if corridor.tilt_limited {
            target_attitude_rad.clamp(
                -TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD,
                TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD,
            )
        } else {
            target_attitude_rad
        }
    }

    fn push_unique_candidate(&self, candidates: &mut Vec<f64>, value: f64) {
        if !value.is_finite() {
            return;
        }
        if candidates
            .iter()
            .any(|candidate| (candidate - value).abs() <= 1.0e-6)
        {
            return;
        }
        candidates.push(value);
    }

    fn coast_attitude_rad(&self, observation: &Observation) -> f64 {
        let tilt_limit_rad = self
            .config
            .terminal
            .terminal_overshoot_tilt_max_rad
            .max(self.config.terminal.terminal_dynamic_tilt_max_rad);
        let upright_retrograde_y = observation.velocity_mps.y.abs().max(1.0);
        (-observation.velocity_mps.x)
            .atan2(upright_retrograde_y)
            .clamp(-tilt_limit_rad, tilt_limit_rad)
    }

    fn passive_coast_observation(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        duration_s: f64,
    ) -> Observation {
        let simulated = self.simulate_transfer_command(
            ctx,
            observation,
            Command {
                throttle_frac: 0.0,
                target_attitude_rad: observation.attitude_rad,
            },
            duration_s,
            self.config.transfer_gate_defer_step_s,
        );
        self.observation_from_sim_state(ctx, observation, simulated)
    }

    fn simulate_transfer_command(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        command: Command,
        duration_s: f64,
        step_s: f64,
    ) -> TransferSimState {
        self.simulate_transfer_command_samples(ctx, observation, command, duration_s, step_s)
            .last()
            .copied()
            .unwrap_or_else(|| self.initial_transfer_sim_state(observation))
    }

    fn simulate_transfer_command_samples(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        command: Command,
        duration_s: f64,
        step_s: f64,
    ) -> Vec<TransferSimState> {
        let mut state = self.initial_transfer_sim_state(observation);
        let mut samples = Vec::new();
        let duration_s = duration_s.max(0.0);
        let step_s = step_s.clamp(1.0e-3, duration_s.max(1.0e-3));
        let mut elapsed_s = 0.0;
        while elapsed_s + 1.0e-9 < duration_s {
            let dt_s = (duration_s - elapsed_s).min(step_s);
            let max_delta = ctx.vehicle.max_rotation_rate_radps.max(0.0) * dt_s;
            let delta = shortest_angle_delta(state.attitude_rad, command.target_attitude_rad);
            let applied_delta = delta.clamp(-max_delta, max_delta);
            state.attitude_rad += applied_delta;

            let throttle_frac = applied_throttle_frac(ctx, command.throttle_frac, state.fuel_kg);
            let fuel_used_kg =
                (ctx.vehicle.max_fuel_burn_kgps.max(0.0) * throttle_frac * dt_s).min(state.fuel_kg);
            state.fuel_kg -= fuel_used_kg;

            let thrust_n = ctx.vehicle.max_thrust_n.max(0.0) * throttle_frac;
            let mass_kg = state.mass_kg();
            let (sin_a, cos_a) = state.attitude_rad.sin_cos();
            let thrust_accel_mps2 =
                Vec2::new((thrust_n / mass_kg) * sin_a, (thrust_n / mass_kg) * cos_a);
            let total_accel_mps2 = Vec2::new(
                thrust_accel_mps2.x,
                thrust_accel_mps2.y - observation.gravity_mps2,
            );
            state.velocity_mps += total_accel_mps2 * dt_s;
            state.position_m += state.velocity_mps * dt_s;
            elapsed_s += dt_s;
            samples.push(state);
        }
        samples
    }

    fn initial_transfer_sim_state(&self, observation: &Observation) -> TransferSimState {
        TransferSimState {
            position_m: observation.position_m,
            velocity_mps: observation.velocity_mps,
            attitude_rad: observation.attitude_rad,
            fuel_kg: observation.fuel_kg.max(0.0),
            dry_mass_kg: (observation.mass_kg - observation.fuel_kg.max(0.0)).max(0.0),
        }
    }

    fn observation_from_sim_state(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        state: TransferSimState,
    ) -> Observation {
        let target_x_m = observation.position_m.x + observation.target_dx_m;
        let target_y_m = observation.target_surface_y_m;
        let terrain_y_m = ctx.world.terrain.sample_height(state.position_m.x);
        let clearance_m =
            state.position_m.y - terrain_y_m - ctx.vehicle.geometry.touchdown_base_offset_m;
        Observation {
            sim_time_s: observation.sim_time_s,
            physics_step: observation.physics_step,
            position_m: state.position_m,
            velocity_mps: state.velocity_mps,
            attitude_rad: state.attitude_rad,
            angular_rate_radps: 0.0,
            mass_kg: state.mass_kg(),
            fuel_kg: state.fuel_kg,
            gravity_mps2: observation.gravity_mps2,
            target_dx_m: target_x_m - state.position_m.x,
            height_above_target_m: state.position_m.y - target_y_m,
            target_surface_y_m: target_y_m,
            target_pad_half_width_m: observation.target_pad_half_width_m,
            touchdown_clearance_m: clearance_m,
            min_hull_clearance_m: clearance_m,
        }
    }

    // Frame assembly intentionally receives the already-computed guidance
    // products so telemetry cannot silently recompute divergent values.
    #[allow(clippy::too_many_arguments)]
    fn frame_for_open_loop_phase(
        &mut self,
        ctx: &RunContext,
        observation: &Observation,
        phase_name: TransferPhase,
        command: Command,
        status: &'static str,
        diagnostics: TransferDiagnostics,
        gate: TerminalEntryAssessment,
        corridor: TransferCorridorState,
        boost_selection: Option<TransferBoostCommandSelection>,
    ) -> ControllerFrame {
        let view = ControllerView::new(ctx, observation);
        let phase = phase_name.as_str().to_owned();
        let builder = ControllerFrameBuilder::new(command)
            .status(status)
            .phase(phase.clone())
            .standard_kinematics(&view)
            .phase_transition_marker(self.last_phase.as_deref(), &phase, &view)
            .metric(metric::GUIDANCE_ACTIVE, true)
            .metric(metric::TRANSFER_PHASE, phase.as_str());
        let frame = self
            .transfer_metrics_builder(builder, diagnostics, gate, corridor, boost_selection)
            .build();
        self.last_phase = Some(phase);
        frame
    }
}

impl WaypointTelemetry {
    fn from_capture(capture: WaypointCaptureSnapshot) -> Self {
        Self {
            active_index: capture.index as i64,
            active_leg_index: capture.index as i64,
            capture_status: capture.status,
            capture_time_s: Some(capture.capture_time_s),
            closest_distance_m: capture.closest_distance_m,
            distance_m: capture.distance_m,
            cross_track_m: capture.cross_track_m,
            plane_progress_m: capture.plane_progress_m,
            outbound_heading_error_rad: capture.outbound_heading_error_rad,
            outbound_progress_mps: capture.outbound_progress_mps,
            outbound_cross_speed_mps: capture.outbound_cross_speed_mps,
            speed_mps: capture.speed_mps,
            vertical_speed_mps: capture.vertical_speed_mps,
            remaining_to_plane_m: capture.approach.remaining_to_plane_m,
            time_to_plane_s: capture.approach.time_to_plane_s,
            required_turn_distance_m: capture.approach.required_turn_distance_m,
            shaping_start_distance_m: capture.approach.shaping_start_distance_m,
            turn_margin_m: capture.approach.turn_margin_m,
            center_m: capture.center_m,
            nominal_handoff_target_m: capture.nominal_handoff_target_m,
            handoff_target_m: capture.handoff_target_m,
            handoff_target_mode: capture.handoff_target_mode,
            remaining_to_handoff_m: capture.approach.remaining_to_handoff_m,
            time_to_handoff_s: capture.approach.time_to_handoff_s,
            handoff_turn_margin_m: capture.approach.handoff_turn_margin_m,
            endpoint_m: capture.endpoint_m,
            steering_target_m: capture.steering_target_m,
            target_state: capture.target_state,
        }
    }
}

fn insert_waypoint_target_state_metrics(
    metrics: &mut BTreeMap<String, TelemetryValue>,
    target_state: WaypointGuidanceTargetState,
) {
    metrics.insert(
        metric::WAYPOINT_TARGET_VX_MPS.to_owned(),
        TelemetryValue::from(target_state.target_velocity_mps.x),
    );
    metrics.insert(
        metric::WAYPOINT_TARGET_VY_MPS.to_owned(),
        TelemetryValue::from(target_state.target_velocity_mps.y),
    );
    metrics.insert(
        metric::WAYPOINT_TARGET_SPEED_MPS.to_owned(),
        TelemetryValue::from(target_state.target_velocity_mps.length()),
    );
    metrics.insert(
        metric::WAYPOINT_TARGET_DEADLINE_REMAINING_S.to_owned(),
        TelemetryValue::from(target_state.deadline_remaining_s),
    );
    metrics.insert(
        metric::WAYPOINT_TARGET_VELOCITY_ERROR_MPS.to_owned(),
        TelemetryValue::from(target_state.velocity_error_mps),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_FEASIBLE.to_owned(),
        TelemetryValue::from(target_state.feasible),
    );
    if let Some(required_accel_ratio) = target_state.final_terminal_required_accel_ratio {
        metrics.insert(
            metric::WAYPOINT_FINAL_TERMINAL_REQUIRED_ACCEL_RATIO.to_owned(),
            TelemetryValue::from(required_accel_ratio),
        );
    }
    if let Some(recoverable) = target_state.final_terminal_recoverable {
        metrics.insert(
            metric::WAYPOINT_FINAL_TERMINAL_RECOVERABLE.to_owned(),
            TelemetryValue::from(recoverable),
        );
    }
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_PLAN_INDEX.to_owned(),
        TelemetryValue::from(target_state.trackability.plan_index as i64),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_PLAN_REVISION.to_owned(),
        TelemetryValue::from(target_state.trackability.plan_revision as i64),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_PLAN_REASON.to_owned(),
        TelemetryValue::from(target_state.trackability.plan_reason.as_str()),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_PLAN_AGE_S.to_owned(),
        TelemetryValue::from(target_state.trackability.plan_age_s),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_REFERENCE_POSITION_ERROR_M.to_owned(),
        TelemetryValue::from(target_state.trackability.reference_position_error_m),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_ERROR_M.to_owned(),
        TelemetryValue::from(target_state.trackability.reference_cross_error_m),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_REFERENCE_VELOCITY_ERROR_MPS.to_owned(),
        TelemetryValue::from(target_state.trackability.reference_velocity_error_mps),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_REFERENCE_CROSS_SPEED_ERROR_MPS.to_owned(),
        TelemetryValue::from(target_state.trackability.reference_cross_speed_error_mps),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_AUTHORITY_MARGIN.to_owned(),
        TelemetryValue::from(target_state.authority_margin),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_THRUST_SATURATED.to_owned(),
        TelemetryValue::from(target_state.thrust_saturated),
    );
    metrics.insert(
        metric::WAYPOINT_GUIDANCE_TILT_SATURATED.to_owned(),
        TelemetryValue::from(target_state.tilt_saturated),
    );
    let reachable = target_state.reachable_prediction;
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_MODEL.to_owned(),
        TelemetryValue::from("actuated_rollout"),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_TIME_TO_GO_S.to_owned(),
        TelemetryValue::from(reachable.prediction.time_to_event_s),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_TRIGGERED.to_owned(),
        TelemetryValue::from(reachable.prediction.assessment.triggered),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_CONTRACT_PASS.to_owned(),
        TelemetryValue::from(reachable.prediction.assessment.contract_pass()),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_CONTRACT_REASONS.to_owned(),
        TelemetryValue::from(reachable.prediction.assessment.reasons()),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
        TelemetryValue::from(reachable.prediction.stats.outbound_heading_error_rad),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
        TelemetryValue::from(reachable.prediction.stats.outbound_cross_speed_mps),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_REQUIRED_ACCEL_RATIO_MAX.to_owned(),
        TelemetryValue::from(reachable.required_accel_ratio_max),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_THRUST_SATURATED_TIME_S.to_owned(),
        TelemetryValue::from(reachable.thrust_saturated_time_s),
    );
    metrics.insert(
        metric::WAYPOINT_REACHABLE_HANDOFF_TILT_SATURATED_TIME_S.to_owned(),
        TelemetryValue::from(reachable.tilt_saturated_time_s),
    );
    if let Some(continuation) = target_state.continuation_prediction {
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_NEXT_INDEX.to_owned(),
            TelemetryValue::from(continuation.next_waypoint_index as i64),
        );
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_CONTRACT_PASS.to_owned(),
            TelemetryValue::from(
                continuation
                    .prediction
                    .prediction
                    .assessment
                    .contract_pass(),
            ),
        );
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_CONTRACT_REASONS.to_owned(),
            TelemetryValue::from(continuation.prediction.prediction.assessment.reasons()),
        );
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
            TelemetryValue::from(
                continuation
                    .prediction
                    .prediction
                    .stats
                    .outbound_heading_error_rad,
            ),
        );
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_REQUIRED_ACCEL_RATIO_MAX.to_owned(),
            TelemetryValue::from(continuation.prediction.required_accel_ratio_max),
        );
        metrics.insert(
            metric::WAYPOINT_CONTINUATION_PASSING_CANDIDATE_COUNT.to_owned(),
            TelemetryValue::from(continuation.passing_candidate_count as i64),
        );
    }
    if let Some(joint) = target_state.joint_prediction {
        insert_waypoint_joint_prediction_metrics(metrics, joint);
    }
    let prediction = target_state.prediction;
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_TIME_TO_GO_S.to_owned(),
        TelemetryValue::from(prediction.time_to_event_s),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_DEADLINE_LEAD_S.to_owned(),
        TelemetryValue::from(prediction.deadline_lead_s),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_TRIGGERED.to_owned(),
        TelemetryValue::from(prediction.assessment.triggered),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_SPATIAL_PASS.to_owned(),
        TelemetryValue::from(prediction.assessment.spatial_pass),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_ENVELOPE_PASS.to_owned(),
        TelemetryValue::from(prediction.assessment.envelope_pass()),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_PASS.to_owned(),
        TelemetryValue::from(prediction.assessment.contract_pass()),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_CONTRACT_REASONS.to_owned(),
        TelemetryValue::from(prediction.assessment.reasons()),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_DISTANCE_M.to_owned(),
        TelemetryValue::from(prediction.stats.distance_m),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_CROSS_TRACK_M.to_owned(),
        TelemetryValue::from(prediction.stats.cross_track_m),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_PLANE_PROGRESS_M.to_owned(),
        TelemetryValue::from(prediction.stats.plane_progress_m),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
        TelemetryValue::from(prediction.stats.outbound_heading_error_rad),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_PROGRESS_MPS.to_owned(),
        TelemetryValue::from(prediction.stats.outbound_progress_mps),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
        TelemetryValue::from(prediction.stats.outbound_cross_speed_mps),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_SPEED_MPS.to_owned(),
        TelemetryValue::from(prediction.stats.speed_mps),
    );
    metrics.insert(
        metric::WAYPOINT_PREDICTED_HANDOFF_VERTICAL_SPEED_MPS.to_owned(),
        TelemetryValue::from(prediction.stats.vertical_speed_mps),
    );
}

fn insert_waypoint_joint_prediction_metrics(
    metrics: &mut BTreeMap<String, TelemetryValue>,
    joint: WaypointJointSearchPrediction,
) {
    metrics.insert(
        metric::WAYPOINT_JOINT_NEXT_INDEX.to_owned(),
        TelemetryValue::from(joint.next_waypoint_index as i64),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_EVALUATED_CANDIDATE_COUNT.to_owned(),
        TelemetryValue::from(joint.evaluated_candidate_count as i64),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_PASSING_CANDIDATE_COUNT.to_owned(),
        TelemetryValue::from(joint.passing_candidate_count as i64),
    );
    let Some(selected) = joint.selected else {
        return;
    };
    metrics.insert(
        metric::WAYPOINT_JOINT_CONTRACT_PASS.to_owned(),
        TelemetryValue::from(selected.contract_pass()),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_ENDPOINT_X_M.to_owned(),
        TelemetryValue::from(selected.current.endpoint_m.x),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_ENDPOINT_Y_M.to_owned(),
        TelemetryValue::from(selected.current.endpoint_m.y),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_TARGET_VX_MPS.to_owned(),
        TelemetryValue::from(selected.current.candidate.target_velocity_mps.x),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_TARGET_VY_MPS.to_owned(),
        TelemetryValue::from(selected.current.candidate.target_velocity_mps.y),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_TIME_TO_GO_S.to_owned(),
        TelemetryValue::from(selected.current.candidate.time_to_go_s),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
        TelemetryValue::from(
            selected
                .continuation
                .prediction
                .stats
                .outbound_heading_error_rad,
        ),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_REQUIRED_ACCEL_RATIO_MAX.to_owned(),
        TelemetryValue::from(selected.required_accel_ratio_max()),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_TOTAL_SATURATED_TIME_S.to_owned(),
        TelemetryValue::from(selected.total_saturated_time_s()),
    );
    metrics.insert(
        metric::WAYPOINT_JOINT_CONTINUATION_PASSING_CANDIDATE_COUNT.to_owned(),
        TelemetryValue::from(selected.continuation_passing_candidate_count as i64),
    );
}

fn insert_waypoint_transition_audit_metrics(
    metrics: &mut BTreeMap<String, TelemetryValue>,
    audit: WaypointTransitionAudit,
) {
    metrics.insert(
        metric::WAYPOINT_TRANSITION_NEXT_INDEX.to_owned(),
        TelemetryValue::from(audit.next_waypoint_index as i64),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_POSITION_ERROR_M.to_owned(),
        TelemetryValue::from(audit.position_error_m),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_VELOCITY_ERROR_MPS.to_owned(),
        TelemetryValue::from(audit.velocity_error_mps),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_ATTITUDE_ERROR_RAD.to_owned(),
        TelemetryValue::from(audit.attitude_error_rad),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_MASS_ERROR_KG.to_owned(),
        TelemetryValue::from(audit.mass_error_kg),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_FUEL_ERROR_KG.to_owned(),
        TelemetryValue::from(audit.fuel_error_kg),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_EVENT_TIME_ERROR_S.to_owned(),
        TelemetryValue::from(audit.event_time_error_s),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_CONTINUATION_CONTRACT_PASS.to_owned(),
        TelemetryValue::from(
            audit
                .continuation_prediction
                .prediction
                .assessment
                .contract_pass(),
        ),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_CONTINUATION_CONTRACT_REASONS.to_owned(),
        TelemetryValue::from(
            audit
                .continuation_prediction
                .prediction
                .assessment
                .reasons(),
        ),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_CONTINUATION_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
        TelemetryValue::from(
            audit
                .continuation_prediction
                .prediction
                .stats
                .outbound_heading_error_rad,
        ),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_CONTINUATION_REQUIRED_ACCEL_RATIO_MAX.to_owned(),
        TelemetryValue::from(audit.continuation_prediction.required_accel_ratio_max),
    );
    metrics.insert(
        metric::WAYPOINT_TRANSITION_CONTINUATION_PASSING_CANDIDATE_COUNT.to_owned(),
        TelemetryValue::from(audit.passing_candidate_count as i64),
    );
}

fn transfer_ballistic_projection(
    dx_m: f64,
    dy_m: f64,
    vx_mps: f64,
    vy_up_mps: f64,
    gravity_mps2: f64,
) -> TransferBallisticProjection {
    let gravity_mps2 = gravity_mps2.max(1.0e-6);
    let discriminant = (vy_up_mps * vy_up_mps) - (2.0 * gravity_mps2 * dy_m);
    let apex_over_target_m = if vy_up_mps > 0.0 {
        -dy_m + ((vy_up_mps * vy_up_mps) / (2.0 * gravity_mps2))
    } else {
        -dy_m
    };

    if discriminant < 0.0 {
        return TransferBallisticProjection {
            has_target_y_solution: false,
            projected_time_s: None,
            projected_dx_m: None,
            impact_angle_deg: None,
            apex_over_target_m,
        };
    }

    let sqrt_discriminant = discriminant.sqrt();
    let t0 = (vy_up_mps - sqrt_discriminant) / gravity_mps2;
    let t1 = (vy_up_mps + sqrt_discriminant) / gravity_mps2;
    let projected_time_s = [t0, t1]
        .into_iter()
        .filter(|time_s| *time_s >= 0.0)
        .max_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap())
        .unwrap_or(0.0);
    let projected_dx_m = dx_m - (vx_mps * projected_time_s);
    let impact_vy_up_mps = vy_up_mps - (gravity_mps2 * projected_time_s);
    let impact_down_speed_mps = (-impact_vy_up_mps).max(0.0);
    let impact_angle_deg = impact_down_speed_mps.atan2(vx_mps.abs()).to_degrees();

    TransferBallisticProjection {
        has_target_y_solution: true,
        projected_time_s: Some(projected_time_s),
        projected_dx_m: Some(projected_dx_m),
        impact_angle_deg: Some(impact_angle_deg),
        apex_over_target_m,
    }
}

fn applied_throttle_frac(ctx: &RunContext, commanded_throttle_frac: f64, fuel_kg: f64) -> f64 {
    if fuel_kg <= 0.0 {
        return 0.0;
    }
    let commanded = commanded_throttle_frac.clamp(0.0, 1.0);
    if commanded <= 0.0 {
        return 0.0;
    }
    let min_throttle = ctx.vehicle.min_throttle_frac.clamp(0.0, 1.0);
    min_throttle + (commanded * (1.0 - min_throttle))
}

fn shortest_angle_delta(from_rad: f64, to_rad: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    (to_rad - from_rad + std::f64::consts::PI).rem_euclid(tau) - std::f64::consts::PI
}

impl Controller for TransferPdgController {
    fn id(&self) -> &str {
        if self.guidance_mode.uses_waypoints() {
            "transfer_waypoint_pdg_v1"
        } else {
            match self.boost_scoring_mode {
                TransferBoostScoringMode::Endpoint => "transfer_pdg_v1",
                TransferBoostScoringMode::ExperimentalPathwise => "transfer_pdg_pathwise_v1",
                TransferBoostScoringMode::ExperimentalRecoverability => {
                    "transfer_pdg_recoverability_v1"
                }
            }
        }
    }

    fn reset(&mut self, ctx: &RunContext) {
        self.phase = TransferPhase::Takeoff;
        self.boost_anchor = None;
        self.transfer_gate_ready_ticks = 0;
        self.last_transfer_gate = None;
        self.last_corridor = TransferCorridorState::inactive();
        self.last_phase = None;
        self.waypoint_active_index = 0;
        self.waypoint_closest_distance_m = f64::INFINITY;
        self.last_waypoint_capture = None;
        self.waypoint_window_entry = None;
        self.waypoint_guidance_plan = None;
        self.waypoint_guidance_replan_count = 0;
        self.waypoint_guidance_contract_failure_ticks = 0;
        self.waypoint_reachable_search_attempted_revision = None;
        self.waypoint_reference_contract_pass_ever = false;
        self.waypoint_continuation_snapshot = None;
        self.waypoint_joint_snapshot = None;
        self.terminal.reset(ctx);
    }

    fn update(&mut self, ctx: &RunContext, observation: &Observation) -> ControllerFrame {
        if let Some(waypoint_context) = self.waypoint_update_context(ctx, observation) {
            let WaypointUpdateContext {
                observation,
                allow_terminal,
                telemetry,
                guidance,
                capture,
            } = waypoint_context;
            let mut frame = if let Some(guidance) = guidance {
                self.update_active_waypoint_frame(ctx, &observation, guidance, telemetry)
            } else {
                self.update_transfer_frame(ctx, &observation, allow_terminal, Some(telemetry))
            };
            if let Some(capture) = capture {
                frame
                    .markers
                    .push(waypoint_handoff_marker(ctx, &observation, capture));
            }
            frame
        } else {
            self.update_transfer_frame(ctx, observation, true, None)
        }
    }
}

fn waypoint_handoff_marker(
    ctx: &RunContext,
    observation: &Observation,
    capture: WaypointCaptureSnapshot,
) -> crate::ControllerMarker {
    let waypoint_id = ctx
        .mission
        .transfer_route
        .as_ref()
        .and_then(|route| route.waypoints.get(capture.index))
        .map_or("unknown", |waypoint| waypoint.id.as_str());
    let view = ControllerView::new(ctx, observation);
    let mut metadata = BTreeMap::from([
        ("kind".to_owned(), TelemetryValue::from("waypoint_handoff")),
        ("waypoint.id".to_owned(), TelemetryValue::from(waypoint_id)),
        (
            "waypoint.index".to_owned(),
            TelemetryValue::from(capture.index as i64),
        ),
        (
            metric::WAYPOINT_CAPTURE_STATUS.to_owned(),
            TelemetryValue::from(capture.status),
        ),
        (
            metric::WAYPOINT_CAPTURE_TIME_S.to_owned(),
            TelemetryValue::from(capture.capture_time_s),
        ),
        (
            "waypoint.position_x_m".to_owned(),
            TelemetryValue::from(observation.position_m.x),
        ),
        (
            "waypoint.position_y_m".to_owned(),
            TelemetryValue::from(observation.position_m.y),
        ),
        (
            "waypoint.velocity_x_mps".to_owned(),
            TelemetryValue::from(observation.velocity_mps.x),
        ),
        (
            "waypoint.velocity_y_mps".to_owned(),
            TelemetryValue::from(observation.velocity_mps.y),
        ),
        (
            metric::WAYPOINT_DISTANCE_M.to_owned(),
            TelemetryValue::from(capture.distance_m),
        ),
        (
            metric::WAYPOINT_CROSS_TRACK_M.to_owned(),
            TelemetryValue::from(capture.cross_track_m),
        ),
        (
            metric::WAYPOINT_PLANE_PROGRESS_M.to_owned(),
            TelemetryValue::from(capture.plane_progress_m),
        ),
        (
            metric::WAYPOINT_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
            TelemetryValue::from(capture.outbound_heading_error_rad),
        ),
        (
            metric::WAYPOINT_OUTBOUND_PROGRESS_MPS.to_owned(),
            TelemetryValue::from(capture.outbound_progress_mps),
        ),
        (
            metric::WAYPOINT_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
            TelemetryValue::from(capture.outbound_cross_speed_mps),
        ),
        (
            metric::WAYPOINT_SPEED_MPS.to_owned(),
            TelemetryValue::from(capture.speed_mps),
        ),
        (
            metric::WAYPOINT_VERTICAL_SPEED_MPS.to_owned(),
            TelemetryValue::from(capture.vertical_speed_mps),
        ),
        (
            metric::WAYPOINT_TURN_MARGIN_M.to_owned(),
            TelemetryValue::from(capture.approach.turn_margin_m),
        ),
        (
            metric::WAYPOINT_CENTER_X_M.to_owned(),
            TelemetryValue::from(capture.center_m.x),
        ),
        (
            metric::WAYPOINT_CENTER_Y_M.to_owned(),
            TelemetryValue::from(capture.center_m.y),
        ),
        (
            metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_X_M.to_owned(),
            TelemetryValue::from(capture.nominal_handoff_target_m.x),
        ),
        (
            metric::WAYPOINT_NOMINAL_HANDOFF_TARGET_Y_M.to_owned(),
            TelemetryValue::from(capture.nominal_handoff_target_m.y),
        ),
        (
            metric::WAYPOINT_HANDOFF_TARGET_X_M.to_owned(),
            TelemetryValue::from(capture.handoff_target_m.x),
        ),
        (
            metric::WAYPOINT_HANDOFF_TARGET_Y_M.to_owned(),
            TelemetryValue::from(capture.handoff_target_m.y),
        ),
        (
            metric::WAYPOINT_HANDOFF_TARGET_MODE.to_owned(),
            TelemetryValue::from(capture.handoff_target_mode),
        ),
        (
            metric::WAYPOINT_REMAINING_TO_HANDOFF_M.to_owned(),
            TelemetryValue::from(capture.approach.remaining_to_handoff_m),
        ),
        (
            metric::WAYPOINT_TIME_TO_HANDOFF_S.to_owned(),
            TelemetryValue::from(capture.approach.time_to_handoff_s),
        ),
        (
            metric::WAYPOINT_HANDOFF_TURN_MARGIN_M.to_owned(),
            TelemetryValue::from(capture.approach.handoff_turn_margin_m),
        ),
        (
            metric::WAYPOINT_GUIDANCE_REPLAN_COUNT.to_owned(),
            TelemetryValue::from(capture.guidance_replan_count as i64),
        ),
        (
            metric::WAYPOINT_HANDOFF_RESOLUTION_REASON.to_owned(),
            TelemetryValue::from(capture.resolution_reason),
        ),
    ]);
    if let Some(entry) = capture.window_entry {
        metadata.extend([
            (
                metric::WAYPOINT_WINDOW_ENTRY_TIME_S.to_owned(),
                TelemetryValue::from(entry.time_s),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_POSITION_X_M.to_owned(),
                TelemetryValue::from(entry.position_m.x),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_POSITION_Y_M.to_owned(),
                TelemetryValue::from(entry.position_m.y),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_VELOCITY_X_MPS.to_owned(),
                TelemetryValue::from(entry.velocity_mps.x),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_VELOCITY_Y_MPS.to_owned(),
                TelemetryValue::from(entry.velocity_mps.y),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_DISTANCE_M.to_owned(),
                TelemetryValue::from(entry.stats.distance_m),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_CROSS_TRACK_M.to_owned(),
                TelemetryValue::from(entry.stats.cross_track_m),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_PLANE_PROGRESS_M.to_owned(),
                TelemetryValue::from(entry.stats.plane_progress_m),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_HEADING_ERROR_RAD.to_owned(),
                TelemetryValue::from(entry.stats.outbound_heading_error_rad),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_PROGRESS_MPS.to_owned(),
                TelemetryValue::from(entry.stats.outbound_progress_mps),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_OUTBOUND_CROSS_SPEED_MPS.to_owned(),
                TelemetryValue::from(entry.stats.outbound_cross_speed_mps),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_SPEED_MPS.to_owned(),
                TelemetryValue::from(entry.stats.speed_mps),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_VERTICAL_SPEED_MPS.to_owned(),
                TelemetryValue::from(entry.stats.vertical_speed_mps),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_PASS.to_owned(),
                TelemetryValue::from(entry.assessment.contract_pass()),
            ),
            (
                metric::WAYPOINT_WINDOW_ENTRY_CONTRACT_REASONS.to_owned(),
                TelemetryValue::from(entry.assessment.reasons()),
            ),
            (
                metric::WAYPOINT_HANDOFF_WINDOW_DURATION_S.to_owned(),
                TelemetryValue::from((capture.capture_time_s - entry.time_s).max(0.0)),
            ),
        ]);
    }
    if let Some(target_state) = capture.target_state {
        insert_waypoint_target_state_metrics(&mut metadata, target_state);
    }
    if let Some(audit) = capture.transition_audit {
        insert_waypoint_transition_audit_metrics(&mut metadata, audit);
    }
    standard_marker(
        crate::kit::marker::WAYPOINT_HANDOFF,
        &format!("waypoint handoff: {waypoint_id}"),
        &view,
        metadata,
    )
}

#[cfg(test)]
// Focused controller tests intentionally start from defaults and mutate only
// the state relevant to the behavior under test.
#[allow(clippy::field_reassign_with_default)]
mod tests;
