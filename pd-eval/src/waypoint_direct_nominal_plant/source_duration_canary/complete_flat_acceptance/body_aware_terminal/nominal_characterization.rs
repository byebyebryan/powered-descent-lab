//! Bounded evaluator-only characterization of a shared nominal acquisition
//! and terminal-entry family. Corpus snapshots are comparison values only;
//! retained H states are always rebuilt from a fresh context and their full
//! original command prefix.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use pd_core::{
    Command, ContactClassification, EndReason, FlightProgramUpdateV1, MissionOutcome,
    PhysicalOutcome, RunContext, ScenarioSpec, SimulationState, SimulationStateSnapshotV1, Vec2,
};
use pd_plan::conservative_ballistic_bridge::KinematicStateV2;
use serde::{Deserialize, Serialize};

use super::*;
mod ground_diagnostic;
mod runtime_airborne;
mod terminal_time;
use crate::{
    WaypointDirectNominalDirectGenerationPolicyV1,
    nominal_direct_flight::{reserve_output_root, write_create_only},
    waypoint_direct_body_aware_terminal::sha256_bytes,
};
pub use ground_diagnostic::run_waypoint_v2_ground_diagnostic;
pub use runtime_airborne::*;
pub use terminal_time::run_waypoint_v2_terminal_time;

pub const WAYPOINT_V2_NOMINAL_CHARACTERIZATION_ID: &str = "waypoint-v2-nominal-characterization";
pub const WAYPOINT_V2_NOMINAL_CHARACTERIZATION_SCHEMA_ID: &str =
    "waypoint_v2_nominal_characterization_v1";
pub const WAYPOINT_V2_NOMINAL_CHARACTERIZATION_CORPUS_SCHEMA_ID: &str =
    "waypoint_v2_nominal_characterization_corpus_v1";

const EXPECTED_CLEAR_STARTS: usize = 8;
const EXPECTED_LOCAL_HANDOFFS: usize = 27;
const EXPECTED_HISTORICAL_CAPTURES: usize = 12;
const EXPECTED_RETAINED_ROWS: usize =
    EXPECTED_CLEAR_STARTS + EXPECTED_LOCAL_HANDOFFS + EXPECTED_HISTORICAL_CAPTURES;
// This is a sealed study, not a generic runtime input interface. Hashing the
// source files alone does not bind arbitrary embedded rows to those sources.
const APPROVED_CORPUS_SHA256: &str =
    "c570da97f46cdaa5daf70238964d52afaad663107f42e5ef3205a7d186a7c128";
const EXPECTED_SYNTHETIC_ROWS: usize = 8;
const HELD_TICKS: u64 = 2;
const MAX_GROUND_PREP_TICKS: u64 = 240;
const MAX_TURN_UPDATES: usize = 3;
const MAX_SEEDS: usize = 13;
const MAX_ENTRY_SCREENS: usize = 4;
const MAX_PHYSICAL_WITNESSES: usize = 3;
const TERMINAL_EXTRA_TICKS: u64 = 4;
const MAX_THRUST_FRACTION: f64 = 0.925;
const FLOAT_COMPARE_TOLERANCE: f64 = 1.0e-9;
const TARGET_ENTRY_RESERVE_M: f64 = 5.0;
const ENTRY_COAST_FRACTIONS: [f64; MAX_ENTRY_SCREENS] = [0.0, 0.25, 0.5, 0.75];
const ACQUISITION_BURN_FRACTIONS: [f64; 3] = [0.25, 0.45, 0.65];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2NominalCharacterizationCorpusV1 {
    pub schema_id: String,
    pub bindings: Vec<WaypointV2NominalCharacterizationBindingV1>,
    pub rows: Vec<WaypointV2NominalCharacterizationCorpusRowV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2NominalCharacterizationBindingV1 {
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2NominalCharacterizationCorpusRowV1 {
    pub id: String,
    pub population: String,
    pub label: String,
    pub scenario: ScenarioSpec,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub absolute_deadline_physics_step: u64,
    pub expected_state: Option<SimulationStateSnapshotV1>,
    pub prefix_updates: Vec<FlightProgramUpdateV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2NominalCharacterizationArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub characterization_id: String,
    pub corpus_sha256: String,
    pub verified_binding_count: usize,
    pub verified_binding_sha256s: Vec<String>,
    pub retained_row_count: usize,
    pub synthetic_row_count: usize,
    pub retained_rows: Vec<NominalCharacterizationRowEvidenceV1>,
    pub synthetic_rows: Vec<NominalCharacterizationRowEvidenceV1>,
    pub integrity_errors: Vec<String>,
    pub integrity_passed: bool,
    /// Stable payload identity omits filesystem paths and observational timing.
    pub repeat_payload_identity: String,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalCharacterizationRowEvidenceV1 {
    pub row_id: String,
    pub population: String,
    pub label: String,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub initial_physics_step: u64,
    pub initial_fuel_kg: f64,
    pub original_deadline_physics_step: u64,
    pub expected_state_match: Option<SnapshotMatchEvidenceV1>,
    pub prefix_update_count: usize,
    pub ground_prep: Option<GroundPrepEvidenceV1>,
    pub seeds: Vec<NominalSeedEvidenceV1>,
    pub shortlist_seed_ids: Vec<String>,
    pub witnesses: Vec<NominalWitnessEvidenceV1>,
    pub finite_miss_reasons: Vec<String>,
    pub integrity_errors: Vec<String>,
    pub status: String,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMatchEvidenceV1 {
    pub matched: bool,
    pub comparison_rule: String,
    pub maximum_float_delta: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroundPrepEvidenceV1 {
    pub success: bool,
    pub physics_ticks: u64,
    pub source_clearance_threshold_m: f64,
    pub required_vertical_speed_mps: f64,
    pub source_contact_steps: Vec<u64>,
    pub minimum_body_reserve_margin_m: Option<f64>,
    pub first_reserve_violation_physics_step: Option<u64>,
    pub reserve_query_error: Option<String>,
    pub integrity_error: Option<String>,
    pub start_state: SimulationStateSnapshotV1,
    pub end_state: SimulationStateSnapshotV1,
    pub commands: Vec<FlightProgramUpdateV1>,
    pub command_identity: String,
    pub independent_replay_passed: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalSeedEvidenceV1 {
    pub seed_id: String,
    pub kind: String,
    pub preserve_vertical: bool,
    pub virtual_arrival_physics_ticks: u64,
    pub burn_fraction: f64,
    pub turn_physics_ticks: u64,
    pub burn_physics_ticks: u64,
    pub remaining_virtual_physics_ticks: u64,
    pub requested_thrust_acceleration_mps2: Option<Vec2>,
    pub predicted_acquisition_end: Option<KinematicStateV2>,
    pub virtual_target_error_m: Option<f64>,
    pub estimated_acquisition_fuel_kg: Option<f64>,
    pub upward_impulse_mps: Option<f64>,
    pub turn_updates: usize,
    pub status: String,
    pub reason: Option<String>,
    pub entry_screens: Vec<NominalEntryScreenEvidenceV1>,
    pub selected_entry_index: Option<usize>,
    pub estimated_fuel_kg: Option<f64>,
    pub predicted_finish_physics_step: Option<u64>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalEntryScreenEvidenceV1 {
    pub entry_index: usize,
    pub coast_fraction: f64,
    pub coast_physics_ticks: u64,
    pub predicted_entry: KinematicStateV2,
    pub predicted_entry_angle_rad: f64,
    pub terminal_time_s: Option<f64>,
    pub terminal_physics_ticks: Option<u64>,
    pub coupled_thrust_bound_mps2: Option<f64>,
    pub terminal_fuel_kg: Option<f64>,
    pub total_predicted_fuel_kg: Option<f64>,
    pub predicted_finish_physics_step: Option<u64>,
    pub lateral_velocity_reversal_checked: bool,
    pub admissible: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalWitnessEvidenceV1 {
    pub seed_id: String,
    pub entry_index: usize,
    pub status: String,
    pub reason: Option<String>,
    pub acquisition_start: SimulationStateSnapshotV1,
    pub predicted_acquisition_end: KinematicStateV2,
    pub actual_acquisition_end: SimulationStateSnapshotV1,
    pub acquisition_position_error_m: f64,
    pub acquisition_velocity_error_mps: f64,
    pub acquisition_fuel_error_kg: f64,
    pub acquisition_turn_command_identity: String,
    pub acquisition_command_identity: String,
    pub actual_terminal_entry: Option<SimulationStateSnapshotV1>,
    pub actual_terminal_entry_angle_rad: Option<f64>,
    pub actual_entry_position_error_m: Option<f64>,
    pub actual_entry_velocity_error_mps: Option<f64>,
    pub commands: Vec<FlightProgramUpdateV1>,
    pub command_identity: String,
    pub independent_replay_passed: bool,
    pub endpoint_state_agreement: bool,
    pub target_plane_witness: Option<TargetPlaneWitnessEvidenceV1>,
    pub terrain_audit: Option<RealTerrainAuditEvidenceV1>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TargetPlaneWitnessEvidenceV1 {
    pub physics_step: u64,
    pub safe_by_existing_target_plane_mirror: bool,
    pub proposal_endpoint_contact_match: bool,
    pub free_space_endpoint_match: bool,
    pub actual_landing_claimed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealTerrainAuditEvidenceV1 {
    pub ordinary_neutral_prefix_parity: bool,
    pub first_contact: Option<TerminalContactAuditEvidence>,
    pub first_contact_physics_step: Option<u64>,
    pub final_state: SimulationStateSnapshotV1,
    pub stopped_on_contact: bool,
    pub landed_on_target: bool,
    pub target_plane_witness_reached_before_terrain_contact: bool,
    pub terrain_contact_blocks_landing_claim: bool,
    pub minimum_hull_clearance_m: Option<f64>,
    pub reserve_phases: Vec<BodyReservePhaseEvidenceV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyReservePhaseEvidenceV1 {
    pub phase: String,
    pub minimum_reserve_margin_m: Option<f64>,
    pub first_reserve_violation_physics_step: Option<u64>,
    pub first_query_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct RepeatPayloadV1<'a> {
    corpus_sha256: &'a str,
    retained_rows: &'a [NominalCharacterizationRowEvidenceV1],
    synthetic_rows: &'a [NominalCharacterizationRowEvidenceV1],
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum WitnessMaterializationFailure {
    FiniteMiss(String),
    Integrity(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2NominalCharacterizationRunSummaryV1 {
    pub characterization_id: String,
    pub identity: String,
    pub repeat_payload_identity: String,
    pub retained_row_count: usize,
    pub synthetic_row_count: usize,
    pub integrity_passed: bool,
    pub finite_miss_row_count: usize,
    pub target_plane_witness_count: usize,
    pub real_terrain_landing_count: usize,
}

pub fn run_waypoint_v2_nominal_characterization(
    corpus_path: &Path,
    output_root: &Path,
) -> Result<WaypointV2NominalCharacterizationRunSummaryV1> {
    let corpus_bytes = fs::read(corpus_path).with_context(|| {
        format!(
            "read nominal-characterization corpus {}",
            corpus_path.display()
        )
    })?;
    let corpus_sha256 = sha256_bytes(&corpus_bytes)?;
    verify_approved_corpus_sha256(&corpus_sha256)?;
    let corpus: WaypointV2NominalCharacterizationCorpusV1 =
        serde_json::from_slice(&corpus_bytes).context("parse nominal-characterization corpus")?;
    validate_corpus(&corpus)?;
    let repo_root = repo_root()?;
    verify_bindings(&repo_root, &corpus.bindings)?;
    reserve_output_root(output_root)?;

    let mut retained_rows = Vec::with_capacity(corpus.rows.len());
    let mut integrity_errors = Vec::new();
    for row in &corpus.rows {
        match evaluate_retained_row(row) {
            Ok(evidence) => {
                integrity_errors.extend(
                    evidence
                        .integrity_errors
                        .iter()
                        .map(|reason| format!("{}: {reason}", evidence.row_id)),
                );
                retained_rows.push(evidence);
            }
            Err(error) => {
                integrity_errors.push(format!("{}: {error:#}", row.id));
                retained_rows.push(integrity_error_row(row, format!("{error:#}"))?);
            }
        }
    }

    let mut synthetic_rows = Vec::new();
    if let Some(base) = corpus
        .rows
        .iter()
        .find(|row| row.population == "clear_start")
    {
        for (scenario, label) in
            synthetic_scenarios(&base.scenario, &base.source_pad_id, &base.target_pad_id)?
        {
            let evidence = evaluate_synthetic_row(
                &scenario,
                &base.source_pad_id,
                &base.target_pad_id,
                base.absolute_deadline_physics_step,
                label,
            )?;
            integrity_errors.extend(
                evidence
                    .integrity_errors
                    .iter()
                    .map(|reason| format!("{}: {reason}", evidence.row_id)),
            );
            synthetic_rows.push(evidence);
        }
    } else {
        integrity_errors.push("synthetic source scenario is missing".to_owned());
    }
    if synthetic_rows.len() != EXPECTED_SYNTHETIC_ROWS {
        integrity_errors.push(format!(
            "expected {EXPECTED_SYNTHETIC_ROWS} synthetic conditions, produced {}",
            synthetic_rows.len()
        ));
    }

    let mut post_binding_errors = verify_bindings(&repo_root, &corpus.bindings)
        .err()
        .map(|error| vec![format!("post-run source binding check: {error:#}")])
        .unwrap_or_default();
    let corpus_after = fs::read(corpus_path).with_context(|| {
        format!(
            "re-read nominal-characterization corpus {}",
            corpus_path.display()
        )
    })?;
    if sha256_bytes(&corpus_after)? != corpus_sha256 {
        post_binding_errors.push("corpus changed while the study was running".to_owned());
    }
    integrity_errors.extend(post_binding_errors);

    let repeat_payload_identity = stable_digest(&RepeatPayloadV1 {
        corpus_sha256: &corpus_sha256,
        retained_rows: &retained_rows,
        synthetic_rows: &synthetic_rows,
    })?;
    let binding_sha256s = corpus
        .bindings
        .iter()
        .map(|binding| binding.sha256.clone())
        .collect::<Vec<_>>();
    let mut artifact = WaypointV2NominalCharacterizationArtifactV1 {
        schema_id: WAYPOINT_V2_NOMINAL_CHARACTERIZATION_SCHEMA_ID.to_owned(),
        schema_version: 1,
        characterization_id: WAYPOINT_V2_NOMINAL_CHARACTERIZATION_ID.to_owned(),
        corpus_sha256,
        verified_binding_count: binding_sha256s.len(),
        verified_binding_sha256s: binding_sha256s,
        retained_row_count: retained_rows.len(),
        synthetic_row_count: synthetic_rows.len(),
        retained_rows,
        synthetic_rows,
        integrity_passed: integrity_errors.is_empty(),
        integrity_errors,
        repeat_payload_identity,
        scope_non_claims: vec![
            "evaluator-only research; no planner policy/default change".into(),
            "not an all-cases landing requirement or implementation-readiness assertion".into(),
            "synthetic rows are fresh initial-state conditions, not retained snapshot execution".into(),
            "target-plane witnesses are free-space evidence and are not actual terrain landing proof".into(),
        ],
        identity: String::new(),
    };
    artifact.identity = stable_digest(&artifact)?;
    write_create_only(&output_root.join("summary.json"), &artifact)?;

    Ok(WaypointV2NominalCharacterizationRunSummaryV1 {
        characterization_id: artifact.characterization_id,
        identity: artifact.identity,
        repeat_payload_identity: artifact.repeat_payload_identity,
        retained_row_count: artifact.retained_row_count,
        synthetic_row_count: artifact.synthetic_row_count,
        integrity_passed: artifact.integrity_passed,
        finite_miss_row_count: artifact
            .retained_rows
            .iter()
            .chain(&artifact.synthetic_rows)
            .filter(|row| row.status == "finite_miss")
            .count(),
        target_plane_witness_count: artifact
            .retained_rows
            .iter()
            .chain(&artifact.synthetic_rows)
            .flat_map(|row| &row.witnesses)
            .filter(|witness| {
                witness.target_plane_witness.as_ref().is_some_and(|plane| {
                    plane.safe_by_existing_target_plane_mirror
                        && plane.proposal_endpoint_contact_match
                })
            })
            .count(),
        real_terrain_landing_count: artifact
            .retained_rows
            .iter()
            .chain(&artifact.synthetic_rows)
            .flat_map(|row| &row.witnesses)
            .filter(|witness| {
                witness
                    .terrain_audit
                    .as_ref()
                    .is_some_and(|audit| audit.landed_on_target)
            })
            .count(),
    })
}

fn repo_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .context("pd-eval manifest has no repository parent")
}

fn verify_approved_corpus_sha256(actual: &str) -> Result<()> {
    if actual != APPROVED_CORPUS_SHA256 {
        bail!(
            "sealed characterization corpus SHA-256 mismatch: expected {APPROVED_CORPUS_SHA256}, received {actual}; regenerate with the corpus builder"
        );
    }
    Ok(())
}

fn ground_prep(
    context: &RunContext,
    scenario: &ScenarioSpec,
    source_pad_id: &str,
    target_pad_id: &str,
    live: &mut SimulationState,
    deadline: u64,
) -> Result<GroundPrepEvidenceV1> {
    let start_state = SimulationStateSnapshotV1::from_state(live);
    let source_pad = scenario
        .world
        .landing_pad(source_pad_id)
        .context("ground prep source pad is missing")?;
    let request = research_request(scenario, source_pad_id, target_pad_id);
    let radius = (context.vehicle.geometry.hull_width_m * 0.5)
        .hypot(context.vehicle.geometry.hull_height_m * 0.5);
    let clearance_threshold = radius + 10.0;
    let required_vertical_speed = context.world.gravity_mps2
        * (std::f64::consts::PI / (2.0 * context.vehicle.max_rotation_rate_radps));
    let max_ticks = MAX_GROUND_PREP_TICKS.min(deadline.saturating_sub(live.physics_step));
    let max_aligned_ticks = max_ticks - max_ticks % HELD_TICKS;
    let mut updates = Vec::new();
    let mut source_contacts = Vec::new();
    let mut minimum_reserve_margin = None::<f64>;
    let mut first_reserve_violation = None;
    let mut reserve_query_error = None;
    let mut integrity_error = None;
    let mut success = false;
    let mut reason = None;
    for _ in 0..max_aligned_ticks {
        if live.physics_step.is_multiple_of(HELD_TICKS) {
            let command = Command {
                throttle_frac: 1.0,
                target_attitude_rad: 0.0,
            };
            updates.push(FlightProgramUpdateV1 {
                physics_step: live.physics_step,
                phase: "ground_source_upright_prep".into(),
                command,
            });
            live.set_command(command);
        }
        let classification = live.step_physics_and_classify_contact(context);
        if classification != ContactClassification::None {
            if source_pad_contact(context, source_pad, live) {
                source_contacts.push(live.physics_step);
            } else {
                reason = Some(format!(
                    "ground prep encountered non-source contact at H{}",
                    live.physics_step
                ));
                break;
            }
        }
        match nominal_body_reserve_query(
            context, &request, live,
            // The private query recognizes the established "upright" corridor
            // name. Keep our more descriptive phase label in the command log.
            "upright", true,
        ) {
            Ok((clearance, required)) => {
                let margin = clearance - required;
                minimum_reserve_margin =
                    Some(minimum_reserve_margin.map_or(margin, |prior| prior.min(margin)));
                if margin < 0.0 && first_reserve_violation.is_none() {
                    first_reserve_violation = Some(live.physics_step);
                }
            }
            Err(error) => {
                let message = format!("{error:#}");
                reserve_query_error.get_or_insert_with(|| message.clone());
                integrity_error
                    .get_or_insert_with(|| format!("ground prep reserve query failed: {message}"));
            }
        };
        if !finite_live_state(live) {
            reason = Some("ground prep produced nonfinite live plant state".into());
            integrity_error = reason.clone();
            break;
        }
        if live.fuel_kg <= 0.0 {
            reason = Some("ground prep exhausted fuel before source clearance".into());
            break;
        }
        if live.physics_step.is_multiple_of(HELD_TICKS)
            && live.position_m.y - source_pad.surface_y_m > clearance_threshold
            && live.velocity_mps.y > required_vertical_speed
        {
            success = true;
            break;
        }
    }
    if !success && reason.is_none() {
        reason = Some(format!(
            "ground prep did not satisfy bounded source clearance within {max_aligned_ticks} ticks"
        ));
    }
    if !live.physics_step.is_multiple_of(HELD_TICKS) {
        bail!("ground prep ended off the global held-command boundary");
    }
    Ok(GroundPrepEvidenceV1 {
        success,
        physics_ticks: live.physics_step - start_state.physics_step,
        source_clearance_threshold_m: clearance_threshold,
        required_vertical_speed_mps: required_vertical_speed,
        source_contact_steps: source_contacts,
        minimum_body_reserve_margin_m: minimum_reserve_margin,
        first_reserve_violation_physics_step: first_reserve_violation,
        reserve_query_error,
        integrity_error,
        start_state,
        end_state: SimulationStateSnapshotV1::from_state(live),
        commands: updates.clone(),
        command_identity: stable_digest(&updates)?,
        independent_replay_passed: false,
        reason,
    })
}

fn verify_ground_prep_replay(context: &RunContext, prep: &mut GroundPrepEvidenceV1) -> Result<()> {
    let fresh = SimulationState::new(context)?;
    if !compare_full_snapshot(
        &SimulationStateSnapshotV1::from_state(&fresh),
        &prep.start_state,
    )
    .matched
    {
        prep.integrity_error = Some("ground prep did not start at the fresh H0 state".into());
        return Ok(());
    }
    match replay_neutral(
        context,
        &fresh,
        &prep.commands,
        prep.end_state.physics_step,
        prep.end_state.physics_step,
        prep.end_state.physics_step,
        &prep.end_state,
    ) {
        Ok(replay) => {
            prep.independent_replay_passed = replay.acquisition_end_match
                && compare_full_snapshot(
                    &SimulationStateSnapshotV1::from_state(&replay.final_state),
                    &prep.end_state,
                )
                .matched;
            if !prep.independent_replay_passed {
                prep.integrity_error = Some(
                    "fresh H0 ground-prep replay disagrees with the realized end state".into(),
                );
            }
        }
        Err(error) => {
            prep.integrity_error = Some(format!("fresh H0 ground-prep replay failed: {error:#}"));
        }
    }
    Ok(())
}

fn source_pad_contact(
    context: &RunContext,
    source_pad: &pd_core::LandingPadSpec,
    state: &SimulationState,
) -> bool {
    let body_radius = (context.vehicle.geometry.hull_width_m * 0.5)
        .hypot(context.vehicle.geometry.hull_height_m * 0.5);
    (state.position_m.x - source_pad.center_x_m).abs() <= source_pad.half_width_m() + body_radius
        && state.position_m.y >= source_pad.surface_y_m - body_radius
        && state.position_m.y <= source_pad.surface_y_m + 2.0 * body_radius
}

fn evaluate_synthetic_row(
    scenario: &ScenarioSpec,
    source_pad_id: &str,
    target_pad_id: &str,
    deadline: u64,
    label: String,
) -> Result<NominalCharacterizationRowEvidenceV1> {
    let row = WaypointV2NominalCharacterizationCorpusRowV1 {
        id: format!("synthetic:{}", scenario.id),
        population: "synthetic_condition".into(),
        label,
        scenario: scenario.clone(),
        source_pad_id: source_pad_id.to_owned(),
        target_pad_id: target_pad_id.to_owned(),
        absolute_deadline_physics_step: deadline,
        expected_state: None,
        prefix_updates: Vec::new(),
    };
    evaluate_retained_row(&row)
}

fn synthetic_scenarios(
    source: &ScenarioSpec,
    source_pad_id: &str,
    target_pad_id: &str,
) -> Result<Vec<(ScenarioSpec, String)>> {
    #[derive(Clone, Copy)]
    struct Condition {
        id: &'static str,
        label: &'static str,
        target_dx_m: f64,
        altitude_above_target_com_m: f64,
        velocity_x_mps: f64,
        velocity_y_mps: f64,
        attitude_rad: f64,
        fuel_kg: Option<f64>,
        uphill_m: f64,
    }
    let conditions = [
        Condition {
            id: "safe_descent",
            label: "synthetic safe descending start; no mandatory future apex",
            target_dx_m: 480.0,
            altitude_above_target_com_m: 700.0,
            velocity_x_mps: 24.0,
            velocity_y_mps: -32.0,
            attitude_rad: 0.0,
            fuel_kg: None,
            uphill_m: 0.0,
        },
        Condition {
            id: "steeper_arrival",
            label: "synthetic steeper arrival",
            target_dx_m: 220.0,
            altitude_above_target_com_m: 250.0,
            velocity_x_mps: 5.0,
            velocity_y_mps: -42.0,
            attitude_rad: 0.0,
            fuel_kg: None,
            uphill_m: 0.0,
        },
        Condition {
            id: "lateral_miss",
            label: "synthetic lateral miss with descending vertical motion",
            target_dx_m: 520.0,
            altitude_above_target_com_m: 440.0,
            velocity_x_mps: 46.0,
            velocity_y_mps: -13.0,
            attitude_rad: 0.0,
            fuel_kg: None,
            uphill_m: 0.0,
        },
        Condition {
            id: "shallow_high_energy",
            label: "synthetic shallow high-energy entry",
            target_dx_m: 260.0,
            altitude_above_target_com_m: 360.0,
            velocity_x_mps: 68.0,
            velocity_y_mps: -20.0,
            attitude_rad: 0.0,
            fuel_kg: None,
            uphill_m: 0.0,
        },
        Condition {
            id: "uphill_target",
            label: "synthetic uphill target; initial state already above the target",
            target_dx_m: 430.0,
            altitude_above_target_com_m: 440.0,
            velocity_x_mps: 31.0,
            velocity_y_mps: -15.0,
            attitude_rad: 0.0,
            fuel_kg: None,
            uphill_m: 120.0,
        },
        Condition {
            id: "low_fuel",
            label: "synthetic low-fuel descent",
            target_dx_m: 350.0,
            altitude_above_target_com_m: 500.0,
            velocity_x_mps: 24.0,
            velocity_y_mps: -24.0,
            attitude_rad: 0.0,
            fuel_kg: Some(3.0),
            uphill_m: 0.0,
        },
        Condition {
            id: "large_turn",
            label: "synthetic large initial attitude turn",
            target_dx_m: 420.0,
            altitude_above_target_com_m: 500.0,
            velocity_x_mps: 28.0,
            velocity_y_mps: -24.0,
            attitude_rad: std::f64::consts::PI,
            fuel_kg: None,
            uphill_m: 0.0,
        },
        Condition {
            id: "overshoot",
            label: "synthetic overshoot with negative forward velocity",
            target_dx_m: -35.0,
            altitude_above_target_com_m: 320.0,
            velocity_x_mps: 36.0,
            velocity_y_mps: -18.0,
            attitude_rad: 0.0,
            fuel_kg: None,
            uphill_m: 0.0,
        },
    ];
    let source_pad = source
        .world
        .landing_pad(source_pad_id)
        .context("synthetic source pad is missing")?;
    let target_pad = source
        .world
        .landing_pad(target_pad_id)
        .context("synthetic target pad is missing")?;
    let source_x = source_pad.center_x_m;
    let target_x = target_pad.center_x_m;
    let target_y = target_pad.surface_y_m;
    let mut rows = Vec::with_capacity(conditions.len());
    for condition in conditions {
        let mut scenario = source.clone();
        scenario.id = format!("synthetic_{}", condition.id);
        scenario.name = format!("Synthetic nominal condition: {}", condition.id);
        scenario.description = condition.label.to_owned();
        scenario
            .tags
            .push("synthetic_nominal_characterization".into());
        scenario.metadata.insert(
            "nominal_characterization_population".into(),
            "synthetic_fresh_initial_state".into(),
        );
        scenario.metadata.insert(
            "nominal_characterization_condition".into(),
            condition.id.to_owned(),
        );
        let target = scenario
            .world
            .landing_pads
            .iter_mut()
            .find(|pad| pad.id == target_pad_id)
            .context("synthetic target pad disappeared")?;
        target.surface_y_m += condition.uphill_m;
        if condition.uphill_m != 0.0 {
            if (target_x - source_x).abs() <= 1.0e-9 {
                bail!("synthetic uphill pads must have distinct horizontal centers");
            }
            let pd_core::TerrainDefinition::Heightfield { points_m } = &mut scenario.world.terrain;
            for point in points_m {
                let fraction = ((point.x - source_x) / (target_x - source_x)).clamp(0.0, 1.0);
                point.y += condition.uphill_m * fraction;
            }
        }
        if let Some(fuel_kg) = condition.fuel_kg {
            scenario.vehicle.initial_fuel_kg = fuel_kg.min(scenario.vehicle.max_fuel_kg);
        }
        let target_com_y =
            target_y + condition.uphill_m + scenario.vehicle.geometry.touchdown_base_offset_m;
        scenario.initial_state.position_m = Vec2::new(
            target_x - condition.target_dx_m,
            target_com_y + condition.altitude_above_target_com_m,
        );
        scenario.initial_state.velocity_mps =
            Vec2::new(condition.velocity_x_mps, condition.velocity_y_mps);
        scenario.initial_state.attitude_rad = condition.attitude_rad;
        scenario.initial_state.angular_rate_radps = 0.0;
        scenario.validate().map_err(anyhow::Error::msg)?;
        rows.push((scenario, condition.label.to_owned()));
    }
    Ok(rows)
}

fn validate_corpus(corpus: &WaypointV2NominalCharacterizationCorpusV1) -> Result<()> {
    if corpus.schema_id != WAYPOINT_V2_NOMINAL_CHARACTERIZATION_CORPUS_SCHEMA_ID {
        bail!("unsupported corpus schema_id '{}'", corpus.schema_id);
    }
    if corpus.rows.len() != EXPECTED_RETAINED_ROWS {
        bail!(
            "expected {EXPECTED_RETAINED_ROWS} retained corpus rows, received {}",
            corpus.rows.len()
        );
    }
    let mut ids = BTreeSet::new();
    let mut counts = BTreeMap::from([
        ("clear_start", 0_usize),
        ("local_handoff", 0_usize),
        ("historical_capture", 0_usize),
    ]);
    for row in &corpus.rows {
        if row.id.trim().is_empty() || row.label.trim().is_empty() || !ids.insert(row.id.as_str()) {
            bail!(
                "corpus row has an empty or duplicate id/label: '{}'",
                row.id
            );
        }
        let Some(count) = counts.get_mut(row.population.as_str()) else {
            bail!(
                "{}: unsupported retained population '{}'",
                row.id,
                row.population
            );
        };
        *count += 1;
        row.scenario
            .validate()
            .map_err(anyhow::Error::msg)
            .with_context(|| format!("{}: invalid scenario", row.id))?;
        if row.scenario.world.landing_pad(&row.source_pad_id).is_none()
            || row.scenario.world.landing_pad(&row.target_pad_id).is_none()
            || row.source_pad_id == row.target_pad_id
        {
            bail!("{}: source/target pad binding is invalid", row.id);
        }
        if row.scenario.mission.goal.target_pad_id() != row.target_pad_id {
            bail!("{}: target pad differs from the scenario mission", row.id);
        }
        let interval = row.scenario.sim.control_interval_steps();
        if row.scenario.sim.physics_hz != 120
            || row.scenario.sim.controller_hz != 60
            || interval != HELD_TICKS
            || row.absolute_deadline_physics_step == 0
        {
            bail!("{}: original 120/60 cadence or deadline is invalid", row.id);
        }
        match (&row.expected_state, row.population.as_str()) {
            (None, "clear_start") => {
                if !row.prefix_updates.is_empty() {
                    bail!(
                        "{}: fresh clear-start row must not have prefix commands",
                        row.id
                    );
                }
            }
            (Some(expected), "local_handoff" | "historical_capture") => {
                let boundary = expected.physics_step;
                if boundary == 0
                    || !boundary.is_multiple_of(interval)
                    || row.absolute_deadline_physics_step <= boundary
                    || row.prefix_updates.len() as u64 != boundary / interval
                {
                    bail!(
                        "{}: expected snapshot or full prefix boundary is invalid",
                        row.id
                    );
                }
                if expected.sim_time_s != boundary as f64 / 120.0 {
                    bail!(
                        "{}: expected snapshot clock disagrees with its physics step",
                        row.id
                    );
                }
                validate_prefix(row, interval)?;
            }
            _ => bail!(
                "{}: expected_state is inconsistent with population '{}'",
                row.id,
                row.population
            ),
        }
        if row.absolute_deadline_physics_step
            > (row.scenario.sim.max_time_s * f64::from(row.scenario.sim.physics_hz)).floor() as u64
        {
            bail!("{}: original deadline exceeds the scenario horizon", row.id);
        }
    }
    if counts.get("clear_start") != Some(&EXPECTED_CLEAR_STARTS)
        || counts.get("local_handoff") != Some(&EXPECTED_LOCAL_HANDOFFS)
        || counts.get("historical_capture") != Some(&EXPECTED_HISTORICAL_CAPTURES)
    {
        bail!("retained corpus population counts do not match the frozen 8/27/12 interface");
    }
    let mut binding_paths = BTreeSet::new();
    for binding in &corpus.bindings {
        validate_relative_source_path(&binding.path)?;
        if !binding_paths.insert(binding.path.as_str()) {
            bail!("duplicate bound source path '{}'", binding.path);
        }
        if binding.sha256.len() != 64
            || !binding
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            bail!("{}: malformed lowercase SHA-256 binding", binding.path);
        }
    }
    if corpus.bindings.is_empty() {
        bail!("corpus has no source bindings");
    }
    Ok(())
}

fn validate_prefix(
    row: &WaypointV2NominalCharacterizationCorpusRowV1,
    interval: u64,
) -> Result<()> {
    for (index, update) in row.prefix_updates.iter().enumerate() {
        let expected_step = (index as u64)
            .checked_mul(interval)
            .context("prefix index overflow")?;
        if update.physics_step != expected_step {
            bail!(
                "{}: prefix has a gap, duplicate, or off-cadence command at index {index}",
                row.id
            );
        }
        validate_update(update).with_context(|| format!("{}: prefix update {index}", row.id))?;
    }
    Ok(())
}

fn validate_update(update: &FlightProgramUpdateV1) -> Result<()> {
    if update.phase.trim().is_empty()
        || !update.command.throttle_frac.is_finite()
        || !(0.0..=1.0).contains(&update.command.throttle_frac)
        || !update.command.target_attitude_rad.is_finite()
        || !(-std::f64::consts::PI..=std::f64::consts::PI)
            .contains(&update.command.target_attitude_rad)
    {
        bail!("command or phase is invalid");
    }
    Ok(())
}

fn validate_relative_source_path(value: &str) -> Result<()> {
    let path = Path::new(value);
    if value.trim().is_empty()
        || value.contains('\\')
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("unsafe corpus source path '{value}'");
    }
    Ok(())
}

fn verify_bindings(
    root: &Path,
    bindings: &[WaypointV2NominalCharacterizationBindingV1],
) -> Result<()> {
    let root =
        fs::canonicalize(root).context("canonicalize repository root for source bindings")?;
    for binding in bindings {
        validate_relative_source_path(&binding.path)?;
        let path = root.join(&binding.path);
        let canonical = fs::canonicalize(&path)
            .with_context(|| format!("resolve bound source {}", binding.path))?;
        if !canonical.starts_with(&root) {
            bail!("bound source escapes repository root: {}", binding.path);
        }
        let bytes =
            fs::read(&canonical).with_context(|| format!("read bound source {}", binding.path))?;
        let actual = sha256_bytes(&bytes)?;
        if actual != binding.sha256 {
            bail!(
                "SHA-256 mismatch for {}: expected {}, observed {}",
                binding.path,
                binding.sha256,
                actual
            );
        }
    }
    Ok(())
}

fn evaluate_retained_row(
    row: &WaypointV2NominalCharacterizationCorpusRowV1,
) -> Result<NominalCharacterizationRowEvidenceV1> {
    let context = RunContext::from_scenario(&row.scenario).map_err(anyhow::Error::msg)?;
    let mut live = SimulationState::new(&context)?;
    let mut expected_state_match = None;
    if let Some(expected) = &row.expected_state {
        let mut prefix_index = 0_usize;
        while live.physics_step < expected.physics_step {
            if live.is_terminal() {
                bail!(
                    "ordinary original-prefix replay became terminal at H{} before expected H{}",
                    live.physics_step,
                    expected.physics_step
                );
            }
            if live.physics_step.is_multiple_of(HELD_TICKS) {
                let update = row
                    .prefix_updates
                    .get(prefix_index)
                    .context("original command prefix ended before its H boundary")?;
                if update.physics_step != live.physics_step {
                    bail!(
                        "original command prefix lost cadence at H{}",
                        live.physics_step
                    );
                }
                live.set_command(update.command);
                prefix_index += 1;
            }
            // Retained handoffs are reconstructed through the same ordinary
            // contact/progress transition as the origin run, never from the
            // expected snapshot or the neutral witness replay seam.
            live.step(&context);
        }
        if prefix_index != row.prefix_updates.len() {
            bail!("original source replay did not consume its full prefix");
        }
        let actual = SimulationStateSnapshotV1::from_state(&live);
        let match_evidence = compare_full_snapshot(&actual, expected);
        if !match_evidence.matched {
            bail!(
                "fresh full-prefix replay disagrees with expected H{} snapshot (max float delta {})",
                expected.physics_step,
                match_evidence.maximum_float_delta
            );
        }
        expected_state_match = Some(match_evidence);
    }

    let incoming_state = SimulationStateSnapshotV1::from_state(&live);
    let mut ground_prep = if row.population == "clear_start" {
        Some(ground_prep(
            &context,
            &row.scenario,
            &row.source_pad_id,
            &row.target_pad_id,
            &mut live,
            row.absolute_deadline_physics_step,
        )?)
    } else {
        None
    };
    if let Some(prep) = &mut ground_prep {
        verify_ground_prep_replay(&context, prep)?;
    }
    let request = research_request(&row.scenario, &row.source_pad_id, &row.target_pad_id);
    let mut evidence = NominalCharacterizationRowEvidenceV1 {
        row_id: row.id.clone(),
        population: row.population.clone(),
        label: row.label.clone(),
        source_pad_id: row.source_pad_id.clone(),
        target_pad_id: row.target_pad_id.clone(),
        initial_physics_step: incoming_state.physics_step,
        initial_fuel_kg: incoming_state.fuel_kg,
        original_deadline_physics_step: row.absolute_deadline_physics_step,
        expected_state_match,
        prefix_update_count: row.prefix_updates.len(),
        ground_prep,
        seeds: Vec::new(),
        shortlist_seed_ids: Vec::new(),
        witnesses: Vec::new(),
        finite_miss_reasons: Vec::new(),
        integrity_errors: Vec::new(),
        status: "finite_miss".to_owned(),
        identity: String::new(),
    };
    if let Some(prep) = &evidence.ground_prep
        && let Some(error) = &prep.integrity_error
    {
        evidence.integrity_errors.push(error.clone());
        evidence.status = "integrity_error".into();
        evidence.identity = row_identity(&evidence)?;
        return Ok(evidence);
    }
    if let Some(prep) = &evidence.ground_prep
        && !prep.success
    {
        evidence.finite_miss_reasons.push(
            prep.reason
                .clone()
                .unwrap_or_else(|| "ground_prep_not_cleared".into()),
        );
        evidence.identity = row_identity(&evidence)?;
        return Ok(evidence);
    }

    let seed_specs = generate_seed_specs(&context, &live)?;
    if seed_specs.is_empty() {
        evidence
            .finite_miss_reasons
            .push("no_finite_virtual_arrival_seed".into());
    }
    let mut evaluated = Vec::with_capacity(seed_specs.len());
    for seed in seed_specs {
        let seed_evidence =
            evaluate_seed(&context, &live, row.absolute_deadline_physics_step, seed)?;
        if seed_evidence.status == "integrity_error" {
            evidence.integrity_errors.push(format!(
                "{}: {}",
                seed_evidence.seed_id,
                seed_evidence
                    .reason
                    .as_deref()
                    .unwrap_or("acquisition estimate integrity failure")
            ));
        }
        evaluated.push(seed_evidence);
    }
    if evaluated.len() > MAX_SEEDS {
        bail!("seed budget exceeded");
    }
    let mut ranked = evaluated
        .iter()
        .enumerate()
        .filter_map(|(index, seed)| {
            seed.selected_entry_index
                .map(|entry_index| (index, entry_index))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_seed, left_entry), (right_seed, right_entry)| {
        compare_rank(
            &evaluated[*left_seed],
            *left_entry,
            &evaluated[*right_seed],
            *right_entry,
        )
    });
    let shortlist = ranked
        .into_iter()
        .take(MAX_PHYSICAL_WITNESSES)
        .collect::<Vec<_>>();
    evidence.shortlist_seed_ids = shortlist
        .iter()
        .map(|(index, _)| evaluated[*index].seed_id.clone())
        .collect();
    for (seed_index, entry_index) in shortlist {
        let seed = &evaluated[seed_index];
        let entry = &seed.entry_screens[entry_index];
        match materialize_witness(
            &context,
            &request,
            &live,
            row.absolute_deadline_physics_step,
            seed,
            entry,
        ) {
            Ok(witness) => {
                if witness.status == "integrity_error" {
                    evidence.integrity_errors.push(format!(
                        "{}: independent replay or endpoint mismatch for {}",
                        row.id, witness.seed_id
                    ));
                }
                evidence.witnesses.push(witness);
            }
            Err(failure) => record_materialization_failure(&mut evidence, &seed.seed_id, failure),
        }
    }
    evidence.finite_miss_reasons.extend(
        evaluated
            .iter()
            .filter(|seed| seed.selected_entry_index.is_none())
            .map(|seed| {
                format!(
                    "{}: {}",
                    seed.seed_id,
                    seed.reason.as_deref().unwrap_or("no admissible entry")
                )
            }),
    );
    evidence.seeds = evaluated;
    evidence.status = if !evidence.integrity_errors.is_empty() {
        "integrity_error".to_owned()
    } else if evidence.witnesses.iter().any(|witness| {
        witness.independent_replay_passed
            && witness.target_plane_witness.as_ref().is_some_and(|plane| {
                plane.safe_by_existing_target_plane_mirror && plane.proposal_endpoint_contact_match
            })
    }) {
        "free_space_target_plane_witness".to_owned()
    } else {
        "finite_miss".to_owned()
    };
    evidence.identity = row_identity(&evidence)?;
    Ok(evidence)
}

fn integrity_error_row(
    row: &WaypointV2NominalCharacterizationCorpusRowV1,
    reason: String,
) -> Result<NominalCharacterizationRowEvidenceV1> {
    let expected_step = row
        .expected_state
        .as_ref()
        .map_or(0, |state| state.physics_step);
    let mut evidence = NominalCharacterizationRowEvidenceV1 {
        row_id: row.id.clone(),
        population: row.population.clone(),
        label: row.label.clone(),
        source_pad_id: row.source_pad_id.clone(),
        target_pad_id: row.target_pad_id.clone(),
        initial_physics_step: expected_step,
        initial_fuel_kg: row
            .expected_state
            .as_ref()
            .map_or(row.scenario.vehicle.initial_fuel_kg, |state| state.fuel_kg),
        original_deadline_physics_step: row.absolute_deadline_physics_step,
        expected_state_match: None,
        prefix_update_count: row.prefix_updates.len(),
        ground_prep: None,
        seeds: Vec::new(),
        shortlist_seed_ids: Vec::new(),
        witnesses: Vec::new(),
        finite_miss_reasons: Vec::new(),
        integrity_errors: vec![reason],
        status: "integrity_error".to_owned(),
        identity: String::new(),
    };
    evidence.identity = row_identity(&evidence)?;
    Ok(evidence)
}

fn row_identity(row: &NominalCharacterizationRowEvidenceV1) -> Result<String> {
    let mut canonical = row.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

fn compare_full_snapshot(
    actual: &SimulationStateSnapshotV1,
    expected: &SimulationStateSnapshotV1,
) -> SnapshotMatchEvidenceV1 {
    if actual == expected {
        return SnapshotMatchEvidenceV1 {
            matched: true,
            comparison_rule: "exact_full_snapshot".into(),
            maximum_float_delta: 0.0,
        };
    }
    let discrete_equal = actual.sim_time_s == expected.sim_time_s
        && actual.physics_step == expected.physics_step
        && actual.held_command == expected.held_command
        && actual.physical_outcome == expected.physical_outcome
        && actual.mission_outcome == expected.mission_outcome
        && actual.end_reason == expected.end_reason
        && actual.waypoint_sequence_passed == expected.waypoint_sequence_passed
        && actual.waypoint_sequence_first_failure_index
            == expected.waypoint_sequence_first_failure_index
        && actual.waypoint_handoff_window_index == expected.waypoint_handoff_window_index;
    let deltas = [
        (actual.position_m.x, expected.position_m.x),
        (actual.position_m.y, expected.position_m.y),
        (actual.velocity_mps.x, expected.velocity_mps.x),
        (actual.velocity_mps.y, expected.velocity_mps.y),
        (actual.attitude_rad, expected.attitude_rad),
        (actual.angular_rate_radps, expected.angular_rate_radps),
        (actual.fuel_kg, expected.fuel_kg),
        (
            actual.min_touchdown_clearance_m,
            expected.min_touchdown_clearance_m,
        ),
        (actual.min_hull_clearance_m, expected.min_hull_clearance_m),
        (actual.max_speed_mps, expected.max_speed_mps),
        (actual.max_abs_attitude_rad, expected.max_abs_attitude_rad),
        (
            actual.max_abs_angular_rate_radps,
            expected.max_abs_angular_rate_radps,
        ),
    ];
    let maximum_float_delta = deltas
        .iter()
        .map(|(left, right)| (left - right).abs())
        .fold(0.0_f64, f64::max);
    let matched = discrete_equal
        && deltas.iter().all(|(left, right)| {
            left.is_finite() && right.is_finite() && (left - right).abs() <= FLOAT_COMPARE_TOLERANCE
        });
    SnapshotMatchEvidenceV1 {
        matched,
        comparison_rule: if matched {
            "exact_clocks_enums_commands_and_float_tolerance_le_1e-9".into()
        } else {
            "full_snapshot_mismatch".into()
        },
        maximum_float_delta,
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SeedSpec {
    seed_id: String,
    kind: String,
    virtual_arrival_ticks: u64,
    burn_fraction: f64,
    natural_profile: bool,
    zero_acquisition: bool,
}

#[derive(Clone, Debug)]
struct AcquisitionPlan {
    thrust_acceleration_mps2: Vec2,
    turn_ticks: u64,
    burn_ticks: u64,
    predicted_end: KinematicStateV2,
    virtual_target_error_m: f64,
    estimated_fuel_kg: f64,
    upward_impulse_mps: f64,
    turn_updates: usize,
    target_attitude_rad: f64,
}

fn generate_seed_specs(context: &RunContext, live: &SimulationState) -> Result<Vec<SeedSpec>> {
    let target_com_y =
        context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m;
    let height = live.position_m.y - target_com_y;
    let gravity = context.world.gravity_mps2;
    let discriminant = live.velocity_mps.y.powi(2) + 2.0 * gravity * height;
    if !height.is_finite() || !discriminant.is_finite() || !gravity.is_finite() || gravity <= 0.0 {
        return Ok(Vec::new());
    }
    let natural_time = if discriminant >= 0.0 {
        (live.velocity_mps.y + discriminant.sqrt()) / gravity
    } else {
        f64::NAN
    };
    let natural_time = (natural_time.is_finite() && natural_time > 0.0).then_some(natural_time);
    let dt = context.sim.physics_dt_s();
    let lateral_distance = (context.target_pad.center_x_m - live.position_m.x).abs();
    let lateral_time = (2.0 * lateral_distance / gravity).sqrt();
    let baseline_time = natural_time.unwrap_or(0.0).max(lateral_time);
    if !baseline_time.is_finite() || baseline_time <= 0.0 {
        return Ok(Vec::new());
    }
    let baseline_ticks = even_ticks_ceil(baseline_time, dt)?;
    let zero_ticks = match natural_time {
        Some(time) => even_ticks_ceil(time, dt)?,
        None => baseline_ticks,
    };
    let mut arrivals = BTreeMap::<u64, bool>::new();
    if let Some(natural_time) = natural_time {
        let ticks = even_ticks_ceil(natural_time, dt)?;
        arrivals.insert(ticks, true);
    }
    arrivals
        .entry(baseline_ticks)
        .and_modify(|existing| *existing |= natural_time == Some(baseline_time))
        .or_insert(natural_time == Some(baseline_time));
    for multiplier in [1.25, 1.5] {
        let ticks = even_ticks_ceil(baseline_time * multiplier, dt)?;
        arrivals.entry(ticks).or_insert(false);
    }
    let mut seeds = vec![SeedSpec {
        seed_id: format!("zero_acquisition_n{zero_ticks}"),
        kind: "zero_acquisition".into(),
        virtual_arrival_ticks: zero_ticks,
        burn_fraction: 0.0,
        natural_profile: true,
        zero_acquisition: true,
    }];
    for (arrival_ticks, natural_profile) in arrivals {
        for burn_fraction in ACQUISITION_BURN_FRACTIONS {
            let fraction_code = (burn_fraction * 100.0).round() as u32;
            let kind = if natural_profile {
                "natural_profile"
            } else {
                "upward_shaping"
            };
            seeds.push(SeedSpec {
                seed_id: format!("{kind}_n{arrival_ticks}_b{fraction_code}"),
                kind: kind.into(),
                virtual_arrival_ticks: arrival_ticks,
                burn_fraction,
                natural_profile,
                zero_acquisition: false,
            });
        }
    }
    if seeds.len() > MAX_SEEDS {
        bail!(
            "bounded seed construction exceeded its proven 1 + 4-by-3 budget: {}",
            seeds.len()
        );
    }
    Ok(seeds)
}

fn even_ticks_ceil(seconds: f64, dt: f64) -> Result<u64> {
    if !seconds.is_finite() || seconds <= 0.0 || !dt.is_finite() || dt <= 0.0 {
        bail!("non-positive or non-finite duration");
    }
    let pairs = (seconds / (HELD_TICKS as f64 * dt)).ceil();
    if !pairs.is_finite() || pairs > (u64::MAX / HELD_TICKS) as f64 {
        bail!("duration exceeds representable held-pair count");
    }
    Ok((pairs as u64).saturating_mul(HELD_TICKS).max(HELD_TICKS))
}

fn even_ticks_nearest_fraction(ticks: u64, fraction: f64) -> u64 {
    let pairs = ((ticks as f64 * fraction) / HELD_TICKS as f64).round() as u64;
    pairs.saturating_mul(HELD_TICKS).min(ticks)
}

fn solve_constant_acquisition(
    incoming: KinematicStateV2,
    target: Vec2,
    arrival_ticks: u64,
    turn_ticks: u64,
    burn_ticks: u64,
    dt: f64,
    gravity_mps2: f64,
) -> Result<Vec2> {
    if arrival_ticks == 0 || burn_ticks == 0 || turn_ticks + burn_ticks > arrival_ticks {
        bail!("acquisition has no positive burn gain before virtual arrival");
    }
    let n = arrival_ticks as f64;
    let turn = turn_ticks as f64;
    let burn = burn_ticks as f64;
    let ballistic_end = incoming.position_m
        + incoming.velocity_mps * (n * dt)
        + Vec2::new(0.0, -gravity_mps2) * (dt * dt * n * (n + 1.0) / 2.0);
    let gain = dt * dt * burn * (n - turn - (burn - 1.0) / 2.0);
    if !gain.is_finite() || gain <= 0.0 {
        bail!("acquisition constant-acceleration gain is not positive");
    }
    let acceleration = (target - ballistic_end) * (1.0 / gain);
    if !acceleration.x.is_finite() || !acceleration.y.is_finite() {
        bail!("acquisition constant-acceleration demand is non-finite");
    }
    Ok(acceleration)
}

fn acquisition_end_prediction(
    incoming: KinematicStateV2,
    turn_ticks: u64,
    burn_ticks: u64,
    thrust_acceleration_mps2: Vec2,
    dt: f64,
    gravity_mps2: f64,
) -> KinematicStateV2 {
    let total = (turn_ticks + burn_ticks) as f64;
    let burn = burn_ticks as f64;
    KinematicStateV2 {
        position_m: incoming.position_m
            + incoming.velocity_mps * (total * dt)
            + Vec2::new(0.0, -gravity_mps2) * (dt * dt * total * (total + 1.0) / 2.0)
            + thrust_acceleration_mps2 * (dt * dt * burn * (burn + 1.0) / 2.0),
        velocity_mps: incoming.velocity_mps
            + Vec2::new(0.0, -gravity_mps2) * (total * dt)
            + thrust_acceleration_mps2 * (burn * dt),
    }
}

fn virtual_arrival_prediction(
    incoming: KinematicStateV2,
    arrival_ticks: u64,
    turn_ticks: u64,
    burn_ticks: u64,
    thrust_acceleration_mps2: Vec2,
    dt: f64,
    gravity_mps2: f64,
) -> KinematicStateV2 {
    let n = arrival_ticks as f64;
    let burn = burn_ticks as f64;
    let turn = turn_ticks as f64;
    let gain = dt * dt * burn * (n - turn - (burn - 1.0) / 2.0);
    KinematicStateV2 {
        position_m: incoming.position_m
            + incoming.velocity_mps * (n * dt)
            + Vec2::new(0.0, -gravity_mps2) * (dt * dt * n * (n + 1.0) / 2.0)
            + thrust_acceleration_mps2 * gain,
        velocity_mps: incoming.velocity_mps
            + Vec2::new(0.0, -gravity_mps2) * (n * dt)
            + thrust_acceleration_mps2 * (burn * dt),
    }
}

fn coast_prediction(
    start: KinematicStateV2,
    coast_ticks: u64,
    dt: f64,
    gravity_mps2: f64,
) -> KinematicStateV2 {
    let ticks = coast_ticks as f64;
    KinematicStateV2 {
        position_m: start.position_m
            + start.velocity_mps * (ticks * dt)
            + Vec2::new(0.0, -gravity_mps2) * (dt * dt * ticks * (ticks + 1.0) / 2.0),
        velocity_mps: start.velocity_mps + Vec2::new(0.0, -gravity_mps2) * (ticks * dt),
    }
}

fn target_kinematics(_context: &RunContext, live: &SimulationState) -> KinematicStateV2 {
    KinematicStateV2 {
        position_m: live.position_m,
        velocity_mps: live.velocity_mps,
    }
}

fn turn_ticks_for(
    context: &RunContext,
    current_attitude_rad: f64,
    target_acceleration_mps2: Vec2,
) -> Result<(u64, f64)> {
    if !current_attitude_rad.is_finite()
        || !target_acceleration_mps2.x.is_finite()
        || !target_acceleration_mps2.y.is_finite()
    {
        bail!("turn estimate contains non-finite state");
    }
    let target_attitude_rad = if target_acceleration_mps2.length() <= 1.0e-12 {
        current_attitude_rad
    } else {
        target_acceleration_mps2.x.atan2(target_acceleration_mps2.y)
    };
    let delta = shortest_angle_delta(current_attitude_rad, target_attitude_rad).abs();
    let max_angle_per_tick = context.vehicle.max_rotation_rate_radps * context.sim.physics_dt_s();
    if !max_angle_per_tick.is_finite() || max_angle_per_tick <= 0.0 {
        bail!("turn estimate has invalid vehicle rotation rate");
    }
    let raw_ticks = (delta / max_angle_per_tick).ceil();
    if !raw_ticks.is_finite() || raw_ticks > (u64::MAX - 1) as f64 {
        bail!("turn duration exceeds representable physics ticks");
    }
    let ticks = (raw_ticks as u64).div_ceil(HELD_TICKS) * HELD_TICKS;
    Ok((ticks, target_attitude_rad))
}

fn estimate_acquisition_fuel(
    context: &RunContext,
    incoming_fuel_kg: f64,
    acceleration_mps2: Vec2,
    burn_ticks: u64,
) -> Result<f64> {
    if burn_ticks == 0 || !burn_ticks.is_multiple_of(HELD_TICKS) {
        bail!("acquisition burn is not a complete held-command duration");
    }
    let policy = BodyAwareTerminalPolicyV1::default();
    let mut fuel = incoming_fuel_kg;
    for _ in 0..burn_ticks / HELD_TICKS {
        let mass = context.vehicle.dry_mass_kg + fuel;
        let command_throttle = paired_throttle(context, &policy, mass, acceleration_mps2.length())?;
        let applied = applied_throttle(command_throttle, context.vehicle.min_throttle_frac);
        let used = applied
            * context.vehicle.max_fuel_burn_kgps
            * context.sim.physics_dt_s()
            * HELD_TICKS as f64;
        if !used.is_finite() || used < 0.0 || fuel - used <= 0.0 {
            bail!("acquisition burn exceeds available fuel");
        }
        fuel -= used;
    }
    Ok(incoming_fuel_kg - fuel)
}

fn applied_throttle(command_throttle: f64, minimum: f64) -> f64 {
    if command_throttle <= 0.0 {
        0.0
    } else {
        minimum + command_throttle.clamp(0.0, 1.0) * (1.0 - minimum)
    }
}

fn make_acquisition_plan(
    context: &RunContext,
    live: &SimulationState,
    seed: &SeedSpec,
) -> Result<AcquisitionPlan> {
    if seed.zero_acquisition {
        let target = Vec2::new(
            context.target_pad.center_x_m,
            context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m,
        );
        let arrival = virtual_arrival_prediction(
            target_kinematics(context, live),
            seed.virtual_arrival_ticks,
            0,
            0,
            Vec2::new(0.0, 0.0),
            context.sim.physics_dt_s(),
            context.world.gravity_mps2,
        );
        return Ok(AcquisitionPlan {
            thrust_acceleration_mps2: Vec2::new(0.0, 0.0),
            turn_ticks: 0,
            burn_ticks: 0,
            predicted_end: target_kinematics(context, live),
            virtual_target_error_m: (arrival.position_m - target).length(),
            estimated_fuel_kg: 0.0,
            upward_impulse_mps: 0.0,
            turn_updates: 0,
            target_attitude_rad: live.attitude_rad,
        });
    }
    let dt = context.sim.physics_dt_s();
    let burn_ticks = even_ticks_nearest_fraction(seed.virtual_arrival_ticks, seed.burn_fraction);
    if burn_ticks == 0 {
        bail!("acquisition burn rounded to zero held pairs");
    }
    let target = Vec2::new(
        context.target_pad.center_x_m,
        context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m,
    );
    let incoming = target_kinematics(context, live);
    let mut turn_ticks = 0_u64;
    let mut updates = 0_usize;
    let mut solved = Vec2::new(0.0, 0.0);
    let mut target_attitude_rad = live.attitude_rad;
    let mut converged = false;
    for _ in 0..MAX_TURN_UPDATES {
        if turn_ticks + burn_ticks > seed.virtual_arrival_ticks {
            bail!("turn plus burn exceeds virtual acquisition time");
        }
        solved = solve_constant_acquisition(
            incoming,
            target,
            seed.virtual_arrival_ticks,
            turn_ticks,
            burn_ticks,
            dt,
            context.world.gravity_mps2,
        )?;
        if seed.natural_profile {
            // Natural-profile timing preserves the vertical arc exactly; any
            // rounded target-height discrepancy remains visible in the result.
            solved.y = 0.0;
        } else if solved.y < -1.0e-10 {
            bail!("upward-only acquisition would require downward vertical thrust");
        }
        let (next_turn, angle) = turn_ticks_for(context, live.attitude_rad, solved)?;
        target_attitude_rad = angle;
        updates += 1;
        if next_turn == turn_ticks {
            converged = true;
            break;
        }
        turn_ticks = next_turn;
    }
    if !converged {
        bail!("coast-turn consistency did not converge within three updates");
    }
    if turn_ticks + burn_ticks > seed.virtual_arrival_ticks {
        bail!("turn plus burn exceeds virtual acquisition time");
    }
    let incoming_max_accel = context.vehicle.max_thrust_n / live.mass_kg(context).max(1.0);
    if solved.length() > MAX_THRUST_FRACTION * incoming_max_accel {
        bail!("acquisition demand exceeds 0.925 of incoming max-thrust acceleration");
    }
    let estimated_fuel_kg = estimate_acquisition_fuel(context, live.fuel_kg, solved, burn_ticks)?;
    let predicted_end = acquisition_end_prediction(
        incoming,
        turn_ticks,
        burn_ticks,
        solved,
        dt,
        context.world.gravity_mps2,
    );
    let predicted_arrival = virtual_arrival_prediction(
        incoming,
        seed.virtual_arrival_ticks,
        turn_ticks,
        burn_ticks,
        solved,
        dt,
        context.world.gravity_mps2,
    );
    let virtual_target_error_m = (predicted_arrival.position_m - target).length();
    let upward_impulse_mps = solved.y.max(0.0) * burn_ticks as f64 * dt;
    Ok(AcquisitionPlan {
        thrust_acceleration_mps2: solved,
        turn_ticks,
        burn_ticks,
        predicted_end,
        virtual_target_error_m,
        estimated_fuel_kg,
        upward_impulse_mps,
        turn_updates: updates,
        target_attitude_rad,
    })
}

fn evaluate_seed(
    context: &RunContext,
    live: &SimulationState,
    deadline: u64,
    spec: SeedSpec,
) -> Result<NominalSeedEvidenceV1> {
    let mut evidence = NominalSeedEvidenceV1 {
        seed_id: spec.seed_id.clone(),
        kind: spec.kind.clone(),
        preserve_vertical: true,
        virtual_arrival_physics_ticks: spec.virtual_arrival_ticks,
        burn_fraction: spec.burn_fraction,
        turn_physics_ticks: 0,
        burn_physics_ticks: 0,
        remaining_virtual_physics_ticks: 0,
        requested_thrust_acceleration_mps2: None,
        predicted_acquisition_end: None,
        virtual_target_error_m: None,
        estimated_acquisition_fuel_kg: None,
        upward_impulse_mps: None,
        turn_updates: 0,
        status: "finite_miss".into(),
        reason: None,
        entry_screens: Vec::new(),
        selected_entry_index: None,
        estimated_fuel_kg: None,
        predicted_finish_physics_step: None,
        identity: String::new(),
    };
    let plan = match make_acquisition_plan(context, live, &spec) {
        Ok(plan) => plan,
        Err(error) => {
            evidence.reason = Some(format!("{error:#}"));
            if is_integrity_acquisition_plan_error(evidence.reason.as_deref().unwrap_or_default()) {
                evidence.status = "integrity_error".into();
            }
            evidence.identity = seed_identity(&evidence)?;
            return Ok(evidence);
        }
    };
    evidence.preserve_vertical = plan.thrust_acceleration_mps2.y.abs() <= 1.0e-12;
    evidence.turn_physics_ticks = plan.turn_ticks;
    evidence.burn_physics_ticks = plan.burn_ticks;
    evidence.remaining_virtual_physics_ticks = spec
        .virtual_arrival_ticks
        .saturating_sub(plan.turn_ticks + plan.burn_ticks);
    evidence.requested_thrust_acceleration_mps2 = Some(plan.thrust_acceleration_mps2);
    evidence.predicted_acquisition_end = Some(plan.predicted_end);
    evidence.virtual_target_error_m = Some(plan.virtual_target_error_m);
    evidence.estimated_acquisition_fuel_kg = Some(plan.estimated_fuel_kg);
    evidence.upward_impulse_mps = Some(plan.upward_impulse_mps);
    evidence.turn_updates = plan.turn_updates;
    let forward_direction = forward_direction(context, live.position_m.x, live.velocity_mps.x);
    if forward_direction * plan.predicted_end.velocity_mps.x < -1.0e-9 {
        evidence.reason = Some("predicted acquisition end has negative forward velocity".into());
        evidence.identity = seed_identity(&evidence)?;
        return Ok(evidence);
    }
    let mut screens = Vec::new();
    for (index, fraction) in ENTRY_COAST_FRACTIONS.iter().copied().enumerate() {
        let coast_ticks =
            even_ticks_nearest_fraction(evidence.remaining_virtual_physics_ticks, fraction);
        if screens
            .iter()
            .any(|screen: &NominalEntryScreenEvidenceV1| screen.coast_physics_ticks == coast_ticks)
        {
            continue;
        }
        screens.push(screen_entry(
            context,
            live,
            deadline,
            &plan,
            index,
            fraction,
            coast_ticks,
        )?);
    }
    if screens.len() > MAX_ENTRY_SCREENS {
        bail!("entry screen budget exceeded for {}", spec.seed_id);
    }
    let selected = screens
        .iter()
        .enumerate()
        .filter(|(_, screen)| screen.admissible)
        .min_by(|(left_index, left), (right_index, right)| {
            left.total_predicted_fuel_kg
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.total_predicted_fuel_kg.unwrap_or(f64::INFINITY))
                .then_with(|| {
                    left.predicted_finish_physics_step
                        .unwrap_or(u64::MAX)
                        .cmp(&right.predicted_finish_physics_step.unwrap_or(u64::MAX))
                })
                .then_with(|| left_index.cmp(right_index))
        })
        .map(|(index, _)| index);
    evidence.selected_entry_index = selected;
    if let Some(index) = selected {
        let screen = &screens[index];
        evidence.status = "shortlisted_estimate".into();
        evidence.estimated_fuel_kg = screen.total_predicted_fuel_kg;
        evidence.predicted_finish_physics_step = screen.predicted_finish_physics_step;
    } else {
        evidence.reason = Some(
            screens
                .iter()
                .filter_map(|screen| screen.reason.as_deref())
                .take(4)
                .collect::<Vec<_>>()
                .join("; "),
        );
    }
    evidence.entry_screens = screens;
    evidence.identity = seed_identity(&evidence)?;
    Ok(evidence)
}

fn seed_identity(seed: &NominalSeedEvidenceV1) -> Result<String> {
    let mut canonical = seed.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

fn forward_direction(context: &RunContext, x: f64, velocity_x: f64) -> f64 {
    let delta = context.target_pad.center_x_m - x;
    if delta.abs() > 1.0e-9 {
        delta.signum()
    } else if velocity_x.abs() > 1.0e-9 {
        velocity_x.signum()
    } else {
        1.0
    }
}

fn screen_entry(
    context: &RunContext,
    live: &SimulationState,
    deadline: u64,
    plan: &AcquisitionPlan,
    entry_index: usize,
    fraction: f64,
    coast_ticks: u64,
) -> Result<NominalEntryScreenEvidenceV1> {
    let dt = context.sim.physics_dt_s();
    let entry = coast_prediction(
        plan.predicted_end,
        coast_ticks,
        dt,
        context.world.gravity_mps2,
    );
    let mut screen = NominalEntryScreenEvidenceV1 {
        entry_index,
        coast_fraction: fraction,
        coast_physics_ticks: coast_ticks,
        predicted_entry: entry,
        predicted_entry_angle_rad: (-entry.velocity_mps.y).atan2(entry.velocity_mps.x.abs()),
        terminal_time_s: None,
        terminal_physics_ticks: None,
        coupled_thrust_bound_mps2: None,
        terminal_fuel_kg: None,
        total_predicted_fuel_kg: None,
        predicted_finish_physics_step: None,
        lateral_velocity_reversal_checked: false,
        admissible: false,
        reason: None,
    };
    let target_com_y =
        context.target_pad.surface_y_m + context.vehicle.geometry.touchdown_base_offset_m;
    let height = entry.position_m.y - target_com_y;
    let down_speed = -entry.velocity_mps.y;
    if !height.is_finite()
        || height <= TARGET_ENTRY_RESERVE_M
        || !entry.velocity_mps.y.is_finite()
        || entry.velocity_mps.y >= 0.0
    {
        screen.reason = Some("terminal entry is not descending above target COM + 5 m".into());
        return Ok(screen);
    }
    let target_down_speed = 0.5 * context.vehicle.safe_touchdown_normal_speed_mps;
    let denominator = down_speed + target_down_speed;
    if !denominator.is_finite() || denominator <= 0.0 {
        screen.reason = Some("terminal braking time has a non-positive denominator".into());
        return Ok(screen);
    }
    let terminal_time_s = (2.0 * height + (down_speed - target_down_speed) * dt) / denominator;
    if !terminal_time_s.is_finite() || terminal_time_s <= 0.0 {
        screen.reason = Some("protocol terminal braking time is not positive and finite".into());
        return Ok(screen);
    }
    let terminal_ticks = match even_ticks_ceil(terminal_time_s, dt) {
        Ok(ticks) => ticks,
        Err(error) => {
            screen.reason = Some(format!("{error:#}"));
            return Ok(screen);
        }
    };
    screen.terminal_time_s = Some(terminal_time_s);
    screen.terminal_physics_ticks = Some(terminal_ticks);
    let future_finish = live
        .physics_step
        .checked_add(plan.turn_ticks)
        .and_then(|tick| tick.checked_add(plan.burn_ticks))
        .and_then(|tick| tick.checked_add(coast_ticks))
        .and_then(|tick| tick.checked_add(terminal_ticks))
        .and_then(|tick| tick.checked_add(TERMINAL_EXTRA_TICKS));
    let Some(future_finish) = future_finish else {
        screen.reason = Some("predicted absolute finish clock overflow".into());
        return Ok(screen);
    };
    screen.predicted_finish_physics_step = Some(future_finish);
    if future_finish > deadline {
        screen.reason = Some("predicted terminal finish exceeds original absolute deadline".into());
        return Ok(screen);
    }

    let reference = match build_reference(
        context,
        &BodyAwareTerminalPolicyV1::default(),
        entry,
        terminal_ticks,
    ) {
        Ok(reference) => reference,
        Err(error) => return Err(error).context("terminal reference construction failed"),
    };
    if !terminal_forward_velocity_valid(context, &reference) {
        screen.lateral_velocity_reversal_checked = true;
        screen.reason = Some("terminal reference reverses forward horizontal velocity".into());
        return Ok(screen);
    }
    screen.lateral_velocity_reversal_checked = true;
    let bound = conservative_terminal_thrust_bound(context, &reference);
    screen.coupled_thrust_bound_mps2 = Some(bound);
    let estimated_terminal_start_fuel = (live.fuel_kg - plan.estimated_fuel_kg).max(0.0);
    let terminal_limit = MAX_THRUST_FRACTION * context.vehicle.max_thrust_n
        / (context.vehicle.dry_mass_kg + estimated_terminal_start_fuel).max(1.0);
    if !bound.is_finite() || bound > terminal_limit {
        screen.reason = Some(format!(
            "coupled terminal thrust bound {bound:.6} exceeds 0.925 incoming-mass limit {terminal_limit:.6}"
        ));
        return Ok(screen);
    }
    let first_terminal_thrust = paired_reference_thrust(context, &reference, 0);
    let first_angle = first_terminal_thrust.x.atan2(first_terminal_thrust.y);
    let acquisition_end_attitude = if plan.turn_ticks > 0 {
        plan.target_attitude_rad
    } else {
        live.attitude_rad
    };
    let align_ticks = match turn_ticks_for(
        context,
        acquisition_end_attitude,
        Vec2::new(first_angle.sin(), first_angle.cos()),
    ) {
        Ok((ticks, _)) => ticks,
        Err(error) => {
            screen.reason = Some(format!("terminal first-thrust alignment: {error:#}"));
            return Ok(screen);
        }
    };
    if coast_ticks < align_ticks {
        screen.reason = Some(format!(
            "terminal entry has {coast_ticks} coasting ticks for {align_ticks} ticks of first-thrust alignment"
        ));
        return Ok(screen);
    }
    let terminal_start_fuel = live.fuel_kg - plan.estimated_fuel_kg;
    let terminal_fuel = match estimate_terminal_fuel(context, &reference, terminal_start_fuel) {
        Ok(fuel) => fuel,
        Err(error) => {
            screen.reason = Some(format!("terminal throttle/fuel screen: {error:#}"));
            return Ok(screen);
        }
    };
    screen.terminal_fuel_kg = Some(terminal_fuel);
    screen.total_predicted_fuel_kg = Some(plan.estimated_fuel_kg + terminal_fuel);
    screen.admissible = true;
    Ok(screen)
}

fn paired_reference_thrust(
    context: &RunContext,
    reference: &BodyAwareTerminalReferenceV1,
    tick: u64,
) -> Vec2 {
    (reference.thrust(context, tick) + reference.thrust(context, tick + 1)) * 0.5
}

fn conservative_terminal_thrust_bound(
    context: &RunContext,
    reference: &BodyAwareTerminalReferenceV1,
) -> f64 {
    let [c0, c1, c2] = reference.horizontal_coefficients_mps2;
    let mut horizontal_samples = vec![c0, c0 + c1 + c2];
    if c2.abs() > 1.0e-14 {
        let vertex = -c1 / (2.0 * c2);
        if (0.0..=1.0).contains(&vertex) {
            horizontal_samples.push(c0 + c1 * vertex + c2 * vertex * vertex);
        }
    }
    let maximum_horizontal = horizontal_samples
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let first_vertical = reference.initial_vertical_acceleration_mps2 + context.world.gravity_mps2;
    let last_vertical = reference.initial_vertical_acceleration_mps2
        + reference.vertical_acceleration_delta_mps2 * (reference.physics_ticks - 1) as f64
        + context.world.gravity_mps2;
    maximum_horizontal.hypot(first_vertical.abs().max(last_vertical.abs()))
}

fn terminal_forward_velocity_valid(
    context: &RunContext,
    reference: &BodyAwareTerminalReferenceV1,
) -> bool {
    let direction = forward_direction(
        context,
        reference.start.position_m.x,
        reference.start.velocity_mps.x,
    );
    let [c0, c1, c2] = reference.horizontal_coefficients_mps2;
    let mut sample_ticks = BTreeSet::from([0_u64, reference.physics_ticks]);
    for root in quadratic_roots(c2, c1, c0) {
        if (0.0..=1.0).contains(&root) {
            let tick = (root * (reference.physics_ticks - 1) as f64).round() as i64;
            for delta in -2_i64..=2 {
                let sample = tick + delta;
                if sample >= 0 && sample <= reference.physics_ticks as i64 {
                    sample_ticks.insert(sample as u64);
                }
            }
        }
    }
    sample_ticks.into_iter().all(|tick| {
        let state = reference.state_at(context, tick);
        state.velocity_mps.x.is_finite() && direction * state.velocity_mps.x >= -1.0e-9
    })
}

fn quadratic_roots(a: f64, b: f64, c: f64) -> Vec<f64> {
    if a.abs() <= 1.0e-14 {
        if b.abs() <= 1.0e-14 {
            Vec::new()
        } else {
            vec![-c / b]
        }
    } else {
        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 || !discriminant.is_finite() {
            Vec::new()
        } else {
            let root = discriminant.sqrt();
            vec![(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)]
        }
    }
}

fn estimate_terminal_fuel(
    context: &RunContext,
    reference: &BodyAwareTerminalReferenceV1,
    initial_fuel_kg: f64,
) -> Result<f64> {
    if !initial_fuel_kg.is_finite() || initial_fuel_kg <= 0.0 {
        bail!("terminal starts without positive fuel");
    }
    let policy = BodyAwareTerminalPolicyV1::default();
    let mut fuel = initial_fuel_kg;
    let end_tick = reference.physics_ticks + TERMINAL_EXTRA_TICKS;
    for tick in (0..end_tick).step_by(HELD_TICKS as usize) {
        let thrust = paired_reference_thrust(context, reference, tick);
        let mass = context.vehicle.dry_mass_kg + fuel;
        let throttle = paired_throttle(context, &policy, mass, thrust.length())?;
        let applied = applied_throttle(throttle, context.vehicle.min_throttle_frac);
        let used = applied
            * context.vehicle.max_fuel_burn_kgps
            * context.sim.physics_dt_s()
            * HELD_TICKS as f64;
        if !used.is_finite() || used < 0.0 || fuel - used <= 0.0 {
            bail!("terminal reference exhausts estimated fuel");
        }
        fuel -= used;
    }
    Ok(initial_fuel_kg - fuel)
}

fn compare_rank(
    left_seed: &NominalSeedEvidenceV1,
    left_entry_index: usize,
    right_seed: &NominalSeedEvidenceV1,
    right_entry_index: usize,
) -> std::cmp::Ordering {
    let left = &left_seed.entry_screens[left_entry_index];
    let right = &right_seed.entry_screens[right_entry_index];
    left_seed
        .preserve_vertical
        .cmp(&right_seed.preserve_vertical)
        .reverse()
        .then_with(|| {
            left_seed
                .upward_impulse_mps
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right_seed.upward_impulse_mps.unwrap_or(f64::INFINITY))
        })
        .then_with(|| {
            left.total_predicted_fuel_kg
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.total_predicted_fuel_kg.unwrap_or(f64::INFINITY))
        })
        .then_with(|| {
            left.predicted_finish_physics_step
                .unwrap_or(u64::MAX)
                .cmp(&right.predicted_finish_physics_step.unwrap_or(u64::MAX))
        })
        .then_with(|| left_seed.seed_id.cmp(&right_seed.seed_id))
        .then_with(|| left_entry_index.cmp(&right_entry_index))
}

#[derive(Clone, Debug)]
struct AcquisitionExecution {
    end_state: SimulationState,
    turn_updates: Vec<FlightProgramUpdateV1>,
    burn_updates: Vec<FlightProgramUpdateV1>,
}

struct NeutralReplay {
    final_state: SimulationState,
    acquisition_end_match: bool,
    terminal_entry: Option<SimulationStateSnapshotV1>,
    target_plane: Option<(u64, bool)>,
    peak_com_height_m: f64,
}

fn materialize_witness(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
    incoming: &SimulationState,
    deadline: u64,
    seed: &NominalSeedEvidenceV1,
    entry: &NominalEntryScreenEvidenceV1,
) -> std::result::Result<NominalWitnessEvidenceV1, WitnessMaterializationFailure> {
    let realized = realize_free_space_witness(context, incoming, deadline, seed, entry)?;
    let mut witness = realized.witness;
    let terrain = run_real_terrain_audit(
        context,
        request,
        incoming,
        &witness.commands,
        realized.end_state.physics_step,
        witness
            .target_plane_witness
            .as_ref()
            .map(|plane| plane.physics_step),
    )
    .map_err(|error| WitnessMaterializationFailure::Integrity(format!("{error:#}")))?;
    if let Some(plane) = &mut witness.target_plane_witness {
        plane.actual_landing_claimed = terrain.landed_on_target;
    }
    witness.terrain_audit = Some(terrain);
    witness.identity = witness_identity_for_characterization(&witness)
        .map_err(|error| WitnessMaterializationFailure::Integrity(format!("{error:#}")))?;
    Ok(witness)
}

/// Shared command realization has no ordinary terrain audit or corpus inputs.
/// The research wrapper attaches its unchanged reporting after this seam.
struct FreeSpaceWitness {
    witness: NominalWitnessEvidenceV1,
    end_state: AirborneFlightStateV1,
    peak_com_height_m: f64,
}

fn realize_free_space_witness(
    context: &RunContext,
    incoming: &SimulationState,
    deadline: u64,
    seed: &NominalSeedEvidenceV1,
    entry: &NominalEntryScreenEvidenceV1,
) -> std::result::Result<FreeSpaceWitness, WitnessMaterializationFailure> {
    let spec = SeedSpec {
        seed_id: seed.seed_id.clone(),
        kind: seed.kind.clone(),
        virtual_arrival_ticks: seed.virtual_arrival_physics_ticks,
        burn_fraction: seed.burn_fraction,
        natural_profile: seed.kind == "natural_profile",
        zero_acquisition: seed.kind == "zero_acquisition",
    };
    let plan = make_acquisition_plan(context, incoming, &spec).map_err(|error| {
        WitnessMaterializationFailure::Integrity(format!(
            "shortlisted seed no longer reconstructs its acquisition estimate: {error:#}"
        ))
    })?;
    if seed.requested_thrust_acceleration_mps2 != Some(plan.thrust_acceleration_mps2)
        || seed.predicted_acquisition_end != Some(plan.predicted_end)
        || seed.turn_physics_ticks != plan.turn_ticks
        || seed.burn_physics_ticks != plan.burn_ticks
    {
        return Err(WitnessMaterializationFailure::Integrity(
            "shortlist estimate does not bind to its recomputed acquisition plan".into(),
        ));
    }
    let acquisition = run_acquisition(context, incoming, &plan)?;
    let acquisition_start = SimulationStateSnapshotV1::from_state(incoming);
    let actual_acquisition_end = SimulationStateSnapshotV1::from_state(&acquisition.end_state);
    let acquisition_position_error_m = distance(
        plan.predicted_end.position_m,
        acquisition.end_state.position_m,
    );
    let acquisition_velocity_error_mps = distance(
        plan.predicted_end.velocity_mps,
        acquisition.end_state.velocity_mps,
    );
    let acquisition_fuel_error_kg =
        acquisition.end_state.fuel_kg - (incoming.fuel_kg - plan.estimated_fuel_kg);
    let turn_identity = stable_digest(&acquisition.turn_updates).map_err(|error| {
        WitnessMaterializationFailure::Integrity(format!(
            "hash acquisition turn commands: {error:#}"
        ))
    })?;
    let acquisition_identity = stable_digest(
        &acquisition
            .turn_updates
            .iter()
            .chain(&acquisition.burn_updates)
            .cloned()
            .collect::<Vec<_>>(),
    )
    .map_err(|error| {
        WitnessMaterializationFailure::Integrity(format!("hash acquisition commands: {error:#}"))
    })?;
    let acquisition_updates = acquisition
        .turn_updates
        .iter()
        .chain(&acquisition.burn_updates)
        .cloned()
        .collect::<Vec<_>>();
    let entry_ticks = entry.terminal_physics_ticks.ok_or_else(|| {
        WitnessMaterializationFailure::Integrity(
            "shortlisted entry is missing terminal duration".into(),
        )
    })?;
    let terminal_start_step = acquisition
        .end_state
        .physics_step
        .checked_add(entry.coast_physics_ticks)
        .ok_or_else(|| {
            WitnessMaterializationFailure::Integrity(
                "terminal-entry boundary overflows original clock".into(),
            )
        })?;

    let proposal = match super::airborne_direct::candidate(
        context,
        &acquisition.end_state,
        entry.coast_physics_ticks,
        entry_ticks,
        deadline,
    ) {
        Ok(proposal) => proposal,
        Err(error) => {
            let candidate_error = match classify_candidate_failure(format!("{error:#}")) {
                WitnessMaterializationFailure::FiniteMiss(reason) => reason,
                failure @ WitnessMaterializationFailure::Integrity(_) => return Err(failure),
            };
            let mut witness = NominalWitnessEvidenceV1 {
                seed_id: seed.seed_id.clone(),
                entry_index: entry.entry_index,
                status: "finite_backend_rejection".into(),
                reason: Some(candidate_error),
                acquisition_start,
                predicted_acquisition_end: plan.predicted_end,
                actual_acquisition_end: actual_acquisition_end.clone(),
                acquisition_position_error_m,
                acquisition_velocity_error_mps,
                acquisition_fuel_error_kg,
                acquisition_turn_command_identity: turn_identity,
                acquisition_command_identity: acquisition_identity.clone(),
                actual_terminal_entry: None,
                actual_terminal_entry_angle_rad: None,
                actual_entry_position_error_m: None,
                actual_entry_velocity_error_mps: None,
                commands: acquisition_updates.clone(),
                command_identity: acquisition_identity,
                independent_replay_passed: false,
                endpoint_state_agreement: false,
                target_plane_witness: None,
                terrain_audit: None,
                identity: String::new(),
            };
            let replay = replay_neutral(
                context,
                incoming,
                &acquisition_updates,
                acquisition.end_state.physics_step,
                acquisition.end_state.physics_step,
                acquisition.end_state.physics_step,
                &actual_acquisition_end,
            )
            .map_err(|error| WitnessMaterializationFailure::Integrity(format!("{error:#}")))?;
            witness.independent_replay_passed = replay.acquisition_end_match;
            if !replay.acquisition_end_match {
                witness.status = "integrity_error".into();
                witness.reason = Some("independent acquisition replay disagrees".into());
            }
            return Ok(FreeSpaceWitness {
                witness,
                end_state: AirborneFlightStateV1::from_live(&replay.final_state),
                peak_com_height_m: replay.peak_com_height_m,
            });
        }
    };
    let mut commands = acquisition_updates.clone();
    if proposal
        .updates
        .first()
        .is_some_and(|update| update.physics_step != acquisition.end_state.physics_step)
    {
        return Err(WitnessMaterializationFailure::Integrity(
            "terminal candidate does not continue the actual acquisition clock".into(),
        ));
    }
    commands.extend(proposal.updates.iter().cloned());
    validate_command_schedule(
        incoming.physics_step,
        proposal.planned_end_physics_step,
        &commands,
    )
    .map_err(|error| WitnessMaterializationFailure::Integrity(format!("{error:#}")))?;
    if !commands.iter().all(|update| {
        update.command.throttle_frac.is_finite()
            && update.command.target_attitude_rad.is_finite()
            && update.command == update.command.clamped()
    }) {
        return Err(WitnessMaterializationFailure::Integrity(
            "realized command list contains a nonfinite or unclamped command".into(),
        ));
    }
    let command_identity = stable_digest(&commands).map_err(|error| {
        WitnessMaterializationFailure::Integrity(format!("hash realized commands: {error:#}"))
    })?;
    let replay = replay_neutral(
        context,
        incoming,
        &commands,
        acquisition.end_state.physics_step,
        terminal_start_step,
        proposal.planned_end_physics_step,
        &actual_acquisition_end,
    )
    .map_err(|error| WitnessMaterializationFailure::Integrity(format!("{error:#}")))?;
    let endpoint_state_agreement = AirborneFlightStateV1::from_live(&replay.final_state)
        == proposal.end_state
        && replay.final_state.physics_step == proposal.planned_end_physics_step;
    let entry_snapshot = replay.terminal_entry.clone();
    let mut status = "free_space_target_plane_witness".to_owned();
    let mut reason = None;
    if !replay.acquisition_end_match || !endpoint_state_agreement {
        status = "integrity_error".into();
        reason = Some(if !replay.acquisition_end_match {
            "independent replay disagrees at actual acquisition endpoint".into()
        } else {
            "independent replay disagrees at commanded proposal endpoint".into()
        });
    } else if replay
        .target_plane
        .is_none_or(|(tick, safe)| !safe || tick != proposal.planned_end_physics_step)
    {
        status = "finite_miss".into();
        reason = Some(
            "first free-space target-plane contact is unsafe or differs from the proposal endpoint"
                .into(),
        );
    }
    let target_plane_witness =
        replay
            .target_plane
            .map(|(physics_step, safe)| TargetPlaneWitnessEvidenceV1 {
                physics_step,
                safe_by_existing_target_plane_mirror: safe,
                proposal_endpoint_contact_match: physics_step == proposal.planned_end_physics_step,
                free_space_endpoint_match: endpoint_state_agreement,
                actual_landing_claimed: false,
            });
    let (actual_entry_angle_rad, actual_entry_position_error_m, actual_entry_velocity_error_mps) =
        if let Some(snapshot) = &entry_snapshot {
            (
                Some((-snapshot.velocity_mps.y).atan2(snapshot.velocity_mps.x.abs())),
                Some(distance(
                    entry.predicted_entry.position_m,
                    snapshot.position_m,
                )),
                Some(distance(
                    entry.predicted_entry.velocity_mps,
                    snapshot.velocity_mps,
                )),
            )
        } else {
            (None, None, None)
        };
    let witness = NominalWitnessEvidenceV1 {
        seed_id: seed.seed_id.clone(),
        entry_index: entry.entry_index,
        status,
        reason,
        acquisition_start,
        predicted_acquisition_end: plan.predicted_end,
        actual_acquisition_end,
        acquisition_position_error_m,
        acquisition_velocity_error_mps,
        acquisition_fuel_error_kg,
        acquisition_turn_command_identity: turn_identity,
        acquisition_command_identity: acquisition_identity,
        actual_terminal_entry: entry_snapshot,
        actual_terminal_entry_angle_rad: actual_entry_angle_rad,
        actual_entry_position_error_m,
        actual_entry_velocity_error_mps,
        commands,
        command_identity,
        independent_replay_passed: replay.acquisition_end_match && endpoint_state_agreement,
        endpoint_state_agreement,
        target_plane_witness,
        terrain_audit: None,
        identity: String::new(),
    };
    Ok(FreeSpaceWitness {
        witness,
        end_state: AirborneFlightStateV1::from_live(&replay.final_state),
        peak_com_height_m: replay.peak_com_height_m,
    })
}

fn run_acquisition(
    context: &RunContext,
    incoming: &SimulationState,
    plan: &AcquisitionPlan,
) -> std::result::Result<AcquisitionExecution, WitnessMaterializationFailure> {
    if !incoming.physics_step.is_multiple_of(HELD_TICKS) {
        return Err(WitnessMaterializationFailure::Integrity(
            "acquisition input is off the original held-command boundary".into(),
        ));
    }
    let mut state = incoming.clone();
    let mut turn_updates = Vec::new();
    let mut burn_updates = Vec::new();
    for _ in 0..plan.turn_ticks {
        if state.physics_step.is_multiple_of(HELD_TICKS) {
            let command = Command {
                throttle_frac: 0.0,
                target_attitude_rad: plan.target_attitude_rad,
            };
            turn_updates.push(FlightProgramUpdateV1 {
                physics_step: state.physics_step,
                phase: "nominal_acquisition_turn".into(),
                command,
            });
            state.set_command(command);
        }
        state.step_physics_and_classify_contact(context);
        if !finite_live_state(&state) {
            return Err(WitnessMaterializationFailure::Integrity(
                "nonfinite live state during acquisition turn".into(),
            ));
        }
    }
    let policy = BodyAwareTerminalPolicyV1::default();
    for _ in 0..plan.burn_ticks / HELD_TICKS {
        if !state.physics_step.is_multiple_of(HELD_TICKS) {
            return Err(WitnessMaterializationFailure::Integrity(
                "acquisition burn lost the original held-command cadence".into(),
            ));
        }
        let throttle = paired_throttle(
            context,
            &policy,
            state.mass_kg(context),
            plan.thrust_acceleration_mps2.length(),
        )
        .map_err(|error| {
            WitnessMaterializationFailure::FiniteMiss(format!(
                "paired acquisition throttle rejected: {error:#}"
            ))
        })?;
        let command = Command {
            throttle_frac: throttle,
            target_attitude_rad: plan.target_attitude_rad,
        };
        burn_updates.push(FlightProgramUpdateV1 {
            physics_step: state.physics_step,
            phase: "nominal_acquisition".into(),
            command,
        });
        state.set_command(command);
        for _ in 0..HELD_TICKS {
            state.step_physics_and_classify_contact(context);
            if !finite_live_state(&state) {
                return Err(WitnessMaterializationFailure::Integrity(
                    "nonfinite live state during acquisition burn".into(),
                ));
            }
        }
    }
    if state.physics_step != incoming.physics_step + plan.turn_ticks + plan.burn_ticks {
        return Err(WitnessMaterializationFailure::Integrity(
            "acquisition ended at an unexpected absolute physics step".into(),
        ));
    }
    Ok(AcquisitionExecution {
        end_state: state,
        turn_updates,
        burn_updates,
    })
}

fn validate_command_schedule(
    start: u64,
    end: u64,
    updates: &[FlightProgramUpdateV1],
) -> Result<()> {
    let covered_pairs = end.saturating_sub(start).div_ceil(HELD_TICKS);
    if start > end || !start.is_multiple_of(HELD_TICKS) || updates.len() as u64 != covered_pairs {
        bail!("realized command list does not cover its complete held-command interval");
    }
    for (index, update) in updates.iter().enumerate() {
        let expected = start
            .checked_add(HELD_TICKS * index as u64)
            .context("realized command clock overflow")?;
        if update.physics_step != expected {
            bail!("realized command list has a gap or duplicate at H{expected}");
        }
        validate_update(update)?;
    }
    Ok(())
}

fn replay_neutral(
    context: &RunContext,
    incoming: &SimulationState,
    updates: &[FlightProgramUpdateV1],
    acquisition_end_step: u64,
    terminal_entry_step: u64,
    planned_end_step: u64,
    expected_acquisition_end: &SimulationStateSnapshotV1,
) -> Result<NeutralReplay> {
    let end = planned_end_step;
    validate_command_schedule(incoming.physics_step, end, updates)?;
    if acquisition_end_step < incoming.physics_step || acquisition_end_step > end {
        bail!("independent replay acquisition boundary lies outside its interval");
    }
    let mut state = incoming.clone();
    let mut command_index = 0_usize;
    let mut acquisition_end_match = if acquisition_end_step == incoming.physics_step {
        compare_full_snapshot(
            &SimulationStateSnapshotV1::from_state(&state),
            expected_acquisition_end,
        )
        .matched
    } else {
        false
    };
    let mut terminal_entry = (terminal_entry_step == incoming.physics_step)
        .then(|| SimulationStateSnapshotV1::from_state(&state));
    let mut target_plane = None;
    let mut peak_com_height_m = state.position_m.y;
    while state.physics_step < end {
        if state.physics_step.is_multiple_of(HELD_TICKS) {
            let update = updates
                .get(command_index)
                .context("independent replay command ended before endpoint")?;
            if update.physics_step != state.physics_step {
                bail!("independent replay command gap at H{}", state.physics_step);
            }
            state.set_command(update.command);
            command_index += 1;
        }
        state.step_physics_and_classify_contact(context);
        peak_com_height_m = peak_com_height_m.max(state.position_m.y);
        if !finite_live_state(&state) {
            bail!(
                "independent replay produced nonfinite state at H{}",
                state.physics_step
            );
        }
        if target_plane.is_none()
            && let Some(safe) = super::airborne_direct::target_plane_contact(context, &state)
        {
            target_plane = Some((state.physics_step, safe));
        }
        if state.physics_step == acquisition_end_step {
            acquisition_end_match = compare_full_snapshot(
                &SimulationStateSnapshotV1::from_state(&state),
                expected_acquisition_end,
            )
            .matched;
        }
        if state.physics_step == terminal_entry_step {
            terminal_entry = Some(SimulationStateSnapshotV1::from_state(&state));
        }
    }
    if command_index != updates.len() || state.physics_step != end {
        bail!("independent replay failed to consume the complete realized schedule");
    }
    Ok(NeutralReplay {
        final_state: state,
        acquisition_end_match,
        terminal_entry,
        target_plane,
        peak_com_height_m,
    })
}

fn run_real_terrain_audit(
    context: &RunContext,
    request: &WaypointDirectNominalDirectGenerationRequest,
    incoming: &SimulationState,
    updates: &[FlightProgramUpdateV1],
    planned_end_step: u64,
    target_plane_step: Option<u64>,
) -> Result<RealTerrainAuditEvidenceV1> {
    let end = planned_end_step;
    validate_command_schedule(incoming.physics_step, end, updates)?;
    let mut state = incoming.clone();
    let mut neutral = incoming.clone();
    let mut ordinary_neutral_prefix_parity = true;
    let mut command_index = 0_usize;
    let mut first_contact = None;
    let mut first_contact_step = None;
    let mut phase_map = BTreeMap::<String, BodyReservePhaseEvidenceV1>::new();
    let mut stopped_on_contact = false;
    while state.physics_step < end && !state.is_terminal() {
        let mut active_phase = "unknown".to_owned();
        if state.physics_step.is_multiple_of(HELD_TICKS) {
            let update = updates
                .get(command_index)
                .context("terrain audit command ended before endpoint")?;
            if update.physics_step != state.physics_step {
                bail!("terrain audit command gap at H{}", state.physics_step);
            }
            state.set_command(update.command);
            neutral.set_command(update.command);
            active_phase = update.phase.clone();
            command_index += 1;
        } else if let Some(update) = command_index.checked_sub(1).and_then(|i| updates.get(i)) {
            active_phase = update.phase.clone();
        }
        let classification = neutral.step_physics_and_classify_contact(context);
        let report = state.step_with_contact_report(context);
        if let Some(contact) = report.incoming_contact.as_ref() {
            ordinary_neutral_prefix_parity &= contact.classification == classification
                && contact.state == SimulationStateSnapshotV1::from_state(&neutral);
            // Ordinary landing handling can snap position and zero velocity.
            // Audit the authoritative incoming state, not that adapted state.
            first_contact = Some(contact_audit(context, &neutral, &classification));
            first_contact_step = Some(state.physics_step);
            stopped_on_contact = true;
            break;
        }
        ordinary_neutral_prefix_parity &= classification == ContactClassification::None
            && SimulationStateSnapshotV1::from_state(&neutral)
                == SimulationStateSnapshotV1::from_state(&state);
        let phase =
            phase_map
                .entry(active_phase.clone())
                .or_insert_with(|| BodyReservePhaseEvidenceV1 {
                    phase: active_phase.clone(),
                    minimum_reserve_margin_m: None,
                    first_reserve_violation_physics_step: None,
                    first_query_error: None,
                });
        match nominal_body_reserve_query(context, request, &state, &active_phase, false) {
            Ok((clearance, required)) => {
                let margin = clearance - required;
                phase.minimum_reserve_margin_m = Some(
                    phase
                        .minimum_reserve_margin_m
                        .map_or(margin, |old| old.min(margin)),
                );
                if margin < 0.0 && phase.first_reserve_violation_physics_step.is_none() {
                    phase.first_reserve_violation_physics_step = Some(state.physics_step);
                }
            }
            Err(error) => {
                if phase.first_query_error.is_none() {
                    phase.first_query_error = Some(format!("{error:#}"));
                }
            }
        }
    }
    if command_index < updates.len() && !state.is_terminal() {
        bail!("terrain audit did not consume its complete realized command schedule");
    }
    if !ordinary_neutral_prefix_parity {
        bail!("real-terrain ordinary/neutral consumed-prefix replay disagreement");
    }
    let landed_on_target = state.physical_outcome == PhysicalOutcome::LandedOnTarget
        && state.mission_outcome == MissionOutcome::Success
        && state.end_reason == EndReason::TouchdownOnTarget;
    let target_before_contact = target_plane_step
        .is_some_and(|plane_step| first_contact_step.is_none_or(|contact| plane_step <= contact));
    Ok(RealTerrainAuditEvidenceV1 {
        ordinary_neutral_prefix_parity,
        first_contact,
        first_contact_physics_step: first_contact_step,
        final_state: SimulationStateSnapshotV1::from_state(&state),
        stopped_on_contact,
        landed_on_target,
        target_plane_witness_reached_before_terrain_contact: target_before_contact,
        terrain_contact_blocks_landing_claim: stopped_on_contact && !landed_on_target,
        minimum_hull_clearance_m: state
            .min_hull_clearance_m
            .is_finite()
            .then_some(state.min_hull_clearance_m),
        reserve_phases: phase_map.into_values().collect(),
    })
}

fn finite_live_state(state: &SimulationState) -> bool {
    [
        state.sim_time_s,
        state.position_m.x,
        state.position_m.y,
        state.velocity_mps.x,
        state.velocity_mps.y,
        state.attitude_rad,
        state.angular_rate_radps,
        state.fuel_kg,
        state.held_command.throttle_frac,
        state.held_command.target_attitude_rad,
    ]
    .iter()
    .all(|value| value.is_finite())
}

fn research_request(
    scenario: &ScenarioSpec,
    source_pad_id: &str,
    target_pad_id: &str,
) -> WaypointDirectNominalDirectGenerationRequest {
    WaypointDirectNominalDirectGenerationRequest {
        scenario: scenario.clone(),
        source_pad_id: source_pad_id.to_owned(),
        target_pad_id: target_pad_id.to_owned(),
        probe_id: scenario.id.clone(),
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    }
}

fn witness_identity_for_characterization(witness: &NominalWitnessEvidenceV1) -> Result<String> {
    let mut canonical = witness.clone();
    canonical.identity.clear();
    stable_digest(&canonical)
}

fn is_known_finite_candidate_rejection(message: &str) -> bool {
    [
        "candidate exceeds original absolute mission deadline",
        "intended pad reached before powered terminal at coast tick",
        "nominal shape requires at most one future apex, not dive and recover",
        "candidate exhausts actual incoming fuel",
        "first intended-pad contact is unsafe or enters from below",
        "finite terminal window contains no safe intended-pad contact",
        "paired throttle above maximum or unsupported remaining mass",
        "paired throttle below minimum",
    ]
    .iter()
    .any(|known| message.contains(known))
}

fn classify_candidate_failure(message: String) -> WitnessMaterializationFailure {
    if is_known_finite_candidate_rejection(&message) {
        WitnessMaterializationFailure::FiniteMiss(message)
    } else {
        WitnessMaterializationFailure::Integrity(format!(
            "unclassified terminal candidate failure: {message}"
        ))
    }
}

fn is_integrity_acquisition_plan_error(message: &str) -> bool {
    [
        "non-finite",
        "invalid vehicle rotation rate",
        "turn duration exceeds representable",
        "gain is not positive",
        "demand is non-finite",
    ]
    .iter()
    .any(|known| message.contains(known))
}

fn record_materialization_failure(
    evidence: &mut NominalCharacterizationRowEvidenceV1,
    seed_id: &str,
    failure: WitnessMaterializationFailure,
) {
    match failure {
        WitnessMaterializationFailure::FiniteMiss(reason) => evidence
            .finite_miss_reasons
            .push(format!("{seed_id}: {reason}")),
        WitnessMaterializationFailure::Integrity(reason) => evidence
            .integrity_errors
            .push(format!("{seed_id}: {reason}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_case() -> (ScenarioSpec, String, String) {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let bytes = fs::read(
            repo.join("fixtures/research/waypoint_direct_body_aware_terminal_fresh_inputs_v1.json"),
        )
        .unwrap();
        let manifest: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let first = &manifest["cases"][0];
        (
            serde_json::from_value(first["scenario"].clone()).unwrap(),
            first["source_pad_id"].as_str().unwrap().to_owned(),
            first["target_pad_id"].as_str().unwrap().to_owned(),
        )
    }

    fn context_for(scenario: &ScenarioSpec) -> RunContext {
        RunContext::from_scenario(scenario).unwrap()
    }

    fn descending_case() -> (ScenarioSpec, String, String) {
        let (mut scenario, source, target) = fixture_case();
        let pad = scenario.world.landing_pad(&target).unwrap();
        scenario.initial_state.position_m = Vec2::new(
            pad.center_x_m - 480.0,
            pad.surface_y_m + scenario.vehicle.geometry.touchdown_base_offset_m + 700.0,
        );
        scenario.initial_state.velocity_mps = Vec2::new(24.0, -32.0);
        (scenario, source, target)
    }

    #[test]
    fn sealed_corpus_binds_embedded_rows_not_just_attached_source_hashes() {
        assert!(verify_approved_corpus_sha256(APPROVED_CORPUS_SHA256).is_ok());
        assert!(verify_approved_corpus_sha256(&"0".repeat(64)).is_err());
    }

    #[test]
    fn zero_acquisition_uses_natural_time_and_replays_an_odd_endpoint() {
        let (scenario, _, _) = descending_case();
        let context = context_for(&scenario);
        let incoming = SimulationState::new(&context).unwrap();
        let seeds = generate_seed_specs(&context, &incoming).unwrap();
        let zero = seeds.iter().find(|seed| seed.zero_acquisition).unwrap();
        let natural = seeds
            .iter()
            .find(|seed| !seed.zero_acquisition && seed.natural_profile)
            .unwrap();
        assert_eq!(zero.virtual_arrival_ticks, natural.virtual_arrival_ticks);
        let baseline_ticks = even_ticks_ceil(
            (960.0 / context.world.gravity_mps2).sqrt(),
            context.sim.physics_dt_s(),
        )
        .unwrap();
        assert!(zero.virtual_arrival_ticks < baseline_ticks);
        let updates = vec![FlightProgramUpdateV1 {
            physics_step: 0,
            phase: "ballistic_coast".into(),
            command: Command::idle(),
        }];
        validate_command_schedule(0, 1, &updates).unwrap();
        let mut expected_end = incoming.clone();
        expected_end.set_command(updates[0].command);
        expected_end.step_physics_and_classify_contact(&context);
        let replay = replay_neutral(
            &context,
            &incoming,
            &updates,
            0,
            1,
            1,
            &SimulationStateSnapshotV1::from_state(&incoming),
        )
        .unwrap();
        assert!(replay.acquisition_end_match);
        assert_eq!(replay.final_state.physics_step, 1);
        assert_eq!(
            SimulationStateSnapshotV1::from_state(&replay.final_state),
            SimulationStateSnapshotV1::from_state(&expected_end)
        );
        assert!(replay.terminal_entry.is_some());
    }

    #[test]
    fn powered_arrival_can_extend_virtual_time_and_steep_energy_still_rejects() {
        let (scenario, _, _) = descending_case();
        let context = context_for(&scenario);
        let incoming = SimulationState::new(&context).unwrap();
        let zero = generate_seed_specs(&context, &incoming)
            .unwrap()
            .into_iter()
            .find(|seed| seed.zero_acquisition)
            .unwrap();
        let estimate = evaluate_seed(&context, &incoming, 9600, zero.clone()).unwrap();
        assert!(estimate.entry_screens.iter().any(|screen| screen.admissible
            && screen.predicted_finish_physics_step.unwrap() > zero.virtual_arrival_ticks));
        let start = target_kinematics(&context, &incoming);
        let reversal = build_reference(
            &context,
            &BodyAwareTerminalPolicyV1::default(),
            KinematicStateV2 {
                position_m: Vec2::new(-20.0, start.position_m.y),
                ..start
            },
            2000,
        )
        .unwrap();
        assert!(reversal.start.velocity_mps.x > 0.0);
        assert!(reversal.end.velocity_mps.x >= 0.0);
        assert!(!terminal_forward_velocity_valid(&context, &reversal));

        let mut unsafe_scenario = scenario.clone();
        unsafe_scenario.initial_state.position_m.y = context.target_pad.surface_y_m
            + context.vehicle.geometry.touchdown_base_offset_m
            + 80.0;
        unsafe_scenario.initial_state.velocity_mps = Vec2::new(5.0, -140.0);
        let unsafe_context = context_for(&unsafe_scenario);
        let unsafe_live = SimulationState::new(&unsafe_context).unwrap();
        let unsafe_zero = generate_seed_specs(&unsafe_context, &unsafe_live)
            .unwrap()
            .into_iter()
            .find(|seed| seed.zero_acquisition)
            .unwrap();
        let plan = make_acquisition_plan(&unsafe_context, &unsafe_live, &unsafe_zero).unwrap();
        let screen = screen_entry(&unsafe_context, &unsafe_live, 9600, &plan, 0, 0.0, 0).unwrap();
        assert!(screen.predicted_entry_angle_rad.to_degrees() > 80.0);
        assert!(!screen.admissible);
        assert!(
            screen
                .reason
                .unwrap()
                .contains("coupled terminal thrust bound")
        );
    }

    #[test]
    fn terrain_contact_audits_raw_incoming_state_not_landing_adaptation() {
        let (mut scenario, source, target) = fixture_case();
        let pad = scenario.world.landing_pad(&target).unwrap();
        scenario.initial_state.position_m = Vec2::new(
            pad.center_x_m,
            pad.surface_y_m + scenario.vehicle.geometry.touchdown_base_offset_m + 0.002,
        );
        scenario.initial_state.velocity_mps = Vec2::new(0.0, -1.5);
        let context = context_for(&scenario);
        let incoming = SimulationState::new(&context).unwrap();
        let updates = vec![FlightProgramUpdateV1 {
            physics_step: 0,
            phase: "terminal_bridge".into(),
            command: Command::idle(),
        }];
        let audit = run_real_terrain_audit(
            &context,
            &research_request(&scenario, &source, &target),
            &incoming,
            &updates,
            1,
            Some(1),
        )
        .unwrap();
        assert!(audit.ordinary_neutral_prefix_parity);
        assert!(audit.landed_on_target);
        assert_eq!(audit.first_contact_physics_step, Some(1));
        assert!(audit.first_contact.is_some());
        assert!(
            audit.reserve_phases.is_empty(),
            "contact is a separate predicate, not an airborne reserve sample"
        );
    }

    #[test]
    fn discrete_acquisition_coefficients_match_constant_affine_bridge_recurrence() {
        let (scenario, _, _) = fixture_case();
        let context = context_for(&scenario);
        let dt = context.sim.physics_dt_s();
        let gravity = context.world.gravity_mps2;
        let incoming = KinematicStateV2 {
            position_m: Vec2::new(-40.0, 180.0),
            velocity_mps: Vec2::new(7.0, -11.0),
        };
        let known_thrust_acceleration = Vec2::new(3.0, 11.0);
        let arrival_ticks = 30;
        let turn_ticks = 6;
        let burn_ticks = 10;
        let target = virtual_arrival_prediction(
            incoming,
            arrival_ticks,
            turn_ticks,
            burn_ticks,
            known_thrust_acceleration,
            dt,
            gravity,
        )
        .position_m;
        let thrust = solve_constant_acquisition(
            incoming,
            target,
            arrival_ticks,
            turn_ticks,
            burn_ticks,
            dt,
            gravity,
        )
        .unwrap();
        assert!(distance(thrust, known_thrust_acceleration) < 1.0e-10);
        let predicted_virtual = virtual_arrival_prediction(
            incoming,
            arrival_ticks,
            turn_ticks,
            burn_ticks,
            thrust,
            dt,
            gravity,
        );
        assert!(distance(predicted_virtual.position_m, target) < 1.0e-10);

        let acquisition_end =
            acquisition_end_prediction(incoming, turn_ticks, burn_ticks, thrust, dt, gravity);
        let mut recurrence = incoming;
        for tick in 0..turn_ticks + burn_ticks {
            let net = if tick < turn_ticks {
                Vec2::new(0.0, -gravity)
            } else {
                Vec2::new(thrust.x, thrust.y - gravity)
            };
            recurrence.velocity_mps += net * dt;
            recurrence.position_m += recurrence.velocity_mps * dt;
        }
        assert!(distance(recurrence.position_m, acquisition_end.position_m) < 1.0e-10);
        assert!(distance(recurrence.velocity_mps, acquisition_end.velocity_mps) < 1.0e-11);

        let burn_start = coast_prediction(incoming, turn_ticks, dt, gravity);
        let constant_affine = BodyAwareTerminalReferenceV1 {
            start: burn_start,
            end: acquisition_end,
            physics_ticks: burn_ticks,
            horizontal_coefficients_mps2: [thrust.x, 0.0, 0.0],
            initial_vertical_acceleration_mps2: thrust.y - gravity,
            vertical_acceleration_delta_mps2: 0.0,
            identity: String::new(),
        };
        let affine_endpoint = constant_affine.state_at(&context, burn_ticks);
        assert!(distance(affine_endpoint.position_m, acquisition_end.position_m) < 1.0e-10);
        assert!(distance(affine_endpoint.velocity_mps, acquisition_end.velocity_mps) < 1.0e-11);

        let analytical_policy = pd_plan::conservative_ballistic_bridge::DirectBridgePolicyV2 {
            physics_hz: context.sim.physics_hz,
            gravity_mps2: gravity,
            duration_multipliers: vec![0.75, 1.0, 1.25, 1.5],
            minimum_clearance_m: 5.0,
            maximum_mission_time_s: 90.0,
            mission_time_reserve_s: 10.0,
            thrust_derate: 0.81,
            declared_robustness_margin: 0.075,
            handoff_interval_s: 0.25,
            bridge_duration_interval_s: 0.5,
            terminal_target_downward_speed_fraction: 0.5,
        };
        let vehicle = pd_plan::conservative_ballistic_bridge::VehicleInputV2 {
            geometry: pd_plan::conservative_ballistic_bridge::VehicleGeometryInputV2 {
                hull_width_m: context.vehicle.geometry.hull_width_m,
                hull_height_m: context.vehicle.geometry.hull_height_m,
                touchdown_half_span_m: context.vehicle.geometry.touchdown_half_span_m,
                touchdown_base_offset_m: context.vehicle.geometry.touchdown_base_offset_m,
            },
            dry_mass_kg: context.vehicle.dry_mass_kg,
            initial_fuel_kg: context.vehicle.initial_fuel_kg,
            max_fuel_kg: context.vehicle.max_fuel_kg,
            max_fuel_burn_kgps: context.vehicle.max_fuel_burn_kgps,
            max_thrust_n: context.vehicle.max_thrust_n,
            min_throttle_frac: context.vehicle.min_throttle_frac,
            max_rotation_rate_radps: context.vehicle.max_rotation_rate_radps,
            safe_touchdown_normal_speed_mps: context.vehicle.safe_touchdown_normal_speed_mps,
            safe_touchdown_tangential_speed_mps: context
                .vehicle
                .safe_touchdown_tangential_speed_mps,
            safe_touchdown_attitude_error_rad: context.vehicle.safe_touchdown_attitude_error_rad,
            safe_touchdown_angular_rate_radps: context.vehicle.safe_touchdown_angular_rate_radps,
        };
        let bridge = pd_plan::conservative_ballistic_bridge::exact_discrete_bridge_v2(
            &analytical_policy,
            &vehicle,
            pd_plan::conservative_ballistic_bridge::BridgeKindV2::Intermediate,
            burn_start,
            acquisition_end,
            burn_ticks,
        )
        .unwrap();
        assert!(
            distance(
                bridge.initial_net_acceleration_mps2,
                Vec2::new(thrust.x, thrust.y - gravity)
            ) < 1.0e-9
        );
        assert!(bridge.net_acceleration_step_mps2.length() < 1.0e-9);
    }

    #[test]
    fn turn_estimate_wraps_at_pi_and_does_not_consume_initial_angular_rate() {
        let (scenario, _, _) = fixture_case();
        let context = context_for(&scenario);
        let target_angle = -std::f64::consts::PI + 0.1;
        let target = Vec2::new(target_angle.sin(), target_angle.cos());
        let current = std::f64::consts::PI - 0.1;
        let (ticks, angle) = turn_ticks_for(&context, current, target).unwrap();
        assert_eq!(ticks, 16);
        assert!((shortest_angle_delta(current, angle).abs() - 0.2).abs() < 1.0e-12);

        let mut live_a = SimulationState::new(&context).unwrap();
        let mut live_b = live_a.clone();
        live_a.attitude_rad = current;
        live_b.attitude_rad = current;
        live_a.angular_rate_radps = 100.0;
        live_b.angular_rate_radps = -100.0;
        assert_eq!(
            turn_ticks_for(&context, live_a.attitude_rad, target).unwrap(),
            turn_ticks_for(&context, live_b.attitude_rad, target).unwrap()
        );
    }

    #[test]
    fn paired_acceleration_units_minimum_throttle_fuel_and_shared_terminal_cap_are_explicit() {
        let (scenario, _, _) = fixture_case();
        let context = context_for(&scenario);
        let policy = BodyAwareTerminalPolicyV1::default();
        let acceleration = Vec2::new(0.0, 6.0);
        let burn_ticks = 4;
        let estimated_fuel = estimate_acquisition_fuel(
            &context,
            context.vehicle.initial_fuel_kg,
            acceleration,
            burn_ticks,
        )
        .unwrap();
        let mut live = SimulationState::new(&context).unwrap();
        live.position_m = Vec2::new(context.target_pad.center_x_m - 200.0, 1000.0);
        live.velocity_mps = Vec2::new(0.0, 0.0);
        let incoming_fuel = live.fuel_kg;
        for _ in 0..burn_ticks / HELD_TICKS {
            let throttle = paired_throttle(
                &context,
                &policy,
                live.mass_kg(&context),
                acceleration.length(),
            )
            .unwrap();
            live.set_command(Command {
                throttle_frac: throttle,
                target_attitude_rad: 0.0,
            });
            for _ in 0..HELD_TICKS {
                assert_eq!(
                    live.step_physics_and_classify_contact(&context),
                    ContactClassification::None
                );
            }
        }
        assert!(((incoming_fuel - live.fuel_kg) - estimated_fuel).abs() < 1.0e-10);
        let measured_net_acceleration = live.velocity_mps.y
            / (burn_ticks as f64 * context.sim.physics_dt_s())
            + context.world.gravity_mps2;
        assert!((measured_net_acceleration - acceleration.y).abs() < 1.0e-10);

        let mut low_minimum_context = context.clone();
        low_minimum_context.vehicle.min_throttle_frac = 0.25;
        assert!(
            paired_throttle(
                &low_minimum_context,
                &policy,
                live.mass_kg(&low_minimum_context),
                0.1,
            )
            .is_err()
        );

        let incoming_max_acceleration = context.vehicle.max_thrust_n / live.mass_kg(&context);
        let component = 0.7 * incoming_max_acceleration;
        let reference = BodyAwareTerminalReferenceV1 {
            start: KinematicStateV2 {
                position_m: live.position_m,
                velocity_mps: live.velocity_mps,
            },
            end: KinematicStateV2 {
                position_m: live.position_m,
                velocity_mps: live.velocity_mps,
            },
            physics_ticks: 4,
            horizontal_coefficients_mps2: [component, 0.0, 0.0],
            initial_vertical_acceleration_mps2: component - context.world.gravity_mps2,
            vertical_acceleration_delta_mps2: 0.0,
            identity: String::new(),
        };
        let coupled = conservative_terminal_thrust_bound(&context, &reference);
        assert!((coupled - component.hypot(component)).abs() < 1.0e-12);
        assert!(coupled > MAX_THRUST_FRACTION * incoming_max_acceleration);
    }

    #[test]
    fn zero_acquisition_and_natural_profile_do_not_require_a_future_apex() {
        let (mut scenario, _, target_pad_id) = fixture_case();
        let target = scenario.world.landing_pad(&target_pad_id).unwrap().clone();
        scenario.initial_state.position_m = Vec2::new(
            target.center_x_m - 480.0,
            target.surface_y_m + scenario.vehicle.geometry.touchdown_base_offset_m + 700.0,
        );
        scenario.initial_state.velocity_mps = Vec2::new(24.0, -32.0);
        let context = context_for(&scenario);
        let live = SimulationState::new(&context).unwrap();
        let seeds = generate_seed_specs(&context, &live).unwrap();
        assert!(!seeds.is_empty());
        assert!(seeds.len() <= MAX_SEEDS);
        let zero = seeds.iter().find(|seed| seed.zero_acquisition).unwrap();
        assert_eq!(zero.kind, "zero_acquisition");
        let natural = seeds
            .iter()
            .find(|seed| seed.natural_profile && !seed.zero_acquisition)
            .unwrap();
        let plan = make_acquisition_plan(&context, &live, natural).unwrap();
        assert_eq!(plan.thrust_acceleration_mps2.y, 0.0);
        assert!(plan.predicted_end.position_m.y < live.position_m.y);
        assert!(plan.predicted_end.velocity_mps.y < live.velocity_mps.y);

        scenario
            .world
            .landing_pads
            .iter_mut()
            .find(|pad| pad.id == target_pad_id)
            .unwrap()
            .surface_y_m += 1000.0;
        let uphill_context = context_for(&scenario);
        let uphill_live = SimulationState::new(&uphill_context).unwrap();
        let uphill = generate_seed_specs(&uphill_context, &uphill_live).unwrap();
        assert!(!uphill.is_empty());
        assert!(uphill.len() <= MAX_SEEDS);
        assert!(uphill.iter().any(|seed| seed.zero_acquisition));
        assert!(uphill.iter().all(|seed| seed.kind != "natural_profile"));
        assert!(uphill.iter().any(|seed| seed.kind == "upward_shaping"));
    }

    #[test]
    fn candidate_backend_errors_are_split_from_integrity_and_labeled_separately() {
        let finite = classify_candidate_failure(
            "finite terminal window contains no safe intended-pad contact".into(),
        );
        let integrity = classify_candidate_failure("internal replay binding disappeared".into());
        assert!(matches!(
            finite,
            WitnessMaterializationFailure::FiniteMiss(_)
        ));
        assert!(matches!(
            integrity,
            WitnessMaterializationFailure::Integrity(_)
        ));

        let snapshot = SimulationStateSnapshotV1 {
            sim_time_s: 0.0,
            physics_step: 0,
            position_m: Vec2::new(0.0, 0.0),
            velocity_mps: Vec2::new(0.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
            fuel_kg: 1.0,
            held_command: Command::idle(),
            physical_outcome: PhysicalOutcome::Flying,
            mission_outcome: MissionOutcome::InProgress,
            end_reason: EndReason::Running,
            min_touchdown_clearance_m: 0.0,
            min_hull_clearance_m: 0.0,
            max_speed_mps: 0.0,
            max_abs_attitude_rad: 0.0,
            max_abs_angular_rate_radps: 0.0,
            waypoint_sequence_passed: 0,
            waypoint_sequence_first_failure_index: None,
            waypoint_handoff_window_index: None,
        };
        let mut evidence = NominalCharacterizationRowEvidenceV1 {
            row_id: "test".into(),
            population: "synthetic_condition".into(),
            label: "failure routing test".into(),
            source_pad_id: "source".into(),
            target_pad_id: "target".into(),
            initial_physics_step: 0,
            initial_fuel_kg: 1.0,
            original_deadline_physics_step: 2,
            expected_state_match: None,
            prefix_update_count: 0,
            ground_prep: None,
            seeds: Vec::new(),
            shortlist_seed_ids: Vec::new(),
            witnesses: Vec::new(),
            finite_miss_reasons: Vec::new(),
            integrity_errors: Vec::new(),
            status: "finite_miss".into(),
            identity: String::new(),
        };
        assert_eq!(snapshot.physics_step, 0);
        record_materialization_failure(&mut evidence, "seed-a", finite);
        record_materialization_failure(&mut evidence, "seed-b", integrity);
        assert_eq!(evidence.finite_miss_reasons.len(), 1);
        assert_eq!(evidence.integrity_errors.len(), 1);
    }

    #[test]
    fn corpus_schema_prefix_clock_fuel_and_binding_mismatches_are_rejected() {
        let malformed = WaypointV2NominalCharacterizationCorpusV1 {
            schema_id: "not_the_frozen_corpus_schema".into(),
            bindings: Vec::new(),
            rows: Vec::new(),
        };
        assert!(validate_corpus(&malformed).is_err());
        assert!(validate_relative_source_path("../Cargo.toml").is_err());

        let repo = repo_root().unwrap();
        let bad_binding = WaypointV2NominalCharacterizationBindingV1 {
            path: "Cargo.toml".into(),
            sha256: "0".repeat(64),
        };
        assert!(verify_bindings(&repo, &[bad_binding]).is_err());

        let (scenario, source, target) = fixture_case();
        let mut row = WaypointV2NominalCharacterizationCorpusRowV1 {
            id: "prefix-test".into(),
            population: "local_handoff".into(),
            label: "gapped prefix".into(),
            scenario: scenario.clone(),
            source_pad_id: source,
            target_pad_id: target,
            absolute_deadline_physics_step: 9600,
            expected_state: Some(SimulationStateSnapshotV1 {
                sim_time_s: 4.0 / 120.0,
                physics_step: 4,
                position_m: Vec2::new(0.0, 0.0),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
                fuel_kg: scenario.vehicle.initial_fuel_kg,
                held_command: Command::idle(),
                physical_outcome: PhysicalOutcome::Flying,
                mission_outcome: MissionOutcome::InProgress,
                end_reason: EndReason::Running,
                min_touchdown_clearance_m: 0.0,
                min_hull_clearance_m: 0.0,
                max_speed_mps: 0.0,
                max_abs_attitude_rad: 0.0,
                max_abs_angular_rate_radps: 0.0,
                waypoint_sequence_passed: 0,
                waypoint_sequence_first_failure_index: None,
                waypoint_handoff_window_index: None,
            }),
            prefix_updates: vec![
                FlightProgramUpdateV1 {
                    physics_step: 0,
                    phase: "origin_prefix".into(),
                    command: Command::idle(),
                },
                FlightProgramUpdateV1 {
                    physics_step: 4,
                    phase: "origin_prefix".into(),
                    command: Command::idle(),
                },
            ],
        };
        assert!(validate_prefix(&row, HELD_TICKS).is_err());

        let context = context_for(&scenario);
        let original = SimulationState::new(&context).unwrap();
        assert_eq!(original.physics_step, 0);
        assert_eq!(original.sim_time_s, 0.0);
        assert_eq!(original.fuel_kg, scenario.vehicle.initial_fuel_kg);
        let mut replay = original.clone();
        let command = Command {
            throttle_frac: 0.5,
            target_attitude_rad: 0.0,
        };
        for _ in 0..4 {
            replay.set_command(command);
            replay.step_physics_and_classify_contact(&context);
        }
        assert_eq!(replay.physics_step, 4);
        assert_eq!(replay.sim_time_s, 4.0 / 120.0);
        assert!(replay.fuel_kg < original.fuel_kg);
        let actual = SimulationStateSnapshotV1::from_state(&replay);
        let mut wrong_clock = actual.clone();
        wrong_clock.physics_step += 2;
        assert!(!compare_full_snapshot(&actual, &wrong_clock).matched);
        let mut wrong_fuel = actual.clone();
        wrong_fuel.fuel_kg += 1.0;
        assert!(!compare_full_snapshot(&actual, &wrong_fuel).matched);
        row.prefix_updates.clear();
    }

    #[test]
    fn clear_prep_has_fresh_h0_command_replay_and_terrain_only_changes_audit_not_choice() {
        let (mut scenario, source_pad_id, target_pad_id) = fixture_case();
        let context = context_for(&scenario);
        let mut live = SimulationState::new(&context).unwrap();
        let prep = ground_prep(
            &context,
            &scenario,
            &source_pad_id,
            &target_pad_id,
            &mut live,
            9600,
        )
        .unwrap();
        assert!(prep.success, "ground prep miss: {:?}", prep.reason);
        assert!(!prep.commands.is_empty());
        let mut checked_prep = prep.clone();
        verify_ground_prep_replay(&context, &mut checked_prep).unwrap();
        assert!(checked_prep.independent_replay_passed);
        assert!(checked_prep.integrity_error.is_none());

        let target = scenario.world.landing_pad(&target_pad_id).unwrap().clone();
        scenario.initial_state.position_m = Vec2::new(
            target.center_x_m - 420.0,
            target.surface_y_m + scenario.vehicle.geometry.touchdown_base_offset_m + 700.0,
        );
        scenario.initial_state.velocity_mps = Vec2::new(22.0, -32.0);
        let mut terrain_twin = scenario.clone();
        let pd_core::TerrainDefinition::Heightfield { points_m } = &mut terrain_twin.world.terrain;
        for point in points_m {
            point.y += 25.0;
        }
        let context_a = context_for(&scenario);
        let context_b = context_for(&terrain_twin);
        let live_a = SimulationState::new(&context_a).unwrap();
        let live_b = SimulationState::new(&context_b).unwrap();
        let seeds_a = generate_seed_specs(&context_a, &live_a).unwrap();
        let seeds_b = generate_seed_specs(&context_b, &live_b).unwrap();
        assert_eq!(seeds_a, seeds_b);
        let mut parity = None;
        for seed in seeds_a.iter().filter(|seed| !seed.zero_acquisition) {
            if let (Ok(plan_a), Ok(plan_b)) = (
                make_acquisition_plan(&context_a, &live_a, seed),
                make_acquisition_plan(&context_b, &live_b, seed),
            ) {
                parity = Some((seed.clone(), plan_a, plan_b));
                break;
            }
        }
        let (seed, plan_a, plan_b) = parity.expect("at least one finite acquisition seed");
        assert_eq!(plan_a.predicted_end, plan_b.predicted_end);
        assert_eq!(
            plan_a.thrust_acceleration_mps2,
            plan_b.thrust_acceleration_mps2
        );
        let execution_a = run_acquisition(&context_a, &live_a, &plan_a).unwrap();
        let execution_b = run_acquisition(&context_b, &live_b, &plan_b).unwrap();
        assert_eq!(execution_a.turn_updates, execution_b.turn_updates);
        assert_eq!(execution_a.burn_updates, execution_b.burn_updates);
        let estimate_a = evaluate_seed(&context_a, &live_a, 9600, seed.clone()).unwrap();
        let estimate_b = evaluate_seed(&context_b, &live_b, 9600, seed).unwrap();
        assert_eq!(
            estimate_a.selected_entry_index,
            estimate_b.selected_entry_index
        );
        assert_eq!(estimate_a.entry_screens, estimate_b.entry_screens);
    }

    #[test]
    fn synthetic_conditions_are_eight_fresh_scenarios_and_outputs_are_create_only() {
        let (scenario, source, target) = fixture_case();
        let synthetic = synthetic_scenarios(&scenario, &source, &target).unwrap();
        assert_eq!(synthetic.len(), EXPECTED_SYNTHETIC_ROWS);
        assert!(
            synthetic
                .iter()
                .all(|(scenario, _)| scenario.validate().is_ok())
        );

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let output = std::env::temp_dir().join(format!(
            "waypoint-v2-nominal-characterization-{}-{unique}",
            std::process::id()
        ));
        reserve_output_root(&output).unwrap();
        let summary =
            serde_json::json!({"schema_id": WAYPOINT_V2_NOMINAL_CHARACTERIZATION_SCHEMA_ID});
        write_create_only(&output.join("summary.json"), &summary).unwrap();
        assert!(write_create_only(&output.join("summary.json"), &summary).is_err());
        assert!(reserve_output_root(&output).is_err());
        fs::remove_file(output.join("summary.json")).unwrap();
        fs::remove_dir(output).unwrap();
    }
}
