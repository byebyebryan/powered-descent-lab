//! Native forecast diagnostic for the direct baseline's first-feasible bias.
//! Predicted contacts are not sealed native Episode landing claims.
use pd_control::maneuver::*;
use pd_core::{Command, ContactClassification, RunContext, ScenarioSpec, SimulationState};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("requires a scenario")?;
    let scenario: ScenarioSpec = serde_json::from_slice(&std::fs::read(path)?)?;
    let ctx = RunContext::from_scenario(&scenario)?;
    let deadline = (ctx.sim.max_time_s * 120.0) as u64;
    let mut rows = vec![];
    let mut steps = 0_u64;
    for coast in [0_u64, 60, 120, 240, 360, 480, 720] {
        for duration in [720_u64, 960, 1200, 1440, 1920, 2400, 2880, 3600, 4320] {
            let end = coast + duration;
            if end + 4 > deadline {
                continue;
            }
            let mut initial = SimulationState::new(&ctx)?;
            let mut contact = ContactClassification::None;
            for _ in 0..coast {
                initial.set_command(Command::idle());
                contact = initial.step_physics_and_classify_contact(&ctx);
                steps += 1;
                if contact != ContactClassification::None {
                    break;
                }
            }
            if contact != ContactClassification::None {
                continue;
            }
            let request = ManeuverRequestV2 {
                schema_version: 2,
                id: "direct-grid-diagnostic".into(),
                goal: ManeuverGoalV2::LandOnMissionTargetAt {
                    target_pad_id: ctx.target_pad.id.clone(),
                    reference_end_physics_step: end,
                },
            };
            let mut burn = PolynomialLandingController::new_checked(&scenario, coast, deadline)
                .map_err(|e| format!("{e:?}"))?;
            if burn.submit(&request, &initial.build_observation(&ctx))? != AdmissionV2::Accepted {
                continue;
            }
            let accepted = burn.feedback().accepted.unwrap();
            let thrust = pd_control::poly_landing::paired_mean(&accepted.reference, &ctx, 0);
            let attitude = thrust.x.atan2(thrust.y);
            let mut state = SimulationState::new(&ctx)?;
            for _ in 0..coast {
                state.set_command(Command {
                    throttle_frac: 0.0,
                    target_attitude_rad: attitude,
                });
                contact = state.step_physics_and_classify_contact(&ctx);
                steps += 1;
                if contact != ContactClassification::None {
                    break;
                }
            }
            if contact != ContactClassification::None {
                continue;
            }
            let mut burn = PolynomialLandingController::new_checked(&scenario, coast, deadline)
                .map_err(|e| format!("{e:?}"))?;
            if burn.submit(&request, &state.build_observation(&ctx))? != AdmissionV2::Accepted {
                continue;
            }
            while state.physics_step < end + 4 && contact == ContactClassification::None {
                let ManeuverFrameV2::Command(frame) = burn.frame(&state.build_observation(&ctx))?
                else {
                    break;
                };
                state.set_command(frame.command);
                for _ in 0..2 {
                    contact = state.step_physics_and_classify_contact(&ctx);
                    steps += 1;
                    if contact != ContactClassification::None {
                        break;
                    }
                }
            }
            rows.push(
                serde_json::json!({"coast_ticks":coast,"duration_ticks":duration,
                "contact":contact,"fuel_used_kg":ctx.vehicle.initial_fuel_kg-state.fuel_kg,
                "contact_tick":state.physics_step}),
            );
        }
    }
    println!(
        "{}",
        serde_json::json!({"rows":rows,"native_forecast_steps":steps,
        "scope":"full direct grid forecast only; no sealed Episode capture or optimality outside this grid"})
    );
    Ok(())
}
