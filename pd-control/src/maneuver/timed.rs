//! Version 2 adds a timed goal and current-observation admission. V1 wire types
//! and callbacks retain their original semantics and have no implicit upgrade.
use pd_core::Observation;
use serde::{Deserialize, Serialize};

use super::{ManeuverBackendDescriptorV1, NativeEndV1};
use crate::{ControllerFrame, poly_landing::PolynomialReferenceV1};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ManeuverGoalV2 {
    LandOnMissionTargetAt {
        target_pad_id: String,
        /// Planned touchdown/reference endpoint on the original physics clock.
        /// Actual first native contact remains authoritative.
        reference_end_physics_step: u64,
    },
    Cancel {},
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManeuverRequestV2 {
    pub schema_version: u32,
    pub id: String,
    pub goal: ManeuverGoalV2,
}
impl ManeuverRequestV2 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 2 || self.id.trim().is_empty() || self.id.len() > 128 {
            return Err("request needs version 2 and a nonempty ID of at most 128 bytes".into());
        }
        if let ManeuverGoalV2::LandOnMissionTargetAt { target_pad_id, .. } = &self.goal
            && (target_pad_id.trim().is_empty() || target_pad_id.len() > 128)
        {
            return Err("request needs a nonempty target ID of at most 128 bytes".into());
        }
        Ok(())
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let request: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        request.validate()?;
        Ok(request)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdmissionV2 {
    Accepted,
    AlreadyActive,
    InvalidRequest { reason: String },
    Unsupported { reason: String },
    NoPlan { reason: String },
    Busy,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedManeuverV2 {
    pub request: ManeuverRequestV2,
    pub entry: Observation,
    pub reference: PolynomialReferenceV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManeuverFeedbackV2 {
    pub schema_version: u32,
    pub accepted: Option<AcceptedManeuverV2>,
    pub command_updates: u64,
    /// Count pure coefficient constructions, separately from physical work.
    pub constructions: u64,
    /// Pure paired-actuation checks across the complete declared burn window.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub admission_actuation_checks: u64,
    pub reference_exhausted: bool,
    pub native_end: Option<NativeEndV1>,
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManeuverDescriptorV2 {
    pub schema_version: u32,
    pub implementation: String,
    pub backend: ManeuverBackendDescriptorV1,
    pub physics_hz: u32,
    pub controller_hz: u32,
    pub first_command_physics_step: u64,
    pub absolute_deadline_physics_step: u64,
    pub coast_owner: String,
    pub command_mapping: String,
}

pub enum ManeuverFrameV2 {
    Command(ControllerFrame),
    /// Honest bounded operation failure; the caller closes the native prefix.
    ReferenceExhausted,
}

pub trait ManeuverControllerV2 {
    fn descriptor(&self) -> ManeuverDescriptorV2;
    fn submit(
        &mut self,
        request: &ManeuverRequestV2,
        observation: &Observation,
    ) -> Result<AdmissionV2, String>;
    fn frame(&mut self, observation: &Observation) -> Result<ManeuverFrameV2, String>;
    fn feedback(&self) -> ManeuverFeedbackV2;
    fn native_end(&mut self, end: NativeEndV1) -> Result<(), String>;
}
