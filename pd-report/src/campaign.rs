//! Common presentation-only campaign projection. Callers own evaluation and
//! immutable evidence; this module never executes or imports an approach.
use std::collections::BTreeMap;

use pd_core::{BoundedRunStopCauseV1, MissionOutcome, PhysicalOutcome};
use pd_run::AgentDescriptorV1;
use serde::{Deserialize, Serialize};

use crate::{batch, batch_tree};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignRowV1 {
    pub attempt_index: u64,
    pub mission_index: u64,
    pub physical_sha256: String,
    pub terrain_family: String,
    pub direction: i32,
    pub terrain_stratum: u32,
    pub disposition: String,
    pub detail: Option<String>,
    pub stop: Option<BoundedRunStopCauseV1>,
    pub physical_outcome: Option<PhysicalOutcome>,
    pub mission_outcome: Option<MissionOutcome>,
    pub execution_valid: bool,
    pub integrity_passed: bool,
    pub physical_replay_verified: bool,
    pub complete_replay_verified: bool,
    pub verified_target_landing: bool,
    pub sim_time_s: Option<f64>,
    pub fuel_kg: Option<f64>,
    pub wall_time_us: Option<u64>,
    pub report_href: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignPageV1 {
    pub schema_version: u32,
    pub title: String,
    pub agent: AgentDescriptorV1,
    pub integrity_passed: bool,
    pub unique_scheduled_missions: usize,
    pub manifest_sha256: String,
    pub collection_role: String,
    pub prior_training_or_tuning: bool,
    pub recorded_input_inspection: bool,
    pub attempts: Vec<CampaignRowV1>,
    pub summary_href: String,
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn verified(row: &CampaignRowV1) -> bool {
    row.verified_target_landing
        && row.execution_valid
        && row.integrity_passed
        && row.physical_replay_verified
        && row.complete_replay_verified
        && row.stop == Some(BoundedRunStopCauseV1::MissionTerminal)
        && row.physical_outcome == Some(PhysicalOutcome::LandedOnTarget)
        && row.mission_outcome == Some(MissionOutcome::Success)
}

fn number(value: Option<f64>) -> String {
    value.map_or_else(|| "—".into(), |n| format!("{n:.2}"))
}

pub fn render_campaign_page(page: &CampaignPageV1) -> Result<String, String> {
    page.agent.validate()?;
    if page.schema_version != 1
        || page.attempts.is_empty()
        || page.unique_scheduled_missions > page.attempts.len()
        || page
            .attempts
            .iter()
            .any(|r| r.verified_target_landing && !verified(r))
    {
        return Err("invalid common campaign projection or incomplete landing tuple".into());
    }
    let scheduled = page.attempts.len();
    let landings = if page.integrity_passed {
        page.attempts.iter().filter(|r| verified(r)).count()
    } else {
        0
    };
    let unsupported = page
        .attempts
        .iter()
        .filter(|r| r.disposition == "unsupported")
        .count();
    let executed = page
        .attempts
        .iter()
        .filter(|r| r.disposition == "executed")
        .count();
    let replayed = page
        .attempts
        .iter()
        .filter(|r| r.complete_replay_verified)
        .count();
    let overview = batch::render_overview_section(
        "",
        &batch::render_overview_table(
            "<th>Approach</th><th>Scheduled attempts</th><th>Unique missions</th><th>Verified landing</th><th>Unsupported</th><th>Executed</th><th>Complete replay</th><th>Campaign integrity</th>",
            &format!(
                "<tr><td><code>{}</code></td><td>{scheduled}</td><td>{}</td><td>{landings}/{scheduled}</td><td>{unsupported}</td><td>{executed}</td><td>{replayed}/{executed}</td><td>{}</td></tr>",
                escape(&page.agent.implementation),
                page.unique_scheduled_missions,
                page.integrity_passed
            ),
        ),
    );
    let mut groups: BTreeMap<String, Vec<&CampaignRowV1>> = BTreeMap::new();
    for row in &page.attempts {
        groups
            .entry(format!(
                "{} · direction {} · stratum {}",
                row.terrain_family, row.direction, row.terrain_stratum
            ))
            .or_default()
            .push(row);
    }
    let headers = "<th>Group / attempt</th><th>Physical input</th><th>Disposition / stop</th><th>Physical / mission</th><th>Verified landing</th><th>Integrity / replay</th><th>Flight s</th><th>Fuel kg</th><th>Application wall ms</th><th>Evidence</th>";
    let mut coverage_rows = String::new();
    let mut tree_rows = String::new();
    for (ordinal, (label, rows)) in groups.iter().enumerate() {
        let group = format!("campaign-group-{ordinal}");
        let landed = if page.integrity_passed {
            rows.iter().filter(|r| verified(r)).count()
        } else {
            0
        };
        coverage_rows.push_str(&batch::render_coverage_row(
            &escape(label),
            &batch::render_coverage_group_cell(
                "coverage-cell",
                &group,
                &format!("{landed}/{}", rows.len()),
            ),
        ));
        tree_rows.push_str(&batch::render_row(Some("summary-row"),
            &format!("data-group=\"{group}\" data-kind=\"group\" data-depth=\"0\" aria-expanded=\"false\" tabindex=\"0\""),
            &format!("<td><span class=\"expander\">+</span> {}</td><td colspan=\"9\">{landed}/{} verified target landings</td>",escape(label),rows.len())));
        for row in rows {
            let proof = format!(
                "{} / {}",
                row.integrity_passed, row.complete_replay_verified
            );
            let outcome = format!("{:?} / {:?}", row.physical_outcome, row.mission_outcome);
            let disposition = format!("{} / {:?}", row.disposition, row.stop);
            let link = row.report_href.as_ref().map_or_else(
                || "—".into(),
                |href| format!("<a href=\"{}\">Run report</a>", escape(href)),
            );
            tree_rows.push_str(&batch::render_row(Some("campaign-row"),
                &format!("data-parent=\"{group}\" data-depth=\"1\""),
                &format!("<td>Attempt {} · mission {}</td><td><code>{}</code></td><td title=\"{}\">{}</td><td>{}</td><td>{}</td><td>{proof}</td><td>{}</td><td>{}</td><td>{}</td><td>{link}</td>",
                    row.attempt_index,row.mission_index,escape(&row.physical_sha256),
                    escape(row.detail.as_deref().unwrap_or("")),escape(&disposition),escape(&outcome),
                    page.integrity_passed&&verified(row),number(row.sim_time_s),number(row.fuel_kg),
                    number(row.wall_time_us.map(|v|v as f64/1000.0)))));
        }
    }
    let coverage = batch::render_coverage_section(
        "All scheduled inputs remain in each denominator, including unsupported inputs.",
        "",
        &batch::render_coverage_table(
            "Family / direction / stratum",
            "<th>Verified landing / scheduled</th>",
            &coverage_rows,
        ),
    );
    let tree = batch::render_review_tree_section(
        &batch_tree::render_controls("Attempts", false),
        "<p>Expand a coverage group to inspect every scheduled attempt and its saved native evidence.</p>",
        &batch::render_review_tree_table("procedural-campaign", headers, &tree_rows),
    );
    let config = escape(&serde_json::to_string_pretty(&page.agent).map_err(|e| e.to_string())?);
    let context = format!(
        "<section class=\"context-section\"><h2>Configuration and provenance</h2><p>Original collection role: {}. Recorded training/tuning exposure: {}. Recorded input inspection: {}.</p><p>Manifest SHA-256 <code>{}</code>. Overall campaign integrity: <strong>{}</strong>. Flight success requires the physical, mission, validity, integrity and complete native replay tuple.</p><details><summary>Effective approach descriptor</summary><pre>{config}</pre></details></section>",
        escape(&page.collection_role),
        page.prior_training_or_tuning,
        page.recorded_input_inspection,
        escape(&page.manifest_sha256),
        page.integrity_passed
    );
    let actions = format!(
        "<a href=\"{}\">Campaign evidence</a> <a href=\"report-input.json\">Report projection</a>",
        escape(&page.summary_href)
    );
    Ok(batch::render_batch_page(batch::BatchPage {
        title: &page.title,
        subtitle: "Complete procedural missions under original physical, fuel and time budgets",
        chips_html: &format!(
            "<span class=\"chip\"><strong>{landings}/{scheduled}</strong> verified target landings</span>"
        ),
        actions_html: &actions,
        before_hero_html: "",
        after_hero_html: "",
        overview_html: &overview,
        planner_html: "",
        coverage_html: &coverage,
        context_html: &context,
        diagnostics_html: "",
        review_tree_html: &tree,
        comparison_html: "",
        appendix_html: "",
        body_class: "",
        tree: batch::BatchTreeOptions {
            max_depth: 1,
            depth_by_kind: &[("group", 0)],
            leaf_selector: "tr.campaign-row",
            default_expansion: 0,
        },
        coverage_tree_jump: true,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_campaign_cannot_promote_individually_verified_landings() {
        let page = CampaignPageV1 {
            schema_version: 1,
            title: "A <campaign>".into(),
            agent: AgentDescriptorV1 {
                schema_version: 1,
                implementation: "independent".into(),
                configuration: serde_json::json!({}),
                checkpoint_sha256: None,
            },
            integrity_passed: false,
            unique_scheduled_missions: 1,
            manifest_sha256: "0".repeat(64),
            collection_role: "Development".into(),
            prior_training_or_tuning: false,
            recorded_input_inspection: false,
            summary_href: "summary.json".into(),
            attempts: vec![CampaignRowV1 {
                attempt_index: 0,
                mission_index: 0,
                physical_sha256: "1".repeat(64),
                terrain_family: "layered".into(),
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
                sim_time_s: Some(1.),
                fuel_kg: Some(1.),
                wall_time_us: Some(1),
                report_href: Some("attempt-000000/report.html".into()),
            }],
        };
        let html = render_campaign_page(&page).unwrap();
        assert!(html.contains("data-batch-template=\"common-v1\""));
        assert!(html.contains("0/1</strong> verified target landings"));
        assert!(html.contains("A &lt;campaign&gt;"));
        assert!(html.contains("Run report"));
        assert!(html.contains("data-parent=\"campaign-group-0\""));
        let mut bad = page;
        bad.attempts[0].complete_replay_verified = false;
        assert!(render_campaign_page(&bad).is_err());
    }
}
