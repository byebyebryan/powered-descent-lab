//! F5 source seals for two fresh conservative-ballistic ridge probes.
//!
//! This module deliberately owns only immutable raw inputs and qualitative
//! predictions. It validates JSON, raw input identities, ordering, and the
//! cross-manifest identity chain, but does not import or invoke analytical,
//! runtime-projection, controller, simulator, scenario, or report APIs. Those
//! reveal stages begin only after this source seal is committed.

use std::collections::BTreeSet;

use pd_plan::conservative_ballistic_bridge::ExperimentalRidgeCaseInputV1;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::canonical_digest;

const INPUTS_FIXTURE_V1: &str =
    include_str!("../../fixtures/manifests/conservative_ballistic_ridge_f5_inputs_v1.json");
const PREDICTIONS_FIXTURE_V1: &str =
    include_str!("../../fixtures/manifests/conservative_ballistic_ridge_f5_predictions_v1.json");

pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_INPUTS_SCHEMA_ID_V1: &str =
    "conservative_ballistic_ridge_f5_inputs_v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_INPUTS_SCHEMA_VERSION_V1: u32 = 1;
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_PREDICTIONS_SCHEMA_ID_V1: &str =
    "conservative_ballistic_ridge_f5_predictions_v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_PREDICTIONS_SCHEMA_VERSION_V1: u32 = 1;
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_CASE_IDS_V1: [&str; 2] =
    ["ridge_progress_056_probe", "ridge_progress_072_probe"];
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_INPUT_IDENTITIES_V1: [&str; 2] =
    ["fnv1a64:1685aab304642342", "fnv1a64:b0dcdcbefac383b6"];
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_INPUT_MANIFEST_IDENTITY_V1: &str = "5a654dd8762a1438";
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_PREDICTION_MANIFEST_IDENTITY_V1: &str =
    "f3d3ff2ee6e83403";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservativeBallisticRidgeF5InputErrorV1 {
    Json,
    UnexpectedField,
    SchemaMismatch,
    CaseCountMismatch,
    CaseOrderMismatch,
    DuplicateCaseId,
    InputInvalid,
    InputIdentityMismatch,
    IdentityMismatch,
}

impl std::fmt::Display for ConservativeBallisticRidgeF5InputErrorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Json => "F5 input manifest JSON is invalid",
            Self::UnexpectedField => "F5 input manifest contains an unexpected field",
            Self::SchemaMismatch => "F5 input manifest schema is invalid",
            Self::CaseCountMismatch => "F5 input manifest must contain exactly two cases",
            Self::CaseOrderMismatch => "F5 input manifest case IDs are not in the frozen order",
            Self::DuplicateCaseId => "F5 input manifest contains a duplicate case ID",
            Self::InputInvalid => "F5 input manifest contains an invalid raw input",
            Self::InputIdentityMismatch => {
                "F5 input manifest does not contain the frozen raw input identity"
            }
            Self::IdentityMismatch => "F5 input manifest identity does not bind its contents",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ConservativeBallisticRidgeF5InputErrorV1 {}

/// A versioned evaluator-owned manifest containing only identity-bound raw
/// input values. The generic planner input type provides its own raw-content
/// validation and identity; this wrapper freezes the F5 order and full set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5InputsV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub cases: Vec<ExperimentalRidgeCaseInputV1>,
    pub identity: String,
}

impl ConservativeBallisticRidgeF5InputsV1 {
    /// Validate raw input contents and the F5-specific order without opening
    /// an analytical result or building a runtime route.
    pub fn validate(&self) -> Result<(), ConservativeBallisticRidgeF5InputErrorV1> {
        if self.schema_id != CONSERVATIVE_BALLISTIC_RIDGE_F5_INPUTS_SCHEMA_ID_V1
            || self.schema_version != CONSERVATIVE_BALLISTIC_RIDGE_F5_INPUTS_SCHEMA_VERSION_V1
        {
            return Err(ConservativeBallisticRidgeF5InputErrorV1::SchemaMismatch);
        }
        if self.cases.len() != CONSERVATIVE_BALLISTIC_RIDGE_F5_CASE_IDS_V1.len() {
            return Err(ConservativeBallisticRidgeF5InputErrorV1::CaseCountMismatch);
        }
        let mut ids = BTreeSet::new();
        for input in &self.cases {
            if !ids.insert(input.probe.id.clone()) {
                return Err(ConservativeBallisticRidgeF5InputErrorV1::DuplicateCaseId);
            }
        }
        for ((input, expected_id), expected_identity) in self
            .cases
            .iter()
            .zip(CONSERVATIVE_BALLISTIC_RIDGE_F5_CASE_IDS_V1)
            .zip(CONSERVATIVE_BALLISTIC_RIDGE_F5_INPUT_IDENTITIES_V1)
        {
            if input.probe.id != expected_id {
                return Err(ConservativeBallisticRidgeF5InputErrorV1::CaseOrderMismatch);
            }
            input
                .validate()
                .map_err(|_| ConservativeBallisticRidgeF5InputErrorV1::InputInvalid)?;
            if input.identity != expected_identity {
                return Err(ConservativeBallisticRidgeF5InputErrorV1::InputIdentityMismatch);
            }
        }
        if self.identity != CONSERVATIVE_BALLISTIC_RIDGE_F5_INPUT_MANIFEST_IDENTITY_V1
            || self.identity != input_manifest_identity_v1(self)?
        {
            return Err(ConservativeBallisticRidgeF5InputErrorV1::IdentityMismatch);
        }
        Ok(())
    }
}

/// Parse and validate a committed F5 input fixture. The explicit JSON-shape
/// audit closes the generic input type's permissive serde boundary, so a raw
/// fixture cannot smuggle a candidate, route, runtime, controller, simulation,
/// or report field through ignored deserialization keys.
pub fn parse_conservative_ballistic_ridge_f5_inputs_v1(
    raw: &str,
) -> Result<ConservativeBallisticRidgeF5InputsV1, ConservativeBallisticRidgeF5InputErrorV1> {
    let value: Value =
        serde_json::from_str(raw).map_err(|_| ConservativeBallisticRidgeF5InputErrorV1::Json)?;
    validate_input_json_shape_v1(&value)?;
    let inputs: ConservativeBallisticRidgeF5InputsV1 = serde_json::from_value(value)
        .map_err(|_| ConservativeBallisticRidgeF5InputErrorV1::Json)?;
    inputs.validate()?;
    Ok(inputs)
}

/// Load the committed F5 raw inputs. This does not reveal analytical or
/// controller behavior; it only decodes and validates source data.
pub fn load_conservative_ballistic_ridge_f5_inputs_v1()
-> Result<ConservativeBallisticRidgeF5InputsV1, ConservativeBallisticRidgeF5InputErrorV1> {
    parse_conservative_ballistic_ridge_f5_inputs_v1(INPUTS_FIXTURE_V1)
}

fn input_manifest_identity_v1(
    inputs: &ConservativeBallisticRidgeF5InputsV1,
) -> Result<String, ConservativeBallisticRidgeF5InputErrorV1> {
    canonical_digest(&InputManifestIdentityMaterialV1 {
        schema_id: &inputs.schema_id,
        schema_version: inputs.schema_version,
        cases: &inputs.cases,
    })
    .map_err(|_| ConservativeBallisticRidgeF5InputErrorV1::IdentityMismatch)
}

#[derive(Serialize)]
struct InputManifestIdentityMaterialV1<'a> {
    schema_id: &'a str,
    schema_version: u32,
    cases: &'a [ExperimentalRidgeCaseInputV1],
}

fn validate_input_json_shape_v1(
    value: &Value,
) -> Result<(), ConservativeBallisticRidgeF5InputErrorV1> {
    require_object_keys(value, &["schema_id", "schema_version", "cases", "identity"])?;
    let cases = value
        .get("cases")
        .and_then(Value::as_array)
        .ok_or(ConservativeBallisticRidgeF5InputErrorV1::Json)?;
    for case in cases {
        require_object_keys(
            case,
            &[
                "schema_id",
                "schema_version",
                "policy",
                "vehicle",
                "probe",
                "identity",
            ],
        )?;
        let policy = case
            .get("policy")
            .ok_or(ConservativeBallisticRidgeF5InputErrorV1::Json)?;
        require_object_keys(
            policy,
            &[
                "physics_hz",
                "gravity_mps2",
                "duration_multipliers",
                "minimum_clearance_m",
                "maximum_mission_time_s",
                "mission_time_reserve_s",
                "thrust_derate",
                "declared_robustness_margin",
                "handoff_interval_s",
                "bridge_duration_interval_s",
                "terminal_target_downward_speed_fraction",
            ],
        )?;
        let vehicle = case
            .get("vehicle")
            .ok_or(ConservativeBallisticRidgeF5InputErrorV1::Json)?;
        require_object_keys(
            vehicle,
            &[
                "geometry",
                "dry_mass_kg",
                "initial_fuel_kg",
                "max_fuel_kg",
                "max_fuel_burn_kgps",
                "max_thrust_n",
                "min_throttle_frac",
                "max_rotation_rate_radps",
                "safe_touchdown_normal_speed_mps",
                "safe_touchdown_tangential_speed_mps",
                "safe_touchdown_attitude_error_rad",
                "safe_touchdown_angular_rate_radps",
            ],
        )?;
        require_object_keys(
            vehicle
                .get("geometry")
                .ok_or(ConservativeBallisticRidgeF5InputErrorV1::Json)?,
            &[
                "hull_width_m",
                "hull_height_m",
                "touchdown_half_span_m",
                "touchdown_base_offset_m",
            ],
        )?;
        let probe = case
            .get("probe")
            .ok_or(ConservativeBallisticRidgeF5InputErrorV1::Json)?;
        require_object_keys(
            probe,
            &[
                "id",
                "source",
                "target",
                "terrain_points_m",
                "initial_position_m",
                "initial_velocity_mps",
            ],
        )?;
        for pad_key in ["source", "target"] {
            require_object_keys(
                probe
                    .get(pad_key)
                    .ok_or(ConservativeBallisticRidgeF5InputErrorV1::Json)?,
                &["center_x_m", "surface_y_m", "width_m"],
            )?;
        }
        let terrain = probe
            .get("terrain_points_m")
            .and_then(Value::as_array)
            .ok_or(ConservativeBallisticRidgeF5InputErrorV1::Json)?;
        for point in terrain {
            require_object_keys(point, &["x", "y"])?;
        }
        for state_key in ["initial_position_m", "initial_velocity_mps"] {
            require_object_keys(
                probe
                    .get(state_key)
                    .ok_or(ConservativeBallisticRidgeF5InputErrorV1::Json)?,
                &["x", "y"],
            )?;
        }
    }
    Ok(())
}

fn require_object_keys(
    value: &Value,
    expected: &[&str],
) -> Result<(), ConservativeBallisticRidgeF5InputErrorV1> {
    let object = value
        .as_object()
        .ok_or(ConservativeBallisticRidgeF5InputErrorV1::Json)?;
    if object.len() != expected.len()
        || !expected.iter().all(|key| object.contains_key(*key))
        || object.keys().any(|key| !expected.contains(&key.as_str()))
    {
        return Err(ConservativeBallisticRidgeF5InputErrorV1::UnexpectedField);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservativeBallisticRidgeF5PredictionErrorV1 {
    Json,
    SchemaMismatch,
    InputManifestInvalid,
    InputManifestIdentityMismatch,
    CaseCountMismatch,
    CaseOrderMismatch,
    DuplicateCaseId,
    CaseInputIdentityMismatch,
    PredictionStructureMismatch,
    IdentityMismatch,
}

impl std::fmt::Display for ConservativeBallisticRidgeF5PredictionErrorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Json => "F5 prediction manifest JSON is invalid",
            Self::SchemaMismatch => "F5 prediction manifest schema is invalid",
            Self::InputManifestInvalid => "bound F5 input manifest is invalid",
            Self::InputManifestIdentityMismatch => {
                "F5 predictions do not bind the supplied input manifest"
            }
            Self::CaseCountMismatch => "F5 prediction manifest must contain exactly two cases",
            Self::CaseOrderMismatch => "F5 prediction case IDs are not in the frozen input order",
            Self::DuplicateCaseId => "F5 prediction manifest contains a duplicate case ID",
            Self::CaseInputIdentityMismatch => {
                "F5 prediction does not bind the corresponding input identity"
            }
            Self::PredictionStructureMismatch => {
                "F5 prediction metadata does not match the frozen contract"
            }
            Self::IdentityMismatch => "F5 prediction manifest identity does not bind its contents",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ConservativeBallisticRidgeF5PredictionErrorV1 {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedF5AnalyticalRouteOutcomeV1 {
    Direct,
    OneWaypoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5FlatControlSelectionRuleV1 {
    ShortestRobustCertified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5DerivedBlockerExpectationV1 {
    Valid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5NominalDirectRejectionV1 {
    TerrainClearance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5WaypointSearchExpectationV1 {
    ExactlyOneCertifiedForwardProgressing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5RuntimeRouteStructureV2 {
    StructurallyValidOneWaypoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5SemanticHandoffContractExpectationV2 {
    Passing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5DeterministicRepeatRequirementV1 {
    ByteIdenticalRepeatRequired,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5FlatControlAnalyticalPredictionV1 {
    pub expected_outcome: ExpectedF5AnalyticalRouteOutcomeV1,
    pub selection_rule: F5FlatControlSelectionRuleV1,
    pub duration_multiplier: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5DerivedMesaAnalyticalPredictionV1 {
    pub blocker: F5DerivedBlockerExpectationV1,
    pub nominal_direct_rejection: F5NominalDirectRejectionV1,
    pub expected_outcome: ExpectedF5AnalyticalRouteOutcomeV1,
    pub bounded_waypoint_search: F5WaypointSearchExpectationV1,
    pub runtime_v2: F5RuntimeRouteStructureV2,
    pub selected_semantic_handoff_contract: F5SemanticHandoffContractExpectationV2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5AnalyticalPredictionV1 {
    pub flat_control: F5FlatControlAnalyticalPredictionV1,
    pub derived_mesa: F5DerivedMesaAnalyticalPredictionV1,
    pub deterministic_repeat: F5DeterministicRepeatRequirementV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5ControllerLandingExpectationV1 {
    TargetLanding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5ControllerTerrainContactExpectationV1 {
    NonTargetTerrainContact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5TerrainContactLocationExpectationV1 {
    DerivedMesa,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5TerrainContactOrderingExpectationV1 {
    BeforeTargetTouchdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5ContactPrecisionExpectationV1 {
    NoExactTimeOrPoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5WaypointCaptureExpectationV1 {
    ExactlyOneContractPassCapture,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5FlatDirectControllerExpectationV1 {
    pub outcome: F5ControllerLandingExpectationV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5MesaDirectControllerExpectationV1 {
    pub outcome: F5ControllerTerrainContactExpectationV1,
    pub location: F5TerrainContactLocationExpectationV1,
    pub ordering: F5TerrainContactOrderingExpectationV1,
    pub precision: F5ContactPrecisionExpectationV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5MesaWaypointControllerExpectationV1 {
    pub capture: F5WaypointCaptureExpectationV1,
    pub terminal_outcome: F5ControllerLandingExpectationV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5ControllerExpectationsV1 {
    pub flat_direct: F5FlatDirectControllerExpectationV1,
    pub mesa_direct: F5MesaDirectControllerExpectationV1,
    pub mesa_waypoint: F5MesaWaypointControllerExpectationV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5PredictionV1 {
    pub case_id: String,
    pub input_identity: String,
    pub analytical: F5AnalyticalPredictionV1,
    pub controller: F5ControllerExpectationsV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5PredictionsV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub input_manifest_identity: String,
    pub cases: Vec<ConservativeBallisticRidgeF5PredictionV1>,
    pub identity: String,
}

impl ConservativeBallisticRidgeF5PredictionV1 {
    fn for_input(input: &ExperimentalRidgeCaseInputV1) -> Self {
        Self {
            case_id: input.probe.id.clone(),
            input_identity: input.identity.clone(),
            analytical: F5AnalyticalPredictionV1 {
                flat_control: F5FlatControlAnalyticalPredictionV1 {
                    expected_outcome: ExpectedF5AnalyticalRouteOutcomeV1::Direct,
                    selection_rule: F5FlatControlSelectionRuleV1::ShortestRobustCertified,
                    duration_multiplier: 1.0,
                },
                derived_mesa: F5DerivedMesaAnalyticalPredictionV1 {
                    blocker: F5DerivedBlockerExpectationV1::Valid,
                    nominal_direct_rejection: F5NominalDirectRejectionV1::TerrainClearance,
                    expected_outcome: ExpectedF5AnalyticalRouteOutcomeV1::OneWaypoint,
                    bounded_waypoint_search:
                        F5WaypointSearchExpectationV1::ExactlyOneCertifiedForwardProgressing,
                    runtime_v2: F5RuntimeRouteStructureV2::StructurallyValidOneWaypoint,
                    selected_semantic_handoff_contract:
                        F5SemanticHandoffContractExpectationV2::Passing,
                },
                deterministic_repeat:
                    F5DeterministicRepeatRequirementV1::ByteIdenticalRepeatRequired,
            },
            controller: F5ControllerExpectationsV1 {
                flat_direct: F5FlatDirectControllerExpectationV1 {
                    outcome: F5ControllerLandingExpectationV1::TargetLanding,
                },
                mesa_direct: F5MesaDirectControllerExpectationV1 {
                    outcome: F5ControllerTerrainContactExpectationV1::NonTargetTerrainContact,
                    location: F5TerrainContactLocationExpectationV1::DerivedMesa,
                    ordering: F5TerrainContactOrderingExpectationV1::BeforeTargetTouchdown,
                    precision: F5ContactPrecisionExpectationV1::NoExactTimeOrPoint,
                },
                mesa_waypoint: F5MesaWaypointControllerExpectationV1 {
                    capture: F5WaypointCaptureExpectationV1::ExactlyOneContractPassCapture,
                    terminal_outcome: F5ControllerLandingExpectationV1::TargetLanding,
                },
            },
        }
    }
}

impl ConservativeBallisticRidgeF5PredictionsV1 {
    /// Validate frozen qualitative predictions against their exact raw inputs.
    /// This is an identity and metadata check only; it never reveals a result.
    pub fn validate(
        &self,
        input_manifest: &ConservativeBallisticRidgeF5InputsV1,
    ) -> Result<(), ConservativeBallisticRidgeF5PredictionErrorV1> {
        input_manifest
            .validate()
            .map_err(|_| ConservativeBallisticRidgeF5PredictionErrorV1::InputManifestInvalid)?;
        if self.schema_id != CONSERVATIVE_BALLISTIC_RIDGE_F5_PREDICTIONS_SCHEMA_ID_V1
            || self.schema_version != CONSERVATIVE_BALLISTIC_RIDGE_F5_PREDICTIONS_SCHEMA_VERSION_V1
        {
            return Err(ConservativeBallisticRidgeF5PredictionErrorV1::SchemaMismatch);
        }
        if self.input_manifest_identity != input_manifest.identity {
            return Err(
                ConservativeBallisticRidgeF5PredictionErrorV1::InputManifestIdentityMismatch,
            );
        }
        if self.cases.len() != CONSERVATIVE_BALLISTIC_RIDGE_F5_CASE_IDS_V1.len()
            || self.cases.len() != input_manifest.cases.len()
        {
            return Err(ConservativeBallisticRidgeF5PredictionErrorV1::CaseCountMismatch);
        }
        let mut ids = BTreeSet::new();
        for prediction in &self.cases {
            if !ids.insert(prediction.case_id.clone()) {
                return Err(ConservativeBallisticRidgeF5PredictionErrorV1::DuplicateCaseId);
            }
        }
        let mut input_identities = BTreeSet::new();
        for ((prediction, input), expected_id) in self
            .cases
            .iter()
            .zip(&input_manifest.cases)
            .zip(CONSERVATIVE_BALLISTIC_RIDGE_F5_CASE_IDS_V1)
        {
            if prediction.case_id != expected_id || prediction.case_id != input.probe.id {
                return Err(ConservativeBallisticRidgeF5PredictionErrorV1::CaseOrderMismatch);
            }
            if prediction.input_identity != input.identity {
                return Err(
                    ConservativeBallisticRidgeF5PredictionErrorV1::CaseInputIdentityMismatch,
                );
            }
            if !input_identities.insert(prediction.input_identity.clone()) {
                return Err(
                    ConservativeBallisticRidgeF5PredictionErrorV1::CaseInputIdentityMismatch,
                );
            }
            if prediction != &ConservativeBallisticRidgeF5PredictionV1::for_input(input) {
                return Err(
                    ConservativeBallisticRidgeF5PredictionErrorV1::PredictionStructureMismatch,
                );
            }
        }
        if self.identity != CONSERVATIVE_BALLISTIC_RIDGE_F5_PREDICTION_MANIFEST_IDENTITY_V1
            || self.identity != prediction_manifest_identity_v1(self)?
        {
            return Err(ConservativeBallisticRidgeF5PredictionErrorV1::IdentityMismatch);
        }
        Ok(())
    }
}

/// Parse the F5 qualitative prediction fixture and bind it to the caller's
/// already sealed raw inputs. This validates only metadata and identity.
pub fn parse_conservative_ballistic_ridge_f5_predictions_v1(
    raw: &str,
    input_manifest: &ConservativeBallisticRidgeF5InputsV1,
) -> Result<ConservativeBallisticRidgeF5PredictionsV1, ConservativeBallisticRidgeF5PredictionErrorV1>
{
    let predictions: ConservativeBallisticRidgeF5PredictionsV1 = serde_json::from_str(raw)
        .map_err(|_| ConservativeBallisticRidgeF5PredictionErrorV1::Json)?;
    predictions.validate(input_manifest)?;
    Ok(predictions)
}

/// Load the committed F5 qualitative predictions and bind them to committed
/// F5 raw inputs. This function has no analytical or controller execution.
pub fn load_conservative_ballistic_ridge_f5_predictions_v1()
-> Result<ConservativeBallisticRidgeF5PredictionsV1, ConservativeBallisticRidgeF5PredictionErrorV1>
{
    let inputs = load_conservative_ballistic_ridge_f5_inputs_v1()
        .map_err(|_| ConservativeBallisticRidgeF5PredictionErrorV1::InputManifestInvalid)?;
    parse_conservative_ballistic_ridge_f5_predictions_v1(PREDICTIONS_FIXTURE_V1, &inputs)
}

fn prediction_manifest_identity_v1(
    predictions: &ConservativeBallisticRidgeF5PredictionsV1,
) -> Result<String, ConservativeBallisticRidgeF5PredictionErrorV1> {
    canonical_digest(&PredictionManifestIdentityMaterialV1 {
        schema_id: &predictions.schema_id,
        schema_version: predictions.schema_version,
        input_manifest_identity: &predictions.input_manifest_identity,
        cases: &predictions.cases,
    })
    .map_err(|_| ConservativeBallisticRidgeF5PredictionErrorV1::IdentityMismatch)
}

#[derive(Serialize)]
struct PredictionManifestIdentityMaterialV1<'a> {
    schema_id: &'a str,
    schema_version: u32,
    input_manifest_identity: &'a str,
    cases: &'a [ConservativeBallisticRidgeF5PredictionV1],
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::Vec2;
    use pd_plan::conservative_ballistic_bridge::load_experimental_ridge_case_manifest_v1;
    use serde_json::{Value, json};

    #[test]
    fn f5_inputs_are_exact_fresh_cases_over_the_historical_raw_base() {
        let inputs = load_conservative_ballistic_ridge_f5_inputs_v1().unwrap();
        let historical = load_experimental_ridge_case_manifest_v1();

        assert_eq!(
            inputs
                .cases
                .iter()
                .map(|case| case.probe.id.as_str())
                .collect::<Vec<_>>(),
            CONSERVATIVE_BALLISTIC_RIDGE_F5_CASE_IDS_V1
        );
        assert_eq!(
            inputs
                .cases
                .iter()
                .map(|case| case.identity.as_str())
                .collect::<Vec<_>>(),
            CONSERVATIVE_BALLISTIC_RIDGE_F5_INPUT_IDENTITIES_V1
        );
        assert_eq!(
            inputs.identity,
            CONSERVATIVE_BALLISTIC_RIDGE_F5_INPUT_MANIFEST_IDENTITY_V1
        );

        let expected_terrain = [
            [
                Vec2::new(-40.0, 0.0),
                Vec2::new(2122.92, 0.0),
                Vec2::new(2172.92, 1200.0),
                Vec2::new(2322.92, 1200.0),
                Vec2::new(2422.92, 0.0),
                Vec2::new(4040.0, 0.0),
            ],
            [
                Vec2::new(-40.0, 0.0),
                Vec2::new(2760.04, 0.0),
                Vec2::new(2810.04, 1200.0),
                Vec2::new(2960.04, 1200.0),
                Vec2::new(3060.04, 0.0),
                Vec2::new(4040.0, 0.0),
            ],
        ];
        for (input, expected) in inputs.cases.iter().zip(expected_terrain) {
            assert_eq!(input.probe.terrain_points_m, expected);
            assert_eq!(input.policy, historical.cases[0].policy);
            assert_eq!(input.vehicle, historical.cases[0].vehicle);
            assert_eq!(input.probe.source, historical.cases[0].probe.source);
            assert_eq!(input.probe.target, historical.cases[0].probe.target);
            assert_eq!(
                input.probe.initial_position_m,
                historical.cases[0].probe.initial_position_m
            );
            assert_eq!(
                input.probe.initial_velocity_mps,
                historical.cases[0].probe.initial_velocity_mps
            );
        }
        assert_eq!(inputs.cases[0].policy, inputs.cases[1].policy);
        assert_eq!(inputs.cases[0].vehicle, inputs.cases[1].vehicle);

        let source_x = inputs.cases[0].probe.source.center_x_m;
        let target_x = inputs.cases[0].probe.target.center_x_m;
        let centers = [
            source_x + 0.56 * (target_x - source_x),
            source_x + 0.72 * (target_x - source_x),
        ];
        assert_eq!(centers, [2247.92, 2885.04]);
        assert_eq!(
            inputs.cases[0].probe.terrain_points_m[1].x,
            centers[0] - 125.0
        );
        assert_eq!(
            inputs.cases[1].probe.terrain_points_m[1].x,
            centers[1] - 125.0
        );
        let progress = centers.map(|center| (center - source_x) / (target_x - source_x));
        assert_eq!(progress, [0.56, 0.72]);
        let historical_progress = [0.50, 0.68];
        assert!(
            historical_progress[0] < progress[0] && progress[0] < historical_progress[1],
            "056 is the interpolation case"
        );
        assert!(
            historical_progress[1] < progress[1] && progress[1] <= 0.75,
            "072 is mild extrapolation"
        );
    }

    #[test]
    fn f5_prediction_manifest_loads_reloads_and_binds_frozen_inputs() {
        let inputs = load_conservative_ballistic_ridge_f5_inputs_v1().unwrap();
        let predictions = load_conservative_ballistic_ridge_f5_predictions_v1().unwrap();
        predictions.validate(&inputs).unwrap();
        assert_eq!(predictions.input_manifest_identity, inputs.identity);
        assert_eq!(
            predictions
                .cases
                .iter()
                .map(|case| case.case_id.as_str())
                .collect::<Vec<_>>(),
            CONSERVATIVE_BALLISTIC_RIDGE_F5_CASE_IDS_V1
        );
        assert_eq!(
            predictions.identity,
            CONSERVATIVE_BALLISTIC_RIDGE_F5_PREDICTION_MANIFEST_IDENTITY_V1
        );
        let reloaded: ConservativeBallisticRidgeF5PredictionsV1 =
            serde_json::from_str(&serde_json::to_string(&predictions).unwrap()).unwrap();
        assert_eq!(reloaded, predictions);
        reloaded.validate(&inputs).unwrap();
    }

    #[test]
    fn f5_input_manifest_rejects_tamper_reorder_duplicate_and_unknown_result_fields() {
        let inputs = load_conservative_ballistic_ridge_f5_inputs_v1().unwrap();

        let mut identity_tampered = inputs.clone();
        identity_tampered.identity.push_str("-tampered");
        assert_eq!(
            identity_tampered.validate(),
            Err(ConservativeBallisticRidgeF5InputErrorV1::IdentityMismatch)
        );

        let mut child_identity_tampered = inputs.clone();
        child_identity_tampered.cases[0]
            .identity
            .push_str("-tampered");
        assert_eq!(
            child_identity_tampered.validate(),
            Err(ConservativeBallisticRidgeF5InputErrorV1::InputInvalid)
        );

        let mut reidentified_different_raw_input = inputs.clone();
        reidentified_different_raw_input.cases[0]
            .probe
            .terrain_points_m[1]
            .x += 0.5;
        let changed = &mut reidentified_different_raw_input.cases[0];
        changed.identity = ExperimentalRidgeCaseInputV1::new(
            changed.policy.clone(),
            changed.vehicle.clone(),
            changed.probe.clone(),
        )
        .unwrap()
        .identity;
        reidentified_different_raw_input.identity =
            input_manifest_identity_v1(&reidentified_different_raw_input).unwrap();
        assert_eq!(
            reidentified_different_raw_input.validate(),
            Err(ConservativeBallisticRidgeF5InputErrorV1::InputIdentityMismatch)
        );

        let mut reordered = inputs.clone();
        reordered.cases.swap(0, 1);
        assert_eq!(
            reordered.validate(),
            Err(ConservativeBallisticRidgeF5InputErrorV1::CaseOrderMismatch)
        );

        let mut duplicate = inputs.clone();
        duplicate.cases[1] = duplicate.cases[0].clone();
        assert_eq!(
            duplicate.validate(),
            Err(ConservativeBallisticRidgeF5InputErrorV1::DuplicateCaseId)
        );

        let mut with_candidate_field = serde_json::to_value(inputs).unwrap();
        with_candidate_field["cases"][0]["candidate"] = json!({ "not": "allowed" });
        assert_eq!(
            parse_conservative_ballistic_ridge_f5_inputs_v1(
                &serde_json::to_string(&with_candidate_field).unwrap()
            ),
            Err(ConservativeBallisticRidgeF5InputErrorV1::UnexpectedField)
        );
    }

    #[test]
    fn f5_prediction_manifest_rejects_tamper_reorder_duplicate_and_broken_bindings() {
        let inputs = load_conservative_ballistic_ridge_f5_inputs_v1().unwrap();
        let predictions = load_conservative_ballistic_ridge_f5_predictions_v1().unwrap();

        let mut identity_tampered = predictions.clone();
        identity_tampered.identity.push_str("-tampered");
        assert_eq!(
            identity_tampered.validate(&inputs),
            Err(ConservativeBallisticRidgeF5PredictionErrorV1::IdentityMismatch)
        );

        let mut input_binding_tampered = predictions.clone();
        input_binding_tampered
            .input_manifest_identity
            .push_str("-tampered");
        assert_eq!(
            input_binding_tampered.validate(&inputs),
            Err(ConservativeBallisticRidgeF5PredictionErrorV1::InputManifestIdentityMismatch)
        );

        let mut child_binding_tampered = predictions.clone();
        child_binding_tampered.cases[0]
            .input_identity
            .push_str("-tampered");
        assert_eq!(
            child_binding_tampered.validate(&inputs),
            Err(ConservativeBallisticRidgeF5PredictionErrorV1::CaseInputIdentityMismatch)
        );

        let mut reordered = predictions.clone();
        reordered.cases.swap(0, 1);
        assert_eq!(
            reordered.validate(&inputs),
            Err(ConservativeBallisticRidgeF5PredictionErrorV1::CaseOrderMismatch)
        );

        let mut duplicate = predictions.clone();
        duplicate.cases[1] = duplicate.cases[0].clone();
        assert_eq!(
            duplicate.validate(&inputs),
            Err(ConservativeBallisticRidgeF5PredictionErrorV1::DuplicateCaseId)
        );

        let mut expectation_tampered = predictions.clone();
        expectation_tampered.cases[0]
            .analytical
            .flat_control
            .duration_multiplier = 1.25;
        assert_eq!(
            expectation_tampered.validate(&inputs),
            Err(ConservativeBallisticRidgeF5PredictionErrorV1::PredictionStructureMismatch)
        );

        let mut with_result_field = serde_json::to_value(predictions).unwrap();
        with_result_field["cases"][0]["analytical"]["derived_mesa"]["primary_crossing"] =
            json!("not_frozen");
        assert_eq!(
            parse_conservative_ballistic_ridge_f5_predictions_v1(
                &serde_json::to_string(&with_result_field).unwrap(),
                &inputs
            ),
            Err(ConservativeBallisticRidgeF5PredictionErrorV1::Json)
        );
    }

    #[test]
    fn f5_seals_serialize_without_derived_or_observed_result_fields() {
        let inputs = load_conservative_ballistic_ridge_f5_inputs_v1().unwrap();
        let predictions = load_conservative_ballistic_ridge_f5_predictions_v1().unwrap();

        let input_value = serde_json::to_value(inputs).unwrap();
        assert_no_json_keys(
            &input_value,
            &[
                "candidate",
                "candidate_identity",
                "projection",
                "route",
                "runtime_result",
                "controller_result",
                "simulation",
                "report",
                "observed_outcome",
            ],
        );

        let prediction_value = serde_json::to_value(predictions).unwrap();
        assert_no_json_keys(
            &prediction_value,
            &[
                "candidate",
                "candidate_identity",
                "projection",
                "route",
                "runtime_result",
                "controller_result",
                "simulation",
                "report",
                "observed_outcome",
                "longer_global_replan",
                "primary_crossing",
                "intermediate_bridge_exit",
                "contact_time_s",
                "capture_time_s",
                "contact_point",
                "capture_point",
                "fuel_remaining_kg",
                "derived_mesa_identity",
            ],
        );
        let Value::Object(top_level) = prediction_value else {
            panic!("F5 prediction manifest must serialize as an object");
        };
        assert_eq!(
            top_level
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "cases",
                "identity",
                "input_manifest_identity",
                "schema_id",
                "schema_version",
            ])
        );
    }

    #[test]
    fn f5_fixture_loaders_are_deterministic() {
        assert_eq!(
            load_conservative_ballistic_ridge_f5_inputs_v1().unwrap(),
            load_conservative_ballistic_ridge_f5_inputs_v1().unwrap()
        );
        assert_eq!(
            load_conservative_ballistic_ridge_f5_predictions_v1().unwrap(),
            load_conservative_ballistic_ridge_f5_predictions_v1().unwrap()
        );
    }

    fn assert_no_json_keys(value: &Value, forbidden: &[&str]) {
        match value {
            Value::Object(object) => {
                for (key, child) in object {
                    assert!(
                        !forbidden.contains(&key.as_str()),
                        "seal must not serialize forbidden field {key}"
                    );
                    assert_no_json_keys(child, forbidden);
                }
            }
            Value::Array(values) => {
                for child in values {
                    assert_no_json_keys(child, forbidden);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
}
