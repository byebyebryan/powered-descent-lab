//! Input-only terrain-equivalence support evidence for the D1b design review.
//!
//! This module is deliberately a spike, not a capability implementation.  It
//! groups the already resolved neutral D0b rows using two predeclared keys and
//! records whether a future progress envelope could have enough leave-one-out
//! support.  It never opens a run manifest, reads an outcome overlay, fits a
//! model, or replaces the query's exact terrain with a signature.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

use crate::progress_interval_envelope::{LoadedDevelopmentRow, load_development_corpus};
use crate::{
    HullContainmentV1, HullPointV1, ProgressIntervalContinuousPhaseV1,
    ProgressIntervalDevelopmentCorpusV1, RouteCapabilityInputV1, canonical_digest,
    classify_hull_point, convex_hull,
};

pub const TERRAIN_EQUIVALENCE_SPIKE_SCHEMA_VERSION: u32 = 1;
pub const TERRAIN_EQUIVALENCE_SPIKE_COMMAND: &str = "terrain-equivalence-spike";
pub const TERRAIN_RELIEF_MOTIF_SIGNATURE_V1: &str = "route_relief_motif_v1";
pub const TERRAIN_EXACT_SIGNATURE_V1: &str = "exact_terrain_geometry_v1";
pub const TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS: usize = 3;
pub const TERRAIN_RELIEF_MOTIF_LOCATION_BOUNDARY_RULE_V1: &str =
    "derived_locations_within_1e-12_normalized_units_snap_to_nearest_0.1_boundary";

const MOTIF_LOCATION_BUCKET_WIDTH: f64 = 0.1;
const MOTIF_LOCATION_BUCKET_COUNT: u8 = 10;
const MOTIF_RELIEF_BUCKET_WIDTH_M: f64 = 10.0;
const MOTIF_LOCATION_BOUNDARY_SNAP_TOLERANCE: f64 = 1.0e-12;
const HULL_TOLERANCE: f64 = 1.0e-9;

/// A positive route-relative terrain relief peak after fixed quantization.
///
/// Location buckets are half-open except for the final closed bucket:
/// `[0.0, 0.1), ..., [0.9, 1.0]`.  Heights use nearest 10 m, with exact
/// half-way values rounded upward.  Only positive relief is admitted.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RouteReliefMotifPeakV1 {
    pub location_bucket: u8,
    pub relief_height_bucket: i32,
}

impl RouteReliefMotifPeakV1 {
    fn validate(self) -> Result<(), String> {
        if self.location_bucket >= MOTIF_LOCATION_BUCKET_COUNT || self.relief_height_bucket < 0 {
            return Err("route relief motif peak bucket is invalid".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RouteReliefMotifV1 {
    pub peak_count: usize,
    pub peaks: Vec<RouteReliefMotifPeakV1>,
}

impl RouteReliefMotifV1 {
    fn validate(&self) -> Result<(), String> {
        if self.peak_count != self.peaks.len() {
            return Err("route relief motif peak count does not match peaks".to_owned());
        }
        if self.peaks.windows(2).any(|pair| pair[0] > pair[1]) {
            return Err("route relief motif peaks are not sorted".to_owned());
        }
        for peak in &self.peaks {
            peak.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerrainEquivalenceTerrainKeyV1 {
    Exact { terrain_geometry_digest: String },
    RouteReliefMotif { motif: RouteReliefMotifV1 },
}

impl TerrainEquivalenceTerrainKeyV1 {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Exact {
                terrain_geometry_digest,
            } if terrain_geometry_digest.trim().is_empty() => {
                Err("exact terrain signature digest must not be empty".to_owned())
            }
            Self::Exact { .. } => Ok(()),
            Self::RouteReliefMotif { motif } => motif.validate(),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TerrainEquivalenceGroupKeyV1 {
    pub terrain: TerrainEquivalenceTerrainKeyV1,
    pub vehicle_physics_digest: String,
    pub waypoint_count: usize,
}

impl TerrainEquivalenceGroupKeyV1 {
    fn validate(&self) -> Result<(), String> {
        self.terrain.validate()?;
        if self.vehicle_physics_digest.trim().is_empty()
            || self.waypoint_count == 0
            || self.waypoint_count > 2
        {
            return Err("terrain equivalence group key is invalid".to_owned());
        }
        Ok(())
    }
}

/// Input-only binding retained so an eventual clearance proof can retrieve
/// the exact query terrain by digest.  The terrain itself is intentionally not
/// copied into this diagnostic artifact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainEquivalenceInputBindingV1 {
    pub row_id: String,
    pub corpus: ProgressIntervalDevelopmentCorpusV1,
    pub input_digest: String,
    pub resolved_input_digest: String,
    pub evidence_physical_digest: String,
    pub exact_terrain_digest: String,
    pub route_relief_motif: RouteReliefMotifV1,
    pub vehicle_physics_digest: String,
    pub waypoint_count: usize,
    pub observed_required_cell_ids: Vec<String>,
    pub route_point: HullPointV1,
}

impl TerrainEquivalenceInputBindingV1 {
    fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("row_id", self.row_id.as_str()),
            ("input_digest", self.input_digest.as_str()),
            ("resolved_input_digest", self.resolved_input_digest.as_str()),
            (
                "evidence_physical_digest",
                self.evidence_physical_digest.as_str(),
            ),
            ("exact_terrain_digest", self.exact_terrain_digest.as_str()),
            (
                "vehicle_physics_digest",
                self.vehicle_physics_digest.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(format!("terrain equivalence binding {name} is empty"));
            }
        }
        self.route_relief_motif.validate()?;
        if self.waypoint_count == 0 || self.waypoint_count > 2 {
            return Err("terrain equivalence binding waypoint count is invalid".to_owned());
        }
        if self
            .observed_required_cell_ids
            .iter()
            .any(|cell_id| cell_id.trim().is_empty())
            || self
                .observed_required_cell_ids
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self
                .observed_required_cell_ids
                .iter()
                .any(|cell_id| !required_cell_ids(self.waypoint_count).contains(cell_id))
        {
            return Err("terrain equivalence observed cell IDs are not canonical".to_owned());
        }
        validate_hull_point(self.route_point)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainEquivalenceCellSupportV1 {
    pub cell_id: String,
    pub resolved_input_digests: Vec<String>,
    pub distinct_resolved_input_digest_count: usize,
    pub minimum_leave_one_out_distinct_resolved_input_digest_count: usize,
}

impl TerrainEquivalenceCellSupportV1 {
    fn validate(&self) -> Result<(), String> {
        if self.cell_id.trim().is_empty() {
            return Err("terrain equivalence cell ID must not be empty".to_owned());
        }
        if self
            .resolved_input_digests
            .iter()
            .any(|digest| digest.trim().is_empty())
            || self
                .resolved_input_digests
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self.distinct_resolved_input_digest_count != self.resolved_input_digests.len()
            || self.minimum_leave_one_out_distinct_resolved_input_digest_count
                > self.distinct_resolved_input_digest_count
        {
            return Err("terrain equivalence cell digest membership is not canonical".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainEquivalenceLeaveOneOutRowV1 {
    pub row_id: String,
    pub excluded_resolved_input_digest: String,
    pub route_point: HullPointV1,
    pub same_group_distinct_resolved_input_digest_support: usize,
    pub route_angle_radius_hull_after_exclusion: Vec<HullPointV1>,
    pub route_angle_radius_hull_eligible: bool,
    pub minimum_required_cell_support: usize,
    pub eligible_for_design: bool,
}

impl TerrainEquivalenceLeaveOneOutRowV1 {
    fn validate(&self) -> Result<(), String> {
        if self.row_id.trim().is_empty() || self.excluded_resolved_input_digest.trim().is_empty() {
            return Err("terrain equivalence leave-one-out row identity is empty".to_owned());
        }
        validate_hull_point(self.route_point)?;
        for point in &self.route_angle_radius_hull_after_exclusion {
            validate_hull_point(*point)?;
        }
        if self.route_angle_radius_hull_after_exclusion.is_empty()
            && self.route_angle_radius_hull_eligible
        {
            return Err("empty route hull cannot be eligible".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainEquivalenceGroupV1 {
    pub key: TerrainEquivalenceGroupKeyV1,
    pub key_digest: String,
    pub row_ids: Vec<String>,
    pub distinct_resolved_input_digests: Vec<String>,
    pub exact_terrain_digests: Vec<String>,
    pub route_angle_radius_hull: Vec<HullPointV1>,
    pub cell_support: Vec<TerrainEquivalenceCellSupportV1>,
    pub leave_one_out: Vec<TerrainEquivalenceLeaveOneOutRowV1>,
    pub minimum_leave_one_out_group_support: usize,
    pub minimum_leave_one_out_cell_support: usize,
    pub all_rows_meet_minimum_support: bool,
}

impl TerrainEquivalenceGroupV1 {
    fn validate(&self) -> Result<(), String> {
        self.key.validate()?;
        if self.key_digest != canonical_digest(&self.key)?
            || self.row_ids.is_empty()
            || self.row_ids.windows(2).any(|pair| pair[0] >= pair[1])
            || self.distinct_resolved_input_digests.is_empty()
            || self
                .distinct_resolved_input_digests
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self.exact_terrain_digests.is_empty()
            || self
                .exact_terrain_digests
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err("terrain equivalence group ordering or identity is invalid".to_owned());
        }
        for point in &self.route_angle_radius_hull {
            validate_hull_point(*point)?;
        }
        for cell in &self.cell_support {
            cell.validate()?;
        }
        let expected_cell_ids = required_cell_ids(self.key.waypoint_count);
        let actual_cell_ids = self
            .cell_support
            .iter()
            .map(|cell| cell.cell_id.clone())
            .collect::<Vec<_>>();
        if actual_cell_ids != expected_cell_ids {
            return Err("terrain equivalence group required cells are not canonical".to_owned());
        }
        if self.leave_one_out.len() != self.row_ids.len()
            || self
                .leave_one_out
                .windows(2)
                .any(|pair| pair[0].row_id >= pair[1].row_id)
        {
            return Err("terrain equivalence leave-one-out rows are incomplete".to_owned());
        }
        for row in &self.leave_one_out {
            row.validate()?;
        }
        if self.minimum_leave_one_out_group_support
            != self
                .leave_one_out
                .iter()
                .map(|row| row.same_group_distinct_resolved_input_digest_support)
                .min()
                .unwrap_or(0)
            || self.minimum_leave_one_out_cell_support
                != self
                    .leave_one_out
                    .iter()
                    .map(|row| row.minimum_required_cell_support)
                    .min()
                    .unwrap_or(0)
        {
            return Err("terrain equivalence group minima are inconsistent".to_owned());
        }
        if self.all_rows_meet_minimum_support
            != self.leave_one_out.iter().all(|row| row.eligible_for_design)
        {
            return Err("terrain equivalence group eligibility is inconsistent".to_owned());
        }
        Ok(())
    }

    fn validate_membership(
        &self,
        signature_id: &str,
        input_rows: &BTreeMap<String, &TerrainEquivalenceInputBindingV1>,
    ) -> Result<(), String> {
        self.validate()?;
        let expected_resolved = self
            .row_ids
            .iter()
            .map(|row_id| {
                input_rows
                    .get(row_id)
                    .ok_or_else(|| format!("group references unknown input row {row_id}"))
                    .map(|row| row.resolved_input_digest.clone())
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let expected_terrain = self
            .row_ids
            .iter()
            .map(|row_id| {
                input_rows
                    .get(row_id)
                    .ok_or_else(|| format!("group references unknown input row {row_id}"))
                    .map(|row| row.exact_terrain_digest.clone())
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        if expected_resolved.iter().cloned().collect::<Vec<_>>()
            != self.distinct_resolved_input_digests
            || expected_terrain.iter().cloned().collect::<Vec<_>>() != self.exact_terrain_digests
        {
            return Err("terrain equivalence group digest membership is inconsistent".to_owned());
        }
        for row_id in &self.row_ids {
            let row = input_rows
                .get(row_id)
                .ok_or_else(|| format!("group references unknown input row {row_id}"))?;
            let expected_key = group_key_for_binding(signature_id, row)?;
            if expected_key != self.key {
                return Err(format!("row {row_id} is assigned to the wrong group key"));
            }
        }
        let expected_hull = convex_hull(
            &self
                .row_ids
                .iter()
                .map(|row_id| input_rows[row_id].route_point)
                .collect::<Vec<_>>(),
            HULL_TOLERANCE,
        )?;
        if expected_hull != self.route_angle_radius_hull {
            return Err("terrain equivalence group hull membership is inconsistent".to_owned());
        }
        for cell in &self.cell_support {
            let expected_cell_digests = self
                .row_ids
                .iter()
                .filter(|row_id| {
                    input_rows[*row_id]
                        .observed_required_cell_ids
                        .binary_search(&cell.cell_id)
                        .is_ok()
                })
                .map(|row_id| input_rows[row_id].resolved_input_digest.clone())
                .collect::<BTreeSet<_>>();
            if expected_cell_digests.iter().cloned().collect::<Vec<_>>()
                != cell.resolved_input_digests
            {
                return Err(format!(
                    "cell {} digest membership does not match input bindings",
                    cell.cell_id
                ));
            }
            let expected_minimum = self
                .row_ids
                .iter()
                .map(|row_id| {
                    let excluded = &input_rows[row_id].resolved_input_digest;
                    cell.resolved_input_digests.len()
                        - usize::from(cell.resolved_input_digests.contains(excluded))
                })
                .min()
                .unwrap_or(0);
            if cell.minimum_leave_one_out_distinct_resolved_input_digest_count != expected_minimum {
                return Err(format!(
                    "cell {} leave-one-out support is inconsistent",
                    cell.cell_id
                ));
            }
        }
        let mut expected_loo_ids = self.row_ids.clone();
        expected_loo_ids.sort();
        let actual_loo_ids = self
            .leave_one_out
            .iter()
            .map(|row| row.row_id.clone())
            .collect::<Vec<_>>();
        if actual_loo_ids != expected_loo_ids {
            return Err("terrain equivalence leave-one-out membership is inconsistent".to_owned());
        }
        for loo in &self.leave_one_out {
            let input = input_rows
                .get(&loo.row_id)
                .ok_or_else(|| format!("leave-one-out references unknown row {}", loo.row_id))?;
            if loo.excluded_resolved_input_digest != input.resolved_input_digest
                || loo.route_point != input.route_point
            {
                return Err(format!(
                    "leave-one-out identity does not match input row {}",
                    loo.row_id
                ));
            }
            let fit_rows = self
                .row_ids
                .iter()
                .filter(|row_id| {
                    input_rows[*row_id].resolved_input_digest != loo.excluded_resolved_input_digest
                })
                .collect::<Vec<_>>();
            let fit_hull = if fit_rows.is_empty() {
                Vec::new()
            } else {
                convex_hull(
                    &fit_rows
                        .iter()
                        .map(|row_id| input_rows[*row_id].route_point)
                        .collect::<Vec<_>>(),
                    HULL_TOLERANCE,
                )?
            };
            if fit_hull != loo.route_angle_radius_hull_after_exclusion {
                return Err(format!(
                    "leave-one-out hull does not match row {}",
                    loo.row_id
                ));
            }
            let expected_hull_eligible = !fit_hull.is_empty()
                && classify_hull_point(&fit_hull, input.route_point, HULL_TOLERANCE)?
                    != HullContainmentV1::Outside;
            if loo.route_angle_radius_hull_eligible != expected_hull_eligible {
                return Err(format!(
                    "leave-one-out hull eligibility does not match row {}",
                    loo.row_id
                ));
            }
            let expected_group_support = fit_rows
                .iter()
                .map(|row_id| input_rows[*row_id].resolved_input_digest.clone())
                .collect::<BTreeSet<_>>()
                .len();
            let expected_cell_support = self
                .cell_support
                .iter()
                .map(|cell| {
                    cell.resolved_input_digests.len()
                        - usize::from(
                            cell.resolved_input_digests
                                .contains(&loo.excluded_resolved_input_digest),
                        )
                })
                .min()
                .unwrap_or(0);
            if loo.same_group_distinct_resolved_input_digest_support != expected_group_support
                || loo.minimum_required_cell_support != expected_cell_support
            {
                return Err(format!(
                    "leave-one-out support does not match row {}",
                    loo.row_id
                ));
            }
            let baseline_hull_gate = input.corpus != ProgressIntervalDevelopmentCorpusV1::Baseline
                || expected_hull_eligible;
            let expected_eligible = expected_group_support
                >= TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS
                && baseline_hull_gate
                && expected_cell_support >= TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS;
            if loo.eligible_for_design != expected_eligible {
                return Err(format!(
                    "leave-one-out eligibility does not match row {}",
                    loo.row_id
                ));
            }
        }
        let expected_min_group = self
            .leave_one_out
            .iter()
            .map(|row| row.same_group_distinct_resolved_input_digest_support)
            .min()
            .unwrap_or(0);
        let expected_min_cell = self
            .leave_one_out
            .iter()
            .map(|row| row.minimum_required_cell_support)
            .min()
            .unwrap_or(0);
        if self.minimum_leave_one_out_group_support != expected_min_group
            || self.minimum_leave_one_out_cell_support != expected_min_cell
            || self.all_rows_meet_minimum_support
                != self.leave_one_out.iter().all(|row| row.eligible_for_design)
        {
            return Err(
                "terrain equivalence group minima or eligibility is inconsistent".to_owned(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainEquivalenceSignatureReportV1 {
    pub signature_id: String,
    pub signature_description: String,
    pub group_count: usize,
    pub groups: Vec<TerrainEquivalenceGroupV1>,
    pub eligible_row_count: usize,
    pub minimum_leave_one_out_group_support: usize,
    pub minimum_leave_one_out_cell_support: usize,
    pub all_baseline_route_points_hull_eligible: bool,
    pub viable_for_a2_design: bool,
    pub failure_reasons: Vec<String>,
}

impl TerrainEquivalenceSignatureReportV1 {
    fn validate(&self) -> Result<(), String> {
        if self.signature_id.trim().is_empty()
            || self.signature_description.trim().is_empty()
            || self.group_count != self.groups.len()
            || self.groups.is_empty()
            || self
                .groups
                .windows(2)
                .any(|pair| pair[0].key >= pair[1].key)
        {
            return Err("terrain equivalence signature report is not canonical".to_owned());
        }
        for group in &self.groups {
            group.validate()?;
        }
        let expected_eligible = self
            .groups
            .iter()
            .flat_map(|group| group.leave_one_out.iter())
            .filter(|row| row.eligible_for_design)
            .count();
        if expected_eligible != self.eligible_row_count
            || self.minimum_leave_one_out_group_support
                != self
                    .groups
                    .iter()
                    .map(|group| group.minimum_leave_one_out_group_support)
                    .min()
                    .unwrap_or(0)
            || self.minimum_leave_one_out_cell_support
                != self
                    .groups
                    .iter()
                    .map(|group| group.minimum_leave_one_out_cell_support)
                    .min()
                    .unwrap_or(0)
        {
            return Err("terrain equivalence signature minima are inconsistent".to_owned());
        }
        if self.viable_for_a2_design
            && (!self.all_baseline_route_points_hull_eligible
                || self.eligible_row_count
                    != self
                        .groups
                        .iter()
                        .map(|group| group.row_ids.len())
                        .sum::<usize>()
                || self.minimum_leave_one_out_group_support
                    < TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS
                || self.minimum_leave_one_out_cell_support
                    < TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS)
        {
            return Err("viable terrain equivalence report lacks required support".to_owned());
        }
        Ok(())
    }

    fn validate_membership(
        &self,
        input_rows: &[TerrainEquivalenceInputBindingV1],
    ) -> Result<(), String> {
        self.validate()?;
        if !matches!(
            self.signature_id.as_str(),
            TERRAIN_EXACT_SIGNATURE_V1 | TERRAIN_RELIEF_MOTIF_SIGNATURE_V1
        ) {
            return Err(format!(
                "unknown terrain equivalence signature {}",
                self.signature_id
            ));
        }
        let input_by_id = input_rows
            .iter()
            .map(|row| (row.row_id.clone(), row))
            .collect::<BTreeMap<_, _>>();
        let mut covered = BTreeSet::new();
        for group in &self.groups {
            group.validate_membership(&self.signature_id, &input_by_id)?;
            for row_id in &group.row_ids {
                if !covered.insert(row_id.clone()) {
                    return Err(format!("input row {row_id} occurs in multiple groups"));
                }
            }
        }
        if covered.len() != input_by_id.len() || covered.iter().ne(input_by_id.keys()) {
            return Err(
                "terrain equivalence signature does not cover inputs exactly once".to_owned(),
            );
        }
        let expected_eligible = self
            .groups
            .iter()
            .flat_map(|group| group.leave_one_out.iter())
            .filter(|row| row.eligible_for_design)
            .count();
        let expected_min_group = self
            .groups
            .iter()
            .map(|group| group.minimum_leave_one_out_group_support)
            .min()
            .unwrap_or(0);
        let expected_min_cell = self
            .groups
            .iter()
            .map(|group| group.minimum_leave_one_out_cell_support)
            .min()
            .unwrap_or(0);
        let expected_baseline_hull = self.groups.iter().all(|group| {
            group.leave_one_out.iter().all(|row| {
                input_by_id.get(&row.row_id).is_some_and(|input| {
                    input.corpus != ProgressIntervalDevelopmentCorpusV1::Baseline
                        || row.route_angle_radius_hull_eligible
                })
            })
        });
        let mut expected_failures = Vec::new();
        if expected_eligible != input_rows.len() {
            expected_failures.push("leave_one_out_support_incomplete".to_owned());
        }
        if !expected_baseline_hull {
            expected_failures.push("baseline_route_point_outside_leave_one_out_hull".to_owned());
        }
        if expected_min_cell < TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS {
            expected_failures.push("required_phase_cell_undercovered".to_owned());
        }
        let expected_viable = expected_failures.is_empty()
            && expected_min_group >= TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS
            && expected_min_cell >= TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS;
        if self.eligible_row_count != expected_eligible
            || self.minimum_leave_one_out_group_support != expected_min_group
            || self.minimum_leave_one_out_cell_support != expected_min_cell
            || self.all_baseline_route_points_hull_eligible != expected_baseline_hull
            || self.failure_reasons != expected_failures
            || self.viable_for_a2_design != expected_viable
        {
            return Err("terrain equivalence signature derived fields are inconsistent".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainEquivalenceSpikeArtifactV1 {
    pub schema_version: u32,
    pub source_manifest_id: String,
    pub source_d0_input_digest: String,
    pub input_manifest_digest: String,
    pub baseline_row_ids: Vec<String>,
    pub diagnostic_row_ids: Vec<String>,
    pub baseline_resolved_input_digest: String,
    pub diagnostic_resolved_input_digest: String,
    pub row_count: usize,
    pub unique_resolved_input_digest_count: usize,
    pub exact_query_terrain_required: bool,
    pub input_rows: Vec<TerrainEquivalenceInputBindingV1>,
    pub signatures: Vec<TerrainEquivalenceSignatureReportV1>,
    pub viable_for_a2_design: bool,
    pub decision_reasons: Vec<String>,
    pub artifact_digest: String,
}

impl TerrainEquivalenceSpikeArtifactV1 {
    fn compute_digest(&self) -> Result<String, String> {
        let mut material = self.clone();
        material.artifact_digest.clear();
        canonical_digest(&material)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != TERRAIN_EQUIVALENCE_SPIKE_SCHEMA_VERSION
            || self.source_manifest_id.trim().is_empty()
            || self.source_d0_input_digest.trim().is_empty()
            || self.input_manifest_digest.trim().is_empty()
            || self.baseline_resolved_input_digest.trim().is_empty()
            || self.diagnostic_resolved_input_digest.trim().is_empty()
            || self.row_count != self.input_rows.len()
            || self.baseline_row_ids.is_empty()
            || self.diagnostic_row_ids.is_empty()
            || !self.exact_query_terrain_required
            || self.signatures.len() != 2
            || self.artifact_digest != self.compute_digest()?
        {
            return Err("terrain equivalence artifact identity or shape is invalid".to_owned());
        }
        if self
            .input_rows
            .windows(2)
            .any(|pair| pair[0].row_id >= pair[1].row_id)
        {
            return Err("terrain equivalence input rows are not uniquely sorted".to_owned());
        }
        let mut resolved = BTreeSet::new();
        for row in &self.input_rows {
            row.validate()?;
            if !resolved.insert(row.resolved_input_digest.clone()) {
                return Err("terrain equivalence resolved input digests are not unique".to_owned());
            }
        }
        if resolved.len() != self.unique_resolved_input_digest_count {
            return Err("terrain equivalence resolved digest count is inconsistent".to_owned());
        }
        if self.input_manifest_digest != canonical_digest(&self.input_rows)? {
            return Err("terrain equivalence input manifest digest mismatch".to_owned());
        }
        let input_by_id = self
            .input_rows
            .iter()
            .map(|row| (row.row_id.clone(), row))
            .collect::<BTreeMap<_, _>>();
        validate_corpus_row_order(
            &self.baseline_row_ids,
            ProgressIntervalDevelopmentCorpusV1::Baseline,
            &input_by_id,
        )?;
        validate_corpus_row_order(
            &self.diagnostic_row_ids,
            ProgressIntervalDevelopmentCorpusV1::Diagnostic,
            &input_by_id,
        )?;
        let mut corpus_rows = self.baseline_row_ids.clone();
        corpus_rows.extend(self.diagnostic_row_ids.iter().cloned());
        let expected_ids = input_by_id.keys().cloned().collect::<BTreeSet<_>>();
        let actual_ids = corpus_rows.iter().cloned().collect::<BTreeSet<_>>();
        if corpus_rows.len() != self.row_count
            || actual_ids != expected_ids
            || actual_ids.len() != corpus_rows.len()
        {
            return Err(
                "terrain equivalence corpus row binding is incomplete or duplicated".to_owned(),
            );
        }
        if self.baseline_resolved_input_digest
            != corpus_digest_for_order(&self.baseline_row_ids, &input_by_id)?
            || self.diagnostic_resolved_input_digest
                != corpus_digest_for_order(&self.diagnostic_row_ids, &input_by_id)?
        {
            return Err("terrain equivalence corpus digest binding mismatch".to_owned());
        }
        if self.signatures[0].signature_id != TERRAIN_EXACT_SIGNATURE_V1
            || self.signatures[1].signature_id != TERRAIN_RELIEF_MOTIF_SIGNATURE_V1
        {
            return Err("terrain equivalence signatures are not in canonical order".to_owned());
        }
        for signature in &self.signatures {
            signature.validate_membership(&self.input_rows)?;
        }
        let motif = &self.signatures[1];
        let expected_decision_reasons = if motif.viable_for_a2_design {
            vec!["route_relief_motif_has_design_support".to_owned()]
        } else {
            motif.failure_reasons.clone()
        };
        if self.viable_for_a2_design != motif.viable_for_a2_design
            || self.decision_reasons != expected_decision_reasons
        {
            return Err("terrain equivalence artifact decision is inconsistent".to_owned());
        }
        Ok(())
    }

    fn seal(mut self) -> Result<Self, String> {
        self.artifact_digest = self.compute_digest()?;
        self.validate()?;
        Ok(self)
    }
}

fn validate_corpus_row_order(
    row_ids: &[String],
    corpus: ProgressIntervalDevelopmentCorpusV1,
    input_by_id: &BTreeMap<String, &TerrainEquivalenceInputBindingV1>,
) -> Result<(), String> {
    if row_ids.iter().any(|row_id| row_id.trim().is_empty())
        || row_ids.windows(2).any(|pair| pair[0] == pair[1])
        || row_ids.iter().any(|row_id| {
            input_by_id
                .get(row_id)
                .is_none_or(|row| row.corpus != corpus)
        })
    {
        return Err("terrain equivalence corpus row order is invalid".to_owned());
    }
    Ok(())
}

fn corpus_digest_for_order(
    row_ids: &[String],
    input_by_id: &BTreeMap<String, &TerrainEquivalenceInputBindingV1>,
) -> Result<String, String> {
    let resolved = row_ids
        .iter()
        .map(|row_id| {
            input_by_id
                .get(row_id)
                .ok_or_else(|| format!("corpus row {row_id} is missing from input binding"))
                .map(|row| row.resolved_input_digest.clone())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!(
        "fnv1a64:{}",
        crate::source_transition_canonical_digest(&resolved)
    ))
}

#[derive(Clone, Copy, Debug)]
struct ReliefKnot {
    normalized_location: f64,
    relief_m: f64,
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

fn validate_hull_point(point: HullPointV1) -> Result<(), String> {
    if !point.route_angle_deg.is_finite() || !point.route_radius_m.is_finite() {
        return Err("hull points must be finite".to_owned());
    }
    if point.route_angle_deg.to_bits() == (-0.0_f64).to_bits()
        || point.route_radius_m.to_bits() == (-0.0_f64).to_bits()
    {
        return Err("hull points must use canonical +0.0".to_owned());
    }
    Ok(())
}

fn location_bucket(value: f64) -> Result<u8, String> {
    let value = canonical_zero(value);
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err("route relief location must be finite and within [0, 1]".to_owned());
    }
    if value == 1.0 {
        return Ok(MOTIF_LOCATION_BUCKET_COUNT - 1);
    }
    let bucket = (value / MOTIF_LOCATION_BUCKET_WIDTH).floor();
    if !(0.0..f64::from(MOTIF_LOCATION_BUCKET_COUNT)).contains(&bucket) {
        return Err("route relief location bucket is out of range".to_owned());
    }
    Ok(bucket as u8)
}

/// Normalize a location derived from world-space route-frame arithmetic before
/// applying the strict public bucket rule.  The tolerance is fixed and
/// versioned: only values within 1e-12 normalized units of a 0.1 boundary are
/// snapped; values beyond it remain on their actual side of the bucket.
/// Signed zero is canonicalized to +0.0 at this boundary.
fn snap_derived_location(value: f64) -> Result<f64, String> {
    let value = canonical_zero(value);
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err("derived route relief location must be finite and within [0, 1]".to_owned());
    }
    let boundary_index = (value / MOTIF_LOCATION_BUCKET_WIDTH).round();
    let boundary = boundary_index * MOTIF_LOCATION_BUCKET_WIDTH;
    if (value - boundary).abs() <= MOTIF_LOCATION_BOUNDARY_SNAP_TOLERANCE {
        Ok(canonical_zero(boundary.clamp(0.0, 1.0)))
    } else {
        Ok(value)
    }
}

fn relief_height_bucket(value: f64) -> Result<i32, String> {
    if !value.is_finite() || value <= 0.0 {
        return Err("route relief height must be positive and finite".to_owned());
    }
    let bucket = (value / MOTIF_RELIEF_BUCKET_WIDTH_M + 0.5).floor();
    if !bucket.is_finite() || bucket > f64::from(i32::MAX) {
        return Err("route relief height bucket is out of range".to_owned());
    }
    Ok(bucket as i32)
}

fn route_relief_motif(input: &RouteCapabilityInputV1) -> Result<RouteReliefMotifV1, String> {
    input.validate()?;
    let physical = &input.physical;
    let span = physical.safety_profile.horizontal_span_m;
    let sign = f64::from(physical.horizontal_sign);
    if !span.is_finite() || span <= 0.0 {
        return Err("route relief requires a positive finite route span".to_owned());
    }
    let free_start = physical.safety_profile.source_transition_end_m / span;
    let free_end = physical.safety_profile.target_transition_start_m / span;
    if !free_start.is_finite() || !free_end.is_finite() || free_start >= free_end {
        return Err("route relief free corridor is invalid".to_owned());
    }

    let source = physical.source_pad;
    let target = physical.target_pad;
    let world_x = |location: f64| source.center_x_m + (sign * span * location);
    let chord_y =
        |location: f64| source.surface_y_m + ((target.surface_y_m - source.surface_y_m) * location);
    let relief_at = |location: f64| -> Result<f64, String> {
        let terrain_y = physical
            .terrain
            .sample_height_strict(world_x(location))
            .map_err(|error| format!("route relief terrain query failed: {error}"))?;
        Ok(canonical_zero(terrain_y - chord_y(location)))
    };

    let mut knots = vec![
        ReliefKnot {
            normalized_location: free_start,
            relief_m: relief_at(free_start)?,
        },
        ReliefKnot {
            normalized_location: free_end,
            relief_m: relief_at(free_end)?,
        },
    ];
    for point in physical.terrain.points() {
        let raw_location = sign * (point.x - source.center_x_m) / span;
        if !(0.0..=1.0).contains(&raw_location) {
            continue;
        }
        if raw_location > free_start && raw_location < free_end {
            knots.push(ReliefKnot {
                // Keep the unsnapped physical coordinate for piecewise-linear
                // relief evaluation.  Snapping is a key-quantization rule
                // only; moving the knot would change chord height and could
                // manufacture a different height bucket.
                normalized_location: raw_location,
                relief_m: canonical_zero(point.y - chord_y(raw_location)),
            });
        }
    }
    knots.sort_by(|left, right| {
        left.normalized_location
            .total_cmp(&right.normalized_location)
    });
    if knots
        .windows(2)
        .any(|pair| pair[0].normalized_location >= pair[1].normalized_location)
    {
        return Err("route relief knots are not strictly ordered".to_owned());
    }

    let mut peaks = Vec::new();
    for (index, knot) in knots
        .iter()
        .enumerate()
        .skip(1)
        .take(knots.len().saturating_sub(2))
    {
        let previous = knots[index - 1].relief_m;
        let next = knots[index + 1].relief_m;
        if knot.relief_m > 0.0
            && knot.relief_m >= previous
            && knot.relief_m >= next
            && (knot.relief_m > previous || knot.relief_m > next)
        {
            peaks.push(RouteReliefMotifPeakV1 {
                location_bucket: location_bucket(snap_derived_location(knot.normalized_location)?)?,
                relief_height_bucket: relief_height_bucket(knot.relief_m)?,
            });
        }
    }
    peaks.sort();
    let motif = RouteReliefMotifV1 {
        peak_count: peaks.len(),
        peaks,
    };
    motif.validate()?;
    Ok(motif)
}

fn exact_terrain_digest(input: &RouteCapabilityInputV1) -> Result<String, String> {
    canonical_digest(&input.physical.terrain)
}

fn vehicle_physics_digest(input: &RouteCapabilityInputV1) -> Result<String, String> {
    canonical_digest(&input.physical.vehicle)
}

fn exact_group_key(row: &LoadedDevelopmentRow) -> Result<TerrainEquivalenceGroupKeyV1, String> {
    Ok(TerrainEquivalenceGroupKeyV1 {
        terrain: TerrainEquivalenceTerrainKeyV1::Exact {
            terrain_geometry_digest: exact_terrain_digest(&row.input)?,
        },
        vehicle_physics_digest: vehicle_physics_digest(&row.input)?,
        waypoint_count: row.input.physical.waypoints.len(),
    })
}

fn motif_group_key(row: &LoadedDevelopmentRow) -> Result<TerrainEquivalenceGroupKeyV1, String> {
    Ok(TerrainEquivalenceGroupKeyV1 {
        terrain: TerrainEquivalenceTerrainKeyV1::RouteReliefMotif {
            motif: route_relief_motif(&row.input)?,
        },
        vehicle_physics_digest: vehicle_physics_digest(&row.input)?,
        waypoint_count: row.input.physical.waypoints.len(),
    })
}

fn group_key_for_binding(
    signature_id: &str,
    row: &TerrainEquivalenceInputBindingV1,
) -> Result<TerrainEquivalenceGroupKeyV1, String> {
    match signature_id {
        TERRAIN_EXACT_SIGNATURE_V1 => Ok(TerrainEquivalenceGroupKeyV1 {
            terrain: TerrainEquivalenceTerrainKeyV1::Exact {
                terrain_geometry_digest: row.exact_terrain_digest.clone(),
            },
            vehicle_physics_digest: row.vehicle_physics_digest.clone(),
            waypoint_count: row.waypoint_count,
        }),
        TERRAIN_RELIEF_MOTIF_SIGNATURE_V1 => Ok(TerrainEquivalenceGroupKeyV1 {
            terrain: TerrainEquivalenceTerrainKeyV1::RouteReliefMotif {
                motif: row.route_relief_motif.clone(),
            },
            vehicle_physics_digest: row.vehicle_physics_digest.clone(),
            waypoint_count: row.waypoint_count,
        }),
        _ => Err(format!(
            "unknown terrain equivalence signature {signature_id}"
        )),
    }
}

fn motif_signature_description() -> String {
    format!(
        "Non-normative route-frame positive piecewise-linear relief peaks in the physical free corridor, with fixed 0.1 normalized-location buckets ([0.0,0.1), ..., [0.9,1.0]), nearest-10 m positive-height buckets with half-up ties, canonical +0.0, and versioned derived-location boundary rule {TERRAIN_RELIEF_MOTIF_LOCATION_BOUNDARY_RULE_V1}; vehicle physics digest and waypoint count remain key dimensions. Exact query terrain remains required."
    )
}

fn route_point(input: &RouteCapabilityInputV1) -> HullPointV1 {
    HullPointV1 {
        route_angle_deg: canonical_zero(input.physical.route_angle_deg),
        route_radius_m: canonical_zero(input.physical.route_radius_m),
    }
}

fn phase_cell_prefix(phase: &ProgressIntervalContinuousPhaseV1) -> String {
    match phase {
        ProgressIntervalContinuousPhaseV1::PadDeparture => "pad_departure".to_owned(),
        ProgressIntervalContinuousPhaseV1::Acquisition => "acquisition".to_owned(),
        ProgressIntervalContinuousPhaseV1::RouteLeg { leg_index } => {
            format!("route_leg_{leg_index}")
        }
    }
}

fn required_cell_ids(waypoint_count: usize) -> Vec<String> {
    let phases = std::iter::once(ProgressIntervalContinuousPhaseV1::PadDeparture)
        .chain(std::iter::once(
            ProgressIntervalContinuousPhaseV1::Acquisition,
        ))
        .chain(
            (0..waypoint_count)
                .map(|leg_index| ProgressIntervalContinuousPhaseV1::RouteLeg { leg_index }),
        )
        .collect::<Vec<_>>();
    let mut cells = Vec::with_capacity(phases.len() * 33 + waypoint_count);
    for phase in phases {
        let prefix = phase_cell_prefix(&phase);
        for bin_index in 0..32 {
            cells.push(format!("{prefix}:bin_{bin_index}"));
        }
        cells.push(format!("{prefix}:terminal"));
    }
    for waypoint_index in 0..waypoint_count {
        cells.push(format!("handoff_{waypoint_index}"));
    }
    cells
}

fn record_cell_digest_sets(row: &LoadedDevelopmentRow) -> BTreeMap<String, BTreeSet<String>> {
    let mut cells = BTreeMap::new();
    let input_digest = row.resolved_input_digest.clone();
    for phase in &row.training.continuous_phases {
        let prefix = phase_cell_prefix(&phase.phase);
        for observation in &phase.observations {
            let bin_index = crate::progress_bin_index(observation.normalized_phase_progress)
                .expect("validated training observation has a valid progress bin");
            cells
                .entry(format!("{prefix}:bin_{bin_index}"))
                .or_insert_with(BTreeSet::new)
                .insert(input_digest.clone());
        }
        if phase.terminal.is_some() {
            cells
                .entry(format!("{prefix}:terminal"))
                .or_insert_with(BTreeSet::new)
                .insert(input_digest.clone());
        }
    }
    for handoff in &row.training.handoffs {
        cells
            .entry(format!("handoff_{}", handoff.waypoint_index))
            .or_insert_with(BTreeSet::new)
            .insert(input_digest.clone());
    }
    cells
}

fn input_binding(row: &LoadedDevelopmentRow) -> Result<TerrainEquivalenceInputBindingV1, String> {
    let binding = TerrainEquivalenceInputBindingV1 {
        row_id: row.row_id.clone(),
        corpus: row.corpus,
        input_digest: row.input.input_digest.clone(),
        resolved_input_digest: row.resolved_input_digest.clone(),
        evidence_physical_digest: row.training.evidence_physical_digest.clone(),
        exact_terrain_digest: exact_terrain_digest(&row.input)?,
        route_relief_motif: route_relief_motif(&row.input)?,
        vehicle_physics_digest: vehicle_physics_digest(&row.input)?,
        waypoint_count: row.input.physical.waypoints.len(),
        observed_required_cell_ids: record_cell_digest_sets(row).into_keys().collect(),
        route_point: route_point(&row.input),
    };
    binding.validate()?;
    Ok(binding)
}

fn group_report(
    key: TerrainEquivalenceGroupKeyV1,
    group_rows: &[&LoadedDevelopmentRow],
) -> Result<TerrainEquivalenceGroupV1, String> {
    key.validate()?;
    let key_digest = canonical_digest(&key)?;
    let mut sorted_rows = group_rows.to_vec();
    sorted_rows.sort_by(|left, right| left.row_id.cmp(&right.row_id));
    let row_ids = sorted_rows
        .iter()
        .map(|row| row.row_id.clone())
        .collect::<Vec<_>>();
    let distinct_resolved_input_digests = sorted_rows
        .iter()
        .map(|row| row.resolved_input_digest.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let exact_terrain_digests = sorted_rows
        .iter()
        .map(|row| exact_terrain_digest(&row.input))
        .collect::<Result<BTreeSet<_>, _>>()?
        .into_iter()
        .collect::<Vec<_>>();
    let points = sorted_rows
        .iter()
        .map(|row| route_point(&row.input))
        .collect::<Vec<_>>();
    let route_angle_radius_hull = convex_hull(&points, HULL_TOLERANCE)?;

    let required_cells = required_cell_ids(key.waypoint_count);
    let mut support_sets = required_cells
        .iter()
        .map(|cell| (cell.clone(), BTreeSet::new()))
        .collect::<BTreeMap<String, BTreeSet<String>>>();
    for row in &sorted_rows {
        for (cell, digests) in record_cell_digest_sets(row) {
            support_sets.entry(cell).or_default().extend(digests);
        }
    }
    let mut cell_support = Vec::with_capacity(required_cells.len());
    for cell_id in required_cells {
        let digest_set = support_sets.get(&cell_id).expect("required cell exists");
        let minimum_leave_one_out = sorted_rows
            .iter()
            .map(|row| {
                digest_set.len() - usize::from(digest_set.contains(&row.resolved_input_digest))
            })
            .min()
            .unwrap_or(0);
        cell_support.push(TerrainEquivalenceCellSupportV1 {
            cell_id,
            resolved_input_digests: digest_set.iter().cloned().collect(),
            distinct_resolved_input_digest_count: digest_set.len(),
            minimum_leave_one_out_distinct_resolved_input_digest_count: minimum_leave_one_out,
        });
    }

    let mut leave_one_out = Vec::with_capacity(sorted_rows.len());
    for row in &sorted_rows {
        let excluded = row.resolved_input_digest.as_str();
        let fit_rows = sorted_rows
            .iter()
            .filter(|candidate| candidate.resolved_input_digest != excluded)
            .collect::<Vec<_>>();
        let fit_digests = fit_rows
            .iter()
            .map(|candidate| candidate.resolved_input_digest.clone())
            .collect::<BTreeSet<_>>();
        let hull_points = fit_rows
            .iter()
            .map(|candidate| route_point(&candidate.input))
            .collect::<Vec<_>>();
        let hull = if hull_points.is_empty() {
            Vec::new()
        } else {
            convex_hull(&hull_points, HULL_TOLERANCE)?
        };
        let hull_eligible = !hull.is_empty()
            && classify_hull_point(&hull, route_point(&row.input), HULL_TOLERANCE)?
                != HullContainmentV1::Outside;
        let minimum_cell_support = cell_support
            .iter()
            .map(|cell| {
                cell.distinct_resolved_input_digest_count
                    - usize::from(
                        support_sets
                            .get(&cell.cell_id)
                            .is_some_and(|set| set.contains(excluded)),
                    )
            })
            .min()
            .unwrap_or(0);
        let same_group_support = fit_digests.len();
        // The route-angle/radius hull is a baseline-only design gate.  The
        // diagnostic archive still contributes input support, but it must not
        // veto design solely because its route point is outside that baseline
        // geometry.  Group and required-cell support remain gates for every
        // row so LOO accounting cannot silently discard diagnostic inputs.
        let baseline_hull_gate =
            row.corpus != ProgressIntervalDevelopmentCorpusV1::Baseline || hull_eligible;
        let eligible_for_design = same_group_support >= TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS
            && baseline_hull_gate
            && minimum_cell_support >= TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS;
        leave_one_out.push(TerrainEquivalenceLeaveOneOutRowV1 {
            row_id: row.row_id.clone(),
            excluded_resolved_input_digest: row.resolved_input_digest.clone(),
            route_point: route_point(&row.input),
            same_group_distinct_resolved_input_digest_support: same_group_support,
            route_angle_radius_hull_after_exclusion: hull,
            route_angle_radius_hull_eligible: hull_eligible,
            minimum_required_cell_support: minimum_cell_support,
            eligible_for_design,
        });
    }
    let minimum_leave_one_out_group_support = leave_one_out
        .iter()
        .map(|row| row.same_group_distinct_resolved_input_digest_support)
        .min()
        .unwrap_or(0);
    let minimum_leave_one_out_cell_support = leave_one_out
        .iter()
        .map(|row| row.minimum_required_cell_support)
        .min()
        .unwrap_or(0);
    let all_rows_meet_minimum_support = leave_one_out.iter().all(|row| row.eligible_for_design);
    let group = TerrainEquivalenceGroupV1 {
        key,
        key_digest,
        row_ids,
        distinct_resolved_input_digests,
        exact_terrain_digests,
        route_angle_radius_hull,
        cell_support,
        leave_one_out,
        minimum_leave_one_out_group_support,
        minimum_leave_one_out_cell_support,
        all_rows_meet_minimum_support,
    };
    group.validate()?;
    Ok(group)
}

fn signature_report(
    signature_id: &str,
    description: &str,
    rows: &[LoadedDevelopmentRow],
    key_for: impl Fn(&LoadedDevelopmentRow) -> Result<TerrainEquivalenceGroupKeyV1, String>,
) -> Result<TerrainEquivalenceSignatureReportV1, String> {
    let mut grouped = BTreeMap::<TerrainEquivalenceGroupKeyV1, Vec<&LoadedDevelopmentRow>>::new();
    for row in rows {
        grouped.entry(key_for(row)?).or_default().push(row);
    }
    let mut groups = Vec::with_capacity(grouped.len());
    for (key, group_rows) in grouped {
        groups.push(group_report(key, &group_rows)?);
    }
    groups.sort_by(|left, right| left.key.cmp(&right.key));
    let eligible_row_count = groups
        .iter()
        .flat_map(|group| group.leave_one_out.iter())
        .filter(|row| row.eligible_for_design)
        .count();
    let all_baseline_route_points_hull_eligible = groups.iter().all(|group| {
        group.leave_one_out.iter().all(|row| {
            rows.iter()
                .find(|candidate| candidate.row_id == row.row_id)
                .is_none_or(|candidate| {
                    candidate.corpus != ProgressIntervalDevelopmentCorpusV1::Baseline
                        || row.route_angle_radius_hull_eligible
                })
        })
    });
    let mut failure_reasons = Vec::new();
    if eligible_row_count != rows.len() {
        failure_reasons.push("leave_one_out_support_incomplete".to_owned());
    }
    if !all_baseline_route_points_hull_eligible {
        failure_reasons.push("baseline_route_point_outside_leave_one_out_hull".to_owned());
    }
    let minimum_leave_one_out_group_support = groups
        .iter()
        .map(|group| group.minimum_leave_one_out_group_support)
        .min()
        .unwrap_or(0);
    let minimum_leave_one_out_cell_support = groups
        .iter()
        .map(|group| group.minimum_leave_one_out_cell_support)
        .min()
        .unwrap_or(0);
    if minimum_leave_one_out_cell_support < TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS {
        failure_reasons.push("required_phase_cell_undercovered".to_owned());
    }
    let viable_for_a2_design = failure_reasons.is_empty()
        && minimum_leave_one_out_group_support >= TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS
        && minimum_leave_one_out_cell_support >= TERRAIN_EQUIVALENCE_MIN_DISTINCT_DIGESTS;
    let report = TerrainEquivalenceSignatureReportV1 {
        signature_id: signature_id.to_owned(),
        signature_description: description.to_owned(),
        group_count: groups.len(),
        groups,
        eligible_row_count,
        minimum_leave_one_out_group_support,
        minimum_leave_one_out_cell_support,
        all_baseline_route_points_hull_eligible,
        viable_for_a2_design,
        failure_reasons,
    };
    report.validate()?;
    Ok(report)
}

fn source_manifest() -> anyhow::Result<crate::SourceTransitionDevelopmentManifest> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("pd-eval crate has no repository root"))?
        .to_path_buf();
    crate::load_source_transition_development_manifest(
        &repo_root.join("fixtures/manifests/source_transition_d0a_development.json"),
    )
}

fn load_rows(
    evidence_dir: &Path,
) -> anyhow::Result<(
    Vec<LoadedDevelopmentRow>,
    crate::SourceTransitionDevelopmentManifest,
)> {
    let source_manifest = source_manifest()?;
    let mut rows = Vec::new();
    load_development_corpus(
        evidence_dir,
        "baseline",
        ProgressIntervalDevelopmentCorpusV1::Baseline,
        &mut rows,
    )?;
    load_development_corpus(
        evidence_dir,
        "diagnostic",
        ProgressIntervalDevelopmentCorpusV1::Diagnostic,
        &mut rows,
    )?;
    rows.sort_by(|left, right| left.row_id.cmp(&right.row_id));
    if rows.len() != 60 {
        bail!(
            "terrain equivalence spike requires 60 rows, found {}",
            rows.len()
        );
    }
    if rows.windows(2).any(|pair| pair[0].row_id == pair[1].row_id) {
        bail!("terrain equivalence spike has duplicate row IDs");
    }
    let baseline_count = rows
        .iter()
        .filter(|row| row.corpus == ProgressIntervalDevelopmentCorpusV1::Baseline)
        .count();
    if baseline_count != 36 {
        bail!("terrain equivalence spike requires 36 baseline rows, found {baseline_count}");
    }
    let expected_baseline_order = source_manifest
        .baseline_cases
        .iter()
        .flat_map(|case| case.resolved_case_keys())
        .collect::<Vec<_>>();
    let expected_diagnostic_order = source_manifest
        .diagnostic_cases
        .iter()
        .flat_map(|case| case.resolved_case_keys())
        .collect::<Vec<_>>();
    let expected_baseline = expected_baseline_order
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected_diagnostic = expected_diagnostic_order
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let actual_baseline = rows
        .iter()
        .filter(|row| row.corpus == ProgressIntervalDevelopmentCorpusV1::Baseline)
        .map(|row| row.row_id.clone())
        .collect::<BTreeSet<_>>();
    let actual_diagnostic = rows
        .iter()
        .filter(|row| row.corpus == ProgressIntervalDevelopmentCorpusV1::Diagnostic)
        .map(|row| row.row_id.clone())
        .collect::<BTreeSet<_>>();
    if actual_baseline != expected_baseline || actual_diagnostic != expected_diagnostic {
        bail!("terrain equivalence spike rows do not match the committed D0 input manifest");
    }
    let mut resolved = BTreeSet::new();
    for row in &rows {
        if row.resolved_input_digest.trim().is_empty()
            || !resolved.insert(row.resolved_input_digest.clone())
        {
            bail!("terrain equivalence spike requires unique resolved input digests");
        }
    }
    let resolved_by_row = rows
        .iter()
        .map(|row| (row.row_id.clone(), row.resolved_input_digest.clone()))
        .collect::<BTreeMap<_, _>>();
    let digest_for_order = |ordered_ids: &[String]| -> anyhow::Result<String> {
        let digests = ordered_ids
            .iter()
            .map(|row_id| {
                resolved_by_row
                    .get(row_id)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("resolved input is missing for row {row_id}"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(format!(
            "fnv1a64:{}",
            crate::source_transition_canonical_digest(&digests)
        ))
    };
    let baseline_digest = digest_for_order(&expected_baseline_order)?;
    let diagnostic_digest = digest_for_order(&expected_diagnostic_order)?;
    if baseline_digest != source_manifest.baseline_resolved_input_digest
        || diagnostic_digest != source_manifest.diagnostic_resolved_input_digest
    {
        bail!("terrain equivalence resolved-input content does not match committed corpus digests");
    }
    Ok((rows, source_manifest))
}

fn write_artifact(path: &Path, artifact: &TerrainEquivalenceSpikeArtifactV1) -> anyhow::Result<()> {
    let serialized = serde_json::to_string_pretty(artifact)?;
    let round_trip: TerrainEquivalenceSpikeArtifactV1 = serde_json::from_str(&serialized)?;
    round_trip.validate().map_err(anyhow::Error::msg)?;
    if round_trip != *artifact {
        bail!("terrain equivalence artifact changes on JSON round-trip");
    }
    fs::write(path, serialized).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// Run the bounded input-only terrain support spike.
pub fn run_terrain_equivalence_spike(
    evidence_dir: &Path,
    output_dir: &Path,
) -> anyhow::Result<TerrainEquivalenceSpikeArtifactV1> {
    let (rows, source_manifest) = load_rows(evidence_dir)?;
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create terrain equivalence output directory {}",
            output_dir.display()
        )
    })?;
    let input_rows = rows
        .iter()
        .map(input_binding)
        .collect::<Result<Vec<_>, _>>()
        .map_err(anyhow::Error::msg)?;
    let input_manifest_digest = canonical_digest(&input_rows).map_err(anyhow::Error::msg)?;
    let baseline_row_ids = source_manifest
        .baseline_cases
        .iter()
        .flat_map(|case| case.resolved_case_keys())
        .collect::<Vec<_>>();
    let diagnostic_row_ids = source_manifest
        .diagnostic_cases
        .iter()
        .flat_map(|case| case.resolved_case_keys())
        .collect::<Vec<_>>();
    let input_by_id = input_rows
        .iter()
        .map(|row| (row.row_id.clone(), row))
        .collect::<BTreeMap<_, _>>();
    let exact = signature_report(
        TERRAIN_EXACT_SIGNATURE_V1,
        "Exact terrain geometry digest plus vehicle physics digest and waypoint count.",
        &rows,
        exact_group_key,
    )
    .map_err(anyhow::Error::msg)?;
    let motif = signature_report(
        TERRAIN_RELIEF_MOTIF_SIGNATURE_V1,
        &motif_signature_description(),
        &rows,
        motif_group_key,
    )
    .map_err(anyhow::Error::msg)?;
    let viable_for_a2_design = motif.viable_for_a2_design;
    let decision_reasons = if viable_for_a2_design {
        vec!["route_relief_motif_has_design_support".to_owned()]
    } else {
        motif.failure_reasons.clone()
    };
    let artifact = TerrainEquivalenceSpikeArtifactV1 {
        schema_version: TERRAIN_EQUIVALENCE_SPIKE_SCHEMA_VERSION,
        source_manifest_id: source_manifest.manifest_id,
        source_d0_input_digest: source_manifest.input_digest,
        input_manifest_digest,
        baseline_row_ids: baseline_row_ids.clone(),
        diagnostic_row_ids: diagnostic_row_ids.clone(),
        baseline_resolved_input_digest: corpus_digest_for_order(&baseline_row_ids, &input_by_id)
            .map_err(anyhow::Error::msg)?,
        diagnostic_resolved_input_digest: corpus_digest_for_order(
            &diagnostic_row_ids,
            &input_by_id,
        )
        .map_err(anyhow::Error::msg)?,
        row_count: rows.len(),
        unique_resolved_input_digest_count: rows
            .iter()
            .map(|row| row.resolved_input_digest.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        exact_query_terrain_required: true,
        input_rows,
        signatures: vec![exact, motif],
        viable_for_a2_design,
        decision_reasons,
        artifact_digest: String::new(),
    }
    .seal()
    .map_err(anyhow::Error::msg)?;
    write_artifact(
        &output_dir.join("terrain_equivalence_spike.json"),
        &artifact,
    )?;
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ProgressIntervalStateObservationV1, ProgressIntervalStratumKeyV1,
        ProgressIntervalTrainingHandoffV1, ProgressIntervalTrainingPhaseV1,
        ProgressIntervalTrainingRecordV1, RouteCapabilityInputProvenanceV1,
        RouteCapabilityPadGeometryV1, RouteCapabilityPhysicalInputV1,
        RouteCapabilityPhysicalPolicyV1, RouteCapabilitySafetyProfileV1, RouteCapabilityWaypointV1,
    };
    use pd_core::{
        CorridorEnvelope, NormalizedRouteGeometry, RouteTopology, Vec2, VehicleGeometry,
        VehicleInitialState, VehicleSpec,
    };

    #[test]
    fn location_buckets_have_canonical_half_open_boundaries() {
        assert_eq!(location_bucket(0.0).unwrap(), 0);
        assert_eq!(location_bucket(0.1).unwrap(), 1);
        assert_eq!(location_bucket(0.2).unwrap(), 2);
        assert_eq!(location_bucket(next_down(0.1)).unwrap(), 0);
        assert_eq!(location_bucket(1.0).unwrap(), 9);
        assert!(location_bucket(-f64::EPSILON).is_err());
        assert!(location_bucket(1.0 + f64::EPSILON).is_err());
    }

    #[test]
    fn derived_location_snap_is_tight_and_raw_buckets_remain_strict() {
        let boundary = 0.4;
        assert_eq!(snap_derived_location(boundary - 1.0e-13).unwrap(), boundary);
        assert_eq!(snap_derived_location(boundary + 1.0e-13).unwrap(), boundary);
        assert_eq!(location_bucket(boundary - 1.0e-9).unwrap(), 3);
        assert_eq!(location_bucket(boundary + 1.0e-9).unwrap(), 4);
        assert_eq!(
            snap_derived_location(boundary - 1.0e-9).unwrap(),
            boundary - 1.0e-9
        );
        assert_eq!(
            snap_derived_location(boundary + 1.0e-9).unwrap(),
            boundary + 1.0e-9
        );
        assert_eq!(
            snap_derived_location(-0.0).unwrap().to_bits(),
            0.0_f64.to_bits()
        );
    }

    #[test]
    fn height_buckets_round_half_up_and_reject_nonpositive() {
        assert_eq!(relief_height_bucket(4.99).unwrap(), 0);
        assert_eq!(relief_height_bucket(5.0).unwrap(), 1);
        assert_eq!(relief_height_bucket(14.99).unwrap(), 1);
        assert_eq!(relief_height_bucket(15.0).unwrap(), 2);
        assert!(relief_height_bucket(0.0).is_err());
        assert!(relief_height_bucket(-1.0).is_err());
    }

    #[test]
    fn motif_and_exact_keys_are_distinct_and_round_trip() {
        let motif = TerrainEquivalenceTerrainKeyV1::RouteReliefMotif {
            motif: RouteReliefMotifV1 {
                peak_count: 1,
                peaks: vec![RouteReliefMotifPeakV1 {
                    location_bucket: 5,
                    relief_height_bucket: 12,
                }],
            },
        };
        let key = TerrainEquivalenceGroupKeyV1 {
            terrain: motif,
            vehicle_physics_digest: "vehicle".to_owned(),
            waypoint_count: 1,
        };
        let bytes = serde_json::to_vec(&key).unwrap();
        let decoded: TerrainEquivalenceGroupKeyV1 = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, key);
        assert_ne!(
            key,
            TerrainEquivalenceGroupKeyV1 {
                terrain: TerrainEquivalenceTerrainKeyV1::Exact {
                    terrain_geometry_digest: "terrain".to_owned(),
                },
                vehicle_physics_digest: "vehicle".to_owned(),
                waypoint_count: 1,
            }
        );
    }

    #[test]
    fn artifact_digest_tampering_is_rejected() {
        let mut artifact = TerrainEquivalenceSpikeArtifactV1 {
            schema_version: TERRAIN_EQUIVALENCE_SPIKE_SCHEMA_VERSION,
            source_manifest_id: "manifest-id".to_owned(),
            source_d0_input_digest: "source".to_owned(),
            input_manifest_digest: "manifest".to_owned(),
            baseline_row_ids: vec!["baseline".to_owned()],
            diagnostic_row_ids: vec!["diagnostic".to_owned()],
            baseline_resolved_input_digest: "baseline-digest".to_owned(),
            diagnostic_resolved_input_digest: "diagnostic-digest".to_owned(),
            row_count: 0,
            unique_resolved_input_digest_count: 0,
            exact_query_terrain_required: true,
            input_rows: Vec::new(),
            signatures: Vec::new(),
            viable_for_a2_design: false,
            decision_reasons: vec!["not_run".to_owned()],
            artifact_digest: String::new(),
        };
        artifact.artifact_digest = artifact.compute_digest().unwrap();
        assert!(artifact.validate().is_err());
        artifact.schema_version = 2;
        assert!(artifact.validate().is_err());
    }

    #[test]
    fn route_frame_translation_and_material_motif_changes_are_observable() {
        let base_points = [
            (0.0, 0.0),
            (0.2, 0.0),
            (0.4, 100.0),
            (0.6, 0.0),
            (0.8, 0.0),
            (1.0, 0.0),
        ];
        let base = synthetic_input(&base_points, 0.0, 1);
        let translated = synthetic_input(&base_points, 1_000.0, 1);
        assert_eq!(
            route_relief_motif(&base).unwrap(),
            route_relief_motif(&translated).unwrap()
        );

        let moved_peak = synthetic_input(
            &[
                (0.0, 0.0),
                (0.2, 0.0),
                (0.5, 100.0),
                (0.6, 0.0),
                (0.8, 0.0),
                (1.0, 0.0),
            ],
            0.0,
            1,
        );
        assert_ne!(
            route_relief_motif(&base).unwrap(),
            route_relief_motif(&moved_peak).unwrap()
        );

        let taller_peak = synthetic_input(
            &[
                (0.0, 0.0),
                (0.2, 0.0),
                (0.4, 120.0),
                (0.6, 0.0),
                (0.8, 0.0),
                (1.0, 0.0),
            ],
            0.0,
            1,
        );
        assert_ne!(
            route_relief_motif(&base).unwrap(),
            route_relief_motif(&taller_peak).unwrap()
        );

        let two_peaks = synthetic_input(
            &[
                (0.0, 0.0),
                (0.2, 0.0),
                (0.35, 100.0),
                (0.45, 0.0),
                (0.65, 100.0),
                (0.75, 0.0),
                (0.8, 0.0),
                (1.0, 0.0),
            ],
            0.0,
            1,
        );
        assert_eq!(route_relief_motif(&base).unwrap().peak_count, 1);
        assert_eq!(route_relief_motif(&two_peaks).unwrap().peak_count, 2);
    }

    #[test]
    fn malformed_and_out_of_domain_terrain_is_rejected() {
        let mut malformed = synthetic_input(
            &[(0.0, 0.0), (0.2, 0.0), (0.4, 100.0), (0.8, 0.0), (1.0, 0.0)],
            0.0,
            1,
        );
        malformed.physical.terrain = pd_core::TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(0.0, 0.0), Vec2::new(0.0, 1.0)],
        };
        malformed.input_digest = canonical_digest(&malformed.physical).unwrap();
        assert!(route_relief_motif(&malformed).is_err());

        let mut out_of_domain = synthetic_input(
            &[(0.0, 0.0), (0.2, 0.0), (0.4, 100.0), (0.8, 0.0), (1.0, 0.0)],
            0.0,
            1,
        );
        out_of_domain.physical.terrain = pd_core::TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)],
        };
        out_of_domain.input_digest = canonical_digest(&out_of_domain.physical).unwrap();
        assert!(route_relief_motif(&out_of_domain).is_err());
    }

    #[test]
    fn exact_singletons_fail_while_motif_group_has_loo_support() {
        let rows = synthetic_loaded_rows();
        let exact = signature_report(
            TERRAIN_EXACT_SIGNATURE_V1,
            "synthetic exact",
            &rows,
            exact_group_key,
        )
        .unwrap();
        let motif = signature_report(
            TERRAIN_RELIEF_MOTIF_SIGNATURE_V1,
            "synthetic motif",
            &rows,
            motif_group_key,
        )
        .unwrap();
        assert_eq!(exact.group_count, 4);
        assert!(!exact.viable_for_a2_design);
        assert_eq!(motif.group_count, 1);
        assert_eq!(motif.groups[0].minimum_leave_one_out_group_support, 3);
        assert_eq!(motif.groups[0].minimum_leave_one_out_cell_support, 3);
        assert_eq!(motif.eligible_row_count, 4);
        assert!(motif.viable_for_a2_design);
    }

    #[test]
    fn artifact_membership_and_derived_decision_tampering_is_rejected() {
        let artifact = synthetic_artifact();
        artifact.validate().unwrap();

        let mut membership = artifact.clone();
        membership.signatures[1].groups[0].cell_support[0]
            .resolved_input_digests
            .pop();
        membership.signatures[1].groups[0].cell_support[0].distinct_resolved_input_digest_count -=
            1;
        membership.artifact_digest = membership.compute_digest().unwrap();
        assert!(membership.validate().is_err());

        let mut decision = artifact.clone();
        decision.signatures[1].viable_for_a2_design = false;
        decision.artifact_digest = decision.compute_digest().unwrap();
        assert!(decision.validate().is_err());

        let mut corpus = artifact.clone();
        corpus.baseline_resolved_input_digest = "tampered".to_owned();
        corpus.artifact_digest = corpus.compute_digest().unwrap();
        assert!(corpus.validate().is_err());
    }

    #[test]
    fn complete_artifact_round_trip_has_no_outcome_or_audit_fields() {
        let artifact = synthetic_artifact();
        artifact.validate().unwrap();
        let serialized = serde_json::to_string(&artifact).unwrap();
        for forbidden in [
            "outcome",
            "mission_outcome",
            "end_reason",
            "controller_updates",
            "controller",
            "audit",
        ] {
            assert!(!serialized.contains(forbidden), "found {forbidden}");
        }
        let round_trip: TerrainEquivalenceSpikeArtifactV1 =
            serde_json::from_str(&serialized).unwrap();
        round_trip.validate().unwrap();
        assert_eq!(round_trip, artifact);
    }

    fn synthetic_input(
        points: &[(f64, f64)],
        translation: f64,
        sign: i8,
    ) -> RouteCapabilityInputV1 {
        let span = 100.0;
        let source_x = translation;
        let target_x = translation + f64::from(sign) * span;
        let mut terrain_points = points
            .iter()
            .map(|(location, height)| {
                Vec2::new(source_x + (f64::from(sign) * span * location), *height)
            })
            .collect::<Vec<_>>();
        terrain_points.sort_by(|left, right| left.x.total_cmp(&right.x));
        let physical = RouteCapabilityPhysicalInputV1 {
            gravity_mps2: 9.81,
            terrain: pd_core::TerrainDefinition::Heightfield {
                points_m: terrain_points,
            },
            source_pad: RouteCapabilityPadGeometryV1 {
                center_x_m: source_x,
                surface_y_m: 0.0,
                width_m: 10.0,
            },
            target_pad: RouteCapabilityPadGeometryV1 {
                center_x_m: target_x,
                surface_y_m: 0.0,
                width_m: 10.0,
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 2.0,
                    hull_height_m: 4.0,
                    touchdown_half_span_m: 1.0,
                    touchdown_base_offset_m: 1.0,
                },
                dry_mass_kg: 100.0,
                initial_fuel_kg: 10.0,
                max_fuel_kg: 20.0,
                max_thrust_n: 1_000.0,
                max_fuel_burn_kgps: 1.0,
                min_throttle_frac: 0.1,
                max_rotation_rate_radps: 1.0,
                safe_touchdown_normal_speed_mps: 5.0,
                safe_touchdown_tangential_speed_mps: 5.0,
                safe_touchdown_attitude_error_rad: 0.5,
                safe_touchdown_angular_rate_radps: 0.5,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(source_x, 100.0),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            policy: RouteCapabilityPhysicalPolicyV1 {
                max_waypoints: 2,
                flight_clearance_margin_m: 24.0,
                endpoint_transition_m: 12.0,
                max_extra_loft_ratio: 0.45,
                max_continuation_ratio: 0.75,
                max_handoff_speed_mps: 130.0,
                min_handoff_speed_mps: 10.0,
                min_outbound_progress_mps: 8.0,
                max_outbound_heading_error_rad: 0.35,
                max_outbound_cross_speed_mps: 20.0,
            },
            safety_profile: RouteCapabilitySafetyProfileV1 {
                source_transition_start_m: 10.0,
                source_transition_end_m: 20.0,
                target_transition_start_m: 80.0,
                target_transition_end_m: 90.0,
                horizontal_span_m: span,
                full_envelope: CorridorEnvelope::new(2.0, 2.0),
                contact_envelope: CorridorEnvelope::new(1.0, 1.0),
            },
            selected_centerline_m: [0.0, 20.0, 50.0, 80.0, 100.0]
                .into_iter()
                .map(|location| Vec2::new(source_x + (f64::from(sign) * location), 0.0))
                .collect(),
            normalized_geometry: NormalizedRouteGeometry {
                horizontal_sign: sign,
                direct_horizontal_span_m: span,
                direct_distance_m: span,
                route_angle_rad: 0.0,
                route_angle_deg: 0.0,
            },
            horizontal_sign: sign,
            waypoints: vec![RouteCapabilityWaypointV1 {
                position_m: Vec2::new(source_x + (f64::from(sign) * 50.0), 0.0),
                handoff_tangent_unit: Some(Vec2::new(f64::from(sign), 0.0)),
                capture_radius_m: 5.0,
                max_cross_track_m: 5.0,
                max_outbound_heading_error_rad: 0.35,
                min_outbound_progress_mps: 8.0,
                max_outbound_cross_speed_mps: Some(20.0),
                min_speed_mps: 10.0,
                max_speed_mps: 30.0,
                min_vertical_speed_mps: None,
                max_vertical_speed_mps: None,
            }],
            topology: RouteTopology::Waypoint,
            route_angle_deg: 0.0,
            route_radius_m: 100.0,
        };
        let input = RouteCapabilityInputV1 {
            schema_version: 1,
            provenance: RouteCapabilityInputProvenanceV1 {
                request_digest: "request".to_owned(),
                route_plan_digest: "plan".to_owned(),
                source_pad_id: "source".to_owned(),
                target_pad_id: "target".to_owned(),
                waypoint_ids: vec!["waypoint".to_owned()],
            },
            input_digest: canonical_digest(&physical).unwrap(),
            physical,
        };
        input.validate().unwrap();
        input
    }

    fn synthetic_training(input: &RouteCapabilityInputV1) -> ProgressIntervalTrainingRecordV1 {
        let observation = |progress: f64| ProgressIntervalStateObservationV1 {
            normalized_phase_progress: progress,
            progress_m: progress * 80.0,
            along_track_error_m: 0.0,
            cross_track_error_m: 0.0,
            along_track_velocity_mps: 10.0,
            cross_track_velocity_mps: 0.0,
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
            mass_kg: 100.0,
            fuel_kg: 10.0,
        };
        let observations = (0..32)
            .map(|index| observation((f64::from(index) + 0.5) / 32.0))
            .collect::<Vec<_>>();
        let terminal = observation(1.0);
        let phases = [
            ProgressIntervalContinuousPhaseV1::PadDeparture,
            ProgressIntervalContinuousPhaseV1::Acquisition,
            ProgressIntervalContinuousPhaseV1::RouteLeg { leg_index: 0 },
        ]
        .into_iter()
        .map(|phase| ProgressIntervalTrainingPhaseV1 {
            phase,
            observations: observations.clone(),
            terminal: Some(terminal.clone()),
        })
        .collect::<Vec<_>>();
        ProgressIntervalTrainingRecordV1 {
            input_digest: input.input_digest.clone(),
            evidence_physical_digest: "evidence".to_owned(),
            stratum: ProgressIntervalStratumKeyV1::from_input(input).unwrap(),
            training_point: HullPointV1 {
                route_angle_deg: 0.0,
                route_radius_m: 100.0,
            },
            horizontal_sign: input.physical.horizontal_sign,
            vehicle_dry_mass_kg: input.physical.vehicle.dry_mass_kg,
            initial_fuel_kg: input.physical.vehicle.initial_fuel_kg,
            initial: observation(0.0),
            continuous_phases: phases,
            handoffs: vec![ProgressIntervalTrainingHandoffV1 {
                waypoint_index: 0,
                terminal,
            }],
        }
    }

    fn synthetic_loaded_rows() -> Vec<LoadedDevelopmentRow> {
        (0..4)
            .map(|index| {
                let height = 100.0 + f64::from(index);
                let input = synthetic_input(
                    &[
                        (0.0, 0.0),
                        (0.2, 0.0),
                        (0.4, height),
                        (0.6, 0.0),
                        (0.8, 0.0),
                        (1.0, 0.0),
                    ],
                    0.0,
                    1,
                );
                LoadedDevelopmentRow {
                    row_id: format!("row-{index:02}"),
                    corpus: if index < 2 {
                        ProgressIntervalDevelopmentCorpusV1::Baseline
                    } else {
                        ProgressIntervalDevelopmentCorpusV1::Diagnostic
                    },
                    training: synthetic_training(&input),
                    resolved_input_digest: format!("resolved-{index:02}"),
                    physics_hz: 100,
                    manifest_path: PathBuf::from("/nonexistent/manifest.json"),
                    input,
                }
            })
            .collect()
    }

    fn synthetic_artifact() -> TerrainEquivalenceSpikeArtifactV1 {
        let rows = synthetic_loaded_rows();
        let input_rows = rows
            .iter()
            .map(input_binding)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let input_by_id = input_rows
            .iter()
            .map(|row| (row.row_id.clone(), row))
            .collect::<BTreeMap<_, _>>();
        let baseline_row_ids = vec!["row-00".to_owned(), "row-01".to_owned()];
        let diagnostic_row_ids = vec!["row-02".to_owned(), "row-03".to_owned()];
        let exact = signature_report(
            TERRAIN_EXACT_SIGNATURE_V1,
            "synthetic exact",
            &rows,
            exact_group_key,
        )
        .unwrap();
        let motif = signature_report(
            TERRAIN_RELIEF_MOTIF_SIGNATURE_V1,
            "synthetic motif",
            &rows,
            motif_group_key,
        )
        .unwrap();
        TerrainEquivalenceSpikeArtifactV1 {
            schema_version: TERRAIN_EQUIVALENCE_SPIKE_SCHEMA_VERSION,
            source_manifest_id: "manifest-id".to_owned(),
            source_d0_input_digest: "source-digest".to_owned(),
            input_manifest_digest: canonical_digest(&input_rows).unwrap(),
            baseline_row_ids: baseline_row_ids.clone(),
            diagnostic_row_ids: diagnostic_row_ids.clone(),
            baseline_resolved_input_digest: corpus_digest_for_order(
                &baseline_row_ids,
                &input_by_id,
            )
            .unwrap(),
            diagnostic_resolved_input_digest: corpus_digest_for_order(
                &diagnostic_row_ids,
                &input_by_id,
            )
            .unwrap(),
            row_count: input_rows.len(),
            unique_resolved_input_digest_count: input_rows.len(),
            exact_query_terrain_required: true,
            input_rows,
            viable_for_a2_design: motif.viable_for_a2_design,
            decision_reasons: if motif.viable_for_a2_design {
                vec!["route_relief_motif_has_design_support".to_owned()]
            } else {
                motif.failure_reasons.clone()
            },
            signatures: vec![exact, motif],
            artifact_digest: String::new(),
        }
        .seal()
        .unwrap()
    }

    fn next_down(value: f64) -> f64 {
        f64::from_bits(value.to_bits() - 1)
    }
}
