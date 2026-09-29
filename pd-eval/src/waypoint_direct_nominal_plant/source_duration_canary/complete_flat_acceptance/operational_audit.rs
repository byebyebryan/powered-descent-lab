//! Fixed evaluator validity guard for saved-coverage execution. No plant step,
//! contact tolerance, generator policy, or continuation authority lives here.

use pd_core::{
    BoundedGuardFailureDispositionV1, BoundedGuardFailureV1, BoundedRunGuard, FlightProgramV1,
    IncomingContactV1,
};

use super::super::super::plant_applied_throttle;
use super::{
    body_aware_terminal::{clearance_policy, phase_at},
    terminal_admissibility::contact_audit,
    *,
};
use crate::{
    WaypointDirectNominalDirectGenerationRequest, nominal_direct_flight_identity,
    preflight_nominal_direct_flight,
};

pub const BODY_AWARE_OPERATIONAL_VALIDITY_ID: &str = "body_aware_operational_validity_v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalGuardFailureEvidenceV1 {
    pub stage: String,
    pub physics_step: u64,
    pub failure: BoundedGuardFailureV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalValidityEvidenceV1 {
    pub guard_identity: String,
    pub request_identity: String,
    pub program_identity: String,
    pub generation_policy_identity: String,
    pub terminal_policy_identity: String,
    pub initial_state_checked: bool,
    pub initial_clearance_scan: GeometryClearanceScanEvidence,
    pub pretransition_checks: u64,
    pub powered_fuel_budget_checks: u64,
    pub posttransition_checks: u64,
    pub airborne_clearance_scan: GeometryClearanceScanEvidence,
    pub first_contact: Option<TerminalContactAuditEvidence>,
    pub source_handoff_state: Option<PlantStateEvidence>,
    pub terminal_entry_state: Option<PlantStateEvidence>,
    pub maximum_actual_slew_radps: f64,
    pub first_failure: Option<OperationalGuardFailureEvidenceV1>,
    pub passed_so_far: bool,
}

/// Construction validates the frozen policy/context binding. The admitting
/// adapter additionally verifies the complete selected witness before motion.
pub(crate) struct BodyAwareOperationalGuardV1 {
    binding: pd_core::FlightProgramBindingV1,
    clearance: ClearancePolicy,
    source_end: u64,
    terminal_entry: u64,
    evidence: OperationalValidityEvidenceV1,
}

impl BodyAwareOperationalGuardV1 {
    pub(crate) fn new(
        context: &RunContext,
        request: &WaypointDirectNominalDirectGenerationRequest,
        policy: &BodyAwareTerminalPolicyV1,
        program: &FlightProgramV1,
    ) -> Result<Self> {
        if preflight_nominal_direct_flight(request, policy)
            .rejection
            .is_some()
        {
            bail!("fixed operational guard requires a supported frozen request");
        }
        program
            .validate_against_context(context)
            .map_err(anyhow::Error::msg)?;
        if program.generation_policy_identity != nominal_direct_flight_identity(&request.policy)?
            || program.terminal_policy_identity != nominal_direct_flight_identity(policy)?
            || program.source_pad_id != request.source_pad_id
            || program.target_pad_id != request.target_pad_id
            || pd_core::FlightProgramBindingV1::from_context(context)
                != pd_core::FlightProgramBindingV1::from_context(
                    &RunContext::from_scenario(&request.scenario).map_err(anyhow::Error::msg)?,
                )
        {
            bail!("fixed operational guard request/program/context binding differs");
        }
        Ok(Self {
            binding: pd_core::FlightProgramBindingV1::from_context(context),
            clearance: clearance_policy(context, request)?,
            source_end: program.source_handoff_physics_step,
            terminal_entry: program.terminal_entry_physics_step,
            evidence: OperationalValidityEvidenceV1 {
                guard_identity: BODY_AWARE_OPERATIONAL_VALIDITY_ID.into(),
                request_identity: nominal_direct_flight_identity(request)?,
                program_identity: nominal_direct_flight_identity(program)?,
                generation_policy_identity: program.generation_policy_identity.clone(),
                terminal_policy_identity: program.terminal_policy_identity.clone(),
                initial_state_checked: false,
                initial_clearance_scan: empty_clearance_scan(),
                pretransition_checks: 0,
                powered_fuel_budget_checks: 0,
                posttransition_checks: 0,
                airborne_clearance_scan: empty_clearance_scan(),
                first_contact: None,
                source_handoff_state: None,
                terminal_entry_state: None,
                maximum_actual_slew_radps: 0.0,
                first_failure: None,
                passed_so_far: true,
            },
        })
    }

    pub(crate) fn evidence(&self) -> &OperationalValidityEvidenceV1 {
        &self.evidence
    }

    fn fail(
        &mut self,
        stage: &str,
        step: u64,
        disposition: BoundedGuardFailureDispositionV1,
        reason: &str,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        let failure = BoundedGuardFailureV1 {
            disposition,
            reason: reason.into(),
        };
        if self.evidence.first_failure.is_none() {
            self.evidence.first_failure = Some(OperationalGuardFailureEvidenceV1 {
                stage: stage.into(),
                physics_step: step,
                failure,
            });
        }
        self.evidence.passed_so_far = false;
        Err(self
            .evidence
            .first_failure
            .as_ref()
            .expect("failure recorded")
            .failure
            .clone())
    }

    fn binding_check(
        &mut self,
        context: &RunContext,
        stage: &str,
        step: u64,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        if let Some(failure) = &self.evidence.first_failure {
            return Err(failure.failure.clone());
        }
        if pd_core::FlightProgramBindingV1::from_context(context) != self.binding {
            return self.fail(
                stage,
                step,
                BoundedGuardFailureDispositionV1::ExecutionInvalid,
                "guard context changed after admission",
            );
        }
        Ok(())
    }
}

impl BoundedRunGuard for BodyAwareOperationalGuardV1 {
    fn initial(
        &mut self,
        context: &RunContext,
        state: &SimulationState,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        self.binding_check(context, "initial", state.physics_step)?;
        self.evidence.initial_state_checked = true;
        record_airborne_clearance(
            context,
            state,
            state.physics_step,
            "upright",
            self.clearance,
            &mut self.evidence.initial_clearance_scan,
        );
        if !self
            .evidence
            .initial_clearance_scan
            .all_airborne_states_passed
        {
            return self.fail(
                "initial",
                state.physics_step,
                BoundedGuardFailureDispositionV1::SafetyRejected,
                "initial body/domain clearance failed",
            );
        }
        Ok(())
    }

    fn before_transition(
        &mut self,
        context: &RunContext,
        state: &SimulationState,
        command: Command,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        self.binding_check(context, "before_transition", state.physics_step)?;
        self.evidence.pretransition_checks += 1;
        let applied =
            plant_applied_throttle(command, context.vehicle.min_throttle_frac, state.fuel_kg);
        let burn = applied * context.vehicle.max_fuel_burn_kgps * context.sim.physics_dt_s();
        if command.throttle_frac > 0.0 {
            self.evidence.powered_fuel_budget_checks += 1;
        }
        if !(state.fuel_kg >= burn
            && (command.throttle_frac == 0.0 || state.fuel_kg > 0.0)
            && (applied == 0.0 || applied >= context.vehicle.min_throttle_frac)
            && applied <= 1.0)
        {
            return self.fail(
                "before_transition",
                state.physics_step,
                BoundedGuardFailureDispositionV1::SafetyRejected,
                "selected/held command is not funded for this ordinary physics transition",
            );
        }
        Ok(())
    }

    fn after_transition(
        &mut self,
        context: &RunContext,
        state: &SimulationState,
        contact: Option<&IncomingContactV1>,
    ) -> std::result::Result<(), BoundedGuardFailureV1> {
        self.binding_check(context, "after_transition", state.physics_step)?;
        self.evidence.posttransition_checks += 1;
        self.evidence.airborne_clearance_scan.poststep_state_count += 1;
        if state.physics_step == self.source_end {
            self.evidence.source_handoff_state = Some(plant_state_evidence(state, context));
        }
        if state.physics_step == self.terminal_entry {
            self.evidence.terminal_entry_state = Some(plant_state_evidence(state, context));
        }
        let slew = contact
            .map_or(state.angular_rate_radps, |incoming| {
                incoming.state.angular_rate_radps
            })
            .abs();
        self.evidence.maximum_actual_slew_radps = self.evidence.maximum_actual_slew_radps.max(slew);
        if slew > context.vehicle.max_rotation_rate_radps + 1.0e-12 {
            return self.fail(
                "after_transition",
                state.physics_step,
                BoundedGuardFailureDispositionV1::ExecutionInvalid,
                "ordinary actuator slew exceeds vehicle limit",
            );
        }
        if let Some(contact) = contact {
            let incoming = contact.state.to_simulation_state();
            let audit = contact_audit(context, &incoming, &contact.classification);
            let mirror = audit.core_matches_predicate_mirror;
            let domain = audit.body_within_strict_terrain_domain;
            let safe_target = audit.classification != "stable_touchdown_on_target"
                || stable_safe_margins_pass(&audit.margins);
            self.evidence.first_contact = Some(audit);
            if !mirror {
                return self.fail(
                    "contact",
                    state.physics_step,
                    BoundedGuardFailureDispositionV1::ExecutionInvalid,
                    "incoming core/contact predicate mirror disagrees",
                );
            }
            if !domain || !safe_target {
                return self.fail(
                    "contact",
                    state.physics_step,
                    BoundedGuardFailureDispositionV1::SafetyRejected,
                    "incoming contact domain/safe-target audit failed",
                );
            }
        } else {
            record_airborne_clearance(
                context,
                state,
                state.physics_step,
                phase_at(state.physics_step, self.source_end, self.terminal_entry),
                self.clearance,
                &mut self.evidence.airborne_clearance_scan,
            );
            if !self
                .evidence
                .airborne_clearance_scan
                .all_airborne_states_passed
            {
                return self.fail(
                    "airborne",
                    state.physics_step,
                    BoundedGuardFailureDispositionV1::SafetyRejected,
                    "airborne body/domain clearance failed",
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> (
        RunContext,
        WaypointDirectNominalDirectGenerationRequest,
        FlightProgramV1,
    ) {
        let request = crate::load_waypoint_direct_generation_fresh_manifest(
            Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap(),
        )
        .unwrap()
        .request("fresh_flat_span_600_delta_000")
        .unwrap();
        let context = RunContext::from_scenario(&request.scenario).unwrap();
        let program = FlightProgramV1 {
            schema_version: 1,
            binding: pd_core::FlightProgramBindingV1::from_context(&context),
            source_pad_id: request.source_pad_id.clone(),
            target_pad_id: request.target_pad_id.clone(),
            generation_policy_identity: nominal_direct_flight_identity(&request.policy).unwrap(),
            terminal_policy_identity: nominal_direct_flight_identity(
                &BodyAwareTerminalPolicyV1::default(),
            )
            .unwrap(),
            witness_identity: "neutral_boundary_fixture_not_a_witness".into(),
            source_handoff_physics_step: 74,
            terminal_entry_physics_step: 76,
            expected_contact_physics_step: 80,
            planned_end_physics_step: 84,
            updates: (0..40)
                .map(|index| pd_core::FlightProgramUpdateV1 {
                    physics_step: index * 2,
                    phase: "upright".into(),
                    command: Command::idle(),
                })
                .collect(),
        };
        (context, request, program)
    }

    fn guard(
        context: &RunContext,
        request: &WaypointDirectNominalDirectGenerationRequest,
        program: &FlightProgramV1,
    ) -> BodyAwareOperationalGuardV1 {
        BodyAwareOperationalGuardV1::new(
            context,
            request,
            &BodyAwareTerminalPolicyV1::default(),
            program,
        )
        .unwrap()
    }

    #[test]
    fn initial_source_support_and_first_launch_tick_do_not_require_cutaway() {
        let (context, request, program) = inputs();
        let mut guard = guard(&context, &request, &program);
        let mut state = SimulationState::new(&context).unwrap();
        guard.initial(&context, &state).unwrap();
        assert_eq!(
            guard
                .evidence()
                .initial_clearance_scan
                .source_corridor_state_count,
            1
        );
        assert_eq!(
            guard
                .evidence()
                .airborne_clearance_scan
                .poststep_state_count,
            0
        );
        let command = Command {
            throttle_frac: 1.0,
            target_attitude_rad: 0.0,
        };
        state.set_command(command);
        guard.before_transition(&context, &state, command).unwrap();
        let transition = state.step_with_contact_report(&context);
        assert!(transition.incoming_contact.is_none());
        guard.after_transition(&context, &state, None).unwrap();
        assert!(guard.evidence().passed_so_far);
        assert_eq!(
            guard
                .evidence()
                .airborne_clearance_scan
                .source_corridor_state_count,
            1
        );
    }

    #[test]
    fn actually_selected_powered_command_is_checked_each_tick_and_failure_is_sticky() {
        let (context, request, program) = inputs();
        let mut guard = guard(&context, &request, &program);
        let mut state = SimulationState::new(&context).unwrap();
        let powered = Command {
            throttle_frac: 1.0,
            target_attitude_rad: 0.0,
        };
        let burn = context.vehicle.max_fuel_burn_kgps * context.sim.physics_dt_s();
        state.fuel_kg = burn;
        guard.before_transition(&context, &state, powered).unwrap();
        state.fuel_kg = burn * 0.5;
        state.held_command = Command::idle();
        let failure = guard
            .before_transition(&context, &state, powered)
            .unwrap_err();
        assert_eq!(
            failure.disposition,
            BoundedGuardFailureDispositionV1::SafetyRejected
        );
        assert_eq!(guard.evidence().pretransition_checks, 2);
        state.fuel_kg = 100.0;
        assert_eq!(
            guard
                .before_transition(&context, &state, Command::idle())
                .unwrap_err(),
            failure
        );
        assert_eq!(guard.evidence().pretransition_checks, 2);
    }

    #[test]
    fn zero_fuel_idle_coast_is_legal_but_powered_command_is_not() {
        let (context, request, program) = inputs();
        let mut guard = guard(&context, &request, &program);
        let mut state = SimulationState::new(&context).unwrap();
        state.fuel_kg = 0.0;
        guard
            .before_transition(&context, &state, Command::idle())
            .unwrap();
        assert!(
            guard
                .before_transition(
                    &context,
                    &state,
                    Command {
                        throttle_frac: 0.5,
                        target_attitude_rad: 0.0
                    }
                )
                .is_err()
        );
    }

    #[test]
    fn changed_context_is_invalid_not_a_safety_or_generation_decision() {
        let (mut context, request, program) = inputs();
        let mut guard = guard(&context, &request, &program);
        let state = SimulationState::new(&context).unwrap();
        context.world.gravity_mps2 += 0.01;
        assert_eq!(
            guard.initial(&context, &state).unwrap_err().disposition,
            BoundedGuardFailureDispositionV1::ExecutionInvalid
        );
    }

    #[test]
    fn initial_and_airborne_clearance_failures_keep_distinct_stage_evidence() {
        let (context, request, program) = inputs();
        let mut state = SimulationState::new(&context).unwrap();
        state.position_m.x += 100.0;
        let mut initial = guard(&context, &request, &program);
        assert!(initial.initial(&context, &state).is_err());
        assert_eq!(
            initial.evidence().first_failure.as_ref().unwrap().stage,
            "initial"
        );
        let mut airborne = guard(&context, &request, &program);
        state.physics_step = 1;
        state.sim_time_s = context.sim.physics_dt_s();
        assert!(airborne.after_transition(&context, &state, None).is_err());
        assert_eq!(
            airborne.evidence().first_failure.as_ref().unwrap().stage,
            "airborne"
        );
    }

    #[test]
    fn incoming_off_target_contact_is_not_airborne_clearance_or_fake_safety_failure() {
        let (context, request, program) = inputs();
        let mut guard = guard(&context, &request, &program);
        let bounded = pd_core::run_simulation_bounded(
            &context,
            "neutral_fixture",
            pd_core::BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 2,
                hard_end_physics_step: 2,
            },
            |_, _| Ok(Command::idle()),
            &mut guard,
        )
        .unwrap();
        assert_eq!(
            bounded.stop,
            pd_core::BoundedRunStopCauseV1::MissionTerminal
        );
        assert!(guard.evidence().passed_so_far);
        assert_eq!(
            guard
                .evidence()
                .airborne_clearance_scan
                .airborne_state_count,
            0
        );
        assert_eq!(
            guard
                .evidence()
                .first_contact
                .as_ref()
                .unwrap()
                .classification,
            "stable_touchdown_off_target"
        );
        assert!(
            bounded
                .incoming_contact
                .as_ref()
                .unwrap()
                .state
                .velocity_mps
                .y
                < 0.0
        );
    }
}
