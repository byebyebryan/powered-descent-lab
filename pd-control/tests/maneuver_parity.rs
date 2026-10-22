//! Predeclared software panel, derived from a tracked fixture without editing it.
//! Comparator: identical episode identity, no timings on either side, exact
//! complete frames, observations, summaries, actions/events/samples and contact.
//! No outcomes or physical fields are excluded and no landing quota is imposed.
use pd_control::{Controller, TerminalPdgController, maneuver::*};
use pd_core::{RunContext, ScenarioSpec, TerrainDefinition, Vec2};
use pd_run::{CaptureModeV1, Episode, scenario_horizon_limits};

fn panel() -> Vec<(&'static str, ScenarioSpec)> {
    let base: ScenarioSpec = serde_json::from_str(include_str!(
        "../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap();
    [
        "nominal",
        "loaded",
        "lateral",
        "attitude",
        "terrain",
        "settling",
        "low-fuel",
        "fuel-exhaustion",
        "short-deadline",
        "ascending",
        "below-target",
    ]
    .into_iter()
    .map(|id| {
        let mut s = base.clone();
        match id {
            "loaded" => s.vehicle.dry_mass_kg = 9450.0,
            "lateral" => {
                s.initial_state.position_m.x = 55.0;
                s.initial_state.velocity_mps.x = -9.0;
            }
            "attitude" => {
                s.initial_state.attitude_rad = 0.3;
                s.initial_state.angular_rate_radps = -0.1;
            }
            "terrain" => {
                s.world.terrain = TerrainDefinition::Heightfield {
                    points_m: vec![
                        Vec2::new(-120.0, 0.0),
                        Vec2::new(-45.0, 14.0),
                        Vec2::new(-20.0, 0.0),
                        Vec2::new(120.0, 0.0),
                    ],
                }
            }
            "settling" => {
                s.initial_state.position_m = Vec2::new(0.3, 5.8);
                s.initial_state.velocity_mps = Vec2::new(0.15, -0.5);
            }
            "low-fuel" => s.vehicle.initial_fuel_kg = 8.0,
            "fuel-exhaustion" => s.vehicle.initial_fuel_kg = 0.008,
            "short-deadline" => s.sim.max_time_s = 3.0 / 120.0,
            "ascending" => s.initial_state.velocity_mps.y = 8.0,
            "below-target" => {
                s.world.landing_pads[0].surface_y_m = 160.0;
            }
            "nominal" => {}
            _ => unreachable!(),
        }
        (id, s)
    })
    .collect()
}

#[test]
fn immediate_adapter_matches_direct_backend_on_frozen_panel() {
    for (id, scenario) in panel() {
        let ctx = RunContext::from_scenario(&scenario).unwrap();
        let limits = scenario_horizon_limits(&ctx).unwrap();
        let mut direct = TerminalPdgController::default();
        direct.reset(&ctx);
        let mut wrapped = TerminalLandingController::new(&scenario).unwrap();
        let request = ManeuverRequestV1 {
            schema_version: 1,
            id: "landing-1".into(),
            goal: ManeuverGoalV1::LandOnMissionTarget {
                target_pad_id: ctx.target_pad.id.clone(),
            },
        };
        assert_eq!(
            wrapped.submit(&request, 0).unwrap(),
            AdmissionV1::Accepted,
            "{id}"
        );
        let mut a = Episode::reset(
            &scenario,
            "maneuver-parity",
            limits,
            CaptureModeV1::Captured,
        )
        .unwrap();
        let mut b = Episode::reset(
            &scenario,
            "maneuver-parity",
            limits,
            CaptureModeV1::Captured,
        )
        .unwrap();
        while a.stop().is_none() {
            assert_eq!(a.observation(), b.observation(), "{id}");
            let original = direct.update(&ctx, &a.observation());
            let facade = wrapped.frame(&b.observation()).unwrap();
            assert_eq!(original, facade, "{id} at {}", a.observation().physics_step);
            assert_eq!(
                a.step_frame(original, None).unwrap(),
                b.step_frame(facade, None).unwrap(),
                "{id}"
            );
        }
        let a = a.finish().unwrap();
        let b = b.finish().unwrap();
        assert_eq!(a, b, "{id}");
        wrapped.native_end((&b.summary).into()).unwrap();
        assert_eq!(
            wrapped.feedback().backend_updates,
            b.summary.manifest.controller_updates,
            "{id}"
        );
    }
}
