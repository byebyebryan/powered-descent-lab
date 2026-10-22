use super::*;
use pd_control::maneuver::{
    ManeuverController, ManeuverDescriptorV1, ManeuverInputErrorV1, TerminalLandingController,
};
use pd_core::{BoundedRunLimitsV1, RunContext, ScenarioSpec};
use pd_missions::{
    NativeScenarioInputV1, json_bytes, load_scenario_inputs, physical_identity,
    read_scenario_input_file, sha256, write_json_create_only,
};
use pd_run::scenario_horizon_limits;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "reason",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum InputDispositionV1 {
    Supported,
    Invalid(String),
    Unsupported(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCaseV1 {
    pub id: String,
    pub role: String,
    pub stratum: String,
    pub scenario_file: String,
    pub scenario_sha256: String,
    pub physical_sha256: String,
    pub disposition: InputDispositionV1,
    pub native_limits: Option<BoundedRunLimitsV1>,
    pub features: BTreeMap<String, f64>,
    pub lineage: Vec<String>,
    pub source_labels: BTreeMap<String, String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedAttemptV1 {
    pub id: String,
    pub case_index: usize,
    pub program_index: usize,
    pub program_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedEvaluationV1 {
    pub schema_version: u32,
    pub recipe: EvaluationRecipeV1,
    pub recipe_file: BoundFileV1,
    pub input_files: Vec<BoundFileV1>,
    pub controller: Option<ManeuverDescriptorV1>,
    pub cases: Vec<PreparedCaseV1>,
    pub attempts: Vec<PreparedAttemptV1>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanCompletionV1 {
    schema_version: u32,
    plan_sha256: String,
    audit_sha256: String,
}

pub fn case_disposition(s: &ScenarioSpec) -> (InputDispositionV1, Option<BoundedRunLimitsV1>) {
    let controller = TerminalLandingController::new(s);
    let disposition = match controller {
        Ok(_) => InputDispositionV1::Supported,
        Err(ManeuverInputErrorV1::InvalidInput(reason)) => {
            return (InputDispositionV1::Invalid(reason), None);
        }
        Err(ManeuverInputErrorV1::UnsupportedInput(reason)) => {
            InputDispositionV1::Unsupported(reason)
        }
    };
    let limits = RunContext::from_scenario(s)
        .and_then(|ctx| scenario_horizon_limits(&ctx).map_err(|e| e.to_string()));
    match limits {
        Ok(limits) if limits.hard_end_physics_step <= crate::MAX_PHYSICS_STEPS => {
            (disposition, Some(limits))
        }
        Ok(_) => (
            InputDispositionV1::Unsupported("single episode exceeds 600 s".into()),
            None,
        ),
        Err(reason) => (InputDispositionV1::Invalid(reason), None),
    }
}

pub fn attempt_identity(
    plan: &PreparedEvaluationV1,
    case: &PreparedCaseV1,
    program_index: usize,
) -> Result<String, String> {
    identity(&(
        plan.schema_version,
        &plan.recipe,
        &plan.recipe_file,
        &plan.input_files,
        &plan.controller,
        &case.id,
        &case.physical_sha256,
        &case.scenario_sha256,
        &case.native_limits,
        &plan.recipe.programs[program_index],
    ))
}

pub fn program_file(index: usize) -> String {
    format!("program-{index:02}.json")
}

struct Proposal {
    id: String,
    scenario: ScenarioSpec,
    role: String,
    stratum: String,
    lineage: Vec<String>,
    source_labels: BTreeMap<String, String>,
}

pub fn prepare_evaluation(
    recipe_path: &Path,
    input_roots: &[PathBuf],
    output: &Path,
) -> Result<PreparedEvaluationV1, String> {
    let recipe_file = bind_file(recipe_path, 4 * 1024 * 1024)?;
    let recipe: EvaluationRecipeV1 = read_bound_json(&recipe_file, 4 * 1024 * 1024)?;
    recipe.validate()?;
    if input_roots.is_empty() || input_roots.len() > 8 {
        return Err("need 1..8 native input sets".into());
    }
    let mut input_files = vec![];
    let mut inputs = vec![];
    for root in input_roots {
        let mut bound = vec![];
        for name in ["request.json", "manifest.json", "completion.json"] {
            bound.push(bind_file(&root.join(name), 32 * 1024 * 1024)?);
        }
        let (_, set) = load_scenario_inputs(root)?;
        for file in &bound {
            if bind_file(Path::new(&file.path), 32 * 1024 * 1024)? != *file {
                return Err("input manifest changed while loading".into());
            }
        }
        input_files.extend(bound);
        inputs.extend(set);
    }
    input_files.sort_by(|a, b| a.path.cmp(&b.path));
    if input_files.windows(2).any(|w| w[0].path == w[1].path) {
        return Err("duplicate input sets".into());
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    pd_missions::check_input_path(parent)?;
    let output = fs::canonicalize(parent)
        .map_err(|e| e.to_string())?
        .join(output.file_name().ok_or("plan needs filename")?);
    for file in input_files.iter().chain(std::iter::once(&recipe_file)) {
        let input = Path::new(&file.path);
        if output.starts_with(input) || input.starts_with(&output) {
            return Err("plan output overlaps input".into());
        }
    }
    let mut proposals = vec![];
    match &recipe.population {
        PopulationV1::Retained { seeds } => {
            for NativeScenarioInputV1 {
                id,
                scenario,
                labels,
                lineage,
                ..
            } in inputs
            {
                if !seeds.is_empty() && !seeds.contains(&scenario.seed) {
                    continue;
                }
                let pack = labels
                    .get("source_pack")
                    .cloned()
                    .unwrap_or_else(|| "native".into());
                let stratum = format!(
                    "{}:{}",
                    labels
                        .get("condition_set")
                        .map(String::as_str)
                        .unwrap_or("unknown"),
                    labels
                        .get("vehicle_variant")
                        .map(String::as_str)
                        .unwrap_or("unknown")
                );
                proposals.push(Proposal {
                    id: format!("{pack}:{id}"),
                    scenario,
                    role: "retained".into(),
                    stratum,
                    lineage,
                    source_labels: labels,
                });
            }
        }
        PopulationV1::Local {
            anchor_id, indices, ..
        } => {
            let anchors = inputs
                .iter()
                .filter(|i| &i.id == anchor_id)
                .collect::<Vec<_>>();
            if anchors.len() != 1 {
                return Err("local anchor must identify exactly one original occurrence".into());
            }
            let anchor = anchors[0];
            for &index in indices {
                let local = local_case(&recipe, &anchor.scenario, index)?;
                let mut labels = BTreeMap::from([
                    ("sampler".into(), "local-joint-sampler-v1".into()),
                    ("transform".into(), identity(&local.transform)?),
                ]);
                labels.insert(
                    "anchor_physical_sha256".into(),
                    physical_identity(&anchor.scenario).map_err(|e| e.to_string())?,
                );
                proposals.push(Proposal {
                    id: local.scenario.id.clone(),
                    scenario: local.scenario,
                    role: local.role,
                    stratum: local.stratum,
                    lineage: vec![local.parent],
                    source_labels: labels,
                });
            }
        }
    }
    if proposals.is_empty() || proposals.len() > pd_missions::MAX_SCENARIO_INPUTS {
        return Err("empty or oversized plan".into());
    }
    fs::create_dir(&output).map_err(|e| format!("reserve new plan: {e}"))?;
    write_json_create_only(&output.join("recipe.json"), &recipe)?;
    let mut plan = PreparedEvaluationV1 {
        schema_version: 1,
        recipe,
        recipe_file,
        input_files,
        controller: None,
        cases: vec![],
        attempts: vec![],
    };
    let mut ids = BTreeSet::new();
    for (
        index,
        Proposal {
            id,
            scenario,
            role,
            stratum,
            lineage,
            source_labels,
        },
    ) in proposals.into_iter().enumerate()
    {
        if !ids.insert(id.clone()) {
            return Err("duplicate case occurrence identity".into());
        }
        let (disposition, native_limits) = case_disposition(&scenario);
        if disposition == InputDispositionV1::Supported {
            let descriptor = TerminalLandingController::new(&scenario)
                .map_err(|e| format!("{e:?}"))?
                .descriptor();
            if let Some(expected) = &plan.controller {
                if *expected != descriptor {
                    return Err("mixed controller capability".into());
                }
            } else {
                plan.controller = Some(descriptor);
            }
        }
        let scenario_file = format!("scenario-{index:06}.json");
        let bytes = json_bytes(&scenario)?;
        if bytes.len() as u64 > pd_missions::MAX_SCENARIO_INPUT_BYTES {
            return Err("scenario too large".into());
        }
        write_json_create_only(&output.join(&scenario_file), &scenario)?;
        plan.cases.push(PreparedCaseV1 {
            id,
            role,
            stratum,
            scenario_file,
            scenario_sha256: sha256(&bytes),
            physical_sha256: physical_identity(&scenario).map_err(|e| e.to_string())?,
            disposition,
            native_limits,
            features: physical_features(&scenario).unwrap_or_default(),
            lineage,
            source_labels,
        });
    }
    for (index, program) in plan.recipe.programs.iter().enumerate() {
        write_json_create_only(&output.join(program_file(index)), program)?;
    }
    for (case_index, case) in plan.cases.iter().enumerate() {
        for program_index in 0..plan.recipe.programs.len() {
            plan.attempts.push(PreparedAttemptV1 {
                id: attempt_identity(&plan, case, program_index)?,
                case_index,
                program_index,
                program_sha256: sha256(&json_bytes(&plan.recipe.programs[program_index])?),
            });
        }
    }
    let audit = audit_plan(&plan)?;
    write_json_create_only(&output.join("plan.json"), &plan)?;
    write_json_create_only(&output.join("audit.json"), &audit)?;
    write_json_create_only(
        &output.join("completion.json"),
        &PlanCompletionV1 {
            schema_version: 1,
            plan_sha256: sha256(&json_bytes(&plan)?),
            audit_sha256: sha256(&json_bytes(&audit)?),
        },
    )?;
    check_evaluation(&output)?;
    Ok(plan)
}

pub fn load_plan(root: &Path) -> Result<PreparedEvaluationV1, String> {
    let bytes = read_scenario_input_file(&root.join("plan.json"), 32 * 1024 * 1024)?;
    let completion: PlanCompletionV1 = read_json(&root.join("completion.json"), 4096)?;
    if completion.schema_version != 1 || completion.plan_sha256 != sha256(&bytes) {
        return Err("plan completion/hash mismatch".into());
    }
    let plan: PreparedEvaluationV1 = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    plan.recipe.validate()?;
    if plan.schema_version != 1
        || plan.cases.is_empty()
        || plan.cases.len() > pd_missions::MAX_SCENARIO_INPUTS
        || plan.attempts.len() != plan.cases.len() * plan.recipe.programs.len()
    {
        return Err("invalid plan schema/schedule".into());
    }
    let recipe: EvaluationRecipeV1 = read_json(&root.join("recipe.json"), 4 * 1024 * 1024)?;
    if recipe != plan.recipe {
        return Err("plan recipe differs".into());
    }
    let audit_bytes = read_scenario_input_file(&root.join("audit.json"), 32 * 1024 * 1024)?;
    if sha256(&audit_bytes) != completion.audit_sha256
        || read_json::<InputAuditV1>(&root.join("audit.json"), 32 * 1024 * 1024)?
            != audit_plan(&plan)?
    {
        return Err("plan audit differs".into());
    }
    Ok(plan)
}

pub fn check_evaluation(root: &Path) -> Result<InputAuditV1, String> {
    let plan = load_plan(root)?;
    for bound in plan
        .input_files
        .iter()
        .chain(std::iter::once(&plan.recipe_file))
        .chain(&plan.recipe.exposure_inventories)
    {
        if bind_file(Path::new(&bound.path), 32 * 1024 * 1024)? != *bound {
            return Err("bound plan input changed".into());
        }
    }
    let mut ids = BTreeSet::new();
    for (index, case) in plan.cases.iter().enumerate() {
        if case.scenario_file != format!("scenario-{index:06}.json") || !ids.insert(&case.id) {
            return Err("case path/order/identity mismatch".into());
        }
        let scenario = load_plan_case(root, &plan, index)?;
        if case_disposition(&scenario) != (case.disposition.clone(), case.native_limits)
            || physical_features(&scenario).unwrap_or_default() != case.features
        {
            return Err("case support, native limits or features differ".into());
        }
    }
    let mut attempts = BTreeSet::new();
    for (index, attempt) in plan.attempts.iter().enumerate() {
        if attempt.case_index != index / plan.recipe.programs.len()
            || attempt.program_index != index % plan.recipe.programs.len()
            || !attempts.insert(&attempt.id)
            || attempt.id
                != attempt_identity(
                    &plan,
                    &plan.cases[attempt.case_index],
                    attempt.program_index,
                )?
            || attempt.program_sha256
                != sha256(&read_scenario_input_file(
                    &root.join(program_file(attempt.program_index)),
                    64 * 1024,
                )?)
        {
            return Err("attempt schedule/program identity differs".into());
        }
    }
    audit_plan(&plan)
}

pub fn load_plan_case(
    root: &Path,
    plan: &PreparedEvaluationV1,
    index: usize,
) -> Result<ScenarioSpec, String> {
    let case = plan.cases.get(index).ok_or("case out of range")?;
    if case.scenario_file != format!("scenario-{index:06}.json") {
        return Err("invalid scenario filename".into());
    }
    let bytes = read_scenario_input_file(
        &root.join(&case.scenario_file),
        pd_missions::MAX_SCENARIO_INPUT_BYTES,
    )?;
    let scenario: ScenarioSpec = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if sha256(&bytes) != case.scenario_sha256
        || physical_identity(&scenario).map_err(|e| e.to_string())? != case.physical_sha256
    {
        return Err("prepared scenario bytes/physical identity changed".into());
    }
    Ok(scenario)
}
