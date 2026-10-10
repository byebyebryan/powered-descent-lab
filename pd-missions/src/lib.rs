//! Procedural input generation and immutable collection management.
//! No strategy, evaluation result, external game checkout or saved flight is an input.

mod collection;
mod generator;
pub use collection::*;
pub use generator::*;

use sha2::{Digest, Sha256};

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The identity excludes display metadata and generator membership. It includes
/// every physical and mission input, including entity references and all clocks.
pub fn physical_identity(scenario: &pd_core::ScenarioSpec) -> Result<String, serde_json::Error> {
    let binding = pd_core::FlightProgramBindingV1 {
        sim: scenario.sim.clone(),
        world: scenario.world.clone(),
        vehicle: scenario.vehicle.clone(),
        initial_state: scenario.initial_state.clone(),
        mission: scenario.mission.clone(),
    };
    Ok(sha256(&serde_json::to_vec(&binding)?))
}
