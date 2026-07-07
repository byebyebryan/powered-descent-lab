use crate::kit::{ControllerFrameBuilder, ControllerView, metric, phase, standard_marker};
use crate::terminal_pdg::{
    TerminalPdgController, TerminalPdgControllerConfig, TransferGateReadiness,
    TransferGateReadinessMode,
};
use crate::{Controller, ControllerFrame, TelemetryValue};
use pd_core::{Command, Observation, RunContext, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
const TRANSFER_CORRIDOR_SAMPLE_COUNT: usize = 24;
const TRANSFER_BOOST_SCORE_NO_TARGET_Y: f64 = 10_000.0;
const TRANSFER_BOOST_SCORE_PROJECTED_DX: f64 = 100.0;
const TRANSFER_BOOST_SCORE_PROJECTED_DX_CENTERING: f64 = 45.0;
const TRANSFER_BOOST_SCORE_SHORTFALL: f64 = 45.0;
const TRANSFER_BOOST_SCORE_MIN_ANGLE: f64 = 60.0;
const TRANSFER_BOOST_SCORE_TARGET_ANGLE: f64 = 20.0;
const TRANSFER_BOOST_SCORE_APEX_UNDERSHOOT: f64 = 18.0;
const TRANSFER_BOOST_SCORE_APEX_OVERSHOOT: f64 = 10.0;
const TRANSFER_BOOST_SCORE_THROTTLE_EFFORT: f64 = 1.0;
const TRANSFER_BOOST_SCORE_TILT_EFFORT: f64 = 0.4;
const TRANSFER_BOOST_RECOVERY_SCORE_ENDPOINT_WEIGHT: f64 = 0.05;
const TRANSFER_BOOST_RECOVERY_SCORE_SETTLED_WEIGHT: f64 = 0.02;
const TRANSFER_BOOST_RECOVERY_SCORE_LATEST_SAFE_MARGIN: f64 = 14.0;
const TRANSFER_BOOST_RECOVERY_SCORE_ACCEL_RATIO: f64 = 1.2;
const TRANSFER_BOOST_RECOVERY_SCORE_PASS_NOT_READY: f64 = 45.0;
const TRANSFER_BOOST_RECOVERY_SCORE_TERRAIN_UNSAFE: f64 = 1_200.0;
const TRANSFER_GATE_DEFER_MAX_NEGATIVE_MARGIN_S: f64 = -0.75;
const TRANSFER_PRE_TARGET_CAPTURE_MAX_LATEST_SAFE_MARGIN_S: f64 = 0.75;
const TRANSFER_PRE_TARGET_CAPTURE_LOOKAHEAD_S: f64 = 1.5;
const TRANSFER_SOURCE_CLEARANCE_SAMPLE_COUNT: usize = 16;

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
pub struct BaselineControllerConfig {
    pub horizontal_position_gain: f64,
    pub horizontal_velocity_limit_mps: f64,
    pub horizontal_velocity_gain: f64,
    pub high_altitude_m: f64,
    pub medium_altitude_m: f64,
    pub low_altitude_m: f64,
    pub high_descent_rate_mps: f64,
    pub medium_descent_rate_mps: f64,
    pub low_descent_rate_mps: f64,
    pub touchdown_descent_rate_mps: f64,
    pub high_attitude_limit_rad: f64,
    pub medium_attitude_limit_rad: f64,
    pub low_attitude_limit_rad: f64,
    pub vertical_speed_gain: f64,
    pub tilt_throttle_gain: f64,
}

impl Default for BaselineControllerConfig {
    fn default() -> Self {
        Self {
            horizontal_position_gain: 0.08,
            horizontal_velocity_limit_mps: 5.0,
            horizontal_velocity_gain: 0.08,
            high_altitude_m: 80.0,
            medium_altitude_m: 30.0,
            low_altitude_m: 12.0,
            high_descent_rate_mps: -18.0,
            medium_descent_rate_mps: -10.0,
            low_descent_rate_mps: -5.0,
            touchdown_descent_rate_mps: -2.0,
            high_attitude_limit_rad: 0.45,
            medium_attitude_limit_rad: 0.25,
            low_attitude_limit_rad: 0.12,
            vertical_speed_gain: 0.09,
            tilt_throttle_gain: 0.04,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StagedDescentControllerConfig {
    pub translate_altitude_m: f64,
    pub flare_altitude_m: f64,
    pub touchdown_altitude_m: f64,
    pub lateral_capture_margin_m: f64,
    pub translate_position_gain: f64,
    pub descent_position_gain: f64,
    pub final_position_gain: f64,
    pub translate_velocity_limit_mps: f64,
    pub descent_velocity_limit_mps: f64,
    pub final_velocity_limit_mps: f64,
    pub lateral_velocity_gain: f64,
    pub translate_attitude_limit_rad: f64,
    pub descent_attitude_limit_rad: f64,
    pub touchdown_attitude_limit_rad: f64,
    pub translate_descent_rate_mps: f64,
    pub descent_rate_mps: f64,
    pub flare_descent_rate_mps: f64,
    pub touchdown_descent_rate_mps: f64,
    pub vertical_speed_gain: f64,
    pub tilt_throttle_gain: f64,
}

impl Default for StagedDescentControllerConfig {
    fn default() -> Self {
        Self {
            translate_altitude_m: 85.0,
            flare_altitude_m: 24.0,
            touchdown_altitude_m: 9.0,
            lateral_capture_margin_m: 5.0,
            translate_position_gain: 0.16,
            descent_position_gain: 0.1,
            final_position_gain: 0.06,
            translate_velocity_limit_mps: 6.0,
            descent_velocity_limit_mps: 3.5,
            final_velocity_limit_mps: 1.5,
            lateral_velocity_gain: 0.1,
            translate_attitude_limit_rad: 0.38,
            descent_attitude_limit_rad: 0.22,
            touchdown_attitude_limit_rad: 0.1,
            translate_descent_rate_mps: -12.0,
            descent_rate_mps: -8.0,
            flare_descent_rate_mps: -4.0,
            touchdown_descent_rate_mps: -1.7,
            vertical_speed_gain: 0.1,
            tilt_throttle_gain: 0.05,
        }
    }
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
        let mut terminal = TerminalPdgControllerConfig::default();
        terminal.terminal_gate_burn_time_max_s = 22.0;
        terminal.terminal_gate_burn_time_offset_long_s = 2.0;

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ControllerSpec {
    Idle,
    BaselineV1 {
        #[serde(flatten)]
        config: BaselineControllerConfig,
    },
    StagedDescentV1 {
        #[serde(flatten)]
        config: StagedDescentControllerConfig,
    },
    TerminalPdgV1 {
        #[serde(flatten)]
        config: TerminalPdgControllerConfig,
    },
    TransferPdgV1 {
        #[serde(flatten)]
        config: TransferPdgControllerConfig,
    },
}

impl ControllerSpec {
    pub fn id(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::BaselineV1 { .. } => "baseline_v1",
            Self::StagedDescentV1 { .. } => "staged_descent_v1",
            Self::TerminalPdgV1 { .. } => "terminal_pdg_v1",
            Self::TransferPdgV1 { config } if config.boost_recoverability_scoring_enabled => {
                "transfer_pdg_recoverability_v1"
            }
            Self::TransferPdgV1 { config } if config.boost_pathwise_scoring_enabled => {
                "transfer_pdg_pathwise_v1"
            }
            Self::TransferPdgV1 { .. } => "transfer_pdg_v1",
        }
    }

    pub fn instantiate(&self) -> Box<dyn Controller> {
        match self {
            Self::Idle => Box::new(IdleController),
            Self::BaselineV1 { config } => Box::new(BaselineController::new(config.clone())),
            Self::StagedDescentV1 { config } => {
                Box::new(StagedDescentController::new(config.clone()))
            }
            Self::TerminalPdgV1 { config } => Box::new(TerminalPdgController::new(config.clone())),
            Self::TransferPdgV1 { config } => Box::new(TransferPdgController::new(config.clone())),
        }
    }
}

pub fn built_in_controller_spec(name: &str) -> Option<ControllerSpec> {
    match name {
        "idle" => Some(ControllerSpec::Idle),
        "baseline" | "baseline_v1" => Some(ControllerSpec::BaselineV1 {
            config: BaselineControllerConfig::default(),
        }),
        "staged" | "staged_descent" | "staged_descent_v1" => {
            Some(ControllerSpec::StagedDescentV1 {
                config: StagedDescentControllerConfig::default(),
            })
        }
        "terminal_pdg" | "terminal_pdg_v1" | "tpdg" => Some(ControllerSpec::TerminalPdgV1 {
            config: TerminalPdgControllerConfig::default(),
        }),
        "transfer_pdg" | "transfer_pdg_v1" | "xpdg" => Some(ControllerSpec::TransferPdgV1 {
            config: TransferPdgControllerConfig::default(),
        }),
        "transfer_pdg_pathwise" | "transfer_pdg_pathwise_v1" | "xpdg_pathwise" => {
            let mut config = TransferPdgControllerConfig::default();
            config.boost_pathwise_scoring_enabled = true;
            Some(ControllerSpec::TransferPdgV1 { config })
        }
        "transfer_pdg_recoverability"
        | "transfer_pdg_recoverability_v1"
        | "xpdg_recoverability" => {
            let mut config = TransferPdgControllerConfig::default();
            config.boost_recoverability_scoring_enabled = true;
            Some(ControllerSpec::TransferPdgV1 { config })
        }
        "terminal_pdg_no_terrain" | "tpdg_no_terrain" => {
            let mut config = TerminalPdgControllerConfig::default();
            config.terrain_clearance_enabled = false;
            Some(ControllerSpec::TerminalPdgV1 { config })
        }
        _ => None,
    }
}

#[derive(Debug, Default)]
pub struct IdleController;

impl Controller for IdleController {
    fn id(&self) -> &str {
        "idle"
    }

    fn update(&mut self, ctx: &RunContext, observation: &Observation) -> ControllerFrame {
        let view = ControllerView::new(ctx, observation);
        ControllerFrameBuilder::new(Command::idle())
            .status("idle")
            .phase(phase::IDLE)
            .standard_kinematics(&view)
            .metric(metric::GUIDANCE_ACTIVE, false)
            .build()
    }
}

#[derive(Debug)]
pub struct BaselineController {
    config: BaselineControllerConfig,
    last_phase: Option<String>,
}

impl Default for BaselineController {
    fn default() -> Self {
        Self::new(BaselineControllerConfig::default())
    }
}

impl BaselineController {
    pub fn new(config: BaselineControllerConfig) -> Self {
        Self {
            config,
            last_phase: None,
        }
    }

    fn phase_for_altitude(&self, altitude_m: f64) -> &'static str {
        if altitude_m > self.config.high_altitude_m {
            phase::ACQUIRE
        } else if altitude_m > self.config.medium_altitude_m {
            phase::DESCENT
        } else if altitude_m > self.config.low_altitude_m {
            phase::FLARE
        } else {
            phase::TOUCHDOWN
        }
    }

    fn desired_vertical_speed_mps(&self, altitude_m: f64) -> f64 {
        if altitude_m > self.config.high_altitude_m {
            self.config.high_descent_rate_mps
        } else if altitude_m > self.config.medium_altitude_m {
            self.config.medium_descent_rate_mps
        } else if altitude_m > self.config.low_altitude_m {
            self.config.low_descent_rate_mps
        } else {
            self.config.touchdown_descent_rate_mps
        }
    }
}

impl Controller for BaselineController {
    fn id(&self) -> &str {
        "baseline_v1"
    }

    fn reset(&mut self, _ctx: &RunContext) {
        self.last_phase = None;
    }

    fn update(&mut self, ctx: &RunContext, observation: &Observation) -> ControllerFrame {
        let view = ControllerView::new(ctx, observation);
        let altitude_m = view.altitude_m();
        let desired_tangential_speed_mps =
            (view.target_dx_m() * self.config.horizontal_position_gain).clamp(
                -self.config.horizontal_velocity_limit_mps,
                self.config.horizontal_velocity_limit_mps,
            );

        let attitude_limit_rad = if altitude_m > self.config.medium_altitude_m {
            self.config.high_attitude_limit_rad
        } else if altitude_m > self.config.low_altitude_m {
            self.config.medium_attitude_limit_rad
        } else {
            self.config.low_attitude_limit_rad
        };
        let target_attitude_rad = view.desired_attitude_for_tangential_speed(
            desired_tangential_speed_mps,
            self.config.horizontal_velocity_gain,
            attitude_limit_rad,
        );

        let desired_vertical_speed_mps = self.desired_vertical_speed_mps(altitude_m);
        let (throttle_frac, vertical_error_mps) = view.throttle_for_vertical_target(
            desired_vertical_speed_mps,
            self.config.vertical_speed_gain,
            self.config.tilt_throttle_gain,
            target_attitude_rad,
        );

        let phase = self.phase_for_altitude(altitude_m).to_owned();
        let status = match phase.as_str() {
            phase::ACQUIRE => "tracking target pad",
            phase::DESCENT => "stabilizing descent rate",
            phase::FLARE => "reducing sink and tilt",
            phase::TOUCHDOWN => "final touchdown envelope",
            _ => "guiding",
        };

        let frame = ControllerFrameBuilder::new(Command {
            throttle_frac,
            target_attitude_rad,
        })
        .status(status)
        .phase(phase.clone())
        .standard_kinematics(&view)
        .phase_transition_marker(self.last_phase.as_deref(), &phase, &view)
        .metric(metric::GUIDANCE_ACTIVE, true)
        .metric(
            metric::DESIRED_TANGENTIAL_SPEED_MPS,
            desired_tangential_speed_mps,
        )
        .metric(
            metric::DESIRED_VERTICAL_SPEED_MPS,
            desired_vertical_speed_mps,
        )
        .metric(metric::DESIRED_ATTITUDE_RAD, target_attitude_rad)
        .metric(metric::HOVER_THROTTLE, view.hover_throttle_frac())
        .metric(metric::VERTICAL_ERROR_MPS, vertical_error_mps)
        .metric(
            metric::LATERAL_ERROR_MPS,
            desired_tangential_speed_mps - view.tangential_velocity_mps(),
        )
        .build();

        self.last_phase = Some(phase);
        frame
    }
}

#[derive(Debug)]
pub struct StagedDescentController {
    config: StagedDescentControllerConfig,
    last_phase: Option<String>,
    lateral_capture_marked: bool,
    terminal_gate_marked: bool,
}

impl Default for StagedDescentController {
    fn default() -> Self {
        Self::new(StagedDescentControllerConfig::default())
    }
}

impl StagedDescentController {
    pub fn new(config: StagedDescentControllerConfig) -> Self {
        Self {
            config,
            last_phase: None,
            lateral_capture_marked: false,
            terminal_gate_marked: false,
        }
    }

    fn phase_for_view(&self, view: &ControllerView<'_>) -> &'static str {
        let altitude_m = view.altitude_m();
        if altitude_m > self.config.translate_altitude_m
            && view.target_dx_m().abs() > self.config.lateral_capture_margin_m
        {
            phase::TRANSLATE
        } else if altitude_m > self.config.flare_altitude_m {
            phase::DESCENT
        } else if altitude_m > self.config.touchdown_altitude_m {
            phase::FLARE
        } else {
            phase::TOUCHDOWN
        }
    }

    fn desired_tangential_speed_mps(&self, view: &ControllerView<'_>, phase_name: &str) -> f64 {
        let (gain, limit) = match phase_name {
            phase::TRANSLATE => (
                self.config.translate_position_gain,
                self.config.translate_velocity_limit_mps,
            ),
            phase::DESCENT => (
                self.config.descent_position_gain,
                self.config.descent_velocity_limit_mps,
            ),
            _ => (
                self.config.final_position_gain,
                self.config.final_velocity_limit_mps,
            ),
        };

        (view.target_dx_m() * gain).clamp(-limit, limit)
    }

    fn attitude_limit_rad(&self, phase_name: &str) -> f64 {
        match phase_name {
            phase::TRANSLATE => self.config.translate_attitude_limit_rad,
            phase::DESCENT | phase::FLARE => self.config.descent_attitude_limit_rad,
            _ => self.config.touchdown_attitude_limit_rad,
        }
    }

    fn desired_vertical_speed_mps(&self, phase_name: &str, view: &ControllerView<'_>) -> f64 {
        let base = match phase_name {
            phase::TRANSLATE => self.config.translate_descent_rate_mps,
            phase::DESCENT => self.config.descent_rate_mps,
            phase::FLARE => self.config.flare_descent_rate_mps,
            _ => self.config.touchdown_descent_rate_mps,
        };

        if phase_name == phase::TRANSLATE
            && view.target_dx_m().abs() > (view.observation.target_pad_half_width_m * 0.9)
        {
            base.max(-9.0)
        } else {
            base
        }
    }
}

impl Controller for StagedDescentController {
    fn id(&self) -> &str {
        "staged_descent_v1"
    }

    fn reset(&mut self, _ctx: &RunContext) {
        self.last_phase = None;
        self.lateral_capture_marked = false;
        self.terminal_gate_marked = false;
    }

    fn update(&mut self, ctx: &RunContext, observation: &Observation) -> ControllerFrame {
        let view = ControllerView::new(ctx, observation);
        let phase = self.phase_for_view(&view).to_owned();
        let desired_tangential_speed_mps = self.desired_tangential_speed_mps(&view, &phase);
        let target_attitude_rad = view.desired_attitude_for_tangential_speed(
            desired_tangential_speed_mps,
            self.config.lateral_velocity_gain,
            self.attitude_limit_rad(&phase),
        );
        let desired_vertical_speed_mps = self.desired_vertical_speed_mps(&phase, &view);
        let (throttle_frac, vertical_error_mps) = view.throttle_for_vertical_target(
            desired_vertical_speed_mps,
            self.config.vertical_speed_gain,
            self.config.tilt_throttle_gain,
            target_attitude_rad,
        );

        let status = match phase.as_str() {
            phase::TRANSLATE => "capturing lateral error before descent",
            phase::DESCENT => "descending inside the pad corridor",
            phase::FLARE => "trimming sink inside terminal gate",
            phase::TOUCHDOWN => "holding final touchdown corridor",
            _ => "guiding",
        };

        let mut builder = ControllerFrameBuilder::new(Command {
            throttle_frac,
            target_attitude_rad,
        })
        .status(status)
        .phase(phase.clone())
        .standard_kinematics(&view)
        .phase_transition_marker(self.last_phase.as_deref(), &phase, &view)
        .metric(metric::GUIDANCE_ACTIVE, true)
        .metric(
            metric::DESIRED_TANGENTIAL_SPEED_MPS,
            desired_tangential_speed_mps,
        )
        .metric(
            metric::DESIRED_VERTICAL_SPEED_MPS,
            desired_vertical_speed_mps,
        )
        .metric(metric::DESIRED_ATTITUDE_RAD, target_attitude_rad)
        .metric(metric::HOVER_THROTTLE, view.hover_throttle_frac())
        .metric(metric::VERTICAL_ERROR_MPS, vertical_error_mps)
        .metric(
            metric::LATERAL_ERROR_MPS,
            desired_tangential_speed_mps - view.tangential_velocity_mps(),
        );

        if !self.lateral_capture_marked
            && view.target_dx_m().abs() <= self.config.lateral_capture_margin_m
        {
            builder = builder.marker(standard_marker(
                crate::kit::marker::LATERAL_CAPTURE,
                "lateral capture",
                &view,
                BTreeMap::from([
                    ("kind".to_owned(), TelemetryValue::from("gate")),
                    ("phase".to_owned(), TelemetryValue::from(phase.as_str())),
                ]),
            ));
            self.lateral_capture_marked = true;
        }

        if !self.terminal_gate_marked && phase == phase::FLARE {
            builder = builder.marker(standard_marker(
                crate::kit::marker::TERMINAL_GATE,
                "terminal descent gate",
                &view,
                BTreeMap::from([
                    ("kind".to_owned(), TelemetryValue::from("gate")),
                    ("phase".to_owned(), TelemetryValue::from(phase.as_str())),
                ]),
            ));
            self.terminal_gate_marked = true;
        }

        self.last_phase = Some(phase);
        builder.build()
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

#[derive(Clone, Copy, Debug)]
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
    terminal: TerminalPdgController,
    phase: TransferPhase,
    boost_anchor: Option<TransferBoostAnchor>,
    transfer_gate_ready_ticks: u32,
    last_transfer_gate: Option<TransferGateReadiness>,
    last_corridor: TransferCorridorState,
    last_phase: Option<String>,
}

impl Default for TransferPdgController {
    fn default() -> Self {
        Self::new(TransferPdgControllerConfig::default())
    }
}

impl TransferPdgController {
    pub fn new(config: TransferPdgControllerConfig) -> Self {
        let terminal = TerminalPdgController::new(config.terminal.clone());
        Self {
            config,
            terminal,
            phase: TransferPhase::Takeoff,
            boost_anchor: None,
            transfer_gate_ready_ticks: 0,
            last_transfer_gate: None,
            last_corridor: TransferCorridorState::inactive(),
            last_phase: None,
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
        if self.config.boost_recoverability_scoring_enabled {
            "recoverability"
        } else if self.config.boost_pathwise_scoring_enabled {
            "pathwise_geometry"
        } else {
            "legacy_endpoint"
        }
    }

    fn transfer_metrics_builder(
        &self,
        builder: ControllerFrameBuilder,
        diagnostics: TransferDiagnostics,
        gate: TransferGateReadiness,
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
                self.boost_scoring_mode(),
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
        gate: TransferGateReadiness,
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
    ) -> TransferGateReadiness {
        let lateral_dx_m = diagnostics
            .projection
            .projected_dx_m
            .filter(|_| diagnostics.projection.has_target_y_solution)
            .unwrap_or(diagnostics.route_dx_m);
        let gate = self.terminal.evaluate_transfer_gate(
            ctx,
            observation,
            lateral_dx_m,
            self.transfer_gate_ready_ticks,
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
        gate: TransferGateReadiness,
    ) -> bool {
        if gate.mode != TransferGateReadinessMode::LatestSafe {
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
            if future_gate.mode == TransferGateReadinessMode::NominalReady {
                return true;
            }
            let ratio_improvement = gate.required_accel_ratio - future_gate.required_accel_ratio;
            if future_gate.mode == TransferGateReadinessMode::LatestSafe
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
    ) -> TransferGateReadiness {
        let lateral_dx_m = diagnostics
            .projection
            .projected_dx_m
            .filter(|_| diagnostics.projection.has_target_y_solution)
            .unwrap_or(diagnostics.route_dx_m);
        let gate =
            self.terminal
                .evaluate_transfer_gate(ctx, observation, lateral_dx_m, ready_ticks);
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
        gate: TransferGateReadiness,
        _corridor: TransferCorridorState,
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

        if diagnostics.boost_quality.passed && self.should_coast(ctx, observation, diagnostics) {
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
        gate: TransferGateReadiness,
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
    ) -> bool {
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
        if corridor.tilt_limited {
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
        corridor: TransferCorridorState,
    ) -> TransferBoostCommandSelection {
        let base_attitude = self.boost_attitude_rad(observation, diagnostics, corridor);
        let eased_throttle =
            self.boost_throttle_frac(ctx, observation, diagnostics, corridor, base_attitude);
        let settled = self.boost_settled_quality(ctx, observation, diagnostics);

        let mut attitude_candidates = Vec::new();
        if base_attitude.abs() <= 1.0e-6 {
            self.push_unique_candidate(&mut attitude_candidates, 0.0);
        } else {
            self.push_unique_candidate(&mut attitude_candidates, base_attitude * 0.6);
            self.push_unique_candidate(&mut attitude_candidates, base_attitude);
        }
        let uphill_attitude = self.apply_corridor_tilt_cap(
            self.boost_lateral_direction(observation, diagnostics)
                * self.uphill_clearance_limited_boost_tilt_rad(observation, diagnostics),
            corridor,
        );
        self.push_unique_candidate(&mut attitude_candidates, uphill_attitude);

        let mut throttle_candidates = Vec::new();
        for throttle in [0.45, 0.70, 1.0, eased_throttle] {
            self.push_unique_candidate(
                &mut throttle_candidates,
                throttle.clamp(ctx.vehicle.min_throttle_frac, 1.0),
            );
        }
        if self.boost_projected_overshoot(observation, diagnostics) {
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
            selected_score: best_score.score,
            settled_projection: settled.projection,
            settled_quality: settled.quality,
        }
    }

    fn score_boost_candidate(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
        corridor: TransferCorridorState,
        command: Command,
    ) -> TransferBoostCandidateScore {
        if self.config.boost_recoverability_scoring_enabled {
            self.score_boost_candidate_recoverability(
                ctx,
                observation,
                diagnostics,
                corridor,
                command,
            )
        } else if self.config.boost_pathwise_scoring_enabled {
            self.score_boost_candidate_pathwise(ctx, observation, diagnostics, corridor, command)
        } else {
            self.score_boost_candidate_endpoint(ctx, observation, corridor, command)
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
        gate: TransferGateReadiness,
        dx_limit_m: f64,
    ) -> f64 {
        let mut score = 0.0;
        if !gate.terrain_clearance_safe {
            score += TRANSFER_BOOST_RECOVERY_SCORE_TERRAIN_UNSAFE;
            score += (-gate.terrain_min_clearance_m).max(0.0).min(200.0);
        }

        if predicted.height_above_target_m <= 0.0 {
            score += 600.0 + (-predicted.height_above_target_m).min(200.0);
        }

        let negative_margin_s = (-gate.latest_safe_margin_s).max(0.0).min(12.0);
        score += TRANSFER_BOOST_RECOVERY_SCORE_LATEST_SAFE_MARGIN
            * negative_margin_s
            * negative_margin_s;

        let accel_excess_ratio = (gate.required_accel_ratio - 1.0).max(0.0).min(12.0);
        score +=
            TRANSFER_BOOST_RECOVERY_SCORE_ACCEL_RATIO * accel_excess_ratio * accel_excess_ratio;

        if predicted_diagnostics.boost_quality.passed
            && gate.mode != TransferGateReadinessMode::NominalReady
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

    fn frame_for_open_loop_phase(
        &mut self,
        ctx: &RunContext,
        observation: &Observation,
        phase_name: TransferPhase,
        command: Command,
        status: &'static str,
        diagnostics: TransferDiagnostics,
        gate: TransferGateReadiness,
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
        if self.config.boost_recoverability_scoring_enabled {
            "transfer_pdg_recoverability_v1"
        } else if self.config.boost_pathwise_scoring_enabled {
            "transfer_pdg_pathwise_v1"
        } else {
            "transfer_pdg_v1"
        }
    }

    fn reset(&mut self, ctx: &RunContext) {
        self.phase = TransferPhase::Takeoff;
        self.boost_anchor = None;
        self.transfer_gate_ready_ticks = 0;
        self.last_transfer_gate = None;
        self.last_corridor = TransferCorridorState::inactive();
        self.last_phase = None;
        self.terminal.reset(ctx);
    }

    fn update(&mut self, ctx: &RunContext, observation: &Observation) -> ControllerFrame {
        let preliminary_diagnostics = self.transfer_diagnostics(observation);
        let gate = self.transfer_gate_readiness(ctx, observation, preliminary_diagnostics);
        let corridor = self.transfer_corridor_state(ctx, observation, preliminary_diagnostics);
        self.transfer_gate_ready_ticks = gate.ready_ticks;
        self.last_transfer_gate = Some(gate);
        self.last_corridor = corridor;

        let phase = self.choose_phase(ctx, observation, preliminary_diagnostics, gate, corridor);
        if phase == TransferPhase::Boost && self.boost_anchor.is_none() {
            self.boost_anchor = Some(TransferBoostAnchor {
                route_dx_m: observation.target_dx_m,
                route_dy_m: -observation.height_above_target_m,
            });
        }
        let diagnostics = self.transfer_diagnostics(observation);
        self.phase = phase;
        match phase {
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
                let selection = self.select_boost_command(ctx, observation, diagnostics, corridor);
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_pdg::TransferGateReadinessMode;
    use pd_core::{
        EvaluationGoal, LandingPadSpec, MissionSpec, RunContext, ScenarioSpec, SimConfig,
        TerrainDefinition, TransferRouteSpec, Vec2, VehicleGeometry, VehicleInitialState,
        VehicleSpec, WorldSpec,
    };
    use std::collections::BTreeMap;

    fn transfer_observation(
        target_dx_m: f64,
        height_above_target_m: f64,
        velocity_mps: Vec2,
        sim_time_s: f64,
    ) -> Observation {
        Observation {
            sim_time_s,
            physics_step: 0,
            position_m: Vec2::new(0.0, height_above_target_m),
            velocity_mps,
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
            mass_kg: 1_000.0,
            fuel_kg: 100.0,
            gravity_mps2: 9.81,
            target_dx_m,
            height_above_target_m,
            target_surface_y_m: 0.0,
            target_pad_half_width_m: 18.0,
            touchdown_clearance_m: height_above_target_m.abs() + 100.0,
            min_hull_clearance_m: height_above_target_m.abs() + 100.0,
        }
    }

    fn uphill_transfer_context() -> RunContext {
        let scenario = ScenarioSpec {
            id: "uphill_transfer".to_owned(),
            name: "Uphill transfer".to_owned(),
            description: "uphill transfer controller unit fixture".to_owned(),
            seed: 1,
            tags: vec!["test".to_owned()],
            metadata: BTreeMap::new(),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 90.0,
                sample_hz: Some(10),
            },
            world: WorldSpec {
                gravity_mps2: 9.81,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![
                        Vec2::new(-180.0, -780.0),
                        Vec2::new(-140.0, -780.0),
                        Vec2::new(0.0, 0.0),
                        Vec2::new(180.0, 0.0),
                    ],
                },
                landing_pads: vec![
                    LandingPadSpec {
                        id: "source".to_owned(),
                        center_x_m: -140.0,
                        surface_y_m: -780.0,
                        width_m: 36.0,
                    },
                    LandingPadSpec {
                        id: "target".to_owned(),
                        center_x_m: 0.0,
                        surface_y_m: 0.0,
                        width_m: 36.0,
                    },
                ],
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
                position_m: Vec2::new(-140.0, -780.0),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                transfer_route: Some(TransferRouteSpec {
                    source_pad_id: "source".to_owned(),
                    target_pad_id: "target".to_owned(),
                    route_angle_deg: 80.0,
                    route_radius_m: 800.0,
                    waypoints: Vec::new(),
                }),
                goal: EvaluationGoal::LandingOnPad {
                    target_pad_id: "target".to_owned(),
                },
            },
        };
        RunContext::from_scenario(&scenario).unwrap()
    }

    #[test]
    fn transfer_projection_reports_descending_target_crossing() {
        let projection = transfer_ballistic_projection(100.0, -50.0, 10.0, 20.0, 10.0);

        assert!(projection.has_target_y_solution);
        assert!((projection.projected_time_s.unwrap() - 5.7416573868).abs() < 1.0e-6);
        assert!((projection.projected_dx_m.unwrap() - 42.583426132).abs() < 1.0e-6);
        assert!(projection.impact_angle_deg.unwrap() > 70.0);
        assert!((projection.apex_over_target_m - 70.0).abs() < 1.0e-9);
    }

    #[test]
    fn transfer_projection_rejects_unreachable_target_y() {
        let projection = transfer_ballistic_projection(100.0, 100.0, 10.0, 20.0, 10.0);

        assert!(!projection.has_target_y_solution);
        assert_eq!(projection.projected_time_s, None);
        assert_eq!(projection.projected_dx_m, None);
        assert_eq!(projection.impact_angle_deg, None);
        assert!((projection.apex_over_target_m + 80.0).abs() < 1.0e-9);
    }

    #[test]
    fn transfer_boost_quality_uses_projection_and_angle() {
        let controller = TransferPdgController::default();
        let good_projection = transfer_ballistic_projection(100.0, -50.0, 10.0, 20.0, 10.0);
        let good = controller.transfer_boost_quality(100.0, -50.0, good_projection);
        assert_eq!(good.verdict, "pass");
        assert!(good.passed);

        let no_solution = transfer_ballistic_projection(100.0, 100.0, 10.0, 20.0, 10.0);
        let no_solution_quality = controller.transfer_boost_quality(100.0, 100.0, no_solution);
        assert_eq!(no_solution_quality.verdict, "no_target_y_solution");
        assert!(!no_solution_quality.passed);

        let dx_miss = transfer_ballistic_projection(500.0, -50.0, 0.0, 20.0, 10.0);
        let dx_quality = controller.transfer_boost_quality(500.0, -50.0, dx_miss);
        assert_eq!(dx_quality.verdict, "dx");
        assert!(!dx_quality.passed);
    }

    #[test]
    fn transfer_boost_continues_until_ballistic_quality_passes() {
        let ctx = uphill_transfer_context();
        let controller = TransferPdgController::default();
        let passing_observation = transfer_observation(100.0, 50.0, Vec2::new(10.0, 20.0), 6.0);
        let passing_diagnostics = controller.transfer_diagnostics(&passing_observation);
        assert!(passing_diagnostics.boost_quality.passed);
        assert!(!controller.boost_should_continue(&ctx, &passing_observation, passing_diagnostics));

        let missing_observation = transfer_observation(500.0, -100.0, Vec2::new(15.0, 5.0), 6.0);
        let missing_diagnostics = controller.transfer_diagnostics(&missing_observation);
        assert!(!missing_diagnostics.boost_quality.passed);
        assert!(controller.boost_should_continue(&ctx, &missing_observation, missing_diagnostics));
    }

    #[test]
    fn transfer_uphill_boost_uses_vertical_bias_until_apex_is_safe() {
        let controller = TransferPdgController::default();
        let observation = transfer_observation(180.0, -360.0, Vec2::new(30.0, 10.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        assert!(diagnostics.route_dy_m > controller.config.uphill_boost_dy_min_m);
        assert!(
            !diagnostics.projection.has_target_y_solution
                || diagnostics.projection.apex_over_target_m
                    < diagnostics.boost_quality.apex_target_over_target_m
        );
        assert_eq!(
            controller.boost_attitude_rad(
                &observation,
                diagnostics,
                TransferCorridorState::inactive()
            ),
            controller.config.uphill_boost_tilt_rad
        );
    }

    #[test]
    fn transfer_boost_quality_uses_frozen_shape_anchor() {
        let mut controller = TransferPdgController::default();
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 800.0,
            route_dy_m: 300.0,
        });
        let observation = transfer_observation(100.0, 50.0, Vec2::new(10.0, 20.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        assert_eq!(
            diagnostics.boost_quality.apex_target_over_target_m,
            controller.boost_apex_target_over_target_m(800.0, 300.0)
        );
        assert_ne!(
            diagnostics.boost_quality.apex_target_over_target_m,
            controller.boost_apex_target_over_target_m(100.0, -50.0)
        );
    }

    #[test]
    fn transfer_steep_uphill_boost_stays_vertical_when_clearance_is_low() {
        let controller = TransferPdgController::default();
        let mut observation = transfer_observation(140.0, -780.0, Vec2::new(10.0, 15.0), 4.0);
        observation.touchdown_clearance_m = 35.0;
        let diagnostics = controller.transfer_diagnostics(&observation);

        let attitude_rad = controller.boost_attitude_rad(
            &observation,
            diagnostics,
            TransferCorridorState::inactive(),
        );

        assert!(attitude_rad > 0.0);
        assert!(attitude_rad < controller.config.uphill_boost_tilt_rad);
    }

    #[test]
    fn transfer_boost_uses_projected_miss_direction_when_target_y_is_reachable() {
        let controller = TransferPdgController::default();
        let observation = transfer_observation(100.0, -50.0, Vec2::new(50.0, 50.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        assert!(diagnostics.projection.projected_dx_m.unwrap() < 0.0);
        assert!(
            controller.boost_attitude_rad(
                &observation,
                diagnostics,
                TransferCorridorState::inactive()
            ) < 0.0
        );
    }

    #[test]
    fn transfer_boost_keeps_anchor_direction_until_target_y_is_reachable() {
        let mut controller = TransferPdgController::default();
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 700.0,
            route_dy_m: 400.0,
        });
        let observation = transfer_observation(500.0, -100.0, Vec2::new(15.0, 5.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        assert!(!diagnostics.projection.has_target_y_solution);
        assert!(
            controller.boost_attitude_rad(
                &observation,
                diagnostics,
                TransferCorridorState::inactive()
            ) > 0.0
        );
    }

    #[test]
    fn transfer_boost_corrects_projected_overshoot_after_target_y_is_reachable() {
        let mut controller = TransferPdgController::default();
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 700.0,
            route_dy_m: 400.0,
        });
        let observation = transfer_observation(100.0, -50.0, Vec2::new(50.0, 50.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        assert!(diagnostics.projection.has_target_y_solution);
        assert!(
            diagnostics.projection.projected_dx_m.unwrap()
                < -controller.boost_dx_limit_m(&observation)
        );
        assert!(
            controller.boost_attitude_rad(
                &observation,
                diagnostics,
                TransferCorridorState::inactive()
            ) < 0.0
        );
        assert!(controller.boost_projected_overshoot(&observation, diagnostics));
    }

    #[test]
    fn transfer_coast_accepts_settled_ascending_solution_above_target_height() {
        let ctx = uphill_transfer_context();
        let mut controller = TransferPdgController::default();
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 700.0,
            route_dy_m: 400.0,
        });
        let observation = transfer_observation(300.0, 200.0, Vec2::new(25.0, 45.0), 12.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        assert!(diagnostics.boost_quality.passed);
        assert!(
            controller
                .boost_settled_quality(&ctx, &observation, diagnostics)
                .quality
                .passed
        );
        assert!(controller.should_coast(&ctx, &observation, diagnostics));
    }

    #[test]
    fn transfer_coast_prealigns_upright_retrograde() {
        let controller = TransferPdgController::default();
        let observation = transfer_observation(400.0, 300.0, Vec2::new(45.0, -8.0), 10.0);

        let attitude_rad = controller.coast_attitude_rad(&observation);

        assert!(attitude_rad < -0.5);
        assert!(attitude_rad.abs() <= controller.config.terminal.terminal_overshoot_tilt_max_rad);
    }

    #[test]
    fn transfer_coast_avoids_max_retrograde_tilt_while_climbing() {
        let controller = TransferPdgController::default();
        let observation = transfer_observation(400.0, 300.0, Vec2::new(26.0, 35.0), 10.0);

        let attitude_rad = controller.coast_attitude_rad(&observation);

        assert!(attitude_rad < 0.0);
        assert!(attitude_rad.abs() < 0.8);
    }

    #[test]
    fn transfer_reset_clears_boost_anchor_and_gate_state() {
        let ctx = uphill_transfer_context();
        let mut controller = TransferPdgController::default();
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 140.0,
            route_dy_m: 780.0,
        });
        controller.transfer_gate_ready_ticks = 3;
        controller.last_transfer_gate = Some(TransferGateReadiness {
            mode: TransferGateReadinessMode::NominalReady,
            ready_ticks: 3,
            burn_time_s: 5.0,
            latest_safe_margin_s: 2.0,
            required_accel_ratio: 0.4,
            terrain_min_clearance_m: 20.0,
            terrain_clearance_safe: true,
            deferred: false,
        });
        controller.last_corridor = TransferCorridorState {
            mode: "active",
            active: true,
            tilt_limited: true,
            margin_m: -10.0,
        };

        controller.reset(&ctx);

        assert!(controller.boost_anchor.is_none());
        assert_eq!(controller.transfer_gate_ready_ticks, 0);
        assert_eq!(controller.last_transfer_gate, None);
        assert_eq!(controller.last_corridor, TransferCorridorState::inactive());
    }

    #[test]
    fn transfer_default_extends_terminal_gate_horizon_without_changing_terminal_default() {
        let terminal = TerminalPdgControllerConfig::default();
        let transfer = TransferPdgControllerConfig::default();

        assert_eq!(terminal.terminal_gate_burn_time_max_s, 14.0);
        assert_eq!(terminal.terminal_gate_burn_time_offset_long_s, 0.8);
        assert_eq!(transfer.terminal.terminal_gate_burn_time_max_s, 22.0);
        assert_eq!(transfer.terminal.terminal_gate_burn_time_offset_long_s, 2.0);
    }

    #[test]
    fn transfer_gate_forces_pending_without_target_y_solution() {
        let ctx = uphill_transfer_context();
        let controller = TransferPdgController::default();
        let mut observation = transfer_observation(140.0, -780.0, Vec2::new(10.0, 15.0), 6.0);
        observation.position_m = Vec2::new(-140.0, -780.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        let gate = controller.transfer_gate_readiness(&ctx, &observation, diagnostics);

        assert!(!diagnostics.projection.has_target_y_solution);
        assert_eq!(gate.mode, TransferGateReadinessMode::Pending);
        assert_eq!(gate.ready_ticks, 0);
    }

    #[test]
    fn transfer_uphill_corridor_caps_boost_tilt() {
        let ctx = uphill_transfer_context();
        let controller = TransferPdgController::default();
        let mut observation = transfer_observation(140.0, -780.0, Vec2::new(10.0, 15.0), 6.0);
        observation.position_m = Vec2::new(-140.0, -780.0);
        let diagnostics = controller.transfer_diagnostics(&observation);
        let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

        let attitude_rad = controller.boost_attitude_rad(&observation, diagnostics, corridor);

        assert!(corridor.active);
        assert!(corridor.tilt_limited);
        assert!(corridor.margin_m < 0.0);
        assert!(attitude_rad.abs() <= TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD);
        assert!(attitude_rad.abs() < controller.config.uphill_boost_tilt_rad);
    }

    #[test]
    fn transfer_moderate_uphill_corridor_does_not_cap_boost_tilt() {
        let mut ctx = uphill_transfer_context();
        ctx.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-740.0, -400.0),
                Vec2::new(-700.0, -400.0),
                Vec2::new(-18.0, 0.0),
                Vec2::new(18.0, 0.0),
                Vec2::new(180.0, 0.0),
            ],
        };
        let controller = TransferPdgController::default();
        let mut observation = transfer_observation(700.0, -400.0, Vec2::new(10.0, 15.0), 6.0);
        observation.position_m = Vec2::new(-700.0, -400.0);
        let diagnostics = controller.transfer_diagnostics(&observation);
        let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

        assert!(corridor.active);
        assert!(!corridor.tilt_limited);
    }

    #[test]
    fn transfer_boost_throttle_eases_when_apex_is_already_high() {
        let ctx = uphill_transfer_context();
        let controller = TransferPdgController::default();
        let observation = transfer_observation(700.0, -200.0, Vec2::new(10.0, 100.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        let throttle = controller.boost_throttle_frac(
            &ctx,
            &observation,
            diagnostics,
            TransferCorridorState::inactive(),
            0.3,
        );

        assert!(
            diagnostics.projection.apex_over_target_m
                > diagnostics.boost_quality.apex_target_over_target_m
        );
        assert!(throttle < 1.0);
        assert!(throttle >= ctx.vehicle.min_throttle_frac);
    }

    #[test]
    fn transfer_boost_scorer_reduces_throttle_when_apex_is_high() {
        let ctx = uphill_transfer_context();
        let mut controller = TransferPdgController::default();
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 100.0,
            route_dy_m: -50.0,
        });
        let observation = transfer_observation(100.0, 50.0, Vec2::new(10.0, 100.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        let selection = controller.select_boost_command(
            &ctx,
            &observation,
            diagnostics,
            TransferCorridorState::inactive(),
        );

        assert!(selection.selected_score.is_finite());
        assert!(selection.command.throttle_frac < 1.0);
        assert!(selection.command.throttle_frac >= ctx.vehicle.min_throttle_frac);
    }

    #[test]
    fn transfer_pathwise_alias_enables_pathwise_boost_scoring() {
        let spec = built_in_controller_spec("transfer_pdg_pathwise")
            .expect("pathwise transfer controller alias should exist");

        match spec {
            ControllerSpec::TransferPdgV1 { config } => {
                assert!(config.boost_pathwise_scoring_enabled);
                assert_eq!(
                    ControllerSpec::TransferPdgV1 { config }.id(),
                    "transfer_pdg_pathwise_v1"
                );
            }
            _ => panic!("pathwise alias should resolve to transfer controller"),
        }
    }

    #[test]
    fn transfer_recoverability_alias_enables_recoverability_boost_scoring() {
        let spec = built_in_controller_spec("transfer_pdg_recoverability")
            .expect("recoverability transfer controller alias should exist");

        match spec {
            ControllerSpec::TransferPdgV1 { config } => {
                assert!(config.boost_recoverability_scoring_enabled);
                assert!(!config.boost_pathwise_scoring_enabled);
                assert_eq!(
                    ControllerSpec::TransferPdgV1 { config }.id(),
                    "transfer_pdg_recoverability_v1"
                );
            }
            _ => panic!("recoverability alias should resolve to transfer controller"),
        }
    }

    #[test]
    fn transfer_pathwise_scorer_keeps_targetward_tilt_for_shortfall() {
        let ctx = uphill_transfer_context();
        let mut config = TransferPdgControllerConfig::default();
        config.boost_pathwise_scoring_enabled = true;
        let mut controller = TransferPdgController::new(config);
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 500.0,
            route_dy_m: 120.0,
        });
        let observation = transfer_observation(500.0, -120.0, Vec2::new(5.0, 30.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        let selection = controller.select_boost_command(
            &ctx,
            &observation,
            diagnostics,
            TransferCorridorState::inactive(),
        );

        assert_eq!(controller.boost_scoring_mode(), "pathwise_geometry");
        assert!(selection.selected_score.is_finite());
        assert!(selection.command.target_attitude_rad > 0.0);
        assert!(selection.command.throttle_frac >= 0.7);
    }

    #[test]
    fn transfer_pathwise_scorer_penalizes_away_thrust_outside_corridor() {
        let mut config = TransferPdgControllerConfig::default();
        config.boost_pathwise_scoring_enabled = true;
        let mut controller = TransferPdgController::new(config);
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 500.0,
            route_dy_m: 120.0,
        });
        let observation = transfer_observation(500.0, -120.0, Vec2::new(5.0, 20.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);
        let dx_limit_m = controller.boost_dx_limit_m(&observation);

        let away = controller.score_boost_no_away_penalty(
            &observation,
            diagnostics,
            Command {
                throttle_frac: 1.0,
                target_attitude_rad: -0.3,
            },
            dx_limit_m,
        );
        let targetward = controller.score_boost_no_away_penalty(
            &observation,
            diagnostics,
            Command {
                throttle_frac: 1.0,
                target_attitude_rad: 0.3,
            },
            dx_limit_m,
        );

        assert!(away > 0.0);
        assert_eq!(targetward, 0.0);
    }

    #[test]
    fn transfer_pathwise_scorer_is_finite_without_target_y_solution() {
        let ctx = uphill_transfer_context();
        let mut config = TransferPdgControllerConfig::default();
        config.boost_pathwise_scoring_enabled = true;
        let controller = TransferPdgController::new(config);
        let observation = transfer_observation(500.0, -220.0, Vec2::new(5.0, 5.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        assert!(!diagnostics.projection.has_target_y_solution);
        let score = controller.score_boost_candidate(
            &ctx,
            &observation,
            diagnostics,
            TransferCorridorState::inactive(),
            Command {
                throttle_frac: 1.0,
                target_attitude_rad: 0.3,
            },
        );

        assert!(score.score.is_finite());
        assert!(!score.quality.passed);
    }

    #[test]
    fn transfer_recoverability_scorer_penalizes_overdue_terminal_gate() {
        let controller = TransferPdgController::default();
        let observation = transfer_observation(220.0, 80.0, Vec2::new(35.0, -18.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);
        let ready = TransferGateReadiness {
            mode: TransferGateReadinessMode::NominalReady,
            ready_ticks: 2,
            burn_time_s: 2.0,
            latest_safe_margin_s: 0.5,
            required_accel_ratio: 0.8,
            terrain_min_clearance_m: 80.0,
            terrain_clearance_safe: true,
            deferred: false,
        };
        let overdue = TransferGateReadiness {
            mode: TransferGateReadinessMode::LatestSafe,
            ready_ticks: 0,
            burn_time_s: 2.0,
            latest_safe_margin_s: -4.0,
            required_accel_ratio: 0.8,
            terrain_min_clearance_m: 80.0,
            terrain_clearance_safe: true,
            deferred: false,
        };

        let ready_score = controller.score_boost_candidate_recoverability_terms(
            &observation,
            diagnostics,
            ready,
            controller.boost_dx_limit_m(&observation),
        );
        let overdue_score = controller.score_boost_candidate_recoverability_terms(
            &observation,
            diagnostics,
            overdue,
            controller.boost_dx_limit_m(&observation),
        );

        assert!(overdue_score > ready_score + 100.0);
    }

    #[test]
    fn transfer_recoverability_scorer_prefers_margin_over_lower_accel_ratio() {
        let controller = TransferPdgController::default();
        let observation = transfer_observation(220.0, 80.0, Vec2::new(35.0, -18.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);
        let better_margin = TransferGateReadiness {
            mode: TransferGateReadinessMode::LatestSafe,
            ready_ticks: 0,
            burn_time_s: 2.0,
            latest_safe_margin_s: -2.0,
            required_accel_ratio: 8.0,
            terrain_min_clearance_m: 80.0,
            terrain_clearance_safe: true,
            deferred: false,
        };
        let lower_accel = TransferGateReadiness {
            mode: TransferGateReadinessMode::LatestSafe,
            ready_ticks: 0,
            burn_time_s: 2.0,
            latest_safe_margin_s: -4.0,
            required_accel_ratio: 2.0,
            terrain_min_clearance_m: 80.0,
            terrain_clearance_safe: true,
            deferred: false,
        };

        let better_margin_score = controller.score_boost_candidate_recoverability_terms(
            &observation,
            diagnostics,
            better_margin,
            controller.boost_dx_limit_m(&observation),
        );
        let lower_accel_score = controller.score_boost_candidate_recoverability_terms(
            &observation,
            diagnostics,
            lower_accel,
            controller.boost_dx_limit_m(&observation),
        );

        assert!(better_margin_score < lower_accel_score);
    }

    #[test]
    fn transfer_recoverability_scorer_is_finite_without_target_y_solution() {
        let ctx = uphill_transfer_context();
        let mut config = TransferPdgControllerConfig::default();
        config.boost_recoverability_scoring_enabled = true;
        let controller = TransferPdgController::new(config);
        let observation = transfer_observation(500.0, -220.0, Vec2::new(5.0, 5.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        assert_eq!(controller.boost_scoring_mode(), "recoverability");
        assert!(!diagnostics.projection.has_target_y_solution);
        let score = controller.score_boost_candidate(
            &ctx,
            &observation,
            diagnostics,
            TransferCorridorState::inactive(),
            Command {
                throttle_frac: 1.0,
                target_attitude_rad: 0.3,
            },
        );

        assert!(score.score.is_finite());
        assert!(!score.quality.passed);
    }

    #[test]
    fn transfer_boost_scorer_keeps_targetward_tilt_for_shortfall() {
        let ctx = uphill_transfer_context();
        let mut controller = TransferPdgController::default();
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 500.0,
            route_dy_m: 120.0,
        });
        let observation = transfer_observation(500.0, -120.0, Vec2::new(5.0, 30.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        let selection = controller.select_boost_command(
            &ctx,
            &observation,
            diagnostics,
            TransferCorridorState::inactive(),
        );

        assert!(selection.command.target_attitude_rad > 0.0);
        assert!(selection.command.throttle_frac >= 0.7);
    }

    #[test]
    fn transfer_boost_scorer_respects_corridor_tilt_cap() {
        let ctx = uphill_transfer_context();
        let mut controller = TransferPdgController::default();
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 140.0,
            route_dy_m: 780.0,
        });
        let mut observation = transfer_observation(140.0, -780.0, Vec2::new(10.0, 15.0), 6.0);
        observation.position_m = Vec2::new(-140.0, -780.0);
        let diagnostics = controller.transfer_diagnostics(&observation);
        let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

        let selection = controller.select_boost_command(&ctx, &observation, diagnostics, corridor);

        assert!(corridor.tilt_limited);
        assert!(
            selection.command.target_attitude_rad.abs() <= TRANSFER_UPHILL_CORRIDOR_TILT_CAP_RAD
        );
    }

    #[test]
    fn transfer_boost_settled_quality_keeps_passive_projection_stable() {
        let ctx = uphill_transfer_context();
        let mut controller = TransferPdgController::default();
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 100.0,
            route_dy_m: -50.0,
        });
        let observation = transfer_observation(100.0, 50.0, Vec2::new(10.0, 20.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        let settled = controller.boost_settled_quality(&ctx, &observation, diagnostics);

        assert!(diagnostics.boost_quality.passed);
        assert!(settled.quality.passed);
        assert!(
            (settled.projection.projected_dx_m.unwrap()
                - diagnostics.projection.projected_dx_m.unwrap())
            .abs()
                < 2.0
        );
    }

    #[test]
    fn transfer_latest_safe_deferral_respects_guard_conditions() {
        let ctx = uphill_transfer_context();
        let controller = TransferPdgController::default();
        let gate = TransferGateReadiness {
            mode: TransferGateReadinessMode::LatestSafe,
            ready_ticks: 0,
            burn_time_s: 5.0,
            latest_safe_margin_s: 0.0,
            required_accel_ratio: 0.8,
            terrain_min_clearance_m: 20.0,
            terrain_clearance_safe: true,
            deferred: false,
        };
        let descending = transfer_observation(100.0, 80.0, Vec2::new(8.0, -2.0), 6.0);
        let descending_diagnostics = controller.transfer_diagnostics(&descending);
        assert!(!controller.should_defer_latest_safe_transfer_gate(
            &ctx,
            &descending,
            descending_diagnostics,
            gate
        ));

        let out_of_band = transfer_observation(500.0, 80.0, Vec2::new(5.0, 12.0), 6.0);
        let out_of_band_diagnostics = controller.transfer_diagnostics(&out_of_band);
        assert!(!controller.should_defer_latest_safe_transfer_gate(
            &ctx,
            &out_of_band,
            out_of_band_diagnostics,
            gate
        ));
    }

    #[test]
    fn transfer_uphill_corridor_releases_after_local_clearance() {
        let ctx = uphill_transfer_context();
        let controller = TransferPdgController::default();
        let mut observation = transfer_observation(140.0, -450.0, Vec2::new(10.0, 15.0), 6.0);
        observation.position_m = Vec2::new(-140.0, -450.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

        assert_eq!(corridor.mode, "clear");
        assert!(!corridor.active);
        assert!(corridor.margin_m > 0.0);
    }

    #[test]
    fn transfer_short_downhill_route_stays_terminal_owned() {
        let ctx = uphill_transfer_context();
        let controller = TransferPdgController::default();
        let mut observation = transfer_observation(140.0, 320.0, Vec2::new(0.0, -4.0), 6.0);
        observation.position_m = Vec2::new(-140.0, 320.0);
        let diagnostics = controller.transfer_diagnostics(&observation);
        let gate = controller.transfer_gate_readiness(&ctx, &observation, diagnostics);
        let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

        let phase = controller.choose_phase(&ctx, &observation, diagnostics, gate, corridor);

        assert_eq!(phase, TransferPhase::Terminal);
    }

    #[test]
    fn transfer_short_downhill_route_holds_takeoff_until_source_clearance_safe() {
        let mut ctx = uphill_transfer_context();
        ctx.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-120.0, 400.0),
                Vec2::new(-70.0, 400.0),
                Vec2::new(-51.0, 400.0),
                Vec2::new(-18.0, 0.0),
                Vec2::new(80.0, 0.0),
            ],
        };
        ctx.world.landing_pads = vec![
            LandingPadSpec {
                id: "source".to_owned(),
                center_x_m: -70.0,
                surface_y_m: 400.0,
                width_m: 36.0,
            },
            LandingPadSpec {
                id: "target".to_owned(),
                center_x_m: 0.0,
                surface_y_m: 0.0,
                width_m: 36.0,
            },
        ];
        ctx.target_pad = ctx.world.landing_pad("target").unwrap().clone();
        let route = ctx.mission.transfer_route.as_mut().unwrap();
        route.route_angle_deg = -80.0;
        route.route_radius_m = 400.0;

        let controller = TransferPdgController::default();
        let mut observation = transfer_observation(70.0, 405.0, Vec2::new(0.0, 8.2), 2.4);
        observation.position_m = Vec2::new(-70.0, 405.0);
        observation.touchdown_clearance_m = 5.0;
        let diagnostics = controller.transfer_diagnostics(&observation);
        let gate = controller.transfer_gate_readiness(&ctx, &observation, diagnostics);
        let corridor = controller.transfer_corridor_state(&ctx, &observation, diagnostics);

        let held_phase = controller.choose_phase(&ctx, &observation, diagnostics, gate, corridor);

        observation.position_m.y = 430.0;
        observation.height_above_target_m = 430.0;
        observation.touchdown_clearance_m = 30.0;
        let safe_diagnostics = controller.transfer_diagnostics(&observation);
        let safe_gate = controller.transfer_gate_readiness(&ctx, &observation, safe_diagnostics);
        let safe_corridor =
            controller.transfer_corridor_state(&ctx, &observation, safe_diagnostics);
        let released_phase = controller.choose_phase(
            &ctx,
            &observation,
            safe_diagnostics,
            safe_gate,
            safe_corridor,
        );

        assert_eq!(held_phase, TransferPhase::Takeoff);
        assert_eq!(released_phase, TransferPhase::Terminal);
    }

    #[test]
    fn transfer_coast_captures_terminal_before_uphill_target_crossing() {
        let ctx = uphill_transfer_context();
        let mut controller = TransferPdgController::default();
        controller.phase = TransferPhase::Coast;
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 565.0,
            route_dy_m: 560.0,
        });
        let observation = transfer_observation(294.0, -60.0, Vec2::new(37.4, 50.8), 14.5);
        let diagnostics = controller.transfer_diagnostics(&observation);
        let gate = controller.transfer_gate_readiness(&ctx, &observation, diagnostics);

        assert_eq!(gate.mode, TransferGateReadinessMode::Pending);
        assert!(diagnostics.boost_quality.passed);
        assert!(
            controller
                .next_target_y_crossing_time_s(&observation)
                .unwrap()
                <= TRANSFER_PRE_TARGET_CAPTURE_LOOKAHEAD_S
        );
        assert!(controller.pre_target_terminal_capture_ready(&observation, diagnostics, gate));

        let phase = controller.choose_phase(
            &ctx,
            &observation,
            diagnostics,
            gate,
            TransferCorridorState::inactive(),
        );

        assert_eq!(phase, TransferPhase::Terminal);
    }

    #[test]
    fn transfer_coast_waits_when_uphill_target_crossing_is_not_imminent() {
        let ctx = uphill_transfer_context();
        let mut controller = TransferPdgController::default();
        controller.phase = TransferPhase::Coast;
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 565.0,
            route_dy_m: 560.0,
        });
        let observation = transfer_observation(410.0, -120.0, Vec2::new(37.4, 50.8), 13.0);
        let diagnostics = controller.transfer_diagnostics(&observation);
        let gate = controller.transfer_gate_readiness(&ctx, &observation, diagnostics);

        assert!(
            controller
                .next_target_y_crossing_time_s(&observation)
                .unwrap()
                > TRANSFER_PRE_TARGET_CAPTURE_LOOKAHEAD_S
        );
        assert!(!controller.pre_target_terminal_capture_ready(&observation, diagnostics, gate));
    }

    #[test]
    fn transfer_started_route_waits_for_ready_gate_before_terminal() {
        let ctx = uphill_transfer_context();
        let mut controller = TransferPdgController::default();
        controller.phase = TransferPhase::Boost;
        controller.boost_anchor = Some(TransferBoostAnchor {
            route_dx_m: 700.0,
            route_dy_m: 400.0,
        });
        let observation = transfer_observation(120.0, -20.0, Vec2::new(20.0, 5.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);
        let gate = TransferGateReadiness {
            mode: TransferGateReadinessMode::Pending,
            ready_ticks: 0,
            burn_time_s: 5.0,
            latest_safe_margin_s: 2.0,
            required_accel_ratio: 0.8,
            terrain_min_clearance_m: 20.0,
            terrain_clearance_safe: true,
            deferred: false,
        };

        let phase = controller.choose_phase(
            &ctx,
            &observation,
            diagnostics,
            gate,
            TransferCorridorState::inactive(),
        );

        assert_ne!(phase, TransferPhase::Terminal);
    }
}
