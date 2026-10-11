//! Explicit population declarations. Stress selection changes the population;
//! a shift needs a changed support/backend, rather than just fresh seeds.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::admission::barrier;
use crate::recipe_v2::validate_range_v2;
use crate::{MissionRecipeV2, RangeV1, TransferGeometryV1};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageAxisV1 {
    Distance,
    SignedRise,
    SourceWidth,
    TargetWidth,
    ChordBarrier,
    SustainedBarrierWidth,
    DepartureDistance,
    ApproachDistance,
    CrestCount,
    MinCrestSpacing,
}

impl CoverageAxisV1 {
    pub fn value(self, g: &TransferGeometryV1, barrier_height_m: f64) -> Option<f64> {
        match self {
            Self::Distance => Some(g.distance_m),
            Self::SignedRise => Some(g.signed_pad_rise_m),
            Self::SourceWidth => Some(g.source_width_m),
            Self::TargetWidth => Some(g.target_width_m),
            Self::ChordBarrier => Some(g.max_chord_barrier_m),
            Self::SustainedBarrierWidth => barrier(g, barrier_height_m).map(|b| b.longest_width_m),
            Self::DepartureDistance => {
                barrier(g, barrier_height_m).and_then(|b| b.departure_distance_m)
            }
            Self::ApproachDistance => {
                barrier(g, barrier_height_m).and_then(|b| b.approach_distance_m)
            }
            Self::CrestCount => Some(g.crests.len() as f64),
            Self::MinCrestSpacing => g.min_crest_spacing_m,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AxisConstraintV1 {
    pub axis: CoverageAxisV1,
    /// Inclusive bounds. Missing geometry never matches a numeric interval.
    pub range: RangeV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StressPanelV1 {
    pub id: String,
    pub quota: usize,
    pub constraints: Vec<AxisConstraintV1>,
}

impl StressPanelV1 {
    pub fn matches(&self, geometry: &TransferGeometryV1, height: f64) -> bool {
        self.constraints.iter().all(|c| {
            c.axis
                .value(geometry, height)
                .is_some_and(|v| v >= c.range.min && v <= c.range.max)
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PopulationDeclarationV2 {
    Broad,
    Stress {
        panels: Vec<StressPanelV1>,
    },
    Shift {
        reference_recipe: Box<MissionRecipeV2>,
        hypothesis: String,
    },
}

impl PopulationDeclarationV2 {
    pub fn validate(&self, recipe: &MissionRecipeV2) -> Result<(), String> {
        recipe.validate()?;
        match self {
            Self::Broad => Ok(()),
            Self::Stress { panels } => {
                if panels.is_empty()
                    || panels.len() > 16
                    || panels.iter().map(|p| &p.id).collect::<BTreeSet<_>>().len() != panels.len()
                    || panels.iter().any(|p| {
                        !safe_id(&p.id)
                            || p.quota == 0
                            || p.quota > 20000
                            || p.constraints.is_empty()
                            || p.constraints.len() > 10
                    })
                {
                    return Err(
                        "stress population requires distinct bounded panels with constraints"
                            .into(),
                    );
                }
                for panel in panels {
                    for constraint in &panel.constraints {
                        validate_range_v2(constraint.range)?;
                    }
                    let axes: Vec<_> = panel.constraints.iter().map(|c| c.axis).collect();
                    if axes.iter().enumerate().any(|(i, a)| axes[..i].contains(a)) {
                        return Err("a stress panel repeats an axis".into());
                    }
                }
                Ok(())
            }
            Self::Shift {
                reference_recipe,
                hypothesis,
            } => {
                reference_recipe.validate()?;
                if hypothesis.trim().is_empty() || hypothesis.len() > 4096 {
                    return Err("shift requires a bounded declared hypothesis".into());
                }
                let pairs = [
                    (recipe.travel_distance_m, reference_recipe.travel_distance_m),
                    (recipe.source_width_m, reference_recipe.source_width_m),
                    (recipe.target_width_m, reference_recipe.target_width_m),
                ]
                .into_iter()
                .chain(
                    recipe
                        .terrain
                        .values()
                        .into_iter()
                        .zip(reference_recipe.terrain.values()),
                );
                if recipe.backend == reference_recipe.backend
                    && !pairs
                        .into_iter()
                        .any(|(a, b)| a.max < b.min || b.max < a.min)
                {
                    return Err("shift must change backend or include a disjoint declared control range; fresh seeds are insufficient".into());
                }
                Ok(())
            }
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Broad => "broad",
            Self::Stress { .. } => "stress",
            Self::Shift { .. } => "shift",
        }
    }
}

pub(crate) fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}
