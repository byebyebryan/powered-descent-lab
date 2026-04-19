use crate::kit::{ControllerFrameBuilder, ControllerView, metric, phase, standard_marker};
use crate::{Controller, ControllerFrame, TelemetryValue};
use pd_core::{Command, Observation, RunContext};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
}

impl ControllerSpec {
    pub fn id(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::BaselineV1 { .. } => "baseline_v1",
            Self::StagedDescentV1 { .. } => "staged_descent_v1",
        }
    }

    pub fn instantiate(&self) -> Box<dyn Controller> {
        match self {
            Self::Idle => Box::new(IdleController),
            Self::BaselineV1 { config } => Box::new(BaselineController::new(config.clone())),
            Self::StagedDescentV1 { config } => {
                Box::new(StagedDescentController::new(config.clone()))
            }
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
