//! Opt-in first-contact contract audit for frozen launch-feasibility replays.
//!
//! The launch-feasibility artifact remains sealed and unchanged. This lane
//! rebuilds its frozen inputs, replays the recorded held commands through a
//! paired ordinary and pre-terminal simulation state, and writes a separate
//! identity-bound diagnostic.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::{
    Command, ContactClassification, EventKind, RunContext, SimulationState, TerrainDefinition, Vec2,
};
use serde::{Deserialize, Serialize};

use super::launch_feasibility::{SourceDurationHoldMode, component_update_schedule};
use super::{
    LaunchFeasibilityCadenceRunEvidence, LaunchFeasibilityInputGateEvidence,
    LaunchFeasibilityProfileBindingEvidence, LaunchRolloutTickEvidence, LaunchTickEvidence,
    PlantStateEvidence, PreparedCaseEvidence, PreparedInputs, RolloutCadence,
    SelectionRoleEvidence, WaypointDirectLaunchFeasibilityArtifact,
    WaypointDirectNominalPlantArtifact, build_input_manifest, enum_label, resolve_output_dir,
    stable_digest,
};

pub const WAYPOINT_DIRECT_LAUNCH_CONTACT_CONTRACT_ID: &str =
    "waypoint-direct-launch-contact-contract";
pub const WAYPOINT_DIRECT_LAUNCH_CONTACT_CONTRACT_SCHEMA_ID: &str =
    "waypoint_direct_launch_contact_contract_v1";
pub const WAYPOINT_DIRECT_LAUNCH_CONTACT_CONTRACT_SCHEMA_VERSION: u32 = 1;

const EXPECTED_BASELINE_IDENTITY: &str = "fnv1a64:d3fa6b24336f7c05";
const EXPECTED_SWEEP_IDENTITY: &str = "fnv1a64:1bcd5a3bd6c6da01";
const EXPECTED_NOMINAL_IDENTITY: &str = "fnv1a64:9e8cbc902ca11fbc";
const EXPECTED_LAUNCH_IDENTITY: &str = "fnv1a64:2c7b965ffdc809d6";
const EXPECTED_LAUNCH_INPUT_GATE_IDENTITY: &str = "fnv1a64:a4654d06c9166b14";
const FLAT_CASE_ID: &str = "continuous_flat_r00";
const EXPECTED_CASES: [&str; 6] = [
    "continuous_flat_r00",
    "continuous_uphill_r+30",
    "continuous_downhill_r-30",
    "center_050_width_025_height_020",
    "center_050_width_025_height_030",
    "center_050_width_025_height_040",
];
const CADENCE_LABELS: [&str; 2] = ["direct_per_tick_120_hz", "held_controller_60_hz"];
const FOOT_HALF_SPAN_M: f64 = 4.0;
const FOOT_BASE_OFFSET_M: f64 = 5.0;
const ORIENTATION_TILT_RAD: f64 = 0.1;

#[derive(Clone, Debug, Serialize)]
pub struct WaypointDirectLaunchContactContractPaths {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WaypointDirectLaunchContactContractRun {
    pub artifact: WaypointDirectLaunchContactContractArtifact,
    pub paths: WaypointDirectLaunchContactContractPaths,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchContactContractInputGateEvidence {
    pub schema_id: String,
    pub schema_version: u32,
    pub baseline_identity: String,
    pub sweep_identity: String,
    pub nominal_plant_identity: String,
    pub nominal_input_manifest_identity: String,
    pub launch_feasibility_identity: String,
    pub launch_feasibility_input_gate_identity: String,
    pub launch_input_gate: LaunchFeasibilityInputGateEvidence,
    pub cases: Vec<LaunchContactContractInputCaseEvidence>,
    pub case_count: u32,
    pub unique_profile_count: u32,
    pub selected_role_count: u32,
    pub existing_cadence_row_count: u32,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchContactContractInputCaseEvidence {
    pub input: PreparedCaseEvidence,
    pub selection_roles: Vec<SelectionRoleEvidence>,
    pub profile_bindings: Vec<LaunchFeasibilityProfileBindingEvidence>,
    pub available_candidate_identities: Vec<String>,
    pub available_cadence_labels: Vec<String>,
    pub launch_case_status: String,
    pub launch_skip_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointDirectLaunchContactContractArtifact {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub baseline_identity: String,
    pub sweep_identity: String,
    pub nominal_plant_identity: String,
    pub nominal_input_manifest_identity: String,
    pub launch_feasibility_identity: String,
    pub launch_feasibility_input_gate_identity: String,
    pub input_gate_identity: String,
    pub protocol: LaunchContactContractProtocolEvidence,
    pub orientation_probes: Vec<OrientationConventionProbeEvidence>,
    pub cross_model_certificate_claim_allowed: bool,
    pub flat_gate: FlatContactContractGateEvidence,
    pub cases: Vec<LaunchContactContractCaseEvidence>,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchContactContractProtocolEvidence {
    pub replay_source: String,
    pub paired_step_rule: String,
    pub cadence_rule: String,
    pub predicate_mirror_rule: String,
    pub orientation_rule: String,
    pub flat_first_rule: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatContactContractGateEvidence {
    pub case_id: String,
    pub required_role_order: Vec<String>,
    pub audited_candidate_count: usize,
    pub audited_cadence_row_count: usize,
    pub required_role_bindings_valid: bool,
    pub all_required_cadence_rows_present: bool,
    pub all_first_contacts_observed: bool,
    pub all_replay_and_classifier_parity_passed: bool,
    pub native_stable_landing_observed: bool,
    pub research_shortest_crash_observed: bool,
    pub passed: bool,
    pub consequence: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchContactContractCaseEvidence {
    pub input: PreparedCaseEvidence,
    pub selection_roles: Vec<SelectionRoleEvidence>,
    pub status: String,
    pub skip_reason: Option<String>,
    pub candidate_profiles: Vec<LaunchContactContractCandidateEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchContactContractCandidateEvidence {
    pub candidate_identity: String,
    pub selected_roles: Vec<String>,
    pub launch_candidate_identity: Option<String>,
    pub launch_candidate_available: bool,
    pub cadence_runs: Vec<LaunchContactContractCadenceEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaunchContactContractCadenceEvidence {
    pub candidate_identity: String,
    pub cadence: String,
    pub status: String,
    pub skip_reason: Option<String>,
    pub trace: Option<ReplayTraceParityEvidence>,
    pub first_contact: Option<FirstContactEvidence>,
    pub profile_end: Option<PlantStateEvidence>,
    pub post_terminal_state: Option<PlantStateEvidence>,
    pub source_handoff_position_error_m: Option<f64>,
    pub source_handoff_velocity_error_mps: Option<f64>,
    pub reseeded_bridge_analytically_eligible: Option<bool>,
    pub reseeded_bridge_source_clearance: Option<super::SourceClearanceScreenEvidence>,
    pub reseeded_bridge_source_attitude_margin: Option<super::MarginEvidence>,
    pub reseeded_bridge_launch_boundary: Option<super::LaunchBoundaryEvidence>,
    pub reseeded_bridge_aggregate_fuel_margin: Option<super::MarginEvidence>,
    pub reseeded_bridge_aggregate_time_margin: Option<super::MarginEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayTraceParityEvidence {
    pub physics_ticks_replayed: u64,
    pub first_physics_step: Option<u64>,
    pub last_physics_step: Option<u64>,
    pub contiguous_from_step_one: bool,
    pub command_schedule_parity: bool,
    pub preterminal_state_parity: bool,
    pub local_mirror_parity: bool,
    pub neutral_to_event_parity: bool,
    pub event_to_launch_log_parity: bool,
    pub terminal_outcome_parity: bool,
    pub first_mismatch: Option<ReplayMismatchEvidence>,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayMismatchEvidence {
    pub physics_step: u64,
    pub phase: String,
    pub field: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrajectorySignGeometryScanEvidence {
    pub physics_ticks_sampled: u64,
    pub first_physics_step: Option<u64>,
    pub last_physics_step: Option<u64>,
    pub original_terminal_physics_step: Option<u64>,
    pub reached_original_terminal: bool,
    pub core_first_geometric_gate_step: Option<u64>,
    pub opposite_sign_first_geometric_gate_step: Option<u64>,
    pub geometric_gate_order: String,
    pub geometry_gate_disagreement_tick_count: u64,
    pub pad_membership_disagreement_tick_count: u64,
    pub stable_predicate_disagreement_tick_count: u64,
    pub earliest_geometry_gate_disagreement: Option<TrajectorySignDisagreementEvidence>,
    pub earliest_pad_membership_disagreement: Option<TrajectorySignDisagreementEvidence>,
    pub earliest_stable_predicate_disagreement: Option<TrajectorySignDisagreementEvidence>,
    pub scope: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrajectorySignDisagreementEvidence {
    pub physics_step: u64,
    pub phase: String,
    pub original_contact_classification: String,
    pub core_value: bool,
    pub opposite_sign_value: bool,
    pub differing_predicates: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FirstContactEvidence {
    pub physics_step: u64,
    pub classification: String,
    pub state: PreterminalContactStateEvidence,
    pub predicates: ContactPredicateMirrorEvidence,
    pub v2_sign_geometry_counterfactual: V2SignGeometryCounterfactualEvidence,
    pub neutral_matches_local_mirror: bool,
    pub ordinary_event_matches_neutral: bool,
    pub launch_log_matches_event: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct V2SignGeometryCounterfactualEvidence {
    pub convention: String,
    pub thrust_direction_unit: Vec2,
    pub footprint_attitude_rad: f64,
    pub feet: [ContactFootEvidence; 2],
    pub minimum_touchdown_clearance_m: f64,
    pub maximum_touchdown_clearance_m: f64,
    pub minimum_hull_clearance_m: f64,
    pub no_contact_clearance_gate: bool,
    pub no_contact_clearance_gate_differs_from_core: bool,
    pub touchdown_pad_contains_both_feet: bool,
    pub stable_minimum_clearance_predicate: bool,
    pub stable_maximum_clearance_predicate: bool,
    pub stable_hull_penetration_predicate: bool,
    pub stable_touchdown_predicate: bool,
    pub stable_geometry_predicates_differ_from_core: bool,
    pub pad_membership_differs_from_core: bool,
    pub geometry_only_counterfactual: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreterminalContactStateEvidence {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
    pub touchdown_feet: [ContactFootEvidence; 2],
    pub minimum_hull_clearance_m: f64,
    pub touchdown_pad_contains_both_feet: bool,
    pub normal_closing_speed_mps: f64,
    pub tangential_closing_speed_mps: f64,
    pub attitude_error_rad: f64,
    pub absolute_angular_rate_radps: f64,
    pub dynamic_hull_penetration_allowance_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactFootEvidence {
    pub position_m: Vec2,
    pub signed_clearance_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactPredicateMirrorEvidence {
    pub no_contact_clearance_gate: bool,
    pub stable_minimum_clearance_predicate: bool,
    pub stable_maximum_clearance_predicate: bool,
    pub stable_hull_penetration_predicate: bool,
    pub stable_touchdown_predicate: bool,
    pub safe_normal_speed_predicate: bool,
    pub safe_tangential_speed_predicate: bool,
    pub safe_attitude_predicate: bool,
    pub safe_angular_rate_predicate: bool,
    pub safe_touchdown_predicate: bool,
    pub predicted_classification: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrientationConventionProbeEvidence {
    pub tilt_attitude_rad: f64,
    pub thrust_direction_unit: Vec2,
    pub core_body_up_after_rotation: Vec2,
    pub core_footprint_attitude_rad: f64,
    pub v2_footprint_attitude_rad: f64,
    pub signed_footprint_attitude_delta_rad: f64,
    pub centered_flat_core_footprint: OrientationFootprintEvidence,
    pub centered_flat_v2_footprint: OrientationFootprintEvidence,
    pub centered_slope_core_footprint: OrientationFootprintEvidence,
    pub centered_slope_v2_footprint: OrientationFootprintEvidence,
    pub flat_pad_edge_core_footprint: OrientationFootprintEvidence,
    pub flat_pad_edge_v2_footprint: OrientationFootprintEvidence,
    pub synthetic_only: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrientationFootprintEvidence {
    pub vehicle_center_m: Vec2,
    pub pad_center_x_m: f64,
    pub pad_half_width_m: f64,
    pub terrain_rule: String,
    pub feet: [ContactFootEvidence; 2],
    pub pad_contains_both_feet: bool,
}

/// Validate every frozen input and role binding without constructing a
/// `SimulationState` or advancing physics.
pub fn validate_waypoint_direct_launch_contact_contract_inputs(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    nominal_summary_path: &Path,
    launch_summary_path: &Path,
) -> Result<LaunchContactContractInputGateEvidence> {
    let (prepared, nominal, launch, launch_gate) = load_and_validate_inputs(
        repo_root,
        baseline_summary_path,
        sweep_summary_path,
        nominal_summary_path,
        launch_summary_path,
    )?;
    let mut gate = build_contact_contract_input_gate(&prepared, &nominal, &launch, &launch_gate)?;
    gate.identity = input_gate_identity(&gate)?;
    let bytes = serde_json::to_vec(&gate)?;
    let reloaded: LaunchContactContractInputGateEvidence = serde_json::from_slice(&bytes)?;
    if reloaded != gate || input_gate_identity(&reloaded)? != reloaded.identity {
        bail!("launch contact-contract input gate identity round-trip failed");
    }
    Ok(reloaded)
}

/// Replay exactly the launch rows present in the frozen launch-feasibility
/// artifact, preserving its flat-first stop gate and command traces.
pub fn run_waypoint_direct_launch_contact_contract(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    nominal_summary_path: &Path,
    launch_summary_path: &Path,
    output_dir: &Path,
) -> Result<WaypointDirectLaunchContactContractRun> {
    let (prepared, nominal, launch, launch_gate) = load_and_validate_inputs(
        repo_root,
        baseline_summary_path,
        sweep_summary_path,
        nominal_summary_path,
        launch_summary_path,
    )?;
    let mut input_gate =
        build_contact_contract_input_gate(&prepared, &nominal, &launch, &launch_gate)?;
    input_gate.identity = input_gate_identity(&input_gate)?;
    let orientation_probes = orientation_convention_probes();

    let flat_case_index = prepared
        .cases
        .iter()
        .position(|case| case.evidence.id == FLAT_CASE_ID)
        .ok_or_else(|| anyhow!("frozen flat case is missing"))?;
    let flat_profiles = flat_profiles_in_role_order(&prepared.cases[flat_case_index]);
    let flat_launch_case = launch
        .cases
        .iter()
        .find(|case| case.input.id == FLAT_CASE_ID)
        .ok_or_else(|| anyhow!("launch artifact has no flat case row"))?;
    let mut flat_rows = Vec::new();
    for selected in flat_profiles.iter().copied() {
        let launch_candidate = flat_launch_case
            .candidate_profiles
            .iter()
            .find(|candidate| candidate.basis_candidate_identity == selected.candidate_identity);
        for cadence in CADENCE_LABELS {
            let run = launch_candidate.and_then(|candidate| {
                candidate
                    .cadence_runs
                    .iter()
                    .find(|run| run.cadence == cadence)
            });
            flat_rows.push(audit_or_skip_cadence(
                &prepared.cases[flat_case_index],
                selected,
                cadence,
                run,
                None,
            )?);
        }
    }
    let role_bindings_valid =
        flat_role_bindings_valid(&prepared.cases[flat_case_index], &flat_profiles);
    let all_required_cadence_rows_present =
        flat_rows_cover_required_bindings(&flat_profiles, &flat_rows);
    let all_first_contacts_observed = all_required_cadence_rows_present
        && flat_rows.iter().all(|row| row.first_contact.is_some());
    let all_flat_parity_passed = all_required_cadence_rows_present
        && flat_rows.iter().all(|row| {
            row.trace.as_ref().is_some_and(|trace| trace.passed)
                && row.first_contact.as_ref().is_some_and(|contact| {
                    contact.neutral_matches_local_mirror
                        && contact.ordinary_event_matches_neutral
                        && contact.launch_log_matches_event
                })
        });
    let native_stable_landing_observed = flat_profiles.first().is_some_and(|native| {
        flat_rows
            .iter()
            .filter(|row| row.candidate_identity == native.candidate_identity)
            .count()
            == CADENCE_LABELS.len()
            && CADENCE_LABELS.iter().all(|cadence| {
                flat_rows.iter().any(|row| {
                    row.candidate_identity == native.candidate_identity
                        && row.cadence == *cadence
                        && row.first_contact.as_ref().is_some_and(|contact| {
                            contact.classification == "stable_touchdown_on_target"
                        })
                })
            })
    });
    let research_shortest_crash_observed = flat_profiles.get(1).is_some_and(|shortest| {
        flat_rows
            .iter()
            .filter(|row| row.candidate_identity == shortest.candidate_identity)
            .count()
            == CADENCE_LABELS.len()
            && CADENCE_LABELS.iter().all(|cadence| {
                flat_rows.iter().any(|row| {
                    row.candidate_identity == shortest.candidate_identity
                        && row.cadence == *cadence
                        && row
                            .first_contact
                            .as_ref()
                            .is_some_and(|contact| contact.classification == "crash")
                })
            })
    });
    let mut flat_gate = FlatContactContractGateEvidence {
        case_id: FLAT_CASE_ID.to_owned(),
        required_role_order: vec![
            "v2_native_selected".to_owned(),
            "research_shortest_certified".to_owned(),
        ],
        audited_candidate_count: flat_profiles.len(),
        audited_cadence_row_count: flat_rows.len(),
        required_role_bindings_valid: role_bindings_valid,
        all_required_cadence_rows_present,
        all_first_contacts_observed,
        all_replay_and_classifier_parity_passed: all_flat_parity_passed,
        native_stable_landing_observed,
        research_shortest_crash_observed,
        passed: false,
        consequence: String::new(),
    };
    let flat_gate_passed = flat_gate_satisfied(&flat_gate);
    flat_gate.passed = flat_gate_passed;
    flat_gate.consequence = if flat_gate_passed {
        "flat contact-contract parity passed; continue through only the remaining profiles materialized by the frozen launch artifact".to_owned()
    } else {
        "stop after both frozen flat roles and both cadences; retain all remaining profile rows as explicitly skipped".to_owned()
    };

    let mut cases = Vec::with_capacity(prepared.cases.len());
    for case in &prepared.cases {
        let launch_case = launch
            .cases
            .iter()
            .find(|candidate| candidate.input.id == case.evidence.id)
            .ok_or_else(|| anyhow!("launch artifact is missing case {}", case.evidence.id))?;
        let selection_roles = launch_case.selection_roles.clone();
        let ordered = if case.evidence.id == FLAT_CASE_ID {
            flat_profiles_in_role_order(case)
        } else {
            case.profile_candidates.iter().collect::<Vec<_>>()
        };
        let mut candidate_profiles = Vec::with_capacity(ordered.len());
        for selected in ordered {
            let launch_candidate = launch_case.candidate_profiles.iter().find(|candidate| {
                candidate.basis_candidate_identity == selected.candidate_identity
            });
            let mut cadence_runs = Vec::with_capacity(2);
            for cadence in CADENCE_LABELS {
                let flat_row = (case.evidence.id == FLAT_CASE_ID)
                    .then(|| {
                        flat_rows
                            .iter()
                            .find(|row| {
                                row.candidate_identity == selected.candidate_identity
                                    && row.cadence == cadence
                            })
                            .cloned()
                    })
                    .flatten();
                if let Some(row) = flat_row {
                    cadence_runs.push(row);
                    continue;
                }
                let prior_flat_failure = case.evidence.id != FLAT_CASE_ID && !flat_gate_passed;
                let run = launch_candidate.and_then(|candidate| {
                    candidate
                        .cadence_runs
                        .iter()
                        .find(|run| run.cadence == cadence)
                });
                cadence_runs.push(audit_or_skip_cadence(
                    case,
                    selected,
                    cadence,
                    run,
                    prior_flat_failure.then_some("flat_contact_contract_parity_failed"),
                )?);
            }
            candidate_profiles.push(LaunchContactContractCandidateEvidence {
                candidate_identity: selected.candidate_identity.clone(),
                selected_roles: selected.selected_roles.clone(),
                launch_candidate_identity: launch_candidate
                    .map(|candidate| candidate.identity.clone()),
                launch_candidate_available: launch_candidate.is_some(),
                cadence_runs,
            });
        }
        let launched_count = candidate_profiles
            .iter()
            .filter(|profile| profile.launch_candidate_available)
            .count();
        let case_status = if case.evidence.id == FLAT_CASE_ID {
            if flat_gate_passed {
                "audited_flat_gate"
            } else {
                "flat_gate_failed"
            }
        } else if !flat_gate_passed {
            "skipped_flat_gate_failed"
        } else if launched_count == 0 {
            "not_materialized_by_launch_gate"
        } else {
            "audited_conditional_profiles"
        };
        let case_skip_reason = match case_status {
            "skipped_flat_gate_failed" => Some("flat_contact_contract_parity_failed".to_owned()),
            "not_materialized_by_launch_gate" => Some(
                launch_case
                    .skip_reason
                    .clone()
                    .unwrap_or_else(|| "profile_not_materialized_by_launch_gate".to_owned()),
            ),
            "flat_gate_failed" => Some("flat_contact_contract_parity_failed".to_owned()),
            _ => None,
        };
        cases.push(LaunchContactContractCaseEvidence {
            input: case.evidence.clone(),
            selection_roles,
            status: case_status.to_owned(),
            skip_reason: case_skip_reason,
            candidate_profiles,
        });
    }

    let mut artifact = WaypointDirectLaunchContactContractArtifact {
        schema_id: WAYPOINT_DIRECT_LAUNCH_CONTACT_CONTRACT_SCHEMA_ID.to_owned(),
        schema_version: WAYPOINT_DIRECT_LAUNCH_CONTACT_CONTRACT_SCHEMA_VERSION,
        characterization_id: WAYPOINT_DIRECT_LAUNCH_CONTACT_CONTRACT_ID.to_owned(),
        baseline_identity: prepared.baseline.identity.clone(),
        sweep_identity: prepared.sweep.identity.clone(),
        nominal_plant_identity: nominal.identity.clone(),
        nominal_input_manifest_identity: nominal.input_manifest_identity.clone(),
        launch_feasibility_identity: launch.identity.clone(),
        launch_feasibility_input_gate_identity: launch.input_gate_identity.clone(),
        input_gate_identity: input_gate.identity,
        protocol: contact_contract_protocol(),
        cross_model_certificate_claim_allowed: false,
        orientation_probes,
        flat_gate,
        cases,
        scope_non_claims: vec![
            "The audit replays only the launch cadence rows already present in the frozen launch-feasibility artifact.".to_owned(),
            "The evaluator-local predicate mirror is diagnostic and does not alter pd-core contact classification.".to_owned(),
            "The synthetic orientation probes show a convention comparison and are not selected-profile or mission evidence.".to_owned(),
            "A passing flat gate permits only the remaining profiles materialized by the frozen launch artifact; it does not add candidates or held-out evidence.".to_owned(),
            "This outcome-known contact contract audit is not a V2 certificate or a planner, controller, F6, or default-selection rule.".to_owned(),
        ],
        identity: String::new(),
    };
    artifact.identity = artifact_identity(&artifact)?;

    let output_dir = resolve_output_dir(repo_root, output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create launch contact-contract output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    let summary_bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&summary_path, &summary_bytes).with_context(|| {
        format!(
            "failed to write launch contact-contract summary {}",
            summary_path.display()
        )
    })?;
    let reloaded: WaypointDirectLaunchContactContractArtifact =
        serde_json::from_slice(&summary_bytes)
            .context("failed to reload launch contact-contract summary")?;
    if serde_json::to_vec_pretty(&reloaded)? != summary_bytes
        || artifact_identity(&reloaded)? != reloaded.identity
    {
        bail!("launch contact-contract summary failed deterministic identity round-trip");
    }
    Ok(WaypointDirectLaunchContactContractRun {
        artifact: reloaded,
        paths: WaypointDirectLaunchContactContractPaths {
            output_dir,
            summary_path,
        },
    })
}

pub(super) fn load_and_validate_inputs(
    repo_root: &Path,
    baseline_summary_path: &Path,
    sweep_summary_path: &Path,
    nominal_summary_path: &Path,
    launch_summary_path: &Path,
) -> Result<(
    PreparedInputs,
    WaypointDirectNominalPlantArtifact,
    WaypointDirectLaunchFeasibilityArtifact,
    LaunchFeasibilityInputGateEvidence,
)> {
    let (prepared, nominal) = super::source_contact::prepare_source_contact_inputs(
        repo_root,
        baseline_summary_path,
        sweep_summary_path,
        nominal_summary_path,
    )?;
    if prepared.baseline.identity != EXPECTED_BASELINE_IDENTITY
        || prepared.sweep.identity != EXPECTED_SWEEP_IDENTITY
        || nominal.identity != EXPECTED_NOMINAL_IDENTITY
    {
        bail!(
            "rebuilt baseline, sweep, or nominal identity differs from the contact-contract protocol"
        );
    }
    let manifest = build_input_manifest(&prepared)?;
    if manifest.identity != nominal.input_manifest_identity {
        bail!("rebuilt nominal input manifest does not match the frozen nominal artifact");
    }
    let launch_gate = super::launch_feasibility::build_input_gate(&prepared, &nominal)?;
    if launch_gate.identity != EXPECTED_LAUNCH_INPUT_GATE_IDENTITY {
        bail!(
            "rebuilt launch-feasibility input gate differs from the frozen contact-contract protocol"
        );
    }
    let launch: WaypointDirectLaunchFeasibilityArtifact = super::read_summary(launch_summary_path)?;
    validate_pinned_launch_identity(&launch)?;
    validate_launch_source_bindings(&prepared, &nominal, &launch, &launch_gate)?;
    Ok((prepared, nominal, launch, launch_gate))
}

fn validate_launch_source_bindings(
    prepared: &PreparedInputs,
    nominal: &WaypointDirectNominalPlantArtifact,
    launch: &WaypointDirectLaunchFeasibilityArtifact,
    launch_gate: &LaunchFeasibilityInputGateEvidence,
) -> Result<()> {
    if launch.schema_id != super::WAYPOINT_DIRECT_LAUNCH_FEASIBILITY_SCHEMA_ID
        || launch.schema_version != super::WAYPOINT_DIRECT_LAUNCH_FEASIBILITY_SCHEMA_VERSION
        || launch.characterization_id != super::WAYPOINT_DIRECT_LAUNCH_FEASIBILITY_ID
        || launch.baseline_identity != prepared.baseline.identity
        || launch.sweep_identity != prepared.sweep.identity
        || launch.nominal_plant_identity != nominal.identity
        || launch.nominal_input_manifest_identity != nominal.input_manifest_identity
        || launch.input_gate_identity != launch_gate.identity
        || launch.protocol != launch_gate.protocol
    {
        bail!("launch-feasibility source identity or protocol binding changed");
    }
    if !matches!(
        launch.execution_stage.as_str(),
        "flat_gate_only" | "conditional_full"
    ) {
        bail!("launch-feasibility execution stage is not recognized");
    }
    if prepared.cases.len() != EXPECTED_CASES.len()
        || launch.cases.len() != EXPECTED_CASES.len()
        || prepared
            .cases
            .iter()
            .map(|case| case.evidence.id.as_str())
            .ne(EXPECTED_CASES)
        || launch
            .cases
            .iter()
            .map(|case| case.input.id.as_str())
            .ne(EXPECTED_CASES)
    {
        bail!("launch contact-contract input requires the exact six frozen case rows");
    }
    if launch_gate.case_count != 6
        || launch_gate.unique_profile_count != 9
        || launch_gate.role_count != 12
        || launch_gate.cases.len() != prepared.cases.len()
    {
        bail!("rebuilt launch input gate has unexpected case, profile, or role counts");
    }

    let flat_index = EXPECTED_CASES
        .iter()
        .position(|id| *id == FLAT_CASE_ID)
        .expect("flat case is in fixed case list");
    let flat_row = &launch.cases[flat_index];
    let flat_expected_profiles = &prepared.cases[flat_index].profile_candidates;
    let flat_ordered_profiles = flat_profiles_in_role_order(&prepared.cases[flat_index]);
    if !flat_role_bindings_valid(&prepared.cases[flat_index], &flat_ordered_profiles) {
        bail!(
            "frozen flat native and research-shortest role bindings must be distinct and complete"
        );
    }
    let flat_contact_free_count = flat_row
        .candidate_profiles
        .iter()
        .filter(|candidate| {
            candidate.cadence_runs.iter().any(|run| {
                run.cadence == CADENCE_LABELS[0] && run.rollout.source_handoff_contact_free
            })
        })
        .count();
    if launch.flat_gate.case_id != FLAT_CASE_ID
        || launch.flat_gate.selected_profile_count != flat_expected_profiles.len()
        || launch
            .flat_gate
            .source_handoff_contact_free_profile_count_120_hz
            != flat_contact_free_count
        || launch.flat_gate.passed != (flat_contact_free_count > 0)
    {
        bail!("frozen launch flat-gate summary does not match its candidate rows");
    }
    let expected_uncut_ids = vec![
        "continuous_uphill_r+30".to_owned(),
        "continuous_downhill_r-30".to_owned(),
    ];
    let uncut_contact_free_count = launch
        .cases
        .iter()
        .filter(|case| expected_uncut_ids.contains(&case.input.id))
        .flat_map(|case| &case.candidate_profiles)
        .filter(|candidate| {
            candidate.cadence_runs.iter().any(|run| {
                run.cadence == CADENCE_LABELS[0] && run.rollout.source_handoff_contact_free
            })
        })
        .count();
    let expected_uncut_evaluated =
        launch.execution_stage == "conditional_full" && launch.flat_gate.passed;
    if launch.uncut_gate.case_ids != expected_uncut_ids
        || launch.uncut_gate.evaluated != expected_uncut_evaluated
        || launch
            .uncut_gate
            .source_handoff_contact_free_profile_count_120_hz
            != uncut_contact_free_count
        || launch.uncut_gate.passed != (expected_uncut_evaluated && uncut_contact_free_count > 0)
    {
        bail!("frozen launch uncut-gate summary does not match its candidate rows");
    }

    for (index, ((prepared_case, launch_case), gate_case)) in prepared
        .cases
        .iter()
        .zip(&launch.cases)
        .zip(&launch_gate.cases)
        .enumerate()
    {
        if prepared_case.evidence != launch_case.input
            || gate_case.input != prepared_case.evidence
            || launch_case.selection_roles != gate_case.selection_roles
        {
            bail!(
                "launch case {} no longer matches rebuilt case and role bindings",
                prepared_case.evidence.id
            );
        }
        let should_run = if index == flat_index {
            true
        } else if index == 1 || index == 2 {
            launch.uncut_gate.evaluated
        } else {
            launch.uncut_gate.passed
        };
        if should_run {
            if launch_case.candidate_profiles.len() != prepared_case.profile_candidates.len() {
                bail!(
                    "launch case {} candidate inventory count changed",
                    prepared_case.evidence.id
                );
            }
            let expected_status = if index == flat_index {
                "ran_flat_gate"
            } else if index == 1 || index == 2 {
                "ran_uncut_gate"
            } else {
                "ran_conditional_obstacle"
            };
            if launch_case.status != expected_status || launch_case.skip_reason.is_some() {
                bail!(
                    "launch case {} has inconsistent run/skip status",
                    prepared_case.evidence.id
                );
            }
        } else if !launch_case.candidate_profiles.is_empty() || launch_case.status != "not_run" {
            bail!(
                "launch case {} contains profiles beyond its flat-first stop gate",
                prepared_case.evidence.id
            );
        }
        if gate_case.profiles.len() != prepared_case.profile_candidates.len() {
            bail!(
                "launch gate case {} profile inventory changed",
                prepared_case.evidence.id
            );
        }
        for ((selected, binding), candidate) in prepared_case
            .profile_candidates
            .iter()
            .zip(&gate_case.profiles)
            .zip(&launch_case.candidate_profiles)
        {
            if binding.basis_candidate_identity != selected.candidate_identity
                || binding.selected_roles != selected.selected_roles
                || candidate.basis_candidate_identity != selected.candidate_identity
                || candidate.selected_roles != selected.selected_roles
                || candidate.original_source_handoff_arc_step
                    != binding.original_source_handoff_arc_step
                || candidate.original_source_bridge_tick_count
                    != binding.original_source_bridge_tick_count
                || candidate.coast_tick_count != binding.coast_tick_count
                || candidate.terminal_bridge_tick_count != binding.terminal_bridge_tick_count
                || candidate.frozen_first_powered_attitude_rad
                    != binding.frozen_first_powered_attitude_rad
                || launch_candidate_identity(candidate)? != candidate.identity
                || candidate
                    .cadence_runs
                    .iter()
                    .map(|run| run.cadence.as_str())
                    .ne(CADENCE_LABELS)
            {
                bail!(
                    "launch profile {} role, phase, or cadence binding changed",
                    selected.candidate_identity
                );
            }
        }
        if should_run && launch_case.candidate_profiles.len() != gate_case.profiles.len() {
            bail!(
                "launch case {} selected profile-to-candidate mapping changed",
                prepared_case.evidence.id
            );
        }
    }
    Ok(())
}

pub(super) fn build_contact_contract_input_gate(
    prepared: &PreparedInputs,
    nominal: &WaypointDirectNominalPlantArtifact,
    launch: &WaypointDirectLaunchFeasibilityArtifact,
    launch_gate: &LaunchFeasibilityInputGateEvidence,
) -> Result<LaunchContactContractInputGateEvidence> {
    let mut existing_cadence_row_count = 0_u32;
    let mut selected_role_count = 0_u32;
    let cases = prepared
        .cases
        .iter()
        .zip(&nominal.cases)
        .zip(&launch.cases)
        .zip(&launch_gate.cases)
        .map(
            |(((prepared_case, nominal_case), launch_case), gate_case)| {
                if nominal_case.input != prepared_case.evidence
                    || launch_case.input != prepared_case.evidence
                    || gate_case.input != prepared_case.evidence
                {
                    bail!(
                        "contact-contract gate case {} identity mismatch",
                        prepared_case.evidence.id
                    );
                }
                if nominal_case.selection_roles != launch_case.selection_roles
                    || nominal_case.selection_roles != gate_case.selection_roles
                {
                    bail!(
                        "contact-contract gate case {} role binding mismatch",
                        prepared_case.evidence.id
                    );
                }
                selected_role_count += nominal_case.selection_roles.len() as u32;
                existing_cadence_row_count += launch_case
                    .candidate_profiles
                    .iter()
                    .map(|candidate| candidate.cadence_runs.len() as u32)
                    .sum::<u32>();
                Ok(LaunchContactContractInputCaseEvidence {
                    input: prepared_case.evidence.clone(),
                    selection_roles: nominal_case.selection_roles.clone(),
                    profile_bindings: gate_case.profiles.clone(),
                    available_candidate_identities: launch_case
                        .candidate_profiles
                        .iter()
                        .map(|candidate| candidate.basis_candidate_identity.clone())
                        .collect(),
                    available_cadence_labels: launch_case
                        .candidate_profiles
                        .iter()
                        .flat_map(|candidate| {
                            candidate.cadence_runs.iter().map(|run| run.cadence.clone())
                        })
                        .collect(),
                    launch_case_status: launch_case.status.clone(),
                    launch_skip_reason: launch_case.skip_reason.clone(),
                })
            },
        )
        .collect::<Result<Vec<_>>>()?;
    let mut gate = LaunchContactContractInputGateEvidence {
        schema_id: "waypoint_direct_launch_contact_contract_input_gate_v1".to_owned(),
        schema_version: 1,
        baseline_identity: prepared.baseline.identity.clone(),
        sweep_identity: prepared.sweep.identity.clone(),
        nominal_plant_identity: nominal.identity.clone(),
        nominal_input_manifest_identity: nominal.input_manifest_identity.clone(),
        launch_feasibility_identity: launch.identity.clone(),
        launch_feasibility_input_gate_identity: launch.input_gate_identity.clone(),
        launch_input_gate: launch_gate.clone(),
        cases,
        case_count: prepared.cases.len() as u32,
        unique_profile_count: prepared
            .cases
            .iter()
            .map(|case| case.profile_candidates.len() as u32)
            .sum(),
        selected_role_count,
        existing_cadence_row_count,
        identity: String::new(),
    };
    if gate.case_count != 6 || gate.unique_profile_count != 9 || gate.selected_role_count != 12 {
        bail!("contact-contract input gate requires six cases, nine profiles, and twelve roles");
    }
    gate.identity = input_gate_identity(&gate)?;
    Ok(gate)
}

fn input_gate_identity(gate: &LaunchContactContractInputGateEvidence) -> Result<String> {
    let mut input = gate.clone();
    input.identity.clear();
    stable_digest(&input)
}

pub(super) fn artifact_identity(
    artifact: &WaypointDirectLaunchContactContractArtifact,
) -> Result<String> {
    let mut input = artifact.clone();
    input.identity.clear();
    stable_digest(&input)
}

fn launch_candidate_identity(
    candidate: &super::LaunchFeasibilityCandidateEvidence,
) -> Result<String> {
    let mut input = candidate.clone();
    input.identity.clear();
    stable_digest(&input)
}

fn validate_pinned_launch_identity(launch: &WaypointDirectLaunchFeasibilityArtifact) -> Result<()> {
    let computed = super::launch_feasibility::artifact_identity(launch)?;
    validate_pinned_launch_identity_values(&launch.identity, &computed)
}

fn validate_pinned_launch_identity_values(stored: &str, computed: &str) -> Result<()> {
    if computed != stored || stored != EXPECTED_LAUNCH_IDENTITY {
        bail!("launch-feasibility semantic identity is not the frozen contact-contract input");
    }
    Ok(())
}

fn contact_contract_protocol() -> LaunchContactContractProtocolEvidence {
    LaunchContactContractProtocolEvidence {
        replay_source: "for each candidate/cadence row already materialized in the exact frozen launch-feasibility artifact, concatenate launch.samples and rollout.per_step; no new candidates or cadence rows are generated".to_owned(),
        paired_step_rule: "start two identical states from the rebuilt frozen scenario; replay only logged held commands; advance one with ordinary SimulationState::step and the other with step_physics_and_classify_contact; record the neutral state before mission terminal handling, then compare classifier, event, state, and sealed tick log".to_owned(),
        cadence_rule: "retain the two frozen launch rows: 120 Hz per-step updates and 60 Hz updates held for two physics steps; reject gaps, overlaps, command changes on held ticks, or incomplete tick accounting".to_owned(),
        predicate_mirror_rule: "evaluator-local reproduction of pd-core landing_snapshot and detect_contact_classification; compare on every tick against the neutral enum, ordinary event, and frozen launch tick label without modifying contact thresholds".to_owned(),
        orientation_rule: "synthetic positive/negative 0.1 rad probes compare both core and V2 footprints independently on centered flat terrain, centered h(x)=0.1x slope, and flat-pad edge geometry at center x=1.75 m; all use the frozen 4 m foot half-span and 5 m base offset and are not mission evidence".to_owned(),
        flat_first_rule: "audit flat native then flat research-shortest at both cadences; only if all four rows have first contact and exact replay/classifier/event/log parity may remaining profiles materialized by the launch artifact be audited; otherwise retain them as skipped".to_owned(),
    }
}

fn flat_profiles_in_role_order(
    case: &super::PreparedCase,
) -> Vec<&super::PreparedProfileCandidate> {
    let mut profiles = Vec::new();
    for identity in [
        case.evidence.native_v2_candidate_identity.as_str(),
        case.evidence
            .research_shortest_certified_candidate_identity
            .as_str(),
    ] {
        if let Some(profile) = case
            .profile_candidates
            .iter()
            .find(|profile| profile.candidate_identity == identity)
            && !profiles
                .iter()
                .any(|seen: &&super::PreparedProfileCandidate| seen.candidate_identity == identity)
        {
            profiles.push(profile);
        }
    }
    profiles
}

fn flat_role_bindings_valid(
    case: &super::PreparedCase,
    profiles: &[&super::PreparedProfileCandidate],
) -> bool {
    let native_identity = case.evidence.native_v2_candidate_identity.as_str();
    let shortest_identity = case
        .evidence
        .research_shortest_certified_candidate_identity
        .as_str();
    if native_identity == shortest_identity || profiles.len() != 2 {
        return false;
    }
    profiles[0].candidate_identity == native_identity
        && profiles[0].selected_roles.as_slice() == ["v2_native_selected"]
        && profiles[1].candidate_identity == shortest_identity
        && profiles[1].selected_roles.as_slice() == ["research_shortest_certified"]
}

fn flat_rows_cover_required_bindings(
    profiles: &[&super::PreparedProfileCandidate],
    rows: &[LaunchContactContractCadenceEvidence],
) -> bool {
    profiles.len() == 2
        && rows.len() == 2 * CADENCE_LABELS.len()
        && profiles.iter().all(|profile| {
            CADENCE_LABELS.iter().all(|cadence| {
                rows.iter()
                    .filter(|row| {
                        row.candidate_identity == profile.candidate_identity
                            && row.cadence == *cadence
                    })
                    .count()
                    == 1
            })
        })
}

fn flat_gate_satisfied(gate: &FlatContactContractGateEvidence) -> bool {
    gate.audited_candidate_count == 2
        && gate.audited_cadence_row_count == 2 * CADENCE_LABELS.len()
        && gate.required_role_bindings_valid
        && gate.all_required_cadence_rows_present
        && gate.all_first_contacts_observed
        && gate.all_replay_and_classifier_parity_passed
        && gate.native_stable_landing_observed
        && gate.research_shortest_crash_observed
}

fn audit_or_skip_cadence(
    case: &super::PreparedCase,
    selected: &super::PreparedProfileCandidate,
    cadence: &str,
    run: Option<&LaunchFeasibilityCadenceRunEvidence>,
    forced_skip_reason: Option<&str>,
) -> Result<LaunchContactContractCadenceEvidence> {
    let base = |status: &str, skip_reason: Option<String>| LaunchContactContractCadenceEvidence {
        candidate_identity: selected.candidate_identity.clone(),
        cadence: cadence.to_owned(),
        status: status.to_owned(),
        skip_reason,
        trace: None,
        first_contact: None,
        profile_end: None,
        post_terminal_state: None,
        source_handoff_position_error_m: None,
        source_handoff_velocity_error_mps: None,
        reseeded_bridge_analytically_eligible: None,
        reseeded_bridge_source_clearance: None,
        reseeded_bridge_source_attitude_margin: None,
        reseeded_bridge_launch_boundary: None,
        reseeded_bridge_aggregate_fuel_margin: None,
        reseeded_bridge_aggregate_time_margin: None,
    };
    if let Some(reason) = forced_skip_reason {
        return Ok(base("skipped_flat_gate_failed", Some(reason.to_owned())));
    }
    let Some(run) = run else {
        return Ok(base(
            "not_materialized_by_launch_gate",
            Some("cadence_row_not_materialized_by_launch_gate".to_owned()),
        ));
    };
    if run.cadence != cadence {
        bail!("contact-contract cadence row identity does not match requested cadence");
    }
    let context = RunContext::from_scenario(&case.scenario).map_err(anyhow::Error::msg)?;
    let ReplayTraceResult {
        trace,
        first_contact,
        profile_end_observed,
        post_terminal_state,
        ..
    } = replay_logged_cadence(&context, run)?;
    let mut evidence = base(
        if trace.passed {
            "audited"
        } else {
            "replay_parity_failed"
        },
        (!trace.passed).then(|| "per_tick_replay_or_contact_classifier_parity_failed".to_owned()),
    );
    evidence.trace = Some(trace);
    evidence.first_contact = first_contact;
    evidence.profile_end = run.rollout.profile_end.clone();
    evidence.post_terminal_state = post_terminal_state;
    evidence.source_handoff_position_error_m = run.rollout.source_handoff_position_error_m;
    evidence.source_handoff_velocity_error_mps = run.rollout.source_handoff_velocity_error_mps;
    if let Some(profile_end) = &run.rollout.profile_end {
        if profile_end_observed.as_ref() != Some(profile_end) {
            if let Some(trace) = &mut evidence.trace {
                trace.terminal_outcome_parity = false;
                trace.passed = false;
            }
            evidence.status = "replay_parity_failed".to_owned();
            evidence.skip_reason = Some("profile_end_state_did_not_match_frozen_log".to_owned());
        }
    } else if profile_end_observed.is_some() {
        if let Some(trace) = &mut evidence.trace {
            trace.terminal_outcome_parity = false;
            trace.passed = false;
        }
        evidence.status = "replay_parity_failed".to_owned();
        evidence.skip_reason = Some("unexpected_profile_end_state_in_replay".to_owned());
    }
    if let Some(bridge) = &run.reseeded_bridge {
        evidence.reseeded_bridge_analytically_eligible = Some(bridge.analytically_eligible);
        evidence.reseeded_bridge_source_clearance = bridge.source_clearance.clone();
        evidence.reseeded_bridge_source_attitude_margin = bridge.source_attitude_margin;
        evidence.reseeded_bridge_launch_boundary = bridge.launch_boundary.clone();
        evidence.reseeded_bridge_aggregate_fuel_margin = bridge.aggregate_fuel_margin;
        evidence.reseeded_bridge_aggregate_time_margin = bridge.aggregate_time_margin;
    }
    Ok(evidence)
}

#[derive(Clone, Debug)]
struct RecordedReplayTick {
    physics_step: u64,
    phase: String,
    expected_contact: String,
    desired_target_attitude_rad: f64,
    held_target_attitude_rad: f64,
    attitude_before_step_rad: f64,
    attitude_after_step_rad: f64,
    angular_rate_radps: f64,
    commanded_throttle_frac: f64,
    applied_throttle_frac: Option<f64>,
    position_m: Option<Vec2>,
    velocity_mps: Option<Vec2>,
    touchdown_clearance_m: f64,
    hull_clearance_m: f64,
    fuel_kg: f64,
    sim_time_s: f64,
}

pub(super) struct ReplayTraceResult {
    pub(super) trace: ReplayTraceParityEvidence,
    pub(super) first_contact: Option<FirstContactEvidence>,
    pub(super) first_contact_margins: Option<FirstContactPredicateMarginsEvidence>,
    pub(super) profile_end_observed: Option<PlantStateEvidence>,
    pub(super) post_terminal_state: Option<PlantStateEvidence>,
    pub(super) sign_geometry_scan: TrajectorySignGeometryScanEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct FirstContactPredicateMarginsEvidence {
    pub no_contact_minimum_foot_clearance_m: f64,
    pub no_contact_minimum_hull_clearance_m: f64,
    pub stable_minimum_clearance_margin_m: f64,
    pub stable_maximum_clearance_margin_m: f64,
    pub stable_hull_penetration_margin_m: f64,
    pub safe_normal_speed_margin_mps: f64,
    pub safe_tangential_speed_margin_mps: f64,
    pub safe_attitude_margin_rad: f64,
    pub safe_angular_rate_margin_radps: f64,
    pub touchdown_pad_left_margin_m: f64,
    pub touchdown_pad_right_margin_m: f64,
}

pub(super) fn replay_logged_cadence(
    context: &RunContext,
    run: &LaunchFeasibilityCadenceRunEvidence,
) -> Result<ReplayTraceResult> {
    replay_logged_cadence_with_hold_mode(context, run, SourceDurationHoldMode::Together)
}

pub(super) fn replay_logged_mixed_cadence(
    context: &RunContext,
    run: &LaunchFeasibilityCadenceRunEvidence,
    hold_mode: SourceDurationHoldMode,
) -> Result<ReplayTraceResult> {
    if run.cadence != CADENCE_LABELS[1] || hold_mode == SourceDurationHoldMode::Together {
        bail!("mixed source-duration replay requires a mixed mode at held 60 Hz cadence");
    }
    replay_logged_cadence_with_hold_mode(context, run, hold_mode)
}

fn replay_logged_cadence_with_hold_mode(
    context: &RunContext,
    run: &LaunchFeasibilityCadenceRunEvidence,
    hold_mode: SourceDurationHoldMode,
) -> Result<ReplayTraceResult> {
    let mut recorded = Vec::with_capacity(run.launch.samples.len() + run.rollout.per_step.len());
    recorded.extend(run.launch.samples.iter().map(recorded_launch_tick));
    recorded.extend(run.rollout.per_step.iter().map(recorded_rollout_tick));

    let mut ordinary = SimulationState::new(context)?;
    let mut neutral = SimulationState::new(context)?;
    let mut command_schedule_parity = true;
    let mut preterminal_state_parity = true;
    let mut local_mirror_parity = true;
    let mut neutral_to_event_parity = true;
    let mut event_to_launch_log_parity = true;
    let mut first_mismatch = None;
    let mut first_contact = None;
    let mut first_contact_margins = None;
    let mut profile_end_observed = None;
    let mut sign_geometry_scan = TrajectorySignGeometryScanEvidence {
        physics_ticks_sampled: 0,
        first_physics_step: None,
        last_physics_step: None,
        original_terminal_physics_step: None,
        reached_original_terminal: false,
        core_first_geometric_gate_step: None,
        opposite_sign_first_geometric_gate_step: None,
        geometric_gate_order: String::new(),
        geometry_gate_disagreement_tick_count: 0,
        pad_membership_disagreement_tick_count: 0,
        stable_predicate_disagreement_tick_count: 0,
        earliest_geometry_gate_disagreement: None,
        earliest_pad_membership_disagreement: None,
        earliest_stable_predicate_disagreement: None,
        scope: "opposite-sign footprint geometry is evaluated on each original replay state through its original terminal; command replay, dynamics, and core contact outcomes remain unchanged; no post-terminal alternate outcome is modeled".to_owned(),
    };
    let interval = context.sim.control_interval_steps();
    let cadence_is_direct = run.cadence == CADENCE_LABELS[0];
    if !cadence_is_direct && run.cadence != CADENCE_LABELS[1] {
        bail!("unsupported frozen launch cadence {}", run.cadence);
    }
    let mut contiguous_from_step_one = true;

    for (tick_index, tick) in recorded.iter().enumerate() {
        if ordinary.is_terminal() || neutral.is_terminal() {
            contiguous_from_step_one = false;
            record_mismatch(
                &mut first_mismatch,
                tick,
                "trace_after_terminal",
                "no further recorded tick".to_owned(),
                "recorded tick after terminal state".to_owned(),
            );
            break;
        }
        if tick.physics_step != ordinary.physics_step + 1
            || tick.physics_step != neutral.physics_step + 1
        {
            contiguous_from_step_one = false;
            record_mismatch(
                &mut first_mismatch,
                tick,
                "physics_step",
                (ordinary.physics_step + 1).to_string(),
                tick.physics_step.to_string(),
            );
            break;
        }
        let cadence = if cadence_is_direct {
            RolloutCadence::DirectPerTick
        } else {
            RolloutCadence::ControllerCadence
        };
        let (throttle_update_due, attitude_update_due) =
            component_update_schedule(hold_mode, cadence, ordinary.physics_step, interval);
        if throttle_update_due || attitude_update_due {
            let ordinary_held = ordinary.held_command;
            let neutral_held = neutral.held_command;
            ordinary.set_command(Command {
                throttle_frac: if throttle_update_due {
                    tick.commanded_throttle_frac
                } else {
                    ordinary_held.throttle_frac
                },
                target_attitude_rad: if attitude_update_due {
                    tick.desired_target_attitude_rad
                } else {
                    ordinary_held.target_attitude_rad
                },
            });
            neutral.set_command(Command {
                throttle_frac: if throttle_update_due {
                    tick.commanded_throttle_frac
                } else {
                    neutral_held.throttle_frac
                },
                target_attitude_rad: if attitude_update_due {
                    tick.desired_target_attitude_rad
                } else {
                    neutral_held.target_attitude_rad
                },
            });
        }
        if !same_f64(
            ordinary.held_command.throttle_frac,
            tick.commanded_throttle_frac,
        ) || !same_f64(
            neutral.held_command.throttle_frac,
            tick.commanded_throttle_frac,
        ) || !same_f64(
            ordinary.held_command.target_attitude_rad,
            tick.held_target_attitude_rad,
        ) || !same_f64(
            neutral.held_command.target_attitude_rad,
            tick.held_target_attitude_rad,
        ) {
            command_schedule_parity = false;
            record_mismatch(
                &mut first_mismatch,
                tick,
                "held_command_component_schedule",
                format!(
                    "{} @ {}",
                    tick.commanded_throttle_frac, tick.held_target_attitude_rad
                ),
                format!(
                    "{} @ {}",
                    ordinary.held_command.throttle_frac, ordinary.held_command.target_attitude_rad
                ),
            );
        }
        if !same_f64(ordinary.attitude_rad, tick.attitude_before_step_rad)
            || !same_f64(neutral.attitude_rad, tick.attitude_before_step_rad)
        {
            preterminal_state_parity = false;
            record_mismatch(
                &mut first_mismatch,
                tick,
                "attitude_before_step_rad",
                tick.attitude_before_step_rad.to_string(),
                ordinary.attitude_rad.to_string(),
            );
        }

        let pre_step_fuel_kg = ordinary.fuel_kg;
        let actual_applied_throttle_frac = super::plant_applied_throttle(
            ordinary.held_command,
            context.vehicle.min_throttle_frac,
            pre_step_fuel_kg,
        );
        let neutral_contact = neutral.step_physics_and_classify_contact(context);
        let neutral_observation = neutral.build_observation(context);
        let predicates = mirror_contact_predicates(&neutral, context);
        record_sign_geometry_sample(
            &mut sign_geometry_scan,
            &neutral,
            context,
            tick,
            &predicates,
        );
        let mirror_label = predicates.predicted_classification.as_str();
        let neutral_label = contact_classification_label(&neutral_contact);
        let mirror_matches_neutral = mirror_label == neutral_label;
        if !mirror_matches_neutral {
            local_mirror_parity = false;
            record_mismatch(
                &mut first_mismatch,
                tick,
                "local_predicted_classification",
                neutral_label.to_owned(),
                mirror_label.to_owned(),
            );
        }

        let events = ordinary.step(context);
        let ordinary_label = event_contact_label(&events);
        let event_matches_neutral = ordinary_label == neutral_label;
        if !event_matches_neutral {
            neutral_to_event_parity = false;
            record_mismatch(
                &mut first_mismatch,
                tick,
                "ordinary_contact_event",
                neutral_label.to_owned(),
                ordinary_label.to_owned(),
            );
        }
        let event_matches_log = ordinary_label == tick.expected_contact;
        if !event_matches_log {
            event_to_launch_log_parity = false;
            record_mismatch(
                &mut first_mismatch,
                tick,
                "sealed_contact_classification",
                tick.expected_contact.clone(),
                ordinary_label.to_owned(),
            );
        }
        if tick.physics_step != neutral.physics_step
            || tick.physics_step != ordinary.physics_step
            || !same_f64(neutral.sim_time_s, ordinary.sim_time_s)
            || neutral.position_m != ordinary.position_m
            || !same_f64(neutral.attitude_rad, ordinary.attitude_rad)
            || !same_f64(neutral.fuel_kg, ordinary.fuel_kg)
            || neutral.min_touchdown_clearance_m != ordinary.min_touchdown_clearance_m
            || neutral.min_hull_clearance_m != ordinary.min_hull_clearance_m
        {
            preterminal_state_parity = false;
            record_mismatch(
                &mut first_mismatch,
                tick,
                "paired_post_physics_state",
                "identical position, attitude, fuel, time, and clearance extrema".to_owned(),
                "paired ordinary/neutral states differ".to_owned(),
            );
        }
        match &neutral_contact {
            ContactClassification::StableTouchdown { .. } => {
                if ordinary.velocity_mps != Vec2::new(0.0, 0.0)
                    || ordinary.angular_rate_radps != 0.0
                {
                    preterminal_state_parity = false;
                    record_mismatch(
                        &mut first_mismatch,
                        tick,
                        "ordinary_stable_terminal_effect",
                        "velocity=0, angular_rate=0".to_owned(),
                        format!(
                            "velocity={:?}, angular_rate={}",
                            ordinary.velocity_mps, ordinary.angular_rate_radps
                        ),
                    );
                }
            }
            ContactClassification::None | ContactClassification::Crash => {
                if neutral.velocity_mps != ordinary.velocity_mps
                    || !same_f64(neutral.angular_rate_radps, ordinary.angular_rate_radps)
                {
                    preterminal_state_parity = false;
                    record_mismatch(
                        &mut first_mismatch,
                        tick,
                        "ordinary_non_landing_terminal_effect",
                        format!(
                            "velocity={:?}, angular_rate={}",
                            neutral.velocity_mps, neutral.angular_rate_radps
                        ),
                        format!(
                            "velocity={:?}, angular_rate={}",
                            ordinary.velocity_mps, ordinary.angular_rate_radps
                        ),
                    );
                }
            }
        }
        if !tick_matches_recorded_state(
            tick,
            &ordinary,
            &neutral_observation,
            neutral_contact_label_is_stable(&neutral_contact),
            actual_applied_throttle_frac,
        ) {
            event_to_launch_log_parity = false;
            record_mismatch(
                &mut first_mismatch,
                tick,
                "recorded_post_step_state",
                "frozen launch tick state and clearance fields".to_owned(),
                "replayed state differs from frozen tick log".to_owned(),
            );
        }
        if run
            .rollout
            .profile_end
            .as_ref()
            .is_some_and(|profile_end| profile_end.physics_step == tick.physics_step)
        {
            profile_end_observed = Some(state_evidence(&ordinary, context.vehicle.initial_fuel_kg));
        }
        if !matches!(neutral_contact, ContactClassification::None) && first_contact.is_none() {
            let v2_sign_geometry_counterfactual =
                v2_sign_geometry_counterfactual(&neutral, context, &predicates);
            let contact_state = preterminal_contact_state(&neutral, context);
            first_contact_margins = Some(first_contact_predicate_margins(&contact_state, context));
            first_contact = Some(FirstContactEvidence {
                physics_step: tick.physics_step,
                classification: neutral_label.to_owned(),
                state: contact_state,
                predicates,
                v2_sign_geometry_counterfactual,
                neutral_matches_local_mirror: mirror_matches_neutral,
                ordinary_event_matches_neutral: event_matches_neutral,
                launch_log_matches_event: event_matches_log,
            });
        }
        if tick_index == run.launch.samples.len().saturating_sub(1)
            && run.launch.completed
            && run.launch.end_state.as_ref()
                != Some(&state_evidence(&ordinary, context.vehicle.initial_fuel_kg))
        {
            event_to_launch_log_parity = false;
            record_mismatch(
                &mut first_mismatch,
                tick,
                "launch_end_state",
                "frozen launch.end_state".to_owned(),
                "replayed 72-step launch state differs".to_owned(),
            );
        }
    }

    let post_terminal_state = ordinary
        .is_terminal()
        .then(|| state_evidence(&ordinary, context.vehicle.initial_fuel_kg));
    sign_geometry_scan.reached_original_terminal =
        ordinary.is_terminal() && ordinary.physics_step == run.rollout.physics_steps_advanced;
    sign_geometry_scan.original_terminal_physics_step = sign_geometry_scan
        .reached_original_terminal
        .then_some(ordinary.physics_step);
    sign_geometry_scan.geometric_gate_order = match (
        sign_geometry_scan.core_first_geometric_gate_step,
        sign_geometry_scan.opposite_sign_first_geometric_gate_step,
    ) {
        (Some(core), Some(opposite)) if core < opposite => {
            "core_geometry_reaches_gate_first".to_owned()
        }
        (Some(core), Some(opposite)) if opposite < core => {
            "opposite_sign_geometry_reaches_gate_first".to_owned()
        }
        (Some(_), Some(_)) => "both_reach_gate_on_same_sample".to_owned(),
        (Some(_), None) => "only_core_geometry_reaches_gate_before_original_terminal".to_owned(),
        (None, Some(_)) => {
            "only_opposite_sign_geometry_reaches_gate_before_original_terminal".to_owned()
        }
        (None, None) => "neither_geometry_reaches_gate_before_original_terminal".to_owned(),
    };
    let terminal_outcome_parity = state_evidence(&ordinary, context.vehicle.initial_fuel_kg)
        == run.rollout.termination
        && ordinary.physics_step == run.rollout.physics_steps_advanced
        && run.launch.samples.len() as u64 == run.launch.physics_ticks_completed;
    if !terminal_outcome_parity {
        let fake_tick = recorded.last().cloned().unwrap_or_else(empty_recorded_tick);
        record_mismatch(
            &mut first_mismatch,
            &fake_tick,
            "ordinary_terminal_outcome",
            format!("frozen termination {:?}", run.rollout.termination),
            format!(
                "replayed {:?}",
                state_evidence(&ordinary, context.vehicle.initial_fuel_kg)
            ),
        );
    }
    let all_tick_parity = command_schedule_parity
        && preterminal_state_parity
        && local_mirror_parity
        && neutral_to_event_parity
        && event_to_launch_log_parity
        && contiguous_from_step_one
        && terminal_outcome_parity;
    Ok(ReplayTraceResult {
        trace: ReplayTraceParityEvidence {
            physics_ticks_replayed: ordinary.physics_step,
            first_physics_step: recorded.first().map(|tick| tick.physics_step),
            last_physics_step: recorded.last().map(|tick| tick.physics_step),
            contiguous_from_step_one,
            command_schedule_parity,
            preterminal_state_parity,
            local_mirror_parity,
            neutral_to_event_parity,
            event_to_launch_log_parity,
            terminal_outcome_parity,
            first_mismatch,
            passed: all_tick_parity,
        },
        first_contact,
        first_contact_margins,
        profile_end_observed,
        post_terminal_state,
        sign_geometry_scan,
    })
}

fn first_contact_predicate_margins(
    state: &PreterminalContactStateEvidence,
    context: &RunContext,
) -> FirstContactPredicateMarginsEvidence {
    let minimum_foot_clearance = state
        .touchdown_feet
        .iter()
        .map(|foot| foot.signed_clearance_m)
        .fold(f64::INFINITY, f64::min);
    let maximum_foot_clearance = state
        .touchdown_feet
        .iter()
        .map(|foot| foot.signed_clearance_m)
        .fold(f64::NEG_INFINITY, f64::max);
    let minimum_foot_x = state
        .touchdown_feet
        .iter()
        .map(|foot| foot.position_m.x)
        .fold(f64::INFINITY, f64::min);
    let maximum_foot_x = state
        .touchdown_feet
        .iter()
        .map(|foot| foot.position_m.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let pad_left = context.target_pad.center_x_m - context.target_pad.half_width_m();
    let pad_right = context.target_pad.center_x_m + context.target_pad.half_width_m();
    FirstContactPredicateMarginsEvidence {
        no_contact_minimum_foot_clearance_m: minimum_foot_clearance,
        no_contact_minimum_hull_clearance_m: state.minimum_hull_clearance_m,
        stable_minimum_clearance_margin_m: 0.05 - minimum_foot_clearance,
        stable_maximum_clearance_margin_m: 0.15 - maximum_foot_clearance,
        stable_hull_penetration_margin_m: state.minimum_hull_clearance_m
            + state.dynamic_hull_penetration_allowance_m,
        safe_normal_speed_margin_mps: context.vehicle.safe_touchdown_normal_speed_mps
            - state.normal_closing_speed_mps,
        safe_tangential_speed_margin_mps: context.vehicle.safe_touchdown_tangential_speed_mps
            - state.tangential_closing_speed_mps,
        safe_attitude_margin_rad: context.vehicle.safe_touchdown_attitude_error_rad
            - state.attitude_error_rad,
        safe_angular_rate_margin_radps: context.vehicle.safe_touchdown_angular_rate_radps
            - state.absolute_angular_rate_radps,
        touchdown_pad_left_margin_m: minimum_foot_x - pad_left,
        touchdown_pad_right_margin_m: pad_right - maximum_foot_x,
    }
}

fn record_sign_geometry_sample(
    scan: &mut TrajectorySignGeometryScanEvidence,
    state: &SimulationState,
    context: &RunContext,
    tick: &RecordedReplayTick,
    core: &ContactPredicateMirrorEvidence,
) {
    let opposite = v2_sign_geometry_counterfactual(state, context, core);
    scan.physics_ticks_sampled += 1;
    scan.first_physics_step.get_or_insert(tick.physics_step);
    scan.last_physics_step = Some(tick.physics_step);
    if !core.no_contact_clearance_gate {
        scan.core_first_geometric_gate_step
            .get_or_insert(tick.physics_step);
    }
    if !opposite.no_contact_clearance_gate {
        scan.opposite_sign_first_geometric_gate_step
            .get_or_insert(tick.physics_step);
    }
    if opposite.no_contact_clearance_gate_differs_from_core {
        scan.geometry_gate_disagreement_tick_count += 1;
        scan.earliest_geometry_gate_disagreement
            .get_or_insert_with(|| TrajectorySignDisagreementEvidence {
                physics_step: tick.physics_step,
                phase: tick.phase.clone(),
                original_contact_classification: tick.expected_contact.clone(),
                core_value: core.no_contact_clearance_gate,
                opposite_sign_value: opposite.no_contact_clearance_gate,
                differing_predicates: vec!["no_contact_clearance_gate".to_owned()],
            });
    }
    if opposite.pad_membership_differs_from_core {
        scan.pad_membership_disagreement_tick_count += 1;
        let core_pad_contains_both_feet =
            preterminal_contact_state(state, context).touchdown_pad_contains_both_feet;
        scan.earliest_pad_membership_disagreement
            .get_or_insert_with(|| TrajectorySignDisagreementEvidence {
                physics_step: tick.physics_step,
                phase: tick.phase.clone(),
                original_contact_classification: tick.expected_contact.clone(),
                core_value: core_pad_contains_both_feet,
                opposite_sign_value: opposite.touchdown_pad_contains_both_feet,
                differing_predicates: vec!["touchdown_pad_contains_both_feet".to_owned()],
            });
    }
    if opposite.stable_geometry_predicates_differ_from_core {
        scan.stable_predicate_disagreement_tick_count += 1;
        let mut differing_predicates = Vec::new();
        if opposite.stable_minimum_clearance_predicate != core.stable_minimum_clearance_predicate {
            differing_predicates.push("stable_minimum_clearance".to_owned());
        }
        if opposite.stable_maximum_clearance_predicate != core.stable_maximum_clearance_predicate {
            differing_predicates.push("stable_maximum_clearance".to_owned());
        }
        if opposite.stable_hull_penetration_predicate != core.stable_hull_penetration_predicate {
            differing_predicates.push("stable_hull_penetration".to_owned());
        }
        if opposite.stable_touchdown_predicate != core.stable_touchdown_predicate {
            differing_predicates.push("stable_touchdown".to_owned());
        }
        scan.earliest_stable_predicate_disagreement
            .get_or_insert_with(|| TrajectorySignDisagreementEvidence {
                physics_step: tick.physics_step,
                phase: tick.phase.clone(),
                original_contact_classification: tick.expected_contact.clone(),
                core_value: core.stable_touchdown_predicate,
                opposite_sign_value: opposite.stable_touchdown_predicate,
                differing_predicates,
            });
    }
}

fn recorded_launch_tick(tick: &LaunchTickEvidence) -> RecordedReplayTick {
    RecordedReplayTick {
        physics_step: tick.physics_step,
        phase: tick.phase.clone(),
        expected_contact: tick.contact_classification.clone(),
        desired_target_attitude_rad: tick.commanded_target_attitude_rad,
        held_target_attitude_rad: tick.held_target_attitude_rad,
        attitude_before_step_rad: tick.attitude_before_step_rad,
        attitude_after_step_rad: tick.attitude_after_step_rad,
        angular_rate_radps: tick.angular_rate_radps,
        commanded_throttle_frac: tick.commanded_throttle_frac,
        applied_throttle_frac: None,
        position_m: Some(tick.position_m),
        velocity_mps: Some(tick.velocity_mps),
        touchdown_clearance_m: tick.touchdown_clearance_m,
        hull_clearance_m: tick.hull_clearance_m,
        fuel_kg: tick.fuel_kg,
        sim_time_s: tick.sim_time_s,
    }
}

fn recorded_rollout_tick(tick: &LaunchRolloutTickEvidence) -> RecordedReplayTick {
    RecordedReplayTick {
        physics_step: tick.physics_step,
        phase: tick.phase.clone(),
        expected_contact: tick.contact_classification.clone(),
        desired_target_attitude_rad: tick.desired_target_attitude_rad,
        held_target_attitude_rad: tick.held_target_attitude_rad,
        attitude_before_step_rad: tick.attitude_before_step_rad,
        attitude_after_step_rad: tick.attitude_after_step_rad,
        angular_rate_radps: tick.angular_rate_radps,
        commanded_throttle_frac: tick.commanded_throttle_frac,
        applied_throttle_frac: Some(tick.applied_throttle_frac),
        position_m: None,
        velocity_mps: None,
        touchdown_clearance_m: tick.touchdown_clearance_m,
        hull_clearance_m: tick.hull_clearance_m,
        fuel_kg: tick.fuel_kg,
        sim_time_s: tick.sim_time_s,
    }
}

fn tick_matches_recorded_state(
    tick: &RecordedReplayTick,
    ordinary: &SimulationState,
    observation: &pd_core::Observation,
    stable_terminal_effect: bool,
    actual_applied_throttle_frac: f64,
) -> bool {
    let velocity_matches = tick.velocity_mps.is_none_or(|expected| {
        if stable_terminal_effect {
            ordinary.velocity_mps == Vec2::new(0.0, 0.0) && expected == Vec2::new(0.0, 0.0)
        } else {
            ordinary.velocity_mps == expected
        }
    });
    same_f64(ordinary.attitude_rad, tick.attitude_after_step_rad)
        && same_f64(ordinary.angular_rate_radps, tick.angular_rate_radps)
        && same_f64(ordinary.sim_time_s, tick.sim_time_s)
        && same_f64(ordinary.fuel_kg, tick.fuel_kg)
        && observation.position_m == ordinary.position_m
        && observation.touchdown_clearance_m == tick.touchdown_clearance_m
        && observation.min_hull_clearance_m == tick.hull_clearance_m
        && tick
            .position_m
            .is_none_or(|expected| ordinary.position_m == expected)
        && velocity_matches
        && tick
            .applied_throttle_frac
            .is_none_or(|expected| same_f64(expected, actual_applied_throttle_frac))
}

fn same_f64(left: f64, right: f64) -> bool {
    left == right
}

fn neutral_contact_label_is_stable(contact: &ContactClassification) -> bool {
    matches!(contact, ContactClassification::StableTouchdown { .. })
}

fn record_mismatch(
    first: &mut Option<ReplayMismatchEvidence>,
    tick: &RecordedReplayTick,
    field: &str,
    expected: String,
    actual: String,
) {
    if first.is_none() {
        *first = Some(ReplayMismatchEvidence {
            physics_step: tick.physics_step,
            phase: tick.phase.clone(),
            field: field.to_owned(),
            expected,
            actual,
        });
    }
}

fn empty_recorded_tick() -> RecordedReplayTick {
    RecordedReplayTick {
        physics_step: 0,
        phase: "empty".to_owned(),
        expected_contact: "none".to_owned(),
        desired_target_attitude_rad: 0.0,
        held_target_attitude_rad: 0.0,
        attitude_before_step_rad: 0.0,
        attitude_after_step_rad: 0.0,
        angular_rate_radps: 0.0,
        commanded_throttle_frac: 0.0,
        applied_throttle_frac: None,
        position_m: None,
        velocity_mps: None,
        touchdown_clearance_m: 0.0,
        hull_clearance_m: 0.0,
        fuel_kg: 0.0,
        sim_time_s: 0.0,
    }
}

fn contact_classification_label(classification: &ContactClassification) -> &'static str {
    match classification {
        ContactClassification::None => "none",
        ContactClassification::StableTouchdown { on_target: true } => "stable_touchdown_on_target",
        ContactClassification::StableTouchdown { on_target: false } => {
            "stable_touchdown_off_target"
        }
        ContactClassification::Crash => "crash",
    }
}

fn event_contact_label(events: &[pd_core::EventRecord]) -> &'static str {
    for event in events {
        match event.kind {
            EventKind::TouchdownOnTarget => return "stable_touchdown_on_target",
            EventKind::TouchdownOffTarget => return "stable_touchdown_off_target",
            EventKind::Crash => return "crash",
            _ => {}
        }
    }
    "none"
}

fn mirror_contact_predicates(
    state: &SimulationState,
    context: &RunContext,
) -> ContactPredicateMirrorEvidence {
    let feet = touchdown_feet(state.position_m, state.attitude_rad, context);
    let minimum_touchdown_clearance_m = feet[0].signed_clearance_m.min(feet[1].signed_clearance_m);
    let maximum_touchdown_clearance_m = feet[0].signed_clearance_m.max(feet[1].signed_clearance_m);
    let minimum_hull_clearance_m = hull_vertices(state.position_m, state.attitude_rad, context)
        .into_iter()
        .map(|point| point.y - context.world.terrain.sample_height(point.x))
        .fold(f64::INFINITY, f64::min);
    let normal = context
        .world
        .terrain
        .sample_surface_normal(context.target_pad.center_x_m);
    let tangent = Vec2::new(normal.y, -normal.x);
    let normal_closing_speed_mps = (-dot(state.velocity_mps, normal)).max(0.0);
    let tangential_closing_speed_mps = dot(state.velocity_mps, tangent).abs();
    let vehicle_up = Vec2::new(state.attitude_rad.sin(), state.attitude_rad.cos());
    let attitude_error_rad = dot(vehicle_up, normal).clamp(-1.0, 1.0).acos();
    let absolute_angular_rate_radps = state.angular_rate_radps.abs();
    let hull_radius_m = (context.vehicle.geometry.hull_width_m * 0.5)
        .hypot(context.vehicle.geometry.hull_height_m * 0.5);
    let dynamic_hull_penetration_allowance_m = 0.012_f64.max(
        (normal_closing_speed_mps + absolute_angular_rate_radps * hull_radius_m)
            * context.sim.physics_dt_s(),
    );
    let pad_left_m = context.target_pad.center_x_m - context.target_pad.half_width_m();
    let pad_right_m = context.target_pad.center_x_m + context.target_pad.half_width_m();
    let touchdown_min_x = feet[0].position_m.x.min(feet[1].position_m.x);
    let touchdown_max_x = feet[0].position_m.x.max(feet[1].position_m.x);
    let on_target = touchdown_min_x >= pad_left_m && touchdown_max_x <= pad_right_m;
    let no_contact_clearance_gate =
        minimum_touchdown_clearance_m > 0.0 && minimum_hull_clearance_m > 0.0;
    let stable_minimum_clearance_predicate = minimum_touchdown_clearance_m <= 0.05;
    let stable_maximum_clearance_predicate = maximum_touchdown_clearance_m <= 0.15;
    let stable_hull_penetration_predicate =
        minimum_hull_clearance_m >= -dynamic_hull_penetration_allowance_m;
    let stable_touchdown_predicate = stable_minimum_clearance_predicate
        && stable_maximum_clearance_predicate
        && stable_hull_penetration_predicate;
    let safe_normal_speed_predicate =
        normal_closing_speed_mps <= context.vehicle.safe_touchdown_normal_speed_mps;
    let safe_tangential_speed_predicate =
        tangential_closing_speed_mps <= context.vehicle.safe_touchdown_tangential_speed_mps;
    let safe_attitude_predicate =
        attitude_error_rad <= context.vehicle.safe_touchdown_attitude_error_rad;
    let safe_angular_rate_predicate =
        absolute_angular_rate_radps <= context.vehicle.safe_touchdown_angular_rate_radps;
    let safe_touchdown_predicate = safe_normal_speed_predicate
        && safe_tangential_speed_predicate
        && safe_attitude_predicate
        && safe_angular_rate_predicate;
    let predicted_classification = if no_contact_clearance_gate {
        "none"
    } else if stable_touchdown_predicate && safe_touchdown_predicate {
        if on_target {
            "stable_touchdown_on_target"
        } else {
            "stable_touchdown_off_target"
        }
    } else {
        "crash"
    };
    ContactPredicateMirrorEvidence {
        no_contact_clearance_gate,
        stable_minimum_clearance_predicate,
        stable_maximum_clearance_predicate,
        stable_hull_penetration_predicate,
        stable_touchdown_predicate,
        safe_normal_speed_predicate,
        safe_tangential_speed_predicate,
        safe_attitude_predicate,
        safe_angular_rate_predicate,
        safe_touchdown_predicate,
        predicted_classification: predicted_classification.to_owned(),
    }
}

fn preterminal_contact_state(
    state: &SimulationState,
    context: &RunContext,
) -> PreterminalContactStateEvidence {
    let feet = touchdown_feet(state.position_m, state.attitude_rad, context);
    let minimum_hull_clearance_m = hull_vertices(state.position_m, state.attitude_rad, context)
        .into_iter()
        .map(|point| point.y - context.world.terrain.sample_height(point.x))
        .fold(f64::INFINITY, f64::min);
    let normal = context
        .world
        .terrain
        .sample_surface_normal(context.target_pad.center_x_m);
    let tangent = Vec2::new(normal.y, -normal.x);
    let normal_closing_speed_mps = (-dot(state.velocity_mps, normal)).max(0.0);
    let tangential_closing_speed_mps = dot(state.velocity_mps, tangent).abs();
    let vehicle_up = Vec2::new(state.attitude_rad.sin(), state.attitude_rad.cos());
    let attitude_error_rad = dot(vehicle_up, normal).clamp(-1.0, 1.0).acos();
    let absolute_angular_rate_radps = state.angular_rate_radps.abs();
    let hull_radius_m = (context.vehicle.geometry.hull_width_m * 0.5)
        .hypot(context.vehicle.geometry.hull_height_m * 0.5);
    let dynamic_hull_penetration_allowance_m = 0.012_f64.max(
        (normal_closing_speed_mps + absolute_angular_rate_radps * hull_radius_m)
            * context.sim.physics_dt_s(),
    );
    let left = context.target_pad.center_x_m - context.target_pad.half_width_m();
    let right = context.target_pad.center_x_m + context.target_pad.half_width_m();
    let foot_min_x = feet[0].position_m.x.min(feet[1].position_m.x);
    let foot_max_x = feet[0].position_m.x.max(feet[1].position_m.x);
    PreterminalContactStateEvidence {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        position_m: state.position_m,
        velocity_mps: state.velocity_mps,
        attitude_rad: state.attitude_rad,
        angular_rate_radps: state.angular_rate_radps,
        fuel_kg: state.fuel_kg,
        touchdown_feet: feet,
        minimum_hull_clearance_m,
        touchdown_pad_contains_both_feet: foot_min_x >= left && foot_max_x <= right,
        normal_closing_speed_mps,
        tangential_closing_speed_mps,
        attitude_error_rad,
        absolute_angular_rate_radps,
        dynamic_hull_penetration_allowance_m,
    }
}

fn v2_sign_geometry_counterfactual(
    state: &SimulationState,
    context: &RunContext,
    core: &ContactPredicateMirrorEvidence,
) -> V2SignGeometryCounterfactualEvidence {
    let thrust_direction_unit = Vec2::new(state.attitude_rad.sin(), state.attitude_rad.cos());
    let footprint_attitude_rad = (-thrust_direction_unit.x).atan2(thrust_direction_unit.y);
    let feet = touchdown_feet(state.position_m, footprint_attitude_rad, context);
    let minimum_touchdown_clearance_m = feet[0].signed_clearance_m.min(feet[1].signed_clearance_m);
    let maximum_touchdown_clearance_m = feet[0].signed_clearance_m.max(feet[1].signed_clearance_m);
    let minimum_hull_clearance_m = hull_vertices(state.position_m, footprint_attitude_rad, context)
        .into_iter()
        .map(|point| point.y - context.world.terrain.sample_height(point.x))
        .fold(f64::INFINITY, f64::min);
    let pad_left_m = context.target_pad.center_x_m - context.target_pad.half_width_m();
    let pad_right_m = context.target_pad.center_x_m + context.target_pad.half_width_m();
    let foot_min_x = feet[0].position_m.x.min(feet[1].position_m.x);
    let foot_max_x = feet[0].position_m.x.max(feet[1].position_m.x);
    let touchdown_pad_contains_both_feet = foot_min_x >= pad_left_m && foot_max_x <= pad_right_m;
    let normal = context
        .world
        .terrain
        .sample_surface_normal(context.target_pad.center_x_m);
    let normal_closing_speed_mps = (-dot(state.velocity_mps, normal)).max(0.0);
    let hull_radius_m = (context.vehicle.geometry.hull_width_m * 0.5)
        .hypot(context.vehicle.geometry.hull_height_m * 0.5);
    let dynamic_hull_penetration_allowance_m = 0.012_f64.max(
        (normal_closing_speed_mps + state.angular_rate_radps.abs() * hull_radius_m)
            * context.sim.physics_dt_s(),
    );
    let no_contact_clearance_gate =
        minimum_touchdown_clearance_m > 0.0 && minimum_hull_clearance_m > 0.0;
    let stable_minimum_clearance_predicate = minimum_touchdown_clearance_m <= 0.05;
    let stable_maximum_clearance_predicate = maximum_touchdown_clearance_m <= 0.15;
    let stable_hull_penetration_predicate =
        minimum_hull_clearance_m >= -dynamic_hull_penetration_allowance_m;
    let stable_touchdown_predicate = stable_minimum_clearance_predicate
        && stable_maximum_clearance_predicate
        && stable_hull_penetration_predicate;
    let stable_geometry_predicates_differ_from_core = stable_minimum_clearance_predicate
        != core.stable_minimum_clearance_predicate
        || stable_maximum_clearance_predicate != core.stable_maximum_clearance_predicate
        || stable_hull_penetration_predicate != core.stable_hull_penetration_predicate
        || stable_touchdown_predicate != core.stable_touchdown_predicate;
    let core_feet = touchdown_feet(state.position_m, state.attitude_rad, context);
    let core_foot_min_x = core_feet[0].position_m.x.min(core_feet[1].position_m.x);
    let core_foot_max_x = core_feet[0].position_m.x.max(core_feet[1].position_m.x);
    let core_pad_contains_both_feet =
        core_foot_min_x >= pad_left_m && core_foot_max_x <= pad_right_m;
    V2SignGeometryCounterfactualEvidence {
        convention: "v2 footprint angle = atan2(-thrust_direction.x, thrust_direction.y); geometry-only counterfactual with unchanged preterminal motion and core contact outcome".to_owned(),
        thrust_direction_unit,
        footprint_attitude_rad,
        feet,
        minimum_touchdown_clearance_m,
        maximum_touchdown_clearance_m,
        minimum_hull_clearance_m,
        no_contact_clearance_gate,
        no_contact_clearance_gate_differs_from_core: no_contact_clearance_gate
            != core.no_contact_clearance_gate,
        touchdown_pad_contains_both_feet,
        stable_minimum_clearance_predicate,
        stable_maximum_clearance_predicate,
        stable_hull_penetration_predicate,
        stable_touchdown_predicate,
        stable_geometry_predicates_differ_from_core,
        pad_membership_differs_from_core: touchdown_pad_contains_both_feet
            != core_pad_contains_both_feet,
        geometry_only_counterfactual: true,
    }
}

fn touchdown_feet(
    center: Vec2,
    attitude_rad: f64,
    context: &RunContext,
) -> [ContactFootEvidence; 2] {
    let half_span = context.vehicle.geometry.touchdown_half_span_m;
    let base = context.vehicle.geometry.touchdown_base_offset_m;
    [Vec2::new(-half_span, -base), Vec2::new(half_span, -base)].map(|local| {
        let position_m = center + local.rotated(attitude_rad);
        ContactFootEvidence {
            position_m,
            signed_clearance_m: position_m.y - context.world.terrain.sample_height(position_m.x),
        }
    })
}

fn hull_vertices(center: Vec2, attitude_rad: f64, context: &RunContext) -> [Vec2; 4] {
    let half_width = context.vehicle.geometry.hull_width_m * 0.5;
    let half_height = context.vehicle.geometry.hull_height_m * 0.5;
    [
        Vec2::new(-half_width, -half_height),
        Vec2::new(half_width, -half_height),
        Vec2::new(half_width, half_height),
        Vec2::new(-half_width, half_height),
    ]
    .map(|point| center + point.rotated(attitude_rad))
}

fn dot(left: Vec2, right: Vec2) -> f64 {
    left.x * right.x + left.y * right.y
}

fn state_evidence(state: &SimulationState, initial_fuel_kg: f64) -> PlantStateEvidence {
    PlantStateEvidence {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        position_m: state.position_m,
        velocity_mps: state.velocity_mps,
        attitude_rad: state.attitude_rad,
        angular_rate_radps: state.angular_rate_radps,
        fuel_kg: state.fuel_kg,
        fuel_used_kg: initial_fuel_kg - state.fuel_kg,
        physical_outcome: enum_label(&state.physical_outcome),
        mission_outcome: enum_label(&state.mission_outcome),
        end_reason: enum_label(&state.end_reason),
    }
}

fn orientation_convention_probes() -> Vec<OrientationConventionProbeEvidence> {
    let flat = TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-20.0, 0.0), Vec2::new(20.0, 0.0)],
    };
    let slope = TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-20.0, -2.0), Vec2::new(20.0, 2.0)],
    };
    [ORIENTATION_TILT_RAD, -ORIENTATION_TILT_RAD]
        .into_iter()
        .map(|tilt| {
            let direction = Vec2::new(tilt.sin(), tilt.cos());
            let v2_attitude = (-direction.x).atan2(direction.y);
            let core_up = Vec2::new(0.0, 1.0).rotated(tilt);
            OrientationConventionProbeEvidence {
                tilt_attitude_rad: tilt,
                thrust_direction_unit: direction,
                core_body_up_after_rotation: core_up,
                core_footprint_attitude_rad: tilt,
                v2_footprint_attitude_rad: v2_attitude,
                signed_footprint_attitude_delta_rad: shortest_angle_delta(tilt, v2_attitude),
                centered_flat_core_footprint: orientation_footprint(
                    Vec2::new(0.0, 5.0),
                    tilt,
                    0.0,
                    6.0,
                    &flat,
                    "flat_height_zero",
                ),
                centered_flat_v2_footprint: orientation_footprint(
                    Vec2::new(0.0, 5.0),
                    v2_attitude,
                    0.0,
                    6.0,
                    &flat,
                    "flat_height_zero",
                ),
                centered_slope_core_footprint: orientation_footprint(
                    Vec2::new(0.0, 5.0),
                    tilt,
                    0.0,
                    6.0,
                    &slope,
                    "plane_height_x_times_0.1",
                ),
                centered_slope_v2_footprint: orientation_footprint(
                    Vec2::new(0.0, 5.0),
                    v2_attitude,
                    0.0,
                    6.0,
                    &slope,
                    "plane_height_x_times_0.1",
                ),
                flat_pad_edge_core_footprint: orientation_footprint(
                    Vec2::new(1.75, 5.0),
                    tilt,
                    0.0,
                    6.0,
                    &flat,
                    "flat_height_zero",
                ),
                flat_pad_edge_v2_footprint: orientation_footprint(
                    Vec2::new(1.75, 5.0),
                    v2_attitude,
                    0.0,
                    6.0,
                    &flat,
                    "flat_height_zero",
                ),
                synthetic_only: true,
            }
        })
        .collect()
}

fn orientation_footprint(
    vehicle_center_m: Vec2,
    attitude_rad: f64,
    pad_center_x_m: f64,
    pad_half_width_m: f64,
    terrain: &TerrainDefinition,
    terrain_rule: &str,
) -> OrientationFootprintEvidence {
    let feet = [
        Vec2::new(-FOOT_HALF_SPAN_M, -FOOT_BASE_OFFSET_M),
        Vec2::new(FOOT_HALF_SPAN_M, -FOOT_BASE_OFFSET_M),
    ]
    .map(|local| {
        let position_m = vehicle_center_m + local.rotated(attitude_rad);
        ContactFootEvidence {
            position_m,
            signed_clearance_m: position_m.y - terrain.sample_height(position_m.x),
        }
    });
    let pad_left_m = pad_center_x_m - pad_half_width_m;
    let pad_right_m = pad_center_x_m + pad_half_width_m;
    let contains = feet
        .iter()
        .all(|foot| foot.position_m.x >= pad_left_m && foot.position_m.x <= pad_right_m);
    OrientationFootprintEvidence {
        vehicle_center_m,
        pad_center_x_m,
        pad_half_width_m,
        terrain_rule: terrain_rule.to_owned(),
        feet,
        pad_contains_both_feet: contains,
    }
}

fn shortest_angle_delta(current_rad: f64, target_rad: f64) -> f64 {
    let turn = std::f64::consts::TAU;
    (target_rad - current_rad + std::f64::consts::PI).rem_euclid(turn) - std::f64::consts::PI
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use pd_core::{
        EvaluationGoal, LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig, VehicleGeometry,
        VehicleInitialState, VehicleSpec, WorldSpec,
    };

    fn test_context() -> RunContext {
        RunContext::from_scenario(&ScenarioSpec {
            id: "contact-contract-test".to_owned(),
            name: "Contact contract test".to_owned(),
            description: "synthetic unit-test context".to_owned(),
            seed: 1,
            tags: vec!["test".to_owned()],
            metadata: BTreeMap::new(),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 10.0,
                sample_hz: None,
            },
            world: WorldSpec {
                gravity_mps2: 1.62,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-100.0, 0.0), Vec2::new(100.0, 0.0)],
                },
                landing_pads: vec![LandingPadSpec {
                    id: "test-pad".to_owned(),
                    center_x_m: 0.0,
                    surface_y_m: 0.0,
                    width_m: 40.0,
                }],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 4.0,
                    hull_height_m: 6.0,
                    touchdown_half_span_m: FOOT_HALF_SPAN_M,
                    touchdown_base_offset_m: FOOT_BASE_OFFSET_M,
                },
                dry_mass_kg: 700.0,
                initial_fuel_kg: 200.0,
                max_fuel_kg: 200.0,
                max_thrust_n: 14_000.0,
                max_fuel_burn_kgps: 10.0,
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.0,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.2,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(0.0, 5.0002),
                velocity_mps: Vec2::new(0.0, -0.1),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                transfer_route: None,
                goal: EvaluationGoal::LandingOnPad {
                    target_pad_id: "test-pad".to_owned(),
                },
            },
        })
        .expect("valid test context")
    }

    #[test]
    fn pinned_launch_identity_rejects_stale_and_self_consistent_tampering() {
        let tampered_digest = "fnv1a64:0000000000000000";
        assert!(
            validate_pinned_launch_identity_values(EXPECTED_LAUNCH_IDENTITY, tampered_digest,)
                .is_err()
        );
        assert!(validate_pinned_launch_identity_values(tampered_digest, tampered_digest).is_err());
        assert!(
            validate_pinned_launch_identity_values(
                EXPECTED_LAUNCH_IDENTITY,
                EXPECTED_LAUNCH_IDENTITY,
            )
            .is_ok()
        );
    }

    #[test]
    fn flat_gate_requires_both_distinct_roles_and_all_four_cadence_rows() {
        let mut gate = FlatContactContractGateEvidence {
            case_id: FLAT_CASE_ID.to_owned(),
            required_role_order: vec![
                "v2_native_selected".to_owned(),
                "research_shortest_certified".to_owned(),
            ],
            audited_candidate_count: 2,
            audited_cadence_row_count: 4,
            required_role_bindings_valid: true,
            all_required_cadence_rows_present: true,
            all_first_contacts_observed: true,
            all_replay_and_classifier_parity_passed: true,
            native_stable_landing_observed: true,
            research_shortest_crash_observed: true,
            passed: false,
            consequence: String::new(),
        };
        assert!(flat_gate_satisfied(&gate));
        gate.audited_candidate_count = 0;
        assert!(!flat_gate_satisfied(&gate));
        gate.audited_candidate_count = 2;
        gate.audited_cadence_row_count = 2;
        assert!(!flat_gate_satisfied(&gate));
        gate.audited_cadence_row_count = 4;
        gate.required_role_bindings_valid = false;
        assert!(!flat_gate_satisfied(&gate));
        gate.required_role_bindings_valid = true;
        gate.all_required_cadence_rows_present = false;
        assert!(!flat_gate_satisfied(&gate));
        gate.all_required_cadence_rows_present = true;
        gate.all_first_contacts_observed = false;
        assert!(!flat_gate_satisfied(&gate));
        gate.all_first_contacts_observed = true;
        gate.all_replay_and_classifier_parity_passed = false;
        assert!(!flat_gate_satisfied(&gate));
        gate.all_replay_and_classifier_parity_passed = true;
        gate.native_stable_landing_observed = false;
        assert!(!flat_gate_satisfied(&gate));
        gate.native_stable_landing_observed = true;
        gate.research_shortest_crash_observed = false;
        assert!(!flat_gate_satisfied(&gate));
    }

    #[test]
    fn local_mirror_matches_core_and_captures_preterminal_landing_velocity() {
        let context = test_context();
        let mut neutral = SimulationState::new(&context).unwrap();
        let mut ordinary = neutral.clone();

        let contact = neutral.step_physics_and_classify_contact(&context);
        let mirror = mirror_contact_predicates(&neutral, &context);
        assert_eq!(
            mirror.predicted_classification,
            contact_classification_label(&contact)
        );
        assert_eq!(
            contact,
            ContactClassification::StableTouchdown { on_target: true }
        );
        let captured = preterminal_contact_state(&neutral, &context);
        assert!(captured.velocity_mps.y < 0.0);
        assert_eq!(captured.physics_step, 1);

        let events = ordinary.step(&context);
        assert_eq!(
            event_contact_label(&events),
            contact_classification_label(&contact)
        );
        assert!(ordinary.is_terminal());
        assert_eq!(ordinary.velocity_mps, Vec2::new(0.0, 0.0));
        assert_ne!(captured.velocity_mps, ordinary.velocity_mps);
    }

    #[test]
    fn local_mirror_matches_max_foot_clearance_crash_with_other_gates_passing() {
        let mut context = test_context();
        context.initial_state.position_m = Vec2::new(0.0, 5.374);
        context.initial_state.velocity_mps = Vec2::new(0.0, 0.0);
        context.initial_state.attitude_rad = 0.1;
        let mut neutral = SimulationState::new(&context).unwrap();
        neutral.set_command(Command {
            throttle_frac: 0.0,
            target_attitude_rad: 0.1,
        });
        let mut ordinary = neutral.clone();

        let contact = neutral.step_physics_and_classify_contact(&context);
        let predicates = mirror_contact_predicates(&neutral, &context);
        assert_eq!(contact, ContactClassification::Crash);
        assert_eq!(predicates.predicted_classification, "crash");
        assert!(!predicates.no_contact_clearance_gate);
        assert!(predicates.stable_minimum_clearance_predicate);
        assert!(!predicates.stable_maximum_clearance_predicate);
        assert!(predicates.stable_hull_penetration_predicate);
        assert!(predicates.safe_normal_speed_predicate);
        assert!(predicates.safe_tangential_speed_predicate);
        assert!(predicates.safe_attitude_predicate);
        assert!(predicates.safe_angular_rate_predicate);

        let events = ordinary.step(&context);
        assert_eq!(event_contact_label(&events), "crash");
    }

    #[test]
    fn orientation_probes_keep_slope_and_flat_pad_edge_evidence_separate() {
        let probes = orientation_convention_probes();
        assert_eq!(probes.len(), 2);
        let positive = probes
            .iter()
            .find(|probe| probe.tilt_attitude_rad > 0.0)
            .unwrap();
        let negative = probes
            .iter()
            .find(|probe| probe.tilt_attitude_rad < 0.0)
            .unwrap();

        assert!(!positive.flat_pad_edge_core_footprint.pad_contains_both_feet);
        assert!(positive.flat_pad_edge_v2_footprint.pad_contains_both_feet);
        assert!(negative.flat_pad_edge_core_footprint.pad_contains_both_feet);
        assert!(!negative.flat_pad_edge_v2_footprint.pad_contains_both_feet);
        assert_ne!(
            positive.centered_slope_core_footprint.feet[0].signed_clearance_m,
            positive.centered_slope_v2_footprint.feet[0].signed_clearance_m,
        );
        assert_eq!(
            positive.flat_pad_edge_core_footprint.terrain_rule,
            "flat_height_zero",
        );
        assert_eq!(
            positive.centered_slope_core_footprint.terrain_rule,
            "plane_height_x_times_0.1",
        );
    }
}
