//! Versioned characterization inputs, schedules and bounded evidence application.
//! No controller tuning, optimizer, promotion or report-server lifecycle.
mod audit;
mod prepare;
mod recipe;
mod recovery;
mod results;
mod run;
mod worker;
pub use audit::*;
pub use prepare::*;
pub use recipe::*;
pub use results::*;
pub use run::*;
pub use worker::{execute_prepared_attempt, execute_reuse_attempt};
#[cfg(test)]
mod tests;

use pd_missions::{read_scenario_input_file, sha256};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundFileV1 {
    pub path: String,
    pub sha256: String,
}

pub fn bind_file(path: &Path, max: u64) -> Result<BoundFileV1, String> {
    let bytes = read_scenario_input_file(path, max)?;
    Ok(BoundFileV1 {
        path: std::fs::canonicalize(path)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into(),
        sha256: sha256(&bytes),
    })
}
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path, max: u64) -> Result<T, String> {
    serde_json::from_slice(&read_scenario_input_file(path, max)?).map_err(|e| e.to_string())
}
pub fn read_bound_json<T: serde::de::DeserializeOwned>(
    bound: &BoundFileV1,
    max: u64,
) -> Result<T, String> {
    let bytes = read_scenario_input_file(Path::new(&bound.path), max)?;
    if sha256(&bytes) != bound.sha256 {
        return Err("bound input bytes changed".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
pub fn identity(value: &impl Serialize) -> Result<String, String> {
    Ok(sha256(
        &serde_json::to_vec(value).map_err(|e| e.to_string())?,
    ))
}
