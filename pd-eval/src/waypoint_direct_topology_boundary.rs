//! Frozen direct-first V2 boundary decision gate.
//!
//! This evaluator is setup-only: it checks the maintained direct baseline,
//! then evaluates the frozen obstacle boundary grid with waypoint fallback
//! only after every direct duration candidate is rejected.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::Vec2;
use pd_plan::conservative_ballistic_bridge::{
    CertificationV2, ComponentMarginsV2, DirectBridgePolicyV2, DirectBridgeProbeV2,
    DirectBridgeReasonV2, VehicleInputV2, WaypointCandidateV2, WaypointSearchEvidenceV2,
    evaluate_direct_bridge_case_v2, evaluate_one_waypoint_case_v2, waypoint_position_step_v2,
};
use serde::{Deserialize, Serialize};

use crate::{
    waypoint_direct_primitive_analytical::build_artifact as build_primitive_baseline,
    waypoint_direct_topology_sweep::{
        TopologyClassification, build_artifact as build_prior_topology_sweep,
    },
};

pub const WAYPOINT_DIRECT_TOPOLOGY_BOUNDARY_ID: &str = "waypoint-direct-topology-boundary";
pub const WAYPOINT_DIRECT_TOPOLOGY_BOUNDARY_SCHEMA_ID: &str =
    "waypoint_direct_topology_boundary_v1";
pub const WAYPOINT_DIRECT_TOPOLOGY_BOUNDARY_SCHEMA_VERSION: u32 = 1;

const EXPECTED_BASELINE_IDENTITY: &str = "fnv1a64:d3fa6b24336f7c05";
const EXPECTED_PRIOR_SWEEP_IDENTITY: &str = "fnv1a64:1bcd5a3bd6c6da01";
const EXPECTED_SPAN_M: f64 = 800.0;
const CENTER_FRACTIONS: [f64; 3] = [0.35, 0.50, 0.65];
const HEIGHT_FRACTIONS: [f64; 4] = [0.45, 0.50, 0.55, 0.60];
const BOUNDARY_WIDTH_FRACTION: f64 = 0.25;
const MAX_WAYPOINT_CANDIDATES: usize = 64;
const CONTINUOUS_CASE_IDS: [&str; 3] = [
    "continuous_flat_r00",
    "continuous_uphill_r+30",
    "continuous_downhill_r-30",
];
const FLAT_CASE_ID: &str = "continuous_flat_r00";

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectTopologyBoundaryPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectTopologyBoundaryRun {
    pub artifact: WaypointDirectTopologyBoundaryArtifact,
    pub paths: WaypointDirectTopologyBoundaryPaths,
}

/// Path-neutral, identity-bound summary for the frozen analytical gate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectTopologyBoundaryArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub evaluator_id: String,
    pub protocol: BoundaryProtocolEvidence,
    pub input: BoundaryInputEvidence,
    pub input_gate: BoundaryInputGateEvidence,
    pub baseline_cases: Vec<BoundaryCaseEvidence>,
    pub overflight_control: Option<BoundaryCaseEvidence>,
    pub overflight_prior_cell_match: Option<PriorOverflightCellMatchEvidence>,
    pub boundary_grid: BoundaryGridEvidence,
    pub boundary_cells: Vec<BoundaryCaseEvidence>,
    pub out_of_envelope_control: Option<BoundaryCaseEvidence>,
    pub boundary_classification_counts: BoundaryClassificationCounts,
    pub first_waypoint_witness_id: Option<String>,
    pub first_witness_transition: Option<FlatTwinTransitionEvidence>,
    pub first_gate: BoundaryFirstGateEvidence,
    pub scope: BoundaryScopeEvidence,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryProtocolEvidence {
    pub evaluation_mode: String,
    pub direct_evaluator: String,
    pub waypoint_evaluator: String,
    pub direct_selection_rule: String,
    pub fallback_rule: String,
    pub ordered_cases: String,
    pub controller_called: bool,
    pub planner_called: bool,
    pub simulation_called: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryInputEvidence {
    pub expected_primitive_baseline_identity: String,
    pub observed_primitive_baseline_identity: String,
    pub expected_prior_sweep_identity: String,
    pub observed_prior_sweep_identity: Option<String>,
    pub prior_sweep_rebuild_error: Option<String>,
    pub flat_probe_identity: Option<String>,
    pub policy_identity: Option<String>,
    pub vehicle_identity: Option<String>,
    pub span_m: Option<f64>,
    pub duration_multipliers: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryInputGateEvidence {
    pub passed: bool,
    pub checks: Vec<BoundaryGateCheck>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryGateCheck {
    pub id: String,
    pub passed: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryGridEvidence {
    pub source_target_span_m: f64,
    pub center_fractions: Vec<f64>,
    pub base_width_fraction: f64,
    pub height_fractions: Vec<f64>,
    pub order: String,
    pub trapezoid_definition: String,
    pub waypoint_position_rule: String,
    pub maximum_waypoint_candidates: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryCaseEvidence {
    pub id: String,
    pub role: String,
    pub parameters: Option<BoundaryCaseParameters>,
    pub probe_identity: String,
    pub probe: DirectBridgeProbeV2,
    pub policy_identity: String,
    pub vehicle_identity: String,
    pub direct_result_identity: Option<String>,
    pub direct_candidate_count: usize,
    pub direct_certified_candidate_count: usize,
    pub direct_candidates: Vec<DirectCandidateEvidence>,
    pub selected_shortest_certified_candidate_identity: Option<String>,
    pub waypoint_fallback: Option<BoundaryWaypointEvidence>,
    pub classification: TopologyClassification,
    pub setup_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryCaseParameters {
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
pub struct DirectCandidateEvidence {
    pub duration_multiplier: f64,
    pub arc_steps: u64,
    pub apex_step: u64,
    pub apex_position_m: Vec2,
    pub identity: String,
    pub classification: CertificationV2,
    pub reasons: Vec<DirectBridgeReasonV2>,
    pub margins: ComponentMarginsV2,
    pub source_bridge_attempt_count: usize,
    pub terminal_bridge_attempt_count: usize,
    pub source_environment_rejection_count: usize,
    pub terminal_environment_rejection_count: usize,
    pub total_time_s: Option<f64>,
    pub total_fuel_burn_kg: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryWaypointEvidence {
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
    pub selected_witness: Option<WaypointCandidateV2>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryClassificationCounts {
    pub analytical_direct: usize,
    pub analytical_one_waypoint: usize,
    pub unknown: usize,
    pub invalid_setup: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PriorOverflightCellMatchEvidence {
    pub prior_sweep_cell_id: String,
    pub prior_probe_identity: String,
    pub current_probe_identity: String,
    pub probe_identity_matches: bool,
    pub prior_classification: TopologyClassification,
    pub current_classification: TopologyClassification,
    pub classification_matches: bool,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatTwinTransitionEvidence {
    pub compared_flat_probe_identity: String,
    pub witness_probe_identity: String,
    pub transitions: Vec<DurationCandidateTransitionEvidence>,
    pub qualifies: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DurationCandidateTransitionEvidence {
    pub duration_multiplier: f64,
    pub flat_candidate_identity: Option<String>,
    pub flat_certified: bool,
    pub terrain_candidate_identity: Option<String>,
    pub terrain_classification: Option<CertificationV2>,
    pub terrain_reasons: Vec<DirectBridgeReasonV2>,
    pub terminal_environment_rejection_count: usize,
    pub terrain_linked_rejection: bool,
}

#[derive(Clone, Debug)]
struct DurationCandidateScreen {
    duration_multiplier: f64,
    identity: String,
    classification: CertificationV2,
    reasons: Vec<DirectBridgeReasonV2>,
    terminal_environment_rejection_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryFirstGateEvidence {
    pub passed: bool,
    pub checks: Vec<BoundaryGateCheck>,
    pub failure_reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryScopeEvidence {
    pub claims: Vec<String>,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug)]
struct ObstacleProbe {
    parameters: BoundaryCaseParameters,
    probe: DirectBridgeProbeV2,
}

struct DirectEvaluationContext<'a> {
    policy: &'a DirectBridgePolicyV2,
    vehicle: &'a VehicleInputV2,
    policy_identity: &'a str,
    vehicle_identity: &'a str,
}

/// Rebuild the accepted primitive baseline and the accepted prior topology
/// sweep, then evaluate the frozen controls and boundary grid without writing.
pub(crate) fn build_artifact(repo_root: &Path) -> Result<WaypointDirectTopologyBoundaryArtifact> {
    let baseline = build_primitive_baseline(repo_root)?;
    let prior_sweep_result = build_prior_topology_sweep(repo_root);
    let prior_sweep_identity = prior_sweep_result
        .as_ref()
        .ok()
        .map(|sweep| sweep.identity.clone());
    let prior_sweep_rebuild_error = prior_sweep_result.as_ref().err().map(ToString::to_string);
    let baseline_identity_matches = baseline.identity == EXPECTED_BASELINE_IDENTITY;
    let prior_sweep_identity_matches =
        prior_sweep_identity.as_deref() == Some(EXPECTED_PRIOR_SWEEP_IDENTITY);
    let baseline_continuous_rows_pass = CONTINUOUS_CASE_IDS.iter().all(|id| {
        baseline
            .cases
            .iter()
            .find(|case| case.id == *id)
            .is_some_and(|case| case.certified_candidate_count > 0)
    });
    let input_gate = BoundaryInputGateEvidence {
        passed: baseline_identity_matches && prior_sweep_identity_matches,
        checks: vec![
            gate_check(
                "primitive_baseline_identity",
                baseline_identity_matches,
                if baseline_identity_matches {
                    format!("accepted identity {} rebuilt", baseline.identity)
                } else {
                    format!(
                        "expected {EXPECTED_BASELINE_IDENTITY}, observed {}",
                        baseline.identity
                    )
                },
            ),
            gate_check(
                "prior_topology_sweep_identity",
                prior_sweep_identity_matches,
                if prior_sweep_identity_matches {
                    format!(
                        "accepted identity {} rebuilt",
                        prior_sweep_identity.as_deref().unwrap_or_default()
                    )
                } else {
                    prior_sweep_rebuild_error.as_ref().map_or_else(
                        || {
                            format!(
                                "expected {EXPECTED_PRIOR_SWEEP_IDENTITY}, observed {}",
                                prior_sweep_identity.as_deref().unwrap_or("missing")
                            )
                        },
                        |error| format!("prior sweep rebuild failed: {error}"),
                    )
                },
            ),
        ],
    };

    let flat_probe = baseline
        .probes
        .iter()
        .find(|probe| probe.id == FLAT_CASE_ID)
        .cloned()
        .ok_or_else(|| anyhow!("primitive baseline is missing {FLAT_CASE_ID}"))?;
    let span = flat_span_m(&flat_probe)?;
    let policy = baseline.evaluated_policy.clone();
    let vehicle = baseline.vehicle.clone();
    let policy_identity = stable_digest(&policy)?;
    let vehicle_identity = stable_digest(&vehicle)?;
    let evaluation = DirectEvaluationContext {
        policy: &policy,
        vehicle: &vehicle,
        policy_identity: &policy_identity,
        vehicle_identity: &vehicle_identity,
    };
    let input = BoundaryInputEvidence {
        expected_primitive_baseline_identity: EXPECTED_BASELINE_IDENTITY.to_owned(),
        observed_primitive_baseline_identity: baseline.identity.clone(),
        expected_prior_sweep_identity: EXPECTED_PRIOR_SWEEP_IDENTITY.to_owned(),
        observed_prior_sweep_identity: prior_sweep_identity,
        prior_sweep_rebuild_error,
        flat_probe_identity: Some(stable_digest(&flat_probe)?),
        policy_identity: Some(policy_identity.clone()),
        vehicle_identity: Some(vehicle_identity.clone()),
        span_m: Some(span),
        duration_multipliers: policy.duration_multipliers.clone(),
    };

    let mut artifact = empty_artifact(input, input_gate);
    if !artifact.input_gate.passed {
        artifact.first_gate =
            skipped_first_gate("pinned input identity gate failed; terrain evaluation skipped");
        artifact.identity = artifact_identity(&artifact)?;
        return Ok(artifact);
    }

    let mut baseline_cases = Vec::with_capacity(CONTINUOUS_CASE_IDS.len());
    for id in CONTINUOUS_CASE_IDS {
        let probe = baseline
            .probes
            .iter()
            .find(|probe| probe.id == id)
            .ok_or_else(|| anyhow!("primitive baseline is missing {id}"))?;
        baseline_cases.push(evaluate_probe(
            probe.clone(),
            "uncut_baseline",
            None,
            &evaluation,
            false,
        ));
    }
    let flat_result =
        evaluate_direct_bridge_case_v2(evaluation.policy, evaluation.vehicle, &flat_probe)
            .map_err(|error| anyhow!("accepted flat baseline direct evaluation failed: {error}"))?;
    let flat_candidates = flat_result
        .candidates
        .iter()
        .map(direct_candidate_evidence)
        .map(|candidate| screen_candidate(&candidate))
        .collect::<Vec<_>>();
    let overflight_probe = build_obstacle_probe(&flat_probe, 0.50, 0.25, 0.20)?;
    let overflight_control = evaluate_probe(
        overflight_probe.probe,
        "overflight_positive_control",
        Some(overflight_probe.parameters),
        &evaluation,
        true,
    );
    let overflight_prior_cell_match = prior_sweep_result
        .as_ref()
        .ok()
        .and_then(|sweep| {
            sweep
                .cases
                .iter()
                .find(|case| case.id == overflight_control.id)
        })
        .map(|prior| {
            let probe_identity_matches = prior.probe_identity == overflight_control.probe_identity;
            let classification_matches = prior.classification == overflight_control.classification;
            PriorOverflightCellMatchEvidence {
                prior_sweep_cell_id: prior.id.clone(),
                prior_probe_identity: prior.probe_identity.clone(),
                current_probe_identity: overflight_control.probe_identity.clone(),
                probe_identity_matches,
                prior_classification: prior.classification,
                current_classification: overflight_control.classification,
                classification_matches,
                passed: probe_identity_matches && classification_matches,
            }
        });

    let mut boundary_cells = Vec::with_capacity(12);
    for center_fraction in CENTER_FRACTIONS {
        for height_fraction in HEIGHT_FRACTIONS {
            let obstacle = build_obstacle_probe(
                &flat_probe,
                center_fraction,
                BOUNDARY_WIDTH_FRACTION,
                height_fraction,
            )?;
            boundary_cells.push(evaluate_probe(
                obstacle.probe,
                "boundary_grid",
                Some(obstacle.parameters),
                &evaluation,
                true,
            ));
        }
    }
    let out_obstacle = build_obstacle_probe(&flat_probe, 0.50, 0.50, 2.00)?;
    let out_of_envelope_control = evaluate_probe(
        out_obstacle.probe,
        "out_of_envelope_diagnostic_control",
        Some(out_obstacle.parameters),
        &evaluation,
        true,
    );

    let first_witness = boundary_cells
        .iter()
        .find(|cell| cell.classification == TopologyClassification::AnalyticalOneWaypoint);
    let first_witness_transition = if let Some(witness) = first_witness {
        let terrain_candidates = witness
            .direct_candidates
            .iter()
            .map(screen_candidate)
            .collect::<Vec<_>>();
        Some(compare_flat_twin_candidates(
            &flat_probe,
            &flat_candidates,
            &terrain_candidates,
            &witness.probe_identity,
            &policy.duration_multipliers,
        )?)
    } else {
        None
    };
    let first_waypoint_witness_id = first_witness.map(|case| case.id.clone());
    let boundary_classification_counts = count_classifications(&boundary_cells);
    artifact.protocol = protocol_evidence(&policy);
    artifact.baseline_cases = baseline_cases;
    artifact.overflight_control = Some(overflight_control);
    artifact.overflight_prior_cell_match = overflight_prior_cell_match;
    artifact.boundary_grid = grid_evidence(span);
    artifact.boundary_cells = boundary_cells;
    artifact.out_of_envelope_control = Some(out_of_envelope_control);
    artifact.boundary_classification_counts = boundary_classification_counts;
    artifact.first_waypoint_witness_id = first_waypoint_witness_id;
    artifact.first_witness_transition = first_witness_transition;
    artifact.first_gate = first_gate(
        baseline_continuous_rows_pass,
        &artifact.baseline_cases,
        artifact.overflight_control.as_ref(),
        artifact.overflight_prior_cell_match.as_ref(),
        &artifact.boundary_cells,
        artifact.first_witness_transition.as_ref(),
        artifact.out_of_envelope_control.as_ref(),
    );
    artifact.identity = artifact_identity(&artifact)?;
    Ok(artifact)
}

pub fn run_waypoint_direct_topology_boundary(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
) -> Result<WaypointDirectTopologyBoundaryRun> {
    let artifact = build_artifact(repo_root)?;
    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create waypoint direct topology boundary output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    let (computed_identity, summary_bytes) = artifact_identity_and_roundtrip(&artifact)?;
    if computed_identity != artifact.identity {
        bail!("boundary artifact identity changed before persistence");
    }
    write_create_only(&summary_path, &summary_bytes)?;
    let reloaded: WaypointDirectTopologyBoundaryArtifact =
        serde_json::from_slice(&summary_bytes).context("failed to reload boundary summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes {
        bail!("boundary summary is not byte-stable after JSON round-trip");
    }
    if artifact_identity(&reloaded)? != reloaded.identity {
        bail!("boundary summary semantic identity failed round-trip check");
    }
    Ok(WaypointDirectTopologyBoundaryRun {
        artifact: reloaded,
        paths: WaypointDirectTopologyBoundaryPaths {
            output_dir,
            summary_path,
        },
    })
}

fn empty_artifact(
    input: BoundaryInputEvidence,
    input_gate: BoundaryInputGateEvidence,
) -> WaypointDirectTopologyBoundaryArtifact {
    let span = input.span_m.unwrap_or(EXPECTED_SPAN_M);
    WaypointDirectTopologyBoundaryArtifact {
        schema_id: WAYPOINT_DIRECT_TOPOLOGY_BOUNDARY_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_TOPOLOGY_BOUNDARY_SCHEMA_VERSION,
        evaluator_id: WAYPOINT_DIRECT_TOPOLOGY_BOUNDARY_ID.to_owned(),
        protocol: BoundaryProtocolEvidence {
            evaluation_mode: "controller_free_analytical_only".to_owned(),
            direct_evaluator:
                "pd_plan::conservative_ballistic_bridge::evaluate_direct_bridge_case_v2"
                    .to_owned(),
            waypoint_evaluator:
                "pd_plan::conservative_ballistic_bridge::evaluate_one_waypoint_case_v2"
                    .to_owned(),
            direct_selection_rule:
                "evaluate all four V2 duration candidates; if any certifies, choose shortest total_time_s then stable candidate identity".to_owned(),
            fallback_rule:
                "invoke the four-position one-waypoint search only when direct_certified_candidate_count is zero; retain its first witness or finite exhaustion".to_owned(),
            ordered_cases:
                "three uncut controls; centered-wide 160 m overflight control; boundary cells ordered center fraction then height fraction; out-of-envelope control".to_owned(),
            controller_called: false,
            planner_called: false,
            simulation_called: false,
        },
        input,
        input_gate,
        baseline_cases: Vec::new(),
        overflight_control: None,
        overflight_prior_cell_match: None,
        boundary_grid: grid_evidence(span),
        boundary_cells: Vec::new(),
        out_of_envelope_control: None,
        boundary_classification_counts: BoundaryClassificationCounts {
            analytical_direct: 0,
            analytical_one_waypoint: 0,
            unknown: 0,
            invalid_setup: 0,
        },
        first_waypoint_witness_id: None,
        first_witness_transition: None,
        first_gate: skipped_first_gate("analysis not run"),
        scope: scope_evidence(),
        identity: String::new(),
    }
}

fn evaluate_probe(
    probe: DirectBridgeProbeV2,
    role: &str,
    parameters: Option<BoundaryCaseParameters>,
    evaluation: &DirectEvaluationContext<'_>,
    allow_waypoint_fallback: bool,
) -> BoundaryCaseEvidence {
    let policy = evaluation.policy;
    let vehicle = evaluation.vehicle;
    let policy_identity = evaluation.policy_identity;
    let vehicle_identity = evaluation.vehicle_identity;
    let probe_identity = match stable_digest(&probe) {
        Ok(identity) => identity,
        Err(error) => {
            return BoundaryCaseEvidence {
                id: probe.id.clone(),
                role: role.to_owned(),
                parameters,
                probe_identity: String::new(),
                probe,
                policy_identity: policy_identity.to_owned(),
                vehicle_identity: vehicle_identity.to_owned(),
                direct_result_identity: None,
                direct_candidate_count: 0,
                direct_certified_candidate_count: 0,
                direct_candidates: Vec::new(),
                selected_shortest_certified_candidate_identity: None,
                waypoint_fallback: None,
                classification: TopologyClassification::InvalidSetup,
                setup_error: Some(format!("failed to identify probe: {error}")),
            };
        }
    };
    let base = BoundaryCaseEvidence {
        id: probe.id.clone(),
        role: role.to_owned(),
        parameters,
        probe_identity,
        probe: probe.clone(),
        policy_identity: policy_identity.to_owned(),
        vehicle_identity: vehicle_identity.to_owned(),
        direct_result_identity: None,
        direct_candidate_count: 0,
        direct_certified_candidate_count: 0,
        direct_candidates: Vec::new(),
        selected_shortest_certified_candidate_identity: None,
        waypoint_fallback: None,
        classification: TopologyClassification::Unknown,
        setup_error: None,
    };
    let result = match evaluate_direct_bridge_case_v2(policy, vehicle, &probe) {
        Ok(result) => result,
        Err(error) => {
            return BoundaryCaseEvidence {
                classification: TopologyClassification::InvalidSetup,
                setup_error: Some(error),
                ..base
            };
        }
    };
    let direct_candidates = result
        .candidates
        .iter()
        .map(direct_candidate_evidence)
        .collect::<Vec<_>>();
    let selected_shortest_certified_candidate_identity = result
        .candidates
        .iter()
        .filter(|candidate| candidate.classification == CertificationV2::Certified)
        .min_by(|left, right| {
            left.total_time_s
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.total_time_s.unwrap_or(f64::INFINITY))
                .then_with(|| left.identity.cmp(&right.identity))
        })
        .map(|candidate| candidate.identity.clone());
    let direct_base = BoundaryCaseEvidence {
        direct_result_identity: Some(result.identity),
        direct_candidate_count: direct_candidates.len(),
        direct_certified_candidate_count: result.certified_candidate_count,
        direct_candidates,
        selected_shortest_certified_candidate_identity,
        ..base
    };
    if !waypoint_fallback_required(result.certified_candidate_count) {
        return BoundaryCaseEvidence {
            classification: TopologyClassification::AnalyticalDirect,
            ..direct_base
        };
    }
    if !allow_waypoint_fallback {
        return BoundaryCaseEvidence {
            classification: TopologyClassification::Unknown,
            ..direct_base
        };
    }
    let step = match waypoint_position_step_v2(policy, vehicle) {
        Ok(step) => step,
        Err(error) => {
            return BoundaryCaseEvidence {
                classification: TopologyClassification::InvalidSetup,
                setup_error: Some(error),
                ..direct_base
            };
        }
    };
    let Some(parameters) = direct_base.parameters.as_ref() else {
        return BoundaryCaseEvidence {
            classification: TopologyClassification::InvalidSetup,
            setup_error: Some("waypoint fallback requires frozen obstacle parameters".to_owned()),
            ..direct_base
        };
    };
    let positions = match waypoint_positions(parameters, step) {
        Ok(positions) => positions,
        Err(error) => {
            return BoundaryCaseEvidence {
                classification: TopologyClassification::InvalidSetup,
                setup_error: Some(error),
                ..direct_base
            };
        }
    };
    let search = match evaluate_one_waypoint_case_v2(policy, vehicle, &probe, &positions) {
        Ok(search) => search,
        Err(error) => {
            return BoundaryCaseEvidence {
                classification: TopologyClassification::InvalidSetup,
                setup_error: Some(error),
                ..direct_base
            };
        }
    };
    if search.max_candidates > MAX_WAYPOINT_CANDIDATES
        || search.candidate_count > MAX_WAYPOINT_CANDIDATES
        || search.certified_candidate_count > 1
    {
        return BoundaryCaseEvidence {
            classification: TopologyClassification::InvalidSetup,
            setup_error: Some(format!(
                "one-waypoint API violated frozen bound: max={}, actual={}, certified={}",
                search.max_candidates, search.candidate_count, search.certified_candidate_count
            )),
            ..direct_base
        };
    }
    let selected_witness = search
        .selected_candidate
        .as_ref()
        .filter(|candidate| candidate.classification == CertificationV2::Certified)
        .cloned();
    if search.certified_candidate_count > 0 && selected_witness.is_none() {
        return BoundaryCaseEvidence {
            classification: TopologyClassification::InvalidSetup,
            setup_error: Some(
                "one-waypoint API returned a certificate count without a witness".to_owned(),
            ),
            ..direct_base
        };
    }
    let waypoint_fallback = waypoint_evidence(step, positions, &search, selected_witness);
    let classification = if waypoint_fallback.selected_witness.is_some() {
        TopologyClassification::AnalyticalOneWaypoint
    } else {
        TopologyClassification::Unknown
    };
    BoundaryCaseEvidence {
        waypoint_fallback: Some(waypoint_fallback),
        classification,
        ..direct_base
    }
}

fn direct_candidate_evidence(
    candidate: &pd_plan::conservative_ballistic_bridge::DirectBridgeCandidateV2,
) -> DirectCandidateEvidence {
    DirectCandidateEvidence {
        duration_multiplier: candidate.duration_multiplier,
        arc_steps: candidate.virtual_arc.steps,
        apex_step: candidate.virtual_arc.apex_step,
        apex_position_m: candidate.virtual_arc.apex_position_m,
        identity: candidate.identity.clone(),
        classification: candidate.classification,
        reasons: candidate.reasons.clone(),
        margins: candidate.margins,
        source_bridge_attempt_count: candidate.source_bridge_attempt_count,
        terminal_bridge_attempt_count: candidate.terminal_bridge_attempt_count,
        source_environment_rejection_count: candidate.source_environment_rejection_count,
        terminal_environment_rejection_count: candidate.terminal_environment_rejection_count,
        total_time_s: candidate.total_time_s,
        total_fuel_burn_kg: candidate.total_fuel_burn_kg,
    }
}

fn screen_candidate(candidate: &DirectCandidateEvidence) -> DurationCandidateScreen {
    DurationCandidateScreen {
        duration_multiplier: candidate.duration_multiplier,
        identity: candidate.identity.clone(),
        classification: candidate.classification,
        reasons: candidate.reasons.clone(),
        terminal_environment_rejection_count: candidate.terminal_environment_rejection_count,
    }
}

fn build_obstacle_probe(
    flat_probe: &DirectBridgeProbeV2,
    center_fraction: f64,
    base_width_fraction: f64,
    height_fraction: f64,
) -> Result<ObstacleProbe> {
    let span = flat_span_m(flat_probe)?;
    if flat_probe.terrain_points_m.len() != 6 {
        bail!("frozen flat probe must have exactly six terrain points");
    }
    let source_y = flat_probe.source.surface_y_m;
    if (flat_probe.target.surface_y_m - source_y).abs() > 1.0e-9
        || flat_probe
            .terrain_points_m
            .iter()
            .any(|point| (point.y - source_y).abs() > 1.0e-9)
    {
        bail!("frozen flat probe must have flat source, target, and terrain");
    }
    let center_x_m = flat_probe.source.center_x_m + span * center_fraction;
    let base_width_m = span * base_width_fraction;
    let height_m = span * height_fraction;
    let obstacle_top_y_m = source_y + height_m;
    let base_left_x_m = center_x_m - base_width_m * 0.5;
    let base_right_x_m = center_x_m + base_width_m * 0.5;
    let top_left_x_m = center_x_m - base_width_m * 0.25;
    let top_right_x_m = center_x_m + base_width_m * 0.25;
    let mut terrain_points_m = Vec::with_capacity(10);
    terrain_points_m.extend_from_slice(&flat_probe.terrain_points_m[..3]);
    terrain_points_m.extend([
        Vec2::new(base_left_x_m, source_y),
        Vec2::new(top_left_x_m, obstacle_top_y_m),
        Vec2::new(top_right_x_m, obstacle_top_y_m),
        Vec2::new(base_right_x_m, source_y),
    ]);
    terrain_points_m.extend_from_slice(&flat_probe.terrain_points_m[3..]);
    let probe = DirectBridgeProbeV2 {
        id: obstacle_id(center_fraction, base_width_fraction, height_fraction),
        source: flat_probe.source.clone(),
        target: flat_probe.target.clone(),
        terrain_points_m,
        initial_position_m: flat_probe.initial_position_m,
        initial_velocity_mps: flat_probe.initial_velocity_mps,
    };
    Ok(ObstacleProbe {
        parameters: BoundaryCaseParameters {
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
    })
}

fn waypoint_positions(
    parameters: &BoundaryCaseParameters,
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

fn waypoint_evidence(
    position_step_m: Vec2,
    positions_m: Vec<Vec2>,
    search: &WaypointSearchEvidenceV2,
    selected_witness: Option<WaypointCandidateV2>,
) -> BoundaryWaypointEvidence {
    BoundaryWaypointEvidence {
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

fn compare_flat_twin_candidates(
    flat_probe: &DirectBridgeProbeV2,
    flat_candidates: &[DurationCandidateScreen],
    terrain_candidates: &[DurationCandidateScreen],
    witness_probe_identity: &str,
    duration_multipliers: &[f64],
) -> Result<FlatTwinTransitionEvidence> {
    let transitions = duration_multipliers
        .iter()
        .copied()
        .map(|duration_multiplier| {
            let flat = flat_candidates.iter().find(|candidate| {
                (candidate.duration_multiplier - duration_multiplier).abs() <= 1.0e-12
            });
            let terrain = terrain_candidates.iter().find(|candidate| {
                (candidate.duration_multiplier - duration_multiplier).abs() <= 1.0e-12
            });
            let flat_certified = flat
                .is_some_and(|candidate| candidate.classification == CertificationV2::Certified);
            let terrain_classification = terrain.map(|candidate| candidate.classification);
            let terrain_reasons =
                terrain.map_or_else(Vec::new, |candidate| candidate.reasons.clone());
            let terminal_environment_rejection_count = terrain.map_or(0, |candidate| {
                candidate.terminal_environment_rejection_count
            });
            let terrain_linked_rejection = flat_certified
                && terrain_classification == Some(CertificationV2::NotCertified)
                && (terrain_reasons.contains(&DirectBridgeReasonV2::TerrainClearance)
                    || terminal_environment_rejection_count > 0);
            DurationCandidateTransitionEvidence {
                duration_multiplier,
                flat_candidate_identity: flat.map(|candidate| candidate.identity.clone()),
                flat_certified,
                terrain_candidate_identity: terrain.map(|candidate| candidate.identity.clone()),
                terrain_classification,
                terrain_reasons,
                terminal_environment_rejection_count,
                terrain_linked_rejection,
            }
        })
        .collect::<Vec<_>>();
    let qualifies = transitions
        .iter()
        .any(|transition| transition.terrain_linked_rejection);
    Ok(FlatTwinTransitionEvidence {
        compared_flat_probe_identity: stable_digest(flat_probe)?,
        witness_probe_identity: witness_probe_identity.to_owned(),
        transitions,
        qualifies,
        reason: if qualifies {
            "at least one flat-certified duration multiplier rejects on witness terrain with terrain clearance or terminal-environment rejection evidence".to_owned()
        } else {
            "no flat-certified duration multiplier has the protocol-required terrain-linked rejection evidence".to_owned()
        },
    })
}

fn first_gate(
    baseline_rows_pass: bool,
    baseline_cases: &[BoundaryCaseEvidence],
    overflight_control: Option<&BoundaryCaseEvidence>,
    overflight_prior_cell_match: Option<&PriorOverflightCellMatchEvidence>,
    boundary_cells: &[BoundaryCaseEvidence],
    first_witness_transition: Option<&FlatTwinTransitionEvidence>,
    out_of_envelope_control: Option<&BoundaryCaseEvidence>,
) -> BoundaryFirstGateEvidence {
    let baseline_all_direct = baseline_rows_pass
        && CONTINUOUS_CASE_IDS.iter().all(|id| {
            baseline_cases
                .iter()
                .find(|case| case.id == *id)
                .is_some_and(|case| case.classification == TopologyClassification::AnalyticalDirect)
        });
    let overflight_is_direct = overflight_control
        .is_some_and(|case| case.classification == TopologyClassification::AnalyticalDirect);
    let overflight_matches_prior_cell =
        overflight_prior_cell_match.is_some_and(|comparison| comparison.passed);
    let at_least_one_waypoint = boundary_cells
        .iter()
        .any(|case| case.classification == TopologyClassification::AnalyticalOneWaypoint);
    let first_witness_has_terrain_link =
        first_witness_transition.is_some_and(|evidence| evidence.qualifies);
    let out_control_is_valid_unknown = out_of_envelope_control.is_some_and(|case| {
        case.setup_error.is_none() && case.classification == TopologyClassification::Unknown
    });
    let checks = vec![
        gate_check(
            "uncut_baselines_and_overflight_direct",
            baseline_all_direct && overflight_is_direct,
            if baseline_all_direct && overflight_is_direct {
                "all three uncut controls and the centered-wide 160 m control are analytical_direct"
                    .to_owned()
            } else {
                "at least one uncut baseline or the overflight control is not analytical_direct"
                    .to_owned()
            },
        ),
        gate_check(
            "overflight_matches_prior_sweep_cell",
            overflight_matches_prior_cell,
            overflight_prior_cell_match.map_or_else(
                || "accepted prior sweep does not contain the overflight control cell".to_owned(),
                |comparison| {
                    format!(
                        "probe_identity_matches={}, classification_matches={} (prior={:?}, current={:?})",
                        comparison.probe_identity_matches,
                        comparison.classification_matches,
                        comparison.prior_classification,
                        comparison.current_classification
                    )
                },
            ),
        ),
        gate_check(
            "boundary_waypoint_witness_exists",
            at_least_one_waypoint,
            if at_least_one_waypoint {
                "at least one frozen boundary cell is analytical_one_waypoint".to_owned()
            } else {
                "no frozen boundary cell is analytical_one_waypoint".to_owned()
            },
        ),
        gate_check(
            "first_witness_has_flat_to_terrain_rejection",
            first_witness_has_terrain_link,
            first_witness_transition.map_or_else(
                || "no first witness exists to compare with the flat twin".to_owned(),
                |evidence| evidence.reason.clone(),
            ),
        ),
        gate_check(
            "out_of_envelope_control_is_unknown",
            out_control_is_valid_unknown,
            if out_control_is_valid_unknown {
                "out-of-envelope control is valid and unknown after bounded search".to_owned()
            } else {
                "out-of-envelope control certified a route or failed setup".to_owned()
            },
        ),
    ];
    let failure_reasons = checks
        .iter()
        .filter(|check| !check.passed)
        .map(|check| format!("{}: {}", check.id, check.reason))
        .collect::<Vec<_>>();
    BoundaryFirstGateEvidence {
        passed: failure_reasons.is_empty(),
        checks,
        failure_reasons,
    }
}

fn skipped_first_gate(reason: &str) -> BoundaryFirstGateEvidence {
    let checks = vec![gate_check("analysis", false, reason.to_owned())];
    BoundaryFirstGateEvidence {
        passed: false,
        failure_reasons: vec![format!("analysis: {reason}")],
        checks,
    }
}

fn gate_check(id: &str, passed: bool, reason: String) -> BoundaryGateCheck {
    BoundaryGateCheck {
        id: id.to_owned(),
        passed,
        reason,
    }
}

fn count_classifications(cases: &[BoundaryCaseEvidence]) -> BoundaryClassificationCounts {
    let mut counts = BoundaryClassificationCounts {
        analytical_direct: 0,
        analytical_one_waypoint: 0,
        unknown: 0,
        invalid_setup: 0,
    };
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

fn waypoint_fallback_required(direct_certified_candidate_count: usize) -> bool {
    direct_certified_candidate_count == 0
}

fn grid_evidence(span_m: f64) -> BoundaryGridEvidence {
    BoundaryGridEvidence {
        source_target_span_m: span_m,
        center_fractions: CENTER_FRACTIONS.to_vec(),
        base_width_fraction: BOUNDARY_WIDTH_FRACTION,
        height_fractions: HEIGHT_FRACTIONS.to_vec(),
        order: "center_fraction, then height_fraction".to_owned(),
        trapezoid_definition:
            "base edges at center +/- base_width/2; top edges at center +/- base_width/4; top height is height_fraction times the flat source-target span above the flat baseline".to_owned(),
        waypoint_position_rule:
            "generate (L-dx,T+dy), (L-2dx,T+2dy), (R+dx,T+dy), (R+2dx,T+2dy); sort by x then y; never clip".to_owned(),
        maximum_waypoint_candidates: MAX_WAYPOINT_CANDIDATES,
    }
}

fn protocol_evidence(policy: &DirectBridgePolicyV2) -> BoundaryProtocolEvidence {
    BoundaryProtocolEvidence {
        evaluation_mode: "controller_free_analytical_only".to_owned(),
        direct_evaluator:
            "pd_plan::conservative_ballistic_bridge::evaluate_direct_bridge_case_v2".to_owned(),
        waypoint_evaluator:
            "pd_plan::conservative_ballistic_bridge::evaluate_one_waypoint_case_v2".to_owned(),
        direct_selection_rule:
            "if any of all four V2 duration candidates certifies, choose shortest total_time_s then candidate identity".to_owned(),
        fallback_rule:
            "run bounded one-waypoint search only after zero direct certificates; retain the first witness or record finite exhaustion as unknown".to_owned(),
        ordered_cases: format!(
            "three uncut controls; overflight control; twelve center-major, height-minor cells; out-of-envelope control; duration multipliers {:?}",
            policy.duration_multipliers
        ),
        controller_called: false,
        planner_called: false,
        simulation_called: false,
    }
}

fn scope_evidence() -> BoundaryScopeEvidence {
    BoundaryScopeEvidence {
        claims: vec![
            "The frozen baseline and obstacle probes were evaluated with the unchanged V2 direct candidate policy and direct-first bounded waypoint fallback.".to_owned(),
            "The reported topology class is the result of the declared finite analytical candidate families.".to_owned(),
            "The first waypoint witness is selected only by the predeclared center-major, height-minor order.".to_owned(),
        ],
        non_claims: vec![
            "A not_certified_under_policy or unknown result does not establish physical impossibility.".to_owned(),
            "V2 certificates are not controller command streams and do not prove a controller landing.".to_owned(),
            "A direct-negative/waypoint-positive result is bounded to this finite V2 family, not all possible direct ballistic routes.".to_owned(),
            "No controller, production planner, simulator, V1 behavior, F6 path, or default selection is changed or invoked.".to_owned(),
        ],
    }
}

fn flat_span_m(probe: &DirectBridgeProbeV2) -> Result<f64> {
    let span = probe.target.center_x_m - probe.source.center_x_m;
    if !span.is_finite() || (span - EXPECTED_SPAN_M).abs() > 1.0e-9 {
        bail!("boundary evaluator requires the frozen {EXPECTED_SPAN_M} m span, got {span}");
    }
    Ok(span)
}

fn obstacle_id(center_fraction: f64, width_fraction: f64, height_fraction: f64) -> String {
    format!(
        "center_{}_width_{}_height_{}",
        fraction_code(center_fraction),
        fraction_code(width_fraction),
        fraction_code(height_fraction)
    )
}

fn fraction_code(value: f64) -> String {
    format!("{value:.2}").replace('.', "")
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

fn artifact_identity(artifact: &WaypointDirectTopologyBoundaryArtifact) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
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
                .join(WAYPOINT_DIRECT_TOPOLOGY_BOUNDARY_ID)
        })
}

fn write_create_only(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("refusing to overwrite existing summary {}", path.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("failed to write summary {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync summary {}", path.display()))?;
    Ok(())
}

fn artifact_identity_and_roundtrip(
    artifact: &WaypointDirectTopologyBoundaryArtifact,
) -> Result<(String, Vec<u8>)> {
    let identity = artifact_identity(artifact)?;
    let mut identified = artifact.clone();
    identified.identity = identity.clone();
    let bytes = serde_json::to_vec_pretty(&identified)?;
    let reloaded: WaypointDirectTopologyBoundaryArtifact = serde_json::from_slice(&bytes)?;
    if serde_json::to_vec_pretty(&reloaded)? != bytes {
        bail!("boundary artifact is not byte-stable after JSON round-trip");
    }
    if artifact_identity(&reloaded)? != reloaded.identity {
        bail!("boundary artifact semantic identity mismatch");
    }
    Ok((identity, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn flat_probe() -> DirectBridgeProbeV2 {
        DirectBridgeProbeV2 {
            id: FLAT_CASE_ID.to_owned(),
            source: pd_plan::conservative_ballistic_bridge::PadInputV2 {
                center_x_m: 18.0,
                surface_y_m: 0.0,
                width_m: 36.0,
            },
            target: pd_plan::conservative_ballistic_bridge::PadInputV2 {
                center_x_m: 818.0,
                surface_y_m: 0.0,
                width_m: 36.0,
            },
            terrain_points_m: vec![
                Vec2::new(-40.0, 0.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(36.0, 0.0),
                Vec2::new(800.0, 0.0),
                Vec2::new(818.0, 0.0),
                Vec2::new(840.0, 0.0),
            ],
            initial_position_m: Vec2::new(18.0, 5.0),
            initial_velocity_mps: Vec2::new(0.0, 0.0),
        }
    }

    fn parameters() -> BoundaryCaseParameters {
        BoundaryCaseParameters {
            center_fraction: 0.35,
            base_width_fraction: 0.25,
            height_fraction: 0.45,
            center_x_m: 298.0,
            base_width_m: 200.0,
            height_m: 360.0,
            obstacle_top_y_m: 360.0,
            base_left_x_m: 198.0,
            base_right_x_m: 398.0,
            top_left_x_m: 248.0,
            top_right_x_m: 348.0,
        }
    }

    fn screen(
        identity: &str,
        classification: CertificationV2,
        reasons: Vec<DirectBridgeReasonV2>,
        terminal_environment_rejection_count: usize,
    ) -> DurationCandidateScreen {
        DurationCandidateScreen {
            duration_multiplier: 1.0,
            identity: identity.to_owned(),
            classification,
            reasons,
            terminal_environment_rejection_count,
        }
    }

    #[test]
    fn boundary_grid_has_twelve_cells_in_frozen_center_then_height_order() {
        let flat = flat_probe();
        let mut cells = Vec::new();
        for center in CENTER_FRACTIONS {
            for height in HEIGHT_FRACTIONS {
                cells.push(build_obstacle_probe(&flat, center, 0.25, height).unwrap());
            }
        }
        assert_eq!(cells.len(), 12);
        assert_eq!(cells[0].probe.id, "center_035_width_025_height_045");
        assert_eq!(cells[3].probe.id, "center_035_width_025_height_060");
        assert_eq!(cells[4].probe.id, "center_050_width_025_height_045");
        assert_eq!(cells[11].probe.id, "center_065_width_025_height_060");
        for cell in cells {
            assert_eq!(cell.parameters.base_width_m, 200.0);
            assert_eq!(cell.probe.terrain_points_m.len(), 10);
            let points = &cell.probe.terrain_points_m;
            assert_eq!(points[3].x, cell.parameters.center_x_m - 100.0);
            assert_eq!(points[4].x, cell.parameters.center_x_m - 50.0);
            assert_eq!(points[5].x, cell.parameters.center_x_m + 50.0);
            assert_eq!(points[6].x, cell.parameters.center_x_m + 100.0);
        }
    }

    #[test]
    fn waypoint_positions_use_frozen_offsets_and_sort_without_clipping() {
        let positions = waypoint_positions(&parameters(), Vec2::new(24.0, 36.0)).unwrap();
        assert_eq!(positions.len(), 4);
        assert_eq!(positions[0], Vec2::new(150.0, 432.0));
        assert_eq!(positions[1], Vec2::new(174.0, 396.0));
        assert_eq!(positions[2], Vec2::new(422.0, 396.0));
        assert_eq!(positions[3], Vec2::new(446.0, 432.0));
    }

    #[test]
    fn waypoint_fallback_is_skipped_when_direct_certificate_exists() {
        let mut called = false;
        if waypoint_fallback_required(1) {
            called = true;
        }
        assert!(!called);
        assert!(waypoint_fallback_required(0));
    }

    #[test]
    fn flat_twin_transition_gate_requires_terrain_linked_direct_rejection() {
        let flat = screen("flat-candidate", CertificationV2::Certified, Vec::new(), 0);
        let terrain = screen(
            "terrain-candidate",
            CertificationV2::NotCertified,
            vec![DirectBridgeReasonV2::TerrainClearance],
            0,
        );
        let transition = compare_flat_twin_candidates(
            &flat_probe(),
            std::slice::from_ref(&flat),
            &[terrain],
            "witness-probe",
            &[1.0],
        )
        .unwrap();
        assert!(transition.qualifies);
        assert!(transition.transitions[0].terrain_linked_rejection);

        let terminal_rejection = screen(
            "terminal-rejection",
            CertificationV2::NotCertified,
            vec![DirectBridgeReasonV2::SourceAttitude],
            1,
        );
        let transition = compare_flat_twin_candidates(
            &flat_probe(),
            std::slice::from_ref(&flat),
            &[terminal_rejection],
            "witness-probe",
            &[1.0],
        )
        .unwrap();
        assert!(transition.qualifies);

        let unrelated_rejection = screen(
            "unrelated-rejection",
            CertificationV2::NotCertified,
            vec![DirectBridgeReasonV2::SourceAttitude],
            0,
        );
        let transition = compare_flat_twin_candidates(
            &flat_probe(),
            &[flat],
            &[unrelated_rejection],
            "witness-probe",
            &[1.0],
        )
        .unwrap();
        assert!(!transition.qualifies);
    }

    #[test]
    fn summary_identity_roundtrips_and_create_only_refuses_existing_file() {
        let input = BoundaryInputEvidence {
            expected_primitive_baseline_identity: EXPECTED_BASELINE_IDENTITY.to_owned(),
            observed_primitive_baseline_identity: EXPECTED_BASELINE_IDENTITY.to_owned(),
            expected_prior_sweep_identity: EXPECTED_PRIOR_SWEEP_IDENTITY.to_owned(),
            observed_prior_sweep_identity: Some(EXPECTED_PRIOR_SWEEP_IDENTITY.to_owned()),
            prior_sweep_rebuild_error: None,
            flat_probe_identity: None,
            policy_identity: None,
            vehicle_identity: None,
            span_m: Some(EXPECTED_SPAN_M),
            duration_multipliers: Vec::new(),
        };
        let gate = BoundaryInputGateEvidence {
            passed: true,
            checks: Vec::new(),
        };
        let mut artifact = empty_artifact(input, gate);
        let (identity, bytes) = artifact_identity_and_roundtrip(&artifact).unwrap();
        artifact.identity = identity.clone();
        let reloaded: WaypointDirectTopologyBoundaryArtifact =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(reloaded.identity, identity);
        assert_eq!(artifact_identity(&reloaded).unwrap(), identity);

        let unique = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "pd-waypoint-direct-boundary-create-only-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("summary.json");
        fs::write(&path, b"existing").unwrap();
        assert!(write_create_only(&path, &bytes).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"existing");
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&dir).unwrap();
    }
}
