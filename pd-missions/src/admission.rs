//! Neutral geometric admission. Thresholds are declared proxies for demand,
//! never a proof of reachability or a controller's ability to solve a mission.
use serde::{Deserialize, Serialize};

use crate::{GeometrySpecV1, TransferGeometryV1, sha256};

pub const CHALLENGE_VERSION: &str = "pd-missions/challenge-v1";
pub const MORPHOLOGY_VERSION: &str = "pd-missions/morphology-v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeRuleV1 {
    pub schema_version: u32,
    pub rule_version: String,
    pub barrier_height_m: f64,
    pub sustained_width_m: f64,
    pub absolute_pad_rise_m: f64,
    pub crest_height_m: f64,
    pub crest_prominence_m: f64,
    pub crest_width_m: f64,
    pub crest_separation_m: f64,
    pub crest_count: usize,
}

impl Default for ChallengeRuleV1 {
    fn default() -> Self {
        Self {
            schema_version: 1,
            rule_version: CHALLENGE_VERSION.into(),
            barrier_height_m: 100.0,
            sustained_width_m: 80.0,
            absolute_pad_rise_m: 300.0,
            crest_height_m: 50.0,
            crest_prominence_m: 80.0,
            crest_width_m: 40.0,
            crest_separation_m: 80.0,
            crest_count: 2,
        }
    }
}

impl ChallengeRuleV1 {
    pub fn validate(&self, spec: &GeometrySpecV1) -> Result<(), String> {
        spec.validate()?;
        if self.schema_version != 1
            || self.rule_version != CHALLENGE_VERSION
            || !spec.barrier_heights_m.contains(&self.barrier_height_m)
            || ![
                (self.barrier_height_m, 50.0),
                (self.sustained_width_m, 40.0),
                (self.absolute_pad_rise_m, 150.0),
                (self.crest_height_m, 25.0),
                (self.crest_prominence_m, 50.0),
                (self.crest_width_m, 16.0),
                (self.crest_separation_m, 40.0),
            ]
            .into_iter()
            .all(|(v, minimum)| v.is_finite() && (minimum..=100000.0).contains(&v))
            || !(2..=32).contains(&self.crest_count)
            || spec.crest_min_prominence_m > self.crest_prominence_m
            || spec.crest_min_half_prominence_width_m > self.crest_width_m
            || spec.crest_min_separation_m > self.crest_separation_m
        {
            return Err(
                "challenge rule is unsupported or not observable by the metric spec".into(),
            );
        }
        Ok(())
    }

    pub fn identity(&self, spec: &GeometrySpecV1) -> Result<String, String> {
        self.validate(spec)?;
        Ok(sha256(
            &serde_json::to_vec(self).map_err(|e| e.to_string())?,
        ))
    }

    pub fn measure(&self, geometry: &TransferGeometryV1) -> ChallengeMeasurementV1 {
        // Filtered crests are sorted by source-relative position. Greedy earliest
        // spacing gives the maximum cardinality for a fixed minimum separation.
        let mut last = None;
        let mut crest_count = 0;
        for crest in &geometry.crests {
            if crest.height_above_chord_m >= self.crest_height_m
                && crest.prominence_m >= self.crest_prominence_m
                && crest.half_prominence_support.width_m() >= self.crest_width_m
                && last.is_none_or(|x| crest.position_m - x >= self.crest_separation_m)
            {
                last = Some(crest.position_m);
                crest_count += 1;
            }
        }
        ChallengeMeasurementV1 {
            sustained_barrier: barrier(geometry, self.barrier_height_m)
                .is_some_and(|b| b.longest_width_m >= self.sustained_width_m),
            elevation_transition: geometry.signed_pad_rise_m.abs() >= self.absolute_pad_rise_m,
            successive_crests: crest_count >= self.crest_count,
            qualifying_crest_count: crest_count,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeMeasurementV1 {
    pub sustained_barrier: bool,
    pub elevation_transition: bool,
    pub successive_crests: bool,
    pub qualifying_crest_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeReceiptV1 {
    pub raw: ChallengeMeasurementV1,
    pub prepared: ChallengeMeasurementV1,
    pub qualifies: bool,
}

impl ChallengeReceiptV1 {
    pub fn measure(
        rule: &ChallengeRuleV1,
        raw: &TransferGeometryV1,
        prepared: &TransferGeometryV1,
    ) -> Self {
        let raw = rule.measure(raw);
        let prepared = rule.measure(prepared);
        let qualifies = raw.sustained_barrier && prepared.sustained_barrier
            || raw.elevation_transition && prepared.elevation_transition
            || raw.successive_crests && prepared.successive_crests;
        Self {
            raw,
            prepared,
            qualifies,
        }
    }
}

pub(crate) fn barrier(g: &TransferGeometryV1, height: f64) -> Option<&crate::BarrierGeometryV1> {
    g.barriers.iter().find(|b| b.height_above_chord_m == height)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MorphologyLabelV1 {
    MountainCrossing,
    BroadMassif,
    SuccessiveRidges,
    ElevationTransition,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MorphologySpecV1 {
    pub schema_version: u32,
    pub taxonomy_version: String,
    pub mountain_height_m: f64,
    pub massif_barrier_height_m: f64,
    pub massif_width_m: f64,
    pub ridge_min_prominence_m: f64,
    pub ridge_count: usize,
    pub absolute_pad_rise_m: f64,
}

impl Default for MorphologySpecV1 {
    fn default() -> Self {
        Self {
            schema_version: 1,
            taxonomy_version: MORPHOLOGY_VERSION.into(),
            mountain_height_m: 200.0,
            massif_barrier_height_m: 100.0,
            massif_width_m: 300.0,
            ridge_min_prominence_m: 80.0,
            ridge_count: 3,
            absolute_pad_rise_m: 400.0,
        }
    }
}

impl MorphologySpecV1 {
    pub fn validate(&self, spec: &GeometrySpecV1) -> Result<(), String> {
        spec.validate()?;
        if self.schema_version != 1
            || self.taxonomy_version != MORPHOLOGY_VERSION
            || !spec
                .barrier_heights_m
                .contains(&self.massif_barrier_height_m)
            || ![
                self.mountain_height_m,
                self.massif_width_m,
                self.ridge_min_prominence_m,
                self.absolute_pad_rise_m,
            ]
            .iter()
            .all(|v| v.is_finite() && (1.0..=100000.0).contains(v))
            || self.ridge_min_prominence_m < spec.crest_min_prominence_m
            || !(2..=32).contains(&self.ridge_count)
        {
            return Err("unsupported morphology projection".into());
        }
        Ok(())
    }

    /// All matching labels, in stable order; an empty result is retained.
    pub fn labels(&self, geometry: &TransferGeometryV1) -> Vec<MorphologyLabelV1> {
        let mut labels = Vec::new();
        if geometry.max_chord_barrier_m >= self.mountain_height_m {
            labels.push(MorphologyLabelV1::MountainCrossing);
        }
        if barrier(geometry, self.massif_barrier_height_m)
            .is_some_and(|b| b.longest_width_m >= self.massif_width_m)
        {
            labels.push(MorphologyLabelV1::BroadMassif);
        }
        if geometry
            .crests
            .iter()
            .filter(|c| {
                c.height_above_chord_m > 0.0 && c.prominence_m >= self.ridge_min_prominence_m
            })
            .count()
            >= self.ridge_count
        {
            labels.push(MorphologyLabelV1::SuccessiveRidges);
        }
        if geometry.signed_pad_rise_m.abs() >= self.absolute_pad_rise_m {
            labels.push(MorphologyLabelV1::ElevationTransition);
        }
        labels
    }
}
