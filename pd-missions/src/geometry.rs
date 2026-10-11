//! Neutral resolved-polyline descriptors. Positions run from source to target;
//! neither controller behavior nor a display taxonomy participates in measurement.
use pd_core::{EvaluationGoal, LandingPadSpec, ScenarioSpec, TerrainDefinition, Vec2};
use serde::{Deserialize, Serialize};

use crate::{physical_identity, sha256};

mod crests;
use crests::significant_crests;

pub const GEOMETRY_VERSION: &str = "pd-missions/transfer-geometry-v1";
const EPS: f64 = 1e-8;
const MAX_POINTS: usize = 100_001;
const MAX_COORDINATE: f64 = 10_000_000.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometrySpecV1 {
    pub schema_version: u32,
    pub metric_version: String,
    pub barrier_heights_m: Vec<f64>,
    pub crest_min_prominence_m: f64,
    pub crest_min_half_prominence_width_m: f64,
    pub crest_min_separation_m: f64,
    pub endpoint_radius_m: f64,
}

impl Default for GeometrySpecV1 {
    fn default() -> Self {
        Self {
            schema_version: 1,
            metric_version: GEOMETRY_VERSION.into(),
            barrier_heights_m: vec![50.0, 100.0, 200.0],
            crest_min_prominence_m: 50.0,
            crest_min_half_prominence_width_m: 8.0,
            crest_min_separation_m: 20.0,
            endpoint_radius_m: 200.0,
        }
    }
}

impl GeometrySpecV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.metric_version != GEOMETRY_VERSION
            || self.barrier_heights_m.is_empty()
            || self.barrier_heights_m.len() > 16
            || self.barrier_heights_m.iter().any(|x| !bounded_positive(*x))
            || self.barrier_heights_m.windows(2).any(|v| v[0] >= v[1])
            || ![
                self.crest_min_prominence_m,
                self.crest_min_half_prominence_width_m,
                self.crest_min_separation_m,
                self.endpoint_radius_m,
            ]
            .into_iter()
            .all(bounded_positive)
        {
            return Err("unsupported or unbounded geometry specification".into());
        }
        Ok(())
    }

    pub fn identity(&self) -> Result<String, String> {
        self.validate()?;
        Ok(sha256(
            &serde_json::to_vec(self).map_err(|e| e.to_string())?,
        ))
    }
}

fn bounded_positive(value: f64) -> bool {
    value.is_finite() && (1e-6..=100_000.0).contains(&value)
}

/// Physical x for pad edits; source-relative distance for barrier/crest support.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryIntervalV1 {
    pub start_m: f64,
    pub end_m: f64,
}

impl GeometryIntervalV1 {
    pub fn width_m(self) -> f64 {
        self.end_m - self.start_m
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarrierGeometryV1 {
    pub height_above_chord_m: f64,
    pub intervals: Vec<GeometryIntervalV1>,
    pub longest_width_m: f64,
    pub total_width_m: f64,
    pub departure_distance_m: Option<f64>,
    pub approach_distance_m: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlopeGeometryV1 {
    pub measured_length_m: f64,
    pub p50_abs: f64,
    pub p95_abs: f64,
    pub max_abs: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrestGeometryV1 {
    pub position_m: f64,
    pub position_fraction: f64,
    pub height_above_chord_m: f64,
    pub prominence_m: f64,
    pub half_prominence_support: GeometryIntervalV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeighborhoodGeometryV1 {
    pub measured_length_m: f64,
    pub max_rise_above_pad_m: f64,
    pub max_fall_below_pad_m: f64,
    pub slopes: Option<SlopeGeometryV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferGeometryV1 {
    pub distance_m: f64,
    pub source_width_m: f64,
    pub target_width_m: f64,
    pub signed_pad_rise_m: f64,
    pub relief_m: f64,
    pub max_chord_barrier_m: f64,
    pub barriers: Vec<BarrierGeometryV1>,
    pub slopes: SlopeGeometryV1,
    pub slopes_outside_pad_edits: Option<SlopeGeometryV1>,
    pub crests: Vec<CrestGeometryV1>,
    pub min_crest_spacing_m: Option<f64>,
    pub max_crest_spacing_m: Option<f64>,
    pub source_inward: NeighborhoodGeometryV1,
    pub source_outward: NeighborhoodGeometryV1,
    pub target_inward: NeighborhoodGeometryV1,
    pub target_outward: NeighborhoodGeometryV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionGeometryReceiptV1 {
    pub schema_version: u32,
    pub physical_sha256: String,
    pub spec: GeometrySpecV1,
    pub spec_sha256: String,
    pub pad_edit_intervals_x_m: Vec<GeometryIntervalV1>,
    pub geometry: TransferGeometryV1,
}

impl MissionGeometryReceiptV1 {
    /// Recompute a saved projection from the bound physical input and metric
    /// definition. This checks geometry, not authoritative flight replay.
    pub fn verify(&self, scenario: &ScenarioSpec) -> Result<(), String> {
        let measured =
            measure_mission_geometry(scenario, &self.spec, &self.pad_edit_intervals_x_m)?;
        if self != &measured {
            return Err(
                "geometry receipt differs from its physical input or metric definition".into(),
            );
        }
        Ok(())
    }
}

/// Measure a complete source-pad mission and bind descriptors to all physical
/// inputs, including vehicle/resources/clocks. No generation or flight occurs.
pub fn measure_mission_geometry(
    scenario: &ScenarioSpec,
    spec: &GeometrySpecV1,
    pad_edits: &[GeometryIntervalV1],
) -> Result<MissionGeometryReceiptV1, String> {
    scenario.validate()?;
    let EvaluationGoal::LandingOnPad { target_pad_id } = &scenario.mission.goal else {
        return Err("geometry requires a target-pad mission".into());
    };
    let pads = &scenario.world.landing_pads;
    if pads.len() != 2 || pads.iter().filter(|p| &p.id == target_pad_id).count() != 1 {
        return Err("geometry requires distinct source and target pads".into());
    }
    let target = pads.iter().find(|p| &p.id == target_pad_id).unwrap();
    let source = pads.iter().find(|p| &p.id != target_pad_id).unwrap();
    if (scenario.initial_state.position_m.x - source.center_x_m).abs() > EPS {
        return Err("geometry requires a source-pad start".into());
    }
    for pad in pads {
        let height = scenario.world.terrain.sample_height(pad.center_x_m);
        if (height - pad.surface_y_m).abs() > EPS {
            return Err("prepared terrain disagrees with pad elevation".into());
        }
    }
    Ok(MissionGeometryReceiptV1 {
        schema_version: 1,
        physical_sha256: physical_identity(scenario).map_err(|e| e.to_string())?,
        spec: spec.clone(),
        spec_sha256: spec.identity()?,
        pad_edit_intervals_x_m: pad_edits.to_vec(),
        geometry: measure_transfer_geometry(
            &scenario.world.terrain,
            source,
            target,
            spec,
            pad_edits,
        )?,
    })
}

/// Measure raw or prepared terrain. Chord anchors use actual terrain at pad
/// centers, allowing raw geometry to be compared without inventing flat pads.
pub fn measure_transfer_geometry(
    terrain: &TerrainDefinition,
    source: &LandingPadSpec,
    target: &LandingPadSpec,
    spec: &GeometrySpecV1,
    pad_edits: &[GeometryIntervalV1],
) -> Result<TransferGeometryV1, String> {
    spec.validate()?;
    validate_inputs(terrain, source, target, pad_edits)?;
    let direction = (target.center_x_m - source.center_x_m).signum();
    let distance = (target.center_x_m - source.center_x_m).abs();
    let all = route_points(terrain, source.center_x_m, direction);
    let points = clip(&all, 0.0, distance);
    let source_y = points[0].y;
    let target_y = points.last().unwrap().y;
    let rise = target_y - source_y;
    let relative: Vec<_> = points
        .iter()
        .map(|p| Vec2::new(p.x, p.y - source_y - rise * (p.x / distance)))
        .collect();
    let mut edits: Vec<_> = pad_edits
        .iter()
        .map(|e| {
            let a = (e.start_m - source.center_x_m) * direction;
            let b = (e.end_m - source.center_x_m) * direction;
            GeometryIntervalV1 {
                start_m: a.min(b),
                end_m: a.max(b),
            }
        })
        .collect();
    edits.sort_by(|a, b| a.start_m.total_cmp(&b.start_m));
    let crests = significant_crests(&relative, spec, &edits, distance);
    let gaps: Vec<_> = crests
        .windows(2)
        .map(|v| v[1].position_m - v[0].position_m)
        .collect();
    let barrier = relative.iter().map(|p| p.y).fold(0.0, f64::max);
    let min_y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    Ok(TransferGeometryV1 {
        distance_m: distance,
        source_width_m: source.width_m,
        target_width_m: target.width_m,
        signed_pad_rise_m: rise,
        relief_m: max_y - min_y,
        max_chord_barrier_m: barrier,
        barriers: spec
            .barrier_heights_m
            .iter()
            .map(|height| {
                let intervals = above_intervals(&relative, *height);
                BarrierGeometryV1 {
                    height_above_chord_m: *height,
                    longest_width_m: intervals.iter().map(|e| e.width_m()).fold(0.0, f64::max),
                    total_width_m: intervals.iter().map(|e| e.width_m()).sum(),
                    departure_distance_m: intervals.first().map(|e| e.start_m),
                    approach_distance_m: intervals.last().map(|e| distance - e.end_m),
                    intervals,
                }
            })
            .collect(),
        slopes: slope_summary(&points, &[]).unwrap(),
        slopes_outside_pad_edits: slope_summary(&points, &edits),
        crests,
        min_crest_spacing_m: gaps.iter().copied().reduce(f64::min),
        max_crest_spacing_m: gaps.iter().copied().reduce(f64::max),
        source_inward: neighborhood(&all, 0.0, source_y, 0.0, spec.endpoint_radius_m),
        source_outward: neighborhood(&all, 0.0, source_y, -spec.endpoint_radius_m, 0.0),
        target_inward: neighborhood(&all, distance, target_y, -spec.endpoint_radius_m, 0.0),
        target_outward: neighborhood(&all, distance, target_y, 0.0, spec.endpoint_radius_m),
    })
}

fn validate_inputs(
    terrain: &TerrainDefinition,
    source: &LandingPadSpec,
    target: &LandingPadSpec,
    edits: &[GeometryIntervalV1],
) -> Result<(), String> {
    terrain.validate()?;
    source.validate()?;
    target.validate()?;
    let points = terrain.points();
    let bounded = |x: f64| x.is_finite() && x.abs() <= MAX_COORDINATE;
    if points.len() > MAX_POINTS
        || points.iter().any(|p| !bounded(p.x) || !bounded(p.y))
        || points.windows(2).any(|v| v[1].x - v[0].x < 1e-6)
        || source.id == target.id
        || (target.center_x_m - source.center_x_m).abs() < 1e-6
        || [source, target].iter().any(|pad| {
            !bounded(pad.center_x_m)
                || !bounded(pad.surface_y_m)
                || pad.width_m > MAX_COORDINATE
                || pad.center_x_m < points[0].x
                || pad.center_x_m > points.last().unwrap().x
        })
        || edits.len() > 16
        || edits.iter().any(|e| {
            !bounded(e.start_m)
                || !bounded(e.end_m)
                || e.start_m >= e.end_m
                || e.start_m < points[0].x
                || e.end_m > points.last().unwrap().x
        })
        || edits.windows(2).any(|v| v[0].end_m > v[1].start_m)
    {
        return Err("geometry exceeds domain, ordering or bounded-work limits".into());
    }
    Ok(())
}

fn route_points(terrain: &TerrainDefinition, source_x: f64, direction: f64) -> Vec<Vec2> {
    let mut points: Vec<_> = terrain
        .points()
        .iter()
        .map(|p| Vec2::new((p.x - source_x) * direction, p.y))
        .collect();
    if direction < 0.0 {
        points.reverse();
    }
    points
}

fn sample(points: &[Vec2], x: f64) -> f64 {
    let i = points
        .partition_point(|p| p.x <= x)
        .saturating_sub(1)
        .min(points.len() - 2);
    let a = points[i];
    let b = points[i + 1];
    a.y + (b.y - a.y) * ((x - a.x) / (b.x - a.x))
}

fn clip(points: &[Vec2], lo: f64, hi: f64) -> Vec<Vec2> {
    let mut result = vec![Vec2::new(lo, sample(points, lo))];
    result.extend(points.iter().copied().filter(|p| p.x > lo && p.x < hi));
    result.push(Vec2::new(hi, sample(points, hi)));
    result
}

fn crossing(a: Vec2, b: Vec2, height: f64) -> f64 {
    a.x + (b.x - a.x) * ((height - a.y) / (b.y - a.y))
}

fn above_intervals(points: &[Vec2], height: f64) -> Vec<GeometryIntervalV1> {
    let mut intervals: Vec<GeometryIntervalV1> = Vec::new();
    for v in points.windows(2) {
        let (a, b) = (v[0], v[1]);
        if a.y < height && b.y < height {
            continue;
        }
        let start = if a.y < height {
            crossing(a, b, height)
        } else {
            a.x
        };
        let end = if b.y < height {
            crossing(a, b, height)
        } else {
            b.x
        };
        if let Some(previous) = intervals
            .last_mut()
            .filter(|e| (e.end_m - start).abs() <= EPS)
        {
            previous.end_m = end;
        } else {
            intervals.push(GeometryIntervalV1 {
                start_m: start,
                end_m: end,
            });
        }
    }
    intervals.retain(|e| e.width_m() > EPS);
    intervals
}

fn slope_summary(points: &[Vec2], edits: &[GeometryIntervalV1]) -> Option<SlopeGeometryV1> {
    let lo = points[0].x;
    let hi = points.last().unwrap().x;
    let mut xs: Vec<_> = points.iter().map(|p| p.x).collect();
    for edit in edits {
        xs.extend(
            [edit.start_m, edit.end_m]
                .into_iter()
                .filter(|x| *x > lo && *x < hi),
        );
    }
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    let mut slopes: Vec<_> = xs
        .windows(2)
        .filter_map(|v| {
            let mid = v[0] + (v[1] - v[0]) * 0.5;
            if edits.iter().any(|e| mid >= e.start_m && mid <= e.end_m) {
                return None;
            }
            Some((
                (sample(points, v[1]) - sample(points, v[0])).abs() / (v[1] - v[0]),
                v[1] - v[0],
            ))
        })
        .collect();
    if slopes.is_empty() {
        return None;
    }
    slopes.sort_by(|a, b| a.0.total_cmp(&b.0));
    let length: f64 = slopes.iter().map(|s| s.1).sum();
    let quantile = |fraction: f64| {
        let mut accumulated = 0.0;
        for &(slope, weight) in &slopes {
            accumulated += weight;
            if accumulated + EPS >= length * fraction {
                return slope;
            }
        }
        slopes.last().unwrap().0
    };
    Some(SlopeGeometryV1 {
        measured_length_m: length,
        p50_abs: quantile(0.5),
        p95_abs: quantile(0.95),
        max_abs: slopes.last().unwrap().0,
    })
}

fn neighborhood(
    points: &[Vec2],
    anchor: f64,
    height: f64,
    a: f64,
    b: f64,
) -> NeighborhoodGeometryV1 {
    let lo = (anchor + a).max(points[0].x);
    let hi = (anchor + b).min(points.last().unwrap().x);
    if hi <= lo {
        return NeighborhoodGeometryV1 {
            measured_length_m: 0.0,
            max_rise_above_pad_m: 0.0,
            max_fall_below_pad_m: 0.0,
            slopes: None,
        };
    }
    let local = clip(points, lo, hi);
    NeighborhoodGeometryV1 {
        measured_length_m: hi - lo,
        max_rise_above_pad_m: local.iter().map(|p| p.y - height).fold(0.0, f64::max),
        max_fall_below_pad_m: local.iter().map(|p| height - p.y).fold(0.0, f64::max),
        slopes: slope_summary(&local, &[]),
    }
}
