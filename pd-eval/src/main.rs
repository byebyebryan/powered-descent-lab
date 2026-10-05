use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use pd_eval::{
    BatchRegressionPolicyStatus, MissingComparePolicy,
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    compare_batch_reports, load_batch_report, preflight_waypoint_v2_flight, promote_pack_cache,
    refresh_report_outputs, report::write_batch_report_artifacts, resolve_pack_compare_baseline,
    run_pack_file_cached, write_waypoint_v2_flight,
};
use pd_plan::waypoint_v2::{WaypointV2Policy, WaypointV2Stop};

#[derive(Debug, Parser)]
#[command(name = "pd-eval")]
#[command(about = "Powered descent lab batch evaluation entry point")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run an evaluation pack; defaults to the current Planner V2 lab suite.
    RunPack(RunPackArgs),
    /// Check a saved native Planner V2 capture without rerunning it.
    CheckPlannerV2(PlannerV2CheckArgs),
    /// Publish an accepted saved native Planner V2 capture; runs no missions.
    PublishPlannerV2(PlannerV2CheckArgs),
    Report(ReportArgs),
    RefreshReports(RefreshReportsArgs),
    /// Refresh report navigation and maintained scorecard indexes only; no report bodies or flights.
    RefreshNavigation,
    PromoteCache(PromoteCacheArgs),
    /// Run the current repeated local-clearing waypoint V2 planner (policy 3 by default).
    WaypointV2Flight(WaypointV2FlightArgs),
}

#[derive(Debug, Parser)]
struct WaypointScenarioFlightArgs {
    #[arg(long, value_name = "SCENARIO_JSON")]
    scenario: PathBuf,
    #[arg(long)]
    source_pad_id: String,
    #[arg(long)]
    target_pad_id: String,
    #[arg(long, conflicts_with = "output_dir")]
    preflight_only: bool,
    #[arg(
        long,
        value_name = "NEW_OUTPUT_DIR",
        required_unless_present = "preflight_only"
    )]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct WaypointV2FlightArgs {
    #[command(flatten)]
    flight: WaypointScenarioFlightArgs,
    #[arg(long, value_parser = clap::value_parser!(u8).range(3..=3), default_value_t = 3)]
    policy_version: u8,
}

fn waypoint_v2_policy_for_version(version: u8) -> WaypointV2Policy {
    match version {
        3 => WaypointV2Policy::revision_3(),
        _ => unreachable!("clap restricts the V2 policy version"),
    }
}

#[derive(Debug, Parser)]
struct RunPackArgs {
    #[arg(value_name = "PACK_JSON")]
    pack: Option<PathBuf>,

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

    /// Retain a native Planner V2 capture without changing the current report.
    #[arg(long)]
    no_publish: bool,
}

#[derive(Debug, Parser)]
struct PlannerV2CheckArgs {
    #[arg(long, value_name = "CAPTURE_DIR")]
    dir: PathBuf,
}

#[derive(Debug, Parser)]
struct ReportArgs {
    #[arg(value_name = "BATCH_DIR")]
    dir: PathBuf,

    #[arg(long, value_name = "BASELINE_DIR", conflicts_with = "preview_dir")]
    baseline_dir: Option<PathBuf>,

    /// Create a native V2 preview beneath outputs/research; do not refresh or publish saved pages.
    #[arg(long, value_name = "NEW_DIR")]
    preview_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct RefreshReportsArgs {
    #[arg(long)]
    all: bool,
    /// Refresh home/topic/library navigation; leave report bodies and maintained scorecards alone.
    #[arg(long, conflicts_with = "all")]
    home_only: bool,
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
            let pack = args.pack.clone().unwrap_or_else(|| {
                repo_root().join(pd_eval::waypoint_v2_pack::DEFAULT_PLANNER_PACK_PATH)
            });
            if pd_eval::waypoint_v2_pack::is_waypoint_v2_pack(&pack)? {
                validate_native_planner_options(&args)?;
                let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&pack)?)?;
                let pack_id = value["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("planner pack has no identity"))?;
                let capture_dir = match args.output_dir {
                    Some(dir) => dir,
                    None => pd_eval::planner_eval_site::default_planner_capture_dir(
                        &repo_root(),
                        pack_id,
                    )?,
                };
                eprintln!(
                    "Planner V2 runs use fresh captures; Git-ref cache/baseline comparison is not applied."
                );
                let report = pd_eval::waypoint_v2_pack::run_waypoint_v2_pack(
                    &pack,
                    &capture_dir,
                    args.workers.unwrap_or_else(default_worker_count),
                )?;
                let acceptance =
                    pd_eval::waypoint_v2_acceptance::check_waypoint_v2_acceptance(&capture_dir)?;
                eprintln!("Planner acceptance: {}", acceptance.status.label());
                let published = if args.no_publish {
                    eprintln!("Publication deferred; the current accepted report is unchanged.");
                    None
                } else {
                    pd_eval::planner_eval_site::publish_planner_batch(&repo_root(), &capture_dir)?
                };
                println!("{}", serde_json::to_string_pretty(&report.summary)?);
                eprintln!(
                    "Planner batch: {}",
                    published
                        .unwrap_or_else(|| capture_dir.join("index.html"))
                        .display()
                );
                enforce_native_planner_acceptance(&acceptance, args.enforce_regression_policy)?;
                return Ok(());
            }
            validate_controller_pack_options(&args)?;
            let default_output_dir = args
                .output_dir
                .clone()
                .unwrap_or_else(|| default_eval_output_dir(&pack));
            let requested_workers = args.workers.unwrap_or_else(default_worker_count);
            if args.enforce_regression_policy
                && resolve_pack_compare_baseline(
                    &pack,
                    Some(args.compare_ref.as_str()),
                    args.baseline_dir.as_deref(),
                    MissingComparePolicy::Error,
                )?
                .is_none()
            {
                bail!("--enforce-regression-policy requires a resolved compare baseline");
            }
            let outcome = run_pack_file_cached(
                &pack,
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
        Commands::CheckPlannerV2(args) => {
            let acceptance =
                pd_eval::waypoint_v2_acceptance::check_waypoint_v2_acceptance(&args.dir)?;
            println!("{}", serde_json::to_string_pretty(&acceptance)?);
            enforce_native_planner_acceptance(&acceptance, true)?;
        }
        Commands::PublishPlannerV2(args) => {
            let acceptance =
                pd_eval::waypoint_v2_acceptance::check_waypoint_v2_acceptance(&args.dir)?;
            enforce_native_planner_acceptance(&acceptance, true)?;
            let published = require_native_planner_publication(
                pd_eval::planner_eval_site::publish_planner_batch(&repo_root(), &args.dir)?,
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema_id": "planner_v2_saved_publication_v1",
                    "published": true,
                    "report_path": published,
                    "acceptance": acceptance,
                }))?
            );
        }
        Commands::Report(args) => render_report(args)?,
        Commands::RefreshReports(args) => {
            if args.home_only {
                pd_report::site::ReportSite::new(repo_root()).refresh_home()?;
                println!(
                    "Refreshed report-home navigation, including topic indexes when configured; no report bodies or captures"
                );
            } else {
                let summary = refresh_report_outputs(args.all)?;
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Commands::RefreshNavigation => {
            pd_eval::report_catalog::write_report_catalog(&repo_root())?;
            println!(
                "Refreshed report navigation and maintained scorecard indexes; no report bodies or flights"
            );
        }
        Commands::PromoteCache(args) => {
            let promoted_dir = promote_pack_cache(
                &args.pack,
                args.source_workspace.as_deref(),
                &args.target_ref,
            )?;
            println!("{}", promoted_dir.display());
        }
        Commands::WaypointV2Flight(args) => {
            let policy_version = args.policy_version;
            let flight_args = args.flight;
            let scenario: pd_core::ScenarioSpec =
                serde_json::from_slice(&std::fs::read(&flight_args.scenario)?)?;
            let request = WaypointDirectNominalDirectGenerationRequest {
                probe_id: scenario.id.clone(),
                scenario,
                source_pad_id: flight_args.source_pad_id,
                target_pad_id: flight_args.target_pad_id,
                policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
            };
            let policy = waypoint_v2_policy_for_version(policy_version);
            if flight_args.preflight_only {
                let preflight = preflight_waypoint_v2_flight(&request, &policy);
                println!("{}", serde_json::to_string_pretty(&preflight)?);
                if !preflight.supported {
                    bail!("waypoint V2 input is unsupported or invalid");
                }
            } else {
                let output = flight_args
                    .output_dir
                    .as_deref()
                    .expect("clap requires output dir");
                let result = write_waypoint_v2_flight(&request, &policy, output)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "policy_version": policy_version,
                        "planning_stop": result.planning_stop,
                        "reason": result.reason,
                        "correction_count": result.correction_count,
                        "initial_nominal_terrain_blocked": result.initial_nominal_terrain_blocked,
                        "integrity_passed": result.integrity_passed,
                        "physical_outcome": result.physical_outcome,
                        "mission_outcome": result.mission_outcome,
                        "final_source_replay_passed": result.final_source_replay_passed,
                        "timings": result.timings,
                        "output_dir": output,
                    }))?
                );
                if matches!(
                    result.planning_stop,
                    WaypointV2Stop::Unsupported
                        | WaypointV2Stop::InvalidInput
                        | WaypointV2Stop::ImplementationError
                ) || !result.integrity_passed
                {
                    bail!(
                        "waypoint V2 stopped with a typed input or integrity failure; evidence retained at {}",
                        output.display()
                    );
                }
            }
        }
    }
    Ok(())
}

fn render_report(args: ReportArgs) -> Result<()> {
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(args.dir.join("summary.json"))?)?;
    if saved["schema_id"] == pd_eval::waypoint_v2_pack::WAYPOINT_V2_BATCH_SCHEMA_ID {
        if args.baseline_dir.is_some() {
            bail!("native Planner V2 reports do not support legacy controller batch baselines");
        }
        if let Some(preview_dir) = args.preview_dir {
            let page = pd_eval::waypoint_v2_pack::render_waypoint_v2_batch_preview(
                &args.dir,
                &preview_dir,
            )?;
            println!("{}", page.display());
            return Ok(());
        }
        let report = pd_eval::waypoint_v2_pack::validated_waypoint_v2_batch(&args.dir)?;
        if pd_eval::planner_eval_site::current_planner_capture(&repo_root(), &report.pack_id)?
            .is_some_and(|current| args.dir.canonicalize().is_ok_and(|dir| dir == current))
            && let Some(page) =
                pd_eval::planner_eval_site::publish_planner_batch(&repo_root(), &args.dir)?
        {
            println!("{}", page.display());
            return Ok(());
        }
        pd_eval::waypoint_v2_pack::render_waypoint_v2_batch(&args.dir)?;
        println!("{}", args.dir.join("index.html").display());
        return Ok(());
    }
    if args.preview_dir.is_some() {
        bail!("--preview-dir is supported only for native Planner V2 batches");
    }
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

fn validate_native_planner_options(args: &RunPackArgs) -> Result<()> {
    if args.baseline_dir.is_some()
        || !matches!(args.compare_ref.as_str(), "auto" | "none")
        || (args.compare_ref == "auto" && args.missing_compare == MissingComparePolicyArg::Error)
    {
        bail!(
            "native Planner V2 packs use fresh captures and do not support controller cache/baseline comparisons; omit baseline-dir and explicit Git refs, and use --compare-ref none when missing-compare is error"
        );
    }
    Ok(())
}

fn validate_controller_pack_options(args: &RunPackArgs) -> Result<()> {
    if args.no_publish {
        bail!(
            "--no-publish is supported only for native Planner V2 packs; no controller runs were started"
        );
    }
    Ok(())
}

fn require_native_planner_publication(published: Option<PathBuf>) -> Result<PathBuf> {
    published.ok_or_else(|| {
        anyhow::anyhow!(
            "saved Planner V2 publication was refused; the capture must be accepted and registered beneath repository outputs"
        )
    })
}

fn enforce_native_planner_acceptance(
    acceptance: &pd_eval::waypoint_v2_acceptance::WaypointV2AcceptanceV1,
    enforce: bool,
) -> Result<()> {
    if !enforce || acceptance.passed {
        return Ok(());
    }
    let reasons = acceptance
        .issues
        .iter()
        .take(3)
        .map(|issue| {
            issue.case_id.as_ref().map_or_else(
                || issue.message.clone(),
                |case_id| format!("{case_id}: {}", issue.message),
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    bail!(
        "Planner V2 acceptance {} with {} issue(s){}",
        acceptance.status.label(),
        acceptance.issues.len(),
        if reasons.is_empty() {
            String::new()
        } else {
            format!(": {reasons}")
        }
    )
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pd-eval crate should live under repo root")
        .to_path_buf()
}

#[cfg(test)]
mod direct_generation_cli_tests {
    use super::*;

    #[test]
    fn native_batch_preview_is_explicit_and_conflicts_with_baseline() {
        assert!(matches!(
            Cli::try_parse_from(["pd-eval", "report", "capture", "--preview-dir", "preview"])
                .unwrap()
                .command,
            Commands::Report(ReportArgs {
                preview_dir: Some(_),
                baseline_dir: None,
                ..
            })
        ));
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "report",
                "capture",
                "--preview-dir",
                "preview",
                "--baseline-dir",
                "baseline"
            ])
            .is_err()
        );
    }

    #[test]
    fn home_only_refresh_is_explicit_and_cannot_expand_to_all_reports() {
        assert!(matches!(
            Cli::try_parse_from(["pd-eval", "refresh-navigation"])
                .unwrap()
                .command,
            Commands::RefreshNavigation
        ));
        assert!(Cli::try_parse_from(["pd-eval", "refresh-navigation", "--all"]).is_err());
        assert!(matches!(
            Cli::try_parse_from(["pd-eval", "refresh-reports", "--home-only"])
                .unwrap()
                .command,
            Commands::RefreshReports(RefreshReportsArgs {
                home_only: true,
                all: false
            })
        ));
        assert!(
            Cli::try_parse_from(["pd-eval", "refresh-reports", "--home-only", "--all"]).is_err()
        );
        assert!(matches!(
            Cli::try_parse_from(["pd-eval", "refresh-reports", "--all"])
                .unwrap()
                .command,
            Commands::RefreshReports(RefreshReportsArgs {
                home_only: false,
                all: true
            })
        ));
    }

    #[test]
    fn retired_frontdoors_are_absent_and_current_commands_remain() {
        use clap::CommandFactory;

        let registry = Cli::command();
        assert_eq!(
            registry
                .get_subcommands()
                .map(|command| command.get_name())
                .collect::<Vec<_>>(),
            [
                "run-pack",
                "check-planner-v2",
                "publish-planner-v2",
                "report",
                "refresh-reports",
                "refresh-navigation",
                "promote-cache",
                "waypoint-v2-flight",
            ]
        );
        for command in [
            "nominal-direct-flight",
            "waypoint-v2-report",
            "waypoint-v2-nominal-characterization",
            "waypoint-direct-generation",
            "bounded-trajectory-physical",
            "controller-shadow",
            "conservative-ballistic-report",
            "final-landing-audit",
        ] {
            assert!(
                registry.find_subcommand(command).is_none(),
                "retired command {command} should not exist in the registry"
            );
        }

        assert!(Cli::try_parse_from(["pd-eval", "run-pack"]).is_ok());
        assert!(Cli::try_parse_from(["pd-eval", "report", "capture"]).is_ok());
        assert!(Cli::try_parse_from(["pd-eval", "check-planner-v2", "--dir", "capture"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-v2-flight",
                "--scenario",
                "scenario.json",
                "--source-pad-id",
                "source",
                "--target-pad-id",
                "target",
                "--preflight-only"
            ])
            .is_ok()
        );
    }

    #[test]
    fn waypoint_v2_flight_has_v2_only_version_selection_and_preflight_contract() {
        let flight = [
            "pd-eval",
            "waypoint-v2-flight",
            "--scenario",
            "scenario.json",
            "--source-pad-id",
            "pad_source",
            "--target-pad-id",
            "pad_main",
        ];
        assert!(Cli::try_parse_from(flight).is_err());
        assert!(Cli::try_parse_from(flight.into_iter().chain(["--preflight-only"])).is_ok());
        let default = Cli::try_parse_from(flight.into_iter().chain(["--output-dir", "new"]))
            .expect("default version with a fresh output path");
        match default.command {
            Commands::WaypointV2Flight(args) => assert_eq!(args.policy_version, 3),
            _ => unreachable!("parsed V2 command"),
        }
        assert!(
            Cli::try_parse_from(flight.into_iter().chain([
                "--policy-version",
                "2",
                "--preflight-only"
            ]))
            .is_err()
        );
        let version_3 = Cli::try_parse_from(flight.into_iter().chain([
            "--policy-version",
            "3",
            "--preflight-only",
        ]))
        .expect("policy 3 is an explicit V2 version");
        match version_3.command {
            Commands::WaypointV2Flight(args) => {
                assert_eq!(args.policy_version, 3);
                assert_eq!(
                    waypoint_v2_policy_for_version(args.policy_version).policy_id,
                    "piecewise_local_clearing_v2_policy_3"
                );
            }
            _ => unreachable!("parsed V2 command"),
        }
        assert!(
            Cli::try_parse_from(flight.into_iter().chain([
                "--policy-version",
                "4",
                "--preflight-only"
            ]))
            .is_err()
        );
        assert!(
            Cli::try_parse_from(flight.into_iter().chain([
                "--preflight-only",
                "--output-dir",
                "new"
            ]))
            .is_err()
        );
    }
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
    fn run_pack_defaults_to_current_planner_and_keeps_explicit_controller_packs() {
        let Commands::RunPack(default) = Cli::try_parse_from(["pd-eval", "run-pack"])
            .unwrap()
            .command
        else {
            panic!("wrong command");
        };
        assert!(default.pack.is_none());
        assert!(validate_native_planner_options(&default).is_ok());
        let Commands::RunPack(explicit) = Cli::try_parse_from([
            "pd-eval",
            "run-pack",
            "fixtures/packs/terminal_bot_lab_suite.json",
        ])
        .unwrap()
        .command
        else {
            panic!("wrong command");
        };
        assert_eq!(
            explicit.pack.as_deref(),
            Some(std::path::Path::new(
                "fixtures/packs/terminal_bot_lab_suite.json"
            ))
        );
        for flags in [
            vec!["--baseline-dir", "old"],
            vec!["--compare-ref", "HEAD^"],
            vec!["--missing-compare", "error"],
        ] {
            let mut command = vec!["pd-eval", "run-pack"];
            command.extend(flags);
            let Commands::RunPack(parsed) = Cli::try_parse_from(command).unwrap().command else {
                panic!("wrong command");
            };
            assert!(validate_native_planner_options(&parsed).is_err());
        }
        let Commands::RunPack(checked) =
            Cli::try_parse_from(["pd-eval", "run-pack", "--enforce-regression-policy"])
                .unwrap()
                .command
        else {
            panic!("wrong command");
        };
        assert!(checked.enforce_regression_policy);
        assert!(validate_native_planner_options(&checked).is_ok());
        let Commands::RunPack(no_compare) = Cli::try_parse_from([
            "pd-eval",
            "run-pack",
            "--compare-ref",
            "none",
            "--missing-compare",
            "error",
            "--no-reuse",
        ])
        .unwrap()
        .command
        else {
            panic!("wrong command");
        };
        assert!(validate_native_planner_options(&no_compare).is_ok());
    }

    #[test]
    fn check_planner_v2_requires_an_explicit_capture_directory() {
        assert!(Cli::try_parse_from(["pd-eval", "check-planner-v2"]).is_err());
        let Commands::CheckPlannerV2(args) =
            Cli::try_parse_from(["pd-eval", "check-planner-v2", "--dir", "capture"])
                .unwrap()
                .command
        else {
            panic!("wrong command");
        };
        assert_eq!(args.dir, PathBuf::from("capture"));
    }

    #[test]
    fn native_capture_can_defer_publication_but_controller_packs_cannot() {
        let Commands::RunPack(default) = Cli::try_parse_from(["pd-eval", "run-pack"])
            .unwrap()
            .command
        else {
            panic!("wrong command");
        };
        assert!(!default.no_publish);
        assert!(validate_controller_pack_options(&default).is_ok());
        let Commands::RunPack(deferred) =
            Cli::try_parse_from(["pd-eval", "run-pack", "--no-publish"])
                .unwrap()
                .command
        else {
            panic!("wrong command");
        };
        assert!(deferred.no_publish);
        assert!(validate_native_planner_options(&deferred).is_ok());
        assert!(validate_controller_pack_options(&deferred).is_err());
    }

    #[test]
    fn saved_publication_requires_an_explicit_capture_and_a_published_result() {
        assert!(Cli::try_parse_from(["pd-eval", "publish-planner-v2"]).is_err());
        let Commands::PublishPlannerV2(args) =
            Cli::try_parse_from(["pd-eval", "publish-planner-v2", "--dir", "capture"])
                .unwrap()
                .command
        else {
            panic!("wrong command");
        };
        assert_eq!(args.dir, PathBuf::from("capture"));
        assert!(require_native_planner_publication(None).is_err());
        let report = PathBuf::from("outputs/reports/eval/planner_v2_lab_suite/index.html");
        assert_eq!(
            require_native_planner_publication(Some(report.clone())).unwrap(),
            report
        );
    }

    #[test]
    fn native_planner_acceptance_enforcement_is_opt_in_for_outcomes() {
        use pd_eval::waypoint_v2_acceptance::{
            WaypointV2AcceptanceIssue, WaypointV2AcceptanceStatus, WaypointV2AcceptanceTotals,
            WaypointV2AcceptanceV1,
        };

        let failed = WaypointV2AcceptanceV1 {
            schema_id: "planner_v2_acceptance_v1".into(),
            status: WaypointV2AcceptanceStatus::Failed,
            passed: false,
            issues: vec![WaypointV2AcceptanceIssue {
                case_id: Some("case-x".into()),
                code: "synthetic_failure".into(),
                message: "synthetic outcome failure".into(),
            }],
            totals: WaypointV2AcceptanceTotals::default(),
        };
        assert!(enforce_native_planner_acceptance(&failed, false).is_ok());
        assert!(enforce_native_planner_acceptance(&failed, true).is_err());

        let mut passed = failed;
        passed.status = WaypointV2AcceptanceStatus::Passed;
        passed.passed = true;
        passed.issues.clear();
        assert!(enforce_native_planner_acceptance(&passed, true).is_ok());
    }

    #[test]
    fn current_v2_flight_defaults_to_policy_three_and_keeps_explicit_history() {
        let common = [
            "pd-eval",
            "waypoint-v2-flight",
            "--scenario",
            "scenario.json",
            "--source-pad-id",
            "source",
            "--target-pad-id",
            "target",
            "--preflight-only",
        ];
        let Commands::WaypointV2Flight(default) = Cli::try_parse_from(common).unwrap().command
        else {
            panic!("wrong command");
        };
        assert_eq!(default.policy_version, 3);
        let mut args = common.to_vec();
        args.extend(["--policy-version", "3"]);
        let Commands::WaypointV2Flight(parsed) = Cli::try_parse_from(args).unwrap().command else {
            panic!("wrong command");
        };
        assert_eq!(parsed.policy_version, 3);
        assert_eq!(
            pd_eval::waypoint_v2_report::policy_version(&waypoint_v2_policy_for_version(
                parsed.policy_version
            ))
            .unwrap(),
            3
        );
        for version in ["1", "2"] {
            let mut args = common.to_vec();
            args.extend(["--policy-version", version]);
            assert!(Cli::try_parse_from(args).is_err());
        }
        // Saved identities do not depend on the current executable default.
        assert_eq!(
            pd_eval::waypoint_v2_report::policy_version(&WaypointV2Policy::default()).unwrap(),
            3
        );
        assert_eq!(
            pd_eval::waypoint_v2_report::policy_version(&WaypointV2Policy::revision_1()).unwrap(),
            1
        );
        assert_eq!(
            pd_eval::waypoint_v2_report::policy_version(&WaypointV2Policy::revision_2()).unwrap(),
            2
        );
    }
}
