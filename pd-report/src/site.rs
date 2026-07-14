use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result};
use serde::Deserialize;

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
        let report_dir = report_file
            .parent()
            .ok_or_else(|| anyhow::anyhow!("report output has no parent directory"))?;
        self.update_latest_link(report_dir)?;

        let resolved_report_dir = self.resolve_repo_relative(report_dir);
        if !resolved_report_dir.starts_with(&self.reports_root) {
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
        for scope in ["runs", "replays", "eval"] {
            let scope_dir = self.reports_root.join(scope);
            if scope_dir.exists() {
                self.write_scope_index(&scope_dir)?;
            }
        }
        self.write_home_index()?;
        self.write_outputs_index()
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
        if self.reports_root.join("guidance/index.html").exists() {
            cards.push_str(&home_card(
                "guidance/",
                "Guidance overview",
                "Curated terminal, direct-transfer, and waypoint evidence.",
                "recommended",
            ));
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

    fn write_outputs_index(&self) -> Result<()> {
        fs::create_dir_all(&self.outputs_root).with_context(|| {
            format!(
                "failed to create outputs root {}",
                self.outputs_root.display()
            )
        })?;
        let body = r#"<div class="card-grid">
<a class="card featured" href="reports/"><span class="eyebrow">recommended</span><strong>Report site</strong><span>Curated guidance evidence and stable report navigation.</span></a>
<div class="card"><span class="eyebrow">raw</span><strong>Artifact directories</strong><span>Use raw bundles when report pages do not expose the required detail.</span><div class="links"><a href="runs/">runs/</a><a href="eval/">eval/</a><a href="replays/">replays/</a></div></div>
</div>"#;
        let html = page(
            "Powered Descent Lab Outputs",
            "Outputs",
            "Stable reports are separated from raw simulation and evaluation artifacts.",
            body,
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
    format!(
        r#"<a class="card{featured}" href="{href}"><span class="eyebrow">{eyebrow}</span><strong>{title}</strong><span>{description}</span></a>"#,
        featured = (eyebrow == "recommended")
            .then_some(" featured")
            .unwrap_or(""),
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
:root{color-scheme:light;--bg:#f3eee3;--paper:#fffdf8;--ink:#201c17;--muted:#6b6155;--line:#d8cebe;--rust:#aa5124;--green:#246a55;--sans:"Avenir Next","IBM Plex Sans",sans-serif;--mono:"Iosevka Term","SFMono-Regular",monospace}*{box-sizing:border-box}body{margin:0;color:var(--ink);font-family:var(--sans);background:radial-gradient(circle at 8% 0,rgba(170,81,36,.12),transparent 28rem),linear-gradient(180deg,#fbf8f1,var(--bg))}main{width:min(1160px,100%);margin:auto;padding:34px 20px 52px}header{display:flex;justify-content:space-between;align-items:flex-start;gap:24px;margin-bottom:24px}h1{font-family:Georgia,serif;font-size:clamp(2rem,5vw,3.4rem);font-weight:500;line-height:1;margin:.2rem 0 .65rem}p{max-width:72ch;margin:0;color:var(--muted);line-height:1.55}.eyebrow{color:var(--rust);font-size:.72rem;font-weight:800;letter-spacing:.12em;text-transform:uppercase}nav,.links{display:flex;flex-wrap:wrap;gap:8px}nav a,.links a{border:1px solid var(--line);border-radius:999px;background:rgba(255,253,248,.8);color:inherit;padding:7px 11px;text-decoration:none}.card-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:14px}.card{display:flex;min-height:154px;flex-direction:column;gap:9px;border:1px solid var(--line);border-radius:17px;background:rgba(255,253,248,.92);box-shadow:0 12px 35px rgba(48,35,24,.06);color:inherit;padding:18px;text-decoration:none}.card.featured{border-top:4px solid var(--green)}.card strong{font-family:Georgia,serif;font-size:1.35rem;font-weight:500}.card>span:last-child{color:var(--muted);line-height:1.45}.card:hover{border-color:var(--rust);transform:translateY(-1px)}.table-wrap{overflow-x:auto;border:1px solid var(--line);border-radius:17px;background:var(--paper);box-shadow:0 12px 35px rgba(48,35,24,.06)}table{width:100%;border-collapse:collapse}th,td{padding:11px 13px;border-bottom:1px solid rgba(216,206,190,.72);text-align:left}th{color:var(--muted);font-size:.72rem;letter-spacing:.08em;text-transform:uppercase}td a{color:var(--green);font-weight:700;text-decoration:none}code{font-family:var(--mono);font-size:.84em;overflow-wrap:anywhere}.muted{color:var(--muted)}@media(max-width:780px){main{padding:24px 14px 40px}header{display:grid}.card-grid{grid-template-columns:1fr}nav{order:-1}th:nth-child(3),td:nth-child(3){display:none}}
"#;

fn scope_title(scope: &str) -> String {
    match scope {
        "runs" => "Run reports".to_owned(),
        "replays" => "Replay reports".to_owned(),
        "eval" => "Batch reports".to_owned(),
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
}
