use super::*;
use pd_missions::{
    NativeScenarioInputV1, ScenarioInputProvenanceV1, physical_identity, write_json_create_only,
    write_scenario_inputs,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
pub(super) struct Temp(pub PathBuf);
impl Temp {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "pd-characterization-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
pub(super) fn anchor() -> pd_core::ScenarioSpec {
    serde_json::from_str(include_str!(
        "../../../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap()
}
fn interval(min: f64, max: f64) -> IntervalV1 {
    IntervalV1 { min, max }
}
pub(super) fn recipe() -> EvaluationRecipeV1 {
    EvaluationRecipeV1 {
        schema_version: 1,
        task_id: "software-characterization".into(),
        population: PopulationV1::Local {
            anchor_id: "anchor".into(),
            cohort: "development".into(),
            seed: 1234,
            indices: (0..20).collect(),
            independent: false,
            bounds: Box::new(LocalBoundsV1 {
                clearance_m: interval(5.0, 20.0),
                offset_per_clearance: interval(0.0, 0.3),
                lateral_residual_mps: interval(-2.0, 2.0),
                braking_speed_ratio: interval(0.1, 0.7),
                attitude_rad: interval(-0.2, 0.2),
                angular_rate_radps: interval(-0.1, 0.1),
                dry_mass_kg: vec![7200.0, 9450.0, 11700.0],
                fuel_kg: interval(1000.0, 6300.0),
                pad_width_m: interval(24.0, 50.0),
                pad_surface_y_m: interval(0.0, 10.0),
                deadline_s: interval(3.0 / 120.0, 3.0 / 120.0),
                terrain: vec![TerrainProfileV1::Flat {}],
            }),
            transforms: vec![
                TransformV1::Identity {},
                TransformV1::Reflect {},
                TransformV1::Translate {
                    x_m: 128.0,
                    y_m: 8.0,
                },
                TransformV1::Relabel { seed_delta: 9 },
            ],
            diagnostics: vec![DiagnosticV1::UnsupportedGravity { mps2: 1.62 }],
        },
        programs: vec![crate::ActivationProgramV1 {
            schema_version: 1,
            activation: crate::ActivationV1::Immediate {},
        }],
        direct_parity: false,
        budget: EvaluationBudgetV1 {
            max_wall_seconds: 60,
            max_attempt_wall_seconds: 20,
            max_output_bytes: 128 * 1024 * 1024,
            max_bundle_bytes: 16 * 1024 * 1024,
            min_free_bytes: 0,
        },
        exposure_inventories: vec![],
        audit_bins: BTreeMap::from([
            ("touchdown_clearance_m".into(), vec![1.0, 5.0, 20.0]),
            ("vy_mps".into(), vec![-20.0, -5.0, 0.0]),
            ("fuel_kg".into(), vec![100.0, 2000.0, 5000.0]),
        ]),
        joint_groups: vec![vec![
            "touchdown_clearance_m".into(),
            "vy_mps".into(),
            "fuel_kg".into(),
        ]],
    }
}
pub(super) fn setup(temp: &Temp, recipe: &EvaluationRecipeV1) -> (PathBuf, PathBuf) {
    let input = temp.0.join("inputs");
    write_scenario_inputs(
        &input,
        ScenarioInputProvenanceV1 {
            implementation: "software-test".into(),
            executable_sha256: "0".repeat(64),
            sources: vec![],
        },
        &[NativeScenarioInputV1 {
            id: "anchor".into(),
            scenario: anchor(),
            labels: BTreeMap::new(),
            parameters: BTreeMap::new(),
            lineage: vec!["software-fixture".into()],
        }],
    )
    .unwrap();
    let path = temp.0.join("recipe.json");
    write_json_create_only(&path, recipe).unwrap();
    (input, path)
}

#[test]
fn preparation_has_complete_roles_aliases_native_budgets_and_no_execution() {
    let temp = Temp::new();
    let recipe = recipe();
    let (input, path) = setup(&temp, &recipe);
    let root = temp.0.join("plan");
    let plan = prepare_evaluation(&path, &[input], &root).unwrap();
    let audit = check_evaluation(&root).unwrap();
    assert_eq!(plan.cases.len(), 20);
    assert_eq!(audit.roles["development"], 16);
    assert_eq!(audit.roles["diagnostic"], 4);
    assert_eq!(audit.dispositions["unsupported"], 4);
    assert!(audit.ready);
    assert_eq!(audit.parent_groups.len(), 5);
    assert!(audit.physical_aliases.values().any(|v| v.len() == 2));
    assert_eq!(audit.original_physics_step_bound, 16 * 3);
    assert_eq!(audit.total_pass_physics_step_bound, 16 * 3 * 8);
    assert!(!root.join("capture").exists());
    assert!(!root.join("runs").exists());
    assert!(prepare_evaluation(&path, &[temp.0.join("inputs")], &root).is_err());
    let case = root.join(&plan.cases[0].scenario_file);
    let mut bytes = std::fs::read(&case).unwrap();
    bytes.push(b' ');
    std::fs::write(case, bytes).unwrap();
    assert!(check_evaluation(&root).unwrap_err().contains("bytes"));
}
#[test]
fn independent_streams_are_order_independent_disjoint_and_exposure_checked() {
    let mut r = recipe();
    let PopulationV1::Local {
        independent,
        diagnostics,
        transforms,
        indices,
        ..
    } = &mut r.population
    else {
        unreachable!()
    };
    *independent = true;
    diagnostics.clear();
    *transforms = vec![TransformV1::Identity {}];
    *indices = (0..32).collect();
    r.validate().unwrap();
    let a = anchor();
    let first = local_case(&r, &a, 0).unwrap();
    for i in (0..32).rev() {
        assert_eq!(
            local_case(&r, &a, i).unwrap().scenario,
            local_case(&r, &a, i).unwrap().scenario
        );
    }
    let mut other = r.clone();
    let PopulationV1::Local { cohort, .. } = &mut other.population else {
        unreachable!()
    };
    *cohort = "fresh-other-stream".into();
    assert_ne!(
        physical_identity(&first.scenario).unwrap(),
        physical_identity(&local_case(&other, &a, 0).unwrap().scenario).unwrap()
    );
    let temp = Temp::new();
    let inventory = temp.0.join("exposure.json");
    write_json_create_only(
        &inventory,
        &ExposureInventoryV1 {
            schema_version: 1,
            exposure: "observed".into(),
            physical_ids: vec![physical_identity(&first.scenario).unwrap()],
            parent_ids: vec![first.parent],
            history_complete: false,
        },
    )
    .unwrap();
    r.exposure_inventories = vec![bind_file(&inventory, 1024 * 1024).unwrap()];
    let (input, path) = setup(&temp, &r);
    let root = temp.0.join("plan");
    prepare_evaluation(&path, &[input], &root).unwrap();
    let audit = check_evaluation(&root).unwrap();
    assert!(!audit.ready);
    assert_eq!(audit.physical_exposure_overlap.len(), 1);
    assert!(!audit.history_complete);
}
#[test]
fn transforms_preserve_parent_and_labels_cannot_change_physical_inputs() {
    let r = recipe();
    let a = anchor();
    let original = local_case(&r, &a, 0).unwrap();
    let label = local_case(&r, &a, 3).unwrap();
    assert_eq!(original.parent, label.parent);
    assert_eq!(
        physical_identity(&original.scenario).unwrap(),
        physical_identity(&label.scenario).unwrap()
    );
    let reflected = local_case(&r, &a, 1).unwrap();
    assert_eq!(
        reflected.scenario.initial_state.velocity_mps.x,
        -original.scenario.initial_state.velocity_mps.x
    );
    assert_eq!(
        reflected.scenario.initial_state.attitude_rad,
        -original.scenario.initial_state.attitude_rad
    );
    let translated = local_case(&r, &a, 2).unwrap();
    assert!(
        (translated.scenario.initial_state.position_m.x
            - original.scenario.initial_state.position_m.x
            - 128.0)
            .abs()
            < 1e-9
    );
    assert_eq!(
        translated.scenario.vehicle.initial_fuel_kg,
        original.scenario.vehicle.initial_fuel_kg
    );
    assert_eq!(translated.scenario.sim, original.scenario.sim);
}
#[test]
fn invalid_diagnostic_is_retained_and_never_assigned_a_native_budget() {
    let temp = Temp::new();
    let mut r = recipe();
    let PopulationV1::Local { diagnostics, .. } = &mut r.population else {
        unreachable!()
    };
    *diagnostics = vec![DiagnosticV1::Fuel { kg: 0.0 }];
    let (input, path) = setup(&temp, &r);
    let root = temp.0.join("plan");
    let plan = prepare_evaluation(&path, &[input], &root).unwrap();
    assert_eq!(check_evaluation(&root).unwrap().dispositions["invalid"], 4);
    assert!(
        plan.cases
            .iter()
            .filter(|c| matches!(c.disposition, InputDispositionV1::Invalid(_)))
            .all(|c| c.native_limits.is_none())
    );
}
