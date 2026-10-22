use super::*;
use crate::{Controller, ControllerFrame, TerminalPdgController};
use pd_core::{Command, RunContext, ScenarioSpec, SimulationState};

fn scenario() -> ScenarioSpec {
    serde_json::from_str(include_str!(
        "../../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap()
}
fn request(id: &str, pad: &str) -> ManeuverRequestV1 {
    ManeuverRequestV1 {
        schema_version: 1,
        id: id.into(),
        goal: ManeuverGoalV1::LandOnMissionTarget {
            target_pad_id: pad.into(),
        },
    }
}
fn end(tick: u64) -> NativeEndV1 {
    NativeEndV1 {
        schema_version: 1,
        physics_step: tick,
        physical_outcome: pd_core::PhysicalOutcome::Flying,
        mission_outcome: pd_core::MissionOutcome::InProgress,
        stop: pd_core::BoundedRunStopCauseV1::HardDeadlineReached,
        failure: None,
    }
}

#[test]
fn wire_errors_are_separate_from_semantic_rejections() {
    for bytes in [
        br#"{"schema_version":2,"id":"a","goal":{"kind":"cancel"}}"#.as_slice(),
        br#"{"schema_version":1,"id":"a","goal":{"kind":"launch"}}"#,
        br#"{"schema_version":1,"id":"a","goal":{"kind":"cancel","typo":1}}"#,
        br#"{"schema_version":1,"id":"","goal":{"kind":"cancel"}}"#,
    ] {
        assert!(ManeuverRequestV1::from_json(bytes).is_err());
    }
    let req = request("a", "pad_main");
    assert_eq!(
        ManeuverRequestV1::from_json(&serde_json::to_vec(&req).unwrap()).unwrap(),
        req
    );
    let mut s = scenario();
    let mut other = s.world.landing_pads[0].clone();
    other.id = "other".into();
    s.world.landing_pads.push(other);
    let mut c = TerminalLandingController::new(&s).unwrap();
    let cancel = ManeuverRequestV1 {
        goal: ManeuverGoalV1::Cancel {},
        ..req.clone()
    };
    assert!(matches!(
        c.submit(&cancel, 0).unwrap(),
        AdmissionV1::Unsupported { .. }
    ));
    assert!(matches!(
        c.submit(&request("b", "absent"), 0).unwrap(),
        AdmissionV1::InvalidRequest { .. }
    ));
    assert!(matches!(
        c.submit(&request("b", "other"), 0).unwrap(),
        AdmissionV1::Unsupported { .. }
    ));
    assert_eq!(c.feedback().command_updates, 0);
    assert_eq!(c.submit(&req, 0).unwrap(), AdmissionV1::Accepted);
    assert_eq!(c.submit(&req, 0).unwrap(), AdmissionV1::AlreadyActive);
    assert!(matches!(
        c.submit(&cancel, 0).unwrap(),
        AdmissionV1::InvalidRequest { .. }
    ));
    assert_eq!(
        c.submit(&request("new", "pad_main"), 0).unwrap(),
        AdmissionV1::Busy
    );
    assert_eq!(c.feedback().backend_updates, 0);
}

#[test]
fn coast_duplicates_rejections_and_queries_do_not_advance_or_reset_backend() {
    let s = scenario();
    let ctx = RunContext::from_scenario(&s).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    let mut c = TerminalLandingController::new(&s).unwrap();
    assert_eq!(
        c.frame(&state.build_observation(&ctx)).unwrap().command,
        Command::idle()
    );
    assert!(c.frame(&state.build_observation(&ctx)).is_err());
    let req = request("a", "pad_main");
    let before = c.feedback();
    assert!(matches!(
        c.submit(&req, 0).unwrap(),
        AdmissionV1::InvalidRequest { .. }
    ));
    assert_eq!(before, c.feedback());
    state.physics_step = 2;
    state.sim_time_s = 2.0 / 120.0;
    assert_eq!(c.submit(&req, 2).unwrap(), AdmissionV1::Accepted);
    let mut reference = TerminalPdgController::default();
    reference.reset(&ctx);
    for tick in [2, 4, 6] {
        state.physics_step = tick;
        state.sim_time_s = tick as f64 / 120.0;
        let before = c.feedback();
        assert_eq!(c.submit(&req, tick).unwrap(), AdmissionV1::AlreadyActive);
        assert_eq!(
            c.submit(&request("b", "pad_main"), tick).unwrap(),
            AdmissionV1::Busy
        );
        assert_eq!(before, c.feedback());
        let obs = state.build_observation(&ctx);
        assert_eq!(c.frame(&obs).unwrap(), reference.update(&ctx, &obs));
    }
    c.native_end(end(7)).unwrap();
    assert!(c.frame(&state.build_observation(&ctx)).is_err());
    assert!(c.submit(&req, 8).is_err());
    assert!(c.native_end(end(8)).is_err());
}

#[test]
fn semantic_scope_is_distinct_from_difficulty_and_configuration_is_frozen() {
    let base = scenario();
    for mutation in [0, 1, 2] {
        let mut s = base.clone();
        match mutation {
            0 => s.vehicle.initial_fuel_kg = 0.008,
            1 => s.initial_state.velocity_mps.y = 8.0,
            _ => s.world.landing_pads[0].surface_y_m = 160.0,
        }
        let mut c = TerminalLandingController::new(&s).unwrap();
        assert_eq!(
            c.submit(&request("a", "pad_main"), 0).unwrap(),
            AdmissionV1::Accepted
        );
        assert_eq!(
            c.descriptor().backend.configuration,
            serde_json::to_value(TerminalPdgController::default().resolved_configuration())
                .unwrap()
        );
    }
    for mutation in [0, 1, 2, 3, 4, 5] {
        let mut s = base.clone();
        match mutation {
            0 => s.world.gravity_mps2 = 1.62,
            1 => s.sim.controller_hz = 30,
            2 => s.vehicle.max_thrust_n = 240001.0,
            3 => s.initial_state.position_m.y = 5.0,
            4 => {
                s.mission.goal = pd_core::EvaluationGoal::TimedCheckpoint {
                    target_pad_id: "pad_main".into(),
                    end_time_s: 1.0,
                    desired_position_offset_m: pd_core::Vec2::new(0.0, 140.0),
                    max_position_error_m: 1.0,
                    desired_velocity_mps: pd_core::Vec2::new(0.0, 0.0),
                    max_velocity_error_mps: 1.0,
                    max_attitude_error_rad: 0.1,
                }
            }
            _ => {
                let mut source = s.world.landing_pads[0].clone();
                source.id = "source".into();
                s.world.landing_pads.push(source);
                s.mission.transfer_route = Some(pd_core::TransferRouteSpec {
                    source_pad_id: "source".into(),
                    target_pad_id: "pad_main".into(),
                    route_angle_deg: 0.0,
                    route_radius_m: 50.0,
                    waypoints: vec![],
                });
            }
        }
        assert!(matches!(
            TerminalLandingController::new(&s),
            Err(ManeuverInputErrorV1::UnsupportedInput(_))
        ));
    }
    let mut invalid = base.clone();
    invalid.vehicle.initial_fuel_kg = 0.0;
    assert!(matches!(
        TerminalLandingController::new(&invalid),
        Err(ManeuverInputErrorV1::InvalidInput(_))
    ));
    let mut c = TerminalLandingController::new(&base).unwrap();
    assert!(c.native_end(end(1)).is_err());
    c.native_end(end(0)).unwrap();
    assert_eq!(c.feedback().accepted, None);
}

// Independent deterministic test implementation: no terminal math or state.
struct TestController {
    lifecycle: Lifecycle,
}
impl ManeuverController for TestController {
    fn descriptor(&self) -> ManeuverDescriptorV1 {
        ManeuverDescriptorV1 {
            schema_version: 1,
            implementation: "test-only-coast".into(),
            capability: "synthetic lifecycle only".into(),
            physics_hz: 120,
            controller_hz: 60,
            backend: ManeuverBackendDescriptorV1 {
                implementation: "synthetic-coast".into(),
                configuration: serde_json::json!({"throttle_frac":0.0,"target_attitude_rad":0.0}),
            },
            coast: "idle".into(),
            command_mapping: "identity".into(),
        }
    }
    fn submit(&mut self, req: &ManeuverRequestV1, tick: u64) -> Result<AdmissionV1, String> {
        self.lifecycle.live()?;
        req.validate()?;
        if tick != self.lifecycle.next_tick {
            return Ok(AdmissionV1::InvalidRequest {
                reason: "tick".into(),
            });
        }
        Ok(self.lifecycle.accept(req, tick))
    }
    fn frame(&mut self, obs: &pd_core::Observation) -> Result<ControllerFrame, String> {
        self.lifecycle.check_frame(obs)?;
        self.lifecycle
            .produced(self.lifecycle.feedback.accepted.is_some());
        Ok(ControllerFrame::command_only(Command::idle()))
    }
    fn feedback(&self) -> ManeuverFeedbackV1 {
        self.lifecycle.feedback.clone()
    }
    fn native_end(&mut self, end: NativeEndV1) -> Result<(), String> {
        self.lifecycle.close(end)
    }
}
#[test]
fn alternative_implementation_uses_the_public_trait_without_terminal_backend() {
    let mut c: Box<dyn ManeuverController> = Box::new(TestController {
        lifecycle: Lifecycle::new(2),
    });
    assert_eq!(c.descriptor().backend.implementation, "synthetic-coast");
    assert_eq!(
        c.submit(&request("test", "pad_main"), 0).unwrap(),
        AdmissionV1::Accepted
    );
    let ctx = RunContext::from_scenario(&scenario()).unwrap();
    c.frame(&SimulationState::new(&ctx).unwrap().build_observation(&ctx))
        .unwrap();
    c.native_end(end(1)).unwrap();
    assert!(c.feedback().native_end.is_some());
}
