use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use pd_eval::{
    load_batch_report, report::write_batch_report_artifacts, run_pack_file_with_workers,
};

#[derive(Debug, Parser)]
#[command(name = "pd-eval")]
#[command(about = "Powered descent lab batch evaluation entry point")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    RunPack(RunPackArgs),
    Report(ReportArgs),
}

#[derive(Debug, Parser)]
struct RunPackArgs {
    #[arg(value_name = "PACK_JSON")]
    pack: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,

    #[arg(long, value_name = "BASELINE_DIR")]
    baseline_dir: Option<PathBuf>,

    #[arg(long, value_name = "N")]
    workers: Option<usize>,
}

#[derive(Debug, Parser)]
struct ReportArgs {
    #[arg(value_name = "BATCH_DIR")]
    dir: PathBuf,

    #[arg(long, value_name = "BASELINE_DIR")]
    baseline_dir: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::RunPack(args) => {
            let default_output_dir = args
                .output_dir
                .clone()
                .unwrap_or_else(|| default_eval_output_dir(&args.pack));
            let requested_workers = args.workers.unwrap_or_else(default_worker_count);
            let report = run_pack_file_with_workers(
                &args.pack,
                Some(default_output_dir.as_path()),
                requested_workers,
            )?;
            let baseline_report = args
                .baseline_dir
                .as_deref()
                .map(load_batch_report)
                .transpose()?;
            write_batch_report_artifacts(
                default_output_dir.as_path(),
                &report,
                args.baseline_dir
                    .as_deref()
                    .zip(baseline_report.as_ref())
                    .map(|(dir, report)| (dir, report)),
            )?;
            println!("{}", serde_json::to_string_pretty(&report.summary)?);
        }
        Commands::Report(args) => render_report(args)?,
    }

    Ok(())
}

fn render_report(args: ReportArgs) -> Result<()> {
    let report = load_batch_report(&args.dir)?;
    let baseline_report = args
        .baseline_dir
        .as_deref()
        .map(load_batch_report)
        .transpose()?;
    write_batch_report_artifacts(
        &args.dir,
        &report,
        args.baseline_dir
            .as_deref()
            .zip(baseline_report.as_ref())
            .map(|(dir, report)| (dir, report)),
    )?;
    Ok(())
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pd-eval crate should live under repo root")
        .to_path_buf()
}

fn default_eval_output_dir(pack_path: &std::path::Path) -> PathBuf {
    repo_root().join("outputs").join("eval").join(
        pack_path
            .file_stem()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("pack"),
    )
}

fn default_worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1)
}
