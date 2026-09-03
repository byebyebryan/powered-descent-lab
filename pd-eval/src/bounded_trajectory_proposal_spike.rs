//! Non-normative W3 finite proposal-backend spike.
//!
//! The backend enumerates a fixed command-template catalog and fixed probe
//! horizons.  It is deliberately untrusted: only the W1 exact verifier may
//! turn a proposal into a positive physical witness.

use std::f64::consts::PI;

use pd_core::RouteTopology;
use serde::{Deserialize, Serialize};

use crate::{
    BOUNDED_REASON_SUPPORTED_EXACT_WITNESS_VERIFIED,
    BOUNDED_REASON_UNKNOWN_COVERAGE_BOUNDED_SEARCH_EXHAUSTED, BOUNDED_TRAJECTORY_MAX_PHYSICS_STEPS,
    BOUNDED_TRAJECTORY_PREDICTION_SCHEMA_VERSION, BoundedTrajectoryArtifactStatusV1,
    BoundedTrajectoryCommandV1, BoundedTrajectoryDecisionV1,
    BoundedTrajectoryProposalConfigurationV1, BoundedTrajectoryVerificationV1,
    BoundedTrajectoryVerificationVerdictV1, BoundedTrajectoryWitnessConfigurationV1,
    BoundedTrajectoryWitnessPredictionV1, BoundedTrajectoryWitnessV1, RouteCapabilityInputV1,
    canonical_digest, verify_bounded_trajectory,
};

pub const FINITE_TEMPLATE_ENGINE_ID: &str = "bounded_finite_template_search_v1";
pub const FINITE_TEMPLATE_PARAMETERIZATION_ID: &str = "constant_command_catalog_7_probe_5s_v1";
pub const FINITE_TEMPLATE_NUMERICAL_SETTINGS_ID: &str = "serial_rust_f64_exact_order_v1";
pub const FINITE_TEMPLATE_RECONSTRUCTION_RULE_ID: &str = "truncate_at_exact_first_final_handoff_v1";
pub const FINITE_TEMPLATE_DETERMINISM_ID: &str = "serial_template_then_horizon_v1";
pub const FINITE_TEMPLATE_PROBE_STEPS: u64 = 600;
pub const FINITE_TEMPLATE_CATALOG_LEN: u64 = 7;
pub const FINITE_TEMPLATE_MAX_SEARCH_LIMIT: u64 = FINITE_TEMPLATE_CATALOG_LEN
    * (BOUNDED_TRAJECTORY_MAX_PHYSICS_STEPS / FINITE_TEMPLATE_PROBE_STEPS + 1);

const UNKNOWN_SCOPE_WAYPOINT_COUNT: &str = "unknown/scope/waypoint_count";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FiniteTemplateAttemptKindV1 {
    Probe,
    ReconstructedTerminal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FiniteTemplateProposalAttemptV1 {
    pub attempt_index: u64,
    pub template_id: String,
    pub kind: FiniteTemplateAttemptKindV1,
    pub witness: BoundedTrajectoryWitnessV1,
    pub verification: BoundedTrajectoryVerificationV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FiniteTemplateProposalSpikeResultV1 {
    pub proposal_configuration: BoundedTrajectoryProposalConfigurationV1,
    pub attempts: Vec<FiniteTemplateProposalAttemptV1>,
    pub selected_attempt_index: Option<u64>,
    pub prediction: BoundedTrajectoryWitnessPredictionV1,
    pub result_digest: String,
}

#[derive(Serialize)]
struct FiniteTemplateResultDigestMaterial<'a> {
    proposal_configuration: &'a BoundedTrajectoryProposalConfigurationV1,
    attempts: &'a [FiniteTemplateProposalAttemptV1],
    selected_attempt_index: Option<u64>,
    prediction: &'a BoundedTrajectoryWitnessPredictionV1,
}

impl FiniteTemplateProposalSpikeResultV1 {
    fn seal(mut self) -> Result<Self, String> {
        self.validate_without_digest()?;
        self.result_digest = self.compute_digest()?;
        self.validate()?;
        Ok(self)
    }

    fn compute_digest(&self) -> Result<String, String> {
        canonical_digest(&FiniteTemplateResultDigestMaterial {
            proposal_configuration: &self.proposal_configuration,
            attempts: &self.attempts,
            selected_attempt_index: self.selected_attempt_index,
            prediction: &self.prediction,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_without_digest()?;
        let expected = self.compute_digest()?;
        if self.result_digest != expected {
            return Err(format!(
                "finite-template result digest mismatch: expected {expected}"
            ));
        }
        Ok(())
    }

    fn validate_without_digest(&self) -> Result<(), String> {
        validate_finite_template_configuration(&self.proposal_configuration)?;
        self.prediction.validate()?;
        if self.prediction.proposal_configuration_digest.as_deref()
            != Some(&self.proposal_configuration.configuration_digest)
        {
            return Err("prediction does not join the finite proposal configuration".to_owned());
        }

        let attempted_digests = self
            .attempts
            .iter()
            .enumerate()
            .map(|(index, attempt)| {
                if attempt.attempt_index != index as u64 {
                    return Err("finite proposal attempts are not contiguous".to_owned());
                }
                if attempt.template_id.trim().is_empty() {
                    return Err("finite proposal attempt template ID is empty".to_owned());
                }
                attempt.witness.validate()?;
                attempt.verification.validate()?;
                if attempt.verification.status != BoundedTrajectoryArtifactStatusV1::Complete
                    || attempt.verification.verdict.is_none()
                {
                    return Err("finite proposal attempt is not a complete verification".to_owned());
                }
                if attempt.witness.input_digest != self.prediction.input_digest
                    || attempt.verification.input_digest != self.prediction.input_digest
                    || attempt.witness.configuration_digest != self.prediction.configuration_digest
                    || attempt.verification.configuration_digest
                        != self.prediction.configuration_digest
                    || attempt.verification.witness_digest != attempt.witness.witness_digest
                {
                    return Err("finite proposal attempt joins are inconsistent".to_owned());
                }
                Ok(attempt.verification.verification_digest.clone())
            })
            .collect::<Result<Vec<_>, String>>()?;
        if attempted_digests != self.prediction.attempted_verification_digests {
            return Err("prediction attempt identities are not in exact search order".to_owned());
        }
        if self.attempts.len() as u64 > self.proposal_configuration.search_limit {
            return Err("finite proposal exceeded its exact-verification budget".to_owned());
        }

        match self.selected_attempt_index {
            Some(selected_index) => {
                let selected_index = usize::try_from(selected_index)
                    .map_err(|_| "selected finite proposal attempt index overflows usize")?;
                let selected = self
                    .attempts
                    .get(selected_index)
                    .ok_or_else(|| "selected finite proposal attempt is missing".to_owned())?;
                if selected_index + 1 != self.attempts.len() {
                    return Err(
                        "finite proposal did not stop at its first verified attempt".to_owned()
                    );
                }
                if selected.verification.verdict
                    != Some(BoundedTrajectoryVerificationVerdictV1::Verified)
                    || self.prediction.decision != Some(BoundedTrajectoryDecisionV1::Supported)
                    || self.prediction.witness_digest.as_deref()
                        != Some(&selected.witness.witness_digest)
                    || self.prediction.verification_digest.as_deref()
                        != Some(&selected.verification.verification_digest)
                    || self.prediction.first_reason.as_deref()
                        != Some(BOUNDED_REASON_SUPPORTED_EXACT_WITNESS_VERIFIED)
                {
                    return Err("selected finite proposal is not the supported witness".to_owned());
                }
                if self.attempts[..selected_index].iter().any(|attempt| {
                    attempt.verification.verdict
                        == Some(BoundedTrajectoryVerificationVerdictV1::Verified)
                }) {
                    return Err("finite proposal skipped an earlier verified attempt".to_owned());
                }
            }
            None => {
                if self.prediction.decision != Some(BoundedTrajectoryDecisionV1::Unknown)
                    || self.attempts.iter().any(|attempt| {
                        attempt.verification.verdict
                            == Some(BoundedTrajectoryVerificationVerdictV1::Verified)
                    })
                {
                    return Err("unselected finite proposal result must abstain".to_owned());
                }
                let reason =
                    self.prediction.first_reason.as_deref().ok_or_else(|| {
                        "finite proposal abstention is missing a reason".to_owned()
                    })?;
                if (self.attempts.is_empty() && !reason.starts_with("unknown/scope/"))
                    || (!self.attempts.is_empty()
                        && reason != BOUNDED_REASON_UNKNOWN_COVERAGE_BOUNDED_SEARCH_EXHAUSTED)
                {
                    return Err(
                        "finite proposal abstention reason disagrees with attempts".to_owned()
                    );
                }
            }
        }
        Ok(())
    }

    /// Re-run the complete finite search and every exact verification before
    /// trusting a loaded spike result.
    pub fn validate_against_exact(
        &self,
        input: &RouteCapabilityInputV1,
        witness_configuration: &BoundedTrajectoryWitnessConfigurationV1,
    ) -> Result<(), String> {
        self.validate()?;
        let expected = run_finite_template_proposal_spike(
            input,
            witness_configuration,
            &self.proposal_configuration,
        )?;
        if &expected != self {
            return Err("finite proposal result does not match deterministic replay".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct ConstantCommandTemplate {
    id: &'static str,
    throttle_frac: f64,
    target_attitude_rad: f64,
}

pub fn finite_template_proposal_configuration_v1(
    search_limit: u64,
) -> Result<BoundedTrajectoryProposalConfigurationV1, String> {
    let configuration = BoundedTrajectoryProposalConfigurationV1 {
        schema_version: 0,
        engine_id: FINITE_TEMPLATE_ENGINE_ID.to_owned(),
        parameterization_id: FINITE_TEMPLATE_PARAMETERIZATION_ID.to_owned(),
        numerical_settings_id: FINITE_TEMPLATE_NUMERICAL_SETTINGS_ID.to_owned(),
        search_limit,
        reconstruction_rule_id: FINITE_TEMPLATE_RECONSTRUCTION_RULE_ID.to_owned(),
        determinism_id: FINITE_TEMPLATE_DETERMINISM_ID.to_owned(),
        configuration_digest: String::new(),
    }
    .seal()?;
    validate_finite_template_configuration(&configuration)?;
    Ok(configuration)
}

fn validate_finite_template_configuration(
    configuration: &BoundedTrajectoryProposalConfigurationV1,
) -> Result<(), String> {
    configuration.validate()?;
    for (name, actual, expected) in [
        (
            "engine_id",
            configuration.engine_id.as_str(),
            FINITE_TEMPLATE_ENGINE_ID,
        ),
        (
            "parameterization_id",
            configuration.parameterization_id.as_str(),
            FINITE_TEMPLATE_PARAMETERIZATION_ID,
        ),
        (
            "numerical_settings_id",
            configuration.numerical_settings_id.as_str(),
            FINITE_TEMPLATE_NUMERICAL_SETTINGS_ID,
        ),
        (
            "reconstruction_rule_id",
            configuration.reconstruction_rule_id.as_str(),
            FINITE_TEMPLATE_RECONSTRUCTION_RULE_ID,
        ),
        (
            "determinism_id",
            configuration.determinism_id.as_str(),
            FINITE_TEMPLATE_DETERMINISM_ID,
        ),
    ] {
        if actual != expected {
            return Err(format!(
                "finite proposal {name} must equal locked value {expected:?}"
            ));
        }
    }
    if configuration.search_limit > FINITE_TEMPLATE_MAX_SEARCH_LIMIT {
        return Err(format!(
            "finite proposal search_limit exceeds {FINITE_TEMPLATE_MAX_SEARCH_LIMIT}"
        ));
    }
    Ok(())
}

fn command_templates(horizontal_sign: i8) -> [ConstantCommandTemplate; 7] {
    let outbound = f64::from(horizontal_sign);
    [
        ConstantCommandTemplate {
            id: "upright_full",
            throttle_frac: 1.0,
            target_attitude_rad: 0.0,
        },
        ConstantCommandTemplate {
            id: "outbound_15deg_full",
            throttle_frac: 1.0,
            target_attitude_rad: outbound * PI / 12.0,
        },
        ConstantCommandTemplate {
            id: "inbound_15deg_full",
            throttle_frac: 1.0,
            target_attitude_rad: -outbound * PI / 12.0,
        },
        ConstantCommandTemplate {
            id: "outbound_30deg_full",
            throttle_frac: 1.0,
            target_attitude_rad: outbound * PI / 6.0,
        },
        ConstantCommandTemplate {
            id: "inbound_30deg_full",
            throttle_frac: 1.0,
            target_attitude_rad: -outbound * PI / 6.0,
        },
        ConstantCommandTemplate {
            id: "upright_three_quarter",
            throttle_frac: 0.75,
            target_attitude_rad: 0.0,
        },
        ConstantCommandTemplate {
            id: "upright_half",
            throttle_frac: 0.5,
            target_attitude_rad: 0.0,
        },
    ]
}

fn commands_for_template(
    template: ConstantCommandTemplate,
    terminal_physics_step: u64,
    hold_steps: u32,
) -> Vec<BoundedTrajectoryCommandV1> {
    let command_count = terminal_physics_step.div_ceil(u64::from(hold_steps));
    (0..command_count)
        .map(|slot| BoundedTrajectoryCommandV1 {
            slot,
            throttle_frac: template.throttle_frac,
            target_attitude_rad: template.target_attitude_rad,
        })
        .collect()
}

fn attempt(
    input: &RouteCapabilityInputV1,
    witness_configuration: &BoundedTrajectoryWitnessConfigurationV1,
    template: ConstantCommandTemplate,
    terminal_physics_step: u64,
    kind: FiniteTemplateAttemptKindV1,
    attempt_index: u64,
) -> Result<FiniteTemplateProposalAttemptV1, String> {
    let witness = BoundedTrajectoryWitnessV1::new(
        input.physical_digest()?,
        witness_configuration.configuration_digest.clone(),
        commands_for_template(
            template,
            terminal_physics_step,
            witness_configuration.hold_steps,
        ),
        terminal_physics_step,
    )?;
    let verification = verify_bounded_trajectory(input, witness_configuration, &witness);
    if verification.status != BoundedTrajectoryArtifactStatusV1::Complete {
        return Err(format!(
            "finite proposal exact verification became invalid: {:?}",
            verification.first_reason
        ));
    }
    Ok(FiniteTemplateProposalAttemptV1 {
        attempt_index,
        template_id: template.id.to_owned(),
        kind,
        witness,
        verification,
    })
}

fn unknown_prediction(
    input: &RouteCapabilityInputV1,
    witness_configuration: &BoundedTrajectoryWitnessConfigurationV1,
    proposal_configuration: &BoundedTrajectoryProposalConfigurationV1,
    attempts: &[FiniteTemplateProposalAttemptV1],
    reason: &str,
) -> Result<BoundedTrajectoryWitnessPredictionV1, String> {
    BoundedTrajectoryWitnessPredictionV1 {
        schema_version: BOUNDED_TRAJECTORY_PREDICTION_SCHEMA_VERSION,
        status: BoundedTrajectoryArtifactStatusV1::Complete,
        decision: Some(BoundedTrajectoryDecisionV1::Unknown),
        input_digest: input.physical_digest()?,
        configuration_digest: witness_configuration.configuration_digest.clone(),
        proposal_configuration_digest: Some(proposal_configuration.configuration_digest.clone()),
        witness_digest: None,
        verification_digest: attempts
            .last()
            .map(|attempt| attempt.verification.verification_digest.clone()),
        attempted_verification_digests: attempts
            .iter()
            .map(|attempt| attempt.verification.verification_digest.clone())
            .collect(),
        first_reason: Some(reason.to_owned()),
        prediction_digest: String::new(),
    }
    .seal()
}

fn complete_result(
    proposal_configuration: &BoundedTrajectoryProposalConfigurationV1,
    attempts: Vec<FiniteTemplateProposalAttemptV1>,
    selected_attempt_index: Option<u64>,
    prediction: BoundedTrajectoryWitnessPredictionV1,
) -> Result<FiniteTemplateProposalSpikeResultV1, String> {
    FiniteTemplateProposalSpikeResultV1 {
        proposal_configuration: proposal_configuration.clone(),
        attempts,
        selected_attempt_index,
        prediction,
        result_digest: String::new(),
    }
    .seal()
}

/// Run the fixed W3 finite-template proposal spike.
///
/// Search-limit exhaustion is a valid `unknown` prediction.  The first exact
/// verified witness wins; no rejection or exhausted catalog is a negative
/// physical certificate.
pub fn run_finite_template_proposal_spike(
    input: &RouteCapabilityInputV1,
    witness_configuration: &BoundedTrajectoryWitnessConfigurationV1,
    proposal_configuration: &BoundedTrajectoryProposalConfigurationV1,
) -> Result<FiniteTemplateProposalSpikeResultV1, String> {
    input.validate()?;
    witness_configuration.validate()?;
    validate_finite_template_configuration(proposal_configuration)?;

    let scope_reason = match input.physical.topology {
        RouteTopology::Direct => Some("unknown/scope/direct_route"),
        RouteTopology::Waypoint if !(1..=2).contains(&input.physical.waypoints.len()) => {
            Some(UNKNOWN_SCOPE_WAYPOINT_COUNT)
        }
        RouteTopology::Waypoint => None,
    };
    if let Some(reason) = scope_reason {
        let prediction = unknown_prediction(
            input,
            witness_configuration,
            proposal_configuration,
            &[],
            reason,
        )?;
        return complete_result(proposal_configuration, Vec::new(), None, prediction);
    }

    let mut attempts = Vec::new();
    for template in command_templates(input.physical.horizontal_sign) {
        let mut probe_step = FINITE_TEMPLATE_PROBE_STEPS;
        while probe_step <= witness_configuration.max_physics_steps {
            if attempts.len() as u64 >= proposal_configuration.search_limit {
                let prediction = unknown_prediction(
                    input,
                    witness_configuration,
                    proposal_configuration,
                    &attempts,
                    BOUNDED_REASON_UNKNOWN_COVERAGE_BOUNDED_SEARCH_EXHAUSTED,
                )?;
                return complete_result(proposal_configuration, attempts, None, prediction);
            }

            let probe = attempt(
                input,
                witness_configuration,
                template,
                probe_step,
                FiniteTemplateAttemptKindV1::Probe,
                attempts.len() as u64,
            )?;
            let verified = probe.verification.verdict
                == Some(BoundedTrajectoryVerificationVerdictV1::Verified);
            let reconstructed_step = if probe.verification.first_reason.as_deref()
                == Some("rejected/terminal/nonminimal")
            {
                probe
                    .verification
                    .phases
                    .last()
                    .and_then(|phase| phase.terminal_physics_step)
                    .filter(|step| *step > 0 && *step < probe_step)
            } else {
                None
            };
            attempts.push(probe);

            if verified {
                return supported_result(
                    input,
                    witness_configuration,
                    proposal_configuration,
                    attempts,
                );
            }

            if let Some(terminal_step) = reconstructed_step {
                if attempts.len() as u64 >= proposal_configuration.search_limit {
                    let prediction = unknown_prediction(
                        input,
                        witness_configuration,
                        proposal_configuration,
                        &attempts,
                        BOUNDED_REASON_UNKNOWN_COVERAGE_BOUNDED_SEARCH_EXHAUSTED,
                    )?;
                    return complete_result(proposal_configuration, attempts, None, prediction);
                }
                let reconstructed = attempt(
                    input,
                    witness_configuration,
                    template,
                    terminal_step,
                    FiniteTemplateAttemptKindV1::ReconstructedTerminal,
                    attempts.len() as u64,
                )?;
                let verified = reconstructed.verification.verdict
                    == Some(BoundedTrajectoryVerificationVerdictV1::Verified);
                attempts.push(reconstructed);
                if verified {
                    return supported_result(
                        input,
                        witness_configuration,
                        proposal_configuration,
                        attempts,
                    );
                }
                break;
            }

            probe_step += FINITE_TEMPLATE_PROBE_STEPS;
        }
    }

    let prediction = unknown_prediction(
        input,
        witness_configuration,
        proposal_configuration,
        &attempts,
        BOUNDED_REASON_UNKNOWN_COVERAGE_BOUNDED_SEARCH_EXHAUSTED,
    )?;
    complete_result(proposal_configuration, attempts, None, prediction)
}

fn supported_result(
    input: &RouteCapabilityInputV1,
    witness_configuration: &BoundedTrajectoryWitnessConfigurationV1,
    proposal_configuration: &BoundedTrajectoryProposalConfigurationV1,
    attempts: Vec<FiniteTemplateProposalAttemptV1>,
) -> Result<FiniteTemplateProposalSpikeResultV1, String> {
    let selected_index = attempts
        .len()
        .checked_sub(1)
        .ok_or_else(|| "supported finite proposal has no attempt".to_owned())?;
    let selected = &attempts[selected_index];
    let mut prediction = BoundedTrajectoryWitnessPredictionV1::from_verification(
        input,
        witness_configuration,
        &selected.verification,
        Some(proposal_configuration.configuration_digest.clone()),
    )?;
    prediction.attempted_verification_digests = attempts
        .iter()
        .map(|attempt| attempt.verification.verification_digest.clone())
        .collect();
    prediction = prediction.seal()?;
    complete_result(
        proposal_configuration,
        attempts,
        Some(selected_index as u64),
        prediction,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ROUTE_CAPABILITY_INPUT_SCHEMA_VERSION, RouteCapabilityInputProvenanceV1,
        RouteCapabilityPadGeometryV1, RouteCapabilityPhysicalInputV1,
        RouteCapabilityPhysicalPolicyV1, RouteCapabilitySafetyProfileV1, RouteCapabilityWaypointV1,
    };
    use pd_core::{
        CorridorEnvelope, NormalizedRouteGeometry, TerrainDefinition, Vec2, VehicleGeometry,
        VehicleInitialState, VehicleSpec,
    };

    fn test_input() -> RouteCapabilityInputV1 {
        RouteCapabilityInputV1 {
            schema_version: ROUTE_CAPABILITY_INPUT_SCHEMA_VERSION,
            physical: RouteCapabilityPhysicalInputV1 {
                gravity_mps2: 1.0,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-20.0, 0.0), Vec2::new(120.0, 0.0)],
                },
                source_pad: RouteCapabilityPadGeometryV1 {
                    center_x_m: 0.0,
                    surface_y_m: 0.0,
                    width_m: 30.0,
                },
                target_pad: RouteCapabilityPadGeometryV1 {
                    center_x_m: 100.0,
                    surface_y_m: 0.0,
                    width_m: 30.0,
                },
                vehicle: VehicleSpec {
                    geometry: VehicleGeometry {
                        hull_width_m: 4.0,
                        hull_height_m: 6.0,
                        touchdown_half_span_m: 2.0,
                        touchdown_base_offset_m: 3.0,
                    },
                    dry_mass_kg: 700.0,
                    initial_fuel_kg: 200.0,
                    max_fuel_kg: 200.0,
                    max_thrust_n: 900.0,
                    max_fuel_burn_kgps: 1.0e-9,
                    min_throttle_frac: 0.25,
                    max_rotation_rate_radps: 1.0,
                    safe_touchdown_normal_speed_mps: 3.0,
                    safe_touchdown_tangential_speed_mps: 2.0,
                    safe_touchdown_attitude_error_rad: 0.15,
                    safe_touchdown_angular_rate_radps: 0.35,
                },
                initial_state: VehicleInitialState {
                    position_m: Vec2::new(0.0, 4.0),
                    velocity_mps: Vec2::new(5.0, 0.0),
                    attitude_rad: 0.0,
                    angular_rate_radps: 0.0,
                },
                policy: RouteCapabilityPhysicalPolicyV1 {
                    max_waypoints: 2,
                    flight_clearance_margin_m: 1.0,
                    endpoint_transition_m: 17.0,
                    max_extra_loft_ratio: 1.0,
                    max_continuation_ratio: 1.0,
                    max_handoff_speed_mps: 100.0,
                    min_handoff_speed_mps: 0.1,
                    min_outbound_progress_mps: 0.1,
                    max_outbound_heading_error_rad: PI,
                    max_outbound_cross_speed_mps: 100.0,
                },
                safety_profile: RouteCapabilitySafetyProfileV1 {
                    source_transition_start_m: 17.0,
                    source_transition_end_m: 34.0,
                    target_transition_start_m: 66.0,
                    target_transition_end_m: 83.0,
                    horizontal_span_m: 100.0,
                    full_envelope: CorridorEnvelope::new(5.0, 3.0),
                    contact_envelope: CorridorEnvelope::new(2.0, 3.0),
                },
                selected_centerline_m: vec![
                    Vec2::new(0.0, 4.0),
                    Vec2::new(34.0, 4.0),
                    Vec2::new(60.0, 4.0),
                    Vec2::new(83.0, 4.0),
                    Vec2::new(100.0, 4.0),
                ],
                normalized_geometry: NormalizedRouteGeometry {
                    horizontal_sign: 1,
                    direct_horizontal_span_m: 100.0,
                    direct_distance_m: 100.0,
                    route_angle_rad: 0.0,
                    route_angle_deg: 0.0,
                },
                horizontal_sign: 1,
                waypoints: vec![RouteCapabilityWaypointV1 {
                    position_m: Vec2::new(60.0, 4.0),
                    handoff_tangent_unit: Some(Vec2::new(1.0, 0.0)),
                    capture_radius_m: 20.001,
                    max_cross_track_m: 20.0,
                    max_outbound_heading_error_rad: PI,
                    min_outbound_progress_mps: 0.1,
                    max_outbound_cross_speed_mps: Some(100.0),
                    min_speed_mps: 0.1,
                    max_speed_mps: 100.0,
                    min_vertical_speed_mps: Some(-100.0),
                    max_vertical_speed_mps: Some(100.0),
                }],
                topology: RouteTopology::Waypoint,
                route_angle_deg: 0.0,
                route_radius_m: 100.0,
            },
            provenance: RouteCapabilityInputProvenanceV1 {
                request_digest: "request-test".to_owned(),
                route_plan_digest: "plan-test".to_owned(),
                source_pad_id: "source".to_owned(),
                target_pad_id: "target".to_owned(),
                waypoint_ids: vec!["waypoint".to_owned()],
            },
            input_digest: String::new(),
        }
        .seal()
        .expect("synthetic proposal input seals")
    }

    fn two_waypoint_input() -> RouteCapabilityInputV1 {
        let mut input = test_input();
        let template = input.physical.waypoints[0];
        input.physical.waypoints = vec![
            RouteCapabilityWaypointV1 {
                position_m: Vec2::new(50.0, 4.0),
                capture_radius_m: 10.001,
                ..template
            },
            RouteCapabilityWaypointV1 {
                position_m: Vec2::new(75.0, 4.0),
                capture_radius_m: 10.001,
                ..template
            },
        ];
        input.physical.selected_centerline_m = vec![
            Vec2::new(0.0, 4.0),
            Vec2::new(34.0, 4.0),
            Vec2::new(50.0, 4.0),
            Vec2::new(75.0, 4.0),
            Vec2::new(83.0, 4.0),
            Vec2::new(100.0, 4.0),
        ];
        input.provenance.waypoint_ids = vec!["waypoint-0".to_owned(), "waypoint-1".to_owned()];
        input.seal().expect("two-waypoint proposal input seals")
    }

    fn witness_configuration() -> BoundedTrajectoryWitnessConfigurationV1 {
        BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("witness configuration seals")
    }

    #[test]
    fn finite_configuration_is_closed_and_tamper_evident() {
        let configuration = finite_template_proposal_configuration_v1(8)
            .expect("finite proposal configuration seals");
        validate_finite_template_configuration(&configuration)
            .expect("finite proposal configuration validates");

        let mut tampered = configuration;
        tampered.parameterization_id = "different-catalog".to_owned();
        tampered = tampered.seal().expect("generic proposal config reseals");
        assert!(validate_finite_template_configuration(&tampered).is_err());
        assert!(
            finite_template_proposal_configuration_v1(FINITE_TEMPLATE_MAX_SEARCH_LIMIT + 1)
                .is_err()
        );
    }

    #[test]
    fn finite_search_reconstructs_exact_one_and_two_waypoint_witnesses() {
        let witness_configuration = witness_configuration();
        let proposal_configuration = finite_template_proposal_configuration_v1(8)
            .expect("finite proposal configuration seals");

        let mut results = Vec::new();
        for (input, terminal_step) in [(test_input(), 960), (two_waypoint_input(), 1_560)] {
            let result = run_finite_template_proposal_spike(
                &input,
                &witness_configuration,
                &proposal_configuration,
            )
            .expect("finite proposal runs");
            assert_eq!(
                result.prediction.decision,
                Some(BoundedTrajectoryDecisionV1::Supported)
            );
            let selected =
                &result.attempts[result.selected_attempt_index.expect("selected attempt") as usize];
            assert_eq!(
                selected.kind,
                FiniteTemplateAttemptKindV1::ReconstructedTerminal
            );
            assert_eq!(selected.witness.terminal_physics_step, terminal_step);
            result
                .validate_against_exact(&input, &witness_configuration)
                .expect("finite proposal exact validation");
            results.push(result);
        }

        let mut mixed = results[1].clone();
        mixed.attempts[0] = results[0].attempts[0].clone();
        mixed.result_digest = mixed.compute_digest().expect("mixed result digest");
        assert!(mixed.validate().is_err());

        let mut invalid_selection = results[0].clone();
        invalid_selection.selected_attempt_index = Some(u64::MAX);
        invalid_selection.result_digest = invalid_selection
            .compute_digest()
            .expect("invalid-selection result digest");
        assert!(invalid_selection.validate().is_err());
    }

    #[test]
    fn finite_search_is_byte_deterministic_and_budget_exhaustion_is_unknown() {
        let input = two_waypoint_input();
        let witness_configuration = witness_configuration();
        let enough = finite_template_proposal_configuration_v1(8)
            .expect("finite proposal configuration seals");
        let first = run_finite_template_proposal_spike(&input, &witness_configuration, &enough)
            .expect("first finite proposal");
        let second = run_finite_template_proposal_spike(&input, &witness_configuration, &enough)
            .expect("second finite proposal");
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).expect("first bytes"),
            serde_json::to_vec(&second).expect("second bytes")
        );

        let exhausted = finite_template_proposal_configuration_v1(1)
            .expect("bounded proposal configuration seals");
        let result = run_finite_template_proposal_spike(&input, &witness_configuration, &exhausted)
            .expect("bounded finite proposal");
        assert_eq!(
            result.prediction.decision,
            Some(BoundedTrajectoryDecisionV1::Unknown)
        );
        assert_eq!(
            result.prediction.first_reason.as_deref(),
            Some(BOUNDED_REASON_UNKNOWN_COVERAGE_BOUNDED_SEARCH_EXHAUSTED)
        );
        assert_eq!(result.attempts.len(), 1);
        result.validate().expect("exhausted result validates");
    }

    #[test]
    fn direct_scope_abstains_without_running_a_template() {
        let mut input = test_input();
        input.physical.topology = RouteTopology::Direct;
        input.physical.waypoints.clear();
        input.provenance.waypoint_ids.clear();
        input = input.seal().expect("direct input seals");
        let witness_configuration = witness_configuration();
        let proposal_configuration = finite_template_proposal_configuration_v1(8)
            .expect("finite proposal configuration seals");
        let result = run_finite_template_proposal_spike(
            &input,
            &witness_configuration,
            &proposal_configuration,
        )
        .expect("direct scope abstention");
        assert!(result.attempts.is_empty());
        assert_eq!(result.selected_attempt_index, None);
        assert_eq!(
            result.prediction.first_reason.as_deref(),
            Some("unknown/scope/direct_route")
        );
        result.validate().expect("scope result validates");
    }
}
