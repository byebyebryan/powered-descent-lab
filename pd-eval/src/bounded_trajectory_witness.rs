//! Controller-neutral, positive-only bounded trajectory witnesses.
//!
//! This module owns the W1 exact verifier.  A witness contains only a finite
//! command trace; all state, contact, phase, and handoff results are derived
//! by replaying the authoritative discrete plant and the already-reviewed D0
//! neutral kernels.  No controller, mission outcome, planner rank, or solver
//! result is accepted by this boundary.

use std::f64::consts::PI;

use pd_core::{
    Command, ContactClassification, EvaluationGoal, LandingPadSpec, MissionSpec, RouteTopology,
    RunContext, SafetyProfile, SampleRecord, ScenarioSpec, SimConfig, SimulationState,
    TerrainDefinition, TransferRouteSpec, TransferWaypointSpec, Vec2, VehicleInitialState,
    VehicleSpec, WorldSpec,
};
use serde::{Deserialize, Serialize};

use crate::{
    RouteCapabilityInputV1, RouteCapabilityPhaseV1, RouteExecutionKernelInput,
    RouteExecutionResolutionKind, SourceTransitionCadence, SourceTransitionKernelInput,
    SourceTransitionKernelOutput, canonical_digest, extract_route_execution_kernel,
    extract_source_transition_kernel, ordered_route_phases,
};

/// Schema version for the controller-neutral witness configuration.
pub const BOUNDED_TRAJECTORY_CONFIGURATION_SCHEMA_VERSION: u32 = 1;
/// Schema version for a canonical command witness.
pub const BOUNDED_TRAJECTORY_WITNESS_SCHEMA_VERSION: u32 = 1;
/// Schema version for exact verification artifacts.
pub const BOUNDED_TRAJECTORY_VERIFICATION_SCHEMA_VERSION: u32 = 1;
/// Schema version for physical witness predictions.
pub const BOUNDED_TRAJECTORY_PREDICTION_SCHEMA_VERSION: u32 = 1;
/// Schema version for proposal identity/configuration metadata.
pub const BOUNDED_TRAJECTORY_PROPOSAL_CONFIGURATION_SCHEMA_VERSION: u32 = 1;

pub const BOUNDED_TRAJECTORY_CONFIGURATION_ID: &str = "bounded_trajectory_witness_configuration_v1";
pub const BOUNDED_TRAJECTORY_PLANT_SEMANTICS_ID: &str = "pd_core_discrete_plant_v1";
pub const BOUNDED_TRAJECTORY_COMMAND_SEMANTICS_ID: &str = "physics_rate_command_hold_2_v1";
pub const BOUNDED_TRAJECTORY_PHASE_SEMANTICS_ID: &str = "d0_physics_rate_phase_extractors_v1";
pub const BOUNDED_TRAJECTORY_CONTACT_SEMANTICS_ID: &str = "pd_core_rotated_hull_contact_v1";
pub const BOUNDED_TRAJECTORY_NUMBER_SEMANTICS_ID: &str = "rust_f64_canonical_numbers_v1";
pub const BOUNDED_TRAJECTORY_EXTRACTOR_SEMANTICS_ID: &str = "d0_source_route_kernels_v1";

pub const BOUNDED_TRAJECTORY_PHYSICS_HZ: u32 = 120;
pub const BOUNDED_TRAJECTORY_COMMAND_HZ: u32 = 60;
pub const BOUNDED_TRAJECTORY_HOLD_STEPS: u32 = 2;
pub const BOUNDED_TRAJECTORY_MAX_TIME_S: f64 = 130.0;
pub const BOUNDED_TRAJECTORY_MAX_PHYSICS_STEPS: u64 = 15_600;
pub const BOUNDED_TRAJECTORY_MAX_COMMAND_SLOTS: u64 = 7_800;
pub const BOUNDED_TRAJECTORY_VERIFICATION_REPETITIONS: u8 = 2;

pub const BOUNDED_REASON_SUPPORTED_EXACT_WITNESS_VERIFIED: &str =
    "supported/exact_witness_verified";
pub const BOUNDED_REASON_UNKNOWN_SCOPE_DIRECT_ROUTE: &str = "unknown/scope/direct_route";
pub const BOUNDED_REASON_UNKNOWN_COVERAGE_BOUNDED_SEARCH_EXHAUSTED: &str =
    "unknown/coverage/bounded_search_exhausted";
pub const BOUNDED_REASON_UNKNOWN_COVERAGE_NO_VERIFIED_WITNESS: &str =
    "unknown/coverage/no_verified_witness";
pub const BOUNDED_REASON_UNKNOWN_NUMERICAL_PROPOSAL_NONCONVERGED: &str =
    "unknown/numerical/proposal_nonconverged";
pub const BOUNDED_REASON_UNKNOWN_NUMERICAL_PROPOSAL_RECONSTRUCTION_REJECTED: &str =
    "unknown/numerical/proposal_reconstruction_rejected";

/// Artifact validity is separate from the physical two-way decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundedTrajectoryArtifactStatusV1 {
    Complete,
    Invalid,
}

/// V1 is deliberately positive-only: there is no physical Unsupported value.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundedTrajectoryDecisionV1 {
    Supported,
    Unknown,
}

/// Exact replay is either verified or a valid rejected proof attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundedTrajectoryVerificationVerdictV1 {
    Verified,
    Rejected,
}

/// Validate the dedicated V1 reason vocabulary.
pub fn validate_bounded_trajectory_reason_v1(reason: &str) -> Result<(), String> {
    if reason.trim().is_empty()
        || reason.ends_with('/')
        || reason.contains("//")
        || !reason
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'.'))
    {
        return Err(format!("invalid bounded trajectory reason: {reason:?}"));
    }

    if reason == BOUNDED_REASON_SUPPORTED_EXACT_WITNESS_VERIFIED {
        return Ok(());
    }
    if reason.starts_with("unknown/scope/") && reason.len() > "unknown/scope/".len() {
        return Ok(());
    }
    if matches!(
        reason,
        BOUNDED_REASON_UNKNOWN_COVERAGE_BOUNDED_SEARCH_EXHAUSTED
            | BOUNDED_REASON_UNKNOWN_COVERAGE_NO_VERIFIED_WITNESS
            | BOUNDED_REASON_UNKNOWN_NUMERICAL_PROPOSAL_NONCONVERGED
            | BOUNDED_REASON_UNKNOWN_NUMERICAL_PROPOSAL_RECONSTRUCTION_REJECTED
    ) {
        return Ok(());
    }
    let diagnostic_prefixes = [
        "rejected/",
        "invalid/input/",
        "invalid/artifact/",
        "invalid/numerical/",
        "invalid/determinism/",
    ];
    if diagnostic_prefixes
        .iter()
        .any(|prefix| reason.starts_with(prefix) && reason.len() > prefix.len())
    {
        return Ok(());
    }
    Err(format!("unsupported bounded trajectory reason: {reason:?}"))
}

fn invalid_reason(family: &str, message: &str) -> String {
    format!("{family}{}", sanitize_reason_token(message))
}

fn sanitize_reason_token(message: &str) -> String {
    let mut token = String::new();
    let mut separator = false;
    for byte in message.bytes() {
        if byte.is_ascii_alphanumeric() {
            token.push(byte.to_ascii_lowercase() as char);
            separator = false;
        } else if !separator {
            token.push('_');
            separator = true;
        }
    }
    let token = token.trim_matches('_');
    if token.is_empty() {
        "malformed".to_owned()
    } else {
        token.to_owned()
    }
}

/// Behavior-bearing exact plant/cadence/horizon semantics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedTrajectoryWitnessConfigurationV1 {
    pub schema_version: u32,
    pub configuration_id: String,
    pub plant_semantics_id: String,
    pub command_semantics_id: String,
    pub phase_semantics_id: String,
    pub extractor_semantics_id: String,
    pub contact_semantics_id: String,
    pub number_semantics_id: String,
    pub physics_hz: u32,
    pub command_hz: u32,
    pub hold_steps: u32,
    pub max_time_s: f64,
    pub max_physics_steps: u64,
    pub max_command_slots: u64,
    pub max_replay_steps: u64,
    pub verification_repetitions: u8,
    pub configuration_digest: String,
}

#[derive(Serialize)]
struct ConfigurationDigestMaterial<'a> {
    schema_version: u32,
    configuration_id: &'a str,
    plant_semantics_id: &'a str,
    command_semantics_id: &'a str,
    phase_semantics_id: &'a str,
    extractor_semantics_id: &'a str,
    contact_semantics_id: &'a str,
    number_semantics_id: &'a str,
    physics_hz: u32,
    command_hz: u32,
    hold_steps: u32,
    max_time_s: f64,
    max_physics_steps: u64,
    max_command_slots: u64,
    max_replay_steps: u64,
    verification_repetitions: u8,
}

impl BoundedTrajectoryWitnessConfigurationV1 {
    pub fn v1() -> Self {
        Self {
            schema_version: BOUNDED_TRAJECTORY_CONFIGURATION_SCHEMA_VERSION,
            configuration_id: BOUNDED_TRAJECTORY_CONFIGURATION_ID.to_owned(),
            plant_semantics_id: BOUNDED_TRAJECTORY_PLANT_SEMANTICS_ID.to_owned(),
            command_semantics_id: BOUNDED_TRAJECTORY_COMMAND_SEMANTICS_ID.to_owned(),
            phase_semantics_id: BOUNDED_TRAJECTORY_PHASE_SEMANTICS_ID.to_owned(),
            extractor_semantics_id: BOUNDED_TRAJECTORY_EXTRACTOR_SEMANTICS_ID.to_owned(),
            contact_semantics_id: BOUNDED_TRAJECTORY_CONTACT_SEMANTICS_ID.to_owned(),
            number_semantics_id: BOUNDED_TRAJECTORY_NUMBER_SEMANTICS_ID.to_owned(),
            physics_hz: BOUNDED_TRAJECTORY_PHYSICS_HZ,
            command_hz: BOUNDED_TRAJECTORY_COMMAND_HZ,
            hold_steps: BOUNDED_TRAJECTORY_HOLD_STEPS,
            max_time_s: BOUNDED_TRAJECTORY_MAX_TIME_S,
            max_physics_steps: BOUNDED_TRAJECTORY_MAX_PHYSICS_STEPS,
            max_command_slots: BOUNDED_TRAJECTORY_MAX_COMMAND_SLOTS,
            max_replay_steps: BOUNDED_TRAJECTORY_MAX_PHYSICS_STEPS,
            verification_repetitions: BOUNDED_TRAJECTORY_VERIFICATION_REPETITIONS,
            configuration_digest: String::new(),
        }
    }

    pub fn digest(&self) -> Result<String, String> {
        self.validate_without_digest()?;
        canonical_digest(&ConfigurationDigestMaterial {
            schema_version: self.schema_version,
            configuration_id: &self.configuration_id,
            plant_semantics_id: &self.plant_semantics_id,
            command_semantics_id: &self.command_semantics_id,
            phase_semantics_id: &self.phase_semantics_id,
            extractor_semantics_id: &self.extractor_semantics_id,
            contact_semantics_id: &self.contact_semantics_id,
            number_semantics_id: &self.number_semantics_id,
            physics_hz: self.physics_hz,
            command_hz: self.command_hz,
            hold_steps: self.hold_steps,
            max_time_s: self.max_time_s,
            max_physics_steps: self.max_physics_steps,
            max_command_slots: self.max_command_slots,
            max_replay_steps: self.max_replay_steps,
            verification_repetitions: self.verification_repetitions,
        })
    }

    pub fn seal(mut self) -> Result<Self, String> {
        self.schema_version = BOUNDED_TRAJECTORY_CONFIGURATION_SCHEMA_VERSION;
        self.validate_without_digest()?;
        self.configuration_digest = self.digest()?;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_without_digest()?;
        let expected = self.digest()?;
        if self.configuration_digest != expected {
            return Err(format!(
                "configuration_digest does not match canonical configuration: expected {expected}"
            ));
        }
        Ok(())
    }

    fn validate_without_digest(&self) -> Result<(), String> {
        if self.schema_version != BOUNDED_TRAJECTORY_CONFIGURATION_SCHEMA_VERSION {
            return Err(format!(
                "configuration schema_version must equal {BOUNDED_TRAJECTORY_CONFIGURATION_SCHEMA_VERSION}"
            ));
        }
        for (name, actual, expected) in [
            (
                "configuration_id",
                self.configuration_id.as_str(),
                BOUNDED_TRAJECTORY_CONFIGURATION_ID,
            ),
            (
                "plant_semantics_id",
                self.plant_semantics_id.as_str(),
                BOUNDED_TRAJECTORY_PLANT_SEMANTICS_ID,
            ),
            (
                "command_semantics_id",
                self.command_semantics_id.as_str(),
                BOUNDED_TRAJECTORY_COMMAND_SEMANTICS_ID,
            ),
            (
                "phase_semantics_id",
                self.phase_semantics_id.as_str(),
                BOUNDED_TRAJECTORY_PHASE_SEMANTICS_ID,
            ),
            (
                "extractor_semantics_id",
                self.extractor_semantics_id.as_str(),
                BOUNDED_TRAJECTORY_EXTRACTOR_SEMANTICS_ID,
            ),
            (
                "contact_semantics_id",
                self.contact_semantics_id.as_str(),
                BOUNDED_TRAJECTORY_CONTACT_SEMANTICS_ID,
            ),
            (
                "number_semantics_id",
                self.number_semantics_id.as_str(),
                BOUNDED_TRAJECTORY_NUMBER_SEMANTICS_ID,
            ),
        ] {
            if actual != expected {
                return Err(format!(
                    "{name} must equal the locked V1 value {expected:?}"
                ));
            }
        }
        if self.max_time_s.to_bits() != BOUNDED_TRAJECTORY_MAX_TIME_S.to_bits() {
            return Err(format!(
                "max_time_s must equal the locked V1 value {BOUNDED_TRAJECTORY_MAX_TIME_S}"
            ));
        }
        if self.physics_hz == 0
            || self.command_hz == 0
            || self.hold_steps == 0
            || !self.physics_hz.is_multiple_of(self.command_hz)
            || self.physics_hz / self.command_hz != self.hold_steps
        {
            return Err("configuration cadence relationship is invalid".to_owned());
        }
        if self.physics_hz != BOUNDED_TRAJECTORY_PHYSICS_HZ
            || self.command_hz != BOUNDED_TRAJECTORY_COMMAND_HZ
            || self.hold_steps != BOUNDED_TRAJECTORY_HOLD_STEPS
        {
            return Err("configuration cadence does not match locked V1 values".to_owned());
        }
        let expected_steps = (self.max_time_s * f64::from(self.physics_hz)).round();
        if !expected_steps.is_finite()
            || expected_steps <= 0.0
            || expected_steps.fract() != 0.0
            || expected_steps as u64 != self.max_physics_steps
        {
            return Err("max_physics_steps does not match max_time_s and physics_hz".to_owned());
        }
        let expected_slots = self
            .max_physics_steps
            .checked_add(u64::from(self.hold_steps) - 1)
            .map(|value| value / u64::from(self.hold_steps))
            .ok_or_else(|| "max_physics_steps overflowed command-slot calculation".to_owned())?;
        if self.max_physics_steps != BOUNDED_TRAJECTORY_MAX_PHYSICS_STEPS
            || self.max_command_slots != BOUNDED_TRAJECTORY_MAX_COMMAND_SLOTS
            || expected_slots != BOUNDED_TRAJECTORY_MAX_COMMAND_SLOTS
            || self.max_replay_steps != BOUNDED_TRAJECTORY_MAX_PHYSICS_STEPS
        {
            return Err("configuration horizon does not match locked V1 values".to_owned());
        }
        if self.verification_repetitions != BOUNDED_TRAJECTORY_VERIFICATION_REPETITIONS {
            return Err("configuration must use two verification repetitions".to_owned());
        }
        Ok(())
    }
}

/// Proposal identity is optional metadata in W1; no proposal engine is owned
/// by this module.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedTrajectoryProposalConfigurationV1 {
    pub schema_version: u32,
    pub engine_id: String,
    pub parameterization_id: String,
    pub numerical_settings_id: String,
    pub search_limit: u64,
    pub reconstruction_rule_id: String,
    pub determinism_id: String,
    pub configuration_digest: String,
}

#[derive(Serialize)]
struct ProposalConfigurationDigestMaterial<'a> {
    schema_version: u32,
    engine_id: &'a str,
    parameterization_id: &'a str,
    numerical_settings_id: &'a str,
    search_limit: u64,
    reconstruction_rule_id: &'a str,
    determinism_id: &'a str,
}

impl BoundedTrajectoryProposalConfigurationV1 {
    pub fn seal(mut self) -> Result<Self, String> {
        self.schema_version = BOUNDED_TRAJECTORY_PROPOSAL_CONFIGURATION_SCHEMA_VERSION;
        self.validate_without_digest()?;
        self.configuration_digest = self.digest()?;
        Ok(self)
    }

    pub fn digest(&self) -> Result<String, String> {
        self.validate_without_digest()?;
        canonical_digest(&ProposalConfigurationDigestMaterial {
            schema_version: self.schema_version,
            engine_id: &self.engine_id,
            parameterization_id: &self.parameterization_id,
            numerical_settings_id: &self.numerical_settings_id,
            search_limit: self.search_limit,
            reconstruction_rule_id: &self.reconstruction_rule_id,
            determinism_id: &self.determinism_id,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_without_digest()?;
        let expected = self.digest()?;
        if self.configuration_digest != expected {
            return Err(format!(
                "proposal configuration digest mismatch: expected {expected}"
            ));
        }
        Ok(())
    }

    fn validate_without_digest(&self) -> Result<(), String> {
        if self.schema_version != BOUNDED_TRAJECTORY_PROPOSAL_CONFIGURATION_SCHEMA_VERSION {
            return Err("proposal configuration schema version is invalid".to_owned());
        }
        for (name, value) in [
            ("engine_id", self.engine_id.as_str()),
            ("parameterization_id", self.parameterization_id.as_str()),
            ("numerical_settings_id", self.numerical_settings_id.as_str()),
            (
                "reconstruction_rule_id",
                self.reconstruction_rule_id.as_str(),
            ),
            ("determinism_id", self.determinism_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("proposal {name} must not be empty"));
            }
        }
        if self.search_limit == 0 {
            return Err("proposal search_limit must be positive".to_owned());
        }
        Ok(())
    }
}

/// One canonical command slot.  The verifier never clamps this value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedTrajectoryCommandV1 {
    pub slot: u64,
    pub throttle_frac: f64,
    pub target_attitude_rad: f64,
}

impl BoundedTrajectoryCommandV1 {
    pub fn validate(&self) -> Result<(), String> {
        if !self.throttle_frac.is_finite() || !self.target_attitude_rad.is_finite() {
            return Err(format!("command slot {} has nonfinite value", self.slot));
        }
        if self.throttle_frac.to_bits() == (-0.0_f64).to_bits()
            || self.target_attitude_rad.to_bits() == (-0.0_f64).to_bits()
        {
            return Err(format!("command slot {} contains negative zero", self.slot));
        }
        if !(0.0..=1.0).contains(&self.throttle_frac) {
            return Err(format!(
                "command slot {} throttle is outside [0, 1]",
                self.slot
            ));
        }
        if !(-PI..=PI).contains(&self.target_attitude_rad) {
            return Err(format!(
                "command slot {} attitude is outside [-pi, pi]",
                self.slot
            ));
        }
        Ok(())
    }

    fn core_command(self) -> Command {
        Command {
            throttle_frac: self.throttle_frac,
            target_attitude_rad: self.target_attitude_rad,
        }
    }
}

/// The sealed finite command trace and its claimed terminal step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedTrajectoryWitnessV1 {
    pub schema_version: u32,
    pub input_digest: String,
    pub configuration_digest: String,
    pub commands: Vec<BoundedTrajectoryCommandV1>,
    pub terminal_physics_step: u64,
    pub witness_digest: String,
}

#[derive(Serialize)]
struct WitnessDigestMaterial<'a> {
    schema_version: u32,
    input_digest: &'a str,
    configuration_digest: &'a str,
    commands: &'a [BoundedTrajectoryCommandV1],
    terminal_physics_step: u64,
}

impl BoundedTrajectoryWitnessV1 {
    pub fn new(
        input_digest: impl Into<String>,
        configuration_digest: impl Into<String>,
        commands: Vec<BoundedTrajectoryCommandV1>,
        terminal_physics_step: u64,
    ) -> Result<Self, String> {
        Self {
            schema_version: BOUNDED_TRAJECTORY_WITNESS_SCHEMA_VERSION,
            input_digest: input_digest.into(),
            configuration_digest: configuration_digest.into(),
            commands,
            terminal_physics_step,
            witness_digest: String::new(),
        }
        .seal()
    }

    pub fn digest(&self) -> Result<String, String> {
        self.validate_without_digest(None)?;
        canonical_digest(&WitnessDigestMaterial {
            schema_version: self.schema_version,
            input_digest: &self.input_digest,
            configuration_digest: &self.configuration_digest,
            commands: &self.commands,
            terminal_physics_step: self.terminal_physics_step,
        })
    }

    pub fn seal(mut self) -> Result<Self, String> {
        self.schema_version = BOUNDED_TRAJECTORY_WITNESS_SCHEMA_VERSION;
        self.validate_without_digest(None)?;
        self.witness_digest = self.digest()?;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_without_digest(None)?;
        let expected = self.digest()?;
        if self.witness_digest != expected {
            return Err(format!(
                "witness_digest does not match canonical witness: expected {expected}"
            ));
        }
        Ok(())
    }

    fn validate_against(
        &self,
        input_digest: &str,
        configuration: &BoundedTrajectoryWitnessConfigurationV1,
    ) -> Result<(), String> {
        self.validate_without_digest(Some(configuration))?;
        let expected = self.digest()?;
        if self.witness_digest != expected {
            return Err(format!(
                "witness_digest does not match canonical witness: expected {expected}"
            ));
        }
        if self.input_digest != input_digest {
            return Err("witness input_digest does not match supplied input".to_owned());
        }
        if self.configuration_digest != configuration.configuration_digest {
            return Err(
                "witness configuration_digest does not match supplied configuration".to_owned(),
            );
        }
        Ok(())
    }

    fn validate_without_digest(
        &self,
        configuration: Option<&BoundedTrajectoryWitnessConfigurationV1>,
    ) -> Result<(), String> {
        if self.schema_version != BOUNDED_TRAJECTORY_WITNESS_SCHEMA_VERSION {
            return Err("witness schema_version is invalid".to_owned());
        }
        if self.input_digest.trim().is_empty() || self.configuration_digest.trim().is_empty() {
            return Err("witness input/configuration digests must not be empty".to_owned());
        }
        if self.terminal_physics_step == 0 {
            return Err("witness terminal_physics_step must be positive".to_owned());
        }
        let hold_steps = configuration.map_or(u64::from(BOUNDED_TRAJECTORY_HOLD_STEPS), |config| {
            u64::from(config.hold_steps)
        });
        let expected_commands = self
            .terminal_physics_step
            .checked_add(hold_steps - 1)
            .ok_or_else(|| "witness command count calculation overflowed".to_owned())?
            / hold_steps;
        if self.commands.len() as u64 != expected_commands {
            return Err(format!(
                "witness has {} commands, expected {expected_commands}",
                self.commands.len()
            ));
        }
        if let Some(config) = configuration
            && (self.terminal_physics_step > config.max_physics_steps
                || expected_commands > config.max_command_slots)
        {
            return Err("witness exceeds configuration horizon".to_owned());
        }
        for (index, command) in self.commands.iter().enumerate() {
            if command.slot != index as u64 {
                return Err(format!("witness command {index} has non-contiguous slot"));
            }
            command.validate()?;
        }
        Ok(())
    }
}

/// Contact classification persisted in the verification artifact.  This is a
/// serializable mirror of the core enum, kept local so the proof schema does
/// not depend on mission outcome serialization.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundedTrajectoryContactClassificationV1 {
    None,
    StableTouchdown { on_target: bool },
    Crash,
}

impl From<ContactClassification> for BoundedTrajectoryContactClassificationV1 {
    fn from(value: ContactClassification) -> Self {
        match value {
            ContactClassification::None => Self::None,
            ContactClassification::StableTouchdown { on_target } => {
                Self::StableTouchdown { on_target }
            }
            ContactClassification::Crash => Self::Crash,
        }
    }
}

/// One proof-bearing raw state.  The initial record is retained with idle as
/// its pre-install command; every later record names the command that caused
/// that state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedTrajectoryStateV1 {
    pub physics_step: u64,
    pub sim_time_s: f64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
    pub mass_kg: f64,
    pub held_command: Option<BoundedTrajectoryCommandV1>,
    pub contact: BoundedTrajectoryContactClassificationV1,
}

/// Command-install events are separate from post-step state records so an
/// even-step command boundary cannot be confused with the command that
/// produced that state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedTrajectoryCommandInstallationV1 {
    pub physics_step: u64,
    pub command: BoundedTrajectoryCommandV1,
}

/// A compact terminal/extrema payload retained by the verification artifact.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedTrajectoryExtremaV1 {
    pub min_touchdown_clearance_m: f64,
    pub min_hull_clearance_m: f64,
    pub max_speed_mps: f64,
    pub max_abs_attitude_rad: f64,
    pub max_abs_angular_rate_radps: f64,
}

/// A phase identity plus its authoritative terminal raw-state index.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedTrajectoryPhaseResultV1 {
    pub phase: RouteCapabilityPhaseV1,
    pub passed: bool,
    pub terminal_physics_step: Option<u64>,
}

/// A source/waypoint boundary identity and its raw endpoint.  D0 brackets are
/// retained in the verifier as canonical serialized JSON for audit/debugging;
/// the raw post-step endpoint is what drives the proof.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedTrajectoryBoundaryResultV1 {
    pub name: String,
    pub physics_step: Option<u64>,
    pub sample_index: Option<usize>,
    pub initial_deadline: bool,
}

/// The exact replay result.  A complete result has a verified or rejected
/// verdict; malformed artifacts have invalid status and no verdict.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedTrajectoryVerificationV1 {
    pub schema_version: u32,
    pub status: BoundedTrajectoryArtifactStatusV1,
    pub verdict: Option<BoundedTrajectoryVerificationVerdictV1>,
    pub input_digest: String,
    pub configuration_digest: String,
    pub witness_digest: String,
    pub terminal_physics_step: Option<u64>,
    pub terminal_state: Option<BoundedTrajectoryStateV1>,
    pub extrema: Option<BoundedTrajectoryExtremaV1>,
    pub phases: Vec<BoundedTrajectoryPhaseResultV1>,
    pub boundaries: Vec<BoundedTrajectoryBoundaryResultV1>,
    pub trajectory_digest: String,
    pub first_reason: Option<String>,
    pub verification_digest: String,
}

#[derive(Serialize)]
struct VerificationDigestMaterial<'a> {
    schema_version: u32,
    status: BoundedTrajectoryArtifactStatusV1,
    verdict: Option<BoundedTrajectoryVerificationVerdictV1>,
    input_digest: &'a str,
    configuration_digest: &'a str,
    witness_digest: &'a str,
    terminal_physics_step: Option<u64>,
    terminal_state: &'a Option<BoundedTrajectoryStateV1>,
    extrema: &'a Option<BoundedTrajectoryExtremaV1>,
    phases: &'a [BoundedTrajectoryPhaseResultV1],
    boundaries: &'a [BoundedTrajectoryBoundaryResultV1],
    trajectory_digest: &'a str,
    first_reason: &'a Option<String>,
}

impl BoundedTrajectoryVerificationV1 {
    fn invalid_payload(
        input_digest: String,
        configuration_digest: String,
        witness_digest: String,
        reason: String,
    ) -> Self {
        Self {
            schema_version: BOUNDED_TRAJECTORY_VERIFICATION_SCHEMA_VERSION,
            status: BoundedTrajectoryArtifactStatusV1::Invalid,
            verdict: None,
            input_digest,
            configuration_digest,
            witness_digest,
            terminal_physics_step: None,
            terminal_state: None,
            extrema: None,
            phases: Vec::new(),
            boundaries: Vec::new(),
            trajectory_digest: "unavailable".to_owned(),
            first_reason: Some(reason),
            verification_digest: String::new(),
        }
    }

    fn new_invalid(
        input_digest: String,
        configuration_digest: String,
        witness_digest: String,
        reason: String,
    ) -> Self {
        let nonempty = |value: String| {
            if value.trim().is_empty() {
                "unavailable".to_owned()
            } else {
                value
            }
        };
        let mut value = Self::invalid_payload(
            nonempty(input_digest),
            nonempty(configuration_digest),
            nonempty(witness_digest),
            reason,
        );
        match value.compute_digest() {
            Ok(digest) => value.verification_digest = digest,
            Err(error) => {
                value.first_reason = Some(invalid_reason("invalid/numerical/", &error));
                // This branch is itself an invalid artifact.  All fields in
                // the invalid payload are strings/options, so canonical JSON
                // failure is not expected; retain an explicit marker rather
                // than manufacturing a valid-looking digest.
                value.verification_digest = "canonicalization_failed".to_owned();
            }
        }
        value
    }

    fn new_complete(payload: VerificationPayload) -> Result<Self, String> {
        let mut value = Self {
            schema_version: BOUNDED_TRAJECTORY_VERIFICATION_SCHEMA_VERSION,
            status: BoundedTrajectoryArtifactStatusV1::Complete,
            verdict: Some(payload.verdict),
            input_digest: payload.input_digest,
            configuration_digest: payload.configuration_digest,
            witness_digest: payload.witness_digest,
            terminal_physics_step: Some(payload.terminal_physics_step),
            terminal_state: Some(payload.terminal_state),
            extrema: Some(payload.extrema),
            phases: payload.phases,
            boundaries: payload.boundaries,
            trajectory_digest: payload.trajectory_digest,
            first_reason: Some(payload.reason),
            verification_digest: String::new(),
        };
        value.verification_digest = value.compute_digest()?;
        Ok(value)
    }

    pub fn digest(&self) -> Result<String, String> {
        self.compute_digest()
    }

    fn compute_digest(&self) -> Result<String, String> {
        canonical_digest(&VerificationDigestMaterial {
            schema_version: self.schema_version,
            status: self.status,
            verdict: self.verdict,
            input_digest: &self.input_digest,
            configuration_digest: &self.configuration_digest,
            witness_digest: &self.witness_digest,
            terminal_physics_step: self.terminal_physics_step,
            terminal_state: &self.terminal_state,
            extrema: &self.extrema,
            phases: &self.phases,
            boundaries: &self.boundaries,
            trajectory_digest: &self.trajectory_digest,
            first_reason: &self.first_reason,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != BOUNDED_TRAJECTORY_VERIFICATION_SCHEMA_VERSION {
            return Err("verification schema_version is invalid".to_owned());
        }
        if self.input_digest.trim().is_empty()
            || self.configuration_digest.trim().is_empty()
            || self.witness_digest.trim().is_empty()
        {
            return Err("verification joins must not be empty".to_owned());
        }
        let expected = self.compute_digest()?;
        if self.verification_digest != expected {
            return Err(format!(
                "verification_digest does not match canonical verification: expected {expected}"
            ));
        }
        match self.status {
            BoundedTrajectoryArtifactStatusV1::Invalid => {
                if self.verdict.is_some() {
                    return Err("invalid verification cannot carry a verdict".to_owned());
                }
                if self.terminal_physics_step.is_some()
                    || self.terminal_state.is_some()
                    || self.extrema.is_some()
                    || !self.phases.is_empty()
                    || !self.boundaries.is_empty()
                    || self.trajectory_digest != "unavailable"
                {
                    return Err("invalid verification cannot carry proof payload".to_owned());
                }
                let reason = self
                    .first_reason
                    .as_deref()
                    .ok_or_else(|| "invalid verification requires a reason".to_owned())?;
                if !reason.starts_with("invalid/") {
                    return Err("invalid verification requires invalid/* reason".to_owned());
                }
                validate_bounded_trajectory_reason_v1(reason)?;
            }
            BoundedTrajectoryArtifactStatusV1::Complete => {
                let verdict = self
                    .verdict
                    .ok_or_else(|| "complete verification requires a verdict".to_owned())?;
                let reason = self
                    .first_reason
                    .as_deref()
                    .ok_or_else(|| "complete verification requires a reason".to_owned())?;
                validate_bounded_trajectory_reason_v1(reason)?;
                match verdict {
                    BoundedTrajectoryVerificationVerdictV1::Verified => {
                        if reason != BOUNDED_REASON_SUPPORTED_EXACT_WITNESS_VERIFIED {
                            return Err(
                                "verified result requires the exact supported reason".to_owned()
                            );
                        }
                    }
                    BoundedTrajectoryVerificationVerdictV1::Rejected => {
                        if !reason.starts_with("rejected/") {
                            return Err("rejected result requires rejected/* reason".to_owned());
                        }
                    }
                }
                let terminal_step = self
                    .terminal_physics_step
                    .ok_or_else(|| "complete verification is missing terminal step".to_owned())?;
                let terminal_state = self
                    .terminal_state
                    .as_ref()
                    .ok_or_else(|| "complete verification is missing terminal state".to_owned())?;
                let extrema = self
                    .extrema
                    .as_ref()
                    .ok_or_else(|| "complete verification is missing extrema".to_owned())?;
                if terminal_step == 0 || terminal_step > BOUNDED_TRAJECTORY_MAX_PHYSICS_STEPS {
                    return Err("verification terminal step is outside locked horizon".to_owned());
                }
                validate_state(terminal_state, terminal_step)?;
                validate_extrema(extrema)?;
                if self.trajectory_digest.trim().is_empty() {
                    return Err("complete verification requires a trajectory digest".to_owned());
                }
                let waypoint_count = validate_phase_results(&self.phases, terminal_step, verdict)?;
                validate_boundary_results(
                    &self.boundaries,
                    waypoint_count,
                    terminal_step,
                    verdict,
                )?;
            }
        }
        Ok(())
    }

    /// Validate a loaded verification artifact against the exact replay of
    /// its sealed physical input, configuration, and command witness.
    pub fn validate_against_exact(
        &self,
        input: &RouteCapabilityInputV1,
        configuration: &BoundedTrajectoryWitnessConfigurationV1,
        witness: &BoundedTrajectoryWitnessV1,
    ) -> Result<(), String> {
        input.validate()?;
        configuration.validate()?;
        let input_digest = input.physical_digest()?;
        witness.validate_against(&input_digest, configuration)?;
        self.validate()?;
        let expected = verify_bounded_trajectory(input, configuration, witness);
        if &expected != self {
            return Err("verification artifact does not match exact replay".to_owned());
        }
        Ok(())
    }
}

struct VerificationPayload {
    input_digest: String,
    configuration_digest: String,
    witness_digest: String,
    verdict: BoundedTrajectoryVerificationVerdictV1,
    terminal_physics_step: u64,
    terminal_state: BoundedTrajectoryStateV1,
    extrema: BoundedTrajectoryExtremaV1,
    phases: Vec<BoundedTrajectoryPhaseResultV1>,
    boundaries: Vec<BoundedTrajectoryBoundaryResultV1>,
    trajectory_digest: String,
    reason: String,
}

fn validate_state(state: &BoundedTrajectoryStateV1, expected_step: u64) -> Result<(), String> {
    if state.physics_step != expected_step {
        return Err("terminal state step does not match artifact terminal step".to_owned());
    }
    let expected_time = expected_step as f64 / f64::from(BOUNDED_TRAJECTORY_PHYSICS_HZ);
    for (field, value) in [
        ("sim_time_s", state.sim_time_s),
        ("position_m.x", state.position_m.x),
        ("position_m.y", state.position_m.y),
        ("velocity_mps.x", state.velocity_mps.x),
        ("velocity_mps.y", state.velocity_mps.y),
        ("attitude_rad", state.attitude_rad),
        ("angular_rate_radps", state.angular_rate_radps),
        ("fuel_kg", state.fuel_kg),
        ("mass_kg", state.mass_kg),
    ] {
        if !value.is_finite() {
            return Err(format!("terminal state {field} is nonfinite"));
        }
    }
    if (state.sim_time_s - expected_time).abs() > 1.0e-12 {
        return Err("terminal state time does not match artifact terminal step".to_owned());
    }
    let command = state
        .held_command
        .ok_or_else(|| "terminal state is missing its producing command".to_owned())?;
    command.validate()?;
    Ok(())
}

fn validate_extrema(extrema: &BoundedTrajectoryExtremaV1) -> Result<(), String> {
    for (field, value) in [
        (
            "min_touchdown_clearance_m",
            extrema.min_touchdown_clearance_m,
        ),
        ("min_hull_clearance_m", extrema.min_hull_clearance_m),
        ("max_speed_mps", extrema.max_speed_mps),
        ("max_abs_attitude_rad", extrema.max_abs_attitude_rad),
        (
            "max_abs_angular_rate_radps",
            extrema.max_abs_angular_rate_radps,
        ),
    ] {
        if !value.is_finite() {
            return Err(format!("verification extrema {field} is nonfinite"));
        }
    }
    Ok(())
}

fn validate_replay_state(state: &SimulationState, context: &RunContext) -> Result<(), String> {
    let values = [
        ("sim_time_s", state.sim_time_s),
        ("position_m.x", state.position_m.x),
        ("position_m.y", state.position_m.y),
        ("velocity_mps.x", state.velocity_mps.x),
        ("velocity_mps.y", state.velocity_mps.y),
        ("attitude_rad", state.attitude_rad),
        ("angular_rate_radps", state.angular_rate_radps),
        ("fuel_kg", state.fuel_kg),
        ("mass_kg", state.mass_kg(context)),
        ("min_touchdown_clearance_m", state.min_touchdown_clearance_m),
        ("min_hull_clearance_m", state.min_hull_clearance_m),
        ("max_speed_mps", state.max_speed_mps),
        ("max_abs_attitude_rad", state.max_abs_attitude_rad),
        (
            "max_abs_angular_rate_radps",
            state.max_abs_angular_rate_radps,
        ),
    ];
    if let Some((field, _)) = values.iter().find(|(_, value)| !value.is_finite()) {
        return Err(format!("replay state {field} is nonfinite"));
    }
    Ok(())
}

fn validate_phase_results(
    phases: &[BoundedTrajectoryPhaseResultV1],
    terminal_step: u64,
    verdict: BoundedTrajectoryVerificationVerdictV1,
) -> Result<usize, String> {
    if phases.len() < 3 || !(phases.len() - 3).is_multiple_of(2) {
        return Err("verification phase sequence has an invalid length".to_owned());
    }
    let waypoint_count = (phases.len() - 3) / 2;
    let expected = ordered_route_phases(waypoint_count);
    if phases
        .iter()
        .zip(expected.iter())
        .any(|(actual, expected)| actual.phase != *expected)
    {
        return Err("verification phase sequence is not canonical".to_owned());
    }
    let mut unresolved = false;
    for (index, phase) in phases.iter().enumerate() {
        if phase.passed != phase.terminal_physics_step.is_some() {
            return Err(format!(
                "verification phase {index} pass/terminal fields disagree"
            ));
        }
        if let Some(step) = phase.terminal_physics_step {
            if step > terminal_step {
                return Err(format!("verification phase {index} exceeds terminal step"));
            }
            if unresolved {
                return Err("verification phase passed after an unresolved phase".to_owned());
            }
        } else {
            unresolved = true;
        }
    }
    if phases[0].terminal_physics_step != Some(0) {
        return Err("verification initial phase must terminate at step zero".to_owned());
    }
    if matches!(verdict, BoundedTrajectoryVerificationVerdictV1::Verified) {
        if !(1..=2).contains(&waypoint_count) || phases.iter().any(|phase| !phase.passed) {
            return Err("verified result must pass one or two complete waypoint phases".to_owned());
        }
        if phases.last().and_then(|phase| phase.terminal_physics_step) != Some(terminal_step) {
            return Err("verified final phase does not align with terminal step".to_owned());
        }
    }
    Ok(waypoint_count)
}

fn validate_boundary_results(
    boundaries: &[BoundedTrajectoryBoundaryResultV1],
    waypoint_count: usize,
    terminal_step: u64,
    verdict: BoundedTrajectoryVerificationVerdictV1,
) -> Result<(), String> {
    let expected_count = 2 + waypoint_count;
    if boundaries.is_empty() || boundaries.len() > expected_count {
        return Err("verification boundary sequence has an invalid length".to_owned());
    }
    for (index, boundary) in boundaries.iter().enumerate() {
        let expected_name = match index {
            0 => "contact_exit".to_owned(),
            1 => "tracking_entry".to_owned(),
            handoff_index => format!("handoff_{}", handoff_index - 2),
        };
        if boundary.name != expected_name {
            return Err("verification boundary sequence is not canonical".to_owned());
        }
        if index < 2 && boundary.initial_deadline {
            return Err("source boundary cannot be an initial deadline".to_owned());
        }
        if boundary.physics_step.is_some() != boundary.sample_index.is_some() {
            return Err("verification boundary step/sample fields disagree".to_owned());
        }
        if let (Some(step), Some(sample_index)) = (boundary.physics_step, boundary.sample_index)
            && (step > terminal_step || sample_index as u64 != step)
        {
            return Err("verification boundary endpoint is outside the replay prefix".to_owned());
        }
    }
    if matches!(verdict, BoundedTrajectoryVerificationVerdictV1::Verified) {
        if boundaries.len() != expected_count
            || boundaries
                .iter()
                .any(|boundary| boundary.physics_step.is_none())
        {
            return Err("verified result must carry every ordered boundary".to_owned());
        }
        if boundaries.last().and_then(|boundary| boundary.physics_step) != Some(terminal_step) {
            return Err("verified final boundary does not align with terminal step".to_owned());
        }
    }
    Ok(())
}

/// Sealed controller-neutral physical prediction.  W1 fills this from an
/// exact verification artifact; proposal engines may add only an optional
/// configuration identity in later stages.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundedTrajectoryWitnessPredictionV1 {
    pub schema_version: u32,
    pub status: BoundedTrajectoryArtifactStatusV1,
    pub decision: Option<BoundedTrajectoryDecisionV1>,
    pub input_digest: String,
    pub configuration_digest: String,
    pub proposal_configuration_digest: Option<String>,
    pub witness_digest: Option<String>,
    pub verification_digest: Option<String>,
    pub attempted_verification_digests: Vec<String>,
    pub first_reason: Option<String>,
    pub prediction_digest: String,
}

#[derive(Serialize)]
struct PredictionDigestMaterial<'a> {
    schema_version: u32,
    status: BoundedTrajectoryArtifactStatusV1,
    decision: Option<BoundedTrajectoryDecisionV1>,
    input_digest: &'a str,
    configuration_digest: &'a str,
    proposal_configuration_digest: &'a Option<String>,
    witness_digest: &'a Option<String>,
    verification_digest: &'a Option<String>,
    attempted_verification_digests: &'a [String],
    first_reason: &'a Option<String>,
}

impl BoundedTrajectoryWitnessPredictionV1 {
    pub fn from_verification(
        input: &RouteCapabilityInputV1,
        configuration: &BoundedTrajectoryWitnessConfigurationV1,
        verification: &BoundedTrajectoryVerificationV1,
        proposal_configuration_digest: Option<String>,
    ) -> Result<Self, String> {
        input.validate()?;
        configuration.validate()?;
        verification.validate()?;
        let input_digest = input.physical_digest()?;
        if verification.input_digest != input_digest
            || verification.configuration_digest != configuration.configuration_digest
        {
            return Err("verification does not match prediction input/configuration".to_owned());
        }
        let (status, decision, witness_digest, reason) = match verification.verdict {
            Some(BoundedTrajectoryVerificationVerdictV1::Verified) => (
                BoundedTrajectoryArtifactStatusV1::Complete,
                Some(BoundedTrajectoryDecisionV1::Supported),
                Some(verification.witness_digest.clone()),
                BOUNDED_REASON_SUPPORTED_EXACT_WITNESS_VERIFIED.to_owned(),
            ),
            Some(BoundedTrajectoryVerificationVerdictV1::Rejected) => {
                let rejection_reason = verification
                    .first_reason
                    .as_deref()
                    .ok_or_else(|| "rejected verification is missing its reason".to_owned())?;
                validate_bounded_trajectory_reason_v1(rejection_reason)?;
                let reason = if let Some(suffix) = rejection_reason.strip_prefix("rejected/scope/")
                {
                    let mapped = format!("unknown/scope/{suffix}");
                    validate_bounded_trajectory_reason_v1(&mapped)?;
                    mapped
                } else {
                    BOUNDED_REASON_UNKNOWN_COVERAGE_NO_VERIFIED_WITNESS.to_owned()
                };
                (
                    BoundedTrajectoryArtifactStatusV1::Complete,
                    Some(BoundedTrajectoryDecisionV1::Unknown),
                    None,
                    reason,
                )
            }
            None => (
                BoundedTrajectoryArtifactStatusV1::Invalid,
                None,
                None,
                verification
                    .first_reason
                    .clone()
                    .unwrap_or_else(|| "invalid/artifact/verification_missing_verdict".to_owned()),
            ),
        };
        let prediction = Self {
            schema_version: BOUNDED_TRAJECTORY_PREDICTION_SCHEMA_VERSION,
            status,
            decision,
            input_digest,
            configuration_digest: configuration.configuration_digest.clone(),
            proposal_configuration_digest,
            witness_digest,
            verification_digest: Some(verification.verification_digest.clone()),
            attempted_verification_digests: vec![verification.verification_digest.clone()],
            first_reason: Some(reason),
            prediction_digest: String::new(),
        };
        prediction.seal()
    }

    pub fn unknown_scope_direct_route(
        input: &RouteCapabilityInputV1,
        configuration: &BoundedTrajectoryWitnessConfigurationV1,
    ) -> Result<Self, String> {
        input.validate()?;
        configuration.validate()?;
        if input.physical.topology != RouteTopology::Direct {
            return Err("unknown_scope_direct_route requires a direct route".to_owned());
        }
        Self {
            schema_version: BOUNDED_TRAJECTORY_PREDICTION_SCHEMA_VERSION,
            status: BoundedTrajectoryArtifactStatusV1::Complete,
            decision: Some(BoundedTrajectoryDecisionV1::Unknown),
            input_digest: input.physical_digest()?,
            configuration_digest: configuration.configuration_digest.clone(),
            proposal_configuration_digest: None,
            witness_digest: None,
            verification_digest: None,
            attempted_verification_digests: Vec::new(),
            first_reason: Some(BOUNDED_REASON_UNKNOWN_SCOPE_DIRECT_ROUTE.to_owned()),
            prediction_digest: String::new(),
        }
        .seal()
    }

    pub fn seal(mut self) -> Result<Self, String> {
        self.validate_without_digest()?;
        self.prediction_digest = self.compute_digest()?;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_without_digest()?;
        let expected = self.compute_digest()?;
        if self.prediction_digest != expected {
            return Err(format!(
                "prediction_digest does not match canonical prediction: expected {expected}"
            ));
        }
        Ok(())
    }

    /// Validate a loaded prediction and its exact verification join.  The
    /// witness is required so a caller cannot make a recomputed prediction
    /// trust an unverified serialized verification payload.
    pub fn validate_against_exact(
        &self,
        input: &RouteCapabilityInputV1,
        configuration: &BoundedTrajectoryWitnessConfigurationV1,
        witness: &BoundedTrajectoryWitnessV1,
        verification: &BoundedTrajectoryVerificationV1,
    ) -> Result<(), String> {
        input.validate()?;
        configuration.validate()?;
        let input_digest = input.physical_digest()?;
        witness.validate_against(&input_digest, configuration)?;
        verification.validate_against_exact(input, configuration, witness)?;
        self.validate()?;
        let expected = Self::from_verification(
            input,
            configuration,
            verification,
            self.proposal_configuration_digest.clone(),
        )?;
        if &expected != self {
            return Err("prediction does not match exact verification and joins".to_owned());
        }
        Ok(())
    }

    fn compute_digest(&self) -> Result<String, String> {
        canonical_digest(&PredictionDigestMaterial {
            schema_version: self.schema_version,
            status: self.status,
            decision: self.decision,
            input_digest: &self.input_digest,
            configuration_digest: &self.configuration_digest,
            proposal_configuration_digest: &self.proposal_configuration_digest,
            witness_digest: &self.witness_digest,
            verification_digest: &self.verification_digest,
            attempted_verification_digests: &self.attempted_verification_digests,
            first_reason: &self.first_reason,
        })
    }

    fn validate_without_digest(&self) -> Result<(), String> {
        if self.schema_version != BOUNDED_TRAJECTORY_PREDICTION_SCHEMA_VERSION {
            return Err("prediction schema_version is invalid".to_owned());
        }
        if self.input_digest.trim().is_empty() || self.configuration_digest.trim().is_empty() {
            return Err("prediction input/configuration digests must not be empty".to_owned());
        }
        let reason = self
            .first_reason
            .as_deref()
            .ok_or_else(|| "prediction requires a first reason".to_owned())?;
        validate_bounded_trajectory_reason_v1(reason)?;
        if self
            .proposal_configuration_digest
            .as_deref()
            .is_some_and(|digest| digest.trim().is_empty())
        {
            return Err("prediction proposal configuration digest must not be empty".to_owned());
        }
        if self
            .witness_digest
            .as_deref()
            .is_some_and(|digest| digest.trim().is_empty())
        {
            return Err("prediction witness digest must not be empty".to_owned());
        }
        if self
            .verification_digest
            .as_deref()
            .is_some_and(|digest| digest.trim().is_empty())
        {
            return Err("prediction verification digest must not be empty".to_owned());
        }
        if self
            .attempted_verification_digests
            .iter()
            .any(|digest| digest.trim().is_empty())
        {
            return Err("prediction attempted verification digests must not be empty".to_owned());
        }
        match self.status {
            BoundedTrajectoryArtifactStatusV1::Invalid => {
                if self.decision.is_some() {
                    return Err("invalid prediction cannot carry a decision".to_owned());
                }
                if !reason.starts_with("invalid/") {
                    return Err("invalid prediction requires invalid/* reason".to_owned());
                }
            }
            BoundedTrajectoryArtifactStatusV1::Complete => {
                let decision = self
                    .decision
                    .ok_or_else(|| "complete prediction requires a decision".to_owned())?;
                match decision {
                    BoundedTrajectoryDecisionV1::Supported => {
                        if reason != BOUNDED_REASON_SUPPORTED_EXACT_WITNESS_VERIFIED
                            || self.witness_digest.is_none()
                            || self.verification_digest.is_none()
                        {
                            return Err(
                                "supported prediction is missing verified witness identity"
                                    .to_owned(),
                            );
                        }
                        if !self
                            .attempted_verification_digests
                            .iter()
                            .any(|digest| Some(digest) == self.verification_digest.as_ref())
                        {
                            return Err(
                                "supported prediction must retain its verified attempt identity"
                                    .to_owned(),
                            );
                        }
                    }
                    BoundedTrajectoryDecisionV1::Unknown => {
                        if !reason.starts_with("unknown/") {
                            return Err("unknown prediction requires unknown/* reason".to_owned());
                        }
                        if self.witness_digest.is_some() {
                            return Err(
                                "unknown prediction cannot carry a supported witness".to_owned()
                            );
                        }
                        if (reason == BOUNDED_REASON_UNKNOWN_COVERAGE_BOUNDED_SEARCH_EXHAUSTED
                            || reason == BOUNDED_REASON_UNKNOWN_NUMERICAL_PROPOSAL_NONCONVERGED
                            || reason
                                == BOUNDED_REASON_UNKNOWN_NUMERICAL_PROPOSAL_RECONSTRUCTION_REJECTED)
                            && self.proposal_configuration_digest.is_none()
                        {
                            return Err(
                                "proposal-bounded unknown prediction requires proposal identity"
                                    .to_owned(),
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ReplayStateRecord {
    pub state: BoundedTrajectoryStateV1,
}

#[derive(Clone, Debug)]
struct ReplayTrace {
    samples: Vec<SampleRecord>,
    states: Vec<ReplayStateRecord>,
    installations: Vec<BoundedTrajectoryCommandInstallationV1>,
    source: Option<SourceTransitionKernelOutput>,
    route: Option<crate::RouteExecutionKernelOutput>,
    terminal_state: BoundedTrajectoryStateV1,
    extrema: BoundedTrajectoryExtremaV1,
    rejection: Option<String>,
    trajectory_digest: String,
    phases: Vec<BoundedTrajectoryPhaseResultV1>,
    boundaries: Vec<BoundedTrajectoryBoundaryResultV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ReplayDigestMaterial<'a> {
    samples: &'a [SampleRecord],
    states: &'a [ReplayStateRecord],
    installations: &'a [BoundedTrajectoryCommandInstallationV1],
    phases: &'a [BoundedTrajectoryPhaseResultV1],
    boundaries: &'a [BoundedTrajectoryBoundaryResultV1],
    rejection: &'a Option<String>,
}

fn replay_trajectory_digest(
    samples: &[SampleRecord],
    states: &[ReplayStateRecord],
    installations: &[BoundedTrajectoryCommandInstallationV1],
    phases: &[BoundedTrajectoryPhaseResultV1],
    boundaries: &[BoundedTrajectoryBoundaryResultV1],
    rejection: &Option<String>,
) -> Result<String, String> {
    canonical_digest(&ReplayDigestMaterial {
        samples,
        states,
        installations,
        phases,
        boundaries,
        rejection,
    })
}

#[derive(Clone, Debug, PartialEq)]
struct ReplayFingerprint {
    timeline_digest: String,
    source_physical_digest: Option<String>,
    route_physical_digest: Option<String>,
    trajectory_digest: String,
    terminal_state: BoundedTrajectoryStateV1,
    extrema: BoundedTrajectoryExtremaV1,
    phases: Vec<BoundedTrajectoryPhaseResultV1>,
    boundaries: Vec<BoundedTrajectoryBoundaryResultV1>,
    rejection: Option<String>,
}

impl ReplayTrace {
    fn fingerprint(&self) -> Result<ReplayFingerprint, String> {
        Ok(ReplayFingerprint {
            timeline_digest: canonical_digest(&(&self.samples, &self.states, &self.installations))?,
            source_physical_digest: self
                .source
                .as_ref()
                .map(|source| source.physical_digest.clone()),
            route_physical_digest: self
                .route
                .as_ref()
                .map(|route| route.physical_digest.clone()),
            trajectory_digest: self.trajectory_digest.clone(),
            terminal_state: self.terminal_state.clone(),
            extrema: self.extrema,
            phases: self.phases.clone(),
            boundaries: self.boundaries.clone(),
            rejection: self.rejection.clone(),
        })
    }
}

/// Replay one sealed command witness against the exact discrete plant and
/// neutral D0 boundary kernels.
pub fn verify_bounded_trajectory(
    input: &RouteCapabilityInputV1,
    configuration: &BoundedTrajectoryWitnessConfigurationV1,
    witness: &BoundedTrajectoryWitnessV1,
) -> BoundedTrajectoryVerificationV1 {
    let configuration_digest = configuration.configuration_digest.clone();
    let witness_digest = witness.witness_digest.clone();

    let input_digest = match input.physical_digest() {
        Ok(digest) => digest,
        Err(error) => {
            return BoundedTrajectoryVerificationV1::new_invalid(
                String::new(),
                configuration_digest,
                witness_digest,
                invalid_reason("invalid/input/", &error),
            );
        }
    };

    if let Err(error) = configuration.validate() {
        return BoundedTrajectoryVerificationV1::new_invalid(
            input_digest,
            configuration_digest,
            witness_digest,
            invalid_reason("invalid/artifact/", &error),
        );
    }
    if let Err(error) = input.validate() {
        return BoundedTrajectoryVerificationV1::new_invalid(
            input_digest,
            configuration.configuration_digest.clone(),
            witness_digest,
            invalid_reason("invalid/input/", &error),
        );
    }
    if let Err(error) = witness.validate_against(&input_digest, configuration) {
        return BoundedTrajectoryVerificationV1::new_invalid(
            input_digest,
            configuration.configuration_digest.clone(),
            witness_digest,
            invalid_reason("invalid/artifact/", &error),
        );
    }

    let first = match replay_once(input, configuration, witness) {
        Ok(trace) => trace,
        Err(error) => {
            return BoundedTrajectoryVerificationV1::new_invalid(
                input_digest,
                configuration.configuration_digest.clone(),
                witness.witness_digest.clone(),
                invalid_reason("invalid/numerical/", &error),
            );
        }
    };
    let second = match replay_once(input, configuration, witness) {
        Ok(trace) => trace,
        Err(error) => {
            return BoundedTrajectoryVerificationV1::new_invalid(
                input_digest,
                configuration.configuration_digest.clone(),
                witness.witness_digest.clone(),
                invalid_reason("invalid/determinism/", &error),
            );
        }
    };
    let first_fingerprint = match first.fingerprint() {
        Ok(fingerprint) => fingerprint,
        Err(error) => {
            return BoundedTrajectoryVerificationV1::new_invalid(
                input_digest,
                configuration.configuration_digest.clone(),
                witness.witness_digest.clone(),
                invalid_reason("invalid/numerical/", &error),
            );
        }
    };
    let second_fingerprint = match second.fingerprint() {
        Ok(fingerprint) => fingerprint,
        Err(error) => {
            return BoundedTrajectoryVerificationV1::new_invalid(
                input_digest,
                configuration.configuration_digest.clone(),
                witness.witness_digest.clone(),
                invalid_reason("invalid/determinism/", &error),
            );
        }
    };
    if first_fingerprint != second_fingerprint {
        return BoundedTrajectoryVerificationV1::new_invalid(
            input_digest,
            configuration.configuration_digest.clone(),
            witness.witness_digest.clone(),
            "invalid/determinism/repeated_replay_disagrees".to_owned(),
        );
    }

    let (verdict, reason) = match first.rejection.as_deref() {
        Some(reason) => (
            BoundedTrajectoryVerificationVerdictV1::Rejected,
            reason.to_owned(),
        ),
        None => (
            BoundedTrajectoryVerificationVerdictV1::Verified,
            BOUNDED_REASON_SUPPORTED_EXACT_WITNESS_VERIFIED.to_owned(),
        ),
    };
    match BoundedTrajectoryVerificationV1::new_complete(VerificationPayload {
        input_digest,
        configuration_digest: configuration.configuration_digest.clone(),
        witness_digest: witness.witness_digest.clone(),
        verdict,
        terminal_physics_step: first.terminal_state.physics_step,
        terminal_state: first.terminal_state,
        extrema: first.extrema,
        phases: first.phases,
        boundaries: first.boundaries,
        trajectory_digest: first.trajectory_digest,
        reason,
    }) {
        Ok(value) => value,
        Err(error) => BoundedTrajectoryVerificationV1::new_invalid(
            witness.input_digest.clone(),
            configuration.configuration_digest.clone(),
            witness.witness_digest.clone(),
            invalid_reason("invalid/numerical/", &error),
        ),
    }
}

fn replay_once(
    input: &RouteCapabilityInputV1,
    configuration: &BoundedTrajectoryWitnessConfigurationV1,
    witness: &BoundedTrajectoryWitnessV1,
) -> Result<ReplayTrace, String> {
    let adapter = NeutralAdapter::from_input(input, configuration)?;
    let mut state = SimulationState::new(&adapter.context)
        .map_err(|error| format!("failed to initialize neutral plant: {error}"))?;
    let mut samples = vec![sample_from_state(&state, &adapter.context, Command::idle())];
    let mut states = vec![ReplayStateRecord {
        state: proof_state(
            &state,
            &adapter.context,
            None,
            BoundedTrajectoryContactClassificationV1::None,
        ),
    }];
    let mut installations = Vec::with_capacity(witness.commands.len());
    let mut current_slot = None;
    let mut rejection = None;

    for physics_step in 1..=witness.terminal_physics_step {
        let slot = (physics_step - 1) / u64::from(configuration.hold_steps);
        let command = witness
            .commands
            .get(slot as usize)
            .ok_or_else(|| format!("missing command slot {slot}"))?;
        if current_slot != Some(slot) {
            let expected_install_step = slot * u64::from(configuration.hold_steps);
            if state.physics_step != expected_install_step {
                return Err(format!(
                    "command slot {slot} installed at invalid step {}",
                    state.physics_step
                ));
            }
            installations.push(BoundedTrajectoryCommandInstallationV1 {
                physics_step: state.physics_step,
                command: *command,
            });
            state.set_command(command.core_command());
            current_slot = Some(slot);
        }

        let contact = state.step_physics_and_classify_contact(&adapter.context);
        validate_replay_state(&state, &adapter.context)?;
        samples.push(sample_from_state(
            &state,
            &adapter.context,
            command.core_command(),
        ));
        states.push(ReplayStateRecord {
            state: proof_state(
                &state,
                &adapter.context,
                Some(command),
                contact.clone().into(),
            ),
        });
        if !matches!(contact, ContactClassification::None) {
            rejection = Some(format!("rejected/contact/step_{}", state.physics_step));
            break;
        }
    }

    let terminal_state = states
        .last()
        .map(|record| record.state.clone())
        .ok_or_else(|| "replay produced no terminal state".to_owned())?;
    let extrema = BoundedTrajectoryExtremaV1 {
        min_touchdown_clearance_m: state.min_touchdown_clearance_m,
        min_hull_clearance_m: state.min_hull_clearance_m,
        max_speed_mps: state.max_speed_mps,
        max_abs_attitude_rad: state.max_abs_attitude_rad,
        max_abs_angular_rate_radps: state.max_abs_angular_rate_radps,
    };

    let (source, route, kernel_rejection) = if rejection.is_some() {
        (None, None, None)
    } else if input.physical.topology == RouteTopology::Direct {
        (None, None, Some("rejected/scope/direct_route".to_owned()))
    } else if !(1..=2).contains(&input.physical.waypoints.len()) {
        (None, None, Some("rejected/scope/waypoint_count".to_owned()))
    } else {
        let source_input = SourceTransitionKernelInput {
            terrain: &adapter.terrain,
            source_pad_center_x_m: adapter.source_pad.center_x_m,
            vehicle: &adapter.vehicle,
            initial_state: &adapter.initial_state,
            geometry: &adapter.geometry,
            profile: &adapter.profile,
            selected_centerline_m: &adapter.normalized_centerline,
            waypoints: &adapter.waypoints,
            samples: &samples,
            cadence: SourceTransitionCadence::physics_rate(configuration.physics_hz),
            audit: crate::SourceTransitionAuditInput::default(),
        };
        let source = extract_source_transition_kernel(&source_input)
            .map_err(|error| format!("source kernel failed: {error}"))?;
        let route_input = RouteExecutionKernelInput {
            terrain: &adapter.terrain,
            source_pad_center_x_m: adapter.source_pad.center_x_m,
            source_pad_surface_y_m: adapter.source_pad.surface_y_m,
            target_pad_center_x_m: adapter.target_pad.center_x_m,
            target_pad_surface_y_m: adapter.target_pad.surface_y_m,
            vehicle: &adapter.vehicle,
            initial_state: &adapter.initial_state,
            geometry: &adapter.geometry,
            profile: &adapter.profile,
            selected_centerline_m: &adapter.normalized_centerline,
            waypoints: &adapter.waypoints,
            samples: &samples,
            cadence: SourceTransitionCadence::physics_rate(configuration.physics_hz),
        };
        let route = extract_route_execution_kernel(&route_input)
            .map_err(|error| format!("route kernel failed: {error}"))?;
        let rejection = route_rejection(
            &source,
            &route,
            adapter.waypoints.len(),
            witness.terminal_physics_step,
        );
        (Some(source), Some(route), rejection)
    };
    if rejection.is_none() && kernel_rejection.is_some() {
        rejection = kernel_rejection;
    }
    let phases = build_phase_results(
        input.physical.waypoints.len(),
        source.as_ref(),
        route.as_ref(),
    );
    let boundaries = build_boundary_results(source.as_ref(), route.as_ref());
    let trajectory_digest = replay_trajectory_digest(
        &samples,
        &states,
        &installations,
        &phases,
        &boundaries,
        &rejection,
    )?;

    Ok(ReplayTrace {
        samples,
        states,
        installations,
        source,
        route,
        terminal_state,
        extrema,
        rejection,
        trajectory_digest,
        phases,
        boundaries,
    })
}

struct NeutralAdapter {
    context: RunContext,
    terrain: TerrainDefinition,
    source_pad: LandingPadSpec,
    target_pad: LandingPadSpec,
    vehicle: VehicleSpec,
    initial_state: VehicleInitialState,
    geometry: pd_core::NormalizedRouteGeometry,
    profile: SafetyProfile,
    normalized_centerline: Vec<Vec2>,
    waypoints: Vec<TransferWaypointSpec>,
}

impl NeutralAdapter {
    fn from_input(
        input: &RouteCapabilityInputV1,
        configuration: &BoundedTrajectoryWitnessConfigurationV1,
    ) -> Result<Self, String> {
        let physical = &input.physical;
        let source_pad = LandingPadSpec {
            id: "bounded_source".to_owned(),
            center_x_m: physical.source_pad.center_x_m,
            surface_y_m: physical.source_pad.surface_y_m,
            width_m: physical.source_pad.width_m,
        };
        let target_pad = LandingPadSpec {
            id: "bounded_target".to_owned(),
            center_x_m: physical.target_pad.center_x_m,
            surface_y_m: physical.target_pad.surface_y_m,
            width_m: physical.target_pad.width_m,
        };
        let waypoints = physical
            .waypoints
            .iter()
            .enumerate()
            .map(|(index, waypoint)| TransferWaypointSpec {
                id: format!("bounded_waypoint_{index}"),
                position_m: waypoint.position_m,
                handoff_tangent_unit: waypoint.handoff_tangent_unit,
                capture_radius_m: waypoint.capture_radius_m,
                max_cross_track_m: waypoint.max_cross_track_m,
                max_outbound_heading_error_rad: waypoint.max_outbound_heading_error_rad,
                min_outbound_progress_mps: waypoint.min_outbound_progress_mps,
                max_outbound_cross_speed_mps: waypoint.max_outbound_cross_speed_mps,
                min_speed_mps: waypoint.min_speed_mps,
                max_speed_mps: waypoint.max_speed_mps,
                min_vertical_speed_mps: waypoint.min_vertical_speed_mps,
                max_vertical_speed_mps: waypoint.max_vertical_speed_mps,
            })
            .collect::<Vec<_>>();
        let route = TransferRouteSpec {
            source_pad_id: source_pad.id.clone(),
            target_pad_id: target_pad.id.clone(),
            route_angle_deg: physical.route_angle_deg,
            route_radius_m: physical.route_radius_m,
            waypoints: waypoints.clone(),
        };
        let goal = if physical.topology == RouteTopology::Direct {
            EvaluationGoal::LandingOnPad {
                target_pad_id: target_pad.id.clone(),
            }
        } else {
            EvaluationGoal::WaypointSequence {
                target_pad_id: target_pad.id.clone(),
            }
        };
        let scenario = ScenarioSpec {
            id: "bounded_trajectory_neutral".to_owned(),
            name: "bounded trajectory neutral verifier".to_owned(),
            description: "synthetic adapter context; not a proof input".to_owned(),
            seed: 0,
            tags: Vec::new(),
            metadata: std::collections::BTreeMap::new(),
            sim: SimConfig {
                physics_hz: configuration.physics_hz,
                controller_hz: configuration.command_hz,
                max_time_s: configuration.max_time_s,
                sample_hz: Some(configuration.physics_hz),
            },
            world: WorldSpec {
                gravity_mps2: physical.gravity_mps2,
                terrain: physical.terrain.clone(),
                landing_pads: vec![source_pad.clone(), target_pad.clone()],
            },
            vehicle: physical.vehicle.clone(),
            initial_state: physical.initial_state.clone(),
            mission: MissionSpec {
                transfer_route: Some(route),
                goal,
            },
        };
        let context = RunContext::from_scenario(&scenario)?;
        let profile = SafetyProfile {
            source_transition_start_m: physical.safety_profile.source_transition_start_m,
            source_transition_end_m: physical.safety_profile.source_transition_end_m,
            target_transition_start_m: physical.safety_profile.target_transition_start_m,
            target_transition_end_m: physical.safety_profile.target_transition_end_m,
            horizontal_span_m: physical.safety_profile.horizontal_span_m,
            full_envelope: physical.safety_profile.full_envelope,
            contact_envelope: physical.safety_profile.contact_envelope,
        };
        let normalized_centerline = physical
            .selected_centerline_m
            .iter()
            .map(|point| {
                Vec2::new(
                    f64::from(physical.horizontal_sign)
                        * (point.x - physical.source_pad.center_x_m),
                    point.y,
                )
            })
            .collect::<Vec<_>>();
        Ok(Self {
            context,
            terrain: physical.terrain.clone(),
            source_pad,
            target_pad,
            vehicle: physical.vehicle.clone(),
            initial_state: physical.initial_state.clone(),
            geometry: physical.normalized_geometry.clone(),
            profile,
            normalized_centerline,
            waypoints,
        })
    }
}

fn sample_from_state(
    state: &SimulationState,
    context: &RunContext,
    command: Command,
) -> SampleRecord {
    SampleRecord {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        observation: state.build_observation(context),
        held_command: command,
    }
}

fn proof_state(
    state: &SimulationState,
    context: &RunContext,
    command: Option<&BoundedTrajectoryCommandV1>,
    contact: BoundedTrajectoryContactClassificationV1,
) -> BoundedTrajectoryStateV1 {
    BoundedTrajectoryStateV1 {
        physics_step: state.physics_step,
        sim_time_s: state.sim_time_s,
        position_m: state.position_m,
        velocity_mps: state.velocity_mps,
        attitude_rad: state.attitude_rad,
        angular_rate_radps: state.angular_rate_radps,
        fuel_kg: state.fuel_kg,
        mass_kg: state.mass_kg(context),
        held_command: command.copied(),
        contact,
    }
}

fn route_rejection(
    source: &SourceTransitionKernelOutput,
    route: &crate::RouteExecutionKernelOutput,
    expected_waypoint_count: usize,
    terminal_physics_step: u64,
) -> Option<String> {
    if source.contact_exit.is_none() {
        return Some("rejected/phase/contact_exit".to_owned());
    }
    if source.tracking_entry.is_none() {
        return Some("rejected/phase/tracking_entry".to_owned());
    }
    if route.waypoints.len() != expected_waypoint_count {
        return Some("rejected/handoff/missing_waypoint_resolution".to_owned());
    }
    for (index, waypoint) in route.waypoints.iter().enumerate() {
        let Some(pass) = waypoint.first_contract_pass.as_ref() else {
            return Some(format!(
                "rejected/handoff/waypoint_{index}_no_contract_pass"
            ));
        };
        if index + 1 == route.waypoints.len() && pass.sample.physics_step < terminal_physics_step {
            return Some("rejected/terminal/nonminimal".to_owned());
        }
    }
    let Some(final_waypoint) = route.waypoints.last() else {
        return Some("rejected/handoff/no_waypoints".to_owned());
    };
    let Some(pass) = final_waypoint.first_contract_pass.as_ref() else {
        return Some("rejected/handoff/final_no_contract_pass".to_owned());
    };
    if pass.sample.physics_step != terminal_physics_step {
        return Some("rejected/terminal/missing_final_handoff".to_owned());
    }
    None
}

fn build_phase_results(
    waypoint_count: usize,
    source: Option<&SourceTransitionKernelOutput>,
    route: Option<&crate::RouteExecutionKernelOutput>,
) -> Vec<BoundedTrajectoryPhaseResultV1> {
    let phases = ordered_route_phases(waypoint_count);
    phases
        .into_iter()
        .map(|phase| {
            let terminal_physics_step = match phase {
                RouteCapabilityPhaseV1::InitialState => Some(0),
                RouteCapabilityPhaseV1::PadDeparture => source
                    .and_then(|source| source.contact_exit.as_ref())
                    .map(|boundary| boundary.after.physics_step),
                RouteCapabilityPhaseV1::Acquisition => source
                    .and_then(|source| source.tracking_entry.as_ref())
                    .map(|boundary| boundary.after.physics_step),
                RouteCapabilityPhaseV1::RouteLeg { leg_index }
                | RouteCapabilityPhaseV1::Handoff {
                    waypoint_index: leg_index,
                } => route
                    .and_then(|route| route.waypoints.get(leg_index))
                    .and_then(|waypoint| waypoint.first_contract_pass.as_ref())
                    .map(|pass| pass.sample.physics_step),
            };
            BoundedTrajectoryPhaseResultV1 {
                phase,
                passed: terminal_physics_step.is_some(),
                terminal_physics_step,
            }
        })
        .collect()
}

fn build_boundary_results(
    source: Option<&SourceTransitionKernelOutput>,
    route: Option<&crate::RouteExecutionKernelOutput>,
) -> Vec<BoundedTrajectoryBoundaryResultV1> {
    let mut boundaries = Vec::new();
    for (name, value) in [
        (
            "contact_exit",
            source.and_then(|source| source.contact_exit.as_ref()),
        ),
        (
            "tracking_entry",
            source.and_then(|source| source.tracking_entry.as_ref()),
        ),
    ] {
        boundaries.push(BoundedTrajectoryBoundaryResultV1 {
            name: name.to_owned(),
            physics_step: value.map(|boundary| boundary.after.physics_step),
            sample_index: value.map(|boundary| boundary.after.sample_index),
            initial_deadline: false,
        });
    }
    if let Some(route) = route {
        for (index, waypoint) in route.waypoints.iter().enumerate() {
            let (physics_step, sample_index, initial_deadline) = waypoint
                .resolution
                .as_ref()
                .map_or((None, None, false), |resolution| {
                    (
                        Some(resolution.physics_step),
                        Some(resolution.sample_index),
                        resolution.kind == RouteExecutionResolutionKind::InitialDeadline,
                    )
                });
            boundaries.push(BoundedTrajectoryBoundaryResultV1 {
                name: format!("handoff_{index}"),
                physics_step,
                sample_index,
                initial_deadline,
            });
        }
    }
    boundaries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ROUTE_CAPABILITY_INPUT_SCHEMA_VERSION, RouteCapabilityInputProvenanceV1,
        RouteCapabilityPadGeometryV1, RouteCapabilityPhysicalInputV1,
        RouteCapabilityPhysicalPolicyV1, RouteCapabilitySafetyProfileV1, RouteCapabilityWaypointV1,
    };
    use pd_core::{
        CorridorEnvelope, NormalizedRouteGeometry, VehicleGeometry, replay_simulation,
        run_simulation,
    };

    fn test_input() -> RouteCapabilityInputV1 {
        let physical = RouteCapabilityPhysicalInputV1 {
            gravity_mps2: 1.0,
            terrain: TerrainDefinition::Heightfield {
                points_m: vec![Vec2::new(-20.0, 0.0), Vec2::new(120.0, 0.0)],
            },
            source_pad: RouteCapabilityPadGeometryV1 {
                center_x_m: 0.0,
                surface_y_m: 0.0,
                width_m: 30.0,
            },
            target_pad: RouteCapabilityPadGeometryV1 {
                center_x_m: 100.0,
                surface_y_m: 0.0,
                width_m: 30.0,
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 4.0,
                    hull_height_m: 6.0,
                    touchdown_half_span_m: 2.0,
                    touchdown_base_offset_m: 3.0,
                },
                dry_mass_kg: 700.0,
                initial_fuel_kg: 200.0,
                max_fuel_kg: 200.0,
                max_thrust_n: 900.0,
                max_fuel_burn_kgps: 1.0e-9,
                min_throttle_frac: 0.25,
                max_rotation_rate_radps: 1.0,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(0.0, 4.0),
                velocity_mps: Vec2::new(5.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            policy: RouteCapabilityPhysicalPolicyV1 {
                max_waypoints: 2,
                flight_clearance_margin_m: 1.0,
                endpoint_transition_m: 17.0,
                max_extra_loft_ratio: 1.0,
                max_continuation_ratio: 1.0,
                max_handoff_speed_mps: 100.0,
                min_handoff_speed_mps: 0.1,
                min_outbound_progress_mps: 0.1,
                max_outbound_heading_error_rad: PI,
                max_outbound_cross_speed_mps: 100.0,
            },
            safety_profile: RouteCapabilitySafetyProfileV1 {
                source_transition_start_m: 17.0,
                source_transition_end_m: 34.0,
                target_transition_start_m: 66.0,
                target_transition_end_m: 83.0,
                horizontal_span_m: 100.0,
                full_envelope: CorridorEnvelope::new(5.0, 3.0),
                contact_envelope: CorridorEnvelope::new(2.0, 3.0),
            },
            selected_centerline_m: vec![
                Vec2::new(0.0, 4.0),
                Vec2::new(34.0, 4.0),
                Vec2::new(60.0, 4.0),
                Vec2::new(83.0, 4.0),
                Vec2::new(100.0, 4.0),
            ],
            normalized_geometry: NormalizedRouteGeometry {
                horizontal_sign: 1,
                direct_horizontal_span_m: 100.0,
                direct_distance_m: 100.0,
                route_angle_rad: 0.0,
                route_angle_deg: 0.0,
            },
            horizontal_sign: 1,
            waypoints: vec![RouteCapabilityWaypointV1 {
                position_m: Vec2::new(60.0, 4.0),
                handoff_tangent_unit: Some(Vec2::new(1.0, 0.0)),
                capture_radius_m: 20.001,
                max_cross_track_m: 20.0,
                max_outbound_heading_error_rad: PI,
                min_outbound_progress_mps: 0.1,
                max_outbound_cross_speed_mps: Some(100.0),
                min_speed_mps: 0.1,
                max_speed_mps: 100.0,
                min_vertical_speed_mps: Some(-100.0),
                max_vertical_speed_mps: Some(100.0),
            }],
            topology: RouteTopology::Waypoint,
            route_angle_deg: 0.0,
            route_radius_m: 100.0,
        };
        RouteCapabilityInputV1 {
            schema_version: ROUTE_CAPABILITY_INPUT_SCHEMA_VERSION,
            physical,
            provenance: RouteCapabilityInputProvenanceV1 {
                request_digest: "request-test".to_owned(),
                route_plan_digest: "plan-test".to_owned(),
                source_pad_id: "source".to_owned(),
                target_pad_id: "target".to_owned(),
                waypoint_ids: vec!["waypoint".to_owned()],
            },
            input_digest: String::new(),
        }
        .seal()
        .expect("valid synthetic physical input")
    }

    fn witness_for(
        input: &RouteCapabilityInputV1,
        terminal_step: u64,
    ) -> BoundedTrajectoryWitnessV1 {
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("valid witness configuration");
        let commands = (0..terminal_step.div_ceil(u64::from(configuration.hold_steps)))
            .map(|slot| BoundedTrajectoryCommandV1 {
                slot,
                throttle_frac: 1.0,
                target_attitude_rad: 0.0,
            })
            .collect();
        BoundedTrajectoryWitnessV1::new(
            input.physical_digest().expect("input digest"),
            configuration.configuration_digest.clone(),
            commands,
            terminal_step,
        )
        .expect("valid synthetic witness")
    }

    fn two_waypoint_input() -> RouteCapabilityInputV1 {
        let mut input = test_input();
        let template = input.physical.waypoints[0];
        input.physical.waypoints = vec![
            RouteCapabilityWaypointV1 {
                position_m: Vec2::new(50.0, 4.0),
                capture_radius_m: 10.001,
                ..template
            },
            RouteCapabilityWaypointV1 {
                position_m: Vec2::new(75.0, 4.0),
                capture_radius_m: 10.001,
                ..template
            },
        ];
        input.physical.selected_centerline_m = vec![
            Vec2::new(0.0, 4.0),
            Vec2::new(34.0, 4.0),
            Vec2::new(50.0, 4.0),
            Vec2::new(75.0, 4.0),
            Vec2::new(83.0, 4.0),
            Vec2::new(100.0, 4.0),
        ];
        input.provenance.waypoint_ids = vec!["waypoint-0".to_owned(), "waypoint-1".to_owned()];
        input.seal().expect("two-waypoint input seals")
    }

    fn odd_terminal_input() -> RouteCapabilityInputV1 {
        let mut input = test_input();
        input.physical.waypoints[0].capture_radius_m = 20.05;
        input.seal().expect("odd-terminal input seals")
    }

    fn contact_input() -> RouteCapabilityInputV1 {
        let mut input = test_input();
        input.physical.initial_state.position_m.y = 3.0;
        input.seal().expect("contact input seals")
    }

    fn deadline_input() -> RouteCapabilityInputV1 {
        let mut input = test_input();
        input.physical.initial_state.velocity_mps.x = 5.0;
        input.physical.waypoints[0].capture_radius_m = 1.0;
        input.physical.waypoints[0].max_cross_track_m = 1.0;
        input.physical.waypoints[0].min_outbound_progress_mps = 6.0;
        input.seal().expect("deadline input seals")
    }

    fn initial_deadline_input() -> RouteCapabilityInputV1 {
        let mut input = test_input();
        input.physical.initial_state.velocity_mps.x = 100.0;
        input.physical.waypoints[0].position_m = Vec2::new(34.1, 4.0);
        input.physical.waypoints[0].capture_radius_m = 1.0;
        input.physical.waypoints[0].max_cross_track_m = 1.0;
        input.physical.selected_centerline_m = vec![
            Vec2::new(0.0, 4.0),
            Vec2::new(34.0, 4.0),
            Vec2::new(34.1, 4.0),
            Vec2::new(83.0, 4.0),
            Vec2::new(100.0, 4.0),
        ];
        input.seal().expect("initial-deadline input seals")
    }

    fn horizon_input() -> RouteCapabilityInputV1 {
        let mut input = test_input();
        input.physical.initial_state.velocity_mps.x = 0.5;
        input.physical.waypoints[0].position_m = Vec2::new(65.0000000005, 4.0);
        input.physical.waypoints[0].capture_radius_m = 1.0e-9;
        input.physical.waypoints[0].max_cross_track_m = 1.0;
        input.physical.selected_centerline_m = vec![
            Vec2::new(0.0, 4.0),
            Vec2::new(34.0, 4.0),
            Vec2::new(65.0000000005, 4.0),
            Vec2::new(83.0, 4.0),
            Vec2::new(100.0, 4.0),
        ];
        input.seal().expect("horizon input seals")
    }

    fn sticky_window_input() -> RouteCapabilityInputV1 {
        let mut input = test_input();
        input.physical.terrain = TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(-500.0, 0.0), Vec2::new(600.0, 0.0)],
        };
        input.physical.target_pad.center_x_m = 200.0;
        input.physical.source_pad.surface_y_m = 400.0;
        input.physical.target_pad.surface_y_m = 400.0;
        input.physical.safety_profile.target_transition_start_m = 166.0;
        input.physical.safety_profile.target_transition_end_m = 183.0;
        input.physical.safety_profile.horizontal_span_m = 200.0;
        input.physical.normalized_geometry.direct_horizontal_span_m = 200.0;
        input.physical.normalized_geometry.direct_distance_m = 200.0;
        input.physical.selected_centerline_m = vec![
            Vec2::new(0.0, 1_000.0),
            Vec2::new(34.0, 1_000.0),
            Vec2::new(150.0, 1_000.0),
            Vec2::new(166.0, 1_000.0),
            Vec2::new(183.0, 1_000.0),
            Vec2::new(200.0, 1_000.0),
        ];
        input.physical.initial_state.position_m.y = 1_000.0;
        input.physical.initial_state.velocity_mps.x = 2.0;
        input.physical.vehicle.max_thrust_n = 900.0;
        input.physical.source_pad.surface_y_m = 1_000.0;
        input.physical.target_pad.surface_y_m = 1_000.0;
        input.physical.waypoints[0].position_m = Vec2::new(150.0, 1_000.0);
        input.physical.waypoints[0].capture_radius_m = 20.0;
        input.physical.waypoints[0].max_cross_track_m = 20.0;
        input.physical.waypoints[0].min_outbound_progress_mps = 7.0;
        input.seal().expect("sticky-window input seals")
    }

    fn out_of_scope_three_waypoint_input() -> RouteCapabilityInputV1 {
        let mut input = two_waypoint_input();
        let template = input.physical.waypoints[0];
        input.physical.waypoints.push(RouteCapabilityWaypointV1 {
            position_m: Vec2::new(90.0, 4.0),
            capture_radius_m: 10.001,
            ..template
        });
        input.physical.selected_centerline_m = vec![
            Vec2::new(0.0, 4.0),
            Vec2::new(34.0, 4.0),
            Vec2::new(50.0, 4.0),
            Vec2::new(75.0, 4.0),
            Vec2::new(90.0, 4.0),
            Vec2::new(100.0, 4.0),
        ];
        input.provenance.waypoint_ids = vec![
            "waypoint-0".to_owned(),
            "waypoint-1".to_owned(),
            "waypoint-2".to_owned(),
        ];
        input.seal().expect("three-waypoint input seals")
    }

    fn assert_invalid_verification(
        input: &RouteCapabilityInputV1,
        configuration: &BoundedTrajectoryWitnessConfigurationV1,
        witness: &BoundedTrajectoryWitnessV1,
    ) -> BoundedTrajectoryVerificationV1 {
        let result = verify_bounded_trajectory(input, configuration, witness);
        assert_eq!(result.status, BoundedTrajectoryArtifactStatusV1::Invalid);
        assert_eq!(result.verdict, None);
        result.validate().expect("invalid result remains canonical");
        result
    }

    fn direct_d0_kernels(
        input: &RouteCapabilityInputV1,
        configuration: &BoundedTrajectoryWitnessConfigurationV1,
        trace: &ReplayTrace,
    ) -> (
        SourceTransitionKernelOutput,
        crate::RouteExecutionKernelOutput,
    ) {
        let adapter = NeutralAdapter::from_input(input, configuration).expect("adapter");
        let source_input = SourceTransitionKernelInput {
            terrain: &adapter.terrain,
            source_pad_center_x_m: adapter.source_pad.center_x_m,
            vehicle: &adapter.vehicle,
            initial_state: &adapter.initial_state,
            geometry: &adapter.geometry,
            profile: &adapter.profile,
            selected_centerline_m: &adapter.normalized_centerline,
            waypoints: &adapter.waypoints,
            samples: &trace.samples,
            cadence: SourceTransitionCadence::physics_rate(configuration.physics_hz),
            audit: crate::SourceTransitionAuditInput::default(),
        };
        let source = extract_source_transition_kernel(&source_input).expect("source kernel");
        let route_input = RouteExecutionKernelInput {
            terrain: &adapter.terrain,
            source_pad_center_x_m: adapter.source_pad.center_x_m,
            source_pad_surface_y_m: adapter.source_pad.surface_y_m,
            target_pad_center_x_m: adapter.target_pad.center_x_m,
            target_pad_surface_y_m: adapter.target_pad.surface_y_m,
            vehicle: &adapter.vehicle,
            initial_state: &adapter.initial_state,
            geometry: &adapter.geometry,
            profile: &adapter.profile,
            selected_centerline_m: &adapter.normalized_centerline,
            waypoints: &adapter.waypoints,
            samples: &trace.samples,
            cadence: SourceTransitionCadence::physics_rate(configuration.physics_hz),
        };
        let route = extract_route_execution_kernel(&route_input).expect("route kernel");
        (source, route)
    }

    #[test]
    fn configuration_and_witness_round_trip_and_tamper_detection() {
        let input = test_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        assert_eq!(configuration.physics_hz, 120);
        assert_eq!(configuration.command_hz, 60);
        assert_eq!(configuration.hold_steps, 2);
        assert_eq!(configuration.max_physics_steps, 15_600);
        assert_eq!(configuration.max_command_slots, 7_800);

        let witness = witness_for(&input, 3);
        let bytes = serde_json::to_vec(&witness).expect("witness json");
        let round_trip: BoundedTrajectoryWitnessV1 =
            serde_json::from_slice(&bytes).expect("witness round trip");
        assert_eq!(round_trip, witness);
        round_trip
            .validate_against(
                &input.physical_digest().expect("input digest"),
                &configuration,
            )
            .expect("round-tripped witness validates");

        let mut tampered = witness.clone();
        tampered.commands[0].throttle_frac = 0.5;
        assert!(
            tampered
                .validate_against(
                    &input.physical_digest().expect("input digest"),
                    &configuration,
                )
                .is_err()
        );
    }

    #[test]
    fn command_validation_rejects_noncanonical_values() {
        let invalid = [
            BoundedTrajectoryCommandV1 {
                slot: 0,
                throttle_frac: -0.1,
                target_attitude_rad: 0.0,
            },
            BoundedTrajectoryCommandV1 {
                slot: 0,
                throttle_frac: 1.1,
                target_attitude_rad: 0.0,
            },
            BoundedTrajectoryCommandV1 {
                slot: 0,
                throttle_frac: f64::NAN,
                target_attitude_rad: 0.0,
            },
            BoundedTrajectoryCommandV1 {
                slot: 0,
                throttle_frac: 0.0,
                target_attitude_rad: f64::NAN,
            },
            BoundedTrajectoryCommandV1 {
                slot: 0,
                throttle_frac: -0.0,
                target_attitude_rad: 0.0,
            },
            BoundedTrajectoryCommandV1 {
                slot: 0,
                throttle_frac: 0.0,
                target_attitude_rad: -0.0,
            },
            BoundedTrajectoryCommandV1 {
                slot: 0,
                throttle_frac: 0.0,
                target_attitude_rad: PI * 2.0,
            },
        ];
        for command in invalid {
            assert!(
                command.validate().is_err(),
                "command unexpectedly valid: {command:?}"
            );
        }
        assert!(
            BoundedTrajectoryCommandV1 {
                slot: 0,
                throttle_frac: 0.0,
                target_attitude_rad: -PI,
            }
            .validate()
            .is_ok()
        );
        assert!(
            BoundedTrajectoryCommandV1 {
                slot: 0,
                throttle_frac: 0.0,
                target_attitude_rad: PI,
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn configuration_and_reason_validation_fail_closed_to_locked_v1() {
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let mutations: [fn(&mut BoundedTrajectoryWitnessConfigurationV1); 7] = [
            |value: &mut BoundedTrajectoryWitnessConfigurationV1| {
                value.physics_hz = 60;
            },
            |value: &mut BoundedTrajectoryWitnessConfigurationV1| {
                value.command_hz = 40;
            },
            |value: &mut BoundedTrajectoryWitnessConfigurationV1| {
                value.hold_steps = 1;
            },
            |value: &mut BoundedTrajectoryWitnessConfigurationV1| {
                value.max_time_s = 129.0;
            },
            |value: &mut BoundedTrajectoryWitnessConfigurationV1| {
                value.max_physics_steps = 15_480;
            },
            |value: &mut BoundedTrajectoryWitnessConfigurationV1| {
                value.configuration_id = "alternate-v1".to_owned();
            },
            |value: &mut BoundedTrajectoryWitnessConfigurationV1| {
                value.plant_semantics_id = "alternate-plant".to_owned();
            },
        ];
        for mutate in mutations {
            let mut tampered = configuration.clone();
            mutate(&mut tampered);
            assert!(
                tampered.validate().is_err(),
                "tampered configuration accepted"
            );
        }
        assert!(
            validate_bounded_trajectory_reason_v1(BOUNDED_REASON_SUPPORTED_EXACT_WITNESS_VERIFIED)
                .is_ok()
        );
        assert!(
            validate_bounded_trajectory_reason_v1("supported/exact_witness_verified/extra")
                .is_err()
        );
        assert!(validate_bounded_trajectory_reason_v1("unknown/coverage/").is_err());
        assert!(validate_bounded_trajectory_reason_v1("unknown/coverage/invented").is_err());
        assert!(validate_bounded_trajectory_reason_v1("unknown/numerical/invented").is_err());
        assert!(validate_bounded_trajectory_reason_v1("unknown/scope//invented").is_err());
        assert!(validate_bounded_trajectory_reason_v1("unsupported/physics/no").is_err());

        let input = test_input();
        let adapter = NeutralAdapter::from_input(&input, &configuration).expect("adapter");
        let mut positive = SimulationState::new(&adapter.context).expect("positive state");
        let mut negative = SimulationState::new(&adapter.context).expect("negative state");
        positive.set_command(Command {
            throttle_frac: 0.0,
            target_attitude_rad: PI,
        });
        negative.set_command(Command {
            throttle_frac: 0.0,
            target_attitude_rad: -PI,
        });
        positive.step_physics_and_classify_contact(&adapter.context);
        negative.step_physics_and_classify_contact(&adapter.context);
        assert!(positive.attitude_rad > 0.0);
        assert!(negative.attitude_rad < 0.0);
    }

    #[test]
    fn feasible_one_waypoint_witness_verifies() {
        let input = test_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let witness = witness_for(&input, 960);
        let verification = verify_bounded_trajectory(&input, &configuration, &witness);
        assert_eq!(
            verification.status,
            BoundedTrajectoryArtifactStatusV1::Complete
        );
        assert_eq!(
            verification.verdict,
            Some(BoundedTrajectoryVerificationVerdictV1::Verified),
            "verification reason: {:?}",
            verification.first_reason
        );
        assert_eq!(verification.terminal_physics_step, Some(960));
        verification.validate().expect("verification seals");

        let prediction = BoundedTrajectoryWitnessPredictionV1::from_verification(
            &input,
            &configuration,
            &verification,
            None,
        )
        .expect("prediction seals");
        assert_eq!(
            prediction.decision,
            Some(BoundedTrajectoryDecisionV1::Supported)
        );
        prediction.validate().expect("prediction validates");
        prediction
            .validate_against_exact(&input, &configuration, &witness, &verification)
            .expect("prediction exact join validates");
    }

    #[test]
    fn feasible_two_waypoint_witness_preserves_ordered_phases_and_boundaries() {
        let input = two_waypoint_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let witness = witness_for(&input, 1_560);
        let verification = verify_bounded_trajectory(&input, &configuration, &witness);
        assert_eq!(
            verification.verdict,
            Some(BoundedTrajectoryVerificationVerdictV1::Verified),
            "verification reason: {:?}",
            verification.first_reason
        );
        assert_eq!(verification.terminal_physics_step, Some(1_560));
        assert_eq!(
            verification
                .phases
                .iter()
                .map(|phase| phase.phase.clone())
                .collect::<Vec<_>>(),
            ordered_route_phases(2)
        );
        assert!(verification.phases.iter().all(|phase| phase.passed));
        assert_eq!(
            verification
                .boundaries
                .iter()
                .map(|boundary| boundary.name.as_str())
                .collect::<Vec<_>>(),
            vec!["contact_exit", "tracking_entry", "handoff_0", "handoff_1"]
        );
        assert_eq!(verification.boundaries[2].physics_step, Some(960));
        assert_eq!(verification.boundaries[3].physics_step, Some(1_560));
        verification
            .validate_against_exact(&input, &configuration, &witness)
            .expect("two-waypoint exact validation");
    }

    #[test]
    fn exact_replay_matches_ordinary_physics_samples_and_d0_kernel_decisions() {
        let input = test_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let witness = witness_for(&input, 960);
        let trace = replay_once(&input, &configuration, &witness).expect("exact replay");
        let adapter = NeutralAdapter::from_input(&input, &configuration).expect("adapter");
        let ordinary = run_simulation(&adapter.context, "synthetic", |_, _| Command {
            throttle_frac: 1.0,
            target_attitude_rad: 0.0,
        })
        .expect("ordinary simulation");
        let replayed = replay_simulation(&adapter.context, "synthetic", &ordinary.actions)
            .expect("ordinary replay");

        assert_eq!(ordinary.samples, trace.samples);
        assert_eq!(replayed.samples, trace.samples);
        assert_eq!(replayed.actions, ordinary.actions);
        assert_eq!(ordinary.actions.len(), trace.installations.len());
        for (action, installation) in ordinary.actions.iter().zip(&trace.installations) {
            assert_eq!(action.physics_step, installation.physics_step);
            assert_eq!(action.command, installation.command.core_command());
        }
        assert_eq!(trace.states.len(), ordinary.samples.len());
        for (record, sample) in trace.states.iter().zip(&ordinary.samples) {
            let state = &record.state;
            let observation = &sample.observation;
            assert_eq!(state.physics_step, observation.physics_step);
            assert_eq!(state.sim_time_s, observation.sim_time_s);
            assert_eq!(state.position_m, observation.position_m);
            assert_eq!(state.velocity_mps, observation.velocity_mps);
            assert_eq!(state.attitude_rad, observation.attitude_rad);
            assert_eq!(state.angular_rate_radps, observation.angular_rate_radps);
            assert_eq!(state.fuel_kg, observation.fuel_kg);
            assert_eq!(state.mass_kg, observation.mass_kg);
            assert_eq!(
                state.contact,
                BoundedTrajectoryContactClassificationV1::None
            );
        }

        let source_input = SourceTransitionKernelInput {
            terrain: &adapter.terrain,
            source_pad_center_x_m: adapter.source_pad.center_x_m,
            vehicle: &adapter.vehicle,
            initial_state: &adapter.initial_state,
            geometry: &adapter.geometry,
            profile: &adapter.profile,
            selected_centerline_m: &adapter.normalized_centerline,
            waypoints: &adapter.waypoints,
            samples: &trace.samples,
            cadence: SourceTransitionCadence::physics_rate(configuration.physics_hz),
            audit: crate::SourceTransitionAuditInput::default(),
        };
        let source = extract_source_transition_kernel(&source_input).expect("source kernel");
        let route_input = RouteExecutionKernelInput {
            terrain: &adapter.terrain,
            source_pad_center_x_m: adapter.source_pad.center_x_m,
            source_pad_surface_y_m: adapter.source_pad.surface_y_m,
            target_pad_center_x_m: adapter.target_pad.center_x_m,
            target_pad_surface_y_m: adapter.target_pad.surface_y_m,
            vehicle: &adapter.vehicle,
            initial_state: &adapter.initial_state,
            geometry: &adapter.geometry,
            profile: &adapter.profile,
            selected_centerline_m: &adapter.normalized_centerline,
            waypoints: &adapter.waypoints,
            samples: &trace.samples,
            cadence: SourceTransitionCadence::physics_rate(configuration.physics_hz),
        };
        let route = extract_route_execution_kernel(&route_input).expect("route kernel");
        assert_eq!(
            trace.source.as_ref().map(|source| &source.physical_digest),
            Some(&source.physical_digest)
        );
        assert_eq!(
            trace.route.as_ref().map(|route| &route.physical_digest),
            Some(&route.physical_digest)
        );
        assert_eq!(
            trace
                .route
                .as_ref()
                .and_then(|route| route.waypoints[0].first_contract_pass.as_ref())
                .map(|pass| pass.sample.physics_step),
            route.waypoints[0]
                .first_contract_pass
                .as_ref()
                .map(|pass| pass.sample.physics_step)
        );
    }

    #[test]
    fn odd_terminal_step_is_accepted_at_first_final_handoff() {
        let input = odd_terminal_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let witness = witness_for(&input, 959);
        let verification = verify_bounded_trajectory(&input, &configuration, &witness);
        assert_eq!(
            verification.verdict,
            Some(BoundedTrajectoryVerificationVerdictV1::Verified),
            "verification reason: {:?}",
            verification.first_reason
        );
        assert_eq!(verification.terminal_physics_step, Some(959));
    }

    #[test]
    fn verification_prediction_round_trips_and_reject_semantic_tampering() {
        let input = test_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let witness = witness_for(&input, 960);
        let verification = verify_bounded_trajectory(&input, &configuration, &witness);
        let verification_bytes = serde_json::to_vec(&verification).expect("verification json");
        let verification_round_trip: BoundedTrajectoryVerificationV1 =
            serde_json::from_slice(&verification_bytes).expect("verification round trip");
        assert_eq!(verification_round_trip, verification);
        verification_round_trip
            .validate_against_exact(&input, &configuration, &witness)
            .expect("verification exact join validates");

        let repeated = verify_bounded_trajectory(&input, &configuration, &witness);
        assert_eq!(repeated, verification);
        assert_eq!(
            serde_json::to_vec(&repeated).expect("repeated verification json"),
            verification_bytes
        );

        let trace = replay_once(&input, &configuration, &witness).expect("replay trace");
        assert_eq!(
            trace.trajectory_digest,
            replay_trajectory_digest(
                &trace.samples,
                &trace.states,
                &trace.installations,
                &trace.phases,
                &trace.boundaries,
                &trace.rejection,
            )
            .expect("trajectory digest")
        );
        let mut tampered_phases = trace.phases.clone();
        tampered_phases[0].passed = false;
        tampered_phases[0].terminal_physics_step = None;
        assert_ne!(
            replay_trajectory_digest(
                &trace.samples,
                &trace.states,
                &trace.installations,
                &tampered_phases,
                &trace.boundaries,
                &trace.rejection,
            )
            .expect("tampered phase digest"),
            trace.trajectory_digest
        );
        let mut tampered_boundaries = trace.boundaries.clone();
        tampered_boundaries[0].name.push_str("_tampered");
        assert_ne!(
            replay_trajectory_digest(
                &trace.samples,
                &trace.states,
                &trace.installations,
                &trace.phases,
                &tampered_boundaries,
                &trace.rejection,
            )
            .expect("tampered boundary digest"),
            trace.trajectory_digest
        );

        let mut semantic_verification = verification.clone();
        semantic_verification
            .terminal_state
            .as_mut()
            .expect("terminal state")
            .position_m
            .x += 1.0;
        semantic_verification.verification_digest = semantic_verification
            .digest()
            .expect("tampered verification can be canonically serialized");
        semantic_verification
            .validate()
            .expect("recomputed tampered digest is structurally valid");
        assert!(
            semantic_verification
                .validate_against_exact(&input, &configuration, &witness)
                .is_err()
        );

        let prediction = BoundedTrajectoryWitnessPredictionV1::from_verification(
            &input,
            &configuration,
            &verification,
            None,
        )
        .expect("prediction seals");
        let prediction_bytes = serde_json::to_vec(&prediction).expect("prediction json");
        let prediction_round_trip: BoundedTrajectoryWitnessPredictionV1 =
            serde_json::from_slice(&prediction_bytes).expect("prediction round trip");
        assert_eq!(prediction_round_trip, prediction);
        prediction_round_trip
            .validate_against_exact(&input, &configuration, &witness, &verification)
            .expect("prediction exact join validates");

        let mut semantic_prediction = prediction.clone();
        semantic_prediction.decision = Some(BoundedTrajectoryDecisionV1::Unknown);
        semantic_prediction.witness_digest = None;
        semantic_prediction.first_reason =
            Some(BOUNDED_REASON_UNKNOWN_COVERAGE_NO_VERIFIED_WITNESS.to_owned());
        semantic_prediction.prediction_digest = semantic_prediction
            .compute_digest()
            .expect("tampered prediction can be canonically serialized");
        semantic_prediction
            .validate()
            .expect("recomputed tampered prediction is structurally valid");
        assert!(
            semantic_prediction
                .validate_against_exact(&input, &configuration, &witness, &verification)
                .is_err()
        );

        let mut different_input = input.clone();
        different_input.physical.gravity_mps2 = 1.1;
        different_input = different_input.seal().expect("different input seals");
        assert!(
            verification
                .validate_against_exact(&different_input, &configuration, &witness)
                .is_err()
        );
    }

    #[test]
    fn horizon_terminal_step_and_nonminimal_terminal_are_distinguished() {
        let input = horizon_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let horizon_witness = witness_for(&input, BOUNDED_TRAJECTORY_MAX_PHYSICS_STEPS);
        let verification = verify_bounded_trajectory(&input, &configuration, &horizon_witness);
        assert_eq!(
            verification.verdict,
            Some(BoundedTrajectoryVerificationVerdictV1::Verified),
            "verification reason: {:?}",
            verification.first_reason
        );
        assert_eq!(
            verification.terminal_physics_step,
            Some(BOUNDED_TRAJECTORY_MAX_PHYSICS_STEPS)
        );

        let early_input = test_input();
        let early_witness = witness_for(&early_input, 1_000);
        let early = verify_bounded_trajectory(&early_input, &configuration, &early_witness);
        assert_eq!(
            early.verdict,
            Some(BoundedTrajectoryVerificationVerdictV1::Rejected)
        );
        assert_eq!(
            early.first_reason.as_deref(),
            Some("rejected/terminal/nonminimal")
        );
    }

    #[test]
    fn contact_and_deadline_failures_are_rejected_not_invalid_or_unsupported() {
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let contact = contact_input();
        let contact_result =
            verify_bounded_trajectory(&contact, &configuration, &witness_for(&contact, 1));
        assert_eq!(
            contact_result.status,
            BoundedTrajectoryArtifactStatusV1::Complete
        );
        assert_eq!(
            contact_result.verdict,
            Some(BoundedTrajectoryVerificationVerdictV1::Rejected)
        );
        assert!(
            contact_result
                .first_reason
                .as_deref()
                .is_some_and(|reason| reason.starts_with("rejected/contact/"))
        );

        let deadline = deadline_input();
        let deadline_result =
            verify_bounded_trajectory(&deadline, &configuration, &witness_for(&deadline, 1_440));
        assert_eq!(
            deadline_result.verdict,
            Some(BoundedTrajectoryVerificationVerdictV1::Rejected),
            "deadline reason: {:?}",
            deadline_result.first_reason
        );
        assert_eq!(
            deadline_result.first_reason.as_deref(),
            Some("rejected/handoff/waypoint_0_no_contract_pass")
        );
    }

    #[test]
    fn initial_deadline_is_recorded_by_replayed_verifier() {
        let input = initial_deadline_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let witness = witness_for(&input, 41);
        let trace = replay_once(&input, &configuration, &witness).expect("replay");
        let route = trace.route.as_ref().expect("route kernel output");
        let waypoint = &route.waypoints[0];
        assert!(waypoint.initially_at_deadline);
        assert!(waypoint.deadline.is_none());
        assert_eq!(
            waypoint
                .resolution
                .as_ref()
                .map(|resolution| resolution.kind),
            Some(RouteExecutionResolutionKind::InitialDeadline)
        );
        assert_eq!(waypoint.opportunity.len(), 1);
        assert_eq!(trace.rejection, None);
        assert_eq!(trace.terminal_state.physics_step, 41);
        let (source, route_direct) = direct_d0_kernels(&input, &configuration, &trace);
        assert_eq!(
            trace.source.as_ref().map(|source| &source.physical_digest),
            Some(&source.physical_digest)
        );
        assert_eq!(
            route_direct.waypoints[0].resolution,
            route.waypoints[0].resolution
        );
        let verification = verify_bounded_trajectory(&input, &configuration, &witness);
        assert_eq!(
            verification.verdict,
            Some(BoundedTrajectoryVerificationVerdictV1::Verified)
        );
    }

    #[test]
    fn sticky_capture_window_is_preserved_in_replayed_route_kernel() {
        let input = sticky_window_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let terminal_step = 10_000_u64;
        let mut commands = Vec::with_capacity(terminal_step as usize / 2);
        for slot in 0..terminal_step.div_ceil(2) {
            let target_attitude_rad = if slot < 3_900 { 0.0 } else { -PI / 2.0 };
            commands.push(BoundedTrajectoryCommandV1 {
                slot,
                throttle_frac: 1.0,
                target_attitude_rad,
            });
        }
        let witness = BoundedTrajectoryWitnessV1::new(
            input.physical_digest().expect("input digest"),
            configuration.configuration_digest.clone(),
            commands,
            terminal_step,
        )
        .expect("sticky witness");
        let trace = replay_once(&input, &configuration, &witness).expect("sticky replay");
        let route = trace.route.as_ref().expect("route kernel output");
        let waypoint = &route.waypoints[0];
        assert!(waypoint.capture_entry.is_some());
        assert!(waypoint.opportunity.iter().any(|sample| sample.window_seen));
        assert!(waypoint.opportunity.windows(2).any(|pair| {
            pair[0].window_seen
                && pair[0].kinematics.distance_m <= waypoint.contract.capture_radius_m
                && pair[1].kinematics.distance_m > waypoint.contract.capture_radius_m
                && pair[1].window_seen
        }));
        let (source_direct, route_direct) = direct_d0_kernels(&input, &configuration, &trace);
        assert_eq!(
            trace.source.as_ref().map(|source| &source.physical_digest),
            Some(&source_direct.physical_digest)
        );
        assert_eq!(
            route_direct.waypoints[0].capture_entry,
            waypoint.capture_entry
        );
        assert_eq!(route_direct.waypoints[0].resolution, waypoint.resolution);
        let verification = verify_bounded_trajectory(&input, &configuration, &witness);
        assert_eq!(
            verification.verdict,
            Some(BoundedTrajectoryVerificationVerdictV1::Rejected)
        );
        assert_eq!(
            verification.first_reason.as_deref(),
            Some("rejected/handoff/waypoint_0_no_contract_pass")
        );
    }

    #[test]
    fn three_waypoint_input_is_a_scope_abstention() {
        let input = out_of_scope_three_waypoint_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let witness = witness_for(&input, 1);
        let verification = verify_bounded_trajectory(&input, &configuration, &witness);
        assert_eq!(
            verification.verdict,
            Some(BoundedTrajectoryVerificationVerdictV1::Rejected)
        );
        assert_eq!(
            verification.first_reason.as_deref(),
            Some("rejected/scope/waypoint_count")
        );
        let prediction = BoundedTrajectoryWitnessPredictionV1::from_verification(
            &input,
            &configuration,
            &verification,
            None,
        )
        .expect("scope prediction seals");
        assert_eq!(
            prediction.decision,
            Some(BoundedTrajectoryDecisionV1::Unknown)
        );
        assert_eq!(
            prediction.first_reason.as_deref(),
            Some("unknown/scope/waypoint_count")
        );
    }

    #[test]
    fn replay_installs_even_slots_and_records_producing_commands() {
        let input = test_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let witness = witness_for(&input, 5);
        let trace = replay_once(&input, &configuration, &witness).expect("replay");
        assert_eq!(
            trace
                .installations
                .iter()
                .map(|event| event.physics_step)
                .collect::<Vec<_>>(),
            vec![0, 2, 4]
        );
        assert_eq!(trace.states[0].state.physics_step, 0);
        assert_eq!(trace.states[0].state.held_command, None);
        assert_eq!(
            trace.states[1].state.held_command,
            Some(witness.commands[0])
        );
        assert_eq!(
            trace.states[2].state.held_command,
            Some(witness.commands[0])
        );
        assert_eq!(
            trace.states[3].state.held_command,
            Some(witness.commands[1])
        );
        assert_eq!(
            trace.states[4].state.held_command,
            Some(witness.commands[1])
        );
        assert_eq!(
            trace.states[5].state.held_command,
            Some(witness.commands[2])
        );
    }

    #[test]
    fn malformed_witnesses_are_invalid_even_when_a_digest_is_stale_or_recomputed() {
        let input = test_input();
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let witness = witness_for(&input, 4);

        let mut slot_hole = witness.clone();
        slot_hole.commands[1].slot = 3;
        assert_invalid_verification(&input, &configuration, &slot_hole);

        let mut duplicate_slot = witness.clone();
        duplicate_slot.commands[1].slot = 0;
        assert_invalid_verification(&input, &configuration, &duplicate_slot);

        let mut missing = witness.clone();
        missing.commands.pop();
        assert_invalid_verification(&input, &configuration, &missing);

        let mut extra = witness.clone();
        extra.commands.push(BoundedTrajectoryCommandV1 {
            slot: 2,
            throttle_frac: 1.0,
            target_attitude_rad: 0.0,
        });
        assert_invalid_verification(&input, &configuration, &extra);

        let mut recomputed_semantic_tamper = witness.clone();
        recomputed_semantic_tamper.input_digest = "different-input".to_owned();
        recomputed_semantic_tamper.witness_digest = recomputed_semantic_tamper
            .digest()
            .expect("tampered witness can be canonically serialized");
        assert_invalid_verification(&input, &configuration, &recomputed_semantic_tamper);

        assert!(
            BoundedTrajectoryWitnessV1::new(
                input.physical_digest().expect("input digest"),
                configuration.configuration_digest.clone(),
                Vec::new(),
                0,
            )
            .is_err()
        );
        assert!(
            BoundedTrajectoryWitnessV1::new(
                input.physical_digest().expect("input digest"),
                configuration.configuration_digest,
                Vec::new(),
                u64::MAX,
            )
            .is_err()
        );
    }

    #[test]
    fn direct_route_is_unknown_scope_without_unsupported() {
        let mut input = test_input();
        input.physical.topology = RouteTopology::Direct;
        input.physical.waypoints.clear();
        input.provenance.waypoint_ids.clear();
        input = input.seal().expect("direct input seals");
        let configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("configuration seals");
        let prediction = BoundedTrajectoryWitnessPredictionV1::unknown_scope_direct_route(
            &input,
            &configuration,
        )
        .expect("direct scope prediction");
        assert_eq!(
            prediction.decision,
            Some(BoundedTrajectoryDecisionV1::Unknown)
        );
        assert!(
            prediction
                .first_reason
                .as_deref()
                .is_some_and(|reason| reason.starts_with("unknown/scope/"))
        );
        prediction.validate().expect("prediction validates");
    }
}
