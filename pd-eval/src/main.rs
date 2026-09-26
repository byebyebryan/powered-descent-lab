use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use pd_eval::{
    BatchRegressionPolicyStatus, MissingComparePolicy,
    WaypointDirectCompleteFlatAcceptanceInputPaths, WaypointDirectCoupledThrustAuditInputPaths,
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    WaypointDirectSourceDurationCanaryInputPaths,
    WaypointDirectSourceDurationHeldCadenceDiagnosticInputPaths, compare_batch_reports,
    compare_waypoint_direct_generation_gate_a, freeze_waypoint_direct_obstacle_discrimination,
    load_batch_report, load_waypoint_direct_generation_fresh_manifest, promote_pack_cache,
    refresh_report_outputs, report::write_batch_report_artifacts, resolve_pack_compare_baseline,
    run_candidate_replay_case, run_candidate_replay_development,
    run_conservative_ballistic_f6_controller_integration_v1,
    run_conservative_ballistic_handoff_controller_development,
    run_conservative_ballistic_handoff_development, run_conservative_ballistic_report,
    run_conservative_ballistic_ridge_f5_analytical_v1,
    run_conservative_ballistic_ridge_f5_controller_v1,
    run_conservative_ballistic_ridge_heldout_analytical_v1, run_controller_shadow,
    run_final_landing_audit, run_pack_file_cached, run_physical_executor_comparison,
    run_physical_witness_development, run_progress_interval_envelope_development_gate,
    run_route_execution_development_case, run_route_execution_development_gate,
    run_source_transition_development_case, run_source_transition_development_gate,
    run_terrain_equivalence_spike, run_waypoint_direct_characterization,
    run_waypoint_direct_complete_flat_acceptance, run_waypoint_direct_controller_comparison,
    run_waypoint_direct_coupled_thrust_audit, run_waypoint_direct_flat_candidate_closure,
    run_waypoint_direct_generation_fresh_gate, run_waypoint_direct_launch_contact_contract,
    run_waypoint_direct_launch_feasibility, run_waypoint_direct_nominal_direct_generation,
    run_waypoint_direct_nominal_plant, run_waypoint_direct_obstacle_discrimination_development,
    run_waypoint_direct_obstacle_discrimination_fresh, run_waypoint_direct_primitive_analytical,
    run_waypoint_direct_source_contact, run_waypoint_direct_source_duration_canary,
    run_waypoint_direct_source_duration_held_cadence_diagnostic,
    run_waypoint_direct_source_duration_paired_command_feasibility,
    run_waypoint_direct_topology_boundary, run_waypoint_direct_topology_sweep,
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
    waypoint_direct_known_flat_generation_request,
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

    /// Load a predeclared fresh input. Run only after Gate A and code freeze.
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
