use pd_missions::*;
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

fn axis(axis: CoverageAxisV1, edges: &[f64]) -> DiagnosticAxisV1 {
    DiagnosticAxisV1 {
        axis,
        interior_boundaries: edges.to_vec(),
        show_underflow_overflow: true,
        show_missing: true,
    }
}
fn design() -> CoverageDesignV1 {
    CoverageDesignV1 {
        schema_version: 1,
        axes: vec![
            axis(CoverageAxisV1::Distance, &[100.0, 200.0]),
            axis(CoverageAxisV1::ApproachDistance, &[100.0, 200.0]),
        ],
        joints: vec![[CoverageAxisV1::Distance, CoverageAxisV1::ApproachDistance]],
    }
}
fn challenge() -> ChallengeReceiptV1 {
    let measured = ChallengeMeasurementV1 {
        sustained_barrier: true,
        elevation_transition: false,
        successive_crests: false,
        qualifying_crest_count: 0,
    };
    ChallengeReceiptV1 {
        raw: measured.clone(),
        prepared: measured,
        qualifies: true,
    }
}
fn row(index: u64, x: f64, y: Option<f64>) -> InputAuditRowV1 {
    InputAuditRowV1 {
        index,
        proposal: CollectionEntryV1 {
            index,
            file: format!("proposal-{index:020}.json"),
            sha256: "a".repeat(64),
            physical_sha256: format!("{index:064x}"),
        },
        disposition: SelectionDispositionV2::Accepted,
        direction: if index.is_multiple_of(2) { 1 } else { -1 },
        labels: vec![],
        values: vec![
            AxisValueV1 {
                axis: CoverageAxisV1::Distance,
                value: Some(x),
            },
            AxisValueV1 {
                axis: CoverageAxisV1::ApproachDistance,
                value: y,
            },
        ],
        scalar_metrics: BTreeMap::from([("relief_m".into(), Some(x))]),
        challenge: challenge(),
        boundary_score: 0.5,
        shape_signature_sha256: format!("{index:064x}"),
    }
}
fn spec(request: &CollectionRequestV2) -> InputAuditSpecV1 {
    let reference_qa = ReferenceQaSpecV1 {
        spacing_m: request.recipe.terrain_spacing_m,
        regular_index_stride: 80,
        regular_index_offsets: vec![0, 1],
        boundary_extra_per_backend: 2,
        maximum_per_backend: 4,
        diagnostic_flags: ResolutionDiagnosticFlagsV1 {
            maximum_raw_interpolation_error_m: 25.0,
            absolute_chord_barrier_change_m: 25.0,
            absolute_sustained_width_change_m: 40.0,
            admission_disagreement: true,
        },
    };
    InputAuditSpecV1 {
        schema_version: 1,
        audit_version: INPUT_AUDIT_VERSION.into(),
        coverage: design(),
        reference_qa,
        preview_maximum: 12,
    }
}

#[test]
fn bins_keep_equality_overflow_missing_and_direction_denominators() {
    let rows = [
        row(0, 0.0, None),
        row(1, 100.0, 0.0.into()),
        row(2, 200.0, 100.0.into()),
        row(3, 300.0, 200.0.into()),
    ];
    let p = summarize_coverage(&rows.iter().collect::<Vec<_>>(), &design()).unwrap();
    assert_eq!(p.count, 4);
    assert_eq!(p.rightward, 2);
    assert_eq!(p.leftward, 2);
    assert_eq!(p.axes[0].counts, vec![1, 1, 2, 0]);
    assert_eq!(p.axes[1].counts, vec![1, 1, 1, 1]);
    assert_eq!(p.axes[0].right_counts, vec![1, 0, 1, 0]);
    assert_eq!(p.axes[0].left_counts, vec![0, 1, 1, 0]);
    assert_eq!(p.joints[0].counts[0][3], 1);
    assert_eq!(p.joints[0].counts.iter().flatten().sum::<usize>(), 4);
    assert_eq!(p.joints[0].observed_pairs, 3);
    assert!((p.joints[0].pearson_r.unwrap() - 1.0).abs() < 1e-12);
    assert_eq!(p.axes[1].summary.missing, 1);
    assert_eq!(p.axes[0].summary.p50, Some(150.0));
    assert_eq!(p.morphology_groups["unclassified"], 4);
    assert_eq!(p.directions_by_group["unclassified"], [2, 2]);
}

#[test]
fn empty_and_constant_populations_do_not_invent_correlations_or_ranges() {
    let p = summarize_coverage(&[], &design()).unwrap();
    assert_eq!(p.count, 0);
    assert_eq!(p.axes[0].summary.min, None);
    assert_eq!(p.joints[0].pearson_r, None);
    assert_eq!(p.concentration.largest_bucket, 0);
    let rows = [row(0, 100.0, Some(2.0)), row(1, 100.0, Some(2.0))];
    let p = summarize_coverage(&rows.iter().collect::<Vec<_>>(), &design()).unwrap();
    assert_eq!(p.joints[0].observed_pairs, 2);
    assert_eq!(p.joints[0].pearson_r, None);
    assert_eq!(p.axes[0].summary.p10, Some(100.0));
    assert!(summarize_scalar([Some(f64::NAN)]).is_err());
}

#[test]
fn overlapping_labels_and_coarse_shape_buckets_do_not_replace_unique_counts() {
    let mut rows = [
        row(0, 50.0, Some(100.0)),
        row(1, 100.0, Some(50.0)),
        row(2, 150.0, Some(0.0)),
    ];
    rows[0].labels = vec![
        MorphologyLabelV1::MountainCrossing,
        MorphologyLabelV1::BroadMassif,
    ];
    rows[1].labels = vec![MorphologyLabelV1::MountainCrossing];
    rows[1].shape_signature_sha256 = rows[0].shape_signature_sha256.clone();
    let p = summarize_coverage(&rows.iter().collect::<Vec<_>>(), &design()).unwrap();
    assert_eq!(p.count, 3);
    assert_eq!(p.morphology_groups.values().sum::<usize>(), 3);
    assert_eq!(p.overlapping_labels["mountain_crossing"], 2);
    assert_eq!(p.overlapping_labels["broad_massif"], 1);
    assert_eq!(p.concentration.unique_buckets, 2);
    assert_eq!(p.concentration.rows_in_shared_buckets, 2);
    assert_eq!(
        p.concentration.largest_groups[0].example_indices,
        vec![0, 1]
    );
    assert!((p.joints[0].pearson_r.unwrap() + 1.0).abs() < 1e-12);
    let reversed = summarize_coverage(&rows.iter().rev().collect::<Vec<_>>(), &design()).unwrap();
    assert_eq!(p, reversed);
}

#[test]
fn designs_reject_hidden_missing_values_repeated_axes_and_undefined_joints() {
    let mut d = design();
    d.axes[0].show_missing = false;
    assert!(d.validate().is_err());
    d = design();
    d.axes[0].interior_boundaries = vec![200.0, 100.0];
    assert!(d.validate().is_err());
    d = design();
    d.axes.push(d.axes[0].clone());
    assert!(d.validate().is_err());
    d = design();
    d.joints
        .push([CoverageAxisV1::ApproachDistance, CoverageAxisV1::Distance]);
    assert!(d.validate().is_err());
    let mut r = row(0, 10.0, None);
    r.values.pop();
    assert!(summarize_coverage(&[&r], &design()).is_err());
}

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "pd-input-audit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn saved_input_audit_is_repeatable_keeps_exclusions_and_preserves_physical_files() {
    let temp = Temp::new();
    let root = temp.0.join("collection");
    let request = CollectionRequestV2::broad(MissionRecipeV2::default(), 981, 32);
    let manifest = write_collection_v2(&root, request.clone()).unwrap();
    let spec = spec(&request);
    let a = audit_collection_v2(&root, spec.clone()).unwrap();
    let b = audit_collection_v2(&root, spec).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.rows.len(), 32);
    assert_eq!(a.proposed.count, 32);
    assert_eq!(a.admitted.count, manifest.counts.accepted);
    assert!(
        a.rows
            .iter()
            .any(|r| r.disposition == SelectionDispositionV2::InsufficientChallenge)
    );
    assert_eq!(a.reference_qa.len(), 4);
    assert_eq!(
        a.reference_qa[..2]
            .iter()
            .map(|q| q.index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(a.preview_indices.len(), 12);
    assert_eq!(a.immutable_file_sha256.len(), 34);
    for r in &a.rows {
        assert!(r.scalar_metrics["outside_pad_p95_change"].unwrap() < 1e-8);
    }
    for q in &a.reference_qa {
        assert!(q.maximum_raw_interpolation_error_m < 1e-8);
        assert!(q.chord_barrier_change_m.abs() < 1e-8);
        assert!(q.sustained_width_change_m.abs() < 1e-8);
        assert!(!q.flags.admission_disagreement);
    }
    assert_eq!(verify_collection_v2(&root).unwrap(), manifest);
    let excluded = a
        .rows
        .iter()
        .find(|r| r.disposition == SelectionDispositionV2::InsufficientChallenge)
        .unwrap();
    fs::write(
        root.join(&excluded.proposal.file),
        b"changed excluded geometry",
    )
    .unwrap();
    assert!(audit_collection_v2(&root, a.spec).is_err());
}

#[test]
fn finer_reference_retains_physics_and_remeasures_pad_heights_in_its_receipt() {
    let request = CollectionRequestV2::broad(MissionRecipeV2::default(), 981, 2);
    let mission = generate_mission_v2(&request.recipe, &request.geometry_spec, 981, 0)
        .unwrap()
        .mission
        .unwrap();
    let bytes = json_bytes(&mission).unwrap();
    let mut qa = spec(&request).reference_qa;
    qa.spacing_m = 1.0;
    let measured = measure_reference_qa(&mission, &request.challenge_rule, &qa).unwrap();
    let fine = mission.raw_reference(1.0).unwrap();
    for (i, pad) in mission.scenario.world.landing_pads.iter().enumerate() {
        assert_eq!(
            measured.reference_pad_surface_y_m[i],
            (fine.sample_height(pad.center_x_m) * 1000.0).round() / 1000.0
        );
    }
    assert_eq!(measured.physical_sha256, mission.physical_sha256);
    assert_eq!(json_bytes(&mission).unwrap(), bytes);
    assert!(measured.maximum_raw_interpolation_error_m > 0.0);
    assert_ne!(measured.reference_raw_sha256, mission.raw_terrain_sha256);
}

#[test]
fn qa_budgets_fail_before_expensive_audits_and_regular_schedule_is_balanced() {
    let request = CollectionRequestV2::broad(MissionRecipeV2::default(), 1, 10000);
    let mut audit = spec(&request);
    audit.reference_qa.maximum_per_backend = 282;
    audit.reference_qa.boundary_extra_per_backend = 32;
    audit.validate(&request).unwrap();
    let selected: Vec<_> = request
        .indices
        .iter()
        .copied()
        .filter(|i| audit.reference_qa.is_regular(*i))
        .collect();
    assert_eq!(selected.len(), 250);
    assert_eq!(selected.iter().filter(|i| i.is_multiple_of(2)).count(), 125);
    audit.reference_qa.maximum_per_backend = 281;
    assert!(audit.validate(&request).is_err());
    audit.reference_qa.regular_index_stride = 0;
    assert!(audit.validate(&request).is_err());
    assert!(!audit.reference_qa.is_regular(0));
}
