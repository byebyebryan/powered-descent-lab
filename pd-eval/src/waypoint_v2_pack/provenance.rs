use std::{fs, path::Path, process::Command};

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use super::{
    input::{ExpandedPack, resolve_pack_file, resolve_repo_file},
    model::{WaypointV2PackGroup, WaypointV2PackInput, WaypointV2SourceState},
};
use crate::evidence_io::sha256_bytes;
use pd_core::ScenarioSpec;

#[derive(Serialize)]
pub(super) struct ExpandedDigestItem<'a> {
    pub(super) case_id: &'a str,
    pub(super) source_set: &'a str,
    pub(super) source_group: &'a str,
    pub(super) group: &'a WaypointV2PackGroup,
    pub(super) family: &'a str,
    pub(super) base_case_id: &'a str,
    pub(super) source_pad_id: &'a str,
    pub(super) target_pad_id: &'a str,
    pub(super) expected_preflight: &'a Option<String>,
    pub(super) scenario: &'a ScenarioSpec,
}

fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    (output.status.success()).then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn append_rust_files(dir: &Path, repo: &Path, bytes: &mut Vec<u8>) -> Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "symlink in Rust source tree: {}",
            path.display()
        );
        if metadata.is_dir() {
            append_rust_files(&path, repo, bytes)?;
        } else if metadata.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            let relative = path.strip_prefix(repo)?.to_string_lossy();
            bytes.extend_from_slice(relative.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(&fs::read(path)?);
            bytes.push(0xff);
        }
    }
    Ok(())
}

pub(crate) fn capture_source_state(repo: &Path) -> Result<WaypointV2SourceState> {
    let mut tree_bytes = Vec::new();
    for crate_name in ["pd-eval", "pd-core", "pd-plan", "pd-report", "pd-control"] {
        let src = repo.join(crate_name).join("src");
        if src.is_dir() {
            append_rust_files(&src, repo, &mut tree_bytes)?;
        }
    }
    for relative in [
        "Cargo.toml",
        "Cargo.lock",
        "pd-eval/Cargo.toml",
        "pd-core/Cargo.toml",
        "pd-plan/Cargo.toml",
        "pd-report/Cargo.toml",
        "pd-control/Cargo.toml",
    ] {
        let path = repo.join(relative);
        ensure!(path.is_file(), "missing source identity input {relative}");
        tree_bytes.extend_from_slice(relative.as_bytes());
        tree_bytes.push(0);
        tree_bytes.extend_from_slice(&fs::read(path)?);
        tree_bytes.push(0xff);
    }
    let executable = std::env::current_exe().context("resolve evaluator executable")?;
    let executable_sha256 = sha256_bytes(&fs::read(executable)?)?;
    let dirty = git_output(repo, &["status", "--porcelain"]);
    Ok(WaypointV2SourceState {
        git_commit: git_output(repo, &["rev-parse", "HEAD"]),
        git_dirty: dirty.as_ref().map(|status| !status.is_empty()),
        rust_source_tree_sha256: sha256_bytes(&tree_bytes)?,
        executable_sha256,
    })
}

pub(super) fn input_identity_still_matches(
    repo: &Path,
    pack_path: &Path,
    expanded: &ExpandedPack,
) -> Result<bool> {
    let pack = match fs::read(resolve_pack_file(pack_path)) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(false),
    };
    if sha256_bytes(&pack)? != expanded.input_identity.pack_file_sha256 {
        return Ok(false);
    }
    for source in &expanded.definition.sources {
        let Some(expected) = expanded
            .input_identity
            .source_fixture_sha256
            .get(&source.source_id)
        else {
            return Ok(false);
        };
        let path = match resolve_repo_file(repo, &source.path) {
            Ok(path) => path,
            Err(_) => return Ok(false),
        };
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(false),
        };
        if sha256_bytes(&bytes)? != *expected {
            return Ok(false);
        }
    }
    for (path, expected) in &expanded.input_identity.source_manifest_sha256 {
        let resolved = match resolve_repo_file(repo, path) {
            Ok(path) => path,
            Err(_) => return Ok(false),
        };
        let bytes = match fs::read(resolved) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(false),
        };
        if sha256_bytes(&bytes)? != *expected {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn digest_expanded_inputs(inputs: &[WaypointV2PackInput]) -> Result<String> {
    let digest_items = inputs
        .iter()
        .map(|case| ExpandedDigestItem {
            case_id: &case.case_id,
            source_set: &case.source_set,
            source_group: &case.source_group,
            group: &case.group,
            family: &case.family,
            base_case_id: &case.base_case_id,
            source_pad_id: &case.source_pad_id,
            target_pad_id: &case.target_pad_id,
            expected_preflight: &case.expected_preflight,
            scenario: &case.scenario,
        })
        .collect::<Vec<_>>();
    sha256_bytes(&serde_json::to_vec(&digest_items)?)
}
