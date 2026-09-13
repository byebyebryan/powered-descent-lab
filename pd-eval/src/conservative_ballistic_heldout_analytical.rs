//! H2b analytical reveal and immutable result for the frozen ridge held-out cases.
//!
//! This module intentionally stops at planner-owned analytical and runtime
//! projection evidence.  It never constructs a scenario, controller, or
//! simulator run.  A stopped compact result is deliberately not H3 authority;
//! the detailed artifact remains an ignored inspection bundle.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_core::Vec2;
use pd_plan::conservative_ballistic_bridge::{
    CertificationV2, DirectBridgeReasonV2, ExperimentalRidgeCandidateOutcomeV2,
    ExperimentalRidgeCaseInputV1, ExperimentalRidgeCaseManifestV1,
    ExperimentalRidgeCaseProjectionV1, ExperimentalRidgeCaseRuntimeProjectionV1,
    ExperimentalRidgeRuntimeOutcomeV2, ExperimentalRidgeRuntimeProjectionErrorV2,
    evaluate_experimental_ridge_case_projection_v1, project_experimental_ridge_case_runtime_v1,
    validate_experimental_ridge_case_projection_v1, validate_experimental_ridge_case_runtime_v1,
};
use pd_report::site::ReportSite;
use serde::{Deserialize, Serialize};

use crate::{
    ConservativeBallisticRidgeHeldoutPredictionV1, ConservativeBallisticRidgeHeldoutPredictionsV1,
    ExpectedAnalyticalRouteOutcomeV1, FlatControlSelectionRuleV1, LongerGlobalReplanExpectationV1,
    NominalDirectRejectionV1, ProjectionValidationRequirementV1,
    SelectedWaypointWitnessExpectationV1, canonical_digest,
    load_conservative_ballistic_ridge_heldout_predictions_v1,
};

pub const CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SCHEMA_ID_V1: &str =
    "conservative_ballistic_ridge_heldout_analytical_v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SCHEMA_VERSION_V1: u32 = 1;
pub const CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_RESULT_SCHEMA_ID_V1: &str =
    "conservative_ballistic_ridge_heldout_analytical_result_v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub const CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SETUP_ID_V1: &str =
    "conservative-ballistic-ridge-heldout-v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_RESULT_FIXTURE_V1: &str =
    "fixtures/manifests/conservative_ballistic_ridge_heldout_analytical_result_v1.json";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticalArtifactStatusV1 {
    Passed,
    StoppedRuntimeProjectionError,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticalExpectationStatusV1 {
    Passed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyticalExpectationComparisonV1 {
    pub status: AnalyticalExpectationStatusV1,
    pub flat_control_direct: bool,
    pub flat_shortest_robust_certified: bool,
    pub flat_duration_multiplier_matches: bool,
    pub derived_blocker_valid: bool,
    pub derived_nominal_terrain_clearance_rejected: bool,
    pub longer_global_replan_certified: bool,
    pub derived_mesa_one_waypoint: bool,
    pub exactly_one_certified_forward_progressing_waypoint: bool,
    pub candidate_projection_validated: bool,
    pub runtime_projection_validated: bool,
}

impl AnalyticalExpectationComparisonV1 {
    fn passed(&self) -> bool {
        self.status == AnalyticalExpectationStatusV1::Passed
            && self.flat_control_direct
            && self.flat_shortest_robust_certified
            && self.flat_duration_multiplier_matches
            && self.derived_blocker_valid
            && self.derived_nominal_terrain_clearance_rejected
            && self.longer_global_replan_certified
            && self.derived_mesa_one_waypoint
            && self.exactly_one_certified_forward_progressing_waypoint
            && self.candidate_projection_validated
            && self.runtime_projection_validated
    }
}

/// The frozen analytical prediction comparison completed before attempting a
/// runtime route.  A stopped H2b result retains this so the runtime failure is
/// not conflated with a candidate-selection mismatch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidatePredictionComparisonV1 {
    pub flat_control_direct: bool,
    pub flat_shortest_robust_certified: bool,
    pub flat_duration_multiplier_matches: bool,
    pub derived_blocker_valid: bool,
    pub derived_nominal_terrain_clearance_rejected: bool,
    pub longer_global_replan_certified: bool,
    pub derived_mesa_one_waypoint: bool,
    pub exactly_one_certified_forward_progressing_waypoint: bool,
    pub candidate_projection_validated: bool,
}

impl CandidatePredictionComparisonV1 {
    fn passed(&self) -> bool {
        self.flat_control_direct
            && self.flat_shortest_robust_certified
            && self.flat_duration_multiplier_matches
            && self.derived_blocker_valid
            && self.derived_nominal_terrain_clearance_rejected
            && self.longer_global_replan_certified
            && self.derived_mesa_one_waypoint
            && self.exactly_one_certified_forward_progressing_waypoint
            && self.candidate_projection_validated
    }
}

/// Display-only samples built by the evaluator from already-validated
/// analytical evidence.  The report crate renders these points; it never
/// regenerates a ballistic trajectory or planner decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeldoutAnalyticalVisualV1 {
    pub flat_twin_terrain_points_m: Vec<Vec2>,
    pub derived_mesa_terrain_points_m: Vec<Vec2>,
    pub nominal_direct_arc_points_m: Vec<Vec2>,
    pub selected_waypoint_source_arc_points_m: Vec<Vec2>,
    pub selected_waypoint_target_arc_points_m: Vec<Vec2>,
    pub analytical_waypoint_position_m: Vec2,
    pub runtime_waypoint_position_m: Option<Vec2>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeHeldoutAnalyticalCaseV1 {
    pub case_id: String,
    pub input_identity: String,
    pub prediction_identity: String,
    pub analytical_canary_identity: String,
    pub candidate_projection_identity: String,
    pub runtime_projection_identity: String,
    pub flat_selected_candidate_identity: String,
    pub derived_nominal_candidate_identity: String,
    pub derived_nominal_rejection_reasons: Vec<DirectBridgeReasonV2>,
    pub non_nominal_direct_candidate_identities: Vec<String>,
    pub non_nominal_direct_certified_count: usize,
    pub waypoint_search_identity: String,
    pub selected_waypoint_candidate_identity: String,
    pub flat_runtime_outcome_identity: String,
    pub derived_runtime_outcome_identity: String,
    pub expectation: AnalyticalExpectationComparisonV1,
    pub visual: HeldoutAnalyticalVisualV1,
    pub candidate_projection: ExperimentalRidgeCaseProjectionV1,
    pub runtime_projection: ExperimentalRidgeCaseRuntimeProjectionV1,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeHeldoutAnalyticalArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub setup_id: String,
    pub input_manifest_identity: String,
    pub prediction_manifest_identity: String,
    pub status: AnalyticalArtifactStatusV1,
    pub cases: Vec<ConservativeBallisticRidgeHeldoutAnalyticalCaseV1>,
    pub failure: Option<ConservativeBallisticRidgeHeldoutAnalyticalFailureV1>,
    pub deterministic_repeat: bool,
    pub identity: String,
}

/// Exact fail-closed H2b stop evidence.  The generic candidate projection has
/// passed independent recomputation; the runtime projection was not emitted
/// because its constructor returned the recorded error.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeHeldoutAnalyticalFailureV1 {
    pub case_id: String,
    pub input_identity: String,
    pub prediction_identity: String,
    pub analytical_canary_identity: String,
    pub candidate_projection_identity: String,
    pub candidate_expectation: CandidatePredictionComparisonV1,
    pub stage: String,
    pub runtime_error: ExperimentalRidgeRuntimeProjectionErrorV2,
    pub error: String,
    pub visual: HeldoutAnalyticalVisualV1,
    pub candidate_projection: ExperimentalRidgeCaseProjectionV1,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeHeldoutAnalyticalResultCaseV1 {
    pub case_id: String,
    pub input_identity: String,
    pub prediction_identity: String,
    pub analytical_canary_identity: String,
    pub candidate_projection_identity: String,
    pub runtime_projection_identity: String,
    pub flat_selected_candidate_identity: String,
    pub derived_nominal_candidate_identity: String,
    pub non_nominal_direct_candidate_identities: Vec<String>,
    pub selected_waypoint_candidate_identity: String,
    pub flat_runtime_outcome_identity: String,
    pub derived_runtime_outcome_identity: String,
    pub expectation: AnalyticalExpectationComparisonV1,
    pub case_identity: String,
}

/// Compact source-controlled outcome of the H2b analytical reveal.  The
/// current frozen result is a stop, so it is explicitly not H3 authority and
/// carries no controller input, controller output, or simulator outcome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeHeldoutAnalyticalResultV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub analytical_artifact_identity: String,
    pub input_manifest_identity: String,
    pub prediction_manifest_identity: String,
    pub status: AnalyticalArtifactStatusV1,
    pub deterministic_repeat: bool,
    pub completed_cases: Vec<ConservativeBallisticRidgeHeldoutAnalyticalResultCaseV1>,
    pub failure_case_id: String,
    pub failure_input_identity: String,
    pub failure_prediction_identity: String,
    pub failure_analytical_canary_identity: String,
    pub failure_candidate_projection_identity: String,
    pub failure_candidate_expectation: CandidatePredictionComparisonV1,
    pub failure_stage: String,
    pub failure_runtime_error: ExperimentalRidgeRuntimeProjectionErrorV2,
    pub failure_error: String,
    pub failure_identity: String,
    pub identity: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConservativeBallisticRidgeHeldoutAnalyticalPathsV1 {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
    pub result_path: PathBuf,
    pub report_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ConservativeBallisticRidgeHeldoutAnalyticalRunV1 {
    pub artifact: ConservativeBallisticRidgeHeldoutAnalyticalArtifactV1,
    pub result: ConservativeBallisticRidgeHeldoutAnalyticalResultV1,
    pub paths: ConservativeBallisticRidgeHeldoutAnalyticalPathsV1,
}

/// Run the H2b analytical reveal.  This validates both frozen manifests
/// before generic analytical evaluation and does not import controller or
/// simulator code.
pub fn run_conservative_ballistic_ridge_heldout_analytical_v1(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
    requested_result_path: Option<&Path>,
) -> Result<ConservativeBallisticRidgeHeldoutAnalyticalRunV1> {
    let input_manifest =
        pd_plan::conservative_ballistic_bridge::load_experimental_ridge_case_manifest_v1();
    input_manifest
        .validate()
        .map_err(|error| anyhow!("held-out input manifest is invalid: {error}"))?;
    let predictions = load_conservative_ballistic_ridge_heldout_predictions_v1()
        .map_err(|error| anyhow!("held-out prediction manifest is invalid: {error}"))?;
    predictions
        .validate(&input_manifest)
        .map_err(|error| anyhow!("held-out prediction manifest is invalid: {error}"))?;

    let mut first = build_artifact(&input_manifest, &predictions)?;
    let mut repeat = build_artifact(&input_manifest, &predictions)?;
    first.deterministic_repeat = true;
    repeat.deterministic_repeat = true;
    first.finalize_identity()?;
    repeat.finalize_identity()?;
    let first_bytes = serde_json::to_vec_pretty(&first)?;
    let repeat_bytes = serde_json::to_vec_pretty(&repeat)?;
    if first_bytes != repeat_bytes {
        bail!("held-out analytical repeat is not byte-identical");
    }
    first.validate_against(&input_manifest, &predictions)?;
    let result = ConservativeBallisticRidgeHeldoutAnalyticalResultV1::from_artifact(&first)?;
    result.validate_against_artifact(&first)?;

    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create held-out analytical output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    fs::write(&summary_path, &first_bytes).with_context(|| {
        format!(
            "failed to write held-out analytical summary {}",
            summary_path.display()
        )
    })?;
    let reloaded = load_conservative_ballistic_ridge_heldout_analytical_artifact_v1(
        &summary_path,
        &input_manifest,
        &predictions,
    )?;
    if serde_json::to_vec_pretty(&reloaded)? != first_bytes {
        bail!("held-out analytical summary is not byte-stable after reload");
    }

    let result_path = resolve_result_path(repo_root, requested_result_path);
    write_analytical_result(&result_path, &result)?;
    let reloaded_result =
        load_conservative_ballistic_ridge_heldout_analytical_result_v1(&result_path)?;
    reloaded_result.validate_against_artifact(&reloaded)?;

    let (report_path, update_site) = report_path(repo_root, &output_dir)?;
    let report_value = serde_json::to_value(&reloaded)?;
    pd_report::conservative_ballistic_heldout::write_conservative_ballistic_heldout_report(
        &report_path,
        &report_value,
    )?;
    if let Some(site) = update_site {
        site.update_indexes_for_file(&report_path)?;
    }

    Ok(ConservativeBallisticRidgeHeldoutAnalyticalRunV1 {
        artifact: reloaded,
        result: reloaded_result,
        paths: ConservativeBallisticRidgeHeldoutAnalyticalPathsV1 {
            output_dir,
            summary_path,
            result_path,
            report_path,
        },
    })
}

pub fn load_conservative_ballistic_ridge_heldout_analytical_artifact_v1(
    path: &Path,
    input_manifest: &ExperimentalRidgeCaseManifestV1,
    predictions: &ConservativeBallisticRidgeHeldoutPredictionsV1,
) -> Result<ConservativeBallisticRidgeHeldoutAnalyticalArtifactV1> {
    let artifact: ConservativeBallisticRidgeHeldoutAnalyticalArtifactV1 =
        serde_json::from_slice(&fs::read(path).with_context(|| {
            format!(
                "failed to read held-out analytical summary {}",
                path.display()
            )
        })?)
        .with_context(|| {
            format!(
                "failed to parse held-out analytical summary {}",
                path.display()
            )
        })?;
    artifact.validate_against(input_manifest, predictions)?;
    Ok(artifact)
}

pub fn load_conservative_ballistic_ridge_heldout_analytical_result_v1(
    path: &Path,
) -> Result<ConservativeBallisticRidgeHeldoutAnalyticalResultV1> {
    let result: ConservativeBallisticRidgeHeldoutAnalyticalResultV1 =
        serde_json::from_slice(&fs::read(path).with_context(|| {
            format!(
                "failed to read held-out analytical result {}",
                path.display()
            )
        })?)
        .with_context(|| {
            format!(
                "failed to parse held-out analytical result {}",
                path.display()
            )
        })?;
    result.validate()?;
    Ok(result)
}

impl ConservativeBallisticRidgeHeldoutAnalyticalArtifactV1 {
    fn finalize_identity(&mut self) -> Result<()> {
        self.identity.clear();
        self.identity = canonical_digest(self).map_err(anyhow::Error::msg)?;
        Ok(())
    }

    pub fn validate_against(
        &self,
        input_manifest: &ExperimentalRidgeCaseManifestV1,
        predictions: &ConservativeBallisticRidgeHeldoutPredictionsV1,
    ) -> Result<()> {
        if self.schema_id != CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SCHEMA_ID_V1
            || self.schema_version
                != CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SCHEMA_VERSION_V1
            || self.setup_id != CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SETUP_ID_V1
        {
            bail!("held-out analytical artifact schema is invalid");
        }
        if !self.deterministic_repeat {
            bail!("held-out analytical artifact does not record a byte-identical repeat");
        }
        if self.input_manifest_identity != input_manifest.identity
            || self.prediction_manifest_identity != predictions.identity
        {
            bail!("held-out analytical artifact manifest identities do not match inputs");
        }
        if self.cases.len() > input_manifest.cases.len()
            || input_manifest.cases.len() != predictions.cases.len()
        {
            bail!("held-out analytical artifact case count does not match frozen manifests");
        }
        let mut ids = BTreeSet::new();
        for ((case, input), prediction) in self
            .cases
            .iter()
            .zip(&input_manifest.cases)
            .zip(&predictions.cases)
        {
            if !ids.insert(case.case_id.clone())
                || case.case_id != input.probe.id
                || case.case_id != prediction.case_id
            {
                bail!("held-out analytical artifact case order or IDs drifted");
            }
            let expected = build_case(input, prediction, &predictions.input_manifest_identity)?;
            if case != &expected {
                bail!(
                    "held-out analytical artifact case {} does not recompute",
                    case.case_id
                );
            }
        }
        match self.status {
            AnalyticalArtifactStatusV1::Passed => {
                if self.failure.is_some() || self.cases.len() != input_manifest.cases.len() {
                    bail!("passing held-out analytical artifact has incomplete cases");
                }
            }
            AnalyticalArtifactStatusV1::StoppedRuntimeProjectionError => {
                let failure = self.failure.as_ref().ok_or_else(|| {
                    anyhow!("stopped held-out analytical artifact has no failure")
                })?;
                let failed_index = self.cases.len();
                let input = input_manifest.cases.get(failed_index).ok_or_else(|| {
                    anyhow!("stopped held-out analytical artifact has no remaining frozen case")
                })?;
                let prediction = predictions.cases.get(failed_index).ok_or_else(|| {
                    anyhow!("stopped held-out analytical artifact has no remaining prediction")
                })?;
                let expected = build_runtime_projection_failure(
                    input,
                    prediction,
                    &predictions.input_manifest_identity,
                )?;
                if failure != &expected {
                    bail!("held-out analytical runtime-projection failure does not recompute");
                }
            }
        }
        let mut material = self.clone();
        material.identity.clear();
        if self.identity.is_empty()
            || self.identity != canonical_digest(&material).map_err(anyhow::Error::msg)?
        {
            bail!("held-out analytical artifact identity does not bind contents");
        }
        Ok(())
    }
}

impl ConservativeBallisticRidgeHeldoutAnalyticalResultV1 {
    pub fn from_artifact(
        artifact: &ConservativeBallisticRidgeHeldoutAnalyticalArtifactV1,
    ) -> Result<Self> {
        let failure = artifact
            .failure
            .as_ref()
            .ok_or_else(|| anyhow!("a passing H2b artifact has no stop result"))?;
        if artifact.status != AnalyticalArtifactStatusV1::StoppedRuntimeProjectionError {
            bail!("H2b result may only be created from a runtime-projection stop");
        }
        let mut result = Self {
            schema_id: CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_RESULT_SCHEMA_ID_V1
                .to_owned(),
            schema_version:
                CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_RESULT_SCHEMA_VERSION_V1,
            analytical_artifact_identity: artifact.identity.clone(),
            input_manifest_identity: artifact.input_manifest_identity.clone(),
            prediction_manifest_identity: artifact.prediction_manifest_identity.clone(),
            status: artifact.status.clone(),
            deterministic_repeat: artifact.deterministic_repeat,
            completed_cases: artifact
                .cases
                .iter()
                .map(ConservativeBallisticRidgeHeldoutAnalyticalResultCaseV1::from_case)
                .collect(),
            failure_case_id: failure.case_id.clone(),
            failure_input_identity: failure.input_identity.clone(),
            failure_prediction_identity: failure.prediction_identity.clone(),
            failure_analytical_canary_identity: failure.analytical_canary_identity.clone(),
            failure_candidate_projection_identity: failure.candidate_projection_identity.clone(),
            failure_candidate_expectation: failure.candidate_expectation.clone(),
            failure_stage: failure.stage.clone(),
            failure_runtime_error: failure.runtime_error.clone(),
            failure_error: failure.error.clone(),
            failure_identity: failure.identity.clone(),
            identity: String::new(),
        };
        result.identity = canonical_digest(&result).map_err(anyhow::Error::msg)?;
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema_id != CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_RESULT_SCHEMA_ID_V1
            || self.schema_version
                != CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_RESULT_SCHEMA_VERSION_V1
            || self.status != AnalyticalArtifactStatusV1::StoppedRuntimeProjectionError
            || !self.deterministic_repeat
            || self.analytical_artifact_identity.is_empty()
            || self.input_manifest_identity.is_empty()
            || self.prediction_manifest_identity.is_empty()
            || self.failure_case_id.is_empty()
            || self.failure_input_identity.is_empty()
            || self.failure_prediction_identity.is_empty()
            || self.failure_analytical_canary_identity.is_empty()
            || self.failure_candidate_projection_identity.is_empty()
            || !self.failure_candidate_expectation.passed()
            || self.failure_stage != "runtime_projection"
            || self.failure_runtime_error
                != ExperimentalRidgeRuntimeProjectionErrorV2::HandoffContractFailed
            || self.failure_error.is_empty()
            || self.failure_identity.is_empty()
        {
            bail!("held-out analytical result schema or required fields are invalid");
        }
        let mut seen = BTreeSet::new();
        for case in &self.completed_cases {
            if case.case_id.is_empty()
                || case.input_identity.is_empty()
                || case.prediction_identity.is_empty()
                || case.analytical_canary_identity.is_empty()
                || case.candidate_projection_identity.is_empty()
                || case.runtime_projection_identity.is_empty()
                || case.flat_selected_candidate_identity.is_empty()
                || case.derived_nominal_candidate_identity.is_empty()
                || case.selected_waypoint_candidate_identity.is_empty()
                || case.flat_runtime_outcome_identity.is_empty()
                || case.derived_runtime_outcome_identity.is_empty()
                || case.case_identity.is_empty()
                || !case.expectation.passed()
                || !seen.insert(case.case_id.clone())
            {
                bail!("held-out analytical result case fields are invalid");
            }
        }
        if !seen.insert(self.failure_case_id.clone()) {
            bail!("held-out analytical result repeats the failed case ID");
        }
        let mut material = self.clone();
        material.identity.clear();
        if self.identity.is_empty()
            || self.identity != canonical_digest(&material).map_err(anyhow::Error::msg)?
        {
            bail!("held-out analytical result identity does not bind contents");
        }
        Ok(())
    }

    pub fn validate_against_artifact(
        &self,
        artifact: &ConservativeBallisticRidgeHeldoutAnalyticalArtifactV1,
    ) -> Result<()> {
        self.validate()?;
        if self.analytical_artifact_identity != artifact.identity
            || self.input_manifest_identity != artifact.input_manifest_identity
            || self.prediction_manifest_identity != artifact.prediction_manifest_identity
            || self.deterministic_repeat != artifact.deterministic_repeat
        {
            bail!("held-out analytical result root join does not match artifact");
        }
        let expected = Self::from_artifact(artifact)?;
        if self != &expected {
            bail!("held-out analytical result does not match artifact");
        }
        Ok(())
    }
}

impl ConservativeBallisticRidgeHeldoutAnalyticalResultCaseV1 {
    fn from_case(case: &ConservativeBallisticRidgeHeldoutAnalyticalCaseV1) -> Self {
        Self {
            case_id: case.case_id.clone(),
            input_identity: case.input_identity.clone(),
            prediction_identity: case.prediction_identity.clone(),
            analytical_canary_identity: case.analytical_canary_identity.clone(),
            candidate_projection_identity: case.candidate_projection_identity.clone(),
            runtime_projection_identity: case.runtime_projection_identity.clone(),
            flat_selected_candidate_identity: case.flat_selected_candidate_identity.clone(),
            derived_nominal_candidate_identity: case.derived_nominal_candidate_identity.clone(),
            non_nominal_direct_candidate_identities: case
                .non_nominal_direct_candidate_identities
                .clone(),
            selected_waypoint_candidate_identity: case.selected_waypoint_candidate_identity.clone(),
            flat_runtime_outcome_identity: case.flat_runtime_outcome_identity.clone(),
            derived_runtime_outcome_identity: case.derived_runtime_outcome_identity.clone(),
            expectation: case.expectation.clone(),
            case_identity: case.identity.clone(),
        }
    }
}

fn build_artifact(
    input_manifest: &ExperimentalRidgeCaseManifestV1,
    predictions: &ConservativeBallisticRidgeHeldoutPredictionsV1,
) -> Result<ConservativeBallisticRidgeHeldoutAnalyticalArtifactV1> {
    let mut cases = Vec::new();
    for (input, prediction) in input_manifest.cases.iter().zip(&predictions.cases) {
        match build_case(input, prediction, &predictions.input_manifest_identity) {
            Ok(case) => cases.push(case),
            Err(error) => {
                let failure = build_runtime_projection_failure(
                    input,
                    prediction,
                    &predictions.input_manifest_identity,
                )
                .with_context(|| {
                    format!(
                        "{} did not produce a recordable runtime-projection failure after: {error}",
                        input.probe.id
                    )
                })?;
                return Ok(ConservativeBallisticRidgeHeldoutAnalyticalArtifactV1 {
                    schema_id: CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SCHEMA_ID_V1
                        .to_owned(),
                    schema_version:
                        CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SCHEMA_VERSION_V1,
                    setup_id: CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SETUP_ID_V1
                        .to_owned(),
                    input_manifest_identity: input_manifest.identity.clone(),
                    prediction_manifest_identity: predictions.identity.clone(),
                    status: AnalyticalArtifactStatusV1::StoppedRuntimeProjectionError,
                    cases,
                    failure: Some(failure),
                    deterministic_repeat: false,
                    identity: String::new(),
                });
            }
        }
    }
    Ok(ConservativeBallisticRidgeHeldoutAnalyticalArtifactV1 {
        schema_id: CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SCHEMA_ID_V1.to_owned(),
        schema_version: CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SCHEMA_VERSION_V1,
        setup_id: CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SETUP_ID_V1.to_owned(),
        input_manifest_identity: input_manifest.identity.clone(),
        prediction_manifest_identity: predictions.identity.clone(),
        status: AnalyticalArtifactStatusV1::Passed,
        cases,
        failure: None,
        deterministic_repeat: false,
        identity: String::new(),
    })
}

fn build_case(
    input: &ExperimentalRidgeCaseInputV1,
    prediction: &ConservativeBallisticRidgeHeldoutPredictionV1,
    input_manifest_identity: &str,
) -> Result<ConservativeBallisticRidgeHeldoutAnalyticalCaseV1> {
    let projection = evaluate_experimental_ridge_case_projection_v1(input)
        .map_err(|error| anyhow!("{} candidate projection failed: {error}", input.probe.id))?;
    validate_experimental_ridge_case_projection_v1(&projection).map_err(|error| {
        anyhow!(
            "{} candidate projection failed validation: {error}",
            input.probe.id
        )
    })?;
    let runtime = project_experimental_ridge_case_runtime_v1(&projection)
        .map_err(|error| anyhow!("{} runtime projection failed: {error}", input.probe.id))?;
    validate_experimental_ridge_case_runtime_v1(&projection, &runtime).map_err(|error| {
        anyhow!(
            "{} runtime projection failed validation: {error}",
            input.probe.id
        )
    })?;

    let (
        flat_selected_candidate_identity,
        selected_waypoint_candidate_identity,
        analytical_waypoint_position_m,
        source_arc_points,
        target_arc_points,
    ) = match (&projection.flat_control, &projection.derived_mesa) {
        (
            ExperimentalRidgeCandidateOutcomeV2::Direct { candidate: flat },
            ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. },
        ) => (
            flat.identity.clone(),
            candidate.identity.clone(),
            candidate.waypoint_position_m,
            sampled_arc_points(&candidate.source_leg),
            sampled_arc_points(&candidate.target_leg),
        ),
        (flat, mesa) => bail!(
            "{} has unsupported or unexpected analytical outcomes: flat={flat:?}, mesa={mesa:?}",
            input.probe.id
        ),
    };
    let nominal_direct_arc_points_m = match &projection.flat_control {
        ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => {
            sampled_arc_points(&candidate.virtual_arc)
        }
        _ => unreachable!("checked above"),
    };
    let (
        flat_runtime_outcome_identity,
        derived_runtime_outcome_identity,
        runtime_waypoint_position_m,
    ) = runtime_outcome_identities_and_waypoint(&runtime)?;
    let expectation = compare_prediction(input, prediction, &projection, &runtime)?;
    if !expectation.passed() {
        bail!("{} analytical prediction comparison failed", input.probe.id);
    }
    let mut case = ConservativeBallisticRidgeHeldoutAnalyticalCaseV1 {
        case_id: input.probe.id.clone(),
        input_identity: input.identity.clone(),
        prediction_identity: prediction_identity(input_manifest_identity, prediction)?,
        analytical_canary_identity: projection.analytical_canary_identity.clone(),
        candidate_projection_identity: projection.identity.clone(),
        runtime_projection_identity: runtime.identity.clone(),
        flat_selected_candidate_identity,
        derived_nominal_candidate_identity: projection
            .derived_nominal_direct
            .candidate_identity
            .clone(),
        derived_nominal_rejection_reasons: projection.derived_nominal_direct.reasons.clone(),
        non_nominal_direct_candidate_identities: projection
            .derived_non_nominal_direct_diagnostics
            .iter()
            .map(|candidate| candidate.candidate_identity.clone())
            .collect(),
        non_nominal_direct_certified_count: projection
            .derived_non_nominal_direct_diagnostics
            .iter()
            .filter(|candidate| candidate.classification == CertificationV2::Certified)
            .count(),
        waypoint_search_identity: projection.waypoint_search_identity.clone(),
        selected_waypoint_candidate_identity,
        flat_runtime_outcome_identity,
        derived_runtime_outcome_identity,
        expectation,
        visual: HeldoutAnalyticalVisualV1 {
            flat_twin_terrain_points_m: flat_twin_terrain(input),
            derived_mesa_terrain_points_m: projection.mesa.terrain_points_m.clone(),
            nominal_direct_arc_points_m,
            selected_waypoint_source_arc_points_m: source_arc_points,
            selected_waypoint_target_arc_points_m: target_arc_points,
            analytical_waypoint_position_m,
            runtime_waypoint_position_m: Some(runtime_waypoint_position_m),
        },
        candidate_projection: projection,
        runtime_projection: runtime,
        identity: String::new(),
    };
    case.identity = canonical_digest(&case).map_err(anyhow::Error::msg)?;
    Ok(case)
}

fn build_runtime_projection_failure(
    input: &ExperimentalRidgeCaseInputV1,
    prediction: &ConservativeBallisticRidgeHeldoutPredictionV1,
    input_manifest_identity: &str,
) -> Result<ConservativeBallisticRidgeHeldoutAnalyticalFailureV1> {
    let projection = evaluate_experimental_ridge_case_projection_v1(input)
        .map_err(|error| anyhow!("{} candidate projection failed: {error}", input.probe.id))?;
    validate_experimental_ridge_case_projection_v1(&projection).map_err(|error| {
        anyhow!(
            "{} candidate projection failed validation: {error}",
            input.probe.id
        )
    })?;
    let candidate_expectation = compare_candidate_prediction(input, prediction, &projection)?;
    if !candidate_expectation.passed() {
        bail!(
            "{} candidate projection does not match the frozen analytical prediction",
            input.probe.id
        );
    }
    let runtime_error = match project_experimental_ridge_case_runtime_v1(&projection) {
        Ok(_) => bail!(
            "{} did not reproduce a runtime-projection failure",
            input.probe.id
        ),
        Err(error) => error,
    };
    let mut failure = ConservativeBallisticRidgeHeldoutAnalyticalFailureV1 {
        case_id: input.probe.id.clone(),
        input_identity: input.identity.clone(),
        prediction_identity: prediction_identity(input_manifest_identity, prediction)?,
        analytical_canary_identity: projection.analytical_canary_identity.clone(),
        candidate_projection_identity: projection.identity.clone(),
        candidate_expectation,
        stage: "runtime_projection".to_owned(),
        error: runtime_error.to_string(),
        runtime_error,
        visual: visual_from_projection(input, &projection, None)?,
        candidate_projection: projection,
        identity: String::new(),
    };
    failure.identity = canonical_digest(&failure).map_err(anyhow::Error::msg)?;
    Ok(failure)
}

fn visual_from_projection(
    input: &ExperimentalRidgeCaseInputV1,
    projection: &ExperimentalRidgeCaseProjectionV1,
    runtime_waypoint_position_m: Option<Vec2>,
) -> Result<HeldoutAnalyticalVisualV1> {
    let nominal_direct_arc_points_m = match &projection.flat_control {
        ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => {
            sampled_arc_points(&candidate.virtual_arc)
        }
        outcome => bail!("flat analytical visual has no direct candidate: {outcome:?}"),
    };
    let (
        selected_waypoint_source_arc_points_m,
        selected_waypoint_target_arc_points_m,
        analytical_waypoint_position_m,
    ) = match &projection.derived_mesa {
        ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. } => (
            sampled_arc_points(&candidate.source_leg),
            sampled_arc_points(&candidate.target_leg),
            candidate.waypoint_position_m,
        ),
        outcome => bail!("derived analytical visual has no waypoint candidate: {outcome:?}"),
    };
    Ok(HeldoutAnalyticalVisualV1 {
        flat_twin_terrain_points_m: flat_twin_terrain(input),
        derived_mesa_terrain_points_m: projection.mesa.terrain_points_m.clone(),
        nominal_direct_arc_points_m,
        selected_waypoint_source_arc_points_m,
        selected_waypoint_target_arc_points_m,
        analytical_waypoint_position_m,
        runtime_waypoint_position_m,
    })
}

fn compare_candidate_prediction(
    input: &ExperimentalRidgeCaseInputV1,
    prediction: &ConservativeBallisticRidgeHeldoutPredictionV1,
    projection: &ExperimentalRidgeCaseProjectionV1,
) -> Result<CandidatePredictionComparisonV1> {
    if prediction.case_id != input.probe.id || prediction.input_identity != input.identity {
        bail!(
            "{} prediction does not join the frozen input",
            input.probe.id
        );
    }
    let flat = match &projection.flat_control {
        ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => candidate,
        ExperimentalRidgeCandidateOutcomeV2::Unsupported { reason, .. } => {
            bail!(
                "{} flat analytical result is unsupported: {reason:?}",
                input.probe.id
            )
        }
        outcome => bail!(
            "{} flat analytical result is not direct: {outcome:?}",
            input.probe.id
        ),
    };
    let waypoint = match &projection.derived_mesa {
        ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
            candidate,
            crossing,
        } => {
            if crossing.candidate_identity != candidate.identity
                || !crossing.strict_directed_crossing
            {
                bail!("{} waypoint crossing evidence is invalid", input.probe.id);
            }
            candidate
        }
        ExperimentalRidgeCandidateOutcomeV2::Unsupported { reason, .. } => {
            bail!(
                "{} derived analytical result is unsupported: {reason:?}",
                input.probe.id
            )
        }
        outcome => bail!(
            "{} derived analytical result is not one_waypoint: {outcome:?}",
            input.probe.id
        ),
    };
    let comparison = CandidatePredictionComparisonV1 {
        flat_control_direct: prediction.analytical.flat_control.expected_outcome
            == ExpectedAnalyticalRouteOutcomeV1::Direct,
        // `flat_control` is the generic H1 recomputed nominal selection.  The
        // all-candidate identity list also contains earlier rejected rows.
        flat_shortest_robust_certified: prediction.analytical.flat_control.selection_rule
            == FlatControlSelectionRuleV1::ShortestRobustCertified
            && flat.classification == CertificationV2::Certified
            && projection
                .flat_candidate_identities
                .contains(&flat.identity),
        flat_duration_multiplier_matches: flat.duration_multiplier
            == prediction.analytical.flat_control.duration_multiplier,
        derived_blocker_valid: prediction.analytical.derived_mesa.blocker
            == crate::DerivedBlockerExpectationV1::Valid
            && projection.derived_blocker_valid,
        derived_nominal_terrain_clearance_rejected: prediction
            .analytical
            .derived_mesa
            .nominal_direct_rejection
            == NominalDirectRejectionV1::TerrainClearance
            && projection.derived_nominal_direct.classification != CertificationV2::Certified
            && projection
                .derived_nominal_direct
                .reasons
                .contains(&DirectBridgeReasonV2::TerrainClearance)
            && projection.derived_nominal_direct.duration_multiplier == flat.duration_multiplier
            && projection
                .ridge_direct_candidate_identities
                .contains(&projection.derived_nominal_direct.candidate_identity),
        longer_global_replan_certified: prediction.analytical.derived_mesa.longer_global_replan
            == LongerGlobalReplanExpectationV1::AtLeastOneCertifiedDiagnostic
            && projection
                .derived_non_nominal_direct_diagnostics
                .iter()
                .any(|candidate| {
                    candidate.duration_multiplier > flat.duration_multiplier
                        && candidate.classification == CertificationV2::Certified
                })
            && projection
                .ridge_direct_candidate_identities
                .contains(&projection.derived_nominal_direct.candidate_identity)
            && projection.ridge_direct_candidate_identities.len()
                == projection.derived_non_nominal_direct_diagnostics.len() + 1
            && projection
                .derived_non_nominal_direct_diagnostics
                .iter()
                .all(|candidate| {
                    projection
                        .ridge_direct_candidate_identities
                        .contains(&candidate.candidate_identity)
                }),
        derived_mesa_one_waypoint: prediction.analytical.derived_mesa.expected_outcome
            == ExpectedAnalyticalRouteOutcomeV1::OneWaypoint
            && waypoint.classification == CertificationV2::Certified,
        exactly_one_certified_forward_progressing_waypoint: prediction
            .analytical
            .derived_mesa
            .selected_waypoint_witness
            == SelectedWaypointWitnessExpectationV1::ExactlyOneCertifiedForwardProgressing
            && projection.waypoint_certified_candidate_count == 1
            && projection
                .waypoint_candidate_identities
                .contains(&waypoint.identity)
            && waypoint
                .route_progress
                .as_ref()
                .is_some_and(|progress| progress.passes),
        candidate_projection_validated: prediction.analytical.validation.candidate_projection
            == ProjectionValidationRequirementV1::Required,
    };
    if !comparison.passed() {
        bail!(
            "{} candidate projection does not match the frozen analytical prediction",
            input.probe.id
        );
    }
    Ok(comparison)
}

fn compare_prediction(
    input: &ExperimentalRidgeCaseInputV1,
    prediction: &ConservativeBallisticRidgeHeldoutPredictionV1,
    projection: &ExperimentalRidgeCaseProjectionV1,
    runtime: &ExperimentalRidgeCaseRuntimeProjectionV1,
) -> Result<AnalyticalExpectationComparisonV1> {
    let _candidate_comparison = compare_candidate_prediction(input, prediction, projection)?;
    if prediction.case_id != input.probe.id || prediction.input_identity != input.identity {
        bail!(
            "{} prediction does not join the frozen input",
            input.probe.id
        );
    }
    let flat = match &projection.flat_control {
        ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => candidate,
        ExperimentalRidgeCandidateOutcomeV2::Unsupported { reason, .. } => {
            bail!(
                "{} flat analytical result is unsupported: {reason:?}",
                input.probe.id
            )
        }
        outcome => bail!(
            "{} flat analytical result is not direct: {outcome:?}",
            input.probe.id
        ),
    };
    let waypoint = match &projection.derived_mesa {
        ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
            candidate,
            crossing,
        } => {
            if crossing.candidate_identity != candidate.identity
                || !crossing.strict_directed_crossing
            {
                bail!("{} waypoint crossing evidence is invalid", input.probe.id);
            }
            candidate
        }
        ExperimentalRidgeCandidateOutcomeV2::Unsupported { reason, .. } => {
            bail!(
                "{} derived analytical result is unsupported: {reason:?}",
                input.probe.id
            )
        }
        outcome => bail!(
            "{} derived analytical result is not one_waypoint: {outcome:?}",
            input.probe.id
        ),
    };
    let flat_runtime = match &runtime.flat_control {
        ExperimentalRidgeRuntimeOutcomeV2::Direct {
            candidate_identity,
            structural_validation,
            ..
        } if candidate_identity == &flat.identity && structural_validation.transfer_route_valid => {
            true
        }
        ExperimentalRidgeRuntimeOutcomeV2::Unsupported { reason, .. } => {
            bail!(
                "{} flat runtime result is unsupported: {reason:?}",
                input.probe.id
            )
        }
        _ => false,
    };
    let derived_runtime = match &runtime.derived_mesa {
        ExperimentalRidgeRuntimeOutcomeV2::OneWaypoint {
            candidate_identity,
            handoff_assessment,
            structural_validation,
            ..
        } => {
            candidate_identity == &waypoint.identity
                && handoff_assessment.contract_pass
                && structural_validation.transfer_route_valid
        }
        ExperimentalRidgeRuntimeOutcomeV2::Unsupported { reason, .. } => {
            bail!(
                "{} derived runtime result is unsupported: {reason:?}",
                input.probe.id
            )
        }
        _ => false,
    };
    let flat_control_direct = prediction.analytical.flat_control.expected_outcome
        == ExpectedAnalyticalRouteOutcomeV1::Direct;
    let flat_shortest_robust_certified = prediction.analytical.flat_control.selection_rule
        == FlatControlSelectionRuleV1::ShortestRobustCertified
        // `flat_control` is the generic H1 recomputed nominal selection.  The
        // identity list also contains earlier rejected candidates, so its
        // first entry must not be confused with the selected certificate.
        && flat.classification == CertificationV2::Certified
        && projection.flat_candidate_identities.contains(&flat.identity);
    let flat_duration_multiplier_matches =
        flat.duration_multiplier == prediction.analytical.flat_control.duration_multiplier;
    let derived_blocker_valid = prediction.analytical.derived_mesa.blocker
        == crate::DerivedBlockerExpectationV1::Valid
        && projection.derived_blocker_valid;
    let derived_nominal_terrain_clearance_rejected =
        prediction.analytical.derived_mesa.nominal_direct_rejection
            == NominalDirectRejectionV1::TerrainClearance
            && projection.derived_nominal_direct.classification != CertificationV2::Certified
            && projection
                .derived_nominal_direct
                .reasons
                .contains(&DirectBridgeReasonV2::TerrainClearance)
            && projection.derived_nominal_direct.duration_multiplier == flat.duration_multiplier
            && projection
                .ridge_direct_candidate_identities
                .contains(&projection.derived_nominal_direct.candidate_identity);
    let longer_global_replan_certified = prediction.analytical.derived_mesa.longer_global_replan
        == LongerGlobalReplanExpectationV1::AtLeastOneCertifiedDiagnostic
        && projection
            .derived_non_nominal_direct_diagnostics
            .iter()
            .any(|candidate| {
                candidate.duration_multiplier > flat.duration_multiplier
                    && candidate.classification == CertificationV2::Certified
            })
        && projection
            .ridge_direct_candidate_identities
            .contains(&projection.derived_nominal_direct.candidate_identity)
        && projection.ridge_direct_candidate_identities.len()
            == projection.derived_non_nominal_direct_diagnostics.len() + 1
        && projection
            .derived_non_nominal_direct_diagnostics
            .iter()
            .all(|candidate| {
                projection
                    .ridge_direct_candidate_identities
                    .contains(&candidate.candidate_identity)
            });
    let derived_mesa_one_waypoint = prediction.analytical.derived_mesa.expected_outcome
        == ExpectedAnalyticalRouteOutcomeV1::OneWaypoint
        && waypoint.classification == CertificationV2::Certified;
    let exactly_one_certified_forward_progressing_waypoint =
        prediction.analytical.derived_mesa.selected_waypoint_witness
            == SelectedWaypointWitnessExpectationV1::ExactlyOneCertifiedForwardProgressing
            && projection.waypoint_certified_candidate_count == 1
            && projection
                .waypoint_candidate_identities
                .contains(&waypoint.identity)
            && waypoint
                .route_progress
                .as_ref()
                .is_some_and(|progress| progress.passes);
    let candidate_projection_validated = prediction.analytical.validation.candidate_projection
        == ProjectionValidationRequirementV1::Required;
    let runtime_projection_validated = prediction.analytical.validation.runtime_projection
        == ProjectionValidationRequirementV1::Required
        && flat_runtime
        && derived_runtime;
    let comparison = AnalyticalExpectationComparisonV1 {
        status: AnalyticalExpectationStatusV1::Passed,
        flat_control_direct,
        flat_shortest_robust_certified,
        flat_duration_multiplier_matches,
        derived_blocker_valid,
        derived_nominal_terrain_clearance_rejected,
        longer_global_replan_certified,
        derived_mesa_one_waypoint,
        exactly_one_certified_forward_progressing_waypoint,
        candidate_projection_validated,
        runtime_projection_validated,
    };
    if !comparison.passed() {
        bail!(
            "{} does not match the frozen analytical prediction",
            input.probe.id
        );
    }
    Ok(comparison)
}

fn runtime_outcome_identities_and_waypoint(
    runtime: &ExperimentalRidgeCaseRuntimeProjectionV1,
) -> Result<(String, String, Vec2)> {
    let flat_identity = match &runtime.flat_control {
        ExperimentalRidgeRuntimeOutcomeV2::Direct { identity, .. } => identity.clone(),
        outcome => bail!("flat runtime outcome is not direct: {outcome:?}"),
    };
    let (derived_identity, waypoint_position_m) = match &runtime.derived_mesa {
        ExperimentalRidgeRuntimeOutcomeV2::OneWaypoint {
            identity, route, ..
        } => (
            identity.clone(),
            route
                .waypoints
                .first()
                .ok_or_else(|| anyhow!("one-waypoint runtime route has no waypoint"))?
                .position_m,
        ),
        outcome => bail!("derived runtime outcome is not one_waypoint: {outcome:?}"),
    };
    Ok((flat_identity, derived_identity, waypoint_position_m))
}

fn prediction_identity(
    input_manifest_identity: &str,
    prediction: &ConservativeBallisticRidgeHeldoutPredictionV1,
) -> Result<String> {
    canonical_digest(&PredictionIdentityMaterialV1 {
        input_manifest_identity,
        prediction,
    })
    .map_err(anyhow::Error::msg)
}

#[derive(Serialize)]
struct PredictionIdentityMaterialV1<'a> {
    input_manifest_identity: &'a str,
    prediction: &'a ConservativeBallisticRidgeHeldoutPredictionV1,
}

fn flat_twin_terrain(input: &ExperimentalRidgeCaseInputV1) -> Vec<Vec2> {
    let first = input
        .probe
        .terrain_points_m
        .first()
        .expect("validated held-out probe has terrain start");
    let last = input
        .probe
        .terrain_points_m
        .last()
        .expect("validated held-out probe has terrain end");
    vec![Vec2::new(first.x, first.y), Vec2::new(last.x, last.y)]
}

fn sampled_arc_points(
    arc: &pd_plan::conservative_ballistic_bridge::VirtualBallisticArcV2,
) -> Vec<Vec2> {
    const DISPLAY_SEGMENTS: u64 = 80;
    (0..=DISPLAY_SEGMENTS)
        .map(|index| {
            arc.state_at(arc.steps * index / DISPLAY_SEGMENTS)
                .position_m
        })
        .collect()
}

fn resolve_output_dir(repo_root: &Path, requested: Option<&Path>) -> PathBuf {
    requested
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                repo_root.join(path)
            }
        })
        .unwrap_or_else(|| {
            repo_root
                .join("outputs/eval")
                .join(CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_SETUP_ID_V1)
        })
}

fn resolve_result_path(repo_root: &Path, requested: Option<&Path>) -> PathBuf {
    requested
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                repo_root.join(path)
            }
        })
        .unwrap_or_else(|| {
            repo_root.join(CONSERVATIVE_BALLISTIC_RIDGE_HELDOUT_ANALYTICAL_RESULT_FIXTURE_V1)
        })
}

fn report_path(repo_root: &Path, output_dir: &Path) -> Result<(PathBuf, Option<ReportSite>)> {
    let repo_outputs = repo_root.join("outputs");
    let repo_output_path = fs::canonicalize(&repo_outputs)
        .ok()
        .zip(fs::canonicalize(output_dir).ok())
        .is_some_and(|(root, output)| output.starts_with(root));
    if repo_output_path {
        let site = ReportSite::new(repo_root);
        let report_path = site.default_output_for_bundle(output_dir).ok_or_else(|| {
            anyhow!("held-out analytical output must be under repository outputs")
        })?;
        Ok((report_path, Some(site)))
    } else {
        Ok((output_dir.join("report/index.html"), None))
    }
}

fn write_analytical_result(
    path: &Path,
    result: &ConservativeBallisticRidgeHeldoutAnalyticalResultV1,
) -> Result<()> {
    result.validate()?;
    let bytes = serde_json::to_vec_pretty(result)?;
    if path.exists() {
        let existing = fs::read(path).with_context(|| {
            format!(
                "failed to read existing held-out analytical result {}",
                path.display()
            )
        })?;
        let reloaded: ConservativeBallisticRidgeHeldoutAnalyticalResultV1 =
            serde_json::from_slice(&existing).with_context(|| {
                format!(
                    "failed to parse existing held-out analytical result {}",
                    path.display()
                )
            })?;
        reloaded.validate()?;
        if reloaded != *result || existing != bytes {
            bail!(
                "existing held-out analytical result {} does not exactly match fresh evidence",
                path.display()
            );
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create held-out analytical result directory {}",
                parent.display()
            )
        })?;
    }
    fs::write(path, bytes).with_context(|| {
        format!(
            "failed to write held-out analytical result {}",
            path.display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("pd-eval-heldout-analytical-{label}-{nonce}"))
    }

    fn manifests() -> (
        ExperimentalRidgeCaseManifestV1,
        ConservativeBallisticRidgeHeldoutPredictionsV1,
    ) {
        (
            pd_plan::conservative_ballistic_bridge::load_experimental_ridge_case_manifest_v1(),
            load_conservative_ballistic_ridge_heldout_predictions_v1().unwrap(),
        )
    }

    #[test]
    fn analytical_stop_recomputes_byte_identically_and_records_runtime_error() {
        let (inputs, predictions) = manifests();
        let mut first = build_artifact(&inputs, &predictions).unwrap();
        let mut second = build_artifact(&inputs, &predictions).unwrap();
        first.deterministic_repeat = true;
        second.deterministic_repeat = true;
        first.finalize_identity().unwrap();
        second.finalize_identity().unwrap();
        assert_eq!(
            serde_json::to_vec_pretty(&first).unwrap(),
            serde_json::to_vec_pretty(&second).unwrap()
        );
        first.validate_against(&inputs, &predictions).unwrap();
        assert_eq!(
            first.status,
            AnalyticalArtifactStatusV1::StoppedRuntimeProjectionError
        );
        assert_eq!(first.cases.len(), 1);
        let failure = first.failure.as_ref().unwrap();
        assert_eq!(failure.case_id, "ridge_progress_068_probe");
        assert_eq!(failure.stage, "runtime_projection");
        assert_eq!(
            failure.runtime_error,
            ExperimentalRidgeRuntimeProjectionErrorV2::HandoffContractFailed
        );
        assert_eq!(
            failure.error,
            "waypoint handoff does not satisfy the canonical contract"
        );
        let result =
            ConservativeBallisticRidgeHeldoutAnalyticalResultV1::from_artifact(&first).unwrap();
        result.validate_against_artifact(&first).unwrap();
    }

    #[test]
    fn analytical_stop_rejects_tampered_failure_and_result_join() {
        let (inputs, predictions) = manifests();
        let mut artifact = build_artifact(&inputs, &predictions).unwrap();
        artifact.deterministic_repeat = true;
        artifact.finalize_identity().unwrap();
        artifact
            .failure
            .as_mut()
            .unwrap()
            .error
            .push_str("-tampered");
        artifact.finalize_identity().unwrap();
        assert!(artifact.validate_against(&inputs, &predictions).is_err());

        let mut valid = build_artifact(&inputs, &predictions).unwrap();
        valid.deterministic_repeat = true;
        valid.finalize_identity().unwrap();
        let mut result =
            ConservativeBallisticRidgeHeldoutAnalyticalResultV1::from_artifact(&valid).unwrap();
        result
            .failure_candidate_projection_identity
            .push_str("-tampered");
        result.identity.clear();
        result.identity = canonical_digest(&result).unwrap();
        assert!(result.validate_against_artifact(&valid).is_err());
    }

    #[test]
    fn analytical_run_reloads_summary_result_and_display_report() {
        let root = temp_root("output");
        let output = root.join("output");
        let result = root.join("result.json");
        let run = run_conservative_ballistic_ridge_heldout_analytical_v1(
            &root,
            Some(&output),
            Some(&result),
        )
        .unwrap();
        assert!(run.paths.summary_path.is_file());
        assert!(run.paths.result_path.is_file());
        assert!(run.paths.report_path.is_file());
        assert!(run.artifact.deterministic_repeat);
        assert_eq!(run.artifact.cases.len(), 1);
        assert_eq!(
            run.result.status,
            AnalyticalArtifactStatusV1::StoppedRuntimeProjectionError
        );
        let report = fs::read_to_string(&run.paths.report_path).unwrap();
        assert!(report.contains("fail-closed analytical stop"));
        assert!(report.contains("waypoint handoff does not satisfy the canonical contract"));
        let mut result_bytes = fs::read(&run.paths.result_path).unwrap();
        result_bytes.push(b'\n');
        fs::write(&run.paths.result_path, result_bytes).unwrap();
        assert!(write_analytical_result(&run.paths.result_path, &run.result).is_err());
        let _ = fs::remove_dir_all(root);
    }
}
