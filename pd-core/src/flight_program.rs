//! Current timed-command updates and read-only historical complete-program DTOs.
//! Decoding a binding/program does not admit execution. Current V2 execution and
//! replay validate their owned schedules and proofs in `pd-eval`.

use serde::{Deserialize, Serialize};

use crate::{Command, MissionSpec, SimConfig, VehicleInitialState, VehicleSpec, WorldSpec};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlightProgramBindingV1 {
    pub sim: SimConfig,
    pub world: WorldSpec,
    pub vehicle: VehicleSpec,
    pub initial_state: VehicleInitialState,
    pub mission: MissionSpec,
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

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::FlightProgramV1;

    fn historical_program_fixture() -> Value {
        json!({
            "schema_version": 1,
            "binding": {
                "sim": {
                    "physics_hz": 120,
                    "controller_hz": 60,
                    "max_time_s": 10.0,
                    "sample_hz": 10
                },
                "world": {
                    "gravity_mps2": 1.62,
                    "terrain": {
                        "kind": "heightfield",
                        "points_m": [
                            { "x": -1.0, "y": 0.0 },
                            { "x": 1.0, "y": 0.0 }
                        ]
                    },
                    "landing_pads": []
                },
                "vehicle": {
                    "geometry": {
                        "hull_width_m": 4.0,
                        "hull_height_m": 6.0,
                        "touchdown_half_span_m": 2.0,
                        "touchdown_base_offset_m": 3.2
                    },
                    "dry_mass_kg": 700.0,
                    "initial_fuel_kg": 240.0,
                    "max_fuel_kg": 240.0,
                    "max_thrust_n": 16000.0,
                    "max_fuel_burn_kgps": 11.0,
                    "min_throttle_frac": 0.0,
                    "max_rotation_rate_radps": 1.2,
                    "safe_touchdown_normal_speed_mps": 3.0,
                    "safe_touchdown_tangential_speed_mps": 2.0,
                    "safe_touchdown_attitude_error_rad": 0.15,
                    "safe_touchdown_angular_rate_radps": 0.35
                },
                "initial_state": {
                    "position_m": { "x": 0.0, "y": 10.0 },
                    "velocity_mps": { "x": 0.0, "y": -1.0 },
                    "attitude_rad": 0.0,
                    "angular_rate_radps": 0.0
                },
                "mission": {
                    "transfer_route": null,
                    "goal": {
                        "kind": "landing_on_pad",
                        "target_pad_id": "target"
                    }
                }
            },
            "source_pad_id": "source",
            "target_pad_id": "target",
            "generation_policy_identity": "generator-v1",
            "terminal_policy_identity": "terminal-v1",
            "witness_identity": "witness-001",
            "source_handoff_physics_step": 3,
            "terminal_entry_physics_step": 6,
            "planned_end_physics_step": 12,
            "expected_contact_physics_step": 10,
            "updates": [{
                "physics_step": 0,
                "phase": "upright",
                "command": {
                    "throttle_frac": 0.0,
                    "target_attitude_rad": 0.0
                }
            }]
        })
    }

    #[test]
    fn historical_program_dto_round_trips_and_rejects_unknown_fields() {
        let fixture = historical_program_fixture();
        let program: FlightProgramV1 = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&program).unwrap(), fixture);

        for path in ["program", "binding", "update"] {
            let mut with_unknown_field = historical_program_fixture();
            let object = match path {
                "program" => with_unknown_field.as_object_mut().unwrap(),
                "binding" => with_unknown_field["binding"].as_object_mut().unwrap(),
                "update" => with_unknown_field["updates"][0].as_object_mut().unwrap(),
                _ => unreachable!(),
            };
            object.insert("unexpected".to_owned(), json!(true));

            assert!(serde_json::from_value::<FlightProgramV1>(with_unknown_field).is_err());
        }
    }
}
