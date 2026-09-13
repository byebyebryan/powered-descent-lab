//! Development-only full-controller execution check for the generic V2 ridge
//! handoff selector.
//!
//! This is deliberately a new artifact boundary.  It consumes the generic
//! case input and runtime projection, executes the existing controllers with
//! their frozen simulator configuration, and records either result without
//! changing planner policy, controller gains, or the frozen controller shadow.

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
    ExperimentalRidgeCaseInputV1, ExperimentalRidgeCaseManifestV1,
    ExperimentalRidgeCaseProjectionV1, ExperimentalRidgeCaseRuntimeOutcomeV2,
    ExperimentalRidgeCaseRuntimeProjectionV2, ExperimentalRidgeRuntimeHandoffAttemptV2,
    ExperimentalRidgeRuntimeHandoffSelectionKindV2, MesaGeometryV2, VehicleInputV2,
    evaluate_experimental_ridge_case_projection_v1, load_experimental_ridge_case_manifest_v1,
    project_experimental_ridge_case_runtime_v2, validate_experimental_ridge_case_projection_v1,
    validate_experimental_ridge_case_runtime_v2,
};
use pd_report::site::ReportSite;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    canonical_digest,
    controller_shadow::{
        AnalyticalShadowOverlay, ShadowLaneSummary, build_analytical_overlay, run_lane,
        semantic_lane,
    },
};

pub const CONSERVATIVE_BALLISTIC_HANDOFF_CONTROLLER_DEVELOPMENT_SCHEMA_ID_V1: &str =
    "conservative-ballistic-handoff-controller-development-v1";
pub const CONSERVATIVE_BALLISTIC_HANDOFF_CONTROLLER_DEVELOPMENT_SCHEMA_VERSION_V1: u32 = 1;
pub const CONSERVATIVE_BALLISTIC_HANDOFF_CONTROLLER_DEVELOPMENT_SETUP_ID_V1: &str =
    "conservative-ballistic-handoff-controller-development-v1";
const F4_CASE_ID: &str = "ridge_progress_068_probe";
const F4_INPUT_IDENTITY: &str = "fnv1a64:a63ff1716b1cf24f";
const F4_CANDIDATE_IDENTITY: &str = "fnv1a64:1ddf250808889d3f";
const F4_RUNTIME_IDENTITY: &str = "fnv1a64:48e2f9d91222c096";

/// Explicitly bound scope.  This artifact is a deterministic simulator and
/// controller observation, never physical proof or held-out authority.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticHandoffControllerDevelopmentScopeV1 {
    pub controller_run: bool,
    pub simulation_run: bool,
    pub physical_execution_run: bool,
    pub non_claims: Vec<String>,
}

fn development_scope() -> ConservativeBallisticHandoffControllerDevelopmentScopeV1 {
    ConservativeBallisticHandoffControllerDevelopmentScopeV1 {
        controller_run: true,
        simulation_run: true,
        physical_execution_run: false,
        non_claims: vec![
            "ridge_progress_068_probe is an exposed development input, not held-out evidence"
                .to_owned(),
            "the frozen controller-shadow-v4 artifact and its acceptance identity are unchanged"
                .to_owned(),
            "a simulator/controller observation is not physical-execution evidence".to_owned(),
            "this artifact does not tune the planner, handoff contract, or controller".to_owned(),
        ],
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticHandoffControllerGateCheckV1 {
    pub passes: bool,
    pub reason: String,
}

/// Per-lane, field-level launch evidence.  Keep the four predicates separate
/// so a source-step setup error is distinguishable from a later controller
/// result in the sealed artifact and report data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticHandoffControllerLaunchEvidenceV1 {
    pub lane_id: String,
    pub phase_takeoff: bool,
    pub first_update_step_zero: bool,
    pub upright_first_target: bool,
    pub survived_source_launch: bool,
    pub passes: bool,
    pub reason: String,
}

/// The individual F4 gates are intentionally independent so an unexpected
/// physical result remains evidence rather than turning the command into a
/// success-only assertion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticHandoffControllerGateEvidenceV1 {
    pub launches: Vec<ConservativeBallisticHandoffControllerLaunchEvidenceV1>,
    pub all_lane_launch_valid: ConservativeBallisticHandoffControllerGateCheckV1,
    pub flat_target_landing: ConservativeBallisticHandoffControllerGateCheckV1,
    pub mesa_direct_ridge_span_crash: ConservativeBallisticHandoffControllerGateCheckV1,
    pub waypoint_exactly_one_captured_contract_pass:
        ConservativeBallisticHandoffControllerGateCheckV1,
    pub waypoint_target_landing: ConservativeBallisticHandoffControllerGateCheckV1,
    pub overall_green: ConservativeBallisticHandoffControllerGateCheckV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticHandoffControllerDevelopmentArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub setup_id: String,
    pub scope: ConservativeBallisticHandoffControllerDevelopmentScopeV1,
    pub input_manifest_identity: String,
    pub case_id: String,
    pub input_identity: String,
    pub input: ExperimentalRidgeCaseInputV1,
    pub candidate_projection: ExperimentalRidgeCaseProjectionV1,
    pub runtime_v2: ExperimentalRidgeCaseRuntimeProjectionV2,
    pub selected_attempt_index: usize,
    pub selected_attempt_identity: String,
    pub analytical_overlay: AnalyticalShadowOverlay,
    pub lanes: Vec<ShadowLaneSummary>,
    pub gate: ConservativeBallisticHandoffControllerGateEvidenceV1,
    pub deterministic_repeat: bool,
    pub identity: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConservativeBallisticHandoffControllerDevelopmentPathsV1 {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
    pub report_path: PathBuf,
    pub preview_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ConservativeBallisticHandoffControllerDevelopmentRunV1 {
    pub artifact: ConservativeBallisticHandoffControllerDevelopmentArtifactV1,
    pub paths: ConservativeBallisticHandoffControllerDevelopmentPathsV1,
}

#[derive(Clone)]
struct PreparedF4 {
    input_manifest_identity: String,
    input: ExperimentalRidgeCaseInputV1,
    candidate_projection: ExperimentalRidgeCaseProjectionV1,
    runtime_v2: ExperimentalRidgeCaseRuntimeProjectionV2,
    selected_attempt_index: usize,
    selected_attempt: ExperimentalRidgeRuntimeHandoffAttemptV2,
    analytical_overlay: AnalyticalShadowOverlay,
    mesa: MesaGeometryV2,
    flat_scenario: ScenarioSpec,
    mesa_direct_scenario: ScenarioSpec,
    mesa_waypoint_scenario: ScenarioSpec,
    direct_controller: ControllerSpec,
    waypoint_controller: ControllerSpec,
}

/// Execute the three development lanes twice, sealing only semantic equality.
/// Wall, CPU, and controller update timing are retained as raw observations
/// but excluded from repeat comparison and the final identity.
pub fn run_conservative_ballistic_handoff_controller_development(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
) -> Result<ConservativeBallisticHandoffControllerDevelopmentRunV1> {
    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create handoff controller development output directory {}",
            output_dir.display()
        )
    })?;

    let manifest = load_experimental_ridge_case_manifest_v1();
    let prepared = prepare(&manifest)?;
    let mut first = execute(&prepared)?;
    let mut repeat = execute(&prepared)?;
    if semantic_material(&first)? != semantic_material(&repeat)? {
        bail!("handoff controller development semantic repeat diverged");
    }
    first.deterministic_repeat = true;
    repeat.deterministic_repeat = true;
    first.finalize_identity()?;
    repeat.finalize_identity()?;
    if first.identity != repeat.identity {
        bail!("handoff controller development semantic repeat identity diverged");
    }
    first.validate_against_source()?;

    write_artifacts(&output_dir, &first)?;
    let summary_path = output_dir.join("summary.json");
    let reloaded =
        load_conservative_ballistic_handoff_controller_development_artifact_v1(&summary_path)?;
    if semantic_material(&reloaded)? != semantic_material(&first)? {
        bail!("handoff controller development summary changed after reload");
    }

    let (report_path, update_site) = resolve_report_path(repo_root, &output_dir)?;
    let preview_path = report_path
        .parent()
        .expect("report path has a parent")
        .join("preview.svg");
    let report_data = serde_json::to_value(&reloaded)?;
    pd_report::conservative_ballistic_handoff_controller_development::write_conservative_ballistic_handoff_controller_development_report(
        &report_path,
        &preview_path,
        &report_data,
    )?;
    if update_site {
        ReportSite::new(repo_root).update_indexes_for_file(&report_path)?;
    }

    Ok(ConservativeBallisticHandoffControllerDevelopmentRunV1 {
        artifact: reloaded,
        paths: ConservativeBallisticHandoffControllerDevelopmentPathsV1 {
            output_dir,
            summary_path,
            report_path,
            preview_path,
        },
    })
}

/// Reload and validate an artifact without replaying the simulator.  Planner
/// evidence is recomputed from the embedded raw source input; each stored
/// lane is then checked against the exact prepared scenario/controller wiring.
pub fn load_conservative_ballistic_handoff_controller_development_artifact_v1(
    path: &Path,
) -> Result<ConservativeBallisticHandoffControllerDevelopmentArtifactV1> {
    let artifact: ConservativeBallisticHandoffControllerDevelopmentArtifactV1 =
        serde_json::from_slice(&fs::read(path).with_context(|| {
            format!(
                "failed to read handoff controller development summary {}",
                path.display()
            )
        })?)
        .with_context(|| {
            format!(
                "failed to parse handoff controller development summary {}",
                path.display()
            )
        })?;
    artifact.validate_against_source()?;
    Ok(artifact)
}

fn prepare(manifest: &ExperimentalRidgeCaseManifestV1) -> Result<PreparedF4> {
    manifest
        .validate()
        .map_err(|error| anyhow!("F4 source manifest is invalid: {error}"))?;
    let mut cases = manifest
        .cases
        .iter()
        .filter(|case| case.probe.id == F4_CASE_ID);
    let input = cases
        .next()
        .cloned()
        .ok_or_else(|| anyhow!("F4 source manifest does not contain {F4_CASE_ID}"))?;
    if cases.next().is_some() {
        bail!("F4 source manifest contains duplicate {F4_CASE_ID}");
    }
    if input.identity != F4_INPUT_IDENTITY {
        bail!("F4 input identity changed: {}", input.identity);
    }

    let candidate_projection = evaluate_experimental_ridge_case_projection_v1(&input)
        .map_err(|error| anyhow!("F4 candidate projection failed: {error}"))?;
    validate_experimental_ridge_case_projection_v1(&candidate_projection)
        .map_err(|error| anyhow!("F4 candidate projection did not validate: {error}"))?;
    if candidate_projection.identity != F4_CANDIDATE_IDENTITY {
        bail!(
            "F4 candidate projection identity changed: {}",
            candidate_projection.identity
        );
    }
    let runtime_v2 = project_experimental_ridge_case_runtime_v2(&candidate_projection)
        .map_err(|error| anyhow!("F4 runtime V2 projection failed: {error}"))?;
    validate_experimental_ridge_case_runtime_v2(&candidate_projection, &runtime_v2)
        .map_err(|error| anyhow!("F4 runtime V2 projection did not validate: {error}"))?;
    if runtime_v2.identity != F4_RUNTIME_IDENTITY {
        bail!("F4 runtime V2 identity changed: {}", runtime_v2.identity);
    }

    let nominal = match &candidate_projection.flat_control {
        ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => candidate,
        outcome => bail!("F4 flat control must be Direct, got {outcome:?}"),
    };
    let waypoint_candidate = match &candidate_projection.derived_mesa {
        ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. } => candidate,
        outcome => bail!("F4 derived mesa must be OneWaypoint, got {outcome:?}"),
    };
    let direct_route = match &runtime_v2.flat_control {
        ExperimentalRidgeCaseRuntimeOutcomeV2::Direct {
            candidate_identity,
            route,
            structural_validation,
            ..
        } if candidate_identity == &nominal.identity
            && structural_validation.transfer_route_valid =>
        {
            route.clone()
        }
        outcome => {
            bail!("F4 flat runtime must be a structurally valid Direct route, got {outcome:?}")
        }
    };
    let (
        waypoint_route,
        selected_attempt_index,
        selected_attempt,
        runtime_candidate_identity,
        runtime_authority,
        runtime_kinematics,
        runtime_assessment,
    ) = match &runtime_v2.derived_mesa {
        ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
            candidate_identity,
            route,
            handoff_selection,
            authority,
            handoff_kinematics,
            handoff_assessment,
            structural_validation,
            ..
        } if structural_validation.transfer_route_valid => {
            let selected = handoff_selection
                .selected_attempt()
                .cloned()
                .ok_or_else(|| anyhow!("F4 runtime selected attempt index is invalid"))?;
            (
                route.clone(),
                handoff_selection.selected_attempt_index,
                selected,
                candidate_identity.clone(),
                authority.clone(),
                *handoff_kinematics,
                handoff_assessment.clone(),
            )
        }
        outcome => bail!(
            "F4 derived runtime must be a structurally valid OneWaypoint route, got {outcome:?}"
        ),
    };
    if runtime_candidate_identity != waypoint_candidate.identity
        || selected_attempt_index != 1
        || selected_attempt.selection_kind
            != ExperimentalRidgeRuntimeHandoffSelectionKindV2::IntermediateBridgeExit
        || selected_attempt.route != waypoint_route
        || selected_attempt.authority != runtime_authority
        || selected_attempt.handoff_kinematics != runtime_kinematics
        || selected_attempt.handoff_assessment != runtime_assessment
        || !selected_attempt.handoff_assessment.contract_pass
        || selected_attempt.handoff_assessment.violations != Vec::<String>::new()
        || selected_attempt.selected_state.position_m
            != Vec2::new(2715.400408105346, 1931.4569053495914)
        || waypoint_route.waypoints.len() != 1
        || waypoint_route.waypoints[0].position_m != selected_attempt.selected_state.position_m
    {
        bail!("F4 selected bridge-exit attempt or route binding changed");
    }
    let selection = match &runtime_v2.derived_mesa {
        ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
            handoff_selection, ..
        } => handoff_selection,
        _ => unreachable!("derived runtime shape was checked above"),
    };
    let Some(primary) = selection.attempts.first() else {
        bail!("F4 primary handoff attempt is absent");
    };
    if selection.attempts.len() != 2
        || primary.selection_kind != ExperimentalRidgeRuntimeHandoffSelectionKindV2::PrimaryCrossing
        || primary.handoff_assessment.violations != vec!["heading".to_owned()]
    {
        bail!("F4 bounded handoff attempt sequence changed");
    }

    let flat_scenario = build_scenario(
        &input.probe,
        &candidate_projection.policy,
        &candidate_projection.vehicle,
        flat_terrain(&input.probe),
        direct_route.clone(),
        "flat-direct",
        "Flat twin direct control for the generic 068 handoff development check",
    )?;
    let mesa_direct_scenario = build_scenario(
        &input.probe,
        &candidate_projection.policy,
        &candidate_projection.vehicle,
        mesa_terrain(&candidate_projection.mesa),
        direct_route,
        "mesa-direct",
        "Derived mesa direct control for the generic 068 handoff development check",
    )?;
    let mesa_waypoint_scenario = build_scenario(
        &input.probe,
        &candidate_projection.policy,
        &candidate_projection.vehicle,
        mesa_terrain(&candidate_projection.mesa),
        waypoint_route,
        "mesa-waypoint",
        "Derived mesa bridge-exit waypoint control for the generic 068 handoff development check; analytical bridge is not replayed",
    )?;
    let direct_controller = built_in_controller_spec("transfer_pdg")
        .ok_or_else(|| anyhow!("F4 direct controller transfer_pdg is unavailable"))?;
    let waypoint_controller = built_in_controller_spec("transfer_waypoint_pdg")
        .ok_or_else(|| anyhow!("F4 waypoint controller transfer_waypoint_pdg is unavailable"))?;
    Ok(PreparedF4 {
        input_manifest_identity: manifest.identity.clone(),
        input,
        candidate_projection: candidate_projection.clone(),
        runtime_v2,
        selected_attempt_index,
        selected_attempt,
        analytical_overlay: build_analytical_overlay(nominal, waypoint_candidate),
        mesa: candidate_projection.mesa.clone(),
        flat_scenario,
        mesa_direct_scenario,
        mesa_waypoint_scenario,
        direct_controller,
        waypoint_controller,
    })
}

fn execute(
    prepared: &PreparedF4,
) -> Result<ConservativeBallisticHandoffControllerDevelopmentArtifactV1> {
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
    let gate = derive_gate(&lanes)?;
    let mut artifact = ConservativeBallisticHandoffControllerDevelopmentArtifactV1 {
        schema_id: CONSERVATIVE_BALLISTIC_HANDOFF_CONTROLLER_DEVELOPMENT_SCHEMA_ID_V1.to_owned(),
        schema_version: CONSERVATIVE_BALLISTIC_HANDOFF_CONTROLLER_DEVELOPMENT_SCHEMA_VERSION_V1,
        setup_id: CONSERVATIVE_BALLISTIC_HANDOFF_CONTROLLER_DEVELOPMENT_SETUP_ID_V1.to_owned(),
        scope: development_scope(),
        input_manifest_identity: prepared.input_manifest_identity.clone(),
        case_id: prepared.input.probe.id.clone(),
        input_identity: prepared.input.identity.clone(),
        input: prepared.input.clone(),
        candidate_projection: prepared.candidate_projection.clone(),
        runtime_v2: prepared.runtime_v2.clone(),
        selected_attempt_index: prepared.selected_attempt_index,
        selected_attempt_identity: prepared.selected_attempt.identity.clone(),
        analytical_overlay: prepared.analytical_overlay.clone(),
        lanes,
        gate,
        deterministic_repeat: false,
        identity: String::new(),
    };
    artifact.finalize_identity()?;
    Ok(artifact)
}

impl ConservativeBallisticHandoffControllerDevelopmentArtifactV1 {
    fn finalize_identity(&mut self) -> Result<()> {
        self.identity = canonical_digest(&semantic_material(self)?).map_err(anyhow::Error::msg)?;
        Ok(())
    }

    pub fn validate_against_source(&self) -> Result<()> {
        if self.schema_id != CONSERVATIVE_BALLISTIC_HANDOFF_CONTROLLER_DEVELOPMENT_SCHEMA_ID_V1
            || self.schema_version
                != CONSERVATIVE_BALLISTIC_HANDOFF_CONTROLLER_DEVELOPMENT_SCHEMA_VERSION_V1
            || self.setup_id != CONSERVATIVE_BALLISTIC_HANDOFF_CONTROLLER_DEVELOPMENT_SETUP_ID_V1
            || self.scope != development_scope()
            || !self.deterministic_repeat
        {
            bail!("F4 artifact schema, scope, or repeat binding is invalid");
        }
        let manifest = load_experimental_ridge_case_manifest_v1();
        let prepared = prepare(&manifest)?;
        if self.input_manifest_identity != prepared.input_manifest_identity
            || self.case_id != F4_CASE_ID
            || self.input_identity != prepared.input.identity
            || self.input != prepared.input
            || self.candidate_projection != prepared.candidate_projection
            || self.runtime_v2 != prepared.runtime_v2
            || self.selected_attempt_index != prepared.selected_attempt_index
            || self.selected_attempt_identity != prepared.selected_attempt.identity
            || self.analytical_overlay != prepared.analytical_overlay
        {
            bail!("F4 artifact planner provenance or selected attempt does not recompute");
        }
        validate_lane_wiring(&self.lanes, &prepared)?;
        if self.gate != derive_gate(&self.lanes)? {
            bail!("F4 artifact gate evidence does not derive from lanes");
        }
        let expected_identity =
            canonical_digest(&semantic_material(self)?).map_err(anyhow::Error::msg)?;
        if self.identity.is_empty() || self.identity != expected_identity {
            bail!(
                "F4 artifact semantic identity does not bind contents: persisted={} expected={expected_identity}",
                self.identity
            );
        }
        Ok(())
    }
}

fn validate_lane_wiring(lanes: &[ShadowLaneSummary], prepared: &PreparedF4) -> Result<()> {
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
    if lanes.len() != expected.len() {
        bail!("F4 artifact must retain exactly three lanes");
    }
    for (lane, (id, terrain_kind, scenario, controller)) in lanes.iter().zip(expected) {
        if lane.id != id
            || lane.terrain_kind != terrain_kind
            || lane.scenario_id != scenario.id
            || lane.controller_id != controller.id()
            || lane.scenario.as_ref() != Some(scenario)
            || lane.controller.as_ref() != Some(controller)
            || lane.preflight.route != scenario.mission.transfer_route
            || lane.run.is_none()
        {
            bail!("F4 lane {id} does not bind the prepared scenario/controller/run wiring");
        }
    }
    let direct_route = prepared
        .flat_scenario
        .mission
        .transfer_route
        .as_ref()
        .expect("prepared direct route");
    let waypoint_route = prepared
        .mesa_waypoint_scenario
        .mission
        .transfer_route
        .as_ref()
        .expect("prepared waypoint route");
    if lanes[1].preflight.route.as_ref() != Some(direct_route)
        || lanes[2].preflight.route.as_ref() != Some(waypoint_route)
        || waypoint_route.waypoints.len() != 1
        || waypoint_route.waypoints[0].position_m
            != prepared.selected_attempt.selected_state.position_m
    {
        bail!("F4 direct or selected waypoint route wiring changed");
    }
    Ok(())
}

fn derive_gate(
    lanes: &[ShadowLaneSummary],
) -> Result<ConservativeBallisticHandoffControllerGateEvidenceV1> {
    if lanes.len() != 3
        || lanes[0].id != "flat-direct"
        || lanes[1].id != "mesa-direct"
        || lanes[2].id != "mesa-waypoint"
    {
        bail!("F4 gate requires ordered flat-direct, mesa-direct, mesa-waypoint lanes");
    }
    let launches = lanes.iter().map(launch_evidence).collect::<Vec<_>>();
    let all_lane_launch_valid = if launches.iter().all(|check| check.passes) {
        ConservativeBallisticHandoffControllerGateCheckV1 {
            passes: true,
            reason: "all lanes began takeoff at controller step 0 with upright first target and survived source launch".to_owned(),
        }
    } else {
        ConservativeBallisticHandoffControllerGateCheckV1 {
            passes: false,
            reason: launches
                .iter()
                .filter(|check| !check.passes)
                .map(|check| check.reason.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        }
    };
    let flat = &lanes[0];
    let flat_target_landing = check(
        flat.end_reason == Some(EndReason::TouchdownOnTarget),
        format!("flat-direct ended as {:?}", flat.end_reason),
        "flat-direct landed on target".to_owned(),
    );
    let mesa_direct = &lanes[1];
    let mesa_direct_ridge_span_crash = check(
        mesa_direct.end_reason == Some(EndReason::Crash)
            && mesa_direct.terrain_contact.reconstruction_valid == Some(true)
            && mesa_direct.terrain_contact.within_derived_mesa_span,
        format!(
            "mesa-direct ended as {:?}; reconstructed_contact_valid={:?}; within_derived_mesa_span={}",
            mesa_direct.end_reason,
            mesa_direct.terrain_contact.reconstruction_valid,
            mesa_direct.terrain_contact.within_derived_mesa_span
        ),
        "mesa-direct crashed at a validated reconstructed contact within the derived mesa span"
            .to_owned(),
    );
    let waypoint = &lanes[2];
    let waypoint_exactly_one_captured_contract_pass = check(
        waypoint.waypoint_contract.as_ref().is_some_and(|contract| {
            contract.marker_count == 1
                && contract.contract_pass
                && contract.resolution_reason.as_deref() == Some("contract_pass")
        }),
        waypoint.waypoint_contract.as_ref().map_or_else(
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
        waypoint.end_reason == Some(EndReason::TouchdownOnTarget),
        format!("mesa-waypoint ended as {:?}", waypoint.end_reason),
        "mesa-waypoint landed on target after the handoff".to_owned(),
    );
    let all_green = all_lane_launch_valid.passes
        && flat_target_landing.passes
        && mesa_direct_ridge_span_crash.passes
        && waypoint_exactly_one_captured_contract_pass.passes
        && waypoint_target_landing.passes;
    let overall_green = if all_green {
        ConservativeBallisticHandoffControllerGateCheckV1 {
            passes: true,
            reason: "flat baseline landed, mesa direct crashed on the ridge, and selected waypoint captured then landed".to_owned(),
        }
    } else {
        ConservativeBallisticHandoffControllerGateCheckV1 {
            passes: false,
            reason: [
                &all_lane_launch_valid,
                &flat_target_landing,
                &mesa_direct_ridge_span_crash,
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
    Ok(ConservativeBallisticHandoffControllerGateEvidenceV1 {
        launches,
        all_lane_launch_valid,
        flat_target_landing,
        mesa_direct_ridge_span_crash,
        waypoint_exactly_one_captured_contract_pass,
        waypoint_target_landing,
        overall_green,
    })
}

fn launch_evidence(
    lane: &ShadowLaneSummary,
) -> ConservativeBallisticHandoffControllerLaunchEvidenceV1 {
    let phase_takeoff = lane.preflight.first_phase.as_deref() == Some("takeoff");
    let first_update_step_zero = lane.preflight.first_controller_update_physics_step == Some(0);
    let upright_first_target = lane
        .preflight
        .first_target_attitude_rad
        .is_some_and(|attitude| attitude.abs() <= 1.0e-12);
    let survived_source_launch = lane.preflight.survived_source_launch == Some(true);
    let passes =
        phase_takeoff && first_update_step_zero && upright_first_target && survived_source_launch;
    ConservativeBallisticHandoffControllerLaunchEvidenceV1 {
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

fn check(
    passes: bool,
    failure_reason: String,
    passing_reason: String,
) -> ConservativeBallisticHandoffControllerGateCheckV1 {
    ConservativeBallisticHandoffControllerGateCheckV1 {
        passes,
        reason: if passes {
            passing_reason
        } else {
            failure_reason
        },
    }
}

fn semantic_material(
    artifact: &ConservativeBallisticHandoffControllerDevelopmentArtifactV1,
) -> Result<Value> {
    Ok(canonicalize_semantic_value(json!({
        "schema_id": artifact.schema_id,
        "schema_version": artifact.schema_version,
        "setup_id": artifact.setup_id,
        "scope": artifact.scope,
        "input_manifest_identity": artifact.input_manifest_identity,
        "case_id": artifact.case_id,
        "input_identity": artifact.input_identity,
        "input": artifact.input,
        "candidate_projection": artifact.candidate_projection,
        "runtime_v2": artifact.runtime_v2,
        "selected_attempt_index": artifact.selected_attempt_index,
        "selected_attempt_identity": artifact.selected_attempt_identity,
        "analytical_overlay": artifact.analytical_overlay,
        "lanes": artifact.lanes.iter().map(semantic_lane).collect::<Vec<_>>(),
        "gate": artifact.gate,
        "deterministic_repeat": artifact.deterministic_repeat,
    })))
}

/// JSON round trips may canonicalize negative zero differently across the
/// planner's analytical floating-point evidence.  Negative and positive zero
/// have the same semantic value here; normalize them before hashing so reload
/// identity remains tied to controller/planner behavior rather than encoding.
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
    artifact: &ConservativeBallisticHandoffControllerDevelopmentArtifactV1,
) -> Result<()> {
    let summary_path = output_dir.join("summary.json");
    fs::write(&summary_path, serde_json::to_vec_pretty(artifact)?)
        .with_context(|| format!("failed to write F4 summary {}", summary_path.display()))?;
    load_conservative_ballistic_handoff_controller_development_artifact_v1(&summary_path)?;
    for lane in &artifact.lanes {
        let scenario = lane
            .scenario
            .as_ref()
            .ok_or_else(|| anyhow!("F4 lane {} lacks scenario evidence", lane.id))?;
        let controller = lane
            .controller
            .as_ref()
            .ok_or_else(|| anyhow!("F4 lane {} lacks controller evidence", lane.id))?;
        let run = lane
            .run
            .as_ref()
            .ok_or_else(|| anyhow!("F4 lane {} lacks run evidence", lane.id))?;
        let lane_dir = output_dir.join("runs").join(&lane.id);
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
                .join(CONSERVATIVE_BALLISTIC_HANDOFF_CONTROLLER_DEVELOPMENT_SETUP_ID_V1)
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
            site.default_output_for_bundle(output_dir)
                .ok_or_else(|| anyhow!("F4 output under outputs has no report bundle path"))?,
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
        id: format!("f4-generic-068-{lane_id}"),
        name: format!("F4 generic 068 {lane_id}"),
        description: description.to_owned(),
        seed: 0,
        tags: vec![
            "research".to_owned(),
            "development".to_owned(),
            "conservative-ballistic-handoff-controller-development".to_owned(),
            lane_id.to_owned(),
        ],
        metadata: BTreeMap::from([
            ("source_case_id".to_owned(), F4_CASE_ID.to_owned()),
            (
                "development_schema".to_owned(),
                CONSERVATIVE_BALLISTIC_HANDOFF_CONTROLLER_DEVELOPMENT_SCHEMA_ID_V1.to_owned(),
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
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("pd-eval-f4-{label}-{nonce}"))
    }

    #[test]
    fn preparation_binds_exact_068_runtime_selection_and_validated_scenarios() {
        let prepared = prepare(&load_experimental_ridge_case_manifest_v1()).unwrap();
        assert_eq!(prepared.input.probe.id, F4_CASE_ID);
        assert_eq!(prepared.input.identity, F4_INPUT_IDENTITY);
        assert_eq!(
            prepared.candidate_projection.identity,
            F4_CANDIDATE_IDENTITY
        );
        assert_eq!(prepared.runtime_v2.identity, F4_RUNTIME_IDENTITY);
        assert_eq!(prepared.selected_attempt_index, 1);
        assert_eq!(
            prepared.selected_attempt.selection_kind,
            ExperimentalRidgeRuntimeHandoffSelectionKindV2::IntermediateBridgeExit
        );
        assert_eq!(prepared.flat_scenario.sim.physics_hz, 120);
        assert_eq!(prepared.flat_scenario.sim.controller_hz, 60);
        assert_eq!(prepared.flat_scenario.sim.max_time_s, 180.0);
        assert_eq!(
            prepared.flat_scenario.mission.transfer_route.as_ref(),
            prepared
                .mesa_direct_scenario
                .mission
                .transfer_route
                .as_ref()
        );
        assert_eq!(
            prepared
                .mesa_waypoint_scenario
                .mission
                .transfer_route
                .as_ref()
                .unwrap()
                .waypoints[0]
                .position_m,
            prepared.selected_attempt.selected_state.position_m
        );
        for scenario in [
            &prepared.flat_scenario,
            &prepared.mesa_direct_scenario,
            &prepared.mesa_waypoint_scenario,
        ] {
            scenario.validate().unwrap();
        }
    }

    #[test]
    fn development_artifact_reloads_and_rejects_provenance_route_lane_and_gate_tampering() {
        let prepared = prepare(&load_experimental_ridge_case_manifest_v1()).unwrap();
        let mut artifact = execute(&prepared).unwrap();
        artifact.deterministic_repeat = true;
        artifact.finalize_identity().unwrap();
        artifact.validate_against_source().unwrap();
        let root = temp_root("reload");
        fs::create_dir_all(&root).unwrap();
        let summary = root.join("summary.json");
        fs::write(&summary, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
        let raw_reloaded: ConservativeBallisticHandoffControllerDevelopmentArtifactV1 =
            serde_json::from_slice(&fs::read(&summary).unwrap()).unwrap();
        if let Some(difference) = first_value_difference(
            &semantic_material(&artifact).unwrap(),
            &semantic_material(&raw_reloaded).unwrap(),
            "$",
        ) {
            panic!("F4 semantic JSON changed on reload at {difference}");
        }
        load_conservative_ballistic_handoff_controller_development_artifact_v1(&summary).unwrap();

        let mut runtime = artifact.clone();
        runtime.runtime_v2.identity.push_str("-tampered");
        runtime.finalize_identity().unwrap();
        assert!(runtime.validate_against_source().is_err());
        let mut tampered_route = artifact.clone();
        let ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint { route, .. } =
            &mut tampered_route.runtime_v2.derived_mesa
        else {
            panic!("prepared F4 runtime must retain one waypoint");
        };
        route.waypoints[0].position_m.x += 1.0;
        tampered_route.finalize_identity().unwrap();
        assert!(tampered_route.validate_against_source().is_err());
        let mut selected = artifact.clone();
        selected.selected_attempt_identity.push_str("-tampered");
        selected.finalize_identity().unwrap();
        assert!(selected.validate_against_source().is_err());
        let mut lane = artifact.clone();
        lane.lanes[2]
            .scenario
            .as_mut()
            .unwrap()
            .id
            .push_str("-tampered");
        lane.finalize_identity().unwrap();
        assert!(lane.validate_against_source().is_err());
        let mut gate = artifact.clone();
        gate.gate.overall_green.passes = !gate.gate.overall_green.passes;
        gate.finalize_identity().unwrap();
        assert!(gate.validate_against_source().is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn semantic_comparison_excludes_timing_but_detects_controller_result_changes() {
        let prepared = prepare(&load_experimental_ridge_case_manifest_v1()).unwrap();
        let mut first = execute(&prepared).unwrap();
        let baseline = semantic_material(&first).unwrap();
        let run = first.lanes[0].run.as_mut().unwrap();
        run.performance.wall_time_us = run.performance.wall_time_us.saturating_add(1);
        run.performance.thread_cpu_time_us = Some(1);
        if let Some(update) = run.controller_updates.first_mut() {
            update.compute_time_us = Some(1);
        }
        assert_eq!(baseline, semantic_material(&first).unwrap());
        first.lanes[0].end_reason = Some(EndReason::Crash);
        assert_ne!(baseline, semantic_material(&first).unwrap());
    }

    fn first_value_difference(left: &Value, right: &Value, path: &str) -> Option<String> {
        match (left, right) {
            (Value::Array(left), Value::Array(right)) => {
                if left.len() != right.len() {
                    return Some(format!(
                        "{path}: array length {} != {}",
                        left.len(),
                        right.len()
                    ));
                }
                left.iter()
                    .zip(right)
                    .enumerate()
                    .find_map(|(index, (left, right))| {
                        first_value_difference(left, right, &format!("{path}[{index}]"))
                    })
            }
            (Value::Object(left), Value::Object(right)) => {
                if left.len() != right.len() {
                    return Some(format!(
                        "{path}: object length {} != {}",
                        left.len(),
                        right.len()
                    ));
                }
                left.iter().find_map(|(key, left)| {
                    right.get(key).map_or_else(
                        || Some(format!("{path}.{key}: missing after reload")),
                        |right| first_value_difference(left, right, &format!("{path}.{key}")),
                    )
                })
            }
            _ if left != right => Some(format!("{path}: {left:?} != {right:?}")),
            _ => None,
        }
    }
}
