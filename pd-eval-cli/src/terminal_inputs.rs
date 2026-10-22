//! Evaluator-owned physical expansion; no controller config loading or flight execution.
use super::*;
use pd_missions::{
    NativeScenarioInputV1, ScenarioInputManifestV1, ScenarioInputProvenanceV1,
    ScenarioInputSourceV1, read_scenario_input_file, sha256, write_scenario_inputs,
};

pub const TERMINAL_INPUT_RESOLVER: &str = "retained-terminal-native-inputs-v1";

fn validate_input_count(pack: &ScenarioPackSpec) -> Result<()> {
    let mut count = 0_usize;
    for entry in &pack.entries {
        let ScenarioPackEntry::TerminalMatrix(entry) = entry else {
            bail!("terminal input export requires terminal-matrix entries only");
        };
        let arcs = selected_terminal_arc_specs(
            entry,
            terminal_arrival_family_spec(&entry.terminal_matrix)?,
        )?
        .len();
        let slots = arcs
            .checked_mul(TERMINAL_BANDS.len())
            .and_then(|v| v.checked_mul(terminal_seed_specs(entry.seed_tier).len()))
            .and_then(|v| v.checked_mul(entry.lanes.len()))
            .ok_or_else(|| anyhow!("terminal input count overflow"))?;
        count = count
            .checked_add(slots)
            .ok_or_else(|| anyhow!("terminal input count overflow"))?;
        if count > pd_missions::MAX_SCENARIO_INPUTS {
            bail!("too many terminal input slots before expansion");
        }
    }
    Ok(())
}

pub fn resolve_terminal_inputs(
    pack: &ScenarioPackSpec,
    base_dir: &Path,
) -> Result<Vec<NativeScenarioInputV1>> {
    validate_pack(pack)?;
    validate_input_count(pack)?;
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
    validate_input_count(&pack)?;
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
    fn export_slot_bound_is_checked_before_loading_any_base_scenario() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/packs");
        let mut pack = load_pack(&root.join("terminal_bot_lab_full.json")).unwrap();
        pack.entries.truncate(1);
        let ScenarioPackEntry::TerminalMatrix(entry) = &mut pack.entries[0] else {
            unreachable!()
        };
        entry.base_scenario = "does-not-exist.json".into();
        let lane = entry.lanes[0].clone();
        entry.lanes = (0..1000)
            .map(|index| TerminalMatrixLaneSpec {
                id: format!("lane-{index}"),
                ..lane.clone()
            })
            .collect();
        assert!(
            resolve_terminal_inputs(&pack, &root)
                .unwrap_err()
                .to_string()
                .contains("too many terminal input slots")
        );
    }

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
