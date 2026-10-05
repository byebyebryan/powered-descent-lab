//! Neutral, post-run source-transition evidence.
//!
//! This module deliberately sits on the evaluator side of the planner and
//! controller boundaries.  The crossing kernel consumes only resolved
//! geometry, physics-rate samples, and an audit input.  The post-run
//! assembler adds provenance and (when available) terminal censoring metadata
//! after the kernel has finished deriving physical features.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use pd_control::{
    ControlledRunArtifacts, ControllerFrame, ControllerSpec, ControllerUpdateRecord,
    TelemetryValue, run_controller_spec,
};
use pd_core::{
    ActionLogEntry, Command, EndReason, EventRecord, NormalizedRouteGeometry, Observation,
    RoutePlan, RoutePlanningRequest, RunArtifacts, RunContext, RunManifest, SafetyProfile,
    SampleRecord, ScenarioSpec, SimulationError, TerrainDefinition, TransferWaypointSpec, Vec2,
    VehicleInitialState, VehicleSpec, validate_route,
};
use serde::{Deserialize, Serialize};

/// Version of the neutral source-transition artifact schema.
pub const SOURCE_TRANSITION_SCHEMA_VERSION: u32 = 1;
/// Version of the deterministic extractor implementation.
pub const SOURCE_TRANSITION_EXTRACTOR_VERSION: &str = "source_transition_d0a_v1";
/// Geometry version attached to path-relative measurements.
pub const SOURCE_TRANSITION_GEOMETRY_VERSION: &str = "selected_centerline_v1";
/// Version of the input-only development manifest and its declared overlay.
pub const SOURCE_TRANSITION_DEVELOPMENT_MANIFEST_VERSION: u32 = 1;
/// Identifier for the only execution overlay admitted by D0a.
pub const SOURCE_TRANSITION_PHYSICS_RATE_OVERLAY_VERSION: &str = "sample_retention_physics_rate_v1";
const INITIAL_STATE_TOLERANCE_M: f64 = 1.0e-9;
const TIME_TOLERANCE_S: f64 = 1.0e-9;
const CENTERLINE_TOLERANCE_M: f64 = 1.0e-9;

/// The two geometry-derived source-transition boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTransitionBoundary {
    ContactExit,
    TrackingEntry,
}

/// Direction of a raw adjacent-pair boundary crossing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTransitionCrossingDirection {
    Forward,
    Reverse,
}

/// Post-run status of source-transition evidence.  This is intentionally not
/// a mission or controller outcome label.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTransitionEvidenceStatus {
    #[default]
    Invalid,
    Complete,
    CensoredBeforeContactExit,
    CensoredBeforeTrackingEntry,
}

/// Input-only reference to a resolved development corpus.  It records no
/// expected outcomes, labels, summaries, or controller-derived features.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionDevelopmentManifest {
    pub schema_version: u32,
    pub manifest_id: String,
    pub source_commit: String,
    pub baseline_pack: String,
    pub baseline_pack_digest: String,
    pub baseline_expected_case_count: usize,
    pub baseline_resolved_input_digest: String,
    pub diagnostic_source_pack: String,
    pub diagnostic_source_commit: String,
    pub diagnostic_pack_digest: String,
    pub diagnostic_expected_case_count: usize,
    pub diagnostic_resolved_input_digest: String,
    pub baseline_cases: Vec<SourceTransitionDevelopmentCase>,
    pub diagnostic_cases: Vec<SourceTransitionDevelopmentCase>,
    pub overlay: SourceTransitionEvidenceOverlay,
    pub input_digest: String,
}

/// Input-only reconstruction of the archived signed route-angle matrix.  It
/// intentionally omits planner-output expectations and all recorded results;
/// the development gate resolves the route anew with the current planner.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceTransitionDiagnosticInputPack {
    pub schema_version: u32,
    pub id: String,
    pub source_commit: String,
    pub cases: Vec<SourceTransitionDiagnosticInputCase>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceTransitionDiagnosticInputCase {
    pub run_id: String,
    pub scenario: ScenarioSpec,
    pub route_plan: RoutePlan,
    pub controller: ControllerSpec,
}

pub const SOURCE_TRANSITION_GATE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTransitionGateFailure {
    pub run_id: String,
    pub reason: String,
}

/// Deterministic machine-readable result of the development-only D0a gate.
/// It may identify development cases, but carries no such identity into the
/// neutral source evidence artifact itself.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionDevelopmentGateSummary {
    pub schema_version: u32,
    pub manifest_id: String,
    pub input_digest: String,
    /// Set only by the non-authoritative single-case probe.
    pub case_filter: Option<String>,
    pub baseline_total_count: usize,
    pub diagnostic_total_count: usize,
    pub baseline_contract_passes: usize,
    pub baseline_contract_failures: Vec<SourceTransitionGateFailure>,
    pub parity_passes: usize,
    pub parity_failures: Vec<SourceTransitionGateFailure>,
    pub evidence_status_counts: BTreeMap<String, usize>,
    pub invalidations: Vec<SourceTransitionGateFailure>,
    pub deterministic_replay_passes: usize,
    pub deterministic_replay_failures: Vec<SourceTransitionGateFailure>,
    pub overall_passed: bool,
    pub failure_reasons: Vec<String>,
}

/// A compact cartesian case description.  The resolved identities are the
/// product of entry, lane, route angle, and seed values, with no result data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionDevelopmentCase {
    pub entry_id: String,
    pub lane_id: String,
    pub controller: String,
    pub vehicle_variant: String,
    pub terrain_profile: String,
    pub route_angles: Vec<String>,
    pub seeds: Vec<u64>,
}

impl SourceTransitionDevelopmentCase {
    /// Return deterministic input-only case keys in resolver order.
    pub fn resolved_case_keys(&self) -> Vec<String> {
        self.route_angles
            .iter()
            .flat_map(|angle| {
                self.seeds.iter().map(move |seed| {
                    source_transition_sanitize_token(&format!(
                        "{}__{}__{}__{}__seed_{seed:02}__{}",
                        self.entry_id,
                        self.terrain_profile,
                        self.vehicle_variant,
                        angle.replace('+', "pos").replace('-', "neg"),
                        self.lane_id,
                    ))
                })
            })
            .collect()
    }
}

fn source_transition_sanitize_token(token: &str) -> String {
    let mut output = String::with_capacity(token.len());
    let mut separator = false;
    for character in token.chars() {
        let normalized = character.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            output.push(normalized);
            separator = false;
        } else if !separator {
            output.push('_');
            separator = true;
        }
    }
    output.trim_matches('_').to_owned()
}

/// The sole D0a overlay changes sample retention cadence and nothing else.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTransitionSampleRetention {
    PhysicsRate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionEvidenceOverlay {
    pub version: String,
    pub sample_retention: SourceTransitionSampleRetention,
    pub preserves_execution_inputs: bool,
}

impl SourceTransitionDevelopmentManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SOURCE_TRANSITION_DEVELOPMENT_MANIFEST_VERSION {
            return Err(format!(
                "schema_version must equal {}",
                SOURCE_TRANSITION_DEVELOPMENT_MANIFEST_VERSION
            ));
        }
        for (name, value) in [
            ("manifest_id", self.manifest_id.as_str()),
            ("source_commit", self.source_commit.as_str()),
            ("baseline_pack", self.baseline_pack.as_str()),
            ("baseline_pack_digest", self.baseline_pack_digest.as_str()),
            (
                "baseline_resolved_input_digest",
                self.baseline_resolved_input_digest.as_str(),
            ),
            (
                "diagnostic_source_pack",
                self.diagnostic_source_pack.as_str(),
            ),
            (
                "diagnostic_source_commit",
                self.diagnostic_source_commit.as_str(),
            ),
            (
                "diagnostic_pack_digest",
                self.diagnostic_pack_digest.as_str(),
            ),
            (
                "diagnostic_resolved_input_digest",
                self.diagnostic_resolved_input_digest.as_str(),
            ),
            ("input_digest", self.input_digest.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("{name} must not be empty"));
            }
        }
        for (name, value) in [
            ("baseline_pack_digest", self.baseline_pack_digest.as_str()),
            (
                "baseline_resolved_input_digest",
                self.baseline_resolved_input_digest.as_str(),
            ),
            (
                "diagnostic_pack_digest",
                self.diagnostic_pack_digest.as_str(),
            ),
            (
                "diagnostic_resolved_input_digest",
                self.diagnostic_resolved_input_digest.as_str(),
            ),
            ("input_digest", self.input_digest.as_str()),
        ] {
            validate_source_transition_digest(name, value)?;
        }
        let mut digest_material = self.clone();
        digest_material.input_digest.clear();
        let expected_input_digest = format!(
            "fnv1a64:{}",
            source_transition_canonical_digest(&digest_material)
        );
        if self.input_digest != expected_input_digest {
            return Err(format!(
                "input_digest does not match canonical input material: expected {expected_input_digest}"
            ));
        }
        if self.overlay.version != SOURCE_TRANSITION_PHYSICS_RATE_OVERLAY_VERSION
            || self.overlay.sample_retention != SourceTransitionSampleRetention::PhysicsRate
            || !self.overlay.preserves_execution_inputs
        {
            return Err(
                "development overlay must be the input-preserving physics-rate overlay".to_owned(),
            );
        }
        validate_development_cases(&self.baseline_cases)?;
        validate_development_cases(&self.diagnostic_cases)?;
        let baseline_count = self
            .baseline_cases
            .iter()
            .map(|case| case.resolved_case_keys().len())
            .sum::<usize>();
        if baseline_count != self.baseline_expected_case_count {
            return Err(format!(
                "baseline_expected_case_count={} does not match case keys={baseline_count}",
                self.baseline_expected_case_count
            ));
        }
        let diagnostic_count = self
            .diagnostic_cases
            .iter()
            .map(|case| case.resolved_case_keys().len())
            .sum::<usize>();
        if diagnostic_count != self.diagnostic_expected_case_count {
            return Err(format!(
                "diagnostic_expected_case_count={} does not match case keys={diagnostic_count}",
                self.diagnostic_expected_case_count
            ));
        }
        let mut resolved_keys = BTreeSet::new();
        for key in self
            .baseline_cases
            .iter()
            .chain(self.diagnostic_cases.iter())
            .flat_map(SourceTransitionDevelopmentCase::resolved_case_keys)
        {
            if !resolved_keys.insert(key.clone()) {
                return Err(format!("duplicate resolved development case key: {key}"));
            }
        }
        Ok(())
    }
}

fn validate_source_transition_digest(name: &str, value: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix("fnv1a64:") else {
        return Err(format!("{name} must use fnv1a64:<16 hex> format"));
    };
    if hex.len() != 16 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{name} must use fnv1a64:<16 hex> format"));
    }
    Ok(())
}

fn validate_development_cases(cases: &[SourceTransitionDevelopmentCase]) -> Result<(), String> {
    if cases.is_empty() {
        return Err("development corpus must contain at least one case".to_owned());
    }
    for case in cases {
        for (name, value) in [
            ("entry_id", case.entry_id.as_str()),
            ("lane_id", case.lane_id.as_str()),
            ("controller", case.controller.as_str()),
            ("vehicle_variant", case.vehicle_variant.as_str()),
            ("terrain_profile", case.terrain_profile.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("case {name} must not be empty"));
            }
        }
        if case.route_angles.is_empty() || case.seeds.is_empty() {
            return Err("development case must contain route_angles and seeds".to_owned());
        }
    }
    Ok(())
}

/// Apply the declared evidence overlay without changing any resolved
/// physical, mission, planner, controller, or seed input.
pub fn with_physics_rate_evidence_overlay(scenario: &ScenarioSpec) -> ScenarioSpec {
    let mut overlaid = scenario.clone();
    overlaid.sim.sample_hz = Some(overlaid.sim.physics_hz);
    overlaid
}

/// Capture the same resolved controller run with physics-rate sample
/// retention.  This is a development/evidence harness only: it clones the
/// existing run context and changes no execution input besides `sample_hz`.
pub fn run_source_transition_physics_rate_capture(
    context: &RunContext,
    controller_spec: &ControllerSpec,
) -> Result<ControlledRunArtifacts, SimulationError> {
    let mut overlaid = context.clone();
    overlaid.sim.sample_hz = Some(overlaid.sim.physics_hz);
    run_controller_spec(&overlaid, controller_spec)
}

/// Run an ordinary configured-cadence capture and its physics-rate evidence
/// companion for a development parity check.  Neither capture is promoted by
/// this helper; callers must inspect [`SourceTransitionCadenceParity`].
pub fn run_source_transition_cadence_pair(
    context: &RunContext,
    controller_spec: &ControllerSpec,
) -> Result<(ControlledRunArtifacts, ControlledRunArtifacts), SimulationError> {
    let ordinary = run_controller_spec(context, controller_spec)?;
    let physics = run_source_transition_physics_rate_capture(context, controller_spec)?;
    Ok((ordinary, physics))
}

/// Load and validate an input-only D0a development manifest.
pub fn load_source_transition_development_manifest(
    path: &Path,
) -> Result<SourceTransitionDevelopmentManifest> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read development manifest {}", path.display()))?;
    let manifest: SourceTransitionDevelopmentManifest = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse development manifest {}", path.display()))?;
    manifest.validate().map_err(anyhow::Error::msg)?;
    Ok(manifest)
}

/// Load and validate the committed input-only reconstruction for the archived
/// diagnostic corpus.
pub fn load_source_transition_diagnostic_input_pack(
    path: &Path,
) -> Result<SourceTransitionDiagnosticInputPack> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read diagnostic input pack {}", path.display()))?;
    let pack: SourceTransitionDiagnosticInputPack = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse diagnostic input pack {}", path.display()))?;
    pack.validate().map_err(anyhow::Error::msg)?;
    Ok(pack)
}

impl SourceTransitionDiagnosticInputPack {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SOURCE_TRANSITION_DEVELOPMENT_MANIFEST_VERSION {
            return Err(format!(
                "schema_version must equal {}",
                SOURCE_TRANSITION_DEVELOPMENT_MANIFEST_VERSION
            ));
        }
        if self.id.trim().is_empty() || self.source_commit.trim().is_empty() {
            return Err("diagnostic input pack id and source_commit must not be empty".to_owned());
        }
        if self.cases.len() != 24 {
            return Err(format!(
                "diagnostic input pack must contain exactly 24 cases, found {}",
                self.cases.len()
            ));
        }
        let mut case_ids = BTreeSet::new();
        for case in &self.cases {
            if case.run_id.trim().is_empty() || !case_ids.insert(case.run_id.clone()) {
                return Err(format!(
                    "duplicate or empty diagnostic case run_id '{}'",
                    case.run_id
                ));
            }
            if case.scenario.id != case.run_id {
                return Err(format!(
                    "diagnostic case '{}' scenario.id does not match run_id",
                    case.run_id
                ));
            }
            case.scenario.validate().map_err(|message| {
                format!("diagnostic case '{}' scenario: {message}", case.run_id)
            })?;
            if case.scenario.sim.sample_hz != Some(10) {
                return Err(format!(
                    "diagnostic case '{}' must preserve ordinary sample_hz=10",
                    case.run_id
                ));
            }
            if case.scenario.mission.transfer_route.as_ref() != Some(&case.route_plan.route) {
                return Err(format!(
                    "diagnostic case '{}' scenario transfer route does not match route_plan",
                    case.run_id
                ));
            }
            for pad_id in [
                case.route_plan.route.source_pad_id.as_str(),
                case.route_plan.route.target_pad_id.as_str(),
            ] {
                if case.scenario.world.landing_pad(pad_id).is_none() {
                    return Err(format!(
                        "diagnostic case '{}' route pad '{}' is missing",
                        case.run_id, pad_id
                    ));
                }
            }
            let request = RoutePlanningRequest {
                world: case.scenario.world.clone(),
                vehicle: case.scenario.vehicle.clone(),
                initial_state: case.scenario.initial_state.clone(),
                source_pad_id: case.route_plan.route.source_pad_id.clone(),
                target_pad_id: case.route_plan.route.target_pad_id.clone(),
                policy: case.route_plan.policy.clone(),
            };
            validate_route(&request, &case.route_plan.route).map_err(|message| {
                format!(
                    "diagnostic case '{}' route validation: {message}",
                    case.run_id
                )
            })?;
            case.controller
                .instantiate()
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

/// Stable reasons for malformed or unverifiable source-transition input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceTransitionInvalidReason {
    MissingSamples,
    MissingPhysicsRate,
    WrongCadence {
        sample_hz: u32,
        physics_hz: u32,
    },
    PhysicsRateMismatch {
        scenario_physics_hz: u32,
        manifest_physics_hz: u32,
    },
    InvalidCadence {
        physics_hz: u32,
    },
    NonContiguousPhysicsSteps {
        previous: u64,
        current: u64,
    },
    OutOfOrderPhysicsSteps {
        previous: u64,
        current: u64,
    },
    DuplicatePhysicsStep {
        step: u64,
    },
    NonFinite {
        sample_index: usize,
        field: String,
    },
    SampleTimeMismatch {
        sample_index: usize,
    },
    ObservationIdentityMismatch {
        sample_index: usize,
    },
    InitialStateMismatch {
        field: String,
    },
    InvalidCommand {
        sample_index: usize,
        field: String,
    },
    MalformedActionLog {
        index: usize,
        field: String,
    },
    MalformedControllerUpdate {
        index: usize,
        field: String,
    },
    InvalidGeometry {
        message: String,
    },
    ClearanceQuery {
        sample_index: usize,
        message: String,
    },
    MissingCrossing {
        boundary: SourceTransitionBoundary,
    },
    MissingTerminalCensor {
        boundary: SourceTransitionBoundary,
    },
    TerminalStepMismatch {
        terminal_step: u64,
        last_sample_step: u64,
    },
    InvalidTerminalReason,
}

impl std::fmt::Display for SourceTransitionInvalidReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSamples => formatter.write_str("missing_samples"),
            Self::MissingPhysicsRate => formatter.write_str("missing_physics_rate"),
            Self::WrongCadence {
                sample_hz,
                physics_hz,
            } => write!(formatter, "wrong_cadence:{sample_hz}!={physics_hz}"),
            Self::PhysicsRateMismatch {
                scenario_physics_hz,
                manifest_physics_hz,
            } => write!(
                formatter,
                "physics_rate_mismatch:{scenario_physics_hz}!={manifest_physics_hz}"
            ),
            Self::InvalidCadence { physics_hz } => {
                write!(formatter, "invalid_cadence:physics_hz={physics_hz}")
            }
            Self::NonContiguousPhysicsSteps { previous, current } => {
                write!(
                    formatter,
                    "non_contiguous_physics_steps:{previous}->{current}"
                )
            }
            Self::OutOfOrderPhysicsSteps { previous, current } => {
                write!(
                    formatter,
                    "out_of_order_physics_steps:{previous}->{current}"
                )
            }
            Self::DuplicatePhysicsStep { step } => {
                write!(formatter, "duplicate_physics_step:{step}")
            }
            Self::NonFinite {
                sample_index,
                field,
            } => write!(formatter, "nonfinite:{sample_index}:{field}"),
            Self::SampleTimeMismatch { sample_index } => {
                write!(formatter, "sample_time_mismatch:{sample_index}")
            }
            Self::ObservationIdentityMismatch { sample_index } => {
                write!(formatter, "observation_identity_mismatch:{sample_index}")
            }
            Self::InitialStateMismatch { field } => {
                write!(formatter, "initial_state_mismatch:{field}")
            }
            Self::InvalidCommand {
                sample_index,
                field,
            } => write!(formatter, "invalid_command:{sample_index}:{field}"),
            Self::MalformedActionLog { index, field } => {
                write!(formatter, "malformed_action_log:{index}:{field}")
            }
            Self::MalformedControllerUpdate { index, field } => {
                write!(formatter, "malformed_controller_update:{index}:{field}")
            }
            Self::InvalidGeometry { message } => write!(formatter, "invalid_geometry:{message}"),
            Self::ClearanceQuery {
                sample_index,
                message,
            } => write!(formatter, "clearance_query:{sample_index}:{message}"),
            Self::MissingCrossing { boundary } => {
                write!(formatter, "missing_crossing:{}", boundary_code(*boundary))
            }
            Self::MissingTerminalCensor { boundary } => {
                write!(
                    formatter,
                    "missing_terminal_censor:{}",
                    boundary_code(*boundary)
                )
            }
            Self::TerminalStepMismatch {
                terminal_step,
                last_sample_step,
            } => write!(
                formatter,
                "terminal_step_mismatch:{terminal_step}!={last_sample_step}"
            ),
            Self::InvalidTerminalReason => formatter.write_str("invalid_terminal_reason"),
        }
    }
}

fn boundary_code(boundary: SourceTransitionBoundary) -> &'static str {
    match boundary {
        SourceTransitionBoundary::ContactExit => "contact_exit",
        SourceTransitionBoundary::TrackingEntry => "tracking_entry",
    }
}

/// Cadence attached to a raw sample stream.  D0a requires the two rates to be
/// explicitly present and equal; an absent `sample_hz` is not upgraded.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionCadence {
    pub sample_hz: Option<u32>,
    pub physics_hz: u32,
}

impl SourceTransitionCadence {
    pub const fn physics_rate(physics_hz: u32) -> Self {
        Self {
            sample_hz: Some(physics_hz),
            physics_hz,
        }
    }
}

/// Terminal metadata admitted by the post-run assembler.  It intentionally
/// carries only the terminal physics step and reason, never an outcome label.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionTerminal {
    pub physics_step: u64,
    pub reason: String,
}

/// Action and controller-update audit input.  The crossing kernel may retain
/// this information in audit fields but never uses it to choose crossings or
/// derive physical features.
#[derive(Clone, Copy, Debug, Default)]
pub struct SourceTransitionAuditInput<'a> {
    pub actions: &'a [ActionLogEntry],
    pub controller_updates: &'a [ControllerUpdateRecord],
}

/// Borrowed immutable input to the pure crossing/feature kernel.
#[derive(Clone, Copy, Debug)]
pub struct SourceTransitionKernelInput<'a> {
    pub terrain: &'a TerrainDefinition,
    pub source_pad_center_x_m: f64,
    /// The resolved vehicle is immutable kernel context.  The initial raw
    /// sample must agree with its dry-mass/initial-fuel anchor.
    pub vehicle: &'a VehicleSpec,
    pub initial_state: &'a VehicleInitialState,
    pub geometry: &'a NormalizedRouteGeometry,
    pub profile: &'a SafetyProfile,
    pub selected_centerline_m: &'a [Vec2],
    /// Waypoints retain their resolved world coordinates.  The kernel maps
    /// them into the normalized source-relative frame using `geometry`.
    pub waypoints: &'a [TransferWaypointSpec],
    pub samples: &'a [SampleRecord],
    pub cadence: SourceTransitionCadence,
    pub audit: SourceTransitionAuditInput<'a>,
}

/// A raw state retained at an anchor or boundary endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionRawState {
    pub sample_index: usize,
    pub physics_step: u64,
    pub sim_time_s: f64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub mass_kg: f64,
    pub fuel_kg: f64,
}

/// The authoritative initial source-pad anchor.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionInitialAnchor {
    pub state: SourceTransitionRawState,
    pub source_progress_m: f64,
    pub normalized_progress: f64,
}

/// A plane-interpolated boundary state.  This is derived diagnostic evidence;
/// raw endpoint states remain authoritative in the containing bracket.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionInterpolatedState {
    pub state: SourceTransitionRawState,
    pub fraction: f64,
    pub time_span_s: f64,
    pub progress_span_m: f64,
    pub before_sample_index: usize,
    pub after_sample_index: usize,
}

/// First directed raw bracket for one source-transition boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionBoundaryBracket {
    pub boundary: SourceTransitionBoundary,
    pub boundary_m: f64,
    pub before: SourceTransitionRawState,
    pub after: SourceTransitionRawState,
    pub before_source_progress_m: f64,
    pub after_source_progress_m: f64,
    pub bracket_width_s: f64,
    pub progress_span_m: f64,
    pub interpolation_fraction: f64,
    pub interpolated: SourceTransitionInterpolatedState,
}

/// Exact shaped-centerline segment reference for a boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionCenterlineReference {
    pub boundary: SourceTransitionBoundary,
    pub segment_index: usize,
    pub point_start_index: usize,
    pub point_end_index: usize,
    pub segment_start_normalized_m: Vec2,
    pub segment_end_normalized_m: Vec2,
    pub boundary_point_normalized_m: Vec2,
    pub segment_start_world_m: Vec2,
    pub segment_end_world_m: Vec2,
    pub boundary_point_world_m: Vec2,
    pub normalized_frame: String,
}

/// Exact first outbound-leg reference used by path-relative observations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionOutboundReference {
    pub geometry_version: String,
    pub route_leg_index: usize,
    pub waypoint_id: Option<String>,
    pub segment_index: usize,
    pub point_start_index: usize,
    pub point_end_index: usize,
    pub segment_start_normalized_m: Vec2,
    pub segment_end_normalized_m: Vec2,
    pub tracking_point_normalized_m: Vec2,
    pub outbound_end_normalized_m: Vec2,
    pub segment_start_world_m: Vec2,
    pub segment_end_world_m: Vec2,
    pub tracking_point_world_m: Vec2,
    pub outbound_end_world_m: Vec2,
    pub unit_tangent_normalized: Vec2,
    pub unit_tangent_world: Vec2,
    pub normalized_frame: String,
}

#[derive(Clone, Debug, Serialize)]
struct SourceTransitionGeometryIdentity<'a> {
    source_pad: &'a pd_core::LandingPadSpec,
    target_pad: &'a pd_core::LandingPadSpec,
    normalized_geometry: &'a NormalizedRouteGeometry,
}

/// A canonical exact point-clearance observation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionPointClearance {
    pub envelope: pd_core::CorridorEnvelope,
    pub clear: bool,
    pub minimum_clearance_m: f64,
    pub residual_m: f64,
    pub required_envelope_y_m: f64,
    pub terrain_position_m: Vec2,
    pub terrain_segment_index: usize,
}

/// Distinct observed and derived clearance values for one raw sample.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionClearance {
    pub observed_hull_clearance_m: f64,
    pub observed_touchdown_clearance_m: Option<f64>,
    pub contact_envelope: SourceTransitionPointClearance,
    pub resolved_envelope: SourceTransitionPointClearance,
    pub full_envelope: SourceTransitionPointClearance,
}

/// Source-relative path and velocity decomposition for one raw sample.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionPathMetrics {
    pub normalized_position_m: Vec2,
    pub along_track_m: f64,
    pub cross_track_m: f64,
    pub velocity_along_track_mps: f64,
    pub velocity_cross_track_mps: f64,
}

/// One retained physics-rate raw sample.  Command/controller data is kept in
/// [`SourceTransitionAudit`] rather than in this physical feature record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionSample {
    pub sample_index: usize,
    pub physics_step: u64,
    pub sim_time_s: f64,
    pub source_progress_m: f64,
    pub normalized_progress: f64,
    pub state: SourceTransitionRawState,
    pub path: SourceTransitionPathMetrics,
    pub clearance: SourceTransitionClearance,
}

/// Scalar extrema retained separately for each transition phase.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionExtremaValue {
    pub min: f64,
    pub max: f64,
}

impl SourceTransitionExtremaValue {
    fn from_values(values: impl Iterator<Item = f64>) -> Option<Self> {
        let mut values = values.peekable();
        let first = values.peek().copied()?;
        let mut minimum = first;
        let mut maximum = first;
        for value in values {
            minimum = minimum.min(value);
            maximum = maximum.max(value);
        }
        Some(Self {
            min: minimum,
            max: maximum,
        })
    }
}

/// Extrema over a source-transition phase's observed raw samples.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionExtrema {
    pub first_sample_index: usize,
    pub last_sample_index: usize,
    pub source_progress_m: SourceTransitionExtremaValue,
    pub normalized_progress: SourceTransitionExtremaValue,
    pub velocity_along_track_mps: SourceTransitionExtremaValue,
    pub velocity_cross_track_mps: SourceTransitionExtremaValue,
    pub speed_mps: SourceTransitionExtremaValue,
    pub vertical_speed_mps: SourceTransitionExtremaValue,
    pub attitude_rad: SourceTransitionExtremaValue,
    pub angular_rate_radps: SourceTransitionExtremaValue,
    pub mass_kg: SourceTransitionExtremaValue,
    pub fuel_kg: SourceTransitionExtremaValue,
    pub along_track_m: SourceTransitionExtremaValue,
    pub cross_track_m: SourceTransitionExtremaValue,
    pub observed_hull_clearance_m: SourceTransitionExtremaValue,
    pub observed_touchdown_clearance_m: SourceTransitionExtremaValue,
    pub contact_clearance_m: SourceTransitionExtremaValue,
    pub resolved_clearance_m: SourceTransitionExtremaValue,
    pub full_clearance_m: SourceTransitionExtremaValue,
}

/// A raw crossing event.  The first forward event is also retained as a
/// boundary bracket; later forward events are re-entries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionCrossing {
    pub boundary: SourceTransitionBoundary,
    pub direction: SourceTransitionCrossingDirection,
    pub before_sample_index: usize,
    pub after_sample_index: usize,
    pub before_physics_step: u64,
    pub after_physics_step: u64,
    pub before_sim_time_s: f64,
    pub after_sim_time_s: f64,
    pub before_source_progress_m: f64,
    pub after_source_progress_m: f64,
    pub boundary_m: f64,
    pub interpolation_fraction: f64,
}

/// Backtracking/re-entry event context retained after the first crossing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionBoundaryEvent {
    pub kind: SourceTransitionBoundaryEventKind,
    pub crossing: SourceTransitionCrossing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTransitionBoundaryEventKind {
    Backtracking,
    Reentry,
}

/// One sample identity in recorded chronological order.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionSampleIdentity {
    pub sample_index: usize,
    pub physics_step: u64,
    pub sim_time_s: f64,
}

/// Controller-update information retained as audit-only bracket context.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionControllerUpdateAudit {
    pub boundary: SourceTransitionBoundary,
    pub controller_update_index: u64,
    pub physics_step: u64,
    pub sim_time_s: f64,
    pub command: Command,
    pub changed_from_previous: bool,
    pub frame_digest: String,
}

/// Audit/provenance sidecar payload.  No field here is part of pure physical
/// features; controller frames are represented only by stable audit digests.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionAudit {
    pub sample_ordering: Vec<SourceTransitionSampleIdentity>,
    pub action_log_digest: String,
    pub controller_updates_digest: String,
    pub held_commands: Vec<SourceTransitionHeldCommandAudit>,
    pub held_command_changes: Vec<SourceTransitionHeldCommandChange>,
    pub controller_update_changes: Vec<SourceTransitionControllerUpdateAudit>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionHeldCommandAudit {
    pub sample_index: usize,
    pub physics_step: u64,
    pub command: Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionHeldCommandChange {
    pub sample_index: usize,
    pub physics_step: u64,
    pub command: Command,
}

/// Immutable and opaque identities supplied by the post-run caller.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionProvenance {
    pub resolved_input_digest: String,
    pub request_digest: String,
    pub policy_digest: String,
    pub route_plan_digest: String,
    pub source_target_geometry_digest: String,
    pub raw_bundle_digest: String,
    pub samples_digest: String,
    pub action_log_digest: String,
    pub artifact_identity: String,
}

/// Serialized neutral evidence artifact.  It deliberately has no scenario
/// seed, controller identity, mission outcome, route label, or prediction
/// digest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionEvidence {
    pub schema_version: u32,
    pub extractor_version: String,
    pub evidence_digest: String,
    pub physical_digest: String,
    pub status: SourceTransitionEvidenceStatus,
    pub invalid_reason: Option<SourceTransitionInvalidReason>,
    pub provenance: SourceTransitionProvenance,
    pub cadence: SourceTransitionCadence,
    pub source_transition_start_m: f64,
    pub source_transition_end_m: f64,
    pub initial_anchor: Option<SourceTransitionInitialAnchor>,
    pub contact_exit: Option<SourceTransitionBoundaryBracket>,
    pub tracking_entry: Option<SourceTransitionBoundaryBracket>,
    pub boundary_centerline_references: Vec<SourceTransitionCenterlineReference>,
    pub first_outbound_reference: Option<SourceTransitionOutboundReference>,
    pub samples: Vec<SourceTransitionSample>,
    pub pad_departure_extrema: Option<SourceTransitionExtrema>,
    pub acquisition_extrema: Option<SourceTransitionExtrema>,
    pub boundary_crossings: Vec<SourceTransitionCrossing>,
    pub backtracking_events: Vec<SourceTransitionBoundaryEvent>,
    pub reentry_events: Vec<SourceTransitionBoundaryEvent>,
    pub terminal: Option<SourceTransitionTerminal>,
    pub audit: SourceTransitionAudit,
}

/// A sidecar containing the audit fields separated from the physical artifact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionAuditSidecar {
    pub schema_version: u32,
    pub extractor_version: String,
    pub evidence_digest: String,
    pub status: SourceTransitionEvidenceStatus,
    pub provenance: SourceTransitionProvenance,
    pub cadence: SourceTransitionCadence,
    pub terminal: Option<SourceTransitionTerminal>,
    pub audit: SourceTransitionAudit,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct StableControllerUpdate<'a> {
    sim_time_s: f64,
    physics_step: u64,
    controller_update_index: u64,
    frame: &'a ControllerFrame,
}

#[derive(Clone, Debug, Serialize)]
struct SourceTransitionRawBundleDigestMaterial<'a> {
    manifest: &'a RunManifest,
    actions: &'a [ActionLogEntry],
    events: &'a [EventRecord],
    samples: &'a [SampleRecord],
    controller_updates: Vec<StableControllerUpdate<'a>>,
}

#[derive(Clone, Debug, Serialize)]
struct PhysicalDigestMaterial<'a> {
    source_transition_start_m: f64,
    source_transition_end_m: f64,
    initial_anchor: &'a SourceTransitionInitialAnchor,
    contact_exit: &'a Option<SourceTransitionBoundaryBracket>,
    tracking_entry: &'a Option<SourceTransitionBoundaryBracket>,
    boundary_centerline_references: &'a [SourceTransitionCenterlineReference],
    first_outbound_reference: &'a Option<SourceTransitionOutboundReference>,
    samples: &'a [SourceTransitionSample],
    pad_departure_extrema: &'a Option<SourceTransitionExtrema>,
    acquisition_extrema: &'a Option<SourceTransitionExtrema>,
    boundary_crossings: &'a [SourceTransitionCrossing],
    backtracking_events: &'a [SourceTransitionBoundaryEvent],
    reentry_events: &'a [SourceTransitionBoundaryEvent],
}

/// Run the pure crossing/feature kernel.  Missing boundaries are represented
/// as `None`; only malformed input is returned as an invalid reason.  Terminal
/// metadata is not accepted here.
pub fn extract_source_transition_kernel(
    input: &SourceTransitionKernelInput<'_>,
) -> Result<SourceTransitionKernelOutput, SourceTransitionInvalidReason> {
    validate_kernel_input(input)?;

    let states = input
        .samples
        .iter()
        .enumerate()
        .map(|(sample_index, sample)| raw_state(sample_index, sample))
        .collect::<Vec<_>>();
    let progress = states
        .iter()
        .enumerate()
        .map(|(sample_index, state)| {
            let progress = f64::from(input.geometry.horizontal_sign)
                * (state.position_m.x - input.source_pad_center_x_m);
            progress.is_finite().then_some(progress).ok_or(
                SourceTransitionInvalidReason::NonFinite {
                    sample_index,
                    field: "source_progress_m".to_owned(),
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let contact_boundary = find_forward_crossing(
        SourceTransitionBoundary::ContactExit,
        input.profile.source_transition_start_m,
        &states,
        &progress,
    )
    .map(|crossing| make_boundary_bracket(input, crossing));
    let tracking_boundary = find_forward_crossing(
        SourceTransitionBoundary::TrackingEntry,
        input.profile.source_transition_end_m,
        &states,
        &progress,
    )
    .map(|crossing| make_boundary_bracket(input, crossing));

    let outbound = first_outbound_reference(input)?;
    let boundary_centerline_references = [
        (
            SourceTransitionBoundary::ContactExit,
            input.profile.source_transition_start_m,
        ),
        (
            SourceTransitionBoundary::TrackingEntry,
            input.profile.source_transition_end_m,
        ),
    ]
    .into_iter()
    .map(|(boundary, coordinate)| {
        centerline_reference(
            boundary,
            coordinate,
            input.selected_centerline_m,
            input.source_pad_center_x_m,
            input.geometry.horizontal_sign,
        )
    })
    .collect::<Result<Vec<_>, _>>()?;

    // A complete prefix ends at the first tracking-entry endpoint.  Until
    // tracking entry is observed, retain the entire sound terminal prefix;
    // contact exit is only an acquisition boundary and must not truncate a
    // right-censored run.
    let retained_end = tracking_boundary
        .as_ref()
        .map(|bracket| bracket.after.sample_index)
        .unwrap_or_else(|| states.len().saturating_sub(1));
    let retained_end = retained_end.min(states.len().saturating_sub(1));
    let samples = states
        .iter()
        .zip(progress.iter().copied())
        .take(retained_end + 1)
        .map(|(state, source_progress_m)| {
            source_transition_sample(input, state, source_progress_m, &outbound)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let initial_anchor = SourceTransitionInitialAnchor {
        state: states[0],
        source_progress_m: progress[0],
        normalized_progress: progress[0] / input.geometry.direct_horizontal_span_m,
    };
    let contact_end = contact_boundary
        .as_ref()
        .map(|bracket| bracket.after.sample_index)
        .unwrap_or(retained_end);
    let pad_end = contact_end.min(retained_end);
    let pad_departure_extrema = Some(extrema(&samples[0..=pad_end]));
    let acquisition_extrema = contact_boundary.as_ref().map(|contact| {
        let acquisition_start = contact.after.sample_index.min(retained_end);
        extrema(&samples[acquisition_start..=retained_end])
    });

    let all_crossings = all_boundary_crossings(input.profile, &states, &progress);
    let (backtracking_events, reentry_events) = boundary_events(
        input.profile,
        &states,
        &progress,
        contact_boundary.as_ref(),
        tracking_boundary.as_ref(),
    );
    let audit = build_audit(
        input,
        &states,
        retained_end,
        contact_boundary.as_ref(),
        tracking_boundary.as_ref(),
    );
    let physical_digest = digest_serialized(&PhysicalDigestMaterial {
        source_transition_start_m: input.profile.source_transition_start_m,
        source_transition_end_m: input.profile.source_transition_end_m,
        initial_anchor: &initial_anchor,
        contact_exit: &contact_boundary,
        tracking_entry: &tracking_boundary,
        boundary_centerline_references: &boundary_centerline_references,
        first_outbound_reference: &Some(outbound.clone()),
        samples: &samples,
        pad_departure_extrema: &pad_departure_extrema,
        acquisition_extrema: &acquisition_extrema,
        boundary_crossings: &all_crossings,
        backtracking_events: &backtracking_events,
        reentry_events: &reentry_events,
    });

    Ok(SourceTransitionKernelOutput {
        initial_anchor,
        contact_exit: contact_boundary,
        tracking_entry: tracking_boundary,
        boundary_centerline_references,
        first_outbound_reference: outbound,
        samples,
        pad_departure_extrema,
        acquisition_extrema,
        boundary_crossings: all_crossings,
        backtracking_events,
        reentry_events,
        audit,
        physical_digest,
    })
}

/// Output of the pure kernel before post-run status/censoring is assigned.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceTransitionKernelOutput {
    pub initial_anchor: SourceTransitionInitialAnchor,
    pub contact_exit: Option<SourceTransitionBoundaryBracket>,
    pub tracking_entry: Option<SourceTransitionBoundaryBracket>,
    pub boundary_centerline_references: Vec<SourceTransitionCenterlineReference>,
    pub first_outbound_reference: SourceTransitionOutboundReference,
    pub samples: Vec<SourceTransitionSample>,
    pub pad_departure_extrema: Option<SourceTransitionExtrema>,
    pub acquisition_extrema: Option<SourceTransitionExtrema>,
    pub boundary_crossings: Vec<SourceTransitionCrossing>,
    pub backtracking_events: Vec<SourceTransitionBoundaryEvent>,
    pub reentry_events: Vec<SourceTransitionBoundaryEvent>,
    pub audit: SourceTransitionAudit,
    pub physical_digest: String,
}

/// Assemble a neutral artifact from the pure kernel and post-run terminal
/// metadata.  The terminal fields are used only for censor classification and
/// audit; they never enter the kernel's crossing math.
pub fn assemble_source_transition_evidence(
    input: &SourceTransitionKernelInput<'_>,
    mut provenance: SourceTransitionProvenance,
    terminal: Option<SourceTransitionTerminal>,
) -> SourceTransitionEvidence {
    fill_raw_digests(&mut provenance, input);
    let kernel = match extract_source_transition_kernel(input) {
        Ok(kernel) => kernel,
        Err(reason) => return invalid_evidence(input, provenance, terminal, reason),
    };

    let status = classify_status(input, &kernel, terminal.as_ref());
    let (status, invalid_reason) = match status {
        Ok(status) => (status, None),
        Err(reason) => (SourceTransitionEvidenceStatus::Invalid, Some(reason)),
    };
    let evidence = SourceTransitionEvidence {
        schema_version: SOURCE_TRANSITION_SCHEMA_VERSION,
        extractor_version: SOURCE_TRANSITION_EXTRACTOR_VERSION.to_owned(),
        evidence_digest: String::new(),
        physical_digest: String::new(),
        status,
        invalid_reason,
        provenance,
        cadence: input.cadence,
        source_transition_start_m: input.profile.source_transition_start_m,
        source_transition_end_m: input.profile.source_transition_end_m,
        initial_anchor: Some(kernel.initial_anchor),
        contact_exit: kernel.contact_exit,
        tracking_entry: kernel.tracking_entry,
        boundary_centerline_references: kernel.boundary_centerline_references,
        first_outbound_reference: Some(kernel.first_outbound_reference),
        samples: kernel.samples,
        pad_departure_extrema: kernel.pad_departure_extrema,
        acquisition_extrema: kernel.acquisition_extrema,
        boundary_crossings: kernel.boundary_crossings,
        backtracking_events: kernel.backtracking_events,
        reentry_events: kernel.reentry_events,
        terminal,
        audit: kernel.audit,
    };
    seal_source_transition_evidence(evidence)
}

/// Assemble source evidence directly from the existing serialized run bundle
/// and route plan.  This is an explicit extraction entrypoint; it does not
/// alter simulation, planner, controller, or maintained pack behavior.
pub fn assemble_source_transition_evidence_from_artifacts(
    scenario: &ScenarioSpec,
    route_plan: &RoutePlan,
    artifacts: &RunArtifacts,
    controller_updates: &[ControllerUpdateRecord],
    mut provenance: SourceTransitionProvenance,
) -> SourceTransitionEvidence {
    // The artifact wrapper owns raw-bundle identity.  Recompute these values
    // from the complete serialized run contents so a caller cannot silently
    // substitute a samples/actions-only digest (or stale provenance).
    provenance.samples_digest = digest_serialized(&artifacts.samples);
    provenance.action_log_digest = digest_serialized(&artifacts.actions);
    provenance.raw_bundle_digest =
        source_transition_raw_bundle_digest(artifacts, controller_updates);
    let terminal = source_transition_terminal_from_manifest(&artifacts.manifest);
    if artifacts.manifest.physics_hz != scenario.sim.physics_hz {
        return invalid_artifact_evidence(
            scenario,
            route_plan,
            artifacts,
            controller_updates,
            provenance,
            SourceTransitionInvalidReason::PhysicsRateMismatch {
                scenario_physics_hz: scenario.sim.physics_hz,
                manifest_physics_hz: artifacts.manifest.physics_hz,
            },
            terminal,
        );
    }
    let Some(source_pad) = scenario.world.landing_pad(&route_plan.route.source_pad_id) else {
        return invalid_artifact_evidence(
            scenario,
            route_plan,
            artifacts,
            controller_updates,
            provenance,
            SourceTransitionInvalidReason::InvalidGeometry {
                message: "route source pad is not present in scenario world".to_owned(),
            },
            terminal,
        );
    };
    let Some(target_pad) = scenario.world.landing_pad(&route_plan.route.target_pad_id) else {
        return invalid_artifact_evidence(
            scenario,
            route_plan,
            artifacts,
            controller_updates,
            provenance,
            SourceTransitionInvalidReason::InvalidGeometry {
                message: "route target pad is not present in scenario world".to_owned(),
            },
            terminal,
        );
    };
    provenance.source_target_geometry_digest =
        source_transition_canonical_digest(&SourceTransitionGeometryIdentity {
            source_pad,
            target_pad,
            normalized_geometry: &route_plan.normalized_geometry,
        });
    let request = pd_core::RoutePlanningRequest {
        world: scenario.world.clone(),
        vehicle: scenario.vehicle.clone(),
        initial_state: scenario.initial_state.clone(),
        source_pad_id: source_pad.id.clone(),
        target_pad_id: target_pad.id.clone(),
        policy: route_plan.policy.clone(),
    };
    let Ok((profile, _)) = pd_core::build_endpoint_profile(
        &request,
        route_plan.normalized_geometry.direct_horizontal_span_m,
    ) else {
        return invalid_artifact_evidence(
            scenario,
            route_plan,
            artifacts,
            controller_updates,
            provenance,
            SourceTransitionInvalidReason::InvalidGeometry {
                message: "route plan endpoint profile could not be resolved".to_owned(),
            },
            terminal,
        );
    };
    let selected_centerline = route_plan
        .diagnostics
        .selected_centerline_m
        .iter()
        .copied()
        .map(|point| {
            world_to_normalized(
                point,
                source_pad.center_x_m,
                route_plan.normalized_geometry.horizontal_sign,
            )
        })
        .collect::<Vec<_>>();
    let input = SourceTransitionKernelInput {
        terrain: &scenario.world.terrain,
        source_pad_center_x_m: source_pad.center_x_m,
        vehicle: &scenario.vehicle,
        initial_state: &scenario.initial_state,
        geometry: &route_plan.normalized_geometry,
        profile: &profile,
        selected_centerline_m: &selected_centerline,
        waypoints: &route_plan.route.waypoints,
        samples: &artifacts.samples,
        cadence: SourceTransitionCadence {
            sample_hz: scenario.sim.sample_hz,
            physics_hz: artifacts.manifest.physics_hz,
        },
        audit: SourceTransitionAuditInput {
            actions: &artifacts.actions,
            controller_updates,
        },
    };
    assemble_source_transition_evidence(&input, provenance, terminal)
}

/// Convenience wrapper for a controller-produced bundle.  The controller
/// identity and performance statistics remain outside the evidence DTO.
pub fn assemble_source_transition_evidence_from_controlled_artifacts(
    scenario: &ScenarioSpec,
    route_plan: &RoutePlan,
    artifacts: &ControlledRunArtifacts,
    provenance: SourceTransitionProvenance,
) -> SourceTransitionEvidence {
    assemble_source_transition_evidence_from_artifacts(
        scenario,
        route_plan,
        &artifacts.run,
        &artifacts.controller_updates,
        provenance,
    )
}

/// Extract terminal step/reason without exposing mission or physical outcome
/// fields to the crossing kernel.
pub fn source_transition_terminal_from_manifest(
    manifest: &RunManifest,
) -> Option<SourceTransitionTerminal> {
    let reason = serde_json::to_string(&manifest.end_reason)
        .ok()?
        .trim_matches('"')
        .to_owned();
    (!matches!(&manifest.end_reason, EndReason::Running)).then_some(SourceTransitionTerminal {
        physics_step: manifest.physics_steps,
        reason,
    })
}

/// Build complete provenance from a resolved scenario, route plan, and the
/// serialized post-run artifacts.  Source/target pad geometry and the raw
/// bundle identity are derived here rather than accepted as caller guesses.
pub fn source_transition_provenance_for_route_plan(
    scenario: &ScenarioSpec,
    route_plan: &RoutePlan,
    artifacts: &RunArtifacts,
    controller_updates: &[ControllerUpdateRecord],
    artifact_identity: impl Into<String>,
) -> Result<SourceTransitionProvenance> {
    let source_pad = scenario
        .world
        .landing_pad(&route_plan.route.source_pad_id)
        .with_context(|| {
            format!(
                "route source pad '{}' is not present in scenario world",
                route_plan.route.source_pad_id
            )
        })?;
    let target_pad = scenario
        .world
        .landing_pad(&route_plan.route.target_pad_id)
        .with_context(|| {
            format!(
                "route target pad '{}' is not present in scenario world",
                route_plan.route.target_pad_id
            )
        })?;
    Ok(SourceTransitionProvenance {
        resolved_input_digest: String::new(),
        request_digest: route_plan.request_digest.clone(),
        policy_digest: source_transition_canonical_digest(&route_plan.policy),
        route_plan_digest: route_plan.plan_digest.clone(),
        source_target_geometry_digest: source_transition_canonical_digest(
            &SourceTransitionGeometryIdentity {
                source_pad,
                target_pad,
                normalized_geometry: &route_plan.normalized_geometry,
            },
        ),
        raw_bundle_digest: source_transition_raw_bundle_digest(artifacts, controller_updates),
        samples_digest: digest_serialized(&artifacts.samples),
        action_log_digest: digest_serialized(&artifacts.actions),
        artifact_identity: artifact_identity.into(),
    })
}

/// Compute a raw-bundle digest from all execution records that identify a
/// run.  Controller performance timing is deliberately omitted because it is
/// observationally nondeterministic; stable frame contents remain included.
pub fn source_transition_raw_bundle_digest(
    artifacts: &RunArtifacts,
    controller_updates: &[ControllerUpdateRecord],
) -> String {
    let controller_updates = controller_updates
        .iter()
        .map(|update| StableControllerUpdate {
            sim_time_s: update.sim_time_s,
            physics_step: update.physics_step,
            controller_update_index: update.controller_update_index,
            frame: &update.frame,
        })
        .collect::<Vec<_>>();
    digest_serialized(&SourceTransitionRawBundleDigestMaterial {
        manifest: &artifacts.manifest,
        actions: &artifacts.actions,
        events: &artifacts.events,
        samples: &artifacts.samples,
        controller_updates,
    })
}

#[derive(Clone, Debug, Serialize)]
struct SourceTransitionResolvedInputMaterial<'a> {
    run_id: &'a str,
    scenario: &'a ScenarioSpec,
    route_plan: &'a RoutePlan,
    controller: &'a ControllerSpec,
}

/// Digest the fully resolved input identity used by the development gate.
/// This is separate from source evidence and intentionally remains outside
/// the pure physical feature DTO.
pub fn source_transition_resolved_input_digest(
    run_id: &str,
    scenario: &ScenarioSpec,
    route_plan: &RoutePlan,
    controller: &ControllerSpec,
) -> String {
    let mut normalized_scenario = scenario.clone();
    normalized_scenario.sim.sample_hz = None;
    digest_serialized(&SourceTransitionResolvedInputMaterial {
        run_id,
        scenario: &normalized_scenario,
        route_plan,
        controller,
    })
}

/// Stable digest of the serialized evidence artifact, excluding its own
/// digest field.
pub fn source_transition_evidence_digest(evidence: &SourceTransitionEvidence) -> String {
    let mut material = evidence.clone();
    material.evidence_digest.clear();
    digest_serialized(&material)
}

/// Recompute the neutral physical payload digest without provenance, terminal,
/// or audit fields.  D1 capability fitting uses this to reject physical-field
/// tampering while keeping audit-only mutations behavior-neutral.
pub fn source_transition_physical_digest(
    evidence: &SourceTransitionEvidence,
) -> Result<String, String> {
    let initial_anchor = evidence
        .initial_anchor
        .as_ref()
        .ok_or_else(|| "source evidence has no initial anchor".to_owned())?;
    Ok(digest_serialized(&PhysicalDigestMaterial {
        source_transition_start_m: evidence.source_transition_start_m,
        source_transition_end_m: evidence.source_transition_end_m,
        initial_anchor,
        contact_exit: &evidence.contact_exit,
        tracking_entry: &evidence.tracking_entry,
        boundary_centerline_references: &evidence.boundary_centerline_references,
        first_outbound_reference: &evidence.first_outbound_reference,
        samples: &evidence.samples,
        pad_departure_extrema: &evidence.pad_departure_extrema,
        acquisition_extrema: &evidence.acquisition_extrema,
        boundary_crossings: &evidence.boundary_crossings,
        backtracking_events: &evidence.backtracking_events,
        reentry_events: &evidence.reentry_events,
    }))
}

/// Validate the schema and physical structure of a persisted neutral source
/// artifact.  Digest checks alone are insufficient because a malformed JSON
/// artifact can be resealed with internally consistent digests.
pub fn validate_persisted_source_transition_evidence(
    evidence: &SourceTransitionEvidence,
) -> Result<(), String> {
    if evidence.schema_version != SOURCE_TRANSITION_SCHEMA_VERSION {
        return Err(format!(
            "source evidence schema_version must equal {SOURCE_TRANSITION_SCHEMA_VERSION}"
        ));
    }
    if evidence.extractor_version != SOURCE_TRANSITION_EXTRACTOR_VERSION {
        return Err(format!(
            "source evidence extractor_version must equal {SOURCE_TRANSITION_EXTRACTOR_VERSION}"
        ));
    }
    if evidence.evidence_digest != source_transition_evidence_digest(evidence) {
        return Err("source evidence digest mismatch".to_owned());
    }
    if evidence.status == SourceTransitionEvidenceStatus::Invalid {
        return evidence
            .invalid_reason
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| "invalid source evidence has no invalid_reason".to_owned());
    }
    if evidence.invalid_reason.is_some() {
        return Err("non-invalid source evidence carries an invalid_reason".to_owned());
    }
    if evidence.cadence.physics_hz == 0
        || evidence.cadence.sample_hz != Some(evidence.cadence.physics_hz)
    {
        return Err("source evidence is not sampled at a positive physics rate".to_owned());
    }
    if !evidence.source_transition_start_m.is_finite()
        || !evidence.source_transition_end_m.is_finite()
        || evidence.source_transition_start_m < 0.0
        || evidence.source_transition_end_m < evidence.source_transition_start_m
    {
        return Err("source transition bounds are not finite and ordered".to_owned());
    }
    if evidence.physical_digest != source_transition_physical_digest(evidence)? {
        return Err("source physical digest mismatch".to_owned());
    }
    let initial = evidence
        .initial_anchor
        .as_ref()
        .ok_or_else(|| "source evidence has no initial anchor".to_owned())?;
    if evidence.samples.is_empty()
        || !source_raw_state_is_finite(&initial.state)
        || !initial.source_progress_m.is_finite()
        || !initial.normalized_progress.is_finite()
        || initial.state.sample_index != 0
    {
        return Err("source initial anchor is malformed".to_owned());
    }
    for (index, sample) in evidence.samples.iter().enumerate() {
        if sample.sample_index != index
            || sample.state.sample_index != index
            || sample.physics_step != sample.state.physics_step
            || sample.sim_time_s != sample.state.sim_time_s
            || (index == 0 && sample.physics_step != 0)
            || (index > 0
                && evidence.samples[index - 1].physics_step.checked_add(1)
                    != Some(sample.physics_step))
            || !source_transition_sample_is_finite(sample)
        {
            return Err(format!(
                "source sample {index} is malformed or non-contiguous"
            ));
        }
    }
    if initial.state != evidence.samples[0].state {
        return Err("source initial anchor does not match sample zero".to_owned());
    }
    validate_persisted_source_bracket(
        evidence,
        SourceTransitionBoundary::ContactExit,
        evidence.source_transition_start_m,
        evidence.contact_exit.as_ref(),
    )?;
    validate_persisted_source_bracket(
        evidence,
        SourceTransitionBoundary::TrackingEntry,
        evidence.source_transition_end_m,
        evidence.tracking_entry.as_ref(),
    )?;
    match evidence.status {
        SourceTransitionEvidenceStatus::Complete => {
            if evidence.contact_exit.is_none() || evidence.tracking_entry.is_none() {
                return Err("complete source evidence is missing a boundary".to_owned());
            }
        }
        SourceTransitionEvidenceStatus::CensoredBeforeContactExit => {
            if evidence.contact_exit.is_some() || evidence.tracking_entry.is_some() {
                return Err("pre-contact censor carries a later boundary".to_owned());
            }
        }
        SourceTransitionEvidenceStatus::CensoredBeforeTrackingEntry => {
            if evidence.contact_exit.is_none() || evidence.tracking_entry.is_some() {
                return Err("pre-tracking censor has inconsistent boundaries".to_owned());
            }
        }
        SourceTransitionEvidenceStatus::Invalid => unreachable!(),
    }
    if evidence.first_outbound_reference.is_none() {
        return Err("source evidence has no outbound reference".to_owned());
    }
    Ok(())
}

fn source_raw_state_is_finite(state: &SourceTransitionRawState) -> bool {
    [
        state.sim_time_s,
        state.position_m.x,
        state.position_m.y,
        state.velocity_mps.x,
        state.velocity_mps.y,
        state.attitude_rad,
        state.angular_rate_radps,
        state.mass_kg,
        state.fuel_kg,
    ]
    .iter()
    .all(|value| value.is_finite())
}

fn source_transition_sample_is_finite(sample: &SourceTransitionSample) -> bool {
    source_raw_state_is_finite(&sample.state)
        && [
            sample.sim_time_s,
            sample.source_progress_m,
            sample.normalized_progress,
            sample.path.normalized_position_m.x,
            sample.path.normalized_position_m.y,
            sample.path.along_track_m,
            sample.path.cross_track_m,
            sample.path.velocity_along_track_mps,
            sample.path.velocity_cross_track_mps,
            sample.clearance.observed_hull_clearance_m,
            sample
                .clearance
                .observed_touchdown_clearance_m
                .unwrap_or(0.0),
            sample.clearance.contact_envelope.minimum_clearance_m,
            sample.clearance.resolved_envelope.minimum_clearance_m,
            sample.clearance.full_envelope.minimum_clearance_m,
        ]
        .iter()
        .all(|value| value.is_finite())
}

fn validate_persisted_source_bracket(
    evidence: &SourceTransitionEvidence,
    boundary: SourceTransitionBoundary,
    expected_boundary_m: f64,
    bracket: Option<&SourceTransitionBoundaryBracket>,
) -> Result<(), String> {
    let Some(bracket) = bracket else {
        return Ok(());
    };
    if bracket.boundary != boundary
        || bracket.boundary_m != expected_boundary_m
        || bracket.before.sample_index.checked_add(1) != Some(bracket.after.sample_index)
        || bracket.before.sample_index >= evidence.samples.len()
        || bracket.after.sample_index >= evidence.samples.len()
        || bracket.before != evidence.samples[bracket.before.sample_index].state
        || bracket.after != evidence.samples[bracket.after.sample_index].state
        || !bracket.before_source_progress_m.is_finite()
        || !bracket.after_source_progress_m.is_finite()
        || bracket.before_source_progress_m >= expected_boundary_m
        || bracket.after_source_progress_m < expected_boundary_m
        || !bracket.interpolation_fraction.is_finite()
        || !(0.0..=1.0).contains(&bracket.interpolation_fraction)
    {
        return Err(format!("persisted {boundary:?} bracket is malformed"));
    }
    Ok(())
}

/// Stable digest helper used for evidence identities and manifest checks.
pub fn source_transition_canonical_digest<T: Serialize>(value: &T) -> String {
    digest_serialized(value)
}

/// Write the neutral evidence and its audit/provenance sidecar.  Given
/// `path=/tmp/source.json`, the sidecar is `/tmp/source.audit.json`.
pub fn write_source_transition_artifacts(
    path: &Path,
    evidence: &SourceTransitionEvidence,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create evidence directory {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(evidence)?;
    fs::write(path, raw)
        .with_context(|| format!("failed to write source evidence {}", path.display()))?;
    let sidecar_path = path.with_extension("audit.json");
    let sidecar = SourceTransitionAuditSidecar {
        schema_version: evidence.schema_version,
        extractor_version: evidence.extractor_version.clone(),
        evidence_digest: evidence.evidence_digest.clone(),
        status: evidence.status,
        provenance: evidence.provenance.clone(),
        cadence: evidence.cadence,
        terminal: evidence.terminal.clone(),
        audit: evidence.audit.clone(),
    };
    fs::write(&sidecar_path, serde_json::to_string_pretty(&sidecar)?)
        .with_context(|| format!("failed to write source audit {}", sidecar_path.display()))?;
    Ok(())
}

/// Persist only the physics-rate raw records required to regenerate D0a
/// evidence.  Ordinary-cadence captures stay parity-only.  Controller update
/// records retain their compute-time fields for audit; the deterministic raw
/// bundle digest normalizes those observational timings out.
pub fn write_source_transition_raw_bundle(
    directory: &Path,
    scenario: &ScenarioSpec,
    route_plan: &RoutePlan,
    artifacts: &ControlledRunArtifacts,
) -> Result<()> {
    fs::create_dir_all(directory).with_context(|| {
        format!(
            "failed to create source-transition raw bundle directory {}",
            directory.display()
        )
    })?;
    // Keep this bundle explicit and inspectable.  Controller update records
    // retain their source fields, while source_transition_raw_bundle_digest
    // normalizes away only compute_time_us for deterministic identity.
    write_source_transition_json(directory, "scenario.json", scenario)?;
    write_source_transition_json(directory, "route_plan.json", route_plan)?;
    write_source_transition_json(directory, "manifest.json", &artifacts.run.manifest)?;
    write_source_transition_json(directory, "actions.json", &artifacts.run.actions)?;
    write_source_transition_json(directory, "events.json", &artifacts.run.events)?;
    write_source_transition_json(directory, "samples.json", &artifacts.run.samples)?;
    write_source_transition_json(
        directory,
        "controller_updates.json",
        &artifacts.controller_updates,
    )?;
    Ok(())
}

fn write_source_transition_json<T: Serialize>(
    directory: &Path,
    name: &str,
    value: &T,
) -> Result<()> {
    let path = directory.join(name);
    fs::write(&path, serde_json::to_string_pretty(value)?).with_context(|| {
        format!(
            "failed to write source-transition raw record {}",
            path.display()
        )
    })?;
    Ok(())
}

/// Deterministic execution-fidelity comparison between an ordinary-cadence
/// capture and its physics-rate evidence capture.  The ordinary bundle is
/// never promoted; this result is only a gate on whether the physics-rate
/// bundle can be used for extraction.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SourceTransitionCadenceParity {
    pub passed: bool,
    pub ordinary_sample_hz: Option<u32>,
    pub physics_sample_hz: Option<u32>,
    pub normalized_inputs_equal: bool,
    pub actions_equal: bool,
    pub controller_updates_equal: bool,
    pub events_equal: bool,
    pub terminal_step_reason_equal: bool,
    pub outcomes_equal: bool,
    pub terminal_state_equal: bool,
    pub shared_cadence_samples_equal: bool,
    pub physics_samples_contiguous: bool,
    pub ordinary_shared_sample_steps: Vec<u64>,
    pub mismatch_reasons: Vec<String>,
}

/// Compare captures after normalizing only the sample-retention setting and
/// controller compute timing.  Terminal samples emitted off the ordinary
/// cadence remain explicit shared-cadence parity evidence.
pub fn compare_source_transition_cadence_parity(
    ordinary_scenario: &ScenarioSpec,
    ordinary: &ControlledRunArtifacts,
    physics_scenario: &ScenarioSpec,
    physics: &ControlledRunArtifacts,
) -> SourceTransitionCadenceParity {
    let ordinary_sample_hz = ordinary_scenario.sim.sample_hz;
    let physics_sample_hz = physics_scenario.sim.sample_hz;
    let mut mismatch_reasons = Vec::new();

    let normalized_inputs_equal = normalized_scenario_digest(ordinary_scenario)
        == normalized_scenario_digest(physics_scenario);
    if !normalized_inputs_equal {
        mismatch_reasons.push("resolved_inputs_differ".to_owned());
    }

    let ordinary_manifest_rate_matches =
        ordinary.run.manifest.physics_hz == ordinary_scenario.sim.physics_hz;
    if !ordinary_manifest_rate_matches {
        mismatch_reasons.push("ordinary_manifest_physics_rate_differs".to_owned());
    }
    let physics_manifest_rate_matches =
        physics.run.manifest.physics_hz == physics_scenario.sim.physics_hz;
    if !physics_manifest_rate_matches {
        mismatch_reasons.push("physics_manifest_physics_rate_differs".to_owned());
    }

    let actions_equal = ordinary.run.actions == physics.run.actions;
    if !actions_equal {
        mismatch_reasons.push("actions_differ".to_owned());
    }
    let controller_updates_equal = normalized_controller_updates_equal(
        &ordinary.controller_updates,
        &physics.controller_updates,
    );
    if !controller_updates_equal {
        mismatch_reasons.push("controller_updates_differ".to_owned());
    }
    let events_equal = ordinary.run.events == physics.run.events;
    if !events_equal {
        mismatch_reasons.push("events_differ".to_owned());
    }
    let terminal_step_reason_equal = ordinary.run.manifest.physics_steps
        == physics.run.manifest.physics_steps
        && ordinary.run.manifest.end_reason == physics.run.manifest.end_reason;
    if !terminal_step_reason_equal {
        mismatch_reasons.push("terminal_step_or_reason_differs".to_owned());
    }
    let outcomes_equal = ordinary.run.manifest.physical_outcome
        == physics.run.manifest.physical_outcome
        && ordinary.run.manifest.mission_outcome == physics.run.manifest.mission_outcome;
    if !outcomes_equal {
        mismatch_reasons.push("outcomes_differ".to_owned());
    }
    let terminal_state_equal = ordinary
        .run
        .samples
        .last()
        .zip(physics.run.samples.last())
        .is_some_and(|(ordinary, physics)| raw_sample_equal(ordinary, physics));
    if !terminal_state_equal {
        mismatch_reasons.push("terminal_state_differs".to_owned());
    }

    let (ordinary_shared_sample_steps, shared_cadence_samples_equal) =
        shared_cadence_sample_parity(ordinary_scenario, ordinary, physics);
    if !shared_cadence_samples_equal {
        mismatch_reasons.push("shared_cadence_raw_samples_differ".to_owned());
    }

    let physics_samples_contiguous =
        physics_samples_are_contiguous(&physics.run.samples, physics.run.manifest.physics_hz);
    if !physics_samples_contiguous {
        mismatch_reasons.push("physics_capture_samples_not_contiguous".to_owned());
    }

    let physics_capture_is_physics_rate = physics_sample_hz
        .zip(Some(physics_scenario.sim.physics_hz))
        .is_some_and(|(sample_hz, physics_hz)| sample_hz == physics_hz)
        && physics_manifest_rate_matches
        && physics_samples_contiguous;
    if !physics_capture_is_physics_rate {
        mismatch_reasons.push("physics_capture_is_not_physics_rate".to_owned());
    }
    let passed = mismatch_reasons.is_empty();
    SourceTransitionCadenceParity {
        passed,
        ordinary_sample_hz,
        physics_sample_hz,
        normalized_inputs_equal,
        actions_equal,
        controller_updates_equal,
        events_equal,
        terminal_step_reason_equal,
        outcomes_equal,
        terminal_state_equal,
        shared_cadence_samples_equal,
        physics_samples_contiguous,
        ordinary_shared_sample_steps,
        mismatch_reasons,
    }
}

/// Alias with a concise name for callers implementing the development
/// manifest's parity gate.
pub fn check_source_transition_cadence_parity(
    ordinary_scenario: &ScenarioSpec,
    ordinary: &ControlledRunArtifacts,
    physics_scenario: &ScenarioSpec,
    physics: &ControlledRunArtifacts,
) -> SourceTransitionCadenceParity {
    compare_source_transition_cadence_parity(ordinary_scenario, ordinary, physics_scenario, physics)
}

fn normalized_scenario_digest(scenario: &ScenarioSpec) -> String {
    let mut normalized = scenario.clone();
    normalized.sim.sample_hz = None;
    digest_serialized(&normalized)
}

fn normalized_controller_updates_equal(
    ordinary: &[ControllerUpdateRecord],
    physics: &[ControllerUpdateRecord],
) -> bool {
    ordinary.len() == physics.len()
        && ordinary.iter().zip(physics).all(|(ordinary, physics)| {
            ordinary.sim_time_s == physics.sim_time_s
                && ordinary.physics_step == physics.physics_step
                && ordinary.controller_update_index == physics.controller_update_index
                && ordinary.frame == physics.frame
        })
}

fn raw_sample_equal(ordinary: &SampleRecord, physics: &SampleRecord) -> bool {
    ordinary.physics_step == physics.physics_step
        && ordinary.sim_time_s == physics.sim_time_s
        && ordinary.observation == physics.observation
}

fn shared_cadence_sample_parity(
    ordinary_scenario: &ScenarioSpec,
    ordinary: &ControlledRunArtifacts,
    physics: &ControlledRunArtifacts,
) -> (Vec<u64>, bool) {
    let Some(sample_hz) = ordinary_scenario.sim.sample_hz else {
        return (Vec::new(), false);
    };
    if sample_hz == 0
        || ordinary.run.manifest.physics_hz == 0
        || !ordinary.run.manifest.physics_hz.is_multiple_of(sample_hz)
    {
        return (Vec::new(), false);
    }
    let interval = u64::from(ordinary.run.manifest.physics_hz / sample_hz);
    let terminal_step = ordinary.run.manifest.physics_steps;
    let mut ordinary_shared = ordinary
        .run
        .samples
        .iter()
        .filter(|sample| {
            sample.physics_step.is_multiple_of(interval) || sample.physics_step == terminal_step
        })
        .collect::<Vec<_>>();
    ordinary_shared.sort_by_key(|sample| sample.physics_step);
    ordinary_shared.dedup_by_key(|sample| sample.physics_step);
    let steps = ordinary_shared
        .iter()
        .map(|sample| sample.physics_step)
        .collect::<Vec<_>>();
    let terminal_present = ordinary_shared
        .iter()
        .any(|sample| sample.physics_step == terminal_step);
    let expected_steps = (0..=terminal_step)
        .filter(|step| step.is_multiple_of(interval))
        .chain((!terminal_step.is_multiple_of(interval)).then_some(terminal_step))
        .collect::<Vec<_>>();
    let equal = terminal_present
        && steps == expected_steps
        && ordinary_shared.iter().all(|ordinary_sample| {
            physics
                .run
                .samples
                .iter()
                .find(|physics_sample| physics_sample.physics_step == ordinary_sample.physics_step)
                .is_some_and(|physics_sample| raw_sample_equal(physics_sample, ordinary_sample))
        });
    (steps, equal)
}

fn physics_samples_are_contiguous(samples: &[SampleRecord], physics_hz: u32) -> bool {
    let Some(first) = samples.first() else {
        return false;
    };
    physics_hz > 0
        && first.physics_step == 0
        && samples.iter().enumerate().all(|(index, sample)| {
            sample.sim_time_s.is_finite()
                && (sample.sim_time_s - sample.physics_step as f64 / f64::from(physics_hz)).abs()
                    <= TIME_TOLERANCE_S
                && sample.physics_step == sample.observation.physics_step
                && approx_eq(sample.sim_time_s, sample.observation.sim_time_s)
                && (index == 0
                    || samples[index - 1].physics_step.checked_add(1) == Some(sample.physics_step))
        })
}

fn invalid_artifact_evidence(
    scenario: &ScenarioSpec,
    route_plan: &RoutePlan,
    artifacts: &RunArtifacts,
    controller_updates: &[ControllerUpdateRecord],
    provenance: SourceTransitionProvenance,
    reason: SourceTransitionInvalidReason,
    terminal: Option<SourceTransitionTerminal>,
) -> SourceTransitionEvidence {
    let Some(source_pad) = scenario.world.landing_pad(&route_plan.route.source_pad_id) else {
        return invalid_minimal_evidence(
            provenance,
            SourceTransitionCadence {
                sample_hz: scenario.sim.sample_hz,
                physics_hz: artifacts.manifest.physics_hz,
            },
            terminal,
            reason,
            &artifacts.samples,
            &artifacts.actions,
            controller_updates,
        );
    };
    let Ok((profile, _)) = pd_core::build_endpoint_profile(
        &pd_core::RoutePlanningRequest {
            world: scenario.world.clone(),
            vehicle: scenario.vehicle.clone(),
            initial_state: scenario.initial_state.clone(),
            source_pad_id: route_plan.route.source_pad_id.clone(),
            target_pad_id: route_plan.route.target_pad_id.clone(),
            policy: route_plan.policy.clone(),
        },
        route_plan.normalized_geometry.direct_horizontal_span_m,
    ) else {
        return invalid_minimal_evidence(
            provenance,
            SourceTransitionCadence {
                sample_hz: scenario.sim.sample_hz,
                physics_hz: artifacts.manifest.physics_hz,
            },
            terminal,
            reason,
            &artifacts.samples,
            &artifacts.actions,
            controller_updates,
        );
    };
    let selected_centerline = route_plan
        .diagnostics
        .selected_centerline_m
        .iter()
        .copied()
        .map(|point| {
            world_to_normalized(
                point,
                source_pad.center_x_m,
                route_plan.normalized_geometry.horizontal_sign,
            )
        })
        .collect::<Vec<_>>();
    let input = SourceTransitionKernelInput {
        terrain: &scenario.world.terrain,
        source_pad_center_x_m: source_pad.center_x_m,
        vehicle: &scenario.vehicle,
        initial_state: &scenario.initial_state,
        geometry: &route_plan.normalized_geometry,
        profile: &profile,
        selected_centerline_m: &selected_centerline,
        waypoints: &route_plan.route.waypoints,
        samples: &artifacts.samples,
        cadence: SourceTransitionCadence {
            sample_hz: scenario.sim.sample_hz,
            physics_hz: artifacts.manifest.physics_hz,
        },
        audit: SourceTransitionAuditInput {
            actions: &artifacts.actions,
            controller_updates,
        },
    };
    let mut evidence = invalid_minimal_evidence(
        provenance,
        input.cadence,
        terminal,
        reason,
        input.samples,
        input.audit.actions,
        input.audit.controller_updates,
    );
    evidence.source_transition_start_m = profile.source_transition_start_m;
    evidence.source_transition_end_m = profile.source_transition_end_m;
    evidence
}

fn invalid_evidence(
    input: &SourceTransitionKernelInput<'_>,
    provenance: SourceTransitionProvenance,
    terminal: Option<SourceTransitionTerminal>,
    reason: SourceTransitionInvalidReason,
) -> SourceTransitionEvidence {
    invalid_minimal_evidence(
        provenance,
        input.cadence,
        terminal,
        reason,
        input.samples,
        input.audit.actions,
        input.audit.controller_updates,
    )
}

fn invalid_minimal_evidence(
    mut provenance: SourceTransitionProvenance,
    cadence: SourceTransitionCadence,
    terminal: Option<SourceTransitionTerminal>,
    reason: SourceTransitionInvalidReason,
    samples: &[SampleRecord],
    actions: &[ActionLogEntry],
    controller_updates: &[ControllerUpdateRecord],
) -> SourceTransitionEvidence {
    provenance.samples_digest = digest_serialized(&samples);
    provenance.action_log_digest = digest_serialized(&actions);
    // A direct kernel call has no RunManifest/events context and therefore
    // cannot claim a complete raw-bundle identity.  Leave the field empty;
    // the serialized-artifact wrapper above always derives the complete
    // digest before extraction.
    let evidence = SourceTransitionEvidence {
        schema_version: SOURCE_TRANSITION_SCHEMA_VERSION,
        extractor_version: SOURCE_TRANSITION_EXTRACTOR_VERSION.to_owned(),
        evidence_digest: String::new(),
        physical_digest: String::new(),
        status: SourceTransitionEvidenceStatus::Invalid,
        invalid_reason: Some(reason),
        provenance,
        cadence,
        // Keep invalid artifacts themselves JSON-serializable.  The profile
        // coordinates are unavailable, so zero is an explicit placeholder;
        // `status` and `invalid_reason` carry the authoritative explanation.
        source_transition_start_m: 0.0,
        source_transition_end_m: 0.0,
        initial_anchor: None,
        contact_exit: None,
        tracking_entry: None,
        boundary_centerline_references: Vec::new(),
        first_outbound_reference: None,
        samples: Vec::new(),
        pad_departure_extrema: None,
        acquisition_extrema: None,
        boundary_crossings: Vec::new(),
        backtracking_events: Vec::new(),
        reentry_events: Vec::new(),
        terminal,
        audit: fallback_audit(samples, actions, controller_updates),
    };
    seal_source_transition_evidence(evidence)
}

fn seal_source_transition_evidence(
    mut evidence: SourceTransitionEvidence,
) -> SourceTransitionEvidence {
    evidence.evidence_digest.clear();
    evidence.physical_digest.clear();
    // Canonical artifacts are sealed from the representation they persist.
    // The workspace enables serde_json's exact float round-trip parser; this
    // bounded loop is a defensive fixed-point check rather than quantization.
    for _ in 0..8 {
        let Ok(bytes) = serde_json::to_vec(&evidence) else {
            break;
        };
        let Ok(next) = serde_json::from_slice::<SourceTransitionEvidence>(&bytes) else {
            break;
        };
        if next == evidence {
            break;
        }
        evidence = next;
    }
    evidence.physical_digest = source_transition_physical_digest(&evidence).unwrap_or_else(|_| {
        digest_serialized(&(
            evidence.invalid_reason.clone(),
            evidence.provenance.samples_digest.clone(),
            evidence.provenance.action_log_digest.clone(),
        ))
    });
    evidence.evidence_digest = source_transition_evidence_digest(&evidence);
    evidence
}

fn fill_raw_digests(
    provenance: &mut SourceTransitionProvenance,
    input: &SourceTransitionKernelInput<'_>,
) {
    provenance.samples_digest = digest_serialized(&input.samples);
    provenance.action_log_digest = digest_serialized(&input.audit.actions);
}

fn classify_status(
    input: &SourceTransitionKernelInput<'_>,
    kernel: &SourceTransitionKernelOutput,
    terminal: Option<&SourceTransitionTerminal>,
) -> Result<SourceTransitionEvidenceStatus, SourceTransitionInvalidReason> {
    if kernel.contact_exit.is_some() && kernel.tracking_entry.is_some() {
        return Ok(SourceTransitionEvidenceStatus::Complete);
    }
    let boundary = if kernel.contact_exit.is_none() {
        SourceTransitionBoundary::ContactExit
    } else {
        SourceTransitionBoundary::TrackingEntry
    };
    let Some(terminal) = terminal else {
        return Err(SourceTransitionInvalidReason::MissingTerminalCensor { boundary });
    };
    if !is_valid_terminal_reason(&terminal.reason) {
        return Err(SourceTransitionInvalidReason::InvalidTerminalReason);
    }
    let Some(last_sample) = input.samples.last() else {
        return Err(SourceTransitionInvalidReason::MissingSamples);
    };
    if terminal.physics_step != last_sample.physics_step {
        return Err(SourceTransitionInvalidReason::TerminalStepMismatch {
            terminal_step: terminal.physics_step,
            last_sample_step: last_sample.physics_step,
        });
    }
    Ok(match boundary {
        SourceTransitionBoundary::ContactExit => {
            SourceTransitionEvidenceStatus::CensoredBeforeContactExit
        }
        SourceTransitionBoundary::TrackingEntry => {
            SourceTransitionEvidenceStatus::CensoredBeforeTrackingEntry
        }
    })
}

fn is_valid_terminal_reason(reason: &str) -> bool {
    matches!(
        reason,
        "checkpoint_satisfied"
            | "checkpoint_failed"
            | "touchdown_on_target"
            | "touchdown_off_target"
            | "crash"
            | "max_time_reached"
    )
}

fn validate_kernel_input(
    input: &SourceTransitionKernelInput<'_>,
) -> Result<(), SourceTransitionInvalidReason> {
    if input.samples.is_empty() {
        return Err(SourceTransitionInvalidReason::MissingSamples);
    }
    let Some(sample_hz) = input.cadence.sample_hz else {
        return Err(SourceTransitionInvalidReason::MissingPhysicsRate);
    };
    if input.cadence.physics_hz == 0 {
        return Err(SourceTransitionInvalidReason::InvalidCadence {
            physics_hz: input.cadence.physics_hz,
        });
    }
    if sample_hz != input.cadence.physics_hz {
        return Err(SourceTransitionInvalidReason::WrongCadence {
            sample_hz,
            physics_hz: input.cadence.physics_hz,
        });
    }
    if !input.source_pad_center_x_m.is_finite() {
        return Err(SourceTransitionInvalidReason::InvalidGeometry {
            message: "source pad center must be finite".to_owned(),
        });
    }
    if !matches!(input.geometry.horizontal_sign, -1 | 1)
        || !input.geometry.direct_horizontal_span_m.is_finite()
        || input.geometry.direct_horizontal_span_m <= 0.0
        || !input.geometry.direct_distance_m.is_finite()
        || input.geometry.direct_distance_m <= 0.0
        || !input.geometry.route_angle_rad.is_finite()
        || !input.geometry.route_angle_deg.is_finite()
    {
        return Err(SourceTransitionInvalidReason::InvalidGeometry {
            message: "normalized route geometry must have sign +/-1 and positive span".to_owned(),
        });
    }
    validate_initial_state(input.initial_state)?;
    let initial_source_progress = f64::from(input.geometry.horizontal_sign)
        * (input.initial_state.position_m.x - input.source_pad_center_x_m);
    if !initial_source_progress.is_finite() {
        return Err(SourceTransitionInvalidReason::NonFinite {
            sample_index: 0,
            field: "initial_state.source_progress_m".to_owned(),
        });
    }
    if initial_source_progress.abs() > INITIAL_STATE_TOLERANCE_M {
        return Err(SourceTransitionInvalidReason::InitialStateMismatch {
            field: "source_pad_center_x_m".to_owned(),
        });
    }
    input
        .vehicle
        .validate()
        .map_err(|message| SourceTransitionInvalidReason::InvalidGeometry {
            message: format!("vehicle context is invalid: {message}"),
        })?;
    validate_profile(input.profile)?;
    validate_centerline(input.selected_centerline_m)?;
    for (sample_index, sample) in input.samples.iter().enumerate() {
        validate_sample(sample_index, sample, input.cadence.physics_hz)?;
        if sample_index > 0 {
            let previous = &input.samples[sample_index - 1];
            if sample.physics_step == previous.physics_step {
                return Err(SourceTransitionInvalidReason::DuplicatePhysicsStep {
                    step: sample.physics_step,
                });
            }
            if sample.physics_step < previous.physics_step {
                return Err(SourceTransitionInvalidReason::OutOfOrderPhysicsSteps {
                    previous: previous.physics_step,
                    current: sample.physics_step,
                });
            }
            if previous.physics_step.checked_add(1) != Some(sample.physics_step) {
                return Err(SourceTransitionInvalidReason::NonContiguousPhysicsSteps {
                    previous: previous.physics_step,
                    current: sample.physics_step,
                });
            }
        } else if sample.physics_step != 0 {
            return Err(SourceTransitionInvalidReason::MissingSamples);
        }
        if !approx_eq(sample.sim_time_s, sample.observation.sim_time_s)
            || sample.physics_step != sample.observation.physics_step
        {
            return Err(SourceTransitionInvalidReason::ObservationIdentityMismatch {
                sample_index,
            });
        }
        if sample_index == 0 {
            validate_initial_anchor(
                sample,
                input.initial_state,
                input.vehicle,
                input.cadence.physics_hz,
            )?;
        }
    }
    validate_actions(input.audit.actions, input.cadence.physics_hz)?;
    validate_controller_updates(input.audit.controller_updates, input.cadence.physics_hz)?;
    Ok(())
}

fn validate_profile(profile: &SafetyProfile) -> Result<(), SourceTransitionInvalidReason> {
    let values = [
        profile.source_transition_start_m,
        profile.source_transition_end_m,
        profile.target_transition_start_m,
        profile.target_transition_end_m,
        profile.horizontal_span_m,
        profile.full_envelope.horizontal_extent_m,
        profile.full_envelope.vertical_extent_m,
        profile.contact_envelope.horizontal_extent_m,
        profile.contact_envelope.vertical_extent_m,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(SourceTransitionInvalidReason::InvalidGeometry {
            message: "safety profile values must be finite".to_owned(),
        });
    }
    if profile.horizontal_span_m <= 0.0
        || profile.source_transition_start_m < 0.0
        || profile.source_transition_start_m >= profile.source_transition_end_m
        || profile.source_transition_end_m > profile.horizontal_span_m
        || profile.target_transition_start_m < profile.source_transition_end_m
        || profile.target_transition_end_m < profile.target_transition_start_m
        || profile.target_transition_end_m > profile.horizontal_span_m
    {
        return Err(SourceTransitionInvalidReason::InvalidGeometry {
            message: "safety profile boundaries are not ordered".to_owned(),
        });
    }
    profile
        .full_envelope
        .validate()
        .and_then(|_| profile.contact_envelope.validate())
        .map_err(|message| SourceTransitionInvalidReason::InvalidGeometry { message })
}

fn validate_initial_state(
    initial_state: &VehicleInitialState,
) -> Result<(), SourceTransitionInvalidReason> {
    for (field, value) in [
        ("position_m.x", initial_state.position_m.x),
        ("position_m.y", initial_state.position_m.y),
        ("velocity_mps.x", initial_state.velocity_mps.x),
        ("velocity_mps.y", initial_state.velocity_mps.y),
        ("attitude_rad", initial_state.attitude_rad),
        ("angular_rate_radps", initial_state.angular_rate_radps),
    ] {
        if !value.is_finite() {
            return Err(SourceTransitionInvalidReason::NonFinite {
                sample_index: 0,
                field: format!("initial_state.{field}"),
            });
        }
    }
    Ok(())
}

fn validate_centerline(points: &[Vec2]) -> Result<(), SourceTransitionInvalidReason> {
    if points.len() < 2 {
        return Err(SourceTransitionInvalidReason::InvalidGeometry {
            message: "selected centerline needs at least two points".to_owned(),
        });
    }
    for (index, point) in points.iter().enumerate() {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(SourceTransitionInvalidReason::InvalidGeometry {
                message: format!("centerline point {index} must be finite"),
            });
        }
        if index > 0 && point.x <= points[index - 1].x {
            return Err(SourceTransitionInvalidReason::InvalidGeometry {
                message: "selected centerline x must be strictly increasing".to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_sample(
    sample_index: usize,
    sample: &SampleRecord,
    physics_hz: u32,
) -> Result<(), SourceTransitionInvalidReason> {
    let expected_time = sample.physics_step as f64 / f64::from(physics_hz);
    if !sample.sim_time_s.is_finite()
        || (sample.sim_time_s - expected_time).abs() > TIME_TOLERANCE_S
    {
        return Err(SourceTransitionInvalidReason::SampleTimeMismatch { sample_index });
    }
    let observation = &sample.observation;
    for (field, value) in observation_values(observation) {
        if !value.is_finite() {
            return Err(SourceTransitionInvalidReason::NonFinite {
                sample_index,
                field: field.to_owned(),
            });
        }
    }
    validate_command(sample_index, "held_command", sample.held_command)
}

fn observation_values(observation: &Observation) -> [(&'static str, f64); 21] {
    [
        ("sim_time_s", observation.sim_time_s),
        ("position_m.x", observation.position_m.x),
        ("position_m.y", observation.position_m.y),
        ("velocity_mps.x", observation.velocity_mps.x),
        ("velocity_mps.y", observation.velocity_mps.y),
        ("attitude_rad", observation.attitude_rad),
        ("angular_rate_radps", observation.angular_rate_radps),
        ("mass_kg", observation.mass_kg),
        ("fuel_kg", observation.fuel_kg),
        ("gravity_mps2", observation.gravity_mps2),
        ("target_dx_m", observation.target_dx_m),
        ("height_above_target_m", observation.height_above_target_m),
        ("target_surface_y_m", observation.target_surface_y_m),
        (
            "target_pad_half_width_m",
            observation.target_pad_half_width_m,
        ),
        ("touchdown_clearance_m", observation.touchdown_clearance_m),
        ("min_hull_clearance_m", observation.min_hull_clearance_m),
        ("physics_step", observation.physics_step as f64),
        ("position_norm", observation.position_m.length()),
        ("velocity_norm", observation.velocity_mps.length()),
        ("attitude_sin", observation.attitude_rad.sin()),
        ("attitude_cos", observation.attitude_rad.cos()),
    ]
}

fn validate_command(
    sample_index: usize,
    prefix: &str,
    command: Command,
) -> Result<(), SourceTransitionInvalidReason> {
    if !command.throttle_frac.is_finite() {
        return Err(SourceTransitionInvalidReason::InvalidCommand {
            sample_index,
            field: format!("{prefix}.throttle_frac"),
        });
    }
    if !command.target_attitude_rad.is_finite() {
        return Err(SourceTransitionInvalidReason::InvalidCommand {
            sample_index,
            field: format!("{prefix}.target_attitude_rad"),
        });
    }
    if !(0.0..=1.0).contains(&command.throttle_frac) {
        return Err(SourceTransitionInvalidReason::InvalidCommand {
            sample_index,
            field: format!("{prefix}.throttle_frac_range"),
        });
    }
    Ok(())
}

fn validate_initial_anchor(
    sample: &SampleRecord,
    initial_state: &VehicleInitialState,
    vehicle: &VehicleSpec,
    physics_hz: u32,
) -> Result<(), SourceTransitionInvalidReason> {
    let observation = &sample.observation;
    for (field, actual, expected) in [
        (
            "position_m.x",
            observation.position_m.x,
            initial_state.position_m.x,
        ),
        (
            "position_m.y",
            observation.position_m.y,
            initial_state.position_m.y,
        ),
        (
            "velocity_mps.x",
            observation.velocity_mps.x,
            initial_state.velocity_mps.x,
        ),
        (
            "velocity_mps.y",
            observation.velocity_mps.y,
            initial_state.velocity_mps.y,
        ),
        (
            "attitude_rad",
            observation.attitude_rad,
            initial_state.attitude_rad,
        ),
        (
            "angular_rate_radps",
            observation.angular_rate_radps,
            initial_state.angular_rate_radps,
        ),
    ] {
        if (actual - expected).abs() > INITIAL_STATE_TOLERANCE_M {
            return Err(SourceTransitionInvalidReason::InitialStateMismatch {
                field: field.to_owned(),
            });
        }
    }
    let expected_mass_kg = vehicle.dry_mass_kg + vehicle.initial_fuel_kg;
    if !expected_mass_kg.is_finite()
        || (observation.mass_kg - expected_mass_kg).abs() > INITIAL_STATE_TOLERANCE_M
    {
        return Err(SourceTransitionInvalidReason::InitialStateMismatch {
            field: "mass_kg".to_owned(),
        });
    }
    if (observation.fuel_kg - vehicle.initial_fuel_kg).abs() > INITIAL_STATE_TOLERANCE_M {
        return Err(SourceTransitionInvalidReason::InitialStateMismatch {
            field: "fuel_kg".to_owned(),
        });
    }
    if sample.physics_step != 0
        || sample.sim_time_s.abs() > TIME_TOLERANCE_S
        || observation.physics_step != 0
        || observation.sim_time_s.abs() > TIME_TOLERANCE_S
        || observation.mass_kg < 0.0
    {
        return Err(SourceTransitionInvalidReason::InitialStateMismatch {
            field: "anchor_time_or_mass".to_owned(),
        });
    }
    let _ = physics_hz;
    Ok(())
}

fn validate_actions(
    actions: &[ActionLogEntry],
    physics_hz: u32,
) -> Result<(), SourceTransitionInvalidReason> {
    let mut previous_step = None;
    for (index, action) in actions.iter().enumerate() {
        if !action.sim_time_s.is_finite()
            || !action.command.throttle_frac.is_finite()
            || !action.command.target_attitude_rad.is_finite()
        {
            return Err(SourceTransitionInvalidReason::MalformedActionLog {
                index,
                field: "nonfinite".to_owned(),
            });
        }
        if !(0.0..=1.0).contains(&action.command.throttle_frac) {
            return Err(SourceTransitionInvalidReason::MalformedActionLog {
                index,
                field: "throttle_range".to_owned(),
            });
        }
        if let Some(previous_step) = previous_step {
            if action.physics_step == previous_step {
                return Err(SourceTransitionInvalidReason::MalformedActionLog {
                    index,
                    field: "duplicate_step".to_owned(),
                });
            }
            if action.physics_step < previous_step {
                return Err(SourceTransitionInvalidReason::MalformedActionLog {
                    index,
                    field: "out_of_order".to_owned(),
                });
            }
        }
        if (action.sim_time_s - action.physics_step as f64 / f64::from(physics_hz)).abs()
            > TIME_TOLERANCE_S
        {
            return Err(SourceTransitionInvalidReason::MalformedActionLog {
                index,
                field: "time_mismatch".to_owned(),
            });
        }
        previous_step = Some(action.physics_step);
    }
    Ok(())
}

fn validate_controller_updates(
    updates: &[ControllerUpdateRecord],
    physics_hz: u32,
) -> Result<(), SourceTransitionInvalidReason> {
    let mut previous_step = None;
    for (index, update) in updates.iter().enumerate() {
        if !update.sim_time_s.is_finite()
            || !update.frame.command.throttle_frac.is_finite()
            || !update.frame.command.target_attitude_rad.is_finite()
            || !controller_frame_is_finite(&update.frame)
        {
            return Err(SourceTransitionInvalidReason::MalformedControllerUpdate {
                index,
                field: "nonfinite".to_owned(),
            });
        }
        if !(0.0..=1.0).contains(&update.frame.command.throttle_frac) {
            return Err(SourceTransitionInvalidReason::MalformedControllerUpdate {
                index,
                field: "throttle_range".to_owned(),
            });
        }
        if let Some(previous_step) = previous_step
            && update.physics_step <= previous_step
        {
            return Err(SourceTransitionInvalidReason::MalformedControllerUpdate {
                index,
                field: "step_order".to_owned(),
            });
        }
        if (update.sim_time_s - update.physics_step as f64 / f64::from(physics_hz)).abs()
            > TIME_TOLERANCE_S
        {
            return Err(SourceTransitionInvalidReason::MalformedControllerUpdate {
                index,
                field: "time_mismatch".to_owned(),
            });
        }
        previous_step = Some(update.physics_step);
    }
    Ok(())
}

fn controller_frame_is_finite(frame: &ControllerFrame) -> bool {
    let telemetry_is_finite = |value: &TelemetryValue| match value {
        TelemetryValue::Float(value) => value.is_finite(),
        TelemetryValue::Integer(_) | TelemetryValue::Bool(_) | TelemetryValue::Text(_) => true,
    };
    frame.metrics.values().all(telemetry_is_finite)
        && frame.markers.iter().all(|marker| {
            marker.x_m.is_none_or(f64::is_finite)
                && marker.y_m.is_none_or(f64::is_finite)
                && marker.metadata.values().all(telemetry_is_finite)
        })
}

pub(crate) fn raw_state(sample_index: usize, sample: &SampleRecord) -> SourceTransitionRawState {
    let observation = &sample.observation;
    SourceTransitionRawState {
        sample_index,
        physics_step: sample.physics_step,
        sim_time_s: sample.sim_time_s,
        position_m: observation.position_m,
        velocity_mps: observation.velocity_mps,
        attitude_rad: observation.attitude_rad,
        angular_rate_radps: observation.angular_rate_radps,
        mass_kg: observation.mass_kg,
        fuel_kg: observation.fuel_kg,
    }
}

fn find_forward_crossing(
    boundary: SourceTransitionBoundary,
    boundary_m: f64,
    states: &[SourceTransitionRawState],
    progress: &[f64],
) -> Option<SourceTransitionCrossing> {
    progress.windows(2).enumerate().find_map(|(index, pair)| {
        (pair[0] < boundary_m && boundary_m <= pair[1]).then(|| SourceTransitionCrossing {
            boundary,
            direction: SourceTransitionCrossingDirection::Forward,
            before_sample_index: index,
            after_sample_index: index + 1,
            before_physics_step: states[index].physics_step,
            after_physics_step: states[index + 1].physics_step,
            before_sim_time_s: states[index].sim_time_s,
            after_sim_time_s: states[index + 1].sim_time_s,
            before_source_progress_m: pair[0],
            after_source_progress_m: pair[1],
            boundary_m,
            interpolation_fraction: (boundary_m - pair[0]) / (pair[1] - pair[0]),
        })
    })
}

fn make_boundary_bracket(
    input: &SourceTransitionKernelInput<'_>,
    crossing: SourceTransitionCrossing,
) -> SourceTransitionBoundaryBracket {
    let before = raw_state(
        crossing.before_sample_index,
        &input.samples[crossing.before_sample_index],
    );
    let after = raw_state(
        crossing.after_sample_index,
        &input.samples[crossing.after_sample_index],
    );
    let fraction = crossing.interpolation_fraction;
    let interpolated_raw = interpolate_raw_state(&before, &after, fraction);
    SourceTransitionBoundaryBracket {
        boundary: crossing.boundary,
        boundary_m: crossing.boundary_m,
        before,
        after,
        before_source_progress_m: crossing.before_source_progress_m,
        after_source_progress_m: crossing.after_source_progress_m,
        bracket_width_s: after.sim_time_s - before.sim_time_s,
        progress_span_m: crossing.after_source_progress_m - crossing.before_source_progress_m,
        interpolation_fraction: fraction,
        interpolated: SourceTransitionInterpolatedState {
            state: interpolated_raw,
            fraction,
            time_span_s: after.sim_time_s - before.sim_time_s,
            progress_span_m: crossing.after_source_progress_m - crossing.before_source_progress_m,
            before_sample_index: before.sample_index,
            after_sample_index: after.sample_index,
        },
    }
}

fn interpolate_raw_state(
    before: &SourceTransitionRawState,
    after: &SourceTransitionRawState,
    fraction: f64,
) -> SourceTransitionRawState {
    SourceTransitionRawState {
        sample_index: after.sample_index,
        physics_step: after.physics_step,
        sim_time_s: lerp(before.sim_time_s, after.sim_time_s, fraction),
        position_m: lerp_vec2(before.position_m, after.position_m, fraction),
        velocity_mps: lerp_vec2(before.velocity_mps, after.velocity_mps, fraction),
        attitude_rad: before.attitude_rad
            + shortest_angle_delta(before.attitude_rad, after.attitude_rad) * fraction,
        angular_rate_radps: lerp(
            before.angular_rate_radps,
            after.angular_rate_radps,
            fraction,
        ),
        mass_kg: lerp(before.mass_kg, after.mass_kg, fraction),
        fuel_kg: lerp(before.fuel_kg, after.fuel_kg, fraction),
    }
}

fn first_outbound_reference(
    input: &SourceTransitionKernelInput<'_>,
) -> Result<SourceTransitionOutboundReference, SourceTransitionInvalidReason> {
    let segment_index = centerline_segment_index(
        input.profile.source_transition_end_m,
        input.selected_centerline_m,
    )
    .ok_or_else(|| SourceTransitionInvalidReason::InvalidGeometry {
        message: "tracking-entry boundary is outside selected centerline".to_owned(),
    })?;
    let containing_start = input.selected_centerline_m[segment_index];
    let containing_end = input.selected_centerline_m[segment_index + 1];
    let tracking = point_on_segment(
        containing_start,
        containing_end,
        input.profile.source_transition_end_m,
    );
    // `tracking` is the start of the outbound leg.  At an exact shaped
    // centerline vertex, `centerline_segment_index` selects the segment after
    // that vertex; for an interior boundary, retain the exact interpolated
    // boundary point while preserving the containing segment index.
    let start = tracking;
    let end_index = segment_index + 1;
    let end = input.selected_centerline_m[end_index];
    let (waypoint_id, outbound_end) = if let Some(waypoint) = input.waypoints.first() {
        let normalized = world_to_normalized(
            waypoint.position_m,
            input.source_pad_center_x_m,
            input.geometry.horizontal_sign,
        );
        if !normalized.x.is_finite() || !normalized.y.is_finite() {
            return Err(SourceTransitionInvalidReason::InvalidGeometry {
                message: "first outbound waypoint is outside finite normalized frame".to_owned(),
            });
        }
        if normalized.x <= tracking.x + CENTERLINE_TOLERANCE_M
            || normalized.x > input.geometry.direct_horizontal_span_m
        {
            return Err(SourceTransitionInvalidReason::InvalidGeometry {
                message: "first outbound waypoint is not after tracking entry".to_owned(),
            });
        }
        if (normalized - end).length() > CENTERLINE_TOLERANCE_M {
            return Err(SourceTransitionInvalidReason::InvalidGeometry {
                message: "first waypoint is not the next selected centerline point".to_owned(),
            });
        }
        (Some(waypoint.id.clone()), end)
    } else {
        (None, end)
    };
    let tangent_vector = outbound_end - tracking;
    let tangent_length = tangent_vector.length();
    if !tangent_length.is_finite() || tangent_length <= CENTERLINE_TOLERANCE_M {
        return Err(SourceTransitionInvalidReason::InvalidGeometry {
            message: "first outbound reference has zero length".to_owned(),
        });
    }
    let tangent_normalized = tangent_vector * (1.0 / tangent_length);
    let tangent_world = Vec2::new(
        tangent_normalized.x * f64::from(input.geometry.horizontal_sign),
        tangent_normalized.y,
    );
    let segment_start_world = normalized_to_world(
        start,
        input.source_pad_center_x_m,
        input.geometry.horizontal_sign,
    );
    let segment_end_world = normalized_to_world(
        end,
        input.source_pad_center_x_m,
        input.geometry.horizontal_sign,
    );
    let tracking_point_world = normalized_to_world(
        tracking,
        input.source_pad_center_x_m,
        input.geometry.horizontal_sign,
    );
    let outbound_end_world = normalized_to_world(
        outbound_end,
        input.source_pad_center_x_m,
        input.geometry.horizontal_sign,
    );
    if [
        segment_start_world,
        segment_end_world,
        tracking_point_world,
        outbound_end_world,
    ]
    .iter()
    .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return Err(SourceTransitionInvalidReason::InvalidGeometry {
            message: "first outbound reference is outside finite world frame".to_owned(),
        });
    }
    Ok(SourceTransitionOutboundReference {
        geometry_version: SOURCE_TRANSITION_GEOMETRY_VERSION.to_owned(),
        route_leg_index: 0,
        waypoint_id,
        segment_index,
        point_start_index: segment_index,
        point_end_index: end_index,
        segment_start_normalized_m: start,
        segment_end_normalized_m: end,
        tracking_point_normalized_m: tracking,
        outbound_end_normalized_m: outbound_end,
        segment_start_world_m: segment_start_world,
        segment_end_world_m: segment_end_world,
        tracking_point_world_m: tracking_point_world,
        outbound_end_world_m: outbound_end_world,
        unit_tangent_normalized: tangent_normalized,
        unit_tangent_world: tangent_world,
        normalized_frame: "source_pad_center_x_m + horizontal_sign * x".to_owned(),
    })
}

fn centerline_reference(
    boundary: SourceTransitionBoundary,
    coordinate: f64,
    centerline: &[Vec2],
    source_center_x_m: f64,
    horizontal_sign: i8,
) -> Result<SourceTransitionCenterlineReference, SourceTransitionInvalidReason> {
    let segment_index = centerline_segment_index(coordinate, centerline).ok_or_else(|| {
        SourceTransitionInvalidReason::InvalidGeometry {
            message: format!("{boundary:?} boundary is outside selected centerline"),
        }
    })?;
    let start = centerline[segment_index];
    let end = centerline[segment_index + 1];
    let boundary_point = point_on_segment(start, end, coordinate);
    Ok(SourceTransitionCenterlineReference {
        boundary,
        segment_index,
        point_start_index: segment_index,
        point_end_index: segment_index + 1,
        segment_start_normalized_m: start,
        segment_end_normalized_m: end,
        boundary_point_normalized_m: boundary_point,
        segment_start_world_m: normalized_to_world(start, source_center_x_m, horizontal_sign),
        segment_end_world_m: normalized_to_world(end, source_center_x_m, horizontal_sign),
        boundary_point_world_m: normalized_to_world(
            boundary_point,
            source_center_x_m,
            horizontal_sign,
        ),
        normalized_frame: "source_pad_center_x_m + horizontal_sign * x".to_owned(),
    })
}

fn centerline_segment_index(coordinate: f64, centerline: &[Vec2]) -> Option<usize> {
    if !coordinate.is_finite() || centerline.len() < 2 {
        return None;
    }
    for (index, pair) in centerline.windows(2).enumerate() {
        let is_last = index + 2 == centerline.len();
        if coordinate >= pair[0].x - CENTERLINE_TOLERANCE_M
            && (coordinate < pair[1].x - CENTERLINE_TOLERANCE_M || is_last)
        {
            return Some(index);
        }
    }
    None
}

fn point_on_segment(start: Vec2, end: Vec2, coordinate: f64) -> Vec2 {
    let fraction = if (end.x - start.x).abs() <= f64::EPSILON {
        0.0
    } else {
        ((coordinate - start.x) / (end.x - start.x)).clamp(0.0, 1.0)
    };
    lerp_vec2(start, end, fraction)
}

fn source_transition_sample(
    input: &SourceTransitionKernelInput<'_>,
    state: &SourceTransitionRawState,
    source_progress_m: f64,
    outbound: &SourceTransitionOutboundReference,
) -> Result<SourceTransitionSample, SourceTransitionInvalidReason> {
    let normalized_progress = source_progress_m / input.geometry.direct_horizontal_span_m;
    let normalized_position = Vec2::new(
        normalized_progress * input.geometry.direct_horizontal_span_m,
        state.position_m.y,
    );
    let delta = normalized_position - outbound.tracking_point_normalized_m;
    let normal = Vec2::new(
        -outbound.unit_tangent_normalized.y,
        outbound.unit_tangent_normalized.x,
    );
    let normalized_velocity = Vec2::new(
        state.velocity_mps.x * f64::from(input.geometry.horizontal_sign),
        state.velocity_mps.y,
    );
    let path = SourceTransitionPathMetrics {
        normalized_position_m: normalized_position,
        along_track_m: dot(delta, outbound.unit_tangent_normalized),
        cross_track_m: dot(delta, normal),
        velocity_along_track_mps: dot(normalized_velocity, outbound.unit_tangent_normalized),
        velocity_cross_track_mps: dot(normalized_velocity, normal),
    };
    let sample = &input.samples[state.sample_index];
    let point_clearance = |envelope: pd_core::CorridorEnvelope| {
        input
            .terrain
            .exact_point_clearance(state.position_m, envelope)
    };
    let contact = point_clearance(input.profile.contact_envelope).map_err(|error| {
        SourceTransitionInvalidReason::ClearanceQuery {
            sample_index: state.sample_index,
            message: error.to_string(),
        }
    })?;
    let resolved =
        point_clearance(input.profile.envelope_at(source_progress_m)).map_err(|error| {
            SourceTransitionInvalidReason::ClearanceQuery {
                sample_index: state.sample_index,
                message: error.to_string(),
            }
        })?;
    let full = point_clearance(input.profile.full_envelope).map_err(|error| {
        SourceTransitionInvalidReason::ClearanceQuery {
            sample_index: state.sample_index,
            message: error.to_string(),
        }
    })?;
    let result = SourceTransitionSample {
        sample_index: state.sample_index,
        physics_step: state.physics_step,
        sim_time_s: state.sim_time_s,
        source_progress_m,
        normalized_progress,
        state: *state,
        path,
        clearance: SourceTransitionClearance {
            observed_hull_clearance_m: sample.observation.min_hull_clearance_m,
            observed_touchdown_clearance_m: Some(sample.observation.touchdown_clearance_m),
            contact_envelope: point_clearance_record(input.profile.contact_envelope, contact),
            resolved_envelope: point_clearance_record(
                input.profile.envelope_at(source_progress_m),
                resolved,
            ),
            full_envelope: point_clearance_record(input.profile.full_envelope, full),
        },
    };
    if source_transition_sample_values(&result)
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(SourceTransitionInvalidReason::NonFinite {
            sample_index: state.sample_index,
            field: "derived_source_transition_sample".to_owned(),
        });
    }
    Ok(result)
}

fn source_transition_sample_values(sample: &SourceTransitionSample) -> [f64; 20] {
    [
        sample.sim_time_s,
        sample.source_progress_m,
        sample.normalized_progress,
        sample.path.normalized_position_m.x,
        sample.path.normalized_position_m.y,
        sample.path.along_track_m,
        sample.path.cross_track_m,
        sample.path.velocity_along_track_mps,
        sample.path.velocity_cross_track_mps,
        sample.clearance.observed_hull_clearance_m,
        sample
            .clearance
            .observed_touchdown_clearance_m
            .unwrap_or(0.0),
        sample.clearance.contact_envelope.minimum_clearance_m,
        sample.clearance.contact_envelope.residual_m,
        sample.clearance.contact_envelope.required_envelope_y_m,
        sample.clearance.resolved_envelope.minimum_clearance_m,
        sample.clearance.resolved_envelope.residual_m,
        sample.clearance.resolved_envelope.required_envelope_y_m,
        sample.clearance.full_envelope.minimum_clearance_m,
        sample.clearance.full_envelope.residual_m,
        sample.clearance.full_envelope.required_envelope_y_m,
    ]
}

pub(crate) fn point_clearance_record(
    envelope: pd_core::CorridorEnvelope,
    clearance: pd_core::CorridorClearance,
) -> SourceTransitionPointClearance {
    SourceTransitionPointClearance {
        envelope,
        clear: clearance.clear,
        minimum_clearance_m: clearance.minimum_clearance_m,
        residual_m: clearance.worst_residual.residual_m,
        required_envelope_y_m: clearance.worst_residual.required_envelope_y_m,
        terrain_position_m: clearance.worst_residual.terrain_position_m,
        terrain_segment_index: clearance.worst_residual.terrain_segment_index,
    }
}

fn extrema(samples: &[SourceTransitionSample]) -> SourceTransitionExtrema {
    let first = &samples[0];
    let last = &samples[samples.len() - 1];
    let value = |values: Vec<f64>| {
        SourceTransitionExtremaValue::from_values(values.into_iter())
            .expect("extrema input is non-empty")
    };
    SourceTransitionExtrema {
        first_sample_index: first.sample_index,
        last_sample_index: last.sample_index,
        source_progress_m: value(
            samples
                .iter()
                .map(|sample| sample.source_progress_m)
                .collect(),
        ),
        normalized_progress: value(
            samples
                .iter()
                .map(|sample| sample.normalized_progress)
                .collect(),
        ),
        velocity_along_track_mps: value(
            samples
                .iter()
                .map(|sample| sample.path.velocity_along_track_mps)
                .collect(),
        ),
        velocity_cross_track_mps: value(
            samples
                .iter()
                .map(|sample| sample.path.velocity_cross_track_mps)
                .collect(),
        ),
        speed_mps: value(
            samples
                .iter()
                .map(|sample| sample.state.velocity_mps.length())
                .collect(),
        ),
        vertical_speed_mps: value(
            samples
                .iter()
                .map(|sample| sample.state.velocity_mps.y)
                .collect(),
        ),
        attitude_rad: value(
            samples
                .iter()
                .map(|sample| sample.state.attitude_rad)
                .collect(),
        ),
        angular_rate_radps: value(
            samples
                .iter()
                .map(|sample| sample.state.angular_rate_radps)
                .collect(),
        ),
        mass_kg: value(samples.iter().map(|sample| sample.state.mass_kg).collect()),
        fuel_kg: value(samples.iter().map(|sample| sample.state.fuel_kg).collect()),
        along_track_m: value(
            samples
                .iter()
                .map(|sample| sample.path.along_track_m)
                .collect(),
        ),
        cross_track_m: value(
            samples
                .iter()
                .map(|sample| sample.path.cross_track_m)
                .collect(),
        ),
        observed_hull_clearance_m: value(
            samples
                .iter()
                .map(|sample| sample.clearance.observed_hull_clearance_m)
                .collect(),
        ),
        observed_touchdown_clearance_m: value(
            samples
                .iter()
                .filter_map(|sample| sample.clearance.observed_touchdown_clearance_m)
                .collect(),
        ),
        contact_clearance_m: value(
            samples
                .iter()
                .map(|sample| sample.clearance.contact_envelope.minimum_clearance_m)
                .collect(),
        ),
        resolved_clearance_m: value(
            samples
                .iter()
                .map(|sample| sample.clearance.resolved_envelope.minimum_clearance_m)
                .collect(),
        ),
        full_clearance_m: value(
            samples
                .iter()
                .map(|sample| sample.clearance.full_envelope.minimum_clearance_m)
                .collect(),
        ),
    }
}

fn all_boundary_crossings(
    profile: &SafetyProfile,
    states: &[SourceTransitionRawState],
    progress: &[f64],
) -> Vec<SourceTransitionCrossing> {
    let mut crossings = Vec::new();
    for (boundary, boundary_m) in [
        (
            SourceTransitionBoundary::ContactExit,
            profile.source_transition_start_m,
        ),
        (
            SourceTransitionBoundary::TrackingEntry,
            profile.source_transition_end_m,
        ),
    ] {
        for (index, pair) in progress.windows(2).enumerate() {
            let direction = if pair[0] < boundary_m && boundary_m <= pair[1] {
                Some(SourceTransitionCrossingDirection::Forward)
            } else if pair[0] >= boundary_m && boundary_m > pair[1] {
                Some(SourceTransitionCrossingDirection::Reverse)
            } else {
                None
            };
            let Some(direction) = direction else { continue };
            crossings.push(crossing_from_pair(
                boundary, boundary_m, direction, index, states, pair,
            ));
        }
    }
    crossings.sort_by_key(|crossing| {
        (
            crossing.before_sample_index,
            boundary_sort_key(crossing.boundary),
        )
    });
    crossings
}

fn boundary_events(
    profile: &SafetyProfile,
    states: &[SourceTransitionRawState],
    progress: &[f64],
    contact: Option<&SourceTransitionBoundaryBracket>,
    tracking: Option<&SourceTransitionBoundaryBracket>,
) -> (
    Vec<SourceTransitionBoundaryEvent>,
    Vec<SourceTransitionBoundaryEvent>,
) {
    let mut backtracking = Vec::new();
    let mut reentry = Vec::new();
    for (boundary, boundary_m, first) in [
        (
            SourceTransitionBoundary::ContactExit,
            profile.source_transition_start_m,
            contact,
        ),
        (
            SourceTransitionBoundary::TrackingEntry,
            profile.source_transition_end_m,
            tracking,
        ),
    ] {
        let Some(first) = first else { continue };
        for (index, pair) in progress.windows(2).enumerate() {
            if index < first.after.sample_index {
                continue;
            }
            if pair[0] >= boundary_m && boundary_m > pair[1] {
                backtracking.push(SourceTransitionBoundaryEvent {
                    kind: SourceTransitionBoundaryEventKind::Backtracking,
                    crossing: crossing_from_pair(
                        boundary,
                        boundary_m,
                        SourceTransitionCrossingDirection::Reverse,
                        index,
                        states,
                        pair,
                    ),
                });
            } else if pair[0] < boundary_m && boundary_m <= pair[1] {
                reentry.push(SourceTransitionBoundaryEvent {
                    kind: SourceTransitionBoundaryEventKind::Reentry,
                    crossing: crossing_from_pair(
                        boundary,
                        boundary_m,
                        SourceTransitionCrossingDirection::Forward,
                        index,
                        states,
                        pair,
                    ),
                });
            }
        }
    }
    backtracking.sort_by_key(|event| {
        (
            event.crossing.before_sample_index,
            boundary_sort_key(event.crossing.boundary),
        )
    });
    reentry.sort_by_key(|event| {
        (
            event.crossing.before_sample_index,
            boundary_sort_key(event.crossing.boundary),
        )
    });
    (backtracking, reentry)
}

fn boundary_sort_key(boundary: SourceTransitionBoundary) -> u8 {
    match boundary {
        SourceTransitionBoundary::ContactExit => 0,
        SourceTransitionBoundary::TrackingEntry => 1,
    }
}

fn crossing_from_pair(
    boundary: SourceTransitionBoundary,
    boundary_m: f64,
    direction: SourceTransitionCrossingDirection,
    index: usize,
    states: &[SourceTransitionRawState],
    pair: &[f64],
) -> SourceTransitionCrossing {
    SourceTransitionCrossing {
        boundary,
        direction,
        before_sample_index: index,
        after_sample_index: index + 1,
        before_physics_step: states[index].physics_step,
        after_physics_step: states[index + 1].physics_step,
        before_sim_time_s: states[index].sim_time_s,
        after_sim_time_s: states[index + 1].sim_time_s,
        before_source_progress_m: pair[0],
        after_source_progress_m: pair[1],
        boundary_m,
        interpolation_fraction: if (pair[1] - pair[0]).abs() <= f64::EPSILON {
            0.0
        } else {
            (boundary_m - pair[0]) / (pair[1] - pair[0])
        },
    }
}

fn build_audit(
    input: &SourceTransitionKernelInput<'_>,
    states: &[SourceTransitionRawState],
    retained_end: usize,
    contact: Option<&SourceTransitionBoundaryBracket>,
    tracking: Option<&SourceTransitionBoundaryBracket>,
) -> SourceTransitionAudit {
    let sample_ordering = states
        .iter()
        .map(|state| SourceTransitionSampleIdentity {
            sample_index: state.sample_index,
            physics_step: state.physics_step,
            sim_time_s: state.sim_time_s,
        })
        .collect::<Vec<_>>();
    let held_commands = input
        .samples
        .iter()
        .take(retained_end + 1)
        .enumerate()
        .map(|(sample_index, sample)| SourceTransitionHeldCommandAudit {
            sample_index,
            physics_step: sample.physics_step,
            command: sample.held_command,
        })
        .collect::<Vec<_>>();
    let held_command_changes = input
        .samples
        .iter()
        .take(retained_end + 1)
        .enumerate()
        .filter_map(|(index, sample)| {
            let previous = index
                .checked_sub(1)
                .map(|previous| input.samples[previous].held_command);
            (previous.is_none() || previous != Some(sample.held_command)).then_some(
                SourceTransitionHeldCommandChange {
                    sample_index: index,
                    physics_step: sample.physics_step,
                    command: sample.held_command,
                },
            )
        })
        .collect::<Vec<_>>();
    let mut controller_update_changes = Vec::new();
    for (boundary, bracket) in [
        (SourceTransitionBoundary::ContactExit, contact),
        (SourceTransitionBoundary::TrackingEntry, tracking),
    ] {
        let Some(bracket) = bracket else { continue };
        let mut previous_frame: Option<&ControllerFrame> = None;
        for update in input.audit.controller_updates.iter().filter(|update| {
            update.physics_step >= bracket.before.physics_step
                && update.physics_step <= bracket.after.physics_step
        }) {
            let changed_from_previous =
                previous_frame.is_none_or(|previous| previous != &update.frame);
            controller_update_changes.push(SourceTransitionControllerUpdateAudit {
                boundary,
                controller_update_index: update.controller_update_index,
                physics_step: update.physics_step,
                sim_time_s: update.sim_time_s,
                command: update.frame.command,
                changed_from_previous,
                frame_digest: digest_serialized(&StableControllerUpdate {
                    sim_time_s: update.sim_time_s,
                    physics_step: update.physics_step,
                    controller_update_index: update.controller_update_index,
                    frame: &update.frame,
                }),
            });
            previous_frame = Some(&update.frame);
        }
    }
    SourceTransitionAudit {
        sample_ordering,
        action_log_digest: digest_serialized(&input.audit.actions),
        controller_updates_digest: digest_serialized(
            &input
                .audit
                .controller_updates
                .iter()
                .map(|update| StableControllerUpdate {
                    sim_time_s: update.sim_time_s,
                    physics_step: update.physics_step,
                    controller_update_index: update.controller_update_index,
                    frame: &update.frame,
                })
                .collect::<Vec<_>>(),
        ),
        held_commands,
        held_command_changes,
        controller_update_changes,
    }
}

fn fallback_audit(
    samples: &[SampleRecord],
    actions: &[ActionLogEntry],
    controller_updates: &[ControllerUpdateRecord],
) -> SourceTransitionAudit {
    let held_commands = samples
        .iter()
        .enumerate()
        .filter(|(_, sample)| command_is_finite(sample.held_command))
        .map(|(sample_index, sample)| SourceTransitionHeldCommandAudit {
            sample_index,
            physics_step: sample.physics_step,
            command: sample.held_command,
        })
        .collect::<Vec<_>>();
    let held_command_changes = samples
        .iter()
        .enumerate()
        .filter(|(_, sample)| command_is_finite(sample.held_command))
        .filter_map(|(sample_index, sample)| {
            let previous = sample_index
                .checked_sub(1)
                .and_then(|previous| samples.get(previous))
                .map(|sample| sample.held_command);
            (previous.is_none() || previous != Some(sample.held_command)).then_some(
                SourceTransitionHeldCommandChange {
                    sample_index,
                    physics_step: sample.physics_step,
                    command: sample.held_command,
                },
            )
        })
        .collect::<Vec<_>>();
    SourceTransitionAudit {
        sample_ordering: samples
            .iter()
            .enumerate()
            .map(|(sample_index, sample)| SourceTransitionSampleIdentity {
                sample_index,
                physics_step: sample.physics_step,
                sim_time_s: if sample.sim_time_s.is_finite() {
                    sample.sim_time_s
                } else {
                    0.0
                },
            })
            .collect(),
        action_log_digest: digest_serialized(&actions),
        controller_updates_digest: digest_serialized(
            &controller_updates
                .iter()
                .map(|update| StableControllerUpdate {
                    sim_time_s: update.sim_time_s,
                    physics_step: update.physics_step,
                    controller_update_index: update.controller_update_index,
                    frame: &update.frame,
                })
                .collect::<Vec<_>>(),
        ),
        held_commands,
        held_command_changes,
        controller_update_changes: Vec::new(),
    }
}

fn command_is_finite(command: Command) -> bool {
    command.throttle_frac.is_finite() && command.target_attitude_rad.is_finite()
}

pub(crate) fn world_to_normalized(
    point: Vec2,
    source_center_x_m: f64,
    horizontal_sign: i8,
) -> Vec2 {
    Vec2::new(
        f64::from(horizontal_sign) * (point.x - source_center_x_m),
        point.y,
    )
}

pub(crate) fn normalized_to_world(
    point: Vec2,
    source_center_x_m: f64,
    horizontal_sign: i8,
) -> Vec2 {
    Vec2::new(
        source_center_x_m + f64::from(horizontal_sign) * point.x,
        point.y,
    )
}

fn approx_eq(lhs: f64, rhs: f64) -> bool {
    (lhs - rhs).abs() <= TIME_TOLERANCE_S
}

fn lerp(lhs: f64, rhs: f64, fraction: f64) -> f64 {
    lhs + (rhs - lhs) * fraction
}

fn lerp_vec2(lhs: Vec2, rhs: Vec2, fraction: f64) -> Vec2 {
    Vec2::new(lerp(lhs.x, rhs.x, fraction), lerp(lhs.y, rhs.y, fraction))
}

fn shortest_angle_delta(current_rad: f64, target_rad: f64) -> f64 {
    let mut delta = (target_rad - current_rad) % std::f64::consts::TAU;
    if delta > std::f64::consts::PI {
        delta -= std::f64::consts::TAU;
    } else if delta < -std::f64::consts::PI {
        delta += std::f64::consts::TAU;
    }
    delta
}

fn dot(lhs: Vec2, rhs: Vec2) -> f64 {
    lhs.x * rhs.x + lhs.y * rhs.y
}

pub(crate) fn digest_serialized<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:012x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::CorridorEnvelope;
    use std::f64::consts::PI;

    fn sample(step: u64, x_m: f64, attitude_rad: f64) -> SampleRecord {
        let sim_time_s = step as f64 / 10.0;
        SampleRecord {
            sim_time_s,
            physics_step: step,
            observation: Observation {
                sim_time_s,
                physics_step: step,
                position_m: Vec2::new(x_m, 20.0),
                velocity_mps: Vec2::new(4.0, 1.0),
                attitude_rad,
                angular_rate_radps: 0.0,
                mass_kg: 10.0,
                fuel_kg: 5.0,
                gravity_mps2: 1.62,
                target_dx_m: 20.0 - x_m,
                height_above_target_m: 20.0,
                target_surface_y_m: 0.0,
                target_pad_half_width_m: 2.0,
                touchdown_clearance_m: 19.0,
                min_hull_clearance_m: 18.0,
            },
            held_command: Command {
                throttle_frac: 0.5,
                target_attitude_rad: 0.0,
            },
        }
    }

    fn test_vehicle() -> &'static VehicleSpec {
        use std::sync::OnceLock;

        static VEHICLE: OnceLock<VehicleSpec> = OnceLock::new();
        VEHICLE.get_or_init(|| VehicleSpec {
            geometry: pd_core::VehicleGeometry {
                hull_width_m: 2.0,
                hull_height_m: 3.0,
                touchdown_half_span_m: 1.0,
                touchdown_base_offset_m: 1.5,
            },
            dry_mass_kg: 5.0,
            initial_fuel_kg: 5.0,
            max_fuel_kg: 5.0,
            max_thrust_n: 100.0,
            max_fuel_burn_kgps: 1.0,
            min_throttle_frac: 0.0,
            max_rotation_rate_radps: 1.0,
            safe_touchdown_normal_speed_mps: 2.0,
            safe_touchdown_tangential_speed_mps: 2.0,
            safe_touchdown_attitude_error_rad: 0.2,
            safe_touchdown_angular_rate_radps: 0.2,
        })
    }

    fn test_input<'a>(
        initial_state: &'a VehicleInitialState,
        geometry: &'a NormalizedRouteGeometry,
        profile: &'a SafetyProfile,
        terrain: &'a TerrainDefinition,
        centerline: &'a [Vec2],
        samples: &'a [SampleRecord],
    ) -> SourceTransitionKernelInput<'a> {
        SourceTransitionKernelInput {
            terrain,
            source_pad_center_x_m: 0.0,
            vehicle: test_vehicle(),
            initial_state,
            geometry,
            profile,
            selected_centerline_m: centerline,
            waypoints: &[],
            samples,
            cadence: SourceTransitionCadence::physics_rate(10),
            audit: SourceTransitionAuditInput::default(),
        }
    }

    fn test_geometry() -> NormalizedRouteGeometry {
        NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        }
    }

    fn test_profile() -> SafetyProfile {
        SafetyProfile {
            source_transition_start_m: 3.0,
            source_transition_end_m: 6.0,
            target_transition_start_m: 8.0,
            target_transition_end_m: 10.0,
            horizontal_span_m: 10.0,
            full_envelope: CorridorEnvelope::new(1.0, 1.0),
            contact_envelope: CorridorEnvelope::new(0.5, 0.5),
        }
    }

    fn test_terrain() -> TerrainDefinition {
        TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(-20.0, 0.0), Vec2::new(20.0, 0.0)],
        }
    }

    fn test_manifest() -> RunManifest {
        RunManifest {
            schema_version: pd_core::model::RUN_SCHEMA_VERSION,
            scenario_id: "source_transition_test".to_owned(),
            scenario_name: "Source transition test".to_owned(),
            scenario_seed: 1,
            scenario_tags: vec!["test".to_owned()],
            controller_id: "test-controller".to_owned(),
            physics_hz: 10,
            controller_hz: 10,
            sim_time_s: 0.1,
            physics_steps: 1,
            controller_updates: 1,
            physical_outcome: pd_core::PhysicalOutcome::TimedOut,
            mission_outcome: pd_core::MissionOutcome::FailedTimeout,
            end_reason: EndReason::MaxTimeReached,
            summary: pd_core::RunSummary::default(),
        }
    }

    fn test_artifacts() -> RunArtifacts {
        RunArtifacts {
            manifest: test_manifest(),
            actions: vec![pd_core::ActionLogEntry {
                sim_time_s: 0.0,
                physics_step: 0,
                controller_update_index: 0,
                command: Command::idle(),
            }],
            events: vec![pd_core::EventRecord {
                sim_time_s: 0.1,
                physics_step: 1,
                kind: pd_core::EventKind::MaxTimeReached,
                message: "test".to_owned(),
            }],
            samples: vec![sample(0, 0.0, 0.0)],
        }
    }

    fn complete_input<'a>(
        initial: &'a VehicleInitialState,
        geometry: &'a NormalizedRouteGeometry,
        profile: &'a SafetyProfile,
        terrain: &'a TerrainDefinition,
        centerline: &'a [Vec2],
        samples: &'a [SampleRecord],
    ) -> SourceTransitionKernelInput<'a> {
        test_input(initial, geometry, profile, terrain, centerline, samples)
    }

    #[test]
    fn kernel_records_first_exact_brackets_and_shortest_angle_interpolation() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(4.0, 1.0),
            attitude_rad: 3.13,
            angular_rate_radps: 0.0,
        };
        let geometry = test_geometry();
        let profile = test_profile();
        let terrain = test_terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let samples = vec![
            sample(0, 0.0, 3.13),
            sample(1, 2.0, 3.13),
            sample(2, 4.0, -3.13),
            sample(3, 7.0, -3.13),
            sample(4, 9.0, -3.13),
        ];
        let input = complete_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &samples,
        );
        let output = extract_source_transition_kernel(&input).unwrap();
        let contact = output.contact_exit.as_ref().unwrap();
        assert_eq!(contact.before.sample_index, 1);
        assert_eq!(contact.after.sample_index, 2);
        assert!((contact.interpolation_fraction - 0.5).abs() < 1.0e-12);
        assert!((contact.interpolated.state.attitude_rad - PI).abs() < 0.02);
        assert_eq!(
            output.tracking_entry.as_ref().unwrap().before.sample_index,
            2
        );
        assert_eq!(
            output.tracking_entry.as_ref().unwrap().after.sample_index,
            3
        );
        assert_eq!(output.samples.len(), 4);
        assert_eq!(
            output
                .pad_departure_extrema
                .as_ref()
                .unwrap()
                .last_sample_index,
            2
        );
        assert_eq!(
            output
                .acquisition_extrema
                .as_ref()
                .unwrap()
                .first_sample_index,
            2
        );
    }

    #[test]
    fn kernel_rejects_vehicle_mass_and_fuel_anchor_mismatches() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(4.0, 1.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = test_geometry();
        let profile = test_profile();
        let terrain = test_terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let mut samples = vec![sample(0, 0.0, 0.0), sample(1, 4.0, 0.0)];

        samples[0].observation.mass_kg = 10.0 + 2.0e-9;
        let input = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &samples,
        );
        assert!(matches!(
            extract_source_transition_kernel(&input),
            Err(SourceTransitionInvalidReason::InitialStateMismatch { field })
                if field == "mass_kg"
        ));

        samples[0].observation.mass_kg = 10.0;
        samples[0].observation.fuel_kg = 5.0 + 2.0e-9;
        let input = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &samples,
        );
        assert!(matches!(
            extract_source_transition_kernel(&input),
            Err(SourceTransitionInvalidReason::InitialStateMismatch { field })
                if field == "fuel_kg"
        ));
    }

    #[test]
    fn kernel_accepts_exact_boundary_endpoints_without_synthesizing_an_anchor_crossing() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(4.0, 1.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = test_geometry();
        let profile = test_profile();
        let terrain = test_terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let samples = vec![
            sample(0, 0.0, 0.0),
            sample(1, 3.0, 0.0),
            sample(2, 6.0, 0.0),
        ];
        let output = extract_source_transition_kernel(&test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &samples,
        ))
        .unwrap();
        let contact = output.contact_exit.as_ref().unwrap();
        let tracking = output.tracking_entry.as_ref().unwrap();
        assert_eq!(contact.before.sample_index, 0);
        assert_eq!(contact.after.sample_index, 1);
        assert_eq!(contact.interpolation_fraction, 1.0);
        assert_eq!(tracking.before.sample_index, 1);
        assert_eq!(tracking.after.sample_index, 2);
        assert_eq!(tracking.interpolation_fraction, 1.0);
        assert_eq!(
            contact.interpolated.state.position_m,
            samples[1].observation.position_m
        );
        assert_eq!(
            tracking.interpolated.state.position_m,
            samples[2].observation.position_m
        );
    }

    #[test]
    fn kernel_handles_mirrored_progress_and_exact_boundary_endpoint() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(10.0, 20.0),
            velocity_mps: Vec2::new(-4.0, 1.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let mut geometry = test_geometry();
        geometry.horizontal_sign = -1;
        let profile = test_profile();
        let terrain = test_terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let mut samples = vec![
            sample(0, 10.0, 0.0),
            sample(1, 7.0, 0.0),
            sample(2, 4.0, 0.0),
        ];
        samples[0].observation.velocity_mps.x = -4.0;
        let input = SourceTransitionKernelInput {
            terrain: &terrain,
            source_pad_center_x_m: 10.0,
            vehicle: test_vehicle(),
            initial_state: &initial,
            geometry: &geometry,
            profile: &profile,
            selected_centerline_m: &centerline,
            waypoints: &[],
            samples: &samples,
            cadence: SourceTransitionCadence::physics_rate(10),
            audit: SourceTransitionAuditInput::default(),
        };
        let output = extract_source_transition_kernel(&input).unwrap();
        assert_eq!(
            output
                .contact_exit
                .as_ref()
                .unwrap()
                .after_source_progress_m,
            3.0
        );
        assert_eq!(
            output
                .tracking_entry
                .as_ref()
                .unwrap()
                .after_source_progress_m,
            6.0
        );
        assert_eq!(output.samples.last().unwrap().source_progress_m, 6.0);
    }

    #[test]
    fn kernel_records_first_outbound_waypoint_reference_in_both_frames() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(4.0, 1.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = test_geometry();
        let profile = test_profile();
        let terrain = test_terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(9.0, 21.0),
            Vec2::new(10.0, 20.0),
        ];
        let samples = vec![
            sample(0, 0.0, 0.0),
            sample(1, 4.0, 0.0),
            sample(2, 7.0, 0.0),
        ];
        let waypoints = [TransferWaypointSpec {
            id: "wp-0".to_owned(),
            position_m: Vec2::new(9.0, 21.0),
            handoff_tangent_unit: None,
            capture_radius_m: 1.0,
            max_cross_track_m: 1.0,
            max_outbound_heading_error_rad: 0.35,
            min_outbound_progress_mps: 1.0,
            max_outbound_cross_speed_mps: None,
            min_speed_mps: 1.0,
            max_speed_mps: 10.0,
            min_vertical_speed_mps: None,
            max_vertical_speed_mps: None,
        }];
        let input = SourceTransitionKernelInput {
            waypoints: &waypoints,
            ..test_input(
                &initial,
                &geometry,
                &profile,
                &terrain,
                &centerline,
                &samples,
            )
        };
        let output = extract_source_transition_kernel(&input).unwrap();
        let outbound = output.first_outbound_reference;
        assert_eq!(outbound.route_leg_index, 0);
        assert_eq!(outbound.waypoint_id.as_deref(), Some("wp-0"));
        assert_eq!(outbound.segment_index, 2);
        assert_eq!(outbound.point_start_index, 2);
        assert_eq!(outbound.point_end_index, 3);
        assert_eq!(outbound.segment_start_normalized_m, Vec2::new(6.0, 20.0));
        assert_eq!(outbound.segment_end_normalized_m, Vec2::new(9.0, 21.0));
        assert_eq!(outbound.tracking_point_normalized_m, Vec2::new(6.0, 20.0));
        assert_eq!(outbound.outbound_end_normalized_m, Vec2::new(9.0, 21.0));
        assert_eq!(outbound.tracking_point_world_m, Vec2::new(6.0, 20.0));
        assert_eq!(outbound.outbound_end_world_m, Vec2::new(9.0, 21.0));
        let segment_vector = Vec2::new(9.0, 21.0) - Vec2::new(6.0, 20.0);
        let segment_tangent = segment_vector * (1.0 / segment_vector.length());
        assert_eq!(outbound.unit_tangent_normalized, segment_tangent);
        assert!((outbound.unit_tangent_normalized.length() - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn assembler_assigns_complete_and_both_right_censor_statuses() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(4.0, 1.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = test_geometry();
        let profile = test_profile();
        let terrain = test_terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let complete_samples = vec![
            sample(0, 0.0, 0.0),
            sample(1, 4.0, 0.0),
            sample(2, 7.0, 0.0),
        ];
        let complete = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &complete_samples,
        );
        let complete_evidence = assemble_source_transition_evidence(
            &complete,
            SourceTransitionProvenance::default(),
            None,
        );
        assert_eq!(
            complete_evidence.status,
            SourceTransitionEvidenceStatus::Complete
        );
        assert!(complete_evidence.invalid_reason.is_none());
        validate_persisted_source_transition_evidence(&complete_evidence).unwrap();
        let mut changed_bounds = complete_evidence.clone();
        changed_bounds.source_transition_end_m += 1.0;
        assert_ne!(
            source_transition_physical_digest(&complete_evidence).unwrap(),
            source_transition_physical_digest(&changed_bounds).unwrap()
        );
        let mut malformed = complete_evidence.clone();
        malformed.samples[1].sample_index = 99;
        malformed.samples[1].state.sample_index = 99;
        let malformed = seal_source_transition_evidence(malformed);
        assert!(validate_persisted_source_transition_evidence(&malformed).is_err());

        let contact_only_samples = vec![
            sample(0, 0.0, 0.0),
            sample(1, 4.0, 0.0),
            sample(2, 5.0, 0.0),
        ];
        let contact_only = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &contact_only_samples,
        );
        let censored_tracking = assemble_source_transition_evidence(
            &contact_only,
            SourceTransitionProvenance::default(),
            Some(SourceTransitionTerminal {
                physics_step: 2,
                reason: "max_time_reached".to_owned(),
            }),
        );
        assert_eq!(
            censored_tracking.status,
            SourceTransitionEvidenceStatus::CensoredBeforeTrackingEntry
        );
        assert_eq!(censored_tracking.samples.last().unwrap().physics_step, 2);
        let acquisition_extrema = censored_tracking
            .acquisition_extrema
            .as_ref()
            .expect("contact acquisition extrema should include terminal prefix");
        assert_eq!(acquisition_extrema.first_sample_index, 1);
        assert_eq!(acquisition_extrema.last_sample_index, 2);
        assert_eq!(acquisition_extrema.source_progress_m.max, 5.0);

        let no_contact_samples = vec![sample(0, 0.0, 0.0), sample(1, 2.0, 0.0)];
        let no_contact = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &no_contact_samples,
        );
        let censored_contact = assemble_source_transition_evidence(
            &no_contact,
            SourceTransitionProvenance::default(),
            Some(SourceTransitionTerminal {
                physics_step: 1,
                reason: "crash".to_owned(),
            }),
        );
        assert_eq!(
            censored_contact.status,
            SourceTransitionEvidenceStatus::CensoredBeforeContactExit
        );
    }

    #[test]
    fn kernel_preserves_backtracking_and_reentry_without_reselecting_brackets() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(4.0, 1.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = test_geometry();
        let profile = test_profile();
        let terrain = test_terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let samples = [
            sample(0, 0.0, 0.0),
            sample(1, 4.0, 0.0),
            sample(2, 2.0, 0.0),
            sample(3, 5.0, 0.0),
            sample(4, 7.0, 0.0),
            sample(5, 5.0, 0.0),
            sample(6, 8.0, 0.0),
        ];
        let input = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &samples,
        );
        let output = extract_source_transition_kernel(&input).unwrap();
        assert_eq!(output.contact_exit.as_ref().unwrap().before.sample_index, 0);
        assert_eq!(
            output.tracking_entry.as_ref().unwrap().before.sample_index,
            3
        );
        assert_eq!(
            output
                .backtracking_events
                .iter()
                .filter(|event| event.crossing.boundary == SourceTransitionBoundary::ContactExit)
                .count(),
            1
        );
        assert_eq!(
            output
                .reentry_events
                .iter()
                .filter(|event| event.crossing.boundary == SourceTransitionBoundary::ContactExit)
                .count(),
            1
        );
        assert_eq!(
            output
                .backtracking_events
                .iter()
                .filter(|event| event.crossing.boundary == SourceTransitionBoundary::TrackingEntry)
                .count(),
            1
        );
    }

    #[test]
    fn malformed_cadence_order_and_nonfinite_inputs_are_invalid() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(4.0, 1.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = test_geometry();
        let profile = test_profile();
        let terrain = test_terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let mut samples = vec![sample(0, 0.0, 0.0), sample(1, 4.0, 0.0)];
        let mut cadence = SourceTransitionCadence::physics_rate(10);
        cadence.sample_hz = Some(5);
        let input = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &samples,
        );
        let input = SourceTransitionKernelInput { cadence, ..input };
        assert!(matches!(
            extract_source_transition_kernel(&input),
            Err(SourceTransitionInvalidReason::WrongCadence { .. })
        ));

        cadence = SourceTransitionCadence::physics_rate(10);
        samples[1].physics_step = 3;
        samples[1].observation.physics_step = 3;
        samples[1].sim_time_s = 0.3;
        samples[1].observation.sim_time_s = 0.3;
        let input = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &samples,
        );
        let input = SourceTransitionKernelInput { cadence, ..input };
        assert!(matches!(
            extract_source_transition_kernel(&input),
            Err(SourceTransitionInvalidReason::NonContiguousPhysicsSteps { .. })
        ));

        samples[1] = sample(1, f64::NAN, 0.0);
        let input = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &samples,
        );
        assert!(matches!(
            extract_source_transition_kernel(&input),
            Err(SourceTransitionInvalidReason::NonFinite { .. })
        ));

        let mut duplicate_samples = vec![
            sample(0, 0.0, 0.0),
            sample(1, 4.0, 0.0),
            sample(2, 7.0, 0.0),
        ];
        duplicate_samples[2].physics_step = 1;
        duplicate_samples[2].observation.physics_step = 1;
        duplicate_samples[2].sim_time_s = 0.1;
        duplicate_samples[2].observation.sim_time_s = 0.1;
        let input = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &duplicate_samples,
        );
        assert!(matches!(
            extract_source_transition_kernel(&input),
            Err(SourceTransitionInvalidReason::DuplicatePhysicsStep { step: 1 })
        ));

        let mut out_of_order_samples = duplicate_samples;
        out_of_order_samples[2].physics_step = 0;
        out_of_order_samples[2].observation.physics_step = 0;
        out_of_order_samples[2].sim_time_s = 0.0;
        out_of_order_samples[2].observation.sim_time_s = 0.0;
        let input = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &out_of_order_samples,
        );
        assert!(matches!(
            extract_source_transition_kernel(&input),
            Err(SourceTransitionInvalidReason::OutOfOrderPhysicsSteps {
                previous: 1,
                current: 0
            })
        ));

        let mut mismatched_observation = vec![sample(0, 0.0, 0.0), sample(1, 4.0, 0.0)];
        mismatched_observation[1].observation.physics_step = 9;
        let input = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &mismatched_observation,
        );
        assert!(matches!(
            extract_source_transition_kernel(&input),
            Err(SourceTransitionInvalidReason::ObservationIdentityMismatch { sample_index: 1 })
        ));

        let no_terminal = assemble_source_transition_evidence(
            &test_input(
                &initial,
                &geometry,
                &profile,
                &terrain,
                &centerline,
                &[sample(0, 0.0, 0.0), sample(1, 2.0, 0.0)],
            ),
            SourceTransitionProvenance::default(),
            None,
        );
        assert_eq!(no_terminal.status, SourceTransitionEvidenceStatus::Invalid);
        assert_eq!(
            no_terminal.invalid_reason,
            Some(SourceTransitionInvalidReason::MissingTerminalCensor {
                boundary: SourceTransitionBoundary::ContactExit
            })
        );

        let malformed_terminal = assemble_source_transition_evidence(
            &test_input(
                &initial,
                &geometry,
                &profile,
                &terrain,
                &centerline,
                &[sample(0, 0.0, 0.0), sample(1, 2.0, 0.0)],
            ),
            SourceTransitionProvenance::default(),
            Some(SourceTransitionTerminal {
                physics_step: 1,
                reason: "unknown_reason".to_owned(),
            }),
        );
        assert_eq!(
            malformed_terminal.invalid_reason,
            Some(SourceTransitionInvalidReason::InvalidTerminalReason)
        );

        let mismatched_terminal = assemble_source_transition_evidence(
            &test_input(
                &initial,
                &geometry,
                &profile,
                &terrain,
                &centerline,
                &[sample(0, 0.0, 0.0), sample(1, 2.0, 0.0)],
            ),
            SourceTransitionProvenance::default(),
            Some(SourceTransitionTerminal {
                physics_step: 2,
                reason: "crash".to_owned(),
            }),
        );
        assert!(matches!(
            mismatched_terminal.invalid_reason,
            Some(SourceTransitionInvalidReason::TerminalStepMismatch { .. })
        ));
    }

    #[test]
    fn evidence_digest_is_deterministic_and_compute_timing_is_not_hashed() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(4.0, 1.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = test_geometry();
        let profile = test_profile();
        let terrain = test_terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let samples = vec![
            sample(0, 0.0, 0.0),
            sample(1, 4.0, 0.0),
            sample(2, 7.0, 0.0),
        ];
        let input = test_input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &samples,
        );
        let mut update = ControllerUpdateRecord {
            sim_time_s: 0.0,
            physics_step: 0,
            controller_update_index: 0,
            compute_time_us: Some(1),
            frame: ControllerFrame::command_only(Command::idle()),
        };
        let first = {
            let input = SourceTransitionKernelInput {
                audit: SourceTransitionAuditInput {
                    actions: &[],
                    controller_updates: std::slice::from_ref(&update),
                },
                ..input
            };
            assemble_source_transition_evidence(&input, SourceTransitionProvenance::default(), None)
        };
        update.compute_time_us = Some(999_999);
        let second = {
            let input = SourceTransitionKernelInput {
                audit: SourceTransitionAuditInput {
                    actions: &[],
                    controller_updates: std::slice::from_ref(&update),
                },
                ..input
            };
            assemble_source_transition_evidence(&input, SourceTransitionProvenance::default(), None)
        };
        assert_eq!(first.evidence_digest, second.evidence_digest);
        assert_eq!(first.physical_digest, second.physical_digest);
        assert_eq!(
            first.provenance.samples_digest,
            second.provenance.samples_digest
        );
        let serialized = serde_json::to_string(&first).unwrap();
        assert!(!serialized.contains("controller_id"));
        assert!(!serialized.contains("scenario_seed"));
        assert!(!serialized.contains("mission_outcome"));
        assert!(!serialized.contains("physical_outcome"));
        assert!(!serialized.contains("phase"));
    }

    #[test]
    fn raw_bundle_digest_covers_manifest_events_and_stable_controller_content() {
        let artifacts = test_artifacts();
        let update = ControllerUpdateRecord {
            sim_time_s: 0.0,
            physics_step: 0,
            controller_update_index: 0,
            compute_time_us: Some(1),
            frame: ControllerFrame::command_only(Command::idle()),
        };
        let first = source_transition_raw_bundle_digest(&artifacts, std::slice::from_ref(&update));

        let mut compute_changed = update.clone();
        compute_changed.compute_time_us = Some(999_999);
        assert_eq!(
            first,
            source_transition_raw_bundle_digest(&artifacts, std::slice::from_ref(&compute_changed))
        );

        let mut event_changed = artifacts.clone();
        event_changed.events[0].message = "changed".to_owned();
        assert_ne!(
            first,
            source_transition_raw_bundle_digest(&event_changed, std::slice::from_ref(&update))
        );
        let mut manifest_changed = artifacts.clone();
        manifest_changed.manifest.scenario_seed = 99;
        assert_ne!(
            first,
            source_transition_raw_bundle_digest(&manifest_changed, std::slice::from_ref(&update))
        );
        let mut frame_changed = update;
        frame_changed.frame.command.throttle_frac = 0.25;
        assert_ne!(
            first,
            source_transition_raw_bundle_digest(&artifacts, std::slice::from_ref(&frame_changed))
        );
    }

    #[test]
    fn geometry_identity_changes_with_resolved_pad_geometry() {
        let normalized_geometry = test_geometry();
        let source_pad = pd_core::LandingPadSpec {
            id: "source".to_owned(),
            center_x_m: 0.0,
            surface_y_m: 0.0,
            width_m: 4.0,
        };
        let target_pad = pd_core::LandingPadSpec {
            id: "target".to_owned(),
            center_x_m: 10.0,
            surface_y_m: 1.0,
            width_m: 4.0,
        };
        let first = source_transition_canonical_digest(&SourceTransitionGeometryIdentity {
            source_pad: &source_pad,
            target_pad: &target_pad,
            normalized_geometry: &normalized_geometry,
        });
        let mut moved_source = source_pad.clone();
        moved_source.surface_y_m = 0.5;
        assert_ne!(
            first,
            source_transition_canonical_digest(&SourceTransitionGeometryIdentity {
                source_pad: &moved_source,
                target_pad: &target_pad,
                normalized_geometry: &normalized_geometry,
            })
        );
        let mut resized_target = target_pad;
        resized_target.width_m = 5.0;
        assert_ne!(
            first,
            source_transition_canonical_digest(&SourceTransitionGeometryIdentity {
                source_pad: &source_pad,
                target_pad: &resized_target,
                normalized_geometry: &normalized_geometry,
            })
        );
    }

    #[test]
    fn invalid_evidence_keeps_provenance_and_is_json_serializable() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(4.0, 1.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = test_geometry();
        let profile = test_profile();
        let terrain = test_terrain();
        let centerline = [Vec2::new(0.0, 20.0), Vec2::new(10.0, 20.0)];
        let mut samples = vec![sample(0, 0.0, 0.0), sample(1, 2.0, 0.0)];
        samples[1].sim_time_s = f64::NAN;
        samples[1].observation.sim_time_s = f64::NAN;
        samples[1].held_command.target_attitude_rad = f64::NAN;
        let input = SourceTransitionKernelInput {
            terrain: &terrain,
            source_pad_center_x_m: 0.0,
            vehicle: test_vehicle(),
            initial_state: &initial,
            geometry: &geometry,
            profile: &profile,
            selected_centerline_m: &centerline,
            waypoints: &[],
            samples: &samples,
            cadence: SourceTransitionCadence {
                sample_hz: None,
                physics_hz: 10,
            },
            audit: SourceTransitionAuditInput::default(),
        };
        let evidence = assemble_source_transition_evidence(
            &input,
            SourceTransitionProvenance {
                request_digest: "request".to_owned(),
                ..SourceTransitionProvenance::default()
            },
            None,
        );
        assert_eq!(evidence.status, SourceTransitionEvidenceStatus::Invalid);
        assert_eq!(evidence.provenance.request_digest, "request");
        assert!(serde_json::to_string(&evidence).is_ok());
        assert_eq!(
            source_transition_evidence_digest(&evidence),
            evidence.evidence_digest
        );
        let decoded: SourceTransitionEvidence =
            serde_json::from_str(&serde_json::to_string(&evidence).unwrap()).unwrap();
        assert_eq!(decoded, evidence);
        assert_eq!(
            source_transition_evidence_digest(&decoded),
            decoded.evidence_digest
        );
    }

    #[test]
    fn cadence_parity_normalizes_compute_time_and_keeps_terminal_sample() {
        let mut ordinary_scenario: ScenarioSpec = serde_json::from_str(include_str!(
            "../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .unwrap();
        let mut physics_scenario = ordinary_scenario.clone();
        ordinary_scenario.sim.physics_hz = 10;
        physics_scenario.sim.physics_hz = 10;
        ordinary_scenario.sim.sample_hz = Some(5);
        physics_scenario.sim.sample_hz = Some(10);
        let full_samples = vec![
            sample(0, 0.0, 0.0),
            sample(1, 2.0, 0.0),
            sample(2, 4.0, 0.0),
            sample(3, 7.0, 0.0),
        ];
        let ordinary_samples = vec![
            full_samples[0].clone(),
            full_samples[2].clone(),
            full_samples[3].clone(),
        ];
        let manifest = |samples: &[SampleRecord]| RunManifest {
            schema_version: 4,
            scenario_id: "parity".to_owned(),
            scenario_name: "parity".to_owned(),
            scenario_seed: 1,
            scenario_tags: Vec::new(),
            controller_id: "test".to_owned(),
            physics_hz: 10,
            controller_hz: 10,
            sim_time_s: samples.last().unwrap().sim_time_s,
            physics_steps: samples.last().unwrap().physics_step,
            controller_updates: 1,
            physical_outcome: pd_core::PhysicalOutcome::Flying,
            mission_outcome: pd_core::MissionOutcome::InProgress,
            end_reason: EndReason::MaxTimeReached,
            summary: pd_core::RunSummary::default(),
        };
        let updates = vec![ControllerUpdateRecord {
            sim_time_s: 0.0,
            physics_step: 0,
            controller_update_index: 0,
            compute_time_us: Some(1),
            frame: ControllerFrame::command_only(Command::idle()),
        }];
        let ordinary = ControlledRunArtifacts {
            run: RunArtifacts {
                manifest: manifest(&ordinary_samples),
                actions: Vec::new(),
                events: Vec::new(),
                samples: ordinary_samples,
            },
            controller_updates: updates.clone(),
            performance: pd_control::RunPerformanceStats {
                wall_time_us: 1,
                thread_cpu_time_us: None,
            },
        };
        let mut physics_updates = updates;
        physics_updates[0].compute_time_us = Some(99_999);
        let physics = ControlledRunArtifacts {
            run: RunArtifacts {
                manifest: manifest(&full_samples),
                actions: Vec::new(),
                events: Vec::new(),
                samples: full_samples,
            },
            controller_updates: physics_updates,
            performance: pd_control::RunPerformanceStats {
                wall_time_us: 2,
                thread_cpu_time_us: None,
            },
        };
        let parity = compare_source_transition_cadence_parity(
            &ordinary_scenario,
            &ordinary,
            &physics_scenario,
            &physics,
        );
        assert!(parity.passed, "{parity:?}");
        assert_eq!(parity.ordinary_shared_sample_steps, vec![0, 2, 3]);
        assert!(parity.controller_updates_equal);
    }

    #[test]
    fn committed_development_manifest_is_input_only_and_counts_both_corpora() {
        let manifest: SourceTransitionDevelopmentManifest = serde_json::from_str(include_str!(
            "../../fixtures/manifests/source_transition_d0a_development.json"
        ))
        .unwrap();
        manifest.validate().unwrap();
        assert_eq!(manifest.baseline_expected_case_count, 36);
        assert_eq!(manifest.diagnostic_expected_case_count, 24);
        assert_eq!(
            manifest.overlay.sample_retention,
            SourceTransitionSampleRetention::PhysicsRate
        );
        let keys = manifest
            .baseline_cases
            .iter()
            .flat_map(SourceTransitionDevelopmentCase::resolved_case_keys)
            .collect::<Vec<_>>();
        assert_eq!(keys.len(), 36);
        assert!(keys.iter().any(|key| key.contains("seed_00")));
        assert!(
            !serde_json::to_string(&manifest)
                .unwrap()
                .contains("outcome")
        );
    }

    #[test]
    fn physics_rate_overlay_changes_only_sample_retention() {
        let scenario: ScenarioSpec = serde_json::from_str(include_str!(
            "../../fixtures/scenarios/flat_terminal_descent.json"
        ))
        .unwrap();
        let overlaid = with_physics_rate_evidence_overlay(&scenario);
        assert_eq!(overlaid.sim.sample_hz, Some(overlaid.sim.physics_hz));
        let mut normalized_original = scenario;
        let mut normalized_overlay = overlaid;
        normalized_original.sim.sample_hz = None;
        normalized_overlay.sim.sample_hz = None;
        assert_eq!(normalized_original, normalized_overlay);
    }
}
