//! A checkpoint adapter example, not a trained landing policy. It imports only
//! the shared platform and has no maintained controller/planner fallback.

use std::path::Path;

use pd_core::{Command, Observation};
use pd_missions::sha256;
use pd_run::{Agent, AgentDescriptorV1, AgentFrame, MissionView, TelemetryValue};
use serde::{Deserialize, Serialize};

pub const OBSERVATION_SCHEMA: &str = "pd-policy-example/physical-features-v1";
pub const ACTION_SCHEMA: &str = "pd-policy-example/throttle-attitude-v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearCheckpointV1 {
    pub schema_version: u32,
    pub observation_schema: String,
    pub action_schema: String,
    pub input_scales: [f64; 12],
    pub feature_clip: f64,
    pub weights: [[f64; 12]; 2],
    pub bias: [f64; 2],
    pub max_target_attitude_rad: f64,
    pub terrain_lookahead_m: f64,
}

impl LinearCheckpointV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.observation_schema != OBSERVATION_SCHEMA
            || self.action_schema != ACTION_SCHEMA
            || self
                .input_scales
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0)
            || self
                .weights
                .iter()
                .flatten()
                .chain(self.bias.iter())
                .any(|value| !value.is_finite() || value.abs() > 1_000_000.0)
            || !self.feature_clip.is_finite()
            || !(0.0..=100.0).contains(&self.feature_clip)
            || self.feature_clip == 0.0
            || !self.max_target_attitude_rad.is_finite()
            || !(0.0..=std::f64::consts::PI).contains(&self.max_target_attitude_rad)
            || !self.terrain_lookahead_m.is_finite()
            || !(0.0..=10_000.0).contains(&self.terrain_lookahead_m)
        {
            return Err("unsupported or invalid checkpoint observation/action mapping".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct LinearPolicy {
    checkpoint: LinearCheckpointV1,
    checkpoint_sha256: String,
}

impl LinearPolicy {
    pub fn load(path: &Path) -> Result<Self, String> {
        Self::from_bytes(&std::fs::read(path).map_err(|e| e.to_string())?)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let checkpoint: LinearCheckpointV1 =
            serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        checkpoint.validate()?;
        Ok(Self {
            checkpoint,
            checkpoint_sha256: sha256(bytes),
        })
    }
    pub fn features(&self, mission: MissionView<'_>, obs: &Observation) -> [f64; 12] {
        let points = mission.world.terrain.points();
        let (left, right) = (points[0].x, points.last().expect("validated terrain").x);
        let probe_x = (obs.position_m.x
            + obs.target_dx_m.signum() * self.checkpoint.terrain_lookahead_m)
            .clamp(left, right);
        let terrain_relative = mission.world.terrain.sample_height(probe_x) - obs.position_m.y;
        let raw = [
            obs.target_dx_m,
            obs.height_above_target_m,
            obs.velocity_mps.x,
            obs.velocity_mps.y,
            obs.attitude_rad,
            obs.angular_rate_radps,
            obs.fuel_kg,
            obs.mass_kg,
            obs.min_hull_clearance_m,
            (mission.sim.max_time_s - obs.sim_time_s).max(0.0),
            terrain_relative,
            mission.target_pad.width_m,
        ];
        std::array::from_fn(|index| {
            (raw[index] / self.checkpoint.input_scales[index])
                .clamp(-self.checkpoint.feature_clip, self.checkpoint.feature_clip)
        })
    }
}

impl Agent for LinearPolicy {
    fn descriptor(&self) -> AgentDescriptorV1 {
        AgentDescriptorV1 {
            schema_version: 1,
            implementation: "linear-checkpoint-policy-example-v1".to_owned(),
            configuration: serde_json::to_value(&self.checkpoint).expect("validated checkpoint"),
            checkpoint_sha256: Some(self.checkpoint_sha256.clone()),
        }
    }
    fn update(
        &mut self,
        mission: MissionView<'_>,
        obs: &Observation,
    ) -> Result<AgentFrame, String> {
        let features = self.features(mission, obs);
        if features.iter().any(|value| !value.is_finite()) {
            return Err("nonfinite physical observation".to_owned());
        }
        let outputs: [f64; 2] = std::array::from_fn(|row| {
            self.checkpoint.bias[row]
                + self.checkpoint.weights[row]
                    .iter()
                    .zip(features)
                    .map(|(weight, value)| weight * value)
                    .sum::<f64>()
        });
        let mut frame = AgentFrame::command_only(Command {
            throttle_frac: 1.0 / (1.0 + (-outputs[0]).exp()),
            target_attitude_rad: outputs[1].tanh() * self.checkpoint.max_target_attitude_rad,
        });
        frame.status = "Checkpoint policy example (untrained)".to_owned();
        frame.phase = Some("policy".to_owned());
        frame.metrics.insert(
            "policy.raw_throttle".to_owned(),
            TelemetryValue::Float(outputs[0]),
        );
        frame.metrics.insert(
            "policy.raw_attitude".to_owned(),
            TelemetryValue::Float(outputs[1]),
        );
        Ok(frame)
    }
}
