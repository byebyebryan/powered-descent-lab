//! Cross-source diagnostic comparison. This never changes execution guards.

use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

pub(super) fn current_contract() -> Value {
    serde_json::from_str(include_str!(
        "../../../studies/terrain_profiles/sentinel_comparison.json"
    ))
    .expect("tracked comparison contract")
}

pub(super) fn latest_contract() -> Value {
    serde_json::from_str(include_str!(
        "../../../studies/terrain_profiles/sentinel_comparison_v2.json"
    ))
    .expect("tracked comparison contract")
}

pub(super) fn contract_filename(contract: &Value) -> &'static str {
    if contract == &latest_contract() {
        "sentinel_comparison_v2.json"
    } else {
        "sentinel_comparison.json"
    }
}

pub(super) fn validate_contract(contract: Option<&Value>) -> Result<()> {
    ensure!(
        contract.is_none_or(|value| value == &current_contract() || value == &latest_contract()),
        "unknown sentinel comparison contract"
    );
    Ok(())
}

// Normalize only authenticated derived data. Everything else, including the
// complete trajectory states, command schedules and policy, is compared exactly.
fn derived_local_data(
    actual: &mut Value,
    baseline: &mut Value,
    exceptions: &mut Vec<Value>,
) -> Result<()> {
    use crate::{LocalClearingProposalV1, local_clearing::proposal_identity};
    let mut identities = [
        std::collections::BTreeMap::new(),
        std::collections::BTreeMap::new(),
    ];
    for (side, flight) in [&mut *baseline, &mut *actual].into_iter().enumerate() {
        for (index, cycle) in flight["cycles"]
            .as_array_mut()
            .context("flight cycles")?
            .iter_mut()
            .enumerate()
        {
            let Some(selected) = cycle
                .pointer("/local_search/selected")
                .filter(|v| !v.is_null())
            else {
                continue;
            };
            let proposal: LocalClearingProposalV1 = serde_json::from_value(selected.clone())?;
            ensure!(
                proposal.identity == proposal_identity(&proposal)?,
                "invalid local proposal content hash at cycle {index}"
            );
            ensure!(
                identities[side]
                    .insert(proposal.identity.clone(), index)
                    .is_none(),
                "duplicate selected proposal identity"
            );
        }
        let mut referenced = std::collections::BTreeSet::new();
        for segment in flight["segments"].as_array().context("flight segments")? {
            if segment["kind"] != "local_correction" {
                continue;
            }
            let identity = segment["proposal_identity"]
                .as_str()
                .context("segment identity")?;
            let index = *identities[side]
                .get(identity)
                .context("unbound local segment identity")?;
            let proposal = &flight["cycles"][index]["local_search"]["selected"];
            ensure!(
                referenced.insert(index)
                    && segment["entry_state"] == proposal["entry_state"]
                    && segment["end_state"] == proposal["handoff_state"]
                    && segment["start_physics_step"] == proposal["schedule"]["entry_physics_step"]
                    && segment["end_physics_step"] == proposal["schedule"]["handoff_physics_step"],
                "invalid local segment binding"
            );
        }
        ensure!(
            referenced.len() == identities[side].len(),
            "selected proposal lacks executed segment binding"
        );
    }
    let count = baseline["cycles"]
        .as_array()
        .context("baseline cycles")?
        .len();
    ensure!(
        actual["cycles"].as_array().context("actual cycles")?.len() == count,
        "flight cycle count differs"
    );
    for index in 0..count {
        let path = format!("/cycles/{index}/local_search/selected");
        let a = actual.pointer(&path).filter(|v| !v.is_null());
        let b = baseline.pointer(&path).filter(|v| !v.is_null());
        ensure!(
            a.is_some() == b.is_some(),
            "selected proposal disposition differs"
        );
        let (Some(a), Some(b)) = (a, b) else {
            continue;
        };
        let required = b["policy"]["minimum_clearance_m"]
            .as_f64()
            .context("selected reserve")?;
        ensure!(
            required.is_finite()
                && a["policy"]["minimum_clearance_m"] == b["policy"]["minimum_clearance_m"],
            "selected reserve differs"
        );
        let (a, b) = (
            a["trajectory"].as_array().context("actual trajectory")?,
            b["trajectory"].as_array().context("baseline trajectory")?,
        );
        ensure!(a.len() == b.len(), "selected trajectory length differs");
        let mut clearances = Vec::new();
        for (sample, (a, b)) in a.iter().zip(b).enumerate() {
            let (a, b) = (
                a["body_clearance_m"]
                    .as_f64()
                    .context("actual body reserve")?,
                b["body_clearance_m"]
                    .as_f64()
                    .context("baseline body reserve")?,
            );
            ensure!(
                a.is_finite() && b.is_finite(),
                "nonfinite selected body reserve"
            );
            if a == b {
                continue;
            }
            let delta = a - b;
            ensure!(
                delta.abs() <= 1e-12
                    && (a >= required) == (b >= required)
                    && (a > 0.0) == (b > 0.0)
                    && (a < 0.0) == (b < 0.0),
                "selected clearance exception exceeds bound or crosses threshold"
            );
            let sample_path = format!("{path}/trajectory/{sample}/body_clearance_m");
            exceptions.push(json!({"path":sample_path,"baseline":b,"actual":a,"delta_m":delta}));
            clearances.push((sample_path, b));
        }
        for (sample_path, value) in clearances {
            *actual
                .pointer_mut(&sample_path)
                .context("selected clearance path")? = json!(value);
        }
        let identity_path = format!("{path}/identity");
        normalize_identity(actual, baseline, &identity_path, index, exceptions)?;
    }
    let segments_a = actual["segments"]
        .as_array_mut()
        .context("actual segments")?;
    let segments_b = baseline["segments"]
        .as_array_mut()
        .context("baseline segments")?;
    ensure!(
        segments_a.len() == segments_b.len(),
        "segment count differs"
    );
    for (index, (a, b)) in segments_a.iter_mut().zip(segments_b).enumerate() {
        if a["kind"] != "local_correction" && b["kind"] != "local_correction" {
            continue;
        }
        let cycle_a = identities[1]
            .get(
                a["proposal_identity"]
                    .as_str()
                    .context("actual segment identity")?,
            )
            .context("actual local binding")?;
        let cycle_b = identities[0]
            .get(
                b["proposal_identity"]
                    .as_str()
                    .context("baseline segment identity")?,
            )
            .context("baseline local binding")?;
        ensure!(
            cycle_a == cycle_b,
            "segment references another clearing cycle"
        );
        let path = format!("/segments/{index}/proposal_identity");
        if a["proposal_identity"] != b["proposal_identity"] {
            exceptions.push(json!({"path":path,"actual":a["proposal_identity"],"baseline":b["proposal_identity"],"validated_content_hash":true}));
        }
        a["proposal_identity"] = json!(format!("authenticated-local-cycle-{cycle_a}"));
        b["proposal_identity"] = a["proposal_identity"].clone();
    }
    Ok(())
}

fn normalize_identity(
    actual: &mut Value,
    baseline: &mut Value,
    path: &str,
    index: usize,
    exceptions: &mut Vec<Value>,
) -> Result<()> {
    let a = actual
        .pointer_mut(path)
        .context("actual proposal identity")?;
    let b = baseline
        .pointer_mut(path)
        .context("baseline proposal identity")?;
    if a != b {
        exceptions.push(json!({"path":path,"actual":a,"baseline":b,"validated_content_hash":true}));
    }
    *a = json!(format!("authenticated-local-cycle-{index}"));
    *b = a.clone();
    Ok(())
}

fn non_timing(flight: &Value) -> Result<Value> {
    let mut value = flight.clone();
    let timings = value
        .as_object_mut()
        .context("flight object")?
        .remove("timings")
        .context("missing flight timings")?;
    let timings = timings.as_object().context("flight timing object")?;
    ensure!(
        timings.len() == 3
            && ["planning_s", "execution_s", "replay_s"]
                .iter()
                .all(|key| timings
                    .get(*key)
                    .and_then(Value::as_f64)
                    .is_some_and(|n| n.is_finite() && n >= 0.0)),
        "unexpected flight timing exclusions"
    );
    Ok(value)
}

fn walk(
    actual: &Value,
    baseline: &Value,
    path: &str,
    parents: Option<(&Value, &Value)>,
    contract: Option<&Value>,
    exceptions: &mut Vec<Value>,
) -> Result<()> {
    match (actual, baseline) {
        (Value::Object(a), Value::Object(b)) => {
            ensure!(a.keys().eq(b.keys()), "flight object keys differ at {path}");
            for (key, value) in a {
                walk(
                    value,
                    &b[key],
                    &format!("{path}/{key}"),
                    Some((actual, baseline)),
                    contract,
                    exceptions,
                )?;
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            ensure!(a.len() == b.len(), "flight array length differs at {path}");
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                walk(a, b, &format!("{path}/{index}"), None, contract, exceptions)?;
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (
                a.as_f64().context("actual flight number")?,
                b.as_f64().context("baseline flight number")?,
            );
            ensure!(
                a.is_finite() && b.is_finite(),
                "non-finite flight number at {path}"
            );
            if actual == baseline {
                return Ok(());
            }
            let mut parts = path.split('/').collect::<Vec<_>>();
            if parts.len() > 2 && parts[2].parse::<usize>().is_ok() {
                parts[2] = "*";
            }
            let pattern = parts.join("/");
            let contract = contract.context(format!("flight value differs at {path}"))?;
            ensure!(
                contract["allowed_paths"]
                    .as_array()
                    .context("allowed paths")?
                    .iter()
                    .any(|p| p == &pattern),
                "flight value differs at {path}"
            );
            let (parent_a, parent_b) = parents.context("clearance parent")?;
            let required_a = parent_a["required_clearance_m"]
                .as_f64()
                .context("actual required reserve")?;
            let required_b = parent_b["required_clearance_m"]
                .as_f64()
                .context("baseline required reserve")?;
            let delta = a - b;
            ensure!(
                required_a.is_finite()
                    && required_b.is_finite()
                    && required_a == required_b
                    && delta.abs()
                        <= contract["absolute_tolerance_m"]
                            .as_f64()
                            .context("comparison bound")?
                    && (a >= required_a) == (b >= required_b)
                    && (a > 0.0) == (b > 0.0)
                    && (a < 0.0) == (b < 0.0),
                "clearance exception exceeds bound or crosses a threshold at {path}"
            );
            exceptions.push(json!({"path":path, "baseline":b, "actual":a, "delta_m":delta}));
        }
        _ => ensure!(actual == baseline, "flight value differs at {path}"),
    }
    Ok(())
}

pub(super) fn compare(actual: &Value, baseline: &Value, contract: Option<&Value>) -> Result<Value> {
    validate_contract(contract)?;
    let mut exceptions = Vec::new();
    let mut actual = non_timing(actual)?;
    let mut baseline = non_timing(baseline)?;
    let derived = contract.is_some_and(|c| c == &latest_contract());
    if derived {
        derived_local_data(&mut actual, &mut baseline, &mut exceptions)?;
    }
    walk(&actual, &baseline, "", None, contract, &mut exceptions)?;
    if derived {
        exceptions.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    }
    Ok(Value::Array(exceptions))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_flight() -> Value {
        use crate::{LocalClearingProposalV1, local_clearing::proposal_identity};
        let scenario: pd_core::ScenarioSpec = serde_json::from_str(include_str!(
            "../../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .unwrap();
        let context = pd_core::RunContext::from_scenario(&scenario).unwrap();
        let state = pd_core::SimulationStateSnapshotV1::from_state(
            &pd_core::SimulationState::new(&context).unwrap(),
        );
        let mut proposal: LocalClearingProposalV1 = serde_json::from_value(json!({
            "policy": pd_plan::local_clearing::LocalClearingPolicyV1::default(),
            "context_identity":"synthetic", "goal":{"first_conflict_physics_step":2,"first_conflict_position_m":{"x":1.0,"y":2.0},"absolute_deadline_physics_step":10},
            "row_id":"synthetic", "template":{"row_index":0,"target_attitude_rad":0.0,"acceleration_factor":1.0,"powered_ticks":2},
            "entry_state":state,"powered_end_state":state,"handoff_state":state,"continuation_end_state":state,
            "minimum_progress_x_m":1.0,"actual_fuel_burn_to_handoff_kg":0.0,
            "schedule":{"entry_physics_step":0,"powered_end_physics_step":0,"handoff_physics_step":0,"continuation_end_physics_step":0,"absolute_deadline_physics_step":10,"updates":[]},
            "trajectory":[{"state":state,"body_clearance_m":8.0}],"identity":""
        })).unwrap();
        proposal.identity = proposal_identity(&proposal).unwrap();
        json!({"cycles":[{"local_search":{"selected":proposal}}],
            "segments":[{"kind":"local_correction","start_physics_step":0,"end_physics_step":0,"entry_state":state,"end_state":state,"proposal_identity":proposal.identity}],
            "timings":{"planning_s":1.0,"execution_s":2.0,"replay_s":3.0}})
    }

    fn rehash(flight: &mut Value) {
        let path = "/cycles/0/local_search/selected";
        let mut proposal: crate::LocalClearingProposalV1 =
            serde_json::from_value(flight.pointer(path).unwrap().clone()).unwrap();
        proposal.identity = crate::local_clearing::proposal_identity(&proposal).unwrap();
        *flight.pointer_mut(path).unwrap() = serde_json::to_value(&proposal).unwrap();
        flight["segments"][0]["proposal_identity"] = json!(proposal.identity);
    }

    #[test]
    fn derived_hashes_are_authenticated_not_ignored() {
        let baseline = local_flight();
        let mut actual = baseline.clone();
        let path = "/cycles/0/local_search/selected/trajectory/0/body_clearance_m";
        *actual.pointer_mut(path).unwrap() = json!(8.0 + 1e-13);
        assert!(compare(&actual, &baseline, Some(&latest_contract())).is_err());
        rehash(&mut actual);
        let exceptions = compare(&actual, &baseline, Some(&latest_contract())).unwrap();
        assert_eq!(exceptions.as_array().unwrap().len(), 3);
        assert!(compare(&actual, &baseline, Some(&current_contract())).is_err());
        let mut forged = actual.clone();
        forged["segments"][0]["proposal_identity"] = json!("forged");
        assert!(compare(&forged, &baseline, Some(&latest_contract())).is_err());
        let mut changed = actual;
        changed["cycles"][0]["local_search"]["selected"]["template"]["target_attitude_rad"] =
            json!(0.1);
        rehash(&mut changed);
        assert!(compare(&changed, &baseline, Some(&latest_contract())).is_err());
    }

    #[test]
    fn derived_clearance_does_not_hide_state_threshold_or_missing_reference() {
        let baseline = local_flight();
        for path in [
            "/cycles/0/local_search/selected/trajectory/0/state/velocity_mps/x",
            "/cycles/0/local_search/selected/policy/minimum_clearance_m",
        ] {
            let mut actual = baseline.clone();
            *actual.pointer_mut(path).unwrap() = json!(6.0);
            rehash(&mut actual);
            assert!(
                compare(&actual, &baseline, Some(&latest_contract())).is_err(),
                "{path}"
            );
        }
        let mut actual = baseline.clone();
        actual["segments"] = json!([]);
        assert!(compare(&actual, &baseline, Some(&latest_contract())).is_err());
        let mut baseline = baseline;
        baseline["cycles"][0]["local_search"]["selected"]["trajectory"][0]["body_clearance_m"] =
            json!(5.0);
        rehash(&mut baseline);
        let mut actual = baseline.clone();
        actual["cycles"][0]["local_search"]["selected"]["trajectory"][0]["body_clearance_m"] =
            json!(5.0 - 1e-13);
        rehash(&mut actual);
        assert!(compare(&actual, &baseline, Some(&latest_contract())).is_err());
        actual["cycles"][0]["local_search"]["selected"]["trajectory"][0]["body_clearance_m"] =
            json!(5.0 + 1e-8);
        rehash(&mut actual);
        assert!(compare(&actual, &baseline, Some(&latest_contract())).is_err());
    }

    fn edited(mut value: Value, edits: Option<&Value>) -> Value {
        for edit in edits.and_then(Value::as_array).into_iter().flatten() {
            let path = edit["path"].as_str().unwrap();
            let (parent, key) = path.rsplit_once('/').unwrap();
            let parent = value.pointer_mut(parent).unwrap();
            if edit["remove"] == true {
                parent.as_object_mut().unwrap().remove(key);
            } else if let Some(parent) = parent.as_object_mut() {
                parent.insert(key.into(), edit["value"].clone());
            } else {
                parent[key.parse::<usize>().unwrap()] = edit["value"].clone();
            }
        }
        value
    }

    #[test]
    fn shared_python_rust_contract_cases() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../studies/terrain_profiles/sentinel_comparison_cases.json"
        ))
        .unwrap();
        for case in corpus["cases"].as_array().unwrap() {
            let baseline = edited(corpus["baseline"].clone(), case.get("baseline_edits"));
            let actual = edited(baseline.clone(), case.get("edits"));
            let contract = edited(current_contract(), case.get("contract_edits"));
            let contract = if case["exact"] == true {
                None
            } else {
                Some(&contract)
            };
            let result = compare(&actual, &baseline, contract);
            assert_eq!(
                result.is_ok(),
                case["accepted"] == true,
                "{}: {result:?}",
                case["name"]
            );
            if let Ok(exceptions) = result {
                let exceptions = exceptions.as_array().unwrap();
                assert_eq!(
                    exceptions.len(),
                    case["exceptions"].as_u64().unwrap() as usize
                );
                for exception in exceptions {
                    assert_eq!(
                        exception["delta_m"].as_f64().unwrap(),
                        exception["actual"].as_f64().unwrap()
                            - exception["baseline"].as_f64().unwrap()
                    );
                    assert!(exception["delta_m"].as_f64().unwrap().abs() <= 1e-12);
                }
                if case["name"] == "multiple-exceptions" {
                    assert!(
                        exceptions[0]["path"]
                            .as_str()
                            .unwrap()
                            .contains("first_violation")
                    );
                    assert!(
                        exceptions[1]["path"]
                            .as_str()
                            .unwrap()
                            .contains("minimum_airborne")
                    );
                }
            }
        }
    }
}
