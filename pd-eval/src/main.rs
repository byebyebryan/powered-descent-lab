use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use pd_eval::{
    BatchRegressionPolicyStatus, BodyAwareTerminalPolicyV1, MissingComparePolicy,
    NominalDirectFlightDecisionV1, WaypointDirectCompleteFlatAcceptanceInputPaths,
    WaypointDirectCoupledThrustAuditInputPaths, WaypointDirectNominalDirectGenerationPolicyV1,
    WaypointDirectNominalDirectGenerationRequest, WaypointDirectSourceDurationCanaryInputPaths,
    WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths, compare_batch_reports,
    compare_waypoint_direct_generation_gate_a, freeze_waypoint_direct_body_aware_terminal,
    freeze_waypoint_direct_obstacle_discrimination, load_batch_report,
    load_waypoint_direct_generation_fresh_manifest, preflight_nominal_direct_contact_phase_study,
    preflight_nominal_direct_flight, preflight_nominal_direct_flight_regression,
    preflight_nominal_direct_operational_gate,
    preflight_waypoint_direct_body_aware_terminal_development,
    preflight_waypoint_direct_body_aware_terminal_freeze,
    preflight_waypoint_direct_body_aware_terminal_fresh_gate,
    preflight_waypoint_direct_terminal_admissibility, preflight_waypoint_v2_flight,
    promote_pack_cache, refresh_report_outputs, report::write_batch_report_artifacts,
    resolve_pack_compare_baseline, run_candidate_replay_case, run_candidate_replay_development,
    run_canonical_initial_direct_canary, run_conservative_ballistic_f6_controller_integration_v1,
    run_conservative_ballistic_handoff_controller_development,
    run_conservative_ballistic_handoff_development, run_conservative_ballistic_report,
    run_conservative_ballistic_ridge_f5_analytical_v1,
    run_conservative_ballistic_ridge_f5_controller_v1,
    run_conservative_ballistic_ridge_heldout_analytical_v1, run_controller_shadow,
    run_final_landing_audit, run_local_clearing_canary, run_nominal_airborne_direct_canary,
    run_nominal_direct_contact_phase_study, run_nominal_direct_flight,
    run_nominal_direct_flight_regression, run_nominal_direct_operational_flight,
    run_nominal_direct_operational_gate, run_pack_file_cached, run_physical_executor_comparison,
    run_physical_witness_development, run_progress_interval_envelope_development_gate,
    run_route_execution_development_case, run_route_execution_development_gate,
    run_source_transition_development_case, run_source_transition_development_gate,
    run_terrain_equivalence_spike, run_waypoint_direct_body_aware_terminal_development,
    run_waypoint_direct_body_aware_terminal_fresh_gate, run_waypoint_direct_characterization,
    run_waypoint_direct_complete_flat_acceptance, run_waypoint_direct_controller_comparison,
    run_waypoint_direct_coupled_thrust_audit, run_waypoint_direct_flat_candidate_closure,
    run_waypoint_direct_generation_fresh_gate, run_waypoint_direct_launch_contact_contract,
    run_waypoint_direct_launch_feasibility, run_waypoint_direct_nominal_direct_generation,
    run_waypoint_direct_nominal_plant, run_waypoint_direct_obstacle_discrimination_development,
    run_waypoint_direct_obstacle_discrimination_fresh, run_waypoint_direct_primitive_analytical,
    run_waypoint_direct_source_contact, run_waypoint_direct_source_duration_canary,
    run_waypoint_direct_source_duration_held_cadence_diagnostic,
    run_waypoint_direct_source_duration_paired_command_feasibility,
    run_waypoint_direct_terminal_admissibility, run_waypoint_direct_topology_boundary,
    run_waypoint_direct_topology_sweep, run_waypoint_v2_ground_diagnostic,
    run_waypoint_v2_nominal_characterization, run_waypoint_v2_terminal_time,
    seal_waypoint_direct_generation_code, validate_waypoint_direct_complete_flat_acceptance_inputs,
    validate_waypoint_direct_coupled_thrust_audit_inputs,
    validate_waypoint_direct_flat_candidate_closure_inputs,
    validate_waypoint_direct_launch_contact_contract_inputs,
    validate_waypoint_direct_launch_feasibility_inputs,
    validate_waypoint_direct_nominal_direct_generation_request,
    validate_waypoint_direct_source_contact_inputs,
    validate_waypoint_direct_source_duration_canary_inputs,
    validate_waypoint_direct_source_duration_held_cadence_diagnostic_inputs,
    validate_waypoint_direct_source_duration_paired_command_feasibility_inputs,
    waypoint_direct_known_flat_generation_request, write_waypoint_v2_flight,
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
    RunPack(RunPackArgs),
    Report(ReportArgs),
    RefreshReports(RefreshReportsArgs),
    /// Refresh report navigation and maintained scorecard indexes only; no report bodies or flights.
    RefreshNavigation,
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
    /// Reveal controller evidence only for F5c-eligible cases and seal the compact result.
    ConservativeBallisticF5Controller(ConservativeBallisticF5ControllerArgs),
    /// Run the opt-in F6 route application through the frozen controller and simulator.
    ConservativeBallisticF6Integration(ConservativeBallisticF6IntegrationArgs),
    /// Run the opt-in direct-route waypoint-planning characterization report.
    WaypointDirectCharacterization(WaypointDirectCharacterizationArgs),
    /// Run the controller-free V2 primitive baseline and continuous-row early gate.
    WaypointDirectPrimitiveAnalytical(WaypointDirectPrimitiveAnalyticalArgs),
    /// Run the frozen controller-free 24-cell direct and one-waypoint topology sweep.
    WaypointDirectTopologySweep(WaypointDirectTopologySweepArgs),
    /// Run the frozen controller-free direct-first boundary decision gate.
    WaypointDirectTopologyBoundary(WaypointDirectTopologyBoundaryArgs),
    /// Compare the sealed direct-only analytical sweep with the unchanged direct controller.
    WaypointDirectControllerComparison(WaypointDirectControllerComparisonArgs),
    /// Validate and characterize the sealed direct-bridge nominal plant inputs.
    WaypointDirectNominalPlant(WaypointDirectNominalPlantArgs),
    /// Measure source-pad contact and explicitly counterfactual profile continuation.
    WaypointDirectSourceContact(WaypointDirectSourceContactArgs),
    /// Run the additive fixed-protocol launch-aware direct-profile canary.
    WaypointDirectLaunchFeasibility(WaypointDirectLaunchFeasibilityArgs),
    /// Audit the exact first-contact contract for frozen launch-feasibility replays.
    WaypointDirectLaunchContactContract(WaypointDirectLaunchContactContractArgs),
    /// Close the third certified flat profile under the frozen launch and contact contract.
    WaypointDirectFlatCandidateClosure(WaypointDirectFlatCandidateClosureArgs),
    /// Audit coupled thrust on the three frozen flat candidates without a new flight.
    WaypointDirectCoupledThrustAudit(WaypointDirectCoupledThrustAuditArgs),
    /// Run the fixed 3 x 5 evaluator-only flat source-duration canary.
    WaypointDirectSourceDurationCanary(WaypointDirectSourceDurationCanaryArgs),
    /// Diagnose the frozen source-duration canary's held-command cadence effects.
    WaypointDirectSourceDurationHeldCadenceDiagnostic(
        WaypointDirectSourceDurationHeldCadenceDiagnosticArgs,
    ),
    /// Fit and replay bounded paired 60 Hz source commands against frozen handoffs.
    WaypointDirectSourceDurationPairedCommandFeasibility(
        WaypointDirectSourceDurationPairedCommandFeasibilityArgs,
    ),
    /// Accept and select complete nominal flat direct witnesses from the frozen paired-command family.
    WaypointDirectCompleteFlatAcceptance(WaypointDirectCompleteFlatAcceptanceArgs),
    /// Generate bounded nominal ballistic-direct witnesses from scenario inputs only.
    WaypointDirectGeneration(WaypointDirectGenerationArgs),
    /// Compare independently generated known-flat evidence with sealed history.
    WaypointDirectGenerationGateA(WaypointDirectGenerationGateAArgs),
    /// Freeze production inputs after the accepted compatibility gate.
    WaypointDirectGenerationFreeze(WaypointDirectGenerationFreezeArgs),
    /// Evaluate all six sealed fresh cases under the accepted frozen policy.
    WaypointDirectGenerationFreshGate(WaypointDirectGenerationFreshGateArgs),
    /// Evaluate four frozen development controls in diagnostic and generation lanes.
    WaypointDirectObstacleDevelopment(WaypointDirectObstacleDevelopmentArgs),
    /// Freeze implementation only after primary acceptance of development.
    WaypointDirectObstacleFreeze(WaypointDirectObstacleFreezeArgs),
    /// Evaluate all eight sealed obstacle cases under the accepted code freeze.
    WaypointDirectObstacleFresh(WaypointDirectObstacleFreshArgs),
    /// Audit frozen terminal references and isolate terminal-only command cadence.
    WaypointDirectTerminalAdmissibility(WaypointDirectTerminalAdmissibilityArgs),
    /// Run the fourteen-case development gate, or its input-only preflight.
    WaypointDirectBodyAwareTerminalDevelopment(WaypointDirectBodyAwareTerminalDevelopmentArgs),
    /// Freeze reviewed body-aware development evidence against exact source and inputs.
    WaypointDirectBodyAwareTerminalFreeze(WaypointDirectBodyAwareTerminalFreezeArgs),
    /// Evaluate the ten sealed body-aware terminal cases under the accepted freeze.
    WaypointDirectBodyAwareTerminalFreshGate(WaypointDirectBodyAwareTerminalFreshGateArgs),
    /// Generate, verify and fly an opt-in nominal ballistic-direct program.
    NominalDirectFlight(NominalDirectFlightArgs),
    /// Compare ordinary-run flight integration to all 24 exposed checkpoint controls.
    NominalDirectFlightRegression(NominalDirectFlightRegressionArgs),
    /// Execute an admitted nominal program under strict saved coverage, without continuation.
    NominalDirectOperationalFlight(NominalDirectFlightArgs),
    /// Run the current repeated local-clearing waypoint V2 planner (policy 3 by default).
    WaypointV2Flight(WaypointV2FlightArgs),
    /// Render saved waypoint V2 evidence only; no planning or simulation.
    WaypointV2Report(WaypointV2ReportArgs),
    /// Characterize the bounded V2 nominal plant family against retained handoffs.
    WaypointV2NominalCharacterization(WaypointV2NominalCharacterizationArgs),
    /// Compare eight proven ground entries without changing nominal acceptance.
    WaypointV2GroundDiagnostic(WaypointV2NominalCharacterizationArgs),
    /// Study two derived terminal times without changing acquisition or runtime policy.
    WaypointV2TerminalTime(WaypointV2NominalCharacterizationArgs),
    /// Validate 24 exposed controls and four sealed fresh operational missions.
    NominalDirectOperationalGate(NominalDirectOperationalGateArgs),
    /// Regenerate finite nominal segments from real airborne baseline states.
    NominalAirborneDirectCanary(NominalAirborneDirectCanaryArgs),
    /// Run sealed opt-in canonical source-rest, terrain-twin, discriminator, live-state, and endpoint gates.
    CanonicalInitialDirectCanary(CanonicalInitialDirectCanaryArgs),
    /// Run the sealed, evaluator-only one-obstruction local-clearing experiment.
    LocalClearingCanary(LocalClearingCanaryArgs),
    /// Study bounded terminal-entry height sensitivity and saved-command coverage.
    NominalDirectContactPhase(NominalDirectContactPhaseArgs),
}

#[derive(Debug, Parser)]
struct NominalAirborneDirectCanaryArgs {
    #[arg(long, value_name = "NEW_OUTPUT_DIR")]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct WaypointV2ReportArgs {
    #[arg(long, value_name = "RETAINED_SUITE_ROOT")]
    suite_root: PathBuf,
    #[arg(long, value_name = "NEW_OUTPUT_DIR")]
    output_dir: PathBuf,
    /// Optional staged single-case presentation review.
    #[arg(long)]
    case: Option<String>,
    /// Navigation preview plus one annotated rich report; uses the pinned capture only.
    #[arg(long, conflicts_with = "case")]
    rich_preview: bool,
    /// Create a navigation edition: same rich reports, one annotated case, site return links.
    #[arg(long, requires = "rich_preview", conflicts_with = "case")]
    site_navigation: bool,
}

#[derive(Debug, Parser)]
struct CanonicalInitialDirectCanaryArgs {
    #[arg(long, value_name = "NEW_OUTPUT_DIR")]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct LocalClearingCanaryArgs {
    #[arg(long, value_name = "NEW_OUTPUT_DIR")]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct NominalDirectFlightArgs {
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
    flight: NominalDirectFlightArgs,
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=3), default_value_t = 3)]
    policy_version: u8,
}

fn waypoint_v2_policy_for_version(version: u8) -> WaypointV2Policy {
    match version {
        1 => WaypointV2Policy::default(),
        2 => WaypointV2Policy::revision_2(),
        3 => WaypointV2Policy::revision_3(),
        _ => unreachable!("clap restricts the V2 policy version"),
    }
}

#[derive(Debug, Parser)]
struct WaypointV2NominalCharacterizationArgs {
    #[arg(long, value_name = "CORPUS_JSON")]
    corpus: PathBuf,
    #[arg(long, value_name = "NEW_OUTPUT_ROOT")]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct NominalDirectFlightRegressionArgs {
    #[arg(
        long,
        value_name = "ARCHIVE_ROOT",
        default_value = "outputs/research/waypoint_direct_body_aware_terminal_20260928"
    )]
    archive_root: PathBuf,
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
struct NominalDirectOperationalGateArgs {
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
struct NominalDirectContactPhaseArgs {
    #[arg(
        long,
        value_name = "INPUT_ROOT",
        default_value = "outputs/research/nominal_direct_flight_integration_20260928/gate_a"
    )]
    input_root: PathBuf,
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
struct WaypointDirectTerminalAdmissibilityArgs {
    #[arg(
        long,
        value_name = "INPUT_ROOT",
        default_value = "outputs/research/waypoint_direct_obstacle_discrimination_20260925/fresh_run_a"
    )]
    input_root: PathBuf,
    #[arg(long, conflicts_with = "output_dir")]
    preflight_only: bool,
    #[arg(
        long,
        value_name = "OUTPUT_DIR",
        required_unless_present = "preflight_only"
    )]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct WaypointDirectBodyAwareTerminalDevelopmentArgs {
    #[arg(long, conflicts_with = "output_dir")]
    preflight_only: bool,
    #[arg(
        long,
        value_name = "OUTPUT_DIR",
        required_unless_present = "preflight_only"
    )]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct WaypointDirectBodyAwareTerminalFreezeArgs {
    #[arg(long, value_name = "SUMMARY")]
    development_summary: PathBuf,
    #[arg(long, conflicts_with = "output_dir")]
    preflight_only: bool,
    #[arg(
        long,
        value_name = "OUTPUT_DIR",
        required_unless_present = "preflight_only"
    )]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct WaypointDirectBodyAwareTerminalFreshGateArgs {
    #[arg(long, value_name = "SUMMARY")]
    development_summary: PathBuf,
    #[arg(long, value_name = "SUMMARY")]
    code_freeze_summary: PathBuf,
    #[arg(long, conflicts_with = "output_dir")]
    preflight_only: bool,
    #[arg(
        long,
        value_name = "OUTPUT_DIR",
        required_unless_present = "preflight_only"
    )]
    output_dir: Option<PathBuf>,
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

#[derive(Debug, Parser)]
struct ConservativeBallisticF5ControllerArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,

    /// Immutable compact controller result. Existing contents must exact-match fresh evidence.
    #[arg(long, value_name = "RESULT_JSON")]
    result_path: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct ConservativeBallisticF6IntegrationArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,

    /// Immutable compact integration result. Existing contents must exact-match fresh evidence.
    #[arg(long, value_name = "RESULT_JSON")]
    result_path: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct WaypointDirectCharacterizationArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct WaypointDirectPrimitiveAnalyticalArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct WaypointDirectTopologySweepArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct WaypointDirectTopologyBoundaryArgs {
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct WaypointDirectControllerComparisonArgs {
    #[arg(long, value_name = "SWEEP_SUMMARY", required = true)]
    sweep_summary: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR", required = true)]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct WaypointDirectNominalPlantArgs {
    #[arg(long, value_name = "BASELINE_SUMMARY", required = true)]
    baseline_summary: PathBuf,

    #[arg(long, value_name = "SWEEP_SUMMARY", required = true)]
    sweep_summary: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR", required = true)]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct WaypointDirectSourceContactArgs {
    #[arg(long, value_name = "BASELINE_SUMMARY", required = true)]
    baseline_summary: PathBuf,

    #[arg(long, value_name = "SWEEP_SUMMARY", required = true)]
    sweep_summary: PathBuf,

    #[arg(long, value_name = "NOMINAL_PLANT_SUMMARY", required = true)]
    nominal_summary: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR", required = true)]
    output_dir: PathBuf,

    /// Validate sealed identities and profile bindings without running physics.
    #[arg(long)]
    preflight_only: bool,
}

#[derive(Debug, Parser)]
struct WaypointDirectLaunchFeasibilityArgs {
    #[arg(long, value_name = "BASELINE_SUMMARY", required = true)]
    baseline_summary: PathBuf,

    #[arg(long, value_name = "SWEEP_SUMMARY", required = true)]
    sweep_summary: PathBuf,

    #[arg(long, value_name = "NOMINAL_PLANT_SUMMARY", required = true)]
    nominal_summary: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR", required = true)]
    output_dir: PathBuf,

    /// Validate the frozen identities and launch protocol without running physics.
    #[arg(long)]
    preflight_only: bool,

    /// Run only the frozen flat gate, leaving conditional cases for review.
    #[arg(long)]
    flat_gate_only: bool,
}

#[derive(Debug, Parser)]
struct WaypointDirectLaunchContactContractArgs {
    #[arg(long, value_name = "BASELINE_SUMMARY", required = true)]
    baseline_summary: PathBuf,

    #[arg(long, value_name = "SWEEP_SUMMARY", required = true)]
    sweep_summary: PathBuf,

    #[arg(long, value_name = "NOMINAL_PLANT_SUMMARY", required = true)]
    nominal_summary: PathBuf,

    #[arg(long, value_name = "LAUNCH_SUMMARY", required = true)]
    launch_summary: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR", required = true)]
    output_dir: PathBuf,

    /// Validate all frozen identities and bindings without running physics.
    #[arg(long)]
    preflight_only: bool,
}

#[derive(Debug, Parser)]
struct WaypointDirectFlatCandidateClosureArgs {
    #[arg(long, value_name = "BASELINE_SUMMARY", required = true)]
    baseline_summary: PathBuf,

    #[arg(long, value_name = "SWEEP_SUMMARY", required = true)]
    sweep_summary: PathBuf,

    #[arg(long, value_name = "NOMINAL_PLANT_SUMMARY", required = true)]
    nominal_summary: PathBuf,

    #[arg(long, value_name = "LAUNCH_SUMMARY", required = true)]
    launch_summary: PathBuf,

    #[arg(long, value_name = "CONTACT_AUDIT_SUMMARY", required = true)]
    contact_audit_summary: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR", required = true)]
    output_dir: PathBuf,

    /// Validate all frozen identities and profile bindings without running physics.
    #[arg(long)]
    preflight_only: bool,
}

#[derive(Debug, Parser)]
struct WaypointDirectCoupledThrustAuditArgs {
    #[arg(long, value_name = "BASELINE_SUMMARY", required = true)]
    baseline_summary: PathBuf,

    #[arg(long, value_name = "SWEEP_SUMMARY", required = true)]
    sweep_summary: PathBuf,

    #[arg(long, value_name = "NOMINAL_PLANT_SUMMARY", required = true)]
    nominal_summary: PathBuf,

    #[arg(long, value_name = "LAUNCH_SUMMARY", required = true)]
    launch_summary: PathBuf,

    #[arg(long, value_name = "CONTACT_AUDIT_SUMMARY", required = true)]
    contact_audit_summary: PathBuf,

    #[arg(long, value_name = "FLAT_CANARY_SUMMARY", required = true)]
    flat_canary_summary: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR", required = true)]
    output_dir: PathBuf,

    /// Validate all frozen identities and bindings without constructing simulation state.
    #[arg(long)]
    preflight_only: bool,
}

#[derive(Debug, Parser)]
struct WaypointDirectSourceDurationCanaryArgs {
    #[arg(long, value_name = "BASELINE_SUMMARY", required = true)]
    baseline_summary: PathBuf,

    #[arg(long, value_name = "SWEEP_SUMMARY", required = true)]
    sweep_summary: PathBuf,

    #[arg(long, value_name = "NOMINAL_PLANT_SUMMARY", required = true)]
    nominal_summary: PathBuf,

    #[arg(long, value_name = "LAUNCH_SUMMARY", required = true)]
    launch_summary: PathBuf,

    #[arg(long, value_name = "CONTACT_AUDIT_SUMMARY", required = true)]
    contact_audit_summary: PathBuf,

    #[arg(long, value_name = "FLAT_CANARY_SUMMARY", required = true)]
    flat_canary_summary: PathBuf,

    #[arg(long, value_name = "COUPLED_THRUST_AUDIT_SUMMARY", required = true)]
    coupled_audit_summary: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR", required = true)]
    output_dir: PathBuf,

    /// Validate all frozen identities and the finite variant family without running physics.
    #[arg(long)]
    preflight_only: bool,
}

#[derive(Debug, Parser)]
struct WaypointDirectSourceDurationHeldCadenceDiagnosticArgs {
    #[arg(long, value_name = "SOURCE_DURATION_SUMMARY", required = true)]
    source_duration_summary: PathBuf,

    #[arg(long, value_name = "BASELINE_SUMMARY", required = true)]
    baseline_summary: PathBuf,

    #[arg(long, value_name = "SWEEP_SUMMARY", required = true)]
    sweep_summary: PathBuf,

    #[arg(long, value_name = "NOMINAL_PLANT_SUMMARY", required = true)]
    nominal_summary: PathBuf,

    #[arg(long, value_name = "LAUNCH_SUMMARY", required = true)]
    launch_summary: PathBuf,

    #[arg(long, value_name = "CONTACT_AUDIT_SUMMARY", required = true)]
    contact_audit_summary: PathBuf,

    #[arg(long, value_name = "FLAT_CANARY_SUMMARY", required = true)]
    flat_canary_summary: PathBuf,

    #[arg(long, value_name = "COUPLED_THRUST_AUDIT_SUMMARY", required = true)]
    coupled_audit_summary: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR", required = true)]
    output_dir: PathBuf,

    /// Validate the frozen source-duration summary and all upstream gates without physics.
    #[arg(long)]
    preflight_only: bool,
}

#[derive(Debug, Parser)]
struct WaypointDirectSourceDurationPairedCommandFeasibilityArgs {
    #[arg(long, value_name = "SOURCE_DURATION_SUMMARY", required = true)]
    source_duration_summary: PathBuf,

    #[arg(long, value_name = "BASELINE_SUMMARY", required = true)]
    baseline_summary: PathBuf,

    #[arg(long, value_name = "SWEEP_SUMMARY", required = true)]
    sweep_summary: PathBuf,

    #[arg(long, value_name = "NOMINAL_PLANT_SUMMARY", required = true)]
    nominal_summary: PathBuf,

    #[arg(long, value_name = "LAUNCH_SUMMARY", required = true)]
    launch_summary: PathBuf,

    #[arg(long, value_name = "CONTACT_AUDIT_SUMMARY", required = true)]
    contact_audit_summary: PathBuf,

    #[arg(long, value_name = "FLAT_CANARY_SUMMARY", required = true)]
    flat_canary_summary: PathBuf,

    #[arg(long, value_name = "COUPLED_THRUST_AUDIT_SUMMARY", required = true)]
    coupled_audit_summary: PathBuf,

    #[arg(long, value_name = "OUTPUT_DIR", required = true)]
    output_dir: PathBuf,

    /// Validate all frozen identities and family coverage without running physics.
    #[arg(long)]
    preflight_only: bool,
}

#[derive(Debug, Parser)]
struct WaypointDirectCompleteFlatAcceptanceArgs {
    #[arg(long, value_name = "PAIRED_COMMAND_SUMMARY", required = true)]
    paired_command_summary: PathBuf,

    #[command(flatten)]
    source: WaypointDirectSourceDurationPairedCommandFeasibilityArgs,
}

#[derive(Debug, Parser)]
#[command(group(clap::ArgGroup::new("input").required(true).multiple(false)
    .args(["scenario", "known_flat", "sealed_case"])))]
struct WaypointDirectGenerationArgs {
    #[arg(long, value_name = "SCENARIO_JSON")]
    scenario: Option<PathBuf>,

    /// Build only the known-flat input, without opening historical summaries.
    #[arg(long)]
    known_flat: bool,

    /// Load a predeclared fresh input for this nominal-direct command.
    #[arg(long, value_name = "CASE_ID")]
    sealed_case: Option<String>,

    #[arg(long, requires = "scenario", conflicts_with_all = ["known_flat", "sealed_case"])]
    source_pad_id: Option<String>,

    #[arg(long, requires = "scenario", conflicts_with_all = ["known_flat", "sealed_case"])]
    target_pad_id: Option<String>,

    /// Identity label only; required for a custom scenario.
    #[arg(long, requires = "scenario", conflicts_with_all = ["known_flat", "sealed_case"])]
    probe_id: Option<String>,

    #[arg(long, value_name = "POLICY_JSON", requires = "scenario", conflicts_with_all = ["known_flat", "sealed_case"])]
    policy: Option<PathBuf>,

    #[arg(
        long,
        value_name = "OUTPUT_DIR",
        required_unless_present = "preflight_only"
    )]
    output_dir: Option<PathBuf>,

    /// Validate input support only: no candidate solving, physics or writes.
    #[arg(long)]
    preflight_only: bool,
}

#[derive(Debug, Parser)]
struct WaypointDirectGenerationGateAArgs {
    #[arg(long)]
    generated_summary: PathBuf,
    #[arg(long)]
    paired_command_summary: PathBuf,
    #[arg(long)]
    acceptance_summary: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct WaypointDirectGenerationFreezeArgs {
    #[arg(long)]
    gate_a_summary: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct WaypointDirectGenerationFreshGateArgs {
    #[arg(long)]
    gate_a_summary: PathBuf,
    #[arg(long)]
    code_freeze_summary: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct WaypointDirectObstacleDevelopmentArgs {
    #[arg(long)]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct WaypointDirectObstacleFreezeArgs {
    #[arg(long)]
    development_summary: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct WaypointDirectObstacleFreshArgs {
    #[arg(long)]
    development_summary: PathBuf,
    #[arg(long)]
    code_freeze_summary: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
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
        Commands::ConservativeBallisticF5Controller(args) => {
            let run = run_conservative_ballistic_ridge_f5_controller_v1(
                &repo_root(),
                args.output_dir.as_deref(),
                args.result_path.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::ConservativeBallisticF6Integration(args) => {
            let run = run_conservative_ballistic_f6_controller_integration_v1(
                &repo_root(),
                args.output_dir.as_deref(),
                args.result_path.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::WaypointDirectCharacterization(args) => {
            let run =
                run_waypoint_direct_characterization(&repo_root(), args.output_dir.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::WaypointDirectPrimitiveAnalytical(args) => {
            let run =
                run_waypoint_direct_primitive_analytical(&repo_root(), args.output_dir.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::WaypointDirectTopologySweep(args) => {
            let run = run_waypoint_direct_topology_sweep(&repo_root(), args.output_dir.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::WaypointDirectTopologyBoundary(args) => {
            let run =
                run_waypoint_direct_topology_boundary(&repo_root(), args.output_dir.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::WaypointDirectControllerComparison(args) => {
            let run = run_waypoint_direct_controller_comparison(
                &repo_root(),
                &args.sweep_summary,
                &args.output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::WaypointDirectNominalPlant(args) => {
            let run = run_waypoint_direct_nominal_plant(
                &repo_root(),
                &args.baseline_summary,
                &args.sweep_summary,
                &args.output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&run.paths)?);
        }
        Commands::WaypointDirectSourceContact(args) => {
            if args.preflight_only {
                let gate = validate_waypoint_direct_source_contact_inputs(
                    &repo_root(),
                    &args.baseline_summary,
                    &args.sweep_summary,
                    &args.nominal_summary,
                )?;
                println!("{}", serde_json::to_string_pretty(&gate)?);
            } else {
                let run = run_waypoint_direct_source_contact(
                    &repo_root(),
                    &args.baseline_summary,
                    &args.sweep_summary,
                    &args.nominal_summary,
                    &args.output_dir,
                )?;
                println!("{}", serde_json::to_string_pretty(&run.paths)?);
            }
        }
        Commands::WaypointDirectLaunchFeasibility(args) => {
            if args.preflight_only {
                let gate = validate_waypoint_direct_launch_feasibility_inputs(
                    &repo_root(),
                    &args.baseline_summary,
                    &args.sweep_summary,
                    &args.nominal_summary,
                )?;
                println!("{}", serde_json::to_string_pretty(&gate)?);
            } else {
                let run = run_waypoint_direct_launch_feasibility(
                    &repo_root(),
                    &args.baseline_summary,
                    &args.sweep_summary,
                    &args.nominal_summary,
                    &args.output_dir,
                    args.flat_gate_only,
                )?;
                println!("{}", serde_json::to_string_pretty(&run.paths)?);
            }
        }
        Commands::WaypointDirectLaunchContactContract(args) => {
            if args.preflight_only {
                let gate = validate_waypoint_direct_launch_contact_contract_inputs(
                    &repo_root(),
                    &args.baseline_summary,
                    &args.sweep_summary,
                    &args.nominal_summary,
                    &args.launch_summary,
                )?;
                println!("{}", serde_json::to_string_pretty(&gate)?);
            } else {
                let run = run_waypoint_direct_launch_contact_contract(
                    &repo_root(),
                    &args.baseline_summary,
                    &args.sweep_summary,
                    &args.nominal_summary,
                    &args.launch_summary,
                    &args.output_dir,
                )?;
                println!("{}", serde_json::to_string_pretty(&run.paths)?);
            }
        }
        Commands::WaypointDirectFlatCandidateClosure(args) => {
            if args.preflight_only {
                let gate = validate_waypoint_direct_flat_candidate_closure_inputs(
                    &repo_root(),
                    &args.baseline_summary,
                    &args.sweep_summary,
                    &args.nominal_summary,
                    &args.launch_summary,
                    &args.contact_audit_summary,
                )?;
                println!("{}", serde_json::to_string_pretty(&gate)?);
            } else {
                let run = run_waypoint_direct_flat_candidate_closure(
                    &repo_root(),
                    &args.baseline_summary,
                    &args.sweep_summary,
                    &args.nominal_summary,
                    &args.launch_summary,
                    &args.contact_audit_summary,
                    &args.output_dir,
                )?;
                println!("{}", serde_json::to_string_pretty(&run.paths)?);
            }
        }
        Commands::WaypointDirectCoupledThrustAudit(args) => {
            let input_paths = WaypointDirectCoupledThrustAuditInputPaths {
                baseline_summary: args.baseline_summary,
                sweep_summary: args.sweep_summary,
                nominal_summary: args.nominal_summary,
                launch_summary: args.launch_summary,
                contact_audit_summary: args.contact_audit_summary,
                flat_canary_summary: args.flat_canary_summary,
            };
            if args.preflight_only {
                let gate = validate_waypoint_direct_coupled_thrust_audit_inputs(
                    &repo_root(),
                    &input_paths,
                )?;
                println!("{}", serde_json::to_string_pretty(&gate)?);
            } else {
                let run = run_waypoint_direct_coupled_thrust_audit(
                    &repo_root(),
                    &input_paths,
                    &args.output_dir,
                )?;
                println!("{}", serde_json::to_string_pretty(&run.paths)?);
            }
        }
        Commands::WaypointDirectSourceDurationCanary(args) => {
            let input_paths = WaypointDirectSourceDurationCanaryInputPaths {
                frozen_inputs: WaypointDirectCoupledThrustAuditInputPaths {
                    baseline_summary: args.baseline_summary,
                    sweep_summary: args.sweep_summary,
                    nominal_summary: args.nominal_summary,
                    launch_summary: args.launch_summary,
                    contact_audit_summary: args.contact_audit_summary,
                    flat_canary_summary: args.flat_canary_summary,
                },
                coupled_audit_summary: args.coupled_audit_summary,
            };
            if args.preflight_only {
                let gate = validate_waypoint_direct_source_duration_canary_inputs(
                    &repo_root(),
                    &input_paths,
                )?;
                println!("{}", serde_json::to_string_pretty(&gate)?);
            } else {
                let run = run_waypoint_direct_source_duration_canary(
                    &repo_root(),
                    &input_paths,
                    &args.output_dir,
                )?;
                println!("{}", serde_json::to_string_pretty(&run.paths)?);
            }
        }
        Commands::WaypointDirectSourceDurationHeldCadenceDiagnostic(args) => {
            let input_paths = WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths {
                source_duration_summary: args.source_duration_summary,
                frozen_inputs: WaypointDirectSourceDurationCanaryInputPaths {
                    frozen_inputs: WaypointDirectCoupledThrustAuditInputPaths {
                        baseline_summary: args.baseline_summary,
                        sweep_summary: args.sweep_summary,
                        nominal_summary: args.nominal_summary,
                        launch_summary: args.launch_summary,
                        contact_audit_summary: args.contact_audit_summary,
                        flat_canary_summary: args.flat_canary_summary,
                    },
                    coupled_audit_summary: args.coupled_audit_summary,
                },
            };
            if args.preflight_only {
                let gate = validate_waypoint_direct_source_duration_held_cadence_diagnostic_inputs(
                    &repo_root(),
                    &input_paths,
                )?;
                println!("{}", serde_json::to_string_pretty(&gate)?);
            } else {
                let run = run_waypoint_direct_source_duration_held_cadence_diagnostic(
                    &repo_root(),
                    &input_paths,
                    &args.output_dir,
                )?;
                println!("{}", serde_json::to_string_pretty(&run.paths)?);
            }
        }
        Commands::WaypointDirectSourceDurationPairedCommandFeasibility(args) => {
            let input_paths = WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths {
                source_duration_summary: args.source_duration_summary,
                frozen_inputs: WaypointDirectSourceDurationCanaryInputPaths {
                    frozen_inputs: WaypointDirectCoupledThrustAuditInputPaths {
                        baseline_summary: args.baseline_summary,
                        sweep_summary: args.sweep_summary,
                        nominal_summary: args.nominal_summary,
                        launch_summary: args.launch_summary,
                        contact_audit_summary: args.contact_audit_summary,
                        flat_canary_summary: args.flat_canary_summary,
                    },
                    coupled_audit_summary: args.coupled_audit_summary,
                },
            };
            if args.preflight_only {
                let gate =
                    validate_waypoint_direct_source_duration_paired_command_feasibility_inputs(
                        &repo_root(),
                        &input_paths,
                    )?;
                println!("{}", serde_json::to_string_pretty(&gate)?);
            } else {
                let run = run_waypoint_direct_source_duration_paired_command_feasibility(
                    &repo_root(),
                    &input_paths,
                    &args.output_dir,
                )?;
                println!("{}", serde_json::to_string_pretty(&run.paths)?);
            }
        }
        Commands::WaypointDirectCompleteFlatAcceptance(args) => {
            let source = args.source;
            let input_paths = WaypointDirectCompleteFlatAcceptanceInputPaths {
                paired_command_summary: args.paired_command_summary,
                source_inputs: WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths {
                    source_duration_summary: source.source_duration_summary,
                    frozen_inputs: WaypointDirectSourceDurationCanaryInputPaths {
                        frozen_inputs: WaypointDirectCoupledThrustAuditInputPaths {
                            baseline_summary: source.baseline_summary,
                            sweep_summary: source.sweep_summary,
                            nominal_summary: source.nominal_summary,
                            launch_summary: source.launch_summary,
                            contact_audit_summary: source.contact_audit_summary,
                            flat_canary_summary: source.flat_canary_summary,
                        },
                        coupled_audit_summary: source.coupled_audit_summary,
                    },
                },
            };
            if source.preflight_only {
                let gate = validate_waypoint_direct_complete_flat_acceptance_inputs(
                    &repo_root(),
                    &input_paths,
                )?;
                println!("{}", serde_json::to_string_pretty(&gate)?);
            } else {
                let run = run_waypoint_direct_complete_flat_acceptance(
                    &repo_root(),
                    &input_paths,
                    &source.output_dir,
                )?;
                println!("{}", serde_json::to_string_pretty(&run.paths)?);
            }
        }
        Commands::WaypointDirectGeneration(args) => {
            let request = if args.known_flat {
                waypoint_direct_known_flat_generation_request(&repo_root())?
            } else if let Some(case_id) = args.sealed_case {
                load_waypoint_direct_generation_fresh_manifest(&repo_root())?.request(&case_id)?
            } else {
                let scenario_path = args.scenario.expect("clap input group requires a scenario");
                let scenario = serde_json::from_slice(&std::fs::read(scenario_path)?)?;
                let policy = match args.policy {
                    Some(path) => serde_json::from_slice(&std::fs::read(path)?)?,
                    None => WaypointDirectNominalDirectGenerationPolicyV1::default(),
                };
                WaypointDirectNominalDirectGenerationRequest {
                    scenario,
                    source_pad_id: args
                        .source_pad_id
                        .unwrap_or_else(|| "pad_source".to_owned()),
                    target_pad_id: args.target_pad_id.unwrap_or_else(|| "pad_main".to_owned()),
                    probe_id: args
                        .probe_id
                        .ok_or_else(|| anyhow::anyhow!("a custom scenario requires --probe-id"))?,
                    policy,
                }
            };
            if args.preflight_only {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &validate_waypoint_direct_nominal_direct_generation_request(&request)?
                    )?
                );
            } else {
                let run = run_waypoint_direct_nominal_direct_generation(
                    &repo_root(),
                    &request,
                    args.output_dir
                        .as_deref()
                        .expect("clap requires output directory"),
                )?;
                println!("{}", serde_json::to_string_pretty(&run.paths)?);
            }
        }
        Commands::WaypointDirectGenerationGateA(args) => {
            let gate = compare_waypoint_direct_generation_gate_a(
                &args.generated_summary,
                &args.paired_command_summary,
                &args.acceptance_summary,
                &args.output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&gate)?);
        }
        Commands::WaypointDirectGenerationFreeze(args) => {
            let freeze = seal_waypoint_direct_generation_code(
                &repo_root(),
                &args.gate_a_summary,
                &args.output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&freeze)?);
        }
        Commands::WaypointDirectGenerationFreshGate(args) => {
            let gate = run_waypoint_direct_generation_fresh_gate(
                &repo_root(),
                &args.gate_a_summary,
                &args.code_freeze_summary,
                &args.output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&gate)?);
        }
        Commands::WaypointDirectObstacleDevelopment(args) => {
            let result = run_waypoint_direct_obstacle_discrimination_development(
                &repo_root(),
                &args.output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::WaypointDirectObstacleFreeze(args) => {
            let result = freeze_waypoint_direct_obstacle_discrimination(
                &repo_root(),
                &args.development_summary,
                &args.output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::WaypointDirectObstacleFresh(args) => {
            let result = run_waypoint_direct_obstacle_discrimination_fresh(
                &repo_root(),
                &args.development_summary,
                &args.code_freeze_summary,
                &args.output_dir,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::WaypointDirectTerminalAdmissibility(args) => {
            let input_root = if args.input_root.is_absolute() {
                args.input_root
            } else {
                repo_root().join(args.input_root)
            };
            if args.preflight_only {
                let result = preflight_waypoint_direct_terminal_admissibility(&input_root)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = run_waypoint_direct_terminal_admissibility(
                    &input_root,
                    args.output_dir
                        .as_deref()
                        .expect("clap requires output dir"),
                )?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Commands::WaypointDirectBodyAwareTerminalDevelopment(args) => {
            if args.preflight_only {
                let result =
                    preflight_waypoint_direct_body_aware_terminal_development(&repo_root())?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = run_waypoint_direct_body_aware_terminal_development(
                    &repo_root(),
                    args.output_dir
                        .as_deref()
                        .expect("clap requires output dir"),
                )?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Commands::WaypointDirectBodyAwareTerminalFreeze(args) => {
            if args.preflight_only {
                let result = preflight_waypoint_direct_body_aware_terminal_freeze(
                    &repo_root(),
                    &args.development_summary,
                )?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = freeze_waypoint_direct_body_aware_terminal(
                    &repo_root(),
                    &args.development_summary,
                    args.output_dir
                        .as_deref()
                        .expect("clap requires output dir"),
                )?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Commands::WaypointDirectBodyAwareTerminalFreshGate(args) => {
            if args.preflight_only {
                let result = preflight_waypoint_direct_body_aware_terminal_fresh_gate(
                    &repo_root(),
                    &args.development_summary,
                    &args.code_freeze_summary,
                )?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = run_waypoint_direct_body_aware_terminal_fresh_gate(
                    &repo_root(),
                    &args.development_summary,
                    &args.code_freeze_summary,
                    args.output_dir
                        .as_deref()
                        .expect("clap requires output dir"),
                )?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Commands::NominalDirectFlight(args) => {
            let scenario: pd_core::ScenarioSpec =
                serde_json::from_slice(&std::fs::read(&args.scenario)?)?;
            let request = WaypointDirectNominalDirectGenerationRequest {
                probe_id: scenario.id.clone(),
                scenario,
                source_pad_id: args.source_pad_id,
                target_pad_id: args.target_pad_id,
                policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
            };
            let policy = BodyAwareTerminalPolicyV1::default();
            if args.preflight_only {
                let result = preflight_nominal_direct_flight(&request, &policy);
                println!("{}", serde_json::to_string_pretty(&result)?);
                if !result.supported {
                    bail!("nominal direct-flight input is unsupported or invalid");
                }
            } else {
                let output_dir = args
                    .output_dir
                    .as_deref()
                    .expect("clap requires output dir");
                let result = run_nominal_direct_flight(&request, &policy, output_dir)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": result.decision.status(), "identity": result.identity,
                        "output_dir": output_dir,
                        "contact_physics_step": result.execution.as_ref().map(|e| e.manifest.physics_steps),
                        "contact_sim_time_s": result.execution.as_ref().map(|e| e.manifest.sim_time_s),
                        "compute": result.compute,
                        "scope": "opt-in nominal ballistic-direct flight; defaults unchanged",
                    }))?
                );
                if matches!(
                    result.decision,
                    NominalDirectFlightDecisionV1::Invalid { .. }
                        | NominalDirectFlightDecisionV1::Unsupported { .. }
                ) {
                    bail!(
                        "{} request; typed decision bundle retained at {}",
                        result.decision.status(),
                        output_dir.display()
                    );
                }
            }
        }
        Commands::NominalDirectOperationalFlight(args) => {
            let scenario: pd_core::ScenarioSpec =
                serde_json::from_slice(&std::fs::read(&args.scenario)?)?;
            let request = WaypointDirectNominalDirectGenerationRequest {
                probe_id: scenario.id.clone(),
                scenario,
                source_pad_id: args.source_pad_id,
                target_pad_id: args.target_pad_id,
                policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
            };
            let policy = BodyAwareTerminalPolicyV1::default();
            if args.preflight_only {
                let result = preflight_nominal_direct_flight(&request, &policy);
                println!("{}", serde_json::to_string_pretty(&result)?);
                if !result.supported {
                    bail!("operational input is unsupported or invalid");
                }
            } else {
                let output = args
                    .output_dir
                    .as_deref()
                    .expect("clap requires output dir");
                let result = run_nominal_direct_operational_flight(&request, &policy, output)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": result.decision.status(), "identity": result.identity,
                        "output_dir": output,
                        "operational_outcome": result.execution.as_ref().map(|execution| execution.operational_outcome),
                        "nominal_comparison": result.execution.as_ref().map(|execution| execution.nominal_comparison),
                        "admission_error": result.admission_error,
                        "scope": "strict saved coverage; no continuation/default promotion",
                    }))?
                );
                if result.admission_error.is_some()
                    || matches!(
                        result.decision,
                        NominalDirectFlightDecisionV1::Invalid { .. }
                            | NominalDirectFlightDecisionV1::Unsupported { .. }
                    )
                    || result.execution.as_ref().is_some_and(|execution| {
                        execution.operational_outcome
                            != pd_eval::NominalDirectOperationalOutcomeV1::CompletedSafeTarget
                    })
                {
                    bail!(
                        "operational completion not achieved; typed evidence retained at {}",
                        output.display()
                    );
                }
            }
        }
        Commands::WaypointV2Report(args) => {
            let receipt = if args.site_navigation {
                pd_eval::waypoint_v2_report::rich_preview::render_navigation_edition(
                    &repo_root(),
                    &args.suite_root,
                    &args.output_dir,
                )?
            } else if args.rich_preview {
                pd_eval::waypoint_v2_report::rich_preview::render_preview(
                    &repo_root(),
                    &args.suite_root,
                    &args.output_dir,
                )?
            } else {
                pd_eval::waypoint_v2_report::render_retained_suite(
                    &repo_root(),
                    &args.suite_root,
                    &args.output_dir,
                    args.case.as_deref(),
                )?
            };
            println!("{}", serde_json::to_string_pretty(&receipt)?);
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
        Commands::WaypointV2NominalCharacterization(args) => {
            let result = run_waypoint_v2_nominal_characterization(&args.corpus, &args.output_dir)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            if !result.integrity_passed {
                bail!(
                    "nominal characterization reported an integrity error; evidence retained at {}",
                    args.output_dir.display()
                );
            }
        }
        Commands::WaypointV2GroundDiagnostic(args) => {
            let result = run_waypoint_v2_ground_diagnostic(&args.corpus, &args.output_dir)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "integrity_passed": result["integrity_passed"],
                    "control_count": result["control_count"],
                    "reference_count": result["reference_count"],
                    "shadow_count": result["shadow_count"],
                    "identity": result["identity"],
                }))?
            );
            if result["integrity_passed"] != true {
                bail!(
                    "ground diagnostic integrity failure; evidence retained at {}",
                    args.output_dir.display()
                );
            }
        }
        Commands::WaypointV2TerminalTime(args) => {
            let result = run_waypoint_v2_terminal_time(&args.corpus, &args.output_dir)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "identity": result["identity"],
                    "integrity_passed": result["integrity_passed"],
                    "matched_entry_gate_passed": result["matched_entry_gate_passed"],
                    "broad_study_executed": result["broad_study_executed"],
                    "retained_rows": result["retained_rows"].as_array().map(Vec::len),
                    "synthetic_rows": result["synthetic_rows"].as_array().map(Vec::len),
                }))?
            );
            if result["integrity_passed"] != true {
                bail!(
                    "terminal-time research integrity failure; evidence retained at {}",
                    args.output_dir.display()
                );
            }
        }
        Commands::NominalDirectOperationalGate(args) => {
            if args.preflight_only {
                let result = preflight_nominal_direct_operational_gate(&repo_root())?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let output = args
                    .output_dir
                    .as_deref()
                    .expect("clap requires output dir");
                let result = run_nominal_direct_operational_gate(&repo_root(), output)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "passed": result.passed, "identity": result.identity,
                        "exposed_exact_safe_match_count": result.exposed_exact_safe_match_count,
                        "fresh_direct_safe_match_count": result.fresh_direct_safe_match_count,
                        "case_count": result.cases.len(), "output_dir": output,
                        "source_unchanged": result.source_unchanged, "archive_unchanged": result.archive_unchanged,
                        "verdict": result.verdict,
                    }))?
                );
                if !result.passed {
                    bail!(
                        "operational gate failed; all case evidence retained at {}",
                        output.display()
                    );
                }
            }
        }
        Commands::NominalAirborneDirectCanary(args) => {
            let result = run_nominal_airborne_direct_canary(&repo_root(), &args.output_dir)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            if !result.passed {
                bail!(
                    "bounded airborne regeneration compatibility failed; evidence retained at {}",
                    args.output_dir.display()
                );
            }
        }
        Commands::CanonicalInitialDirectCanary(args) => {
            let root = repo_root();
            let output_dir = if args.output_dir.is_absolute() {
                args.output_dir
            } else {
                root.join(args.output_dir)
            };
            let result = run_canonical_initial_direct_canary(&root, &output_dir)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "passed": result.passed,
                    "identity": result.identity,
                    "case_count": result.cases.len(),
                    "gates": result.gates,
                    "stop_reason": result.stop_reason,
                    "output_dir": output_dir,
                }))?
            );
            if !result.passed {
                bail!(
                    "canonical initial direct canary did not pass every declared gate; evidence retained at {}",
                    output_dir.display()
                );
            }
        }
        Commands::LocalClearingCanary(args) => {
            let root = repo_root();
            let output_dir = if args.output_dir.is_absolute() {
                args.output_dir
            } else {
                root.join(args.output_dir)
            };
            let result = run_local_clearing_canary(&root, &output_dir)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            if !result.passed {
                bail!(
                    "local-clearing canary did not pass every declared gate; evidence retained at {}",
                    output_dir.display()
                );
            }
        }
        Commands::NominalDirectFlightRegression(args) => {
            let archive_root = if args.archive_root.is_absolute() {
                args.archive_root
            } else {
                repo_root().join(args.archive_root)
            };
            if args.preflight_only {
                let result =
                    preflight_nominal_direct_flight_regression(&repo_root(), &archive_root)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let output_dir = args
                    .output_dir
                    .as_deref()
                    .expect("clap requires output dir");
                let result =
                    run_nominal_direct_flight_regression(&repo_root(), &archive_root, output_dir)?;
                let value = serde_json::to_value(&result)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "passed": value.get("passed"), "identity": value.get("identity"),
                        "verdict": value.get("verdict"), "output_dir": output_dir,
                        "case_count": value.get("rows").or_else(|| value.get("cases")).and_then(|v| v.as_array()).map(Vec::len),
                        "scope": "24 exposed exact-parity controls, not new held-out coverage",
                    }))?
                );
                if value.get("passed").and_then(|v| v.as_bool()) != Some(true) {
                    bail!(
                        "nominal direct-flight regression failed; evidence retained at {}",
                        output_dir.display()
                    );
                }
            }
        }
        Commands::NominalDirectContactPhase(args) => {
            let input_root = if args.input_root.is_absolute() {
                args.input_root
            } else {
                repo_root().join(args.input_root)
            };
            if args.preflight_only {
                let result = preflight_nominal_direct_contact_phase_study(&input_root)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
                if !result.ready {
                    bail!("nominal direct contact phase input preflight failed");
                }
            } else {
                let output_dir = args
                    .output_dir
                    .as_deref()
                    .expect("clap requires output dir");
                let result = run_nominal_direct_contact_phase_study(&input_root, output_dir)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "passed": result.artifact.passed,
                        "identity": result.artifact.identity,
                        "verdict": result.artifact.verdict,
                        "deterministic_decision": result.artifact.deterministic_decision,
                        "completed_row_count": result.artifact.completed_row_count,
                        "expected_row_count": result.artifact.expected_row_count,
                        "output_dir": output_dir,
                    }))?
                );
                if !result.artifact.passed {
                    bail!(
                        "nominal direct contact phase study is incomplete or failed its baseline gate; evidence retained at {}",
                        output_dir.display()
                    );
                }
            }
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

#[cfg(test)]
mod direct_generation_cli_tests {
    use super::*;

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
    fn v2_report_requires_explicit_source_and_fresh_output_not_flight_options() {
        assert!(Cli::try_parse_from(["pd-eval", "waypoint-v2-report"]).is_err());
        let cli = Cli::try_parse_from([
            "pd-eval",
            "waypoint-v2-report",
            "--suite-root",
            "saved",
            "--output-dir",
            "fresh",
            "--case",
            "v2_ridge_late",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::WaypointV2Report(_)));
        let preview = [
            "pd-eval",
            "waypoint-v2-report",
            "--suite-root",
            "saved",
            "--output-dir",
            "fresh",
            "--rich-preview",
        ];
        assert!(matches!(
            Cli::try_parse_from(preview.into_iter().chain(["--site-navigation"]))
                .unwrap()
                .command,
            Commands::WaypointV2Report(WaypointV2ReportArgs {
                site_navigation: true,
                rich_preview: true,
                ..
            })
        ));
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-v2-report",
                "--suite-root",
                "saved",
                "--output-dir",
                "fresh",
                "--site-navigation"
            ])
            .is_err()
        );
        assert!(matches!(
            Cli::try_parse_from(preview).unwrap().command,
            Commands::WaypointV2Report(WaypointV2ReportArgs {
                rich_preview: true,
                ..
            })
        ));
        assert!(
            Cli::try_parse_from(preview.into_iter().chain(["--case", "v2_ridge_late"])).is_err()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-v2-report",
                "--suite-root",
                "saved",
                "--output-dir",
                "fresh",
                "--policy-version",
                "3"
            ])
            .is_err()
        );
    }

    #[test]
    fn terminal_diagnostic_requires_preflight_or_create_only_output() {
        let command = ["pd-eval", "waypoint-direct-terminal-admissibility"];
        assert!(Cli::try_parse_from(command).is_err());
        assert!(Cli::try_parse_from([command[0], command[1], "--preflight-only"]).is_ok());
        assert!(Cli::try_parse_from([command[0], command[1], "--output-dir", "new"]).is_ok());
        assert!(
            Cli::try_parse_from([
                command[0],
                command[1],
                "--preflight-only",
                "--output-dir",
                "new"
            ])
            .is_err()
        );
    }

    #[test]
    fn operational_modes_require_create_only_output_or_read_only_preflight() {
        let flight = [
            "pd-eval",
            "nominal-direct-operational-flight",
            "--scenario",
            "scenario.json",
            "--source-pad-id",
            "pad_source",
            "--target-pad-id",
            "pad_main",
        ];
        assert!(Cli::try_parse_from(flight).is_err());
        assert!(Cli::try_parse_from(flight.into_iter().chain(["--preflight-only"])).is_ok());
        assert!(Cli::try_parse_from(flight.into_iter().chain(["--output-dir", "new"])).is_ok());
        assert!(
            Cli::try_parse_from(flight.into_iter().chain([
                "--preflight-only",
                "--output-dir",
                "new"
            ]))
            .is_err()
        );
        let gate = ["pd-eval", "nominal-direct-operational-gate"];
        assert!(Cli::try_parse_from(gate).is_err());
        assert!(Cli::try_parse_from(gate.into_iter().chain(["--preflight-only"])).is_ok());
        assert!(Cli::try_parse_from(gate.into_iter().chain(["--output-dir", "new"])).is_ok());
        assert!(
            Cli::try_parse_from(gate.into_iter().chain([
                "--preflight-only",
                "--output-dir",
                "new"
            ]))
            .is_err()
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
            Commands::WaypointV2Flight(args) => assert_eq!(args.policy_version, 1),
            _ => unreachable!("parsed V2 command"),
        }
        assert!(
            Cli::try_parse_from(flight.into_iter().chain([
                "--policy-version",
                "2",
                "--preflight-only",
            ]))
            .is_ok()
        );
        let version_3 = Cli::try_parse_from(flight.into_iter().chain([
            "--policy-version",
            "3",
            "--preflight-only",
        ]))
        .expect("policy 3 is an explicit opt-in V2 version");
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
                "--preflight-only",
            ]))
            .is_err()
        );
        assert!(
            Cli::try_parse_from(flight.into_iter().chain([
                "--preflight-only",
                "--output-dir",
                "new",
            ]))
            .is_err()
        );

        let mut nominal = flight;
        nominal[1] = "nominal-direct-operational-flight";
        assert!(
            Cli::try_parse_from(nominal.into_iter().chain([
                "--preflight-only",
                "--policy-version",
                "2",
            ]))
            .is_err()
        );
    }

    #[test]
    fn waypoint_v2_nominal_characterization_requires_corpus_and_new_output_root() {
        let command = ["pd-eval", "waypoint-v2-nominal-characterization"];
        assert!(Cli::try_parse_from(command).is_err());
        assert!(
            Cli::try_parse_from(command.into_iter().chain(["--corpus", "corpus.json"])).is_err()
        );
        assert!(Cli::try_parse_from(command.into_iter().chain(["--output-dir", "new"])).is_err());
        assert!(
            Cli::try_parse_from(command.into_iter().chain([
                "--corpus",
                "corpus.json",
                "--output-dir",
                "new",
            ]))
            .is_ok()
        );
    }

    #[test]
    fn waypoint_v2_ground_diagnostic_requires_corpus_and_new_output_root() {
        let command = ["pd-eval", "waypoint-v2-ground-diagnostic"];
        assert!(Cli::try_parse_from(command).is_err());
        assert!(
            Cli::try_parse_from(command.into_iter().chain(["--corpus", "corpus.json"])).is_err()
        );
        assert!(Cli::try_parse_from(command.into_iter().chain(["--output-dir", "new"])).is_err());
        let cli = Cli::try_parse_from(command.into_iter().chain([
            "--corpus",
            "corpus.json",
            "--output-dir",
            "new",
        ]))
        .unwrap();
        assert!(matches!(
            cli.command,
            Commands::WaypointV2GroundDiagnostic(_)
        ));
    }

    #[test]
    fn waypoint_v2_terminal_time_requires_sealed_corpus_and_new_output() {
        assert!(Cli::try_parse_from(["pd-eval", "waypoint-v2-terminal-time"]).is_err());
        let cli = Cli::try_parse_from([
            "pd-eval",
            "waypoint-v2-terminal-time",
            "--corpus",
            "corpus.json",
            "--output-dir",
            "new",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::WaypointV2TerminalTime(_)));
    }

    #[test]
    fn body_aware_terminal_gates_require_stage_inputs_or_preflight_only() {
        let command = ["pd-eval", "waypoint-direct-body-aware-terminal-development"];
        assert!(Cli::try_parse_from(command).is_err());
        assert!(Cli::try_parse_from([command[0], command[1], "--preflight-only"]).is_ok());
        assert!(Cli::try_parse_from([command[0], command[1], "--output-dir", "new"]).is_ok());
        assert!(
            Cli::try_parse_from([
                command[0],
                command[1],
                "--preflight-only",
                "--output-dir",
                "new"
            ])
            .is_err()
        );

        let freeze = ["pd-eval", "waypoint-direct-body-aware-terminal-freeze"];
        assert!(Cli::try_parse_from([freeze[0], freeze[1], "--preflight-only"]).is_err());
        assert!(
            Cli::try_parse_from([
                freeze[0],
                freeze[1],
                "--development-summary",
                "dev.json",
                "--preflight-only"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                freeze[0],
                freeze[1],
                "--development-summary",
                "dev.json",
                "--output-dir",
                "new"
            ])
            .is_ok()
        );

        let fresh = ["pd-eval", "waypoint-direct-body-aware-terminal-fresh-gate"];
        assert!(
            Cli::try_parse_from([
                fresh[0],
                fresh[1],
                "--development-summary",
                "dev.json",
                "--preflight-only"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                fresh[0],
                fresh[1],
                "--development-summary",
                "dev.json",
                "--code-freeze-summary",
                "freeze.json",
                "--preflight-only"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                fresh[0],
                fresh[1],
                "--development-summary",
                "dev.json",
                "--code-freeze-summary",
                "freeze.json",
                "--output-dir",
                "new"
            ])
            .is_ok()
        );
    }

    #[test]
    fn obstacle_gates_require_explicit_evidence_and_output_paths() {
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-obstacle-development",
                "--output-dir",
                "new"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-obstacle-freeze",
                "--output-dir",
                "new"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-obstacle-fresh",
                "--development-summary",
                "dev.json",
                "--output-dir",
                "new"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-obstacle-fresh",
                "--development-summary",
                "dev.json",
                "--code-freeze-summary",
                "freeze.json",
                "--output-dir",
                "new"
            ])
            .is_ok()
        );
    }

    #[test]
    fn preflight_needs_exactly_one_input_and_no_output() {
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-generation",
                "--known-flat",
                "--preflight-only",
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from(["pd-eval", "waypoint-direct-generation", "--preflight-only",])
                .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-generation",
                "--known-flat",
                "--scenario",
                "input.json",
                "--preflight-only",
            ])
            .is_err()
        );
    }

    #[test]
    fn evaluation_requires_an_explicit_create_only_output_path() {
        assert!(
            Cli::try_parse_from(["pd-eval", "waypoint-direct-generation", "--known-flat",])
                .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-generation",
                "--known-flat",
                "--output-dir",
                "new-path",
            ])
            .is_ok()
        );
    }

    #[test]
    fn factory_inputs_cannot_silently_ignore_custom_pad_options() {
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-generation",
                "--known-flat",
                "--source-pad-id",
                "custom",
                "--preflight-only",
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-generation",
                "--sealed-case",
                "fresh_flat_span_600_delta_000",
                "--preflight-only",
            ])
            .is_ok()
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
        for version in ["1", "2", "3"] {
            let mut args = common.to_vec();
            args.extend(["--policy-version", version]);
            let Commands::WaypointV2Flight(parsed) = Cli::try_parse_from(args).unwrap().command
            else {
                panic!("wrong command");
            };
            assert_eq!(parsed.policy_version.to_string(), version);
            assert_eq!(
                pd_eval::waypoint_v2_report::policy_version(&waypoint_v2_policy_for_version(
                    parsed.policy_version
                ))
                .unwrap()
                .to_string(),
                version
            );
        }
        // Frozen policy constructors retain their historical identity; only
        // user-facing selection changes, not numerical or recorded policies.
        assert_eq!(
            pd_eval::waypoint_v2_report::policy_version(&WaypointV2Policy::default()).unwrap(),
            1
        );
    }

    #[test]
    fn airborne_canary_requires_explicit_output_and_has_no_promotion_flag() {
        assert!(Cli::try_parse_from(["pd-eval", "nominal-airborne-direct-canary"]).is_err());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "nominal-airborne-direct-canary",
                "--output-dir",
                "new-root"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "nominal-airborne-direct-canary",
                "--output-dir",
                "new-root",
                "--promote"
            ])
            .is_err()
        );
    }

    #[test]
    fn canonical_initial_canary_requires_explicit_output_and_has_no_promotion_flag() {
        assert!(Cli::try_parse_from(["pd-eval", "canonical-initial-direct-canary"]).is_err());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "canonical-initial-direct-canary",
                "--output-dir",
                "new-root"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "canonical-initial-direct-canary",
                "--output-dir",
                "new-root",
                "--promote"
            ])
            .is_err()
        );
    }

    #[test]
    fn nominal_direct_flight_requires_mission_pads_and_exactly_one_output_mode() {
        let common = [
            "pd-eval",
            "nominal-direct-flight",
            "--scenario",
            "scenario.json",
            "--source-pad-id",
            "source",
            "--target-pad-id",
            "target",
        ];
        assert!(Cli::try_parse_from(common).is_err());
        let mut preflight = common.to_vec();
        preflight.push("--preflight-only");
        assert!(Cli::try_parse_from(preflight).is_ok());
        let mut flight = common.to_vec();
        flight.extend(["--output-dir", "/tmp/pd-eval-direct-cli"]);
        assert!(Cli::try_parse_from(flight.clone()).is_ok());
        flight.push("--preflight-only");
        assert!(Cli::try_parse_from(flight).is_err());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "nominal-direct-flight",
                "--scenario",
                "scenario.json",
                "--preflight-only"
            ])
            .is_err()
        );
    }

    #[test]
    fn nominal_direct_regression_requires_output_or_read_only_preflight() {
        assert!(Cli::try_parse_from(["pd-eval", "nominal-direct-flight-regression"]).is_err());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "nominal-direct-flight-regression",
                "--preflight-only"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "nominal-direct-flight-regression",
                "--output-dir",
                "/tmp/pd-eval-direct-gate"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "nominal-direct-flight-regression",
                "--output-dir",
                "/tmp/pd-eval-direct-gate",
                "--preflight-only"
            ])
            .is_err()
        );
    }

    #[test]
    fn nominal_direct_contact_phase_requires_output_or_read_only_preflight() {
        assert!(Cli::try_parse_from(["pd-eval", "nominal-direct-contact-phase"]).is_err());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "nominal-direct-contact-phase",
                "--preflight-only"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "nominal-direct-contact-phase",
                "--output-dir",
                "/tmp/pd-contact-phase-cli"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "nominal-direct-contact-phase",
                "--preflight-only",
                "--output-dir",
                "/tmp/pd-contact-phase-cli"
            ])
            .is_err()
        );
    }

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
    fn waypoint_direct_characterization_accepts_optional_output_dir() {
        assert!(Cli::try_parse_from(["pd-eval", "waypoint-direct-characterization"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-characterization",
                "--output-dir",
                "/tmp/pd-direct-characterization",
            ])
            .is_ok()
        );
    }

    #[test]
    fn waypoint_direct_primitive_analytical_accepts_optional_output_dir() {
        assert!(Cli::try_parse_from(["pd-eval", "waypoint-direct-primitive-analytical"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-primitive-analytical",
                "--output-dir",
                "/tmp/pd-direct-primitive-analytical",
            ])
            .is_ok()
        );
    }

    #[test]
    fn waypoint_direct_topology_sweep_accepts_optional_output_dir() {
        assert!(Cli::try_parse_from(["pd-eval", "waypoint-direct-topology-sweep"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-topology-sweep",
                "--output-dir",
                "/tmp/pd-direct-topology-sweep",
            ])
            .is_ok()
        );
    }

    #[test]
    fn waypoint_direct_topology_boundary_accepts_optional_output_dir() {
        assert!(Cli::try_parse_from(["pd-eval", "waypoint-direct-topology-boundary"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-topology-boundary",
                "--output-dir",
                "/tmp/pd-direct-topology-boundary",
            ])
            .is_ok()
        );
    }

    #[test]
    fn waypoint_direct_controller_comparison_requires_sealed_input_and_output_paths() {
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-controller-comparison",
                "--sweep-summary",
                "/tmp/pd-topology/summary.json",
                "--output-dir",
                "/tmp/pd-controller-comparison",
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-controller-comparison",
                "--output-dir",
                "/tmp/pd-controller-comparison",
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-controller-comparison",
                "--sweep-summary",
                "/tmp/pd-topology/summary.json",
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-source-contact",
                "--baseline-summary",
                "/tmp/pd-primitive/summary.json",
                "--sweep-summary",
                "/tmp/pd-sweep/summary.json",
                "--nominal-summary",
                "/tmp/pd-nominal/summary.json",
                "--output-dir",
                "/tmp/pd-source-contact-unused-by-preflight",
                "--preflight-only",
            ])
            .is_ok()
        );
    }

    #[test]
    fn waypoint_direct_nominal_plant_requires_both_sealed_inputs_and_output() {
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-nominal-plant",
                "--baseline-summary",
                "/tmp/pd-primitive/summary.json",
                "--sweep-summary",
                "/tmp/pd-sweep/summary.json",
                "--output-dir",
                "/tmp/pd-nominal-plant",
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-nominal-plant",
                "--sweep-summary",
                "/tmp/pd-sweep/summary.json",
                "--output-dir",
                "/tmp/pd-nominal-plant",
            ])
            .is_err()
        );
    }

    #[test]
    fn waypoint_direct_source_contact_requires_all_sealed_inputs_and_output() {
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-source-contact",
                "--baseline-summary",
                "/tmp/pd-primitive/summary.json",
                "--sweep-summary",
                "/tmp/pd-sweep/summary.json",
                "--nominal-summary",
                "/tmp/pd-nominal/summary.json",
                "--output-dir",
                "/tmp/pd-source-contact",
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-source-contact",
                "--baseline-summary",
                "/tmp/pd-primitive/summary.json",
                "--sweep-summary",
                "/tmp/pd-sweep/summary.json",
                "--nominal-summary",
                "/tmp/pd-nominal/summary.json",
            ])
            .is_err()
        );
    }

    #[test]
    fn waypoint_direct_launch_contact_contract_requires_frozen_launch_and_supports_preflight() {
        let common = [
            "pd-eval",
            "waypoint-direct-launch-contact-contract",
            "--baseline-summary",
            "/tmp/pd-primitive/summary.json",
            "--sweep-summary",
            "/tmp/pd-sweep/summary.json",
            "--nominal-summary",
            "/tmp/pd-nominal/summary.json",
            "--launch-summary",
            "/tmp/pd-launch/summary.json",
            "--output-dir",
            "/tmp/pd-contact-contract",
        ];
        assert!(Cli::try_parse_from(common).is_ok());

        let mut preflight = common.to_vec();
        preflight.push("--preflight-only");
        assert!(Cli::try_parse_from(preflight).is_ok());

        let missing_launch = [
            "pd-eval",
            "waypoint-direct-launch-contact-contract",
            "--baseline-summary",
            "/tmp/pd-primitive/summary.json",
            "--sweep-summary",
            "/tmp/pd-sweep/summary.json",
            "--nominal-summary",
            "/tmp/pd-nominal/summary.json",
            "--output-dir",
            "/tmp/pd-contact-contract",
            "--preflight-only",
        ];
        assert!(Cli::try_parse_from(missing_launch).is_err());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-launch-contact-contract",
                "--baseline-summary",
                "/tmp/pd-primitive/summary.json",
                "--sweep-summary",
                "/tmp/pd-sweep/summary.json",
                "--nominal-summary",
                "/tmp/pd-nominal/summary.json",
                "--launch-summary",
                "/tmp/pd-launch/summary.json",
            ])
            .is_err()
        );
    }

    #[test]
    fn waypoint_direct_flat_candidate_closure_requires_all_frozen_inputs_and_output() {
        let common = [
            "pd-eval",
            "waypoint-direct-flat-candidate-closure",
            "--baseline-summary",
            "/tmp/pd-primitive/summary.json",
            "--sweep-summary",
            "/tmp/pd-sweep/summary.json",
            "--nominal-summary",
            "/tmp/pd-nominal/summary.json",
            "--launch-summary",
            "/tmp/pd-launch/summary.json",
            "--contact-audit-summary",
            "/tmp/pd-contact-contract/summary.json",
            "--output-dir",
            "/tmp/pd-flat-candidate-closure",
        ];
        assert!(Cli::try_parse_from(common).is_ok());

        let mut preflight = common.to_vec();
        preflight.push("--preflight-only");
        assert!(Cli::try_parse_from(preflight).is_ok());

        let missing_contact = [
            "pd-eval",
            "waypoint-direct-flat-candidate-closure",
            "--baseline-summary",
            "/tmp/pd-primitive/summary.json",
            "--sweep-summary",
            "/tmp/pd-sweep/summary.json",
            "--nominal-summary",
            "/tmp/pd-nominal/summary.json",
            "--launch-summary",
            "/tmp/pd-launch/summary.json",
            "--output-dir",
            "/tmp/pd-flat-candidate-closure",
            "--preflight-only",
        ];
        assert!(Cli::try_parse_from(missing_contact).is_err());
    }

    #[test]
    fn waypoint_direct_coupled_thrust_audit_requires_all_frozen_inputs_and_supports_preflight() {
        let common = [
            "pd-eval",
            "waypoint-direct-coupled-thrust-audit",
            "--baseline-summary",
            "/tmp/pd-primitive/summary.json",
            "--sweep-summary",
            "/tmp/pd-sweep/summary.json",
            "--nominal-summary",
            "/tmp/pd-nominal/summary.json",
            "--launch-summary",
            "/tmp/pd-launch/summary.json",
            "--contact-audit-summary",
            "/tmp/pd-contact/summary.json",
            "--flat-canary-summary",
            "/tmp/pd-flat-canary/summary.json",
            "--output-dir",
            "/tmp/pd-coupled-thrust-audit",
        ];
        assert!(Cli::try_parse_from(common).is_ok());
        let mut preflight = common.to_vec();
        preflight.push("--preflight-only");
        assert!(Cli::try_parse_from(preflight).is_ok());

        let missing_canary = [
            "pd-eval",
            "waypoint-direct-coupled-thrust-audit",
            "--baseline-summary",
            "/tmp/pd-primitive/summary.json",
            "--sweep-summary",
            "/tmp/pd-sweep/summary.json",
            "--nominal-summary",
            "/tmp/pd-nominal/summary.json",
            "--launch-summary",
            "/tmp/pd-launch/summary.json",
            "--contact-audit-summary",
            "/tmp/pd-contact/summary.json",
            "--output-dir",
            "/tmp/pd-coupled-thrust-audit",
            "--preflight-only",
        ];
        assert!(Cli::try_parse_from(missing_canary).is_err());
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

    #[test]
    fn f5_controller_reveal_accepts_only_its_optional_evidence_paths() {
        assert!(Cli::try_parse_from(["pd-eval", "conservative-ballistic-f5-controller"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "conservative-ballistic-f5-controller",
                "--output-dir",
                "/tmp/pd-eval-f5-controller-cli-test",
                "--result-path",
                "/tmp/pd-eval-f5-controller-result-cli-test.json",
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "conservative-ballistic-f5-controller",
                "--unknown",
            ])
            .is_err()
        );
    }

    #[test]
    fn f6_integration_accepts_only_its_optional_evidence_paths() {
        assert!(Cli::try_parse_from(["pd-eval", "conservative-ballistic-f6-integration"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "conservative-ballistic-f6-integration",
                "--output-dir",
                "/tmp/pd-eval-f6-controller-cli-test",
                "--result-path",
                "/tmp/pd-eval-f6-controller-result-cli-test.json",
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "conservative-ballistic-f6-integration",
                "--unknown",
            ])
            .is_err()
        );
    }

    #[test]
    fn held_cadence_diagnostic_requires_frozen_inputs_and_accepts_preflight() {
        let required = [
            "pd-eval",
            "waypoint-direct-source-duration-held-cadence-diagnostic",
            "--source-duration-summary",
            "run_d/summary.json",
            "--baseline-summary",
            "baseline/summary.json",
            "--sweep-summary",
            "sweep/summary.json",
            "--nominal-summary",
            "nominal/summary.json",
            "--launch-summary",
            "launch/summary.json",
            "--contact-audit-summary",
            "contact/summary.json",
            "--flat-canary-summary",
            "flat/summary.json",
            "--coupled-audit-summary",
            "coupled/summary.json",
            "--output-dir",
            "diagnostic",
        ];
        assert!(Cli::try_parse_from(required).is_ok());
        let mut preflight = required.to_vec();
        preflight.push("--preflight-only");
        assert!(Cli::try_parse_from(preflight).is_ok());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-source-duration-held-cadence-diagnostic",
                "--preflight-only",
            ])
            .is_err()
        );
    }

    #[test]
    fn paired_command_feasibility_requires_frozen_inputs_and_accepts_preflight() {
        let required = [
            "pd-eval",
            "waypoint-direct-source-duration-paired-command-feasibility",
            "--source-duration-summary",
            "run_d/summary.json",
            "--baseline-summary",
            "baseline/summary.json",
            "--sweep-summary",
            "sweep/summary.json",
            "--nominal-summary",
            "nominal/summary.json",
            "--launch-summary",
            "launch/summary.json",
            "--contact-audit-summary",
            "contact/summary.json",
            "--flat-canary-summary",
            "flat/summary.json",
            "--coupled-audit-summary",
            "coupled/summary.json",
            "--output-dir",
            "paired-command",
        ];
        assert!(Cli::try_parse_from(required).is_ok());
        let mut preflight = required.to_vec();
        preflight.push("--preflight-only");
        assert!(Cli::try_parse_from(preflight).is_ok());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-source-duration-paired-command-feasibility",
                "--preflight-only",
            ])
            .is_err()
        );
    }

    #[test]
    fn complete_flat_acceptance_requires_frozen_inputs_and_accepts_preflight() {
        let required = [
            "pd-eval",
            "waypoint-direct-complete-flat-acceptance",
            "--paired-command-summary",
            "paired/run_e/summary.json",
            "--source-duration-summary",
            "source/run_d/summary.json",
            "--baseline-summary",
            "baseline/summary.json",
            "--sweep-summary",
            "sweep/summary.json",
            "--nominal-summary",
            "nominal/summary.json",
            "--launch-summary",
            "launch/summary.json",
            "--contact-audit-summary",
            "contact/summary.json",
            "--flat-canary-summary",
            "flat/summary.json",
            "--coupled-audit-summary",
            "coupled/summary.json",
            "--output-dir",
            "complete-flat",
        ];
        assert!(Cli::try_parse_from(required).is_ok());
        let mut preflight = required.to_vec();
        preflight.push("--preflight-only");
        assert!(Cli::try_parse_from(preflight).is_ok());
        let without_paired: Vec<_> = required[..2]
            .iter()
            .chain(required[4..].iter())
            .copied()
            .collect();
        assert!(Cli::try_parse_from(without_paired).is_err());
        assert!(
            Cli::try_parse_from([
                "pd-eval",
                "waypoint-direct-complete-flat-acceptance",
                "--preflight-only",
            ])
            .is_err()
        );
    }
}
