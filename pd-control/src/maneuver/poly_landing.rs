use pd_core::{Command, EvaluationGoal, Observation, RunContext, ScenarioSpec, Vec2};

use super::*;
use crate::{
    ControllerFrame,
    poly_landing::{LandingKinematicsV1, build_reference, paired_mean, paired_throttle},
};

pub const POLY_LANDING_BACKEND: &str = "polynomial_landing_v1";
pub const CHECKED_POLY_LANDING_BACKEND: &str = "polynomial_landing_v2";
const RESIDUAL_TICKS: u64 = 4;
const INVERSE_ITERATIONS: u32 = 48;

/// Finite-reference execution only. The caller retains the original episode and
/// all command authority before the explicitly bound first command boundary.
pub struct PolynomialLandingController {
    ctx: RunContext,
    first_tick: u64,
    next_tick: u64,
    deadline: u64,
    checked_admission: bool,
    feedback: ManeuverFeedbackV2,
}

impl PolynomialLandingController {
    pub fn new(
        scenario: &ScenarioSpec,
        first_command_physics_step: u64,
        absolute_deadline_physics_step: u64,
    ) -> Result<Self, ManeuverInputErrorV1> {
        use ManeuverInputErrorV1::{InvalidInput, UnsupportedInput};
        let ctx = RunContext::from_scenario(scenario).map_err(InvalidInput)?;
        let v = &ctx.vehicle;
        let g = &v.geometry;
        if ctx.sim.physics_hz != 120
            || ctx.sim.controller_hz != 60
            || ctx.world.gravity_mps2 != 9.81
            || ctx.mission.transfer_route.is_some()
            || !matches!(ctx.mission.goal, EvaluationGoal::LandingOnPad { .. })
            || v.dry_mass_kg != 7200.0
            || v.max_fuel_kg != 6300.0
            || v.max_thrust_n != 240_000.0
            || v.max_fuel_burn_kgps != 49.5
            || v.min_throttle_frac != 0.25
            || v.max_rotation_rate_radps != std::f64::consts::FRAC_PI_2
            || v.safe_touchdown_normal_speed_mps != 3.0
            || v.safe_touchdown_tangential_speed_mps != 2.0
            || v.safe_touchdown_attitude_error_rad != 0.15
            || v.safe_touchdown_angular_rate_radps != 0.35
            || g.hull_width_m != 8.0
            || g.hull_height_m != 10.0
            || g.touchdown_half_span_m != 4.0
            || g.touchdown_base_offset_m != 5.0
        {
            return Err(UnsupportedInput(
                "Poly Landing requires route-free landing, accepted 7200 kg vehicle, Earth 9.81 and 120/60 Hz".into(),
            ));
        }
        let horizon = (ctx.sim.max_time_s * f64::from(ctx.sim.physics_hz)).floor() as u64;
        if !first_command_physics_step.is_multiple_of(2)
            || first_command_physics_step >= absolute_deadline_physics_step
            || absolute_deadline_physics_step > horizon.min(72_000)
        {
            return Err(InvalidInput(
                "invalid original command/deadline clocks".into(),
            ));
        }
        Ok(Self {
            ctx,
            first_tick: first_command_physics_step,
            next_tick: first_command_physics_step,
            deadline: absolute_deadline_physics_step,
            checked_admission: false,
            feedback: ManeuverFeedbackV2 {
                schema_version: 2,
                accepted: None,
                command_updates: 0,
                constructions: 0,
                admission_actuation_checks: 0,
                reference_exhausted: false,
                native_end: None,
            },
        })
    }

    /// Opt-in complete algebraic actuation/fuel admission. No plant transition,
    /// attitude forecast, reference modification or duration selection occurs.
    pub fn new_checked(
        scenario: &ScenarioSpec,
        first_command_physics_step: u64,
        absolute_deadline_physics_step: u64,
    ) -> Result<Self, ManeuverInputErrorV1> {
        let mut controller = Self::new(
            scenario,
            first_command_physics_step,
            absolute_deadline_physics_step,
        )?;
        controller.checked_admission = true;
        Ok(controller)
    }

    fn check_observation(&self, observation: &Observation) -> Result<(), String> {
        if self.feedback.native_end.is_some() {
            return Err("maneuver controller is closed".into());
        }
        if observation.physics_step != self.next_tick
            || observation.sim_time_s != observation.physics_step as f64 / 120.0
        {
            return Err("stale, future, duplicate or skipped command observation".into());
        }
        if [
            observation.position_m.x,
            observation.position_m.y,
            observation.velocity_mps.x,
            observation.velocity_mps.y,
            observation.attitude_rad,
            observation.angular_rate_radps,
            observation.fuel_kg,
            observation.mass_kg,
            observation.touchdown_clearance_m,
            observation.min_hull_clearance_m,
        ]
        .iter()
        .any(|value| !value.is_finite())
            || observation.fuel_kg < 0.0
            || observation.fuel_kg > self.ctx.vehicle.initial_fuel_kg
            || observation.mass_kg != self.ctx.vehicle.dry_mass_kg + observation.fuel_kg
            || observation.gravity_mps2 != self.ctx.world.gravity_mps2
            || observation.target_dx_m != self.ctx.target_pad.center_x_m - observation.position_m.x
            || observation.target_surface_y_m != self.ctx.target_pad.surface_y_m
        {
            return Err("observation is non-finite or inconsistent with the bound episode".into());
        }
        Ok(())
    }
}

impl ManeuverControllerV2 for PolynomialLandingController {
    fn descriptor(&self) -> ManeuverDescriptorV2 {
        ManeuverDescriptorV2 {
            schema_version: 2,
            implementation: if self.checked_admission {
                "polynomial-landing-controller-v2"
            } else {
                "polynomial-landing-controller-v1"
            }
            .into(),
            backend: ManeuverBackendDescriptorV1 {
                implementation: if self.checked_admission {
                    CHECKED_POLY_LANDING_BACKEND
                } else {
                    POLY_LANDING_BACKEND
                }
                .into(),
                configuration: serde_json::json!({
                    "reference_family":"quadratic_horizontal_affine_vertical_exact_discrete",
                    "terminal_target_downward_speed_fraction":0.5,
                    "contact_undershoot_m":0.005,
                    "maximum_after_reference_ticks":RESIDUAL_TICKS,
                    "scalar_inverse_iterations":INVERSE_ITERATIONS,
                    "construction":if self.checked_admission { "full_paired_actuation_scan_no_native_forecast" } else { "current_observation_once_no_native_forecast" },
                }),
            },
            physics_hz: 120,
            controller_hz: 60,
            first_command_physics_step: self.first_tick,
            absolute_deadline_physics_step: self.deadline,
            coast_owner: "external planner/prefix; no controller coast command".into(),
            command_mapping: "paired native inverse; raw engine setting and absolute attitude"
                .into(),
        }
    }

    fn submit(
        &mut self,
        request: &ManeuverRequestV2,
        observation: &Observation,
    ) -> Result<AdmissionV2, String> {
        if self.feedback.native_end.is_some() {
            return Err("maneuver controller is closed".into());
        }
        if let Err(reason) = request
            .validate()
            .and_then(|()| self.check_observation(observation))
        {
            return Ok(AdmissionV2::InvalidRequest { reason });
        }
        if let Some(active) = &self.feedback.accepted {
            return Ok(if request.id == active.request.id {
                if *request == active.request {
                    AdmissionV2::AlreadyActive
                } else {
                    AdmissionV2::InvalidRequest {
                        reason: "ID is bound to another payload".into(),
                    }
                }
            } else {
                AdmissionV2::Busy
            });
        }
        let ManeuverGoalV2::LandOnMissionTargetAt {
            target_pad_id,
            reference_end_physics_step,
        } = &request.goal
        else {
            return Ok(AdmissionV2::Unsupported {
                reason: "cancellation is unsupported".into(),
            });
        };
        if self.ctx.world.landing_pad(target_pad_id).is_none() {
            return Ok(AdmissionV2::InvalidRequest {
                reason: "unknown target pad".into(),
            });
        }
        if *target_pad_id != self.ctx.target_pad.id {
            return Ok(AdmissionV2::Unsupported {
                reason: "retargeting is unsupported".into(),
            });
        }
        let end_tick = *reference_end_physics_step;
        if end_tick <= observation.physics_step
            || !end_tick.is_multiple_of(2)
            || end_tick
                .checked_add(RESIDUAL_TICKS)
                .is_none_or(|limit| limit > self.deadline)
        {
            return Ok(AdmissionV2::InvalidRequest { reason: "reference end must be future, pair-aligned and leave the original four-tick allowance".into() });
        }
        if observation.touchdown_clearance_m <= 0.0 || observation.min_hull_clearance_m <= 0.0 {
            return Ok(AdmissionV2::Unsupported {
                reason: "requires an actual airborne activation observation".into(),
            });
        }
        let end = LandingKinematicsV1 {
            position_m: Vec2::new(
                self.ctx.target_pad.center_x_m,
                self.ctx.target_pad.surface_y_m + self.ctx.vehicle.geometry.touchdown_base_offset_m
                    - 0.005,
            ),
            velocity_mps: Vec2::new(0.0, -0.5 * self.ctx.vehicle.safe_touchdown_normal_speed_mps),
        };
        self.feedback.constructions += 1;
        let construction = (|| {
            let reference = build_reference(
                &self.ctx,
                LandingKinematicsV1 {
                    position_m: observation.position_m,
                    velocity_mps: observation.velocity_mps,
                },
                end,
                end_tick - observation.physics_step,
            )?;
            let first = paired_mean(&reference, &self.ctx, 0);
            paired_throttle(
                &self.ctx,
                INVERSE_ITERATIONS,
                observation.mass_kg,
                first.length(),
            )?;
            if self.checked_admission {
                let mut fuel = observation.fuel_kg;
                for tick in (0..reference.physics_ticks + RESIDUAL_TICKS).step_by(2) {
                    self.feedback.admission_actuation_checks += 1;
                    let setting = paired_throttle(
                        &self.ctx,
                        INVERSE_ITERATIONS,
                        self.ctx.vehicle.dry_mass_kg + fuel,
                        paired_mean(&reference, &self.ctx, tick).length(),
                    )?;
                    if setting <= 0.0 {
                        return Err("declared burn contains an engine-off pair".into());
                    }
                    let q = self.ctx.vehicle.min_throttle_frac
                        + setting * (1.0 - self.ctx.vehicle.min_throttle_frac);
                    for _ in 0..2 {
                        let required =
                            self.ctx.vehicle.max_fuel_burn_kgps * q * self.ctx.sim.physics_dt_s();
                        if required > fuel {
                            return Err("declared burn exceeds remaining fuel".into());
                        }
                        fuel -= required;
                    }
                }
            }
            Ok::<_, String>(reference)
        })();
        let reference = match construction {
            Ok(reference) => reference,
            Err(reason) => return Ok(AdmissionV2::NoPlan { reason }),
        };
        self.feedback.accepted = Some(AcceptedManeuverV2 {
            request: request.clone(),
            entry: observation.clone(),
            reference,
        });
        Ok(AdmissionV2::Accepted)
    }

    fn frame(&mut self, observation: &Observation) -> Result<ManeuverFrameV2, String> {
        self.check_observation(observation)?;
        let active = self
            .feedback
            .accepted
            .as_ref()
            .ok_or("no accepted timed landing; caller retains command authority")?;
        if self.feedback.command_updates == 0 && *observation != active.entry {
            return Err(
                "activation observation changed between admission and first command".into(),
            );
        }
        let tick = observation.physics_step - active.entry.physics_step;
        if tick >= active.reference.physics_ticks + RESIDUAL_TICKS {
            self.feedback.reference_exhausted = true;
            return Ok(ManeuverFrameV2::ReferenceExhausted);
        }
        let thrust = paired_mean(&active.reference, &self.ctx, tick);
        let setting = paired_throttle(
            &self.ctx,
            INVERSE_ITERATIONS,
            observation.mass_kg,
            thrust.length(),
        )?;
        if setting <= 0.0 {
            return Err("finite landing reference requested an engine-off command".into());
        }
        let mut frame = ControllerFrame::command_only(Command {
            throttle_frac: setting,
            target_attitude_rad: thrust.x.atan2(thrust.y),
        });
        frame.phase = Some("poly_landing".into());
        frame
            .metrics
            .insert("reference_local_physics_step".into(), (tick as f64).into());
        frame.metrics.insert(
            "requested_thrust_acceleration_mps2".into(),
            thrust.length().into(),
        );
        frame.metrics.insert(
            "mapped_thrust_fraction".into(),
            (0.25 + 0.75 * setting).into(),
        );
        self.next_tick += 2;
        self.feedback.command_updates += 1;
        Ok(ManeuverFrameV2::Command(frame))
    }

    fn feedback(&self) -> ManeuverFeedbackV2 {
        self.feedback.clone()
    }

    fn native_end(&mut self, end: NativeEndV1) -> Result<(), String> {
        if self.feedback.native_end.is_some() {
            return Err("maneuver controller is closed".into());
        }
        if self.feedback.accepted.is_none() {
            if end.schema_version != 1 || end.physics_step > self.deadline {
                return Err("inactive native end exceeds the original episode clock".into());
            }
            self.feedback.native_end = Some(end);
            return Ok(());
        }
        if end.schema_version != 1
            || end.physics_step
                < self
                    .next_tick
                    .saturating_sub(if self.feedback.command_updates == 0 {
                        0
                    } else {
                        2
                    })
            || end.physics_step > self.next_tick
        {
            return Err("native end is outside the last emitted command interval".into());
        }
        if self.checked_admission
            && self.feedback.accepted.as_ref().is_some_and(|active| {
                end.physics_step
                    == active.entry.physics_step + active.reference.physics_ticks + RESIDUAL_TICKS
            })
            && end.physical_outcome == pd_core::PhysicalOutcome::Flying
        {
            self.feedback.reference_exhausted = true;
        }
        self.feedback.native_end = Some(end);
        Ok(())
    }
}
