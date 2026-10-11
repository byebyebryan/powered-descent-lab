use crate::{
    ChallengeReceiptV1, CollectionRequestV2, ResolvedMissionV2, TransferGeometryV1,
    measure_transfer_geometry, prepare_pads_v2, sha256,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionDiagnosticFlagsV1 {
    pub maximum_raw_interpolation_error_m: f64,
    pub absolute_chord_barrier_change_m: f64,
    pub absolute_sustained_width_change_m: f64,
    pub admission_disagreement: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceQaSpecV1 {
    pub spacing_m: f64,
    pub regular_index_stride: u64,
    pub regular_index_offsets: Vec<u64>,
    pub boundary_extra_per_backend: usize,
    pub maximum_per_backend: usize,
    pub diagnostic_flags: ResolutionDiagnosticFlagsV1,
}

impl ReferenceQaSpecV1 {
    pub fn validate(&self, request: &CollectionRequestV2) -> Result<(), String> {
        request.validate()?;
        self.validate_recipe(&request.recipe)?;
        let regular = request
            .indices
            .iter()
            .filter(|&&i| self.is_regular(i))
            .count();
        if regular + self.boundary_extra_per_backend > self.maximum_per_backend {
            return Err("reference QA schedule exceeds its declared budget".into());
        }
        Ok(())
    }
    fn validate_recipe(&self, recipe: &crate::MissionRecipeV2) -> Result<(), String> {
        recipe.validate()?;
        if !self.spacing_m.is_finite()
            || self.spacing_m < 1.0
            || self.spacing_m > recipe.terrain_spacing_m
            || self.regular_index_stride == 0
            || self.regular_index_stride > 1_000_000
            || self.regular_index_offsets.is_empty()
            || self.regular_index_offsets.len() > 16
            || self
                .regular_index_offsets
                .iter()
                .any(|&i| i >= self.regular_index_stride)
            || self.regular_index_offsets.windows(2).any(|v| v[0] >= v[1])
            || self.boundary_extra_per_backend > 128
            || self.maximum_per_backend > 1024
            || !self.diagnostic_flags.admission_disagreement
            || ![
                self.diagnostic_flags.maximum_raw_interpolation_error_m,
                self.diagnostic_flags.absolute_chord_barrier_change_m,
                self.diagnostic_flags.absolute_sustained_width_change_m,
            ]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0 && *v <= 100000.0)
            || (recipe.travel_distance_m.max
                + 2.0 * recipe.domain_margin_m
                + 2.0 * recipe.terrain_spacing_m)
                / self.spacing_m
                > 100000.0
        {
            return Err("unsupported or unbounded reference QA declaration".into());
        }
        Ok(())
    }
    pub fn is_regular(&self, index: u64) -> bool {
        self.regular_index_stride > 0
            && self
                .regular_index_offsets
                .contains(&(index % self.regular_index_stride))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionFlagsV1 {
    pub raw_interpolation: bool,
    pub chord_barrier: bool,
    pub sustained_width: bool,
    pub admission_disagreement: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionQaReceiptV1 {
    pub index: u64,
    pub physical_sha256: String,
    pub coarse_raw_sha256: String,
    pub reference_spacing_m: f64,
    pub reference_raw_sha256: String,
    pub reference_prepared_sha256: String,
    /// Pad elevations are remeasured/rounded on the reference raw polyline.
    /// This is a counterfactual geometry comparison, never replacement physics.
    pub reference_pad_surface_y_m: [f64; 2],
    pub maximum_raw_interpolation_error_m: f64,
    pub chord_barrier_change_m: f64,
    pub sustained_width_change_m: f64,
    pub original_challenge: ChallengeReceiptV1,
    pub reference_challenge: ChallengeReceiptV1,
    pub reference_raw_geometry: TransferGeometryV1,
    pub reference_prepared_geometry: TransferGeometryV1,
    pub flags: ResolutionFlagsV1,
}

pub fn measure_reference_qa(
    mission: &ResolvedMissionV2,
    rule: &crate::ChallengeRuleV1,
    spec: &ReferenceQaSpecV1,
) -> Result<ResolutionQaReceiptV1, String> {
    mission.validate()?;
    spec.validate_recipe(&mission.recipe)?;
    rule.validate(&mission.geometry.spec)?;
    let raw = mission.raw_reference(spec.spacing_m)?;
    let mut pads = mission.scenario.world.landing_pads.clone();
    for pad in &mut pads {
        pad.surface_y_m = (raw.sample_height(pad.center_x_m) * 1000.0).round() / 1000.0;
    }
    let (prepared, edits) = prepare_pads_v2(&raw, &pads, mission.recipe.pad_transition_m)?;
    let raw_geometry =
        measure_transfer_geometry(&raw, &pads[0], &pads[1], &mission.geometry.spec, &[])?;
    let prepared_geometry = measure_transfer_geometry(
        &prepared,
        &pads[0],
        &pads[1],
        &mission.geometry.spec,
        &edits,
    )?;
    let maximum_raw_interpolation_error_m = raw
        .points()
        .iter()
        .map(|p| (p.y - mission.raw_terrain.sample_height(p.x)).abs())
        .fold(0.0, f64::max);
    let chord_barrier_change_m =
        prepared_geometry.max_chord_barrier_m - mission.geometry.geometry.max_chord_barrier_m;
    let width = |g: &TransferGeometryV1| {
        crate::admission::barrier(g, rule.barrier_height_m).map_or(0.0, |b| b.longest_width_m)
    };
    let sustained_width_change_m = width(&prepared_geometry) - width(&mission.geometry.geometry);
    let original_challenge =
        ChallengeReceiptV1::measure(rule, &mission.raw_geometry, &mission.geometry.geometry);
    let reference_challenge = ChallengeReceiptV1::measure(rule, &raw_geometry, &prepared_geometry);
    let thresholds = &spec.diagnostic_flags;
    let flags = ResolutionFlagsV1 {
        raw_interpolation: maximum_raw_interpolation_error_m
            > thresholds.maximum_raw_interpolation_error_m,
        chord_barrier: chord_barrier_change_m.abs() > thresholds.absolute_chord_barrier_change_m,
        sustained_width: sustained_width_change_m.abs()
            > thresholds.absolute_sustained_width_change_m,
        admission_disagreement: original_challenge.qualifies != reference_challenge.qualifies,
    };
    Ok(ResolutionQaReceiptV1 {
        index: mission.proposal.index,
        physical_sha256: mission.physical_sha256.clone(),
        coarse_raw_sha256: mission.raw_terrain_sha256.clone(),
        reference_spacing_m: spec.spacing_m,
        reference_raw_sha256: sha256(&serde_json::to_vec(&raw).map_err(|e| e.to_string())?),
        reference_prepared_sha256: sha256(
            &serde_json::to_vec(&prepared).map_err(|e| e.to_string())?,
        ),
        reference_pad_surface_y_m: [pads[0].surface_y_m, pads[1].surface_y_m],
        maximum_raw_interpolation_error_m,
        chord_barrier_change_m,
        sustained_width_change_m,
        original_challenge,
        reference_challenge,
        reference_raw_geometry: raw_geometry,
        reference_prepared_geometry: prepared_geometry,
        flags,
    })
}
