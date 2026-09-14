//! F5 held-out controller reveal boundary.
//!
//! This module is deliberately downstream of the F5 source seals and the
//! committed F5c analytical result. It materializes only cases that F5c has
//! already marked controller-eligible into the frozen simulator/controller
//! seam. It neither alters the planner/controller nor feeds simulator results
//! back into either one.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_control::{ControllerSpec, built_in_controller_spec};
use pd_core::{
    EndReason, EvaluationGoal, LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig,
    TerrainDefinition, TransferRouteSpec, Vec2, VehicleGeometry, VehicleInitialState, VehicleSpec,
    WorldSpec,
};
use pd_plan::conservative_ballistic_bridge::{
    DirectBridgePolicyV2, DirectBridgeProbeV2, ExperimentalRidgeCandidateOutcomeV2,
    ExperimentalRidgeCaseInputV1, ExperimentalRidgeCaseProjectionV1,
    ExperimentalRidgeCaseRuntimeOutcomeV2, ExperimentalRidgeCaseRuntimeProjectionV2,
    ExperimentalRidgeRuntimeHandoffAttemptV2, ExperimentalRidgeRuntimeHandoffSelectionKindV2,
    MesaGeometryV2, VehicleInputV2, evaluate_experimental_ridge_case_projection_v1,
    project_experimental_ridge_case_runtime_v2, validate_experimental_ridge_case_projection_v1,
    validate_experimental_ridge_case_runtime_v2,
};
use pd_report::site::ReportSite;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    ConservativeBallisticRidgeF5AnalyticalResultCaseV1,
    ConservativeBallisticRidgeF5AnalyticalResultV1, ConservativeBallisticRidgeF5InputsV1,
    ConservativeBallisticRidgeF5PredictionV1, ConservativeBallisticRidgeF5PredictionsV1,
    F5AnalyticalCaseStatusV1, F5ControllerExpectationsV1, F5ControllerLandingExpectationV1,
    F5ControllerTerrainContactExpectationV1, F5TerrainContactLocationExpectationV1,
    F5TerrainContactOrderingExpectationV1, F5WaypointCaptureExpectationV1, canonical_digest,
    controller_shadow::{
        AnalyticalShadowOverlay, ShadowLaneClass, ShadowLaneSummary, build_analytical_overlay,
        run_lane, semantic_lane,
    },
    load_conservative_ballistic_ridge_f5_analytical_result_v1,
    load_conservative_ballistic_ridge_f5_inputs_v1,
    load_conservative_ballistic_ridge_f5_predictions_v1,
};

pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_SCHEMA_ID_V1: &str =
    "conservative_ballistic_ridge_f5_controller_v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_SCHEMA_VERSION_V1: u32 = 1;
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_SETUP_ID_V1: &str =
    "conservative-ballistic-ridge-f5-controller-v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_RESULT_SCHEMA_ID_V1: &str =
    "conservative_ballistic_ridge_f5_controller_result_v1";
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_RESULT_FIXTURE_V1: &str =
    "fixtures/manifests/conservative_ballistic_ridge_f5_controller_result_v1.json";
pub const CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_IDENTITY_V1: &str = "6ebab8240feaea3e";

const F5_EXPECTED_ELIGIBLE_CASE_ID_V1: &str = "ridge_progress_056_probe";
const F5_EXPECTED_INELIGIBLE_CASE_ID_V1: &str = "ridge_progress_072_probe";

/// Explicitly bound scope for both the detailed and compact F5d evidence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5ControllerScopeV1 {
    pub controller_run: bool,
    pub simulation_run: bool,
    pub physical_execution_run: bool,
    pub non_claims: Vec<String>,
}

fn controller_scope() -> ConservativeBallisticRidgeF5ControllerScopeV1 {
    ConservativeBallisticRidgeF5ControllerScopeV1 {
        controller_run: true,
        simulation_run: true,
        physical_execution_run: false,
        non_claims: vec![
            "F5 controller lanes run only after the committed analytical eligibility join".to_owned(),
            "analytically stopped cases have no controller scenario, lane, or simulator data".to_owned(),
            "controller and simulator observations do not tune the planner, route policy, or controller".to_owned(),
            "simulator/controller evidence is not physical-execution evidence".to_owned(),
        ],
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F5ControllerEvidenceStatusV1 {
    ControllerPredictionsMatched,
    ControllerPredictionMismatch,
    InvalidSetup,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5ControllerGateCheckV1 {
    pub passes: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5ControllerLaunchEvidenceV1 {
    pub lane_id: String,
    pub phase_takeoff: bool,
    pub first_update_step_zero: bool,
    pub upright_first_target: bool,
    pub survived_source_launch: bool,
    pub passes: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5ControllerGateEvidenceV1 {
    pub launches: Vec<F5ControllerLaunchEvidenceV1>,
    pub all_lane_launch_valid: F5ControllerGateCheckV1,
    pub flat_target_landing: F5ControllerGateCheckV1,
    pub mesa_direct_non_target_terrain_contact_before_target_touchdown: F5ControllerGateCheckV1,
    pub waypoint_exactly_one_captured_contract_pass: F5ControllerGateCheckV1,
    pub waypoint_target_landing: F5ControllerGateCheckV1,
    pub controller_predictions_matched: F5ControllerGateCheckV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5ControllerIneligibleCaseV1 {
    pub case_id: String,
    pub input_identity: String,
    pub analytical_case_identity: String,
    pub analytical_status: F5AnalyticalCaseStatusV1,
    pub reason: String,
    pub controller_lanes_executed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5ControllerCaseV1 {
    pub case_id: String,
    pub input_identity: String,
    pub prediction_identity: String,
    pub analytical_case_identity: String,
    pub candidate_projection: ExperimentalRidgeCaseProjectionV1,
    pub runtime_v2: ExperimentalRidgeCaseRuntimeProjectionV2,
    pub selected_attempt_index: usize,
    pub selected_attempt: ExperimentalRidgeRuntimeHandoffAttemptV2,
    pub analytical_overlay: AnalyticalShadowOverlay,
    pub lanes: Vec<ShadowLaneSummary>,
    pub gate: F5ControllerGateEvidenceV1,
    pub evidence_status: F5ControllerEvidenceStatusV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5ControllerArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub setup_id: String,
    pub scope: ConservativeBallisticRidgeF5ControllerScopeV1,
    pub input_manifest_identity: String,
    pub prediction_manifest_identity: String,
    pub analytical_result_identity: String,
    pub analytical_artifact_identity: String,
    pub eligible_case_ids: Vec<String>,
    pub analytically_ineligible_cases: Vec<F5ControllerIneligibleCaseV1>,
    pub cases: Vec<ConservativeBallisticRidgeF5ControllerCaseV1>,
    pub evidence_status: F5ControllerEvidenceStatusV1,
    pub deterministic_repeat: bool,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F5ControllerCompactLaneV1 {
    pub lane_id: String,
    pub semantic_identity: String,
    pub controller_id: String,
    pub scenario_id: String,
    pub launch: F5ControllerLaunchEvidenceV1,
    pub class: ShadowLaneClass,
    pub end_reason: Option<EndReason>,
    pub physical_outcome: Option<pd_core::PhysicalOutcome>,
    pub mission_outcome: Option<pd_core::MissionOutcome>,
    pub terrain_contact_reconstruction_valid: Option<bool>,
    pub waypoint_contract_pass: Option<bool>,
    pub waypoint_contract_marker_count: Option<usize>,
    pub waypoint_contract_resolution_reason: Option<String>,
    pub terrain_contact_within_derived_mesa_span: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5ControllerResultCaseV1 {
    pub case_id: String,
    pub input_identity: String,
    pub prediction_identity: String,
    pub analytical_case_identity: String,
    pub candidate_projection_identity: String,
    pub selected_waypoint_candidate_identity: String,
    pub runtime_projection_identity: String,
    pub selected_attempt_index: usize,
    pub selected_attempt_identity: String,
    pub selected_handoff_selection_kind: ExperimentalRidgeRuntimeHandoffSelectionKindV2,
    pub selected_handoff_selection_identity: String,
    pub selected_route_identity: String,
    pub selected_waypoint_position_m: Vec2,
    pub lanes: Vec<F5ControllerCompactLaneV1>,
    pub gate: F5ControllerGateEvidenceV1,
    pub evidence_status: F5ControllerEvidenceStatusV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticRidgeF5ControllerResultV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub scope: ConservativeBallisticRidgeF5ControllerScopeV1,
    pub analytical_result_identity: String,
    pub analytical_artifact_identity: String,
    pub input_manifest_identity: String,
    pub prediction_manifest_identity: String,
    pub eligible_case_ids: Vec<String>,
    pub analytically_ineligible_cases: Vec<F5ControllerIneligibleCaseV1>,
    pub cases: Vec<ConservativeBallisticRidgeF5ControllerResultCaseV1>,
    pub evidence_status: F5ControllerEvidenceStatusV1,
    pub deterministic_repeat: bool,
    pub identity: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConservativeBallisticRidgeF5ControllerPathsV1 {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
    pub result_path: PathBuf,
    pub report_path: PathBuf,
    pub preview_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ConservativeBallisticRidgeF5ControllerRunV1 {
    pub artifact: ConservativeBallisticRidgeF5ControllerArtifactV1,
    pub result: ConservativeBallisticRidgeF5ControllerResultV1,
    pub paths: ConservativeBallisticRidgeF5ControllerPathsV1,
}

#[derive(Clone)]
struct PreparedF5Case {
    case_id: String,
    input_identity: String,
    prediction_identity: String,
    analytical_case_identity: String,
    candidate_projection: ExperimentalRidgeCaseProjectionV1,
    runtime_v2: ExperimentalRidgeCaseRuntimeProjectionV2,
    selected_attempt_index: usize,
    selected_attempt: ExperimentalRidgeRuntimeHandoffAttemptV2,
    controller_expectations: F5ControllerExpectationsV1,
    analytical_overlay: AnalyticalShadowOverlay,
    mesa: MesaGeometryV2,
    flat_scenario: ScenarioSpec,
    mesa_direct_scenario: ScenarioSpec,
    mesa_waypoint_scenario: ScenarioSpec,
    direct_controller: ControllerSpec,
    waypoint_controller: ControllerSpec,
}

#[derive(Clone)]
struct PreparedF5 {
    input_manifest_identity: String,
    prediction_manifest_identity: String,
    analytical_result_identity: String,
    analytical_artifact_identity: String,
    cases: Vec<PreparedF5Case>,
    ineligible_cases: Vec<F5ControllerIneligibleCaseV1>,
}

/// Reveal the controller evidence only after a committed F5c result has
/// established controller eligibility. The sealed inputs and F5c result are
/// checked before the output directory is created or any controller lane runs.
pub fn run_conservative_ballistic_ridge_f5_controller_v1(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
    requested_result_path: Option<&Path>,
) -> Result<ConservativeBallisticRidgeF5ControllerRunV1> {
    let prepared = prepare_committed_f5(repo_root).context(
        "F5 controller setup is invalid before controller evidence; no lane, output, or compact result was produced",
    )?;
    let mut first = execute(&prepared).context(
        "F5 controller execution did not produce trustworthy complete evidence; no output or compact result was produced",
    )?;
    let mut repeat = execute(&prepared).context(
        "F5 controller repeat did not produce trustworthy complete evidence; no output or compact result was produced",
    )?;
    if semantic_material(&first)? != semantic_material(&repeat)? {
        bail!(
            "F5 controller semantic repeat diverged: INVALID SETUP; no output or compact result was produced"
        );
    }
    first.deterministic_repeat = true;
    repeat.deterministic_repeat = true;
    first.finalize_identity()?;
    repeat.finalize_identity()?;
    if first.identity != repeat.identity {
        bail!(
            "F5 controller semantic repeat identity diverged: INVALID SETUP; no output or compact result was produced"
        );
    }
    first.validate_against_source(repo_root).context(
        "F5 controller evidence violates the sealed setup contract: INVALID SETUP; no output or compact result was produced",
    )?;
    let result = ConservativeBallisticRidgeF5ControllerResultV1::from_artifact(&first)?;
    result.validate_against_artifact(repo_root, &first).context(
        "F5 controller compact evidence violates the sealed setup contract: INVALID SETUP; no output or compact result was produced",
    )?;

    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    write_artifacts(&output_dir, &first)?;
    let summary_path = output_dir.join("summary.json");
    let reloaded =
        load_conservative_ballistic_ridge_f5_controller_artifact_v1(repo_root, &summary_path)?;
    if semantic_material(&reloaded)? != semantic_material(&first)? {
        bail!("F5 controller summary changed after reload");
    }
    let result_path = resolve_result_path(repo_root, requested_result_path);
    write_controller_result(&result_path, &result)?;
    let reloaded_result =
        load_conservative_ballistic_ridge_f5_controller_result_v1(repo_root, &result_path)?;
    reloaded_result.validate_against_artifact(repo_root, &reloaded)?;

    let (report_path, update_site) = resolve_report_path(repo_root, &output_dir)?;
    let preview_path = report_path
        .parent()
        .expect("F5 controller report path has a parent")
        .join("preview.svg");
    pd_report::conservative_ballistic_f5_controller::write_conservative_ballistic_f5_controller_report(
        &report_path,
        &preview_path,
        &serde_json::to_value(&reloaded)?,
    )?;
    if update_site {
        ReportSite::new(repo_root).update_indexes_for_file(&report_path)?;
    }
    Ok(ConservativeBallisticRidgeF5ControllerRunV1 {
        artifact: reloaded,
        result: reloaded_result,
        paths: ConservativeBallisticRidgeF5ControllerPathsV1 {
            output_dir,
            summary_path,
            result_path,
            report_path,
            preview_path,
        },
    })
}

pub fn load_conservative_ballistic_ridge_f5_controller_artifact_v1(
    repo_root: &Path,
    path: &Path,
) -> Result<ConservativeBallisticRidgeF5ControllerArtifactV1> {
    let artifact: ConservativeBallisticRidgeF5ControllerArtifactV1 = serde_json::from_slice(
        &fs::read(path)
            .with_context(|| format!("failed to read F5 controller summary {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse F5 controller summary {}", path.display()))?;
    artifact.validate_against_source(repo_root)?;
    Ok(artifact)
}

pub fn load_conservative_ballistic_ridge_f5_controller_result_v1(
    repo_root: &Path,
    path: &Path,
) -> Result<ConservativeBallisticRidgeF5ControllerResultV1> {
    let result: ConservativeBallisticRidgeF5ControllerResultV1 = serde_json::from_slice(
        &fs::read(path)
            .with_context(|| format!("failed to read F5 controller result {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse F5 controller result {}", path.display()))?;
    result.validate_against_source(repo_root)?;
    Ok(result)
}

impl ConservativeBallisticRidgeF5ControllerArtifactV1 {
    fn finalize_identity(&mut self) -> Result<()> {
        self.identity = canonical_digest(&semantic_material(self)?).map_err(anyhow::Error::msg)?;
        Ok(())
    }

    pub fn validate_against_source(&self, repo_root: &Path) -> Result<()> {
        if self.schema_id != CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_SCHEMA_ID_V1
            || self.schema_version != CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_SCHEMA_VERSION_V1
            || self.setup_id != CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_SETUP_ID_V1
            || self.scope != controller_scope()
            || !self.deterministic_repeat
        {
            bail!("F5 controller artifact schema, scope, or repeat binding is invalid");
        }
        let prepared = prepare_committed_f5(repo_root)?;
        if self.input_manifest_identity != prepared.input_manifest_identity
            || self.prediction_manifest_identity != prepared.prediction_manifest_identity
            || self.analytical_result_identity != prepared.analytical_result_identity
            || self.analytical_artifact_identity != prepared.analytical_artifact_identity
            || self.eligible_case_ids
                != prepared
                    .cases
                    .iter()
                    .map(|case| case.case_id.clone())
                    .collect::<Vec<_>>()
            || self.analytically_ineligible_cases != prepared.ineligible_cases
            || self.cases.len() != prepared.cases.len()
        {
            bail!("F5 controller artifact provenance or eligibility join is invalid");
        }
        for (case, prepared_case) in self.cases.iter().zip(&prepared.cases) {
            validate_case_wiring(case, prepared_case)?;
        }
        if self.evidence_status != aggregate_evidence_status(&self.cases) {
            bail!("F5 controller artifact evidence status does not derive from case evidence");
        }
        let expected_identity =
            canonical_digest(&semantic_material(self)?).map_err(anyhow::Error::msg)?;
        if self.identity.is_empty() || self.identity != expected_identity {
            bail!("F5 controller semantic identity does not bind contents");
        }
        Ok(())
    }
}

impl ConservativeBallisticRidgeF5ControllerResultV1 {
    fn from_artifact(artifact: &ConservativeBallisticRidgeF5ControllerArtifactV1) -> Result<Self> {
        let mut result = Self {
            schema_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_RESULT_SCHEMA_ID_V1.to_owned(),
            schema_version: 1,
            scope: artifact.scope.clone(),
            analytical_result_identity: artifact.analytical_result_identity.clone(),
            analytical_artifact_identity: artifact.analytical_artifact_identity.clone(),
            input_manifest_identity: artifact.input_manifest_identity.clone(),
            prediction_manifest_identity: artifact.prediction_manifest_identity.clone(),
            eligible_case_ids: artifact.eligible_case_ids.clone(),
            analytically_ineligible_cases: artifact.analytically_ineligible_cases.clone(),
            cases: artifact
                .cases
                .iter()
                .map(compact_case)
                .collect::<Result<_>>()?,
            evidence_status: artifact.evidence_status.clone(),
            deterministic_repeat: artifact.deterministic_repeat,
            identity: String::new(),
        };
        result.identity = canonical_digest(&result).map_err(anyhow::Error::msg)?;
        Ok(result)
    }

    pub fn validate_against_source(&self, repo_root: &Path) -> Result<()> {
        if self.schema_id != CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_RESULT_SCHEMA_ID_V1
            || self.schema_version != 1
            || self.scope != controller_scope()
            || !self.deterministic_repeat
        {
            bail!("F5 controller result root contract is invalid");
        }
        let mut material = self.clone();
        material.identity.clear();
        let expected_identity = canonical_digest(&material).map_err(anyhow::Error::msg)?;
        if self.identity.is_empty() || self.identity != expected_identity {
            bail!("F5 controller result identity does not bind contents");
        }
        let prepared = prepare_committed_f5(repo_root)?;
        if self.input_manifest_identity != prepared.input_manifest_identity
            || self.prediction_manifest_identity != prepared.prediction_manifest_identity
            || self.analytical_result_identity != prepared.analytical_result_identity
            || self.analytical_artifact_identity != prepared.analytical_artifact_identity
            || self.eligible_case_ids
                != prepared
                    .cases
                    .iter()
                    .map(|case| case.case_id.clone())
                    .collect::<Vec<_>>()
            || self.analytically_ineligible_cases != prepared.ineligible_cases
            || self.cases.len() != prepared.cases.len()
        {
            bail!("F5 controller result provenance or eligibility join is invalid");
        }
        if self.evidence_status != aggregate_compact_evidence_status(&self.cases) {
            bail!(
                "F5 controller result evidence status does not derive from compact case evidence"
            );
        }
        for (case, prepared_case) in self.cases.iter().zip(&prepared.cases) {
            validate_compact_case_against_prepared(case, prepared_case)?;
        }
        Ok(())
    }

    fn validate_against_artifact(
        &self,
        repo_root: &Path,
        artifact: &ConservativeBallisticRidgeF5ControllerArtifactV1,
    ) -> Result<()> {
        self.validate_against_source(repo_root)?;
        if self != &Self::from_artifact(artifact)? {
            bail!("F5 controller result does not exactly match detailed artifact");
        }
        Ok(())
    }
}

fn prepare_committed_f5(repo_root: &Path) -> Result<PreparedF5> {
    let inputs = load_conservative_ballistic_ridge_f5_inputs_v1()
        .map_err(|error| anyhow!("F5 input seal is invalid: {error}"))?;
    let predictions = load_conservative_ballistic_ridge_f5_predictions_v1()
        .map_err(|error| anyhow!("F5 prediction seal is invalid: {error}"))?;
    predictions
        .validate(&inputs)
        .map_err(|error| anyhow!("F5 prediction seal is invalid: {error}"))?;
    let result_path =
        repo_root.join(crate::CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_FIXTURE_V1);
    let analytical = load_conservative_ballistic_ridge_f5_analytical_result_v1(&result_path)?;
    analytical
        .validate_against_seals(&inputs, &predictions)
        .context("F5c analytical result does not bind the F5 seals")?;
    if analytical.identity != CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_IDENTITY_V1 {
        bail!(
            "committed F5c analytical result identity changed: {}",
            analytical.identity
        );
    }
    prepare_from_sealed_evidence(&inputs, &predictions, &analytical)
}

fn prepare_from_sealed_evidence(
    inputs: &ConservativeBallisticRidgeF5InputsV1,
    predictions: &ConservativeBallisticRidgeF5PredictionsV1,
    analytical: &ConservativeBallisticRidgeF5AnalyticalResultV1,
) -> Result<PreparedF5> {
    inputs
        .validate()
        .map_err(|error| anyhow!("F5 input seal is invalid: {error}"))?;
    predictions
        .validate(inputs)
        .map_err(|error| anyhow!("F5 prediction seal is invalid: {error}"))?;
    analytical.validate_against_seals(inputs, predictions)?;
    if analytical.identity != CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_IDENTITY_V1 {
        bail!("F5c analytical result identity is not the committed checkpoint");
    }
    let (eligible_indices, ineligible_cases) = select_controller_eligibility(analytical)?;
    let cases = eligible_indices
        .into_iter()
        .map(|index| {
            prepare_eligible_case(
                &inputs.cases[index],
                &predictions.cases[index],
                &analytical.cases[index],
            )
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(PreparedF5 {
        input_manifest_identity: inputs.identity.clone(),
        prediction_manifest_identity: predictions.identity.clone(),
        analytical_result_identity: analytical.identity.clone(),
        analytical_artifact_identity: analytical.analytical_artifact_identity.clone(),
        cases,
        ineligible_cases,
    })
}

/// Derive the controller executor scope from F5c alone. This is intentionally
/// pure: pre-reveal tests can prove that the stopped case remains unreachable
/// without evaluating candidate/runtime code or calling the simulator seam.
fn select_controller_eligibility(
    analytical: &ConservativeBallisticRidgeF5AnalyticalResultV1,
) -> Result<(Vec<usize>, Vec<F5ControllerIneligibleCaseV1>)> {
    let mut eligible_indices = Vec::new();
    let mut ineligible_cases = Vec::new();
    for (index, analytical_case) in analytical.cases.iter().enumerate() {
        if analytical_case.eligible_for_controller {
            eligible_indices.push(index);
        } else {
            ineligible_cases.push(F5ControllerIneligibleCaseV1 {
                case_id: analytical_case.case_id.clone(),
                input_identity: analytical_case.input_identity.clone(),
                analytical_case_identity: analytical_case.case_identity.clone(),
                analytical_status: analytical_case.status.clone(),
                reason: "analytically_ineligible_not_run".to_owned(),
                controller_lanes_executed: false,
            });
        }
    }
    if eligible_indices != [0]
        || analytical.cases[eligible_indices[0]].case_id != F5_EXPECTED_ELIGIBLE_CASE_ID_V1
        || ineligible_cases.len() != 1
        || ineligible_cases[0].case_id != F5_EXPECTED_INELIGIBLE_CASE_ID_V1
        || ineligible_cases[0].analytical_status
            != F5AnalyticalCaseStatusV1::StoppedCandidateUnsupported
    {
        bail!("committed F5c eligibility set differs from the F5d reveal contract");
    }
    Ok((eligible_indices, ineligible_cases))
}

fn prepare_eligible_case(
    input: &ExperimentalRidgeCaseInputV1,
    prediction: &ConservativeBallisticRidgeF5PredictionV1,
    analytical_case: &ConservativeBallisticRidgeF5AnalyticalResultCaseV1,
) -> Result<PreparedF5Case> {
    if !analytical_case.eligible_for_controller
        || analytical_case.status != F5AnalyticalCaseStatusV1::EligibleForController
        || analytical_case.case_id != input.probe.id
        || analytical_case.case_id != prediction.case_id
        || analytical_case.input_identity != input.identity
    {
        bail!("F5 eligible analytical case does not join sealed source evidence");
    }
    let candidate_projection = evaluate_experimental_ridge_case_projection_v1(input)
        .map_err(|error| anyhow!("F5 candidate projection failed: {error}"))?;
    validate_experimental_ridge_case_projection_v1(&candidate_projection)
        .map_err(|error| anyhow!("F5 candidate projection did not validate: {error}"))?;
    if Some(&candidate_projection.identity)
        != analytical_case.candidate_projection_identity.as_ref()
        || Some(&candidate_projection.analytical_canary_identity)
            != analytical_case.analytical_canary_identity.as_ref()
        || Some(&candidate_projection.waypoint_search_identity)
            != analytical_case.waypoint_search_identity.as_ref()
    {
        bail!("F5 candidate projection does not exactly join committed F5c evidence");
    }
    let (nominal, waypoint_candidate) = match (
        &candidate_projection.flat_control,
        &candidate_projection.derived_mesa,
    ) {
        (
            ExperimentalRidgeCandidateOutcomeV2::Direct { candidate: nominal },
            ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
                candidate: waypoint,
                ..
            },
        ) => (nominal, waypoint),
        _ => bail!("F5 eligible case does not retain direct plus one-waypoint candidates"),
    };
    if Some(&nominal.identity) != analytical_case.flat_selected_candidate_identity.as_ref()
        || Some(
            &candidate_projection
                .derived_nominal_direct
                .candidate_identity,
        ) != analytical_case.derived_nominal_candidate_identity.as_ref()
        || Some(&waypoint_candidate.identity)
            != analytical_case
                .selected_waypoint_candidate_identity
                .as_ref()
    {
        bail!("F5 candidate identities do not exactly join committed F5c evidence");
    }
    let runtime_v2 = project_experimental_ridge_case_runtime_v2(&candidate_projection)
        .map_err(|error| anyhow!("F5 runtime V2 projection failed: {error}"))?;
    validate_experimental_ridge_case_runtime_v2(&candidate_projection, &runtime_v2)
        .map_err(|error| anyhow!("F5 runtime V2 projection did not validate: {error}"))?;
    if Some(&runtime_v2.identity) != analytical_case.runtime_projection_identity.as_ref() {
        bail!("F5 runtime V2 identity does not exactly join committed F5c evidence");
    }
    let direct_route = match &runtime_v2.flat_control {
        ExperimentalRidgeCaseRuntimeOutcomeV2::Direct {
            candidate_identity,
            route,
            structural_validation,
            identity,
        } if candidate_identity == &nominal.identity
            && structural_validation.transfer_route_valid =>
        {
            if Some(identity) != analytical_case.flat_runtime_outcome_identity.as_ref() {
                bail!("F5 flat runtime outcome identity does not join F5c evidence");
            }
            route.clone()
        }
        _ => bail!("F5 flat runtime must retain a structurally valid direct route"),
    };
    let (
        waypoint_route,
        selected_attempt_index,
        selected_attempt,
        selection_identity,
        waypoint_identity,
    ) = match &runtime_v2.derived_mesa {
        ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
            candidate_identity,
            route,
            handoff_selection,
            structural_validation,
            identity,
            ..
        } if candidate_identity == &waypoint_candidate.identity
            && structural_validation.transfer_route_valid =>
        {
            if Some(identity) != analytical_case.derived_runtime_outcome_identity.as_ref() {
                bail!("F5 derived runtime outcome identity does not join F5c evidence");
            }
            let selected = handoff_selection
                .selected_attempt()
                .cloned()
                .ok_or_else(|| anyhow!("F5 runtime selected attempt index is invalid"))?;
            (
                route.clone(),
                handoff_selection.selected_attempt_index,
                selected,
                handoff_selection.identity.clone(),
                candidate_identity.clone(),
            )
        }
        _ => bail!("F5 derived runtime must retain a structurally valid one-waypoint route"),
    };
    if waypoint_identity != waypoint_candidate.identity
        || waypoint_route.waypoints.len() != 1
        || waypoint_route.waypoints[0].position_m != selected_attempt.selected_state.position_m
        || selected_attempt.route != waypoint_route
        || !selected_attempt.structural_validation.transfer_route_valid
        || !selected_attempt.handoff_assessment.contract_pass
    {
        bail!("F5 selected runtime attempt does not provide one valid waypoint route");
    }
    let observed = analytical_case
        .runtime_expectation
        .as_ref()
        .and_then(|comparison| comparison.observed_handoff.as_ref())
        .ok_or_else(|| anyhow!("F5c eligible case lacks observed selected-handoff evidence"))?;
    let route_identity = canonical_digest(&selected_attempt.route).map_err(anyhow::Error::msg)?;
    if observed.selection_kind != selected_attempt.selection_kind
        || observed.selected_attempt_index != selected_attempt_index
        || observed.handoff_selection_identity != selection_identity
        || observed.selected_attempt_identity != selected_attempt.identity
        || observed.route_identity != route_identity
        || observed.waypoint_position_m != selected_attempt.selected_state.position_m
    {
        bail!("F5 selected runtime attempt does not exactly join committed F5c handoff evidence");
    }
    let flat_scenario = build_scenario(
        &input.probe,
        &candidate_projection.policy,
        &candidate_projection.vehicle,
        flat_terrain(&input.probe),
        direct_route.clone(),
        "flat-direct",
        &format!("F5 {} flat direct controller lane", input.probe.id),
    )?;
    let mesa_direct_scenario = build_scenario(
        &input.probe,
        &candidate_projection.policy,
        &candidate_projection.vehicle,
        mesa_terrain(&candidate_projection.mesa),
        direct_route,
        "mesa-direct",
        &format!("F5 {} mesa direct controller lane", input.probe.id),
    )?;
    let mesa_waypoint_scenario = build_scenario(
        &input.probe,
        &candidate_projection.policy,
        &candidate_projection.vehicle,
        mesa_terrain(&candidate_projection.mesa),
        waypoint_route,
        "mesa-waypoint",
        &format!(
            "F5 {} selected runtime waypoint controller lane",
            input.probe.id
        ),
    )?;
    let direct_controller = built_in_controller_spec("transfer_pdg")
        .ok_or_else(|| anyhow!("F5 direct controller transfer_pdg is unavailable"))?;
    let waypoint_controller = built_in_controller_spec("transfer_waypoint_pdg")
        .ok_or_else(|| anyhow!("F5 waypoint controller transfer_waypoint_pdg is unavailable"))?;
    Ok(PreparedF5Case {
        case_id: input.probe.id.clone(),
        input_identity: input.identity.clone(),
        prediction_identity: analytical_case.prediction_identity.clone(),
        analytical_case_identity: analytical_case.case_identity.clone(),
        candidate_projection: candidate_projection.clone(),
        runtime_v2,
        selected_attempt_index,
        selected_attempt,
        controller_expectations: prediction.controller.clone(),
        analytical_overlay: build_analytical_overlay(nominal, waypoint_candidate),
        mesa: candidate_projection.mesa.clone(),
        flat_scenario,
        mesa_direct_scenario,
        mesa_waypoint_scenario,
        direct_controller,
        waypoint_controller,
    })
}

fn execute(prepared: &PreparedF5) -> Result<ConservativeBallisticRidgeF5ControllerArtifactV1> {
    let cases = prepared
        .cases
        .iter()
        .map(execute_case)
        .collect::<Result<Vec<_>>>()?;
    let mut artifact = ConservativeBallisticRidgeF5ControllerArtifactV1 {
        schema_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_SCHEMA_ID_V1.to_owned(),
        schema_version: CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_SCHEMA_VERSION_V1,
        setup_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_SETUP_ID_V1.to_owned(),
        scope: controller_scope(),
        input_manifest_identity: prepared.input_manifest_identity.clone(),
        prediction_manifest_identity: prepared.prediction_manifest_identity.clone(),
        analytical_result_identity: prepared.analytical_result_identity.clone(),
        analytical_artifact_identity: prepared.analytical_artifact_identity.clone(),
        eligible_case_ids: prepared
            .cases
            .iter()
            .map(|case| case.case_id.clone())
            .collect(),
        analytically_ineligible_cases: prepared.ineligible_cases.clone(),
        evidence_status: aggregate_evidence_status(&cases),
        cases,
        deterministic_repeat: false,
        identity: String::new(),
    };
    artifact.finalize_identity()?;
    Ok(artifact)
}

fn execute_case(prepared: &PreparedF5Case) -> Result<ConservativeBallisticRidgeF5ControllerCaseV1> {
    let lanes = vec![
        run_lane(
            "flat-direct",
            "flat",
            prepared.flat_scenario.clone(),
            prepared.direct_controller.clone(),
            &prepared.mesa,
            None,
        )?,
        run_lane(
            "mesa-direct",
            "mesa",
            prepared.mesa_direct_scenario.clone(),
            prepared.direct_controller.clone(),
            &prepared.mesa,
            None,
        )?,
        run_lane(
            "mesa-waypoint",
            "mesa",
            prepared.mesa_waypoint_scenario.clone(),
            prepared.waypoint_controller.clone(),
            &prepared.mesa,
            Some(prepared.selected_attempt.selected_state.position_m),
        )?,
    ];
    let gate = derive_gate(&lanes, &prepared.controller_expectations)?;
    let evidence_status = classify_evidence_status(&gate);
    Ok(ConservativeBallisticRidgeF5ControllerCaseV1 {
        case_id: prepared.case_id.clone(),
        input_identity: prepared.input_identity.clone(),
        prediction_identity: prepared.prediction_identity.clone(),
        analytical_case_identity: prepared.analytical_case_identity.clone(),
        candidate_projection: prepared.candidate_projection.clone(),
        runtime_v2: prepared.runtime_v2.clone(),
        selected_attempt_index: prepared.selected_attempt_index,
        selected_attempt: prepared.selected_attempt.clone(),
        analytical_overlay: prepared.analytical_overlay.clone(),
        lanes,
        gate,
        evidence_status,
    })
}

fn validate_case_wiring(
    case: &ConservativeBallisticRidgeF5ControllerCaseV1,
    prepared: &PreparedF5Case,
) -> Result<()> {
    if case.case_id != prepared.case_id
        || case.input_identity != prepared.input_identity
        || case.prediction_identity != prepared.prediction_identity
        || case.analytical_case_identity != prepared.analytical_case_identity
        || case.candidate_projection != prepared.candidate_projection
        || case.runtime_v2 != prepared.runtime_v2
        || case.selected_attempt_index != prepared.selected_attempt_index
        || case.selected_attempt != prepared.selected_attempt
        || case.analytical_overlay != prepared.analytical_overlay
    {
        bail!("F5 controller case does not bind recomputed source/runtime evidence");
    }
    let expected = [
        (
            "flat-direct",
            "flat",
            &prepared.flat_scenario,
            &prepared.direct_controller,
        ),
        (
            "mesa-direct",
            "mesa",
            &prepared.mesa_direct_scenario,
            &prepared.direct_controller,
        ),
        (
            "mesa-waypoint",
            "mesa",
            &prepared.mesa_waypoint_scenario,
            &prepared.waypoint_controller,
        ),
    ];
    if case.lanes.len() != expected.len() {
        bail!("F5 controller case must retain exactly three lanes");
    }
    for (lane, (id, terrain_kind, scenario, controller)) in case.lanes.iter().zip(expected) {
        if lane.id != id
            || lane.terrain_kind != terrain_kind
            || lane.scenario_id != scenario.id
            || lane.controller_id != controller.id()
            || lane.scenario.as_ref() != Some(scenario)
            || lane.controller.as_ref() != Some(controller)
            || lane.preflight.route != scenario.mission.transfer_route
            || lane.run.is_none()
        {
            bail!("F5 controller lane {id} does not bind prepared scenario/controller/run wiring");
        }
    }
    let waypoint_route = prepared
        .mesa_waypoint_scenario
        .mission
        .transfer_route
        .as_ref()
        .expect("prepared F5 waypoint route");
    if waypoint_route.waypoints.len() != 1
        || waypoint_route.waypoints[0].position_m
            != prepared.selected_attempt.selected_state.position_m
        || case.lanes[2].preflight.route.as_ref() != Some(waypoint_route)
    {
        bail!("F5 selected runtime waypoint route wiring changed");
    }
    let expected_gate = derive_gate(&case.lanes, &prepared.controller_expectations)?;
    if case.gate != expected_gate
        || case.evidence_status != classify_evidence_status(&expected_gate)
    {
        bail!("F5 controller gate or evidence classification does not derive from lanes");
    }
    Ok(())
}

/// Validate everything a compact result can prove without replaying a
/// controller/simulator lane. The detailed artifact is still checked on the
/// initial reveal write, while this read-side gate prevents a compact fixture
/// from becoming self-authenticating merely by recomputing its root digest.
fn validate_compact_case_against_prepared(
    case: &ConservativeBallisticRidgeF5ControllerResultCaseV1,
    prepared: &PreparedF5Case,
) -> Result<()> {
    let (waypoint_candidate_identity, handoff_selection_identity) = match (
        &prepared.candidate_projection.derived_mesa,
        &prepared.runtime_v2.derived_mesa,
    ) {
        (
            ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. },
            ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
                handoff_selection, ..
            },
        ) => (&candidate.identity, &handoff_selection.identity),
        _ => bail!("F5 compact result prepared source lacks selected waypoint evidence"),
    };
    let selected_route_identity =
        canonical_digest(&prepared.selected_attempt.route).map_err(anyhow::Error::msg)?;
    if case.case_id != prepared.case_id
        || case.input_identity != prepared.input_identity
        || case.prediction_identity != prepared.prediction_identity
        || case.analytical_case_identity != prepared.analytical_case_identity
        || case.candidate_projection_identity != prepared.candidate_projection.identity
        || case.selected_waypoint_candidate_identity != *waypoint_candidate_identity
        || case.runtime_projection_identity != prepared.runtime_v2.identity
        || case.selected_attempt_index != prepared.selected_attempt_index
        || case.selected_attempt_identity != prepared.selected_attempt.identity
        || case.selected_handoff_selection_kind != prepared.selected_attempt.selection_kind
        || case.selected_handoff_selection_identity != *handoff_selection_identity
        || case.selected_route_identity != selected_route_identity
        || case.selected_waypoint_position_m != prepared.selected_attempt.selected_state.position_m
    {
        bail!("F5 compact result case provenance or selected route join is invalid");
    }
    let expected = [
        (
            "flat-direct",
            &prepared.direct_controller,
            &prepared.flat_scenario,
        ),
        (
            "mesa-direct",
            &prepared.direct_controller,
            &prepared.mesa_direct_scenario,
        ),
        (
            "mesa-waypoint",
            &prepared.waypoint_controller,
            &prepared.mesa_waypoint_scenario,
        ),
    ];
    if case.lanes.len() != expected.len() {
        bail!("F5 compact result case must retain exactly three ordered lanes");
    }
    for (lane, (id, controller, scenario)) in case.lanes.iter().zip(expected) {
        if lane.lane_id != id
            || lane.controller_id != controller.id()
            || lane.scenario_id != scenario.id
            || !is_canonical_digest(&lane.semantic_identity)
        {
            bail!("F5 compact result lane {id} does not bind source scenario/controller evidence");
        }
        let expected_launch = compact_launch_evidence(lane);
        if lane.launch != expected_launch {
            bail!("F5 compact result lane {id} launch evidence is internally inconsistent");
        }
        if !compact_lane_outcomes_match_end_reason(lane) {
            bail!("F5 compact result lane {id} outcomes do not agree with its end reason");
        }
        if lane.class != compact_lane_class(lane) {
            bail!(
                "F5 compact result lane {id} class does not derive from compact outcome evidence"
            );
        }
        if id == "mesa-waypoint" {
            let no_contract_evidence = lane.waypoint_contract_pass.is_none()
                && lane.waypoint_contract_marker_count.is_none()
                && lane.waypoint_contract_resolution_reason.is_none();
            let complete_contract_evidence = lane.waypoint_contract_pass.is_some()
                && lane.waypoint_contract_marker_count.is_some();
            // A completed, valid simulator lane may emit no capture evidence.
            // That is a controller prediction mismatch, not an invalid setup.
            // Conversely, a partially represented contract would make the compact
            // fixture unable to reproduce the detailed gate truthfully.
            if !no_contract_evidence && !complete_contract_evidence {
                bail!("F5 compact result waypoint lane has partial contract evidence");
            }
        } else if lane.waypoint_contract_pass.is_some()
            || lane.waypoint_contract_marker_count.is_some()
            || lane.waypoint_contract_resolution_reason.is_some()
        {
            bail!("F5 compact result non-waypoint lane carries waypoint contract evidence");
        }
    }
    let expected_gate = derive_compact_gate(&case.lanes, &prepared.controller_expectations)?;
    if case.gate != expected_gate
        || case.evidence_status != classify_evidence_status(&expected_gate)
    {
        bail!(
            "F5 compact result gate or evidence status does not derive from compact lane evidence"
        );
    }
    Ok(())
}

/// `canonical_digest` renders an FNV-1a u64 as lowercase hexadecimal with a
/// minimum width of 12. A semantic lane digest cannot be recomputed without
/// replaying its simulator evidence, but it must retain this exact evaluator
/// representation rather than a planner-owned `fnv1a64:` identity prefix.
fn is_canonical_digest(value: &str) -> bool {
    (12..=16).contains(&value.len())
        && value.bytes().all(|byte| {
            byte.is_ascii_digit() || (byte.is_ascii_lowercase() && byte.is_ascii_hexdigit())
        })
}

fn compact_lane_outcomes_match_end_reason(lane: &F5ControllerCompactLaneV1) -> bool {
    matches!(
        (
            lane.end_reason.as_ref(),
            lane.physical_outcome.as_ref(),
            lane.mission_outcome.as_ref(),
        ),
        (
            Some(EndReason::Running),
            Some(pd_core::PhysicalOutcome::Flying),
            Some(pd_core::MissionOutcome::InProgress),
        ) | (
            Some(EndReason::CheckpointSatisfied),
            Some(pd_core::PhysicalOutcome::Flying),
            Some(pd_core::MissionOutcome::Success),
        ) | (
            Some(EndReason::CheckpointFailed),
            Some(pd_core::PhysicalOutcome::Flying),
            Some(pd_core::MissionOutcome::FailedCheckpoint),
        ) | (
            Some(EndReason::TouchdownOnTarget),
            Some(pd_core::PhysicalOutcome::LandedOnTarget),
            Some(pd_core::MissionOutcome::Success),
        ) | (
            Some(EndReason::TouchdownOffTarget),
            Some(pd_core::PhysicalOutcome::LandedOffTarget),
            Some(pd_core::MissionOutcome::FailedOffTarget),
        ) | (
            Some(EndReason::Crash),
            Some(pd_core::PhysicalOutcome::Crashed),
            Some(pd_core::MissionOutcome::FailedCrash),
        ) | (
            Some(EndReason::MaxTimeReached),
            Some(pd_core::PhysicalOutcome::TimedOut),
            Some(pd_core::MissionOutcome::FailedTimeout),
        )
    )
}

fn compact_launch_evidence(lane: &F5ControllerCompactLaneV1) -> F5ControllerLaunchEvidenceV1 {
    let passes = lane.launch.phase_takeoff
        && lane.launch.first_update_step_zero
        && lane.launch.upright_first_target
        && lane.launch.survived_source_launch;
    F5ControllerLaunchEvidenceV1 {
        lane_id: lane.lane_id.clone(),
        phase_takeoff: lane.launch.phase_takeoff,
        first_update_step_zero: lane.launch.first_update_step_zero,
        upright_first_target: lane.launch.upright_first_target,
        survived_source_launch: lane.launch.survived_source_launch,
        passes,
        reason: if passes {
            format!("{} launch preflight is valid", lane.lane_id)
        } else {
            format!(
                "{} launch phase_takeoff={}; first_update_step_zero={}; upright_first_target={}; survived_source_launch={}",
                lane.lane_id,
                lane.launch.phase_takeoff,
                lane.launch.first_update_step_zero,
                lane.launch.upright_first_target,
                lane.launch.survived_source_launch,
            )
        },
    }
}

fn compact_lane_class(lane: &F5ControllerCompactLaneV1) -> ShadowLaneClass {
    if lane.lane_id == "flat-direct" && lane.end_reason == Some(EndReason::TouchdownOnTarget) {
        ShadowLaneClass::TargetLanding
    } else if lane.lane_id == "mesa-direct"
        && lane.end_reason == Some(EndReason::Crash)
        && lane.terrain_contact_within_derived_mesa_span
    {
        ShadowLaneClass::TerrainCrash
    } else if lane.lane_id == "mesa-waypoint"
        && lane.end_reason == Some(EndReason::TouchdownOnTarget)
        && lane.waypoint_contract_pass == Some(true)
    {
        ShadowLaneClass::TargetLanding
    } else {
        ShadowLaneClass::Inconclusive
    }
}

fn derive_compact_gate(
    lanes: &[F5ControllerCompactLaneV1],
    expectations: &F5ControllerExpectationsV1,
) -> Result<F5ControllerGateEvidenceV1> {
    if lanes.len() != 3
        || lanes[0].lane_id != "flat-direct"
        || lanes[1].lane_id != "mesa-direct"
        || lanes[2].lane_id != "mesa-waypoint"
    {
        bail!("F5 compact gate requires ordered flat-direct, mesa-direct, mesa-waypoint lanes");
    }
    let launches = lanes
        .iter()
        .map(compact_launch_evidence)
        .collect::<Vec<_>>();
    let all_lane_launch_valid = aggregate_launches(&launches);
    let flat_target_landing = check(
        expectations.flat_direct.outcome == F5ControllerLandingExpectationV1::TargetLanding
            && lanes[0].end_reason == Some(EndReason::TouchdownOnTarget),
        format!("flat-direct ended as {:?}", lanes[0].end_reason),
        "flat-direct landed on target".to_owned(),
    );
    let mesa_direct_non_target_terrain_contact_before_target_touchdown = check(
        expectations.mesa_direct.outcome
            == F5ControllerTerrainContactExpectationV1::NonTargetTerrainContact
            && expectations.mesa_direct.location
                == F5TerrainContactLocationExpectationV1::DerivedMesa
            && expectations.mesa_direct.ordering
                == F5TerrainContactOrderingExpectationV1::BeforeTargetTouchdown
            && lanes[1].end_reason == Some(EndReason::Crash)
            && lanes[1].terrain_contact_reconstruction_valid == Some(true)
            && lanes[1].terrain_contact_within_derived_mesa_span,
        format!(
            "mesa-direct ended as {:?}; reconstructed_contact_valid={:?}; within_derived_mesa_span={}",
            lanes[1].end_reason,
            lanes[1].terrain_contact_reconstruction_valid,
            lanes[1].terrain_contact_within_derived_mesa_span,
        ),
        "mesa-direct contacted non-target terrain within the derived mesa before target touchdown"
            .to_owned(),
    );
    let waypoint_contract_failure_reason = if lanes[2].waypoint_contract_pass.is_none()
        && lanes[2].waypoint_contract_marker_count.is_none()
        && lanes[2].waypoint_contract_resolution_reason.is_none()
    {
        "mesa-waypoint emitted no waypoint contract evidence".to_owned()
    } else {
        format!(
            "mesa-waypoint marker_count={}; contract_pass={}; resolution_reason={:?}",
            lanes[2].waypoint_contract_marker_count.unwrap_or_default(),
            lanes[2].waypoint_contract_pass.unwrap_or(false),
            lanes[2].waypoint_contract_resolution_reason,
        )
    };
    let waypoint_exactly_one_captured_contract_pass = check(
        expectations.mesa_waypoint.capture
            == F5WaypointCaptureExpectationV1::ExactlyOneContractPassCapture
            && lanes[2].waypoint_contract_marker_count == Some(1)
            && lanes[2].waypoint_contract_pass == Some(true)
            && lanes[2].waypoint_contract_resolution_reason.as_deref() == Some("contract_pass"),
        waypoint_contract_failure_reason,
        "mesa-waypoint emitted exactly one captured contract_pass marker".to_owned(),
    );
    let waypoint_target_landing = check(
        expectations.mesa_waypoint.terminal_outcome
            == F5ControllerLandingExpectationV1::TargetLanding
            && lanes[2].end_reason == Some(EndReason::TouchdownOnTarget),
        format!("mesa-waypoint ended as {:?}", lanes[2].end_reason),
        "mesa-waypoint landed on target".to_owned(),
    );
    let predictions_match = all_lane_launch_valid.passes
        && flat_target_landing.passes
        && mesa_direct_non_target_terrain_contact_before_target_touchdown.passes
        && waypoint_exactly_one_captured_contract_pass.passes
        && waypoint_target_landing.passes;
    let controller_predictions_matched = if predictions_match {
        F5ControllerGateCheckV1 {
            passes: true,
            reason: "all sealed F5 controller predictions matched".to_owned(),
        }
    } else {
        F5ControllerGateCheckV1 {
            passes: false,
            reason: [
                &all_lane_launch_valid,
                &flat_target_landing,
                &mesa_direct_non_target_terrain_contact_before_target_touchdown,
                &waypoint_exactly_one_captured_contract_pass,
                &waypoint_target_landing,
            ]
            .into_iter()
            .filter(|check| !check.passes)
            .map(|check| check.reason.as_str())
            .collect::<Vec<_>>()
            .join("; "),
        }
    };
    Ok(F5ControllerGateEvidenceV1 {
        launches,
        all_lane_launch_valid,
        flat_target_landing,
        mesa_direct_non_target_terrain_contact_before_target_touchdown,
        waypoint_exactly_one_captured_contract_pass,
        waypoint_target_landing,
        controller_predictions_matched,
    })
}

fn derive_gate(
    lanes: &[ShadowLaneSummary],
    expectations: &F5ControllerExpectationsV1,
) -> Result<F5ControllerGateEvidenceV1> {
    if lanes.len() != 3
        || lanes[0].id != "flat-direct"
        || lanes[1].id != "mesa-direct"
        || lanes[2].id != "mesa-waypoint"
    {
        bail!("F5 controller gate requires ordered flat-direct, mesa-direct, mesa-waypoint lanes");
    }
    let launches = lanes.iter().map(launch_evidence).collect::<Vec<_>>();
    let all_lane_launch_valid = aggregate_launches(&launches);
    let flat_target_landing = check(
        expectations.flat_direct.outcome == F5ControllerLandingExpectationV1::TargetLanding
            && lanes[0].end_reason == Some(EndReason::TouchdownOnTarget),
        format!("flat-direct ended as {:?}", lanes[0].end_reason),
        "flat-direct landed on target".to_owned(),
    );
    let mesa_direct_non_target_terrain_contact_before_target_touchdown = check(
        expectations.mesa_direct.outcome
            == F5ControllerTerrainContactExpectationV1::NonTargetTerrainContact
            && expectations.mesa_direct.location
                == F5TerrainContactLocationExpectationV1::DerivedMesa
            && expectations.mesa_direct.ordering
                == F5TerrainContactOrderingExpectationV1::BeforeTargetTouchdown
            && lanes[1].end_reason == Some(EndReason::Crash)
            && lanes[1].terrain_contact.reconstruction_valid == Some(true)
            && lanes[1].terrain_contact.within_derived_mesa_span,
        format!(
            "mesa-direct ended as {:?}; reconstructed_contact_valid={:?}; within_derived_mesa_span={}",
            lanes[1].end_reason,
            lanes[1].terrain_contact.reconstruction_valid,
            lanes[1].terrain_contact.within_derived_mesa_span,
        ),
        "mesa-direct contacted non-target terrain within the derived mesa before target touchdown"
            .to_owned(),
    );
    let waypoint_exactly_one_captured_contract_pass = check(
        expectations.mesa_waypoint.capture
            == F5WaypointCaptureExpectationV1::ExactlyOneContractPassCapture
            && lanes[2].waypoint_contract.as_ref().is_some_and(|contract| {
                contract.marker_count == 1
                    && contract.contract_pass
                    && contract.resolution_reason.as_deref() == Some("contract_pass")
            }),
        lanes[2].waypoint_contract.as_ref().map_or_else(
            || "mesa-waypoint emitted no waypoint contract evidence".to_owned(),
            |contract| {
                format!(
                    "mesa-waypoint marker_count={}; contract_pass={}; resolution_reason={:?}",
                    contract.marker_count, contract.contract_pass, contract.resolution_reason
                )
            },
        ),
        "mesa-waypoint emitted exactly one captured contract_pass marker".to_owned(),
    );
    let waypoint_target_landing = check(
        expectations.mesa_waypoint.terminal_outcome
            == F5ControllerLandingExpectationV1::TargetLanding
            && lanes[2].end_reason == Some(EndReason::TouchdownOnTarget),
        format!("mesa-waypoint ended as {:?}", lanes[2].end_reason),
        "mesa-waypoint landed on target".to_owned(),
    );
    let predictions_match = all_lane_launch_valid.passes
        && flat_target_landing.passes
        && mesa_direct_non_target_terrain_contact_before_target_touchdown.passes
        && waypoint_exactly_one_captured_contract_pass.passes
        && waypoint_target_landing.passes;
    let controller_predictions_matched = if predictions_match {
        F5ControllerGateCheckV1 {
            passes: true,
            reason: "all sealed F5 controller predictions matched".to_owned(),
        }
    } else {
        F5ControllerGateCheckV1 {
            passes: false,
            reason: [
                &all_lane_launch_valid,
                &flat_target_landing,
                &mesa_direct_non_target_terrain_contact_before_target_touchdown,
                &waypoint_exactly_one_captured_contract_pass,
                &waypoint_target_landing,
            ]
            .into_iter()
            .filter(|check| !check.passes)
            .map(|check| check.reason.as_str())
            .collect::<Vec<_>>()
            .join("; "),
        }
    };
    Ok(F5ControllerGateEvidenceV1 {
        launches,
        all_lane_launch_valid,
        flat_target_landing,
        mesa_direct_non_target_terrain_contact_before_target_touchdown,
        waypoint_exactly_one_captured_contract_pass,
        waypoint_target_landing,
        controller_predictions_matched,
    })
}

fn classify_evidence_status(gate: &F5ControllerGateEvidenceV1) -> F5ControllerEvidenceStatusV1 {
    if !gate.all_lane_launch_valid.passes {
        F5ControllerEvidenceStatusV1::InvalidSetup
    } else if gate.controller_predictions_matched.passes {
        F5ControllerEvidenceStatusV1::ControllerPredictionsMatched
    } else {
        F5ControllerEvidenceStatusV1::ControllerPredictionMismatch
    }
}

fn aggregate_evidence_status(
    cases: &[ConservativeBallisticRidgeF5ControllerCaseV1],
) -> F5ControllerEvidenceStatusV1 {
    if cases
        .iter()
        .any(|case| case.evidence_status == F5ControllerEvidenceStatusV1::InvalidSetup)
    {
        F5ControllerEvidenceStatusV1::InvalidSetup
    } else if cases.iter().all(|case| {
        case.evidence_status == F5ControllerEvidenceStatusV1::ControllerPredictionsMatched
    }) {
        F5ControllerEvidenceStatusV1::ControllerPredictionsMatched
    } else {
        F5ControllerEvidenceStatusV1::ControllerPredictionMismatch
    }
}

fn aggregate_compact_evidence_status(
    cases: &[ConservativeBallisticRidgeF5ControllerResultCaseV1],
) -> F5ControllerEvidenceStatusV1 {
    if cases
        .iter()
        .any(|case| case.evidence_status == F5ControllerEvidenceStatusV1::InvalidSetup)
    {
        F5ControllerEvidenceStatusV1::InvalidSetup
    } else if cases.iter().all(|case| {
        case.evidence_status == F5ControllerEvidenceStatusV1::ControllerPredictionsMatched
    }) {
        F5ControllerEvidenceStatusV1::ControllerPredictionsMatched
    } else {
        F5ControllerEvidenceStatusV1::ControllerPredictionMismatch
    }
}

fn aggregate_launches(launches: &[F5ControllerLaunchEvidenceV1]) -> F5ControllerGateCheckV1 {
    if launches.iter().all(|check| check.passes) {
        F5ControllerGateCheckV1 {
            passes: true,
            reason: "all lanes began takeoff at controller step 0 with upright first target and survived source launch".to_owned(),
        }
    } else {
        F5ControllerGateCheckV1 {
            passes: false,
            reason: launches
                .iter()
                .filter(|check| !check.passes)
                .map(|check| check.reason.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        }
    }
}

fn launch_evidence(lane: &ShadowLaneSummary) -> F5ControllerLaunchEvidenceV1 {
    let phase_takeoff = lane.preflight.first_phase.as_deref() == Some("takeoff");
    let first_update_step_zero = lane.preflight.first_controller_update_physics_step == Some(0);
    let upright_first_target = lane
        .preflight
        .first_target_attitude_rad
        .is_some_and(|attitude| attitude.abs() <= 1.0e-12);
    let survived_source_launch = lane.preflight.survived_source_launch == Some(true);
    let passes =
        phase_takeoff && first_update_step_zero && upright_first_target && survived_source_launch;
    F5ControllerLaunchEvidenceV1 {
        lane_id: lane.id.clone(),
        phase_takeoff,
        first_update_step_zero,
        upright_first_target,
        survived_source_launch,
        passes,
        reason: if passes {
            format!("{} launch preflight is valid", lane.id)
        } else {
            format!(
                "{} launch phase_takeoff={phase_takeoff}; first_update_step_zero={first_update_step_zero}; upright_first_target={upright_first_target}; survived_source_launch={survived_source_launch}",
                lane.id,
            )
        },
    }
}

fn check(passes: bool, failure_reason: String, passing_reason: String) -> F5ControllerGateCheckV1 {
    F5ControllerGateCheckV1 {
        passes,
        reason: if passes {
            passing_reason
        } else {
            failure_reason
        },
    }
}

fn compact_case(
    case: &ConservativeBallisticRidgeF5ControllerCaseV1,
) -> Result<ConservativeBallisticRidgeF5ControllerResultCaseV1> {
    let (handoff_selection_identity, route_identity) = match &case.runtime_v2.derived_mesa {
        ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
            handoff_selection, ..
        } => (
            handoff_selection.identity.clone(),
            canonical_digest(&case.selected_attempt.route).map_err(anyhow::Error::msg)?,
        ),
        _ => bail!("F5 compact controller result lacks selected runtime one-waypoint evidence"),
    };
    Ok(ConservativeBallisticRidgeF5ControllerResultCaseV1 {
        case_id: case.case_id.clone(),
        input_identity: case.input_identity.clone(),
        prediction_identity: case.prediction_identity.clone(),
        analytical_case_identity: case.analytical_case_identity.clone(),
        candidate_projection_identity: case.candidate_projection.identity.clone(),
        selected_waypoint_candidate_identity: match &case.candidate_projection.derived_mesa {
            ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. } => {
                candidate.identity.clone()
            }
            _ => bail!("F5 compact controller result lacks selected waypoint candidate evidence"),
        },
        runtime_projection_identity: case.runtime_v2.identity.clone(),
        selected_attempt_index: case.selected_attempt_index,
        selected_attempt_identity: case.selected_attempt.identity.clone(),
        selected_handoff_selection_kind: case.selected_attempt.selection_kind,
        selected_handoff_selection_identity: handoff_selection_identity,
        selected_route_identity: route_identity,
        selected_waypoint_position_m: case.selected_attempt.selected_state.position_m,
        lanes: case
            .lanes
            .iter()
            .map(|lane| {
                Ok(F5ControllerCompactLaneV1 {
                    lane_id: lane.id.clone(),
                    semantic_identity: canonical_digest(&semantic_lane(lane))
                        .map_err(anyhow::Error::msg)?,
                    controller_id: lane.controller_id.clone(),
                    scenario_id: lane.scenario_id.clone(),
                    launch: launch_evidence(lane),
                    class: lane.class.clone(),
                    end_reason: lane.end_reason.clone(),
                    physical_outcome: lane.physical_outcome.clone(),
                    mission_outcome: lane.mission_outcome.clone(),
                    terrain_contact_reconstruction_valid: lane.terrain_contact.reconstruction_valid,
                    waypoint_contract_pass: lane
                        .waypoint_contract
                        .as_ref()
                        .map(|value| value.contract_pass),
                    waypoint_contract_marker_count: lane
                        .waypoint_contract
                        .as_ref()
                        .map(|value| value.marker_count),
                    waypoint_contract_resolution_reason: lane
                        .waypoint_contract
                        .as_ref()
                        .and_then(|value| value.resolution_reason.clone()),
                    terrain_contact_within_derived_mesa_span: lane
                        .terrain_contact
                        .within_derived_mesa_span,
                })
            })
            .collect::<Result<Vec<_>>>()?,
        gate: case.gate.clone(),
        evidence_status: case.evidence_status.clone(),
    })
}

fn semantic_material(artifact: &ConservativeBallisticRidgeF5ControllerArtifactV1) -> Result<Value> {
    Ok(canonicalize_semantic_value(json!({
        "schema_id": artifact.schema_id,
        "schema_version": artifact.schema_version,
        "setup_id": artifact.setup_id,
        "scope": artifact.scope,
        "input_manifest_identity": artifact.input_manifest_identity,
        "prediction_manifest_identity": artifact.prediction_manifest_identity,
        "analytical_result_identity": artifact.analytical_result_identity,
        "analytical_artifact_identity": artifact.analytical_artifact_identity,
        "eligible_case_ids": artifact.eligible_case_ids,
        "analytically_ineligible_cases": artifact.analytically_ineligible_cases,
        "cases": artifact.cases.iter().map(|case| json!({
            "case_id": case.case_id,
            "input_identity": case.input_identity,
            "prediction_identity": case.prediction_identity,
            "analytical_case_identity": case.analytical_case_identity,
            "candidate_projection": case.candidate_projection,
            "runtime_v2": case.runtime_v2,
            "selected_attempt_index": case.selected_attempt_index,
            "selected_attempt": case.selected_attempt,
            "analytical_overlay": case.analytical_overlay,
            "lanes": case.lanes.iter().map(semantic_lane).collect::<Vec<_>>(),
            "gate": case.gate,
            "evidence_status": case.evidence_status,
        })).collect::<Vec<_>>(),
        "evidence_status": artifact.evidence_status,
        "deterministic_repeat": artifact.deterministic_repeat,
    })))
}

fn canonicalize_semantic_value(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(canonicalize_semantic_value)
                .collect(),
        ),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, canonicalize_semantic_value(value)))
                .collect(),
        ),
        Value::Number(number) => match number.as_f64() {
            Some(0.0) => Value::from(0),
            Some(value)
                if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 =>
            {
                Value::from(value as i64)
            }
            _ => Value::Number(number),
        },
        other => other,
    }
}

fn write_artifacts(
    output_dir: &Path,
    artifact: &ConservativeBallisticRidgeF5ControllerArtifactV1,
) -> Result<()> {
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create F5 controller output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    fs::write(&summary_path, serde_json::to_vec_pretty(artifact)?).with_context(|| {
        format!(
            "failed to write F5 controller summary {}",
            summary_path.display()
        )
    })?;
    for case in &artifact.cases {
        for lane in &case.lanes {
            let scenario = lane
                .scenario
                .as_ref()
                .ok_or_else(|| anyhow!("F5 lane {} lacks scenario evidence", lane.id))?;
            let controller = lane
                .controller
                .as_ref()
                .ok_or_else(|| anyhow!("F5 lane {} lacks controller evidence", lane.id))?;
            let run = lane
                .run
                .as_ref()
                .ok_or_else(|| anyhow!("F5 lane {} lacks run evidence", lane.id))?;
            let lane_dir = output_dir.join("runs").join(&case.case_id).join(&lane.id);
            fs::create_dir_all(&lane_dir)?;
            fs::write(
                lane_dir.join("scenario.json"),
                serde_json::to_vec_pretty(scenario)?,
            )?;
            fs::write(
                lane_dir.join("controller.json"),
                serde_json::to_vec_pretty(controller)?,
            )?;
            fs::write(lane_dir.join("run.json"), serde_json::to_vec_pretty(run)?)?;
        }
    }
    Ok(())
}

fn write_controller_result(
    path: &Path,
    result: &ConservativeBallisticRidgeF5ControllerResultV1,
) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(result)?;
    if path.exists() {
        let existing = fs::read(path).with_context(|| {
            format!(
                "failed to read existing F5 controller result {}",
                path.display()
            )
        })?;
        if existing != bytes {
            bail!(
                "existing F5 controller result {} does not exactly match fresh evidence",
                path.display()
            );
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create F5 controller result directory {}",
                parent.display()
            )
        })?;
    }
    fs::write(path, bytes)
        .with_context(|| format!("failed to write F5 controller result {}", path.display()))?;
    Ok(())
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
                .join(CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_SETUP_ID_V1)
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
            repo_root.join(CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_RESULT_FIXTURE_V1)
        })
}

fn resolve_report_path(repo_root: &Path, output_dir: &Path) -> Result<(PathBuf, bool)> {
    let site = ReportSite::new(repo_root);
    if let (Ok(outputs), Ok(output)) = (
        fs::canonicalize(repo_root.join("outputs")),
        fs::canonicalize(output_dir),
    ) && output.starts_with(outputs)
    {
        return Ok((
            site.default_output_for_bundle(output_dir).ok_or_else(|| {
                anyhow!("F5 controller output under outputs has no report bundle path")
            })?,
            true,
        ));
    }
    Ok((output_dir.join("report/index.html"), false))
}

fn flat_terrain(case: &DirectBridgeProbeV2) -> TerrainDefinition {
    TerrainDefinition::Heightfield {
        points_m: vec![
            case.terrain_points_m[0],
            *case
                .terrain_points_m
                .last()
                .expect("validated terrain endpoint"),
        ],
    }
}

fn mesa_terrain(mesa: &MesaGeometryV2) -> TerrainDefinition {
    TerrainDefinition::Heightfield {
        points_m: mesa.terrain_points_m.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_scenario(
    case: &DirectBridgeProbeV2,
    policy: &DirectBridgePolicyV2,
    vehicle_input: &VehicleInputV2,
    terrain: TerrainDefinition,
    route: TransferRouteSpec,
    lane_id: &str,
    description: &str,
) -> Result<ScenarioSpec> {
    let source = LandingPadSpec {
        id: "source".to_owned(),
        center_x_m: case.source.center_x_m,
        surface_y_m: case.source.surface_y_m,
        width_m: case.source.width_m,
    };
    let target = LandingPadSpec {
        id: "target".to_owned(),
        center_x_m: case.target.center_x_m,
        surface_y_m: case.target.surface_y_m,
        width_m: case.target.width_m,
    };
    let scenario = ScenarioSpec {
        id: format!("f5-controller-{}-{lane_id}", case.id),
        name: format!("F5 controller {} {lane_id}", case.id),
        description: description.to_owned(),
        seed: 0,
        tags: vec![
            "research".to_owned(),
            "held-out".to_owned(),
            "conservative-ballistic-f5-controller".to_owned(),
            lane_id.to_owned(),
        ],
        metadata: BTreeMap::from([
            ("source_case_id".to_owned(), case.id.clone()),
            (
                "controller_schema".to_owned(),
                CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_SCHEMA_ID_V1.to_owned(),
            ),
            ("lane_id".to_owned(), lane_id.to_owned()),
        ]),
        sim: SimConfig {
            physics_hz: crate::controller_shadow::CONTROLLER_SHADOW_PHYSICS_HZ,
            controller_hz: crate::controller_shadow::CONTROLLER_SHADOW_CONTROLLER_HZ,
            max_time_s: crate::controller_shadow::CONTROLLER_SHADOW_MAX_TIME_S,
            sample_hz: Some(crate::controller_shadow::CONTROLLER_SHADOW_PHYSICS_HZ),
        },
        world: WorldSpec {
            gravity_mps2: policy.gravity_mps2,
            terrain,
            landing_pads: vec![source, target],
        },
        vehicle: vehicle_from_fixture(vehicle_input),
        initial_state: VehicleInitialState {
            position_m: case.initial_position_m,
            velocity_mps: case.initial_velocity_mps,
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        },
        mission: MissionSpec {
            transfer_route: Some(route),
            goal: EvaluationGoal::LandingOnPad {
                target_pad_id: "target".to_owned(),
            },
        },
    };
    scenario.validate().map_err(|error| anyhow!(error))?;
    Ok(scenario)
}

fn vehicle_from_fixture(vehicle: &VehicleInputV2) -> VehicleSpec {
    VehicleSpec {
        geometry: VehicleGeometry {
            hull_width_m: vehicle.geometry.hull_width_m,
            hull_height_m: vehicle.geometry.hull_height_m,
            touchdown_half_span_m: vehicle.geometry.touchdown_half_span_m,
            touchdown_base_offset_m: vehicle.geometry.touchdown_base_offset_m,
        },
        dry_mass_kg: vehicle.dry_mass_kg,
        initial_fuel_kg: vehicle.initial_fuel_kg,
        max_fuel_kg: vehicle.max_fuel_kg,
        max_thrust_n: vehicle.max_thrust_n,
        max_fuel_burn_kgps: vehicle.max_fuel_burn_kgps,
        min_throttle_frac: vehicle.min_throttle_frac,
        max_rotation_rate_radps: vehicle.max_rotation_rate_radps,
        safe_touchdown_normal_speed_mps: vehicle.safe_touchdown_normal_speed_mps,
        safe_touchdown_tangential_speed_mps: vehicle.safe_touchdown_tangential_speed_mps,
        safe_touchdown_attitude_error_rad: vehicle.safe_touchdown_attitude_error_rad,
        safe_touchdown_angular_rate_radps: vehicle.safe_touchdown_angular_rate_radps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_plan::conservative_ballistic_bridge::load_experimental_ridge_case_manifest_v1;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("pd-eval-f5-controller-{label}-{nonce}"))
    }

    fn synthetic_compact_result_for_validation_only(
        prepared: &PreparedF5,
    ) -> ConservativeBallisticRidgeF5ControllerResultV1 {
        let case = prepared
            .cases
            .first()
            .expect("committed F5c has one controller-eligible case");
        let (waypoint_candidate_identity, handoff_selection_identity) = match (
            &case.candidate_projection.derived_mesa,
            &case.runtime_v2.derived_mesa,
        ) {
            (
                ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. },
                ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
                    handoff_selection, ..
                },
            ) => (
                candidate.identity.clone(),
                handoff_selection.identity.clone(),
            ),
            _ => panic!("prepared F5 source must retain one waypoint"),
        };
        let valid_launch = |lane_id: &str| F5ControllerLaunchEvidenceV1 {
            lane_id: lane_id.to_owned(),
            phase_takeoff: true,
            first_update_step_zero: true,
            upright_first_target: true,
            survived_source_launch: true,
            passes: true,
            reason: format!("{lane_id} launch preflight is valid"),
        };
        let mut lanes = vec![
            F5ControllerCompactLaneV1 {
                lane_id: "flat-direct".to_owned(),
                semantic_identity: "000000000000".to_owned(),
                controller_id: case.direct_controller.id().to_owned(),
                scenario_id: case.flat_scenario.id.clone(),
                launch: valid_launch("flat-direct"),
                class: ShadowLaneClass::TargetLanding,
                end_reason: Some(EndReason::TouchdownOnTarget),
                physical_outcome: Some(pd_core::PhysicalOutcome::LandedOnTarget),
                mission_outcome: Some(pd_core::MissionOutcome::Success),
                terrain_contact_reconstruction_valid: None,
                waypoint_contract_pass: None,
                waypoint_contract_marker_count: None,
                waypoint_contract_resolution_reason: None,
                terrain_contact_within_derived_mesa_span: false,
            },
            F5ControllerCompactLaneV1 {
                lane_id: "mesa-direct".to_owned(),
                semantic_identity: "000000000000".to_owned(),
                controller_id: case.direct_controller.id().to_owned(),
                scenario_id: case.mesa_direct_scenario.id.clone(),
                launch: valid_launch("mesa-direct"),
                class: ShadowLaneClass::TerrainCrash,
                end_reason: Some(EndReason::Crash),
                physical_outcome: Some(pd_core::PhysicalOutcome::Crashed),
                mission_outcome: Some(pd_core::MissionOutcome::FailedCrash),
                terrain_contact_reconstruction_valid: Some(true),
                waypoint_contract_pass: None,
                waypoint_contract_marker_count: None,
                waypoint_contract_resolution_reason: None,
                terrain_contact_within_derived_mesa_span: true,
            },
            F5ControllerCompactLaneV1 {
                lane_id: "mesa-waypoint".to_owned(),
                semantic_identity: "000000000000".to_owned(),
                controller_id: case.waypoint_controller.id().to_owned(),
                scenario_id: case.mesa_waypoint_scenario.id.clone(),
                launch: valid_launch("mesa-waypoint"),
                class: ShadowLaneClass::TargetLanding,
                end_reason: Some(EndReason::TouchdownOnTarget),
                physical_outcome: Some(pd_core::PhysicalOutcome::LandedOnTarget),
                mission_outcome: Some(pd_core::MissionOutcome::Success),
                terrain_contact_reconstruction_valid: None,
                waypoint_contract_pass: Some(true),
                waypoint_contract_marker_count: Some(1),
                waypoint_contract_resolution_reason: Some("contract_pass".to_owned()),
                terrain_contact_within_derived_mesa_span: false,
            },
        ];
        let gate = derive_compact_gate(&lanes, &case.controller_expectations).unwrap();
        let evidence_status = classify_evidence_status(&gate);
        let compact_case = ConservativeBallisticRidgeF5ControllerResultCaseV1 {
            case_id: case.case_id.clone(),
            input_identity: case.input_identity.clone(),
            prediction_identity: case.prediction_identity.clone(),
            analytical_case_identity: case.analytical_case_identity.clone(),
            candidate_projection_identity: case.candidate_projection.identity.clone(),
            selected_waypoint_candidate_identity: waypoint_candidate_identity,
            runtime_projection_identity: case.runtime_v2.identity.clone(),
            selected_attempt_index: case.selected_attempt_index,
            selected_attempt_identity: case.selected_attempt.identity.clone(),
            selected_handoff_selection_kind: case.selected_attempt.selection_kind,
            selected_handoff_selection_identity: handoff_selection_identity,
            selected_route_identity: canonical_digest(&case.selected_attempt.route).unwrap(),
            selected_waypoint_position_m: case.selected_attempt.selected_state.position_m,
            lanes: std::mem::take(&mut lanes),
            gate,
            evidence_status,
        };
        let mut result = ConservativeBallisticRidgeF5ControllerResultV1 {
            schema_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_RESULT_SCHEMA_ID_V1.to_owned(),
            schema_version: 1,
            scope: controller_scope(),
            analytical_result_identity: prepared.analytical_result_identity.clone(),
            analytical_artifact_identity: prepared.analytical_artifact_identity.clone(),
            input_manifest_identity: prepared.input_manifest_identity.clone(),
            prediction_manifest_identity: prepared.prediction_manifest_identity.clone(),
            eligible_case_ids: prepared
                .cases
                .iter()
                .map(|case| case.case_id.clone())
                .collect(),
            analytically_ineligible_cases: prepared.ineligible_cases.clone(),
            cases: vec![compact_case],
            evidence_status: F5ControllerEvidenceStatusV1::ControllerPredictionsMatched,
            deterministic_repeat: true,
            identity: String::new(),
        };
        result.identity = canonical_digest(&result).unwrap();
        result
    }

    fn rehash_compact_result(result: &mut ConservativeBallisticRidgeF5ControllerResultV1) {
        result.identity.clear();
        result.identity = canonical_digest(result).unwrap();
    }

    #[test]
    fn committed_f5c_evidence_derives_exactly_one_eligible_case_without_controller_execution() {
        let inputs = load_conservative_ballistic_ridge_f5_inputs_v1().unwrap();
        let predictions = load_conservative_ballistic_ridge_f5_predictions_v1().unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let result = load_conservative_ballistic_ridge_f5_analytical_result_v1(
            &root.join(crate::CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_FIXTURE_V1),
        )
        .unwrap();
        assert_eq!(
            result.identity,
            CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_IDENTITY_V1
        );
        let eligible = result
            .cases
            .iter()
            .filter(|case| case.eligible_for_controller)
            .map(|case| case.case_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(eligible, [F5_EXPECTED_ELIGIBLE_CASE_ID_V1]);
        let stopped = result
            .cases
            .iter()
            .filter(|case| !case.eligible_for_controller)
            .map(|case| case.case_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(stopped, [F5_EXPECTED_INELIGIBLE_CASE_ID_V1]);
        assert_eq!(inputs.cases.len(), predictions.cases.len());
        let (eligible_indices, ineligible) = select_controller_eligibility(&result).unwrap();
        assert_eq!(eligible_indices, [0]);
        assert_eq!(ineligible.len(), 1);
        assert!(!ineligible[0].controller_lanes_executed);
        // This test never evaluates candidate/runtime code or calls run_lane.
    }

    #[test]
    fn f5c_identity_or_eligibility_tampering_is_rejected_before_executor_selection() {
        let inputs = load_conservative_ballistic_ridge_f5_inputs_v1().unwrap();
        let predictions = load_conservative_ballistic_ridge_f5_predictions_v1().unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let result = load_conservative_ballistic_ridge_f5_analytical_result_v1(
            &root.join(crate::CONSERVATIVE_BALLISTIC_RIDGE_F5_ANALYTICAL_RESULT_FIXTURE_V1),
        )
        .unwrap();
        let mut wrong_identity = result.clone();
        wrong_identity.identity.push('x');
        assert!(
            wrong_identity
                .validate_against_seals(&inputs, &predictions)
                .is_err()
        );
        let mut wrong_eligibility = result.clone();
        wrong_eligibility.cases[1].eligible_for_controller = true;
        assert!(select_controller_eligibility(&wrong_eligibility).is_err());
    }

    #[test]
    fn prediction_mismatch_and_invalid_setup_have_distinct_classifications() {
        let invalid_gate = F5ControllerGateEvidenceV1 {
            launches: vec![],
            all_lane_launch_valid: F5ControllerGateCheckV1 {
                passes: false,
                reason: "launch".to_owned(),
            },
            flat_target_landing: F5ControllerGateCheckV1 {
                passes: false,
                reason: "flat".to_owned(),
            },
            mesa_direct_non_target_terrain_contact_before_target_touchdown:
                F5ControllerGateCheckV1 {
                    passes: false,
                    reason: "mesa".to_owned(),
                },
            waypoint_exactly_one_captured_contract_pass: F5ControllerGateCheckV1 {
                passes: false,
                reason: "capture".to_owned(),
            },
            waypoint_target_landing: F5ControllerGateCheckV1 {
                passes: false,
                reason: "waypoint".to_owned(),
            },
            controller_predictions_matched: F5ControllerGateCheckV1 {
                passes: false,
                reason: "all".to_owned(),
            },
        };
        assert_eq!(
            classify_evidence_status(&invalid_gate),
            F5ControllerEvidenceStatusV1::InvalidSetup
        );
        let mut mismatch_gate = invalid_gate;
        mismatch_gate.all_lane_launch_valid.passes = true;
        assert_eq!(
            classify_evidence_status(&mismatch_gate),
            F5ControllerEvidenceStatusV1::ControllerPredictionMismatch
        );
    }

    #[test]
    fn compact_result_rejects_self_rehashed_provenance_wiring_and_gate_tampering() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let prepared = prepare_committed_f5(root).unwrap();
        let result = synthetic_compact_result_for_validation_only(&prepared);
        result.validate_against_source(root).unwrap();

        // No captured waypoint marker is a completed-lane prediction mismatch,
        // not invalid setup. This is a synthetic DTO only: it never calls
        // run_lane or writes a held-out F5 artifact.
        let mut mismatch = result.clone();
        mismatch.cases[0].lanes[2].class = ShadowLaneClass::Inconclusive;
        mismatch.cases[0].lanes[2].waypoint_contract_pass = None;
        mismatch.cases[0].lanes[2].waypoint_contract_marker_count = None;
        mismatch.cases[0].lanes[2].waypoint_contract_resolution_reason = None;
        mismatch.cases[0].gate = derive_compact_gate(
            &mismatch.cases[0].lanes,
            &prepared.cases[0].controller_expectations,
        )
        .unwrap();
        mismatch.cases[0].evidence_status = classify_evidence_status(&mismatch.cases[0].gate);
        mismatch.evidence_status = aggregate_compact_evidence_status(&mismatch.cases);
        rehash_compact_result(&mut mismatch);
        assert_eq!(
            mismatch.evidence_status,
            F5ControllerEvidenceStatusV1::ControllerPredictionMismatch
        );
        mismatch.validate_against_source(root).unwrap();

        let mut attempt = result.clone();
        attempt.cases[0]
            .selected_attempt_identity
            .push_str("-tampered");
        rehash_compact_result(&mut attempt);
        assert!(attempt.validate_against_source(root).is_err());

        let mut route = result.clone();
        route.cases[0].selected_route_identity.push_str("-tampered");
        rehash_compact_result(&mut route);
        assert!(route.validate_against_source(root).is_err());

        let mut scenario = result.clone();
        scenario.cases[0].lanes[2].scenario_id.push_str("-tampered");
        rehash_compact_result(&mut scenario);
        assert!(scenario.validate_against_source(root).is_err());

        let mut semantic_format = result.clone();
        semantic_format.cases[0].lanes[0].semantic_identity = "fnv1a64:0000000000000000".to_owned();
        rehash_compact_result(&mut semantic_format);
        assert!(semantic_format.validate_against_source(root).is_err());

        let mut gate = result.clone();
        gate.cases[0].gate.flat_target_landing.passes = false;
        rehash_compact_result(&mut gate);
        assert!(gate.validate_against_source(root).is_err());

        let mut marker = result.clone();
        marker.cases[0].lanes[2].waypoint_contract_marker_count = Some(2);
        rehash_compact_result(&mut marker);
        assert!(marker.validate_against_source(root).is_err());

        let mut launch = result.clone();
        launch.cases[0].gate.launches.swap(0, 1);
        rehash_compact_result(&mut launch);
        assert!(launch.validate_against_source(root).is_err());
    }

    #[test]
    fn scenario_builder_preserves_exposed_068_route_and_frozen_controller_timing() {
        let manifest = load_experimental_ridge_case_manifest_v1();
        let input = manifest
            .cases
            .iter()
            .find(|case| case.probe.id == "ridge_progress_068_probe")
            .unwrap();
        let projection = evaluate_experimental_ridge_case_projection_v1(input).unwrap();
        let runtime = project_experimental_ridge_case_runtime_v2(&projection).unwrap();
        let route = match &runtime.flat_control {
            ExperimentalRidgeCaseRuntimeOutcomeV2::Direct { route, .. } => route.clone(),
            _ => panic!("exposed 068 flat runtime must remain direct"),
        };
        let (waypoint_route, selected_waypoint) = match &runtime.derived_mesa {
            ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
                route,
                handoff_selection,
                ..
            } => {
                let selected = handoff_selection.selected_attempt().unwrap();
                assert_eq!(selected.route, *route);
                (route.clone(), selected.selected_state.position_m)
            }
            _ => panic!("exposed 068 mesa runtime must retain one waypoint"),
        };
        let scenario = build_scenario(
            &input.probe,
            &projection.policy,
            &projection.vehicle,
            flat_terrain(&input.probe),
            route.clone(),
            "flat-direct",
            "synthetic pre-reveal wiring test",
        )
        .unwrap();
        let waypoint_scenario = build_scenario(
            &input.probe,
            &projection.policy,
            &projection.vehicle,
            mesa_terrain(&projection.mesa),
            waypoint_route.clone(),
            "mesa-waypoint",
            "synthetic pre-reveal waypoint wiring test",
        )
        .unwrap();
        assert_eq!(scenario.mission.transfer_route, Some(route));
        assert_eq!(
            waypoint_scenario.mission.transfer_route,
            Some(waypoint_route)
        );
        assert_eq!(
            waypoint_scenario
                .mission
                .transfer_route
                .as_ref()
                .unwrap()
                .waypoints[0]
                .position_m,
            selected_waypoint
        );
        assert_eq!(
            scenario.sim.physics_hz,
            crate::controller_shadow::CONTROLLER_SHADOW_PHYSICS_HZ
        );
        assert_eq!(
            scenario.sim.controller_hz,
            crate::controller_shadow::CONTROLLER_SHADOW_CONTROLLER_HZ
        );
        assert_eq!(
            scenario.sim.max_time_s,
            crate::controller_shadow::CONTROLLER_SHADOW_MAX_TIME_S
        );
        scenario.validate().unwrap();
        waypoint_scenario.validate().unwrap();
        // This uses only exposed development input and never calls run_lane.
    }

    #[test]
    fn immutable_compact_writer_accepts_exact_bytes_and_rejects_differences() {
        let root = temp_root("immutable");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("result.json");
        let result = ConservativeBallisticRidgeF5ControllerResultV1 {
            schema_id: CONSERVATIVE_BALLISTIC_RIDGE_F5_CONTROLLER_RESULT_SCHEMA_ID_V1.to_owned(),
            schema_version: 1,
            scope: controller_scope(),
            analytical_result_identity: "analytical".to_owned(),
            analytical_artifact_identity: "artifact".to_owned(),
            input_manifest_identity: "input".to_owned(),
            prediction_manifest_identity: "prediction".to_owned(),
            eligible_case_ids: vec![],
            analytically_ineligible_cases: vec![],
            cases: vec![],
            evidence_status: F5ControllerEvidenceStatusV1::ControllerPredictionsMatched,
            deterministic_repeat: true,
            identity: String::new(),
        };
        write_controller_result(&path, &result).unwrap();
        write_controller_result(&path, &result).unwrap();
        std::fs::write(&path, b"different").unwrap();
        assert!(write_controller_result(&path, &result).is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
