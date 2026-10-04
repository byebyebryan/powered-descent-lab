//! Current-view publication for native planner batches. Captures remain
//! create-only; only derived site pages and the explicit selection are updated.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::waypoint_v2_pack::{is_waypoint_v2_pack, render_waypoint_v2_batch};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentPlannerBatch {
    schema_id: String,
    pack_id: String,
    capture_dir: String,
}

const CURRENT_SCHEMA: &str = "planner_v2_current_batch_v1";

pub fn default_planner_capture_dir(repo_root: &Path, pack_id: &str) -> Result<PathBuf> {
    validate_id(pack_id)?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(repo_root
        .join("outputs/eval")
        .join(pack_id)
        .join(format!("capture-{stamp}")))
}

fn validate_id(id: &str) -> Result<()> {
    ensure!(
        !id.is_empty()
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')),
        "unsafe planner pack identity"
    );
    Ok(())
}

pub fn current_planner_capture(repo_root: &Path, pack_id: &str) -> Result<Option<PathBuf>> {
    validate_id(pack_id)?;
    let selection_path = repo_root
        .join("outputs/eval")
        .join(pack_id)
        .join("current.json");
    if !selection_path.try_exists()? {
        return Ok(None);
    }
    let selected: CurrentPlannerBatch = serde_json::from_slice(&fs::read(&selection_path)?)?;
    ensure!(
        selected.schema_id == CURRENT_SCHEMA && selected.pack_id == pack_id,
        "planner current selection identity mismatch"
    );
    let relative = Path::new(&selected.capture_dir);
    ensure!(
        relative.components().count() > 1
            && relative
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "unsafe planner current capture path"
    );
    let outputs = repo_root.join("outputs").canonicalize()?;
    let capture = outputs
        .join(relative)
        .canonicalize()
        .context("selected planner capture is unavailable")?;
    ensure!(
        capture.starts_with(&outputs),
        "planner current capture escaped outputs"
    );
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(capture.join("summary.json"))?)?;
    ensure!(
        saved["schema_id"] == crate::waypoint_v2_pack::WAYPOINT_V2_BATCH_SCHEMA_ID
            && saved["pack_id"] == pack_id
            && saved["status"] == "completed"
            && saved["policy_version"] == 3,
        "current planner capture is not a completed matching batch"
    );
    Ok(Some(capture))
}

/// A completed custom capture outside outputs is still usable offline but does
/// not publish itself over the registered planner's current site view.
pub fn publish_planner_batch(repo_root: &Path, capture_root: &Path) -> Result<Option<PathBuf>> {
    let capture = capture_root.canonicalize()?;
    let report = render_waypoint_v2_batch(&capture)?;
    // Explicit historical policy captures remain available offline; they must
    // not replace the current policy-3 planner evaluation entrypoint.
    if report.policy_version != 3 {
        return Ok(None);
    }
    validate_id(&report.pack_id)?;
    ensure!(
        report.status == "completed",
        "incomplete planner batch cannot be published"
    );
    ensure!(
        report.provenance.unchanged_during_capture == Some(true),
        "source or input drift during capture cannot replace current evidence"
    );
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(capture.join("summary.json"))?)?;
    let cases = saved["cases"]
        .as_array()
        .context("missing planner batch records")?;
    ensure!(!cases.is_empty(), "empty planner batch cannot be published");
    for case in cases {
        ensure!(
            case["integrity_passed"] == true
                && !matches!(
                    case["planning_stop"].as_str(),
                    Some("implementation_error" | "invalid_input")
                ),
            "untrustworthy planner batch cannot replace current evidence"
        );
        ensure!(
            case["physical_outcome"].is_null() || case["final_source_replay_passed"] == true,
            "unreplayed planner flight cannot replace current evidence"
        );
    }
    let outputs = match repo_root.join("outputs").canonicalize() {
        Ok(outputs) => outputs,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let Ok(relative) = capture.strip_prefix(&outputs) else {
        return Ok(None);
    };
    let registered_pack = repo_root
        .join("fixtures/packs")
        .join(format!("{}.json", report.pack_id));
    if !registered_pack.is_file() || !is_waypoint_v2_pack(&registered_pack)? {
        return Ok(None);
    }
    let href = format!(
        "/{}/",
        relative
            .to_str()
            .context("non-UTF8 capture path")?
            .split('/')
            .map(url_component)
            .collect::<Vec<_>>()
            .join("/")
    );
    let html = fs::read_to_string(capture.join("index.html"))?;
    ensure!(
        html.contains("<head>"),
        "planner batch report has no document head"
    );
    let site_html = html.replacen("<head>", &format!("<head>\n<base href=\"{href}\">"), 1);
    let site_dir = outputs.join("reports/eval").join(&report.pack_id);
    // Never follow a generated-site or selection directory symlink into raw
    // captures, historical evidence, or another filesystem location.
    for relative in [
        PathBuf::from("reports"),
        PathBuf::from("reports/eval"),
        PathBuf::from("reports/eval").join(&report.pack_id),
        PathBuf::from("eval"),
        PathBuf::from("eval").join(&report.pack_id),
    ] {
        let path = outputs.join(relative);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => ensure!(
                metadata.is_dir() && !metadata.file_type().is_symlink(),
                "unsafe planner publication directory {}",
                path.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    fs::create_dir_all(&site_dir)?;
    let selected = CurrentPlannerBatch {
        schema_id: CURRENT_SCHEMA.into(),
        pack_id: report.pack_id.clone(),
        capture_dir: relative.to_str().context("non-UTF8 capture path")?.into(),
    };
    let selection_dir = outputs.join("eval").join(&report.pack_id);
    fs::create_dir_all(&selection_dir)?;
    atomic_derived_write(&site_dir.join("index.html"), site_html.as_bytes())?;
    atomic_derived_write(
        &selection_dir.join("current.json"),
        &serde_json::to_vec_pretty(&selected)?,
    )?;
    crate::report_catalog::write_report_catalog(repo_root)?;
    Ok(Some(site_dir.join("index.html")))
}

fn url_component(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn atomic_derived_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temp = path.with_extension(format!("{}.{}.tmp", std::process::id(), nonce));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temp, path)
        .with_context(|| format!("publish derived planner page {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captures_are_unique_and_bad_pack_paths_fail() {
        let root = Path::new("/tmp/planner-capture-path-test");
        let first = default_planner_capture_dir(root, "planner_v2_lab_suite").unwrap();
        let second = default_planner_capture_dir(root, "planner_v2_lab_suite").unwrap();
        assert_ne!(first, second);
        assert!(first.starts_with(root.join("outputs/eval/planner_v2_lab_suite")));
        assert!(default_planner_capture_dir(root, "../terminal").is_err());
        assert!(current_planner_capture(root, "../terminal").is_err());
        assert_eq!(url_component("new capture"), "new%20capture");
    }

    #[test]
    fn unsafe_or_missing_current_selection_cannot_mutate_existing_site() {
        let root = std::env::temp_dir().join(format!(
            "pd-planner-selection-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let selection_dir = root.join("outputs/eval/planner_v2_lab_suite");
        fs::create_dir_all(&selection_dir).unwrap();
        let sentinel = root.join("outputs/index.html");
        fs::write(&sentinel, "existing report home").unwrap();
        assert!(
            current_planner_capture(&root, "planner_v2_lab_suite")
                .unwrap()
                .is_none()
        );
        for capture in ["../outside", "/tmp/outside", "eval/missing/capture"] {
            let selected = CurrentPlannerBatch {
                schema_id: CURRENT_SCHEMA.into(),
                pack_id: "planner_v2_lab_suite".into(),
                capture_dir: capture.into(),
            };
            fs::write(
                selection_dir.join("current.json"),
                serde_json::to_vec(&selected).unwrap(),
            )
            .unwrap();
            assert!(current_planner_capture(&root, "planner_v2_lab_suite").is_err());
            assert_eq!(
                fs::read_to_string(&sentinel).unwrap(),
                "existing report home"
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}
