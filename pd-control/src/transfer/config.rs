use super::*;

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
