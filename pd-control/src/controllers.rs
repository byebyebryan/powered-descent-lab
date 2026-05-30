use crate::kit::{ControllerFrameBuilder, ControllerView, metric, phase, standard_marker};
use crate::terminal_pdg::{TerminalPdgController, TerminalPdgControllerConfig};
use crate::{Controller, ControllerFrame, TelemetryValue};
use pd_core::{Command, Observation, RunContext};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const TRANSFER_UPHILL_STEEP_TILT_SCALE: f64 = 0.25;
const TRANSFER_UPHILL_STEEP_TILT_MIN_RAD: f64 = 0.0;
const TRANSFER_UPHILL_LOW_CLEARANCE_M: f64 = 240.0;
const TRANSFER_UPHILL_CLEARANCE_BLEND_FLOOR_M: f64 = 20.0;

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
    pub coast_min_altitude_m: f64,
    pub terminal_gate_dx_m: f64,
    pub terminal_gate_altitude_m: f64,
    #[serde(default)]
    pub terminal: TerminalPdgControllerConfig,
}

impl Default for TransferPdgControllerConfig {
    fn default() -> Self {
        Self {
            takeoff_clearance_m: 45.0,
            takeoff_min_vertical_speed_mps: 8.0,
            max_takeoff_time_s: 5.0,
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
            coast_min_altitude_m: 80.0,
            terminal_gate_dx_m: 260.0,
            terminal_gate_altitude_m: 260.0,
            terminal: TerminalPdgControllerConfig::default(),
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

#[derive(Debug)]
pub struct TransferPdgController {
    config: TransferPdgControllerConfig,
    terminal: TerminalPdgController,
    phase: TransferPhase,
    boost_anchor: Option<TransferBoostAnchor>,
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

    fn transfer_metrics_builder(
        &self,
        builder: ControllerFrameBuilder,
        diagnostics: TransferDiagnostics,
    ) -> ControllerFrameBuilder {
        builder
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
    }

    fn insert_transfer_metrics(
        &self,
        frame: &mut ControllerFrame,
        diagnostics: TransferDiagnostics,
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
    }

    fn choose_phase(
        &self,
        ctx: &RunContext,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
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

        if self.phase == TransferPhase::Terminal {
            return TransferPhase::Terminal;
        }

        let needs_transfer_burn = observation.target_dx_m.abs() > self.config.terminal_gate_dx_m
            || diagnostics.route_dy_m > self.config.uphill_boost_dy_min_m;
        if self.phase != TransferPhase::Coast
            && needs_transfer_burn
            && self.boost_should_continue(observation, diagnostics)
        {
            return TransferPhase::Boost;
        }

        if self.should_coast(observation, diagnostics) {
            return TransferPhase::Coast;
        }

        TransferPhase::Terminal
    }

    fn boost_should_continue(
        &self,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
    ) -> bool {
        if observation.sim_time_s >= self.config.boost_max_time_s {
            return false;
        }
        if diagnostics.boost_quality.passed {
            return false;
        }

        true
    }

    fn should_coast(&self, observation: &Observation, diagnostics: TransferDiagnostics) -> bool {
        let clear_to_coast = observation.touchdown_clearance_m > self.config.coast_min_altitude_m;
        if !clear_to_coast {
            return false;
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
        direction * tilt_rad
    }

    fn boost_lateral_direction(
        &self,
        observation: &Observation,
        diagnostics: TransferDiagnostics,
    ) -> f64 {
        diagnostics
            .projection
            .projected_dx_m
            .filter(|projected_dx_m| projected_dx_m.abs() > observation.target_pad_half_width_m)
            .map_or_else(|| observation.target_dx_m.signum(), f64::signum)
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

    fn frame_for_open_loop_phase(
        &mut self,
        ctx: &RunContext,
        observation: &Observation,
        phase_name: TransferPhase,
        command: Command,
        status: &'static str,
        diagnostics: TransferDiagnostics,
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
        let frame = self.transfer_metrics_builder(builder, diagnostics).build();
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

impl Controller for TransferPdgController {
    fn id(&self) -> &str {
        "transfer_pdg_v1"
    }

    fn reset(&mut self, ctx: &RunContext) {
        self.phase = TransferPhase::Takeoff;
        self.last_phase = None;
        self.terminal.reset(ctx);
    }

    fn update(&mut self, ctx: &RunContext, observation: &Observation) -> ControllerFrame {
        let preliminary_diagnostics = self.transfer_diagnostics(observation);
        let phase = self.choose_phase(ctx, observation, preliminary_diagnostics);
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
            ),
            TransferPhase::Boost => self.frame_for_open_loop_phase(
                ctx,
                observation,
                phase,
                Command {
                    throttle_frac: 1.0,
                    target_attitude_rad: self.boost_attitude_rad(observation, diagnostics),
                },
                "boosting toward terminal gate",
                diagnostics,
            ),
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
            ),
            TransferPhase::Terminal => {
                let mut frame = self.terminal.update(ctx, observation);
                self.insert_transfer_metrics(&mut frame, diagnostics);
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
    use pd_core::Vec2;

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
        let controller = TransferPdgController::default();
        let passing_observation = transfer_observation(100.0, 50.0, Vec2::new(10.0, 20.0), 6.0);
        let passing_diagnostics = controller.transfer_diagnostics(&passing_observation);
        assert!(passing_diagnostics.boost_quality.passed);
        assert!(!controller.boost_should_continue(&passing_observation, passing_diagnostics));

        let missing_observation = transfer_observation(500.0, -100.0, Vec2::new(15.0, 5.0), 6.0);
        let missing_diagnostics = controller.transfer_diagnostics(&missing_observation);
        assert!(!missing_diagnostics.boost_quality.passed);
        assert!(controller.boost_should_continue(&missing_observation, missing_diagnostics));
    }

    #[test]
    fn transfer_uphill_boost_uses_vertical_bias_until_apex_is_safe() {
        let controller = TransferPdgController::default();
        let observation = transfer_observation(400.0, -250.0, Vec2::new(30.0, 10.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        assert!(diagnostics.route_dy_m > controller.config.uphill_boost_dy_min_m);
        assert!(
            !diagnostics.projection.has_target_y_solution
                || diagnostics.projection.apex_over_target_m
                    < diagnostics.boost_quality.apex_target_over_target_m
        );
        assert_eq!(
            controller.boost_attitude_rad(&observation, diagnostics),
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

        let attitude_rad = controller.boost_attitude_rad(&observation, diagnostics);

        assert!(attitude_rad > 0.0);
        assert!(attitude_rad < controller.config.uphill_boost_tilt_rad);
    }

    #[test]
    fn transfer_boost_uses_projected_miss_direction_when_target_y_is_reachable() {
        let controller = TransferPdgController::default();
        let observation = transfer_observation(100.0, -50.0, Vec2::new(50.0, 50.0), 6.0);
        let diagnostics = controller.transfer_diagnostics(&observation);

        assert!(diagnostics.projection.projected_dx_m.unwrap() < 0.0);
        assert!(controller.boost_attitude_rad(&observation, diagnostics) < 0.0);
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
}
