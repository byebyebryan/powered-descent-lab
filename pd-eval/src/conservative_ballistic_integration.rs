//! F6 controller-free integration boundary for the conservative ballistic route.
//!
//! The analytical and runtime projections remain owned by `pd-plan`. This
//! module selects the derived-mesa outcome into a compact evaluator decision;
//! it neither calls the ordinary planner nor executes a controller.

use std::fmt;

use pd_core::{RouteTopology, TransferRouteSpec};
use pd_plan::conservative_ballistic_bridge::{
    DirectBridgeReasonV2, ExperimentalRidgeCandidateErrorV2, ExperimentalRidgeCaseInputV1,
    ExperimentalRidgeCaseProjectionV1, ExperimentalRidgeCaseRuntimeOutcomeV2,
    ExperimentalRidgeCaseRuntimeProjectionErrorV2, ExperimentalRidgeCaseRuntimeProjectionV2,
    ExperimentalRidgeRuntimeHandoffSelectionKindV2, ExperimentalRidgeUnsupportedReasonV2,
    evaluate_experimental_ridge_case_projection_v1, project_experimental_ridge_case_runtime_v2,
    validate_experimental_ridge_case_projection_v1, validate_experimental_ridge_case_runtime_v2,
};
use serde::{Deserialize, Serialize};

use crate::canonical_digest;

pub const CONSERVATIVE_BALLISTIC_F6_INPUT_SCHEMA_ID_V1: &str =
    "conservative_ballistic_ridge_f6_integration_input_v1";
pub const CONSERVATIVE_BALLISTIC_F6_INPUT_SCHEMA_VERSION_V1: u32 = 1;
pub const CONSERVATIVE_BALLISTIC_F6_ROUTE_SOURCE_V1: &str = "derived_mesa_runtime_v2";
pub const CONSERVATIVE_BALLISTIC_F6_SOURCE_CASE_ID_V1: &str = "ridge_progress_056_probe";
pub const CONSERVATIVE_BALLISTIC_F6_SOURCE_INPUT_IDENTITY_V1: &str = "fnv1a64:1685aab304642342";
pub const CONSERVATIVE_BALLISTIC_F6_INPUT_FIXTURE_V1: &str =
    "fixtures/manifests/conservative_ballistic_ridge_f6_integration_input_v1.json";

pub const CONSERVATIVE_BALLISTIC_F6_DECISION_SCHEMA_ID_V1: &str =
    "conservative_ballistic_integration_decision_v1";
pub const CONSERVATIVE_BALLISTIC_F6_DECISION_SCHEMA_VERSION_V1: u32 = 1;
pub const CONSERVATIVE_BALLISTIC_F6_APPLICATION_SCHEMA_ID_V1: &str =
    "conservative_ballistic_scenario_application_v1";
pub const CONSERVATIVE_BALLISTIC_F6_APPLICATION_SCHEMA_VERSION_V1: u32 = 1;

const EMBEDDED_F6_INPUT: &str = include_str!(concat!(
    "../../fixtures/manifests/",
    "conservative_ballistic_ridge_f6_integration_input_v1.json"
));

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticF6InputV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub route_source: String,
    pub input: ExperimentalRidgeCaseInputV1,
    pub identity: String,
}

#[derive(Serialize)]
struct F6InputIdentityMaterial<'a> {
    schema_id: &'a str,
    schema_version: u32,
    route_source: &'a str,
    input: &'a ExperimentalRidgeCaseInputV1,
}

impl ConservativeBallisticF6InputV1 {
    pub fn validate(&self) -> Result<(), ConservativeBallisticIntegrationErrorV1> {
        if self.schema_id != CONSERVATIVE_BALLISTIC_F6_INPUT_SCHEMA_ID_V1
            || self.schema_version != CONSERVATIVE_BALLISTIC_F6_INPUT_SCHEMA_VERSION_V1
            || self.route_source != CONSERVATIVE_BALLISTIC_F6_ROUTE_SOURCE_V1
        {
            return Err(ConservativeBallisticIntegrationErrorV1::FixtureContract);
        }
        self.input
            .validate()
            .map_err(ConservativeBallisticIntegrationErrorV1::Candidate)?;
        if self.input.probe.id != CONSERVATIVE_BALLISTIC_F6_SOURCE_CASE_ID_V1
            || self.input.identity != CONSERVATIVE_BALLISTIC_F6_SOURCE_INPUT_IDENTITY_V1
        {
            return Err(ConservativeBallisticIntegrationErrorV1::FixtureSourceMismatch);
        }
        let expected = canonical_digest(&F6InputIdentityMaterial {
            schema_id: &self.schema_id,
            schema_version: self.schema_version,
            route_source: &self.route_source,
            input: &self.input,
        })
        .map_err(ConservativeBallisticIntegrationErrorV1::Identity)?;
        if self.identity != expected {
            return Err(ConservativeBallisticIntegrationErrorV1::FixtureIdentityMismatch);
        }
        Ok(())
    }
}

pub fn parse_conservative_ballistic_f6_input_v1(
    raw: &str,
) -> Result<ConservativeBallisticF6InputV1, ConservativeBallisticIntegrationErrorV1> {
    let raw_value = serde_json::from_str::<serde_json::Value>(raw)
        .map_err(|_| ConservativeBallisticIntegrationErrorV1::FixtureJson)?;
    let fixture = serde_json::from_value::<ConservativeBallisticF6InputV1>(raw_value.clone())
        .map_err(|_| ConservativeBallisticIntegrationErrorV1::FixtureJson)?;
    let parsed_value = serde_json::to_value(&fixture)
        .map_err(|_| ConservativeBallisticIntegrationErrorV1::FixtureJson)?;
    if raw_value != parsed_value {
        return Err(ConservativeBallisticIntegrationErrorV1::FixtureShape);
    }
    fixture.validate()?;
    Ok(fixture)
}

pub fn load_conservative_ballistic_f6_input_v1()
-> Result<ConservativeBallisticF6InputV1, ConservativeBallisticIntegrationErrorV1> {
    parse_conservative_ballistic_f6_input_v1(EMBEDDED_F6_INPUT)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservativeBallisticIntegrationDispositionV1 {
    SupportedDirect,
    SupportedOneWaypoint,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservativeBallisticFallbackDispositionV1 {
    NotSelected,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticIntegrationProvenanceV1 {
    pub route_source: String,
    pub input_identity: String,
    pub analytical_policy_identity: String,
    pub analytical_projection_identity: String,
    pub analytical_canary_identity: String,
    pub runtime_projection_identity: String,
    pub selected_candidate_identity: Option<String>,
    pub selected_outcome_identity: String,
    pub route_identity: Option<String>,
    pub topology: Option<RouteTopology>,
    pub waypoint_count: Option<usize>,
    pub derived_mesa_identity: String,
    pub selected_handoff_attempt_index: Option<usize>,
    pub selected_handoff_selection_kind: Option<ExperimentalRidgeRuntimeHandoffSelectionKindV2>,
    pub selected_handoff_selection_identity: Option<String>,
    pub selected_handoff_attempt_identity: Option<String>,
    pub disposition: ConservativeBallisticIntegrationDispositionV1,
    pub fallback: ConservativeBallisticFallbackDispositionV1,
    pub controller_run: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticIntegrationDecisionV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub input: ExperimentalRidgeCaseInputV1,
    pub analytical_projection: ExperimentalRidgeCaseProjectionV1,
    pub runtime_projection: ExperimentalRidgeCaseRuntimeProjectionV2,
    pub provenance: ConservativeBallisticIntegrationProvenanceV1,
    pub route: Option<TransferRouteSpec>,
    pub unsupported_reason: Option<ExperimentalRidgeUnsupportedReasonV2>,
    pub rejection_reasons: Vec<DirectBridgeReasonV2>,
    pub identity: String,
}

#[derive(Serialize)]
struct F6DecisionIdentityMaterial<'a> {
    schema_id: &'a str,
    schema_version: u32,
    input: &'a ExperimentalRidgeCaseInputV1,
    analytical_projection: &'a ExperimentalRidgeCaseProjectionV1,
    runtime_projection: &'a ExperimentalRidgeCaseRuntimeProjectionV2,
    provenance: &'a ConservativeBallisticIntegrationProvenanceV1,
    route: &'a Option<TransferRouteSpec>,
    unsupported_reason: &'a Option<ExperimentalRidgeUnsupportedReasonV2>,
    rejection_reasons: &'a [DirectBridgeReasonV2],
}

impl ConservativeBallisticIntegrationDecisionV1 {
    fn finalize_identity(&mut self) -> Result<(), ConservativeBallisticIntegrationErrorV1> {
        self.identity = decision_identity(self)?;
        Ok(())
    }

    pub fn validate_against_input(
        &self,
        input: &ExperimentalRidgeCaseInputV1,
    ) -> Result<(), ConservativeBallisticIntegrationErrorV1> {
        validate_decision_shape(self)?;
        if &self.input != input {
            return Err(ConservativeBallisticIntegrationErrorV1::DecisionMismatch);
        }
        self.input
            .validate()
            .map_err(ConservativeBallisticIntegrationErrorV1::Candidate)?;
        validate_experimental_ridge_case_projection_v1(&self.analytical_projection)
            .map_err(ConservativeBallisticIntegrationErrorV1::Candidate)?;
        validate_experimental_ridge_case_runtime_v2(
            &self.analytical_projection,
            &self.runtime_projection,
        )
        .map_err(ConservativeBallisticIntegrationErrorV1::Runtime)?;
        if self.identity != decision_identity(self)? {
            return Err(ConservativeBallisticIntegrationErrorV1::DecisionIdentityMismatch);
        }
        let expected = resolve_conservative_ballistic_integration_v1(input)?;
        if self != &expected {
            return Err(ConservativeBallisticIntegrationErrorV1::DecisionMismatch);
        }
        Ok(())
    }
}

fn decision_identity(
    decision: &ConservativeBallisticIntegrationDecisionV1,
) -> Result<String, ConservativeBallisticIntegrationErrorV1> {
    canonical_digest(&F6DecisionIdentityMaterial {
        schema_id: &decision.schema_id,
        schema_version: decision.schema_version,
        input: &decision.input,
        analytical_projection: &decision.analytical_projection,
        runtime_projection: &decision.runtime_projection,
        provenance: &decision.provenance,
        route: &decision.route,
        unsupported_reason: &decision.unsupported_reason,
        rejection_reasons: &decision.rejection_reasons,
    })
    .map_err(ConservativeBallisticIntegrationErrorV1::Identity)
}

fn validate_decision_shape(
    decision: &ConservativeBallisticIntegrationDecisionV1,
) -> Result<(), ConservativeBallisticIntegrationErrorV1> {
    if decision.schema_id != CONSERVATIVE_BALLISTIC_F6_DECISION_SCHEMA_ID_V1
        || decision.schema_version != CONSERVATIVE_BALLISTIC_F6_DECISION_SCHEMA_VERSION_V1
        || decision.provenance.route_source != CONSERVATIVE_BALLISTIC_F6_ROUTE_SOURCE_V1
        || decision.provenance.controller_run
        || decision.provenance.fallback != ConservativeBallisticFallbackDispositionV1::NotSelected
        || decision.input.identity != decision.provenance.input_identity
        || decision.analytical_projection.input_identity != decision.input.identity
        || decision.analytical_projection.identity
            != decision.provenance.analytical_projection_identity
        || decision.analytical_projection.analytical_canary_identity
            != decision.provenance.analytical_canary_identity
        || decision.runtime_projection.input_identity != decision.input.identity
        || decision.runtime_projection.identity != decision.provenance.runtime_projection_identity
        || decision.runtime_projection.analytical_canary_identity
            != decision.provenance.analytical_canary_identity
        || decision.analytical_projection.mesa.identity != decision.provenance.derived_mesa_identity
        || decision.provenance.analytical_policy_identity
            != canonical_digest(&decision.input.policy)
                .map_err(ConservativeBallisticIntegrationErrorV1::Identity)?
        || decision.provenance.selected_outcome_identity.is_empty()
    {
        return Err(ConservativeBallisticIntegrationErrorV1::DecisionContract);
    }
    match decision.provenance.disposition {
        ConservativeBallisticIntegrationDispositionV1::SupportedDirect => {
            let route = decision
                .route
                .as_ref()
                .ok_or(ConservativeBallisticIntegrationErrorV1::DecisionContract)?;
            if !route.waypoints.is_empty()
                || decision.provenance.topology != Some(RouteTopology::Direct)
                || decision.provenance.waypoint_count != Some(0)
                || decision.unsupported_reason.is_some()
                || !decision.rejection_reasons.is_empty()
                || decision.provenance.selected_handoff_attempt_index.is_some()
                || decision
                    .provenance
                    .selected_handoff_selection_kind
                    .is_some()
                || decision
                    .provenance
                    .selected_handoff_selection_identity
                    .is_some()
                || decision
                    .provenance
                    .selected_handoff_attempt_identity
                    .is_some()
            {
                return Err(ConservativeBallisticIntegrationErrorV1::DecisionContract);
            }
        }
        ConservativeBallisticIntegrationDispositionV1::SupportedOneWaypoint => {
            let route = decision
                .route
                .as_ref()
                .ok_or(ConservativeBallisticIntegrationErrorV1::DecisionContract)?;
            if route.waypoints.len() != 1
                || decision.provenance.topology != Some(RouteTopology::Waypoint)
                || decision.provenance.waypoint_count != Some(1)
                || decision.unsupported_reason.is_some()
                || !decision.rejection_reasons.is_empty()
                || decision.provenance.selected_handoff_attempt_index.is_none()
                || decision
                    .provenance
                    .selected_handoff_selection_kind
                    .is_none()
                || decision
                    .provenance
                    .selected_handoff_selection_identity
                    .is_none()
                || decision
                    .provenance
                    .selected_handoff_attempt_identity
                    .is_none()
            {
                return Err(ConservativeBallisticIntegrationErrorV1::DecisionContract);
            }
        }
        ConservativeBallisticIntegrationDispositionV1::Unsupported => {
            if decision.route.is_some()
                || decision.provenance.route_identity.is_some()
                || decision.provenance.topology.is_some()
                || decision.provenance.waypoint_count.is_some()
                || decision.provenance.selected_candidate_identity.is_some()
                || decision.unsupported_reason.is_none()
                || decision.provenance.selected_handoff_attempt_index.is_some()
                || decision
                    .provenance
                    .selected_handoff_selection_kind
                    .is_some()
                || decision
                    .provenance
                    .selected_handoff_selection_identity
                    .is_some()
                || decision
                    .provenance
                    .selected_handoff_attempt_identity
                    .is_some()
            {
                return Err(ConservativeBallisticIntegrationErrorV1::DecisionContract);
            }
        }
    }
    if let Some(route) = &decision.route {
        route
            .validate()
            .map_err(|_| ConservativeBallisticIntegrationErrorV1::DecisionContract)?;
        if decision.provenance.route_identity
            != Some(
                canonical_digest(route)
                    .map_err(ConservativeBallisticIntegrationErrorV1::Identity)?,
            )
            || decision.provenance.selected_candidate_identity.is_none()
        {
            return Err(ConservativeBallisticIntegrationErrorV1::DecisionContract);
        }
    }
    Ok(())
}

pub fn resolve_conservative_ballistic_integration_v1(
    input: &ExperimentalRidgeCaseInputV1,
) -> Result<ConservativeBallisticIntegrationDecisionV1, ConservativeBallisticIntegrationErrorV1> {
    input
        .validate()
        .map_err(ConservativeBallisticIntegrationErrorV1::Candidate)?;
    let candidate = evaluate_experimental_ridge_case_projection_v1(input)
        .map_err(ConservativeBallisticIntegrationErrorV1::Candidate)?;
    validate_experimental_ridge_case_projection_v1(&candidate)
        .map_err(ConservativeBallisticIntegrationErrorV1::Candidate)?;
    let runtime = project_experimental_ridge_case_runtime_v2(&candidate)
        .map_err(ConservativeBallisticIntegrationErrorV1::Runtime)?;
    validate_experimental_ridge_case_runtime_v2(&candidate, &runtime)
        .map_err(ConservativeBallisticIntegrationErrorV1::Runtime)?;
    let analytical_policy_identity = canonical_digest(&input.policy)
        .map_err(ConservativeBallisticIntegrationErrorV1::Identity)?;

    let base = |disposition| ConservativeBallisticIntegrationProvenanceV1 {
        route_source: CONSERVATIVE_BALLISTIC_F6_ROUTE_SOURCE_V1.to_owned(),
        input_identity: input.identity.clone(),
        analytical_policy_identity: analytical_policy_identity.clone(),
        analytical_projection_identity: candidate.identity.clone(),
        analytical_canary_identity: candidate.analytical_canary_identity.clone(),
        runtime_projection_identity: runtime.identity.clone(),
        selected_candidate_identity: None,
        selected_outcome_identity: String::new(),
        route_identity: None,
        topology: None,
        waypoint_count: None,
        derived_mesa_identity: candidate.mesa.identity.clone(),
        selected_handoff_attempt_index: None,
        selected_handoff_selection_kind: None,
        selected_handoff_selection_identity: None,
        selected_handoff_attempt_identity: None,
        disposition,
        fallback: ConservativeBallisticFallbackDispositionV1::NotSelected,
        controller_run: false,
    };

    let (mut provenance, route, unsupported_reason, rejection_reasons) = match &runtime.derived_mesa
    {
        ExperimentalRidgeCaseRuntimeOutcomeV2::Direct {
            candidate_identity,
            route,
            structural_validation,
            identity,
        } => {
            if !structural_validation.transfer_route_valid || !route.waypoints.is_empty() {
                return Err(ConservativeBallisticIntegrationErrorV1::DecisionContract);
            }
            let mut provenance =
                base(ConservativeBallisticIntegrationDispositionV1::SupportedDirect);
            provenance.selected_candidate_identity = Some(candidate_identity.clone());
            provenance.selected_outcome_identity = identity.clone();
            provenance.route_identity = Some(
                canonical_digest(route)
                    .map_err(ConservativeBallisticIntegrationErrorV1::Identity)?,
            );
            provenance.topology = Some(RouteTopology::Direct);
            provenance.waypoint_count = Some(0);
            (provenance, Some(route.clone()), None, Vec::new())
        }
        ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
            candidate_identity,
            route,
            handoff_selection,
            handoff_assessment,
            structural_validation,
            identity,
            ..
        } => {
            let selected = handoff_selection
                .selected_attempt()
                .ok_or(ConservativeBallisticIntegrationErrorV1::DecisionContract)?;
            if !structural_validation.transfer_route_valid
                || !handoff_assessment.contract_pass
                || !selected.structural_validation.transfer_route_valid
                || !selected.handoff_assessment.contract_pass
                || selected.route != *route
                || route.waypoints.len() != 1
            {
                return Err(ConservativeBallisticIntegrationErrorV1::DecisionContract);
            }
            let mut provenance =
                base(ConservativeBallisticIntegrationDispositionV1::SupportedOneWaypoint);
            provenance.selected_candidate_identity = Some(candidate_identity.clone());
            provenance.selected_outcome_identity = identity.clone();
            provenance.route_identity = Some(
                canonical_digest(route)
                    .map_err(ConservativeBallisticIntegrationErrorV1::Identity)?,
            );
            provenance.topology = Some(RouteTopology::Waypoint);
            provenance.waypoint_count = Some(1);
            provenance.selected_handoff_attempt_index =
                Some(handoff_selection.selected_attempt_index);
            provenance.selected_handoff_selection_kind = Some(selected.selection_kind);
            provenance.selected_handoff_selection_identity =
                Some(handoff_selection.identity.clone());
            provenance.selected_handoff_attempt_identity = Some(selected.identity.clone());
            (provenance, Some(route.clone()), None, Vec::new())
        }
        ExperimentalRidgeCaseRuntimeOutcomeV2::Unsupported {
            reason,
            rejection_reasons,
            identity,
        } => {
            let mut provenance = base(ConservativeBallisticIntegrationDispositionV1::Unsupported);
            provenance.selected_outcome_identity = identity.clone();
            (provenance, None, Some(*reason), rejection_reasons.clone())
        }
    };
    // The selection is controller-free by construction; keep the assignment
    // explicit so later adapters cannot inherit a stale true value.
    provenance.controller_run = false;
    let mut decision = ConservativeBallisticIntegrationDecisionV1 {
        schema_id: CONSERVATIVE_BALLISTIC_F6_DECISION_SCHEMA_ID_V1.to_owned(),
        schema_version: CONSERVATIVE_BALLISTIC_F6_DECISION_SCHEMA_VERSION_V1,
        input: input.clone(),
        analytical_projection: candidate,
        runtime_projection: runtime,
        provenance,
        route,
        unsupported_reason,
        rejection_reasons,
        identity: String::new(),
    };
    decision.finalize_identity()?;
    validate_decision_shape(&decision)?;
    Ok(decision)
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConservativeBallisticIntegrationErrorV1 {
    FixtureJson,
    FixtureShape,
    FixtureContract,
    FixtureSourceMismatch,
    FixtureIdentityMismatch,
    Candidate(ExperimentalRidgeCandidateErrorV2),
    Runtime(ExperimentalRidgeCaseRuntimeProjectionErrorV2),
    Identity(String),
    DecisionContract,
    DecisionIdentityMismatch,
    DecisionMismatch,
}

impl fmt::Display for ConservativeBallisticIntegrationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FixtureJson => formatter.write_str("F6 input fixture is not valid JSON"),
            Self::FixtureShape => {
                formatter.write_str("F6 input fixture contains fields outside its typed contract")
            }
            Self::FixtureContract => formatter.write_str("F6 input fixture contract is invalid"),
            Self::FixtureSourceMismatch => {
                formatter.write_str("F6 input fixture does not bind the retained 056 source")
            }
            Self::FixtureIdentityMismatch => {
                formatter.write_str("F6 input fixture identity does not bind its contents")
            }
            Self::Candidate(error) => write!(formatter, "F6 candidate projection failed: {error}"),
            Self::Runtime(error) => write!(formatter, "F6 runtime projection failed: {error}"),
            Self::Identity(error) => write!(formatter, "F6 identity failed: {error}"),
            Self::DecisionContract => formatter.write_str("F6 decision contract is invalid"),
            Self::DecisionIdentityMismatch => {
                formatter.write_str("F6 decision identity does not bind its contents")
            }
            Self::DecisionMismatch => {
                formatter.write_str("F6 decision does not recompute from its input")
            }
        }
    }
}

impl std::error::Error for ConservativeBallisticIntegrationErrorV1 {}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::Vec2;

    #[test]
    fn f6_fixture_contains_only_the_retained_056_input() {
        assert!(!EMBEDDED_F6_INPUT.contains("072"));
        assert!(!EMBEDDED_F6_INPUT.contains("prediction"));
        assert!(!EMBEDDED_F6_INPUT.contains("outcome"));
        let fixture = load_conservative_ballistic_f6_input_v1().unwrap();
        assert_eq!(
            fixture.input.probe.id,
            CONSERVATIVE_BALLISTIC_F6_SOURCE_CASE_ID_V1
        );
        assert_eq!(
            fixture.input.identity,
            CONSERVATIVE_BALLISTIC_F6_SOURCE_INPUT_IDENTITY_V1
        );
    }

    #[test]
    fn f6_fixture_rejects_nested_unknown_fields_and_identity_tampering() {
        let mut value: serde_json::Value = serde_json::from_str(EMBEDDED_F6_INPUT).unwrap();
        value["input"]["probe"]["prediction"] = serde_json::Value::Bool(true);
        assert_eq!(
            parse_conservative_ballistic_f6_input_v1(&value.to_string()),
            Err(ConservativeBallisticIntegrationErrorV1::FixtureShape)
        );

        let mut fixture = load_conservative_ballistic_f6_input_v1().unwrap();
        fixture.identity = "000000000000".to_owned();
        assert_eq!(
            fixture.validate(),
            Err(ConservativeBallisticIntegrationErrorV1::FixtureIdentityMismatch)
        );
    }

    #[test]
    fn f6_056_decision_is_deterministic_and_one_waypoint() {
        let fixture = load_conservative_ballistic_f6_input_v1().unwrap();
        let first = resolve_conservative_ballistic_integration_v1(&fixture.input).unwrap();
        let second = resolve_conservative_ballistic_integration_v1(&fixture.input).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.provenance.disposition,
            ConservativeBallisticIntegrationDispositionV1::SupportedOneWaypoint
        );
        assert_eq!(first.provenance.waypoint_count, Some(1));
        assert_eq!(first.route.as_ref().unwrap().waypoints.len(), 1);
        assert!(!first.provenance.controller_run);
        first.validate_against_input(&fixture.input).unwrap();
    }

    #[test]
    fn f6_decision_rejects_tampering() {
        let fixture = load_conservative_ballistic_f6_input_v1().unwrap();
        let mut decision = resolve_conservative_ballistic_integration_v1(&fixture.input).unwrap();
        decision.provenance.controller_run = true;
        assert_eq!(
            decision.validate_against_input(&fixture.input),
            Err(ConservativeBallisticIntegrationErrorV1::DecisionContract)
        );

        let mut decision = resolve_conservative_ballistic_integration_v1(&fixture.input).unwrap();
        decision.runtime_projection.identity = "fnv1a64:0000000000000000".to_owned();
        assert!(decision.validate_against_input(&fixture.input).is_err());
    }

    #[test]
    fn generic_terminal_blocker_is_valid_unsupported_not_an_error() {
        let fixture = load_conservative_ballistic_f6_input_v1().unwrap();
        let mut probe = fixture.input.probe.clone();
        probe.id = "generic_terminal_blocker".to_owned();
        probe.terrain_points_m = vec![
            Vec2::new(-40.0, 0.0),
            Vec2::new(2675.0, 0.0),
            Vec2::new(2725.0, 1200.0),
            Vec2::new(2875.0, 1200.0),
            Vec2::new(2975.0, 0.0),
            Vec2::new(4040.0, 0.0),
        ];
        let input = ExperimentalRidgeCaseInputV1::new(
            fixture.input.policy.clone(),
            fixture.input.vehicle.clone(),
            probe,
        )
        .unwrap();
        let decision = resolve_conservative_ballistic_integration_v1(&input).unwrap();
        assert_eq!(
            decision.provenance.disposition,
            ConservativeBallisticIntegrationDispositionV1::Unsupported
        );
        assert!(decision.route.is_none());
        assert!(decision.unsupported_reason.is_some());
        assert!(!decision.provenance.controller_run);
        decision.validate_against_input(&input).unwrap();
    }
}
