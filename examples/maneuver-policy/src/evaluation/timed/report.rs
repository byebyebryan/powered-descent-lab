//! Display-only projection of frozen timed results into the established common
//! mission tree and rich detail templates. No agent/native replay is executed.
use super::{
    TimedAttemptResultV2, TimedEvaluationSummaryV2, check_timed_evaluation, check_timed_results,
};
use crate::evaluation::{
    BoundFileV1, bind_file, read_bound_json, read_json, tree_bytes, write_limited,
};
use pd_core::ScenarioSpec;
use pd_eval::EpisodeEvaluationV1;
use pd_report::{batch, campaign_view, detail_view, paired_review::*};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedReportPopulationV2 {
    pub label: String,
    pub plan: PathBuf,
    pub results: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedReportSpecV2 {
    pub schema_version: u32,
    pub title: String,
    pub populations: Vec<TimedReportPopulationV2>,
    pub historical_regression: Option<PathBuf>,
    pub historical_inventory: Option<PathBuf>,
    pub max_report_bytes: u64,
}
struct Mission {
    population: String,
    role: String,
    stratum: String,
    id: String,
    scenario: ScenarioSpec,
    physical: String,
    evaluation: Option<EpisodeEvaluationV1>,
    source_evaluation: Option<BoundFileV1>,
    outcome: String,
    proof: String,
    note: String,
}
struct Builder<'a> {
    output: &'a Path,
    max_bytes: u64,
    review: PairedReviewV1,
    branch_ids: BTreeMap<(String, String), String>,
    group_ids: BTreeMap<String, String>,
    rich_views: usize,
}
fn esc(value: &str) -> String {
    batch::escape_html(value)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}
impl Builder<'_> {
    fn add(&mut self, item: Mission) -> Result<(), String> {
        if tree_bytes(self.output)?.saturating_add(32 * 1024 * 1024) > self.max_bytes {
            return Err("report reservation exhausted; partial view retained".into());
        }
        let ordinal = self.review.missions.len();
        let mission_id = format!("mission-{ordinal}");
        let mut parent = None;
        let mut path = vec![];
        for (depth, (kind, label)) in [
            ("family", &item.population),
            ("morphology", &item.role),
            ("band", &item.stratum),
        ]
        .into_iter()
        .enumerate()
        {
            let key = (
                kind.to_string(),
                format!("{}::{label}", parent.as_deref().unwrap_or("root")),
            );
            let branch = if let Some(id) = self.branch_ids.get(&key) {
                id.clone()
            } else {
                let id = format!("{kind}-{}", self.review.branches.len());
                self.review.branches.push(MissionBranchV1 {
                    id: id.clone(),
                    parent: parent.clone(),
                    label: label.clone(),
                    depth,
                    kind: kind.into(),
                });
                self.branch_ids.insert(key, id.clone());
                id
            };
            parent = Some(branch.clone());
            path.push(branch);
        }
        let branch = parent.ok_or("report branch missing")?;
        let group = if let Some(id) = self.group_ids.get(&branch) {
            id.clone()
        } else {
            let id = format!("coverage-{}", self.review.groups.len());
            self.group_ids.insert(branch.clone(), id.clone());
            self.review.groups.push(PairedGroupV1 {
                id: id.clone(),
                label: format!("{} · {} · {}", item.population, item.role, item.stratum),
                branch,
                direction: 0,
                direction_unavailable: false,
            });
            id
        };
        let mut href = None;
        let mut preview = None;
        let mut endpoint = "No native flight allocated".to_string();
        if let Some(evaluation) = &item.evaluation {
            let file = format!("flight-{ordinal}.html");
            let svg_file = format!("flight-{ordinal}.svg");
            let episode = &evaluation.execution.episode;
            let raw = pd_report::render_agent_run_report(
                &evaluation.scenario,
                &evaluation.execution,
                pd_report::AgentReportEvidenceV1 {
                    integrity_passed: evaluation.integrity_passed,
                    physical_prefix_verified: evaluation.replay.physical_prefix_verified,
                    complete_episode_verified: evaluation.replay.complete_episode_verified,
                    verified_target_landing: evaluation.verified_target_landing,
                },
                pd_report::agent_capture_navigation(),
            )
            .map_err(|e| e.to_string())?;
            let svg = pd_report::build_saved_flight_preview_svg(
                &evaluation.scenario,
                Some(&episode.run.manifest),
                &episode.run.samples,
                &[],
            );
            let metrics = detail_view::metric_fallback(&episode.run.samples);
            let html = detail_view::with_static_fallback(&raw, &svg, &metrics, &item.note)
                .map_err(|e| e.to_string())?;
            let nav = format!(
                "<nav class=\"review-detail-nav\" aria-label=\"Mission navigation\"><a href=\"report.html#{mission_id}\" data-back-review>Back to mission tree</a><a href=\"#\" data-review-previous hidden>Previous matched mission</a><a href=\"#\" data-review-next hidden>Next matched mission</a></nav>"
            );
            let outcome = format!(
                "<h1>{}</h1><p><strong>{}</strong> · {}</p><p>{}</p><p>{}</p><p><a href=\"flight-{ordinal}-evidence.json\">Saved native proof, raw contact and source bindings</a></p>",
                esc(&item.id),
                esc(&item.outcome),
                esc(&item.proof),
                esc(&item.note),
                esc(&format!("Incoming contact: {:?}", episode.incoming_contact))
            );
            let html = detail_view::with_review_context(
                &html,
                &nav,
                &outcome,
                "/poly-export/",
                &mission_id,
                0,
                evaluation.verified_target_landing,
            )
            .map_err(|e| e.to_string())?
            .replace("/poly-export/", "./")
            .replace(
                "new URL(body.dataset.reviewRoot,location.origin)",
                "new URL(body.dataset.reviewRoot,location.href)",
            )
            .replace(
                "href=\"evaluation.json\"",
                &format!("href=\"flight-{ordinal}-evidence.json\""),
            )
            .replace(
                "href=\"seal.json\"",
                &format!("href=\"flight-{ordinal}-evidence.json\""),
            );
            write_new(&self.output.join(&file), html.as_bytes())?;
            write_new(&self.output.join(&svg_file), svg.as_bytes())?;
            write_limited(
                &self.output.join(format!("flight-{ordinal}-evidence.json")),
                &serde_json::json!({
                "physical_sha256":evaluation.physical_sha256,"descriptor":evaluation.execution.descriptor,"source_evaluation":item.source_evaluation,
                    "evidence_before":evaluation.evidence_before,"evidence_after":evaluation.evidence_after,
                    "integrity_passed":evaluation.integrity_passed,"replay":evaluation.replay,"verified_target_landing":evaluation.verified_target_landing,
                    "manifest":episode.run.manifest,"incoming_contact":episode.incoming_contact,
                    "note":item.note,"new_physics_steps":0,"new_replays":0,
                }),
                16 * 1024 * 1024,
            )?;
            href = Some(file);
            preview = Some(svg_file);
            self.rich_views += 1;
            endpoint = format!(
                "{:.3} s · {:.2} kg fuel remaining",
                episode.final_state.sim_time_s, episode.final_state.fuel_kg
            );
        }
        let cells = format!(
            "<td><strong>{}</strong></td><td>{}<span class=\"row-note\">{}</span></td><td>{}</td>",
            esc(&item.outcome),
            esc(&endpoint),
            esc(&item.note),
            esc(&item.proof)
        );
        let dx = item
            .scenario
            .world
            .landing_pad(item.scenario.mission.goal.target_pad_id())
            .map_or(0.0, |p| {
                p.center_x_m - item.scenario.initial_state.position_m.x
            });
        self.review.missions.push(PairedMissionV1 {
            id: mission_id,
            group,
            path,
            direction: if dx < 0.0 { -1 } else { 1 },
            direction_unavailable: false,
            label: item.id,
            mission_index: ordinal as u64,
            physical_sha256: item.physical,
            occurrence: 0,
            attempts: vec![PairedAttemptV1 {
                approach: 0,
                attempt_index: ordinal as u64,
                outcome: item.outcome,
                cells,
                href,
                preview,
                caption: item.note,
            }],
        });
        Ok(())
    }
}

pub fn render_timed_evaluation_report(spec_path: &Path, output: &Path) -> Result<(), String> {
    let spec: TimedReportSpecV2 = read_json(spec_path, 1024 * 1024)?;
    if spec.schema_version != 2
        || spec.max_report_bytes < 64 * 1024 * 1024
        || spec.max_report_bytes > 6 * 1024 * 1024 * 1024
    {
        return Err("invalid bounded timed report specification".into());
    }
    crate::verify::no_symlinks(output.parent().ok_or("report needs a parent")?)?;
    std::fs::create_dir(output).map_err(|e| e.to_string())?;
    let mut builder = Builder {
        output,
        max_bytes: spec.max_report_bytes,
        review: PairedReviewV1 {
            schema_version: 1,
            labels: vec!["Poly Landing · timed input".into()],
            groups: vec![],
            branches: vec![],
            missions: vec![],
        },
        branch_ids: BTreeMap::new(),
        group_ids: BTreeMap::new(),
        rich_views: 0,
    };
    let mut summaries: Vec<(String, TimedEvaluationSummaryV2)> = vec![];
    let mut input_files = vec![bind_file(spec_path, 1024 * 1024)?];
    for population in &spec.populations {
        let plan = check_timed_evaluation(&population.plan)?;
        let summary = check_timed_results(&population.plan, &population.results)?;
        input_files.push(bind_file(
            &population.plan.join("plan.json"),
            32 * 1024 * 1024,
        )?);
        input_files.push(bind_file(
            &population.results.join("summary.json"),
            8 * 1024 * 1024,
        )?);
        for (index, case) in plan.cases.iter().enumerate() {
            let scenario: ScenarioSpec = read_bound_json(&case.scenario, 1024 * 1024)?;
            let row = summary.rows.get(index).ok_or("report slot missing")?;
            let item: Option<TimedAttemptResultV2> = row
                .result
                .as_ref()
                .map(|f| read_bound_json(f, 1024 * 1024))
                .transpose()?;
            let mut evaluation = None;
            let mut source_evaluation = None;
            let mut outcome = "not-launched".to_string();
            let mut proof = row.reason.clone().unwrap_or_else(|| "Not started".into());
            let mut note = format!("{} · {}", case.role, case.stratum);
            if let Some(item) = item {
                if item.disposition == "unsupported" {
                    outcome = "unsupported".into();
                    proof = item.support;
                } else if let Some(file) = item
                    .payload_files
                    .iter()
                    .find(|f| f.path.ends_with("/capture/evaluation.json"))
                {
                    let saved: EpisodeEvaluationV1 = read_bound_json(file, 64 * 1024 * 1024)?;
                    outcome = if item.verified_target_landing {
                        "landed"
                    } else {
                        match saved.execution.episode.run.manifest.physical_outcome {
                            pd_core::PhysicalOutcome::Crashed => "crashed",
                            pd_core::PhysicalOutcome::TimedOut => "timed-out",
                            pd_core::PhysicalOutcome::LandedOffTarget => "off-target",
                            _ => "stopped",
                        }
                    }
                    .into();
                    proof = if item.verification.as_ref().is_some_and(|v| v.passed()) {
                        "Saved native replay + controller/planner reconstruction passed"
                    } else {
                        "Saved proof incomplete"
                    }
                    .into();
                    if let Some(shape) = item.shape {
                        note = format!(
                            "{note} · burn shape {} · max flying vy {:.6} m/s · {} actual unpowered ticks · contact residual {:?} ticks",
                            if shape.passed { "passed" } else { "failed" },
                            shape.maximum_flying_vertical_speed_mps,
                            shape.actual_unpowered_physics_steps,
                            shape.contact_time_residual_ticks
                        );
                    }
                    evaluation = Some(saved);
                    source_evaluation = Some(file.clone());
                }
            }
            builder.add(Mission {
                population: population.label.clone(),
                role: case.role.clone(),
                stratum: case.stratum.clone(),
                id: case.id.clone(),
                scenario,
                physical: case.physical_sha256.clone(),
                evaluation,
                source_evaluation,
                outcome,
                proof,
                note,
            })?;
        }
        summaries.push((population.label.clone(), summary));
    }
    if let (Some(regression), Some(inventory_path)) =
        (&spec.historical_regression, &spec.historical_inventory)
    {
        add_historical(&mut builder, regression, inventory_path)?;
        input_files.push(bind_file(&regression.join("summary.json"), 1024 * 1024)?);
        input_files.push(bind_file(inventory_path, 1024 * 1024)?);
    }
    let controls=controls(&builder.review)
        .replace("Complete procedural transfers → terrain family → measured morphology → chord-barrier band → mission. Controller results sit inside each mission.","Poly Landing → population → entry/timing group → mission. Saved flights sit inside each mission.")
        .replace("one terrain band","one entry group").replace("terrain groups","entry groups").replace("Expand families","Expand populations");
    let tree = batch::render_review_tree_section(
        "",
        &controls,
        &batch::render_table(
            "scenario-table paired-table",
            "data-tree-table=\"paired-missions\"",
            "<th>Mission / saved flight</th><th>Outcome / stop</th><th>Flight / endpoint</th><th>Evidence</th>",
            "",
        ),
    );
    let overview = summary_html(&summaries);
    let html = batch::render_review_first_batch_page(batch::BatchPage {
        title: &spec.title,
        subtitle: "Fixed-reference landing burns; flat prepared timed-entry qualification, with historical preservation kept separate",
        chips_html: &format!(
            "<span class=\"chip\">{} mission occurrences · {} complete rich flight views</span>",
            builder.review.missions.len(),
            builder.rich_views
        ),
        actions_html: "<a href=\"#review-tree-root\">Browse missions</a><a href=\"report-input.json\">Complete report projection</a><a href=\"receipt.json\">Report receipt</a>",
        before_hero_html: &format!("<style>{}</style>", campaign_view::STYLE),
        after_hero_html: "",
        overview_html: &overview,
        planner_html: "",
        coverage_html: "",
        context_html: "",
        diagnostics_html: "",
        review_tree_html: &tree,
        comparison_html: "",
        appendix_html: &format!(
            "<script>{}</script>{}",
            campaign_view::SCRIPT,
            data_script(&builder.review)?
        ),
        body_class: "procedural-comparison campaign-view",
        tree: batch::BatchTreeOptions {
            max_depth: 0,
            depth_by_kind: &[],
            leaf_selector: "tr.paired-attempt-row",
            default_expansion: 0,
        },
        coverage_tree_jump: false,
    });
    write_new(&output.join("report.html"), html.as_bytes())?;
    write_limited(
        &output.join("report-input.json"),
        &builder.review,
        32 * 1024 * 1024,
    )?;
    let files = std::fs::read_dir(output)
        .map_err(|e| e.to_string())?
        .map(|p| p.map(|p| p.path()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut bound = Vec::new();
    for path in files {
        bound.push(bind_file(&path, 64 * 1024 * 1024)?);
    }
    write_limited(
        &output.join("receipt.json"),
        &serde_json::json!({"schema_version":2,"input_files":input_files,"output_files":bound,
        "mission_occurrences":builder.review.missions.len(),"rich_views":builder.rich_views,"new_physics_steps":0,"new_replays":0,
        "templates":"pd-report common batch, paired mission navigation and complete rich agent detail"}),
        4 * 1024 * 1024,
    )?;
    if tree_bytes(output)? > spec.max_report_bytes {
        return Err("report byte ceiling exceeded; all files retained".into());
    }
    Ok(())
}

fn summary_html(summaries: &[(String, TimedEvaluationSummaryV2)]) -> String {
    let mut rows = String::new();
    for (label, s) in summaries {
        rows.push_str(&format!("<tr><td>{}</td><td>{}/{}</td><td>{}</td><td>{}/{}</td><td>{} ({} unsupported)</td><td>{}</td></tr>",esc(label),s.verified_primary_landings,s.primary_originals,s.primary_shape_failures,s.verified_transform_landings,s.transforms,s.diagnostics,s.diagnostic_unsupported,s.confirmation_lower_95.map_or_else(||"Not a fresh reliability estimate".into(),|v|format!("95% lower bound {:.5}% · {}",v*100.0,if s.qualified {"passes declared criterion"} else {"does not pass"}))));
    }
    format!(
        "<section class=\"overview-section\"><h2>Declared populations and outcomes</h2><p>Planner owns coast and endpoint time; activation starts the finite burn. New local cases use flat ground and prepared attitude. These are reference-defined conditional tasks, not arbitrary airborne states or whole-mission reliability. Historical, development, transformations and diagnostics have separate denominators.</p><div class=\"table-wrap\"><table><thead><tr><th>Population</th><th>Verified primary landings</th><th>Primary shape failures</th><th>Transform landings + shape</th><th>Diagnostics</th><th>Fresh criterion</th></tr></thead><tbody>{rows}</tbody></table></div></section>"
    )
}

fn add_historical(
    builder: &mut Builder<'_>,
    root: &Path,
    inventory_path: &Path,
) -> Result<(), String> {
    let summary: serde_json::Value = read_json(&root.join("summary.json"), 1024 * 1024)?;
    let inventory: serde_json::Value = read_json(inventory_path, 1024 * 1024)?;
    if summary["passed"] != true || summary["exact_burns"] != 38 {
        return Err("historical preservation is not complete".into());
    }
    let capture = PathBuf::from(
        inventory["capture"]
            .as_str()
            .ok_or("historical capture missing")?,
    );
    for value in inventory["cases"]
        .as_array()
        .ok_or("historical inventory missing")?
    {
        let id = value["case_id"].as_str().ok_or("historical ID missing")?;
        let scenario: ScenarioSpec = read_json(
            &capture.join("runs").join(id).join("scenario.json"),
            16 * 1024 * 1024,
        )?;
        let mut evaluation = None;
        let mut source_evaluation = None;
        let mut outcome = "not-launched".into();
        let mut proof =
            "Original historical nonlanding/unsupported role; no finite burn allocated".to_string();
        if let Some(row) = summary["slots"]
            .as_array()
            .ok_or("historical slots missing")?
            .iter()
            .find(|r| r["case_id"] == id && r["status"] == "completed")
        {
            let directory =
                PathBuf::from(row["output"].as_str().ok_or("historical output missing")?);
            let receipt: crate::poly::verify::TimedBundleReceiptV2 =
                read_json(&directory.join("bundle/receipt.json"), 32 * 1024 * 1024)?;
            let file = bind_file(
                &directory.join("bundle/capture/evaluation.json"),
                64 * 1024 * 1024,
            )?;
            if file.sha256 != receipt.evaluation_sha256 || row["parity"]["passed"] != true {
                return Err("historical sealed parity binding differs".into());
            }
            evaluation = Some(read_bound_json(&file, 64 * 1024 * 1024)?);
            source_evaluation = Some(file);
            outcome = "landed".into();
            proof="Bit-exact original commands, samples, contact and final state; saved replay/reconstructions passed".into();
        }
        let role = if id.starts_with("v2_diag_") {
            "historical diagnostic"
        } else {
            "historical core"
        };
        builder.add(Mission {population:"Historical preservation · October 5".into(),role:role.into(),stratum:if value["flat_scene"]==true {"Flat source mission"} else {"Original source scene"}.into(),
            id:id.into(),physical:pd_missions::physical_identity(&scenario).map_err(|e|e.to_string())?,scenario,evaluation,source_evaluation,outcome,proof,
            note:"Original prefix and finite terminal burn preserved. Historical source epoch; no new local reliability denominator.".into()})?;
    }
    Ok(())
}
