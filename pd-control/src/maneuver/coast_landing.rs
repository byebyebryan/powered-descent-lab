//! Small controller-owned engine-off hold followed by one fixed timed burn.
use super::*;
use pd_core::{Command, Observation, ScenarioSpec};

pub struct CoastLandingController {
    scenario: ScenarioSpec,
    hold_attitude: f64,
    reference_end: u64,
    deadline: u64,
    next_tick: u64,
    attempted: bool,
    active: bool,
    closed: bool,
    landing: Option<PolynomialLandingController>,
}

impl CoastLandingController {
    pub fn new(scenario: &ScenarioSpec, reference_end: u64) -> Result<Self, String> {
        if scenario.sim.max_time_s != 40.0 {
            return Err("coast experiment requires the original 40-second mission deadline".into());
        }
        PolynomialLandingController::new_checked(scenario, 0, 4800)
            .map_err(|e| format!("{e:?}"))?;
        if reference_end <= 960
            || !reference_end.is_multiple_of(2)
            || reference_end.checked_add(4).is_none_or(|v| v > 4800)
        {
            return Err(
                "coast task needs a pair-aligned endpoint after the ignition window".into(),
            );
        }
        Ok(Self {
            scenario: scenario.clone(),
            hold_attitude: scenario.initial_state.attitude_rad,
            reference_end,
            deadline: 4800,
            next_tick: 0,
            attempted: false,
            active: false,
            closed: false,
            landing: None,
        })
    }

    fn check(&self, observation: &Observation) -> Result<(), String> {
        if self.closed || observation.physics_step != self.next_tick {
            return Err("closed controller or stale/skipped command observation".into());
        }
        Ok(())
    }

    pub fn ignite(&mut self, observation: &Observation) -> Result<AdmissionV2, String> {
        self.check(observation)?;
        if self.attempted {
            return Err("only one ignition attempt is permitted".into());
        }
        self.attempted = true;
        let mut landing = PolynomialLandingController::new_checked(
            &self.scenario,
            observation.physics_step,
            self.deadline,
        )
        .map_err(|e| format!("{e:?}"))?;
        let request = ManeuverRequestV2 {
            schema_version: 2,
            id: "coast-ignite-once".into(),
            goal: ManeuverGoalV2::LandOnMissionTargetAt {
                target_pad_id: self.scenario.mission.goal.target_pad_id().into(),
                reference_end_physics_step: self.reference_end,
            },
        };
        let admission = landing.submit(&request, observation)?;
        match &admission {
            AdmissionV2::Accepted => self.active = true,
            AdmissionV2::NoPlan { .. } => {}
            other => return Err(format!("unexpected task admission: {other:?}")),
        }
        self.landing = Some(landing);
        Ok(admission)
    }

    pub fn frame(&mut self, observation: &Observation) -> Result<crate::ControllerFrame, String> {
        self.check(observation)?;
        let frame = if self.active {
            match self.landing.as_mut().unwrap().frame(observation)? {
                ManeuverFrameV2::Command(frame) => frame,
                ManeuverFrameV2::ReferenceExhausted => {
                    return Err("coverage must close at reference end plus four".into());
                }
            }
        } else {
            let mut frame = crate::ControllerFrame::command_only(Command {
                throttle_frac: 0.0,
                target_attitude_rad: self.hold_attitude,
            });
            frame.phase = Some("coast_hold".into());
            frame
        };
        self.next_tick += 2;
        Ok(frame)
    }

    pub fn feedback(&self) -> Option<ManeuverFeedbackV2> {
        self.landing.as_ref().map(|c| c.feedback())
    }

    pub fn native_end(&mut self, end: NativeEndV1) -> Result<(), String> {
        if self.closed {
            return Err("controller already closed".into());
        }
        if let Some(landing) = &mut self.landing {
            landing.native_end(end)?;
        }
        self.closed = true;
        Ok(())
    }
}
