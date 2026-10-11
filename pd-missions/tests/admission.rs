use pd_core::{LandingPadSpec, TerrainDefinition, Vec2};
use pd_missions::*;

fn geometry(points: &[(f64, f64)]) -> TransferGeometryV1 {
    let terrain = TerrainDefinition::Heightfield {
        points_m: points.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
    };
    measure_transfer_geometry(
        &terrain,
        &LandingPadSpec {
            id: "source".into(),
            center_x_m: 0.0,
            surface_y_m: points[0].1,
            width_m: 36.0,
        },
        &LandingPadSpec {
            id: "target".into(),
            center_x_m: 1000.0,
            surface_y_m: points[points.len() - 1].1,
            width_m: 36.0,
        },
        &GeometrySpecV1::default(),
        &[],
    )
    .unwrap()
}

fn admitted(g: &TransferGeometryV1) -> bool {
    ChallengeReceiptV1::measure(&ChallengeRuleV1::default(), g, g).qualifies
}

#[test]
fn mild_and_needle_terrain_fail_but_sustained_unclassified_terrain_is_retained() {
    assert!(!admitted(&geometry(&[
        (0.0, 0.0),
        (500.0, 40.0),
        (1000.0, 0.0)
    ])));
    assert!(!admitted(&geometry(&[
        (0.0, 0.0),
        (499.0, 0.0),
        (500.0, 250.0),
        (501.0, 0.0),
        (1000.0, 0.0)
    ])));
    let g = geometry(&[
        (0.0, 0.0),
        (350.0, 0.0),
        (500.0, 150.0),
        (650.0, 0.0),
        (1000.0, 0.0),
    ]);
    assert!(admitted(&g));
    assert_eq!(g.barriers[1].longest_width_m, 100.0);
    assert!(MorphologySpecV1::default().labels(&g).is_empty());
}

#[test]
fn elevation_and_repeated_crest_paths_are_independent_of_mountain_labels() {
    for rise in [-300.0, 300.0] {
        let g = geometry(&[(0.0, 0.0), (1000.0, rise)]);
        assert!(admitted(&g));
        assert_eq!(g.max_chord_barrier_m, 0.0);
        assert!(MorphologySpecV1::default().labels(&g).is_empty());
    }
    let g = geometry(&[
        (0.0, 0.0),
        (150.0, 0.0),
        (250.0, 90.0),
        (350.0, 0.0),
        (550.0, 0.0),
        (650.0, 90.0),
        (750.0, 0.0),
        (1000.0, 0.0),
    ]);
    let c = ChallengeRuleV1::default().measure(&g);
    assert!(c.successive_crests);
    assert_eq!(c.qualifying_crest_count, 2);
    assert!(!c.sustained_barrier);
    assert!(MorphologySpecV1::default().labels(&g).is_empty());
}

#[test]
fn pad_preparation_cannot_manufacture_a_qualifying_challenge() {
    let raw = geometry(&[(0.0, 0.0), (1000.0, 0.0)]);
    let prepared = geometry(&[(0.0, 0.0), (500.0, 400.0), (1000.0, 0.0)]);
    assert!(admitted(&prepared));
    let receipt = ChallengeReceiptV1::measure(&ChallengeRuleV1::default(), &raw, &prepared);
    assert!(receipt.prepared.sustained_barrier);
    assert!(!receipt.qualifies);
    let uphill = geometry(&[(0.0, 0.0), (1000.0, 400.0)]);
    assert!(
        !ChallengeReceiptV1::measure(&ChallengeRuleV1::default(), &uphill, &prepared).qualifies
    );
}

#[test]
fn morphology_overlaps_and_threshold_equality_is_explicit() {
    let g = geometry(&[(0.0, 0.0), (500.0, 400.0), (1000.0, 0.0)]);
    assert_eq!(
        MorphologySpecV1::default().labels(&g),
        vec![
            MorphologyLabelV1::MountainCrossing,
            MorphologyLabelV1::BroadMassif
        ]
    );
    let mut rule = ChallengeRuleV1 {
        sustained_width_m: g.barriers[1].longest_width_m,
        ..ChallengeRuleV1::default()
    };
    assert!(rule.measure(&g).sustained_barrier);
    rule.sustained_width_m += 1e-6;
    assert!(!rule.measure(&g).sustained_barrier);
}

#[test]
fn challenge_and_projection_validate_observability_and_meaningful_lower_bounds() {
    let spec = GeometrySpecV1::default();
    let mut rule = ChallengeRuleV1 {
        sustained_width_m: 0.0,
        ..ChallengeRuleV1::default()
    };
    assert!(rule.validate(&spec).is_err());
    rule = ChallengeRuleV1::default();
    rule.barrier_height_m = 123.0;
    assert!(rule.validate(&spec).is_err());
    let mut coarse = spec.clone();
    coarse.crest_min_separation_m = 100.0;
    assert!(ChallengeRuleV1::default().validate(&coarse).is_err());
    assert_ne!(
        ChallengeRuleV1::default().identity(&spec).unwrap(),
        ChallengeRuleV1 {
            absolute_pad_rise_m: 350.0,
            ..ChallengeRuleV1::default()
        }
        .identity(&spec)
        .unwrap()
    );
}

#[test]
fn stress_axes_preserve_null_obstacles_and_shift_requires_changed_support() {
    let flat = geometry(&[(0.0, 0.0), (1000.0, 0.0)]);
    assert_eq!(CoverageAxisV1::DepartureDistance.value(&flat, 100.0), None);
    assert_eq!(CoverageAxisV1::MinCrestSpacing.value(&flat, 100.0), None);
    let panel = StressPanelV1 {
        id: "near".into(),
        quota: 1,
        constraints: vec![AxisConstraintV1 {
            axis: CoverageAxisV1::DepartureDistance,
            range: RangeV1 {
                min: 0.0,
                max: 100.0,
            },
        }],
    };
    assert!(!panel.matches(&flat, 100.0));
    let recipe = MissionRecipeV2::default();
    let shift = PopulationDeclarationV2::Shift {
        reference_recipe: Box::new(recipe.clone()),
        hypothesis: "Fresh seed".into(),
    };
    assert!(shift.validate(&recipe).is_err());
    assert!(shift.validate(&MissionRecipeV2::value_noise()).is_ok());
    let mut longer = recipe.clone();
    longer.travel_distance_m = RangeV1 {
        min: 2500.0,
        max: 3000.0,
    };
    assert!(shift.validate(&longer).is_ok());
}
