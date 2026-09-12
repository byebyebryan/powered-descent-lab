//! Orchestration for the deterministic V2 direct-bridge setup-only report.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_plan::conservative_ballistic_bridge::{
    DirectBridgeReportV2, build_report_artifact_v2, validate_report_artifact_v2,
};
use pd_report::{
    setup::{
        write_conservative_ballistic_setup_preview_svg, write_conservative_ballistic_setup_report,
    },
    site::ReportSite,
};
use serde::Serialize;

pub const CONSERVATIVE_BALLISTIC_SETUP_ID: &str = "conservative-ballistic-direct-bridge-v2";

#[derive(Clone, Debug, Serialize)]
pub struct ConservativeBallisticReportPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
    pub report_path: PathBuf,
    pub preview_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ConservativeBallisticReportRun {
    pub report: DirectBridgeReportV2,
    pub paths: ConservativeBallisticReportPaths,
}

/// Generate the V2 direct-bridge report from its embedded frozen fixture. This function
/// writes only `summary.json`, a setup HTML page, a setup SVG preview, and the
/// report-site navigation indexes; it never starts a controller or simulator.
pub fn run_conservative_ballistic_report(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
) -> Result<ConservativeBallisticReportRun> {
    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create V2 direct-bridge setup output directory {}",
            output_dir.display()
        )
    })?;
    let report = build_report_artifact_v2();
    validate_report_artifact_v2(&report).map_err(|error| anyhow!(error))?;

    let summary_path = output_dir.join("summary.json");
    let summary_bytes = serde_json::to_vec_pretty(&report)?;
    fs::write(&summary_path, &summary_bytes).with_context(|| {
        format!(
            "failed to write V2 direct-bridge setup summary {}",
            summary_path.display()
        )
    })?;
    let reloaded = reload_report(&summary_path)?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes {
        bail!("V2 direct-bridge setup summary is not byte-stable after reload");
    }

    let site = ReportSite::new(repo_root);
    let repo_outputs = repo_root.join("outputs");
    let (report_path, update_site) = if is_repo_outputs_path(&repo_outputs, &output_dir) {
        let report_path = site.default_output_for_bundle(&output_dir).ok_or_else(|| {
            anyhow!("V2 direct-bridge setup output must be under the repository outputs")
        })?;
        (report_path, true)
    } else {
        // External output roots are self-contained: keep the raw summary at
        // the requested root and place the stable page beneath `report/`.
        (output_dir.join("report/index.html"), false)
    };
    let preview_path = report_path
        .parent()
        .expect("V2 direct-bridge report path has a parent")
        .join("preview.svg");
    write_conservative_ballistic_setup_report(&report_path, &reloaded)?;
    write_conservative_ballistic_setup_preview_svg(&preview_path, &reloaded)?;
    if update_site {
        site.update_indexes_for_file(&report_path)?;
    }

    Ok(ConservativeBallisticReportRun {
        report: reloaded,
        paths: ConservativeBallisticReportPaths {
            output_dir,
            summary_path,
            report_path,
            preview_path,
        },
    })
}

fn is_repo_outputs_path(repo_outputs: &Path, output_dir: &Path) -> bool {
    let Ok(repo_outputs) = fs::canonicalize(repo_outputs) else {
        return false;
    };
    let Ok(output_dir) = fs::canonicalize(output_dir) else {
        return false;
    };
    output_dir.starts_with(repo_outputs)
}

fn resolve_output_dir(repo_root: &Path, requested: Option<&Path>) -> PathBuf {
    requested
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                repo_root.join(path)
            }
        })
        .unwrap_or_else(|| {
            repo_root
                .join("outputs/setups")
                .join(CONSERVATIVE_BALLISTIC_SETUP_ID)
        })
}

fn reload_report(path: &Path) -> Result<DirectBridgeReportV2> {
    let raw = fs::read(path).with_context(|| {
        format!(
            "failed to read V2 direct-bridge setup summary {}",
            path.display()
        )
    })?;
    let report = serde_json::from_slice(&raw).with_context(|| {
        format!(
            "failed to parse V2 direct-bridge setup summary {}",
            path.display()
        )
    })?;
    validate_report_artifact_v2(&report).map_err(|error| anyhow!(error))?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::{
        CONSERVATIVE_BALLISTIC_SETUP_ID, run_conservative_ballistic_report,
        validate_report_artifact_v2,
    };
    use pd_plan::conservative_ballistic_bridge::{REPORT_SCHEMA_ID_V2, REPORT_SCHEMA_VERSION_V2};
    use std::{
        fs,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("pd-eval-direct-bridge-v2-{label}-{nonce}"))
    }

    #[test]
    fn setup_report_default_paths_are_reloadable_and_have_no_run_artifacts() {
        let root = temp_root("default");
        let run = run_conservative_ballistic_report(&root, None).unwrap();
        assert_eq!(
            CONSERVATIVE_BALLISTIC_SETUP_ID,
            "conservative-ballistic-direct-bridge-v2"
        );
        assert_eq!(run.report.schema_id, REPORT_SCHEMA_ID_V2);
        assert_eq!(run.report.schema_version, REPORT_SCHEMA_VERSION_V2);
        assert_eq!(
            run.report
                .fixture
                .cases
                .iter()
                .map(|case| case.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "clear_direct_probe",
                "long_span_probe",
                "ridge_probe",
                "long_range_probe",
            ]
        );
        let mut tampered = run.report.clone();
        tampered.evaluation.results[0].id.push_str("-tampered");
        assert!(validate_report_artifact_v2(&tampered).is_err());
        let mut tampered_canary = run.report.clone();
        tampered_canary.evaluation.ridge_canary.direct_status =
            pd_plan::conservative_ballistic_bridge::MissionStatusV2::Green;
        assert!(validate_report_artifact_v2(&tampered_canary).is_err());
        let mut tampered_progress = run.report.clone();
        tampered_progress
            .evaluation
            .ridge_canary
            .waypoint_search
            .selected_candidate
            .as_mut()
            .and_then(|candidate| candidate.route_progress.as_mut())
            .expect("ridge witness has route-progress evidence")
            .total_backtracking_distance_m += 1.0;
        assert!(validate_report_artifact_v2(&tampered_progress).is_err());
        assert_eq!(
            run.paths.output_dir,
            root.join("outputs/setups")
                .join(CONSERVATIVE_BALLISTIC_SETUP_ID)
        );
        assert_eq!(
            run.paths.report_path,
            root.join("outputs/reports/setups")
                .join(CONSERVATIVE_BALLISTIC_SETUP_ID)
                .join("index.html")
        );
        assert!(run.paths.summary_path.is_file());
        assert!(run.paths.report_path.is_file());
        assert!(run.paths.preview_path.is_file());
        let entries = fs::read_dir(&run.paths.output_dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(entries, vec!["summary.json"]);
        assert!(!run.paths.output_dir.join("manifest.json").exists());
        assert!(!run.paths.output_dir.join("events.json").exists());
        assert!(!run.paths.output_dir.join("samples.jsonl").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn setup_report_custom_roots_are_byte_deterministic() {
        let first = temp_root("first");
        let second = temp_root("second");
        let first_run =
            run_conservative_ballistic_report(&first, Some(Path::new("custom"))).unwrap();
        let second_run =
            run_conservative_ballistic_report(&second, Some(Path::new("custom"))).unwrap();
        assert_eq!(
            first_run.paths.report_path,
            first.join("custom/report/index.html")
        );
        assert_eq!(
            first_run.paths.preview_path,
            first.join("custom/report/preview.svg")
        );
        assert!(
            !first
                .join("outputs/reports/setups")
                .join(CONSERVATIVE_BALLISTIC_SETUP_ID)
                .join("index.html")
                .exists()
        );
        assert_eq!(
            fs::read(&first_run.paths.summary_path).unwrap(),
            fs::read(&second_run.paths.summary_path).unwrap()
        );
        assert_eq!(
            fs::read(&first_run.paths.report_path).unwrap(),
            fs::read(&second_run.paths.report_path).unwrap()
        );
        assert_eq!(
            fs::read(&first_run.paths.preview_path).unwrap(),
            fs::read(&second_run.paths.preview_path).unwrap()
        );
        let _ = fs::remove_dir_all(first);
        let _ = fs::remove_dir_all(second);
    }
}
