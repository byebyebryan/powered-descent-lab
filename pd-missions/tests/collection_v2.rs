use pd_missions::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pd-collection-v2-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn child(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_replaced(root: &Path, manifest: &CollectionManifestV2) {
    // Deliberately corrupt only new test-owned inputs, never frozen fixtures.
    fs::write(root.join("manifest.json"), json_bytes(manifest).unwrap()).unwrap();
}

#[test]
fn canonical_broad_collections_account_for_every_request_and_retain_exclusions() {
    let tmp = Temp::new();
    let request = CollectionRequestV2::broad(MissionRecipeV2::default(), 981, 32);
    let a = write_collection_v2(&tmp.child("a"), request.clone()).unwrap();
    let mut reversed = request;
    reversed.indices.reverse();
    let b = write_collection_v2(&tmp.child("b"), reversed).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.counts.scheduled, 32);
    assert!(a.counts.accepted > 0);
    assert_eq!(
        a.counts.accepted
            + a.counts.insufficient_challenge
            + a.counts.physical_duplicate
            + a.counts.structural_exhaustion
            + a.counts.stress_excluded,
        32
    );
    assert_eq!(verify_collection_v2(&tmp.child("a")).unwrap(), a);
    for r in &a.receipts {
        if let Some(p) = &r.proposal {
            assert_eq!(
                fs::read(tmp.child("a").join(&p.file)).unwrap(),
                fs::read(tmp.child("b").join(&p.file)).unwrap()
            );
            let m = load_collection_proposal_v2(&tmp.child("a"), &a, r).unwrap();
            assert_eq!(m.geometry.spec, a.request.geometry_spec);
            assert_eq!(
                r.challenge.as_ref().unwrap().qualifies,
                r.disposition == SelectionDispositionV2::Accepted
            );
        }
    }
    assert!(write_collection_v2(&tmp.child("a"), a.request.clone()).is_err());
}

#[test]
fn taxonomy_changes_keep_broad_membership_and_challenging_unclassified_inputs() {
    let tmp = Temp::new();
    let request = CollectionRequestV2::broad(MissionRecipeV2::default(), 22, 16);
    let a = write_collection_v2(&tmp.child("a"), request.clone()).unwrap();
    let mut relabeled = request;
    relabeled.morphology_spec.mountain_height_m = 100000.0;
    relabeled.morphology_spec.massif_width_m = 100000.0;
    relabeled.morphology_spec.absolute_pad_rise_m = 100000.0;
    relabeled.morphology_spec.ridge_min_prominence_m = 100000.0;
    let b = write_collection_v2(&tmp.child("b"), relabeled).unwrap();
    assert!(!a.entries.is_empty());
    assert_eq!(a.entries, b.entries);
    assert_eq!(a.counts, b.counts);
    assert!(b.receipts.iter().all(|r| r.labels.is_empty()));
    assert_eq!(verify_collection_v2(&tmp.child("b")).unwrap(), b);
}

#[test]
fn flat_inputs_and_structural_exhaustion_are_complete_distinct_receipts() {
    let tmp = Temp::new();
    let mut recipe = MissionRecipeV2::default();
    recipe.terrain.macro_amplitude_m = RangeV1 { min: 0.0, max: 0.0 };
    recipe.terrain.structure_amplitude_m = RangeV1 { min: 0.0, max: 0.0 };
    recipe.terrain.feature_amplitude_m = RangeV1 { min: 0.0, max: 0.0 };
    let flat = write_collection_v2(
        &tmp.child("flat"),
        CollectionRequestV2::broad(recipe.clone(), 4, 4),
    )
    .unwrap();
    assert_eq!(flat.counts.insufficient_challenge, 4);
    assert!(flat.entries.is_empty());
    assert!(flat.receipts.iter().all(|r| r.proposal.is_some()));
    verify_collection_v2(&tmp.child("flat")).unwrap();
    recipe.travel_distance_m = RangeV1 {
        min: 20.0,
        max: 20.0,
    };
    let exhausted = write_collection_v2(
        &tmp.child("exhausted"),
        CollectionRequestV2::broad(recipe, 4, 4),
    )
    .unwrap();
    assert_eq!(exhausted.counts.structural_exhaustion, 4);
    assert!(
        exhausted
            .receipts
            .iter()
            .all(|r| r.proposal.is_none() && r.structural_rejections.len() == 8)
    );
    verify_collection_v2(&tmp.child("exhausted")).unwrap();
    let mut corrupt = exhausted.clone();
    corrupt.receipts[0].structural_rejections.pop();
    write_replaced(&tmp.child("exhausted"), &corrupt);
    assert!(read_collection_manifest_v2(&tmp.child("exhausted")).is_err());
}

#[test]
fn stress_panels_have_separate_quotas_and_an_overlapping_unique_denominator() {
    let tmp = Temp::new();
    let mut request = CollectionRequestV2::broad(MissionRecipeV2::default(), 66, 20);
    request.population = PopulationDeclarationV2::Stress {
        panels: ["wide", "long"]
            .into_iter()
            .map(|id| StressPanelV1 {
                id: id.into(),
                quota: 2,
                constraints: vec![AxisConstraintV1 {
                    axis: CoverageAxisV1::Distance,
                    range: RangeV1 {
                        min: 300.0,
                        max: 2400.0,
                    },
                }],
            })
            .collect(),
    };
    let m = write_collection_v2(&tmp.child("stress"), request).unwrap();
    assert_eq!(m.entries.len(), 2);
    assert_eq!(
        m.receipts
            .iter()
            .map(|r| r.selected_stress_panels.len())
            .sum::<usize>(),
        4
    );
    assert!(m.counts.stress_excluded > 0);
    assert_eq!(verify_collection_v2(&tmp.child("stress")).unwrap(), m);
    let mut corrupt = m.clone();
    corrupt
        .receipts
        .iter_mut()
        .find(|r| r.disposition == SelectionDispositionV2::Accepted)
        .unwrap()
        .selected_stress_panels
        .pop();
    write_replaced(&tmp.child("stress"), &corrupt);
    assert!(read_collection_manifest_v2(&tmp.child("stress")).is_err());
}

#[test]
fn saved_duplicate_receipts_bind_both_proposals_and_remove_only_the_repeat() {
    let tmp = Temp::new();
    let root = tmp.child("dupes");
    let mut request = CollectionRequestV2::broad(MissionRecipeV2::default(), 981, 1);
    // Two same-direction requests. Fixture substitution tests saved accounting,
    // not a claim that these continuous generator draws naturally collide.
    request.indices = (0..32)
        .step_by(2)
        .filter(|&index| {
            let mission =
                generate_mission_v2(&request.recipe, &request.geometry_spec, request.seed, index)
                    .unwrap()
                    .mission
                    .unwrap();
            ChallengeReceiptV1::measure(
                &request.challenge_rule,
                &mission.raw_geometry,
                &mission.geometry.geometry,
            )
            .qualifies
        })
        .take(2)
        .collect();
    assert_eq!(request.indices.len(), 2);
    let mut m = write_collection_v2(&root, request).unwrap();
    assert_eq!(m.entries.len(), 2);
    let mut duplicate = load_collection_proposal_v2(&root, &m, &m.receipts[0]).unwrap();
    let first_index = m.receipts[0].index;
    let duplicate_index = m.receipts[1].index;
    duplicate.proposal.index = duplicate_index;
    duplicate.scenario.id = "duplicate-test".into();
    duplicate.validate().unwrap();
    let bytes = json_bytes(&duplicate).unwrap();
    let mut r = m.receipts[0].clone();
    r.index = duplicate_index;
    let p = r.proposal.as_mut().unwrap();
    p.index = duplicate_index;
    p.file = m.receipts[1].proposal.as_ref().unwrap().file.clone();
    p.sha256 = sha256(&bytes);
    fs::write(root.join(&p.file), bytes).unwrap();
    r.disposition = SelectionDispositionV2::PhysicalDuplicate { first_index };
    m.receipts[1] = r;
    m.entries.truncate(1);
    m.counts = SelectionCountsV2::from_receipts(&m.receipts);
    write_replaced(&root, &m);
    assert_eq!(verify_collection_v2(&root).unwrap(), m);
    let mut wrong = m.clone();
    wrong.receipts[1].disposition = SelectionDispositionV2::PhysicalDuplicate {
        first_index: duplicate_index,
    };
    write_replaced(&root, &wrong);
    assert!(read_collection_manifest_v2(&root).is_err());
}

#[test]
fn versioned_usage_readers_bind_v2_and_reject_excluded_input_and_path_drift() {
    let tmp = Temp::new();
    let root = tmp.child("collection");
    let m = write_collection_v2(
        &root,
        CollectionRequestV2::broad(MissionRecipeV2::value_noise(), 100, 8),
    )
    .unwrap();
    let view = read_versioned_collection_manifest(&root).unwrap();
    assert_eq!(view.entries(), m.entries);
    assert!(read_collection_manifest(&root).is_err());
    let receipt = record_collection_usage(
        &root,
        "inspect",
        CollectionExposureV1::InputInspection,
        "new development audit",
    )
    .unwrap();
    assert_eq!(read_collection_usage(&root).unwrap(), vec![receipt]);
    assert!(
        record_collection_usage(
            &root,
            "inspect",
            CollectionExposureV1::InputInspection,
            "repeat"
        )
        .is_err()
    );
    verify_versioned_collection(&root).unwrap();
    let mut corrupt = m.clone();
    corrupt.receipts[0].proposal.as_mut().unwrap().file = "../escape.json".into();
    write_replaced(&root, &corrupt);
    assert!(read_versioned_collection_manifest(&root).is_err());
    write_replaced(&root, &m);
    let excluded = m
        .receipts
        .iter()
        .find(|r| r.disposition != SelectionDispositionV2::Accepted)
        .and_then(|r| r.proposal.as_ref());
    if let Some(excluded) = excluded {
        assert!(load_collection_mission_v2(&root, &m, excluded).is_err());
    }
    let p = m.receipts[0].proposal.as_ref().unwrap();
    fs::write(root.join(&p.file), b"{}\n").unwrap();
    assert!(verify_versioned_collection(&root).is_err());
}

#[test]
fn label_and_challenge_projection_drift_is_detected_without_noise_regeneration() {
    let tmp = Temp::new();
    let root = tmp.child("projection");
    let m = write_collection_v2(
        &root,
        CollectionRequestV2::broad(MissionRecipeV2::default(), 981, 32),
    )
    .unwrap();
    let mut changed = m.clone();
    let r = changed
        .receipts
        .iter_mut()
        .find(|r| !r.labels.is_empty())
        .unwrap();
    r.labels.clear();
    write_replaced(&root, &changed);
    assert!(read_collection_manifest_v2(&root).is_ok());
    assert!(verify_collection_v2(&root).is_err());
    let mut changed = m.clone();
    let r = changed
        .receipts
        .iter_mut()
        .find(|r| r.proposal.is_some())
        .unwrap();
    r.challenge.as_mut().unwrap().raw.qualifying_crest_count += 1;
    write_replaced(&root, &changed);
    assert!(read_collection_manifest_v2(&root).is_ok());
    assert!(verify_collection_v2(&root).is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_v2_proposals_and_manifests_are_rejected() {
    let tmp = Temp::new();
    let root = tmp.child("links");
    let m = write_collection_v2(
        &root,
        CollectionRequestV2::broad(MissionRecipeV2::default(), 981, 2),
    )
    .unwrap();
    let entry = m.receipts[0].proposal.as_ref().unwrap();
    let real = tmp.child("real-proposal.json");
    std::fs::rename(root.join(&entry.file), &real).unwrap();
    std::os::unix::fs::symlink(&real, root.join(&entry.file)).unwrap();
    assert!(verify_collection_v2(&root).is_err());
    let real_manifest = tmp.child("real-manifest.json");
    std::fs::rename(root.join("manifest.json"), &real_manifest).unwrap();
    std::os::unix::fs::symlink(&real_manifest, root.join("manifest.json")).unwrap();
    assert!(read_versioned_collection_manifest(&root).is_err());
}
