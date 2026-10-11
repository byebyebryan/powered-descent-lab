use std::collections::BTreeMap;

use pd_core::Vec2;

use super::{CrestGeometryV1, EPS, GeometryIntervalV1, GeometrySpecV1, crossing};

/// Range extrema and nearest-threshold queries keep crest work O(n log n),
/// including equal-height peaks/long plateaus and tolerance-adjacent heights.
struct ExtremaTree {
    size: usize,
    values: Vec<f64>,
    maxima: Vec<f64>,
}

impl ExtremaTree {
    fn new(points: &[Vec2]) -> Self {
        let size = points.len().next_power_of_two();
        let mut values = vec![f64::INFINITY; 2 * size];
        let mut maxima = vec![f64::NEG_INFINITY; 2 * size];
        for (i, p) in points.iter().enumerate() {
            values[size + i] = p.y;
            maxima[size + i] = p.y;
        }
        for i in (1..size).rev() {
            values[i] = values[2 * i].min(values[2 * i + 1]);
            maxima[i] = maxima[2 * i].max(maxima[2 * i + 1]);
        }
        Self {
            size,
            values,
            maxima,
        }
    }

    fn minimum(&self, a: usize, b: usize) -> f64 {
        let (mut lo, mut hi) = (a + self.size, b + self.size + 1);
        let mut value = f64::INFINITY;
        while lo < hi {
            if lo % 2 == 1 {
                value = value.min(self.values[lo]);
                lo += 1;
            }
            if hi % 2 == 1 {
                hi -= 1;
                value = value.min(self.values[hi]);
            }
            lo /= 2;
            hi /= 2;
        }
        value
    }

    fn below(&self, range: std::ops::Range<usize>, height: f64, last: bool) -> Option<usize> {
        self.find_below(1, 0..self.size, &range, height, last)
    }

    fn above(&self, range: std::ops::Range<usize>, height: f64, last: bool) -> Option<usize> {
        self.find_above(1, 0..self.size, &range, height, last)
    }

    fn find_above(
        &self,
        node: usize,
        span: std::ops::Range<usize>,
        query: &std::ops::Range<usize>,
        height: f64,
        last: bool,
    ) -> Option<usize> {
        if span.end <= query.start || span.start >= query.end || self.maxima[node] <= height + EPS {
            return None;
        }
        if span.end - span.start == 1 {
            return Some(span.start);
        }
        let mid = (span.start + span.end) / 2;
        let left = (node * 2, span.start..mid);
        let right = (node * 2 + 1, mid..span.end);
        let (first, second) = if last { (right, left) } else { (left, right) };
        self.find_above(first.0, first.1, query, height, last)
            .or_else(|| self.find_above(second.0, second.1, query, height, last))
    }

    fn find_below(
        &self,
        node: usize,
        span: std::ops::Range<usize>,
        query: &std::ops::Range<usize>,
        height: f64,
        last: bool,
    ) -> Option<usize> {
        if span.end <= query.start || span.start >= query.end || self.values[node] >= height {
            return None;
        }
        if span.end - span.start == 1 {
            return Some(span.start);
        }
        let mid = (span.start + span.end) / 2;
        let left = (node * 2, span.start..mid);
        let right = (node * 2 + 1, mid..span.end);
        let (first, second) = if last { (right, left) } else { (left, right) };
        self.find_below(first.0, first.1, query, height, last)
            .or_else(|| self.find_below(second.0, second.1, query, height, last))
    }
}

pub(super) fn significant_crests(
    points: &[Vec2],
    spec: &GeometrySpecV1,
    edits: &[GeometryIntervalV1],
    distance: f64,
) -> Vec<CrestGeometryV1> {
    let minima = ExtremaTree::new(points);
    let mut candidates = Vec::new();
    let mut i = 1;
    while i + 1 < points.len() {
        let start = i;
        while i + 1 < points.len() && (points[i + 1].y - points[start].y).abs() <= EPS {
            i += 1;
        }
        let end = i;
        i += 1;
        if end + 1 == points.len()
            || points[start].y <= points[start - 1].y + EPS
            || points[end].y <= points[end + 1].y + EPS
        {
            continue;
        }
        let height = points[start].y;
        let left = minima.above(0..start, height, true).unwrap_or(0);
        let right = minima
            .above(end + 1..points.len(), height, false)
            .unwrap_or(points.len() - 1);
        let base = minima.minimum(left, start).max(minima.minimum(end, right));
        let prominence = height - base;
        if prominence + EPS < spec.crest_min_prominence_m {
            continue;
        }
        let level = height - prominence * 0.5;
        let a = minima.below(0..start, level, true).unwrap();
        let b = minima.below(end + 1..points.len(), level, false).unwrap();
        let support = GeometryIntervalV1 {
            start_m: crossing(points[a], points[a + 1], level),
            end_m: crossing(points[b - 1], points[b], level),
        };
        if support.width_m() + EPS < spec.crest_min_half_prominence_width_m
            || edits
                .iter()
                .any(|e| support.start_m < e.end_m && support.end_m > e.start_m)
        {
            continue;
        }
        let position = points[start].x + (points[end].x - points[start].x) * 0.5;
        candidates.push(CrestGeometryV1 {
            position_m: position,
            position_fraction: position / distance,
            height_above_chord_m: height,
            prominence_m: prominence,
            half_prominence_support: support,
        });
    }
    candidates.sort_by(|a, b| {
        b.prominence_m
            .total_cmp(&a.prominence_m)
            .then_with(|| b.height_above_chord_m.total_cmp(&a.height_above_chord_m))
            .then_with(|| a.position_m.total_cmp(&b.position_m))
    });
    let mut occupied = BTreeMap::<i64, Vec<f64>>::new();
    let mut selected = Vec::new();
    for crest in candidates {
        let bucket = (crest.position_m / spec.crest_min_separation_m).floor() as i64;
        if (bucket - 1..=bucket + 1).any(|k| {
            occupied.get(&k).is_some_and(|positions| {
                positions
                    .iter()
                    .any(|x| (x - crest.position_m).abs() + EPS < spec.crest_min_separation_m)
            })
        }) {
            continue;
        }
        occupied.entry(bucket).or_default().push(crest.position_m);
        selected.push(crest);
    }
    selected.sort_by(|a, b| a.position_m.total_cmp(&b.position_m));
    selected
}
