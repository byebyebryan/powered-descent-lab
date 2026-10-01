//! Read-only comparison of two completed canaries, with an optional create-only
//! comparison report. This never runs a flight or modifies a source evidence root.
use std::{fs::OpenOptions, path::PathBuf};

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let left = PathBuf::from(args.next().context("expected left canary root")?);
    let right = PathBuf::from(args.next().context("expected right canary root")?);
    let output = args.next().map(PathBuf::from);
    if args.next().is_some() {
        bail!(
            "usage: compare_local_clearing_canary_repeats LEFT_ROOT RIGHT_ROOT [NEW_REPORT_JSON]"
        );
    }
    let comparison = pd_eval::compare_local_clearing_canary_repeats(&left, &right)?;
    if let Some(output) = output {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
            .with_context(|| format!("create-only comparison report {}", output.display()))?;
        serde_json::to_writer_pretty(file, &comparison)?;
    }
    println!("{}", serde_json::to_string_pretty(&comparison)?);
    if !comparison.passed {
        bail!("complete deterministic canary repeat comparison failed");
    }
    Ok(())
}
