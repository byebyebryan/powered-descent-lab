use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::math::Vec2;

/// A conservative vehicle-center envelope used by exact corridor queries.
///
/// The envelope is axis aligned in the world frame.  Keeping this primitive
/// independent of the vehicle model lets callers describe endpoint tapers and
/// other policy-owned profiles without making terrain depend on a planner.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorridorEnvelope {
    pub horizontal_extent_m: f64,
    pub vertical_extent_m: f64,
}

impl CorridorEnvelope {
    pub const fn new(horizontal_extent_m: f64, vertical_extent_m: f64) -> Self {
        Self {
            horizontal_extent_m,
            vertical_extent_m,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.horizontal_extent_m.is_finite() || self.horizontal_extent_m < 0.0 {
            return Err("corridor horizontal extent must be finite and non-negative".to_owned());
        }
        if !self.vertical_extent_m.is_finite() || self.vertical_extent_m < 0.0 {
            return Err("corridor vertical extent must be finite and non-negative".to_owned());
        }
        Ok(())
    }
}

/// A strict terrain query failure.  Unlike [`TerrainDefinition::sample_height`],
/// strict queries never clamp an out-of-domain horizontal coordinate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TerrainQueryError {
    InvalidTerrain {
        message: String,
    },
    DomainOverrun {
        x_m: f64,
        domain_min_x_m: f64,
        domain_max_x_m: f64,
    },
    InvalidCorridor {
        message: String,
    },
}

impl std::fmt::Display for TerrainQueryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTerrain { message } | Self::InvalidCorridor { message } => {
                formatter.write_str(message)
            }
            Self::DomainOverrun {
                x_m,
                domain_min_x_m,
                domain_max_x_m,
            } => write!(
                formatter,
                "terrain query x={x_m} lies outside domain [{domain_min_x_m}, {domain_max_x_m}]"
            ),
        }
    }
}

impl std::error::Error for TerrainQueryError {}

/// The exact worst residual found by a corridor query.
///
/// `residual_m` is terrain height minus the required lower centerline
/// envelope.  A non-positive residual is clear; a positive residual is a
/// collision.  The location records both the centerline position and the
/// terrain location that produced the residual so planner diagnostics can
/// identify a narrow vertex or a lateral-envelope collision.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorridorResidual {
    pub residual_m: f64,
    pub centerline_position_m: Vec2,
    pub terrain_position_m: Vec2,
    pub terrain_segment_index: usize,
    pub required_envelope_y_m: f64,
    pub centerline_y_m: f64,
    pub vertical_extent_m: f64,
}

/// Result of an exact corridor query over a piecewise-linear heightfield.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorridorClearance {
    pub clear: bool,
    pub minimum_clearance_m: f64,
    pub worst_residual: CorridorResidual,
}

impl CorridorClearance {
    pub fn residual_m(&self) -> f64 {
        self.worst_residual.residual_m
    }
}

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

    /// Sample terrain without the historical edge clamping behavior.
    pub fn sample_height_strict(&self, x_m: f64) -> Result<f64, TerrainQueryError> {
        self.validate()
            .map_err(|message| TerrainQueryError::InvalidTerrain { message })?;
        let (min_x, max_x) = self.domain_x();
        if !x_m.is_finite() {
            return Err(TerrainQueryError::InvalidCorridor {
                message: "strict terrain query x must be finite".to_owned(),
            });
        }
        if x_m < min_x || x_m > max_x {
            return Err(TerrainQueryError::DomainOverrun {
                x_m,
                domain_min_x_m: min_x,
                domain_max_x_m: max_x,
            });
        }
        Ok(self.sample_height(x_m))
    }

    fn domain_x(&self) -> (f64, f64) {
        let points = self.points();
        (points[0].x, points[points.len() - 1].x)
    }

    /// Return the exact minimum centerline-to-terrain clearance for a segment.
    ///
    /// The centerline and both envelope extents are linearly interpolated over
    /// the segment.  The query partitions the segment at every terrain vertex
    /// crossing of the left/right lateral envelope and evaluates each linear
    /// residual at its exact interval endpoints.  This is sufficient for a
    /// piecewise-linear terrain/profile pair and intentionally does not rely on
    /// fixed sampling.
    pub fn exact_corridor_clearance(
        &self,
        start_m: Vec2,
        end_m: Vec2,
        start_envelope: CorridorEnvelope,
        end_envelope: CorridorEnvelope,
    ) -> Result<CorridorClearance, TerrainQueryError> {
        self.validate()
            .map_err(|message| TerrainQueryError::InvalidTerrain { message })?;
        start_envelope
            .validate()
            .and_then(|_| end_envelope.validate())
            .map_err(|message| TerrainQueryError::InvalidCorridor { message })?;
        for (label, point) in [("start", start_m), ("end", end_m)] {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(TerrainQueryError::InvalidCorridor {
                    message: format!("corridor {label} point must be finite"),
                });
            }
        }
        let dx = end_m.x - start_m.x;
        if dx.abs() <= f64::EPSILON {
            return Err(TerrainQueryError::InvalidCorridor {
                message: "corridor segment must have non-zero horizontal span".to_owned(),
            });
        }

        // The lateral interval is linear in segment parameter.  Checking its
        // endpoints is enough to reject a domain overrun, because the terrain
        // domain itself is an interval.
        let domain = self.domain_x();
        let left_start = start_m.x - start_envelope.horizontal_extent_m;
        let right_start = start_m.x + start_envelope.horizontal_extent_m;
        let left_end = end_m.x - end_envelope.horizontal_extent_m;
        let right_end = end_m.x + end_envelope.horizontal_extent_m;
        for x_m in [left_start, right_start, left_end, right_end] {
            if x_m < domain.0 || x_m > domain.1 {
                return Err(TerrainQueryError::DomainOverrun {
                    x_m,
                    domain_min_x_m: domain.0,
                    domain_max_x_m: domain.1,
                });
            }
        }

        // Build an exact arrangement of all parameter locations at which an
        // envelope boundary crosses a terrain vertex.  On each resulting
        // interval, terrain evaluated at either envelope boundary and at any
        // included terrain vertex is affine, so its maximum is at an endpoint.
        let mut parameters = vec![0.0, 1.0];
        for vertex in self.points().iter().skip(1).take(self.points().len() - 2) {
            if let Some(t) = solve_linear_crossing(left_start, left_end, vertex.x)
                && t > 0.0
                && t < 1.0
            {
                parameters.push(t);
            }
            if let Some(t) = solve_linear_crossing(right_start, right_end, vertex.x)
                && t > 0.0
                && t < 1.0
            {
                parameters.push(t);
            }
        }
        parameters.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        parameters.dedup_by(|a, b| (*a - *b).abs() <= 1.0e-12);

        let mut worst: Option<CorridorResidual> = None;
        for window in parameters.windows(2) {
            let t0 = window[0];
            let t1 = window[1];
            // Evaluate both interval endpoints.  This catches envelope
            // boundaries and every vertex entering/leaving the lateral span.
            for t in [t0, t1] {
                let center = interpolate(start_m, end_m, t);
                let horizontal_extent = lerp(
                    start_envelope.horizontal_extent_m,
                    end_envelope.horizontal_extent_m,
                    t,
                );
                let vertical_extent = lerp(
                    start_envelope.vertical_extent_m,
                    end_envelope.vertical_extent_m,
                    t,
                );
                let left = center.x - horizontal_extent;
                let right = center.x + horizontal_extent;

                // The domain check above covers the extrema of the lateral
                // boundaries.  Keep the per-evaluation guard for numerical
                // safety and to make this function robust to future profile
                // changes.
                for x_m in [left, right] {
                    if x_m < domain.0 || x_m > domain.1 {
                        return Err(TerrainQueryError::DomainOverrun {
                            x_m,
                            domain_min_x_m: domain.0,
                            domain_max_x_m: domain.1,
                        });
                    }
                }

                let mut evaluate = |terrain_x: f64| {
                    let terrain_y = self.sample_height(terrain_x);
                    let required_y = center.y - vertical_extent;
                    let residual = terrain_y - required_y;
                    let candidate = CorridorResidual {
                        residual_m: residual,
                        centerline_position_m: center,
                        terrain_position_m: Vec2::new(terrain_x, terrain_y),
                        terrain_segment_index: self.segment_index_for(terrain_x),
                        required_envelope_y_m: required_y,
                        centerline_y_m: center.y,
                        vertical_extent_m: vertical_extent,
                    };
                    if worst
                        .as_ref()
                        .is_none_or(|current| residual > current.residual_m)
                    {
                        worst = Some(candidate);
                    }
                };

                evaluate(left);
                evaluate(right);
                for (index, vertex) in self.points().iter().enumerate() {
                    if vertex.x >= left && vertex.x <= right {
                        // Record the canonical vertex segment.  At a shared
                        // vertex either adjacent segment is equivalent for
                        // clearance; retaining the lower-index segment keeps
                        // diagnostics stable.
                        let _ = index;
                        evaluate(vertex.x);
                    }
                }
            }
        }

        let worst_residual = worst.ok_or_else(|| TerrainQueryError::InvalidCorridor {
            message: "corridor query produced no evaluation points".to_owned(),
        })?;
        Ok(CorridorClearance {
            clear: worst_residual.residual_m <= 0.0,
            minimum_clearance_m: -worst_residual.residual_m,
            worst_residual,
        })
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

fn interpolate(start: Vec2, end: Vec2, t: f64) -> Vec2 {
    Vec2::new(lerp(start.x, end.x, t), lerp(start.y, end.y, t))
}

fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + ((end - start) * t)
}

fn solve_linear_crossing(start: f64, end: f64, value: f64) -> Option<f64> {
    let delta = end - start;
    if delta.abs() <= f64::EPSILON {
        None
    } else {
        Some((value - start) / delta)
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

    #[test]
    fn strict_height_query_rejects_domain_overrun() {
        let terrain = TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(-10.0, 0.0), Vec2::new(10.0, 0.0)],
        };
        assert!(matches!(
            terrain.sample_height_strict(-10.001),
            Err(TerrainQueryError::DomainOverrun { .. })
        ));
        assert_eq!(terrain.sample_height_strict(10.0).unwrap(), 0.0);
    }

    #[test]
    fn exact_corridor_catches_narrow_terrain_vertex() {
        let terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-10.0, 0.0),
                Vec2::new(0.0, 20.0),
                Vec2::new(10.0, 0.0),
            ],
        };
        let clearance = terrain
            .exact_corridor_clearance(
                Vec2::new(-10.0, 10.0),
                Vec2::new(10.0, 10.0),
                CorridorEnvelope::new(0.0, 0.0),
                CorridorEnvelope::new(0.0, 0.0),
            )
            .unwrap();
        assert!(!clearance.clear);
        assert_eq!(
            clearance.worst_residual.terrain_position_m,
            Vec2::new(0.0, 20.0)
        );
        assert_eq!(clearance.worst_residual.terrain_segment_index, 0);
    }

    #[test]
    fn exact_corridor_accounts_for_lateral_envelope_on_steep_slope() {
        let terrain = TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 100.0)],
        };
        let clearance = terrain
            .exact_corridor_clearance(
                Vec2::new(4.0, 55.0),
                Vec2::new(6.0, 55.0),
                CorridorEnvelope::new(2.0, 0.0),
                CorridorEnvelope::new(2.0, 0.0),
            )
            .unwrap();
        assert!(!clearance.clear);
        assert!(clearance.worst_residual.terrain_position_m.x >= 7.9);
    }

    #[test]
    fn exact_threshold_is_clear_but_lower_centerline_is_not() {
        let terrain = TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)],
        };
        let threshold = terrain
            .exact_corridor_clearance(
                Vec2::new(0.0, 10.0),
                Vec2::new(10.0, 10.0),
                CorridorEnvelope::new(0.0, 10.0),
                CorridorEnvelope::new(0.0, 10.0),
            )
            .unwrap();
        assert!(threshold.clear);
        assert_eq!(threshold.minimum_clearance_m, 0.0);

        let below = terrain
            .exact_corridor_clearance(
                Vec2::new(0.0, 9.999),
                Vec2::new(10.0, 9.999),
                CorridorEnvelope::new(0.0, 10.0),
                CorridorEnvelope::new(0.0, 10.0),
            )
            .unwrap();
        assert!(!below.clear);
    }
}
