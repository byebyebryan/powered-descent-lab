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
    let review = crate::paired_review::build(page)?;
    let mut cards = String::new();
    let mut accounting = String::new();
    let mut context = String::new();
    for (ordinal, approach) in page.approaches.iter().enumerate() {
        let integrity = approach.campaigns.iter().all(|c| c.page.integrity_passed);
        let rows: Vec<_> = approach
            .campaigns
            .iter()
            .flat_map(|c| c.page.attempts.iter().map(|r| (r, integrity)))
            .collect();
        let scheduled = rows.len();
        let landed = rows.iter().filter(|(r, ok)| *ok && verified(r)).count();
        cards.push_str(&crate::campaign_view::render_outcome_bar(
            &approach.label,
            &rows,
            ordinal,
        ));
        accounting.push_str(&format!(
            "<tr><td>{}</td><td>{scheduled}</td><td>{}</td><td>{landed}/{scheduled}</td></tr>",
            escape(&approach.label),
            reference.len()
        ));
        let descriptor = serde_json::to_string_pretty(&approach.campaigns[0].page.agent)
            .map_err(|e| e.to_string())?;
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
        context.push_str(&format!(
            "<details><summary>{}</summary><p>{links}</p><pre>{}</pre></details>",
            escape(&approach.label),
            escape(&descriptor)
        ));
    }
    let incumbent = page
        .approaches
        .iter()
        .position(|a| a.label.to_lowercase().contains("reflection"))
        .unwrap_or(page.approaches.len() - 1);
    let shortcuts = format!(
        "<div class=\"review-shortcuts\"><button type=\"button\" data-review-outcome=\"misses\" data-approach=\"{incumbent}\">Review incumbent misses</button><button type=\"button\" data-review-outcome=\"unsupported\" data-approach=\"all\">Review unsupported inputs</button><button type=\"button\" data-review-outcome=\"not-launched\" data-approach=\"all\">Review stopped before departure</button></div>"
    );
    let overview = format!(
        "<section class=\"header-overview\"><h2>Complete procedural missions</h2><p class=\"section-note\">Source-pad rest → target-pad landing, with procedural terrain along the complete transfer. Browse the mission structure below, then inspect saved controller results for each input.</p><details class=\"controller-overview\"><summary>Controller outcomes across this mission set</summary>{shortcuts}<p class=\"section-note\">Each bar uses the full scheduled denominator. Click an outcome to find those missions. Replay proof is separate from the physical outcome.</p><div class=\"outcome-cards\">{cards}</div><details><summary>Schedule accounting</summary><div class=\"table-wrap\">{}</div></details></details></section>",
        batch::render_overview_table(
            "<th>Approach</th><th>Scheduled attempts</th><th>Unique physical inputs</th><th>Verified landings</th>",
            &accounting
        )
    );
    let coverage_headers = page
        .approaches
        .iter()
        .map(|a| format!("<th>{}</th>", escape(&a.label)))
        .collect::<String>();
    let mut coverage_rows = String::new();
    for group in &review.groups {
        let missions: Vec<_> = review
            .missions
            .iter()
            .filter(|m| m.group == group.id)
            .collect();
        let cells=page.approaches.iter().enumerate().map(|(approach,_)|{
            let total=missions.len();let landed=missions.iter().filter(|m|m.attempts[approach].outcome=="landed").count();let pct=100.0*landed as f64/total.max(1) as f64;
            format!("<td><button type=\"button\" class=\"coverage-inspect {}\" data-paired-focus=\"{}\" data-approach=\"{approach}\" aria-label=\"Inspect {} for {}\">{landed}/{total}<span class=\"row-note\">{pct:.1}%</span></button></td>",if landed==0 {"zero"}else if pct>=90.0 {"high"}else{"mixed"},group.id,escape(&group.label),escape(&page.approaches[approach].label))
        }).collect::<String>();
        coverage_rows.push_str(&batch::render_coverage_row(&group.label, &cells));
    }
    let coverage = format!(
        "<details class=\"paired-coverage\"><summary>Coverage · {} family / direction / stratum groups</summary>{}</details>",
        review.groups.len(),
        batch::render_coverage_section(
            "Click a cell to focus that exact coverage group and approach. Unsupported inputs remain scheduled.",
            "",
            &format!(
                "<div class=\"table-wrap\">{}</div>",
                batch::render_coverage_table(
                    "Physical mission coverage",
                    &coverage_headers,
                    &coverage_rows
                )
            )
        )
    );
    let tree = batch::render_review_tree_section(
        "",
        &crate::paired_review::controls(&review),
        &batch::render_table(
            "scenario-table paired-table",
            "data-tree-table=\"paired-missions\"",
            "<th>Mission / saved flight</th><th>Outcome / stop</th><th>Flight / endpoint</th><th>Evidence</th>",
            "",
        ),
    );
    let context = format!(
        "<details class=\"provenance-disclosure context-section\"><summary>Configuration, campaign evidence and provenance</summary><p>Each approach covers {} unique physical inputs. Repeated physical inputs remain separate scheduled occurrences. Landing counts require campaign integrity and the complete physical, mission, execution-validity and replay tuple. A failed constituent campaign gates its approach's combined landing count.</p>{context}</details>",
        reference.len()
    );
    let scripts = format!(
        "<script>{}</script>{}",
        crate::campaign_view::SCRIPT,
        crate::paired_review::data_script(&review)?
    );
    Ok(batch::render_review_first_batch_page(batch::BatchPage {
        title: &page.title,
        subtitle: "Paired complete procedural missions under identical physical and resource budgets",
        chips_html: &format!(
            "<span class=\"chip\">{} unique physical inputs · {} scheduled occurrences · {} approaches</span>",
            reference.len(),
            review.missions.len(),
            page.approaches.len()
        ),
        actions_html: &format!(
            "<a href=\"#review-tree-root\">Inspect missions</a><a href=\"/\">Report library</a><a href=\"{}\">Experiment provenance</a><a href=\"comparison-input.json\">Report projection</a>",
            escape(&page.provenance_href)
        ),
        before_hero_html: &format!("<style>{}</style>", crate::campaign_view::STYLE),
        after_hero_html: "",
        overview_html: &overview,
        planner_html: "",
        coverage_html: &coverage,
        context_html: &context,
        diagnostics_html: "",
        review_tree_html: &tree,
        comparison_html: "",
        appendix_html: &scripts,
        body_class: "procedural-comparison campaign-view",
        tree: batch::BatchTreeOptions {
            max_depth: 0,
            depth_by_kind: &[],
            leaf_selector: "tr.paired-attempt-row",
            default_expansion: 0,
        },
        coverage_tree_jump: false,
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

    #[test]
    fn pairing_uses_identity_when_approach_order_differs_and_keeps_repeats() {
        let mut page = comparison();
        for approach in &mut page.approaches {
            let campaign = &mut approach.campaigns[0].page;
            let mut second = campaign.attempts[0].clone();
            second.physical_sha256 = "c".repeat(64);
            second.mission_index = 19;
            second.attempt_index = 19;
            campaign.attempts.push(second);
            campaign.unique_scheduled_missions = 2;
        }
        page.approaches[1].campaigns[0].page.attempts.reverse();
        render_comparison_page(&page).unwrap();
        let review = crate::paired_review::build(&page).unwrap();
        assert_eq!(review.missions.len(), 2);
        for mission in &review.missions {
            assert_eq!(
                mission.attempts[0].attempt_index,
                mission.attempts[1].attempt_index
            );
        }
        for approach in &mut page.approaches {
            let mut repeat = approach.campaigns[0]
                .page
                .attempts
                .iter()
                .find(|r| r.mission_index == 19)
                .unwrap()
                .clone();
            repeat.attempt_index = 20;
            approach.campaigns[0].page.attempts.push(repeat);
        }
        render_comparison_page(&page).unwrap();
        let review = crate::paired_review::build(&page).unwrap();
        assert_eq!(review.missions.len(), 3);
        assert_eq!(
            review
                .missions
                .iter()
                .filter(|m| m.mission_index == 19)
                .count(),
            2
        );
        assert_eq!(
            review
                .missions
                .iter()
                .find(|m| m.occurrence == 1)
                .unwrap()
                .attempts[1]
                .attempt_index,
            20
        );
    }

    #[test]
    fn mission_hierarchy_merges_direction_and_keeps_attempts_and_coverage_identity() {
        let mut page = comparison();
        for approach in &mut page.approaches {
            let campaign = &mut approach.campaigns[0].page;
            for (index, family, direction, stratum) in [
                (1, "pd-missions/value-noise-rest-v1", -1, 2),
                (2, "pd-missions/layered-perlin-rest-v1", 1, 0),
                (3, "pd-missions/value-noise-rest-v1", -1, 0),
            ] {
                let mut row = campaign.attempts[0].clone();
                row.physical_sha256 = format!("{index:064x}");
                row.mission_index = index;
                row.terrain_family = family.into();
                row.direction = direction;
                row.terrain_stratum = stratum;
                campaign.attempts.push(row);
            }
            campaign.unique_scheduled_missions = 4;
        }
        let review = crate::paired_review::build(&page).unwrap();
        assert_eq!(
            review
                .branches
                .iter()
                .filter(|b| b.parent.is_none())
                .count(),
            2
        );
        for mission in &review.missions {
            let path: Vec<_> = mission
                .path
                .iter()
                .map(|id| review.branches.iter().find(|b| &b.id == id).unwrap())
                .collect();
            assert_eq!(
                path.iter().map(|b| b.kind.as_str()).collect::<Vec<_>>(),
                ["family", "stratum"]
            );
            assert_eq!(path[1].parent.as_ref(), Some(&path[0].id));
            let coverage = review
                .groups
                .iter()
                .find(|g| g.id == mission.group)
                .unwrap();
            assert_eq!(path[1].id, coverage.branch);
            assert_eq!(mission.direction, coverage.direction);
            assert_eq!(mission.attempts.len(), page.approaches.len());
        }
        let same_stratum: Vec<_> = review
            .missions
            .iter()
            .filter(|m| m.label.starts_with("Value noise") && m.label.ends_with("stratum 0"))
            .collect();
        assert_eq!(same_stratum.len(), 2);
        assert_eq!(same_stratum[0].path, same_stratum[1].path);
        assert_ne!(same_stratum[0].direction, same_stratum[1].direction);
        assert_ne!(same_stratum[0].group, same_stratum[1].group);
        assert!(same_stratum[0].mission_index < same_stratum[1].mission_index);
        let mut prior_order: Vec<_> = page.approaches[0].campaigns[0]
            .page
            .attempts
            .iter()
            .collect();
        prior_order.sort_by_key(|r| {
            (
                crate::paired_review::coverage_label(
                    &r.terrain_family,
                    r.direction,
                    r.terrain_stratum,
                ),
                r.mission_index,
                &r.physical_sha256,
            )
        });
        for (ordinal, row) in prior_order.iter().enumerate() {
            let mission = review
                .missions
                .iter()
                .find(|m| m.physical_sha256 == row.physical_sha256)
                .unwrap();
            assert_eq!(mission.id, format!("mission-{ordinal}"));
        }
        let campaign =
            crate::campaign::render_campaign_page(&page.approaches[0].campaigns[0].page).unwrap();
        assert!(campaign.contains("data-kind=\"family\" data-depth=\"0\""));
        assert!(!campaign.contains("data-kind=\"direction\""));
        assert!(campaign.contains("data-kind=\"stratum\" data-depth=\"1\""));
        assert!(campaign.contains("data-depth=\"2\" data-direction="));
        assert!(campaign.contains("id=\"campaign-direction\""));
    }
}
