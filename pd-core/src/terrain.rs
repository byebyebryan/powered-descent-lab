use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::math::Vec2;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerrainDefinition {
    Heightfield { points_m: Vec<Vec2> },
}

impl TerrainDefinition {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Heightfield { points_m } => {
                if points_m.len() < 2 {
                    return Err("heightfield terrain needs at least two points".to_owned());
                }
                let mut prev_x = f64::NEG_INFINITY;
                for point in points_m {
                    if !point.x.is_finite() || !point.y.is_finite() {
                        return Err("terrain points must be finite".to_owned());
                    }
                    if point.x <= prev_x {
                        return Err(
                            "heightfield points must be strictly increasing in x".to_owned()
                        );
                    }
                    prev_x = point.x;
                }
                Ok(())
            }
        }
    }

    pub fn points(&self) -> &[Vec2] {
        match self {
            Self::Heightfield { points_m } => points_m.as_slice(),
        }
    }

    pub fn sample_height(&self, x_m: f64) -> f64 {
        match self {
            Self::Heightfield { points_m } => {
                let segment = self.segment_index_for(x_m);
                let p0 = points_m[segment];
                let p1 = points_m[segment + 1];
                let dx = p1.x - p0.x;
                if dx.abs() <= f64::EPSILON {
                    return p0.y;
                }
                let t = ((x_m - p0.x) / dx).clamp(0.0, 1.0);
                p0.y + ((p1.y - p0.y) * t)
            }
        }
    }

    pub fn sample_slope(&self, x_m: f64) -> f64 {
        match self {
            Self::Heightfield { points_m } => {
                let segment = self.segment_index_for(x_m);
                let p0 = points_m[segment];
                let p1 = points_m[segment + 1];
                let dx = p1.x - p0.x;
                if dx.abs() <= f64::EPSILON {
                    0.0
                } else {
                    (p1.y - p0.y) / dx
                }
            }
        }
    }

    pub fn sample_surface_normal(&self, x_m: f64) -> Vec2 {
        let slope = self.sample_slope(x_m);
        let normal = Vec2::new(-slope, 1.0);
        let length = normal.length();
        if length <= f64::EPSILON {
            Vec2::new(0.0, 1.0)
        } else {
            Vec2::new(normal.x / length, normal.y / length)
        }
    }

    fn segment_index_for(&self, x_m: f64) -> usize {
        match self {
            Self::Heightfield { points_m } => {
                if x_m <= points_m[0].x {
                    return 0;
                }
                if x_m >= points_m[points_m.len() - 1].x {
                    return points_m.len() - 2;
                }
                match points_m
                    .binary_search_by(|point| point.x.partial_cmp(&x_m).unwrap_or(Ordering::Less))
                {
                    Ok(index) => index.saturating_sub(1).min(points_m.len() - 2),
                    Err(index) => index.saturating_sub(1).min(points_m.len() - 2),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_heightfield_linearly() {
        let terrain = TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(-10.0, 0.0), Vec2::new(10.0, 20.0)],
        };

        assert_eq!(terrain.sample_height(0.0), 10.0);
        assert_eq!(terrain.sample_slope(0.0), 1.0);
    }
}
