//! Display-only projection into the maintained mission tree and rich views.
use super::{decode, error, write_new};
use pd_eval::EpisodeEvaluationV1;
use pd_report::{batch, campaign_view, detail_view, paired_review::*};
use pyo3::prelude::*;
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
struct Spec {
    title: String,
    summary: String,
    rows: Vec<Row>,
    #[serde(default)]
    grouped: bool,
    #[serde(default)]
    focus_flat: bool,
    max_bytes: Option<u64>,
    overview_html: Option<String>,
}
#[derive(Deserialize)]
struct Row {
    capture: String,
    population: String,
    label: String,
    context_html: Option<String>,
}

#[pyfunction]
pub fn render_report(spec_json: &str, output: &str) -> PyResult<()> {
    let spec: Spec = decode(spec_json)?;
    let output = Path::new(output);
    std::fs::create_dir(output).map_err(error)?;
    let mut review = PairedReviewV1 {
        schema_version: 1,
        labels: vec!["Saved policy execution".into()],
        groups: vec![],
        branches: vec![],
        missions: vec![],
    };
    let mut groups: BTreeMap<String, String> = BTreeMap::new();
    let mut mission_ids: BTreeMap<(String, String), usize> = BTreeMap::new();
    if spec.grouped {
        review.labels.clear();
        for row in &spec.rows {
            if !review.labels.contains(&row.label) {
                review.labels.push(row.label.clone());
            }
        }
    }
    let max_bytes = spec.max_bytes.unwrap_or(4 * 1024 * 1024 * 1024);
    let mut written_bytes = 0_u64;
    for (index, row) in spec.rows.iter().enumerate() {
        let root = Path::new(&row.capture);
        let bytes = std::fs::read(root.join("evaluation.json")).map_err(error)?;
        let seal: serde_json::Value =
            decode(&std::fs::read_to_string(root.join("seal.json")).map_err(error)?)?;
        let proof: serde_json::Value =
            decode(&std::fs::read_to_string(root.join("proof.json")).map_err(error)?)?;
        let hash = pd_missions::sha256(&bytes);
        if seal["evaluation_sha256"] != hash
            || proof["capture_sha256"] != hash
            || proof["physical_replay"] != true
            || proof["controller_reconstruction"] != true
            || proof["planner_reconstruction"] != true
        {
            return Err(error("report requires a bound complete proof"));
        }
        let e: EpisodeEvaluationV1 = serde_json::from_slice(&bytes).map_err(error)?;
        if !e.integrity_passed || !e.replay.complete_episode_verified {
            return Err(error("incomplete capture evidence"));
        }
        let group = if let Some(g) = groups.get(&row.population) {
            g.clone()
        } else {
            let g = format!("population-{}", groups.len());
            review.branches.push(MissionBranchV1 {
                id: g.clone(),
                parent: None,
                label: row.population.clone(),
                depth: 0,
                kind: "family".into(),
            });
            review.groups.push(PairedGroupV1 {
                id: g.clone(),
                label: row.population.clone(),
                branch: g.clone(),
                direction: 0,
                direction_unavailable: false,
            });
            groups.insert(row.population.clone(), g.clone());
            g
        };
        let mission_index = if spec.grouped {
            let next = review.missions.len();
            *mission_ids
                .entry((group.clone(), e.physical_sha256.clone()))
                .or_insert(next)
        } else {
            index
        };
        let approach = if spec.grouped {
            review
                .labels
                .iter()
                .position(|label| label == &row.label)
                .unwrap()
        } else {
            0
        };
        if written_bytes + 32 * 1024 * 1024 > max_bytes {
            return Err(error("report byte reservation exhausted"));
        }
        let name = format!("flight-{index}");
        let manifest = &e.execution.episode.run.manifest;
        let outcome = if e.verified_target_landing {
            "landed"
        } else {
            match manifest.physical_outcome {
                pd_core::PhysicalOutcome::Crashed => "crashed",
                pd_core::PhysicalOutcome::LandedOffTarget => "off-target",
                _ => "stopped",
            }
        };
        let info: serde_json::Value =
            decode(&std::fs::read_to_string(root.join("decisions.json")).map_err(error)?)?;
        if pd_missions::sha256(&std::fs::read(root.join("decisions.json")).map_err(error)?)
            != proof["decisions_sha256"]
        {
            return Err(error("decision proof binding differs"));
        }
        let feedback: Option<pd_control::maneuver::ManeuverFeedbackV2> =
            serde_json::from_value(info["feedback"].clone()).map_err(error)?;
        let ctx = pd_core::RunContext::from_scenario(&e.scenario).map_err(error)?;
        let reference = feedback.and_then(|f| f.accepted).map(|a| {
            pd_report::flight_annotations::ReferenceOverlayV1 {
                label: "Saved Poly Landing reference".into(),
                points: (0..=a.reference.physics_ticks)
                    .step_by(2)
                    .map(|tick| a.reference.state_at(&ctx, tick).position_m)
                    .collect(),
            }
        });
        let camera = if spec.focus_flat {
            let pd_core::TerrainDefinition::Heightfield { points_m } = &e.scenario.world.terrain;
            if points_m.iter().any(|p| p.y != points_m[0].y) {
                return Err(error("flight camera is scoped to flat terrain"));
            }
            let mut positions: Vec<_> = e
                .execution
                .episode
                .run
                .samples
                .iter()
                .map(|s| s.observation.position_m)
                .collect();
            positions.push(pd_core::Vec2::new(
                ctx.target_pad.center_x_m - ctx.target_pad.half_width_m(),
                ctx.target_pad.surface_y_m,
            ));
            positions.push(pd_core::Vec2::new(
                ctx.target_pad.center_x_m + ctx.target_pad.half_width_m(),
                ctx.target_pad.surface_y_m,
            ));
            if let Some(reference) = &reference {
                positions.extend(reference.points.iter().copied());
            }
            let min_x = positions.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let max_x = positions
                .iter()
                .map(|p| p.x)
                .fold(f64::NEG_INFINITY, f64::max);
            let min_y = positions.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let max_y = positions
                .iter()
                .map(|p| p.y)
                .fold(f64::NEG_INFINITY, f64::max);
            let mx = (0.08 * (max_x - min_x)).max(10.0);
            let my = (0.08 * (max_y - min_y)).max(10.0);
            Some(([min_x - mx, max_x + mx], [min_y - my, max_y + my]))
        } else {
            None
        };
        let raw = pd_report::render_agent_run_report_with_reference(
            &e.scenario,
            &e.execution,
            pd_report::AgentReportEvidenceV1 {
                integrity_passed: e.integrity_passed,
                physical_prefix_verified: e.replay.physical_prefix_verified,
                complete_episode_verified: e.replay.complete_episode_verified,
                verified_target_landing: e.verified_target_landing,
            },
            pd_report::agent_capture_navigation(),
            reference,
        )
        .map_err(error)?;
        let svg = pd_report::build_saved_flight_preview_svg(
            &e.scenario,
            Some(manifest),
            &e.execution.episode.run.samples,
            &[],
        );
        let metrics = detail_view::metric_fallback(&e.execution.episode.run.samples);
        let rich =
            detail_view::with_static_fallback(&raw, &svg, &metrics, &row.label).map_err(error)?;
        let nav = format!(
            "<nav class=\"review-detail-nav\" aria-label=\"Mission navigation\"><a href=\"report.html#mission-{mission_index}\" data-back-review>Back to mission tree</a><a href=\"#\" data-review-previous hidden>Previous matched mission</a><a href=\"#\" data-review-next hidden>Next matched mission</a></nav>"
        );
        let source_href = format!(
            "../{}/evaluation.json",
            root.strip_prefix(
                output
                    .parent()
                    .ok_or_else(|| error("report needs a parent"))?
            )
            .map_err(error)?
            .to_string_lossy()
        );
        let contact = e.execution.episode.incoming_contact.as_ref().map_or_else(||"No native incoming contact recorded.".into(),|c|format!("Incoming native contact at {:.3} s: COM vx {:.3} m/s, vy {:.3} m/s; attitude {:.4} rad, angular rate {:.4} rad/s; classification {:?}.",c.state.sim_time_s,c.state.velocity_mps.x,c.state.velocity_mps.y,c.state.attitude_rad,c.state.angular_rate_radps,c.classification));
        let mut context = format!(
            "<h1>{}</h1><p>{} · native replay, controller and policy reconstruction passed.</p><p>{}</p><p><a href=\"{name}-evidence.json\">Contact and source evidence</a> · <a href=\"{}\">Complete sealed native capture</a> · <a href=\"{name}-proof.json\">Proof receipt</a></p>",
            batch::escape_html(&e.scenario.id),
            batch::escape_html(&row.label),
            batch::escape_html(&contact),
            batch::escape_html(&source_href)
        );
        if let Some(extra) = &row.context_html {
            context.push_str(extra);
        }
        let html = detail_view::with_review_context(
            &rich,
            &nav,
            &context,
            "/coast-export/",
            &format!("mission-{mission_index}"),
            approach,
            e.verified_target_landing,
        )
        .map_err(error)?
        .replace("/coast-export/", "./")
        .replace(
            "new URL(body.dataset.reviewRoot,location.origin)",
            "new URL(body.dataset.reviewRoot,location.href)",
        )
        .replace(
            "href=\"evaluation.json\"",
            &format!("href=\"{name}-evidence.json\""),
        )
        .replace("href=\"seal.json\"", &format!("href=\"{name}-proof.json\""))
        .replace("Landing Quality", "Post-contact summary")
        .replace("Impact attitude", "Stored attitude")
        .replace("Impact speed", "Stored speed")
        .replace("Velocity And Thrust", "Velocity And Engine Setting")
        .replace("name: \"thrust\"", "name: \"engine setting\"")
        .replace("Thrust (0..1)", "Engine setting (0..1)");
        let html = if let Some((x, y)) = camera {
            let old = "xaxis: axisStyle({ title: \"\" }),\n          yaxis: axisStyle({ title: \"\", scaleanchor: \"x\", scaleratio: 1 }),";
            if html.matches(old).count() != 1 {
                return Err(error("maintained initial-camera boundary missing"));
            }
            let replacement = format!(
                "xaxis: axisStyle({{ title: \"\", range: {} }}),\n          yaxis: axisStyle({{ title: \"\", scaleanchor: \"x\", scaleratio: 1, range: {} }}),",
                serde_json::to_string(&x).map_err(error)?,
                serde_json::to_string(&y).map_err(error)?
            );
            html.replacen(old, &replacement, 1)
        } else {
            html
        };
        write_new(&output.join(format!("{name}.html")), html.as_bytes()).map_err(error)?;
        write_new(&output.join(format!("{name}.svg")), svg.as_bytes()).map_err(error)?;
        let evidence = serde_json::to_vec(&serde_json::json!({"physical_sha256":e.physical_sha256,"descriptor":e.execution.descriptor,"incoming_contact":e.execution.episode.incoming_contact,"final_state":e.execution.episode.final_state,"manifest":manifest,"stop":e.execution.episode.stop,"integrity_passed":e.integrity_passed,"replay":e.replay,"source_evaluation_href":source_href,"source_evaluation_sha256":hash,"policy_decisions":info})).map_err(error)?;
        write_new(&output.join(format!("{name}-evidence.json")), &evidence).map_err(error)?;
        write_new(
            &output.join(format!("{name}-proof.json")),
            &serde_json::to_vec(&proof).map_err(error)?,
        )
        .map_err(error)?;
        written_bytes += (html.len()
            + svg.len()
            + evidence.len()
            + serde_json::to_vec(&proof).map_err(error)?.len()) as u64;
        let dx = e.scenario.initial_state.position_m.x;
        let attempt = PairedAttemptV1 {
            approach,
            attempt_index: index as u64,
            outcome: outcome.into(),
            cells: format!(
                "<td>{outcome}</td><td>{:.3} s · {:.1} kg fuel</td><td>Three reconstruction/replay checks passed</td>",
                e.execution.episode.final_state.sim_time_s, e.execution.episode.final_state.fuel_kg
            ),
            href: Some(format!("{name}.html")),
            preview: Some(format!("{name}.svg")),
            caption: row.label.clone(),
        };
        if let Some(mission) = review.missions.get_mut(mission_index) {
            mission.attempts.push(attempt);
        } else {
            review.missions.push(PairedMissionV1 {
                id: format!("mission-{mission_index}"),
                group: group.clone(),
                path: vec![group],
                direction: if dx > 0.0 { -1 } else { 1 },
                direction_unavailable: false,
                label: if spec.grouped {
                    e.scenario.id.clone()
                } else {
                    format!("{} · {}", e.scenario.id, row.label)
                },
                mission_index: mission_index as u64,
                physical_sha256: e.physical_sha256,
                occurrence: 0,
                attempts: vec![attempt],
            });
        }
    }
    let tree = batch::render_review_tree_section(
        "",
        &controls(&review),
        &batch::render_table(
            "scenario-table paired-table",
            "data-tree-table=\"paired-missions\"",
            "<th>Mission / policy</th><th>Outcome</th><th>Endpoint</th><th>Evidence</th>",
            "",
        ),
    );
    let overview = spec.overview_html.unwrap_or_else(|| {
        format!(
            "<section><h2>Measured result</h2><p>{}</p></section>",
            batch::escape_html(&spec.summary)
        )
    });
    let html = batch::render_review_first_batch_page(batch::BatchPage {
        title: &spec.title,
        subtitle: "Supplied ballistic arcs · controller-owned coast · fixed Poly Landing burn",
        chips_html: &format!(
            "<span class=\"chip\">{} complete rich flight views</span>",
            review.missions.len()
        ),
        actions_html: "<a href=\"#review-tree-root\">Browse missions</a><a href=\"report-input.json\">Projection data</a>",
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
            data_script(&review).map_err(error)?
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
    write_new(&output.join("report.html"), html.as_bytes()).map_err(error)?;
    write_new(
        &output.join("report-input.json"),
        &serde_json::to_vec(&review).map_err(error)?,
    )
    .map_err(error)?;
    write_new(
        &output.join("navigation.json"),
        &serde_json::to_vec(&review).map_err(error)?,
    )
    .map_err(error)?;
    Ok(())
}
