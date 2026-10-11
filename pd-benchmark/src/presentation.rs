//! CLI composition only: strict version dispatch and neutral report projections.
use std::{collections::BTreeMap, io::Write, path::Path};

use pd_missions::{VersionedCollectionManifest, VersionedResolvedMission};
use pd_report::campaign_view::{CampaignGeometryV2, CampaignPresentationV1};

pub struct InputMetadata {
    pub family: String,
    pub direction: i32,
    pub stratum: Option<u32>,
    pub geometry: Option<CampaignGeometryV2>,
}

pub fn schedule(
    manifest: &VersionedCollectionManifest,
    count: Option<usize>,
    proposal_count: Option<usize>,
) -> Result<Vec<u64>, String> {
    let entries = manifest.entries();
    if count.is_some_and(|n| n == 0 || n > entries.len()) {
        return Err("count must select a nonempty declared admitted prefix".into());
    }
    let selected: Vec<u64> = if let Some(n) = proposal_count {
        let VersionedCollectionManifest::V2(m) = manifest else {
            return Err("proposal-count requires a V2 collection".into());
        };
        if n == 0 || n > m.request.indices.len() {
            return Err("proposal-count exceeds the declared proposal schedule".into());
        }
        let prefix: std::collections::BTreeSet<_> =
            m.request.indices.iter().take(n).copied().collect();
        entries
            .iter()
            .filter(|e| prefix.contains(&e.index))
            .map(|e| e.index)
            .collect()
    } else {
        entries
            .iter()
            .take(count.unwrap_or(entries.len()))
            .map(|e| e.index)
            .collect()
    };
    if selected.is_empty() {
        return Err("declared selection contains no admitted inputs".into());
    }
    Ok(selected)
}

pub fn metadata(
    root: &Path,
    manifest: &VersionedCollectionManifest,
    indices: &[u64],
) -> Result<BTreeMap<u64, InputMetadata>, String> {
    let entries: BTreeMap<_, _> = manifest.entries().iter().map(|e| (e.index, e)).collect();
    let mut result = BTreeMap::new();
    for index in indices {
        if result.contains_key(index) {
            continue;
        }
        let entry = entries
            .get(index)
            .ok_or("scheduled input is not admitted")?;
        let mission = pd_missions::load_versioned_collection_mission(root, manifest, entry)?;
        let input = match mission {
            VersionedResolvedMission::V1(m) => InputMetadata {
                family: m.recipe.generator_version.clone(),
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
                    .find(|r| r.index == *index)
                    .ok_or("missing selection receipt")?;
                let morphology = match receipt.labels.as_slice() {
                    [] => "Unclassified",
                    [pd_missions::MorphologyLabelV1::MountainCrossing] => "Mountain crossing",
                    [pd_missions::MorphologyLabelV1::BroadMassif] => "Broad massif",
                    [pd_missions::MorphologyLabelV1::SuccessiveRidges] => "Successive ridges",
                    [pd_missions::MorphologyLabelV1::ElevationTransition] => "Elevation transition",
                    _ => "Mixed morphology",
                };
                let g = &m.geometry.geometry;
                InputMetadata {
                    family: match m.recipe.backend {
                        pd_missions::TerrainBackendV2::LayeredPerlin => {
                            "pd-missions/layered-perlin-rest-v2"
                        }
                        pd_missions::TerrainBackendV2::ValueNoise => {
                            "pd-missions/value-noise-rest-v2"
                        }
                    }
                    .into(),
                    direction: m.direction,
                    stratum: None,
                    geometry: Some(CampaignGeometryV2 {
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
        };
        result.insert(*index, input);
    }
    Ok(result)
}

pub fn write_preview(
    root: &Path,
    attempt: u64,
    evaluation: &pd_eval::EpisodeEvaluationV1,
) -> Result<CampaignPresentationV1, String> {
    let scenario = &evaluation.scenario;
    let episode = &evaluation.execution.episode;
    let mut samples = episode.run.samples.clone();
    let state = &episode.final_state;
    if samples
        .last()
        .is_none_or(|s| s.physics_step != state.physics_step)
    {
        let ctx = pd_core::RunContext::from_scenario(scenario)?;
        let sim = state.to_simulation_state();
        samples.push(pd_core::SampleRecord {
            sim_time_s: sim.sim_time_s,
            physics_step: sim.physics_step,
            observation: sim.build_observation(&ctx),
            held_command: sim.held_command,
        });
    }
    let svg = pd_report::build_saved_flight_preview_svg(
        scenario,
        Some(&episode.run.manifest),
        &samples,
        &[],
    );
    let href = format!("previews/attempt-{attempt:06}.svg");
    std::fs::create_dir_all(root.join("previews")).map_err(|e| e.to_string())?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(&href))
        .map_err(|e| e.to_string())?
        .write_all(svg.as_bytes())
        .map_err(|e| e.to_string())?;
    let target = scenario
        .world
        .landing_pad(scenario.mission.goal.target_pad_id());
    Ok(CampaignPresentationV1 {
        schema_version: 1,
        preview_href: href,
        caption: "Recorded terrain and flight · direction normalized for display".into(),
        target_horizontal_offset_m: target.map(|p| (state.position_m.x - p.center_x_m).abs()),
        height_above_upright_touchdown_m: target.map(|p| {
            state.position_m.y - p.surface_y_m - scenario.vehicle.geometry.touchdown_base_offset_m
        }),
        endpoint_speed_mps: Some(state.velocity_mps.x.hypot(state.velocity_mps.y)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proposal_prefix_keeps_exclusions_in_the_input_budget_without_refilling() {
        let request =
            pd_missions::CollectionRequestV2::broad(pd_missions::MissionRecipeV2::default(), 1, 8);
        let manifest =
            VersionedCollectionManifest::V2(Box::new(pd_missions::CollectionManifestV2 {
                schema_version: 2,
                request,
                entries: [0, 2, 4, 7]
                    .into_iter()
                    .map(|index| pd_missions::CollectionEntryV1 {
                        index,
                        file: format!("proposal-{index:020}.json"),
                        sha256: "0".repeat(64),
                        physical_sha256: "1".repeat(64),
                    })
                    .collect(),
                receipts: vec![],
                counts: pd_missions::SelectionCountsV2::default(),
            }));
        assert_eq!(schedule(&manifest, None, Some(4)).unwrap(), [0, 2]);
        assert_eq!(schedule(&manifest, Some(3), None).unwrap(), [0, 2, 4]);
        assert!(schedule(&manifest, None, Some(0)).is_err());
        assert!(schedule(&manifest, None, Some(9)).is_err());
    }
}
