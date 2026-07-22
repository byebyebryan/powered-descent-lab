use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TransferPhase {
    Takeoff,
    Boost,
    Coast,
    Terminal,
}

impl TransferPhase {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Takeoff => "takeoff",
            Self::Boost => "boost",
            Self::Coast => "coast",
            Self::Terminal => "terminal",
        }
    }
}

pub(super) fn waypoint_post_capture_phase(
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

pub(super) fn waypoint_terminal_spatial_ownership(
    config: &TransferPdgControllerConfig,
    observation: &Observation,
) -> bool {
    observation.height_above_target_m >= 0.0
        || (observation.target_dx_m.abs() <= config.terminal_gate_dx_m
            && -observation.height_above_target_m <= config.terminal_gate_altitude_m)
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferBallisticProjection {
    pub(super) has_target_y_solution: bool,
    pub(super) projected_time_s: Option<f64>,
    pub(super) projected_dx_m: Option<f64>,
    pub(super) impact_angle_deg: Option<f64>,
    pub(super) apex_over_target_m: f64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferBoostQuality {
    pub(super) verdict: &'static str,
    pub(super) passed: bool,
    pub(super) apex_target_over_target_m: f64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferBoostAnchor {
    pub(super) route_dx_m: f64,
    pub(super) route_dy_m: f64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferDiagnostics {
    pub(super) route_dx_m: f64,
    pub(super) route_dy_m: f64,
    pub(super) anchor: Option<TransferBoostAnchor>,
    pub(super) projection: TransferBallisticProjection,
    pub(super) boost_quality: TransferBoostQuality,
}

#[derive(Clone, Debug)]
pub(super) struct WaypointUpdateContext {
    pub(super) observation: Observation,
    pub(super) allow_terminal: bool,
    pub(super) telemetry: WaypointTelemetry,
    pub(super) guidance: Option<WaypointGuidanceFrame>,
    pub(super) capture: Option<WaypointCaptureSnapshot>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaypointTelemetry {
    pub(super) active_index: i64,
    pub(super) active_leg_index: i64,
    pub(super) capture_status: &'static str,
    pub(super) capture_time_s: Option<f64>,
    pub(super) closest_distance_m: f64,
    pub(super) distance_m: f64,
    pub(super) cross_track_m: f64,
    pub(super) plane_progress_m: f64,
    pub(super) outbound_heading_error_rad: f64,
    pub(super) outbound_progress_mps: f64,
    pub(super) outbound_cross_speed_mps: f64,
    pub(super) speed_mps: f64,
    pub(super) vertical_speed_mps: f64,
    pub(super) remaining_to_plane_m: f64,
    pub(super) time_to_plane_s: f64,
    pub(super) required_turn_distance_m: f64,
    pub(super) shaping_start_distance_m: f64,
    pub(super) turn_margin_m: f64,
    pub(super) center_m: Vec2,
    pub(super) nominal_handoff_target_m: Vec2,
    pub(super) handoff_target_m: Vec2,
    pub(super) handoff_target_mode: &'static str,
    pub(super) remaining_to_handoff_m: f64,
    pub(super) time_to_handoff_s: f64,
    pub(super) handoff_turn_margin_m: f64,
    pub(super) endpoint_m: Vec2,
    pub(super) steering_target_m: Vec2,
    pub(super) target_state: Option<WaypointGuidanceTargetState>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaypointCaptureSnapshot {
    pub(super) index: usize,
    pub(super) capture_time_s: f64,
    pub(super) status: &'static str,
    pub(super) closest_distance_m: f64,
    pub(super) distance_m: f64,
    pub(super) cross_track_m: f64,
    pub(super) plane_progress_m: f64,
    pub(super) outbound_heading_error_rad: f64,
    pub(super) outbound_progress_mps: f64,
    pub(super) outbound_cross_speed_mps: f64,
    pub(super) speed_mps: f64,
    pub(super) vertical_speed_mps: f64,
    pub(super) approach: WaypointApproachState,
    pub(super) center_m: Vec2,
    pub(super) nominal_handoff_target_m: Vec2,
    pub(super) handoff_target_m: Vec2,
    pub(super) handoff_target_mode: &'static str,
    pub(super) endpoint_m: Vec2,
    pub(super) steering_target_m: Vec2,
    pub(super) target_state: Option<WaypointGuidanceTargetState>,
    pub(super) transition_audit: Option<WaypointTransitionAudit>,
    pub(super) guidance_replan_count: u32,
    pub(super) window_entry: Option<WaypointWindowEntrySnapshot>,
    pub(super) resolution_reason: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaypointWindowEntrySnapshot {
    pub(super) time_s: f64,
    pub(super) position_m: Vec2,
    pub(super) velocity_mps: Vec2,
    pub(super) stats: WaypointLegStats,
    pub(super) assessment: WaypointGuidanceAssessment,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaypointLegGeometry<'a> {
    pub(super) active_index: usize,
    pub(super) waypoint: &'a TransferWaypointSpec,
    pub(super) anchor_m: Vec2,
    pub(super) target_m: Vec2,
    pub(super) leg_unit: Vec2,
    pub(super) leg_length_m: f64,
    pub(super) handoff_tangent_unit: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointLegStats {
    pub(super) distance_m: f64,
    pub(super) cross_track_m: f64,
    pub(super) plane_progress_m: f64,
    pub(super) outbound_heading_error_rad: f64,
    pub(super) outbound_progress_mps: f64,
    pub(super) outbound_cross_speed_mps: f64,
    pub(super) speed_mps: f64,
    pub(super) vertical_speed_mps: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointApproachState {
    pub(super) remaining_to_plane_m: f64,
    pub(super) time_to_plane_s: f64,
    pub(super) remaining_to_handoff_m: f64,
    pub(super) time_to_handoff_s: f64,
    pub(super) required_turn_distance_m: f64,
    pub(super) shaping_start_distance_m: f64,
    pub(super) turn_margin_m: f64,
    pub(super) handoff_turn_margin_m: f64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaypointGuidanceFrame {
    pub(super) active_index: usize,
    pub(super) center_m: Vec2,
    pub(super) nominal_handoff_target_m: Vec2,
    pub(super) handoff_target_m: Vec2,
    pub(super) handoff_target_mode: &'static str,
    pub(super) endpoint_m: Vec2,
    pub(super) steering_target_m: Vec2,
    pub(super) leg_unit: Vec2,
    pub(super) handoff_tangent_unit: Vec2,
    pub(super) envelope: WaypointGuidanceEnvelope,
    pub(super) approach: WaypointApproachState,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaypointGuidanceEnvelope {
    pub(super) capture_radius_m: f64,
    pub(super) max_cross_track_m: f64,
    pub(super) max_outbound_heading_error_rad: f64,
    pub(super) min_outbound_progress_mps: f64,
    pub(super) max_outbound_cross_speed_mps: Option<f64>,
    pub(super) min_speed_mps: f64,
    pub(super) max_speed_mps: f64,
    pub(super) min_vertical_speed_mps: Option<f64>,
    pub(super) max_vertical_speed_mps: Option<f64>,
}

impl WaypointGuidanceEnvelope {
    pub(super) fn assess(self, stats: WaypointLegStats) -> WaypointGuidanceAssessment {
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
pub(super) struct WaypointGuidanceAssessment {
    pub(super) triggered: bool,
    pub(super) capture_window_open: bool,
    pub(super) deadline_reached: bool,
    pub(super) spatial_pass: bool,
    pub(super) violation_mask: u8,
}

impl WaypointGuidanceAssessment {
    pub(super) fn envelope_pass(self) -> bool {
        self.violation_mask == 0
    }

    pub(super) fn contract_pass(self) -> bool {
        self.triggered && self.spatial_pass && self.envelope_pass()
    }

    pub(super) fn contract_pass_in_window(self, window_open: bool) -> bool {
        (self.triggered || window_open) && self.spatial_pass && self.envelope_pass()
    }

    pub(super) fn resolved_in_window(self, window_open: bool) -> bool {
        self.contract_pass_in_window(window_open) || self.deadline_reached
    }

    pub(super) fn with_window_open(mut self, window_open: bool) -> Self {
        self.capture_window_open |= window_open;
        self.triggered |= window_open;
        self
    }

    pub(super) fn reasons(self) -> String {
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
pub(super) struct WaypointGuidancePrediction {
    pub(super) time_to_event_s: f64,
    pub(super) deadline_lead_s: f64,
    pub(super) stats: WaypointLegStats,
    pub(super) assessment: WaypointGuidanceAssessment,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointReachablePrediction {
    pub(super) prediction: WaypointGuidancePrediction,
    pub(super) event_state: TransferSimState,
    pub(super) required_accel_ratio_max: f64,
    pub(super) thrust_saturated_time_s: f64,
    pub(super) tilt_saturated_time_s: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointContinuationPrediction {
    pub(super) next_waypoint_index: usize,
    pub(super) source_event_state: TransferSimState,
    pub(super) source_event_time_s: f64,
    pub(super) prediction: WaypointReachablePrediction,
    pub(super) passing_candidate_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointTransitionAudit {
    pub(super) next_waypoint_index: usize,
    pub(super) position_error_m: f64,
    pub(super) velocity_error_mps: f64,
    pub(super) attitude_error_rad: f64,
    pub(super) mass_error_kg: f64,
    pub(super) fuel_error_kg: f64,
    pub(super) event_time_error_s: f64,
    pub(super) continuation_prediction: WaypointReachablePrediction,
    pub(super) passing_candidate_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointGuidancePlan {
    pub(super) waypoint_index: usize,
    pub(super) revision: u32,
    pub(super) reason: WaypointGuidancePlanReason,
    pub(super) created_time_s: f64,
    pub(super) start_position_m: Vec2,
    pub(super) start_velocity_mps: Vec2,
    pub(super) endpoint_m: Vec2,
    pub(super) target_mode: &'static str,
    pub(super) target_velocity_mps: Vec2,
    pub(super) arrival_time_s: f64,
    pub(super) target_envelope_feasible: bool,
    pub(super) final_terminal_required_accel_ratio: Option<f64>,
    pub(super) final_terminal_recoverable: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointFinalRecoverySearchAttempt {
    pub(super) plan_revision: u32,
    pub(super) time_to_event_s: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WaypointGuidancePlanReason {
    Initial,
    Expired,
    AuthorityRecovery,
    ContractRecovery,
    ReachableRecovery,
}

impl WaypointGuidancePlanReason {
    pub(super) fn as_str(self) -> &'static str {
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
pub(super) struct WaypointGuidanceTrackability {
    pub(super) plan_index: usize,
    pub(super) plan_revision: u32,
    pub(super) plan_reason: WaypointGuidancePlanReason,
    pub(super) plan_age_s: f64,
    pub(super) reference_position_error_m: f64,
    pub(super) reference_cross_error_m: f64,
    pub(super) reference_velocity_error_mps: f64,
    pub(super) reference_cross_speed_error_mps: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointGuidanceCandidate {
    pub(super) target_velocity_mps: Vec2,
    pub(super) time_to_go_s: f64,
    pub(super) required_accel_mps2: Vec2,
    pub(super) required_accel_ratio: f64,
    pub(super) tilt_feasible: bool,
    pub(super) target_envelope_feasible: bool,
    pub(super) prediction: WaypointGuidancePrediction,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointReachableCandidate {
    pub(super) candidate: WaypointGuidanceCandidate,
    pub(super) endpoint_m: Vec2,
    pub(super) target_mode: &'static str,
    pub(super) reachable_prediction: WaypointReachablePrediction,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointFinalCandidate {
    pub(super) reachable: WaypointReachableCandidate,
    pub(super) terminal_gate: TerminalEntryAssessment,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointGuidancePlanSelection {
    pub(super) candidate: WaypointGuidanceCandidate,
    pub(super) endpoint_m: Vec2,
    pub(super) target_mode: &'static str,
    pub(super) terminal_gate: Option<TerminalEntryAssessment>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointJointCandidatePrediction {
    pub(super) current: WaypointReachableCandidate,
    pub(super) continuation: WaypointReachablePrediction,
    pub(super) continuation_passing_candidate_count: usize,
}

impl WaypointJointCandidatePrediction {
    pub(super) fn contract_pass(self) -> bool {
        self.current
            .reachable_prediction
            .prediction
            .assessment
            .contract_pass()
            && self.continuation.prediction.assessment.contract_pass()
    }

    pub(super) fn total_saturated_time_s(self) -> f64 {
        self.current.reachable_prediction.thrust_saturated_time_s
            + self.current.reachable_prediction.tilt_saturated_time_s
            + self.continuation.thrust_saturated_time_s
            + self.continuation.tilt_saturated_time_s
    }

    pub(super) fn required_accel_ratio_max(self) -> f64 {
        self.current
            .reachable_prediction
            .required_accel_ratio_max
            .max(self.continuation.required_accel_ratio_max)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaypointJointSearchPrediction {
    pub(super) next_waypoint_index: usize,
    pub(super) selected: Option<WaypointJointCandidatePrediction>,
    pub(super) passing_candidate_count: usize,
    pub(super) evaluated_candidate_count: usize,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaypointGuidanceCommandState {
    pub(super) command: Command,
    pub(super) target_velocity_mps: Vec2,
    pub(super) time_to_go_s: f64,
    pub(super) required_accel_ratio: f64,
    pub(super) feasible: bool,
    pub(super) path_correction_mps2: Vec2,
    pub(super) deadline_remaining_s: f64,
    pub(super) velocity_error_mps: f64,
    pub(super) authority_margin: f64,
    pub(super) thrust_saturated: bool,
    pub(super) tilt_saturated: bool,
    pub(super) trackability: WaypointGuidanceTrackability,
    pub(super) prediction: WaypointGuidancePrediction,
    pub(super) reachable_prediction: WaypointReachablePrediction,
    pub(super) continuation_prediction: Option<WaypointContinuationPrediction>,
    pub(super) joint_prediction: Option<WaypointJointSearchPrediction>,
    pub(super) final_terminal_required_accel_ratio: Option<f64>,
    pub(super) final_terminal_recoverable: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaypointGuidanceTargetState {
    pub(super) target_velocity_mps: Vec2,
    pub(super) deadline_remaining_s: f64,
    pub(super) velocity_error_mps: f64,
    pub(super) feasible: bool,
    pub(super) authority_margin: f64,
    pub(super) thrust_saturated: bool,
    pub(super) tilt_saturated: bool,
    pub(super) trackability: WaypointGuidanceTrackability,
    pub(super) prediction: WaypointGuidancePrediction,
    pub(super) reachable_prediction: WaypointReachablePrediction,
    pub(super) continuation_prediction: Option<WaypointContinuationPrediction>,
    pub(super) joint_prediction: Option<WaypointJointSearchPrediction>,
    pub(super) final_terminal_required_accel_ratio: Option<f64>,
    pub(super) final_terminal_recoverable: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TransferSimState {
    pub(super) position_m: Vec2,
    pub(super) velocity_mps: Vec2,
    pub(super) attitude_rad: f64,
    pub(super) fuel_kg: f64,
    pub(super) dry_mass_kg: f64,
}

impl TransferSimState {
    pub(super) fn mass_kg(self) -> f64 {
        (self.dry_mass_kg + self.fuel_kg.max(0.0)).max(1.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferBoostCandidateScore {
    pub(super) score: f64,
    pub(super) projection: TransferBallisticProjection,
    pub(super) quality: TransferBoostQuality,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferBoostCommandSelection {
    pub(super) command: Command,
    pub(super) scoring_mode: &'static str,
    pub(super) selected_score: f64,
    pub(super) settled_projection: TransferBallisticProjection,
    pub(super) settled_quality: TransferBoostQuality,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TransferCorridorState {
    pub(super) mode: &'static str,
    pub(super) active: bool,
    pub(super) tilt_limited: bool,
    pub(super) margin_m: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TransferGuidanceMode {
    Direct,
    Waypoint,
}

impl TransferGuidanceMode {
    pub(super) fn from_config(config: &TransferPdgControllerConfig) -> Self {
        if config.waypoint_guidance_enabled {
            Self::Waypoint
        } else {
            Self::Direct
        }
    }

    pub(super) fn uses_waypoints(self) -> bool {
        self == Self::Waypoint
    }
}

impl TransferCorridorState {
    pub(super) fn inactive() -> Self {
        Self {
            mode: "inactive",
            active: false,
            tilt_limited: false,
            margin_m: 1.0e9,
        }
    }
}
