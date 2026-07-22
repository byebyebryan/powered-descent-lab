use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum GuidanceMode {
    NominalPending,
    NominalReady,
    LatestSafe,
}

impl GuidanceMode {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::NominalPending => "nominal pending",
            Self::NominalReady => "nominal ready",
            Self::LatestSafe => "latest safe",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TerminalEntryMode {
    Pending,
    NominalReady,
    LatestSafe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalEntryTerrainPolicy {
    Configured,
    Ignore,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TerminalEntryRequest {
    pub(crate) lateral_dx_m: f64,
    pub(crate) ready_ticks: u32,
    pub(crate) terrain_policy: TerminalEntryTerrainPolicy,
}

impl TerminalEntryMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::NominalReady => "nominal_ready",
            Self::LatestSafe => "latest_safe",
        }
    }

    pub(crate) fn is_ready(self) -> bool {
        matches!(self, Self::NominalReady | Self::LatestSafe)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BallisticProjection {
    pub(super) projected_dx_m: f64,
    pub(super) time_to_cross_s: f64,
    pub(super) has_target_y_solution: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TerrainClearanceEstimate {
    pub(super) min_clearance_m: f64,
    pub(super) first_violation_time_s: Option<f64>,
    pub(super) safe: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TerminalGateCandidate {
    pub(super) burn_time_s: f64,
    pub(super) required_accel_ratio: f64,
    pub(super) upward_accel_mps2: f64,
    pub(super) tilt_feasible: bool,
    pub(super) ready: bool,
    pub(super) terrain_min_clearance_m: f64,
    pub(super) terrain_first_violation_time_s: Option<f64>,
    pub(super) terrain_clearance_safe: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TerminalGuidancePlan {
    pub(super) arrival_time_s: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GuidancePlanReleaseReason {
    CapturedBrakingBoundary,
    VerticalBrakingMargin,
}

impl GuidancePlanReleaseReason {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::CapturedBrakingBoundary => "captured_braking_boundary",
            Self::VerticalBrakingMargin => "vertical_braking_margin",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TerminalCommandState {
    pub(super) mode: GuidanceMode,
    pub(super) candidate: TerminalGateCandidate,
    pub(super) projected_dx_m: f64,
    pub(super) projected_time_s: f64,
    pub(super) has_target_y_solution: bool,
    pub(super) desired_vertical_speed_mps: f64,
    pub(super) target_attitude_rad: f64,
    pub(super) throttle_frac: f64,
    pub(super) max_tilt_rad: f64,
    pub(super) latest_safe_margin_s: f64,
    pub(super) candidate_burn_time_s: f64,
    pub(super) plan_arrival_time_s: Option<f64>,
    pub(super) plan_replan_count: u32,
    pub(super) plan_release_reason: Option<GuidancePlanReleaseReason>,
    pub(super) vertical_braking_margin_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TerminalEntryAssessment {
    pub(crate) mode: TerminalEntryMode,
    pub(crate) ready_ticks: u32,
    pub(crate) burn_time_s: f64,
    pub(crate) latest_safe_margin_s: f64,
    pub(crate) required_accel_ratio: f64,
    pub(crate) terrain_min_clearance_m: f64,
    pub(crate) terrain_clearance_safe: bool,
    pub(crate) deferred: bool,
}

impl TerminalEntryAssessment {
    pub(crate) fn is_ready(self) -> bool {
        self.mode.is_ready()
    }

    pub(crate) fn forced_pending(mut self) -> Self {
        self.mode = TerminalEntryMode::Pending;
        self.ready_ticks = 0;
        self
    }

    pub(crate) fn deferred_pending(mut self) -> Self {
        self.mode = TerminalEntryMode::Pending;
        self.ready_ticks = 0;
        self.deferred = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SettledDescentCommand {
    pub(super) command: Command,
    pub(super) target_down_speed_mps: f64,
}
