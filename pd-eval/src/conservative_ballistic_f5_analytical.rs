//! F5 analytical/runtime-V2 reveal boundary.
//!
//! The public runner is deliberately the first consumer that evaluates the
//! sealed F5 raw inputs.  Tests below exercise the generic machinery only
//! with already-exposed historical inputs.  This module never imports a
//! controller, scenario, or simulator API.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::Vec2;
use pd_plan::conservative_ballistic_bridge::{
    CertificationV2, DirectBridgeReasonV2, ExperimentalRidgeCandidateErrorV2,
    ExperimentalRidgeCandidateOutcomeV2, ExperimentalRidgeCaseInputV1,
    ExperimentalRidgeCaseProjectionV1, ExperimentalRidgeCaseRuntimeOutcomeV2,
    ExperimentalRidgeCaseRuntimeProjectionErrorV2, ExperimentalRidgeCaseRuntimeProjectionV2,
    ExperimentalRidgeRuntimeHandoffSelectionKindV2, ExperimentalRidgeUnsupportedReasonV2,
    evaluate_experimental_ridge_case_projection_v1, project_experimental_ridge_case_runtime_v2,
    validate_experimental_ridge_case_projection_v1, validate_experimental_ridge_case_runtime_v2,
};
use pd_report::site::ReportSite;
use serde::{Deserialize, Serialize};

use crate::{
    ConservativeBallisticRidgeF5InputsV1, ConservativeBallisticRidgeF5PredictionV1,
    ConservativeBallisticRidgeF5PredictionsV1, ExpectedF5AnalyticalRouteOutcomeV1,
    F5DerivedBlockerExpectationV1, F5DeterministicRepeatRequirementV1,
    F5FlatControlSelectionRuleV1, F5NominalDirectRejectionV1, F5RuntimeRouteStructureV2,
    F5SemanticHandoffContractExpectationV2, F5WaypointSearchExpectationV1, canonical_digest,
    load_conservative_ballistic_ridge_f5_inputs_v1,
    load_conservative_ballistic_ridge_f5_predictions_v1,
};

pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_SCHEMA_ID_V1: &str =
    "conservative_ballistic_ridge_f5_analytical_v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_SCHEMA_ID_V1: &str =
    "conservative_ballistic_ridge_f5_analytical_result_v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_SETUP_ID_V1: &str =
    "conservative-ballistic-ridge-f5-analytical-v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_FIXTURE_V1: &str =
    "fixtures/manifests/conservative_ballistic_ridge_f5_analytical_result_v1.json";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5AnalyticalArtifactStatusV1 {
    AllEligibleForController,
    OneOrMoreCasesStopped,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5AnalyticalCaseStatusV1 {
    EligibleForController,
    StoppedCandidateUnsupported,
    StoppedCandidateError,
    StoppedCandidatePredictionMismatch,
    StoppedRuntimeUnsupported,
    StoppedRuntimeProjectionError,
    StoppedRuntimePredictionMismatch,
    StoppedInvalidEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5AnalyticalStopStageV1 {
    CandidateProjection,
    CandidatePrediction,
    RuntimeProjection,
    RuntimePrediction,
    EvidenceValidation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5AnalyticalStopClassV1 {
    Unsupported,
    ProjectionError,
    PredictionMismatch,
    InvalidEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5AnalyticalLaneV1 {
    FlatControl,
    DerivedMesa,
}

/// Machine-readable stop provenance retained by the compact result.  The
/// display prose is deliberately secondary: later F5d gating must be able to
/// distinguish a valid bounded `Unsupported` outcome from invalid evidence,
/// an evaluator error, or a failed frozen check.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum F5AnalyticalStopDetailV1 {
    Unsupported {
        lane: F5AnalyticalLaneV1,
        reason: ExperimentalRidgeUnsupportedReasonV2,
        rejection_reasons: Vec<DirectBridgeReasonV2>,
    },
    CandidateError {
        error: ExperimentalRidgeCandidateErrorV2,
    },
    RuntimeProjectionError {
        error: ExperimentalRidgeCaseRuntimeProjectionErrorV2,
    },
    FailedChecks {
        failed_checks: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5AnalyticalStopV1 {
    pub stage: F5AnalyticalStopStageV1,
    pub classification: F5AnalyticalStopClassV1,
    pub reason: String,
    pub detail: F5AnalyticalStopDetailV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5CandidateExpectationComparisonV1 {
    pub flat_control_direct: bool,
    pub flat_shortest_robust_certified: bool,
    pub flat_duration_multiplier_matches: bool,
    pub derived_blocker_valid: bool,
    pub derived_nominal_terrain_clearance_rejected: bool,
    pub derived_mesa_one_waypoint: bool,
    pub exactly_one_certified_forward_progressing_waypoint: bool,
    pub candidate_projection_validated: bool,
}

impl F5CandidateExpectationComparisonV1 {
    fn passed(&self) -> bool {
        self.flat_control_direct
            && self.flat_shortest_robust_certified
            && self.flat_duration_multiplier_matches
            && self.derived_blocker_valid
            && self.derived_nominal_terrain_clearance_rejected
            && self.derived_mesa_one_waypoint
            && self.exactly_one_certified_forward_progressing_waypoint
            && self.candidate_projection_validated
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5ObservedHandoffV2 {
    pub selection_kind: ExperimentalRidgeRuntimeHandoffSelectionKindV2,
    pub selected_attempt_index: usize,
    pub handoff_selection_identity: String,
    pub selected_attempt_identity: String,
    pub route_identity: String,
    pub waypoint_position_m: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5RuntimeExpectationComparisonV1 {
    pub flat_direct_structurally_valid: bool,
    pub derived_structurally_valid_one_waypoint: bool,
    pub selected_semantic_handoff_contract_passes: bool,
    pub selected_identity_chain_valid: bool,
    pub runtime_projection_validated: bool,
    pub observed_handoff: Option<F5ObservedHandoffV2>,
}

impl F5RuntimeExpectationComparisonV1 {
    fn passed(&self) -> bool {
        self.flat_direct_structurally_valid
            && self.derived_structurally_valid_one_waypoint
            && self.selected_semantic_handoff_contract_passes
            && self.selected_identity_chain_valid
            && self.runtime_projection_validated
            && self.observed_handoff.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5AnalyticalVisualV1 {
    pub flat_twin_terrain_points_m: Vec<Vec2>,
    pub derived_mesa_terrain_points_m: Vec<Vec2>,
    pub nominal_direct_arc_points_m: Vec<Vec2>,
    pub selected_waypoint_source_arc_points_m: Vec<Vec2>,
    pub selected_waypoint_target_arc_points_m: Vec<Vec2>,
    pub analytical_waypoint_position_m: Option<Vec2>,
    pub runtime_waypoint_position_m: Option<Vec2>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5AnalyticalCaseV1 {
    pub case_id: String,
    pub input_identity: String,
    pub prediction_identity: String,
    pub analytical_canary_identity: Option<String>,
    pub waypoint_search_identity: Option<String>,
    pub status: F5AnalyticalCaseStatusV1,
    pub eligible_for_controller: bool,
    pub candidate_projection_identity: Option<String>,
    pub runtime_projection_identity: Option<String>,
    pub flat_selected_candidate_identity: Option<String>,
    pub derived_nominal_candidate_identity: Option<String>,
    pub selected_waypoint_candidate_identity: Option<String>,
    pub flat_runtime_outcome_identity: Option<String>,
    pub derived_runtime_outcome_identity: Option<String>,
    pub candidate_expectation: Option<F5CandidateExpectationComparisonV1>,
    pub runtime_expectation: Option<F5RuntimeExpectationComparisonV1>,
    pub stop: Option<F5AnalyticalStopV1>,
    pub visual: Option<F5AnalyticalVisualV1>,
    pub candidate_projection: Option<ExperimentalRidgeCaseProjectionV1>,
    pub runtime_projection: Option<ExperimentalRidgeCaseRuntimeProjectionV2>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5AnalyticalArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub setup_id: String,
    pub input_manifest_identity: String,
    pub prediction_manifest_identity: String,
    pub status: F5AnalyticalArtifactStatusV1,
    pub cases: Vec<ConservativeBallisticRidgeF5AnalyticalCaseV1>,
    pub deterministic_repeat: bool,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5AnalyticalResultCaseV1 {
    pub case_id: String,
    pub input_identity: String,
    pub prediction_identity: String,
    pub analytical_canary_identity: Option<String>,
    pub waypoint_search_identity: Option<String>,
    pub status: F5AnalyticalCaseStatusV1,
    pub eligible_for_controller: bool,
    pub candidate_projection_identity: Option<String>,
    pub runtime_projection_identity: Option<String>,
    pub flat_selected_candidate_identity: Option<String>,
    pub derived_nominal_candidate_identity: Option<String>,
    pub selected_waypoint_candidate_identity: Option<String>,
    pub flat_runtime_outcome_identity: Option<String>,
    pub derived_runtime_outcome_identity: Option<String>,
    pub candidate_expectation: Option<F5CandidateExpectationComparisonV1>,
    pub runtime_expectation: Option<F5RuntimeExpectationComparisonV1>,
    pub stop: Option<F5AnalyticalStopV1>,
    pub case_identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5AnalyticalResultV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub analytical_artifact_identity: String,
    pub input_manifest_identity: String,
    pub prediction_manifest_identity: String,
    pub status: F5AnalyticalArtifactStatusV1,
    pub deterministic_repeat: bool,
    pub cases: Vec<ConservativeBallisticRidgeF5AnalyticalResultCaseV1>,
    pub identity: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConservativeBallisticRidgeF5AnalyticalPathsV1 {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
    pub result_path: PathBuf,
    pub report_path: PathBuf,
}
#[derive(Clone, Debug)]
pub struct ConservativeBallisticRidgeF5AnalyticalRunV1 {
    pub artifact: ConservativeBallisticRidgeF5AnalyticalArtifactV1,
    pub result: ConservativeBallisticRidgeF5AnalyticalResultV1,
    pub paths: ConservativeBallisticRidgeF5AnalyticalPathsV1,
}

/// Reveal the sealed F5 cases through analytical and runtime-V2 APIs only.
/// Calling this is intentionally an explicit phase boundary; it writes the
/// detailed ignored bundle, display-only report, and immutable compact result.
pub fn run_conservative_ballistic_ridge_f5_analytical_v1(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
    requested_result_path: Option<&Path>,
) -> Result<ConservativeBallisticRidgeF5AnalyticalRunV1> {
    let inputs = load_conservative_ballistic_ridge_f5_inputs_v1()
        .map_err(|e| anyhow!("F5 input seal is invalid: {e}"))?;
    let predictions = load_conservative_ballistic_ridge_f5_predictions_v1()
        .map_err(|e| anyhow!("F5 prediction seal is invalid: {e}"))?;
    predictions
        .validate(&inputs)
        .map_err(|e| anyhow!("F5 prediction seal is invalid: {e}"))?;
    let mut first = build_artifact(&inputs, &predictions)?;
    let mut repeat = build_artifact(&inputs, &predictions)?;
    first.deterministic_repeat = true;
    repeat.deterministic_repeat = true;
    first.finalize_identity()?;
    repeat.finalize_identity()?;
    let first_bytes = serde_json::to_vec_pretty(&first)?;
    if first_bytes != serde_json::to_vec_pretty(&repeat)? {
        bail!("F5 analytical repeat is not byte-identical");
    }
    first.validate_against(&inputs, &predictions)?;
    let result = ConservativeBallisticRidgeF5AnalyticalResultV1::from_artifact(&first)?;
    result.validate_against_artifact(&first)?;
    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create F5 analytical output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    fs::write(&summary_path, &first_bytes).with_context(|| {
        format!(
            "failed to write F5 analytical summary {}",
            summary_path.display()
        )
    })?;
    let reloaded = load_conservative_ballistic_ridge_f5_analytical_artifact_v1(
        &summary_path,
        &inputs,
        &predictions,
    )?;
    if serde_json::to_vec_pretty(&reloaded)? != first_bytes {
        bail!("F5 analytical summary is not byte-stable after reload");
    }
    let result_path = resolve_result_path(repo_root, requested_result_path);
    write_analytical_result(&result_path, &result)?;
    let reloaded_result = load_conservative_ballistic_ridge_f5_analytical_result_v1(&result_path)?;
    reloaded_result.validate_against_artifact(&reloaded)?;
    let (report_path, site) = report_path(repo_root, &output_dir)?;
    pd_report::conservative_ballistic_f5_analytical::write_conservative_ballistic_f5_analytical_report(&report_path, &serde_json::to_value(&reloaded)?)?;
    if let Some(site) = site {
        site.update_indexes_for_file(&report_path)?;
    }
    Ok(ConservativeBallisticRidgeF5AnalyticalRunV1 {
        artifact: reloaded,
        result: reloaded_result,
        paths: ConservativeBallisticRidgeF5AnalyticalPathsV1 {
            output_dir,
            summary_path,
            result_path,
            report_path,
        },
    })
}

pub fn load_conservative_ballistic_ridge_f5_analytical_artifact_v1(
    path: &Path,
    inputs: &ConservativeBallisticRidgeF5InputsV1,
    predictions: &ConservativeBallisticRidgeF5PredictionsV1,
) -> Result<ConservativeBallisticRidgeF5AnalyticalArtifactV1> {
    let artifact: ConservativeBallisticRidgeF5AnalyticalArtifactV1 = serde_json::from_slice(
        &fs::read(path)
            .with_context(|| format!("failed to read F5 analytical summary {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse F5 analytical summary {}", path.display()))?;
    artifact.validate_against(inputs, predictions)?;
    Ok(artifact)
}
pub fn load_conservative_ballistic_ridge_f5_analytical_result_v1(
    path: &Path,
) -> Result<ConservativeBallisticRidgeF5AnalyticalResultV1> {
    let result: ConservativeBallisticRidgeF5AnalyticalResultV1 = serde_json::from_slice(
        &fs::read(path)
            .with_context(|| format!("failed to read F5 analytical result {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse F5 analytical result {}", path.display()))?;
    let inputs = load_conservative_ballistic_ridge_f5_inputs_v1()
        .map_err(|e| anyhow!("F5 input seal is invalid: {e}"))?;
    let predictions = load_conservative_ballistic_ridge_f5_predictions_v1()
        .map_err(|e| anyhow!("F5 prediction seal is invalid: {e}"))?;
    result.validate_against_seals(&inputs, &predictions)?;
    Ok(result)
}

impl ConservativeBallisticRidgeF5AnalyticalArtifactV1 {
    fn finalize_identity(&mut self) -> Result<()> {
        self.identity.clear();
        self.identity = canonical_digest(self).map_err(anyhow::Error::msg)?;
        Ok(())
    }
    pub fn validate_against(
        &self,
        inputs: &ConservativeBallisticRidgeF5InputsV1,
        predictions: &ConservativeBallisticRidgeF5PredictionsV1,
    ) -> Result<()> {
        inputs
            .validate()
            .map_err(|e| anyhow!("F5 input seal invalid: {e}"))?;
        predictions
            .validate(inputs)
            .map_err(|e| anyhow!("F5 prediction seal invalid: {e}"))?;
        if self.schema_id != CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_SCHEMA_ID_V1
            || self.schema_version != 1
            || self.setup_id != CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_SETUP_ID_V1
            || !self.deterministic_repeat
            || self.input_manifest_identity != inputs.identity
            || self.prediction_manifest_identity != predictions.identity
            || self.cases.len() != inputs.cases.len()
        {
            bail!("F5 analytical artifact root contract is invalid");
        }
        let expected_status = if self.cases.iter().all(|case| case.eligible_for_controller) {
            F5AnalyticalArtifactStatusV1::AllEligibleForController
        } else {
            F5AnalyticalArtifactStatusV1::OneOrMoreCasesStopped
        };
        if self.status != expected_status {
            bail!("F5 analytical artifact aggregate status does not match case eligibility");
        }
        for ((case, input), prediction) in
            self.cases.iter().zip(&inputs.cases).zip(&predictions.cases)
        {
            let expected = build_case(input, prediction, &inputs.identity)?;
            validate_case_state(case)?;
            if case != &expected {
                bail!("F5 analytical case {} does not recompute", case.case_id);
            }
        }
        let mut material = self.clone();
        material.identity.clear();
        if self.identity.is_empty()
            || self.identity != canonical_digest(&material).map_err(anyhow::Error::msg)?
        {
            bail!("F5 analytical artifact identity does not bind contents");
        }
        Ok(())
    }
}

impl ConservativeBallisticRidgeF5AnalyticalResultV1 {
    fn from_artifact(artifact: &ConservativeBallisticRidgeF5AnalyticalArtifactV1) -> Result<Self> {
        let mut result = Self {
            schema_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_SCHEMA_ID_V1.to_owned(),
            schema_version: 1,
            analytical_artifact_identity: artifact.identity.clone(),
            input_manifest_identity: artifact.input_manifest_identity.clone(),
            prediction_manifest_identity: artifact.prediction_manifest_identity.clone(),
            status: artifact.status.clone(),
            deterministic_repeat: artifact.deterministic_repeat,
            cases: artifact.cases.iter().map(compact_case).collect(),
            identity: String::new(),
        };
        result.identity = canonical_digest(&result).map_err(anyhow::Error::msg)?;
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema_id != CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_SCHEMA_ID_V1
            || self.schema_version != 1
            || !self.deterministic_repeat
            || self.analytical_artifact_identity.is_empty()
            || self.input_manifest_identity.is_empty()
            || self.prediction_manifest_identity.is_empty()
            || self.cases.len() != 2
        {
            bail!("F5 analytical result root contract is invalid");
        }
        let mut ids = BTreeSet::new();
        for case in &self.cases {
            if case.case_id.is_empty()
                || case.input_identity.is_empty()
                || case.prediction_identity.is_empty()
                || case.case_identity.is_empty()
                || !ids.insert(case.case_id.clone())
                || case.eligible_for_controller
                    != matches!(case.status, F5AnalyticalCaseStatusV1::EligibleForController)
                || !compact_case_state_valid(case)
            {
                bail!("F5 analytical result case contract is invalid");
            }
        }
        let expected_status = if self.cases.iter().all(|case| case.eligible_for_controller) {
            F5AnalyticalArtifactStatusV1::AllEligibleForController
        } else {
            F5AnalyticalArtifactStatusV1::OneOrMoreCasesStopped
        };
        if self.status != expected_status {
            bail!("F5 analytical result aggregate status does not match case eligibility");
        }
        let mut material = self.clone();
        material.identity.clear();
        if self.identity.is_empty()
            || self.identity != canonical_digest(&material).map_err(anyhow::Error::msg)?
        {
            bail!("F5 analytical result identity does not bind contents");
        }
        Ok(())
    }
    pub fn validate_against_artifact(
        &self,
        artifact: &ConservativeBallisticRidgeF5AnalyticalArtifactV1,
    ) -> Result<()> {
        self.validate()?;
        if self != &Self::from_artifact(artifact)? {
            bail!("F5 analytical result does not match artifact");
        }
        Ok(())
    }

    /// Validate the compact result's seal joins without executing candidate
    /// or runtime projection code.  This is the read-side F5d join gate.
    pub fn validate_against_seals(
        &self,
        inputs: &ConservativeBallisticRidgeF5InputsV1,
        predictions: &ConservativeBallisticRidgeF5PredictionsV1,
    ) -> Result<()> {
        self.validate()?;
        inputs
            .validate()
            .map_err(|e| anyhow!("F5 input seal invalid: {e}"))?;
        predictions
            .validate(inputs)
            .map_err(|e| anyhow!("F5 prediction seal invalid: {e}"))?;
        if self.input_manifest_identity != inputs.identity
            || self.prediction_manifest_identity != predictions.identity
            || self.cases.len() != inputs.cases.len()
        {
            bail!("F5 analytical result manifest identities do not bind committed seals");
        }
        for ((case, input), prediction) in
            self.cases.iter().zip(&inputs.cases).zip(&predictions.cases)
        {
            if case.case_id != input.probe.id
                || case.case_id != prediction.case_id
                || case.input_identity != input.identity
                || case.prediction_identity != prediction_identity(&inputs.identity, prediction)?
            {
                bail!("F5 analytical result case order or seal join is invalid");
            }
        }
        Ok(())
    }
}

fn compact_case(
    case: &ConservativeBallisticRidgeF5AnalyticalCaseV1,
) -> ConservativeBallisticRidgeF5AnalyticalResultCaseV1 {
    ConservativeBallisticRidgeF5AnalyticalResultCaseV1 {
        case_id: case.case_id.clone(),
        input_identity: case.input_identity.clone(),
        prediction_identity: case.prediction_identity.clone(),
        analytical_canary_identity: case.analytical_canary_identity.clone(),
        waypoint_search_identity: case.waypoint_search_identity.clone(),
        status: case.status.clone(),
        eligible_for_controller: case.eligible_for_controller,
        candidate_projection_identity: case.candidate_projection_identity.clone(),
        runtime_projection_identity: case.runtime_projection_identity.clone(),
        flat_selected_candidate_identity: case.flat_selected_candidate_identity.clone(),
        derived_nominal_candidate_identity: case.derived_nominal_candidate_identity.clone(),
        selected_waypoint_candidate_identity: case.selected_waypoint_candidate_identity.clone(),
        flat_runtime_outcome_identity: case.flat_runtime_outcome_identity.clone(),
        derived_runtime_outcome_identity: case.derived_runtime_outcome_identity.clone(),
        candidate_expectation: case.candidate_expectation.clone(),
        runtime_expectation: case.runtime_expectation.clone(),
        stop: case.stop.clone(),
        case_identity: case.identity.clone(),
    }
}

fn validate_case_state(case: &ConservativeBallisticRidgeF5AnalyticalCaseV1) -> Result<()> {
    if case.case_id.is_empty()
        || case.input_identity.is_empty()
        || case.prediction_identity.is_empty()
        || case.identity.is_empty()
    {
        bail!("F5 analytical case has an empty required identity");
    }
    if case.eligible_for_controller {
        if case.status != F5AnalyticalCaseStatusV1::EligibleForController
            || case.stop.is_some()
            || case
                .analytical_canary_identity
                .as_deref()
                .is_none_or(str::is_empty)
            || case
                .waypoint_search_identity
                .as_deref()
                .is_none_or(str::is_empty)
            || case
                .candidate_projection_identity
                .as_deref()
                .is_none_or(str::is_empty)
            || case
                .runtime_projection_identity
                .as_deref()
                .is_none_or(str::is_empty)
            || case
                .flat_selected_candidate_identity
                .as_deref()
                .is_none_or(str::is_empty)
            || case
                .derived_nominal_candidate_identity
                .as_deref()
                .is_none_or(str::is_empty)
            || case
                .selected_waypoint_candidate_identity
                .as_deref()
                .is_none_or(str::is_empty)
            || case
                .flat_runtime_outcome_identity
                .as_deref()
                .is_none_or(str::is_empty)
            || case
                .derived_runtime_outcome_identity
                .as_deref()
                .is_none_or(str::is_empty)
            || case
                .candidate_expectation
                .as_ref()
                .is_none_or(|value| !value.passed())
            || case
                .runtime_expectation
                .as_ref()
                .is_none_or(|value| !value.passed())
            || case.visual.is_none()
            || case.candidate_projection.is_none()
            || case.runtime_projection.is_none()
        {
            bail!("eligible F5 analytical case has incomplete or inconsistent evidence");
        }
    } else if case.status == F5AnalyticalCaseStatusV1::EligibleForController
        || case
            .stop
            .as_ref()
            .is_none_or(|stop| !stop_matches_status(&case.status, stop))
    {
        bail!("stopped F5 analytical case has inconsistent stop evidence");
    }
    Ok(())
}

fn compact_case_state_valid(case: &ConservativeBallisticRidgeF5AnalyticalResultCaseV1) -> bool {
    if case.eligible_for_controller {
        case.status == F5AnalyticalCaseStatusV1::EligibleForController
            && case.stop.is_none()
            && case
                .analytical_canary_identity
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && case
                .waypoint_search_identity
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && case
                .candidate_projection_identity
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && case
                .runtime_projection_identity
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && case
                .flat_selected_candidate_identity
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && case
                .derived_nominal_candidate_identity
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && case
                .selected_waypoint_candidate_identity
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && case
                .flat_runtime_outcome_identity
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && case
                .derived_runtime_outcome_identity
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && case
                .candidate_expectation
                .as_ref()
                .is_some_and(F5CandidateExpectationComparisonV1::passed)
            && case
                .runtime_expectation
                .as_ref()
                .is_some_and(F5RuntimeExpectationComparisonV1::passed)
    } else {
        case.status != F5AnalyticalCaseStatusV1::EligibleForController
            && case
                .stop
                .as_ref()
                .is_some_and(|stop| stop_matches_status(&case.status, stop))
    }
}

fn stop_matches_status(status: &F5AnalyticalCaseStatusV1, stop: &F5AnalyticalStopV1) -> bool {
    if stop.reason.is_empty() {
        return false;
    }
    match (status, &stop.stage, &stop.classification, &stop.detail) {
        (
            F5AnalyticalCaseStatusV1::StoppedCandidateUnsupported,
            F5AnalyticalStopStageV1::CandidateProjection,
            F5AnalyticalStopClassV1::Unsupported,
            F5AnalyticalStopDetailV1::Unsupported { .. },
        ) => true,
        (
            F5AnalyticalCaseStatusV1::StoppedCandidateError,
            F5AnalyticalStopStageV1::CandidateProjection,
            F5AnalyticalStopClassV1::ProjectionError,
            F5AnalyticalStopDetailV1::CandidateError { .. },
        ) => true,
        (
            F5AnalyticalCaseStatusV1::StoppedCandidatePredictionMismatch,
            F5AnalyticalStopStageV1::CandidatePrediction,
            F5AnalyticalStopClassV1::PredictionMismatch,
            F5AnalyticalStopDetailV1::FailedChecks { failed_checks },
        ) => !failed_checks.is_empty(),
        (
            F5AnalyticalCaseStatusV1::StoppedRuntimeUnsupported,
            F5AnalyticalStopStageV1::RuntimeProjection,
            F5AnalyticalStopClassV1::Unsupported,
            F5AnalyticalStopDetailV1::Unsupported { .. },
        ) => true,
        (
            F5AnalyticalCaseStatusV1::StoppedRuntimeProjectionError,
            F5AnalyticalStopStageV1::RuntimeProjection,
            F5AnalyticalStopClassV1::ProjectionError,
            F5AnalyticalStopDetailV1::RuntimeProjectionError { .. },
        ) => true,
        (
            F5AnalyticalCaseStatusV1::StoppedRuntimePredictionMismatch,
            F5AnalyticalStopStageV1::RuntimePrediction,
            F5AnalyticalStopClassV1::PredictionMismatch,
            F5AnalyticalStopDetailV1::FailedChecks { failed_checks },
        ) => !failed_checks.is_empty(),
        (
            F5AnalyticalCaseStatusV1::StoppedInvalidEvidence,
            F5AnalyticalStopStageV1::EvidenceValidation,
            F5AnalyticalStopClassV1::InvalidEvidence,
            F5AnalyticalStopDetailV1::CandidateError { .. },
        ) => true,
        (
            F5AnalyticalCaseStatusV1::StoppedInvalidEvidence,
            F5AnalyticalStopStageV1::EvidenceValidation,
            F5AnalyticalStopClassV1::InvalidEvidence,
            F5AnalyticalStopDetailV1::RuntimeProjectionError { .. },
        ) => true,
        (
            F5AnalyticalCaseStatusV1::StoppedInvalidEvidence,
            F5AnalyticalStopStageV1::EvidenceValidation,
            F5AnalyticalStopClassV1::InvalidEvidence,
            F5AnalyticalStopDetailV1::FailedChecks { failed_checks },
        ) => !failed_checks.is_empty(),
        _ => false,
    }
}

fn build_artifact(
    inputs: &ConservativeBallisticRidgeF5InputsV1,
    predictions: &ConservativeBallisticRidgeF5PredictionsV1,
) -> Result<ConservativeBallisticRidgeF5AnalyticalArtifactV1> {
    let cases = inputs
        .cases
        .iter()
        .zip(&predictions.cases)
        .map(|(input, prediction)| build_case(input, prediction, &inputs.identity))
        .collect::<Result<Vec<_>>>()?;
    let status = if cases.iter().all(|case| case.eligible_for_controller) {
        F5AnalyticalArtifactStatusV1::AllEligibleForController
    } else {
        F5AnalyticalArtifactStatusV1::OneOrMoreCasesStopped
    };
    Ok(ConservativeBallisticRidgeF5AnalyticalArtifactV1 {
        schema_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_SCHEMA_ID_V1.to_owned(),
        schema_version: 1,
        setup_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_SETUP_ID_V1.to_owned(),
        input_manifest_identity: inputs.identity.clone(),
        prediction_manifest_identity: predictions.identity.clone(),
        status,
        cases,
        deterministic_repeat: false,
        identity: String::new(),
    })
}

fn build_case(
    input: &ExperimentalRidgeCaseInputV1,
    prediction: &ConservativeBallisticRidgeF5PredictionV1,
    input_manifest_identity: &str,
) -> Result<ConservativeBallisticRidgeF5AnalyticalCaseV1> {
    let prediction_identity = prediction_identity(input_manifest_identity, prediction)?;
    let mut case = base_case(input, prediction_identity);
    if prediction.case_id != input.probe.id || prediction.input_identity != input.identity {
        return finish_stopped(
            case,
            F5AnalyticalCaseStatusV1::StoppedInvalidEvidence,
            F5AnalyticalStopStageV1::EvidenceValidation,
            F5AnalyticalStopClassV1::InvalidEvidence,
            "prediction does not join input",
            F5AnalyticalStopDetailV1::FailedChecks {
                failed_checks: vec!["prediction_input_join".to_owned()],
            },
        );
    }
    let projection = match evaluate_experimental_ridge_case_projection_v1(input) {
        Ok(value) => value,
        Err(error) => {
            let invalid_input = matches!(
                error,
                ExperimentalRidgeCandidateErrorV2::GenericInputInvalid
                    | ExperimentalRidgeCandidateErrorV2::GenericInputIdentityMismatch
            );
            return finish_stopped(
                case,
                if invalid_input {
                    F5AnalyticalCaseStatusV1::StoppedInvalidEvidence
                } else {
                    F5AnalyticalCaseStatusV1::StoppedCandidateError
                },
                if invalid_input {
                    F5AnalyticalStopStageV1::EvidenceValidation
                } else {
                    F5AnalyticalStopStageV1::CandidateProjection
                },
                if invalid_input {
                    F5AnalyticalStopClassV1::InvalidEvidence
                } else {
                    F5AnalyticalStopClassV1::ProjectionError
                },
                error.to_string(),
                F5AnalyticalStopDetailV1::CandidateError { error },
            );
        }
    };
    case.candidate_projection_identity = Some(projection.identity.clone());
    case.analytical_canary_identity = Some(projection.analytical_canary_identity.clone());
    case.waypoint_search_identity = Some(projection.waypoint_search_identity.clone());
    if let Err(error) = validate_experimental_ridge_case_projection_v1(&projection) {
        return finish_stopped(
            case,
            F5AnalyticalCaseStatusV1::StoppedInvalidEvidence,
            F5AnalyticalStopStageV1::EvidenceValidation,
            F5AnalyticalStopClassV1::InvalidEvidence,
            error.to_string(),
            F5AnalyticalStopDetailV1::CandidateError { error },
        );
    }
    case.candidate_expectation = Some(compare_candidate(input, prediction, &projection));
    case.visual = Some(visual_from_projection(input, &projection));
    populate_candidate_identities(&mut case, &projection);
    if let Some(detail) = unsupported_candidate_detail(&projection) {
        case.candidate_projection = Some(projection);
        return finish_stopped(
            case,
            F5AnalyticalCaseStatusV1::StoppedCandidateUnsupported,
            F5AnalyticalStopStageV1::CandidateProjection,
            F5AnalyticalStopClassV1::Unsupported,
            "candidate outcome is unsupported",
            detail,
        );
    }
    if !case
        .candidate_expectation
        .as_ref()
        .is_some_and(F5CandidateExpectationComparisonV1::passed)
    {
        case.candidate_projection = Some(projection);
        let failed_checks = candidate_failed_checks(
            case.candidate_expectation
                .as_ref()
                .expect("comparison was assigned"),
        );
        return finish_stopped(
            case,
            F5AnalyticalCaseStatusV1::StoppedCandidatePredictionMismatch,
            F5AnalyticalStopStageV1::CandidatePrediction,
            F5AnalyticalStopClassV1::PredictionMismatch,
            "candidate outcome does not match frozen prediction",
            F5AnalyticalStopDetailV1::FailedChecks { failed_checks },
        );
    }
    let runtime = match project_experimental_ridge_case_runtime_v2(&projection) {
        Ok(value) => value,
        Err(error) => {
            case.candidate_projection = Some(projection);
            return finish_stopped(
                case,
                F5AnalyticalCaseStatusV1::StoppedRuntimeProjectionError,
                F5AnalyticalStopStageV1::RuntimeProjection,
                F5AnalyticalStopClassV1::ProjectionError,
                error.to_string(),
                F5AnalyticalStopDetailV1::RuntimeProjectionError { error },
            );
        }
    };
    case.runtime_projection_identity = Some(runtime.identity.clone());
    if let Err(error) = validate_experimental_ridge_case_runtime_v2(&projection, &runtime) {
        case.candidate_projection = Some(projection);
        case.runtime_projection = Some(runtime);
        return finish_stopped(
            case,
            F5AnalyticalCaseStatusV1::StoppedInvalidEvidence,
            F5AnalyticalStopStageV1::EvidenceValidation,
            F5AnalyticalStopClassV1::InvalidEvidence,
            error.to_string(),
            F5AnalyticalStopDetailV1::RuntimeProjectionError { error },
        );
    }
    case.runtime_expectation = Some(compare_runtime(prediction, &projection, &runtime));
    if let (Some(visual), Some(observed)) = (
        case.visual.as_mut(),
        case.runtime_expectation
            .as_ref()
            .and_then(|comparison| comparison.observed_handoff.as_ref()),
    ) {
        visual.runtime_waypoint_position_m = Some(observed.waypoint_position_m);
    }
    populate_runtime_identities(&mut case, &runtime);
    if let Some(detail) = unsupported_runtime_detail(&runtime) {
        case.candidate_projection = Some(projection);
        case.runtime_projection = Some(runtime);
        return finish_stopped(
            case,
            F5AnalyticalCaseStatusV1::StoppedRuntimeUnsupported,
            F5AnalyticalStopStageV1::RuntimeProjection,
            F5AnalyticalStopClassV1::Unsupported,
            "runtime outcome is unsupported",
            detail,
        );
    }
    if !case
        .runtime_expectation
        .as_ref()
        .is_some_and(F5RuntimeExpectationComparisonV1::passed)
    {
        case.candidate_projection = Some(projection);
        case.runtime_projection = Some(runtime);
        let failed_checks = runtime_failed_checks(
            case.runtime_expectation
                .as_ref()
                .expect("comparison was assigned"),
        );
        return finish_stopped(
            case,
            F5AnalyticalCaseStatusV1::StoppedRuntimePredictionMismatch,
            F5AnalyticalStopStageV1::RuntimePrediction,
            F5AnalyticalStopClassV1::PredictionMismatch,
            "runtime V2 outcome does not match frozen prediction",
            F5AnalyticalStopDetailV1::FailedChecks { failed_checks },
        );
    }
    case.status = F5AnalyticalCaseStatusV1::EligibleForController;
    case.eligible_for_controller = true;
    case.candidate_projection = Some(projection);
    case.runtime_projection = Some(runtime);
    finish_case(case)
}

fn base_case(
    input: &ExperimentalRidgeCaseInputV1,
    prediction_identity: String,
) -> ConservativeBallisticRidgeF5AnalyticalCaseV1 {
    ConservativeBallisticRidgeF5AnalyticalCaseV1 {
        case_id: input.probe.id.clone(),
        input_identity: input.identity.clone(),
        prediction_identity,
        analytical_canary_identity: None,
        waypoint_search_identity: None,
        status: F5AnalyticalCaseStatusV1::StoppedInvalidEvidence,
        eligible_for_controller: false,
        candidate_projection_identity: None,
        runtime_projection_identity: None,
        flat_selected_candidate_identity: None,
        derived_nominal_candidate_identity: None,
        selected_waypoint_candidate_identity: None,
        flat_runtime_outcome_identity: None,
        derived_runtime_outcome_identity: None,
        candidate_expectation: None,
        runtime_expectation: None,
        stop: None,
        visual: None,
        candidate_projection: None,
        runtime_projection: None,
        identity: String::new(),
    }
}
fn finish_stopped(
    mut case: ConservativeBallisticRidgeF5AnalyticalCaseV1,
    status: F5AnalyticalCaseStatusV1,
    stage: F5AnalyticalStopStageV1,
    classification: F5AnalyticalStopClassV1,
    reason: impl Into<String>,
    detail: F5AnalyticalStopDetailV1,
) -> Result<ConservativeBallisticRidgeF5AnalyticalCaseV1> {
    case.status = status;
    case.eligible_for_controller = false;
    case.stop = Some(F5AnalyticalStopV1 {
        stage,
        classification,
        reason: reason.into(),
        detail,
    });
    finish_case(case)
}
fn finish_case(
    mut case: ConservativeBallisticRidgeF5AnalyticalCaseV1,
) -> Result<ConservativeBallisticRidgeF5AnalyticalCaseV1> {
    case.identity.clear();
    case.identity = canonical_digest(&case).map_err(anyhow::Error::msg)?;
    Ok(case)
}

fn compare_candidate(
    input: &ExperimentalRidgeCaseInputV1,
    prediction: &ConservativeBallisticRidgeF5PredictionV1,
    projection: &ExperimentalRidgeCaseProjectionV1,
) -> F5CandidateExpectationComparisonV1 {
    let flat = match &projection.flat_control {
        ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => Some(candidate),
        _ => None,
    };
    let waypoint = match &projection.derived_mesa {
        ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
            candidate,
            crossing,
        } if crossing.candidate_identity == candidate.identity
            && crossing.strict_directed_crossing =>
        {
            Some(candidate)
        }
        _ => None,
    };
    F5CandidateExpectationComparisonV1 {
        flat_control_direct: prediction.analytical.flat_control.expected_outcome
            == ExpectedF5AnalyticalRouteOutcomeV1::Direct
            && flat.is_some(),
        flat_shortest_robust_certified: prediction.analytical.flat_control.selection_rule
            == F5FlatControlSelectionRuleV1::ShortestRobustCertified
            && flat.is_some_and(|candidate| {
                candidate.classification == CertificationV2::Certified
                    && projection
                        .flat_candidate_identities
                        .contains(&candidate.identity)
            }),
        flat_duration_multiplier_matches: flat.is_some_and(|candidate| {
            candidate.duration_multiplier == prediction.analytical.flat_control.duration_multiplier
        }),
        derived_blocker_valid: prediction.analytical.derived_mesa.blocker
            == F5DerivedBlockerExpectationV1::Valid
            && projection.derived_blocker_valid,
        derived_nominal_terrain_clearance_rejected: prediction
            .analytical
            .derived_mesa
            .nominal_direct_rejection
            == F5NominalDirectRejectionV1::TerrainClearance
            && flat.is_some_and(|candidate| {
                projection.derived_nominal_direct.classification != CertificationV2::Certified
                    && projection
                        .derived_nominal_direct
                        .reasons
                        .contains(&DirectBridgeReasonV2::TerrainClearance)
                    && projection.derived_nominal_direct.duration_multiplier
                        == candidate.duration_multiplier
                    && projection
                        .ridge_direct_candidate_identities
                        .contains(&projection.derived_nominal_direct.candidate_identity)
            }),
        derived_mesa_one_waypoint: prediction.analytical.derived_mesa.expected_outcome
            == ExpectedF5AnalyticalRouteOutcomeV1::OneWaypoint
            && waypoint
                .is_some_and(|candidate| candidate.classification == CertificationV2::Certified),
        exactly_one_certified_forward_progressing_waypoint: prediction
            .analytical
            .derived_mesa
            .bounded_waypoint_search
            == F5WaypointSearchExpectationV1::ExactlyOneCertifiedForwardProgressing
            && projection.waypoint_certified_candidate_count == 1
            && waypoint.is_some_and(|candidate| {
                projection
                    .waypoint_candidate_identities
                    .contains(&candidate.identity)
                    && candidate
                        .route_progress
                        .as_ref()
                        .is_some_and(|value| value.passes)
            }),
        candidate_projection_validated: input.identity == projection.input_identity,
    }
}

fn compare_runtime(
    prediction: &ConservativeBallisticRidgeF5PredictionV1,
    projection: &ExperimentalRidgeCaseProjectionV1,
    runtime: &ExperimentalRidgeCaseRuntimeProjectionV2,
) -> F5RuntimeExpectationComparisonV1 {
    let flat = match (&projection.flat_control, &runtime.flat_control) {
        (
            ExperimentalRidgeCandidateOutcomeV2::Direct { candidate },
            ExperimentalRidgeCaseRuntimeOutcomeV2::Direct {
                candidate_identity,
                structural_validation,
                ..
            },
        ) if candidate_identity == &candidate.identity => {
            structural_validation.transfer_route_valid
        }
        _ => false,
    };
    let mut observed_handoff = None;
    let mut derived = false;
    let mut handoff = false;
    let mut chain = false;
    if let (
        ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
            candidate,
            crossing,
        },
        ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
            candidate_identity,
            route,
            crossing: runtime_crossing,
            handoff_selection,
            handoff_assessment,
            structural_validation,
            ..
        },
    ) = (&projection.derived_mesa, &runtime.derived_mesa)
        && let Some(selected) = handoff_selection.selected_attempt()
    {
        let route_identity = canonical_digest(&selected.route).unwrap_or_default();
        observed_handoff = Some(F5ObservedHandoffV2 {
            selection_kind: selected.selection_kind,
            selected_attempt_index: handoff_selection.selected_attempt_index,
            handoff_selection_identity: handoff_selection.identity.clone(),
            selected_attempt_identity: selected.identity.clone(),
            route_identity,
            waypoint_position_m: selected
                .route
                .waypoints
                .first()
                .map(|waypoint| waypoint.position_m)
                .unwrap_or(Vec2::new(f64::NAN, f64::NAN)),
        });
        derived = prediction.analytical.derived_mesa.runtime_v2
            == F5RuntimeRouteStructureV2::StructurallyValidOneWaypoint
            && candidate_identity == &candidate.identity
            && structural_validation.transfer_route_valid
            && route.waypoints.len() == 1;
        handoff = prediction
            .analytical
            .derived_mesa
            .selected_semantic_handoff_contract
            == F5SemanticHandoffContractExpectationV2::Passing
            && handoff_assessment.contract_pass
            && selected.handoff_assessment.contract_pass;
        chain = runtime.input_identity == projection.input_identity
            && runtime.analytical_canary_identity == projection.analytical_canary_identity
            && runtime_crossing == &**crossing
            && handoff_selection.candidate_identity == candidate.identity
            && selected.route == *route
            && selected.handoff_assessment == *handoff_assessment;
    }
    F5RuntimeExpectationComparisonV1 {
        flat_direct_structurally_valid: flat,
        derived_structurally_valid_one_waypoint: derived,
        selected_semantic_handoff_contract_passes: handoff,
        selected_identity_chain_valid: chain,
        runtime_projection_validated: runtime.input_identity == projection.input_identity
            && prediction.analytical.deterministic_repeat
                == F5DeterministicRepeatRequirementV1::ByteIdenticalRepeatRequired,
        observed_handoff,
    }
}

fn populate_candidate_identities(
    case: &mut ConservativeBallisticRidgeF5AnalyticalCaseV1,
    projection: &ExperimentalRidgeCaseProjectionV1,
) {
    case.derived_nominal_candidate_identity =
        Some(projection.derived_nominal_direct.candidate_identity.clone());
    if let ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } = &projection.flat_control {
        case.flat_selected_candidate_identity = Some(candidate.identity.clone());
    }
    if let ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. } =
        &projection.derived_mesa
    {
        case.selected_waypoint_candidate_identity = Some(candidate.identity.clone());
    }
}
fn populate_runtime_identities(
    case: &mut ConservativeBallisticRidgeF5AnalyticalCaseV1,
    runtime: &ExperimentalRidgeCaseRuntimeProjectionV2,
) {
    if let ExperimentalRidgeCaseRuntimeOutcomeV2::Direct { identity, .. } = &runtime.flat_control {
        case.flat_runtime_outcome_identity = Some(identity.clone());
    }
    if let ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint { identity, .. } =
        &runtime.derived_mesa
    {
        case.derived_runtime_outcome_identity = Some(identity.clone());
    }
}
fn unsupported_candidate_detail(
    projection: &ExperimentalRidgeCaseProjectionV1,
) -> Option<F5AnalyticalStopDetailV1> {
    unsupported_candidate_outcome_detail(&projection.flat_control, F5AnalyticalLaneV1::FlatControl)
        .or_else(|| {
            unsupported_candidate_outcome_detail(
                &projection.derived_mesa,
                F5AnalyticalLaneV1::DerivedMesa,
            )
        })
}

fn unsupported_candidate_outcome_detail(
    outcome: &ExperimentalRidgeCandidateOutcomeV2,
    lane: F5AnalyticalLaneV1,
) -> Option<F5AnalyticalStopDetailV1> {
    match outcome {
        ExperimentalRidgeCandidateOutcomeV2::Unsupported {
            reason,
            rejection_reasons,
        } => Some(F5AnalyticalStopDetailV1::Unsupported {
            lane,
            reason: *reason,
            rejection_reasons: rejection_reasons.clone(),
        }),
        _ => None,
    }
}

fn unsupported_runtime_detail(
    runtime: &ExperimentalRidgeCaseRuntimeProjectionV2,
) -> Option<F5AnalyticalStopDetailV1> {
    unsupported_runtime_outcome_detail(&runtime.flat_control, F5AnalyticalLaneV1::FlatControl)
        .or_else(|| {
            unsupported_runtime_outcome_detail(
                &runtime.derived_mesa,
                F5AnalyticalLaneV1::DerivedMesa,
            )
        })
}

fn unsupported_runtime_outcome_detail(
    outcome: &ExperimentalRidgeCaseRuntimeOutcomeV2,
    lane: F5AnalyticalLaneV1,
) -> Option<F5AnalyticalStopDetailV1> {
    match outcome {
        ExperimentalRidgeCaseRuntimeOutcomeV2::Unsupported {
            reason,
            rejection_reasons,
            ..
        } => Some(F5AnalyticalStopDetailV1::Unsupported {
            lane,
            reason: *reason,
            rejection_reasons: rejection_reasons.clone(),
        }),
        _ => None,
    }
}

fn candidate_failed_checks(comparison: &F5CandidateExpectationComparisonV1) -> Vec<String> {
    [
        ("flat_control_direct", comparison.flat_control_direct),
        (
            "flat_shortest_robust_certified",
            comparison.flat_shortest_robust_certified,
        ),
        (
            "flat_duration_multiplier_matches",
            comparison.flat_duration_multiplier_matches,
        ),
        ("derived_blocker_valid", comparison.derived_blocker_valid),
        (
            "derived_nominal_terrain_clearance_rejected",
            comparison.derived_nominal_terrain_clearance_rejected,
        ),
        (
            "derived_mesa_one_waypoint",
            comparison.derived_mesa_one_waypoint,
        ),
        (
            "exactly_one_certified_forward_progressing_waypoint",
            comparison.exactly_one_certified_forward_progressing_waypoint,
        ),
        (
            "candidate_projection_validated",
            comparison.candidate_projection_validated,
        ),
    ]
    .into_iter()
    .filter_map(|(name, passes)| (!passes).then_some(name.to_owned()))
    .collect()
}

fn runtime_failed_checks(comparison: &F5RuntimeExpectationComparisonV1) -> Vec<String> {
    [
        (
            "flat_direct_structurally_valid",
            comparison.flat_direct_structurally_valid,
        ),
        (
            "derived_structurally_valid_one_waypoint",
            comparison.derived_structurally_valid_one_waypoint,
        ),
        (
            "selected_semantic_handoff_contract_passes",
            comparison.selected_semantic_handoff_contract_passes,
        ),
        (
            "selected_identity_chain_valid",
            comparison.selected_identity_chain_valid,
        ),
        (
            "runtime_projection_validated",
            comparison.runtime_projection_validated,
        ),
        ("observed_handoff", comparison.observed_handoff.is_some()),
    ]
    .into_iter()
    .filter_map(|(name, passes)| (!passes).then_some(name.to_owned()))
    .collect()
}
fn visual_from_projection(
    input: &ExperimentalRidgeCaseInputV1,
    projection: &ExperimentalRidgeCaseProjectionV1,
) -> F5AnalyticalVisualV1 {
    let flat = input
        .probe
        .terrain_points_m
        .first()
        .zip(input.probe.terrain_points_m.last())
        .map(|(a, b)| vec![*a, *b])
        .unwrap_or_default();
    let nominal = match &projection.flat_control {
        ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => {
            sampled_arc(&candidate.virtual_arc)
        }
        _ => Vec::new(),
    };
    let (source, target, waypoint) = match &projection.derived_mesa {
        ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. } => (
            sampled_arc(&candidate.source_leg),
            sampled_arc(&candidate.target_leg),
            Some(candidate.waypoint_position_m),
        ),
        _ => (Vec::new(), Vec::new(), None),
    };
    F5AnalyticalVisualV1 {
        flat_twin_terrain_points_m: flat,
        derived_mesa_terrain_points_m: projection.mesa.terrain_points_m.clone(),
        nominal_direct_arc_points_m: nominal,
        selected_waypoint_source_arc_points_m: source,
        selected_waypoint_target_arc_points_m: target,
        analytical_waypoint_position_m: waypoint,
        runtime_waypoint_position_m: None,
    }
}
fn sampled_arc(arc: &pd_plan::conservative_ballistic_bridge::VirtualBallisticArcV2) -> Vec<Vec2> {
    (0..=80)
        .map(|i| arc.state_at(arc.steps * i / 80).position_m)
        .collect()
}
fn prediction_identity(
    input_manifest_identity: &str,
    prediction: &ConservativeBallisticRidgeF5PredictionV1,
) -> Result<String> {
    canonical_digest(&(input_manifest_identity, prediction)).map_err(anyhow::Error::msg)
}
fn resolve_output_dir(root: &Path, requested: Option<&Path>) -> PathBuf {
    requested
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            }
        })
        .unwrap_or_else(|| {
            root.join("outputs/eval")
                .join(CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_SETUP_ID_V1)
        })
}
fn resolve_result_path(root: &Path, requested: Option<&Path>) -> PathBuf {
    requested
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            }
        })
        .unwrap_or_else(|| root.join(CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_FIXTURE_V1))
}
fn report_path(root: &Path, output: &Path) -> Result<(PathBuf, Option<ReportSite>)> {
    let repo_outputs = root.join("outputs");
    if fs::canonicalize(&repo_outputs)
        .ok()
        .zip(fs::canonicalize(output).ok())
        .is_some_and(|(base, path)| path.starts_with(base))
    {
        let site = ReportSite::new(root);
        Ok((
            site.default_output_for_bundle(output)
                .ok_or_else(|| anyhow!("F5 analytical output must be under repository outputs"))?,
            Some(site),
        ))
    } else {
        Ok((output.join("report/index.html"), None))
    }
}
fn write_analytical_result(
    path: &Path,
    result: &ConservativeBallisticRidgeF5AnalyticalResultV1,
) -> Result<()> {
    result.validate()?;
    let bytes = serde_json::to_vec_pretty(result)?;
    if path.exists() {
        let existing = fs::read(path).with_context(|| {
            format!(
                "failed to read existing F5 analytical result {}",
                path.display()
            )
        })?;
        let reloaded: ConservativeBallisticRidgeF5AnalyticalResultV1 =
            serde_json::from_slice(&existing).with_context(|| {
                format!(
                    "failed to parse existing F5 analytical result {}",
                    path.display()
                )
            })?;
        reloaded.validate()?;
        if existing != bytes || reloaded != *result {
            bail!(
                "existing F5 analytical result {} does not exactly match fresh evidence",
                path.display()
            );
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create F5 analytical result directory {}",
                parent.display()
            )
        })?;
    }
    fs::write(path, bytes)
        .with_context(|| format!("failed to write F5 analytical result {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_plan::conservative_ballistic_bridge::load_experimental_ridge_case_manifest_v1;

    fn historical_case(index: usize) -> ExperimentalRidgeCaseInputV1 {
        load_experimental_ridge_case_manifest_v1().cases[index].clone()
    }
    fn matching_prediction(
        input: &ExperimentalRidgeCaseInputV1,
    ) -> ConservativeBallisticRidgeF5PredictionV1 {
        let projection = evaluate_experimental_ridge_case_projection_v1(input).unwrap();
        let flat_duration = match projection.flat_control {
            ExperimentalRidgeCandidateOutcomeV2::Direct { ref candidate } => {
                candidate.duration_multiplier
            }
            _ => 1.0,
        };
        ConservativeBallisticRidgeF5PredictionV1 {
            case_id: input.probe.id.clone(),
            input_identity: input.identity.clone(),
            analytical: crate::F5AnalyticalPredictionV1 {
                flat_control: crate::F5FlatControlAnalyticalPredictionV1 {
                    expected_outcome: ExpectedF5AnalyticalRouteOutcomeV1::Direct,
                    selection_rule: F5FlatControlSelectionRuleV1::ShortestRobustCertified,
                    duration_multiplier: flat_duration,
                },
                derived_mesa: crate::F5DerivedMesaAnalyticalPredictionV1 {
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
            controller: crate::F5ControllerExpectationsV1 {
                flat_direct: crate::F5FlatDirectControllerExpectationV1 {
                    outcome: crate::F5ControllerLandingExpectationV1::TargetLanding,
                },
                mesa_direct: crate::F5MesaDirectControllerExpectationV1 {
                    outcome:
                        crate::F5ControllerTerrainContactExpectationV1::NonTargetTerrainContact,
                    location: crate::F5TerrainContactLocationExpectationV1::DerivedMesa,
                    ordering: crate::F5TerrainContactOrderingExpectationV1::BeforeTargetTouchdown,
                    precision: crate::F5ContactPrecisionExpectationV1::NoExactTimeOrPoint,
                },
                mesa_waypoint: crate::F5MesaWaypointControllerExpectationV1 {
                    capture: crate::F5WaypointCaptureExpectationV1::ExactlyOneContractPassCapture,
                    terminal_outcome: crate::F5ControllerLandingExpectationV1::TargetLanding,
                },
            },
        }
    }
    #[test]
    fn exposed_cases_exercise_both_branch_neutral_handoff_paths() {
        let mut kinds = BTreeSet::new();
        for index in 0..2 {
            let input = historical_case(index);
            let prediction = matching_prediction(&input);
            let case = build_case(&input, &prediction, "test").unwrap();
            assert!(case.eligible_for_controller, "{}: {case:?}", input.probe.id);
            assert_eq!(
                case.visual
                    .as_ref()
                    .and_then(|visual| visual.runtime_waypoint_position_m),
                case.runtime_expectation
                    .as_ref()
                    .and_then(|comparison| comparison.observed_handoff.as_ref())
                    .map(|handoff| handoff.waypoint_position_m)
            );
            kinds.insert(format!(
                "{:?}",
                case.runtime_expectation
                    .unwrap()
                    .observed_handoff
                    .unwrap()
                    .selection_kind
            ));
        }
        assert_eq!(kinds.len(), 2);
    }
    #[test]
    fn first_case_prediction_stop_does_not_prevent_second_case_evaluation() {
        let first = historical_case(0);
        let second = historical_case(1);
        let mut bad = matching_prediction(&first);
        bad.analytical.flat_control.duration_multiplier += 0.5;
        let good = matching_prediction(&second);
        let stopped = build_case(&first, &bad, "test").unwrap();
        let eligible = build_case(&second, &good, "test").unwrap();
        assert!(!stopped.eligible_for_controller);
        assert!(eligible.eligible_for_controller);
    }

    #[test]
    fn exposed_invalid_input_is_recorded_as_a_typed_case_stop() {
        let mut input = historical_case(0);
        let mut prediction = matching_prediction(&input);
        input.identity.push_str("-tampered");
        prediction.input_identity = input.identity.clone();
        let case = build_case(&input, &prediction, "test").unwrap();
        assert_eq!(
            case.status,
            F5AnalyticalCaseStatusV1::StoppedInvalidEvidence
        );
        let stop = case.stop.unwrap();
        assert_eq!(
            stop.classification,
            F5AnalyticalStopClassV1::InvalidEvidence
        );
        assert!(matches!(
            stop.detail,
            F5AnalyticalStopDetailV1::CandidateError {
                error: ExperimentalRidgeCandidateErrorV2::GenericInputIdentityMismatch
            }
        ));
    }
    #[test]
    fn compact_result_has_no_controller_or_simulator_fields_and_is_tamper_evident() {
        let input = historical_case(0);
        let prediction = matching_prediction(&input);
        let case = build_case(&input, &prediction, "test").unwrap();
        let mut second_case = case.clone();
        second_case.case_id.push_str("-second");
        second_case.identity.clear();
        second_case.identity = canonical_digest(&second_case).unwrap();
        let mut artifact = ConservativeBallisticRidgeF5AnalyticalArtifactV1 {
            schema_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_SCHEMA_ID_V1.to_owned(),
            schema_version: 1,
            setup_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_SETUP_ID_V1.to_owned(),
            input_manifest_identity: "test".to_owned(),
            prediction_manifest_identity: "test".to_owned(),
            status: F5AnalyticalArtifactStatusV1::AllEligibleForController,
            cases: vec![case, second_case],
            deterministic_repeat: true,
            identity: String::new(),
        };
        artifact.finalize_identity().unwrap();
        let mut result =
            ConservativeBallisticRidgeF5AnalyticalResultV1::from_artifact(&artifact).unwrap();
        let json = serde_json::to_value(&result).unwrap().to_string();
        assert!(!json.contains("controller_outcome"));
        assert!(!json.contains("simulation_outcome"));
        assert!(!json.contains("simulator_outcome"));
        let root = std::env::temp_dir().join(format!(
            "pd-eval-f5-analytical-result-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("result.json");
        write_analytical_result(&path, &result).unwrap();
        let reloaded: ConservativeBallisticRidgeF5AnalyticalResultV1 =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        reloaded.validate().unwrap();
        assert_eq!(reloaded.identity, result.identity);
        let mut bytes = fs::read(&path).unwrap();
        bytes.push(b'\n');
        fs::write(&path, bytes).unwrap();
        assert!(write_analytical_result(&path, &result).is_err());
        let _ = fs::remove_dir_all(root);
        result.cases[0].case_identity.push('x');
        result.identity.clear();
        result.identity = canonical_digest(&result).unwrap();
        assert!(result.validate_against_artifact(&artifact).is_err());
    }

    #[test]
    fn compact_result_rejects_missing_stop_incomplete_eligibility_and_aggregate_drift() {
        let first = historical_case(0);
        let second = historical_case(1);
        let mut bad = matching_prediction(&first);
        bad.analytical.flat_control.duration_multiplier += 0.5;
        let stopped = build_case(&first, &bad, "test").unwrap();
        let mut detailed_status_drift = stopped.clone();
        detailed_status_drift.status = F5AnalyticalCaseStatusV1::StoppedRuntimePredictionMismatch;
        assert!(validate_case_state(&detailed_status_drift).is_err());
        let mut detailed_stage_drift = stopped.clone();
        detailed_stage_drift.stop.as_mut().unwrap().stage =
            F5AnalyticalStopStageV1::RuntimePrediction;
        assert!(validate_case_state(&detailed_stage_drift).is_err());
        let eligible = build_case(&second, &matching_prediction(&second), "test").unwrap();
        let mut artifact = ConservativeBallisticRidgeF5AnalyticalArtifactV1 {
            schema_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_SCHEMA_ID_V1.to_owned(),
            schema_version: 1,
            setup_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_SETUP_ID_V1.to_owned(),
            input_manifest_identity: "test".to_owned(),
            prediction_manifest_identity: "test".to_owned(),
            status: F5AnalyticalArtifactStatusV1::OneOrMoreCasesStopped,
            cases: vec![stopped, eligible],
            deterministic_repeat: true,
            identity: String::new(),
        };
        artifact.finalize_identity().unwrap();
        let result =
            ConservativeBallisticRidgeF5AnalyticalResultV1::from_artifact(&artifact).unwrap();

        let mut missing_stop = result.clone();
        missing_stop.cases[0].stop = None;
        missing_stop.identity.clear();
        missing_stop.identity = canonical_digest(&missing_stop).unwrap();
        assert!(missing_stop.validate().is_err());

        let mut missing_candidate_comparison = result.clone();
        missing_candidate_comparison.cases[1].candidate_expectation = None;
        missing_candidate_comparison.identity.clear();
        missing_candidate_comparison.identity =
            canonical_digest(&missing_candidate_comparison).unwrap();
        assert!(missing_candidate_comparison.validate().is_err());

        let mut status_drift = result.clone();
        status_drift.cases[0].status = F5AnalyticalCaseStatusV1::StoppedRuntimePredictionMismatch;
        status_drift.identity.clear();
        status_drift.identity = canonical_digest(&status_drift).unwrap();
        assert!(status_drift.validate().is_err());

        let mut stage_drift = result.clone();
        stage_drift.cases[0].stop.as_mut().unwrap().stage =
            F5AnalyticalStopStageV1::RuntimePrediction;
        stage_drift.identity.clear();
        stage_drift.identity = canonical_digest(&stage_drift).unwrap();
        assert!(stage_drift.validate().is_err());

        let mut aggregate_drift = result;
        aggregate_drift.status = F5AnalyticalArtifactStatusV1::AllEligibleForController;
        aggregate_drift.identity.clear();
        aggregate_drift.identity = canonical_digest(&aggregate_drift).unwrap();
        assert!(aggregate_drift.validate().is_err());
        let inputs = load_conservative_ballistic_ridge_f5_inputs_v1().unwrap();
        let predictions = load_conservative_ballistic_ridge_f5_predictions_v1().unwrap();
        assert!(
            aggregate_drift
                .validate_against_seals(&inputs, &predictions)
                .is_err()
        );
    }
}
