//! Live airborne entry to the existing waypoint kernel, without route or
//! terminal-PDG orchestration. Admission accepts an attempt, not reachability.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArrivalRequest {
    pub waypoint: TransferWaypointSpec,
    pub deadline_physics_step: u64,
}

/// Read-only contract assessment for independent native witness tapes.
pub fn assess_arrival_handoff(
    request: &ArrivalRequest,
    anchor: Vec2,
    o: &Observation,
) -> pd_core::WaypointHandoffAssessment {
    let delta = request.waypoint.position_m - anchor;
    let stats = waypoint_leg_stats_from_axes(
        o.position_m,
        o.velocity_mps,
        request.waypoint.position_m,
        delta * (1.0 / delta.length()),
        request.waypoint.handoff_tangent_unit.unwrap(),
    );
    request
        .waypoint
        .assess_handoff(waypoint_handoff_kinematics(stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maneuver::AdmissionV2;
    use pd_core::{ScenarioSpec, SimulationState};

    fn fixture() -> (ScenarioSpec, RunContext, SimulationState, ArrivalRequest) {
        let mut s: ScenarioSpec = serde_json::from_str(include_str!(
            "../../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .unwrap();
        s.initial_state.position_m = Vec2::new(-500.0, 600.0);
        s.initial_state.velocity_mps = Vec2::new(30.0, -20.0);
        let ctx = RunContext::from_scenario(&s).unwrap();
        let state = SimulationState::new(&ctx).unwrap();
        let request = ArrivalRequest {
            deadline_physics_step: 2400,
            waypoint: TransferWaypointSpec {
                id: "test-arrival".into(),
                position_m: Vec2::new(-200.0, 300.0),
                handoff_tangent_unit: Some(Vec2::new(0.6, -0.8)),
                capture_radius_m: 30.0,
                max_cross_track_m: 30.0,
                max_outbound_heading_error_rad: 0.25,
                min_outbound_progress_mps: 10.0,
                max_outbound_cross_speed_mps: Some(5.0),
                min_speed_mps: 10.0,
                max_speed_mps: 50.0,
                min_vertical_speed_mps: Some(-45.0),
                max_vertical_speed_mps: Some(0.0),
            },
        };
        (s, ctx, state, request)
    }

    #[test]
    fn admission_is_idempotent_busy_and_bound_to_live_anchor_and_clocks() {
        let (s, ctx, state, r) = fixture();
        let o = state.build_observation(&ctx);
        let mut c = AirborneArrivalController::new(&s, 0).unwrap();
        assert!(c.frame(&o).is_err());
        assert_eq!(c.submit(&r, &o).unwrap(), AdmissionV2::Accepted);
        assert_eq!(c.submit(&r, &o).unwrap(), AdmissionV2::AlreadyActive);
        let mut other = r.clone();
        other.waypoint.id = "other".into();
        assert_eq!(c.submit(&other, &o).unwrap(), AdmissionV2::Busy);
        other = r.clone();
        other.deadline_physics_step += 2;
        assert!(matches!(
            c.submit(&other, &o).unwrap(),
            AdmissionV2::InvalidRequest { .. }
        ));
        assert_eq!(c.feedback().anchor, Some(o.clone()));
        let ArrivalFrame::Command(_) = c.frame(&o).unwrap() else {
            panic!()
        };
        assert!(c.frame(&o).is_err());
        assert!(c.submit(&r, &o).is_err());
    }

    #[test]
    fn live_adapter_commands_equal_shared_kernel_without_terminal_advice() {
        let (s, ctx, mut state, r) = fixture();
        let mut c = AirborneArrivalController::new(&s, 0).unwrap();
        c.submit(&r, &state.build_observation(&ctx)).unwrap();
        let anchor = state.position_m;
        let delta = r.waypoint.position_m - anchor;
        let g = WaypointLegGeometry {
            active_index: 0,
            waypoint: &r.waypoint,
            anchor_m: anchor,
            target_m: r.waypoint.position_m,
            leg_unit: delta * (1.0 / delta.length()),
            leg_length_m: delta.length(),
            handoff_tangent_unit: r.waypoint.handoff_tangent_unit.unwrap(),
        };
        let mut kernel = TransferPdgController::default();
        for _ in 0..30 {
            let o = state.build_observation(&ctx);
            let stats = waypoint_leg_stats(&o, &g);
            let approach = waypoint_approach_state(&ctx, &o, &g, stats, 0.72);
            let guidance = waypoint_guidance_frame(&g, stats, approach);
            let expected = kernel.waypoint_guidance_command_state(&ctx, &o, guidance);
            assert!(expected.continuation_prediction.is_none());
            assert!(expected.joint_prediction.is_none());
            assert!(expected.final_terminal_recoverable.is_none());
            let ArrivalFrame::Command(actual) = c.frame(&o).unwrap() else {
                panic!()
            };
            assert_eq!(actual.command, expected.command);
            state.set_command(actual.command);
            for _ in 0..2 {
                let _ = state.step_physics_and_classify_contact(&ctx);
            }
        }
        assert_eq!(c.feedback().command_updates, 30);
        assert!(c.feedback().model_steps > 0);
    }

    #[test]
    fn capture_requires_velocity_and_plane_miss_is_not_completion() {
        for pass in [false, true] {
            let (s, ctx, mut state, r) = fixture();
            let mut c = AirborneArrivalController::new(&s, 0).unwrap();
            c.submit(&r, &state.build_observation(&ctx)).unwrap();
            let ArrivalFrame::Command(_) = c.frame(&state.build_observation(&ctx)).unwrap() else {
                panic!()
            };
            state.position_m = r.waypoint.position_m;
            state.velocity_mps = if pass {
                Vec2::new(12.0, -16.0)
            } else {
                Vec2::new(-20.0, 0.0)
            };
            state.physics_step = 2;
            state.sim_time_s = 2.0 / 120.0;
            assert!(matches!(
                c.frame(&state.build_observation(&ctx)).unwrap(),
                ArrivalFrame::Resolved
            ));
            assert_eq!(
                c.feedback().status,
                if pass {
                    ArrivalStatus::Reached
                } else {
                    ArrivalStatus::PlaneMiss
                }
            );
            assert_eq!(c.feedback().window_entry_physics_step, Some(2));
            c.native_end(&state.build_observation(&ctx));
            assert!(c.submit(&r, &state.build_observation(&ctx)).is_err());
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArrivalStatus {
    Idle,
    Active,
    Reached,
    PlaneMiss,
    Expired,
    NativeEnd,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArrivalFeedback {
    pub backend: String,
    pub status: ArrivalStatus,
    pub request: Option<ArrivalRequest>,
    pub anchor: Option<Observation>,
    pub window_entry_physics_step: Option<u64>,
    pub resolution: Option<Observation>,
    pub replans: u32,
    pub model_steps: u64,
    pub command_updates: u64,
    pub tilt_saturated_updates: u64,
    pub thrust_saturated_updates: u64,
    pub deadline_remaining_s: Option<f64>,
    pub violations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuation: Option<ArrivalActuationFeedback>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArrivalActuationFeedback {
    pub profile: crate::arrival_actuation::ArrivalActuationProfile,
    pub last: Option<crate::arrival_actuation::ArrivalAllocation>,
    pub regime_updates: std::collections::BTreeMap<String, u64>,
    pub engine_switches: u64,
    pub previous_engine_on: Option<bool>,
    pub maximum_tracking_scalar_error_mps2: f64,
    pub ideal_contract_pass: bool,
    pub actuator_forecast_contract_pass: bool,
    pub ideal_pass_ever: bool,
    pub forecast_time_s: f64,
    pub operation_remaining_s: f64,
}

pub enum ArrivalFrame {
    Command(ControllerFrame),
    Resolved,
}

pub struct AirborneArrivalController {
    ctx: RunContext,
    kernel: TransferPdgController,
    next_tick: u64,
    feedback: ArrivalFeedback,
    closed: bool,
}

impl AirborneArrivalController {
    pub fn new(scenario: &pd_core::ScenarioSpec, first_tick: u64) -> Result<Self, String> {
        let ctx = RunContext::from_scenario(scenario)?;
        let pd_core::TerrainDefinition::Heightfield { points_m } = &scenario.world.terrain;
        let flat = points_m.iter().all(|p| p.y == ctx.target_pad.surface_y_m);
        if !flat
            || ctx.mission.transfer_route.is_some()
            || ctx.sim.physics_hz != 120
            || ctx.sim.controller_hz != 60
            || !first_tick.is_multiple_of(2)
            || first_tick >= (ctx.sim.max_time_s * 120.0).floor() as u64
        {
            return Err("airborne arrival v1 supports flat route-free 120/60 Hz episodes".into());
        }
        Ok(Self {
            ctx,
            kernel: TransferPdgController::default(),
            next_tick: first_tick,
            closed: false,
            feedback: ArrivalFeedback {
                backend: "waypoint_live_arrival_v1_no_terminal_advice".into(),
                status: ArrivalStatus::Idle,
                request: None,
                anchor: None,
                window_entry_physics_step: None,
                resolution: None,
                replans: 0,
                model_steps: 0,
                command_updates: 0,
                tilt_saturated_updates: 0,
                thrust_saturated_updates: 0,
                deadline_remaining_s: None,
                violations: vec![],
                actuation: None,
            },
        })
    }

    /// Explicit experiment profile; the ordinary constructor remains legacy.
    pub fn with_actuation_profile(
        scenario: &pd_core::ScenarioSpec,
        first_tick: u64,
        profile: crate::arrival_actuation::ArrivalActuationProfile,
    ) -> Result<Self, String> {
        let mut controller = Self::new(scenario, first_tick)?;
        controller.kernel.arrival_actuation_profile = profile;
        if profile == crate::arrival_actuation::ArrivalActuationProfile::NativePairV2 {
            controller.feedback.backend =
                "waypoint_live_arrival_v2_native_pair_no_terminal_advice".into();
        }
        controller.feedback.actuation = Some(ArrivalActuationFeedback {
            profile,
            last: None,
            regime_updates: Default::default(),
            engine_switches: 0,
            previous_engine_on: None,
            maximum_tracking_scalar_error_mps2: 0.0,
            ideal_contract_pass: false,
            actuator_forecast_contract_pass: false,
            ideal_pass_ever: false,
            forecast_time_s: 0.0,
            operation_remaining_s: 0.0,
        });
        Ok(controller)
    }

    fn check(&self, o: &Observation) -> Result<(), String> {
        if self.closed
            || o.physics_step != self.next_tick
            || o.sim_time_s != o.physics_step as f64 / 120.0
            || [
                o.position_m.x,
                o.position_m.y,
                o.velocity_mps.x,
                o.velocity_mps.y,
                o.attitude_rad,
                o.angular_rate_radps,
                o.mass_kg,
                o.fuel_kg,
                o.min_hull_clearance_m,
                o.touchdown_clearance_m,
            ]
            .iter()
            .any(|v| !v.is_finite())
            || o.fuel_kg < 0.0
            || o.fuel_kg > self.ctx.vehicle.initial_fuel_kg
            || o.mass_kg != self.ctx.vehicle.dry_mass_kg + o.fuel_kg
            || o.gravity_mps2 != self.ctx.world.gravity_mps2
            || o.target_dx_m != self.ctx.target_pad.center_x_m - o.position_m.x
            || o.target_surface_y_m != self.ctx.target_pad.surface_y_m
        {
            return Err("stale/skipped or inconsistent arrival observation".into());
        }
        Ok(())
    }

    pub fn submit(
        &mut self,
        request: &ArrivalRequest,
        o: &Observation,
    ) -> Result<crate::maneuver::AdmissionV2, String> {
        use crate::maneuver::AdmissionV2;
        if self.feedback.status == ArrivalStatus::NativeEnd {
            return Err("arrival controller is closed".into());
        }
        self.check(o)?;
        if let Some(active) = &self.feedback.request {
            return Ok(if request.waypoint.id == active.waypoint.id {
                if request == active {
                    AdmissionV2::AlreadyActive
                } else {
                    AdmissionV2::InvalidRequest {
                        reason: "arrival ID already bound".into(),
                    }
                }
            } else {
                AdmissionV2::Busy
            });
        }
        if let Err(reason) = request.waypoint.validate() {
            return Ok(AdmissionV2::InvalidRequest { reason });
        }
        let w = &request.waypoint;
        if request.deadline_physics_step <= o.physics_step
            || !request.deadline_physics_step.is_multiple_of(2)
            || request.deadline_physics_step > (self.ctx.sim.max_time_s * 120.0).floor() as u64
            || w.max_outbound_heading_error_rad > std::f64::consts::PI
            || w.min_outbound_progress_mps < 0.0
            || w.handoff_tangent_unit.is_none()
            || (w.position_m - o.position_m).length() <= w.capture_radius_m
        {
            return Ok(AdmissionV2::InvalidRequest {
                reason: "invalid arrival bounds or initial capture region".into(),
            });
        }
        if o.touchdown_clearance_m <= 0.0 || o.min_hull_clearance_m <= 0.0 {
            return Ok(AdmissionV2::Unsupported {
                reason: "arrival requires airborne entry".into(),
            });
        }
        self.feedback.request = Some(request.clone());
        self.feedback.anchor = Some(o.clone());
        self.feedback.status = ArrivalStatus::Active;
        Ok(AdmissionV2::Accepted)
    }

    pub fn frame(&mut self, o: &Observation) -> Result<ArrivalFrame, String> {
        self.check(o)?;
        if self.feedback.status != ArrivalStatus::Active {
            return Err("arrival has no active command authority".into());
        }
        if self.feedback.command_updates == 0 && self.feedback.anchor.as_ref() != Some(o) {
            return Err("arrival entry changed after admission".into());
        }
        let request = self.feedback.request.as_ref().unwrap();
        let anchor_m = self.feedback.anchor.as_ref().unwrap().position_m;
        let delta = request.waypoint.position_m - anchor_m;
        let geometry = WaypointLegGeometry {
            active_index: 0,
            waypoint: &request.waypoint,
            anchor_m,
            target_m: request.waypoint.position_m,
            leg_unit: delta * (1.0 / delta.length()),
            leg_length_m: delta.length(),
            handoff_tangent_unit: request.waypoint.handoff_tangent_unit.unwrap(),
        };
        let stats = waypoint_leg_stats(o, &geometry);
        let assessment = request
            .waypoint
            .assess_handoff(waypoint_handoff_kinematics(stats));
        if assessment.capture_window_open && self.feedback.window_entry_physics_step.is_none() {
            self.feedback.window_entry_physics_step = Some(o.physics_step);
        }
        self.feedback.violations = assessment
            .violations
            .iter()
            .map(|v| v.as_str().into())
            .collect();
        let window = self.feedback.window_entry_physics_step.is_some();
        let status = if assessment.contract_pass_in_window(window) {
            ArrivalStatus::Reached
        } else if assessment.deadline_reached {
            ArrivalStatus::PlaneMiss
        } else if o.physics_step >= request.deadline_physics_step {
            ArrivalStatus::Expired
        } else {
            ArrivalStatus::Active
        };
        if status != ArrivalStatus::Active {
            self.feedback.status = status;
            self.feedback.resolution = Some(o.clone());
            return Ok(ArrivalFrame::Resolved);
        }
        let tilt = self
            .kernel
            .config
            .boost_tilt_rad
            .max(self.kernel.config.uphill_boost_tilt_rad);
        let approach = waypoint_approach_state(&self.ctx, o, &geometry, stats, tilt);
        let guidance = waypoint_guidance_frame(&geometry, stats, approach);
        let state = self
            .kernel
            .waypoint_guidance_command_state(&self.ctx, o, guidance);
        if let Some(a) = &mut self.feedback.actuation {
            let allocated = crate::arrival_actuation::allocate_arrival_acceleration(
                &self.ctx,
                o,
                state.requested_accel_mps2,
                tilt,
                a.profile,
            )?;
            if allocated.command != state.command {
                return Err("live/forecast allocation profile mismatch".into());
            }
            let key = serde_json::to_value(allocated.regime)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned();
            *a.regime_updates.entry(key).or_default() += 1;
            let on = allocated.command.throttle_frac > 0.0;
            if a.previous_engine_on.is_some_and(|previous| previous != on) {
                a.engine_switches += 1;
            }
            a.previous_engine_on = Some(on);
            if allocated.regime == crate::arrival_actuation::AllocationRegime::Tracking {
                a.maximum_tracking_scalar_error_mps2 = a
                    .maximum_tracking_scalar_error_mps2
                    .max(allocated.aligned_error_mps2.abs());
            }
            a.last = Some(allocated);
            a.ideal_contract_pass = state.prediction.assessment.contract_pass();
            a.actuator_forecast_contract_pass = state
                .reachable_prediction
                .prediction
                .assessment
                .contract_pass();
            a.forecast_time_s = state.reachable_prediction.prediction.time_to_event_s;
            a.ideal_pass_ever = self.kernel.waypoint_reference_contract_pass_ever;
            a.operation_remaining_s =
                (request.deadline_physics_step - o.physics_step) as f64 / 120.0;
        }
        self.feedback.replans = self.kernel.waypoint_guidance_replan_count;
        self.feedback.model_steps = self
            .kernel
            .arrival_model_steps
            .load(std::sync::atomic::Ordering::Relaxed);
        self.feedback.deadline_remaining_s = Some(state.deadline_remaining_s);
        self.feedback.command_updates += 1;
        self.feedback.tilt_saturated_updates += u64::from(state.tilt_saturated);
        self.feedback.thrust_saturated_updates += u64::from(state.thrust_saturated);
        let mut frame = ControllerFrame::command_only(state.command);
        frame.phase = Some("airborne_arrival".into());
        for (key, value) in [
            ("arrival_distance_m", stats.distance_m),
            ("arrival_velocity_error_mps", state.velocity_error_mps),
            ("arrival_horizon_s", state.deadline_remaining_s),
            ("arrival_required_accel_ratio", state.required_accel_ratio),
            ("arrival_target_x_m", request.waypoint.position_m.x),
            ("arrival_target_y_m", request.waypoint.position_m.y),
        ] {
            frame
                .metrics
                .insert(key.into(), TelemetryValue::Float(value));
        }
        if let Some(a) = &self.feedback.actuation
            && a.profile == crate::arrival_actuation::ArrivalActuationProfile::NativePairV2
        {
            let allocation = a.last.unwrap();
            for (key, value) in [
                (
                    "arrival_projected_ax_mps2",
                    allocation.projected_accel_mps2.x,
                ),
                (
                    "arrival_projected_ay_mps2",
                    allocation.projected_accel_mps2.y,
                ),
                (
                    "arrival_aligned_mean_accel_mps2",
                    allocation.aligned_mean_accel_mps2,
                ),
                (
                    "arrival_allocation_error_mps2",
                    allocation.aligned_error_mps2,
                ),
            ] {
                frame
                    .metrics
                    .insert(key.into(), TelemetryValue::Float(value));
            }
        }
        self.next_tick += 2;
        Ok(ArrivalFrame::Command(frame))
    }

    pub fn feedback(&self) -> &ArrivalFeedback {
        &self.feedback
    }

    pub fn native_end(&mut self, o: &Observation) {
        self.closed = true;
        if self.feedback.status == ArrivalStatus::Active {
            self.feedback.status = ArrivalStatus::NativeEnd;
            self.feedback.resolution = Some(o.clone());
        }
    }
}
