//! Experimental composition. Selection is a new, named finite search; the
//! accepted finite Poly Landing burn and original episode remain unchanged.
use super::*;
use crate::{
    AirborneArrivalController, ArrivalFeedback, ArrivalFrame, ArrivalRequest, ControllerFrame,
};
use pd_core::{
    Command, ContactClassification, Observation, RunContext, ScenarioSpec, SimulationState,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArrivalLandingProgram {
    pub arrival: Option<ArrivalRequest>,
    pub land: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_forecast_selection_preserves_live_state_and_matches_actual_touchdown() {
        let mut s: ScenarioSpec = serde_json::from_str(include_str!(
            "../../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .unwrap();
        s.initial_state.position_m = pd_core::Vec2::new(0.0, 600.0);
        s.initial_state.velocity_mps = pd_core::Vec2::new(0.0, -10.0);
        let ctx = RunContext::from_scenario(&s).unwrap();
        let mut live = SimulationState::new(&ctx).unwrap();
        let original = live.build_observation(&ctx);
        let mut c = ArrivalLandingController::new(
            &s,
            ArrivalLandingProgram {
                arrival: None,
                land: true,
            },
        )
        .unwrap();
        let first = c.frame(&original, 2_000_000).unwrap();
        assert_eq!(live.build_observation(&ctx), original);
        assert!(!c.landing_search().no_plan);
        assert!(c.landing_search().forecast_steps > 0);
        let expected = c.landing_search().candidates.last().unwrap()["contact_tick"]
            .as_u64()
            .unwrap();
        assert!(c.frame(&original, 2_000_000).is_err());
        let mut frame = first;
        loop {
            live.set_command(frame.command);
            for _ in 0..2 {
                let contact = live.step_physics_and_classify_contact(&ctx);
                if contact != ContactClassification::None {
                    assert_eq!(
                        contact,
                        ContactClassification::StableTouchdown { on_target: true }
                    );
                    assert_eq!(live.physics_step, expected);
                    return;
                }
            }
            frame = c.frame(&live.build_observation(&ctx), 2_000_000).unwrap();
            assert!(live.physics_step < 7200);
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LandingSearchFeedback {
    pub selector: String,
    pub selection_observation: Option<Observation>,
    pub candidates: Vec<serde_json::Value>,
    pub forecast_steps: u64,
    pub actuation_checks: u64,
    pub chosen_coast_ticks: Option<u64>,
    pub chosen_duration_ticks: Option<u64>,
    pub chosen_first_attitude_rad: Option<f64>,
    pub activation: Option<Observation>,
    pub no_plan: bool,
    pub reference_exhausted: bool,
}

pub struct ArrivalLandingController {
    scenario: ScenarioSpec,
    ctx: RunContext,
    program: ArrivalLandingProgram,
    arrival: Option<AirborneArrivalController>,
    arrival_active: bool,
    landing: Option<PolynomialLandingController>,
    selection: LandingSearchFeedback,
    next_tick: u64,
    deadline: u64,
    burn_start: Option<u64>,
    reference_end: Option<u64>,
    hold_attitude: f64,
    closed: bool,
}

impl ArrivalLandingController {
    pub fn new(scenario: &ScenarioSpec, program: ArrivalLandingProgram) -> Result<Self, String> {
        let ctx = RunContext::from_scenario(scenario)?;
        let deadline = (ctx.sim.max_time_s * 120.0).floor() as u64;
        PolynomialLandingController::new_checked(scenario, 0, deadline)
            .map_err(|e| format!("{e:?}"))?;
        let arrival = program
            .arrival
            .as_ref()
            .map(|_| AirborneArrivalController::new(scenario, 0))
            .transpose()?;
        Ok(Self {
            scenario: scenario.clone(),
            ctx,
            program,
            arrival,
            arrival_active: false,
            landing: None,
            selection: LandingSearchFeedback::default(),
            next_tick: 0,
            deadline,
            burn_start: None,
            reference_end: None,
            hold_attitude: scenario.initial_state.attitude_rad,
            closed: false,
        })
    }

    fn state_at(&self, o: &Observation) -> Result<SimulationState, String> {
        let mut s = SimulationState::new(&self.ctx).map_err(|e| e.to_string())?;
        s.physics_step = o.physics_step;
        s.sim_time_s = o.sim_time_s;
        s.position_m = o.position_m;
        s.velocity_mps = o.velocity_mps;
        s.attitude_rad = o.attitude_rad;
        s.angular_rate_radps = o.angular_rate_radps;
        s.fuel_kg = o.fuel_kg;
        Ok(s)
    }

    fn select(&mut self, o: &Observation, remaining_work: u64) -> Result<(), String> {
        self.selection.selector = "poly_native_first_feasible_coast_duration_grid_v1".into();
        self.selection.selection_observation = Some(o.clone());
        // Frozen order: first feasible, not a claim of minimum fuel.
        for coast_s in [0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 6.0] {
            let coast = (coast_s * 120.0) as u64;
            for duration_s in [6_u64, 8, 10, 12, 16, 20, 24, 30, 36] {
                let start = o.physics_step + coast;
                let end = start + duration_s * 120;
                if end + 4 > self.deadline {
                    continue;
                }
                // Native zero-thrust translation is independent of pointing.
                let mut entry = self.state_at(o)?;
                for _ in 0..coast {
                    if self.selection.forecast_steps >= remaining_work {
                        return Err("landing forecast resource limit".into());
                    }
                    entry.set_command(Command {
                        throttle_frac: 0.0,
                        target_attitude_rad: o.attitude_rad,
                    });
                    let contact = entry.step_physics_and_classify_contact(&self.ctx);
                    self.selection.forecast_steps += 1;
                    if contact != ContactClassification::None {
                        break;
                    }
                }
                if entry.physics_step != start
                    || entry.contact_diagnostics(&self.ctx).classification
                        != ContactClassification::None
                {
                    self.selection.candidates.push(serde_json::json!({"coast_ticks":coast,"duration_ticks":duration_s*120,"result":"coast_contact"}));
                    continue;
                }
                let projected = entry.build_observation(&self.ctx);
                let mut burn =
                    PolynomialLandingController::new_checked(&self.scenario, start, self.deadline)
                        .map_err(|e| format!("{e:?}"))?;
                let request = ManeuverRequestV2 {
                    schema_version: 2,
                    id: "arrival-then-poly-once".into(),
                    goal: ManeuverGoalV2::LandOnMissionTargetAt {
                        target_pad_id: self.ctx.target_pad.id.clone(),
                        reference_end_physics_step: end,
                    },
                };
                let admission = burn.submit(&request, &projected)?;
                self.selection.actuation_checks += burn.feedback().admission_actuation_checks;
                if admission != AdmissionV2::Accepted {
                    self.selection.candidates.push(serde_json::json!({"coast_ticks":coast,"duration_ticks":duration_s*120,"admission":admission}));
                    continue;
                }
                let reference = burn.feedback().accepted.unwrap().reference;
                let thrust = crate::poly_landing::paired_mean(&reference, &self.ctx, 0);
                let attitude = thrust.x.atan2(thrust.y);
                // Repeat the coast with preparation, then reconstruct the exact
                // adapter at its actual predicted activation observation.
                let mut forecast = self.state_at(o)?;
                let mut contact = ContactClassification::None;
                for _ in 0..coast {
                    if self.selection.forecast_steps >= remaining_work {
                        return Err("landing forecast resource limit".into());
                    }
                    forecast.set_command(Command {
                        throttle_frac: 0.0,
                        target_attitude_rad: attitude,
                    });
                    contact = forecast.step_physics_and_classify_contact(&self.ctx);
                    self.selection.forecast_steps += 1;
                    if contact != ContactClassification::None {
                        break;
                    }
                }
                if contact != ContactClassification::None {
                    continue;
                }
                let mut burn =
                    PolynomialLandingController::new_checked(&self.scenario, start, self.deadline)
                        .map_err(|e| format!("{e:?}"))?;
                if burn.submit(&request, &forecast.build_observation(&self.ctx))?
                    != AdmissionV2::Accepted
                {
                    return Err("deterministic prepared-coast admission changed".into());
                }
                self.selection.actuation_checks += burn.feedback().admission_actuation_checks;
                while forecast.physics_step < end + 4 && contact == ContactClassification::None {
                    let ManeuverFrameV2::Command(frame) =
                        burn.frame(&forecast.build_observation(&self.ctx))?
                    else {
                        break;
                    };
                    forecast.set_command(frame.command);
                    for _ in 0..2 {
                        if self.selection.forecast_steps >= remaining_work {
                            return Err("landing forecast resource limit".into());
                        }
                        contact = forecast.step_physics_and_classify_contact(&self.ctx);
                        self.selection.forecast_steps += 1;
                        if contact != ContactClassification::None {
                            break;
                        }
                    }
                }
                let pass = contact == ContactClassification::StableTouchdown { on_target: true };
                self.selection.candidates.push(serde_json::json!({"coast_ticks":coast,"duration_ticks":duration_s*120,"contact":contact,"contact_tick":forecast.physics_step,"result":if pass{"predicted_target_landing"}else{"forecast_failed"}}));
                if pass {
                    self.selection.chosen_coast_ticks = Some(coast);
                    self.selection.chosen_duration_ticks = Some(duration_s * 120);
                    self.selection.chosen_first_attitude_rad = Some(attitude);
                    self.burn_start = Some(start);
                    self.reference_end = Some(end);
                    self.hold_attitude = attitude;
                    return Ok(());
                }
            }
        }
        self.selection.no_plan = true;
        Ok(())
    }

    pub fn frame(
        &mut self,
        o: &Observation,
        remaining_forecast_work: u64,
    ) -> Result<ControllerFrame, String> {
        if self.closed
            || o.physics_step != self.next_tick
            || o.sim_time_s != o.physics_step as f64 / 120.0
        {
            return Err("closed or stale composition observation".into());
        }
        if self.next_tick == 0
            && let Some(arrival) = &mut self.arrival
        {
            if arrival.submit(self.program.arrival.as_ref().unwrap(), o)? != AdmissionV2::Accepted {
                return Err("arrival program admission failed".into());
            }
            self.arrival_active = true;
        }
        if self.arrival_active {
            match self.arrival.as_mut().unwrap().frame(o)? {
                ArrivalFrame::Command(frame) => {
                    self.next_tick += 2;
                    return Ok(frame);
                }
                ArrivalFrame::Resolved => self.arrival_active = false,
            }
        }
        if self.program.land && self.selection.selection_observation.is_none() {
            self.select(o, remaining_forecast_work)?;
        }
        if self.burn_start == Some(o.physics_step) {
            let mut landing = PolynomialLandingController::new_checked(
                &self.scenario,
                o.physics_step,
                self.deadline,
            )
            .map_err(|e| format!("{e:?}"))?;
            let admission = landing.submit(
                &ManeuverRequestV2 {
                    schema_version: 2,
                    id: "arrival-then-poly-once".into(),
                    goal: ManeuverGoalV2::LandOnMissionTargetAt {
                        target_pad_id: self.ctx.target_pad.id.clone(),
                        reference_end_physics_step: self.reference_end.unwrap(),
                    },
                },
                o,
            )?;
            if admission != AdmissionV2::Accepted {
                return Err("forecast/live Poly admission mismatch".into());
            }
            self.selection.activation = Some(o.clone());
            self.landing = Some(landing);
        }
        let mut frame = ControllerFrame::command_only(Command {
            throttle_frac: 0.0,
            target_attitude_rad: self.hold_attitude,
        });
        frame.phase = Some(
            if self.program.land {
                "landing_guidance_coast"
            } else {
                "arrival_observation_tail"
            }
            .into(),
        );
        if !self.selection.reference_exhausted
            && let Some(landing) = &mut self.landing
        {
            match landing.frame(o)? {
                ManeuverFrameV2::Command(command) => frame = command,
                ManeuverFrameV2::ReferenceExhausted => {
                    self.selection.reference_exhausted = true;
                    frame.phase = Some("poly_reference_exhausted".into());
                }
            }
        }
        self.next_tick += 2;
        Ok(frame)
    }

    pub fn arrival_feedback(&self) -> Option<&ArrivalFeedback> {
        self.arrival.as_ref().map(|a| a.feedback())
    }
    pub fn landing_search(&self) -> &LandingSearchFeedback {
        &self.selection
    }
    pub fn feedback(&self) -> Option<ManeuverFeedbackV2> {
        self.landing.as_ref().map(|p| p.feedback())
    }
    pub fn work_steps(&self) -> u64 {
        self.selection.forecast_steps + self.arrival_feedback().map_or(0, |f| f.model_steps)
    }
    pub fn native_end(&mut self, o: &Observation, end: NativeEndV1) -> Result<(), String> {
        if self.closed {
            return Err("composition already closed".into());
        }
        if let Some(arrival) = &mut self.arrival {
            arrival.native_end(o);
        }
        if let Some(landing) = &mut self.landing {
            landing.native_end(end)?;
        }
        self.closed = true;
        Ok(())
    }
}
