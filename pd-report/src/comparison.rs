//! Paired saved-campaign comparison without importing an evaluator or strategy.
use std::collections::{BTreeMap, BTreeSet};

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

type InputInventory = BTreeMap<String, (String, i32, Option<u32>, String, usize)>;

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
            let geometry = serde_json::to_string(&row.geometry).map_err(|e| e.to_string())?;
            let entry = result.entry(row.physical_sha256.clone()).or_insert((
                row.terrain_family.clone(),
                row.direction,
                row.terrain_stratum,
                geometry.clone(),
                0,
            ));
            if entry.0 != row.terrain_family
                || entry.1 != row.direction
                || entry.2 != row.terrain_stratum
                || entry.3 != geometry
            {
                return Err("physical input has inconsistent coverage metadata".into());
            }
            entry.4 += 1;
        }
    }
    Ok(result)
}

fn validate_comparison(page: &ComparisonPageV1) -> Result<InputInventory, String> {
    if page.schema_version != 1 || page.approaches.len() < 2 {
        return Err("comparison requires at least two declared approaches".into());
    }
    let reference = inventory(&page.approaches[0])?;
    let canonical_role = |role: &str| {
        let role = role.to_ascii_lowercase().replace('-', "_");
        if role == "heldout" {
            "held_out".into()
        } else {
            role
        }
    };
    let campaigns: Vec<_> = page
        .approaches
        .iter()
        .flat_map(|a| a.campaigns.iter())
        .collect();
    let role = canonical_role(&campaigns[0].page.collection_role);
    if campaigns
        .iter()
        .any(|c| canonical_role(&c.page.collection_role) != role)
    {
        return Err("comparison cannot pool different collection roles".into());
    }
    let declared = campaigns.iter().any(|c| c.page.cohort.is_some());
    if declared {
        let mut per_family = BTreeMap::new();
        let mut population = None;
        for campaign in &campaigns {
            let cohort = campaign
                .page
                .cohort
                .as_ref()
                .ok_or("comparison mixes declared and unrecorded populations")?;
            let key = (&cohort.population, &cohort.population_sha256);
            if population.is_some_and(|p| p != key) {
                return Err("comparison cannot pool different declared populations".into());
            }
            population = Some(key);
            for row in &campaign.page.attempts {
                if let Some(previous) = per_family.insert(&row.terrain_family, cohort)
                    && previous != cohort
                {
                    return Err(
                        "comparison changes recipe/admission/geometry declaration within a backend"
                            .into(),
                    );
                }
            }
        }
    }
    for approach in &page.approaches[1..] {
        if inventory(approach)? != reference {
            return Err(
                "comparison approaches do not share the same physical inputs and repeat schedule"
                    .into(),
            );
        }
    }
    Ok(reference)
}

/// Compose disjoint saved populations, retaining each campaign and approach.
/// Labels align approaches; descriptors and physical inventories remain checked.
pub fn combine_comparison_pages(pages: &[ComparisonPageV1]) -> Result<ComparisonPageV1, String> {
    let first = pages.first().ok_or("no saved comparisons supplied")?;
    let mut combined = first.clone();
    let mut identities = BTreeSet::new();
    let labels: BTreeSet<_> = first.approaches.iter().map(|a| a.label.as_str()).collect();
    for (ordinal, page) in pages.iter().enumerate() {
        let inputs = validate_comparison(page)?;
        let current: BTreeSet<_> = page.approaches.iter().map(|a| a.label.as_str()).collect();
        if current != labels || current.len() != page.approaches.len() {
            return Err("saved comparisons require the same unique approach labels".into());
        }
        for identity in inputs.keys() {
            if !identities.insert(identity.clone()) {
                return Err("saved populations overlap in physical identity".into());
            }
        }
        if ordinal > 0 {
            for approach in &mut combined.approaches {
                let source = page
                    .approaches
                    .iter()
                    .find(|a| a.label == approach.label)
                    .unwrap();
                if source.campaigns[0].page.agent != approach.campaigns[0].page.agent {
                    return Err("saved comparisons change an approach descriptor".into());
                }
                approach.campaigns.extend(source.campaigns.iter().cloned());
            }
        }
    }
    validate_comparison(&combined)?;
    Ok(combined)
}

pub fn render_comparison_page(page: &ComparisonPageV1) -> Result<String, String> {
    let reference = validate_comparison(page)?;
    let families: BTreeSet<_> = reference.values().map(|v| v.0.as_str()).collect();
    let multiple_families = families.len() > 1;
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
        for family in &families {
            let family_rows: Vec<_> = rows
                .iter()
                .copied()
                .filter(|(r, _)| &r.terrain_family == family)
                .collect();
            let scheduled = family_rows.len();
            let landed = family_rows
                .iter()
                .filter(|(r, ok)| *ok && verified(r))
                .count();
            let label = if multiple_families {
                format!("{} · {}", family_label(family), approach.label)
            } else {
                approach.label.clone()
            };
            let bar = crate::campaign_view::render_outcome_bar(&label, &family_rows, ordinal);
            if multiple_families {
                let branch = review
                    .missions
                    .iter()
                    .find(|m| m.physical_sha256 == family_rows[0].0.physical_sha256)
                    .expect("validated family branch");
                cards.push_str(&format!(
                    "<section data-outcome-family=\"{}\">{bar}</section>",
                    branch.path[0]
                ));
            } else {
                cards.push_str(&bar);
            }
            let unique = reference.values().filter(|v| &v.0 == family).count();
            accounting.push_str(&format!(
                "<tr>{}<td>{}</td><td>{scheduled}</td><td>{unique}</td><td>{landed}/{scheduled}</td></tr>",
                if multiple_families {format!("<td>{}</td>", escape(family_label(family)))} else {String::new()},
                escape(&approach.label)
            ));
        }
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
    let geometry_coverage = page.approaches[0]
        .campaigns
        .iter()
        .flat_map(|campaign| &campaign.page.attempts)
        .all(|row| row.geometry.is_some());
    let headline = if geometry_coverage {
        let approach = &page.approaches[incumbent];
        let integrity = approach.campaigns.iter().all(|c| c.page.integrity_passed);
        let rows: Vec<_> = approach
            .campaigns
            .iter()
            .flat_map(|c| &c.page.attempts)
            .collect();
        families
            .iter()
            .map(|family| {
                let rows: Vec<_> = rows
                    .iter()
                    .filter(|row| &row.terrain_family == family)
                    .collect();
                let landed = rows.iter().filter(|row| integrity && verified(row)).count();
                format!(
                    "<span class=\"chip\">{}{}: {landed}/{} verified landings ({:.1}%)</span>",
                    if multiple_families {
                        format!("{} · ", escape(family_label(family)))
                    } else {
                        String::new()
                    },
                    escape(&approach.label),
                    rows.len(),
                    100.0 * landed as f64 / rows.len().max(1) as f64
                )
            })
            .collect::<String>()
    } else {
        String::new()
    };
    let shortcuts = format!(
        "<div class=\"review-shortcuts\"><button type=\"button\" data-review-outcome=\"misses\" data-approach=\"{incumbent}\">Review incumbent misses</button><button type=\"button\" data-review-outcome=\"unsupported\" data-approach=\"all\">Review unsupported inputs</button><button type=\"button\" data-review-outcome=\"not-launched\" data-approach=\"all\">Review stopped before departure</button></div>"
    );
    let denominator_note = if multiple_families {
        "Each terrain family is scored separately. Each bar uses that family's full scheduled denominator."
    } else {
        "Each bar uses the full scheduled denominator."
    };
    let overview = format!(
        "<section class=\"header-overview\"><h2>Complete procedural missions</h2><p class=\"section-note\">Source-pad rest → target-pad landing, with procedural terrain along the complete transfer. Browse the mission structure below, then inspect saved controller results for each input.</p><details class=\"controller-overview\"><summary>Controller outcomes across this mission set</summary>{shortcuts}<p class=\"section-note\">{denominator_note} Click an outcome to find those missions. Replay proof is separate from the physical outcome.</p><div class=\"outcome-cards\">{cards}</div><details><summary>Schedule accounting</summary><div class=\"table-wrap\">{}</div></details></details></section>",
        batch::render_overview_table(
            &format!(
                "{}<th>Approach</th><th>Scheduled attempts</th><th>Unique physical inputs</th><th>Verified landings</th>",
                if multiple_families {
                    "<th>Terrain family</th>"
                } else {
                    ""
                }
            ),
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
        "<details class=\"paired-coverage\"><summary>Coverage · {} {} groups</summary>{}</details>",
        review.groups.len(),
        if geometry_coverage {
            "backend / direction / morphology / barrier"
        } else {
            "family / direction / stratum"
        },
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
            "<th>Mission / occurrence</th><th>Declared run(s)</th>",
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
            "{headline}<span class=\"chip\">{} unique physical inputs · {} scheduled occurrences · {} approaches</span>",
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
                    cohort: None,
                    prior_training_or_tuning: Some(false),
                    recorded_input_inspection: Some(false),
                    summary_href: "summary.json".into(),
                    attempts: vec![CampaignRowV1 {
                        attempt_index: 0,
                        mission_index: 0,
                        physical_sha256: "b".repeat(64),
                        terrain_family: "pd-missions/value-noise-rest-v1".into(),
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
    fn role_population_and_backend_contracts_cannot_be_pooled() {
        use crate::campaign::CampaignCohortV1;
        let mut page = comparison();
        page.approaches[1].campaigns[0].page.collection_role = "held_out".into();
        assert!(render_comparison_page(&page).is_ok());
        page.approaches[1].campaigns[0].page.collection_role = "validation".into();
        assert!(render_comparison_page(&page).is_err());
        page.approaches[1].campaigns[0].page.collection_role = "HeldOut".into();
        let cohort = CampaignCohortV1 {
            schema_version: 1,
            population: "broad".into(),
            population_sha256: "0".repeat(64),
            recipe_sha256: "1".repeat(64),
            geometry_spec_sha256: Some("2".repeat(64)),
            challenge_rule_sha256: Some("3".repeat(64)),
            morphology_spec_sha256: Some("4".repeat(64)),
        };
        for a in &mut page.approaches {
            a.campaigns[0].page.cohort = Some(cohort.clone());
        }
        assert!(render_comparison_page(&page).is_ok());
        let mut disjoint = page.clone();
        for a in &mut disjoint.approaches {
            a.campaigns[0].page.attempts[0].physical_sha256 = "c".repeat(64);
            let c = a.campaigns[0].page.cohort.as_mut().unwrap();
            c.population = "stress".into();
            c.population_sha256 = "5".repeat(64);
        }
        assert!(combine_comparison_pages(&[page.clone(), disjoint.clone()]).is_err());
        for a in &mut disjoint.approaches {
            let c = a.campaigns[0].page.cohort.as_mut().unwrap();
            *c = cohort.clone();
            c.recipe_sha256 = "6".repeat(64);
        }
        assert!(combine_comparison_pages(&[page.clone(), disjoint]).is_err());
        page.approaches[1].campaigns[0].page.cohort = None;
        assert!(render_comparison_page(&page).is_err());
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
    fn inline_fixture_exports_use_the_real_campaign_and_comparison_renderers_when_requested() {
        let Ok(root) = std::env::var("PD_INLINE_FIXTURE_OUTPUT") else {
            return;
        };
        use std::fs::{self, OpenOptions};
        use std::io::Write;

        fn create(root: &std::path::Path, path: &str, content: &[u8]) {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap();
            file.write_all(content).unwrap();
        }

        let root = std::path::Path::new(&root);
        assert!(
            root.is_absolute(),
            "PD_INLINE_FIXTURE_OUTPUT must be an absolute path"
        );
        fs::create_dir_all(root).unwrap();
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="156" height="92" viewBox="0 0 156 92"><path d="M8 84 76 49 148 8" fill="none" stroke="#356a96" stroke-width="2"/><path d="M120 84h30" stroke="#2f9e44" stroke-width="4"/></svg>"##;

        let mut source = comparison();
        let mut single = source.approaches.remove(0).campaigns.remove(0).page;
        single.title = "Software rendered single campaign fixture".into();
        single.integrity_passed = true;
        single.attempts[0].presentation = Some(crate::campaign_view::CampaignPresentationV1 {
            schema_version: 1,
            preview_href: "previews/single.svg".into(),
            caption: "Software rendering fixture · one saved run".into(),
            target_horizontal_offset_m: Some(12.5),
            height_above_upright_touchdown_m: Some(4.0),
            endpoint_speed_mps: Some(0.8),
        });
        create(
            root,
            "single-campaign.html",
            crate::campaign::render_campaign_page(&single)
                .unwrap()
                .as_bytes(),
        );
        create(root, "previews/single.svg", svg);

        let mut paired = comparison();
        paired.title = "Software rendered non-study comparison fixture".into();
        for (index, approach) in paired.approaches.iter_mut().enumerate() {
            approach.campaigns[0].page.integrity_passed = true;
            approach.campaigns[0].page.attempts[0].presentation =
                Some(crate::campaign_view::CampaignPresentationV1 {
                    schema_version: 1,
                    preview_href: format!("previews/approach-{index}.svg"),
                    caption: format!("Software rendering fixture · approach {index}"),
                    target_horizontal_offset_m: Some(12.5 + index as f64),
                    height_above_upright_touchdown_m: Some(4.0),
                    endpoint_speed_mps: Some(0.8 + index as f64),
                });
        }
        create(
            root,
            "non-study-comparison.html",
            render_comparison_page(&paired).unwrap().as_bytes(),
        );
        create(root, "previews/approach-0.svg", svg);
        create(root, "previews/approach-1.svg", svg);
    }

    fn population(family: &str, identity: &str, repeats: usize) -> ComparisonPageV1 {
        let mut page = comparison();
        page.approaches[0].label = "Incumbent reflection".into();
        for approach in &mut page.approaches {
            let campaign = &mut approach.campaigns[0].page;
            let row = &mut campaign.attempts[0];
            row.physical_sha256 = identity.repeat(64);
            row.terrain_family = family.into();
            row.terrain_stratum = None;
            row.geometry = Some(crate::campaign_view::CampaignGeometryV2 {
                schema_version: 2,
                morphology: "Mixed morphology".into(),
                chord_barrier_m: 400.0,
                distance_m: 1200.0,
                signed_pad_rise_m: -300.0,
                source_width_m: 40.0,
                target_width_m: 50.0,
                crest_count: 3,
            });
            let row = row.clone();
            for occurrence in 1..repeats {
                let mut repeat = row.clone();
                repeat.attempt_index = occurrence as u64;
                campaign.attempts.push(repeat);
            }
        }
        page
    }

    #[test]
    fn joined_populations_keep_backend_rates_repeats_and_approach_alignment() {
        let layered = population("pd-missions/layered-perlin-rest-v2", "b", 2);
        let mut value = population("pd-missions/value-noise-rest-v2", "c", 1);
        value.approaches.reverse();
        let sources = [layered, value];
        let before = serde_json::to_value(&sources).unwrap();
        let combined = combine_comparison_pages(&sources).unwrap();
        assert_eq!(combined.approaches[0].label, "Incumbent reflection");
        assert_eq!(combined.approaches[0].campaigns.len(), 2);
        let review = crate::paired_review::build(&combined).unwrap();
        assert_eq!(review.missions.len(), 3);
        assert_eq!(
            review
                .branches
                .iter()
                .filter(|b| b.parent.is_none())
                .count(),
            2
        );
        assert_eq!(
            review.missions.iter().filter(|m| m.occurrence == 1).count(),
            1
        );
        assert!(
            review
                .missions
                .iter()
                .all(|m| m.path.len() == 3 && m.attempts.len() == 2)
        );
        let html = render_comparison_page(&combined).unwrap();
        assert!(html.contains("Layered Perlin · Incumbent reflection: 2/2 verified landings"));
        assert!(html.contains("Value noise · Incumbent reflection: 1/1 verified landings"));
        assert!(!html.contains("3/3 verified landings"));
        assert_eq!(html.matches("data-outcome-family=").count(), 4);
        assert!(html.contains(
            "<td>Layered Perlin</td><td>Incumbent reflection</td><td>2</td><td>1</td><td>2/2</td>"
        ));
        assert!(html.contains(
            "<td>Value noise</td><td>Incumbent reflection</td><td>1</td><td>1</td><td>1/1</td>"
        ));
        assert_eq!(serde_json::to_value(&sources).unwrap(), before);
    }

    #[test]
    fn joining_rejects_overlap_descriptor_changes_missing_approaches_and_unpaired_sources() {
        let layered = population("pd-missions/layered-perlin-rest-v2", "b", 2);
        let value = population("pd-missions/value-noise-rest-v2", "c", 1);
        assert!(combine_comparison_pages(&[]).is_err());
        assert!(
            combine_comparison_pages(&[layered.clone(), layered.clone()])
                .unwrap_err()
                .contains("overlap")
        );
        let mut changed = value.clone();
        changed.approaches[0].campaigns[0].page.agent.configuration =
            serde_json::json!({"changed":true});
        assert!(
            combine_comparison_pages(&[layered.clone(), changed])
                .unwrap_err()
                .contains("descriptor")
        );
        let mut changed = value.clone();
        changed.approaches[0].label = "different".into();
        assert!(
            combine_comparison_pages(&[layered.clone(), changed])
                .unwrap_err()
                .contains("labels")
        );
        let mut changed = value;
        changed.approaches[1].campaigns[0].page.attempts[0].physical_sha256 = "d".repeat(64);
        assert!(
            combine_comparison_pages(&[layered, changed])
                .unwrap_err()
                .contains("same physical inputs")
        );
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
    fn v2_measured_hierarchy_preserves_mixed_inputs_and_rejects_metadata_drift() {
        let mut page = comparison();
        for approach in &mut page.approaches {
            let row = &mut approach.campaigns[0].page.attempts[0];
            row.terrain_stratum = None;
            row.terrain_family = "pd-missions/layered-perlin-rest-v2".into();
            row.geometry = Some(crate::campaign_view::CampaignGeometryV2 {
                schema_version: 2,
                morphology: "Mixed morphology".into(),
                chord_barrier_m: 400.0,
                distance_m: 1200.0,
                signed_pad_rise_m: -300.0,
                source_width_m: 40.0,
                target_width_m: 50.0,
                crest_count: 3,
            });
        }
        let review = crate::paired_review::build(&page).unwrap();
        assert_eq!(review.missions.len(), 1);
        assert_eq!(review.missions[0].path.len(), 3);
        assert_eq!(
            review
                .branches
                .iter()
                .map(|b| b.kind.as_str())
                .collect::<Vec<_>>(),
            ["family", "morphology", "band"]
        );
        assert_eq!(review.groups[0].branch, review.missions[0].path[2]);
        let html = render_comparison_page(&page).unwrap();
        assert!(html.contains("Mixed morphology"));
        assert!(html.contains("Chord barrier ≥400 m"));
        let campaign =
            crate::campaign::render_campaign_page(&page.approaches[0].campaigns[0].page).unwrap();
        assert!(campaign.contains("data-kind=\"band\" data-depth=\"2\""));
        assert!(!campaign.contains("Terrain stratum 0"));
        page.approaches[1].campaigns[0].page.attempts[0]
            .geometry
            .as_mut()
            .unwrap()
            .distance_m += 1.0;
        assert!(render_comparison_page(&page).is_err());
        page.approaches[0].campaigns[0].page.attempts[0].terrain_stratum = Some(0);
        assert!(render_comparison_page(&page).is_err());
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
                row.terrain_stratum = Some(stratum);
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
                    r.terrain_stratum.unwrap(),
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
