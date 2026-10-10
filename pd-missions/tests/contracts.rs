use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use pd_missions::*;

fn temporary_parent() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "pd-missions-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    root
}

fn request(role: CollectionRoleV1) -> CollectionRequestV1 {
    CollectionRequestV1 {
        schema_version: 1,
        role,
        recipe: MissionRecipeV1::default(),
        seed: 987,
        indices: (0..8).collect(),
    }
}

#[test]
fn deterministic_order_independent_generation_and_declared_strata() {
    let recipe = MissionRecipeV1::default();
    let first: Vec<_> = (0..32)
        .map(|index| generate_mission(&recipe, 987, index).unwrap())
        .collect();
    for index in (0..32).rev() {
        let result = generate_mission(&recipe, 987, index).unwrap();
        assert_eq!(result, first[index as usize]);
        let mission = result.mission.unwrap();
        mission.validate().unwrap();
        assert_eq!(mission.terrain_stratum, (index % 4) as u32);
        assert_eq!(mission.direction, if index / 4 % 2 == 0 { 1 } else { -1 });
        let pads = &mission.scenario.world.landing_pads;
        assert_ne!(pads[0].width_m, pads[1].width_m);
        assert_eq!(mission.scenario.mission.transfer_route, None);
        assert_eq!(
            mission.scenario.initial_state.position_m.y,
            pads[0].surface_y_m + 5.0
        );
    }
    let mut invalid = recipe.clone();
    invalid.schema_version = 2;
    assert!(generate_mission(&invalid, 1, 1).is_err());
    invalid = recipe;
    invalid.source_x_m.min = f64::NAN;
    assert!(generate_mission(&invalid, 1, 1).is_err());
}

#[test]
fn finite_structural_rejection_retains_every_attempt() {
    let recipe = MissionRecipeV1 {
        travel_distance_m: RangeV1 { min: 1.0, max: 2.0 },
        ..MissionRecipeV1::default()
    };
    let result = generate_mission(&recipe, 987, 0).unwrap();
    assert!(result.mission.is_none());
    assert_eq!(result.rejections.len(), recipe.max_attempts as usize);
    for (index, rejection) in result.rejections.iter().enumerate() {
        assert_eq!(rejection.proposal.attempt as usize, index);
        assert_eq!(rejection.reason, StructuralRejectionV1::OverlappingPads);
    }
}

#[test]
fn create_only_collections_verify_bytes_and_record_heldout_exposure() {
    let parent = temporary_parent();
    let root = parent.join("collection");
    let manifest = write_collection(&root, request(CollectionRoleV1::HeldOut)).unwrap();
    let original = std::fs::read(root.join("manifest.json")).unwrap();
    assert_eq!(manifest.entries.len(), 8);
    assert_eq!(verify_collection(&root).unwrap(), manifest);
    assert!(write_collection(&root, request(CollectionRoleV1::HeldOut)).is_err());
    let receipt = record_collection_usage(
        &root,
        "first-exposure",
        CollectionExposureV1::Tuning,
        "consumed development inputs",
    )
    .unwrap();
    assert_eq!(read_collection_usage(&root).unwrap(), vec![receipt]);
    assert_eq!(std::fs::read(root.join("manifest.json")).unwrap(), original);
    assert!(
        record_collection_usage(
            &root,
            "first-exposure",
            CollectionExposureV1::Tuning,
            "repeat"
        )
        .is_err()
    );
    assert!(
        record_collection_usage(
            &root,
            "../../escape",
            CollectionExposureV1::Training,
            "bad path"
        )
        .is_err()
    );
    let file = root.join(&manifest.entries[0].file);
    let mut changed = std::fs::read(&file).unwrap();
    changed.push(b' ');
    std::fs::write(&file, changed).unwrap();
    assert!(
        verify_collection(&root)
            .unwrap_err()
            .contains("bytes changed")
    );
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn exhausted_collection_and_corrupt_accounting_are_distinct_from_empty_success() {
    let parent = temporary_parent();
    let root = parent.join("collection");
    let mut req = request(CollectionRoleV1::Development);
    req.recipe.travel_distance_m = RangeV1 { min: 1.0, max: 2.0 };
    let mut manifest = write_collection(&root, req).unwrap();
    assert!(manifest.entries.is_empty());
    assert_eq!(manifest.exhausted_indices.len(), 8);
    assert_eq!(verify_collection(&root).unwrap(), manifest);
    manifest.rejections.pop();
    std::fs::write(root.join("manifest.json"), json_bytes(&manifest).unwrap()).unwrap();
    assert!(read_collection_manifest(&root).is_err());
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn physical_identity_ignores_membership_but_binds_physics_and_mission() {
    let mission = generate_mission(&MissionRecipeV1::default(), 987, 0)
        .unwrap()
        .mission
        .unwrap();
    let mut scenario = mission.scenario.clone();
    scenario.id = "another membership".to_owned();
    scenario.seed = 123;
    scenario.metadata.clear();
    scenario.tags.clear();
    assert_eq!(
        physical_identity(&scenario).unwrap(),
        mission.physical_sha256
    );
    scenario.vehicle.initial_fuel_kg -= 1.0;
    assert_ne!(
        physical_identity(&scenario).unwrap(),
        mission.physical_sha256
    );
}

#[test]
fn readers_reject_traversal_and_symlinked_mission_files() {
    let parent = temporary_parent();
    let root = parent.join("collection");
    let mut manifest = write_collection(&root, request(CollectionRoleV1::Development)).unwrap();
    let entry = manifest.entries[0].clone();
    manifest.entries[0].file = "../outside.json".to_owned();
    std::fs::write(root.join("manifest.json"), json_bytes(&manifest).unwrap()).unwrap();
    assert!(read_collection_manifest(&root).is_err());
    manifest.entries[0] = entry.clone();
    std::fs::write(root.join("manifest.json"), json_bytes(&manifest).unwrap()).unwrap();
    #[cfg(unix)]
    {
        let actual = root.join(&entry.file);
        let moved = root.join("actual.json");
        std::fs::rename(&actual, &moved).unwrap();
        std::os::unix::fs::symlink(&moved, actual).unwrap();
        assert!(verify_collection(&root).is_err());
    }
    std::fs::remove_dir_all(parent).unwrap();
}
