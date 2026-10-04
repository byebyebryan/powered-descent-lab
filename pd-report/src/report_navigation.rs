use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

use crate::escape_html;

const NAVIGATION_FIXTURE: &str = "fixtures/reports/report_navigation.json";

/// Native evaluator packs selected by topic navigation, distinct from legacy
/// controller scorecards. Used by report-only refresh to include the active
/// planner without pretending that its records are controller batch records.
pub fn configured_batch_pack_ids(repo_root: &Path) -> Result<Vec<String>> {
    let path = repo_root.join(NAVIGATION_FIXTURE);
    if !path.try_exists()? {
        return Ok(Vec::new());
    }
    let manifest: NavigationFixture = serde_json::from_slice(&fs::read(path)?)?;
    validate_manifest(&manifest)?;
    Ok(manifest
        .topics
        .iter()
        .flat_map(|topic| &topic.entries)
        .filter_map(|entry| {
            if let TopicSource::BatchPack { pack_id } = &entry.source {
                Some(pack_id.clone())
            } else {
                None
            }
        })
        .collect())
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PreviewTargets {
    pub home: Option<String>,
    pub collection: Option<String>,
}

pub(crate) struct PreparedReportNavigation {
    pages: Vec<(PathBuf, String)>,
}

impl PreparedReportNavigation {
    pub fn pages(&self) -> &[(PathBuf, String)] {
        &self.pages
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NavigationFixture {
    schema_version: u32,
    preview: PreviewCopy,
    topics: Vec<Topic>,
    history: Vec<ManualEntry>,
    raw_collections: Vec<RawCollection>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviewCopy {
    title: String,
    description: String,
    report_type: String,
    status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Topic {
    id: String,
    title: String,
    description: String,
    entries: Vec<TopicEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TopicEntry {
    id: String,
    title: String,
    description: String,
    report_type: String,
    status: String,
    source: TopicSource,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum TopicSource {
    SelectedPreview,
    GuidanceGroup { group_id: String },
    BatchPack { pack_id: String },
    ReportPage { path: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManualEntry {
    id: String,
    title: String,
    description: String,
    path: String,
    topic_id: String,
    report_type: String,
    status: String,
    capture_label: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCollection {
    id: String,
    title: String,
    description: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct GuidanceCatalog {
    schema_version: u32,
    groups: Vec<GuidanceGroup>,
}

#[derive(Debug, Deserialize)]
struct GuidanceGroup {
    id: String,
    description: String,
    reports: Vec<GuidanceReport>,
}

#[derive(Clone, Debug, Deserialize)]
struct GuidanceReport {
    pack_id: String,
    label: String,
    role: String,
    evidence: String,
}

#[derive(Debug, Deserialize)]
struct FixturePack {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Clone)]
struct Row {
    id: String,
    title: String,
    description: String,
    topic_id: String,
    topic: String,
    type_id: String,
    report_type: String,
    status: String,
    availability: String,
    href: Option<String>,
}

/// Parse and render every navigation-owned page before any index is written.
/// An absent manifest preserves the older generic site behavior.
pub(crate) fn prepare(
    repo_root: &Path,
    reports_root: &Path,
    preview: &PreviewTargets,
) -> Result<Option<PreparedReportNavigation>> {
    let manifest_path = repo_root.join(NAVIGATION_FIXTURE);
    if !manifest_path.try_exists()? {
        return Ok(None);
    }
    let manifest = serde_json::from_slice::<NavigationFixture>(&fs::read(&manifest_path)?)
        .with_context(|| {
            format!(
                "parse report navigation fixture {}",
                manifest_path.display()
            )
        })?;
    validate_manifest(&manifest)?;

    let guidance = load_guidance(&repo_root.join("fixtures/reports/guidance_catalog.json"))?;
    let packs = load_packs(&repo_root.join("fixtures/packs"))?;
    for entry in manifest.topics.iter().flat_map(|topic| &topic.entries) {
        if let TopicSource::BatchPack { pack_id } = &entry.source {
            ensure!(packs.contains_key(pack_id), "unknown batch pack {pack_id}");
        }
    }
    let preview = resolve_preview(reports_root, preview)?;
    let topic_labels = manifest
        .topics
        .iter()
        .map(|topic| (topic.id.as_str(), topic.title.as_str()))
        .collect::<BTreeMap<_, _>>();

    let home = render_home(&manifest, &preview, reports_root)?;
    let mut pages = vec![
        (reports_root.join("index.html"), home.clone()),
        (repo_root.join("outputs/index.html"), home),
    ];
    for topic in &manifest.topics {
        pages.push((
            reports_root
                .join("topics")
                .join(&topic.id)
                .join("index.html"),
            render_topic(topic, &guidance, reports_root, &preview)?,
        ));
    }

    let rows = build_rows(
        &manifest,
        &guidance,
        &packs,
        reports_root,
        &preview,
        &topic_labels,
    )?;
    pages.push((
        reports_root.join("history/index.html"),
        render_history(&manifest, reports_root)?,
    ));
    let library = render_library(&rows, &topic_labels);
    pages.push((reports_root.join("library/index.html"), library.clone()));
    pages.push((reports_root.join("eval/index.html"), library));
    pages.push((
        reports_root.join("setups/index.html"),
        render_setup_index(&rows),
    ));
    pages.push((
        reports_root.join("data/index.html"),
        render_data(&manifest, &repo_root.join("outputs"))?,
    ));
    Ok(Some(PreparedReportNavigation { pages }))
}

fn validate_manifest(manifest: &NavigationFixture) -> Result<()> {
    ensure!(
        manifest.schema_version == 1,
        "unsupported report navigation schema"
    );
    let mut topic_ids = BTreeSet::new();
    let mut entry_ids = BTreeSet::new();
    let mut group_ids = BTreeSet::new();
    let mut preview_count = 0;
    for topic in &manifest.topics {
        let mut batch_count = 0;
        validate_id(&topic.id)?;
        validate_text(&topic.title)?;
        validate_text(&topic.description)?;
        ensure!(
            topic_ids.insert(topic.id.as_str()),
            "duplicate report topic id {}",
            topic.id
        );
        for entry in &topic.entries {
            validate_id(&entry.id)?;
            validate_text(&entry.title)?;
            validate_text(&entry.description)?;
            validate_text(&entry.report_type)?;
            validate_text(&entry.status)?;
            ensure!(
                entry_ids.insert(entry.id.as_str()),
                "duplicate navigation entry id {}",
                entry.id
            );
            match &entry.source {
                TopicSource::SelectedPreview => preview_count += 1,
                TopicSource::GuidanceGroup { group_id } => {
                    validate_id(group_id)?;
                    ensure!(
                        group_ids.insert(group_id.as_str()),
                        "guidance group mapped more than once: {group_id}"
                    );
                }
                TopicSource::BatchPack { pack_id } => {
                    validate_id(pack_id)?;
                    batch_count += 1;
                }
                TopicSource::ReportPage { path } => validate_report_path(path)?,
            }
        }
        ensure!(
            batch_count <= 1,
            "a subject has more than one current batch pack"
        );
    }
    ensure!(
        topic_ids == BTreeSet::from(["waypoint-planning", "flight-control"]),
        "report navigation must define exactly Waypoint planning and Flight and landing control"
    );
    ensure!(preview_count == 1, "expected one selected V2 preview entry");
    ensure!(
        group_ids == BTreeSet::from(["planner", "terminal", "transfer", "waypoint"]),
        "report navigation must map all four maintained guidance responsibilities"
    );

    for topic in &manifest.topics {
        for entry in &topic.entries {
            match &entry.source {
                TopicSource::SelectedPreview => ensure!(
                    topic.id == "waypoint-planning" && entry.title == "Waypoint planner V2",
                    "the selected V2 preview must be a Waypoint planning entry named Waypoint planner V2"
                ),
                TopicSource::GuidanceGroup { group_id } => ensure!(
                    (topic.id == "waypoint-planning" && group_id == "planner")
                        || (topic.id == "flight-control"
                            && matches!(group_id.as_str(), "terminal" | "transfer" | "waypoint")),
                    "guidance group {group_id} is mapped to the wrong report subject"
                ),
                TopicSource::ReportPage { .. } => ensure!(
                    topic.id == "waypoint-planning",
                    "explicit analytical report pages belong under Waypoint planning"
                ),
                TopicSource::BatchPack { .. } => ensure!(
                    topic.id == "waypoint-planning",
                    "the current planner batch belongs under Waypoint planning"
                ),
            }
        }
    }

    let mut history_paths = BTreeSet::new();
    let mut all_paths = BTreeSet::new();
    for topic in &manifest.topics {
        for entry in &topic.entries {
            if let TopicSource::ReportPage { path } = &entry.source {
                ensure!(
                    all_paths.insert(path.as_str()),
                    "duplicate navigation report path {path}"
                );
            }
        }
    }
    for entry in &manifest.history {
        validate_id(&entry.id)?;
        validate_text(&entry.title)?;
        validate_text(&entry.description)?;
        validate_text(&entry.topic_id)?;
        validate_text(&entry.report_type)?;
        validate_text(&entry.status)?;
        validate_text(&entry.capture_label)?;
        validate_report_path(&entry.path)?;
        ensure!(
            entry_ids.insert(entry.id.as_str()),
            "duplicate navigation entry id {}",
            entry.id
        );
        ensure!(
            history_paths.insert(entry.path.as_str()),
            "duplicate history path {}",
            entry.path
        );
        ensure!(
            all_paths.insert(entry.path.as_str()),
            "duplicate navigation report path {}",
            entry.path
        );
        ensure!(
            topic_ids.contains(entry.topic_id.as_str()) || entry.topic_id == "unclassified",
            "history entry {} refers to unknown topic {}",
            entry.id,
            entry.topic_id
        );
    }
    let mut raw_ids = BTreeSet::new();
    let mut raw_paths = BTreeSet::new();
    for raw in &manifest.raw_collections {
        validate_id(&raw.id)?;
        validate_text(&raw.title)?;
        validate_text(&raw.description)?;
        validate_raw_path(&raw.path)?;
        ensure!(
            raw_ids.insert(raw.id.as_str()),
            "duplicate raw collection id {}",
            raw.id
        );
        ensure!(
            raw_paths.insert(raw.path.as_str()),
            "duplicate raw collection path {}",
            raw.path
        );
    }
    for value in [
        &manifest.preview.title,
        &manifest.preview.description,
        &manifest.preview.report_type,
        &manifest.preview.status,
    ] {
        validate_text(value)?;
    }
    Ok(())
}

fn load_guidance(path: &Path) -> Result<GuidanceCatalog> {
    let guidance = serde_json::from_slice::<GuidanceCatalog>(&fs::read(path)?)
        .with_context(|| format!("parse guidance catalog {}", path.display()))?;
    ensure!(
        guidance.schema_version == 1,
        "unsupported guidance catalog schema"
    );
    let mut ids = BTreeSet::new();
    let mut pack_ids = BTreeSet::new();
    for group in &guidance.groups {
        validate_id(&group.id)?;
        validate_text(&group.description)?;
        ensure!(
            ids.insert(group.id.as_str()),
            "duplicate guidance group id {}",
            group.id
        );
        for report in &group.reports {
            validate_id(&report.pack_id)?;
            validate_text(&report.label)?;
            validate_text(&report.role)?;
            validate_text(&report.evidence)?;
            ensure!(
                pack_ids.insert(report.pack_id.as_str()),
                "guidance pack appears more than once: {}",
                report.pack_id
            );
        }
    }
    Ok(guidance)
}

fn load_packs(directory: &Path) -> Result<BTreeMap<String, FixturePack>> {
    let mut packs = BTreeMap::new();
    for entry in fs::read_dir(directory)
        .with_context(|| format!("read fixture packs {}", directory.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let pack = serde_json::from_slice::<FixturePack>(&fs::read(&path)?)
            .with_context(|| format!("parse fixture pack {}", path.display()))?;
        validate_id(&pack.id)?;
        validate_text(&pack.name)?;
        if !pack.description.is_empty() {
            validate_text(&pack.description)?;
        }
        ensure!(
            path.file_stem().and_then(|stem| stem.to_str()) == Some(pack.id.as_str()),
            "fixture pack id does not match filename {}",
            path.display()
        );
        ensure!(
            packs.insert(pack.id.clone(), pack).is_none(),
            "duplicate fixture pack id"
        );
    }
    Ok(packs)
}

fn resolve_preview(root: &Path, preview: &PreviewTargets) -> Result<PreviewTargets> {
    let (Some(home), Some(collection)) = (&preview.home, &preview.collection) else {
        return Ok(PreviewTargets::default());
    };
    validate_report_path(home)?;
    validate_report_path(collection)?;
    ensure!(
        home != collection,
        "selected preview home and collection must differ"
    );
    if report_file_exists(root, home)? && report_file_exists(root, collection)? {
        Ok(preview.clone())
    } else {
        Ok(PreviewTargets::default())
    }
}

fn render_home(
    manifest: &NavigationFixture,
    preview: &PreviewTargets,
    reports_root: &Path,
) -> Result<String> {
    let subjects = manifest
        .topics
        .iter()
        .map(|topic| -> Result<String> {
            let batch_pack = topic.entries.iter().find_map(|entry| {
                if let TopicSource::BatchPack { pack_id } = &entry.source { Some(pack_id) } else { None }
            });
            let shortcut = if let Some(pack_id) = batch_pack {
                match active_href(reports_root, Some(&format!("eval/{pack_id}/index.html")))? {
                    Some(href) => format!(r#"<a class="shortcut" href="{href}">Open current planner V2 batch</a>"#),
                    None => "<p class=\"shortcut\">Current planner V2 batch not captured yet</p>".into(),
                }
            } else if topic.id == "waypoint-planning" {
                preview.collection.as_deref().map(|path| format!(
                    r#"<a class="shortcut" href="{}">Open current V2 preview</a>"#,
                    report_href(path),
                )).unwrap_or_default()
            } else {
                String::new()
            };
            Ok(format!(
                r#"<article class="subject"><a href="/reports/topics/{}/index.html"><span class="eyebrow">Choose by subject</span><h2>{}</h2><p>{}</p><span class="action">Browse this subject</span></a>{shortcut}</article>"#,
                escape_html(&topic.id), escape_html(&topic.title), escape_html(&topic.description),
            ))
        })
        .collect::<Result<Vec<_>>>()?.join("");
    Ok(page(
        "Reports",
        "Reports",
        "Start with the flight question you have. Report type and status describe the evidence inside each subject.",
        &format!(
            r#"<section><h2>Browse by subject</h2><div class="subject-grid">{subjects}</div></section><section class="secondary"><h2>Other ways to browse</h2><div class="secondary-links"><a href="/reports/history/index.html">Research and history</a><a href="/reports/library/index.html">Browse all reports</a><a href="/reports/data/index.html">Raw data</a></div></section>"#
        ),
    ))
}

fn render_topic(
    topic: &Topic,
    guidance: &GuidanceCatalog,
    reports_root: &Path,
    preview: &PreviewTargets,
) -> Result<String> {
    let mut rows = String::new();
    for entry in &topic.entries {
        let href = match &entry.source {
            TopicSource::SelectedPreview => {
                active_href(reports_root, preview.collection.as_deref())?
            }
            TopicSource::GuidanceGroup { group_id } => {
                ensure!(
                    guidance.groups.iter().any(|group| group.id == *group_id),
                    "unknown guidance group {group_id}"
                );
                let path = format!("guidance/{group_id}/index.html");
                active_href(reports_root, Some(&path))?
                    .map(|_| format!("/reports/guidance/{group_id}/"))
            }
            TopicSource::ReportPage { path } => active_href(reports_root, Some(path))?,
            TopicSource::BatchPack { pack_id } => {
                active_href(reports_root, Some(&format!("eval/{pack_id}/index.html")))?
            }
        };
        rows.push_str(&entry_row(
            &entry.title,
            &entry.description,
            &entry.report_type,
            &entry.status,
            "",
            href.as_deref(),
        ));
    }
    Ok(page(
        &topic.title,
        &topic.title,
        &topic.description,
        &format!(
            r#"<nav class="breadcrumbs" aria-label="Breadcrumb"><a href="/reports/index.html">Reports</a><span>›</span><span aria-current="page">{}</span></nav><section class="entry-list">{rows}</section><p class="section-footer"><a href="/reports/library/index.html">Browse all reports</a> · <a href="/reports/data/index.html">Raw data</a></p>"#,
            escape_html(&topic.title),
        ),
    ))
}

fn render_history(manifest: &NavigationFixture, reports_root: &Path) -> Result<String> {
    let mut rows = String::new();
    for entry in &manifest.history {
        let href = active_href(reports_root, Some(&entry.path))?;
        rows.push_str(&entry_row(
            &entry.title,
            &entry.description,
            &entry.report_type,
            &entry.status,
            &entry.capture_label,
            href.as_deref(),
        ));
    }
    Ok(page(
        "Research and history",
        "Research and history",
        "Explicitly selected studies and older view editions. A presentation edition records a different view of a capture, not another flight capture.",
        &format!(
            r#"<nav class="breadcrumbs" aria-label="Breadcrumb"><a href="/reports/index.html">Reports</a><span>›</span><span aria-current="page">Research and history</span></nav><section class="entry-list">{rows}</section><p class="section-footer"><a href="/reports/library/index.html">Browse all reports</a></p>"#
        ),
    ))
}

fn build_rows(
    manifest: &NavigationFixture,
    guidance: &GuidanceCatalog,
    packs: &BTreeMap<String, FixturePack>,
    reports_root: &Path,
    preview: &PreviewTargets,
    topic_labels: &BTreeMap<&str, &str>,
) -> Result<Vec<Row>> {
    let mut group_topics = BTreeMap::<&str, &Topic>::new();
    let mut group_by_pack = BTreeMap::<&str, (&GuidanceGroup, &GuidanceReport)>::new();
    for topic in &manifest.topics {
        for item in &topic.entries {
            if let TopicSource::GuidanceGroup { group_id } = &item.source {
                let group = guidance
                    .groups
                    .iter()
                    .find(|group| group.id == *group_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown guidance group {group_id}"))?;
                group_topics.insert(group_id, topic);
                for report in &group.reports {
                    ensure!(
                        group_by_pack
                            .insert(&report.pack_id, (group, report))
                            .is_none(),
                        "duplicate guidance pack mapping {}",
                        report.pack_id
                    );
                }
            }
        }
    }

    let mut rows = Vec::new();
    let mut paths = BTreeSet::new();
    for (id, pack) in packs {
        let relative = format!("eval/{id}/index.html");
        let href = active_href(reports_root, Some(&relative))?;
        let guidance_row = group_by_pack.get(id.as_str());
        let current_batch = manifest.topics.iter().find_map(|topic| {
            topic.entries.iter().find(|entry| matches!(&entry.source, TopicSource::BatchPack { pack_id } if pack_id == id))
                .map(|entry| (topic, entry))
        });
        let topic = current_batch.map(|(topic, _)| topic).or_else(|| {
            guidance_row.and_then(|(group, _)| group_topics.get(group.id.as_str()).copied())
        });
        let title = current_batch
            .map(|(_, entry)| entry.title.clone())
            .or_else(|| guidance_row.map(|(_, report)| report.label.clone()))
            .unwrap_or_else(|| pack.name.clone());
        let description = if !pack.description.is_empty() {
            pack.description.clone()
        } else {
            guidance_row
                .map(|(group, _)| group.description.clone())
                .unwrap_or_else(|| "No description is recorded in the fixture pack.".to_owned())
        };
        let topic_id = topic
            .map(|topic| topic.id.as_str())
            .unwrap_or("unclassified");
        let topic_label = topic
            .map(|topic| topic.title.as_str())
            .unwrap_or("Unclassified");
        let status = current_batch
            .map(|(_, entry)| entry.status.clone())
            .or_else(|| {
                guidance_row.map(|(_, report)| {
                    format!("Guidance catalog · {} · {}", report.role, report.evidence)
                })
            })
            .unwrap_or_else(|| "Unclassified".to_owned());
        rows.push(Row {
            id: id.clone(),
            title,
            description,
            topic_id: topic_id.to_owned(),
            topic: topic_label.to_owned(),
            type_id: "batch-report".to_owned(),
            report_type: "Batch report".to_owned(),
            status,
            availability: if href.is_some() {
                "Captured"
            } else {
                "Not captured"
            }
            .to_owned(),
            href,
        });
        paths.insert(relative);
    }

    for relative in stable_entries(reports_root, "eval")? {
        if paths.contains(&relative) {
            continue;
        }
        let id = path_id(&relative);
        rows.push(unclassified_row(
            &id,
            &slug_title(&id),
            "No topic metadata is present in the guidance catalog or fixture pack.",
            "batch-report",
            "Batch report",
            active_href(reports_root, Some(&relative))?,
        ));
        paths.insert(relative);
    }

    for topic in &manifest.topics {
        for entry in &topic.entries {
            if let TopicSource::ReportPage { path } = &entry.source {
                if paths.contains(path) {
                    continue;
                }
                let href = active_href(reports_root, Some(path))?;
                rows.push(Row {
                    id: path_id(path),
                    title: entry.title.clone(),
                    description: entry.description.clone(),
                    topic_id: topic.id.clone(),
                    topic: topic.title.clone(),
                    type_id: type_id(&entry.report_type),
                    report_type: entry.report_type.clone(),
                    status: entry.status.clone(),
                    availability: if href.is_some() {
                        "Available"
                    } else {
                        "Not available"
                    }
                    .to_owned(),
                    href,
                });
                paths.insert(path.clone());
            }
        }
    }

    for relative in stable_entries(reports_root, "setups")? {
        if paths.contains(&relative) {
            continue;
        }
        let id = path_id(&relative);
        rows.push(unclassified_row(
            &id,
            &slug_title(&id),
            "No topic metadata is present in report navigation.",
            "analytical-setup",
            "Analytical setup",
            active_href(reports_root, Some(&relative))?,
        ));
        paths.insert(relative);
    }

    for entry in &manifest.history {
        if paths.contains(&entry.path) {
            continue;
        }
        let href = active_href(reports_root, Some(&entry.path))?;
        let topic = topic_labels
            .get(entry.topic_id.as_str())
            .copied()
            .unwrap_or("Unclassified");
        rows.push(Row {
            id: entry.id.clone(),
            title: entry.title.clone(),
            description: entry.description.clone(),
            topic_id: entry.topic_id.clone(),
            topic: topic.to_owned(),
            type_id: type_id(&entry.report_type),
            report_type: entry.report_type.clone(),
            status: entry.status.clone(),
            availability: if href.is_some() {
                "Available"
            } else {
                "Not available"
            }
            .to_owned(),
            href,
        });
        paths.insert(entry.path.clone());
    }

    let collection = preview.collection.as_deref();
    let href = active_href(reports_root, collection)?;
    let topic = topic_labels
        .get("waypoint-planning")
        .copied()
        .unwrap_or("Unclassified");
    rows.push(Row {
        id: "selected-waypoint-v2-capture".to_owned(),
        title: manifest.preview.title.clone(),
        description: manifest.preview.description.clone(),
        topic_id: "waypoint-planning".to_owned(),
        topic: topic.to_owned(),
        type_id: type_id(&manifest.preview.report_type),
        report_type: manifest.preview.report_type.clone(),
        status: manifest.preview.status.clone(),
        availability: if href.is_some() {
            "Available"
        } else {
            "Unavailable"
        }
        .to_owned(),
        href,
    });

    rows.sort_by(|left, right| {
        left.title
            .to_lowercase()
            .cmp(&right.title.to_lowercase())
            .then(left.id.cmp(&right.id))
    });
    Ok(rows)
}

fn unclassified_row(
    id: &str,
    title: &str,
    description: &str,
    type_id: &str,
    report_type: &str,
    href: Option<String>,
) -> Row {
    Row {
        id: id.to_owned(),
        title: title.to_owned(),
        description: description.to_owned(),
        topic_id: "unclassified".to_owned(),
        topic: "Unclassified".to_owned(),
        type_id: type_id.to_owned(),
        report_type: report_type.to_owned(),
        status: "Unclassified".to_owned(),
        availability: if href.is_some() {
            "Captured"
        } else {
            "Not captured"
        }
        .to_owned(),
        href,
    }
}

fn render_library(rows: &[Row], topic_labels: &BTreeMap<&str, &str>) -> String {
    let topics = topic_labels
        .iter()
        .map(|(id, label)| {
            format!(
                r#"<option value="{}">{}</option>"#,
                escape_html(id),
                escape_html(label)
            )
        })
        .chain(std::iter::once(
            r#"<option value="unclassified">Unclassified</option>"#.to_owned(),
        ))
        .collect::<String>();
    let mut types = BTreeMap::new();
    for row in rows {
        types.insert(row.type_id.as_str(), row.report_type.as_str());
    }
    let types = types
        .iter()
        .map(|(id, label)| {
            format!(
                r#"<option value="{}">{}</option>"#,
                escape_html(id),
                escape_html(label)
            )
        })
        .collect::<String>();
    let rendered_rows = if rows.is_empty() {
        r#"<p class="empty">No stable report entrypoints are available.</p>"#.to_owned()
    } else {
        rows.iter().map(render_library_row).collect::<String>()
    };
    page(
        "Browse all reports",
        "Browse all reports",
        "Search stable report entrypoints by subject and report type.",
        &format!(
            r#"<nav class="breadcrumbs" aria-label="Breadcrumb"><a href="/reports/index.html">Reports</a><span>›</span><span aria-current="page">Browse all reports</span></nav><p class="library-note">Fixture-defined reports remain visible when a capture is missing. Missing captures have no active link. Reports without topic metadata are marked Unclassified.</p><div id="report-library"><div class="filters"><label>Search reports<input id="report-search" type="search" placeholder="Name or report ID"></label><label>Topic<select id="topic-filter"><option value="all">All topics</option>{topics}</select></label><label>Report type<select id="type-filter"><option value="all">All types</option>{types}</select></label><span id="report-count" aria-live="polite"></span></div><div class="report-rows">{rendered_rows}</div></div><script>{LIBRARY_JS}</script>"#
        ),
    )
}

fn render_library_row(row: &Row) -> String {
    let title = row
        .href
        .as_ref()
        .map(|href| {
            format!(
                r#"<a class="report-title" href="{}">{}</a>"#,
                escape_html(href),
                escape_html(&row.title)
            )
        })
        .unwrap_or_else(|| {
            format!(
                r#"<span class="report-title unavailable">{}</span>"#,
                escape_html(&row.title)
            )
        });
    let search = format!(
        "{} {} {} {} {} {}",
        row.title, row.description, row.id, row.topic, row.report_type, row.status
    )
    .to_lowercase();
    format!(
        r#"<article class="report-row" data-topic="{}" data-type="{}" data-status="{}" data-search="{}"><div class="report-main">{title}<p>{description}</p><code>{id}</code></div><div class="report-meta"><span>{topic}</span><span>{report_type}</span><span>{status}</span><span>{availability}</span></div></article>"#,
        escape_html(&row.topic_id),
        escape_html(&row.type_id),
        escape_html(&slug(&row.status)),
        escape_html(&search),
        title = title,
        description = escape_html(&row.description),
        id = escape_html(&row.id),
        topic = escape_html(&row.topic),
        report_type = escape_html(&row.report_type),
        status = escape_html(&row.status),
        availability = escape_html(&row.availability)
    )
}

fn render_setup_index(rows: &[Row]) -> String {
    let entries = rows
        .iter()
        .filter(|row| row.type_id == "analytical-setup")
        .map(|row| {
            entry_row(
                &row.title,
                &row.description,
                &row.report_type,
                &row.status,
                &row.availability,
                row.href.as_deref(),
            )
        })
        .collect::<String>();
    page(
        "Related analytical studies",
        "Related analytical studies",
        "Setup-only evidence linked from Waypoint planning. These reports do not contain simulated flight outcomes.",
        &format!(
            r#"<nav class="breadcrumbs" aria-label="Breadcrumb"><a href="/reports/index.html">Reports</a><span>›</span><a href="/reports/topics/waypoint-planning/index.html">Waypoint planning</a><span>›</span><span aria-current="page">Related studies</span></nav><section class="entry-list">{entries}</section><p class="section-footer"><a href="/reports/library/index.html">Browse all reports</a></p>"#
        ),
    )
}

fn render_data(manifest: &NavigationFixture, outputs_root: &Path) -> Result<String> {
    let mut rows = String::new();
    for raw in &manifest.raw_collections {
        if !raw_collection_exists(outputs_root, &raw.path)? {
            continue;
        }
        rows.push_str(&format!(r#"<article class="entry-row"><div><a class="entry-title" href="/{}/">{}</a><p>{}</p><code>{}/</code></div><div class="entry-meta"><span>Raw data</span><span>Directory</span></div></article>"#,
            escape_html(&raw.path), escape_html(&raw.title), escape_html(&raw.description), escape_html(&raw.path)));
    }
    if rows.is_empty() {
        rows = r#"<p class="empty">No approved raw data collections are available.</p>"#.to_owned();
    }
    Ok(page(
        "Raw data",
        "Raw data",
        "Direct links to selected raw artifact directories. These are not curated HTML report pages.",
        &format!(
            r#"<nav class="breadcrumbs" aria-label="Breadcrumb"><a href="/reports/index.html">Reports</a><span>›</span><span aria-current="page">Raw data</span></nav><section class="entry-list">{rows}</section><p class="section-footer"><a href="/reports/library/index.html">Browse all reports</a></p>"#
        ),
    ))
}

fn entry_row(
    title: &str,
    description: &str,
    report_type: &str,
    status: &str,
    availability: &str,
    href: Option<&str>,
) -> String {
    let availability = if href.is_none() {
        if availability.is_empty() {
            "Unavailable".to_owned()
        } else {
            format!("{availability} · Unavailable")
        }
    } else {
        availability.to_owned()
    };
    let availability = if availability.is_empty() {
        String::new()
    } else {
        format!("<span>{}</span>", escape_html(&availability))
    };
    let title = href
        .map(|href| {
            format!(
                r#"<a class="entry-title" href="{}">{}</a>"#,
                escape_html(href),
                escape_html(title)
            )
        })
        .unwrap_or_else(|| {
            format!(
                r#"<span class="entry-title unavailable">{}</span>"#,
                escape_html(title)
            )
        });
    format!(
        r#"<article class="entry-row"><div class="entry-main">{title}<p>{}</p></div><div class="entry-meta"><span>{}</span><span>{}</span>{availability}</div></article>"#,
        escape_html(description),
        escape_html(report_type),
        escape_html(status),
        title = title,
        availability = availability
    )
}

fn page(title: &str, heading: &str, intro: &str, body: &str) -> String {
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{}</title><style>{CSS}</style></head><body><main class="page"><header class="hero"><a class="brand" href="/reports/index.html">Powered Descent Lab · Reports</a><h1>{}</h1><p>{}</p></header>{}<footer><a href="/reports/index.html">Reports home</a><a href="/reports/library/index.html">Browse all reports</a><a href="/reports/data/index.html">Raw data</a></footer></main></body></html>"#,
        escape_html(title),
        escape_html(heading),
        escape_html(intro),
        body
    )
}

fn stable_entries(root: &Path, scope: &str) -> Result<Vec<String>> {
    let directory = root.join(scope);
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for child in fs::read_dir(&directory).with_context(|| format!("read stable {scope} reports"))? {
        let child = child?;
        let name = child.file_name().to_string_lossy().into_owned();
        if name == "index.html" || name == "latest" {
            continue;
        }
        let path = Path::new(scope)
            .join(name)
            .join("index.html")
            .to_string_lossy()
            .into_owned();
        if report_file_exists(root, &path)? {
            entries.push(path);
        }
    }
    entries.sort();
    Ok(entries)
}

fn active_href(root: &Path, path: Option<&str>) -> Result<Option<String>> {
    let Some(path) = path else { return Ok(None) };
    if report_file_exists(root, path)? {
        Ok(Some(report_href(path)))
    } else {
        Ok(None)
    }
}

fn report_file_exists(root: &Path, relative: &str) -> Result<bool> {
    validate_report_path(relative)?;
    if fs::symlink_metadata(root).is_err() {
        return Ok(false);
    }
    let canonical_root = root.canonicalize()?;
    let target = root.join(relative);
    match fs::symlink_metadata(&target) {
        Ok(_) => {
            let canonical = target
                .canonicalize()
                .with_context(|| format!("resolve report entrypoint {}", target.display()))?;
            ensure!(
                canonical.starts_with(&canonical_root),
                "report entrypoint escapes stable report tree: {}",
                target.display()
            );
            ensure!(
                canonical.is_file(),
                "report entrypoint is not a file: {}",
                target.display()
            );
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut ancestor = target.as_path();
            loop {
                match fs::symlink_metadata(ancestor) {
                    Ok(_) => {
                        ensure!(
                            ancestor.canonicalize()?.starts_with(&canonical_root),
                            "report path escapes stable report tree: {}",
                            ancestor.display()
                        );
                        return Ok(false);
                    }
                    Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => {
                        ancestor = ancestor.parent().ok_or_else(|| {
                            anyhow::anyhow!("report entrypoint has no existing ancestor")
                        })?;
                    }
                    Err(other) => return Err(other).context("inspect report entrypoint ancestor"),
                }
            }
        }
        Err(error) => {
            Err(error).with_context(|| format!("inspect report entrypoint {}", target.display()))
        }
    }
}

fn raw_collection_exists(root: &Path, relative: &str) -> Result<bool> {
    validate_raw_path(relative)?;
    if fs::symlink_metadata(root).is_err() {
        return Ok(false);
    }
    let canonical_root = root.canonicalize()?;
    let target = root.join(relative);
    match fs::symlink_metadata(&target) {
        Ok(_) => {
            let canonical = target
                .canonicalize()
                .with_context(|| format!("resolve raw collection {}", target.display()))?;
            ensure!(
                canonical.starts_with(&canonical_root),
                "raw collection escapes outputs: {}",
                target.display()
            );
            ensure!(
                canonical.is_dir(),
                "raw collection is not a directory: {}",
                target.display()
            );
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => {
            Err(error).with_context(|| format!("inspect raw collection {}", target.display()))
        }
    }
}

fn validate_id(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')),
        "invalid navigation id: {value:?}"
    );
    Ok(())
}

fn validate_text(value: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && value.len() <= 2_000 && !value.chars().any(char::is_control),
        "invalid navigation text"
    );
    Ok(())
}

fn validate_report_path(value: &str) -> Result<()> {
    let path = Path::new(value);
    ensure!(
        path.is_relative()
            && path.components().count() > 1
            && path.components().all(|c| matches!(c, Component::Normal(_)))
            && path.file_name().is_some_and(|name| name == "index.html")
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/')),
        "unsafe report entrypoint path: {value}"
    );
    Ok(())
}

fn validate_raw_path(value: &str) -> Result<()> {
    let path = Path::new(value);
    ensure!(
        path.is_relative()
            && path.components().count() == 1
            && path.components().all(|c| matches!(c, Component::Normal(_)))
            && value != "reports"
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.')),
        "unsafe raw collection path: {value}"
    );
    Ok(())
}

fn report_href(path: &str) -> String {
    format!("/reports/{path}")
}
fn path_id(path: &str) -> String {
    path.strip_suffix("/index.html")
        .unwrap_or(path)
        .replace('/', "-")
}
fn type_id(value: &str) -> String {
    slug(value)
}
fn slug(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
fn slug_title(value: &str) -> String {
    value
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

const LIBRARY_JS: &str = r#"
const rows=[...document.querySelectorAll('#report-library .report-row')],search=document.querySelector('#report-search'),topic=document.querySelector('#topic-filter'),type=document.querySelector('#type-filter'),count=document.querySelector('#report-count');
function apply(){const q=search.value.trim().toLowerCase();let n=0;for(const row of rows){const show=(topic.value==='all'||row.dataset.topic===topic.value)&&(type.value==='all'||row.dataset.type===type.value)&&(!q||row.dataset.search.includes(q));row.hidden=!show;if(show)n++;}count.textContent=`${n} reports`;}
search.addEventListener('input',apply);topic.addEventListener('change',apply);type.addEventListener('change',apply);apply();
"#;

const CSS: &str = r#"
:root{color-scheme:light;--canvas:#f1ede5;--paper:#fffdf8;--ink:#20211e;--muted:#6d665c;--line:#d9cdbc;--rust:#b95024;--green:#176b5c;--shadow:0 18px 44px rgba(54,39,25,.08);--sans:"Avenir Next","IBM Plex Sans","Trebuchet MS",sans-serif;--display:"Iowan Old Style","Palatino Linotype",Georgia,serif;--mono:"Iosevka Term","SFMono-Regular",Consolas,monospace}
*{box-sizing:border-box}body{margin:0;color:var(--ink);font-family:var(--sans);background:radial-gradient(circle at 7% -8%,rgba(185,80,36,.13),transparent 31rem),linear-gradient(180deg,#fbf8f2,var(--canvas));min-height:100vh}.page{width:min(1120px,100%);margin:auto;padding:28px 22px 60px}.hero{border:1px solid var(--line);border-radius:22px;background:rgba(255,253,248,.92);box-shadow:var(--shadow);padding:22px 26px;margin:0 0 22px}.brand,.eyebrow{font-size:.75rem;letter-spacing:.1em;text-transform:uppercase;color:var(--muted)}.brand{text-decoration:none}.hero h1{font:clamp(2rem,5vw,3.4rem)/1.05 var(--display);margin:16px 0 9px}.hero p{max-width:760px;line-height:1.55;color:var(--muted);margin:0}.page>section{margin:26px 0}.page h2{font:1.55rem var(--display);margin:0 0 14px}.subject-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:14px}.subject{border:1px solid var(--line);border-radius:16px;background:var(--paper);box-shadow:var(--shadow);padding:18px;display:flex;flex-direction:column;gap:12px}.subject>a:first-child{color:inherit;text-decoration:none}.subject h2{font:1.65rem var(--display);margin:8px 0}.subject p,.entry-main p,.report-main p{color:var(--muted);line-height:1.5;margin:7px 0}.action,.shortcut,.minor-link{display:inline-block;color:var(--green);font-weight:700}.shortcut{border-top:1px solid var(--line);padding-top:11px;font-size:.9rem}.secondary{border-top:1px solid var(--line);padding-top:20px}.secondary-links{display:flex;flex-wrap:wrap;gap:10px}.secondary-links a,.section-footer a{border:1px solid var(--line);border-radius:999px;background:var(--paper);padding:9px 14px;color:var(--ink);text-decoration:none}.breadcrumbs{display:flex;flex-wrap:wrap;gap:8px;align-items:center;color:var(--muted);font-size:.88rem;margin:0 0 16px}.breadcrumbs a,footer a{color:var(--green)}.entry-list{display:grid;gap:9px}.entry-row,.report-row{border:1px solid var(--line);border-radius:12px;background:var(--paper);padding:14px 16px;display:grid;grid-template-columns:minmax(0,1fr) auto;gap:14px;align-items:center}.entry-title,.report-title{font-weight:700;color:var(--ink);text-decoration:none}.entry-title:hover,.report-title:hover,.breadcrumbs a:hover{color:var(--rust);text-decoration:underline}.unavailable{color:var(--muted)}.entry-main code,.report-main code{display:block;font:.78rem var(--mono);color:var(--muted);margin-top:7px;overflow-wrap:anywhere}.entry-meta,.report-meta{display:flex;flex-wrap:wrap;justify-content:flex-end;gap:7px;max-width:340px}.entry-meta span,.report-meta span{border:1px solid var(--line);border-radius:999px;padding:5px 8px;font-size:.77rem;color:var(--muted)}.minor-link{display:block;font-size:.82rem;margin-top:8px}.section-footer{margin-top:18px}.library-note{color:var(--muted);line-height:1.5}.filters{display:grid;grid-template-columns:minmax(220px,2fr) repeat(2,minmax(150px,1fr)) auto;gap:10px;align-items:end;margin:18px 0}.filters label{display:grid;gap:6px;font-size:.83rem;font-weight:700}.filters input,.filters select{font:inherit;border:1px solid var(--line);border-radius:9px;background:var(--paper);padding:10px;color:var(--ink);min-width:0}.filters #report-count{padding:10px;color:var(--muted);font-size:.85rem}.report-rows{display:grid;gap:7px}.report-row{grid-template-columns:minmax(0,1fr) minmax(150px,270px)}.report-main{min-width:0}.report-title{display:inline-block}.report-meta{align-items:flex-end}.report-row[hidden]{display:none}.empty{padding:18px;background:var(--paper);border:1px solid var(--line);border-radius:12px;color:var(--muted)}footer{display:flex;flex-wrap:wrap;gap:16px;border-top:1px solid var(--line);padding-top:18px;margin-top:30px;font-size:.86rem}
@media(max-width:760px){.page{padding:16px 12px 42px}.hero{padding:19px}.subject-grid{grid-template-columns:1fr}.entry-row,.report-row{grid-template-columns:1fr}.entry-meta,.report-meta{justify-content:flex-start;max-width:none}.filters{grid-template-columns:1fr 1fr}.filters label:first-child,.filters #report-count{grid-column:1/-1}.filters #report-count{padding:2px 0}.report-row{padding:12px}.subject{padding:16px}}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::site::ReportSite;

    fn fixture_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "pd-topic-nav-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("fixtures/reports")).unwrap();
        fs::create_dir_all(root.join("fixtures/packs")).unwrap();
        fs::create_dir_all(root.join("outputs/reports/eval/unknown_study")).unwrap();
        fs::write(
            root.join(NAVIGATION_FIXTURE),
            include_str!("../../fixtures/reports/report_navigation.json"),
        )
        .unwrap();
        fs::write(
            root.join("fixtures/reports/guidance_catalog.json"),
            include_str!("../../fixtures/reports/guidance_catalog.json"),
        )
        .unwrap();
        fs::write(root.join("fixtures/packs/terminal_bot_lab_suite.json"), r#"{"id":"terminal_bot_lab_suite","name":"Terminal test","description":"<script>unsafe & text</script>"}"#).unwrap();
        fs::write(root.join("fixtures/packs/planner_v2_lab_suite.json"), r#"{"id":"planner_v2_lab_suite","name":"Planner V2 lab suite","description":"Current V2 evaluation"}"#).unwrap();
        fs::write(
            root.join("outputs/reports/eval/unknown_study/index.html"),
            "immutable report body",
        )
        .unwrap();
        root
    }

    #[test]
    fn hierarchy_has_one_home_and_honest_missing_and_unclassified_reports() {
        let root = fixture_root();
        ReportSite::new(&root).refresh_home().unwrap();
        let home = fs::read_to_string(root.join("outputs/index.html")).unwrap();
        assert_eq!(
            home,
            fs::read_to_string(root.join("outputs/reports/index.html")).unwrap()
        );
        assert!(home.contains("Waypoint planning") && home.contains("Flight and landing control"));
        assert!(
            !home.contains("href=\"/reports/runs") && !home.contains("href=\"/reports/replays")
        );
        let library = fs::read_to_string(root.join("outputs/reports/library/index.html")).unwrap();
        assert_eq!(
            library,
            fs::read_to_string(root.join("outputs/reports/eval/index.html")).unwrap()
        );
        assert!(library.contains("Unclassified") && library.contains("Unknown Study"));
        assert!(library.contains("href=\"/reports/eval/unknown_study/index.html\""));
        assert!(
            library.contains("Not captured")
                && library.contains("&lt;script&gt;unsafe &amp; text&lt;/script&gt;")
        );
        assert!(!library.contains("href=\"/reports/eval/terminal_bot_lab_suite"));
        assert!(library.contains("id=\"topic-filter\"") && library.contains("id=\"type-filter\""));
        let planning =
            fs::read_to_string(root.join("outputs/reports/topics/waypoint-planning/index.html"))
                .unwrap();
        assert!(planning.contains("Waypoint planner V2") && planning.contains("unavailable"));
        assert!(!planning.contains("Open preview landing page"));
        assert_eq!(
            fs::read_to_string(root.join("outputs/reports/eval/unknown_study/index.html")).unwrap(),
            "immutable report body"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_navigation_fails_before_home_writes() {
        let root = fixture_root();
        fs::write(root.join("outputs/index.html"), "root sentinel").unwrap();
        fs::write(root.join("outputs/reports/index.html"), "reports sentinel").unwrap();
        let original: serde_json::Value = serde_json::from_str(include_str!(
            "../../fixtures/reports/report_navigation.json"
        ))
        .unwrap();
        for mutate in 0..5 {
            let mut bad = original.clone();
            match mutate {
                0 => bad["schema_version"] = 99.into(),
                1 => {
                    bad["topics"][0]["entries"][2]["source"]["path"] = "../escape/index.html".into()
                }
                2 => bad["topics"][1]["id"] = "other-topic".into(),
                3 => bad["topics"][0]["entries"][1]["id"] = "planner-v2-current".into(),
                _ => bad["unknown_setting"] = true.into(),
            }
            fs::write(
                root.join(NAVIGATION_FIXTURE),
                serde_json::to_vec(&bad).unwrap(),
            )
            .unwrap();
            assert!(ReportSite::new(&root).refresh_home().is_err());
            assert_eq!(
                fs::read_to_string(root.join("outputs/index.html")).unwrap(),
                "root sentinel"
            );
            assert_eq!(
                fs::read_to_string(root.join("outputs/reports/index.html")).unwrap(),
                "reports sentinel"
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn current_planner_batch_is_reachable_and_classified_without_promoting_old_preview() {
        let root = fixture_root();
        ReportSite::new(&root).refresh_home().unwrap();
        let missing = fs::read_to_string(root.join("outputs/index.html")).unwrap();
        assert!(missing.contains("Current planner V2 batch not captured yet"));
        assert!(!missing.contains("Open current V2 preview"));
        let batch_dir = root.join("outputs/reports/eval/planner_v2_lab_suite");
        fs::create_dir_all(&batch_dir).unwrap();
        fs::write(batch_dir.join("index.html"), "current batch sentinel").unwrap();
        ReportSite::new(&root).refresh_home().unwrap();
        let home = fs::read_to_string(root.join("outputs/index.html")).unwrap();
        assert!(home.contains("Open current planner V2 batch"));
        assert!(home.contains("href=\"/reports/eval/planner_v2_lab_suite/index.html\""));
        let topic =
            fs::read_to_string(root.join("outputs/reports/topics/waypoint-planning/index.html"))
                .unwrap();
        assert!(topic.contains("href=\"/reports/eval/planner_v2_lab_suite/index.html\""));
        assert!(
            topic.contains("Legacy V1 planner baseline")
                && topic.contains("Historical presentation")
        );
        let library = fs::read_to_string(root.join("outputs/reports/library/index.html")).unwrap();
        assert!(library.contains("Active · default planner evaluation"));
        assert!(library.contains("Planner V2 · current evaluation batch"));
        assert_eq!(
            fs::read_to_string(batch_dir.join("index.html")).unwrap(),
            "current batch sentinel"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn hierarchy_rejects_input_and_output_symlink_escapes_without_writes() {
        let root = fixture_root();
        let outside = root.join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("index.html"), "outside sentinel").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("outputs/reports/eval/escaped_report"))
            .unwrap();
        assert!(ReportSite::new(&root).refresh_home().is_err());
        assert!(!root.join("outputs/index.html").exists());
        fs::remove_file(root.join("outputs/reports/eval/escaped_report")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("outputs/reports/topics")).unwrap();
        assert!(ReportSite::new(&root).refresh_home().is_err());
        assert!(!root.join("outputs/index.html").exists());
        assert_eq!(
            fs::read_to_string(outside.join("index.html")).unwrap(),
            "outside sentinel"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
