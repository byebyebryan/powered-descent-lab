//! Approach-specific Python binding; the neutral physical bridge stays independent.
mod generator;
mod report;
#[cfg(test)]
mod tests;

use pd_control::maneuver::*;
use pd_core::{BoundedRunLimitsV1, MissionOutcome, Observation, PhysicalOutcome, ScenarioSpec};
use pd_eval::{EvidenceScope, EvidenceSnapshotV1};
use pd_run::{
    AgentDescriptorV1, BoundedAgentRunArtifactsV1, CaptureModeV1, Episode, RunPerformanceStats,
};
use pyo3::{exceptions::PyValueError, prelude::*};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Instant};

fn error(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}
fn json<T: Serialize>(v: &T) -> PyResult<String> {
    serde_json::to_string(v).map_err(error)
}
fn decode<T: for<'de> Deserialize<'de>>(s: &str) -> PyResult<T> {
    serde_json::from_str(s).map_err(error)
}

fn features(o: &Observation, end: u64) -> Vec<f64> {
    vec![
        o.target_dx_m / 1500.0,
        o.height_above_target_m / 1000.0,
        o.velocity_mps.x / 100.0,
        o.velocity_mps.y / 100.0,
        o.attitude_rad.sin(),
        o.attitude_rad.cos(),
        o.angular_rate_radps / std::f64::consts::FRAC_PI_2,
        o.fuel_kg / 6300.0,
        (end as f64 - o.physics_step as f64) / 4800.0,
        (81_u64.saturating_sub(o.physics_step / 12)) as f64 / 81.0,
    ]
}

#[pyclass]
struct CoastEpisode {
    scenario: ScenarioSpec,
    end: u64,
    episode: Option<Episode>,
    controller: CoastLandingController,
    descriptor: AgentDescriptorV1,
    decisions: Vec<(u64, u8)>,
    committed: bool,
    closed: bool,
    fingerprint: Sha256,
    started: Instant,
    native_success: bool,
    evidence_scope: Option<EvidenceScope>,
    evidence_before: Option<EvidenceSnapshotV1>,
    work_limit: u64,
}

#[pymethods]
impl CoastEpisode {
    #[new]
    #[pyo3(signature = (scenario_json, reference_end, captured=false, descriptor_json=None, input_files=None))]
    fn new(
        scenario_json: &str,
        reference_end: u64,
        captured: bool,
        descriptor_json: Option<&str>,
        input_files: Option<Vec<String>>,
    ) -> PyResult<Self> {
        let scenario: ScenarioSpec = decode(scenario_json)?;
        let controller = CoastLandingController::new(&scenario, reference_end).map_err(error)?;
        let descriptor = if let Some(s) = descriptor_json {
            decode(s)?
        } else {
            AgentDescriptorV1 {
                schema_version: 1,
                implementation: "coast-ignite-v1".into(),
                checkpoint_sha256: None,
                configuration: serde_json::json!({"reference_end":reference_end,"policy":"software-fixture"}),
            }
        };
        descriptor.validate().map_err(error)?;
        let episode = Episode::reset(
            &scenario,
            &descriptor.implementation,
            BoundedRunLimitsV1 {
                hard_end_physics_step: 4800,
                command_coverage_end_physics_step: reference_end + 4,
            },
            if captured {
                CaptureModeV1::Captured
            } else {
                CaptureModeV1::Summary
            },
        )
        .map_err(error)?;
        if episode.stop().is_some() {
            return Err(error("requires an active airborne initial episode"));
        }
        let evidence_scope = captured.then(|| scope(input_files.unwrap_or_default()));
        let evidence_before = evidence_scope.as_ref().map(checked_snapshot).transpose()?;
        Ok(Self {
            scenario,
            end: reference_end,
            episode: Some(episode),
            controller,
            descriptor,
            decisions: vec![],
            committed: false,
            closed: false,
            fingerprint: Sha256::new(),
            started: Instant::now(),
            native_success: false,
            evidence_scope,
            evidence_before,
            work_limit: u64::MAX,
        })
    }

    fn observation(&self) -> PyResult<Vec<f64>> {
        Ok(features(&self.live()?.observation(), self.end))
    }
    fn physical_json(&self) -> PyResult<String> {
        json(&self.live()?.observation())
    }
    fn set_work_limit(&mut self, limit: u64) {
        self.work_limit = limit;
    }

    fn step(&mut self, py: Python<'_>, action: u8) -> PyResult<(Vec<f64>, f64, bool, String)> {
        if action > 1 || self.closed || self.committed || self.live()?.stop().is_some() {
            return Err(error(
                "invalid action or episode has no further policy decision",
            ));
        }
        let tick = self.live()?.observation().physics_step;
        if !tick.is_multiple_of(12) || tick > 960 {
            return Err(error("off-grid policy decision"));
        }
        self.decisions.push((tick, action));
        if action == 1 {
            let obs = self.live()?.observation();
            self.controller.ignite(&obs).map_err(error)?;
            self.committed = true;
        } else if tick == 960 {
            self.committed = true;
        }
        let next = tick + 12;
        while self.live()?.stop().is_none()
            && (self.committed || self.live()?.observation().physics_step < next)
        {
            py.check_signals()?;
            if self.work_limit < 2 {
                return Err(error(
                    "native resource budget exhausted before next command",
                ));
            }
            let obs = self.live()?.observation();
            let frame = self.controller.frame(&obs).map_err(error)?;
            self.fingerprint
                .update(serde_json::to_vec(&frame).map_err(error)?);
            let result = self
                .episode
                .as_mut()
                .unwrap()
                .step_frame(frame, None)
                .map_err(error)?;
            self.work_limit -= result.physics_steps_advanced;
            self.fingerprint
                .update(serde_json::to_vec(&result).map_err(error)?);
            self.native_success = result.physical_outcome == PhysicalOutcome::LandedOnTarget
                && result.mission_outcome == MissionOutcome::Success;
            if matches!(
                result.stop,
                Some(pd_core::BoundedRunStopCauseV1::ExecutionInvalid)
            ) {
                return Err(error("native software/numerical failure"));
            }
        }
        let done = self.live()?.stop().is_some();
        let obs = self.live()?.observation();
        let info = self.info()?;
        let reward = if done && info["native_success"] == true {
            1.0
        } else {
            0.0
        };
        Ok((features(&obs, self.end), reward, done, json(&info)?))
    }

    fn status_json(&self) -> PyResult<String> {
        json(&self.info()?)
    }

    fn finish_json(&mut self) -> PyResult<String> {
        let (result, info) = self.finish()?;
        json(&serde_json::json!({"summary":result.summary,"info":info}))
    }

    fn save_capture(&mut self, output: &str) -> PyResult<String> {
        let before = self
            .evidence_before
            .clone()
            .ok_or_else(|| error("Summary has no evidence scope"))?;
        let (result, info) = self.finish()?;
        let execution = BoundedAgentRunArtifactsV1 {
            descriptor: self.descriptor.clone(),
            episode: result
                .capture
                .ok_or_else(|| error("Summary has no capture"))?,
            controller_updates: result.agent_updates.unwrap(),
            performance: RunPerformanceStats {
                wall_time_us: self.started.elapsed().as_micros() as u64,
                thread_cpu_time_us: None,
            },
        };
        if replay_fingerprint(&self.scenario, &execution).map_err(error)?
            != format!("{:x}", self.fingerprint.clone().finalize())
        {
            return Err(error(
                "saved command/state fingerprint differs from live execution",
            ));
        }
        let evaluation = pd_eval::assess_execution(
            &self.scenario,
            execution,
            before,
            checked_snapshot(self.evidence_scope.as_ref().unwrap())?,
            true,
        )
        .map_err(error)?;
        if !evaluation.integrity_passed || !evaluation.replay.complete_episode_verified {
            return Err(error("physical replay/integrity failed"));
        }
        pd_eval::save_evaluation(Path::new(output), &evaluation).map_err(error)?;
        write_new(
            &Path::new(output).join("decisions.json"),
            json(&info)?.as_bytes(),
        )
        .map_err(error)?;
        json(
            &serde_json::json!({"info":info,"verified_target_landing":evaluation.verified_target_landing,"integrity_passed":evaluation.integrity_passed,"replay":evaluation.replay,"native_replay_steps":evaluation.execution.episode.final_state.physics_step,"neutral_replay_passes":3}),
        )
    }
}

fn replay_fingerprint(
    scenario: &ScenarioSpec,
    execution: &BoundedAgentRunArtifactsV1,
) -> Result<String, String> {
    let mut episode = Episode::reset(
        scenario,
        &execution.descriptor.implementation,
        execution.episode.limits,
        CaptureModeV1::Summary,
    )
    .map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    for update in &execution.controller_updates {
        if episode.observation().physics_step != update.physics_step {
            return Err("saved command clocks differ".into());
        }
        digest.update(serde_json::to_vec(&update.frame).map_err(|e| e.to_string())?);
        let step = episode
            .step_frame(update.frame.clone(), None)
            .map_err(|e| e.to_string())?;
        digest.update(serde_json::to_vec(&step).map_err(|e| e.to_string())?);
    }
    if episode.stop() != Some(execution.episode.stop) {
        return Err("saved command coverage differs".into());
    }
    Ok(format!("{:x}", digest.finalize()))
}

impl CoastEpisode {
    fn live(&self) -> PyResult<&Episode> {
        self.episode
            .as_ref()
            .ok_or_else(|| error("episode consumed"))
    }
    fn info(&self) -> PyResult<serde_json::Value> {
        let episode = self.live()?;
        let obs = episode.observation();
        Ok(
            serde_json::json!({"physics_steps":obs.physics_step,"stop":episode.stop(),"decisions":self.decisions,"fingerprint":format!("{:x}",self.fingerprint.clone().finalize()),"feedback":self.controller.feedback(),"native_success":self.native_success,"observation":obs}),
        )
    }
    fn finish(&mut self) -> PyResult<(pd_run::EpisodeResultV1, serde_json::Value)> {
        if self.live()?.stop().is_none() {
            return Err(error("cannot finish an active physical episode"));
        }
        let info = self.info()?;
        let result = self.episode.take().unwrap().finish().map_err(error)?;
        if result.summary.failure.is_some() {
            return Err(error("native failure is a software fault"));
        }
        self.controller
            .native_end((&result.summary).into())
            .map_err(error)?;
        self.closed = true;
        let mut info = info;
        info["feedback"] = serde_json::to_value(self.controller.feedback()).map_err(error)?;
        Ok((result, info))
    }
}

fn scope(inputs: Vec<String>) -> EvidenceScope {
    let files: Vec<pd_eval::FileIdentityV1> = serde_json::from_str(include_str!(concat!(
        env!("OUT_DIR"),
        "/compiled-sources.json"
    )))
    .unwrap();
    EvidenceScope {
        sources: files.into_iter().map(|f| f.path.into()).collect(),
        inputs: inputs.into_iter().map(Into::into).collect(),
    }
}
fn checked_snapshot(scope: &EvidenceScope) -> PyResult<EvidenceSnapshotV1> {
    let snapshot = scope.snapshot().map_err(error)?;
    let expected: Vec<pd_eval::FileIdentityV1> = decode(include_str!(concat!(
        env!("OUT_DIR"),
        "/compiled-sources.json"
    )))?;
    if snapshot.source_files != expected {
        return Err(error(
            "compiled source differs from runtime source; rebuild",
        ));
    }
    Ok(snapshot)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())
}

#[pyfunction]
fn source_identity_json() -> PyResult<String> {
    json(&checked_snapshot(&scope(vec![]))?)
}
#[pyfunction]
fn generate_proposal(anchor_json: &str, seed: u64, cohort: &str, index: u64) -> PyResult<String> {
    json(&generator::proposal(&decode(anchor_json)?, seed, cohort, index).map_err(error)?)
}
#[pymodule]
fn pd_coast_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<CoastEpisode>()?;
    m.add_function(wrap_pyfunction!(source_identity_json, m)?)?;
    m.add_function(wrap_pyfunction!(generate_proposal, m)?)?;
    m.add_function(wrap_pyfunction!(report::render_report, m)?)?;
    Ok(())
}
