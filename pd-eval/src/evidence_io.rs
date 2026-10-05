//! Neutral exact-byte hashing and create-only evidence output helpers.
//!
//! These helpers preserve bytes and create-only failure boundaries without
//! imposing any path, symlink, or output-root trust policy on their callers.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use serde::Serialize;

/// SHA-256 over exact bytes, shared by evaluator evidence paths.
pub(crate) fn sha256_bytes(bytes: &[u8]) -> Result<String> {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("starting sha256sum")?;
    child
        .stdin
        .take()
        .context("opening sha256sum stdin")?
        .write_all(bytes)
        .context("writing bytes to sha256sum")?;
    let output = child.wait_with_output().context("waiting for sha256sum")?;
    if !output.status.success() {
        bail!("sha256sum failed while hashing bytes");
    }
    let digest = std::str::from_utf8(&output.stdout)?
        .split_whitespace()
        .next()
        .unwrap_or_default();
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("sha256sum returned a malformed digest");
    }
    Ok(digest.to_owned())
}

/// Create a fresh output root, including any missing parent directories.
pub(crate) fn reserve_output_root(output_dir: &Path) -> Result<()> {
    if let Some(parent) = output_dir.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(output_dir)
        .with_context(|| format!("create-only output root {}", output_dir.display()))
}

/// Serialize canonical pretty JSON and preserve the nominal writer's single
/// trailing newline and create-only artifact error context.
pub(crate) fn write_json_create_only<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("create-only artifact {}", path.display()))?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    Ok(())
}

/// Write bytes exactly as supplied. `create_context` retains the caller's
/// established open/create error label (for example, artifact or report).
pub(crate) fn write_bytes_create_only_with_context(
    path: &Path,
    bytes: &[u8],
    create_context: &str,
) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("{create_context} {}", path.display()))?;
    file.write_all(bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use serde::Serialize;

    use super::{
        reserve_output_root, sha256_bytes, write_bytes_create_only_with_context,
        write_json_create_only,
    };

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "pd-eval-evidence-io-{}-{}",
                std::process::id(),
                NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn sha256_hashes_exact_known_bytes_including_empty_input() {
        assert_eq!(
            sha256_bytes(b"").unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_bytes(b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn json_writer_emits_pretty_json_and_exactly_one_trailing_newline() {
        #[derive(Serialize)]
        struct Example {
            answer: u32,
            label: &'static str,
        }

        let temp = TestDir::new();
        let path = temp.path().join("summary.json");
        let expected = "{\n  \"answer\": 42,\n  \"label\": \"ready\"\n}\n";
        write_json_create_only(
            &path,
            &Example {
                answer: 42,
                label: "ready",
            },
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), expected);

        let error = write_json_create_only(
            &path,
            &Example {
                answer: 0,
                label: "replacement",
            },
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("create-only artifact"));
        assert_eq!(fs::read_to_string(path).unwrap(), expected);
    }

    #[test]
    fn raw_writer_adds_no_newline_and_never_replaces_existing_bytes() {
        let temp = TestDir::new();
        let path = temp.path().join("report.html");
        write_bytes_create_only_with_context(&path, b"raw bytes", "create-only report").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"raw bytes");

        let error =
            write_bytes_create_only_with_context(&path, b"replacement", "create-only report")
                .unwrap_err();
        assert!(format!("{error:#}").contains("create-only report"));
        assert_eq!(fs::read(path).unwrap(), b"raw bytes");
    }

    #[test]
    fn output_root_reservation_refuses_an_existing_root() {
        let temp = TestDir::new();
        let existing = temp.path().join("already-there");
        fs::create_dir(&existing).unwrap();
        let error = reserve_output_root(&existing).unwrap_err();
        assert!(format!("{error:#}").contains("create-only output root"));
        assert!(existing.is_dir());
    }

    #[test]
    fn output_root_reservation_does_not_replace_a_file_blocking_its_parent() {
        let temp = TestDir::new();
        let blocked_parent = temp.path().join("not-a-directory");
        fs::write(&blocked_parent, b"preserve me").unwrap();
        let requested_root = blocked_parent.join("output");

        assert!(reserve_output_root(&requested_root).is_err());
        assert_eq!(fs::read(blocked_parent).unwrap(), b"preserve me");
        assert!(!requested_root.exists());
    }

    #[test]
    fn raw_writer_preserves_create_error_context_when_parent_is_missing() {
        let temp = TestDir::new();
        let missing_parent = temp.path().join("missing");
        let error = write_bytes_create_only_with_context(
            &missing_parent.join("artifact.bin"),
            b"data",
            "create-only artifact",
        )
        .unwrap_err();
        let rendered = format!("{error:#}");
        assert!(rendered.contains("create-only artifact"));
        assert!(rendered.contains("artifact.bin"));
        assert!(!missing_parent.exists());
    }
}
