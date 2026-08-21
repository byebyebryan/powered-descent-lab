//! Research-only trajectory-tube shadow evaluator.
//!
//! The evaluator intentionally lives outside the planner contract.  It reads
//! only physical/setup fields from each persisted bundle, constructs a finite
//! family of sequential quintic witnesses, and reports its result separately
//! from the recorded simulation outcome.  Bundle manifests are read by the
//! aggregation layer only and are never passed into the pure evaluator.

use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use pd_core::{
    TerrainDefinition, TransferRouteSpec, TransferWaypointSpec, Vec2, VehicleInitialState,
    VehicleSpec, WorldSpec,
};
use serde::Deserialize;

const SAMPLES_PER_SEGMENT: usize = 96;
const SPEED_FRACTIONS: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];
// The profile's average-speed floor is an upper duration bound.  Every
// candidate therefore travels at least that average speed (or the profile's
// explicit maximum-duration cap, whichever is smaller).
const DURATION_FACTORS: [f64; 3] = [1.0, 0.85, 0.70];

#[derive(Clone, Copy, Debug)]
struct ResearchProfile {
    name: &'static str,
    tube_radius_m: f64,
    extra_clearance_m: f64,
    endpoint_taper_m: f64,
    handoff_reserve_fraction: f64,
    handoff_velocity_error_budget_mps: f64,
    minimum_average_speed_mps: f64,
    maximum_duration_s: f64,
    maximum_thrust_utilization: f64,
    tilt_cap_rad: f64,
}

const PROFILES: [ResearchProfile; 3] = [
    ResearchProfile {
        name: "permissive",
        tube_radius_m: 2.0,
        extra_clearance_m: 4.0,
        endpoint_taper_m: 120.0,
        handoff_reserve_fraction: 0.0,
        handoff_velocity_error_budget_mps: 0.75,
        minimum_average_speed_mps: 14.0,
        maximum_duration_s: 90.0,
        maximum_thrust_utilization: 1.00,
        tilt_cap_rad: 1.05,
    },
    ResearchProfile {
        name: "moderate",
        tube_radius_m: 5.0,
        extra_clearance_m: 10.0,
        endpoint_taper_m: 140.0,
        handoff_reserve_fraction: 0.05,
        handoff_velocity_error_budget_mps: 1.25,
        minimum_average_speed_mps: 18.0,
        maximum_duration_s: 75.0,
        maximum_thrust_utilization: 0.90,
        tilt_cap_rad: 0.90,
    },
    ResearchProfile {
        name: "conservative",
        tube_radius_m: 8.0,
        extra_clearance_m: 18.0,
        endpoint_taper_m: 160.0,
        handoff_reserve_fraction: 0.10,
        handoff_velocity_error_budget_mps: 1.75,
        minimum_average_speed_mps: 22.0,
        maximum_duration_s: 62.0,
        maximum_thrust_utilization: 0.80,
        tilt_cap_rad: 0.78,
    },
];

#[derive(Debug)]
struct InputError(String);

impl fmt::Display for InputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for InputError {}

#[derive(Clone, Debug, Deserialize)]
struct SummaryFile {
    records: Vec<SummaryRecord>,
}

#[derive(Clone, Debug, Deserialize)]
struct SummaryRecord {
    bundle_dir: String,
    resolved: ResolvedRecord,
}

#[derive(Clone, Debug, Deserialize)]
struct ResolvedRecord {
    run_id: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ManifestRecord {
    scenario_id: String,
    mission_outcome: String,
}

/// Deliberately narrow physical projection of scenario.json.  Unknown fields
/// (including id, seed, tags, metadata, controller information, and labels)
/// are ignored by serde and never enter the evaluator.
#[derive(Clone, Debug, Deserialize)]
struct ScenarioPhysicalInput {
    world: WorldSpec,
    vehicle: VehicleSpec,
    initial_state: VehicleInitialState,
}

/// Deliberately narrow route projection of route_plan.json.  The plan digest,
/// algorithm id, policy, and diagnostics are not evaluator inputs.
#[derive(Clone, Debug, Deserialize)]
struct RoutePlanInput {
    route: TransferRouteSpec,
}

#[derive(Clone, Debug)]
struct CorpusCase {
    corpus: String,
    run_id: String,
    outcome_success: bool,
    bundle_dir: PathBuf,
}

#[derive(Clone, Debug)]
struct PhysicalInput {
    world: WorldSpec,
    vehicle: VehicleSpec,
    initial_state: VehicleInitialState,
    route: TransferRouteSpec,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvaluationStatus {
    Feasible,
    Infeasible,
    SkippedDirect,
}

impl EvaluationStatus {
    fn predicted(self) -> Option<bool> {
        match self {
            Self::Feasible => Some(true),
            Self::Infeasible => Some(false),
            Self::SkippedDirect => None,
        }
    }
}

#[derive(Clone, Debug)]
struct WitnessDiagnostics {
    total_duration_s: f64,
    handoff_speeds_mps: Vec<f64>,
    segment_durations_s: Vec<f64>,
    minimum_clearance_m: f64,
    minimum_clearance_location: Option<ClearanceLocation>,
    maximum_thrust_utilization: f64,
    maximum_speed_mps: f64,
    maximum_tilt_rad: f64,
    maximum_attitude_rate_radps: f64,
}

#[derive(Clone, Debug)]
struct EvaluationResult {
    status: EvaluationStatus,
    limiting_constraint: String,
    limiting_violation: Option<f64>,
    witness: Option<WitnessDiagnostics>,
    best_attempt: Option<WitnessDiagnostics>,
    attempted_witnesses: usize,
}

#[derive(Clone, Debug)]
struct CaseResult {
    case: CorpusCase,
    result: EvaluationResult,
}

#[derive(Clone, Debug, Default)]
struct Counts {
    total: usize,
    feasible: usize,
    infeasible: usize,
    skipped: usize,
    true_positive: usize,
    true_negative: usize,
    false_positive: usize,
    false_negative: usize,
    limiting_constraints: BTreeMap<String, usize>,
}

impl Counts {
    fn add(&mut self, result: &CaseResult) {
        self.total += 1;
        match result.result.status {
            EvaluationStatus::Feasible => self.feasible += 1,
            EvaluationStatus::Infeasible => self.infeasible += 1,
            EvaluationStatus::SkippedDirect => self.skipped += 1,
        }
        *self
            .limiting_constraints
            .entry(result.result.limiting_constraint.clone())
            .or_default() += 1;
        let Some(predicted) = result.result.status.predicted() else {
            return;
        };
        match (predicted, result.case.outcome_success) {
            (true, true) => self.true_positive += 1,
            (false, false) => self.true_negative += 1,
            (true, false) => self.false_positive += 1,
            (false, true) => self.false_negative += 1,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Constraint {
    Input,
    Handoff,
    Duration,
    TerrainTube,
    Thrust,
    Tilt,
    AttitudeRate,
}

impl Constraint {
    fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Handoff => "handoff_envelope",
            Self::Duration => "duration_bound",
            Self::TerrainTube => "terrain_hull_tube",
            Self::Thrust => "gravity_inclusive_thrust",
            Self::Tilt => "tilt_cap",
            Self::AttitudeRate => "attitude_rate",
        }
    }
}

#[derive(Clone, Debug)]
struct Failure {
    constraint: Constraint,
    normalized_violation: f64,
    diagnostics: Option<Box<CandidateDiagnostics>>,
}

#[derive(Clone, Copy, Debug)]
struct ClearanceLocation {
    elapsed_time_s: f64,
    segment_index: usize,
    segment_fraction: f64,
    position: Vec2,
    source_contact: bool,
}

#[derive(Clone, Debug)]
struct CandidateDiagnostics {
    total_duration_s: f64,
    handoff_speeds_mps: Vec<f64>,
    segment_durations_s: Vec<f64>,
    minimum_clearance_m: f64,
    minimum_clearance_location: Option<ClearanceLocation>,
    maximum_thrust_utilization: f64,
    maximum_speed_mps: f64,
    maximum_tilt_rad: f64,
    maximum_attitude_rate_radps: f64,
}

#[derive(Clone, Copy, Debug)]
struct PolySegment {
    start: Vec2,
    end: Vec2,
    start_velocity: Vec2,
    end_velocity: Vec2,
    duration_s: f64,
}

#[derive(Clone, Copy, Debug)]
struct PolyState {
    position: Vec2,
    velocity: Vec2,
    acceleration: Vec2,
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.is_empty() {
        return Err(Box::new(InputError(
            "usage: trajectory_tube_spike LABEL=SUMMARY_JSON [...]".to_owned(),
        )));
    }

    let mut cases = Vec::new();
    for argument in arguments {
        let (label, summary_path) = argument.split_once('=').ok_or_else(|| {
            InputError(format!(
                "input '{argument}' must use LABEL=SUMMARY_JSON (labels are aggregation-only)"
            ))
        })?;
        if label.trim().is_empty() || summary_path.trim().is_empty() {
            return Err(Box::new(InputError(format!(
                "input '{argument}' has an empty label or summary path"
            ))));
        }
        cases.extend(load_summary(label, Path::new(summary_path))?);
    }
    if cases.is_empty() {
        return Err(Box::new(InputError(
            "all supplied summaries contain zero records".to_owned(),
        )));
    }

    let mut profile_results = Vec::new();
    for profile in PROFILES {
        let mut results = Vec::with_capacity(cases.len());
        for case in &cases {
            let physical = load_physical_input(&case.bundle_dir).map_err(|error| {
                InputError(format!(
                    "{} case {} bundle {}: {error}",
                    case.corpus,
                    case.run_id,
                    case.bundle_dir.display()
                ))
            })?;
            let result = evaluate(profile, &physical).map_err(|error| {
                InputError(format!(
                    "{} case {} bundle {}: {error}",
                    case.corpus,
                    case.run_id,
                    case.bundle_dir.display()
                ))
            })?;
            results.push(CaseResult {
                case: case.clone(),
                result,
            });
        }
        profile_results.push((profile, results));
    }

    println!(
        "trajectory_tube_spike evaluator=label_free sequential_quintic scope=initial_state_to_last_waypoint target_landing=excluded"
    );
    println!(
        "sampling=research_dense samples_per_segment={} duration_grid_factors=1.00,0.85,0.70 duration_semantics=maximum_from_min_average_speed fuel_check=omitted",
        SAMPLES_PER_SEGMENT,
    );
    for (profile, results) in &profile_results {
        print_profile_summary(*profile, results);
    }

    println!("profile_mismatches:");
    for (profile, results) in &profile_results {
        for result in results {
            let mismatch = result.result.status.predicted() != Some(result.case.outcome_success);
            if mismatch {
                print_case_result(*profile, result);
            }
        }
    }
    Ok(())
}

fn print_profile_summary(profile: ResearchProfile, results: &[CaseResult]) {
    let mut by_corpus: BTreeMap<&str, Counts> = BTreeMap::new();
    for result in results {
        by_corpus
            .entry(result.case.corpus.as_str())
            .or_default()
            .add(result);
    }
    println!(
        "profile={} tube_m={:.1} extra_clearance_m={:.1} taper_m={:.1} handoff_reserve={:.2} velocity_error_budget_mps={:.2} min_avg_speed_mps={:.1} max_duration_s={:.1} max_thrust_utilization={:.2} tilt_cap_rad={:.2}",
        profile.name,
        profile.tube_radius_m,
        profile.extra_clearance_m,
        profile.endpoint_taper_m,
        profile.handoff_reserve_fraction,
        profile.handoff_velocity_error_budget_mps,
        profile.minimum_average_speed_mps,
        profile.maximum_duration_s,
        profile.maximum_thrust_utilization,
        profile.tilt_cap_rad
    );
    for (corpus, counts) in by_corpus {
        println!(
            "  corpus={} total={} feasible={} infeasible={} skipped={} TP={} TN={} FP={} FN={}",
            corpus,
            counts.total,
            counts.feasible,
            counts.infeasible,
            counts.skipped,
            counts.true_positive,
            counts.true_negative,
            counts.false_positive,
            counts.false_negative
        );
        let limiting_distribution = counts
            .limiting_constraints
            .iter()
            .map(|(constraint, count)| format!("{constraint}:{count}"))
            .collect::<Vec<_>>()
            .join(",");
        println!("    limiting_distribution={limiting_distribution}");
    }
}

fn print_case_result(profile: ResearchProfile, result: &CaseResult) {
    let predicted = match result.result.status {
        EvaluationStatus::Feasible => "feasible",
        EvaluationStatus::Infeasible => "infeasible",
        EvaluationStatus::SkippedDirect => "skipped_direct",
    };
    let actual = if result.case.outcome_success {
        "success"
    } else {
        "failure"
    };
    let mismatch = result.result.status.predicted() != Some(result.case.outcome_success);
    let diagnostic = result
        .result
        .witness
        .as_ref()
        .or(result.result.best_attempt.as_ref());
    let witness = diagnostic.map(|witness| {
        let clearance_location = witness
            .minimum_clearance_location
            .map(|location| {
                format!(
                    "t={:.2},seg={},tau={:.3},pos=({:.2},{:.2}),source_contact={}",
                    location.elapsed_time_s,
                    location.segment_index,
                    location.segment_fraction,
                    location.position.x,
                    location.position.y,
                    location.source_contact
                )
            })
            .unwrap_or_else(|| "none".to_owned());
        format!(
            "duration_s={:.2},speeds_mps={},clearance_m={:.2},clearance_location={},thrust={:.3},speed_mps={:.2},tilt_rad={:.3},rate_radps={:.3}",
            witness.total_duration_s,
            format_f64s(&witness.handoff_speeds_mps),
            witness.minimum_clearance_m,
            clearance_location,
            witness.maximum_thrust_utilization,
            witness.maximum_speed_mps,
            witness.maximum_tilt_rad,
            witness.maximum_attitude_rate_radps
        )
    })
        .unwrap_or_else(|| "none".to_owned());
    println!(
        "  case profile={} corpus={} run_id={} predicted={} actual={} mismatch={} limiting={} normalized_violation={} attempts={} witness_or_best_attempt={} segments_s={}",
        profile.name,
        result.case.corpus,
        result.case.run_id,
        predicted,
        actual,
        mismatch,
        result.result.limiting_constraint,
        result
            .result
            .limiting_violation
            .map_or_else(|| "none".to_owned(), |value| format!("{value:.6}")),
        result.result.attempted_witnesses,
        witness,
        diagnostic
            .map(|witness| format_f64s(&witness.segment_durations_s))
            .unwrap_or_else(|| "none".to_owned())
    );
}

fn format_f64s(values: &[f64]) -> String {
    values
        .iter()
        .map(|value| format!("{value:.3}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn load_summary(label: &str, path: &Path) -> Result<Vec<CorpusCase>, Box<dyn Error>> {
    let raw = fs::read_to_string(path)
        .map_err(|error| InputError(format!("cannot read summary {}: {error}", path.display())))?;
    let summary: SummaryFile = serde_json::from_str(&raw)
        .map_err(|error| InputError(format!("cannot parse summary {}: {error}", path.display())))?;
    if summary.records.is_empty() {
        return Err(Box::new(InputError(format!(
            "summary {} has no records",
            path.display()
        ))));
    }
    let summary_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut cases = Vec::with_capacity(summary.records.len());
    for (index, record) in summary.records.into_iter().enumerate() {
        if record.bundle_dir.trim().is_empty() {
            return Err(Box::new(InputError(format!(
                "summary {} record {} has no bundle_dir",
                path.display(),
                index
            ))));
        }
        let bundle_dir = PathBuf::from(&record.bundle_dir);
        let bundle_dir = if bundle_dir.is_absolute() {
            bundle_dir
        } else {
            summary_dir.join(bundle_dir)
        };
        if !bundle_dir.is_dir() {
            return Err(Box::new(InputError(format!(
                "summary {} record {} bundle_dir is not a directory: {}",
                path.display(),
                index,
                bundle_dir.display()
            ))));
        }
        if record.resolved.run_id.trim().is_empty() {
            return Err(Box::new(InputError(format!(
                "summary {} record {} has an empty run_id",
                path.display(),
                index
            ))));
        }
        let manifest = load_manifest(&bundle_dir).map_err(|error| {
            InputError(format!(
                "summary {} record {} bundle {} manifest: {error}",
                path.display(),
                index,
                bundle_dir.display()
            ))
        })?;
        // These are the serialized MissionOutcome variants.  The evaluator
        // deliberately does not infer a result from controller telemetry or
        // any scenario metadata.
        let outcome_success = match manifest.mission_outcome.as_str() {
            "success" => true,
            "failed_off_target" | "failed_checkpoint" | "failed_crash" | "failed_timeout" => false,
            other => {
                return Err(Box::new(InputError(format!(
                    "summary {} record {} bundle manifest has unsupported mission_outcome '{other}'",
                    path.display(),
                    index
                ))));
            }
        };
        cases.push(CorpusCase {
            corpus: label.to_owned(),
            run_id: record.resolved.run_id,
            outcome_success,
            bundle_dir,
        });
    }
    Ok(cases)
}

fn load_manifest(bundle_dir: &Path) -> Result<ManifestRecord, Box<dyn Error>> {
    let path = bundle_dir.join("manifest.json");
    if !path.is_file() {
        return Err(Box::new(InputError(format!(
            "required artifact is missing: {}",
            path.display()
        ))));
    }
    let raw = fs::read_to_string(&path)
        .map_err(|error| InputError(format!("cannot read {}: {error}", path.display())))?;
    let manifest: ManifestRecord = serde_json::from_str(&raw)
        .map_err(|error| InputError(format!("cannot parse {}: {error}", path.display())))?;
    if manifest.scenario_id.trim().is_empty() {
        return Err(Box::new(InputError(format!(
            "manifest {} has an empty scenario_id",
            path.display()
        ))));
    }
    if manifest.mission_outcome.trim().is_empty() {
        return Err(Box::new(InputError(format!(
            "manifest {} has an empty mission_outcome",
            path.display()
        ))));
    }
    Ok(manifest)
}

fn load_physical_input(bundle_dir: &Path) -> Result<PhysicalInput, Box<dyn Error>> {
    let scenario_path = bundle_dir.join("scenario.json");
    let route_path = bundle_dir.join("route_plan.json");
    for path in [&scenario_path, &route_path] {
        if !path.is_file() {
            return Err(Box::new(InputError(format!(
                "required artifact is missing: {}",
                path.display()
            ))));
        }
    }
    let scenario_raw = fs::read_to_string(&scenario_path)
        .map_err(|error| InputError(format!("cannot read {}: {error}", scenario_path.display())))?;
    let scenario: ScenarioPhysicalInput = serde_json::from_str(&scenario_raw).map_err(|error| {
        InputError(format!("cannot parse {}: {error}", scenario_path.display()))
    })?;
    let route_raw = fs::read_to_string(&route_path)
        .map_err(|error| InputError(format!("cannot read {}: {error}", route_path.display())))?;
    let route_plan: RoutePlanInput = serde_json::from_str(&route_raw)
        .map_err(|error| InputError(format!("cannot parse {}: {error}", route_path.display())))?;
    scenario
        .world
        .validate()
        .map_err(|message| InputError(format!("invalid world: {message}")))?;
    scenario
        .vehicle
        .validate()
        .map_err(|message| InputError(format!("invalid vehicle: {message}")))?;
    scenario
        .initial_state
        .validate()
        .map_err(|message| InputError(format!("invalid initial state: {message}")))?;
    route_plan
        .route
        .validate()
        .map_err(|message| InputError(format!("invalid route: {message}")))?;
    Ok(PhysicalInput {
        world: scenario.world,
        vehicle: scenario.vehicle,
        initial_state: scenario.initial_state,
        route: route_plan.route,
    })
}

/// Pure evaluator entry point.  No corpus label, run id, manifest outcome,
/// seed, controller data, or metadata is accepted here.
fn evaluate(
    profile: ResearchProfile,
    input: &PhysicalInput,
) -> Result<EvaluationResult, Box<dyn Error>> {
    if input.route.waypoints.is_empty() {
        return Ok(EvaluationResult {
            status: EvaluationStatus::SkippedDirect,
            limiting_constraint: "direct_route_not_in_spike_scope".to_owned(),
            limiting_violation: None,
            witness: None,
            best_attempt: None,
            attempted_witnesses: 0,
        });
    }
    let source_pad = input
        .world
        .landing_pad(&input.route.source_pad_id)
        .ok_or_else(|| InputError("route source pad is absent from world".to_owned()))?;
    let target_pad = input
        .world
        .landing_pad(&input.route.target_pad_id)
        .ok_or_else(|| InputError("route target pad is absent from world".to_owned()))?;
    let source_reference = Vec2::new(
        source_pad.center_x_m,
        source_pad.surface_y_m + input.vehicle.geometry.touchdown_base_offset_m,
    );
    let target = Vec2::new(
        target_pad.center_x_m,
        target_pad.surface_y_m + input.vehicle.geometry.touchdown_base_offset_m,
    );
    if (input.initial_state.position_m - source_reference).length() > 1.0e-6 {
        return Err(Box::new(InputError(
            "initial state does not match source touchdown reference".to_owned(),
        )));
    }
    validate_terrain_domain(input, profile)?;

    let source = input.initial_state.position_m;
    // The corpus goal is waypoint handoff/ordered-sequence evidence.  Keep
    // the target in the geometry frame for outbound tangent direction and
    // source taper orientation, but do not score the uncontracted landing leg.
    let geometry_points = route_points(source, target, &input.route);
    let points = route_prefix_points(source, &input.route);
    let speed_options = handoff_speed_options(&geometry_points, &input.route, profile)?;
    let leg_lengths = points
        .windows(2)
        .map(|window| (window[1] - window[0]).length())
        .collect::<Vec<_>>();
    if leg_lengths.iter().any(|length| *length <= f64::EPSILON) {
        return Err(Box::new(InputError(
            "route contains a zero-length segment".to_owned(),
        )));
    }
    let covered_length_m = leg_lengths.iter().sum::<f64>();
    let duration_candidates =
        duration_candidates(covered_length_m, profile).map_err(|message| {
            InputError(format!(
                "invalid duration contract for {}: {message}",
                profile.name
            ))
        })?;
    if duration_candidates.is_empty() {
        return Ok(EvaluationResult {
            status: EvaluationStatus::Infeasible,
            limiting_constraint: Constraint::Duration.as_str().to_owned(),
            limiting_violation: Some(1.0),
            witness: None,
            best_attempt: None,
            attempted_witnesses: 0,
        });
    }

    if speed_options.len() != input.route.waypoints.len() || speed_options.iter().any(Vec::is_empty)
    {
        return Ok(EvaluationResult {
            status: EvaluationStatus::Infeasible,
            limiting_constraint: Constraint::Handoff.as_str().to_owned(),
            limiting_violation: Some(1.0),
            witness: None,
            best_attempt: None,
            attempted_witnesses: 0,
        });
    }

    let mut combinations = Vec::new();
    enumerate_speed_combinations(&speed_options, 0, &mut Vec::new(), &mut combinations);
    if combinations.is_empty() {
        return Ok(EvaluationResult {
            status: EvaluationStatus::Infeasible,
            limiting_constraint: Constraint::Handoff.as_str().to_owned(),
            limiting_violation: Some(1.0),
            witness: None,
            best_attempt: None,
            attempted_witnesses: 0,
        });
    }

    let mut best_witness: Option<CandidateDiagnostics> = None;
    let mut least_bad_failure: Option<Failure> = None;
    let mut attempts = 0;
    for speed_values in combinations {
        for total_duration_s in &duration_candidates {
            attempts += 1;
            let durations = leg_lengths
                .iter()
                .map(|length| *total_duration_s * length / covered_length_m)
                .collect::<Vec<_>>();
            let segments = build_segments(
                &points,
                &geometry_points,
                &input.initial_state,
                &input.route,
                &speed_values,
                &durations,
            )?;
            match assess_candidate(
                profile,
                input,
                &geometry_points,
                &segments,
                &speed_values,
                &durations,
            ) {
                Ok(candidate) => {
                    if best_witness
                        .as_ref()
                        .is_none_or(|best| candidate_is_better(&candidate, best))
                    {
                        best_witness = Some(candidate);
                    }
                }
                Err(failure) => {
                    if least_bad_failure
                        .as_ref()
                        .is_none_or(|best| failure.normalized_violation < best.normalized_violation)
                    {
                        least_bad_failure = Some(failure);
                    }
                }
            }
        }
    }
    if let Some(witness) = best_witness {
        return Ok(EvaluationResult {
            status: EvaluationStatus::Feasible,
            limiting_constraint: "none".to_owned(),
            limiting_violation: None,
            witness: Some(witness_to_diagnostics(&witness)),
            best_attempt: Some(witness_to_diagnostics(&witness)),
            attempted_witnesses: attempts,
        });
    }
    let best_attempt = least_bad_failure
        .as_ref()
        .and_then(|failure| failure.diagnostics.as_deref())
        .map(witness_to_diagnostics);
    let limiting_constraint = least_bad_failure
        .as_ref()
        .map(|failure| failure.constraint.as_str().to_owned())
        .unwrap_or_else(|| Constraint::Input.as_str().to_owned());
    let limiting_violation = least_bad_failure
        .as_ref()
        .map(|failure| failure.normalized_violation);
    Ok(EvaluationResult {
        status: EvaluationStatus::Infeasible,
        limiting_constraint,
        limiting_violation,
        witness: None,
        best_attempt,
        attempted_witnesses: attempts,
    })
}

fn witness_to_diagnostics(candidate: &CandidateDiagnostics) -> WitnessDiagnostics {
    WitnessDiagnostics {
        total_duration_s: candidate.total_duration_s,
        handoff_speeds_mps: candidate.handoff_speeds_mps.clone(),
        segment_durations_s: candidate.segment_durations_s.clone(),
        minimum_clearance_m: candidate.minimum_clearance_m,
        minimum_clearance_location: candidate.minimum_clearance_location,
        maximum_thrust_utilization: candidate.maximum_thrust_utilization,
        maximum_speed_mps: candidate.maximum_speed_mps,
        maximum_tilt_rad: candidate.maximum_tilt_rad,
        maximum_attitude_rate_radps: candidate.maximum_attitude_rate_radps,
    }
}

fn duration_candidates(
    covered_length_m: f64,
    profile: ResearchProfile,
) -> Result<Vec<f64>, String> {
    if !covered_length_m.is_finite() || covered_length_m <= 0.0 {
        return Err("covered route length must be positive and finite".to_owned());
    }
    if !profile.minimum_average_speed_mps.is_finite() || profile.minimum_average_speed_mps <= 0.0 {
        return Err("minimum average speed must be positive and finite".to_owned());
    }
    if !profile.maximum_duration_s.is_finite() || profile.maximum_duration_s <= 0.0 {
        return Err("maximum duration must be positive and finite".to_owned());
    }
    // This is an upper bound: duration <= covered length / minimum average
    // speed.  A profile may impose an even tighter absolute duration cap.
    let speed_bound_s = covered_length_m / profile.minimum_average_speed_mps;
    let allowed_max_s = speed_bound_s.min(profile.maximum_duration_s);
    if !allowed_max_s.is_finite() || allowed_max_s <= 0.0 {
        return Err("duration upper bound must be positive and finite".to_owned());
    }
    let mut candidates = DURATION_FACTORS
        .iter()
        .map(|factor| allowed_max_s * factor)
        .filter(|duration| duration.is_finite() && *duration > 0.0)
        .collect::<Vec<_>>();
    candidates.dedup_by(|left, right| (*left - *right).abs() <= 1.0e-9);
    Ok(candidates)
}

fn validate_terrain_domain(
    input: &PhysicalInput,
    profile: ResearchProfile,
) -> Result<(), Box<dyn Error>> {
    let TerrainDefinition::Heightfield { points_m } = &input.world.terrain;
    let (domain_min, domain_max) = (points_m[0].x, points_m[points_m.len() - 1].x);
    let max_horizontal_extent = input.vehicle.geometry.hull_width_m * 0.5
        + profile.tube_radius_m
        + profile.extra_clearance_m;
    // This coarse check covers only physical points in the scored prefix.
    // The target pad is intentionally outside the handoff contract; dense
    // samples are authoritative for the actual prefix hull/tube clearance.
    for point in route_prefix_points(input.initial_state.position_m, &input.route) {
        if point.x - max_horizontal_extent < domain_min - 1.0e-6
            || point.x + max_horizontal_extent > domain_max + 1.0e-6
        {
            return Err(Box::new(InputError(format!(
                "terrain domain cannot contain tracking tube near x={:.3}",
                point.x
            ))));
        }
    }
    Ok(())
}

fn route_points(source: Vec2, target: Vec2, route: &TransferRouteSpec) -> Vec<Vec2> {
    let mut points = Vec::with_capacity(route.waypoints.len() + 2);
    points.push(source);
    points.extend(route.waypoints.iter().map(|waypoint| waypoint.position_m));
    points.push(target);
    points
}

fn route_prefix_points(source: Vec2, route: &TransferRouteSpec) -> Vec<Vec2> {
    let mut points = Vec::with_capacity(route.waypoints.len() + 1);
    points.push(source);
    points.extend(route.waypoints.iter().map(|waypoint| waypoint.position_m));
    points
}

fn handoff_speed_options(
    points: &[Vec2],
    route: &TransferRouteSpec,
    profile: ResearchProfile,
) -> Result<Vec<Vec<f64>>, Box<dyn Error>> {
    let mut options = Vec::with_capacity(route.waypoints.len());
    for (index, waypoint) in route.waypoints.iter().enumerate() {
        let tangent = waypoint_tangent(points, index + 1, waypoint)?;
        let budget = profile.handoff_velocity_error_budget_mps;
        let minimum = (waypoint.min_speed_mps * (1.0 + profile.handoff_reserve_fraction))
            .max(waypoint.min_outbound_progress_mps * (1.0 + profile.handoff_reserve_fraction))
            + budget;
        let maximum = waypoint.max_speed_mps * (1.0 - profile.handoff_reserve_fraction) - budget;
        let reserve = profile.handoff_reserve_fraction;
        let position_limit =
            waypoint.capture_radius_m.min(waypoint.max_cross_track_m) * (1.0 - reserve);
        if profile.tube_radius_m > position_limit + 1.0e-9 {
            return Ok(Vec::new());
        }
        if waypoint
            .max_outbound_cross_speed_mps
            .is_some_and(|maximum| maximum * (1.0 - reserve) < budget)
        {
            return Ok(Vec::new());
        }
        let (vertical_minimum, vertical_maximum) =
            vertical_speed_interval(waypoint, tangent, budget);
        let minimum = minimum.max(vertical_minimum);
        let maximum = maximum.min(vertical_maximum);
        if minimum > maximum + 1.0e-9 {
            return Ok(Vec::new());
        }
        let mut speeds = SPEED_FRACTIONS
            .iter()
            .map(|fraction| minimum + (maximum - minimum) * fraction)
            .collect::<Vec<_>>();
        speeds.dedup_by(|left, right| (*left - *right).abs() <= 1.0e-9);
        if speeds
            .iter()
            .any(|speed| !speed.is_finite() || *speed < 0.0)
        {
            return Err(Box::new(InputError(format!(
                "waypoint {index} produced a non-finite speed grid"
            ))));
        }
        options.push(speeds);
    }
    Ok(options)
}

fn vertical_speed_interval(
    waypoint: &TransferWaypointSpec,
    tangent: Vec2,
    budget: f64,
) -> (f64, f64) {
    let mut minimum: f64 = 0.0;
    let mut maximum: f64 = f64::INFINITY;
    if let Some(limit) = waypoint.min_vertical_speed_mps {
        if tangent.y > 0.0 {
            minimum = minimum.max((limit + budget) / tangent.y);
        } else if tangent.y < 0.0 {
            maximum = maximum.min((limit + budget) / tangent.y);
        } else if limit > -budget {
            return (1.0, 0.0);
        }
    }
    if let Some(limit) = waypoint.max_vertical_speed_mps {
        if tangent.y > 0.0 {
            maximum = maximum.min((limit - budget) / tangent.y);
        } else if tangent.y < 0.0 {
            minimum = minimum.max((limit - budget) / tangent.y);
        } else if limit < budget {
            return (1.0, 0.0);
        }
    }
    (minimum.max(0.0), maximum)
}

fn enumerate_speed_combinations(
    options: &[Vec<f64>],
    index: usize,
    current: &mut Vec<f64>,
    combinations: &mut Vec<Vec<f64>>,
) {
    if index == options.len() {
        combinations.push(current.clone());
        return;
    }
    for speed in &options[index] {
        current.push(*speed);
        enumerate_speed_combinations(options, index + 1, current, combinations);
        current.pop();
    }
}

fn build_segments(
    points: &[Vec2],
    geometry_points: &[Vec2],
    initial_state: &VehicleInitialState,
    route: &TransferRouteSpec,
    handoff_speeds: &[f64],
    durations: &[f64],
) -> Result<Vec<PolySegment>, Box<dyn Error>> {
    let mut velocities = Vec::with_capacity(points.len());
    velocities.push(initial_state.velocity_mps);
    for (index, (waypoint, speed)) in route
        .waypoints
        .iter()
        .zip(handoff_speeds.iter().copied())
        .enumerate()
    {
        let tangent = waypoint_tangent(geometry_points, index + 1, waypoint)?;
        velocities.push(tangent * speed);
    }
    Ok(points
        .windows(2)
        .zip(durations)
        .enumerate()
        .map(|(index, (window, duration_s))| PolySegment {
            start: window[0],
            end: window[1],
            start_velocity: velocities[index],
            end_velocity: velocities[index + 1],
            duration_s: *duration_s,
        })
        .collect())
}

fn waypoint_tangent(
    points: &[Vec2],
    point_index: usize,
    waypoint: &TransferWaypointSpec,
) -> Result<Vec2, Box<dyn Error>> {
    let tangent = waypoint
        .handoff_tangent_unit
        .unwrap_or(points[point_index + 1] - points[point_index]);
    let length = tangent.length();
    if !length.is_finite() || length <= f64::EPSILON {
        return Err(Box::new(InputError(format!(
            "waypoint at index {point_index} has no usable outbound tangent"
        ))));
    }
    Ok(tangent * (1.0 / length))
}

fn assess_candidate(
    profile: ResearchProfile,
    input: &PhysicalInput,
    geometry_points: &[Vec2],
    segments: &[PolySegment],
    handoff_speeds: &[f64],
    durations: &[f64],
) -> Result<CandidateDiagnostics, Failure> {
    let mut minimum_clearance_m = f64::INFINITY;
    let mut minimum_clearance_location = None;
    let mut maximum_thrust_utilization: f64 = 0.0;
    let mut maximum_speed_mps: f64 = 0.0;
    let mut maximum_tilt_rad: f64 = 0.0;
    let mut maximum_attitude_rate_radps: f64 = 0.0;
    let mut previous_attitude = input.initial_state.attitude_rad;
    let mass_kg = input.vehicle.dry_mass_kg + input.vehicle.initial_fuel_kg;
    let max_rate_radps = input.vehicle.max_rotation_rate_radps;
    let mut elapsed_time_s = 0.0;
    for (segment_index, segment) in segments.iter().enumerate() {
        for sample_index in 0..=SAMPLES_PER_SEGMENT {
            let fraction = sample_index as f64 / SAMPLES_PER_SEGMENT as f64;
            let state = evaluate_quintic(*segment, fraction);
            let thrust_acceleration = state.acceleration + Vec2::new(0.0, input.world.gravity_mps2);
            let thrust_magnitude = thrust_acceleration.length();
            let thrust_utilization = thrust_magnitude * mass_kg / input.vehicle.max_thrust_n;
            let tilt_rad = thrust_acceleration.x.abs().atan2(thrust_acceleration.y);
            let attitude = thrust_acceleration.x.atan2(thrust_acceleration.y);
            let dt_s = segment.duration_s / SAMPLES_PER_SEGMENT as f64;
            let rate = wrap_angle(attitude - previous_attitude).abs() / dt_s;
            maximum_attitude_rate_radps = maximum_attitude_rate_radps.max(rate);
            previous_attitude = attitude;
            maximum_thrust_utilization = maximum_thrust_utilization.max(thrust_utilization);
            maximum_speed_mps = maximum_speed_mps.max(state.velocity.length());
            maximum_tilt_rad = maximum_tilt_rad.max(tilt_rad);
            let clearance = terrain_hull_tube_clearance(profile, input, state.position, attitude)
                .map_err(|_| Failure {
                constraint: Constraint::TerrainTube,
                normalized_violation: 1.0,
                diagnostics: None,
            })?;
            if clearance < minimum_clearance_m {
                minimum_clearance_m = clearance;
                minimum_clearance_location = Some(ClearanceLocation {
                    elapsed_time_s: elapsed_time_s + fraction * segment.duration_s,
                    segment_index,
                    segment_fraction: fraction,
                    position: state.position,
                    source_contact: endpoint_taper_fraction(profile, input, state.position)
                        <= 1.0e-9,
                });
            }
        }
        elapsed_time_s += segment.duration_s;
    }
    let diagnostics = CandidateDiagnostics {
        total_duration_s: durations.iter().sum(),
        handoff_speeds_mps: handoff_speeds.to_vec(),
        segment_durations_s: durations.to_vec(),
        minimum_clearance_m,
        minimum_clearance_location,
        maximum_thrust_utilization,
        maximum_speed_mps,
        maximum_tilt_rad,
        maximum_attitude_rate_radps,
    };

    let mut worst_violation: Option<(Constraint, f64)> = None;
    let mut consider_violation = |constraint: Constraint, violation: f64| {
        if violation.is_finite()
            && violation > 0.0
            && worst_violation
                .as_ref()
                .is_none_or(|(_, current)| violation > *current)
        {
            worst_violation = Some((constraint, violation));
        }
    };
    for (index, waypoint) in input.route.waypoints.iter().enumerate() {
        let tangent =
            waypoint_tangent(geometry_points, index + 1, waypoint).map_err(|_| Failure {
                constraint: Constraint::Handoff,
                normalized_violation: 1.0,
                diagnostics: Some(Box::new(diagnostics.clone())),
            })?;
        let speed = handoff_speeds[index];
        let velocity = tangent * speed;
        let reserve = profile.handoff_reserve_fraction;
        let budget = profile.handoff_velocity_error_budget_mps;
        let minimum_speed = waypoint.min_speed_mps * (1.0 + reserve) + budget;
        let maximum_speed = waypoint.max_speed_mps * (1.0 - reserve) - budget;
        consider_violation(
            Constraint::Handoff,
            (minimum_speed - speed) / minimum_speed.max(1.0),
        );
        consider_violation(
            Constraint::Handoff,
            (speed - maximum_speed) / maximum_speed.abs().max(1.0),
        );
        let minimum_progress = waypoint.min_outbound_progress_mps * (1.0 + reserve) + budget;
        consider_violation(
            Constraint::Handoff,
            (minimum_progress - dot(velocity, tangent)) / minimum_progress.max(1.0),
        );
        let position_limit =
            waypoint.capture_radius_m.min(waypoint.max_cross_track_m) * (1.0 - reserve);
        consider_violation(
            Constraint::Handoff,
            (profile.tube_radius_m - position_limit) / position_limit.max(1.0),
        );
        if let Some(maximum_cross_speed) = waypoint.max_outbound_cross_speed_mps {
            let cross_limit = maximum_cross_speed * (1.0 - reserve);
            consider_violation(
                Constraint::Handoff,
                (budget - cross_limit) / cross_limit.max(1.0),
            );
        }
        let heading_uncertainty = heading_uncertainty_rad(budget, speed);
        let heading_limit = waypoint.max_outbound_heading_error_rad * (1.0 - reserve);
        consider_violation(
            Constraint::Handoff,
            (heading_uncertainty - heading_limit) / heading_limit.max(1.0e-6),
        );
        if let Some(minimum_vertical_speed) = waypoint.min_vertical_speed_mps {
            consider_violation(
                Constraint::Handoff,
                (minimum_vertical_speed + budget - velocity.y)
                    / (minimum_vertical_speed.abs() + budget).max(1.0),
            );
        }
        if let Some(maximum_vertical_speed) = waypoint.max_vertical_speed_mps {
            consider_violation(
                Constraint::Handoff,
                (velocity.y - (maximum_vertical_speed - budget))
                    / (maximum_vertical_speed.abs() + budget).max(1.0),
            );
        }
    }
    consider_violation(
        Constraint::TerrainTube,
        (-minimum_clearance_m) / (profile.tube_radius_m + profile.extra_clearance_m).max(1.0),
    );
    consider_violation(
        Constraint::Thrust,
        (maximum_thrust_utilization / profile.maximum_thrust_utilization) - 1.0,
    );
    consider_violation(
        Constraint::Tilt,
        (maximum_tilt_rad / profile.tilt_cap_rad) - 1.0,
    );
    consider_violation(
        Constraint::AttitudeRate,
        (maximum_attitude_rate_radps / max_rate_radps) - 1.0,
    );
    if let Some((constraint, normalized_violation)) = worst_violation {
        return Err(Failure {
            constraint,
            normalized_violation,
            diagnostics: Some(Box::new(diagnostics.clone())),
        });
    }
    Ok(diagnostics)
}

fn heading_uncertainty_rad(velocity_error_budget_mps: f64, candidate_speed_mps: f64) -> f64 {
    velocity_error_budget_mps.atan2(candidate_speed_mps.max(f64::EPSILON))
}

fn candidate_is_better(candidate: &CandidateDiagnostics, best: &CandidateDiagnostics) -> bool {
    candidate
        .maximum_thrust_utilization
        .total_cmp(&best.maximum_thrust_utilization)
        .then_with(|| candidate.maximum_tilt_rad.total_cmp(&best.maximum_tilt_rad))
        .then_with(|| {
            candidate
                .maximum_attitude_rate_radps
                .total_cmp(&best.maximum_attitude_rate_radps)
        })
        .then_with(|| {
            best.minimum_clearance_m
                .total_cmp(&candidate.minimum_clearance_m)
        })
        .then_with(|| candidate.total_duration_s.total_cmp(&best.total_duration_s))
        == std::cmp::Ordering::Less
}

fn evaluate_quintic(segment: PolySegment, fraction: f64) -> PolyState {
    let tau = fraction.clamp(0.0, 1.0);
    let duration = segment.duration_s;
    let delta = segment.end - segment.start;
    let coefficients = |p0: f64, dp: f64, v0: f64, v1: f64| {
        let c0 = p0;
        let c1 = v0 * duration;
        let c2 = 0.0;
        let c3 = 10.0 * dp - 6.0 * v0 * duration - 4.0 * v1 * duration;
        let c4 = -15.0 * dp + 8.0 * v0 * duration + 7.0 * v1 * duration;
        let c5 = 6.0 * dp - 3.0 * v0 * duration - 3.0 * v1 * duration;
        (c0, c1, c2, c3, c4, c5)
    };
    let x = coefficients(
        segment.start.x,
        delta.x,
        segment.start_velocity.x,
        segment.end_velocity.x,
    );
    let y = coefficients(
        segment.start.y,
        delta.y,
        segment.start_velocity.y,
        segment.end_velocity.y,
    );
    let value = |coefficients: (f64, f64, f64, f64, f64, f64)| {
        let (c0, c1, c2, c3, c4, c5) = coefficients;
        c0 + c1 * tau + c2 * tau.powi(2) + c3 * tau.powi(3) + c4 * tau.powi(4) + c5 * tau.powi(5)
    };
    let derivative = |coefficients: (f64, f64, f64, f64, f64, f64)| {
        let (_, c1, c2, c3, c4, c5) = coefficients;
        (c1 + 2.0 * c2 * tau
            + 3.0 * c3 * tau.powi(2)
            + 4.0 * c4 * tau.powi(3)
            + 5.0 * c5 * tau.powi(4))
            / duration
    };
    let second_derivative = |coefficients: (f64, f64, f64, f64, f64, f64)| {
        let (_, _, c2, c3, c4, c5) = coefficients;
        (2.0 * c2 + 6.0 * c3 * tau + 12.0 * c4 * tau.powi(2) + 20.0 * c5 * tau.powi(3))
            / duration.powi(2)
    };
    PolyState {
        position: Vec2::new(value(x), value(y)),
        velocity: Vec2::new(derivative(x), derivative(y)),
        acceleration: Vec2::new(second_derivative(x), second_derivative(y)),
    }
}

fn terrain_hull_tube_clearance(
    profile: ResearchProfile,
    input: &PhysicalInput,
    position: Vec2,
    attitude_rad: f64,
) -> Result<f64, ()> {
    let TerrainDefinition::Heightfield { points_m } = &input.world.terrain;
    let (sin_a, cos_a) = attitude_rad.sin_cos();
    let hull_half_width = input.vehicle.geometry.hull_width_m * 0.5;
    let hull_half_height = input.vehicle.geometry.hull_height_m * 0.5;
    let taper = endpoint_taper_fraction(profile, input, position);
    let horizontal_extent = hull_half_width * cos_a.abs()
        + hull_half_height * sin_a.abs()
        + profile.tube_radius_m * taper;
    let vertical_extent = hull_half_width * sin_a.abs()
        + hull_half_height * cos_a.abs()
        + (profile.tube_radius_m + profile.extra_clearance_m) * taper;
    let left = position.x - horizontal_extent;
    let right = position.x + horizontal_extent;
    let domain_min = points_m.first().ok_or(())?.x;
    let domain_max = points_m.last().ok_or(())?.x;
    if left < domain_min || right > domain_max {
        return Err(());
    }
    let mut terrain_height = input.world.terrain.sample_height(left);
    terrain_height = terrain_height.max(input.world.terrain.sample_height(right));
    for point in points_m {
        if point.x >= left && point.x <= right {
            terrain_height = terrain_height.max(point.y);
        }
    }
    Ok(position.y - vertical_extent - terrain_height)
}

fn endpoint_taper_fraction(profile: ResearchProfile, input: &PhysicalInput, position: Vec2) -> f64 {
    let Some(source_pad) = input.world.landing_pad(&input.route.source_pad_id) else {
        return 1.0;
    };
    let Some(target_pad) = input.world.landing_pad(&input.route.target_pad_id) else {
        return 1.0;
    };
    let direction = (target_pad.center_x_m - source_pad.center_x_m).signum();
    if direction == 0.0 {
        return 1.0;
    }
    let source_distance = ((position.x - source_pad.center_x_m) * direction).max(0.0);
    let source_start = source_pad.half_width_m() + input.vehicle.geometry.touchdown_half_span_m;
    let taper_length = profile.endpoint_taper_m.max(f64::EPSILON);
    ((source_distance - source_start) / taper_length).clamp(0.0, 1.0)
}

fn dot(left: Vec2, right: Vec2) -> f64 {
    left.x * right.x + left.y * right.y
}

fn wrap_angle(angle: f64) -> f64 {
    let mut wrapped = angle;
    while wrapped > std::f64::consts::PI {
        wrapped -= std::f64::consts::TAU;
    }
    while wrapped < -std::f64::consts::PI {
        wrapped += std::f64::consts::TAU;
    }
    wrapped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_quintic_has_zero_acceleration_at_both_boundaries() {
        let segment = PolySegment {
            start: Vec2::new(2.0, 5.0),
            end: Vec2::new(22.0, -3.0),
            start_velocity: Vec2::new(1.5, -2.0),
            end_velocity: Vec2::new(3.0, 0.5),
            duration_s: 4.0,
        };
        let start = evaluate_quintic(segment, 0.0);
        let end = evaluate_quintic(segment, 1.0);
        assert!((start.position.x - segment.start.x).abs() < 1.0e-9);
        assert!((start.position.y - segment.start.y).abs() < 1.0e-9);
        assert!((end.position.x - segment.end.x).abs() < 1.0e-9);
        assert!((end.position.y - segment.end.y).abs() < 1.0e-9);
        assert!((start.velocity.x - segment.start_velocity.x).abs() < 1.0e-9);
        assert!((start.velocity.y - segment.start_velocity.y).abs() < 1.0e-9);
        assert!((end.velocity.x - segment.end_velocity.x).abs() < 1.0e-9);
        assert!((end.velocity.y - segment.end_velocity.y).abs() < 1.0e-9);
        assert!(start.acceleration.length() < 1.0e-9);
        assert!(end.acceleration.length() < 1.0e-9);

        let next = PolySegment {
            start: segment.end,
            end: Vec2::new(30.0, 2.0),
            start_velocity: segment.end_velocity,
            end_velocity: Vec2::new(0.0, 0.0),
            duration_s: 2.0,
        };
        let next_start = evaluate_quintic(next, 0.0);
        assert!((end.velocity.x - next_start.velocity.x).abs() < 1.0e-9);
        assert!((end.velocity.y - next_start.velocity.y).abs() < 1.0e-9);
    }

    #[test]
    fn duration_candidates_never_exceed_average_speed_bound() {
        let profile = PROFILES[0];
        let covered_length_m = 420.0;
        let bound = covered_length_m / profile.minimum_average_speed_mps;
        let candidates = duration_candidates(covered_length_m, profile).unwrap();
        assert!(
            candidates
                .iter()
                .all(|duration| *duration <= bound + 1.0e-9)
        );
        assert!(candidates.windows(2).all(|pair| pair[0] >= pair[1]));
    }

    #[test]
    fn heading_uncertainty_uses_actual_candidate_speed() {
        let budget = 1.5;
        let low_speed_heading = heading_uncertainty_rad(budget, 10.0);
        let high_speed_heading = heading_uncertainty_rad(budget, 20.0);
        assert!(low_speed_heading > high_speed_heading);
        assert!(low_speed_heading > 0.1);
        assert!(high_speed_heading < 0.1);
    }

    #[test]
    fn terrain_domain_and_route_prefix_end_at_last_waypoint() {
        let route = TransferRouteSpec {
            source_pad_id: "source".to_owned(),
            target_pad_id: "target".to_owned(),
            route_angle_deg: 0.0,
            route_radius_m: 10.0,
            waypoints: vec![TransferWaypointSpec {
                id: "last".to_owned(),
                position_m: Vec2::new(7.0, 4.0),
                handoff_tangent_unit: Some(Vec2::new(1.0, 0.0)),
                capture_radius_m: 5.0,
                max_cross_track_m: 5.0,
                max_outbound_heading_error_rad: 0.5,
                min_outbound_progress_mps: 1.0,
                max_outbound_cross_speed_mps: Some(2.0),
                min_speed_mps: 2.0,
                max_speed_mps: 5.0,
                min_vertical_speed_mps: None,
                max_vertical_speed_mps: None,
            }],
        };
        let prefix = route_prefix_points(Vec2::new(0.0, 0.0), &route);
        // The same prefix is used by the coarse terrain-domain check; the
        // unscored target must not expand that check or the sampled witness.
        assert_eq!(prefix.last(), Some(&route.waypoints[0].position_m));
        assert_eq!(prefix.len(), route.waypoints.len() + 1);
    }
}
