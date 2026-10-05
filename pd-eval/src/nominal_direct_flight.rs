//! Explicit nominal direct-flight mode. This does not replace planner V1.
//!
//! The research generator remains the backend, including its original-policy
//! comparison work. A decision carries the accepted executable program; no
//! chord-plan/controller compatibility is inferred from a Direct label.

use std::{path::Path, time::Instant};

use anyhow::{Context, Result, bail};
use pd_control::{ControlledRunArtifacts, run_flight_program};
use pd_core::{
    EvaluationGoal, FlightProgramBindingV1, FlightProgramUpdateV1, FlightProgramV1, MissionOutcome,
    PhysicalOutcome, RunContext, RunManifest, ScenarioSpec, Vec2, replay_simulation,
};
use serde::{Deserialize, Serialize};

use crate::{
    BodyAwareTerminalCaseArtifactV1, BodyAwareTerminalPolicyV1, BodyAwareTerminalVerificationV1,
    BodyAwareTerminalWitnessV1, WaypointDirectNominalDirectGenerationPolicyV1,
    WaypointDirectNominalDirectGenerationRequest, evaluate_waypoint_direct_body_aware_terminal,
    validate_waypoint_direct_nominal_direct_generation_request, verify_body_aware_terminal_witness,
};

pub(crate) use crate::evidence_io::{
    reserve_output_root, write_json_create_only as write_create_only,
};

pub const NOMINAL_DIRECT_FLIGHT_PROTOCOL: &str =
    "docs/nominal_direct_flight_integration_protocol.md";

const NON_CLAIMS: [&str; 5] = [
    "Opt-in nominal ballistic-direct flight mode; production V1 planner and built-in defaults are unchanged.",
    "Finite Unknown is not physical impossibility or proof of waypoint necessity; no fallback or waypoint search runs here.",
    "Pointwise discrete body safety is not swept-path proof or perturbation robustness; small contact margins remain visible.",
    "The unchanged research backend includes original-policy comparison work; measured cost is not optimized or real-time planner authority.",
    "No arbitrary incoming waypoint state, waypoint composition, policy retuning, or default promotion is claimed.",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum NominalDirectFlightDecisionV1 {
    Direct {
        generation_identity: String,
        selected_row_index: usize,
        program_identity: String,
        program: Box<FlightProgramV1>,
    },
    Unknown {
        generation_identity: String,
        reason: String,
    },
    Unsupported {
        reason: String,
    },
    Invalid {
        reason: String,
    },
}

impl NominalDirectFlightDecisionV1 {
    pub fn status(&self) -> &'static str {
        match self {
            Self::Direct { .. } => "direct",
            Self::Unknown { .. } => "unknown",
            Self::Unsupported { .. } => "unsupported",
            Self::Invalid { .. } => "invalid",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalDirectFlightPreflightV1 {
    pub supported: bool,
    pub rejection: Option<NominalDirectFlightDecisionV1>,
    pub simulation_created: bool,
}

/// Cheap, typed input rejection. Never run a candidate or construct physical
/// simulation state, and never classify a valid finite failure as Unsupported.
pub fn preflight_nominal_direct_flight(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
) -> NominalDirectFlightPreflightV1 {
    let invalid = |reason: String| NominalDirectFlightDecisionV1::Invalid { reason };
    let unsupported = |reason: &str| NominalDirectFlightDecisionV1::Unsupported {
        reason: reason.to_owned(),
    };
    let check = || -> std::result::Result<(), NominalDirectFlightDecisionV1> {
        request.scenario.validate().map_err(invalid)?;
        if request.probe_id.trim().is_empty()
            || request.source_pad_id.trim().is_empty()
            || request.target_pad_id.trim().is_empty()
            || request.source_pad_id == request.target_pad_id
        {
            return Err(invalid(
                "probe and distinct source/target pad IDs are required".into(),
            ));
        }
        let scenario = &request.scenario;
        for pad_id in [&request.source_pad_id, &request.target_pad_id] {
            if scenario
                .world
                .landing_pads
                .iter()
                .filter(|pad| &pad.id == pad_id)
                .count()
                != 1
            {
                return Err(invalid("requested pad must resolve exactly once".into()));
            }
        }
        let source = scenario
            .world
            .landing_pad(&request.source_pad_id)
            .ok_or_else(|| invalid("requested source pad is missing".into()))?;
        let target = scenario
            .world
            .landing_pad(&request.target_pad_id)
            .ok_or_else(|| invalid("requested target pad is missing".into()))?;
        if scenario.mission.goal.target_pad_id() != request.target_pad_id {
            return Err(invalid(
                "requested target differs from the mission target".into(),
            ));
        }
        if let Some(route) = &scenario.mission.transfer_route {
            if route.source_pad_id != request.source_pad_id
                || route.target_pad_id != request.target_pad_id
            {
                return Err(invalid(
                    "requested pad IDs differ from the authored route".into(),
                ));
            }
            if !route.waypoints.is_empty() {
                return Err(unsupported(
                    "authored operational waypoints are unsupported",
                ));
            }
        }
        if !matches!(scenario.mission.goal, EvaluationGoal::LandingOnPad { .. }) {
            return Err(unsupported("only landing_on_pad missions are supported"));
        }
        if request.policy != WaypointDirectNominalDirectGenerationPolicyV1::default()
            || *policy != BodyAwareTerminalPolicyV1::default()
        {
            return Err(unsupported(
                "unsupported or tuned generation/terminal policy",
            ));
        }
        if scenario.sim.physics_hz != request.policy.physics_hz
            || scenario.sim.controller_hz != request.policy.controller_hz
            || scenario.world.gravity_mps2 != request.policy.analytical_policy.gravity_mps2
        {
            return Err(unsupported(
                "physics rate, controller rate or gravity is unsupported",
            ));
        }
        let supported = pd_plan::conservative_ballistic_bridge::load_embedded_fixture_v2().vehicle;
        let supported_vehicle = pd_core::VehicleSpec {
            geometry: pd_core::VehicleGeometry {
                hull_width_m: supported.geometry.hull_width_m,
                hull_height_m: supported.geometry.hull_height_m,
                touchdown_half_span_m: supported.geometry.touchdown_half_span_m,
                touchdown_base_offset_m: supported.geometry.touchdown_base_offset_m,
            },
            dry_mass_kg: supported.dry_mass_kg,
            initial_fuel_kg: supported.initial_fuel_kg,
            max_fuel_kg: supported.max_fuel_kg,
            max_thrust_n: supported.max_thrust_n,
            max_fuel_burn_kgps: supported.max_fuel_burn_kgps,
            min_throttle_frac: supported.min_throttle_frac,
            max_rotation_rate_radps: supported.max_rotation_rate_radps,
            safe_touchdown_normal_speed_mps: supported.safe_touchdown_normal_speed_mps,
            safe_touchdown_tangential_speed_mps: supported.safe_touchdown_tangential_speed_mps,
            safe_touchdown_attitude_error_rad: supported.safe_touchdown_attitude_error_rad,
            safe_touchdown_angular_rate_radps: supported.safe_touchdown_angular_rate_radps,
        };
        if scenario.vehicle != supported_vehicle {
            return Err(unsupported(
                "full VehicleSpec differs from the supported vehicle",
            ));
        }
        if scenario.initial_state.attitude_rad != 0.0
            || scenario.initial_state.angular_rate_radps != 0.0
            || scenario.initial_state.velocity_mps != Vec2::new(0.0, 0.0)
            || (scenario.initial_state.position_m.x - source.center_x_m).abs() > 1.0e-9
            || (scenario.initial_state.position_m.y
                - source.surface_y_m
                - scenario.vehicle.geometry.touchdown_base_offset_m)
                .abs()
                > 1.0e-9
        {
            return Err(unsupported(
                "source must be upright source-pad rest with zero angular rate",
            ));
        }
        if target.center_x_m <= source.center_x_m {
            return Err(unsupported(
                "only forward source-to-target geometry is supported",
            ));
        }
        let flat =
            |pad: &pd_core::LandingPadSpec| {
                let left = pad.center_x_m - pad.half_width_m();
                let right = pad.center_x_m + pad.half_width_m();
                scenario.world.terrain.sample_height_strict(left).ok() == Some(pad.surface_y_m)
                    && scenario.world.terrain.sample_height_strict(right).ok()
                        == Some(pad.surface_y_m)
                    && scenario.world.terrain.points().iter().all(|point| {
                        point.x < left || point.x > right || point.y == pad.surface_y_m
                    })
            };
        let half_width = (scenario.vehicle.geometry.hull_width_m * 0.5)
            .max(scenario.vehicle.geometry.touchdown_half_span_m);
        if !flat(source) || half_width > source.half_width_m() || !flat(target) {
            return Err(unsupported(
                "pads must be flat in-domain shelves with a supported source footprint",
            ));
        }
        // Guard against this typed front-end drifting from the original backend.
        // Backend disagreement is an integration error, not a finite Unknown.
        validate_waypoint_direct_nominal_direct_generation_request(request)
            .map_err(|error| invalid(format!("backend input-contract disagreement: {error}")))?;
        Ok(())
    };
    let rejection = check().err();
    NominalDirectFlightPreflightV1 {
        supported: rejection.is_none(),
        rejection,
        simulation_created: false,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightComputeV1 {
    pub generation_wall_time_us: u64,
    pub selected_verification_wall_time_us: u64,
    pub ordinary_execution_wall_time_us: u64,
    pub action_replay_wall_time_us: u64,
    pub artifact_writing_wall_time_us: u64,
    pub basis_count: usize,
    pub source_rows_predeclared: usize,
    pub source_schedules_available: usize,
    pub terminal_attempts: usize,
    pub accepted_witnesses: usize,
    /// Subtotal from terminal attempts with verifier evidence, ordinary + neutral.
    pub terminal_attempt_verification_physics_ticks: u64,
    pub selected_witness_verification_physics_ticks: u64,
    pub ordinary_execution_physics_ticks: u64,
    pub action_replay_physics_ticks: u64,
    /// Original source fitting, comparison, and reference pose probes are not
    /// instrumented in this adapter. Do not mislabel the subtotal as total work.
    pub total_generation_physics_ticks: Option<u64>,
    pub accounting_scope: String,
}

#[derive(Clone, Debug)]
pub struct NominalDirectFlightEvaluationV1 {
    pub decision: NominalDirectFlightDecisionV1,
    pub generation: Option<BodyAwareTerminalCaseArtifactV1>,
    pub compute: NominalDirectFlightComputeV1,
}

pub fn nominal_direct_flight_identity<T: Serialize>(value: &T) -> Result<String> {
    Ok(format!(
        "fnv1a64:{:016x}",
        crate::runtime::fnv1a64(&serde_json::to_vec(value)?)
    ))
}

pub(crate) fn program_from_witness(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
    witness: &BodyAwareTerminalWitnessV1,
) -> Result<FlightProgramV1> {
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    let source_end = request
        .policy
        .launch_upright_ticks
        .checked_add(request.policy.launch_tilt_ticks)
        .and_then(|launch| launch.checked_add(witness.source_bridge_tick_count))
        .context("source program duration overflow")?;
    let program = FlightProgramV1 {
        schema_version: 1,
        binding: FlightProgramBindingV1::from_context(&context),
        source_pad_id: request.source_pad_id.clone(),
        target_pad_id: request.target_pad_id.clone(),
        generation_policy_identity: nominal_direct_flight_identity(&request.policy)?,
        terminal_policy_identity: nominal_direct_flight_identity(policy)?,
        witness_identity: witness.identity.clone(),
        source_handoff_physics_step: source_end,
        terminal_entry_physics_step: witness.terminal_entry.physics_step,
        planned_end_physics_step: witness.planned_physics_tick_count,
        expected_contact_physics_step: witness.verification.physics_ticks_advanced,
        updates: witness
            .commands
            .iter()
            .map(|update| {
                Ok(FlightProgramUpdateV1 {
                    physics_step: update
                        .physics_step
                        .checked_sub(1)
                        .context("post-step command index is zero")?,
                    phase: update.phase.clone(),
                    command: update.command,
                })
            })
            .collect::<Result<Vec<_>>>()?,
    };
    program
        .validate_against_context(&context)
        .map_err(anyhow::Error::msg)?;
    Ok(program)
}

/// Generate from physical inputs alone. Historical summaries are not inputs.
pub fn evaluate_nominal_direct_flight(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
) -> Result<NominalDirectFlightEvaluationV1> {
    let mut compute = NominalDirectFlightComputeV1 {
        accounting_scope: "Generation includes unchanged original-policy comparison flights, source fits and terminal-family validation. Total ticks are uninstrumented; terminal verifier ticks are a subtotal only. Selected verification additionally regenerates the original source family; its recorded ticks count only the complete ordinary/neutral witness replay.".into(),
        ..Default::default()
    };
    if let Some(decision) = preflight_nominal_direct_flight(request, policy).rejection {
        return Ok(NominalDirectFlightEvaluationV1 {
            decision,
            generation: None,
            compute,
        });
    }
    let start = Instant::now();
    let generation = evaluate_waypoint_direct_body_aware_terminal(request, policy)?;
    compute.generation_wall_time_us = elapsed_us(start);
    compute.basis_count = request.policy.maximum_basis_candidates;
    compute.source_rows_predeclared = generation.rows.len();
    compute.source_schedules_available = generation.baseline_scheduled_count;
    compute.accepted_witnesses = generation.accepted_witness_count;
    compute.terminal_attempts = generation
        .rows
        .iter()
        .flat_map(|row| &row.attempts)
        .filter(|attempt| attempt.status != "not_attempted_after_accepted_witness")
        .count();
    compute.terminal_attempt_verification_physics_ticks = generation
        .rows
        .iter()
        .flat_map(|row| &row.attempts)
        .filter_map(|attempt| attempt.verification.as_ref())
        .map(|v| v.physics_ticks_advanced * 2)
        .sum();
    if generation.rows.len() != request.policy.maximum_variants
        || generation.rows.iter().enumerate().any(|(i, row)| {
            row.row_index != i
                || row.accepted != row.witness.is_some()
                || row
                    .witness
                    .as_ref()
                    .is_some_and(|w| !w.verification.passed || w.row_index != i)
        })
    {
        bail!("generator returned an incomplete or inconsistent finite ledger");
    }
    let selected = generation
        .rows
        .iter()
        .filter_map(|row| row.witness.as_ref())
        .min_by(|a, b| {
            a.planned_physics_tick_count
                .cmp(&b.planned_physics_tick_count)
                .then_with(|| a.identity.cmp(&b.identity))
        });
    if generation.selected_row_index != selected.map(|w| w.row_index)
        || generation.accepted_witness_count
            != generation.rows.iter().filter(|r| r.accepted).count()
        || generation.passed != selected.is_some()
    {
        bail!("generator acceptance count or accepted-only ranked selection is inconsistent");
    }
    let decision = if let Some(witness) = selected {
        let program = program_from_witness(request, policy, witness)?;
        NominalDirectFlightDecisionV1::Direct {
            generation_identity: generation.identity.clone(),
            selected_row_index: witness.row_index,
            program_identity: nominal_direct_flight_identity(&program)?,
            program: Box::new(program),
        }
    } else {
        NominalDirectFlightDecisionV1::Unknown {
            generation_identity: generation.identity.clone(),
            reason: "no complete accepted ballistic-direct witness in the declared finite family; waypoint necessity is not established".into(),
        }
    };
    Ok(NominalDirectFlightEvaluationV1 {
        decision,
        generation: Some(generation),
        compute,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightExecutionEvidenceV1 {
    pub program_identity: String,
    pub witness_identity: String,
    pub safety_audit: BodyAwareTerminalVerificationV1,
    pub manifest: RunManifest,
    pub ordinary_run_identity: String,
    pub exact_command_and_clock_parity: bool,
    pub exact_contact_tick_and_fuel_parity: bool,
    pub ordinary_action_replay_parity: bool,
    pub safe_target_landing: bool,
    pub passed: bool,
}

/// Verify the supplied witness, bind every program field/command to it, then
/// run through the ordinary controller path. A self-rehashed altered program
/// cannot obtain acceptance merely by passing neutral structural validation.
pub fn execute_nominal_direct_flight_program(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
    witness: &BodyAwareTerminalWitnessV1,
    program: &FlightProgramV1,
    compute: &mut NominalDirectFlightComputeV1,
) -> Result<(
    NominalDirectFlightExecutionEvidenceV1,
    ControlledRunArtifacts,
)> {
    if let Some(rejection) = preflight_nominal_direct_flight(request, policy).rejection {
        bail!("cannot execute {} request", rejection.status());
    }
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    program
        .validate_against_context(&context)
        .map_err(anyhow::Error::msg)?;
    if *program != program_from_witness(request, policy, witness)? {
        bail!("program differs from the complete selected witness payload/bindings");
    }
    let started = Instant::now();
    let safety = verify_body_aware_terminal_witness(request, policy, witness)?;
    compute.selected_verification_wall_time_us = elapsed_us(started);
    compute.selected_witness_verification_physics_ticks = safety.physics_ticks_advanced * 2;
    if !safety.passed || safety != witness.verification {
        bail!("selected witness fails independent full safety verification");
    }
    let started = Instant::now();
    let artifacts = run_flight_program(&context, program)?;
    compute.ordinary_execution_wall_time_us = elapsed_us(started);
    compute.ordinary_execution_physics_ticks = artifacts.run.manifest.physics_steps;
    let started = Instant::now();
    let replay = replay_simulation(
        &context,
        &artifacts.run.manifest.controller_id,
        &artifacts.run.actions,
    )?;
    compute.action_replay_wall_time_us = elapsed_us(started);
    compute.action_replay_physics_ticks = replay.manifest.physics_steps;
    let commands_match = artifacts.run.actions.len() == program.updates.len()
        && artifacts
            .run
            .actions
            .iter()
            .zip(&program.updates)
            .enumerate()
            .all(|(i, (a, u))| {
                a.physics_step == u.physics_step
                    && a.command == u.command
                    && a.controller_update_index == i as u64
                    && a.sim_time_s == a.physics_step as f64 / f64::from(context.sim.physics_hz)
            });
    let contact = safety
        .first_contact
        .as_ref()
        .context("accepted witness lacks contact evidence")?;
    let contact_match = artifacts.run.manifest.physics_steps == contact.state.physics_step
        && artifacts.run.manifest.sim_time_s == contact.state.sim_time_s
        && artifacts.run.manifest.summary.fuel_remaining_kg == contact.state.fuel_kg;
    let safe_landing = artifacts.run.manifest.physical_outcome == PhysicalOutcome::LandedOnTarget
        && artifacts.run.manifest.mission_outcome == MissionOutcome::Success
        && contact.classification == "stable_touchdown_on_target";
    let replay_matches = replay == artifacts.run;
    let evidence = NominalDirectFlightExecutionEvidenceV1 {
        program_identity: nominal_direct_flight_identity(program)?,
        witness_identity: witness.identity.clone(),
        safety_audit: safety,
        manifest: artifacts.run.manifest.clone(),
        ordinary_run_identity: nominal_direct_flight_identity(&artifacts.run)?,
        exact_command_and_clock_parity: commands_match,
        exact_contact_tick_and_fuel_parity: contact_match,
        ordinary_action_replay_parity: replay_matches,
        safe_target_landing: safe_landing,
        passed: commands_match && contact_match && replay_matches && safe_landing,
    };
    if !evidence.passed {
        bail!("ordinary flight differs from the independently verified command/contact contract");
    }
    Ok((evidence, artifacts))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NominalDirectFlightArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub request_identity: String,
    pub decision: NominalDirectFlightDecisionV1,
    pub execution: Option<NominalDirectFlightExecutionEvidenceV1>,
    pub compute: NominalDirectFlightComputeV1,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

pub fn nominal_direct_flight_artifact_identity(
    artifact: &NominalDirectFlightArtifactV1,
) -> Result<String> {
    // Keep deterministic counters but exclude all observational timings.
    let mut canonical = artifact.clone();
    canonical.identity.clear();
    canonical.compute.generation_wall_time_us = 0;
    canonical.compute.selected_verification_wall_time_us = 0;
    canonical.compute.ordinary_execution_wall_time_us = 0;
    canonical.compute.action_replay_wall_time_us = 0;
    canonical.compute.artifact_writing_wall_time_us = 0;
    nominal_direct_flight_identity(&canonical)
}

/// New roots only: existing partial or complete evidence is never overwritten.
pub fn run_nominal_direct_flight(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
    output_dir: &Path,
) -> Result<NominalDirectFlightArtifactV1> {
    reserve_output_root(output_dir)?;
    let writing = Instant::now();
    write_create_only(&output_dir.join("request.json"), request)?;
    write_create_only(&output_dir.join("scenario.json"), &request.scenario)?;
    write_create_only(&output_dir.join("terminal_policy.json"), policy)?;
    let input_writing_us = elapsed_us(writing);
    let evaluation = match evaluate_nominal_direct_flight(request, policy) {
        Ok(evaluation) => evaluation,
        Err(error) => {
            write_create_only(
                &output_dir.join("generation_error.json"),
                &format!("{error:#}"),
            )?;
            return Err(error);
        }
    };
    let mut artifact = NominalDirectFlightArtifactV1 {
        schema_id: "nominal_direct_flight_v1".into(),
        schema_version: 1,
        request_identity: nominal_direct_flight_identity(request)?,
        decision: evaluation.decision,
        execution: None,
        compute: evaluation.compute,
        scope_non_claims: NON_CLAIMS.iter().map(|s| (*s).into()).collect(),
        identity: String::new(),
    };
    let writing = Instant::now();
    write_create_only(&output_dir.join("decision.json"), &artifact.decision)?;
    if let Some(generation) = &evaluation.generation {
        write_create_only(&output_dir.join("generation.json"), generation)?;
    }
    if let NominalDirectFlightDecisionV1::Direct { program, .. } = &artifact.decision {
        write_create_only(&output_dir.join("program.json"), program)?;
    }
    // Preserve the complete proposed program and witness even if execution
    // fails. An empty failed root would hide the decisive integration boundary.
    if let NominalDirectFlightDecisionV1::Direct {
        selected_row_index, ..
    } = &artifact.decision
    {
        let witness = evaluation
            .generation
            .as_ref()
            .context("Direct has no generated ledger")?
            .rows
            .get(*selected_row_index)
            .and_then(|row| row.witness.as_ref())
            .context("Direct has no accepted selected witness")?;
        write_create_only(&output_dir.join("witness.json"), witness)?;
    }
    artifact.compute.artifact_writing_wall_time_us = input_writing_us + elapsed_us(writing);
    let ordinary = if let NominalDirectFlightDecisionV1::Direct {
        selected_row_index,
        program,
        ..
    } = &artifact.decision
    {
        let witness = evaluation
            .generation
            .as_ref()
            .context("Direct has no generated ledger")?
            .rows
            .get(*selected_row_index)
            .and_then(|row| row.witness.as_ref())
            .context("Direct has no accepted selected witness")?;
        match execute_nominal_direct_flight_program(
            request,
            policy,
            witness,
            program,
            &mut artifact.compute,
        ) {
            Ok((evidence, artifacts)) => {
                artifact.execution = Some(evidence);
                Some(artifacts)
            }
            Err(error) => {
                write_create_only(
                    &output_dir.join("execution_error.json"),
                    &format!("{error:#}"),
                )?;
                return Err(error);
            }
        }
    } else {
        None
    };
    let writing = Instant::now();
    if let Some(evidence) = &artifact.execution {
        write_create_only(
            &output_dir.join("safety_audit.json"),
            &evidence.safety_audit,
        )?;
    }
    if let Some(ordinary) = &ordinary {
        write_ordinary_bundle(output_dir, &request.scenario, ordinary)?;
    }
    artifact.compute.artifact_writing_wall_time_us += elapsed_us(writing);
    artifact.identity = nominal_direct_flight_artifact_identity(&artifact)?;
    write_create_only(&output_dir.join("performance.json"), &artifact.compute)?;
    write_create_only(&output_dir.join("summary.json"), &artifact)?;
    Ok(artifact)
}

fn write_ordinary_bundle(
    output_dir: &Path,
    scenario: &ScenarioSpec,
    artifacts: &ControlledRunArtifacts,
) -> Result<()> {
    write_create_only(&output_dir.join("manifest.json"), &artifacts.run.manifest)?;
    write_create_only(&output_dir.join("actions.json"), &artifacts.run.actions)?;
    write_create_only(&output_dir.join("events.json"), &artifacts.run.events)?;
    write_create_only(&output_dir.join("samples.json"), &artifacts.run.samples)?;
    write_create_only(
        &output_dir.join("controller_updates.json"),
        &artifacts.controller_updates,
    )?;
    write_create_only(
        &output_dir.join("run_performance.json"),
        &artifacts.performance,
    )?;
    // None means no fake controller-registry spec and no misleading V1 chord plan.
    pd_report::write_run_report(
        &output_dir.join("report.html"),
        scenario,
        None,
        &artifacts.run.manifest,
        &artifacts.run.events,
        &artifacts.run.samples,
        &artifacts.controller_updates,
        Some(&artifacts.performance),
    )?;
    Ok(())
}

fn elapsed_us(start: Instant) -> u64 {
    start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests;
