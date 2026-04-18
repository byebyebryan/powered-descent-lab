use std::{
    collections::BTreeMap,
    fs,
    path::Path,
};

use anyhow::{Context, Result, bail};
use pd_control::{
    ControllerSpec, ControlledRunArtifacts, built_in_controller_spec, run_controller_spec,
};
use pd_core::{MissionOutcome, RunContext, RunManifest, ScenarioSpec};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioPackSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub entries: Vec<ScenarioPackEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioPackEntry {
    pub id: String,
    pub scenario: String,
    pub controller: String,
    #[serde(default)]
    pub controller_config: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchRunRecord {
    pub entry_id: String,
    pub scenario_path: String,
    pub controller_spec: ControllerSpec,
    pub manifest: RunManifest,
    pub bundle_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchSummary {
    pub total_runs: usize,
    pub success_runs: usize,
    pub mean_sim_time_s: f64,
    pub mission_outcomes: BTreeMap<String, usize>,
    pub end_reasons: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchReport {
    pub pack_id: String,
    pub pack_name: String,
    pub total_runs: usize,
    pub records: Vec<BatchRunRecord>,
    pub summary: BatchSummary,
}

pub fn load_pack(path: &Path) -> Result<ScenarioPackSpec> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario pack file {}", path.display()))?;
    let pack: ScenarioPackSpec = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse scenario pack json {}", path.display()))?;
    validate_pack(&pack)?;
    Ok(pack)
}

pub fn run_pack_file(path: &Path, output_dir: Option<&Path>) -> Result<BatchReport> {
    let pack = load_pack(path)?;
    let base_dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("pack path has no parent directory"))?;
    run_pack(&pack, base_dir, output_dir)
}

pub fn run_pack(
    pack: &ScenarioPackSpec,
    base_dir: &Path,
    output_dir: Option<&Path>,
) -> Result<BatchReport> {
    validate_pack(pack)?;

    let mut records = Vec::with_capacity(pack.entries.len());
    for entry in &pack.entries {
        let scenario_path = base_dir.join(&entry.scenario);
        let scenario = load_scenario(&scenario_path)?;
        let controller_spec = load_controller_spec(base_dir, entry)?;
        let ctx = RunContext::from_scenario(&scenario)
            .map_err(anyhow::Error::msg)
            .with_context(|| format!("failed to build run context for entry {}", entry.id))?;
        let artifacts = run_controller_spec(&ctx, &controller_spec)
            .with_context(|| format!("failed to run controller for entry {}", entry.id))?;

        let bundle_dir = output_dir.map(|root| root.join("runs").join(&entry.id));
        if let Some(bundle_dir) = bundle_dir.as_deref() {
            write_artifact_bundle(bundle_dir, &scenario, &controller_spec, &artifacts)?;
        }

        records.push(BatchRunRecord {
            entry_id: entry.id.clone(),
            scenario_path: entry.scenario.clone(),
            controller_spec: controller_spec.clone(),
            manifest: artifacts.run.manifest,
            bundle_dir: bundle_dir.map(|path| path.to_string_lossy().into_owned()),
        });
    }

    let report = BatchReport {
        pack_id: pack.id.clone(),
        pack_name: pack.name.clone(),
        total_runs: records.len(),
        summary: summarize_records(&records),
        records,
    };

    if let Some(output_dir) = output_dir {
        fs::create_dir_all(output_dir).with_context(|| {
            format!(
                "failed to create batch eval output directory {}",
                output_dir.display()
            )
        })?;
        write_json(&output_dir.join("pack.json"), pack)?;
        write_json(&output_dir.join("summary.json"), &report)?;
    }

    Ok(report)
}

fn validate_pack(pack: &ScenarioPackSpec) -> Result<()> {
    if pack.id.trim().is_empty() {
        bail!("pack id must not be empty");
    }
    if pack.name.trim().is_empty() {
        bail!("pack name must not be empty");
    }
    if pack.entries.is_empty() {
        bail!("pack must contain at least one entry");
    }

    let mut seen_ids = BTreeMap::new();
    for entry in &pack.entries {
        if entry.id.trim().is_empty() {
            bail!("pack entry id must not be empty");
        }
        if entry.scenario.trim().is_empty() {
            bail!("pack entry scenario path must not be empty");
        }
        if entry.controller.trim().is_empty() {
            bail!("pack entry controller must not be empty");
        }
        if seen_ids.insert(entry.id.clone(), true).is_some() {
            bail!("duplicate pack entry id '{}'", entry.id);
        }
    }

    Ok(())
}

fn load_scenario(path: &Path) -> Result<ScenarioSpec> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse scenario json {}", path.display()))
}

fn load_controller_spec(base_dir: &Path, entry: &ScenarioPackEntry) -> Result<ControllerSpec> {
    if let Some(path) = entry.controller_config.as_deref() {
        let full_path = base_dir.join(path);
        let raw = fs::read_to_string(&full_path).with_context(|| {
            format!("failed to read controller config file {}", full_path.display())
        })?;
        return serde_json::from_str(&raw).with_context(|| {
            format!(
                "failed to parse controller config json {}",
                full_path.display()
            )
        });
    }

    built_in_controller_spec(&entry.controller)
        .ok_or_else(|| anyhow::anyhow!("unknown controller '{}'", entry.controller))
}

fn summarize_records(records: &[BatchRunRecord]) -> BatchSummary {
    let total_runs = records.len();
    let success_runs = records
        .iter()
        .filter(|record| matches!(record.manifest.mission_outcome, MissionOutcome::Success))
        .count();
    let mean_sim_time_s = if total_runs == 0 {
        0.0
    } else {
        records
            .iter()
            .map(|record| record.manifest.sim_time_s)
            .sum::<f64>()
            / total_runs as f64
    };

    let mut mission_outcomes = BTreeMap::new();
    let mut end_reasons = BTreeMap::new();
    for record in records {
        *mission_outcomes
            .entry(enum_label(&record.manifest.mission_outcome))
            .or_insert(0) += 1;
        *end_reasons
            .entry(enum_label(&record.manifest.end_reason))
            .or_insert(0) += 1;
    }

    BatchSummary {
        total_runs,
        success_runs,
        mean_sim_time_s,
        mission_outcomes,
        end_reasons,
    }
}

fn write_artifact_bundle(
    path: &Path,
    scenario: &ScenarioSpec,
    controller_spec: &ControllerSpec,
    artifacts: &ControlledRunArtifacts,
) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create artifact bundle dir {}", path.display()))?;
    write_json(&path.join("scenario.json"), scenario)?;
    write_json(&path.join("controller.json"), controller_spec)?;
    write_json(
        &path.join("controller_updates.json"),
        &artifacts.controller_updates,
    )?;
    write_json(&path.join("manifest.json"), &artifacts.run.manifest)?;
    write_json(&path.join("actions.json"), &artifacts.run.actions)?;
    write_json(&path.join("events.json"), &artifacts.run.events)?;
    write_json(&path.join("samples.json"), &artifacts.run.samples)?;
    Ok(())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let raw = serde_json::to_string_pretty(value)?;
    fs::write(path, raw)
        .with_context(|| format!("failed to write json file {}", path.display()))?;
    Ok(())
}

fn enum_label<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"unknown\"".to_owned())
        .trim_matches('"')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn run_pack_aggregates_nominal_suite() {
        let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures");
        let pack = ScenarioPackSpec {
            id: "unit_pack".to_owned(),
            name: "Unit pack".to_owned(),
            description: "unit pack".to_owned(),
            entries: vec![
                ScenarioPackEntry {
                    id: "landing_success".to_owned(),
                    scenario: "scenarios/flat_terminal_descent.json".to_owned(),
                    controller: "baseline".to_owned(),
                    controller_config: None,
                },
                ScenarioPackEntry {
                    id: "checkpoint_success".to_owned(),
                    scenario: "scenarios/timed_checkpoint_idle.json".to_owned(),
                    controller: "idle".to_owned(),
                    controller_config: None,
                },
            ],
        };

        let report = run_pack(&pack, &base_dir, None).unwrap();

        assert_eq!(report.total_runs, 2);
        assert_eq!(report.summary.success_runs, 2);
        assert_eq!(
            report.summary.mission_outcomes.get("success").copied(),
            Some(2)
        );
    }
}
