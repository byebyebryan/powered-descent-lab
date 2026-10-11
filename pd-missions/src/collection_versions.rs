//! Collection dispatch is explicit and leaves both persisted schemas strict.
use crate::collection::read_regular_file;
use crate::{
    CollectionEntryV1, CollectionManifestV1, CollectionManifestV2, CollectionRoleV1,
    VersionedResolvedMission, load_collection_mission, load_collection_mission_v2,
    read_collection_manifest, read_collection_manifest_v2, verify_collection, verify_collection_v2,
};
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum VersionedCollectionManifest {
    V1(Box<CollectionManifestV1>),
    V2(Box<CollectionManifestV2>),
}

impl VersionedCollectionManifest {
    pub fn entries(&self) -> &[CollectionEntryV1] {
        match self {
            Self::V1(m) => &m.entries,
            Self::V2(m) => &m.entries,
        }
    }
    pub fn role(&self) -> CollectionRoleV1 {
        match self {
            Self::V1(m) => m.request.role,
            Self::V2(m) => m.request.role,
        }
    }
    pub fn exhausted_count(&self) -> usize {
        match self {
            Self::V1(m) => m.exhausted_indices.len(),
            Self::V2(m) => m.counts.structural_exhaustion,
        }
    }
    /// Include exclusions as immutable campaign evidence. Append-only usage
    /// receipts are bound separately by the evaluator, just as for V1.
    pub fn immutable_files(&self) -> Vec<&str> {
        let mut files = vec!["request.json", "manifest.json"];
        match self {
            Self::V1(m) => files.extend(m.entries.iter().map(|e| e.file.as_str())),
            Self::V2(m) => files.extend(
                m.receipts
                    .iter()
                    .filter_map(|r| r.proposal.as_ref())
                    .map(|e| e.file.as_str()),
            ),
        }
        files
    }
}

pub fn read_versioned_collection_manifest(
    root: &Path,
) -> Result<VersionedCollectionManifest, String> {
    let bytes = read_regular_file(&root.join("manifest.json"), 64 * 1024 * 1024)?;
    let header: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    match header
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
    {
        Some(1) => Ok(VersionedCollectionManifest::V1(Box::new(
            read_collection_manifest(root)?,
        ))),
        Some(2) => Ok(VersionedCollectionManifest::V2(Box::new(
            read_collection_manifest_v2(root)?,
        ))),
        _ => Err("unsupported collection schema_version".into()),
    }
}

pub fn load_versioned_collection_mission(
    root: &Path,
    manifest: &VersionedCollectionManifest,
    entry: &CollectionEntryV1,
) -> Result<VersionedResolvedMission, String> {
    match manifest {
        VersionedCollectionManifest::V1(m) => Ok(VersionedResolvedMission::V1(Box::new(
            load_collection_mission(root, m, entry)?,
        ))),
        VersionedCollectionManifest::V2(m) => Ok(VersionedResolvedMission::V2(Box::new(
            load_collection_mission_v2(root, m, entry)?,
        ))),
    }
}

pub fn verify_versioned_collection(root: &Path) -> Result<VersionedCollectionManifest, String> {
    match read_versioned_collection_manifest(root)? {
        VersionedCollectionManifest::V1(_) => Ok(VersionedCollectionManifest::V1(Box::new(
            verify_collection(root)?,
        ))),
        VersionedCollectionManifest::V2(_) => Ok(VersionedCollectionManifest::V2(Box::new(
            verify_collection_v2(root)?,
        ))),
    }
}
