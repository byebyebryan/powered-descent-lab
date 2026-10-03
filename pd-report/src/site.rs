use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

use crate::report_navigation::{self, PreparedReportNavigation, PreviewTargets};

/// Pins only preview navigation, not an accepted capture or planner default.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NavigationPreviewSelection {
    schema_version: u32,
    home_entrypoint: String,
    collection_entrypoint: String,
}

enum NavigationPreview {
    Absent,
    Unavailable,
    Available { home: String, collection: String },
}

/// Shared owner for the stable HTML report tree under `outputs/reports`.
pub struct ReportSite {
    repo_root: PathBuf,
    outputs_root: PathBuf,
    reports_root: PathBuf,
    fixture_pack_dir: Option<PathBuf>,
}

impl ReportSite {
    pub fn new(repo_root: impl Into<PathBuf>) -> Self {
        let repo_root = repo_root.into();
        let outputs_root = repo_root.join("outputs");
        let reports_root = outputs_root.join("reports");
        Self {
            repo_root,
            outputs_root,
            reports_root,
            fixture_pack_dir: None,
        }
    }

    pub fn with_fixture_pack_dir(mut self, fixture_pack_dir: impl Into<PathBuf>) -> Self {
        self.fixture_pack_dir = Some(fixture_pack_dir.into());
        self
    }

    pub fn outputs_root(&self) -> &Path {
        &self.outputs_root
    }

    pub fn reports_root(&self) -> &Path {
        &self.reports_root
    }

    pub fn default_output_for_bundle(&self, bundle_dir: &Path) -> Option<PathBuf> {
        let resolved = self.resolve_repo_relative(bundle_dir);
        let relative = resolved.strip_prefix(&self.outputs_root).ok()?;
        Some(self.reports_root.join(relative).join("index.html"))
    }

    pub fn update_indexes_for_file(&self, report_file: &Path) -> Result<()> {
        let navigation = self.prepare_navigation()?;
        let report_dir = report_file
            .parent()
            .ok_or_else(|| anyhow::anyhow!("report output has no parent directory"))?;
        self.update_latest_link(report_dir)?;

        let resolved_report_dir = self.resolve_repo_relative(report_dir);
        if !resolved_report_dir.starts_with(&self.reports_root) {
            return Ok(());
        }
        if let Some(navigation) = navigation {
            if self.generic_scope_index_allowed(&resolved_report_dir)? {
                if let Some(collection) = collection_dir(&resolved_report_dir, &self.reports_root) {
                    self.write_collection_index(&collection)?;
                }
                if let Some(scope) = scope_dir(&resolved_report_dir, &self.reports_root) {
                    self.write_scope_index(&scope)?;
                }
            }
            self.write_navigation(&navigation)?;
            return Ok(());
        }
        if let Some(collection) = collection_dir(&resolved_report_dir, &self.reports_root) {
            self.write_collection_index(&collection)?;
        }
        if let Some(scope) = scope_dir(&resolved_report_dir, &self.reports_root) {
            self.write_scope_index(&scope)?;
        }
        self.write_home_index()?;
        self.write_outputs_index()?;
        Ok(())
    }

    pub fn refresh_indexes(&self) -> Result<()> {
        let navigation = self.prepare_navigation()?;
        for scope in ["runs", "replays", "eval", "setups"] {
            let scope_dir = self.reports_root.join(scope);
            if scope_dir.exists() && (navigation.is_none() || matches!(scope, "runs" | "replays")) {
                self.write_scope_index(&scope_dir)?;
            }
        }
        if let Some(navigation) = navigation {
            return self.write_navigation(&navigation);
        }
        self.write_home_index()?;
        self.write_outputs_index()
    }

    pub fn refresh_home(&self) -> Result<()> {
        if let Some(navigation) = self.prepare_navigation()? {
            return self.write_navigation(&navigation);
        }
        self.write_home_index()?;
        self.write_outputs_index()
    }

    fn prepare_navigation(&self) -> Result<Option<PreparedReportNavigation>> {
        let preview = match self.navigation_preview()? {
            NavigationPreview::Available { home, collection } => PreviewTargets {
                home: Some(home),
                collection: Some(collection),
            },
            NavigationPreview::Absent | NavigationPreview::Unavailable => PreviewTargets::default(),
        };
        report_navigation::prepare(&self.repo_root, &self.reports_root, &preview)
    }

    fn write_navigation(&self, navigation: &PreparedReportNavigation) -> Result<()> {
        // Validate every destination before writing any page. Fixed navigation
        // paths must not overwrite an archive through a planted symlink.
        for (path, _) in navigation.pages() {
            let relative = path.strip_prefix(&self.outputs_root)?;
            let mut current = self.outputs_root.clone();
            for component in std::iter::once(None).chain(relative.components().map(Some)) {
                if let Some(component) = component {
                    current.push(component.as_os_str());
                }
                match fs::symlink_metadata(&current) {
                    Ok(metadata) => ensure!(
                        !metadata.file_type().is_symlink(),
                        "navigation write destination contains a symlink: {}",
                        current.display()
                    ),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(error).context("validate navigation write destination");
                    }
                }
            }
        }
        for (path, html) in navigation.pages() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "failed to create navigation page directory {}",
                        parent.display()
                    )
                })?;
            }
            fs::write(path, html).with_context(|| {
                format!("failed to write report navigation page {}", path.display())
            })?;
        }
        Ok(())
    }

    fn navigation_manifest_exists(&self) -> Result<bool> {
        Ok(self
            .repo_root
            .join("fixtures/reports/report_navigation.json")
            .try_exists()?)
    }

    fn generic_scope_index_allowed(&self, report_dir: &Path) -> Result<bool> {
        if !self.navigation_manifest_exists()? {
            return Ok(true);
        }
        let relative = report_dir
            .strip_prefix(&self.reports_root)
            .unwrap_or(report_dir);
        Ok(relative
            .components()
            .next()
            .and_then(|component| component.as_os_str().to_str())
            .is_some_and(|scope| matches!(scope, "runs" | "replays")))
    }

    pub fn update_latest_link(&self, target_dir: &Path) -> Result<()> {
        let resolved_target = self.resolve_repo_relative(target_dir);
        if !resolved_target.starts_with(&self.outputs_root) {
            return Ok(());
        }
        let Some(parent) = resolved_target.parent() else {
            return Ok(());
        };
        let Some(target_name) = resolved_target.file_name() else {
            return Ok(());
        };
        let latest = parent.join("latest");
        if let Ok(metadata) = fs::symlink_metadata(&latest) {
            if metadata.file_type().is_symlink() || metadata.is_file() {
                fs::remove_file(&latest).with_context(|| {
                    format!("failed to remove existing latest link {}", latest.display())
                })?;
            } else {
                return Ok(());
            }
        }
        create_dir_symlink(Path::new(target_name), &latest).with_context(|| {
            format!(
                "failed to create latest link {} -> {}",
                latest.display(),
                target_name.to_string_lossy()
            )
        })
    }

    fn resolve_repo_relative(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.repo_root.join(path)
        }
    }

    fn write_home_index(&self) -> Result<()> {
        fs::create_dir_all(&self.reports_root).with_context(|| {
            format!(
                "failed to create reports root {}",
                self.reports_root.display()
            )
        })?;
        let mut cards = String::new();
        let preview = self.navigation_preview()?;
        match &preview {
            NavigationPreview::Available { collection, .. } => cards.push_str(&home_card(
                collection,
                "Waypoint planner V2 preview",
                "Grouped mission browsing and the full Late ridge report with waypoint annotations. Under review; other supported missions use their original full reports.",
                "navigation preview · under review",
            )),
            NavigationPreview::Unavailable => cards.push_str(preview_unavailable_card()),
            NavigationPreview::Absent => {}
        }
        if self.reports_root.join("guidance/index.html").exists() {
            cards.push_str(&home_card(
                "guidance/",
                "Guidance overview",
                "Curated terminal, direct-transfer, and waypoint evidence.",
                "recommended",
            ));
        }
        // An explicit preview is the sole V2 reading entry here. Earlier lean
        // editions remain reachable in its history, not competing current cards.
        if matches!(preview, NavigationPreview::Absent)
            && let Some(cards_html) = self.waypoint_v2_cards()?
        {
            cards.push_str(&cards_html);
        }
        for (scope, title, description) in [
            (
                "runs",
                "Run reports",
                "Individual mission reports and plots.",
            ),
            (
                "replays",
                "Replay reports",
                "Deterministic replay evidence.",
            ),
            ("eval", "Batch reports", "All maintained evaluation packs."),
            (
                "setups",
                "Analytical setups",
                "Deterministic setup-only planning evidence.",
            ),
        ] {
            let count = self.scope_entries(&self.reports_root.join(scope))?.len();
            cards.push_str(&home_card(
                &format!("{scope}/"),
                title,
                &format!("{description} {count} indexed entries."),
                "reports",
            ));
        }
        let html = page(
            "Powered Descent Lab Reports",
            "Report Site",
            "Start with curated guidance evidence. Raw bundles remain available outside this stable HTML tree.",
            &format!(r#"<div class="card-grid">{cards}</div>"#),
            "",
        );
        fs::write(self.reports_root.join("index.html"), html).with_context(|| {
            format!(
                "failed to write reports home index {}",
                self.reports_root.join("index.html").display()
            )
        })
    }

    fn navigation_preview(&self) -> Result<NavigationPreview> {
        let selection_file = self
            .repo_root
            .join("fixtures/reports/navigation_preview.json");
        if !selection_file.try_exists()? {
            return Ok(NavigationPreview::Absent);
        }
        let selection: NavigationPreviewSelection =
            serde_json::from_slice(&fs::read(&selection_file)?)
                .context("parse report navigation preview selection")?;
        ensure!(
            selection.schema_version == 1,
            "unsupported navigation preview schema"
        );
        let home = Path::new(&selection.home_entrypoint);
        let collection = Path::new(&selection.collection_entrypoint);
        for relative in [home, collection] {
            ensure!(
                relative.components().count() > 1
                    && relative
                        .components()
                        .all(|c| matches!(c, Component::Normal(_)))
                    && relative.file_name().is_some_and(|n| n == "index.html")
                    && relative.to_str().is_some_and(|s| s.bytes().all(|b| b
                        .is_ascii_alphanumeric()
                        || matches!(b, b'_' | b'-' | b'.' | b'/'))),
                "unsafe navigation preview entrypoint"
            );
        }
        ensure!(
            home != collection && collection.starts_with(home.parent().unwrap()),
            "preview collection must belong to the selected preview home"
        );
        let root = match self.reports_root.canonicalize() {
            Ok(root) => root,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(NavigationPreview::Unavailable);
            }
            Err(error) => return Err(error).context("resolve stable report tree"),
        };
        let mut resolved = Vec::new();
        for relative in [home, collection] {
            let path = match self.reports_root.join(relative).canonicalize() {
                Ok(path) => path,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(NavigationPreview::Unavailable);
                }
                Err(error) => return Err(error).context("resolve selected report preview"),
            };
            ensure!(
                path.starts_with(&root) && path.is_file(),
                "navigation preview escapes the report tree or is not a file"
            );
            resolved.push(path);
        }
        ensure!(
            resolved[0] != resolved[1] && resolved[1].starts_with(resolved[0].parent().unwrap()),
            "resolved preview collection is not bound to its home"
        );
        Ok(NavigationPreview::Available {
            home: selection.home_entrypoint,
            collection: selection.collection_entrypoint,
        })
    }

    /// Independent opt-in presentations, never entries in controller scorecards.
    /// Only completed all-case render receipts are advertised; staged single-case
    /// reviews and arbitrary research directories are not scanned.
    fn waypoint_v2_cards(&self) -> Result<Option<String>> {
        let root = self.reports_root.join("waypoint-v2");
        if !root.is_dir() {
            return Ok(None);
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() || !entry.path().join("index.html").is_file() {
                continue;
            }
            let receipt_path = entry.path().join("render-provenance.json");
            if !receipt_path.is_file() {
                continue;
            }
            let receipt: serde_json::Value = serde_json::from_slice(&fs::read(receipt_path)?)?;
            if receipt["schema_id"] != "waypoint_v2_report_render_v1"
                || receipt.get("selected_case_id") != Some(&serde_json::Value::Null)
            {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            {
                continue;
            }
            entries.push((name, receipt["case_count"].as_u64().unwrap_or(0)));
        }
        entries.sort();
        let html = entries.iter().map(|(name, count)| home_card(
            &format!("waypoint-v2/{name}/index.html"),
            "Waypoint planner V2",
            &format!("{count} retained cases. Actual flight stories and dynamic handoffs; opt-in evidence, separate from controller scorecards."),
            "planner presentation",
        )).collect::<String>();
        Ok((!html.is_empty()).then_some(html))
    }

    fn write_outputs_index(&self) -> Result<()> {
        fs::create_dir_all(&self.outputs_root).with_context(|| {
            format!(
                "failed to create outputs root {}",
                self.outputs_root.display()
            )
        })?;
        let preview_card = match self.navigation_preview()? {
            NavigationPreview::Available { home, .. } => home_card(
                &format!("reports/{home}"),
                "Report navigation preview",
                "Start here to review organized report browsing and the full Late ridge report with waypoint annotations. Preview under review, not a new flight capture.",
                "navigation preview · under review",
            ),
            NavigationPreview::Unavailable => preview_unavailable_card().to_owned(),
            NavigationPreview::Absent => String::new(),
        };
        let body = format!(
            r#"<div class="card-grid">{preview_card}
<a class="card featured" href="reports/"><span class="eyebrow">recommended</span><strong>Report site</strong><span>Curated guidance evidence and stable report navigation.</span></a>
<div class="card"><span class="eyebrow">raw</span><strong>Artifact directories</strong><span>Use raw bundles when report pages do not expose the required detail.</span><div class="links"><a href="runs/">runs/</a><a href="eval/">eval/</a><a href="replays/">replays/</a><a href="setups/">setups/</a></div></div>
</div>"#
        );
        let html = page(
            "Powered Descent Lab Outputs",
            "Outputs",
            "Stable reports are separated from raw simulation and evaluation artifacts.",
            &body,
            "",
        );
        fs::write(self.outputs_root.join("index.html"), html).with_context(|| {
            format!(
                "failed to write outputs root index {}",
                self.outputs_root.join("index.html").display()
            )
        })
    }

    fn write_scope_index(&self, scope_dir: &Path) -> Result<()> {
        if !self.generic_scope_index_allowed(scope_dir)? {
            return Ok(());
        }
        fs::create_dir_all(scope_dir)
            .with_context(|| format!("failed to create scope dir {}", scope_dir.display()))?;
        let entries = self.scope_entries(scope_dir)?;
        let scope = scope_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("reports");
        let rows = report_rows(&entries, Path::new(scope));
        let actions = if scope_dir.join("latest").exists() {
            r#"<a href="../">reports/</a><a href="latest/">latest</a>"#
        } else {
            r#"<a href="../">reports/</a>"#
        };
        let body = format!(
            r#"<div class="table-wrap"><table><thead><tr><th>Name</th><th>Updated</th><th>URL</th></tr></thead><tbody>{rows}</tbody></table></div>"#
        );
        let html = page(
            &format!("{} reports", scope_title(scope)),
            &scope_title(scope),
            "Newest stable report directories first.",
            &body,
            actions,
        );
        fs::write(scope_dir.join("index.html"), html).with_context(|| {
            format!(
                "failed to write scope report index {}",
                scope_dir.join("index.html").display()
            )
        })
    }

    fn write_collection_index(&self, collection_dir: &Path) -> Result<()> {
        if !self.generic_scope_index_allowed(collection_dir)? {
            return Ok(());
        }
        fs::create_dir_all(collection_dir).with_context(|| {
            format!(
                "failed to create collection dir {}",
                collection_dir.display()
            )
        })?;
        let entries = self.scope_entries(collection_dir)?;
        let relative = collection_dir
            .strip_prefix(&self.reports_root)
            .unwrap_or(collection_dir);
        let rows = report_rows(&entries, relative);
        let body = format!(
            r#"<div class="table-wrap"><table><thead><tr><th>Name</th><th>Updated</th><th>URL</th></tr></thead><tbody>{rows}</tbody></table></div>"#
        );
        let html = page(
            &collection_title(relative),
            &collection_title(relative),
            "Nested stable report collection.",
            &body,
            r#"<a href="../">up</a><a href="../../">reports/</a>"#,
        );
        fs::write(collection_dir.join("index.html"), html).with_context(|| {
            format!(
                "failed to write collection index {}",
                collection_dir.join("index.html").display()
            )
        })
    }

    fn scope_entries(&self, scope_dir: &Path) -> Result<Vec<ScopeEntry>> {
        let mut entries = Vec::new();
        if !scope_dir.exists() {
            return Ok(entries);
        }
        let eval_scope =
            normalize_path(scope_dir) == normalize_path(&self.reports_root.join("eval"));
        let fixture_ids = if eval_scope {
            self.fixture_pack_dir
                .as_deref()
                .map(load_fixture_pack_ids)
                .transpose()?
        } else {
            None
        };
        for dir_entry in fs::read_dir(scope_dir)
            .with_context(|| format!("failed to read scope dir {}", scope_dir.display()))?
        {
            let dir_entry = dir_entry?;
            let path = dir_entry.path();
            let name = dir_entry.file_name().to_string_lossy().into_owned();
            if name == "latest" || name == "index.html" || name == "guidance" {
                continue;
            }
            if let Some(fixture_ids) = fixture_ids.as_ref()
                && !eval_report_entry_is_fixture_backed(
                    &self.outputs_root.join("eval"),
                    &name,
                    fixture_ids,
                )
            {
                continue;
            }
            let metadata = fs::symlink_metadata(&path)?;
            if !(metadata.is_dir() || metadata.file_type().is_symlink()) {
                continue;
            }
            let modified = entry_modified_time(&path, &metadata);
            entries.push(ScopeEntry {
                name,
                modified,
                modified_label: modified_label(modified),
            });
        }
        entries.sort_by(|lhs, rhs| {
            rhs.modified
                .cmp(&lhs.modified)
                .then(lhs.name.cmp(&rhs.name))
        });
        Ok(entries)
    }
}

fn preview_unavailable_card() -> &'static str {
    r#"<div class="card"><span class="eyebrow">navigation preview · under review</span><strong>Report navigation preview unavailable</strong><span>The explicitly selected preview has not been rendered here. No historical edition is substituted.</span></div>"#
}

#[derive(Deserialize)]
struct PackIdentity {
    id: String,
}

pub fn load_fixture_pack_ids(fixtures_dir: &Path) -> Result<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for entry in fs::read_dir(fixtures_dir).with_context(|| {
        format!(
            "failed to read scenario pack fixtures {}",
            fixtures_dir.display()
        )
    })? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read scenario pack fixture {}", path.display()))?;
        let identity = serde_json::from_str::<PackIdentity>(&raw)
            .with_context(|| format!("failed to parse scenario pack fixture {}", path.display()))?;
        ids.insert(identity.id);
    }
    Ok(ids)
}

pub fn eval_report_entry_is_fixture_backed(
    raw_eval_dir: &Path,
    entry_name: &str,
    fixture_pack_ids: &BTreeSet<String>,
) -> bool {
    fs::read_to_string(raw_eval_dir.join(entry_name).join("pack.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<PackIdentity>(&raw).ok())
        .is_some_and(|pack| fixture_pack_ids.contains(&pack.id))
}

fn scope_dir(report_dir: &Path, reports_root: &Path) -> Option<PathBuf> {
    let relative = report_dir.strip_prefix(reports_root).ok()?;
    Some(reports_root.join(relative.iter().next()?))
}

fn collection_dir(report_dir: &Path, reports_root: &Path) -> Option<PathBuf> {
    let parent = report_dir.parent()?;
    let relative = parent.strip_prefix(reports_root).ok()?;
    (relative.components().count() > 1).then(|| parent.to_path_buf())
}

fn report_rows(entries: &[ScopeEntry], relative_dir: &Path) -> String {
    if entries.is_empty() {
        return r#"<tr><td colspan="3" class="muted">No reports yet.</td></tr>"#.to_owned();
    }
    entries
        .iter()
        .map(|entry| {
            let path = relative_dir.join(&entry.name);
            format!(
                r#"<tr><td><a href="{name}/">{name}</a></td><td>{modified}</td><td><code>{path}/</code></td></tr>"#,
                name = escape_html(&entry.name),
                modified = escape_html(&entry.modified_label),
                path = escape_html(&path.display().to_string()),
            )
        })
        .collect()
}

fn home_card(href: &str, title: &str, description: &str, eyebrow: &str) -> String {
    let featured = if eyebrow == "recommended" {
        " featured"
    } else {
        ""
    };
    format!(
        r#"<a class="card{featured}" href="{href}"><span class="eyebrow">{eyebrow}</span><strong>{title}</strong><span>{description}</span></a>"#,
        featured = featured,
        href = escape_html(href),
        eyebrow = escape_html(eyebrow),
        title = escape_html(title),
        description = escape_html(description),
    )
}

fn page(title: &str, heading: &str, intro: &str, body: &str, actions: &str) -> String {
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{title}</title><style>{SITE_CSS}</style></head><body><main><header><div><span class="eyebrow">powered descent lab</span><h1>{heading}</h1><p>{intro}</p></div><nav>{actions}</nav></header>{body}</main></body></html>"#,
        title = escape_html(title),
        heading = escape_html(heading),
        intro = escape_html(intro),
    )
}

const SITE_CSS: &str = r#"
:root {
  color-scheme: light;
  --canvas: #f1ede5;
  --paper: #fffdf8;
  --ink: #20211e;
  --muted: #6d665c;
  --line: #d9cdbc;
  --rust: #b95024;
  --green: #176b5c;
  --blue: #315f86;
  --display: "Iowan Old Style", "Palatino Linotype", Georgia, serif;
  --sans: "Avenir Next", "IBM Plex Sans", "Trebuchet MS", sans-serif;
  --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
  --shadow: 0 18px 44px rgba(54, 39, 25, 0.08);
}
* { box-sizing: border-box; }
body {
  margin: 0;
  min-height: 100vh;
  color: var(--ink);
  font-family: var(--sans);
  background:
    radial-gradient(circle at 7% -8%, rgba(185, 80, 36, 0.15), transparent 31rem),
    linear-gradient(rgba(55, 43, 31, 0.018) 1px, transparent 1px),
    linear-gradient(90deg, rgba(55, 43, 31, 0.018) 1px, transparent 1px),
    linear-gradient(180deg, #fbf8f2, var(--canvas));
  background-size: auto, 32px 32px, 32px 32px, auto;
}
main {
  width: min(1180px, 100%);
  margin: auto;
  padding: 28px 22px 64px;
}
header {
  position: relative;
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 24px;
  margin-bottom: 22px;
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
h1 {
  margin: 0.2rem 0 0.55rem;
  font-family: var(--display);
  font-size: clamp(2.1rem, 4.5vw, 3.35rem);
  font-weight: 500;
  line-height: 0.98;
  letter-spacing: -0.025em;
}
p {
  max-width: 72ch;
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
nav, .links { display: flex; flex-wrap: wrap; gap: 8px; }
nav a, .links a {
  border: 1px solid var(--line);
  border-radius: 999px;
  background: rgba(255, 253, 248, 0.82);
  color: inherit;
  padding: 7px 12px;
  text-decoration: none;
  transition: border-color 140ms ease, transform 140ms ease;
}
nav a:hover, .links a:hover { border-color: var(--rust); transform: translateY(-1px); }
a:focus-visible { outline: 3px solid rgba(49, 95, 134, 0.28); outline-offset: 2px; }
.card-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 14px; }
.card {
  position: relative;
  display: flex;
  min-height: 164px;
  flex-direction: column;
  gap: 9px;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 18px;
  background: rgba(255, 253, 248, 0.94);
  box-shadow: 0 12px 32px rgba(54, 39, 25, 0.065);
  color: inherit;
  padding: 18px 19px;
  text-decoration: none;
  transition: transform 160ms ease, box-shadow 160ms ease, border-color 160ms ease;
}
.card::before {
  content: "";
  position: absolute;
  inset: 0 auto 0 0;
  width: 4px;
  background: var(--blue);
  opacity: 0.75;
}
.card.featured::before { background: var(--green); opacity: 1; }
.card strong {
  font-family: var(--display);
  font-size: 1.38rem;
  font-weight: 500;
}
.card > span:last-child { color: var(--muted); line-height: 1.45; }
.card:hover {
  border-color: var(--rust);
  box-shadow: 0 18px 40px rgba(54, 39, 25, 0.12);
  transform: translateY(-3px);
}
.table-wrap {
  overflow-x: auto;
  border: 1px solid var(--line);
  border-radius: 18px;
  background: var(--paper);
  box-shadow: var(--shadow);
}
table { width: 100%; border-collapse: collapse; }
th, td { padding: 11px 13px; border-bottom: 1px solid rgba(216, 206, 190, 0.72); text-align: left; }
th { color: var(--muted); font-size: 0.7rem; letter-spacing: 0.09em; text-transform: uppercase; }
tbody tr:hover { background: #fbf5eb; }
td a { color: var(--green); font-weight: 700; text-decoration: none; }
code { font-family: var(--mono); font-size: 0.84em; overflow-wrap: anywhere; }
.muted { color: var(--muted); }
@media (max-width: 780px) {
  main { padding: 14px 12px 40px; }
  header { display: grid; padding: 18px 17px 20px; }
  header::after { right: -70px; }
  nav { order: -1; }
  .card-grid { grid-template-columns: 1fr; }
  th:nth-child(3), td:nth-child(3) { display: none; }
}
"#;

fn scope_title(scope: &str) -> String {
    match scope {
        "runs" => "Run reports".to_owned(),
        "replays" => "Replay reports".to_owned(),
        "eval" => "Batch reports".to_owned(),
        "setups" => "Analytical setup reports".to_owned(),
        other => format!("{other} reports"),
    }
}

fn collection_title(relative_dir: &Path) -> String {
    let parts = relative_dir
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        "Report collection".to_owned()
    } else {
        format!("{} index", parts.join(" / "))
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn entry_modified_time(path: &Path, metadata: &fs::Metadata) -> SystemTime {
    fs::metadata(path.join("index.html"))
        .and_then(|report| report.modified())
        .unwrap_or_else(|_| metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH))
}

fn modified_label(modified: SystemTime) -> String {
    modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| format!("unix {}", duration.as_secs()))
        .unwrap_or_else(|_| "unknown".to_owned())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

struct ScopeEntry {
    name: String,
    modified: SystemTime,
    modified_label: String,
}

#[cfg(unix)]
fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

#[cfg(test)]
mod tests {
    use super::{ReportSite, eval_report_entry_is_fixture_backed, load_fixture_pack_ids};
    use std::{fs, time::SystemTime};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pd-report-site-{label}-{nonce}"))
    }

    const PREVIEW_HOME: &str = "waypoint-v2/review/index.html";
    const PREVIEW_COLLECTION: &str = "waypoint-v2/review/waypoint-v2/index.html";

    fn select_preview(root: &std::path::Path, home: &str, collection: &str) {
        let dir = root.join("fixtures/reports");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("navigation_preview.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema_version": 1, "home_entrypoint": home, "collection_entrypoint": collection
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn make_preview(root: &std::path::Path) {
        for (relative, body) in [
            (PREVIEW_HOME, "preview home"),
            (PREVIEW_COLLECTION, "preview collection"),
        ] {
            let file = root.join("outputs/reports").join(relative);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, body).unwrap();
        }
        select_preview(root, PREVIEW_HOME, PREVIEW_COLLECTION);
    }

    #[test]
    fn preview_is_reachable_from_both_homes_and_survives_refresh() {
        let root = temp_dir("preview-links");
        make_preview(&root);
        let old = root.join("outputs/reports/waypoint-v2/old");
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("index.html"), "historical report").unwrap();
        fs::write(old.join("render-provenance.json"), r#"{"schema_id":"waypoint_v2_report_render_v1","selected_case_id":null,"case_count":32}"#).unwrap();
        let eval = root.join("outputs/reports/eval/index.html");
        fs::create_dir_all(eval.parent().unwrap()).unwrap();
        fs::write(&eval, "curated catalogue sentinel").unwrap();
        let site = ReportSite::new(&root);
        site.refresh_home().unwrap();
        let outputs = fs::read_to_string(root.join("outputs/index.html")).unwrap();
        let reports = fs::read_to_string(root.join("outputs/reports/index.html")).unwrap();
        assert_eq!(
            outputs
                .matches(&format!("href=\"reports/{PREVIEW_HOME}\""))
                .count(),
            1
        );
        assert_eq!(
            reports
                .matches(&format!("href=\"{PREVIEW_COLLECTION}\""))
                .count(),
            1
        );
        assert!(outputs.contains("navigation preview · under review"));
        assert!(reports.contains("navigation preview · under review"));
        assert!(!reports.contains("waypoint-v2/old/index.html"));
        site.refresh_home().unwrap();
        assert_eq!(
            outputs,
            fs::read_to_string(root.join("outputs/index.html")).unwrap()
        );
        assert_eq!(
            reports,
            fs::read_to_string(root.join("outputs/reports/index.html")).unwrap()
        );
        assert_eq!(
            fs::read_to_string(eval).unwrap(),
            "curated catalogue sentinel"
        );
        assert_eq!(
            fs::read_to_string(old.join("index.html")).unwrap(),
            "historical report"
        );
        assert_eq!(
            fs::read_to_string(root.join("outputs/reports").join(PREVIEW_HOME)).unwrap(),
            "preview home"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_selected_preview_is_unavailable_not_a_historical_fallback() {
        let root = temp_dir("preview-missing");
        make_preview(&root);
        fs::remove_file(root.join("outputs/reports").join(PREVIEW_COLLECTION)).unwrap();
        ReportSite::new(&root).refresh_home().unwrap();
        for relative in ["outputs/index.html", "outputs/reports/index.html"] {
            let html = fs::read_to_string(root.join(relative)).unwrap();
            assert!(html.contains("Report navigation preview unavailable"));
            assert!(!html.contains("href=\"waypoint-v2/review"));
            assert!(!html.contains("href=\"reports/waypoint-v2/review"));
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unsafe_or_mismatched_preview_selection_fails_before_index_writes() {
        let root = temp_dir("preview-invalid");
        make_preview(&root);
        fs::write(root.join("outputs/index.html"), "root sentinel").unwrap();
        fs::write(root.join("outputs/reports/index.html"), "reports sentinel").unwrap();
        let site = ReportSite::new(&root);
        for (home, collection) in [
            ("../escape/index.html", PREVIEW_COLLECTION),
            ("/absolute/index.html", PREVIEW_COLLECTION),
            ("waypoint-v2/a/index.html?x=1", PREVIEW_COLLECTION),
            ("index.html", PREVIEW_COLLECTION),
            (PREVIEW_HOME, "other-preview/index.html"),
            (PREVIEW_HOME, PREVIEW_HOME),
        ] {
            select_preview(&root, home, collection);
            assert!(site.refresh_home().is_err(), "{home} / {collection}");
            assert_eq!(
                fs::read_to_string(root.join("outputs/index.html")).unwrap(),
                "root sentinel"
            );
            assert_eq!(
                fs::read_to_string(root.join("outputs/reports/index.html")).unwrap(),
                "reports sentinel"
            );
        }
        fs::write(root.join("fixtures/reports/navigation_preview.json"), r#"{"schema_version":99,"home_entrypoint":"waypoint-v2/review/index.html","collection_entrypoint":"waypoint-v2/review/waypoint-v2/index.html"}"#).unwrap();
        assert!(site.refresh_home().is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn preview_selection_rejects_symlink_escape() {
        let root = temp_dir("preview-symlink");
        make_preview(&root);
        let escaped = root.join("outside/index.html");
        fs::create_dir_all(escaped.parent().unwrap()).unwrap();
        fs::write(&escaped, "outside the report tree").unwrap();
        let collection = root.join("outputs/reports").join(PREVIEW_COLLECTION);
        fs::remove_file(&collection).unwrap();
        std::os::unix::fs::symlink(escaped, collection).unwrap();
        assert!(ReportSite::new(&root).refresh_home().is_err());
        assert!(!root.join("outputs/index.html").exists());
        assert!(!root.join("outputs/reports/index.html").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn opt_in_v2_navigation_advertises_only_complete_presentations() {
        let root = temp_dir("v2-navigation");
        for (name, selected) in [("complete", "null"), ("stage", "\"v2_ridge_late\"")] {
            let dir = root.join("outputs/reports/waypoint-v2").join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("index.html"), "retained presentation").unwrap();
            fs::write(dir.join("render-provenance.json"), format!(r#"{{"schema_id":"waypoint_v2_report_render_v1","selected_case_id":{selected},"case_count":32}}"#)).unwrap();
        }
        let site = ReportSite::new(&root);
        site.refresh_home().unwrap();
        let html = fs::read_to_string(root.join("outputs/reports/index.html")).unwrap();
        assert!(html.contains("waypoint-v2/complete/index.html"));
        assert!(!html.contains("waypoint-v2/stage/index.html"));
        assert!(html.contains("separate from controller scorecards"));
        assert!(!root.join("outputs/reports/eval/index.html").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn maps_output_bundles_into_stable_report_tree() {
        let root = temp_dir("mapping");
        let site = ReportSite::new(&root);
        assert_eq!(
            site.default_output_for_bundle(&root.join("outputs/runs/example")),
            Some(root.join("outputs/reports/runs/example/index.html"))
        );
    }

    #[test]
    fn recognizes_only_fixture_backed_eval_entries() {
        let root = temp_dir("fixtures");
        let fixtures = root.join("fixtures");
        let raw_eval = root.join("eval");
        fs::create_dir_all(&fixtures).unwrap();
        fs::create_dir_all(raw_eval.join("known")).unwrap();
        fs::write(fixtures.join("known.json"), r#"{"id":"pack_known"}"#).unwrap();
        fs::write(raw_eval.join("known/pack.json"), r#"{"id":"pack_known"}"#).unwrap();
        let ids = load_fixture_pack_ids(&fixtures).unwrap();
        assert!(eval_report_entry_is_fixture_backed(
            &raw_eval, "known", &ids
        ));
        assert!(!eval_report_entry_is_fixture_backed(
            &raw_eval, "missing", &ids
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn setup_scope_is_indexed_without_changing_other_scopes() {
        let root = temp_dir("setups");
        let setup_dir = root.join("outputs/reports/setups/conservative-ballistic-direct-bridge-v2");
        fs::create_dir_all(&setup_dir).unwrap();
        fs::write(setup_dir.join("index.html"), "setup").unwrap();
        let site = ReportSite::new(&root);
        site.refresh_indexes().unwrap();
        let home = fs::read_to_string(root.join("outputs/reports/index.html")).unwrap();
        let outputs = fs::read_to_string(root.join("outputs/index.html")).unwrap();
        let scope = fs::read_to_string(root.join("outputs/reports/setups/index.html")).unwrap();
        assert!(home.contains("setups/"));
        assert!(home.contains("Analytical setups"));
        assert!(outputs.contains("setups/"));
        assert!(scope.contains("conservative-ballistic-direct-bridge-v2"));
        let _ = fs::remove_dir_all(root);
    }
}
