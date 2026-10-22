// Bind the sources compiled into this binary. Runtime snapshots must match this
// ledger, so rebuilding after source edits is an explicit evidence boundary.
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn append(path: &Path, files: &mut Vec<PathBuf>) {
    println!("cargo:rerun-if-changed={}", path.display());
    let meta = fs::symlink_metadata(path).expect("compiled source metadata");
    assert!(
        !meta.is_symlink(),
        "compiled evidence scope contains symlink"
    );
    if meta.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            append(&entry.unwrap().path(), files);
        }
    } else {
        assert!(meta.is_file());
        files.push(path.into());
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.parent().unwrap().parent().unwrap();
    let mut files = Vec::new();
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "examples/coast-ignite-policy/build.rs",
        "experiments/coast-ignite/pyproject.toml",
    ] {
        append(&root.join(path), &mut files);
    }
    for package in [
        "pd-core",
        "pd-control",
        "pd-run",
        "pd-missions",
        "pd-eval",
        "pd-report",
        "examples/coast-ignite-policy",
    ] {
        append(&root.join(package).join("Cargo.toml"), &mut files);
        append(&root.join(package).join("src"), &mut files);
    }
    for name in [
        "coast_ignite.py",
        "run_loop.py",
        "stopping_policy.py",
        "run_stopping.py",
        "render_stopping.py",
    ] {
        let path = root.join("experiments/coast-ignite").join(name);
        if path.exists() {
            append(&path, &mut files);
        }
    }
    let lock = root.join("experiments/coast-ignite/uv.lock");
    if lock.exists() {
        append(&lock, &mut files);
    }
    files.sort();
    files.dedup();
    let identities: Vec<_> = files
        .into_iter()
        .map(|path| {
            serde_json::json!({
                "path": path.to_string_lossy(),
                "sha256": format!("{:x}", Sha256::digest(fs::read(&path).unwrap())),
            })
        })
        .collect();
    let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("compiled-sources.json");
    fs::write(output, serde_json::to_vec(&identities).unwrap()).unwrap();
}
