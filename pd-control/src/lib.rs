use pd_core::{Command, Observation, RunArtifacts, RunContext, SimulationError, run_simulation};

pub trait Controller {
    fn id(&self) -> &str;

    fn reset(&mut self, _ctx: &RunContext) {}

    fn update(&mut self, ctx: &RunContext, observation: &Observation) -> Command;
}

#[derive(Debug, Default)]
pub struct IdleController;

impl Controller for IdleController {
    fn id(&self) -> &str {
        "idle"
    }

    fn update(&mut self, _ctx: &RunContext, _observation: &Observation) -> Command {
        Command::idle()
    }
}

#[derive(Debug, Default)]
pub struct BaselineController;

impl Controller for BaselineController {
    fn id(&self) -> &str {
        "baseline_v1"
    }

    fn update(&mut self, ctx: &RunContext, observation: &Observation) -> Command {
        let max_accel_mps2 = ctx.vehicle.max_thrust_n / observation.mass_kg.max(1.0);
        let hover_throttle = observation.gravity_mps2 / max_accel_mps2.max(f64::EPSILON);
        let altitude_m = observation.touchdown_clearance_m.max(0.0);

        let desired_vx_mps = (observation.target_dx_m * 0.08).clamp(-5.0, 5.0);
        let raw_attitude_rad =
            ((desired_vx_mps - observation.velocity_mps.x) * 0.08).clamp(-0.45, 0.45);
        let attitude_limit_rad = if altitude_m > 40.0 {
            0.45
        } else if altitude_m > 15.0 {
            0.25
        } else {
            0.12
        };
        let target_attitude_rad = raw_attitude_rad.clamp(-attitude_limit_rad, attitude_limit_rad);

        let desired_vy_mps = if altitude_m > 80.0 {
            -18.0
        } else if altitude_m > 30.0 {
            -10.0
        } else if altitude_m > 12.0 {
            -5.0
        } else {
            -2.0
        };
        let vertical_error_mps = desired_vy_mps - observation.velocity_mps.y;
        let throttle_frac =
            (hover_throttle + (vertical_error_mps * 0.09) + target_attitude_rad.abs() * 0.04)
                .clamp(0.0, 1.0);

        Command {
            throttle_frac,
            target_attitude_rad,
        }
    }
}

pub fn run_controller(
    ctx: &RunContext,
    controller: &mut dyn Controller,
) -> Result<RunArtifacts, SimulationError> {
    controller.reset(ctx);
    let controller_id = controller.id().to_owned();
    run_simulation(ctx, &controller_id, |ctx, observation| {
        controller.update(ctx, observation)
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use pd_core::{
        EndReason, EvaluationGoal, LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig,
        TerrainDefinition, Vec2, VehicleGeometry, VehicleInitialState, VehicleSpec, WorldSpec,
    };

    fn flat_scenario() -> ScenarioSpec {
        ScenarioSpec {
            id: "controller_smoke".to_owned(),
            name: "Controller smoke".to_owned(),
            description: "controller smoke".to_owned(),
            seed: 3,
            tags: vec!["test".to_owned(), "landing".to_owned()],
            metadata: BTreeMap::from([("suite".to_owned(), "control".to_owned())]),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 45.0,
                sample_hz: Some(10),
            },
            world: WorldSpec {
                gravity_mps2: 1.62,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-120.0, 0.0), Vec2::new(120.0, 0.0)],
                },
                landing_pads: vec![LandingPadSpec {
                    id: "pad_a".to_owned(),
                    center_x_m: 0.0,
                    surface_y_m: 0.0,
                    width_m: 36.0,
                }],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 4.0,
                    hull_height_m: 6.0,
                    touchdown_half_span_m: 2.0,
                    touchdown_base_offset_m: 3.2,
                },
                dry_mass_kg: 700.0,
                initial_fuel_kg: 240.0,
                max_fuel_kg: 240.0,
                max_thrust_n: 16_000.0,
                max_fuel_burn_kgps: 11.0,
                max_rotation_rate_radps: 1.2,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(18.0, 140.0),
                velocity_mps: Vec2::new(-1.0, -12.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                goal: EvaluationGoal::LandingOnPad {
                    target_pad_id: "pad_a".to_owned(),
                },
            },
        }
    }

    #[test]
    fn baseline_controller_emits_bounded_commands() {
        let ctx = RunContext::from_scenario(&flat_scenario()).unwrap();
        let observation = pd_core::SimulationState::new(&ctx)
            .unwrap()
            .build_observation(&ctx);
        let mut controller = BaselineController;
        let command = controller.update(&ctx, &observation);

        assert!((0.0..=1.0).contains(&command.throttle_frac));
        assert!(command.target_attitude_rad.is_finite());
    }

    #[test]
    fn baseline_controller_lands_flat_fixture() {
        let ctx = RunContext::from_scenario(&flat_scenario()).unwrap();
        let mut controller = BaselineController;
        let artifacts = run_controller(&ctx, &mut controller).unwrap();

        assert!(matches!(
            artifacts.manifest.end_reason,
            EndReason::TouchdownOnTarget
        ));
    }
}
