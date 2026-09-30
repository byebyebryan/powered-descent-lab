//! Evaluator-only finite canary for regenerating a coast-plus-terminal segment
//! from retained, ordinary airborne state. No planner/default APIs change.

use std::{collections::BTreeMap, fs, path::Path, time::Instant};

use anyhow::{Context, Result, bail};
use pd_core::{
    ActionLogEntry, Command, EndReason, EvaluationGoal, EventKind, EventRecord,
    FlightProgramUpdateV1, IncomingContactV1, MissionOutcome, PhysicalOutcome, RunArtifacts,
    RunContext, SampleRecord, ScenarioSpec, SimulationState, SimulationStateSnapshotV1,
    TerrainDefinition, Vec2, replay_simulation,
};
use serde::{Deserialize, Serialize};

use crate::{
    AirborneDirectAuditV1, AirborneDirectProposalV1, AirborneDirectSearchV1,
    BodyAwareTerminalPolicyV1, NominalDirectFlightDecisionV1,
    NominalDirectFlightExecutionEvidenceV1, NominalDirectFlightSourceBindingV1,
    NominalDirectFlightSourceFileV1, WaypointDirectNominalDirectGenerationRequest,
    audit_airborne_direct_proposal, evaluate_airborne_nominal_direct,
    evaluate_nominal_direct_flight, execute_nominal_direct_flight_program,
    load_nominal_direct_operational_fresh_inputs,
    nominal_direct_flight::{reserve_output_root, write_create_only},
    nominal_direct_flight_identity,
    waypoint_direct_body_aware_terminal::sha256_bytes,
};

pub const NOMINAL_AIRBORNE_DIRECT_CANARY_PROTOCOL: &str =
    "docs/nominal_airborne_direct_canary_protocol.md";
pub const NOMINAL_AIRBORNE_DIRECT_CANARY_SCHEMA: &str = "nominal_airborne_direct_canary_v1";
const INPUT_MANIFEST: &str = "fixtures/research/nominal_direct_operational_fresh_inputs_v1.json";
const MINIMUM_CLEARANCE_M: f64 = 5.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalAirborneDirectCanaryInputBindingV1 {
    pub relative_path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalAirborneDirectCanaryCaseIndexV1 {
    pub case_id: String,
    pub status: String,
    pub identity: String,
    pub artifact_path: String,
    pub compatibility_failures: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalAirborneDirectCanarySummaryV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub protocol_path: String,
    pub protocol_sha256: String,
    pub input_manifest_path: String,
    pub input_manifest_sha256: String,
    pub source_binding: NominalDirectFlightSourceBindingV1,
    pub input_binding: NominalAirborneDirectCanaryInputBindingV1,
    pub case_count: usize,
    pub passed: bool,
    pub compatibility_verdict: String,
    pub cases: Vec<NominalAirborneDirectCanaryCaseIndexV1>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalAirborneDirectCanaryCaseV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub case_id: String,
    pub request: WaypointDirectNominalDirectGenerationRequest,
    pub status: String,
    pub baseline_decision: Option<NominalDirectFlightDecisionV1>,
    pub baseline_generation: Option<crate::BodyAwareTerminalCaseArtifactV1>,
    pub baseline_compute: Option<crate::NominalDirectFlightComputeV1>,
    pub baseline_execution: Option<NominalDirectFlightExecutionEvidenceV1>,
    pub baseline_ordinary_run: Option<RunArtifacts>,
    pub baseline_action_replay: Option<RunArtifacts>,
    pub baseline_replay_matches_ordinary: Option<bool>,
    pub baseline_execution_error: Option<String>,
    pub capture_trace: Vec<AirborneCoastBoundaryV1>,
    pub captures: Vec<NominalAirborneCaptureV1>,
    pub compatibility_failures: Vec<String>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirborneCoastBoundaryV1 {
    pub physics_step: u64,
    pub sim_time_s: f64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub held_command: Command,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalAirborneCaptureV1 {
    pub role: String,
    pub physics_step: u64,
    pub sim_time_s: f64,
    pub velocity_y_mps: f64,
    pub full_state_snapshot: SimulationStateSnapshotV1,
    pub search: Option<AirborneDirectSearchV1>,
    pub search_wall_time_us: Option<u64>,
    pub selected_proposal: Option<AirborneDirectProposalV1>,
    pub nominal_audit: Option<AirborneDirectAuditV1>,
    pub nominal_audit_error: Option<String>,
    pub nominal_audit_wall_time_us: Option<u64>,
    pub terrain_twin: Option<NominalAirborneTerrainTwinEvidenceV1>,
    pub stitched_replay: Option<NominalAirborneStitchedReplayV1>,
    pub compatibility_failures: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalAirborneTerrainTwinEvidenceV1 {
    pub obstacle_x_min_m: Option<f64>,
    pub obstacle_x_peak_m: Option<f64>,
    pub obstacle_x_max_m: Option<f64>,
    pub obstacle_peak_y_m: Option<f64>,
    pub peak_above_nominal_future_peak_m: Option<f64>,
    pub source_pad_unchanged: bool,
    pub target_pad_unchanged: bool,
    pub source_shelf_preserved: bool,
    pub target_shelf_preserved: bool,
    pub nonterrain_context_unchanged: bool,
    pub obstruction_is_ahead_and_before_target_shelf: bool,
    pub search_identical: Option<bool>,
    pub search_identity_matches: Option<bool>,
    pub selected_commands_identical: Option<bool>,
    pub twin_search: Option<AirborneDirectSearchV1>,
    pub audit: Option<AirborneDirectAuditV1>,
    pub audit_error: Option<String>,
    pub audit_rejected: Option<bool>,
    pub compatibility_failures: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalAirborneStitchedReplayV1 {
    pub capture_physics_step: u64,
    pub prefix_action_count: usize,
    pub candidate_update_count_supplied: usize,
    pub candidate_updates: Vec<FlightProgramUpdateV1>,
    pub actions: Vec<ActionLogEntry>,
    pub suffix_terminal_reached_without_fallback: bool,
    pub suffix_final_state: SimulationStateSnapshotV1,
    pub source_replay_final_state: Option<SimulationStateSnapshotV1>,
    pub source_replay_contact: Option<IncomingContactV1>,
    pub source_replay_events: Vec<EventRecord>,
    pub source_replay_samples: Vec<SampleRecord>,
    pub no_checkpoint_events: bool,
    pub contiguous_absolute_action_clock: bool,
    pub replay: Option<RunArtifacts>,
    pub replay_error: Option<String>,
    pub replay_actions_match: Option<bool>,
    pub source_replay_artifacts_match_official_replay: Option<bool>,
    pub source_replay_final_snapshot_matches_live_audit: Option<bool>,
    pub source_replay_contact_matches_live_audit: Option<bool>,
    pub replay_contact_time_tick_fuel_match: Option<bool>,
    pub replay_target_landing: Option<bool>,
    pub replay_checkpoint_events_absent: Option<bool>,
}

pub(crate) struct CapturedLiveState {
    pub(crate) state: SimulationState,
}

pub(crate) struct CaptureCollection {
    pub(crate) first: Option<CapturedLiveState>,
    pub(crate) near_apex: Option<CapturedLiveState>,
    pub(crate) last: Option<CapturedLiveState>,
    pub(crate) trace: Vec<AirborneCoastBoundaryV1>,
    pub(crate) failures: Vec<String>,
}

pub(crate) struct TerrainTwin {
    pub(crate) context: RunContext,
    pub(crate) evidence: NominalAirborneTerrainTwinEvidenceV1,
}

pub(crate) struct OrdinarySourceReplay {
    pub(crate) final_state: SimulationStateSnapshotV1,
    pub(crate) incoming_contact: Option<IncomingContactV1>,
    pub(crate) actions: Vec<ActionLogEntry>,
    pub(crate) events: Vec<EventRecord>,
    pub(crate) samples: Vec<SampleRecord>,
}

/// Run the opt-in four-input canary into a new create-only output root.
/// Finite compatibility failures are returned and persisted, not discarded.
pub fn run_nominal_airborne_direct_canary(
    repo_root: &Path,
    output_root: &Path,
) -> Result<NominalAirborneDirectCanarySummaryV1> {
    let input_bytes = fs::read(repo_root.join(INPUT_MANIFEST))
        .with_context(|| format!("read sealed input manifest {INPUT_MANIFEST}"))?;
    let input_sha256 = sha256_bytes(&input_bytes)?;
    let protocol_sha256 = sha256_bytes(&fs::read(
        repo_root.join(NOMINAL_AIRBORNE_DIRECT_CANARY_PROTOCOL),
    )?)?;
    let source_binding_before = source_binding(repo_root)?;
    let inputs = load_nominal_direct_operational_fresh_inputs(repo_root)?;

    reserve_output_root(output_root)?;
    let cases_dir = output_root.join("cases");
    fs::create_dir(&cases_dir).context("create-only case artifact directory")?;
    let mut case_index = Vec::with_capacity(inputs.len());
    for (case_id, request) in inputs {
        let mut case = run_case(case_id.clone(), request)?;
        case.identity = case_identity(&case)?;
        let file_name = format!("{case_id}.json");
        write_create_only(&cases_dir.join(&file_name), &case)?;
        case_index.push(NominalAirborneDirectCanaryCaseIndexV1 {
            case_id,
            status: case.status.clone(),
            identity: case.identity,
            artifact_path: format!("cases/{file_name}"),
            compatibility_failures: case.compatibility_failures,
        });
    }
    if source_binding(repo_root)? != source_binding_before {
        bail!(
            "canary source/input/protocol closure changed during measurements; case evidence retained"
        );
    }
    let passed =
        inputs_count_is_four(&case_index) && case_index.iter().all(|case| case.status == "pass");
    let mut summary = NominalAirborneDirectCanarySummaryV1 {
        schema_id: NOMINAL_AIRBORNE_DIRECT_CANARY_SCHEMA.into(),
        schema_version: 1,
        protocol_path: NOMINAL_AIRBORNE_DIRECT_CANARY_PROTOCOL.into(),
        protocol_sha256,
        input_manifest_path: INPUT_MANIFEST.into(),
        input_manifest_sha256: input_sha256.clone(),
        source_binding: source_binding_before,
        input_binding: NominalAirborneDirectCanaryInputBindingV1 {
            relative_path: INPUT_MANIFEST.into(),
            sha256: input_sha256,
        },
        case_count: case_index.len(),
        passed,
        compatibility_verdict: if passed {
            "bounded_airborne_regeneration_compatibility_pass".into()
        } else {
            "bounded_airborne_regeneration_compatibility_failure".into()
        },
        cases: case_index,
        identity: String::new(),
    };
    summary.identity = summary_identity(&summary)?;
    write_create_only(&output_root.join("summary.json"), &summary)?;
    Ok(summary)
}

fn inputs_count_is_four(cases: &[NominalAirborneDirectCanaryCaseIndexV1]) -> bool {
    cases.len() == 4
}

fn source_binding(repo_root: &Path) -> Result<NominalDirectFlightSourceBindingV1> {
    let mut binding = crate::nominal_direct_flight_gate::source_binding(repo_root)?;
    for path in [NOMINAL_AIRBORNE_DIRECT_CANARY_PROTOCOL, INPUT_MANIFEST] {
        if binding.files.iter().any(|file| file.relative_path == path) {
            continue;
        }
        let bytes = fs::read(repo_root.join(path))
            .with_context(|| format!("read source-binding file {path}"))?;
        binding.files.push(NominalDirectFlightSourceFileV1 {
            relative_path: path.into(),
            sha256: sha256_bytes(&bytes)?,
        });
    }
    let mut files = binding.files;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    binding.identity_sha256 = sha256_bytes(&serde_json::to_vec(&files)?)?;
    binding.files = files;
    Ok(binding)
}

fn run_case(
    case_id: String,
    request: WaypointDirectNominalDirectGenerationRequest,
) -> Result<NominalAirborneDirectCanaryCaseV1> {
    let mut case = NominalAirborneDirectCanaryCaseV1 {
        schema_id: "nominal_airborne_direct_canary_case_v1".into(),
        schema_version: 1,
        case_id: case_id.clone(),
        request: request.clone(),
        status: "finite_compatibility_failure".into(),
        baseline_decision: None,
        baseline_generation: None,
        baseline_compute: None,
        baseline_execution: None,
        baseline_ordinary_run: None,
        baseline_action_replay: None,
        baseline_replay_matches_ordinary: None,
        baseline_execution_error: None,
        capture_trace: Vec::new(),
        captures: Vec::new(),
        compatibility_failures: Vec::new(),
        identity: String::new(),
    };

    let policy = BodyAwareTerminalPolicyV1::default();
    let evaluation = match evaluate_nominal_direct_flight(&request, &policy) {
        Ok(evaluation) => evaluation,
        Err(error) => {
            case.compatibility_failures
                .push(format!("fresh baseline generation failed: {error:#}"));
            finalize_case(&mut case)?;
            return Ok(case);
        }
    };
    case.baseline_decision = Some(evaluation.decision.clone());
    case.baseline_generation = evaluation.generation.clone();
    let program = match &evaluation.decision {
        NominalDirectFlightDecisionV1::Direct { program, .. } => Some(program.as_ref().clone()),
        decision => {
            case.compatibility_failures.push(format!(
                "fresh baseline decision is {}, not Direct",
                decision.status()
            ));
            None
        }
    };
    let Some(program) = program else {
        case.baseline_compute = Some(evaluation.compute);
        finalize_case(&mut case)?;
        return Ok(case);
    };
    let Some(generation) = evaluation.generation.as_ref() else {
        case.compatibility_failures
            .push("fresh Direct baseline has no generation ledger".into());
        case.baseline_compute = Some(evaluation.compute);
        finalize_case(&mut case)?;
        return Ok(case);
    };
    let witness = generation
        .rows
        .get(match &evaluation.decision {
            NominalDirectFlightDecisionV1::Direct {
                selected_row_index, ..
            } => *selected_row_index,
            _ => unreachable!(),
        })
        .and_then(|row| row.witness.as_ref());
    let Some(witness) = witness else {
        case.compatibility_failures
            .push("fresh Direct baseline has no selected accepted witness".into());
        case.baseline_compute = Some(evaluation.compute);
        finalize_case(&mut case)?;
        return Ok(case);
    };

    let mut compute = evaluation.compute.clone();
    match execute_nominal_direct_flight_program(&request, &policy, witness, &program, &mut compute)
    {
        Ok((execution, ordinary)) => {
            case.baseline_execution = Some(execution.clone());
            case.baseline_ordinary_run = Some(ordinary.run.clone());
            let replay = replay_simulation(
                &RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?,
                &ordinary.run.manifest.controller_id,
                &ordinary.run.actions,
            );
            match replay {
                Ok(replay) => {
                    let matches = replay == ordinary.run;
                    if !matches || !execution.passed {
                        case.compatibility_failures.push(
                            "fresh source-rest baseline execution/replay parity failed".into(),
                        );
                    }
                    case.baseline_replay_matches_ordinary = Some(matches);
                    case.baseline_action_replay = Some(replay);
                }
                Err(error) => {
                    case.baseline_execution_error = Some(format!(
                        "fresh source-rest baseline action replay failed: {error}"
                    ));
                    case.compatibility_failures
                        .push("fresh source-rest baseline action replay failed".into());
                }
            }
        }
        Err(error) => {
            case.baseline_execution_error = Some(format!("{error:#}"));
            case.compatibility_failures.push(format!(
                "fresh source-rest baseline ordinary execution failed: {error:#}"
            ));
        }
    }
    case.baseline_compute = Some(compute);

    let context = match RunContext::from_scenario(&request.scenario) {
        Ok(context) => context,
        Err(error) => {
            case.compatibility_failures
                .push(format!("original run context invalid: {error}"));
            finalize_case(&mut case)?;
            return Ok(case);
        }
    };
    let captures = collect_baseline_captures(&context, &program);
    case.capture_trace = captures.trace.clone();
    case.compatibility_failures
        .extend(captures.failures.iter().cloned());
    let selected_ticks = [
        captures
            .first
            .as_ref()
            .map(|capture| capture.state.physics_step),
        captures
            .near_apex
            .as_ref()
            .map(|capture| capture.state.physics_step),
        captures
            .last
            .as_ref()
            .map(|capture| capture.state.physics_step),
    ];
    if selected_ticks.iter().any(Option::is_none)
        || selected_ticks[0] == selected_ticks[1]
        || selected_ticks[1] == selected_ticks[2]
        || selected_ticks[0] == selected_ticks[2]
    {
        case.compatibility_failures.push(
            "baseline ballistic coast did not supply three distinct required capture ticks".into(),
        );
    }
    for (role, capture) in [
        ("first_coast", captures.first),
        ("near_apex", captures.near_apex),
        ("last_coast", captures.last),
    ] {
        let Some(capture) = capture else {
            continue;
        };
        let mut record = NominalAirborneCaptureV1 {
            role: role.into(),
            physics_step: capture.state.physics_step,
            sim_time_s: capture.state.sim_time_s,
            velocity_y_mps: capture.state.velocity_mps.y,
            full_state_snapshot: SimulationStateSnapshotV1::from_state(&capture.state),
            search: None,
            search_wall_time_us: None,
            selected_proposal: None,
            nominal_audit: None,
            nominal_audit_error: None,
            nominal_audit_wall_time_us: None,
            terrain_twin: None,
            stitched_replay: None,
            compatibility_failures: Vec::new(),
        };
        if (role == "first_coast" && capture.state.velocity_mps.y <= 0.0)
            || (role == "last_coast" && capture.state.velocity_mps.y >= 0.0)
        {
            record.compatibility_failures.push(format!(
                "{role} capture has incompatible observed vy={} m/s",
                capture.state.velocity_mps.y
            ));
        }
        if capture.state.physics_step % 2 != 0
            || capture.state.sim_time_s != capture.state.physics_step as f64 / 120.0
            || capture.state.physical_outcome != PhysicalOutcome::Flying
            || capture.state.mission_outcome != MissionOutcome::InProgress
            || capture.state.end_reason != EndReason::Running
            || capture.state.held_command.throttle_frac != 0.0
            || capture.state.velocity_mps.x <= 0.0
        {
            record.compatibility_failures.push(
                "captured state is not an aligned running neutral-throttle forward airborne state"
                    .into(),
            );
        }
        if !captures.failures.is_empty()
            || !record.compatibility_failures.is_empty()
            || case.compatibility_failures.iter().any(|failure| {
                failure.contains("three distinct required capture ticks")
                    || failure.contains("did not produce required")
            })
        {
            case.compatibility_failures.extend(
                record
                    .compatibility_failures
                    .iter()
                    .map(|failure| format!("{role}: {failure}")),
            );
            case.captures.push(record);
            continue;
        }
        let deadline = (request.policy.analytical_policy.mission_budget_s() * 120.0).floor() as u64;
        let started = Instant::now();
        match evaluate_airborne_nominal_direct(&context, &capture.state, deadline) {
            Ok(search) => {
                record.search_wall_time_us = Some(elapsed_us(started));
                let selected = search.selected.clone();
                record.search = Some(search.clone());
                let Some(proposal) = selected else {
                    record.compatibility_failures.push(format!(
                        "finite airborne proposal search selected no candidate: {}",
                        search
                            .unsupported_reason
                            .as_deref()
                            .unwrap_or("no admissible candidate")
                    ));
                    case.compatibility_failures.extend(
                        record
                            .compatibility_failures
                            .iter()
                            .map(|failure| format!("{role}: {failure}")),
                    );
                    case.captures.push(record);
                    continue;
                };
                record.selected_proposal = Some(proposal.clone());
                let audit_started = Instant::now();
                match audit_airborne_direct_proposal(
                    &context,
                    &capture.state,
                    &proposal,
                    MINIMUM_CLEARANCE_M,
                ) {
                    Ok(audit) => {
                        record.nominal_audit_wall_time_us = Some(elapsed_us(audit_started));
                        if !audit.passed {
                            record.compatibility_failures.extend(
                                audit
                                    .rejection_reasons
                                    .iter()
                                    .map(|reason| format!("nominal terrain audit: {reason}")),
                            );
                        }
                        record.nominal_audit = Some(audit);
                    }
                    Err(error) => {
                        record.nominal_audit_error = Some(format!("{error:#}"));
                        record
                            .compatibility_failures
                            .push("selected proposal terrain audit failed structurally".into());
                    }
                }
                record.terrain_twin = Some(audit_terrain_twin(
                    &context,
                    &capture.state,
                    &request,
                    &proposal,
                    &search,
                ));
                if let Some(twin) = &record.terrain_twin {
                    record
                        .compatibility_failures
                        .extend(twin.compatibility_failures.iter().cloned());
                    case.compatibility_failures.extend(
                        twin.compatibility_failures
                            .iter()
                            .map(|failure| format!("{role} terrain twin: {failure}")),
                    );
                }
                record.stitched_replay = Some(run_stitched_replay(
                    &context,
                    &capture.state,
                    &program,
                    &proposal,
                    record.nominal_audit.as_ref(),
                ));
                if let Some(stitched) = &record.stitched_replay {
                    if !stitched.suffix_terminal_reached_without_fallback {
                        record.compatibility_failures.push(
                            "selected updates did not reach terminal contact without fallback"
                                .into(),
                        );
                    }
                    if stitched.replay_actions_match != Some(true)
                        || stitched.source_replay_artifacts_match_official_replay != Some(true)
                        || stitched.source_replay_final_snapshot_matches_live_audit != Some(true)
                        || stitched.source_replay_contact_matches_live_audit != Some(true)
                        || stitched.replay_contact_time_tick_fuel_match != Some(true)
                        || stitched.replay_target_landing != Some(true)
                        || stitched.replay_checkpoint_events_absent != Some(true)
                    {
                        record
                            .compatibility_failures
                            .push("stitched whole-prefix ordinary replay parity failed".into());
                    }
                }
                case.compatibility_failures.extend(
                    record
                        .compatibility_failures
                        .iter()
                        .map(|failure| format!("{role}: {failure}")),
                );
            }
            Err(error) => {
                record.search_wall_time_us = Some(elapsed_us(started));
                record
                    .compatibility_failures
                    .push(format!("airborne proposal search failed: {error:#}"));
                case.compatibility_failures.extend(
                    record
                        .compatibility_failures
                        .iter()
                        .map(|failure| format!("{role}: {failure}")),
                );
            }
        }
        case.captures.push(record);
    }

    if case
        .baseline_execution
        .as_ref()
        .is_none_or(|execution| !execution.passed)
        || case.baseline_replay_matches_ordinary != Some(true)
    {
        case.compatibility_failures
            .push("source-rest baseline full execution/replay did not pass".into());
    }
    let captures_passed = case.captures.len() == 3
        && case.captures.iter().all(|capture| {
            capture
                .search
                .as_ref()
                .is_some_and(|search| search.selected.is_some())
                && capture
                    .nominal_audit
                    .as_ref()
                    .is_some_and(|audit| audit.passed)
                && capture.terrain_twin.as_ref().is_some_and(|twin| {
                    twin.search_identical == Some(true) && twin.audit_rejected == Some(true)
                })
                && capture.stitched_replay.as_ref().is_some_and(|stitched| {
                    stitched.suffix_terminal_reached_without_fallback
                        && stitched.replay_actions_match == Some(true)
                        && stitched.source_replay_artifacts_match_official_replay == Some(true)
                        && stitched.source_replay_final_snapshot_matches_live_audit == Some(true)
                        && stitched.source_replay_contact_matches_live_audit == Some(true)
                        && stitched.replay_contact_time_tick_fuel_match == Some(true)
                        && stitched.replay_target_landing == Some(true)
                        && stitched.replay_checkpoint_events_absent == Some(true)
                })
                && capture.compatibility_failures.is_empty()
        });
    if captures_passed && case.compatibility_failures.is_empty() {
        case.status = "pass".into();
    }
    finalize_case(&mut case)?;
    Ok(case)
}

pub(crate) fn collect_baseline_captures(
    context: &RunContext,
    program: &pd_core::FlightProgramV1,
) -> CaptureCollection {
    let mut collection = CaptureCollection {
        first: None,
        near_apex: None,
        last: None,
        trace: Vec::new(),
        failures: Vec::new(),
    };
    if let Err(error) = program.validate_against_context(context) {
        collection.failures.push(format!(
            "fresh baseline program/context binding invalid: {error}"
        ));
        return collection;
    }
    if !matches!(context.mission.goal, EvaluationGoal::LandingOnPad { .. })
        || context.mission.transfer_route.is_some()
    {
        collection
            .failures
            .push("canary requires an original route-free LandingOnPad mission".into());
        return collection;
    }
    let Some(coast_start) = program
        .updates
        .iter()
        .find(|update| update.phase == "ballistic_coast")
        .map(|update| update.physics_step)
    else {
        collection
            .failures
            .push("fresh baseline program has no ballistic_coast command".into());
        return collection;
    };
    let terminal_start = program.terminal_entry_physics_step;
    let Some(first_tick) = coast_start.checked_add(2) else {
        collection.failures.push("first coast tick overflow".into());
        return collection;
    };
    let Some(last_tick) = terminal_start.checked_sub(2) else {
        collection
            .failures
            .push("baseline terminal entry has no prior full coast hold".into());
        return collection;
    };
    if coast_start != program.source_handoff_physics_step
        || first_tick > last_tick
        || coast_start % 2 != 0
        || !terminal_start.is_multiple_of(2)
    {
        collection
            .failures
            .push("baseline coast/terminal boundary is not the supported aligned schedule".into());
        return collection;
    }
    let updates: BTreeMap<_, _> = program
        .updates
        .iter()
        .map(|update| (update.physics_step, update))
        .collect();
    if updates.len() != program.updates.len() {
        collection
            .failures
            .push("fresh baseline has duplicate command-update ticks".into());
        return collection;
    }
    let mut state = match SimulationState::new(context) {
        Ok(state) => state,
        Err(error) => {
            collection
                .failures
                .push(format!("source-rest prefix initialization failed: {error}"));
            return collection;
        }
    };
    let mut earliest_min_abs_vy = f64::INFINITY;
    while state.physics_step <= last_tick {
        let tick = state.physics_step;
        if tick == first_tick {
            collection.first = Some(CapturedLiveState {
                state: state.clone(),
            });
        }
        if tick >= first_tick && tick <= last_tick {
            let abs_vy = state.velocity_mps.y.abs();
            collection.trace.push(AirborneCoastBoundaryV1 {
                physics_step: tick,
                sim_time_s: state.sim_time_s,
                position_m: state.position_m,
                velocity_mps: state.velocity_mps,
                held_command: state.held_command,
            });
            if abs_vy < earliest_min_abs_vy {
                earliest_min_abs_vy = abs_vy;
                collection.near_apex = Some(CapturedLiveState {
                    state: state.clone(),
                });
            }
            if tick == last_tick {
                collection.last = Some(CapturedLiveState {
                    state: state.clone(),
                });
                break;
            }
        }
        let Some(update) = updates.get(&tick) else {
            collection.failures.push(format!(
                "fresh baseline has no command at global control boundary {tick}"
            ));
            break;
        };
        if tick >= terminal_start || update.phase == "terminal_bridge" {
            collection.failures.push(format!(
                "prefix encountered terminal command before final coast capture at tick {tick}"
            ));
            break;
        }
        state.set_command(update.command);
        for _ in 0..2 {
            let report = state.step_with_contact_report(context);
            if report.incoming_contact.is_some() || state.is_terminal() {
                collection.failures.push(format!(
                    "source prefix reached contact or terminal state before capture at physics tick {}",
                    state.physics_step
                ));
                return collection;
            }
        }
    }
    if collection.first.is_none()
        || collection.near_apex.is_none()
        || collection.last.is_none()
        || collection.trace.is_empty()
    {
        collection.failures.push(
            "baseline coast did not supply first, argmin-|vy|, and last capture states".into(),
        );
    }
    collection
}

fn audit_terrain_twin(
    context: &RunContext,
    live: &SimulationState,
    request: &WaypointDirectNominalDirectGenerationRequest,
    proposal: &AirborneDirectProposalV1,
    original_search: &AirborneDirectSearchV1,
) -> NominalAirborneTerrainTwinEvidenceV1 {
    let twin = match construct_terrain_twin(
        context,
        &request.scenario,
        &request.source_pad_id,
        live.position_m,
        proposal.peak_com_height_m,
    ) {
        Ok(twin) => twin,
        Err(error) => {
            return terrain_twin_failure(format!("could not construct terrain twin: {error:#}"));
        }
    };
    let mut evidence = twin.evidence;
    let deadline = original_search.absolute_deadline_physics_step;
    match evaluate_airborne_nominal_direct(&twin.context, live, deadline) {
        Ok(search) => {
            evidence.search_identical = Some(search == *original_search);
            evidence.search_identity_matches = Some(search.identity == original_search.identity);
            evidence.selected_commands_identical = Some(
                search.selected.as_ref().map(|selected| &selected.updates)
                    == Some(&proposal.updates),
            );
            if evidence.search_identical != Some(true)
                || evidence.search_identity_matches != Some(true)
                || evidence.selected_commands_identical != Some(true)
            {
                evidence.compatibility_failures.push(
                    "terrain twin changed the complete nominal search/selected commands".into(),
                );
            }
            evidence.twin_search = Some(search);
        }
        Err(error) => evidence
            .compatibility_failures
            .push(format!("terrain-twin nominal search failed: {error:#}")),
    }
    match audit_airborne_direct_proposal(&twin.context, live, proposal, MINIMUM_CLEARANCE_M) {
        Ok(audit) => {
            let actual_terrain_rejection = !audit.clearance_scan.all_airborne_states_passed
                || audit
                    .first_contact
                    .as_ref()
                    .is_some_and(|contact| contact.classification != "stable_touchdown_on_target");
            let rejected =
                !audit.passed && actual_terrain_rejection && audit.ordinary_neutral_parity;
            if !rejected {
                evidence.compatibility_failures.push(
                    "twin audit did not reject through actual clearance/contact with ordinary-neutral parity".into(),
                );
            }
            evidence.audit_rejected = Some(rejected);
            evidence.audit = Some(audit);
        }
        Err(error) => {
            evidence.audit_error = Some(format!("{error:#}"));
            evidence
                .compatibility_failures
                .push("terrain-twin actual audit failed structurally".into());
        }
    }
    evidence
}

fn terrain_twin_failure(reason: String) -> NominalAirborneTerrainTwinEvidenceV1 {
    NominalAirborneTerrainTwinEvidenceV1 {
        obstacle_x_min_m: None,
        obstacle_x_peak_m: None,
        obstacle_x_max_m: None,
        obstacle_peak_y_m: None,
        peak_above_nominal_future_peak_m: None,
        source_pad_unchanged: false,
        target_pad_unchanged: false,
        source_shelf_preserved: false,
        target_shelf_preserved: false,
        nonterrain_context_unchanged: false,
        obstruction_is_ahead_and_before_target_shelf: false,
        search_identical: None,
        search_identity_matches: None,
        selected_commands_identical: None,
        twin_search: None,
        audit: None,
        audit_error: Some(reason.clone()),
        audit_rejected: None,
        compatibility_failures: vec![reason],
    }
}

pub(crate) fn construct_terrain_twin(
    context: &RunContext,
    scenario: &ScenarioSpec,
    source_pad_id: &str,
    incoming_position: Vec2,
    future_peak_com_height_m: f64,
) -> Result<TerrainTwin> {
    let source_pad = context
        .world
        .landing_pad(source_pad_id)
        .context("source pad missing while constructing terrain twin")?;
    let target_pad = &context.target_pad;
    let margin = context.vehicle.geometry.hull_width_m * 0.5 + 2.0;
    let clear_left = (incoming_position.x + margin)
        .max(source_pad.center_x_m + source_pad.half_width_m() + margin);
    let clear_right = target_pad.center_x_m - target_pad.half_width_m() - margin;
    let open_span = clear_right - clear_left;
    if !future_peak_com_height_m.is_finite() || open_span <= 4.0 * margin {
        bail!("no bounded ahead-of-body, pre-target obstacle interval");
    }
    let center_x = (clear_left + clear_right) * 0.5;
    let half_width = (open_span * 0.25).min(10.0);
    if half_width <= margin {
        bail!("terrain-twin triangle is too narrow for the vehicle footprint");
    }
    let left_x = center_x - half_width;
    let right_x = center_x + half_width;
    let original_terrain = &context.world.terrain;
    let base_left_y = original_terrain.sample_height(left_x);
    let base_right_y = original_terrain.sample_height(right_x);
    let peak_y = (future_peak_com_height_m + 50.001).max(original_terrain.sample_height(center_x));
    if ![left_x, center_x, right_x, base_left_y, base_right_y, peak_y]
        .iter()
        .all(|value| value.is_finite())
        || peak_y - future_peak_com_height_m <= 50.0
    {
        bail!("terrain-twin triangle geometry is nonfinite or too short");
    }
    let TerrainDefinition::Heightfield { points_m } = original_terrain;
    let mut twin_points = points_m.clone();
    for point in &mut twin_points {
        if point.x >= left_x && point.x <= right_x {
            point.y = point.y.max(triangle_height(
                point.x,
                left_x,
                center_x,
                right_x,
                base_left_y,
                peak_y,
                base_right_y,
            ));
        }
    }
    insert_terrain_point(&mut twin_points, Vec2::new(left_x, base_left_y));
    insert_terrain_point(&mut twin_points, Vec2::new(center_x, peak_y));
    insert_terrain_point(&mut twin_points, Vec2::new(right_x, base_right_y));
    twin_points.sort_by(|left, right| left.x.total_cmp(&right.x));
    let terrain = TerrainDefinition::Heightfield {
        points_m: twin_points,
    };
    terrain.validate().map_err(anyhow::Error::msg)?;
    let mut twin_context = context.clone();
    twin_context.world.terrain = terrain;
    let source_pad_unchanged = twin_context.world.landing_pad(source_pad_id) == Some(source_pad);
    let target_pad_unchanged = twin_context.world.landing_pad(&target_pad.id) == Some(target_pad);
    let source_shelf_preserved = shelf_preserved(
        &context.world.terrain,
        &twin_context.world.terrain,
        source_pad,
    );
    let target_shelf_preserved = shelf_preserved(
        &context.world.terrain,
        &twin_context.world.terrain,
        target_pad,
    );
    let nonterrain_context_unchanged = context.sim == twin_context.sim
        && context.vehicle == twin_context.vehicle
        && context.initial_state == twin_context.initial_state
        && context.mission == twin_context.mission
        && context.world.gravity_mps2 == twin_context.world.gravity_mps2
        && context.world.landing_pads == twin_context.world.landing_pads
        && context.target_pad == twin_context.target_pad
        && context.scenario_id == twin_context.scenario_id
        && context.scenario_name == twin_context.scenario_name
        && context.scenario_seed == twin_context.scenario_seed
        && context.scenario_tags == twin_context.scenario_tags;
    let obstruction_is_ahead_and_before_target_shelf = left_x
        > incoming_position.x + margin - 1.0e-9
        && right_x < target_pad.center_x_m - target_pad.half_width_m() - margin + 1.0e-9
        && left_x > source_pad.center_x_m + source_pad.half_width_m();
    let evidence = NominalAirborneTerrainTwinEvidenceV1 {
        obstacle_x_min_m: Some(left_x),
        obstacle_x_peak_m: Some(center_x),
        obstacle_x_max_m: Some(right_x),
        obstacle_peak_y_m: Some(peak_y),
        peak_above_nominal_future_peak_m: Some(peak_y - future_peak_com_height_m),
        source_pad_unchanged,
        target_pad_unchanged,
        source_shelf_preserved,
        target_shelf_preserved,
        nonterrain_context_unchanged,
        obstruction_is_ahead_and_before_target_shelf,
        search_identical: None,
        search_identity_matches: None,
        selected_commands_identical: None,
        twin_search: None,
        audit: None,
        audit_error: None,
        audit_rejected: None,
        compatibility_failures: Vec::new(),
    };
    if !evidence.source_pad_unchanged
        || !evidence.target_pad_unchanged
        || !evidence.source_shelf_preserved
        || !evidence.target_shelf_preserved
        || !evidence.nonterrain_context_unchanged
        || !evidence.obstruction_is_ahead_and_before_target_shelf
    {
        bail!("terrain twin did not preserve pads/shelves/context or obstacle placement");
    }
    // The scenario argument ensures that the counterfactual context is derived
    // from the exact fresh case rather than from a modified serialized input.
    if scenario.world.landing_pads != context.world.landing_pads
        || scenario.world.gravity_mps2 != context.world.gravity_mps2
    {
        bail!("terrain twin scenario/context physical inputs diverged");
    }
    Ok(TerrainTwin {
        context: twin_context,
        evidence,
    })
}

fn triangle_height(
    x: f64,
    left_x: f64,
    center_x: f64,
    right_x: f64,
    left_y: f64,
    peak_y: f64,
    right_y: f64,
) -> f64 {
    if x <= center_x {
        let fraction = (x - left_x) / (center_x - left_x);
        left_y + fraction * (peak_y - left_y)
    } else {
        let fraction = (x - center_x) / (right_x - center_x);
        peak_y + fraction * (right_y - peak_y)
    }
}

fn insert_terrain_point(points: &mut Vec<Vec2>, point: Vec2) {
    match points.binary_search_by(|candidate| candidate.x.total_cmp(&point.x)) {
        Ok(index) => points[index] = point,
        Err(index) => points.insert(index, point),
    }
}

fn shelf_preserved(
    baseline: &TerrainDefinition,
    twin: &TerrainDefinition,
    pad: &pd_core::LandingPadSpec,
) -> bool {
    (0..=32).all(|sample| {
        let fraction = sample as f64 / 32.0;
        let x = pad.center_x_m - pad.half_width_m() + pad.width_m * fraction;
        baseline.sample_height(x).to_bits() == twin.sample_height(x).to_bits()
    })
}

pub(crate) fn run_stitched_replay(
    context: &RunContext,
    live: &SimulationState,
    program: &pd_core::FlightProgramV1,
    proposal: &AirborneDirectProposalV1,
    nominal_audit: Option<&AirborneDirectAuditV1>,
) -> NominalAirborneStitchedReplayV1 {
    let prefix: Vec<_> = program
        .updates
        .iter()
        .filter(|update| update.physics_step < live.physics_step)
        .cloned()
        .collect();
    let final_state = nominal_audit
        .map(|audit| audit.final_state.clone())
        .unwrap_or_else(|| SimulationStateSnapshotV1::from_state(live));
    let terminal_reached = nominal_audit.is_some_and(|audit| {
        audit.first_contact.is_some() && audit.final_state.end_reason != EndReason::Running
    });
    let suffix_updates: Vec<_> = if terminal_reached {
        proposal
            .updates
            .iter()
            .filter(|update| update.physics_step < final_state.physics_step)
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let mut stitched_updates = prefix.clone();
    stitched_updates.extend(suffix_updates.iter().cloned());
    let actions = absolute_actions(
        &stitched_updates,
        context.sim.physics_hz,
        context.sim.control_interval_steps(),
    )
    .unwrap_or_default();
    let contiguous = !actions.is_empty()
        && actions.iter().enumerate().all(|(index, action)| {
            action.controller_update_index == index as u64
                && action
                    .physics_step
                    .is_multiple_of(context.sim.control_interval_steps())
                && action.sim_time_s
                    == action.physics_step as f64 / f64::from(context.sim.physics_hz)
                && (index == 0
                    || action.physics_step
                        == actions[index - 1].physics_step + context.sim.control_interval_steps())
        })
        && actions
            .first()
            .is_some_and(|action| action.physics_step == 0)
        && suffix_updates
            .first()
            .is_some_and(|update| update.physics_step == live.physics_step);
    let no_checkpoint_goal = matches!(context.mission.goal, EvaluationGoal::LandingOnPad { .. })
        && context.mission.transfer_route.is_none();
    let source_result = if terminal_reached && contiguous && no_checkpoint_goal {
        Some(replay_stitched_from_source(context, &stitched_updates))
    } else {
        None
    };
    let (source_replay, replay, replay_error) = match source_result {
        Some(Ok(source)) => {
            match replay_simulation(
                context,
                "nominal_airborne_direct_stitched_replay",
                &source.actions,
            ) {
                Ok(replay) => (Some(source), Some(replay), None),
                Err(error) => (
                    Some(source),
                    None,
                    Some(format!("official replay failed: {error}")),
                ),
            }
        }
        Some(Err(error)) => (
            None,
            None,
            Some(format!("ordinary source replay failed: {error:#}")),
        ),
        None => (
            None,
            None,
            Some(
                "not run: selected updates did not reach terminal contact on the global clock"
                    .into(),
            ),
        ),
    };
    let replay_actions_match = replay
        .as_ref()
        .zip(source_replay.as_ref())
        .map(|(replay, source)| replay.actions == source.actions && replay.actions == actions);
    let source_replay_artifacts_match_official_replay = replay
        .as_ref()
        .zip(source_replay.as_ref())
        .map(|(replay, source)| replay.events == source.events && replay.samples == source.samples);
    let source_replay_final_state = source_replay
        .as_ref()
        .map(|source| source.final_state.clone());
    let source_replay_contact = source_replay
        .as_ref()
        .and_then(|source| source.incoming_contact.clone());
    let source_replay_events = source_replay
        .as_ref()
        .map_or_else(Vec::new, |source| source.events.clone());
    let source_replay_samples = source_replay
        .as_ref()
        .map_or_else(Vec::new, |source| source.samples.clone());
    let source_replay_final_snapshot_matches_live_audit = source_replay
        .as_ref()
        .zip(nominal_audit)
        .map(|(source, audit)| source.final_state == audit.final_state);
    let source_replay_contact_matches_live_audit =
        source_replay
            .as_ref()
            .zip(nominal_audit)
            .map(|(source, audit)| {
                source
                    .incoming_contact
                    .as_ref()
                    .is_some_and(|incoming| incoming_contact_matches_audit(incoming, audit))
            });
    let replay_contact_time_tick_fuel_match =
        replay
            .as_ref()
            .zip(source_replay.as_ref())
            .map(|(replay, source)| {
                replay.manifest.physics_steps == source.final_state.physics_step
                    && replay.manifest.sim_time_s == source.final_state.sim_time_s
                    && replay.manifest.summary.fuel_remaining_kg.to_bits()
                        == source.final_state.fuel_kg.to_bits()
                    && replay.manifest.physical_outcome == source.final_state.physical_outcome
                    && replay.manifest.mission_outcome == source.final_state.mission_outcome
                    && replay.manifest.end_reason == source.final_state.end_reason
            });
    let replay_target_landing =
        replay
            .as_ref()
            .zip(source_replay.as_ref())
            .map(|(replay, source)| {
                replay.manifest.physical_outcome == PhysicalOutcome::LandedOnTarget
                    && replay.manifest.mission_outcome == MissionOutcome::Success
                    && replay.manifest.end_reason == EndReason::TouchdownOnTarget
                    && source.incoming_contact.as_ref().is_some_and(|contact| {
                        matches!(
                            &contact.classification,
                            &pd_core::ContactClassification::StableTouchdown { on_target: true }
                        )
                    })
                    && nominal_audit.is_some_and(|audit| audit.safe_target_contact)
            });
    let replay_checkpoint_events_absent =
        replay
            .as_ref()
            .zip(source_replay.as_ref())
            .map(|(replay, source)| {
                source
                    .events
                    .iter()
                    .all(|event| !is_checkpoint_or_waypoint_event(&event.kind))
                    && replay
                        .events
                        .iter()
                        .all(|event| !is_checkpoint_or_waypoint_event(&event.kind))
                    && no_checkpoint_goal
            });
    NominalAirborneStitchedReplayV1 {
        capture_physics_step: live.physics_step,
        prefix_action_count: prefix.len(),
        candidate_update_count_supplied: suffix_updates.len(),
        candidate_updates: suffix_updates,
        actions,
        suffix_terminal_reached_without_fallback: terminal_reached,
        suffix_final_state: final_state,
        source_replay_final_state,
        source_replay_contact,
        source_replay_events,
        source_replay_samples,
        no_checkpoint_events: no_checkpoint_goal,
        contiguous_absolute_action_clock: contiguous,
        replay,
        replay_error,
        replay_actions_match,
        source_replay_artifacts_match_official_replay,
        source_replay_final_snapshot_matches_live_audit,
        source_replay_contact_matches_live_audit,
        replay_contact_time_tick_fuel_match,
        replay_target_landing,
        replay_checkpoint_events_absent,
    }
}

pub(crate) fn replay_stitched_from_source(
    context: &RunContext,
    updates: &[FlightProgramUpdateV1],
) -> Result<OrdinarySourceReplay> {
    let expected_actions = absolute_actions(
        updates,
        context.sim.physics_hz,
        context.sim.control_interval_steps(),
    )?;
    if expected_actions
        .first()
        .is_none_or(|action| action.physics_step != 0)
    {
        bail!("stitched source replay must start with the original source command");
    }
    let mut state = SimulationState::new(context)?;
    let sample_interval = context.sim.sample_interval_steps();
    let mut actions = Vec::with_capacity(updates.len());
    let mut events = Vec::new();
    let mut samples = Vec::new();
    let mut incoming_contact = None;
    let mut update_index = 0;
    maybe_push_sample(&mut samples, &state, context, sample_interval);
    while !state.is_terminal() {
        if state
            .physics_step
            .is_multiple_of(context.sim.control_interval_steps())
        {
            let Some(update) = updates.get(update_index) else {
                bail!(
                    "stitched source schedule ended before terminal state at global tick {}",
                    state.physics_step
                );
            };
            if update.physics_step != state.physics_step {
                bail!(
                    "stitched source action tick mismatch at {}: expected {}",
                    state.physics_step,
                    update.physics_step
                );
            }
            state.set_command(update.command);
            let action = expected_actions[update_index].clone();
            actions.push(action);
            events.push(EventRecord {
                sim_time_s: state.sim_time_s,
                physics_step: state.physics_step,
                kind: EventKind::ControllerUpdated,
                message: "controller_updated".into(),
            });
            update_index += 1;
        }
        let transition = state.step_with_contact_report(context);
        events.extend(transition.events);
        if let Some(contact) = transition.incoming_contact
            && incoming_contact.replace(contact).is_some()
        {
            bail!("stitched source run reported multiple first-contact states");
        }
        maybe_push_sample(&mut samples, &state, context, sample_interval);
    }
    if update_index != updates.len() {
        bail!(
            "stitched source action list contains {} updates after terminal contact",
            updates.len() - update_index
        );
    }
    if incoming_contact.is_none() {
        bail!("stitched source run ended without an incoming contact report");
    }
    Ok(OrdinarySourceReplay {
        final_state: SimulationStateSnapshotV1::from_state(&state),
        incoming_contact,
        actions,
        events,
        samples,
    })
}

fn maybe_push_sample(
    samples: &mut Vec<SampleRecord>,
    state: &SimulationState,
    context: &RunContext,
    sample_interval_steps: Option<u64>,
) {
    let Some(interval) = sample_interval_steps else {
        return;
    };
    if !state.physics_step.is_multiple_of(interval) && !state.is_terminal() {
        return;
    }
    if samples
        .last()
        .is_some_and(|sample| sample.physics_step == state.physics_step)
    {
        return;
    }
    samples.push(SampleRecord {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        observation: state.build_observation(context),
        held_command: state.held_command,
    });
}

fn is_checkpoint_or_waypoint_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::CheckpointSatisfied
            | EventKind::CheckpointFailed
            | EventKind::WaypointHandoffSatisfied
            | EventKind::WaypointHandoffFailed
    )
}

fn incoming_contact_matches_audit(
    incoming: &IncomingContactV1,
    audited: &AirborneDirectAuditV1,
) -> bool {
    audited.incoming_contact.as_ref() == Some(incoming)
}

fn absolute_actions(
    updates: &[FlightProgramUpdateV1],
    physics_hz: u32,
    control_interval_steps: u64,
) -> Result<Vec<ActionLogEntry>> {
    let mut actions = Vec::with_capacity(updates.len());
    let mut previous = None;
    for (ordinal, update) in updates.iter().enumerate() {
        if update.command != update.command.clamped()
            || !update.command.throttle_frac.is_finite()
            || !update.command.target_attitude_rad.is_finite()
        {
            bail!(
                "stitched update has invalid command at tick {}",
                update.physics_step
            );
        }
        if !update.physics_step.is_multiple_of(control_interval_steps)
            || previous.is_some_and(|tick| tick + control_interval_steps != update.physics_step)
        {
            bail!("stitched updates contain duplicate or missing global control tick");
        }
        actions.push(ActionLogEntry {
            sim_time_s: update.physics_step as f64 / f64::from(physics_hz),
            physics_step: update.physics_step,
            controller_update_index: ordinal as u64,
            command: update.command,
        });
        previous = Some(update.physics_step);
    }
    Ok(actions)
}

fn finalize_case(case: &mut NominalAirborneDirectCanaryCaseV1) -> Result<()> {
    if !case.compatibility_failures.is_empty() {
        case.status = "finite_compatibility_failure".into();
    }
    case.identity = case_identity(case)?;
    Ok(())
}

fn case_identity(case: &NominalAirborneDirectCanaryCaseV1) -> Result<String> {
    let mut canonical = case.clone();
    canonical.identity.clear();
    if let Some(compute) = &mut canonical.baseline_compute {
        compute.generation_wall_time_us = 0;
        compute.selected_verification_wall_time_us = 0;
        compute.ordinary_execution_wall_time_us = 0;
        compute.action_replay_wall_time_us = 0;
        compute.artifact_writing_wall_time_us = 0;
    }
    for capture in &mut canonical.captures {
        capture.search_wall_time_us = None;
        capture.nominal_audit_wall_time_us = None;
    }
    nominal_direct_flight_identity(&canonical)
}

fn summary_identity(summary: &NominalAirborneDirectCanarySummaryV1) -> Result<String> {
    let mut canonical = summary.clone();
    canonical.identity.clear();
    nominal_direct_flight_identity(&canonical)
}

fn elapsed_us(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn failed_twin_geometry_roundtrips_without_nonfinite_placeholders() {
        let failure = terrain_twin_failure("no ahead-of-body interval".into());
        let mut value = serde_json::to_value(&failure).unwrap();
        let restored: NominalAirborneTerrainTwinEvidenceV1 =
            serde_json::from_value(value.clone()).unwrap();
        assert_eq!(restored, failure);
        value["unapproved_fallback"] = serde_json::json!(true);
        assert!(serde_json::from_value::<NominalAirborneTerrainTwinEvidenceV1>(value).is_err());
    }

    #[test]
    fn incoming_contact_comparison_rejects_changes_outside_kinematic_subset() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let (_, request) = load_nominal_direct_operational_fresh_inputs(repo)
            .unwrap()
            .remove(0);
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let mut live = SimulationState::new(&context).unwrap();
        // Synthetic structure test, not a measured airborne acceptance state.
        live.physics_step = 120;
        live.sim_time_s = 1.0;
        live.position_m = Vec2::new(context.target_pad.center_x_m - 400.0, 200.0);
        live.velocity_mps = Vec2::new(25.0, 10.0);
        live.fuel_kg -= 10.0;
        let proposal = evaluate_airborne_nominal_direct(&context, &live, 9600)
            .unwrap()
            .selected
            .unwrap();
        let audit = audit_airborne_direct_proposal(&context, &live, &proposal, 0.0).unwrap();
        let mut incoming = audit.incoming_contact.clone().unwrap();
        assert!(incoming_contact_matches_audit(&incoming, &audit));
        incoming.state.min_hull_clearance_m -= 1.0;
        assert!(!incoming_contact_matches_audit(&incoming, &audit));
        let mut incoming = audit.incoming_contact.clone().unwrap();
        incoming.state.held_command.target_attitude_rad += 0.001;
        assert!(!incoming_contact_matches_audit(&incoming, &audit));
    }

    #[test]
    fn near_apex_rule_uses_observed_absolute_vy_with_earliest_tie() {
        let observations: [(u64, f64); 4] = [(10, 4.0), (12, 0.25), (14, -0.25), (16, -3.0)];
        let chosen = observations
            .iter()
            .min_by(|left, right| left.1.abs().total_cmp(&right.1.abs()))
            .unwrap();
        assert_eq!(chosen.0, 12);
    }

    #[test]
    fn stitched_action_helper_preserves_absolute_ticks_and_contiguous_ordinals() {
        let updates = vec![
            FlightProgramUpdateV1 {
                physics_step: 0,
                phase: "source".into(),
                command: Command::idle(),
            },
            FlightProgramUpdateV1 {
                physics_step: 2,
                phase: "ballistic_coast".into(),
                command: Command {
                    throttle_frac: 0.0,
                    target_attitude_rad: 0.1,
                },
            },
        ];
        let actions = absolute_actions(&updates, 120, 2).unwrap();
        assert_eq!(actions[0].physics_step, 0);
        assert_eq!(actions[1].physics_step, 2);
        assert_eq!(actions[1].controller_update_index, 1);
        assert_eq!(actions[1].sim_time_s, 2.0 / 120.0);
        let gap = vec![
            updates[0].clone(),
            FlightProgramUpdateV1 {
                physics_step: 4,
                phase: "coast".into(),
                command: Command::idle(),
            },
        ];
        assert!(absolute_actions(&gap, 120, 2).is_err());
    }

    #[test]
    fn terrain_twin_preserves_both_pad_shelves_and_all_other_context_fields() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let (_, request) = load_nominal_direct_operational_fresh_inputs(repo)
            .unwrap()
            .remove(0);
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let source = context.world.landing_pad(&request.source_pad_id).unwrap();
        let target = &context.target_pad;
        let incoming = Vec2::new(
            source.center_x_m + source.half_width_m() + 5.0,
            context.initial_state.position_m.y,
        );
        let twin = construct_terrain_twin(
            &context,
            &request.scenario,
            &request.source_pad_id,
            incoming,
            100.0,
        )
        .unwrap();
        assert!(twin.evidence.source_shelf_preserved);
        assert!(twin.evidence.target_shelf_preserved);
        assert!(twin.evidence.source_pad_unchanged && twin.evidence.target_pad_unchanged);
        assert!(twin.evidence.nonterrain_context_unchanged);
        assert!(twin.evidence.obstruction_is_ahead_and_before_target_shelf);
        assert!(twin.evidence.peak_above_nominal_future_peak_m.unwrap() > 50.0);
        assert_eq!(
            context.world.terrain.sample_height(target.center_x_m),
            twin.context.world.terrain.sample_height(target.center_x_m)
        );
    }

    #[test]
    fn create_only_artifact_writer_refuses_replacement() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "pd-eval-airborne-canary-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("summary.json");
        write_create_only(&path, &serde_json::json!({"version": 1})).unwrap();
        assert!(write_create_only(&path, &serde_json::json!({"version": 2})).is_err());
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
