use super::*;
use crate::poly_landing::{LandingKinematicsV1, build_reference, paired_mean};
use pd_core::{Command, RunContext, ScenarioSpec, SimulationState, Vec2};

fn scenario() -> ScenarioSpec {
    let mut s: ScenarioSpec = serde_json::from_str(include_str!(
        "../../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap();
    s.vehicle.initial_fuel_kg = 5806.024;
    s.initial_state.position_m = Vec2::new(-317.915, 197.882);
    s.initial_state.velocity_mps = Vec2::new(62.85, -13.552);
    s
}
fn request(tick: u64) -> ManeuverRequestV2 {
    ManeuverRequestV2 {
        schema_version: 2,
        id: "timed-landing".into(),
        goal: ManeuverGoalV2::LandOnMissionTargetAt {
            target_pad_id: "pad_main".into(),
            reference_end_physics_step: tick,
        },
    }
}
fn controller(s: &ScenarioSpec) -> PolynomialLandingController {
    PolynomialLandingController::new(s, 0, 5400).unwrap()
}

#[test]
fn timed_wire_has_no_v1_alias_or_silent_backend_upgrade() {
    let req = request(1740);
    let bytes = serde_json::to_vec(&req).unwrap();
    assert_eq!(ManeuverRequestV2::from_json(&bytes).unwrap(), req);
    assert!(ManeuverRequestV1::from_json(&bytes).is_err());
    for bytes in [
        br#"{"schema_version":1,"id":"a","goal":{"kind":"land_on_mission_target_at","target_pad_id":"pad_main","reference_end_physics_step":1740}}"#.as_slice(),
        br#"{"schema_version":2,"id":"a","goal":{"kind":"land_on_mission_target","target_pad_id":"pad_main"}}"#,
        br#"{"schema_version":2,"id":"a","goal":{"kind":"cancel","unknown":true}}"#,
    ] { assert!(ManeuverRequestV2::from_json(bytes).is_err()); }
    assert_eq!(
        controller(&scenario()).descriptor().backend.implementation,
        POLY_LANDING_BACKEND
    );
    assert_eq!(
        TerminalLandingController::new(&scenario())
            .unwrap()
            .descriptor()
            .backend
            .implementation,
        "ordinary-terminal-pdg-v1"
    );
}

#[test]
fn timing_identity_and_observation_rejections_cannot_reset_a_burn() {
    let s = scenario();
    let ctx = RunContext::from_scenario(&s).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    let mut c = controller(&s);
    let obs = state.build_observation(&ctx);
    assert!(c.frame(&obs).is_err());
    for tick in [0, 1, 1739, 5400, u64::MAX] {
        assert!(matches!(
            c.submit(&request(tick), &obs).unwrap(),
            AdmissionV2::InvalidRequest { .. }
        ));
    }
    assert_eq!(c.feedback().constructions, 0);
    let req = request(1740);
    assert_eq!(c.submit(&req, &obs).unwrap(), AdmissionV2::Accepted);
    let bound = c.feedback();
    assert_eq!(c.submit(&req, &obs).unwrap(), AdmissionV2::AlreadyActive);
    assert!(matches!(
        c.submit(&request(1800), &obs).unwrap(),
        AdmissionV2::InvalidRequest { .. }
    ));
    let mut other = req.clone();
    other.id = "another".into();
    assert_eq!(c.submit(&other, &obs).unwrap(), AdmissionV2::Busy);
    assert_eq!(c.feedback(), bound);
    let mut changed = obs.clone();
    changed.position_m.y += 1.0;
    assert!(c.frame(&changed).is_err());
    let ManeuverFrameV2::Command(frame) = c.frame(&obs).unwrap() else {
        panic!()
    };
    assert!(frame.command.throttle_frac > 0.0);
    assert!(c.frame(&obs).is_err());
    assert!(matches!(
        c.submit(&req, &obs).unwrap(),
        AdmissionV2::InvalidRequest { .. }
    ));
    state.set_command(frame.command);
    for _ in 0..2 {
        let _ = state.step_physics_and_classify_contact(&ctx);
    }
    let next = state.build_observation(&ctx);
    assert_eq!(c.submit(&req, &next).unwrap(), AdmissionV2::AlreadyActive);
    assert_eq!(c.feedback().constructions, 1);
    assert_eq!(c.feedback().accepted, bound.accepted);
    let end = NativeEndV1 {
        schema_version: 1,
        physics_step: 1,
        physical_outcome: pd_core::PhysicalOutcome::Flying,
        mission_outcome: pd_core::MissionOutcome::InProgress,
        stop: pd_core::BoundedRunStopCauseV1::HardDeadlineReached,
        failure: None,
    };
    c.native_end(end.clone()).unwrap();
    assert!(c.native_end(end).is_err());
    assert!(c.frame(&next).is_err());
    assert!(c.submit(&req, &next).is_err());
}

#[test]
fn valid_but_unconstructible_burn_is_noplan_without_fallback_or_time_advance() {
    let s = scenario();
    let ctx = RunContext::from_scenario(&s).unwrap();
    let obs = SimulationState::new(&ctx).unwrap().build_observation(&ctx);
    let mut c = controller(&s);
    assert!(matches!(
        c.submit(&request(4), &obs).unwrap(),
        AdmissionV2::NoPlan { .. }
    ));
    assert!(c.feedback().accepted.is_none());
    assert_eq!(c.feedback().command_updates, 0);
    assert_eq!(c.feedback().constructions, 1);
    assert!(c.frame(&obs).is_err());
    assert_eq!(
        c.submit(&request(1740), &obs).unwrap(),
        AdmissionV2::Accepted
    );
}

#[test]
fn reference_exhaustion_never_extends_the_clock_or_inserts_settling() {
    // Synthetic observation stream tests the callback boundary only, with no
    // physical landing claim. The immutable reference cannot follow these states.
    let s = scenario();
    let ctx = RunContext::from_scenario(&s).unwrap();
    let mut obs = SimulationState::new(&ctx).unwrap().build_observation(&ctx);
    let mut c = controller(&s);
    assert_eq!(
        c.submit(&request(1740), &obs).unwrap(),
        AdmissionV2::Accepted
    );
    for tick in (0..1744).step_by(2) {
        obs.physics_step = tick;
        obs.sim_time_s = tick as f64 / 120.0;
        assert!(matches!(
            c.frame(&obs).unwrap(),
            ManeuverFrameV2::Command(_)
        ));
    }
    obs.physics_step = 1744;
    obs.sim_time_s = 1744.0 / 120.0;
    assert!(matches!(
        c.frame(&obs).unwrap(),
        ManeuverFrameV2::ReferenceExhausted
    ));
    assert!(c.feedback().reference_exhausted);
    assert_eq!(c.feedback().command_updates, 872);
    assert_eq!(c.feedback().constructions, 1);
    assert!(matches!(
        c.frame(&obs).unwrap(),
        ManeuverFrameV2::ReferenceExhausted
    ));
    assert_eq!(c.feedback().command_updates, 872);
}

#[test]
fn live_airborne_admission_can_follow_an_original_ground_launch() {
    let mut s = scenario();
    s.initial_state.position_m = Vec2::new(0.0, 5.0);
    s.initial_state.velocity_mps = Vec2::new(0.0, 0.0);
    s.initial_state.attitude_rad = 0.0;
    let ctx = RunContext::from_scenario(&s).unwrap();
    let mut state = SimulationState::new(&ctx).unwrap();
    state.set_command(Command {
        throttle_frac: 1.0,
        target_attitude_rad: 0.0,
    });
    for _ in 0..2 {
        let _ = state.step_physics_and_classify_contact(&ctx);
    }
    let mut c = PolynomialLandingController::new(&s, 2, 5400).unwrap();
    let obs = state.build_observation(&ctx);
    assert_eq!(
        c.submit(&request(1202), &obs).unwrap(),
        AdmissionV2::Accepted
    );
    assert_eq!(c.feedback().accepted.unwrap().entry.physics_step, 2);
    assert!(matches!(
        c.frame(&obs).unwrap(),
        ManeuverFrameV2::Command(_)
    ));
}

#[test]
fn prepared_native_smoke_has_one_continuous_burn_and_no_reclimb() {
    let mut s = scenario();
    let ctx = RunContext::from_scenario(&s).unwrap();
    let reference = build_reference(
        &ctx,
        LandingKinematicsV1 {
            position_m: s.initial_state.position_m,
            velocity_mps: s.initial_state.velocity_mps,
        },
        LandingKinematicsV1 {
            position_m: Vec2::new(0.0, 4.995),
            velocity_mps: Vec2::new(0.0, -1.5),
        },
        1740,
    )
    .unwrap();
    let thrust = paired_mean(&reference, &ctx, 0);
    s.initial_state.attitude_rad = thrust.x.atan2(thrust.y);
    let ctx = RunContext::from_scenario(&s).unwrap();
    let mut episode = pd_run::Episode::reset(
        &s,
        "poly-unit-smoke",
        pd_run::scenario_horizon_limits(&ctx).unwrap(),
        pd_run::CaptureModeV1::Summary,
    )
    .unwrap();
    let mut c = controller(&s);
    assert_eq!(
        c.submit(&request(1740), &episode.observation()).unwrap(),
        AdmissionV2::Accepted
    );
    while episode.stop().is_none() {
        let obs = episode.observation();
        assert!(obs.velocity_mps.y <= 0.0);
        let ManeuverFrameV2::Command(frame) = c.frame(&obs).unwrap() else {
            panic!("reference exhausted")
        };
        assert!(frame.command.throttle_frac > 0.0);
        episode.step_frame(frame, None).unwrap();
    }
    let result = episode.finish().unwrap();
    c.native_end((&result.summary).into()).unwrap();
    assert_eq!(
        result.summary.manifest.physical_outcome,
        pd_core::PhysicalOutcome::LandedOnTarget
    );
    assert_eq!(
        result.summary.manifest.mission_outcome,
        pd_core::MissionOutcome::Success
    );
    assert_eq!(c.feedback().constructions, 1);
}
