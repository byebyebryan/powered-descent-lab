//! Create-only challenging collections with one receipt per scheduled index.
//! Every resolved proposal is retained, including exclusions and duplicates.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::collection::{read_regular_file, valid_digest};
use crate::{
    ChallengeReceiptV1, ChallengeRuleV1, CollectionEntryV1, CollectionRoleV1, GeometrySpecV1,
    MissionRecipeV2, MorphologyLabelV1, MorphologySpecV1, PopulationDeclarationV2,
    RejectedProposalV2, ResolvedMissionV2, generate_mission_v2, json_bytes, sha256,
    write_json_create_only,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionRequestV2 {
    pub schema_version: u32,
    pub role: CollectionRoleV1,
    pub recipe: MissionRecipeV2,
    pub geometry_spec: GeometrySpecV1,
    pub challenge_rule: ChallengeRuleV1,
    pub morphology_spec: MorphologySpecV1,
    pub population: PopulationDeclarationV2,
    pub seed: u64,
    /// Canonical ascending indices. Writers sort the supplied schedule before
    /// reserving output; selection and duplicate winners never depend on workers.
    pub indices: Vec<u64>,
}

impl CollectionRequestV2 {
    /// Build a bounded declaration. Oversized counts retain an invalid sentinel
    /// schedule so validation fails before any output is reserved.
    pub fn broad(recipe: MissionRecipeV2, seed: u64, count: u64) -> Self {
        Self {
            schema_version: 2,
            role: CollectionRoleV1::Development,
            recipe,
            geometry_spec: GeometrySpecV1::default(),
            challenge_rule: ChallengeRuleV1::default(),
            morphology_spec: MorphologySpecV1::default(),
            population: PopulationDeclarationV2::Broad,
            seed,
            indices: (0..count.min(20001)).collect(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.recipe.validate()?;
        self.challenge_rule.validate(&self.geometry_spec)?;
        self.morphology_spec.validate(&self.geometry_spec)?;
        self.population.validate(&self.recipe)?;
        if self.schema_version != 2
            || self.indices.is_empty()
            || self.indices.len() > 20000
            || self.indices.windows(2).any(|p| p[0] >= p[1])
        {
            return Err(
                "V2 collection requires 1..20000 ascending distinct scheduled indices".into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SelectionDispositionV2 {
    Accepted,
    InsufficientChallenge,
    PhysicalDuplicate { first_index: u64 },
    StructuralExhaustion,
    StressExcluded,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionReceiptV2 {
    pub index: u64,
    pub structural_rejections: Vec<RejectedProposalV2>,
    pub proposal: Option<CollectionEntryV1>,
    pub challenge: Option<ChallengeReceiptV1>,
    pub labels: Vec<MorphologyLabelV1>,
    pub matching_stress_panels: Vec<String>,
    /// A unique accepted mission can contribute to several panel quotas. Panel
    /// counts overlap and must not be summed to obtain a mission denominator.
    pub selected_stress_panels: Vec<String>,
    pub disposition: SelectionDispositionV2,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionCountsV2 {
    pub scheduled: usize,
    pub accepted: usize,
    pub insufficient_challenge: usize,
    pub physical_duplicate: usize,
    pub structural_exhaustion: usize,
    pub stress_excluded: usize,
}

impl SelectionCountsV2 {
    pub fn from_receipts(receipts: &[SelectionReceiptV2]) -> Self {
        let mut counts = Self {
            scheduled: receipts.len(),
            ..Self::default()
        };
        for receipt in receipts {
            match receipt.disposition {
                SelectionDispositionV2::Accepted => counts.accepted += 1,
                SelectionDispositionV2::InsufficientChallenge => counts.insufficient_challenge += 1,
                SelectionDispositionV2::PhysicalDuplicate { .. } => counts.physical_duplicate += 1,
                SelectionDispositionV2::StructuralExhaustion => counts.structural_exhaustion += 1,
                SelectionDispositionV2::StressExcluded => counts.stress_excluded += 1,
            }
        }
        counts
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionManifestV2 {
    pub schema_version: u32,
    pub request: CollectionRequestV2,
    pub entries: Vec<CollectionEntryV1>,
    pub receipts: Vec<SelectionReceiptV2>,
    pub counts: SelectionCountsV2,
}

fn proposal_filename(index: u64) -> String {
    format!("proposal-{index:020}.json")
}

#[derive(Default)]
struct SelectionState {
    physical: BTreeMap<String, u64>,
    panels: BTreeMap<String, usize>,
}

impl SelectionState {
    fn decide(
        &mut self,
        request: &CollectionRequestV2,
        receipt: &SelectionReceiptV2,
    ) -> (SelectionDispositionV2, Vec<String>) {
        let Some(proposal) = &receipt.proposal else {
            return (SelectionDispositionV2::StructuralExhaustion, Vec::new());
        };
        if !receipt.challenge.as_ref().is_some_and(|c| c.qualifies) {
            return (SelectionDispositionV2::InsufficientChallenge, Vec::new());
        }
        if let Some(&first_index) = self.physical.get(&proposal.physical_sha256) {
            return (
                SelectionDispositionV2::PhysicalDuplicate { first_index },
                Vec::new(),
            );
        }
        self.physical
            .insert(proposal.physical_sha256.clone(), receipt.index);
        if let PopulationDeclarationV2::Stress { panels } = &request.population {
            let mut selected = Vec::new();
            for panel in panels {
                let count = self.panels.entry(panel.id.clone()).or_default();
                if receipt.matching_stress_panels.contains(&panel.id) && *count < panel.quota {
                    *count += 1;
                    selected.push(panel.id.clone());
                }
            }
            if selected.is_empty() {
                return (SelectionDispositionV2::StressExcluded, selected);
            }
            return (SelectionDispositionV2::Accepted, selected);
        }
        (SelectionDispositionV2::Accepted, Vec::new())
    }
}

fn matching_panels(request: &CollectionRequestV2, mission: &ResolvedMissionV2) -> Vec<String> {
    match &request.population {
        PopulationDeclarationV2::Stress { panels } => panels
            .iter()
            .filter(|p| {
                p.matches(
                    &mission.geometry.geometry,
                    request.challenge_rule.barrier_height_m,
                )
            })
            .map(|p| p.id.clone())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn write_collection_v2(
    root: &Path,
    mut request: CollectionRequestV2,
) -> Result<CollectionManifestV2, String> {
    request.indices.sort_unstable();
    request.validate()?;
    fs::create_dir(root).map_err(|e| format!("reserve collection {}: {e}", root.display()))?;
    write_json_create_only(&root.join("request.json"), &request)?;
    let mut receipts = Vec::new();
    let mut entries = Vec::new();
    let mut selection = SelectionState::default();
    for &index in &request.indices {
        let result =
            generate_mission_v2(&request.recipe, &request.geometry_spec, request.seed, index)?;
        let mut receipt = SelectionReceiptV2 {
            index,
            structural_rejections: result.rejections,
            proposal: None,
            challenge: None,
            labels: Vec::new(),
            matching_stress_panels: Vec::new(),
            selected_stress_panels: Vec::new(),
            disposition: SelectionDispositionV2::StructuralExhaustion,
        };
        if let Some(mission) = result.mission {
            let file = proposal_filename(index);
            let bytes = json_bytes(&mission)?;
            if bytes.len() > 16 * 1024 * 1024 {
                return Err("resolved V2 proposal exceeds bounded reader size".into());
            }
            crate::collection::write_bytes_create_only(&root.join(&file), &bytes)?;
            receipt.proposal = Some(CollectionEntryV1 {
                index,
                file,
                sha256: sha256(&bytes),
                physical_sha256: mission.physical_sha256.clone(),
            });
            receipt.challenge = Some(ChallengeReceiptV1::measure(
                &request.challenge_rule,
                &mission.raw_geometry,
                &mission.geometry.geometry,
            ));
            receipt.labels = request.morphology_spec.labels(&mission.geometry.geometry);
            receipt.matching_stress_panels = matching_panels(&request, &mission);
        }
        (receipt.disposition, receipt.selected_stress_panels) =
            selection.decide(&request, &receipt);
        if receipt.disposition == SelectionDispositionV2::Accepted {
            entries.push(receipt.proposal.clone().ok_or("accepted proposal absent")?);
        }
        receipts.push(receipt);
    }
    let counts = SelectionCountsV2::from_receipts(&receipts);
    let manifest = CollectionManifestV2 {
        schema_version: 2,
        request,
        entries,
        receipts,
        counts,
    };
    let bytes = json_bytes(&manifest)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("collection manifest exceeds bounded size".into());
    }
    crate::collection::write_bytes_create_only(&root.join("manifest.json"), &bytes)?;
    Ok(manifest)
}

/// Authenticate the declaration and complete selection ledger. Full input and
/// derived-geometry validation is explicit in `verify_collection_v2`; this read
/// does not regenerate terrain or silently consume all proposal files.
pub fn read_collection_manifest_v2(root: &Path) -> Result<CollectionManifestV2, String> {
    let manifest: CollectionManifestV2 = serde_json::from_slice(&read_regular_file(
        &root.join("manifest.json"),
        64 * 1024 * 1024,
    )?)
    .map_err(|e| e.to_string())?;
    manifest.request.validate()?;
    let request: CollectionRequestV2 = serde_json::from_slice(&read_regular_file(
        &root.join("request.json"),
        8 * 1024 * 1024,
    )?)
    .map_err(|e| e.to_string())?;
    if manifest.schema_version != 2
        || request != manifest.request
        || manifest
            .receipts
            .iter()
            .map(|r| r.index)
            .ne(request.indices.iter().copied())
        || manifest.counts != SelectionCountsV2::from_receipts(&manifest.receipts)
    {
        return Err("V2 collection declaration, schedule or counts mismatch".into());
    }
    let mut selection = SelectionState::default();
    let mut entries = Vec::new();
    for receipt in &manifest.receipts {
        let rejects = &receipt.structural_rejections;
        if rejects.len() > request.recipe.max_attempts as usize
            || rejects.iter().enumerate().any(|(attempt, r)| {
                r.proposal.seed != request.seed
                    || r.proposal.index != receipt.index
                    || r.proposal.attempt as usize != attempt
                    || r.detail.len() > 4096
            })
        {
            return Err("V2 structural attempt accounting mismatch".into());
        }
        match (&receipt.proposal, &receipt.challenge) {
            (None, None)
                if rejects.len() == request.recipe.max_attempts as usize
                    && receipt.labels.is_empty()
                    && receipt.matching_stress_panels.is_empty() => {}
            (Some(p), Some(c))
                if rejects.len() < request.recipe.max_attempts as usize
                    && p.index == receipt.index
                    && p.file == proposal_filename(receipt.index)
                    && valid_digest(&p.sha256)
                    && valid_digest(&p.physical_sha256)
                    && c.qualifies
                        == (c.raw.sustained_barrier && c.prepared.sustained_barrier
                            || c.raw.elevation_transition && c.prepared.elevation_transition
                            || c.raw.successive_crests && c.prepared.successive_crests) => {}
            _ => return Err("V2 resolved/exhausted proposal accounting mismatch".into()),
        }
        if receipt
            .labels
            .iter()
            .enumerate()
            .any(|(i, l)| receipt.labels[..i].contains(l))
        {
            return Err("duplicate morphology label".into());
        }
        let declared_panels: Vec<_> = match &request.population {
            PopulationDeclarationV2::Stress { panels } => {
                panels.iter().map(|p| p.id.clone()).collect()
            }
            _ => Vec::new(),
        };
        let canonical_matches: Vec<_> = declared_panels
            .into_iter()
            .filter(|p| receipt.matching_stress_panels.contains(p))
            .collect();
        if canonical_matches != receipt.matching_stress_panels {
            return Err("undeclared or noncanonical stress matches".into());
        }
        let (disposition, panels) = selection.decide(&request, receipt);
        if disposition != receipt.disposition || panels != receipt.selected_stress_panels {
            return Err("V2 challenge, duplicate or stress selection accounting mismatch".into());
        }
        if disposition == SelectionDispositionV2::Accepted {
            entries.push(receipt.proposal.clone().ok_or("accepted proposal absent")?);
        }
    }
    if entries != manifest.entries {
        return Err("V2 accepted entry accounting mismatch".into());
    }
    Ok(manifest)
}

/// Load either an admitted or excluded proposal for an explicit input audit.
pub fn load_collection_proposal_v2(
    root: &Path,
    manifest: &CollectionManifestV2,
    receipt: &SelectionReceiptV2,
) -> Result<ResolvedMissionV2, String> {
    let position = manifest
        .request
        .indices
        .binary_search(&receipt.index)
        .map_err(|_| "proposal outside schedule")?;
    if manifest.receipts.get(position) != Some(receipt) {
        return Err("proposal is not a declared receipt".into());
    }
    let entry = receipt
        .proposal
        .as_ref()
        .ok_or("structurally exhausted request has no resolved proposal")?;
    if entry.file != proposal_filename(receipt.index) {
        return Err("invalid V2 proposal filename".into());
    }
    let bytes = read_regular_file(&root.join(&entry.file), 16 * 1024 * 1024)?;
    if sha256(&bytes) != entry.sha256 {
        return Err("V2 resolved proposal bytes changed".into());
    }
    let mission: ResolvedMissionV2 = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    mission.validate()?;
    if mission.recipe != manifest.request.recipe
        || mission.geometry.spec != manifest.request.geometry_spec
        || mission.proposal.seed != manifest.request.seed
        || mission.proposal.index != receipt.index
        || mission.proposal.attempt as usize != receipt.structural_rejections.len()
        || mission.physical_sha256 != entry.physical_sha256
        || receipt.challenge
            != Some(ChallengeReceiptV1::measure(
                &manifest.request.challenge_rule,
                &mission.raw_geometry,
                &mission.geometry.geometry,
            ))
        || receipt.labels
            != manifest
                .request
                .morphology_spec
                .labels(&mission.geometry.geometry)
        || receipt.matching_stress_panels != matching_panels(&manifest.request, &mission)
    {
        return Err("V2 proposal/selection receipt mismatch".into());
    }
    Ok(mission)
}

pub fn load_collection_mission_v2(
    root: &Path,
    manifest: &CollectionManifestV2,
    entry: &CollectionEntryV1,
) -> Result<ResolvedMissionV2, String> {
    if !manifest.entries.contains(entry) {
        return Err("mission is not an admitted V2 entry".into());
    }
    let position = manifest
        .request
        .indices
        .binary_search(&entry.index)
        .map_err(|_| "entry outside schedule")?;
    let receipt = manifest
        .receipts
        .get(position)
        .ok_or("V2 entry has no selection receipt")?;
    if receipt.disposition != SelectionDispositionV2::Accepted
        || receipt.proposal.as_ref() != Some(entry)
    {
        return Err("V2 entry is not an admitted selection receipt".into());
    }
    load_collection_proposal_v2(root, manifest, receipt)
}

pub fn verify_collection_v2(root: &Path) -> Result<CollectionManifestV2, String> {
    let manifest = read_collection_manifest_v2(root)?;
    let mut physical = BTreeSet::new();
    for receipt in &manifest.receipts {
        if receipt.proposal.is_some() {
            let mission = load_collection_proposal_v2(root, &manifest, receipt)?;
            if receipt.disposition == SelectionDispositionV2::Accepted
                && !physical.insert(mission.physical_sha256)
            {
                return Err("V2 admitted physical identity repeated".into());
            }
        }
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_winner_is_canonical_and_panels_do_not_inflate_unique_count() {
        let mut request = CollectionRequestV2::broad(MissionRecipeV2::default(), 1, 3);
        request.population = PopulationDeclarationV2::Stress {
            panels: ["a", "b"]
                .into_iter()
                .map(|id| crate::StressPanelV1 {
                    id: id.into(),
                    quota: 1,
                    constraints: vec![crate::AxisConstraintV1 {
                        axis: crate::CoverageAxisV1::Distance,
                        range: crate::RangeV1 {
                            min: 0.0,
                            max: 100000.0,
                        },
                    }],
                })
                .collect(),
        };
        let c = ChallengeReceiptV1 {
            raw: crate::ChallengeMeasurementV1 {
                sustained_barrier: true,
                elevation_transition: false,
                successive_crests: false,
                qualifying_crest_count: 0,
            },
            prepared: crate::ChallengeMeasurementV1 {
                sustained_barrier: true,
                elevation_transition: false,
                successive_crests: false,
                qualifying_crest_count: 0,
            },
            qualifies: true,
        };
        let mut receipts: Vec<_> = [2, 0, 1]
            .into_iter()
            .map(|index| SelectionReceiptV2 {
                index,
                structural_rejections: vec![],
                proposal: Some(CollectionEntryV1 {
                    index,
                    file: proposal_filename(index),
                    sha256: "a".repeat(64),
                    physical_sha256: if index == 2 { "b" } else { "a" }.repeat(64),
                }),
                challenge: Some(c.clone()),
                labels: vec![],
                matching_stress_panels: vec!["a".into(), "b".into()],
                selected_stress_panels: vec![],
                disposition: SelectionDispositionV2::StructuralExhaustion,
            })
            .collect();
        receipts.sort_by_key(|r| r.index);
        let mut state = SelectionState::default();
        for r in &mut receipts {
            (r.disposition, r.selected_stress_panels) = state.decide(&request, r);
        }
        assert_eq!(receipts[0].selected_stress_panels.len(), 2);
        assert_eq!(
            receipts[1].disposition,
            SelectionDispositionV2::PhysicalDuplicate { first_index: 0 }
        );
        assert_eq!(
            receipts[2].disposition,
            SelectionDispositionV2::StressExcluded
        );
        assert_eq!(SelectionCountsV2::from_receipts(&receipts).accepted, 1);
    }
}
