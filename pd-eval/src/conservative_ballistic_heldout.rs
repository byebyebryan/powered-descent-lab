//! H2 input and prediction contracts for the held-out conservative-ballistic
//! ridge cases.
//!
//! This module owns only frozen prediction metadata.  It intentionally has no
//! analytical evaluator, runtime projection, controller, or outcome fields;
//! those belong to later H2b/H3 stages and must join this manifest by the
//! sealed input identities below.

use std::collections::BTreeSet;

use pd_plan::conservative_ballistic_bridge::{
    EXPERIMENTAL_RIDGE_HELDOUT_CASE_IDS_V1, ExperimentalRidgeCaseInputV1,
    ExperimentalRidgeCaseManifestV1,
};
use serde::{Deserialize, Serialize};

use crate::canonical_digest;

const PREDICTIONS_FIXTURE_V1: &str = include_str!(
    "../../fixtures/manifests/conservative_ballistic_ridge_heldout_predictions_v1.json"
);

pub const CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_PREDICTIONS_SCHEMA_ID_V1: &str =
    "conservative_ballistic_ridge_heldout_predictions_v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_PREDICTIONS_SCHEMA_VERSION_V1: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservativeBallisticRidgeHeldoutPredictionErrorV1 {
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

impl std::fmt::Display for ConservativeBallisticRidgeHeldoutPredictionErrorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Json => "held-out prediction manifest JSON is invalid",
            Self::SchemaMismatch => "held-out prediction manifest schema is invalid",
            Self::InputManifestInvalid => "bound held-out input manifest is invalid",
            Self::InputManifestIdentityMismatch => {
                "held-out predictions do not bind the supplied input manifest"
            }
            Self::CaseCountMismatch => {
                "held-out prediction manifest must contain exactly two cases"
            }
            Self::CaseOrderMismatch => {
                "held-out prediction case IDs are not in the frozen input order"
            }
            Self::DuplicateCaseId => "held-out prediction manifest contains a duplicate ID",
            Self::CaseInputIdentityMismatch => {
                "held-out prediction does not bind the corresponding input identity"
            }
            Self::PredictionStructureMismatch => {
                "held-out prediction metadata does not match the frozen H2 contract"
            }
            Self::IdentityMismatch => {
                "held-out prediction manifest identity does not bind its contents"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ConservativeBallisticRidgeHeldoutPredictionErrorV1 {}

/// The only analytical route outcome expected from the bounded H2 family.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedAnalyticalRouteOutcomeV1 {
    Direct,
    OneWaypoint,
}

/// Frozen selection rule for the flat-twin control.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlatControlSelectionRuleV1 {
    ShortestRobustCertified,
}

/// Frozen reason for rejecting the nominal direct lane over the derived mesa.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NominalDirectRejectionV1 {
    TerrainClearance,
}

/// H2 expects a valid derived blocker, without recording its post-evaluation
/// geometry or any candidate identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivedBlockerExpectationV1 {
    Valid,
}

/// The longer direct candidates are a diagnostic global-replan comparison,
/// not a replacement for the shortest flat-twin selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LongerGlobalReplanExpectationV1 {
    AtLeastOneCertifiedDiagnostic,
}

/// The selected waypoint witness must be both analytically certified and
/// forward-progressing, with exactly one selected result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectedWaypointWitnessExpectationV1 {
    ExactlyOneCertifiedForwardProgressing,
}

/// Validation requirements are metadata only.  They do not contain any
/// observed candidate, canary, runtime, or controller result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionValidationRequirementV1 {
    Required,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeterministicRepeatRequirementV1 {
    ByteIdenticalRepeatRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedResultPolicyV1 {
    StopExperiment,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatControlAnalyticalPredictionV1 {
    pub expected_outcome: ExpectedAnalyticalRouteOutcomeV1,
    pub selection_rule: FlatControlSelectionRuleV1,
    pub duration_multiplier: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedMesaAnalyticalPredictionV1 {
    pub blocker: DerivedBlockerExpectationV1,
    pub nominal_direct_rejection: NominalDirectRejectionV1,
    pub longer_global_replan: LongerGlobalReplanExpectationV1,
    pub expected_outcome: ExpectedAnalyticalRouteOutcomeV1,
    pub selected_waypoint_witness: SelectedWaypointWitnessExpectationV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyticalValidationRequirementsV1 {
    pub candidate_projection: ProjectionValidationRequirementV1,
    pub runtime_projection: ProjectionValidationRequirementV1,
    pub deterministic_repeat: DeterministicRepeatRequirementV1,
    pub unsupported_result: UnsupportedResultPolicyV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyticalPredictionV1 {
    pub flat_control: FlatControlAnalyticalPredictionV1,
    pub derived_mesa: DerivedMesaAnalyticalPredictionV1,
    pub validation: AnalyticalValidationRequirementsV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerLandingExpectationV1 {
    TargetLanding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerTerrainContactExpectationV1 {
    NonTargetTerrainContact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainContactLocationExpectationV1 {
    DerivedMesa,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainContactOrderingExpectationV1 {
    BeforeTargetTouchdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactPrecisionExpectationV1 {
    NoExactTimeOrPoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointHandoffExpectationV1 {
    ContractPass,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatDirectControllerExpectationV1 {
    pub outcome: ControllerLandingExpectationV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MesaDirectControllerExpectationV1 {
    pub outcome: ControllerTerrainContactExpectationV1,
    pub location: TerrainContactLocationExpectationV1,
    pub ordering: TerrainContactOrderingExpectationV1,
    pub precision: ContactPrecisionExpectationV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MesaWaypointControllerExpectationV1 {
    pub handoff: WaypointHandoffExpectationV1,
    pub terminal_outcome: ControllerLandingExpectationV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerExpectationsV1 {
    pub flat_direct: FlatDirectControllerExpectationV1,
    pub mesa_direct: MesaDirectControllerExpectationV1,
    pub mesa_waypoint: MesaWaypointControllerExpectationV1,
}

/// One prediction record.  It deliberately has no record identity of its
/// own: the manifest identity binds the ordered records, while the input
/// identity binds the complete raw case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeHeldoutPredictionV1 {
    pub case_id: String,
    pub input_identity: String,
    pub analytical: AnalyticalPredictionV1,
    pub controller: ControllerExpectationsV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeHeldoutPredictionsV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub input_manifest_identity: String,
    pub cases: Vec<ConservativeBallisticRidgeHeldoutPredictionV1>,
    pub identity: String,
}

impl ConservativeBallisticRidgeHeldoutPredictionV1 {
    fn for_input(input: &ExperimentalRidgeCaseInputV1) -> Self {
        Self {
            case_id: input.probe.id.clone(),
            input_identity: input.identity.clone(),
            analytical: AnalyticalPredictionV1 {
                flat_control: FlatControlAnalyticalPredictionV1 {
                    expected_outcome: ExpectedAnalyticalRouteOutcomeV1::Direct,
                    selection_rule: FlatControlSelectionRuleV1::ShortestRobustCertified,
                    duration_multiplier: 1.0,
                },
                derived_mesa: DerivedMesaAnalyticalPredictionV1 {
                    blocker: DerivedBlockerExpectationV1::Valid,
                    nominal_direct_rejection: NominalDirectRejectionV1::TerrainClearance,
                    longer_global_replan:
                        LongerGlobalReplanExpectationV1::AtLeastOneCertifiedDiagnostic,
                    expected_outcome: ExpectedAnalyticalRouteOutcomeV1::OneWaypoint,
                    selected_waypoint_witness:
                        SelectedWaypointWitnessExpectationV1::ExactlyOneCertifiedForwardProgressing,
                },
                validation: AnalyticalValidationRequirementsV1 {
                    candidate_projection: ProjectionValidationRequirementV1::Required,
                    runtime_projection: ProjectionValidationRequirementV1::Required,
                    deterministic_repeat:
                        DeterministicRepeatRequirementV1::ByteIdenticalRepeatRequired,
                    unsupported_result: UnsupportedResultPolicyV1::StopExperiment,
                },
            },
            controller: ControllerExpectationsV1 {
                flat_direct: FlatDirectControllerExpectationV1 {
                    outcome: ControllerLandingExpectationV1::TargetLanding,
                },
                mesa_direct: MesaDirectControllerExpectationV1 {
                    outcome: ControllerTerrainContactExpectationV1::NonTargetTerrainContact,
                    location: TerrainContactLocationExpectationV1::DerivedMesa,
                    ordering: TerrainContactOrderingExpectationV1::BeforeTargetTouchdown,
                    precision: ContactPrecisionExpectationV1::NoExactTimeOrPoint,
                },
                mesa_waypoint: MesaWaypointControllerExpectationV1 {
                    handoff: WaypointHandoffExpectationV1::ContractPass,
                    terminal_outcome: ControllerLandingExpectationV1::TargetLanding,
                },
            },
        }
    }
}

impl ConservativeBallisticRidgeHeldoutPredictionsV1 {
    /// Build the complete frozen prediction metadata for an already validated
    /// input manifest.  This performs no analytical evaluation.
    pub fn from_input_manifest(
        input_manifest: &ExperimentalRidgeCaseManifestV1,
    ) -> Result<Self, ConservativeBallisticRidgeHeldoutPredictionErrorV1> {
        input_manifest.validate().map_err(|_| {
            ConservativeBallisticRidgeHeldoutPredictionErrorV1::InputManifestInvalid
        })?;
        let mut predictions = Self {
            schema_id: CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_PREDICTIONS_SCHEMA_ID_V1.to_owned(),
            schema_version: CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_PREDICTIONS_SCHEMA_VERSION_V1,
            input_manifest_identity: input_manifest.identity.clone(),
            cases: input_manifest
                .cases
                .iter()
                .map(ConservativeBallisticRidgeHeldoutPredictionV1::for_input)
                .collect(),
            identity: String::new(),
        };
        predictions.identity = prediction_manifest_identity_v1(&predictions)?;
        predictions.validate(input_manifest)?;
        Ok(predictions)
    }

    /// Validate the sealed prediction metadata against its exact input
    /// manifest.  This is a pre-evaluation check and never opens an outcome.
    pub fn validate(
        &self,
        input_manifest: &ExperimentalRidgeCaseManifestV1,
    ) -> Result<(), ConservativeBallisticRidgeHeldoutPredictionErrorV1> {
        input_manifest.validate().map_err(|_| {
            ConservativeBallisticRidgeHeldoutPredictionErrorV1::InputManifestInvalid
        })?;
        if self.schema_id != CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_PREDICTIONS_SCHEMA_ID_V1
            || self.schema_version
                != CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_PREDICTIONS_SCHEMA_VERSION_V1
        {
            return Err(ConservativeBallisticRidgeHeldoutPredictionErrorV1::SchemaMismatch);
        }
        if self.input_manifest_identity != input_manifest.identity {
            return Err(
                ConservativeBallisticRidgeHeldoutPredictionErrorV1::InputManifestIdentityMismatch,
            );
        }
        if self.cases.len() != EXPERIMENTAL_RIDGE_HELDOUT_CASE_IDS_V1.len()
            || self.cases.len() != input_manifest.cases.len()
        {
            return Err(ConservativeBallisticRidgeHeldoutPredictionErrorV1::CaseCountMismatch);
        }
        let mut ids = BTreeSet::new();
        let mut input_identities = BTreeSet::new();
        for ((prediction, input), expected_id) in self
            .cases
            .iter()
            .zip(&input_manifest.cases)
            .zip(EXPERIMENTAL_RIDGE_HELDOUT_CASE_IDS_V1)
        {
            if prediction.case_id != expected_id || prediction.case_id != input.probe.id {
                return Err(ConservativeBallisticRidgeHeldoutPredictionErrorV1::CaseOrderMismatch);
            }
            if !ids.insert(prediction.case_id.clone()) {
                return Err(ConservativeBallisticRidgeHeldoutPredictionErrorV1::DuplicateCaseId);
            }
            if prediction.input_identity != input.identity {
                return Err(
                    ConservativeBallisticRidgeHeldoutPredictionErrorV1::CaseInputIdentityMismatch,
                );
            }
            if !input_identities.insert(prediction.input_identity.clone()) {
                return Err(
                    ConservativeBallisticRidgeHeldoutPredictionErrorV1::CaseInputIdentityMismatch,
                );
            }
            if prediction != &ConservativeBallisticRidgeHeldoutPredictionV1::for_input(input) {
                return Err(
                    ConservativeBallisticRidgeHeldoutPredictionErrorV1::PredictionStructureMismatch,
                );
            }
        }
        if self.identity != prediction_manifest_identity_v1(self)? {
            return Err(ConservativeBallisticRidgeHeldoutPredictionErrorV1::IdentityMismatch);
        }
        Ok(())
    }
}

/// Parse and validate held-out predictions against a caller-selected input
/// manifest.  Only the prediction metadata is decoded.
pub fn parse_conservative_ballistic_ridge_heldout_predictions_v1(
    raw: &str,
    input_manifest: &ExperimentalRidgeCaseManifestV1,
) -> Result<
    ConservativeBallisticRidgeHeldoutPredictionsV1,
    ConservativeBallisticRidgeHeldoutPredictionErrorV1,
> {
    let predictions: ConservativeBallisticRidgeHeldoutPredictionsV1 = serde_json::from_str(raw)
        .map_err(|_| ConservativeBallisticRidgeHeldoutPredictionErrorV1::Json)?;
    predictions.validate(input_manifest)?;
    Ok(predictions)
}

/// Load the committed H2 prediction metadata and bind it to the committed raw
/// input manifest.  This function performs no analytical or controller work.
pub fn load_conservative_ballistic_ridge_heldout_predictions_v1() -> Result<
    ConservativeBallisticRidgeHeldoutPredictionsV1,
    ConservativeBallisticRidgeHeldoutPredictionErrorV1,
> {
    let input_manifest =
        pd_plan::conservative_ballistic_bridge::load_experimental_ridge_case_manifest_v1();
    parse_conservative_ballistic_ridge_heldout_predictions_v1(
        PREDICTIONS_FIXTURE_V1,
        &input_manifest,
    )
}

fn prediction_manifest_identity_v1(
    predictions: &ConservativeBallisticRidgeHeldoutPredictionsV1,
) -> Result<String, ConservativeBallisticRidgeHeldoutPredictionErrorV1> {
    canonical_digest(&PredictionManifestIdentityMaterialV1 {
        schema_id: &predictions.schema_id,
        schema_version: predictions.schema_version,
        input_manifest_identity: &predictions.input_manifest_identity,
        cases: &predictions.cases,
    })
    .map_err(|_| ConservativeBallisticRidgeHeldoutPredictionErrorV1::IdentityMismatch)
}

#[derive(Serialize)]
struct PredictionManifestIdentityMaterialV1<'a> {
    schema_id: &'a str,
    schema_version: u32,
    input_manifest_identity: &'a str,
    cases: &'a [ConservativeBallisticRidgeHeldoutPredictionV1],
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn inputs() -> ExperimentalRidgeCaseManifestV1 {
        pd_plan::conservative_ballistic_bridge::load_experimental_ridge_case_manifest_v1()
    }

    fn predictions() -> ConservativeBallisticRidgeHeldoutPredictionsV1 {
        load_conservative_ballistic_ridge_heldout_predictions_v1().unwrap()
    }

    #[test]
    fn heldout_prediction_manifest_loads_reloads_and_binds_frozen_inputs() {
        let input_manifest = inputs();
        let prediction_manifest = predictions();
        prediction_manifest.validate(&input_manifest).unwrap();
        assert_eq!(
            prediction_manifest.input_manifest_identity,
            input_manifest.identity
        );
        assert_eq!(
            prediction_manifest
                .cases
                .iter()
                .map(|case| case.case_id.as_str())
                .collect::<Vec<_>>(),
            EXPERIMENTAL_RIDGE_HELDOUT_CASE_IDS_V1
        );
        assert_eq!(prediction_manifest.cases.len(), 2);
        assert_eq!(
            prediction_manifest.cases[0].input_identity,
            "fnv1a64:1906ea1c9d5b51eb"
        );
        assert_eq!(
            prediction_manifest.cases[1].input_identity,
            "fnv1a64:a63ff1716b1cf24f"
        );
        assert_eq!(prediction_manifest.identity, "fd55fc7f3f51c65e");

        let reloaded: ConservativeBallisticRidgeHeldoutPredictionsV1 =
            serde_json::from_str(&serde_json::to_string(&prediction_manifest).unwrap()).unwrap();
        assert_eq!(reloaded, prediction_manifest);
        reloaded.validate(&input_manifest).unwrap();
    }

    #[test]
    fn heldout_prediction_manifest_rejects_tamper_reorder_extra_and_missing() {
        let input_manifest = inputs();
        let manifest = predictions();

        let mut identity_tampered = manifest.clone();
        identity_tampered.identity.push_str("-tampered");
        assert_eq!(
            identity_tampered.validate(&input_manifest),
            Err(ConservativeBallisticRidgeHeldoutPredictionErrorV1::IdentityMismatch)
        );

        let mut input_bind_tampered = manifest.clone();
        input_bind_tampered
            .input_manifest_identity
            .push_str("-tampered");
        assert_eq!(
            input_bind_tampered.validate(&input_manifest),
            Err(ConservativeBallisticRidgeHeldoutPredictionErrorV1::InputManifestIdentityMismatch)
        );

        let mut reordered = manifest.clone();
        reordered.cases.swap(0, 1);
        assert_eq!(
            reordered.validate(&input_manifest),
            Err(ConservativeBallisticRidgeHeldoutPredictionErrorV1::CaseOrderMismatch)
        );

        let mut missing = manifest.clone();
        missing.cases.pop();
        assert_eq!(
            missing.validate(&input_manifest),
            Err(ConservativeBallisticRidgeHeldoutPredictionErrorV1::CaseCountMismatch)
        );

        let mut extra = manifest.clone();
        extra.cases.push(extra.cases[0].clone());
        assert_eq!(
            extra.validate(&input_manifest),
            Err(ConservativeBallisticRidgeHeldoutPredictionErrorV1::CaseCountMismatch)
        );

        let mut expectation_tampered = manifest;
        expectation_tampered.cases[0]
            .analytical
            .flat_control
            .duration_multiplier = 1.25;
        assert_eq!(
            expectation_tampered.validate(&input_manifest),
            Err(ConservativeBallisticRidgeHeldoutPredictionErrorV1::PredictionStructureMismatch)
        );
    }

    #[test]
    fn prediction_records_have_no_derived_or_observed_artifact_fields() {
        let value = serde_json::to_value(predictions()).unwrap();
        let Value::Object(top_level) = value else {
            panic!("prediction manifest must serialize as an object");
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
        let serialized = serde_json::to_string(&top_level).unwrap();
        for forbidden in [
            "candidate_identity",
            "analytical_canary_identity",
            "runtime_identity",
            "controller_result",
            "observed_candidate",
            "observed_outcome",
            "selected_candidate",
        ] {
            assert!(
                !serialized.contains(&format!("\"{forbidden}\"")),
                "prediction metadata must not contain {forbidden}"
            );
        }
    }
}
