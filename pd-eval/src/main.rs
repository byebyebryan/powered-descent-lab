use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use pd_eval::{
    BatchRegressionPolicyStatus, MissingComparePolicy, compare_batch_reports, load_batch_report,
    promote_pack_cache, refresh_report_outputs, report::write_batch_report_artifacts,
    resolve_pack_compare_baseline, run_candidate_replay_case, run_candidate_replay_development,
    run_conservative_ballistic_handoff_controller_development,
    run_conservative_ballistic_handoff_development, run_conservative_ballistic_report,
    run_conservative_ballistic_ridge_f5_analytical_v1,
    run_conservative_ballistic_ridge_heldout_analytical_v1, run_controller_shadow,
    run_final_landing_audit, run_pack_file_cached, run_physical_executor_comparison,
    run_physical_witness_development, run_progress_interval_envelope_development_gate,
    run_route_execution_development_case, run_route_execution_development_gate,
    run_source_transition_development_case, run_source_transition_development_gate,
    run_terrain_equivalence_spike,
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
    RefreshReports(RefreshReportsArgs),
    PromoteCache(PromoteCacheArgs),
    SourceTransitionGate(SourceTransitionGateArgs),
    RouteExecutionGate(RouteExecutionGateArgs),
    ProgressIntervalEnvelopeGate(ProgressIntervalEnvelopeGateArgs),
    TerrainEquivalenceSpike(TerrainEquivalenceSpikeArgs),
    CandidateReplay(CandidateReplayArgs),
    FinalLandingAudit(FinalLandingAuditArgs),
    /// Generate and seal the physical W4 lane before any executor artifacts
    /// are opened.
    BoundedTrajectoryPhysical(BoundedTrajectoryPhysicalArgs),
    /// Join a sealed physical W4 lane with a separately sealed R1 lane.
    PhysicalExecutorComparison(PhysicalExecutorComparisonArgs),
    /// Generate the deterministic, setup-only V2 direct-bridge analytical report.
    ConservativeBallisticReport(ConservativeBallisticReportArgs),
    /// Run the controller-free development regression for generic ridge runtime V2.
    ConservativeBallisticHandoffDevelopment(ConservativeBallisticHandoffDevelopmentArgs),
    /// Execute the development-only generic 068 controller handoff experiment.
    ConservativeBallisticHandoffControllerDevelopment(
        ConservativeBallisticHandoffControllerDevelopmentArgs,
    ),
    /// Run the evaluator-only full-controller ridge-canary shadow.
    ControllerShadow(ControllerShadowArgs),
    /// Reveal the frozen ridge held-out analytical result only; a stopped result is not H3 authority.
    ConservativeBallisticHeldoutAnalytical(ConservativeBallisticHeldoutAnalyticalArgs),
    /// Reveal the sealed F5 analytical/runtime-V2 evidence only; no controller run is performed.
    ConservativeBallisticF5Analytical(ConservativeBallisticF5AnalyticalArgs),
}

#[derive(Debug, Parser)]
struct RunPackArgs {
    #[arg(value_name = "PACK_JSON")]
    pack: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,

    #[arg(long, value_name = "BASELINE_DIR")]
    baseline_dir: Option<PathBuf>,

    #[arg(long, value_name = "REF", default_value = "auto")]
    compare_ref: String,

    #[arg(long, value_enum, default_value_t = MissingComparePolicyArg::Skip)]
    missing_compare: MissingComparePolicyArg,

    #[arg(long)]
    no_reuse: bool,

    #[arg(long, value_name = "N")]
    workers: Option<usize>,

    #[arg(long)]
    enforce_regression_policy: bool,
}

#[derive(Debug, Parser)]
struct ReportArgs {
    #[arg(value_name = "BATCH_DIR")]
    dir: PathBuf,

    #[arg(long, value_name = "BASELINE_DIR")]
    baseline_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct RefreshReportsArgs {
    #[arg(long)]
    all: bool,
}

#[derive(Debug, Parser)]
struct PromoteCacheArgs {
    #[arg(value_name = "PACK_JSON")]
    pack: PathBuf,

    #[arg(long, value_name = "WORKSPACE_KEY")]
    source_workspace: Option<String>,

    #[arg(long, value_name = "REF", default_value = "HEAD")]
    target_ref: String,
}

#[derive(Debug, Parser)]
struct SourceTransitionGateArgs {
    #[arg(
        long,
        value_name = "MANIFEST_JSON",
        default_value = "fixtures/manifests/source_transition_d0a_development.json"
    )]
    manifest: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: PathBuf,

    /// Non-authoritative single-case inspection after complete input checks.
    #[arg(long, value_name = "RUN_ID")]
    case: Option<String>,
}

#[derive(Debug, Parser)]
struct RouteExecutionGateArgs {
    #[arg(
        long,
        value_name = "MANIFEST_JSON",
        default_value = "fixtures/manifests/source_transition_d0a_development.json"
    )]
    manifest: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: PathBuf,

    /// Non-authoritative single-case inspection after complete input checks.
    #[arg(long, value_name = "RUN_ID")]
    case: Option<String>,
}

#[derive(Debug, Parser)]
struct ProgressIntervalEnvelopeGateArgs {
    /// Explicit fresh D0b route-execution evidence root.
    #[arg(long, value_name = "D0B_EVIDENCE_DIR")]
    evidence_dir: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct TerrainEquivalenceSpikeArgs {
    /// Explicit fresh D0b route-execution evidence root.
    #[arg(long, value_name = "D0B_EVIDENCE_DIR")]
    evidence_dir: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct CandidateReplayArgs {
    #[arg(
        long,
        value_name = "MANIFEST_JSON",
        default_value = "fixtures/manifests/source_transition_d0a_development.json"
    )]
    manifest: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: PathBuf,

    /// Run exactly one named case.
    #[arg(
        long,
        value_name = "RUN_ID",
        conflicts_with = "all",
        required_unless_present = "all"
    )]
    case: Option<String>,

    /// Explicitly run the complete development corpus.
    #[arg(long, conflicts_with = "case", required_unless_present = "case")]
    all: bool,
}

#[derive(Debug, Parser)]
struct FinalLandingAuditArgs {
    #[arg(
        long,
        value_name = "MANIFEST_JSON",
        default_value = "fixtures/manifests/source_transition_d0a_development.json"
    )]
    manifest: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct BoundedTrajectoryPhysicalArgs {
    #[arg(
        long,
        value_name = "MANIFEST_JSON",
        default_value = "fixtures/manifests/source_transition_d0a_development.json"
    )]
    manifest: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct PhysicalExecutorComparisonArgs {
    #[arg(long, value_name = "PHYSICAL_DIR")]
    physical_dir: PathBuf,

    #[arg(long, value_name = "EXECUTOR_DIR")]
    executor_dir: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct ConservativeBallisticReportArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct ConservativeBallisticHandoffDevelopmentArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct ConservativeBallisticHandoffControllerDevelopmentArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct ControllerShadowArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct ConservativeBallisticHeldoutAnalyticalArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,

    /// Immutable analytical result. Existing contents must exact-match the fresh result.
    #[arg(long, value_name = "RESULT_JSON")]
    result_path: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct ConservativeBallisticF5AnalyticalArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,

    /// Immutable compact analytical result. Existing contents must exact-match fresh evidence.
    #[arg(long, value_name = "RESULT_JSON")]
    result_path: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum MissingComparePolicyArg {
    Skip,
    Error,
}

impl From<MissingComparePolicyArg> for MissingComparePolicy {
    fn from(value: MissingComparePolicyArg) -> Self {
        match value {
            MissingComparePolicyArg::Skip => MissingComparePolicy::Skip,
            MissingComparePolicyArg::Error => MissingComparePolicy::Error,
        }
    }
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
            if args.enforce_regression_policy
                && resolve_pack_compare_baseline(
                    &args.pack,
                    Some(args.compare_ref.as_str()),
                    args.baseline_dir.as_deref(),
                    MissingComparePolicy::Error,
                )?
                .is_none()
            {
                bail!("--enforce-regression-policy requires a resolved compare baseline");
            }
            let outcome = run_pack_file_cached(
                &args.pack,
                Some(default_output_dir.as_path()),
                requested_workers,
                Some(args.compare_ref.as_str()),
                args.baseline_dir.as_deref(),
                args.missing_compare.into(),
                !args.no_reuse,
            )?;
            if args.enforce_regression_policy {
                let baseline = outcome.baseline.as_ref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "--enforce-regression-policy requires a resolved compare baseline"
                    )
                })?;
                let comparison = compare_batch_reports(&outcome.report, &baseline.report);
                if comparison.policy.status == BatchRegressionPolicyStatus::Fail {
                    bail!("regression policy failed: {}", comparison.policy.summary);
                }
                eprintln!("regression policy: {}", comparison.policy.summary);
            }
            println!("{}", serde_json::to_string_pretty(&outcome.report.summary)?);
        }
        Commands::Report(args) => render_report(args)?,
        Commands::RefreshReports(args) => {
            let summary = refresh_report_outputs(args.all)?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Commands::PromoteCache(args) => {
            let promoted_dir = promote_pack_cache(
                &args.pack,
                args.source_workspace.as_deref(),
                &args.target_ref,
            )?;
            println!("{}", promoted_dir.display());
        }
        Commands::SourceTransitionGate(args) => {
            let root = repo_root();
            let summary = match args.case.as_deref() {
                Some(case_id) => run_source_transition_development_case(
                    &args.manifest,
                    &root,
                    &args.output_dir,
                    case_id,
                )?,
                None => {
                    run_source_transition_development_gate(&args.manifest, &root, &args.output_dir)?
                }
            };
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Commands::RouteExecutionGate(args) => {
            let root = repo_root();
            let summary = match args.case.as_deref() {
                Some(case_id) => run_route_execution_development_case(
                    &args.manifest,
                    &root,
                    &args.output_dir,
                    case_id,
                )?,
                None => {
                    run_route_execution_development_gate(&args.manifest, &root, &args.output_dir)?
                }
            };
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Commands::ProgressIntervalEnvelopeGate(args) => {
            let summary = run_progress_interval_envelope_development_gate(
                &args.evidence_dir,
                &args.output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Commands::TerrainEquivalenceSpike(args) => {
            let artifact = run_terrain_equivalence_spike(&args.evidence_dir, &args.output_dir)?;
            println!("{}", serde_json::to_string_pretty(&artifact)?);
        }
        Commands::CandidateReplay(args) => {
            let root = repo_root();
            let summary = if args.all {
                run_candidate_replay_development(&args.manifest, &root, &args.output_dir)?
            } else {
                let case_id = args.case.as_deref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "candidate-replay requires exactly one of --case RUN_ID or --all"
                    )
                })?;
                run_candidate_replay_case(&args.manifest, &root, &args.output_dir, case_id)?
            };
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Commands::FinalLandingAudit(args) => {
            let summary = run_final_landing_audit(&args.manifest, &repo_root(), &args.output_dir)?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Commands::BoundedTrajectoryPhysical(args) => {
            let summary =
                run_physical_witness_development(&args.manifest, &repo_root(), &args.output_dir)?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Commands::PhysicalExecutorComparison(args) => {
            let summary = run_physical_executor_comparison(
                &args.physical_dir,
                &args.executor_dir,
                &args.output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Commands::ConservativeBallisticReport(args) => {
            let run = run_conservative_ballistic_report(&repo_root(), args.output_dir.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::ConservativeBallisticHandoffDevelopment(args) => {
            let run = run_conservative_ballistic_handoff_development(
                &repo_root(),
                args.output_dir.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::ConservativeBallisticHandoffControllerDevelopment(args) => {
            let run = run_conservative_ballistic_handoff_controller_development(
                &repo_root(),
                args.output_dir.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::ControllerShadow(args) => {
            let run = run_controller_shadow(&repo_root(), args.output_dir.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::ConservativeBallisticHeldoutAnalytical(args) => {
            let run = run_conservative_ballistic_ridge_heldout_analytical_v1(
                &repo_root(),
                args.output_dir.as_deref(),
                args.result_path.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::ConservativeBallisticF5Analytical(args) => {
            let run = run_conservative_ballistic_ridge_f5_analytical_v1(
                &repo_root(),
                args.output_dir.as_deref(),
                args.result_path.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
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
        args.baseline_dir.as_deref().zip(baseline_report.as_ref()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_replay_requires_exactly_one_scope_selector() {
        let common = [
            "pd-eval",
            "candidate-replay",
            "--output-dir",
            "/tmp/pd-eval-r1-cli-test",
        ];
        assert!(Cli::try_parse_from(common).is_err());

        let mut case = common.to_vec();
        case.extend(["--case", "row"]);
        assert!(Cli::try_parse_from(case).is_ok());

        let mut all = common.to_vec();
        all.push("--all");
        assert!(Cli::try_parse_from(all).is_ok());

        let mut both = common.to_vec();
        both.extend(["--case", "row", "--all"]);
        assert!(Cli::try_parse_from(both).is_err());
    }

    #[test]
    fn final_landing_audit_requires_output_and_accepts_manifest() {
        let common = [
            "pd-eval",
            "final-landing-audit",
            "--output-dir",
            "/tmp/pd-eval-cb0-cli-test",
        ];
        assert!(Cli::try_parse_from(common).is_ok());

        let with_manifest = [
            "pd-eval",
            "final-landing-audit",
            "--manifest",
            "manifest.json",
            "--output-dir",
            "/tmp/pd-eval-cb0-cli-test",
        ];
        assert!(Cli::try_parse_from(with_manifest).is_ok());
        assert!(Cli::try_parse_from(["pd-eval", "final-landing-audit"]).is_err());
    }

    #[test]
    fn physical_w4_command_has_only_input_manifest_and_output_scope() {
        let common = [
            "pd-eval",
            "bounded-trajectory-physical",
            "--output-dir",
            "/tmp/w4",
        ];
        assert!(Cli::try_parse_from(common).is_ok());

        let with_manifest = [
            "pd-eval",
            "bounded-trajectory-physical",
            "--manifest",
            "manifest.json",
            "--output-dir",
            "/tmp/w4",
        ];
        assert!(Cli::try_parse_from(with_manifest).is_ok());

        let with_executor = [
            "pd-eval",
            "bounded-trajectory-physical",
            "--executor-dir",
            "/tmp/r1",
            "--output-dir",
            "/tmp/w4",
        ];
        assert!(Cli::try_parse_from(with_executor).is_err());
    }

    #[test]
    fn physical_executor_comparison_requires_three_roots() {
        let common = [
            "pd-eval",
            "physical-executor-comparison",
            "--physical-dir",
            "/tmp/physical",
            "--executor-dir",
            "/tmp/r1",
            "--output-dir",
            "/tmp/comparison",
        ];
        assert!(Cli::try_parse_from(common).is_ok());

        for missing in [
            [
                "pd-eval",
                "physical-executor-comparison",
                "--executor-dir",
                "/tmp/r1",
                "--output-dir",
                "/tmp/comparison",
            ],
            [
                "pd-eval",
                "physical-executor-comparison",
                "--physical-dir",
                "/tmp/physical",
                "--output-dir",
                "/tmp/comparison",
            ],
            [
                "pd-eval",
                "physical-executor-comparison",
                "--physical-dir",
                "/tmp/physical",
                "--executor-dir",
                "/tmp/r1",
            ],
        ] {
            assert!(Cli::try_parse_from(missing).is_err());
        }
    }

    #[test]
    fn conservative_ballistic_report_accepts_default_and_custom_output() {
        assert!(Cli::try_parse_from(["pd-eval", "conservative-ballistic-report"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "conservative-ballistic-report",
                "--output-dir",
                "/tmp/pd-eval-direct-bridge-v2-cli-test",
            ])
            .is_ok()
        );
    }
}
