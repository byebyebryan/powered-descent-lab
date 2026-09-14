//! F6 end-to-end controller evidence for the opt-in conservative-ballistic route.
//!
//! This module exercises exactly the scenario emitted by the F6 application
//! boundary. It does not reopen the F5 held-out reveal, rerun the rejected 072
//! setup, or change planner/controller behavior.

use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use pd_control::{ControllerSpec, built_in_controller_spec};
use pd_core::{EndReason, MissionOutcome, PhysicalOutcome, ScenarioSpec, Vec2};
use pd_plan::conservative_ballistic_bridge::MesaGeometryV2;
use pd_report::site::ReportSite;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    ConservativeBallisticIntegrationDecisionV1, ConservativeBallisticIntegrationDispositionV1,
    ConservativeBallisticScenarioApplicationStatusV1, ConservativeBallisticScenarioApplicationV1,
    apply_conservative_ballistic_integration_v1, build_conservative_ballistic_f6_scenario_v1,
    canonical_digest,
    controller_shadow::{ShadowLaneSummary, run_lane, semantic_lane},
    load_conservative_ballistic_f6_input_v1, resolve_conservative_ballistic_integration_v1,
};

pub const CONSERVATIVE_BALLISTIC_F6_CONTROLLER_SCHEMA_ID_V1: &str =
    "conservative_ballistic_f6_controller_integration_v1";
pub const CONSERVATIVE_BALLISTIC_F6_CONTROLLER_SCHEMA_VERSION_V1: u32 = 1;
pub const CONSERVATIVE_BALLISTIC_F6_CONTROLLER_SETUP_ID_V1: &str =
    "conservative-ballistic-ridge-f6-integration-v1";
pub const CONSERVATIVE_BALLISTIC_F6_CONTROLLER_RESULT_SCHEMA_ID_V1: &str =
    "conservative_ballistic_f6_controller_integration_result_v1";
pub const CONSERVATIVE_BALLISTIC_F6_CONTROLLER_RESULT_FIXTURE_V1: &str =
    "fixtures/manifests/conservative_ballistic_ridge_f6_integration_result_v1.json";

const F6_CONTROLLER_ID: &str = "transfer_waypoint_pdg";
const F6_LANE_ID: &str = "mesa-waypoint";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticF6ControllerScopeV1 {
    pub controller_run: bool,
    pub simulation_run: bool,
    pub physical_execution_run: bool,
    pub lane_count: usize,
    pub non_claims: Vec<String>,
}

fn controller_scope() -> ConservativeBallisticF6ControllerScopeV1 {
    ConservativeBallisticF6ControllerScopeV1 {
        controller_run: true,
        simulation_run: true,
        physical_execution_run: false,
        lane_count: 1,
        non_claims: vec![
            "F6 runs only the retained 056 derived-mesa mission through the opt-in route application boundary".to_owned(),
            "F6 does not establish support for arbitrary heightfields, additional waypoints, or the rejected 072 setup".to_owned(),
            "controller evidence does not tune the analytical policy, route adapter, simulator, or controller".to_owned(),
            "simulator/controller evidence is not physical-execution evidence".to_owned(),
        ],
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservativeBallisticF6ControllerStatusV1 {
    IntegrationPass,
    ControllerMismatch,
    InvalidSetup,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticF6GateCheckV1 {
    pub passes: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticF6LaunchEvidenceV1 {
    pub phase_takeoff: bool,
    pub first_update_step_zero: bool,
    pub upright_first_target: bool,
    pub survived_source_launch: bool,
    pub passes: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticF6GateEvidenceV1 {
    pub decision_supported_one_waypoint: ConservativeBallisticF6GateCheckV1,
    pub route_application_injected: ConservativeBallisticF6GateCheckV1,
    pub exact_route_join: ConservativeBallisticF6GateCheckV1,
    pub launch: ConservativeBallisticF6LaunchEvidenceV1,
    pub exactly_one_contract_pass_capture: ConservativeBallisticF6GateCheckV1,
    pub target_landing: ConservativeBallisticF6GateCheckV1,
    pub integration_pass: ConservativeBallisticF6GateCheckV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticF6ControllerArtifactV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub setup_id: String,
    pub scope: ConservativeBallisticF6ControllerScopeV1,
    pub fixture_identity: String,
    pub decision: ConservativeBallisticIntegrationDecisionV1,
    pub application: ConservativeBallisticScenarioApplicationV1,
    pub selected_route_identity: String,
    pub selected_waypoint_position_m: Vec2,
    pub lane: ShadowLaneSummary,
    pub gate: ConservativeBallisticF6GateEvidenceV1,
    pub status: ConservativeBallisticF6ControllerStatusV1,
    pub deterministic_repeat: bool,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticF6CompactLaneV1 {
    pub lane_id: String,
    pub semantic_identity: String,
    pub controller_id: String,
    pub scenario_id: String,
    pub end_reason: Option<EndReason>,
    pub physical_outcome: Option<PhysicalOutcome>,
    pub mission_outcome: Option<MissionOutcome>,
    pub waypoint_contract_pass: Option<bool>,
    pub waypoint_contract_marker_count: Option<usize>,
    pub waypoint_contract_resolution_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservativeBallisticF6ControllerResultV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub setup_id: String,
    pub scope: ConservativeBallisticF6ControllerScopeV1,
    pub artifact_identity: String,
    pub fixture_identity: String,
    pub decision_identity: String,
    pub application_identity: String,
    pub selected_route_identity: String,
    pub selected_waypoint_position_m: Vec2,
    pub lane: ConservativeBallisticF6CompactLaneV1,
    pub gate: ConservativeBallisticF6GateEvidenceV1,
    pub status: ConservativeBallisticF6ControllerStatusV1,
    pub deterministic_repeat: bool,
    pub identity: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConservativeBallisticF6ControllerPathsV1 {
    pub output_dir: PathBuf,
    pub summary_path: PathBuf,
    pub result_path: PathBuf,
    pub report_path: PathBuf,
    pub preview_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ConservativeBallisticF6ControllerRunV1 {
    pub artifact: ConservativeBallisticF6ControllerArtifactV1,
    pub result: ConservativeBallisticF6ControllerResultV1,
    pub paths: ConservativeBallisticF6ControllerPathsV1,
}

#[derive(Clone)]
struct PreparedF6 {
    fixture_identity: String,
    decision: ConservativeBallisticIntegrationDecisionV1,
    application: ConservativeBallisticScenarioApplicationV1,
    injected_scenario: ScenarioSpec,
    selected_route_identity: String,
    selected_waypoint_position_m: Vec2,
    mesa: MesaGeometryV2,
    controller: ControllerSpec,
}

/// Run the injected F6 scenario twice, compare semantic evidence, and only
/// then publish the detailed and compact artifacts.
pub fn run_conservative_ballistic_f6_controller_integration_v1(
    repo_root: &Path,
    requested_output_dir: Option<&Path>,
    requested_result_path: Option<&Path>,
) -> Result<ConservativeBallisticF6ControllerRunV1> {
    let prepared = prepare().context(
        "F6 integration setup is invalid before controller execution; no evidence was written",
    )?;
    let mut first = execute(&prepared)
        .context("F6 controller execution failed; no integration evidence was written")?;
    let mut repeat = execute(&prepared)
        .context("F6 controller repeat failed; no integration evidence was written")?;
    if semantic_material(&first) != semantic_material(&repeat) {
        bail!("F6 controller semantic repeat diverged; no integration evidence was written");
    }
    first.deterministic_repeat = true;
    repeat.deterministic_repeat = true;
    first.finalize_identity()?;
    repeat.finalize_identity()?;
    if first.identity != repeat.identity {
        bail!(
            "F6 controller semantic repeat identity diverged; no integration evidence was written"
        );
    }
    first.validate_against_source()?;
    let result = ConservativeBallisticF6ControllerResultV1::from_artifact(&first)?;
    result.validate_against_artifact(&first)?;

    let output_dir = resolve_output_dir(repo_root, requested_output_dir);
    let summary_path = output_dir.join("summary.json");
    let result_path = resolve_result_path(repo_root, requested_result_path);
    let fresh_publication = publication_files(&output_dir, &first, &result_path, &result)?;
    validate_unique_destinations(&fresh_publication)?;
    let persisted_artifact = persisted_artifact(&summary_path, &first)?;
    let publication = publication_files(&output_dir, &persisted_artifact, &result_path, &result)?;
    preflight_publication(&publication)?;
    write_publication(&publication)?;
    assert_no_mismatched_publication(&publication)?;
    let reloaded = load_conservative_ballistic_f6_controller_artifact_v1(&summary_path)?;
    if semantic_material(&reloaded) != semantic_material(&first) {
        bail!("F6 controller summary changed after reload");
    }
    let reloaded_result = load_conservative_ballistic_f6_controller_result_v1(&result_path)?;
    reloaded_result.validate_against_artifact(&reloaded)?;
    let (report_path, update_site) = resolve_report_path(repo_root, &output_dir)?;
    let preview_path = report_path
        .parent()
        .expect("F6 controller report path has a parent")
        .join("preview.svg");
    pd_report::conservative_ballistic_f6_integration::write_conservative_ballistic_f6_integration_report(
        &report_path,
        &preview_path,
        &serde_json::to_value(&reloaded)?,
    )?;
    if update_site {
        ReportSite::new(repo_root).update_indexes_for_file(&report_path)?;
    }

    Ok(ConservativeBallisticF6ControllerRunV1 {
        artifact: reloaded,
        result: reloaded_result,
        paths: ConservativeBallisticF6ControllerPathsV1 {
            output_dir,
            summary_path,
            result_path,
            report_path,
            preview_path,
        },
    })
}

pub fn load_conservative_ballistic_f6_controller_artifact_v1(
    path: &Path,
) -> Result<ConservativeBallisticF6ControllerArtifactV1> {
    let artifact = serde_json::from_slice::<ConservativeBallisticF6ControllerArtifactV1>(
        &fs::read(path)
            .with_context(|| format!("failed to read F6 controller summary {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse F6 controller summary {}", path.display()))?;
    artifact.validate_against_source()?;
    Ok(artifact)
}

pub fn load_conservative_ballistic_f6_controller_result_v1(
    path: &Path,
) -> Result<ConservativeBallisticF6ControllerResultV1> {
    let result = serde_json::from_slice::<ConservativeBallisticF6ControllerResultV1>(
        &fs::read(path)
            .with_context(|| format!("failed to read F6 controller result {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse F6 controller result {}", path.display()))?;
    result.validate_against_source()?;
    Ok(result)
}

impl ConservativeBallisticF6ControllerArtifactV1 {
    fn finalize_identity(&mut self) -> Result<()> {
        self.identity = canonical_digest(&semantic_material(self)).map_err(anyhow::Error::msg)?;
        Ok(())
    }

    pub fn validate_against_source(&self) -> Result<()> {
        if self.schema_id != CONSERVATIVE_BALLISTIC_F6_CONTROLLER_SCHEMA_ID_V1
            || self.schema_version != CONSERVATIVE_BALLISTIC_F6_CONTROLLER_SCHEMA_VERSION_V1
            || self.setup_id != CONSERVATIVE_BALLISTIC_F6_CONTROLLER_SETUP_ID_V1
            || self.scope != controller_scope()
            || !self.deterministic_repeat
        {
            bail!("F6 controller artifact root contract is invalid");
        }
        let prepared = prepare()?;
        validate_source_join(self, &prepared)?;
        let expected_gate = derive_gate(
            &self.decision,
            &self.application,
            &self.selected_route_identity,
            &self.lane,
        );
        if self.gate != expected_gate || self.status != classify_status(&expected_gate) {
            bail!("F6 controller gate or status does not derive from evidence");
        }
        let expected_identity =
            canonical_digest(&semantic_material(self)).map_err(anyhow::Error::msg)?;
        if self.identity.is_empty() || self.identity != expected_identity {
            bail!("F6 controller artifact identity does not bind semantic contents");
        }
        Ok(())
    }
}

impl ConservativeBallisticF6ControllerResultV1 {
    fn from_artifact(artifact: &ConservativeBallisticF6ControllerArtifactV1) -> Result<Self> {
        let lane = &artifact.lane;
        let mut result = Self {
            schema_id: CONSERVATIVE_BALLISTIC_F6_CONTROLLER_RESULT_SCHEMA_ID_V1.to_owned(),
            schema_version: 1,
            setup_id: artifact.setup_id.clone(),
            scope: artifact.scope.clone(),
            artifact_identity: artifact.identity.clone(),
            fixture_identity: artifact.fixture_identity.clone(),
            decision_identity: artifact.decision.identity.clone(),
            application_identity: artifact.application.identity.clone(),
            selected_route_identity: artifact.selected_route_identity.clone(),
            selected_waypoint_position_m: artifact.selected_waypoint_position_m,
            lane: ConservativeBallisticF6CompactLaneV1 {
                lane_id: lane.id.clone(),
                semantic_identity: canonical_digest(&semantic_lane(lane))
                    .map_err(anyhow::Error::msg)?,
                controller_id: lane.controller_id.clone(),
                scenario_id: lane.scenario_id.clone(),
                end_reason: lane.end_reason.clone(),
                physical_outcome: lane.physical_outcome.clone(),
                mission_outcome: lane.mission_outcome.clone(),
                waypoint_contract_pass: lane
                    .waypoint_contract
                    .as_ref()
                    .map(|contract| contract.contract_pass),
                waypoint_contract_marker_count: lane
                    .waypoint_contract
                    .as_ref()
                    .map(|contract| contract.marker_count),
                waypoint_contract_resolution_reason: lane
                    .waypoint_contract
                    .as_ref()
                    .and_then(|contract| contract.resolution_reason.clone()),
            },
            gate: artifact.gate.clone(),
            status: artifact.status,
            deterministic_repeat: artifact.deterministic_repeat,
            identity: String::new(),
        };
        result.identity = result_identity(&result)?;
        Ok(result)
    }

    pub fn validate_against_source(&self) -> Result<()> {
        if self.schema_id != CONSERVATIVE_BALLISTIC_F6_CONTROLLER_RESULT_SCHEMA_ID_V1
            || self.schema_version != 1
            || self.setup_id != CONSERVATIVE_BALLISTIC_F6_CONTROLLER_SETUP_ID_V1
            || self.scope != controller_scope()
            || self.artifact_identity.is_empty()
            || !self.deterministic_repeat
        {
            bail!("F6 controller result root contract is invalid");
        }
        let prepared = prepare()?;
        if self.fixture_identity != prepared.fixture_identity
            || self.decision_identity != prepared.decision.identity
            || self.application_identity != prepared.application.identity
            || self.selected_route_identity != prepared.selected_route_identity
            || self.selected_waypoint_position_m != prepared.selected_waypoint_position_m
            || self.lane.lane_id != F6_LANE_ID
            || self.lane.controller_id != prepared.controller.id()
            || self.lane.scenario_id != prepared.injected_scenario.id
        {
            bail!("F6 controller result does not join the current opt-in source");
        }
        if self.identity.is_empty() || self.identity != result_identity(self)? {
            bail!("F6 controller result identity does not bind contents");
        }
        let launch_passes = self.gate.launch.phase_takeoff
            && self.gate.launch.first_update_step_zero
            && self.gate.launch.upright_first_target
            && self.gate.launch.survived_source_launch;
        let capture_passes = self.lane.waypoint_contract_marker_count == Some(1)
            && self.lane.waypoint_contract_pass == Some(true)
            && self.lane.waypoint_contract_resolution_reason.as_deref() == Some("contract_pass");
        let landing_passes = self.lane.end_reason == Some(EndReason::TouchdownOnTarget)
            && self.lane.physical_outcome == Some(PhysicalOutcome::LandedOnTarget)
            && self.lane.mission_outcome == Some(MissionOutcome::Success);
        if !self.gate.decision_supported_one_waypoint.passes
            || !self.gate.route_application_injected.passes
            || !self.gate.exact_route_join.passes
            || self.gate.launch.passes != launch_passes
            || self.gate.exactly_one_contract_pass_capture.passes != capture_passes
            || self.gate.target_landing.passes != landing_passes
        {
            bail!("F6 compact gate checks do not derive from source and lane evidence");
        }
        let setup_passes = self.gate.decision_supported_one_waypoint.passes
            && self.gate.route_application_injected.passes
            && self.gate.exact_route_join.passes
            && self.gate.launch.passes;
        let integration_passes = setup_passes
            && self.gate.exactly_one_contract_pass_capture.passes
            && self.gate.target_landing.passes;
        if self.gate.integration_pass.passes != integration_passes
            || self.status
                != if !setup_passes {
                    ConservativeBallisticF6ControllerStatusV1::InvalidSetup
                } else if integration_passes {
                    ConservativeBallisticF6ControllerStatusV1::IntegrationPass
                } else {
                    ConservativeBallisticF6ControllerStatusV1::ControllerMismatch
                }
        {
            bail!("F6 compact gate or status is not internally derived");
        }
        Ok(())
    }

    fn validate_against_artifact(
        &self,
        artifact: &ConservativeBallisticF6ControllerArtifactV1,
    ) -> Result<()> {
        self.validate_against_source()?;
        if self.artifact_identity != artifact.identity {
            bail!("F6 compact result does not bind the detailed artifact identity");
        }
        if self != &Self::from_artifact(artifact)? {
            bail!("F6 compact result does not exactly match detailed evidence");
        }
        Ok(())
    }
}

fn result_identity(result: &ConservativeBallisticF6ControllerResultV1) -> Result<String> {
    let mut material = result.clone();
    material.identity.clear();
    canonical_digest(&material).map_err(anyhow::Error::msg)
}

fn prepare() -> Result<PreparedF6> {
    let fixture = load_conservative_ballistic_f6_input_v1()
        .map_err(|error| anyhow!("F6 input fixture is invalid: {error}"))?;
    let decision = resolve_conservative_ballistic_integration_v1(&fixture.input)
        .map_err(|error| anyhow!("F6 route decision failed: {error}"))?;
    if decision.provenance.disposition
        != ConservativeBallisticIntegrationDispositionV1::SupportedOneWaypoint
    {
        bail!("F6 retained 056 source is not a supported one-waypoint decision");
    }
    let original = build_conservative_ballistic_f6_scenario_v1(&decision)
        .map_err(|error| anyhow!("F6 route-free scenario failed: {error}"))?;
    let application = apply_conservative_ballistic_integration_v1(&decision, &original)
        .map_err(|error| anyhow!("F6 route application failed: {error}"))?;
    if application.status != ConservativeBallisticScenarioApplicationStatusV1::RouteInjected {
        bail!("F6 supported decision did not inject a scenario");
    }
    let injected_scenario = application
        .injected_scenario
        .clone()
        .ok_or_else(|| anyhow!("F6 route application lacks an injected scenario"))?;
    let route = decision
        .route
        .as_ref()
        .ok_or_else(|| anyhow!("F6 one-waypoint decision lacks a route"))?;
    if route.waypoints.len() != 1
        || injected_scenario.mission.transfer_route.as_ref() != Some(route)
    {
        bail!("F6 injected scenario does not carry the exact selected one-waypoint route");
    }
    let selected_route_identity = decision
        .provenance
        .route_identity
        .clone()
        .ok_or_else(|| anyhow!("F6 one-waypoint decision lacks a route identity"))?;
    if selected_route_identity != canonical_digest(route).map_err(anyhow::Error::msg)? {
        bail!("F6 selected route identity does not bind the injected route");
    }
    let controller = built_in_controller_spec(F6_CONTROLLER_ID)
        .ok_or_else(|| anyhow!("F6 controller {F6_CONTROLLER_ID} is unavailable"))?;
    Ok(PreparedF6 {
        fixture_identity: fixture.identity,
        decision: decision.clone(),
        application,
        injected_scenario,
        selected_route_identity,
        selected_waypoint_position_m: route.waypoints[0].position_m,
        mesa: decision.analytical_projection.mesa.clone(),
        controller,
    })
}

fn execute(prepared: &PreparedF6) -> Result<ConservativeBallisticF6ControllerArtifactV1> {
    let lane = run_lane(
        F6_LANE_ID,
        "mesa",
        prepared.injected_scenario.clone(),
        prepared.controller.clone(),
        &prepared.mesa,
        Some(prepared.selected_waypoint_position_m),
    )?;
    let gate = derive_gate(
        &prepared.decision,
        &prepared.application,
        &prepared.selected_route_identity,
        &lane,
    );
    let mut artifact = ConservativeBallisticF6ControllerArtifactV1 {
        schema_id: CONSERVATIVE_BALLISTIC_F6_CONTROLLER_SCHEMA_ID_V1.to_owned(),
        schema_version: CONSERVATIVE_BALLISTIC_F6_CONTROLLER_SCHEMA_VERSION_V1,
        setup_id: CONSERVATIVE_BALLISTIC_F6_CONTROLLER_SETUP_ID_V1.to_owned(),
        scope: controller_scope(),
        fixture_identity: prepared.fixture_identity.clone(),
        decision: prepared.decision.clone(),
        application: prepared.application.clone(),
        selected_route_identity: prepared.selected_route_identity.clone(),
        selected_waypoint_position_m: prepared.selected_waypoint_position_m,
        lane,
        gate,
        status: ConservativeBallisticF6ControllerStatusV1::InvalidSetup,
        deterministic_repeat: false,
        identity: String::new(),
    };
    artifact.status = classify_status(&artifact.gate);
    artifact.finalize_identity()?;
    Ok(artifact)
}

fn validate_source_join(
    artifact: &ConservativeBallisticF6ControllerArtifactV1,
    prepared: &PreparedF6,
) -> Result<()> {
    if artifact.fixture_identity != prepared.fixture_identity
        || artifact.decision != prepared.decision
        || artifact.application != prepared.application
        || artifact.selected_route_identity != prepared.selected_route_identity
        || artifact.selected_waypoint_position_m != prepared.selected_waypoint_position_m
        || artifact.lane.id != F6_LANE_ID
        || artifact.lane.terrain_kind != "mesa"
        || artifact.lane.controller_id != prepared.controller.id()
        || artifact.lane.scenario_id != prepared.injected_scenario.id
        || artifact.lane.scenario.as_ref() != Some(&prepared.injected_scenario)
        || artifact.lane.controller.as_ref() != Some(&prepared.controller)
        || artifact.lane.preflight.route != prepared.injected_scenario.mission.transfer_route
        || artifact.lane.run.is_none()
    {
        bail!("F6 controller lane does not bind the exact injected scenario and controller");
    }
    Ok(())
}

fn derive_gate(
    decision: &ConservativeBallisticIntegrationDecisionV1,
    application: &ConservativeBallisticScenarioApplicationV1,
    selected_route_identity: &str,
    lane: &ShadowLaneSummary,
) -> ConservativeBallisticF6GateEvidenceV1 {
    let decision_supported_one_waypoint = check(
        decision.provenance.disposition
            == ConservativeBallisticIntegrationDispositionV1::SupportedOneWaypoint
            && decision
                .route
                .as_ref()
                .is_some_and(|route| route.waypoints.len() == 1),
        "decision is not a supported one-waypoint route",
        "decision selected exactly one waypoint",
    );
    let route_application_injected = check(
        application.status == ConservativeBallisticScenarioApplicationStatusV1::RouteInjected
            && application.injected_scenario.is_some(),
        "application did not produce an injected controller scenario",
        "application produced the controller scenario",
    );
    let exact_route_join = check(
        decision.provenance.route_identity.as_deref() == Some(selected_route_identity)
            && decision.route == lane.preflight.route
            && application
                .injected_scenario
                .as_ref()
                .and_then(|scenario| scenario.mission.transfer_route.as_ref())
                == decision.route.as_ref(),
        "controller lane route differs from the selected application route",
        "controller lane carries the exact selected application route",
    );
    let launch = launch_evidence(lane);
    let exactly_one_contract_pass_capture = check(
        lane.waypoint_contract.as_ref().is_some_and(|contract| {
            contract.marker_count == 1
                && contract.contract_pass
                && contract.resolution_reason.as_deref() == Some("contract_pass")
        }),
        "controller did not emit exactly one contract_pass waypoint capture",
        "controller emitted exactly one contract_pass waypoint capture",
    );
    let target_landing = check(
        lane.end_reason == Some(EndReason::TouchdownOnTarget)
            && lane.physical_outcome == Some(PhysicalOutcome::LandedOnTarget)
            && lane.mission_outcome == Some(MissionOutcome::Success),
        "controller did not complete a successful target landing",
        "controller completed a successful target landing",
    );
    let passes = decision_supported_one_waypoint.passes
        && route_application_injected.passes
        && exact_route_join.passes
        && launch.passes
        && exactly_one_contract_pass_capture.passes
        && target_landing.passes;
    let integration_pass = check(
        passes,
        "one or more F6 integration gates failed",
        "selected route was injected, captured once, and landed on target",
    );
    ConservativeBallisticF6GateEvidenceV1 {
        decision_supported_one_waypoint,
        route_application_injected,
        exact_route_join,
        launch,
        exactly_one_contract_pass_capture,
        target_landing,
        integration_pass,
    }
}

fn launch_evidence(lane: &ShadowLaneSummary) -> ConservativeBallisticF6LaunchEvidenceV1 {
    let phase_takeoff = lane.preflight.first_phase.as_deref() == Some("takeoff");
    let first_update_step_zero = lane.preflight.first_controller_update_physics_step == Some(0);
    let upright_first_target = lane
        .preflight
        .first_target_attitude_rad
        .is_some_and(|attitude| attitude.abs() <= 1.0e-12);
    let survived_source_launch = lane.preflight.survived_source_launch == Some(true);
    let passes =
        phase_takeoff && first_update_step_zero && upright_first_target && survived_source_launch;
    ConservativeBallisticF6LaunchEvidenceV1 {
        phase_takeoff,
        first_update_step_zero,
        upright_first_target,
        survived_source_launch,
        passes,
        reason: if passes {
            "controller began takeoff at step 0, targeted upright, and survived source launch"
                .to_owned()
        } else {
            format!(
                "phase_takeoff={phase_takeoff}; first_update_step_zero={first_update_step_zero}; upright_first_target={upright_first_target}; survived_source_launch={survived_source_launch}"
            )
        },
    }
}

fn classify_status(
    gate: &ConservativeBallisticF6GateEvidenceV1,
) -> ConservativeBallisticF6ControllerStatusV1 {
    if !gate.decision_supported_one_waypoint.passes
        || !gate.route_application_injected.passes
        || !gate.exact_route_join.passes
        || !gate.launch.passes
    {
        ConservativeBallisticF6ControllerStatusV1::InvalidSetup
    } else if gate.integration_pass.passes {
        ConservativeBallisticF6ControllerStatusV1::IntegrationPass
    } else {
        ConservativeBallisticF6ControllerStatusV1::ControllerMismatch
    }
}

fn check(
    passes: bool,
    failure_reason: &str,
    passing_reason: &str,
) -> ConservativeBallisticF6GateCheckV1 {
    ConservativeBallisticF6GateCheckV1 {
        passes,
        reason: if passes {
            passing_reason.to_owned()
        } else {
            failure_reason.to_owned()
        },
    }
}

fn semantic_material(artifact: &ConservativeBallisticF6ControllerArtifactV1) -> Value {
    json!({
        "schema_id": artifact.schema_id,
        "schema_version": artifact.schema_version,
        "setup_id": artifact.setup_id,
        "scope": artifact.scope,
        "fixture_identity": artifact.fixture_identity,
        "decision": artifact.decision,
        "application": artifact.application,
        "selected_route_identity": artifact.selected_route_identity,
        "selected_waypoint_position_m": artifact.selected_waypoint_position_m,
        "lane": semantic_lane(&artifact.lane),
        "gate": artifact.gate,
        "status": artifact.status,
        "deterministic_repeat": artifact.deterministic_repeat,
    })
}

fn publication_files(
    output_dir: &Path,
    artifact: &ConservativeBallisticF6ControllerArtifactV1,
    result_path: &Path,
    result: &ConservativeBallisticF6ControllerResultV1,
) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    let scenario = artifact
        .lane
        .scenario
        .as_ref()
        .ok_or_else(|| anyhow!("F6 lane lacks scenario evidence"))?;
    let controller = artifact
        .lane
        .controller
        .as_ref()
        .ok_or_else(|| anyhow!("F6 lane lacks controller evidence"))?;
    let run = artifact
        .lane
        .run
        .as_ref()
        .ok_or_else(|| anyhow!("F6 lane lacks run evidence"))?;
    let lane_dir = output_dir.join("runs").join(F6_LANE_ID);
    Ok(vec![
        (
            output_dir.join("summary.json"),
            serde_json::to_vec_pretty(artifact)?,
        ),
        (
            lane_dir.join("scenario.json"),
            serde_json::to_vec_pretty(scenario)?,
        ),
        (
            lane_dir.join("controller.json"),
            serde_json::to_vec_pretty(controller)?,
        ),
        (lane_dir.join("run.json"), serde_json::to_vec_pretty(run)?),
        (
            result_path.to_path_buf(),
            serde_json::to_vec_pretty(result)?,
        ),
    ])
}

fn preflight_publication(files: &[(PathBuf, Vec<u8>)]) -> Result<()> {
    validate_unique_destinations(files)?;
    for (path, expected) in files {
        if path.exists() {
            let existing = fs::read(path).with_context(|| {
                format!("failed to read existing F6 evidence {}", path.display())
            })?;
            if existing != *expected {
                bail!(
                    "existing F6 evidence {} does not exactly match retained semantic evidence; nothing was written",
                    path.display()
                );
            }
        }
    }
    Ok(())
}

fn validate_unique_destinations(files: &[(PathBuf, Vec<u8>)]) -> Result<()> {
    let mut destinations = BTreeSet::new();
    for (path, _) in files {
        if !destinations.insert(normalize_destination(path)) {
            bail!(
                "F6 evidence destination {} aliases another output; nothing was written",
                path.display()
            );
        }
    }
    Ok(())
}

fn persisted_artifact(
    summary_path: &Path,
    fresh: &ConservativeBallisticF6ControllerArtifactV1,
) -> Result<ConservativeBallisticF6ControllerArtifactV1> {
    if !summary_path.exists() {
        return Ok(fresh.clone());
    }
    let existing = load_conservative_ballistic_f6_controller_artifact_v1(summary_path)?;
    if semantic_material(&existing) != semantic_material(fresh) {
        bail!(
            "existing F6 summary {} does not semantically match fresh evidence; nothing was written",
            summary_path.display()
        );
    }
    Ok(existing)
}

fn normalize_destination(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn write_publication(files: &[(PathBuf, Vec<u8>)]) -> Result<()> {
    for (path, bytes) in files {
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create F6 evidence directory {}",
                    parent.display()
                )
            })?;
        }
        fs::write(path, bytes)
            .with_context(|| format!("failed to write F6 evidence {}", path.display()))?;
    }
    Ok(())
}

fn assert_no_mismatched_publication(files: &[(PathBuf, Vec<u8>)]) -> Result<()> {
    for (path, expected) in files {
        if path.exists() && fs::read(path)? != *expected {
            bail!("F6 evidence {} differs after publication", path.display());
        }
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
                .join(CONSERVATIVE_BALLISTIC_F6_CONTROLLER_SETUP_ID_V1)
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
        .unwrap_or_else(|| repo_root.join(CONSERVATIVE_BALLISTIC_F6_CONTROLLER_RESULT_FIXTURE_V1))
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
                anyhow!("F6 controller output under outputs has no report bundle path")
            })?,
            true,
        ));
    }
    Ok((output_dir.join("report/index.html"), false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::OnceLock,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn executed_artifact() -> ConservativeBallisticF6ControllerArtifactV1 {
        static ARTIFACT: OnceLock<ConservativeBallisticF6ControllerArtifactV1> = OnceLock::new();
        ARTIFACT
            .get_or_init(|| {
                let prepared = prepare().unwrap();
                let mut artifact = execute(&prepared).unwrap();
                artifact.deterministic_repeat = true;
                artifact.finalize_identity().unwrap();
                artifact
            })
            .clone()
    }

    #[test]
    fn f6_controller_preparation_binds_the_exact_injected_route() {
        let prepared = prepare().unwrap();
        assert_eq!(
            prepared.decision.provenance.disposition,
            ConservativeBallisticIntegrationDispositionV1::SupportedOneWaypoint
        );
        assert_eq!(
            prepared.application.status,
            ConservativeBallisticScenarioApplicationStatusV1::RouteInjected
        );
        assert_eq!(
            prepared.injected_scenario.mission.transfer_route,
            prepared.decision.route
        );
        assert_eq!(
            prepared.selected_route_identity,
            prepared.decision.provenance.route_identity.unwrap()
        );
    }

    #[test]
    fn f6_status_distinguishes_setup_from_controller_mismatch() {
        let passing = ConservativeBallisticF6GateCheckV1 {
            passes: true,
            reason: "pass".to_owned(),
        };
        let mut gate = ConservativeBallisticF6GateEvidenceV1 {
            decision_supported_one_waypoint: passing.clone(),
            route_application_injected: passing.clone(),
            exact_route_join: passing.clone(),
            launch: ConservativeBallisticF6LaunchEvidenceV1 {
                phase_takeoff: true,
                first_update_step_zero: true,
                upright_first_target: true,
                survived_source_launch: true,
                passes: true,
                reason: "pass".to_owned(),
            },
            exactly_one_contract_pass_capture: passing.clone(),
            target_landing: passing.clone(),
            integration_pass: passing,
        };
        assert_eq!(
            classify_status(&gate),
            ConservativeBallisticF6ControllerStatusV1::IntegrationPass
        );
        gate.target_landing.passes = false;
        gate.integration_pass.passes = false;
        assert_eq!(
            classify_status(&gate),
            ConservativeBallisticF6ControllerStatusV1::ControllerMismatch
        );
        gate.launch.passes = false;
        assert_eq!(
            classify_status(&gate),
            ConservativeBallisticF6ControllerStatusV1::InvalidSetup
        );
    }

    #[test]
    fn f6_injected_controller_lane_passes_and_rejects_tampering() {
        let artifact = executed_artifact();
        assert_eq!(
            artifact.status,
            ConservativeBallisticF6ControllerStatusV1::IntegrationPass
        );
        assert!(artifact.gate.integration_pass.passes);
        artifact.validate_against_source().unwrap();

        let result = ConservativeBallisticF6ControllerResultV1::from_artifact(&artifact).unwrap();
        result.validate_against_artifact(&artifact).unwrap();

        let mut tampered = result;
        tampered.gate.target_landing.passes = false;
        tampered.identity = result_identity(&tampered).unwrap();
        assert!(tampered.validate_against_source().is_err());

        let mut tampered = artifact;
        tampered.selected_route_identity = "0000000000000000".to_owned();
        tampered.finalize_identity().unwrap();
        assert!(tampered.validate_against_source().is_err());
    }

    #[test]
    fn f6_mismatched_destination_fails_before_any_new_file_is_written() {
        let artifact = executed_artifact();
        let result = ConservativeBallisticF6ControllerResultV1::from_artifact(&artifact).unwrap();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "pd-eval-f6-publication-test-{}-{unique}",
            std::process::id()
        ));
        let output_dir = root.join("output");
        let result_path = root.join("result.json");
        fs::create_dir_all(&root).unwrap();
        fs::write(&result_path, b"mismatch").unwrap();

        let files = publication_files(&output_dir, &artifact, &result_path, &result).unwrap();
        assert!(preflight_publication(&files).is_err());
        assert!(!output_dir.exists());
        assert_eq!(fs::read(&result_path).unwrap(), b"mismatch");

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn f6_aliased_destination_fails_before_any_file_is_written() {
        let artifact = executed_artifact();
        let result = ConservativeBallisticF6ControllerResultV1::from_artifact(&artifact).unwrap();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "pd-eval-f6-alias-test-{}-{unique}",
            std::process::id()
        ));
        let output_dir = root.join("output");
        let aliased_result_path = output_dir.join("nested/../summary.json");
        let files =
            publication_files(&output_dir, &artifact, &aliased_result_path, &result).unwrap();

        assert!(preflight_publication(&files).is_err());
        assert!(!root.exists());
    }

    #[test]
    fn f6_rerun_retains_semantically_equal_detailed_artifact_verbatim() {
        let fresh = executed_artifact();
        let mut existing = fresh.clone();
        existing.lane.run.as_mut().unwrap().performance.wall_time_us += 1;
        existing.finalize_identity().unwrap();
        assert_eq!(existing.identity, fresh.identity);
        assert_ne!(existing, fresh);

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "pd-eval-f6-semantic-rerun-test-{}-{unique}",
            std::process::id()
        ));
        let summary_path = root.join("summary.json");
        fs::create_dir_all(&root).unwrap();
        fs::write(&summary_path, serde_json::to_vec_pretty(&existing).unwrap()).unwrap();

        let persisted = persisted_artifact(&summary_path, &fresh).unwrap();
        assert_eq!(persisted.identity, existing.identity);
        assert_eq!(
            persisted.lane.run.unwrap().performance.wall_time_us,
            existing.lane.run.unwrap().performance.wall_time_us
        );

        fs::remove_dir_all(&root).unwrap();
    }
}
