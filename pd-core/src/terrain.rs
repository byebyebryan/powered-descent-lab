use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::math::Vec2;

/// A conservative vehicle-center envelope used by exact terrain clearance queries.
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

    /// Return the exact minimum clearance between a point-centred envelope
    /// and the terrain over that envelope's lateral span.
    ///
    /// The terrain is piecewise linear, so its maximum over the closed span is
    /// attained at a span endpoint or at a terrain vertex contained by the
    /// span. A point query is intentionally strict about the terrain domain:
    /// callers must not accidentally turn an out-of-domain clearance into an
    /// edge-clamped observation.
    pub fn exact_point_clearance(
        &self,
        center_m: Vec2,
        envelope: CorridorEnvelope,
    ) -> Result<CorridorClearance, TerrainQueryError> {
        self.validate()
            .map_err(|message| TerrainQueryError::InvalidTerrain { message })?;
        envelope
            .validate()
            .map_err(|message| TerrainQueryError::InvalidCorridor { message })?;
        if !center_m.x.is_finite() || !center_m.y.is_finite() {
            return Err(TerrainQueryError::InvalidCorridor {
                message: "point query center must be finite".to_owned(),
            });
        }

        let domain = self.domain_x();
        let left = center_m.x - envelope.horizontal_extent_m;
        let right = center_m.x + envelope.horizontal_extent_m;
        for x_m in [left, right] {
            if !x_m.is_finite() {
                return Err(TerrainQueryError::InvalidCorridor {
                    message: "point query lateral span must be finite".to_owned(),
                });
            }
            if x_m < domain.0 || x_m > domain.1 {
                return Err(TerrainQueryError::DomainOverrun {
                    x_m,
                    domain_min_x_m: domain.0,
                    domain_max_x_m: domain.1,
                });
            }
        }

        let required_y = center_m.y - envelope.vertical_extent_m;
        if !required_y.is_finite() {
            return Err(TerrainQueryError::InvalidCorridor {
                message: "point query required envelope height must be finite".to_owned(),
            });
        }
        let mut candidate_xs = vec![left, right];
        for vertex in self.points() {
            if vertex.x >= left && vertex.x <= right {
                candidate_xs.push(vertex.x);
            }
        }

        let mut worst: Option<CorridorResidual> = None;
        for terrain_x in candidate_xs {
            let terrain_y = self.sample_height(terrain_x);
            let residual = terrain_y - required_y;
            if !residual.is_finite() {
                return Err(TerrainQueryError::InvalidCorridor {
                    message: "point query clearance residual must be finite".to_owned(),
                });
            }
            let candidate = CorridorResidual {
                residual_m: residual,
                centerline_position_m: center_m,
                terrain_position_m: Vec2::new(terrain_x, terrain_y),
                terrain_segment_index: self.segment_index_for(terrain_x),
                required_envelope_y_m: required_y,
                centerline_y_m: center_m.y,
                vertical_extent_m: envelope.vertical_extent_m,
            };
            if worst
                .as_ref()
                .is_none_or(|current| residual > current.residual_m)
            {
                worst = Some(candidate);
            }
        }

        let worst_residual = worst.ok_or_else(|| TerrainQueryError::InvalidCorridor {
            message: "point query produced no evaluation points".to_owned(),
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
    fn exact_point_clearance_handles_zero_horizontal_span() {
        let terrain = TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0)],
        };
        let clearance = terrain
            .exact_point_clearance(Vec2::new(5.0, 20.0), CorridorEnvelope::new(0.0, 2.0))
            .unwrap();
        assert!(clearance.clear);
        assert!((clearance.minimum_clearance_m - 13.0).abs() < 1.0e-12);
        assert_eq!(
            clearance.worst_residual.terrain_position_m,
            Vec2::new(5.0, 5.0)
        );
    }

    #[test]
    fn exact_point_clearance_handles_zero_vertical_extent_and_vertices() {
        let terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(5.0, 20.0),
                Vec2::new(10.0, 0.0),
            ],
        };
        let clearance = terrain
            .exact_point_clearance(Vec2::new(5.0, 10.0), CorridorEnvelope::new(5.0, 0.0))
            .unwrap();
        assert!(!clearance.clear);
        assert_eq!(clearance.minimum_clearance_m, -10.0);
        assert_eq!(
            clearance.worst_residual.terrain_position_m,
            Vec2::new(5.0, 20.0)
        );
        assert_eq!(clearance.worst_residual.terrain_segment_index, 0);
    }

    #[test]
    fn exact_point_clearance_rejects_domain_and_geometry_errors() {
        let terrain = TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)],
        };
        assert!(matches!(
            terrain.exact_point_clearance(Vec2::new(0.0, 1.0), CorridorEnvelope::new(0.1, 0.0)),
            Err(TerrainQueryError::DomainOverrun { .. })
        ));
        assert!(matches!(
            terrain.exact_point_clearance(Vec2::new(5.0, 1.0), CorridorEnvelope::new(-1.0, 0.0)),
            Err(TerrainQueryError::InvalidCorridor { .. })
        ));
        assert!(matches!(
            terrain
                .exact_point_clearance(Vec2::new(f64::NAN, 1.0), CorridorEnvelope::new(0.0, 0.0)),
            Err(TerrainQueryError::InvalidCorridor { .. })
        ));
    }
}
