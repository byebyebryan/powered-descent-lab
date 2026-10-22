//! Procedural input generation and immutable collection management.
//! No strategy, evaluation result, external game checkout or saved flight is an input.

mod admission;
mod collection;
mod collection_v2;
mod collection_versions;
mod coverage;
mod generator;
mod generator_v2;
mod geometry;
mod input_audit;
mod layered;
mod metadata;
mod pads_v2;
mod population;
mod recipe_v2;
mod scenario_inputs;
mod terrain_v2;
mod versions;
pub use admission::*;
pub use collection::*;
pub use collection_v2::*;
pub use collection_versions::*;
pub use coverage::*;
pub use generator::*;
pub use generator_v2::*;
pub use geometry::*;
pub use input_audit::*;
pub use metadata::*;
pub use pads_v2::*;
pub use population::*;
pub use recipe_v2::*;
pub use scenario_inputs::*;
pub use versions::*;

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
