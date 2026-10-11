//! Paired saved-campaign comparison without importing an evaluator or strategy.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::batch;
use crate::campaign::{CampaignPageV1, escape, family_label, validate_campaign_page, verified};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonCampaignV1 {
    pub page: CampaignPageV1,
    pub report_href: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonApproachV1 {
    pub label: String,
    pub campaigns: Vec<ComparisonCampaignV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonPageV1 {
    pub schema_version: u32,
    pub title: String,
    pub provenance_href: String,
    pub approaches: Vec<ComparisonApproachV1>,
}

type InputInventory = BTreeMap<String, (String, i32, u32, usize)>;

fn inventory(approach: &ComparisonApproachV1) -> Result<InputInventory, String> {
    if approach.campaigns.is_empty() {
        return Err("comparison approach has no campaigns".into());
    }
    let mut result = BTreeMap::new();
    let descriptor = &approach.campaigns[0].page.agent;
    for campaign in &approach.campaigns {
        let page = &campaign.page;
        validate_campaign_page(page)?;
        if &page.agent != descriptor {
            return Err("comparison approach changes descriptor across campaigns".into());
        }
        for row in &page.attempts {
            let entry = result.entry(row.physical_sha256.clone()).or_insert((
                row.terrain_family.clone(),
                row.direction,
                row.terrain_stratum,
                0,
            ));
            if entry.0 != row.terrain_family
                || entry.1 != row.direction
                || entry.2 != row.terrain_stratum
            {
                return Err("physical input has inconsistent coverage metadata".into());
            }
            entry.3 += 1;
        }
    }
    Ok(result)
}

pub fn render_comparison_page(page: &ComparisonPageV1) -> Result<String, String> {
    if page.schema_version != 1 || page.approaches.len() < 2 {
        return Err("comparison requires at least two declared approaches".into());
    }
    let reference = inventory(&page.approaches[0])?;
    for approach in &page.approaches[1..] {
        if inventory(approach)? != reference {
            return Err(
                "comparison approaches do not share the same physical inputs and repeat schedule"
                    .into(),
            );
        }
    }
    let mut overview_rows = String::new();
    let mut context = String::new();
    let mut groups: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    let mut tree_rows = String::new();
    for (ordinal, approach) in page.approaches.iter().enumerate() {
        let scheduled: usize = approach
            .campaigns
            .iter()
            .map(|c| c.page.attempts.len())
            .sum();
        let integrity = approach.campaigns.iter().all(|c| c.page.integrity_passed);
        let rows: Vec<_> = approach
            .campaigns
            .iter()
            .flat_map(|c| &c.page.attempts)
            .collect();
        let landings = if integrity {
            rows.iter().filter(|r| verified(r)).count()
        } else {
            0
        };
        let unsupported = rows
            .iter()
            .filter(|r| r.disposition == "unsupported")
            .count();
        let replayed = rows.iter().filter(|r| r.complete_replay_verified).count();
        let executed = rows.iter().filter(|r| r.disposition == "executed").count();
        let failures = scheduled - executed - unsupported;
        let links = approach
            .campaigns
            .iter()
            .map(|c| {
                format!(
                    "<a href=\"{}\">{}</a>",
                    escape(&c.report_href),
                    escape(family_label(&c.page.attempts[0].terrain_family))
                )
            })
            .collect::<Vec<_>>()
            .join(" · ");
        overview_rows.push_str(&format!("<tr><td>{}</td><td>{scheduled}</td><td>{}</td><td>{landings}/{scheduled}</td><td>{unsupported}</td><td>{failures}</td><td>{replayed}/{executed}</td><td>{integrity}</td><td>{links}</td></tr>",escape(&approach.label),reference.len()));
        let descriptor = serde_json::to_string_pretty(&approach.campaigns[0].page.agent)
            .map_err(|e| e.to_string())?;
        context.push_str(&format!(
            "<details><summary>{}</summary><pre>{}</pre></details>",
            escape(&approach.label),
            escape(&descriptor)
        ));
        let group_id = format!("comparison-{ordinal}");
        tree_rows.push_str(&batch::render_row(Some("summary-row"), &format!("data-group=\"{group_id}\" data-kind=\"group\" data-depth=\"0\" aria-expanded=\"false\" tabindex=\"0\""),
            &format!("<td><span class=\"expander\">+</span> {}</td><td>{landings}/{scheduled}</td><td>{links}</td>",escape(&approach.label))));
        for campaign in &approach.campaigns {
            let count = campaign.page.attempts.len();
            let landed = if integrity {
                campaign
                    .page
                    .attempts
                    .iter()
                    .filter(|r| verified(r))
                    .count()
            } else {
                0
            };
            tree_rows.push_str(&batch::render_row(Some("comparison-row"),&format!("data-parent=\"{group_id}\" data-depth=\"1\""),
                &format!("<td>{}</td><td>{landed}/{count}</td><td><a href=\"{}\">Campaign and native flight evidence</a></td>",escape(family_label(&campaign.page.attempts[0].terrain_family)),escape(&campaign.report_href))));
            for row in &campaign.page.attempts {
                let key = format!(
                    "{} · {} · stratum {}",
                    family_label(&row.terrain_family),
                    if row.direction < 0 {
                        "left"
                    } else if row.direction > 0 {
                        "right"
                    } else {
                        "vertical"
                    },
                    row.terrain_stratum
                );
                let counters = groups
                    .entry(key)
                    .or_insert_with(|| vec![(0, 0); page.approaches.len()]);
                counters[ordinal].0 += usize::from(integrity && verified(row));
                counters[ordinal].1 += 1;
            }
        }
    }
    let overview = batch::render_overview_section(
        "",
        &batch::render_overview_table(
            "<th>Approach</th><th>Scheduled</th><th>Unique inputs</th><th>Verified landing</th><th>Unsupported</th><th>Failed preparation/capture</th><th>Complete replay</th><th>Integrity</th><th>Campaigns</th>",
            &overview_rows,
        ),
    );
    let headers = page
        .approaches
        .iter()
        .map(|a| format!("<th>{}</th>", escape(&a.label)))
        .collect::<String>();
    let coverage_rows = groups
        .iter()
        .map(|(label, counters)| {
            batch::render_coverage_row(
                label,
                &counters
                    .iter()
                    .enumerate()
                    .map(|(ordinal, (landed, scheduled))| {
                        batch::render_coverage_group_cell(
                            "coverage-cell",
                            &format!("comparison-{ordinal}"),
                            &format!("{landed}/{scheduled}"),
                        )
                    })
                    .collect::<String>(),
            )
        })
        .collect::<String>();
    let coverage = batch::render_coverage_section(
        "Same physical inputs and repeat schedule for every approach. Unsupported inputs remain in the full denominator.",
        "",
        &batch::render_coverage_table("Family / direction / stratum", &headers, &coverage_rows),
    );
    let tree = batch::render_review_tree_section(
        "",
        "Expand an approach to open its common campaigns and rich individual native captures.",
        &batch::render_review_tree_table(
            "procedural-comparison",
            "<th>Approach / family</th><th>Verified landing / scheduled</th><th>Saved evidence</th>",
            &tree_rows,
        ),
    );
    let context = format!(
        "<section class=\"context-section\"><h2>Configuration and provenance</h2><p>Each approach covers {} unique physical inputs. Counts require campaign integrity and the complete physical, mission, execution-validity and native replay tuple. A failed constituent campaign gates that approach's combined landing count to zero.</p>{context}</section>",
        reference.len()
    );
    Ok(batch::render_batch_page(batch::BatchPage {
        title: &page.title,
        subtitle: "Paired complete procedural missions under identical physical and resource budgets",
        chips_html: &format!(
            "<span class=\"chip\">{} unique physical inputs per approach</span>",
            reference.len()
        ),
        actions_html: &format!(
            "<a href=\"{}\">Experiment provenance</a>",
            escape(&page.provenance_href)
        ),
        before_hero_html: "<style>body.procedural-comparison .coverage-table th:first-child { width: 320px; text-transform: none; }</style>",
        after_hero_html: "",
        overview_html: &overview,
        planner_html: "",
        coverage_html: &coverage,
        context_html: &context,
        diagnostics_html: "",
        review_tree_html: &tree,
        comparison_html: "",
        appendix_html: "",
        body_class: "procedural-comparison",
        tree: batch::BatchTreeOptions {
            max_depth: 1,
            depth_by_kind: &[("group", 0)],
            leaf_selector: "tr.comparison-row",
            default_expansion: 0,
        },
        coverage_tree_jump: true,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::CampaignRowV1;
    use pd_core::{BoundedRunStopCauseV1, MissionOutcome, PhysicalOutcome};
    use pd_run::AgentDescriptorV1;

    fn approach(label: &str, integrity: bool) -> ComparisonApproachV1 {
        ComparisonApproachV1 {
            label: label.into(),
            campaigns: vec![ComparisonCampaignV1 {
                report_href: format!("{label}/report.html"),
                page: CampaignPageV1 {
                    schema_version: 1,
                    title: label.into(),
                    agent: AgentDescriptorV1 {
                        schema_version: 1,
                        implementation: label.into(),
                        configuration: serde_json::json!({}),
                        checkpoint_sha256: None,
                    },
                    integrity_passed: integrity,
                    unique_scheduled_missions: 1,
                    manifest_sha256: "a".repeat(64),
                    collection_role: "HeldOut".into(),
                    prior_training_or_tuning: Some(false),
                    recorded_input_inspection: Some(false),
                    summary_href: "summary.json".into(),
                    attempts: vec![CampaignRowV1 {
                        attempt_index: 0,
                        mission_index: 0,
                        physical_sha256: "b".repeat(64),
                        terrain_family: "pd-missions/value-noise-rest-v1".into(),
                        direction: 1,
                        terrain_stratum: 0,
                        disposition: "executed".into(),
                        detail: None,
                        stop: Some(BoundedRunStopCauseV1::MissionTerminal),
                        physical_outcome: Some(PhysicalOutcome::LandedOnTarget),
                        mission_outcome: Some(MissionOutcome::Success),
                        execution_valid: true,
                        integrity_passed: true,
                        physical_replay_verified: true,
                        complete_replay_verified: true,
                        verified_target_landing: true,
                        sim_time_s: Some(10.0),
                        fuel_kg: Some(3000.0),
                        wall_time_us: Some(100),
                        report_href: Some("attempt/report.html".into()),
                        presentation: None,
                    }],
                },
            }],
        }
    }

    fn comparison() -> ComparisonPageV1 {
        ComparisonPageV1 {
            schema_version: 1,
            title: "Paired <review>".into(),
            provenance_href: "freeze.json".into(),
            approaches: vec![approach("first", true), approach("second", false)],
        }
    }

    #[test]
    fn common_shell_gates_invalid_campaign_and_preserves_evidence_links() {
        let html = render_comparison_page(&comparison()).unwrap();
        assert!(html.contains("data-batch-template=\"common-v1\""));
        assert!(html.contains("Paired &lt;review&gt;"));
        assert!(html.contains("<td>first</td><td>1</td><td>1</td><td>1/1</td>"));
        assert!(html.contains("<td>second</td><td>1</td><td>1</td><td>0/1</td>"));
        assert!(html.contains("href=\"second/report.html\""));
    }

    #[test]
    fn different_inputs_or_repeat_denominators_cannot_be_paired() {
        let mut page = comparison();
        page.approaches[1].campaigns[0].page.attempts[0].physical_sha256 = "c".repeat(64);
        assert!(
            render_comparison_page(&page)
                .unwrap_err()
                .contains("same physical inputs")
        );
        let mut page = comparison();
        let repeated = page.approaches[1].campaigns[0].page.attempts[0].clone();
        page.approaches[1].campaigns[0].page.attempts.push(repeated);
        assert!(
            render_comparison_page(&page)
                .unwrap_err()
                .contains("repeat schedule")
        );
    }
}
