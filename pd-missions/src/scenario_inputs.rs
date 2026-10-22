//! Portable native scenarios and uninterpreted provenance, independent of any strategy.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::{Component, Path},
};

use pd_core::ScenarioSpec;
use serde::{Deserialize, Serialize};

use crate::{json_bytes, physical_identity, sha256, write_json_create_only};

pub const MAX_SCENARIO_INPUTS: usize = 16_384;
pub const MAX_SCENARIO_INPUT_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_SCENARIO_SET_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInputSourceV1 {
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInputProvenanceV1 {
    pub implementation: String,
    pub executable_sha256: String,
    /// Exact producer inputs, not a claim about the producer's compiled sources.
    pub sources: Vec<ScenarioInputSourceV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeScenarioInputV1 {
    pub id: String,
    pub scenario: ScenarioSpec,
    pub labels: BTreeMap<String, String>,
    pub parameters: BTreeMap<String, f64>,
    pub lineage: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInputRequestV1 {
    pub schema_version: u32,
    pub provenance: ScenarioInputProvenanceV1,
    pub occurrence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInputEntryV1 {
    pub id: String,
    pub file: String,
    pub sha256: String,
    pub physical_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInputManifestV1 {
    pub schema_version: u32,
    pub request: ScenarioInputRequestV1,
    pub entries: Vec<ScenarioInputEntryV1>,
    /// All occurrences, including singleton groups. No implicit deduplication.
    pub physical_aliases: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputCompletionV1 {
    schema_version: u32,
    request_sha256: String,
    manifest_sha256: String,
}

/// Check every component before following it, including ancestors of a relative path.
pub fn check_input_path(path: &Path) -> Result<(), String> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let mut prefix = std::path::PathBuf::new();
    for component in absolute.components() {
        if component == Component::ParentDir {
            prefix.pop();
            continue;
        }
        prefix.push(component);
        let meta =
            fs::symlink_metadata(&prefix).map_err(|e| format!("{}: {e}", prefix.display()))?;
        if meta.is_symlink() {
            return Err("symlink in input path".into());
        }
    }
    Ok(())
}

pub fn read_scenario_input_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>, String> {
    check_input_path(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return Err("input must be a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max_bytes {
        return Err("input grew beyond limit".into());
    }
    Ok(bytes)
}

fn digest_valid(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit())
}

fn validate_request(request: &ScenarioInputRequestV1) -> Result<(), String> {
    if request.schema_version != 1
        || request.provenance.implementation.trim().is_empty()
        || !digest_valid(&request.provenance.executable_sha256)
        || request
            .provenance
            .sources
            .iter()
            .any(|s| s.path.is_empty() || !digest_valid(&s.sha256))
        || request.occurrence_ids.is_empty()
        || request.occurrence_ids.len() > MAX_SCENARIO_INPUTS
        || request
            .occurrence_ids
            .iter()
            .any(|id| id.trim().is_empty() || id.len() > 1024)
        || request.occurrence_ids.iter().collect::<BTreeSet<_>>().len()
            != request.occurrence_ids.len()
    {
        return Err("invalid input request schema, provenance or occurrence identities".into());
    }
    Ok(())
}

fn input_filename(index: usize) -> String {
    format!("input-{index:06}.json")
}

pub fn write_scenario_inputs(
    root: &Path,
    provenance: ScenarioInputProvenanceV1,
    inputs: &[NativeScenarioInputV1],
) -> Result<ScenarioInputManifestV1, String> {
    let request = ScenarioInputRequestV1 {
        schema_version: 1,
        provenance,
        occurrence_ids: inputs.iter().map(|i| i.id.clone()).collect(),
    };
    validate_request(&request)?;
    let parent = root
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    check_input_path(parent)?;
    let destination = fs::canonicalize(parent)
        .map_err(|e| e.to_string())?
        .join(root.file_name().ok_or("input set needs a filename")?);
    for source in &request.provenance.sources {
        let source_path = fs::canonicalize(&source.path).map_err(|e| e.to_string())?;
        if source_path.starts_with(&destination) || destination.starts_with(&source_path) {
            return Err("input destination overlaps a source".into());
        }
    }
    let mut total = 0_u64;
    for input in inputs {
        input.scenario.validate()?;
        if input.parameters.values().any(|v| !v.is_finite()) {
            return Err("nonfinite input parameter".into());
        }
        let size = json_bytes(input)?.len() as u64;
        total = total.checked_add(size).ok_or("input size overflow")?;
        if size > MAX_SCENARIO_INPUT_BYTES || total > MAX_SCENARIO_SET_BYTES {
            return Err("input set byte limit exceeded".into());
        }
    }
    fs::create_dir(&destination).map_err(|e| format!("reserve new input set: {e}"))?;
    write_json_create_only(&destination.join("request.json"), &request)?;
    let mut manifest = ScenarioInputManifestV1 {
        schema_version: 1,
        request,
        entries: vec![],
        physical_aliases: BTreeMap::new(),
    };
    for (index, input) in inputs.iter().enumerate() {
        let file = input_filename(index);
        write_json_create_only(&destination.join(&file), input)?;
        let physical_sha256 = physical_identity(&input.scenario).map_err(|e| e.to_string())?;
        manifest
            .physical_aliases
            .entry(physical_sha256.clone())
            .or_default()
            .push(input.id.clone());
        manifest.entries.push(ScenarioInputEntryV1 {
            id: input.id.clone(),
            file,
            sha256: sha256(&json_bytes(input)?),
            physical_sha256,
        });
    }
    write_json_create_only(&destination.join("manifest.json"), &manifest)?;
    write_json_create_only(
        &destination.join("completion.json"),
        &InputCompletionV1 {
            schema_version: 1,
            request_sha256: sha256(&json_bytes(&manifest.request)?),
            manifest_sha256: sha256(&json_bytes(&manifest)?),
        },
    )?;
    Ok(manifest)
}

pub fn load_scenario_inputs(
    root: &Path,
) -> Result<(ScenarioInputManifestV1, Vec<NativeScenarioInputV1>), String> {
    let manifest_bytes = read_scenario_input_file(&root.join("manifest.json"), 32 * 1024 * 1024)?;
    let request_bytes = read_scenario_input_file(&root.join("request.json"), 16 * 1024 * 1024)?;
    let completion: InputCompletionV1 = serde_json::from_slice(&read_scenario_input_file(
        &root.join("completion.json"),
        4096,
    )?)
    .map_err(|e| e.to_string())?;
    let manifest: ScenarioInputManifestV1 =
        serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    let request: ScenarioInputRequestV1 =
        serde_json::from_slice(&request_bytes).map_err(|e| e.to_string())?;
    validate_request(&request)?;
    if manifest.schema_version != 1
        || completion.schema_version != 1
        || manifest.request != request
        || completion.request_sha256 != sha256(&request_bytes)
        || completion.manifest_sha256 != sha256(&manifest_bytes)
        || manifest.entries.len() != request.occurrence_ids.len()
    {
        return Err("input set completion, request or schema mismatch".into());
    }
    let mut inputs = Vec::with_capacity(manifest.entries.len());
    let mut aliases = BTreeMap::<String, Vec<String>>::new();
    let mut total = 0_u64;
    for (index, entry) in manifest.entries.iter().enumerate() {
        if entry.id != request.occurrence_ids[index] || entry.file != input_filename(index) {
            return Err("input occurrence order or filename mismatch".into());
        }
        let bytes = read_scenario_input_file(&root.join(&entry.file), MAX_SCENARIO_INPUT_BYTES)?;
        total = total
            .checked_add(bytes.len() as u64)
            .ok_or("input size overflow")?;
        if total > MAX_SCENARIO_SET_BYTES || sha256(&bytes) != entry.sha256 {
            return Err("input bytes changed or set too large".into());
        }
        let input: NativeScenarioInputV1 =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        input.scenario.validate()?;
        if input.id != entry.id
            || input.parameters.values().any(|v| !v.is_finite())
            || physical_identity(&input.scenario).map_err(|e| e.to_string())?
                != entry.physical_sha256
        {
            return Err("input identity mismatch".into());
        }
        aliases
            .entry(entry.physical_sha256.clone())
            .or_default()
            .push(entry.id.clone());
        inputs.push(input);
    }
    if aliases != manifest.physical_aliases {
        return Err("physical alias inventory mismatch".into());
    }
    Ok((manifest, inputs))
}
