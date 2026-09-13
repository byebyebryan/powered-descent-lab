//! Controller-free development regression for the generic ridge runtime V2.
//!
//! The two cases in the bound input manifest are exposed development inputs,
//! not a held-out result lane.  This module recomputes the candidate
//! projection, records the historical crossing-only V1 result, evaluates the
//! new bounded two-state runtime V2 selector, and writes only a reloadable
//! summary plus its display-only report.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_plan::conservative_ballistic_bridge::{
    ExperimentalRidgeCandidateOutcomeV2, ExperimentalRidgeCaseInputV1,
    ExperimentalRidgeCaseManifestV1, ExperimentalRidgeCaseProjectionV1,
    ExperimentalRidgeCaseRuntimeOutcomeV2, ExperimentalRidgeCaseRuntimeProjectionV1,
    ExperimentalRidgeCaseRuntimeProjectionV2, ExperimentalRidgeRuntimeHandoffSelectionKindV2,
    ExperimentalRidgeRuntimeOutcomeV2, ExperimentalRidgeRuntimeProjectionErrorV2,
    evaluate_experimental_ridge_case_projection_v1, load_experimental_ridge_case_manifest_v1,
    project_experimental_ridge_case_runtime_v1, project_experimental_ridge_case_runtime_v2,
    validate_experimental_ridge_case_projection_v1, validate_experimental_ridge_case_runtime_v2,
};
use pd_report::site::ReportSite;
use serde::{Deserialize, Serialize};

use crate::canonical_digest;

pub const CONSERVATIVE_BALLISTIC_HANDOFF_DEVELOPMENT_SCHEMA_ID_V1: &str =
    "conservative-ballistic-handoff-development-v1";
pub const CONSERVATIVE_BALLISTIC_HANDOFF_DEVELOPMENT_SCHEMA_VERSION_V1: u32 = 1;
pub const CONSERVATIVE_BALLISTIC_HANDOFF_DEVELOPMENT_SETUP_ID_V1: &str =
    "conservative-ballistic-handoff-development-v1";

/// Explicit scope binding for this artifact.  These inputs exercise planner
/// evidence only; no controller, simulator, or physical execution is part of
/// the development regression.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticHandoffDevelopmentScopeV1 {
    pub controller_run: bool,
    pub simulation_run: bool,
    pub physical_execution_run: bool,
    pub non_claims: Vec<String>,
}

fn development_scope() -> ConservativeBallisticHandoffDevelopmentScopeV1 {
    ConservativeBallisticHandoffDevelopmentScopeV1 {
        controller_run: false,
        simulation_run: false,
        physical_execution_run: false,
        non_claims: vec![
            "exposed development inputs are not held-out evidence".to_owned(),
            "historical H2/H4 STOP behavior remains unchanged".to_owned(),
            "analytical pass is not a controller or physical-execution result".to_owned(),
        ],
    }
}

/// The historical generic V1 runtime result is retained verbatim: successful
/// cases carry the old projection, while a stopped case carries its exact
/// typed error.  This keeps the crossing-only behavior auditable next to V2.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ConservativeBallisticHistoricalRuntimeV1 {
    Passed {
        projection: Box<ExperimentalRidgeCaseRuntimeProjectionV1>,
    },
    Failed {
        error: ExperimentalRidgeRuntimeProjectionErrorV2,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticHandoffDevelopmentCaseV1 {
    pub case_id: String,
    pub input_identity: String,
    pub input: ExperimentalRidgeCaseInputV1,
    pub candidate_projection: ExperimentalRidgeCaseProjectionV1,
    pub historical_v1: ConservativeBallisticHistoricalRuntimeV1,
    pub runtime_v2: ExperimentalRidgeCaseRuntimeProjectionV2,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticHandoffDevelopmentArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub setup_id: String,
    pub input_manifest_identity: String,
    pub case_ids: Vec<String>,
    pub deterministic_repeat: bool,
    pub scope: ConservativeBallisticHandoffDevelopmentScopeV1,
    pub cases: Vec<ConservativeBallisticHandoffDevelopmentCaseV1>,
    pub identity: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConservativeBallisticHandoffDevelopmentPathsV1 {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
    pub report_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ConservativeBallisticHandoffDevelopmentRunV1 {
    pub artifact: ConservativeBallisticHandoffDevelopmentArtifactV1,
    pub paths: ConservativeBallisticHandoffDevelopmentPathsV1,
}

/// Run the controller-free development regression over both exposed ridge
/// inputs.  The default summary path is ignored `outputs/eval/.../summary.json`;
/// no source-controlled result manifest is created.
pub fn run_conservative_ballistic_handoff_development(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
) -> Result<ConservativeBallisticHandoffDevelopmentRunV1> {
    let input_manifest = load_experimental_ridge_case_manifest_v1();
    input_manifest
        .validate()
        .map_err(|error| anyhow!("development input manifest is invalid: {error}"))?;

    let mut first = build_artifact(&input_manifest)?;
    let mut repeat = build_artifact(&input_manifest)?;
    first.deterministic_repeat = true;
    repeat.deterministic_repeat = true;
    first.finalize_identity()?;
    repeat.finalize_identity()?;
    let first_bytes = serde_json::to_vec_pretty(&first)?;
    let repeat_bytes = serde_json::to_vec_pretty(&repeat)?;
    if first_bytes != repeat_bytes {
        bail!("development handoff regression repeat is not byte-identical");
    }
    first.validate_against(&input_manifest)?;

    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create handoff development output directory {}",
            output_dir.display()
        )
    })?;
    let summary_path = output_dir.join("summary.json");
    fs::write(&summary_path, &first_bytes).with_context(|| {
        format!(
            "failed to write handoff development summary {}",
            summary_path.display()
        )
    })?;
    let reloaded = load_conservative_ballistic_handoff_development_artifact_v1(
        &summary_path,
        &input_manifest,
    )?;
    if serde_json::to_vec_pretty(&reloaded)? != first_bytes {
        bail!("development handoff summary is not byte-stable after reload");
    }

    let (report_path, update_site) = resolve_report_path(repo_root, &output_dir)?;
    let report_data = serde_json::to_value(&reloaded)?;
    pd_report::conservative_ballistic_handoff_development::write_conservative_ballistic_handoff_development_report(
        &report_path,
        &report_data,
    )?;
    if update_site {
        ReportSite::new(repo_root).update_indexes_for_file(&report_path)?;
    }

    Ok(ConservativeBallisticHandoffDevelopmentRunV1 {
        artifact: reloaded,
        paths: ConservativeBallisticHandoffDevelopmentPathsV1 {
            output_dir,
            summary_path,
            report_path,
        },
    })
}

pub fn load_conservative_ballistic_handoff_development_artifact_v1(
    path: &Path,
    input_manifest: &ExperimentalRidgeCaseManifestV1,
) -> Result<ConservativeBallisticHandoffDevelopmentArtifactV1> {
    let artifact: ConservativeBallisticHandoffDevelopmentArtifactV1 =
        serde_json::from_slice(&fs::read(path).with_context(|| {
            format!(
                "failed to read handoff development summary {}",
                path.display()
            )
        })?)
        .with_context(|| {
            format!(
                "failed to parse handoff development summary {}",
                path.display()
            )
        })?;
    artifact.validate_against(input_manifest)?;
    Ok(artifact)
}

fn build_artifact(
    input_manifest: &ExperimentalRidgeCaseManifestV1,
) -> Result<ConservativeBallisticHandoffDevelopmentArtifactV1> {
    let cases = input_manifest
        .cases
        .iter()
        .map(build_case)
        .collect::<Result<Vec<_>>>()?;
    let mut artifact = ConservativeBallisticHandoffDevelopmentArtifactV1 {
        schema_id: CONSERVATIVE_BALLISTIC_HANDOFF_DEVELOPMENT_SCHEMA_ID_V1.to_owned(),
        schema_version: CONSERVATIVE_BALLISTIC_HANDOFF_DEVELOPMENT_SCHEMA_VERSION_V1,
        setup_id: CONSERVATIVE_BALLISTIC_HANDOFF_DEVELOPMENT_SETUP_ID_V1.to_owned(),
        input_manifest_identity: input_manifest.identity.clone(),
        case_ids: cases.iter().map(|case| case.case_id.clone()).collect(),
        deterministic_repeat: false,
        scope: development_scope(),
        cases,
        identity: String::new(),
    };
    artifact.finalize_identity()?;
    Ok(artifact)
}

fn build_case(
    input: &ExperimentalRidgeCaseInputV1,
) -> Result<ConservativeBallisticHandoffDevelopmentCaseV1> {
    let candidate_projection = evaluate_experimental_ridge_case_projection_v1(input)
        .map_err(|error| anyhow!("{} candidate projection failed: {error}", input.probe.id))?;
    validate_experimental_ridge_case_projection_v1(&candidate_projection).map_err(|error| {
        anyhow!(
            "{} candidate projection did not validate: {error}",
            input.probe.id
        )
    })?;
    let historical_v1 = match project_experimental_ridge_case_runtime_v1(&candidate_projection) {
        Ok(projection) => ConservativeBallisticHistoricalRuntimeV1::Passed {
            projection: Box::new(projection),
        },
        Err(error) => ConservativeBallisticHistoricalRuntimeV1::Failed { error },
    };
    let runtime_v2 = project_experimental_ridge_case_runtime_v2(&candidate_projection)
        .map_err(|error| anyhow!("{} V2 runtime projection failed: {error}", input.probe.id))?;
    validate_experimental_ridge_case_runtime_v2(&candidate_projection, &runtime_v2).map_err(
        |error| {
            anyhow!(
                "{} V2 runtime projection did not validate: {error}",
                input.probe.id
            )
        },
    )?;

    let mut case = ConservativeBallisticHandoffDevelopmentCaseV1 {
        case_id: input.probe.id.clone(),
        input_identity: input.identity.clone(),
        input: input.clone(),
        candidate_projection,
        historical_v1,
        runtime_v2,
        identity: String::new(),
    };
    validate_case_semantics(&case)?;
    case.identity = case_identity(&case)?;
    Ok(case)
}

impl ConservativeBallisticHandoffDevelopmentArtifactV1 {
    fn finalize_identity(&mut self) -> Result<()> {
        self.identity.clear();
        self.identity = canonical_digest(self).map_err(anyhow::Error::msg)?;
        Ok(())
    }

    pub fn validate_against(&self, input_manifest: &ExperimentalRidgeCaseManifestV1) -> Result<()> {
        input_manifest
            .validate()
            .map_err(|error| anyhow!("development input manifest is invalid: {error}"))?;
        if self.schema_id != CONSERVATIVE_BALLISTIC_HANDOFF_DEVELOPMENT_SCHEMA_ID_V1
            || self.schema_version != CONSERVATIVE_BALLISTIC_HANDOFF_DEVELOPMENT_SCHEMA_VERSION_V1
            || self.setup_id != CONSERVATIVE_BALLISTIC_HANDOFF_DEVELOPMENT_SETUP_ID_V1
            || !self.deterministic_repeat
            || self.input_manifest_identity != input_manifest.identity
            || self.scope != development_scope()
            || self.cases.len() != input_manifest.cases.len()
            || self.case_ids
                != input_manifest
                    .cases
                    .iter()
                    .map(|case| case.probe.id.clone())
                    .collect::<Vec<_>>()
        {
            bail!("handoff development artifact schema, scope, or manifest binding is invalid");
        }
        let mut ids = BTreeSet::new();
        for (case, input) in self.cases.iter().zip(&input_manifest.cases) {
            if !ids.insert(case.case_id.clone())
                || case.case_id != input.probe.id
                || case.input_identity != input.identity
                || case.input != *input
            {
                bail!("handoff development case order or input binding is invalid");
            }
            let expected = build_case(input)?;
            if case != &expected {
                bail!(
                    "handoff development case {} does not recompute",
                    case.case_id
                );
            }
        }
        let mut material = self.clone();
        material.identity.clear();
        if self.identity.is_empty()
            || self.identity != canonical_digest(&material).map_err(anyhow::Error::msg)?
        {
            bail!("handoff development artifact identity does not bind contents");
        }
        Ok(())
    }
}

fn case_identity(case: &ConservativeBallisticHandoffDevelopmentCaseV1) -> Result<String> {
    let mut material = case.clone();
    material.identity.clear();
    canonical_digest(&material).map_err(anyhow::Error::msg)
}

fn validate_case_semantics(case: &ConservativeBallisticHandoffDevelopmentCaseV1) -> Result<()> {
    match case.case_id.as_str() {
        "ridge_progress_050_probe" => validate_050(case),
        "ridge_progress_068_probe" => validate_068(case),
        other => bail!("unexpected development case {other}"),
    }
}

fn validate_050(case: &ConservativeBallisticHandoffDevelopmentCaseV1) -> Result<()> {
    let ConservativeBallisticHistoricalRuntimeV1::Passed {
        projection: historical,
    } = &case.historical_v1
    else {
        bail!("050 historical V1 runtime must pass");
    };
    let ExperimentalRidgeRuntimeOutcomeV2::OneWaypoint {
        route: historical_route,
        crossing: historical_crossing,
        authority: historical_authority,
        handoff_kinematics: historical_kinematics,
        handoff_assessment: historical_assessment,
        ..
    } = &historical.derived_mesa
    else {
        bail!("050 historical V1 runtime must retain one waypoint");
    };
    let ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
        route,
        crossing,
        authority,
        handoff_kinematics,
        handoff_assessment,
        handoff_selection,
        ..
    } = &case.runtime_v2.derived_mesa
    else {
        bail!("050 V2 runtime must retain one waypoint");
    };
    if route != historical_route
        || crossing != historical_crossing
        || authority != historical_authority
        || handoff_kinematics != historical_kinematics
        || handoff_assessment != historical_assessment
        || handoff_selection.attempts.len() != 1
        || handoff_selection.selected_attempt_index != 0
    {
        bail!("050 V2 primary selection is not exact-compatible with V1");
    }
    let selected = handoff_selection
        .selected_attempt()
        .ok_or_else(|| anyhow!("050 V2 selected attempt index is invalid"))?;
    if selected.selection_kind != ExperimentalRidgeRuntimeHandoffSelectionKindV2::PrimaryCrossing
        || selected.target_leg_arc_step.is_some()
        || selected.route != *historical_route
        || selected.authority != *historical_authority
        || selected.handoff_kinematics != *historical_kinematics
        || selected.handoff_assessment != *historical_assessment
        || selected.selected_state != crossing.selected_state
        || selected.applied_steps != crossing.selected_applied_steps
    {
        bail!("050 V2 primary attempt is not exact-compatible with V1");
    }
    Ok(())
}

fn validate_068(case: &ConservativeBallisticHandoffDevelopmentCaseV1) -> Result<()> {
    let ConservativeBallisticHistoricalRuntimeV1::Failed { error } = &case.historical_v1 else {
        bail!("068 historical V1 runtime must stop");
    };
    if error != &ExperimentalRidgeRuntimeProjectionErrorV2::HandoffContractFailed {
        bail!("068 historical V1 runtime stop reason changed: {error}");
    }
    let ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
        candidate,
        crossing: _,
    } = &case.candidate_projection.derived_mesa
    else {
        bail!("068 candidate projection must retain one waypoint");
    };
    let ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
        handoff_selection, ..
    } = &case.runtime_v2.derived_mesa
    else {
        bail!("068 V2 runtime must retain one waypoint");
    };
    if handoff_selection.attempts.len() != 2 || handoff_selection.selected_attempt_index != 1 {
        bail!("068 V2 must select its second bounded handoff attempt");
    }
    let primary = &handoff_selection.attempts[0];
    if primary.selection_kind != ExperimentalRidgeRuntimeHandoffSelectionKindV2::PrimaryCrossing
        || primary.applied_steps != 1406
        || primary.target_leg_arc_step.is_some()
        || primary.handoff_assessment.violations != vec!["heading".to_owned()]
        || !close(
            primary.handoff_kinematics.outbound_heading_error_rad,
            0.3879286123780121,
        )
    {
        bail!("068 primary crossing evidence changed");
    }
    let selected = handoff_selection
        .selected_attempt()
        .ok_or_else(|| anyhow!("068 V2 selected attempt index is invalid"))?;
    if selected.selection_kind
        != ExperimentalRidgeRuntimeHandoffSelectionKindV2::IntermediateBridgeExit
        || selected.applied_steps != 3960
        || selected.target_leg_arc_step != Some(candidate.target_leg.apex_step)
        || !selected.handoff_assessment.contract_pass
        || !selected.handoff_assessment.violations.is_empty()
        || !close(selected.selected_state.position_m.x, 2715.400408105346)
        || !close(selected.selected_state.position_m.y, 1931.4569053495914)
        || !close(selected.selected_state.velocity_mps.x, 64.82420144127775)
        || !close(selected.selected_state.velocity_mps.y, 0.027651643418408867)
        || !close(selected.authority.handoff_speed_cap_mps, 93.2925267109143)
        || !close(
            selected.handoff_kinematics.outbound_heading_error_rad,
            0.19282817224525745,
        )
        || !close(
            selected.handoff_kinematics.outbound_cross_speed_mps,
            12.4226137521142,
        )
        || !close(selected.handoff_kinematics.speed_mps, 64.8242073388695)
    {
        bail!("068 bridge-exit fallback evidence changed");
    }
    Ok(())
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 1.0e-12
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
                .join(CONSERVATIVE_BALLISTIC_HANDOFF_DEVELOPMENT_SETUP_ID_V1)
        })
}

fn resolve_report_path(repo_root: &Path, output_dir: &Path) -> Result<(PathBuf, bool)> {
    let site = ReportSite::new(repo_root);
    if let (Ok(outputs), Ok(output)) = (
        fs::canonicalize(repo_root.join("outputs")),
        fs::canonicalize(output_dir),
    ) && output.starts_with(outputs)
    {
        let report_path = site
            .default_output_for_bundle(output_dir)
            .ok_or_else(|| anyhow!("development output must be under repository outputs"))?;
        return Ok((report_path, true));
    }
    Ok((output_dir.join("report/index.html"), false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("pd-eval-handoff-development-{label}-{nonce}"))
    }

    #[test]
    fn development_artifact_has_exact_exposed_case_semantics_and_reload_identity() {
        let manifest = load_experimental_ridge_case_manifest_v1();
        let first = build_artifact(&manifest).unwrap();
        let mut artifact = first.clone();
        artifact.deterministic_repeat = true;
        artifact.finalize_identity().unwrap();
        let bytes = serde_json::to_vec_pretty(&artifact).unwrap();
        let reloaded: ConservativeBallisticHandoffDevelopmentArtifactV1 =
            serde_json::from_slice(&bytes).unwrap();
        reloaded.validate_against(&manifest).unwrap();
        assert_eq!(bytes, serde_json::to_vec_pretty(&reloaded).unwrap());
        assert_eq!(reloaded.cases.len(), 2);
        assert_eq!(reloaded.cases[0].case_id, "ridge_progress_050_probe");
        assert_eq!(reloaded.cases[1].case_id, "ridge_progress_068_probe");
    }

    #[test]
    fn development_run_writes_only_summary_and_report_paths() {
        let root = temp_root("run");
        let run = run_conservative_ballistic_handoff_development(&root, None).unwrap();
        assert_eq!(
            run.paths.output_dir,
            root.join("outputs/eval/conservative-ballistic-handoff-development-v1")
        );
        assert_eq!(
            run.paths.report_path,
            root.join(
                "outputs/reports/eval/conservative-ballistic-handoff-development-v1/index.html"
            )
        );
        assert!(run.paths.summary_path.is_file());
        assert!(run.paths.report_path.is_file());
        let entries = fs::read_dir(&run.paths.output_dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(entries, vec!["summary.json"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn development_artifact_rejects_case_tampering() {
        let manifest = load_experimental_ridge_case_manifest_v1();
        let mut artifact = build_artifact(&manifest).unwrap();
        artifact.deterministic_repeat = true;
        artifact.finalize_identity().unwrap();
        artifact.cases[1].runtime_v2.identity.push_str("-tampered");
        assert!(artifact.validate_against(&manifest).is_err());
    }
}
