//! Explicit capability diagnostic; no maintained planner/default changes.
use clap::Parser;
use pd_eval::ballistic_feedback::terminal_coast_diagnostic::{self, TerminalSetup};
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    scenario: PathBuf,
    #[arg(long)]
    feedback: PathBuf,
    #[arg(long)]
    prefix_handoff: usize,
    #[arg(long)]
    takeover_handoff: usize,
    #[arg(long, value_enum)]
    terminal_setup: TerminalSetup,
    #[arg(long)]
    output: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let result = terminal_coast_diagnostic::run(
        &args.scenario,
        &args.feedback,
        args.prefix_handoff,
        args.takeover_handoff,
        args.terminal_setup,
        &args.output,
    )?;
    println!(
        "{}",
        serde_json::json!({
            "stop": result.stop, "physical": result.final_state.physical_outcome,
            "mission": result.final_state.mission_outcome, "seconds": result.final_state.sim_time_s,
            "actual_waypoint_handoffs": result.handoffs.len(),
            "integrity": result.integrity_passed, "replay": result.source_replay_passed,
            "decisions_reproduced": result.decisions_reproduced,
        })
    );
    Ok(())
}
