//! Descriptive input coverage. Occupied cells describe only the declared axes;
//! they do not establish complete terrain coverage or mission solvability.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{CoverageAxisV1, MorphologyLabelV1};

impl CoverageAxisV1 {
    pub fn key(self) -> &'static str {
        match self {
            Self::Distance => "distance",
            Self::SignedRise => "signed_rise",
            Self::SourceWidth => "source_width",
            Self::TargetWidth => "target_width",
            Self::ChordBarrier => "chord_barrier",
            Self::SustainedBarrierWidth => "sustained_barrier_width",
            Self::DepartureDistance => "departure_distance",
            Self::ApproachDistance => "approach_distance",
            Self::CrestCount => "crest_count",
            Self::MinCrestSpacing => "min_crest_spacing",
        }
    }
}

pub fn morphology_group_key(labels: &[MorphologyLabelV1]) -> &'static str {
    match labels {
        [] => "unclassified",
        [MorphologyLabelV1::MountainCrossing] => "mountain_crossing",
        [MorphologyLabelV1::BroadMassif] => "broad_massif",
        [MorphologyLabelV1::SuccessiveRidges] => "successive_ridges",
        [MorphologyLabelV1::ElevationTransition] => "elevation_transition",
        _ => "mixed",
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticAxisV1 {
    pub axis: CoverageAxisV1,
    pub interior_boundaries: Vec<f64>,
    pub show_underflow_overflow: bool,
    pub show_missing: bool,
}

impl DiagnosticAxisV1 {
    /// Equality enters the bin starting at that boundary. Missing is a distinct
    /// final cell, even when it has zero observations.
    pub fn slot(&self, value: Option<f64>) -> usize {
        value.map_or(self.interior_boundaries.len() + 1, |v| {
            self.interior_boundaries.partition_point(|&edge| v >= edge)
        })
    }
    pub fn slots(&self) -> usize {
        self.interior_boundaries.len() + 2
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageDesignV1 {
    pub schema_version: u32,
    pub axes: Vec<DiagnosticAxisV1>,
    pub joints: Vec<[CoverageAxisV1; 2]>,
}

impl CoverageDesignV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.axes.is_empty()
            || self.axes.len() > 10
            || self.joints.len() > 20
        {
            return Err("unsupported or unbounded coverage design".into());
        }
        for (i, a) in self.axes.iter().enumerate() {
            if self.axes[..i].iter().any(|b| b.axis == a.axis)
                || !a.show_underflow_overflow
                || !a.show_missing
                || a.interior_boundaries.is_empty()
                || a.interior_boundaries.len() > 20
                || a.interior_boundaries
                    .iter()
                    .any(|v| !v.is_finite() || v.abs() > 1e6)
                || a.interior_boundaries.windows(2).any(|v| v[0] >= v[1])
            {
                return Err("coverage axes require unique ordered bounded edges and visible overflow/missing".into());
            }
        }
        for (i, pair) in self.joints.iter().enumerate() {
            if pair[0] == pair[1]
                || pair.iter().any(|a| self.axis(*a).is_none())
                || self.joints[..i]
                    .iter()
                    .any(|p| p == pair || p == &[pair[1], pair[0]])
            {
                return Err("coverage joints require distinct declared axes and pairs".into());
            }
        }
        Ok(())
    }
    pub fn axis(&self, axis: CoverageAxisV1) -> Option<&DiagnosticAxisV1> {
        self.axes.iter().find(|a| a.axis == axis)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AxisValueV1 {
    pub axis: CoverageAxisV1,
    pub value: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarSummaryV1 {
    pub observed: usize,
    pub missing: usize,
    pub min: Option<f64>,
    pub p10: Option<f64>,
    pub p50: Option<f64>,
    pub p90: Option<f64>,
    pub max: Option<f64>,
}

pub fn summarize_scalar(
    values: impl IntoIterator<Item = Option<f64>>,
) -> Result<ScalarSummaryV1, String> {
    let values: Vec<_> = values.into_iter().collect();
    if values.iter().flatten().any(|v| !v.is_finite()) {
        return Err("coverage scalar is nonfinite".into());
    }
    let mut present: Vec<_> = values.iter().flatten().copied().collect();
    present.sort_by(f64::total_cmp);
    let percentile = |q: f64| {
        if present.is_empty() {
            return None;
        }
        let position = (present.len() - 1) as f64 * q;
        let low = position.floor() as usize;
        let high = position.ceil() as usize;
        Some(present[low] + (present[high] - present[low]) * (position - low as f64))
    };
    Ok(ScalarSummaryV1 {
        observed: present.len(),
        missing: values.len() - present.len(),
        min: present.first().copied(),
        p10: percentile(0.1),
        p50: percentile(0.5),
        p90: percentile(0.9),
        max: present.last().copied(),
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarginalCoverageV1 {
    pub axis: CoverageAxisV1,
    pub counts: Vec<usize>,
    /// Direction is conditional on these cells; it is not a navigation branch.
    pub right_counts: Vec<usize>,
    pub left_counts: Vec<usize>,
    pub summary: ScalarSummaryV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointCoverageV1 {
    pub axes: [CoverageAxisV1; 2],
    pub counts: Vec<Vec<usize>>,
    pub observed_pairs: usize,
    pub pearson_r: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DescriptorClusterV1 {
    pub signature_sha256: String,
    pub count: usize,
    pub example_indices: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DescriptorConcentrationV1 {
    pub method_version: String,
    pub unique_buckets: usize,
    pub shared_buckets: usize,
    pub rows_in_shared_buckets: usize,
    pub largest_bucket: usize,
    pub largest_groups: Vec<DescriptorClusterV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationCoverageV1 {
    pub count: usize,
    pub rightward: usize,
    pub leftward: usize,
    pub axes: Vec<MarginalCoverageV1>,
    pub joints: Vec<JointCoverageV1>,
    pub morphology_groups: BTreeMap<String, usize>,
    pub overlapping_labels: BTreeMap<String, usize>,
    pub challenge_paths: BTreeMap<String, usize>,
    /// [rightward, leftward] within each mutually exclusive display group.
    pub directions_by_group: BTreeMap<String, [usize; 2]>,
    pub scalar_summaries: BTreeMap<String, ScalarSummaryV1>,
    pub concentration: DescriptorConcentrationV1,
}

/// Summarize an explicitly selected population of already-verified audit rows.
/// No bin, correlation, label or concentration finding feeds back into admission.
pub fn summarize_coverage(
    rows: &[&crate::InputAuditRowV1],
    design: &CoverageDesignV1,
) -> Result<PopulationCoverageV1, String> {
    design.validate()?;
    if rows.len() > 20000 {
        return Err("coverage population exceeds bounded schedule".into());
    }
    for r in rows {
        if ![-1, 1].contains(&r.direction)
            || r.values
                .iter()
                .map(|v| v.axis)
                .ne(design.axes.iter().map(|a| a.axis))
            || r.values
                .iter()
                .filter_map(|v| v.value)
                .any(|v| !v.is_finite())
        {
            return Err("coverage row direction, axes or values are invalid".into());
        }
    }
    let value = |r: &crate::InputAuditRowV1, a: CoverageAxisV1| {
        r.values.iter().find(|v| v.axis == a).and_then(|v| v.value)
    };
    let mut axes = Vec::new();
    for axis in &design.axes {
        let mut counts = vec![0; axis.slots()];
        let mut right_counts = counts.clone();
        let mut left_counts = counts.clone();
        for row in rows {
            let slot = axis.slot(value(row, axis.axis));
            counts[slot] += 1;
            if row.direction == 1 {
                right_counts[slot] += 1;
            } else {
                left_counts[slot] += 1;
            }
        }
        axes.push(MarginalCoverageV1 {
            axis: axis.axis,
            counts,
            right_counts,
            left_counts,
            summary: summarize_scalar(rows.iter().map(|r| value(r, axis.axis)))?,
        });
    }
    let mut joints = Vec::new();
    for pair in &design.joints {
        let x = design.axis(pair[0]).ok_or("missing joint axis")?;
        let y = design.axis(pair[1]).ok_or("missing joint axis")?;
        let mut counts = vec![vec![0; y.slots()]; x.slots()];
        let (mut n, mut mx, mut my, mut xx, mut yy, mut xy) = (0usize, 0.0, 0.0, 0.0, 0.0, 0.0);
        for row in rows {
            let a = value(row, pair[0]);
            let b = value(row, pair[1]);
            counts[x.slot(a)][y.slot(b)] += 1;
            if let (Some(a), Some(b)) = (a, b) {
                n += 1;
                let dx = a - mx;
                let dy = b - my;
                mx += dx / n as f64;
                my += dy / n as f64;
                xx += dx * (a - mx);
                yy += dy * (b - my);
                xy += dx * (b - my);
            }
        }
        let pearson_r =
            (n > 1 && xx > 0.0 && yy > 0.0).then(|| (xy / xx.sqrt() / yy.sqrt()).clamp(-1.0, 1.0));
        joints.push(JointCoverageV1 {
            axes: *pair,
            counts,
            observed_pairs: n,
            pearson_r,
        });
    }
    let mut morphology_groups = BTreeMap::new();
    let mut overlapping_labels = BTreeMap::new();
    let mut challenge_paths = BTreeMap::from([
        ("sustained_barrier".to_owned(), 0),
        ("elevation_transition".to_owned(), 0),
        ("successive_crests".to_owned(), 0),
    ]);
    let mut directions_by_group = BTreeMap::<String, [usize; 2]>::new();
    let mut signatures = BTreeMap::<String, Vec<u64>>::new();
    let mut metric_names = BTreeSet::new();
    for row in rows {
        let group = morphology_group_key(&row.labels).to_owned();
        directions_by_group.entry(group).or_default()[usize::from(row.direction == -1)] += 1;
        for (path, qualifies) in [
            (
                "sustained_barrier",
                row.challenge.raw.sustained_barrier && row.challenge.prepared.sustained_barrier,
            ),
            (
                "elevation_transition",
                row.challenge.raw.elevation_transition
                    && row.challenge.prepared.elevation_transition,
            ),
            (
                "successive_crests",
                row.challenge.raw.successive_crests && row.challenge.prepared.successive_crests,
            ),
        ] {
            if qualifies {
                *challenge_paths
                    .get_mut(path)
                    .ok_or("missing challenge path")? += 1;
            }
        }
        *morphology_groups
            .entry(morphology_group_key(&row.labels).to_owned())
            .or_default() += 1;
        for label in &row.labels {
            let key = serde_json::to_value(label)
                .map_err(|e| e.to_string())?
                .as_str()
                .ok_or("invalid label key")?
                .to_owned();
            *overlapping_labels.entry(key).or_default() += 1;
        }
        signatures
            .entry(row.shape_signature_sha256.clone())
            .or_default()
            .push(row.index);
        metric_names.extend(row.scalar_metrics.keys().cloned());
    }
    let scalar_summaries = metric_names
        .into_iter()
        .map(|name| {
            let stats = summarize_scalar(
                rows.iter()
                    .map(|r| r.scalar_metrics.get(&name).copied().flatten()),
            )?;
            Ok((name, stats))
        })
        .collect::<Result<_, String>>()?;
    let shared_buckets = signatures.values().filter(|v| v.len() > 1).count();
    let rows_in_shared_buckets = signatures
        .values()
        .filter(|v| v.len() > 1)
        .map(Vec::len)
        .sum();
    let largest_bucket = signatures.values().map(Vec::len).max().unwrap_or(0);
    let unique_buckets = signatures.len();
    let mut groups: Vec<_> = signatures
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .collect();
    groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    let largest_groups = groups
        .into_iter()
        .take(20)
        .map(|(signature_sha256, mut indices)| {
            indices.sort_unstable();
            let count = indices.len();
            indices.truncate(12);
            DescriptorClusterV1 {
                signature_sha256,
                count,
                example_indices: indices,
            }
        })
        .collect();
    Ok(PopulationCoverageV1 {
        count: rows.len(),
        rightward: rows.iter().filter(|r| r.direction == 1).count(),
        leftward: rows.iter().filter(|r| r.direction == -1).count(),
        axes,
        joints,
        morphology_groups,
        overlapping_labels,
        challenge_paths,
        directions_by_group,
        scalar_summaries,
        concentration: DescriptorConcentrationV1 {
            method_version: "pd-missions/shape-bucket-v1".into(),
            unique_buckets,
            shared_buckets,
            rows_in_shared_buckets,
            largest_bucket,
            largest_groups,
        },
    })
}
