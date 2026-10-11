use pd_core::{LandingPadSpec, TerrainDefinition, Vec2};

use crate::GeometryIntervalV1;

/// Replace each shelf and its bounded linear transitions. Exact polyline
/// geometry outside the declared edits is retained, including boundary heights.
/// A boundary within 1e-6 m of a raw vertex snaps to that vertex and is recorded.
pub fn prepare_pads_v2(
    raw: &TerrainDefinition,
    pads: &[LandingPadSpec],
    transition_m: f64,
) -> Result<(TerrainDefinition, Vec<GeometryIntervalV1>), String> {
    raw.validate()?;
    if pads.is_empty()
        || pads.len() > 2
        || !transition_m.is_finite()
        || !(1.0..=128.0).contains(&transition_m)
        || raw.points().len() > 100001
    {
        return Err("unbounded V2 pad preparation".into());
    }
    let snap = |x: f64| {
        let i = raw.points().partition_point(|p| p.x < x);
        [i.saturating_sub(1), i.min(raw.points().len() - 1)]
            .into_iter()
            .map(|j| raw.points()[j].x)
            .find(|p| (*p - x).abs() < 1e-6)
            .unwrap_or(x)
    };
    let mut shelves = Vec::new();
    for pad in pads {
        pad.validate()?;
        if pad.width_m > 256.0 {
            return Err("V2 pad is outside bounded local preparation".into());
        }
        let start = pad.center_x_m - pad.half_width_m();
        let end = pad.center_x_m + pad.half_width_m();
        let edit = GeometryIntervalV1 {
            start_m: snap(start - transition_m),
            end_m: snap(end + transition_m),
        };
        if edit.start_m < raw.points()[0].x || edit.end_m > raw.points().last().unwrap().x {
            return Err("V2 pad preparation exceeds raw terrain domain".into());
        }
        shelves.push((edit, start, end, pad.surface_y_m));
    }
    shelves.sort_by(|a, b| a.0.start_m.total_cmp(&b.0.start_m));
    if shelves.windows(2).any(|v| v[0].0.end_m >= v[1].0.start_m) {
        return Err("V2 local pad edit intervals overlap".into());
    }
    let mut points: Vec<_> = raw
        .points()
        .iter()
        .copied()
        .filter(|p| {
            !shelves
                .iter()
                .any(|s| p.x >= s.0.start_m && p.x <= s.0.end_m)
        })
        .collect();
    for &(edit, start, end, y) in &shelves {
        points.extend([
            Vec2::new(edit.start_m, raw.sample_height(edit.start_m)),
            Vec2::new(start, y),
            Vec2::new(end, y),
            Vec2::new(edit.end_m, raw.sample_height(edit.end_m)),
        ]);
    }
    points.sort_by(|a, b| a.x.total_cmp(&b.x));
    let prepared = TerrainDefinition::Heightfield { points_m: points };
    prepared.validate()?;
    Ok((prepared, shelves.into_iter().map(|s| s.0).collect()))
}
