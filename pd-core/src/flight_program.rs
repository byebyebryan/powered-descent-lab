use serde::{Deserialize, Serialize};

use crate::{
    Command, EvaluationGoal, MissionSpec, RunContext, SimConfig, VehicleInitialState, VehicleSpec,
    WorldSpec,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlightProgramBindingV1 {
    pub sim: SimConfig,
    pub world: WorldSpec,
    pub vehicle: VehicleSpec,
    pub initial_state: VehicleInitialState,
    pub mission: MissionSpec,
}

impl FlightProgramBindingV1 {
    pub fn from_context(context: &RunContext) -> Self {
        Self {
            sim: context.sim.clone(),
            world: context.world.clone(),
            vehicle: context.vehicle.clone(),
            initial_state: context.initial_state.clone(),
            mission: context.mission.clone(),
        }
    }

    fn validate_components(&self) -> Result<(), String> {
        self.sim.validate()?;
        self.world.validate()?;
        self.vehicle.validate()?;
        self.initial_state.validate()?;
        self.mission.validate()?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlightProgramUpdateV1 {
    /// Physics tick observed immediately before this command is applied.
    pub physics_step: u64,
    pub phase: String,
    /// Throttle is in [0, 1] and target attitude is canonical in [-pi, pi].
    pub command: Command,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlightProgramV1 {
    pub schema_version: u32,
    pub binding: FlightProgramBindingV1,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub generation_policy_identity: String,
    pub terminal_policy_identity: String,
    pub witness_identity: String,
    pub source_handoff_physics_step: u64,
    pub terminal_entry_physics_step: u64,
    pub planned_end_physics_step: u64,
    pub expected_contact_physics_step: u64,
    pub updates: Vec<FlightProgramUpdateV1>,
}

impl FlightProgramV1 {
    pub fn validate_against_context(&self, context: &RunContext) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported flight program schema_version {}",
                self.schema_version
            ));
        }

        self.binding.validate_components()?;
        validate_context_components(context)?;

        if self.binding != FlightProgramBindingV1::from_context(context) {
            return Err("flight program binding does not match run context".to_owned());
        }

        let context_target = context
            .world
            .landing_pad(context.mission.goal.target_pad_id())
            .ok_or_else(|| "run context mission target pad is unresolved".to_owned())?;
        if context.target_pad != *context_target {
            return Err("run context target_pad does not match its mission target".to_owned());
        }

        self.validate_pad_bindings()?;
        self.validate_timing()?;
        self.validate_updates()
    }

    fn validate_pad_bindings(&self) -> Result<(), String> {
        for (label, id) in [
            ("source_pad_id", self.source_pad_id.as_str()),
            ("target_pad_id", self.target_pad_id.as_str()),
            (
                "generation_policy_identity",
                self.generation_policy_identity.as_str(),
            ),
            (
                "terminal_policy_identity",
                self.terminal_policy_identity.as_str(),
            ),
            ("witness_identity", self.witness_identity.as_str()),
        ] {
            if id.trim().is_empty() {
                return Err(format!("flight program {label} must not be empty"));
            }
        }
        if self.source_pad_id == self.target_pad_id {
            return Err("flight program source and target pad IDs must differ".to_owned());
        }

        resolve_unique_pad(&self.binding.world, &self.source_pad_id, "source")?;
        resolve_unique_pad(&self.binding.world, &self.target_pad_id, "target")?;

        let EvaluationGoal::LandingOnPad { target_pad_id } = &self.binding.mission.goal else {
            return Err("flight program binding mission goal must be landing_on_pad".to_owned());
        };
        if target_pad_id != &self.target_pad_id {
            return Err("flight program target pad does not match landing goal".to_owned());
        }

        if let Some(route) = &self.binding.mission.transfer_route {
            if !route.waypoints.is_empty() {
                return Err(
                    "flight program binding must not contain authored route waypoints".to_owned(),
                );
            }
            if route.source_pad_id != self.source_pad_id
                || route.target_pad_id != self.target_pad_id
            {
                return Err("flight program pad IDs do not match transfer route pads".to_owned());
            }
        }
        Ok(())
    }

    fn validate_timing(&self) -> Result<(), String> {
        let interval = self.binding.sim.control_interval_steps();
        if self.source_handoff_physics_step == 0 {
            return Err("source_handoff_physics_step must be > 0".to_owned());
        }
        if self.terminal_entry_physics_step <= self.source_handoff_physics_step {
            return Err(
                "terminal_entry_physics_step must be after source_handoff_physics_step".to_owned(),
            );
        }
        if !self.terminal_entry_physics_step.is_multiple_of(interval) {
            return Err(
                "terminal_entry_physics_step must be on the global control clock".to_owned(),
            );
        }
        if self.expected_contact_physics_step <= self.terminal_entry_physics_step {
            return Err("expected_contact_physics_step must be after terminal entry".to_owned());
        }
        if self.expected_contact_physics_step > self.planned_end_physics_step {
            return Err("expected contact must not exceed planned end".to_owned());
        }

        let horizon_product = self.binding.sim.max_time_s * f64::from(self.binding.sim.physics_hz);
        if !horizon_product.is_finite() || horizon_product.ceil() > u64::MAX as f64 {
            return Err("simulation horizon is not representable in physics steps".to_owned());
        }
        let horizon_physics_steps = horizon_product.ceil() as u64;
        if self.planned_end_physics_step > horizon_physics_steps {
            return Err("planned end exceeds the simulation horizon".to_owned());
        }
        Ok(())
    }

    fn validate_updates(&self) -> Result<(), String> {
        let interval = self.binding.sim.control_interval_steps();
        let contact_step = self.expected_contact_physics_step;
        let expected_count =
            contact_step / interval + u64::from(!contact_step.is_multiple_of(interval));
        let actual_count = u64::try_from(self.updates.len())
            .map_err(|_| "flight program update count is not representable".to_owned())?;
        if actual_count != expected_count {
            return Err(format!(
                "flight program must contain {expected_count} updates before expected contact, got {actual_count}"
            ));
        }

        for (index, update) in self.updates.iter().enumerate() {
            let update_index = u64::try_from(index)
                .map_err(|_| "flight program update index is not representable".to_owned())?;
            let expected_step = update_index
                .checked_mul(interval)
                .ok_or_else(|| "flight program update tick overflow".to_owned())?;
            if update.physics_step != expected_step {
                return Err(format!(
                    "flight program update {index} must use physics_step {expected_step}, got {}",
                    update.physics_step
                ));
            }
            if update.phase.trim().is_empty() {
                return Err(format!(
                    "flight program update {index} phase must not be empty"
                ));
            }
            if !matches!(
                update.phase.as_str(),
                "upright" | "tilt" | "source_bridge" | "ballistic_coast" | "terminal_bridge"
            ) {
                return Err(format!(
                    "flight program update {index} has unknown phase '{}'",
                    update.phase
                ));
            }
            if !update.command.throttle_frac.is_finite()
                || !update.command.target_attitude_rad.is_finite()
            {
                return Err(format!(
                    "flight program update {index} command values must be finite"
                ));
            }
            if !(0.0..=1.0).contains(&update.command.throttle_frac) {
                return Err(format!(
                    "flight program update {index} throttle_frac must be within [0, 1]"
                ));
            }
            if !(-std::f64::consts::PI..=std::f64::consts::PI)
                .contains(&update.command.target_attitude_rad)
            {
                return Err(format!(
                    "flight program update {index} target_attitude_rad must be within [-pi, pi]"
                ));
            }
        }
        Ok(())
    }
}

fn validate_context_components(context: &RunContext) -> Result<(), String> {
    context.sim.validate()?;
    context.world.validate()?;
    context.vehicle.validate()?;
    context.initial_state.validate()?;
    context.mission.validate()?;
    Ok(())
}

fn resolve_unique_pad<'a>(
    world: &'a WorldSpec,
    pad_id: &str,
    label: &str,
) -> Result<&'a crate::LandingPadSpec, String> {
    let mut matches = world.landing_pads.iter().filter(|pad| pad.id == pad_id);
    let Some(pad) = matches.next() else {
        return Err(format!(
            "flight program {label} pad '{pad_id}' is unresolved"
        ));
    };
    if matches.next().is_some() {
        return Err(format!(
            "flight program {label} pad '{pad_id}' is ambiguous"
        ));
    }
    Ok(pad)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        LandingPadSpec, ScenarioSpec, TerrainDefinition, TransferRouteSpec, Vec2, VehicleGeometry,
    };

    use super::*;

    fn fixture() -> (RunContext, FlightProgramV1) {
        let scenario = ScenarioSpec {
            id: "flight_program_test".to_owned(),
            name: "Flight program test".to_owned(),
            description: "small complete-program contract fixture".to_owned(),
            seed: 1,
            tags: vec!["test".to_owned()],
            metadata: BTreeMap::new(),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 10.0,
                sample_hz: Some(10),
            },
            world: WorldSpec {
                gravity_mps2: 1.62,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-120.0, 0.0), Vec2::new(120.0, 0.0)],
                },
                landing_pads: vec![
                    LandingPadSpec {
                        id: "source".to_owned(),
                        center_x_m: -40.0,
                        surface_y_m: 0.0,
                        width_m: 36.0,
                    },
                    LandingPadSpec {
                        id: "target".to_owned(),
                        center_x_m: 0.0,
                        surface_y_m: 0.0,
                        width_m: 36.0,
                    },
                ],
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
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.2,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(-40.0, 10.0),
                velocity_mps: Vec2::new(0.0, -1.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                transfer_route: Some(TransferRouteSpec {
                    source_pad_id: "source".to_owned(),
                    target_pad_id: "target".to_owned(),
                    route_angle_deg: 0.0,
                    route_radius_m: 40.0,
                    waypoints: Vec::new(),
                }),
                goal: EvaluationGoal::LandingOnPad {
                    target_pad_id: "target".to_owned(),
                },
            },
        };
        let context = RunContext::from_scenario(&scenario).unwrap();
        let program = FlightProgramV1 {
            schema_version: 1,
            binding: FlightProgramBindingV1::from_context(&context),
            source_pad_id: "source".to_owned(),
            target_pad_id: "target".to_owned(),
            generation_policy_identity: "generator-v1".to_owned(),
            terminal_policy_identity: "terminal-v1".to_owned(),
            witness_identity: "witness-001".to_owned(),
            source_handoff_physics_step: 3,
            terminal_entry_physics_step: 6,
            planned_end_physics_step: 12,
            expected_contact_physics_step: 10,
            updates: (0..5)
                .map(|index| FlightProgramUpdateV1 {
                    physics_step: index * 2,
                    phase: "upright".to_owned(),
                    command: Command::idle(),
                })
                .collect(),
        };
        (context, program)
    }

    #[test]
    fn complete_program_serialization_round_trips_without_outputs() {
        let (_, program) = fixture();
        let encoded = serde_json::to_string(&program).unwrap();
        let decoded: FlightProgramV1 = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, program);
        assert!(
            serde_json::from_str::<FlightProgramV1>(&encoded.replace(
                "\"schema_version\":1",
                "\"schema_version\":1,\"unexpected\":true"
            ))
            .is_err()
        );
    }

    #[test]
    fn program_rejects_context_binding_and_target_pad_mismatches() {
        let (context, mut program) = fixture();
        program.validate_against_context(&context).unwrap();

        program.binding.initial_state.position_m.x += 1.0;
        assert!(program.validate_against_context(&context).is_err());

        let (_, mut program) = fixture();
        let mut mismatched_context = context;
        mismatched_context.target_pad.center_x_m += 1.0;
        assert!(
            program
                .validate_against_context(&mismatched_context)
                .is_err()
        );

        program.binding.vehicle.max_thrust_n = 0.0;
        assert!(
            program
                .validate_against_context(&mismatched_context)
                .is_err()
        );
    }

    #[test]
    fn program_rejects_nonfinite_and_out_of_range_commands() {
        let (context, mut program) = fixture();
        program.updates[0].command.throttle_frac = f64::NAN;
        assert!(program.validate_against_context(&context).is_err());

        let (context, mut program) = fixture();
        program.updates[0].command.target_attitude_rad = std::f64::consts::PI + 0.01;
        assert!(program.validate_against_context(&context).is_err());
    }

    #[test]
    fn program_rejects_missing_extra_gapped_duplicate_and_off_clock_updates() {
        let (context, mut program) = fixture();
        program.updates.pop();
        assert!(program.validate_against_context(&context).is_err());

        let (context, mut program) = fixture();
        program.updates.push(FlightProgramUpdateV1 {
            physics_step: 10,
            phase: "terminal_bridge".to_owned(),
            command: Command::idle(),
        });
        assert!(program.validate_against_context(&context).is_err());

        let (context, mut program) = fixture();
        program.updates[2].physics_step = 6;
        assert!(program.validate_against_context(&context).is_err());

        let (context, mut program) = fixture();
        program.updates[2].physics_step = 2;
        assert!(program.validate_against_context(&context).is_err());

        let (context, mut program) = fixture();
        program.updates[1].physics_step = 1;
        assert!(program.validate_against_context(&context).is_err());
    }

    #[test]
    fn odd_source_handoff_is_allowed_but_terminal_entry_must_be_on_clock() {
        let (context, mut program) = fixture();
        program.validate_against_context(&context).unwrap();

        program.terminal_entry_physics_step = 7;
        assert!(program.validate_against_context(&context).is_err());
    }

    #[test]
    fn program_rejects_authored_waypoints_and_route_pad_mismatch() {
        let (context, mut program) = fixture();
        program
            .binding
            .mission
            .transfer_route
            .as_mut()
            .unwrap()
            .source_pad_id = "target".to_owned();
        assert!(program.validate_against_context(&context).is_err());

        let (context, mut program) = fixture();
        program
            .binding
            .mission
            .transfer_route
            .as_mut()
            .unwrap()
            .waypoints
            .push(crate::TransferWaypointSpec {
                id: "authored".to_owned(),
                position_m: Vec2::new(-20.0, 10.0),
                handoff_tangent_unit: None,
                capture_radius_m: 1.0,
                max_cross_track_m: 1.0,
                max_outbound_heading_error_rad: 0.1,
                min_outbound_progress_mps: 1.0,
                max_outbound_cross_speed_mps: None,
                min_speed_mps: 0.0,
                max_speed_mps: 3.0,
                min_vertical_speed_mps: None,
                max_vertical_speed_mps: None,
            });
        assert!(program.validate_against_context(&context).is_err());
    }
}
