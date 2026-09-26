//! Frozen, controller-free 24-cell topology sweep for the V2 analytical
//! direct-leg primitive.
//!
//! Each probe is derived from the passing continuous-flat primitive baseline.
//! Direct candidates are evaluated first. The finite generic one-waypoint
//! search runs only when the direct candidate family has no certificate.
//! Classifications describe this bounded analytical search and make no claim
//! about physical impossibility or controller performance.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::Vec2;
use pd_plan::conservative_ballistic_bridge::{
    CertificationV2, DirectBridgePolicyV2, DirectBridgeProbeV2, DirectBridgeReasonV2,
    VehicleInputV2, WaypointCandidateV2, WaypointSearchEvidenceV2, evaluate_direct_bridge_case_v2,
    evaluate_one_waypoint_case_v2, waypoint_position_step_v2,
};
use serde::{Deserialize, Serialize};

use crate::waypoint_direct_primitive_analytical::{
    CandidateEvidence, EarlyGateEvidence, build_artifact as build_primitive_baseline,
};

pub const WAYPOINT_DIRECT_TOPOLOGY_SWEEP_ID: &str = "waypoint-direct-topology-sweep";
pub const WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_ID: &str = "waypoint_direct_topology_sweep_v1";
pub const WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_VERSION: u32 = 1;

const CENTER_FRACTIONS: [f64; 3] = [0.35, 0.50, 0.65];
const BASE_WIDTH_FRACTIONS: [f64; 2] = [0.05, 0.25];
const HEIGHT_FRACTIONS: [f64; 4] = [0.10, 0.20, 0.30, 0.40];
const EXPECTED_PROBE_COUNT: usize = 24;
const EXPECTED_FLAT_SPAN_M: f64 = 800.0;
const MAX_WAYPOINT_CANDIDATES: usize = 64;
const FLAT_BASELINE_CASE_ID: &str = "continuous_flat_r00";

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectTopologySweepPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectTopologySweepRun {
    pub artifact: WaypointDirectTopologySweepArtifact,
    pub paths: WaypointDirectTopologySweepPaths,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectTopologySweepArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub sweep_id: String,
    pub protocol: TopologySweepProtocolEvidence,
    pub baseline_identity: String,
    pub baseline_input_identity: String,
    pub baseline_early_gate: EarlyGateEvidence,
    pub source_flat_probe_identity: String,
    pub source_flat_probe: DirectBridgeProbeV2,
    pub policy_identity: String,
    pub policy: DirectBridgePolicyV2,
    pub vehicle_identity: String,
    pub vehicle: VehicleInputV2,
    pub grid: NormalizedGridEvidence,
    pub cases: Vec<TopologyCellEvidence>,
    pub classification_counts: TopologyClassificationCounts,
    pub scope: TopologySweepScopeEvidence,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologySweepProtocolEvidence {
    pub evaluation_mode: String,
    pub direct_evaluator: String,
    pub waypoint_evaluator: String,
    pub candidate_duration_multipliers: Vec<f64>,
    pub direct_selection_rule: String,
    pub fallback_rule: String,
    pub waypoint_position_rule: String,
    pub waypoint_search_order: String,
    pub maximum_waypoint_candidates: usize,
    pub controller_called: bool,
    pub planner_called: bool,
    pub simulation_called: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedGridEvidence {
    pub source_target_span_m: f64,
    pub center_fractions: Vec<f64>,
    pub base_width_fractions: Vec<f64>,
    pub height_fractions: Vec<f64>,
    pub order: String,
    pub trapezoid_definition: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologyCellEvidence {
    pub id: String,
    pub parameters: TopologyCellParameters,
    pub probe_identity: String,
    pub probe: DirectBridgeProbeV2,
    pub policy_identity: String,
    pub vehicle_identity: String,
    pub direct_result_identity: Option<String>,
    pub direct_candidate_count: usize,
    pub direct_certified_candidate_count: usize,
    pub direct_candidates: Vec<CandidateEvidence>,
    pub direct_research_selection: Option<CandidateEvidence>,
    pub waypoint_fallback: Option<WaypointFallbackEvidence>,
    pub classification: TopologyClassification,
    /// Exact validation/evaluation error returned while validating the probe
    /// or the frozen waypoint positions. This is populated only for
    /// `invalid_setup` cells.
    pub setup_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologyCellParameters {
    pub center_fraction: f64,
    pub base_width_fraction: f64,
    pub height_fraction: f64,
    pub center_x_m: f64,
    pub base_width_m: f64,
    pub height_m: f64,
    pub obstacle_top_y_m: f64,
    pub base_left_x_m: f64,
    pub base_right_x_m: f64,
    pub top_left_x_m: f64,
    pub top_right_x_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointFallbackEvidence {
    pub position_step_m: Vec2,
    pub positions_m: Vec<Vec2>,
    pub search_identity: String,
    pub max_candidates: usize,
    pub waypoint_position_count: usize,
    pub leg_duration_pair_count: usize,
    pub candidate_count: usize,
    pub certified_candidate_count: usize,
    pub rejection_counts: Vec<(DirectBridgeReasonV2, usize)>,
    pub selected_candidate_identity: Option<String>,
    /// Present only when the bounded search returned a certified witness.
    pub selected_witness: Option<WaypointCandidateV2>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyClassification {
    AnalyticalDirect,
    AnalyticalOneWaypoint,
    Unknown,
    InvalidSetup,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyClassificationCounts {
    pub analytical_direct: usize,
    pub analytical_one_waypoint: usize,
    pub unknown: usize,
    pub invalid_setup: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologySweepScopeEvidence {
    pub claims: Vec<String>,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug)]
struct GridProbe {
    parameters: TopologyCellParameters,
    probe: DirectBridgeProbeV2,
}

/// Evaluate the frozen sweep and write a deterministic `summary.json`.
/// The passing direct primitive baseline is rebuilt from its maintained input
/// projection before any obstacle cell is evaluated.
pub fn run_waypoint_direct_topology_sweep(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
) -> Result<WaypointDirectTopologySweepRun> {
    let artifact = build_artifact(repo_root)?;
    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create waypoint direct topology sweep output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&summary_path, &summary_bytes).with_context(|| {
        format!(
            "failed to write waypoint direct topology sweep summary {}",
            summary_path.display()
        )
    })?;
    let reloaded: WaypointDirectTopologySweepArtifact = serde_json::from_slice(&summary_bytes)
        .context("failed to reload waypoint direct topology sweep summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes {
        bail!("waypoint direct topology sweep summary is not byte-stable after reload");
    }
    if artifact_identity(&reloaded)? != reloaded.identity {
        bail!("waypoint direct topology sweep semantic identity failed round-trip check");
    }

    Ok(WaypointDirectTopologySweepRun {
        artifact: reloaded,
        paths: WaypointDirectTopologySweepPaths {
            output_dir,
            summary_path,
        },
    })
}

/// Build the current frozen topology sweep without writing output or invoking
/// any controller. Controller comparisons use this as a source-validation
/// seam before they begin a run.
pub(crate) fn build_artifact(repo_root: &Path) -> Result<WaypointDirectTopologySweepArtifact> {
    let baseline = build_primitive_baseline(repo_root)?;
    if !baseline.early_gate.passed {
        bail!(
            "topology sweep requires the passing continuous-row early gate; failed cases: {:?}",
            baseline.early_gate.failed_case_ids
        );
    }
    let flat_index = baseline
        .probes
        .iter()
        .position(|probe| probe.id == FLAT_BASELINE_CASE_ID)
        .ok_or_else(|| anyhow!("primitive baseline is missing {FLAT_BASELINE_CASE_ID}"))?;
    let flat_probe = &baseline.probes[flat_index];
    let grid_probes = build_grid_probes(flat_probe)?;
    let policy = &baseline.evaluated_policy;
    let vehicle = &baseline.vehicle;
    let policy_identity = stable_digest(policy)?;
    let vehicle_identity = stable_digest(vehicle)?;

    let mut cases = Vec::with_capacity(grid_probes.len());
    for grid_probe in &grid_probes {
        cases.push(evaluate_cell(
            grid_probe,
            policy,
            vehicle,
            &policy_identity,
            &vehicle_identity,
        )?);
    }
    let classification_counts = count_classifications(&cases);
    let mut artifact = WaypointDirectTopologySweepArtifact {
        schema_id: WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_VERSION,
        sweep_id: WAYPOINT_DIRECT_TOPOLOGY_SWEEP_ID.to_owned(),
        protocol: TopologySweepProtocolEvidence {
            evaluation_mode: "controller_free_analytical_only".to_owned(),
            direct_evaluator:
                "pd_plan::conservative_ballistic_bridge::evaluate_direct_bridge_case_v2"
                    .to_owned(),
            waypoint_evaluator:
                "pd_plan::conservative_ballistic_bridge::evaluate_one_waypoint_case_v2".to_owned(),
            candidate_duration_multipliers: policy.duration_multipliers.clone(),
            direct_selection_rule:
                "if any direct candidate certifies, choose shortest total_time_s, then candidate identity".to_owned(),
            fallback_rule:
                "run one-waypoint search only if no direct candidate certifies; stop at first witness or exhaust the bounded search".to_owned(),
            waypoint_position_rule:
                "compute dx and dy from the shared rotated hull envelope and policy clearance; generate the four frozen left/right offsets; sort by x then y; never clip".to_owned(),
            waypoint_search_order:
                "four positions; complete duration-multiplier pairs ordered by total ballistic leg steps, then the two multipliers and step counts; stop at first certificate; maximum 64 candidates".to_owned(),
            maximum_waypoint_candidates: MAX_WAYPOINT_CANDIDATES,
            controller_called: false,
            planner_called: false,
            simulation_called: false,
        },
        baseline_identity: baseline.identity.clone(),
        baseline_input_identity: baseline.input.identity.clone(),
        baseline_early_gate: baseline.early_gate.clone(),
        source_flat_probe_identity: stable_digest(flat_probe)?,
        source_flat_probe: flat_probe.clone(),
        policy_identity,
        policy: policy.clone(),
        vehicle_identity,
        vehicle: vehicle.clone(),
        grid: NormalizedGridEvidence {
            source_target_span_m: flat_span_m(flat_probe)?,
            center_fractions: CENTER_FRACTIONS.to_vec(),
            base_width_fractions: BASE_WIDTH_FRACTIONS.to_vec(),
            height_fractions: HEIGHT_FRACTIONS.to_vec(),
            order: "center_fraction, then base_width_fraction, then height_fraction".to_owned(),
            trapezoid_definition:
                "base edges at center +/- base_width/2; top edges at center +/- base_width/4; top height is height_fraction times the flat source-target span above the flat baseline".to_owned(),
        },
        cases,
        classification_counts,
        scope: TopologySweepScopeEvidence {
            claims: vec![
                "The frozen 24-cell terrain grid was evaluated by the V2 direct bridge, with bounded one-waypoint fallback only after direct rejection.".to_owned(),
                "The reported topology class is the result of the declared finite analytical candidate families under the shared 90 s policy and vehicle.".to_owned(),
                "Direct certified candidates are selected by shortest total time, then identity.".to_owned(),
            ],
            non_claims: vec![
                "A rejected direct family or exhausted one-waypoint search does not establish physical impossibility.".to_owned(),
                "No production planner, controller, or simulator outcome is included.".to_owned(),
                "This analytical sweep does not promote a planner, route topology, controller behavior, or default.".to_owned(),
                "The 24 frozen probes do not establish general terrain-family coverage.".to_owned(),
            ],
        },
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;
    Ok(artifact)
}

fn build_grid_probes(flat_probe: &DirectBridgeProbeV2) -> Result<Vec<GridProbe>> {
    let span_m = flat_span_m(flat_probe)?;
    let source_y_m = flat_probe.source.surface_y_m;
    let target_y_m = flat_probe.target.surface_y_m;
    if (source_y_m - target_y_m).abs() > 1.0e-9 {
        bail!("topology sweep flat baseline source and target heights differ");
    }
    if flat_probe.terrain_points_m.len() != 6 {
        bail!(
            "topology sweep flat baseline must have exactly three source-side and three target-side terrain points"
        );
    }
    if flat_probe.terrain_points_m[..3]
        .iter()
        .any(|point| (point.y - source_y_m).abs() > 1.0e-9)
        || flat_probe.terrain_points_m[3..]
            .iter()
            .any(|point| (point.y - target_y_m).abs() > 1.0e-9)
    {
        bail!("topology sweep source or target pad/domain points are not flat");
    }

    let mut probes = Vec::with_capacity(EXPECTED_PROBE_COUNT);
    for center_fraction in CENTER_FRACTIONS {
        for base_width_fraction in BASE_WIDTH_FRACTIONS {
            for height_fraction in HEIGHT_FRACTIONS {
                let center_x_m = flat_probe.source.center_x_m + span_m * center_fraction;
                let base_width_m = span_m * base_width_fraction;
                let height_m = span_m * height_fraction;
                let obstacle_top_y_m = source_y_m + height_m;
                let base_left_x_m = center_x_m - base_width_m * 0.5;
                let base_right_x_m = center_x_m + base_width_m * 0.5;
                let top_left_x_m = center_x_m - base_width_m * 0.25;
                let top_right_x_m = center_x_m + base_width_m * 0.25;
                let id = probe_id(center_fraction, base_width_fraction, height_fraction);
                let mut terrain_points_m = Vec::with_capacity(10);
                terrain_points_m.extend_from_slice(&flat_probe.terrain_points_m[..3]);
                terrain_points_m.extend([
                    Vec2::new(base_left_x_m, source_y_m),
                    Vec2::new(top_left_x_m, obstacle_top_y_m),
                    Vec2::new(top_right_x_m, obstacle_top_y_m),
                    Vec2::new(base_right_x_m, source_y_m),
                ]);
                terrain_points_m.extend_from_slice(&flat_probe.terrain_points_m[3..]);
                let probe = DirectBridgeProbeV2 {
                    id: id.clone(),
                    source: flat_probe.source.clone(),
                    target: flat_probe.target.clone(),
                    terrain_points_m,
                    initial_position_m: flat_probe.initial_position_m,
                    initial_velocity_mps: flat_probe.initial_velocity_mps,
                };
                probes.push(GridProbe {
                    parameters: TopologyCellParameters {
                        center_fraction,
                        base_width_fraction,
                        height_fraction,
                        center_x_m,
                        base_width_m,
                        height_m,
                        obstacle_top_y_m,
                        base_left_x_m,
                        base_right_x_m,
                        top_left_x_m,
                        top_right_x_m,
                    },
                    probe,
                });
            }
        }
    }
    if probes.len() != EXPECTED_PROBE_COUNT {
        bail!(
            "frozen topology grid generated {} probes instead of {EXPECTED_PROBE_COUNT}",
            probes.len()
        );
    }
    Ok(probes)
}

fn evaluate_cell(
    grid_probe: &GridProbe,
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    policy_identity: &str,
    vehicle_identity: &str,
) -> Result<TopologyCellEvidence> {
    let probe = &grid_probe.probe;
    let base = cell_metadata(grid_probe, policy_identity, vehicle_identity)?;
    let result = match evaluate_direct_bridge_case_v2(policy, vehicle, probe) {
        Ok(result) => result,
        Err(error) => {
            return Ok(TopologyCellEvidence {
                classification: TopologyClassification::InvalidSetup,
                setup_error: Some(error),
                ..base
            });
        }
    };
    let direct_candidates = result
        .candidates
        .iter()
        .map(compact_candidate)
        .collect::<Vec<_>>();
    let direct_certified_candidate_count = result.certified_candidate_count;
    let direct_research_selection = result
        .candidates
        .iter()
        .filter(|candidate| candidate.classification == CertificationV2::Certified)
        .min_by(|left, right| {
            left.total_time_s
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.total_time_s.unwrap_or(f64::INFINITY))
                .then_with(|| left.identity.cmp(&right.identity))
        })
        .map(compact_candidate);
    let direct_base = TopologyCellEvidence {
        direct_result_identity: Some(result.identity),
        direct_candidate_count: direct_candidates.len(),
        direct_certified_candidate_count,
        direct_candidates,
        direct_research_selection,
        ..base
    };
    if direct_certified_candidate_count > 0 {
        return Ok(TopologyCellEvidence {
            classification: TopologyClassification::AnalyticalDirect,
            ..direct_base
        });
    }

    let step = match waypoint_position_step_v2(policy, vehicle) {
        Ok(step) => step,
        Err(error) => {
            return Ok(TopologyCellEvidence {
                classification: TopologyClassification::InvalidSetup,
                setup_error: Some(error),
                ..direct_base
            });
        }
    };
    let positions = match waypoint_positions(&grid_probe.parameters, step) {
        Ok(positions) => positions,
        Err(error) => {
            return Ok(TopologyCellEvidence {
                classification: TopologyClassification::InvalidSetup,
                setup_error: Some(error),
                ..direct_base
            });
        }
    };
    let search = match evaluate_one_waypoint_case_v2(policy, vehicle, probe, &positions) {
        Ok(search) => search,
        Err(error) => {
            return Ok(TopologyCellEvidence {
                classification: TopologyClassification::InvalidSetup,
                setup_error: Some(error),
                ..direct_base
            });
        }
    };
    if search.max_candidates > MAX_WAYPOINT_CANDIDATES
        || search.candidate_count > MAX_WAYPOINT_CANDIDATES
    {
        bail!(
            "one-waypoint API exceeded the frozen 64-candidate bound for {}: max={}, actual={}",
            probe.id,
            search.max_candidates,
            search.candidate_count
        );
    }
    let selected_witness = search
        .selected_candidate
        .as_ref()
        .filter(|candidate| candidate.classification == CertificationV2::Certified)
        .cloned();
    if search.certified_candidate_count > 0 && selected_witness.is_none() {
        bail!(
            "one-waypoint API reports a certified candidate without returning its witness for {}",
            probe.id
        );
    }
    if search.certified_candidate_count > 1 {
        bail!(
            "one-waypoint API returned more than the first certified witness for {}",
            probe.id
        );
    }
    let waypoint_fallback = waypoint_fallback_evidence(step, positions, &search, selected_witness);
    let classification = if waypoint_fallback.selected_witness.is_some() {
        TopologyClassification::AnalyticalOneWaypoint
    } else {
        TopologyClassification::Unknown
    };
    Ok(TopologyCellEvidence {
        waypoint_fallback: Some(waypoint_fallback),
        classification,
        ..direct_base
    })
}

fn cell_metadata(
    grid_probe: &GridProbe,
    policy_identity: &str,
    vehicle_identity: &str,
) -> Result<TopologyCellEvidence> {
    Ok(TopologyCellEvidence {
        id: grid_probe.probe.id.clone(),
        parameters: grid_probe.parameters.clone(),
        probe_identity: stable_digest(&grid_probe.probe)?,
        probe: grid_probe.probe.clone(),
        policy_identity: policy_identity.to_owned(),
        vehicle_identity: vehicle_identity.to_owned(),
        direct_result_identity: None,
        direct_candidate_count: 0,
        direct_certified_candidate_count: 0,
        direct_candidates: Vec::new(),
        direct_research_selection: None,
        waypoint_fallback: None,
        classification: TopologyClassification::Unknown,
        setup_error: None,
    })
}

fn waypoint_positions(
    parameters: &TopologyCellParameters,
    step: Vec2,
) -> Result<Vec<Vec2>, String> {
    if !step.x.is_finite() || !step.y.is_finite() || step.x <= 0.0 || step.y <= 0.0 {
        return Err("waypoint position step must be finite and positive on both axes".to_owned());
    }
    let mut positions = vec![
        Vec2::new(
            parameters.base_left_x_m - step.x,
            parameters.obstacle_top_y_m + step.y,
        ),
        Vec2::new(
            parameters.base_left_x_m - (2.0 * step.x),
            parameters.obstacle_top_y_m + (2.0 * step.y),
        ),
        Vec2::new(
            parameters.base_right_x_m + step.x,
            parameters.obstacle_top_y_m + step.y,
        ),
        Vec2::new(
            parameters.base_right_x_m + (2.0 * step.x),
            parameters.obstacle_top_y_m + (2.0 * step.y),
        ),
    ];
    if positions
        .iter()
        .any(|position| !position.x.is_finite() || !position.y.is_finite())
    {
        return Err("waypoint positions must be finite".to_owned());
    }
    positions.sort_by(|left, right| {
        left.x
            .total_cmp(&right.x)
            .then_with(|| left.y.total_cmp(&right.y))
    });
    if positions.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("waypoint positions contain a duplicate".to_owned());
    }
    Ok(positions)
}

fn waypoint_fallback_evidence(
    position_step_m: Vec2,
    positions_m: Vec<Vec2>,
    search: &WaypointSearchEvidenceV2,
    selected_witness: Option<WaypointCandidateV2>,
) -> WaypointFallbackEvidence {
    WaypointFallbackEvidence {
        position_step_m,
        positions_m,
        search_identity: search.identity.clone(),
        max_candidates: search.max_candidates,
        waypoint_position_count: search.waypoint_position_count,
        leg_duration_pair_count: search.leg_duration_pair_count,
        candidate_count: search.candidate_count,
        certified_candidate_count: search.certified_candidate_count,
        rejection_counts: search.rejection_counts.clone(),
        selected_candidate_identity: search.selected_candidate_identity.clone(),
        selected_witness,
    }
}

fn compact_candidate(
    candidate: &pd_plan::conservative_ballistic_bridge::DirectBridgeCandidateV2,
) -> CandidateEvidence {
    CandidateEvidence {
        identity: candidate.identity.clone(),
        classification: candidate.classification,
        reasons: candidate.reasons.clone(),
        minimum_normalized_margin: candidate.margins.minimum_normalized(),
        total_time_s: candidate.total_time_s,
        total_fuel_burn_kg: candidate.total_fuel_burn_kg,
    }
}

fn count_classifications(cases: &[TopologyCellEvidence]) -> TopologyClassificationCounts {
    let mut counts = TopologyClassificationCounts::default();
    for case in cases {
        match case.classification {
            TopologyClassification::AnalyticalDirect => counts.analytical_direct += 1,
            TopologyClassification::AnalyticalOneWaypoint => counts.analytical_one_waypoint += 1,
            TopologyClassification::Unknown => counts.unknown += 1,
            TopologyClassification::InvalidSetup => counts.invalid_setup += 1,
        }
    }
    counts
}

fn flat_span_m(probe: &DirectBridgeProbeV2) -> Result<f64> {
    let span_m = probe.target.center_x_m - probe.source.center_x_m;
    if !span_m.is_finite() || span_m <= 0.0 {
        bail!("topology sweep requires a positive finite source-to-target x span");
    }
    if (span_m - EXPECTED_FLAT_SPAN_M).abs() > 1.0e-9 {
        bail!(
            "topology sweep requires the frozen {EXPECTED_FLAT_SPAN_M} m flat source-target span, got {span_m} m"
        );
    }
    Ok(span_m)
}

fn probe_id(center_fraction: f64, width_fraction: f64, height_fraction: f64) -> String {
    format!(
        "center_{}_width_{}_height_{}",
        fraction_code(center_fraction),
        fraction_code(width_fraction),
        fraction_code(height_fraction)
    )
}

fn fraction_code(value: f64) -> String {
    format!("{:.2}", value).replace('.', "")
}

fn resolve_output_dir(repo_root: &Path, requested: Option<&Path>) -> PathBuf {
    requested
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                repo_root.join(path)
            }
        })
        .unwrap_or_else(|| {
            repo_root
                .join("outputs/setups")
                .join(WAYPOINT_DIRECT_TOPOLOGY_SWEEP_ID)
        })
}

fn stable_digest<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("fnv1a64:{hash:016x}"))
}

fn artifact_identity(artifact: &WaypointDirectTopologySweepArtifact) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waypoint_direct_primitive_analytical::{
        EarlyGateEvidence, WaypointDirectPrimitiveAnalyticalArtifact,
        build_artifact as build_primitive_baseline,
    };
    use pd_core::{Command, RunContext, SimulationState, TerrainDefinition};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval crate is under repository root")
            .to_path_buf()
    }

    #[test]
    fn selected_direct_profiles_have_measured_first_tick_slew_gate() {
        let baseline = build_primitive_baseline(&repo_root()).expect("baseline evaluates");
        let sweep = build_artifact(&repo_root()).expect("sweep evaluates");
        assert_eq!(baseline.identity, "fnv1a64:d3fa6b24336f7c05");
        assert_eq!(sweep.identity, "fnv1a64:1bcd5a3bd6c6da01");
        let selected = [
            (
                "continuous_flat_r00",
                "fnv1a64:4e6c0b23f9eb1b8f",
                0.011771414680,
                true,
            ),
            (
                "continuous_uphill_r+30",
                "fnv1a64:e24e67d8cfbb1e45",
                0.006045287907,
                true,
            ),
            (
                "continuous_downhill_r-30",
                "fnv1a64:974f4a0c5118e294",
                0.030261510909,
                false,
            ),
            (
                "center_050_width_025_height_020",
                "fnv1a64:e6cc4147423f0741",
                0.011771414680,
                true,
            ),
            (
                "center_050_width_025_height_030",
                "fnv1a64:f97e2d787a8cd3f2",
                0.009446454319,
                true,
            ),
            (
                "center_050_width_025_height_040",
                "fnv1a64:d68e2873ab6b7df8",
                0.006477374512,
                true,
            ),
        ];
        for (id, expected_identity, expected_angle, expected_reachable) in selected {
            let (policy, vehicle, probe, selected_identity) =
                if let Some(index) = baseline.cases.iter().position(|case| case.id == id) {
                    (
                        &baseline.evaluated_policy,
                        &baseline.vehicle,
                        &baseline.probes[index],
                        baseline.cases[index]
                            .research_selection
                            .as_ref()
                            .expect("baseline direct selection")
                            .identity
                            .as_str(),
                    )
                } else {
                    let cell = sweep
                        .cases
                        .iter()
                        .find(|cell| cell.id == id)
                        .expect("sweep cell exists");
                    (
                        &sweep.policy,
                        &sweep.vehicle,
                        &cell.probe,
                        cell.direct_research_selection
                            .as_ref()
                            .expect("sweep direct selection")
                            .identity
                            .as_str(),
                    )
                };
            let result = evaluate_direct_bridge_case_v2(policy, vehicle, probe)
                .expect("direct probe evaluates");
            if id == "continuous_downhill_r-30" {
                let alternative = result
                    .candidates
                    .iter()
                    .find(|candidate| {
                        candidate.classification == CertificationV2::Certified
                            && candidate.identity == "fnv1a64:8e9646e20efe007d"
                    })
                    .expect("downhill has the pre-existing later certified alternative");
                let sample = alternative
                    .source_bridge
                    .as_ref()
                    .expect("source bridge")
                    .samples
                    .iter()
                    .find(|sample| sample.thrust_direction_unit.is_some())
                    .expect("powered sample");
                let direction = sample.thrust_direction_unit.expect("direction");
                assert_eq!(sample.tick, 0);
                assert!(
                    direction.x.atan2(direction.y).abs()
                        < vehicle.max_rotation_rate_radps / policy.physics_hz as f64
                );
            }
            let candidate = result
                .candidates
                .iter()
                .find(|candidate| candidate.identity == selected_identity)
                .expect("selected candidate materializes");
            assert_eq!(selected_identity, expected_identity);
            assert_eq!(candidate.classification, CertificationV2::Certified);
            let source = candidate
                .source_bridge
                .as_ref()
                .expect("source bridge exists");
            let first = source
                .samples
                .iter()
                .find_map(|sample| {
                    sample.thrust_direction_unit.map(|direction| {
                        (
                            sample.tick,
                            direction,
                            sample.thrust_acceleration_mps2.length(),
                        )
                    })
                })
                .expect("source has powered sample");
            let angle = first.1.x.atan2(first.1.y).abs();
            assert!((angle - expected_angle).abs() < 1.0e-12, "{id}");
            let reachable =
                vehicle.max_rotation_rate_radps * (first.0 + 1) as f64 / policy.physics_hz as f64;
            assert_eq!(first.0, 0, "{id}: source first thrust is at tick zero");
            assert_eq!(angle <= reachable + 1.0e-12, expected_reachable, "{id}");
            let dt = 1.0 / policy.physics_hz as f64;
            let initial_mass = vehicle.dry_mass_kg + vehicle.initial_fuel_kg;
            let applied_throttle = first.2 * initial_mass
                / (vehicle.max_thrust_n + first.2 * vehicle.max_fuel_burn_kgps * dt);
            assert!(applied_throttle >= vehicle.min_throttle_frac, "{id}");
            assert!(applied_throttle <= 1.0, "{id}");
            if id == "continuous_downhill_r-30" {
                assert!((reachable - 0.013089969390).abs() < 1.0e-12);
                let scenario =
                    crate::waypoint_direct_characterization::controller_scenario_for_case(
                        &repo_root(),
                        id,
                    )
                    .expect("frozen downhill scenario exists");
                let context = RunContext::from_scenario(&scenario).expect("valid context");
                assert_eq!(context.sim.physics_hz, policy.physics_hz);
                assert_eq!(context.sim.controller_hz, 60);
                assert_eq!(context.sim.control_interval_steps(), 2);
                assert_eq!(
                    source.start_state.position_m,
                    context.initial_state.position_m
                );
                assert_eq!(
                    source.start_state.velocity_mps,
                    context.initial_state.velocity_mps
                );
                assert_eq!(context.initial_state.attitude_rad, 0.0);
                let mut plant = SimulationState::new(&context).expect("valid plant");
                plant.set_command(Command {
                    throttle_frac: (applied_throttle - vehicle.min_throttle_frac)
                        / (1.0 - vehicle.min_throttle_frac),
                    target_attitude_rad: first.1.x.atan2(first.1.y),
                });
                plant.step_physics_and_classify_contact(&context);
                let expected_fuel =
                    vehicle.initial_fuel_kg - applied_throttle * vehicle.max_fuel_burn_kgps * dt;
                assert!((plant.fuel_kg - expected_fuel).abs() < 1.0e-9);
                let realized_thrust_magnitude =
                    vehicle.max_thrust_n * applied_throttle / (vehicle.dry_mass_kg + plant.fuel_kg);
                assert!((realized_thrust_magnitude - first.2).abs() < 1.0e-12);
                assert!((plant.attitude_rad.abs() - reachable).abs() < 1.0e-12);
                assert!(plant.attitude_rad.abs() < angle);
                let reference = source.state_at(1);
                let position_error = (plant.position_m - reference.position_m).length();
                let velocity_error = (plant.velocity_mps - reference.velocity_mps).length();
                assert!(position_error > 1.0e-5);
                assert!(velocity_error > 1.0e-3);
            }
        }
    }

    fn flat_probe() -> (
        WaypointDirectPrimitiveAnalyticalArtifact,
        DirectBridgeProbeV2,
    ) {
        let baseline = build_primitive_baseline(&repo_root()).expect("frozen baseline evaluates");
        assert!(baseline.early_gate.passed);
        let probe = baseline
            .probes
            .iter()
            .find(|probe| probe.id == FLAT_BASELINE_CASE_ID)
            .expect("flat source probe exists")
            .clone();
        (baseline, probe)
    }

    #[test]
    fn grid_has_24_cells_in_position_width_height_order() {
        let (_, flat) = flat_probe();
        let probes = build_grid_probes(&flat).expect("frozen grid builds");
        assert_eq!(probes.len(), 24);
        assert_eq!(
            probes
                .iter()
                .take(5)
                .map(|entry| (
                    entry.parameters.center_fraction,
                    entry.parameters.base_width_fraction,
                    entry.parameters.height_fraction,
                ))
                .collect::<Vec<_>>(),
            vec![
                (0.35, 0.05, 0.10),
                (0.35, 0.05, 0.20),
                (0.35, 0.05, 0.30),
                (0.35, 0.05, 0.40),
                (0.35, 0.25, 0.10),
            ]
        );
        assert_eq!(
            probes.first().expect("first cell").probe.id,
            "center_035_width_005_height_010"
        );
        assert_eq!(
            probes.last().expect("last cell").probe.id,
            "center_065_width_025_height_040"
        );
    }

    #[test]
    fn grid_geometry_preserves_flat_pad_domain_points_and_inserts_exact_trapezoid() {
        let (_, flat) = flat_probe();
        let probes = build_grid_probes(&flat).expect("frozen grid builds");
        let span_m = flat.target.center_x_m - flat.source.center_x_m;
        for entry in &probes {
            let parameters = &entry.parameters;
            let terrain = &entry.probe.terrain_points_m;
            assert_eq!(terrain.len(), 10);
            assert_eq!(&terrain[..3], &flat.terrain_points_m[..3]);
            assert_eq!(&terrain[7..], &flat.terrain_points_m[3..]);
            assert_eq!(
                terrain[3].x,
                parameters.center_x_m - parameters.base_width_m / 2.0
            );
            assert_eq!(terrain[3].y, flat.source.surface_y_m);
            assert_eq!(
                terrain[4].x,
                parameters.center_x_m - parameters.base_width_m / 4.0
            );
            assert_eq!(terrain[4].y, parameters.obstacle_top_y_m);
            assert_eq!(
                terrain[5].x,
                parameters.center_x_m + parameters.base_width_m / 4.0
            );
            assert_eq!(terrain[5].y, parameters.obstacle_top_y_m);
            assert_eq!(
                terrain[6].x,
                parameters.center_x_m + parameters.base_width_m / 2.0
            );
            assert_eq!(terrain[6].y, flat.source.surface_y_m);
            assert_eq!(
                parameters.center_x_m,
                flat.source.center_x_m + span_m * parameters.center_fraction
            );
            assert_eq!(
                parameters.base_width_m,
                span_m * parameters.base_width_fraction
            );
            assert_eq!(parameters.height_m, span_m * parameters.height_fraction);
            TerrainDefinition::Heightfield {
                points_m: terrain.clone(),
            }
            .validate()
            .expect("grid terrain remains a valid increasing heightfield");
        }
    }

    #[test]
    fn all_grid_cells_are_stamped_with_the_same_policy_and_vehicle() {
        let (baseline, flat) = flat_probe();
        let probes = build_grid_probes(&flat).expect("frozen grid builds");
        let policy_identity = stable_digest(&baseline.evaluated_policy).expect("policy digest");
        let vehicle_identity = stable_digest(&baseline.vehicle).expect("vehicle digest");
        let cells = probes
            .iter()
            .map(|probe| cell_metadata(probe, &policy_identity, &vehicle_identity))
            .collect::<Result<Vec<_>>>()
            .expect("cell metadata builds");
        assert_eq!(cells.len(), EXPECTED_PROBE_COUNT);
        assert!(
            cells
                .iter()
                .all(|cell| cell.policy_identity == policy_identity)
        );
        assert!(
            cells
                .iter()
                .all(|cell| cell.vehicle_identity == vehicle_identity)
        );
        assert_eq!(baseline.evaluated_policy.maximum_mission_time_s, 90.0);
    }

    #[test]
    fn direct_certificate_prevents_any_waypoint_search() {
        let (baseline, mut flat) = flat_probe();
        flat.id = "direct_priority_flat_probe_test".to_owned();
        let grid_probe = GridProbe {
            parameters: TopologyCellParameters {
                center_fraction: 0.50,
                base_width_fraction: 0.05,
                height_fraction: 0.10,
                center_x_m: -400.0,
                base_width_m: 40.0,
                height_m: 80.0,
                obstacle_top_y_m: 80.0,
                base_left_x_m: -420.0,
                base_right_x_m: -380.0,
                top_left_x_m: -410.0,
                top_right_x_m: -390.0,
            },
            probe: flat,
        };
        let policy_identity = stable_digest(&baseline.evaluated_policy).expect("policy digest");
        let vehicle_identity = stable_digest(&baseline.vehicle).expect("vehicle digest");
        let cell = evaluate_cell(
            &grid_probe,
            &baseline.evaluated_policy,
            &baseline.vehicle,
            &policy_identity,
            &vehicle_identity,
        )
        .expect("direct analysis succeeds");
        assert!(cell.direct_certified_candidate_count > 0);
        assert_eq!(
            cell.classification,
            TopologyClassification::AnalyticalDirect
        );
        assert!(cell.waypoint_fallback.is_none());
        assert_eq!(cell.policy_identity, policy_identity);
        assert_eq!(cell.vehicle_identity, vehicle_identity);
    }

    #[test]
    fn invalid_probe_retains_the_exact_validation_error() {
        let (baseline, mut flat) = flat_probe();
        flat.terrain_points_m[1].x = flat.terrain_points_m[0].x;
        let grid_probe = GridProbe {
            parameters: TopologyCellParameters {
                center_fraction: 0.35,
                base_width_fraction: 0.05,
                height_fraction: 0.10,
                center_x_m: -520.0,
                base_width_m: 40.0,
                height_m: 80.0,
                obstacle_top_y_m: 80.0,
                base_left_x_m: -540.0,
                base_right_x_m: -500.0,
                top_left_x_m: -530.0,
                top_right_x_m: -510.0,
            },
            probe: flat,
        };
        let policy_identity = stable_digest(&baseline.evaluated_policy).expect("policy digest");
        let vehicle_identity = stable_digest(&baseline.vehicle).expect("vehicle digest");
        let cell = evaluate_cell(
            &grid_probe,
            &baseline.evaluated_policy,
            &baseline.vehicle,
            &policy_identity,
            &vehicle_identity,
        )
        .expect("validation rejection is recorded as a cell outcome");
        assert_eq!(cell.classification, TopologyClassification::InvalidSetup);
        assert_eq!(
            cell.setup_error.as_deref(),
            Some("heightfield points must be strictly increasing in x")
        );
        assert!(cell.waypoint_fallback.is_none());
    }

    #[test]
    fn waypoint_positions_are_sorted_and_duplicate_positions_are_invalid() {
        let parameters = TopologyCellParameters {
            center_fraction: 0.35,
            base_width_fraction: 0.05,
            height_fraction: 0.10,
            center_x_m: -520.0,
            base_width_m: 40.0,
            height_m: 80.0,
            obstacle_top_y_m: 80.0,
            base_left_x_m: -540.0,
            base_right_x_m: -500.0,
            top_left_x_m: -530.0,
            top_right_x_m: -510.0,
        };
        let positions = waypoint_positions(&parameters, Vec2::new(24.0, 36.0))
            .expect("four finite, unique positions");
        assert_eq!(positions.len(), 4);
        assert_eq!(positions[0], Vec2::new(-588.0, 152.0));
        assert_eq!(positions[1], Vec2::new(-564.0, 116.0));
        assert_eq!(positions[2], Vec2::new(-476.0, 116.0));
        assert_eq!(positions[3], Vec2::new(-452.0, 152.0));
        assert!(positions.windows(2).all(|pair| {
            pair[0].x < pair[1].x || (pair[0].x == pair[1].x && pair[0].y < pair[1].y)
        }));
        let duplicate_parameters = TopologyCellParameters {
            base_left_x_m: 30.0,
            base_right_x_m: -18.0,
            ..parameters
        };
        assert_eq!(
            waypoint_positions(&duplicate_parameters, Vec2::new(24.0, 36.0))
                .expect_err("overlapping left/right offsets are not clipped"),
            "waypoint positions contain a duplicate"
        );
    }

    #[test]
    fn artifact_identity_is_deterministic_and_json_roundtrips() {
        let (baseline, flat) = flat_probe();
        let artifact = WaypointDirectTopologySweepArtifact {
            schema_id: WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_ID.to_owned(),
            schema_version: WAYPOINT_DIRECT_TOPOLOGY_SWEEP_SCHEMA_VERSION,
            sweep_id: WAYPOINT_DIRECT_TOPOLOGY_SWEEP_ID.to_owned(),
            protocol: TopologySweepProtocolEvidence {
                evaluation_mode: "controller_free_analytical_only".to_owned(),
                direct_evaluator: "direct".to_owned(),
                waypoint_evaluator: "waypoint".to_owned(),
                candidate_duration_multipliers: baseline
                    .evaluated_policy
                    .duration_multipliers
                    .clone(),
                direct_selection_rule: "shortest then identity".to_owned(),
                fallback_rule: "only after direct rejection".to_owned(),
                waypoint_position_rule: "four frozen offsets".to_owned(),
                waypoint_search_order: "bounded".to_owned(),
                maximum_waypoint_candidates: MAX_WAYPOINT_CANDIDATES,
                controller_called: false,
                planner_called: false,
                simulation_called: false,
            },
            baseline_identity: baseline.identity,
            baseline_input_identity: baseline.input.identity,
            baseline_early_gate: EarlyGateEvidence {
                status: "passed".to_owned(),
                passed: true,
                required_case_ids: Vec::new(),
                case_certification: Vec::new(),
                failed_case_ids: Vec::new(),
                consequence: String::new(),
                reason: String::new(),
            },
            source_flat_probe_identity: stable_digest(&flat).expect("probe digest"),
            source_flat_probe: flat,
            policy_identity: stable_digest(&baseline.evaluated_policy).expect("policy digest"),
            policy: baseline.evaluated_policy,
            vehicle_identity: stable_digest(&baseline.vehicle).expect("vehicle digest"),
            vehicle: baseline.vehicle,
            grid: NormalizedGridEvidence {
                source_target_span_m: 800.0,
                center_fractions: CENTER_FRACTIONS.to_vec(),
                base_width_fractions: BASE_WIDTH_FRACTIONS.to_vec(),
                height_fractions: HEIGHT_FRACTIONS.to_vec(),
                order: "center, width, height".to_owned(),
                trapezoid_definition: "frozen".to_owned(),
            },
            cases: Vec::new(),
            classification_counts: TopologyClassificationCounts::default(),
            scope: TopologySweepScopeEvidence {
                claims: Vec::new(),
                non_claims: Vec::new(),
            },
            identity: String::new(),
        };
        assert_eq!(
            artifact_identity(&artifact).expect("identity computes"),
            artifact_identity(&artifact).expect("identity recomputes deterministically")
        );
        let bytes = serde_json::to_vec_pretty(&artifact).expect("artifact serializes");
        let roundtrip: WaypointDirectTopologySweepArtifact =
            serde_json::from_slice(&bytes).expect("artifact reloads");
        assert_eq!(roundtrip, artifact);
    }
}
