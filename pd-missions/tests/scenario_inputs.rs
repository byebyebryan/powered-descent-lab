use pd_missions::*;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "pd-native-input-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn inputs() -> Vec<NativeScenarioInputV1> {
    let scenario = serde_json::from_str(include_str!(
        "../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap();
    let a = NativeScenarioInputV1 {
        id: "a".into(),
        scenario,
        labels: BTreeMap::new(),
        parameters: BTreeMap::new(),
        lineage: vec![],
    };
    let mut b = a.clone();
    b.id = "b".into();
    b.scenario.seed += 1;
    b.scenario.name = "alias".into();
    vec![a, b]
}
fn provenance() -> ScenarioInputProvenanceV1 {
    ScenarioInputProvenanceV1 {
        implementation: "test-v1".into(),
        executable_sha256: "0".repeat(64),
        sources: vec![],
    }
}
#[test]
fn native_input_set_preserves_bytes_labels_and_physical_aliases_create_only() {
    let temp = Temp::new();
    let root = temp.0.join("set");
    let inputs = inputs();
    let manifest = write_scenario_inputs(&root, provenance(), &inputs).unwrap();
    assert_eq!(manifest.physical_aliases.len(), 1);
    let (loaded, native) = load_scenario_inputs(&root).unwrap();
    assert_eq!(loaded, manifest);
    assert_eq!(native, inputs);
    assert!(write_scenario_inputs(&root, provenance(), &inputs).is_err());
    assert_eq!(load_scenario_inputs(&root).unwrap().0, manifest);
    std::fs::write(root.join(&manifest.entries[0].file), b"{}").unwrap();
    assert!(
        load_scenario_inputs(&root)
            .unwrap_err()
            .contains("bytes changed")
    );
}
#[test]
fn incomplete_sets_and_rewritten_aliases_are_not_complete_evidence() {
    let temp = Temp::new();
    let root = temp.0.join("set");
    let mut manifest = write_scenario_inputs(&root, provenance(), &inputs()).unwrap();
    std::fs::remove_file(root.join("completion.json")).unwrap();
    assert!(load_scenario_inputs(&root).is_err());
    manifest.physical_aliases.clear();
    std::fs::write(root.join("manifest.json"), json_bytes(&manifest).unwrap()).unwrap();
    let request = std::fs::read(root.join("request.json")).unwrap();
    std::fs::write(root.join("completion.json"), serde_json::to_vec(&serde_json::json!({
        "schema_version":1,"request_sha256":sha256(&request),"manifest_sha256":sha256(&json_bytes(&manifest).unwrap())
    })).unwrap()).unwrap();
    assert!(load_scenario_inputs(&root).unwrap_err().contains("alias"));
}
#[test]
fn invalid_requests_do_not_create_a_destination() {
    let temp = Temp::new();
    let root = temp.0.join("set");
    let mut input = inputs();
    input[1].id = input[0].id.clone();
    assert!(write_scenario_inputs(&root, provenance(), &input).is_err());
    assert!(!root.exists());
    input = inputs();
    input[0].scenario.vehicle.initial_fuel_kg = 0.0;
    assert!(write_scenario_inputs(&root, provenance(), &input).is_err());
    assert!(!root.exists());
}
#[cfg(unix)]
#[test]
fn symlink_ancestors_and_oversized_files_are_rejected() {
    let temp = Temp::new();
    let root = temp.0.join("set");
    write_scenario_inputs(&root, provenance(), &inputs()).unwrap();
    std::os::unix::fs::symlink(&root, temp.0.join("alias")).unwrap();
    assert!(load_scenario_inputs(&temp.0.join("alias")).is_err());
    assert!(read_scenario_input_file(&root.join("manifest.json"), 1).is_err());
}
