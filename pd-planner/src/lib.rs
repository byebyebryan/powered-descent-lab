//! Concrete maintained solver implementation. Neutral runtime and evaluation
//! packages do not depend on this crate.
pub mod ballistic_feedback;
pub mod planner_flight;
pub use planner_flight::*;
pub mod local_clearing;
pub use local_clearing::*;
pub mod waypoint_v2;
pub use waypoint_v2::*;
/// A diagnostic recorder supplied by an application. Solvers own computations;
/// callers own filesystem destinations and create-only artifact handling.
pub trait DiagnosticRecorder {
    fn write_json<T: serde::Serialize + ?Sized>(
        &mut self,
        name: &str,
        value: &T,
    ) -> anyhow::Result<()>;
}
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
#[cfg(test)]
mod test_inputs;

mod configuration;
pub use configuration::*;
