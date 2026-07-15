use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use pd_core::MissionOutcome;
use pd_report::site::ReportSite;
use serde::Deserialize;

use crate::{
    BatchComparison, BatchRegressionPolicyStatus, BatchReport, BatchRunAnalyticClass,
    BatchRunRecord,
};

#[derive(Clone, Debug, Deserialize)]
pub struct GuidanceCatalog {
    pub schema_version: u32,
    pub groups: Vec<GuidanceGroup>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GuidanceGroup {
    pub id: String,
    pub title: String,
    pub description: String,
    pub reports: Vec<GuidanceReport>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GuidanceReport {
    pub pack_id: String,
    pub label: String,
    pub role: GuidanceRole,
    pub evidence: String,
    #[serde(default)]
    pub pair_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuidanceRole {
    Primary,
    Supporting,
}

impl GuidanceRole {
    fn label(self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Supporting => "Supporting",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct EvidenceSummary {
    total: usize,
    core_success: usize,
    core_failure: usize,
    frontier_success: usize,
    frontier_failure: usize,
    impossible: usize,
}

impl EvidenceSummary {
    fn include(&mut self, other: &Self) {
        self.total += other.total;
        self.core_success += other.core_success;
        self.core_failure += other.core_failure;
        self.frontier_success += other.frontier_success;
        self.frontier_failure += other.frontier_failure;
        self.impossible += other.impossible;
    }

    fn core_total(&self) -> usize {
        self.core_success + self.core_failure
    }

    fn frontier_total(&self) -> usize {
        self.frontier_success + self.frontier_failure
    }
}

#[derive(Clone, Debug)]
struct CatalogEvidence {
    report: GuidanceReport,
    batch: Option<BatchReport>,
    comparison: Option<BatchComparison>,
    summary: EvidenceSummary,
}

#[derive(Clone, Debug, Deserialize)]
struct FixturePack {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    entries: Vec<FixtureEntry>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct FixtureEntry {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    expectation_tier: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum EvalCategory {
    Terminal,
    DirectTransfer,
    Waypoint,
    Diagnostic,
    Fixture,
}

impl EvalCategory {
    fn id(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::DirectTransfer => "direct-transfer",
            Self::Waypoint => "waypoint",
            Self::Diagnostic => "diagnostic",
            Self::Fixture => "fixture",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Terminal => "Terminal",
            Self::DirectTransfer => "Direct transfer",
            Self::Waypoint => "Waypoint",
            Self::Diagnostic => "Diagnostic / experimental",
            Self::Fixture => "Fixture / foundation",
        }
    }
}

pub fn load_guidance_catalog(repo_root: &Path) -> Result<GuidanceCatalog> {
    let path = repo_root.join("fixtures/reports/guidance_catalog.json");
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read guidance catalog {}", path.display()))?;
    let catalog = serde_json::from_str::<GuidanceCatalog>(&raw)
        .with_context(|| format!("failed to parse guidance catalog {}", path.display()))?;
    anyhow::ensure!(
        catalog.schema_version == 1,
        "unsupported guidance catalog schema"
    );
    Ok(catalog)
}

pub fn refresh_pack_ids(repo_root: &Path, all: bool) -> Result<Vec<String>> {
    let mut pack_ids = BTreeSet::new();
    if all {
        let packs_dir = repo_root.join("fixtures/packs");
        for entry in fs::read_dir(&packs_dir)
            .with_context(|| format!("failed to read fixture packs {}", packs_dir.display()))?
        {
            let path = entry?.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("failed to read fixture pack {}", path.display()))?;
            let fixture = serde_json::from_str::<FixturePack>(&raw)
                .with_context(|| format!("failed to parse fixture pack {}", path.display()))?;
            pack_ids.insert(fixture.id);
        }
    } else {
        for group in load_guidance_catalog(repo_root)?.groups {
            pack_ids.extend(group.reports.into_iter().map(|report| report.pack_id));
        }
    }
    Ok(pack_ids.into_iter().collect())
}

pub fn write_report_catalog(repo_root: &Path) -> Result<()> {
    let catalog = load_guidance_catalog(repo_root)?;
    let reports_root = repo_root.join("outputs/reports");
    let guidance_root = reports_root.join("guidance");
    fs::create_dir_all(&guidance_root).with_context(|| {
        format!(
            "failed to create guidance report directory {}",
            guidance_root.display()
        )
    })?;

    for group in &catalog.groups {
        let output_dir = guidance_root.join(&group.id);
        fs::create_dir_all(&output_dir)?;
        fs::write(
            output_dir.join("index.html"),
            render_group_page(repo_root, group),
        )?;
    }
    fs::write(
        guidance_root.join("index.html"),
        render_guidance_overview(repo_root, &catalog),
    )?;
    fs::create_dir_all(reports_root.join("eval"))?;
    fs::write(
        reports_root.join("eval/index.html"),
        render_eval_index(repo_root, &catalog)?,
    )?;
    ReportSite::new(repo_root).refresh_home()?;
    Ok(())
}

fn render_guidance_overview(repo_root: &Path, catalog: &GuidanceCatalog) -> String {
    let sections = catalog
        .groups
        .iter()
        .map(|group| {
            let primary = group
                .reports
                .iter()
                .filter(|report| report.role == GuidanceRole::Primary)
                .map(|report| load_evidence(repo_root, report))
                .collect::<Vec<_>>();
            let captured = primary.iter().filter(|item| item.batch.is_some()).count();
            let summary = primary.iter().fold(EvidenceSummary::default(), |mut total, item| {
                total.include(&item.summary);
                total
            });
            format!(
                r#"<a class="guidance-card" href="{id}/"><span class="eyebrow">{captured}/{count} primary reports captured</span><h2>{title}</h2><p>{description}</p>{metrics}</a>"#,
                id = escape_html(&group.id),
                captured = captured,
                count = primary.len(),
                title = escape_html(&group.title),
                description = escape_html(&group.description),
                metrics = render_guidance_metrics(&summary),
            )
        })
        .collect::<String>();
    page(
        "Guidance Overview",
        "Guidance Overview",
        "Curated evidence for the three maintained guidance responsibilities. Core outcomes are separated from scored frontier annotations and analytic impossibilities.",
        r#"<a href="../">reports/</a><a href="../eval/">all batch reports</a>"#,
        &format!(r#"<section class="guidance-grid">{sections}</section>"#),
    )
}

fn render_group_page(repo_root: &Path, group: &GuidanceGroup) -> String {
    let rows = group
        .reports
        .iter()
        .map(|report| render_evidence_row(&load_evidence(repo_root, report)))
        .collect::<String>();
    let body = format!(
        r#"<p class="score-note">Core outcomes exclude analytic frontier and impossible cases. Frontier failures remain scored by the regression policy, but are shown separately here so stress-boundary evidence does not read as ordinary guidance failure.</p><section class="scorecard"><div class="score-head"><span>Evidence</span><span>Core outcomes</span><span>Frontier / invalid</span><span>Capture</span></div>{rows}</section>"#
    );
    page(
        &group.title,
        &group.title,
        &group.description,
        r#"<a href="../">guidance overview</a><a href="../../eval/">all batch reports</a>"#,
        &body,
    )
}

fn render_evidence_row(item: &CatalogEvidence) -> String {
    let role = item.report.role.label();
    let pair = item
        .report
        .pair_id
        .as_deref()
        .map(|id| format!(r#"<span class="pair">pair {}</span>"#, escape_html(id)))
        .unwrap_or_default();
    let Some(batch) = item.batch.as_ref() else {
        return format!(
            r#"<div class="score-row missing"><div><span class="eyebrow">{role} · {evidence}</span><h3>{label}</h3>{pair}<code>{pack}</code></div><div class="muted">No captured result</div><div>—</div><div><span class="status missing">missing</span></div></div>"#,
            role = role,
            evidence = escape_html(&item.report.evidence),
            label = escape_html(&item.report.label),
            pair = pair,
            pack = escape_html(&item.report.pack_id),
        );
    };
    let comparison = item
        .comparison
        .as_ref()
        .map(|comparison| {
            let status = match comparison.policy.status {
                BatchRegressionPolicyStatus::Pass => "pass",
                BatchRegressionPolicyStatus::Warn => "warn",
                BatchRegressionPolicyStatus::Fail => "fail",
            };
            format!(r#"<span class="status {status}">compare {status}</span>"#)
        })
        .unwrap_or_else(|| r#"<span class="status quiet">standalone</span>"#.to_owned());
    let commit = batch
        .provenance
        .cache
        .as_ref()
        .map(|cache| cache.commit_key.as_str())
        .unwrap_or("unknown");
    format!(
        r#"<a class="score-row" href="../../eval/{pack}/"><div><span class="eyebrow">{role} · {evidence}</span><h3>{label}</h3>{pair}<code>{pack}</code></div>{outcomes}<div><span class="status captured">captured</span>{comparison}<div class="muted">{wall:.2}s wall · {commit}</div></div></a>"#,
        pack = escape_html(&item.report.pack_id),
        role = role,
        evidence = escape_html(&item.report.evidence),
        label = escape_html(&item.report.label),
        pair = pair,
        outcomes = render_evidence_outcomes(&item.summary),
        comparison = comparison,
        wall = batch.wall_clock_s,
        commit = escape_html(commit),
    )
}

fn render_guidance_metrics(summary: &EvidenceSummary) -> String {
    let failure_class = if summary.core_failure > 0 { "bad" } else { "" };
    format!(
        r#"<dl><div><dt>Core</dt><dd>{core_total}</dd></div><div><dt>Pass</dt><dd>{core_success}</dd></div><div><dt>Fail</dt><dd class="{failure_class}">{core_failure}</dd></div></dl><div class="guidance-card-note"><strong>{frontier_success}</strong> frontier pass <span class="separator">/</span> <strong class="frontier-bad">{frontier_failure}</strong> fail · {impossible} impossible</div>"#,
        core_total = summary.core_total(),
        core_success = summary.core_success,
        core_failure = summary.core_failure,
        frontier_success = summary.frontier_success,
        frontier_failure = summary.frontier_failure,
        impossible = summary.impossible,
    )
}

fn render_evidence_outcomes(summary: &EvidenceSummary) -> String {
    let core_failure_class = if summary.core_failure > 0 { "bad" } else { "" };
    let frontier_failure_class = if summary.frontier_failure > 0 {
        "frontier-bad"
    } else {
        ""
    };
    format!(
        r#"<div><strong>{core_success}</strong> pass <span class="separator">/</span> <strong class="{core_failure_class}">{core_failure}</strong> fail<div class="muted">{core_total} core runs</div></div><div><strong>{frontier_success}</strong> pass <span class="separator">/</span> <strong class="{frontier_failure_class}">{frontier_failure}</strong> fail<div class="muted">{frontier_total} frontier · {impossible} impossible</div></div>"#,
        core_success = summary.core_success,
        core_failure = summary.core_failure,
        core_total = summary.core_total(),
        frontier_success = summary.frontier_success,
        frontier_failure = summary.frontier_failure,
        frontier_total = summary.frontier_total(),
        impossible = summary.impossible,
    )
}

fn load_evidence(repo_root: &Path, report: &GuidanceReport) -> CatalogEvidence {
    let raw_dir = repo_root.join("outputs/eval").join(&report.pack_id);
    let batch = read_json::<BatchReport>(&raw_dir.join("summary.json"));
    let comparison = read_json::<BatchComparison>(&raw_dir.join("compare.json"));
    let summary = batch
        .as_ref()
        .map(preferred_lane_summary)
        .unwrap_or_default();
    CatalogEvidence {
        report: report.clone(),
        batch,
        comparison,
        summary,
    }
}

fn preferred_lane_summary(report: &BatchReport) -> EvidenceSummary {
    let lane_ids = report
        .records
        .iter()
        .map(|record| record.resolved.lane_id.as_str())
        .collect::<BTreeSet<_>>();
    let lane = if lane_ids.contains("current") {
        Some("current")
    } else if lane_ids.contains("staged") {
        Some("staged")
    } else if lane_ids.len() == 1 {
        lane_ids.first().copied()
    } else {
        None
    };
    let records = report
        .records
        .iter()
        .filter(|record| lane.is_none_or(|lane| record.resolved.lane_id == lane));
    summarize_records(records)
}

fn summarize_records<'a>(records: impl Iterator<Item = &'a BatchRunRecord>) -> EvidenceSummary {
    let mut summary = EvidenceSummary::default();
    for record in records {
        summary.total += 1;
        match record.analytic.class {
            BatchRunAnalyticClass::Impossible => summary.impossible += 1,
            BatchRunAnalyticClass::Frontier => {
                if record.manifest.mission_outcome == MissionOutcome::Success {
                    summary.frontier_success += 1;
                } else {
                    summary.frontier_failure += 1;
                }
            }
            BatchRunAnalyticClass::Scored => {
                if record.manifest.mission_outcome == MissionOutcome::Success {
                    summary.core_success += 1;
                } else {
                    summary.core_failure += 1;
                }
            }
        }
    }
    summary
}

fn render_eval_index(repo_root: &Path, catalog: &GuidanceCatalog) -> Result<String> {
    let primary_ids = catalog
        .groups
        .iter()
        .flat_map(|group| group.reports.iter())
        .filter(|report| report.role == GuidanceRole::Primary)
        .map(|report| report.pack_id.as_str())
        .collect::<BTreeSet<_>>();
    let supporting_ids = catalog
        .groups
        .iter()
        .flat_map(|group| group.reports.iter())
        .filter(|report| report.role == GuidanceRole::Supporting)
        .map(|report| report.pack_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut categories = BTreeMap::<EvalCategory, Vec<FixturePack>>::new();
    for fixture in load_fixture_packs(&repo_root.join("fixtures/packs"))? {
        categories
            .entry(classify_fixture(&fixture))
            .or_default()
            .push(fixture);
    }
    let mut sections = String::new();
    for category in [
        EvalCategory::Terminal,
        EvalCategory::DirectTransfer,
        EvalCategory::Waypoint,
        EvalCategory::Diagnostic,
        EvalCategory::Fixture,
    ] {
        let Some(mut packs) = categories.remove(&category) else {
            continue;
        };
        packs.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        let cards = packs
            .iter()
            .map(|pack| {
                let captured = repo_root
                    .join("outputs/reports/eval")
                    .join(&pack.id)
                    .join("index.html")
                    .exists();
                let role = if primary_ids.contains(pack.id.as_str()) {
                    "primary"
                } else if supporting_ids.contains(pack.id.as_str()) {
                    "supporting"
                } else {
                    "maintained"
                };
                let href = if captured {
                    format!(r#"href="{}/""#, escape_html(&pack.id))
                } else {
                    String::new()
                };
                format!(
                    r#"<a class="eval-card {captured}" {href} data-category="{category}" data-search="{search}"><span class="eyebrow">{role} · {capture}</span><strong>{name}</strong><span>{description}</span><code>{id}</code></a>"#,
                    captured = if captured { "captured" } else { "missing" },
                    href = href,
                    category = category.id(),
                    search = escape_html(&format!("{} {}", pack.id, pack.name).to_lowercase()),
                    role = role,
                    capture = if captured { "captured" } else { "not captured" },
                    name = escape_html(&pack.name),
                    description = escape_html(&pack.description),
                    id = escape_html(&pack.id),
                )
            })
            .collect::<String>();
        sections.push_str(&format!(
            r#"<section class="eval-group" data-group="{id}"><h2>{label}</h2><div class="eval-grid">{cards}</div></section>"#,
            id = category.id(),
            label = category.label(),
        ));
    }
    let controls = [
        ("all", "All"),
        ("terminal", "Terminal"),
        ("direct-transfer", "Direct transfer"),
        ("waypoint", "Waypoint"),
        ("diagnostic", "Diagnostic"),
        ("fixture", "Fixture"),
    ]
    .iter()
    .map(|(id, label)| format!(r#"<button data-filter="{id}">{label}</button>"#))
    .collect::<String>();
    let body = format!(
        r#"<div class="eval-controls"><input id="report-search" type="search" placeholder="Filter reports" aria-label="Filter reports"><div>{controls}</div></div>{sections}<script>{FILTER_JS}</script>"#
    );
    Ok(page(
        "Batch Reports",
        "Batch Reports",
        "Maintained evaluation packs grouped by guidance responsibility. Missing captures remain visible so corpus coverage is explicit.",
        r#"<a href="../">reports/</a><a href="../guidance/">guidance overview</a>"#,
        &body,
    ))
}

fn load_fixture_packs(fixtures_dir: &Path) -> Result<Vec<FixturePack>> {
    let mut packs = Vec::new();
    for entry in fs::read_dir(fixtures_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        if let Some(pack) = read_json(&path) {
            packs.push(pack);
        }
    }
    Ok(packs)
}

fn classify_fixture(pack: &FixturePack) -> EvalCategory {
    let tags = pack
        .entries
        .iter()
        .flat_map(|entry| entry.tags.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let diagnostic = tags.contains("diagnostic")
        || tags.contains("experimental")
        || pack.entries.iter().any(|entry| {
            entry
                .expectation_tier
                .as_deref()
                .is_some_and(|tier| tier == "diagnostic")
        });
    if diagnostic {
        EvalCategory::Diagnostic
    } else if tags.contains("waypoint_guidance") {
        EvalCategory::Waypoint
    } else if tags.contains("terminal_guidance") {
        EvalCategory::Terminal
    } else if tags.contains("transfer_guidance") {
        EvalCategory::DirectTransfer
    } else {
        EvalCategory::Fixture
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

fn page(title: &str, heading: &str, intro: &str, actions: &str, body: &str) -> String {
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{title}</title><style>{CATALOG_CSS}</style></head><body><main><header><div><span class="eyebrow">powered descent lab</span><h1>{heading}</h1><p>{intro}</p></div><nav>{actions}</nav></header>{body}</main></body></html>"#,
        title = escape_html(title),
        heading = escape_html(heading),
        intro = escape_html(intro),
    )
}

const FILTER_JS: &str = r#"
const cards=[...document.querySelectorAll('.eval-card')];const groups=[...document.querySelectorAll('.eval-group')];const search=document.querySelector('#report-search');let active='all';function apply(){const term=search.value.trim().toLowerCase();cards.forEach(card=>{const category=card.dataset.category;card.hidden=!((active==='all'||active===category)&&(!term||card.dataset.search.includes(term)));});groups.forEach(group=>group.hidden=![...group.querySelectorAll('.eval-card')].some(card=>!card.hidden));}document.querySelectorAll('[data-filter]').forEach(button=>button.addEventListener('click',()=>{active=button.dataset.filter;document.querySelectorAll('[data-filter]').forEach(item=>item.classList.toggle('active',item===button));apply();}));search.addEventListener('input',apply);document.querySelector('[data-filter="all"]').classList.add('active');
"#;

const CATALOG_CSS: &str = r#"
:root {
  color-scheme: light;
  --canvas: #f1ede5;
  --paper: #fffdf8;
  --paper-deep: #f8f2e8;
  --ink: #20211e;
  --muted: #6d665c;
  --line: #d9cdbc;
  --rust: #b95024;
  --green: #176b5c;
  --blue: #315f86;
  --red: #a43a2c;
  --amber: #966515;
  --display: "Iowan Old Style", "Palatino Linotype", Georgia, serif;
  --sans: "Avenir Next", "IBM Plex Sans", "Trebuchet MS", sans-serif;
  --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
  --shadow: 0 18px 44px rgba(54, 39, 25, 0.08);
}
* { box-sizing: border-box; }
body {
  margin: 0;
  color: var(--ink);
  font-family: var(--sans);
  background:
    radial-gradient(circle at 7% -8%, rgba(185, 80, 36, 0.15), transparent 31rem),
    linear-gradient(rgba(55, 43, 31, 0.018) 1px, transparent 1px),
    linear-gradient(90deg, rgba(55, 43, 31, 0.018) 1px, transparent 1px),
    linear-gradient(180deg, #fbf8f2, var(--canvas));
  background-size: auto, 32px 32px, 32px 32px, auto;
  min-height: 100vh;
}
main {
  width: min(1280px, 100%);
  margin: auto;
  padding: 28px 22px 64px;
}
header {
  position: relative;
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 24px;
  margin-bottom: 24px;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 22px;
  background:
    radial-gradient(circle at 88% 18%, rgba(49, 95, 134, 0.09), transparent 18rem),
    rgba(255, 253, 248, 0.91);
  box-shadow: var(--shadow);
  padding: 22px 24px 24px;
}
header::before {
  content: "";
  position: absolute;
  inset: 0 0 auto;
  height: 5px;
  background: linear-gradient(90deg, var(--rust) 0 30%, var(--green) 30% 65%, var(--blue) 65%);
}
header::after {
  content: "";
  position: absolute;
  width: 190px;
  height: 190px;
  right: 8%;
  top: -142px;
  border: 1px solid rgba(49, 95, 134, 0.16);
  border-radius: 50%;
}
header > * { position: relative; z-index: 1; }
h1, h2, h3 {
  font-family: var(--display);
  font-weight: 500;
}
h1 {
  margin: 0.2rem 0 0.55rem;
  font-size: clamp(2.15rem, 4.5vw, 3.45rem);
  line-height: 0.98;
  letter-spacing: -0.025em;
}
p {
  max-width: 76ch;
  margin: 0;
  color: var(--muted);
  line-height: 1.52;
}
.eyebrow {
  color: var(--rust);
  font-size: 0.68rem;
  font-weight: 800;
  letter-spacing: 0.13em;
  text-transform: uppercase;
}
nav, .links {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
nav a, .eval-controls button {
  border: 1px solid var(--line);
  border-radius: 999px;
  background: rgba(255, 253, 248, 0.82);
  color: inherit;
  padding: 7px 12px;
  text-decoration: none;
  transition: border-color 140ms ease, background 140ms ease, transform 140ms ease;
}
nav a:hover, .eval-controls button:hover {
  border-color: var(--rust);
  background: #fffaf3;
  transform: translateY(-1px);
}
a:focus-visible, button:focus-visible, input:focus-visible {
  outline: 3px solid rgba(49, 95, 134, 0.28);
  outline-offset: 2px;
}
.guidance-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 16px;
}
.guidance-card {
  --card-accent: var(--rust);
  position: relative;
  isolation: isolate;
  display: flex;
  flex-direction: column;
  min-height: 246px;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 19px;
  background: rgba(255, 253, 248, 0.94);
  box-shadow: var(--shadow);
  color: inherit;
  padding: 20px 20px 18px 23px;
  text-decoration: none;
  transition: transform 180ms ease, box-shadow 180ms ease, border-color 180ms ease;
}
.guidance-card:nth-child(2) { --card-accent: var(--green); }
.guidance-card:nth-child(3) { --card-accent: var(--blue); }
.guidance-card::before {
  content: "";
  position: absolute;
  inset: 0 auto 0 0;
  width: 5px;
  background: var(--card-accent);
}
.guidance-card::after {
  content: "01";
  position: absolute;
  right: 14px;
  top: 5px;
  z-index: -1;
  color: color-mix(in srgb, var(--card-accent) 9%, transparent);
  font-family: var(--display);
  font-size: 5.2rem;
  line-height: 1;
}
.guidance-card:nth-child(2)::after { content: "02"; }
.guidance-card:nth-child(3)::after { content: "03"; }
.guidance-card:hover {
  border-color: color-mix(in srgb, var(--card-accent) 52%, var(--line));
  box-shadow: 0 22px 48px rgba(54, 39, 25, 0.13);
  transform: translateY(-4px);
}
.guidance-card h2 {
  margin: 0.45rem 0 0.5rem;
  font-size: 1.52rem;
}
.guidance-card p { max-width: 34ch; }
.guidance-card dl {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  margin: auto 0 0;
  padding-top: 20px;
}
.guidance-card dl div {
  border-top: 1px solid var(--line);
  padding-top: 8px;
}
.guidance-card dt {
  color: var(--muted);
  font-size: 0.68rem;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}
.guidance-card dd {
  margin: 3px 0 0;
  font-size: 1.28rem;
  font-weight: 800;
}
.guidance-card-note {
  margin-top: 8px;
  color: var(--muted);
  font-size: 0.76rem;
  line-height: 1.35;
}
.guidance-card-note strong { color: var(--green); }
.guidance-card-note strong.frontier-bad { color: var(--amber); }
.score-note {
  max-width: none;
  margin: 0 2px 12px;
  font-size: 0.84rem;
}
.scorecard {
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 19px;
  background: rgba(255, 253, 248, 0.95);
  box-shadow: var(--shadow);
}
.score-head, .score-row {
  display: grid;
  grid-template-columns: minmax(250px, 1.45fr) minmax(190px, 1fr) minmax(160px, 0.8fr) minmax(180px, 1fr);
  gap: 18px;
  align-items: center;
  padding: 13px 17px;
}
.score-head {
  background: #e9e1d5;
  color: var(--muted);
  font-size: 0.68rem;
  font-weight: 800;
  letter-spacing: 0.09em;
  text-transform: uppercase;
}
.score-row {
  position: relative;
  border-top: 1px solid var(--line);
  color: inherit;
  text-decoration: none;
  transition: background 140ms ease, padding-left 140ms ease;
}
.score-row::before {
  content: "";
  position: absolute;
  inset: 8px auto 8px 0;
  width: 3px;
  border-radius: 999px;
  background: var(--green);
  opacity: 0;
  transition: opacity 140ms ease;
}
.score-row:hover {
  background: #fbf5eb;
  padding-left: 21px;
}
.score-row:hover::before { opacity: 1; }
.score-row h3 {
  margin: 0.23rem 0;
  font-size: 1.16rem;
}
.score-row code {
  display: block;
  margin-top: 5px;
  color: var(--muted);
  font-family: var(--mono);
  font-size: 0.71rem;
  overflow-wrap: anywhere;
}
.score-row.missing { opacity: 0.62; }
.muted {
  color: var(--muted);
  font-size: 0.82rem;
  margin-top: 4px;
}
.separator { color: var(--line); }
.bad { color: var(--red); }
.frontier-bad { color: var(--amber); }
.pair, .status {
  display: inline-block;
  border: 1px solid var(--line);
  border-radius: 999px;
  font-size: 0.67rem;
  padding: 3px 7px;
  margin: 2px 5px 2px 0;
}
.status.captured, .status.pass { border-color: #78a996; color: var(--green); }
.status.warn { border-color: #c8a45d; color: var(--amber); }
.status.fail, .status.missing { border-color: #cc8c82; color: var(--red); }
.status.quiet { color: var(--muted); }
.eval-controls {
  position: sticky;
  top: 10px;
  z-index: 5;
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
  margin-bottom: 26px;
  border: 1px solid rgba(217, 205, 188, 0.85);
  border-radius: 16px;
  background: rgba(255, 253, 248, 0.88);
  box-shadow: 0 10px 28px rgba(54, 39, 25, 0.07);
  padding: 9px 10px;
  backdrop-filter: blur(10px);
}
.eval-controls input {
  min-width: min(360px, 100%);
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--paper);
  padding: 9px 11px;
  font: inherit;
}
.eval-controls > div { display: flex; flex-wrap: wrap; gap: 6px; }
.eval-controls button { cursor: pointer; }
.eval-controls button.active {
  border-color: var(--rust);
  background: #f7dfd0;
  color: #7b3115;
}
.eval-group { margin-top: 30px; }
.eval-group h2 {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 0 0 12px;
  font-size: 1.5rem;
}
.eval-group h2::after {
  content: "";
  height: 1px;
  flex: 1;
  background: linear-gradient(90deg, var(--line), transparent);
}
.eval-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}
.eval-card {
  --card-accent: var(--green);
  display: flex;
  min-height: 168px;
  flex-direction: column;
  gap: 8px;
  border: 1px solid var(--line);
  border-top: 3px solid var(--card-accent);
  border-radius: 16px;
  background: rgba(255, 253, 248, 0.94);
  box-shadow: 0 10px 26px rgba(54, 39, 25, 0.055);
  color: inherit;
  padding: 14px 15px;
  text-decoration: none;
  transition: transform 150ms ease, box-shadow 150ms ease;
}
.eval-card[data-category="terminal"] { --card-accent: var(--rust); }
.eval-card[data-category="direct-transfer"] { --card-accent: var(--green); }
.eval-card[data-category="waypoint"] { --card-accent: var(--blue); }
.eval-card[data-category="diagnostic"] { --card-accent: var(--amber); }
.eval-card strong {
  font-family: var(--display);
  font-size: 1.14rem;
  font-weight: 500;
  line-height: 1.2;
}
.eval-card > span:nth-of-type(2) {
  color: var(--muted);
  font-size: 0.84rem;
  line-height: 1.42;
}
.eval-card code {
  margin-top: auto;
  color: var(--muted);
  font-family: var(--mono);
  font-size: 0.69rem;
  overflow-wrap: anywhere;
}
.eval-card.missing { opacity: 0.54; }
.eval-card.captured:hover {
  box-shadow: 0 16px 34px rgba(54, 39, 25, 0.11);
  transform: translateY(-3px);
}
[hidden] { display: none !important; }
@media (max-width: 900px) {
  .guidance-grid, .eval-grid { grid-template-columns: 1fr 1fr; }
  .score-head { display: none; }
  .score-row { grid-template-columns: 1fr 1fr; }
  .eval-controls { align-items: stretch; flex-direction: column; }
}
@media (max-width: 620px) {
  main { padding: 14px 12px 40px; }
  header { display: grid; padding: 18px 17px 20px; }
  header::after { right: -70px; }
  h1 { font-size: clamp(2rem, 12vw, 2.75rem); }
  .guidance-grid, .eval-grid, .score-row { grid-template-columns: 1fr; }
  .guidance-card { min-height: 225px; }
  .guidance-card dl { grid-template-columns: repeat(3, 1fr); }
  nav { order: -1; }
  .eval-controls { position: static; }
  .eval-controls input { min-width: 0; width: 100%; }
}
"#;

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::{
        EvalCategory, EvidenceSummary, classify_fixture, load_guidance_catalog,
        preferred_lane_summary, refresh_pack_ids, render_evidence_outcomes,
        render_guidance_metrics,
    };
    use crate::load_batch_report;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn catalog_declares_each_guidance_group() {
        let catalog = load_guidance_catalog(&repo_root()).unwrap();
        assert_eq!(catalog.schema_version, 1);
        assert_eq!(
            catalog
                .groups
                .iter()
                .map(|group| group.id.as_str())
                .collect::<Vec<_>>(),
            ["terminal", "transfer", "waypoint"]
        );
    }

    #[test]
    fn terminal_scorecard_uses_current_lane_only_when_fixture_is_available() {
        let report_path = repo_root().join("outputs/eval/terminal_bot_lab_suite");
        if !report_path.exists() {
            return;
        }
        let report = load_batch_report(&report_path).unwrap();
        let summary = preferred_lane_summary(&report);
        assert_eq!(
            summary,
            EvidenceSummary {
                total: 189,
                core_success: 168,
                core_failure: 0,
                frontier_success: 3,
                frontier_failure: 9,
                impossible: 9,
            }
        );
    }

    #[test]
    fn terminal_catalog_separates_core_failures_from_frontier_failures() {
        let clean_smoke = EvidenceSummary {
            total: 189,
            core_success: 168,
            core_failure: 0,
            frontier_success: 3,
            frontier_failure: 9,
            impossible: 9,
        };
        let outcomes_html = render_evidence_outcomes(&clean_smoke);
        assert!(outcomes_html.contains("<strong>168</strong> pass"));
        assert!(outcomes_html.contains("<strong class=\"\">0</strong> fail"));
        assert!(outcomes_html.contains("<strong class=\"frontier-bad\">9</strong> fail"));
        assert!(outcomes_html.contains("12 frontier · 9 impossible"));

        let mut aggregate = clean_smoke;
        aggregate.include(&EvidenceSummary {
            total: 756,
            core_success: 672,
            core_failure: 0,
            frontier_success: 22,
            frontier_failure: 26,
            impossible: 36,
        });
        let overview_html = render_guidance_metrics(&aggregate);
        assert!(overview_html.contains("<dt>Core</dt><dd>840</dd>"));
        assert!(overview_html.contains("<dt>Fail</dt><dd class=\"\">0</dd>"));
        assert!(overview_html.contains("<strong>25</strong> frontier pass"));
        assert!(overview_html.contains("<strong class=\"frontier-bad\">35</strong> fail"));
    }

    #[test]
    fn waypoint_tags_take_precedence_over_transfer_tags() {
        let fixture = super::FixturePack {
            id: "waypoint".to_owned(),
            name: "Waypoint".to_owned(),
            description: String::new(),
            entries: vec![super::FixtureEntry {
                tags: vec![
                    "transfer_guidance".to_owned(),
                    "waypoint_guidance".to_owned(),
                ],
                expectation_tier: None,
            }],
        };
        assert_eq!(classify_fixture(&fixture), EvalCategory::Waypoint);
    }

    #[test]
    fn refresh_scope_defaults_to_catalog_and_all_uses_fixtures() {
        let catalog = refresh_pack_ids(&repo_root(), false).unwrap();
        assert!(catalog.contains(&"terminal_bot_lab_suite".to_owned()));
        assert!(catalog.contains(&"transfer_route_angle_radius_suite".to_owned()));
        assert!(
            catalog.contains(&"transfer_waypoint_sequence_route_angle_radius_smoke".to_owned())
        );
        assert!(!catalog.contains(&"core_suite".to_owned()));

        let all = refresh_pack_ids(&repo_root(), true).unwrap();
        assert!(all.contains(&"core_suite".to_owned()));
        assert!(all.len() > catalog.len());
    }
}
