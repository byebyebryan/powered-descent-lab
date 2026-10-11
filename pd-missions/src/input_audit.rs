//! Bounded native inspection of sealed V2 inputs. No flights, outcomes or agents.
mod reference;
mod selection;
pub use reference::*;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::collection::read_regular_file;
use crate::{
    AxisValueV1, ChallengeReceiptV1, CollectionEntryV1, CollectionRequestV2, CoverageDesignV1,
    MorphologyLabelV1, PopulationCoverageV1, ResolvedMissionV2, SelectionCountsV2,
    SelectionDispositionV2, SelectionReceiptV2, load_collection_proposal_v2,
    measure_transfer_geometry, read_collection_manifest_v2, sha256, summarize_coverage,
};

pub const INPUT_AUDIT_VERSION: &str = "pd-missions/input-coverage-v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputAuditSpecV1 {
    pub schema_version: u32,
    pub audit_version: String,
    pub coverage: CoverageDesignV1,
    pub reference_qa: ReferenceQaSpecV1,
    pub preview_maximum: usize,
}

impl InputAuditSpecV1 {
    pub fn validate(&self, request: &CollectionRequestV2) -> Result<(), String> {
        request.validate()?;
        self.coverage.validate()?;
        self.reference_qa.validate(request)?;
        if self.schema_version != 1
            || self.audit_version != INPUT_AUDIT_VERSION
            || !(1..=192).contains(&self.preview_maximum)
        {
            return Err("unsupported or unbounded input audit specification".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputAuditRowV1 {
    pub index: u64,
    pub proposal: CollectionEntryV1,
    pub disposition: SelectionDispositionV2,
    pub direction: i32,
    pub labels: Vec<MorphologyLabelV1>,
    pub values: Vec<AxisValueV1>,
    pub scalar_metrics: BTreeMap<String, Option<f64>>,
    pub challenge: ChallengeReceiptV1,
    pub boundary_score: f64,
    pub shape_signature_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputCoverageAuditV1 {
    pub schema_version: u32,
    pub spec: InputAuditSpecV1,
    pub request: CollectionRequestV2,
    pub counts: SelectionCountsV2,
    pub proposed: PopulationCoverageV1,
    pub admitted: PopulationCoverageV1,
    pub rows: Vec<InputAuditRowV1>,
    pub reference_qa: Vec<ResolutionQaReceiptV1>,
    pub preview_indices: Vec<u64>,
    /// Declared immutable bytes were fully loaded/verified and authenticated
    /// again at completion. Usage receipts are separately append-only exposure.
    pub immutable_file_sha256: BTreeMap<String, String>,
    pub complete_saved_input_verification: bool,
}

pub fn admission_boundary_score(mission: &ResolvedMissionV2, rule: &crate::ChallengeRuleV1) -> f64 {
    let mut score = f64::INFINITY;
    for g in [&mission.raw_geometry, &mission.geometry.geometry] {
        let width =
            crate::admission::barrier(g, rule.barrier_height_m).map_or(0.0, |b| b.longest_width_m);
        for (v, threshold) in [
            (width, rule.sustained_width_m),
            (g.signed_pad_rise_m.abs(), rule.absolute_pad_rise_m),
        ] {
            score = score.min((v / threshold - 1.0).abs());
        }
        for c in &g.crests {
            for (v, threshold) in [
                (c.height_above_chord_m, rule.crest_height_m),
                (c.prominence_m, rule.crest_prominence_m),
                (c.half_prominence_support.width_m(), rule.crest_width_m),
            ] {
                score = score.min((v / threshold - 1.0).abs());
            }
        }
        for pair in g.crests.windows(2) {
            score = score.min(
                ((pair[1].position_m - pair[0].position_m) / rule.crest_separation_m - 1.0).abs(),
            );
        }
    }
    score
}

fn shape_signature(mission: &ResolvedMissionV2) -> Result<String, String> {
    let g = &mission.geometry.geometry;
    let source = &mission.scenario.world.landing_pads[0];
    let target = &mission.scenario.world.landing_pads[1];
    let mut signature = vec![
        (g.distance_m / 100.0).round() as i64,
        (g.signed_pad_rise_m / 100.0).round() as i64,
        (g.source_width_m / 8.0).round() as i64,
        (g.target_width_m / 8.0).round() as i64,
        g.crests.len() as i64,
    ];
    for i in 0..=12 {
        let fraction = i as f64 / 12.0;
        let x = source.center_x_m + (target.center_x_m - source.center_x_m) * fraction;
        let chord = source.surface_y_m + (target.surface_y_m - source.surface_y_m) * fraction;
        signature.push(
            ((mission.scenario.world.terrain.sample_height(x) - chord) / 50.0).round() as i64,
        );
    }
    Ok(sha256(
        &serde_json::to_vec(&signature).map_err(|e| e.to_string())?,
    ))
}

fn audit_row(
    mission: &ResolvedMissionV2,
    receipt: &SelectionReceiptV2,
    spec: &InputAuditSpecV1,
    request: &CollectionRequestV2,
) -> Result<InputAuditRowV1, String> {
    let g = &mission.geometry.geometry;
    let raw = &mission.raw_geometry;
    let pads = &mission.scenario.world.landing_pads;
    let raw_excluded = measure_transfer_geometry(
        &mission.raw_terrain,
        &pads[0],
        &pads[1],
        &mission.geometry.spec,
        &mission.geometry.pad_edit_intervals_x_m,
    )?;
    let mut scalar_metrics = BTreeMap::new();
    for (name, value) in [
        ("relief_m", g.relief_m),
        ("raw_relief_m", raw.relief_m),
        ("raw_chord_barrier_m", raw.max_chord_barrier_m),
        ("prepared_max_abs_slope", g.slopes.max_abs),
        ("raw_max_abs_slope", raw.slopes.max_abs),
        ("prepared_p95_abs_slope", g.slopes.p95_abs),
        ("raw_p95_abs_slope", raw.slopes.p95_abs),
        ("source_inward_rise_m", g.source_inward.max_rise_above_pad_m),
        (
            "source_outward_rise_m",
            g.source_outward.max_rise_above_pad_m,
        ),
        ("target_inward_rise_m", g.target_inward.max_rise_above_pad_m),
        (
            "target_outward_rise_m",
            g.target_outward.max_rise_above_pad_m,
        ),
        ("source_inward_fall_m", g.source_inward.max_fall_below_pad_m),
        (
            "source_outward_fall_m",
            g.source_outward.max_fall_below_pad_m,
        ),
        ("target_inward_fall_m", g.target_inward.max_fall_below_pad_m),
        (
            "target_outward_fall_m",
            g.target_outward.max_fall_below_pad_m,
        ),
        ("source_inward_length_m", g.source_inward.measured_length_m),
        (
            "source_outward_length_m",
            g.source_outward.measured_length_m,
        ),
        ("target_inward_length_m", g.target_inward.measured_length_m),
        (
            "target_outward_length_m",
            g.target_outward.measured_length_m,
        ),
        ("raw_crest_count", raw.crests.len() as f64),
        ("prepared_crest_count", g.crests.len() as f64),
        (
            "crest_count_change",
            g.crests.len() as f64 - raw.crests.len() as f64,
        ),
        (
            "target_width_to_hull_ratio",
            g.target_width_m / mission.scenario.vehicle.geometry.hull_width_m,
        ),
        (
            "distance_to_hull_ratio",
            g.distance_m / mission.scenario.vehicle.geometry.hull_width_m,
        ),
        (
            "source_width_to_hull_ratio",
            g.source_width_m / mission.scenario.vehicle.geometry.hull_width_m,
        ),
    ] {
        scalar_metrics.insert(name.to_owned(), Some(value));
    }
    let outside = g.slopes_outside_pad_edits.as_ref();
    let raw_outside = raw_excluded.slopes_outside_pad_edits.as_ref();
    scalar_metrics.insert(
        "outside_pad_max_abs_slope".into(),
        outside.map(|s| s.max_abs),
    );
    if let Some(b) = crate::admission::barrier(g, request.challenge_rule.barrier_height_m) {
        scalar_metrics.insert(
            "departure_barrier_fraction".into(),
            b.departure_distance_m.map(|v| v / g.distance_m),
        );
        scalar_metrics.insert(
            "approach_barrier_fraction".into(),
            b.approach_distance_m.map(|v| v / g.distance_m),
        );
    }
    scalar_metrics.insert(
        "outside_pad_p95_abs_slope".into(),
        outside.map(|s| s.p95_abs),
    );
    scalar_metrics.insert(
        "outside_pad_p95_change".into(),
        outside
            .zip(raw_outside)
            .map(|(a, b)| (a.p95_abs - b.p95_abs).abs()),
    );
    scalar_metrics.insert(
        "pad_edits_dominate_max_slope".into(),
        outside.map(|s| f64::from(g.slopes.max_abs > s.max_abs + 1e-8)),
    );
    let source_x = pads[0].center_x_m.min(pads[1].center_x_m);
    let target_x = pads[0].center_x_m.max(pads[1].center_x_m);
    let edited: f64 = mission
        .geometry
        .pad_edit_intervals_x_m
        .iter()
        .map(|e| (e.end_m.min(target_x) - e.start_m.max(source_x)).max(0.0))
        .sum();
    scalar_metrics.insert(
        "pad_edit_fraction_between_centers".into(),
        Some(edited / g.distance_m),
    );
    Ok(InputAuditRowV1 {
        index: receipt.index,
        proposal: receipt
            .proposal
            .clone()
            .ok_or("audit requires resolved proposal")?,
        disposition: receipt.disposition.clone(),
        direction: mission.direction,
        labels: receipt.labels.clone(),
        values: spec
            .coverage
            .axes
            .iter()
            .map(|a| AxisValueV1 {
                axis: a.axis,
                value: a.axis.value(g, request.challenge_rule.barrier_height_m),
            })
            .collect(),
        scalar_metrics,
        challenge: receipt
            .challenge
            .clone()
            .ok_or("audit requires challenge receipt")?,
        boundary_score: admission_boundary_score(mission, &request.challenge_rule),
        shape_signature_sha256: shape_signature(mission)?,
    })
}

/// Read-only audit with bounded row/reference/preview work. Explicit usage
/// receipts and create-only output belong to the caller, not this function.
pub fn audit_collection_v2(
    root: &Path,
    spec: InputAuditSpecV1,
) -> Result<InputCoverageAuditV1, String> {
    let mut immutable_file_sha256 = BTreeMap::new();
    for name in ["manifest.json", "request.json"] {
        immutable_file_sha256.insert(
            name.into(),
            sha256(&read_regular_file(&root.join(name), 64 * 1024 * 1024)?),
        );
    }
    let manifest = read_collection_manifest_v2(root)?;
    spec.validate(&manifest.request)?;
    let mut rows = Vec::new();
    let mut row_bytes = 0;
    for receipt in &manifest.receipts {
        if let Some(proposal) = &receipt.proposal {
            let mission = load_collection_proposal_v2(root, &manifest, receipt)?;
            let row = audit_row(&mission, receipt, &spec, &manifest.request)?;
            row_bytes += serde_json::to_vec(&row).map_err(|e| e.to_string())?.len();
            if row_bytes > 64 * 1024 * 1024 {
                return Err("input audit rows exceed bounded size".into());
            }
            immutable_file_sha256.insert(proposal.file.clone(), proposal.sha256.clone());
            rows.push(row);
        }
    }
    let proposed_rows: Vec<_> = rows.iter().collect();
    let admitted_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.disposition == SelectionDispositionV2::Accepted)
        .collect();
    let proposed = summarize_coverage(&proposed_rows, &spec.coverage)?;
    let admitted = summarize_coverage(&admitted_rows, &spec.coverage)?;
    let qa_indices = selection::qa_indices(&rows, &spec.reference_qa);
    let mut reference_qa = Vec::new();
    let mut qa_bytes = 0;
    for index in qa_indices {
        let position = manifest
            .request
            .indices
            .binary_search(&index)
            .map_err(|_| "QA index outside schedule")?;
        let mission = load_collection_proposal_v2(root, &manifest, &manifest.receipts[position])?;
        let qa = measure_reference_qa(
            &mission,
            &manifest.request.challenge_rule,
            &spec.reference_qa,
        )?;
        qa_bytes += serde_json::to_vec(&qa).map_err(|e| e.to_string())?.len();
        if qa_bytes > 32 * 1024 * 1024 {
            return Err("reference QA exceeds bounded receipt size".into());
        }
        reference_qa.push(qa);
    }
    let preview_indices = selection::preview_indices(&rows, &spec.coverage, spec.preview_maximum);
    for (file, digest) in &immutable_file_sha256 {
        let limit = if file == "manifest.json" {
            64 * 1024 * 1024
        } else {
            16 * 1024 * 1024
        };
        if sha256(&read_regular_file(&root.join(file), limit)?) != *digest {
            return Err("immutable collection input changed during audit".into());
        }
    }
    if proposed.count + manifest.counts.structural_exhaustion != manifest.counts.scheduled
        || admitted.count != manifest.counts.accepted
    {
        return Err("input audit population accounting mismatch".into());
    }
    Ok(InputCoverageAuditV1 {
        schema_version: 1,
        spec,
        request: manifest.request,
        counts: manifest.counts,
        proposed,
        admitted,
        rows,
        reference_qa,
        preview_indices,
        immutable_file_sha256,
        complete_saved_input_verification: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coarse_shape_bucket_is_translation_and_reflection_invariant() {
        let recipe = crate::MissionRecipeV2::default();
        let original =
            crate::generate_mission_v2(&recipe, &crate::GeometrySpecV1::default(), 981, 0)
                .unwrap()
                .mission
                .unwrap();
        let signature = shape_signature(&original).unwrap();
        for sign in [-1.0, 1.0] {
            // Synthetic coordinate transform for a descriptor test, without a
            // claim that its stored noise seed generated the transformed world.
            let mut shifted = original.clone();
            let mut points: Vec<_> = shifted
                .scenario
                .world
                .terrain
                .points()
                .iter()
                .map(|p| pd_core::Vec2::new(sign * p.x + 4.0, p.y))
                .collect();
            if sign < 0.0 {
                points.reverse();
            }
            shifted.scenario.world.terrain =
                pd_core::TerrainDefinition::Heightfield { points_m: points };
            for p in &mut shifted.scenario.world.landing_pads {
                p.center_x_m = sign * p.center_x_m + 4.0;
            }
            assert_eq!(shape_signature(&shifted).unwrap(), signature);
        }
    }
}
