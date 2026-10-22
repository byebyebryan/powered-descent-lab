use pd_core::{Command, EvaluationGoal, Observation, RunContext, ScenarioSpec, SimulationState};

use super::{
    AdmissionV1, Lifecycle, ManeuverController, ManeuverDescriptorV1, ManeuverFeedbackV1,
    ManeuverGoalV1, ManeuverInputErrorV1, ManeuverRequestV1, NativeEndV1,
};
use crate::{Controller, ControllerFrame, TerminalPdgController};

/// Ordinary terminal implementation, frozen constructor and episode-local state.
/// The original context is owned here and cannot be replaced during execution.
pub struct TerminalLandingController {
    ctx: RunContext,
    lifecycle: Lifecycle,
    backend: Option<TerminalPdgController>,
}

impl TerminalLandingController {
    pub fn new(scenario: &ScenarioSpec) -> Result<Self, ManeuverInputErrorV1> {
        use ManeuverInputErrorV1::{InvalidInput, UnsupportedInput};
        let ctx = RunContext::from_scenario(scenario).map_err(InvalidInput)?;
        // Fixed engine/geometry family; mass, fuel, terrain, pads, deadline and
        // finite airborne entry state may vary. These are semantics, not a win gate.
        let v = &ctx.vehicle;
        let g = &v.geometry;
        if ctx.sim.physics_hz != 120
            || ctx.sim.controller_hz != 60
            || ctx.world.gravity_mps2 != 9.81
            || ctx.mission.transfer_route.is_some()
            || !matches!(ctx.mission.goal, EvaluationGoal::LandingOnPad { .. })
            || g.hull_width_m != 8.0
            || g.hull_height_m != 10.0
            || g.touchdown_half_span_m != 4.0
            || g.touchdown_base_offset_m != 5.0
            || v.max_thrust_n != 240_000.0
            || v.max_fuel_burn_kgps != 49.5
            || v.min_throttle_frac != 0.25
            || v.max_rotation_rate_radps != std::f64::consts::FRAC_PI_2
            || v.safe_touchdown_normal_speed_mps != 3.0
            || v.safe_touchdown_tangential_speed_mps != 2.0
            || v.safe_touchdown_attitude_error_rad != 0.15
            || v.safe_touchdown_angular_rate_radps != 0.35
        {
            return Err(UnsupportedInput(
                "requires route-free landing, Earth 9.81, 120/60 Hz and terminal vehicle family v1"
                    .into(),
            ));
        }
        let observation = SimulationState::new(&ctx)
            .map_err(|e| InvalidInput(e.to_string()))?
            .build_observation(&ctx);
        if observation.touchdown_clearance_m <= 0.0 || observation.min_hull_clearance_m <= 0.0 {
            return Err(UnsupportedInput(
                "requires an airborne initial state".into(),
            ));
        }
        Ok(Self {
            ctx,
            lifecycle: Lifecycle::new(2),
            backend: None,
        })
    }
}

impl ManeuverController for TerminalLandingController {
    fn descriptor(&self) -> ManeuverDescriptorV1 {
        ManeuverDescriptorV1 {
            schema_version: 1,
            implementation: "terminal-landing-controller-v1".into(),
            capability: "route-free airborne LandOnMissionTarget; Earth 9.81; terminal vehicle v1 fixed geometry/engine/servo/contact limits; variable valid mass/fuel/terrain/pads/entry/deadline".into(),
            physics_hz: 120, controller_hz: 60,
            backend: super::ManeuverBackendDescriptorV1 {
                implementation: "ordinary-terminal-pdg-v1".into(),
                configuration: serde_json::to_value(TerminalPdgController::default().resolved_configuration()).expect("finite frozen terminal configuration"),
            },
            coast: "Command::idle(): engine off, upright attitude servo".into(),
            command_mapping: "raw native engine setting and absolute attitude; no remap or added clamp".into(),
        }
    }

    fn submit(
        &mut self,
        request: &ManeuverRequestV1,
        physics_step: u64,
    ) -> Result<AdmissionV1, String> {
        self.lifecycle.live()?;
        if physics_step != self.lifecycle.next_tick {
            return Ok(AdmissionV1::InvalidRequest {
                reason: "stale or future submission tick".into(),
            });
        }
        request.validate()?;
        // Identity conflict precedes goal support, including changed Cancel payload.
        if let Some(active) = &self.lifecycle.feedback.accepted
            && request.id == active.request.id
        {
            return Ok(self.lifecycle.accept(request, physics_step));
        }
        let ManeuverGoalV1::LandOnMissionTarget { target_pad_id } = &request.goal else {
            return Ok(AdmissionV1::Unsupported {
                reason: "cancellation is unsupported".into(),
            });
        };
        if self.ctx.world.landing_pad(target_pad_id).is_none() {
            return Ok(AdmissionV1::InvalidRequest {
                reason: "unknown target pad".into(),
            });
        }
        if *target_pad_id != self.ctx.target_pad.id {
            return Ok(AdmissionV1::Unsupported {
                reason: "retargeting is unsupported".into(),
            });
        }
        let admission = self.lifecycle.accept(request, physics_step);
        if admission == AdmissionV1::Accepted {
            let mut backend = TerminalPdgController::default();
            backend.reset(&self.ctx);
            self.backend = Some(backend);
        }
        Ok(admission)
    }

    fn frame(&mut self, observation: &Observation) -> Result<ControllerFrame, String> {
        self.lifecycle.check_frame(observation)?;
        let frame = match &mut self.backend {
            Some(backend) => backend.update(&self.ctx, observation),
            None => ControllerFrame::command_only(Command::idle()),
        };
        self.lifecycle.produced(self.backend.is_some());
        Ok(frame)
    }

    fn feedback(&self) -> ManeuverFeedbackV1 {
        self.lifecycle.feedback.clone()
    }
    fn native_end(&mut self, end: NativeEndV1) -> Result<(), String> {
        self.lifecycle.close(end)
    }
}
