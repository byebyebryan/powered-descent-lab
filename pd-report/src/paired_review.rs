//! Compact paired presentation, joined by physical identity and repeat occurrence.
use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    campaign::{escape, family_label},
    campaign_view,
    comparison::ComparisonPageV1,
};

#[derive(Serialize)]
pub struct PairedAttemptV1 {
    pub approach: usize,
    pub attempt_index: u64,
    pub outcome: String,
    pub cells: String,
    pub href: Option<String>,
    pub preview: Option<String>,
    pub caption: String,
}

#[derive(Serialize)]
pub struct PairedMissionV1 {
    pub id: String,
    pub group: String,
    pub path: Vec<String>,
    pub label: String,
    pub mission_index: u64,
    pub physical_sha256: String,
    pub occurrence: usize,
    pub attempts: Vec<PairedAttemptV1>,
}

#[derive(Serialize)]
pub struct PairedGroupV1 {
    pub id: String,
    pub label: String,
}

#[derive(Serialize)]
pub struct MissionBranchV1 {
    pub id: String,
    pub parent: Option<String>,
    pub label: String,
    pub depth: usize,
    pub kind: String,
}

#[derive(Serialize)]
pub struct PairedReviewV1 {
    pub schema_version: u32,
    pub labels: Vec<String>,
    pub groups: Vec<PairedGroupV1>,
    pub branches: Vec<MissionBranchV1>,
    pub missions: Vec<PairedMissionV1>,
}

pub fn coverage_label(family: &str, direction: i32, stratum: u32) -> String {
    format!(
        "{} · {} · stratum {stratum}",
        family_label(family),
        if direction < 0 {
            "left"
        } else if direction > 0 {
            "right"
        } else {
            "vertical"
        }
    )
}

/// Caller validates inventory parity first. Never selects the best repeat.
pub fn build(page: &ComparisonPageV1) -> Result<PairedReviewV1, String> {
    let mut inventories = Vec::new();
    for approach in &page.approaches {
        let integrity = approach.campaigns.iter().all(|c| c.page.integrity_passed);
        let mut rows: BTreeMap<&str, Vec<_>> = BTreeMap::new();
        for campaign in &approach.campaigns {
            for row in &campaign.page.attempts {
                rows.entry(&row.physical_sha256)
                    .or_default()
                    .push((row, integrity));
            }
        }
        inventories.push(rows);
    }
    let mut missions = Vec::new();
    for (identity, rows) in &inventories[0] {
        for (occurrence, (row, _)) in rows.iter().enumerate() {
            let label = coverage_label(&row.terrain_family, row.direction, row.terrain_stratum);
            let mut attempts = Vec::new();
            for (approach, inventory) in inventories.iter().enumerate() {
                let (attempt, integrity) = inventory
                    .get(identity)
                    .and_then(|r| r.get(occurrence))
                    .ok_or("paired repeat missing")?;
                let metric = |n: Option<f64>, unit: &str| {
                    n.map_or_else(|| "Unavailable".into(), |n| format!("{n:.2} {unit}"))
                };
                let endpoint = attempt.presentation.as_ref().map_or_else(String::new, |v| {
                    format!(
                        "<span class=\"row-note\">Target offset {} · endpoint speed {}</span>",
                        metric(v.target_horizontal_offset_m, "m"),
                        metric(v.endpoint_speed_mps, "m/s")
                    )
                });
                attempts.push(PairedAttemptV1 { approach, attempt_index:attempt.attempt_index,
                    outcome:campaign_view::outcome(attempt,*integrity).key().into(),
                    cells: format!("<td>{}</td><td>{}<span class=\"row-note\">Fuel remaining {}</span>{endpoint}</td><td>{}</td>",campaign_view::render_outcome_cell(attempt,*integrity),metric(attempt.sim_time_s,"s"),metric(attempt.fuel_kg,"kg"),escape(campaign_view::proof_label(attempt,*integrity))),
                    href:attempt.report_href.clone(), preview:attempt.presentation.as_ref().map(|v|v.preview_href.clone()),
                    caption:attempt.presentation.as_ref().map_or_else(||"No preview in this saved projection".into(),|v|v.caption.clone()) });
            }
            missions.push(PairedMissionV1 {
                id: String::new(),
                group: label.clone(),
                path: Vec::new(),
                label,
                mission_index: row.mission_index,
                physical_sha256: (*identity).into(),
                occurrence,
                attempts,
            });
        }
    }
    missions.sort_by(|a, b| {
        (&a.group, a.mission_index, a.occurrence, &a.physical_sha256).cmp(&(
            &b.group,
            b.mission_index,
            b.occurrence,
            &b.physical_sha256,
        ))
    });
    let mut groups = Vec::new();
    let mut ids = BTreeMap::new();
    let mut branches = Vec::new();
    let mut branch_ids = BTreeMap::new();
    for (index, mission) in missions.iter_mut().enumerate() {
        mission.id = format!("mission-{index}");
        let group = ids.entry(mission.group.clone()).or_insert_with(|| {
            let id = format!("coverage-{}", groups.len());
            groups.push(PairedGroupV1 {
                id: id.clone(),
                label: mission.label.clone(),
            });
            id
        });
        mission.group = group.clone();
        // Coverage identities and mission ordinals remain stable. The hierarchy
        // is a separate presentation of the recorded input axes, never agents.
        let row = inventories[0][mission.physical_sha256.as_str()][mission.occurrence].0;
        let family = family_label(&row.terrain_family);
        let direction = match row.direction.cmp(&0) {
            std::cmp::Ordering::Less => "Travel left",
            std::cmp::Ordering::Greater => "Travel right",
            std::cmp::Ordering::Equal => "Vertical",
        };
        let mut parent = None;
        for (depth, (kind, key, label)) in [
            ("family", row.terrain_family.clone(), family.to_owned()),
            (
                "direction",
                format!("{}/{}", row.terrain_family, row.direction),
                direction.into(),
            ),
            (
                "stratum",
                mission.group.clone(),
                format!("Terrain stratum {}", row.terrain_stratum),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let id = branch_ids
                .entry((kind, key))
                .or_insert_with(|| {
                    let id = if depth == 2 {
                        mission.group.clone()
                    } else {
                        format!("branch-{}", branches.len())
                    };
                    branches.push(MissionBranchV1 {
                        id: id.clone(),
                        parent: parent.clone(),
                        label,
                        depth,
                        kind: kind.into(),
                    });
                    id
                })
                .clone();
            mission.path.push(id.clone());
            parent = Some(id);
        }
    }
    Ok(PairedReviewV1 {
        schema_version: 1,
        labels: page.approaches.iter().map(|a| a.label.clone()).collect(),
        groups,
        branches,
        missions,
    })
}

pub fn data_script(review: &PairedReviewV1) -> Result<String, String> {
    let json = serde_json::to_string(review)
        .map_err(|e| e.to_string())?
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    Ok(format!(
        "<script type=\"application/json\" id=\"paired-review-data\">{json}</script><script>{}</script>",
        format_args!(
            "{}{}",
            include_str!("paired_model.js"),
            include_str!("paired_review.js")
        )
    ))
}

pub fn controls(review: &PairedReviewV1) -> String {
    let options = review
        .labels
        .iter()
        .enumerate()
        .map(|(i, label)| format!("<option value=\"{i}\">{}</option>", escape(label)))
        .collect::<String>();
    format!(
        "<p class=\"section-note\">Complete procedural transfers → terrain family → travel direction → terrain stratum → mission. Controller results sit inside each mission.</p><div class=\"paired-controls\"><label>Show results for<select id=\"review-view\">{options}</select></label><label>Find mission<input id=\"review-search\" type=\"search\" placeholder=\"Number, terrain or physical identity\"></label><label>Match outcome<select id=\"review-outcome\"><option value=\"all\">All outcomes</option><option value=\"misses\">No verified landing</option>{}</select></label><label>Match approach<select id=\"review-approach\"><option value=\"all\">Any approach</option>{options}</select></label></div><div class=\"paired-controls hierarchy-controls\"><button type=\"button\" data-paired-depth=\"1\">Expand families</button><button type=\"button\" data-paired-depth=\"2\">Expand directions</button><button type=\"button\" data-paired-depth=\"3\">Browse missions</button><button type=\"button\" data-paired-action=\"collapse\">Collapse</button><button type=\"button\" data-paired-action=\"reset\">Reset filters</button></div><p class=\"section-note\">Open one stratum at a time. Browse 12 missions per page; summary counts cover every match.</p><p id=\"review-count\" aria-live=\"polite\"></p><details class=\"comparison-options\"><summary>Preview comparison options</summary><div class=\"paired-controls\"><label>Compare A<select id=\"review-a\">{options}</select></label><label>Compare B<select id=\"review-b\">{options}</select></label></div><p class=\"section-note\">Expand a mission to inspect all saved controller attempts and compare two previews with shared axes. Filters retain every scheduled occurrence.</p></details>",
        campaign_view::DisplayOutcome::ALL
            .into_iter()
            .map(|o| format!("<option value=\"{}\">{}</option>", o.key(), o.label()))
            .collect::<String>()
    )
}
