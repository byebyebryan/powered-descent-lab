use pd_missions::*;

#[test]
fn explicit_dispatch_retains_v1_generation_bytes_and_rejects_cross_schema_fields() {
    let recipe = MissionRecipeV1::default();
    let decoded = VersionedMissionRecipe::from_json(&json_bytes(&recipe).unwrap()).unwrap();
    assert_eq!(json_bytes(&decoded).unwrap(), json_bytes(&recipe).unwrap());
    let expected = generate_mission(&recipe, 987, 7).unwrap();
    let actual = generate_versioned_mission(&decoded, 987, 7, None).unwrap();
    assert_eq!(json_bytes(&expected).unwrap(), json_bytes(&actual).unwrap());
    let mission = expected.mission.unwrap();
    let read = VersionedResolvedMission::from_json(&json_bytes(&mission).unwrap()).unwrap();
    assert_eq!(read.scenario(), &mission.scenario);
    assert_eq!(json_bytes(&read).unwrap(), json_bytes(&mission).unwrap());
    assert!(
        generate_versioned_mission(&decoded, 987, 7, Some(&GeometrySpecV1::default())).is_err()
    );
    let mut changed = serde_json::to_value(recipe).unwrap();
    changed["schema_version"] = serde_json::json!(2);
    assert!(VersionedMissionRecipe::from_json(&serde_json::to_vec(&changed).unwrap()).is_err());
    changed["schema_version"] = serde_json::json!(1);
    changed["pad_transition_m"] = serde_json::json!(24.0);
    assert!(VersionedMissionRecipe::from_json(&serde_json::to_vec(&changed).unwrap()).is_err());
}

#[test]
fn v2_dispatch_retains_shape_and_validates_resolved_physical_inputs() {
    let recipe = MissionRecipeV2::default();
    let decoded = VersionedMissionRecipe::from_json(&json_bytes(&recipe).unwrap()).unwrap();
    let expected = generate_mission_v2(&recipe, &GeometrySpecV1::default(), 991, 0).unwrap();
    let actual = generate_versioned_mission(&decoded, 991, 0, None).unwrap();
    assert_eq!(json_bytes(&actual).unwrap(), json_bytes(&expected).unwrap());
    let mission = expected.mission.unwrap();
    let read = VersionedResolvedMission::from_json(&json_bytes(&mission).unwrap()).unwrap();
    assert_eq!(read.scenario(), &mission.scenario);
    assert_eq!(json_bytes(&read).unwrap(), json_bytes(&mission).unwrap());
    for version in [0, 3] {
        let mut changed = serde_json::to_value(&recipe).unwrap();
        changed["schema_version"] = serde_json::json!(version);
        assert!(VersionedMissionRecipe::from_json(&serde_json::to_vec(&changed).unwrap()).is_err());
    }
    assert!(VersionedMissionRecipe::from_json(b"{}").is_err());
    let mut changed = mission;
    changed.geometry.geometry.relief_m += 1.0;
    assert!(VersionedResolvedMission::from_json(&json_bytes(&changed).unwrap()).is_err());
}

#[test]
fn family_frontdoor_keeps_old_aliases_and_explicit_new_versions() {
    assert_eq!(
        VersionedMissionRecipe::family("layered").unwrap(),
        VersionedMissionRecipe::V1(MissionRecipeV1::layered())
    );
    assert_eq!(
        VersionedMissionRecipe::family("value-noise").unwrap(),
        VersionedMissionRecipe::V1(MissionRecipeV1::default())
    );
    assert_eq!(
        VersionedMissionRecipe::family("layered-v2").unwrap(),
        VersionedMissionRecipe::V2(MissionRecipeV2::default())
    );
    assert!(VersionedMissionRecipe::family("mountain").is_err());
}
