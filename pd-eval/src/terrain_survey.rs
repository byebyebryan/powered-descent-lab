//! Presentation-only adapter for a separate frozen procedural coverage survey.
//! Does not relax the sealed benchmark, simulate, or select flight trajectories.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};

use anyhow::{Context, Result, ensure};
use pd_core::{MissionOutcome, PhysicalOutcome, ScenarioSpec};
use pd_plan::waypoint_v2::{WaypointV2Policy, WaypointV2Stop};
use pd_report::{
    batch,
    flight_annotations::{AnnotationNavigation, NavigationLink},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    WaypointV2FlightResult,
    evidence_io::{reserve_output_root, sha256_bytes, write_bytes_create_only_with_context},
    nominal_direct_flight_identity,
    waypoint_v2_bundle::{validate_compact_summary, validate_supported_saved_result},
    waypoint_v2_report::render_rich_flight,
};

mod comparison;

/// Read-only comparator shared by collection and publication. Typed native
/// serialization authenticates proposal hashes; no simulation is constructed.
pub fn compare_saved_flights(actual: &Path, baseline: &Path, contract_id: &str) -> Result<Value> {
    let contract = comparison::latest_contract();
    ensure!(
        contract["id"] == contract_id,
        "unknown comparison contract id"
    );
    comparison::compare(
        &serde_json::from_slice(&fs::read(actual)?)?,
        &serde_json::from_slice(&fs::read(baseline)?)?,
        Some(&contract),
    )
}

#[derive(Debug, Deserialize)]
struct Survey {
    schema: String,
    manifest_sha256: String,
    source_before: Value,
    source_after: Value,
    stopped_reason: Option<String>,
    rows: Vec<Attempt>,
    summary: Value,
    #[serde(skip)]
    sentinel_comparison: Option<Value>,
    #[serde(skip)]
    challenge_phase: Option<String>,
}

type SavedFlight = Option<(ScenarioSpec, WaypointV2FlightResult)>;

fn artifact_names(root: &Path, directory: &Path, names: &mut BTreeSet<String>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        ensure!(
            !entry.file_type()?.is_symlink(),
            "symlink in survey inventory"
        );
        if entry.file_type()?.is_dir() {
            artifact_names(root, &entry.path(), names)?;
        } else {
            ensure!(entry.file_type()?.is_file(), "non-file survey artifact");
            names.insert(
                entry
                    .path()
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Attempt {
    attempt_id: String,
    case_id: String,
    cohort: String,
    seed: Option<u64>,
    scenario_path: String,
    status: String,
    #[serde(default)]
    output_dir: Option<String>,
    #[serde(default)]
    result: Option<Projection>,
    #[serde(default)]
    failure: Option<String>,
    #[serde(default)]
    comparison_exceptions: Vec<Value>,
    #[serde(default)]
    geometry: Value,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Projection {
    nominal_class: String,
    verified_landing: bool,
    planning_stop: WaypointV2Stop,
    physical_outcome: Option<PhysicalOutcome>,
    mission_outcome: Option<MissionOutcome>,
    correction_count: u32,
    integrity_passed: bool,
    final_source_replay_passed: bool,
}

fn project(result: &WaypointV2FlightResult) -> Projection {
    let nominal = match result.cycles.first().and_then(|c| c.audit.as_ref()) {
        None => "not_established",
        Some(audit) if audit.passed => "clear",
        Some(_) if result.initial_nominal_terrain_blocked => "blocked",
        Some(_) => "rejected",
    };
    Projection {
        nominal_class: nominal.into(),
        verified_landing: result.planning_stop == WaypointV2Stop::Landed
            && result.physical_outcome == Some(PhysicalOutcome::LandedOnTarget)
            && result.mission_outcome == Some(MissionOutcome::Success)
            && result.integrity_passed
            && result.final_source_replay_passed,
        planning_stop: result.planning_stop,
        physical_outcome: result.physical_outcome.clone(),
        mission_outcome: result.mission_outcome.clone(),
        correction_count: result.correction_count,
        integrity_passed: result.integrity_passed,
        final_source_replay_passed: result.final_source_replay_passed,
    }
}

fn safe_file(root: &Path, relative: &str) -> Result<std::path::PathBuf> {
    let relative = Path::new(relative);
    ensure!(
        !relative.as_os_str().is_empty()
            && relative
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "unsafe survey artifact path"
    );
    let mut path = root.to_path_buf();
    ensure!(
        !fs::symlink_metadata(&path)?.file_type().is_symlink(),
        "symlink survey root"
    );
    for part in relative.components() {
        path.push(part);
        ensure!(
            !fs::symlink_metadata(&path)?.file_type().is_symlink(),
            "symlink survey artifact"
        );
    }
    ensure!(path.is_file(), "survey artifact is not a file");
    Ok(path)
}

fn read(root: &Path, relative: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(safe_file(
        root, relative,
    )?)?)?)
}

fn enum_text<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("serializable enum")
        .as_str()
        .expect("string enum")
        .into()
}

fn counts<I: IntoIterator<Item = String>>(values: I) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value).or_default() += 1;
    }
    counts
}

fn totals(rows: &[Attempt]) -> Value {
    let primary = rows
        .iter()
        .filter(|r| r.cohort == "random")
        .collect::<Vec<_>>();
    let results = primary
        .iter()
        .filter(|r| r.status == "recorded")
        .filter_map(|r| r.result.as_ref())
        .collect::<Vec<_>>();
    json!({
        "primary_count": primary.len(), "recorded_count": results.len(),
        "verified_landings": results.iter().filter(|r| r.verified_landing).count(),
        "dispositions": counts(primary.iter().map(|r| r.status.clone())),
        "nominal_classes": counts(results.iter().map(|r| r.nominal_class.clone())),
        "planning_stops": counts(results.iter().map(|r| enum_text(&r.planning_stop))),
        "physical_outcomes": counts(results.iter().map(|r| r.physical_outcome.as_ref().map_or_else(|| "None".into(), enum_text))),
        "corrections": counts(results.iter().map(|r| r.correction_count.to_string())),
        "sentinels_recorded": rows.iter().filter(|r| r.cohort == "sentinel" && r.status == "recorded").count(),
        "repeats_recorded": rows.iter().filter(|r| r.cohort == "repeat" && r.status == "recorded").count(),
    })
}

fn validate(root: &Path) -> Result<(Survey, Vec<SavedFlight>)> {
    let root = root.canonicalize()?;
    let receipt = read(&root, "receipt.json")?;
    let mut actual = BTreeSet::new();
    artifact_names(&root, &root, &mut actual)?;
    actual.remove("receipt.json");
    ensure!(
        actual
            == receipt["files"]
                .as_object()
                .context("missing hashes")?
                .keys()
                .cloned()
                .collect(),
        "incomplete survey artifact inventory"
    );
    for (relative, hash) in receipt["files"]
        .as_object()
        .context("missing artifact hashes")?
    {
        ensure!(
            sha256_bytes(&fs::read(safe_file(&root, relative)?)?)?
                == hash.as_str().context("invalid hash")?,
            "survey hash mismatch: {relative}"
        );
    }
    let bytes = fs::read(safe_file(&root, "survey.json")?)?;
    let mut survey: Survey = serde_json::from_slice(&bytes)?;
    let manifest = read(&root, "manifest.json")?;
    survey.sentinel_comparison = manifest.get("sentinel_comparison").cloned();
    comparison::validate_contract(survey.sentinel_comparison.as_ref())?;
    if let Some(contract) = &survey.sentinel_comparison {
        let filename = comparison::contract_filename(contract);
        let path = format!("inputs/tools/{filename}");
        let source_path = format!("studies/terrain_profiles/{filename}");
        ensure!(
            &read(&root, &path)? == contract
                && manifest["source"]["files"][&source_path]
                    == sha256_bytes(&fs::read(safe_file(&root, &path)?)?)?,
            "sentinel comparison contract is not source-bound"
        );
    }
    let plan = read(&root, "plan.json")?;
    ensure!(
        [
            include_str!("../../studies/terrain_profiles/survey_plan.json"),
            include_str!("../../studies/terrain_profiles/challenge_calibration_plan.json"),
            include_str!("../../studies/terrain_profiles/challenge_plan.json"),
            include_str!("../../studies/terrain_profiles/intervention_timing_plan.json")
        ]
        .iter()
        .any(|text| serde_json::from_str::<Value>(text).is_ok_and(|expected| expected == plan)),
        "survey plan differs from frozen contract"
    );
    let timing = plan["schema"] == "pd-lab.intervention-timing-pass.v1";
    survey.challenge_phase = if timing {
        let phase = manifest["pass_phase"].as_str().context("timing phase")?;
        ensure!(
            matches!(phase, "diagnostics" | "focus" | "challenge"),
            "unknown timing phase"
        );
        Some(format!("timing-{phase}"))
    } else {
        plan["phase"].as_str().map(str::to_owned)
    };
    if let Some(phase) = &survey.challenge_phase {
        let filename = if timing {
            "intervention_timing_plan.json"
        } else if phase == "calibration" {
            "challenge_calibration_plan.json"
        } else {
            "challenge_plan.json"
        };
        let source_path = format!("studies/terrain_profiles/{filename}");
        let bytes = fs::read(safe_file(&root, "plan.json")?)?;
        ensure!(
            manifest["plan_source_path"] == source_path
                && manifest["source"]["files"][&source_path] == sha256_bytes(&bytes)?
                && fs::read(safe_file(&root, &format!("inputs/source/{source_path}"))?)? == bytes,
            "challenge plan is not source-bound"
        );
    }
    let focused = timing && manifest["pass_phase"] != "challenge";
    let primary_count = if focused {
        plan["focus_indices"]
            .as_array()
            .context("focus indices")?
            .len()
    } else {
        plan["seed_count"].as_u64().context("population count")? as usize
    };
    let repeat_indices = if focused {
        Vec::new()
    } else {
        plan["repeat_indices"]
            .as_array()
            .context("repeat indices")?
            .clone()
    };
    let sentinel_count = if timing { 0 } else { 3 };
    let primary_end = sentinel_count + primary_count;
    ensure!(
        survey.schema == "pd-lab.random-terrain-survey.v1"
            && survey.rows.len() == primary_end + repeat_indices.len()
            && manifest["schema"] == "pd-lab.random-terrain-inputs.v1"
            && survey.source_before == manifest["source"]
            && (survey.stopped_reason.is_some() || survey.source_before == survey.source_after),
        "survey source/count/schema mismatch"
    );
    ensure!(
        survey.manifest_sha256 == sha256_bytes(&fs::read(safe_file(&root, "manifest.json")?)?)?,
        "unbound survey manifest"
    );
    let random = manifest["random_cases"]
        .as_array()
        .context("missing random cases")?;
    let sentinels = manifest["sentinel_cases"]
        .as_array()
        .context("missing sentinels")?;
    ensure!(
        random.len() == primary_count && sentinels.len() == sentinel_count,
        "input cohort count mismatch"
    );
    if timing {
        let baseline = read(&root, "inputs/baseline-manifest.json")?;
        let inputs = read(&root, "inputs/baseline-inputs-receipt.json")?;
        for (path, key) in [
            ("inputs/baseline-manifest.json", "baseline_manifest_sha256"),
            (
                "inputs/baseline-inputs-receipt.json",
                "baseline_inputs_receipt_sha256",
            ),
        ] {
            ensure!(
                sha256_bytes(&fs::read(safe_file(&root, path)?)?)?
                    == plan[key].as_str().context("baseline hash")?,
                "unbound timing baseline"
            );
        }
        for (index, row) in random.iter().enumerate() {
            let original_index = if focused {
                plan["focus_indices"][index]
                    .as_u64()
                    .context("focus index")? as usize
            } else {
                index
            };
            ensure!(
                baseline["random_cases"][original_index] == *row,
                "timing input differs from original challenge"
            );
            let path = row["scenario_path"].as_str().context("scenario path")?;
            ensure!(
                sha256_bytes(&fs::read(safe_file(&root, path)?)?)?
                    == inputs["files"][path]
                        .as_str()
                        .context("original scenario hash")?,
                "timing scenario changed"
            );
        }
    }
    let mut flights = Vec::new();
    for (index, row) in survey.rows.iter().enumerate() {
        let (expected, cohort, id) = if index < sentinel_count {
            (
                &sentinels[index],
                "sentinel",
                sentinels[index]["case_id"]
                    .as_str()
                    .context("sentinel id")?
                    .to_owned(),
            )
        } else if index < primary_end {
            (
                &random[index - sentinel_count],
                "random",
                if timing {
                    random[index - sentinel_count]["case_id"]
                        .as_str()
                        .context("case id")?
                        .to_owned()
                } else {
                    format!("random-{:03}", index - sentinel_count)
                },
            )
        } else {
            let repeated = repeat_indices[index - primary_end]
                .as_u64()
                .context("repeat index")? as usize;
            (&random[repeated], "repeat", format!("repeat-{repeated:03}"))
        };
        ensure!(
            row.attempt_id == id
                && row.cohort == cohort
                && expected["case_id"] == row.case_id
                && expected["seed"] == serde_json::to_value(row.seed)?
                && expected["scenario_path"] == row.scenario_path
                && expected.get("geometry").unwrap_or(&Value::Null) == &row.geometry,
            "survey row differs from frozen input at {index}"
        );
        ensure!(
            row.attempt_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "unsafe attempt id"
        );
        if row.status != "recorded" {
            ensure!(
                matches!(
                    row.status.as_str(),
                    "not_attempted" | "runner_timeout" | "runner_error" | "evidence_error"
                ) && survey.stopped_reason.is_some(),
                "unaccounted survey attempt"
            );
            flights.push(None);
            continue;
        }
        let prefix = format!("runs/{}", row.attempt_id);
        ensure!(
            row.output_dir.as_deref() == Some(prefix.as_str()),
            "unexpected flight directory"
        );
        let scenario: ScenarioSpec = serde_json::from_value(read(&root, &row.scenario_path)?)?;
        let captured: ScenarioSpec =
            serde_json::from_value(read(&root, &format!("{prefix}/scenario.json"))?)?;
        ensure!(
            serde_json::to_value(&scenario)? == serde_json::to_value(&captured)?,
            "flight scenario mismatch"
        );
        let result: WaypointV2FlightResult =
            serde_json::from_value(read(&root, &format!("{prefix}/flight.json"))?)?;
        let request = WaypointDirectNominalDirectGenerationRequest {
            probe_id: scenario.id.clone(),
            scenario: scenario.clone(),
            source_pad_id: "pad_source".into(),
            target_pad_id: "pad_main".into(),
            policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
        };
        ensure!(
            result.policy == WaypointV2Policy::revision_3()
                && result.input_identity
                    == nominal_direct_flight_identity(&(&request, &result.policy))?,
            "native input identity mismatch"
        );
        validate_supported_saved_result(&result, &request)?;
        validate_compact_summary(&read(&root, &format!("{prefix}/summary.json"))?, &result)?;
        ensure!(
            row.result.as_ref() == Some(&project(&result))
                && result.integrity_passed
                && result.final_source_replay_passed,
            "survey projection or recorded proof mismatch"
        );
        if cohort != "random" {
            let comparison_path = if cohort == "sentinel" {
                format!("controls/{}.json", row.case_id)
            } else {
                format!("runs/{}/flight.json", row.case_id)
            };
            let baseline = read(&root, &comparison_path)?;
            let actual = serde_json::to_value(&result)?;
            let contract = if cohort == "sentinel" {
                survey.sentinel_comparison.as_ref()
            } else {
                None
            };
            let exceptions = comparison::compare(&actual, &baseline, contract)?;
            ensure!(
                exceptions == serde_json::to_value(&row.comparison_exceptions)?,
                "recorded comparison exception list disagrees"
            );
        }
        flights.push(Some((scenario, result)));
    }
    ensure!(
        survey.summary == totals(&survey.rows),
        "survey summary differs from recorded rows"
    );
    Ok((survey, flights))
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn metric(value: Option<f64>, unit: &str) -> String {
    value.map_or_else(|| "—".into(), |v| format!("{v:.3} {unit}"))
}

fn conditional_coverage(rows: &[Attempt]) -> String {
    let primary = rows
        .iter()
        .filter(|r| r.cohort == "random")
        .collect::<Vec<_>>();
    let mut groups = BTreeMap::<String, Vec<&Attempt>>::new();
    groups.insert("All primary cases".into(), primary.clone());
    for row in primary {
        if let Some(recipe) = row.geometry["recipe_id"].as_str() {
            groups.entry(recipe.into()).or_default().push(row);
        }
    }
    let mut html = String::from(
        "<div class=\"table-wrap\"><table><thead><tr><th>Recipe / scope</th><th>Recorded / frozen</th><th>Initial blocked · landed</th><th>Initial clear · landed</th><th>No nominal / other rejection</th><th>Multi-handoff flights</th></tr></thead><tbody>",
    );
    for (name, rows) in groups {
        let projections = rows
            .iter()
            .filter(|r| r.status == "recorded")
            .filter_map(|r| r.result.as_ref())
            .collect::<Vec<_>>();
        let count = |class: &str| {
            projections
                .iter()
                .filter(|p| p.nominal_class == class)
                .count()
        };
        let landed = |class: &str| {
            projections
                .iter()
                .filter(|p| p.nominal_class == class && p.verified_landing)
                .count()
        };
        html.push_str(&format!("<tr><td>{}</td><td>{}/{}</td><td>{}/{} blocked routes landed</td><td>{}/{} clear routes landed</td><td>{}</td><td>{}</td></tr>",
            escape(&name),projections.len(),rows.len(),landed("blocked"),count("blocked"),landed("clear"),count("clear"),
            count("not_established")+count("rejected"),projections.iter().filter(|p| p.correction_count>=2).count()));
    }
    html.push_str("</tbody></table></div><p>Blocked and clear denominators use recorded initial audits. Unrecorded cases are not treated as clear or successful; policy rejection is not proof of physical impossibility. Calibration and held-out populations are separate captures.</p>");
    html
}

fn batch_page(survey: &Survey, flights: &[SavedFlight], base: &str) -> String {
    let mut overview_rows = String::new();
    let mut tree_rows = String::new();
    let headers = "<tr><th>Selector</th><th>Success / Outcome</th><th>Fuel Used</th><th>Flight Time</th><th>Landing Offset</th><th>Reference deviation</th><th>Preview</th><th>Nominal</th><th>Handoffs</th><th>Planning</th></tr>";
    for cohort in ["random", "sentinel", "repeat"] {
        let group = survey
            .rows
            .iter()
            .filter(|r| r.cohort == cohort)
            .collect::<Vec<_>>();
        let landed = group
            .iter()
            .filter(|r| {
                r.status == "recorded" && r.result.as_ref().is_some_and(|p| p.verified_landing)
            })
            .count();
        let attempted = group.iter().filter(|r| r.status != "not_attempted").count();
        let detail = format!(
            "{landed} verified target landings · {attempted} attempted · {} unattempted",
            group.len() - attempted
        );
        overview_rows.push_str(&batch::render_overview_row(
            "current-summary-row",
            &[
                cohort,
                "Separate frozen cohort",
                &group.len().to_string(),
                &detail,
                &metric(
                    survey
                        .rows
                        .iter()
                        .zip(flights)
                        .filter(|(r, _)| r.cohort == cohort)
                        .filter_map(|(_, f)| f.as_ref().map(|(_, f)| f.timings.planning_s))
                        .reduce(|a, b| a + b),
                    "s total planning",
                ),
                &metric(
                    survey
                        .rows
                        .iter()
                        .zip(flights)
                        .filter(|(r, _)| r.cohort == cohort)
                        .filter_map(|(_, f)| {
                            f.as_ref()
                                .and_then(|(_, f)| f.manifest.as_ref())
                                .map(|m| m.summary.fuel_used_kg)
                        })
                        .reduce(|a, b| a + b),
                    "kg total fuel",
                ),
                "No reference controller",
            ],
        ));
        tree_rows.push_str(&batch::render_row(Some("summary-row current-row"),
            &format!("data-group=\"{cohort}\" data-kind=\"cohort\" data-depth=\"0\" aria-expanded=\"false\" tabindex=\"0\""),
            &format!("<td class=\"tree-label\"><span class=\"expander\">+</span> {cohort}</td><td>{detail}</td><td colspan=\"8\">{} cases · expand to inspect</td>", group.len())));
        for (index, row) in survey
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.cohort == cohort)
        {
            let flight = flights[index].as_ref().map(|(_, f)| f);
            let projection = row.result.as_ref().filter(|_| row.status == "recorded");
            let outcome = projection.map_or_else(
                || row.status.clone(),
                |p| {
                    if p.verified_landing {
                        "verified target landing".into()
                    } else {
                        enum_text(&p.planning_stop)
                    }
                },
            );
            let nominal = projection.map_or("not classified", |p| p.nominal_class.as_str());
            let handoffs =
                projection.map_or_else(|| "—".into(), |p| p.correction_count.to_string());
            let fuel = flight
                .and_then(|f| f.manifest.as_ref())
                .map(|m| m.summary.fuel_used_kg);
            let time = flight
                .and_then(|f| f.manifest.as_ref())
                .map(|m| m.sim_time_s);
            let clearance = flight
                .and_then(|f| f.manifest.as_ref())
                .map(|m| m.summary.min_hull_clearance_m);
            let offset = flight
                .and_then(|f| f.manifest.as_ref())
                .and_then(|m| m.summary.landing.as_ref())
                .map(|landing| landing.touchdown_center_offset_m);
            let preview = flights[index]
                .as_ref()
                .and_then(|(s, f)| {
                    f.ordinary_flight.as_ref().map(|o| {
                        let handoffs = f
                            .segments
                            .iter()
                            .filter(|segment| {
                                segment.kind == crate::WaypointV2SegmentKind::LocalCorrection
                            })
                            .map(|segment| segment.end_state.position_m)
                            .collect::<Vec<_>>();
                        pd_report::build_saved_flight_preview_svg(
                            s,
                            f.manifest.as_ref(),
                            &o.samples,
                            &handoffs,
                        )
                    })
                })
                .unwrap_or_default();
            let cells = format!(
                "<td class=\"tree-label\" style=\"--depth:1\"><a class=\"mission-link\" href=\"runs/{}/index.html\">{} · seed {}</a></td><td>{}<span class=\"row-note\">{}</span></td><td>{}</td><td>{}</td><td>{}</td><td>— · no recorded comparison</td><td><a class=\"run-preview\" href=\"runs/{}/index.html\">{}</a><span class=\"row-note\">{}</span></td><td>{}</td><td>{}</td><td>{}</td>",
                row.attempt_id,
                escape(&row.geometry["recipe_id"].as_str().map_or_else(
                    || row.case_id.clone(),
                    |recipe| format!("{recipe} · {}", row.case_id)
                )),
                row.seed.map_or_else(|| "control".into(), |s| s.to_string()),
                escape(&outcome),
                if projection.is_some() {
                    "Recorded integrity / source replay passed"
                } else {
                    "No verified flight claim"
                },
                metric(fuel, "kg"),
                metric(time, "s"),
                metric(offset, "m"),
                row.attempt_id,
                preview,
                metric(clearance, "m hull"),
                escape(nominal),
                handoffs,
                metric(flight.map(|f| f.timings.planning_s), "s")
            );
            tree_rows.push_str(&batch::render_row(
                Some("seed-row mission-row current-row"),
                &format!(
                    "data-parent=\"{cohort}\" data-case-id=\"{}\" data-depth=\"1\" hidden",
                    row.attempt_id
                ),
                &cells,
            ));
        }
    }
    let overview = batch::render_overview_section(
        "",
        &batch::render_overview_table(
            "<tr><th>Cohort</th><th>Reference</th><th>Scope</th><th>Result</th><th>Timing</th><th>Efficiency</th><th>Reference / recovery</th></tr>",
            &overview_rows,
        ),
    );
    let conditional = conditional_coverage(&survey.rows);
    let coverage = format!(
        "<section class=\"coverage-section\"><div class=\"section-head\"><h2>Coverage</h2></div><p>Initial nominal: {}. Completed corrections: {}. These are observed categories, not selected terrain quotas.</p>{}</section>",
        escape(&survey.summary["nominal_classes"].to_string()),
        escape(&survey.summary["corrections"].to_string()),
        conditional
    );
    let population = survey.rows.iter().filter(|r| r.cohort == "random").count();
    let phase = survey.challenge_phase.as_deref().unwrap_or("sanity survey");
    let setup = format!(
        "{population} frozen cases · {phase} · global Pylander recipes · 1200 m · Earth · 120/60 Hz · 90 s · only local pads"
    );
    let context = batch::render_context_section(
        false,
        false,
        "warn",
        "Development survey",
        &batch::render_context_table(
            "<tr><th>Contract</th><th>Value</th></tr>",
            &batch::render_context_row(&["Frozen setup", &setup]),
        ),
    );
    let diagnostics = format!(
        "<section class=\"diagnostics-section\"><h2>Diagnostics</h2><p>Collection: {}. Planning stops: {}. Dispositions: {}. Sentinels/repeats do not enter the {population}-case denominator.</p><p>Audited cross-source diagnostic exceptions: {}. See Survey JSON for exact paths and values.</p></section>",
        escape(survey.stopped_reason.as_deref().unwrap_or("complete")),
        escape(&survey.summary["planning_stops"].to_string()),
        escape(&survey.summary["dispositions"].to_string()),
        survey
            .rows
            .iter()
            .map(|row| row.comparison_exceptions.len())
            .sum::<usize>()
    );
    let comparison_html = if survey
        .challenge_phase
        .as_deref()
        .is_some_and(|p| p.starts_with("timing-"))
    {
        "<section><h2>Comparison</h2><p>Reuses exact scenarios from the previous 100-case challenge, not fresh held-out worlds. Diagnostics-only motion is compared to the saved baseline excluding additive row diagnostics and three wall timings. Intentional timing changes may select different corrected flights; accepted-pack outcomes are checked separately. Two fixed same-source repeats compare complete non-timing records. The 16-case focus is a diagnostic subset, not a reliability estimate.</p></section>"
    } else if survey
        .sentinel_comparison
        .as_ref()
        .is_some_and(|c| c == &comparison::latest_contract())
    {
        "<section><h2>Comparison</h2><p>No previous random population baseline. Cross-source sentinels keep maneuver states, commands, selection, policy and outcomes exact. Only named body-clearance diagnostics may differ within 1e-12 m without crossing safety thresholds. Each selected proposal content hash and executed segment binding is independently authenticated before derived identities are compared. Every exception is recorded. Same-source repeats remain exact except the three flight wall timings. Runtime safety and proof checks are unchanged.</p></section>"
    } else if survey.sentinel_comparison.is_some() {
        "<section><h2>Comparison</h2><p>No previous random population baseline. Cross-source sentinels keep all non-timing flight fields exact except two named geometry clearance diagnostics within 1e-12 m, without changing metadata or safety decisions. Every exception is recorded. Same-source repeats remain exact except the three flight wall timings. Execution safety thresholds are unchanged.</p></section>"
    } else {
        "<section><h2>Comparison</h2><p>No previous random population baseline. Three sentinels and five predetermined repeats compare full flight records, excluding only the three flight wall timings.</p></section>"
    };
    let tree = batch::render_review_tree_section(
        "",
        "<p>Random cases, preservation controls and repeats are separate branches. H = completed executed handoff.</p>",
        &batch::render_review_tree_table("survey", headers, &tree_rows),
    );
    let title = match survey.challenge_phase.as_deref() {
        Some("timing-diagnostics") => "Planner V2 · timing diagnostics baseline",
        Some("timing-focus") => "Planner V2 · intervention timing focus",
        Some("timing-challenge") => "Planner V2 · intervention timing recheck",
        Some("calibration") => "Planner V2 · harder terrain calibration",
        Some(_) => "Planner V2 · harder terrain challenge",
        None => "Planner V2 · random procedural terrain",
    };
    let policy_caption = if phase.starts_with("timing-") {
        "Policy 3 · source-bound timing pass"
    } else {
        "Policy 3 unchanged"
    };
    let chips = format!(
        "<span class=\"chip\">{population} frozen cases · {phase}</span><span class=\"chip\">{policy_caption}</span>"
    );
    batch::render_batch_page(batch::BatchPage {
        title,
        subtitle: "Exploratory development coverage — not the accepted benchmark or a universal reliability claim",
        chips_html: &chips,
        actions_html: &format!(
            "<a href=\"{base}survey.json\">Survey JSON</a><a href=\"{base}manifest.json\">Frozen inputs</a><a href=\"{base}receipt.json\">Receipt</a>"
        ),
        before_hero_html: "<nav><a href=\"/reports/\">Report home</a> · <a href=\"/reports/topics/waypoint-planning/index.html\">Waypoint planning</a> · <a href=\"/reports/eval/planner_v2_lab_suite/\">Accepted benchmark</a></nav>",
        after_hero_html: "",
        overview_html: &overview,
        planner_html: "",
        coverage_html: &coverage,
        context_html: &context,
        diagnostics_html: &diagnostics,
        review_tree_html: &tree,
        comparison_html,
        appendix_html: "",
        body_class: "planner-v2-common",
        tree: batch::BatchTreeOptions {
            max_depth: 1,
            depth_by_kind: &[("cohort", 0)],
            leaf_selector: "tr.seed-row",
            default_expansion: 0,
        },
        coverage_tree_jump: false,
    })
}

fn validate_report_target(repo: &Path, output: &Path) -> Result<()> {
    let outputs = repo.join("outputs");
    let expected = outputs.join("reports/eval/planner_v2_random_terrain");
    let named_child = output.parent() == Some(expected.as_path())
        && output
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|name| {
                name.strip_prefix("recheck-").is_some_and(|id| {
                    !id.is_empty()
                        && id
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                })
            });
    ensure!(
        output.is_absolute() && (output == expected || named_child),
        "unexpected survey report target"
    );
    let metadata = fs::symlink_metadata(&outputs)?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "unsafe outputs root"
    );
    let mut path = outputs.clone();
    for part in output.strip_prefix(&outputs)?.components() {
        ensure!(matches!(part, Component::Normal(_)), "unsafe report path");
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                ensure!(!metadata.file_type().is_symlink(), "symlink report path");
                ensure!(
                    path != output,
                    "create-only survey report target already exists"
                );
                ensure!(metadata.is_dir(), "report ancestor is not a directory");
            }
            Err(error) if path == output && error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("checking survey report target"),
        }
    }
    Ok(())
}

/// Create a separate common-template report collection from authenticated saved
/// records. Never executes/replays a flight, changes selectors, or overwrites pages.
pub fn render_saved_survey(capture: &Path, output: &Path, base: &str) -> Result<()> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("repo root")?;
    validate_report_target(repo, output)?;
    ensure!(
        base.starts_with("/eval/planner_v2_random_terrain/")
            && base.ends_with('/')
            && base.split('/').all(|p| p.is_empty()
                || p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')),
        "unsafe survey capture URL"
    );
    ensure!(
        !fs::symlink_metadata(capture)?.file_type().is_symlink(),
        "symlink capture root"
    );
    let (survey, flights) = validate(capture)?;
    // Restrict publication to the separate served development subtree.
    let expected_capture = repo.join("outputs").join(base.trim_start_matches('/'));
    ensure!(
        capture.canonicalize()? == expected_capture.canonicalize()?,
        "capture URL differs from source directory"
    );
    ensure!(
        output
            .parent()
            .context("site parent")?
            .canonicalize()?
            .starts_with(repo.join("outputs").canonicalize()?),
        "unsafe report parent"
    );
    reserve_output_root(output)?;
    let write = |path: &Path, contents: &str| {
        write_bytes_create_only_with_context(path, contents.as_bytes(), "create survey report")
    };
    for (index, row) in survey.rows.iter().enumerate() {
        let dir = output.join("runs").join(&row.attempt_id);
        fs::create_dir_all(&dir)?;
        let link = |label: &str, href: String| NavigationLink {
            label: label.into(),
            href,
        };
        let navigation = AnnotationNavigation {
            home: Some(link("Report home", "/reports/".into())),
            collection: Some(link("Survey summary", "../../index.html".into())),
            previous: index.checked_sub(1).map(|i| {
                link(
                    "Previous case",
                    format!("../{}/index.html", survey.rows[i].attempt_id),
                )
            }),
            next: survey
                .rows
                .get(index + 1)
                .map(|r| link("Next case", format!("../{}/index.html", r.attempt_id))),
            source_links: vec![
                link("Scenario JSON", format!("{base}{}", row.scenario_path)),
                link("Survey JSON", format!("{base}survey.json")),
                link(
                    "Flight JSON",
                    format!("{base}runs/{}/flight.json", row.attempt_id),
                ),
            ],
        };
        let html = if let Some((scenario, result)) = &flights[index] {
            render_rich_flight(
                scenario,
                result,
                navigation,
                format!(
                    "Procedural terrain · {} · {} · {} · {} · development",
                    survey.challenge_phase.as_deref().unwrap_or("sanity survey"),
                    row.geometry["recipe_id"]
                        .as_str()
                        .unwrap_or("control / refined"),
                    row.cohort,
                    row.attempt_id
                ),
            )?
        } else {
            let mut links = vec![
                format!(
                    "<a href=\"{base}{}\">Scenario JSON</a>",
                    escape(&row.scenario_path)
                ),
                format!("<a href=\"{base}survey.json\">Survey JSON</a>"),
            ];
            for (relative, label) in [
                (
                    format!("runs/{}/flight.json", row.attempt_id),
                    "Unverified flight JSON",
                ),
                (
                    format!("runs/{}/summary.json", row.attempt_id),
                    "Native summary JSON",
                ),
                (format!("logs/{}.stdout", row.attempt_id), "Native stdout"),
                (format!("logs/{}.stderr", row.attempt_id), "Native stderr"),
            ] {
                if capture.join(&relative).is_file() {
                    links.push(format!("<a href=\"{base}{relative}\">{label}</a>"));
                }
            }
            batch::render_status_page(
                &row.attempt_id,
                "<nav><a href=\"../../index.html\">Survey summary</a> · <a href=\"/reports/\">Report home</a></nav>",
                &format!(
                    "<p>{}: {}. No verified flight or landing is claimed.</p><p>{}</p>",
                    escape(&row.status),
                    escape(row.failure.as_deref().unwrap_or("No completed record")),
                    links.join(" · ")
                ),
            )
        };
        write(&dir.join("index.html"), &html)?;
    }
    write(
        &output.join("index.html"),
        &batch_page(&survey, &flights, base),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct TestRepo(PathBuf);

    impl TestRepo {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "pd-survey-report-target-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::create_dir_all(path.join("outputs/reports/eval")).unwrap();
            Self(path)
        }

        fn site(&self) -> PathBuf {
            self.0
                .join("outputs/reports/eval/planner_v2_random_terrain")
        }
    }

    impl Drop for TestRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn report_target_accepts_only_a_new_root_or_one_named_recheck_child() {
        let repo = TestRepo::new();
        let root = repo.site();
        assert!(validate_report_target(&repo.0, &root).is_ok());
        assert!(!root.exists());
        fs::create_dir(&root).unwrap();
        fs::write(root.join("index.html"), b"preserved stopped report").unwrap();
        let child = root.join("recheck-synthetic_01");
        assert!(validate_report_target(&repo.0, &child).is_ok());
        assert!(!child.exists());
        for target in [
            root.clone(),
            root.join("recheck-"),
            root.join("other"),
            root.join("recheck-ok/nested"),
            root.join("../planner_v2_lab_suite"),
            root.join("recheck-<script>"),
            PathBuf::from("outputs/reports/eval/planner_v2_random_terrain"),
        ] {
            assert!(
                validate_report_target(&repo.0, &target).is_err(),
                "{}",
                target.display()
            );
        }
        fs::create_dir(&child).unwrap();
        assert!(validate_report_target(&repo.0, &child).is_err());
        assert_eq!(
            fs::read(root.join("index.html")).unwrap(),
            b"preserved stopped report"
        );
    }

    #[test]
    #[cfg(unix)]
    fn report_target_rejects_symlink_ancestors_and_dangling_destinations() {
        use std::os::unix::fs::symlink;
        let repo = TestRepo::new();
        let root = repo.site();
        let elsewhere = repo.0.join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();
        symlink(&elsewhere, &root).unwrap();
        assert!(validate_report_target(&repo.0, &root.join("recheck-safe")).is_err());
        assert!(!elsewhere.join("recheck-safe").exists());
        fs::remove_file(&root).unwrap();
        fs::create_dir(&root).unwrap();
        let dangling = root.join("recheck-dangling");
        symlink(repo.0.join("absent"), &dangling).unwrap();
        assert!(validate_report_target(&repo.0, &dangling).is_err());

        let reports = repo.0.join("outputs/reports");
        let real_reports = repo.0.join("reports-real");
        fs::rename(&reports, &real_reports).unwrap();
        symlink(&real_reports, &reports).unwrap();
        assert!(validate_report_target(&repo.0, &root.join("recheck-safe")).is_err());
        assert!(
            !real_reports
                .join("eval/planner_v2_random_terrain/recheck-safe")
                .exists()
        );
    }

    #[test]
    fn missing_nominal_is_not_classified_from_default_false() {
        let p: Projection = serde_json::from_value(json!({"nominal_class":"not_established", "verified_landing":false,
            "planning_stop":"no_nominal", "physical_outcome":"flying", "mission_outcome":"in_progress",
            "correction_count":0, "integrity_passed":true, "final_source_replay_passed":true})).unwrap();
        let row: Attempt = serde_json::from_value(
            json!({"attempt_id":"random-000", "case_id":"random-000", "cohort":"random", "seed":5,
            "scenario_path":"scenarios/random-000.json", "status":"recorded", "result":p}),
        )
        .unwrap();
        let total = totals(&[row]);
        assert_eq!(total["nominal_classes"], json!({"not_established":1}));
        assert_eq!(total["verified_landings"], 0);
    }

    #[test]
    fn separate_cohorts_and_incomplete_cases_keep_truthful_denominators() {
        let row = |cohort: &str| {
            serde_json::from_value::<Attempt>(
                json!({"attempt_id":"x", "case_id":"x", "cohort":cohort,
            "seed":null, "scenario_path":"x.json", "status":"not_attempted"}),
            )
            .unwrap()
        };
        let total = totals(&[row("random"), row("random"), row("sentinel"), row("repeat")]);
        assert_eq!(total["primary_count"], 2);
        assert_eq!(total["recorded_count"], 0);
        assert_eq!(total["dispositions"], json!({"not_attempted":2}));
    }

    #[test]
    fn traversal_is_rejected_before_reading_an_artifact() {
        assert!(safe_file(Path::new("."), "../Cargo.toml").is_err());
        assert!(safe_file(Path::new("."), "/etc/passwd").is_err());
        assert!(safe_file(Path::new("."), "").is_err());
        assert_eq!(escape("<script>\"&"), "&lt;script&gt;&quot;&amp;");
    }

    #[test]
    fn survey_batch_keeps_common_sections_tree_navigation_and_honest_absence() {
        let row: Attempt = serde_json::from_value(
            json!({"attempt_id":"random-000", "case_id":"random-000", "cohort":"random",
            "seed":5, "scenario_path":"scenarios/random-000.json", "status":"not_attempted"}),
        )
        .unwrap();
        let summary = totals(std::slice::from_ref(&row));
        let survey = Survey {
            schema: "synthetic_unit_only".into(),
            manifest_sha256: String::new(),
            source_before: Value::Null,
            source_after: Value::Null,
            stopped_reason: Some("synthetic stop".into()),
            rows: vec![row],
            summary,
            sentinel_comparison: None,
            challenge_phase: None,
        };
        let html = batch_page(
            &survey,
            &[None],
            "/eval/planner_v2_random_terrain/synthetic/",
        );
        assert!(html.contains("data-batch-template=\"common-v1\""));
        for section in [
            "Overview",
            "Coverage",
            "Context",
            "Diagnostics",
            "Review Tree",
            "Comparison",
        ] {
            assert!(html.contains(section));
        }
        assert!(
            html.contains("data-group=\"random\"") && html.contains("runs/random-000/index.html")
        );
        assert!(html.contains("not classified") && html.contains("No verified flight claim"));
        assert!(html.contains("/reports/topics/waypoint-planning/index.html"));
        assert!(html.contains("0 verified target landings · 0 attempted · 1 unattempted"));
        assert!(!html.contains("-0.000 s total planning") && !html.contains("0.000 kg total fuel"));
    }

    #[test]
    fn partial_survey_summary_distinguishes_attempts_from_frozen_population() {
        let make = |id, status, result: Value| {
            serde_json::from_value::<Attempt>(json!({
                "attempt_id":id, "case_id":id, "cohort":"random", "seed":5,
                "scenario_path":"scenario.json", "status":status, "result":result
            }))
            .unwrap()
        };
        let rows = vec![
            make(
                "random-000",
                "recorded",
                json!({"nominal_class":"clear", "verified_landing":true,
            "planning_stop":"landed", "physical_outcome":"landed_on_target", "mission_outcome":"success",
            "correction_count":0, "integrity_passed":true, "final_source_replay_passed":true}),
            ),
            make("random-001", "runner_error", Value::Null),
            make("random-002", "not_attempted", Value::Null),
        ];
        let survey = Survey {
            schema: "synthetic_only".into(),
            manifest_sha256: String::new(),
            source_before: Value::Null,
            source_after: Value::Null,
            stopped_reason: Some("synthetic stop".into()),
            summary: totals(&rows),
            rows,
            sentinel_comparison: None,
            challenge_phase: None,
        };
        let html = batch_page(
            &survey,
            &[None, None, None],
            "/eval/planner_v2_random_terrain/synthetic/",
        );
        assert!(html.contains("1 verified target landings · 2 attempted · 1 unattempted"));
        assert!(!html.contains("1/3 verified target landings"));
    }
}
