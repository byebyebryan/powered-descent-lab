use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposureInventoryV1 {
    pub schema_version: u32,
    pub exposure: String,
    pub physical_ids: Vec<String>,
    pub parent_ids: Vec<String>,
    pub history_complete: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputAuditV1 {
    pub schema_version: u32,
    pub cases: usize,
    pub attempts: usize,
    pub dispositions: BTreeMap<String, usize>,
    pub roles: BTreeMap<String, usize>,
    pub strata: BTreeMap<String, usize>,
    pub ranges: BTreeMap<String, [f64; 2]>,
    pub physical_aliases: BTreeMap<String, Vec<String>>,
    pub parent_groups: BTreeMap<String, Vec<String>>,
    pub joint_counts: BTreeMap<String, BTreeMap<String, usize>>,
    pub joint_cell_totals: BTreeMap<String, usize>,
    pub original_physics_step_bound: u64,
    pub total_pass_physics_step_bound: u64,
    pub physical_exposure_overlap: Vec<String>,
    pub parent_exposure_overlap: Vec<String>,
    pub history_complete: bool,
    pub ready: bool,
    pub reasons: Vec<String>,
}
pub fn audit_plan(plan: &PreparedEvaluationV1) -> Result<InputAuditV1, String> {
    let mut audit = InputAuditV1 {
        schema_version: 1,
        cases: plan.cases.len(),
        attempts: plan.attempts.len(),
        dispositions: BTreeMap::new(),
        roles: BTreeMap::new(),
        strata: BTreeMap::new(),
        ranges: BTreeMap::new(),
        physical_aliases: BTreeMap::new(),
        parent_groups: BTreeMap::new(),
        joint_counts: BTreeMap::new(),
        joint_cell_totals: BTreeMap::new(),
        original_physics_step_bound: 0,
        total_pass_physics_step_bound: 0,
        physical_exposure_overlap: vec![],
        parent_exposure_overlap: vec![],
        history_complete: !plan.recipe.exposure_inventories.is_empty(),
        ready: true,
        reasons: vec![],
    };
    let mut exposed = BTreeSet::new();
    let mut parents = BTreeSet::new();
    for bound in &plan.recipe.exposure_inventories {
        let inventory: ExposureInventoryV1 = read_bound_json(bound, 32 * 1024 * 1024)?;
        if inventory.schema_version != 1 {
            return Err("unknown exposure schema".into());
        }
        audit.history_complete &= inventory.history_complete;
        exposed.extend(inventory.physical_ids);
        parents.extend(inventory.parent_ids);
    }
    for case in &plan.cases {
        let key = match case.disposition {
            InputDispositionV1::Supported => "supported",
            InputDispositionV1::Invalid(_) => "invalid",
            InputDispositionV1::Unsupported(_) => "unsupported",
        };
        *audit.dispositions.entry(key.into()).or_default() += 1;
        *audit.roles.entry(case.role.clone()).or_default() += 1;
        *audit
            .strata
            .entry(format!("{}:{}", case.role, case.stratum))
            .or_default() += 1;
        audit
            .physical_aliases
            .entry(case.physical_sha256.clone())
            .or_default()
            .push(case.id.clone());
        if exposed.contains(&case.physical_sha256) {
            audit.physical_exposure_overlap.push(case.id.clone());
        }
        for parent in &case.lineage {
            audit
                .parent_groups
                .entry(parent.clone())
                .or_default()
                .push(case.id.clone());
            if parents.contains(parent) {
                audit.parent_exposure_overlap.push(case.id.clone());
            }
        }
        if case.role != "diagnostic" && key != "supported" {
            audit.ready = false;
            audit
                .reasons
                .push(format!("unsupported/invalid promised case {}", case.id));
        }
        if case.role != "diagnostic"
            && matches!(&plan.recipe.population, PopulationV1::Local { .. })
            && !case
                .features
                .get("pad_surface_max_error_m")
                .is_some_and(|error| *error <= 1e-9)
        {
            audit.ready = false;
            audit.reasons.push(format!(
                "local target pad and terrain surfaces disagree for {}",
                case.id
            ));
        }
        if let Some(limits) = case.native_limits
            && case.disposition == InputDispositionV1::Supported
        {
            let original = limits
                .hard_end_physics_step
                .checked_mul(plan.recipe.programs.len() as u64)
                .ok_or("step bound overflow")?;
            audit.original_physics_step_bound = audit
                .original_physics_step_bound
                .checked_add(original)
                .ok_or("step bound overflow")?;
        }
        for (name, value) in &case.features {
            let range = audit.ranges.entry(name.clone()).or_insert([*value, *value]);
            range[0] = range[0].min(*value);
            range[1] = range[1].max(*value);
        }
        for group in &plan.recipe.joint_groups {
            let mut cell = vec![];
            for axis in group {
                let Some(value) = case.features.get(axis) else {
                    cell.push("missing".into());
                    continue;
                };
                cell.push(
                    plan.recipe.audit_bins[axis]
                        .partition_point(|edge| value >= edge)
                        .to_string(),
                );
            }
            *audit
                .joint_counts
                .entry(format!("{}:{}", case.role, group.join("/")))
                .or_default()
                .entry(cell.join(","))
                .or_default() += 1;
        }
    }
    for group in &plan.recipe.joint_groups {
        audit.joint_cell_totals.insert(
            group.join("/"),
            group
                .iter()
                .map(|name| plan.recipe.audit_bins[name].len() + 1)
                .product(),
        );
    }
    audit.total_pass_physics_step_bound = audit
        .original_physics_step_bound
        .checked_mul(if plan.recipe.direct_parity { 9 } else { 8 })
        .ok_or("proof step bound overflow")?;
    if matches!(
        plan.recipe.population,
        PopulationV1::Local {
            independent: true,
            ..
        }
    ) && (audit.physical_aliases.values().any(|v| v.len() != 1)
        || !audit.physical_exposure_overlap.is_empty()
        || !audit.parent_exposure_overlap.is_empty())
    {
        audit.ready = false;
        audit
            .reasons
            .push("confirmation aliases or audited exposure overlap".into());
    }
    Ok(audit)
}
