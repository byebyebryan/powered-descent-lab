//! Opt-in admitted execution under strict saved-command coverage. This is not
//! a new planner, a disturbance admission path, or a continuation controller.

use std::{fs::OpenOptions, io::Write, path::Path, time::Instant};

use anyhow::{Context, Result, bail};
use pd_control::{
    OperationalFlightProgramArtifactsV1, replay_flight_program_operational,
    run_flight_program_operational,
};
use pd_core::{BoundedRunArtifactsV1, BoundedRunStopCauseV1, FlightProgramV1, RunContext};
use serde::{Deserialize, Serialize};

use crate::{
    BodyAwareOperationalGuardV1, BodyAwareTerminalPolicyV1, BodyAwareTerminalVerificationV1,
    BodyAwareTerminalWitnessV1, NominalDirectFlightComputeV1, NominalDirectFlightDecisionV1,
    OperationalValidityEvidenceV1, WaypointDirectNominalDirectGenerationRequest,
    evaluate_nominal_direct_flight,
    nominal_direct_flight::{program_from_witness, reserve_output_root, write_create_only},
    nominal_direct_flight_identity, preflight_nominal_direct_flight,
    verify_body_aware_terminal_witness,
};

pub const NOMINAL_DIRECT_OPERATIONAL_PROTOCOL: &str =
    "docs/nominal_direct_operational_execution_protocol.md";
pub const STRICT_SAVED_COVERAGE_POLICY_ID: &str = "strict_saved_coverage_v1";

const NON_CLAIMS: [&str; 5] = [
    "Opt-in admitted nominal execution with strict saved coverage; planner/default controllers are unchanged.",
    "Airborne coverage/deadline stops are laboratory execution boundaries, not physical crashes or mission success.",
    "No extra command, idle fallback, continuation, terminal hold, disturbance admission, or operating-tolerance claim.",
    "Finite Unknown is a generator coverage gap, not physical infeasibility or waypoint necessity.",
    "Existing pointwise discrete body safety is not swept safety, robustness, arbitrary waypoint-state composition, or real-time authority.",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NominalDirectOperationalOutcomeV1 {
    CompletedSafeTarget,
    UnsafeContact,
    OffTargetContact,
    CoverageExhausted,
    HardDeadlineReached,
    ScenarioHorizonReached,
    SafetyRejected,
    ExecutionInvalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NominalComparisonV1 {
    Match,
    Deviation,
    NotComparable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalNominalChecksV1 {
    pub full_witness_admitted: bool,
    pub exact_command_and_clock_parity: bool,
    pub exact_contact_audit: bool,
    pub exact_contact_tick_and_fuel: bool,
    pub exact_airborne_clearance_scan: bool,
    pub source_handoff_matches: bool,
    pub exact_terminal_entry: bool,
    pub exact_maximum_actual_slew: bool,
    pub bounded_action_replay_parity: bool,
    pub valid_safe_target_contact: bool,
}

impl OperationalNominalChecksV1 {
    fn all_passed(&self) -> bool {
        self.full_witness_admitted
            && self.exact_command_and_clock_parity
            && self.exact_contact_audit
            && self.exact_contact_tick_and_fuel
            && self.exact_airborne_clearance_scan
            && self.source_handoff_matches
            && self.exact_terminal_entry
            && self.exact_maximum_actual_slew
            && self.bounded_action_replay_parity
            && self.valid_safe_target_contact
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalReplayEvidenceV1 {
    pub recomputed_from_full_program_bounds: bool,
    pub bounded_run_matches: bool,
    pub fixed_guard_matches: bool,
    pub replay: Option<BoundedRunArtifactsV1>,
    pub validity: Option<OperationalValidityEvidenceV1>,
    pub failure_detail: Option<String>,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalDirectOperationalExecutionEvidenceV1 {
    pub execution_policy_identity: String,
    pub program_identity: String,
    pub witness_identity: String,
    pub admission: BodyAwareTerminalVerificationV1,
    pub observed: BoundedRunArtifactsV1,
    pub validity: OperationalValidityEvidenceV1,
    pub replay: OperationalReplayEvidenceV1,
    pub operational_outcome: NominalDirectOperationalOutcomeV1,
    pub nominal_comparison: NominalComparisonV1,
    pub nominal_checks: OperationalNominalChecksV1,
}

fn outcome(
    bounded: &BoundedRunArtifactsV1,
    validity: &OperationalValidityEvidenceV1,
    replay_passed: bool,
) -> NominalDirectOperationalOutcomeV1 {
    use NominalDirectOperationalOutcomeV1 as O;
    if !replay_passed
        || bounded.stop == BoundedRunStopCauseV1::ExecutionInvalid
        || validity.first_failure.as_ref().is_some_and(|failure| {
            failure.failure.disposition
                == pd_core::BoundedGuardFailureDispositionV1::ExecutionInvalid
        })
    {
        return O::ExecutionInvalid;
    }
    if bounded.stop == BoundedRunStopCauseV1::SafetyRejected || !validity.passed_so_far {
        return O::SafetyRejected;
    }
    let manifest = &bounded.run.manifest;
    let raw_matches = |physical, mission, reason| {
        manifest.physical_outcome == physical
            && manifest.mission_outcome == mission
            && manifest.end_reason == reason
    };
    let airborne_prefix = raw_matches(
        pd_core::PhysicalOutcome::Flying,
        pd_core::MissionOutcome::InProgress,
        pd_core::EndReason::Running,
    ) && bounded.incoming_contact.is_none()
        && validity.first_contact.is_none();
    match bounded.stop {
        BoundedRunStopCauseV1::MissionTerminal => match validity
            .first_contact
            .as_ref()
            .map(|contact| contact.classification.as_str())
        {
            Some("stable_touchdown_on_target")
                if raw_matches(
                    pd_core::PhysicalOutcome::LandedOnTarget,
                    pd_core::MissionOutcome::Success,
                    pd_core::EndReason::TouchdownOnTarget,
                ) =>
            {
                O::CompletedSafeTarget
            }
            Some("stable_touchdown_off_target")
                if raw_matches(
                    pd_core::PhysicalOutcome::LandedOffTarget,
                    pd_core::MissionOutcome::FailedOffTarget,
                    pd_core::EndReason::TouchdownOffTarget,
                ) =>
            {
                O::OffTargetContact
            }
            Some("crash")
                if raw_matches(
                    pd_core::PhysicalOutcome::Crashed,
                    pd_core::MissionOutcome::FailedCrash,
                    pd_core::EndReason::Crash,
                ) =>
            {
                O::UnsafeContact
            }
            _ => O::ExecutionInvalid,
        },
        BoundedRunStopCauseV1::CoverageExhausted if airborne_prefix => O::CoverageExhausted,
        BoundedRunStopCauseV1::HardDeadlineReached if airborne_prefix => O::HardDeadlineReached,
        BoundedRunStopCauseV1::ScenarioHorizonReached
            if raw_matches(
                pd_core::PhysicalOutcome::TimedOut,
                pd_core::MissionOutcome::FailedTimeout,
                pd_core::EndReason::MaxTimeReached,
            ) =>
        {
            O::ScenarioHorizonReached
        }
        BoundedRunStopCauseV1::SafetyRejected => O::SafetyRejected,
        BoundedRunStopCauseV1::ExecutionInvalid => O::ExecutionInvalid,
        _ => O::ExecutionInvalid,
    }
}

/// Structural or witness rejection occurs before motion. After admission,
/// bounded failure evidence is returned intact; it is never converted to safe
/// completion merely because the nominal witness had an accepted contact.
pub fn execute_nominal_direct_operational_program(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
    witness: &BodyAwareTerminalWitnessV1,
    program: &FlightProgramV1,
    compute: &mut NominalDirectFlightComputeV1,
) -> Result<(
    NominalDirectOperationalExecutionEvidenceV1,
    OperationalFlightProgramArtifactsV1,
)> {
    if preflight_nominal_direct_flight(request, policy)
        .rejection
        .is_some()
    {
        bail!("operational execution requires a supported frozen request");
    }
    let context = RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?;
    program
        .validate_against_context(&context)
        .map_err(anyhow::Error::msg)?;
    if *program != program_from_witness(request, policy, witness)? {
        bail!("program differs from the complete selected witness payload/bindings");
    }
    let started = Instant::now();
    let admission = verify_body_aware_terminal_witness(request, policy, witness)?;
    compute.selected_verification_wall_time_us = elapsed_us(started);
    compute.selected_witness_verification_physics_ticks = admission.physics_ticks_advanced * 2;
    if !admission.passed || admission != witness.verification {
        bail!("selected witness fails unchanged independent full safety verification");
    }
    let mut guard = BodyAwareOperationalGuardV1::new(&context, request, policy, program)?;
    let started = Instant::now();
    let artifacts = run_flight_program_operational(&context, program, &mut guard)?;
    compute.ordinary_execution_wall_time_us = elapsed_us(started);
    compute.ordinary_execution_physics_ticks = artifacts.bounded.run.manifest.physics_steps;
    let validity = guard.evidence().clone();
    let mut replay_guard = BodyAwareOperationalGuardV1::new(&context, request, policy, program)?;
    let started = Instant::now();
    let replay_result = replay_flight_program_operational(
        &context,
        program,
        &artifacts.bounded.run.actions,
        &mut replay_guard,
    );
    compute.action_replay_wall_time_us = elapsed_us(started);
    let replay = match replay_result {
        Ok(replayed) => {
            let replayed = replayed.bounded;
            compute.action_replay_physics_ticks = replayed.run.manifest.physics_steps;
            let bounded_run_matches = replayed == artifacts.bounded;
            let replay_validity = replay_guard.evidence().clone();
            let fixed_guard_matches = replay_validity == validity;
            OperationalReplayEvidenceV1 {
                recomputed_from_full_program_bounds: true, bounded_run_matches, fixed_guard_matches,
                replay: Some(replayed), validity: Some(replay_validity),
                failure_detail: (!bounded_run_matches || !fixed_guard_matches)
                    .then(|| "bounded actual prefix or fixed-guard evidence differs from independent replay".into()),
                passed: bounded_run_matches && fixed_guard_matches,
            }
        }
        Err(error) => OperationalReplayEvidenceV1 {
            recomputed_from_full_program_bounds: true,
            bounded_run_matches: false,
            fixed_guard_matches: false,
            replay: None,
            validity: Some(replay_guard.evidence().clone()),
            failure_detail: Some(error.to_string()),
            passed: false,
        },
    };
    let operational_outcome = outcome(&artifacts.bounded, &validity, replay.passed);
    let actions = &artifacts.bounded.run.actions;
    let exact_command_and_clock_parity =
        actions.len() == program.updates.len()
            && actions.iter().zip(&program.updates).enumerate().all(
                |(index, (actual, expected))| {
                    actual.physics_step == expected.physics_step
                        && actual.command == expected.command
                        && actual.controller_update_index == index as u64
                        && actual.sim_time_s
                            == actual.physics_step as f64 / f64::from(context.sim.physics_hz)
                },
            );
    let contact = validity.first_contact.as_ref();
    let nominal_checks = OperationalNominalChecksV1 {
        full_witness_admitted: true,
        exact_command_and_clock_parity,
        exact_contact_audit: contact == admission.first_contact.as_ref(),
        exact_contact_tick_and_fuel: contact.zip(admission.first_contact.as_ref()).is_some_and(
            |(actual, expected)| {
                actual.state.physics_step == expected.state.physics_step
                    && actual.state.sim_time_s == expected.state.sim_time_s
                    && actual.state.fuel_kg == expected.state.fuel_kg
                    && artifacts.bounded.run.manifest.summary.fuel_remaining_kg
                        == expected.state.fuel_kg
            },
        ),
        exact_airborne_clearance_scan: validity.airborne_clearance_scan == admission.clearance_scan,
        source_handoff_matches: validity.source_handoff_state.as_ref().is_some_and(|state| {
            (state.position_m - witness.source_handoff_reference.position_m).length() <= 1.0e-6
                && (state.velocity_mps - witness.source_handoff_reference.velocity_mps).length()
                    <= 1.0e-6
        }),
        exact_terminal_entry: validity.terminal_entry_state == admission.actual_terminal_entry,
        exact_maximum_actual_slew: validity.maximum_actual_slew_radps
            == admission.maximum_actual_slew_radps,
        bounded_action_replay_parity: replay.passed,
        valid_safe_target_contact: operational_outcome
            == NominalDirectOperationalOutcomeV1::CompletedSafeTarget,
    };
    let nominal_comparison =
        if operational_outcome == NominalDirectOperationalOutcomeV1::ExecutionInvalid {
            NominalComparisonV1::NotComparable
        } else if nominal_checks.all_passed() {
            NominalComparisonV1::Match
        } else {
            NominalComparisonV1::Deviation
        };
    Ok((
        NominalDirectOperationalExecutionEvidenceV1 {
            execution_policy_identity: STRICT_SAVED_COVERAGE_POLICY_ID.into(),
            program_identity: nominal_direct_flight_identity(program)?,
            witness_identity: witness.identity.clone(),
            admission,
            observed: artifacts.bounded.clone(),
            validity,
            replay,
            operational_outcome,
            nominal_comparison,
            nominal_checks,
        },
        artifacts,
    ))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NominalDirectOperationalArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub request_identity: String,
    pub decision: NominalDirectFlightDecisionV1,
    pub execution: Option<NominalDirectOperationalExecutionEvidenceV1>,
    pub admission_error: Option<String>,
    pub compute: NominalDirectFlightComputeV1,
    pub scope_non_claims: Vec<String>,
    pub identity: String,
}

pub fn nominal_direct_operational_artifact_identity(
    artifact: &NominalDirectOperationalArtifactV1,
) -> Result<String> {
    let mut canonical = artifact.clone();
    canonical.identity.clear();
    canonical.compute.generation_wall_time_us = 0;
    canonical.compute.selected_verification_wall_time_us = 0;
    canonical.compute.ordinary_execution_wall_time_us = 0;
    canonical.compute.action_replay_wall_time_us = 0;
    canonical.compute.artifact_writing_wall_time_us = 0;
    nominal_direct_flight_identity(&canonical)
}

pub fn run_nominal_direct_operational_flight(
    request: &WaypointDirectNominalDirectGenerationRequest,
    policy: &BodyAwareTerminalPolicyV1,
    output_dir: &Path,
) -> Result<NominalDirectOperationalArtifactV1> {
    reserve_output_root(output_dir)?;
    let started = Instant::now();
    write_create_only(&output_dir.join("request.json"), request)?;
    write_create_only(&output_dir.join("scenario.json"), &request.scenario)?;
    write_create_only(&output_dir.join("terminal_policy.json"), policy)?;
    let mut writing_us = elapsed_us(started);
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
    let mut artifact = NominalDirectOperationalArtifactV1 {
        schema_id: "nominal_direct_operational_flight_v1".into(),
        schema_version: 1,
        request_identity: nominal_direct_flight_identity(request)?,
        decision: evaluation.decision,
        execution: None,
        admission_error: None,
        compute: evaluation.compute,
        scope_non_claims: NON_CLAIMS.iter().map(|claim| (*claim).into()).collect(),
        identity: String::new(),
    };
    let started = Instant::now();
    write_create_only(&output_dir.join("decision.json"), &artifact.decision)?;
    if let Some(generation) = &evaluation.generation {
        write_create_only(&output_dir.join("generation.json"), generation)?;
    }
    let selected = if let NominalDirectFlightDecisionV1::Direct {
        selected_row_index,
        program,
        ..
    } = &artifact.decision
    {
        write_create_only(&output_dir.join("program.json"), program)?;
        let witness = evaluation
            .generation
            .as_ref()
            .context("Direct has no generated ledger")?
            .rows
            .get(*selected_row_index)
            .and_then(|row| row.witness.as_ref())
            .context("Direct has no accepted selected witness")?;
        write_create_only(&output_dir.join("witness.json"), witness)?;
        Some((program.as_ref(), witness))
    } else {
        None
    };
    writing_us += elapsed_us(started);
    if let Some((program, witness)) = selected {
        match execute_nominal_direct_operational_program(
            request,
            policy,
            witness,
            program,
            &mut artifact.compute,
        ) {
            Ok((evidence, controlled)) => {
                let started = Instant::now();
                write_create_only(&output_dir.join("admission.json"), &evidence.admission)?;
                write_create_only(
                    &output_dir.join("bounded_execution.json"),
                    &controlled.bounded,
                )?;
                write_create_only(&output_dir.join("validity.json"), &evidence.validity)?;
                write_create_only(&output_dir.join("replay.json"), &evidence.replay)?;
                write_create_only(
                    &output_dir.join("controller_updates.json"),
                    &controlled.controller_updates,
                )?;
                write_create_only(
                    &output_dir.join("run_performance.json"),
                    &controlled.performance,
                )?;
                let run = &controlled.bounded.run;
                write_create_only(&output_dir.join("manifest.json"), &run.manifest)?;
                write_create_only(&output_dir.join("actions.json"), &run.actions)?;
                write_create_only(&output_dir.join("events.json"), &run.events)?;
                write_create_only(&output_dir.join("samples.json"), &run.samples)?;
                // Display-only timing-free projection. Raw measured logs and
                // performance remain above; default report behavior is unchanged.
                let mut trace_updates = controlled.controller_updates.clone();
                for update in &mut trace_updates {
                    update.compute_time_us = None;
                }
                pd_report::write_run_report(
                    &output_dir.join("prefix_trace.html"),
                    &request.scenario,
                    None,
                    &run.manifest,
                    &run.events,
                    &run.samples,
                    &trace_updates,
                    None,
                )?;
                writing_us += elapsed_us(started);
                artifact.execution = Some(evidence);
            }
            Err(error) => {
                artifact.admission_error = Some(format!("{error:#}"));
                write_create_only(
                    &output_dir.join("admission_error.json"),
                    &artifact.admission_error,
                )?;
            }
        }
    }
    artifact.compute.artifact_writing_wall_time_us = writing_us;
    artifact.identity = nominal_direct_operational_artifact_identity(&artifact)?;
    write_create_only(&output_dir.join("performance.json"), &artifact.compute)?;
    write_create_only(&output_dir.join("summary.json"), &artifact)?;
    let axes = artifact
        .execution
        .as_ref()
        .map(|execution| {
            format!(
                "{:?} / {:?}",
                execution.operational_outcome, execution.nominal_comparison
            )
        })
        .unwrap_or_else(|| format!("{} / no executed prefix", artifact.decision.status()));
    let trace = if artifact.execution.is_some() {
        "<p><a href='prefix_trace.html'>Ordinary prefix trace</a> (raw core outcomes; execution stop is reported above; observational compute timing is in the JSON performance/log files).</p>"
    } else {
        ""
    };
    let observed = artifact.execution.as_ref().map(|execution| {
        let bounded = &execution.observed;
        let state = &bounded.final_state;
        format!("<p>Stop: {:?}; physics tick {}; time {:.6} s; fuel {:.6} kg.</p><p>Coverage K={}; hard end H={}; raw core {:?} / {:?} / {:?}.</p>",
            bounded.stop, state.physics_step, state.sim_time_s, state.fuel_kg,
            bounded.limits.command_coverage_end_physics_step, bounded.limits.hard_end_physics_step,
            state.physical_outcome, state.mission_outcome, state.end_reason)
    }).unwrap_or_default();
    write_text_create_only(
        &output_dir.join("report.html"),
        &format!(
            "<!doctype html><meta charset='utf-8'><title>Strict saved-coverage execution</title><h1>Strict saved-coverage execution V1</h1><p>{axes}</p>{observed}<p>No continuation or default planner promotion.</p>{trace}<p><a href='summary.json'>Complete outcome and nominal checks</a> · <a href='decision.json'>Planning decision</a></p>"
        ),
    )?;
    Ok(artifact)
}

pub(crate) fn write_text_create_only(path: &Path, value: &str) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(value.as_bytes())?;
    Ok(())
}

fn elapsed_us(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests;
