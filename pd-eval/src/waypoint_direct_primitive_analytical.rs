//! Controller-free V2 primitive baseline over the existing five direct-route
//! waypoint characterization inputs.
//!
//! This is an opt-in research gate. It reuses the characterization fixture
//! projection, evaluates each input with the additive direct-bridge V2 API,
//! and records whether the three continuous rows each have a certified direct
//! candidate. A failed gate is evidence and does not mean physical
//! impossibility.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::{LandingPadSpec, VehicleSpec};
use pd_plan::conservative_ballistic_bridge::{
    CertificationV2, DirectBridgeCandidateV2, DirectBridgeFixtureV2, DirectBridgePolicyV2,
    DirectBridgeProbeResultV2, DirectBridgeProbeV2, DirectBridgeReasonV2, PadInputV2,
    VehicleGeometryInputV2, VehicleInputV2, evaluate_direct_bridge_case_v2,
    load_embedded_fixture_v2,
};
use serde::{Deserialize, Serialize};

use crate::waypoint_direct_characterization::project_waypoint_direct_primitive_inputs;

pub const WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_ID: &str = "waypoint-direct-primitive-analytical";
pub const WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_SCHEMA_ID: &str =
    "waypoint_direct_primitive_analytical_v1";
pub const WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_SCHEMA_VERSION: u32 = 1;

const SOURCE_SCENARIO_PATH: &str = "fixtures/scenarios/flat_terminal_descent.json";
const COMMON_MAX_TIME_S: f64 = 90.0;
const EXPECTED_CASE_IDS: [&str; 5] = [
    "continuous_flat_r00",
    "continuous_uphill_r+30",
    "continuous_downhill_r-30",
    "bounded_narrow_ridge",
    "bounded_broad_mesa",
];
const REQUIRED_CONTINUOUS_CASE_IDS: [&str; 3] = [
    "continuous_flat_r00",
    "continuous_uphill_r+30",
    "continuous_downhill_r-30",
];

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectPrimitiveAnalyticalPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectPrimitiveAnalyticalRun {
    pub artifact: WaypointDirectPrimitiveAnalyticalArtifact,
    pub paths: WaypointDirectPrimitiveAnalyticalPaths,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectPrimitiveAnalyticalArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub protocol: ProtocolEvidence,
    pub input: InputEvidence,
    pub embedded_policy: DirectBridgePolicyV2,
    pub evaluated_policy: DirectBridgePolicyV2,
    pub vehicle: VehicleInputV2,
    pub probes: Vec<DirectBridgeProbeV2>,
    pub cases: Vec<CaseEvidence>,
    pub early_gate: EarlyGateEvidence,
    pub scope: PrimitiveScopeEvidence,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProtocolEvidence {
    pub evaluator: String,
    pub evaluation_mode: String,
    pub policy_copy_rule: String,
    pub changed_policy_fields: Vec<String>,
    pub controller_called: bool,
    pub planner_called: bool,
    pub simulation_called: bool,
    pub v2_selected_candidate_role: String,
    pub research_selection_rule: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputEvidence {
    pub source_scenario_path: String,
    pub source_characterization_fixture_identity: String,
    pub physics_hz: u32,
    pub gravity_mps2: f64,
    pub common_max_time_s: f64,
    pub supported_source_attitude_rad: f64,
    pub supported_source_angular_rate_radps: f64,
    pub ordered_case_ids: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseEvidence {
    pub id: String,
    pub input_identity: String,
    pub result_identity: String,
    pub certified_candidate_count: usize,
    /// The identity selected by the V2 evaluator's native diagnostic ranking.
    pub v2_selected_candidate_identity: String,
    pub v2_selected_candidate: CandidateEvidence,
    /// Independent research choice: shortest certified candidate, with
    /// candidate identity as the deterministic tie-breaker.
    pub research_selection: Option<CandidateEvidence>,
    pub candidates: Vec<CandidateEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateEvidence {
    pub identity: String,
    pub classification: CertificationV2,
    pub reasons: Vec<DirectBridgeReasonV2>,
    pub minimum_normalized_margin: f64,
    pub total_time_s: Option<f64>,
    pub total_fuel_burn_kg: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EarlyGateEvidence {
    pub status: String,
    pub passed: bool,
    pub required_case_ids: Vec<String>,
    pub case_certification: Vec<ContinuousCaseGateEvidence>,
    pub failed_case_ids: Vec<String>,
    pub consequence: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContinuousCaseGateEvidence {
    pub id: String,
    pub has_certified_direct_candidate: bool,
    pub certified_candidate_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrimitiveScopeEvidence {
    pub claims: Vec<String>,
    pub non_claims: Vec<String>,
}

/// Evaluate all five existing characterization cases and write a deterministic
/// controller-free `summary.json`. Gate failure is retained as evidence and
/// does not turn the command into an execution error.
pub fn run_waypoint_direct_primitive_analytical(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
) -> Result<WaypointDirectPrimitiveAnalyticalRun> {
    let artifact = build_artifact(repo_root)?;
    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create direct primitive analytical output directory {}",
            output_dir.display()
        )
    })?;

    let summary_path = output_dir.join("summary.json");
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&summary_path, &summary_bytes).with_context(|| {
        format!(
            "failed to write direct primitive analytical summary {}",
            summary_path.display()
        )
    })?;
    let reloaded: WaypointDirectPrimitiveAnalyticalArtifact =
        serde_json::from_slice(&summary_bytes)
            .context("failed to reload direct primitive analytical summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes {
        bail!("direct primitive analytical summary is not byte-stable after reload");
    }
    if artifact_identity(&reloaded)? != reloaded.identity {
        bail!("direct primitive analytical semantic identity failed round-trip check");
    }

    Ok(WaypointDirectPrimitiveAnalyticalRun {
        artifact: reloaded,
        paths: WaypointDirectPrimitiveAnalyticalPaths {
            output_dir,
            summary_path,
        },
    })
}

pub(crate) fn build_artifact(
    repo_root: &Path,
) -> Result<WaypointDirectPrimitiveAnalyticalArtifact> {
    let frozen: DirectBridgeFixtureV2 = load_embedded_fixture_v2();
    let projection = project_waypoint_direct_primitive_inputs(repo_root)?;
    let ids = projection
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<Vec<_>>();
    if ids != EXPECTED_CASE_IDS {
        bail!("characterization primitive input order or identity changed: {ids:?}");
    }
    let first = projection
        .cases
        .first()
        .ok_or_else(|| anyhow!("characterization primitive input projection is empty"))?;
    if first.max_time_s != COMMON_MAX_TIME_S {
        bail!(
            "characterization common max_time_s must be {COMMON_MAX_TIME_S}, got {}",
            first.max_time_s
        );
    }
    for case in &projection.cases {
        if case.physics_hz != first.physics_hz
            || case.gravity_mps2 != first.gravity_mps2
            || case.max_time_s != first.max_time_s
            || case.vehicle != first.vehicle
            || case.initial_attitude_rad != first.initial_attitude_rad
            || case.initial_angular_rate_radps != first.initial_angular_rate_radps
        {
            bail!("characterization primitive inputs do not share the frozen physical setup");
        }
        if case.initial_attitude_rad != 0.0 || case.initial_angular_rate_radps != 0.0 {
            bail!(
                "case {} is not at the supported source rest attitude/rate",
                case.id
            );
        }
    }
    if first.physics_hz != frozen.policy.physics_hz
        || first.gravity_mps2 != frozen.policy.gravity_mps2
    {
        bail!("characterization physics rate or gravity differs from the embedded V2 policy");
    }

    let vehicle = vehicle_input_v2(&first.vehicle);
    if vehicle != frozen.vehicle {
        bail!("ScenarioSpec vehicle physical fields do not exactly equal embedded V2 vehicle");
    }
    let mut evaluated_policy = frozen.policy.clone();
    evaluated_policy.maximum_mission_time_s = first.max_time_s;
    evaluated_policy
        .validate()
        .map_err(|error| anyhow!("capped V2 policy is invalid: {error}"))?;

    let probes = projection
        .cases
        .iter()
        .map(|case| {
            let mapped_vehicle = vehicle_input_v2(&case.vehicle);
            if mapped_vehicle != frozen.vehicle {
                bail!(
                    "case {} vehicle physical fields do not exactly equal embedded V2 vehicle",
                    case.id
                );
            }
            Ok(DirectBridgeProbeV2 {
                id: case.id.clone(),
                source: pad_input_v2(&case.source_pad),
                target: pad_input_v2(&case.target_pad),
                terrain_points_m: case.terrain_points_m.clone(),
                initial_position_m: case.initial_position_m,
                initial_velocity_mps: case.initial_velocity_mps,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let input_identity = stable_digest(&(
        &projection.fixture_identity,
        first.physics_hz,
        first.gravity_mps2,
        first.max_time_s,
        &vehicle,
        &probes,
    ))?;
    let mut cases = Vec::with_capacity(probes.len());
    for probe in &probes {
        let result = evaluate_direct_bridge_case_v2(&evaluated_policy, &vehicle, probe).map_err(
            |error| anyhow!("direct bridge evaluation failed for {}: {error}", probe.id),
        )?;
        cases.push(project_result(probe, &result)?);
    }
    let early_gate = build_early_gate(&cases);
    let ordered_case_ids = probes.iter().map(|probe| probe.id.clone()).collect();
    let mut artifact = WaypointDirectPrimitiveAnalyticalArtifact {
        schema_id: WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_ID.to_owned(),
        protocol: ProtocolEvidence {
            evaluator: "pd_plan::conservative_ballistic_bridge::evaluate_direct_bridge_case_v2"
                .to_owned(),
            evaluation_mode: "controller_free_analytical_only".to_owned(),
            policy_copy_rule: "copy embedded V2 policy and replace only maximum_mission_time_s with the common characterization scenario max_time_s".to_owned(),
            changed_policy_fields: vec!["maximum_mission_time_s".to_owned()],
            controller_called: false,
            planner_called: false,
            simulation_called: false,
            v2_selected_candidate_role: "native V2 diagnostic selection; retained unchanged".to_owned(),
            research_selection_rule:
                "shortest certified direct candidate by total_time_s, then candidate identity".to_owned(),
        },
        input: InputEvidence {
            source_scenario_path: SOURCE_SCENARIO_PATH.to_owned(),
            source_characterization_fixture_identity: projection.fixture_identity,
            physics_hz: first.physics_hz,
            gravity_mps2: first.gravity_mps2,
            common_max_time_s: first.max_time_s,
            supported_source_attitude_rad: first.initial_attitude_rad,
            supported_source_angular_rate_radps: first.initial_angular_rate_radps,
            ordered_case_ids,
            identity: input_identity,
        },
        embedded_policy: frozen.policy,
        evaluated_policy,
        vehicle,
        probes,
        cases,
        early_gate,
        scope: PrimitiveScopeEvidence {
            claims: vec![
                "All five direct-route characterization inputs were evaluated by the V2 direct-bridge analytical API.".to_owned(),
                "The early gate opens only when every continuous flat, uphill, and downhill row has at least one certified direct candidate.".to_owned(),
                "The reported V2 selected candidate and the shortest certified research selection are distinct diagnostics.".to_owned(),
            ],
            non_claims: vec![
                "A rejected candidate or failed early gate is not a claim of physical impossibility.".to_owned(),
                "This baseline does not invoke the production planner, a controller, or the simulator.".to_owned(),
                "A passing gate is not a topology-sweep result, one-waypoint search, controller comparison, or default-planner promotion.".to_owned(),
                "The five characterization rows do not establish general terrain-family coverage.".to_owned(),
            ],
        },
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;
    Ok(artifact)
}

fn project_result(
    probe: &DirectBridgeProbeV2,
    result: &DirectBridgeProbeResultV2,
) -> Result<CaseEvidence> {
    if result.id != probe.id {
        bail!("V2 direct-bridge result id does not match input probe id");
    }
    let selected = result
        .candidates
        .iter()
        .find(|candidate| candidate.identity == result.selected_candidate_identity)
        .ok_or_else(|| anyhow!("V2 selected candidate identity is absent from candidate list"))?;
    let research_selection = result
        .candidates
        .iter()
        .filter(|candidate| candidate.classification == CertificationV2::Certified)
        .min_by(|left, right| {
            left.total_time_s
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.total_time_s.unwrap_or(f64::INFINITY))
                .then_with(|| left.identity.cmp(&right.identity))
        })
        .map(candidate_evidence);
    let candidates = result.candidates.iter().map(candidate_evidence).collect();
    Ok(CaseEvidence {
        id: result.id.clone(),
        input_identity: stable_digest(probe)?,
        result_identity: result.identity.clone(),
        certified_candidate_count: result.certified_candidate_count,
        v2_selected_candidate_identity: result.selected_candidate_identity.clone(),
        v2_selected_candidate: candidate_evidence(selected),
        research_selection,
        candidates,
    })
}

fn candidate_evidence(candidate: &DirectBridgeCandidateV2) -> CandidateEvidence {
    CandidateEvidence {
        identity: candidate.identity.clone(),
        classification: candidate.classification,
        reasons: candidate.reasons.clone(),
        minimum_normalized_margin: candidate.margins.minimum_normalized(),
        total_time_s: candidate.total_time_s,
        total_fuel_burn_kg: candidate.total_fuel_burn_kg,
    }
}

fn build_early_gate(cases: &[CaseEvidence]) -> EarlyGateEvidence {
    let case_certification = REQUIRED_CONTINUOUS_CASE_IDS
        .iter()
        .map(|required_id| {
            let certified_candidate_count = cases
                .iter()
                .find(|case| case.id == *required_id)
                .map_or(0, |case| case.certified_candidate_count);
            ContinuousCaseGateEvidence {
                id: (*required_id).to_owned(),
                has_certified_direct_candidate: certified_candidate_count > 0,
                certified_candidate_count,
            }
        })
        .collect::<Vec<_>>();
    let failed_case_ids = case_certification
        .iter()
        .filter(|case| !case.has_certified_direct_candidate)
        .map(|case| case.id.clone())
        .collect::<Vec<_>>();
    let passed = failed_case_ids.is_empty();
    EarlyGateEvidence {
        status: if passed { "passed" } else { "failed" }.to_owned(),
        passed,
        required_case_ids: REQUIRED_CONTINUOUS_CASE_IDS
            .iter()
            .map(|id| (*id).to_owned())
            .collect(),
        case_certification,
        failed_case_ids,
        consequence: "The gate controls eligibility for a later 24-cell obstacle topology sweep only; all five baseline rows remain in this artifact.".to_owned(),
        reason: if passed {
            "Each required continuous row has at least one certified direct candidate under the capped V2 policy.".to_owned()
        } else {
            "At least one required continuous row has no certified direct candidate under the capped V2 policy; broader route feasibility remains unknown and the topology sweep gate remains closed.".to_owned()
        },
    }
}

fn vehicle_input_v2(vehicle: &VehicleSpec) -> VehicleInputV2 {
    VehicleInputV2 {
        geometry: VehicleGeometryInputV2 {
            hull_width_m: vehicle.geometry.hull_width_m,
            hull_height_m: vehicle.geometry.hull_height_m,
            touchdown_half_span_m: vehicle.geometry.touchdown_half_span_m,
            touchdown_base_offset_m: vehicle.geometry.touchdown_base_offset_m,
        },
        dry_mass_kg: vehicle.dry_mass_kg,
        initial_fuel_kg: vehicle.initial_fuel_kg,
        max_fuel_kg: vehicle.max_fuel_kg,
        max_fuel_burn_kgps: vehicle.max_fuel_burn_kgps,
        max_thrust_n: vehicle.max_thrust_n,
        min_throttle_frac: vehicle.min_throttle_frac,
        max_rotation_rate_radps: vehicle.max_rotation_rate_radps,
        safe_touchdown_normal_speed_mps: vehicle.safe_touchdown_normal_speed_mps,
        safe_touchdown_tangential_speed_mps: vehicle.safe_touchdown_tangential_speed_mps,
        safe_touchdown_attitude_error_rad: vehicle.safe_touchdown_attitude_error_rad,
        safe_touchdown_angular_rate_radps: vehicle.safe_touchdown_angular_rate_radps,
    }
}

fn pad_input_v2(pad: &LandingPadSpec) -> PadInputV2 {
    PadInputV2 {
        center_x_m: pad.center_x_m,
        surface_y_m: pad.surface_y_m,
        width_m: pad.width_m,
    }
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
                .join(WAYPOINT_DIRECT_PRIMITIVE_ANALYTICAL_ID)
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

fn artifact_identity(artifact: &WaypointDirectPrimitiveAnalyticalArtifact) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval crate is under repository root")
            .to_path_buf()
    }

    #[test]
    fn five_case_projection_keeps_original_order_and_identity() {
        let projection = project_waypoint_direct_primitive_inputs(&repo_root())
            .expect("existing characterization fixture projects");
        assert_eq!(
            projection
                .cases
                .iter()
                .map(|case| case.id.as_str())
                .collect::<Vec<_>>(),
            EXPECTED_CASE_IDS
        );
        assert_eq!(projection.fixture_identity, "fnv1a64:0feb123b6d00ad98");
    }

    #[test]
    fn protocol_mapping_matches_embedded_vehicle_and_caps_policy_at_ninety_seconds() {
        let frozen = load_embedded_fixture_v2();
        let projection = project_waypoint_direct_primitive_inputs(&repo_root())
            .expect("existing characterization fixture projects");
        let first = projection.cases.first().expect("five fixture rows");
        let mapped = vehicle_input_v2(&first.vehicle);
        assert_eq!(mapped, frozen.vehicle);
        for case in &projection.cases {
            assert_eq!(case.physics_hz, frozen.policy.physics_hz);
            assert_eq!(case.gravity_mps2, frozen.policy.gravity_mps2);
            assert_eq!(case.max_time_s, COMMON_MAX_TIME_S);
            assert_eq!(vehicle_input_v2(&case.vehicle), frozen.vehicle);
            assert_eq!(case.initial_attitude_rad, 0.0);
            assert_eq!(case.initial_angular_rate_radps, 0.0);
        }
        let mut expected = frozen.policy.clone();
        expected.maximum_mission_time_s = COMMON_MAX_TIME_S;
        let mut actual = frozen.policy.clone();
        actual.maximum_mission_time_s = projection.cases[0].max_time_s;
        assert_eq!(actual, expected);
        assert_eq!(actual.maximum_mission_time_s, 90.0);
    }

    #[test]
    fn actual_five_case_evaluation_records_native_and_research_selections_and_gate() {
        let artifact = build_artifact(&repo_root()).expect("five exact inputs evaluate");
        assert_eq!(artifact.cases.len(), 5);
        assert_eq!(artifact.probes.len(), 5);
        assert_eq!(artifact.input.ordered_case_ids.len(), 5);
        assert_eq!(
            artifact
                .probes
                .iter()
                .map(|probe| probe.id.as_str())
                .collect::<Vec<_>>(),
            EXPECTED_CASE_IDS
        );
        for case in &artifact.cases {
            assert_eq!(
                case.v2_selected_candidate_identity,
                case.v2_selected_candidate.identity
            );
            assert_eq!(
                case.research_selection.is_some(),
                case.certified_candidate_count > 0
            );
            if let Some(research) = &case.research_selection {
                assert_eq!(research.classification, CertificationV2::Certified);
                assert!(case.candidates.iter().any(|candidate| {
                    candidate.identity == research.identity
                        && candidate.classification == CertificationV2::Certified
                }));
            }
        }
        assert!(artifact.early_gate.passed);
        assert!(artifact.early_gate.failed_case_ids.is_empty());
        assert_eq!(artifact.early_gate.required_case_ids.len(), 3);
        assert!(artifact.early_gate.case_certification.iter().all(|case| {
            case.has_certified_direct_candidate && case.certified_candidate_count > 0
        }));
    }
}
