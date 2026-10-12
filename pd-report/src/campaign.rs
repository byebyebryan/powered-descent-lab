//! Common presentation-only campaign projection. Callers own evaluation and
//! immutable evidence; this module never executes or imports an approach.
use std::collections::BTreeMap;

use pd_core::{BoundedRunStopCauseV1, MissionOutcome, PhysicalOutcome};
use pd_run::AgentDescriptorV1;
use serde::{Deserialize, Serialize};

use crate::campaign_view::{self, CampaignGeometryV2, CampaignPresentationV1};
use crate::{batch, batch_tree};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignRowV1 {
    pub attempt_index: u64,
    pub mission_index: u64,
    pub physical_sha256: String,
    pub terrain_family: String,
    pub direction: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain_stratum: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<CampaignGeometryV2>,
    /// Explicit input failure; absence preserves the original V1/V2 encoding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_metadata_unavailable: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cohort: Option<CampaignCohortV1>,
    #[serde(default)]
    pub prior_training_or_tuning: Option<bool>,
    #[serde(default)]
    pub recorded_input_inspection: Option<bool>,
    pub attempts: Vec<CampaignRowV1>,
    pub summary_href: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignCohortV1 {
    pub schema_version: u32,
    pub population: String,
    pub population_sha256: String,
    pub recipe_sha256: String,
    pub geometry_spec_sha256: Option<String>,
    pub challenge_rule_sha256: Option<String>,
    pub morphology_spec_sha256: Option<String>,
}

impl CampaignCohortV1 {
    fn validate(&self) -> bool {
        let digest = |s: &str| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit());
        self.schema_version == 1
            && matches!(
                self.population.as_str(),
                "legacy_v1" | "broad" | "stress" | "shift"
            )
            && digest(&self.recipe_sha256)
            && digest(&self.population_sha256)
            && [
                &self.geometry_spec_sha256,
                &self.challenge_rule_sha256,
                &self.morphology_spec_sha256,
            ]
            .into_iter()
            .flatten()
            .all(|s| digest(s))
            && (self.population == "legacy_v1"
                || [
                    &self.geometry_spec_sha256,
                    &self.challenge_rule_sha256,
                    &self.morphology_spec_sha256,
                ]
                .into_iter()
                .all(|v| v.is_some()))
    }
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
        "pd-missions/value-noise-rest-v2" => "Value noise",
        "pd-missions/layered-perlin-rest-v2" => "Layered Perlin",
        other => other,
    }
}

pub(crate) fn mission_path(row: &CampaignRowV1) -> Vec<(&'static str, String, String)> {
    let family = row.terrain_family.clone();
    let mut path = vec![("family", family.clone(), family_label(&family).to_owned())];
    if let Some(g) = &row.geometry {
        let key = format!("{family}/{}", g.morphology);
        path.push(("morphology", key.clone(), g.morphology.clone()));
        path.push((
            "band",
            format!("{key}/{}", g.barrier_band()),
            g.barrier_band().into(),
        ));
    } else if let Some(stratum) = row.terrain_stratum {
        path.push((
            "stratum",
            format!("{family}/{stratum}"),
            format!("Terrain stratum {stratum}"),
        ));
    } else if row.input_metadata_unavailable.is_some() {
        path.push((
            "metadata",
            format!("{family}/unavailable"),
            "Input metadata unavailable".into(),
        ));
    }
    path
}

pub(crate) fn row_coverage_label(row: &CampaignRowV1) -> String {
    if row.input_metadata_unavailable.is_some() {
        format!(
            "{} · input metadata unavailable",
            family_label(&row.terrain_family)
        )
    } else if let Some(g) = &row.geometry {
        format!(
            "{} · {} · {} · {}",
            family_label(&row.terrain_family),
            g.morphology,
            g.barrier_band(),
            if row.direction < 0 { "left" } else { "right" }
        )
    } else {
        crate::paired_review::coverage_label(
            &row.terrain_family,
            row.direction,
            row.terrain_stratum.expect("validated V1 stratum"),
        )
    }
}

pub(crate) fn validate_campaign_page(page: &CampaignPageV1) -> Result<(), String> {
    page.agent.validate()?;
    if page.schema_version != 1
        || page.cohort.as_ref().is_some_and(|c| !c.validate())
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
        || page.attempts.iter().any(|r| {
            match (
                &r.geometry,
                r.terrain_stratum,
                &r.input_metadata_unavailable,
            ) {
                (Some(g), None, None) => !g.validate(),
                (None, Some(_), None) => false,
                (None, None, Some(reason)) => {
                    reason.trim().is_empty()
                        || r.direction != 0
                        || r.disposition != "input_invalid"
                        || r.stop.is_some()
                        || r.physical_outcome.is_some()
                        || r.mission_outcome.is_some()
                        || r.execution_valid
                        || r.integrity_passed
                        || r.physical_replay_verified
                        || r.complete_replay_verified
                        || r.verified_target_landing
                        || r.sim_time_s.is_some()
                        || r.fuel_kg.is_some()
                        || r.report_href.is_some()
                        || r.presentation.is_some()
                }
                _ => true,
            }
        })
    {
        return Err("invalid common campaign projection or incomplete landing tuple".into());
    }
    Ok(())
}

pub fn render_campaign_page(page: &CampaignPageV1) -> Result<String, String> {
    render_campaign_with_links(page, "report-input.json", None)
}

/// A campaign companion inside a paired edition, with its own projection link.
pub fn render_campaign_edition_page(
    page: &CampaignPageV1,
    projection_href: &str,
    collection_href: &str,
) -> Result<String, String> {
    for href in [projection_href, collection_href] {
        if href.is_empty()
            || href.starts_with('/')
            || href.contains([':', '\\', '?', '#'])
            || href
                .split('/')
                .any(|p| p.is_empty() || p == ".." || p == ".")
            || href.chars().any(char::is_control)
        {
            return Err("unsafe campaign edition link".into());
        }
    }
    render_campaign_with_links(page, projection_href, Some(collection_href))
}

fn render_campaign_with_links(
    page: &CampaignPageV1,
    projection_href: &str,
    collection_href: Option<&str>,
) -> Result<String, String> {
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
    let mut groups: BTreeMap<Vec<(&str, String, String)>, Vec<&CampaignRowV1>> = BTreeMap::new();
    for row in &page.attempts {
        groups.entry(mission_path(row)).or_default().push(row);
    }
    let headers = "<th>Mission / saved flight</th><th>Outcome / stop</th><th>Flight time</th><th>Fuel remaining</th><th>Evidence</th>";
    let mut coverage_rows = String::new();
    let mut tree_rows = String::new();
    let mut branches = BTreeMap::new();
    for (ordinal, (path, rows)) in groups.iter_mut().enumerate() {
        rows.sort_by_key(|row| (row.mission_index, row.attempt_index));
        let label = path
            .iter()
            .map(|(_, _, label)| label.as_str())
            .collect::<Vec<_>>()
            .join(" · ");
        let group = format!("campaign-group-{ordinal}");
        let mut parent = None;
        for (depth, (kind, key, branch_label)) in path.iter().enumerate() {
            let branch_key = (*kind, key.clone());
            let id = branches.get(&branch_key).cloned().unwrap_or_else(|| {
                let id = if depth + 1 == path.len() { group.clone() } else { format!("campaign-axis-{depth}-{}", branches.len()) };
                let branch_rows: Vec<_> = page.attempts.iter().filter(|r| mission_path(r).get(depth).is_some_and(|p| p.0 == *kind && p.1 == *key)).collect();
                let landed = branch_rows.iter().filter(|r| page.integrity_passed && verified(r)).count();
                let parent_attr = parent.as_ref().map_or_else(String::new, |id| format!(" data-parent=\"{id}\""));
                tree_rows.push_str(&batch::render_row(Some("summary-row"),
                    &format!("data-group=\"{id}\"{parent_attr} data-kind=\"{kind}\" data-depth=\"{depth}\" aria-expanded=\"false\" tabindex=\"0\""),
                    &format!("<td><span class=\"expander\">+</span> {}</td><td colspan=\"4\">{landed}/{} verified target landings</td>", escape(branch_label), branch_rows.len())));
                branches.insert(branch_key, id.clone());
                id
            });
            parent = Some(id);
        }
        let parent = parent.expect("validated nonempty hierarchy");
        let landed = if page.integrity_passed {
            rows.iter().filter(|r| verified(r)).count()
        } else {
            0
        };
        coverage_rows.push_str(&batch::render_coverage_row(
            &label,
            &batch::render_coverage_group_cell(
                "coverage-cell",
                &group,
                &format!("{landed}/{}", rows.len()),
            ),
        ));
        for row in rows {
            let direction_key = if row.input_metadata_unavailable.is_some() {
                "unavailable".into()
            } else {
                row.direction.to_string()
            };
            tree_rows.push_str(&batch::render_row(Some("campaign-row"),
                &format!("data-parent=\"{parent}\" data-depth=\"{}\" data-direction=\"{}\" data-outcome=\"{}\" data-search=\"{}\"",path.len(),direction_key,campaign_view::outcome(row,page.integrity_passed).key(),escape(&format!("{} {} {}",row.mission_index,row.physical_sha256,format_args!("{label} {}",if row.input_metadata_unavailable.is_some() {"direction unavailable"} else if row.direction < 0 {"left"} else if row.direction > 0 {"right"} else {"vertical"})))),
                &format!("<td class=\"tree-label\">{}</td><td>{}</td><td>{} s</td><td>{} kg</td><td>{}</td>",
                    campaign_view::render_mission_cell(row),campaign_view::render_outcome_cell(row,page.integrity_passed),number(row.sim_time_s),number(row.fuel_kg),campaign_view::render_evidence_cell(row,page.integrity_passed))));
        }
    }
    let coverage = batch::render_coverage_section(
        "All scheduled inputs remain in each denominator, including unsupported inputs.",
        "",
        &batch::render_coverage_table(
            "Terrain / measured geometry",
            "<th>Verified landing / scheduled</th>",
            &coverage_rows,
        ),
    );
    let tree = batch::render_review_tree_section(
        &batch_tree::render_controls("Attempts", false),
        &format!(
            "<p>Terrain family → input geometry → mission and saved flight. V2 uses morphology and chord-barrier bands; V1 retains its authored strata. Summary totals cover the full collection.</p>{}<p id=\"campaign-filter-count\" aria-live=\"polite\"></p>",
            campaign_view::render_direction_filter("campaign-direction")
        ),
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
        "<a href=\"{}\">Campaign evidence</a> <a href=\"{}\">Report projection</a>",
        escape(&page.summary_href),
        escape(projection_href)
    );
    Ok(batch::render_review_first_batch_page(batch::BatchPage {
        title: &page.title,
        subtitle: "Complete procedural missions under original physical, fuel and time budgets",
        chips_html: &format!(
            "<span class=\"chip\"><strong>{landings}/{scheduled}</strong> verified target landings</span>"
        ),
        actions_html: &actions,
        before_hero_html: &format!("<style>{}</style>{}", campaign_view::STYLE,collection_href.map_or_else(String::new,|href|format!("<nav class=\"review-detail-nav\"><a href=\"{}\">Back to mission collection</a></nav>",escape(href)))),
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
            max_depth: if page.attempts.iter().any(|r| r.geometry.is_some()) { 3 } else { 2 },
            depth_by_kind: &[("family", 0), ("stratum", 1), ("morphology", 1), ("band", 2)],
            leaf_selector: "tr.campaign-row",
            default_expansion: 0,
        },
        coverage_tree_jump: true,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_page() -> CampaignPageV1 {
        CampaignPageV1 {
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
            cohort: None,
            prior_training_or_tuning: Some(false),
            recorded_input_inspection: Some(false),
            summary_href: "summary.json".into(),
            attempts: vec![CampaignRowV1 {
                attempt_index: 0,
                mission_index: 0,
                physical_sha256: "1".repeat(64),
                terrain_family: "layered".into(),
                direction: 1,
                terrain_stratum: Some(0),
                geometry: None,
                input_metadata_unavailable: None,
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
        }
    }

    #[test]
    fn invalid_campaign_cannot_promote_individually_verified_landings() {
        let page = fixture_page();
        let html = render_campaign_page(&page).unwrap();
        assert!(html.contains("data-batch-template=\"common-v1\""));
        assert!(html.contains("0/1</strong> verified target landings"));
        assert!(html.contains("A &lt;campaign&gt;"));
        assert!(html.contains("Mission 0 · attempt 0"));
        assert!(html.contains("data-parent=\"campaign-group-0\""));
        let edition =
            render_campaign_edition_page(&page, "campaign-input.json", "report.html").unwrap();
        assert!(edition.contains("href=\"campaign-input.json\""));
        assert!(edition.contains("Back to mission collection"));
        assert!(render_campaign_edition_page(&page, "../other.json", "report.html").is_err());
        assert!(render_campaign_edition_page(&page, "input.json", "javascript:bad").is_err());
        let mut bad = page;
        bad.attempts[0].complete_replay_verified = false;
        assert!(render_campaign_page(&bad).is_err());
    }

    #[test]
    fn unavailable_input_metadata_requires_explicit_failure_without_a_flight() {
        let mut page = fixture_page();
        let row = &mut page.attempts[0];
        row.terrain_stratum = None;
        row.direction = 0;
        row.disposition = "input_invalid".into();
        row.detail = Some("Original input bytes unavailable".into());
        row.stop = None;
        row.physical_outcome = None;
        row.mission_outcome = None;
        row.execution_valid = false;
        row.integrity_passed = false;
        row.physical_replay_verified = false;
        row.complete_replay_verified = false;
        row.verified_target_landing = false;
        row.sim_time_s = None;
        row.fuel_kg = None;
        row.report_href = None;
        assert!(render_campaign_page(&page).is_err());
        page.attempts[0].input_metadata_unavailable = Some("Input cannot be verified".into());
        let html = render_campaign_page(&page).unwrap();
        assert!(html.contains("Input metadata unavailable"));
        assert!(html.contains("Direction unavailable"));
        assert!(!html.contains("Terrain stratum 0"));
        assert!(!html.contains("Travel right"));
        page.attempts[0].direction = 1;
        assert!(render_campaign_page(&page).is_err());
        page.attempts[0].direction = 0;
        page.attempts[0].sim_time_s = Some(0.0);
        assert!(render_campaign_page(&page).is_err());
    }
}
