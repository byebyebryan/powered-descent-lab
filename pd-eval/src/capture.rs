use std::fs;
use std::path::Path;

use pd_missions::{json_bytes, sha256, write_json_create_only};
use serde::{Deserialize, Serialize};

use crate::{EpisodeEvaluationV1, validate_evaluation};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeCaptureSealV1 {
    pub schema_version: u32,
    pub evaluation_sha256: String,
}

/// New destinations only. Existing bundles, reports and selectors are untouched.
/// Failed-integrity attempts can be saved, with their truthful false disposition.
pub fn save_evaluation(
    root: &Path,
    evaluation: &EpisodeEvaluationV1,
) -> Result<EpisodeCaptureSealV1, String> {
    validate_evaluation(evaluation)?;
    fs::create_dir(root).map_err(|e| format!("reserve episode {}: {e}", root.display()))?;
    let seal = EpisodeCaptureSealV1 {
        schema_version: 1,
        evaluation_sha256: sha256(&json_bytes(evaluation)?),
    };
    write_json_create_only(&root.join("evaluation.json"), evaluation)?;
    write_json_create_only(&root.join("seal.json"), &seal)?;
    Ok(seal)
}

pub fn load_verified_evaluation(root: &Path) -> Result<EpisodeEvaluationV1, String> {
    let bytes = read_regular(root.join("evaluation.json").as_path(), 64 * 1024 * 1024)?;
    let seal: EpisodeCaptureSealV1 = serde_json::from_slice(&read_regular(
        root.join("seal.json").as_path(),
        1024 * 1024,
    )?)
    .map_err(|e| e.to_string())?;
    if seal.schema_version != 1 || seal.evaluation_sha256 != sha256(&bytes) {
        return Err("episode capture byte identity mismatch".to_owned());
    }
    let evaluation = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    validate_evaluation(&evaluation)?;
    Ok(evaluation)
}

fn read_regular(path: &Path, max_bytes: u64) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return Err("capture requires bounded regular files".to_owned());
    }
    fs::read(path).map_err(|e| e.to_string())
}
