use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use pd_eval::run_pack_file;

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
}

#[derive(Debug, Parser)]
struct RunPackArgs {
    #[arg(value_name = "PACK_JSON")]
    pack: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::RunPack(args) => {
            let default_output_dir = args
                .output_dir
                .clone()
                .unwrap_or_else(|| default_eval_output_dir(&args.pack));
            let report = run_pack_file(&args.pack, Some(default_output_dir.as_path()))?;
            println!("{}", serde_json::to_string_pretty(&report.summary)?);
        }
    }

    Ok(())
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pd-eval crate should live under repo root")
        .to_path_buf()
}

fn default_eval_output_dir(pack_path: &std::path::Path) -> PathBuf {
    repo_root()
        .join("outputs")
        .join("eval")
        .join(
            pack_path
                .file_stem()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .unwrap_or("pack"),
        )
}
