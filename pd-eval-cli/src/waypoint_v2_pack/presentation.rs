use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use pd_report::flight_annotations::{AnnotationNavigation, NavigationLink};
use serde_json;

use super::{
    capture_validation::{safe_capture_file, validate_batch_capture},
    model::{WaypointV2BatchCase, WaypointV2BatchReport, WaypointV2PackInput},
    provenance::capture_source_state,
    tree_report,
};
use crate::{
    WaypointV2FlightResult,
    evidence_io::{
        reserve_output_root, sha256_bytes, write_bytes_create_only_with_context,
        write_json_create_only,
    },
    waypoint_v2_report::render_rich_flight,
};

pub(super) fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) fn detail_html(
    input: &WaypointV2PackInput,
    case: &WaypointV2BatchCase,
    result: &WaypointV2FlightResult,
    index: usize,
    cases: &[WaypointV2BatchCase],
    source_base: Option<&str>,
) -> Result<String> {
    let source_link = |relative: &str, local: &str| {
        source_base.map_or_else(|| local.to_owned(), |base| format!("{base}{relative}"))
    };
    if result.manifest.is_some() {
        let prior = index.checked_sub(1).map(|i| &cases[i]);
        let next = cases.get(index + 1);
        let link = |label: &str, href: String| NavigationLink {
            label: label.into(),
            href,
        };
        let navigation = AnnotationNavigation {
            home: Some(link("Report home", "/reports/".into())),
            collection: Some(link("Batch summary", "../../index.html".into())),
            previous: prior
                .map(|case| link("Previous case", format!("../{}/index.html", case.case_id))),
            next: next.map(|case| link("Next case", format!("../{}/index.html", case.case_id))),
            source_links: vec![
                link(
                    "Scenario JSON",
                    source_link(&case.scenario_path, "scenario.json"),
                ),
                link("Flight JSON", source_link(&case.flight_path, "flight.json")),
                link(
                    "Run summary JSON",
                    source_link(&case.summary_path, "summary.json"),
                ),
                link(
                    "Expanded batch inputs",
                    source_link("expanded-inputs.json", "../../expanded-inputs.json"),
                ),
                link(
                    "Waypoint planning topic",
                    "/reports/topics/waypoint-planning/index.html".into(),
                ),
            ],
        };
        return render_rich_flight(
            &input.scenario,
            result,
            navigation,
            format!(
                "Native V2 lab · {} · {} / {} · policy {}",
                case.case_id,
                case.group.as_str(),
                case.family,
                result.policy.policy_id
            ),
        );
    }
    let stop = escape_html(case.planning_stop.as_deref().unwrap_or("missing"));
    let reason = escape_html(
        case.reason
            .as_deref()
            .unwrap_or("No preflight reason recorded."),
    );
    let status_message = if case.status == "preflight_rejected" {
        "This input was rejected during preflight. No simulator trajectory or rich flight report was created."
    } else {
        "A simulator attempt exists, but no final-source-validated trajectory report was created. Treat this as unverified evidence, not as a preflight rejection or a landing."
    };
    let previous = index
        .checked_sub(1)
        .and_then(|previous| cases.get(previous))
        .map(|previous| {
            format!(
                "<a rel=\"prev\" href=\"../{}/index.html\">Previous case</a>",
                escape_html(&previous.case_id)
            )
        })
        .unwrap_or_default();
    let next = cases
        .get(index + 1)
        .map(|next| {
            format!(
                "<a rel=\"next\" href=\"../{}/index.html\">Next case</a>",
                escape_html(&next.case_id)
            )
        })
        .unwrap_or_default();
    let navigation = format!(
        r#"<nav><a href="../../index.html">Batch summary</a> · <a href="/reports/">Report home</a> · <a href="/reports/topics/waypoint-planning/index.html">Waypoint planning</a> · {previous} {next}</nav>"#
    );
    let body = format!(
        r###"<p>{}</p><p>Planning stop: <code>{}</code></p><p>{}</p><p>Expected preflight: <code>{}</code></p><ul><li><a href="{}">Scenario JSON</a></li><li><a href="{}">Typed preflight result JSON</a></li><li><a href="{}">Run summary JSON</a></li><li><a href="{}">Expanded inputs snapshot</a></li></ul>"###,
        escape_html(status_message),
        stop,
        reason,
        escape_html(case.expected_preflight.as_deref().unwrap_or("none")),
        escape_html(&source_link(&case.scenario_path, "scenario.json")),
        escape_html(&source_link(&case.flight_path, "flight.json")),
        escape_html(&source_link(&case.summary_path, "summary.json")),
        escape_html(&source_link(
            "expanded-inputs.json",
            "../../expanded-inputs.json"
        ))
    );
    Ok(pd_report::batch::render_status_page(
        &case.case_id,
        &navigation,
        &body,
    ))
}

fn write_derived(path: &Path, contents: &[u8]) -> Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "refusing unsafe derived page target {}",
            path.display()
        );
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temp = path.with_extension(format!("{}.{}.tmp", std::process::id(), nonce));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    fs::rename(&temp, path).with_context(|| format!("write derived page {}", path.display()))
}

pub fn render_waypoint_v2_batch(capture_root: &Path) -> Result<WaypointV2BatchReport> {
    let summary_path = safe_capture_file(capture_root, "summary.json")?;
    let report: WaypointV2BatchReport = serde_json::from_slice(&fs::read(summary_path)?)?;
    let verified = validate_batch_capture(capture_root, &report)?;
    let mut pages = Vec::with_capacity(report.cases.len() + 2);
    for (index, ((input, result), case)) in verified.iter().zip(&report.cases).enumerate() {
        let html = detail_html(input, case, result, index, &report.cases, None)?;
        let path = safe_output_path(capture_root, &case.annotated_report_path)?;
        pages.push((path, html));
    }
    let batch_html = tree_report::render(&report, &verified, None)?;
    pages.push((
        safe_output_path(capture_root, "index.html")?,
        batch_html.clone(),
    ));
    pages.push((safe_output_path(capture_root, "report.html")?, batch_html));
    // All raw artifacts and every generated page are validated/built before the first write.
    for (path, html) in pages {
        write_derived(&path, html.as_bytes())?;
    }
    Ok(report)
}

/// Validate a native capture without changing any captured or derived artifacts.
pub fn validated_waypoint_v2_batch(capture_root: &Path) -> Result<WaypointV2BatchReport> {
    let bytes = fs::read(safe_capture_file(capture_root, "summary.json")?)?;
    let report: WaypointV2BatchReport = serde_json::from_slice(&bytes)?;
    validate_batch_capture(capture_root, &report)?;
    Ok(report)
}

/// Build the normal report-site edition entirely in memory. Raw evidence links
/// point to the capture, while normal batch/case navigation stays in the site.
pub fn render_waypoint_v2_site_pages(
    capture_root: &Path,
    source_base: &str,
) -> Result<(WaypointV2BatchReport, Vec<(PathBuf, String)>)> {
    ensure!(source_base.starts_with('/') && source_base.ends_with('/'));
    ensure!(
        source_base
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/-_.%".contains(&b)),
        "unsafe capture URL"
    );
    let report: WaypointV2BatchReport =
        serde_json::from_slice(&fs::read(safe_capture_file(capture_root, "summary.json")?)?)?;
    let verified = validate_batch_capture(capture_root, &report)?;
    let mut pages = Vec::with_capacity(report.case_count + 1);
    for (index, ((input, result), case)) in verified.iter().zip(&report.cases).enumerate() {
        pages.push((
            PathBuf::from(&case.annotated_report_path),
            detail_html(input, case, result, index, &report.cases, Some(source_base))?,
        ));
    }
    pages.push((
        PathBuf::from("index.html"),
        tree_report::render_site(&report, &verified, source_base)?,
    ));
    Ok((report, pages))
}

/// Build only a separate tree preview, linking to unchanged captured details.
/// No simulation, detail regeneration, current selection or site publication.
pub fn render_waypoint_v2_batch_preview(
    capture_root: &Path,
    preview_root: &Path,
) -> Result<PathBuf> {
    let summary_path = safe_capture_file(capture_root, "summary.json")?;
    let summary_bytes = fs::read(summary_path)?;
    let report: WaypointV2BatchReport = serde_json::from_slice(&summary_bytes)?;
    let verified = validate_batch_capture(capture_root, &report)?;
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("repository root")?;
    let outputs = repo.join("outputs").canonicalize()?;
    let source = capture_root.canonicalize()?;
    let relative_source = source
        .strip_prefix(&outputs)
        .context("preview capture must be beneath served outputs")?;
    let destination = preview_destination(&outputs, &source, preview_root)?;
    let source_href = format!("/{}/", relative_source.to_string_lossy().replace('\\', "/"));
    ensure!(
        source_href
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/-_.".contains(&b)),
        "unsafe capture URL"
    );
    let html = tree_report::render(&report, &verified, Some(&source_href))?;
    let renderer_source = capture_source_state(repo)?;
    reserve_output_root(&destination)?;
    write_bytes_create_only_with_context(
        &destination.join("index.html"),
        html.as_bytes(),
        "create-only artifact",
    )?;
    write_json_create_only(
        &destination.join("preview.json"),
        &serde_json::json!({
            "schema_id":"planner_v2_batch_tree_preview_v1", "source_capture":source,
            "source_summary_sha256":sha256_bytes(&summary_bytes)?, "capture_base_href":source_href,
            "rendered_html_sha256":sha256_bytes(html.as_bytes())?, "renderer_source":renderer_source,
            "case_count":report.case_count, "scope":"presentation only; current batch and detailed pages unchanged"
        }),
    )?;
    Ok(destination.join("index.html"))
}

pub(super) fn preview_destination(
    outputs: &Path,
    source: &Path,
    requested: &Path,
) -> Result<PathBuf> {
    let destination = if requested.is_absolute() {
        requested.to_owned()
    } else {
        std::env::current_dir()?.join(requested)
    };
    ensure!(
        !destination
            .components()
            .any(|c| matches!(c, Component::ParentDir)),
        "preview destination cannot contain parent traversal"
    );
    ensure!(
        destination.starts_with(outputs.join("research"))
            && destination != outputs.join("research")
            && !destination.starts_with(source),
        "preview must be a separate directory beneath outputs/research"
    );
    let mut cursor = PathBuf::new();
    for component in destination.components() {
        cursor.push(component);
        if let Ok(metadata) = fs::symlink_metadata(&cursor) {
            ensure!(
                !metadata.file_type().is_symlink(),
                "symlink in preview destination"
            );
        }
    }
    Ok(destination)
}

fn safe_output_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    ensure!(
        !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "unsafe derived output path {relative}"
    );
    let root_metadata = fs::symlink_metadata(root)?;
    ensure!(
        root_metadata.is_dir() && !root_metadata.file_type().is_symlink(),
        "capture root must be a real directory"
    );
    let canonical_root = root.canonicalize()?;
    let components = path.components().collect::<Vec<_>>();
    let mut cursor = canonical_root.clone();
    for (index, component) in components.iter().enumerate() {
        cursor.push(component.as_os_str());
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) if index + 1 == components.len() => ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "unsafe derived page target {}",
                cursor.display()
            ),
            Ok(metadata) => ensure!(
                metadata.is_dir() && !metadata.file_type().is_symlink(),
                "unsafe derived page parent {}",
                cursor.display()
            ),
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && index + 1 == components.len() => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(cursor)
}
