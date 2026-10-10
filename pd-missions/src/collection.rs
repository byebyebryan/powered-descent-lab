use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{MissionRecipeV1, RejectedProposalV1, ResolvedMissionV1, generate_mission, sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionRoleV1 {
    Training,
    Development,
    Validation,
    HeldOut,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionRequestV1 {
    pub schema_version: u32,
    pub role: CollectionRoleV1,
    pub recipe: MissionRecipeV1,
    pub seed: u64,
    pub indices: Vec<u64>,
}

impl CollectionRequestV1 {
    pub fn validate(&self) -> Result<(), String> {
        self.recipe.validate()?;
        if self.schema_version != 1
            || self.indices.is_empty()
            || self.indices.len() > 100_000
            || self.indices.iter().collect::<BTreeSet<_>>().len() != self.indices.len()
        {
            return Err(
                "collection requires 1..100,000 distinct scheduled indices and version 1"
                    .to_owned(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionEntryV1 {
    pub index: u64,
    pub file: String,
    pub sha256: String,
    pub physical_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionManifestV1 {
    pub schema_version: u32,
    pub request: CollectionRequestV1,
    pub entries: Vec<CollectionEntryV1>,
    pub rejections: Vec<RejectedProposalV1>,
    pub exhausted_indices: Vec<u64>,
    pub physical_duplicate_groups: Vec<Vec<u64>>,
}

fn mission_filename(index: u64) -> String {
    format!("mission-{index:020}.json")
}

/// Reserve a fresh collection. Every scheduled request is retained as an entry
/// or an exhausted disposition; duplicate physical inputs are reported, not removed.
pub fn write_collection(
    root: &Path,
    request: CollectionRequestV1,
) -> Result<CollectionManifestV1, String> {
    request.validate()?;
    fs::create_dir(root).map_err(|e| format!("reserve collection {}: {e}", root.display()))?;
    // Preserve the declaration even if an I/O failure prevents completion.
    write_json_create_only(&root.join("request.json"), &request)?;
    let mut manifest = CollectionManifestV1 {
        schema_version: 1,
        request,
        entries: Vec::new(),
        rejections: Vec::new(),
        exhausted_indices: Vec::new(),
        physical_duplicate_groups: Vec::new(),
    };
    let mut physical = BTreeMap::<String, Vec<u64>>::new();
    for &index in &manifest.request.indices {
        let result = generate_mission(&manifest.request.recipe, manifest.request.seed, index)?;
        manifest.rejections.extend(result.rejections);
        if let Some(mission) = result.mission {
            let file = mission_filename(index);
            let bytes = json_bytes(&mission)?;
            write_bytes_create_only(&root.join(&file), &bytes)?;
            physical
                .entry(mission.physical_sha256.clone())
                .or_default()
                .push(index);
            manifest.entries.push(CollectionEntryV1 {
                index,
                file,
                sha256: sha256(&bytes),
                physical_sha256: mission.physical_sha256,
            });
        } else {
            manifest.exhausted_indices.push(index);
        }
    }
    manifest.physical_duplicate_groups = physical
        .into_values()
        .filter(|indices| indices.len() > 1)
        .collect();
    write_json_create_only(&root.join("manifest.json"), &manifest)?;
    Ok(manifest)
}

/// Load immutable declarations without consulting the generator or any strategy.
pub fn read_collection_manifest(root: &Path) -> Result<CollectionManifestV1, String> {
    let manifest: CollectionManifestV1 = serde_json::from_slice(&read_regular_file(
        &root.join("manifest.json"),
        64 * 1024 * 1024,
    )?)
    .map_err(|e| e.to_string())?;
    manifest.request.validate()?;
    let request: CollectionRequestV1 = serde_json::from_slice(&read_regular_file(
        &root.join("request.json"),
        8 * 1024 * 1024,
    )?)
    .map_err(|e| e.to_string())?;
    if manifest.schema_version != 1 || request != manifest.request {
        return Err("collection manifest/request mismatch".to_owned());
    }
    let scheduled: BTreeSet<_> = request.indices.iter().copied().collect();
    let admitted: BTreeSet<_> = manifest.entries.iter().map(|entry| entry.index).collect();
    let exhausted: BTreeSet<_> = manifest.exhausted_indices.iter().copied().collect();
    if admitted.len() != manifest.entries.len()
        || exhausted.len() != manifest.exhausted_indices.len()
        || !admitted.is_disjoint(&exhausted)
        || admitted.union(&exhausted).copied().collect::<BTreeSet<_>>() != scheduled
        || manifest.entries.iter().any(|entry| {
            entry.file != mission_filename(entry.index)
                || !valid_digest(&entry.sha256)
                || !valid_digest(&entry.physical_sha256)
        })
    {
        return Err("collection schedule, filename or identity is invalid".to_owned());
    }
    // Rejection attempts are contiguous prefixes, including every exhausted request.
    let mut attempts = BTreeMap::<u64, Vec<u32>>::new();
    for rejection in &manifest.rejections {
        if rejection.proposal.seed != request.seed || !scheduled.contains(&rejection.proposal.index)
        {
            return Err("rejection is outside declared collection".to_owned());
        }
        attempts
            .entry(rejection.proposal.index)
            .or_default()
            .push(rejection.proposal.attempt);
    }
    for &index in &scheduled {
        let values = attempts.entry(index).or_default();
        if values.iter().copied().ne(0..values.len() as u32)
            || values.len() >= request.recipe.max_attempts as usize && !exhausted.contains(&index)
            || exhausted.contains(&index) && values.len() != request.recipe.max_attempts as usize
        {
            return Err("collection rejection/exhaustion accounting is invalid".to_owned());
        }
    }
    Ok(manifest)
}

pub fn load_collection_mission(
    root: &Path,
    manifest: &CollectionManifestV1,
    entry: &CollectionEntryV1,
) -> Result<ResolvedMissionV1, String> {
    if !manifest.entries.contains(entry) || entry.file != mission_filename(entry.index) {
        return Err("mission is not a declared collection entry".to_owned());
    }
    let bytes = read_regular_file(&root.join(&entry.file), 16 * 1024 * 1024)?;
    if sha256(&bytes) != entry.sha256 {
        return Err("resolved mission bytes changed".to_owned());
    }
    let mission: ResolvedMissionV1 = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    mission.validate()?;
    let rejected = manifest
        .rejections
        .iter()
        .filter(|r| r.proposal.index == entry.index)
        .count();
    if mission.recipe != manifest.request.recipe
        || mission.proposal.seed != manifest.request.seed
        || mission.proposal.index != entry.index
        || mission.proposal.attempt as usize != rejected
        || mission.physical_sha256 != entry.physical_sha256
    {
        return Err("mission/collection receipt mismatch".to_owned());
    }
    Ok(mission)
}

pub fn verify_collection(root: &Path) -> Result<CollectionManifestV1, String> {
    let manifest = read_collection_manifest(root)?;
    let mut physical = BTreeMap::<String, Vec<u64>>::new();
    for entry in &manifest.entries {
        let mission = load_collection_mission(root, &manifest, entry)?;
        physical
            .entry(mission.physical_sha256)
            .or_default()
            .push(entry.index);
    }
    let groups: Vec<_> = physical.into_values().filter(|v| v.len() > 1).collect();
    if groups != manifest.physical_duplicate_groups {
        return Err("physical duplicate accounting mismatch".to_owned());
    }
    Ok(manifest)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionExposureV1 {
    InputInspection,
    Training,
    Tuning,
    Evaluation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionUsageReceiptV1 {
    pub schema_version: u32,
    pub receipt_id: String,
    pub manifest_sha256: String,
    pub exposure: CollectionExposureV1,
    pub purpose: String,
}

pub fn record_collection_usage(
    root: &Path,
    receipt_id: &str,
    exposure: CollectionExposureV1,
    purpose: &str,
) -> Result<CollectionUsageReceiptV1, String> {
    if !valid_receipt_id(receipt_id) || purpose.trim().is_empty() {
        return Err("usage receipt requires a safe unique id and purpose".to_owned());
    }
    read_collection_manifest(root)?;
    let receipt = CollectionUsageReceiptV1 {
        schema_version: 1,
        receipt_id: receipt_id.to_owned(),
        exposure,
        purpose: purpose.to_owned(),
        manifest_sha256: sha256(&read_regular_file(
            &root.join("manifest.json"),
            64 * 1024 * 1024,
        )?),
    };
    write_json_create_only(&root.join(format!("usage-{receipt_id}.json")), &receipt)?;
    Ok(receipt)
}

pub fn read_collection_usage(root: &Path) -> Result<Vec<CollectionUsageReceiptV1>, String> {
    read_collection_manifest(root)?;
    let manifest_digest = sha256(&read_regular_file(
        &root.join("manifest.json"),
        64 * 1024 * 1024,
    )?);
    let mut files = Vec::new();
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name().to_string_lossy().starts_with("usage-") {
            files.push(entry.path());
        }
    }
    files.sort();
    let mut receipts = Vec::new();
    for file in files {
        let receipt: CollectionUsageReceiptV1 =
            serde_json::from_slice(&read_regular_file(&file, 1024 * 1024)?)
                .map_err(|e| e.to_string())?;
        if receipt.schema_version != 1
            || !valid_receipt_id(&receipt.receipt_id)
            || file.file_name().and_then(|n| n.to_str())
                != Some(format!("usage-{}.json", receipt.receipt_id).as_str())
            || receipt.manifest_sha256 != manifest_digest
            || receipt.purpose.trim().is_empty()
        {
            return Err("invalid collection usage receipt".to_owned());
        }
        receipts.push(receipt);
    }
    Ok(receipts)
}

fn valid_receipt_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}
fn valid_digest(digest: &str) -> bool {
    digest.len() == 64 && digest.bytes().all(|c| c.is_ascii_hexdigit())
}

pub fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn write_json_create_only(path: &Path, value: &impl Serialize) -> Result<(), String> {
    write_bytes_create_only(path, &json_bytes(value)?)
}

fn write_bytes_create_only(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("create {}: {e}", path.display()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| format!("write {}: {e}", path.display()))
}

fn read_regular_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return Err(format!("{} must be a bounded regular file", path.display()));
    }
    fs::read(path).map_err(|e| e.to_string())
}
