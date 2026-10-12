//! Verified orchestration metadata, never an agent observation. Both composition
//! paths use this projection without importing one another's strategy packages.
use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    MorphologyLabelV1, TerrainBackendV2, VersionedCollectionManifest, VersionedResolvedMission,
    load_versioned_collection_mission,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputGeometryMetadataV2 {
    pub schema_version: u32,
    pub morphology: String,
    pub chord_barrier_m: f64,
    pub distance_m: f64,
    pub signed_pad_rise_m: f64,
    pub source_width_m: f64,
    pub target_width_m: f64,
    pub crest_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionInputMetadataV1 {
    pub family: String,
    pub direction: i32,
    pub stratum: Option<u32>,
    pub geometry: Option<InputGeometryMetadataV2>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CollectionContextMetadataV1 {
    pub schema_version: u32,
    pub population: String,
    pub population_sha256: String,
    pub recipe_sha256: String,
    pub geometry_spec_sha256: Option<String>,
    pub challenge_rule_sha256: Option<String>,
    pub morphology_spec_sha256: Option<String>,
}

pub fn collection_context_metadata(
    manifest: &VersionedCollectionManifest,
) -> Result<CollectionContextMetadataV1, String> {
    let hash = |v| {
        serde_json::to_vec(&v)
            .map(|b| crate::sha256(&b))
            .map_err(|e| e.to_string())
    };
    match manifest {
        VersionedCollectionManifest::V1(m) => Ok(CollectionContextMetadataV1 {
            schema_version: 1,
            population: "legacy_v1".into(),
            population_sha256: hash(serde_json::json!("legacy_v1"))?,
            recipe_sha256: m.request.recipe.identity()?,
            geometry_spec_sha256: None,
            challenge_rule_sha256: None,
            morphology_spec_sha256: None,
        }),
        VersionedCollectionManifest::V2(m) => Ok(CollectionContextMetadataV1 {
            schema_version: 1,
            population: m.request.population.key().into(),
            population_sha256: hash(
                serde_json::to_value(&m.request.population).map_err(|e| e.to_string())?,
            )?,
            recipe_sha256: m.request.recipe.identity()?,
            geometry_spec_sha256: Some(m.request.geometry_spec.identity()?),
            challenge_rule_sha256: Some(
                m.request
                    .challenge_rule
                    .identity(&m.request.geometry_spec)?,
            ),
            morphology_spec_sha256: Some(hash(
                serde_json::to_value(&m.request.morphology_spec).map_err(|e| e.to_string())?,
            )?),
        }),
    }
}

pub fn collection_family(manifest: &VersionedCollectionManifest) -> &str {
    match manifest {
        VersionedCollectionManifest::V1(m) => &m.request.recipe.generator_version,
        VersionedCollectionManifest::V2(m) => match m.request.recipe.backend {
            TerrainBackendV2::LayeredPerlin => "pd-missions/layered-perlin-rest-v2",
            TerrainBackendV2::ValueNoise => "pd-missions/value-noise-rest-v2",
        },
    }
}

pub fn collection_input_metadata(
    root: &Path,
    manifest: &VersionedCollectionManifest,
    index: u64,
) -> Result<CollectionInputMetadataV1, String> {
    let entry = manifest
        .entries()
        .iter()
        .find(|e| e.index == index)
        .ok_or("scheduled input is not admitted")?;
    let mission = load_versioned_collection_mission(root, manifest, entry)?;
    Ok(match mission {
        VersionedResolvedMission::V1(m) => CollectionInputMetadataV1 {
            family: m.recipe.generator_version,
            direction: m.direction,
            stratum: Some(m.terrain_stratum),
            geometry: None,
        },
        VersionedResolvedMission::V2(m) => {
            let VersionedCollectionManifest::V2(collection) = manifest else {
                return Err("resolved/collection schema mismatch".into());
            };
            let receipt = collection
                .receipts
                .iter()
                .find(|r| r.index == index)
                .ok_or("missing selection receipt")?;
            let morphology = match receipt.labels.as_slice() {
                [] => "Unclassified",
                [MorphologyLabelV1::MountainCrossing] => "Mountain crossing",
                [MorphologyLabelV1::BroadMassif] => "Broad massif",
                [MorphologyLabelV1::SuccessiveRidges] => "Successive ridges",
                [MorphologyLabelV1::ElevationTransition] => "Elevation transition",
                _ => "Mixed morphology",
            };
            let g = &m.geometry.geometry;
            CollectionInputMetadataV1 {
                family: collection_family(manifest).into(),
                direction: m.direction,
                stratum: None,
                geometry: Some(InputGeometryMetadataV2 {
                    schema_version: 2,
                    morphology: morphology.into(),
                    chord_barrier_m: g.max_chord_barrier_m,
                    distance_m: g.distance_m,
                    signed_pad_rise_m: g.signed_pad_rise_m,
                    source_width_m: g.source_width_m,
                    target_width_m: g.target_width_m,
                    crest_count: g.crests.len(),
                }),
            }
        }
    })
}

/// A bounded selected-index cache. Loading verifies each original input and its
/// selection receipt; callers separately verify/bind the full file inventory.
pub fn collection_metadata(
    root: &Path,
    manifest: &VersionedCollectionManifest,
    indices: &[u64],
) -> Result<BTreeMap<u64, CollectionInputMetadataV1>, String> {
    if indices.is_empty() || indices.len() > 100_000 {
        return Err("metadata schedule requires 1..100000 indices".into());
    }
    let mut result = BTreeMap::new();
    for &index in indices {
        if let std::collections::btree_map::Entry::Vacant(entry) = result.entry(index) {
            entry.insert(collection_input_metadata(root, manifest, index)?);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn metadata_reconciles_backends_and_rejects_input_or_receipt_drift() {
        let parent = std::env::temp_dir().join(format!("pd-input-metadata-{}", std::process::id()));
        std::fs::create_dir(&parent).unwrap();
        for (family, expected) in [
            ("layered-v2", "pd-missions/layered-perlin-rest-v2"),
            ("value-noise-v2", "pd-missions/value-noise-rest-v2"),
        ] {
            let root = parent.join(family);
            let VersionedMissionRecipe::V2(recipe) =
                VersionedMissionRecipe::family(family).unwrap()
            else {
                panic!()
            };
            let m = write_collection_v2(&root, CollectionRequestV2::broad(recipe, 981, 8)).unwrap();
            let manifest = VersionedCollectionManifest::V2(Box::new(m));
            let entry = &manifest.entries()[0];
            let VersionedCollectionManifest::V2(collection) = &manifest else {
                panic!()
            };
            let context = collection_context_metadata(&manifest).unwrap();
            assert_eq!(
                context.recipe_sha256,
                collection.request.recipe.identity().unwrap()
            );
            assert_eq!(
                context.geometry_spec_sha256,
                Some(collection.request.geometry_spec.identity().unwrap())
            );
            let meta = collection_input_metadata(&root, &manifest, entry.index).unwrap();
            assert_eq!(meta.family, expected);
            assert!(meta.geometry.is_some());
            assert!(meta.stratum.is_none());
            assert_eq!(
                collection_metadata(&root, &manifest, &[entry.index, entry.index])
                    .unwrap()
                    .len(),
                1
            );
            let mut changed = manifest.clone();
            let VersionedCollectionManifest::V2(collection) = &mut changed else {
                panic!()
            };
            let receipt = collection
                .receipts
                .iter_mut()
                .find(|r| r.index == entry.index)
                .unwrap();
            receipt.labels = if receipt.labels == [MorphologyLabelV1::MountainCrossing] {
                vec![MorphologyLabelV1::BroadMassif]
            } else {
                vec![MorphologyLabelV1::MountainCrossing]
            };
            assert!(collection_input_metadata(&root, &changed, entry.index).is_err());
            std::fs::write(root.join(&entry.file), b"{}").unwrap();
            assert!(collection_input_metadata(&root, &manifest, entry.index).is_err());
        }
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn v1_metadata_retains_real_direction_and_stratum_without_v2_geometry() {
        let root =
            std::env::temp_dir().join(format!("pd-input-metadata-v1-{}", std::process::id()));
        let m = write_collection(
            &root,
            CollectionRequestV1 {
                schema_version: 1,
                role: CollectionRoleV1::Development,
                recipe: MissionRecipeV1::default(),
                seed: 45,
                indices: vec![0, 4],
            },
        )
        .unwrap();
        let manifest = VersionedCollectionManifest::V1(Box::new(m));
        for entry in manifest.entries() {
            let metadata = collection_input_metadata(&root, &manifest, entry.index).unwrap();
            let VersionedResolvedMission::V1(mission) =
                load_versioned_collection_mission(&root, &manifest, entry).unwrap()
            else {
                panic!()
            };
            assert_eq!(metadata.family, mission.recipe.generator_version);
            assert_eq!(metadata.direction, mission.direction);
            assert_eq!(metadata.stratum, Some(mission.terrain_stratum));
            assert!(metadata.geometry.is_none());
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
