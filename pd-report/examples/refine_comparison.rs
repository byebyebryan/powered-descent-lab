//! Create-only presentation of sealed, already replayed campaigns. No execution.
#[path = "support/saved_campaign.rs"]
pub mod saved_campaign;

use anyhow::{Result, ensure};
use pd_report::{
    campaign::CampaignPageV1,
    campaign_view::{self, CampaignPresentationV1},
    comparison::ComparisonPageV1,
    paired_review,
};
use saved_campaign::*;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

fn source_directory(root: &Path, href: &str) -> Result<PathBuf> {
    let path = Path::new(href);
    ensure!(
        path.components()
            .all(|p| matches!(p, std::path::Component::Normal(_)))
            && path.file_name().is_some_and(|p| p == "report.html"),
        "unsafe campaign source path"
    );
    let directory = root.join(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("missing campaign directory"))?,
    );
    ensure!(
        fs::symlink_metadata(&directory)?.is_dir(),
        "campaign source is not a regular directory"
    );
    Ok(directory)
}

fn verify_summary(page: &CampaignPageV1, bytes: &[u8]) -> Result<()> {
    let summary: serde_json::Value = serde_json::from_slice(bytes)?;
    ensure!(
        summary["schema_version"] == 1
            && summary["request"]["agent"] == serde_json::to_value(&page.agent)?
            && summary["request"]["manifest_sha256"] == page.manifest_sha256
            && summary["integrity_passed"] == page.integrity_passed
            && summary["unique_scheduled_missions"] == page.unique_scheduled_missions,
        "campaign summary identity differs"
    );
    let attempts = summary["attempts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("missing summary attempts"))?;
    ensure!(
        attempts.len() == page.attempts.len(),
        "summary schedule differs"
    );
    for (actual, row) in attempts.iter().zip(&page.attempts) {
        let d = &actual["disposition"];
        ensure!(
            actual["attempt_index"] == row.attempt_index
                && actual["mission_index"] == row.mission_index
                && d["kind"] == row.disposition,
            "summary attempt identity differs"
        );
        if row.disposition == "executed" {
            for (key, expected) in [
                ("stop", serde_json::to_value(row.stop)?),
                (
                    "physical_outcome",
                    serde_json::to_value(&row.physical_outcome)?,
                ),
                (
                    "mission_outcome",
                    serde_json::to_value(&row.mission_outcome)?,
                ),
                ("integrity_passed", row.integrity_passed.into()),
                (
                    "physical_replay_verified",
                    row.physical_replay_verified.into(),
                ),
                (
                    "complete_replay_verified",
                    row.complete_replay_verified.into(),
                ),
                (
                    "verified_target_landing",
                    row.verified_target_landing.into(),
                ),
            ] {
                ensure!(d[key] == expected, "summary outcome or proof differs");
            }
            ensure!(
                row.report_href.as_deref()
                    == d["capture"]
                        .as_str()
                        .map(|capture| format!("{capture}/report.html"))
                        .as_deref(),
                "summary capture link differs"
            );
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 4 || args.len() == 6,
        "usage: refine_comparison SAVED_COMPARISON_DIR NEW_EDITION_DIR SOURCE_HREF_BASE EDITION_HREF_BASE [--mission-limit N]"
    );
    let source = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    let base = &args[2];
    let edition = &args[3];
    for href in [base, edition] {
        ensure!(
            href.starts_with('/')
                && !href.starts_with("//")
                && !href.contains(['\\', '?', '#', ':'])
                && href.ends_with('/'),
            "href bases must be local absolute directory URLs"
        );
    }
    let limit = if args.len() == 6 {
        ensure!(args[4] == "--mission-limit", "unknown inspection option");
        let n = args[5].parse::<u64>()?;
        ensure!(n > 0, "inspection limit must be positive");
        Some(n)
    } else {
        None
    };
    let input = read_regular(&source.join("comparison-input.json"), 32 * 1024 * 1024)?;
    let mut page: ComparisonPageV1 = serde_json::from_slice(&input)?;
    pd_report::comparison::render_comparison_page(&page).map_err(anyhow::Error::msg)?;
    let verification_bytes = read_regular(
        &source.join("independent-verification.json"),
        4 * 1024 * 1024,
    )?;
    let verification: serde_json::Value = serde_json::from_slice(&verification_bytes)?;
    ensure!(
        verification["schema_version"] == 1 && verification["all_passed"] == true,
        "retained independent verification did not pass"
    );
    let proofs = verification["campaigns"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("missing retained campaign verification"))?;
    let mut sources = Vec::new();
    let mut campaign_bindings = Vec::new();
    for approach in &mut page.approaches {
        let mut directories = Vec::new();
        for campaign in &mut approach.campaigns {
            let directory = source_directory(&source, &campaign.report_href)?;
            let bytes = read_regular(&directory.join("summary.json"), 16 * 1024 * 1024)?;
            let identity = digest(&bytes);
            verify_summary(&campaign.page, &bytes)?;
            let proof = proofs
                .iter()
                .find(|v| {
                    v["directory"]
                        .as_str()
                        .is_some_and(|p| Path::new(p) == directory)
                })
                .ok_or_else(|| anyhow::anyhow!("retained verification does not bind campaign"))?;
            ensure!(
                proof["summary_sha256"] == identity
                    && proof["scheduled"] == campaign.page.attempts.len(),
                "retained proof summary differs"
            );
            campaign_bindings
                .push(serde_json::json!({"directory":directory,"summary_sha256":identity}));
            if let Some(n) = limit {
                campaign.page.attempts.retain(|r| r.mission_index < n);
                campaign.page.unique_scheduled_missions = campaign
                    .page
                    .attempts
                    .iter()
                    .map(|r| &r.physical_sha256)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len();
                campaign.page.collection_role = "Inspection slice of retained campaign".into();
            }
            directories.push(directory);
        }
        sources.push(directories);
    }
    let original = page.clone();
    let review = paired_review::build(&page).map_err(anyhow::Error::msg)?;
    let maps: Vec<BTreeMap<String, Vec<(usize, usize)>>> = original
        .approaches
        .iter()
        .map(|a| {
            let mut map: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
            for (c, campaign) in a.campaigns.iter().enumerate() {
                for (r, row) in campaign.page.attempts.iter().enumerate() {
                    map.entry(row.physical_sha256.clone())
                        .or_default()
                        .push((c, r));
                }
            }
            map
        })
        .collect();
    fs::create_dir_all(
        output
            .parent()
            .ok_or_else(|| anyhow::anyhow!("missing output parent"))?,
    )?;
    ensure!(
        !output
            .parent()
            .expect("checked parent")
            .canonicalize()?
            .starts_with(source.canonicalize()?),
        "edition must be outside original capture"
    );
    fs::create_dir(&output)?;
    let started = Instant::now();
    let mut capture_bindings = Vec::new();
    let mut status_count = 0;
    for (ordinal, mission) in review.missions.iter().enumerate() {
        let mut captures = Vec::new();
        let mut lower = pd_core::Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut upper = pd_core::Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        let coordinates: Vec<_> = maps
            .iter()
            .map(|m| m[&mission.physical_sha256][mission.occurrence])
            .collect();
        for (a, (c, r)) in coordinates.iter().copied().enumerate() {
            let campaign = &original.approaches[a].campaigns[c].page;
            let row = &campaign.attempts[r];
            if row.disposition == "executed" {
                let directory = capture_directory(&sources[a][c], row)?;
                let (value, identity) = read_capture(&directory, campaign, row)?;
                for point in value
                    .execution
                    .episode
                    .run
                    .samples
                    .iter()
                    .map(|s| s.observation.position_m)
                    .chain(std::iter::once(
                        value.execution.episode.final_state.position_m,
                    ))
                {
                    lower.x = lower.x.min(point.x);
                    lower.y = lower.y.min(point.y);
                    upper.x = upper.x.max(point.x);
                    upper.y = upper.y.max(point.y);
                }
                captures.push(Some((directory, value, identity)));
            } else {
                captures.push(None);
            }
        }
        let bounds = if lower.x.is_finite() {
            vec![lower, upper]
        } else {
            Vec::new()
        };
        for (a, (c, r)) in coordinates.iter().copied().enumerate() {
            let row = &original.approaches[a].campaigns[c].page.attempts[r];
            let preview = format!("previews/{}-a{a}.svg", mission.id);
            let detail = format!("details/{}-a{a}.html", mission.id);
            let source_campaign_href =
                format!("{base}{}", original.approaches[a].campaigns[c].report_href);
            let nav = format!(
                "<nav class=\"review-detail-nav\" aria-label=\"Paired mission navigation\"><a href=\"{edition}report.html#{}\" data-back-review>Back to paired review</a><a href=\"#\" data-review-previous hidden>Previous matched mission</a><a href=\"#\" data-review-next hidden>Next matched mission</a><a href=\"{}\">Original campaign</a></nav>",
                mission.id,
                pd_report::escape_html(&source_campaign_href)
            );
            let integrity = original.approaches[a]
                .campaigns
                .iter()
                .all(|c| c.page.integrity_passed);
            let mut view = None;
            let html = if let Some((directory, value, identity)) = &captures[a] {
                let (svg, presentation) =
                    preview_projection_with_bounds(value, preview.clone(), &bounds)?;
                write_new(&output.join(&preview), svg.as_bytes())?;
                let original_bytes = read_regular(&directory.join("report.html"), 8 * 1024 * 1024)?;
                let original_html = std::str::from_utf8(&original_bytes)?;
                let payload = saved_report_payload(original_html, value)?;
                let original_data: serde_json::Value = serde_json::from_str(&payload)?;
                let handoff_count = original_data["flightAnnotations"]["corrections"]
                    .as_array()
                    .map_or(0, Vec::len);
                let refined = pd_report::detail_view::with_static_fallback(
                    original_html,
                    &svg,
                    &pd_report::detail_view::metric_fallback(&value.execution.episode.run.samples),
                    &presentation.caption,
                )?;
                let endpoint = format!(
                    "<h1>Mission {} · {}</h1><p>{}</p><p>Endpoint: target horizontal offset {} m · height above upright touchdown {} m · speed {} m/s.</p><p>{}</p>",
                    row.mission_index,
                    pd_report::escape_html(&original.approaches[a].label),
                    campaign_view::render_outcome_cell(row, integrity),
                    presentation
                        .target_horizontal_offset_m
                        .map_or_else(|| "Unavailable".into(), |n| format!("{n:.2}")),
                    presentation
                        .height_above_upright_touchdown_m
                        .map_or_else(|| "Unavailable".into(), |n| format!("{n:.2}")),
                    presentation
                        .endpoint_speed_mps
                        .map_or_else(|| "Unavailable".into(), |n| format!("{n:.2}")),
                    campaign_view::proof_label(row, integrity)
                );
                let html = pd_report::detail_view::with_review_context(
                    &refined,
                    &nav,
                    &endpoint,
                    edition,
                    &mission.id,
                    a,
                    campaign_view::outcome(row, integrity) == campaign_view::DisplayOutcome::Landed,
                )?;
                ensure!(
                    saved_report_payload(&html, value)? == payload,
                    "refinement changed the saved rich payload"
                );
                capture_bindings.push(serde_json::json!({"mission":mission.id,"approach":a,"physical_sha256":mission.physical_sha256,"source":directory,"evaluation_sha256":identity,"source_html_sha256":digest(&original_bytes),"saved_payload_sha256":digest(payload.as_bytes()),"executed_handoffs":handoff_count,"preview_sha256":digest(svg.as_bytes()),"detail_sha256":digest(html.as_bytes()),"shared_world_bounds":bounds}));
                view = Some(presentation);
                html
            } else {
                status_count += 1;
                let terrain = captures.iter().flatten().next().map(|(_, v, _)| {
                    let svg = pd_report::build_saved_flight_preview_svg_with_context(
                        &v.scenario,
                        None,
                        &[],
                        &[],
                        &bounds,
                    );
                    (
                        svg,
                        CampaignPresentationV1 {
                            schema_version: 1,
                            preview_href: preview.clone(),
                            caption: "Terrain and target only · no recorded flight".into(),
                            target_horizontal_offset_m: None,
                            height_above_upright_touchdown_m: None,
                            endpoint_speed_mps: None,
                        },
                    )
                });
                let terrain_html = if let Some((svg, presentation)) = terrain {
                    write_new(&output.join(&preview), svg.as_bytes())?;
                    view = Some(presentation);
                    svg
                } else {
                    String::new()
                };
                let content = format!(
                    "<p>{}</p><p>{}</p><p>No simulator trajectory or flight proof exists for this scheduled attempt.</p>{terrain_html}<p>Physical input <code>{}</code></p>",
                    campaign_view::render_outcome_cell(row, integrity),
                    campaign_view::proof_label(row, integrity),
                    pd_report::escape_html(&row.physical_sha256)
                );
                let html = pd_report::batch::render_status_page(
                    &format!(
                        "Mission {} · {}",
                        row.mission_index, original.approaches[a].label
                    ),
                    &nav,
                    &content,
                );
                pd_report::detail_view::with_review_context(
                    &html,
                    "",
                    "",
                    edition,
                    &mission.id,
                    a,
                    false,
                )?
            };
            write_new(&output.join(&detail), html.as_bytes())?;
            page.approaches[a].campaigns[c].page.attempts[r].report_href = Some(detail);
            page.approaches[a].campaigns[c].page.attempts[r].presentation = view;
        }
        if ordinal % 128 == 0 {
            println!(
                "Rendered {}/{} paired missions ({:.1}s)",
                ordinal + 1,
                review.missions.len(),
                started.elapsed().as_secs_f64()
            );
        }
    }
    for (a, approach) in page.approaches.iter_mut().enumerate() {
        for (c, campaign) in approach.campaigns.iter_mut().enumerate() {
            let href = &original.approaches[a].campaigns[c].report_href;
            campaign.page.summary_href = format!(
                "{base}{}summary.json",
                href.strip_suffix("report.html").expect("validated suffix")
            );
            let projection_href = format!("campaign-a{a}-c{c}-input.json");
            campaign.report_href = format!("campaign-a{a}-c{c}.html");
            let html = pd_report::campaign::render_campaign_edition_page(
                &campaign.page,
                &projection_href,
                "report.html",
            )
            .map_err(anyhow::Error::msg)?;
            write_new(&output.join(&campaign.report_href), html.as_bytes())?;
            write_new(
                &output.join(&projection_href),
                &serde_json::to_vec(&campaign.page)?,
            )?;
        }
    }
    page.provenance_href = format!("{base}{}", original.provenance_href);
    page.title = if limit.is_some() {
        format!(
            "Procedural comparison · inspection slice ({})",
            review.missions.len()
        )
    } else {
        format!(
            "Procedural mission comparison · {} missions / {} approaches",
            review.missions.len(),
            review.labels.len()
        )
    };
    let refined = paired_review::build(&page).map_err(anyhow::Error::msg)?;
    let mut navigation = serde_json::to_value(&refined)?;
    for mission in navigation["missions"]
        .as_array_mut()
        .expect("serialized missions")
    {
        for attempt in mission["attempts"]
            .as_array_mut()
            .expect("serialized attempts")
        {
            attempt
                .as_object_mut()
                .expect("serialized attempt")
                .remove("cells");
        }
    }
    write_new(
        &output.join("navigation.json"),
        &serde_json::to_vec(&navigation)?,
    )?;
    write_new(
        &output.join("comparison-input.json"),
        &serde_json::to_vec(&page)?,
    )?;
    let html = pd_report::comparison::render_comparison_page(&page).map_err(anyhow::Error::msg)?;
    write_new(&output.join("report.html"), html.as_bytes())?;
    let receipt = serde_json::json!({"schema_version":1,"kind":"saved_procedural_presentation_refinement","source":source,"source_projection_sha256":digest(&input),"independent_verification_sha256":digest(&verification_bytes),"campaigns":campaign_bindings,"source_base_href":base,"edition_base_href":edition,"inspection_limit":limit,"scheduled_missions":refined.missions.len(),"scheduled_attempts":refined.missions.len()*refined.labels.len(),"native_details":capture_bindings.len(),"campaign_pages":campaign_bindings.len(),"status_details":status_count,"captures":capture_bindings,"report_sha256":digest(html.as_bytes()),"elapsed_s":started.elapsed().as_secs_f64(),"new_flights":0,"new_replays":0,"saved_rich_payloads_preserved":true});
    write_new(
        &output.join("render.json"),
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    println!(
        "Completed {} paired missions in {:.1}s: {}",
        refined.missions.len(),
        started.elapsed().as_secs_f64(),
        output.display()
    );
    Ok(())
}
