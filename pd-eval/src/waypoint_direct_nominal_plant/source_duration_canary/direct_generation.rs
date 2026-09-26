//! Input-driven generation over the bounded V2 duration family.
//!
//! This lane consumes a caller-supplied physical scenario and fixed policy.
//! It does not read historical evaluator summaries or import stored commands.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow, bail};
use pd_core::{RunContext, ScenarioSpec};
use pd_plan::conservative_ballistic_bridge::{
    BridgeKindV2, CertificationV2, DirectBridgeCandidateV2, DirectBridgePolicyV2,
    DirectBridgeProbeV2, KinematicStateV2, PadInputV2, VehicleInputV2,
    evaluate_direct_bridge_case_v2, exact_discrete_bridge_v2,
};
use serde::{Deserialize, Serialize};

use super::super::stable_digest;
use super::{
    AcceptedWrapperSelectionEvidence, CompleteFlatWrapperEvidence, FullFlightEvidence,
    PreparedDurationVariant, SourceDurationVariantEvidence, UnlaunchedSourceBridgeEvidence,
    bridge_measurements,
    held_cadence_diagnostic::RunWithSamples,
    paired_command_feasibility::{PairedScheduleEvidence, run_schedule_experiment_from_physical},
};

pub const WAYPOINT_DIRECT_NOMINAL_DIRECT_GENERATION_ID: &str =
    "waypoint-direct-nominal-direct-generation";
pub const WAYPOINT_DIRECT_NOMINAL_DIRECT_GENERATION_SCHEMA_ID: &str =
    "waypoint_direct_nominal_direct_generation_v1";
pub const WAYPOINT_DIRECT_NOMINAL_DIRECT_GENERATION_SCHEMA_VERSION: u32 = 1;

const GENERATION_POLICY_VERSION: &str = "launch_aware_held60_nominal_direct_generation_v1";
const GEOMETRY_CONVENTION: &str = "core_current_rotated_feet_and_hull";
const SOLVER_VERSION: &str = "paired_mean_vector_damped_finite_difference_shooting_v1";
const DURATION_OFFSETS_TICKS: [i64; 5] = [-240, -180, -120, -60, 0];

fn vehicle_spec_from_v2(vehicle: &VehicleInputV2) -> pd_core::VehicleSpec {
    pd_core::VehicleSpec {
        geometry: pd_core::VehicleGeometry {
            hull_width_m: vehicle.geometry.hull_width_m,
            hull_height_m: vehicle.geometry.hull_height_m,
            touchdown_half_span_m: vehicle.geometry.touchdown_half_span_m,
            touchdown_base_offset_m: vehicle.geometry.touchdown_base_offset_m,
        },
        dry_mass_kg: vehicle.dry_mass_kg,
        initial_fuel_kg: vehicle.initial_fuel_kg,
        max_fuel_kg: vehicle.max_fuel_kg,
        max_thrust_n: vehicle.max_thrust_n,
        max_fuel_burn_kgps: vehicle.max_fuel_burn_kgps,
        min_throttle_frac: vehicle.min_throttle_frac,
        max_rotation_rate_radps: vehicle.max_rotation_rate_radps,
        safe_touchdown_normal_speed_mps: vehicle.safe_touchdown_normal_speed_mps,
        safe_touchdown_tangential_speed_mps: vehicle.safe_touchdown_tangential_speed_mps,
        safe_touchdown_attitude_error_rad: vehicle.safe_touchdown_attitude_error_rad,
        safe_touchdown_angular_rate_radps: vehicle.safe_touchdown_angular_rate_radps,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointDirectNominalDirectGenerationRequest {
    pub scenario: ScenarioSpec,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub probe_id: String,
    pub policy: WaypointDirectNominalDirectGenerationPolicyV1,
}

/// Sealed controls for the first version of input-driven nominal generation.
/// Every field is identity-bound and validation rejects values outside this
/// declared policy; callers can serialize the policy but cannot tune a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointDirectNominalDirectGenerationPolicyV1 {
    pub version: String,
    pub analytical_policy: DirectBridgePolicyV2,
    pub physics_hz: u32,
    pub controller_hz: u32,
    pub launch_upright_ticks: u64,
    pub launch_tilt_ticks: u64,
    pub source_duration_offsets_ticks: Vec<i64>,
    pub maximum_basis_candidates: usize,
    pub maximum_variants: usize,
    pub solver_version: String,
    pub maximum_solver_iterations: usize,
    pub maximum_line_search_steps: usize,
    pub finite_difference_step_mps2: f64,
    pub maximum_correction_abs_mps2: f64,
    pub initial_damping: f64,
    pub position_tolerance_m: f64,
    pub velocity_tolerance_mps: f64,
    pub geometry_convention: String,
    pub selection_rule: String,
}

impl Default for WaypointDirectNominalDirectGenerationPolicyV1 {
    fn default() -> Self {
        Self {
            version: GENERATION_POLICY_VERSION.to_owned(),
            analytical_policy: DirectBridgePolicyV2 {
                physics_hz: 120,
                gravity_mps2: 9.81,
                duration_multipliers: vec![0.75, 1.0, 1.25, 1.5],
                minimum_clearance_m: 5.0,
                maximum_mission_time_s: 90.0,
                mission_time_reserve_s: 10.0,
                thrust_derate: 0.81,
                declared_robustness_margin: 0.075,
                handoff_interval_s: 0.25,
                bridge_duration_interval_s: 0.5,
                terminal_target_downward_speed_fraction: 0.5,
            },
            physics_hz: 120,
            controller_hz: 60,
            launch_upright_ticks: 60,
            launch_tilt_ticks: 12,
            source_duration_offsets_ticks: DURATION_OFFSETS_TICKS.to_vec(),
            maximum_basis_candidates: 4,
            maximum_variants: 20,
            solver_version: SOLVER_VERSION.to_owned(),
            maximum_solver_iterations: 6,
            maximum_line_search_steps: 8,
            finite_difference_step_mps2: 1.0e-4,
            maximum_correction_abs_mps2: 0.25,
            initial_damping: 1.0e-3,
            position_tolerance_m: 1.0e-6,
            velocity_tolerance_mps: 1.0e-6,
            geometry_convention: GEOMETRY_CONVENTION.to_owned(),
            selection_rule:
                "complete accepted witnesses only; planned total mission time then stable witness identity"
                    .to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectNominalDirectGenerationValidation {
    pub probe_id: String,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub scenario_identity: String,
    pub policy_identity: String,
    pub vehicle_identity: String,
    pub source_target_horizontal_span_m: f64,
    pub source_pad_supported: bool,
    pub maximum_basis_candidates: usize,
    pub maximum_variants: usize,
    pub physics_hz: u32,
    pub controller_hz: u32,
    pub simulation_created: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectNominalDirectGenerationPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectNominalDirectGenerationRun {
    pub artifact: WaypointDirectNominalDirectGenerationArtifact,
    pub paths: WaypointDirectNominalDirectGenerationPaths,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectNominalDirectGenerationArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub probe_id: String,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub scenario_identity: String,
    pub policy_identity: String,
    pub request: WaypointDirectNominalDirectGenerationRequest,
    pub policy: WaypointDirectNominalDirectGenerationPolicyV1,
    pub provenance_convention: String,
    pub scope_non_claims: Vec<String>,
    pub bases: Vec<WaypointDirectNominalDirectGenerationBasisEvidence>,
    pub rows: Vec<WaypointDirectNominalDirectGenerationRowEvidence>,
    pub family_proof: WaypointDirectNominalDirectGenerationFamilyProofEvidence,
    pub selection: Option<AcceptedWrapperSelectionEvidence>,
    pub selected_physical_witness:
        Option<WaypointDirectNominalDirectGenerationSelectedWitnessEvidence>,
    pub execution_status: String,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectNominalDirectGenerationBasisEvidence {
    pub basis_index: usize,
    /// The V2 seed candidate ID, retained for Gate A row association only.
    pub candidate_identity: String,
    /// Request-bound new basis identity used by every generated witness.
    pub generated_basis_identity: String,
    pub duration_multiplier: f64,
    pub original_v2_classification: CertificationV2,
    pub original_v2_reasons: Vec<String>,
    pub original_total_time_s: Option<f64>,
    pub original_total_fuel_burn_kg: Option<f64>,
    pub original_source_handoff: Option<KinematicStateV2>,
    pub original_source_bridge_tick_count: Option<u64>,
    pub coast_tick_count: Option<u64>,
    pub terminal_bridge_tick_count: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectNominalDirectGenerationSelectedWitnessEvidence {
    pub row_index: usize,
    pub basis_candidate_identity: String,
    pub generated_basis_identity: String,
    pub duration_offset_ticks: i64,
    pub source_bridge_tick_count: u64,
    pub source_handoff: KinematicStateV2,
    pub wrapper_identity: String,
    pub command_schedule_identity: String,
    pub first_contact: Option<super::FirstContactEvidence>,
    pub planned_total_mission_time_s: f64,
    pub actual_touchdown_time_s: Option<f64>,
    pub actual_touchdown_fuel_used_kg: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectNominalDirectGenerationRowEvidence {
    pub row_index: usize,
    pub basis_index: usize,
    pub basis_candidate_identity: String,
    pub generated_basis_identity: String,
    pub duration_offset_ticks: i64,
    pub source_bridge_tick_count: Option<u64>,
    pub original_v2_classification: CertificationV2,
    pub source_handoff: Option<KinematicStateV2>,
    pub analytical_survivor: bool,
    pub source_handoff_survivor: bool,
    pub status: String,
    pub skip_reason: Option<String>,
    pub source_duration: Option<SourceDurationVariantEvidence>,
    pub paired_schedule: Option<PairedScheduleEvidence>,
    pub wrapper: Option<CompleteFlatWrapperEvidence>,
    pub accepted: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectNominalDirectGenerationFamilyProofEvidence {
    pub basis_count: usize,
    pub expected_basis_count: usize,
    pub row_count: usize,
    pub expected_row_count: usize,
    pub retained_noncertified_basis_count: usize,
    pub analytical_skip_count: usize,
    pub scheduled_count: usize,
    pub completed_acceptance_count: usize,
    pub accepted_witness_count: usize,
    pub omitted_row_count: usize,
    pub compute_counters: WaypointDirectNominalDirectGenerationComputeCounters,
    pub all_predeclared_rows_recorded: bool,
    pub completion_gate_passed: bool,
    pub stopping_result: String,
}

/// Deterministic work accounting for the bounded family. Physics ticks are
/// reported as an explicit budget upper bound until every nested replay call
/// exposes an authoritative cumulative counter.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectNominalDirectGenerationComputeCounters {
    pub analytical_basis_candidates_evaluated: usize,
    pub complete_certified_basis_profiles: usize,
    pub duration_rows_predeclared: usize,
    pub duration_bridge_build_attempts: usize,
    pub source_analysis_runs: usize,
    pub source_handoff_runs: usize,
    pub held_baseline_runs: usize,
    pub paired_schedule_fits: usize,
    pub paired_solver_iterations: usize,
    pub finite_difference_columns: usize,
    pub line_search_trials: usize,
    pub complete_witness_verifications: usize,
    pub physics_tick_budget_upper_bound: u64,
    pub actual_physics_ticks: Option<u64>,
    pub physics_tick_accounting: String,
}

/// Validate policy and scenario support without constructing `SimulationState`
/// or evaluating any V2 candidate.
pub fn validate_waypoint_direct_nominal_direct_generation_request(
    request: &WaypointDirectNominalDirectGenerationRequest,
) -> Result<WaypointDirectNominalDirectGenerationValidation> {
    request
        .scenario
        .validate()
        .map_err(|error| anyhow!("malformed request scenario: {error}"))?;
    if request.policy != WaypointDirectNominalDirectGenerationPolicyV1::default() {
        bail!("unsupported request policy version or tuned parameter");
    }
    if request.probe_id.trim().is_empty()
        || request.source_pad_id.trim().is_empty()
        || request.target_pad_id.trim().is_empty()
        || request.source_pad_id == request.target_pad_id
    {
        bail!("malformed request: probe and distinct source/target pad IDs are required");
    }
    let scenario = &request.scenario;
    if scenario.sim.physics_hz != request.policy.physics_hz
        || scenario.sim.controller_hz != request.policy.controller_hz
        || scenario.world.gravity_mps2 != request.policy.analytical_policy.gravity_mps2
    {
        bail!("unsupported request physics rate, controller rate, or gravity setting");
    }
    let context = RunContext::from_scenario(scenario)
        .map_err(|error| anyhow!("malformed request scenario: {error}"))?;
    let supported_vehicle =
        pd_plan::conservative_ballistic_bridge::load_embedded_fixture_v2().vehicle;
    let supported_vehicle_spec = vehicle_spec_from_v2(&supported_vehicle);
    if scenario.vehicle != supported_vehicle_spec {
        bail!("unsupported request full VehicleSpec differs from current vehicle parameters");
    }
    if scenario.initial_state.attitude_rad != 0.0
        || scenario.initial_state.angular_rate_radps != 0.0
        || scenario.initial_state.velocity_mps != pd_core::Vec2::new(0.0, 0.0)
    {
        bail!("unsupported source state: expected upright, zero angular rate, and rest");
    }
    if let Some(route) = &scenario.mission.transfer_route {
        if !route.waypoints.is_empty() {
            bail!("unsupported authored operational waypoints");
        }
        if route.source_pad_id != request.source_pad_id
            || route.target_pad_id != request.target_pad_id
        {
            bail!("malformed request: route pad IDs differ from the requested pads");
        }
    }
    if scenario.mission.goal.target_pad_id() != request.target_pad_id {
        bail!("malformed request: target pad ID must match the scenario landing goal");
    }
    let source_pad = scenario
        .world
        .landing_pad(&request.source_pad_id)
        .ok_or_else(|| {
            anyhow!(
                "malformed request: source pad {} is missing",
                request.source_pad_id
            )
        })?;
    let target_pad = scenario
        .world
        .landing_pad(&request.target_pad_id)
        .ok_or_else(|| {
            anyhow!(
                "malformed request: target pad {} is missing",
                request.target_pad_id
            )
        })?;
    if target_pad.center_x_m <= source_pad.center_x_m {
        bail!("unsupported geometry: direct generation requires a forward source-to-target span");
    }
    if (scenario.initial_state.position_m.x - source_pad.center_x_m).abs() > 1.0e-9
        || (scenario.initial_state.position_m.y
            - (source_pad.surface_y_m + scenario.vehicle.geometry.touchdown_base_offset_m))
            .abs()
            > 1.0e-9
    {
        bail!("unsupported source state: initial pose is not supported source-pad rest");
    }
    let body_half_width = (scenario.vehicle.geometry.hull_width_m * 0.5)
        .max(scenario.vehicle.geometry.touchdown_half_span_m);
    let source_left = source_pad.center_x_m - source_pad.width_m * 0.5;
    let source_right = source_pad.center_x_m + source_pad.width_m * 0.5;
    if source_pad.width_m <= 0.0
        || !pad_has_flat_in_domain(&scenario.world.terrain, source_pad)
        || source_pad.center_x_m - body_half_width < source_left
        || source_pad.center_x_m + body_half_width > source_right
    {
        bail!("unsupported source pad: not flat or not wide enough for the actual feet and hull");
    }
    if target_pad.width_m <= 0.0 || !pad_has_flat_in_domain(&scenario.world.terrain, target_pad) {
        bail!("unsupported target pad: not a flat shelf inside the terrain domain");
    }
    let source_target_horizontal_span_m = target_pad.center_x_m - source_pad.center_x_m;
    let scenario_identity = stable_digest(scenario)?;
    let policy_identity = stable_digest(&request.policy)?;
    let vehicle_identity = stable_digest(&scenario.vehicle)?;
    Ok(WaypointDirectNominalDirectGenerationValidation {
        probe_id: request.probe_id.clone(),
        source_pad_id: request.source_pad_id.clone(),
        target_pad_id: request.target_pad_id.clone(),
        scenario_identity,
        policy_identity,
        vehicle_identity,
        source_target_horizontal_span_m,
        source_pad_supported: true,
        maximum_basis_candidates: request.policy.maximum_basis_candidates,
        maximum_variants: request.policy.maximum_variants,
        physics_hz: context.sim.physics_hz,
        controller_hz: context.sim.controller_hz,
        simulation_created: false,
    })
}

fn pad_has_flat_in_domain(
    terrain: &pd_core::TerrainDefinition,
    pad: &pd_core::LandingPadSpec,
) -> bool {
    let left = pad.center_x_m - pad.width_m * 0.5;
    let right = pad.center_x_m + pad.width_m * 0.5;
    let (Ok(left_height), Ok(right_height)) = (
        terrain.sample_height_strict(left),
        terrain.sample_height_strict(right),
    ) else {
        return false;
    };
    left_height == pad.surface_y_m
        && right_height == pad.surface_y_m
        && terrain
            .points()
            .iter()
            .all(|point| point.x < left || point.x > right || point.y == pad.surface_y_m)
}

/// Generate all predeclared basis/duration rows in memory. This is the same
/// operation used by the create-only writer, and is exposed for Gate A parity.
pub fn evaluate_waypoint_direct_nominal_direct_generation(
    request: &WaypointDirectNominalDirectGenerationRequest,
) -> Result<WaypointDirectNominalDirectGenerationArtifact> {
    let validation = validate_waypoint_direct_nominal_direct_generation_request(request)?;
    let request_identity = stable_digest(request)?;
    let scenario_identity = validation.scenario_identity.clone();
    let policy_identity = validation.policy_identity.clone();
    let vehicle = super::super::vehicle_input_v2(&request.scenario.vehicle);
    let probe = generated_probe(request)?;
    let v2_policy = &request.policy.analytical_policy;
    let v2_result = evaluate_direct_bridge_case_v2(v2_policy, &vehicle, &probe)
        .map_err(|error| anyhow!("fresh direct V2 basis evaluation failed: {error}"))?;
    if v2_result.candidates.len() != request.policy.maximum_basis_candidates
        || v2_result.candidates.len() != v2_policy.duration_multipliers.len()
    {
        bail!("fresh direct V2 evaluator returned an incomplete basis family");
    }
    for (index, (candidate, multiplier)) in v2_result
        .candidates
        .iter()
        .zip(&v2_policy.duration_multipliers)
        .enumerate()
    {
        if candidate.duration_multiplier != *multiplier {
            bail!("fresh V2 basis order changed at analytical candidate {index}");
        }
    }

    let mut bases = Vec::with_capacity(v2_result.candidates.len());
    let mut rows = Vec::with_capacity(request.policy.maximum_variants);
    let mut complete_certified_basis_profiles = 0;
    let mut duration_bridge_build_attempts = 0;
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    for (basis_index, candidate) in v2_result.candidates.iter().enumerate() {
        let generated_basis_identity = generated_basis_identity(
            &request_identity,
            &scenario_identity,
            &policy_identity,
            basis_index,
            candidate,
        )?;
        let original_source_bridge = candidate.source_bridge.as_ref();
        let source_handoff = candidate.source_handoff.map(|handoff| handoff.state);
        let coast_tick_count = candidate
            .selected_coast
            .as_ref()
            .map(|coast| (coast.duration_s * f64::from(request.policy.physics_hz)).round() as u64);
        bases.push(WaypointDirectNominalDirectGenerationBasisEvidence {
            basis_index,
            candidate_identity: candidate.identity.clone(),
            generated_basis_identity: generated_basis_identity.clone(),
            duration_multiplier: candidate.duration_multiplier,
            original_v2_classification: candidate.classification,
            original_v2_reasons: candidate
                .reasons
                .iter()
                .map(|reason| format!("{reason:?}"))
                .collect(),
            original_total_time_s: candidate.total_time_s,
            original_total_fuel_burn_kg: candidate.total_fuel_burn_kg,
            original_source_handoff: source_handoff,
            original_source_bridge_tick_count: original_source_bridge.map(|bridge| bridge.steps),
            coast_tick_count,
            terminal_bridge_tick_count: candidate
                .terminal_bridge
                .as_ref()
                .map(|bridge| bridge.steps),
        });
        if candidate.classification != CertificationV2::Certified {
            for (offset_index, offset) in DURATION_OFFSETS_TICKS.iter().copied().enumerate() {
                rows.push(skipped_basis_row(
                    basis_index * DURATION_OFFSETS_TICKS.len() + offset_index,
                    basis_index,
                    &candidate.identity,
                    &generated_basis_identity,
                    offset,
                    candidate.classification,
                    "base_v2_not_certified",
                    None,
                    None,
                ));
            }
            continue;
        }
        complete_certified_basis_profiles += 1;
        let profile =
            super::super::materialize_profile(&v2_result, candidate, &probe, v2_policy, &vehicle)?;
        let selected_profile = super::super::PreparedProfileCandidate {
            candidate_identity: candidate.identity.clone(),
            selected_roles: Vec::new(),
            profile,
        };
        let (variants, invalid_duration_rows) = prepare_generated_duration_variants(
            basis_index,
            candidate,
            &generated_basis_identity,
            v2_policy,
            &vehicle,
        )?;
        duration_bridge_build_attempts += variants.len();
        rows.extend(invalid_duration_rows);
        let basis = super::execute_basis_variants_from_physical(
            &request.scenario,
            &probe,
            v2_policy,
            &vehicle,
            candidate,
            &selected_profile,
            None,
            "input_generated_complete_v2_seed_profile",
            variants,
        )?;
        for source_duration in basis.duration_variants {
            rows.push(complete_generated_row(
                request,
                &request_identity,
                &scenario_identity,
                &policy_identity,
                &context,
                &probe,
                &vehicle,
                candidate,
                &selected_profile,
                &generated_basis_identity,
                source_duration,
            )?);
        }
    }
    rows.sort_by_key(|row| row.row_index);
    if rows.len() != request.policy.maximum_variants
        || rows
            .iter()
            .enumerate()
            .any(|(index, row)| row.row_index != index)
    {
        bail!("direct-generation omitted or reordered a predeclared basis/offset row");
    }
    let analytical_skip_count = rows.iter().filter(|row| !row.analytical_survivor).count();
    let scheduled_count = rows
        .iter()
        .filter(|row| row.paired_schedule.is_some())
        .count();
    let completed_acceptance_count = rows.iter().filter(|row| row.wrapper.is_some()).count();
    let accepted_witness_count = rows.iter().filter(|row| row.accepted).count();
    let row_count = rows.len();
    let all_predeclared_rows_recorded = row_count == request.policy.maximum_variants;
    let completion_gate_passed = all_predeclared_rows_recorded && accepted_witness_count > 0;
    let selection = select_generated_witness(&rows, completion_gate_passed);
    let selected_physical_witness = selection
        .as_ref()
        .map(|selection| selected_witness_evidence(&rows[selection.row_index], selection))
        .transpose()?;
    let compute_counters = compute_counters(
        &rows,
        request,
        v2_result.candidates.len(),
        complete_certified_basis_profiles,
        duration_bridge_build_attempts,
    )?;
    let stopping_result = if completion_gate_passed {
        "nominal_replay_validated_direct"
    } else {
        "unknown_no_complete_accepted_witness_within_finite_family"
    }
    .to_owned();
    let mut artifact = WaypointDirectNominalDirectGenerationArtifact {
        schema_id: WAYPOINT_DIRECT_NOMINAL_DIRECT_GENERATION_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_NOMINAL_DIRECT_GENERATION_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_NOMINAL_DIRECT_GENERATION_ID.to_owned(),
        probe_id: request.probe_id.clone(),
        source_pad_id: request.source_pad_id.clone(),
        target_pad_id: request.target_pad_id.clone(),
        scenario_identity,
        policy_identity,
        request: request.clone(),
        policy: request.policy.clone(),
        provenance_convention: "Generated wrapper provenance uses domain-separated request, policy, generated basis, launch, source command schedule, and unchanged candidate-tail identities. Legacy-shaped provenance field labels do not bind historical summaries or commands.".to_owned(),
        scope_non_claims: vec![
            "This artifact is an input-driven finite nominal direct-generation family; no historical summaries or stored schedules are generator inputs.".to_owned(),
            "A valid finite family with no accepted witness is unknown, not a physical-impossibility result.".to_owned(),
            "Pointwise discrete replay is not a swept-path proof or perturbation-robustness guarantee.".to_owned(),
            "Generated wrapper decisions are not V2 Certified and do not authorize planner/controller/default changes; retained seed classifications are unchanged.".to_owned(),
        ],
        bases,
        rows,
        family_proof: WaypointDirectNominalDirectGenerationFamilyProofEvidence {
            basis_count: v2_result.candidates.len(),
            expected_basis_count: request.policy.maximum_basis_candidates,
            row_count,
            expected_row_count: request.policy.maximum_variants,
            retained_noncertified_basis_count: v2_result
                .candidates
                .iter()
                .filter(|candidate| candidate.classification != CertificationV2::Certified)
                .count(),
            analytical_skip_count,
            scheduled_count,
            completed_acceptance_count,
            accepted_witness_count,
            omitted_row_count: request.policy.maximum_variants.saturating_sub(row_count),
            compute_counters,
            all_predeclared_rows_recorded,
            completion_gate_passed,
            stopping_result: stopping_result.clone(),
        },
        selection,
        selected_physical_witness,
        execution_status: stopping_result,
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;
    Ok(artifact)
}

pub fn run_waypoint_direct_nominal_direct_generation(
    repo_root: &Path,
    request: &WaypointDirectNominalDirectGenerationRequest,
    output_dir: &Path,
) -> Result<WaypointDirectNominalDirectGenerationRun> {
    validate_waypoint_direct_nominal_direct_generation_request(request)?;
    let output_dir = super::super::resolve_output_dir(repo_root, output_dir);
    let summary_path = output_dir.join("summary.json");
    if output_dir.exists() {
        bail!(
            "direct-generation refuses to reuse existing output directory {}",
            output_dir.display()
        );
    }
    if summary_path.exists() {
        bail!(
            "direct-generation refuses to overwrite existing summary {}",
            summary_path.display()
        );
    }
    let artifact = evaluate_waypoint_direct_nominal_direct_generation(request)?;
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    let reloaded: WaypointDirectNominalDirectGenerationArtifact =
        serde_json::from_slice(&summary_bytes)?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes
        || artifact_identity(&reloaded)? != reloaded.identity
        || reloaded.identity != artifact.identity
    {
        bail!("direct-generation summary failed deterministic round-trip validation");
    }
    if let Some(parent_dir) = output_dir.parent() {
        fs::create_dir_all(parent_dir)?;
    }
    fs::create_dir(&output_dir).map_err(|error| {
        anyhow!(
            "direct-generation refuses to reuse output directory {}: {error}",
            output_dir.display()
        )
    })?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&summary_path)
        .map_err(|error| {
            anyhow!(
                "direct-generation refuses to overwrite {}: {error}",
                summary_path.display()
            )
        })?;
    file.write_all(&summary_bytes)?;
    drop(file);
    Ok(WaypointDirectNominalDirectGenerationRun {
        artifact: reloaded,
        paths: WaypointDirectNominalDirectGenerationPaths {
            output_dir,
            summary_path,
        },
    })
}

fn generated_probe(
    request: &WaypointDirectNominalDirectGenerationRequest,
) -> Result<DirectBridgeProbeV2> {
    let source = request
        .scenario
        .world
        .landing_pad(&request.source_pad_id)
        .ok_or_else(|| anyhow!("malformed request: source pad disappeared after validation"))?;
    let target = request
        .scenario
        .world
        .landing_pad(&request.target_pad_id)
        .ok_or_else(|| anyhow!("malformed request: target pad disappeared after validation"))?;
    Ok(DirectBridgeProbeV2 {
        id: request.probe_id.clone(),
        source: PadInputV2 {
            center_x_m: source.center_x_m,
            surface_y_m: source.surface_y_m,
            width_m: source.width_m,
        },
        target: PadInputV2 {
            center_x_m: target.center_x_m,
            surface_y_m: target.surface_y_m,
            width_m: target.width_m,
        },
        terrain_points_m: request.scenario.world.terrain.points().to_vec(),
        initial_position_m: request.scenario.initial_state.position_m,
        initial_velocity_mps: request.scenario.initial_state.velocity_mps,
    })
}

fn generated_basis_identity(
    request_identity: &str,
    scenario_identity: &str,
    policy_identity: &str,
    basis_index: usize,
    candidate: &DirectBridgeCandidateV2,
) -> Result<String> {
    stable_digest(&(
        "input_driven_direct_generation_basis_v1",
        GENERATION_POLICY_VERSION,
        request_identity,
        scenario_identity,
        policy_identity,
        basis_index,
        &candidate.identity,
        candidate.duration_multiplier,
    ))
}

fn prepare_generated_duration_variants(
    basis_index: usize,
    candidate: &DirectBridgeCandidateV2,
    generated_basis_identity: &str,
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
) -> Result<(
    Vec<PreparedDurationVariant>,
    Vec<WaypointDirectNominalDirectGenerationRowEvidence>,
)> {
    let original = candidate.source_bridge.as_ref().ok_or_else(|| {
        anyhow!(
            "certified basis {} has no source bridge",
            candidate.identity
        )
    })?;
    let handoff = candidate.source_handoff.ok_or_else(|| {
        anyhow!(
            "certified basis {} has no source handoff",
            candidate.identity
        )
    })?;
    let mut variants = Vec::with_capacity(DURATION_OFFSETS_TICKS.len());
    let mut invalid_rows = Vec::new();
    for (duration_index, offset) in DURATION_OFFSETS_TICKS.iter().copied().enumerate() {
        let row_index = basis_index * DURATION_OFFSETS_TICKS.len() + duration_index;
        let tick_count = i64::try_from(original.steps)
            .ok()
            .and_then(|ticks| ticks.checked_add(offset))
            .filter(|ticks| *ticks > 0)
            .and_then(|ticks| u64::try_from(ticks).ok());
        let Some(tick_count) = tick_count else {
            invalid_rows.push(skipped_basis_row(
                row_index,
                basis_index,
                &candidate.identity,
                generated_basis_identity,
                offset,
                candidate.classification,
                "source_duration_tick_count_nonpositive",
                None,
                Some(handoff.state),
            ));
            continue;
        };
        if !tick_count.is_multiple_of(2) {
            invalid_rows.push(skipped_basis_row(
                row_index,
                basis_index,
                &candidate.identity,
                generated_basis_identity,
                offset,
                candidate.classification,
                "source_duration_not_on_60hz_controller_cadence",
                Some(tick_count),
                Some(handoff.state),
            ));
            continue;
        }
        let build = exact_discrete_bridge_v2(
            policy,
            vehicle,
            BridgeKindV2::Source,
            original.start_state,
            handoff.state,
            tick_count,
        );
        if offset == 0
            && build
                .as_ref()
                .is_ok_and(|bridge| bridge.identity != original.identity)
        {
            bail!("generated baseline source bridge no longer matches its V2 seed basis");
        }
        if offset == 0
            && let Err(error) = &build
        {
            bail!("generated baseline source bridge failed to rebuild: {error}");
        }
        let (unlaunched_bridge, launch_target, evidence) = match build {
            Ok(bridge) => {
                let launch_target = bridge
                    .samples
                    .iter()
                    .find(|sample| sample.thrust_acceleration_mps2.length() > 1.0e-12)
                    .map(|sample| {
                        sample
                            .thrust_acceleration_mps2
                            .x
                            .atan2(sample.thrust_acceleration_mps2.y)
                    });
                let evidence = UnlaunchedSourceBridgeEvidence {
                    build_status: "materialized".to_owned(),
                    build_error: None,
                    identity: Some(bridge.identity.clone()),
                    matches_frozen_original_identity: bridge.identity == original.identity,
                    classification: Some(bridge.classification),
                    reasons: bridge.reasons.clone(),
                    margins: Some(bridge.margins),
                    tick_count: bridge.steps,
                    duration_s: Some(bridge.duration_s),
                    endpoint_position_error_m: Some(bridge.endpoint_position_error_m),
                    endpoint_velocity_error_mps: Some(bridge.endpoint_velocity_error_mps),
                    first_powered_attitude_rad: launch_target,
                    measurements: Some(bridge_measurements(&bridge, policy, vehicle)),
                };
                (Some(bridge), launch_target, evidence)
            }
            Err(error) => (
                None,
                None,
                UnlaunchedSourceBridgeEvidence {
                    build_status: "bridge_build_failed".to_owned(),
                    build_error: Some(error),
                    identity: None,
                    matches_frozen_original_identity: false,
                    classification: None,
                    reasons: Vec::new(),
                    margins: None,
                    tick_count,
                    duration_s: None,
                    endpoint_position_error_m: None,
                    endpoint_velocity_error_mps: None,
                    first_powered_attitude_rad: None,
                    measurements: None,
                },
            ),
        };
        variants.push(PreparedDurationVariant {
            row_index,
            duration_offset_ticks: offset,
            source_bridge_tick_count: tick_count,
            unlaunched_bridge,
            unlaunched_evidence: evidence,
            launch_target_attitude_rad: launch_target,
        });
    }
    Ok((variants, invalid_rows))
}

#[allow(clippy::too_many_arguments)]
fn skipped_basis_row(
    row_index: usize,
    basis_index: usize,
    basis_candidate_identity: &str,
    generated_basis_identity: &str,
    duration_offset_ticks: i64,
    classification: CertificationV2,
    reason: &str,
    source_bridge_tick_count: Option<u64>,
    source_handoff: Option<KinematicStateV2>,
) -> WaypointDirectNominalDirectGenerationRowEvidence {
    WaypointDirectNominalDirectGenerationRowEvidence {
        row_index,
        basis_index,
        basis_candidate_identity: basis_candidate_identity.to_owned(),
        generated_basis_identity: generated_basis_identity.to_owned(),
        duration_offset_ticks,
        source_bridge_tick_count,
        original_v2_classification: classification,
        source_handoff,
        analytical_survivor: false,
        source_handoff_survivor: false,
        status: "analytical_skip".to_owned(),
        skip_reason: Some(reason.to_owned()),
        source_duration: None,
        paired_schedule: None,
        wrapper: None,
        accepted: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn complete_generated_row(
    request: &WaypointDirectNominalDirectGenerationRequest,
    request_identity: &str,
    scenario_identity: &str,
    policy_identity: &str,
    context: &RunContext,
    probe: &DirectBridgeProbeV2,
    vehicle: &VehicleInputV2,
    candidate: &DirectBridgeCandidateV2,
    selected: &super::super::PreparedProfileCandidate,
    generated_basis_identity: &str,
    mut source_duration: SourceDurationVariantEvidence,
) -> Result<WaypointDirectNominalDirectGenerationRowEvidence> {
    let row_index = source_duration.row_index;
    let basis_index = row_index / DURATION_OFFSETS_TICKS.len();
    let source_handoff = candidate
        .source_handoff
        .ok_or_else(|| anyhow!("generated certified basis has no source handoff"))?;
    let analytical_survivor = source_duration.analytical_survivor;
    let source_handoff_survivor = source_duration.source_handoff_survivor;
    let mut row = WaypointDirectNominalDirectGenerationRowEvidence {
        row_index,
        basis_index,
        basis_candidate_identity: candidate.identity.clone(),
        generated_basis_identity: generated_basis_identity.to_owned(),
        duration_offset_ticks: source_duration.duration_offset_ticks,
        source_bridge_tick_count: Some(source_duration.source_bridge_tick_count),
        original_v2_classification: candidate.classification,
        source_handoff: Some(source_handoff.state),
        analytical_survivor,
        source_handoff_survivor,
        status: "source_handoff_screen_skip".to_owned(),
        skip_reason: source_duration
            .source_handoff_skip_reason
            .clone()
            .or_else(|| source_duration.analysis_error.clone()),
        source_duration: Some(source_duration.clone()),
        paired_schedule: None,
        wrapper: None,
        accepted: false,
    };
    if !analytical_survivor {
        row.status = "analytical_screen_skip".to_owned();
        row.skip_reason = source_duration
            .analysis_error
            .clone()
            .or_else(|| source_duration.source_handoff_skip_reason.clone());
        return Ok(row);
    }
    if !source_handoff_survivor {
        return Ok(row);
    }
    let analysis = source_duration
        .launch_and_analytical_screen
        .as_ref()
        .ok_or_else(|| {
            anyhow!("source survivor row {row_index} has no launch/analysis evidence")
        })?;
    let source_gate = source_duration
        .source_handoff_gate
        .as_ref()
        .ok_or_else(|| anyhow!("source survivor row {row_index} has no source gate evidence"))?;
    let launch_target = source_duration
        .launch_target_attitude_rad
        .ok_or_else(|| anyhow!("source survivor row {row_index} has no launch target"))?;
    let source_ticks = source_duration.source_bridge_tick_count;
    let mut full_flights = Vec::with_capacity(2);
    let mut direct_prefix_matches = false;
    let mut held_baseline = None;
    let mut baseline_replay_parity = true;
    let mut direct_and_held_traces_passed = true;
    for cadence in [
        super::super::RolloutCadence::DirectPerTick,
        super::super::RolloutCadence::ControllerCadence,
    ] {
        let run = super::super::launch_feasibility::run_source_duration_variant_diagnostic(
            super::super::launch_feasibility::SourceDurationRunRequest {
                scenario: &request.scenario,
                probe,
                selected,
                basis: candidate,
                policy: &request.policy.analytical_policy,
                vehicle,
                source_bridge_steps: source_ticks,
                launch_tilt_attitude_rad: launch_target,
                cadence,
                stage: super::super::launch_feasibility::SourceDurationReplayStage::FullFlight,
            },
            super::super::launch_feasibility::SourceDurationHoldMode::Together,
        )?;
        let replay =
            super::super::launch_contact_contract::replay_logged_cadence(context, &run.run)?;
        let matches_analysis = run.run.launch == analysis.launch
            && run.run.reseeded_bridge == analysis.reseeded_bridge;
        baseline_replay_parity &= matches_analysis && replay.trace.passed;
        direct_and_held_traces_passed &= replay.trace.passed;
        let source_handoff_prefix_matches_gate = (cadence
            == super::super::RolloutCadence::DirectPerTick)
            .then(|| super::source_gate_prefix_matches_evidence(source_gate, analysis, &run.run));
        if cadence == super::super::RolloutCadence::DirectPerTick {
            direct_prefix_matches = source_handoff_prefix_matches_gate == Some(true);
        } else {
            held_baseline = Some(RunWithSamples {
                run: run.run.clone(),
                samples: run.state_samples.clone(),
                command_samples: run.command_samples.clone(),
            });
        }
        let rollout = &run.run.rollout;
        let strict_source_handoff = rollout.source_handoff_reached
            && rollout.source_handoff_contact_free
            && rollout
                .source_handoff_position_error_m
                .is_some_and(|error| error.is_finite() && error <= 1.0e-6)
            && rollout
                .source_handoff_velocity_error_mps
                .is_some_and(|error| error.is_finite() && error <= 1.0e-6);
        full_flights.push(FullFlightEvidence {
            cadence: run.run.cadence.clone(),
            launch_matches_analysis: run.run.launch == analysis.launch,
            source_handoff_prefix_matches_gate,
            status: rollout.status.clone(),
            source_handoff_reached: rollout.source_handoff_reached,
            source_handoff_contact_free: rollout.source_handoff_contact_free,
            source_handoff_position_error_m: rollout.source_handoff_position_error_m,
            source_handoff_velocity_error_mps: rollout.source_handoff_velocity_error_mps,
            strict_source_handoff_within_tolerance: strict_source_handoff,
            rollout: rollout.clone(),
            first_contact: replay.first_contact,
            contact_replay_trace: Some(replay.trace),
            contact_replay_error: None,
        });
    }
    source_duration.full_flights = full_flights;
    let held_baseline = held_baseline
        .ok_or_else(|| anyhow!("source survivor row {row_index} has no held-60 baseline"))?;
    baseline_replay_parity &= direct_and_held_traces_passed && direct_prefix_matches;
    let schedule = run_schedule_experiment_from_physical(
        &request.scenario,
        probe,
        &request.policy.analytical_policy,
        vehicle,
        candidate,
        selected,
        &PreparedDurationVariant {
            row_index,
            duration_offset_ticks: source_duration.duration_offset_ticks,
            source_bridge_tick_count: source_ticks,
            unlaunched_bridge: None,
            unlaunched_evidence: source_duration.unlaunched_source_bridge.clone(),
            launch_target_attitude_rad: Some(launch_target),
        },
        &held_baseline,
    )?;
    let verifier = super::CompleteWitnessVerifierRequest {
        context,
        scenario: &request.scenario,
        probe,
        candidate,
        selected,
        profile: &selected.profile,
        source_pad: &probe.source,
        target_pad: &probe.target,
        policy: &request.policy.analytical_policy,
        row_index,
        basis_candidate_identity: generated_basis_identity,
        source_bridge_tick_count: source_ticks,
        source_handoff: source_handoff.state,
        launch: &analysis.launch,
        reseeded_bridge: &analysis.reseeded_bridge,
        schedule: &schedule,
        scenario_identity,
        baseline_replay_parity,
        identity_bindings: super::CompleteWitnessIdentityBindings::Generated {
            request_identity,
            generation_policy_identity: policy_identity,
        },
    };
    let wrapper = super::verify_complete_witness(&verifier)?;
    row.status = if wrapper.accepted {
        "complete_witness_accepted".to_owned()
    } else {
        "complete_witness_rejected".to_owned()
    };
    row.skip_reason = None;
    row.accepted = wrapper.accepted;
    row.source_duration = Some(source_duration);
    row.paired_schedule = Some(schedule);
    row.wrapper = Some(wrapper);
    Ok(row)
}

fn select_generated_witness(
    rows: &[WaypointDirectNominalDirectGenerationRowEvidence],
    completion_gate_passed: bool,
) -> Option<AcceptedWrapperSelectionEvidence> {
    let candidates = rows.iter().filter_map(|row| {
        let wrapper = row.wrapper.as_ref()?;
        Some(GeneratedSelectionCandidate {
            row_index: row.row_index,
            wrapper_identity: wrapper.wrapper_identity.clone(),
            planned_total_mission_time_s: wrapper.planned_total_mission_time_s,
            accepted: row.accepted && wrapper.accepted,
        })
    });
    select_ranked_generated_candidate(candidates, completion_gate_passed)
}

struct GeneratedSelectionCandidate {
    row_index: usize,
    wrapper_identity: String,
    planned_total_mission_time_s: f64,
    accepted: bool,
}

fn select_ranked_generated_candidate(
    candidates: impl IntoIterator<Item = GeneratedSelectionCandidate>,
    completion_gate_passed: bool,
) -> Option<AcceptedWrapperSelectionEvidence> {
    if !completion_gate_passed {
        return None;
    }
    candidates
        .into_iter()
        .filter(|candidate| {
            candidate.accepted && candidate.planned_total_mission_time_s.is_finite()
        })
        .min_by(|left, right| {
            left.planned_total_mission_time_s
                .total_cmp(&right.planned_total_mission_time_s)
                .then_with(|| left.wrapper_identity.cmp(&right.wrapper_identity))
        })
        .map(|candidate| AcceptedWrapperSelectionEvidence {
            row_index: candidate.row_index,
            wrapper_identity: candidate.wrapper_identity,
            planned_total_mission_time_s: candidate.planned_total_mission_time_s,
            selection_rule: "complete accepted witnesses only; planned total mission time then stable witness identity".to_owned(),
        })
}

fn selected_witness_evidence(
    row: &WaypointDirectNominalDirectGenerationRowEvidence,
    selection: &AcceptedWrapperSelectionEvidence,
) -> Result<WaypointDirectNominalDirectGenerationSelectedWitnessEvidence> {
    let source_bridge_tick_count = row.source_bridge_tick_count.ok_or_else(|| {
        anyhow!(
            "selected row {} has no source bridge tick count",
            row.row_index
        )
    })?;
    let source_handoff = row
        .source_handoff
        .ok_or_else(|| anyhow!("selected row {} has no source handoff", row.row_index))?;
    let schedule = row
        .paired_schedule
        .as_ref()
        .ok_or_else(|| anyhow!("selected row {} has no command schedule", row.row_index))?;
    let wrapper = row
        .wrapper
        .as_ref()
        .ok_or_else(|| anyhow!("selected row {} has no accepted wrapper", row.row_index))?;
    if !row.accepted
        || !wrapper.accepted
        || wrapper.wrapper_identity != selection.wrapper_identity
        || row.row_index != selection.row_index
    {
        bail!("selected generated witness does not match its acceptance row");
    }
    Ok(
        WaypointDirectNominalDirectGenerationSelectedWitnessEvidence {
            row_index: row.row_index,
            basis_candidate_identity: row.basis_candidate_identity.clone(),
            generated_basis_identity: row.generated_basis_identity.clone(),
            duration_offset_ticks: row.duration_offset_ticks,
            source_bridge_tick_count,
            source_handoff,
            wrapper_identity: wrapper.wrapper_identity.clone(),
            command_schedule_identity: stable_digest(&schedule.commands)?,
            first_contact: wrapper.first_contact.clone(),
            planned_total_mission_time_s: wrapper.planned_total_mission_time_s,
            actual_touchdown_time_s: wrapper.actual_touchdown_time_s,
            actual_touchdown_fuel_used_kg: wrapper.actual_touchdown_fuel_used_kg,
        },
    )
}

fn compute_counters(
    rows: &[WaypointDirectNominalDirectGenerationRowEvidence],
    request: &WaypointDirectNominalDirectGenerationRequest,
    analytical_basis_candidates_evaluated: usize,
    complete_certified_basis_profiles: usize,
    duration_bridge_build_attempts: usize,
) -> Result<WaypointDirectNominalDirectGenerationComputeCounters> {
    let source_analysis_runs = rows
        .iter()
        .filter_map(|row| row.source_duration.as_ref())
        .filter(|duration| {
            duration.launch_and_analytical_screen.is_some() || duration.analysis_error.is_some()
        })
        .count();
    let source_handoff_runs = rows
        .iter()
        .filter_map(|row| row.source_duration.as_ref())
        .filter(|duration| {
            duration.source_handoff_gate.is_some() || duration.source_handoff_error.is_some()
        })
        .count();
    let full_flight_runs: usize = rows
        .iter()
        .filter_map(|row| row.source_duration.as_ref())
        .map(|duration| duration.full_flights.len())
        .sum();
    let schedules: Vec<&PairedScheduleEvidence> = rows
        .iter()
        .filter_map(|row| row.paired_schedule.as_ref())
        .collect();
    let paired_solver_iterations = schedules
        .iter()
        .map(|schedule| schedule.iterations.len())
        .sum();
    let finite_difference_columns = schedules
        .iter()
        .flat_map(|schedule| &schedule.iterations)
        .map(|iteration| iteration.finite_difference_columns)
        .sum();
    let line_search_trials = schedules
        .iter()
        .flat_map(|schedule| &schedule.iterations)
        .map(|iteration| iteration.line_search_trials.len())
        .sum();
    let complete_witness_verifications = rows.iter().filter(|row| row.wrapper.is_some()).count();

    // Use the fixed policy maxima for the declared budget. Observed iterations,
    // finite-difference columns, and line-search trials above remain separate
    // counters and cannot accidentally shrink this bound after a failed eval.
    let maximum_fit_evaluations = 1_usize.saturating_add(
        request.policy.maximum_solver_iterations.saturating_mul(
            2_usize
                .saturating_mul(4) // plus/minus evaluations for each of four parameters
                .saturating_add(request.policy.maximum_line_search_steps),
        ),
    );
    // These multipliers intentionally include source-run shadowing and replay
    // lanes; they bound work without pretending to be observed run counts.
    let source_analysis_run_equivalents = source_analysis_runs.saturating_mul(2);
    let source_handoff_run_equivalents = source_handoff_runs.saturating_mul(2);
    let baseline_full_flight_run_equivalents = full_flight_runs.saturating_mul(4);
    let solver_run_equivalents = schedules.len().saturating_mul(
        maximum_fit_evaluations
            .saturating_mul(2) // source-run/shadow lanes per fitter evaluation
            .saturating_add(4), // final full flight, shadow, ordinary, and neutral replay
    );
    let verifier_run_equivalents = complete_witness_verifications.saturating_mul(8);
    let run_equivalents = source_analysis_run_equivalents
        .saturating_add(source_handoff_run_equivalents)
        .saturating_add(baseline_full_flight_run_equivalents)
        .saturating_add(solver_run_equivalents)
        .saturating_add(verifier_run_equivalents);
    let maximum_ticks_per_run =
        (request.scenario.sim.max_time_s * f64::from(request.policy.physics_hz)).ceil() as u64;
    let physics_tick_budget_upper_bound = u64::try_from(run_equivalents)
        .ok()
        .and_then(|count| count.checked_mul(maximum_ticks_per_run))
        .ok_or_else(|| anyhow!("direct-generation physics tick budget upper bound overflowed"))?;

    Ok(WaypointDirectNominalDirectGenerationComputeCounters {
        analytical_basis_candidates_evaluated,
        complete_certified_basis_profiles,
        duration_rows_predeclared: request.policy.maximum_variants,
        duration_bridge_build_attempts,
        source_analysis_runs,
        source_handoff_runs,
        held_baseline_runs: schedules.len(),
        paired_schedule_fits: schedules.len(),
        paired_solver_iterations,
        finite_difference_columns,
        line_search_trials,
        complete_witness_verifications,
        physics_tick_budget_upper_bound,
        actual_physics_ticks: None,
        physics_tick_accounting:
            "Policy-worst-case lane bound: each source analysis/handoff charges two full horizons for run/shadow work; each baseline full-flight charges four for run/shadow plus ordinary/neutral replay; each paired fit charges twice the initial plus six-by-(eight finite-difference evaluations plus eight line-search trials) maximum evaluations, then four final-flight/replay lanes; each complete verifier charges eight horizons for independent source, authoritative, ordinary/neutral, and stored parity replay. Actual cumulative physics ticks are not instrumented; observed iterations, finite-difference columns, and line-search trials are reported separately.".to_owned(),
    })
}

fn artifact_identity(artifact: &WaypointDirectNominalDirectGenerationArtifact) -> Result<String> {
    let mut identity_input = artifact.clone();
    identity_input.identity.clear();
    stable_digest(&identity_input)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use pd_core::Vec2;

    use super::super::paired_command_feasibility::{
        HeldCommandEvidence, PhysicalScheduleScreenReplayRequest,
        replay_source_schedule_screens_from_physical,
    };
    use super::*;

    fn repository_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval has the workspace root as its parent")
    }

    fn known_flat_request() -> WaypointDirectNominalDirectGenerationRequest {
        crate::waypoint_direct_known_flat_generation_request(repository_root())
            .expect("known-flat input factory should succeed")
    }

    fn unique_temp_directory() -> PathBuf {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        for _ in 0..100 {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "pd-direct-generation-test-{}-{id}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return path,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("creating test directory {}: {error}", path.display()),
            }
        }
        panic!("could not allocate a unique direct-generation test directory")
    }

    fn certified_known_flat_basis(
        request: &WaypointDirectNominalDirectGenerationRequest,
    ) -> (
        usize,
        DirectBridgeCandidateV2,
        VehicleInputV2,
        DirectBridgeProbeV2,
    ) {
        let vehicle = super::super::super::vehicle_input_v2(&request.scenario.vehicle);
        let probe = generated_probe(request).expect("known-flat pads should exist");
        let evaluated =
            evaluate_direct_bridge_case_v2(&request.policy.analytical_policy, &vehicle, &probe)
                .expect("known-flat analytical basis evaluation should succeed");
        let (index, candidate) = evaluated
            .candidates
            .iter()
            .enumerate()
            .find(|(_, candidate)| candidate.classification == CertificationV2::Certified)
            .expect("known-flat family should retain a certified seed");
        (index, candidate.clone(), vehicle, probe)
    }

    #[test]
    fn validation_distinguishes_malformed_unsupported_and_tuned_policy_inputs() {
        let request = known_flat_request();
        let validation = validate_waypoint_direct_nominal_direct_generation_request(&request)
            .expect("known-flat input should pass no-physics validation");
        assert!(!validation.simulation_created);

        let mut malformed = request.clone();
        malformed.probe_id.clear();
        let error = validate_waypoint_direct_nominal_direct_generation_request(&malformed)
            .expect_err("empty probe identity must fail as malformed input");
        assert!(error.to_string().contains("malformed request"));

        let mut unsupported = request.clone();
        unsupported.scenario.initial_state.attitude_rad = 0.01;
        let error = validate_waypoint_direct_nominal_direct_generation_request(&unsupported)
            .expect_err("non-upright source rest must be unsupported");
        assert!(error.to_string().contains("unsupported source state"));

        let mut tuned = request;
        tuned.policy.maximum_line_search_steps += 1;
        let error = validate_waypoint_direct_nominal_direct_generation_request(&tuned)
            .expect_err("caller-tuned generation policy must be rejected");
        assert!(error.to_string().contains("unsupported request policy"));
    }

    #[test]
    fn nonpositive_and_odd_source_durations_are_retained_as_explicit_skips() {
        let request = known_flat_request();
        let (basis_index, mut candidate, vehicle, _) = certified_known_flat_basis(&request);
        candidate
            .source_bridge
            .as_mut()
            .expect("certified seed should carry its source bridge")
            .steps = 1;
        let generated_identity = "test-generated-basis";
        let (variants, skips) = prepare_generated_duration_variants(
            basis_index,
            &candidate,
            generated_identity,
            &request.policy.analytical_policy,
            &vehicle,
        )
        .expect("invalid offsets are recorded without substitution");

        assert!(variants.is_empty());
        assert_eq!(skips.len(), DURATION_OFFSETS_TICKS.len());
        assert_eq!(
            skips.iter().map(|row| row.row_index).collect::<Vec<_>>(),
            (basis_index * DURATION_OFFSETS_TICKS.len()
                ..(basis_index + 1) * DURATION_OFFSETS_TICKS.len())
                .collect::<Vec<_>>()
        );
        assert!(skips[..4].iter().all(|row| {
            row.skip_reason.as_deref() == Some("source_duration_tick_count_nonpositive")
        }));
        assert_eq!(
            skips[4].skip_reason.as_deref(),
            Some("source_duration_not_on_60hz_controller_cadence")
        );
    }

    #[test]
    fn existing_output_directory_is_refused_before_summary_creation() {
        let request = known_flat_request();
        let temporary = unique_temp_directory();
        let output_dir = temporary.join("occupied");
        fs::create_dir(&output_dir).expect("create occupied output directory");

        let error =
            run_waypoint_direct_nominal_direct_generation(repository_root(), &request, &output_dir)
                .expect_err("create-only writer must refuse any existing output directory");
        assert!(error.to_string().contains("existing output directory"));
        assert!(!output_dir.join("summary.json").exists());
        assert_eq!(
            fs::read_dir(&output_dir)
                .expect("output directory remains inspectable")
                .count(),
            0
        );
        fs::remove_dir_all(temporary).expect("remove only this test's temporary directory");
    }

    #[test]
    fn selection_uses_only_accepted_witnesses_then_planned_time_and_identity() {
        let candidates = vec![
            GeneratedSelectionCandidate {
                row_index: 0,
                wrapper_identity: "rejected-earlier".to_owned(),
                planned_total_mission_time_s: 10.0,
                accepted: false,
            },
            GeneratedSelectionCandidate {
                row_index: 8,
                wrapper_identity: "tie-z".to_owned(),
                planned_total_mission_time_s: 34.0,
                accepted: true,
            },
            GeneratedSelectionCandidate {
                row_index: 3,
                wrapper_identity: "tie-a".to_owned(),
                planned_total_mission_time_s: 34.0,
                accepted: true,
            },
            GeneratedSelectionCandidate {
                row_index: 4,
                wrapper_identity: "later".to_owned(),
                planned_total_mission_time_s: 35.0,
                accepted: true,
            },
        ];
        let selected = select_ranked_generated_candidate(candidates, true)
            .expect("at least one finite accepted candidate should be selectable");
        assert_eq!(selected.row_index, 3);
        assert_eq!(selected.wrapper_identity, "tie-a");
        assert_eq!(selected.planned_total_mission_time_s, 34.0);

        assert!(
            select_ranked_generated_candidate(
                [GeneratedSelectionCandidate {
                    row_index: 3,
                    wrapper_identity: "accepted".to_owned(),
                    planned_total_mission_time_s: 34.0,
                    accepted: true,
                }],
                false,
            )
            .is_none()
        );
    }

    #[test]
    fn independent_schedule_replay_rejects_tampered_pair_provenance_before_physics() {
        let request = known_flat_request();
        let (basis_index, _candidate, vehicle, probe) = certified_known_flat_basis(&request);
        let v2_result =
            evaluate_direct_bridge_case_v2(&request.policy.analytical_policy, &vehicle, &probe)
                .expect("known-flat analytical basis evaluation should succeed");
        let candidate = &v2_result.candidates[basis_index];
        let profile = super::super::super::materialize_profile(
            &v2_result,
            candidate,
            &probe,
            &request.policy.analytical_policy,
            &vehicle,
        )
        .expect("known-flat certified profile should materialize");
        let selected = super::super::super::PreparedProfileCandidate {
            candidate_identity: candidate.identity.clone(),
            selected_roles: Vec::new(),
            profile,
        };
        let commands = [HeldCommandEvidence {
            pair_index: 1,
            is_first_powered_pair: true,
            first_physics_step: 73,
            last_physics_step: 74,
            desired_thrust_acceleration_mps2: Vec2::new(0.0, 0.0),
            target_attitude_rad: 0.0,
            commanded_throttle_frac: None,
            applied_throttle_fraction_first_tick: None,
            applied_throttle_fraction_second_tick: None,
        }];
        let replay_request = PhysicalScheduleScreenReplayRequest {
            scenario: &request.scenario,
            probe: &probe,
            policy: &request.policy.analytical_policy,
            vehicle: &vehicle,
            candidate,
            selected: &selected,
            source_bridge_tick_count: 2,
            launch_tilt_attitude_rad: 0.0,
            commands: &commands,
        };
        let error = replay_source_schedule_screens_from_physical(&replay_request)
            .expect_err("changed held-command pair index must fail before replay");
        assert!(error.to_string().contains("provenance changed at pair 0"));
    }
}
