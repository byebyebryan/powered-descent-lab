//! Evaluator-owned physical expansion; no controller config loading or flight execution.
use super::*;
use pd_missions::{
    NativeScenarioInputV1, ScenarioInputManifestV1, ScenarioInputProvenanceV1,
    ScenarioInputSourceV1, read_scenario_input_file, sha256, write_scenario_inputs,
};

pub const TERMINAL_INPUT_RESOLVER: &str = "retained-terminal-native-inputs-v1";

pub fn resolve_terminal_inputs(
    pack: &ScenarioPackSpec,
    base_dir: &Path,
) -> Result<Vec<NativeScenarioInputV1>> {
    validate_pack(pack)?;
    let mut inputs = Vec::new();
    for entry in &pack.entries {
        let ScenarioPackEntry::TerminalMatrix(entry) = entry else {
            bail!("terminal input export requires terminal-matrix entries only");
        };
        let base = load_scenario(&base_dir.join(&entry.base_scenario))?;
        let expansion = TerminalInputExpansion::new(entry, &base, pack.terminal_matrix_max_time_s)?;
        for lane in &entry.lanes {
            for input in expansion.lane(&lane.id)? {
                let selector = serde_json::to_value(&input.selector)?;
                let mut labels = BTreeMap::new();
                for (key, value) in selector.as_object().expect("selector object") {
                    if let Some(value) = value.as_str() {
                        labels.insert(key.clone(), value.to_owned());
                    }
                }
                labels.extend([
                    ("source_pack".into(), pack.id.clone()),
                    ("entry_id".into(), entry.id.clone()),
                    ("lane_id".into(), input.lane_id),
                    ("controller_provenance".into(), lane.controller.clone()),
                    ("scenario_source".into(), entry.base_scenario.clone()),
                ]);
                if let Some(config) = &lane.controller_config {
                    labels.insert("controller_config_provenance".into(), config.clone());
                }
                inputs.push(NativeScenarioInputV1 {
                    id: input.run_id,
                    scenario: input.scenario,
                    labels,
                    parameters: input.resolved_parameters,
                    lineage: vec![format!("pack:{}", pack.id)],
                });
            }
        }
    }
    if inputs.len() > pd_missions::MAX_SCENARIO_INPUTS
        || inputs.iter().map(|i| &i.id).collect::<BTreeSet<_>>().len() != inputs.len()
    {
        bail!("too many or duplicate terminal input occurrences");
    }
    Ok(inputs)
}

pub fn export_terminal_inputs(pack_path: &Path, output: &Path) -> Result<ScenarioInputManifestV1> {
    let pack_bytes =
        read_scenario_input_file(pack_path, 4 * 1024 * 1024).map_err(anyhow::Error::msg)?;
    let pack: ScenarioPackSpec = serde_json::from_slice(&pack_bytes)?;
    validate_pack(&pack)?;
    let base_dir = pack_path.parent().unwrap_or(Path::new("."));
    let mut sources = BTreeMap::new();
    sources.insert(fs::canonicalize(pack_path)?, sha256(&pack_bytes));
    for entry in &pack.entries {
        let ScenarioPackEntry::TerminalMatrix(entry) = entry else {
            bail!("terminal input export requires terminal-matrix entries only");
        };
        let path = base_dir.join(&entry.base_scenario);
        let bytes = read_scenario_input_file(&path, pd_missions::MAX_SCENARIO_INPUT_BYTES)
            .map_err(anyhow::Error::msg)?;
        sources.insert(fs::canonicalize(&path)?, sha256(&bytes));
    }
    let inputs = resolve_terminal_inputs(&pack, base_dir)?;
    for (path, digest) in &sources {
        if sha256(
            &read_scenario_input_file(path, pd_missions::MAX_SCENARIO_INPUT_BYTES)
                .map_err(anyhow::Error::msg)?,
        ) != *digest
        {
            bail!("terminal source input changed during expansion");
        }
    }
    let executable = read_scenario_input_file(&std::env::current_exe()?, 512 * 1024 * 1024)
        .map_err(anyhow::Error::msg)?;
    let manifest = write_scenario_inputs(
        output,
        ScenarioInputProvenanceV1 {
            implementation: TERMINAL_INPUT_RESOLVER.into(),
            executable_sha256: sha256(&executable),
            sources: sources
                .into_iter()
                .map(|(path, sha256)| ScenarioInputSourceV1 {
                    path: path.to_string_lossy().into(),
                    sha256,
                })
                .collect(),
        },
        &inputs,
    )
    .map_err(anyhow::Error::msg)?;
    let (loaded, native) = pd_missions::load_scenario_inputs(output).map_err(anyhow::Error::msg)?;
    if loaded != manifest || native != inputs {
        bail!("exported terminal inputs did not round-trip exactly");
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_match_legacy_exactly_and_ignore_unavailable_controller_configuration() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/packs");
        for filename in [
            "terminal_bot_lab_suite.json",
            "terminal_bot_lab_full.json",
            "terminal_traj_err_suite.json",
            "terminal_traj_err_full.json",
        ] {
            let mut pack = load_pack(&root.join(filename)).unwrap();
            let original = resolve_pack_runs(&pack, &root).unwrap();
            let inputs = resolve_terminal_inputs(&pack, &root).unwrap();
            assert_eq!(original.len(), inputs.len());
            for (a, b) in original.iter().zip(&inputs) {
                assert_eq!(a.scenario, b.scenario);
                assert_eq!(a.descriptor.run_id, b.id);
                assert_eq!(a.descriptor.resolved_parameters, b.parameters);
            }
            for entry in &mut pack.entries {
                if let ScenarioPackEntry::TerminalMatrix(entry) = entry {
                    for lane in &mut entry.lanes {
                        lane.controller_config = Some("does-not-exist.json".into());
                    }
                }
            }
            assert!(resolve_pack_runs(&pack, &root).is_err());
            let independent = resolve_terminal_inputs(&pack, &root).unwrap();
            assert!(
                inputs
                    .iter()
                    .zip(&independent)
                    .all(|(a, b)| a.scenario == b.scenario)
            );
        }
    }
}
