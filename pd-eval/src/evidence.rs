use std::fs;
use std::path::{Path, PathBuf};

use pd_missions::sha256;
use serde::{Deserialize, Serialize};

/// Explicit source/input scope chosen by the composition application. It must
/// include its own policy source and checkpoint, as well as runtime dependencies.
/// No repository-wide scan or external checkout is assumed by this library.
#[derive(Clone, Debug, Default)]
pub struct EvidenceScope {
    pub sources: Vec<PathBuf>,
    pub inputs: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileIdentityV1 {
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSnapshotV1 {
    pub schema_version: u32,
    pub executable_sha256: String,
    pub source_files: Vec<FileIdentityV1>,
    pub input_files: Vec<FileIdentityV1>,
}

impl EvidenceScope {
    pub fn snapshot(&self) -> Result<EvidenceSnapshotV1, String> {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        Ok(EvidenceSnapshotV1 {
            schema_version: 1,
            executable_sha256: sha256(&fs::read(executable).map_err(|e| e.to_string())?),
            source_files: identities(&self.sources)?,
            input_files: identities(&self.inputs)?,
        })
    }
}

fn identities(paths: &[PathBuf]) -> Result<Vec<FileIdentityV1>, String> {
    let mut files = Vec::new();
    for path in paths {
        append_files(path, &mut files)?;
    }
    files.sort();
    files.dedup();
    files
        .into_iter()
        .map(|path| {
            Ok(FileIdentityV1 {
                path: path.to_string_lossy().into_owned(),
                sha256: sha256(&fs::read(path).map_err(|e| e.to_string())?),
            })
        })
        .collect()
}

fn append_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|e| format!("bind evidence {}: {e}", path.display()))?;
    if metadata.is_symlink() {
        return Err(format!("symlink in evidence scope: {}", path.display()));
    }
    if metadata.is_file() {
        files.push(path.to_owned());
    } else if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            append_files(&entry.map_err(|e| e.to_string())?.path(), files)?;
        }
    } else {
        return Err(format!("non-regular evidence input {}", path.display()));
    }
    Ok(())
}
