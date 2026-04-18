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
            let report = run_pack_file(&args.pack, args.output_dir.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&report.summary)?);
        }
    }

    Ok(())
}
