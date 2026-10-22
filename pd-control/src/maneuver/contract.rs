use pd_core::{
    BoundedRunFailureV1, BoundedRunStopCauseV1, MissionOutcome, Observation, PhysicalOutcome,
};
use pd_run::EpisodeSummaryV1;
use serde::{Deserialize, Serialize};

use crate::ControllerFrame;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ManeuverGoalV1 {
    LandOnMissionTarget {
        target_pad_id: String,
    },
    /// Recognizable wire operation, deliberately unsupported by this capability.
    Cancel {},
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManeuverRequestV1 {
    pub schema_version: u32,
    pub id: String,
    pub goal: ManeuverGoalV1,
}

impl ManeuverRequestV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.id.trim().is_empty() || self.id.len() > 128 {
            return Err("request needs version 1 and a nonempty ID of at most 128 bytes".into());
        }
        if let ManeuverGoalV1::LandOnMissionTarget { target_pad_id } = &self.goal
            && (target_pad_id.trim().is_empty() || target_pad_id.len() > 128)
        {
            return Err("request needs a nonempty target ID of at most 128 bytes".into());
        }
        Ok(())
    }

    /// Decode the versioned input before semantic admission.
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let request: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        request.validate()?;
        Ok(request)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdmissionV1 {
    Accepted,
    AlreadyActive,
    InvalidRequest { reason: String },
    Unsupported { reason: String },
    Busy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CommandOwnerV1 {
    UprightCoast,
    Landing { request_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedManeuverV1 {
    pub request: ManeuverRequestV1,
    pub activation_physics_step: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEndV1 {
    pub schema_version: u32,
    pub physics_step: u64,
    pub physical_outcome: PhysicalOutcome,
    pub mission_outcome: MissionOutcome,
    pub stop: BoundedRunStopCauseV1,
    pub failure: Option<BoundedRunFailureV1>,
}

impl From<&EpisodeSummaryV1> for NativeEndV1 {
    fn from(summary: &EpisodeSummaryV1) -> Self {
        Self {
            schema_version: 1,
            physics_step: summary.final_state.physics_step,
            physical_outcome: summary.manifest.physical_outcome.clone(),
            mission_outcome: summary.manifest.mission_outcome.clone(),
            stop: summary.stop,
            failure: summary.failure.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManeuverFeedbackV1 {
    pub schema_version: u32,
    pub accepted: Option<AcceptedManeuverV1>,
    pub owner: CommandOwnerV1,
    pub command_updates: u64,
    pub backend_updates: u64,
    pub native_end: Option<NativeEndV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManeuverBackendDescriptorV1 {
    pub implementation: String,
    /// Configuration identity only. Requests remain typed maneuver goals.
    pub configuration: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManeuverDescriptorV1 {
    pub schema_version: u32,
    pub implementation: String,
    pub capability: String,
    pub physics_hz: u32,
    pub controller_hz: u32,
    pub backend: ManeuverBackendDescriptorV1,
    pub coast: String,
    pub command_mapping: String,
}

/// Construction separates unsupported complete inputs from execution failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManeuverInputErrorV1 {
    InvalidInput(String),
    UnsupportedInput(String),
}

/// Fresh object per episode. Context is bound at construction, with no replace
/// or restore operation. Only the native harness supplies ticks and final ends.
pub trait ManeuverController {
    fn descriptor(&self) -> ManeuverDescriptorV1;
    fn submit(
        &mut self,
        request: &ManeuverRequestV1,
        physics_step: u64,
    ) -> Result<AdmissionV1, String>;
    fn frame(&mut self, observation: &Observation) -> Result<ControllerFrame, String>;
    fn feedback(&self) -> ManeuverFeedbackV1;
    fn native_end(&mut self, end: NativeEndV1) -> Result<(), String>;
}

/// Shared lifecycle for episode-local controllers. Admission does not tick the
/// backend. It must be followed by a command at the same boundary.
pub(crate) struct Lifecycle {
    pub feedback: ManeuverFeedbackV1,
    pub next_tick: u64,
    interval: u64,
}

impl Lifecycle {
    pub fn new(interval: u64) -> Self {
        Self {
            feedback: ManeuverFeedbackV1 {
                schema_version: 1,
                accepted: None,
                owner: CommandOwnerV1::UprightCoast,
                command_updates: 0,
                backend_updates: 0,
                native_end: None,
            },
            next_tick: 0,
            interval,
        }
    }

    pub fn live(&self) -> Result<(), String> {
        if self.feedback.native_end.is_some() {
            return Err("maneuver controller is closed".into());
        }
        Ok(())
    }

    pub fn accept(&mut self, request: &ManeuverRequestV1, tick: u64) -> AdmissionV1 {
        if let Some(active) = &self.feedback.accepted {
            return if request.id == active.request.id {
                if *request == active.request {
                    AdmissionV1::AlreadyActive
                } else {
                    AdmissionV1::InvalidRequest {
                        reason: "ID is bound to another payload".into(),
                    }
                }
            } else {
                AdmissionV1::Busy
            };
        }
        self.feedback.accepted = Some(AcceptedManeuverV1 {
            request: request.clone(),
            activation_physics_step: tick,
        });
        self.feedback.owner = CommandOwnerV1::Landing {
            request_id: request.id.clone(),
        };
        AdmissionV1::Accepted
    }

    pub fn check_frame(&self, observation: &Observation) -> Result<(), String> {
        self.live()?;
        if observation.physics_step != self.next_tick {
            return Err("duplicate, skipped or out-of-order command boundary".into());
        }
        self.next_tick
            .checked_add(self.interval)
            .ok_or("command tick overflow")?;
        Ok(())
    }

    pub fn produced(&mut self, backend: bool) {
        self.next_tick += self.interval;
        self.feedback.command_updates += 1;
        self.feedback.backend_updates += u64::from(backend);
    }

    pub fn close(&mut self, end: NativeEndV1) -> Result<(), String> {
        self.live()?;
        let earliest = self.next_tick.saturating_sub(self.interval);
        if end.schema_version != 1
            || end.physics_step < earliest
            || end.physics_step > self.next_tick
        {
            return Err("native end is outside the final command interval".into());
        }
        self.feedback.native_end = Some(end);
        Ok(())
    }
}
