//! Common presentation-only campaign projection. Callers own evaluation and
//! immutable evidence; this module never executes or imports an approach.
use std::collections::BTreeMap;

use pd_core::{BoundedRunStopCauseV1, MissionOutcome, PhysicalOutcome};
use pd_run::AgentDescriptorV1;
use serde::{Deserialize, Serialize};

use crate::campaign_view::{self, CampaignPresentationV1};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<CampaignPresentationV1>,
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
    #[serde(default = "unrecorded_role")]
    pub collection_role: String,
    #[serde(default)]
    pub prior_training_or_tuning: Option<bool>,
    #[serde(default)]
    pub recorded_input_inspection: Option<bool>,
    pub attempts: Vec<CampaignRowV1>,
    pub summary_href: String,
}

fn unrecorded_role() -> String {
    "Unrecorded".into()
}
fn exposure(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unrecorded",
    }
}

pub(crate) fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(crate) fn verified(row: &CampaignRowV1) -> bool {
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

pub(crate) fn family_label(version: &str) -> &str {
    match version {
        "pd-missions/value-noise-rest-v1" => "Value noise",
        "pd-missions/layered-perlin-rest-v1" => "Layered Perlin",
        other => other,
    }
}

pub(crate) fn validate_campaign_page(page: &CampaignPageV1) -> Result<(), String> {
    page.agent.validate()?;
    if page.schema_version != 1
        || page.attempts.is_empty()
        || page.unique_scheduled_missions > page.attempts.len()
        || page
            .attempts
            .iter()
            .any(|r| r.verified_target_landing && !verified(r))
        || page
            .attempts
            .iter()
            .any(|r| r.presentation.as_ref().is_some_and(|view| !view.validate()))
    {
        return Err("invalid common campaign projection or incomplete landing tuple".into());
    }
    Ok(())
}

pub fn render_campaign_page(page: &CampaignPageV1) -> Result<String, String> {
    validate_campaign_page(page)?;
    let scheduled = page.attempts.len();
    let landings = if page.integrity_passed {
        page.attempts.iter().filter(|r| verified(r)).count()
    } else {
        0
    };
    let outcome_rows: Vec<_> = page
        .attempts
        .iter()
        .map(|row| (row, page.integrity_passed))
        .collect();
    let overview = format!(
        "<section class=\"header-overview\"><h2>Outcomes</h2>{}<button type=\"button\" data-campaign-reset>Show all scheduled attempts</button></section>",
        campaign_view::render_outcome_bar("Scheduled missions", &outcome_rows, 0)
    );
    let mut groups: BTreeMap<String, Vec<&CampaignRowV1>> = BTreeMap::new();
    for row in &page.attempts {
        groups
            .entry(format!(
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
            ))
            .or_default()
            .push(row);
    }
    let headers = "<th>Mission / saved flight</th><th>Outcome / stop</th><th>Flight time</th><th>Fuel remaining</th><th>Evidence</th>";
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
            label,
            &batch::render_coverage_group_cell(
                "coverage-cell",
                &group,
                &format!("{landed}/{}", rows.len()),
            ),
        ));
        tree_rows.push_str(&batch::render_row(Some("summary-row"),
            &format!("data-group=\"{group}\" data-kind=\"group\" data-depth=\"0\" aria-expanded=\"false\" tabindex=\"0\""),
            &format!("<td><span class=\"expander\">+</span> {}</td><td colspan=\"4\">{landed}/{} verified target landings</td>",escape(label),rows.len())));
        for row in rows {
            tree_rows.push_str(&batch::render_row(Some("campaign-row"),
                &format!("data-parent=\"{group}\" data-depth=\"1\" data-outcome=\"{}\" data-search=\"{}\"",campaign_view::outcome(row,page.integrity_passed).key(),escape(&format!("{} {} {}",row.mission_index,row.physical_sha256,label))),
                &format!("<td class=\"tree-label\">{}</td><td>{}</td><td>{} s</td><td>{} kg</td><td>{}</td>",
                    campaign_view::render_mission_cell(row),campaign_view::render_outcome_cell(row,page.integrity_passed),number(row.sim_time_s),number(row.fuel_kg),campaign_view::render_evidence_cell(row,page.integrity_passed))));
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
        exposure(page.prior_training_or_tuning),
        exposure(page.recorded_input_inspection),
        escape(&page.manifest_sha256),
        page.integrity_passed
    );
    let actions = format!(
        "<a href=\"{}\">Campaign evidence</a> <a href=\"report-input.json\">Report projection</a>",
        escape(&page.summary_href)
    );
    Ok(batch::render_review_first_batch_page(batch::BatchPage {
        title: &page.title,
        subtitle: "Complete procedural missions under original physical, fuel and time budgets",
        chips_html: &format!(
            "<span class=\"chip\"><strong>{landings}/{scheduled}</strong> verified target landings</span>"
        ),
        actions_html: &actions,
        before_hero_html: &format!("<style>{}</style>", campaign_view::STYLE),
        after_hero_html: "",
        overview_html: &overview,
        planner_html: "",
        coverage_html: &coverage,
        context_html: &format!(
            "<details class=\"provenance-disclosure\"><summary>Configuration and provenance</summary>{context}</details>"
        ),
        diagnostics_html: "",
        review_tree_html: &tree,
        comparison_html: "",
        appendix_html: &format!("<script>{}</script>", campaign_view::SCRIPT),
        body_class: "procedural-campaign campaign-view",
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
            prior_training_or_tuning: Some(false),
            recorded_input_inspection: Some(false),
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
                presentation: None,
            }],
        };
        let html = render_campaign_page(&page).unwrap();
        assert!(html.contains("data-batch-template=\"common-v1\""));
        assert!(html.contains("0/1</strong> verified target landings"));
        assert!(html.contains("A &lt;campaign&gt;"));
        assert!(html.contains("Mission 0 · attempt 0"));
        assert!(html.contains("data-parent=\"campaign-group-0\""));
        let mut bad = page;
        bad.attempts[0].complete_replay_verified = false;
        assert!(render_campaign_page(&bad).is_err());
    }
}
